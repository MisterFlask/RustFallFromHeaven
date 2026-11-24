//! Waypoints (docs/ARMIES.md, `army/waypoint.rs`): a city's waypoint sends the units it trains to
//! a plot or an army under an ordinary go-to; on the way they are in no army and no garrison; they
//! arrive (join the army, a garrison, an army on the plot, or stay alone); any order of their
//! owner's ends the waypoint; a target gone, disbanded or merged; governed cities; ships and
//! fleets; a city changing hands; a save; determinism. Every situation is built by hand.

use ffh_core::army::Waypoint;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);

/// A duel game (no barbarians) on one flat block of grassland (x 1..=38, y 2..=9) inside coast,
/// with a separate small island (x 30..=32, y 12), a peak on (30, 8) and a pocket on (35, 5) ringed
/// by peaks, revealed to both teams, the starting units gone,
/// player 0 (the Bannor) human with its turn active and Sailing and Bronze Working known. Player 0 has an
/// inland city `home` on (3, 6), a port `port` on (5, 2) and an inland city `east` on (20, 6).
struct World {
    g: Game,
    home: CityId,
    port: CityId,
    east: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    // (The Bannor: no unique unit for the Axeman.)
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 15, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=9).contains(&y) && (1..=38).contains(&x) || y == 12 && (30..=32).contains(&x);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    // A peak on (30, 8), and a pocket on (35, 5) ringed by peaks: no way there.
    let peak = |x: i32, y: i32| (x, y) == (30, 8) || (34..=36).contains(&x) && (4..=6).contains(&y) && (x, y) != (35, 5);
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if peak(x, y) {
            g.plot_set_plot_type(q, PlotType::PLOT_PEAK, false);
        }
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let t0 = g.players[P0].team;
    for tech in ["TECH_SAILING", "TECH_BRONZE_WORKING"] {
        let k = d.t::<TechType>(tech);
        g.team_set_has_tech(t0, k, true, Some(P0), false, false);
    }
    g.players[P0].human = true;
    g.players[P0].turn_active = true;
    let home = g.player_init_city(P0, 3, 6, true, true);
    let port = g.player_init_city(P0, 5, 2, true, true);
    let east = g.player_init_city(P0, 20, 6, true, true);
    World { g, home, port, east }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(P0, ty, x, y, None, None)
    }

    /// An army of `n` warriors on (x, y).
    fn army(&mut self, n: usize, x: i32, y: i32) -> ArmyId {
        let units: Vec<UnitId> = (0..n).map(|_| self.unit("UNIT_WARRIOR", x, y)).collect();
        self.g.army_form(P0, &units).expect("formed")
    }

    /// City `c` finishes a unit of `name` (the order at `index` of its queue, pushed as `push`
    /// does); returns the new unit.
    fn finish(&mut self, c: CityId, name: &str, index: i32) -> UnitId {
        let t = self.g.d.t::<UnitType>(name);
        self.g.cities[c].unit_production[t] = self.g.city_production_needed_unit(c, t);
        let before: Vec<UnitId> = self.g.players[P0].units.clone();
        self.g.city_pop_order(c, index, true, false);
        *self.g.players[P0].units.iter().find(|u| !before.contains(u)).expect("a new unit")
    }

    /// City `c` trains a unit of `name` (an own order, finished at once).
    fn train(&mut self, c: CityId, name: &str) -> UnitId {
        let t = self.g.d.t::<UnitType>(name);
        self.g.city_push_order(c, OrderType::ORDER_TRAIN, t.idx() as i32, -1, false, false, false, true);
        let index = self.g.cities[c].order_queue.iter().position(|o| o.data1 == t.idx() as i32 && !o.empire).expect("the order") as i32;
        self.finish(c, name, index)
    }

    /// The start of player 0's turn (its units' moves back).
    fn start(&mut self) {
        self.g.player_do_turn_units(P0);
    }

    /// Its groups' missions carried on (as the auto moves do), then the end of its turn (its turn
    /// processing: the army sync, then the waypoints).
    fn end(&mut self) {
        for grp in self.g.players[P0].groups.clone() {
            if self.g.groups.contains(grp) {
                // (The interface's move timers have run out.)
                self.g.groups[grp].mission_timer = 0;
                self.g.group_auto_mission(grp);
            }
        }
        // (The turn ends once no unit is busy: the move timers have run out.)
        for grp in self.g.players[P0].groups.clone() {
            if self.g.groups.contains(grp) {
                self.g.groups[grp].mission_timer = 0;
            }
        }
        self.g.player_do_turn(P0);
    }

    fn turn(&mut self) {
        self.start();
        self.end();
    }

    fn head_goto(&self, u: UnitId) -> Option<(i32, i32)> {
        let grp = self.g.units[u].group?;
        self.g.groups[grp].mission_queue.first().filter(|m| m.mission_type == MissionType::MISSION_MOVE_TO).map(|m| (m.data1, m.data2))
    }

    fn told(&self, text: &str) -> bool {
        self.g.log.entries.iter().any(|m| m.text.contains(text))
    }
}

