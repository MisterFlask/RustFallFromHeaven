//! The sea, phase 1 (docs/NAVAL.md, section 3): the range field (ports, ties, techs, lakes, open
//! borders), attrition and its exemptions, boats (moves, the Lanun's two plots, strength 1, no
//! attack, loading, landing, no healing), a ship sinking a unit in boats by the duel, a warship
//! defending first, supply at sea, wear and its reset, and a save of the new fields; on
//! situations built by hand.

use ffh_core::army::SupplySource;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians) on one flat block of grassland (x 1..=38, y 2..=9) between two
/// bands of coast (y 0..=1 and 10..=11), revealed to both teams, the teams met, the starting units
/// gone, player 0's turn active; player 0 has a port on (5, 2), player 1 no city.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    port: CityId,
}

fn world_as(civ0: Option<&str>) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    if let Some(name) = civ0 {
        let civ = d.t::<CivilizationType>(name);
        setup.players[0].civ = Some(civ);
        setup.players[0].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
    }
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
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    let port = g.player_init_city(p0, 5, 2, true, true);
    World { g, p0, p1, port }
}

fn world() -> World {
    world_as(None)
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

    fn range(&self, x: i32, y: i32) -> Option<i32> {
        self.g.plot_sea_range(self.t0(), self.at(x, y))
    }

    fn war(&mut self) {
        let (t0, t1) = (self.t0(), self.g.players[self.p1].team);
        self.g.team_declare_war(t0, t1, false, None);
    }

    fn logged(&self, text: &str) -> usize {
        self.g.log.entries.iter().filter(|m| m.text.contains(text)).count()
    }

    /// A new turn's moves for `u`.
    fn rest(&mut self, u: UnitId) {
        self.g.units[u].moves = 0;
    }
}

// ---- The range field (3.1) ----

#[test]
fn the_defines_and_data() {
    let w = world();
    let d = &w.g.d.defines;
    assert_eq!((d.sea_range, d.sea_attrition_percent, d.embarked_moves, d.embarked_strength), (8, 15, 1, 1));
    assert_eq!((d.sea_wear_free_turns, d.sea_wear_percent, d.sea_wear_max_percent), (10, 5, 50));
    let tech = |n: &str| w.g.d.techs[w.g.d.t::<TechType>(n)].sea_range_change;
    assert_eq!((tech("TECH_OPTICS"), tech("TECH_ASTRONOMY")), (4, 4));
    let lanun = w.g.d.t::<CivilizationType>("CIVILIZATION_LANUN");
    assert_eq!(w.g.d.civilizations[lanun].embarked_moves_change, 1);
    let ocean = w.g.d.t::<TerrainType>("TERRAIN_OCEAN");
    for ship in ["UNIT_GALLEY", "UNIT_TRIREME"] {
        assert!(!w.g.d.units[w.g.d.t::<UnitType>(ship)].terrain_impassable[ocean], "{ship} crosses the ocean");
    }
}

#[test]
fn a_port_gives_range_over_water() {
    let w = world();
    let t0 = w.t0();
    assert_eq!(w.range(5, 1), Some(1), "next to the port");
    assert_eq!(w.range(4, 0), Some(2));
    assert_eq!(w.range(13, 1), Some(8));
    assert_eq!(w.range(14, 1), Some(9));
    assert_eq!(w.range(5, 5), None, "land");
    assert_eq!(w.g.team_sea_range(t0), 8);
    assert!(w.g.plot_in_sea_range(t0, w.at(13, 1)));
    assert!(!w.g.plot_in_sea_range(t0, w.at(14, 1)));
    assert!(w.g.plot_in_sea_range(t0, w.at(20, 5)), "land is always in range");
    assert_eq!(w.g.plot_sea_port(t0, w.at(13, 1)), Some(w.port));
    // A team with no city has no range rule.
    let t1 = w.g.players[w.p1].team;
    assert!(w.g.team_sea_exempt(t1));
    assert!(w.g.plot_in_sea_range(t1, w.at(30, 1)));
}

