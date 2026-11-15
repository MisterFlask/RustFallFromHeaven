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
    g.cities[c].specialty_options = vec![(specialty, None, None)];
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
    for (i, &(s, k, mana)) in options.iter().enumerate() {
        assert!(g.city_specialty_eligible(a, s));
        assert!(options[..i].iter().all(|o| o.0 != s));
        if s == CLASS_XP { assert!(k.is_some()); }
        if s == MANA { assert!(mana.is_some()); }
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
        g.cities[c].specialty_options = vec![(FOOD, None, None), (CULTURE, None, None), (WONDERS, None, None)];
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
fn specialist_religion_wonder_and_class_bonuses_have_the_documented_amounts() {
    let (mut g, cs) = game(1);
    let c = cs[0];
    let d = g.d;
    let research = CommerceType::COMMERCE_RESEARCH;
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
    g.cities[c].specialty_options = vec![(CLASS_XP, d.units[warrior].unit_combat_type, None)];
    assert!(g.city_choose_specialty(c, 0));
    assert_eq!(g.city_production_experience(c, Some(warrior)), xp + 3);
}

#[test]
fn revised_pool_rejects_all_seven_removed_specialties() {
    let (mut g, cs) = game(1);
    let c = cs[0];
    assert_eq!(SPECIALTIES.len(), 16);
    for retired in [1, 2, 4, 5, 10, 12, 14] {
        assert!(!g.city_specialty_eligible(c, retired));
        g.cities[c].specialty_options = vec![(retired, None, None)];
        assert!(!g.city_choose_specialty(c, 0));
    }
}

#[test]
fn typed_mana_is_disclosed_saved_and_transferred_with_the_city() {
    let (mut g, cs) = game(2);
    let c = cs[0];
    let mana = g.d.t::<BonusType>("BONUS_MANA_FIRE");
    assert!(g.city_specialty_manas().contains(&mana));
    assert!(!g.city_specialty_manas().contains(&g.d.t("BONUS_MANA")));
    g.cities[c].specialty_options = vec![(MANA, None, Some(mana)), (FOOD, None, None), (WONDERS, None, None)];
    assert_eq!(g.specialty_text(MANA, None, Some(mana)), "This city provides 1 Fire Mana.");
    let saved = g.save();
    let mut restored = Game::load(g.d, &saved).unwrap();
    assert_eq!(restored.cities[c].specialty_options[0].2, Some(mana));
    let before = restored.city_num_bonuses(c, mana);
    assert!(restored.city_choose_specialty(c, 0));
    assert_eq!(restored.cities[c].specialty_mana, Some(mana));
    assert_eq!(restored.city_num_bonuses(c, mana), before + 1);
    assert!(!restored.city_choose_specialty(c, 0));
    let saved = restored.save();
    let mut restored = Game::load(g.d, &saved).unwrap();
    assert_eq!(restored.save(), saved);
    assert_eq!(restored.city_num_bonuses(c, mana), before + 1);
    let plot = restored.city_plot(c);
    restored.player_acquire_city(PlayerId(1), c, false, true, true);
    let captured = restored.map.plots[plot].plot_city.unwrap();
    assert_eq!(restored.cities[captured].specialty_mana, Some(mana));
    assert!(restored.city_has_bonus(captured, mana));
}

#[test]
fn double_growth_adds_two_one_or_zero_without_crossing_the_happiness_cap() {
    for room in [2, 1, 0] {
        let (mut g, cs) = game(1);
        let c = cs[0];
        g.city_set_population(c, 5);
        g.cities[c].military_happiness_units = 1;
        g.cities[c].ai_emphasize_avoid_angry_citizens_count = 0;
        g.cities[c].ai_emphasize_avoid_unhealthy_citizens_count = 0;
        g.cities[c].ai_emphasize_avoid_growth_count = 0;
        let change = g.city_unhappy_level(c, 0) - g.city_happy_level(c) + room;
        g.city_change_extra_happiness(c, change);
        assert_eq!(g.city_happy_level(c) - g.city_unhappy_level(c, 0), room);
        offer(&mut g, c, DOUBLE_GROWTH);
        assert_eq!(g.city_population_growth(c), room);
        g.cities[c].food = g.city_growth_threshold(c) + 100;
        g.city_do_growth(c);
        assert_eq!(g.cities[c].population, 5 + room);
        assert!(g.city_happy_level(c) >= g.city_unhappy_level(c, 0));
    }
}