#[test]
fn a_unit_trained_with_an_army_waypoint_walks_there_and_joins_it() {
    let mut w = world();
    let army = w.army(2, 10, 6);
    let (name, grp) = (w.g.armies[army].name.clone(), w.g.armies[army].group);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    assert_eq!(w.g.army_waypoint_cities(army), vec![w.home]);
    assert!(w.g.city_waypoint_turns(w.home, w.g.d.t::<UnitType>("UNIT_WARRIOR")).is_some_and(|t| t >= 5));
    let u = w.train(w.home, "UNIT_WARRIOR");
    // On its way: an ordinary go-to, in no army and no garrison.
    assert_eq!(w.g.units[u].waypoint, Some(Waypoint::Army(army)));
    assert_eq!(w.head_goto(u), Some((10, 6)));
    assert_eq!(w.g.unit_garrison_city(u), None);
    assert!(w.g.city_garrison_units(w.home).is_empty());
    assert_eq!(w.g.army_waypoint_units(army), vec![u]);
    assert_eq!(w.g.player_waypoint_units(P0), vec![u]);
    let eta = w.g.unit_waypoint_eta(u).expect("a way there");
    assert!(eta >= 5, "{eta}");
    w.g.player_army_sync(P0);
    assert_eq!(w.g.units[u].army, None, "the sync leaves it out");
    let mut turns = 0;
    while w.g.units[u].waypoint.is_some() {
        turns += 1;
        assert!(turns <= eta + 1, "arrived within its estimate ({eta})");
        w.start();
        // The army's counter as the turn starts (it is traced then).
        w.g.armies[army].unsupplied_turns = 2;
        w.end();
        if w.g.units[u].waypoint.is_some() {
            assert_eq!(w.g.units[u].army, None, "in no army on the way");
            assert!(w.g.armies.contains(army));
        }
    }
    assert_eq!(w.g.units[u].army, Some(army), "joined");
    assert_eq!(w.g.armies[army].name, name);
    assert_eq!(w.g.armies[army].group, grp, "the army keeps its group");
    assert_eq!(w.g.units[u].group, grp);
    assert_eq!(w.g.armies[army].unsupplied_turns, 2, "the army keeps its counter");
    assert_eq!(w.g.army_units(army).len(), 3);
    assert!(w.told("Warrior joined the 1st Army."));
    assert!(w.g.army_waypoint_units(army).is_empty());
}

#[test]
fn it_follows_an_army_that_moves() {
    let mut w = world();
    let army = w.army(2, 8, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    assert_eq!(w.head_goto(u), Some((8, 6)));
    // The army walks three plots east (one a turn) and stops.
    let grp = w.g.armies[army].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 11, 6, 0, false, false, None, None, None);
    let mut retargeted = false;
    for _ in 0..20 {
        if w.g.units[u].waypoint.is_none() {
            break;
        }
        w.turn();
        if w.g.units[u].waypoint.is_some() {
            let at = w.g.army_plot(army).map(|q| w.g.map.xy(q));
            assert_eq!(w.head_goto(u), at, "its go-to is where the army stands");
            retargeted |= at != Some((8, 6));
        }

    }
    assert!(retargeted, "the go-to followed the army");
    assert_eq!(w.g.units[u].army, Some(army));
    assert_eq!(w.g.army_plot(army), Some(w.at(11, 6)));
}