#[test]
fn two_ports_the_nearest_counts_and_ties_go_to_the_lower_city() {
    let mut w = world();
    let second = w.g.player_init_city(w.p0, 21, 2, true, true);
    let t0 = w.t0();
    assert_eq!(w.range(18, 1), Some(3));
    assert_eq!(w.g.plot_sea_port(t0, w.at(18, 1)), Some(second));
    // (13, 1): 8 steps from either port.
    assert_eq!(w.range(13, 1), Some(8));
    assert_eq!(w.g.plot_sea_port(t0, w.at(13, 1)), Some(w.port.min(second)));
    // The second city gone: the field follows.
    w.g.city_kill(second, false);
    assert_eq!(w.range(18, 1), Some(13));
}

#[test]
fn a_tech_adds_to_the_range() {
    let mut w = world();
    let t0 = w.t0();
    let optics = w.g.d.t::<TechType>("TECH_OPTICS");
    w.g.team_set_has_tech(t0, optics, true, Some(w.p0), false, false);
    assert_eq!(w.g.team_sea_range(t0), 12);
    assert!(w.g.plot_in_sea_range(t0, w.at(17, 1)));
    assert!(!w.g.plot_in_sea_range(t0, w.at(18, 1)));
}

#[test]
fn a_lake_no_port_touches_is_out_of_reach() {
    let mut w = world();
    let coast = w.g.d.t::<TerrainType>("TERRAIN_COAST");
    let lake = w.at(30, 6);
    w.g.plot_set_plot_type(lake, PlotType::PLOT_OCEAN, false);
    w.g.plot_set_terrain_type(lake, coast, false);
    w.g.map_recalculate_areas();
    assert_eq!(w.range(30, 6), None, "no port's water reaches it");
    assert!(!w.g.plot_in_sea_range(w.t0(), lake));
    // A city beside it makes it a port's water, as any.
    w.g.player_init_city(w.p0, 31, 6, true, true);
    assert_eq!(w.range(30, 6), Some(1));
    assert!(w.g.sea_range_cache_stale().is_empty(), "the cache equals a fresh field");
}

#[test]
fn open_borders_lend_ports_war_takes_them() {
    let mut w = world();
    let (t0, t1) = (w.t0(), w.g.players[w.p1].team);
    w.g.player_init_city(w.p1, 30, 2, true, true);
    assert_eq!(w.range(30, 1), Some(25), "from the own port only");
    w.g.team_set_open_borders(t0, t1, true);
    w.g.team_set_open_borders(t1, t0, true);
    assert_eq!(w.range(30, 1), Some(1), "a port of a team with open borders");
    w.war();
    assert_eq!(w.range(30, 1), Some(25), "not while at war");
}

// ---- Attrition (3.1) ----

#[test]
fn out_of_range_a_ship_loses_hp_and_does_not_heal() {
    let mut w = world();
    let galley = w.unit(w.p0, "UNIT_GALLEY", 20, 1);
    let near = w.unit(w.p0, "UNIT_GALLEY", 8, 1);
    assert!(w.g.unit_is_out_of_sea_range(galley));
    assert!(!w.g.unit_is_out_of_sea_range(near));
    w.g.player_update_sea(w.p0);
    assert_eq!(w.g.units[galley].damage, 15);
    assert_eq!(w.g.units[near].damage, 0);
    assert_eq!(w.logged("out of range at sea"), 1);
    w.g.player_update_sea(w.p0);
    assert_eq!(w.g.units[galley].damage, 30);
    assert_eq!(w.logged("out of range at sea"), 1, "a message on the first turn only");
    assert_eq!(w.g.unit_heal_rate(galley, w.at(20, 1)), 0, "no healing out of range");
    w.g.units[near].damage = 30;
    assert!(w.g.unit_heal_rate(near, w.at(8, 1)) > 0, "in range it heals");
    // It kills.
    w.g.units[galley].damage = 90;
    w.g.player_update_sea(w.p0);
    assert!(!w.g.units.contains(galley) || w.g.unit_is_dead(galley));
    assert_eq!(w.logged("lost at sea"), 1);
}

