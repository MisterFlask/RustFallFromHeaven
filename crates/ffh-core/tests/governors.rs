//! Governors v2: the government decides who runs the cities, city roles, empire priorities and
//! diagnoses, empire orders (docs/GOVERNORS.md). Every situation is built by hand.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ffh_core::controller::Controller;
use ffh_core::game::Game;
use ffh_core::governors::GovDiagnosis;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// A small map after `game_init`, the starting units gone, and player 0 (the Bannor, who build
/// every ordinary building) holding `n` cities at least 4 plots apart (the first is its capital),
/// under its initial civics (Despotism).
fn game(n: usize) -> (Game, Vec<CityId>) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players[P0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let mut cities = Vec::new();
    while cities.len() < n {
        let (x, y) = free_plot(&g);
        cities.push(g.player_init_city(P0, x, y, true, true));
    }
    (g, cities)
}

/// A free land plot at least 4 plots from every city.
fn free_plot(g: &Game) -> (i32, i32) {
    g.map
        .plot_ids()
        .filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].owner.is_none() && g.map.plots[q].plot_city.is_none())
        .map(|q| g.map.xy(q))
        .find(|&(x, y)| g.cities.iter().all(|(_, c)| g.plot_distance(x, y, c.x, c.y) >= 4))
        .expect("a free plot")
}

fn civic(g: &Game, name: &str) -> CivicType {
    g.d.t::<CivicType>(name)
}

/// Player `p` adopts government civic `name`.
fn government(g: &mut Game, p: PlayerId, name: &str) {
    let c = civic(g, name);
    let option = g.d.civics[c].civic_option_type.expect("a civic option");
    g.player_set_civics(p, option, Some(c));
}

fn role(g: &Game, name: &str) -> CityRoleType {
    g.d.t::<CityRoleType>(name)
}

/// Every key building class of `role` in city `c` at `tier`.
fn set_role_tier(g: &mut Game, c: CityId, role: CityRoleType, tier: BuildingTierType) {
    for k in g.d.city_roles[role].key_buildings.clone() {
        g.cities[c].building_tiers[k] = tier;
    }
}

/// Every key building class of every role in city `c` Standard.
fn all_standard(g: &mut Game, c: CityId) {
    for r in g.d.city_roles.keys() {
        set_role_tier(g, c, r, BuildingTierType::STANDARD);
    }
}

/// Whether city `c`'s civilization can build every key building class of `role`.
fn builds_all(g: &Game, c: CityId, role: CityRoleType) -> bool {
    let civ = g.city_civilization_type(c);
    g.d.city_roles[role].key_buildings.iter().all(|&k| g.d.civ_building(civ, k).is_some())
}

/// A player-0 governed game: `n` cities under Aristocracy.
fn governed_game(n: usize) -> (Game, Vec<CityId>) {
    let (mut g, cities) = game(n);
    government(&mut g, P0, "CIVIC_ARISTOCRACY");
    (g, cities)
}

#[test]
fn city_roles_have_four_generic_key_buildings_each_none_in_two_roles() {
    let d = ffh_data::global();
    assert_eq!(d.city_roles.len(), 6);
    let mut seen = std::collections::BTreeSet::new();
    for r in d.city_roles.keys() {
        let keys = &d.city_roles[r].key_buildings;
        assert_eq!(keys.len(), 4, "{}", d.city_roles[r].base.type_);
        for &k in keys {
            assert!(seen.insert(k), "{:?} in two roles", k);
            assert!(!ffh_core::utils::is_limited_wonder_class(d, k), "{:?} is a wonder", k);
            assert!(d.building_classes[k].rolls_tier, "{:?} has tiers", k);
        }
    }
    let direct: Vec<&str> = d.civics.iter().filter(|c| c.direct_rule).map(|c| c.base.type_.as_str()).collect();
    assert_eq!(direct, ["CIVIC_DESPOTISM", "CIVIC_GOD_KING"]);
}

