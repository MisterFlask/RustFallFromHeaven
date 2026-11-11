//! The sea, phase 4 (docs/NAVAL.md, section 9): far shores on generated maps (invariants that
//! hold for any seed), founding outposts, what an outpost is, raising and capturing one, the
//! lane's three states, the trade network, raiding gold, starving and the port rule, and a save;
//! on situations built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::outposts::{LaneState, OutpostSiteRefusal, FOUND_OUTPOST};
use ffh_core::setup::GameSetup;
use ffh_data::*;

// ---- Far shores (9.1) ----

fn generated(world: &str, seed: u32, civs: usize) -> Game {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, civs, world, seed));
    g.map_script = std::sync::Arc::new(FractalMapScript::default());
    g.game_init();
    g
}

/// What holds on any generated map: the far shores are the land areas with no start and at least
/// FAR_SHORE_MIN_PLOTS plots; every flagged bonus off the starting landmasses is on a far shore,
/// and every one left on a starting landmass is in a start's city radius or could not move.
fn check_far_shores(g: &Game) {
    let d = g.d;
    let report = g.far_shore_report().expect("the pass ran");
    let shores = g.map_far_shores();
    assert_eq!(report.far_shores.iter().map(|e| e.0).collect::<Vec<_>>(), shores);
    for &a in &shores {
        let area = &g.map.areas[a];
        assert!(!area.water && area.num_starting_plots == 0 && area.num_tiles >= d.defines.far_shore_min_plots);
    }
    if shores.is_empty() {
        assert!(report.moved.is_empty() && report.mana.is_empty(), "no far shore: nothing changes");
        return;
    }
    for &(b, from, to) in &report.moved {
        assert!(d.bonuses[b].far_shore);
        assert_eq!(g.map.plots[to].bonus_type, Some(b), "a moved bonus is where it went");
        assert!(g.map.plots[to].area.is_some_and(|a| shores.contains(&a)));
        assert_ne!(g.map.plots[from].bonus_type, Some(b));
    }
    for &(b, q) in report.stayed.iter().chain(report.kept_by_start.iter()) {
        assert_eq!(g.map.plots[q].bonus_type, Some(b), "a bonus that stayed is still there");
    }
    // Every flagged bonus on a starting landmass is one the report kept.
    for q in g.map.plot_ids() {
        let Some(b) = g.map.plots[q].bonus_type else { continue };
        if !d.bonuses[b].far_shore || g.plot_is_water(q) {
            continue;
        }
        let area = g.map.plots[q].area.unwrap();
        if g.map.areas[area].num_starting_plots > 0 {
            assert!(report.stayed.contains(&(b, q)) || report.kept_by_start.contains(&(b, q)), "{q:?}: a flagged bonus on a starting landmass stayed for a reason");
        }
    }
    let mana = d.t::<BonusType>("BONUS_MANA");
    assert!(report.mana_wanted >= shores.len() as i32);
    for &q in &report.mana {
        assert_eq!(g.map.plots[q].bonus_type, Some(mana));
        assert!(g.map.plots[q].area.is_some_and(|a| shores.contains(&a)));
    }
}

#[test]
fn far_shores_on_generated_maps() {
    let mut any = false;
    for (world, seed, civs) in [("WORLDSIZE_SMALL", 2, 4), ("WORLDSIZE_STANDARD", 2, 6), ("WORLDSIZE_STANDARD", 3, 6), ("WORLDSIZE_DUEL", 1, 2)] {
        let g = generated(world, seed, civs);
        check_far_shores(&g);
        any |= !g.map_far_shores().is_empty();
    }
    assert!(any, "some map has a far shore");
}

#[test]
fn far_shores_are_deterministic() {
    let (a, b) = (generated("WORLDSIZE_STANDARD", 3, 6), generated("WORLDSIZE_STANDARD", 3, 6));
    assert_eq!(a.far_shore_report(), b.far_shore_report());
    assert!(a.map.plot_ids().all(|q| a.map.plots[q].bonus_type == b.map.plots[q].bonus_type));
}

// ---- A world by hand ----