#[test]
fn attrition_spares_cargo_animals_and_teams_without_cities() {
    let mut w = world();
    let galleon = w.unit(w.p0, "UNIT_GALLEON", 20, 1);
    let warrior = w.unit(w.p0, "UNIT_WARRIOR", 20, 2);
    w.g.unit_move(warrior, w.at(20, 1));
    assert!(w.g.unit_is_cargo(warrior), "loaded onto the galleon");
    let serpent = w.unit(w.p0, "UNIT_SEA_SERPENT", 22, 1);
    let theirs = w.unit(w.p1, "UNIT_GALLEY", 24, 1);
    w.g.player_update_sea(w.p0);
    w.g.player_update_sea(w.p1);
    assert_eq!(w.g.units[galleon].damage, 15, "the ship pays");
    assert_eq!(w.g.units[warrior].damage, 0, "its cargo does not");
    assert_eq!(w.g.units[serpent].damage, 0, "an animal is exempt");
    assert_eq!(w.g.units[theirs].damage, 0, "a team without a city traces no range");
}

// ---- Boats (3.2) ----

#[test]
fn a_land_unit_enters_water_in_boats_one_plot_a_turn() {
    let mut w = world();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    assert!(w.g.unit_can_move_into(u, w.at(10, 1), false, false, false));
    w.g.unit_move(u, w.at(10, 1));
    assert!(w.g.unit_is_in_boats(u));
    assert_eq!(w.g.unit_moves_left(u), 0, "entering water ends its moves");
    w.rest(u);
    w.g.unit_move(u, w.at(11, 0));
    assert_eq!(w.g.unit_moves_left(u), 0, "one water plot a turn");
    assert_eq!(w.g.unit_visibility_range(u), 1, "sight 1");
    // Landing ends its moves.
    let scout = w.unit(w.p0, "UNIT_SCOUT", 12, 2);
    w.g.unit_move(scout, w.at(12, 1));
    w.rest(scout);
    w.g.unit_move(scout, w.at(13, 2));
    assert!(!w.g.unit_is_in_boats(scout));
    assert_eq!(w.g.unit_moves_left(scout), 0, "landing ends its moves");
}

#[test]
fn the_lanun_cross_two_water_plots_a_turn() {
    let mut w = world_as(Some("CIVILIZATION_LANUN"));
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    assert_eq!(w.g.unit_embarked_moves(u), 2);
    w.g.unit_move(u, w.at(10, 1));
    assert!(w.g.unit_moves_left(u) > 0, "half its turn");
    w.g.unit_move(u, w.at(11, 1));
    assert_eq!(w.g.unit_moves_left(u), 0);
    assert!(w.g.unit_is_in_boats(u));
}

#[test]
fn in_boats_strength_1_without_modifiers() {
    let mut w = world();
    w.war();
    let u = w.unit(w.p0, "UNIT_AXEMAN", 10, 2);
    let combat1 = w.g.d.t::<PromotionType>("PROMOTION_COMBAT1");
    w.g.unit_set_has_promotion(u, combat1, true);
    let enemy = w.unit(w.p1, "UNIT_FRIGATE", 11, 0);
    let on_land = w.g.unit_max_combat_str(u, Some(w.at(10, 2)), Some(enemy), None);
    assert!(on_land > 100);
    w.g.unit_move(u, w.at(10, 1));
    assert_eq!(w.g.unit_max_combat_str(u, Some(w.at(10, 1)), Some(enemy), None), 100);
    assert_eq!(w.g.unit_first_strikes(u), 0);
    assert!(w.g.unit_can_defend(u, None));
}

