//! Branching a game at a decision (`ffh save` / `ffh resume`): the first step in judging an AI
//! behaviour (CLAUDE.md, "AI changes"; docs/AI.md, "Tools"). A game is played to a turn and saved;
//! the save is resumed twice, the behaviour on and off (its switch), and the local facts it should
//! change are printed turn by turn for both. Up to the fork the two branches are the same game --
//! same map, same opponents, same random-number state -- so the difference is that one decision's
//! (and what follows from it).
//!
//!   ffh save FILE --to-turn T [run options]
//!       play the game the run options describe (as `ffh stats` does) to the start of turn T and
//!       write it to FILE;
//!   ffh resume FILE --turns N [--watch SPEC]... [--reseed K] [--summary]
//!       load FILE, give every player the AI (fresh, as after any load), play N turns and print
//!       one line per watch and turn. SPEC is `city:X,Y` (the city there: owner, size, defenders
//!       and their health, defense damage), `player:P` (cities, units, army, power, research,
//!       gold, wars), `area:X,Y,R` (every player's units within R plots: count, health, types) or
//!       `unit:ID` (alive?, where, health, experience).
//!       `--reseed K` (an integer) replaces the game RNG's state after loading by a hash of that
//!       state and K, so that K = 0, 1, 2.. play different, reproducible continuations of the
//!       same position: combat and every other die roll part ways at once. Only `soren_rand` is
//!       touched (`map_rand` serves map generation only). Without it the save plays on exactly
//!       as the game it was saved from.
//!       `--summary` prints, instead of the lines, one JSON line at the end: `turn`,
//!       `fork_turn`, `reseed`, `checksum`, `game_on` and `values`, a flat object
//!       `"SPEC FIELD": value` of each watch's final values (numbers, strings, booleans); a
//!       `player:` watch also has `d_FIELD`, the change since the fork.
//!
//! `tools/ai-branch.sh` runs the pair and shows them side by side; with `--rolls N` it resumes
//! each branch N times (`--reseed 1..N --summary`) and tabulates the distributions
//! (`tools/ai-branch-rolls.py`).

use std::sync::Arc;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::mapscript::FractalMapScript;
use ffh_data::*;

use crate::run;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(2)
}

/// `ffh save FILE --to-turn T [run options]`.
pub fn cmd_save(args: &[String], parse: impl Fn(&[String]) -> run::RunConfig) {
    let file = args.first().unwrap_or_else(|| fail("usage: ffh save FILE --to-turn T [run options]"));
    let mut rest: Vec<String> = Vec::new();
    let mut to_turn = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--to-turn" {
            to_turn = args.get(i + 1).and_then(|v| v.parse::<i32>().ok());
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }
    let to_turn = to_turn.unwrap_or_else(|| fail("ffh save: --to-turn T is required"));
    let mut cfg = parse(&rest);
    let (mut g, _) = run::new_game(&cfg);
    cfg.turns = to_turn - g.game_turn();
    cfg.until_supremacy = false;
    run::play(&mut g, &cfg, |_, _| {});
    // The safe point: no player's turn active (where `play` stops anyway).
    while g.game_state == GameStateType::GAMESTATE_ON && !g.at_turn_boundary() {
        g.update();
    }
    if g.game_turn() != to_turn {
        fail(format!("ffh save: the game ended at turn {} (before {to_turn})", g.game_turn()));
    }
    std::fs::write(file, g.save()).unwrap_or_else(|e| fail(format!("ffh save {file}: {e}")));
    println!("saved turn {} to {file} (checksum {:08x})", g.game_turn(), g.calculate_sync_checksum() as u32);
}

/// What `--watch` prints.
enum Watch {
    City(i32, i32),
    Player(PlayerId),
    Area(i32, i32, i32),
    Unit(UnitId),
}

fn parse_watch(spec: &str) -> Watch {
    let (kind, rest) = spec.split_once(':').unwrap_or_else(|| fail(format!("--watch {spec}: KIND:ARGS")));
    let n: Vec<i32> = rest.split(',').map(|v| v.trim().parse().unwrap_or_else(|_| fail(format!("--watch {spec}: numbers")))).collect();
    match (kind, n.as_slice()) {
        ("city", [x, y]) => Watch::City(*x, *y),
        ("player", [p]) => Watch::Player(PlayerId(*p as u8)),
        ("area", [x, y, r]) => Watch::Area(*x, *y, *r),
        ("unit", [u]) => Watch::Unit(UnitId(*u as u32)),
        _ => fail(format!("--watch {spec}: city:X,Y | player:P | area:X,Y,R | unit:ID")),
    }
}

