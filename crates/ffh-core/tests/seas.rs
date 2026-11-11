//! The sea, phase 2 (docs/NAVAL.md, sections 4 and 5): seas cut when the map is made (their
//! invariants on generated maps), Patrol, strength and each control state, hidden nationality,
//! the blockade of plots and trade and its refresh, harbours, interception, a port under siege,
//! supply ships, lanes, shore fire and a save; on situations built by hand.

use std::collections::BTreeSet;

use ffh_core::army::{SupplyShipRefusal, SupplySource};
use ffh_core::engagement::PreviewInput;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::seas::{Harbour, SeaControl};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians) on one flat block of grassland (x 1..=38, y 2..=9) inside a ring of
/// coast, revealed to both teams, the teams met, the starting units gone, player 0's turn active;
/// player 0 has a port on (5, 2). Seas cut by hand: West (y 0..=1, x 0..=12), East (y 0..=1,
/// x 13..=39), South (y 10..=11) and Column (x 0, y 2..=9); x 39, y 2..=9 has none.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    port: CityId,
    west: SeaId,
    east: SeaId,
    south: SeaId,
    column: SeaId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=9).contains(&y) && (1..=38).contains(&x);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let at = |g: &Game, x: i32, y: i32| g.map.plot_xy(x, y).unwrap();
    let band = |g: &Game, xs: std::ops::RangeInclusive<i32>, ys: std::ops::RangeInclusive<i32>| {
        let mut v = Vec::new();
        for y in ys.clone() {
            for x in xs.clone() {
                v.push(at(g, x, y));
            }
        }
        v
    };
    let west = g.map_cut_sea("West", &band(&g, 0..=12, 0..=1));
    let east = g.map_cut_sea("East", &band(&g, 13..=39, 0..=1));
    let south = g.map_cut_sea("South", &band(&g, 0..=39, 10..=11));
    let column = g.map_cut_sea("Column", &band(&g, 0..=0, 2..=9));
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    let port = g.player_init_city(p0, 5, 2, true, true);
    World { g, p0, p1, port, west, east, south, column }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    /// A warship of `owner` on Patrol at `(x, y)`.
    fn patrol(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let u = self.unit(owner, name, x, y);
        assert!(self.g.unit_can_plunder(u, self.at(x, y), false), "{name} can patrol");
        assert!(self.g.unit_plunder(u));
        assert!(self.g.units[u].blockading);
        u
    }

    fn t0(&self) -> TeamId {
        self.g.players[self.p0].team
    }

    fn t1(&self) -> TeamId {
        self.g.players[self.p1].team
    }

    fn war(&mut self) {
        let (t0, t1) = (self.t0(), self.t1());
        self.g.team_declare_war(t0, t1, false, None);
    }

    fn logged(&self, text: &str) -> usize {
        self.g.log.entries.iter().filter(|m| m.text.contains(text)).count()
    }

    fn fresh(&self) {
        assert!(self.g.sea_control_stale().is_empty(), "the stored control equals a fresh one");
    }

    /// Moves `u`'s group one step to `(x, y)` (a completed step: the interception's hook).
    fn step(&mut self, u: UnitId, x: i32, y: i32) {
        let grp = self.g.units[u].group.unwrap();
        let to = self.at(x, y);
        self.g.group_move(grp, to, false, None, false);
    }

    fn alive(&self, u: UnitId) -> bool {
        self.g.units.contains(u) && !self.g.unit_is_dead(u)
    }
}

// ---- Seas when the map is made (4.1) ----

