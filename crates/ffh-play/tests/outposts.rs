//! Outposts and their lanes as the human sees them (docs/NAVAL.md, section 10;
//! `view/outposts.rs`): the city panel's lines and Raise to city, the cities' `outpost` field, the
//! view's `outpost_lanes` (open and cut), the Found outpost action and its reason, the plot's
//! `Not on your landmasses.`; on situations built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

/// A duel game on two islands of grassland (x 1..=12 and x 22..=33, y 2..=13) in coast, revealed;
/// player 0's capital at (6, 7), its port at (12, 10); a sea "Gap" over x 13..=21.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    gap: SeaId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    setup.players[0].human = true;
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let gap = g.map_cut_sea("Gap", &gap_plots);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    g.player_init_city(p0, 6, 7, true, true);
    g.player_init_city(p0, 12, 10, true, true);
    World { g, p0, p1, gap }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn v(&self) -> Viewer<'_> {
        Viewer::new(&self.g, self.p0, false)
    }

    fn outpost(&mut self) -> CityId {
        let u = self.unit(self.p0, "UNIT_COLONIST", 22, 7);
        assert!(self.g.unit_found_outpost(u));
        self.g.map.plot_xy(22, 7).and_then(|q| self.g.map.plots[q].plot_city).expect("an outpost")
    }
}

fn strs(j: &Json, k: &str) -> Vec<String> {
    j.get(k).and_then(Json::as_arr).unwrap_or(&[]).iter().filter_map(|l| l.as_str().map(str::to_string)).collect()
}

#[test]
fn the_outpost_panel_and_the_cities_list() {
    let mut w = world();
    let c = w.outpost();
    let city = view::city::city(&w.v(), c).expect("its panel");
    let o = city.get("outpost").expect("an outpost part");
    let lines = strs(o, "lines");
    assert!(lines[0].starts_with("Lane: open, to "), "{lines:?}");
    let raise = o.get("raise").unwrap();
    assert_eq!(raise.get("label").and_then(Json::as_str), Some(format!("Raise to city ({} gold)", w.g.outpost_raise_cost()).as_str()));
    w.g.players[w.p0].gold = 0;
    let city = view::city::city(&w.v(), c).unwrap();
    assert_eq!(city.get("outpost").unwrap().get("raise").unwrap().get("enabled").and_then(Json::as_bool), Some(false));
    // The cities: the field; never idle.
    let cities = view::cities(&w.v());
    let row = cities.as_arr().unwrap().iter().find(|r| r.get("id").and_then(Json::as_i64) == Some(c.0 as i64)).unwrap();
    assert!(row.get("outpost").is_some_and(|o| o.get("lane").and_then(Json::as_str) == Some("open")));
    assert_eq!(row.get("idle").and_then(Json::as_bool), Some(false));
    // A city has none.
    let port = w.g.map.plot_xy(12, 10).and_then(|q| w.g.map.plots[q].plot_city).unwrap();
    assert!(view::city::city(&w.v(), port).unwrap().get("outpost").is_none_or(Json::is_null));
}

#[test]
fn lanes_open_and_cut_on_the_sea_layer() {
    let mut w = world();
    let c = w.outpost();
    let lanes = view::outposts::lanes(&w.v());
    let l = &lanes.as_arr().unwrap()[0];
    assert_eq!(l.get("state").and_then(Json::as_str), Some("open"));
    let segs = l.get("segments").and_then(Json::as_arr).unwrap();
    assert_eq!(segs.len(), 1);
    // From the port's end: its last point is next to the outpost.
    let last = segs[0].as_arr().unwrap().last().unwrap().as_arr().unwrap();
    assert_eq!(last[0].as_i64(), Some(21));
    // Cut: two parts and a mark with the turns left.
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_declare_war(t1, t0, false, None);
    let f = w.unit(w.p1, "UNIT_FRIGATE", 17, 8);
    let at = w.g.map.plot_xy(17, 8).unwrap();
    assert!(w.g.unit_can_plunder(f, at, false));
    assert!(w.g.unit_plunder(f));
    assert!(w.g.sea_is_closed_to(w.gap, t0));
    w.g.player_update_lanes(w.p0);
    let lanes = view::outposts::lanes(&w.v());
    let l = &lanes.as_arr().unwrap()[0];
    assert_eq!(l.get("state").and_then(Json::as_str), Some("cut"));
    let cut = l.get("cut").unwrap();
    assert_eq!(cut.get("sea").and_then(Json::as_str), Some("Gap"));
    assert_eq!(cut.get("turns").and_then(Json::as_i64), Some((w.g.d.defines.lane_starve_turns - 1) as i64));
    let lines = strs(view::city::city(&w.v(), c).unwrap().get("outpost").unwrap(), "lines");
    assert_eq!(lines[0], format!("Lane: cut in Gap, turn 1 of {}.", w.g.d.defines.lane_starve_turns));
}

#[test]
fn the_found_outpost_action_and_its_reason() {
    let mut w = world();
    let colonist = w.unit(w.p0, "UNIT_COLONIST", 12, 4);
    let acts = view::units::actions(&w.v(), &[colonist]);
    let a = acts.iter().find(|a| a.get("outpost").is_some_and(|o| o.as_bool() == Some(true))).expect("the action");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(false));
    assert!(a.get("help").and_then(Json::as_str).unwrap().contains("another landmass than your capital's"));
    // No found-city action for the Colonist.
    let found = MissionType::MISSION_FOUND;
    assert!(!acts.iter().any(|a| a.get("mission").and_then(Json::as_i64) == Some(found.idx() as i64) && a.get("outpost").is_none()));
}

#[test]
fn a_resource_not_on_your_landmasses() {
    let mut w = world();
    let silk = w.g.d.t::<BonusType>("BONUS_SILK");
    let far = w.g.map.plot_xy(28, 8).unwrap();
    w.g.plot_set_bonus_type(far, Some(silk));
    let v = w.v();
    assert_eq!(view::outposts::bonus_line(&v, silk), Some("Not on your landmasses."));
    let home = w.g.map.plot_xy(3, 3).unwrap();
    w.g.plot_set_bonus_type(home, Some(silk));
    assert_eq!(view::outposts::bonus_line(&w.v(), silk), None);
}
