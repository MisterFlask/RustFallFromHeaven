//! Governors: direct rule, governed cities, the bench (docs/GOVERNORS.md).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A small map with governors (and armies) on, after `game_init`, the starting units gone, and
/// player 0 holding `n` cities at least 3 plots apart (the first is its capital).
fn game(n: usize) -> (Game, Vec<CityId>) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.options[GameOptionType::GAMEOPTION_GOVERNORS] = true;
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    let p0 = PlayerId(0);
    for u in g.players[p0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let mut cities = Vec::new();
    let land: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].owner.is_none()).collect();
    for q in land {
        if cities.len() == n {
            break;
        }
        let (x, y) = g.map.xy(q);
        let far = cities.iter().all(|&c: &CityId| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) >= 4);
        if far && g.map.plots[q].owner.is_none() && g.map.plots[q].plot_city.is_none() {
            cities.push(g.player_init_city(p0, x, y, true, true));
        }
    }
    assert_eq!(cities.len(), n, "room for {n} cities");
    (g, cities)
}

#[test]
fn the_first_cities_are_ruled_directly_and_the_rest_governed() {
    let (g, cities) = game(6);
    let slots = g.governor_direct_slots();
    assert!(g.city_is_capital(cities[0]));
    for (i, &c) in cities.iter().enumerate() {
        assert_eq!(g.city_is_governed(c), i >= slots, "city {i}");
    }
    let governed = cities[slots];
    assert!(g.cities[governed].production_automated, "the governor chooses its production");
    assert_eq!(g.cities[governed].governor, None, "no candidate yet: an interim governor");
    assert!(g.city_governor_profile(governed).is_some_and(|p| g.d.governor_profiles[p].interim));
    assert!(g.log.entries.iter().any(|m| m.text.contains("interim governor")), "told");
}

#[test]
fn governed_cities_build_no_settlers_wonders_or_rituals() {
    let (g, cities) = game(5);
    let (direct, governed) = (cities[1], cities[4]);
    let settler = g.d.t::<UnitType>("UNIT_SETTLER");
    assert!(!g.city_can_train(governed, settler, false, true, true, true));
    assert!(g.city_can_train(direct, settler, false, true, true, true) || !g.city_can_train(direct, settler, false, true, true, false));
    let wonder = g.d.buildings.keys().find(|&b| g.d.buildings[b].building_class_type.is_some_and(|bc| ffh_core::utils::is_world_wonder_class(g.d, bc))).expect("a world wonder");
    assert!(!g.city_can_construct(governed, wonder, false, true, true));
    let ritual = g.d.projects.keys().next().expect("a project");
    assert!(!g.city_can_create(governed, ritual, false, true));
}

#[test]
fn a_candidate_takes_the_vacant_post_and_a_replacement_costs_both_cities() {
    let (mut g, cities) = game(6);
    let p0 = PlayerId(0);
    let (a, b) = (cities[4], cities[5]);
    let first = g.governor_generate(p0);
    let second = g.governor_generate(p0);
    // (no traits: only the penalty moves the modifiers)
    g.governors[first].traits.clear();
    g.governors[second].traits.clear();
    assert_eq!(g.player_bench(p0), vec![first, second]);
    let base = g.city_base_yield_rate_modifier(a, YieldType::YIELD_PRODUCTION, 0);
    g.city_assign_governor(p0, a, first).unwrap();
    assert_eq!(g.cities[a].governor, Some(first));
    assert_eq!(g.governors[first].city, Some(a));
    assert_eq!(g.cities[a].governor_penalty_turns, 0, "filling a vacant post costs nothing");
    // The second replaces the first: the first goes back to the bench, the city pays.
    g.city_assign_governor(p0, a, second).unwrap();
    assert_eq!(g.player_bench(p0), vec![first]);
    assert!(g.cities[a].governor_penalty_turns > 0);
    let penalty = g.d.defines.governor_swap_penalty;
    assert_eq!(g.city_base_yield_rate_modifier(a, YieldType::YIELD_PRODUCTION, 0), base - penalty);
    // Moving the second to b: both cities pay (the spec), a is interim again.
    g.city_assign_governor(p0, b, second).unwrap();
    assert_eq!(g.cities[a].governor, None);
    assert!(g.cities[b].governor_penalty_turns > 0);
    assert!(g.cities[a].governor_penalty_turns > 0);
    // The penalty wears off.
    for _ in 0..g.d.defines.governor_swap_turns {
        g.player_governance_turn(p0);
    }
    assert_eq!(g.cities[a].governor_penalty_turns, 0);
}

