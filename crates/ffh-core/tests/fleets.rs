//! The sea, phase 3 (docs/NAVAL.md, section 7): fleets. The sync (a lone warship at sea is a
//! fleet, in port it is not; a transport and the army it carries are two containers; the army
//! lands as one), names numbered apart from the armies, split and merge, requisition from a
//! coastal city (an inland city refused; the ship sails to the fleet and joins), disband to port,
//! no supply rule, the Patrol standing order across turns, Escort, a move order ending a standing
//! order, and a save; on situations built by hand.

use ffh_core::army::{ArmyDomain, EscortTarget, RecruitDest, Station};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians) on one flat block of grassland (x 1..=38, y 2..=9) inside a ring of
/// coast, revealed to both teams, the teams met, the starting units gone, player 0's turn active,
/// Sailing and Bronze Working known; player 0 has ports on (5, 2) and (20, 2) and an inland city
/// on (12, 6). Seas cut by hand: West (y 0..=1, x 0..=12), East (y 0..=1, x 13..=39), South
/// (y 10..=11).
struct World {
    g: Game,
    p0: PlayerId,
    port: CityId,
    port2: CityId,
    inland: CityId,
    west: SeaId,
    east: SeaId,
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
    let band = |g: &Game, xs: std::ops::RangeInclusive<i32>, ys: std::ops::RangeInclusive<i32>| {
        let mut v = Vec::new();
        for y in ys.clone() {
            for x in xs.clone() {
                v.push(g.map.plot_xy(x, y).unwrap());
            }
        }
        v
    };
    let west = g.map_cut_sea("West", &band(&g, 0..=12, 0..=1));
    let east = g.map_cut_sea("East", &band(&g, 13..=39, 0..=1));
    g.map_cut_sea("South", &band(&g, 0..=39, 10..=11));
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    for tech in ["TECH_SAILING", "TECH_BRONZE_WORKING"] {
        let k = d.t::<TechType>(tech);
        g.team_set_has_tech(t0, k, true, Some(p0), false, false);
    }
    g.players[p0].turn_active = true;
    let port = g.player_init_city(p0, 5, 2, true, true);
    let port2 = g.player_init_city(p0, 20, 2, true, true);
    let inland = g.player_init_city(p0, 12, 6, true, true);
    World { g, p0, port, port2, inland, west, east }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(self.p0, ty, x, y, None, None)
    }

    fn sync(&mut self) {
        let p0 = self.p0;
        self.g.player_army_sync(p0);
    }

    fn army(&self, u: UnitId) -> Option<ArmyId> {
        self.g.units[u].army
    }

    fn name(&self, u: UnitId) -> String {
        self.g.armies[self.army(u).expect("in an army")].name.clone()
    }

    /// Moves `u`'s group one step to `(x, y)`.
    fn step(&mut self, u: UnitId, x: i32, y: i32) {
        let grp = self.g.units[u].group.unwrap();
        let to = self.at(x, y);
        self.g.group_move(grp, to, false, None, false);
    }

    /// The start of player 0's turn (its units' moves back), its groups' missions carried on, then
    /// the end of its turn (the recruits march, the sync, the fleets' standing orders).
    fn turn(&mut self) {
        let p0 = self.p0;
        self.g.player_do_turn_units(p0);
        for grp in self.g.players[p0].groups.clone() {
            if self.g.groups.contains(grp) {
                self.g.group_auto_mission(grp);
            }
        }
        self.g.player_do_march(p0);
        self.g.player_march_join(p0);
        self.g.player_army_sync(p0);
        self.g.player_fleet_stations(p0);
    }
}

