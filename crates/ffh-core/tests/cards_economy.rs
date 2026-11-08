//! The economy and content batch of cards (docs/CARDS.md, section 5): each card's rules on a
//! situation built by hand. The cards are techs; a test gives its team the card (and every other
//! tech it needs).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

/// A small map after `game_init`, the starting units gone, player 0 with one city on open land and
/// every tech but the cards; players 0 and 1 met.
fn game() -> (Game, PlayerId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    let p0 = PlayerId(0);
    for p in [p0, PlayerId(1)] {
        for u in g.players[p].units.clone() {
            g.unit_kill(u, false, None);
        }
    }
    // A land plot with land all around it, far from the other player's start.
    let q = g
        .map
        .plot_ids()
        .find(|&q| {
            let (x, y) = g.map.xy(q);
            !g.plot_is_water(q)
                && !g.plot_is_impassable(q)
                && g.map.plots[q].owner.is_none()
                && (-4..=4).all(|dx| (-4..=4).all(|dy| g.map.plot_xy(x + dx, y + dy).is_some_and(|r| !g.plot_is_water(r) && g.map.plots[r].owner.is_none())))
        })
        .expect("open land");
    let (x, y) = g.map.xy(q);
    let c = g.player_init_city(p0, x, y, true, true);
    let team = g.players[p0].team;
    for t in d.techs.keys() {
        if d.techs[t].card.is_none() {
            g.team_set_has_tech(team, t, true, Some(p0), false, false);
        }
    }
    let t1 = g.players[PlayerId(1)].team;
    g.team_meet(team, t1, false);
    (g, p0, c)
}

fn card(g: &mut Game, p: PlayerId, name: &str) {
    let t = g.d.t::<TechType>(name);
    let team = g.players[p].team;
    g.team_set_has_tech(team, t, true, Some(p), false, false);
}

fn build(g: &mut Game, c: CityId, name: &str) -> BuildingType {
    let b = g.d.t::<BuildingType>(name);
    g.city_set_num_real_building(c, b, 1);
    b
}

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, None, None)
}

/// The base production cost of `unit` for `p`, before the game speed and era (the scaling
/// `player_get_production_needed_unit` applies to every unit alike).
fn scaled(g: &Game, base: i32) -> i32 {
    let d = g.d;
    let mut n = base * d.defines.unit_production_percent / 100;
    n = n * d.game_speeds[g.game_speed_type()].train_percent / 100;
    n * d.eras[g.start_era()].train_percent / 100
}

/// Gives city `c` one of resource `bonus` (a free bonus, as a building gives it).
fn give_bonus(g: &mut Game, c: CityId, name: &str) -> BonusType {
    let b = g.d.t::<BonusType>(name);
    g.city_change_free_bonus(c, b, 1);
    b
}

#[test]
fn the_beast_market_trains_the_animals_at_its_prices() {
    let (mut g, p, c) = game();
    g.players[p].human = true; // no AI handicap on the costs
    let bear = g.d.t::<UnitType>("UNIT_BEAR");
    let lion = g.d.t::<UnitType>("UNIT_LION");
    assert_eq!(g.d.units[bear].production_cost, -1, "the Bear has no cost of its own");
    assert!(!g.city_can_train(c, bear, false, false, false, false), "no city trains animals");
    card(&mut g, p, "TECH_CARD_BEAST_MARKET");
    build(&mut g, c, "BUILDING_BEAST_MARKET");
    assert!(g.city_can_train(c, bear, false, false, false, false));
    assert!(g.city_can_train(c, lion, false, false, false, false));
    assert_eq!(g.city_production_needed_unit(c, bear), scaled(&g, 75));
    assert_eq!(g.city_production_needed_unit(c, lion), scaled(&g, 50));
    // Trained, an animal is its owner's like any unit.
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let u = g.player_init_unit(p, bear, x, y, None, None);
    assert_eq!(g.units[u].owner, p);
}