#[test]
fn an_army_in_the_city_takes_the_unit_at_once() {
    let mut w = world();
    let army = w.army(2, 3, 6);
    assert_eq!(w.g.army_plot(army), Some(w.g.city_plot(w.home)));
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    assert_eq!(w.g.units[u].waypoint, None);
    assert_eq!(w.g.units[u].army, Some(army));
    assert_eq!(w.g.units[u].group, w.g.armies[army].group);
    assert!(w.told("Warrior joined the 1st Army."));
}

#[test]
fn a_plot_waypoint_joins_the_oldest_army_there_else_the_unit_stays_alone() {
    let mut w = world();
    // An empty plot: it stays alone there (a one-unit army by the sync).
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.at(6, 7)))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    assert_eq!(w.head_goto(u), Some((6, 7)));
    for _ in 0..10 {
        if w.g.units[u].waypoint.is_none() {
            break;
        }
        w.turn();
    }
    assert_eq!(w.g.units[u].waypoint, None);
    assert_eq!(w.g.unit_plot(u), w.at(6, 7));
    let alone = w.g.units[u].army.expect("a one-unit army");
    assert_eq!(w.g.army_units(alone), vec![u]);
    // Two armies on a plot: it joins the older.
    let older = w.army(1, 7, 4);
    let newer = w.army(1, 7, 4);
    assert_ne!(older, newer);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.at(7, 4)))).unwrap();
    let v = w.train(w.home, "UNIT_WARRIOR");
    for _ in 0..10 {
        if w.g.units[v].waypoint.is_none() {
            break;
        }
        w.turn();
    }
    assert_eq!(w.g.units[v].army, Some(older));
    assert_eq!(w.g.army_units(newer).len(), 1);
    // A city's plot: its garrison.
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.g.city_plot(w.port)))).unwrap();
    let x = w.train(w.home, "UNIT_WARRIOR");
    for _ in 0..10 {
        if w.g.units[x].waypoint.is_none() {
            break;
        }
        w.turn();
    }
    assert_eq!(w.g.unit_garrison_city(x), Some(w.port));
    assert_eq!(w.g.units[x].army, None);
}

#[test]
fn a_hero_or_mage_follows_a_plot_waypoint_but_no_army() {
    let mut w = world();
    let mage = w.g.d.t::<UnitType>("UNIT_ADEPT");
    let u = w.g.player_init_unit(P0, mage, 3, 6, None, None);
    let army = w.army(1, 8, 6);
    assert!(!w.g.unit_follows_waypoint(u, Waypoint::Army(army)));
    assert!(w.g.unit_follows_waypoint(u, Waypoint::Plot(w.at(8, 6))));
    assert!(!w.g.unit_follows_waypoint(u, Waypoint::Plot(w.at(8, 0))), "not to water");
}

