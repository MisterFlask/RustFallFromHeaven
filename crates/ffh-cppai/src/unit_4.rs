//! Port of CvUnitAI.cpp:8508-11297 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES: none.

#![allow(unused_variables, unused_mut, unused_imports, unused_assignments, clippy::all)]

use crate::prelude::*;

/// C++ `getGroup()` of a unit (the C++ dereferences it: the unit must have a group).
fn group_of(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnit::getGroup: unit has no selection group")
}

/// C++ `pGroup->pushMission(eMission)` with every default argument.
fn push_simple(g: &mut Game, grp: GroupId, mission: MissionType) {
    g.group_push_mission(grp, mission, -1, -1, 0, false, false, None, None, None);
}

/// C++ `pGroup->pushMission(eMission, iData1)` (other arguments default).
fn push_data(g: &mut Game, grp: GroupId, mission: MissionType, data1: i32) {
    g.group_push_mission(grp, mission, data1, -1, 0, false, false, None, None, None);
}

/// C++ `pGroup->pushMission(MISSION_SKIP, -1, -1, 0, false, false, eMissionAI, pMissionAIPlot)`.
fn push_skip_ai(g: &mut Game, grp: GroupId, mission_ai: MissionAIType, ai_plot: Option<PlotIdx>) {
    g.group_push_mission(grp, MissionType::MISSION_SKIP, -1, -1, 0, false, false, Some(mission_ai), ai_plot, None);
}

/// C++ `pGroup->pushMission(eMission, pPlot->getX_INLINE(), pPlot->getY_INLINE(), 0, false, false,
/// eMissionAI, pMissionAIPlot)`.
fn push_to_plot(g: &mut Game, grp: GroupId, mission: MissionType, plot: PlotIdx, mission_ai: Option<MissionAIType>, ai_plot: Option<PlotIdx>) {
    let (x, y) = g.map.xy(plot);
    g.group_push_mission(grp, mission, x, y, 0, false, false, mission_ai, ai_plot, None);
}

/// C++ `pEjectedUnit->getGroup() == pOldGroup` (false if the unit died meanwhile).
fn in_group(g: &Game, u: UnitId, grp: GroupId) -> bool {
    g.units.get(u).map_or(false, |unit| unit.group == Some(grp))
}

/// C++ `isLimitedUnitClass(GC.getUnitInfo(eUnit).getUnitClassType())`.
fn unit_is_limited(g: &Game, unit: UnitType) -> bool {
    g.d.units[unit].unit_class_type.map_or(false, |c| is_limited_unit_class(g.d, c))
}

/// C++ `isLimitedWonderClass(GC.getBuildingInfo(eBuilding).getBuildingClassType())`.
fn building_is_limited(g: &Game, building: BuildingType) -> bool {
    g.d.buildings[building].building_class_type.map_or(false, |c| is_limited_wonder_class(g.d, c))
}

/// C++ `isWorldWonderClass(GC.getBuildingInfo(eBuilding).getBuildingClassType())`.
fn building_is_world_wonder(g: &Game, building: BuildingType) -> bool {
    g.d.buildings[building].building_class_type.map_or(false, |c| is_world_wonder_class(g.d, c))
}

impl Cai {
    /// C++ `bool CvUnitAI::AI_guardCity(bool bLeave = false, bool bSearch = false, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:8509-8758)
    pub fn unit_ai_guard_city(&self, g: &mut Game, u: UnitId, leave: bool, search: bool, max_path: i32) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let city = g.map.plots[plot].plot_city;