#[test]
fn the_elemental_foundry_needs_the_mana_and_prices_the_second_of_an_element() {
    let (mut g, p, c) = game();
    g.players[p].human = true;
    let fire = g.d.t::<UnitType>("UNIT_FIRE_ELEMENTAL");
    let water = g.d.t::<UnitType>("UNIT_WATER_ELEMENTAL");
    card(&mut g, p, "TECH_CARD_ELEMENTAL_FOUNDRY");
    build(&mut g, c, "BUILDING_ELEMENTAL_FOUNDRY");
    assert!(!g.city_can_train(c, fire, false, false, false, false), "no Fire mana");
    give_bonus(&mut g, c, "BONUS_MANA_FIRE");
    assert!(g.city_can_train(c, fire, false, false, false, false));
    assert!(!g.city_can_train(c, water, false, false, false, false), "Fire mana is not Water mana");
    let normal = g.city_production_needed_unit(c, fire);
    assert_eq!(normal, scaled(&g, 300));
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    g.player_init_unit(p, fire, x, y, None, None);
    let percent = g.d.defines.national_unit_over_limit_cost_percent;
    assert_eq!(g.city_production_needed_unit(c, fire), normal * percent / 100, "one per element, then 500%");
}

#[test]
fn gold_standard_rushes_units_for_two_gold_a_hammer_and_not_buildings() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_GOLD_STANDARD");
    let economy = d.t::<CivicOptionType>("CIVICOPTION_ECONOMY");
    let civic = d.t::<CivicType>("CIVIC_GOLD_STANDARD");
    g.player_set_civics(p, economy, Some(civic));
    let hurry = d.t::<HurryType>("HURRY_GOLD_STANDARD");
    assert!(g.player_can_hurry(p, hurry), "under any Labor civic");
    assert_eq!(d.civics[civic].commerce_modifier[CommerceType::COMMERCE_RESEARCH.idx()], -20);
    g.players[p].gold = 10_000;
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, false);
    assert!(g.city_can_hurry(c, hurry, false));
    let cost = g.city_hurry_cost(c, false);
    assert_eq!(g.city_hurry_gold(c, hurry), (cost * 2).max(1));
    g.city_clear_order_queue(c);
    let monument = d.t::<BuildingType>("BUILDING_MONUMENT");
    g.city_push_order(c, OrderType::ORDER_CONSTRUCT, monument.i(), -1, false, false, false, false);
    assert!(!g.city_can_hurry(c, hurry, false), "units only");
    assert!(!g.city_can_hurry_building(c, hurry, monument, false));
}

#[test]
fn magocracy_halves_the_casters_and_raises_the_rest() {
    let (mut g, p, _) = game();
    g.players[p].human = true;
    let d = g.d;
    let adept = d.t::<UnitType>("UNIT_ADEPT");
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let (a0, w0) = (g.player_get_production_needed_unit(p, adept), g.player_get_production_needed_unit(p, warrior));
    card(&mut g, p, "TECH_CARD_MAGOCRACY");
    let gov = d.t::<CivicOptionType>("CIVICOPTION_GOVERNMENT");
    g.player_set_civics(p, gov, Some(d.t::<CivicType>("CIVIC_MAGOCRACY")));
    assert_eq!(g.player_get_production_needed_unit(p, adept), a0 / 2);
    assert_eq!(g.player_get_production_needed_unit(p, warrior), w0 * 125 / 100);
}

#[test]
fn stoicism_ends_the_unhappiness_of_war_weariness() {
    let (mut g, p, c) = game();
    let d = g.d;
    g.players[p].war_weariness_percent_anger = 200;
    assert!(g.city_war_weariness_percent_anger(c) > 0);
    card(&mut g, p, "TECH_CARD_STOICISM");
    let values = d.t::<CivicOptionType>("CIVICOPTION_CULTURAL_VALUES");
    g.player_set_civics(p, values, Some(d.t::<CivicType>("CIVIC_STOICISM")));
    assert_eq!(g.city_war_weariness_percent_anger(c), 0);
    assert_eq!(d.civics[d.t::<CivicType>("CIVIC_STOICISM")].commerce_modifier[CommerceType::COMMERCE_CULTURE.idx()], -15);
}

