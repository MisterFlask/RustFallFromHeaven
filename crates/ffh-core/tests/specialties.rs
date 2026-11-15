//! City specialty rules, built situations rather than AI outcomes.
use ffh_core::city::specialties::*;
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
    g.players[p0].human = true;
    (g, cities)
}

fn offer(g: &mut Game, c: CityId, specialty: u8) {
    g.cities[c].specialty = None;
    g.cities[c].specialty_options = vec![(specialty, None)];
    assert!(g.city_choose_specialty(c, 0));
}

#[test]
fn three_choices_at_developing_are_saved_and_chosen_only_once() {
    let (mut g, cs) = game(2);
    let (a, b) = (cs[0], cs[1]);
    let developing = CultureLevelType::from_usize(3);
    g.city_set_culture_level(a, Some(CultureLevelType::from_usize(2)), true);
    assert!(g.cities[a].specialty_options.is_empty());
    g.city_set_culture_level(a, Some(developing), true);
    let options = g.cities[a].specialty_options.clone();
    assert_eq!(options.len(), 3);
    for (i, &(s, k)) in options.iter().enumerate() {
        assert!(g.city_specialty_eligible(a, s));
        assert!(options[..i].iter().all(|o| o.0 != s));
        if s == CLASS_XP { assert!(k.is_some()); }
    }
    let bytes = g.save();
    let mut loaded = Game::load(g.d, &bytes).unwrap();
    loaded.city_offer_specialty(a);
    assert_eq!(loaded.save(), bytes, "no reroll on load or repeated offer");
    assert!(!g.city_choose_specialty(a, 3));
    assert!(g.city_choose_specialty(a, 1));
    assert_eq!(g.cities[a].specialty, Some(options[1].0));
    assert!(!g.city_choose_specialty(a, 0));
    g.city_set_culture_level(a, Some(CultureLevelType::from_usize(1)), true);
    g.city_set_culture_level(a, Some(developing), true);
    assert!(g.cities[a].specialty_options.is_empty());
    g.city_set_culture_level(b, Some(developing), true);
    assert!(g.cities[b].specialty_options.iter().all(|o| Some(o.0) != g.cities[a].specialty));
}

#[test]
fn choosing_refreshes_a_conflicting_offer_and_capture_keeps_the_effect() {
    let (mut g, cs) = game(2);
    let (a, b) = (cs[0], cs[1]);
    for c in [a, b] {
        g.city_set_culture_level(c, Some(CultureLevelType::from_usize(3)), true);
        g.cities[c].specialty_options = vec![(FOOD, None), (CULTURE, None), (WONDERS, None)];
    }
    g.city_choose_specialty(a, 0);
    assert!(g.cities[b].specialty_options.iter().all(|o| o.0 != FOOD));
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    g.player_acquire_city(PlayerId(1), a, false, true, true);
    let c = g.map.plots[g.map.plot_xy(x, y).unwrap()].plot_city.unwrap();
    assert_eq!(g.cities[c].specialty, Some(FOOD));
    assert!(g.cities[c].specialty_options.is_empty());
}

#[test]
fn economic_and_training_effects_apply_at_their_sources() {
    let (mut g, cs) = game(1);
    let c = cs[0];
    let d = g.d;
    let food = g.city_yield_rate(c, YieldType::YIELD_FOOD);
    offer(&mut g, c, FOOD);
    assert_eq!(g.city_yield_rate(c, YieldType::YIELD_FOOD), food + 5);
    g.cities[c].base_great_people_rate = 3;
    g.cities[c].specialty = None;
    let gp = g.city_total_great_people_rate_modifier(c);
    offer(&mut g, c, GREAT_PEOPLE);
    assert_eq!(g.city_total_great_people_rate_modifier(c), gp * 2);
    let settler = d.t::<UnitType>("UNIT_SETTLER");
    offer(&mut g, c, SETTLERS);
    assert!(!g.city_is_food_production_unit(c, settler));
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let unit = g.player_init_unit(PlayerId(0), warrior, g.cities[c].x, g.cities[c].y, None, None);
    offer(&mut g, c, MOBILITY);
    g.city_add_production_experience(c, unit, false);
    assert!(g.units[unit].has_promotion[d.t::<PromotionType>("PROMOTION_MOBILITY1")]);
    offer(&mut g, c, NO_UPKEEP);
    g.city_add_production_experience(c, unit, false);
    assert_eq!(g.units[unit].trained_specialty, Some(NO_UPKEEP));
    let cost = g.player_calculate_unit_cost_breakdown(PlayerId(0));
    assert!(cost.free_units > 0);
    offer(&mut g, c, POP_RUSH);
    let pop = d.hurries.keys().find(|&h| d.hurries[h].production_per_population > 0).unwrap();
    assert!(g.city_specialty_can_hurry(c, pop));
    assert_eq!(g.city_hurry_anger_length(c, pop), 0);
    offer(&mut g, c, GOLD_RUSH);
    let gold = d.hurries.keys().find(|&h| d.hurries[h].gold_per_production > 0).unwrap();
    assert!(g.city_specialty_can_hurry(c, gold));
    assert_eq!(g.city_hurry_gold_for_cost(c, gold, 100), 50 * d.hurries[gold].gold_per_production);
}

