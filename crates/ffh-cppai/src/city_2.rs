//! Port of CvCityAI.cpp:2749-5256 (FFH2 0.41 C++ AI).
//!
//! Notes:
//! - `bAsync` random draws (`GC.getASyncRand()`) use the AI's own stream like the synchronous
//!   ones (`self.rand`).
//! - PORT GAP: `eIgnoreAdvisor` in `AI_bestBuildingThreshold`: ffh-data does not load the
//!   buildings' advisor types (AdvisorInfos are not vendored), so the filter never excludes a
//!   building. The C++ AI always passes `NO_ADVISOR`, so no AI path is affected.
//! - The emphasize setters delegate to ffh-core (`Game::city_ai_set_emphasize` ports the same
//!   C++ and reassigns citizens through the owner's controller, i.e. `AI_assignWorkingPlots`).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

const MAX_INT: i32 = i32::MAX;

const Y_FOOD: usize = 0;
const Y_PROD: usize = 1;
const Y_COMM: usize = 2;
const C_GOLD: usize = 0;
const C_RESEARCH: usize = 1;
const C_CULTURE: usize = 2;
const C_ESPIONAGE: usize = 3;

impl Cai {
    /// C++ `BuildingTypes CvCityAI::AI_bestBuilding(int iFocusFlags = 0, int iMaxTurns = 0, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2749-2752)
    pub fn city_ai_best_building(&self, g: &Game, c: CityId, focus_flags: i32, max_turns: i32, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<BuildingType> {
        self.city_ai_best_building_threshold(g, c, focus_flags, max_turns, 0, async_, ignore_advisor)
    }

    /// C++ `BuildingTypes CvCityAI::AI_bestBuildingThreshold(int iFocusFlags = 0, int iMaxTurns = 0, int iMinThreshold = 0, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2754-2928)
    pub fn city_ai_best_building_threshold(&self, g: &Game, c: CityId, focus_flags: i32, max_turns: i32, min_threshold: i32, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<BuildingType> {
        let d = g.d;
        let p = g.cities[c].owner;
        let civ = g.city_civilization_type(c);

        let area_alone = self.player_ai_is_area_alone(g, p, g.city_area(c));

        let production_rank = g.city_find_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);

        let mut best_value = 0;
        let mut best_building: Option<BuildingType> = None;

        if focus_flags & BUILDINGFOCUS_CAPITAL != 0 {
            let mut best_turns_left = if max_turns > 0 { max_turns } else { MAX_INT };
            for class in d.building_classes.keys() {
                if let Some(loop_building) = d.civ_building(civ, class) {
                    if d.buildings[loop_building].capital {
                        if g.city_can_construct(c, loop_building, false, false, false) {
                            let turns_left = g.city_production_turns_left_building(c, loop_building, 0);

                            if turns_left <= best_turns_left {
                                best_building = Some(loop_building);
                                best_turns_left = turns_left;
                            }
                        }
                    }
                }
            }

            return best_building;
        }

        let num_cities = g.players[p].num_cities();
        let production_automated = g.cities[c].production_automated;

        for class in d.building_classes.keys() {
            if g.player_is_building_class_maxed_out(p, class, d.building_classes[class].extra_player_instances) {
                continue;
            }
            let Some(loop_building) = d.civ_building(civ, class) else {
                continue;
            };

            if !(g.city_num_building(c, loop_building) < d.defines.city_max_num_buildings
                && (!production_automated || !(is_world_wonder_class(d, class) || is_national_wonder_class(d, class))))
            {
                continue;
            }

            // don't build wonders?
            if !((focus_flags == 0) || (focus_flags & BUILDINGFOCUS_WONDEROK != 0) || (focus_flags & BUILDINGFOCUS_WORLDWONDER != 0))
                && (is_world_wonder_class(d, class)
                    || is_team_wonder_class(d, class)
                    || is_national_wonder_class(d, class)
                    || is_limited_wonder_class(d, class))
            {
                continue;
            }

            // PORT GAP: buildings' advisor types are not loaded; the eIgnoreAdvisor filter
            // never excludes a building (C++ AI callers always pass NO_ADVISOR).
            let _ = ignore_advisor;

            if !g.city_can_construct(c, loop_building, false, false, false) {
                continue;
            }

            let kb = &d.buildings[loop_building];
            let mut value = self.city_ai_building_value_threshold(g, c, loop_building, focus_flags, min_threshold);

            if let Some(free_class) = kb.free_building_class {
                if let Some(free_building) = d.civ_building(civ, free_class) {
                    value += self.city_ai_building_value(g, c, free_building, focus_flags)
                        * (num_cities - g.player_get_building_class_count_plus_making(p, free_class));
                }
            }

            if production_automated {
                if kb.prereq_num_of_building_class.iter().any(|&n| n > 0) {
                    value = 0;
                }
            }

            if value > 0 {
                let turns_left = g.city_production_turns_left_building(c, loop_building, 0);

                if is_world_wonder_class(d, class) {
                    if production_rank <= 3.min((num_cities + 2) / 3) {
                        let rand_max = d.leader_heads[g.city_personality_type(c)].wonder_construct_rand;
                        let mut temp_value = if async_ {
                            self.rand(rand_max, "Wonder Construction Rand ASYNC")
                        } else {
                            self.rand(rand_max, "Wonder Construction Rand")
                        };

                        if area_alone {
                            temp_value *= 2;
                        }
                        value += temp_value;
                    }
                }

                if async_ {
                    value *= self.rand(25, "AI Best Building ASYNC") + 100;
                    value /= 100;
                } else {
                    value *= self.rand(25, "AI Best Building") + 100;
                    value /= 100;
                }

                value += g.cities[c].building_production[loop_building];

                let mut valid = max_turns <= 0;
                if !valid {
                    let construct_percent = d.game_speeds[g.game_speed_type()].construct_percent;
                    valid = turns_left <= self.game_ai_turns_percent(g, max_turns, construct_percent);
                }
                if !valid {
                    for hurry in d.hurries.keys() {
                        if g.city_can_hurry_building(c, hurry, loop_building, true) {
                            if self.city_ai_get_happy_from_hurry_4(g, c, hurry, loop_building, true) > 0 {
                                valid = true;
                                break;
                            }
                        }
                    }
                }

                if valid {
                    // C++ asserts (MAX_INT / 1000) > iValue; wrap like the 32-bit int would.
                    value = value.wrapping_mul(1000);
                    value /= (turns_left + 3).max(1);

                    value = value.max(1);

                    if value > best_value {
                        best_value = value;
                        best_building = Some(loop_building);
                    }
                }
            }
        }

        best_building
    }

    /// C++ `int CvCityAI::AI_buildingValue(BuildingTypes eBuilding, int iFocusFlags = 0)` (CvCityAI.cpp:2931-2934)
    pub fn city_ai_building_value(&self, g: &Game, c: CityId, building: BuildingType, focus_flags: i32) -> i32 {
        self.city_ai_building_value_threshold(g, c, building, focus_flags, 0)
    }

    /// C++ `int CvCityAI::AI_buildingValueThreshold(BuildingTypes eBuilding, int iFocusFlags = 0, int iThreshold = 0)` (CvCityAI.cpp:2938-4384)
    pub fn city_ai_building_value_threshold(&self, g: &Game, c: CityId, building: BuildingType, focus_flags: i32, threshold: i32) -> i32 {
        let d = g.d;
        let p = g.cities[c].owner;
        let t = g.city_team(c);
        let civ = g.city_civilization_type(c);
        let area = g.city_area(c);
        let kb = &d.buildings[building];
        let Some(building_class) = kb.building_class_type else {
            // PORT ADAPT: C++ assumes every building has a class.
            return 0;
        };
        let limited_wonder_limit = limited_wonder_class_limit(d, building_class);
        let is_limited_wonder = limited_wonder_limit >= 0;

        let state_religion = g.player_get_state_religion(p);

        let area_alone = self.player_ai_is_area_alone(g, p, area);
        let provides_power = kb.power || kb.power_bonus.map_or(false, |b| g.city_has_bonus(c, b));

        let has_met_count = g.team_get_has_met_civ_count(t, true);
        let food_difference = g.city_food_difference(c, false);

        let population = g.cities[c].population;
        let happiness_level = g.city_happy_level(c) - g.city_unhappy_level(c, 1);
        let angry_population = range(-happiness_level, 0, population + 1);
        let health_level = g.city_good_health(c) - g.city_bad_health(c, false, 0.max((happiness_level + 1) / 2));
        let bad_health = 0.max(-health_level);

        let mut happy_modifier = if happiness_level >= health_level && happiness_level <= 6 { 6 } else { 3 };
        let mut health_modifier = if health_level > happiness_level && health_level <= 4 { 4 } else { 2 };
        if happiness_level >= 10 {
            happy_modifier = 1;
        }
        // C++ quirk: health_modifier is at most 4 here, so this never fires.
        if health_modifier >= 8 {
            health_modifier = 0;
        }

        let total_population = g.players[p].total_population;
        let num_cities = g.players[p].num_cities();
        let num_cities_in_area = g.map.areas[area].cities_per_player[p];

        let mut yield_rank = [MAX_INT; YieldType::NUM];
        let mut commerce_rank = [MAX_INT; CommerceType::NUM];

        yield_rank[Y_PROD] = g.city_find_base_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);
        let is_high_production_city = yield_rank[Y_PROD] <= 3.max(num_cities / 2);

        let culture_rank = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
        let cultural_victory_num_culture_cities = g.num_culture_victory_cities;

        let financial_trouble = self.player_ai_is_financial_trouble(g, p);

        let cultural_victory1 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1);
        let cultural_victory2 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2);
        let cultural_victory3 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3);

        let can_pop_rush = g.player_can_pop_rush(p);

        let mut foreign_trade = false;
        let num_trade_routes = g.city_trade_routes(c);
        for i in 0..num_trade_routes.max(0) as usize {
            if let Some(trade_city) = g.city_trade_city(c, i) {
                if g.players[g.cities[trade_city].owner].team != t || g.city_area(trade_city) != area {
                    foreign_trade = true;
                    break;
                }
            }
        }

        if kb.capital {
            return 0;
        }

        for (r, &change) in kb.religion_change.iter_enumerated() {
            if change > 0 {
                if !g.team_has_holy_city(t, r) {
                    return 0;
                }
            }
        }

        let is_power = g.city_is_power(c);
        let base_yield_rate = g.cities[c].base_yield_rate;
        let trade_yield = g.cities[c].trade_yield;
        let commerce_percent = g.players[p].commerce_percent;
        // FIX: C++ divides by the owner's city count; guard against 0 (never in practice).
        let num_cities_div = num_cities.max(1);

        let mut value = 0;

        for pass in 0..2 {
            if !((focus_flags == 0) || (value > 0) || (pass == 0)) {
                continue;
            }

            if (focus_flags & BUILDINGFOCUS_WORLDWONDER != 0) || (pass > 0) {
                if is_world_wonder_class(d, building_class) {
                    if yield_rank[Y_PROD] <= 3 {
                        value += 1;
                    }

                    // FfH: Added by Kael 07/15/2008
                    if Some(building) == d.leader_heads[g.city_personality_type(c)].favorite_wonder {
                        value += 8;
                    }
                }
            }

            if (focus_flags & BUILDINGFOCUS_DEFENSE != 0) || (pass > 0) {
                if !area_alone {
                    if g.best_land_unit.map_or(true, |u| !d.units[u].ignore_building_defense) {
                        value += 0.max(((kb.defense_modifier + g.cities[c].building_defense) - g.city_natural_defense(c) - 10).min(kb.defense_modifier)) / 4;
                    }
                }

                value += kb.bombard_defense_modifier / 8;

                value += -kb.air_modifier / 4;
                value += -kb.nuke_modifier / 4;

                value += (kb.all_city_defense_modifier * num_cities) / 5;

                value += kb.airlift * 25;
            }

            if (focus_flags & BUILDINGFOCUS_ESPIONAGE != 0) || (pass > 0) {
                value += kb.espionage_defense_modifier / 8;
            }

            if ((focus_flags & BUILDINGFOCUS_HAPPY != 0) || (pass > 0)) && !g.city_is_no_unhappiness(c) {
                let mut best_happy = 0;
                for hurry in d.hurries.keys() {
                    if g.city_can_hurry_building(c, hurry, building, true) {
                        let happy_from_hurry = self.city_ai_get_happy_from_hurry_4(g, c, hurry, building, true);
                        if happy_from_hurry > best_happy {
                            best_happy = happy_from_hurry;
                        }
                    }
                }
                value += best_happy * 10;

                if kb.no_unhappiness {
                    value += (angry_population * 10) + population;
                }

                let building_happiness = kb.happiness;
                if building_happiness != 0 {
                    value += (building_happiness.min(angry_population) * 10) + (0.max(building_happiness - angry_population) * happy_modifier);
                }

                value += kb.area_happiness * num_cities_in_area * 8;
                value += kb.global_happiness * num_cities * 8;

                let war_weariness_percent_anger = g.player_get_war_weariness_percent_anger(p);
                let war_weariness_modifier = kb.war_weariness_modifier;
                if war_weariness_modifier > 0 {
                    value += (-(((war_weariness_modifier * war_weariness_percent_anger) / 100) / d.defines.percent_anger_divisor)).min(angry_population) * 8;
                    value += (-war_weariness_modifier * happy_modifier) / 16;
                }

                let global_war_weariness_modifier = kb.global_war_weariness_modifier;
                if global_war_weariness_modifier > 0 {
                    value += (-((global_war_weariness_modifier * war_weariness_percent_anger / 100) / d.defines.percent_anger_divisor)) * num_cities;
                    value += (-global_war_weariness_modifier * happy_modifier) / 16;
                }

                value += (-kb.hurry_anger_modifier * g.city_hurry_percent_anger(c, 0)) / 100;

                let state_religion_happiness = kb.state_religion_happiness;
                if kb.religion_type == state_religion && state_religion_happiness != 0 {
                    value += (state_religion_happiness.min(angry_population) * 8) + (0.max(state_religion_happiness - angry_population) * happy_modifier);
                }

                for i in 0..CommerceType::NUM {
                    value += ((kb.commerce_happiness[i] * commerce_percent[i]) / 100).min(angry_population) * 8;
                    value += (kb.commerce_happiness[i] * happy_modifier) / 4;
                }

                for (bonus, &bonus_happiness_change) in kb.bonus_happiness_changes.iter_enumerated() {
                    if g.city_has_bonus(c, bonus) {
                        value += (bonus_happiness_change.min(angry_population) * 8) + (0.max(bonus_happiness_change - angry_population) * happy_modifier);
                    }
                }

                for (bc, &change) in kb.building_happiness_changes.iter_enumerated() {
                    if change != 0 {
                        value += change * g.players[p].building_class_count[bc] * 8;
                    }
                }
            }

            // Skyre Mod, BETTER AI (isIgnoreFood) merged Sephi
            if ((focus_flags & BUILDINGFOCUS_HEALTHY != 0) || (pass > 0)) && !g.city_is_no_unhealthy_population(c) && !g.players[p].ignore_food {
                if provides_power {
                    if g.city_is_dirty_power(c) && !kb.dirty_power {
                        value += (-d.defines.dirty_power_health_change).min(bad_health) * 8;
                    }
                }

                if kb.no_unhealthy_population {
                    let unhealthy_population = g.city_unhealthy_population(c, false, 0);
                    value += (unhealthy_population.min(bad_health) * 12) + (0.max(unhealthy_population - bad_health) * health_modifier);
                }

                if kb.building_only_healthy {
                    let building_bad_health = -g.cities[c].building_bad_health;
                    value += (building_bad_health.min(bad_health) * 12) + ((0.max(building_bad_health - bad_health) + 1) * health_modifier);
                }

                let building_health = kb.health;
                if building_health != 0 {
                    value += (building_health.min(bad_health) * 12) + (0.max(building_health - bad_health) * health_modifier);
                }

                value += kb.area_health * num_cities_in_area * 4;
                value += kb.global_health * num_cities * 4;

                for (bonus, &bonus_health_change) in kb.bonus_health_changes.iter_enumerated() {
                    if g.city_has_bonus(c, bonus) {
                        value += (bonus_health_change.min(bad_health) * 12) + (0.max(bonus_health_change - bad_health) * health_modifier);
                    }
                }
            }

            if (focus_flags & BUILDINGFOCUS_EXPERIENCE != 0) || (pass > 0) {
                value += kb.free_experience * if has_met_count > 0 { 12 } else { 6 };

                for unit_combat in d.unit_combats.keys() {
                    if g.city_can_train_combat(c, unit_combat) {
                        value += kb.unit_combat_free_experience[unit_combat] * if has_met_count > 0 { 6 } else { 3 };
                    }
                }

                for (domain, &exp) in kb.domain_free_experience.iter_enumerated() {
                    let domain_exp_value = if domain == DomainType::DOMAIN_SEA {
                        7
                    } else if domain == DomainType::DOMAIN_LAND {
                        12
                    } else {
                        6
                    };
                    value += exp * if has_met_count > 0 { domain_exp_value } else { domain_exp_value / 2 };
                }
            }

            // since this duplicates BUILDINGFOCUS_EXPERIENCE checks, do not repeat on pass 1
            if focus_flags & BUILDINGFOCUS_DOMAINSEA != 0 {
                value += kb.free_experience * if has_met_count > 0 { 16 } else { 8 };

                for unit_class in d.unit_classes.keys() {
                    if let Some(unit) = d.civ_unit(civ, unit_class) {
                        let ui = &d.units[unit];
                        if ui.domain_type == Some(DomainType::DOMAIN_SEA) && g.city_can_train(c, unit, false, false, false, false) {
                            if let Some(combat_type) = ui.unit_combat_type {
                                value += kb.unit_combat_free_experience[combat_type] * if has_met_count > 0 { 6 } else { 3 };
                            }
                        }
                    }
                }

                value += kb.domain_free_experience[DomainType::DOMAIN_SEA] * if has_met_count > 0 { 16 } else { 8 };

                value += kb.domain_production_modifier[DomainType::DOMAIN_SEA] / 4;
            }

            if (focus_flags & BUILDINGFOCUS_MAINTENANCE != 0) || (focus_flags & BUILDINGFOCUS_GOLD != 0) || (pass > 0) {
                let base_maintenance = g.cities[c].maintenance;
                let maintenance_modifier = g.cities[c].maintenance_modifier;
                let existing_upkeep = (base_maintenance * 0.max(100 + maintenance_modifier)) / 100;
                let new_upkeep = (base_maintenance * 0.max(100 + maintenance_modifier + kb.maintenance_modifier)) / 100;

                let mut temp_value = (existing_upkeep - new_upkeep) / 16;

                if financial_trouble {
                    temp_value *= 2;
                }

                value += temp_value;
            }

            if (focus_flags & BUILDINGFOCUS_SPECIALIST != 0) || (pass > 0) {
                let mut specialists_value = 0;
                let mut current_specialists_runnable = 0;
                for s in d.specialists.keys() {
                    if s.i() != d.defines.default_specialist {
                        let unlimited = g.player_is_specialist_valid(p, s);
                        let runnable = (g.cities[c].max_specialist_count[s] > 0) as i32;

                        if unlimited || (runnable > 0) {
                            if unlimited {
                                current_specialists_runnable += 5;
                            } else {
                                current_specialists_runnable += runnable;
                            }
                        }

                        if kb.specialist_count[s] > 0 {
                            if !unlimited && (runnable < 5) {
                                let mut temp_value = self.city_ai_specialist_value(g, c, s, false, false);

                                temp_value *= 20 + (40 * kb.specialist_count[s]);
                                temp_value /= 100;

                                // C++ quirk: these divide the running building value, not the
                                // specialist value.
                                if food_difference < 2 {
                                    value /= 4;
                                }
                                if runnable > 0 {
                                    value /= 1 + runnable;
                                }

                                specialists_value += 12.max(temp_value / 100);
                            }
                        }
                    }
                }

                if specialists_value > 0 {
                    value += specialists_value / 2.max(current_specialists_runnable);
                }
            }

            if (focus_flags & (BUILDINGFOCUS_GOLD | BUILDINGFOCUS_RESEARCH) != 0) || pass > 0 {
                // trade routes
                let mut temp_value = (kb.trade_routes * ((8 * 0.max(g.city_total_trade_modifier(c, None) + 100)) / 100)) * (population / 5 + 1);
                let global_trade_value = ((6 * total_population) / 5) / num_cities_div;
                temp_value += kb.coastal_trade_routes * g.player_count_num_coastal_cities(p) * global_trade_value;
                temp_value += kb.global_trade_routes * num_cities * global_trade_value;

                temp_value += (kb.trade_route_modifier * trade_yield[Y_COMM]) / if foreign_trade { 12 } else { 25 };
                if foreign_trade {
                    temp_value += (kb.foreign_trade_route_modifier * trade_yield[Y_COMM]) / 12;
                }

                if financial_trouble {
                    temp_value *= 2;
                }

                if g.player_is_no_foreign_trade(p) {
                    temp_value /= 3;
                }

                value += temp_value;
            }

            if pass > 0 {
                if kb.area_clean_power {
                    value += num_cities_in_area * 10; // XXX count cities without clean power???
                }

                if kb.domestic_great_general_rate_modifier != 0 {
                    value += kb.domestic_great_general_rate_modifier / 10;
                }

                if kb.area_border_obstacle {
                    value += num_cities_in_area;
                }

                if kb.government_center {
                    value += (g.city_calculate_distance_maintenance(c) - 3) * num_cities_in_area;
                }

                if kb.map_centering {
                    value += 1;
                }

                if let Some(free_bonus) = kb.free_bonus {
                    value += self.player_ai_bonus_val(g, p, free_bonus, 1)
                        * if g.player_get_num_tradeable_bonuses(p, free_bonus) == 0 { 2 } else { 1 }
                        * (num_cities + kb.num_free_bonuses);
                }

                if let Some(no_bonus) = kb.no_bonus {
                    value -= self.player_ai_bonus_val(g, p, no_bonus, 1);
                }

                if kb.free_promotion.is_some() {
                    value += if has_met_count > 0 { 100 } else { 40 }; // XXX some sort of promotion value???
                }

                if let Some(civic_option) = kb.civic_option {
                    for civic in d.civics.keys() {
                        if d.civics[civic].civic_option_type == Some(civic_option) {
                            if !g.player_can_do_civics(p, Some(civic)) {
                                value += self.player_ai_civic_value(g, p, civic) / 10;
                            }
                        }
                    }
                }

                let great_people_rate_modifier = kb.great_people_rate_modifier;
                if great_people_rate_modifier > 0 {
                    let great_people_rate = g.cities[c].base_great_people_rate;
                    const TARGET_GP_RATE: i32 = 10;

                    // either not a wonder, or a wonder and our GP rate is at least the target rate
                    if !is_limited_wonder || great_people_rate >= TARGET_GP_RATE {
                        value += (great_people_rate_modifier * great_people_rate) / 16;
                    }
                    // otherwise, this is a limited wonder (aka National Epic), we _really_ do not want to build this here
                    // subtract from the value (if this wonder has a lot of other stuff, we still might build it)
                    else {
                        value -= (great_people_rate_modifier * (TARGET_GP_RATE - great_people_rate)) / 12;
                    }
                }

                value += (kb.global_great_people_rate_modifier * num_cities) / 8;

                value += -kb.anarchy_modifier / 4;

                value += -kb.global_hurry_modifier * 2;

                value += kb.global_free_experience * num_cities * if has_met_count > 0 { 6 } else { 3 };

                if can_pop_rush {
                    value += kb.food_kept / 2;
                }

                value += kb.airlift * (population * 3 + 10);

                let air_defense = -kb.air_modifier;
                if air_defense > 0 {
                    if ((self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_DEFENSE_AIR) > 0)
                        && (self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_ATTACK_AIR) > 0))
                        || (self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_MISSILE_AIR) > 0)
                    {
                        value += air_defense / if has_met_count > 0 { 2 } else { 4 };
                    }
                }

                value += kb.air_unit_capacity * (population * 2 + 10);

                value += -kb.nuke_modifier / if has_met_count > 0 { 10 } else { 20 };

                value += kb.free_specialist * 16;
                value += kb.area_free_specialist * num_cities_in_area * 12;
                value += kb.global_free_specialist * num_cities * 12;

                value += (kb.worker_speed_modifier * self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_WORKER)) / 10;

                let military_production_modifier = kb.military_production_modifier;
                if has_met_count > 0 && military_production_modifier > 0 {
                    // either not a wonder, or a wonder and we are a high production city
                    if !is_limited_wonder || is_high_production_city {
                        value += military_production_modifier / 4;

                        // if a wonder, then pick one of the best cities
                        if is_limited_wonder {
                            // if one of the top 3 production cities, give a big boost
                            if yield_rank[Y_PROD] <= (2 + limited_wonder_limit) {
                                value += (2 * military_production_modifier) / (2 + yield_rank[Y_PROD]);
                            }
                        }
                        // otherwise, any of the top half of cities will do
                        else if is_high_production_city {
                            value += military_production_modifier / 4;
                        }
                        value += (military_production_modifier * (g.cities[c].free_experience + g.cities[c].specialist_free_experience)) / 10;
                    }
                    // otherwise, this is a limited wonder (aka Heroic Epic), we _really_ do not want to build this here
                    // subtract from the value (if this wonder has a lot of other stuff, we still might build it)
                    else {
                        value -= (military_production_modifier * yield_rank[Y_PROD]) / 5;
                    }
                }

                value += kb.space_production_modifier / 5;
                value += (kb.global_space_production_modifier * num_cities) / 20;

                if kb.great_people_unit_class.is_some() {
                    value += 1; // XXX improve this for diversity...
                }

                // prefer to build great people buildings in places that already have some GP points
                value += (kb.great_people_rate_change * 10) * (1 + (g.cities[c].base_great_people_rate / 2));

                if !area_alone {
                    value += kb.heal_rate_change / 2;
                }

                value += kb.global_population_change * num_cities * 4;

                value += kb.free_techs * 80;

                value += kb.enemy_war_weariness_modifier / 2;

                for (s, &count) in kb.free_specialist_count.iter_enumerated() {
                    if count > 0 {
                        value += (self.city_ai_specialist_value(g, c, s, false, false) * count) / 50;
                    }
                }

                for (imp, &count) in kb.improvement_free_specialist.iter_enumerated() {
                    if count > 0 {
                        value += count * g.city_count_num_improved_plots(c, Some(imp), true) * 50;
                    }
                }

                for &modifier in kb.domain_production_modifier.iter() {
                    value += modifier / 5;

                    if is_high_production_city {
                        value += modifier / 5;
                    }
                }

                // FfH: Modified by Kael 10/30/2007 (units this building allows);
                // Xienwolf Tweak (BETTER AI, merged Sephi): barbarians do not value it.
                if !(g.is_option(GameOptionType::GAMEOPTION_AI_NO_BUILDING_PREREQS) || g.city_is_barbarian(c)) {
                    for unit_class in d.unit_classes.keys() {
                        let Some(loop_unit) = d.civ_unit(civ, unit_class) else {
                            continue;
                        };
                        let ui = &d.units[loop_unit];
                        if ui.prereq_building == Some(building) || ui.prereq_building_class == kb.building_class_type {
                            // PORT ADAPT: a unit without a default UnitAI counts as none in the area.
                            let area_unit_ais = ui.default_unit_ai_type.map_or(0, |ai| self.player_ai_total_area_unit_ais(g, p, area, ai));
                            if area_unit_ais == 0 {
                                value += num_cities_in_area * 5;
                            }
                            let combat_value = self.player_ai_combat_value(g, p, loop_unit);
                            value += combat_value * 10;
                            if g.players[p].building_class_count[building_class] < 4 {
                                value += combat_value * 40;
                            } else {
                                value += combat_value * 10;
                            }
                            if let Some(religion) = ui.prereq_religion {
                                if cultural_victory1 || g.city_is_holy_city(c, religion) || g.city_is_capital(c) {
                                    value += 2 + num_cities_in_area;
                                }
                                if cultural_victory2 && ui.religion_spreads[religion] != 0 {
                                    let religion_count = g.players[p].has_religion_count[religion];
                                    value += (100 * (num_cities - religion_count)) / (num_cities_div * (religion_count + 1));
                                }
                            }
                        }
                    }
                }

                // is this building needed to build other buildings?
                for loop_building in d.buildings.keys() {
                    let prereq_buildings = g.player_get_building_class_prereq_building(p, loop_building, building_class, 0);

                    // if we need some of us to build iI building, and we dont need more than we have cities
                    if prereq_buildings > 0 && prereq_buildings <= num_cities {
                        // do we need more than what we are currently building?
                        if prereq_buildings > g.player_get_building_class_count_plus_making(p, building_class) {
                            value += num_cities * 3;

                            if cultural_victory1 {
                                let klb = &d.buildings[loop_building];
                                let loop_building_culture_modifier = klb.commerce_modifier[C_CULTURE];
                                if loop_building_culture_modifier > 0 {
                                    let loop_buildings_built = klb.building_class_type.map_or(0, |bc| g.players[p].building_class_count[bc]);

                                    // if we have less than the number needed in culture cities
                                    //		OR we are one of the top cities and we do not have the building
                                    if loop_buildings_built < cultural_victory_num_culture_cities
                                        || (culture_rank <= cultural_victory_num_culture_cities && 0 == g.city_num_building(c, loop_building))
                                    {
                                        value += loop_building_culture_modifier;

                                        if cultural_victory3 {
                                            value += loop_building_culture_modifier * 2;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if kb.vote_source_type.is_some() {
                    if kb.is_state_religion {
                        let mut share_religion_count = 0;
                        let mut player_count = 0;
                        let owner_state_religion = g.player_get_state_religion(p);
                        for loop_player in g.player_ids() {
                            if loop_player != p && g.players[loop_player].alive {
                                player_count += 1;
                                if owner_state_religion == g.player_get_state_religion(loop_player) {
                                    share_religion_count += 1;
                                }
                            }
                        }
                        value += (200 * (1 + share_religion_count)) / (1 + player_count);
                    } else {
                        value += 100;
                    }
                }
            }

            if pass > 0 {
                for i in 0..YieldType::NUM {
                    let y = YieldType::from_usize(i);
                    let mut temp_value = 0;

                    value += (kb.trade_route_modifier * trade_yield[i]) / 12;
                    if foreign_trade {
                        value += (kb.foreign_trade_route_modifier * trade_yield[i]) / 12;
                    }

                    if food_difference > 0 {
                        value += kb.food_kept / 2;
                    }

                    if kb.sea_plot_yield_change[i] > 0 {
                        temp_value += kb.sea_plot_yield_change[i] * self.city_ai_building_special_yield_change_value(g, c, building, y);
                    }
                    if kb.river_plot_yield_change[i] > 0 {
                        temp_value += kb.river_plot_yield_change[i] * g.city_count_num_river_plots(c) * 4;
                    }
                    temp_value += kb.global_sea_plot_yield_change[i] * g.player_count_num_coastal_cities(p) * 8;
                    temp_value += kb.yield_change[i] * 6;
                    temp_value += (kb.yield_modifier[i] * base_yield_rate[i]) / 10;
                    temp_value += (kb.power_yield_modifier[i] * base_yield_rate[i]) / if provides_power || is_power { 12 } else { 15 };
                    temp_value += (kb.area_yield_modifier[i] * num_cities_in_area) / 3;
                    temp_value += (kb.global_yield_modifier[i] * num_cities) / 3;

                    if provides_power && !is_power {
                        temp_value += (g.cities[c].power_yield_rate_modifier[i] * base_yield_rate[i]) / 12;
                    }

                    for changes in kb.specialist_yield_change.iter() {
                        temp_value += (changes[i] * total_population) / 5;
                    }

                    for (bonus, modifiers) in kb.bonus_yield_modifier.iter_enumerated() {
                        if g.city_has_bonus(c, bonus) {
                            temp_value += (modifiers[i] * base_yield_rate[i]) / 12;
                        }
                    }

                    if temp_value != 0 {
                        if financial_trouble && i == Y_COMM {
                            temp_value *= 2;
                        }

                        temp_value *= self.player_ai_yield_weight(g, p, y);
                        temp_value /= 100;

                        if yield_rank[i] == MAX_INT {
                            yield_rank[i] = g.city_find_base_yield_rate_rank(c, y);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                        // we do _not_ want to build this here (unless the value was small anyway)
                        if is_limited_wonder && (yield_rank[i] > (3 + limited_wonder_limit)) {
                            temp_value *= -1;
                        }

                        value += temp_value;
                    }
                }
            } else {
                if focus_flags & BUILDINGFOCUS_FOOD != 0 {
                    value += kb.food_kept;

                    if kb.sea_plot_yield_change[Y_FOOD] > 0 {
                        let temp_value = kb.sea_plot_yield_change[Y_FOOD] * self.city_ai_building_special_yield_change_value(g, c, building, YieldType::YIELD_FOOD);
                        if (temp_value < 8) && (population > 3) {
                            // don't bother
                        } else {
                            value += (temp_value * 4) / 2.max(food_difference);
                        }
                    }

                    if kb.river_plot_yield_change[Y_FOOD] > 0 {
                        value += kb.river_plot_yield_change[Y_FOOD] * g.city_count_num_river_plots(c) * 4;
                    }
                }

                if focus_flags & BUILDINGFOCUS_PRODUCTION != 0 {
                    let mut temp_value = (kb.yield_modifier[Y_PROD] * base_yield_rate[Y_PROD]) / 20;
                    temp_value += (kb.power_yield_modifier[Y_PROD] * base_yield_rate[Y_PROD]) / if provides_power || is_power { 24 } else { 30 };
                    if kb.sea_plot_yield_change[Y_PROD] > 0 {
                        let num_water_plots = g.city_count_num_water_plots(c);
                        if !is_limited_wonder || (num_water_plots > (NUM_CITY_PLOTS / 2) as i32) {
                            temp_value += kb.sea_plot_yield_change[Y_PROD] * num_water_plots;
                        }
                    }
                    if kb.river_plot_yield_change[Y_PROD] > 0 {
                        temp_value += kb.river_plot_yield_change[Y_PROD] * g.city_count_num_river_plots(c) * 4;
                    }
                    if provides_power && !is_power {
                        temp_value += (g.cities[c].power_yield_rate_modifier[Y_PROD] * base_yield_rate[Y_PROD]) / 12;
                    }

                    // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                    // we do _not_ want to build this here (unless the value was small anyway)
                    if is_limited_wonder && (yield_rank[Y_PROD] > (3 + limited_wonder_limit)) {
                        temp_value *= -1;
                    }

                    value += temp_value;
                }

                if focus_flags & BUILDINGFOCUS_GOLD != 0 {
                    let mut temp_value = kb.yield_modifier[Y_COMM] * base_yield_rate[Y_COMM];
                    temp_value *= commerce_percent[C_GOLD];

                    if financial_trouble {
                        temp_value *= 2;
                    }

                    temp_value /= 3000;

                    if MAX_INT == commerce_rank[C_GOLD] {
                        commerce_rank[C_GOLD] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_GOLD);
                    }

                    // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                    // we do _not_ want to build this here (unless the value was small anyway)
                    if is_limited_wonder && (commerce_rank[C_GOLD] > (3 + limited_wonder_limit)) {
                        temp_value *= -1;
                    }

                    value += temp_value;
                }
            }

            if pass > 0 {
                for i in 0..CommerceType::NUM {
                    let ct = CommerceType::from_usize(i);
                    let mut temp_value = 0;

                    temp_value += kb.commerce_change[i] * 4;
                    temp_value += kb.obsolete_safe_commerce_change[i] * 4;
                    temp_value *= 100 + kb.commerce_modifier[i];
                    temp_value /= 100;

                    if i == C_CULTURE {
                        if cultural_victory1 {
                            temp_value *= 2;
                        }
                    }

                    if kb.commerce_change_double_time[i] > 0 {
                        if (kb.commerce_change[i] > 0) || (kb.obsolete_safe_commerce_change[i] > 0) {
                            temp_value += 1000 / kb.commerce_change_double_time[i];
                        }
                    }

                    // add value for a commerce modifier
                    let commerce_modifier = kb.commerce_modifier[i];
                    let base_commerce_rate = g.city_base_commerce_rate(c, ct);
                    let mut commerce_multiplier_value = commerce_modifier * base_commerce_rate;
                    if i == C_CULTURE && commerce_modifier != 0 {
                        if cultural_victory1 {
                            // if this is one of our top culture cities, then we want to build this here first!
                            if culture_rank <= cultural_victory_num_culture_cities {
                                commerce_multiplier_value /= 8;

                                // if we at culture level 3, then these need to get built asap
                                if cultural_victory3 {
                                    // its most important to build in the lowest rate city, but important everywhere
                                    commerce_multiplier_value += 100.max(500 - base_commerce_rate) * commerce_modifier;
                                }
                            } else {
                                let count_built = g.player_get_building_class_count_plus_making(p, building_class);

                                // do we have enough buildings to build extras?
                                let mut have_enough = true;

                                // if its limited and the limit is less than the number we need in culture cities, do not build here
                                if is_limited_wonder && (limited_wonder_limit <= cultural_victory_num_culture_cities) {
                                    have_enough = false;
                                }

                                for bc in d.building_classes.keys() {
                                    if !have_enough {
                                        break;
                                    }
                                    // count excess the number of prereq buildings which do not have this building built for yet
                                    let prereq_buildings = g.player_get_building_class_prereq_building(p, building, bc, -count_built);

                                    // do we not have enough built (do not count ones in progress)
                                    if prereq_buildings > 0 && g.players[p].building_class_count[bc] < prereq_buildings {
                                        have_enough = false;
                                    }
                                }

                                // if we have enough and our rank is close to the top, then possibly build here too
                                if have_enough && (culture_rank - cultural_victory_num_culture_cities) <= 3 {
                                    commerce_multiplier_value /= 12;
                                }
                                // otherwise, we really do not want to build this here
                                else {
                                    commerce_multiplier_value /= 30;
                                }
                            }
                        } else {
                            commerce_multiplier_value /= 15;

                            // increase priority if we need culture oppressed city
                            commerce_multiplier_value *= 100 - g.city_calculate_culture_percent(c, p);
                        }
                    } else {
                        commerce_multiplier_value /= 15;
                    }
                    temp_value += commerce_multiplier_value;

                    temp_value += (kb.global_commerce_modifier[i] * num_cities) / 4;
                    temp_value += (kb.specialist_extra_commerce[i] * total_population) / 3;

                    if let Some(sr) = state_religion {
                        temp_value += kb.state_religion_commerce[i] * g.players[p].has_religion_count[sr] * 3;
                    }

                    if let Some(grc) = kb.global_religion_commerce {
                        temp_value += d.religions[grc].global_religion_commerce[i] * g.count_religion_levels(grc) * 2;
                        if state_religion == Some(grc) {
                            temp_value += 10;
                        }
                    }

                    let mut corp_value = 0;
                    if let Some(corporation) = kb.founds_corporation {
                        corp_value = self.player_ai_corporation_value(g, p, corporation, Some(c));

                        for corp in d.corporations.keys() {
                            if corp != corporation {
                                if g.player_has_headquarters(p, corp) {
                                    if g.is_competing_corporation(corporation, corp) {
                                        if self.player_ai_corporation_value(g, p, corp, Some(c)) > corp_value {
                                            corp_value = -1;
                                            break;
                                        } else if !g.cities[c].has_corporation[corp] {
                                            corp_value = -1;
                                        }
                                    }
                                }
                            }
                        }

                        temp_value += corp_value / 100;
                    }

                    if corp_value >= 0 {
                        // Don't build if it'll hurt us.
                        if let Some(gcc) = kb.global_corporation_commerce {
                            let hq_commerce = d.corporations[gcc].headquarter_commerce[i];
                            let mut gold_value = hq_commerce * g.count_corporation_levels(gcc) * 2;

                            gold_value += hq_commerce;
                            if gold_value > 0 {
                                gold_value += 2 + (num_cities / 4);
                                gold_value += gold_value.min(g.cities[c].building_commerce[C_GOLD] / 2) / 2;
                            }
                            let total_gold_modifier = g.city_total_commerce_rate_modifier(c, CommerceType::COMMERCE_GOLD);
                            gold_value *= 2;
                            gold_value *= total_gold_modifier;
                            gold_value *= 50.max(total_gold_modifier - 150);
                            gold_value /= 5000;
                            corp_value += gold_value;
                        }
                    }

                    // C++ quirk: the corporation value is added once per commerce type.
                    if corp_value > 0 {
                        if g.player_is_no_corporations(p) {
                            corp_value /= 2;
                        }
                        value += corp_value;
                    }

                    if kb.commerce_flexible[i] {
                        if !g.player_is_commerce_flexible(p, ct) {
                            temp_value += 40;
                        }
                    }

                    if kb.commerce_change_original_owner[i] {
                        if (kb.commerce_change[i] > 0) || (kb.obsolete_safe_commerce_change[i] > 0) {
                            temp_value += 1;
                        }
                    }

                    if temp_value != 0 {
                        if financial_trouble && i == C_GOLD {
                            temp_value *= 2;
                        }

                        temp_value *= self.player_ai_commerce_weight(g, p, ct, Some(c));
                        temp_value = (temp_value + 99) / 100;

                        // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                        // we do _not_ want to build this here (unless the value was small anyway)
                        if MAX_INT == commerce_rank[i] {
                            commerce_rank[i] = g.city_find_commerce_rate_rank(c, ct);
                        }
                        if (is_limited_wonder && (commerce_rank[i] > (3 + limited_wonder_limit)))
                            || (cultural_victory1 && (i == C_CULTURE) && (commerce_rank[i] == 1))
                        {
                            temp_value *= -1;

                            // for culture, just set it to zero, not negative, just about every wonder gives culture
                            if i == C_CULTURE {
                                temp_value = 0;
                            }
                        }
                        value += temp_value;
                    }
                }

                for (r, &change) in kb.religion_change.iter_enumerated() {
                    if change > 0 {
                        if g.team_has_holy_city(t, r) {
                            value += change * if state_religion == Some(r) { 10 } else { 1 };
                        }
                    }
                }

                if kb.vote_source_type.is_some() {
                    value += 100;
                }
            } else {
                let no_espionage = g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE);

                if focus_flags & BUILDINGFOCUS_GOLD != 0 {
                    let mut temp_value = (kb.commerce_modifier[C_GOLD] * g.city_base_commerce_rate(c, CommerceType::COMMERCE_GOLD)) / 40;

                    if temp_value != 0 {
                        if financial_trouble {
                            temp_value *= 2;
                        }

                        if MAX_INT == commerce_rank[C_GOLD] {
                            commerce_rank[C_GOLD] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_GOLD);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                        // we do _not_ want to build this here (unless the value was small anyway)
                        if is_limited_wonder && (commerce_rank[C_GOLD] > (3 + limited_wonder_limit)) {
                            temp_value *= -1;
                        }
                        value += temp_value;
                    }

                    value += kb.commerce_change[C_GOLD] * 4;
                    value += kb.obsolete_safe_commerce_change[C_GOLD] * 4;
                }

                if focus_flags & BUILDINGFOCUS_RESEARCH != 0 {
                    let mut temp_value = (kb.commerce_modifier[C_RESEARCH] * g.city_base_commerce_rate(c, CommerceType::COMMERCE_RESEARCH)) / 40;

                    if temp_value != 0 {
                        if MAX_INT == commerce_rank[C_RESEARCH] {
                            commerce_rank[C_RESEARCH] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_RESEARCH);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                        // we do _not_ want to build this here (unless the value was small anyway)
                        if is_limited_wonder && (commerce_rank[C_RESEARCH] > (3 + limited_wonder_limit)) {
                            temp_value *= -1;
                        }

                        value += temp_value;
                    }
                    value += kb.commerce_change[C_RESEARCH] * 4;
                    value += kb.obsolete_safe_commerce_change[C_RESEARCH] * 4;
                }

                if focus_flags & BUILDINGFOCUS_CULTURE != 0 {
                    let mut temp_value = kb.commerce_change[C_CULTURE] * 3;
                    temp_value += kb.obsolete_safe_commerce_change[C_CULTURE] * 3;
                    if no_espionage {
                        temp_value += kb.commerce_change[C_ESPIONAGE] * 3;
                        temp_value += kb.obsolete_safe_commerce_change[C_ESPIONAGE] * 3;
                    }

                    if (g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) == 0) && (self.city_ai_calculate_target_culture_per_turn(g, c) == 1) {
                        if temp_value >= 3 {
                            temp_value += 7;
                        }
                    }

                    if temp_value != 0 {
                        if MAX_INT == commerce_rank[C_CULTURE] {
                            commerce_rank[C_CULTURE] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category,
                        // do not count the culture value
                        // we probably do not want to build this here (but we might)
                        if is_limited_wonder && (commerce_rank[C_CULTURE] > (3 + limited_wonder_limit)) {
                            temp_value = 0;
                        }
                        value += temp_value;
                    }

                    value += (kb.commerce_modifier[C_CULTURE] * g.city_base_commerce_rate(c, CommerceType::COMMERCE_CULTURE)) / 15;
                    if no_espionage {
                        value += (kb.commerce_modifier[C_ESPIONAGE] * g.city_base_commerce_rate(c, CommerceType::COMMERCE_ESPIONAGE)) / 15;
                    }
                }

                if focus_flags & BUILDINGFOCUS_BIGCULTURE != 0 {
                    let mut temp_value = kb.commerce_modifier[C_CULTURE] / 5;
                    if temp_value != 0 {
                        if MAX_INT == commerce_rank[C_CULTURE] {
                            commerce_rank[C_CULTURE] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category,
                        // do not count the culture value
                        // we probably do not want to build this here (but we might)
                        if is_limited_wonder && (commerce_rank[C_CULTURE] > (3 + limited_wonder_limit)) {
                            temp_value = 0;
                        }

                        value += temp_value;
                    }
                }

                if (focus_flags & BUILDINGFOCUS_ESPIONAGE != 0) || (no_espionage && (focus_flags & BUILDINGFOCUS_CULTURE != 0)) {
                    let mut temp_value = (kb.commerce_modifier[C_ESPIONAGE] * g.city_base_commerce_rate(c, CommerceType::COMMERCE_ESPIONAGE)) / 60;

                    if temp_value != 0 {
                        if MAX_INT == commerce_rank[C_ESPIONAGE] {
                            commerce_rank[C_ESPIONAGE] = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_ESPIONAGE);
                        }

                        // if this is a limited wonder, and we are not one of the top 4 in this category, subtract the value
                        // we do _not_ want to build this here (unless the value was small anyway)
                        if is_limited_wonder && (commerce_rank[C_ESPIONAGE] > (3 + limited_wonder_limit)) {
                            temp_value *= -1;
                        }

                        value += temp_value;
                    }
                    let mut temp_value = kb.commerce_change[C_ESPIONAGE] * 3;
                    temp_value += kb.obsolete_safe_commerce_change[C_ESPIONAGE] * 3;
                    temp_value *= 100 + kb.commerce_modifier[C_ESPIONAGE];
                    value += temp_value / 100;
                }
            }

            if (threshold > 0) && (pass == 0) {
                if value < threshold {
                    value = 0;
                }
            }

            if pass > 0 && !g.city_is_human(c) {
                value += kb.ai_weight;
                if value > 0 {
                    for (flavor, &flavor_value) in kb.flavor_value.iter_enumerated() {
                        value += self.player_ai_get_flavor_value(g, p, flavor) * flavor_value;
                    }
                }
            }
        }

        // (The obsolescence and prereq-of-other-buildings checks are commented out in C++.)

        if !g.city_can_construct(c, building, false, false, false) {
            // This building is being constructed in some special way,
            // reduce the value for small cities.
            if population < 6 {
                value /= 8 - population;
            }
        }

        0.max(value)
    }

    /// C++ `ProjectTypes CvCityAI::AI_bestProject()` (CvCityAI.cpp:4387-4468)
    pub fn city_ai_best_project(&self, g: &Game, c: CityId) -> Option<ProjectType> {
        let d = g.d;
        let p = g.cities[c].owner;
        let t = g.city_team(c);

        // Skyre Mod, BETTER AI (block AI from building projects too early) merged Sephi
        if g.players[p].max_cities != 1 {
            if g.players[p].num_cities() == 1 {
                return None;
            }
        }

        let production_rank = g.city_find_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);

        let mut best_value = 0;
        let mut best_project: Option<ProjectType> = None;

        let create_percent = d.game_speeds[g.game_speed_type()].create_percent;
        let num_cities = g.players[p].num_cities();

        for project in d.projects.keys() {
            if !g.city_can_create(c, project, false, false) {
                continue;
            }
            let pi = &d.projects[project];
            let mut value = self.city_ai_project_value(g, c, project);

            if pi.everyone_special_unit.is_some() || pi.everyone_special_building.is_some() || pi.allows_nukes {
                if self.rand(100, "Project Everyone") == 0 {
                    value += 1;
                }
            }

            if value > 0 {
                value += g.cities[c].project_production[project];

                let turns_left = g.city_production_turns_left_project(c, project, 0);

                if (turns_left <= self.game_ai_turns_percent(g, 10, create_percent)) || !g.team_is_human(t) {
                    if (turns_left <= self.game_ai_turns_percent(g, 20, create_percent)) || (production_rank <= 3.max(num_cities / 2)) {
                        if production_rank == 1 {
                            value += turns_left;
                        } else {
                            value = value.wrapping_mul(1000);
                            value /= (turns_left + 10).max(1);
                        }

                        value = value.max(1);

                        if value > best_value {
                            best_value = value;
                            best_project = Some(project);
                        }
                    }
                }
            }
        }

        best_project
    }

    /// C++ `int CvCityAI::AI_projectValue(ProjectTypes eProject)` (CvCityAI.cpp:4471-4512)
    pub fn city_ai_project_value(&self, g: &Game, c: CityId, project: ProjectType) -> i32 {
        let d = g.d;
        let t = g.city_team(c);
        let pi = &d.projects[project];

        let mut value = 0;

        if pi.nuke_interception > 0 {
            if g.can_train_nukes() {
                value += pi.nuke_interception / 10;
            }
        }

        if pi.tech_share > 0 {
            if pi.tech_share < g.team_get_has_met_civ_count(t, true) {
                value += 20 / pi.tech_share;
            }
        }

        let project_count = g.teams[t].project_count[project];

        for (victory, &victory_threshold) in pi.victory_threshold.iter_enumerated() {
            if g.is_victory_valid(victory) {
                value += 0.max(victory_threshold - project_count) * 20;
            }
        }

        for loop_project in d.projects.keys() {
            value += 0.max(d.projects[loop_project].projects_needed[project] - project_count) * 10;
        }

        // FfH: Added by Kael 09/26/2008
        value += pi.ai_weight;

        value
    }

    /// C++ `ProcessTypes CvCityAI::AI_bestProcess()` (CvCityAI.cpp:4515-4518)
    pub fn city_ai_best_process(&self, g: &Game, c: CityId) -> Option<ProcessType> {
        self.best_process_for(g, c, None)
    }

    /// C++ `ProcessTypes CvCityAI::AI_bestProcess(CommerceTypes eCommerceType)` (CvCityAI.cpp:4520-4545)
    pub fn city_ai_best_process_2(&self, g: &Game, c: CityId, commerce_type: CommerceType) -> Option<ProcessType> {
        self.best_process_for(g, c, Some(commerce_type))
    }

    /// C++ `int CvCityAI::AI_processValue(ProcessTypes eProcess)` (CvCityAI.cpp:4548-4551)
    pub fn city_ai_process_value(&self, g: &Game, c: CityId, process: ProcessType) -> i32 {
        self.process_value_for(g, c, process, None)
    }

    /// C++ `int CvCityAI::AI_processValue(ProcessTypes eProcess, CommerceTypes eCommerceType)` (CvCityAI.cpp:4553-4590)
    pub fn city_ai_process_value_2(&self, g: &Game, c: CityId, process: ProcessType, commerce_type: CommerceType) -> i32 {
        self.process_value_for(g, c, process, Some(commerce_type))
    }

    /// C++ `AI_bestProcess(eCommerceType)` with `NO_COMMERCE` allowed (`None`).
    fn best_process_for(&self, g: &Game, c: CityId, commerce_type: Option<CommerceType>) -> Option<ProcessType> {
        let mut best_value = 0;
        let mut best_process: Option<ProcessType> = None;

        for process in g.d.processes.keys() {
            if g.city_can_maintain(c, process, false) {
                let value = self.process_value_for(g, c, process, commerce_type);

                if value > best_value {
                    best_value = value;
                    best_process = Some(process);
                }
            }
        }

        best_process
    }

    /// C++ `AI_processValue(eProcess, eCommerceType)` with `NO_COMMERCE` allowed (`None`).
    fn process_value_for(&self, g: &Game, c: CityId, process: ProcessType, commerce_type: Option<CommerceType>) -> i32 {
        let p = g.cities[c].owner;
        let pi = &g.d.processes[process];
        let mut valid = commerce_type.is_none();

        let mut value = 0;

        if self.player_ai_is_financial_trouble(g, p) {
            value += pi.production_to_commerce_modifier[C_GOLD];
        }

        // if we own less than 50%, or we need to pop borders
        if (g.plot_calculate_culture_percent(g.city_plot(c), p) < 50) || (opt_to_i32(g.cities[c].culture_level) <= 1) {
            value += pi.production_to_commerce_modifier[C_CULTURE];
        }

        for i in 0..CommerceType::NUM {
            let ct = CommerceType::from_usize(i);
            let mut temp_value = pi.production_to_commerce_modifier[i];
            if !valid && (Some(ct) == commerce_type) && (temp_value > 0) {
                valid = true;
                temp_value *= 2;
            }

            temp_value *= self.player_ai_commerce_weight(g, p, ct, Some(c));

            temp_value /= 100;

            value += temp_value;
        }

        if valid {
            value
        } else {
            0
        }
    }

    /// C++ `int CvCityAI::AI_neededSeaWorkers()` (CvCityAI.cpp:4593-4657)
    pub fn city_ai_needed_sea_workers(&self, g: &Game, c: CityId) -> i32 {
        let p = g.cities[c].owner;
        let mut needed_sea_workers = 0;

        let Some(water_area) = g.city_water_area(c) else {
            return 0;
        };

        let need_route = false;

        if need_route {
            needed_sea_workers += 1;
        }

        needed_sea_workers += g.player_count_unimproved_bonuses(p, water_area, Some(g.city_plot(c)));

        // Skyre Mod, BETTER AI (include possible pirate coves for seaworkers needed) merged Sephi
        if g.player_is_pirate(p) {
            // Add a needed worker provided there is at least one valid cove location
            for i in 0..g.city_num_city_plots(c).max(0) as usize {
                if i != CITY_HOME_PLOT {
                    if let Some(plot) = g.city_index_plot(c, i) {
                        if g.map.plots[plot].working_city == Some(c) {
                            // Make sure the plot is in one of our water areas
                            if g.map.plots[plot].area == Some(water_area) {
                                if g.plot_is_pirate_cove_valid(plot, p) {
                                    needed_sea_workers += 1;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        needed_sea_workers
    }

    /// C++ `bool CvCityAI::AI_isDefended(int iExtra = 0)` (CvCityAI.cpp:4660-4665)
    pub fn city_ai_is_defended(&self, g: &Game, c: CityId, extra: i32) -> bool {
        let owner = g.cities[c].owner;
        let count = g.plot_plot_count(g.city_plot(c), Some(Game::puf_can_defend_group_head), -1, -1, Some(owner), None, Some(Game::puf_is_city_ai_type), -1, -1);
        (count + extra) >= self.city_ai_needed_defenders(g, c) // XXX check for other team's units?
    }

    /// C++ `bool CvCityAI::AI_isAirDefended(int iExtra = 0)` (CvCityAI.cpp:4668-4673)
    pub fn city_ai_is_air_defended(&self, g: &Game, c: CityId, extra: i32) -> bool {
        let owner = g.cities[c].owner;
        let count = g.plot_plot_count(
            g.city_plot(c),
            Some(Game::puf_can_air_defend),
            -1,
            -1,
            Some(owner),
            None,
            Some(Game::puf_is_domain_type),
            DomainType::DOMAIN_AIR.i(),
            -1,
        );
        (count + extra) >= self.city_ai_needed_air_defenders(g, c) // XXX check for other team's units?
    }

    /// C++ `int CvCityAI::AI_neededDefenders()` (CvCityAI.cpp:4676-4755)
    pub fn city_ai_needed_defenders(&self, g: &Game, c: CityId) -> i32 {
        let p = g.cities[c].owner;
        let t = g.city_team(c);
        let area = g.city_area(c);
        let area_ai = g.map.areas[area].area_ai_type[t];
        let offense_war = area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING);
        let defense_war = area_ai == Some(AreaAIType::AREAAI_DEFENSIVE);

        if !self.team_ai_is_war_possible(g, t) {
            return 1;
        }

        if g.city_is_barbarian(c) {
            return 2 + ((g.cities[c].population - 2) / 7);
        }

        let mut defenders = 1;

        if g.city_has_active_world_wonder(c) || g.city_is_capital(c) {
            defenders += 1;
        }

        if !self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CRUSH) {
            defenders += self.city_ai_needed_floating_defenders(g, c);
        } else {
            defenders += (self.city_ai_needed_floating_defenders(g, c) + 2) / 4;
        }

        if defense_war {
            if !g.plot_is_hills(g.city_plot(c)) {
                defenders += 1;
            }
        }

        let turns_since_acquired = g.game_turn() - g.cities[c].game_turn_acquired;

        if turns_since_acquired < 10 {
            if offense_war {
                if !g.city_has_active_world_wonder(c) && !g.city_is_any_holy_city(c) {
                    defenders /= 2;
                    defenders = defenders.max(1);
                }
            }
        }

        if turns_since_acquired < 10 {
            defenders = defenders.max(2);
            if self.city_ai_is_danger(g, c) {
                defenders += 1;
            }
            if defense_war {
                defenders += 1;
            }
        }

        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_LAST_STAND) {
            defenders += 10;
        }

        defenders = defenders.max(self.city_ai_min_defenders(g, c));

        defenders
    }

    /// C++ `int CvCityAI::AI_minDefenders()` (CvCityAI.cpp:4757-4771)
    pub fn city_ai_min_defenders(&self, g: &Game, c: CityId) -> i32 {
        let p = g.cities[c].owner;
        let mut defenders = 1;
        let era = g.players[p].current_era.i();
        if era > 0 {
            defenders += 1;
        }
        if ((era - g.start_era().i() / 2) >= g.d.eras.len() as i32 / 2) && g.city_is_coastal(c, g.d.defines.min_water_size_for_ocean) {
            defenders += 1;
        }

        defenders
    }

    /// C++ `int CvCityAI::AI_neededFloatingDefenders()` (CvCityAI.cpp:4773-4780)
    pub fn city_ai_needed_floating_defenders(&self, g: &Game, c: CityId) -> i32 {
        let cache_turn = self.s().city(c).needed_floating_defenders_cache_turn;
        if cache_turn != g.game_turn() {
            self.city_ai_update_needed_floating_defenders(g, c);
        }
        self.s().city(c).needed_floating_defenders
    }

    /// C++ `void CvCityAI::AI_updateNeededFloatingDefenders()` (CvCityAI.cpp:4782-4796)
    pub fn city_ai_update_needed_floating_defenders(&self, g: &Game, c: CityId) {
        let p = g.cities[c].owner;
        let area = g.city_area(c);

        let mut floating_defenders = self.player_ai_get_total_floating_defenders_needed(g, p, area);

        let total_threat = 1.max(self.player_ai_get_total_area_city_threat(g, p, area));

        floating_defenders -= g.map.areas[area].cities_per_player[p];

        floating_defenders *= self.city_ai_city_threat(g, c, false);
        floating_defenders += total_threat / 2;
        floating_defenders /= total_threat;

        let turn = g.game_turn();
        let mut s = self.s();
        let ca = s.city(c);
        ca.needed_floating_defenders = floating_defenders;
        ca.needed_floating_defenders_cache_turn = turn;
    }

    /// C++ `int CvCityAI::AI_neededAirDefenders()` (CvCityAI.cpp:4798-4846)
    pub fn city_ai_needed_air_defenders(&self, g: &Game, c: CityId) -> i32 {
        let t = g.city_team(c);

        if !self.team_ai_is_war_possible(g, t) {
            return 0;
        }

        let mut defenders = 0;

        let range = 5;

        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let mut other_team = 0;
        let mut enemy_team = 0;
        for dx in -range..=range {
            for dy in -range..=range {
                if let Some(loop_plot) = g.plot_xy(x, y, dx, dy) {
                    if g.plot_is_owned(loop_plot) {
                        let plot_team = g.plot_get_team(loop_plot);
                        if plot_team != Some(t) {
                            other_team += 1;
                            if let Some(plot_team) = plot_team {
                                if self.team_ai_get_war_plan(g, t, plot_team).is_some() {
                                    enemy_team += 2;
                                    if g.plot_is_city(loop_plot, false, None) {
                                        enemy_team += 6;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        defenders += (other_team + enemy_team + 2) / 8;

        defenders = (if enemy_team > 0 { 4 } else { 2 }).min(defenders);

        if defenders == 0 {
            if g.city_is_coastal(c, g.d.defines.min_water_size_for_ocean) {
                defenders += 1;
            }
        }
        defenders
    }

    /// C++ `bool CvCityAI::AI_isDanger()` (CvCityAI.cpp:4849-4852)
    pub fn city_ai_is_danger(&self, g: &Game, c: CityId) -> bool {
        let p = g.cities[c].owner;
        self.player_ai_get_plot_danger(g, p, g.city_plot(c), 2, false) != 0
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidGrowthCount()` (CvCityAI.cpp:4855-4858)
    pub fn city_ai_get_emphasize_avoid_growth_count(&self, g: &Game, c: CityId) -> i32 {
        g.cities[c].ai_emphasize_avoid_growth_count
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidGrowth()` (CvCityAI.cpp:4861-4864)
    pub fn city_ai_is_emphasize_avoid_growth(&self, g: &Game, c: CityId) -> bool {
        g.city_ai_is_emphasize_avoid_growth(c)
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidAngryCitizensCount()` (CvCityAI.cpp:4871-4874)
    pub fn city_ai_get_emphasize_avoid_angry_citizens_count(&self, g: &Game, c: CityId) -> i32 {
        g.cities[c].ai_emphasize_avoid_angry_citizens_count
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidAngryCitizens()` (CvCityAI.cpp:4876-4879)
    pub fn city_ai_is_emphasize_avoid_angry_citizens(&self, g: &Game, c: CityId) -> bool {
        g.city_ai_is_emphasize_avoid_angry_citizens(c)
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidUnhealthyCitizensCount()` (CvCityAI.cpp:4881-4884)
    pub fn city_ai_get_emphasize_avoid_unhealthy_citizens_count(&self, g: &Game, c: CityId) -> i32 {
        g.cities[c].ai_emphasize_avoid_unhealthy_citizens_count
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidUnhealthyCitizens()` (CvCityAI.cpp:4886-4889)
    pub fn city_ai_is_emphasize_avoid_unhealthy_citizens(&self, g: &Game, c: CityId) -> bool {
        g.city_ai_is_emphasize_avoid_unhealthy_citizens(c)
    }

    /// C++ `int CvCityAI::AI_getEmphasizeGreatPeopleCount()` (CvCityAI.cpp:4894-4897)
    pub fn city_ai_get_emphasize_great_people_count(&self, g: &Game, c: CityId) -> i32 {
        g.cities[c].ai_emphasize_great_people_count
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeGreatPeople()` (CvCityAI.cpp:4900-4903)
    pub fn city_ai_is_emphasize_great_people(&self, g: &Game, c: CityId) -> bool {
        g.city_ai_is_emphasize_great_people(c)
    }

    /// C++ `bool CvCityAI::AI_isAssignWorkDirty()` (CvCityAI.cpp:4906-4909)
    pub fn city_ai_is_assign_work_dirty(&self, g: &Game, c: CityId) -> bool {
        g.cities[c].ai_assign_work_dirty
    }

    /// C++ `void CvCityAI::AI_setAssignWorkDirty(bool bNewValue)` (CvCityAI.cpp:4912-4915)
    pub fn city_ai_set_assign_work_dirty(&self, g: &mut Game, c: CityId, new_value: bool) {
        g.cities[c].ai_assign_work_dirty = new_value;
    }

    /// C++ `bool CvCityAI::AI_isChooseProductionDirty()` (CvCityAI.cpp:4918-4921)
    pub fn city_ai_is_choose_production_dirty(&self, g: &Game, c: CityId) -> bool {
        g.cities[c].ai_choose_production_dirty
    }

    /// C++ `void CvCityAI::AI_setChooseProductionDirty(bool bNewValue)` (CvCityAI.cpp:4924-4927)
    pub fn city_ai_set_choose_production_dirty(&self, g: &mut Game, c: CityId, new_value: bool) {
        g.cities[c].ai_choose_production_dirty = new_value;
    }

    /// C++ `CvCity* CvCityAI::AI_getRouteToCity() const` (CvCityAI.cpp:4930-4933)
    pub fn city_ai_get_route_to_city(&self, g: &Game, c: CityId) -> Option<CityId> {
        // C++ `getCity(IDInfo)`: NULL for a city that no longer exists.
        let route_to_city = self.s().city(c).route_to_city;
        route_to_city.filter(|&rc| g.cities.contains(rc))
    }

    /// C++ `void CvCityAI::AI_updateRouteToCity()` (CvCityAI.cpp:4936-4984)
    pub fn city_ai_update_route_to_city(&self, g: &mut Game, c: CityId) {
        let g: &Game = &*g;

        g.route_finder.borrow_mut().force_reset();

        let mut best_value = MAX_INT;
        let mut best_city: Option<CityId> = None;

        let owner = g.cities[c].owner;
        let t = g.city_team(c);
        let area = g.city_area(c);
        let (x, y) = (g.cities[c].x, g.cities[c].y);

        for loop_player in g.player_ids() {
            if g.players[loop_player].team == t {
                for loop_city in g.players[loop_player].cities.clone() {
                    if loop_city != c {
                        if g.city_area(loop_city) == area {
                            let (lx, ly) = (g.cities[loop_city].x, g.cities[loop_city].y);
                            let found = g.route_finder.borrow_mut().generate_path(g, x, y, lx, ly, false, owner.i(), true);
                            if !found {
                                let value = g.plot_distance(x, y, lx, ly);

                                if value < best_value {
                                    best_value = value;
                                    best_city = Some(loop_city);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.s().city(c).route_to_city = best_city;
    }

    /// C++ `int CvCityAI::AI_getEmphasizeYieldCount(YieldTypes eIndex)` (CvCityAI.cpp:4987-4992)
    pub fn city_ai_get_emphasize_yield_count(&self, g: &Game, c: CityId, index: YieldType) -> i32 {
        g.cities[c].ai_emphasize_yield_count[index.idx()]
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeYield(YieldTypes eIndex)` (CvCityAI.cpp:4995-4998)
    pub fn city_ai_is_emphasize_yield(&self, g: &Game, c: CityId, index: YieldType) -> bool {
        g.city_ai_is_emphasize_yield(c, index)
    }

    /// C++ `int CvCityAI::AI_getEmphasizeCommerceCount(CommerceTypes eIndex)` (CvCityAI.cpp:5001-5006)
    pub fn city_ai_get_emphasize_commerce_count(&self, g: &Game, c: CityId, index: CommerceType) -> i32 {
        // C++ quirk: returns `count > 0` (a bool as int), not the count.
        (g.cities[c].ai_emphasize_commerce_count[index.idx()] > 0) as i32
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeCommerce(CommerceTypes eIndex)` (CvCityAI.cpp:5009-5012)
    pub fn city_ai_is_emphasize_commerce(&self, g: &Game, c: CityId, index: CommerceType) -> bool {
        g.city_ai_is_emphasize_commerce(c, index)
    }

    /// C++ `bool CvCityAI::AI_isEmphasize(EmphasizeTypes eIndex)` (CvCityAI.cpp:5015-5021)
    pub fn city_ai_is_emphasize(&self, g: &Game, c: CityId, index: EmphasizeType) -> bool {
        g.cities[c].ai_emphasize[index]
    }

    /// C++ `void CvCityAI::AI_setEmphasize(EmphasizeTypes eIndex, bool bNewValue)` (CvCityAI.cpp:5024-5091)
    pub fn city_ai_set_emphasize(&self, g: &mut Game, c: CityId, index: EmphasizeType, new_value: bool) {
        // ffh-core ports the same C++ (counts, then AI_assignWorkingPlots through the owner's
        // controller).
        g.city_ai_set_emphasize(c, index, new_value);
    }

    /// C++ `void CvCityAI::AI_forceEmphasizeCulture(bool bNewValue)` (CvCityAI.cpp:5093-5102)
    pub fn city_ai_force_emphasize_culture(&self, g: &mut Game, c: CityId, new_value: bool) {
        let current = self.s().city(c).force_emphasize_culture;
        if current != new_value {
            self.s().city(c).force_emphasize_culture = new_value;

            g.cities[c].ai_emphasize_commerce_count[C_CULTURE] += if new_value { 1 } else { -1 };
        }
    }

    /// C++ `int CvCityAI::AI_getBestBuildValue(int iIndex)` (CvCityAI.cpp:5105-5110)
    pub fn city_ai_get_best_build_value(&self, g: &Game, c: CityId, index: i32) -> i32 {
        self.s().city(c).best_build_value[index as usize]
    }

    /// C++ `int CvCityAI::AI_totalBestBuildValue(CvArea* pArea)` (CvCityAI.cpp:5113-5145)
    pub fn city_ai_total_best_build_value(&self, g: &Game, c: CityId, area: AreaId) -> i32 {
        let p = g.cities[c].owner;
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let safe_automation = g.players[p].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION];
        let ruins = g.d.defines.ruins_improvement;

        let mut total_value = 0;

        // FfH: Modified by Kael 11/18/2007 (getNumCityPlots)
        for i in 0..g.city_num_city_plots(c).max(0) as usize {
            if i != CITY_HOME_PLOT {
                if let Some(loop_plot) = g.plot_city(x, y, i) {
                    if g.map.plots[loop_plot].area == Some(area) {
                        let improvement = g.map.plots[loop_plot].improvement_type;
                        if improvement.is_none() || !(safe_automation && !(opt_to_i32(improvement) == ruins)) {
                            total_value += self.city_ai_get_best_build_value(g, c, i as i32);
                        }
                    }
                }
            }
        }

        total_value
    }

    /// C++ `int CvCityAI::AI_clearFeatureValue(int iIndex)` (CvCityAI.cpp:5147-5214)
    pub fn city_ai_clear_feature_value(&self, g: &Game, c: CityId, index: i32) -> i32 {
        let d = g.d;
        let p = g.cities[c].owner;
        let t = g.city_team(c);
        let (x, y) = (g.cities[c].x, g.cities[c].y);

        // PORT ADAPT: C++ asserts a plot with a feature (and would crash otherwise).
        let Some(plot) = g.plot_city(x, y, index.max(0) as usize) else {
            return 0;
        };
        let Some(feature) = g.map.plots[plot].feature_type else {
            return 0;
        };

        let fi = &d.features[feature];
        let improvement = g.map.plots[plot].improvement_type;

        let mut value = 0;
        value += fi.yield_change[Y_FOOD] * 100;
        value += fi.yield_change[Y_PROD] * 60;
        value += fi.yield_change[Y_COMM] * 40;

        if value > 0 && g.plot_is_being_worked(plot) {
            value *= 3;
            value /= 2;
        }
        if value != 0 {
            if let Some(bonus) = g.plot_get_bonus_type(plot, Some(t)) {
                value *= 3;
                if let Some(imp) = improvement {
                    if d.improvements[imp].improvement_bonus[bonus].bonus_trade {
                        value *= 4;
                    }
                }
            }
        }

        let mut health_value = 0;
        if fi.health_percent != 0 {
            let health = g.city_good_health(c) - g.city_bad_health(c, false, 0);

            health_value += (6 * fi.health_percent) / 3.max(1 + health);
            if health_value > 0 && !g.plot_is_being_worked(plot) {
                health_value *= 3;
                health_value /= 2;
            }
        }
        value += health_value;

        if value > 0 {
            if let Some(imp) = improvement {
                if d.improvements[imp].requires_feature {
                    value += 500;
                }
            }

            if g.players[p].advanced_start_points >= 0 {
                value += 400;
            }
        }

        -value
    }

    /// C++ `BuildTypes CvCityAI::AI_getBestBuild(int iIndex)` (CvCityAI.cpp:5216-5221)
    pub fn city_ai_get_best_build(&self, g: &Game, c: CityId, index: i32) -> Option<BuildType> {
        self.s().city(c).best_build[index as usize]
    }

    /// C++ `int CvCityAI::AI_countBestBuilds(CvArea* pArea)` (CvCityAI.cpp:5224-5256)
    pub fn city_ai_count_best_builds(&self, g: &Game, c: CityId, area: AreaId) -> i32 {
        let (x, y) = (g.cities[c].x, g.cities[c].y);

        let mut count = 0;

        // FfH: Modified by Kael 11/18/2007 (getNumCityPlots)
        for i in 0..g.city_num_city_plots(c).max(0) as usize {
            if i != CITY_HOME_PLOT {
                if let Some(loop_plot) = g.plot_city(x, y, i) {
                    if g.map.plots[loop_plot].area == Some(area) {
                        if self.city_ai_get_best_build(g, c, i as i32).is_some() {
                            count += 1;
                        }
                    }
                }
            }
        }

        count
    }
}
