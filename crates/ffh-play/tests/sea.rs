//! The sea, phase 1, as the human sees it (docs/NAVAL.md, section 10; `view/sea.rs`): the map's
//! `sea` array (clipped to revealed plots), `me.sea_range`, a port's mark, the unit panel's lines,
//! the path preview's lines, the actions a unit in boats is offered, the combat preview's wear and
//! boats, and an army's supply at sea; on situations built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::path::{FinderKind, PathFinder};
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

/// A duel game on a block of grassland (x 1..=38, y 2..=9) between two bands of coast (y 0..=1
/// and 10..=11); player 0 has a port on (5, 2) and has revealed x < 30 only; player 1 no city.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    // The human (the AI's path finder keeps land groups off the water).
    setup.players[0].human = true;
    let mut g = Game::new(d, setup);
    g.game_init();
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
            if g.map.xy(q).0 < 30 {
                g.plot_set_revealed(q, t, true, false, None, false);
            }
        }
    }
    g.update_plot_groups();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    World { g, p0, p1 }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn port(&mut self) -> CityId {
        self.g.player_init_city(self.p0, 5, 2, true, true)
    }

    fn v(&self) -> Viewer<'_> {
        Viewer::new(&self.g, self.p0, false)
    }
}

fn lines(j: &Json) -> Vec<String> {
    j.get("lines").and_then(Json::as_arr).unwrap_or(&[]).iter().filter_map(|l| l.as_str().map(str::to_string)).collect()
}

fn int(j: &Json, k: &str) -> Option<i64> {
    j.get(k).and_then(Json::as_i64)
}

#[test]
fn the_sea_array_is_the_range_clipped_to_revealed_water() {
    let mut w = world();
    // No city yet: no range rule, nothing to show.
    let rows = view::map::rows(&w.v());
    let sea = view::map::LAYERS.iter().position(|&l| l == "sea").expect("a sea layer");
    assert!(rows.rows.iter().all(|r| r[sea] == -1));
    assert!(view::me(&w.v()).get("sea_range").is_some_and(Json::is_null));
    w.port();
    let rows = view::map::rows(&w.v());
    let at = |x: i32, y: i32| rows.rows[(y * 40 + x) as usize][sea];
    assert_eq!(at(5, 1), 1, "next to the port");
    assert_eq!(at(10, 1), 5);
    assert_eq!(at(13, 1), 8, "the range's last plot");
    assert_eq!(at(14, 1), 0, "beyond it: only out");
    assert_eq!(at(10, 5), -1, "land");
    assert_eq!(at(35, 1), -1, "not revealed");
    assert_eq!(rows.rows[(35 + 40) as usize][0], 0, "unrevealed");
    assert_eq!(int(&view::me(&w.v()), "sea_range"), Some(8));
}

#[test]
fn a_port_is_marked_for_its_team() {
    let mut w = world();
    let c = w.port();
    let inland = w.g.player_init_city(w.p0, 20, 6, true, true);
    let cities = view::cities(&w.v());
    let port_of = |id: CityId| cities.as_arr().unwrap().iter().find(|j| int(j, "id") == Some(id.0 as i64)).and_then(|j| j.get("sea_port")).and_then(Json::as_bool);
    assert_eq!(port_of(c), Some(true));
    assert_eq!(port_of(inland), Some(false));
}

#[test]
fn the_unit_panel_says_range_wear_and_boats() {
    let mut w = world();
    w.port();
    let near = w.unit(w.p0, "UNIT_GALLEY", 8, 1);
    let far = w.unit(w.p0, "UNIT_GALLEY", 20, 1);
    let v = w.v();
    let sea = |u| view::units::unit_details(&v, u).get("sea").cloned().unwrap();
    assert_eq!(lines(&sea(near)), ["Range: in range (3 of 8).", "Wear: none (turn 0 of 10)."]);
    assert_eq!(lines(&sea(far)), ["Out of range: -15% HP a turn.", "Wear: none (turn 0 of 10)."]);
    w.g.units[near].sea_turns = 13;
    let v = w.v();
    let s = view::units::unit_details(&v, near).get("sea").cloned().unwrap();
    assert_eq!(lines(&s)[1], "Wear: -15% strength. Enter a port to clear.");
    assert_eq!(int(&view::unit_json(&v, near), "wear"), Some(15));
    // A unit in boats.
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    let v = w.v();
    assert!(view::units::unit_details(&v, u).get("sea").is_some_and(Json::is_null), "on land: nothing");
    w.g.unit_move(u, w.at(10, 1));
    let v = w.v();
    assert_eq!(view::unit_json(&v, u).get("boats").and_then(Json::as_bool), Some(true));
    let s = view::units::unit_details(&v, u).get("sea").cloned().unwrap();
    assert_eq!(lines(&s), ["In boats: strength 1, cannot attack, does not heal.", "Range: in range (5 of 8)."]);
}

