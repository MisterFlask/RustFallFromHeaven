//! Building suitability: the building tiers (docs/SUITABILITY.md).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A small map after `game_init`, the starting units gone, and player 0 (the Bannor, who build
/// every ordinary building) holding `n` cities at least 4 plots apart, its team knowing every
/// tech.
fn game(n: usize) -> (Game, Vec<CityId>) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let team = g.players[p0].team;
    for t in d.techs.keys() {
        g.team_set_has_tech(team, t, true, Some(p0), false, false);
    }
    (g, cities)
}

fn b(d: &GameData, name: &str) -> BuildingType {
    d.t::<BuildingType>(name)
}

fn class(d: &GameData, b: BuildingType) -> BuildingClassType {
    d.buildings[b].building_class_type.unwrap()
}

#[test]
fn every_city_rolls_a_tier_for_each_class_that_rolls_one() {
    let (g, cities) = game(12);
    let d = g.d;
    let mut seen = vec![0; BuildingTierType::NUM];
    for &c in &cities {
        for k in d.building_classes.keys() {
            let tier = g.cities[c].building_tiers[k];
            if d.building_classes[k].rolls_tier {
                seen[tier.idx()] += 1;
            } else {
                assert_eq!(tier, BuildingTierType::STANDARD, "{} rolls no tier", d.building_classes[k].base.type_);
            }
        }
    }
    // About 100 classes x 12 cities: every tier comes up, roughly by its weight.
    let total: usize = seen.iter().sum();
    for t in d.building_tiers.keys() {
        let share = seen[t.idx()] * 100 / total;
        let weight = d.building_tiers[t].weight as usize;
        assert!(share + 5 >= weight && share <= weight + 5, "{}: {share}% for weight {weight}", d.building_tiers[t].base.type_);
    }
    let library = b(d, "BUILDING_LIBRARY");
    assert!(d.building_classes[class(d, library)].rolls_tier, "a class with variants rolls");
    let great_library = b(d, "BUILDING_GREAT_LIBRARY");
    assert!(d.building_classes[class(d, great_library)].rolls_tier, "a world wonder rolls (its cost)");
    let mastery = b(d, "BUILDING_TOWER_OF_MASTERY");
    assert!(!d.building_classes[class(d, mastery)].rolls_tier, "a victory building does not");
}

#[test]
fn a_city_builds_only_its_own_tier_and_holds_only_it() {
    let (mut g, cities) = game(1);
    let d = g.d;
    let c = cities[0];
    let library = b(d, "BUILDING_LIBRARY");
    let fine = d.building_variant(library, BuildingTierType::FINE);
    assert_ne!(fine, library);
    g.cities[c].building_tiers[class(d, library)] = BuildingTierType::FINE;

    for &v in &d.buildings[library].tier_variants {
        assert_eq!(g.city_can_construct(c, v, false, false, false), v == fine, "{}", d.buildings[v].base.type_);
    }

    // Given at another tier (an event, a spell, a capture), it is the city's own.
    let before = g.cities[c].commerce_rate_modifier[CommerceType::COMMERCE_RESEARCH.idx()];
    g.city_set_num_real_building(c, library, 1);
    assert_eq!(g.cities[c].num_real_building[fine], 1);
    assert_eq!(g.cities[c].num_real_building[library], 0);
    assert_eq!(g.city_num_family(c, library), 1);
    assert_eq!(g.city_num_family(c, d.building_variant(library, BuildingTierType::GRAND)), 1, "any tier asks for the family");
    assert_eq!(g.city_family_tier(c, library), Some(BuildingTierType::FINE));
    let after = g.cities[c].commerce_rate_modifier[CommerceType::COMMERCE_RESEARCH.idx()];
    assert_eq!(after - before, 38, "a Fine Library is +38% research");
    assert!(!g.city_can_construct(c, fine, false, false, false), "built");

    // Removing it by any tier removes the city's.
    g.city_set_num_real_building(c, library, 0);
    assert_eq!(g.city_num_family(c, library), 0);
    assert_eq!(g.cities[c].commerce_rate_modifier[CommerceType::COMMERCE_RESEARCH.idx()], before);
}