#[test]
fn plot_specialist_religion_wonder_and_class_bonuses_have_the_documented_amounts() {
    let (mut g, cs) = game(1);
    let c = cs[0];
    let d = g.d;
    let plot = g.city_plot(c);
    let food = YieldType::YIELD_FOOD;
    let prod = YieldType::YIELD_PRODUCTION;
    let research = CommerceType::COMMERCE_RESEARCH;
    g.map.plots[plot].improvement_type = Some(d.t("IMPROVEMENT_FARM"));
    offer(&mut g, c, FARMS);
    assert_eq!(g.city_specialty_plot_yield(c, plot, food), 1);
    assert_eq!(g.city_specialty_plot_yield(c, plot, prod), 0);
    offer(&mut g, c, MINES);
    assert_eq!(g.city_specialty_plot_yield(c, plot, prod), 0);
    g.map.plots[plot].improvement_type = Some(d.t("IMPROVEMENT_QUARRY"));
    assert_eq!(g.city_specialty_plot_yield(c, plot, prod), 1);
    g.cities[c].specialty = None;
    let base = g.city_base_commerce_rate_times100(c, research);
    let sage = d.specialists.keys().next().unwrap();
    g.cities[c].free_specialist_count[sage] += 2;
    // Directly edited counts change the ordinary extra-commerce term too; compare after edit.
    let base_with_specialists = g.city_base_commerce_rate_times100(c, research);
    assert!(base_with_specialists >= base);
    offer(&mut g, c, SAGES);
    let count: i32 = d.specialists.keys().map(|s| g.cities[c].specialist_count[s] + g.cities[c].free_specialist_count[s]).sum();
    assert_eq!(g.city_base_commerce_rate_times100(c, research), base_with_specialists + 100 * count);
    let religion = d.religions.keys().next().unwrap();
    g.cities[c].has_religion[religion] = true;
    g.cities[c].specialty = None;
    let culture = g.city_base_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE);
    let gold = g.city_base_commerce_rate_times100(c, CommerceType::COMMERCE_GOLD);
    offer(&mut g, c, RELIGIONS);
    assert_eq!(g.city_base_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE), culture + 200 * g.city_religion_count(c));
    assert_eq!(g.city_base_commerce_rate_times100(c, CommerceType::COMMERCE_GOLD), gold + 100 * g.city_religion_count(c));
    let wonder = d.t::<BuildingType>("BUILDING_GREAT_LIBRARY");
    let library = d.t::<BuildingType>("BUILDING_LIBRARY");
    g.cities[c].specialty = None;
    let w = g.city_production_modifier_building(c, wonder);
    let b = g.city_production_modifier_building(c, library);
    offer(&mut g, c, WONDERS);
    assert_eq!(g.city_production_modifier_building(c, wonder), (100 + w) * 3 / 2 - 100);
    assert_eq!(g.city_production_modifier_building(c, library), b);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    g.cities[c].specialty = None;
    let xp = g.city_production_experience(c, Some(warrior));
    g.cities[c].specialty_options = vec![(CLASS_XP, d.units[warrior].unit_combat_type)];
    assert!(g.city_choose_specialty(c, 0));
    assert_eq!(g.city_production_experience(c, Some(warrior)), xp + 3);
}
