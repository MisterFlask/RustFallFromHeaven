//! Port of CvUnitAI.cpp:14123-16967 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES: `unit_ai_improve_local_plot` and `unit_ai_next_city_to_improve` take
//! `Option<CityId>` (the C++ passes NULL; integrator's request). The C++ also calls
//! `AI_improvePlot(pPlot, NO_BUILD)` (the route case, `eBestBonusBuild` from `AI_improveBonus`),
//! which the stub's `BuildType` cannot express: `unit_ai_improve_plot` keeps its signature and
//! delegates to `unit_ai_improve_plot_opt(.., Option<BuildType>)`, which such callers should use.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `bool CvUnitAI::AI_specialSeaTransportMissionary()` (CvUnitAI.cpp:14124-14333)
    pub fn unit_ai_special_sea_transport_missionary(&self, g: &mut Game, u: UnitId) -> bool {
        if !g.unit_can_cargo_all_move(u) {
            return false;
        }

        let plot = g.unit_plot(u);

        let mut missionary_unit = None;
        for &loop_unit in &g.map.plots[plot].units {
            if g.units[loop_unit].transport_unit == Some(u) {
                if g.units[loop_unit].ai_type == Some(UnitAIType::UNITAI_MISSIONARY) {
                    missionary_unit = Some(loop_unit);
                    break;
                }
            }
        }

        let Some(missionary_unit) = missionary_unit else {
            return false;
        };

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let group = unit_group(g, u);
        let state_religion = g.player_get_state_religion(owner);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_spread_plot: Option<PlotIdx> = None;
        let mut executive = false;

        // XXX what about non-coastal cities?
        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if !g.plot_is_coastal_land(loop_plot, -1) {
                continue;
            }
            let Some(city) = g.map.plots[loop_plot].plot_city else {
                continue;
            };

            let mut value = 0;
            let mut corp_value = 0;

            for religion in g.d.religions.keys() {
                if g.unit_can_spread(missionary_unit, loop_plot, Some(religion), false) {
                    if state_religion == Some(religion) {
                        value += 3;
                    }

                    if g.player_has_holy_city(owner, religion) {
                        value += 1;
                    }
                }
            }

            for corporation in g.d.corporations.keys() {
                if g.unit_can_spread_corporation(missionary_unit, loop_plot, Some(corporation), false) {
                    if g.player_has_headquarters(owner, corporation) {
                        corp_value += 3;
                    }
                }
            }

            if value > 0 {
                if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                    if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_SPREAD, Some(group), 0) == 0 {
                        let mut path_turns = 0;
                        if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                            value *= g.cities[city].population;

                            if g.cities[city].owner == owner {
                                value *= 4;
                            } else if g.city_team(city) == team {
                                value *= 3;
                            }

                            if g.city_religion_count(city) == 0 {
                                value *= 2;
                            }

                            value /= g.city_religion_count(city) + 1;

                            if path_turns == 1 {
                                value *= 2;
                            }

                            value *= 1000;

                            value /= path_turns + 1;

                            if value > best_value {
                                best_value = value;
                                best_plot = g.unit_get_path_end_turn_plot(u);
                                best_spread_plot = Some(loop_plot);
                                executive = false;
                            }
                        }
                    }
                }
            }

            if corp_value > 0 {
                if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                    if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_SPREAD_CORPORATION, Some(group), 0) == 0 {
                        let mut path_turns = 0;
                        if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                            corp_value *= g.cities[city].population;

                            if path_turns == 1 {
                                // C++ quirk: doubles iValue (the religion value), not iCorpValue.
                                value *= 2;
                            }

                            corp_value *= 1000;

                            corp_value /= path_turns + 1;

                            if corp_value > best_value {
                                best_value = corp_value;
                                best_plot = g.unit_get_path_end_turn_plot(u);
                                best_spread_plot = Some(loop_plot);
                                executive = true;
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_spread_plot)) = (best_plot, best_spread_plot) {
            let mission_ai = if executive { MissionAIType::MISSIONAI_SPREAD_CORPORATION } else { MissionAIType::MISSIONAI_SPREAD };
            let (bx, by) = g.map.xy(best_plot);
            let (sx, sy) = g.map.xy(best_spread_plot);

            if best_plot == best_spread_plot || g.step_distance(bx, by, sx, sy) == 1 {
                if g.unit_at_plot(u, best_spread_plot) {
                    g.unit_unload_all(u); // XXX is this dangerous (not pushing a mission...) XXX air units?
                    return true;
                } else {
                    push_mission(g, u, MissionType::MISSION_MOVE_TO, sx, sy, 0, false, Some(mission_ai), Some(best_spread_plot));
                    return true;
                }
            } else {
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, Some(mission_ai), Some(best_spread_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_specialSeaTransportSpy()` (CvUnitAI.cpp:14337-14448)
    pub fn unit_ai_special_sea_transport_spy(&self, g: &mut Game, u: UnitId) -> bool {
        if !g.unit_can_cargo_all_move(u) {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let group = unit_group(g, u);

        let mut best_value = 0;
        let mut best_player: Option<PlayerId> = None;

        for i in g.civ_player_ids() {
            if g.players[i].alive {
                if g.players[i].team != team {
                    if self.player_ai_get_attitude(g, owner, i, true) <= AttitudeType::ATTITUDE_ANNOYED {
                        let value = g.players[i].total_population;

                        if value > best_value {
                            best_value = value;
                            best_player = Some(i);
                        }
                    }
                }
            }
        }

        let Some(best_player) = best_player else {
            return false;
        };

        let mut best_plot: Option<PlotIdx> = None;
        let mut best_spy_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if g.plot_is_coastal_land(loop_plot, -1) {
                if g.map.plots[loop_plot].owner == Some(best_player) {
                    let mut value = g.map.plots[loop_plot].area.map_or(0, |a| g.map.areas[a].cities_per_player[best_player]);

                    if value > 0 {
                        if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_ATTACK_SPY, Some(group), 4) == 0 {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                value *= 1000;

                                value /= path_turns + 1;

                                if value > best_value {
                                    best_value = value;
                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                    best_spy_plot = Some(loop_plot);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_spy_plot)) = (best_plot, best_spy_plot) {
            let (bx, by) = g.map.xy(best_plot);
            let (sx, sy) = g.map.xy(best_spy_plot);

            if best_plot == best_spy_plot || g.step_distance(bx, by, sx, sy) == 1 {
                if g.unit_at_plot(u, best_spy_plot) {
                    g.unit_unload_all(u); // XXX is this dangerous (not pushing a mission...) XXX air units?
                    return true;
                } else {
                    push_mission(g, u, MissionType::MISSION_MOVE_TO, sx, sy, 0, false, Some(MissionAIType::MISSIONAI_ATTACK_SPY), Some(best_spy_plot));
                    return true;
                }
            } else {
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_ATTACK_SPY), Some(best_spy_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_carrierSeaTransport()` (CvUnitAI.cpp:14452-14632)
    pub fn unit_ai_carrier_sea_transport(&self, g: &mut Game, u: UnitId) -> bool {
        // XXX maybe protect land troops?

        let mut max_air_range = 0;

        for cargo_unit in g.unit_get_cargo_units(u) {
            max_air_range = max_air_range.max(g.unit_air_range(cargo_unit));
        }

        if max_air_range == 0 {
            return false;
        }

        let owner = g.units[u].owner;
        let group = unit_group(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_carrier_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                if g.plot_is_adjacent_to_land(loop_plot) {
                    let (lx, ly) = g.map.xy(loop_plot);
                    let mut value = 0;

                    for dx in -max_air_range..=max_air_range {
                        for dy in -max_air_range..=max_air_range {
                            let Some(loop_plot_air) = g.plot_xy(lx, ly, dx, dy) else {
                                continue;
                            };
                            let (ax, ay) = g.map.xy(loop_plot_air);

                            if g.plot_distance(lx, ly, ax, ay) <= max_air_range {
                                if !g.plot_is_barbarian(loop_plot_air) {
                                    if g.unit_potential_war_action(u, loop_plot_air) {
                                        if g.plot_is_city(loop_plot_air, false, None) {
                                            value += 1;
                                        }

                                        if g.map.plots[loop_plot_air].improvement_type.is_some() {
                                            value += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if value > 0 {
                        if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                            let stealth = g.unit_get_invisible_type(u).is_some();
                            if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_CARRIER, Some(group), if stealth { 5 } else { 3 })
                                <= if stealth { 0 } else { 3 }
                            {
                                let mut path_turns = 0;
                                if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                    value *= 1000;

                                    for direction in 0..DirectionType::NUM {
                                        if let Some(direction_plot) = g.plot_direction(lx, ly, DirectionType::from_usize(direction)) {
                                            if g.plot_is_city(direction_plot, false, None) && g.unit_is_enemy(u, g.plot_get_team(direction_plot), Some(loop_plot)) {
                                                value /= 2;
                                                break;
                                            }
                                        }
                                    }

                                    value += self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_ASSAULT, Some(group), 2) * 2000;

                                    value /= path_turns + 1;

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                        best_carrier_plot = Some(loop_plot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_carrier_plot)) = (best_plot, best_carrier_plot) {
            if g.unit_at_plot(u, best_carrier_plot) {
                if g.group_has_cargo(unit_group(g, u)) {
                    let plot = g.unit_plot(u);

                    let num_units = g.map.plots[plot].units.len();

                    for _ in 0..num_units {
                        let mut done = true;
                        for cargo_unit in g.map.plots[plot].units.clone() {
                            if !g.units.contains(cargo_unit) || !g.units.contains(u) {
                                continue;
                            }

                            if g.unit_is_cargo(cargo_unit) {
                                let my_group = g.units[u].group;
                                let transport_group = g.units[cargo_unit].transport_unit.and_then(|t| g.units[t].group);
                                if g.units[cargo_unit].owner == owner
                                    && transport_group == my_group
                                    && g.unit_get_domain_type(cargo_unit) == DomainType::DOMAIN_AIR
                                {
                                    if g.unit_can_move(cargo_unit) && g.unit_is_group_head(cargo_unit) {
                                        // careful, this might kill the cargo group
                                        let cargo_group = unit_group(g, cargo_unit);
                                        if self.group_ai_update(g, cargo_group) {
                                            done = false;
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        if done {
                            break;
                        }
                    }
                }

                // PORT ADAPT: guard (C++ assumes the carrier survives its cargo's updates).
                if !g.units.contains(u) {
                    return true;
                }

                if g.unit_can_plunder(u, best_carrier_plot, false) {
                    push_mission(g, u, MissionType::MISSION_PLUNDER, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_CARRIER), Some(best_carrier_plot));
                } else {
                    push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                }
                return true;
            } else {
                let (bx, by) = g.map.xy(best_plot);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_CARRIER), Some(best_carrier_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_connectPlot(CvPlot* pPlot, int iRange = 0)` (CvUnitAI.cpp:14636-14695)
    pub fn unit_ai_connect_plot(&self, g: &mut Game, u: UnitId, plot: PlotIdx, range: i32) -> bool {
        let owner = g.units[u].owner;
        let group = unit_group(g, u);

        if !g.plot_is_visible_enemy_unit_for_unit(plot, u) {
            if self.player_ai_plot_target_mission_ais(g, owner, plot, MissionAIType::MISSIONAI_BUILD, Some(group), range) == 0 {
                if g.unit_generate_path(u, plot, MOVE_SAFE_TERRITORY, true, None) {
                    let (px, py) = g.map.xy(plot);
                    let my_plot_group = g.plot_get_plot_group(g.unit_plot(u), owner);

                    for loop_city in g.players[owner].cities.clone() {
                        if !g.plot_is_connected_to(plot, loop_city) {
                            if my_plot_group == g.plot_get_plot_group(g.city_plot(loop_city), owner) {
                                push_mission(g, u, MissionType::MISSION_ROUTE_TO, px, py, MOVE_SAFE_TERRITORY, false, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
                                return true;
                            }
                        }
                    }

                    for loop_city in g.players[owner].cities.clone() {
                        if !g.plot_is_connected_to(plot, loop_city) {
                            let city_plot = g.city_plot(loop_city);
                            if !g.plot_is_visible_enemy_unit_for_unit(city_plot, u) {
                                if g.unit_generate_path(u, city_plot, MOVE_SAFE_TERRITORY, true, None) {
                                    let (cx, cy) = (g.cities[loop_city].x, g.cities[loop_city].y);
                                    if g.unit_at_plot(u, plot) {
                                        // need to test before moving...
                                        push_mission(g, u, MissionType::MISSION_ROUTE_TO, cx, cy, MOVE_SAFE_TERRITORY, false, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
                                    } else {
                                        push_mission(g, u, MissionType::MISSION_ROUTE_TO, cx, cy, MOVE_SAFE_TERRITORY, false, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
                                        let append = queue_nonempty(g, u);
                                        push_mission(g, u, MissionType::MISSION_ROUTE_TO, px, py, MOVE_SAFE_TERRITORY, append, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
                                    }

                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_improveCity(CvCity* pCity)` (CvUnitAI.cpp:14699-14751)
    pub fn unit_ai_improve_city(&self, g: &mut Game, u: UnitId, city: CityId) -> bool {
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_build: Option<BuildType> = None;

        if self.unit_ai_best_city_build(g, u, city, &mut best_plot, &mut best_build, None, Some(u)) {
            let (Some(best_plot), Some(best_build)) = (best_plot, best_build) else {
                return false;
            };

            let here = g.unit_plot(u);
            let mission;
            if g.map.plots[here].working_city != Some(city) || g.d.builds[best_build].route.is_some() {
                mission = MissionType::MISSION_ROUTE_TO;
            } else {
                let mut m = MissionType::MISSION_MOVE_TO;
                if g.unit_generate_path(u, best_plot, 0, false, None) && {
                    let (data1, data2) = path_last_node(g);
                    data2 == 1 && data1 == 0
                } {
                    if g.map.plots[best_plot].route_type.is_some() {
                        m = MissionType::MISSION_ROUTE_TO;
                    }
                } else if g.map.plots[here].route_type.is_none() {
                    if plot_move_cost(g, here) > 1 {
                        m = MissionType::MISSION_ROUTE_TO;
                    }
                }
                mission = m;
            }

            let best_build = self.unit_ai_better_plot_build(g, u, best_plot, best_build).unwrap_or(best_build);

            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, mission, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            push_build(g, u, best_build, best_plot);

            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_improveLocalPlot(int iRange, CvCity* pIgnoreCity)` (CvUnitAI.cpp:14753-14894)
    /// (`pIgnoreCity` may be NULL).
    pub fn unit_ai_improve_local_plot(&self, g: &mut Game, u: UnitId, range: i32, ignore_city: Option<CityId>) -> bool {
        let owner = g.units[u].owner;
        let group = unit_group(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let safe_automation = g.players[owner].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION];
        let ruins = opt_from_i32::<ImprovementType>(g.d.defines.ruins_improvement);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_build: Option<BuildType> = None;

        for ix in -range..=range {
            for iy in -range..=range {
                let Some(loop_plot) = g.plot_xy(ux, uy, ix, iy) else {
                    continue;
                };
                if !g.plot_is_city_radius(loop_plot) {
                    continue;
                }
                let Some(city) = g.map.plots[loop_plot].working_city else {
                    continue;
                };
                if g.cities[city].owner != owner {
                    continue;
                }
                if ignore_city == Some(city) {
                    continue;
                }
                if !self.unit_ai_plot_valid(g, u, loop_plot) {
                    continue;
                }

                // PORT ADAPT: C++ `getCityPlotIndex` is -1 outside the radius (never for the
                // plot's working city); skip it rather than index out of bounds.
                let Some(index) = g.city_plot_index(city, loop_plot) else {
                    continue;
                };
                let index = index as i32;
                if index == CITY_HOME_PLOT as i32 {
                    continue;
                }

                let workers_ok = ignore_city.is_none() || {
                    let needed = self.city_ai_get_workers_needed(g, city);
                    let have = self.city_ai_get_workers_have(g, city);
                    needed > 0 && have < 1 + needed * 2 / 3
                };
                if !workers_ok {
                    continue;
                }
                let Some(city_best_build) = self.city_ai_get_best_build(g, city, index) else {
                    continue;
                };

                if g.unit_can_build(u, loop_plot, city_best_build, false) {
                    let mut allowed = true;
                    let improvement = g.map.plots[loop_plot].improvement_type;

                    if safe_automation {
                        if improvement.is_some() && improvement != ruins {
                            allowed = false;
                        }
                    }

                    if allowed {
                        if improvement.is_some() && g.d.builds[city_best_build].improvement.is_some() {
                            allowed = false;
                        }
                    }

                    if allowed {
                        let mut value = self.city_ai_get_best_build_value(g, city, index);
                        let mut path_turns = 0;
                        if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                            let mut max_workers = 1;
                            if g.unit_plot(u) == loop_plot {
                                value *= 3;
                                value /= 2;
                            } else if path_last_node(g).0 == 0 {
                                path_turns += 1;
                            } else if path_turns <= 1 {
                                max_workers = self.unit_ai_calculate_plot_workers_needed(g, u, loop_plot, city_best_build);
                            }

                            if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 0) < max_workers {
                                value *= 1000;
                                value /= 1 + path_turns;

                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(loop_plot);
                                    best_build = Some(city_best_build);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_build)) = (best_plot, best_build) {
            if let Some(working_city) = g.map.plots[best_plot].working_city {
                self.city_ai_change_workers_have(g, working_city, 1);

                if let Some(here_city) = g.map.plots[g.unit_plot(u)].working_city {
                    self.city_ai_change_workers_have(g, here_city, -1);
                }
            }
            let mut mission = MissionType::MISSION_MOVE_TO;

            let here = g.unit_plot(u);
            let mut path_turns = 0;
            if g.unit_generate_path(u, best_plot, 0, true, Some(&mut path_turns)) && {
                let (data1, data2) = path_last_node(g);
                data2 == 1 && data1 == 0
            } {
                if g.map.plots[best_plot].route_type.is_some() {
                    mission = MissionType::MISSION_ROUTE_TO;
                }
            } else if g.map.plots[here].route_type.is_none() {
                if plot_move_cost(g, here) > 1 {
                    mission = MissionType::MISSION_ROUTE_TO;
                }
            }

            let best_build = self.unit_ai_better_plot_build(g, u, best_plot, best_build).unwrap_or(best_build);

            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, mission, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            push_build(g, u, best_build, best_plot);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_nextCityToImprove(CvCity* pCity)` (CvUnitAI.cpp:14897-14980)
    /// (`pCity` may be NULL).
    pub fn unit_ai_next_city_to_improve(&self, g: &mut Game, u: UnitId, city: Option<CityId>) -> bool {
        let owner = g.units[u].owner;

        let mut best_value = 0;
        let mut best_build: Option<BuildType> = None;
        let mut best_plot: Option<PlotIdx> = None;

        for loop_city in g.players[owner].cities.clone() {
            if Some(loop_city) != city {
                //iValue = pLoopCity->AI_totalBestBuildValue(area());
                let workers_needed = self.city_ai_get_workers_needed(g, loop_city);
                let workers_have = self.city_ai_get_workers_have(g, loop_city);

                let mut value = 0.max(workers_needed - workers_have) * 100;
                value += workers_needed * 10;
                value *= workers_needed + 1;
                value /= workers_have + 1;

                if value > 0 {
                    let mut plot: Option<PlotIdx> = None;
                    let mut build: Option<BuildType> = None;
                    if self.unit_ai_best_city_build(g, u, loop_city, &mut plot, &mut build, None, Some(u)) {
                        let (Some(plot), Some(build)) = (plot, build) else {
                            continue;
                        };

                        value *= 1000;

                        if g.city_is_capital(loop_city) {
                            value *= 2;
                        }

                        let mut path_turns = 0;
                        g.unit_generate_path(u, plot, 0, true, Some(&mut path_turns));
                        // C++ quirk: without a path iPathTurns is MAX_INT and MAX_INT + 1
                        // overflows to MIN_INT (the value becomes 0).
                        value /= path_turns.wrapping_add(1);

                        if value > best_value {
                            best_value = value;
                            best_build = Some(build);
                            best_plot = Some(plot);
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_build)) = (best_plot, best_build) {
            if let Some(here_city) = g.map.plots[g.unit_plot(u)].working_city {
                self.city_ai_change_workers_have(g, here_city, -1);
            }

            if let Some(working_city) = g.map.plots[best_plot].working_city {
                self.city_ai_change_workers_have(g, working_city, 1);
            }

            let best_build = self.unit_ai_better_plot_build(g, u, best_plot, best_build).unwrap_or(best_build);

            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_ROUTE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            push_build(g, u, best_build, best_plot);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_nextCityToImproveAirlift()` (CvUnitAI.cpp:14984-15040)
    pub fn unit_ai_next_city_to_improve_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        let group = unit_group(g, u);
        if g.groups[group].units.len() > 1 {
            return false;
        }

        let Some(city) = g.map.plots[g.unit_plot(u)].plot_city else {
            return false;
        };

        if g.cities[city].max_airlift == 0 {
            return false;
        }

        let owner = g.units[u].owner;
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for loop_city in g.players[owner].cities.clone() {
            if loop_city != city {
                if g.unit_can_airlift_at(u, g.city_plot(city), g.cities[loop_city].x, g.cities[loop_city].y) {
                    let value = self.city_ai_total_best_build_value(g, loop_city, g.city_area(loop_city));

                    if value > best_value {
                        best_value = value;
                        best_plot = Some(g.city_plot(loop_city));
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_AIRLIFT, bx, by, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_irrigateTerritory()` (CvUnitAI.cpp:15044-15186)
    pub fn unit_ai_irrigate_territory(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let group = unit_group(g, u);
        let civ = g.unit_get_civilization_type(u);
        let safe_automation = g.players[owner].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION];
        let leave_forests = g.players[owner].options[PlayerOptionType::PLAYEROPTION_LEAVE_FORESTS];
        let ruins = opt_from_i32::<ImprovementType>(g.d.defines.ruins_improvement);

        let mut best_value = 0;
        let mut best_build: Option<BuildType> = None;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if !self.unit_ai_plot_valid(g, u, loop_plot) {
                continue;
            }
            if g.map.plots[loop_plot].owner != Some(owner) {
                // XXX team???
                continue;
            }
            if g.map.plots[loop_plot].working_city.is_some() {
                continue;
            }

            let improvement = g.map.plots[loop_plot].improvement_type;

            if improvement.is_none() || !(safe_automation && !(improvement == ruins)) {
                if improvement.map_or(true, |imp| !g.d.improvements[imp].carries_irrigation) {
                    let non_obsolete_bonus = g.plot_get_non_obsolete_bonus_type(loop_plot, team);

                    let bonus_trade = match (improvement, non_obsolete_bonus) {
                        (Some(imp), Some(bonus)) => g.d.improvements[imp].improvement_bonus[bonus].bonus_trade,
                        _ => false,
                    };
                    if !bonus_trade {
                        if g.plot_is_irrigation_available(loop_plot, true) {
                            let mut best_temp_build_value = MAX_INT;
                            let mut best_temp_build: Option<BuildType> = None;

                            for build in g.d.builds.keys() {
                                if let Some(build_improvement) = g.d.builds[build].improvement {
                                    if g.d.improvements[build_improvement].carries_irrigation {
                                        if g.unit_can_build(u, loop_plot, build, false) {
                                            let mut value = 10000;

                                            value /= g.d.builds[build].time + 1;

                                            // XXX feature production???

                                            // C++ quirk: keeps the lowest value (the slowest build).
                                            if value < best_temp_build_value {
                                                best_temp_build_value = value;
                                                best_temp_build = Some(build);
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(best_temp_build) = best_temp_build {
                                let mut valid = true;

                                if leave_forests {
                                    if let Some(feature) = g.map.plots[loop_plot].feature_type {
                                        //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
                                        if g.d.builds[best_temp_build].feature_remove[feature] && !g.d.civilizations[civ].maintain_features[feature] {
                                            if g.d.features[feature].yield_change[YieldType::YIELD_PRODUCTION.idx()] > 0 {
                                                valid = false;
                                            }
                                        }
                                    }
                                }

                                if valid {
                                    if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                                        if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 1) == 0 {
                                            let mut path_turns = 0;
                                            // XXX should this actually be at the top of the loop? (with saved paths and all...)
                                            if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                                let mut value = 10000;

                                                value /= path_turns + 1;

                                                if value > best_value {
                                                    best_value = value;
                                                    best_build = Some(best_temp_build);
                                                    best_plot = Some(loop_plot);
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
        }

        if let (Some(best_plot), Some(best_build)) = (best_plot, best_build) {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_ROUTE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            push_build(g, u, best_build, best_plot);

            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_fortTerritory(bool bCanal, bool bAirbase)` (CvUnitAI.cpp:15188-15306)
    pub fn unit_ai_fort_territory(&self, g: &mut Game, u: UnitId, canal: bool, airbase: bool) -> bool {
        let owner = g.units[u].owner;
        let group = unit_group(g, u);
        let civ = g.unit_get_civilization_type(u);
        let leave_forests = g.players[owner].options[PlayerOptionType::PLAYEROPTION_LEAVE_FORESTS];

        let mut best_value = 0;
        let mut best_build: Option<BuildType> = None;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if !self.unit_ai_plot_valid(g, u, loop_plot) {
                continue;
            }
            if g.map.plots[loop_plot].owner != Some(owner) {
                // XXX team???
                continue;
            }
            if g.map.plots[loop_plot].improvement_type.is_some() {
                continue;
            }

            let mut value = 0;
            value += if canal { self.player_ai_get_plot_canal_value(g, owner, loop_plot) } else { 0 };
            value += if airbase { self.player_ai_get_plot_airbase_value(g, owner, loop_plot) } else { 0 };

            if value > 0 {
                let mut best_temp_build_value = MAX_INT;
                let mut best_temp_build: Option<BuildType> = None;

                for build in g.d.builds.keys() {
                    if let Some(build_improvement) = g.d.builds[build].improvement {
                        let info = &g.d.improvements[build_improvement];
                        if info.acts_as_city {
                            if info.defense_modifier > 0 {
                                if g.unit_can_build(u, loop_plot, build, false) {
                                    // C++ quirk: overwrites iValue (the canal/airbase value), so
                                    // the plot value below is the last buildable fort's speed.
                                    value = 10000;

                                    value /= g.d.builds[build].time + 1;

                                    if value < best_temp_build_value {
                                        best_temp_build_value = value;
                                        best_temp_build = Some(build);
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(best_temp_build) = best_temp_build {
                    if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                        let mut valid = true;

                        if leave_forests {
                            if let Some(feature) = g.map.plots[loop_plot].feature_type {
                                //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
                                if g.d.builds[best_temp_build].feature_remove[feature] && !g.d.civilizations[civ].maintain_features[feature] {
                                    if g.d.features[feature].yield_change[YieldType::YIELD_PRODUCTION.idx()] > 0 {
                                        valid = false;
                                    }
                                }
                            }
                        }

                        if valid {
                            if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 3) == 0 {
                                let mut path_turns = 0;
                                if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                    value *= 1000;
                                    value /= path_turns + 1;

                                    if value > best_value {
                                        best_value = value;
                                        best_build = Some(best_temp_build);
                                        best_plot = Some(loop_plot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_build)) = (best_plot, best_build) {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_ROUTE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            push_build(g, u, best_build, best_plot);

            return true;
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_improveBonus(int iMinValue = 0, CvPlot** ppBestPlot = NULL, BuildTypes* peBestBuild = NULL, int* piBestValue = NULL)` (CvUnitAI.cpp:15309-15607)
    ///
    /// The out-parameters are written when the best value is below `min_value` (C++: only when
    /// `ppBestPlot` is not NULL; callers passing NULL pass dummies here).
    pub fn unit_ai_improve_bonus(&self, g: &mut Game, u: UnitId, min_value: i32, best_plot_out: &mut Option<PlotIdx>, best_build_out: &mut Option<BuildType>, best_value_out: &mut i32) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let group = unit_group(g, u);
        let civ = g.unit_get_civilization_type(u);
        let safe_automation = g.players[owner].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION];
        let leave_forests = g.players[owner].options[PlayerOptionType::PLAYEROPTION_LEAVE_FORESTS];
        let ruins = opt_from_i32::<ImprovementType>(g.d.defines.ruins_improvement);
        let is_human = g.unit_is_human(u);
        let personality = g.players[owner].personality_type;

        let mut best_value = 0;
        let mut best_resource_value = 0;
        let mut best_build: Option<BuildType> = None;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_build_is_route = false;

        let can_route = g.unit_can_build_route(u);

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if !(g.map.plots[loop_plot].owner == Some(owner) && self.unit_ai_plot_valid(g, u, loop_plot)) {
                continue;
            }

            let here = g.unit_plot(u);
            let loop_area = g.map.plots[loop_plot].area;
            let mut can_improve = loop_area == g.map.plots[here].area;
            if !can_improve {
                if g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && g.plot_is_water(loop_plot) {
                    if let Some(loop_area) = loop_area {
                        if g.plot_is_adjacent_to_area(here, loop_area) {
                            can_improve = true;
                        }
                    }
                }
            }

            //FfH: Added by Kael 12/20/2008
            if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                can_improve = false;
            }
            if !g.unit_at_plot(u, loop_plot) {
                if !g.unit_can_move_into(u, loop_plot, false, false, false) {
                    can_improve = false;
                }
            }
            //FfH: End Add

            if !can_improve {
                continue;
            }

            let Some(non_obsolete_bonus) = g.plot_get_non_obsolete_bonus_type(loop_plot, team) else {
                continue;
            };

            let is_connected = g.plot_is_connected_to_capital(loop_plot, Some(owner));
            if !(g.map.plots[loop_plot].working_city.is_some() || (is_connected || can_route)) {
                continue;
            }

            let improvement = g.map.plots[loop_plot].improvement_type;
            let feature = g.map.plots[loop_plot].feature_type;

            let mut do_improve = match improvement {
                None => true,
                Some(imp) if g.d.improvements[imp].acts_as_city || g.d.improvements[imp].improvement_bonus[non_obsolete_bonus].bonus_trade => false,
                Some(imp) if Some(imp) == ruins => true,
                Some(_) => !safe_automation,
            };

            let mut best_temp_build_value = MAX_INT;
            let mut best_temp_build: Option<BuildType> = None;

            if do_improve {
                for build in g.d.builds.keys() {
                    let Some(build_improvement) = g.d.builds[build].improvement else {
                        continue;
                    };
                    let imp_info = &g.d.improvements[build_improvement];
                    if imp_info.improvement_bonus[non_obsolete_bonus].bonus_trade || (!g.plot_is_city_radius(loop_plot) && imp_info.acts_as_city) {
                        if g.unit_can_build(u, loop_plot, build, false) {
                            //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
                            let feature_ok = match feature {
                                None => true,
                                Some(f) => !g.d.builds[build].feature_remove[f] || !leave_forests || g.d.civilizations[civ].maintain_features[f],
                            };
                            if feature_ok {
                                let mut value = 10000;

                                value /= g.d.builds[build].time + 1;

                                /*FfH: Added by Chalid AiManaAndBonus 06/10/2006*/
                                if !is_human {
                                    value /= 100;
                                    // PORT ADAPT: no personality reads as modifier 0 (C++ indexes NO_LEADER).
                                    let modifier = personality.map_or(0, |l| g.d.leader_heads[l].improvement_weight_modifier[build_improvement]);
                                    value *= 0.max(100 - modifier);
                                }
                                value -= self.rand(4000, "AIBonus");
                                //FfH: End Add

                                // XXX feature production???

                                if value < best_temp_build_value {
                                    best_temp_build_value = value;
                                    best_temp_build = Some(build);
                                }
                            }
                        }
                    }
                }
            }
            if best_temp_build.is_none() {
                do_improve = false;
            }

            if best_temp_build.is_some() || (can_route && !is_connected) {
                let mut path_turns = 0;
                if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                    let mut value = self.player_ai_bonus_val(g, owner, non_obsolete_bonus, 1);

                    if do_improve {
                        let btb = best_temp_build.unwrap();
                        let new_improvement = g.d.builds[btb].improvement.unwrap();
                        //iValue += (GC.getImprovementInfo((ImprovementTypes) GC.getBuildInfo(eBestTempBuild).getImprovement()))
                        value += 5 * g.plot_calculate_improvement_yield_change(loop_plot, new_improvement, YieldType::YIELD_FOOD, Some(owner), false, false);
                        //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
                        let ignore_feature = match feature {
                            None => true,
                            Some(f) => g.d.builds[btb].feature_remove[f] && !g.d.civilizations[civ].maintain_features[f],
                        };
                        value += 5 * g.plot_calculate_nature_yield(loop_plot, YieldType::YIELD_FOOD, Some(team), ignore_feature);
                    }

                    value += 0.max(100 * g.d.bonuses[non_obsolete_bonus].ai_objective);

                    if g.player_get_num_tradeable_bonuses(owner, non_obsolete_bonus) == 0 {
                        value *= 2;
                    }

                    let mut max_workers = 1;
                    if let Some(btb) = best_temp_build {
                        if !g.d.builds[btb].kill {
                            //allow teaming.
                            max_workers = self.unit_ai_calculate_plot_workers_needed(g, u, loop_plot, btb);
                            if path_last_node(g).0 == 0 {
                                max_workers = ((max_workers + 1) / 2).min(1 + self.player_ai_base_bonus_val(g, owner, non_obsolete_bonus) / 20);
                            }
                        }
                    }

                    if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 0) < max_workers
                        && (!do_improve || g.plot_get_build_turns_left(loop_plot, best_temp_build.unwrap(), 0, 0) > path_turns * 2 - 1)
                    {
                        if do_improve {
                            value *= 1000;

                            if g.unit_at_plot(u, loop_plot) {
                                value *= 3;
                            }

                            value /= path_turns + 1;

                            if g.plot_is_city_radius(loop_plot) {
                                value *= 2;
                            }

                            if value > best_value {
                                best_value = value;
                                best_build = best_temp_build;
                                best_plot = Some(loop_plot);
                                best_build_is_route = false;
                                best_resource_value = value;
                            }
                        } else {
                            let improvement = g.map.plots[loop_plot].improvement_type;
                            if let Some(imp) = improvement {
                                if g.d.improvements[imp].improvement_bonus[non_obsolete_bonus].bonus_trade {
                                    value *= 1000;
                                    value /= path_turns + 1;

                                    if value > best_value {
                                        best_value = value;
                                        best_build = None;
                                        best_plot = Some(loop_plot);
                                        best_build_is_route = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if best_value < min_value {
            *best_plot_out = best_plot;
            *best_build_out = best_build;
            *best_value_out = best_resource_value;
        }

        if let Some(best_plot) = best_plot {
            if let Some(best_build) = best_build {
                let mut best_mission = MissionType::MISSION_MOVE_TO;

                let connected_city = g.map.plots[best_plot].working_city.map_or(false, |c| g.city_is_connected_to_capital(c, None));
                if !connected_city {
                    best_mission = MissionType::MISSION_ROUTE_TO;
                } else {
                    let (bx, by) = g.map.xy(best_plot);
                    let distance = g.step_distance(g.units[u].x, g.units[u].y, bx, by);
                    let mut path_turns = 0;
                    if g.unit_generate_path(u, best_plot, 0, false, Some(&mut path_turns)) {
                        if path_turns >= distance {
                            best_mission = MissionType::MISSION_ROUTE_TO;
                        }
                    }
                }

                let best_build = self.unit_ai_better_plot_build(g, u, best_plot, best_build).unwrap_or(best_build);
                let (bx, by) = g.map.xy(best_plot);
                push_mission(g, u, best_mission, bx, by, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
                push_build(g, u, best_build, best_plot);

                return true;
            } else if best_build_is_route {
                if self.unit_ai_connect_plot(g, u, best_plot, 0) {
                    return true;
                }
                /*else
                {
                    // the plot may be connected, but not connected to capital, if capital is not on same area, or if civ has no capital (like barbarians)
                    FAssertMsg(false, "Expected that a route could be built to eBestPlot");
                }*/
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_improvePlot(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:15611-15638)
    pub fn unit_ai_improve_plot(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> bool {
        self.unit_ai_improve_plot_opt(g, u, plot, Some(build))
    }

    /// C++ `bool CvUnitAI::AI_improvePlot(CvPlot* pPlot, BuildTypes eBuild)` with a possibly
    /// NO_BUILD `eBuild` ("if eBuild is NO_BUILD, assumes a route is desired").
    pub fn unit_ai_improve_plot_opt(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: Option<BuildType>) -> bool {
        if let Some(build) = build {
            let build = self.unit_ai_better_plot_build(g, u, plot, build).unwrap_or(build);
            if !g.unit_at_plot(u, plot) {
                let (px, py) = g.map.xy(plot);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, px, py, 0, false, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
            }
            push_build(g, u, build, plot);

            return true;
        } else if g.unit_can_build_route(u) {
            if self.unit_ai_connect_plot(g, u, plot, 0) {
                return true;
            }
        }

        false
    }

    /// C++ `BuildTypes CvUnitAI::AI_betterPlotBuild(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:15640-15796)
    ///
    /// Always `Some` (the C++ returns `eBuild` when nothing is better).
    pub fn unit_ai_better_plot_build(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> Option<BuildType> {
        let mut build_route = false;
        let mut clear_feature = false;

        let feature = g.map.plots[plot].feature_type;
        let owner = g.units[u].owner;
        let civ = g.unit_get_civilization_type(u);

        if g.d.builds[build].route.is_some() {
            return Some(build);
        }

        let mut workers_needed = self.unit_ai_calculate_plot_workers_needed(g, u, plot, build);

        let bonus = g.plot_get_bonus_type(plot, None);
        let working_city = g.map.plots[plot].working_city;

        if bonus.is_none() {
            if let Some(working_city) = working_city {
                workers_needed = 1.max(workers_needed.min(self.city_ai_get_workers_have(g, working_city)));
            }
        }

        let original = &g.d.builds[build];
        let hills_extra_movement = g.d.defines.hills_extra_movement;
        let hills = g.plot_is_hills(plot);

        if let Some(feature) = feature {
            let feature_info = &g.d.features[feature];
            //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
            if original.feature_remove[feature] && !g.d.civilizations[civ].maintain_features[feature] {
                if original.improvement.is_none()
                    || (!g.plot_is_being_worked(plot)
                        || (feature_info.yield_change[YieldType::YIELD_FOOD.idx()] + feature_info.yield_change[YieldType::YIELD_PRODUCTION.idx()]) <= 0)
                {
                    clear_feature = true;
                }
            }

            if feature_info.movement_cost > 1 && workers_needed > 1 {
                build_route = true;
            }
        }

        if bonus.is_some() {
            build_route = true;
        } else if hills {
            if hills_extra_movement > 0 && workers_needed > 1 {
                build_route = true;
            }
        }

        if g.map.plots[plot].route_type.is_some() {
            build_route = false;
        }

        let (px, py) = g.map.xy(plot);
        let mut best_build: Option<BuildType> = None;
        let mut best_value = 0;
        for loop_build in g.d.builds.keys() {
            let build_info = &g.d.builds[loop_build];

            let route = build_info.route;

            //FfH: Added by Kael 04/24/2008 (isMaintainFeatures)
            let clears = clear_feature && feature.map_or(false, |f| build_info.feature_remove[f] && !g.d.civilizations[civ].maintain_features[f]);
            if (build_route && route.is_some()) || clears {
                if g.unit_can_build(u, plot, loop_build, false) {
                    let mut value = 10000;
                    if let (true, Some(route)) = (build_route, route) {
                        value *= 1 + g.d.routes[route].value;
                        value /= 2;

                        if bonus.is_some() {
                            value *= 2;
                        }

                        if working_city.is_some() {
                            value *= 2 + workers_needed + if hills && workers_needed > 1 { 2 * hills_extra_movement } else { 0 };
                            value /= 3;
                        }

                        if let Some(improvement) = original.improvement {
                            let route_yields = &g.d.improvements[improvement].route_yield_changes[route];
                            let mut route_multiplier = route_yields[YieldType::YIELD_FOOD.idx()] * 100;
                            route_multiplier += route_yields[YieldType::YIELD_PRODUCTION.idx()] * 100;
                            route_multiplier += route_yields[YieldType::YIELD_COMMERCE.idx()] * 60;
                            value *= 100 + route_multiplier;
                            value /= 100;
                        }

                        let mut plot_group_id: Option<PlotGroupId> = None;
                        for direction in 0..DirectionType::NUM {
                            if let Some(loop_plot) = g.plot_direction(px, py, DirectionType::from_usize(direction)) {
                                if g.plot_is_river(plot) || g.map.plots[loop_plot].route_type.is_some() {
                                    if let Some(loop_group) = g.plot_get_plot_group(loop_plot, owner) {
                                        // C++ quirk: iPlotGroupId starts at -1, so the first
                                        // plot group found always counts as a bridge.
                                        if Some(loop_group) != plot_group_id {
                                            //This plot bridges plot groups, so route it.
                                            value *= 4;
                                            break;
                                        } else {
                                            plot_group_id = Some(loop_group);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    value /= build_info.time + 1;

                    if value > best_value {
                        best_value = value;
                        best_build = Some(loop_build);
                    }
                }
            }
        }

        match best_build {
            None => Some(build),
            Some(b) => Some(b),
        }
    }

    /// C++ `bool CvUnitAI::AI_connectBonus(bool bTestTrade = true)` (CvUnitAI.cpp:15800-15838)
    pub fn unit_ai_connect_bonus(&self, g: &mut Game, u: UnitId, test_trade: bool) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        // XXX how do we make sure that we can build roads???

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                if g.map.plots[loop_plot].owner == Some(owner) {
                    // XXX team???
                    if let Some(non_obsolete_bonus) = g.plot_get_non_obsolete_bonus_type(loop_plot, team) {
                        if !g.plot_is_connected_to_capital(loop_plot, None) {
                            let improvement = g.map.plots[loop_plot].improvement_type;
                            if !test_trade || improvement.map_or(false, |imp| g.d.improvements[imp].improvement_bonus[non_obsolete_bonus].bonus_trade) {
                                if self.unit_ai_connect_plot(g, u, loop_plot, 0) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_connectCity()` (CvUnitAI.cpp:15842-15881)
    pub fn unit_ai_connect_city(&self, g: &mut Game, u: UnitId) -> bool {
        // XXX how do we make sure that we can build roads???

        if let Some(loop_city) = g.map.plots[g.unit_plot(u)].working_city {
            let city_plot = g.city_plot(loop_city);
            if self.unit_ai_plot_valid(g, u, city_plot) {
                if !g.city_is_connected_to_capital(loop_city, None) {
                    if self.unit_ai_connect_plot(g, u, city_plot, 1) {
                        return true;
                    }
                }
            }
        }

        let owner = g.units[u].owner;
        for loop_city in g.players[owner].cities.clone() {
            let city_plot = g.city_plot(loop_city);
            if self.unit_ai_plot_valid(g, u, city_plot) {
                if !g.city_is_connected_to_capital(loop_city, None) {
                    if self.unit_ai_connect_plot(g, u, city_plot, 1) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_routeCity()` (CvUnitAI.cpp:15885-15927)
    pub fn unit_ai_route_city(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let group = unit_group(g, u);

        for loop_city in g.players[owner].cities.clone() {
            let city_plot = g.city_plot(loop_city);
            if self.unit_ai_plot_valid(g, u, city_plot) {
                if let Some(route_to_city) = self.city_ai_get_route_to_city(g, loop_city) {
                    if !g.plot_is_visible_enemy_unit_for_unit(city_plot, u) {
                        if g.unit_generate_path(u, city_plot, MOVE_SAFE_TERRITORY, true, None) {
                            let route_plot = g.city_plot(route_to_city);
                            if !g.plot_is_visible_enemy_unit_for_unit(route_plot, u) {
                                if self.player_ai_plot_target_mission_ais(g, owner, route_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 0) == 0 {
                                    if g.unit_generate_path(u, route_plot, MOVE_SAFE_TERRITORY, true, None) {
                                        let (cx, cy) = (g.cities[loop_city].x, g.cities[loop_city].y);
                                        let (rx, ry) = (g.cities[route_to_city].x, g.cities[route_to_city].y);
                                        push_mission(g, u, MissionType::MISSION_ROUTE_TO, cx, cy, MOVE_SAFE_TERRITORY, false, Some(MissionAIType::MISSIONAI_BUILD), Some(route_plot));
                                        let append = queue_nonempty(g, u);
                                        push_mission(g, u, MissionType::MISSION_ROUTE_TO, rx, ry, MOVE_SAFE_TERRITORY, append, Some(MissionAIType::MISSIONAI_BUILD), Some(route_plot));

                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_routeTerritory(bool bImprovementOnly = false)` (CvUnitAI.cpp:15931-16023)
    pub fn unit_ai_route_territory(&self, g: &mut Game, u: UnitId, improvement_only: bool) -> bool {
        // XXX how do we make sure that we can build roads???

        let owner = g.units[u].owner;
        let group = unit_group(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                if g.map.plots[loop_plot].owner == Some(owner) {
                    // XXX team???
                    if let Some(best_route) = g.player_get_best_route(owner, Some(loop_plot)) {
                        if Some(best_route) != g.map.plots[loop_plot].route_type {
                            let valid = if improvement_only {
                                match g.map.plots[loop_plot].improvement_type {
                                    Some(improvement) => g.d.improvements[improvement].route_yield_changes[best_route].iter().any(|&y| y > 0),
                                    None => false,
                                }
                            } else {
                                true
                            };

                            if valid {
                                if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                                    if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 1) == 0 {
                                        let mut path_turns = 0;
                                        if g.unit_generate_path(u, loop_plot, MOVE_SAFE_TERRITORY, true, Some(&mut path_turns)) {
                                            let mut value = 10000;

                                            value /= path_turns + 1;

                                            if value > best_value {
                                                best_value = value;
                                                best_plot = Some(loop_plot);
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

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_ROUTE_TO, bx, by, MOVE_SAFE_TERRITORY, false, Some(MissionAIType::MISSIONAI_BUILD), Some(best_plot));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_travelToUpgradeCity()` (CvUnitAI.cpp:16027-16134)
    pub fn unit_ai_travel_to_upgrade_city(&self, g: &mut Game, u: UnitId) -> bool {
        // is there a city which can upgrade us?
        let Some(upgrade_city) = g.unit_get_upgrade_city(u, /*bSearch*/ true) else {
            return false;
        };

        // cache some stuff
        let plot = g.unit_plot(u);
        let domain = g.unit_get_domain_type(u);
        let sea_unit = domain == DomainType::DOMAIN_SEA;
        let can_airlift_unit = domain == DomainType::DOMAIN_LAND;
        let should_skip_to_upgrade = domain != DomainType::DOMAIN_AIR;
        let upgrade_plot = g.city_plot(upgrade_city);
        let (upx, upy) = (g.cities[upgrade_city].x, g.cities[upgrade_city].y);

        // if we at the upgrade city, stop, wait to get upgraded
        if upgrade_plot == plot {
            if !should_skip_to_upgrade {
                return false;
            }

            push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
            return true;
        }

        if domain == DomainType::DOMAIN_AIR {
            push_mission(g, u, MissionType::MISSION_MOVE_TO, upx, upy, 0, false, None, None);
            return true;
        }

        // find the closest city
        let mut closest_city = g.map.plots[plot].plot_city;
        let at_closest_city = closest_city.is_some();
        if closest_city.is_none() {
            closest_city = g.map.plots[plot].working_city;
        }
        if closest_city.is_none() {
            let team = g.unit_get_team(u);
            closest_city = g.map_find_city(g.units[u].x, g.units[u].y, None, Some(team), true, sea_unit, None, None, None);
        }

        // can we path to the upgrade city?
        let mut upgrade_city_path_turns = 0;
        let mut this_turn_plot: Option<PlotIdx> = None;
        let can_path_to_upgrade_city = g.unit_generate_path(u, upgrade_plot, 0, true, Some(&mut upgrade_city_path_turns));
        if can_path_to_upgrade_city {
            this_turn_plot = g.unit_get_path_end_turn_plot(u);
        }

        // if we close to upgrade city, head there
        if let (Some(this_turn_plot), Some(closest_city)) = (this_turn_plot, closest_city) {
            if closest_city == upgrade_city || upgrade_city_path_turns < 4 {
                let (tx, ty) = g.map.xy(this_turn_plot);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, tx, ty, 0, false, None, None);
                return true;
            }
        }

        // check for better airlift choice
        if let Some(closest_city) = closest_city {
            if can_airlift_unit && g.cities[closest_city].max_airlift > 0 {
                // if we at the closest city, then do the airlift, or wait
                if at_closest_city {
                    // can we do the airlift this turn?
                    if g.unit_can_airlift_at(u, g.city_plot(closest_city), upx, upy) {
                        push_mission(g, u, MissionType::MISSION_AIRLIFT, upx, upy, 0, false, None, None);
                        return true;
                    }
                    // wait to do it next turn
                    else {
                        push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                        return true;
                    }
                }

                let mut closest_city_path_turns = 0;
                let mut this_turn_plot_for_airlift: Option<PlotIdx> = None;
                let can_path_to_closest_city = g.unit_generate_path(u, g.city_plot(closest_city), 0, true, Some(&mut closest_city_path_turns));
                if can_path_to_closest_city {
                    this_turn_plot_for_airlift = g.unit_get_path_end_turn_plot(u);
                }

                // is the closest city closer pathing? If so, move toward closest city
                if let Some(t) = this_turn_plot_for_airlift {
                    if !can_path_to_upgrade_city || closest_city_path_turns < upgrade_city_path_turns {
                        let (tx, ty) = g.map.xy(t);
                        push_mission(g, u, MissionType::MISSION_MOVE_TO, tx, ty, 0, false, None, None);
                        return true;
                    }
                }
            }
        }

        // did not have better airlift choice, go ahead and path to the upgrade city
        if let Some(this_turn_plot) = this_turn_plot {
            let (tx, ty) = g.map.xy(this_turn_plot);
            push_mission(g, u, MissionType::MISSION_MOVE_TO, tx, ty, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_retreatToCity(bool bPrimary = false, bool bAirlift = false, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:16137-16263)
    pub fn unit_ai_retreat_to_city(&self, g: &mut Game, u: UnitId, primary: bool, airlift: bool, max_path: i32) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let here = g.unit_plot(u);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = MAX_INT;
        let current_danger = self.player_ai_get_plot_danger(g, owner, here, -1, true);

        let city = g.map.plots[here].plot_city;

        if current_danger == 0 {
            if self.retreat_city_here_ok(g, u, city, primary, airlift) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                return true;
            }
        }

        let is_settler_sea = g.units[u].ai_type == Some(UnitAIType::UNITAI_SETTLER_SEA);

        let mut pass = 0;
        while pass < 4 {
            for loop_city in g.players[owner].cities.clone() {
                let city_plot = g.city_plot(loop_city);
                if !self.unit_ai_plot_valid(g, u, city_plot) {
                    continue;
                }
                if primary && !self.player_ai_is_primary_area(g, owner, g.city_area(loop_city)) {
                    continue;
                }
                if airlift && g.cities[loop_city].max_airlift <= 0 {
                    continue;
                }
                if g.plot_is_visible_enemy_unit_for_unit(city_plot, u) {
                    continue;
                }

                let mut path_turns = 0;
                if !g.unit_at_plot(u, city_plot) && g.unit_generate_path(u, city_plot, if pass > 1 { MOVE_IGNORE_DANGER } else { 0 }, true, Some(&mut path_turns)) {
                    if path_turns <= if pass == 2 { 1 } else { max_path } {
                        let group = unit_group(g, u);
                        if pass > 0 || (g.group_can_fight(group) || self.player_ai_get_plot_danger(g, owner, city_plot, -1, true) < current_danger) {
                            let mut value = path_turns;

                            if is_settler_sea {
                                let area = g.city_area(loop_city);
                                value *= 1 + 0.max(self.player_ai_total_area_unit_ais(g, owner, area, UnitAIType::UNITAI_SETTLE) - self.player_ai_total_area_unit_ais(g, owner, area, UnitAIType::UNITAI_SETTLER_SEA));
                            }

                            if value < best_value {
                                best_value = value;
                                best_plot = g.unit_get_path_end_turn_plot(u);
                            }
                        }
                    }
                }
            }

            if best_plot.is_some() {
                break;
            } else if pass == 0 {
                if self.retreat_city_here_ok(g, u, city, primary, airlift) {
                    push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                    return true;
                }
            }

            if g.group_always_invisible(unit_group(g, u)) {
                break;
            }

            pass += 1;
        }

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, if pass > 0 { MOVE_IGNORE_DANGER } else { 0 }, false, None, None);
            return true;
        }

        if let Some(city) = city {
            if g.city_team(city) == team {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                return true;
            }
        }

        false
    }

    /// The repeated test of `AI_retreatToCity` on the city the unit stands in: ours, in a
    /// primary area if asked, with an airlift if asked, and with no visible enemy.
    fn retreat_city_here_ok(&self, g: &Game, u: UnitId, city: Option<CityId>, primary: bool, airlift: bool) -> bool {
        let Some(city) = city else {
            return false;
        };
        let owner = g.units[u].owner;
        if g.cities[city].owner != owner {
            return false;
        }
        if primary && !self.player_ai_is_primary_area(g, owner, g.city_area(city)) {
            return false;
        }
        if airlift && g.cities[city].max_airlift <= 0 {
            return false;
        }
        !g.plot_is_visible_enemy_unit_for_unit(g.city_plot(city), u)
    }

    /// C++ `bool CvUnitAI::AI_pickup(UnitAITypes eUnitAI)` (CvUnitAI.cpp:16267-16363)
    pub fn unit_ai_pickup(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType) -> bool {
        let cargo_space = g.units[u].cargo_capacity;
        if cargo_space == 0 {
            return false;
        }

        let owner = g.units[u].owner;
        let group = unit_group(g, u);
        let is_assault_sea = g.units[u].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA);

        if let Some(city) = g.map.plots[g.unit_plot(u)].plot_city {
            if g.cities[city].owner == owner {
                let city_plot = g.city_plot(city);
                if g.plot_plot_count(city_plot, Some(Game::puf_is_unit_ai_type), unit_ai.i(), -1, Some(owner), None, None, -1, -1) > 0 {
                    if !is_assault_sea || self.city_ai_is_defended(g, city, -1) {
                        push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_PICKUP), Some(city_plot));
                        return true;
                    }
                }
            }
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_pickup_plot: Option<PlotIdx> = None;

        for loop_city in g.players[owner].cities.clone() {
            let city_plot = g.city_plot(loop_city);
            if self.unit_ai_plot_valid(g, u, city_plot) {
                if !is_assault_sea || self.city_ai_is_defended(g, loop_city, -1) {
                    let mut count = g.plot_plot_count(city_plot, Some(Game::puf_is_unit_ai_type), unit_ai.i(), -1, Some(owner), None, Some(Game::puf_is_finite_range), -1, -1);
                    let mut value = count * 10;

                    if g.city_production_unit_ai(loop_city) == Some(unit_ai) {
                        if let Some(production_unit) = g.city_production_unit(loop_city) {
                            let unit_info = &g.d.units[production_unit];
                            if unit_info.domain_type != Some(DomainType::DOMAIN_AIR) || unit_info.air_range > 0 {
                                value += 1;
                                count += 1;
                            }
                        }
                    }

                    if value > 0 {
                        value += g.cities[loop_city].population;

                        if !g.plot_is_visible_enemy_unit_for_unit(city_plot, u) {
                            if self.player_ai_plot_target_mission_ais(g, owner, city_plot, MissionAIType::MISSIONAI_PICKUP, Some(group), 0) < (count + (cargo_space - 1)) / cargo_space {
                                let mut path_turns = 0;
                                if !g.unit_at_plot(u, city_plot) && g.unit_generate_path(u, city_plot, 0, true, Some(&mut path_turns)) {
                                    value *= 1000;

                                    value /= path_turns + 1;

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = Some(city_plot);
                                        best_pickup_plot = Some(city_plot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_pickup_plot)) = (best_plot, best_pickup_plot) {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, Some(MissionAIType::MISSIONAI_PICKUP), Some(best_pickup_plot));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airOffensiveCity()` (CvUnitAI.cpp:16367-16484)
    pub fn unit_ai_air_offensive_city(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let air_range = g.unit_air_range(u);
        let can_air_attack = g.unit_can_air_attack(u);
        let nuke_range = g.unit_nuke_range(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if !g.plot_is_city(loop_plot, true, Some(team)) {
                continue;
            }
            if g.plot_get_team(loop_plot) != Some(team) {
                // XXX team???
                continue;
            }

            let loop_city = g.map.plots[loop_plot].plot_city;
            let mut valid = false;

            let mut air_base_value = if loop_city.is_some() { 0 } else { self.player_ai_get_plot_airbase_value(g, owner, loop_plot) };
            air_base_value /= 6;

            let mut defenders = g.plot_plot_count(loop_plot, Some(Game::puf_can_defend), -1, -1, Some(owner), None, None, -1, -1);

            if let Some(loop_city) = loop_city {
                if defenders > 2 {
                    valid = true;
                    if !self.city_ai_is_danger(g, loop_city) {
                        defenders += 2;
                    }
                }
            } else if air_base_value > 0 {
                valid = true;
            }

            if valid {
                if g.unit_at_plot(u, loop_plot) || g.unit_can_move_into(u, loop_plot, false, false, false) {
                    // XXX is in danger?
                    let mut value = air_base_value;
                    if let Some(loop_city) = loop_city {
                        value = g.cities[loop_city].population + 20;
                        value += self.city_ai_city_threat(g, loop_city, false);
                    }

                    if g.map.plots[loop_plot].owner == Some(owner) {
                        value *= 3;
                        value /= 2;
                    }

                    value *= 1000;

                    let (lx, ly) = g.map.xy(loop_plot);
                    if let Some(nearest_enemy_city) = g.map_find_city(lx, ly, None, None, false, false, Some(team), None, None) {
                        let distance = g.plot_distance(lx, ly, g.cities[nearest_enemy_city].x, g.cities[nearest_enemy_city].y);
                        if distance > air_range {
                            value /= 10 * (2 + air_range);
                        } else {
                            value /= 2 + distance;
                        }
                    }

                    let mut attack_air_count = g.plot_plot_count(loop_plot, Some(Game::puf_can_air_attack), -1, -1, None, Some(team), None, -1, -1);
                    attack_air_count += 2 * g.plot_plot_count(loop_plot, Some(Game::puf_is_unit_ai_type), UnitAIType::UNITAI_ICBM.i(), -1, None, Some(team), None, -1, -1);
                    if g.unit_at_plot(u, loop_plot) {
                        attack_air_count += if can_air_attack { -1 } else { 0 };
                        attack_air_count += if nuke_range >= 0 { -2 } else { 0 };
                    }

                    if attack_air_count <= defenders {
                        if value > best_value {
                            best_value = value;
                            best_plot = Some(loop_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            if !g.unit_at_plot(u, best_plot) {
                let (bx, by) = g.map.xy(best_plot);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airDefensiveCity()` (CvUnitAI.cpp:16488-16550)
    pub fn unit_ai_air_defensive_city(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;

        if g.unit_can_air_defend(u, None) {
            if let Some(city) = g.map.plots[g.unit_plot(u)].plot_city {
                if g.cities[city].owner == owner {
                    if !self.city_ai_is_air_defended(g, city, -1) {
                        push_mission(g, u, MissionType::MISSION_AIRPATROL, -1, -1, 0, false, None, None);
                        return true;
                    }
                }
            }
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for loop_city in g.players[owner].cities.clone() {
            let city_plot = g.city_plot(loop_city);
            if g.unit_can_air_defend(u, Some(city_plot)) {
                if !self.city_ai_is_air_defended(g, loop_city, 0) {
                    if !g.unit_at_plot(u, city_plot) && g.unit_can_move_into(u, city_plot, false, false, false) {
                        let value = g.cities[loop_city].population + self.city_ai_city_threat(g, loop_city, false);

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(city_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airCarrier()` (CvUnitAI.cpp:16554-16634)
    pub fn unit_ai_air_carrier(&self, g: &mut Game, u: UnitId) -> bool {
        if g.units[u].cargo > 0 {
            return false;
        }

        if g.unit_is_cargo(u) {
            if g.unit_can_air_defend(u, None) {
                push_mission(g, u, MissionType::MISSION_AIRPATROL, -1, -1, 0, false, None, None);
                return true;
            } else {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
                return true;
            }
        }

        let owner = g.units[u].owner;
        let mut best_value = 0;
        let mut best_unit: Option<UnitId> = None;

        for loop_unit in g.players[owner].units.clone() {
            let loop_plot = g.unit_plot(loop_unit);
            if g.unit_can_load_unit(u, loop_unit, loop_plot) {
                let mut value = 10;

                if !g.plot_is_city(loop_plot, false, None) {
                    value += 20;
                }

                if g.plot_is_owned(loop_plot) {
                    if g.unit_is_enemy(u, g.plot_get_team(loop_plot), Some(loop_plot)) {
                        value += 20;
                    }
                } else {
                    value += 10;
                }

                value /= g.units[loop_unit].cargo + 1;

                if value > best_value {
                    best_value = value;
                    best_unit = Some(loop_unit);
                }
            }
        }

        if let Some(best_unit) = best_unit {
            if g.unit_at_plot(u, g.unit_plot(best_unit)) {
                g.unit_set_transport_unit(u, Some(best_unit)); // XXX is this dangerous (not pushing a mission...) XXX air units?
                return true;
            } else {
                let (bx, by) = (g.units[best_unit].x, g.units[best_unit].y);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_missileLoad(UnitAITypes eTargetUnitAI, int iMaxOwnUnitAI = -1, bool bStealthOnly = false)` (CvUnitAI.cpp:16636-16688)
    pub fn unit_ai_missile_load(&self, g: &mut Game, u: UnitId, target_unit_ai: UnitAIType, max_own_unit_ai: i32, stealth_only: bool) -> bool {
        let owner = g.units[u].owner;
        let my_unit_ai = g.units[u].ai_type;

        let mut best_unit: Option<UnitId> = None;
        let mut best_value = 0;
        for loop_unit in g.players[owner].units.clone() {
            if !stealth_only || g.unit_get_invisible_type(loop_unit).is_some() {
                if g.units[loop_unit].ai_type == Some(target_unit_ai) {
                    if max_own_unit_ai == -1 || unit_ai_cargo(g, loop_unit, my_unit_ai) <= max_own_unit_ai {
                        if g.unit_can_load_unit(u, loop_unit, g.unit_plot(loop_unit)) {
                            let mut value = 100;

                            value += self.rand(100, "AI missile load");

                            value *= 1 + g.units[loop_unit].cargo;

                            if value > best_value {
                                best_value = value;
                                best_unit = Some(loop_unit);
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_unit) = best_unit {
            if g.unit_at_plot(u, g.unit_plot(best_unit)) {
                g.unit_set_transport_unit(u, Some(best_unit)); // XXX is this dangerous (not pushing a mission...) XXX air units?
                return true;
            } else {
                let (bx, by) = (g.units[best_unit].x, g.units[best_unit].y);
                push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, None, None);
                // C++ quirk: loads the missile into a transport on another plot.
                g.unit_set_transport_unit(u, Some(best_unit));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airStrike()` (CvUnitAI.cpp:16692-16785)
    pub fn unit_ai_air_strike(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let group = unit_group(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let search_range = g.unit_air_range(u);
        let production_cost = g.d.units[g.units[u].unit_type].production_cost;
        let suicide = g.unit_is_suicide(u);

        let mut best_value = if suicide && production_cost > 0 { (5 * production_cost) / 6 } else { 0 };
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };

                if g.unit_can_move_into(u, loop_plot, true, false, false) {
                    let mut potential_attackers = self.player_ai_adjacent_potential_attackers(g, owner, loop_plot, false);
                    if g.plot_is_city(loop_plot, false, None) {
                        potential_attackers += self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_ASSAULT, Some(group), 1) * 2;
                    }
                    if g.plot_is_water(loop_plot) || potential_attackers > 0 || g.plot_is_adjacent_team(loop_plot, team, false) {
                        // PORT ADAPT: C++ asserts a defender exists (and would crash without one).
                        let Some(defender) = g.plot_get_best_defender(loop_plot, None, Some(owner), Some(u), true, false, false) else {
                            continue;
                        };

                        // XXX factor in air defenses...

                        let damage = g.unit_air_combat_damage(u, defender);
                        let defender_damage = g.units[defender].damage;

                        let mut value = 0.max((defender_damage + damage).min(g.unit_air_combat_limit(u)) - defender_damage);

                        value += ((damage * g.unit_collateral_damage(u)) / 100) * (g.plot_get_num_visible_enemy_defenders(loop_plot, u) - 1).min(g.unit_collateral_damage_max_units(u)) / 2;

                        value *= 3 + potential_attackers;
                        value /= 4;

                        if let Some(interceptor) = g.unit_best_interceptor(u, loop_plot) {
                            let mut intercept_prob = if suicide { 100 } else { g.unit_curr_interception_probability(interceptor) };

                            intercept_prob *= 0.max(100 - g.unit_evasion_probability(u));
                            intercept_prob /= 100;

                            value *= 0.max(100 - intercept_prob / 2);
                            value /= 100;
                        }

                        if g.plot_is_water(loop_plot) {
                            value *= 3;
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(loop_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_MOVE_TO, bx, by, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airBombPlots()` (CvUnitAI.cpp:16787-16873)
    pub fn unit_ai_air_bomb_plots(&self, g: &mut Game, u: UnitId) -> bool {
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let here = g.unit_plot(u);
        let suicide = g.unit_is_suicide(u);

        let search_range = g.unit_air_range(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };

                if !g.plot_is_city(loop_plot, false, None) && g.plot_is_owned(loop_plot) && loop_plot != here {
                    let (lx, ly) = g.map.xy(loop_plot);
                    if g.unit_can_air_bomb_at(u, here, lx, ly) {
                        let mut value = 0;
                        let plot_team = g.plot_get_team(loop_plot);

                        if g.plot_get_bonus_type(loop_plot, plot_team).is_some() {
                            value += self.unit_ai_pillage_value(g, u, loop_plot, 15);

                            value += self.rand(10, "AI Air Bomb");
                        } else if suicide {
                            //This should only be reached when the unit is desperate to die
                            value += self.unit_ai_pillage_value(g, u, loop_plot, 0);
                            // Guided missiles lean towards destroying resource-producing tiles as opposed to improvements like Towns
                            if let Some(bonus) = g.plot_get_bonus_type(loop_plot, plot_team) {
                                //and even more so if it's a resource
                                if let Some(plot_owner) = g.map.plots[loop_plot].owner {
                                    value += self.player_ai_bonus_val(g, plot_owner, bonus, 1);
                                }
                            }
                        }

                        if value > 0 {
                            if let Some(interceptor) = g.unit_best_interceptor(u, loop_plot) {
                                let mut intercept_prob = if suicide { 100 } else { g.unit_curr_interception_probability(interceptor) };

                                intercept_prob *= 0.max(100 - g.unit_evasion_probability(u));
                                intercept_prob /= 100;

                                value *= 0.max(100 - intercept_prob / 2);
                                value /= 100;
                            }

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
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_AIRBOMB, bx, by, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_airBombDefenses()` (CvUnitAI.cpp:16876-16967)
    pub fn unit_ai_air_bomb_defenses(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let group = unit_group(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let here = g.unit_plot(u);
        let suicide = g.unit_is_suicide(u);

        let search_range = g.unit_air_range(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };

                let Some(city) = g.map.plots[loop_plot].plot_city else {
                    continue;
                };

                let mut value = 0;

                let (lx, ly) = g.map.xy(loop_plot);
                if g.unit_can_air_bomb_at(u, here, lx, ly) {
                    let city_plot = g.city_plot(city);
                    let mut potential_attackers = self.player_ai_adjacent_potential_attackers(g, owner, loop_plot, false);
                    potential_attackers += 0.max(plot_target_any_mission_ais(g, owner, city_plot, Some(group), 2) - 4);

                    if potential_attackers > 1 {
                        let defense_damage = g.cities[city].defense_damage;
                        value += 0.max((defense_damage + g.unit_air_bomb_curr_rate(u)).min(g.d.defines.max_city_defense_damage) - defense_damage);

                        value *= 4 + potential_attackers;

                        if self.city_ai_is_danger(g, city) {
                            value *= 2;
                        }

                        if Some(city) == g.area_get_target_city(g.city_area(city), owner) {
                            value *= 2;
                        }
                    }

                    if value > 0 {
                        if let Some(interceptor) = g.unit_best_interceptor(u, loop_plot) {
                            let mut intercept_prob = if suicide { 100 } else { g.unit_curr_interception_probability(interceptor) };

                            intercept_prob *= 0.max(100 - g.unit_evasion_probability(u));
                            intercept_prob /= 100;

                            value *= 0.max(100 - intercept_prob / 2);
                            value /= 100;
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(loop_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            let (bx, by) = g.map.xy(best_plot);
            push_mission(g, u, MissionType::MISSION_AIRBOMB, bx, by, 0, false, None, None);
            return true;
        }

        false
    }
}

/// C++ `getGroup()` of a unit (never NULL for a live unit).
fn unit_group(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnitAI: unit without a selection group")
}

/// C++ `getGroup()->pushMission(eMission, iData1, iData2, iFlags, bAppend, false, eMissionAI,
/// pMissionAIPlot)` (never manual, no mission AI unit).
#[allow(clippy::too_many_arguments)]
fn push_mission(g: &mut Game, u: UnitId, mission: MissionType, data1: i32, data2: i32, flags: i32, append: bool, mission_ai: Option<MissionAIType>, mission_ai_plot: Option<PlotIdx>) {
    let group = unit_group(g, u);
    g.group_push_mission(group, mission, data1, data2, flags, append, false, mission_ai, mission_ai_plot, None);
}

/// C++ `getGroup()->getLengthMissionQueue() > 0`.
fn queue_nonempty(g: &Game, u: UnitId) -> bool {
    !g.groups[unit_group(g, u)].mission_queue.is_empty()
}

/// C++ `getGroup()->pushMission(MISSION_BUILD, eBuild, -1, 0, (getGroup()->getLengthMissionQueue() > 0), false, MISSIONAI_BUILD, pPlot)`.
fn push_build(g: &mut Game, u: UnitId, build: BuildType, plot: PlotIdx) {
    let append = queue_nonempty(g, u);
    push_mission(g, u, MissionType::MISSION_BUILD, build.i(), -1, 0, append, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
}

/// C++ `getPathLastNode()->m_iData1, m_iData2` (moves left, turn) of the last generated path.
fn path_last_node(g: &Game) -> (i32, i32) {
    g.path_finder.borrow().last_node().map_or((0, 0), |n| (n.data1, n.data2))
}

/// The movement cost the worker AI computes inline (`AI_improveCity`, `AI_improveLocalPlot`):
/// the feature's (else the terrain's) movement cost, plus `HILLS_EXTRA_MOVEMENT` on hills.
fn plot_move_cost(g: &Game, plot: PlotIdx) -> i32 {
    let p = &g.map.plots[plot];
    let mut cost = match p.feature_type {
        None => p.terrain_type.map_or(1, |t| g.d.terrains[t].movement_cost),
        Some(f) => g.d.features[f].movement_cost,
    };

    if g.plot_is_hills(plot) {
        cost += g.d.defines.hills_extra_movement;
    }
    cost
}

/// C++ `CvUnit::getUnitAICargo(eUnitAI)` with a possibly NO_UNITAI `eUnitAI`.
fn unit_ai_cargo(g: &Game, u: UnitId, unit_ai: Option<UnitAIType>) -> i32 {
    match unit_ai {
        Some(unit_ai) => g.unit_get_unit_ai_cargo(u, unit_ai),
        None => g.unit_get_cargo_units(u).into_iter().filter(|&c| g.units[c].ai_type.is_none()).count() as i32,
    }
}

/// C++ `CvPlayerAI::AI_plotTargetMissionAIs(pPlot, NO_MISSIONAI, pSkipSelectionGroup, iRange)`:
/// NO_MISSIONAI counts the units of every group whose mission AI plot is within `range` (the
/// shared port takes a `MissionAIType`, which cannot be NO_MISSIONAI).
fn plot_target_any_mission_ais(g: &Game, p: PlayerId, plot: PlotIdx, skip: Option<GroupId>, range: i32) -> i32 {
    let (px, py) = g.map.xy(plot);
    let mut count = 0;
    for &loop_group in &g.players[p].groups {
        if Some(loop_group) == skip {
            continue;
        }
        if let Some(mission_plot) = g.group_ai_get_mission_ai_plot(loop_group) {
            let (mx, my) = g.map.xy(mission_plot);
            if g.step_distance(px, py, mx, my) <= range {
                count += g.groups[loop_group].units.len() as i32;
            }
        }
    }
    count
}