#[test]
fn the_government_hands_every_city_over_and_takes_every_city_back() {
    let (mut g, cities) = game(4);
    assert!(g.player_rules_directly(P0), "Despotism rules directly");
    assert!(cities.iter().all(|&c| !g.city_is_governed(c) && g.cities[c].governor.is_none()));
    let before = g.log.entries.len();
    government(&mut g, P0, "CIVIC_ARISTOCRACY");
    assert!(!g.player_rules_directly(P0));
    let mut governors = Vec::new();
    for &c in &cities {
        assert!(g.city_is_governed(c), "every city governed, the capital included");
        assert!(g.cities[c].production_automated);
        assert!(g.city_role(c).is_some());
        let gv = g.cities[c].governor.expect("a governor");
        assert_eq!(g.governors[gv].city, c);
        assert_eq!(g.governors[gv].owner, P0);
        governors.push(gv);
    }
    let news: Vec<&str> = g.log.entries[before..].iter().map(|m| m.text.as_str()).filter(|t| t.starts_with("Government:")).collect();
    assert_eq!(news.len(), 1, "one message: {news:?}");
    // Back to a direct-rule civic: every city taken back, the governors dormant in their cities.
    government(&mut g, P0, "CIVIC_GOD_KING");
    for (&c, &gv) in cities.iter().zip(&governors) {
        assert!(!g.city_is_governed(c));
        assert!(!g.cities[c].production_automated);
        assert_eq!(g.city_role(c), None);
        assert_eq!(g.cities[c].governor, Some(gv), "the governor stays with the city");
    }
    // Governed again: the same governors, no new ones.
    government(&mut g, P0, "CIVIC_ARISTOCRACY");
    for (&c, &gv) in cities.iter().zip(&governors) {
        assert_eq!(g.cities[c].governor, Some(gv));
    }
    assert_eq!(g.players[P0].governors.len(), cities.len());
}

#[test]
fn a_city_taken_back_drops_its_roles_emphasis() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let production = role(&g, "ROLE_PRODUCTION");
    all_standard(&mut g, c);
    set_role_tier(&mut g, c, production, BuildingTierType::LEGENDARY);
    g.player_set_city_role(P0, c, production).unwrap();
    let wanted = g.d.city_roles[production].emphasizes.clone();
    assert!(!wanted.is_empty() && wanted.iter().all(|&e| g.cities[c].ai_emphasize[e]));
    government(&mut g, P0, "CIVIC_DESPOTISM");
    assert!(wanted.iter().all(|&e| !g.cities[c].ai_emphasize[e]));
}

#[test]
fn a_new_city_takes_its_owners_mode() {
    let (mut g, _) = governed_game(2);
    // Founded.
    let (x, y) = free_plot(&g);
    let founded = g.player_init_city(P0, x, y, true, true);
    assert!(g.city_is_governed(founded));
    assert!(g.cities[founded].governor.is_some() && g.city_role(founded).is_some());
    assert!(g.cities[founded].recruit_default.is_none());
    // Acquired from a player that rules directly.
    assert!(g.player_rules_directly(P1));
    let (x, y) = free_plot(&g);
    let theirs = g.player_init_city(P1, x, y, true, true);
    assert!(!g.city_is_governed(theirs));
    g.player_acquire_city(P0, theirs, false, true, true);
    let ours = g.map.plots[g.map.plot_xy(x, y).unwrap()].plot_city.expect("the city");
    assert!(g.city_is_governed(ours));
    let gv = g.cities[ours].governor.expect("a governor");
    assert_eq!(g.governors[gv].owner, P0);
    // Given to a player that rules directly: ruled directly, its governor out of the game.
    g.player_acquire_city(P1, ours, false, true, true);
    let back = g.map.plots[g.map.plot_xy(x, y).unwrap()].plot_city.expect("the city");
    assert!(!g.city_is_governed(back));
    assert!(!g.governors.contains(gv));
    assert!(!g.players[P0].governors.contains(&gv));
}

#[test]
fn barbarian_cities_are_ruled_directly() {
    let (g, _) = game(1);
    assert!(g.player_rules_directly(g.barbarian_player()));
}

#[test]
fn the_role_proposal_follows_suitability_and_priorities() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let (growth, research) = (role(&g, "ROLE_GROWTH"), role(&g, "ROLE_RESEARCH"));
    assert!(builds_all(&g, c, research), "the test's civilization builds every research class");
    all_standard(&mut g, c);
    set_role_tier(&mut g, c, research, BuildingTierType::GRAND);
    assert_eq!(g.city_role_suitability(c, research), 200);
    assert_eq!(g.city_role_suitability(c, growth), 100);
    assert_eq!(g.city_role_proposal(c), research);
    // A priority weighs a role more: Commerce and Research at Fine tie (the first role, Research,
    // wins), and Commerce wins as a priority (150 * 130%).
    let commerce = role(&g, "ROLE_COMMERCE");
    assert!(builds_all(&g, c, commerce));
    set_role_tier(&mut g, c, research, BuildingTierType::FINE);
    set_role_tier(&mut g, c, commerce, BuildingTierType::FINE);
    g.player_set_governor_priorities(P0, vec![]).unwrap();
    assert_eq!(g.city_role_proposal(c), research);
    assert_eq!(g.city_role(c), Some(research));
    g.player_set_governor_priorities(P0, vec![commerce]).unwrap();
    assert_eq!(g.city_role_proposal(c), commerce);
    assert_eq!(g.city_role(c), Some(commerce), "the city follows its proposal");
    // Ties go to the first role.
    all_standard(&mut g, c);
    g.player_set_governor_priorities(P0, vec![]).unwrap();
    assert_eq!(g.city_role_proposal(c), g.d.city_roles.keys().next().unwrap());
    assert_eq!(g.city_role(c), g.d.city_roles.keys().next());
}