/// The invariants of a generated map's seas (docs/NAVAL.md, 4.1) that hold for any seed.
fn check_generated(g: &Game) {
    let d = &g.d.defines;
    let (min, target) = (d.sea_min_plots as usize, d.sea_target_plots as usize);
    // Every water plot of an area of at least SEA_MIN_PLOTS plots has a sea; no other plot has.
    for (a, area) in g.map.areas.iter() {
        let plots: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.map.plots[q].area == Some(a)).collect();
        let big = area.water && plots.len() >= min;
        for &q in &plots {
            assert_eq!(g.map.plots[q].sea.is_some(), big, "plot {q:?} of area {a:?} ({} plots, water {})", plots.len(), area.water);
        }
        if big {
            let seas: BTreeSet<SeaId> = plots.iter().map(|&q| g.map.plots[q].sea.unwrap()).collect();
            let want = ((plots.len() + target / 2) / target).max(1);
            assert_eq!(seas.len(), want, "an area of {} plots is cut into {want} seas", plots.len());
        }
    }
    // Each sea: its plots are the plots that name it, in order, connected over water (8 ways);
    // its size near the target.
    let mut names = BTreeSet::new();
    for s in g.sea_ids() {
        let sea = &g.map.seas[s.idx()];
        assert!(names.insert(sea.name.clone()), "the name {} is unique", sea.name);
        let named: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.map.plots[q].sea == Some(s)).collect();
        assert_eq!(sea.plots, named);
        assert!(!sea.plots.is_empty());
        assert!(sea.plots.len() <= 4 * target, "{} has {} plots", sea.name, sea.plots.len());
        let mut seen = BTreeSet::from([sea.plots[0]]);
        let mut todo = vec![sea.plots[0]];
        while let Some(q) = todo.pop() {
            let (x, y) = g.map.xy(q);
            for i in 0..DirectionType::NUM {
                if let Some(o) = g.plot_direction(x, y, DirectionType::from_usize(i)) {
                    if g.map.plots[o].sea == Some(s) && seen.insert(o) {
                        todo.push(o);
                    }
                }
            }
        }
        assert_eq!(seen.len(), sea.plots.len(), "{} is connected", sea.name);
        assert!(sea.closed_to.is_empty(), "nobody patrols at the start");
    }
    // The sizes, on average, near the target.
    let cut: Vec<usize> = g.map.seas.iter().map(|s| s.plots.len()).collect();
    if cut.len() >= 4 {
        let mean = cut.iter().sum::<usize>() / cut.len();
        assert!(mean >= target / 2 && mean <= 2 * target, "mean sea size {mean} near {target}");
    }
}

fn generated(world: &str, seed: u32, civs: usize) -> Game {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, civs, world, seed));
    // The players' maps (as `ffh` and `ffh-play` make them).
    g.map_script = std::sync::Arc::new(FractalMapScript::default());
    g.game_init();
    g
}

#[test]
fn generated_maps_are_cut_into_seas() {
    for (world, seed, civs) in [("WORLDSIZE_DUEL", 1, 2), ("WORLDSIZE_SMALL", 2, 4), ("WORLDSIZE_STANDARD", 3, 6)] {
        let g = generated(world, seed, civs);
        assert!(!g.map.seas.is_empty(), "{world} has seas");
        check_generated(&g);
    }
}

#[test]
fn the_same_seed_cuts_the_same_seas() {
    let (a, b) = (generated("WORLDSIZE_TINY", 4, 3), generated("WORLDSIZE_TINY", 4, 3));
    assert_eq!(a.map.seas, b.map.seas);
    assert!(a.map.plot_ids().all(|q| a.map.plots[q].sea == b.map.plots[q].sea));
}

#[test]
fn the_defines_and_names() {
    let w = world();
    let d = &w.g.d.defines;
    assert_eq!((d.sea_min_plots, d.sea_target_plots, d.sea_hold_percent), (6, 40, 200));
    assert_eq!((d.port_siege_heal_percent, d.shore_fire_ships, d.intercept_min_odds), (50, 3, 50));
    let names: Vec<&String> = w.g.d.sea_names.iter().flat_map(|l| l.names.iter()).collect();
    assert!(names.len() >= 80);
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len(), "no name twice");
    assert!(w.g.d.sea_names.iter().any(|l| l.lake && !l.names.is_empty()));
}

