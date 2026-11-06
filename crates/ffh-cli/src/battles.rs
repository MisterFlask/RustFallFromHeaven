//! `ffh battles FILE [--runs N] [--seed S]`: staged engagements for balance work
//! (docs/ENGAGEMENTS.md). Each line of FILE is one matchup; each is fought `--runs` times through
//! the rules' own entry (`group_attack`, the auto formation on both sides), from a save of the
//! staged world with the sync RNG reseeded per run, and printed as one JSON line of totals.
//!
//! A line: `name | ATTACKERS | DEFENDERS | GROUND`, `#` starts a comment. ATTACKERS and DEFENDERS
//! are comma-separated `[N*]UNIT[+PROMOTION]..` (the `UNIT_` and `PROMOTION_` prefixes may be left
//! out); an attacker's `@front`, `@middle` or `@rear` suffix moves it to that row of the auto
//! formation (to a free cell, else swapping with a unit there; not for large units). GROUND is
//! space-separated words: `open` (the default: flat grassland, no feature), `forest`, `hills`,
//! `city` (the defender's city, size 6), `walls` (a city with Walls), `fort:N` (the defenders'
//! fortify turns), `dmg:N` (every unit's damage, percent), `waves:N` (up to N attack orders in
//! the turn, each committing the attackers that have not attacked yet, until the plot falls),
//! `bombard:N` (N turns of siege first: each attacker that can bombard does, then the attackers'
//! attacks and moves are reset; the defenders do not heal, as in a besieged city).
//!
//! The world is a duel map, players 0 (attacker) and 1 (defender) at war, no controllers. Every
//! unit keeps only its unit type's free promotions and the ones the line names (the civs' races and
//! the leaders' traits are stripped), and no weapons, so a line is the same battle whatever civs
//! the map drew.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::{BattleReport, LogEvent, LogFate};
use ffh_core::engagement::{HitCtx, Place, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_core::setup::GameSetup;
use ffh_data::*;

struct Stack {
    ty: UnitType,
    promos: Vec<PromotionType>,
    n: usize,
    row: Option<u8>,
}

struct Spec {
    name: String,
    att: Vec<Stack>,
    def: Vec<Stack>,
    forest: bool,
    hills: bool,
    city: bool,
    walls: bool,
    fort: i32,
    dmg: i32,
    waves: usize,
    bombard: usize,
}

fn parse_stacks(d: &GameData, s: &str) -> Result<Vec<Stack>, String> {
    let mut out = Vec::new();
    for item in s.split(',').map(str::trim).filter(|x| !x.is_empty()) {
        let (n, rest) = match item.split_once('*') {
            Some((n, r)) => (
                n.trim()
                    .parse::<usize>()
                    .map_err(|_| format!("bad count in {item:?}"))?,
                r.trim(),
            ),
            None => (1, item),
        };
        let (rest, row) = match rest.split_once('@') {
            Some((r, "front")) => (r, Some(0)),
            Some((r, "middle")) => (r, Some(1)),
            Some((r, "rear")) => (r, Some(2)),
            Some(_) => return Err(format!("bad row in {item:?}")),
            None => (rest, None),
        };
        let mut parts = rest.split('+').map(str::trim);
        let uname = parts.next().unwrap_or("");
        let uname = if uname.starts_with("UNIT_") {
            uname.to_string()
        } else {
            format!("UNIT_{uname}")
        };
        let ty = d
            .ty::<UnitType>(&uname)
            .ok_or_else(|| format!("unknown unit {uname}"))?;
        let mut promos = Vec::new();
        for p in parts {
            let pname = if p.starts_with("PROMOTION_") {
                p.to_string()
            } else {
                format!("PROMOTION_{p}")
            };
            promos.push(
                d.ty::<PromotionType>(&pname)
                    .ok_or_else(|| format!("unknown promotion {pname}"))?,
            );
        }
        out.push(Stack { ty, promos, n, row });
    }
    Ok(out)
}

fn parse_line(d: &GameData, line: &str) -> Result<Option<Spec>, String> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return Ok(None);
    }
    let f: Vec<&str> = line.split('|').map(str::trim).collect();
    if f.len() < 3 {
        return Err(format!(
            "want `name | attackers | defenders | ground`: {line:?}"
        ));
    }
    let mut s = Spec {
        name: f[0].to_string(),
        att: parse_stacks(d, f[1])?,
        def: parse_stacks(d, f[2])?,
        forest: false,
        hills: false,
        city: false,
        walls: false,
        fort: 0,
        dmg: 0,
        waves: 1,
        bombard: 0,
    };
    for w in f.get(3).copied().unwrap_or("").split_whitespace() {
        match w {
            "open" => {}
            "forest" => s.forest = true,
            "hills" => s.hills = true,
            "city" => s.city = true,
            "walls" => {
                s.city = true;
                s.walls = true;
            }
            _ => {
                if let Some(n) = w.strip_prefix("fort:") {
                    s.fort = n.parse().map_err(|_| format!("bad {w}"))?;
                } else if let Some(n) = w.strip_prefix("bombard:") {
                    s.bombard = n.parse().map_err(|_| format!("bad {w}"))?;
                } else if let Some(n) = w.strip_prefix("waves:") {
                    s.waves = n.parse().map_err(|_| format!("bad {w}"))?;
                } else if let Some(n) = w.strip_prefix("dmg:") {
                    s.dmg = n.parse().map_err(|_| format!("bad {w}"))?;
                } else {
                    return Err(format!("unknown ground word {w:?}"));
                }
            }
        }
    }
    Ok(Some(s))
}