#[test]
fn the_earthmother_lifts_the_mithril_golems_counter_and_halves_its_cost() {
    let (mut g, p, _) = game();
    g.players[p].human = true;
    let d = g.d;
    let golem = d.t::<UnitType>("UNIT_MITHRIL_GOLEM");
    assert!(d.units[golem].prereq_global_counter > g.get_global_counter());
    let kilmorph = d.t::<ReligionType>("RELIGION_RUNES_OF_KILMORPH");
    g.player_set_last_state_religion(p, Some(kilmorph));
    assert!(!g.player_can_train(p, golem, false, false, false), "the counter is below 70");
    let full = g.player_get_production_needed_unit(p, golem);
    card(&mut g, p, "TECH_CARD_GOLEMS_OF_THE_EARTHMOTHER");
    assert!(g.player_can_train(p, golem, false, false, false), "whatever the counter");
    assert_eq!(g.player_get_production_needed_unit(p, golem), full / 2);
    // Kilmorph stays its state religion.
    g.player_set_last_state_religion(p, None);
    assert!(!g.player_can_train(p, golem, false, false, false));
}

#[test]
fn maponos_cradle_makes_a_settler_every_twenty_turns_or_a_worker() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_MAPONOS_CRADLE");
    let b = build(&mut g, c, "BUILDING_MAPONOS_CRADLE");
    let built = g.cities[c].building_original_time[b];
    let settler = d.t::<UnitType>("UNIT_SETTLER");
    let worker = d.t::<UnitType>("UNIT_WORKER");
    let count = |g: &Game, t: UnitType| g.players[p].units.iter().filter(|&&u| g.units[u].unit_type == t).count();
    let pop = g.cities[c].population;
    // Not before the twentieth turn.
    g.set_game_turn(built + 19);
    g.city_do_free_units(c);
    assert_eq!(count(&g, settler), 0);
    g.set_game_turn(built + 20);
    assert!(g.city_free_site_near(c, ffh_core::ffh::cards::economy::FREE_UNIT_SITE_RANGE));
    g.city_do_free_units(c);
    assert_eq!(count(&g, settler), 1);
    assert_eq!(g.cities[c].population, pop, "without losing population");
    // With no site free, a Worker (every plot within 10 marked as no-city land).
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    for dx in -10..=10 {
        for dy in -10..=10 {
            if let Some(q) = g.map.plot_xy(x + dx, y + dy) {
                g.map.plots[q].found_disabled = true;
            }
        }
    }
    g.set_game_turn(built + 40);
    g.city_do_free_units(c);
    assert_eq!((count(&g, settler), count(&g, worker)), (1, 1));
}

#[test]
fn muster_turns_production_into_experience_for_the_least_experienced() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_MUSTER");
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let a = unit(&mut g, p, "UNIT_WARRIOR", x, y);
    let b = unit(&mut g, p, "UNIT_WARRIOR", x, y);
    g.units[a].experience = 3;
    let muster = d.t::<ProcessType>("PROCESS_MUSTER");
    g.city_clear_order_queue(c);
    g.city_push_order(c, OrderType::ORDER_MAINTAIN, muster.i(), -1, false, false, false, false);
    let hammers = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
    assert!(hammers > 0);
    // Enough turns of production for 4 points of experience.
    let per = d.processes[muster].production_to_experience;
    g.cities[c].muster_production = 4 * per - hammers;
    g.city_do_muster(c);
    // b (0) takes points until it passes a (3): 0->1->2->3, then the tie goes to the first.
    assert_eq!(g.units[a].experience + g.units[b].experience, 3 + 4);
    assert!(g.units[b].experience >= 3);
    assert_eq!(g.cities[c].muster_production, 0);
}