#[test]
fn water_made_later_joins_a_sea_and_land_leaves_it() {
    let mut w = world();
    let (grass, coast) = (w.g.d.t::<TerrainType>("TERRAIN_GRASS"), w.g.d.t::<TerrainType>("TERRAIN_COAST"));
    // (12, 2) touches West (x <= 12) and East (13, 1): the lower sea index.
    let q = w.at(12, 2);
    w.g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, true);
    w.g.plot_set_terrain_type(q, coast, true);
    assert_eq!(w.g.plot_sea(q), Some(w.west.min(w.east)));
    assert!(w.g.map.seas[w.west.min(w.east).idx()].plots.contains(&q));
    // A lake inland: no sea.
    let lake = w.at(20, 6);
    w.g.plot_set_plot_type(lake, PlotType::PLOT_OCEAN, true);
    assert_eq!(w.g.plot_sea(lake), None);
    // Land leaves its sea.
    let shore = w.at(20, 1);
    w.g.plot_set_plot_type(shore, PlotType::PLOT_LAND, true);
    w.g.plot_set_terrain_type(shore, grass, true);
    assert_eq!(w.g.plot_sea(shore), None);
    assert!(!w.g.map.seas[w.east.idx()].plots.contains(&shore));
    w.fresh();
}

// ---- Patrol, strength and control (4.2, 4.3) ----

#[test]
fn patrol_is_a_warship_of_a_player_in_a_sea_outside_a_city() {
    let mut w = world();
    let tri = w.unit(w.p0, "UNIT_TRIREME", 8, 1);
    assert!(w.g.unit_can_plunder(tri, w.at(8, 1), false));
    assert!(!w.g.unit_can_plunder(tri, w.at(39, 5), false), "x 39 has no sea");
    let in_port = w.unit(w.p0, "UNIT_TRIREME", 5, 2);
    assert!(!w.g.unit_can_plunder(in_port, w.at(5, 2), false), "not in a city");
    let serpent = w.unit(w.p1, "UNIT_SEA_SERPENT", 30, 0);
    assert!(!w.g.unit_can_plunder(serpent, w.at(30, 0), false), "an animal is no warship");
    // Taking it ends the moves; moving ends it.
    assert!(w.g.unit_plunder(tri));
    assert_eq!(w.g.unit_moves_left(tri), 0);
    assert_eq!(w.g.sea_strength(w.west, w.t0()), 7);
    w.g.units[tri].moves = 0;
    w.g.unit_move(tri, w.at(9, 1));
    assert!(!w.g.units[tri].blockading, "moving ends the Patrol");
    assert_eq!(w.g.sea_strength(w.west, w.t0()), 0);
}