/// The staged world of `s`: the game, the attacking and defended plots, an attacker, the
/// attackers' row overrides.
fn stage(s: &Spec) -> (Game, PlotIdx, PlotIdx, UnitId, Vec<(UnitId, u8)>) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p)
            && !g.plot_is_peak(p)
            && !g.plot_is_hills(p)
            && plot.units.is_empty()
            && plot.owner.is_none()
            && plot.feature_type.is_none()
            && plot.improvement_type.is_none()
            && g.plot_defense_modifier(p, None, false) == 0
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST)))
                .then_some((p, q))
        })
        .expect("two flat plots side by side");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    g.map.plots[from].terrain_type = Some(grass);
    g.map.plots[to].terrain_type = Some(grass);
    if s.forest {
        g.map.plots[to].feature_type = Some(d.t::<FeatureType>("FEATURE_FOREST"));
    }
    if s.hills {
        g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    }
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    g.players[p1].human = true; // the battle log is written for a human's battle
    if s.city {
        let (x, y) = g.map.xy(to);
        let c = g.player_init_city(p1, x, y, true, true);
        g.city_set_population(c, 6);
        if s.walls {
            let walls = d.t::<BuildingType>("BUILDING_WALLS");
            let class = d.buildings[walls].building_class_type.unwrap();
            g.cities[c].building_tiers[class] = BuildingTierType::STANDARD;
            g.city_set_num_real_building(c, walls, 1);
        }
    }
    let mut first = None;
    let mut rows = Vec::new();
    for (owner, at, stacks) in [(p0, from, &s.att), (p1, to, &s.def)] {
        let (x, y) = g.map.xy(at);
        for st in stacks {
            for _ in 0..st.n {
                let u = g.player_init_unit(owner, st.ty, x, y, None, None);
                let own = &d.units[st.ty].free_promotions;
                for p in d.promotions.keys() {
                    if g.units[u].has_promotion[p] && !own[p] {
                        g.unit_set_has_promotion(u, p, false);
                    }
                }
                for &p in &st.promos {
                    g.unit_set_has_promotion(u, p, true);
                }
                if owner == p1 && s.fort > 0 {
                    g.unit_set_fortify_turns(u, s.fort);
                }
                g.units[u].damage = s.dmg;
                if owner == p0 && first.is_none() {
                    first = Some(u);
                }
                if let (true, Some(r)) = (owner == p0, st.row) {
                    rows.push((u, r));
                }
            }
        }
        g.player_army_sync(owner);
    }
    (g, from, to, first.expect("an attacker"), rows)
}

/// The attacker's auto formation with `rows` applied (module comment).
fn formation_with(g: &Game, from: PlotIdx, to: PlotIdx, rows: &[(UnitId, u8)]) -> Vec<Place> {
    let ctx = HitCtx::new(from, to);
    let slots = g.engage_slots(&ctx);
    let pool = g.engage_attackers(PlayerId(0), from, to, WarTest::AtWar);
    let mut f = g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
    let lanes = g.engage_lanes(&slots)[0];
    let centre = |l: u8| (2 * l as i32 - (lanes as i32 - 1)).abs();
    for &(u, r) in rows {
        let Some(i) = f.iter().position(|p| p.unit == u) else {
            continue;
        };
        if f[i].row == r || g.unit_engage_large(u) {
            continue;
        }
        let taken = |f: &[Place], l: u8| {
            f.iter().any(|p| {
                p.row == r && (p.lane == l || (g.unit_engage_large(p.unit) && p.lane + 1 == l))
            })
        };
        let mut ls: Vec<u8> = (0..lanes).collect();
        ls.sort_by_key(|&l| (centre(l), l));
        if let Some(&l) = ls.iter().find(|&&l| !taken(&f, l)) {
            f[i].lane = l;
            f[i].row = r;
        } else if let Some(j) = f.iter().position(|p| {
            p.row == r
                && !g.unit_engage_large(p.unit)
                && !rows.iter().any(|&(v, rr)| v == p.unit && rr == r)
        }) {
            let (l0, r0) = (f[i].lane, f[i].row);
            f[i].lane = f[j].lane;
            f[i].row = r;
            f[j].lane = l0;
            f[j].row = r0;
        }
    }
    f
}