#[test]
fn a_building_that_unlocks_units_unlocks_them_at_every_tier() {
    let (mut g, cities) = game(1);
    let d = g.d;
    let c = cities[0];
    let stable = b(d, "BUILDING_STABLE");
    let horseman = d.t::<UnitType>("UNIT_HORSEMAN");
    let horses = d.t::<BonusType>("BONUS_HORSE");
    g.city_change_free_bonus(c, horses, 1);
    let base_xp = g.city_production_experience(c, Some(horseman));
    let base_mod = g.city_production_modifier_unit(c, horseman);
    for (tier, xp, modifier) in [
        (BuildingTierType::LESSER, 0, -25),
        (BuildingTierType::STANDARD, 0, 0),
        (BuildingTierType::FINE, 2, 0),
        (BuildingTierType::GRAND, 4, 0),
        (BuildingTierType::LEGENDARY, 4, 0),
    ] {
        g.city_set_num_real_building(c, stable, 0);
        g.cities[c].building_tiers[class(d, stable)] = tier;
        assert!(!g.city_can_train(c, horseman, false, false, false, true), "{tier:?}: no stable yet");
        g.city_set_num_real_building(c, stable, 1);
        assert!(g.city_can_train(c, horseman, false, false, false, true), "{tier:?}: any tier of the Stable");
        assert_eq!(g.city_production_experience(c, Some(horseman)) - base_xp, xp, "{tier:?}");
        assert_eq!(g.city_production_modifier_unit(c, horseman) - base_mod, modifier, "{tier:?}");
    }
    // The Legendary promotion goes to the units it unlocks.
    let (_, _, promotions) = g.city_unlocked_unit_bonus(c, horseman);
    assert_eq!(promotions, vec![d.t::<PromotionType>("PROMOTION_MOBILITY1")]);
}

#[test]
fn a_wonder_costs_its_tier_share() {
    let (mut g, cities) = game(1);
    let d = g.d;
    let c = cities[0];
    let wonder = b(d, "BUILDING_GREAT_LIBRARY");
    let k = class(d, wonder);
    g.cities[c].building_tiers[k] = BuildingTierType::STANDARD;
    let standard = g.city_production_needed_building(c, wonder);
    for t in d.building_tiers.keys() {
        g.cities[c].building_tiers[k] = t;
        assert_eq!(g.city_production_needed_building(c, wonder), standard * d.building_tiers[t].wonder_cost_percent / 100);
    }
    // An ordinary building's cost is the same at every tier.
    let library = b(d, "BUILDING_LIBRARY");
    let cost = g.city_production_needed_building(c, library);
    for &v in &d.buildings[library].tier_variants {
        assert_eq!(g.city_production_needed_building(c, v), cost);
    }
}

#[test]
fn a_city_changing_hands_keeps_its_tiers_and_its_buildings_tiers() {
    let (mut g, cities) = game(1);
    let d = g.d;
    let c = cities[0];
    let walls = b(d, "BUILDING_WALLS");
    g.cities[c].building_tiers[class(d, walls)] = BuildingTierType::GRAND;
    g.city_set_num_real_building(c, walls, 1);
    let tiers = g.cities[c].building_tiers.clone();
    let p1 = PlayerId(1);
    let n_before = g.players[p1].cities.len();
    g.player_acquire_city(p1, c, false, true, true);
    let new_city = *g.players[p1].cities.last().unwrap();
    assert_eq!(g.players[p1].cities.len(), n_before + 1);
    assert_eq!(g.cities[new_city].building_tiers, tiers);
    assert_eq!(g.city_family_tier(new_city, walls), Some(BuildingTierType::GRAND), "traded: the walls stay, Grand");
}

#[test]
fn a_lesser_warrens_slows_the_units_it_copies() {
    let (mut g, cities) = game(1);
    let d = g.d;
    let c = cities[0];
    let warrens = b(d, "BUILDING_WARRENS");
    let axeman = d.t::<UnitType>("UNIT_AXEMAN");
    let catapult = d.t::<UnitType>("UNIT_CATAPULT");
    let (axe0, cat0) = (g.city_production_modifier_unit(c, axeman), g.city_production_modifier_unit(c, catapult));
    g.cities[c].building_tiers[class(d, warrens)] = BuildingTierType::LESSER;
    g.city_set_num_real_building(c, warrens, 1);
    assert_eq!(g.city_production_modifier_unit(c, axeman) - axe0, -25);
    assert_eq!(g.city_production_modifier_unit(c, catapult) - cat0, 0, "siege units are not copied");
}

#[test]
fn the_tiers_are_saved() {
    let (g, cities) = game(2);
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).expect("load");
    for &c in &cities {
        assert_eq!(h.cities[c].building_tiers, g.cities[c].building_tiers);
    }
}