#[test]
fn each_control_state() {
    let mut w = world();
    let (t0, t1) = (w.t0(), w.t1());
    let open = w.g.sea_state(w.east, t0);
    assert_eq!((open.control, open.ours, open.theirs), (SeaControl::Open, 0, 0));
    let a = w.patrol(w.p0, "UNIT_TRIREME", 8, 1);
    let s = w.g.sea_state(w.west, t0);
    assert_eq!((s.control, s.ours, s.theirs), (SeaControl::HeldByUs, 7, 0));
    let s = w.g.sea_state(w.west, t1);
    assert_eq!((s.control, s.theirs), (SeaControl::PatrolledBy(Some(t0)), 7), "at peace");
    assert!(!w.g.sea_is_closed_to(w.west, t1));
    w.war();
    let s = w.g.sea_state(w.west, t1);
    assert_eq!((s.control, s.ours, s.theirs), (SeaControl::HeldBy(Some(t0)), 0, 7));
    assert!(w.g.sea_is_closed_to(w.west, t1), "war refreshed the stored control");
    assert_eq!(w.g.sea_closed_to(w.west, t1), Some(t0));
    // A frigate (10) against 7: contested both ways.
    let f = w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    let s = w.g.sea_state(w.west, t0);
    assert_eq!((s.control, s.ours, s.theirs), (SeaControl::Contested(Some(t1)), 7, 10));
    assert_eq!(w.g.sea_state(w.west, t1).control, SeaControl::Contested(Some(t0)));
    assert!(!w.g.sea_is_closed_to(w.west, t1) && !w.g.sea_is_closed_to(w.west, t0));
    // 21 against 10: held.
    w.patrol(w.p0, "UNIT_TRIREME", 7, 1);
    let b = w.patrol(w.p0, "UNIT_TRIREME", 6, 0);
    assert_eq!(w.g.sea_state(w.west, t0).control, SeaControl::HeldByUs);
    assert!(w.g.sea_is_closed_to(w.west, t1));
    // Damage takes it under twice the enemy's: contested again, refreshed at once.
    w.g.unit_set_damage(b, 50, None);
    assert_eq!(w.g.sea_strength(w.west, t0), 17);
    assert_eq!(w.g.sea_state(w.west, t0).control, SeaControl::Contested(Some(t1)));
    assert!(!w.g.sea_is_closed_to(w.west, t1));
    w.fresh();
    // A death: the frigate gone, the sea is held again.
    w.g.unit_kill(f, false, None);
    assert!(w.g.sea_is_closed_to(w.west, t1));
    w.fresh();
    // Peace: open to both, patrolled by the other.
    let (t0c, t1c) = (t0, t1);
    w.g.team_make_peace(t0c, t1c, false);
    assert!(!w.g.sea_is_closed_to(w.west, t1));
    assert_eq!(w.g.sea_state(w.west, t1).control, SeaControl::PatrolledBy(Some(t0)));
    let _ = a;
    w.fresh();
}

#[test]
fn hidden_nationality_counts_as_at_war_with_everyone() {
    let mut w = world();
    let (t0, t1) = (w.t0(), w.t1());
    // A ship of team 0 beside it sees it.
    w.unit(w.p0, "UNIT_GALLEY", 10, 1);
    let pirate = w.unit(w.p1, "UNIT_FRIGATE", 9, 0);
    w.g.unit_change_hidden_nationality(pirate, 1);
    assert!(w.g.unit_is_hidden_nationality(pirate));
    assert!(w.g.unit_plunder(pirate));
    // At peace, the sea is closed to team 0 by a ship of hidden nationality.
    assert_eq!(w.g.sea_state(w.west, t0).control, SeaControl::HeldBy(Some(t1)), "the truth names its team");
    assert!(w.g.sea_is_closed_to(w.west, t0));
    // What team 0 can work out: an unknown party.
    assert_eq!(w.g.sea_state_known(w.west, t0).control, SeaControl::HeldBy(None));
    let lane = w.g.sea_lane_known(t0, w.at(9, 1), t0).expect("a lane");
    assert_eq!(lane.cut_in, Some((w.west, w.g.barbarian_team())), "an unknown holder reads as the barbarians");
    w.fresh();
}

#[test]
fn the_view_counts_only_ships_it_sees() {
    let mut w = world();
    let (t0, t1) = (w.t0(), w.t1());
    w.war();
    w.patrol(w.p1, "UNIT_FRIGATE", 30, 0);
    assert!(!w.g.plot_is_visible(w.at(30, 0), t0), "no unit of team 0 near it");
    assert_eq!(w.g.sea_state(w.east, t0).control, SeaControl::HeldBy(Some(t1)), "the truth");
    assert_eq!(w.g.sea_state_known(w.east, t0).control, SeaControl::Open, "unseen ships do not count");
}

// ---- What a closed sea does (4.4) ----