#[derive(Default, Clone)]
struct TypeAgg {
    /// Fighter-battles committed, fell, withdrew, sat out.
    n: i64,
    fell: i64,
    withdrew: i64,
    sat: i64,
    /// Centi-HP: max HP, dealt, taken, healed.
    max_hp: i64,
    dealt: i64,
    taken: i64,
    healed: i64,
    actions: i64,
    swings: i64,
    misses: i64,
    crits: i64,
    kills: i64,
    /// Battles where it was in the formation, by the row it started in.
    rows: [i64; 3],
}

#[derive(Default)]
struct SideAgg {
    committed: i64,
    fallen: i64,
    escaped: i64,
    max_hp: i64,
    taken: i64,
    cost: i64,
    cost_lost: i64,
    def_strike_dmg: i64,
}

#[derive(Default)]
struct Agg {
    runs: i64,
    waves: i64,
    /// Turns of siege before the assault (`bombard:N` and the AI's bombards).
    siege_turns: i64,
    /// Per run, from the attacker's side: the defender's share of HP lost less its own (percent),
    /// and the hammers of HP it destroyed less those it lost; sums and sums of squares.
    net_hp: [f64; 2],
    net_cw: [f64; 2],
    results: [i64; 4],
    rounds: i64,
    sides: [SideAgg; 2],
    types: [BTreeMap<UnitType, TypeAgg>; 2],
    by_kind: [BTreeMap<String, i64>; 2],
    reserve: [i64; 2],
    slots: [i64; 2],
    lanes: [i64; 2],
}

fn fight(spec: &Spec, runs: i64, seed: u32) -> Agg {
    let d = ffh_data::global();
    let (g0, from, to, first, rows) = stage(spec);
    let bytes = g0.save();
    let mut a = Agg::default();
    let (tx, ty) = g0.map.xy(to);
    for k in 0..runs {
        let mut g = Game::load(d, &bytes).expect("a save of this build");
        g.soren_rand = Rng::new(
            seed.wrapping_add(k as u32)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(17),
        );
        for _ in 0..spec.bombard {
            siege_turn(&mut g, from, true);
        }
        let mut siege_turns = spec.bombard as i64;
        let mut reports: Vec<BattleReport> = Vec::new();
        while reports.len() < spec.waves {
            let ready = g.map.plots[from]
                .units
                .iter()
                .copied()
                .find(|&u| g.units[u].owner == PlayerId(0) && !g.units[u].made_attack);
            let Some(lead) = ready.or(if reports.is_empty() {
                Some(first)
            } else {
                None
            }) else {
                break;
            };
            let before = g.battle_report_next;
            let grp = g.units[lead].group.expect("a group");
            let formation = if rows.is_empty() {
                None
            } else {
                Some(formation_with(&g, from, to, &rows))
            };
            if let Err(e) = g.group_attack_force(grp, tx, ty, MOVE_DIRECT_ATTACK, formation) {
                eprintln!("{}: refused: {e}", spec.name);
                break;
            }
            let Some(r) = g.battle_reports.iter().find(|r| r.id >= before) else {
                // An AI stack with siege units bombards instead while a Sentinel stands
                // (`group_attack_force`'s bombard-first): another siege turn, up to 10 in all.
                if spec.bombard > 0 && siege_turns < 10 && g.units[lead].made_attack {
                    siege_turn(&mut g, from, false);
                    siege_turns += 1;
                    continue;
                }
                break;
            };
            let taken = r.outcome.result == ffh_core::engagement::EngageResult::Taken;
            reports.push(r.clone());
            if taken {
                break;
            }
        }
        if reports.is_empty() {
            eprintln!("{}: no engagement", spec.name);
            continue;
        }
        a.waves += reports.len() as i64;
        a.siege_turns += siege_turns;
        a.runs += 1;
        let last = reports.last().unwrap();
        use ffh_core::engagement::EngageResult as R;
        a.results[match last.outcome.result {
            R::Taken => 0,
            R::Cleared => 1,
            R::Repulsed => 2,
            R::Held => 3,
        }] += 1;
        for r in &reports {
            add_report(&mut a, r, &spec.name);
        }
        let (hp, cw) = run_net(&reports);
        a.net_hp[0] += hp;
        a.net_hp[1] += hp * hp;
        a.net_cw[0] += cw;
        a.net_cw[1] += cw * cw;
    }
    a
}