#[test]
fn direct_rule_swaps_respect_the_cooldown_and_the_capital() {
    let (mut g, cities) = game(6);
    let p0 = PlayerId(0);
    let (capital, direct, governed) = (cities[0], cities[1], cities[4]);
    assert!(g.city_hand_to_governor(p0, capital).is_err(), "the capital is always ruled directly");
    assert!(g.player_set_direct_rule(p0, governed, None).is_err(), "no free slot");
    g.player_set_direct_rule(p0, governed, Some(direct)).expect("swapped");
    assert!(!g.city_is_governed(governed));
    assert!(g.city_is_governed(direct));
    // Swapping back at once: refused (the cooldown).
    assert!(g.player_set_direct_rule(p0, direct, Some(governed)).is_err());
    assert!(!g.city_rule_change_ready(governed));
    // A free slot: a governed city may take it without a swap.
    let other = cities[2];
    g.city_hand_to_governor(p0, other).expect("handed over");
    assert!(g.player_set_direct_rule(p0, cities[5], None).is_ok());
}

#[test]
fn governor_traits_change_their_city() {
    let (mut g, cities) = game(5);
    let p0 = PlayerId(0);
    let c = cities[4];
    let g1 = g.governor_generate(p0);
    let frugal = g.d.t::<GovernorTraitType>("GOVERNORTRAIT_FRUGAL");
    g.governors[g1].traits = vec![frugal];
    let before = g.city_total_commerce_rate_modifier(c, CommerceType::COMMERCE_GOLD);
    g.city_assign_governor(p0, c, g1).unwrap();
    assert_eq!(g.city_total_commerce_rate_modifier(c, CommerceType::COMMERCE_GOLD), before + 10);
}

#[test]
fn the_bench_fills_the_vacant_posts() {
    let (mut g, cities) = game(6);
    let p0 = PlayerId(0);
    let mut turns = 0;
    while !g.player_vacant_posts(p0).is_empty() {
        turns += 1;
        assert!(turns < 300, "the bench never filled the posts");
        g.player_governance_turn(p0);
    }
    for &c in &cities[4..] {
        let gv = g.cities[c].governor.expect("a governor");
        assert_eq!(g.governors[gv].owner, p0);
        assert!(!g.governors[gv].name.is_empty());
    }
}

#[test]
fn a_lost_city_takes_its_governor_out_of_the_game() {
    let (mut g, cities) = game(5);
    let p0 = PlayerId(0);
    let c = cities[4];
    let gv = g.governor_generate(p0);
    g.city_assign_governor(p0, c, gv).unwrap();
    g.player_acquire_city(PlayerId(1), c, false, true, true);
    assert!(!g.governors.contains(gv));
    assert!(!g.players[p0].governors.contains(&gv));
}

#[test]
fn governors_survive_a_save() {
    let (mut g, cities) = game(6);
    let p0 = PlayerId(0);
    let gv = g.governor_generate(p0);
    g.city_assign_governor(p0, cities[4], gv).unwrap();
    g.city_set_hostile_culture(p0, cities[4], true).unwrap();
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.governors[gv], g.governors[gv]);
    assert_eq!(h.cities[cities[4]].governor, Some(gv));
    assert!(h.cities[cities[4]].governor_hostile_culture);
    assert_eq!(h.save(), bytes);
}

#[test]
fn off_without_the_option() {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let p0 = PlayerId(0);
    let u = g.players[p0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let c = g.player_init_city(p0, x, y, true, true);
    assert!(!g.city_is_governed(c));
    g.player_governance_turn(p0);
    assert!(g.governors.iter().next().is_none());
}

#[test]
fn a_city_handed_to_a_governor_drops_the_orders_it_may_no_longer_build() {
    let (mut g, cities) = game(4);
    let p0 = PlayerId(0);
    let c = cities[3];
    let settler = g.d.t::<UnitType>("UNIT_SETTLER");
    g.cities[c].order_queue.clear();
    g.city_push_order(c, OrderType::ORDER_TRAIN, settler.idx() as i32, -1, false, false, true, true);
    assert!(g.cities[c].order_queue.iter().any(|o| o.data1 == settler.idx() as i32));
    g.city_hand_to_governor(p0, c).unwrap();
    assert!(g.city_is_governed(c));
    assert!(!g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_TRAIN && o.data1 == settler.idx() as i32), "no settler in a governed city");
}