#[test]
fn a_closed_sea_blockades_its_plots_and_trade_and_the_harbour() {
    let mut w = world();
    let (t0, t1) = (w.t0(), w.t1());
    let water = w.at(5, 1);
    assert_eq!(w.g.city_harbour(w.port), Harbour::Open);
    let workable = w.g.city_can_work(w.port, water);
    w.war();
    let f = w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    assert!(w.g.sea_is_closed_to(w.west, t0));
    assert_eq!(w.g.map.plots[water].blockaded_count[t0], 1);
    assert_eq!(w.g.map.plots[w.at(20, 0)].blockaded_count[t0], 0, "another sea");
    assert!(!w.g.city_can_work(w.port, water), "its water is not worked");
    assert!(!w.g.plot_is_trade_network(water, t0), "no trade of team 0 over it");
    assert_eq!(w.g.city_harbour(w.port), Harbour::Blockaded(vec![(w.west, t1)]));
    assert_eq!(w.logged("holds West"), 1, "the message to the port's owner");
    // The Patrol ends: open again.
    w.g.unit_set_blockading(f, false);
    assert_eq!(w.g.map.plots[water].blockaded_count[t0], 0);
    assert_eq!(w.g.city_harbour(w.port), Harbour::Open);
    assert_eq!(w.g.city_can_work(w.port, water), workable);
    assert_eq!(w.logged("West is no longer closed"), 1);
    w.fresh();
}

#[test]
fn a_harbour_on_two_seas_is_blockaded_only_when_both_are_closed() {
    let mut w = world();
    let t1 = w.t1();
    let corner = w.g.player_init_city(w.p0, 1, 2, true, true);
    assert_eq!(w.g.city_seas(corner), vec![w.west, w.column]);
    assert_eq!(w.g.city_harbour(corner), Harbour::Open);
    w.war();
    w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    assert_eq!(w.g.city_harbour(corner), Harbour::Open, "the Column is open");
    assert_eq!(w.g.sea_ports_blockaded_if_held(w.column, w.t0()), vec![corner], "if the Column were held");
    w.patrol(w.p1, "UNIT_FRIGATE", 0, 6);
    assert_eq!(w.g.city_harbour(corner), Harbour::Blockaded(vec![(w.west, t1), (w.column, t1)]));
    // A city on no sea.
    let inland = w.g.player_init_city(w.p0, 20, 6, true, true);
    assert_eq!(w.g.city_harbour(inland), Harbour::None);
}

// ---- Interception (4.4 point 4) ----

#[test]
fn a_patrol_intercepts_once_a_turn() {
    let mut w = world();
    w.war();
    let ship = w.patrol(w.p1, "UNIT_TRIREME", 20, 0);
    let a = w.unit(w.p0, "UNIT_WARRIOR", 25, 2);
    assert_eq!(w.g.sea_interceptors_for(a, w.at(25, 1), Some(a)), vec![ship], "the preview's interceptor");
    w.step(a, 25, 1);
    assert!(!w.alive(a), "sunk by the patrol");
    assert!(w.g.units[ship].intercepted);
    assert_eq!(w.g.unit_plot(ship), w.at(20, 0), "it did not move");
    assert!(w.g.units[ship].blockading, "still on Patrol");
    assert_eq!(w.logged("Intercepted in East"), 1, "to the mover's owner");
    assert_eq!(w.logged("intercepted Warrior in East"), 1, "to the patrol's owner");
    // Once a turn.
    let b = w.unit(w.p0, "UNIT_WARRIOR", 26, 2);
    w.step(b, 26, 1);
    assert!(w.alive(b), "the patrol has intercepted this turn");
    // Its owner's next turn: ready again.
    w.g.player_update_seas(w.p1);
    assert!(!w.g.units[ship].intercepted);
    w.fresh();
}