/// A turn of siege: the attackers on `from` that can bombard do (when `bombard`), then every
/// attacker's attacks and moves are reset for the next turn.
fn siege_turn(g: &mut Game, from: PlotIdx, bombard: bool) {
    let ours: Vec<UnitId> = g.map.plots[from].units.iter().copied().filter(|&u| g.units[u].owner == PlayerId(0)).collect();
    if bombard {
        for &u in &ours {
            if g.units.contains(u) && g.unit_can_bombard(u, from) {
                g.unit_bombard(u);
            }
        }
    }
    for &u in &ours {
        if g.units.contains(u) {
            g.units[u].made_attack = false;
            g.units[u].attacks_made = 0;
            g.units[u].moves = 0;
        }
    }
}

/// One run's net result from the attacker's side (`Agg::net_hp`, `Agg::net_cw`). A side's HP lost is
/// net of its healers' heals (the cost-weighted figure is not).
fn run_net(reports: &[BattleReport]) -> (f64, f64) {
    let d = ffh_data::global();
    let (mut taken, mut max_hp, mut cw) = ([0f64; 2], [0f64; 2], [0f64; 2]);
    for log in reports.iter().filter_map(|r| r.log.as_ref()) {
        for f in log.fighters.iter().filter(|f| f.place.is_some()) {
            let s = f.side.index();
            // Its side's net loss: what its units took before the battle (a siege's bombards; the
            // lab's units start whole) and in it, less what its healers healed them.
            let before = (100 * f.max_hp - f.hp_start).max(0);
            taken[s] += (before + f.taken - f.healed) as f64;
            max_hp[s] += 100.0 * f.max_hp as f64;
            if !f.sentinel && f.max_hp > 0 {
                cw[s] += f.taken as f64 / (100.0 * f.max_hp as f64) * d.units[f.unit_type].production_cost as f64;
            }
        }
    }
    let frac = |s: usize| if max_hp[s] > 0.0 { 100.0 * taken[s] / max_hp[s] } else { 0.0 };
    (frac(1) - frac(0), cw[1] - cw[0])
}

/// Adds one engagement's battle log to the totals.
fn add_report(a: &mut Agg, r: &BattleReport, name: &str) {
    let d = ffh_data::global();
    let Some(log) = r.log.as_ref() else {
        eprintln!("{name}: no battle log");
        return;
    };
    for s in 0..2 {
        a.slots[s] += r.sides[s].slots as i64;
        a.lanes[s] += r.sides[s].lanes as i64;
    }
    for f in &log.fighters {
        let s = f.side.index();
        if f.place.is_none() {
            a.reserve[s] += 1;
            continue;
        }
        let cost = if f.sentinel {
            0
        } else {
            d.units[f.unit_type].production_cost as i64
        };
        let t = a.types[s].entry(f.unit_type).or_default();
        t.n += 1;
        t.max_hp += f.max_hp as i64;
        t.dealt += f.dealt as i64;
        t.taken += f.taken as i64;
        t.healed += f.healed as i64;
        if let Some((_, row)) = f.place {
            t.rows[(row as usize).min(2)] += 1;
        }
        let side = &mut a.sides[s];
        side.committed += 1;
        side.max_hp += f.max_hp as i64;
        side.taken += f.taken as i64;
        side.cost += cost;
        match f.fate {
            Some(LogFate::Fell { .. }) => {
                t.fell += 1;
                side.fallen += 1;
                side.cost_lost += cost;
            }
            Some(LogFate::Withdrew { .. }) => {
                t.withdrew += 1;
                side.escaped += 1;
            }
            Some(LogFate::SatOut) => t.sat += 1,
            _ => {}
        }
    }
    for e in &log.events {
        match e {
            LogEvent::Action(ev) => {
                let actor = &log.fighters[ev.actor as usize];
                let s = actor.side.index();
                let kind = format!("{:?}", ev.kind);
                let mut dmg = 0i64;
                let t = a.types[s].entry(actor.unit_type).or_default();
                t.actions += 1;
                for (j, h) in ev.hits.iter().enumerate() {
                    dmg += h.dmg as i64;
                    if j == 0 && !h.splash {
                        t.swings += 1;
                        if h.miss {
                            t.misses += 1;
                        }
                        if h.crit {
                            t.crits += 1;
                        }
                    }
                    if h.killed {
                        t.kills += 1;
                    }
                }
                *a.by_kind[s].entry(kind).or_default() += dmg;
                if let Some(h) = &ev.heal {
                    *a.by_kind[s].entry("Heal".into()).or_default() += h.amount as i64;
                }
            }
            LogEvent::DefStrike { striker, dmg, .. } => {
                let s = log.fighters[*striker as usize].side.index();
                a.sides[s].def_strike_dmg += *dmg as i64;
            }
            LogEvent::End { rounds, .. } => a.rounds += *rounds as i64,
            _ => {}
        }
    }
}