#[test]
fn the_owners_order_ends_the_waypoint() {
    let mut w = world();
    let army = w.army(2, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    // A fortify order in the city: the unit is in its garrison.
    let u = w.train(w.home, "UNIT_WARRIOR");
    let grp = w.g.units[u].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_FORTIFY, -1, -1, 0, false, true, None, None, None);
    assert_eq!(w.g.units[u].waypoint, None);
    w.g.player_army_sync(P0);
    assert_eq!(w.g.unit_garrison_city(u), Some(w.home));
    // A move order on the way: an ordinary one-unit army once outside.
    let v = w.train(w.home, "UNIT_WARRIOR");
    w.turn();
    assert!(w.g.units[v].waypoint.is_some());
    assert_ne!(w.g.unit_plot(v), w.g.city_plot(w.home), "on the way");
    let grp = w.g.units[v].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 4, 8, 0, false, true, None, None, None);
    assert_eq!(w.g.units[v].waypoint, None);
    w.g.player_army_sync(P0);
    assert!(w.g.units[v].army.is_some_and(|a| a != army), "a one-unit army of its own");
    // Joining a group with another unit.
    let x = w.train(w.home, "UNIT_WARRIOR");
    let y = w.train(w.home, "UNIT_WARRIOR");
    let gy = w.g.units[y].group;
    w.g.unit_join_group(x, gy, true);
    assert_eq!(w.g.units[x].waypoint, None);
    assert_eq!(w.g.units[y].waypoint, None);
    // An army command (not refused): the waypoint ends and the command proceeds.
    let z = w.train(w.home, "UNIT_WARRIOR");
    assert!(w.g.units[z].waypoint.is_some());
    let formed = w.g.army_form(P0, &[z]).expect("formed");
    assert_eq!(w.g.units[z].waypoint, None);
    assert_eq!(w.g.units[z].army, Some(formed));
    // The rules' own continuation keeps it (a turn of auto moves).
    let r = w.train(w.home, "UNIT_WARRIOR");
    w.turn();
    assert_eq!(w.g.units[r].waypoint, Some(Waypoint::Army(army)));
}

#[test]
fn an_upgrade_on_the_way_keeps_the_waypoint_and_the_go_to() {
    let mut w = world();
    let army = w.army(2, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    w.g.players[P0].gold = 1000;
    // (Its moves back: an upgrade needs them; it is still in the city.)
    w.start();
    assert_eq!(w.g.unit_plot(u), w.g.city_plot(w.home));
    let axe = w.g.d.t::<UnitType>("UNIT_AXEMAN");
    let yard = w.g.city_building_for_class(w.home, w.g.d.t::<BuildingClassType>("BUILDINGCLASS_TRAINING_YARD")).expect("a training yard");
    w.g.city_set_num_real_building(w.home, yard, 1);
    assert!(w.g.unit_can_upgrade(u, Some(axe), false), "the upgrade is possible here");
    let grp = w.g.units[u].group.unwrap();
    w.g.unit_upgrade(u, Some(axe));
    let n = *w.g.groups[grp].units.first().expect("the new unit in the group");
    assert_ne!(n, u);
    assert_eq!(w.g.units[n].unit_type, axe);
    assert_eq!(w.g.units[n].waypoint, Some(Waypoint::Army(army)));
    assert_eq!(w.head_goto(n), Some((12, 6)));
}

#[test]
fn no_path_the_unit_stays_and_a_lost_way_ends_the_waypoint() {
    let mut w = world();
    // A peak: no way there.
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.at(30, 8)))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    assert_eq!(w.g.units[u].waypoint, None);
    assert!(w.told("Warrior: no way to plot 30, 8."));
    assert_eq!(w.g.unit_garrison_city(u), Some(w.home));
    // A plot target whose go-to the rules dropped.
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.at(9, 6)))).unwrap();
    let v = w.train(w.home, "UNIT_WARRIOR");
    w.turn();
    assert!(w.g.units[v].waypoint.is_some());
    let grp = w.g.units[v].group.unwrap();
    w.g.group_clear_mission_queue(grp);
    w.turn();
    assert_eq!(w.g.units[v].waypoint, None);
    assert!(w.told("Warrior: no way to plot 9, 6. It waits for orders."));
    assert!(w.g.units[v].army.is_some(), "an ordinary unit: a one-unit army outside");
    // An army that went where it cannot be reached.
    let army = w.army(1, 14, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let x = w.train(w.home, "UNIT_WARRIOR");
    assert!(w.g.units[x].waypoint.is_some());
    for a in w.g.army_units(army) {
        w.g.unit_set_xy(a, 35, 5, true);
    }
    w.g.player_army_sync(P0);
    w.turn();
    assert_eq!(w.g.units[x].waypoint, None);
    let gone = format!("Warrior: no way to {}. It waits for orders.", w.g.army_display_name(army));
    assert!(w.told(&gone), "{:?}", w.g.log.entries.iter().map(|m| m.text.clone()).collect::<Vec<_>>());
    assert!(w.g.groups[w.g.units[x].group.unwrap()].mission_queue.is_empty());
}