#[test]
fn the_path_preview_says_where_boats_and_range_begin() {
    let mut w = world();
    w.port();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    let mut finder = PathFinder::new(FinderKind::Path);
    let v = w.v();
    let p = view::units::path(&v, &mut finder, &[u], 16, 1, 0).expect("a path");
    assert_eq!(p.get("ok").and_then(Json::as_bool), Some(true), "{p}");
    let sea = p.get("sea").cloned().expect("sea");
    let l = lines(&sea);
    assert_eq!(l.len(), 2, "{sea}");
    let nodes = p.get("nodes").and_then(Json::as_arr).unwrap();
    let xy = |i: usize| (int(&nodes[i], "x").unwrap(), int(&nodes[i], "y").unwrap());
    let boats = int(&sea, "boats_from").expect("in boats on the way") as usize;
    assert!(xy(boats).1 <= 1 && (boats == 1 || xy(boats - 1).1 >= 2), "the first water plot: {:?}", xy(boats));
    assert!(l[0].starts_with(&format!("In boats from plot {boats}: strength 1, 1 plot a turn. ")), "{}", l[0]);
    let out = int(&sea, "out_from").expect("out of range on the way") as usize;
    assert!(xy(out).0 >= 14 && xy(out).1 <= 1, "out of range: {:?}", xy(out));
    assert!((boats..out).all(|i| xy(i).1 > 1 || xy(i).0 <= 13), "the first plot out of range");
    assert_eq!(l[1], format!("Out of range from plot {out}: -15% HP a turn."));
    // Over land: nothing.
    let p = view::units::path(&v, &mut finder, &[u], 12, 4, 0).expect("a path");
    assert!(p.get("sea").is_some_and(Json::is_null));
}

#[test]
fn in_boats_only_moving_waiting_and_deleting_are_offered() {
    let mut w = world();
    w.port();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    let v = w.v();
    let actions = view::units::actions(&v, &[u]);
    let missions: Vec<i64> = actions.iter().filter(|a| a.get("kind").and_then(Json::as_str) == Some("mission")).filter_map(|a| int(a, "mission")).collect();
    let ok = [MissionType::MISSION_SKIP, MissionType::MISSION_SLEEP, MissionType::MISSION_SENTRY].map(|m| m.idx() as i64);
    assert!(!missions.is_empty(), "it can still wait");
    for m in &missions {
        assert!(ok.contains(m), "mission {m} offered in boats");
    }
    let fortify = MissionType::MISSION_FORTIFY.idx() as i64;
    let on_land = w.unit(w.p0, "UNIT_WARRIOR", 12, 4);
    let v = w.v();
    assert!(view::units::actions(&v, &[on_land]).iter().any(|a| int(a, "mission") == Some(fortify)), "on land it fortifies");
    let modes: Vec<&str> = actions.iter().filter_map(|a| a.get("mode").and_then(Json::as_str)).collect();
    assert!(modes.iter().all(|&m| m == "INTERFACEMODE_GO_TO"), "{modes:?}");
}

#[test]
fn an_army_at_sea_reads_supplied_at_sea() {
    let mut w = world();
    w.port();
    let units: Vec<UnitId> = (0..2).map(|_| w.unit(w.p0, "UNIT_WARRIOR", 10, 2)).collect();
    let a = w.g.army_form(w.p0, &units).expect("an army");
    for &u in &units {
        w.g.unit_move(u, w.at(10, 1));
    }
    let v = w.v();
    let s = view::armies::supply_json(&v, a);
    let name = view::city_name(&w.g, w.g.players[w.p0].cities[0]);
    assert_eq!(s.get("text").and_then(Json::as_str), Some(format!("Supplied: at sea, in range of {name}.").as_str()));
    assert_eq!(s.get("line").and_then(|l| l.get("source_kind")).and_then(Json::as_str), Some("sea"));
}

#[test]
fn the_combat_preview_shows_wear_and_boats() {
    let mut w = world();
    w.port();
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_declare_war(t0, t1, false, None);
    let galley = w.unit(w.p0, "UNIT_TRIREME", 9, 1);
    w.g.units[galley].sea_turns = 12;
    let theirs = w.unit(w.p1, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(theirs, w.at(10, 1));
    assert!(w.g.unit_is_in_boats(theirs));
    let v = w.v();
    let c = view::units::combat(&v, &[galley], 10, 1).expect("a preview");
    assert_eq!(c.get("valid").and_then(Json::as_bool), Some(true), "{c}");
    assert_eq!(c.get("defender_in_boats").and_then(Json::as_bool), Some(true));
    assert_eq!(c.get("attacker_details").and_then(|d| int(d, "sea_wear")), Some(-10));
}