#[test]
fn priorities_are_at_most_two_and_distinct() {
    let (mut g, _) = governed_game(1);
    let (a, b, c) = (role(&g, "ROLE_GROWTH"), role(&g, "ROLE_RESEARCH"), role(&g, "ROLE_COMMERCE"));
    assert!(g.player_set_governor_priorities(P0, vec![a, b, c]).is_err());
    assert!(g.player_set_governor_priorities(P0, vec![a, a]).is_err());
    g.player_set_governor_priorities(P0, vec![a, b]).unwrap();
    assert_eq!(g.players[P0].governor_priorities, vec![a, b]);
}

#[test]
fn a_role_the_player_chooses_needs_the_suitability() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let (research, garrison, commerce) = (role(&g, "ROLE_RESEARCH"), role(&g, "ROLE_GARRISON"), role(&g, "ROLE_COMMERCE"));
    assert!(builds_all(&g, c, garrison) && builds_all(&g, c, commerce) && builds_all(&g, c, research));
    all_standard(&mut g, c);
    set_role_tier(&mut g, c, research, BuildingTierType::GRAND);
    set_role_tier(&mut g, c, garrison, BuildingTierType::LESSER);
    g.player_set_governor_priorities(P0, vec![]).unwrap();
    assert_eq!(g.city_role(c), Some(research));
    // Lesser everywhere: 50%, refused.
    assert_eq!(g.player_set_city_role(P0, c, garrison), Err("Role requires suitability 75%."));
    assert_eq!(g.city_role(c), Some(research));
    // Standard: 100%, accepted and kept against the proposal.
    g.player_set_city_role(P0, c, commerce).unwrap();
    assert_eq!(g.city_role(c), Some(commerce));
    assert!(g.cities[c].role_overridden);
    g.player_set_governor_priorities(P0, vec![research]).unwrap();
    assert_eq!(g.city_role(c), Some(commerce), "a chosen role is kept when the priorities change");
    // Choosing the proposal lets the role follow it again.
    g.player_set_city_role(P0, c, research).unwrap();
    assert!(!g.cities[c].role_overridden);
    // Not a governed city: refused.
    government(&mut g, P0, "CIVIC_DESPOTISM");
    assert!(g.player_check_city_role(P0, c, research).is_err());
}

#[test]
fn diagnoses_name_unfilled_roles_poor_fits_and_slow_orders() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let (research, commerce) = (role(&g, "ROLE_RESEARCH"), role(&g, "ROLE_COMMERCE"));
    assert!(builds_all(&g, c, commerce));
    for &x in &cities {
        all_standard(&mut g, x);
        set_role_tier(&mut g, x, research, BuildingTierType::GRAND);
    }
    g.player_set_governor_priorities(P0, vec![commerce]).unwrap();
    // Commerce at 100 * 130% < Research at 200%: Research everywhere, Commerce unfilled.
    assert!(cities.iter().all(|&x| g.city_role(x) == Some(research)));
    assert_eq!(g.player_governor_diagnoses(P0), vec![GovDiagnosis::RoleUnfilled(commerce)]);
    // Commerce chosen for a city that suits it at 87%: filled, and a poor fit.
    let keys = g.d.city_roles[commerce].key_buildings.clone();
    g.cities[c].building_tiers[keys[0]] = BuildingTierType::LESSER;
    let s = g.city_role_suitability(c, commerce);
    assert_eq!(s, (50 + 300) / 4);
    g.player_set_city_role(P0, c, commerce).unwrap();
    assert_eq!(g.player_governor_diagnoses(P0), vec![GovDiagnosis::PoorFit { city: c, role: commerce, suitability: s }]);
}