/// A duel game (no barbarians) on a 40x16 map of coast: the home island (x 1..=12, y 2..=13) and
/// the far island (x 22..=33, y 2..=13), all revealed. Player 0's capital at (6, 7) (inland), its
/// port at (12, 10). Seas cut by hand: Gap (x 13..=21) and Rim (the rest of the water).
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    capital: CityId,
    port: CityId,
    gap: SeaId,
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
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 16, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=13).contains(&y) && ((1..=12).contains(&x) || (22..=33).contains(&x));
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
    let gap_plots: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.plot_is_water(q) && (13..=21).contains(&g.map.xy(q).0)).collect();
    let rim_plots: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.plot_is_water(q) && !(13..=21).contains(&g.map.xy(q).0)).collect();
    let gap = g.map_cut_sea("Gap", &gap_plots);
    g.map_cut_sea("Rim", &rim_plots);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    let capital = g.player_init_city(p0, 6, 7, true, true);
    let port = g.player_init_city(p0, 12, 10, true, true);
    assert!(g.city_is_capital(capital));
    World { g, p0, p1, capital, port, gap }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
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

    /// Player 0's outpost at (22, 7) (founded by a Colonist).
    fn outpost(&mut self) -> CityId {
        let u = self.unit(self.p0, "UNIT_COLONIST", 22, 7);
        assert!(self.g.unit_found_outpost(u), "the colonist founds an outpost");
        self.g.map.plots[self.at(22, 7)].plot_city.expect("an outpost")
    }

    /// A warship of `owner` on Patrol at `(x, y)`.
    fn patrol(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let u = self.unit(owner, name, x, y);
        assert!(self.g.unit_can_plunder(u, self.at(x, y), false), "{name} can patrol");
        assert!(self.g.unit_plunder(u));
        u
    }

    fn logged(&self, text: &str) -> usize {
        self.g.log.entries.iter().filter(|m| m.text.contains(text)).count()
    }

    fn same_group(&self, a: CityId, b: CityId) -> bool {
        let pa = self.g.plot_get_owner_plot_group(self.g.city_plot(a));
        pa.is_some() && pa == self.g.plot_get_owner_plot_group(self.g.city_plot(b))
    }
}

// ---- Founding (9.2) ----

#[test]
fn the_site_rule_for_both_units() {
    let mut w = world();
    let colonist = w.unit(w.p0, "UNIT_COLONIST", 22, 7);
    let settler = w.unit(w.p0, "UNIT_SETTLER", 22, 7);
    let (here, home, inland) = (w.at(22, 7), w.at(12, 4), w.at(27, 7));
    // The Colonist founds outposts only; the Settler both.
    assert!(w.g.unit_can_found_outpost(colonist, here, false));
    assert!(!w.g.unit_can_found(colonist, here, false));
    assert!(w.g.unit_can_found_outpost(settler, here, false));
    assert!(w.g.unit_can_found(settler, here, false));
    // The capital's landmass, an inland plot.
    assert_eq!(w.g.player_outpost_site_refusal(w.p0, home, false), Some(OutpostSiteRefusal::CapitalLandmass));
    assert_eq!(w.g.player_outpost_site_refusal(w.p0, inland, false), Some(OutpostSiteRefusal::NotCoastal));
    assert_eq!(w.g.player_outpost_site_refusal(w.p0, here, false), None);
    // A player with no capital founds none.
    assert_eq!(w.g.player_outpost_site_refusal(w.p1, here, false), Some(OutpostSiteRefusal::NoCapital));
    // The mission: data1 FOUND_OUTPOST.
    let grp = w.g.units[settler].group.unwrap();
    assert!(w.g.group_can_start_mission(grp, MissionType::MISSION_FOUND, FOUND_OUTPOST, -1, Some(here), false));
    w.g.group_push_mission(grp, MissionType::MISSION_FOUND, FOUND_OUTPOST, -1, 0, false, false, None, None, None);
    let c = w.g.map.plots[here].plot_city.expect("founded");
    assert!(w.g.cities[c].outpost);
    assert!(!w.g.units.contains(settler));
}

// ---- What an outpost is (9.2) ----