#[test]
fn the_spire_gives_a_trained_caster_a_sphere_of_a_mana_held() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_SPIRE_OF_THE_ARCHMAGI");
    build(&mut g, c, "BUILDING_SPIRE_OF_THE_ARCHMAGI");
    give_bonus(&mut g, c, "BONUS_MANA_WATER");
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let adept = unit(&mut g, p, "UNIT_ADEPT", x, y);
    let spire = d.t::<BuildingType>("BUILDING_SPIRE_OF_THE_ARCHMAGI");
    let options = &d.buildings[spire].free_promotion_choices;
    // The default controller takes the first it can acquire (the Bannor palace's Earth, Law and
    // Spirit, and the Water given above).
    let first = options.iter().copied().find(|&pr| g.unit_can_acquire_promotion(adept, pr)).expect("a sphere");
    g.city_add_production_experience(c, adept, false);
    let taken: Vec<PromotionType> = options.iter().copied().filter(|&pr| g.units[adept].has_promotion[pr]).collect();
    assert_eq!(taken, vec![first], "one sphere");
    assert!(d.promotions[first].bonus_prereq.is_some_and(|b| g.player_has_bonus(p, b)), "of a mana its empire holds");
    assert_eq!(g.units[adept].level, 1, "without a level");
    // A warrior takes none.
    let warrior = unit(&mut g, p, "UNIT_WARRIOR", x, y);
    g.city_add_production_experience(c, warrior, false);
    assert!(options.iter().all(|&pr| !g.units[warrior].has_promotion[pr]));
}

#[test]
fn agares_well_gives_the_chosen_mana_and_moves_the_counter() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_AGARES_WELL");
    let before = g.get_global_counter();
    let b = build(&mut g, c, "BUILDING_AGARES_WELL");
    // The default controller takes the first choice: Air.
    let air = d.t::<BonusType>("BONUS_MANA_AIR");
    assert_eq!(g.cities[c].free_bonus_choices, vec![(b, air)]);
    assert!(g.city_has_bonus(c, air));
    assert_eq!(g.get_global_counter(), before + 3);
    g.city_set_num_real_building(c, b, 0);
    assert!(!g.city_has_bonus(c, air), "gone with the Well");
    assert!(g.cities[c].free_bonus_choices.is_empty(), "chosen again if built again");
    assert_eq!(g.get_global_counter(), before);
}

#[test]
fn a_trading_post_claims_its_plots_until_it_is_pillaged() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_TRADING_POST");
    let post_build = d.t::<BuildType>("BUILD_TRADING_POST");
    let worker_type = d.t::<UnitType>("UNIT_WORKER");
    assert!(d.units[worker_type].builds[post_build], "workers build it (same_builders_as)");
    assert!(d.units[d.t::<UnitType>("UNIT_PIONEER")].builds[post_build]);
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    // Not in the city's borders, not within 2 plots of a city.
    let near = g.map.plot_xy(cx + 2, cy).unwrap();
    assert!(!g.player_can_build(p, near, post_build, false, false));
    // An unowned flat land plot 4 to 8 plots out, with land around it, where a post can go.
    let site = g
        .map
        .plot_ids()
        .find(|&q| {
            let (x, y) = g.map.xy(q);
            let dist = g.step_distance(x, y, cx, cy);
            (4..=8).contains(&dist)
                && !g.plot_is_peak(q)
                && (-1..=1).all(|dx| (-1..=1).all(|dy| g.map.plot_xy(x + dx, y + dy).is_some_and(|r| !g.plot_is_water(r) && g.map.plots[r].owner.is_none())))
                && g.player_can_build(p, q, post_build, false, false)
        })
        .expect("a site for the post");
    let (sx, sy) = g.map.xy(site);
    let w = unit(&mut g, p, "UNIT_WORKER", sx, sy);
    let mut done = false;
    for _ in 0..100 {
        g.units[w].moves = 0; // (moves spent)
        if g.unit_build(w, post_build) {
            done = true;
            break;
        }
    }
    assert!(done, "the worker finishes the post");
    let post = d.t::<ImprovementType>("IMPROVEMENT_TRADING_POST");
    assert_eq!(g.map.plots[site].improvement_type, Some(post));
    for dx in -1..=1 {
        for dy in -1..=1 {
            let q = g.map.plot_xy(sx + dx, sy + dy).unwrap();
            if !g.plot_is_water(q) {
                assert_eq!(g.map.plots[q].owner, Some(p), "({dx},{dy}) claimed");
                // Culture does not move it.
                g.plot_update_culture(q, false, true);
                assert_eq!(g.map.plots[q].owner, Some(p));
            }
        }
    }
    // Pillaged, the claim ends.
    g.plot_set_improvement_type(site, None);
    assert_eq!(g.map.plots[site].owner, None, "unowned again");
}