#[test]
fn a_lone_warship_at_sea_is_a_fleet_in_port_it_is_not() {
    let mut w = world();
    let at_sea = w.unit("UNIT_TRIREME", 8, 1);
    let in_port = w.unit("UNIT_TRIREME", 5, 2);
    let soldier = w.unit("UNIT_WARRIOR", 8, 4);
    w.sync();
    let f = w.army(at_sea).expect("a fleet");
    assert_eq!(w.g.armies[f].domain, ArmyDomain::Sea);
    assert_eq!(w.name(at_sea), "1st Fleet");
    assert_eq!(w.army(in_port), None, "a ship in port in no fleet is simply a ship");
    assert_eq!(w.name(soldier), "1st Army", "numbered apart from the fleets");
    assert_eq!(w.g.armies[w.army(soldier).unwrap()].domain, ArmyDomain::Land);
    // The ship in port leaves it: a fleet of its own.
    w.step(in_port, 6, 1);
    w.sync();
    assert_eq!(w.name(in_port), "2nd Fleet");
    // The fleet enters the port: it stays a fleet; disbanded there, its ship is simply a ship.
    let a = w.army(in_port).unwrap();
    w.g.units[in_port].moves = 0;
    w.step(in_port, 5, 2);
    w.sync();
    assert_eq!(w.army(in_port), Some(a), "a fleet entering a city stays a fleet");
    w.g.army_disband(w.p0, a).unwrap();
    assert_eq!(w.army(in_port), None);
    assert!(!w.g.armies.contains(a));
    // Ships are no army units and land units no fleet units.
    assert!(w.g.army_form(w.p0, &[at_sea, soldier]).is_err(), "not both at once");
    assert!(w.g.army_join(w.p0, &[soldier], f).is_err());
}

#[test]
fn a_transport_and_its_cargo_are_two_containers_and_the_army_lands_as_one() {
    let mut w = world();
    let galleon = w.unit("UNIT_GALLEON", 8, 1);
    let men: Vec<UnitId> = (0..2).map(|_| w.unit("UNIT_WARRIOR", 8, 2)).collect();
    w.sync();
    let army = w.g.army_form(w.p0, &men).unwrap();
    let fleet = w.army(galleon).unwrap();
    assert!(w.g.army_is_fleet(fleet) && !w.g.army_is_fleet(army));
    // The army boards: each unit aboard has a group of its own, but stays in the army.
    w.step(men[0], 8, 1);
    for &m in &men {
        assert!(w.g.unit_is_cargo(m), "aboard");
    }
    w.sync();
    for &m in &men {
        assert_eq!(w.army(m), Some(army), "the cargo stays in its own army");
    }
    assert_eq!(w.army(galleon), Some(fleet));
    assert!(w.g.armies[army].carried);
    assert_eq!(w.g.players[w.p0].armies.len(), 2, "two containers");
    // The transport sails; the cargo goes with it.
    w.step(galleon, 9, 1);
    w.sync();
    assert_eq!(w.g.army_plot(army), Some(w.at(9, 1)));
    // It lands, unit by unit: one army again, in one group.
    for &m in &men {
        w.g.units[m].moves = 0;
        w.step(m, 9, 2);
    }
    w.sync();
    assert_eq!(w.army(men[0]), Some(army));
    assert_eq!(w.army(men[1]), Some(army), "landing did not split it");
    assert_eq!(w.g.units[men[0]].group, w.g.units[men[1]].group);
    assert!(!w.g.armies[army].carried);
}

#[test]
fn split_merge_rename_upgrade_and_target_size() {
    let mut w = world();
    let ships: Vec<UnitId> = (0..3).map(|_| w.unit("UNIT_TRIREME", 8, 1)).collect();
    let f = w.g.army_form(w.p0, &ships).unwrap();
    assert_eq!(w.g.army_units(f).len(), 3);
    let g2 = w.g.army_split(w.p0, &ships[..1]).unwrap();
    assert_eq!(w.g.armies[g2].name, "2nd Fleet");
    assert_eq!(w.g.army_units(f).len(), 2);
    w.g.army_merge(w.p0, g2, f).unwrap();
    assert!(!w.g.armies.contains(g2));
    assert_eq!(w.g.army_units(f).len(), 3);
    w.g.army_rename(w.p0, f, "Home Squadron").unwrap();
    assert_eq!(w.g.armies[f].name, "Home Squadron");
    w.g.army_set_target_size(w.p0, f, 5).unwrap();
    assert_eq!(w.g.army_shortfall(f), 2);
    assert!(w.g.army_set_tactic(w.p0, f, w.g.armies[f].tactic).is_err(), "no tactic at sea");
    assert!(w.g.army_upgrade_all(w.p0, f).is_ok());
}