        if let Some(city) = city.filter(|&c| g.cities[c].owner == owner) {
            // XXX check for other team?
            let extra = if leave && !self.city_ai_is_danger(g, city) {
                1
            } else if search {
                0
            } else {
                -self.player_ai_get_plot_danger(g, owner, plot, 2, true)
            };

            let mut defend = false;

            if g.plot_plot_count(plot, Some(Game::puf_can_defend_group_head), -1, -1, Some(owner), None, None, -1, -1) == 1 {
                // XXX check for other team's units?
                defend = true;
            } else if !self.city_ai_is_defended(g, city, (if g.unit_ai_is_city_ai_type(u) { -1 } else { 0 }) + extra) {
                // XXX check for other team's units?
                if g.unit_ai_is_city_ai_type(u) {
                    defend = true;
                } else {
                    let mut count = 0;

                    for lu in g.map.plots[plot].units.clone() {
                        if g.units[lu].owner == owner {
                            if g.unit_is_group_head(lu) {
                                if !g.unit_is_cargo(lu) {
                                    if g.unit_can_defend(lu, None) {
                                        if !g.unit_ai_is_city_ai_type(lu) {
                                            if !g.unit_is_hurt(lu) {
                                                if g.unit_is_waiting(lu) {
                                                    count += 1;
                                                }
                                            }
                                        } else {
                                            let lg = group_of(g, lu);
                                            if g.group_get_mission_type(lg, 0) != Some(MissionType::MISSION_SKIP) {
                                                count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !self.city_ai_is_defended(g, city, count + extra) {
                        // XXX check for other team's units?
                        defend = true;
                    }
                }
            }

            if defend {
                let old_group = group_of(g, u);
                let ejected = self.group_ai_eject_best_defender(g, old_group, plot);

                if let Some(ej) = ejected {
                    if g.plot_plot_count(plot, Some(Game::puf_is_city_ai_type), -1, -1, Some(owner), None, None, -1, -1) == 0 {
                        if g.unit_city_defense_modifier(ej) > 0 {
                            g.unit_ai_set_unit_ai_type(ej, UnitAIType::UNITAI_CITY_DEFENSE);
                        }
                    }
                    let eg = group_of(g, ej);
                    push_skip_ai(g, eg, MissionAIType::MISSIONAI_GUARD_CITY, None);
                    return in_group(g, ej, old_group) || ej == u;
                } else {
                    //This unit is not suited for defense, skip the mission
                    //to protect this city but encourage others to defend instead.
                    let grp = group_of(g, u);
                    push_simple(g, grp, MissionType::MISSION_SKIP);
                    if g.units.contains(u) && !g.unit_is_hurt(u) {
                        g.unit_finish_moves(u);
                    }
                }
                return true;
            }
        }

        if search {
            let mut best_value = MAX_INT;
            let mut best_plot: Option<PlotIdx> = None;
            let mut best_guard_plot: Option<PlotIdx> = None;

            let is_city_ai = g.unit_ai_is_city_ai_type(u);
            let grp = group_of(g, u);
            let (ux, uy) = g.map.xy(plot);

            for lc in g.players[owner].cities.clone() {
                let lc_plot = g.city_plot(lc);
                if self.unit_ai_plot_valid(g, u, lc_plot) {
                    let extra = if !is_city_ai {
                        g.plot_plot_count(
                            lc_plot,
                            Some(Game::puf_can_defend_group_head),
                            -1,
                            -1,
                            Some(owner),
                            None,
                            Some(Game::puf_is_not_city_ai_type),
                            -1,
                            -1,
                        )
                    } else {
                        0
                    };
                    // XXX check for other team's units?
                    if !self.city_ai_is_defended(g, lc, extra) {
                        if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                            if (g.game_turn() - g.cities[lc].game_turn_acquired < 10)
                                || self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_GUARD_CITY, Some(grp), 0) < 2
                            {
                                // SPEED TWEAK Sephi: only path to cities roughly within iMaxPath.
                                let (cx, cy) = g.map.xy(lc_plot);
                                let x_dist = cx - ux;
                                let y_dist = cy - uy;
                                // C++ quirk: `iMaxPath*iMaxPath*4` is int arithmetic; for the default
                                // iMaxPath = MAX_INT it wraps to 4, so only cities within one plot
                                // are searched (kept as the DLL played, as in unit_3's AI_group).
                                if x_dist * x_dist + y_dist * y_dist < max_path.wrapping_mul(max_path).wrapping_mul(4) {
                                    if !g.unit_at_plot(u, lc_plot) {
                                        let mut path_turns = 0;
                                        if g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                                            if path_turns <= max_path {
                                                let value = path_turns;
                                                if value < best_value {
                                                    best_value = value;
                                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                                    best_guard_plot = Some(lc_plot);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if best_plot.is_some() {
                        break;
                    }
                }
            }

            if let (Some(best_plot), Some(best_guard_plot)) = (best_plot, best_guard_plot) {
                // split up group if we are going to defend, so rest of group has opportunity to do something else
                let old_group = group_of(g, u);
                let ejected = self.group_ai_eject_best_defender(g, old_group, plot);

                if let Some(ej) = ejected {
                    let eg = group_of(g, ej);
                    push_to_plot(g, eg, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_CITY), None);
                    return in_group(g, ej, old_group) || ej == u;
                } else {
                    //This unit is not suited for defense, skip the mission
                    //to protect this city but encourage others to defend instead.
                    let grp = group_of(g, u);
                    if g.unit_at_plot(u, best_guard_plot) {
                        push_simple(g, grp, MissionType::MISSION_SKIP);
                    } else {
                        push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_CITY), None);
                    }
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardCityAirlift()` (CvUnitAI.cpp:8762-8826)
    pub fn unit_ai_guard_city_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        let grp = group_of(g, u);
        if g.groups[grp].units.len() > 1 {
            return false;
        }

        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let Some(city) = g.map.plots[plot].plot_city else {
            return false;
        };

        if g.cities[city].max_airlift == 0 {
            return false;
        }

        let city_plot = g.city_plot(city);
        let is_city_ai = g.unit_ai_is_city_ai_type(u);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            if lc != city {
                let (lx, ly) = (g.cities[lc].x, g.cities[lc].y);
                if g.unit_can_airlift_at(u, city_plot, lx, ly) {
                    let lc_plot = g.city_plot(lc);
                    let extra = if !is_city_ai {
                        g.plot_plot_count(
                            lc_plot,
                            Some(Game::puf_can_defend_group_head),
                            -1,
                            -1,
                            Some(owner),
                            None,
                            Some(Game::puf_is_not_city_ai_type),
                            -1,
                            -1,
                        )
                    } else {
                        0
                    };
                    // XXX check for other team's units?
                    if !self.city_ai_is_defended(g, lc, extra) {
                        let mut value = g.cities[lc].population;

                        if self.city_ai_is_danger(g, lc) {
                            value *= 2;
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(lc_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_AIRLIFT, best_plot, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardBonus(int iMinValue = 0)` (CvUnitAI.cpp:8830-8935)
    pub fn unit_ai_guard_bonus(&self, g: &mut Game, u: UnitId, min_value: i32) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_guard_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                if g.map.plots[loop_plot].owner == Some(owner) {
                    let non_obsolete_bonus = g.plot_get_non_obsolete_bonus_type(loop_plot, team);

                    //FfH: Modified by Kael 03/22/2008 (ranged-defense improvements are worth
                    // guarding without a bonus)
                    let mut value = 0;
                    if let Some(improvement) = g.map.plots[loop_plot].improvement_type {
                        if let Some(bonus) = non_obsolete_bonus {
                            if g.d.improvements[improvement].improvement_bonus[bonus].bonus_trade {
                                value += self.player_ai_bonus_val(g, owner, bonus, 1);
                                value += 0.max(200 * g.d.bonuses[bonus].ai_objective);
                                if g.plot_get_plot_group_connected_bonus(loop_plot, owner, bonus) == 1 {
                                    value *= 2;
                                }
                            }
                        }
                        let info = &g.d.improvements[improvement];
                        value += info.range_defense_modifier * info.range * 200;
                        //FfH: End Modify

                        if value > min_value {
                            if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                                if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_GUARD_BONUS, Some(grp), 0) == 0 {
                                    let mut path_turns = 0;
                                    if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                        value *= 1000;

                                        value /= path_turns + 1;

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                            best_guard_plot = Some(loop_plot);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_guard_plot)) = (best_plot, best_guard_plot) {
            if g.unit_at_plot(u, best_guard_plot) {
                push_skip_ai(g, grp, MissionAIType::MISSIONAI_GUARD_BONUS, Some(best_guard_plot));
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_BONUS), Some(best_guard_plot));
                return true;
            }
        }

        false
    }

    /// C++ `int CvUnitAI::AI_getPlotDefendersNeeded(CvPlot* pPlot, int iExtra)` (CvUnitAI.cpp:8937-8994)
    pub fn unit_ai_get_plot_defenders_needed(&self, g: &Game, u: UnitId, plot: PlotIdx, extra: i32) -> i32 {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        let mut needed = extra;
        if let Some(bonus) = g.plot_get_non_obsolete_bonus_type(plot, team) {
            needed += (self.player_ai_bonus_val(g, owner, bonus, 1) + 10) / 19;
        }

        let defense = g.plot_defense_modifier(plot, Some(team), true);

        needed += (defense + 25) / 50;

        if needed == 0 {
            return 0;
        }

        needed += self.player_ai_get_plot_airbase_value(g, owner, plot) / 50;

        let mut num_hostiles = 0;
        let mut num_plots = 0;

        let (px, py) = g.map.xy(plot);
        let range = 2;
        for dx in -range..=range {
            for dy in -range..=range {
                if let Some(loop_plot) = g.plot_xy(px, py, dx, dy) {
                    num_hostiles += g.plot_get_num_visible_enemy_defenders(loop_plot, u);
                    let loop_team = g.plot_get_team(loop_plot);
                    if loop_team != Some(team) || g.plot_is_coastal_land(loop_plot, -1) {
                        num_plots += 1;
                        if g.unit_is_enemy(u, loop_team, None) {
                            num_plots += 4;
                        }
                    }
                }
            }
        }

        if num_hostiles == 0 && num_plots < 4 {
            if needed > 1 {
                needed = 1;
            } else {
                needed = 0;
            }
        }

        needed
    }

    /// C++ `bool CvUnitAI::AI_guardFort(bool bSearch = true)` (CvUnitAI.cpp:8996-9085)
    pub fn unit_ai_guard_fort(&self, g: &mut Game, u: UnitId, search: bool) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let grp = group_of(g, u);

        if g.map.plots[plot].owner == Some(owner) {
            if let Some(improvement) = g.map.plots[plot].improvement_type {
                if g.d.improvements[improvement].acts_as_city {
                    if g.plot_plot_count(plot, Some(Game::puf_is_city_ai_type), -1, -1, Some(owner), None, None, -1, -1)
                        <= self.unit_ai_get_plot_defenders_needed(g, u, plot, 0)
                    {
                        push_skip_ai(g, grp, MissionAIType::MISSIONAI_GUARD_BONUS, Some(plot));
                        return true;
                    }
                }
            }
        }

        if !search {
            return false;
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_guard_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) && !g.unit_at_plot(u, loop_plot) {
                if g.map.plots[loop_plot].owner == Some(owner) {
                    if let Some(improvement) = g.map.plots[loop_plot].improvement_type {
                        if g.d.improvements[improvement].acts_as_city {
                            let mut value = self.unit_ai_get_plot_defenders_needed(g, u, loop_plot, 0);

                            if value > 0 {
                                if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                                    if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_GUARD_BONUS, Some(grp), 0) < value {
                                        let mut path_turns = 0;
                                        if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                            value *= 1000;

                                            value /= path_turns + 2;

                                            if value > best_value {
                                                best_value = value;
                                                best_plot = g.unit_get_path_end_turn_plot(u);
                                                best_guard_plot = Some(loop_plot);
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

        if let (Some(best_plot), Some(best_guard_plot)) = (best_plot, best_guard_plot) {
            if g.unit_at_plot(u, best_guard_plot) {
                push_skip_ai(g, grp, MissionAIType::MISSIONAI_GUARD_BONUS, Some(best_guard_plot));
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_BONUS), Some(best_guard_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardCitySite()` (CvUnitAI.cpp:9087-9137)
    pub fn unit_ai_guard_city_site(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_guard_plot: Option<PlotIdx> = None;

        let num_sites = self.player_ai_get_num_city_sites(g, owner);
        for i in 0..num_sites {
            let Some(loop_plot) = self.player_ai_get_city_site(g, owner, i) else {
                continue;
            };
            if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_GUARD_CITY, Some(grp), 0) == 0 {
                let mut path_turns = 0;
                if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                    let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                    let value = g.plot_get_found_value(loop_plot, owner);
                    if value > best_value {
                        best_value = value;
                        best_plot = end_turn_plot;
                        best_guard_plot = Some(loop_plot);
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_guard_plot)) = (best_plot, best_guard_plot) {
            if g.unit_at_plot(u, best_guard_plot) {
                push_skip_ai(g, grp, MissionAIType::MISSIONAI_GUARD_CITY, Some(best_guard_plot));
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_CITY), Some(best_guard_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardSpy(int iRandomPercent)` (CvUnitAI.cpp:9142-9225)
    pub fn unit_ai_guard_spy(&self, g: &mut Game, u: UnitId, random_percent: i32) -> bool {
        let owner = g.units[u].owner;
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_guard_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) {
                if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                    let mut value = 0;
                    if g.city_is_production_unit(lc) {
                        if g.city_production_unit(lc).map_or(false, |unit| unit_is_limited(g, unit)) {
                            value = 4;
                        }
                    } else if g.city_is_production_building(lc) {
                        if g.city_production_building(lc).map_or(false, |b| building_is_limited(g, b)) {
                            value = 5;
                        }
                    } else if g.city_is_production_project(lc) {
                        if g.city_production_project(lc).map_or(false, |p| is_limited_project(g.d, p)) {
                            value = 6;
                        }
                    }
                    if value > 0 {
                        if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_GUARD_SPY, Some(grp), 0) == 0 {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                                value *= 100 + self.rand(random_percent, "AI Guard Spy");
                                //iValue /= 100;
                                value /= path_turns + 1;

                                if value > best_value {
                                    best_value = value;
                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                    best_guard_plot = Some(lc_plot);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_guard_plot)) = (best_plot, best_guard_plot) {
            if g.unit_at_plot(u, best_guard_plot) {
                push_skip_ai(g, grp, MissionAIType::MISSIONAI_GUARD_SPY, Some(best_guard_plot));
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GUARD_SPY), Some(best_guard_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_destroySpy()` (CvUnitAI.cpp:9229-9333)
    pub fn unit_ai_destroy_spy(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_city: Option<CityId> = None;

        for i in g.civ_player_ids() {
            if g.players[i].alive {
                if g.players[i].team != team {
                    if self.player_ai_get_attitude(g, owner, i, true) <= AttitudeType::ATTITUDE_ANNOYED {
                        for lc in g.players[i].cities.clone() {
                            let lc_plot = g.city_plot(lc);
                            if self.unit_ai_plot_valid(g, u, lc_plot) {
                                if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_ATTACK_SPY, Some(grp), 0) == 0 {
                                    if g.unit_generate_path(u, lc_plot, 0, true, None) {
                                        let mut value = g.cities[lc].population * 2;

                                        value += g.city_yield_rate(lc, YieldType::YIELD_PRODUCTION);

                                        if g.unit_at_plot(u, lc_plot) {
                                            value *= 4;
                                            value /= 3;
                                        }

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                            best_city = Some(lc);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_city)) = (best_plot, best_city) {
            let best_city_plot = g.city_plot(best_city);
            if g.unit_at_plot(u, best_city_plot) {
                if g.unit_can_destroy(u, best_city_plot, false) {
                    if g.city_production(best_city) > (g.city_production_needed(best_city) * 2) / 3 {
                        if g.city_is_production_unit(best_city) {
                            if g.city_production_unit(best_city).map_or(false, |unit| unit_is_limited(g, unit)) {
                                push_simple(g, grp, MissionType::MISSION_DESTROY);
                                return true;
                            }
                        } else if g.city_is_production_building(best_city) {
                            if g.city_production_building(best_city).map_or(false, |b| building_is_limited(g, b)) {
                                push_simple(g, grp, MissionType::MISSION_DESTROY);
                                return true;
                            }
                        } else if g.city_is_production_project(best_city) {
                            if g.city_production_project(best_city).map_or(false, |p| is_limited_project(g.d, p)) {
                                push_simple(g, grp, MissionType::MISSION_DESTROY);
                                return true;
                            }
                        }
                    }
                }

                push_skip_ai(g, grp, MissionAIType::MISSIONAI_ATTACK_SPY, Some(best_city_plot));
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_ATTACK_SPY), Some(best_city_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_sabotageSpy()` (CvUnitAI.cpp:9337-9440)
    pub fn unit_ai_sabotage_spy(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);

        let mut player_angry: Vec<bool> = vec![false; g.num_players()];
        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team != team {
                    if self.player_ai_get_attitude(g, owner, i, true) <= AttitudeType::ATTITUDE_ANNOYED {
                        player_angry[i.idx()] = true;
                    }
                }
            }
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_sabotage_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                let Some(plot_owner) = g.map.plots[loop_plot].owner else {
                    continue;
                };
                let Some(plot_team) = g.plot_get_team(loop_plot) else {
                    continue;
                };
                if plot_team != team {
                    if player_angry[plot_owner.idx()] {
                        if let Some(bonus) = g.plot_get_non_obsolete_bonus_type(loop_plot, plot_team) {
                            let improvement = g.map.plots[loop_plot].improvement_type;

                            if improvement.map_or(false, |imp| g.d.improvements[imp].improvement_bonus[bonus].bonus_trade) {
                                if g.unit_can_sabotage(u, loop_plot, false) {
                                    let mut value = self.player_ai_bonus_val(g, plot_owner, bonus, 1);

                                    if g.plot_is_connected_to_capital(loop_plot, None)
                                        && g.plot_get_plot_group_connected_bonus(loop_plot, plot_owner, bonus) == 1
                                    {
                                        value *= 3;
                                    }

                                    if value > 25 {
                                        if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_ATTACK_SPY, Some(grp), 0) == 0 {
                                            if g.unit_generate_path(u, loop_plot, 0, true, None) {
                                                if value > best_value {
                                                    best_value = value;
                                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                                    best_sabotage_plot = Some(loop_plot);
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

        if let (Some(best_plot), Some(best_sabotage_plot)) = (best_plot, best_sabotage_plot) {
            if g.unit_at_plot(u, best_sabotage_plot) {
                push_simple(g, grp, MissionType::MISSION_SABOTAGE);
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_ATTACK_SPY), Some(best_sabotage_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_pickupTargetSpy()` (CvUnitAI.cpp:9443-9507)
    pub fn unit_ai_pickup_target_spy(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let grp = group_of(g, u);
        let min_water = g.d.defines.min_water_size_for_ocean;

        if let Some(city) = g.map.plots[plot].plot_city {
            if g.cities[city].owner == owner {
                if g.city_is_coastal(city, min_water) {
                    let city_plot = g.city_plot(city);
                    push_skip_ai(g, grp, MissionAIType::MISSIONAI_ATTACK_SPY, Some(city_plot));
                    return true;
                }
            }
        }

        let mut best_value = MAX_INT;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_pickup_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) {
                if g.city_is_coastal(lc, min_water) {
                    if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                        let mut path_turns = 0;
                        if !g.unit_at_plot(u, lc_plot) && g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                            let value = path_turns;

                            if value < best_value {
                                best_value = value;
                                best_plot = g.unit_get_path_end_turn_plot(u);
                                best_pickup_plot = Some(lc_plot);
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_pickup_plot)) = (best_plot, best_pickup_plot) {
            push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_ATTACK_SPY), Some(best_pickup_plot));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_chokeDefend()` (CvUnitAI.cpp:9511-9545)
    pub fn unit_ai_choke_defend(&self, g: &mut Game, u: UnitId) -> bool {
        // XXX what about amphib invasions?
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);

        if let Some(city) = g.map.plots[plot].plot_city {
            if g.cities[city].owner == owner {
                if self.city_ai_needed_defenders(g, city) > 1 {
                    let city_plot = g.city_plot(city);
                    let count = g.plot_plot_count(
                        city_plot,
                        Some(Game::puf_can_defend_group_head),
                        -1,
                        -1,
                        Some(owner),
                        None,
                        Some(Game::puf_is_not_city_ai_type),
                        -1,
                        -1,
                    );
                    if self.city_ai_is_defended(g, city, count) {
                        let plot_danger = self.player_ai_get_plot_danger(g, owner, plot, 3, true);

                        if plot_danger <= 4 {
                            if self.unit_ai_any_attack(g, u, 1, 65, 0.max(plot_danger - 1), false) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_heal(int iDamagePercent = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:9549-9691)
    pub fn unit_ai_heal(&self, g: &mut Game, u: UnitId, damage_percent: i32, max_path: i32) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);

        if let Some(feature) = g.map.plots[plot].feature_type {
            if g.d.features[feature].turn_damage != 0 {
                //Pass through
                //(actively seeking a safe spot may result in unit getting stuck)
                return false;
            }
        }

        //FfH: Added by Kael 10/01/2007 (so the AI won't try to sit and heal in areas where they cant heal)
        if g.unit_heal_rate(u, plot) == 0 {
            return false;
        }
        //FfH: End Add

        let group = group_of(g, u);

        let mut damage_percent = damage_percent;
        if damage_percent == 0 {
            damage_percent = 10;
        }

        let mut retreat = false;

        if g.groups[group].units.len() == 1 {
            if g.units[u].damage > 0 {
                if g.plot_is_city(plot, false, None) || g.unit_heal_turns(u, plot) == 1 {
                    if !g.unit_is_always_heal(u) {
                        push_simple(g, group, MissionType::MISSION_HEAL);
                        return true;
                    }
                }
            }
            return false;
        }

        let max_path = max_path.min(2);

        let mut damaged_units: Vec<UnitId> = Vec::new();
        let mut total_damage = 0;
        let mut total_hitpoints = 0;
        let mut hurt_unit_count = 0;
        let has_leader = g.units[u].leader_unit_type.is_some();

        for lu in g.groups[group].units.clone() {
            let mut damage_threshold = (g.unit_max_hit_points(lu) * damage_percent) / 100;

            if has_leader {
                damage_threshold /= 2;
            }

            let damage = g.units[lu].damage;
            if damage > 0 {
                hurt_unit_count += 1;
            }
            total_damage += damage;
            total_hitpoints += g.unit_max_hit_points(lu);

            if damage > damage_threshold {
                retreat = true;

                if !g.unit_has_moved(lu) {
                    if !g.unit_is_always_heal(lu) {
                        let lu_plot = g.unit_plot(lu);
                        if g.unit_heal_turns(lu, lu_plot) <= max_path {
                            damaged_units.push(lu);
                        }
                    }
                }
            }
        }
        if hurt_unit_count == 0 {
            return false;
        }

        let mut pushed_mission = false;
        if g.plot_is_city(plot, false, None) && g.map.plots[plot].owner == Some(owner) {
            for unit_to_heal in damaged_units {
                if !g.units.contains(unit_to_heal) {
                    continue;
                }
                g.unit_join_group(unit_to_heal, None, true);
                if let Some(heal_group) = g.units[unit_to_heal].group {
                    push_simple(g, heal_group, MissionType::MISSION_HEAL);
                }

                // note, removing the head unit from a group will force the group to be completely split if non-human
                if unit_to_heal == u {
                    pushed_mission = true;
                }

                hurt_unit_count -= 1;
            }
        }

        // C++ reads the old group's size (the group is gone if every unit left it).
        let group_units = if g.groups.contains(group) { g.groups[group].units.len() as i32 } else { 0 };
        if hurt_unit_count * 2 > group_units {
            if !g.units.contains(u) {
                return pushed_mission;
            }
            if self.unit_ai_move_into_city(g, u, 2) {
                return true;
            } else if g.units.contains(u) && g.unit_heal_rate(u, g.unit_plot(u)) > 10 {
                if g.groups.contains(group) {
                    push_simple(g, group, MissionType::MISSION_HEAL);
                    return true;
                }
            }
        }

        pushed_mission
    }

    /// C++ `bool CvUnitAI::AI_afterAttack()` (CvUnitAI.cpp:9695-9756)
    pub fn unit_ai_after_attack(&self, g: &mut Game, u: UnitId) -> bool {
        if !g.units[u].made_attack {
            return false;
        }

        if !g.unit_can_fight(u) {
            return false;
        }

        if g.unit_is_blitz(u) {
            return false;
        }

        if g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND {
            if self.unit_ai_guard_city(g, u, false, true, 1) {
                return true;
            }
        }

        if self.unit_ai_pillage_range(g, u, 1, 0) {
            return true;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, 1) {
            return true;
        }

        if self.unit_ai_hide(g, u) {
            return true;
        }

        if self.unit_ai_goody(g, u, 1) {
            return true;
        }

        if self.unit_ai_pillage_range(g, u, 2, 0) {
            return true;
        }

        if self.unit_ai_defend(g, u) {
            return true;
        }

        if self.unit_ai_safety(g, u) {
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_goldenAge()` (CvUnitAI.cpp:9760-9769)
    pub fn unit_ai_golden_age(&self, g: &mut Game, u: UnitId) -> bool {
        let plot = g.unit_plot(u);
        if g.unit_can_golden_age(u, plot, false) {
            let grp = group_of(g, u);
            push_simple(g, grp, MissionType::MISSION_GOLDEN_AGE);
            return true;
        }

        false
    }

    /// The religion a missionary-type unit spreads (C++ preamble of `AI_spreadReligion` and
    /// `AI_spreadReligionAirlift`): the state religion if the unit spreads it, else the first
    /// religion it spreads.
    fn unit_spread_religion_choice(&self, g: &Game, u: UnitId) -> Option<ReligionType> {
        let owner = g.units[u].owner;
        let info = &g.d.units[g.units[u].unit_type];
        let mut religion = None;

        if let Some(state) = g.player_get_state_religion(owner) {
            if info.religion_spreads[state] > 0 {
                religion = Some(state);
            }
        }

        if religion.is_none() {
            for r in g.d.religions.keys() {
                //if (bCultureVictory || GET_TEAM(getTeam()).hasHolyCity((ReligionTypes)iI))
                if info.religion_spreads[r] > 0 {
                    religion = Some(r);
                    break;
                }
            }
        }

        religion
    }

    /// C++ `bool CvUnitAI::AI_spreadReligion()` (CvUnitAI.cpp:9773-10019)
    pub fn unit_ai_spread_religion(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);
        let plot = g.unit_plot(u);
        let area = g.unit_area(u);

        let culture_victory = self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE2);

        let Some(religion) = self.unit_spread_religion_choice(g, u) else {
            return false;
        };

        let has_holy_city = g.team_has_holy_city(team, religion);
        let mut has_any_holy_city = has_holy_city;
        if !has_any_holy_city {
            for r in g.d.religions.keys() {
                if has_any_holy_city {
                    break;
                }
                has_any_holy_city = g.team_has_holy_city(team, r);
            }
        }

        let our_state_religion = g.player_get_state_religion(owner);
        let is_human = g.unit_is_human(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_spread_plot: Option<PlotIdx> = None;

        for i in g.player_ids() {
            if g.players[i].alive {
                let mut player_multiplier_percent = 0;

                if g.players[i].team != team {
                    if has_holy_city {
                        player_multiplier_percent = 100;
                        if !culture_victory || (Some(religion) == our_state_religion && has_holy_city) {
                            match g.player_get_state_religion(i) {
                                None => {
                                    if g.players[i].non_state_religion_happiness == 0 {
                                        player_multiplier_percent += 600;
                                    }
                                }
                                Some(their_state) if their_state == religion => {
                                    player_multiplier_percent += 300;
                                }
                                Some(their_state) => {
                                    if g.player_has_holy_city(i, their_state) {
                                        player_multiplier_percent += 50;
                                    } else {
                                        player_multiplier_percent += 300;
                                    }
                                }
                            }

                            let religion_count = g.player_count_total_has_religion(i);
                            let city_count = g.players[owner].num_cities();
                            //magic formula to produce normalized adjustment factor based on religious infusion
                            let mut adjustment = 100 * (city_count + 1);
                            adjustment /= (city_count + 1) + religion_count;
                            adjustment = ((adjustment - 25) * 4) / 3;

                            adjustment = 10.max(adjustment);

                            player_multiplier_percent *= adjustment;
                            player_multiplier_percent /= 100;

                            // if we have a holy city, but not this holy city, then we will likely switch
                            // religions soon, so try to spread this religion internally, not externally
                            if has_any_holy_city && !has_holy_city {
                                player_multiplier_percent /= 10;
                            }
                        }
                    }
                } else if i == owner {
                    player_multiplier_percent = 100;
                } else if has_holy_city {
                    player_multiplier_percent = 80;
                }

                if player_multiplier_percent > 0 {
                    for lc in g.players[i].cities.clone() {
                        let lc_plot = g.city_plot(lc);
                        if self.unit_ai_plot_valid(g, u, lc_plot) && g.city_area(lc) == area {
                            if g.unit_can_spread(u, lc_plot, Some(religion), false) {
                                if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                                    if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_SPREAD, Some(grp), 0) == 0 {
                                        let mut path_turns = 0;
                                        if g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                                            let mut value = 7 + g.cities[lc].population * 4;

                                            let mut our_city = false;
                                            if g.cities[lc].owner == owner {
                                                value *= if culture_victory { 16 } else { 4 };
                                                our_city = true;
                                            } else if g.city_team(lc) == team {
                                                value *= 3;
                                                our_city = true;
                                            } else {
                                                value *= player_multiplier_percent;
                                                value /= 100;
                                            }

                                            let city_religion_count = g.city_religion_count(lc);
                                            let mut religion_count_factor = city_religion_count;

                                            if our_city {
                                                // count cities with no religion the same as cities with 2 religions
                                                // prefer a city with exactly 1 religion already
                                                if city_religion_count == 0 {
                                                    religion_count_factor = 2;
                                                } else if city_religion_count == 1 {
                                                    value *= 2;
                                                }
                                            } else {
                                                // absolutely prefer cities with zero religions
                                                if city_religion_count == 0 {
                                                    value *= 2;
                                                }

                                                // not our city, so prefer the lowest number of religions (increment so no divide by zero)
                                                religion_count_factor += 1;
                                            }

                                            value /= religion_count_factor;

                                            let mut force_move = false;
                                            if is_human {
                                                //If human, prefer to spread to the player where automated from.
                                                if g.map.plots[plot].owner == Some(g.cities[lc].owner) {
                                                    value *= 10;
                                                    if g.city_is_revealed(lc, team, false) {
                                                        force_move = true;
                                                    }
                                                }
                                            }

                                            value *= 1000;

                                            value /= path_turns + 2;

                                            if value > best_value {
                                                best_value = value;
                                                best_plot = if force_move { Some(lc_plot) } else { g.unit_get_path_end_turn_plot(u) };
                                                best_spread_plot = Some(lc_plot);
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

        if let (Some(best_plot), Some(best_spread_plot)) = (best_plot, best_spread_plot) {
            if g.unit_at_plot(u, best_spread_plot) {
                push_data(g, grp, MissionType::MISSION_SPREAD, religion.i());
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_SPREAD), Some(best_spread_plot));
                return true;
            }
        }

        false
    }

    /// The first corporation the unit spreads (C++ preamble of the corporation spread AIs).
    fn unit_spread_corporation_choice(&self, g: &Game, u: UnitId) -> Option<CorporationType> {
        let info = &g.d.units[g.units[u].unit_type];
        g.d.corporations.keys().find(|&c| info.corporation_spreads[c] > 0)
    }

    /// C++ `bool CvUnitAI::AI_spreadCorporation()` (CvUnitAI.cpp:10023-10162)
    pub fn unit_ai_spread_corporation(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);
        let plot = g.unit_plot(u);

        let Some(corporation) = self.unit_spread_corporation_choice(g, u) else {
            return false;
        };
        // C++ quirk: `hasHeadquarters((CorporationTypes)iI)` reads the first loop's index after
        // the loop (MSVC for-scope), which is the corporation found.
        let has_hq = g.team_has_headquarters(team, corporation);

        let is_human = g.unit_is_human(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_spread_plot: Option<PlotIdx> = None;

        for i in g.player_ids() {
            let loop_team = g.players[i].team;
            if g.players[i].alive && (has_hq || team == loop_team) {
                let loop_player_corp_count = g.player_count_corporations(i, corporation);
                for lc in g.players[i].cities.clone() {
                    let lc_plot = g.city_plot(lc);
                    if self.unit_ai_plot_valid(g, u, lc_plot) {
                        if g.unit_can_spread_corporation(u, lc_plot, Some(corporation), false) {
                            if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                                if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_SPREAD_CORPORATION, Some(grp), 0) == 0 {
                                    let mut path_turns = 0;
                                    if g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                                        let mut value = 10 + g.cities[lc].population * 2;

                                        if g.cities[lc].owner == owner {
                                            value *= 4;
                                        } else if g.teams[loop_team].vassal[team] {
                                            value *= 3;
                                        } else if g.teams[team].vassal[loop_team] {
                                            if loop_player_corp_count == 0 {
                                                value *= 10;
                                            } else {
                                                value *= 3;
                                                value /= 2;
                                            }
                                        } else if g.city_team(lc) == team {
                                            value *= 2;
                                        }

                                        if loop_player_corp_count == 0 {
                                            //Generally prefer to heavily target one player
                                            value /= 2;
                                        }

                                        let mut force_move = false;
                                        if is_human {
                                            //If human, prefer to spread to the player where automated from.
                                            if g.map.plots[plot].owner == Some(g.cities[lc].owner) {
                                                value *= 10;
                                                if g.city_is_revealed(lc, team, false) {
                                                    force_move = true;
                                                }
                                            }
                                        }

                                        value *= 1000;

                                        value /= path_turns + 1;

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = if force_move { Some(lc_plot) } else { g.unit_get_path_end_turn_plot(u) };
                                            best_spread_plot = Some(lc_plot);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_spread_plot)) = (best_plot, best_spread_plot) {
            if g.unit_at_plot(u, best_spread_plot) {
                if g.unit_can_spread_corporation(u, best_spread_plot, Some(corporation), false) {
                    push_data(g, grp, MissionType::MISSION_SPREAD_CORPORATION, corporation.i());
                } else {
                    push_skip_ai(g, grp, MissionAIType::MISSIONAI_SPREAD_CORPORATION, Some(best_spread_plot));
                }
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_SPREAD_CORPORATION), Some(best_spread_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_spreadReligionAirlift()` (CvUnitAI.cpp:10164-10278)
    pub fn unit_ai_spread_religion_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        let grp = group_of(g, u);
        if g.groups[grp].units.len() > 1 {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let Some(city) = g.map.plots[plot].plot_city else {
            return false;
        };

        if g.cities[city].max_airlift == 0 {
            return false;
        }

        //bool bCultureVictory = GET_PLAYER(getOwnerINLINE()).AI_isDoStrategy(AI_STRATEGY_CULTURE2);
        let Some(religion) = self.unit_spread_religion_choice(g, u) else {
            return false;
        };

        let city_plot = g.city_plot(city);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in g.player_ids() {
            if g.players[i].alive && team == g.players[i].team {
                for lc in g.players[i].cities.clone() {
                    let (lx, ly) = (g.cities[lc].x, g.cities[lc].y);
                    if g.unit_can_airlift_at(u, city_plot, lx, ly) {
                        let lc_plot = g.city_plot(lc);
                        if g.unit_can_spread(u, lc_plot, Some(religion), false) {
                            if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_SPREAD, Some(grp), 0) == 0 {
                                let mut value = 7 + g.cities[lc].population * 4;

                                let city_religion_count = g.city_religion_count(lc);
                                let mut religion_count_factor = city_religion_count;

                                // count cities with no religion the same as cities with 2 religions
                                // prefer a city with exactly 1 religion already
                                if city_religion_count == 0 {
                                    religion_count_factor = 2;
                                } else if city_religion_count == 1 {
                                    value *= 2;
                                }

                                value /= religion_count_factor;
                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(lc_plot);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_AIRLIFT, best_plot, Some(MissionAIType::MISSIONAI_SPREAD), Some(best_plot));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_spreadCorporationAirlift()` (CvUnitAI.cpp:10280-10364)
    pub fn unit_ai_spread_corporation_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        let grp = group_of(g, u);
        if g.groups[grp].units.len() > 1 {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let Some(city) = g.map.plots[plot].plot_city else {
            return false;
        };

        if g.cities[city].max_airlift == 0 {
            return false;
        }

        let Some(corporation) = self.unit_spread_corporation_choice(g, u) else {
            return false;
        };

        let city_plot = g.city_plot(city);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in g.player_ids() {
            if g.players[i].alive && team == g.players[i].team {
                for lc in g.players[i].cities.clone() {
                    let (lx, ly) = (g.cities[lc].x, g.cities[lc].y);
                    if g.unit_can_airlift_at(u, city_plot, lx, ly) {
                        let lc_plot = g.city_plot(lc);
                        if g.unit_can_spread_corporation(u, lc_plot, Some(corporation), false) {
                            if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_SPREAD_CORPORATION, Some(grp), 0) == 0 {
                                let mut value = g.cities[lc].population * 4;

                                if g.cities[lc].owner == owner {
                                    value *= 4;
                                } else {
                                    value *= 3;
                                }

                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(lc_plot);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_AIRLIFT, best_plot, Some(MissionAIType::MISSIONAI_SPREAD), Some(best_plot));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_discover(bool bThisTurnOnly = false, bool bFirstResearchOnly = false)` (CvUnitAI.cpp:10367-10418)
    pub fn unit_ai_discover(&self, g: &mut Game, u: UnitId, this_turn_only: bool, first_research_only: bool) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);

        if g.unit_can_discover(u, plot) {
            // PORT ADAPT: canDiscover implies a discovery tech; without one there is nothing to do.
            let Some(discover_tech) = g.unit_get_discovery_tech(u) else {
                return false;
            };
            let is_first_tech = self.player_ai_is_first_tech(g, owner, discover_tech);

            if first_research_only && !is_first_tech {
                return false;
            }

            let discover_research = g.unit_get_discover_research(u, Some(discover_tech));
            // FIX: the C++ divides by getDiscoverResearch(NO_TECH) unguarded.
            let percent_wasted = 100 - (discover_research * 100) / g.unit_get_discover_research(u, None).max(1);

            if discover_research >= g.team_get_research_left(team, discover_tech) {
                if percent_wasted < 51 && first_research_only && is_first_tech {
                    let grp = group_of(g, u);
                    push_simple(g, grp, MissionType::MISSION_DISCOVER);
                    return true;
                }

                if percent_wasted < (if is_first_tech { 31 } else { 11 }) {
                    //I need a good way to assess if the tech is actually valuable...
                    //but don't have one.
                    let grp = group_of(g, u);
                    push_simple(g, grp, MissionType::MISSION_DISCOVER);
                    return true;
                }
            } else if this_turn_only {
                return false;
            }

            if percent_wasted <= 11 {
                if g.player_get_current_research(owner) == Some(discover_tech) {
                    let grp = group_of(g, u);
                    push_simple(g, grp, MissionType::MISSION_DISCOVER);
                    return true;
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_lead(std::vector<UnitAITypes>& aeUnitAITypes)` (CvUnitAI.cpp:10421-10528)
    /// (`unit_ai_types`: C++ `UnitAITypes` values, -1 = `NO_UNITAI`.)
    pub fn unit_ai_lead(&self, g: &mut Game, u: UnitId, unit_ai_types: &mut Vec<i32>) -> bool {
        let owner = g.units[u].owner;
        let area = g.unit_area(u);
        let grp = group_of(g, u);

        let mut need_leader = false;
        for t in g.civ_team_ids() {
            if g.unit_is_enemy(u, Some(t), None) {
                if g.team_count_num_units_by_area(t, area) > 0 {
                    need_leader = true;
                    break;
                }
            }
        }

        let mut best_unit: Option<UnitId> = None;
        let mut best_plot: Option<PlotIdx> = None;

        // AI may use Warlords to create super-medic units
        let mut best_str_unit: Option<UnitId> = None;
        let mut best_str_plot: Option<PlotIdx> = None;

        let mut best_heal_unit: Option<UnitId> = None;
        let mut best_heal_plot: Option<PlotIdx> = None;

        if need_leader {
            let mut best_strength = 0;
            let mut best_healing = 0;
            for lu in g.players[owner].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                let lu_ai = g.units[lu].ai_type.map_or(-1, |t| t.i());
                for &ai_type in unit_ai_types.iter() {
                    if lu_ai == ai_type || ai_type == -1 {
                        // PORT ADAPT: C++ would crash on a unit off the map; skip it.
                        let Some(lu_plot) = g.unit_plot_opt(lu) else {
                            break;
                        };
                        if g.unit_can_lead(u, lu_plot, Some(lu)) != 0 {
                            if self.unit_ai_plot_valid(g, u, lu_plot) {
                                if !g.plot_is_visible_enemy_unit_for_unit(lu_plot, u) {
                                    if g.unit_generate_path(u, lu_plot, 0, true, None) {
                                        // pick the unit with the highest current strength
                                        let combat_strength = g.unit_curr_combat_str(lu, None, None, None);
                                        if combat_strength > best_strength {
                                            best_strength = combat_strength;
                                            best_str_unit = Some(lu);
                                            best_str_plot = g.unit_get_path_end_turn_plot(u);
                                        }

                                        // or the unit with the best healing ability
                                        let healing = g.units[lu].same_tile_heal + g.units[lu].adjacent_tile_heal;
                                        if healing > best_healing {
                                            best_healing = healing;
                                            best_heal_unit = Some(lu);
                                            best_heal_plot = g.unit_get_path_end_turn_plot(u);
                                        }

                                        if self.rand(3, "AI Warlord mash unit") != 0 {
                                            best_plot = best_str_plot;
                                            best_unit = best_str_unit;
                                        } else {
                                            best_plot = best_heal_plot;
                                            best_unit = best_heal_unit;
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            match best_unit {
                Some(best_unit) if g.unit_at_plot(u, best_plot) => {
                    push_data(g, grp, MissionType::MISSION_LEAD, best_unit.0 as i32);
                }
                _ => {
                    push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, None, None);
                }
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_join(int iMaxCount = MAX_INT)` (CvUnitAI.cpp:10532-10610)
    pub fn unit_ai_join(&self, g: &mut Game, u: UnitId, max_count: i32) -> bool {
        // iMaxCounts = 1 would mean join a city if there's no existing joined GP of that type.
        let owner = g.units[u].owner;
        let unit_type = g.units[u].unit_type;

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_specialist: Option<SpecialistType> = None;
        let mut count = 0;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) {
                if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                    if g.unit_generate_path(u, lc_plot, 0, true, None) {
                        let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                        for specialist in g.d.specialists.keys() {
                            let does_join = g.d.units[unit_type].great_peoples[specialist];
                            if does_join {
                                count += g.cities[lc].specialist_count[specialist];
                                if count >= max_count {
                                    return false;
                                }
                            }

                            if g.unit_can_join(u, lc_plot, Some(specialist)) {
                                if self.player_ai_get_plot_danger(g, owner, lc_plot, 2, true) == 0 {
                                    let avoid_growth = self.city_ai_avoid_growth(g, lc);
                                    let value = self.city_ai_specialist_value(g, lc, specialist, avoid_growth, false);
                                    if value > best_value {
                                        best_value = value;
                                        best_plot = end_turn_plot;
                                        best_specialist = Some(specialist);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_specialist)) = (best_plot, best_specialist) {
            let grp = group_of(g, u);
            if g.unit_at_plot(u, best_plot) {
                push_data(g, grp, MissionType::MISSION_JOIN, best_specialist.i());
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_construct(int iMaxCount = MAX_INT, int iMaxSingleBuildingCount = MAX_INT, int iThreshold = 15)` (CvUnitAI.cpp:10615-10722)
    pub fn unit_ai_construct(&self, g: &mut Game, u: UnitId, max_count: i32, max_single_building_count: i32, threshold: i32) -> bool {
        // iMaxCount = 1 would mean construct only if there are no existing buildings
        //   constructed by this GP type.
        let owner = g.units[u].owner;
        let unit_type = g.units[u].unit_type;
        let civ = g.unit_get_civilization_type(u);
        let area = g.unit_area(u);
        let grp = group_of(g, u);
        let is_prophet = g.units[u].ai_type == Some(UnitAIType::UNITAI_PROPHET);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_construct_plot: Option<PlotIdx> = None;
        let mut best_building: Option<BuildingType> = None;
        let mut count = 0;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) && g.city_area(lc) == area {
                if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                    if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_CONSTRUCT, Some(grp), 0) == 0 {
                        if g.unit_generate_path(u, lc_plot, 0, true, None) {
                            let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                            for bc in g.d.building_classes.keys() {
                                let Some(building) = g.d.civ_building(civ, bc) else {
                                    continue;
                                };

                                let info = &g.d.units[unit_type];
                                let does_build = info.force_buildings[building] || info.buildings[building];

                                if does_build && g.city_num_building(lc, building) > 0 {
                                    count += 1;
                                    if count >= max_count {
                                        return false;
                                    }
                                }

                                let class_count = g.d.buildings[building]
                                    .building_class_type
                                    .map_or(0, |c| g.players[owner].building_class_count[c]);
                                if does_build && class_count < max_single_building_count {
                                    if g.unit_can_construct(u, lc_plot, Some(building), false) {
                                        let mut value = self.city_ai_building_value(g, lc, building, 0);

                                        // BETTER AI (Religion Victory) Sephi 0.41k
                                        if is_prophet {
                                            if g.city_is_capital(lc) {
                                                value += 10000;
                                            }
                                        }

                                        if value > threshold && value > best_value {
                                            best_value = value;
                                            best_plot = end_turn_plot;
                                            best_construct_plot = Some(lc_plot);
                                            best_building = Some(building);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_construct_plot), Some(best_building)) = (best_plot, best_construct_plot, best_building) {
            if g.unit_at_plot(u, best_construct_plot) {
                push_data(g, grp, MissionType::MISSION_CONSTRUCT, best_building.i());
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_CONSTRUCT), Some(best_construct_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_switchHurry()` (CvUnitAI.cpp:10726-10789)
    pub fn unit_ai_switch_hurry(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let civ = g.unit_get_civilization_type(u);

        let Some(city) = g.map.plots[plot].plot_city.filter(|&c| g.cities[c].owner == owner) else {
            return false;
        };

        let mut best_value = 0;
        let mut best_building: Option<BuildingType> = None;

        for bc in g.d.building_classes.keys() {
            if is_world_wonder_class(g.d, bc) {
                if let Some(building) = g.d.civ_building(civ, bc) {
                    if g.city_can_construct(city, building, false, false, false) {
                        if g.cities[city].building_production[building] == 0 {
                            if g.unit_get_max_hurry_production(u, city) >= g.city_production_needed_building(city, building) {
                                let value = self.city_ai_building_value(g, city, building, 0);

                                if value > best_value {
                                    best_value = value;
                                    best_building = Some(building);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_building) = best_building {
            g.city_push_order(city, OrderType::ORDER_CONSTRUCT, best_building.i(), -1, false, false, false, false);

            if g.city_production_building(city) == Some(best_building) {
                if g.unit_can_hurry(u, plot, false) {
                    let grp = group_of(g, u);
                    push_simple(g, grp, MissionType::MISSION_HURRY);
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_hurry()` (CvUnitAI.cpp:10793-10874)
    pub fn unit_ai_hurry(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_hurry_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) {
                if g.unit_can_hurry(u, lc_plot, false) {
                    if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                        if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_HURRY, Some(grp), 0) == 0 {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, lc_plot, 0, true, Some(&mut path_turns)) {
                                let mut hurry = false;

                                if g.city_is_production_building(lc) {
                                    if g.city_production_building(lc).map_or(false, |b| building_is_world_wonder(g, b)) {
                                        hurry = true;
                                    }
                                }

                                if hurry {
                                    let mut turns_left = g.city_production_turns_left(lc);

                                    turns_left -= path_turns;

                                    if turns_left > 8 {
                                        let value = turns_left;

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                            best_hurry_plot = Some(lc_plot);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_hurry_plot)) = (best_plot, best_hurry_plot) {
            if g.unit_at_plot(u, best_hurry_plot) {
                push_simple(g, grp, MissionType::MISSION_HURRY);
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_HURRY), Some(best_hurry_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_greatWork()` (CvUnitAI.cpp:10878-10939)
    pub fn unit_ai_great_work(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let grp = group_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_great_work_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            let lc_plot = g.city_plot(lc);
            if self.unit_ai_plot_valid(g, u, lc_plot) {
                if g.unit_can_great_work(u, lc_plot) {
                    if !g.plot_is_visible_enemy_unit_for_unit(lc_plot, u) {
                        if self.player_ai_plot_target_mission_ais(g, owner, lc_plot, MissionAIType::MISSIONAI_GREAT_WORK, Some(grp), 0) == 0 {
                            if g.unit_generate_path(u, lc_plot, 0, true, None) {
                                let mut value = self.city_ai_calculate_culture_pressure(g, lc, true);
                                let lc_owner = g.cities[lc].owner;
                                value -= (100 * g.city_culture(lc, lc_owner)) / 1.max(g.unit_get_great_work_culture(u, lc_plot));
                                if value > 0 {
                                    if value > best_value {
                                        best_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                        best_great_work_plot = Some(lc_plot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(best_plot), Some(best_great_work_plot)) = (best_plot, best_great_work_plot) {
            if g.unit_at_plot(u, best_great_work_plot) {
                push_simple(g, grp, MissionType::MISSION_GREAT_WORK);
                return true;
            } else {
                push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, Some(MissionAIType::MISSIONAI_GREAT_WORK), Some(best_great_work_plot));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_offensiveAirlift()` (CvUnitAI.cpp:10943-11034)
    pub fn unit_ai_offensive_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        let grp = group_of(g, u);
        if g.groups[grp].units.len() > 1 {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let area = g.unit_area(u);

        if g.area_get_target_city(area, owner).is_some() {
            return false;
        }

        let plot = g.unit_plot(u);
        let Some(city) = g.map.plots[plot].plot_city else {
            return false;
        };

        if g.cities[city].max_airlift == 0 {
            return false;
        }

        let city_plot = g.city_plot(city);
        let city_area = g.city_area(city);
        // C++ AI_getUnitAIType() is never NO_UNITAI for an AI unit.
        let unit_ai = g.units[u].ai_type.unwrap_or(UnitAIType::UNITAI_UNKNOWN);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for lc in g.players[owner].cities.clone() {
            let lc_area = g.city_area(lc);
            if lc_area != city_area {
                let (lx, ly) = (g.cities[lc].x, g.cities[lc].y);
                if g.unit_can_airlift_at(u, city_plot, lx, ly) {
                    let Some(target_city) = g.area_get_target_city(lc_area, owner) else {
                        continue;
                    };

                    let target_area = g.city_area(target_city);
                    let area_ai_type = g.map.areas[target_area].area_ai_type[team];
                    if area_ai_type == Some(AreaAIType::AREAAI_OFFENSIVE)
                        || area_ai_type == Some(AreaAIType::AREAAI_DEFENSIVE)
                        || area_ai_type == Some(AreaAIType::AREAAI_MASSING)
                        || self.city_ai_is_danger(g, target_city)
                    {
                        let mut value = 10000;

                        value *= self.player_ai_military_weight(g, owner, lc_area) + 10;
                        value /= self.player_ai_total_area_unit_ais(g, owner, lc_area, unit_ai) + 10;

                        let lc_plot = g.city_plot(lc);
                        let target_plot = g.city_plot(target_city);
                        let path_distance = g.map_calculate_path_distance(lc_plot, target_plot);
                        value += 1.max((g.map.max_step_distance() * 2) - path_distance);

                        if unit_ai == UnitAIType::UNITAI_PARADROP {
                            let nearest_enemy_city = g.map_find_city(lx, ly, None, None, false, false, Some(team), None, None);

                            if let Some(nearest) = nearest_enemy_city {
                                let distance = g.plot_distance(lx, ly, g.cities[nearest].x, g.cities[nearest].y);
                                if distance <= g.unit_get_drop_range(u) {
                                    value *= 5;
                                }
                            }
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(lc_plot);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_AIRLIFT, best_plot, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_paradrop(int iRange)` (CvUnitAI.cpp:11038-11142)
    pub fn unit_ai_paradrop(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let grp = group_of(g, u);
        if g.groups[grp].units.len() > 1 {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);

        let paratrooper_count = g.plot_plot_count(
            plot,
            Some(Game::puf_is_unit_ai_type),
            UnitAIType::UNITAI_PARADROP.i(),
            -1,
            Some(owner),
            None,
            None,
            -1,
            -1,
        );

        if !g.unit_can_paradrop(u, plot) {
            return false;
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        let search_range = self.unit_ai_search_range(g, u, range);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };

                if g.unit_is_potential_enemy(u, g.plot_get_team(loop_plot), Some(loop_plot)) {
                    let (lx, ly) = g.map.xy(loop_plot);
                    if g.unit_can_paradrop_at(u, plot, lx, ly) {
                        let mut value = 0;

                        // C++ asserts the plot is owned (potential enemy territory).
                        let Some(target_player) = g.map.plots[loop_plot].owner else {
                            continue;
                        };

                        if let Some(bonus) = g.map.plots[loop_plot].bonus_type {
                            value += self.player_ai_bonus_val(g, target_player, bonus, 1) - 10;
                        }

                        for i in -1..=1 {
                            for j in -1..=1 {
                                if let Some(adjacent_plot) = g.plot_xy(lx, ly, i, j) {
                                    if let Some(adjacent_city) = g.map.plots[adjacent_plot].plot_city {
                                        if g.cities[adjacent_city].owner == target_player {
                                            let attacker_count = self.player_ai_adjacent_potential_attackers(g, owner, adjacent_plot, true);
                                            let defender_count = g.plot_get_num_visible_enemy_defenders(adjacent_plot, u);
                                            // FIX: guard the C++ division (iParatrooperCount counts this unit, so > 0).
                                            value += 20
                                                * (self.unit_ai_attack_odds(g, u, adjacent_plot, true)
                                                    - ((50 * defender_count) / (paratrooper_count + attacker_count).max(1)));
                                        }
                                    }
                                }
                            }
                        }

                        if value > 0 {
                            value += g.plot_defense_modifier(loop_plot, Some(team), g.unit_ignore_building_defense(u));

                            if let Some(interceptor) = g.unit_best_interceptor(u, loop_plot) {
                                let mut intercept_prob =
                                    if g.unit_is_suicide(u) { 100 } else { g.unit_curr_interception_probability(interceptor) };

                                intercept_prob *= 0.max(100 - g.unit_evasion_probability(u));
                                intercept_prob /= 100;

                                value *= 0.max(100 - intercept_prob / 2);
                                value /= 100;
                            }
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
            push_to_plot(g, grp, MissionType::MISSION_PARADROP, best_plot, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_protect(int iOddsThreshold)` (CvUnitAI.cpp:11146-11221)
    pub fn unit_ai_protect(&self, g: &mut Game, u: UnitId, odds_threshold: i32) -> bool {
        let owner = g.units[u].owner;
        let grp = group_of(g, u);
        let (ux, uy) = g.map.xy(g.unit_plot(u));

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..g.map.num_plots() {
            let loop_plot = PlotIdx(i as u32);

            if self.unit_ai_plot_valid(g, u, loop_plot) {
                if g.map.plots[loop_plot].owner == Some(owner) {
                    if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                        // SPEED TWEAK Sephi: only plots close by (no wrap).
                        let (lx, ly) = g.map.xy(loop_plot);
                        let x_dist = lx - ux;
                        let y_dist = ly - uy;
                        if (x_dist * x_dist) + (y_dist * y_dist) < 10 {
                            if !g.unit_at_plot(u, loop_plot) && g.unit_generate_path(u, loop_plot, 0, true, None) {
                                let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                                let value = self.group_ai_attack_odds(g, grp, loop_plot, true);
                                if value >= self.unit_ai_final_odds_threshold(g, u, loop_plot, odds_threshold) {
                                    if value > best_value {
                                        best_value = value;
                                        best_plot = end_turn_plot;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_patrol()` (CvUnitAI.cpp:11225-11297)
    pub fn unit_ai_patrol(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = group_of(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let is_barbarian = g.unit_is_barbarian(u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for i in 0..DirectionType::NUM {
            let Some(adjacent_plot) = g.plot_direction(ux, uy, DirectionType::from_usize(i)) else {
                continue;
            };

            if self.unit_ai_plot_valid(g, u, adjacent_plot) {
                if !g.plot_is_visible_enemy_unit_for_unit(adjacent_plot, u) {
                    if g.unit_generate_path(u, adjacent_plot, 0, true, None) {
                        let mut value = 1 + self.rand(10000, "AI Patrol");

                        if is_barbarian {
                            if !g.plot_is_owned(adjacent_plot) {
                                value += 20000;
                            }

                            if !g.plot_is_adjacent_owned(adjacent_plot) {
                                value += 10000;
                            }
                        } else {
                            if g.plot_is_revealed_goody(adjacent_plot, Some(team)) {
                                value += 100000;
                            }

                            if g.map.plots[adjacent_plot].owner == Some(owner) {
                                value += 10000;
                            }
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = g.unit_get_path_end_turn_plot(u);
                        }
                    }
                }
            }
        }

        if let Some(best_plot) = best_plot {
            push_to_plot(g, grp, MissionType::MISSION_MOVE_TO, best_plot, None, None);
            return true;
        }

        false
    }
}
