//! Port of CvPlayerAI.cpp:3368-6803 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES (for the integrator):
//! - `AI_bestTech` calls `AI_bestAreaUnitAIValue(UNITAI_ASSAULT_SEA, NULL, &eUnit)` (NULL area,
//!   i.e. `AI_bestCityUnitAIValue(.., NULL, ..)` and `AI_unitValue(.., NULL)`), but
//!   `player_ai_best_area_unit_ai_value` / `player_ai_best_city_unit_ai_value` /
//!   `player_ai_unit_value` take a non-optional `AreaId` / `CityId`. Ported here as the private
//!   `best_unit_ai_value_no_city` (PORT ADAPT: the capital's area stands in for NULL in
//!   `AI_unitValue`). Making those parameters `Option<..>` would allow the faithful call.
//! - `AI_chooseResearch` / `AI_chooseFreeTech` return the tech C++ would push / grant (the rules
//!   push or grant it themselves; see their doc comments).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `int CvPlayerAI::AI_getWaterDanger(CvPlot* pPlot, int iRange, bool bTestMoves = true) const` (CvPlayerAI.cpp:3368-3424)
    pub fn player_ai_get_water_danger(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> i32 {
        let team = g.players[p].team;
        let mut count = 0;

        let range = if range == -1 { DANGER_RANGE } else { range };

        // C++ computes pPlot->waterArea() here but never uses it.
        let (px, py) = g.map.xy(plot);

        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                    continue;
                };

                if !g.plot_is_water(loop_plot) {
                    continue;
                }

                let Some(loop_area) = g.map.plots[loop_plot].area else {
                    continue;
                };

                if !g.plot_is_adjacent_to_area(plot, loop_area) {
                    continue;
                }

                for &u in &g.map.plots[loop_plot].units {
                    if g.unit_is_enemy(u, Some(team), None) {
                        if g.unit_can_attack(u) {
                            if !g.unit_is_invisible(u, team, false) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `bool CvPlayerAI::AI_avoidScience() const` (CvPlayerAI.cpp:3427-3444)
    pub fn player_ai_avoid_science(&self, g: &Game, p: PlayerId) -> bool {
        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE4) {
            return true;
        }
        if g.player_is_current_research_repeat(p) {
            return true;
        }

        if g.player_is_no_research_available(p) {
            return true;
        }

        false
    }

    /// C++ `bool CvPlayerAI::AI_isFinancialTrouble() const` (CvPlayerAI.cpp:3448-3505)
    pub fn player_ai_is_financial_trouble(&self, g: &Game, p: PlayerId) -> bool {
        if g.player_is_barbarian(p) {
            return false;
        }

        let gold_per_turn = g.players[p].gold_per_turn;
        let net_commerce = 1
            + g.player_get_commerce_rate(p, CommerceType::COMMERCE_GOLD)
            + g.player_get_commerce_rate(p, CommerceType::COMMERCE_RESEARCH)
            + gold_per_turn.max(0);
        // UNOFFICIAL_PATCH: std::max(0, -getGoldPerTurn()).
        let net_expenses = g.player_calculate_inflated_costs(p) + (-gold_per_turn).max(0);

        let funded_percent = (100 * (net_commerce - net_expenses)) / net_commerce.max(1);

        let mut safe_percent = 40;
        if self.player_ai_avoid_science(g, p) {
            safe_percent -= 8;
        }

        let team = g.players[p].team;
        if g.team_get_any_war_plan_count(team, true) != 0 {
            safe_percent -= 12;
        }

        if g.player_is_current_research_repeat(p) {
            safe_percent -= 10;
        }

        funded_percent < safe_percent
    }

    /// C++ `int CvPlayerAI::AI_goldTarget() const` (CvPlayerAI.cpp:3507-3558)
    pub fn player_ai_gold_target(&self, g: &Game, p: PlayerId) -> i32 {
        let d = g.d;
        let mut gold = 0;

        if g.elapsed_game_turns >= 40 {
            let gs = &d.game_speeds[g.game_speed_type()];
            let mut multiplier = 0;
            multiplier += gs.research_percent;
            multiplier += gs.train_percent;
            multiplier += gs.construct_percent;
            multiplier /= 3;

            gold += (g.players[p].num_cities() * 3) + (g.players[p].total_population / 3);

            gold += g.elapsed_game_turns / 2;

            gold *= multiplier;
            gold /= 100;

            let team = g.players[p].team;
            let any_war = g.team_get_any_war_plan_count(team, true) > 0;
            if any_war {
                gold *= 3;
                gold /= 2;
            }

            if self.player_ai_avoid_science(g, p) {
                gold *= 10;
            }

            gold += self.player_ai_gold_to_upgrade_all_units(g, p, 0) / (if any_war { 1 } else { 2 });

            let mut active_corporation: Option<CorporationType> = None;
            for corp in d.corporations.keys() {
                if g.player_get_has_corporation_count(p, corp) > 0 {
                    active_corporation = Some(corp);
                    break;
                }
            }
            if let Some(corp) = active_corporation {
                let mut spread_cost = 0.max(d.corporations[corp].spread_cost * (100 + g.player_calculate_inflation_rate(p)));
                spread_cost /= 50;
                gold += spread_cost;
            }
        }

        let extra = self.s().player(p).extra_gold_target;
        gold + extra
    }

    /// C++ `int CvPlayerAI::AI_getGoldReserve() const` (CvPlayerAI.cpp:3566-3590)
    pub fn player_ai_get_gold_reserve(&self, g: &Game, p: PlayerId) -> i32 {
        if g.players[p].human {
            return 0;
        }
        let d = g.d;
        let mut gold = 0;
        let civ = g.player_get_civilization_type(p).map_or(-1, |c| c.0 as i32);
        if civ == d.defines.civilization_khazad {
            let has = |define: i32| opt_from_i32::<TechType>(define).map_or(false, |tech| g.player_is_has_tech(p, tech));
            if has(d.defines.tech_taxation) {
                gold += 200;
            }
            if has(d.defines.tech_way_of_the_earthmother) {
                gold += 150;
            }
            if has(d.defines.tech_festivals) {
                gold += 150;
            }
            gold *= g.players[p].num_cities();
        }
        gold
    }

    /// C++ `TechTypes CvPlayerAI::AI_bestTech(int iMaxPathLength = 1, bool bIgnoreCost = false, bool bAsync = false, TechTypes eIgnoreTech = NO_TECH, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR) const` (CvPlayerAI.cpp:3595-5202)
    ///
    /// `bAsync` draws come from the AI's stream too ([`Cai::rand`]).
    /// PORT GAP: `eIgnoreAdvisor` is ignored: the tech advisor types are not resolved in
    /// ffh-data (`TechInfo::advisor_type` is a string); only the tech-choice popup sets it.
    pub fn player_ai_best_tech(&self, g: &Game, p: PlayerId, max_path_length: i32, ignore_cost: bool, async_: bool, ignore_tech: Option<TechType>, ignore_advisor: Option<AdvisorType>) -> Option<TechType> {
        let d = g.d;

        //FfH: Added by Kael 10/30/2008
        const AI_FLAVOR_MILITARY: FlavorType = FlavorType(0);

        let capital_city = g.player_get_capital_city(p);

        let t = g.players[p].team;
        let civ = civ_of(g, p);
        let leader = lh(g, p);
        let human = g.players[p].human;

        let war_plan = g.team_get_any_war_plan_count(t, true) > 0;
        let capital_alone = if g.elapsed_game_turns > 0 { self.player_ai_is_capital_area_alone(g, p) } else { false };
        let financial_trouble = self.player_ai_is_financial_trouble(g, p);
        let advanced_start = g.players[p].advanced_start_points >= 0;

        let has_met_count = g.team_get_has_met_civ_count(t, true);
        let coastal_cities = g.player_count_num_coastal_cities(p);
        let connected_foreign_cities = g.player_count_potential_foreign_trade_cities_connected(p);

        let num_cities = g.players[p].num_cities();
        let city_count = num_cities;
        let team_city_count = g.teams[t].num_cities;
        let total_population = g.players[p].total_population;

        let mut best_value = 0;
        let mut best_tech: Option<TechType> = None;

        // Strategy flags (AI_isDoStrategy has no side effect but its cache).
        let strat_dagger = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_DAGGER);
        let strat_culture1 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1);
        let strat_culture2 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2);
        let strat_culture3 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3);

        // AI_yieldWeight / AI_commerceWeight(NULL) are pure: computed once.
        let mut yield_weight = [0; YieldType::NUM];
        for k in 0..YieldType::NUM {
            yield_weight[k] = self.player_ai_yield_weight(g, p, YieldType::from_usize(k));
        }
        let mut commerce_weight = [0; CommerceType::NUM];
        for k in 0..CommerceType::NUM {
            commerce_weight[k] = self.player_ai_commerce_weight(g, p, CommerceType::from_usize(k), None);
        }

        let food = YieldType::YIELD_FOOD.idx();
        let commerce_y = YieldType::YIELD_COMMERCE.idx();
        let culture = CommerceType::COMMERCE_CULTURE.idx();

        let nbc = d.bonus_classes.len();
        let mut bonus_class_revealed = vec![0i32; nbc];
        let mut bonus_class_unrevealed = vec![0i32; nbc];
        let mut bonus_class_have = vec![0i32; nbc];

        for bonus in d.bonuses.keys() {
            let bi = &d.bonuses[bonus];
            if let Some(reveal_tech) = bi.tech_reveal {
                // PORT ADAPT: a bonus without a class would index -1 in C++.
                let Some(bonus_class) = bi.bonus_class_type else {
                    continue;
                };
                let c = bonus_class.idx();
                if g.team_is_has_tech(t, Some(reveal_tech)) {
                    bonus_class_revealed[c] += 1;
                } else {
                    bonus_class_unrevealed[c] += 1;
                }

                if g.player_get_num_available_bonuses(p, bonus) > 0 {
                    bonus_class_have[c] += 1;
                } else if g.player_count_owned_bonuses(p, bonus) > 0 {
                    bonus_class_have[c] += 1;
                }
            }
        }

        let mut percent_of_domination = 0;
        let our_pop_percent = 100 * g.team_get_total_population(t, true) / g.total_population.max(1);

        for victory in d.victories.keys() {
            if d.victories[victory].population_percent_lead > 0 {
                percent_of_domination = 100 * our_pop_percent / g.adjusted_population_percent(victory).max(1);
            }
        }

        let any_primary_assault = || -> bool {
            for (area, a) in g.map.areas.iter() {
                if self.player_ai_is_primary_area(g, p, area) {
                    if a.area_ai_type.get(t).copied().flatten() == Some(AreaAIType::AREAAI_ASSAULT) {
                        return true;
                    }
                }
            }
            false
        };

        for tech in d.techs.keys() {
            if ignore_tech.is_some() && ignore_tech == Some(tech) {
                continue;
            }
            // PORT GAP: eIgnoreAdvisor (see the doc comment).
            if !g.player_can_ever_research(p, tech) {
                continue;
            }
            if g.team_is_has_tech(t, Some(tech)) {
                continue;
            }

            //FfH: Modified by Kael 06/17/2009 (era check removed)

            let path_length = g.player_find_path_length(p, tech, false);

            if path_length > max_path_length {
                continue;
            }

            let ti = &d.techs[tech];

            let mut value: i32 = 1;

            let random_factor = if async_ { self.rand(2000, "AI Research ASYNC") } else { self.rand(2000, "AI Research") };
            value += random_factor;

            value += g.team_get_research_progress(t, Some(tech));

            if ti.extra_water_see_from {
                if coastal_cities > 0 {
                    value += 100;

                    if capital_alone {
                        value += 400;
                    }
                }
            }

            if ti.map_centering {
                value += 100;
            }

            if ti.map_visible {
                value += 100;

                if capital_alone {
                    value += 400;
                }
            }

            if ti.map_trading {
                value += 100;

                if capital_alone {
                    value += 400;
                }
            }

            if ti.tech_trading && !g.is_option(GameOptionType::GAMEOPTION_NO_TECH_TRADING) {
                value += 500;

                value += 500 * has_met_count;
            }

            if ti.gold_trading {
                value += 200;

                if has_met_count > 0 {
                    value += 400;
                }
            }

            if ti.open_borders_trading {
                if has_met_count > 0 {
                    value += 500;

                    if coastal_cities > 0 {
                        value += 400;
                    }
                }
            }

            if ti.defensive_pact_trading {
                value += 400;
            }

            if ti.permanent_alliance_trading {
                value += 200;
            }

            if ti.vassal_state_trading {
                value += 200;
            }

            if ti.bridge_building {
                value += 200;
            }

            if ti.irrigation {
                value += 400;
            }

            if ti.ignore_irrigation {
                value += 500;
            }

            if ti.water_work {
                value += 600 * coastal_cities;
            }

            value += ti.feature_production_modifier * 2;
            value += ti.worker_speed_modifier * 4;
            value += ti.trade_routes * ((num_cities + 2).max(connected_foreign_cities) + 1) * (if financial_trouble { 200 } else { 100 });

            if percent_of_domination < 90 {
                value += ti.health * 200;
            } else {
                value += ti.health * 350;
            }

            for route in d.routes.keys() {
                value += -(d.routes[route].tech_movement_change[tech] * 100);
            }

            for &moves in ti.domain_extra_moves.iter() {
                value += moves * 200;
            }

            for j in 0..CommerceType::NUM {
                if ti.commerce_flexible[j] {
                    value += 100;
                    if j == culture && strat_culture2 {
                        value += 1000;
                    }
                }
            }

            for terrain in d.terrains.keys() {
                if ti.terrain_trade[terrain] {
                    if d.terrains[terrain].water {
                        if let Some(cap) = capital_city {
                            value += g.player_count_potential_foreign_trade_cities(p, Some(g.city_area(cap))) * 100;
                        }

                        if coastal_cities > 0 {
                            value += if capital_alone { 950 } else { 350 };
                        }

                        value += 50;
                    } else {
                        value += 1000;
                    }
                }
            }

            if ti.river_trade {
                value += 1000;
            }

            for imp in d.improvements.keys() {
                let improvement_count = g.players[p].improvement_count[imp];
                for k in 0..YieldType::NUM {
                    let mut temp_value = 0;

                    temp_value += d.improvements[imp].tech_yield_changes[tech][k] * improvement_count * 50;

                    temp_value *= yield_weight[k];
                    temp_value /= 100;

                    value += temp_value;
                }
            }

            let mut build_value = 0;
            for build in d.builds.keys() {
                let bi = &d.builds[build];
                if bi.tech_prereq != Some(tech) {
                    continue;
                }

                let mut improvement = bi.improvement;
                if let Some(i) = improvement {
                    improvement = final_improvement_upgrade(d, i, 0);
                } else {
                    // only increment build value if it is not an improvement, otherwise handle it there
                    build_value += 200;
                }

                if let Some(improvement) = improvement {
                    let ki = &d.improvements[improvement];

                    let mut improvement_value = 300;

                    improvement_value += if ki.acts_as_city { 100 } else { 0 };
                    improvement_value += if ki.hills_makes_valid { 100 } else { 0 };
                    improvement_value += if ki.fresh_water_makes_valid { 200 } else { 0 };
                    improvement_value += if ki.river_side_makes_valid { 100 } else { 0 };
                    improvement_value += if ki.carries_irrigation { 300 } else { 0 };

                    for &valid in ki.terrain_makes_valid.iter() {
                        improvement_value += if valid { 50 } else { 0 };
                    }

                    for &valid in ki.feature_makes_valid.iter() {
                        improvement_value += if valid { 50 } else { 0 };
                    }

                    for k in 0..YieldType::NUM {
                        let mut temp_value = 0;

                        temp_value += ki.yield_change[k] * 200;
                        temp_value += ki.river_side_yield_change[k] * 100;
                        temp_value += ki.hills_yield_change[k] * 100;
                        temp_value += ki.irrigated_yield_change[k] * 150;

                        // land food yield is more valueble
                        if k == food && !ki.water {
                            temp_value *= 3;
                            temp_value /= 2;
                        }

                        if financial_trouble && k == commerce_y {
                            temp_value *= 2;
                        }

                        temp_value *= yield_weight[k];
                        temp_value /= 100;

                        improvement_value += temp_value;
                    }

                    for bonus in d.bonuses.keys() {
                        let ib = &ki.improvement_bonus[bonus];
                        let mut bonus_value = 0;

                        bonus_value += if ib.bonus_makes_valid { 450 } else { 0 };
                        bonus_value += if ib.bonus_trade { 45 * g.player_ai_bonus_val(p, bonus, 1) } else { 0 };

                        if bonus_value > 0 {
                            for l in 0..YieldType::NUM {
                                let mut temp_value = 0;

                                temp_value += ib.yield_change[l] * 300;
                                temp_value += ki.irrigated_yield_change[l] * 200;

                                // food bonuses are more valueble
                                if l == food {
                                    temp_value *= 2;
                                }
                                // otherwise, devalue the bonus slightly
                                else if l == commerce_y && financial_trouble {
                                    temp_value *= 4;
                                    temp_value /= 3;
                                } else {
                                    temp_value *= 3;
                                    temp_value /= 4;
                                }

                                if advanced_start && g.players[p].current_era.0 < 2 {
                                    // C++ quirk: multiplies the tech's whole value, not iTempValue.
                                    value = value.wrapping_mul(if l == food { 3 } else { 2 });
                                }

                                temp_value *= yield_weight[l];
                                temp_value /= 100;

                                bonus_value += temp_value;
                            }

                            let num_bonuses = g.player_count_owned_bonuses(p, bonus);

                            if num_bonuses > 0 {
                                bonus_value *= num_bonuses + 2;
                                bonus_value /= if ki.water { 4 } else { 3 }; // water resources are worth less

                                improvement_value += bonus_value;
                            }
                        }
                    }

                    // if water improvement, weight by coastal cities (weight the whole build)
                    if ki.water {
                        improvement_value *= coastal_cities;
                        improvement_value /= (city_count / 2).max(1);
                    }

                    build_value += improvement_value;
                }

                if let Some(route) = bi.route {
                    build_value += (if g.player_get_best_route(p, None).is_none() { 700 } else { 200 })
                        * (num_cities + if advanced_start { 4 } else { 0 });

                    for k in 0..YieldType::NUM {
                        let mut temp_value = 0;

                        temp_value += d.routes[route].yield_change[k] * 100;

                        for imp in d.improvements.keys() {
                            temp_value += d.improvements[imp].route_yield_changes[route][k] * 50;
                        }

                        temp_value *= yield_weight[k];
                        temp_value /= 100;

                        build_value += temp_value;
                    }
                }
            }

            // the way feature-remove is done in XML is pretty weird: counted once per feature
            for feature in d.features.keys() {
                let is_feature_remove = d.builds.iter().any(|b| b.feature_tech[feature] == Some(tech));

                if is_feature_remove {
                    build_value += 100;

                    let fi = &d.features[feature];
                    if fi.health_percent < 0
                        || (fi.yield_change[YieldType::YIELD_FOOD.idx()]
                            + fi.yield_change[YieldType::YIELD_PRODUCTION.idx()]
                            + fi.yield_change[YieldType::YIELD_COMMERCE.idx()])
                            < 0
                    {
                        build_value += 25 * g.player_count_city_features(p, feature);
                    }
                }
            }

            //FfH: Added by Kael 06/06/2008
            build_value /= 5;

            value += build_value;

            // does tech reveal bonus resources
            for bonus in d.bonuses.keys() {
                let bi = &d.bonuses[bonus];
                if bi.tech_reveal == Some(tech) {
                    let mut reveal_value = 150;
                    reveal_value += g.player_ai_bonus_val(p, bonus, 1) * 50;

                    //iMultiplier is basically a desperation value
                    //it gets larger as the AI runs out of options
                    let mut multiplier = 0;
                    // PORT ADAPT: a bonus without a class would index -1 in C++ (multiplier 0 here).
                    if let Some(bonus_class) = bi.bonus_class_type {
                        let c = bonus_class.idx();
                        let bonus_class_total = bonus_class_revealed[c] + bonus_class_unrevealed[c];
                        if bonus_class_total > 0 {
                            multiplier = bonus_class_revealed[c] - bonus_class_have[c];
                            multiplier *= 100;
                            multiplier /= bonus_class_total;

                            multiplier *= bonus_class_revealed[c] + 1;
                            multiplier /= (bonus_class_have[c] * bonus_class_total) + 1;
                        }
                    }

                    multiplier *= 3.min(num_cities);
                    multiplier /= 3;

                    reveal_value *= 100 + multiplier;
                    reveal_value /= 100;

                    value += reveal_value;
                }
            }

            let mut enables_unit_wonder = false;
            for class in d.unit_classes.keys() {
                let Some(loop_unit) = d.civ_unit(civ, class) else {
                    continue;
                };

                if !is_tech_required_for_unit(d, tech, loop_unit) {
                    continue;
                }

                let ku = &d.units[loop_unit];
                value += 200;
                let mut unit_value = 0;
                let mut naval_value = 0;

                //FfH: Modified by Kael 04/29/2009
                if d.unit_classes[class].default_unit_index != Some(loop_unit) {
                    unit_value += 200;
                }
                if let Some(favorite) = leader.favorite_unit_combat {
                    if ku.unit_combat_type == Some(favorite) {
                        unit_value += 400;
                    }
                }
                //FfH: End Modify

                if ku.prereq_and_tech != Some(tech) {
                    continue;
                }

                let mut military_value = 0;

                //FfH: Added by Kael 01/30/2009 (the naval cases also need AI_totalUnitAIs < getNumCities)
                let coastal_and_few = |ai: UnitAIType| coastal_cities > 0 && self.player_ai_total_unit_ais(g, p, ai) < num_cities;

                match ku.default_unit_ai_type {
                    Some(UnitAIType::UNITAI_UNKNOWN) | Some(UnitAIType::UNITAI_ANIMAL) => {}

                    Some(UnitAIType::UNITAI_SETTLE) => {
                        unit_value += 1200;
                    }

                    Some(UnitAIType::UNITAI_WORKER) => {
                        unit_value += 800;
                    }

                    Some(UnitAIType::UNITAI_ATTACK) => {
                        military_value += if war_plan { 600 } else { 300 };
                        military_value += if strat_dagger { 800 } else { 0 };
                        unit_value += 100;
                    }

                    Some(UnitAIType::UNITAI_ATTACK_CITY) => {
                        military_value += if war_plan { 800 } else { 400 };
                        military_value += if strat_dagger { 800 } else { 0 };
                        if ku.bombard_rate > 0 {
                            military_value += 200;

                            if self.player_ai_calculate_total_bombard(g, p, DomainType::DOMAIN_LAND) == 0 {
                                military_value += 800;
                                if strat_dagger {
                                    military_value += 1000;
                                }
                            }
                        }
                        unit_value += 100;
                    }

                    Some(UnitAIType::UNITAI_COLLATERAL) => {
                        military_value += if war_plan { 600 } else { 300 };
                        military_value += if strat_dagger { 500 } else { 0 };
                    }

                    Some(UnitAIType::UNITAI_PILLAGE) => {
                        military_value += if war_plan { 200 } else { 100 };
                    }

                    Some(UnitAIType::UNITAI_RESERVE) => {
                        military_value += if war_plan { 200 } else { 100 };
                    }

                    Some(UnitAIType::UNITAI_COUNTER) => {
                        military_value += if war_plan { 600 } else { 300 };
                        military_value += if strat_dagger { 600 } else { 0 };
                    }

                    Some(UnitAIType::UNITAI_PARADROP) => {
                        military_value += if war_plan { 600 } else { 300 };
                    }

                    Some(UnitAIType::UNITAI_CITY_DEFENSE) => {
                        military_value += if war_plan { 800 } else { 400 };
                        military_value += if !capital_alone { 400 } else { 200 };
                        unit_value += if has_met_count > 0 { 800 } else { 200 };
                    }

                    Some(UnitAIType::UNITAI_CITY_COUNTER) => {
                        military_value += if war_plan { 800 } else { 400 };
                    }

                    Some(UnitAIType::UNITAI_CITY_SPECIAL) => {
                        military_value += if war_plan { 800 } else { 400 };
                    }

                    Some(UnitAIType::UNITAI_EXPLORE) => {
                        unit_value += if capital_alone { 100 } else { 200 };
                    }

                    Some(UnitAIType::UNITAI_MISSIONARY) => {
                        unit_value += if g.player_get_state_religion(p).is_some() { 600 } else { 300 };
                    }

                    Some(UnitAIType::UNITAI_PROPHET)
                    | Some(UnitAIType::UNITAI_ARTIST)
                    | Some(UnitAIType::UNITAI_SCIENTIST)
                    | Some(UnitAIType::UNITAI_GENERAL)
                    | Some(UnitAIType::UNITAI_MERCHANT)
                    | Some(UnitAIType::UNITAI_ENGINEER) => {}

                    Some(UnitAIType::UNITAI_SPY) => {
                        military_value += if war_plan { 100 } else { 50 };
                    }

                    Some(UnitAIType::UNITAI_ICBM) => {
                        military_value += if war_plan { 200 } else { 100 };
                    }

                    Some(UnitAIType::UNITAI_WORKER_SEA) => {
                        // note, workboat improvements are already counted in the improvement section above
                    }

                    Some(UnitAIType::UNITAI_ATTACK_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_ATTACK_SEA) {
                            military_value += if war_plan { 200 } else { 100 };
                        }
                        naval_value += 100;
                    }

                    Some(UnitAIType::UNITAI_RESERVE_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_RESERVE_SEA) {
                            military_value += if war_plan { 100 } else { 50 };
                        }
                        naval_value += 100;
                    }

                    Some(UnitAIType::UNITAI_ESCORT_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_ESCORT_SEA) {
                            military_value += if war_plan { 100 } else { 50 };
                        }
                        naval_value += 100;
                    }

                    Some(UnitAIType::UNITAI_EXPLORE_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_EXPLORE_SEA) {
                            unit_value += if capital_alone { 1800 } else { 600 };
                        }
                    }

                    Some(UnitAIType::UNITAI_ASSAULT_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_ASSAULT_SEA) {
                            military_value += if war_plan || capital_alone { 400 } else { 200 };
                        }
                        naval_value += 200;
                    }

                    Some(UnitAIType::UNITAI_SETTLER_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_SETTLER_SEA) {
                            unit_value += if war_plan || capital_alone { 100 } else { 200 };
                        }
                        naval_value += 200;
                    }

                    Some(UnitAIType::UNITAI_MISSIONARY_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_MISSIONARY_SEA) {
                            unit_value += 100;
                        }
                    }

                    Some(UnitAIType::UNITAI_SPY_SEA) => {
                        if coastal_cities > 0 {
                            military_value += 100;
                        }
                    }

                    Some(UnitAIType::UNITAI_CARRIER_SEA) => {
                        if coastal_cities > 0 {
                            military_value += if war_plan { 100 } else { 50 };
                        }
                    }

                    Some(UnitAIType::UNITAI_MISSILE_CARRIER_SEA) => {
                        if coastal_cities > 0 {
                            military_value += if war_plan { 100 } else { 50 };
                        }
                    }

                    Some(UnitAIType::UNITAI_PIRATE_SEA) => {
                        if coastal_and_few(UnitAIType::UNITAI_PIRATE_SEA) {
                            military_value += 100;
                        }
                        naval_value += 100;
                    }

                    Some(UnitAIType::UNITAI_ATTACK_AIR) => {
                        military_value += if war_plan { 1200 } else { 800 };
                    }

                    Some(UnitAIType::UNITAI_DEFENSE_AIR) => {
                        military_value += if war_plan { 1200 } else { 800 };
                    }

                    Some(UnitAIType::UNITAI_CARRIER_AIR) => {
                        if coastal_cities > 0 {
                            military_value += if war_plan { 200 } else { 100 };
                        }
                        naval_value += 400;
                    }

                    Some(UnitAIType::UNITAI_MISSILE_AIR) => {
                        military_value += if war_plan { 200 } else { 100 };
                    }

                    _ => {}
                }

                //FfH: Modified by Kael 06/06/2008 (no halving without contact / capital alone)

                if ku.unit_ai_type[UnitAIType::UNITAI_ASSAULT_SEA] {
                    let mut assault_value = 0;
                    let mut existing_unit: Option<UnitType> = None;
                    if best_unit_ai_value_no_city(self, g, p, UnitAIType::UNITAI_ASSAULT_SEA, &mut existing_unit) == 0 {
                        assault_value += 1000;
                    } else if let Some(existing_unit) = existing_unit {
                        assault_value += 100
                            * 0.max(self.player_ai_unit_impassable_count(g, p, loop_unit) - self.player_ai_unit_impassable_count(g, p, existing_unit));
                        if assault_value > 0 {
                            assault_value += 200;
                        }
                        let new_capacity = ku.moves * ku.cargo_space;
                        let old_capacity = d.units[existing_unit].moves * d.units[existing_unit].cargo_space;
                        //modified to prevent Division by zero Sephi
                        assault_value += (800 * (new_capacity - old_capacity)) / (old_capacity + 1);
                    }

                    if assault_value > 0 {
                        if any_primary_assault() {
                            naval_value += assault_value;
                        } else {
                            naval_value += assault_value / 6;
                        }
                    }
                }

                if naval_value > 0 {
                    if let Some(cap) = capital_city {
                        let cap_area = g.city_area(cap);
                        naval_value *= 2 * (num_cities - g.map.areas[cap_area].cities_per_player[p]);
                        naval_value /= num_cities;

                        unit_value += naval_value;
                    }
                }

                unit_value += military_value;

                // PORT ADAPT: a unit without a default AI would index -1 in C++ (skipped here).
                if let Some(default_ai) = ku.default_unit_ai_type {
                    if self.player_ai_total_unit_ais(g, p, default_ai) == 0 {
                        // do not give bonus to seagoing units if they are worthless
                        if unit_value > 0 {
                            unit_value *= 2;
                        }

                        if default_ai == UnitAIType::UNITAI_EXPLORE {
                            if let Some(cap) = capital_city {
                                //FfH: Modified by Kael 06/06/2008 (* 400 -> * 10)
                                unit_value += self.player_ai_needed_explorers(g, p, g.city_area(cap)) * 10;
                            }
                        }

                        if default_ai == UnitAIType::UNITAI_EXPLORE_SEA {
                            unit_value += 400;
                            unit_value += (g.count_civ_teams_alive() - has_met_count) * 200;
                        }
                    }
                }

                //Early Expansion by sea
                if ku.unit_ai_type[UnitAIType::UNITAI_SETTLER_SEA] {
                    if self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_SETTLER_SEA) == 0 {
                        if let Some(cap) = capital_city {
                            if let Some(water_area) = g.city_water_area(cap) {
                                let mut best_area_value = 0;
                                let mut best_other_value = 0;
                                let cap_area_id = g.city_area(cap).0 as i32;
                                self.player_ai_get_num_area_city_sites(g, p, cap_area_id, &mut best_area_value);
                                self.player_ai_get_num_adjacent_area_city_sites(g, p, water_area.0 as i32, cap_area_id, &mut best_other_value);

                                if best_area_value == 0 {
                                    unit_value += 2000;
                                } else if best_area_value < best_other_value {
                                    unit_value += 1000;
                                } else if best_other_value > 0 {
                                    unit_value += 500;
                                }
                            }
                        }
                    }
                }

                if path_length <= 1 {
                    if total_population > 5 {
                        if is_world_unit_class(d, class) {
                            if !g.is_unit_class_maxed_out(class, 0) {
                                enables_unit_wonder = true;
                            }
                        }
                    }
                }

                //FfH: Modified by Kael 06/06/2008 (iUnitValue -> iUnitValue * 3)
                value += unit_value * 3;
            }

            if enables_unit_wonder {
                let wonder_random =
                    if async_ { self.rand(400, "AI Research Wonder Unit ASYNC") } else { self.rand(400, "AI Research Wonder Unit") };
                value += wonder_random + if capital_alone { 200 } else { 0 };
                // (iRandomMax / iRandomFactor are debug only)
            }

            let mut best_land_building_value = 0;
            let mut enables_wonder = false;
            let mut existing_culture_building_count = 0;
            let mut is_culture_building = false;
            for bclass in d.building_classes.keys() {
                let Some(loop_building) = d.civ_building(civ, bclass) else {
                    continue;
                };
                let kb = &d.buildings[loop_building];

                if is_tech_required_for_building(d, tech, loop_building) {
                    let mut building_value = 0;

                    if kb.special_building_type.is_some() {
                        building_value += if capital_alone { 100 } else { 25 };
                    } else {
                        building_value += if capital_alone { 200 } else { 50 };
                    }

                    //the granary effect is SO powerful it deserves special code
                    if kb.food_kept > 0 {
                        building_value += 15 * kb.food_kept;
                    }

                    // C++ quirk: for a negative maintenance modifier C++ averages the
                    // maintenance of the last (up to five) cities into iTempValue and never
                    // uses it; nothing to port.

                    building_value += 100;

                    if d.building_classes[bclass].default_building_index != Some(loop_building) {
                        //UB
                        building_value += 600;
                    }

                    if kb.commerce_change[culture] > 0 {
                        is_culture_building = true;
                    }

                    if strat_culture2 {
                        let multiplier = if is_limited_wonder_class(d, bclass) { 1 } else { 3 };
                        building_value += (150 * kb.commerce_change[culture]) * multiplier;
                        building_value += kb.commerce_modifier[culture] * 4 * multiplier;
                    }

                    if financial_trouble {
                        building_value += kb.maintenance_modifier * 15;
                        building_value += kb.yield_modifier[commerce_y] * 8;
                        building_value += kb.commerce_modifier[CommerceType::COMMERCE_GOLD.idx()] * 15;
                    }

                    // if this is a religious building, its not as useful
                    if let Some(religion) = kb.religion_type {
                        // reduce by a factor based on how many cities we have with that relgion
                        if team_city_count > 0 {
                            let cities_with_religion = g.team_get_has_religion_count(t, religion);

                            building_value *= 4 + cities_with_religion;
                            building_value /= 4 + team_city_count;
                        }

                        // only count things like temples once, not 7 times
                        building_value /= (d.religions.len() as i32).max(1);
                    }

                    // if we're close to pop domination, we love medicine!
                    if percent_of_domination >= 90 && kb.health > 0 {
                        building_value += kb.health * 150;
                    }

                    if kb.prereq_and_tech == Some(tech) {
                        if path_length <= 1 {
                            if total_population > 5 {
                                if is_world_wonder_class(d, bclass) {
                                    if !g.is_building_class_maxed_out(bclass, 0) {
                                        enables_wonder = true;

                                        if strat_culture1 {
                                            value += 400;
                                        }

                                        if capital_alone {
                                            building_value += 400;
                                        }

                                        //FfH: Added by Kael 07/15/2008
                                        if Some(loop_building) == leader.favorite_wonder {
                                            value += 600;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if building_value > best_land_building_value {
                        best_land_building_value = building_value;
                    }

                    // if water building, weight by coastal cities
                    if kb.water {
                        building_value *= coastal_cities;
                        building_value /= (city_count / 2).max(1);
                    }
                    // if land building, is it the best?
                    else if building_value > best_land_building_value {
                        best_land_building_value = building_value;
                    }

                    //FfH: Added by Kael 6/6/2008
                    building_value += kb.ai_weight * 5;

                    value += building_value;
                } else if g.player_can_construct(p, loop_building, false, false, false) {
                    if !is_limited_wonder_class(d, bclass) {
                        if kb.commerce_change[culture] > 0 {
                            existing_culture_building_count += 1;
                        }
                    }
                }
            }

            if is_culture_building && existing_culture_building_count < 2 {
                if g.players[p].free_city_commerce[culture] == 0 {
                    if num_cities > 1 {
                        value += 150 * 1.max(3 - 2 * existing_culture_building_count) * (num_cities - 1);
                    }
                }
            }
            // if tech gives at least one building (so we can count each individual building less)
            if best_land_building_value > 0 {
                value += best_land_building_value;
            }

            // if it gives at least one wonder
            if enables_wonder {
                let wonder_random =
                    if async_ { self.rand(800, "AI Research Wonder Building ASYNC") } else { self.rand(800, "AI Research Wonder Building") };
                value += (500 + wonder_random) / (if advanced_start { 5 } else { 1 });
            }

            let mut enables_project_wonder = false;
            for project in d.projects.keys() {
                if d.projects[project].tech_prereq == Some(tech) {
                    value += 1000;

                    if path_length <= 1 {
                        if total_population > 5 {
                            if is_world_project(d, project) {
                                if !g.is_project_maxed_out(project, 0) {
                                    enables_project_wonder = true;

                                    if capital_alone {
                                        value += 100;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if enables_project_wonder {
                let wonder_random =
                    if async_ { self.rand(200, "AI Research Wonder Project ASYNC") } else { self.rand(200, "AI Research Wonder Project") };
                value += wonder_random;
            }

            let mut is_good_process = false;

            for process in d.processes.keys() {
                if d.processes[process].tech_prereq == Some(tech) {
                    value += 100;

                    for k in 0..CommerceType::NUM {
                        let mut temp_value = d.processes[process].production_to_commerce_modifier[k] * 4;

                        temp_value *= commerce_weight[k];
                        temp_value /= 100;

                        if k == CommerceType::COMMERCE_GOLD.idx() || k == CommerceType::COMMERCE_RESEARCH.idx() {
                            // C++ quirk: any process of this tech counts as good, whatever its modifiers.
                            is_good_process = true;
                        } else if k == culture && strat_culture1 {
                            temp_value *= 3;
                        }

                        value += temp_value;
                    }
                }
            }

            if is_good_process && financial_trouble {
                let mut have_good_process = false;
                for process in d.processes.keys() {
                    let pi = &d.processes[process];
                    if g.team_is_has_tech(t, pi.tech_prereq) {
                        have_good_process = (pi.production_to_commerce_modifier[CommerceType::COMMERCE_GOLD.idx()]
                            + pi.production_to_commerce_modifier[CommerceType::COMMERCE_RESEARCH.idx()])
                            > 0;
                        if have_good_process {
                            break;
                        }
                    }
                }
                if !have_good_process {
                    value += 1500;
                }
            }

            for civic in d.civics.keys() {
                if d.civics[civic].tech_prereq == Some(tech) {
                    value += 200;

                    let current = d.civics[civic].civic_option_type.and_then(|opt| g.players[p].civics[opt]);
                    if let Some(current) = current {
                        let current_civic_value = self.player_ai_civic_value(g, p, current);
                        let new_civic_value = self.player_ai_civic_value(g, p, civic);

                        if new_civic_value > current_civic_value {
                            value += 2400.min((2400 * (new_civic_value - current_civic_value)) / current_civic_value.max(1));
                        }

                        if Some(current) == leader.favorite_civic {
                            value += 600;
                        }
                    }
                }
            }

            //FfH: Modified by Kael 04/29/2009 (religion block replaced)
            if path_length <= 2 {
                if g.count_known_tech_num_teams(tech) == 0 {
                    for religion in d.religions.keys() {
                        if d.religions[religion].tech_prereq == Some(tech) {
                            if !g.religion_slot_taken[religion] {
                                value += 1200;
                            }
                            value *= 100 + leader.religion_weight_modifier[religion];
                            value /= 100;
                        }
                    }
                    //FfH: End Modify

                    if let Some(free_class) = ti.first_free_unit_class {
                        if d.civ_unit(civ, free_class).is_some() {
                            let great_people_random = if async_ {
                                self.rand(3200, "AI Research Great People ASYNC")
                            } else {
                                self.rand(3200, "AI Research Great People")
                            };
                            value += great_people_random;

                            if capital_alone {
                                value += 400;
                            }

                            value += 200;
                        }
                    }

                    let free_tech_random =
                        if async_ { self.rand(3200, "AI Research Free Tech ASYNC") } else { self.rand(3200, "AI Research Free Tech") };
                    value += ti.first_free_techs * (200 + (if capital_alone { 400 } else { 0 }) + free_tech_random);
                }
            }

            value += ti.ai_weight;

            //FfH: Added by Kael 07/27/2008
            if let Some(preferred) = ti.preferred_alignment {
                if Some(preferred) == g.players[p].alignment {
                    value += value / 4;
                }
            }
            //FfH: End Add

            if !human {
                for flavor in ti.flavor_value.keys() {
                    //FfH: Modified by Kael 10/30/2008
                    let flavor_value = self.player_ai_get_flavor_value(g, p, flavor);
                    if flavor == AI_FLAVOR_MILITARY {
                        let mut temp_value = flavor_value * ti.flavor_value[flavor] * 20;
                        if g.team_get_has_met_civ_count(t, true) == 0 {
                            temp_value /= 4;
                        }
                        if g.team_get_at_war_count(t, true) > 0 {
                            temp_value *= 2;
                        }
                        value += temp_value;
                    } else {
                        value += flavor_value * ti.flavor_value[flavor] * 20;
                    }
                    //FfH: End Modify
                }
            }

            if ti.repeat {
                value /= 10;
            }

            if ignore_cost {
                value = value.wrapping_mul(1 + g.player_get_research_turns_left(p, tech, false));
                value /= 10;
            } else if value > 0 {
                //this stops quick speed messing up.... might want to adjust by other things too...
                let adjustment = d.game_speeds[g.game_speed_type()].research_percent;

                let turns_left = g.player_get_research_turns_left_times100(p, tech, false);
                let cheap_booster = (turns_left < (2 * adjustment))
                    && (0 == if async_ { self.rand(5, "AI Choose Cheap Tech") } else { self.rand(5, "AI Choose Cheap Tech") });

                //FfH: Modified by Kael 09/24/2008 (iValue *= 10000 / (...))
                let divisor = turns_left + if cheap_booster { 1 } else { 5 * adjustment };
                value = value.wrapping_mul(10000 / divisor.max(1));
            }

            //Tech Whore
            if !g.is_option(GameOptionType::GAMEOPTION_NO_TECH_TRADING) {
                if ti.tech_trading || g.team_is_tech_trading(t) {
                    let roll = if async_ { self.rand(100, "AI Tech Whore ASYNC") } else { self.rand(100, "AI Tech Whore") };
                    if roll < (if g.is_option(GameOptionType::GAMEOPTION_NO_TECH_BROKERING) { 20 } else { 10 }) {
                        let mut known_count = 0;
                        let mut possible_known_count = 0;

                        for team in g.civ_team_ids() {
                            if g.team_is_alive(team) {
                                if g.teams[t].has_met[team] {
                                    if g.team_is_has_tech(team, Some(tech)) {
                                        known_count += 1;
                                    }
                                }

                                possible_known_count += 1;
                            }
                        }

                        if known_count == 0 {
                            if possible_known_count > 2 {
                                let trade_modifier = 150.min(25 * (possible_known_count - 2));
                                value = value.wrapping_mul(100 + trade_modifier);
                                value /= 100;
                            }
                        }
                    }
                }
            }

            if strat_culture3 {
                let cv_value = self.player_ai_culture_victory_tech_value(g, p, tech);
                value = value.wrapping_mul(cv_value + 10);
                value /= if cv_value < 100 { 400 } else { 100 };
            }

            value = value.max(1);

            //FfH: Added by Kael 06/17/2009
            if let Some(favorite) = leader.favorite_tech {
                if favorite == tech {
                    value = value.wrapping_add(200000);
                }
            }
            if ti.water {
                if coastal_cities == 0 {
                    value /= 4;
                }
            }
            if value < 0 {
                value = 1;
            }
            //FfH: End Add

            if value > best_value {
                best_value = value;
                best_tech = Some(tech);
            }
        }

        best_tech
    }

    /// C++ `void CvPlayerAI::AI_chooseFreeTech()` (CvPlayerAI.cpp:5205-5228)
    ///
    /// Returns the tech C++ would grant (`setHasTech(eBestTech, true, getID(), true, true)`):
    /// Python `AI_chooseTech(ePlayer, bFree = true)`, else `AI_bestTech(1, true)`. The rules
    /// clear the research queue and grant the tech themselves (`ai_choose_free_tech` in
    /// ffh-core).
    pub fn player_ai_choose_free_tech(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        // clearResearchQueue(): done by the rules before asking.
        if let Some(tech) = self.py_ai_choose_tech(g, p, true) {
            return Some(tech);
        }

        self.player_ai_best_tech(g, p, 1, true, false, None, None)
    }

    /// C++ `void CvPlayerAI::AI_chooseResearch()` (CvPlayerAI.cpp:5231-5298)
    ///
    /// Returns the tech C++ would push (`pushResearch(eBestTech)`), in C++ order: `None` while
    /// `getDisableResearch() > 0`; a teammate's current research if we can research it; else
    /// the Python pick (`AI_chooseTech(ePlayer, false)`); else `AI_bestTech(3)`. The rules push
    /// the returned tech themselves (`player/economy.rs`, `ai_choose_research`).
    ///
    /// PORT ADAPT: C++ starts with `clearResearchQueue()` (so `getCurrentResearch()` is
    /// `NO_TECH` below) and pushes every researchable teammate tech (the first becomes current);
    /// only the first is returned here.
    pub fn player_ai_choose_research(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        // Skyre Mod / BETTER AI (better teching) merged Sephi
        if g.players[p].disable_research > 0 {
            return None;
        }

        // clearResearchQueue(): getCurrentResearch() is NO_TECH from here on.
        let team = g.players[p].team;
        for other in g.player_ids() {
            if g.players[other].alive {
                if other != p && g.players[other].team == team {
                    if let Some(research) = g.player_get_current_research(other) {
                        if g.player_can_research(p, research, false) {
                            return Some(research);
                        }
                    }
                }
            }
        }

        if let Some(tech) = self.py_ai_choose_tech(g, p, false) {
            return Some(tech);
        }

        //FfH: Added by Kael 06/17/2009 (AI_bestTech(3) for everyone)
        self.player_ai_best_tech(g, p, 3, false, false, None, None)
    }

    /// C++ `bool CvPlayerAI::AI_isWillingToTalk(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5334-5382)
    pub fn player_ai_is_willing_to_talk(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        let t = g.players[p].team;
        let their = g.players[player].team;

        if their == t || g.teams[their].vassal[t] || g.teams[t].vassal[their] {
            return true;
        }

        if g.team_is_human(t) {
            return false;
        }

        if g.teams[t].at_war[their] {
            let leader = lh(g, p);
            let mut refuse_duration = leader.refuse_to_talk_war_threshold * (if g.team_ai_is_chosen_war(t, their) { 2 } else { 1 });

            let our_success = 1 + g.teams[t].ai_war_success[their];
            let their_success = 1 + g.teams[their].ai_war_success[t];
            if their_success > our_success * 2 {
                refuse_duration *= 20 + ((80 * our_success * 2) / their_success);
                refuse_duration /= 100;
            }

            if g.teams[t].ai_at_war_counter[their] < refuse_duration {
                return false;
            }

            if g.team_is_a_vassal(t) {
                return false;
            }
        } else if g.players[p].ai_memory_count[player][MemoryType::MEMORY_STOPPED_TRADING_RECENT] > 0 {
            return false;
        }

        true
    }

    /// C++ `bool CvPlayerAI::AI_demandRebukedSneak(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5387-5404)
    pub fn player_ai_demand_rebuked_sneak(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        let t = g.players[p].team;
        let their = g.players[player].team;

        if self.rand(100, "AI Demand Rebuked") < lh(g, p).demand_rebuked_sneak_prob {
            if g.team_get_power(t, true) > g.team_get_defensive_power(their) {
                return true;
            }
        }

        false
    }

    /// C++ `bool CvPlayerAI::AI_demandRebukedWar(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5409-5430)
    pub fn player_ai_demand_rebuked_war(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        let t = g.players[p].team;
        let their = g.players[player].team;

        // needs to be async because it only happens on the computer of the player who is in diplomacy...
        if self.rand(100, "AI Demand Rebuked ASYNC") < lh(g, p).demand_rebuked_war_prob {
            if g.team_get_power(t, true) > g.team_get_defensive_power(their) {
                if self.team_ai_is_ally_land_target(g, t, their) {
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvPlayerAI::AI_hasTradedWithTeam(TeamTypes eTeam) const` (CvPlayerAI.cpp:5434-5453)
    pub fn player_ai_has_traded_with_team(&self, g: &Game, p: PlayerId, team: TeamId) -> bool {
        for other in g.player_ids() {
            if g.players[other].alive {
                if g.players[other].team == team {
                    if (g.players[p].ai_peacetime_grant_value[other] + g.players[p].ai_peacetime_trade_value[other]) > 0 {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// C++ `AttitudeTypes CvPlayerAI::AI_getAttitudeFromValue(int iAttitudeVal)` (CvPlayerAI.cpp:5456-5478)
    pub fn player_ai_get_attitude_from_value(&self, g: &Game, attitude_val: i32) -> AttitudeType {
        if attitude_val >= 10 {
            AttitudeType::ATTITUDE_FRIENDLY
        } else if attitude_val >= 3 {
            AttitudeType::ATTITUDE_PLEASED
        } else if attitude_val <= -10 {
            AttitudeType::ATTITUDE_FURIOUS
        } else if attitude_val <= -3 {
            AttitudeType::ATTITUDE_ANNOYED
        } else {
            AttitudeType::ATTITUDE_CAUTIOUS
        }
    }

    /// C++ `AttitudeTypes CvPlayerAI::AI_getAttitude(PlayerTypes ePlayer, bool bForced = true) const` (CvPlayerAI.cpp:5480-5487)
    ///
    /// The full C++ function (forced overrides included); the controller's `attitude` calls it
    /// with `forced = false` (ffh-core's `player_ai_get_attitude` applies the overrides first).
    pub fn player_ai_get_attitude(&self, g: &Game, p: PlayerId, player: PlayerId, forced: bool) -> AttitudeType {
        let val = self.player_ai_get_attitude_val(g, p, player, forced);
        self.player_ai_get_attitude_from_value(g, val)
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeVal(PlayerTypes ePlayer, bool bForced = true) const` (CvPlayerAI.cpp:5490-5588)
    pub fn player_ai_get_attitude_val(&self, g: &Game, p: PlayerId, player: PlayerId, forced: bool) -> i32 {
        let d = g.d;
        let t = g.players[p].team;
        let their = g.players[player].team;

        if forced {
            if t == their || (g.teams[t].vassal[their] && !g.teams[t].capitulated) {
                return 100;
            }

            if g.player_is_barbarian(p) || g.player_is_barbarian(player) {
                return -100;
            }
        }

        let leader = lh(g, p);

        let mut attitude = leader.base_attitude;

        attitude += g.player_get_handicap_type(player).map_or(0, |h| d.handicaps[h].attitude_change);

        if !g.players[player].human {
            let our_peace_weight = peace_weight(self, g, p);
            let their_peace_weight = peace_weight(self, g, player);
            attitude += 4 - (our_peace_weight - their_peace_weight).abs();
            attitude += leader.warmonger_respect.min(lh(g, player).warmonger_respect);
        }

        attitude -= 0.max(g.teams[their].num_members - g.teams[t].num_members);

        let rank_difference = g.player_rank[p] - g.player_rank[player];
        let ever_alive = g.count_civ_players_ever_alive();

        if rank_difference > 0 {
            attitude += (leader.worse_rank_difference_attitude_change * rank_difference) / (ever_alive + 1);
        } else {
            attitude += (leader.better_rank_difference_attitude_change * -rank_difference) / (ever_alive + 1);
        }

        if (g.player_rank[p] >= (ever_alive / 2)) && (g.player_rank[player] >= (ever_alive / 2)) {
            attitude += 1;
        }

        if g.teams[their].ai_war_success[t] > g.teams[t].ai_war_success[their] {
            attitude += leader.lost_war_attitude_change;
        }

        attitude += self.player_ai_get_close_borders_attitude(g, p, player);
        attitude += self.player_ai_get_war_attitude(g, p, player);
        attitude += self.player_ai_get_peace_attitude(g, p, player);
        attitude += self.player_ai_get_same_religion_attitude(g, p, player);
        attitude += self.player_ai_get_different_religion_attitude(g, p, player);
        attitude += self.player_ai_get_bonus_trade_attitude(g, p, player);
        attitude += self.player_ai_get_open_borders_attitude(g, p, player);
        attitude += self.player_ai_get_defensive_pact_attitude(g, p, player);
        attitude += self.player_ai_get_rival_defensive_pact_attitude(g, p, player);
        attitude += self.player_ai_get_rival_vassal_attitude(g, p, player);
        attitude += self.player_ai_get_share_war_attitude(g, p, player);
        attitude += self.player_ai_get_favorite_civic_attitude(g, p, player);
        attitude += self.player_ai_get_trade_attitude(g, p, player);
        attitude += self.player_ai_get_rival_trade_attitude(g, p, player);

        //FfH: Added by Kael 08/15/2007
        attitude += self.player_ai_get_alignment_attitude(g, p, player);
        attitude += self.player_ai_get_bad_bonus_attitude(g, p, player);
        attitude += self.player_ai_get_favorite_wonder_attitude(g, p, player);
        attitude += self.player_ai_get_gender_attitude(g, p, player);
        attitude += self.player_ai_get_trust_attitude(g, p, player);
        attitude += self.player_ai_get_civic_share_attitude(g, p, player);
        //FfH: End Add

        for memory in d.memories.keys() {
            attitude += self.player_ai_get_memory_attitude(g, p, player, memory);
        }

        attitude += self.player_ai_get_colony_attitude(g, p, player);
        attitude += g.players[p].ai_attitude_extra[player];

        range(attitude, -100, 100)
    }

    /// C++ `int CvPlayerAI::AI_calculateStolenCityRadiusPlots(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5591-5617)
    pub fn player_ai_calculate_stolen_city_radius_plots(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let mut count = 0;

        for i in 0..g.map.plots.len() {
            let plot = PlotIdx(i as u32);

            if g.map.plots[plot].owner == Some(player) {
                if g.plot_is_player_city_radius(plot, p) {
                    count += 1;
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_getCloseBordersAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5620-5642)
    ///
    /// Cached in `close_borders_attitude_cache` (C++ `m_aiCloseBordersAttitudeCache`, `MAX_INT`
    /// = invalid; `AI_reset` sets 0, `AI_doTurnPre` invalidates).
    pub fn player_ai_get_close_borders_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let n = g.players.len();
        let cached = {
            let mut s = self.s();
            s.ensure_players(n);
            s.player(p).close_borders_attitude_cache[player.idx()]
        };

        if cached == MAX_INT {
            let t = g.players[p].team;
            let their = g.players[player].team;

            if t == their || g.teams[t].vassal[their] || g.teams[their].vassal[t] {
                return 0;
            }

            let mut percent = 60.min(self.player_ai_calculate_stolen_city_radius_plots(g, p, player) * 3);

            if self.team_ai_is_land_target(g, t, their) {
                percent += 40;
            }

            let value = (lh(g, p).close_borders_attitude_change * percent) / 100;
            self.s().player(p).close_borders_attitude_cache[player.idx()] = value;
            return value;
        }

        cached
    }

    /// C++ `int CvPlayerAI::AI_getWarAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5645-5664)
    pub fn player_ai_get_war_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);

        let mut attitude = 0;

        if g.teams[t].at_war[their] {
            attitude -= 3;
        }

        if leader.at_war_attitude_divisor != 0 {
            let change = g.teams[t].ai_at_war_counter[their] / leader.at_war_attitude_divisor;
            let limit = leader.at_war_attitude_change_limit.abs();
            attitude += range(change, -limit, limit);
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getPeaceAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5667-5678)
    pub fn player_ai_get_peace_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);

        if leader.at_peace_attitude_divisor != 0 {
            let change = g.teams[t].ai_at_peace_counter[their] / leader.at_peace_attitude_divisor;
            let limit = leader.at_peace_attitude_change_limit.abs();
            return range(change, -limit, limit);
        }

        0
    }

    /// C++ `int CvPlayerAI::AI_getSameReligionAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5681-5705)
    pub fn player_ai_get_same_religion_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let leader = lh(g, p);
        let mut attitude = 0;

        let our_religion = g.player_get_state_religion(p);
        if let Some(religion) = our_religion {
            if our_religion == g.player_get_state_religion(player) {
                attitude += leader.same_religion_attitude_change;

                if g.player_has_holy_city(p, religion) {
                    attitude += 1;
                }

                if leader.same_religion_attitude_divisor != 0 {
                    let counter = counter_of(self, g, p, player, |s| &s.same_religion_counter);
                    let change = counter / leader.same_religion_attitude_divisor;
                    let limit = leader.same_religion_attitude_change_limit.abs();
                    attitude += range(change, -limit, limit);
                }
            }
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getDifferentReligionAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5708-5740)
    pub fn player_ai_get_different_religion_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let leader = lh(g, p);
        let mut attitude = 0;

        let our_religion = g.player_get_state_religion(p);
        let their_religion = g.player_get_state_religion(player);
        if let (Some(ours), Some(theirs)) = (our_religion, their_religion) {
            if ours != theirs {
                attitude += leader.different_religion_attitude_change;

                if g.player_has_holy_city(p, ours) {
                    attitude -= 1;
                }

                if leader.different_religion_attitude_divisor != 0 {
                    let counter = counter_of(self, g, p, player, |s| &s.different_religion_counter);
                    let change = counter / leader.different_religion_attitude_divisor;
                    let limit = leader.different_religion_attitude_change_limit.abs();
                    attitude += range(change, -limit, limit);
                }

                //FfH: Added by Kael 11/05/2007
                if !g.player_can_see_religion(p, theirs, None) || !g.player_can_see_religion(player, ours, None) {
                    attitude = 0;
                }
                //FfH: End Add
            }
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getBonusTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5743-5757)
    pub fn player_ai_get_bonus_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);

        if !g.teams[t].at_war[their] {
            if leader.bonus_trade_attitude_divisor != 0 {
                let counter = counter_of(self, g, p, player, |s| &s.bonus_trade_counter);
                let change = counter / leader.bonus_trade_attitude_divisor;
                let limit = leader.bonus_trade_attitude_change_limit.abs();
                return range(change, -limit, limit);
            }
        }

        0
    }

    /// C++ `int CvPlayerAI::AI_getOpenBordersAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5760-5774)
    pub fn player_ai_get_open_borders_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);

        if !g.teams[t].at_war[their] {
            if leader.open_borders_attitude_divisor != 0 {
                let change = g.teams[t].ai_open_borders_counter[their] / leader.open_borders_attitude_divisor;
                let limit = leader.open_borders_attitude_change_limit.abs();
                return range(change, -limit, limit);
            }
        }

        0
    }

    /// C++ `int CvPlayerAI::AI_getDefensivePactAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5777-5796)
    pub fn player_ai_get_defensive_pact_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);

        if t != their && (g.teams[t].vassal[their] || g.teams[their].vassal[t]) {
            return leader.defensive_pact_attitude_change_limit;
        }

        if !g.teams[t].at_war[their] {
            if leader.defensive_pact_attitude_divisor != 0 {
                let change = g.teams[t].ai_defensive_pact_counter[their] / leader.defensive_pact_attitude_divisor;
                let limit = leader.defensive_pact_attitude_change_limit.abs();
                return range(change, -limit, limit);
            }
        }

        0
    }

    /// C++ `int CvPlayerAI::AI_getRivalDefensivePactAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5799-5814)
    pub fn player_ai_get_rival_defensive_pact_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let mut attitude = 0;

        if t == their || g.teams[t].vassal[their] || g.teams[their].vassal[t] {
            return attitude;
        }

        if !g.teams[t].defensive_pact[their] {
            // C++ quirk: getDefensivePactCount is asked of their team with their own team.
            attitude -= (4 * g.team_get_defensive_pact_count(their, Some(their))) / (g.count_civ_teams_alive() - 2).max(1);
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getRivalVassalAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5817-5832)
    pub fn player_ai_get_rival_vassal_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let mut attitude = 0;

        if t == their || g.teams[t].vassal[their] || g.teams[their].vassal[t] {
            return attitude;
        }

        if g.team_get_vassal_count(their, Some(t)) > 0 {
            attitude -= (6 * g.team_get_power(their, true)) / g.count_total_civ_power().max(1);
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getShareWarAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5835-5857)
    pub fn player_ai_get_share_war_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let t = g.players[p].team;
        let their = g.players[player].team;
        let leader = lh(g, p);
        let mut attitude = 0;

        if !g.teams[t].at_war[their] {
            if g.team_ai_share_war(t, their) {
                attitude += leader.share_war_attitude_change;
            }

            if leader.share_war_attitude_divisor != 0 {
                let change = g.teams[t].ai_share_war_counter[their] / leader.share_war_attitude_divisor;
                let limit = leader.share_war_attitude_change_limit.abs();
                attitude += range(change, -limit, limit);
            }
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteCivicAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5860-5883)
    pub fn player_ai_get_favorite_civic_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let leader = lh(g, p);
        let mut attitude = 0;

        if let Some(favorite) = leader.favorite_civic {
            if g.player_is_civic(p, favorite) && g.player_is_civic(player, favorite) {
                attitude += leader.favorite_civic_attitude_change;

                if leader.favorite_civic_attitude_divisor != 0 {
                    let counter = counter_of(self, g, p, player, |s| &s.favorite_civic_counter);
                    let change = counter / leader.favorite_civic_attitude_divisor;
                    let limit = leader.favorite_civic_attitude_change_limit.abs();
                    attitude += range(change, -limit, limit);
                }
            }
        }

        attitude
    }

    /// C++ `int CvPlayerAI::AI_getTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5886-5890)
    pub fn player_ai_get_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        // XXX human only?
        let t = g.players[p].team;
        let their = g.players[player].team;
        let grant = g.players[p].ai_peacetime_grant_value[player];
        let trade = g.players[p].ai_peacetime_trade_value[player];
        let their_trade = g.players[player].ai_peacetime_trade_value[p];
        range((grant + 0.max(trade - their_trade)) / ((g.teams[t].ai_has_met_counter[their] + 1) * 5), 0, 4)
    }

    /// C++ `int CvPlayerAI::AI_getRivalTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5893-5897)
    pub fn player_ai_get_rival_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        // XXX human only?
        let t = g.players[p].team;
        let their = g.players[player].team;
        -(range(
            (g.teams[t].ai_enemy_peacetime_grant_value[their] + (g.teams[t].ai_enemy_peacetime_trade_value[their] / 3))
                / ((g.teams[t].ai_has_met_counter[their] + 1) * 10),
            0,
            4,
        ))
    }

    /// C++ `int CvPlayerAI::AI_getMemoryAttitude(PlayerTypes ePlayer, MemoryTypes eMemory) const` (CvPlayerAI.cpp:5900-5903)
    pub fn player_ai_get_memory_attitude(&self, g: &Game, p: PlayerId, player: PlayerId, memory: MemoryType) -> i32 {
        (g.players[p].ai_memory_count[player][memory] * lh(g, p).memory_attitude_percent[memory]) / 100
    }

    /// C++ `int CvPlayerAI::AI_getColonyAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5905-5915)
    pub fn player_ai_get_colony_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let mut attitude = 0;

        if g.players[p].parent == Some(player) {
            attitude += lh(g, p).freedom_appreciation;
        }

        attitude
    }

    /// C++ `PlayerVoteTypes CvPlayerAI::AI_diploVote(const VoteSelectionSubData& kVoteData, VoteSourceTypes eVoteSource, bool bPropose)` (CvPlayerAI.cpp:5919-6803)
    pub fn player_ai_diplo_vote(&self, g: &Game, p: PlayerId, vote_data: &VoteSelectionSubData, vote_source: VoteSourceType, propose: bool) -> PlayerVote {
        let d = g.d;
        let t = g.players[p].team;
        let leader = lh(g, p);

        let vote = vote_data.vote;
        let vi = &d.votes[vote];

        if g.is_team_vote(vote) {
            if g.is_team_vote_eligible(t, vote_source) {
                return PlayerVote::for_team(t);
            }

            let mut best_value = if vi.victory { 7 } else { 0 };

            let mut best_team = PlayerVote::PLAYER_VOTE_ABSTAIN;

            for team in g.team_ids() {
                if g.team_is_alive(team) {
                    if g.is_team_vote_eligible(team, vote_source) {
                        if g.teams[t].vassal[team] {
                            return PlayerVote::for_team(team);
                        }

                        let value = g.team_ai_get_attitude_val(t, team, true);

                        if value > best_value {
                            best_value = value;
                            best_team = PlayerVote::for_team(team);
                        }
                    }
                }
            }

            return best_team;
        }

        let secretary_general = g.secretary_general(vote_source);

        // BETTER_BTS_AI_MOD 12/30/08 jdog5000: remove blanket auto approval for friendly secretary
        let mut friendly_to_secretary = false;
        if !propose {
            if let Some(sg) = secretary_general {
                if sg == t {
                    return PlayerVote::PLAYER_VOTE_YES;
                } else {
                    friendly_to_secretary = g.team_ai_get_attitude(t, sg, true) == AttitudeType::ATTITUDE_FRIENDLY;
                }
            }
        }

        let aggressive_ai = g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI);

        let mut defy = false;

        let mut valid = true;

        if valid {
            for civic in d.civics.keys() {
                if vi.force_civic[civic] {
                    if !g.player_is_civic(p, civic) {
                        // PORT ADAPT: a civic without an option would index -1 in C++.
                        let Some(civic_option) = d.civics[civic].civic_option_type else {
                            continue;
                        };
                        let best_civic = self.player_ai_best_civic(g, p, civic_option);

                        if let Some(best_civic) = best_civic {
                            if best_civic != civic {
                                let best_civic_value = self.player_ai_civic_value(g, p, best_civic);
                                let mut new_civic_value = self.player_ai_civic_value(g, p, civic);

                                // Increase threshold of voting for friend's proposal
                                if friendly_to_secretary {
                                    new_civic_value *= 6;
                                    new_civic_value /= 5;
                                }

                                if best_civic_value > ((new_civic_value * 120) / 100) {
                                    valid = false;

                                    // Increase odds of defiance, particularly on AggressiveAI
                                    let roll = self.rand(if aggressive_ai { 60 } else { 80 }, "AI Erratic Defiance (Force Civic)");
                                    if best_civic_value > ((new_civic_value * (140 + roll)) / 100) {
                                        defy = true;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if valid {
            if vi.trade_routes > 0 {
                if friendly_to_secretary {
                    return PlayerVote::PLAYER_VOTE_YES;
                }

                if g.players[p].num_cities() > ((g.num_cities * 2) / (g.count_civ_players_alive() + 1)) {
                    valid = false;
                }
            }
        }

        if valid {
            if vi.no_nukes {
                let mut vote_ban_threshold = 0;
                vote_ban_threshold += g.teams[t].nuke_interception / 3;
                vote_ban_threshold += leader.build_unit_prob;
                vote_ban_threshold *= 1.max(leader.warmonger_respect);
                if aggressive_ai {
                    vote_ban_threshold *= 2;
                }

                let mut any_has_sdi = false;
                for team in g.team_ids() {
                    if g.team_is_alive(team) && team != t {
                        if g.teams[team].nuke_interception > 0 {
                            any_has_sdi = true;
                            break;
                        }
                    }
                }

                if !any_has_sdi && g.teams[t].nuke_interception > 0 && g.team_get_num_nuke_units(t) > 0 {
                    vote_ban_threshold *= 2;
                }

                if friendly_to_secretary {
                    vote_ban_threshold *= 2;
                    vote_ban_threshold /= 3;
                }

                valid = self.rand(100, "AI nuke ban vote") > vote_ban_threshold;

                if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_OWABWNW) {
                    valid = false;
                } else if (g.team_get_num_nuke_units(t) / g.teams[t].num_members.max(1))
                    < (g.count_total_nuke_units() / g.count_civ_players_alive().max(1))
                {
                    valid = false;
                }
                if !valid && self.player_ai_get_num_train_ai_units(g, p, UnitAIType::UNITAI_ICBM) > 0 {
                    let n = if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_OWABWNW) { 2 } else { 3 };
                    if self.rand(n, "AI Erratic Defiance (No Nukes)") == 0 {
                        defy = true;
                    }
                }
            }
        }

        if valid {
            if vi.free_trade {
                if friendly_to_secretary {
                    return PlayerVote::PLAYER_VOTE_YES;
                }

                let mut open_count = 0;
                let mut closed_count = 0;

                for team in g.team_ids() {
                    if g.team_is_alive(team) {
                        if team != t {
                            if g.teams[t].open_borders[team] {
                                open_count += g.teams[team].num_cities;
                            } else {
                                closed_count += g.teams[team].num_cities;
                            }
                        }
                    }
                }

                if open_count >= (g.players[p].num_cities() * g.players[p].trade_routes) {
                    valid = false;
                }

                if closed_count == 0 {
                    valid = false;
                }
            }
        }

        if valid {
            if vi.open_borders {
                if friendly_to_secretary {
                    return PlayerVote::PLAYER_VOTE_YES;
                }

                valid = true;

                for team in g.civ_team_ids() {
                    if team != t {
                        if g.team_is_voting_member(team, vote_source) {
                            if self.team_ai_open_borders_trade(g, t, team).is_some() {
                                valid = false;
                                break;
                            }
                        }
                    }
                }
            } else if vi.defensive_pact {
                if friendly_to_secretary {
                    return PlayerVote::PLAYER_VOTE_YES;
                }

                valid = true;

                for team in g.civ_team_ids() {
                    if team != t {
                        if g.team_is_voting_member(team, vote_source) {
                            if self.team_ai_defensive_pact_trade(g, t, team).is_some() {
                                valid = false;
                                break;
                            }
                        }
                    }
                }
            } else if vi.force_peace {
                // PORT ADAPT: C++ asserts kVoteData.ePlayer != NO_PLAYER.
                if let Some(vote_player) = vote_data.player {
                    let peace_team = g.players[vote_player].team;
                    let (v, df) = diplo_vote_force_peace(self, g, p, peace_team, secretary_general, friendly_to_secretary, propose);
                    valid = v;
                    if df {
                        defy = true;
                    }
                } else {
                    valid = false;
                }
            } else if vi.force_no_trade {
                // PORT ADAPT: C++ asserts kVoteData.ePlayer != NO_PLAYER.
                if let Some(vote_player) = vote_data.player {
                    let embargo_team = g.players[vote_player].team;

                    if secretary_general == Some(t) && !propose {
                        valid = true;
                    } else if embargo_team == t {
                        valid = false;
                        if !g.player_is_no_foreign_trade(p) {
                            defy = true;
                        }
                    } else if friendly_to_secretary {
                        return PlayerVote::PLAYER_VOTE_YES;
                    } else if g.player_can_stop_trading_with_team(p, embargo_team, false) {
                        valid = self.player_ai_stop_trading_trade(g, p, embargo_team, vote_player).is_none();
                        if valid {
                            valid = g.team_ai_get_attitude(t, embargo_team, true) <= AttitudeType::ATTITUDE_CAUTIOUS;
                        }
                    } else {
                        valid = g.team_ai_get_attitude(t, embargo_team, true) < AttitudeType::ATTITUDE_CAUTIOUS;
                    }
                } else {
                    valid = false;
                }
            } else if vi.force_war {
                // PORT ADAPT: C++ asserts kVoteData.ePlayer != NO_PLAYER.
                if let Some(vote_player) = vote_data.player {
                    let war_team = g.players[vote_player].team;

                    if secretary_general == Some(t) && !propose {
                        valid = true;
                    } else if war_team == t || g.teams[t].vassal[war_team] {
                        // Explicit rejection by all who will definitely be attacked
                        valid = false;
                    } else if g.teams[t].ai_war_plan[war_team].is_some() {
                        valid = true;
                    } else {
                        if !propose && g.team_is_a_vassal(t) {
                            // Vassals always deny war trade requests and thus previously always voted no
                            valid = false;

                            if g.team_get_any_war_plan_count(t, true) == 0 {
                                let sg_ok = match secretary_general {
                                    None => true,
                                    Some(sg) => {
                                        attitude_i(Some(g.team_ai_get_attitude(t, sg, true)))
                                            > attitude_i(leader.declare_war_refuse_attitude_threshold)
                                    }
                                };
                                if sg_ok {
                                    if let Some(sg) = secretary_general {
                                        if g.teams[t].vassal[sg] {
                                            valid = true;
                                        }
                                    }
                                }
                            }
                        } else {
                            // PORT ADAPT: C++ passes NO_TEAM to AI_declareWarTrade when there is
                            // no secretary general (undefined); treated as a denial.
                            valid = propose
                                || secretary_general
                                    .map_or(false, |sg| self.team_ai_declare_war_trade(g, t, war_team, sg, true).is_none());
                        }

                        if valid {
                            let no_war_odds = leader.no_war_attitude_prob[g.team_ai_get_attitude(t, war_team, true)];
                            valid = (no_war_odds < 30) || (self.rand(100, "AI War Vote Attitude Check (Force War)") > no_war_odds);
                        }
                    }
                } else {
                    valid = false;
                }
            } else if vi.assign_city {
                valid = false;

                // PORT ADAPT: C++ asserts kVoteData.ePlayer != NO_PLAYER.
                if let Some(vote_player) = vote_data.player {
                    // kPlayer.getCity(iCityId): a city of that player.
                    let city = vote_data.city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == vote_player);
                    if let Some(city) = city {
                        if let Some(other_player) = vote_data.other_player {
                            if other_player != g.cities[city].owner {
                                if (!propose && secretary_general == Some(t)) || other_player == p {
                                    valid = true;
                                } else if vote_player == p {
                                    valid = false;
                                    if self.rand(3, "AI Erratic Defiance (Assign City)") == 0 {
                                        defy = true;
                                    }
                                } else {
                                    valid = self.player_ai_get_attitude(g, p, vote_player, true)
                                        < self.player_ai_get_attitude(g, p, other_player, true);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Don't defy resolutions from friends
        if defy && !friendly_to_secretary && g.player_can_defy_resolution(p, vote_source, vote_data) {
            return PlayerVote::PLAYER_VOTE_NEVER;
        }

        //FfH: Added by Kael 05/08/2008
        if vi.gambling_ring || vi.slave_trade {
            if g.players[p].alignment == Some(AlignmentType::ALIGNMENT_NEUTRAL) {
                valid = false;
            }
        }
        if vi.no_outside_tech_trades {
            // getPlayerRank(getRankPlayer(countCivPlayersAlive() / 2)): that rank is held by an
            // alive (scored) player, ranked as in C++.
            let rank = (g.count_civ_players_alive() / 2).max(0) as usize;
            if let Some(&rank_player) = g.rank_player.get(rank) {
                if g.player_rank[p] < g.player_rank[rank_player] {
                    valid = false;
                }
            }
        }
        if vi.smuggling_ring {
            if g.player_count_num_coastal_cities(p) < 3 {
                valid = false;
            }
        }
        if let Some(no_bonus) = vi.no_bonus {
            if g.player_get_num_available_bonuses(p, no_bonus) > 0 {
                valid = false;
            }
        }
        //FfH: End Add

        if valid {
            PlayerVote::PLAYER_VOTE_YES
        } else {
            PlayerVote::PLAYER_VOTE_NO
        }
    }
}

/// The `isForcePeace()` branch of `AI_diploVote` (BETTER_BTS_AI_MOD 01/03/09): returns
/// `(bValid, bDefy)`.
fn diplo_vote_force_peace(
    cai: &Cai,
    g: &Game,
    p: PlayerId,
    peace_team: TeamId,
    secretary_general: Option<TeamId>,
    friendly_to_secretary: bool,
    propose: bool,
) -> (bool, bool) {
    let t = g.players[p].team;
    let leader = lh(g, p);
    let mut valid;
    let mut defy = false;

    let mut wars_winning = 0;
    let mut wars_losing = 0;
    let mut chosen_war = 0;

    let mut losing_big = false;
    let mut winning_big = false;
    let mut this_player_winning = false;

    let mut win_delta_threshold = 3 * g.d.defines.war_success_attacking;
    let mut loss_abs_threshold = 3.max(g.players[p].num_military_units / 40) * g.d.defines.war_success_attacking;

    let aggressive_ai = g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI);
    if aggressive_ai {
        win_delta_threshold *= 2;
        win_delta_threshold /= 3;

        loss_abs_threshold *= 4;
        loss_abs_threshold /= 3;
    }

    // Is ePeaceTeam winning wars?
    for team in g.civ_team_ids() {
        if !g.team_is_alive(team) || team == peace_team {
            continue;
        }
        if !g.teams[team].at_war[peace_team] {
            continue;
        }

        let peace_team_success = g.teams[peace_team].ai_war_success[team];
        let other_team_success = g.teams[team].ai_war_success[peace_team];

        if (peace_team_success - other_team_success) > win_delta_threshold {
            // Have to be ahead by at least a few victories to count as win
            wars_winning += 1;

            if (peace_team_success - other_team_success) > (3 * win_delta_threshold) {
                winning_big = true;
            }
        } else if other_team_success >= peace_team_success {
            if team == t {
                if (other_team_success - peace_team_success) > win_delta_threshold {
                    this_player_winning = true;
                }
            }

            if other_team_success > loss_abs_threshold {
                // Have to have non-trivial loses
                wars_losing += 1;

                if (other_team_success - peace_team_success) > (3 * loss_abs_threshold) {
                    losing_big = true;
                }
            } else if g.teams[peace_team].ai_at_war_counter[team] < 10 {
                // Not winning, just recently attacked, and in multiple wars, be pessimistic
                // Counts ties from no actual battles
                if (g.team_get_at_war_count(peace_team, true) > 1) && !g.team_ai_is_chosen_war(peace_team, team) {
                    wars_losing += 1;
                }
            }
        }

        if g.team_ai_is_chosen_war(peace_team, team) {
            chosen_war += 1;
        }
    }

    if peace_team == t {
        let mut peace_rand = leader.base_peace_weight;
        peace_rand /= if aggressive_ai { 2 } else { 1 };

        // Always true for real war-mongers, rarely true for less aggressive types
        let warmonger_roll = cai.rand(peace_rand, "AI Erratic Defiance (Force Peace)") == 0;

        if losing_big && (!warmonger_roll || propose) {
            // Non-warmongers want peace to escape loss
            valid = true;
        } else if !losing_big && (chosen_war > wars_losing) {
            // If chosen to be in most wars, keep it going
            valid = false;
        } else {
            // If losing most wars, vote for peace
            valid = wars_losing > wars_winning;
        }

        if !valid && !losing_big && winning_big {
            // Can we continue this war with defiance penalties?
            if !cai.player_ai_is_financial_trouble(g, p) {
                if warmonger_roll {
                    defy = true;
                }
            }
        }
    } else if secretary_general == Some(t) && !propose {
        valid = true;
    } else if g.teams[peace_team].at_war[t] {
        let wants_to_end_war = cai.team_ai_end_war_val(g, t, peace_team) > (3 * cai.team_ai_end_war_val(g, peace_team, t)) / 2;
        valid = wants_to_end_war;

        if valid {
            valid = winning_big || (wars_winning > wars_losing) || (g.team_get_at_war_count(t, true) > 1);
        }

        if !valid && this_player_winning && (wars_losing >= wars_winning) && !propose {
            if !g.team_is_a_vassal(t) {
                if (g.team_get_at_war_count(t, true) == 1) || losing_big {
                    // Can we continue this war with defiance penalties?
                    if !cai.player_ai_is_financial_trouble(g, p) {
                        let mut defy_rand = leader.base_peace_weight;
                        defy_rand /= if aggressive_ai { 2 } else { 1 };

                        if cai.rand(defy_rand, "AI Erratic Defiance (Force Peace)") == 0 {
                            defy = true;
                        }
                    }
                }
            }
        }

        if !valid && !defy && !propose {
            // PORT ADAPT: with no secretary general C++ reads NO_TEAM (undefined): as an empty
            // team (ATTITUDE_CAUTIOUS), and AI_makePeaceTrade(.., NO_TEAM) as a denial.
            let sg_attitude = secretary_general.map_or(AttitudeType::ATTITUDE_CAUTIOUS, |sg| g.team_ai_get_attitude(t, sg, true));
            if attitude_i(Some(sg_attitude)) > attitude_i(leader.vassal_refuse_attitude_threshold) {
                // Influence by secretary
                let peace_ok = secretary_general.map_or(false, |sg| cai.team_ai_make_peace_trade(g, t, peace_team, sg).is_none());
                if peace_ok {
                    valid = true;
                } else if let Some(sg) = secretary_general {
                    if g.teams[t].vassal[sg] {
                        valid = true;
                    }
                }
            }
        }
    } else {
        if g.teams[t].ai_war_plan[peace_team].is_some() {
            // Keep planned enemy occupied
            valid = false;
        } else if g.team_ai_share_war(t, peace_team) && !g.teams[t].vassal[peace_team] {
            // Keep ePeaceTeam at war with our common enemies
            valid = false;
        } else if wars_losing > wars_winning {
            // Feel pity for team that is losing (if like them enough to not declare war on them)
            valid = attitude_i(Some(g.team_ai_get_attitude(t, peace_team, true)))
                >= attitude_i(leader.declare_war_them_refuse_attitude_threshold);
        } else {
            // Stop a team that is winning (if don't like them enough to join them in war)
            valid = attitude_i(Some(g.team_ai_get_attitude(t, peace_team, true)))
                < attitude_i(leader.declare_war_refuse_attitude_threshold);
        }

        if !valid {
            if friendly_to_secretary && !g.teams[t].vassal[peace_team] {
                // Influence by secretary
                valid = true;
            }
        }
    }

    (valid, defy)
}

/// The player's leader (`GC.getLeaderHeadInfo(getPersonalityType())`).
/// PORT ADAPT: falls back to the leader type, then to the first leader, where C++ would index
/// NO_LEADER (an empty slot).
fn lh(g: &Game, p: PlayerId) -> &'static LeaderHeadInfo {
    let d: &'static GameData = g.d;
    let leader = g.players[p].personality_type.or_else(|| g.player_get_leader_type(p)).unwrap_or(LeaderHeadType(0));
    &d.leader_heads[leader]
}

/// `getCivilizationType()` (PORT ADAPT: the first civilization for an empty slot).
fn civ_of(g: &Game, p: PlayerId) -> CivilizationType {
    g.player_get_civilization_type(p).unwrap_or(CivilizationType(0))
}

/// An `AttitudeTypes` value as the C++ int (`NO_ATTITUDE` = -1), for comparisons with the
/// leaders' attitude thresholds.
fn attitude_i(a: Option<AttitudeType>) -> i32 {
    a.map_or(-1, |a| a.0 as i32)
}

/// `AI_getPeaceWeight()` of `p` (state of this controller; 0 before `AI_init`).
fn peace_weight(cai: &Cai, g: &Game, p: PlayerId) -> i32 {
    let n = g.players.len();
    let mut s = cai.s();
    s.ensure_players(n);
    s.player(p).peace_weight
}

/// A per-player counter of `p` towards `other` kept in the controller state
/// (`AI_getSameReligionCounter` & co).
fn counter_of(cai: &Cai, g: &Game, p: PlayerId, other: PlayerId, f: impl Fn(&PlayerAi) -> &Vec<i32>) -> i32 {
    let n = g.players.len();
    let mut s = cai.s();
    s.ensure_players(n);
    f(s.player(p)).get(other.idx()).copied().unwrap_or(0)
}

/// C++ `AI_bestAreaUnitAIValue(eUnitAI, NULL, &eBestUnit)`, i.e. `AI_bestCityUnitAIValue(eUnitAI,
/// NULL, &eBestUnit)`: the best `AI_unitValue` of the units the player can train.
/// PORT ADAPT: the stubs take a non-NULL area/city; `AI_unitValue`'s area (NULL in C++) is the
/// capital's area here (any city's without a capital; 0 without cities).
fn best_unit_ai_value_no_city(cai: &Cai, g: &Game, p: PlayerId, unit_ai: UnitAIType, best_unit_type: &mut Option<UnitType>) -> i32 {
    let d = g.d;
    let area = g.player_get_capital_city(p).or_else(|| g.players[p].cities.first().copied()).map(|c| g.city_area(c));
    let Some(area) = area else {
        return 0;
    };
    let civ = civ_of(g, p);

    let mut best_value = 0;

    for class in d.unit_classes.keys() {
        let Some(loop_unit) = d.civ_unit(civ, class) else {
            continue;
        };

        if !g.players[p].human || d.units[loop_unit].default_unit_ai_type == Some(unit_ai) {
            if g.player_can_train(p, loop_unit, false, false, false) {
                let value = cai.player_ai_unit_value(g, p, loop_unit, unit_ai, area);
                if value > best_value {
                    best_value = value;
                    *best_unit_type = Some(loop_unit);
                }
            }
        }
    }

    best_value
}