#[test]
fn the_font_of_rebirth_finds_the_nearest_font_once() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_FONT_OF_REBIRTH");
    build(&mut g, c, "BUILDING_FONT_OF_REBIRTH");
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let battle = g.map.plot_xy(cx + 3, cy).unwrap();
    let far = g.map.plot_xy(cx + 4, cy).unwrap();
    let u = unit(&mut g, p, "UNIT_WARRIOR", cx + 3, cy);
    let reborn = d.t::<PromotionType>("PROMOTION_REBORN");
    assert_eq!(g.unit_rebirth_font(u, battle), Some((c, reborn)));
    assert_eq!(g.unit_rebirth_font(u, far), None, "beyond 3 plots");
    g.unit_carry_to_font(u, c, reborn);
    assert_eq!((g.units[u].x, g.units[u].y), (cx, cy));
    assert!(g.units[u].has_promotion[reborn]);
    assert_eq!(g.unit_rebirth_font(u, battle), None, "a Reborn unit dies normally");
    // Never drawn by an evil civilization.
    let font = d.t::<TechType>("TECH_CARD_FONT_OF_REBIRTH");
    assert_eq!(d.techs[font].card.as_ref().unwrap().block_alignment, Some(d.t::<AlignmentType>("ALIGNMENT_EVIL")));
}

#[test]
fn mana_grafting_turns_a_node_into_another_type_once() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_MANA_GRAFTING");
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let a = g.map.plot_xy(cx + 1, cy).unwrap();
    let b = g.map.plot_xy(cx - 1, cy).unwrap();
    let (fire, water) = (d.t::<BonusType>("BONUS_MANA_FIRE"), d.t::<BonusType>("BONUS_MANA_WATER"));
    g.plot_set_bonus_type(a, Some(d.t::<BonusType>("BONUS_MANA")));
    g.plot_set_bonus_type(b, Some(d.t::<BonusType>("BONUS_MANA")));
    g.plot_set_improvement_type(a, Some(d.t::<ImprovementType>("IMPROVEMENT_MANA_FIRE")));
    g.plot_set_improvement_type(b, Some(d.t::<ImprovementType>("IMPROVEMENT_MANA_WATER")));
    assert_eq!((g.map.plots[a].bonus_type, g.map.plots[b].bonus_type), (Some(fire), Some(water)));
    assert_eq!(g.plot_graft_targets(a, p), vec![water]);
    let (ax, ay) = g.map.xy(a);
    let adept = unit(&mut g, p, "UNIT_ADEPT", ax, ay);
    let spell = d.t::<SpellType>("SPELL_MANA_GRAFT");
    let target = SpellTarget::Choice(water.i());
    assert!(g.unit_can_cast_at(adept, spell, target));
    g.unit_cast_at(adept, spell, target);
    assert_eq!(g.map.plots[a].bonus_type, Some(water));
    assert_eq!(g.map.plots[a].improvement_type, Some(d.t::<ImprovementType>("IMPROVEMENT_MANA_WATER")));
    assert!(g.map.plots[a].grafted);
    assert!(g.plot_graft_targets(a, p).is_empty(), "grafted once, for good");
    // A warrior is no caster.
    let warrior = unit(&mut g, p, "UNIT_WARRIOR", ax, ay);
    assert!(!g.unit_can_cast(warrior, spell, false));
}