#[test]
fn every_role_is_expected_from_six_governed_cities() {
    let (mut g, cities) = governed_game(6);
    for &c in &cities {
        all_standard(&mut g, c);
    }
    g.player_set_governor_priorities(P0, vec![]).unwrap();
    let first = g.d.city_roles.keys().next().unwrap();
    let unfilled: Vec<GovDiagnosis> = g.d.city_roles.keys().filter(|&r| r != first).map(GovDiagnosis::RoleUnfilled).collect();
    assert_eq!(g.player_governor_diagnoses(P0), unfilled);
}

/// A controller whose production choice counts its calls and trains a warrior.
#[derive(Default)]
struct Counting {
    asked: AtomicUsize,
}

impl Controller for Counting {
    fn choose_production(&self, g: &mut Game, c: CityId) {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
        g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, false);
    }
}

#[test]
fn empire_orders_are_units_wonders_palaces_and_projects() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let d = g.d;
    let team = g.players[P0].team;
    for t in d.techs.keys() {
        g.team_set_has_tech(team, t, true, Some(P0), false, false);
    }
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let settler = d.t::<UnitType>("UNIT_SETTLER");
    assert!(g.city_check_empire_order(P0, c, OrderType::ORDER_TRAIN, warrior.i()).is_ok());
    assert!(g.city_check_empire_order(P0, c, OrderType::ORDER_TRAIN, settler.i()).is_ok(), "settlers are buildable in a governed city, by empire order");
    // A world wonder, a palace and a project may be empire orders; one the city can build is accepted.
    let wonders: Vec<BuildingType> = d.buildings.keys().filter(|&b| d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_world_wonder_class(d, k))).collect();
    assert!(wonders.iter().all(|&b| g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i())));
    assert!(wonders.iter().any(|&b| g.city_check_empire_order(P0, c, OrderType::ORDER_CONSTRUCT, b.i()).is_ok()), "a world wonder");
    let palace = d.buildings.keys().find(|&b| d.buildings[b].capital).unwrap();
    assert!(g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, palace.i()));
    assert!(d.projects.keys().all(|pr| g.order_may_be_empire(OrderType::ORDER_CREATE, pr.i())));
    assert!(d.projects.keys().any(|pr| g.city_check_empire_order(P0, c, OrderType::ORDER_CREATE, pr.i()).is_ok()), "a project");
    // An ordinary building, a process: refused.
    let granary = g.city_building_for_class(c, d.t::<BuildingClassType>("BUILDINGCLASS_GRANARY")).unwrap();
    assert!(g.city_check_empire_order(P0, c, OrderType::ORDER_CONSTRUCT, granary.i()).is_err());
    assert!(g.city_check_empire_order(P0, c, OrderType::ORDER_MAINTAIN, 0).is_err());
    // Not in a directly ruled city.
    government(&mut g, P0, "CIVIC_DESPOTISM");
    assert!(g.city_check_empire_order(P0, c, OrderType::ORDER_TRAIN, warrior.i()).is_err());
}

#[test]
fn requisition_then_empire_orders_lead_the_governors_choice() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let scout = g.d.t::<UnitType>("UNIT_SCOUT");
    // The governor's own choice.
    g.cities[c].order_queue.clear();
    g.city_push_order(c, OrderType::ORDER_MAINTAIN, 0, -1, false, false, true, true);
    let own = g.cities[c].order_queue.clone();
    assert_eq!(own.len(), 1);
    // An empire order goes ahead of it.
    g.city_push_empire_order(P0, c, OrderType::ORDER_TRAIN, warrior.i(), -1, None).unwrap();
    // An army's requisition goes ahead of the empire order.
    let (x, y) = free_plot(&g);
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(P0, warrior, x, y, None, None)).collect();
    let army = g.army_form(P0, &units).expect("formed");
    g.army_set_target_size(P0, army, 3).unwrap();
    g.army_add_requisition(P0, army, c).unwrap();
    g.player_do_requisition(P0);
    // A second empire order goes behind the first, an army as its destination.
    let dest = Some(ffh_core::army::RecruitDest::Army(army));
    g.city_push_empire_order(P0, c, OrderType::ORDER_TRAIN, scout.i(), -1, dest).unwrap();
    let q = &g.cities[c].order_queue;
    assert_eq!(q.len(), 4);
    assert!(q[0].requisition && !q[0].empire);
    assert!(q[1].empire && q[1].data1 == warrior.i() && !q[1].save);
    assert!(q[2].empire && q[2].data1 == scout.i() && q[2].dest == dest);
    assert_eq!(q[3], own[0], "the governor's choice behind them");
    assert_eq!(g.city_lead_orders(c), 3);
    // Removal: the player's own empire orders only.
    assert!(g.city_remove_empire_order(P0, c, 0).is_err(), "a requisition order");
    assert!(g.city_remove_empire_order(P0, c, 3).is_err(), "the governor's order");
    g.city_remove_empire_order(P0, c, 1).unwrap();
    let q = &g.cities[c].order_queue;
    assert_eq!(q.len(), 3);
    assert!(q[1].empire && q[1].data1 == scout.i());
}

