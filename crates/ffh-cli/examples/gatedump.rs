//! Throwaway: hashes the answers of the rules' yes/no gates over a wide sweep of inputs during
//! an AI game, so a refactor of the gates can be checked for unchanged answers.
//!
//!   gatedump --seed N --civs 6 --turns T --every K
//!
//! Plays the game `ffh autoplay` plays with the same arguments and, every K turns and at the
//! end, prints `turn gate count_true count_total hash` per gate, then `TOTAL <hash>`.
//! A gate that panics answers 2 in the hash (and is counted on stderr).

#![allow(dead_code)]

#[path = "../src/run.rs"]
mod run;

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::EventTriggeredData;
use ffh_data::*;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv_bytes(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(FNV_PRIME);
    }
}

thread_local! {
    static IN_GATE: Cell<bool> = const { Cell::new(false) };
}

fn guarded<T>(f: impl FnOnce() -> T) -> std::thread::Result<T> {
    IN_GATE.with(|c| c.set(true));
    let r = catch_unwind(AssertUnwindSafe(f));
    IN_GATE.with(|c| c.set(false));
    r
}

/// One gate's accumulated answers.
struct Acc {
    name: &'static str,
    hash: u64,
    trues: u64,
    total: u64,
    panics: u64,
}

impl Acc {
    fn new(name: &'static str) -> Acc {
        Acc { name, hash: FNV_OFFSET, trues: 0, total: 0, panics: 0 }
    }
    fn b(&mut self, f: impl FnOnce() -> bool) {
        let v = match guarded(f) {
            Ok(true) => 1u8,
            Ok(false) => 0u8,
            Err(_) => {
                self.panics += 1;
                2u8
            }
        };
        fnv_bytes(&mut self.hash, &[v]);
        self.total += 1;
        if v == 1 {
            self.trues += 1;
        }
    }
    fn i(&mut self, f: impl FnOnce() -> i32) {
        match guarded(f) {
            Ok(v) => {
                fnv_bytes(&mut self.hash, &[0]);
                fnv_bytes(&mut self.hash, &v.to_le_bytes());
                if v > 0 {
                    self.trues += 1;
                }
            }
            Err(_) => {
                self.panics += 1;
                fnv_bytes(&mut self.hash, &[2]);
            }
        }
        self.total += 1;
    }
}

struct Accs(Vec<Acc>);

impl Accs {
    fn get(&mut self, name: &'static str) -> &mut Acc {
        if let Some(i) = self.0.iter().position(|a| a.name == name) {
            return &mut self.0[i];
        }
        self.0.push(Acc::new(name));
        self.0.last_mut().unwrap()
    }
}

fn alive_players(g: &Game) -> Vec<PlayerId> {
    g.player_ids().filter(|&p| g.players[p].alive).collect()
}

/// Up to `n` cities of `p`: the capital first, then others in list order.
fn some_cities(g: &Game, p: PlayerId, n: usize) -> Vec<CityId> {
    let mut v = Vec::new();
    if let Some(c) = g.player_get_capital_city(p) {
        v.push(c);
    }
    for &c in &g.players[p].cities {
        if v.len() >= n {
            break;
        }
        if g.cities.contains(c) && !v.contains(&c) {
            v.push(c);
        }
    }
    v
}