#[test]
fn no_interception_at_low_odds_or_at_peace() {
    let mut w = world();
    let weak = w.patrol(w.p1, "UNIT_GALLEY", 20, 0);
    let frigate = w.unit(w.p0, "UNIT_FRIGATE", 25, 0);
    w.step(frigate, 26, 0);
    assert!(!w.g.units[weak].intercepted, "at peace");
    w.war();
    w.g.unit_set_damage(weak, 80, None);
    assert!(w.g.get_combat_odds(weak, frigate) < 500);
    w.g.units[frigate].moves = 0;
    w.step(frigate, 27, 0);
    assert!(!w.g.units[weak].intercepted, "odds too low");
    assert!(w.alive(frigate) && w.alive(weak));
}

#[test]
fn the_mover_dies_mid_path() {
    let mut w = world();
    w.war();
    w.patrol(w.p1, "UNIT_FRIGATE", 20, 0);
    let galley = w.unit(w.p0, "UNIT_GALLEY", 14, 1);
    let grp = w.g.units[galley].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 18, 1, 0, false, false, None, None, None);
    // Whatever the duel gave, the game goes on.
    if !w.alive(galley) {
        assert!(!w.g.groups.contains(grp) || w.g.groups[grp].units.is_empty());
    }
    w.fresh();
}

#[test]
fn an_escort_defends_a_unit_in_boats() {
    let mut w = world();
    w.war();
    let ship = w.patrol(w.p1, "UNIT_FRIGATE", 20, 0);
    let escort = w.unit(w.p0, "UNIT_GALLEY", 27, 1);
    let a = w.unit(w.p0, "UNIT_WARRIOR", 27, 2);
    w.step(a, 27, 1);
    assert!(w.g.unit_is_in_boats(a));
    assert!(w.g.units[ship].intercepted, "it intercepted");
    assert!(w.alive(a), "the escort fought, not the unit in boats");
    assert!(!w.alive(escort) || w.g.units[escort].damage > 0 || w.g.units[ship].damage > 0);
}

// ---- A port under siege (4.5) ----

#[test]
fn a_port_under_siege_heals_and_repairs_by_its_harbour() {
    let mut w = world();
    let c = w.port;
    let walls = w.g.d.t::<BuildingType>("BUILDING_WALLS");
    let class = w.g.d.buildings[walls].building_class_type.unwrap();
    w.g.cities[c].building_tiers[class] = BuildingTierType::STANDARD;
    w.g.city_set_num_real_building(c, walls, 1);
    let to = w.g.city_plot(c);
    let soldier = w.unit(w.p0, "UNIT_WARRIOR", 5, 2);
    w.g.units[soldier].damage = 50;
    let full = w.g.unit_heal_rate(soldier, to);
    let city_part = w.g.d.defines.city_heal_rate + w.g.cities[c].heal_rate;
    let repair = w.g.d.defines.sentinel_repair_percent;
    // 1. Not besieged.
    w.g.city_set_sentinel_damage(c, 0, 60);
    w.g.city_repair_sentinels(c);
    assert_eq!(w.g.cities[c].sentinel_damage[0], 60 - repair);
    // 2. Besieged, the harbour open: half.
    w.war();
    w.unit(w.p1, "UNIT_WARRIOR", 5, 3);
    assert!(w.g.city_is_besieged(c));
    assert_eq!(w.g.city_harbour(c), Harbour::Open);
    assert_eq!(w.g.unit_heal_rate(soldier, to), full - city_part + city_part * 50 / 100);
    w.g.city_set_sentinel_damage(c, 0, 60);
    w.g.city_repair_sentinels(c);
    assert_eq!(w.g.cities[c].sentinel_damage[0], 60 - repair * 50 / 100);
    // 3. Besieged and blockaded: nothing from the city.
    w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    assert!(matches!(w.g.city_harbour(c), Harbour::Blockaded(_)));
    assert_eq!(w.g.unit_heal_rate(soldier, to), full - city_part);
    w.g.city_set_sentinel_damage(c, 0, 60);
    w.g.city_repair_sentinels(c);
    assert_eq!(w.g.cities[c].sentinel_damage[0], 60);
}