#[test]
fn the_paymaster_buys_an_adjacent_enemy_unit() {
    let (mut g, p, c) = game();
    let d = g.d;
    card(&mut g, p, "TECH_CARD_PAYMASTER");
    let enemy = PlayerId(1);
    let (t0, t1) = (g.players[p].team, g.players[enemy].team);
    g.team_declare_war(t0, t1, false, None);
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let pm = unit(&mut g, p, "UNIT_PAYMASTER", cx + 2, cy);
    let axe = unit(&mut g, enemy, "UNIT_AXEMAN", cx + 3, cy);
    let price = g.unit_buy_out_price(axe).unwrap();
    let axeman = d.t::<UnitType>("UNIT_AXEMAN");
    assert_eq!(price, 3 * (d.units[axeman].production_cost * d.game_speeds[g.game_speed_type()].train_percent / 100));
    g.players[p].gold = price - 1;
    assert!(g.unit_buy_out_targets(pm).is_empty(), "too dear");
    g.players[p].gold = price + 10;
    let spell = d.t::<SpellType>("SPELL_BUY_OUT");
    assert!(g.unit_can_cast_at(pm, spell, SpellTarget::Unit(axe)));
    g.unit_cast_at(pm, spell, SpellTarget::Unit(axe));
    assert!(!g.units.contains(axe) || g.units[axe].death_delay, "the enemy's unit is gone");
    assert_eq!(g.players[p].gold, 10);
    assert!(g.players[p].units.iter().any(|&u| g.units[u].unit_type == axeman && (g.units[u].x, g.units[u].y) == (cx + 2, cy)));
    // A hero is never for sale.
    let hero = unit(&mut g, enemy, "UNIT_AXEMAN", cx + 3, cy);
    g.unit_set_has_promotion(hero, d.t::<PromotionType>("PROMOTION_HERO"), true);
    assert_eq!(g.unit_buy_out_price(hero), None);
}

#[test]
fn the_academy_and_the_admiralty_start_units_at_level_three() {
    let d = ffh_data::global();
    let academy = d.t::<BuildingType>("BUILDING_ACADEMY_OF_WAR");
    let admiralty = d.t::<BuildingType>("BUILDING_ADMIRALTY");
    // Level 2 needs 2 experience, level 3 another... 5 in all (`experience_needed`: level^2 + 1).
    assert_eq!(d.buildings[academy].domain_free_experience[DomainType::DOMAIN_LAND], 5);
    assert_eq!(d.buildings[admiralty].domain_free_experience[DomainType::DOMAIN_SEA], 5);
    assert!(d.buildings[admiralty].water);
    let (mut g, p, c) = game();
    build(&mut g, c, "BUILDING_ACADEMY_OF_WAR");
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let w = unit(&mut g, p, "UNIT_WARRIOR", x, y);
    g.city_add_production_experience(c, w, false);
    assert!(g.units[w].experience >= g.py_get_experience_needed(2, p), "enough for level 3");
}

#[test]
fn pioneers_build_three_times_as_fast() {
    let d = ffh_data::global();
    let pioneer = d.t::<UnitType>("UNIT_PIONEER");
    let worker = d.t::<UnitType>("UNIT_WORKER");
    assert_eq!(d.units[pioneer].work_rate, 3 * d.units[worker].work_rate);
    let class = d.units[pioneer].unit_class_type.unwrap();
    assert_eq!(d.unit_classes[class].max_player_instances, 4);
    assert!(!d.unit_classes[class].hard_limit, "national: 4, then 500%");
    for b in d.builds.keys() {
        assert_eq!(d.units[pioneer].builds[b], d.units[worker].builds[b], "{}", d.builds[b].base.type_);
    }
}