fn sweep(g: &Game, turn: i32, total: &mut u64) {
    let d = g.d;
    let mut accs = Accs(Vec::new());
    let players = alive_players(g);
    let units: Vec<UnitId> = g.units.ids();
    let nplots = g.map.plots.len();

    // --- spells
    for &u in &units {
        for s in d.spells.keys() {
            for tv in [false, true] {
                accs.get("unit_can_cast").b(|| g.unit_can_cast(u, s, tv));
            }
            accs.get("py_can_cast").b(|| g.py_can_cast(u, s));
            accs.get("unit_can_create_unit").b(|| g.unit_can_create_unit(u, s));
            accs.get("unit_can_create_building").b(|| g.unit_can_create_building(u, s));
        }
    }

    // --- builds and improvements over every plot
    let mut teams: Vec<Option<TeamId>> = vec![None];
    for &p in &players {
        let t = Some(g.players[p].team);
        if !teams.contains(&t) {
            teams.push(t);
        }
    }
    for i in 0..nplots {
        let plot = PlotIdx(i as u32);
        for b in d.builds.keys() {
            for &p in &players {
                for tv in [false, true] {
                    accs.get("plot_can_build").b(|| g.plot_can_build(plot, b, p, tv));
                }
            }
        }
        for imp in d.improvements.keys() {
            for &t in &teams {
                for pot in [false, true] {
                    accs.get("plot_can_have_improvement").b(|| g.plot_can_have_improvement(plot, imp, t, pot));
                }
            }
        }
    }

    // --- unit gates. Plot-free gates once per unit; the plot-taking gates at the unit's own
    // plot, its 8 neighbours and every city plot on the map (in that order, no duplicates).
    let city_plots: Vec<PlotIdx> = g.cities.ids().into_iter().map(|c| g.city_plot(c)).collect();
    for &u in &units {
        for pr in d.promotions.keys() {
            accs.get("unit_can_acquire_promotion").b(|| g.unit_can_acquire_promotion(u, pr));
        }
        for tv in [false, true] {
            accs.get("unit_can_upgrade").b(|| g.unit_can_upgrade(u, None, tv));
            for ut in d.units.keys() {
                accs.get("unit_can_upgrade").b(|| g.unit_can_upgrade(u, Some(ut), tv));
            }
            for tt in [false, true] {
                accs.get("unit_can_gift").b(|| g.unit_can_gift(u, tv, tt));
            }
        }
        let own = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut plots = vec![own];
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(q) = g.map.plot_xy(ux + dx, uy + dy) {
                    if !plots.contains(&q) {
                        plots.push(q);
                    }
                }
            }
        }
        for &q in &city_plots {
            if !plots.contains(&q) {
                plots.push(q);
            }
        }
        for &plot in &plots {
            for b in d.builds.keys() {
                for tv in [false, true] {
                    accs.get("unit_can_build").b(|| g.unit_can_build(u, plot, b, tv));
                }
            }
            for tv in [false, true] {
                accs.get("unit_can_spread").b(|| g.unit_can_spread(u, plot, None, tv));
                for r in d.religions.keys() {
                    accs.get("unit_can_spread").b(|| g.unit_can_spread(u, plot, Some(r), tv));
                }
                accs.get("unit_can_construct").b(|| g.unit_can_construct(u, plot, None, tv));
                for bl in d.buildings.keys() {
                    accs.get("unit_can_construct").b(|| g.unit_can_construct(u, plot, Some(bl), tv));
                }
                accs.get("unit_can_hurry").b(|| g.unit_can_hurry(u, plot, tv));
                accs.get("unit_can_trade").b(|| g.unit_can_trade(u, plot, tv));
                accs.get("unit_can_golden_age").b(|| g.unit_can_golden_age(u, plot, tv));
                accs.get("unit_can_found").b(|| g.unit_can_found(u, plot, tv));
                accs.get("unit_can_plunder").b(|| g.unit_can_plunder(u, plot, tv));
            }
            accs.get("unit_can_join").b(|| g.unit_can_join(u, plot, None));
            for sp in d.specialists.keys() {
                accs.get("unit_can_join").b(|| g.unit_can_join(u, plot, Some(sp)));
            }
            accs.get("unit_can_bombard").b(|| g.unit_can_bombard(u, plot));
            accs.get("unit_can_discover").b(|| g.unit_can_discover(u, plot));
            accs.get("unit_can_great_work").b(|| g.unit_can_great_work(u, plot));
            accs.get("unit_can_pillage").b(|| g.unit_can_pillage(u, plot));
            accs.get("unit_can_heal").b(|| g.unit_can_heal(u, plot));
            accs.get("unit_can_sentry").b(|| g.unit_can_sentry(u, plot));
            accs.get("unit_can_fortify").b(|| g.unit_can_fortify(u, plot));
            accs.get("unit_can_sleep").b(|| g.unit_can_sleep(u, plot));
            accs.get("unit_can_hold").b(|| g.unit_can_hold(u, plot));
            accs.get("unit_can_load").b(|| g.unit_can_load(u, plot));
            accs.get("unit_can_lead").i(|| g.unit_can_lead(u, plot, None));
            for &o in &g.map.plots[plot.0 as usize].units {
                if o != u {
                    accs.get("unit_can_lead").i(|| g.unit_can_lead(u, plot, Some(o)));
                }
            }
        }
    }

    // --- events
    let mut triggers_of: IdxVec<EventType, Vec<EventTriggerType>> = IdxVec::from_fn(d.events.len(), |_| Vec::new());
    for (t, info) in d.event_triggers.iter_enumerated() {
        for &e in &info.events {
            if !triggers_of[e].contains(&t) {
                triggers_of[e].push(t);
            }
        }
    }
    let religions: Vec<Option<ReligionType>> = std::iter::once(None).chain(d.religions.keys().map(Some)).collect();
    let corps: Vec<Option<CorporationType>> = std::iter::once(None).chain(d.corporations.keys().map(Some)).collect();
    let buildings: Vec<BuildingType> = d.buildings.keys().collect();
    // The per-entity apply gates take only the entity and the event: every plot and every unit.
    for e in d.events.keys() {
        for i in 0..nplots {
            accs.get("plot_can_apply_event").b(|| g.plot_can_apply_event(PlotIdx(i as u32), e));
        }
        for &u in &units {
            accs.get("unit_can_apply_event").b(|| g.unit_can_apply_event(u, e));
        }
    }
    let mut cyc: usize = 0;
    for &p in &players {
        let cities = some_cities(g, p, 4);
        let p_units: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units.contains(u)).take(2).collect();
        let others: Vec<PlayerId> = players.iter().copied().filter(|&o| o != p).collect();
        let city_opts: Vec<Option<CityId>> = if cities.is_empty() { vec![None] } else { cities.iter().copied().map(Some).collect() };
        let unit_opts: Vec<Option<UnitId>> = if p_units.is_empty() { vec![None] } else { p_units.iter().copied().map(Some).collect() };
        let other_opts: Vec<Option<PlayerId>> = if others.is_empty() { vec![None] } else { others.iter().copied().map(Some).collect() };
        for e in d.events.keys() {
            let trigs: Vec<EventTriggerType> = if triggers_of[e].is_empty() {
                vec![EventTriggerType::from_usize(0)]
            } else {
                triggers_of[e].iter().copied().take(3).collect()
            };
            for &t in &trigs {
                for &c in &city_opts {
                    let (px, py) = match c {
                        Some(c) => (g.cities[c].x, g.cities[c].y),
                        None => (-1, -1),
                    };
                    for &u in &unit_opts {
                        for &o in &other_opts {
                            cyc += 1;
                            let data = EventTriggeredData {
                                id: TriggeredId(0),
                                trigger: t,
                                turn: g.game_turn(),
                                player: p,
                                city: c,
                                plot_x: px,
                                plot_y: py,
                                unit: u,
                                other_player: o,
                                other_player_city: o.and_then(|o| g.player_get_capital_city(o)),
                                religion: religions[cyc % religions.len()],
                                corporation: corps[(cyc / 3) % corps.len()],
                                building: if buildings.is_empty() || cyc % 4 == 0 { None } else { Some(buildings[(cyc * 7) % buildings.len()]) },
                                text: String::new(),
                                global_text: String::new(),
                            };
                            accs.get("player_can_do_event").b(|| g.player_can_do_event(p, e, &data));
                            accs.get("py_event_can_do").b(|| g.py_event_can_do(e, &data));
                            if let Some(c) = c {
                                accs.get("city_can_apply_event").b(|| g.city_can_apply_event(c, e, &data));
                            }
                        }
                    }
                }
            }
        }
    }

    for a in &accs.0 {
        println!("{} {} {} {} {:016x}", turn, a.name, a.trues, a.total, a.hash);
        if a.panics > 0 {
            eprintln!("turn {} gate {}: {} panics", turn, a.name, a.panics);
        }
        fnv_bytes(total, a.name.as_bytes());
        fnv_bytes(total, &a.trues.to_le_bytes());
        fnv_bytes(total, &a.total.to_le_bytes());
        fnv_bytes(total, &a.hash.to_le_bytes());
    }
    // The game state itself (not in TOTAL): a divergence here means the game played differently.
    println!("{} CHECKSUM units {} cities {} sync {:08x}", turn, g.units.len(), g.cities.len(), g.calculate_sync_checksum() as u32);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = run::RunConfig::default();
    let mut every = 25i32;
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--seed" => cfg.seed = v.parse().expect("--seed"),
            "--civs" => cfg.civs = v.parse().expect("--civs"),
            "--turns" => cfg.turns = v.parse().expect("--turns"),
            "--size" => cfg.world = v,
            "--every" => every = v.parse().expect("--every"),
            a => panic!("unknown argument {a}"),
        }
        i += 2;
    }
    cfg.verbose = false;
    // Silence panic messages from inside the gates (counted instead); others print as usual.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !IN_GATE.with(|c| c.get()) {
            default_hook(info);
        }
    }));

    let (mut g, _timed) = run::new_game(&cfg);
    let mut total = FNV_OFFSET;
    let mut last_sampled = -1;
    let played = run::play(&mut g, &cfg, |g, _stat| {
        let t = g.game_turn();
        if every > 0 && t % every == 0 {
            sweep(g, t, &mut total);
            last_sampled = t;
        }
    });
    if g.game_turn() != last_sampled {
        sweep(&g, g.game_turn(), &mut total);
    }
    let _ = played;
    println!("TOTAL {:016x}", total);
}