// ---- Supply ships (4.6) and lanes (section 5) ----

/// Team 0 with Optics and Astronomy (range 16).
fn far_world() -> World {
    let mut w = world();
    let t0 = w.t0();
    for tech in ["TECH_OPTICS", "TECH_ASTRONOMY"] {
        let t = w.g.d.t::<TechType>(tech);
        w.g.team_set_has_tech(t0, t, true, Some(w.p0), false, false);
    }
    assert_eq!(w.g.team_sea_range(t0), 16);
    w
}

#[test]
fn a_lane_runs_down_the_range_field() {
    let w = far_world();
    let t0 = w.t0();
    // Ties: (7, 0) before (7, 1), the lower index.
    let lane = w.g.sea_lane(t0, w.at(8, 0)).expect("a lane");
    assert_eq!(lane.path, vec![w.at(8, 0), w.at(7, 0), w.at(6, 1)]);
    assert_eq!(lane.port, w.port);
    assert_eq!(lane.cut_in, None);
    // From land: the best of its water plots.
    let lane = w.g.sea_lane(t0, w.at(20, 2)).expect("from the shore");
    assert_eq!(lane.path.len() as i32, w.g.plot_sea_range(t0, lane.path[0]).unwrap());
    assert!(lane.path.windows(2).all(|p| w.g.plot_distance(w.g.map.xy(p[0]).0, w.g.map.xy(p[0]).1, w.g.map.xy(p[1]).0, w.g.map.xy(p[1]).1) == 1));
    // No path: out of range, or a team with no city.
    assert_eq!(w.g.sea_lane(t0, w.at(30, 11)), None, "out of range");
    assert_eq!(w.g.sea_lane(w.t1(), w.at(8, 0)), None, "no port");
}

#[test]
fn a_lane_is_cut_in_the_first_closed_sea() {
    let mut w = far_world();
    let (t0, t1) = (w.t0(), w.t1());
    w.war();
    w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    let lane = w.g.sea_lane(t0, w.at(20, 1)).expect("a lane");
    assert_eq!(lane.cut_in, Some((w.west, t1)));
    assert!(lane.is_cut());
}

#[test]
fn a_supply_ship_needs_open_water_and_an_open_lane() {
    let mut w = far_world();
    let (t0, t1) = (w.t0(), w.t1());
    w.war();
    let galley = w.unit(w.p0, "UNIT_GALLEON", 20, 1);
    let shore = w.at(20, 2);
    let source = |w: &World| w.g.plot_supply_line(w.p0, shore).map(|l| l.source);
    // Open.
    assert_eq!(w.g.supply_ship_refused(t0, w.at(20, 1)), None);
    assert_eq!(source(&w), Some(SupplySource::Ship(galley)));
    // Contested: still a source.
    w.patrol(w.p0, "UNIT_TRIREME", 25, 0);
    w.patrol(w.p1, "UNIT_FRIGATE", 26, 0);
    assert!(matches!(w.g.sea_state(w.east, t0).control, SeaControl::Contested(_)));
    assert_eq!(source(&w), Some(SupplySource::Ship(galley)));
    // Closed: no source.
    w.patrol(w.p1, "UNIT_FRIGATE", 27, 0);
    assert!(w.g.sea_is_closed_to(w.east, t0));
    assert_eq!(w.g.supply_ship_refused(t0, w.at(20, 1)), Some(SupplyShipRefusal::SeaClosed(w.east, t1)));
    assert_eq!(source(&w), None);
    // Known: only what the viewer sees counts.
    assert_eq!(w.g.plot_supply_line_known(w.p0, shore, t0).map(|l| l.source), None);
}