fn civ(g: &Game, p: PlayerId) -> String {
    g.player_get_civilization_type(p).map_or_else(|| format!("p{}", p.0), |c| format!("p{} {}", p.0, g.d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_")))
}

fn hp(g: &Game, u: UnitId) -> i32 {
    let max = g.d.defines.max_hit_points.max(1);
    100 - g.units[u].damage * 100 / max
}

fn unit_name(g: &Game, u: UnitId) -> &str {
    g.d.units[g.units[u].unit_type].base.type_.trim_start_matches("UNIT_")
}

fn show(g: &Game, w: &Watch) -> String {
    match *w {
        Watch::City(x, y) => {
            let Some(c) = g.map.plot_xy(x, y).and_then(|q| g.map.plots[q].plot_city) else { return format!("city {x},{y}: none") };
            let city = &g.cities[c];
            let plot = g.city_plot(c);
            let def: Vec<UnitId> = g.map.plots[plot].units.iter().copied().filter(|&u| g.units[u].owner == city.owner && g.d.units[g.units[u].unit_type].combat > 0).collect();
            let health = if def.is_empty() { 0 } else { def.iter().map(|&u| hp(g, u)).sum::<i32>() / def.len() as i32 };
            format!("city {x},{y}: {} size {} defenders {} (health {}%) defense damage {}", civ(g, city.owner), city.population, def.len(), health, city.defense_damage)
        }
        Watch::Player(p) => {
            if p.idx() >= g.players.len() {
                return format!("player {}: none", p.0);
            }
            let pl = &g.players[p];
            if !pl.alive {
                return format!("{}: dead", civ(g, p));
            }
            let army = pl.units.iter().filter(|&&u| g.d.units[g.units[u].unit_type].combat > 0).count();
            let team = pl.team;
            let wars: Vec<String> = g.team_ids().into_iter().filter(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)).map(|t| format!("t{}", t.0)).collect();
            format!(
                "{}: cities {} units {} army {} power {} research {} gold {} wars [{}]",
                civ(g, p),
                pl.cities.len(),
                pl.units.len(),
                army,
                pl.power,
                g.player_calculate_research_rate(p, None),
                pl.gold,
                wars.join(" ")
            )
        }
        Watch::Area(x, y, r) => {
            let mut by: std::collections::BTreeMap<PlayerId, Vec<UnitId>> = std::collections::BTreeMap::new();
            for dx in -r..=r {
                for dy in -r..=r {
                    let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                    for &u in &g.map.plots[q].units {
                        by.entry(g.units[u].owner).or_default().push(u);
                    }
                }
            }
            let parts: Vec<String> = by
                .iter()
                .map(|(&p, us)| {
                    let health = us.iter().map(|&u| hp(g, u)).sum::<i32>() / us.len().max(1) as i32;
                    let mut kinds: std::collections::BTreeMap<&str, i32> = std::collections::BTreeMap::new();
                    for &u in us {
                        *kinds.entry(unit_name(g, u)).or_insert(0) += 1;
                    }
                    let kinds: Vec<String> = kinds.iter().map(|(k, n)| if *n > 1 { format!("{k} x{n}") } else { k.to_string() }).collect();
                    format!("{} {}u {}% [{}]", civ(g, p), us.len(), health, kinds.join(", "))
                })
                .collect();
            format!("area {x},{y},{r}: {}", if parts.is_empty() { "empty".to_string() } else { parts.join("; ") })
        }
        Watch::Unit(u) => {
            if !g.units.contains(u) {
                return format!("unit {}: gone", u.0);
            }
            let unit = &g.units[u];
            format!("unit {} {} of {}: at {},{} health {}% xp {} level {}", u.0, unit_name(g, u), civ(g, unit.owner), unit.x, unit.y, hp(g, u), unit.experience, unit.level)
        }
    }
}

/// A watched value in `--summary`.
enum Val {
    Num(i64),
    Str(String),
    Bool(bool),
}

impl Val {
    fn json(&self) -> String {
        match self {
            Val::Num(n) => n.to_string(),
            Val::Bool(b) => b.to_string(),
            Val::Str(s) => json_str(s),
        }
    }
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn civ_name(g: &Game, p: PlayerId) -> String {
    g.player_get_civilization_type(p).map_or_else(String::new, |c| g.d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_").to_string())
}

/// The players with units in an `area:` watch now (none for other watches).
fn area_owners(g: &Game, w: &Watch) -> Vec<PlayerId> {
    let Watch::Area(x, y, r) = *w else { return Vec::new() };
    let mut ps: Vec<PlayerId> = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            for &u in &g.map.plots[q].units {
                let o = g.units[u].owner;
                if !ps.contains(&o) {
                    ps.push(o);
                }
            }
        }
    }
    ps.sort();
    ps
}

/// The values of a watch for `--summary` (the facts `show` prints, as values). `area_players`:
/// the players an `area:` watch reports even with no unit there (those present at the fork).
fn facts(g: &Game, w: &Watch, area_players: &[PlayerId]) -> Vec<(String, Val)> {
    use Val::*;
    let mut v: Vec<(String, Val)> = Vec::new();
    match *w {
        Watch::City(x, y) => {
            let c = g.map.plot_xy(x, y).and_then(|q| g.map.plots[q].plot_city);
            v.push(("exists".into(), Bool(c.is_some())));
            if let Some(c) = c {
                let city = &g.cities[c];
                let plot = g.city_plot(c);
                let def: Vec<UnitId> = g.map.plots[plot].units.iter().copied().filter(|&u| g.units[u].owner == city.owner && g.d.units[g.units[u].unit_type].combat > 0).collect();
                v.push(("owner".into(), Str(format!("p{}", city.owner.0))));
                v.push(("owner_civ".into(), Str(civ_name(g, city.owner))));
                v.push(("size".into(), Num(city.population as i64)));
                v.push(("defenders".into(), Num(def.len() as i64)));
                v.push(("defender_health".into(), Num(def.iter().map(|&u| hp(g, u) as i64).sum())));
            } else {
                v.push(("owner".into(), Str("none".into())));
                v.push(("owner_civ".into(), Str("none".into())));
                v.push(("size".into(), Num(0)));
                v.push(("defenders".into(), Num(0)));
                v.push(("defender_health".into(), Num(0)));
            }
        }
        Watch::Player(p) => {
            if p.idx() >= g.players.len() {
                return v;
            }
            let pl = &g.players[p];
            let alive = pl.alive;
            let army = pl.units.iter().filter(|&&u| g.d.units[g.units[u].unit_type].combat > 0).count();
            let team = pl.team;
            let wars = if alive { g.team_ids().filter(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)).count() } else { 0 };
            v.push(("alive".into(), Bool(alive)));
            v.push(("cities".into(), Num(pl.cities.len() as i64)));
            v.push(("units".into(), Num(pl.units.len() as i64)));
            v.push(("army".into(), Num(army as i64)));
            v.push(("power".into(), Num(if alive { pl.power as i64 } else { 0 })));
            v.push(("research".into(), Num(if alive { g.player_calculate_research_rate(p, None) as i64 } else { 0 })));
            v.push(("gold".into(), Num(if alive { pl.gold as i64 } else { 0 })));
            v.push(("wars".into(), Num(wars as i64)));
        }
        Watch::Area(x, y, r) => {
            let mut by: std::collections::BTreeMap<PlayerId, (i64, i64)> = area_players.iter().map(|&p| (p, (0, 0))).collect();
            for dx in -r..=r {
                for dy in -r..=r {
                    let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                    for &u in &g.map.plots[q].units {
                        let e = by.entry(g.units[u].owner).or_default();
                        e.0 += 1;
                        e.1 += hp(g, u) as i64;
                    }
                }
            }
            for (p, (n, h)) in by {
                v.push((format!("p{} units", p.0), Num(n)));
                v.push((format!("p{} health", p.0), Num(h)));
            }
        }
        Watch::Unit(u) => {
            let alive = g.units.contains(u);
            v.push(("alive".into(), Bool(alive)));
            if alive {
                let unit = &g.units[u];
                v.push(("plot".into(), Str(format!("{},{}", unit.x, unit.y))));
                v.push(("health".into(), Num(hp(g, u) as i64)));
                v.push(("experience".into(), Num(unit.experience as i64)));
            } else {
                v.push(("plot".into(), Str("gone".into())));
                v.push(("health".into(), Num(0)));
            }
        }
    }
    v
}

