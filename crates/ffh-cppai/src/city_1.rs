//! Port of CvCityAI.cpp:69-2746 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `city_ai_count_num_bonuses` (city_4.rs) takes `bonus: BonusType`, but C++ callers pass
//!   `NO_BONUS`; it should be `Option<BonusType>`. Until then this file uses the private
//!   [`count_num_bonuses_any`] (a port of `AI_countNumBonuses` for `eBonus == NO_BONUS`).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvCityAI::AI_init()` (CvCityAI.cpp:69-80)
    pub fn city_ai_init(&self, g: &mut Game, c: CityId) {
        self.city_ai_reset(g, c);

        //--------------------------------
        // Init other game data
        self.city_ai_assign_working_plots(g, c);

        self.city_ai_update_workers_needed_here(g, c);

        self.city_ai_update_best_build(g, c);
    }

    /// C++ `void CvCityAI::AI_reset()` (CvCityAI.cpp:91-190)
    pub fn city_ai_reset(&self, g: &mut Game, c: CityId) {
        // Members ffh-core keeps.
        {
            let city = &mut g.cities[c];
            city.ai_emphasize_avoid_growth_count = 0;
            city.ai_emphasize_great_people_count = 0;
            city.ai_assign_work_dirty = false;
            city.ai_choose_production_dirty = false;
            city.ai_emphasize_yield_count = [0; YieldType::NUM];
            city.ai_emphasize_commerce_count = [0; CommerceType::NUM];
            // GrowthControl (Jean Elcard, merged by Sephi)
            city.ai_emphasize_avoid_angry_citizens_count = 0;
            city.ai_emphasize_avoid_unhealthy_citizens_count = 0;
            city.ai_emphasize.fill(false);
        }

        // The rest (m_bForceEmphasizeCulture, m_routeToCity, best builds, special yield
        // multipliers, player closeness, floating defenders, workers, BETTER AI defense and
        // patrol bookkeeping): the C++ AI_reset values.
        let n = g.players.len();
        self.s().ensure_players(n);
        let fresh = CityAi::new(n);
        self.s().cities.insert(c, fresh);
    }

    /// C++ `void CvCityAI::AI_doTurn()` (CvCityAI.cpp:193-255)
    pub fn city_ai_do_turn(&self, g: &mut Game, c: CityId) {
        // BETTER AI (CvCity::AI_doTurn) Sephi
        if g.city_is_barbarian(c) {
            return;
        }

        if !g.city_is_human(c) {
            // Choose which City produces Better Defense Units for this City
            self.city_ai_city_choose_defense_production(g, c);
            // Choose which City produces Better Patrol Units for this City
            self.city_ai_city_choose_patrol_production(g, c);
        }
        if !g.cities.contains(c) {
            return;
        }

        if !g.city_is_human(c) {
            for s in g.d.specialists.keys() {
                g.city_set_force_specialist_count(c, s, 0);
            }
        }

        if !g.city_is_human(c) {
            self.city_ai_steal_plots(g, c);
        }

        self.city_ai_update_workers_needed_here(g, c);

        self.city_ai_update_best_build(g, c);

        self.city_ai_update_route_to_city(g, c);

        if g.city_is_human(c) {
            if g.cities[c].production_automated {
                self.city_ai_do_hurry(g, c, false);
            }
            return;
        }

        self.city_ai_do_panic(g, c);
        if !g.cities.contains(c) {
            return;
        }

        self.city_ai_do_draft(g, c, false);
        if !g.cities.contains(c) {
            return;
        }

        self.city_ai_do_hurry(g, c, false);
        if !g.cities.contains(c) {
            return;
        }

        self.city_ai_do_emphasize(g, c);
    }

    /// C++ `void CvCityAI::AI_assignWorkingPlots()` (CvCityAI.cpp:258-416)
    pub fn city_ai_assign_working_plots(&self, g: &mut Game, c: CityId) {
        if g.d.defines.ai_shouldnt_manage_plot_assignment != 0 {
            return;
        }

        let owner = g.cities[c].owner;

        // remove all assigned plots if we automated
        if !g.city_is_human(c) || g.cities[c].citizens_automated {
            //FfH: Modified by Kael 11/18/2007 (getNumCityPlots instead of NUM_CITY_PLOTS)
            let n = g.city_num_city_plots(c).max(0) as usize;
            for i in 0..n {
                g.city_set_working_plot(c, i, false);
            }
        }

        //update the special yield multiplier to be current
        self.city_ai_update_special_yield_multiplier(g, c);

        // remove any plots we can no longer work for any reason
        g.city_verify_working_plots(c);

        // if forcing specialists, try to make all future specialists of the same type
        let mut is_specialist_forced = false;
        let mut total_forced_specialists = 0;

        // make sure at least the forced amount of specialists are assigned
        for s in g.d.specialists.keys() {
            let forced_specialist_count = g.cities[c].force_specialist_count[s];
            if forced_specialist_count > 0 {
                is_specialist_forced = true;
                total_forced_specialists += forced_specialist_count;
            }

            if !g.city_is_human(c) || g.cities[c].citizens_automated || g.cities[c].specialist_count[s] < forced_specialist_count {
                g.city_set_specialist_count(c, s, forced_specialist_count);
            }
        }

        // if we have more specialists of any type than this city can have, reduce to the max
        for s in g.d.specialists.keys() {
            if !g.city_is_specialist_valid(c, s, 0) {
                let max = g.cities[c].max_specialist_count[s];
                if g.cities[c].specialist_count[s] > max {
                    g.city_set_specialist_count(c, s, max);
                }
            }
        }

        // always work the home plot (center)
        if let Some(home_plot) = g.city_index_plot(c, CITY_HOME_PLOT) {
            let work = g.cities[c].population > 0 && g.city_can_work(c, home_plot);
            g.city_set_working_plot(c, CITY_HOME_PLOT, work);
        }

        // keep removing the worst citizen until we are not over the limit
        while g.city_extra_population(c) < 0 {
            if !self.city_ai_remove_worst_citizen(g, c, None) {
                break;
            }
        }

        // do we have population unassigned
        while g.city_extra_population(c) > 0 {
            // (AI_addBestCitizen now handles forced specialist logic)
            if !self.city_ai_add_best_citizen(g, c, /*bWorkers*/ true, /*bSpecialists*/ true, &mut -1, &mut None) {
                break;
            }
        }

        // if forcing specialists, assign any other specialists that we must place based on forced specialists
        let initial_extra_specialists = g.city_extra_specialists(c);
        let mut extra_specialists = initial_extra_specialists;
        if is_specialist_forced && extra_specialists > 0 {
            for s in g.d.specialists.keys() {
                if g.city_is_specialist_valid(c, s, 1) {
                    let forced_specialist_count = g.cities[c].force_specialist_count[s];
                    if forced_specialist_count > 0 {
                        let mut specialist_count = g.cities[c].specialist_count[s];
                        let max_specialist_count = g.cities[c].max_specialist_count[s];

                        let mut specialists_to_add = ((initial_extra_specialists * forced_specialist_count) + (total_forced_specialists / 2)) / total_forced_specialists;
                        if extra_specialists < specialists_to_add {
                            specialists_to_add = extra_specialists;
                        }

                        specialist_count += specialists_to_add;
                        extra_specialists -= specialists_to_add;

                        // if we cannot fit that many, then add as many as we can
                        if specialist_count > max_specialist_count && !g.player_is_specialist_valid(owner, s) {
                            extra_specialists += specialist_count - max_specialist_count;
                            specialist_count = max_specialist_count;
                        }

                        g.city_set_specialist_count(c, s, specialist_count);
                    }
                }
            }
        }

        // if we still have population to assign, assign specialists
        while g.city_extra_specialists(c) > 0 {
            if !self.city_ai_add_best_citizen(g, c, /*bWorkers*/ false, /*bSpecialists*/ true, &mut -1, &mut None) {
                break;
            }
        }

        // if automated, look for better choices than the current ones
        if !g.city_is_human(c) || g.cities[c].citizens_automated {
            self.city_ai_juggle_citizens(g, c);
        }

        g.cities[c].ai_assign_work_dirty = false;

        // UI: citizen buttons dirty bit for the active player's selected city.
    }

    /// C++ `void CvCityAI::AI_updateAssignWork()` (CvCityAI.cpp:419-425)
    pub fn city_ai_update_assign_work(&self, g: &mut Game, c: CityId) {
        if g.cities[c].ai_assign_work_dirty {
            self.city_ai_assign_working_plots(g, c);
        }
    }

    /// C++ `bool CvCityAI::AI_stopGrowth()` (CvCityAI.cpp:432-486)
    pub fn city_ai_stop_growth(&self, g: &Game, c: CityId) -> bool {
        // GrowthControl (Jean Elcard, merged by Sephi): ported in ffh-core (`doGrowth` reads it).
        g.city_ai_stop_growth(c)
    }

    /// C++ `bool CvCityAI::AI_avoidGrowth()` (CvCityAI.cpp:491-552)
    pub fn city_ai_avoid_growth(&self, g: &Game, c: CityId) -> bool {
        // GrowthControl: a more general custom method replaces the AI_isEmphasizeAvoidGrowth test.
        if self.city_ai_stop_growth(g, c) {
            return true;
        }

        if g.city_is_food_production(c) {
            return true;
        }

        //FfH: Added by Kael 08/02/2007
        let owner = g.cities[c].owner;
        if g.players[owner].ignore_food {
            return true;
        }
        //FfH: End Add

        if !g.city_ai_is_emphasize_yield(c, YieldType::YIELD_FOOD) && !g.city_ai_is_emphasize_great_people(c) {
            let extra = if g.city_is_human(c) { 0 } else { 1 };
            let mut happiness_level = g.city_happy_level(c) - g.city_unhappy_level(c, extra);

            // ignore military unhappy, since we assume it will be fixed quickly, grow anyway
            if g.cities[c].military_happiness_units == 0 {
                happiness_level += (g.d.defines.no_military_percent_anger * (g.cities[c].population + 1)) / g.d.defines.percent_anger_divisor;
            }

            // if we can pop rush, we want to grow one over the cap
            if g.player_can_pop_rush(owner) {
                happiness_level += 1;
            }

            // if we have angry citizens
            if happiness_level < 0 {
                return true;
            }
        }

        false
    }

    /// C++ `bool CvCityAI::AI_ignoreGrowth()` (CvCityAI.cpp:555-568)
    pub fn city_ai_ignore_growth(&self, g: &Game, c: CityId) -> bool {
        if !g.city_ai_is_emphasize_yield(c, YieldType::YIELD_FOOD) && !g.city_ai_is_emphasize_great_people(c) {
            let extra = if g.city_is_human(c) { 0 } else { 1 };
            if !self.city_ai_food_available(g, c, extra) {
                return true;
            }
        }

        false
    }

    /// C++ `int CvCityAI::AI_specialistValue(SpecialistTypes eSpecialist, bool bAvoidGrowth, bool bRemove)` (CvCityAI.cpp:571-743)
    pub fn city_ai_specialist_value(&self, g: &Game, c: CityId, specialist: SpecialistType, avoid_growth: bool, remove: bool) -> i32 {
        let owner = g.cities[c].owner;
        let num_cities = g.players[owner].num_cities();
        let human = g.city_is_human(c);

        // C++ stores these in `short` arrays.
        let mut yields = [0i32; YieldType::NUM];
        for i in 0..YieldType::NUM {
            yields[i] = g.player_specialist_yield(owner, specialist, YieldType::from_usize(i)) as i16 as i32;
        }

        let mut commerce_yields = [0i32; CommerceType::NUM];
        for i in 0..CommerceType::NUM {
            commerce_yields[i] = g.player_specialist_commerce(owner, specialist, CommerceType::from_usize(i)) as i16 as i32;
        }

        let mut value = self.city_ai_yield_value(g, c, &yields, &commerce_yields, avoid_growth, remove, false, false, false, false);

        let great_people_rate = g.d.specialists[specialist].great_people_rate_change;

        let mut emphasis_count = 0;
        if great_people_rate != 0 {
            let mut gpp_value = 4;
            if g.city_ai_is_emphasize_great_people(c) {
                gpp_value = if human { 30 } else { 20 };
            } else {
                if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_COMMERCE) {
                    gpp_value = 2;
                    emphasis_count += 1;
                }
                if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_PRODUCTION) {
                    gpp_value = 1;
                    emphasis_count += 1;
                }
                if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_FOOD) {
                    gpp_value = 1;
                    emphasis_count += 1;
                }
            }

            let mut temp_value = great_people_rate * gpp_value;

            if !human || g.city_ai_is_emphasize_great_people(c) {
                let progress = g.cities[c].great_people_progress;
                if progress > 0 {
                    let threshold = g.player_great_people_threshold(owner, false);
                    // C++ int arithmetic (wraps on overflow in the DLL).
                    let num = great_people_rate.wrapping_mul(if human { 1 } else { 4 }).wrapping_mul(gpp_value).wrapping_mul(progress).wrapping_mul(progress);
                    let den = threshold.wrapping_mul(threshold);
                    if den != 0 {
                        temp_value += num / den;
                    }
                }
            }

            let current_era = g.players[owner].current_era.i();
            let total_eras = g.d.eras.len() as i32;

            if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE2) {
                let unit_class = g.d.specialists[specialist].great_people_unit_class;
                if let Some(unit_class) = unit_class {
                    let civ = g.city_civilization_type(c);
                    let great_people_unit = g.d.civilizations[civ].civilization_units(unit_class, g.d.unit_classes[unit_class].default_unit_index);
                    if let Some(gp_unit) = great_people_unit {
                        let unit_info = &g.d.units[gp_unit];
                        if unit_info.great_work_culture > 0 {
                            temp_value += unit_info.great_work_culture / if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE3) { 200 } else { 350 };
                        }
                    }
                }
            }

            if !human && current_era <= (total_eras * 2) / 3 {
                // try to spawn a prophet for any shrines we have yet to build
                let mut need_prophet = false;
                let mut best_spread_value = 0;

                for religion in g.d.religions.keys() {
                    if g.city_is_holy_city(c, religion) && !g.city_has_shrine(c, religion) && (current_era < total_eras / 2 || g.count_religion_levels(religion) >= 10) {
                        let civ = g.city_civilization_type(c);
                        let unit_class = g.d.specialists[specialist].great_people_unit_class;
                        let great_people_unit = unit_class.and_then(|uc| g.d.civilizations[civ].civilization_units(uc, g.d.unit_classes[uc].default_unit_index));
                        if let Some(gp_unit) = great_people_unit {
                            // note, for normal XML, this count will be one (there is only 1 shrine building for each religion)
                            let shrine_building_count = g.get_shrine_building_count(Some(religion));
                            for i in 0..shrine_building_count {
                                let Some(building) = g.get_shrine_building(i, Some(religion)) else { continue };

                                // if this unit builds or forceBuilds this building
                                if g.d.units[gp_unit].buildings[building] || g.d.units[gp_unit].force_buildings[building] {
                                    need_prophet = true;
                                    best_spread_value = best_spread_value.max(g.count_religion_levels(religion));
                                }
                            }
                        }
                    }
                }

                if need_prophet {
                    temp_value += great_people_rate * best_spread_value;
                }
            }
            temp_value *= 100;
            // FIX: guard a zero average (C++ would divide by zero).
            temp_value /= self.player_ai_average_great_people_multiplier(g, owner).max(1);

            temp_value /= 1 + emphasis_count;
            value += temp_value;
        } else {
            let generic_citizen = g.d.defines.default_specialist;

            // are we the generic specialist?
            if specialist.i() == generic_citizen {
                value *= 60;
                value /= 100;
            }
        }

        let experience = g.d.specialists[specialist].experience;
        if experience != 0 {
            let production_rank = g.city_find_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);
            let has_met_count = g.team_get_has_met_civ_count(g.city_team(c), true);

            value += experience * if has_met_count > 0 { 4 } else { 2 };
            if production_rank <= num_cities / 2 + 1 {
                value += experience * 4;
            }
            value += (g.cities[c].military_production_modifier * experience * 8) / 100;
        }

        value * 100
    }

    /// C++ `void CvCityAI::AI_chooseProduction()` (CvCityAI.cpp:746-2263)
    pub fn city_ai_choose_production(&self, g: &mut Game, c: CityId) {
        let danger = self.city_ai_is_danger(g, c);

        let p = g.cities[c].owner;
        let t = g.city_team(c);

        if g.city_is_production(c) {
            if g.city_production(c) > 0 {
                // PORT ADAPT: ffh-core clears `ai_choose_production_dirty` before asking the
                // controller; C++ clears it only once it re-decides below. The rules only reach
                // these "keep building" returns from `doProduction` with the flag set, so it is
                // set again to keep the C++ re-check on the next turns.
                let keep = |g: &mut Game| {
                    g.cities[c].ai_choose_production_dirty = true;
                };

                if g.city_production_unit_ai(c) == Some(UnitAIType::UNITAI_SETTLE) && self.player_ai_is_financial_trouble(g, p) {
                }
                //if we are killing our growth to train this, then finish it.
                else if !danger && g.city_is_food_production(c) {
                    let a = g.city_area(c);
                    if g.map.areas[a].area_ai_type[t] != Some(AreaAIType::AREAAI_DEFENSIVE) {
                        keep(g);
                        return;
                    }
                }
                // if less than 3 turns left, keep building current item
                else if g.city_production_turns_left(c) <= 3 {
                    keep(g);
                    return;
                }

                // if building a combat unit, and we have no defenders, keep building it
                if let Some(production_unit) = g.city_production_unit(c) {
                    if g.plot_get_num_defenders(g.city_plot(c), p) == 0 {
                        if g.d.units[production_unit].combat > 0 {
                            keep(g);
                            return;
                        }
                    }
                }

                // if we are building a wonder, do not cancel, keep building it (if no danger)
                if let Some(production_building) = g.city_production_building(c) {
                    if !danger {
                        if let Some(class) = g.d.buildings[production_building].building_class_type {
                            if is_limited_wonder_class(g.d, class) {
                                keep(g);
                                return;
                            }
                        }
                    }
                }
            }

            g.city_clear_order_queue(c);
        }

        if g.player_is_anarchy(p) {
            return;
        }

        // only clear the dirty bit if we actually do a check, multiple items might be queued
        g.cities[c].ai_choose_production_dirty = false;

        // allow python to handle it
        if self.py_ai_choose_production(g, c) {
            return;
        }
        if !g.cities.contains(c) {
            return;
        }

        if g.city_is_human(c) && g.cities[c].production_automated {
            self.city_ai_build_governor_choose_production(g, c);
            return;
        }

        let area = g.city_area(c);
        let mut water_area = g.city_water_area(c);
        let mut maybe_water_area = false;

        if let Some(wa) = water_area {
            maybe_water_area = true;
            if !self.team_ai_is_water_area_relevant(g, t, wa) {
                water_area = None;
            }
        }

        let num_cities = g.players[p].num_cities();
        let was_food_production = g.city_is_food_production(c);
        let has_met_human = g.team_has_met_human(t);
        let area_ai = g.map.areas[area].area_ai_type[t];
        let land_war = area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING);
        let defense_war = area_ai == Some(AreaAIType::AREAAI_DEFENSIVE);
        let assault_assist = area_ai == Some(AreaAIType::AREAAI_ASSAULT_ASSIST);
        let assault = assault_assist || area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_MASSING);
        let primary_area = self.player_ai_is_primary_area(g, p, area);
        let financial_trouble = self.player_ai_is_financial_trouble(g, p);
        let culture_pressure = self.city_ai_calculate_culture_pressure(g, c, false);
        let num_cities_in_area = g.map.areas[area].cities_per_player[p];
        let mut important_city = false; //be very careful about setting this.
        let mut big_culture_city = false;
        let culture_rate_rank = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
        let cultural_victory_num_culture_cities = g.num_culture_victory_cities;

        let get_better_units = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_GET_BETTER_UNITS);
        let aggressive_ai = g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI);
        let always_peace = g.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE);

        let unit_cost_percentage = (g.player_calculate_unit_cost(p) * 100) / g.player_calculate_pre_inflated_costs(p).max(1);
        let water_percent = self.city_ai_calculate_water_world_percent(g, c);

        let build_unit_prob = self.city_ai_build_unit_prob(g, c);

        let existing_workers = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_WORKER);
        let needed_workers = self.player_ai_needed_workers(g, p, area);
        let needed_sea_workers = if water_area.is_none() { 0 } else { self.city_ai_needed_sea_workers(g, c) };

        let target_culture_per_turn = self.city_ai_calculate_target_culture_per_turn(g, c);

        let mut area_best_found_value = 0;
        let num_area_city_sites = self.player_ai_get_num_area_city_sites(g, p, area.0 as i32, &mut area_best_found_value);

        let mut water_area_best_found_value = 0;
        let num_water_area_city_sites = match water_area {
            None => 0,
            Some(wa) => self.player_ai_get_num_adjacent_area_city_sites(g, p, wa.0 as i32, area.0 as i32, &mut water_area_best_found_value),
        };
        let num_settlers = self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_SETTLE);

        let capital = g.player_get_capital_city(p);
        let mut is_capital_area = false;
        let mut num_capital_area_cities = 0;
        if let Some(cap) = capital {
            let cap_area = g.city_area(cap);
            num_capital_area_cities = g.map.areas[cap_area].cities_per_player[p];
            if area == cap_area {
                is_capital_area = true;
            }
        }

        let mut max_settlers = 0;
        if !financial_trouble {
            max_settlers = ((num_cities + 1) / 2).min(num_area_city_sites + num_water_area_city_sites);
            if land_war || assault {
                max_settlers = (max_settlers + 2) / 3;
            }
        }

        let mut choose_worker = false;

        let mut economy_flags = 0;
        economy_flags |= BUILDINGFOCUS_PRODUCTION;
        economy_flags |= BUILDINGFOCUS_GOLD;
        economy_flags |= BUILDINGFOCUS_RESEARCH;
        economy_flags |= BUILDINGFOCUS_MAINTENANCE;
        economy_flags |= BUILDINGFOCUS_HAPPY;
        economy_flags |= BUILDINGFOCUS_HEALTHY;
        if g.city_ai_is_emphasize_great_people(c) {
            economy_flags |= BUILDINGFOCUS_SPECIALIST;
        }
        if !g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE) {
            economy_flags |= BUILDINGFOCUS_ESPIONAGE;
        }

        if num_cities_in_area > 2 {
            if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                if culture_rate_rank <= cultural_victory_num_culture_cities + 1 {
                    big_culture_city = true;

                    // if we do not have enough cities, then the highest culture city will not get special attention
                    if culture_rate_rank > 1 || num_cities > cultural_victory_num_culture_cities + 1 {
                        if ((num_area_city_sites + num_water_area_city_sites) > 0 && num_cities < 6) && self.rand(2, "AI Less Culture More Expand") == 0 {
                            important_city = false;
                        } else {
                            important_city = true;
                        }
                    }
                }
            }
        }

        let production_rank = g.city_find_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);

        g.city_clear_order_queue(c);

        if was_food_production {
            self.city_ai_assign_working_plots(g, c);
        }

        // if we need to pop borders, then do that immediately if we have drama and can do it
        if target_culture_per_turn > 0 && opt_to_i32(g.cities[c].culture_level) <= 1 {
            if self.city_ai_choose_process(g, c, Some(CommerceType::COMMERCE_CULTURE)) {
                return;
            }
        }

        // if we just captured this city, dont build a unit first
        if g.city_is_occupation(c) {
            // pick granary or lighthouse, any duration
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_FOOD, MAX_INT, 0) {
                return;
            }

            // try picking forge, etc, any duration
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_PRODUCTION, MAX_INT, 0) {
                return;
            }

            // just pick any building, any duration
            if self.city_ai_choose_building(g, c, 0, MAX_INT, 0) {
                return;
            }
        }

        let plot = g.city_plot(c);

        if g.plot_get_num_defenders(plot, p) == 0 {
            // XXX check for other team's units?
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
                return;
            }

            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_COUNTER)) {
                return;
            }

            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_SPECIAL)) {
                return;
            }

            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                return;
            }
        }

        if g.city_is_barbarian(c) {
            let attack_count = plot_unit_ai_count(g, plot, UnitAIType::UNITAI_ATTACK, p);
            if !self.city_ai_is_defended(g, c, attack_count) {
                // XXX check for other team's units?
                if self.city_ai_choose_defender(g, c) {
                    return;
                }

                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                    return;
                }
            }

            if !danger && needed_workers > 0 && self.city_ai_get_workers_needed(g, c) > 0 && self.city_ai_get_workers_have(g, c) == 0 {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
            }

            if !danger && needed_sea_workers > 0 {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER_SEA)) {
                    return;
                }
            }

            if self.rand(100, "AI Build Unit Production") > self.city_ai_build_unit_prob(g, c) {
                let mut barbarian_flags = 0;
                barbarian_flags |= BUILDINGFOCUS_FOOD;
                barbarian_flags |= BUILDINGFOCUS_PRODUCTION;
                barbarian_flags |= BUILDINGFOCUS_EXPERIENCE;

                if self.city_ai_choose_building(g, c, barbarian_flags, MAX_INT, 0) {
                    return;
                }

                if self.city_ai_choose_building(g, c, 0, MAX_INT, 0) {
                    return;
                }
            }

            if plot_unit_ai_count(g, plot, UnitAIType::UNITAI_ASSAULT_SEA, p) > 0 {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK_CITY)) {
                    return;
                }
            }

            if let Some(wa) = water_area {
                if water_percent > 30 {
                    if self.rand(2, "AI Coast Raiders!") == 0 {
                        if self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_ASSAULT_SEA) <= 1 + num_cities / 2 {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ASSAULT_SEA)) {
                                return;
                            }
                        }
                    }
                    if self.rand(110, "AI arrrr!") < water_percent + 10 {
                        if self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_PIRATE_SEA) <= num_cities {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_PIRATE_SEA)) {
                                return;
                            }
                        }

                        //FfH: Modified by Kael 01/30/2009 (was < iNumCitiesInArea)
                        if self.player_ai_total_area_unit_ais(g, p, wa, UnitAIType::UNITAI_ATTACK_SEA) < num_cities_in_area / 3 {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK_SEA)) {
                                return;
                            }
                        }
                    }
                }
            }

            if self.city_ai_choose_unit(g, c, None) {
                return;
            }

            return;
        }

        if (target_culture_per_turn > 0 || g.cities[c].population > 5) && g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) == 0 {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, 30, 0) {
                return;
            }
        }

        let train_percent = g.d.game_speeds[g.game_speed_type()].train_percent;
        if existing_workers == 0 && !danger && (self.city_ai_get_workers_needed(g, c) > 0 || (g.city_is_capital(c) && g.elapsed_game_turns < (30 * train_percent) / 100)) {
            let land_bonuses = count_num_bonuses_any(g, c, /*bIncludeOurs*/ true, /*bIncludeNeutral*/ true, -1, /*bLand*/ true, /*bWater*/ false);
            if land_bonuses > 1 || g.cities[c].population > 3 {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
            if needed_sea_workers > 0 && g.cities[c].population < 3 {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER_SEA)) {
                    return;
                }
            }
            if land_bonuses == 1 || self.city_ai_get_workers_needed(g, c) > 0 {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
        }

        let mut percent_of_domination = 0;
        let our_pop_percent = 100 * g.team_get_total_population(t, true) / g.total_population.max(1);

        for v in g.d.victories.keys() {
            if g.d.victories[v].population_percent_lead > 0 {
                percent_of_domination = 100 * our_pop_percent / g.adjusted_population_percent(v).max(1);
            }
        }

        if percent_of_domination >= 90 {
            if (g.city_good_health(c) - g.city_bad_health(c, true, 0)) < 1 {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_HEALTHY, MAX_INT, 0) {
                    return;
                }
            }
        }

        if danger && self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK) < 2 {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                return;
            }
        }

        if maybe_water_area {
            //FfH: Modified by Kael 01/30/2009 (was < 3)
            if self.player_ai_get_num_train_ai_units(g, p, UnitAIType::UNITAI_ATTACK_SEA) + self.player_ai_get_num_train_ai_units(g, p, UnitAIType::UNITAI_PIRATE_SEA) + self.player_ai_get_num_train_ai_units(g, p, UnitAIType::UNITAI_RESERVE_SEA) == 0 {
                let water_danger = maybe_water_area && self.player_ai_get_water_danger(g, p, plot, 4, true) > 0;
                if water_danger || (water_area.is_some() && primary_area && self.player_ai_count_num_area_hostile_units(g, p, water_area.unwrap(), true, false, false, false) > 0) {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK_SEA)) {
                        return;
                    }
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_PIRATE_SEA)) {
                        return;
                    }
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_RESERVE_SEA)) {
                        return;
                    }
                }
            }
            if let Some(wa) = water_area {
                if area_best_found_value == 0 || water_area_best_found_value > area_best_found_value || (water_percent > 60 && self.rand(4, "AI Train Early Sea Explore or Settler") == 0) {
                    if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_EXPLORE_SEA) == 0 {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_EXPLORE_SEA)) {
                            return;
                        }
                    }
                    if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_SETTLER_SEA) == 0 {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SETTLER_SEA)) {
                            return;
                        }
                    }
                }
            }
        }

        if g.cities[c].domain_free_experience[DomainType::DOMAIN_LAND.idx()] == 0 && g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) > 4 {
            let max_turns = if g.players[p].current_era.i() > 1 { 0 } else { 7 };
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_EXPERIENCE, max_turns, 33) {
                return;
            }
        }

        let plot_settler_count = if num_settlers == 0 { 0 } else { plot_unit_ai_count(g, plot, UnitAIType::UNITAI_SETTLE, p) };
        let plot_city_defender_count = plot_unit_ai_count(g, plot, UnitAIType::UNITAI_CITY_DEFENSE, p);
        //minimal defense.
        if plot_city_defender_count <= plot_settler_count {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
                return;
            }
        }

        if !primary_area {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_FOOD, 60, 10) {
                return;
            }
        }

        let current_era = g.players[p].current_era.i();
        // C++ quirk: `!bDanger && (A) || (B)` parses as `(!bDanger && A) || B`.
        if (!danger && current_era > g.start_era().i() + production_rank / 2) || current_era > g.d.eras.len() as i32 / 2 {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_PRODUCTION, 20, 15) {
                return;
            }
            if existing_workers < (needed_workers + 1) / 2 {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
        }

        let crush_strategy = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CRUSH);
        let needed_floating_defenders = if g.city_is_barbarian(c) || crush_strategy { 0 } else { self.player_ai_get_total_floating_defenders_needed(g, p, area) };
        let total_floating_defenders = if g.city_is_barbarian(c) { 0 } else { self.player_ai_get_total_floating_defenders(g, p, area) };

        let mut floating_defender_types: Vec<(UnitAIType, i32)> = Vec::new();
        floating_defender_types.push((UnitAIType::UNITAI_CITY_DEFENSE, 125));
        floating_defender_types.push((UnitAIType::UNITAI_CITY_COUNTER, 100));
        //floatingDefenderTypes.push_back(std::make_pair(UNITAI_CITY_SPECIAL, 0));
        floating_defender_types.push((UnitAIType::UNITAI_RESERVE, 100));
        floating_defender_types.push((UnitAIType::UNITAI_COLLATERAL, 100));

        if total_floating_defenders < (needed_floating_defenders + 1) / (if get_better_units { 3 } else { 2 }) {
            if self.city_ai_choose_least_represented_unit(g, c, &mut floating_defender_types.clone()) {
                return;
            }
        }

        if existing_workers != 0 {
            if !danger && existing_workers < (needed_workers + 1) / 2 {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
        }

        //do a check for one tile island type thing?
        //this can be overridden by "wait and grow more"
        if danger && existing_workers == 0 && (g.city_is_capital(c) || needed_workers > 0 || needed_sea_workers > 0) {
            if count_num_bonuses_any(g, c, /*bIncludeOurs*/ true, /*bIncludeNeutral*/ true, -1, /*bLand*/ true, /*bWater*/ false) > 0 || (g.city_is_capital(c) && g.cities[c].population > 3) {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
            if needed_sea_workers > 0 {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER_SEA)) {
                    return;
                }
            }
        }

        if let Some(wa) = water_area {
            if needed_sea_workers > 0 {
                if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_WORKER_SEA) < needed_sea_workers {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER_SEA)) {
                        return;
                    }
                }
            }
        }

        if !land_war && !assault && target_culture_per_turn > g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) {
            if self.rand(if aggressive_ai { 3 } else { 2 }, "AI Culture Build") == 0 {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, if aggressive_ai { 10 } else { 20 }, 0) {
                    return;
                }
            }
        }

        let mut min_found_value = self.player_ai_get_min_found_value(g, p);
        if danger {
            min_found_value *= 3;
            min_found_value /= 2;
        }

        if !get_better_units && is_capital_area && area_best_found_value < min_found_value * 2 {
            //Building an early city hunting stack.
            let attack_city_count = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK);
            if attack_city_count > 0 {
                if attack_city_count < 3 + build_unit_prob / 10 {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                        return;
                    }
                }
            } else {
                let mut start_attack_stack_rand = 0;
                let barbarian = g.barbarian_player();
                if g.map.areas[area].cities_per_player[barbarian] != 0 {
                    start_attack_stack_rand += 15;
                }
                if g.map.areas[area].num_cities - num_cities_in_area > 0 {
                    start_attack_stack_rand += build_unit_prob / 2;
                }

                if self.rand(100, "AI start city attack stack") <= start_attack_stack_rand {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                        return;
                    }
                }
            }
        }

        let wonder_construct_rand = g.d.leader_heads[g.city_personality_type(c)].wonder_construct_rand;

        //oppurunistic wonder build (1)
        if !danger && !g.city_has_active_world_wonder(c) && num_cities <= 3 {
            let mut wonder_time = self.rand(wonder_construct_rand, "Wonder Construction Rand");
            wonder_time /= 5;
            wonder_time += 7;
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_WORLDWONDER, wonder_time, 0) {
                return;
            }
        }

        if !danger && !is_capital_area && g.map.areas[area].cities_per_player[p] > num_capital_area_cities {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CAPITAL, 15, 0) {
                return;
            }
        }

        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_FOOD, if g.city_is_capital(c) { 5 } else { 30 }, 30) {
            return;
        }

        let mut spread_unit_threshold = 1000;
        spread_unit_threshold += if land_war { 1000 } else { 0 };

        let mut best_spread_unit: Option<UnitType> = None;
        let mut best_spread_unit_value = -1;

        {
            let mut spread_unit_roll = (100 - build_unit_prob) / 3;
            spread_unit_roll += if land_war { 0 } else { 10 };

            if self.city_ai_best_spread_unit(g, c, true, true, spread_unit_roll, &mut best_spread_unit, &mut best_spread_unit_value) {
                if best_spread_unit_value > spread_unit_threshold {
                    if let Some(u) = best_spread_unit {
                        if self.city_ai_choose_unit_2(g, c, u, UnitAIType::UNITAI_MISSIONARY) {
                            return;
                        }
                    }
                }
            }
        }

        if !danger && production_rank <= (num_cities / 5) + 1 {
            if self.city_ai_choose_project(g, c) {
                return;
            }
        }

        //minimal defense.
        if plot_unit_ai_count(g, plot, UnitAIType::UNITAI_CITY_DEFENSE, p) < self.city_ai_min_defenders(g, c) + plot_settler_count {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
                return;
            }
        }

        if area_best_found_value > min_found_value || water_area_best_found_value > min_found_value {
            if let Some(wa) = water_area {
                let total_cities = num_cities;
                let mut settler_sea_needed = num_water_area_city_sites.min(((total_cities + 4) / 8) + 1);
                if let Some(cap) = capital {
                    let cap_area = g.city_area(cap);
                    let over_seas_colonies = total_cities - g.map.areas[cap_area].cities_per_player[p];
                    let mut loop_ = 2;
                    let mut extras = 0;
                    while over_seas_colonies >= loop_ {
                        extras += 1;
                        loop_ += loop_ + 2;
                    }
                    settler_sea_needed += (self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_WORKER) / 4).min(extras);
                }
                if assault {
                    settler_sea_needed = settler_sea_needed.min(1);
                }

                if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_SETTLER_SEA) < settler_sea_needed {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SETTLER_SEA)) {
                        return;
                    }
                }
            }

            if plot_settler_count == 0 {
                if num_settlers < max_settlers && (!land_war || self.rand(2, "AI War Settler") == 0) {
                    if plot_city_defender_count == 1 {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
                            return;
                        }
                    } else if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SETTLE)) {
                        return;
                    }
                }
            }
        }

        //this is needed to build the cathedrals quickly
        //also very good for giving cultural cities first dibs on wonders
        if important_city && culture_rate_rank <= cultural_victory_num_culture_cities {
            if culture_rate_rank == cultural_victory_num_culture_cities {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_BIGCULTURE | BUILDINGFOCUS_CULTURE | BUILDINGFOCUS_WONDEROK, 40, 0) {
                    return;
                }
            } else if self.rand((if culture_rate_rank == 1 { 4 } else { 1 }) + cultural_victory_num_culture_cities * 2, "AI Build up Culture") < culture_rate_rank {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_BIGCULTURE | BUILDINGFOCUS_CULTURE | BUILDINGFOCUS_WONDEROK, 40, 0) {
                    return;
                }
            }
        }

        // don't build frivolous things if this is an important city unless we at war
        if !important_city || land_war || assault {
            if primary_area {
                if self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK) == 0 {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                        return;
                    }
                }
            }

            if !land_war && !danger {
                if self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_EXPLORE) < self.player_ai_needed_explorers(g, p, area) {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_EXPLORE)) {
                        return;
                    }
                }
            }

            if defense_war {
                if self.rand(100, "AI Train Defensive Unit (Panic)") > (if get_better_units { 60 } else { 40 }) {
                    let mut panic_defender_types: Vec<(UnitAIType, i32)> = Vec::new();
                    panic_defender_types.push((UnitAIType::UNITAI_RESERVE, 100));
                    panic_defender_types.push((UnitAIType::UNITAI_COUNTER, 100));
                    panic_defender_types.push((UnitAIType::UNITAI_COLLATERAL, 100));
                    panic_defender_types.push((UnitAIType::UNITAI_ATTACK, 100));
                    if self.city_ai_choose_least_represented_unit(g, c, &mut panic_defender_types) {
                        return;
                    }
                }
            }
        }

        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_FOOD, 60, 10) {
            return;
        }

        //oppurunistic wonder build
        if !danger && (!g.city_has_active_world_wonder(c) || num_cities > 3) {
            let mut wonder_time = self.rand(wonder_construct_rand, "Wonder Construction Rand");
            wonder_time /= 5;
            wonder_time += 8;
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_WORLDWONDER, wonder_time, 0) {
                return;
            }
        }

        if needed_workers < existing_workers {
            if self.city_ai_get_workers_needed(g, c) > 0 && self.city_ai_get_workers_have(g, c) == 0 {
                if !choose_worker && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                    return;
                }
                choose_worker = true;
            }
        }

        //essential economic builds
        if self.city_ai_choose_building(g, c, economy_flags, 10, 25) {
            return;
        }

        if best_spread_unit_value > (spread_unit_threshold * 60) / 100 {
            if let Some(u) = best_spread_unit {
                if self.city_ai_choose_unit_2(g, c, u, UnitAIType::UNITAI_MISSIONARY) {
                    return;
                }
            }
        }

        let mut max_unit_spending = (if aggressive_ai { 7 } else { 3 }) + build_unit_prob / 3;
        if always_peace {
            max_unit_spending = -10;
        } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FINAL_WAR) {
            max_unit_spending = 5 + max_unit_spending + (100 - max_unit_spending) / 2;
        } else {
            max_unit_spending += if defense_war { 4 } else { 0 };
            match area_ai {
                Some(AreaAIType::AREAAI_OFFENSIVE) => max_unit_spending += 5,
                Some(AreaAIType::AREAAI_DEFENSIVE) => max_unit_spending += 10,
                Some(AreaAIType::AREAAI_MASSING) => max_unit_spending += 25,
                Some(AreaAIType::AREAAI_ASSAULT) => max_unit_spending += 8,
                Some(AreaAIType::AREAAI_ASSAULT_MASSING) => max_unit_spending += 16,
                Some(AreaAIType::AREAAI_ASSAULT_ASSIST) => max_unit_spending += 6,
                _ => {}
            }
        }

        if unit_cost_percentage < max_unit_spending + 10 {
            let mut best_sea_assault_capacity = 0;
            if let (Some(wa), true) = (water_area, assault) {
                let mut best_assault_unit: Option<UnitType> = None;
                self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_ASSAULT_SEA, c, &mut best_assault_unit);
                if let Some(u) = best_assault_unit {
                    best_sea_assault_capacity = g.d.units[u].cargo_space;
                }

                let mut units_to_transport = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK_CITY);
                units_to_transport += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK);
                units_to_transport += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_COUNTER);

                // BETTER AI (UNITAI_WARWIZARD) Sephi
                units_to_transport += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_WARWIZARD);

                let mut transports = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ASSAULT_SEA);
                transports += self.player_ai_total_area_unit_ais(g, p, wa, UnitAIType::UNITAI_ASSAULT_SEA);

                let mut escorts = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ESCORT_SEA);
                escorts += self.player_ai_total_area_unit_ais(g, p, wa, UnitAIType::UNITAI_ESCORT_SEA);

                //The way of calculating numbers is a bit fuzzy since the ships
                //can make return trips. When massing for a war it'll train enough
                //ships to move it's entire army. Once the war is underway it'll stop
                //training so many ships on the assumption that those out at sea
                //will return...

                if escorts < (1 + 2 * transports) / 3 && self.rand(2, "AI train escort sea") == 0 {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ESCORT_SEA)) {
                        self.city_ai_choose_building(g, c, BUILDINGFOCUS_DOMAINSEA, MAX_INT, 0);
                        return;
                    }
                }

                let mut best_attack_sea_unit: Option<UnitType> = None;
                self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_ATTACK_SEA, c, &mut best_attack_sea_unit);
                if let Some(u) = best_attack_sea_unit {
                    if g.d.units[u].bombard_rate > 0 {
                        let mut attack_sea = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_ATTACK_SEA);
                        attack_sea += self.player_ai_total_area_unit_ais(g, p, wa, UnitAIType::UNITAI_ATTACK_SEA);

                        if attack_sea < (1 + 2 * transports) / 2 && self.rand(2, "AI train attack sea") == 0 {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK_SEA)) {
                                self.city_ai_choose_building(g, c, BUILDINGFOCUS_DOMAINSEA, MAX_INT, 0);
                                return;
                            }
                        }
                    }
                }

                if units_to_transport > transports * best_sea_assault_capacity {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ASSAULT_SEA)) {
                        self.city_ai_choose_building(g, c, BUILDINGFOCUS_DOMAINSEA, MAX_INT, 0);
                        return;
                    }
                }

                let carriers = self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_CARRIER_SEA);

                if carriers > 0 {
                    let mut best_carrier_unit: Option<UnitType> = None;
                    self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_CARRIER_SEA, c, &mut best_carrier_unit);
                    if let Some(u) = best_carrier_unit {
                        let carrier_air_needed = carriers * g.d.units[u].cargo_space;

                        if self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_CARRIER_AIR) < carrier_air_needed {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CARRIER_AIR)) {
                                return;
                            }
                        }
                    }
                }

                if carriers < self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_ASSAULT_SEA) / 4 {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CARRIER_SEA)) {
                        return;
                    }
                }

                let missile_carriers = self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_MISSILE_CARRIER_SEA);

                if missile_carriers > 0 {
                    let mut best_missile_carrier_unit: Option<UnitType> = None;
                    self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_MISSILE_CARRIER_SEA, c, &mut best_missile_carrier_unit);
                    if let Some(u) = best_missile_carrier_unit {
                        let cargo = g.d.units[u].cargo_space;
                        let missile_carrier_air_needed = missile_carriers * cargo;

                        if self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_MISSILE_AIR) < missile_carrier_air_needed
                            || (primary_area && self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_MISSILE_CARRIER_SEA) * cargo < self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_MISSILE_AIR))
                        {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_MISSILE_AIR)) {
                                return;
                            }
                        }
                    }
                }
            }
        }

        let mut air_unit_types: Vec<(UnitAIType, i32)> = Vec::new();

        let mut aircraft_need = 0;
        let mut aircraft_have = 0;
        let mut best_attack_aircraft: Option<UnitType> = None;
        let mut best_missile: Option<UnitType> = None;

        if unit_cost_percentage < max_unit_spending + 4 {
            let best_air_value = self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_ATTACK_AIR, c, &mut best_attack_aircraft);
            let best_missile_value = self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_MISSILE_AIR, c, &mut best_missile);
            if best_air_value + best_missile_value > 0 {
                aircraft_have = self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_ATTACK_AIR) + self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_DEFENSE_AIR) + self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_MISSILE_AIR);
                if let Some(u) = best_attack_aircraft {
                    aircraft_need = (2 + num_cities * (3 * g.d.units[u].air_combat)) / (2 * g.best_land_unit_combat().max(1));
                    let best_defense_value = self.player_ai_best_city_unit_ai_value(g, p, UnitAIType::UNITAI_DEFENSE_AIR, c, &mut None);
                    if best_defense_value > 0 && best_air_value > best_defense_value {
                        aircraft_need *= 3;
                        aircraft_need /= 2;
                    }
                }
                if best_missile_value > 0 {
                    aircraft_need = aircraft_need.max(1 + num_cities / 2);
                }

                let air_blitz = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_AIR_BLITZ);
                let land_blitz = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_LAND_BLITZ);
                if air_blitz {
                    aircraft_need *= 3;
                    aircraft_need /= 2;
                } else if land_blitz {
                    aircraft_need /= 2;
                    aircraft_need += 1;
                }

                air_unit_types.push((UnitAIType::UNITAI_ATTACK_AIR, if air_blitz { 125 } else { 80 }));
                air_unit_types.push((UnitAIType::UNITAI_DEFENSE_AIR, if land_blitz { 100 } else { 100 }));
                if best_missile_value > 0 {
                    air_unit_types.push((UnitAIType::UNITAI_MISSILE_AIR, 50));
                }

                air_unit_types.push((UnitAIType::UNITAI_ICBM, 20));

                if aircraft_have * 2 < aircraft_need {
                    if self.city_ai_choose_least_represented_unit(g, c, &mut air_unit_types.clone()) {
                        return;
                    }
                }
            }
        }

        //FfH: Modified by Kael 05/08/2008 (the nuke block is commented out)

        if (!important_city || defense_war) && unit_cost_percentage < max_unit_spending {
            if !financial_trouble && !get_better_units && (land_war || ((assault || self.player_ai_is_do_strategy(g, p, AI_STRATEGY_DAGGER)) && !assault_assist)) {
                let mut train_invader_chance = build_unit_prob + 10;
                if aggressive_ai {
                    train_invader_chance += 15;
                }
                if area_ai == Some(AreaAIType::AREAAI_MASSING) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_MASSING) {
                    train_invader_chance = 100 - ((100 - train_invader_chance) / if crush_strategy { 6 } else { 3 });
                }
                if self.rand(100, "AI Dagger Offense Unit") < train_invader_chance {
                    if self.city_ai_choose_building(g, c, BUILDINGFOCUS_EXPERIENCE, 20, 0) {
                        return;
                    }

                    let mut invader_types: Vec<(UnitAIType, i32)> = Vec::new();
                    invader_types.push((UnitAIType::UNITAI_ATTACK_CITY, 100));
                    invader_types.push((UnitAIType::UNITAI_COUNTER, 50));
                    invader_types.push((UnitAIType::UNITAI_ATTACK, 40));
                    let paradrop = (if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_AIR_BLITZ) { 30 } else { 20 }) / if assault { 2 } else { 1 };
                    invader_types.push((UnitAIType::UNITAI_PARADROP, paradrop));
                    if !assault {
                        if self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_PILLAGE) <= (num_cities_in_area + 1) / 2 {
                            invader_types.push((UnitAIType::UNITAI_PILLAGE, 30));
                        }
                    }

                    if self.city_ai_choose_least_represented_unit(g, c, &mut invader_types) {
                        return;
                    }
                }
            }
        }

        //Arr.
        if let (Some(wa), false, false) = (water_area, land_war, assault) {
            let pirate_count = self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_PIRATE_SEA);
            let mut needed_pirates = 1 + (g.map.areas[wa].num_tiles / (200 - build_unit_prob).max(1));
            needed_pirates *= 20 + water_percent;
            needed_pirates /= 100;

            if g.player_is_no_foreign_trade(p) {
                needed_pirates *= 3;
                needed_pirates /= 2;
            }
            if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_PIRATE_SEA) < needed_pirates {
                if self.player_ai_calculate_unit_ai_viability(g, p, UnitAIType::UNITAI_PIRATE_SEA, DomainType::DOMAIN_SEA) > 49 {
                    if self.rand(100, "AI train pirate") < water_percent / (1 + pirate_count) {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_PIRATE_SEA)) {
                            return;
                        }
                    }
                }
            }
        }

        if !land_war {
            if let Some(wa) = water_area {
                if water_percent > 40 {
                    if self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_SPY) > 0 {
                        if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_SPY_SEA) == 0 {
                            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SPY_SEA)) {
                                return;
                            }
                        }
                    }
                }
            }
        }

        if best_spread_unit_value > (spread_unit_threshold * 40) / 100 {
            if let Some(u) = best_spread_unit {
                if self.city_ai_choose_unit_2(g, c, u, UnitAIType::UNITAI_MISSIONARY) {
                    return;
                }
            }
        }

        if total_floating_defenders < needed_floating_defenders {
            if self.rand(100, "AI Build Floating Defender") < 50 {
                if self.city_ai_choose_least_represented_unit(g, c, &mut floating_defender_types.clone()) {
                    return;
                }
            }
        }

        let num_spies = self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_SPY);
        let mut needed_spies = num_cities_in_area / 3;
        needed_spies += if g.city_is_capital(c) { 1 } else { 0 };
        if num_spies < needed_spies {
            if self.rand(100, "AI Train Spy") < 5 + 50 / (1 + num_spies) {
                if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SPY)) {
                    return;
                }
            }
        }

        if land_war {
            if num_settlers < max_settlers {
                if !financial_trouble {
                    if area_best_found_value > min_found_value {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_SETTLE)) {
                            return;
                        }
                    }
                }
            }
        }

        if production_rank <= (if num_cities > 8 { 3 } else { 2 }) && g.cities[c].population > 3 {
            let mut wonder_rand = 8 + self.rand(wonder_construct_rand, "Wonder Construction Rand");

            // increase chance of going for an early wonder
            let construct_percent = g.d.game_speeds[g.game_speed_type()].construct_percent;
            if g.elapsed_game_turns < (100 * construct_percent / 100) && num_cities_in_area > 1 {
                wonder_rand *= 35;
                wonder_rand /= 100;
            } else if num_cities_in_area >= 3 {
                wonder_rand *= 30;
                wonder_rand /= 100;
            } else {
                wonder_rand *= 25;
                wonder_rand /= 100;
            }

            if aggressive_ai {
                wonder_rand *= 2;
                wonder_rand /= 3;
            }

            let mut wonder_roll = self.rand(100, "Wonder Build Rand");

            if production_rank == 1 {
                wonder_roll /= 2;
            }

            if wonder_roll < wonder_rand {
                let mut wonder_max_turns = 20 + ((wonder_rand - wonder_roll) * 2);
                if land_war {
                    wonder_max_turns /= 2;
                }

                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_WORLDWONDER, wonder_max_turns, 0) {
                    return;
                }
            }
        }

        if unit_cost_percentage < max_unit_spending + 4 {
            if aircraft_have * 2 >= aircraft_need && aircraft_have < aircraft_need {
                if self.city_ai_choose_least_represented_unit(g, c, &mut air_unit_types.clone()) {
                    return;
                }
            }
        }

        if !land_war {
            if culture_pressure > 90 || self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, 20, 0) {
                    return;
                }
            }

            if let Some(wa) = water_area {
                if self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_MISSIONARY) > 0 {
                    if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_MISSIONARY_SEA) == 0 {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_MISSIONARY_SEA)) {
                            return;
                        }
                    }
                }
            }
        }

        if g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE) == 0 {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, 30, 0) {
                return;
            }
        }

        if !always_peace && (danger || self.rand(30, "AI Build Defense") < g.cities[c].population) {
            if !danger {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_EXPERIENCE, 20, 0) {
                    return;
                }
            }

            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_DEFENSE, 20, 0) {
                return;
            }

            if danger {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_EXPERIENCE, 20, 0) {
                    return;
                }
            }
        }

        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_PRODUCTION, 20, 4) {
            return;
        }

        //20 means 5g or ~2 happiness...
        if self.city_ai_choose_building(g, c, economy_flags, 15, 20) {
            return;
        }

        if !land_war {
            if self.city_ai_choose_building(g, c, economy_flags, 40, 8) {
                return;
            }

            if culture_pressure > 50 {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, 60, 0) {
                    return;
                }
            }

            if let Some(wa) = water_area {
                if primary_area {
                    if self.player_ai_total_water_area_unit_ais(g, p, wa, UnitAIType::UNITAI_EXPLORE_SEA) < 1.min(self.player_ai_needed_explorers(g, p, wa)) {
                        if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_EXPLORE_SEA)) {
                            return;
                        }
                    }
                }
            }

            if g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] >= 8 {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_PRODUCTION, 80, 0) {
                    return;
                }
            }
        }

        if g.plot_plot_check(plot, Game::puf_is_unit_ai_type, UnitAIType::UNITAI_CITY_COUNTER.i(), -1, Some(p), None, None, -1, -1).is_none() {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_COUNTER)) {
                return;
            }
        }

        // we do a similar check lower, in the landwar case
        if !land_war && financial_trouble {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_GOLD, MAX_INT, 0) {
                return;
            }
        }

        let mut choose_unit = false;
        if unit_cost_percentage < max_unit_spending + 5 {
            if land_war
                || (num_cities <= 3 && g.elapsed_game_turns < 60)
                || self.rand(100, "AI Build Unit Production") < self.city_ai_build_unit_prob(g, c)
                || (g.city_is_human(c) && g.cities[c].game_turn_founded == g.game_turn())
            {
                if self.city_ai_choose_unit(g, c, None) {
                    return;
                }

                choose_unit = true;
            }
        }

        if self.city_ai_choose_project(g, c) {
            return;
        }

        if self.city_ai_choose_building(g, c, 0, MAX_INT, 0) {
            return;
        }

        if !choose_unit && !financial_trouble && self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FINAL_WAR) {
            if self.city_ai_choose_unit(g, c, None) {
                return;
            }
        }

        if self.city_ai_choose_process(g, c, None) {
            return;
        }
    }

    /// C++ `UnitTypes CvCityAI::AI_bestUnit(bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR, UnitAITypes* peBestUnitAI = NULL)` (CvCityAI.cpp:2266-2526)
    pub fn city_ai_best_unit(&self, g: &Game, c: CityId, async_: bool, ignore_advisor: Option<AdvisorType>, best_unit_ai: &mut Option<UnitAIType>) -> Option<UnitType> {
        use UnitAIType as U;

        let mut ai_unit_ai_val = [0i32; UnitAIType::NUM];
        let mut coastal_cities = 0;

        *best_unit_ai = None;

        let p = g.cities[c].owner;
        let t = g.city_team(c);
        let area = g.city_area(c);
        let water_area = g.city_water_area(c);
        let area_ai = g.map.areas[area].area_ai_type[t];
        let num_cities = g.players[p].num_cities();

        let war_plan = g.team_get_any_war_plan_count(t, true) > 0;
        let defense = area_ai == Some(AreaAIType::AREAAI_DEFENSIVE);
        let land_war = defense || area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING);
        let assault = area_ai == Some(AreaAIType::AREAAI_ASSAULT);
        let primary_area = self.player_ai_is_primary_area(g, p, area);
        let area_alone = self.player_ai_is_area_alone(g, p, area);
        let financial_trouble = self.player_ai_is_financial_trouble(g, p);
        let war_possible = self.team_ai_is_war_possible(g, t);
        let danger = self.city_ai_is_danger(g, c);

        let has_met_count = g.team_get_has_met_civ_count(t, true);
        let military_weight = self.player_ai_military_weight(g, p, area);
        let num_cities_in_area = g.map.areas[area].cities_per_player[p];

        if let Some(wa) = water_area {
            coastal_cities = g.player_count_num_coastal_cities_by_area(p, wa);
        }

        let found = if primary_area { g.player_find_best_found_value(p) > 0 } else { g.map.areas[area].best_found_value[p] > 0 };
        if !financial_trouble && found {
            ai_unit_ai_val[U::UNITAI_SETTLE.idx()] += 1;
        }

        ai_unit_ai_val[U::UNITAI_WORKER.idx()] += self.player_ai_needed_workers(g, p, area);

        ai_unit_ai_val[U::UNITAI_ATTACK.idx()] += (military_weight / if war_plan || land_war || assault { 7 } else { 12 }) + (if primary_area && war_possible { 2 } else { 0 }) + 1;

        ai_unit_ai_val[U::UNITAI_CITY_DEFENSE.idx()] += num_cities_in_area + 1;
        ai_unit_ai_val[U::UNITAI_CITY_COUNTER.idx()] += (5 * (num_cities_in_area + 1)) / 8;
        ai_unit_ai_val[U::UNITAI_CITY_SPECIAL.idx()] += (num_cities_in_area + 1) / 2;

        let pa1 = if primary_area { 1 } else { 0 };

        if war_possible {
            ai_unit_ai_val[U::UNITAI_ATTACK_CITY.idx()] += (military_weight / if war_plan || land_war || assault { 10 } else { 17 }) + pa1;
            ai_unit_ai_val[U::UNITAI_COUNTER.idx()] += (military_weight / if war_plan || land_war || assault { 13 } else { 22 }) + pa1;
            ai_unit_ai_val[U::UNITAI_PARADROP.idx()] += (military_weight / if war_plan || land_war || assault { 5 } else { 8 }) + pa1;

            ai_unit_ai_val[U::UNITAI_DEFENSE_AIR.idx()] += num_cities + 1;
            ai_unit_ai_val[U::UNITAI_CARRIER_AIR.idx()] += self.player_ai_count_cargo_space(g, p, U::UNITAI_CARRIER_SEA);
            ai_unit_ai_val[U::UNITAI_MISSILE_AIR.idx()] += self.player_ai_count_cargo_space(g, p, U::UNITAI_MISSILE_CARRIER_SEA);

            if primary_area {
                let alive = g.count_civ_players_alive();
                ai_unit_ai_val[U::UNITAI_ICBM.idx()] += (g.players[p].total_population / 25).max((alive + g.count_total_nuke_units()) / (alive + 1));
            }
        }

        if g.city_is_barbarian(c) {
            ai_unit_ai_val[U::UNITAI_ATTACK.idx()] *= 2;
        } else {
            if !land_war {
                ai_unit_ai_val[U::UNITAI_EXPLORE.idx()] += self.player_ai_needed_explorers(g, p, area);
            }

            if let Some(wa) = water_area {
                ai_unit_ai_val[U::UNITAI_WORKER_SEA.idx()] += self.city_ai_needed_sea_workers(g, c);

                let water_tiles = g.map.areas[wa].num_tiles;

                if num_cities > 3 || g.area_get_num_unowned_tiles(area) < 10 {
                    if primary_area {
                        ai_unit_ai_val[U::UNITAI_EXPLORE_SEA.idx()] += self.player_ai_needed_explorers(g, p, wa);
                    }

                    if primary_area && g.player_find_best_found_value(p) > 0 && water_tiles > 300 {
                        ai_unit_ai_val[U::UNITAI_SETTLER_SEA.idx()] += 1;
                    }

                    if primary_area && self.player_ai_total_area_unit_ais(g, p, area, U::UNITAI_MISSIONARY) > 0 && water_tiles > 400 {
                        ai_unit_ai_val[U::UNITAI_MISSIONARY_SEA.idx()] += 1;
                    }

                    if primary_area && self.player_ai_total_area_unit_ais(g, p, area, U::UNITAI_SPY) > 0 && water_tiles > 500 {
                        ai_unit_ai_val[U::UNITAI_SPY_SEA.idx()] += 1;
                    }

                    ai_unit_ai_val[U::UNITAI_PIRATE_SEA.idx()] += water_tiles / 600;

                    if war_possible {
                        ai_unit_ai_val[U::UNITAI_ATTACK_SEA.idx()] += (water_tiles / 150).min((((coastal_cities * 2) + (military_weight / 9)) / if assault { 4 } else { 6 }) + pa1);
                        ai_unit_ai_val[U::UNITAI_RESERVE_SEA.idx()] += (water_tiles / 200).min((((coastal_cities * 2) + (military_weight / 7)) / 5) + pa1);
                        ai_unit_ai_val[U::UNITAI_ESCORT_SEA.idx()] += self.player_ai_total_water_area_unit_ais(g, p, wa, U::UNITAI_ASSAULT_SEA) + (self.player_ai_total_water_area_unit_ais(g, p, wa, U::UNITAI_CARRIER_SEA) * 2);
                        ai_unit_ai_val[U::UNITAI_ASSAULT_SEA.idx()] += (water_tiles / 250).min((((coastal_cities * 2) + (military_weight / 6)) / if assault { 5 } else { 8 }) + pa1);
                        ai_unit_ai_val[U::UNITAI_CARRIER_SEA.idx()] += (water_tiles / 350).min((((coastal_cities * 2) + (military_weight / 8)) / 7) + pa1);
                        ai_unit_ai_val[U::UNITAI_MISSILE_CARRIER_SEA.idx()] += (water_tiles / 350).min((((coastal_cities * 2) + (military_weight / 8)) / 7) + pa1);
                    }
                }
            }

            if has_met_count > 0 && war_possible {
                if land_war || assault || !financial_trouble || g.player_calculate_unit_cost(p) == 0 {
                    let pa = if primary_area && !area_alone { 1 } else { 0 };
                    ai_unit_ai_val[U::UNITAI_ATTACK.idx()] += (military_weight / if land_war || assault { 9 } else { 16 }) + pa;
                    ai_unit_ai_val[U::UNITAI_ATTACK_CITY.idx()] += (military_weight / if land_war || assault { 7 } else { 15 }) + pa;
                    ai_unit_ai_val[U::UNITAI_COLLATERAL.idx()] += (military_weight / if defense { 8 } else { 14 }) + pa;
                    ai_unit_ai_val[U::UNITAI_PILLAGE.idx()] += (military_weight / if land_war || assault { 10 } else { 19 }) + pa;
                    ai_unit_ai_val[U::UNITAI_RESERVE.idx()] += (military_weight / if land_war { 12 } else { 17 }) + pa;
                    ai_unit_ai_val[U::UNITAI_COUNTER.idx()] += (military_weight / if land_war || assault { 9 } else { 16 }) + pa;
                    ai_unit_ai_val[U::UNITAI_PARADROP.idx()] += (military_weight / if land_war || assault { 4 } else { 8 }) + pa;

                    ai_unit_ai_val[U::UNITAI_ATTACK_AIR.idx()] += num_cities + 1;

                    if let Some(wa) = water_area {
                        if num_cities > 3 || g.area_get_num_unowned_tiles(area) < 10 {
                            let water_tiles = g.map.areas[wa].num_tiles;
                            ai_unit_ai_val[U::UNITAI_ATTACK_SEA.idx()] += (water_tiles / 100).min((((coastal_cities * 2) + (military_weight / 10)) / if assault { 5 } else { 7 }) + pa1);
                            ai_unit_ai_val[U::UNITAI_RESERVE_SEA.idx()] += (water_tiles / 150).min((((coastal_cities * 2) + (military_weight / 11)) / 8) + pa1);
                        }
                    }
                }
            }
        }

        // XXX this should account for air and heli units too...
        for i in 0..UnitAIType::NUM {
            let ai = UnitAIType::from_usize(i);
            let domain = self.player_ai_unit_ai_domain_type(g, p, ai);
            if domain == Some(DomainType::DOMAIN_SEA) {
                if let Some(wa) = water_area {
                    ai_unit_ai_val[i] -= self.player_ai_total_water_area_unit_ais(g, p, wa, ai);
                }
            } else if domain == Some(DomainType::DOMAIN_AIR) || ai == U::UNITAI_ICBM {
                ai_unit_ai_val[i] -= self.player_ai_total_unit_ais(g, p, ai);
            } else {
                ai_unit_ai_val[i] -= self.player_ai_total_area_unit_ais(g, p, area, ai);
            }
        }

        ai_unit_ai_val[U::UNITAI_SETTLE.idx()] *= if danger { 8 } else { 20 };
        ai_unit_ai_val[U::UNITAI_WORKER.idx()] *= if danger { 2 } else { 7 };
        ai_unit_ai_val[U::UNITAI_ATTACK.idx()] *= 3;
        ai_unit_ai_val[U::UNITAI_ATTACK_CITY.idx()] *= 4;
        ai_unit_ai_val[U::UNITAI_COLLATERAL.idx()] *= 5;
        ai_unit_ai_val[U::UNITAI_PILLAGE.idx()] *= 3;
        ai_unit_ai_val[U::UNITAI_RESERVE.idx()] *= 3;
        ai_unit_ai_val[U::UNITAI_COUNTER.idx()] *= 3;
        ai_unit_ai_val[U::UNITAI_COUNTER.idx()] *= 2;
        ai_unit_ai_val[U::UNITAI_CITY_DEFENSE.idx()] *= 2;
        ai_unit_ai_val[U::UNITAI_CITY_COUNTER.idx()] *= 2;
        ai_unit_ai_val[U::UNITAI_CITY_SPECIAL.idx()] *= 2;
        ai_unit_ai_val[U::UNITAI_EXPLORE.idx()] *= if danger { 6 } else { 15 };
        ai_unit_ai_val[U::UNITAI_ICBM.idx()] *= 18;
        ai_unit_ai_val[U::UNITAI_WORKER_SEA.idx()] *= if danger { 3 } else { 10 };
        ai_unit_ai_val[U::UNITAI_ATTACK_SEA.idx()] *= 5;
        ai_unit_ai_val[U::UNITAI_RESERVE_SEA.idx()] *= 4;
        ai_unit_ai_val[U::UNITAI_ESCORT_SEA.idx()] *= 20;
        ai_unit_ai_val[U::UNITAI_EXPLORE_SEA.idx()] *= 18;
        ai_unit_ai_val[U::UNITAI_ASSAULT_SEA.idx()] *= 14;
        ai_unit_ai_val[U::UNITAI_SETTLER_SEA.idx()] *= 16;
        ai_unit_ai_val[U::UNITAI_MISSIONARY_SEA.idx()] *= 12;
        ai_unit_ai_val[U::UNITAI_SPY_SEA.idx()] *= 10;
        ai_unit_ai_val[U::UNITAI_CARRIER_SEA.idx()] *= 8;
        ai_unit_ai_val[U::UNITAI_MISSILE_CARRIER_SEA.idx()] *= 8;
        ai_unit_ai_val[U::UNITAI_PIRATE_SEA.idx()] *= 5;
        ai_unit_ai_val[U::UNITAI_ATTACK_AIR.idx()] *= 6;
        ai_unit_ai_val[U::UNITAI_DEFENSE_AIR.idx()] *= 3;
        ai_unit_ai_val[U::UNITAI_CARRIER_AIR.idx()] *= 15;
        ai_unit_ai_val[U::UNITAI_MISSILE_AIR.idx()] *= 15;

        let personality = g.city_personality_type(c);
        for i in 0..UnitAIType::NUM {
            ai_unit_ai_val[i] *= (g.d.leader_heads[personality].unit_ai_weight_modifier[UnitAIType::from_usize(i)] + 100).max(0);
            ai_unit_ai_val[i] /= 100;
        }

        //FfH: Added by Kael 04/29/2009 *remove unused unitai's*
        ai_unit_ai_val[U::UNITAI_ICBM.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_SPY_SEA.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_CARRIER_SEA.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_MISSILE_CARRIER_SEA.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_ATTACK_AIR.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_DEFENSE_AIR.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_CARRIER_AIR.idx()] = 0;
        ai_unit_ai_val[U::UNITAI_MISSILE_AIR.idx()] = 0;
        //FfH: End Add

        let mut best_value = 0;
        let mut best_unit: Option<UnitType> = None;

        for i in 0..UnitAIType::NUM {
            if ai_unit_ai_val[i] > 0 {
                // C++: GC.getASyncRand() when bAsync, else getSorenRandNum: both the AI stream here.
                if async_ {
                    ai_unit_ai_val[i] += self.rand(military_weight, "AI Best UnitAI ASYNC");
                } else {
                    ai_unit_ai_val[i] += self.rand(military_weight, "AI Best UnitAI");
                }

                if ai_unit_ai_val[i] > best_value {
                    let ai = UnitAIType::from_usize(i);
                    let unit = self.city_ai_best_unit_ai(g, c, ai, async_, ignore_advisor);

                    if let Some(unit) = unit {
                        best_value = ai_unit_ai_val[i];
                        best_unit = Some(unit);
                        *best_unit_ai = Some(ai);
                    }
                }
            }
        }

        best_unit
    }

    /// C++ `UnitTypes CvCityAI::AI_bestUnitAI(UnitAITypes eUnitAI, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2529-2746)
    pub fn city_ai_best_unit_ai(&self, g: &Game, c: CityId, unit_ai: UnitAIType, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<UnitType> {
        let p = g.cities[c].owner;
        let area = g.city_area(c);
        let human = g.city_is_human(c);
        let population = g.cities[c].population;
        let num_cities = g.players[p].num_cities();

        let mut grow_more = false;

        if g.city_food_difference(c, true) > 0 {
            if num_cities <= 2 {
                grow_more = population < 3 && self.city_ai_count_good_tiles(g, c, true, false, 100, false) >= population;
            } else {
                grow_more = population < 3 || self.city_ai_count_good_tiles(g, c, true, false, 100, false) >= population;
            }
            if !grow_more && population < 6 && self.city_ai_count_good_tiles(g, c, true, false, 80, false) >= population {
                if (g.cities[c].food - (g.cities[c].food_kept / 2)) >= (g.city_growth_threshold(c) / 2) {
                    if g.city_angry_population(c, 1) == 0 && g.city_health_rate(c, false, 1) == 0 {
                        grow_more = true;
                    }
                }
            }
        }

        let civ = g.city_civilization_type(c);

        // The candidate units: C++ walks the unit classes twice with the same filter (the
        // first pass for the best AI_unitValue, the second to score); AI_unitValue is a pure
        // valuation, so it is computed once per candidate here.
        // PORT ADAPT: ffh-data does not read units' `<Advisor>`, so every unit counts as
        // NO_ADVISOR and passes `getAdvisorType() != eIgnoreAdvisor` for any advisor.
        let mut candidates: Vec<(UnitClassType, UnitType, i32)> = Vec::new();
        let mut best_original_value = 0;
        for class in g.d.unit_classes.keys() {
            let Some(loop_unit) = g.d.civilizations[civ].civilization_units(class, g.d.unit_classes[class].default_unit_index) else { continue };
            if human && g.d.units[loop_unit].default_unit_ai_type != Some(unit_ai) {
                continue;
            }
            if grow_more && g.city_is_food_production_unit(c, loop_unit) {
                continue;
            }
            if !g.city_can_train(c, loop_unit, false, false, false, false) {
                continue;
            }
            let original_value = self.player_ai_unit_value(g, p, loop_unit, unit_ai, area);
            if original_value > best_original_value {
                best_original_value = original_value;
            }
            candidates.push((class, loop_unit, original_value));
        }

        let mut best_value = 0;
        let mut best_unit: Option<UnitType> = None;

        for &(class, loop_unit, original_value) in &candidates {
            let mut value = original_value;

            if value > (best_original_value * 2) / 3 && (unit_ai != UnitAIType::UNITAI_EXPLORE || value >= best_original_value) {
                let info = &g.d.units[loop_unit];
                value *= g.city_production_experience(c, Some(loop_unit)) + 10;
                value /= 10;

                //free promotions. slow?
                //only 1 promotion per source is counted (ie protective isn't counted twice)
                let mut promotion_value = 0;
                //buildings
                for promo in g.d.promotions.keys() {
                    if g.city_is_free_promotion(c, promo) && !info.free_promotions[promo] {
                        if let Some(combat) = info.unit_combat_type {
                            if g.d.promotions[promo].unit_combat[combat] {
                                promotion_value += 15;
                                break;
                            }
                        }
                    }
                }

                //special to the unit
                for promo in g.d.promotions.keys() {
                    if info.free_promotions[promo] {
                        promotion_value += 15;
                        break;
                    }
                }

                //traits
                for tr in g.d.traits.keys() {
                    if g.city_has_trait(c, tr) {
                        for promo in g.d.promotions.keys() {
                            if g.d.traits[tr].free_promotion[promo] {
                                if let Some(combat) = info.unit_combat_type {
                                    if g.d.traits[tr].free_promotion_unit_combat[combat] {
                                        promotion_value += 15;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                value *= promotion_value + 100;
                value /= 100;

                // C++: GC.getASyncRand() when bAsync, else getSorenRandNum: both the AI stream here.
                if async_ {
                    value *= self.rand(50, "AI Best Unit ASYNC") + 100;
                    value /= 100;
                } else {
                    value *= self.rand(50, "AI Best Unit") + 100;
                    value /= 100;
                }

                let mut best_happy = 0;
                for hurry in g.d.hurries.keys() {
                    if g.city_can_hurry_unit(c, hurry, loop_unit, true) {
                        let happy = self.city_ai_get_happy_from_hurry_3(g, c, hurry, loop_unit, true);
                        if happy > best_happy {
                            best_happy = happy;
                        }
                    }
                }

                if best_happy == 0 {
                    value += g.cities[c].unit_production[loop_unit];
                }

                value *= num_cities * 2;
                value /= g.player_get_unit_class_count_plus_making(p, class) + num_cities + 1;

                value = value.wrapping_mul(1000);

                let is_suicide = info.suicide;

                if is_suicide {
                    //much of this is compensated
                    value /= 3;
                }

                if best_happy == 0 {
                    value /= (g.city_production_turns_left_unit(c, loop_unit, 0) + if info.suicide { 1 } else { 4 }).max(1);
                } else {
                    value *= 2 + 3 * best_happy;
                    value /= 100;
                }

                //FfH: Added by Kael 04/29/2009
                if let Some(lh) = g.players[p].personality_type {
                    if let Some(fav) = g.d.leader_heads[lh].favorite_unit_combat {
                        if info.unit_combat_type == Some(fav) {
                            value += 100;
                        }
                    }
                }
                //FfH: End Add

                value = value.max(1);

                if value > best_value {
                    best_value = value;
                    best_unit = Some(loop_unit);
                }
            }
        }

        best_unit
    }
}

/// C++ `pPlot->plotCount(PUF_isUnitAIType, eUnitAI, -1, eOwner)`.
fn plot_unit_ai_count(g: &Game, plot: PlotIdx, unit_ai: UnitAIType, owner: PlayerId) -> i32 {
    g.plot_plot_count(plot, Some(Game::puf_is_unit_ai_type), unit_ai.i(), -1, Some(owner), None, None, -1, -1)
}

/// C++ `CvCityAI::AI_countNumBonuses(NO_BONUS, bIncludeOurs, bIncludeNeutral,
/// iOtherCultureThreshold, bLand, bWater)` (CvCityAI.cpp:9592-9635): the `Cai` method in
/// city_4.rs takes a `BonusType` and cannot express NO_BONUS (see SIGNATURE NOTES).
fn count_num_bonuses_any(g: &Game, c: CityId, include_ours: bool, include_neutral: bool, other_culture_threshold: i32, land: bool, water: bool) -> i32 {
    let owner = g.cities[c].owner;
    let team = g.city_team(c);
    let area = g.city_area(c);
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let mut count = 0;
    for i in 0..NUM_CITY_PLOTS {
        let Some(loop_plot) = g.plot_city(x, y, i) else { continue };
        if g.map.plots[loop_plot].area == Some(area) || (water && g.plot_is_water(loop_plot)) {
            let loop_bonus = g.plot_get_bonus_type(loop_plot, Some(team));
            if loop_bonus.is_some() {
                let plot_owner = g.map.plots[loop_plot].owner;
                if include_ours && plot_owner == Some(owner) && g.map.plots[loop_plot].working_city == Some(c) {
                    count += 1;
                } else if include_neutral && !g.plot_is_owned(loop_plot) {
                    count += 1;
                } else if other_culture_threshold > 0 && (g.plot_is_owned(loop_plot) && plot_owner != Some(owner)) {
                    let po = plot_owner.unwrap();
                    if g.map.plots[loop_plot].culture[po] - g.map.plots[loop_plot].culture[owner] < other_culture_threshold {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}
