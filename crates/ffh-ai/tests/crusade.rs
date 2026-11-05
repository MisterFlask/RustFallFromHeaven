//! The crusade AI on constructed maps (docs/CRUSADE.md): AI lenders pledge and deliver to a
//! called crusade, and the Host (its own AI player) musters, marches and takes the target's
//! city; the Host-competence measure (`host_competence_report`, ignored: it prints a win rate).

use std::sync::Arc;

use ffh_core::controller::{Controller, ControllerRef};
use ffh_core::crusade::{CrusadeId, CrusadeKind, Dissolution, Objective};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::SetupOptions;
use ffh_data::*;

/// A defender that does nothing: its units stay where they are (fortified).
struct Sit;
impl Controller for Sit {
    fn play_turn(&self, g: &mut Game, p: PlayerId) {
        for grp in g.players[p].groups.clone() {
            if g.groups.contains(grp) && g.group_ready_to_move(grp, true) {
                g.group_push_mission(grp, MissionType::MISSION_FORTIFY, -1, -1, 0, false, false, None, None, None);
                if g.group_ready_to_move(grp, true) {
                    g.group_push_mission(grp, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
                }
            }
        }
    }
}

fn play_turn(g: &mut Game) {
    for _ in 0..200_000 {
        g.update();
        if g.at_turn_boundary() || g.game_state != GameStateType::GAMESTATE_ON {
            return;
        }
    }
    panic!("turn {} does not end", g.game_turn());
}

/// Three civs on a flat 28x10 grassland: lender A at (3, 5), lender B at (3, 8), target T at
/// (14, 5). Every team has met every other; no barbarians, no events. A and B are AIs; T is
/// `defender`.
fn scenario(seed: u32, defender: ControllerRef) -> Game {
    let d = ffh_data::global();
    let mut o = SetupOptions::new(d, 3, "WORLDSIZE_DUEL", seed);
    o.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    o.options.push(GameOptionType::GAMEOPTION_NO_EVENTS);
    let mut g = Game::new_with_options(d, &o);
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.controllers[PlayerId(2)] = defender;
    g.autorun = true;
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (w, h) = (28, 10);
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: w, grid_h: h, ..Default::default() }, np, nt);
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for p in g.map.plot_ids() {
        g.plot_set_plot_type(p, PlotType::PLOT_LAND, false);
        g.plot_set_terrain_type(p, grass, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for p in g.map.plot_ids() {
            g.plot_set_revealed(p, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    for a in 0..3u8 {
        for b in 0..3u8 {
            if a != b {
                let (ta, tb) = (g.players[PlayerId(a)].team, g.players[PlayerId(b)].team);
                g.team_meet(ta, tb, false);
            }
        }
    }
    g.player_found(PlayerId(0), 3, 5);
    g.player_found(PlayerId(1), 3, 8);
    g.player_found(PlayerId(2), 14, 5);
    for c in g.cities.ids() {
        g.city_set_population(c, 4);
    }
    for t in [g.players[PlayerId(0)].team, g.players[PlayerId(1)].team] {
        for tech in d.techs.keys() {
            g.team_set_has_tech(t, tech, true, None, false, false);
        }
    }
    g
}

fn units(g: &mut Game, p: PlayerId, name: &str, n: usize, x: i32, y: i32, ai: UnitAIType) -> Vec<UnitId> {
    let t = g.d.t::<UnitType>(name);
    (0..n).map(|_| g.player_init_unit(p, t, x, y, Some(ai), None)).collect()
}

fn target_city(g: &Game) -> PlotIdx {
    g.map.plot_idx(14, 5).unwrap()
}

fn call(g: &mut Game) -> CrusadeId {
    let target = g.players[PlayerId(2)].team;
    let plot = target_city(g);
    g.crusade_call(CrusadeKind::Direct, target, None, Objective::Capture(vec![plot]), 30, 3, &[PlayerId(0), PlayerId(1)]).expect("Host slot")
}

#[test]
fn ai_lenders_deliver_and_the_host_takes_a_weakly_held_city() {
    // (the defender holds its city, so the Host must fight for it: a kill is a lender's credit)
    let mut g = scenario(5, Arc::new(Sit));
    // Lenders: a garrison each plus a field army; the target: one warrior.
    units(&mut g, PlayerId(0), "UNIT_WARRIOR", 2, 3, 5, UnitAIType::UNITAI_CITY_DEFENSE);
    let a0 = units(&mut g, PlayerId(0), "UNIT_AXEMAN", 3, 4, 5, UnitAIType::UNITAI_ATTACK);
    units(&mut g, PlayerId(1), "UNIT_WARRIOR", 2, 3, 8, UnitAIType::UNITAI_CITY_DEFENSE);
    let a1 = units(&mut g, PlayerId(1), "UNIT_AXEMAN", 3, 4, 8, UnitAIType::UNITAI_ATTACK);
    units(&mut g, PlayerId(2), "UNIT_WARRIOR", 1, 14, 5, UnitAIType::UNITAI_CITY_DEFENSE);
    let id = call(&mut g);
    let host = g.crusade(id).unwrap().host;
    // The AIs answered the call at once, each pledging its obligation or more.
    for c in &g.crusade(id).unwrap().calls {
        assert!(matches!(c.answer, Some(ffh_core::crusade::Pledge::Strength(s)) if s >= c.obligation.min(1)), "pledged: {c:?}");
    }
    // Their whole field armies serve (as a large pledge would): the lenders keep their garrisons,
    // so only the Host can take the city.
    g.crusade_lend(id, PlayerId(0), &a0).unwrap();
    g.crusade_lend(id, PlayerId(1), &a1).unwrap();
    let mut captured_by_host = false;
    for _ in 0..30 {
        play_turn(&mut g);
        if g.crusade(id).is_some() {
            let tc = g.map.plots[target_city(&g)].plot_city;
            if tc.is_some_and(|c| g.cities[c].owner == host) {
                captured_by_host = true;
            }
        } else {
            break;
        }
    }
    let report = g.crusade_reports.iter().find(|r| r.id == id).expect("the crusade dissolved");
    assert!(report.delivered > 0, "units were lent");
    assert!(captured_by_host || report.reason == Dissolution::Objective, "the Host took the city: {report:?}");
    assert_eq!(report.reason, Dissolution::Objective, "dissolved on its objective");
    // The city went to a lender in the draft (both are close enough to want it).
    let owner = g.map.plots[target_city(&g)].plot_city.map(|c| g.cities[c].owner);
    assert!(matches!(owner, Some(PlayerId(0)) | Some(PlayerId(1))), "drafted by a lender: {owner:?}");
    assert!(!g.players[host].alive, "the Host is gone");
    // The survivors are home.
    assert!(g.players[PlayerId(0)].units.len() + g.players[PlayerId(1)].units.len() > 4);
}

/// Host competence (docs/CRUSADE.md, "Self-play measures"): the Host against a scripted defender
/// (units that sit fortified in their city), over seeds and unit mixes, at 1x, 1.5x and 2x the
/// defenders' raw strength (raw strength ignores fortification and the city's defense, so 1x is
/// the defender's game). Prints the win rate (city taken within 30 turns of the call, 10 plots
/// away). `cargo test --release -p ffh-ai --test crusade -- --ignored --nocapture`.
#[test]
#[ignore]
fn host_competence_report() {
    let mixes: [(&str, &str); 4] = [("UNIT_AXEMAN", "UNIT_ARCHER"), ("UNIT_CHAMPION", "UNIT_LONGBOWMAN"), ("UNIT_HORSEMAN", "UNIT_AXEMAN"), ("UNIT_AXEMAN", "UNIT_WARRIOR")];
    for ratio in [100, 150, 200, 300] {
        let mut line = format!("{ratio}% of the defenders' strength:");
        for baseline in [false, true] {
            let (mut wins, mut games) = (0, 0);
            for (attacker, defender) in mixes {
                for seed in 1..=10u32 {
                    wins += i32::from(duel(seed, attacker, defender, ratio, baseline));
                    games += 1;
                }
            }
            let who = if baseline { "civ AI (baseline)" } else { "Host" };
            line += &format!("  {who} {wins}/{games} = {:.0}%", 100.0 * wins as f64 / games as f64);
        }
        println!("{line}");
    }
}

/// One duel: 8 attackers (4 from each lender) against fortified defenders worth `ratio`% less
/// raw strength, in the Host's hands or (`baseline`) in lender A's own (at war, no crusade).
/// True if the city fell within 30 turns.
fn duel(seed: u32, attacker: &str, defender: &str, ratio: i32, baseline: bool) -> bool {
    let mut g = scenario(seed, Arc::new(Sit));
    units(&mut g, PlayerId(0), "UNIT_WARRIOR", 1, 3, 5, UnitAIType::UNITAI_CITY_DEFENSE);
    units(&mut g, PlayerId(1), "UNIT_WARRIOR", 1, 3, 8, UnitAIType::UNITAI_CITY_DEFENSE);
    let a0 = units(&mut g, PlayerId(0), attacker, 4, 4, 5, UnitAIType::UNITAI_ATTACK);
    let a1 = if baseline { units(&mut g, PlayerId(0), attacker, 4, 4, 5, UnitAIType::UNITAI_ATTACK) } else { units(&mut g, PlayerId(1), attacker, 4, 4, 8, UnitAIType::UNITAI_ATTACK) };
    let lent: i32 = a0.iter().chain(&a1).map(|&u| g.crusade_unit_strength(u)).sum();
    let dt = g.d.t::<UnitType>(defender);
    let probe = g.player_init_unit(PlayerId(2), dt, 14, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    let each = g.crusade_unit_strength(probe).max(1);
    let n_def = (lent * 100 / ratio / each).max(1) as usize;
    if n_def > 1 {
        units(&mut g, PlayerId(2), defender, n_def - 1, 14, 5, UnitAIType::UNITAI_CITY_DEFENSE);
    }
    let id = if baseline {
        let (ta, tt) = (g.players[PlayerId(0)].team, g.players[PlayerId(2)].team);
        g.team_declare_war(ta, tt, false, None);
        None
    } else {
        let id = call(&mut g);
        g.crusade_lend(id, PlayerId(0), &a0).unwrap();
        g.crusade_lend(id, PlayerId(1), &a1).unwrap();
        Some(id)
    };
    for _ in 0..30 {
        play_turn(&mut g);
        let fallen = g.map.plots[target_city(&g)].plot_city.is_none_or(|c| g.cities[c].owner != PlayerId(2));
        if fallen || id.is_some_and(|id| g.crusade_reports.iter().any(|r| r.id == id && r.reason == Dissolution::Objective)) {
            return true;
        }
        if id.is_some_and(|id| g.crusade(id).is_none()) {
            return false;
        }
    }
    false
}