/// `--reseed K`: the game RNG's new state, a hash (splitmix64's finalizer) of its state and K.
/// Deterministic; different K give different states.
fn reseeded(seed: u32, k: u64) -> u32 {
    let mut z = ((seed as u64) << 32 | 0x5eed).wrapping_add(k.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 32) as u32
}

/// `ffh resume FILE --turns N [--watch SPEC]... [--reseed K] [--summary]`.
pub fn cmd_resume(args: &[String]) {
    let file = args.first().unwrap_or_else(|| fail("usage: ffh resume FILE --turns N [--watch SPEC]... [--reseed K] [--summary]"));
    let mut turns = 10;
    let mut specs: Vec<String> = Vec::new();
    let mut watches = Vec::new();
    let mut reseed: Option<u64> = None;
    let mut summary = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--summary" {
            summary = true;
            i += 1;
            continue;
        }
        let val = args.get(i + 1).cloned().unwrap_or_else(|| fail(format!("{}: a value", args[i])));
        match args[i].as_str() {
            "--turns" => turns = val.parse().unwrap_or_else(|_| fail("--turns N")),
            "--watch" => {
                watches.push(parse_watch(&val));
                specs.push(val);
            }
            "--reseed" => reseed = Some(val.parse().unwrap_or_else(|_| fail("--reseed K: a non-negative integer"))),
            o => fail(format!("ffh resume: unknown option {o}")),
        }
        i += 2;
    }
    let d = ffh_data::global();
    let bytes = std::fs::read(file).unwrap_or_else(|e| fail(format!("ffh resume {file}: {e}")));
    let mut g = Game::load(d, &bytes).unwrap_or_else(|e| fail(format!("ffh resume {file}: cannot load: {e} (a save loads only with builds of its save::FORMAT_VERSION)")));
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    if std::env::var("FFH_LOG").is_ok() {
        g.log.enabled = true;
    }
    if let Some(k) = reseed {
        let s = reseeded(g.soren_rand.seed(), k);
        g.soren_rand.reseed(s);
    }
    let fork_turn = g.game_turn();
    let area_players: Vec<Vec<PlayerId>> = watches.iter().map(|w| area_owners(&g, w)).collect();
    let at_fork: Vec<Vec<(String, Val)>> = watches.iter().zip(&area_players).map(|(w, ap)| facts(&g, w, ap)).collect();
    let print = |g: &Game| {
        if summary {
            return;
        }
        for w in &watches {
            println!("t{} {}", g.game_turn(), show(g, w));
        }
    };
    print(&g);
    let cfg = run::RunConfig { turns: 1, ..run::RunConfig::default() };
    for _ in 0..turns {
        if g.game_state != GameStateType::GAMESTATE_ON {
            break;
        }
        run::play(&mut g, &cfg, |_, _| {});
        print(&g);
    }
    if !summary {
        println!("t{} checksum {:08x}", g.game_turn(), g.calculate_sync_checksum() as u32);
        return;
    }
    let mut vals: Vec<String> = Vec::new();
    for (((spec, w), ap), fork) in specs.iter().zip(&watches).zip(&area_players).zip(&at_fork) {
        let now = facts(&g, w, ap);
        for (k, v) in &now {
            vals.push(format!("{}: {}", json_str(&format!("{spec} {k}")), v.json()));
        }
        if let Watch::Player(_) = w {
            for (k, v) in &now {
                if let (Val::Num(a), Some((_, Val::Num(b)))) = (v, fork.iter().find(|(fk, _)| fk == k)) {
                    vals.push(format!("{}: {}", json_str(&format!("{spec} d_{k}")), a - b));
                }
            }
        }
    }
    println!(
        "{{\"turn\": {}, \"fork_turn\": {fork_turn}, \"reseed\": {}, \"checksum\": \"{:08x}\", \"game_on\": {}, \"values\": {{{}}}}}",
        g.game_turn(),
        reseed.map_or_else(|| "null".to_string(), |k| k.to_string()),
        g.calculate_sync_checksum() as u32,
        g.game_state == GameStateType::GAMESTATE_ON,
        vals.join(", ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reseed_is_a_function_of_state_and_k() {
        assert_eq!(reseeded(12345, 3), reseeded(12345, 3));
        let mut seen: Vec<u32> = (0..64).map(|k| reseeded(12345, k)).collect();
        seen.push(reseeded(12346, 0));
        seen.push(12345);
        let n = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), n);
    }

    #[test]
    fn json_strings_escape() {
        assert_eq!(json_str("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    }
}
