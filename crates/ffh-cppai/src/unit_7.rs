//! Port of CvUnitAI.cpp:16969-19726 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `unit_ai_potential_enemy` takes `team: Option<TeamId>` (C++ passes `pPlot->getTeam()`,
//!   which is `NO_TEAM` for an unowned plot); callers elsewhere pass `Some(t)` or the plot's team.

#![allow(unused_variables, unused_mut, unused_imports, unused_assignments, clippy::all)]

use crate::prelude::*;

/// C++ `getGroup()` of a unit (every unit has a selection group).
fn grp_of(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnitAI: unit without a selection group")
}

/// C++ `getGroup()->pushMission(eMission, iData1, iData2, iFlags, bAppend)` (other arguments at
/// their C++ defaults).
fn push_mission(g: &mut Game, u: UnitId, mission: MissionType, data1: i32, data2: i32, flags: i32, append: bool) {
    let grp = grp_of(g, u);
    g.group_push_mission(grp, mission, data1, data2, flags, append, false, None, None, None);
}

/// `getGroup()->pushMission(MISSION_MOVE_TO, x, y, iFlags)`.
fn push_move_to(g: &mut Game, u: UnitId, plot: PlotIdx, flags: i32) {
    let (x, y) = g.map.xy(plot);
    push_mission(g, u, MissionType::MISSION_MOVE_TO, x, y, flags, false);
}

/// The unit still exists with a group (after a pushed mission that may have moved/killed it).
fn unit_alive(g: &Game, u: UnitId) -> bool {
    g.units.contains(u) && g.units[u].group.map_or(false, |grp| g.groups.contains(grp))
}

/// C++ `getPathLastNode()` (the path finder's last node) `(m_iData1, m_iData2)`.
fn path_last_node_data(g: &Game) -> Option<(i32, i32)> {
    g.path_finder.borrow().last_node().map(|n| (n.data1, n.data2))
}

/// C++ `pPlot->getWorkingCity()` (NULL for a city that no longer exists).
fn plot_working_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].working_city.filter(|&c| g.cities.contains(c))
}

/// C++ `pPlot->getPlotCity()`.
fn plot_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))
}

/// The unit AI types for which `isUnitAllowed*Group` return false, shared part.
fn is_civilian_or_sea_ai(t: Option<UnitAIType>) -> bool {
    matches!(
        t,
        Some(UnitAIType::UNITAI_SETTLE)
            | Some(UnitAIType::UNITAI_WORKER)
            | Some(UnitAIType::UNITAI_TERRAFORMER)
            | Some(UnitAIType::UNITAI_MANA_UPGRADE)
            | Some(UnitAIType::UNITAI_MISSIONARY)
            | Some(UnitAIType::UNITAI_ANIMAL)
            | Some(UnitAIType::UNITAI_PROPHET)
            | Some(UnitAIType::UNITAI_ARTIST)
            | Some(UnitAIType::UNITAI_SCIENTIST)
            | Some(UnitAIType::UNITAI_GENERAL)
            | Some(UnitAIType::UNITAI_MERCHANT)
            | Some(UnitAIType::UNITAI_ENGINEER)
            | Some(UnitAIType::UNITAI_SETTLER_SEA)
            | Some(UnitAIType::UNITAI_WORKER_SEA)
            | Some(UnitAIType::UNITAI_ATTACK_SEA)
            | Some(UnitAIType::UNITAI_RESERVE_SEA)
            | Some(UnitAIType::UNITAI_ESCORT_SEA)
            | Some(UnitAIType::UNITAI_ASSAULT_SEA)
            | Some(UnitAIType::UNITAI_MISSIONARY_SEA)
            | Some(UnitAIType::UNITAI_SPY_SEA)
            | Some(UnitAIType::UNITAI_PIRATE_SEA)
    )
}

