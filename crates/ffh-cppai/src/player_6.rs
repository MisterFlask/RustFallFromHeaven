//! Port of CvPlayerAI.cpp:16742-19674 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES (changes made here, and changes other files need):
//! - `player_ai_recalculate_found_values`, `player_ai_update_city_sites` take `&mut Game`
//!   (C++ `const`, but they write plot found values / area best found values). Their only
//!   caller is `AI_updateFoundValues` (player_1.rs, `&mut Game`), whose call compiles as is.
//! - `player_ai_advanced_start_find_capital_plot` takes `&mut Game` (its fallback calls
//!   `AI_updateFoundValues`); its only caller is `AI_doAdvancedStart` in this file.
//! - `player_ai_unit_value` (player_3.rs) should take `Option<AreaId>`: C++ passes a NULL area
//!   from `AI_bestCityUnitAIValue(.., NULL)`; until then `unit_value_opt_area` stands in.
//! - `player_ai_best_area_unit_ai_value` should take `Option<AreaId>` and
//!   `player_ai_best_city_unit_ai_value` `Option<CityId>` (C++ callers pass NULL); the bodies
//!   are in `best_area_unit_ai_value_opt` / `best_city_unit_ai_value_opt` already.
//! - `player_ai_set_expansion_plot` should take `Option<PlotIdx>` (C++ passes NULL from
//!   `AI_doExpansion`); `set_expansion_plot_opt` has the body.
//!
//! `AI_doAdvancedStart` is ported but not hooked: the rules (`player_do_advanced_start_action`,
//! Exit/Automate) leave `// AI: AI_doAdvancedStart(true)` comments and there is no `AiHook` for it.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;
use ffh_core::player::AdvancedStartAction;