#[test]
fn requisition_from_a_coastal_city_sails_to_the_fleet() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 9, 1);
    w.sync();
    let f = w.army(ship).unwrap();
    assert!(w.g.army_add_requisition(w.p0, f, w.inland).is_err(), "an inland city is refused");
    w.g.army_add_requisition(w.p0, f, w.port).unwrap();
    w.g.army_set_target_size(w.p0, f, 2).unwrap();
    w.g.player_do_requisition(w.p0);
    let o = w.g.cities[w.port].order_queue[0].clone();
    assert!(o.requisition, "a requisition order at the front");
    assert_eq!(o.dest, Some(RecruitDest::Army(f)));
    let ty = UnitType::from_usize(o.data1 as usize);
    assert!(w.g.unit_type_is_fleet_unit(ty), "a ship for the fleet");
    assert_eq!(w.g.army_shortfall(f), 0);
    // The ship is trained: it sails to the fleet as a recruit marches, and joins it.
    let new = w.g.player_init_unit(w.p0, ty, 5, 2, Some(UnitAIType::UNITAI_ATTACK_SEA), None);
    assert_eq!(w.g.city_recruit_dest(w.port, o.dest, ty), Some(RecruitDest::Army(f)));
    w.g.unit_recruit_to(new, w.port, RecruitDest::Army(f));
    assert!(w.g.unit_march(new).is_some(), "it sails on its own");
    assert!(w.g.log.entries.iter().any(|m| m.text.contains("sails to the 1st Fleet")));
    for _ in 0..6 {
        if w.army(new) == Some(f) {
            break;
        }
        w.turn();
    }
    assert_eq!(w.army(new), Some(f), "joined the fleet");
    assert_eq!(w.g.units[new].group, w.g.units[ship].group);
    // A land unit ordered for the fleet stays in its city.
    let warrior = w.g.d.t::<UnitType>("UNIT_WARRIOR");
    assert_eq!(w.g.city_recruit_dest(w.port, Some(RecruitDest::Army(f)), warrior), None);
}

#[test]
fn a_disbanded_fleet_sails_to_port_and_its_ships_are_then_simply_ships() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 10, 0);
    w.sync();
    let f = w.army(ship).unwrap();
    w.g.army_disband(w.p0, f).unwrap();
    assert_eq!(w.g.armies.get(f).and_then(|a| a.disband_to), Some(w.port), "the nearest port");
    for _ in 0..6 {
        if !w.g.armies.contains(f) {
            break;
        }
        w.turn();
    }
    assert!(!w.g.armies.contains(f), "gone in port");
    assert_eq!(w.g.unit_plot(ship), w.g.city_plot(w.port));
    assert_eq!(w.army(ship), None);
}

#[test]
fn a_fleet_has_no_supply_rule() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 36, 11);
    w.sync();
    let f = w.army(ship).unwrap();
    for _ in 0..4 {
        w.g.player_update_supply(w.p0);
    }
    assert_eq!(w.g.armies[f].unsupplied_turns, 0, "never hungry");
    assert!(w.g.army_in_supply(f));
}