#[test]
fn in_boats_no_attack_and_no_attack_onto_water() {
    let mut w = world();
    w.war();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    w.rest(u);
    let theirs = w.unit(w.p1, "UNIT_WARRIOR", 11, 2);
    assert!(!w.g.unit_can_move_into(u, w.at(11, 2), true, false, false), "no attack from boats");
    assert!(!w.g.unit_can_move_into(u, w.at(11, 2), false, false, false), "no plot holding an enemy");
    assert!(!w.g.unit_can_move_into(theirs, w.at(10, 1), true, false, false), "no attack onto water");
    assert!(!w.g.unit_can_move_into(theirs, w.at(12, 1), true, false, false));
    // Nor an enemy city.
    w.g.player_init_city(w.p1, 12, 2, true, true);
    let v = w.unit(w.p0, "UNIT_WARRIOR", 11, 1);
    assert!(w.g.unit_is_in_boats(v));
    assert!(!w.g.unit_can_move_into(v, w.at(12, 2), false, false, false), "no enemy city from boats");
    assert!(w.g.unit_can_move_into(v, w.at(10, 2), false, false, false), "an empty plot: it lands");
}

#[test]
fn in_boats_no_heal_no_fortify_no_cast_no_found() {
    let mut w = world();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    w.g.units[u].damage = 40;
    let plot = w.at(10, 1);
    assert_eq!(w.g.unit_heal_rate(u, plot), 0);
    assert!(!w.g.unit_can_fortify(u, plot));
    let settler = w.unit(w.p0, "UNIT_SETTLER", 12, 1);
    assert!(w.g.unit_is_in_boats(settler));
    assert!(!w.g.unit_can_found(settler, w.at(12, 1), false));
    w.g.unit_do_turn(u);
    assert_eq!(w.g.units[u].damage, 40, "no healing in boats");
    assert_eq!(w.g.units[u].fortify_turns, 0);
}

#[test]
fn a_unit_steps_aboard_a_ship_with_room() {
    let mut w = world();
    let galleon = w.unit(w.p0, "UNIT_GALLEON", 10, 1);
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    assert!(w.g.unit_is_cargo(u));
    assert_eq!(w.g.units[u].transport_unit, Some(galleon));
    assert!(!w.g.unit_is_in_boats(u));
}

#[test]
fn a_ship_sinks_a_unit_in_boats_by_the_duel() {
    let mut w = world();
    w.war();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    let ship = w.unit(w.p1, "UNIT_MAN_O_WAR", 11, 0);
    assert!(w.g.unit_can_move_into(ship, w.at(10, 1), true, false, false));
    assert!(!w.g.engage_applies(ship, w.at(10, 1)), "the duel");
    assert!(w.g.get_combat_odds(ship, u) >= 990);
    let reports = w.g.battle_reports.len();
    w.g.unit_attack(ship, w.at(10, 1));
    assert!(!w.g.units.contains(u) || w.g.unit_is_dead(u), "sunk");
    assert_eq!(w.g.battle_reports.len(), reports, "no engagement");
}

#[test]
fn a_warship_on_the_plot_defends_first() {
    let mut w = world();
    w.war();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    w.g.unit_move(u, w.at(10, 1));
    let escort = w.unit(w.p0, "UNIT_GALLEY", 10, 1);
    assert!(w.g.unit_is_in_boats(u), "the galley has no room: still in boats");
    let ship = w.unit(w.p1, "UNIT_FRIGATE", 11, 0);
    let def = w.g.plot_get_best_defender(w.at(10, 1), None, Some(w.p1), Some(ship), false, false, false);
    assert_eq!(def, Some(escort));
}

// ---- Supply at sea (3.3) ----

#[test]
fn an_army_at_sea_is_supplied_in_range() {
    let mut w = world();
    let units: Vec<UnitId> = (0..2).map(|_| w.unit(w.p0, "UNIT_WARRIOR", 10, 2)).collect();
    let a = w.g.army_form(w.p0, &units).expect("an army");
    for &u in &units {
        w.g.unit_move(u, w.at(10, 1));
    }
    let line = w.g.army_supply_line(a).expect("supplied at sea");
    assert_eq!(line.source, SupplySource::Sea(w.port));
    assert_eq!(line.path, vec![w.at(10, 1)]);
    let known = w.g.plot_supply_line_known(w.p0, w.at(10, 1), w.t0()).expect("known");
    assert_eq!(known.source, SupplySource::Sea(w.port));
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(20, 1)), None, "out of range: cut");
}