impl Cai {
    /// C++ `void CvPlayerAI::AI_nowHasTech(TechTypes eTech)` (CvPlayerAI.cpp:16742-16764)
    pub fn player_ai_now_has_tech(&self, g: &mut Game, p: PlayerId, tech: TechType) {
        // while its _possible_ to do checks, for financial trouble, and this tech adds financial buildings
        // if in war and this tech adds important war units
        // etc
        // it makes more sense to just redetermine what to produce
        // that is already done every time a civ meets a new civ, it makes sense to do it when a new tech is learned
        // if this is changed, then at a minimum, AI_isFinancialTrouble should be checked
        if !g.players[p].human {
            let game_turn = g.game_turn();

            // only reset at most every 10 turns
            let last = self.s().player(p).turn_last_production_dirty;
            if game_turn > last + 10 {
                // redeterimine the best things to build in each city
                self.player_ai_make_production_dirty(g, p);

                self.s().player(p).turn_last_production_dirty = game_turn;
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_countDeadlockedBonuses(CvPlot* pPlot) const` (CvPlayerAI.cpp:16767-16825)
    pub fn player_ai_count_deadlocked_bonuses(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        let team = g.players[p].team;
        let (px, py) = g.map.xy(plot);
        let plot_area = g.map.plots[plot].area;

        let min_range = g.d.defines.min_city_range;
        let range = min_range * 2;
        let mut count = 0;

        for dx in -range..=range {
            for dy in -range..=range {
                if g.plot_distance(dx, dy, 0, 0) > CITY_PLOTS_RADIUS {
                    let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                        continue;
                    };
                    if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                        if !g.plot_is_city_radius(loop_plot) && (g.map.plots[loop_plot].area == plot_area || g.plot_is_water(loop_plot)) {
                            let mut can_found = false;
                            let mut never_found = true;
                            //potentially blockable resource
                            //look for a city site within a city radius
                            let (lx, ly) = g.map.xy(loop_plot);
                            for i in 0..NUM_CITY_PLOTS {
                                if let Some(loop_plot2) = g.plot_city(lx, ly, i) {
                                    let (l2x, l2y) = g.map.xy(loop_plot2);
                                    //canFound usually returns very quickly
                                    if g.player_can_found(p, l2x, l2y, false) {
                                        never_found = false;
                                        if g.step_distance(px, py, l2x, l2y) > min_range {
                                            can_found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            if !never_found && !can_found {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_getOurPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves) const` (CvPlayerAI.cpp:16827-16889)
    pub fn player_ai_get_our_plot_strength(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool) -> i32 {
        let team = g.players[p].team;
        let (px, py) = g.map.xy(plot);
        let plot_area = g.map.plots[plot].area;
        let str_plot = if defensive_bonuses { Some(plot) } else { None };
        let mut value = 0;

        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                    continue;
                };
                if g.map.plots[loop_plot].area == plot_area {
                    let (lx, ly) = g.map.xy(loop_plot);
                    let distance = g.step_distance(px, py, lx, ly);
                    for &u in &g.map.plots[loop_plot].units {
                        if g.units[u].owner == p {
                            if (defensive_bonuses && g.unit_can_defend(u, None)) || g.unit_can_attack(u) {
                                if !g.unit_is_invisible(u, team, true) {
                                    if g.unit_at_plot(u, plot)
                                        || g.unit_can_move_into(u, plot, false, false, false)
                                        || g.unit_can_move_into(u, plot, /*bAttack*/ true, false, false)
                                    {
                                        if !test_moves {
                                            value += g.unit_curr_effective_str(u, str_plot, None, None);
                                        } else if g.unit_base_moves(u) >= distance {
                                            value += g.unit_curr_effective_str(u, str_plot, None, None);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        value
    }

    /// C++ `int CvPlayerAI::AI_getEnemyPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves) const` (CvPlayerAI.cpp:16891-16956)
    pub fn player_ai_get_enemy_plot_strength(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool) -> i32 {
        let team = g.players[p].team;
        let (px, py) = g.map.xy(plot);
        let plot_area = g.map.plots[plot].area;
        let str_plot = if defensive_bonuses { Some(plot) } else { None };
        let mut value = 0;

        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                    continue;
                };
                if g.map.plots[loop_plot].area == plot_area {
                    let (lx, ly) = g.map.xy(loop_plot);
                    let distance = g.step_distance(px, py, lx, ly);
                    for &u in &g.map.plots[loop_plot].units {
                        if g.at_war(g.unit_get_team(u), team) {
                            if (defensive_bonuses && g.unit_can_defend(u, None)) || g.unit_can_attack(u) {
                                if !g.unit_is_invisible(u, team, true) {
                                    if g.plot_is_valid_domain_for_action(plot, u) {
                                        if !test_moves {
                                            value += g.unit_curr_effective_str(u, str_plot, None, None);
                                        } else {
                                            let mut danger_range = g.unit_base_moves(u);
                                            danger_range += if g.plot_is_valid_route(loop_plot, u) { 1 } else { 0 };
                                            if danger_range >= distance {
                                                value += g.unit_curr_effective_str(u, str_plot, None, None);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        value
    }

    /// C++ `int CvPlayerAI::AI_goldToUpgradeAllUnits(int iExpThreshold = 0) const` (CvPlayerAI.cpp:16958-17067)
    pub fn player_ai_gold_to_upgrade_all_units(&self, g: &Game, p: PlayerId, exp_threshold: i32) -> i32 {
        let game_turn = g.game_turn();
        let (cache_turn, cached_threshold, cached_gold) = {
            let mut s = self.s();
            let pa = s.player(p);
            (pa.upgrade_units_cache_turn, pa.upgrade_units_cached_exp_threshold, pa.upgrade_units_cached_gold)
        };
        if cache_turn == game_turn && cached_threshold == exp_threshold {
            return cached_gold;
        }

        let d = g.d;
        let mut total_gold = 0;

        // cache the value for each unit type
        let mut unit_upgrade_price: IdxVec<UnitType, i32> = IdxVec::from_elem(0, d.units.len()); // initializes to zeros

        let capital = g.player_get_capital_city(p);

        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            // if experience is below threshold, skip this unit
            if g.units[u].experience < exp_threshold {
                continue;
            }

            let unit_type = g.units[u].unit_type;

            // check cached value for this unit type
            let cached_unit_gold = unit_upgrade_price[unit_type];
            if cached_unit_gold != 0 {
                // if positive, add it to the sum
                if cached_unit_gold > 0 {
                    total_gold += cached_unit_gold;
                }

                // either way, done with this unit
                continue;
            }

            let mut unit_gold = 0;
            let mut unit_upgrade_possibilities = 0;

            let unit_ai_type = g.units[u].ai_type;
            let unit_area = g.unit_area(u);
            // PORT ADAPT: C++ AI_unitValue(.., NO_UNITAI, ..) for a unit without a UnitAI;
            // such a unit (and its upgrades) are valued 0 here.
            let unit_value = match unit_ai_type {
                Some(ai) => self.player_ai_unit_value(g, p, unit_type, ai, unit_area),
                None => 0,
            };

            for upgrade_class in d.unit_classes.keys() {
                let Some(upgrade_unit_type) = civ_unit(g, p, upgrade_class) else {
                    continue;
                };

                // is it better?
                let upgrade_value = match unit_ai_type {
                    Some(ai) => self.player_ai_unit_value(g, p, upgrade_unit_type, ai, unit_area),
                    None => 0,
                };
                if upgrade_value > unit_value {
                    // is this a valid upgrade?
                    if g.unit_upgrade_available(u, unit_type, upgrade_class, 0) {
                        // can we actually make this upgrade?
                        let mut can_upgrade = false;
                        if capital.map_or(false, |c| g.city_can_train(c, upgrade_unit_type, false, false, false, false)) {
                            can_upgrade = true;
                        } else {
                            let (ux, uy) = (g.units[u].x, g.units[u].y);
                            let coastal = g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA;
                            let close_city = g.map_find_city(ux, uy, Some(p), None, true, coastal, None, None, None);
                            if let Some(cc) = close_city {
                                if g.city_can_train(cc, upgrade_unit_type, false, false, false, false) {
                                    can_upgrade = true;
                                }
                            }
                        }

                        if can_upgrade {
                            unit_gold += g.unit_upgrade_price(u, upgrade_unit_type);
                            unit_upgrade_possibilities += 1;
                        }
                    }
                }
            }

            // if we found any, find average and add to total
            if unit_upgrade_possibilities > 0 {
                unit_gold /= unit_upgrade_possibilities;

                // add to cache
                unit_upgrade_price[unit_type] = unit_gold;

                // add to sum
                total_gold += unit_gold;
            } else {
                // add to cache, dont upgrade to this type
                unit_upgrade_price[unit_type] = -1;
            }
        }

        {
            let mut s = self.s();
            let pa = s.player(p);
            pa.upgrade_units_cache_turn = game_turn;
            pa.upgrade_units_cached_exp_threshold = exp_threshold;
            pa.upgrade_units_cached_gold = total_gold;
        }

        total_gold
    }

    /// C++ `int CvPlayerAI::AI_goldTradeValuePercent() const` (CvPlayerAI.cpp:17069-17078)
    pub fn player_ai_gold_trade_value_percent(&self, g: &Game, p: PlayerId) -> i32 {
        let mut value = 2;
        if self.player_ai_is_financial_trouble(g, p) {
            value += 1;
        }
        100 * value
    }

    /// C++ `int CvPlayerAI::AI_averageYieldMultiplier(YieldTypes eYield) const` (CvPlayerAI.cpp:17080-17092)
    pub fn player_ai_average_yield_multiplier(&self, g: &Game, p: PlayerId, yield_: YieldType) -> i32 {
        self.ensure_averages(g, p);
        self.s().player(p).average_yield_multiplier[yield_.idx()]
    }

    /// C++ `int CvPlayerAI::AI_averageCommerceMultiplier(CommerceTypes eCommerce) const` (CvPlayerAI.cpp:17094-17105)
    pub fn player_ai_average_commerce_multiplier(&self, g: &Game, p: PlayerId, commerce: CommerceType) -> i32 {
        self.ensure_averages(g, p);
        self.s().player(p).average_commerce_multiplier[commerce.idx()]
    }

    /// C++ `int CvPlayerAI::AI_averageGreatPeopleMultiplier() const` (CvPlayerAI.cpp:17107-17114)
    pub fn player_ai_average_great_people_multiplier(&self, g: &Game, p: PlayerId) -> i32 {
        self.ensure_averages(g, p);
        self.s().player(p).average_great_people_multiplier
    }

    /// C++ `int CvPlayerAI::AI_averageCommerceExchange(CommerceTypes eCommerce) const` (CvPlayerAI.cpp:17117-17128)
    pub fn player_ai_average_commerce_exchange(&self, g: &Game, p: PlayerId, commerce: CommerceType) -> i32 {
        //"100 eCommerce is worth (return) raw YIELD_COMMERCE
        self.ensure_averages(g, p);
        self.s().player(p).average_commerce_exchange[commerce.idx()]
    }

    /// C++ `void CvPlayerAI::AI_calculateAverages() const` (CvPlayerAI.cpp:17130-17239)
    pub fn player_ai_calculate_averages(&self, g: &Game, p: PlayerId) {
        let mut average_yield_multiplier = [0i32; YieldType::NUM];
        let mut average_commerce_multiplier = [0i32; CommerceType::NUM];
        let mut average_great_people_multiplier = 0;

        let mut total_population = 0;

        let cities = g.players[p].cities.clone();

        for &c in &cities {
            let population = g.cities[c].population.max(NUM_CITY_PLOTS as i32);
            total_population += population;

            for i in 0..YieldType::NUM {
                average_yield_multiplier[i] += population * self.city_ai_yield_multiplier(g, c, YieldType(i as u16));
            }
            for i in 0..CommerceType::NUM {
                average_commerce_multiplier[i] += population * g.city_total_commerce_rate_modifier(c, CommerceType(i as u16));
            }
            average_great_people_multiplier += population * g.city_total_great_people_rate_modifier(c);
        }

        if total_population > 0 {
            for v in average_yield_multiplier.iter_mut() {
                *v /= total_population;
            }
            for v in average_commerce_multiplier.iter_mut() {
                *v /= total_population;
            }
            average_great_people_multiplier /= total_population;
        } else {
            average_yield_multiplier = [100; YieldType::NUM];
            average_commerce_multiplier = [100; CommerceType::NUM];
            average_great_people_multiplier = 100;
        }

        //Calculate Exchange Rate

        let mut average_commerce_exchange = [0i32; CommerceType::NUM];

        let mut total_commerce = 0;

        for &c in &cities {
            let commerce = g.city_yield_rate(c, YieldType::YIELD_COMMERCE);
            total_commerce += commerce;

            let city = &g.cities[c];
            let pl = &g.players[p];
            let mut extra_commerce = 0;
            for i in 0..CommerceType::NUM {
                extra_commerce += (city.specialist_population + city.num_great_people) * pl.specialist_extra_commerce[i];
                extra_commerce += city.building_commerce[i] + city.specialist_commerce[i] + city.religion_commerce[i] + pl.free_city_commerce[i];
            }
            total_commerce += extra_commerce;

            for i in 0..CommerceType::NUM {
                average_commerce_exchange[i] += ((commerce + extra_commerce) * g.city_total_commerce_rate_modifier(c, CommerceType(i as u16))) / 100;
            }
        }

        for v in average_commerce_exchange.iter_mut() {
            if *v > 0 {
                *v = (100 * total_commerce) / *v;
            } else {
                *v = 100;
            }
        }

        let game_turn = g.game_turn();
        let mut s = self.s();
        let pa = s.player(p);
        pa.average_yield_multiplier = average_yield_multiplier;
        pa.average_commerce_multiplier = average_commerce_multiplier;
        pa.average_great_people_multiplier = average_great_people_multiplier;
        pa.average_commerce_exchange = average_commerce_exchange;
        pa.averages_cache_turn = game_turn;
    }

    /// C++ `void CvPlayerAI::AI_convertUnitAITypesForCrush()` (CvPlayerAI.cpp:17241-17285)
    pub fn player_ai_convert_unit_ai_types_for_crush(&self, g: &mut Game, p: PlayerId) {
        let team = g.players[p].team;
        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            let mut valid = false;
            let ai_type = g.units[u].ai_type;
            if ai_type == Some(UnitAIType::UNITAI_RESERVE) || (self.unit_ai_is_city_ai_type(g, u) && g.units[u].extra_city_defense_percent <= 0) {
                valid = true;
            }
            let area_ai = g.map.areas[g.unit_area(u)].area_ai_type[team];
            if area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) {
                valid = false;
            }

            if !g.unit_can_attack(u) || ai_type == Some(UnitAIType::UNITAI_CITY_SPECIAL) {
                valid = false;
            }

            if valid {
                let plot = g.unit_plot(u);
                if g.plot_is_city(plot, false, None) {
                    if let Some(city) = g.map.plots[plot].plot_city {
                        if g.cities[city].owner == p {
                            if g.plot_get_best_defender(plot, Some(p), None, None, false, false, false) == Some(u) {
                                valid = false;
                            }
                        }
                    }
                }
            }

            if valid {
                self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_ATTACK_CITY);
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_playerCloseness(PlayerTypes eIndex, int iMaxDistance) const` (CvPlayerAI.cpp:17287-17301)
    pub fn player_ai_player_closeness(&self, g: &Game, p: PlayerId, index: PlayerId, max_distance: i32) -> i32 {
        let mut value = 0;
        for c in g.players[p].cities.clone() {
            value += self.city_ai_player_closeness(g, c, index, max_distance);
        }
        value
    }

    /// C++ `int CvPlayerAI::AI_getTotalAreaCityThreat(CvArea* pArea) const` (CvPlayerAI.cpp:17304-17320)
    pub fn player_ai_get_total_area_city_threat(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        let mut value = 0;
        for c in g.players[p].cities.clone() {
            if g.city_area(c) == area {
                value += self.city_ai_city_threat(g, c, false);
            }
        }
        value
    }

    /// C++ `int CvPlayerAI::AI_countNumAreaHostileUnits(CvArea* pArea, bool bPlayer, bool bTeam, bool bNeutral, bool bHostile) const` (CvPlayerAI.cpp:17322-17342)
    pub fn player_ai_count_num_area_hostile_units(&self, g: &Game, p: PlayerId, area: AreaId, player: bool, team: bool, neutral: bool, hostile: bool) -> i32 {
        let our_team = g.players[p].team;
        let mut count = 0;

        for i in 0..g.map.plots.len() {
            let loop_plot = PlotIdx(i as u32);
            if g.map.plots[loop_plot].area != Some(area) || !g.plot_is_visible(loop_plot, our_team) {
                continue;
            }
            let owner = g.map.plots[loop_plot].owner;
            let plot_team = g.plot_get_team(loop_plot);
            let owned = g.plot_is_owned(loop_plot);
            if (player && owner == Some(p))
                || (team && plot_team == Some(our_team))
                || (neutral && !owned)
                || (hostile && owned && plot_team.map_or(false, |t| g.teams[our_team].at_war[t]))
            {
                count += g.plot_plot_count(loop_plot, Some(Game::puf_is_enemy), p.i(), false as i32, None, None, Some(Game::puf_is_visible), p.i(), -1);
            }
        }
        count
    }

    /// C++ `int CvPlayerAI::AI_getTotalFloatingDefendersNeeded(CvArea* pArea) const` (CvPlayerAI.cpp:17345-17415)
    pub fn player_ai_get_total_floating_defenders_needed(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        //this doesn't include the minimal one or two garrison units in each city.
        let team = g.players[p].team;
        let mut current_era = g.players[p].current_era.i();
        let area_cities = g.map.areas[area].cities_per_player[p];
        let area_ai = g.map.areas[area].area_ai_type[team];

        current_era = 0.max(current_era - g.start_era().i() / 2);

        let mut defenders = 1 + ((current_era + if g.max_city_elimination() > 0 { 3 } else { 2 }) * area_cities);
        defenders /= 3;
        defenders += g.map.areas[area].population_per_player[p] / 7;

        if area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) {
            defenders *= 2;
        } else if area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING) {
            defenders *= 2;
            defenders /= 3;
        }

        if self.player_ai_get_total_area_city_threat(g, p, area) == 0 {
            defenders /= 2;
        }

        if !g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
            defenders *= 2;
            defenders /= 3;
        }

        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_GET_BETTER_UNITS) && area_ai != Some(AreaAIType::AREAAI_DEFENSIVE) {
            defenders /= 2;
        }

        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3) {
            defenders += 2 * area_cities;
            if area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) {
                defenders *= 2; //go crazy
            }
        }

        defenders *= 60;
        defenders /= 30.max(g.d.handicaps[g.handicap].ai_train_percent - 20);

        //FfH: Modified by Kael 11/25/2008 (because we dont eras like vanilla civ4)
        //	if ((iCurrentEra < 3) && (GC.getGameINLINE().isOption(GAMEOPTION_RAGING_BARBARIANS)))
        if g.is_option(GameOptionType::GAMEOPTION_RAGING_BARBARIANS) {
            //FfH: End Modify
            defenders += 2;
        }

        if let Some(cap) = g.player_get_capital_city(p) {
            if g.city_area(cap) != area {
                //Defend offshore islands only lightly.
                defenders = defenders.min(area_cities * area_cities - 1);
            }
        }

        defenders
    }

    /// C++ `int CvPlayerAI::AI_getTotalFloatingDefenders(CvArea* pArea) const` (CvPlayerAI.cpp:17417-17429)
    pub fn player_ai_get_total_floating_defenders(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        let mut count = 0;

        count += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_COLLATERAL);
        count += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_RESERVE);
        count += 0.max(self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_CITY_DEFENSE) - (g.map.areas[area].cities_per_player[p] * 2));
        count += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_CITY_COUNTER);
        count += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_CITY_SPECIAL);
        count += self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_DEFENSE_AIR);
        count
    }

    /// C++ `RouteTypes CvPlayerAI::AI_bestAdvancedStartRoute(CvPlot* pPlot, int* piYieldValue = NULL) const` (CvPlayerAI.cpp:17431-17471)
    pub fn player_ai_best_advanced_start_route(&self, g: &Game, p: PlayerId, plot: PlotIdx, yield_value: &mut i32) -> Option<RouteType> {
        let d = g.d;
        let mut best_route = None;
        let mut best_value = -1;
        for route in d.routes.keys() {
            let mut value = 0;
            let cost = g.player_get_advanced_start_route_cost(p, Some(route), true, Some(plot));

            if cost >= 0 {
                value += d.routes[route].value;

                if value > 0 {
                    let mut plot_yield_value = 0;
                    if let Some(imp) = g.map.plots[plot].improvement_type {
                        let changes = &d.improvements[imp].route_yield_changes[route];
                        plot_yield_value += changes[YieldType::YIELD_FOOD.idx()] * 100;
                        plot_yield_value += changes[YieldType::YIELD_PRODUCTION.idx()] * 60;
                        plot_yield_value += changes[YieldType::YIELD_COMMERCE.idx()] * 40;
                    }
                    value *= 1000;
                    value /= 1 + cost;

                    if value > best_value {
                        best_value = value;
                        best_route = Some(route);
                        *yield_value = plot_yield_value;
                    }
                }
            }
        }
        best_route
    }

    /// C++ `UnitTypes CvPlayerAI::AI_bestAdvancedStartUnitAI(CvPlot* pPlot, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:17473-17577)
    pub fn player_ai_best_advanced_start_unit_ai(&self, g: &Game, p: PlayerId, plot: PlotIdx, unit_ai: UnitAIType) -> Option<UnitType> {
        let d = g.d;
        let mut best_value = 0;
        let mut best_unit = None;
        let plot_area = g.map.plots[plot].area;

        for class in d.unit_classes.keys() {
            let Some(loop_unit) = civ_unit(g, p, class) else {
                continue;
            };
            let info = &d.units[loop_unit];

            //if (!isHuman() || (GC.getUnitInfo(eLoopUnit).getDefaultUnitAIType() == eUnitAI))
            let unit_cost = g.player_get_advanced_start_unit_cost(p, loop_unit, true, Some(plot));
            if unit_cost >= 0 {
                let mut value = self.unit_value_opt_area(g, p, loop_unit, unit_ai, plot_area);

                if value > 0 {
                    //free promotions. slow?
                    //only 1 promotion per source is counted (ie protective isn't counted twice)
                    let mut promotion_value = 0;

                    //special to the unit
                    for promotion in d.promotions.keys() {
                        if info.free_promotions[promotion] {
                            promotion_value += 15;
                            break;
                        }
                    }

                    for promotion in d.promotions.keys() {
                        if info.unit_combat_type.map_or(false, |uc| g.player_is_free_promotion_unit_combat(p, uc, promotion)) {
                            promotion_value += 15;
                            break;
                        }

                        if info.unit_class_type.map_or(false, |ucl| g.player_is_free_promotion_unit_class(p, ucl, promotion)) {
                            promotion_value += 15;
                            break;
                        }
                    }

                    //traits
                    for t in d.traits.keys() {
                        if g.players[p].traits[t] {
                            for promotion in d.promotions.keys() {
                                if d.traits[t].free_promotion[promotion] {
                                    if let Some(uc) = info.unit_combat_type {
                                        if d.traits[t].free_promotion_unit_combat[uc] {
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

                    value *= self.rand(40, "AI Best Advanced Start Unit") + 100;
                    value /= 100;

                    let num_cities = g.players[p].num_cities();
                    value *= num_cities + 2;
                    value /= g.player_get_unit_class_count_plus_making(p, class) + num_cities + 2;

                    // (C++ FAssert((MAX_INT / 1000) > iValue).)
                    value = value.saturating_mul(1000);

                    value /= 1 + unit_cost;

                    value = 1.max(value);

                    if value > best_value {
                        best_value = value;
                        best_unit = Some(loop_unit);
                    }
                }
            }
        }

        best_unit
    }

    /// C++ `CvPlot* CvPlayerAI::AI_advancedStartFindCapitalPlot() const` (CvPlayerAI.cpp:17579-17680)
    /// (`&mut Game`: the fallback runs `AI_updateFoundValues`; see SIGNATURE NOTES.)
    pub fn player_ai_advanced_start_find_capital_plot(&self, g: &mut Game, p: PlayerId) -> Option<PlotIdx> {
        let team = g.players[p].team;
        let mut best_plot = None;
        let mut best_value = -1;

        for other in g.player_ids() {
            if g.players[other].alive && g.players[other].team == team {
                if let Some(loop_plot) = g.player_get_starting_plot(other) {
                    if g.player_get_advanced_start_city_cost(p, true, Some(loop_plot)) > 0 {
                        let (x, y) = g.map.xy(loop_plot);

                        let mut value = 1000;
                        if other == p {
                            value += 1000;
                        } else {
                            value += self.rand(100, "AI Advanced Start Choose Team Start");
                        }
                        if let Some(nearest) = g.map_find_city(x, y, None, Some(team), true, false, None, None, None) {
                            let distance = g.step_distance(x, y, g.cities[nearest].x, g.cities[nearest].y);
                            if distance < 10 {
                                value /= 10 - distance;
                            }
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(loop_plot);
                        }
                    }
                }
                // else: FAssertMsg(false, "StartingPlot for a live player is NULL!")
            }
        }

        if best_plot.is_some() {
            return best_plot;
        }

        //Execution should almost never reach here.

        //Update found values just in case - particulary important for simultaneous turns.
        self.player_ai_update_found_values(g, p, false);

        best_plot = None;
        best_value = -1;

        if let Some(start) = g.player_get_starting_plot(p) {
            let start_area = g.map.plots[start].area;
            for i in 0..g.map.plots.len() {
                let loop_plot = PlotIdx(i as u32);
                if g.map.plots[loop_plot].area == start_area {
                    let value = g.plot_get_found_value(loop_plot, p);
                    if value > 0 {
                        if g.player_get_advanced_start_city_cost(p, true, Some(loop_plot)) > 0 {
                            if value > best_value {
                                best_value = value;
                                best_plot = Some(loop_plot);
                            }
                        }
                    }
                }
            }
        }

        //Commence panic (None).
        best_plot
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartPlaceExploreUnits(bool bLand)` (CvPlayerAI.cpp:17683-17765)
    pub fn player_ai_advanced_start_place_explore_units(&self, g: &mut Game, p: PlayerId, land: bool) -> bool {
        let team = g.players[p].team;
        let mut best_explore_plot = None;
        let mut best_explore_value = 0;
        let mut best_unit_type = None;

        let unit_ai = if land { UnitAIType::UNITAI_EXPLORE } else { UnitAIType::UNITAI_EXPLORE_SEA };

        for c in g.players[p].cities.clone() {
            let loop_plot = g.city_plot(c);
            let loop_area = if land { Some(g.city_area(c)) } else { g.plot_water_area(loop_plot) };

            let Some(loop_area) = loop_area else {
                continue;
            };
            let mut value = 0.max(g.area_get_num_unrevealed_tiles(loop_area, team) - 10) * 10;
            value += 0.max(g.map.areas[loop_area].num_tiles - 50);

            if value > 0 {
                let mut other_plot_count = 0;
                let mut goody_count = 0;
                let mut explorer_count = 0;

                let (px, py) = g.map.xy(loop_plot);
                let range = 4;
                for dx in -range..=range {
                    for dy in -range..=range {
                        if let Some(loop_plot2) = g.plot_xy(px, py, dx, dy) {
                            explorer_count += g.plot_plot_count(loop_plot2, Some(Game::puf_is_unit_ai_type), unit_ai.i(), -1, None, Some(team), None, -1, -1);
                            if g.map.plots[loop_plot2].area == Some(loop_area) {
                                if g.plot_is_goody(loop_plot2, None) {
                                    goody_count += 1;
                                }
                                if g.plot_get_team(loop_plot2) != Some(team) {
                                    other_plot_count += 1;
                                }
                            }
                        }
                    }
                }

                value -= 300 * explorer_count;
                value += 200 * goody_count;
                value += 10 * other_plot_count;
                if value > best_explore_value {
                    if let Some(unit) = self.player_ai_best_advanced_start_unit_ai(g, p, loop_plot, unit_ai) {
                        best_unit_type = Some(unit);
                        best_explore_value = value;
                        best_explore_plot = Some(loop_plot);
                    }
                }
            }
        }

        if let (Some(plot), Some(unit)) = (best_explore_plot, best_unit_type) {
            let (x, y) = g.map.xy(plot);
            g.player_do_advanced_start_action(p, AdvancedStartAction::Unit, x, y, unit.i(), true);
            return true;
        }
        false
    }

    /// C++ `void CvPlayerAI::AI_advancedStartRevealRadius(CvPlot* pPlot, int iRadius)` (CvPlayerAI.cpp:17767-17790)
    pub fn player_ai_advanced_start_reveal_radius(&self, g: &mut Game, p: PlayerId, plot: PlotIdx, radius: i32) {
        let (px, py) = g.map.xy(plot);
        for range in 1..=radius {
            for dx in -range..=range {
                for dy in -range..=range {
                    if g.plot_distance(0, 0, dx, dy) <= radius {
                        if let Some(loop_plot) = g.plot_xy(px, py, dx, dy) {
                            if g.player_get_advanced_start_visibility_cost(p, true, Some(loop_plot)) > 0 {
                                let (lx, ly) = g.map.xy(loop_plot);
                                g.player_do_advanced_start_action(p, AdvancedStartAction::Visibility, lx, ly, -1, true);
                            }
                        }
                    }
                }
            }
        }
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartPlaceCity(CvPlot* pPlot)` (CvPlayerAI.cpp:17792-17899)
    pub fn player_ai_advanced_start_place_city(&self, g: &mut Game, p: PlayerId, plot: PlotIdx) -> bool {
        let d = g.d;
        let (px, py) = g.map.xy(plot);

        //If there is already a city, then improve it.
        let mut city = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c));
        if city.is_none() {
            g.player_do_advanced_start_action(p, AdvancedStartAction::City, px, py, -1, true);

            city = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c));
            match city {
                Some(c) if g.cities[c].owner == p => {}
                _ => {
                    //this should never happen since the cost for a city should be 0 if
                    //the city can't be placed.
                    //(It can happen if another player has placed a city in the fog)
                    return false;
                }
            }
        }
        let c = city.unwrap();

        if opt_to_i32(g.cities[c].culture_level) <= 1 {
            g.player_do_advanced_start_action(p, AdvancedStartAction::Culture, px, py, -1, true);
        }

        //to account for culture expansion.
        self.city_ai_update_best_build(g, c);

        let mut plots_improved = 0;
        for i in 0..NUM_CITY_PLOTS {
            if i != CITY_HOME_PLOT {
                if let Some(loop_plot) = g.plot_city(px, py, i) {
                    if g.map.plots[loop_plot].working_city == Some(c) {
                        if g.map.plots[loop_plot].improvement_type.is_some() {
                            plots_improved += 1;
                        }
                    }
                }
            }
        }

        let target_population = g.city_happy_level(c) + (g.players[p].current_era.i() / 2);

        while plots_improved < target_population {
            let mut best_plot = None;
            let mut best_improvement = None;
            let mut best_value = 0;
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            for i in 0..NUM_CITY_PLOTS {
                let value = self.city_ai_get_best_build_value(g, c, i as i32);
                if value > best_value {
                    if let Some(build) = self.city_ai_get_best_build(g, c, i as i32) {
                        if let Some(improvement) = d.builds[build].improvement {
                            if let Some(loop_plot) = g.plot_city(cx, cy, i) {
                                if g.map.plots[loop_plot].improvement_type != Some(improvement) {
                                    best_improvement = Some(improvement);
                                    best_plot = Some(loop_plot);
                                    best_value = value;
                                }
                            }
                        }
                    }
                }
            }

            if best_value > 0 {
                let best_plot = best_plot.expect("AI_advancedStartPlaceCity: best plot");
                let (bx, by) = g.map.xy(best_plot);
                g.player_do_advanced_start_action(p, AdvancedStartAction::Improvement, bx, by, opt_to_i32(best_improvement), true);
                plots_improved += 1;
                if g.cities[c].population < plots_improved {
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Pop, bx, by, -1, true);
                }
            } else {
                break;
            }
        }

        while plots_improved > g.cities[c].population {
            let pop_cost = g.player_get_advanced_start_pop_cost(p, true, Some(c));
            if pop_cost <= 0 || pop_cost > g.players[p].advanced_start_points {
                break;
            }
            if g.city_health_rate(c, false, 0) < 0 {
                break;
            }
            let population_before = g.cities[c].population;
            g.player_do_advanced_start_action(p, AdvancedStartAction::Pop, px, py, -1, true);
            if g.cities[c].population == population_before {
                // PORT ADAPT: guard against an endless loop if the rules refused the pop.
                break;
            }
        }

        self.city_ai_update_assign_work(g, c);

        true
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartDoRoute(CvPlot* pFromPlot, CvPlot* pToPlot)` (CvPlayerAI.cpp:17902-17937)
    pub fn player_ai_advanced_start_do_route(&self, g: &mut Game, p: PlayerId, from_plot: PlotIdx, to_plot: PlotIdx) -> bool {
        //Returns false if we have no more points.
        let (fx, fy) = g.map.xy(from_plot);
        let (tx, ty) = g.map.xy(to_plot);

        // The path (last node back to the start), read out of the step finder before acting.
        let path: Option<(i32, Vec<(i32, i32)>)> = {
            let gr: &Game = g;
            let mut finder = gr.step_finder.borrow_mut();
            finder.force_reset();
            if finder.generate_path(gr, fx, fy, tx, ty, false, 0, true) {
                match finder.last_node() {
                    Some(node) => {
                        let data1 = node.data1;
                        let mut nodes = Vec::new();
                        let mut cur = Some(node);
                        while let Some(n) = cur {
                            nodes.push((n.x, n.y));
                            cur = finder.parent_of(n);
                        }
                        Some((data1, nodes))
                    }
                    None => Some((-1, Vec::new())),
                }
            } else {
                None
            }
        };

        if let Some((data1, nodes)) = path {
            if !nodes.is_empty() {
                if data1 > 1 + g.step_distance(fx, fy, tx, ty) {
                    //Don't build convulted paths.
                    return true;
                }
            }

            for (nx, ny) in nodes {
                let Some(plot) = g.map.plot_soren(nx, ny) else {
                    continue;
                };
                let route = self.player_ai_best_advanced_start_route(g, p, plot, &mut 0);
                if let Some(route) = route {
                    if g.player_get_advanced_start_route_cost(p, Some(route), true, Some(plot)) > g.players[p].advanced_start_points {
                        return false;
                    }
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Route, nx, ny, route.i(), true);
                }
            }
        }
        true
    }

    /// C++ `void CvPlayerAI::AI_advancedStartRouteTerritory()` (CvPlayerAI.cpp:17939-18139)
    pub fn player_ai_advanced_start_route_territory(&self, g: &mut Game, p: PlayerId) {
        let d = g.d;
        let team = g.players[p].team;

        // (The commented-out plot-group heuristic of the C++ is omitted.)

        for i in 0..g.map.plots.len() {
            let loop_plot = PlotIdx(i as u32);
            if g.map.plots[loop_plot].owner == Some(p) && g.map.plots[loop_plot].route_type.is_none() {
                if let Some(improvement) = g.map.plots[loop_plot].improvement_type {
                    if let Some(bonus) = g.plot_get_bonus_type(loop_plot, Some(team)) {
                        if d.improvements[improvement].improvement_bonus[bonus].bonus_trade {
                            let bonus_value = self.player_ai_bonus_val(g, p, bonus, 1);
                            if bonus_value > 9 {
                                let mut best_value = 0;
                                let mut best_plot = None;
                                let (lx, ly) = g.map.xy(loop_plot);
                                let range = 2;
                                for dx in -range..=range {
                                    for dy in -range..=range {
                                        let Some(loop_plot2) = g.plot_xy(lx, ly, dx, dy) else {
                                            continue;
                                        };
                                        if g.map.plots[loop_plot2].owner == Some(p) {
                                            let is_city = g.plot_is_city(loop_plot2, false, None);
                                            if g.plot_is_connected_to_capital(loop_plot2, None) || is_city {
                                                let mut value = 1000;
                                                if is_city {
                                                    value += 100;
                                                    if let Some(pc) = g.map.plots[loop_plot2].plot_city {
                                                        if g.city_is_capital(pc) {
                                                            value += 100;
                                                        }
                                                    }
                                                }
                                                if g.plot_is_route(loop_plot2) {
                                                    value += 100;
                                                }
                                                let distance = g.map_calculate_path_distance(loop_plot, loop_plot2);
                                                if distance > 0 {
                                                    value /= 1 + distance;

                                                    if value > best_value {
                                                        best_value = value;
                                                        best_plot = Some(loop_plot2);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(best_plot) = best_plot {
                                    if !self.player_ai_advanced_start_do_route(g, p, loop_plot, best_plot) {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    if g.map.plots[loop_plot].route_type.is_none() {
                        let mut route_yield_value = 0;
                        let route = self.player_ai_best_advanced_start_route(g, p, loop_plot, &mut route_yield_value);
                        if let Some(route) = route {
                            if route_yield_value > 0 {
                                let (lx, ly) = g.map.xy(loop_plot);
                                g.player_do_advanced_start_action(p, AdvancedStartAction::Route, lx, ly, route.i(), true);
                            }
                        }
                    }
                }
            }
        }

        //Connect Cities
        for c in g.players[p].cities.clone() {
            if !g.cities.contains(c) {
                continue;
            }
            if !g.city_is_capital(c) && !g.city_is_connected_to_capital(c, None) {
                let mut best_value = 0;
                let mut best_plot = None;
                let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                let city_plot = g.city_plot(c);
                let range = 5;
                for dx in -range..=range {
                    for dy in -range..=range {
                        let Some(loop_plot) = g.plot_xy(cx, cy, dx, dy) else {
                            continue;
                        };
                        if g.map.plots[loop_plot].owner == Some(p) {
                            let is_city = g.plot_is_city(loop_plot, false, None);
                            if g.plot_is_connected_to_capital(loop_plot, None) || is_city {
                                let mut value = 1000;
                                if is_city {
                                    value += 500;
                                    if let Some(pc) = g.map.plots[loop_plot].plot_city {
                                        if g.city_is_capital(pc) {
                                            value += 500;
                                        }
                                    }
                                }
                                if g.plot_is_route(loop_plot) {
                                    value += 100;
                                }
                                let distance = g.map_calculate_path_distance(city_plot, loop_plot);
                                if distance > 0 {
                                    value /= 1 + distance;

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = Some(loop_plot);
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(best_plot) = best_plot {
                    if !self.player_ai_advanced_start_do_route(g, p, best_plot, city_plot) {
                        return;
                    }
                }
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_doAdvancedStart(bool bNoExit = false)` (CvPlayerAI.cpp:18142-18459)
    pub fn player_ai_do_advanced_start(&self, g: &mut Game, p: PlayerId, no_exit: bool) {
        let d = g.d;
        let team = g.players[p].team;

        if g.player_get_starting_plot(p).is_none() {
            return;
        }

        let starting_points = g.players[p].advanced_start_points;
        let reveal_points = (starting_points * 10) / 100;
        let build_unit_prob = g.players[p].personality_type.map_or(0, |l| d.leader_heads[l].build_unit_prob);
        let military_points = (starting_points * if g.players[p].human { 17 } else { 10 + (build_unit_prob / 3) }) / 100;
        let mut city_points = starting_points - (military_points + reveal_points);

        if let Some(cap) = g.player_get_capital_city(p) {
            let cap_plot = g.city_plot(cap);
            self.player_ai_advanced_start_place_city(g, p, cap_plot);
        } else {
            let mut pass = 0;
            while pass < 2 && g.player_get_capital_city(p).is_none() {
                let best_capital_plot = self.player_ai_advanced_start_find_capital_plot(g, p);

                if let Some(best_capital_plot) = best_capital_plot {
                    // (FAssertMsg on failure: "AS AI: Unexpected failure placing capital")
                    self.player_ai_advanced_start_place_city(g, p, best_capital_plot);
                    break;
                } else {
                    //If this point is reached, the advanced start system is broken.
                    //Find a new starting plot for this player
                    let new_start = g.player_find_starting_plot(p, false);
                    g.player_set_starting_plot(p, new_start, true);
                    //Redo Starting visibility
                    if let Some(starting_plot) = g.player_get_starting_plot(p) {
                        let (sx, sy) = g.map.xy(starting_plot);
                        let sight = d.defines.advanced_start_sight_range;
                        for i in 0..g.map.plots.len() {
                            let plot = PlotIdx(i as u32);
                            let (x, y) = g.map.xy(plot);
                            if g.plot_distance(x, y, sx, sy) <= sight {
                                g.plot_set_revealed(plot, team, true, false, None, false);
                            }
                        }
                    }
                }
                pass += 1;
            }

            if g.player_get_capital_city(p).is_none() {
                if !no_exit {
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Exit, -1, -1, -1, true);
                }
                return;
            }
        }

        city_points -= starting_points - g.players[p].advanced_start_points;

        let mut last_points_total = g.players[p].advanced_start_points;

        for _pass in 0..6 {
            for i in 0..g.map.plots.len() {
                let loop_plot = PlotIdx(i as u32);
                if g.map.plots[loop_plot].revealed[team] {
                    if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                        self.player_ai_advanced_start_reveal_radius(g, p, loop_plot, CITY_PLOTS_RADIUS);
                    } else {
                        let (lx, ly) = g.map.xy(loop_plot);
                        for j in 0..CardinalDirectionType::NUM {
                            let Some(loop_plot2) = g.plot_cardinal_direction(lx, ly, CardinalDirectionType(j as u16)) else {
                                continue;
                            };
                            if g.player_get_advanced_start_visibility_cost(p, true, Some(loop_plot2)) > 0 {
                                //Mildly maphackery but any smart human can see the terrain type of a tile.
                                let Some(terrain) = g.map.plots[loop_plot2].terrain_type else {
                                    continue;
                                };
                                let mut food_yield = d.terrains[terrain].yield_[YieldType::YIELD_FOOD.idx()];
                                if let Some(feature) = g.map.plots[loop_plot2].feature_type {
                                    food_yield += d.features[feature].yield_change[YieldType::YIELD_FOOD.idx()];
                                }
                                if ((food_yield >= 2) && !g.plot_is_fresh_water(loop_plot2)) || g.plot_is_hills(loop_plot2) || g.plot_is_river(loop_plot2) {
                                    let (l2x, l2y) = g.map.xy(loop_plot2);
                                    g.player_do_advanced_start_action(p, AdvancedStartAction::Visibility, l2x, l2y, -1, true);
                                }
                            }
                        }
                    }
                }
                if (last_points_total - g.players[p].advanced_start_points) > reveal_points {
                    break;
                }
            }
        }

        last_points_total = g.players[p].advanced_start_points;
        city_points = city_points.min(last_points_total);
        let area = -1; //getStartingPlot()->getArea();

        //Spend econ points on a tech?
        let mut tech_rand = 90 + self.rand(20, "AI AS Buy Tech 1");
        let mut total_tech_spending = 0;

        if g.players[p].current_era.i() == 0 {
            let tech = self.player_ai_best_tech(g, p, 1, false, false, None, None);
            if let Some(tech) = tech.filter(|&t| !d.techs[t].repeat) {
                let tech_cost = g.player_get_advanced_start_tech_cost(p, Some(tech), true);
                if tech_cost > 0 {
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Tech, -1, -1, tech.i(), true);
                    tech_rand -= 50;
                    total_tech_spending += tech_cost;
                }
            }
        }

        let mut done_placing_cities = false;
        for _pass in 0..100 {
            let rand_threshold = tech_rand + 10 * g.players[p].num_cities();
            if rand_threshold > 0 && self.rand(100, "AI AS Buy Tech 2") < rand_threshold {
                let tech = self.player_ai_best_tech(g, p, 1, false, false, None, None);
                if let Some(tech) = tech.filter(|&t| !d.techs[t].repeat) {
                    let tech_cost = g.player_get_advanced_start_tech_cost(p, Some(tech), true);
                    if tech_cost > 0 && (tech_cost + total_tech_spending) < (city_points / 4) {
                        g.player_do_advanced_start_action(p, AdvancedStartAction::Tech, -1, -1, tech.i(), true);
                        tech_rand -= 50;
                        total_tech_spending += tech_cost;

                        for c in g.players[p].cities.clone() {
                            if g.cities.contains(c) {
                                let plot = g.city_plot(c);
                                self.player_ai_advanced_start_place_city(g, p, plot);
                            }
                        }
                    }
                }
            }
            let mut best_found_value = 0;
            let mut best_found_plot = None;
            self.player_ai_update_found_values(g, p, false);
            let Some(start) = g.player_get_starting_plot(p) else {
                break;
            };
            let (sx, sy) = g.map.xy(start);
            for i in 0..g.map.plots.len() {
                let loop_plot = PlotIdx(i as u32);
                //if (pLoopPlot->area() == getStartingPlot()->area())
                let (lx, ly) = g.map.xy(loop_plot);
                if g.plot_distance(sx, sy, lx, ly) < 9 {
                    let found_value = g.plot_get_found_value(loop_plot, p);
                    if found_value > best_found_value {
                        if g.player_get_advanced_start_city_cost(p, true, Some(loop_plot)) > 0 {
                            best_found_plot = Some(loop_plot);
                            best_found_value = found_value;
                        }
                    }
                }
            }

            let num_cities = g.players[p].num_cities();
            if best_found_value < if num_cities == 0 { 1 } else { 500 + 250 * num_cities } {
                done_placing_cities = true;
            }
            if !done_placing_cities {
                let cost = g.player_get_advanced_start_city_cost(p, true, best_found_plot);
                if cost > g.players[p].advanced_start_points {
                    done_placing_cities = true;
                }
                // at 500pts, we have 200, we spend 100.
                else if ((last_points_total - g.players[p].advanced_start_points) + cost) > city_points {
                    done_placing_cities = true;
                }
            }

            if !done_placing_cities {
                let plot = best_found_plot.expect("AI_doAdvancedStart: best found plot");
                if !self.player_ai_advanced_start_place_city(g, p, plot) {
                    // (FAssertMsg(false, "AS AI: Failed to place city (non-capital)"))
                    done_placing_cities = true;
                }
            }

            if done_placing_cities {
                break;
            }
        }

        let mut done_with_techs = false;
        while !done_with_techs {
            done_with_techs = true;
            let tech = self.player_ai_best_tech(g, p, 1, false, false, None, None);
            if let Some(tech) = tech.filter(|&t| !d.techs[t].repeat) {
                let tech_cost = g.player_get_advanced_start_tech_cost(p, Some(tech), true);
                if tech_cost > 0 && (tech_cost + last_points_total - g.players[p].advanced_start_points) <= city_points {
                    let points_before = g.players[p].advanced_start_points;
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Tech, -1, -1, tech.i(), true);
                    // PORT ADAPT: stop if the purchase did not go through (C++ would loop).
                    done_with_techs = g.players[p].advanced_start_points == points_before;
                }
            }
        }

        {
            //Land
            self.player_ai_advanced_start_place_explore_units(g, p, true);
            if g.players[p].current_era.i() > 2 {
                //Sea
                self.player_ai_advanced_start_place_explore_units(g, p, false);
                if g.circumnavigation_available() {
                    let alive = g.count_civ_players_alive();
                    if self.rand(alive, "AI AS buy 2nd sea explorer") < 2 {
                        self.player_ai_advanced_start_place_explore_units(g, p, false);
                    }
                }
            }
        }

        self.player_ai_advanced_start_route_territory(g, p);

        let mut done_buildings = (last_points_total - g.players[p].advanced_start_points) > city_points;
        let mut pass = 0;
        while pass < 10 && !done_buildings {
            for c in g.players[p].cities.clone() {
                if !g.cities.contains(c) {
                    continue;
                }
                if let Some(building) = self.city_ai_best_advanced_start_building(g, c, pass) {
                    done_buildings = (last_points_total
                        - (g.players[p].advanced_start_points - g.player_get_advanced_start_building_cost(p, building, true, Some(c))))
                        > city_points;
                    if !done_buildings {
                        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                        g.player_do_advanced_start_action(p, AdvancedStartAction::Building, cx, cy, building.i(), true);
                    } else {
                        //continue there might be cheaper buildings in other cities we can afford
                    }
                }
            }
            pass += 1;
        }

        //Units
        let unit_ai_types = [UnitAIType::UNITAI_CITY_DEFENSE, UnitAIType::UNITAI_WORKER, UnitAIType::UNITAI_RESERVE, UnitAIType::UNITAI_COUNTER];

        let start_area = g.player_get_starting_plot(p).and_then(|s| g.map.plots[s].area);
        let mut done = false;
        for pass in 0..10usize {
            for c in g.players[p].cities.clone() {
                if !g.cities.contains(c) {
                    continue;
                }
                if pass == 0 || Some(g.city_area(c)) == start_area {
                    let unit_plot = g.city_plot(c);
                    //Token defender
                    let best_unit = self.player_ai_best_advanced_start_unit_ai(g, p, unit_plot, unit_ai_types[pass % unit_ai_types.len()]);
                    if let Some(best_unit) = best_unit {
                        if g.player_get_advanced_start_unit_cost(p, best_unit, true, Some(unit_plot)) > g.players[p].advanced_start_points {
                            done = true;
                            break;
                        }
                        let (ux, uy) = g.map.xy(unit_plot);
                        g.player_do_advanced_start_action(p, AdvancedStartAction::Unit, ux, uy, best_unit.i(), true);
                    }
                }
            }
        }

        if g.players[p].human {
            // remove unhappy population
            for c in g.players[p].cities.clone() {
                if !g.cities.contains(c) {
                    continue;
                }
                while g.city_angry_population(c, 0) > 0 && g.player_get_advanced_start_pop_cost(p, false, Some(c)) > 0 {
                    let population_before = g.cities[c].population;
                    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                    g.player_do_advanced_start_action(p, AdvancedStartAction::Pop, cx, cy, -1, false);
                    if !g.cities.contains(c) || g.cities[c].population == population_before {
                        // PORT ADAPT: endless-loop guard.
                        break;
                    }
                }
            }
        }

        if !no_exit {
            g.player_do_advanced_start_action(p, AdvancedStartAction::Exit, -1, -1, -1, true);
        }
    }

    /// C++ `void CvPlayerAI::AI_recalculateFoundValues(int iX, int iY, int iInnerRadius, int iOuterRadius) const` (CvPlayerAI.cpp:18462-18516)
    /// (`&mut Game`: writes plot found values; see SIGNATURE NOTES.)
    pub fn player_ai_recalculate_found_values(&self, g: &mut Game, p: PlayerId, x: i32, y: i32, inner_radius: i32, outer_radius: i32) {
        let team = g.players[p].team;
        for dx in -outer_radius..=outer_radius {
            for dy in -outer_radius..=outer_radius {
                let Some(loop_plot) = g.plot_xy(x, y, dx, dy) else {
                    continue;
                };
                if self.player_ai_is_plot_city_site(g, p, loop_plot) {
                    continue;
                }
                if g.step_distance(0, 0, dx, dy) <= inner_radius {
                    if !(dx == 0 && dy == 0) {
                        g.map.plots[loop_plot].found_value[p] = 0;
                    }
                } else if g.map.plots[loop_plot].revealed[team] {
                    // (The `getCityFoundValue` Python callback is off in FFH2 and returns -1.)
                    let (lx, ly) = g.map.xy(loop_plot);
                    let value = self.player_ai_found_value(g, p, lx, ly, -1, false);

                    // C++ `m_aiFoundValue` is a `short` array.
                    g.map.plots[loop_plot].found_value[p] = value as i16 as i32;

                    if let Some(a) = g.map.plots[loop_plot].area {
                        if value > g.map.areas[a].best_found_value[p] {
                            g.map.areas[a].best_found_value[p] = value;
                        }
                    }
                }
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_getMinFoundValue() const` (CvPlayerAI.cpp:18519-18545)
    pub fn player_ai_get_min_found_value(&self, g: &Game, p: PlayerId) -> i32 {
        // ffh-core ports the same C++ (with the UNOFFICIAL_PATCH expense fix).
        g.player_ai_min_found_value(p)
    }

    /// C++ `void CvPlayerAI::AI_updateCitySites(int iMinFoundValueThreshold, int iMaxSites) const` (CvPlayerAI.cpp:18547-18596)
    /// (`&mut Game`: recalculates found values; see SIGNATURE NOTES.)
    pub fn player_ai_update_city_sites(&self, g: &mut Game, p: PlayerId, min_found_value_threshold: i32, max_sites: i32) {
        let team = g.players[p].team;
        let mut pass = 0;
        while pass < max_sites {
            //Add a city to the list.
            let mut best_found_value = 0;
            let mut best_found_plot = None;

            for i in 0..g.map.plots.len() {
                let loop_plot = PlotIdx(i as u32);
                if g.map.plots[loop_plot].revealed[team] {
                    let mut value = g.plot_get_found_value(loop_plot, p);
                    if value > min_found_value_threshold {
                        if !self.player_ai_is_plot_city_site(g, p, loop_plot) {
                            //FfH: Modified by Kael 07/13/2008
                            //	iValue *= std::min(NUM_CITY_PLOTS * 2, pLoopPlot->area()->getNumUnownedTiles());
                            let unowned = g.map.plots[loop_plot].area.map_or(0, |a| g.area_get_num_unowned_tiles(a));
                            value *= 42.min(unowned);
                            //FfH: End Modify

                            if value > best_found_value {
                                best_found_value = value;
                                best_found_plot = Some(loop_plot);
                            }
                        }
                    }
                }
            }
            if let Some(best) = best_found_plot {
                self.s().player(p).ai_city_sites.push(best);
                let (bx, by) = g.map.xy(best);
                self.player_ai_recalculate_found_values(g, p, bx, by, CITY_PLOTS_RADIUS, 2 * CITY_PLOTS_RADIUS);
            } else {
                break;
            }
            pass += 1;
        }
    }

    /// C++ `void CvPlayerAI::AI_invalidateCitySites(int iMinFoundValueThreshold) const` (CvPlayerAI.cpp:18598-18601)
    pub fn player_ai_invalidate_city_sites(&self, g: &Game, p: PlayerId, min_found_value_threshold: i32) {
        self.s().player(p).ai_city_sites.clear();
    }

    /// C++ `int CvPlayerAI::AI_getNumCitySites() const` (CvPlayerAI.cpp:18603-18607)
    pub fn player_ai_get_num_city_sites(&self, g: &Game, p: PlayerId) -> i32 {
        self.s().player(p).ai_city_sites.len() as i32
    }

    /// C++ `bool CvPlayerAI::AI_isPlotCitySite(CvPlot* pPlot) const` (CvPlayerAI.cpp:18609-18623)
    pub fn player_ai_is_plot_city_site(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
        self.s().player(p).ai_city_sites.contains(&plot)
    }

    /// C++ `int CvPlayerAI::AI_getNumAreaCitySites(int iAreaID, int& iBestValue) const` (CvPlayerAI.cpp:18625-18641)
    pub fn player_ai_get_num_area_city_sites(&self, g: &Game, p: PlayerId, area_id: i32, best_value: &mut i32) -> i32 {
        let sites = self.s().player(p).ai_city_sites.clone();
        let mut count = 0;
        *best_value = 0;

        for site in sites {
            if opt_to_i32(g.map.plots[site].area) == area_id {
                count += 1;
                *best_value = (*best_value).max(self.plot_found_value(g, site, p));
            }
        }
        count
    }

    /// C++ `int CvPlayerAI::AI_getNumAdjacentAreaCitySites(int iWaterAreaID, int iExcludeArea, int& iBestValue) const` (CvPlayerAI.cpp:18643-18664)
    pub fn player_ai_get_num_adjacent_area_city_sites(&self, g: &Game, p: PlayerId, water_area_id: i32, exclude_area: i32, best_value: &mut i32) -> i32 {
        let sites = self.s().player(p).ai_city_sites.clone();
        let water_area = opt_from_i32::<AreaId>(water_area_id);
        let mut count = 0;
        *best_value = 0;

        for site in sites {
            if opt_to_i32(g.map.plots[site].area) != exclude_area {
                if water_area.map_or(false, |a| g.plot_is_adjacent_to_area(site, a)) {
                    count += 1;
                    *best_value = (*best_value).max(self.plot_found_value(g, site, p));
                }
            }
        }
        count
    }

    /// C++ `CvPlot* CvPlayerAI::AI_getCitySite(int iIndex) const` (CvPlayerAI.cpp:18666-18670)
    pub fn player_ai_get_city_site(&self, g: &Game, p: PlayerId, index: i32) -> Option<PlotIdx> {
        if index < 0 {
            return None;
        }
        self.s().player(p).ai_city_sites.get(index as usize).copied()
    }

    /// C++ `int CvPlayerAI::AI_bestAreaUnitAIValue(UnitAITypes eUnitAI, CvArea* pArea, UnitTypes* peBestUnitType = NULL) const` (CvPlayerAI.cpp:18672-18725)
    pub fn player_ai_best_area_unit_ai_value(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, area: AreaId, best_unit_type: &mut Option<UnitType>) -> i32 {
        self.best_area_unit_ai_value_opt(g, p, unit_ai, Some(area), best_unit_type)
    }

    /// C++ `int CvPlayerAI::AI_bestCityUnitAIValue(UnitAITypes eUnitAI, CvCity* pCity, UnitTypes* peBestUnitType = NULL) const` (CvPlayerAI.cpp:18727-18763)
    pub fn player_ai_best_city_unit_ai_value(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, city: CityId, best_unit_type: &mut Option<UnitType>) -> i32 {
        self.best_city_unit_ai_value_opt(g, p, unit_ai, Some(city), best_unit_type)
    }

    /// C++ `int CvPlayerAI::AI_calculateTotalBombard(DomainTypes eDomain) const` (CvPlayerAI.cpp:18765-18794)
    pub fn player_ai_calculate_total_bombard(&self, g: &Game, p: PlayerId, domain: DomainType) -> i32 {
        let d = g.d;
        let mut total_bombard = 0;

        for class in d.unit_classes.keys() {
            let Some(loop_unit) = civ_unit(g, p, class) else {
                continue;
            };
            let info = &d.units[loop_unit];
            if info.domain_type == Some(domain) {
                let class_count = g.players[p].unit_class_count[class];
                let bombard_rate = info.bombard_rate;
                if bombard_rate > 0 {
                    total_bombard += bombard_rate * class_count;
                }

                let bomb_rate = info.bomb_rate;
                if bomb_rate > 0 {
                    total_bombard += bomb_rate * class_count;
                }
            }
        }

        total_bombard
    }

    /// C++ `void CvPlayerAI::AI_updateBonusValue(BonusTypes eBonus)` (CvPlayerAI.cpp:18796-18802)
    pub fn player_ai_update_bonus_value(&self, g: &mut Game, p: PlayerId, bonus: BonusType) {
        //reset
        g.players[p].ai_bonus_value[bonus.idx()].set(-1);
    }

    /// C++ `void CvPlayerAI::AI_updateBonusValue()` (CvPlayerAI.cpp:18805-18815)
    pub fn player_ai_update_bonus_value_2(&self, g: &mut Game, p: PlayerId) {
        // ffh-core resets every bonus (the same loop).
        g.player_ai_update_bonus_value(p);
    }

    /// C++ `int CvPlayerAI::AI_getUnitClassWeight(UnitClassTypes eUnitClass) const` (CvPlayerAI.cpp:18817-18820)
    pub fn player_ai_get_unit_class_weight(&self, g: &Game, p: PlayerId, unit_class: UnitClassType) -> i32 {
        self.s().player(p).unit_class_weights[unit_class.idx()] / 100
    }

    /// C++ `int CvPlayerAI::AI_getUnitCombatWeight(UnitCombatTypes eUnitCombat) const` (CvPlayerAI.cpp:18822-18825)
    pub fn player_ai_get_unit_combat_weight(&self, g: &Game, p: PlayerId, unit_combat: UnitCombatType) -> i32 {
        self.s().player(p).unit_combat_weights[unit_combat.idx()] / 100
    }

    /// C++ `void CvPlayerAI::AI_doEnemyUnitData()` (CvPlayerAI.cpp:18827-18970)
    pub fn player_ai_do_enemy_unit_data(&self, g: &mut Game, p: PlayerId) {
        let d = g.d;
        let team = g.players[p].team;
        let mut unit_counts: IdxVec<UnitType, i32> = IdxVec::from_elem(0, d.units.len());
        let mut domain_sums = [0i32; DomainType::NUM];

        let mut new_total = 0;

        let mut unit_class_weights = self.s().player(p).unit_class_weights.clone();

        for i in 0..g.map.plots.len() {
            let loop_plot = PlotIdx(i as u32);
            let mut adjacent_attackers = -1;
            if g.plot_is_visible(loop_plot, team) {
                for u in g.map.plots[loop_plot].units.clone() {
                    if !g.unit_can_fight(u) {
                        continue;
                    }
                    let mut unit_value = 1;
                    let unit_team = g.unit_get_team(u);
                    if g.at_war(team, unit_team) {
                        unit_value += 10;

                        if g.map.plots[loop_plot].owner == Some(p) {
                            unit_value += 15;
                        } else if g.plot_get_team(loop_plot).map_or(false, |t| g.at_war(team, t)) {
                            if adjacent_attackers == -1 {
                                let owner = g.map.plots[loop_plot].owner.expect("owned plot");
                                adjacent_attackers = self.player_ai_adjacent_potential_attackers(g, owner, loop_plot, false);
                            }
                            if adjacent_attackers > 0 {
                                unit_value += 15;
                            }
                        }
                    } else if g.units[u].owner != p {
                        let can_attack = g.unit_can_attack(u);
                        unit_value += if can_attack { 4 } else { 1 };
                        if g.map.plots[loop_plot].culture[p] > 0 {
                            unit_value += if can_attack { 4 } else { 1 };
                        }
                    }

                    if unit_class_weights[g.unit_get_unit_class_type(u).idx()] == 0 {
                        unit_value *= 4;
                    }

                    unit_value *= g.unit_base_combat_str(u);
                    unit_counts[g.units[u].unit_type] += unit_value;
                    domain_sums[g.unit_get_domain_type(u).idx()] += unit_value;
                    new_total += unit_value;
                }
            }
        }

        if new_total == 0 {
            //This should rarely happen.
            return;
        }

        //Decay
        for w in unit_class_weights.iter_mut() {
            *w -= 100;
            *w *= 3;
            *w /= 4;
            *w = 0.max(*w);
        }

        let current_era = g.players[p].current_era.i();
        for unit in d.units.keys() {
            if unit_counts[unit] > 0 {
                let info = &d.units[unit];
                let tech = info.prereq_and_tech;

                let mut era_diff = match tech {
                    None => 4,
                    Some(t) => 4.min(current_era - opt_to_i32(d.techs[t].era)),
                };

                if era_diff > 1 {
                    era_diff -= 1;
                    unit_counts[unit] *= 3 - era_diff;
                    unit_counts[unit] /= 3;
                }
                let domain_sum = info.domain_type.map_or(0, |dm| domain_sums[dm.idx()]);
                if let Some(class) = info.unit_class_type {
                    unit_class_weights[class.idx()] += (5000 * unit_counts[unit]) / 1.max(domain_sum);
                }
            }
        }

        let mut unit_combat_weights = vec![0i32; d.unit_combats.len()];

        for class in d.unit_classes.keys() {
            if unit_class_weights[class.idx()] > 0 {
                let Some(unit) = d.unit_classes[class].default_unit_index else {
                    continue;
                };

                //FfH: Modified by Kael 03/23/2009
                //			m_aiUnitCombatWeights[GC.getUnitInfo(eUnit).getUnitCombatType()] += m_aiUnitClassWeights[iI];
                if let Some(uc) = d.units[unit].unit_combat_type {
                    unit_combat_weights[uc.idx()] += unit_class_weights[class.idx()];
                }
                //FfH: End Modify
            }
        }

        for w in unit_combat_weights.iter_mut() {
            if *w > 25 {
                *w += 2500;
            } else if *w > 0 {
                *w += 1000;
            }
        }

        let mut s = self.s();
        let pa = s.player(p);
        pa.unit_class_weights = unit_class_weights;
        pa.unit_combat_weights = unit_combat_weights;
    }

    /// C++ `int CvPlayerAI::AI_calculateUnitAIViability(UnitAITypes eUnitAI, DomainTypes eDomain) const` (CvPlayerAI.cpp:18972-18998)
    pub fn player_ai_calculate_unit_ai_viability(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, domain: DomainType) -> i32 {
        let d = g.d;
        let team = g.players[p].team;
        let mut best_unit_ai_strength = 0;
        let mut best_other_strength = 0;

        let weights = self.s().player(p).unit_class_weights.clone();

        for class in d.unit_classes.keys() {
            // FIX: C++ reads `GC.getUnitInfo((UnitTypes)iI)` with the unit class index `iI` and
            // never uses `eLoopUnit`; the class's default unit is clearly what was meant.
            let Some(loop_unit) = d.unit_classes[class].default_unit_index else {
                continue;
            };
            let info = &d.units[loop_unit];
            if info.domain_type == Some(domain) {
                let tech = info.prereq_and_tech;
                if weights[class.idx()] > 0 || g.team_is_has_tech(team, tech) {
                    if info.unit_ai_type[unit_ai] {
                        best_unit_ai_strength = best_unit_ai_strength.max(info.combat);
                    }

                    best_other_strength = best_other_strength.max(info.combat);
                }
            }
        }

        (100 * best_unit_ai_strength) / 1.max(best_other_strength)
    }

    /// C++ `ReligionTypes CvPlayerAI::AI_chooseReligion()` (CvPlayerAI.cpp:19000-19023)
    pub fn player_ai_choose_religion(&self, g: &Game, p: PlayerId) -> Option<ReligionType> {
        let d = g.d;
        let favorite = g.player_get_leader_type(p).and_then(|l| d.leader_heads[l].favorite_religion);
        if let Some(fav) = favorite {
            if !g.is_religion_founded(fav) {
                return Some(fav);
            }
        }

        let religions: Vec<ReligionType> = d.religions.keys().filter(|&r| !g.is_religion_founded(r)).collect();

        if !religions.is_empty() {
            return Some(religions[self.rand(religions.len() as i32, "AI pick religion") as usize]);
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeWeight(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19025-19048)
    pub fn player_ai_get_attitude_weight(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let attitude = self.player_ai_get_attitude(g, p, player, true);
        if attitude == AttitudeType::ATTITUDE_FURIOUS {
            -100
        } else if attitude == AttitudeType::ATTITUDE_ANNOYED {
            -50
        } else if attitude == AttitudeType::ATTITUDE_CAUTIOUS {
            0
        } else if attitude == AttitudeType::ATTITUDE_PLEASED {
            50
        } else if attitude == AttitudeType::ATTITUDE_FRIENDLY {
            100
        } else {
            0
        }
    }

    /// C++ `int CvPlayerAI::AI_getPlotAirbaseValue(CvPlot* pPlot) const` (CvPlayerAI.cpp:19050-19172)
    pub fn player_ai_get_plot_airbase_value(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        let d = g.d;
        let team = g.players[p].team;

        if g.plot_get_team(plot) != Some(team) {
            return 0;
        }

        if g.plot_is_city_radius(plot) {
            if let Some(working_city) = g.map.plots[plot].working_city.filter(|&c| g.cities.contains(c)) {
                if self.working_city_best_build(g, working_city, plot).is_some() {
                    return 0;
                }
                if let Some(imp) = g.map.plots[plot].improvement_type {
                    if !d.improvements[imp].acts_as_city {
                        return 0;
                    }
                }
            }
        }

        let mut min_other_city_distance = MAX_INT;
        let mut min_other_city_plot: Option<PlotIdx> = None;

        let mut min_friendly_city_distance = MAX_INT;
        let mut min_friendly_city_plot: Option<PlotIdx> = None;

        let mut other_city_count = 0;

        let (px, py) = g.map.xy(plot);
        let range = 4;
        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                    continue;
                };
                if loop_plot == plot {
                    continue;
                }
                let (lx, ly) = g.map.xy(loop_plot);
                let distance = g.plot_distance(px, py, lx, ly);

                if g.plot_get_team(loop_plot) == Some(team) {
                    if g.plot_is_city(loop_plot, true, None) {
                        if distance == 1 {
                            return 0;
                        }
                        if distance < min_friendly_city_distance {
                            min_friendly_city_distance = distance;
                            min_friendly_city_plot = Some(loop_plot);
                        }
                    }
                } else if g.plot_is_owned(loop_plot) {
                    if g.plot_is_city(loop_plot, false, None) {
                        if distance < min_other_city_distance {
                            min_other_city_distance = distance;
                            min_other_city_plot = Some(loop_plot);
                            other_city_count += 1;
                        }
                    }
                }
            }
        }

        if other_city_count == 0 {
            return 0;
        }

        // (Two commented-out C++ distance checks omitted.)

        let defense_modifier = g.plot_defense_modifier(plot, Some(team), false);

        let mut value = other_city_count * 50;
        value *= 100 + (2 * (defense_modifier + if g.plot_is_hills(plot) { 25 } else { 0 }));
        value /= 100;

        value
    }

    /// C++ `int CvPlayerAI::AI_getPlotCanalValue(CvPlot* pPlot) const` (CvPlayerAI.cpp:19174-19226)
    pub fn player_ai_get_plot_canal_value(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        let d = g.d;
        let team = g.players[p].team;

        if g.plot_is_owned(plot) {
            if g.plot_get_team(plot) != Some(team) {
                return 0;
            }
            if g.plot_is_city_radius(plot) {
                if let Some(working_city) = g.map.plots[plot].working_city.filter(|&c| g.cities.contains(c)) {
                    if self.working_city_best_build(g, working_city, plot).is_some() {
                        return 0;
                    }
                    if let Some(imp) = g.map.plots[plot].improvement_type {
                        if !d.improvements[imp].acts_as_city {
                            return 0;
                        }
                    }
                }
            }
        }

        let (px, py) = g.map.xy(plot);
        for i in 0..DirectionType::NUM {
            if let Some(loop_plot) = g.plot_direction(px, py, DirectionType(i as u16)) {
                if g.plot_is_city(loop_plot, true, None) {
                    return 0;
                }
            }
        }

        let Some(second_water_area) = g.plot_second_water_area(plot) else {
            return 0;
        };

        // C++ quirk: min(0, ..) makes the value never positive.
        10 * 0.min(g.map.areas[second_water_area].num_tiles - 2)
    }

    /// C++ `int CvPlayerAI::AI_getHappinessWeight(int iHappy, int iExtraPop) const` (CvPlayerAI.cpp:19231-19273)
    pub fn player_ai_get_happiness_weight(&self, g: &Game, p: PlayerId, happy: i32, extra_pop: i32) -> i32 {
        //This returns a positive number equal approximately to the sum
        //of the percentage values of each unit (there is no need to scale the output by iHappy)
        //100 * iHappy means a high value.
        let mut happy = happy;
        let mut count = 0;

        if happy == 0 {
            happy = 1;
        }
        let mut value = 0;
        for &c in &g.players[p].cities {
            let mut city_happy = g.city_happy_level(c) - g.city_unhappy_level(c, extra_pop);

            city_happy -= 0.max(g.city_commerce_happiness(c));
            let happy_now = city_happy;
            let happy_then = city_happy + happy;

            //Integration
            let temp_value = (100 * happy_then - 10 * happy_then * happy_then) - (100 * happy_now - 10 * happy_now * happy_now);
            if happy > 0 {
                value += 0.max(temp_value);
            } else {
                value += 0.max(-temp_value);
            }

            count += 1;
            if count > 6 {
                break;
            }
        }

        if count == 0 {
            50 * happy
        } else {
            value / count
        }
    }

    /// C++ `int CvPlayerAI::AI_getHealthWeight(int iHealth, int iExtraPop) const` (CvPlayerAI.cpp:19275-19315)
    pub fn player_ai_get_health_weight(&self, g: &Game, p: PlayerId, health: i32, extra_pop: i32) -> i32 {
        let mut health = health;
        let mut count = 0;

        if health == 0 {
            health = 1;
        }
        let mut value = 0;
        for &c in &g.players[p].cities {
            let city_health = g.city_good_health(c) - g.city_bad_health(c, false, extra_pop);

            let health_now = city_health;
            let health_then = city_health + health;

            //Integration
            let temp_value = (100 * health_then - 6 * health_then * health_then) - (100 * health_now - 6 * health_now * health_now);
            if health > 0 {
                value += 0.max(temp_value);
            } else {
                value += 0.max(-temp_value);
            }
            count += 1;
            if count > 6 {
                break;
            }
        }

        if count == 0 {
            50
        } else {
            value / count
        }
    }

    /// C++ `void CvPlayerAI::AI_invalidateCloseBordersAttitudeCache()` (CvPlayerAI.cpp:19317-19323)
    pub fn player_ai_invalidate_close_borders_attitude_cache(&self, g: &Game, p: PlayerId) {
        let n = g.players.len();
        let mut s = self.s();
        s.ensure_players(n);
        for v in s.player(p).close_borders_attitude_cache.iter_mut() {
            *v = MAX_INT;
        }
    }

    /// C++ `bool CvPlayerAI::AI_isPlotThreatened(CvPlot* pPlot, int iRange = -1, bool bTestMoves = true) const` (CvPlayerAI.cpp:19325-19375)
    pub fn player_ai_is_plot_threatened(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> bool {
        let team = g.players[p].team;
        let plot_area = g.map.plots[plot].area;

        let range = if range == -1 { DANGER_RANGE } else { range };

        let (px, py) = g.map.xy(plot);
        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                    continue;
                };
                if g.map.plots[loop_plot].area != plot_area {
                    continue;
                }
                for &u in &g.map.plots[loop_plot].units {
                    if g.unit_is_enemy(u, Some(team), None) && g.unit_can_attack(u) && !g.unit_is_invisible(u, team, true) {
                        if g.unit_can_move_or_attack_into(u, plot, false) {
                            let mut path_turns = 0;
                            if test_moves {
                                path_turns = match g.units[u].group {
                                    Some(grp) => g.group_generate_path(grp, loop_plot, plot, MOVE_MAX_MOVES | MOVE_IGNORE_DANGER, false).unwrap_or(MAX_INT),
                                    None => MAX_INT,
                                };
                            }

                            if path_turns <= 1 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvPlayerAI::AI_isFirstTech(TechTypes eTech) const` (CvPlayerAI.cpp:19377-19400)
    pub fn player_ai_is_first_tech(&self, g: &Game, p: PlayerId, tech: TechType) -> bool {
        let d = g.d;
        for religion in d.religions.keys() {
            if d.religions[religion].tech_prereq == Some(tech) {
                if !g.religion_slot_taken[religion] {
                    return true;
                }
            }
        }

        if g.count_known_tech_num_teams(tech) == 0 {
            if g.player_get_tech_free_unit(p, tech).is_some() || d.techs[tech].first_free_techs > 0 {
                return true;
            }
        }

        false
    }

    /// C++ `int CvPlayerAI::AI_getAlignmentAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19403-19452)
    pub fn player_ai_get_alignment_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        //FfH: Added by Kael 08/15/2007
        let defs = &g.d.defines;
        let theirs = g.players[player].alignment;
        let ours = g.players[p].alignment;
        let good = Some(AlignmentType::ALIGNMENT_GOOD);
        let neutral = Some(AlignmentType::ALIGNMENT_NEUTRAL);
        let evil = Some(AlignmentType::ALIGNMENT_EVIL);
        let mut attitude = 0;
        if theirs == good {
            if ours == good {
                attitude = defs.alignment_attitude_good_to_good;
            }
            if ours == neutral {
                attitude = defs.alignment_attitude_neutral_to_good;
            }
            if ours == evil {
                attitude = defs.alignment_attitude_evil_to_good - (g.get_global_counter() / 10);
            }
        }
        if theirs == neutral {
            if ours == good {
                attitude = defs.alignment_attitude_good_to_neutral;
            }
            if ours == neutral {
                attitude = defs.alignment_attitude_neutral_to_neutral;
            }
            if ours == evil {
                attitude = defs.alignment_attitude_evil_to_neutral;
            }
        }
        if theirs == evil {
            if ours == good {
                attitude = defs.alignment_attitude_good_to_evil - (g.get_global_counter() / 10);
            }
            if ours == neutral {
                attitude = defs.alignment_attitude_neutral_to_evil;
            }
            if ours == evil {
                attitude = defs.alignment_attitude_evil_to_evil;
            }
        }
        attitude
    }

    /// C++ `int CvPlayerAI::AI_getBadBonusAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19454-19465)
    pub fn player_ai_get_bad_bonus_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let d = g.d;
        let attitude_bad_bonus = g.players[p].personality_type.map_or(0, |l| d.leader_heads[l].attitude_bad_bonus);
        let mut attitude = 0;
        for bonus in d.bonuses.keys() {
            if g.player_has_bonus(player, bonus) {
                attitude += d.bonuses[bonus].bad_attitude * attitude_bad_bonus;
            }
        }
        attitude
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteWonderAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19467-19478)
    pub fn player_ai_get_favorite_wonder_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let d = g.d;
        let mut attitude = 0;
        if let Some(wonder) = g.players[p].personality_type.and_then(|l| d.leader_heads[l].favorite_wonder) {
            if g.player_count_num_buildings(player, wonder) > 0 {
                attitude += 4;
            }
        }
        attitude
    }

    /// C++ `int CvPlayerAI::AI_getGenderAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19480-19500)
    pub fn player_ai_get_gender_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let d = g.d;
        // C++ indexes both personalities unchecked; a missing one reads as no attitude.
        let (Some(ours), Some(theirs)) = (g.players[p].personality_type, g.players[player].personality_type) else {
            return 0;
        };
        let ours = &d.leader_heads[ours];
        let theirs = &d.leader_heads[theirs];
        let mut attitude = 0;
        if ours.female {
            attitude += theirs.attitude_from_females;
        }
        if !ours.female {
            attitude += theirs.attitude_from_males;
        }
        if theirs.female {
            attitude += ours.attitude_to_females;
        }
        if !theirs.female {
            attitude += ours.attitude_to_males;
        }
        attitude
    }

    /// C++ `int CvPlayerAI::AI_getTrustAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19502-19510)
    pub fn player_ai_get_trust_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let mut attitude = 0;
        if g.players[player].feat_accomplished[FeatType::FEAT_TRUST] {
            attitude += 4;
        }
        attitude
    }

    /// C++ `int CvPlayerAI::AI_trueCombatValue(UnitTypes eUnit) const` (CvPlayerAI.cpp:19512-19558)
    pub fn player_ai_true_combat_value(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        // (ffh-core's `ai_true_combat_value` ports the same C++ but is private to the rules.)
        let d = g.d;
        let info = &d.units[unit];

        let mut combat = if info.domain_type == Some(DomainType::DOMAIN_AIR) { info.air_combat } else { info.combat };

        for damage_type in d.damages.keys() {
            combat += info.damage_type_combat[damage_type];
        }

        for bonus in d.bonuses.keys() {
            combat += info.bonus_affinity[bonus] * g.player_get_num_available_bonuses(p, bonus);
        }

        if info.weapon_tier > 0 {
            let available = |bonus_define: i32| opt_from_i32::<BonusType>(bonus_define).map_or(0, |b| g.player_get_num_available_bonuses(p, b));

            let mut combat_bonus = 0;
            if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier1) {
                if available(d.defines.weapon_req_bonus_tier1) > 0 {
                    combat_bonus = d.promotions[promotion].extra_combat_str;
                }
            }
            if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier2) {
                if info.weapon_tier > 1 && available(d.defines.weapon_req_bonus_tier2) > 0 {
                    combat_bonus = d.promotions[promotion].extra_combat_str;
                }
            }
            if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier3) {
                if info.weapon_tier > 2 && available(d.defines.weapon_req_bonus_tier3) > 0 {
                    combat_bonus = d.promotions[promotion].extra_combat_str;
                }
            }
            combat += combat_bonus;
        }

        combat
    }

    /// C++ `int CvPlayerAI::AI_combatValue(UnitTypes eUnit) const` (CvPlayerAI.cpp:19560-19567)
    pub fn player_ai_combat_value(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        let d = g.d;
        let info = &d.units[unit];
        let mut value = 100 * self.player_ai_true_combat_value(g, p, unit);
        value *= (((info.first_strikes * 2) + info.chance_first_strikes) * (d.defines.combat_damage / 5)) + 100;
        value /= 100;
        value /= g.best_land_unit_combat();
        value
    }

    /// C++ `int CvPlayerAI::AI_getCivicShareAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19569-19580)
    pub fn player_ai_get_civic_share_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let d = g.d;
        let mut attitude = 0;
        for option in d.civic_options.keys() {
            let ours = g.players[p].civics[option];
            if ours == g.players[player].civics[option] {
                // C++ indexes NO_CIVIC unchecked; no civic shares nothing.
                if let Some(civic) = ours {
                    attitude += d.civics[civic].attitude_share_mod;
                }
            }
        }
        attitude
    }

    /// C++ `CvPlot* CvPlayerAI::AI_getExpansionPlot() const` (CvPlayerAI.cpp:19583-19593)
    pub fn player_ai_get_expansion_plot(&self, g: &Game, p: PlayerId) -> Option<PlotIdx> {
        let (x, y) = {
            let mut s = self.s();
            let pa = s.player(p);
            (pa.expansion_x, pa.expansion_y)
        };
        if x == -1 && y == -1 {
            None
        } else {
            g.plot_xy(x, y, 0, 0)
        }
    }

    /// C++ `void CvPlayerAI::AI_setExpansionPlot(CvPlot* newplot)` (CvPlayerAI.cpp:19595-19607)
    pub fn player_ai_set_expansion_plot(&self, g: &mut Game, p: PlayerId, newplot: PlotIdx) {
        self.set_expansion_plot_opt(g, p, Some(newplot));
    }

    /// C++ `void CvPlayerAI::AI_doExpansion()` (CvPlayerAI.cpp:19609-19674)
    pub fn player_ai_do_expansion(&self, g: &mut Game, p: PlayerId) {
        // Do not even have a Capital
        let Some(cap) = g.player_get_capital_city(p) else {
            self.set_expansion_plot_opt(g, p, None);
            return;
        };
        let (cap_x, cap_y) = (g.cities[cap].x, g.cities[cap].y);
        let cap_area = g.city_area(cap);

        let mut best_found_plot = None;
        let mut best_found_value = 0;

        let num_sites = self.player_ai_get_num_city_sites(g, p);
        for i in 0..num_sites {
            let Some(city_site_plot) = self.player_ai_get_city_site(g, p, i) else {
                continue;
            };
            //		if (canFound(pCitySitePlot))
            let mut value = g.plot_get_found_value(city_site_plot, p);
            let (sx, sy) = g.map.xy(city_site_plot);
            let dif_x = (sx - cap_x).abs();
            let dif_y = (sy - cap_y).abs();
            value /= dif_x + 10;
            value /= dif_y + 10;

            if g.map.plots[city_site_plot].area == Some(cap_area) {
                value += 10000;
            }

            if value > best_found_value {
                best_found_value = value;
                best_found_plot = Some(city_site_plot);
            }
        }
        if let Some(best) = best_found_plot {
            self.set_expansion_plot_opt(g, p, Some(best));
        }

        // (USE_DEBUG_LOG naval log output omitted.)
    }

    // ------------------------------------------------------------------ private helpers

    /// `m_iAveragesCacheTurn != getGameTurn()` -> `AI_calculateAverages()`.
    fn ensure_averages(&self, g: &Game, p: PlayerId) {
        let cache_turn = self.s().player(p).averages_cache_turn;
        if cache_turn != g.game_turn() {
            self.player_ai_calculate_averages(g, p);
        }
    }

    /// `AI_setExpansionPlot(newplot)` with a possibly NULL plot.
    fn set_expansion_plot_opt(&self, g: &Game, p: PlayerId, newplot: Option<PlotIdx>) {
        let (x, y) = match newplot {
            None => (-1, -1),
            Some(pl) => g.map.xy(pl),
        };
        let mut s = self.s();
        let pa = s.player(p);
        pa.expansion_x = x;
        pa.expansion_y = y;
    }

    /// `pWorkingCity->AI_getBestBuild(pWorkingCity->getCityPlotIndex(pPlot))`.
    /// (C++ reads index -1 for a plot outside the city's radius; here that is NO_BUILD.)
    fn working_city_best_build(&self, g: &Game, city: CityId, plot: PlotIdx) -> Option<BuildType> {
        let index = g.city_plot_index(city, plot)?;
        self.city_ai_get_best_build(g, city, index as i32)
    }

    /// C++ `AI_unitValue(eUnit, eUnitAI, pArea)` with a possibly NULL `pArea`.
    fn unit_value_opt_area(&self, g: &Game, p: PlayerId, unit: UnitType, unit_ai: UnitAIType, area: Option<AreaId>) -> i32 {
        match area {
            Some(a) => self.player_ai_unit_value(g, p, unit, unit_ai, a),
            None => {
                // PORT ADAPT: C++ passes NULL (AI_unitValue then skips its area tests);
                // `player_ai_unit_value` takes an AreaId (SIGNATURE NOTE), so the capital's
                // area, else the starting plot's, stands in.
                let stand_in = g
                    .player_get_capital_city(p)
                    .map(|c| g.city_area(c))
                    .or_else(|| g.player_get_starting_plot(p).and_then(|s| g.map.plots[s].area))
                    .or_else(|| g.map.plots.first().and_then(|pl| pl.area));
                match stand_in {
                    Some(a) => self.player_ai_unit_value(g, p, unit, unit_ai, a),
                    None => 0,
                }
            }
        }
    }

    /// Body of `AI_bestAreaUnitAIValue` (C++ `pArea` may be NULL).
    fn best_area_unit_ai_value_opt(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, area: Option<AreaId>, best_unit_type: &mut Option<UnitType>) -> i32 {
        let mut city = None;

        if let Some(area) = area {
            let water = g.map.areas[area].water;
            if let Some(cap) = g.player_get_capital_city(p) {
                if water {
                    if g.plot_is_adjacent_to_area(g.city_plot(cap), area) {
                        city = Some(cap);
                    }
                } else if g.city_area(cap) == area {
                    city = Some(cap);
                }
            }

            if city.is_none() {
                for &c in &g.players[p].cities {
                    if water {
                        if g.plot_is_adjacent_to_area(g.city_plot(c), area) {
                            city = Some(c);
                            break;
                        }
                    } else if g.city_area(c) == area {
                        city = Some(c);
                        break;
                    }
                }
            }
        }

        self.best_city_unit_ai_value_opt(g, p, unit_ai, city, best_unit_type)
    }

    /// Body of `AI_bestCityUnitAIValue` (C++ `pCity` may be NULL).
    fn best_city_unit_ai_value_opt(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, city: Option<CityId>, best_unit_type: &mut Option<UnitType>) -> i32 {
        let d = g.d;
        let mut best_value = 0;
        let human = g.players[p].human;

        for class in d.unit_classes.keys() {
            let Some(loop_unit) = civ_unit(g, p, class) else {
                continue;
            };
            if !human || d.units[loop_unit].default_unit_ai_type == Some(unit_ai) {
                let can_train = match city {
                    None => g.player_can_train(p, loop_unit, false, false, false),
                    Some(c) => g.city_can_train(c, loop_unit, false, false, false, false),
                };
                if can_train {
                    let value = self.unit_value_opt_area(g, p, loop_unit, unit_ai, city.map(|c| g.city_area(c)));
                    if value > best_value {
                        best_value = value;
                        *best_unit_type = Some(loop_unit);
                    }
                }
            }
        }

        best_value
    }
}

/// C++ `GC.getCivilizationInfo(getCivilizationType()).getCivilizationUnits(eClass)`.
fn civ_unit(g: &Game, p: PlayerId, class: UnitClassType) -> Option<UnitType> {
    let civ = g.player_get_civilization_type(p)?;
    g.d.civilizations[civ].civilization_units(class, g.d.unit_classes[class].default_unit_index)
}
