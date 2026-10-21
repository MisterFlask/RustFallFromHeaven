//! Port of CvCityAI.cpp:5259-7883 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `city_ai_count_num_bonuses` (city_4.rs) takes `bonus: BonusType`, but C++ callers pass
//!   `NO_BONUS`; this file uses the private [`count_num_bonuses_any`] (a port of
//!   `AI_countNumBonuses` with `eBonus = NO_BONUS`) until that parameter is an `Option`.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// PORT ADAPT: the C++ divides by AI averages / multipliers / thresholds that are positive in
/// play; a zero divisor here leaves the dividend unchanged instead of crashing.
fn sdiv(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        a / b
    }
}

/// PORT ADAPT: `a % b` that does not panic on a zero divisor (C++ would crash).
fn smod(a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a % b
    }
}

/// C++ `GC.getCivilizationInfo(eCiv).getCivilizationUnits(eClass)`.
fn civ_unit(g: &Game, civ: CivilizationType, class: UnitClassType) -> Option<UnitType> {
    g.d.civilizations[civ].civilization_units(class, g.d.unit_classes[class].default_unit_index)
}

/// C++ `CvCityAI::AI_countNumBonuses(NO_BONUS, ..)` (CvCityAI.cpp:9592-9635). The city_4.rs
/// port takes a non-optional bonus; every caller in this file passes `NO_BONUS`.
fn count_num_bonuses_any(g: &Game, c: CityId, include_ours: bool, include_neutral: bool, other_culture_threshold: i32, land: bool, water: bool) -> i32 {
    let city = &g.cities[c];
    let owner = city.owner;
    let team = g.city_team(c);
    let area = g.city_area(c);
    let mut count = 0;
    for i in 0..NUM_CITY_PLOTS {
        let Some(pl) = g.plot_city(city.x, city.y, i) else {
            continue;
        };
        // C++ quirk: bLand is never read.
        if g.map.plots[pl].area == Some(area) || (water && g.plot_is_water(pl)) {
            if g.plot_get_bonus_type(pl, Some(team)).is_some() {
                let plot_owner = g.map.plots[pl].owner;
                if include_ours && plot_owner == Some(owner) && g.map.plots[pl].working_city == Some(c) {
                    count += 1;
                } else if include_neutral && !g.plot_is_owned(pl) {
                    count += 1;
                } else if other_culture_threshold > 0 && (g.plot_is_owned(pl) && plot_owner != Some(owner)) {
                    let po = plot_owner.unwrap();
                    if g.map.plots[pl].culture[po] - g.map.plots[pl].culture[owner] < other_culture_threshold {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Area AI of the city's area for its team is offensive, defensive or massing.
fn is_land_war_area(g: &Game, c: CityId) -> bool {
    let area = g.city_area(c);
    let team = g.city_team(c);
    let t = g.map.areas[area].area_ai_type[team];
    t == Some(AreaAIType::AREAAI_OFFENSIVE) || t == Some(AreaAIType::AREAAI_DEFENSIVE) || t == Some(AreaAIType::AREAAI_MASSING)
}

impl Cai {
    /// C++ `void CvCityAI::AI_updateBestBuild()` (CvCityAI.cpp:5260-5754)
    pub fn city_ai_update_best_build(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let food_cons = d.defines.food_consumption_per_population;
        let (mut best_build, mut best_build_value) = {
            let mut s = self.s();
            let ca = s.city(c);
            (ca.best_build, ca.best_build_value)
        };

        let mut final_yields = [0i32; YieldType::NUM];

        let mut bonus_food_surplus = 0;
        let mut bonus_food_deficit = 0;
        let mut feature_food_surplus = 0;
        let mut hill_food_deficit = 0;
        let mut food_total = d.yields[YieldType::YIELD_FOOD].min_city;
        let mut production_total = d.yields[YieldType::YIELD_PRODUCTION].min_city;
        let mut food_multiplier = 100;
        let mut commerce_multiplier = 100;
        let mut production_multiplier = 100;
        let mut worker_count = 0;

        let mut worked_food = 0;
        let mut workable_food = 0;
        let mut workable_food_plot_count = 0;

        let mut chop = false;

        let mut good_tile_count = 0;

        let specialist_count = g.cities[c].specialist_population - g.city_total_free_specialists(c);

        let mut happy_adjust = 0;
        let mut health_adjust = 0;

        let num_city_plots = (g.city_num_city_plots(c).max(0) as usize).min(NUM_CITY_PLOTS);

        // FfH: Modified by Kael 11/18/2007 (getNumCityPlots)
        for i in 0..num_city_plots {
            if i == CITY_HOME_PLOT {
                continue;
            }
            let Some(pl) = g.city_index_plot(c, i) else {
                continue;
            };
            if g.map.plots[pl].working_city != Some(c) {
                continue;
            }
            let count = self.player_ai_plot_target_mission_ais(g, owner, pl, MissionAIType::MISSIONAI_BUILD, None, 0);
            worker_count += count;

            let mut use_base_value = true;
            // If the tile has a BestBuild, then use the BestBuild
            // determine if the tile is being improved.
            if let Some(build) = best_build[i] {
                if best_build_value[i] > 0 && count > 0 {
                    if let Some(improvement) = d.builds[build].improvement {
                        let mut ignore_feature = false;
                        if let Some(feature) = g.map.plots[pl].feature_type {
                            // FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
                            if d.builds[build].feature_remove[feature] && !d.civilizations[g.city_civilization_type(c)].maintain_features[feature] {
                                ignore_feature = true;
                            }
                        }

                        happy_adjust += d.improvements[improvement].happiness;
                        if let Some(cur) = g.map.plots[pl].improvement_type {
                            happy_adjust -= d.improvements[cur].happiness;
                        }

                        use_base_value = false;
                        for j in 0..YieldType::NUM {
                            let y = YieldType::from_usize(j);
                            final_yields[j] = g.plot_calculate_nature_yield(pl, y, Some(team), ignore_feature) + g.plot_calculate_improvement_yield_change(pl, improvement, y, Some(owner), false, false);
                        }
                    }
                }
            }

            // Otherwise use the base value.
            if use_base_value {
                for j in 0..YieldType::NUM {
                    // by default we'll use the current value
                    final_yields[j] = g.map.plots[pl].yields[j];
                    if let Some(feature) = g.map.plots[pl].feature_type {
                        final_yields[j] += 0.max(-d.features[feature].yield_change[j]);
                    }
                }
            }

            let food = final_yields[YieldType::YIELD_FOOD.idx()];
            let prod = final_yields[YieldType::YIELD_PRODUCTION.idx()];
            let comm = final_yields[YieldType::YIELD_COMMERCE.idx()];
            let being_worked = g.plot_is_being_worked(pl);

            if being_worked {
                worked_food += food;
            } else if food >= food_cons {
                workable_food += food;
                workable_food_plot_count += 1;
            }

            let bonus = g.plot_get_bonus_type(pl, Some(team));

            if being_worked || ((food * 10) + (prod * 6) + (comm * 4)) > 21 {
                good_tile_count += 1;
                if being_worked {
                    food_total += food;
                } else {
                    food_total += food / 2;
                }
                if prod > 1 {
                    production_total += prod;
                }
            }

            if bonus.is_some() {
                let net_food = food - food_cons;
                bonus_food_surplus += 0.max(net_food);
                bonus_food_deficit += 0.max(-net_food);
            }

            if g.map.plots[pl].feature_type.is_some() {
                feature_food_surplus += 0.max(g.plot_calculate_nature_yield(pl, YieldType::YIELD_FOOD, Some(team), false) - food_cons);
            }

            if g.plot_is_hills(pl) {
                hill_food_deficit += 0.max(food_cons - g.plot_calculate_nature_yield(pl, YieldType::YIELD_FOOD, Some(team), false));
            }
        }

        //XXX rewrite this to fix too many farms issue
        let mut specialist_adjustment = 0;
        if workable_food_plot_count > 0 {
            //Calculate approximately how much food the assigned specialists could work
            specialist_adjustment = (specialist_count.min(workable_food_plot_count) * workable_food) / workable_food_plot_count;
        }
        food_total += specialist_adjustment;

        let bonus_food_diff = (bonus_food_surplus + feature_food_surplus) - (bonus_food_deficit + hill_food_deficit / 2);

        let population = g.cities[c].population;
        let health = g.city_good_health(c) - g.city_bad_health(c, false, 0);
        let mut target_size = good_tile_count.min(population + (g.city_happy_level(c) - g.city_unhappy_level(c, 0)));
        target_size = target_size.min(1 + population + health);

        let current_era = g.players[owner].current_era.i();
        if g.players[owner].advanced_start_points >= 0 {
            target_size += 2 + current_era / 2;
        }

        // C++ quirk: the advanced start bonus is applied twice.
        if g.players[owner].advanced_start_points >= 0 {
            target_size += current_era / 2;
        }

        if bonus_food_diff < 2 {
            food_multiplier += 10 * (2 - bonus_food_diff);
        }

        let mut extra_food_for_growth = (0.max(target_size - population) + 3) / 4;
        if population < target_size {
            extra_food_for_growth += 1;
        }

        let food_difference = food_total - ((target_size * food_cons) + extra_food_for_growth);

        let mut desired_food_change = -food_difference + 0.max(-health);
        if target_size > population {
            if desired_food_change > 3 {
                desired_food_change = (desired_food_change + 3) / 2;
            }
        }

        if food_difference < 0 {
            food_multiplier += -food_difference * 4;
        }

        if food_difference > 4 {
            food_multiplier -= 8 + 4 * food_difference;
        }

        if production_total < 10 {
            production_multiplier += 80 - 8 * production_total;
        }
        let production_target = 1 + population.min((target_size * 3) / 5);

        if production_total < production_target {
            production_multiplier += 8 * (production_target - production_total);
        }

        if (bonus_food_surplus + feature_food_surplus > 5) && ((bonus_food_deficit + hill_food_deficit) > 5) {
            if (bonus_food_deficit + hill_food_deficit) > 8 {
                //probably a good candidate for a wonder pump
                production_multiplier += 40;
                commerce_multiplier += if self.player_ai_is_financial_trouble(g, owner) { 0 } else { -40 };
            }
        }

        let gold_per_turn = g.players[owner].gold_per_turn;
        let net_commerce = 1 + g.player_get_commerce_rate(owner, CommerceType::COMMERCE_GOLD) + g.player_get_commerce_rate(owner, CommerceType::COMMERCE_RESEARCH) + 0.max(gold_per_turn);
        let net_expenses = g.player_calculate_inflated_costs(owner) + 0.min(gold_per_turn);
        let ratio = (100 * net_expenses) / 1.max(net_commerce);

        if ratio > 40 {
            commerce_multiplier += (33 * (ratio - 40)) / 60;
        }

        if self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_FOOD) {
            food_multiplier *= 130;
            food_multiplier /= 100;
        }
        if self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_PRODUCTION) {
            production_multiplier *= 140;
            production_multiplier /= 100;
        }
        if self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_COMMERCE) {
            commerce_multiplier *= 140;
            commerce_multiplier /= 100;
        }

        let mut production_advantage = 100 * self.city_ai_yield_multiplier(g, c, YieldType::YIELD_PRODUCTION);
        production_advantage = sdiv(production_advantage, self.player_ai_average_yield_multiplier(g, owner, YieldType::YIELD_PRODUCTION));
        production_advantage *= self.player_ai_average_yield_multiplier(g, owner, YieldType::YIELD_COMMERCE);
        production_advantage = sdiv(production_advantage, self.city_ai_yield_multiplier(g, c, YieldType::YIELD_COMMERCE));

        //now we normalize the effect by # of cities
        let num_cities = g.players[owner].num_cities();

        //in short in an OCC the relative multipliers should *never* make a difference
        //so this indeed equals "100" for the iNumCities == 0 case.
        production_advantage = (production_advantage * (num_cities - 1) + 200) / (num_cities + 1);

        production_multiplier *= production_advantage;
        production_multiplier /= 100;

        commerce_multiplier *= 100;
        commerce_multiplier = sdiv(commerce_multiplier, production_advantage);

        let mut great_people_advantage = 100 * g.city_total_great_people_rate_modifier(c);
        great_people_advantage = sdiv(great_people_advantage, self.player_ai_average_great_people_multiplier(g, owner));
        great_people_advantage = (great_people_advantage * (num_cities - 1) + 200) / (num_cities + 1);
        great_people_advantage += 200; //gpp multipliers are larger than others so lets not go overboard
        great_people_advantage /= 3;

        //With great people we want to slightly increase food priority at the expense of commerce
        //this gracefully handles both wonder and specialist based GPP...
        commerce_multiplier *= 100;
        commerce_multiplier = sdiv(commerce_multiplier, great_people_advantage);
        food_multiplier *= great_people_advantage;
        food_multiplier /= 100;

        // if leader flavor likes production, increase production, reduce commerce
        if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_PRODUCTION) {
            production_multiplier += 10;
            commerce_multiplier -= 10;
        }

        if food_multiplier < 100 {
            food_multiplier = 10000 / (200 - food_multiplier);
        }
        if production_multiplier < 100 {
            production_multiplier = 10000 / (200 - production_multiplier);
        }
        if commerce_multiplier < 100 {
            commerce_multiplier = 10000 / (200 - commerce_multiplier);
        }

        if g.city_angry_population(c, 1) > 0 {
            food_multiplier /= 2;
        }

        if !chop {
            if let Some(project) = g.city_production_project(c) {
                chop = self.city_ai_project_value(g, c, project) > 0;
            }
        }
        if !chop {
            if let Some(building) = g.city_production_building(c) {
                chop = d.buildings[building].building_class_type.map_or(false, |bc| is_world_wonder_class(d, bc));
            }
        }
        if !chop {
            if let Some(unit) = g.city_production_unit(c) {
                chop = d.units[unit].food_production;
            }
        }
        if !chop {
            chop = is_land_war_area(g, c);
        }

        if let Some(building) = g.city_production_building(c) {
            happy_adjust += g.city_building_happiness(c, building);
            health_adjust += g.city_building_health(c, building);
        }

        // FfH: Modified by Kael 11/18/2007
        if g.players[owner].ignore_food {
            food_multiplier = 0;
        }
        for i in 0..num_city_plots {
            best_build_value[i] = 0;
            best_build[i] = None;

            if i == CITY_HOME_PLOT {
                continue;
            }
            let Some(pl) = g.city_index_plot(c, i) else {
                continue;
            };
            if g.map.plots[pl].working_city == Some(c) {
                let mut v = 0;
                let mut b = None;
                self.city_ai_best_plot_build(g, c, pl, &mut v, &mut b, food_multiplier, production_multiplier, commerce_multiplier, chop, happy_adjust, health_adjust, desired_food_change);
                v *= 4;
                v += 3 + worker_count; // to round up
                v /= 4 + worker_count;
                best_build_value[i] = v;
                best_build[i] = b;
            }
        }

        {
            //new experimental yieldValue calcuation
            let mut best_plot: Option<usize> = None;
            let mut best_plot_value = -1;

            let mut best_unworked_plot_value = 0;
            let mut values = [0i32; NUM_CITY_PLOTS];

            // FfH: Modified by Kael 02/05/2009 (getNumCityPlots)
            for i in 0..num_city_plots {
                if i == CITY_HOME_PLOT {
                    continue;
                }
                let Some(pl) = g.city_index_plot(c, i) else {
                    continue;
                };
                if g.map.plots[pl].working_city != Some(c) {
                    continue;
                }
                if let Some(build) = best_build[i] {
                    let mut yields = [0i32; YieldType::NUM];
                    for j in 0..YieldType::NUM {
                        yields[j] = g.plot_get_yield_with_build(pl, build, YieldType::from_usize(j), true);
                    }

                    let mut value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                    values[i] = value;
                    if value > 0 && g.map.plots[pl].route_type.is_some() {
                        value += 1;
                    }

                    value = 0.max(value);

                    best_build_value[i] *= value + 100;
                    best_build_value[i] /= 100;

                    if value > best_plot_value {
                        best_plot = Some(i);
                        best_plot_value = value;
                    }
                }
                if !g.plot_is_being_worked(pl) {
                    let yields = g.map.plots[pl].yields;
                    let value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                    best_unworked_plot_value = best_unworked_plot_value.max(value);
                }
            }
            if let Some(bp) = best_plot {
                best_build_value[bp] *= 2;
            }

            //Prune plots which are sub-par.
            if best_unworked_plot_value > 0 {
                for i in 0..num_city_plots {
                    if i == CITY_HOME_PLOT {
                        continue;
                    }
                    let Some(pl) = g.city_index_plot(c, i) else {
                        continue;
                    };
                    if g.map.plots[pl].working_city != Some(c) {
                        continue;
                    }
                    let Some(build) = best_build[i] else {
                        continue;
                    };
                    let cur_imp = g.map.plots[pl].improvement_type;
                    if !g.plot_is_being_worked(pl) && cur_imp.is_none() {
                        if d.builds[build].improvement.is_some() {
                            if values[i] <= best_unworked_plot_value && values[i] < 500 {
                                best_build_value[i] = 1;
                            }
                        }
                    } else if cur_imp.is_some() && d.builds[build].improvement.is_some() {
                        let yields = g.map.plots[pl].yields;
                        let value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                        if value > values[i] {
                            best_build_value[i] = 1;
                        }
                    }
                }
            }
        }

        let mut s = self.s();
        let ca = s.city(c);
        ca.best_build = best_build;
        ca.best_build_value = best_build_value;
    }

    /// C++ `void CvCityAI::AI_doDraft(bool bForce = false)` (CvCityAI.cpp:5759-5814)
    pub fn city_ai_do_draft(&self, g: &mut Game, c: CityId, force: bool) {
        if g.city_is_barbarian(c) {
            return;
        }

        if g.city_can_conscript(c) {
            let Some(conscript_unit) = g.city_conscript_unit(c) else {
                return;
            };
            if g.d.units[conscript_unit].combat > 5 {
                if force {
                    g.city_conscript(c);
                    return;
                }

                let land_war = is_land_war_area(g, c);
                let danger = !self.city_ai_is_defended(g, c, 0) && self.city_ai_is_danger(g, c);
                let conscript_pop = g.city_conscript_population(c);
                let happy_diff = g.d.defines.conscript_pop_anger - conscript_pop;

                if land_war && 0 == g.city_angry_population(c, happy_diff) {
                    let mut wait = false;

                    if !danger {
                        if !wait {
                            if g.cities[c].conscript_anger_timer > 0 {
                                wait = true;
                            }
                        }

                        if !wait {
                            if 3 * (g.cities[c].population - conscript_pop) < g.cities[c].highest_population * 2 {
                                wait = true;
                            }
                        }
                    }

                    if !wait {
                        g.city_conscript(c);
                    }
                }
            }
        }
    }

    /// C++ `void CvCityAI::AI_doHurry(bool bForce = false)` (CvCityAI.cpp:5817-6306)
    pub fn city_ai_do_hurry(&self, g: &mut Game, c: CityId, force: bool) {
        let d = g.d;

        if g.city_is_barbarian(c) {
            return;
        }

        if g.city_production(c) == 0 && !force {
            return;
        }

        let owner = g.cities[c].owner;
        let area = g.city_area(c);
        let water_area = g.city_water_area(c);

        let production_unit = g.city_production_unit(c);
        let production_unit_ai = g.city_production_unit_ai(c);
        let production_building = g.city_production_building(c);

        let danger = self.city_ai_is_danger(g, c);

        for hurry in d.hurries.keys() {
            if !g.city_can_hurry(c, hurry, false) {
                continue;
            }
            if force {
                g.city_hurry(c, hurry);
                break;
            }
            let hurry_anger_length = g.city_hurry_anger_length(c, hurry);
            let hurry_population = g.city_hurry_population(c, hurry);

            let mut min_turns = i32::MAX;
            let mut essential = false;
            let mut growth = false;

            // Whip to eliminate unhappiness - thank you Blake!
            if g.city_production(c) > 0 {
                if self.city_ai_get_happy_from_hurry(g, c, hurry) > 0 {
                    g.city_hurry(c, hurry);
                    break;
                }
            }

            if hurry_anger_length == 0 && hurry_population == 0 {
                if self.player_ai_avoid_science(g, owner) {
                    if g.players[owner].gold > self.player_ai_gold_target(g, owner) {
                        min_turns = min_turns.min(10);
                    }
                }
                if let Some(building) = production_building {
                    let mut value_per_turn = self.city_ai_building_value_threshold(g, c, building, BUILDINGFOCUS_GOLD | BUILDINGFOCUS_MAINTENANCE | BUILDINGFOCUS_PRODUCTION, 0);

                    value_per_turn /= 3;

                    if value_per_turn > 0 {
                        let hurry_gold = g.city_hurry_gold(c, hurry);
                        if (hurry_gold / value_per_turn) < g.city_production_turns_left_building(c, building, 1) {
                            // FfH: Modified by Kael 01/26/2010 (gold reserve)
                            if hurry_gold < ((g.players[owner].gold - self.player_ai_get_gold_reserve(g, owner)) / 3) {
                                g.city_hurry(c, hurry);
                                return;
                            }
                        }
                    }
                }
            }

            if let Some(building) = production_building {
                let bi = &d.buildings[building];

                if bi.building_class_type.map_or(false, |bc| is_world_wonder_class(d, bc)) {
                    min_turns = min_turns.min(10);
                }

                if bi.defense_modifier > 0 {
                    if danger {
                        min_turns = min_turns.min(3);
                        essential = true;
                    }
                }

                if bi.bombard_defense_modifier > 0 {
                    if danger {
                        min_turns = min_turns.min(3);
                        essential = true;
                    }
                }

                if bi.yield_modifier[YieldType::YIELD_PRODUCTION.idx()] > 0 {
                    if g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] >= 6 {
                        min_turns = min_turns.min(10);
                        growth = true;
                    }
                }

                let culture = CommerceType::COMMERCE_CULTURE.idx();
                if bi.commerce_change[culture] > 0 || bi.obsolete_safe_commerce_change[culture] > 0 {
                    let culture_rate = g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE);
                    if culture_rate == 0 || g.plot_calculate_culture_percent(g.city_plot(c), owner) < 40 {
                        min_turns = min_turns.min(10);
                        if culture_rate == 0 {
                            essential = true;
                            min_turns = min_turns.min(5);
                            if count_num_bonuses_any(g, c, false, true, 2, true, true) > 0 {
                                growth = true;
                            }
                        }
                    }
                }

                if bi.happiness > 0 {
                    if g.city_angry_population(c, 0) > 0 {
                        min_turns = min_turns.min(10);
                    }
                }

                if bi.health > 0 {
                    if g.city_health_rate(c, false, 0) < 0 {
                        min_turns = min_turns.min(10);
                    }
                }

                let food = YieldType::YIELD_FOOD.idx();
                if bi.sea_plot_yield_change[food] > 0 || bi.river_plot_yield_change[food] > 0 {
                    min_turns = min_turns.min(10);

                    if self.city_ai_building_special_yield_change_value(g, c, building, YieldType::YIELD_FOOD) > (g.cities[c].population * 2) {
                        essential = true;
                        growth = true;
                    }
                }

                if bi.free_experience > 0 {
                    if danger {
                        min_turns = min_turns.min(3);
                        essential = true;
                    }
                }

                if bi.maintenance_modifier < 0 {
                    if g.city_maintenance(c) >= 10 {
                        min_turns = min_turns.min(10);
                        essential = true;
                    }
                }

                if let Some(default_specialist) = opt_from_i32::<SpecialistType>(d.defines.default_specialist) {
                    if g.cities[c].specialist_count[default_specialist] > 0 {
                        for j in d.specialists.keys() {
                            if bi.specialist_count[j] > 0 {
                                min_turns = min_turns.min(10);
                                break;
                            }
                        }
                    }
                }

                if bi.commerce_modifier[CommerceType::COMMERCE_GOLD.idx()] > 0 {
                    if self.player_ai_is_financial_trouble(g, owner) {
                        if g.city_base_commerce_rate(c, CommerceType::COMMERCE_GOLD) >= 16 {
                            min_turns = min_turns.min(10);
                        }
                    }
                }

                if bi.commerce_modifier[CommerceType::COMMERCE_RESEARCH.idx()] > 0 {
                    if !self.player_ai_avoid_science(g, owner) {
                        if g.city_base_commerce_rate(c, CommerceType::COMMERCE_RESEARCH) >= 16 {
                            min_turns = min_turns.min(10);
                        }
                    }
                }

                if bi.food_kept > 0 {
                    min_turns = min_turns.min(5);
                    essential = true;
                    growth = true;
                }
            }

            if let Some(unit) = production_unit {
                if d.units[unit].domain_type == Some(DomainType::DOMAIN_LAND) {
                    if d.units[unit].combat > 0 {
                        if danger {
                            min_turns = min_turns.min(3);
                            essential = true;
                        }
                    }
                }
            }

            if production_unit_ai == Some(UnitAIType::UNITAI_CITY_DEFENSE) {
                if g.plot_plot_check(g.city_plot(c), Game::puf_is_unit_ai_type, UnitAIType::UNITAI_SETTLE.i(), -1, Some(owner), None, None, -1, -1).is_some() {
                    if !self.city_ai_is_defended(g, c, -2) {
                        // XXX check for other team's units?
                        min_turns = min_turns.min(5);
                    }
                }
            }

            if production_unit_ai == Some(UnitAIType::UNITAI_SETTLE) {
                if g.map.areas[area].num_ai_units[owner][UnitAIType::UNITAI_SETTLE] == 0 {
                    if !self.player_ai_is_financial_trouble(g, owner) {
                        if g.map.areas[area].best_found_value[owner] > 0 {
                            min_turns = min_turns.min(5);
                            essential = true;
                            growth = true;
                        }
                    }
                }
            }

            if production_unit_ai == Some(UnitAIType::UNITAI_SETTLER_SEA) {
                if let Some(wa) = water_area {
                    if g.map.areas[wa].num_ai_units[owner][UnitAIType::UNITAI_SETTLER_SEA] == 0 {
                        if g.map.areas[area].num_ai_units[owner][UnitAIType::UNITAI_SETTLE] > 0 {
                            min_turns = min_turns.min(5);
                        }
                    }
                }
            }

            if production_unit_ai == Some(UnitAIType::UNITAI_WORKER) {
                if self.player_ai_needed_workers(g, owner, area) > (g.map.areas[area].num_ai_units[owner][UnitAIType::UNITAI_WORKER] * 2) {
                    min_turns = min_turns.min(5);
                    essential = true;
                    growth = true;
                }
            }

            if production_unit_ai == Some(UnitAIType::UNITAI_WORKER_SEA) {
                if self.city_ai_needed_sea_workers(g, c) > 0 {
                    min_turns = min_turns.min(5);
                    essential = true;
                    growth = true;
                }
            }

            // adjust for game speed
            let speed = &d.game_speeds[g.game_speed_type()];
            // C++ quirk: with no reason found iMinTurns is MAX_INT and the multiplication
            // overflows (MAX_INT * 100 wraps to -100, so iMinTurns becomes -1 and the
            // "turns left > iMinTurns" test below passes); kept as the DLL plays.
            if g.city_production_unit(c).is_some() {
                min_turns = min_turns.wrapping_mul(speed.train_percent);
            } else if g.city_production_building(c).is_some() {
                min_turns = min_turns.wrapping_mul(speed.construct_percent);
            } else if g.city_production_project(c).is_some() {
                min_turns = min_turns.wrapping_mul(speed.create_percent);
            } else {
                min_turns = min_turns.wrapping_mul(100);
            }

            min_turns /= 100;

            let population = g.cities[c].population;

            //this overrides everything.
            if growth {
                let hurry_gold = g.city_hurry_gold(c, hurry);

                // FfH: Modified by Kael 01/26/2010 (gold reserve)
                if hurry_gold > 0 && (hurry_gold.wrapping_mul(16)) < (g.players[owner].gold - self.player_ai_get_gold_reserve(g, owner)) {
                    g.city_hurry(c, hurry);
                    return;
                }
                if self.city_ai_count_good_tiles(g, c, g.city_health_rate(c, false, 0) == 0, false, 100, false) <= (population - hurry_population) {
                    g.city_hurry(c, hurry);
                    return;
                }
            }
            if self.city_ai_count_good_tiles(g, c, g.city_health_rate(c, false, 0) == 0, false, 100, false) <= (population - hurry_population) {
                let turns_left = g.city_production_turns_left(c);
                if turns_left > min_turns {
                    let mut wait = g.city_is_human(c);

                    if (hurry_population * 3) > (turns_left * 2) {
                        wait = true;
                    }

                    if !wait {
                        if hurry_anger_length > 0 {
                            let hurry_pop_anger = d.defines.hurry_pop_anger;
                            //is the whip just too small or the population just too reduced to bother?
                            if !essential && ((hurry_population < (1 + hurry_pop_anger)) || ((population - hurry_population) <= 3.max(g.cities[c].highest_population / 2))) {
                                wait = true;
                            } else {
                                //sometimes it's worth whipping even with existing anger
                                if g.cities[c].hurry_anger_timer > 1 {
                                    if !essential {
                                        wait = true;
                                    } else if hurry_pop_anger == hurry_population && g.city_angry_population(c, 0) > 0 {
                                        //ideally we'll whip something more expensive
                                        wait = true;
                                    }
                                }
                            }

                            //if the city is just lame then don't whip the poor thing
                            //(it'll still get whipped when unhappy/unhealthy)
                            if !wait && !essential {
                                let mut food_surplus = 0;
                                let food_cons = d.defines.food_consumption_per_population;

                                for j in 0..NUM_CITY_PLOTS {
                                    if j == CITY_HOME_PLOT {
                                        continue;
                                    }
                                    if let Some(pl) = g.city_index_plot(c, j) {
                                        if g.map.plots[pl].working_city == Some(c) {
                                            food_surplus += 0.max(g.map.plots[pl].yields[YieldType::YIELD_FOOD.idx()] - food_cons);
                                        }
                                    }
                                }

                                if food_surplus < 3 {
                                    wait = true;
                                }
                            }
                        }
                    }

                    if !wait {
                        // BETTER AI (better AI gold management) Sephi: only hurry when we have
                        // saved up the gold for it.
                        let hurry_gold = g.city_hurry_gold(c, hurry);
                        if g.players[owner].gold - self.player_ai_get_gold_reserve(g, owner) > hurry_gold {
                            g.city_hurry(c, hurry);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// C++ `void CvCityAI::AI_doEmphasize()` (CvCityAI.cpp:6310-6435)
    pub fn city_ai_do_emphasize(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let owner = g.cities[c].owner;
        let culture_victory = self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE2);

        //Note from Blake:
        //Emphasis proved to be too clumsy to manage AI economies,
        //as such it's been nearly completely phased out by
        //the AI_specialYieldMultiplier system which allows arbitary
        //value-boosts and works very well.
        //Ideally the AI should never use emphasis.

        let first_tech = match g.player_get_current_research(owner) {
            Some(tech) => self.player_ai_is_first_tech(g, owner, tech),
            None => false,
        };

        let population_rank = g.city_find_population_rank(c);

        for e in d.emphasizes.keys() {
            if !g.cities.contains(c) {
                return;
            }
            let info = &d.emphasizes[e];
            let mut emphasize = false;

            if self.city_ai_special_yield_multiplier(g, c, YieldType::YIELD_PRODUCTION) < 50 {
                if info.yield_change[YieldType::YIELD_COMMERCE.idx()] > 0 {
                    if first_tech {
                        emphasize = true;
                    }
                }

                if info.commerce_change[CommerceType::COMMERCE_RESEARCH.idx()] > 0 {
                    if first_tech && !culture_victory {
                        if population_rank < ((g.players[owner].num_cities() / 4) + 1) {
                            emphasize = true;
                        }
                    }
                }

                if info.great_people {
                    let mut high_food_total = 0;
                    let mut high_food_plot_count = 0;
                    let mut high_hammer_plot_count = 0;
                    let mut high_hammer_total = 0;
                    let mut good_food_sink = 0;
                    let food_per_pop = d.defines.food_consumption_per_population;
                    for i in 0..NUM_CITY_PLOTS {
                        let Some(pl) = g.city_index_plot(c, i) else {
                            continue;
                        };
                        if g.map.plots[pl].working_city != Some(c) {
                            continue;
                        }
                        let food = g.map.plots[pl].yields[YieldType::YIELD_FOOD.idx()];
                        if food > food_per_pop {
                            high_food_total += food;
                            high_food_plot_count += 1;
                        }
                        let hammers = g.map.plots[pl].yields[YieldType::YIELD_PRODUCTION.idx()];
                        if hammers >= 3 && (hammers + food) >= 4 {
                            high_hammer_plot_count += 1;
                            high_hammer_total += hammers;
                        }
                        let commerce = g.map.plots[pl].yields[YieldType::YIELD_COMMERCE.idx()];
                        if (commerce * 2 + hammers * 3) > 9 {
                            good_food_sink += 0.max(food_per_pop - food);
                        }
                    }

                    if (high_food_total + high_food_plot_count - good_food_sink) >= g.city_food_consumption(c, true, 0) {
                        if high_hammer_plot_count < 2 && high_hammer_total < g.cities[c].population {
                            if self.city_ai_count_good_tiles(g, c, true, false, 100, true) < g.cities[c].population {
                                emphasize = true;
                            }
                        }
                    }
                }
            }

            self.city_ai_set_emphasize(g, c, e, emphasize);

            // BETTER AI (GrowthControl for AI) Sephi
            if info.avoid_angry_citizens {
                self.city_ai_set_emphasize(g, c, e, true);
            }
        }
    }

    /// C++ `bool CvCityAI::AI_chooseUnit(UnitAITypes eUnitAI = NO_UNITAI)` (CvCityAI.cpp:6438-6458)
    pub fn city_ai_choose_unit(&self, g: &mut Game, c: CityId, unit_ai: Option<UnitAIType>) -> bool {
        let mut unit_ai = unit_ai;
        let best_unit = match unit_ai {
            Some(ai) => self.city_ai_best_unit_ai(g, c, ai, false, None),
            None => self.city_ai_best_unit(g, c, false, None, &mut unit_ai),
        };

        if let Some(unit) = best_unit {
            g.city_push_order(c, OrderType::ORDER_TRAIN, unit.i(), opt_to_i32(unit_ai), false, false, false, false);
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_chooseUnit(UnitTypes eUnit, UnitAITypes eUnitAI)` (CvCityAI.cpp:6460-6469)
    pub fn city_ai_choose_unit_2(&self, g: &mut Game, c: CityId, unit: UnitType, unit_ai: UnitAIType) -> bool {
        g.city_push_order(c, OrderType::ORDER_TRAIN, unit.i(), unit_ai.i(), false, false, false, false);
        true
    }

    /// C++ `bool CvCityAI::AI_chooseDefender()` (CvCityAI.cpp:6472-6496)
    pub fn city_ai_choose_defender(&self, g: &mut Game, c: CityId) -> bool {
        let owner = g.cities[c].owner;
        let plot = g.city_plot(c);
        if g.plot_plot_check(plot, Game::puf_is_unit_ai_type, UnitAIType::UNITAI_CITY_SPECIAL.i(), -1, Some(owner), None, None, -1, -1).is_none() {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_SPECIAL)) {
                return true;
            }
        }

        if g.plot_plot_check(plot, Game::puf_is_unit_ai_type, UnitAIType::UNITAI_CITY_COUNTER.i(), -1, Some(owner), None, None, -1, -1).is_none() {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_COUNTER)) {
                return true;
            }
        }

        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_chooseLeastRepresentedUnit(UnitTypeWeightArray & allowedTypes)` (CvCityAI.cpp:6498-6524)
    pub fn city_ai_choose_least_represented_unit(&self, g: &mut Game, c: CityId, allowed_types: &mut Vec<(UnitAIType, i32)>) -> bool {
        let owner = g.cities[c].owner;
        let area = g.city_area(c);

        // std::multimap<int, UnitAITypes, std::greater<int>>: descending value, equal values in
        // insertion order (the sort is stable).
        let mut best_types: Vec<(i32, UnitAIType)> = Vec::new();
        for &(ai, weight) in allowed_types.iter() {
            let mut value = weight;
            value *= 750 + self.rand(250, "AI choose least represented unit");
            value /= 1 + self.player_ai_total_area_unit_ais(g, owner, area, ai);
            best_types.push((value, ai));
        }
        best_types.sort_by(|a, b| b.0.cmp(&a.0));

        for &(_, ai) in &best_types {
            if self.city_ai_choose_unit(g, c, Some(ai)) {
                return true;
            }
        }
        false
    }

    /// C++ `bool CvCityAI::AI_bestSpreadUnit(bool bMissionary, bool bExecutive, int iBaseChance, UnitTypes* eBestSpreadUnit, int* iBestSpreadUnitValue)` (CvCityAI.cpp:6526-6682)
    pub fn city_ai_best_spread_unit(&self, g: &Game, c: CityId, missionary: bool, executive: bool, base_chance: i32, best_spread_unit: &mut Option<UnitType>, best_spread_unit_value: &mut i32) -> bool {
        let d = g.d;
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let area = g.city_area(c);
        let civ = g.city_civilization_type(c);
        let city_plot = g.city_plot(c);

        let mut best_value = 0;

        if missionary {
            for religion in d.religions.keys() {
                if !g.cities[c].has_religion[religion] {
                    continue;
                }
                let has_count = g.players[owner].has_religion_count[religion];
                // PORT ADAPT: guard the division (C++ asserts iHasCount > 0).
                let mut roll = if has_count > 4 { base_chance } else { ((100 - base_chance) / has_count.max(1)) + base_chance };
                let state_religion = g.player_get_state_religion(owner);
                if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_MISSIONARY) {
                    roll *= if state_religion == Some(religion) { 170 } else { 65 };
                    roll /= 100;
                }
                if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE2) {
                    roll += 25;
                } else if !g.team_has_holy_city(team, religion) && !(state_religion == Some(religion)) {
                    roll /= 2;
                    if g.player_is_no_non_state_religion_spread(owner) {
                        roll /= 2;
                    }
                }

                if roll > self.rand(100, "AI choose missionary") {
                    let religion_value = self.player_ai_missionary_value(g, owner, area, religion, &mut None);
                    if religion_value > 0 {
                        for class in d.unit_classes.keys() {
                            let Some(loop_unit) = civ_unit(g, civ, class) else {
                                continue;
                            };
                            let unit_info = &d.units[loop_unit];
                            if unit_info.religion_spreads[religion] > 0 {
                                if g.city_can_train(c, loop_unit, false, false, false, false) {
                                    let mut value = religion_value;
                                    value = sdiv(value, unit_info.production_cost);

                                    if value > best_value {
                                        best_value = value;
                                        *best_spread_unit = Some(loop_unit);
                                        *best_spread_unit_value = religion_value;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if executive {
            for corporation in d.corporations.keys() {
                if !g.city_is_active_corporation(c, corporation) {
                    continue;
                }
                let has_count = g.players[owner].has_corporation_count[corporation];
                // PORT ADAPT: guard the division (C++ asserts iHasCount > 0).
                let mut roll = if has_count > 4 { base_chance } else { ((100 - base_chance) / has_count.max(1)) + base_chance };
                if !g.team_has_headquarters(team, corporation) {
                    roll /= 8;
                }

                if roll > self.rand(100, "AI choose executive") {
                    let mut corporation_value = self.player_ai_executive_value(g, owner, area, corporation, &mut None);
                    if corporation_value > 0 {
                        for class in d.unit_classes.keys() {
                            let Some(loop_unit) = civ_unit(g, civ, class) else {
                                continue;
                            };
                            let unit_info = &d.units[loop_unit];
                            if unit_info.corporation_spreads[corporation] > 0 {
                                if g.city_can_train(c, loop_unit, false, false, false, false) {
                                    let mut value = corporation_value;
                                    value = sdiv(value, unit_info.production_cost);

                                    let mut total_count = 0;
                                    let mut plot_count = 0;
                                    for &u in &g.players[owner].units {
                                        if g.units[u].ai_type == Some(UnitAIType::UNITAI_MISSIONARY) && d.units[g.units[u].unit_type].corporation_spreads[corporation] > 0 {
                                            total_count += 1;
                                            if g.unit_plot(u) == city_plot {
                                                plot_count += 1;
                                            }
                                        }
                                    }
                                    // C++ quirk: iCorporationValue is divided again for every
                                    // executive unit type checked.
                                    corporation_value /= 1.max((total_count / 4) + plot_count);

                                    let mut cost = 0.max(d.corporations[corporation].spread_cost * (100 + g.player_calculate_inflation_rate(owner)));
                                    cost /= 100;

                                    let gold = g.players[owner].gold;
                                    if gold >= cost {
                                        cost *= d.defines.corporation_foreign_spread_cost_percent;
                                        cost /= 100;
                                        if gold < cost && total_count > 1 {
                                            corporation_value /= 2;
                                        }
                                    } else if total_count > 1 {
                                        corporation_value /= 5;
                                    }
                                    if value > best_value {
                                        best_value = value;
                                        *best_spread_unit = Some(loop_unit);
                                        *best_spread_unit_value = corporation_value;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // FIX: C++ returns `*eBestSpreadUnit != NULL` (compares the unit type with 0, so it is
        // true even when nothing was found); callers only use the unit when one was found.
        best_spread_unit.is_some()
    }

    /// C++ `bool CvCityAI::AI_chooseBuilding(int iFocusFlags = 0, int iMaxTurns = MAX_INT, int iMinThreshold = 0)` (CvCityAI.cpp:6684-6697)
    pub fn city_ai_choose_building(&self, g: &mut Game, c: CityId, focus_flags: i32, max_turns: i32, min_threshold: i32) -> bool {
        if let Some(building) = self.city_ai_best_building_threshold(g, c, focus_flags, max_turns, min_threshold, false, None) {
            g.city_push_order(c, OrderType::ORDER_CONSTRUCT, building.i(), -1, false, false, false, false);
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_chooseProject()` (CvCityAI.cpp:6700-6713)
    pub fn city_ai_choose_project(&self, g: &mut Game, c: CityId) -> bool {
        if let Some(project) = self.city_ai_best_project(g, c) {
            g.city_push_order(c, OrderType::ORDER_CREATE, project.i(), -1, false, false, false, false);
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_chooseProcess(CommerceTypes eCommerceType = NO_COMMERCE)` (CvCityAI.cpp:6716-6729)
    pub fn city_ai_choose_process(&self, g: &mut Game, c: CityId, commerce_type: Option<CommerceType>) -> bool {
        let best_process = match commerce_type {
            Some(ct) => self.city_ai_best_process_2(g, c, ct),
            None => self.city_ai_best_process(g, c),
        };

        if let Some(process) = best_process {
            g.city_push_order(c, OrderType::ORDER_MAINTAIN, process.i(), -1, false, false, false, false);
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_addBestCitizen(bool bWorkers, bool bSpecialists, int* piBestPlot = NULL, SpecialistTypes* peBestSpecialist = NULL)` (CvCityAI.cpp:6733-6878)
    pub fn city_ai_add_best_citizen(&self, g: &mut Game, c: CityId, workers: bool, specialists: bool, best_plot: &mut i32, best_specialist: &mut Option<SpecialistType>) -> bool {
        let d = g.d;
        let avoid_growth = self.city_ai_avoid_growth(g, c);
        let ignore_growth = self.city_ai_ignore_growth(g, c);
        let mut is_specialist_forced = false;

        let mut best_specialist_value = 0;
        let mut e_best_specialist: Option<SpecialistType> = None;
        let mut best_forced_specialist: Option<SpecialistType> = None;

        if specialists {
            // count the total forced specialists
            let mut total_forced_specialists = 0;
            for sp in d.specialists.keys() {
                let forced = g.cities[c].force_specialist_count[sp];
                if forced > 0 {
                    is_specialist_forced = true;
                    total_forced_specialists += forced;
                }
            }

            // if forcing any specialists, find the best one that we can still assign
            if is_specialist_forced {
                let mut best_forced_value = i32::MIN;

                let total_specialists = 1 + g.cities[c].specialist_population;
                for sp in d.specialists.keys() {
                    if g.city_is_specialist_valid(c, sp, 1) {
                        let forced = g.cities[c].force_specialist_count[sp];
                        if forced > 0 {
                            let count = g.cities[c].specialist_count[sp];

                            // the value is based on how close we are to our goal ratio forced/total
                            let forced_value = ((forced * 128) / total_forced_specialists) - ((count * 128) / total_specialists);
                            if forced_value >= best_forced_value {
                                let specialist_value = self.city_ai_specialist_value(g, c, sp, avoid_growth, false);

                                // if forced value larger, or if equal, does this specialist have a higher value
                                if forced_value > best_forced_value || specialist_value > best_specialist_value {
                                    best_forced_value = forced_value;
                                    best_specialist_value = specialist_value;
                                    best_forced_specialist = Some(sp);
                                    e_best_specialist = best_forced_specialist;
                                }
                            }
                        }
                    }
                }
            }

            // if we do not have a best specialist yet, then just find the one with the best value
            if e_best_specialist.is_none() {
                for sp in d.specialists.keys() {
                    if g.city_is_specialist_valid(c, sp, 1) {
                        let value = self.city_ai_specialist_value(g, c, sp, avoid_growth, false);
                        if value >= best_specialist_value {
                            best_specialist_value = value;
                            e_best_specialist = Some(sp);
                        }
                    }
                }
            }
        }

        let mut best_plot_value = 0;
        let mut i_best_plot: i32 = -1;
        if workers {
            for i in 0..NUM_CITY_PLOTS {
                if i == CITY_HOME_PLOT {
                    continue;
                }
                if g.cities[c].working_plot[i] {
                    continue;
                }
                let Some(pl) = g.city_index_plot(c, i) else {
                    continue;
                };
                if g.city_can_work(c, pl) {
                    let value = self.city_ai_plot_value(g, c, pl, avoid_growth, false, false, ignore_growth, false);

                    if value > best_plot_value {
                        best_plot_value = value;
                        i_best_plot = i as i32;
                    }
                }
            }
        }

        // if we found a plot to work
        if i_best_plot != -1 {
            // if the best plot value is better than the best specialist, or if we forcing and we could not assign a forced specialst
            if best_plot_value > best_specialist_value || (is_specialist_forced && best_forced_specialist.is_none()) {
                // do not work the specialist
                e_best_specialist = None;
            }
        }

        if let Some(sp) = e_best_specialist {
            g.city_change_specialist_count(c, sp, 1);
            *best_specialist = Some(sp);
            *best_plot = -1;
            return true;
        } else if i_best_plot != -1 {
            g.city_set_working_plot(c, i_best_plot as usize, true);
            *best_specialist = None;
            *best_plot = i_best_plot;
            return true;
        }

        false
    }

    /// C++ `bool CvCityAI::AI_removeWorstCitizen(SpecialistTypes eIgnoreSpecialist = NO_SPECIALIST)` (CvCityAI.cpp:6882-7003)
    pub fn city_ai_remove_worst_citizen(&self, g: &mut Game, c: CityId, ignore_specialist: Option<SpecialistType>) -> bool {
        let d = g.d;
        let default_specialist = opt_from_i32::<SpecialistType>(d.defines.default_specialist);

        // if we are using more specialists than the free ones we get
        if g.city_extra_free_specialists(c) < 0 {
            // does generic 'citizen' specialist exist?
            if let Some(ds) = default_specialist {
                // is ignore something other than generic citizen?
                if ignore_specialist != Some(ds) {
                    // do we have at least one more generic citizen than we are forcing?
                    if g.cities[c].specialist_count[ds] > g.cities[c].force_specialist_count[ds] {
                        // remove the extra generic citzen
                        g.city_change_specialist_count(c, ds, -1);
                        return true;
                    }
                }
            }
        }

        let avoid_growth = self.city_ai_avoid_growth(g, c);
        let ignore_growth = self.city_ai_ignore_growth(g, c);

        let mut worst_value = i32::MAX;
        let mut worst_specialist: Option<SpecialistType> = None;
        let mut worst_plot: Option<usize> = None;

        // if we are using more specialists than the free ones we get
        if g.city_extra_free_specialists(c) < 0 {
            for sp in d.specialists.keys() {
                if ignore_specialist != Some(sp) {
                    if g.cities[c].specialist_count[sp] > g.cities[c].force_specialist_count[sp] {
                        let value = self.city_ai_specialist_value(g, c, sp, avoid_growth, true);

                        if value < worst_value {
                            worst_value = value;
                            worst_specialist = Some(sp);
                            worst_plot = None;
                        }
                    }
                }
            }
        }

        // check all the plots we working
        for i in 0..NUM_CITY_PLOTS {
            if i == CITY_HOME_PLOT {
                continue;
            }
            if !g.cities[c].working_plot[i] {
                continue;
            }
            let Some(pl) = g.city_index_plot(c, i) else {
                continue;
            };
            let value = self.city_ai_plot_value(g, c, pl, avoid_growth, true, false, ignore_growth, false);

            if value < worst_value {
                worst_value = value;
                worst_specialist = None;
                worst_plot = Some(i);
            }
        }

        if let Some(sp) = worst_specialist {
            g.city_change_specialist_count(c, sp, -1);
            return true;
        } else if let Some(i) = worst_plot {
            g.city_set_working_plot(c, i, false);
            return true;
        }

        // if we still have not removed one, then try again, but do not ignore the one we were told to ignore
        if g.city_extra_free_specialists(c) < 0 {
            for sp in d.specialists.keys() {
                if g.cities[c].specialist_count[sp] > 0 {
                    let value = self.city_ai_specialist_value(g, c, sp, avoid_growth, true);

                    if value < worst_value {
                        worst_value = value;
                        worst_specialist = Some(sp);
                        worst_plot = None;
                    }
                }
            }
        }

        if let Some(sp) = worst_specialist {
            g.city_change_specialist_count(c, sp, -1);
            return true;
        }

        false
    }

    /// C++ `void CvCityAI::AI_juggleCitizens()` (CvCityAI.cpp:7006-7084)
    pub fn city_ai_juggle_citizens(&self, g: &mut Game, c: CityId) {
        let avoid_growth = self.city_ai_avoid_growth(g, c);
        let ignore_growth = self.city_ai_ignore_growth(g, c);

        // one at a time, remove the worst citizen, then add the best citizen
        // until we add back the same one we removed
        for pass in 0..2 {
            let mut completed_checks = false;
            let mut count = 0;

            let mut worst_plots: Vec<usize> = Vec::new();

            while !completed_checks {
                let mut lowest_value = i32::MAX;
                let mut worst_plot: Option<usize> = None;

                for i in 0..NUM_CITY_PLOTS {
                    if i == CITY_HOME_PLOT {
                        continue;
                    }
                    if !g.cities[c].working_plot[i] {
                        continue;
                    }
                    let Some(pl) = g.city_index_plot(c, i) else {
                        continue;
                    };
                    let value = self.city_ai_plot_value(g, c, pl, avoid_growth, true, false, ignore_growth, pass == 0);

                    // use <= so that we pick the last one that is lowest, to avoid infinite loop with AI_addBestCitizen
                    if value <= lowest_value {
                        lowest_value = value;
                        worst_plot = Some(i);
                    }
                }

                // if no worst plot, or we looped back around and are trying to remove the first plot we removed, stop
                match worst_plot {
                    Some(wp) if !worst_plots.contains(&wp) => {
                        // if this the first worst plot, remember it
                        worst_plots.push(wp);

                        g.city_set_working_plot(c, wp, false);

                        if self.city_ai_add_best_citizen(g, c, true, true, &mut -1, &mut None) {
                            if g.cities[c].working_plot[wp] {
                                completed_checks = true;
                            }
                        }
                    }
                    _ => {
                        completed_checks = true;
                    }
                }

                count += 1;
                if count > (NUM_CITY_PLOTS as i32 + 1) {
                    break; // XXX
                }
            }

            if pass == 0 && g.city_food_difference(c, false) >= 0 {
                //good enough, the starvation code
                break;
            }
        }
    }

    /// C++ `bool CvCityAI::AI_potentialPlot(short* piYields)` (CvCityAI.cpp:7087-7110)
    pub fn city_ai_potential_plot(&self, g: &Game, c: CityId, yields: &[i32]) -> bool {
        let food = yields[YieldType::YIELD_FOOD.idx()];
        let prod = yields[YieldType::YIELD_PRODUCTION.idx()];
        let comm = yields[YieldType::YIELD_COMMERCE.idx()];
        let net_food = food - g.d.defines.food_consumption_per_population;

        if net_food < 0 {
            if food == 0 {
                if prod + comm < 2 {
                    return false;
                }
            } else if prod + comm == 0 {
                return false;
            }
        }

        true
    }

    /// C++ `bool CvCityAI::AI_foodAvailable(int iExtra = 0)` (CvCityAI.cpp:7113-7208)
    pub fn city_ai_food_available(&self, g: &Game, c: CityId, extra: i32) -> bool {
        let d = g.d;
        let mut plot_available = [false; NUM_CITY_PLOTS];
        let mut food_count = 0;

        // FfH: Modified by Kael 01/31/2009 (getNumCityPlots)
        let num_city_plots = (g.city_num_city_plots(c).max(0) as usize).min(NUM_CITY_PLOTS);

        for i in 0..num_city_plots {
            let Some(pl) = g.city_index_plot(c, i) else {
                continue;
            };
            if i == CITY_HOME_PLOT {
                food_count += g.map.plots[pl].yields[YieldType::YIELD_FOOD.idx()];
            } else if g.map.plots[pl].working_city == Some(c) && self.city_ai_potential_plot(g, c, &g.map.plots[pl].yields) {
                plot_available[i] = true;
            }
        }

        let mut population = g.cities[c].population + extra;

        while population > 0 {
            let mut best_value = 0;
            let mut best_plot = CITY_HOME_PLOT;

            for i in 0..num_city_plots {
                if plot_available[i] {
                    let Some(pl) = g.city_index_plot(c, i) else {
                        continue;
                    };
                    let value = g.map.plots[pl].yields[YieldType::YIELD_FOOD.idx()];

                    if value > best_value {
                        best_value = value;
                        best_plot = i;
                    }
                }
            }

            if best_plot != CITY_HOME_PLOT {
                food_count += best_value;
                plot_available[best_plot] = false;
            } else {
                break;
            }

            population -= 1;
        }

        for sp in d.specialists.keys() {
            food_count += d.specialists[sp].yield_change[YieldType::YIELD_FOOD.idx()] * g.cities[c].free_specialist_count[sp];
        }

        if food_count < g.city_food_consumption(c, false, extra) {
            return false;
        }

        true
    }

    /// C++ `int CvCityAI::AI_yieldValue(short* piYields, short* piCommerceYields, bool bAvoidGrowth, bool bRemove, bool bIgnoreFood = false, bool bIgnoreGrowth = false, bool bIgnoreStarvation = false, bool bWorkerOptimization = false)` (CvCityAI.cpp:7211-7778)
    ///
    /// `commerce_yields` empty = C++ `NULL`.
    pub fn city_ai_yield_value(&self, g: &Game, c: CityId, yields: &[i32], commerce_yields: &[i32], avoid_growth: bool, remove: bool, ignore_food: bool, ignore_growth: bool, ignore_starvation: bool, worker_optimization: bool) -> i32 {
        let d = g.d;
        let owner = g.cities[c].owner;
        const BASE_PRODUCTION_VALUE: i32 = 15;
        const BASE_COMMERCE_VALUE: i32 = 7;

        const MAX_FOOD_VALUE: i32 = (3 * BASE_PRODUCTION_VALUE) - 1;

        let food_i = YieldType::YIELD_FOOD.idx();
        let prod_i = YieldType::YIELD_PRODUCTION.idx();
        let comm_i = YieldType::YIELD_COMMERCE.idx();

        let mut a_yields = [0i32; YieldType::NUM];
        let mut commerce_yields_times100 = [0i32; CommerceType::NUM];

        let mut extra_production_modifier = 0;
        let mut base_production_modifier = 100;

        let emphasize_food = self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_FOOD);
        let food_is_production = g.city_is_food_production(c);
        let can_pop_rush = g.player_can_pop_rush(owner);

        for i in 0..YieldType::NUM {
            if yields[i] == 0 {
                a_yields[i] = 0;
            } else {
                // Get yield for city after adding/removing the citizen in question
                let mut old_city_yield = g.cities[c].base_yield_rate[i];
                let mut new_city_yield = if remove { old_city_yield - yields[i] } else { old_city_yield + yields[i] };
                let mut modifier = g.city_base_yield_rate_modifier(c, YieldType::from_usize(i), 0);
                if i == prod_i {
                    base_production_modifier = modifier;
                    extra_production_modifier = g.city_production_modifier(c);
                    modifier += extra_production_modifier;
                }

                new_city_yield = (new_city_yield * modifier) / 100;
                old_city_yield = (old_city_yield * modifier) / 100;

                // The yield of the citizen in question is the difference of total yields
                // to account for rounding of modifiers
                a_yields[i] = if remove { old_city_yield - new_city_yield } else { new_city_yield - old_city_yield };
            }
        }

        for j in 0..CommerceType::NUM {
            let modifier = g.city_total_commerce_rate_modifier(c, CommerceType::from_usize(j));

            let mut commerce_times100 = a_yields[comm_i] * g.players[owner].commerce_percent[j];
            if !commerce_yields.is_empty() {
                commerce_times100 += commerce_yields[j] * 100;
            }
            commerce_yields_times100[j] += (commerce_times100 * modifier) / 100;
        }

        // UNOFFICIAL_PATCH 07/09/09 jdog5000: the production process conversion of the original
        // BTS code is disabled.

        // should not really use this much, but making it accurate
        a_yields[comm_i] = 0;
        for j in 0..CommerceType::NUM {
            a_yields[comm_i] += commerce_yields_times100[j] / 100;
        }

        let mut value = 0;
        let mut slavery_value = 0;

        let mut food_growth_value = 0;
        let mut food_gpp_value = 0;

        if !ignore_food && a_yields[food_i] != 0 {
            // tiny food factor, to ensure that even when we don't want to grow,
            // we still prefer more food if everything else is equal
            value += a_yields[food_i] * 1;

            let mut food_per_turn = g.city_food_difference(c, false) - if remove { a_yields[food_i] } else { 0 };
            let food_level = g.cities[c].food;
            let food_to_grow = g.city_growth_threshold(c);
            let mut health_level = g.city_good_health(c) - g.city_bad_health(c, false, 0);
            let mut happiness_level = if g.city_is_no_unhappiness(c) { 3.max(health_level + 5) } else { g.city_happy_level(c) - g.city_unhappy_level(c, 0) };
            let population = g.cities[c].population;
            let extra_population_that_can_work = (population - range(-happiness_level, 0, population) + 0.min(g.city_extra_free_specialists(c))).min(NUM_CITY_PLOTS as i32) - g.cities[c].working_population + if remove { 1 } else { 0 };
            let consumption_per_pop = d.defines.food_consumption_per_population;

            let adjusted_food_difference = (g.city_yield_rate(c, YieldType::YIELD_FOOD) + 0.min(health_level)) - ((population + 0.min(happiness_level) - if remove { 1 } else { 0 }) * consumption_per_pop);

            // if we not human, allow us to starve to half full if avoiding growth
            if !ignore_starvation {
                let mut starving_allowance = 0;
                if avoid_growth && !g.city_is_human(c) {
                    starving_allowance = 0.max(food_level - 1.max((9 * food_to_grow) / 10));
                }

                if starving_allowance < 1 && food_level > ((food_to_grow * 75) / 100) {
                    starving_allowance = 1;
                }

                // if still starving
                if (food_per_turn + starving_allowance) < 0 {
                    // if working plots all like this one will save us from starving
                    if 0.max(extra_population_that_can_work * a_yields[food_i]) >= -food_per_turn {
                        // if this is high food, then we want to pick it first, this will allow us to pick some great non-food later
                        let high_food_threshold = g.city_best_yield_available(c, YieldType::YIELD_FOOD).min(consumption_per_pop + 1);
                        if food_per_turn <= (if self.city_ai_is_emphasize_great_people(g, c) { 0 } else { -high_food_threshold }) && a_yields[food_i] >= high_food_threshold {
                            // value all the food that will contribute to not starving
                            value += 2048 * a_yields[food_i].min(-food_per_turn);
                        } else {
                            // give a huge boost to this plot, but not based on how much food it has
                            // ie, if working a bunch of 1f 7h plots will stop us from starving, then do not force working unimproved 2f plot
                            value += 2048;
                        }
                    } else {
                        // value food high(32), but not forced
                        value += 32 * a_yields[food_i].min(-food_per_turn);
                    }
                }
            }

            // if food isnt production, then adjust for growth
            if worker_optimization || !food_is_production {
                // C++ quirk: this outer iPopToGrow is shadowed by the one declared in the growth
                // block below and stays 0 for the "very high food override".
                let outer_pop_to_grow = 0;
                if !avoid_growth {
                    // only do relative checks on food if we want to grow AND we not emph food
                    // the emp food case will just give a big boost to all food under all circumstances
                    if worker_optimization || !ignore_growth {
                        // adjust iFoodPerTurn assuming that we work plots all equal to iConsumtionPerPop
                        // that way it is our guesstimate of how much excess food we will have
                        food_per_turn += extra_population_that_can_work * consumption_per_pop;

                        // we have less than 10 extra happy, do some checks to see if we can increase it
                        if happiness_level < 10 {
                            // if we have anger becase no military, do not count it, on the assumption that it will
                            // be remedied soon, and that we still want to grow
                            if g.cities[c].military_happiness_units == 0 {
                                if g.players[owner].num_cities() > 2 {
                                    happiness_level += sdiv(d.defines.no_military_percent_anger * (population + 1), d.defines.percent_anger_divisor);
                                }
                            }

                            // currently we can at most increase happy by 2 in the following checks
                            const MAX_HAPPY_INCREASE: i32 = 2;

                            // if happy is large enough so that it will be over zero after we do the checks
                            let new_food_per_turn = food_per_turn + a_yields[food_i] - consumption_per_pop;
                            if (happiness_level + MAX_HAPPY_INCREASE) > 0 && new_food_per_turn > 0 {
                                let approx_turns_to_grow = if new_food_per_turn > 0 { (food_to_grow - food_level) / new_food_per_turn } else { i32::MAX };

                                // do we have hurry anger?
                                let hurry_anger_timer = g.cities[c].hurry_anger_timer;
                                if hurry_anger_timer > 0 {
                                    let turns_until_anger_is_reduced = smod(hurry_anger_timer, g.city_flat_hurry_anger_length(c));

                                    // angry population is bad but if we'll recover by the time we grow...
                                    if turns_until_anger_is_reduced <= approx_turns_to_grow {
                                        happiness_level += 1;
                                    }
                                }

                                // do we have conscript anger?
                                let conscript_anger_timer = g.cities[c].conscript_anger_timer;
                                if conscript_anger_timer > 0 {
                                    let turns_until_anger_is_reduced = smod(conscript_anger_timer, g.city_flat_conscript_anger_length(c));

                                    // angry population is bad but if we'll recover by the time we grow...
                                    if turns_until_anger_is_reduced <= approx_turns_to_grow {
                                        happiness_level += 1;
                                    }
                                }

                                // do we have defy resolution anger?
                                let defy_resolution_anger_timer = g.cities[c].defy_resolution_anger_timer;
                                if defy_resolution_anger_timer > 0 {
                                    let turns_until_anger_is_reduced = smod(defy_resolution_anger_timer, g.city_flat_defy_resolution_anger_length(c));

                                    // angry population is bad but if we'll recover by the time we grow...
                                    if turns_until_anger_is_reduced <= approx_turns_to_grow {
                                        happiness_level += 1;
                                    }
                                }
                            }
                        }

                        if emphasize_food {
                            //If we are emphasize food, pay less heed to caps.
                            health_level += 5;
                            happiness_level += 2;
                        }

                        let bar_full = food_level + food_per_turn /*+ aiYields[YIELD_FOOD]*/ > ((90 * food_to_grow) / 100);

                        let mut pop_to_grow = 0.max(happiness_level);
                        let mut good_tiles = self.city_ai_count_good_tiles(g, c, health_level > 0, true, 50, true);
                        good_tiles += self.city_ai_count_good_specialists(g, c, health_level > 0);
                        good_tiles += if bar_full { 0 } else { 1 };

                        if !emphasize_food {
                            pop_to_grow = pop_to_grow.min(good_tiles + if remove { 1 } else { 0 });
                        }

                        // if we have growth pontential, fill food bar to 85%
                        let mut filling_bar = false;
                        if pop_to_grow == 0 && happiness_level >= 0 && good_tiles >= 0 && health_level >= 0 {
                            if !bar_full {
                                if self.city_ai_special_yield_multiplier(g, c, YieldType::YIELD_PRODUCTION) < 50 {
                                    filling_bar = true;
                                }
                            }
                        }

                        if population < 3 {
                            pop_to_grow = pop_to_grow.max(3 - population);
                            pop_to_grow += 2;
                        }

                        // if we want to grow
                        if pop_to_grow > 0 || filling_bar {
                            // will multiply this by factors
                            food_growth_value = a_yields[food_i];
                            if health_level < (if filling_bar { 0 } else { 1 }) {
                                food_growth_value -= 1;
                            }

                            // (range 1-25) - we want to grow more if we have a lot of growth to do
                            // factor goes up like this: 0:1, 1:8, 2:9, 3:10, 4:11, 5:13, 6:14, 7:15, 8:16, 9:17, ... 17:25
                            let factor_pop_to_grow = if pop_to_grow < 1 || filling_bar {
                                20 - sdiv(10 * (food_level + food_per_turn + a_yields[food_i]), food_to_grow)
                            } else if pop_to_grow < 7 {
                                17 + 3 * pop_to_grow
                            } else {
                                41
                            };

                            food_growth_value *= factor_pop_to_grow;

                            //If we already grow somewhat fast, devalue further food
                            //Remember growth acceleration is not dependent on food eaten per
                            //pop, 4f twice as fast as 2f twice as fast as 1f...
                            let mut high_growth_threshold = 2 + (0.max(5 - population)).max((pop_to_grow + 1) / 2);
                            if emphasize_food {
                                high_growth_threshold *= 2;
                            }

                            if food_per_turn > high_growth_threshold {
                                food_growth_value *= 25 + ((75 * high_growth_threshold) / food_per_turn);
                                food_growth_value /= 100;
                            }
                        }
                    }

                    //very high food override
                    if g.city_is_human(c) && (outer_pop_to_grow > 0 || can_pop_rush) {
                        //very high food override
                        let mut temp_value = 0.max(30 * a_yields[food_i] - 15 * consumption_per_pop);
                        temp_value *= 0.max(3 * consumption_per_pop - adjusted_food_difference);
                        temp_value = sdiv(temp_value, 3 * consumption_per_pop);
                        if happiness_level < 0 {
                            temp_value *= 2;
                            temp_value /= 1 + 2 * -happiness_level;
                        }
                        food_growth_value += temp_value;
                    }
                    //Slavery Override
                    if can_pop_rush && happiness_level > 0 {
                        slavery_value = 30 * 14 * 0.max(a_yields[food_i] - if health_level < 0 { 1 } else { 0 });
                        slavery_value /= 10.max(g.city_growth_threshold(c) * (100 - g.cities[c].max_food_kept_percent));

                        slavery_value *= 100;
                        // Xienwolf Tweak 03/27/09 (merged Sephi): prevent division by zero.
                        slavery_value /= 1.max(g.city_hurry_cost_modifier(c, true));

                        slavery_value *= consumption_per_pop * 2;
                        slavery_value /= 1.max(consumption_per_pop * 2 + 0.max(adjusted_food_difference));
                    }

                    //Great People Override
                    if extra_population_that_can_work > 1 && self.city_ai_is_emphasize_great_people(g, c) {
                        let mut adjust = consumption_per_pop;
                        if food_per_turn == 0 {
                            adjust -= 1;
                        }
                        food_gpp_value += 0.max(a_yields[food_i] - adjust) * 0.max(12 + 5 * 0.min(happiness_level));
                    }
                }
            }
        }

        let mut production_value = 0;
        let mut commerce_value = 0;
        let mut food_value = food_growth_value.min(MAX_FOOD_VALUE * a_yields[food_i]);
        // if food is production, the count it
        let adjusted_yield_production = (if food_is_production { a_yields[food_i] } else { 0 }) + a_yields[prod_i];

        // value production medium(15)
        production_value += adjusted_yield_production * BASE_PRODUCTION_VALUE;
        if !g.city_is_production(c) && !g.city_is_human(c) {
            production_value /= 2;
        }
        // value commerce low(6)

        // BETTER_BTS_AI_MOD 05/18/09 jdog5000 (City AI)
        // If city has more than enough food, but very little production, add large value to production
        // Particularly helps coastal cities with plains forests
        if a_yields[prod_i] > 0 {
            if !food_is_production && g.city_is_production(c) {
                if g.city_food_difference(c, false) >= d.defines.food_consumption_per_population {
                    if g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) < (1 + g.cities[c].population / 3) {
                        value += 128 + 8 * a_yields[prod_i];
                    }
                }
            }
        }

        for i in 0..CommerceType::NUM {
            if commerce_yields_times100[i] != 0 {
                let ct = CommerceType::from_usize(i);
                let mut commerce_weight = self.player_ai_commerce_weight(g, owner, ct, None);
                if self.city_ai_is_emphasize_commerce(g, c, ct) {
                    commerce_weight *= 200;
                    commerce_weight /= 100;
                }
                if ct == CommerceType::COMMERCE_CULTURE {
                    if g.cities[c].culture_level.map_or(true, |l| l.i() <= 1) {
                        commerce_value += (15 * commerce_yields_times100[i]) / 100;
                    }
                }
                // C++ int arithmetic (wraps on overflow).
                commerce_value += commerce_weight.wrapping_mul(commerce_yields_times100[i].wrapping_mul(BASE_COMMERCE_VALUE)).wrapping_mul(self.player_ai_average_commerce_exchange(g, owner, ct)) / 1000000;
            }
        }

        //Slavery translation
        if slavery_value > 0 && slavery_value > food_value {
            //treat the food component as production
            food_value = 0;
        } else {
            //treat it as just food
            slavery_value = 0;
        }

        food_value += food_gpp_value;

        //Lets have some fun with the multipliers, this basically bluntens the impact of
        //massive bonuses.....

        //normalize the production... this allows the system to account for rounding
        //and such while preventing an "out to lunch smoking weed" scenario with
        //unusually high transient production modifiers.
        //Other yields don't have transient bonuses in quite the same way.

        let emph_prod = self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_PRODUCTION);
        let emph_comm = self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_COMMERCE);

        if emph_prod {
            production_value *= 130;
            production_value /= 100;

            if food_is_production {
                food_value *= 130;
                food_value /= 100;
            }

            if !emph_comm {
                commerce_value *= 60;
                commerce_value /= 100;
            }
            if !emphasize_food {
                food_value *= 75;
                food_value /= 100;
            }
        }
        if emphasize_food {
            if !food_is_production {
                food_value *= 130;
                food_value /= 100;
                slavery_value *= 130;
                slavery_value /= 100;
            }
        }
        if emph_comm {
            commerce_value *= 130;
            commerce_value /= 100;
            if !emph_prod {
                production_value *= 75;
                production_value /= 100;
            }
            if !emphasize_food {
                //Don't supress twice.
                if !emph_prod {
                    food_value *= 80;
                    food_value /= 100;
                }
            }
        }

        if food_is_production {
            production_value *= 100 + if worker_optimization { 0 } else { self.city_ai_special_yield_multiplier(g, c, YieldType::YIELD_PRODUCTION) };
            production_value /= 100;
        } else {
            production_value *= base_production_modifier;
            production_value = sdiv(production_value, base_production_modifier + extra_production_modifier);

            production_value += slavery_value;
            production_value *= 100 + if worker_optimization { 0 } else { self.city_ai_special_yield_multiplier(g, c, YieldType::YIELD_PRODUCTION) };

            production_value = sdiv(production_value, self.player_ai_average_yield_multiplier(g, owner, YieldType::YIELD_PRODUCTION));
        }

        value += production_value;

        commerce_value *= 100 + if worker_optimization { 0 } else { self.city_ai_special_yield_multiplier(g, c, YieldType::YIELD_COMMERCE) };
        commerce_value = sdiv(commerce_value, self.player_ai_average_yield_multiplier(g, owner, YieldType::YIELD_COMMERCE));
        value += commerce_value;

        food_value *= 100;
        food_value = sdiv(food_value, self.player_ai_average_yield_multiplier(g, owner, YieldType::YIELD_FOOD));
        value += food_value;

        value
    }

    /// C++ `int CvCityAI::AI_plotValue(CvPlot* pPlot, bool bAvoidGrowth, bool bRemove, bool bIgnoreFood = false, bool bIgnoreGrowth = false, bool bIgnoreStarvation = false)` (CvCityAI.cpp:7781-7862)
    pub fn city_ai_plot_value(&self, g: &Game, c: CityId, plot: PlotIdx, avoid_growth: bool, remove: bool, ignore_food: bool, ignore_growth: bool, ignore_starvation: bool) -> i32 {
        let d = g.d;
        let owner = g.cities[c].owner;
        let team = g.city_team(c);

        let mut value = 0;

        let mut yields = g.map.plots[plot].yields;

        let current_improvement = g.map.plots[plot].improvement_type;
        let final_improvement = current_improvement.and_then(|imp| final_improvement_upgrade(d, imp, 0));

        let mut yield_value = self.city_ai_yield_value(g, c, &yields, &[], avoid_growth, remove, ignore_food, ignore_growth, ignore_starvation, false) * 100;

        if let (Some(fin), Some(cur)) = (final_improvement, current_improvement) {
            for i in 0..YieldType::NUM {
                let y = YieldType::from_usize(i);
                let yield_diff = g.plot_calculate_improvement_yield_change(plot, fin, y, Some(owner), false, false) - g.plot_calculate_improvement_yield_change(plot, cur, y, Some(owner), false, false);
                yields[i] += yield_diff;
            }
            let final_yield_value = self.city_ai_yield_value(g, c, &yields, &[], avoid_growth, remove, ignore_food, ignore_growth, ignore_starvation, false) * 100;

            if final_yield_value > yield_value {
                yield_value = (40 * yield_value + 60 * final_yield_value) / 100;
            } else {
                yield_value = (60 * yield_value + 40 * final_yield_value) / 100;
            }
        }
        // unless we are emph food (and also food not production)
        if !(self.city_ai_is_emphasize_yield(g, c, YieldType::YIELD_FOOD) && !g.city_is_food_production(c)) {
            // if this plot is super bad (less than 2 food and less than combined 2 prod/commerce
            if !self.city_ai_potential_plot(g, c, &yields) {
                // undervalue it even more!
                yield_value /= 16;
            }
        }
        value += yield_value;

        if let Some(cur) = current_improvement {
            if g.plot_get_bonus_type(plot, Some(team)).is_none() {
                // XXX double-check CvGame::doFeature that the checks are the same...
                for b in d.bonuses.keys() {
                    if g.team_is_has_tech(team, d.bonuses[b].tech_reveal) {
                        if d.improvements[cur].improvement_bonus[b].discover_rand > 0 {
                            value += 35;
                        }
                    }
                }
            }

            if d.improvements[cur].improvement_upgrade.is_some() {
                value += 200;
                value -= g.plot_get_upgrade_time_left(plot, cur, None);
            }
        }

        value
    }

    /// C++ `int CvCityAI::AI_experienceWeight()` (CvCityAI.cpp:7865-7868)
    pub fn city_ai_experience_weight(&self, g: &Game, c: CityId) -> i32 {
        (g.city_production_experience(c, None) + g.cities[c].domain_free_experience[DomainType::DOMAIN_SEA.idx()]) * 2
    }

    /// C++ `int CvCityAI::AI_buildUnitProb()` (CvCityAI.cpp:7871-7883)
    pub fn city_ai_build_unit_prob(&self, g: &Game, c: CityId) -> i32 {
        let mut prob = g.d.leader_heads[g.city_personality_type(c)].build_unit_prob + self.city_ai_experience_weight(g, c);

        if self.player_ai_is_financial_trouble(g, g.cities[c].owner) {
            prob /= 2;
        }

        prob
    }
}