fn json(spec: &Spec, a: &Agg) -> String {
    let d = ffh_data::global();
    let mut s = String::new();
    let _ = write!(
        s,
        "{{\"name\":{:?},\"runs\":{},\"waves\":{},\"results\":{:?},\"rounds\":{},\"siege_turns\":{},\"net_hp\":[{:.3},{:.3}],\"net_cw\":[{:.3},{:.3}]",
        spec.name, a.runs, a.waves, a.results, a.rounds, a.siege_turns, a.net_hp[0], a.net_hp[1], a.net_cw[0], a.net_cw[1]
    );
    let _ = write!(
        s,
        ",\"slots\":{:?},\"lanes\":{:?},\"reserve\":{:?}",
        a.slots, a.lanes, a.reserve
    );
    s.push_str(",\"sides\":[");
    for k in 0..2 {
        let x = &a.sides[k];
        if k > 0 {
            s.push(',');
        }
        let _ = write!(
            s,
            "{{\"committed\":{},\"fallen\":{},\"escaped\":{},\"max_hp\":{},\"taken\":{},\"cost\":{},\"cost_lost\":{},\"def_strike_dmg\":{},\"by_kind\":{{",
            x.committed, x.fallen, x.escaped, x.max_hp, x.taken, x.cost, x.cost_lost, x.def_strike_dmg
        );
        for (i, (kind, v)) in a.by_kind[k].iter().enumerate() {
            let _ = write!(s, "{}{:?}:{}", if i > 0 { "," } else { "" }, kind, v);
        }
        s.push_str("},\"types\":{");
        for (i, (ty, t)) in a.types[k].iter().enumerate() {
            let _ = write!(
                s,
                "{}{:?}:{{\"n\":{},\"fell\":{},\"withdrew\":{},\"sat\":{},\"max_hp\":{},\"dealt\":{},\"taken\":{},\"healed\":{},\"actions\":{},\"swings\":{},\"misses\":{},\"crits\":{},\"kills\":{},\"rows\":{:?},\"cost\":{}}}",
                if i > 0 { "," } else { "" },
                d.type_name(*ty),
                t.n,
                t.fell,
                t.withdrew,
                t.sat,
                t.max_hp,
                t.dealt,
                t.taken,
                t.healed,
                t.actions,
                t.swings,
                t.misses,
                t.crits,
                t.kills,
                t.rows,
                d.units[*ty].production_cost
            );
        }
        s.push_str("}}");
    }
    s.push_str("]}");
    s
}

pub fn cmd_battles(args: &[String]) {
    let usage = || -> ! {
        eprintln!("usage: ffh battles FILE [--runs N] [--seed S]");
        std::process::exit(2)
    };
    let mut file = None;
    let (mut runs, mut seed) = (200i64, 1u32);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| usage());
                i += 2;
            }
            "--seed" => {
                seed = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| usage());
                i += 2;
            }
            f if file.is_none() && !f.starts_with("--") => {
                file = Some(f.to_string());
                i += 1;
            }
            _ => usage(),
        }
    }
    let d = ffh_data::global();
    let text = std::fs::read_to_string(file.unwrap_or_else(|| usage())).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let mut specs = Vec::new();
    for (n, line) in text.lines().enumerate() {
        match parse_line(d, line) {
            Ok(Some(s)) => specs.push(s),
            Ok(None) => {}
            Err(e) => {
                eprintln!("line {}: {e}", n + 1);
                std::process::exit(2)
            }
        }
    }
    for s in &specs {
        let a = fight(s, runs, seed);
        println!("{}", json(s, &a));
    }
}
