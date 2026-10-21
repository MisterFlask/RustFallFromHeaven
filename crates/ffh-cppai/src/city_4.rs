//! Port of CvCityAI.cpp:7887-10755 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `city_ai_count_num_bonuses` now takes `bonus: Option<BonusType>` (`None` = C++ NO_BONUS,
//!   which every caller in CvCityAI.cpp passes; CvPlayer.cpp:3674 passes `Some(bonus)`).
//! - `city_ai_set_city_patrol_origin` takes a `PlotIdx`; C++ callers passing NULL should call
//!   `city_ai_clear_city_patrol_origin` (added here).
//! - `city_ai_cache_player_closeness` / `city_ai_player_closeness` take `&Game` while ffh-core's
//!   `map_calculate_path_distance` takes `&mut Game`; this file uses a private `&Game` copy of it
//!   (the core one only needs the step finder's `RefCell`).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;
use ffh_core::plot::ConstPlotUnitFunc;

const FOOD: usize = 0;
const PROD: usize = 1;
const COMM: usize = 2;

/// C++ `getCultureLevel()` as the C++ `int` (NO_CULTURELEVEL = -1).
fn culture_level_i(g: &Game, c: CityId) -> i32 {
    opt_to_i32(g.cities[c].culture_level)
}

/// C++ `area()->getAreaAIType(getTeam())` of a city.
fn city_area_ai_type(g: &Game, c: CityId) -> Option<AreaAIType> {
    let team = g.city_team(c);
    g.map.areas[g.city_area(c)].area_ai_type[team]
}

/// C++ `GC.getLeaderHeadInfo(GET_PLAYER(p).getLeaderType())`.
fn player_leader_info(g: &Game, p: PlayerId) -> Option<&'static LeaderHeadInfo> {
    let d: &'static GameData = g.d;
    g.player_get_leader_type(p).map(|l| &d.leader_heads[l])
}

/// C++ `CvMap::calculatePathDistance` from `&Game` (ffh-core's `map_calculate_path_distance`
/// takes `&mut Game` but only uses the step finder's `RefCell`).
fn calculate_path_distance(g: &Game, source: PlotIdx, dest: PlotIdx) -> i32 {
    let (sx, sy) = g.map.xy(source);
    let (dx, dy) = g.map.xy(dest);
    let mut finder = g.step_finder.borrow_mut();
    if finder.generate_path(g, sx, sy, dx, dy, false, 0, true) {
        if let Some(node) = finder.last_node() {
            return node.data1;
        }
    }
    -1
}