#[test]
fn patrol_standing_order_sails_over_turns_and_patrols_the_sea() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 27, 0);
    w.sync();
    let f = w.army(ship).unwrap();
    assert!(w.g.fleet_patrol_seas(w.p0).contains(&w.west));
    w.g.army_set_station(w.p0, f, Station::Patrol(w.west)).unwrap();
    let mut turns = 0;
    while !w.g.units[ship].blockading && turns < 12 {
        w.turn();
        turns += 1;
    }
    assert!(turns > 1, "several turns on the way ({turns})");
    assert!(w.g.units[ship].blockading, "on Patrol");
    assert_eq!(w.g.map.plots[w.g.unit_plot(ship)].sea, Some(w.west));
    assert_eq!(w.g.armies[f].station, Station::Patrol(w.west), "the order stands");
    // A turn later it is still there, on Patrol.
    w.turn();
    assert!(w.g.units[ship].blockading);
    // A move order ends it.
    let grp = w.g.armies[f].group.unwrap();
    let to = w.at(13, 0);
    let (x, y) = w.g.map.xy(to);
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, x, y, 0, false, true, None, None, None);
    assert_eq!(w.g.armies[f].station, Station::None);
    // A sea out of range is refused (South: more than 8 plots of water from both ports).
    let south = w.g.map.plots[w.at(20, 11)].sea.expect("a sea");
    assert!(!w.g.fleet_patrol_seas(w.p0).contains(&south));
    assert!(w.g.army_set_station(w.p0, f, Station::Patrol(south)).is_err());
    assert!(w.g.fleet_patrol_seas(w.p0).contains(&w.east));
    let _ = w.port2;
}

#[test]
fn escort_follows_a_group_in_boats_and_waits_when_it_lands() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 14, 0);
    let soldier = w.unit("UNIT_WARRIOR", 15, 2);
    w.sync();
    let f = w.army(ship).unwrap();
    let a = w.army(soldier).unwrap();
    // Into boats, beside the fleet.
    w.step(soldier, 15, 1);
    assert!(w.g.unit_is_in_boats(soldier));
    w.g.army_set_station(w.p0, f, Station::Escort(EscortTarget::Army(a))).unwrap();
    assert_eq!(w.g.unit_plot(ship), w.at(15, 1), "joined its target at once");
    // Next turn the group in boats moves on: the fleet follows it.
    w.turn();
    w.step(soldier, 16, 1);
    assert_eq!(w.g.unit_plot(soldier), w.at(16, 1));
    assert_eq!(w.g.unit_plot(ship), w.at(16, 1), "the escort followed");
    assert_eq!(w.g.armies[f].station, Station::Escort(EscortTarget::Army(a)));
    // It lands: the fleet waits where it is.
    w.turn();
    w.step(soldier, 16, 2);
    assert!(!w.g.unit_is_in_boats(soldier));
    w.turn();
    assert_eq!(w.g.unit_plot(ship), w.at(16, 1), "waits beside it");
}

#[test]
fn two_fleets_escorting_each_other_do_not_loop() {
    let mut w = world();
    let s1 = w.unit("UNIT_TRIREME", 14, 0);
    let s2 = w.unit("UNIT_TRIREME", 16, 0);
    w.sync();
    let (f1, f2) = (w.army(s1).unwrap(), w.army(s2).unwrap());
    w.g.army_set_station(w.p0, f1, Station::Escort(EscortTarget::Army(f2))).unwrap();
    w.g.army_set_station(w.p0, f2, Station::Escort(EscortTarget::Army(f1))).unwrap();
    w.turn();
    let before = w.g.unit_plot(s2);
    w.g.units[s2].moves = 0;
    let (x, y) = w.g.map.xy(before);
    w.step(s2, x + 1, y);
    assert_eq!(w.g.unit_plot(s1), w.g.unit_plot(s2), "one follows the other");
}

#[test]
fn fleets_survive_a_save() {
    let mut w = world();
    let ship = w.unit("UNIT_TRIREME", 8, 1);
    w.sync();
    let f = w.army(ship).unwrap();
    w.g.army_set_station(w.p0, f, Station::Patrol(w.west)).unwrap();
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.armies[f].domain, ArmyDomain::Sea);
    assert_eq!(h.armies[f].station, Station::Patrol(w.west));
    assert_eq!(h.armies[f].name, "1st Fleet");
    assert_eq!(h.save(), bytes, "the same bytes again");
}