#[test]
fn what_an_outpost_is() {
    let mut w = world();
    let maint_before = w.g.city_calculate_num_cities_maintenance_times100(w.port);
    let c = w.outpost();
    let d = w.g.d;
    // Radius 1: the 8 plots around, no more.
    assert_eq!(w.g.city_num_city_plots(c), 9);
    assert!(w.g.city_index_plot(c, 8).is_some());
    assert!(w.g.city_index_plot(c, 9).is_none());
    // Borders stay at the 8 plots: culture level 1 at most.
    w.g.city_set_culture(c, w.p0, 100_000, true, true);
    assert!(w.g.cities[c].culture_level.is_some_and(|l| l.idx() <= 1));
    assert_eq!(w.g.map.plots[w.at(24, 7)].owner, None, "two plots away is not its land");
    // Population at most OUTPOST_MAX_POP.
    w.g.city_change_population(c, 5);
    assert_eq!(w.g.cities[c].population, d.defines.outpost_max_pop);
    w.g.cities[c].food = 10_000;
    w.g.city_do_growth(c);
    assert_eq!(w.g.cities[c].population, d.defines.outpost_max_pop);
    // Builds nothing; no governor; no maintenance; not counted for the others' maintenance.
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    assert!(!w.g.city_can_train(c, warrior, false, false, false, false));
    assert!(d.buildings.keys().all(|b| !w.g.city_can_construct(c, b, false, false, false)));
    assert!(!w.g.city_is_governed(c));
    w.g.city_update_maintenance(c);
    assert_eq!(w.g.cities[c].maintenance, 0);
    assert_eq!(w.g.city_calculate_num_cities_maintenance_times100(w.port), maint_before);
    assert!(d.buildings.keys().all(|b| w.g.cities[c].num_free_building[b] == 0), "no free buildings");
    w.g.city_do_production(c, true);
    assert_eq!(w.g.city_production(c), 0);
    assert_eq!(w.g.cities[c].overflow_production, 0);
    assert_eq!(w.g.player_get_num_outposts(w.p0), 1);
    // A port.
    assert!(w.g.city_is_port(c));
}

#[test]
fn an_outpost_is_never_a_capital_and_alone_is_no_city() {
    let mut w = world();
    let c = w.outpost();
    w.g.city_kill(w.capital, true);
    w.g.player_find_new_capital(w.p0);
    assert!(w.g.city_is_capital(w.port), "the port, not the outpost, takes the palace");
    w.g.city_kill(w.port, true);
    w.g.player_find_new_capital(w.p0);
    assert!(!w.g.city_is_capital(c));
    w.g.player_verify_alive(w.p0);
    assert!(!w.g.players[w.p0].alive, "a player with outposts only is eliminated");
}

#[test]
fn raising_an_outpost() {
    let mut w = world();
    let c = w.outpost();
    let cost = w.g.outpost_raise_cost();
    assert_eq!(cost, w.g.d.defines.outpost_raise_cost * w.g.d.game_speeds[w.g.game_speed_type()].construct_percent / 100);
    w.g.players[w.p0].gold = cost - 1;
    assert!(!w.g.player_raise_outpost(w.p0, c));
    w.g.players[w.p0].gold = cost + 5;
    assert!(w.g.player_raise_outpost(w.p0, c));
    assert_eq!(w.g.players[w.p0].gold, 5);
    assert!(!w.g.cities[c].outpost);
    assert!(w.g.city_num_city_plots(c) >= 21);
    let warrior = w.g.d.t::<UnitType>("UNIT_WARRIOR");
    assert!(w.g.city_can_train(c, warrior, false, false, false, false));
    assert_eq!(w.g.player_get_num_outposts(w.p0), 0);
}

#[test]
fn a_captured_outpost_stays_an_outpost() {
    let mut w = world();
    let c = w.outpost();
    w.war();
    w.g.player_acquire_city(w.p1, c, true, false, true);
    let c2 = w.g.map.plots[w.at(22, 7)].plot_city.expect("still a city");
    assert_eq!(w.g.cities[c2].owner, w.p1);
    assert!(w.g.cities[c2].outpost);
    assert_eq!(w.g.city_num_city_plots(c2), 9);
}

// ---- Lanes (9.3) ----