impl Cai {
    /// C++ `void CvCityAI::AI_bestPlotBuild(CvPlot* pPlot, int* piBestValue, BuildTypes* peBestBuild, int iFoodPriority, int iProductionPriority, int iCommercePriority, bool bChop, int iHappyAdjust, int iHealthAdjust, int iFoodChange)` (CvCityAI.cpp:7888-8705)
    pub fn city_ai_best_plot_build(&self, g: &Game, c: CityId, plot: PlotIdx, best_value: &mut i32, best_build: &mut Option<BuildType>, food_priority: i32, production_priority: i32, commerce_priority: i32, chop: bool, happy_adjust: i32, health_adjust: i32, food_change: i32) {
        let d = g.d;
        let mut food_priority = food_priority;
        let mut production_priority = production_priority;
        let mut commerce_priority = commerce_priority;

        *best_value = 0;
        *best_build = None;

        if g.map.plots[plot].working_city != Some(c) {
            return;
        }

        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let civ = g.city_civilization_type(c);
        let pl = &g.map.plots[plot];
        let (px, py) = (pl.x, pl.y);
        let plot_improvement = pl.improvement_type;
        let plot_feature = pl.feature_type;
        let fcpp = d.defines.food_consumption_per_population;

        //When improving new plots only, count emphasis twice
        //helps to avoid too much tearing up of old improvements.
        if plot_improvement.is_none() {
            if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_FOOD) {
                food_priority *= 130;
                food_priority /= 100;
            }
            if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_PRODUCTION) {
                production_priority *= 180;
                production_priority /= 100;
            }
            if g.city_ai_is_emphasize_yield(c, YieldType::YIELD_COMMERCE) {
                commerce_priority *= 180;
                commerce_priority /= 100;
            }
        }

        let bonus = g.plot_get_bonus_type(plot, Some(team));
        let non_obsolete_bonus = g.plot_get_non_obsolete_bonus_type(plot, team);

        let mut has_bonus_improvement = false;
        if let Some(nob) = non_obsolete_bonus {
            if let Some(imp) = plot_improvement {
                if d.improvements[imp].improvement_bonus[nob].bonus_trade {
                    has_bonus_improvement = true;
                }
            }
        }

        let mut best_diff_yields = [0i32; YieldType::NUM];

        //If a worker is already building a build, force that Build.
        let mut forced_build: Option<BuildType> = None;
        for &u in &pl.units {
            if let Some(b) = g.unit_get_build_type(u) {
                if d.builds[b].improvement.is_some() {
                    forced_build = Some(b);
                    break;
                }
            }
        }

        let mut i_best_value = 0;
        let mut e_best_build: Option<BuildType> = None;

        let mut clear_feature_value = 0;
        if plot_feature.is_some() {
            let index = g.city_plot_index(c, plot).map_or(-1, |i| i as i32);
            clear_feature_value = self.city_ai_clear_feature_value(g, c, index);
        }

        let mut emphasize_irrigation = false;
        if !has_bonus_improvement {
            //It looks unwieldly but the code has to be rigid to avoid "worker ADD"
            //where they keep connecting then disconnecting a crops resource or building
            //multiple farms to connect a single crop resource.
            if pl.irrigated || (g.plot_is_fresh_water(plot) && g.plot_can_have_potential_irrigation(plot)) {
                for dir in 0..DirectionType::NUM {
                    let adj = match g.plot_direction(px, py, DirectionType::from_usize(dir)) {
                        Some(a) => a,
                        None => continue,
                    };
                    if g.map.plots[adj].owner == Some(owner) && g.plot_is_city_radius(adj) {
                        if !g.plot_is_fresh_water(adj) {
                            //check for a city? cities can conduct irrigation and that effect is quite
                            //useful... so I think irrigate cities.
                            if g.plot_is_potential_irrigation(adj) {
                                let mut best_irrigation_plot: Option<PlotIdx> = None;
                                let (ax, ay) = g.map.xy(adj);
                                for dir2 in 0..DirectionType::NUM {
                                    let adj2 = match g.plot_direction(ax, ay, DirectionType::from_usize(dir2)) {
                                        Some(a) => a,
                                        None => continue,
                                    };
                                    if g.map.plots[adj2].owner == Some(owner) {
                                        let temp_bonus = g.plot_get_non_obsolete_bonus_type(adj2, team);
                                        if g.map.plots[adj].irrigated {
                                            //the irrigation has to be coming from somewhere
                                            if g.map.plots[adj2].irrigated {
                                                //if we find a tile which is already carrying irrigation
                                                //then lets not replace that one...
                                                best_irrigation_plot = Some(adj2);

                                                if g.plot_is_city(adj2, false, None) || temp_bonus.is_some() || !g.plot_is_city_radius(adj2) {
                                                    if g.plot_is_fresh_water(adj2) {
                                                        //these are all ideal for irrigation chains so stop looking.
                                                        break;
                                                    }
                                                }
                                            }
                                        } else if g.plot_get_non_obsolete_bonus_type(adj2, team).is_none() {
                                            if g.plot_can_have_potential_irrigation(adj2) && g.plot_is_irrigation_available(adj2, false) {
                                                //could use more sophisticated logic
                                                best_irrigation_plot = Some(adj2);
                                                break;
                                            }
                                        }
                                    }
                                }

                                if best_irrigation_plot == Some(plot) {
                                    emphasize_irrigation = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        let leave_forests = g.players[owner].options[PlayerOptionType::PLAYEROPTION_LEAVE_FORESTS];
        let maintains = |feat: FeatureType| d.civilizations[civ].maintain_features[feat];

        for improvement in d.improvements.keys() {
            let mut best_temp_build_value = 0;
            let mut best_temp_build: Option<BuildType> = None;

            let mut ignore_feature = false;
            let mut valid = false;

            if Some(improvement) == plot_improvement {
                valid = true;
            } else {
                if let Some(fb) = forced_build {
                    if d.builds[fb].improvement == Some(improvement) {
                        best_temp_build = Some(fb);
                    }
                } else {
                    for build in d.builds.keys() {
                        if d.builds[build].improvement == Some(improvement) {
                            if g.player_can_build(owner, plot, build, false, false) {
                                let mut value = 10000;
                                value /= d.builds[build].time + 1;

                                // XXX feature production???

                                if value > best_temp_build_value {
                                    best_temp_build_value = value;
                                    best_temp_build = Some(build);
                                }
                            }
                        }
                    }
                }

                if let Some(bt) = best_temp_build {
                    valid = true;

                    if let Some(feat) = plot_feature {
                        //FfH: Added by Kael 04/24/2008 (maintain features)
                        if d.builds[bt].feature_remove[feat] && !maintains(feat) {
                            ignore_feature = true;

                            if d.features[feat].yield_change[PROD] > 0 {
                                if non_obsolete_bonus.is_none() {
                                    if leave_forests {
                                        valid = false;
                                    } else if g.city_health_rate(c, false, 0) < 0 && d.features[feat].health_percent > 0 {
                                        valid = false;
                                    } else if g.players[owner].feature_happiness[feat] > 0 {
                                        valid = false;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !valid {
                continue;
            }

            let final_improvement = final_improvement_upgrade(d, improvement, 0).unwrap_or(improvement);
            let fi = &d.improvements[final_improvement];

            let mut value = 0;

            if bonus.is_some() {
                if let Some(nob) = non_obsolete_bonus {
                    if fi.improvement_bonus[nob].bonus_trade {
                        value += self.player_ai_bonus_val(g, owner, nob, 1) * 10;
                        value += 200;
                        if let Some(bb) = e_best_build {
                            if d.builds[bb].improvement.map_or(true, |bi| !d.improvements[bi].improvement_bonus[nob].bonus_trade) {
                                //Always prefer improvements which connect bonuses.
                                e_best_build = None;
                                i_best_value = 0;
                            }
                        }
                    } else if let Some(bb) = e_best_build {
                        if d.builds[bb].improvement.map_or(false, |bi| d.improvements[bi].improvement_bonus[nob].bonus_trade) {
                            value -= 1000;
                        }
                    }
                }
            } else {
                for b in d.bonuses.keys() {
                    if fi.improvement_bonus[b].discover_rand > 0 {
                        value += 1;
                    }
                }
                // Xienwolf Tweak 03/18/09 (BETTER AI: workers value Resources more):
                // big bonus for giving us a resource where none existed previously
                if let Some(convert) = fi.bonus_convert {
                    let temp = self.player_ai_bonus_val(g, owner, convert, 1);
                    value += temp * temp;
                }
            }

            if value >= 0 {
                value *= 2;
                let mut final_yields = [0i32; YieldType::NUM];
                let mut diff_yields = [0i32; YieldType::NUM];
                for j in 0..YieldType::NUM {
                    let y = YieldType::from_usize(j);
                    // BETTER_BTS_AI_MOD 10/06/09 jdog5000 (City AI)
                    final_yields[j] = 2 * g.plot_calculate_nature_yield(plot, y, Some(team), ignore_feature);
                    final_yields[j] += g.plot_calculate_improvement_yield_change(plot, final_improvement, y, Some(owner), false, true);
                    final_yields[j] += g.plot_calculate_improvement_yield_change(plot, improvement, y, Some(owner), false, true);
                    if ignore_feature {
                        if let Some(feat) = plot_feature {
                            final_yields[j] -= 2 * d.features[feat].yield_change[j];
                        }
                    }

                    let mut cur_yield = 2 * g.plot_calculate_nature_yield(plot, y, Some(team), false);

                    if let Some(cur_improvement) = plot_improvement {
                        let cur_final_improvement = final_improvement_upgrade(d, cur_improvement, 0).unwrap_or(cur_improvement);
                        cur_yield += g.plot_calculate_improvement_yield_change(plot, cur_final_improvement, y, Some(owner), false, true);
                        cur_yield += g.plot_calculate_improvement_yield_change(plot, cur_improvement, y, Some(owner), false, true);
                    }

                    diff_yields[j] = final_yields[j] - cur_yield;
                }

                value += diff_yields[FOOD] * ((100 * food_priority) / 100);
                value += diff_yields[PROD] * ((60 * production_priority) / 100);
                value += diff_yields[COMM] * ((40 * commerce_priority) / 100);

                value /= 2;

                for j in 0..YieldType::NUM {
                    final_yields[j] /= 2;
                    diff_yields[j] /= 2;
                }

                if value > 0 {
                    // this is mainly to make it improve better tiles first
                    //flood plain > grassland > plain > tundra
                    value += final_yields[FOOD] * 10;
                    value += final_yields[PROD] * 6;
                    value += final_yields[COMM] * 4;

                    if final_yields[FOOD] >= fcpp {
                        //this is a food yielding tile
                        if food_priority > 100 {
                            value *= 100 + food_priority;
                            value /= 200;
                        }
                        if food_change > 0 {
                            value += (10 * (1 + diff_yields[FOOD]) * (1 + final_yields[FOOD] - fcpp) * food_change * food_priority) / 100;
                        }
                        if commerce_priority > 100 {
                            value *= 100 + (((commerce_priority - 100) * diff_yields[COMM]) / 2);
                            value /= 100;
                        }
                    } else if final_yields[FOOD] < fcpp {
                        if diff_yields[PROD] > 0 && final_yields[FOOD] + final_yields[PROD] > 3 {
                            if food_priority < 100 || g.players[owner].current_era.i() < 2 {
                                //value booster for mines on hills
                                value *= 100 + 25 * diff_yields[PROD];
                                value /= 100;
                            }
                        }
                        if food_change < 0 {
                            value *= 4 - food_change;
                            // PORT ADAPT: guard (C++ divides unguarded; 3 + food is never 0 in practice).
                            let divisor = 3 + final_yields[FOOD];
                            if divisor != 0 {
                                value /= divisor;
                            }
                        }
                    }

                    if food_priority < 100 && production_priority > 100 {
                        value *= 200 + production_priority;
                        value /= 200;
                    }
                    if bonus.is_none() {
                        if food_change > 0 {
                            //We want more food.
                            value *= 2 + diff_yields[FOOD].max(0);
                            value /= 2 * (1 + (-diff_yields[FOOD]).max(0));
                        }
                    }
                }

                if emphasize_irrigation && fi.carries_irrigation {
                    value += 500;
                }

                if g.cities[c].improvement_free_specialists[final_improvement] > 0 {
                    value += 2000;
                }

                let happiness = fi.happiness;
                if happiness != 0 && !(g.players[owner].advanced_start_points >= 0) {
                    let mut happy_level = happy_adjust + (g.city_happy_level(c) - g.city_unhappy_level(c, 0));
                    if Some(improvement) == plot_improvement {
                        happy_level -= happiness;
                    }
                    let health_level = g.city_good_health(c) - g.city_bad_health(c, false, 0);

                    let mut happy_value = 0;
                    if happy_level <= 0 {
                        happy_value += 400;
                    }
                    let can_grow = true; // (getYieldRate(YIELD_FOOD) > foodConsumption());

                    if happy_level <= health_level {
                        happy_value += 200 * 0.max((if can_grow { 6.min(2 + health_level - happy_level) } else { 0 }) - happy_level);
                    } else {
                        happy_value += 200 * 0.max((if can_grow { 1 } else { 0 }) - happy_level);
                    }
                    if !g.plot_is_being_worked(plot) {
                        happy_value *= 4;
                        happy_value /= 3;
                    }
                    happy_value += 0.max(pl.city_radius_count - 1) * (if happy_value > 0 { happy_level / 2 } else { 200 });
                    value += happy_value * happiness;
                }

                //FfH: Added by Kael 05/12/2008
                if g.plot_get_range_defense(plot, team, 0, true, false) > 0 {
                    value += fi.range * fi.range_defense_modifier * 50;
                } else if let Some(bt) = best_temp_build {
                    if g.plot_get_range_defense(plot, team, 5, true, true) == 0 {
                        if !g.plot_is_building(plot, bt, team, 5, true) {
                            value += fi.range * fi.range_defense_modifier * 100;
                        }
                    }
                }
                //FfH: End Add

                if !g.city_is_human(c) {
                    value *= 0.max(d.leader_heads[g.city_personality_type(c)].improvement_weight_modifier[final_improvement] + 200);
                    value /= 200;
                }

                match plot_improvement {
                    None => {
                        if g.plot_is_being_worked(plot) {
                            value *= 5;
                            value /= 4;
                        }

                        if let Some(bt) = best_temp_build {
                            if let Some(feat) = plot_feature {
                                //FfH: Added by Kael 04/24/2008 (maintain features)
                                if d.builds[bt].feature_remove[feat] && !maintains(feat) {
                                    value += g.plot_get_feature_production(plot, bt, team).0 * 2;
                                    value += clear_feature_value;
                                }
                            }
                        }
                    }
                    Some(cur_improvement) => {
                        // cottage/villages (don't want to chop them up if turns have been invested)
                        let mut downgrade = d.improvements[cur_improvement].improvement_pillage;
                        // PORT ADAPT: bounded walk (C++ loops forever on a pillage cycle).
                        let mut steps = 0;
                        while let Some(dg) = downgrade {
                            if steps > d.improvements.len() {
                                break;
                            }
                            steps += 1;
                            value -= d.improvements[dg].upgrade_time * 8;
                            downgrade = d.improvements[dg].improvement_pillage;
                        }

                        if d.improvements[cur_improvement].improvement_upgrade.is_some() {
                            value -= (d.improvements[cur_improvement].upgrade_time * 8 * pl.upgrade_progress) / 1.max(g.improvement_upgrade_time(cur_improvement));
                        }

                        if non_obsolete_bonus.is_none() {
                            if g.city_is_working_plot(c, plot) {
                                if (food_priority < 100 && final_yields[FOOD] >= fcpp) || d.improvements[cur_improvement].improvement_pillage.is_some() {
                                    value -= 70;
                                    value *= 2;
                                    value /= 3;
                                }
                            }
                        }

                        if g.players[owner].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION] {
                            value /= 4; //Greatly prefer builds which are legal.
                        }

                        //FfH: Added by Kael 02/24/2009 dont build over recently built improvmenets
                        // C++ quirk: the bonus goes to replacing the improvement (doubled when it
                        // is recent), the opposite of what the comment says.
                        if Some(improvement) != plot_improvement {
                            value += value / 4;
                            if pl.improvement_duration < 20 {
                                value *= 2;
                            }
                        }
                        //FfH: End Add
                    }
                }

                if value > i_best_value {
                    i_best_value = value;
                    e_best_build = best_temp_build;

                    best_diff_yields = diff_yields;
                }
            }
        }

        if clear_feature_value > 0 {
            if let Some(feat) = plot_feature {
                let fy = &d.features[feat].yield_change;
                if d.features[feat].health_percent < 0 || (fy[FOOD] + fy[PROD] + fy[COMM]) < 0 {
                    for build in d.builds.keys() {
                        if d.builds[build].improvement.is_none() {
                            //FfH: Added by Kael 04/24/2008 (maintain features)
                            if d.builds[build].feature_remove[feat] && !maintains(feat) {
                                if g.player_can_build(owner, plot, build, false, false) {
                                    let mut value = clear_feature_value;
                                    value += g.plot_get_feature_production(plot, build, team).0 * 10;

                                    value *= 400;
                                    value /= 1.max(d.builds[build].feature_time[feat] + 100);

                                    if value > i_best_value || (value > 0 && e_best_build.is_none()) {
                                        i_best_value = value;
                                        e_best_build = Some(build);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        //Chop - maybe integrate this better with the other feature-clear code tho the logic
        //is kinda different
        if chop && bonus.is_none() && plot_feature.is_some() && plot_improvement.is_none() && !leave_forests {
            let feat = plot_feature.unwrap();
            for build in d.builds.keys() {
                if d.builds[build].improvement.is_none() {
                    //FfH: Added by Kael 04/24/2008 (maintain features)
                    if d.builds[build].feature_remove[feat] && !maintains(feat) {
                        if g.player_can_build(owner, plot, build, false, false) {
                            let mut value = g.plot_get_feature_production(plot, build, team).0 * 10;

                            if value > 0 {
                                value += clear_feature_value;

                                if value > 0 {
                                    if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_DAGGER) {
                                        value += 20;
                                        value *= 2;
                                    }
                                    value *= 500;
                                    value /= 1.max(d.builds[build].feature_time[feat] + 100);

                                    if value > i_best_value {
                                        i_best_value = value;
                                        e_best_build = Some(build);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let old_route = pl.route_type;
        for route in d.routes.keys() {
            if Some(route) == old_route {
                continue;
            }
            let mut temp_value = 0;
            if let Some(cur_improvement) = plot_improvement {
                if old_route.map_or(true, |o| d.routes[route].value > d.routes[o].value) {
                    let ryc = &d.improvements[cur_improvement].route_yield_changes[route];
                    temp_value += ryc[FOOD] * 100;
                    temp_value += ryc[PROD] * 60;
                    temp_value += ryc[COMM] * 40;
                }

                if g.plot_is_being_worked(plot) {
                    temp_value *= 2;
                }
                //road up bonuses if sort of bored.
                if old_route.is_none() && bonus.is_some() {
                    temp_value += if g.plot_is_connected_to_capital(plot, None) { 10 } else { 30 };
                }
            }

            if temp_value > 0 {
                for build in d.builds.keys() {
                    if d.builds[build].route == Some(route) {
                        if g.player_can_build(owner, plot, build, false, false) {
                            //the value multiplier is based on the default time...
                            let mut value = temp_value * 5 * 300;
                            // PORT ADAPT: guard (C++ divides by the build time unguarded).
                            value /= d.builds[build].time.max(1);

                            if value > i_best_value || (value > 0 && e_best_build.is_none()) {
                                i_best_value = value;
                                e_best_build = Some(build);
                            }
                        }
                    }
                }
            }
        }

        if let Some(bb) = e_best_build {
            //Now modify the priority for this build.
            if self.player_ai_is_financial_trouble(g, owner) {
                if d.builds[bb].improvement.is_some() {
                    i_best_value += (i_best_value * 0.max(best_diff_yields[COMM])) / 4;
                    i_best_value = 1.max(i_best_value);
                }
            }

            *best_value = i_best_value;
            *best_build = Some(bb);
        }
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry)` (CvCityAI.cpp:8707-8710)
    pub fn city_ai_get_happy_from_hurry(&self, g: &Game, c: CityId, hurry: HurryType) -> i32 {
        self.city_ai_get_happy_from_hurry_2(g, c, g.city_hurry_population(c, hurry))
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(int iHurryPopulation)` (CvCityAI.cpp:8712-8727)
    pub fn city_ai_get_happy_from_hurry_2(&self, g: &Game, c: CityId, hurry_population: i32) -> i32 {
        let happy_diff = hurry_population - g.d.defines.hurry_pop_anger;
        if happy_diff > 0 {
            if g.cities[c].hurry_anger_timer <= 1 {
                if 2 * g.city_angry_population(c, 1) - g.city_health_rate(c, false, 1) > 1 {
                    return happy_diff;
                }
            }
        }

        0
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry, UnitTypes eUnit, bool bIgnoreNew)` (CvCityAI.cpp:8729-8732)
    pub fn city_ai_get_happy_from_hurry_3(&self, g: &Game, c: CityId, hurry: HurryType, unit: UnitType, ignore_new: bool) -> i32 {
        let cost = g.city_hurry_cost_unit(c, true, unit, ignore_new);
        self.city_ai_get_happy_from_hurry_2(g, c, g.city_hurry_population_for_cost(c, hurry, cost))
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry, BuildingTypes eBuilding, bool bIgnoreNew)` (CvCityAI.cpp:8734-8737)
    pub fn city_ai_get_happy_from_hurry_4(&self, g: &Game, c: CityId, hurry: HurryType, building: BuildingType, ignore_new: bool) -> i32 {
        let cost = g.city_hurry_cost_building(c, true, building, ignore_new);
        self.city_ai_get_happy_from_hurry_2(g, c, g.city_hurry_population_for_cost(c, hurry, cost))
    }

    /// C++ `int CvCityAI::AI_cityValue() const` (CvCityAI.cpp:8740-8758)
    pub fn city_ai_city_value(&self, g: &Game, c: CityId) -> i32 {
        let area_ai = city_area_ai_type(g, c);
        if area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING) || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) {
            return 0;
        }

        let mut value = 0;

        value += g.city_commerce_rate_times100(c, CommerceType::COMMERCE_GOLD);
        value += g.city_commerce_rate_times100(c, CommerceType::COMMERCE_RESEARCH);
        value += 100 * g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);

        value -= 3 * g.city_calculate_colony_maintenance_times100(c);

        value
    }

    /// C++ `bool CvCityAI::AI_doPanic()` (CvCityAI.cpp:8760-8848)
    pub fn city_ai_do_panic(&self, g: &mut Game, c: CityId) -> bool {
        let area_ai = city_area_ai_type(g, c);
        let land_war = area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) || area_ai == Some(AreaAIType::AREAAI_MASSING);

        if land_war {
            let owner = g.cities[c].owner;
            let city_plot = g.city_plot(c);
            let our_defense = self.player_ai_get_our_plot_strength(g, owner, city_plot, 0, true, false);
            let enemy_offense = self.player_ai_get_enemy_plot_strength(g, owner, city_plot, 2, false, false);
            let ratio = (100 * enemy_offense) / 1.max(our_defense);

            if ratio > 100 {
                let production_unit = g.city_production_unit(c);

                if let Some(pu) = production_unit {
                    if g.city_production(c) > 0 {
                        if g.d.units[pu].combat > 0 {
                            // BETTER AI (better AI gold management) Sephi: do not force Hurry, ever
                            self.city_ai_do_hurry(g, c, false);
                            return true;
                        }
                    }
                } else {
                    if self.rand(2, "AI choose panic unit") == 0 && self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_COUNTER)) {
                        // BETTER AI Sephi: do not force Hurry, ever (orig: AI_doHurry(iRatio > 140))
                        self.city_ai_do_hurry(g, c, false);
                    } else if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
                        self.city_ai_do_hurry(g, c, false);
                    } else if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_ATTACK)) {
                        self.city_ai_do_hurry(g, c, false);
                    }
                }
            }
        }
        false
    }

    /// C++ `int CvCityAI::AI_calculateCulturePressure(bool bGreatWork = false)` (CvCityAI.cpp:8850-8929)
    pub fn city_ai_calculate_culture_pressure(&self, g: &Game, c: CityId, great_work: bool) -> i32 {
        let (cx, cy, owner) = (g.cities[c].x, g.cities[c].y, g.cities[c].owner);
        let team = g.city_team(c);

        let mut value = 0;
        for i in 0..NUM_CITY_PLOTS {
            let lp = match g.plot_city(cx, cy, i) {
                Some(p) => p,
                None => continue,
            };
            let plot_owner = g.map.plots[lp].owner;
            match plot_owner {
                None => {
                    value += 1;
                }
                Some(po) => {
                    let mut temp_value = g.plot_calculate_culture_percent(lp, owner);
                    if temp_value == 100 {
                        //do nothing
                    } else if temp_value == 0 || temp_value > 75 {
                        value += 1;
                    } else {
                        temp_value = 100 - temp_value;

                        if i != CITY_HOME_PLOT {
                            temp_value *= 4;
                            temp_value /= NUM_CITY_PLOTS as i32;
                        }

                        if let Some(nob) = g.plot_get_non_obsolete_bonus_type(lp, team) {
                            let mult = if g.player_get_num_tradeable_bonuses(owner, nob) == 0 { 4 } else { 2 };
                            temp_value += self.player_ai_bonus_val(g, owner, nob, 1) * mult;
                        }

                        // C++ quirk: compares the plot owner with the city's ID
                        // (`getOwnerINLINE() == getID()`); FFreeList city ids are never player
                        // indices, so this "captured territory" case never runs.
                        let captured_territory = false;
                        if temp_value > 80 && captured_territory {
                            //captured territory special case
                            temp_value *= 100 - temp_value;
                            temp_value /= 100;
                        }

                        if g.plot_get_team(lp) == Some(team) {
                            temp_value /= if great_work { 10 } else { 2 };
                        } else {
                            temp_value *= 2;
                            if great_work {
                                if self.player_ai_get_attitude(g, owner, po, true) == AttitudeType::ATTITUDE_FRIENDLY {
                                    // C++ quirk: divides the running total, not iTempValue.
                                    value /= 10;
                                }
                            }
                        }

                        value += temp_value;
                    }
                }
            }
        }

        value
    }

    /// C++ `void CvCityAI::AI_buildGovernorChooseProduction()` (CvCityAI.cpp:8932-9103)
    pub fn city_ai_build_governor_choose_production(&self, g: &mut Game, c: CityId) {
        let danger = self.city_ai_is_danger(g, c);

        // only clear the dirty bit if we actually do a check, multiple items might be queued
        self.city_ai_set_choose_production_dirty(g, c, false);

        let water_area = g.city_water_area(c);

        let was_food_production = g.city_is_food_production(c);
        let culture_pressure = self.city_ai_calculate_culture_pressure(g, c, false);
        let population = g.cities[c].population;
        let mut min_value_divisor = 1;
        if population < 3 {
            min_value_divisor = 3;
        } else if population < 7 {
            min_value_divisor = 2;
        }

        g.city_clear_order_queue(c);

        if was_food_production {
            self.city_ai_assign_working_plots(g, c);
        }

        // if we need to pop borders, then do that immediately if we have drama and can do it
        if culture_level_i(g, c) <= 1 && (g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) < 2 || culture_pressure > 0) {
            if self.city_ai_choose_process(g, c, Some(CommerceType::COMMERCE_CULTURE)) {
                return;
            }
        }

        let owner = g.cities[c].owner;

        //workboat
        if let Some(wa) = water_area {
            if self.player_ai_total_water_area_unit_ais(g, owner, wa, UnitAIType::UNITAI_WORKER_SEA) == 0 {
                if self.city_ai_needed_sea_workers(g, c) > 0 {
                    if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER_SEA)) {
                        return;
                    }
                }
            }
        }

        if self.city_ai_count_num_bonuses(g, c, None, false, true, 10, true, true) > 0
            && g.cities[c].population > self.city_ai_count_num_bonuses(g, c, None, true, false, -1, true, true)
        {
            if g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) == 0 {
                self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, MAX_INT, 0);
                return;
            }
        }

        // pick granary or lighthouse, any duration
        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_FOOD, MAX_INT, 0) {
            return;
        }

        if g.city_angry_population(c, 1) > 1 {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_HAPPY, 40, 0) {
                return;
            }
        }

        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_PRODUCTION, 30, 10 / min_value_divisor) {
            return;
        }

        if self.city_ai_choose_building(g, c, BUILDINGFOCUS_EXPERIENCE, 8, 33) {
            return;
        }

        if (g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE) == 0 && culture_pressure != 0) || culture_pressure > 100 {
            if self.city_ai_choose_building(g, c, BUILDINGFOCUS_CULTURE, 30, 0) {
                return;
            }
        }

        let mut economy_flags = 0;
        economy_flags |= BUILDINGFOCUS_GOLD;
        economy_flags |= BUILDINGFOCUS_RESEARCH;
        economy_flags |= BUILDINGFOCUS_MAINTENANCE;
        economy_flags |= BUILDINGFOCUS_HAPPY;
        economy_flags |= BUILDINGFOCUS_HEALTHY;
        economy_flags |= BUILDINGFOCUS_SPECIALIST;
        if !g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE) {
            economy_flags |= BUILDINGFOCUS_ESPIONAGE;
        }

        //20 means 5g or ~2 happiness...
        if self.city_ai_choose_building(g, c, economy_flags, 20, 20 / min_value_divisor) {
            return;
        }

        let area = g.city_area(c);
        let existing_workers = self.player_ai_total_area_unit_ais(g, owner, area, UnitAIType::UNITAI_WORKER);
        let needed_workers = self.player_ai_needed_workers(g, owner, area);

        if !danger && existing_workers < (needed_workers + 1) / 2 {
            if self.city_ai_choose_unit(g, c, Some(UnitAIType::UNITAI_WORKER)) {
                return;
            }
        }

        if let Some(default_specialist) = opt_from_i32::<SpecialistType>(g.d.defines.default_specialist) {
            if g.cities[c].specialist_count[default_specialist] > 0 {
                if self.city_ai_choose_building(g, c, BUILDINGFOCUS_SPECIALIST, 60, 0) {
                    return;
                }
            }
        }

        if self.city_ai_choose_building(g, c, economy_flags, 40, 15 / min_value_divisor) {
            return;
        }

        if self.city_ai_choose_building(g, c, economy_flags | BUILDINGFOCUS_CULTURE, 10, 10 / min_value_divisor) {
            return;
        }

        if self.city_ai_choose_process(g, c, None) {
            return;
        }

        if self.city_ai_choose_building(g, c, 0, MAX_INT, 0) {
            return;
        }

        if self.city_ai_choose_unit(g, c, None) {
            return;
        }
    }

    /// C++ `int CvCityAI::AI_calculateWaterWorldPercent()` (CvCityAI.cpp:9105-9145)
    pub fn city_ai_calculate_water_world_percent(&self, g: &Game, c: CityId) -> i32 {
        let team = g.city_team(c);
        let area = g.city_area(c);
        let mut team_city_count = 0;
        let mut other_city_count = 0;
        for t in g.team_ids() {
            if g.team_is_alive(t) {
                if t == team || g.teams[t].vassal[team] || g.teams[team].vassal[t] {
                    team_city_count += g.team_count_num_cities_by_area(t, area);
                } else {
                    other_city_count += g.team_count_num_cities_by_area(t, area);
                }
            }
        }

        let mut water_percent;
        if other_city_count == 0 {
            water_percent = 100;
        } else {
            water_percent = 100 - ((team_city_count + other_city_count) * 100) / 1.max(g.num_cities);
        }

        water_percent *= 50;
        water_percent /= 100;

        water_percent += (50 * (2 + team_city_count)) / (2 + team_city_count + other_city_count);

        water_percent = 1.max(water_percent);

        water_percent
    }

    /// C++ `int CvCityAI::AI_getYieldMagicValue(const int* piYieldsTimes100, bool bHealthy)` (CvCityAI.cpp:9148-9159)
    pub fn city_ai_get_yield_magic_value(&self, g: &Game, c: CityId, yields_times100: &[i32], healthy: bool) -> i32 {
        let mut pop_eats = g.d.defines.food_consumption_per_population;
        pop_eats += if healthy { 0 } else { 1 };
        pop_eats *= 100;

        let mut value = (yields_times100[FOOD] * 100 + yields_times100[PROD] * 55 + yields_times100[COMM] * 40) - pop_eats * 102;
        value /= 100;
        value
    }

    /// C++ `int CvCityAI::AI_getPlotMagicValue(CvPlot* pPlot, bool bHealthy, bool bWorkerOptimization = false)` (CvCityAI.cpp:9168-9208)
    pub fn city_ai_get_plot_magic_value(&self, g: &Game, c: CityId, plot: PlotIdx, healthy: bool, worker_optimization: bool) -> i32 {
        let d = g.d;
        let owner = g.cities[c].owner;
        let mut yields = [0i32; YieldType::NUM];

        let best_build = if worker_optimization && g.map.plots[plot].working_city == Some(c) {
            let index = g.city_plot_index(c, plot).map_or(-1, |i| i as i32);
            self.city_ai_get_best_build(g, c, index)
        } else {
            None
        };
        for i in 0..YieldType::NUM {
            if let Some(bb) = best_build {
                yields[i] = g.plot_get_yield_with_build(plot, bb, YieldType::from_usize(i), true);
            } else {
                yields[i] = g.map.plots[plot].yields[i] * 100;
            }
        }

        if let Some(current_improvement) = g.map.plots[plot].improvement_type {
            let final_improvement = final_improvement_upgrade(d, current_improvement, 0);

            if let Some(fi) = final_improvement.filter(|&f| f != current_improvement) {
                for i in 0..YieldType::NUM {
                    let y = YieldType::from_usize(i);
                    let mut yield_diff = 100 * g.plot_calculate_improvement_yield_change(plot, fi, y, Some(owner), false, false);
                    yield_diff -= 100 * g.plot_calculate_improvement_yield_change(plot, current_improvement, y, Some(owner), false, false);
                    yields[i] += yield_diff / 2;
                }
            }
        }

        self.city_ai_get_yield_magic_value(g, c, &yields, healthy)
    }

    /// C++ `int CvCityAI::AI_countGoodTiles(bool bHealthy, bool bUnworkedOnly, int iThreshold = 50, bool bWorkerOptimization = false)` (CvCityAI.cpp:9213-9238)
    pub fn city_ai_count_good_tiles(&self, g: &Game, c: CityId, healthy: bool, unworked_only: bool, threshold: i32, worker_optimization: bool) -> i32 {
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let mut count = 0;
        for i in 0..NUM_CITY_PLOTS {
            if i == CITY_HOME_PLOT {
                continue;
            }
            if let Some(lp) = g.plot_city(cx, cy, i) {
                if g.map.plots[lp].working_city == Some(c) {
                    if !unworked_only || !g.plot_is_being_worked(lp) {
                        // C++ quirk: bWorkerOptimization is not passed on.
                        if self.city_ai_get_plot_magic_value(g, c, lp, healthy, false) > threshold {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    /// C++ `int CvCityAI::AI_calculateTargetCulturePerTurn()` (CvCityAI.cpp:9240-9279)
    pub fn city_ai_calculate_target_culture_per_turn(&self, g: &Game, c: CityId) -> i32 {
        // C++: the body is commented out; it returns 1.
        1
    }

    /// C++ `int CvCityAI::AI_countGoodSpecialists(bool bHealthy)` (CvCityAI.cpp:9281-9309)
    pub fn city_ai_count_good_specialists(&self, g: &Game, c: CityId, healthy: bool) -> i32 {
        let owner = g.cities[c].owner;
        let mut count = 0;
        for specialist in g.d.specialists.keys() {
            let mut value = 0;

            value += 100 * g.player_specialist_yield(owner, specialist, YieldType::YIELD_FOOD);
            value += 65 * g.player_specialist_yield(owner, specialist, YieldType::YIELD_PRODUCTION);
            value += 40 * g.player_specialist_yield(owner, specialist, YieldType::YIELD_COMMERCE);

            value += 40 * g.player_specialist_commerce(owner, specialist, CommerceType::COMMERCE_RESEARCH);
            value += 40 * g.player_specialist_commerce(owner, specialist, CommerceType::COMMERCE_GOLD);
            value += 20 * g.player_specialist_commerce(owner, specialist, CommerceType::COMMERCE_ESPIONAGE);
            value += 15 * g.player_specialist_commerce(owner, specialist, CommerceType::COMMERCE_CULTURE);
            value += 25 * g.d.specialists[specialist].great_people_rate_change;

            if value >= if healthy { 200 } else { 300 } {
                count += g.cities[c].max_specialist_count[specialist];
            }
        }
        count -= g.cities[c].free_specialist;

        count
    }

    /// C++ `int CvCityAI::AI_getCityImportance(bool bEconomy, bool bMilitary)` (CvCityAI.cpp:9312-9336)
    pub fn city_ai_get_city_importance(&self, g: &Game, c: CityId, economy: bool, military: bool) -> i32 {
        let mut value = 0;
        let owner = g.cities[c].owner;
        if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE2) {
            let culture_rate_rank = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
            let cultural_victory_num_culture_cities = g.num_culture_victory_cities;

            if culture_rate_rank <= cultural_victory_num_culture_cities {
                value += 100;

                if culture_level_i(g, c) < g.d.culture_levels.len() as i32 - 1 {
                    value += if !military { 100 } else { 0 };
                } else {
                    value += if military { 100 } else { 0 };
                }
            }
        }

        value
    }

    /// C++ `void CvCityAI::AI_stealPlots()` (CvCityAI.cpp:9338-9397)
    pub fn city_ai_steal_plots(&self, g: &mut Game, c: CityId) {
        let importance = self.city_ai_get_city_importance(g, c, true, false);
        let owner = g.cities[c].owner;
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);

        // BUGFIX (modified CityRadius) Sephi: makes sure an AI city only steals plots it can
        // actually use.
        let mut num_city_plots = 21;
        if g.cities[c].plot_radius == 3 {
            num_city_plots = 37;
        }
        for i in 0..num_city_plots {
            let lp = match g.plot_city(cx, cy, i) {
                Some(p) => p,
                None => continue,
            };

            if importance > 0 {
                if g.map.plots[lp].owner == Some(owner) {
                    if let Some(working_city) = g.map.plots[lp].working_city {
                        if working_city != c {
                            let other_importance = self.city_ai_get_city_importance(g, working_city, true, false);
                            if importance > other_importance {
                                g.plot_set_working_city_override(lp, Some(c));
                            }
                        }
                    }
                }
            }

            if g.map.plots[lp].working_city_override == Some(c) {
                if g.map.plots[lp].owner != Some(owner) {
                    g.plot_set_working_city_override(lp, None);
                }
            }
        }
    }

    /// C++ `int CvCityAI::AI_buildingSpecialYieldChangeValue(BuildingTypes eBuilding, YieldTypes eYield)` (CvCityAI.cpp:9407-9454)
    pub fn city_ai_building_special_yield_change_value(&self, g: &Game, c: CityId, building: BuildingType, yield_: YieldType) -> i32 {
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let team = g.city_team(c);
        let mut value = 0;
        let kbuilding = &g.d.buildings[building];
        let mut worked_count = 0;

        let yield_change = kbuilding.sea_plot_yield_change[yield_.idx()];
        if yield_change > 0 {
            let mut water_count = 0;
            for i in 0..NUM_CITY_PLOTS {
                if i == CITY_HOME_PLOT {
                    continue;
                }
                if let Some(lp) = g.plot_city(cx, cy, i) {
                    if g.map.plots[lp].working_city == Some(c) {
                        if g.plot_is_water(lp) {
                            water_count += 1;
                            let mut food = g.map.plots[lp].yields[FOOD];
                            food += if yield_ == YieldType::YIELD_FOOD { yield_change } else { 0 };

                            value += 0.max(food * 2 - 1);
                            if g.plot_is_being_worked(lp) {
                                value += 4;
                                worked_count += 1;
                            }
                            value += if g.plot_get_bonus_type(lp, Some(team)).is_some() { 8 } else { 0 };
                        }
                    }
                }
            }
        }
        if worked_count == 0 {
            let default_specialist = opt_from_i32::<SpecialistType>(g.d.defines.default_specialist);
            if g.cities[c].population > 2 && default_specialist.map_or(true, |s| g.cities[c].specialist_count[s] == 0) {
                value /= 2;
            }
        }

        value
    }

    /// C++ `int CvCityAI::AI_yieldMultiplier(YieldTypes eYield)` (CvCityAI.cpp:9457-9476)
    pub fn city_ai_yield_multiplier(&self, g: &Game, c: CityId, yield_: YieldType) -> i32 {
        let mut multiplier = g.city_base_yield_rate_modifier(c, yield_, 0);

        if yield_ == YieldType::YIELD_PRODUCTION {
            multiplier += g.cities[c].military_production_modifier / 2;
        }

        if yield_ == YieldType::YIELD_COMMERCE {
            let crm = &g.cities[c].commerce_rate_modifier;
            multiplier += (crm[CommerceType::COMMERCE_RESEARCH.idx()] * 60) / 100;
            multiplier += (crm[CommerceType::COMMERCE_GOLD.idx()] * 35) / 100;
            multiplier += (crm[CommerceType::COMMERCE_CULTURE.idx()] * 15) / 100;
        }

        multiplier
    }

    /// C++ `void CvCityAI::AI_updateSpecialYieldMultiplier()` (CvCityAI.cpp:9481-9584)
    pub fn city_ai_update_special_yield_multiplier(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let mut m = [0i32; YieldType::NUM];

        if let Some(production_unit) = g.city_production_unit(c) {
            let ai = d.units[production_unit].default_unit_ai_type;
            if ai == Some(UnitAIType::UNITAI_WORKER_SEA) {
                m[PROD] += 50;
                m[COMM] -= 50;
            }
            if ai == Some(UnitAIType::UNITAI_WORKER) || ai == Some(UnitAIType::UNITAI_SETTLE) {
                m[COMM] -= 50;
            }
        }

        if let Some(production_building) = g.city_production_building(c) {
            let bi = &d.buildings[production_building];
            if bi.building_class_type.map_or(false, |bc| is_world_wonder_class(d, bc)) || g.city_is_production_project(c) {
                m[PROD] += 50;
                m[COMM] -= 25;
            }
            m[PROD] += (-25).max(bi.food_kept);

            let culture = CommerceType::COMMERCE_CULTURE.idx();
            if bi.commerce_change[culture] > 0 || bi.obsolete_safe_commerce_change[culture] > 0 {
                let target_culture_rate = self.city_ai_calculate_target_culture_per_turn(g, c);
                if target_culture_rate > 0 {
                    let culture_rate = g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE);
                    if culture_rate == 0 {
                        m[PROD] += 50;
                    } else if culture_rate < target_culture_rate {
                        m[PROD] += 20;
                    }
                }
            }
        }

        // non-human production value increase
        if !g.city_is_human(c) {
            let owner = g.cities[c].owner;
            let area_ai_type = city_area_ai_type(g, c);

            if (self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_DAGGER) && g.cities[c].population >= 4)
                || area_ai_type == Some(AreaAIType::AREAAI_OFFENSIVE)
                || area_ai_type == Some(AreaAIType::AREAAI_DEFENSIVE)
                || area_ai_type == Some(AreaAIType::AREAAI_MASSING)
                || area_ai_type == Some(AreaAIType::AREAAI_ASSAULT)
            {
                m[PROD] += 10;
                if !self.player_ai_is_financial_trouble(g, owner) {
                    m[COMM] -= 40;
                }
            }

            let gold_per_turn = g.players[owner].gold_per_turn;
            let mut income = 1 + g.player_get_commerce_rate(owner, CommerceType::COMMERCE_GOLD) + g.player_get_commerce_rate(owner, CommerceType::COMMERCE_RESEARCH) + 0.max(gold_per_turn);
            let expenses = 1 + g.player_calculate_inflated_costs(owner) - 0.min(gold_per_turn);

            //FfH: Modified by Kael 03/30/2009
            if income < 1 {
                income = 1;
            }
            //FfH: End Modify

            let ratio = (100 * expenses) / income;
            //Gold -> Production Reduced To
            // 40- -> 100%
            // 60 -> 83%
            // 100 -> 28%
            // 110+ -> 14%
            m[PROD] += 100;
            if ratio > 60 {
                //Greatly decrease production weight
                m[PROD] *= 10.max(120 - ratio);
                m[PROD] /= 72;
            } else if ratio > 40 {
                //Slightly decrease production weight.
                m[PROD] *= 160 - ratio;
                m[PROD] /= 120;
            }
            m[PROD] -= 100;
        }

        self.s().city(c).special_yield_multiplier = m;
    }

    /// C++ `int CvCityAI::AI_specialYieldMultiplier(YieldTypes eYield)` (CvCityAI.cpp:9586-9589)
    pub fn city_ai_special_yield_multiplier(&self, g: &Game, c: CityId, yield_: YieldType) -> i32 {
        self.s().city(c).special_yield_multiplier[yield_.idx()]
    }

    /// C++ `int CvCityAI::AI_countNumBonuses(BonusTypes eBonus, bool bIncludeOurs, bool bIncludeNeutral, int iOtherCultureThreshold, bool bLand = true, bool bWater = true)` (CvCityAI.cpp:9592-9635)
    /// `bonus` `None` = C++ `NO_BONUS` (any bonus).
    pub fn city_ai_count_num_bonuses(&self, g: &Game, c: CityId, bonus: Option<BonusType>, include_ours: bool, include_neutral: bool, other_culture_threshold: i32, land: bool, water: bool) -> i32 {
        let (cx, cy, owner) = (g.cities[c].x, g.cities[c].y, g.cities[c].owner);
        let team = g.city_team(c);
        let area = g.city_area(c);
        let mut count = 0;
        for i in 0..NUM_CITY_PLOTS {
            let lp = match g.plot_city(cx, cy, i) {
                Some(p) => p,
                None => continue,
            };
            // C++ quirk: bLand is not read.
            if g.map.plots[lp].area == Some(area) || (water && g.plot_is_water(lp)) {
                if let Some(loop_bonus) = g.plot_get_bonus_type(lp, Some(team)) {
                    if bonus.map_or(true, |b| b == loop_bonus) {
                        let plot_owner = g.map.plots[lp].owner;
                        if include_ours && plot_owner == Some(owner) && g.map.plots[lp].working_city == Some(c) {
                            count += 1;
                        } else if include_neutral && !g.plot_is_owned(lp) {
                            count += 1;
                        } else if other_culture_threshold > 0 && g.plot_is_owned(lp) && plot_owner != Some(owner) {
                            if let Some(po) = plot_owner {
                                if g.map.plots[lp].culture[po] - g.map.plots[lp].culture[owner] < other_culture_threshold {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvCityAI::AI_playerCloseness(PlayerTypes eIndex, int iMaxDistance)` (CvCityAI.cpp:9637-9649)
    pub fn city_ai_player_closeness(&self, g: &Game, c: CityId, index: PlayerId, max_distance: i32) -> i32 {
        let (turn, distance) = {
            let mut s = self.s();
            let ca = s.city(c);
            (ca.cache_player_closeness_turn, ca.cache_player_closeness_distance)
        };
        if turn != g.game_turn() || distance != max_distance {
            self.city_ai_cache_player_closeness(g, c, max_distance);
        }

        self.s().city(c).player_closeness.get(index.idx()).copied().unwrap_or(0)
    }

    /// C++ `void CvCityAI::AI_cachePlayerCloseness(int iMaxDistance)` (CvCityAI.cpp:9651-9728)
    pub fn city_ai_cache_player_closeness(&self, g: &Game, c: CityId, max_distance: i32) {
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let team = g.city_team(c);
        let area = g.city_area(c);
        let city_plot = g.city_plot(c);
        let population = g.cities[c].population;

        // BETTER_BTS_AI_MOD 6/17/08 jdog5000 (General AI, closeness changes)
        let mut results: Vec<(PlayerId, i32)> = Vec::new();
        for p in g.player_ids() {
            if g.players[p].alive && g.teams[team].has_met[g.players[p].team] {
                let mut value = 0;
                let mut best_value = 0;
                for &lc in &g.players[p].cities {
                    let (lx, ly) = (g.cities[lc].x, g.cities[lc].y);
                    let loop_area = g.city_area(lc);
                    let mut distance = g.step_distance(cx, cy, lx, ly);
                    if area != loop_area {
                        distance += 1;
                        distance /= 2;
                    }
                    if distance <= max_distance {
                        if area == loop_area {
                            let path_distance = calculate_path_distance(g, city_plot, g.city_plot(lc));
                            if path_distance > 0 {
                                distance = path_distance;
                            }
                        }
                        if distance <= max_distance {
                            // Weight by population of both cities, not just pop of other city
                            let mut temp_value = 20 + g.cities[lc].population + population;

                            temp_value *= 1 + (max_distance - distance);
                            temp_value /= 1 + max_distance;

                            //reduce for small islands.
                            let area_city_count = g.map.areas[loop_area].num_cities;
                            temp_value *= area_city_count.min(5);
                            temp_value /= 5;
                            if area_city_count < 3 {
                                temp_value /= 2;
                            }

                            if g.city_is_barbarian(lc) {
                                temp_value /= 4;
                            }

                            value += temp_value;
                            best_value = best_value.max(temp_value);
                        }
                    }
                }
                results.push((p, best_value + value / 4));
            }
        }

        let n = g.num_players();
        let turn = g.game_turn();
        let mut s = self.s();
        let ca = s.city(c);
        if ca.player_closeness.len() < n {
            ca.player_closeness.resize(n, 0);
        }
        for (p, v) in results {
            if ca.player_closeness.len() <= p.idx() {
                ca.player_closeness.resize(p.idx() + 1, 0);
            }
            ca.player_closeness[p.idx()] = v;
        }
        ca.cache_player_closeness_turn = turn;
        ca.cache_player_closeness_distance = max_distance;
    }

    /// C++ `int CvCityAI::AI_cityThreat(bool bDangerPercent = false)` (CvCityAI.cpp:9730-9814)
    pub fn city_ai_city_threat(&self, g: &Game, c: CityId, danger_percent: bool) -> i32 {
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let mut value = 0;
        let crush_strategy = self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CRUSH);
        for p in g.player_ids() {
            if p != owner && g.players[p].alive {
                let mut temp_value = self.city_ai_player_closeness(g, c, p, DEFAULT_PLAYER_CLOSENESS);
                if temp_value > 0 {
                    let other_team = g.players[p].team;
                    if crush_strategy && g.teams[team].ai_war_plan[other_team].is_some() {
                        temp_value *= 400;
                    } else if g.at_war(team, other_team) {
                        temp_value *= 300;
                    } else {
                        let attitude = self.player_ai_get_attitude(g, owner, p, true);
                        if attitude == AttitudeType::ATTITUDE_FURIOUS {
                            temp_value *= 180;
                        } else if attitude == AttitudeType::ATTITUDE_ANNOYED {
                            temp_value *= 130;
                        } else if attitude == AttitudeType::ATTITUDE_CAUTIOUS {
                            temp_value *= 100;
                        } else if attitude == AttitudeType::ATTITUDE_PLEASED {
                            temp_value *= 50;
                        } else if attitude == AttitudeType::ATTITUDE_FRIENDLY {
                            temp_value *= 20;
                        }
                        if crush_strategy {
                            // C++ quirk: halves the running total, not iTempValue.
                            value /= 2;
                        }
                    }
                    temp_value /= 100;
                    value += temp_value;
                }
            }
        }

        if g.city_is_coastal(c, g.d.defines.min_water_size_for_ocean) {
            let current_era = g.players[owner].current_era.i();
            value += 0.max(((10 * current_era) / 3) - 6); //there are better ways to do this
        }

        value += g.cities[c].num_world_wonders * 5;

        if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE3) {
            value += 10;
            value += g.cities[c].commerce_rate_modifier[CommerceType::COMMERCE_CULTURE.idx()] / 20;
            let num_levels = g.d.culture_levels.len() as i32;
            if culture_level_i(g, c) >= num_levels - 2 {
                value += 20;
                if culture_level_i(g, c) >= num_levels - 1 {
                    value += 30;
                }
            }
        }

        value += 2 * self.player_ai_get_plot_danger(g, owner, g.city_plot(c), 3, false);

        value
    }

    /// C++ `int CvCityAI::AI_getWorkersHave()` (CvCityAI.cpp:9820-9823)
    pub fn city_ai_get_workers_have(&self, g: &Game, c: CityId) -> i32 {
        self.s().city(c).workers_have
    }

    /// C++ `int CvCityAI::AI_getWorkersNeeded()` (CvCityAI.cpp:9825-9828)
    pub fn city_ai_get_workers_needed(&self, g: &Game, c: CityId) -> i32 {
        self.s().city(c).workers_needed
    }

    /// C++ `void CvCityAI::AI_changeWorkersHave(int iChange)` (CvCityAI.cpp:9830-9835)
    pub fn city_ai_change_workers_have(&self, g: &mut Game, c: CityId, change: i32) {
        let mut s = self.s();
        let ca = s.city(c);
        ca.workers_have += change;
        ca.workers_have = 0.max(ca.workers_have);
    }

    /// C++ `void CvCityAI::AI_updateWorkersNeededHere()` (CvCityAI.cpp:9838-10029)
    pub fn city_ai_update_workers_needed_here(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let area = g.city_area(c);

        let mut yields = [0i32; YieldType::NUM];

        let mut workers_needed = 0;
        let mut workers_have = 0;
        let mut unimproved_worked_plot_count = 0;
        let mut unimproved_unworked_plot_count = 0;
        let mut worked_unimprovable_count = 0;
        let mut improved_unworked_plot_count = 0;

        let mut special_count = 0;

        let mut worst_worked_plot_value = MAX_INT;
        let mut best_unworked_plot_value = 0;

        if g.city_production_unit(c).is_some() {
            if g.city_production_unit_ai(c) == Some(UnitAIType::UNITAI_WORKER) {
                if g.city_production_turns_left(c) <= 2 {
                    workers_have += 1;
                }
            }
        }

        for i in 0..NUM_CITY_PLOTS {
            let lp = match g.city_index_plot(c, i) {
                Some(p) => p,
                None => continue,
            };
            if g.map.plots[lp].working_city != Some(c) {
                continue;
            }
            if g.map.plots[lp].area != Some(area) {
                continue;
            }
            //How slow is this? It could be almost NUM_CITY_PLOT times faster
            //by iterating groups and seeing if the plot target lands in this city
            //but since this is only called once/turn i'm not sure it matters.
            workers_have += self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_BUILD, None, 0);
            workers_have += g.plot_plot_count(
                lp,
                Some(Game::puf_is_unit_ai_type as ConstPlotUnitFunc),
                UnitAIType::UNITAI_WORKER.i(),
                -1,
                Some(owner),
                Some(team),
                Some(Game::puf_is_no_mission as ConstPlotUnitFunc),
                -1,
                -1,
            );
            if i != CITY_HOME_PLOT {
                let has_best_build = self.city_ai_get_best_build(g, c, i as i32).is_some();
                if g.map.plots[lp].improvement_type.is_none() {
                    if g.plot_is_being_worked(lp) {
                        if has_best_build {
                            unimproved_worked_plot_count += 1;
                        } else {
                            worked_unimprovable_count += 1;
                        }
                    } else if has_best_build {
                        unimproved_unworked_plot_count += 1;
                    }
                } else if !g.plot_is_being_worked(lp) {
                    improved_unworked_plot_count += 1;
                }

                for j in 0..YieldType::NUM {
                    // C++ stores the yields in shorts.
                    yields[j] = g.map.plots[lp].yields[j] as i16 as i32;
                }

                if g.plot_is_being_worked(lp) {
                    let plot_value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                    worst_worked_plot_value = worst_worked_plot_value.min(plot_value);
                } else {
                    let plot_value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                    best_unworked_plot_value = best_unworked_plot_value.max(plot_value);
                }
            }
        }
        //specialists?

        unimproved_worked_plot_count += unimproved_unworked_plot_count.min(worked_unimprovable_count) / 2;

        workers_needed += 2 * unimproved_worked_plot_count;

        let mut best_potential_plot_value = -1;
        if worst_worked_plot_value != MAX_INT {
            //Add an additional citizen to account for future growth.
            let mut best_plot = -1;
            let mut best_specialist: Option<SpecialistType> = None;

            if g.city_angry_population(c, 0) == 0 {
                self.city_ai_add_best_citizen(g, c, true, true, &mut best_plot, &mut best_specialist);
            }

            for i in 0..NUM_CITY_PLOTS {
                if i == CITY_HOME_PLOT {
                    continue;
                }
                let lp = match g.city_index_plot(c, i) {
                    Some(p) => p,
                    None => continue,
                };

                if g.map.plots[lp].working_city == Some(c) && g.map.plots[lp].area == Some(area) {
                    if let Some(best_build) = self.city_ai_get_best_build(g, c, i as i32) {
                        // C++ reads m_aeBestBuild[iI] directly (the same value).
                        for j in 0..YieldType::NUM {
                            yields[j] = g.plot_get_yield_with_build(lp, best_build, YieldType::from_usize(j), true) as i16 as i32;
                        }

                        let plot_value = self.city_ai_yield_value(g, c, &yields, &[], false, false, false, false, true, true);
                        if let Some(improvement) = d.builds[best_build].improvement {
                            if g.cities[c].improvement_free_specialists[improvement] > 0 || d.improvements[improvement].happiness > 0 {
                                special_count += 1;
                            }
                        }
                        best_potential_plot_value = best_potential_plot_value.max(plot_value);
                    }
                }
            }

            if best_plot != -1 {
                g.city_set_working_plot(c, best_plot as usize, false);
            }
            if let Some(bs) = best_specialist {
                g.city_change_specialist_count(c, bs, -1);
            }

            if best_potential_plot_value > worst_worked_plot_value {
                workers_needed += 2;
            }
        }
        workers_needed += (0.max(unimproved_worked_plot_count - 1) * g.players[owner].current_era.i()) / 3;

        if self.player_ai_is_financial_trouble(g, owner) {
            workers_needed *= 3;
            workers_needed /= 2;
        }

        if workers_needed > 0 {
            workers_needed += 1;
            workers_needed = 1.max(workers_needed / 3);
        }

        let mut specialist_extra = (g.cities[c].specialist_population - g.city_total_free_specialists(c)).min(unimproved_unworked_plot_count);
        specialist_extra -= improved_unworked_plot_count;

        workers_needed += 0.max(1 + specialist_extra) / 2;

        if worst_worked_plot_value <= best_unworked_plot_value && best_unworked_plot_value >= best_potential_plot_value {
            workers_needed /= 2;
        }
        if g.city_angry_population(c, 1) > 0 {
            workers_needed += 1;
            workers_needed /= 2;
        }

        workers_needed += (special_count + 1) / 2;

        workers_needed = ((unimproved_worked_plot_count + 1) / 2).max(workers_needed);

        let mut s = self.s();
        let ca = s.city(c);
        ca.workers_needed = workers_needed;
        ca.workers_have = workers_have;
    }

    /// C++ `BuildingTypes CvCityAI::AI_bestAdvancedStartBuilding(int iPass)` (CvCityAI.cpp:10031-10060)
    pub fn city_ai_best_advanced_start_building(&self, g: &Game, c: CityId, pass: i32) -> Option<BuildingType> {
        let mut focus_flags = 0;
        if pass >= 0 {
            focus_flags |= BUILDINGFOCUS_FOOD;
        }
        if pass >= 1 {
            focus_flags |= BUILDINGFOCUS_PRODUCTION;
        }
        if pass >= 2 {
            focus_flags |= BUILDINGFOCUS_EXPERIENCE;
        }
        if pass >= 3 {
            focus_flags |= BUILDINGFOCUS_HAPPY | BUILDINGFOCUS_HEALTHY;
        }
        if pass >= 4 {
            focus_flags |= BUILDINGFOCUS_GOLD | BUILDINGFOCUS_RESEARCH | BUILDINGFOCUS_MAINTENANCE;
            if !g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE) {
                focus_flags |= BUILDINGFOCUS_ESPIONAGE;
            }
        }

        self.city_ai_best_building_threshold(g, c, focus_flags, 0, 0.max(20 - pass * 5), false, None)
    }

    // BETTER AI (new Functions) Sephi

    /// C++ `void CvCityAI::AI_calculateNeededPermDefense()` (CvCityAI.cpp:10068-10146)
    pub fn city_ai_calculate_needed_perm_defense(&self, g: &mut Game, c: CityId) {
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let population = g.cities[c].population;
        let leader = player_leader_info(g, owner);
        let ai_value_defense = leader.map_or(100, |l| l.ai_value_defense);
        let ai_value_mage = leader.map_or(100, |l| l.ai_value_mage);
        let city_plot = g.city_plot(c);

        let mut needed = [0i32; NUM_PERMDEFENDER_TYPES];
        for flag in 0..NUM_PERMDEFENDER_TYPES {
            let mut needed_defenders;
            match flag {
                0 => {
                    //CITY_DEFENSE
                    needed_defenders = (1 + (population + 9) / 12) * (ai_value_defense / 100);
                    needed_defenders = 1.max(needed_defenders);

                    if g.teams[team].barbarian_ally {
                        needed_defenders = 1;
                    }
                }
                1 => {
                    //CITY_COUNTER
                    needed_defenders = 1;
                }
                2 => {
                    //CITY_MAGE
                    needed_defenders = (1 + population / 10) * (ai_value_mage / 100);
                    needed_defenders = 1.max(needed_defenders);
                }
                3 => {
                    //CITY_MEDIC
                    needed_defenders = 1;
                }
                _ => {
                    needed_defenders = 1;
                }
            }
            let mut count_defenders = 0;

            let wanted = match flag {
                0 => Some(UnitAIType::UNITAI_CITY_DEFENSE),
                1 => Some(UnitAIType::UNITAI_CITY_COUNTER),
                2 => Some(UnitAIType::UNITAI_MAGE),
                3 => Some(UnitAIType::UNITAI_MEDIC),
                _ => None,
            };
            for &u in &g.map.plots[city_plot].units {
                let unit = &g.units[u];
                if unit.ai_groupflag == GROUPFLAG_PERMDEFENSE {
                    if wanted.is_some() && unit.ai_type == wanted {
                        count_defenders += 1;
                    }
                }
            }

            needed[flag] = 0.max(needed_defenders - count_defenders);
        }

        self.s().city(c).permdefender_needed = needed;
    }

    /// C++ `void CvCityAI::AI_calculateNeededPatrol()` (CvCityAI.cpp:10149-10211)
    pub fn city_ai_calculate_needed_patrol(&self, g: &mut Game, c: CityId) {
        let owner = g.cities[c].owner;
        let team = g.city_team(c);
        let leader = player_leader_info(g, owner);
        let patrol_group_size = leader.map_or(100, |l| l.ai_patrol_group_size);
        let patrol_group_mage = leader.map_or(100, |l| l.ai_patrol_group_mage);
        let patrol_origin = self.city_ai_get_city_patrol_origin(g, c);

        let mut needed = [0i32; NUM_PATROL_TYPES];
        for flag in 0..NUM_PATROL_TYPES {
            let mut patrol_needed = 0;

            match flag {
                0 => {
                    //normal units
                    patrol_needed = 3;
                    patrol_needed *= patrol_group_size / 100;
                    if g.city_is_capital(c) {
                        patrol_needed += 1;
                        patrol_needed = 4.max(patrol_needed);
                    }

                    if g.teams[team].barbarian_ally {
                        patrol_needed = 2;
                    }
                }
                1 => {
                    patrol_needed = 1;
                    patrol_needed *= patrol_group_mage / 100;
                }
                _ => {}
            }

            let mut count_patrol_units = 0;
            if patrol_origin.is_some() {
                for &u in &g.players[owner].units {
                    if let Some(origin) = g.unit_ai_get_origin_plot(u) {
                        if Some(origin) == patrol_origin && g.units[u].ai_groupflag == GROUPFLAG_PATROL {
                            if flag == 0 && g.units[u].ai_type == Some(UnitAIType::UNITAI_COUNTER) {
                                count_patrol_units += 1;
                            }
                            if flag == 1 && g.units[u].ai_type == Some(UnitAIType::UNITAI_WARWIZARD) {
                                count_patrol_units += 1;
                            }
                        }
                    }
                }
            }

            needed[flag] = 0.max(patrol_needed - count_patrol_units);
        }

        self.s().city(c).patrol_needed = needed;
    }

    /// C++ `void CvCityAI::AI_calculateNeededCityDefenseProduction()` (CvCityAI.cpp:10214-10340)
    pub fn city_ai_calculate_needed_city_defense_production(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let owner = g.cities[c].owner;
        let civ = g.city_civilization_type(c);
        let ci = &d.civilizations[civ];
        let can_train_class = |g: &Game, class: Option<UnitClassType>| -> bool {
            class.and_then(|cl| d.civ_unit(civ, cl)).map_or(false, |u| g.city_can_train(c, u, false, false, false, false))
        };
        let is_production = self.city_ai_is_city_defense_production(g, c);
        let my_production = self.city_ai_get_city_defense_production(g, c);

        let mut needed = [0i32; NUM_CITYDEFENSE_TIERS];
        // C++ quirk: type 0 never counts any unit (only types 1-3 do) and type 3 is beyond
        // NUM_CITYDEFENSE_TIERS, so only types 1 and 2 can be nonzero.
        for type_ in 0..NUM_CITYDEFENSE_TIERS {
            if !is_production {
                needed[type_] = 0;
                continue;
            }

            if type_ == 1 {
                if ci.best_defender1.is_none() {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_defender2.is_some() && can_train_class(g, ci.best_defender2) {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_defender3.is_some() && can_train_class(g, ci.best_defender3) {
                    needed[type_] = 0;
                    continue;
                }
            }

            if type_ == 2 {
                if ci.best_defender2.is_none() {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_defender3.is_some() && can_train_class(g, ci.best_defender3) {
                    needed[type_] = 0;
                    continue;
                }
            }

            if type_ == 3 {
                if ci.best_defender3.is_none() {
                    needed[type_] = 0;
                    continue;
                }
            }

            let mut units_needed = 0;

            for &lc in &g.players[owner].cities {
                if self.city_ai_get_city_defense_production(g, lc) == my_production {
                    let lc_plot = g.city_plot(lc);
                    for &u in &g.map.plots[lc_plot].units {
                        if g.units[u].ai_groupflag == GROUPFLAG_PERMDEFENSE && g.units[u].ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE) {
                            let class = Some(g.unit_get_unit_class_type(u));
                            if type_ == 1 {
                                if class != ci.best_defender1 && class != ci.best_defender2 && class != ci.best_defender3 {
                                    units_needed += 1;
                                }
                            }
                            if type_ == 2 {
                                if class != ci.best_defender2 && class != ci.best_defender3 {
                                    units_needed += 1;
                                }
                            }
                            if type_ == 3 {
                                if class != ci.best_defender3 {
                                    units_needed += 1;
                                }
                            }
                        }
                    }
                }
            }

            needed[type_] = units_needed;
        }

        self.s().city(c).city_defense_production_needed = needed;
    }

    /// C++ `void CvCityAI::AI_calculateNeededCityPatrolProduction()` (CvCityAI.cpp:10343-10454)
    pub fn city_ai_calculate_needed_city_patrol_production(&self, g: &mut Game, c: CityId) {
        let d = g.d;
        let owner = g.cities[c].owner;
        let civ = g.city_civilization_type(c);
        let ci = &d.civilizations[civ];
        let can_train_class = |g: &Game, class: Option<UnitClassType>| -> bool {
            class.and_then(|cl| d.civ_unit(civ, cl)).map_or(false, |u| g.city_can_train(c, u, false, false, false, false))
        };
        let is_production = self.city_ai_is_city_patrol_production(g, c);
        let patrol_origin = self.city_ai_get_city_patrol_origin(g, c);
        let needed_patrol_0 = self.city_ai_needed_patrol(g, c, 0);

        let mut needed = [0i32; NUM_PATROL_TIERS];
        for type_ in 0..NUM_PATROL_TIERS {
            if !is_production {
                needed[type_] = 0;
                continue;
            }

            if type_ == 1 {
                if ci.best_patrol1.is_none() {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_patrol2.is_some() && can_train_class(g, ci.best_patrol2) {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_patrol3.is_some() && can_train_class(g, ci.best_patrol3) {
                    needed[type_] = 0;
                    continue;
                }
            }

            if type_ == 2 {
                if ci.best_patrol2.is_none() {
                    needed[type_] = 0;
                    continue;
                }
                if ci.best_patrol3.is_some() && can_train_class(g, ci.best_patrol3) {
                    needed[type_] = 0;
                    continue;
                }
            }

            if type_ == 3 {
                if ci.best_patrol3.is_none() {
                    needed[type_] = 0;
                    continue;
                }
            }

            let mut units_needed = 0;
            for &u in &g.players[owner].units {
                if g.units[u].ai_groupflag == GROUPFLAG_PATROL && g.units[u].ai_type == Some(UnitAIType::UNITAI_COUNTER) && g.unit_ai_get_origin_plot(u) == patrol_origin {
                    let class = Some(g.unit_get_unit_class_type(u));
                    if type_ == 1 {
                        if class != ci.best_patrol1 && class != ci.best_patrol2 && class != ci.best_patrol3 {
                            units_needed += 1;
                        }
                    }
                    if type_ == 2 {
                        if class != ci.best_patrol2 && class != ci.best_patrol3 {
                            units_needed += 1;
                        }
                    }
                    if type_ == 3 {
                        if class != ci.best_patrol3 {
                            units_needed += 1;
                        }
                    }
                }
            }

            needed[type_] = units_needed.max(needed_patrol_0);
        }

        self.s().city(c).patrol_production_needed = needed;
    }

    /// C++ `int CvCityAI::AI_neededPermDefense(int flag)` (CvCityAI.cpp:10458-10464)
    pub fn city_ai_needed_perm_defense(&self, g: &Game, c: CityId, flag: i32) -> i32 {
        self.s().city(c).permdefender_needed.get(flag as usize).copied().unwrap_or(0)
    }

    /// C++ `void CvCityAI::AI_setCityPatrolOrigin(CvPlot* newpatrolorigin)` (CvCityAI.cpp:10466-10478)
    pub fn city_ai_set_city_patrol_origin(&self, g: &mut Game, c: CityId, newpatrolorigin: PlotIdx) {
        let (x, y) = g.map.xy(newpatrolorigin);
        let mut s = self.s();
        let ca = s.city(c);
        ca.city_patrolgroup_origin_x = x;
        ca.city_patrolgroup_origin_y = y;
    }

    /// `AI_setCityPatrolOrigin(NULL)`: clears the patrol origin (see the SIGNATURE NOTES).
    pub fn city_ai_clear_city_patrol_origin(&self, g: &mut Game, c: CityId) {
        let mut s = self.s();
        let ca = s.city(c);
        ca.city_patrolgroup_origin_x = -1;
        ca.city_patrolgroup_origin_y = -1;
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityPatrolOrigin()` (CvCityAI.cpp:10480-10490)
    pub fn city_ai_get_city_patrol_origin(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        let (x, y) = {
            let mut s = self.s();
            let ca = s.city(c);
            (ca.city_patrolgroup_origin_x, ca.city_patrolgroup_origin_y)
        };
        if x == -1 && y == -1 {
            None
        } else {
            g.plot_xy(x, y, 0, 0)
        }
    }

    /// C++ `int CvCityAI::AI_neededPatrol(int flag)` (CvCityAI.cpp:10492-10498)
    pub fn city_ai_needed_patrol(&self, g: &Game, c: CityId, flag: i32) -> i32 {
        self.s().city(c).patrol_needed.get(flag as usize).copied().unwrap_or(0)
    }

    /// C++ `int CvCityAI::AI_CityDangerValue()` (CvCityAI.cpp:10500-10561)
    pub fn city_ai_city_danger_value(&self, g: &Game, c: CityId) -> i32 {
        let team = g.city_team(c);
        if g.team_get_at_war_count(team, true) == 0 {
            return 0;
        }

        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let plot_area = g.city_area(c);
        let city_plot = g.city_plot(c);

        let mut danger_value = 0;
        // C++ quirk: tempvalue is declared outside the loops (and its power-value computation is
        // commented out), so every unit adds 5 / (distance + 1).
        let mut temp_value = 0;

        let range = 4;

        for dx in -range..=range {
            for dy in -range..=range {
                let lp = match g.plot_xy(cx, cy, dx, dy) {
                    Some(p) => p,
                    None => continue,
                };
                if g.map.plots[lp].area == Some(plot_area) || g.plot_is_water(lp) {
                    let (lx, ly) = g.map.xy(lp);
                    let distance = g.step_distance(cx, cy, lx, ly);

                    for &u in &g.map.plots[lp].units {
                        if g.unit_is_enemy(u, Some(team), None) {
                            if g.unit_can_attack(u) {
                                if !g.unit_is_invisible(u, team, false) {
                                    if g.unit_can_move_or_attack_into(u, city_plot, false) {
                                        temp_value = 5.max(temp_value);
                                        temp_value /= distance + 1;
                                        danger_value += temp_value;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        danger_value
    }

    /// C++ `void CvCityAI::AI_setCityDefenseProduction(CvPlot* newplot)` (CvCityAI.cpp:10563-10575)
    pub fn city_ai_set_city_defense_production(&self, g: &mut Game, c: CityId, newplot: PlotIdx) {
        let (x, y) = g.map.xy(newplot);
        let mut s = self.s();
        let ca = s.city(c);
        ca.city_defense_production_x = x;
        ca.city_defense_production_y = y;
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityDefenseProduction()` (CvCityAI.cpp:10577-10587)
    pub fn city_ai_get_city_defense_production(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        let (x, y) = {
            let mut s = self.s();
            let ca = s.city(c);
            (ca.city_defense_production_x, ca.city_defense_production_y)
        };
        if x == -1 && y == -1 {
            None
        } else {
            g.plot_xy(x, y, 0, 0)
        }
    }

    /// C++ `void CvCityAI::AI_CityChooseDefenseProduction()` (CvCityAI.cpp:10589-10640)
    pub fn city_ai_city_choose_defense_production(&self, g: &mut Game, c: CityId) {
        let owner = g.cities[c].owner;
        //Do we have a valid City for Production?
        if let Some(dp) = self.city_ai_get_city_defense_production(g, c) {
            if g.map.plots[dp].owner == Some(owner) && g.plot_is_city(dp, false, None) {
                return;
            }
        }

        let range = 8;
        let mut best_city: Option<PlotIdx> = None;
        let mut best_value = 10;
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let area = g.city_area(c);
        // Is there a production city close by?
        for dx in -range..=range {
            for dy in -range..=range {
                let lp = match g.plot_xy(cx, cy, dx, dy) {
                    Some(p) => p,
                    None => continue,
                };
                if let Some(lc) = g.map.plots[lp].plot_city.filter(|&lc| g.cities.contains(lc)) {
                    if g.map.plots[lp].area == Some(area) && g.map.plots[lp].owner == Some(owner) {
                        let lc_production = self.city_ai_get_city_defense_production(g, lc);
                        if lc_production.is_some() && lc_production == Some(lp) {
                            let (lx, ly) = g.map.xy(lp);
                            let value = g.step_distance(cx, cy, lx, ly);
                            if value < best_value {
                                best_value = value;
                                best_city = Some(lp);
                            }
                        }
                    }
                }
            }
        }
        match best_city {
            Some(bc) => self.city_ai_set_city_defense_production(g, c, bc),
            //become a production City
            None => {
                let city_plot = g.city_plot(c);
                self.city_ai_set_city_defense_production(g, c, city_plot);
            }
        }
    }

    /// C++ `int CvCityAI::AI_neededCityDefenseProduction(int type)` (CvCityAI.cpp:10642-10645)
    pub fn city_ai_needed_city_defense_production(&self, g: &Game, c: CityId, type_: i32) -> i32 {
        // PORT ADAPT: C++ reads past the array for type >= NUM_CITYDEFENSE_TIERS; 0 here.
        self.s().city(c).city_defense_production_needed.get(type_ as usize).copied().unwrap_or(0)
    }

    /// C++ `bool CvCityAI::AI_isCityDefenseProduction()` (CvCityAI.cpp:10647-10659)
    pub fn city_ai_is_city_defense_production(&self, g: &Game, c: CityId) -> bool {
        match self.city_ai_get_city_defense_production(g, c) {
            None => false,
            Some(p) => p == g.city_plot(c),
        }
    }

    /// C++ `void CvCityAI::AI_setCityPatrolProduction(CvPlot* newplot)` (CvCityAI.cpp:10661-10673)
    pub fn city_ai_set_city_patrol_production(&self, g: &mut Game, c: CityId, newplot: PlotIdx) {
        let (x, y) = g.map.xy(newplot);
        let mut s = self.s();
        let ca = s.city(c);
        ca.city_patrol_production_x = x;
        ca.city_patrol_production_y = y;
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityPatrolProduction()` (CvCityAI.cpp:10675-10685)
    pub fn city_ai_get_city_patrol_production(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        let (x, y) = {
            let mut s = self.s();
            let ca = s.city(c);
            (ca.city_patrol_production_x, ca.city_patrol_production_y)
        };
        if x == -1 && y == -1 {
            None
        } else {
            g.plot_xy(x, y, 0, 0)
        }
    }

    /// C++ `void CvCityAI::AI_CityChoosePatrolProduction()` (CvCityAI.cpp:10688-10735)
    pub fn city_ai_city_choose_patrol_production(&self, g: &mut Game, c: CityId) {
        let city_plot = g.city_plot(c);
        //Capital always is a Patrol Production City to get things started
        if g.city_is_capital(c) {
            self.city_ai_set_city_patrol_production(g, c, city_plot);
            return;
        }

        //Do we already have a Patrol group?
        let patrol_origin = self.city_ai_get_city_patrol_origin(g, c);
        if patrol_origin.is_none() {
            return;
        }
        let owner = g.cities[c].owner;
        //Do we have a valid City for Production?
        if let Some(pp) = self.city_ai_get_city_patrol_production(g, c) {
            if g.map.plots[pp].owner == Some(owner) && g.plot_is_city(pp, false, None) {
                return;
            }
        }

        let area = g.city_area(c);
        for lc in g.players[owner].cities.clone() {
            if lc != c {
                if self.city_ai_get_city_patrol_origin(g, lc) == patrol_origin {
                    if g.city_area(lc) == area {
                        if self.city_ai_get_city_patrol_production(g, lc).is_some() {
                            // C++ quirk: takes the other city's own plot, not its production
                            // plot, and keeps looking (the last match wins).
                            let lc_plot = g.city_plot(lc);
                            self.city_ai_set_city_patrol_production(g, c, lc_plot);
                        }
                    }
                }
            }
        }
        if self.city_ai_get_city_patrol_production(g, c).is_none() {
            self.city_ai_set_city_patrol_production(g, c, city_plot);
        }
    }

    /// C++ `int CvCityAI::AI_neededCityPatrolProduction(int type)` (CvCityAI.cpp:10738-10741)
    pub fn city_ai_needed_city_patrol_production(&self, g: &Game, c: CityId, type_: i32) -> i32 {
        // PORT ADAPT: C++ reads past the array for type >= NUM_PATROL_TIERS; 0 here.
        self.s().city(c).patrol_production_needed.get(type_ as usize).copied().unwrap_or(0)
    }

    /// C++ `bool CvCityAI::AI_isCityPatrolProduction()` (CvCityAI.cpp:10743-10755)
    pub fn city_ai_is_city_patrol_production(&self, g: &Game, c: CityId) -> bool {
        match self.city_ai_get_city_patrol_production(g, c) {
            None => false,
            Some(p) => p == g.city_plot(c),
        }
    }
}