#[test]
fn an_army_destroyed_or_disbanded_clears_its_waypoints_and_a_merged_one_hands_them_on() {
    let mut w = world();
    // Destroyed.
    let army = w.army(1, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let u = w.train(w.home, "UNIT_WARRIOR");
    for a in w.g.army_units(army) {
        w.g.unit_kill(a, false, None);
    }
    w.g.player_army_sync(P0);
    assert!(!w.g.armies.contains(army));
    assert_eq!(w.g.cities[w.home].waypoint, None);
    assert_eq!(w.g.units[u].waypoint, None);
    assert!(w.g.groups[w.g.units[u].group.unwrap()].mission_queue.is_empty());
    assert!(w.told("waypoint cleared. The 1st Army is gone."));
    assert!(w.told("Warrior: the 1st Army is gone. It waits for orders."));
    // Disbanded (it walks to a city: it stays an army a while, but no waypoint).
    let army = w.army(1, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    let v = w.train(w.home, "UNIT_WARRIOR");
    w.g.army_disband(P0, army).unwrap();
    assert_eq!(w.g.cities[w.home].waypoint, None);
    assert_eq!(w.g.units[v].waypoint, None);
    assert_eq!(w.g.city_check_waypoint(P0, w.home, Some(Waypoint::Army(army))), Err("that army is disbanding"));
    // Merged into another.
    let from = w.army(1, 14, 6);
    let into = w.army(1, 14, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(from))).unwrap();
    let x = w.train(w.home, "UNIT_WARRIOR");
    w.g.army_merge(P0, from, into).unwrap();
    assert!(!w.g.armies.contains(from));
    assert_eq!(w.g.cities[w.home].waypoint, Some(Waypoint::Army(into)));
    assert_eq!(w.g.units[x].waypoint, Some(Waypoint::Army(into)));
    for _ in 0..20 {
        if w.g.units[x].waypoint.is_none() {
            break;
        }
        w.turn();
    }
    assert_eq!(w.g.units[x].army, Some(into));
}

#[test]
fn in_a_governed_city_only_the_empire_orders_units_follow() {
    let mut w = world();
    let army = w.army(1, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    w.g.player_set_city_governed(P0, w.home, true).unwrap();
    w.g.cities[w.home].order_queue.clear();
    // The governor's own unit stays.
    let own = w.train(w.home, "UNIT_WARRIOR");
    assert_eq!(w.g.units[own].waypoint, None);
    assert_eq!(w.g.unit_garrison_city(own), Some(w.home));
    // An empire order's unit goes.
    let t = w.g.d.t::<UnitType>("UNIT_WARRIOR");
    w.g.cities[w.home].order_queue.clear();
    w.g.city_push_empire_order(P0, w.home, OrderType::ORDER_TRAIN, t.idx() as i32, -1).unwrap();
    let emp = w.finish(w.home, "UNIT_WARRIOR", 0);
    assert_eq!(w.g.units[emp].waypoint, Some(Waypoint::Army(army)));
}

#[test]
fn settlers_and_caravans_stay() {
    let mut w = world();
    let army = w.army(1, 12, 6);
    for wp in [Waypoint::Army(army), Waypoint::Plot(w.at(9, 6))] {
        w.g.city_set_waypoint(P0, w.home, Some(wp)).unwrap();
        for name in ["UNIT_SETTLER", "UNIT_CARAVAN"] {
            let u = w.train(w.home, name);
            assert_eq!(w.g.units[u].waypoint, None, "{name}");
            assert!(w.head_goto(u).is_none(), "{name}");
            assert_eq!(w.g.unit_plot(u), w.g.city_plot(w.home));
        }
    }
}

#[test]
fn a_ship_follows_a_fleet_waypoint_from_a_port() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 10, 1);
    w.g.player_army_sync(P0);
    let fleet = w.g.units[ship].army.expect("a lone warship at sea is a fleet");
    assert!(w.g.army_is_fleet(fleet));
    assert_eq!(w.g.city_check_waypoint(P0, w.home, Some(Waypoint::Army(fleet))), Err("a fleet is a waypoint only for a coastal city"));
    w.g.city_set_waypoint(P0, w.port, Some(Waypoint::Army(fleet))).unwrap();
    // A land unit does not serve in a fleet.
    let warrior = w.train(w.port, "UNIT_WARRIOR");
    assert_eq!(w.g.units[warrior].waypoint, None);
    let t = w.train(w.port, "UNIT_TRIREME");
    assert_eq!(w.g.units[t].waypoint, Some(Waypoint::Army(fleet)));
    for _ in 0..20 {
        if w.g.units[t].waypoint.is_none() {
            break;
        }
        w.turn();
    }
    assert_eq!(w.g.units[t].army, Some(fleet));
    assert_eq!(w.g.army_units(fleet).len(), 2);
    // A plot waypoint: ships to water, not land.
    assert!(w.g.unit_follows_waypoint(t, Waypoint::Plot(w.at(12, 0))));
    assert!(!w.g.unit_follows_waypoint(t, Waypoint::Plot(w.at(12, 5))));
}