#[test]
fn an_open_lane_carries_trade() {
    let mut w = world();
    let c = w.outpost();
    let lane = match w.g.outpost_lane_state(c, w.t0()) {
        LaneState::Open(l) => l,
        other => panic!("open, not {other:?}"),
    };
    assert_eq!(lane.port, w.port);
    assert_eq!(w.g.cities[c].lane, lane.path);
    assert!(w.same_group(c, w.port), "the outpost is on the port's trade network");
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].lane_cut_turns, 0);
    // The sea panel lists it.
    assert_eq!(w.g.sea_outpost_lanes_through(w.gap, w.t0(), w.t0()).len(), 1);
}

#[test]
fn a_cut_lane_pays_the_holder_and_starves_the_outpost() {
    let mut w = world();
    let c = w.outpost();
    w.g.city_change_population(c, 1);
    assert_eq!(w.g.cities[c].population, 2);
    w.war();
    let gold = w.g.players[w.p1].gold;
    w.patrol(w.p1, "UNIT_FRIGATE", 17, 8);
    assert!(w.g.sea_is_closed_to(w.gap, w.t0()));
    match w.g.outpost_lane_state(c, w.t0()) {
        LaneState::Cut(l) => assert_eq!(l.cut_in.map(|e| e.0), Some(w.gap)),
        other => panic!("cut, not {other:?}"),
    }
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].lane_cut_turns, 1);
    assert!(w.g.cities[c].lane.is_empty());
    assert!(w.g.plot_get_owner_plot_group(w.g.city_plot(c)).is_none(), "off the trade network");
    assert_eq!(w.g.players[w.p1].gold, gold + w.g.d.defines.lane_raid_gold);
    assert_eq!(w.logged("lane cut in Gap"), 1);
    // Starving above LANE_STARVE_TURNS: -1 population a turn, not a port.
    let starve = w.g.d.defines.lane_starve_turns;
    let water = w.at(26, 7);
    let field_before = w.g.plot_sea_range(w.t0(), water);
    for _ in 1..=starve {
        w.g.player_update_lanes(w.p0);
    }
    assert!(w.g.city_outpost_starved(c));
    assert_eq!(w.g.cities[c].population, 1);
    assert!(!w.g.city_is_port(c));
    assert_ne!(w.g.plot_sea_range(w.t0(), water), field_before, "its water is farther from a port");
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].population, 1, "to 1 at least");
    assert!(w.g.sea_range_cache_stale().is_empty());
    // Recovery: the patrol goes; the lane opens.
    let (t0, t1) = (w.t0(), w.t1());
    w.g.team_make_peace(t1, t0, false);
    w.g.seas_refresh_all();
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].lane_cut_turns, 0);
    assert!(w.g.city_is_port(c));
    assert!(w.same_group(c, w.port));
    assert_eq!(w.logged("lane open again"), 1);
}

#[test]
fn a_contested_sea_pays_the_raider_and_the_lane_stays_open() {
    let mut w = world();
    let c = w.outpost();
    w.war();
    w.patrol(w.p0, "UNIT_FRIGATE", 15, 8);
    w.patrol(w.p1, "UNIT_FRIGATE", 18, 8);
    assert!(!w.g.sea_is_closed_to(w.gap, w.t0()));
    let gold = w.g.players[w.p1].gold;
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].lane_cut_turns, 0);
    assert!(!w.g.cities[c].lane.is_empty());
    assert_eq!(w.g.players[w.p1].gold, gold + w.g.d.defines.lane_raid_gold_contested);
}

#[test]
fn no_home_port_no_lane() {
    let mut w = world();
    let c = w.outpost();
    w.g.city_kill(w.port, true);
    assert_eq!(w.g.outpost_lane_state(c, w.t0()), LaneState::NoLane, "the capital is inland");
    w.g.player_update_lanes(w.p0);
    assert_eq!(w.g.cities[c].lane_cut_turns, 1);
    assert_eq!(w.logged("no lane to a home port"), 1);
    let _ = w.capital;
}

// ---- Saves ----

#[test]
fn outposts_survive_a_save() {
    let mut w = world();
    let c = w.outpost();
    w.g.cities[c].lane_cut_turns = 3;
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert!(h.cities[c].outpost);
    assert_eq!(h.cities[c].lane_cut_turns, 3);
    assert_eq!(h.cities[c].lane, w.g.cities[c].lane);
    assert!(h.plot_on_outpost_lane(h.cities[c].lane[0], w.t0()));
    assert_eq!(h.save(), bytes);
}