#[test]
fn a_supply_ship_with_a_cut_lane_is_no_source() {
    let mut w = far_world();
    let t1 = w.t1();
    w.war();
    let galley = w.unit(w.p0, "UNIT_GALLEON", 20, 1);
    let shore = w.at(20, 2);
    assert_eq!(w.g.plot_supply_line(w.p0, shore).map(|l| l.source), Some(SupplySource::Ship(galley)));
    w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    assert_eq!(w.g.supply_ship_refused(w.t0(), w.at(20, 1)), Some(SupplyShipRefusal::LaneCut(w.west, t1)));
    assert_eq!(w.g.plot_supply_line(w.p0, shore).map(|l| l.source), None);
}

// ---- Shore fire (4.7) ----

#[test]
fn ships_fire_into_a_battle_on_the_coast() {
    let mut w = world();
    w.war();
    let from = w.at(20, 3);
    let to = w.at(20, 2);
    let axe = w.unit(w.p0, "UNIT_AXEMAN", 20, 3);
    w.unit(w.p1, "UNIT_WARRIOR", 20, 2);
    let tris: Vec<UnitId> = [(19, 1), (19, 1), (20, 1), (20, 1)].iter().map(|&(x, y)| w.unit(w.p0, "UNIT_TRIREME", x, y)).collect();
    let frigate = w.unit(w.p1, "UNIT_FRIGATE", 21, 1);
    assert!(w.g.engage_applies(axe, to));
    let preview = w.g.engage_preview(&PreviewInput::new(w.p0, from, to)).expect("a preview");
    let att: Vec<UnitId> = preview.shore.iter().filter(|s| s.side == ffh_core::engagement::Side::Attacker).map(|s| s.unit).collect();
    let def: Vec<UnitId> = preview.shore.iter().filter(|s| s.side == ffh_core::engagement::Side::Defender).map(|s| s.unit).collect();
    assert_eq!(att.len(), 3, "at most SHORE_FIRE_SHIPS a side");
    assert_eq!(def, vec![frigate]);
    let reports = w.g.battle_reports.len();
    w.g.unit_attack(axe, to);
    assert_eq!(w.g.battle_reports.len(), reports + 1);
    let r = w.g.battle_reports.last().unwrap();
    assert_eq!(r.sides[0].shore.len(), 3, "the preview's ships fired");
    assert_eq!(r.sides[1].shore.len(), 1);
    for &u in &att {
        assert!(w.g.units[u].shore_fired);
    }
    let rest: Vec<UnitId> = tris.iter().copied().filter(|u| !att.contains(u)).collect();
    assert_eq!(rest.len(), 1);
    assert!(!w.g.units[rest[0]].shore_fired);
    // Once a turn: the next battle's set-up has only the ship that did not fire.
    if w.alive(axe) && w.g.map.plots[to].units.iter().any(|&u| w.g.units[u].owner == w.p1) {
        let again = w.g.engage_preview(&PreviewInput::new(w.p0, from, to)).expect("a preview");
        assert_eq!(again.shore.iter().map(|s| s.unit).collect::<Vec<_>>(), rest);
    }
    w.g.player_update_seas(w.p0);
    assert!(att.iter().all(|&u| !w.g.units[u].shore_fired));
}

// ---- Saves ----

#[test]
fn seas_and_their_state_survive_a_save() {
    let mut w = world();
    w.war();
    let f = w.patrol(w.p1, "UNIT_FRIGATE", 10, 0);
    w.g.units[f].intercepted = true;
    w.g.units[f].shore_fired = true;
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.map.seas, w.g.map.seas);
    assert!(h.map.plot_ids().all(|q| h.map.plots[q].sea == w.g.map.plots[q].sea));
    assert!(h.sea_is_closed_to(w.west, w.t0()));
    assert!(h.units[f].intercepted && h.units[f].shore_fired && h.units[f].blockading);
    assert!(h.sea_control_stale().is_empty());
    assert_eq!(h.save(), bytes);
    let _ = (w.south, w.column);
}