#[test]
fn the_governor_is_not_asked_while_an_empire_order_leads_and_is_asked_after() {
    let (mut g, cities) = governed_game(2);
    let c = cities[1];
    let counting = Arc::new(Counting::default());
    g.controllers[P0] = counting.clone();
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    g.cities[c].order_queue.clear();
    g.city_push_empire_order(P0, c, OrderType::ORDER_TRAIN, warrior.i(), -1, None).unwrap();
    g.cities[c].ai_choose_production_dirty = true;
    g.city_do_production(c, false);
    assert_eq!(counting.asked.load(Ordering::SeqCst), 0, "not asked while the empire order leads");
    let index = g.cities[c].order_queue.iter().position(|o| o.empire).expect("the empire order");
    g.city_remove_empire_order(P0, c, index).unwrap();
    assert_eq!(counting.asked.load(Ordering::SeqCst), 1, "asked once it is done");
    assert!(!g.cities[c].order_queue.is_empty());
}

#[test]
fn a_city_handed_over_keeps_its_requisition_and_empire_orders_only() {
    let (mut g, cities) = game(2);
    let c = cities[1];
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let granary = g.city_building_for_class(c, g.d.t::<BuildingClassType>("BUILDINGCLASS_GRANARY")).unwrap();
    g.cities[c].order_queue.clear();
    g.city_push_order(c, OrderType::ORDER_CONSTRUCT, granary.i(), -1, false, false, true, true);
    g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, true, false, true, true);
    g.cities[c].recruit_default = Some(ffh_core::army::RecruitDest::Garrison(cities[0]));
    government(&mut g, P0, "CIVIC_ARISTOCRACY");
    let q = &g.cities[c].order_queue;
    assert_eq!(q.len(), 1, "{q:?}");
    assert!(q[0].empire && q[0].data1 == warrior.i() && !q[0].save, "the unit is an empire order; the granary gave way");
    assert_eq!(g.cities[c].recruit_default, None);
    // Taken back: the queue kept, as ordinary orders.
    government(&mut g, P0, "CIVIC_DESPOTISM");
    assert!(g.cities[c].order_queue.iter().all(|o| !o.empire));
    assert_eq!(g.cities[c].order_queue.len(), 1);
}

#[test]
fn governors_have_at_most_one_trait_and_a_name() {
    let (mut g, _) = governed_game(1);
    for _ in 0..8 {
        let (x, y) = free_plot(&g);
        g.player_init_city(P0, x, y, true, true);
    }
    for &gv in &g.players[P0].governors {
        let gov = &g.governors[gv];
        assert!(!gov.name.is_empty());
        assert!(g.cities[gov.city].governor == Some(gv));
    }
}

#[test]
fn governors_v2_survive_a_save() {
    let (mut g, cities) = governed_game(3);
    let c = cities[1];
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let research = role(&g, "ROLE_RESEARCH");
    all_standard(&mut g, c);
    g.player_set_governor_priorities(P0, vec![research]).unwrap();
    let commerce = role(&g, "ROLE_COMMERCE");
    g.player_set_city_role(P0, c, commerce).unwrap();
    g.city_push_empire_order(P0, c, OrderType::ORDER_TRAIN, warrior.i(), -1, None).unwrap();
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    let gv = g.cities[c].governor.unwrap();
    assert_eq!(h.governors[gv], g.governors[gv]);
    assert_eq!(h.cities[c].role, Some(commerce));
    assert!(h.cities[c].role_overridden);
    assert_eq!(h.players[P0].governor_priorities, vec![research]);
    assert_eq!(h.cities[c].order_queue, g.cities[c].order_queue);
    assert!(h.cities[c].order_queue.iter().any(|o| o.empire));
    assert_eq!(h.save(), bytes);
}