#[test]
fn setting_a_waypoint_is_checked() {
    let mut w = world();
    let p1 = PlayerId(1);
    assert_eq!(w.g.city_set_waypoint(p1, w.home, None), Err("not your city"));
    let t = w.g.players[P0].team;
    let q = w.at(30, 8);
    w.g.map.plots[q].revealed[t] = false;
    assert_eq!(w.g.city_check_waypoint(P0, w.home, Some(Waypoint::Plot(q))), Err("that plot is not revealed"));
    assert_eq!(w.g.city_check_waypoint(P0, w.home, Some(Waypoint::Plot(PlotIdx(100_000)))), Err("that plot is not on the map"));
    assert_eq!(w.g.city_check_waypoint(P0, w.home, Some(Waypoint::Army(ArmyId(999)))), Err("no such army"));
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Plot(w.at(9, 6)))).unwrap();
    assert_eq!(w.g.waypoint_name(Waypoint::Plot(w.at(9, 6))), "plot 9, 6");
    w.g.city_set_waypoint(P0, w.home, None).unwrap();
    assert_eq!(w.g.cities[w.home].waypoint, None);
}

#[test]
fn a_captured_city_has_no_waypoint() {
    let mut w = world();
    w.g.city_set_waypoint(P0, w.east, Some(Waypoint::Plot(w.at(9, 6)))).unwrap();
    let plot = w.g.city_plot(w.east);
    w.g.player_acquire_city(PlayerId(1), w.east, false, true, true);
    let c = w.g.map.plots[plot].plot_city.expect("a city there");
    assert_eq!(w.g.cities[c].owner, PlayerId(1));
    assert_eq!(w.g.cities[c].waypoint, None);
}

/// A scenario with units on their way and cities with waypoints.
fn scenario() -> World {
    let mut w = world();
    let army = w.army(2, 12, 6);
    w.g.city_set_waypoint(P0, w.home, Some(Waypoint::Army(army))).unwrap();
    w.g.city_set_waypoint(P0, w.east, Some(Waypoint::Plot(w.at(16, 4)))).unwrap();
    w.train(w.home, "UNIT_WARRIOR");
    w.train(w.east, "UNIT_WARRIOR");
    w.turn();
    w.train(w.home, "UNIT_WARRIOR");
    w
}

#[test]
fn waypoints_survive_a_save() {
    let w = scenario();
    assert!(!w.g.player_waypoint_units(P0).is_empty());
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.cities[w.home].waypoint, w.g.cities[w.home].waypoint);
    assert_eq!(h.cities[w.east].waypoint, w.g.cities[w.east].waypoint);
    for &u in &w.g.players[P0].units {
        assert_eq!(h.units[u].waypoint, w.g.units[u].waypoint);
    }
    assert_eq!(h.save(), bytes);
}

#[test]
fn waypoints_are_deterministic() {
    let mut a = scenario();
    let mut b = scenario();
    assert_eq!(a.g.save(), b.g.save());
    for _ in 0..8 {
        a.turn();
        b.turn();
        assert_eq!(a.g.save(), b.g.save());
    }
}