// ---- Wear (3.4) ----

#[test]
fn wear_lowers_strength_and_a_port_clears_it() {
    let mut w = world();
    let galley = w.unit(w.p0, "UNIT_GALLEY", 8, 1);
    let fresh = w.g.unit_max_combat_str(galley, None, None, None);
    for _ in 0..10 {
        w.g.player_update_sea(w.p0);
    }
    assert_eq!(w.g.units[galley].sea_turns, 10);
    assert_eq!(w.g.unit_sea_wear_percent(galley), 0);
    assert_eq!(w.logged("wear"), 0);
    w.g.player_update_sea(w.p0);
    assert_eq!(w.g.unit_sea_wear_percent(galley), 5);
    assert_eq!(w.logged("wear"), 1);
    assert!(w.g.unit_max_combat_str(galley, None, None, None) < fresh);
    w.g.units[galley].sea_turns = 40;
    assert_eq!(w.g.unit_sea_wear_percent(galley), 50, "at most");
    // A turn start in port.
    w.g.unit_set_xy(galley, 5, 2, false);
    w.g.player_update_sea(w.p0);
    assert_eq!(w.g.units[galley].sea_turns, 0);
    assert_eq!(w.g.unit_max_combat_str(galley, None, None, None), fresh);
}

// ---- Saves ----

#[test]
fn the_new_fields_survive_a_save() {
    let mut w = world();
    let galley = w.unit(w.p0, "UNIT_GALLEY", 20, 1);
    w.g.units[galley].sea_turns = 7;
    w.g.player_update_sea(w.p0);
    assert!(w.g.units[galley].sea_out_of_range);
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.units[galley].sea_turns, 8);
    assert!(h.units[galley].sea_out_of_range);
    assert_eq!(h.plot_sea_range(h.players[w.p0].team, w.at(13, 1)), Some(8), "the field is computed again");
    assert_eq!(h.save(), bytes);
}

// ---- Paths (docs/NAVAL.md, sections 3.2 and 8) ----

#[test]
fn a_human_path_crosses_water_an_ai_path_only_with_boats() {
    use ffh_core::defs::MOVE_BOATS;
    let mut w = world();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 10, 2);
    let grp = w.g.units[u].group.unwrap();
    let (from, sea, far) = (w.at(10, 2), w.at(11, 1), w.at(14, 1));
    // The AI's path finder keeps an AI land group off water, unless the search allows boats (in
    // range only).
    assert!(w.g.group_find_path(grp, from, sea, 0).is_none());
    let path = w.g.group_find_path(grp, from, sea, MOVE_BOATS).expect("in boats");
    assert_eq!(path.turns, 1);
    assert!(w.g.group_find_path(grp, from, far, MOVE_BOATS).is_none(), "out of range");
    // A human's path crosses water with no flag, one water plot a turn: the preview's turns.
    w.g.players[w.p0].human = true;
    let path = w.g.group_find_path(grp, from, w.at(13, 1), 0).expect("a human's path");
    assert_eq!(path.turns, 3, "(11|12|13, 1): one a turn");
    let path = w.g.group_find_path(grp, from, w.at(14, 2), 0).expect("and lands");
    assert_eq!(path.turns, 4);
}

#[test]
fn an_ai_ship_stays_in_range_unless_already_out() {
    let mut w = world();
    let galley = w.unit(w.p0, "UNIT_GALLEY", 8, 1);
    let grp = w.g.units[galley].group.unwrap();
    assert!(w.g.group_find_path(grp, w.at(8, 1), w.at(13, 1), 0).is_some());
    assert!(w.g.group_find_path(grp, w.at(8, 1), w.at(16, 1), 0).is_none(), "out of range");
    let lost = w.unit(w.p0, "UNIT_GALLEY", 20, 1);
    let lg = w.g.units[lost].group.unwrap();
    assert!(w.g.group_find_path(lg, w.at(20, 1), w.at(6, 1), 0).is_some(), "back into range");
}