impl Cai {
    /// C++ `bool CvUnitAI::AI_exploreAir()` (CvUnitAI.cpp:16969-17020)
    pub fn unit_ai_explore_air(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        for i in g.player_ids() {
            if g.players[i].alive && !g.player_is_barbarian(i) {
                let team_i = g.players[i].team;
                if team_i != team {
                    for c in g.players[i].cities.clone() {
                        if !g.city_is_visible(c, team, false) {
                            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                            if g.unit_can_recon_at(u, plot, cx, cy) {
                                let mut value = 1 + self.rand(15, "AI explore air");
                                if g.unit_is_enemy(u, Some(team_i), None) {
                                    value += 10;
                                    let area = g.city_area(c);
                                    value += 10.min(g.map.areas[area].num_ai_units[owner][UnitAIType::UNITAI_ATTACK_CITY]);
                                    let cplot = g.city_plot(c);
                                    value += 10 * self.player_ai_plot_target_mission_ais(g, owner, cplot, MissionAIType::MISSIONAI_ASSAULT, None, 0);
                                }

                                value *= g.plot_distance(ux, uy, cx, cy);

                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(g.city_plot(c));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            let (x, y) = g.map.xy(bp);
            push_mission(g, u, MissionType::MISSION_RECON, x, y, 0, false);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_nuke()` (CvUnitAI.cpp:17024-17073)
    pub fn unit_ai_nuke(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let mut best_city: Option<CityId> = None;
        let mut best_value = 0;

        for i in g.player_ids() {
            if g.players[i].alive && !g.player_is_barbarian(i) {
                if g.unit_is_enemy(u, Some(g.players[i].team), None) {
                    if self.player_ai_get_attitude(g, owner, i, true) == AttitudeType::ATTITUDE_FURIOUS {
                        for c in g.players[i].cities.clone() {
                            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                            if g.unit_can_nuke_at(u, plot, cx, cy) {
                                let value = self.unit_ai_nuke_value(g, u, c);

                                if value > best_value {
                                    best_value = value;
                                    best_city = Some(c);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(c) = best_city {
            let (x, y) = (g.cities[c].x, g.cities[c].y);
            push_mission(g, u, MissionType::MISSION_NUKE, x, y, 0, false);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_nukeRange(int iRange)` (CvUnitAI.cpp:17075-17194)
    pub fn unit_ai_nuke_range(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let nuke_range = g.unit_nuke_range(u);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        for dx in -range..=range {
            for dy in -range..=range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };
                let (lx, ly) = g.map.xy(loop_plot);
                if !g.unit_can_nuke_at(u, plot, lx, ly) {
                    continue;
                }

                let mut value = -99;

                for dx2 in -nuke_range..=nuke_range {
                    for dy2 in -nuke_range..=nuke_range {
                        let Some(loop_plot2) = g.plot_xy(lx, ly, dx2, dy2) else {
                            continue;
                        };

                        let mut enemy_count = 0;
                        let mut team_count = 0;
                        let mut neutral_count = 0;
                        let mut damaged_enemy_count = 0;

                        for &lu in &g.map.plots[loop_plot2].units {
                            if !g.unit_is_nuke_immune(lu) {
                                let lteam = g.unit_get_team(lu);
                                if lteam == team {
                                    team_count += 1;
                                } else if !g.unit_is_invisible(lu, team, true) {
                                    if g.unit_is_enemy(u, Some(lteam), None) {
                                        enemy_count += 1;
                                        if g.units[lu].damage * 2 > g.unit_max_hit_points(lu) {
                                            damaged_enemy_count += 1;
                                        }
                                    } else {
                                        neutral_count += 1;
                                    }
                                }
                            }
                        }

                        value += (enemy_count + damaged_enemy_count) * (if g.plot_is_water(loop_plot2) { 25 } else { 12 });
                        value -= team_count * 15;
                        value -= neutral_count * 20;

                        let plot2_team = g.plot_get_team(loop_plot2);
                        let multiplier = if plot2_team == Some(team) {
                            -2
                        } else if g.unit_is_enemy(u, plot2_team, None) {
                            1
                        } else if !g.plot_is_owned(loop_plot2) {
                            0
                        } else {
                            -10
                        };

                        if g.map.plots[loop_plot2].improvement_type.is_some() {
                            value += multiplier * 10;
                        }

                        if g.plot_get_bonus_type(loop_plot2, None).is_some() {
                            value += multiplier * 20;
                        }

                        if g.plot_is_city(loop_plot2, false, None) {
                            let pop = plot_city(g, loop_plot2).map_or(0, |c| g.cities[c].population);
                            value += 0.max(multiplier * (-20 + 15 * pop));
                        }
                    }
                }

                if value > best_value {
                    best_value = value;
                    best_plot = Some(loop_plot);
                }
            }
        }

        if let Some(bp) = best_plot {
            let (x, y) = g.map.xy(bp);
            push_mission(g, u, MissionType::MISSION_NUKE, x, y, 0, false);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_trade(int iValueThreshold)` (CvUnitAI.cpp:17196-17267)
    pub fn unit_ai_trade(&self, g: &mut Game, u: UnitId, value_threshold: i32) -> bool {
        let team = g.unit_get_team(u);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_trade_plot: Option<PlotIdx> = None;

        for i in g.player_ids() {
            if !g.players[i].alive {
                continue;
            }
            for c in g.players[i].cities.clone() {
                let cplot = g.city_plot(c);
                if self.unit_ai_plot_valid(g, u, cplot) {
                    if team != g.city_team(c) {
                        let mut value = g.unit_get_trade_gold(u, cplot);

                        if value >= value_threshold && g.unit_can_trade(u, cplot, true) {
                            if !g.plot_is_visible_enemy_unit_for_unit(cplot, u) {
                                let mut path_turns = 0;
                                if g.unit_generate_path(u, cplot, 0, true, Some(&mut path_turns)) {
                                    value /= 4 + path_turns;

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                        best_trade_plot = Some(cplot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(btp)) = (best_plot, best_trade_plot) {
            if g.unit_at_plot(u, btp) {
                push_mission(g, u, MissionType::MISSION_TRADE, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_infiltrate()` (CvUnitAI.cpp:17269-17341)
    pub fn unit_ai_infiltrate(&self, g: &mut Game, u: UnitId) -> bool {
        let team = g.unit_get_team(u);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        let plot = g.unit_plot(u);
        if g.unit_can_infiltrate(u, plot, false) {
            push_mission(g, u, MissionType::MISSION_INFILTRATE, -1, -1, 0, false);
            return true;
        }

        for i in g.player_ids() {
            if g.players[i].alive && g.players[i].team != team {
                for c in g.players[i].cities.clone() {
                    let cplot = g.city_plot(c);
                    if g.unit_can_infiltrate(u, cplot, false) {
                        let mut value = g.unit_get_espionage_points(u, Some(cplot));

                        if value > 0 {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, cplot, 0, true, Some(&mut path_turns)) {
                                if path_last_node_data(g).map_or(false, |(d1, _)| d1 == 0) {
                                    path_turns += 1;
                                }

                                value /= 1 + path_turns;

                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(cplot);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_mission(g, u, MissionType::MISSION_INFILTRATE, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
                if unit_alive(g, u) {
                    let grp = grp_of(g, u);
                    let append = !g.groups[grp].mission_queue.is_empty();
                    push_mission(g, u, MissionType::MISSION_INFILTRATE, -1, -1, 0, append);
                }
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_reconSpy(int iRange)` (CvUnitAI.cpp:17343-17443)
    pub fn unit_ai_recon_spy(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let team = g.unit_get_team(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_target_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        let search_range = self.unit_ai_search_range(g, u, range);

        for ix in -search_range..=search_range {
            for iy in -search_range..=search_range {
                let loop_plot = g.plot_xy(ux, uy, ix, iy);
                let distance = g.step_distance(0, 0, ix, iy);
                let Some(loop_plot) = loop_plot else {
                    continue;
                };
                if !(distance > 0 && self.unit_ai_plot_valid(g, u, loop_plot)) {
                    continue;
                }

                let mut value = 0;
                if plot_city(g, loop_plot).is_some() {
                    value += self.rand(4000, "AI Spy Scout City");
                }

                if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                    value += self.rand(1000, "AI Spy Recon Bonus");
                }

                let (lx, ly) = g.map.xy(loop_plot);
                for i in 0..DirectionType::NUM {
                    if let Some(adj) = g.plot_direction(lx, ly, DirectionType(i as u16)) {
                        if !g.map.plots[adj].revealed[team] {
                            value += 500;
                        } else if !g.plot_is_visible(adj, team) {
                            value += 200;
                        }
                    }
                }

                if value > 0 {
                    let mut path_turns = 0;
                    if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                        if path_turns <= range {
                            // don't give each and every plot in range a value before generating the patch (performance hit)
                            value += self.rand(250, "AI Spy Scout Best Plot");

                            value *= distance;

                            if value > best_value {
                                best_value = value;
                                best_target_plot = g.unit_get_path_end_turn_plot(u);
                                best_plot = Some(loop_plot);
                            }
                        }
                    }
                }
            }
        }

        if let (Some(_), Some(btp)) = (best_plot, best_target_plot) {
            if g.unit_at_plot(u, btp) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            } else {
                push_move_to(g, u, btp, 0);
                // C++ quirk: the skip is pushed without bAppend (it replaces an unfinished move).
                if unit_alive(g, u) {
                    push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
                }
            }
            return true;
        }

        false
    }

    /// C++ `int CvUnitAI::AI_getEspionageTargetValue(CvPlot* pPlot, int iMaxPath)` (CvUnitAI.cpp:17445-17492)
    pub fn unit_ai_get_espionage_target_value(&self, g: &Game, u: UnitId, plot: PlotIdx, max_path: i32) -> i32 {
        let team = g.unit_get_team(u);
        let mut value = 0;

        let Some(plot_team) = g.plot_get_team(plot) else {
            return 0;
        };
        if plot_team != team && !g.teams[team].vassal[plot_team] {
            if self.unit_ai_plot_valid(g, u, plot) {
                if g.plot_is_city(plot, false, None) {
                    value += 10;
                    let r = self.rand(8, "AI spy choose city");
                    value += r * r;
                } else {
                    if let Some(bonus) = g.plot_get_non_obsolete_bonus_type(plot, team) {
                        let plot_owner = g.map.plots[plot].owner.expect("owned plot");
                        value += self.player_ai_base_bonus_val(g, plot_owner, bonus) - 10;
                    }
                }

                let mut path_turns = 0;
                if g.unit_generate_path(u, plot, 0, true, Some(&mut path_turns)) {
                    if path_turns <= max_path {
                        if g.teams[team].ai_war_plan[plot_team].is_none() {
                            value *= 1;
                        } else if g.team_ai_is_sneak_attack_preparing(team, plot_team) {
                            value *= if g.plot_is_city(plot, false, None) { 15 } else { 10 };
                        } else {
                            value *= 3;
                        }
                    }
                }
            }
        }

        value
    }

    /// C++ `bool CvUnitAI::AI_cityOffenseSpy(int iMaxPath)` (CvUnitAI.cpp:17495-17540)
    pub fn unit_ai_city_offense_spy(&self, g: &mut Game, u: UnitId, max_path: i32) -> bool {
        let team = g.unit_get_team(u);
        let area = g.unit_area(u);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for p in g.civ_player_ids() {
            let p_team = g.players[p].team;
            if g.players[p].alive && p_team != team && !g.teams[team].vassal[p_team] {
                for c in g.players[p].cities.clone() {
                    if g.city_area(c) == area {
                        let loop_plot = g.city_plot(c);
                        if self.unit_ai_plot_valid(g, u, loop_plot) {
                            let value = self.unit_ai_get_espionage_target_value(g, u, loop_plot, max_path);
                            if value > best_value {
                                best_value = value;
                                best_plot = Some(loop_plot);
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
                // C++ quirk: the skip is pushed without bAppend.
                if unit_alive(g, u) {
                    push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
                }
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_bonusOffenseSpy(int iRange)` (CvUnitAI.cpp:17542-17583)
    pub fn unit_ai_bonus_offense_spy(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let team = g.unit_get_team(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        let search_range = self.unit_ai_search_range(g, u, range);

        for ix in -search_range..=search_range {
            for iy in -search_range..=search_range {
                if let Some(loop_plot) = g.plot_xy(ux, uy, ix, iy) {
                    if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                        let value = self.unit_ai_get_espionage_target_value(g, u, loop_plot, range);
                        if value > best_value {
                            best_value = value;
                            best_plot = Some(loop_plot);
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
                // C++ quirk: the skip is pushed without bAppend.
                if unit_alive(g, u) {
                    push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
                }
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_espionageSpy()` (CvUnitAI.cpp:17586-17615)
    pub fn unit_ai_espionage_spy(&self, g: &mut Game, u: UnitId) -> bool {
        let plot = g.unit_plot(u);
        if !g.unit_can_espionage(u, plot, false) {
            return false;
        }

        let owner = g.units[u].owner;
        let mut target_plot: Option<PlotIdx> = None;
        let mut target_player: Option<PlayerId> = None;
        let mut extra_data = -1;

        let best_mission = self.player_ai_best_plot_espionage(g, owner, plot, &mut target_player, &mut target_plot, &mut extra_data);
        let Some(best_mission) = best_mission else {
            return false;
        };

        if !g.player_can_do_espionage_mission(owner, best_mission, target_player, target_plot, extra_data, Some(u)) {
            return false;
        }

        if !g.unit_espionage(u, Some(best_mission), extra_data) {
            return false;
        }

        true
    }

    /// C++ `bool CvUnitAI::AI_moveToStagingCity()` (CvUnitAI.cpp:17617-17691)
    pub fn unit_ai_move_to_staging_city(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        let mut war_count = 0;
        let mut target_team: Option<TeamId> = None;
        for t in g.team_ids() {
            if t != team && g.team_is_alive(t) {
                if g.team_ai_is_sneak_attack_preparing(team, t) {
                    target_team = Some(t);
                    war_count += 1;
                }
            }
        }
        if war_count > 1 {
            target_team = None;
        }

        for c in g.players[owner].cities.clone() {
            let cplot = g.city_plot(c);
            if self.unit_ai_plot_valid(g, u, cplot) {
                let mut value = self.city_ai_city_threat(g, c, false);
                if value > 0 {
                    let mut path_turns = 0;
                    if g.unit_generate_path(u, cplot, 0, true, Some(&mut path_turns)) {
                        value *= 1000;
                        value /= 5 + path_turns;
                        // C++ `isVisible(NO_TEAM)` is false.
                        if cplot != plot && target_team.map_or(false, |tt| g.city_is_visible(c, tt, false)) {
                            value /= 2;
                        }

                        if value > best_value {
                            best_value = value;
                            best_plot = g.unit_get_path_end_turn_plot(u);
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_seaRetreatFromCityDanger()` (CvUnitAI.cpp:17693-17718)
    pub fn unit_ai_sea_retreat_from_city_danger(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        //prioritize getting outta there
        if g.plot_is_city(plot, true, None) && self.player_ai_get_plot_danger(g, owner, plot, 2, true) > 0 {
            if self.unit_ai_any_attack(g, u, 2, 40, 0, false) {
                return true;
            }

            if self.unit_ai_any_attack(g, u, 4, 50, 0, false) {
                return true;
            }

            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return true;
            }

            if self.unit_ai_safety(g, u) {
                return true;
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_airRetreatFromCityDanger()` (CvUnitAI.cpp:17720-17739)
    pub fn unit_ai_air_retreat_from_city_danger(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        if g.plot_is_city(plot, true, None) {
            let city = plot_city(g, plot);
            if self.player_ai_get_plot_danger(g, owner, plot, 2, true) > 0
                || city.map_or(false, |c| !self.city_ai_is_defended(g, c, 0))
            {
                if self.unit_ai_air_offensive_city(g, u) {
                    return true;
                }

                if g.unit_can_air_defend(u, None) && self.unit_ai_air_defensive_city(g, u) {
                    return true;
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_airAttackDamagedSkip()` (CvUnitAI.cpp:17741-17778)
    pub fn unit_ai_air_attack_damaged_skip(&self, g: &mut Game, u: UnitId) -> bool {
        if g.units[u].damage == 0 {
            return false;
        }

        let mut skip = g.unit_curr_hit_points(u) * 100 / g.unit_max_hit_points(u) < 40;
        if !skip {
            let search_range = g.unit_air_range(u);
            let (ux, uy) = (g.units[u].x, g.units[u].y);
            let mut skies_clear = true;
            'outer: for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    if let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) {
                        if g.unit_best_interceptor(u, loop_plot).is_some() {
                            skies_clear = false;
                            break 'outer;
                        }
                    }
                }
            }
            skip = !skies_clear;
        }

        if skip {
            push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_followBombard()` (CvUnitAI.cpp:17782-17847)
    pub fn unit_ai_follow_bombard(&self, g: &mut Game, u: UnitId) -> bool {
        let plot = g.unit_plot(u);
        if g.unit_can_bombard(u, plot) {
            push_mission(g, u, MissionType::MISSION_BOMBARD, -1, -1, 0, false);
            return true;
        }

        if g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND {
            let owner = g.units[u].owner;
            let my_group = g.units[u].group;
            let (ux, uy) = (g.units[u].x, g.units[u].y);
            for i in 0..DirectionType::NUM {
                let Some(adj1) = g.plot_direction(ux, uy, DirectionType(i as u16)) else {
                    continue;
                };
                if !g.plot_is_city(adj1, false, None) {
                    continue;
                }
                if !self.unit_ai_potential_enemy(g, u, g.plot_get_team(adj1), Some(adj1)) {
                    continue;
                }
                let (ax, ay) = g.map.xy(adj1);
                for j in 0..DirectionType::NUM {
                    let Some(adj2) = g.plot_direction(ax, ay, DirectionType(j as u16)) else {
                        continue;
                    };
                    for &lu in &g.map.plots[adj2].units {
                        if g.units[lu].owner == owner {
                            if g.unit_can_bombard(lu, adj2) {
                                if g.unit_is_group_head(lu) {
                                    if g.units[lu].group != my_group {
                                        if let Some(lg) = g.units[lu].group {
                                            if g.group_ready_to_move(lg, false) {
                                                return true;
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

        false
    }

    /// C++ `bool CvUnitAI::AI_potentialEnemy(TeamTypes eTeam, const CvPlot* pPlot = NULL)` (CvUnitAI.cpp:17851-17861)
    /// `team` None is `NO_TEAM` (an unowned plot's team).
    pub fn unit_ai_potential_enemy(&self, g: &Game, u: UnitId, team: Option<TeamId>, plot: Option<PlotIdx>) -> bool {
        let grp = grp_of(g, u);
        if self.group_ai_is_declare_war(g, grp, plot) {
            g.unit_is_potential_enemy(u, team, plot)
        } else {
            g.unit_is_enemy(u, team, plot)
        }
    }

    /// C++ `bool CvUnitAI::AI_defendPlot(CvPlot* pPlot)` (CvUnitAI.cpp:17865-17903)
    pub fn unit_ai_defend_plot(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        if !g.unit_can_defend(u, Some(plot)) {
            return false;
        }

        let owner = g.units[u].owner;

        if let Some(c) = plot_city(g, plot) {
            if g.cities[c].owner == owner {
                if self.city_ai_is_danger(g, c) {
                    return true;
                }
            }
        } else {
            let here = if g.unit_at_plot(u, plot) { 1 } else { 0 };
            if g.plot_plot_count(plot, Some(Game::puf_can_defend_group_head), -1, -1, Some(owner), None, None, -1, -1) <= here {
                if g.plot_plot_count(plot, Some(Game::puf_cannot_defend), -1, -1, Some(owner), None, None, -1, -1) > 0 {
                    return true;
                }
            }
        }

        false
    }

    /// C++ `int CvUnitAI::AI_pillageValue(CvPlot* pPlot, int iBonusValueThreshold = 0)` (CvUnitAI.cpp:17906-18017)
    pub fn unit_ai_pillage_value(&self, g: &Game, u: UnitId, plot: PlotIdx, bonus_value_threshold: i32) -> i32 {
        let Some(plot_team) = g.plot_get_team(plot) else {
            return 0;
        };
        let plot_owner = g.map.plots[plot].owner.expect("owned plot");
        let team = g.unit_get_team(u);

        let mut bonus_value = 0;
        let non_obsolete_bonus = g.plot_get_non_obsolete_bonus_type(plot, plot_team);
        if let Some(b) = non_obsolete_bonus {
            bonus_value = self.player_ai_bonus_val(g, plot_owner, b, 1);
        }

        if bonus_value_threshold > 0 {
            if non_obsolete_bonus.is_none() {
                return 0;
            } else if bonus_value < bonus_value_threshold {
                return 0;
            }
        }

        let mut value = 0;

        if g.unit_get_domain_type(u) != DomainType::DOMAIN_AIR {
            if g.plot_is_route(plot) {
                value += 1;
                if non_obsolete_bonus.is_some() {
                    value += bonus_value * 4;
                }

                // C++ quirk: the adjacent plots are those of the unit, not of pPlot.
                let (ux, uy) = (g.units[u].x, g.units[u].y);
                for i in 0..DirectionType::NUM {
                    if let Some(adj) = g.plot_direction(ux, uy, DirectionType(i as u16)) {
                        if g.plot_get_team(adj) == Some(plot_team) {
                            if g.plot_is_city(adj, false, None) {
                                value += 10;
                            }

                            if !g.plot_is_route(adj) {
                                if !g.plot_is_water(adj) && !g.plot_is_impassable(adj) {
                                    value += 2;
                                }
                            }
                        }
                    }
                }
            }
        }

        let improvement = if g.map.plots[plot].improvement_duration > (if g.plot_is_water(plot) { 20 } else { 5 }) {
            g.map.plots[plot].improvement_type
        } else {
            g.map.plots[plot].revealed_improvement_type[team]
        };

        if let Some(imp) = improvement {
            if plot_working_city(g, plot).is_some() {
                value += g.plot_calculate_improvement_yield_change(plot, imp, YieldType::YIELD_FOOD, Some(plot_owner), false, false) * 5;
                value += g.plot_calculate_improvement_yield_change(plot, imp, YieldType::YIELD_PRODUCTION, Some(plot_owner), false, false) * 4;
                value += g.plot_calculate_improvement_yield_change(plot, imp, YieldType::YIELD_COMMERCE, Some(plot_owner), false, false) * 3;
            }

            if g.unit_get_domain_type(u) != DomainType::DOMAIN_AIR {
                value += g.d.improvements[imp].pillage_gold;
            }

            if let Some(b) = non_obsolete_bonus {
                if g.d.improvements[imp].improvement_bonus[b].bonus_trade {
                    let mut temp_value = bonus_value * 4;

                    if g.plot_is_connected_to_capital(plot, None) && g.plot_get_plot_group_connected_bonus(plot, plot_owner, b) == 1 {
                        temp_value *= 2;
                    }

                    value += temp_value;
                }
            }
        }

        value
    }

    /// C++ `int CvUnitAI::AI_nukeValue(CvCity* pCity)` (CvUnitAI.cpp:18020-18089)
    pub fn unit_ai_nuke_value(&self, g: &Game, u: UnitId, city: CityId) -> i32 {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let cplot = g.city_plot(city);

        for t in g.team_ids() {
            if g.team_is_alive(t) && !g.unit_is_enemy(u, Some(t), None) {
                if g.unit_is_nuke_victim(u, cplot, t) {
                    // Don't start wars with neutrals
                    return 0;
                }
            }
        }

        let pop = g.cities[city].population;
        let city_owner = g.cities[city].owner;
        let mut value = 1;

        value += self.rand(pop + 1, "AI Nuke City Value");
        value += 0.max(pop - 10);

        value += (pop * (100 + g.city_calculate_culture_percent(city, city_owner))) / 100;

        value += -(self.player_ai_get_attitude_val(g, owner, city_owner, true) / 3);

        let nuke_range = g.unit_nuke_range(u);
        let (cx, cy) = (g.cities[city].x, g.cities[city].y);
        for dx in -nuke_range..=nuke_range {
            for dy in -nuke_range..=nuke_range {
                if let Some(loop_plot) = g.plot_xy(cx, cy, dx, dy) {
                    if g.map.plots[loop_plot].improvement_type.is_some() {
                        value += 1;
                    }

                    if g.plot_get_bonus_type(loop_plot, None).is_some() {
                        value += 1;
                    }
                }
            }
        }

        if !g.cities[city].ever_owned[owner] {
            value *= 3;
            value /= 2;
        }

        if !g.team_is_a_vassal(g.city_team(city)) {
            value *= 2;
        }

        if g.plot_is_visible(cplot, team) {
            value += 2 * g.plot_get_num_visible_enemy_defenders(cplot, u);
        } else {
            value += 6;
        }

        value
    }

    /// C++ `int CvUnitAI::AI_searchRange(int iRange)` (CvUnitAI.cpp:18092-18107)
    pub fn unit_ai_search_range(&self, g: &Game, u: UnitId, range: i32) -> i32 {
        if range == 0 {
            return 0;
        }

        if g.unit_flat_movement_cost(u) || g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA {
            range * g.unit_base_moves(u)
        } else {
            (range + 1) * (g.unit_base_moves(u) + 1)
        }
    }

    /// C++ `bool CvUnitAI::AI_plotValid(CvPlot* pPlot)` (CvUnitAI.cpp:18111-18154)
    pub fn unit_ai_plot_valid(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        if g.d.units[g.units[u].unit_type].no_reveal_map && g.unit_will_reveal_by_move(u, plot) {
            return false;
        }

        match g.unit_get_domain_type(u) {
            DomainType::DOMAIN_SEA => {
                if g.plot_is_water(plot) || g.unit_can_move_all_terrain(u) {
                    return true;
                } else if g.plot_is_friendly_city(plot, u, true) && g.plot_is_coastal_land(plot, -1) {
                    return true;
                }
            }
            DomainType::DOMAIN_LAND => {
                let unit_area = g.map.plots[g.unit_plot(u)].area;
                if g.map.plots[plot].area == unit_area || g.unit_can_move_all_terrain(u) {
                    return true;
                }
            }
            // DOMAIN_AIR, DOMAIN_IMMOBILE: FAssert(false)
            _ => {}
        }

        false
    }

    /// C++ `int CvUnitAI::AI_finalOddsThreshold(CvPlot* pPlot, int iOddsThreshold)` (CvUnitAI.cpp:18157-18203)
    pub fn unit_ai_final_odds_threshold(&self, g: &Game, u: UnitId, plot: PlotIdx, odds_threshold: i32) -> i32 {
        let mut final_odds_threshold = odds_threshold;

        let city = plot_city(g, plot);

        if let Some(c) = city {
            let defense_damage = g.cities[c].defense_damage;
            if defense_damage < (g.d.defines.max_city_defense_damage * 3) / 4 {
                final_odds_threshold += 0.max(defense_damage - g.cities[c].last_defense_damage - g.d.defines.city_defense_damage_heal_rate * 2);
            }
        }

        if g.plot_get_num_visible_potential_enemy_defenders(plot, u) == 1 {
            if city.is_some() {
                final_odds_threshold *= 2;
                final_odds_threshold /= 3;
            } else {
                final_odds_threshold *= 7;
                final_odds_threshold /= 8;
            }
        }

        let grp = grp_of(g, u);
        if g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && !g.group_has_cargo(grp) {
            final_odds_threshold *= 3;
            final_odds_threshold /= 2 + g.groups[grp].units.len() as i32;
        } else {
            let owner = g.units[u].owner;
            let (px, py) = g.map.xy(plot);
            let far = if g.step_distance(g.units[u].x, g.units[u].y, px, py) > 1 { 1 } else { 0 };
            let city_ai = if g.unit_ai_is_city_ai_type(u) { 2 } else { 0 };
            final_odds_threshold *= 6;
            final_odds_threshold /= 3 + self.player_ai_adjacent_potential_attackers(g, owner, plot, true) + far + city_ai;
        }

        final_odds_threshold.max(1).min(99)
    }

    /// C++ `int CvUnitAI::AI_stackOfDoomExtra()` (CvUnitAI.cpp:18206-18209)
    pub fn unit_ai_stack_of_doom_extra(&self, g: &Game, u: UnitId) -> i32 {
        let owner = g.units[u].owner;
        (g.units[u].ai_birthmark % (1 + g.players[owner].current_era.i())) + 4
    }

    /// C++ `bool CvUnitAI::AI_stackAttackCity(int iRange, int iPowerThreshold, bool bFollow = true)` (CvUnitAI.cpp:18211-18279)
    pub fn unit_ai_stack_attack_city(&self, g: &mut Game, u: UnitId, range: i32, power_threshold: i32, follow: bool) -> bool {
        let search_range = if follow { 1 } else { self.unit_ai_search_range(g, u, range) };
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let grp = grp_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };
                if !self.unit_ai_plot_valid(g, u, loop_plot) {
                    continue;
                }
                if g.plot_is_city(loop_plot, false, None)
                    || (g.plot_is_city(loop_plot, true, None) && g.plot_is_visible_enemy_unit_for_unit(loop_plot, u))
                {
                    if self.unit_ai_potential_enemy(g, u, g.plot_get_team(loop_plot), Some(loop_plot)) {
                        let reachable = !g.unit_at_plot(u, loop_plot)
                            && if follow {
                                g.unit_can_move_into(u, loop_plot, true, true, false)
                            } else {
                                let mut path_turns = 0;
                                g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) && path_turns <= range
                            };
                        if reachable {
                            let value = self.group_ai_compare_stacks(g, grp, loop_plot, true, true, true);

                            if value >= power_threshold {
                                if value > best_value {
                                    best_value = value;
                                    best_plot = if follow { Some(loop_plot) } else { g.unit_get_path_end_turn_plot(u) };
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, if follow { MOVE_DIRECT_ATTACK } else { 0 });
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_moveIntoCity(int iRange)` (CvUnitAI.cpp:18281-18346)
    pub fn unit_ai_move_into_city(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        let plot = g.unit_plot(u);
        if g.plot_is_city(plot, false, None) {
            return false;
        }

        let search_range = self.unit_ai_search_range(g, u, range);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };
                if self.unit_ai_plot_valid(g, u, loop_plot) && !g.unit_is_enemy(u, g.plot_get_team(loop_plot), Some(loop_plot)) {
                    if g.plot_is_city(loop_plot, false, None) || g.plot_is_city(loop_plot, true, None) {
                        let mut path_turns = 0;
                        if g.unit_can_move_into(u, loop_plot, false, false, false)
                            && (g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) && path_turns <= 1)
                        {
                            let mut value = 1;
                            if let Some(c) = plot_city(g, loop_plot) {
                                value += g.cities[c].population;
                            }

                            if value > best_value {
                                best_value = value;
                                best_plot = g.unit_get_path_end_turn_plot(u);
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, 0);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_artistCultureVictoryMove()` (CvUnitAI.cpp:18350-18478)
    pub fn unit_ai_artist_culture_victory_move(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let mut great_work = false;
        let mut join = true;

        if !self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE1) {
            return false;
        }

        if self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_CULTURE3) {
            //Great Work
            great_work = true;
        }

        let culture_cities_needed = g.num_culture_victory_cities;

        let mut best_plot: Option<PlotIdx> = None;
        let mut best_specialist: Option<SpecialistType> = None;
        let mut best_city: Option<CityId> = None;
        let mut best_value = 0;

        let mut target_culture_rank = culture_cities_needed;
        while target_culture_rank > 0 && best_city.is_none() {
            for c in g.players[owner].cities.clone() {
                let cplot = g.city_plot(c);
                if !self.unit_ai_plot_valid(g, u, cplot) {
                    continue;
                }
                // instead of commerce rate rank should use the culture on tile...
                if g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE) == target_culture_rank {
                    // if the city is a fledgling, probably building culture, try next higher city
                    if opt_to_i32(g.cities[c].culture_level) < 2 {
                        break;
                    }

                    // if we cannot path there, try the next higher culture city
                    if !g.unit_generate_path(u, cplot, 0, true, None) {
                        break;
                    }

                    best_city = Some(c);
                    best_plot = Some(cplot);
                    if great_work {
                        if g.unit_can_great_work(u, cplot) {
                            break;
                        }
                    }

                    for i in g.d.specialists.keys() {
                        if g.unit_can_join(u, cplot, Some(i)) {
                            let avoid_growth = self.city_ai_avoid_growth(g, c);
                            let value = self.city_ai_specialist_value(g, c, i, avoid_growth, false);

                            if value > best_value {
                                best_value = value;
                                best_specialist = Some(i);
                            }
                        }
                    }

                    if best_specialist.is_none() {
                        join = false;
                        if g.unit_can_great_work(u, cplot) {
                            great_work = true;
                            break;
                        }
                        great_work = false;
                    }
                    break;
                }
            }

            target_culture_rank -= 1;
        }

        let (Some(_), Some(bp)) = (best_city, best_plot) else {
            //should try to airlift there...
            return false;
        };

        if g.unit_at_plot(u, bp) {
            if great_work {
                push_mission(g, u, MissionType::MISSION_GREAT_WORK, -1, -1, 0, false);
                return true;
            }
            if join {
                push_mission(g, u, MissionType::MISSION_JOIN, opt_to_i32(best_specialist), -1, 0, false);
                return true;
            }
            false
        } else {
            push_move_to(g, u, bp, 0);
            true
        }
    }

    /// C++ `bool CvUnitAI::AI_poach()` (CvUnitAI.cpp:18480-18588)
    pub fn unit_ai_poach(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        let mut best_poach_value = 0;
        let mut best_poach_plot: Option<PlotIdx> = None;
        let mut best_poach_team: Option<TeamId> = None;

        if !g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
            return false;
        }

        if g.teams[team].num_members > 1 {
            return false;
        }

        let mut no_poach_roll = self.player_ai_total_unit_ais(g, owner, UnitAIType::UNITAI_WORKER);
        no_poach_roll += g.players[owner].num_cities();
        no_poach_roll = 0.max((no_poach_roll - 1) / 2);
        if self.rand(no_poach_roll, "AI Poach") > 0 {
            return false;
        }

        if g.team_get_any_war_plan_count(team, true) > 0 {
            return false;
        }

        let civ = g.unit_get_civilization_type(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let range = 1;
        //Look for a unit which is non-combat
        //and has a capture unit type
        for ix in -range..=range {
            for iy in -range..=range {
                // C++ quirk: only the diagonal neighbours are looked at (`&&` instead of `||`).
                if ix != 0 && iy != 0 {
                    let Some(loop_plot) = g.plot_xy(ux, uy, ix, iy) else {
                        continue;
                    };
                    if g.plot_get_team(loop_plot) != Some(team) && g.plot_is_visible(loop_plot, team) {
                        let mut poach_count = 0;
                        let mut defender_count = 0;
                        let mut poach_unit: Option<UnitId> = None;
                        for &lu in &g.map.plots[loop_plot].units {
                            let lteam = g.unit_get_team(lu);
                            if lteam != team && g.team_can_declare_war(team, lteam) {
                                if !g.unit_can_defend(lu, None) {
                                    if g.unit_get_capture_unit_type(lu, civ).is_some() {
                                        poach_count += 1;
                                        poach_unit = Some(lu);
                                    }
                                } else {
                                    defender_count += 1;
                                }
                            }
                        }

                        if let Some(pu) = poach_unit {
                            if defender_count == 0 {
                                let mut value = poach_count * 100;
                                value -= no_poach_roll * 25;
                                if value > best_poach_value {
                                    best_poach_value = value;
                                    best_poach_plot = Some(loop_plot);
                                    best_poach_team = Some(g.unit_get_team(pu));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bt)) = (best_poach_plot, best_poach_team) {
            //No war roll.
            if !self.team_ai_perform_no_war_rolls(g, team, bt) {
                g.team_declare_war(team, bt, true, Some(WarPlanType::WARPLAN_LIMITED));

                if !unit_alive(g, u) {
                    return true;
                }
                push_move_to(g, u, bp, MOVE_DIRECT_ATTACK);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_choke(int iRange = 1)` (CvUnitAI.cpp:18590-18659)
    pub fn unit_ai_choke(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        for ix in -range..=range {
            for iy in -range..=range {
                let Some(loop_plot) = g.plot_xy(ux, uy, ix, iy) else {
                    continue;
                };
                let plot_team = g.plot_get_team(loop_plot);
                if !g.unit_is_enemy(u, plot_team, None) {
                    continue;
                }
                let Some(working_city) = plot_working_city(g, loop_plot) else {
                    continue;
                };
                if Some(g.city_team(working_city)) != plot_team {
                    continue;
                }
                let plot_owner = g.map.plots[loop_plot].owner.expect("owned plot");

                let mut value = -15;
                if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                    if let Some(b) = g.plot_get_bonus_type(loop_plot, None) {
                        value += self.player_ai_bonus_val(g, plot_owner, b, 0);
                    }
                }

                let yields = g.map.plots[loop_plot].yields;
                value += yields[YieldType::YIELD_PRODUCTION.idx()] * 10;
                value += yields[YieldType::YIELD_FOOD.idx()] * 10;
                value += yields[YieldType::YIELD_COMMERCE.idx()] * 5;

                if g.unit_no_defensive_bonus(u) {
                    value *= 0.max((g.unit_base_combat_str(u) * 120) - g.best_land_unit_combat());
                } else {
                    value *= g.plot_defense_modifier(loop_plot, Some(team), false);
                }

                if value > 0 {
                    value *= 10;

                    value /= 1.max(g.plot_get_num_defenders(loop_plot, owner) + if loop_plot == plot { 0 } else { 1 });

                    if g.unit_generate_path(u, loop_plot, 0, false, None) {
                        // C++ quirk: no comparison with iBestValue (the last reachable plot wins).
                        best_plot = g.unit_get_path_end_turn_plot(u);
                        best_value = value;
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
            } else {
                push_move_to(g, u, bp, 0);
            }
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_solveBlockageProblem(CvPlot* pDestPlot, bool bDeclareWar)` (CvUnitAI.cpp:18661-18736)
    pub fn unit_ai_solve_blockage_problem(&self, g: &mut Game, u: UnitId, dest_plot: PlotIdx, declare_war: bool) -> bool {
        let source_plot = g.unit_plot(u);
        let (sx, sy) = g.map.xy(source_plot);
        let (dx, dy) = g.map.xy(dest_plot);

        // The step path from the destination back to the source (GC.getStepFinder(), bReuse).
        let steps: Vec<(i32, i32)> = {
            let gr: &Game = g;
            let mut finder = gr.step_finder.borrow_mut();
            if !finder.generate_path(gr, sx, sy, dx, dy, false, 0) {
                return false;
            }
            let mut steps = Vec::new();
            let mut node = finder.last_node();
            while let Some(n) = node {
                steps.push((n.x, n.y));
                node = finder.parent_of(n);
            }
            steps
        };

        let team = g.unit_get_team(u);

        for i in 0..steps.len() {
            let (stx, sty) = steps[i];
            let step_plot = g.map.plot_xy(stx, sty).expect("step finder node on the map");
            if g.unit_can_move_or_attack_into(u, step_plot, false) && g.unit_generate_path(u, step_plot, 0, true, None) {
                // PORT ADAPT: C++ reads `pStepNode->m_pPrev`, FAStar's open/closed-list link, which
                // ffh-core's finder does not keep; use the node visited just before in this walk
                // (the next step towards the destination), which is what the code means.
                if declare_war && i > 0 {
                    let (px, py) = steps[i - 1];
                    let p_plot = g.map.plot_xy(px, py).expect("step finder node on the map");
                    if let Some(p_team) = g.plot_get_team(p_plot) {
                        if !g.unit_can_move_into(u, p_plot, true, true, false) {
                            if !g.unit_is_potential_enemy(u, Some(p_team), Some(p_plot)) {
                                if g.team_can_declare_war(team, p_team) {
                                    let mut war_plan = WarPlanType::WARPLAN_LIMITED;
                                    let dest_team = g.plot_get_team(dest_plot);
                                    let existing_war_plan = dest_team.and_then(|dt| g.teams[team].ai_war_plan[dt]);
                                    if let (Some(existing), Some(dt)) = (existing_war_plan, dest_team) {
                                        if existing == WarPlanType::WARPLAN_TOTAL || existing == WarPlanType::WARPLAN_PREPARING_TOTAL {
                                            war_plan = WarPlanType::WARPLAN_TOTAL;
                                        }

                                        if !g.teams[team].at_war[dt] {
                                            g.team_ai_set_war_plan(team, dt, None, true);
                                        }
                                    }
                                    g.team_ai_set_war_plan(team, p_team, Some(war_plan), true);
                                    return self.unit_ai_target_city(g, u, 0);
                                }
                            }
                        }
                    }
                }
                if g.plot_is_visible_enemy_unit_for_unit(step_plot, u) {
                    let mut best_plot = Some(step_plot);
                    //To prevent puppeteering attempt to barge through
                    //if quite close
                    if path_last_node_data(g).map_or(false, |(_, d2)| d2 > 3) {
                        best_plot = g.unit_get_path_end_turn_plot(u);
                    }

                    if let Some(bp) = best_plot {
                        push_move_to(g, u, bp, MOVE_DIRECT_ATTACK);
                        return true;
                    }
                }
            }
        }

        false
    }

    /// C++ `int CvUnitAI::AI_calculatePlotWorkersNeeded(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:18738-18763)
    pub fn unit_ai_calculate_plot_workers_needed(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> i32 {
        let build_time = g.plot_get_build_time(plot, build) - g.map.plots[plot].build_progress[build];
        let work_rate = g.unit_work_rate(u, true);

        if work_rate <= 0 {
            return 1;
        }
        let mut turns = build_time / work_rate;

        if build_time > turns * work_rate {
            turns += 1;
        }

        let mut needed = 1.max((turns + 2) / 3);

        if g.plot_get_bonus_type(plot, None).is_some() {
            needed *= 2;
        }
        needed
    }

    /// C++ `bool CvUnitAI::AI_canGroupWithAIType(UnitAITypes eUnitAI) const` (CvUnitAI.cpp:18765-18805)
    pub fn unit_ai_can_group_with_ai_type(&self, g: &Game, u: UnitId, unit_ai: UnitAIType) -> bool {
        if Some(unit_ai) != g.units[u].ai_type {
            match unit_ai {
                UnitAIType::UNITAI_ATTACK_CITY => {
                    let plot = g.unit_plot(u);
                    if g.plot_is_city(plot, false, None) {
                        if let Some(c) = plot_city(g, plot) {
                            if g.game_turn() - g.cities[c].game_turn_acquired <= 1 {
                                return false;
                            }
                        }
                    }
                }
                // BETTER AI (first city) Sephi
                UnitAIType::UNITAI_SETTLE => {
                    if g.game_turn() < 10 {
                        return false;
                    }
                }
                UnitAIType::UNITAI_BARBSMASHER => {
                    if g.units[u].ai_type != Some(UnitAIType::UNITAI_BARBSMASHER) {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// C++ `bool CvUnitAI::AI_allowGroup(const CvUnit* pUnit, UnitAITypes eUnitAI) const` (CvUnitAI.cpp:18809-19006)
    pub fn unit_ai_allow_group(&self, g: &Game, u: UnitId, unit: UnitId, unit_ai: UnitAIType) -> bool {
        let p_group = grp_of(g, unit);
        let p_plot = g.unit_plot(unit);
        let my_group = grp_of(g, u);
        let my_plot = g.unit_plot(u);
        let owner = g.units[u].owner;

        let is_loading_or_guarding = |mission_ai: Option<MissionAIType>| {
            matches!(
                mission_ai,
                Some(MissionAIType::MISSIONAI_GUARD_CITY)
                    | Some(MissionAIType::MISSIONAI_LOAD_SETTLER)
                    | Some(MissionAIType::MISSIONAI_LOAD_ASSAULT)
                    | Some(MissionAIType::MISSIONAI_LOAD_SPECIAL)
            )
        };

        // BETTER AI (Group with Conquest stack) Sephi
        if let Some(temp_unit) = g.group_get_head_unit(my_group) {
            if g.units[temp_unit].ai_groupflag == GROUPFLAG_CONQUEST {
                if unit == u {
                    return false;
                }
                if !g.unit_is_group_head(unit) {
                    return false;
                }
                if p_group == my_group {
                    return false;
                }
                if g.unit_is_cargo(unit) {
                    return false;
                }
                //FfH: Added by Kael 08/18/2008
                if my_plot != p_plot && g.plot_is_visible_enemy_unit(p_plot, owner) {
                    return false;
                }

                // do not join groups that are guarding cities or loading into transports
                if is_loading_or_guarding(g.groups[p_group].ai_mission_ai_type) {
                    return false;
                }

                if !g.unit_can_join_group(u, p_plot, p_group) {
                    return false;
                }

                if unit_ai == UnitAIType::UNITAI_ASSAULT_SEA {
                    if !g.group_has_cargo(p_group) {
                        return false;
                    }
                }

                if g.unit_get_invisible_type(unit).is_some() {
                    if g.unit_get_invisible_type(u).is_none() {
                        return false;
                    }
                }
                match g.group_get_head_unit(p_group) {
                    None => return false,
                    Some(h) => {
                        if g.units[h].ai_groupflag != GROUPFLAG_CONQUEST {
                            return false;
                        }
                    }
                }

                return true;
            }
        }

        if unit == u {
            return false;
        }

        if !g.unit_is_group_head(unit) {
            return false;
        }

        if p_group == my_group {
            return false;
        }

        if g.unit_is_cargo(unit) {
            return false;
        }

        if g.units[unit].ai_type != Some(unit_ai) {
            return false;
        }

        //FfH: Added by Kael 08/18/2008
        if my_plot != p_plot && g.plot_is_visible_enemy_unit(p_plot, owner) {
            return false;
        }

        // do not join groups that are guarding cities, or loading into transports (we might not
        // fit and get stuck in loop forever)
        if is_loading_or_guarding(g.groups[p_group].ai_mission_ai_type) {
            return false;
        }

        if g.groups[p_group].activity_type == ActivityType::ACTIVITY_HEAL {
            // do not attempt to join groups which are healing this turn
            // (healing is cleared every turn for automated groups, so we know we pushed a heal this turn)
            return false;
        }

        if !g.unit_can_join_group(u, p_plot, p_group) {
            return false;
        }

        if unit_ai == UnitAIType::UNITAI_SETTLE {
            if self.player_ai_get_plot_danger(g, owner, p_plot, 3, true) > 0 {
                return false;
            }
        } else if unit_ai == UnitAIType::UNITAI_ASSAULT_SEA {
            if !g.group_has_cargo(p_group) {
                return false;
            }
        }

        if g.group_get_head_unit_ai(my_group) == Some(UnitAIType::UNITAI_CITY_DEFENSE) {
            let team = g.unit_get_team(u);
            if g.plot_is_city(my_plot, false, None) && g.plot_get_team(my_plot) == Some(team) {
                let best_defender = g.plot_get_best_defender(my_plot, Some(owner), None, None, false, false, false);
                if best_defender.map_or(false, |bd| g.units[bd].group == Some(my_group)) {
                    return false;
                }
            }
        }

        if g.map.plots[my_plot].owner == Some(owner) {
            if let Some(target_plot) = g.group_ai_get_mission_ai_plot(p_group) {
                if g.plot_is_owned(target_plot) {
                    if g.unit_is_potential_enemy(u, g.plot_get_team(target_plot), Some(target_plot)) {
                        //Do not join groups which have debarked on an offensive mission
                        return false;
                    }
                }
            }
        }

        if g.unit_get_invisible_type(unit).is_some() {
            if g.unit_get_invisible_type(u).is_none() {
                return false;
            }
        }

        true
    }

    /// C++ `void CvUnitAI::AI_summonAttackMove()` (CvUnitAI.cpp:19071-19092)
    pub fn unit_ai_summon_attack_move(&self, g: &mut Game, u: UnitId) {
        let base_moves = g.unit_base_moves(u);
        if self.unit_ai_any_attack(g, u, base_moves, 5, 0, false) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
    }

    /// C++ `CvPlot* CvUnitAI::getOriginPlot() const` (CvUnitAI.cpp:19100-19107)
    pub fn unit_ai_get_origin_plot(&self, g: &Game, u: UnitId) -> Option<PlotIdx> {
        g.unit_ai_get_origin_plot(u)
    }

    /// C++ `void CvUnitAI::setOriginPlot(CvPlot* newplot)` (CvUnitAI.cpp:19109-19121)
    pub fn unit_ai_set_origin_plot(&self, g: &mut Game, u: UnitId, newplot: PlotIdx) {
        g.unit_ai_set_origin_plot(u, Some(newplot));
    }

    /// C++ `int CvUnitAI::AI_getGroupflag() const` (CvUnitAI.cpp:19123-19126)
    pub fn unit_ai_get_groupflag(&self, g: &Game, u: UnitId) -> i32 {
        g.units[u].ai_groupflag
    }

    /// C++ `void CvUnitAI::AI_setGroupflag(int newflag)` (CvUnitAI.cpp:19128-19131)
    pub fn unit_ai_set_groupflag(&self, g: &mut Game, u: UnitId, newflag: i32) {
        g.units[u].ai_groupflag = newflag;
    }

    /// C++ `void CvUnitAI::AI_chooseGroupflag()` (CvUnitAI.cpp:19134-19226)
    pub fn unit_ai_choose_groupflag(&self, g: &mut Game, u: UnitId) {
        if g.units[u].ai_groupflag != GROUPFLAG_NONE {
            return;
        }
        let owner = g.units[u].owner;
        //Don't Choose a Groupflag if we haven't already build a city
        if g.players[owner].num_cities() == 0 {
            return;
        }

        if g.unit_is_hidden_nationality(u) {
            g.units[u].ai_groupflag = GROUPFLAG_HNGROUP;
            return;
        }

        match g.units[u].ai_type {
            Some(UnitAIType::UNITAI_MAGE) => {
                g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                return;
            }
            Some(UnitAIType::UNITAI_HERO) => {
                g.units[u].ai_groupflag = GROUPFLAG_HERO;
                return;
            }
            Some(UnitAIType::UNITAI_SETTLER_SEA) => {
                g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE_NEW;
                return;
            }
            _ => {}
        }

        if self.unit_ai_is_unit_allowed_perm_defense(g, u) {
            if self.unit_ai_decide_permdefensegroup(g, u) {
                return;
            }
        }

        if self.unit_ai_is_unit_allowed_patrol_group(g, u) {
            if self.unit_ai_decide_patrolgroup(g, u) {
                return;
            }
        }

        if g.units[u].ai_type == Some(UnitAIType::UNITAI_FEASTING) {
            g.units[u].ai_groupflag = GROUPFLAG_FEASTING;
            return;
        }

        // move to subfunction later
        let religion_victory = g.player_get_leader_type(owner).map_or(false, |l| g.d.leader_heads[l].religion_victory);
        if religion_victory {
            // Must use UNITAI, since PromotionSelection peneds on Groupflag
            if g.units[u].ai_type == Some(UnitAIType::UNITAI_MEDIC) {
                if (g.player_count_group_flag_units(owner, GROUPFLAG_INQUISITION) - 2) < (g.players[owner].num_cities() / 5) {
                    g.units[u].ai_groupflag = GROUPFLAG_INQUISITION;
                    return;
                }
            }
        }

        if self.unit_ai_is_unit_allowed_explore_group(g, u) {
            if self.unit_ai_decide_exploregroup(g, u) {
                return;
            }
        }

        if self.unit_ai_is_unit_allowed_defense_group(g, u) {
            g.units[u].ai_groupflag = GROUPFLAG_DEFENSE_NEW;
            push_mission(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false);
        }
    }

    /// C++ `bool CvUnitAI::isUnitAllowedExploreGroup()` (CvUnitAI.cpp:19228-19290)
    pub fn unit_ai_is_unit_allowed_explore_group(&self, g: &Game, u: UnitId) -> bool {
        let info = &g.d.units[g.units[u].unit_type];

        if g.unit_get_unit_class_type(u).i() == g.d.defines.unitclass_scout {
            return true;
        }

        if g.units[u].duration > 0 {
            return false;
        }

        if g.unit_is_hidden_nationality(u) {
            return false;
        }

        if info.ai_block_explore {
            return false;
        }

        let ai = g.units[u].ai_type;
        if is_civilian_or_sea_ai(ai) || ai == Some(UnitAIType::UNITAI_HERO) || ai == Some(UnitAIType::UNITAI_WARWIZARD) {
            return false;
        }

        if info.tier == 1 {
            return false;
        }

        if info.tier == 4 {
            return false;
        }

        g.units[u].ai_allowed_explore
    }

    /// C++ `bool CvUnitAI::AI_decide_exploregroup()` (CvUnitAI.cpp:19293-19319)
    pub fn unit_ai_decide_exploregroup(&self, g: &mut Game, u: UnitId) -> bool {
        if g.unit_get_unit_class_type(u).i() == g.d.defines.unitclass_scout {
            g.units[u].ai_groupflag = GROUPFLAG_EXPLORE;
            return true;
        }
        let owner = g.units[u].owner;
        let area = g.unit_area(u);
        let mut count = 0;
        for &lu in &g.players[owner].units {
            if g.unit_area(lu) == area {
                if g.units[lu].ai_groupflag == GROUPFLAG_EXPLORE {
                    count += 1;
                    if count >= 1 {
                        // replace 1 with a general function later
                        return false;
                    }
                }
            }
        }
        g.units[u].ai_groupflag = GROUPFLAG_EXPLORE;
        true
    }

    /// C++ `bool CvUnitAI::AI_decide_permdefensegroup()` (CvUnitAI.cpp:19322-19382)
    pub fn unit_ai_decide_permdefensegroup(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        if g.map.plots[plot].owner != Some(owner) {
            return false;
        }
        //Check for Units specialized in Defense
        let unit_class = Some(g.unit_get_unit_class_type(u));
        let civ = g.player_get_civilization_type(owner);
        let (bd1, bd2, bd3) = match civ {
            Some(civ) => {
                let ci = &g.d.civilizations[civ];
                (ci.best_defender1, ci.best_defender2, ci.best_defender3)
            }
            None => (None, None, None),
        };

        if unit_class == bd1 {
            if self.plot_ai_needed_better_defender(g, plot, 1) > 0 {
                g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                return true;
            }
        }

        if unit_class == bd2 {
            if self.plot_ai_needed_better_defender(g, plot, 2) > 0 {
                g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                return true;
            }
        }

        if unit_class == bd3 {
            if self.plot_ai_needed_better_defender(g, plot, 3) > 0 {
                g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                return true;
            }
        }

        let ai = g.units[u].ai_type;
        if self.plot_ai_needed_perm_defense_reserve(g, plot, 3) > 0 && ai == Some(UnitAIType::UNITAI_MEDIC) {
            g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
            return true;
        }
        if self.plot_ai_needed_perm_defense_reserve(g, plot, 2) > 0 && ai == Some(UnitAIType::UNITAI_MAGE) {
            g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
            return true;
        }
        if self.plot_ai_needed_perm_defense_reserve(g, plot, 0) > 0 {
            g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
            self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_CITY_DEFENSE);
            return true;
        }
        if self.plot_ai_needed_perm_defense_reserve(g, plot, 1) > 0 {
            g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
            self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_CITY_COUNTER);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_decide_patrolgroup()` (CvUnitAI.cpp:19385-19462)
    pub fn unit_ai_decide_patrolgroup(&self, g: &mut Game, u: UnitId) -> bool {
        let search_range = 10;
        let owner = g.units[u].owner;
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let flag = match g.units[u].ai_type {
            Some(UnitAIType::UNITAI_COUNTER) => 0,
            Some(UnitAIType::UNITAI_WARWIZARD) => 1,
            _ => 0,
        };
        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };
                if !g.plot_is_city(loop_plot, false, None) {
                    continue;
                }
                if g.map.plots[loop_plot].owner != Some(owner) {
                    continue;
                }
                let Some(city) = plot_city(g, loop_plot) else {
                    continue;
                };
                let mut path_turns = 0;
                if let Some(origin) = self.city_ai_get_city_patrol_origin(g, city) {
                    let mut patrol_group: Option<GroupId> = None;

                    for &lu in &g.players[owner].units {
                        if g.units[lu].ai_groupflag == GROUPFLAG_PATROL {
                            if g.unit_ai_get_origin_plot(lu) == Some(origin) {
                                patrol_group = g.units[lu].group;
                                break;
                            }
                        }
                    }

                    if patrol_group.is_none() {
                        if g.unit_generate_path(u, origin, 0, true, Some(&mut path_turns)) {
                            g.units[u].ai_groupflag = GROUPFLAG_PATROL_NEW;
                            return true;
                        }
                    } else if self.city_ai_needed_patrol(g, city, flag) > 0 {
                        if g.unit_generate_path(u, origin, 0, true, Some(&mut path_turns)) {
                            g.units[u].ai_groupflag = GROUPFLAG_PATROL_NEW;
                            return true;
                        }
                    }
                } else if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                    g.units[u].ai_groupflag = GROUPFLAG_PATROL_NEW;
                    return true;
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::isUnitAllowedPermDefense()` (CvUnitAI.cpp:19464-19531)
    pub fn unit_ai_is_unit_allowed_perm_defense(&self, g: &Game, u: UnitId) -> bool {
        let info = &g.d.units[g.units[u].unit_type];
        if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return false;
        }

        if g.units[u].duration > 0 {
            return false;
        }

        if self.unit_ai_is_permanent_summon(g, u) {
            return false;
        }

        if g.unit_no_defensive_bonus(u) {
            return false;
        }

        if info.ai_block_perm_defense {
            return false;
        }

        if g.unit_is_hidden_nationality(u) {
            return false;
        }

        let ai = g.units[u].ai_type;
        if is_civilian_or_sea_ai(ai) || ai == Some(UnitAIType::UNITAI_HERO) || ai == Some(UnitAIType::UNITAI_WARWIZARD) {
            return false;
        }
        if info.tier == 4 {
            return false;
        }

        g.units[u].ai_allowed_perm_defense
    }

    /// C++ `bool CvUnitAI::isUnitAllowedPatrolGroup()` (CvUnitAI.cpp:19533-19595)
    pub fn unit_ai_is_unit_allowed_patrol_group(&self, g: &Game, u: UnitId) -> bool {
        let info = &g.d.units[g.units[u].unit_type];

        if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return false;
        }

        if g.unit_is_hidden_nationality(u) {
            return false;
        }

        if g.units[u].duration > 0 {
            return false;
        }

        if self.unit_ai_is_permanent_summon(g, u) {
            return false;
        }

        if info.ai_block_patrol {
            return false;
        }

        let ai = g.units[u].ai_type;
        if is_civilian_or_sea_ai(ai)
            || ai == Some(UnitAIType::UNITAI_HERO)
            || ai == Some(UnitAIType::UNITAI_FEASTING)
            || ai == Some(UnitAIType::UNITAI_MEDIC)
        {
            return false;
        }
        if info.tier == 4 {
            return false;
        }

        g.units[u].ai_allowed_patrol
    }

    /// C++ `bool CvUnitAI::isUnitAllowedDefenseGroup()` (CvUnitAI.cpp:19598-19644)
    pub fn unit_ai_is_unit_allowed_defense_group(&self, g: &Game, u: UnitId) -> bool {
        if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return false;
        }

        if g.unit_is_hidden_nationality(u) {
            return false;
        }

        if g.unit_get_unit_class_type(u).i() == g.d.defines.unitclass_scout {
            return false;
        }

        let ai = g.units[u].ai_type;
        if is_civilian_or_sea_ai(ai) || ai == Some(UnitAIType::UNITAI_FEASTING) {
            return false;
        }

        true
    }

    /// C++ `bool CvUnitAI::AI_groupheal(int iDamagePercent = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:19647-19726)
    pub fn unit_ai_groupheal(&self, g: &mut Game, u: UnitId, damage_percent: i32, max_path: i32) -> bool {
        let plot = g.unit_plot(u);
        let mut damage_percent = damage_percent;
        let mut max_path = max_path;

        if let Some(feature) = g.map.plots[plot].feature_type {
            if g.d.features[feature].turn_damage != 0 {
                //Pass through
                //(actively seeking a safe spot may result in unit getting stuck)
                if !g.plot_is_city(plot, false, None) && self.unit_ai_retreat_to_city(g, u, false, false, 1) {
                    return true;
                }

                return false;
            }
        }

        //FfH: Added by Kael 10/01/2007 (so the AI won't try to sit and heal in areas where they cant heal)
        if g.unit_heal_rate(u, plot) <= 0 {
            if !g.plot_is_city(plot, false, None) && self.unit_ai_retreat_to_city(g, u, false, false, 1) {
                return true;
            }

            return false;
        }
        //FfH: End Add

        let grp = grp_of(g, u);

        if damage_percent == 0 {
            damage_percent = 10;
        }

        max_path = max_path.min(1);

        let mut total_damage = 0;
        let mut total_hitpoints = 0;
        let mut hurt_unit_count = 0;
        for &lu in &g.groups[grp].units {
            let damage_threshold = (g.unit_max_hit_points(lu) * damage_percent) / 100;

            if g.units[lu].damage > damage_threshold {
                hurt_unit_count += 1;
            }
            total_damage += g.units[lu].damage;
            total_hitpoints += g.unit_max_hit_points(lu);
        }
        if hurt_unit_count * 3 > g.groups[grp].units.len() as i32 {
            if !g.plot_is_city(plot, false, None) && self.unit_ai_retreat_to_city(g, u, false, false, 1) {
                return true;
            }
            if g.groups.contains(grp) {
                g.group_push_mission(grp, MissionType::MISSION_HEAL, -1, -1, 0, false, false, None, None, None);
            }
            return true;
        }
        false
    }
}
