//! Port of CvUnitAI.cpp:11300-14120 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES (for the integrator):
//! - `Cai::plot_ai_sum_strength` (py_2.rs) takes `owner: PlayerId`, but C++ `CvPlot::AI_sumStrength`
//!   is called with `eOwner = NO_PLAYER` by `AI_bombardCity`; it should take `Option<PlayerId>`.
//!   This file uses a private port (`plot_sum_strength`) meanwhile.
//! - `Cai::unit_ai_potential_enemy` (unit_7.rs) takes `team: TeamId`, but C++ passes
//!   `pPlot->getTeam()`, which can be NO_TEAM (an unowned acts-as-city improvement with a visible
//!   enemy unit); it should take `Option<TeamId>`. This file uses `potential_enemy_opt` meanwhile.
//! - `Cai::player_ai_wake_plot_target_mission_ais` (CvPlayerAI) takes `&Game` but wakes groups
//!   (mutates); it is called here with the `&mut Game` reborrowed.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// C++ `getGroup()` of a unit (every unit on the map has a group).
fn grp_of(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnit::getGroup: unit without a selection group")
}

/// C++ `getGroup()->pushMission(eMission, iData1, iData2, iFlags, bAppend, false, eMissionAI,
/// pMissionAIPlot)` (re-reads the unit's group; does nothing if the unit is gone).
#[allow(clippy::too_many_arguments)]
fn push(
    g: &mut Game,
    u: UnitId,
    mission: MissionType,
    data1: i32,
    data2: i32,
    flags: i32,
    append: bool,
    mission_ai: Option<MissionAIType>,
    mission_ai_plot: Option<PlotIdx>,
) {
    if !g.units.contains(u) {
        return;
    }
    let Some(grp) = g.units[u].group else {
        return;
    };
    g.group_push_mission(grp, mission, data1, data2, flags, append, false, mission_ai, mission_ai_plot, None);
}

/// C++ `getGroup()->pushMission(MISSION_MOVE_TO, pPlot->getX_INLINE(), pPlot->getY_INLINE(), iFlags,
/// false, false, eMissionAI, pMissionAIPlot)`.
fn push_move(g: &mut Game, u: UnitId, plot: PlotIdx, flags: i32, mission_ai: Option<MissionAIType>, mission_ai_plot: Option<PlotIdx>) {
    let (x, y) = g.map.xy(plot);
    push(g, u, MissionType::MISSION_MOVE_TO, x, y, flags, false, mission_ai, mission_ai_plot);
}

/// C++ `getGroup()->pushMission(MISSION_SKIP)`.
fn push_skip(g: &mut Game, u: UnitId) {
    push(g, u, MissionType::MISSION_SKIP, -1, -1, 0, false, None, None);
}

/// C++ `getGroup()->getLengthMissionQueue()`.
fn mission_queue_len(g: &Game, u: UnitId) -> i32 {
    match g.units.contains(u).then(|| g.units[u].group).flatten() {
        Some(grp) if g.groups.contains(grp) => g.groups[grp].mission_queue.len() as i32,
        _ => 0,
    }
}

/// C++ `generatePath(pPlot, iFlags, true, &iPathTurns)`: `Some(iPathTurns)` on success.
fn path_turns(g: &Game, u: UnitId, plot: PlotIdx, flags: i32) -> Option<i32> {
    let mut turns = 0;
    if g.unit_generate_path(u, plot, flags, true, Some(&mut turns)) {
        Some(turns)
    } else {
        None
    }
}

/// C++ `getPathLastNode()->m_iData1, m_iData2` (moves left, turns) of the last generated path.
fn path_last_node_data(g: &Game) -> (i32, i32) {
    g.path_finder.borrow().last_node().map(|n| (n.data1, n.data2)).unwrap_or((0, 0))
}

/// C++ `pPlot->getPlotCity()` (NULL for a city that no longer exists).
fn plot_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))
}

/// C++ `pPlot->getWorkingCity()` (NULL for a city that no longer exists).
fn working_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].working_city.filter(|&c| g.cities.contains(c))
}

/// C++ `pLoopUnit->getTransportUnit()` (NULL once the transport is gone).
fn transport_of(g: &Game, u: UnitId) -> Option<UnitId> {
    g.units[u].transport_unit.filter(|&t| g.units.contains(t))
}

/// C++ `CvUnitAI::AI_potentialEnemy(eTeam, pPlot)` with an `eTeam` that may be NO_TEAM.
/// PORT ADAPT: `Cai::unit_ai_potential_enemy` takes a `TeamId`; NO_TEAM is ported inline here.
fn potential_enemy_opt(cai: &Cai, g: &Game, u: UnitId, team: Option<TeamId>, plot: Option<PlotIdx>) -> bool {
    match team {
        Some(t) => cai.unit_ai_potential_enemy(g, u, t, plot),
        None => {
            let grp = grp_of(g, u);
            if cai.group_ai_is_declare_war(g, grp, plot) {
                g.unit_is_potential_enemy(u, None, plot)
            } else {
                g.unit_is_enemy(u, None, plot)
            }
        }
    }
}

/// C++ `CvPlot::AI_sumStrength(eOwner, eAttackingPlayer, eDomainType, bDefensiveBonuses,
/// bTestAtWar, bTestPotentialEnemy)` with `eOwner` possibly NO_PLAYER.
/// PORT ADAPT: `Cai::plot_ai_sum_strength` takes `owner: PlayerId`; ported here with an
/// optional owner.
#[allow(clippy::too_many_arguments)]
fn plot_sum_strength(
    g: &Game,
    plot: PlotIdx,
    owner: Option<PlayerId>,
    attacking_player: Option<PlayerId>,
    domain: Option<DomainType>,
    defensive_bonuses: bool,
    test_at_war: bool,
    test_potential_enemy: bool,
) -> i32 {
    let mut str_sum = 0;
    let attacking_team = attacking_player.map(|ap| g.players[ap].team);
    for &lu in &g.map.plots[plot].units {
        if owner.map_or(true, |o| g.units[lu].owner == o) {
            if attacking_team.map_or(true, |at| !g.unit_is_invisible(lu, at, true)) {
                if !test_at_war || attacking_team.map_or(true, |at| g.at_war(at, g.unit_get_team(lu))) {
                    if !test_potential_enemy || attacking_team.map_or(true, |at| g.unit_is_potential_enemy(lu, Some(at), Some(plot))) {
                        if domain.map_or(true, |d| g.unit_get_domain_type(lu) == d) {
                            let p = if defensive_bonuses { Some(plot) } else { None };
                            str_sum += g.unit_curr_effective_str(lu, p, None, None);
                        }
                    }
                }
            }
        }
    }
    str_sum
}

/// The units on `plot` whose head-of-group is waiting or cannot move (owned by `owner`, able to
/// defend, not in the group led by `u`): the shared count loop of `AI_safety` / `AI_hide`.
fn count_waiting_defenders(g: &Game, u: UnitId, plot: PlotIdx, owner: PlayerId) -> i32 {
    let mut count = 0;
    for &lu in &g.map.plots[plot].units {
        if g.units[lu].owner == owner {
            if g.unit_can_defend(lu, None) {
                let head = g.units[lu].group.filter(|&gr| g.groups.contains(gr)).and_then(|gr| g.group_get_head_unit(gr));
                if let Some(head) = head {
                    if head != u {
                        if g.unit_is_waiting(head) || !g.unit_can_move(head) {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

impl Cai {
    /// C++ `bool CvUnitAI::AI_defend()` (CvUnitAI.cpp:11301-11366)
    pub fn unit_ai_defend(&self, g: &mut Game, u: UnitId) -> bool {
        let here = g.unit_plot(u);
        if self.unit_ai_defend_plot(g, u, here) {
            push_skip(g, u);
            return true;
        }

        let search_range = self.unit_ai_search_range(g, u, 1);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if self.unit_ai_defend_plot(g, u, lp) {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if !g.unit_at_plot(u, lp) {
                                if let Some(turns) = path_turns(g, u, lp, 0) {
                                    if turns <= 1 {
                                        let value = 1 + self.rand(10000, "AI Defend");

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = Some(lp);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move(g, u, bp, 0, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_safety()` (CvUnitAI.cpp:11370-11482)
    pub fn unit_ai_safety(&self, g: &mut Game, u: UnitId) -> bool {
        let search_range = self.unit_ai_search_range(g, u, 1);
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for pass in 0..2 {
            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                    if self.unit_ai_plot_valid(g, u, lp) {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            let flags = if pass > 0 { MOVE_IGNORE_DANGER } else { 0 };
                            if let Some(turns) = path_turns(g, u, lp, flags) {
                                if turns <= 1 {
                                    let count = count_waiting_defenders(g, u, lp, owner);

                                    let mut value = count * 100;

                                    value += g.plot_defense_modifier(lp, Some(team), false);

                                    if g.unit_at_plot(u, lp) {
                                        value += 50;
                                    } else {
                                        value += self.rand(50, "AI Safety");
                                    }

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = Some(lp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_skip(g, u);
                return true;
            } else {
                // C++ quirk: `iPass` is 2 after the loop, so the move always ignores danger.
                push_move(g, u, bp, MOVE_IGNORE_DANGER, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_hide()` (CvUnitAI.cpp:11486-11618)
    pub fn unit_ai_hide(&self, g: &mut Game, u: UnitId) -> bool {
        let Some(invisible) = g.unit_get_invisible_type(u) else {
            return false;
        };

        let search_range = self.unit_ai_search_range(g, u, 1);
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    let mut valid = true;

                    for t in g.team_ids() {
                        if g.team_is_alive(t) {
                            if g.plot_is_invisible_visible(lp, t, invisible) {
                                valid = false;
                                break;
                            }
                        }
                    }

                    if valid {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if let Some(turns) = path_turns(g, u, lp, 0) {
                                if turns <= 1 {
                                    let count = 1 + count_waiting_defenders(g, u, lp, owner);

                                    let mut value = count * 100;

                                    value += g.plot_defense_modifier(lp, Some(team), false);

                                    if g.unit_at_plot(u, lp) {
                                        value += 50;
                                    } else {
                                        value += self.rand(50, "AI Hide");
                                    }

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = Some(lp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                push_skip(g, u);
                return true;
            } else {
                push_move(g, u, bp, 0, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_goody(int iRange)` (CvUnitAI.cpp:11622-11690)
    pub fn unit_ai_goody(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        if g.unit_is_barbarian(u) {
            return false;
        }

        let search_range = self.unit_ai_search_range(g, u, range);
        let team = g.unit_get_team(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if g.plot_is_revealed_goody(lp, Some(team)) {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if !g.unit_at_plot(u, lp) {
                                if let Some(turns) = path_turns(g, u, lp, 0) {
                                    if turns <= range {
                                        let mut value = 1 + self.rand(10000, "AI Goody");

                                        value /= turns + 1;

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move(g, u, bp, 0, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_explore()` (CvUnitAI.cpp:11694-11801)
    pub fn unit_ai_explore(&self, g: &mut Game, u: UnitId) -> bool {
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_explore_plot: Option<PlotIdx> = None;

        let team = g.unit_get_team(u);
        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let no_contact = g.count_civ_teams_alive() > g.team_get_has_met_civ_count(team, true);

        for lp in g.map.plot_ids() {
            if self.unit_ai_plot_valid(g, u, lp) {
                let mut value = 0;

                if g.plot_is_revealed_goody(lp, Some(team)) {
                    value += 100000;
                }

                if value > 0 || self.rand(4, "AI make explore faster ;)") == 0 {
                    if !g.map.plots[lp].revealed[team] {
                        value += 10000;
                    }

                    let (lx, ly) = g.map.xy(lp);
                    for j in 0..DirectionType::NUM {
                        if let Some(adj) = g.plot_direction(lx, ly, DirectionType::from_usize(j)) {
                            if !g.map.plots[adj].revealed[team] {
                                value += 1000;
                            } else if no_contact {
                                if g.plot_get_revealed_team(adj, team) != g.plot_get_team(adj) {
                                    value += 100;
                                }
                            }
                        }
                    }

                    if value > 0 {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_EXPLORE, Some(grp), 3) == 0 {
                                if !g.unit_at_plot(u, lp) {
                                    if let Some(turns) = path_turns(g, u, lp, MOVE_NO_ENEMY_TERRITORY) {
                                        // C++ quirk: the parentheses make this 250 * |dx| + |dy|.
                                        let num = 250 * g.x_distance(ux, lx).abs() + g.y_distance(uy, ly).abs();
                                        value += self.rand(num, "AI explore");

                                        if g.plot_is_adjacent_to_land(lp) {
                                            value += 10000;
                                        }

                                        if g.plot_is_owned(lp) {
                                            value += 5000;
                                        }

                                        value /= 3 + turns.max(1);

                                        if value > best_value {
                                            best_value = value;
                                            best_plot = if g.plot_is_revealed_goody(lp, Some(team)) {
                                                g.unit_get_path_end_turn_plot(u)
                                            } else {
                                                Some(lp)
                                            };
                                            best_explore_plot = Some(lp);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bep)) = (best_plot, best_explore_plot) {
            push_move(g, u, bp, MOVE_NO_ENEMY_TERRITORY, Some(MissionAIType::MISSIONAI_EXPLORE), Some(bep));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_exploreRange(int iRange)` (CvUnitAI.cpp:11805-11963)
    pub fn unit_ai_explore_range(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        let search_range = self.unit_ai_search_range(g, u, range);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_explore_plot: Option<PlotIdx> = None;

        let team = g.unit_get_team(u);
        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let unit_type = g.units[u].unit_type;
        let is_human = g.unit_is_human(u);
        let ai_type = g.units[u].ai_type;
        let birthmark = g.units[u].ai_birthmark;
        let domain = g.unit_get_domain_type(u);

        let impassable_count = self.player_ai_unit_impassable_count(g, owner, unit_type);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    let mut value: i32 = 0;

                    if g.plot_is_revealed_goody(lp, Some(team)) {
                        value += 100000;
                    }

                    if !g.map.plots[lp].revealed[team] {
                        value += 10000;
                    }

                    let (lx, ly) = g.map.xy(lp);
                    for i in 0..DirectionType::NUM {
                        if let Some(adj) = g.plot_direction(lx, ly, DirectionType::from_usize(i)) {
                            if !g.map.plots[adj].revealed[team] {
                                value += 1000;
                            }
                        }
                    }

                    if value > 0 {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_EXPLORE, Some(grp), 3) == 0 {
                                if !g.unit_at_plot(u, lp) {
                                    if let Some(turns) = path_turns(g, u, lp, MOVE_NO_ENEMY_TERRITORY) {
                                        if turns <= range {
                                            value += self.rand(10000, "AI Explore");

                                            if g.plot_is_adjacent_to_land(lp) {
                                                value += 10000;
                                            }

                                            if g.plot_is_owned(lp) {
                                                value += 5000;
                                            }

                                            if !is_human {
                                                let mut direction_modifier: i32 = 100;

                                                if ai_type == Some(UnitAIType::UNITAI_EXPLORE_SEA) && impassable_count == 0 {
                                                    direction_modifier += (50 * (dx.abs() + dy.abs())) / search_range;
                                                    if g.circumnavigation_available() {
                                                        if g.map.wrap_x {
                                                            let sign = if birthmark % 2 == 0 { 1 } else { -1 };
                                                            if dx * sign > 0 {
                                                                direction_modifier = direction_modifier.wrapping_mul(150 + (dx * 100) / search_range);
                                                            } else {
                                                                direction_modifier /= 2;
                                                            }
                                                        }
                                                        if g.map.wrap_y {
                                                            let sign = if (birthmark >> 1) % 2 == 0 { 1 } else { -1 };
                                                            if dy * sign > 0 {
                                                                direction_modifier = direction_modifier.wrapping_mul(150 + (dy * 100) / search_range);
                                                            } else {
                                                                direction_modifier /= 2;
                                                            }
                                                        }
                                                    }
                                                    // C++ int arithmetic (can overflow on maps wrapping both ways).
                                                    value = value.wrapping_mul(direction_modifier);
                                                    value /= 100;
                                                }
                                            }

                                            if value > best_value {
                                                best_value = value;
                                                if domain == DomainType::DOMAIN_LAND {
                                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                                } else {
                                                    best_plot = Some(lp);
                                                }
                                                best_explore_plot = Some(lp);
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

        if let (Some(bp), Some(bep)) = (best_plot, best_explore_plot) {
            push_move(g, u, bp, MOVE_NO_ENEMY_TERRITORY, Some(MissionAIType::MISSIONAI_EXPLORE), Some(bep));
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_targetCity(int iFlags = 0)` (CvUnitAI.cpp:11967-12118)
    pub fn unit_ai_target_city(&self, g: &mut Game, u: UnitId, flags: i32) -> bool {
        let mut best_value;
        let mut best_city: Option<CityId> = None;

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let area = g.unit_area(u);
        let grp = grp_of(g, u);
        let ai_type = g.units[u].ai_type;

        if let Some(target_city) = g.area_get_target_city(area, owner) {
            let tplot = g.city_plot(target_city);
            if self.unit_ai_potential_enemy(g, u, g.city_team(target_city), Some(tplot)) {
                if !g.unit_at_plot(u, tplot) && g.unit_generate_path(u, tplot, flags, true, None) {
                    best_city = Some(target_city);
                }
            }
        }

        if best_city.is_none() {
            best_value = 0;
            for pl in g.civ_player_ids() {
                if g.players[pl].alive {
                    let pl_team = g.players[pl].team;
                    for c in g.players[pl].cities.clone() {
                        let cplot = g.city_plot(c);
                        // BBAI efficiency: check area for land units before generating path
                        if self.unit_ai_plot_valid(g, u, cplot) && g.city_area(c) == area {
                            if self.unit_ai_potential_enemy(g, u, pl_team, Some(cplot)) {
                                if !g.unit_at_plot(u, cplot) {
                                    if let Some(mut turns) = path_turns(g, u, cplot, flags) {
                                        let mut value = if ai_type == Some(UnitAIType::UNITAI_ATTACK_CITY) {
                                            // lemming?
                                            self.player_ai_target_city_value(g, owner, c, false, false)
                                        } else {
                                            self.player_ai_target_city_value(g, owner, c, true, true)
                                        };

                                        value *= 1000;

                                        if g.map.areas[area].area_ai_type[team] == Some(AreaAIType::AREAAI_DEFENSIVE) {
                                            if g.city_calculate_culture_percent(c, owner) < 75 {
                                                value /= 2;
                                            }
                                        }

                                        // If stack has poor bombard, direct towards lower defense cities
                                        turns += 16.min(g.group_get_bombard_turns(grp, c)) / 2;

                                        value /= 4 + turns * turns;

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
            }
        }

        if let Some(bc) = best_city {
            best_value = 0;
            let mut best_plot: Option<PlotIdx> = None;
            let cplot = g.city_plot(bc);
            let (cx, cy) = (g.cities[bc].x, g.cities[bc].y);

            if 0 == (flags & MOVE_THROUGH_ENEMY) {
                for i in 0..DirectionType::NUM {
                    let Some(adj) = g.plot_direction(cx, cy, DirectionType::from_usize(i)) else { continue };

                    if self.unit_ai_plot_valid(g, u, adj) {
                        if !g.plot_is_visible_enemy_unit_for_unit(adj, u) {
                            if let Some(turns) = path_turns(g, u, adj, flags) {
                                let mut value = 0.max(g.plot_defense_modifier(adj, Some(team), false) + 100);

                                if !g.plot_is_river_crossing(adj, g.direction_xy_plots(adj, cplot)) {
                                    value += 12 * -g.d.defines.river_attack_modifier;
                                }

                                if !g.unit_is_enemy(u, g.plot_get_team(adj), Some(adj)) {
                                    value += 100;
                                }

                                value = value.max(1);

                                value *= 1000;

                                value /= turns + 1;

                                if value > best_value {
                                    best_value = value;
                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                }
                            }
                        }
                    }
                }
            } else {
                best_plot = Some(cplot);
            }

            if let Some(bp) = best_plot {
                if !g.unit_at_plot(u, bp) {
                    push_move(g, u, bp, flags, None, None);
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_targetBarbCity()` (CvUnitAI.cpp:12127-12234)
    pub fn unit_ai_target_barb_city(&self, g: &mut Game, u: UnitId) -> bool {
        if g.unit_is_barbarian(u) {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        let mut best_value = 0;
        let mut best_city: Option<CityId> = None;

        let barb = g.barbarian_player();
        for c in g.players[barb].cities.clone() {
            let cplot = g.city_plot(c);
            if self.unit_ai_plot_valid(g, u, cplot) {
                if g.city_is_revealed(c, team, false) {
                    if !g.unit_at_plot(u, cplot) {
                        if let Some(turns) = path_turns(g, u, cplot, 0) {
                            if turns < 10 {
                                let mut value = self.player_ai_target_city_value(g, owner, c, false, false);

                                value *= 1000;

                                value /= turns + 1;

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

        if let Some(bc) = best_city {
            best_value = 0;
            let mut best_plot: Option<PlotIdx> = None;
            let cplot = g.city_plot(bc);
            let (cx, cy) = (g.cities[bc].x, g.cities[bc].y);

            for i in 0..DirectionType::NUM {
                let Some(adj) = g.plot_direction(cx, cy, DirectionType::from_usize(i)) else { continue };

                if self.unit_ai_plot_valid(g, u, adj) {
                    if !g.plot_is_visible_enemy_unit_for_unit(adj, u) {
                        if let Some(turns) = path_turns(g, u, adj, 0) {
                            let mut value = 0.max(g.plot_defense_modifier(adj, Some(team), false) + 100);

                            if !g.plot_is_river_crossing(adj, g.direction_xy_plots(adj, cplot)) {
                                value += 10 * -g.d.defines.river_attack_modifier;
                            }

                            value = value.max(1);

                            value *= 1000;

                            value /= turns + 1;

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
                    push_skip(g, u);
                    return true;
                } else {
                    push_move(g, u, bp, 0, None, None);
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_bombardCity()` (CvUnitAI.cpp:12241-12312)
    pub fn unit_ai_bombard_city(&self, g: &mut Game, u: UnitId) -> bool {
        let here = g.unit_plot(u);
        if g.unit_can_bombard(u, here) {
            let Some(bombard_city) = g.unit_bombard_target(u, here) else {
                return false;
            };
            let owner = g.units[u].owner;
            let grp = grp_of(g, u);
            let bplot = g.city_plot(bombard_city);

            // do not bombard cities with no defenders
            let defender_strength = plot_sum_strength(g, bplot, None, Some(owner), Some(DomainType::DOMAIN_LAND), true, true, false);
            if defender_strength == 0 {
                return false;
            }

            // do not bombard cities if we have overwelming odds
            let attack_odds = self.group_ai_attack_odds(g, grp, bplot, true);
            if attack_odds > 95 {
                return false;
            }

            // BETTER_BTS_AI_MOD 10/28/09 jdog5000: War Tactics AI
            // If we have reasonable odds, check for attacking without waiting for bombards
            if attack_odds > 1.max(g.d.defines.bbai_skip_bombard_best_attack_odds) {
                let mut base = g.d.defines.bbai_skip_bombard_base_stack_ratio;
                base = if base == 0 { 300 } else { base };
                let comparison = self.group_ai_compare_stacks(g, grp, bplot, true, true, true);

                // Big troop advantage plus pretty good starting odds, don't wait to allow reinforcements
                if comparison > base - 2 * attack_odds {
                    return false;
                }

                let mut min = g.d.defines.bbai_skip_bombard_min_stack_ratio;
                min = if min == 0 { 140 } else { min };
                // Bombard at least one turn to allow bombers/ships to get some shots in too
                if g.units[u].fortify_turns > 0 && g.cities[bombard_city].defense_damage > g.d.defines.max_city_defense_damage / 5 {
                    let bombard_turns = g.group_get_bombard_turns(grp, bombard_city);
                    if comparison > min.max(base - 2 * attack_odds - 3 * bombard_turns) {
                        return false;
                    }
                }
            }

            if g.cities[bombard_city].defense_damage < (g.d.defines.max_city_defense_damage * 3) / 4 {
                push(g, u, MissionType::MISSION_BOMBARD, -1, -1, 0, false, None, None);
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_cityAttack(int iRange, int iOddsThreshold, bool bFollow = false)` (CvUnitAI.cpp:12316-12385)
    pub fn unit_ai_city_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, follow: bool) -> bool {
        let search_range = if follow { 1 } else { self.unit_ai_search_range(g, u, range) };

        let team = g.unit_get_team(u);
        let grp = grp_of(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if g.plot_is_city(lp, false, None) || (g.plot_is_city(lp, true, Some(team)) && g.plot_is_visible_enemy_unit_for_unit(lp, u)) {
                        if potential_enemy_opt(self, g, u, g.plot_get_team(lp), Some(lp)) {
                            let reachable = !g.unit_at_plot(u, lp)
                                && if follow {
                                    g.unit_can_move_into(u, lp, true, false, false)
                                } else {
                                    path_turns(g, u, lp, 0).map_or(false, |t| t <= range)
                                };
                            if reachable {
                                let value = self.group_ai_attack_odds(g, grp, lp, true);

                                if value >= self.unit_ai_final_odds_threshold(g, u, lp, odds_threshold) {
                                    if value > best_value {
                                        best_value = value;
                                        best_plot = if follow { Some(lp) } else { g.unit_get_path_end_turn_plot(u) };
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move(g, u, bp, if follow { MOVE_DIRECT_ATTACK } else { 0 }, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_anyAttack(int iRange, int iOddsThreshold, int iMinStack = 0, bool bFollow = false)` (CvUnitAI.cpp:12389-12500)
    pub fn unit_ai_any_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, min_stack: i32, follow: bool) -> bool {
        if self.unit_ai_range_attack(g, u, range) {
            return true;
        }

        let search_range = if follow { 1 } else { self.unit_ai_search_range(g, u, range) };

        let grp = grp_of(g, u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if g.plot_is_visible_enemy_unit_for_unit(lp, u)
                        || (g.plot_is_city(lp, false, None) && potential_enemy_opt(self, g, u, g.plot_get_team(lp), None))
                    {
                        let reachable = !g.unit_at_plot(u, lp)
                            && if follow {
                                g.unit_can_move_into(u, lp, true, false, false)
                            } else {
                                path_turns(g, u, lp, 0).map_or(false, |t| t <= range)
                            };
                        if reachable {
                            if g.plot_get_num_visible_enemy_defenders(lp, u) >= min_stack {
                                // BETTER AI (Sephi): skip plots holding only summoned units.
                                let mut only_summons = true;
                                for &lu in &g.map.plots[lp].units {
                                    if g.units[lu].duration == 0 && !self.unit_ai_is_permanent_summon(g, lu) {
                                        only_summons = false;
                                        break;
                                    }
                                }
                                if !only_summons {
                                    let value = self.group_ai_attack_odds(g, grp, lp, true);

                                    if value >= self.unit_ai_final_odds_threshold(g, u, lp, odds_threshold) {
                                        if value > best_value {
                                            best_value = value;
                                            best_plot = if follow { Some(lp) } else { g.unit_get_path_end_turn_plot(u) };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move(g, u, bp, if follow { MOVE_DIRECT_ATTACK } else { 0 }, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_rangeAttack(int iRange)` (CvUnitAI.cpp:12503-12552)
    pub fn unit_ai_range_attack(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        if !g.unit_can_range_strike(u) {
            return false;
        }

        let search_range = self.unit_ai_search_range(g, u, range);
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if g.plot_is_visible_enemy_unit_for_unit(lp, u)
                    || (g.plot_is_city(lp, false, None) && potential_enemy_opt(self, g, u, g.plot_get_team(lp), None))
                {
                    let (lx, ly) = g.map.xy(lp);
                    if !g.unit_at_plot(u, lp) && g.unit_can_range_strike_at(u, here, lx, ly) {
                        let value = self.group_ai_attack_odds(g, grp, lp, true);

                        if value > best_value {
                            best_value = value;
                            best_plot = Some(lp);
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            let (x, y) = g.map.xy(bp);
            push(g, u, MissionType::MISSION_RANGE_ATTACK, x, y, 0, false, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_leaveAttack(int iRange, int iOddsThreshold, int iStrengthThreshold)` (CvUnitAI.cpp:12554-12636)
    pub fn unit_ai_leave_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, strength_threshold: i32) -> bool {
        let search_range = range;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        if let Some(city) = plot_city(g, here) {
            if g.cities[city].owner == owner {
                let our_strength = self.player_ai_get_our_plot_strength(g, owner, here, 0, false, false);
                let enemy_strength = self.player_ai_get_enemy_plot_strength(g, owner, here, 2, false, false);
                if enemy_strength > 0 {
                    if (our_strength * 100) / enemy_strength < strength_threshold {
                        return false;
                    }
                    let defenders = g.plot_plot_count(here, Some(Game::puf_can_defend_group_head), -1, -1, Some(owner), None, None, -1, -1);
                    if defenders <= g.groups[grp].units.len() as i32 {
                        return false;
                    }
                }
            }
        }

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if g.plot_is_visible_enemy_unit_for_unit(lp, u)
                        || (g.plot_is_city(lp, false, None) && potential_enemy_opt(self, g, u, g.plot_get_team(lp), Some(lp)))
                    {
                        if !g.unit_at_plot(u, lp) && path_turns(g, u, lp, 0).map_or(false, |t| t <= range) {
                            if g.plot_get_num_visible_enemy_defenders(lp, u) > 0 {
                                let value = self.group_ai_attack_odds(g, grp, lp, true);

                                if value >= self.unit_ai_final_odds_threshold(g, u, lp, odds_threshold) {
                                    if value > best_value {
                                        best_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move(g, u, bp, 0, None, None);
            return true;
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_blockade()` (CvUnitAI.cpp:12639-12754)
    pub fn unit_ai_blockade(&self, g: &mut Game, u: UnitId) -> bool {
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_blockade_plot: Option<PlotIdx> = None;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let min_water = g.d.defines.min_water_size_for_ocean;

        for lp in g.map.plot_ids() {
            if self.unit_ai_plot_valid(g, u, lp) {
                if g.unit_potential_war_action(u, lp) {
                    if let Some(city) = working_city(g, lp) {
                        if g.city_is_coastal(city, min_water) {
                            if !g.city_is_barbarian(city) {
                                if !g.plot_is_visible_enemy_unit_for_unit(lp, u) && g.unit_can_plunder(u, lp, false) {
                                    if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_BLOCKADE, Some(grp), 2) == 0 {
                                        if let Some(turns) = path_turns(g, u, lp, 0) {
                                            // (getPathLastNode is read before the calls below; they
                                            // generate no paths in C++ either)
                                            let last_moves = path_last_node_data(g).0;
                                            let cplot = g.city_plot(city);
                                            let mut value = 1;

                                            value += g.cities[city].population.min(g.city_count_num_water_plots(city));

                                            value += self.player_ai_adjacent_potential_attackers(g, owner, cplot, false);

                                            value += self.player_ai_plot_target_mission_ais(g, owner, cplot, MissionAIType::MISSIONAI_ASSAULT, Some(grp), 2) * 3;

                                            if g.unit_can_bombard(u, lp) {
                                                value *= 2;
                                            }

                                            value *= 1000;

                                            value /= turns + 1;

                                            if turns == 1 {
                                                // Prefer to have movement remaining to Bombard + Plunder
                                                value *= 1 + 2.min(last_moves);
                                            }

                                            // if not at war with this plot owner, then devalue plot if we already inside this owner's borders
                                            // (because declaring war will pop us some unknown distance away)
                                            let lp_team = g.plot_get_team(lp);
                                            if !g.unit_is_enemy(u, lp_team, None) && g.plot_get_team(here) == lp_team {
                                                value /= 10;
                                            }

                                            if value > best_value {
                                                best_value = value;
                                                best_plot = g.unit_get_path_end_turn_plot(u);
                                                best_blockade_plot = Some(lp);
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

        if let (Some(bp), Some(bbp)) = (best_plot, best_blockade_plot) {
            if g.unit_at_plot(u, bbp) && !g.unit_is_enemy(u, g.plot_get_team(bbp), Some(bbp)) {
                g.group_declare_war(grp, bbp, true);
                if !g.units.contains(u) {
                    return true;
                }
            }

            if g.unit_at_plot(u, bbp) {
                let now_here = g.unit_plot(u);
                if g.unit_can_bombard(u, now_here) {
                    push(g, u, MissionType::MISSION_BOMBARD, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                }

                let append = mission_queue_len(g, u) > 0;
                push(g, u, MissionType::MISSION_PLUNDER, -1, -1, 0, append, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));

                return true;
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_pirateBlockade()` (CvUnitAI.cpp:12758-12979)
    pub fn unit_ai_pirate_blockade(&self, g: &mut Game, u: UnitId) -> bool {
        let num_plots = g.map.num_plots();
        let mut death_zone: Vec<i32> = vec![0; num_plots];

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = grp_of(g, u);
        let area = g.unit_area(u);
        let here = g.unit_plot(u);

        for lp in g.map.plot_ids() {
            if self.unit_ai_plot_valid(g, u, lp) || (g.plot_is_city(lp, false, None) && g.plot_is_adjacent_to_area(lp, area)) {
                if g.plot_is_owned(lp) && g.plot_get_team(lp) != Some(team) {
                    let mut best_hostile_moves = 0;
                    for &lu in &g.map.plots[lp].units {
                        if g.unit_is_enemy(u, Some(g.unit_get_team(lu)), g.unit_plot_opt(lu)) {
                            if g.unit_get_domain_type(lu) == DomainType::DOMAIN_SEA && !g.unit_is_invisible(lu, team, true) {
                                if g.unit_can_attack(lu) {
                                    if g.unit_curr_effective_str(lu, None, None, None) > g.unit_curr_effective_str(u, Some(lp), Some(lu), None) {
                                        // FIX: C++ uses `getMoves()` (movement already used, in
                                        // 1/MOVE_DENOMINATOR units) as a radius in plots, which marks
                                        // most of the map as a death zone; use the unit's moves in plots.
                                        best_hostile_moves = best_hostile_moves.max(g.unit_base_moves(lu));
                                    }
                                }
                            }
                        }
                    }
                    if best_hostile_moves > 0 {
                        let (lx, ly) = g.map.xy(lp);
                        for ix in -best_hostile_moves..=best_hostile_moves {
                            for iy in -best_hostile_moves..=best_hostile_moves {
                                if let Some(rp) = g.plot_xy(lx, ly, ix, iy) {
                                    death_zone[rp.idx()] += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        let is_in_danger = death_zone[here.idx()] > 0;

        if !is_in_danger {
            let damage = g.units[u].damage;
            if damage > 0 {
                if !g.plot_is_owned(here) && !g.plot_is_adjacent_owned(here) {
                    if self.unit_ai_retreat_to_city(g, u, false, false, 1 + damage / 20) {
                        return true;
                    }
                    push_skip(g, u);
                    return true;
                }
            }
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_blockade_plot: Option<PlotIdx> = None;
        let mut best_is_force_move = false;
        let mut best_is_move = false;

        let blockade_range = g.d.defines.ship_blockade_range - 1;

        for lp in g.map.plot_ids() {
            if self.unit_ai_plot_valid(g, u, lp) {
                if !g.plot_is_visible_enemy_unit_for_unit(lp, u) && g.unit_can_plunder(u, lp, false) {
                    if self.rand(4, "AI Pirate Blockade") == 0 {
                        if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_BLOCKADE, Some(grp), 3) == 0 {
                            if let Some(turns) = path_turns(g, u, lp, 0) {
                                let mut blockaded_count = 0;
                                let mut population_value = 0;
                                let (lx, ly) = g.map.xy(lp);
                                for ix in -blockade_range..=blockade_range {
                                    for iy in -blockade_range..=blockade_range {
                                        let Some(rp) = g.plot_xy(lx, ly, ix, iy) else { continue };
                                        let mut plot_blockaded = false;
                                        let rp_team = g.plot_get_team(rp);
                                        if g.plot_is_water(rp) && g.plot_is_owned(rp) && g.unit_is_enemy(u, rp_team, Some(lp)) {
                                            plot_blockaded = true;
                                            if let Some(rt) = rp_team {
                                                blockaded_count += g.map.plots[rp].blockaded_count[rt];
                                            }
                                        }

                                        if !plot_blockaded {
                                            if let Some(pc) = plot_city(g, rp) {
                                                let pc_team = g.city_team(pc);
                                                if g.unit_is_enemy(u, Some(pc_team), Some(lp)) {
                                                    let mut city_value = 3 + g.cities[pc].population;
                                                    city_value *= if g.at_war(team, pc_team) { 1 } else { 3 };
                                                    if g.player_is_no_foreign_trade(g.cities[pc].owner) {
                                                        city_value /= 2;
                                                    }
                                                    population_value += city_value;
                                                }
                                            }
                                        }
                                    }
                                }
                                let mut value = population_value;

                                value *= 1000;

                                value /= 16 + blockaded_count;

                                let (last_moves, last_turns) = path_last_node_data(g);
                                let is_move = last_turns == 1 && last_moves > 0;
                                if g.unit_at_plot(u, lp) {
                                    value *= 3;
                                } else if is_move {
                                    value *= 2;
                                }

                                let death = death_zone[lp.idx()];

                                let mut force_move = false;
                                if death != 0 {
                                    value /= 10;
                                } else if is_in_danger && turns <= 2 && 0 == population_value {
                                    if last_moves == 0 {
                                        if !g.plot_is_adjacent_owned(lp) {
                                            let rnd = self.rand(2500, "AI Pirate Retreat");
                                            value += rnd;
                                            if rnd > 1000 {
                                                value += self.rand(2500, "AI Pirate Retreat");
                                                force_move = true;
                                            }
                                        }
                                    }
                                }

                                if !force_move {
                                    value /= turns + 1;
                                }

                                if value > best_value {
                                    best_value = value;
                                    best_plot = if force_move { Some(lp) } else { g.unit_get_path_end_turn_plot(u) };
                                    best_blockade_plot = Some(lp);
                                    best_is_force_move = force_move;
                                    best_is_move = is_move;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bbp)) = (best_plot, best_blockade_plot) {
            if g.unit_at_plot(u, bbp) {
                let append = mission_queue_len(g, u) > 0;
                push(g, u, MissionType::MISSION_PLUNDER, -1, -1, 0, append, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                return true;
            } else if best_is_force_move {
                push_move(g, u, bp, 0, None, None);
                return true;
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                if best_is_move {
                    let append = mission_queue_len(g, u) > 0;
                    push(g, u, MissionType::MISSION_PLUNDER, -1, -1, 0, append, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                }
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_seaBombardRange(int iMaxRange)` (CvUnitAI.cpp:12982-13104)
    pub fn unit_ai_sea_bombard_range(&self, g: &mut Game, u: UnitId, max_range: i32) -> bool {
        // cached values
        let owner = g.units[u].owner;
        let here = g.unit_plot(u);
        let grp = grp_of(g, u);
        let ai_type = g.units[u].ai_type;

        // can any unit in this group bombard?
        let mut has_bombard_unit = false;
        let mut bombard_unit_can_bombard_now = false;
        for &lu in &g.groups[grp].units {
            if bombard_unit_can_bombard_now {
                break;
            }
            if g.unit_bombard_rate(lu) > 0 {
                has_bombard_unit = true;

                if g.unit_can_move(lu) && !g.units[lu].made_attack {
                    bombard_unit_can_bombard_now = true;
                }
            }
        }

        if !has_bombard_unit {
            return false;
        }

        // best match
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_bombard_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        let (px, py) = g.map.xy(here);
        let max_city_defense_damage = g.d.defines.max_city_defense_damage;

        // iterate over plots at each range
        for dx in -max_range..=max_range {
            for dy in -max_range..=max_range {
                let Some(lp) = g.plot_xy(px, py, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) {
                    if let Some(bombard_city) = g.unit_bombard_target(u, lp) {
                        if g.unit_is_enemy(u, Some(g.city_team(bombard_city)), Some(lp)) && g.cities[bombard_city].defense_damage < max_city_defense_damage {
                            if let Some(turns) = path_turns(g, u, lp, 0) {
                                let last_moves = path_last_node_data(g).0;
                                let cplot = g.city_plot(bombard_city);
                                let mut value = if ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA) { 0 } else { 1 };

                                value += self.player_ai_plot_target_mission_ais(g, owner, cplot, MissionAIType::MISSIONAI_ASSAULT, None, 2) * 3;
                                value += self.player_ai_adjacent_potential_attackers(g, owner, cplot, true);

                                if value > 0 {
                                    value *= 1000;

                                    value /= turns + 1;

                                    if turns == 1 {
                                        // Prefer to have movement remaining to Bombard + Plunder
                                        value *= 1 + 2.min(last_moves);
                                    }

                                    if value > best_value {
                                        best_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                        best_bombard_plot = Some(lp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bbp)) = (best_plot, best_bombard_plot) {
            if g.unit_at_plot(u, bbp) {
                // if we are at the plot from which to bombard, and we have a unit that can bombard this turn, do it
                if bombard_unit_can_bombard_now && g.group_can_bombard(grp, bbp) {
                    push(g, u, MissionType::MISSION_BOMBARD, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));

                    if g.units.contains(u) {
                        // if city bombarded enough, wake up any units that were waiting to bombard this city
                        let bombard_city = g.unit_bombard_target(u, bbp); // is NULL if city cannot be bombarded any more
                        if bombard_city.map_or(true, |bc| g.cities[bc].defense_damage < (max_city_defense_damage * 5) / 6) {
                            let skip = g.units[u].group;
                            self.player_ai_wake_plot_target_mission_ais(g, owner, bbp, MissionAIType::MISSIONAI_BLOCKADE, skip);
                        }
                    }
                }
                // otherwise, skip until next turn, when we will surely bombard
                else if g.unit_can_plunder(u, bbp, false) {
                    push(g, u, MissionType::MISSION_PLUNDER, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                } else {
                    push_skip(g, u);
                }

                return true;
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_BLOCKADE), Some(bbp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_pillage(int iBonusValueThreshold = 0)` (CvUnitAI.cpp:13109-13227)
    pub fn unit_ai_pillage(&self, g: &mut Game, u: UnitId, bonus_value_threshold: i32) -> bool {
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_pillage_plot: Option<PlotIdx> = None;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let (hx, hy) = g.map.xy(here);
        let area = g.unit_area(u);
        let target_city = g.area_get_target_city(area, owner);

        for lp in g.map.plot_ids() {
            if self.unit_ai_plot_valid(g, u, lp) && !g.plot_is_barbarian(lp) {
                if g.unit_potential_war_action(u, lp) {
                    if let Some(wc) = working_city(g, lp) {
                        if !(Some(wc) == target_city) && g.unit_can_pillage(u, lp) {
                            if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                                if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_PILLAGE, Some(grp), 1) == 0 {
                                    // SPEED TWEAK (Sephi): only look for a path to plots within a
                                    // rough maximum distance (no wrapping, as in C++).
                                    let (lx, ly) = g.map.xy(lp);
                                    let x_dist = lx - hx;
                                    let y_dist = ly - hy;
                                    if (x_dist * x_dist) + (y_dist * y_dist) < 200 {
                                        if let Some(turns) = path_turns(g, u, lp, 0) {
                                            let mut value = self.unit_ai_pillage_value(g, u, lp, bonus_value_threshold);
                                            value *= 1000;
                                            value /= turns + 1;
                                            // if not at war with this plot owner, then devalue plot if we already inside this owner's borders
                                            // (because declaring war will pop us some unknown distance away)
                                            let lp_team = g.plot_get_team(lp);
                                            if !g.unit_is_enemy(u, lp_team, None) && g.plot_get_team(here) == lp_team {
                                                value /= 10;
                                            }
                                            if value > best_value {
                                                best_value = value;
                                                best_plot = g.unit_get_path_end_turn_plot(u);
                                                best_pillage_plot = Some(lp);
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

        if let (Some(bp), Some(bpp)) = (best_plot, best_pillage_plot) {
            let bpp_team = g.plot_get_team(bpp);
            if g.unit_at_plot(u, bpp) && !g.unit_is_enemy(u, bpp_team, None) {
                // rather than declare war, just find something else to do, since we may already be deep in enemy territory
                return false;
            }

            if g.unit_at_plot(u, bpp) {
                if g.unit_is_enemy(u, bpp_team, None) {
                    push(g, u, MissionType::MISSION_PILLAGE, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_PILLAGE), Some(bpp));
                    return true;
                }
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_PILLAGE), Some(bpp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_canPillage(CvPlot& kPlot) const` (CvUnitAI.cpp:13229-13265)
    pub fn unit_ai_can_pillage(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        if g.unit_is_enemy(u, g.plot_get_team(plot), Some(plot)) {
            return true;
        }

        if !g.plot_is_owned(plot) {
            let mut pillage_unowned = true;

            for pl in g.civ_player_ids() {
                if !pillage_unowned {
                    break;
                }
                let pl_team = g.players[pl].team;
                if !g.unit_is_enemy(u, Some(pl_team), Some(plot)) {
                    let plot_group = g.plot_get_plot_group(plot, pl);
                    for &c in &g.players[pl].cities {
                        if plot_group == g.plot_get_plot_group(g.city_plot(c), pl) {
                            pillage_unowned = false;
                            break;
                        }
                    }
                }
            }

            if pillage_unowned {
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_pillageRange(int iRange, int iBonusValueThreshold = 0)` (CvUnitAI.cpp:13269-13376)
    pub fn unit_ai_pillage_range(&self, g: &mut Game, u: UnitId, range: i32, bonus_value_threshold: i32) -> bool {
        let search_range = self.unit_ai_search_range(g, u, range);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_pillage_plot: Option<PlotIdx> = None;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let area = g.unit_area(u);
        let target_city = g.area_get_target_city(area, owner);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp) && !g.plot_is_barbarian(lp) {
                    if g.unit_potential_war_action(u, lp) {
                        if let Some(wc) = working_city(g, lp) {
                            if !(Some(wc) == target_city) && g.unit_can_pillage(u, lp) {
                                if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                                    if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_PILLAGE, Some(grp), 0) == 0 {
                                        if let Some(mut turns) = path_turns(g, u, lp, 0) {
                                            if path_last_node_data(g).0 == 0 {
                                                turns += 1;
                                            }

                                            if turns <= range {
                                                let mut value = self.unit_ai_pillage_value(g, u, lp, bonus_value_threshold);

                                                value *= 1000;

                                                value /= turns + 1;

                                                // if not at war with this plot owner, then devalue plot if we already inside this owner's borders
                                                // (because declaring war will pop us some unknown distance away)
                                                let lp_team = g.plot_get_team(lp);
                                                if !g.unit_is_enemy(u, lp_team, None) && g.plot_get_team(here) == lp_team {
                                                    value /= 10;
                                                }

                                                if value > best_value {
                                                    best_value = value;
                                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                                    best_pillage_plot = Some(lp);
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

        if let (Some(bp), Some(bpp)) = (best_plot, best_pillage_plot) {
            let bpp_team = g.plot_get_team(bpp);
            if g.unit_at_plot(u, bpp) && !g.unit_is_enemy(u, bpp_team, None) {
                // rather than declare war, just find something else to do, since we may already be deep in enemy territory
                return false;
            }

            if g.unit_at_plot(u, bpp) {
                if g.unit_is_enemy(u, bpp_team, None) {
                    push(g, u, MissionType::MISSION_PILLAGE, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_PILLAGE), Some(bpp));
                    return true;
                }
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_PILLAGE), Some(bpp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_found()` (CvUnitAI.cpp:13380-13486)
    pub fn unit_ai_found(&self, g: &mut Game, u: UnitId) -> bool {
        // (the commented-out original BTS whole-map search is not ported)
        let mut best_found_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_found_plot: Option<PlotIdx> = None;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let unit_area = g.map.plots[here].area;

        let num_sites = self.player_ai_get_num_city_sites(g, owner);
        for i in 0..num_sites {
            let Some(site) = self.player_ai_get_city_site(g, owner, i) else { continue };
            if g.map.plots[site].area == unit_area {
                if g.unit_can_found(u, site, false) {
                    if !g.plot_is_visible_enemy_unit_for_unit(site, u) {
                        if self.player_ai_plot_target_mission_ais(g, owner, site, MissionAIType::MISSIONAI_FOUND, Some(grp), 0) == 0 {
                            if g.group_can_defend(grp) || self.player_ai_plot_target_mission_ais(g, owner, site, MissionAIType::MISSIONAI_GUARD_CITY, None, 0) > 0 {
                                if let Some(turns) = path_turns(g, u, site, MOVE_SAFE_TERRITORY) {
                                    let mut value = g.plot_get_found_value(site, owner);
                                    value *= 1000;
                                    value /= turns + 1;
                                    if value > best_found_value {
                                        best_found_value = value;
                                        best_plot = g.unit_get_path_end_turn_plot(u);
                                        best_found_plot = Some(site);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bfp)) = (best_plot, best_found_plot) {
            if g.unit_at_plot(u, bfp) {
                push(g, u, MissionType::MISSION_FOUND, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            } else {
                push_move(g, u, bp, MOVE_SAFE_TERRITORY, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_foundRange(int iRange, bool bFollow = false)` (CvUnitAI.cpp:13490-13563)
    pub fn unit_ai_found_range(&self, g: &mut Game, u: UnitId, range: i32, follow: bool) -> bool {
        let search_range = self.unit_ai_search_range(g, u, range);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_found_plot: Option<PlotIdx> = None;

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };

                if self.unit_ai_plot_valid(g, u, lp)
                    && (lp != here
                        || self.player_ai_get_plot_danger(g, owner, lp, 1, true)
                            <= g.plot_plot_count(lp, Some(Game::puf_can_defend), -1, -1, Some(owner), None, None, -1, -1))
                {
                    if g.unit_can_found(u, lp, false) {
                        let value = g.plot_get_found_value(lp, owner);

                        if value > best_value {
                            if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                                if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_FOUND, Some(grp), 3) == 0 {
                                    if let Some(turns) = path_turns(g, u, lp, MOVE_SAFE_TERRITORY) {
                                        if turns <= range {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                            best_found_plot = Some(lp);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bfp)) = (best_plot, best_found_plot) {
            if g.unit_at_plot(u, bfp) {
                push(g, u, MissionType::MISSION_FOUND, -1, -1, 0, false, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            } else if !follow {
                push_move(g, u, bp, MOVE_SAFE_TERRITORY, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_assaultSeaTransport(bool bBarbarian = false)` (CvUnitAI.cpp:13567-13799)
    pub fn unit_ai_assault_sea_transport(&self, g: &mut Game, u: UnitId, barbarian: bool) -> bool {
        let is_attack_city = g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_ATTACK_CITY) > 0;

        if !g.unit_can_cargo_all_move(u) {
            return false;
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);

        let group_cargo: Vec<UnitId> = g.map.plots[here]
            .units
            .iter()
            .copied()
            .filter(|&lu| transport_of(g, lu).map_or(false, |t| g.units[t].group == Some(grp)))
            .collect();

        let cargo = g.group_get_cargo(grp);
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_assault_plot: Option<PlotIdx> = None;

        let is_minor_civ = g.player_is_minor_civ(owner);
        let river_attack_modifier = g.d.defines.river_attack_modifier;

        for lp in g.map.plot_ids() {
            if !g.plot_is_coastal_land(lp, -1) {
                continue;
            }
            let Some(lp_owner) = g.map.plots[lp].owner else { continue };
            if !((barbarian || !g.plot_is_barbarian(lp)) || is_minor_civ) {
                continue;
            }
            if !g.unit_is_potential_enemy(u, g.plot_get_team(lp), Some(lp)) {
                continue;
            }
            let Some(lp_area) = g.map.plots[lp].area else { continue };
            let target_cities = g.map.areas[lp_area].cities_per_player[lp_owner];
            if target_cities <= 0 {
                continue;
            }

            let mut can_cargo_all_unload = true;
            let visible_enemy_defenders = g.plot_get_num_visible_enemy_defenders(lp, u);
            if visible_enemy_defenders > 0 {
                for &attacker in &group_cargo {
                    let attacker_owner = g.units[attacker].owner;
                    let defender = g.plot_get_best_defender(lp, None, Some(attacker_owner), Some(attacker), true, false, false);
                    if defender.map_or(true, |d| !g.unit_can_attack_unit(attacker, d)) {
                        can_cargo_all_unload = false;
                        break;
                    }
                }
            }

            if !can_cargo_all_unload {
                continue;
            }

            let Some(mut turns) = path_turns(g, u, lp, 0) else { continue };

            let mut value = 1;

            if !is_attack_city {
                value += self.unit_ai_pillage_value(g, u, lp, 15) * 10;
            }

            let assaults_here = self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_ASSAULT, Some(grp), 0);

            value += assaults_here * 100;

            let mut city = plot_city(g, lp);

            if city.is_none() {
                let (lx, ly) = g.map.xy(lp);
                for j in 0..DirectionType::NUM {
                    if let Some(adj) = g.plot_direction(lx, ly, DirectionType::from_usize(j)) {
                        city = plot_city(g, adj);

                        if let Some(c) = city {
                            if g.cities[c].owner == lp_owner {
                                break;
                            } else {
                                city = None;
                            }
                        }
                    }
                }
            }

            if let Some(c) = city {
                let cplot = g.city_plot(c);
                if !g.plot_is_river_crossing(lp, g.direction_xy_plots(lp, cplot)) {
                    value += 50 * -river_attack_modifier;
                }

                value += 15 * g.plot_defense_modifier(lp, Some(team), false);
                value += 1000;
                value += self.player_ai_adjacent_potential_attackers(g, owner, cplot, false) * 200;

                if turns == 1 {
                    value += self.rand(50, "AI Assault");
                }
            }

            if turns == 1 {
                if let Some(c) = city {
                    if g.map.areas[g.city_area(c)].num_cities > 1 {
                        value *= 2;
                    }
                }
            }

            value *= 1000;

            if target_cities <= assaults_here {
                value /= 2;
            }

            if target_cities == 1 {
                if cargo > 7 {
                    value *= 3;
                    value /= cargo - 4;
                }
            }

            if g.plot_is_city(lp, false, None) {
                if visible_enemy_defenders * 3 > cargo {
                    value /= 10;
                } else {
                    value *= cargo;
                    value /= 1.max(visible_enemy_defenders * 3);
                }
            } else if 0 == visible_enemy_defenders {
                value *= 4;
                value /= 3;
            } else {
                value /= visible_enemy_defenders;
            }

            // if more than 3 turns to get there, then put some randomness into our preference of distance
            // +/- 33%
            if turns > 3 {
                let path_adjustment = self.rand(67, "AI Assault Target");

                turns *= 66 + path_adjustment;
                turns /= 100;
            }

            value /= turns + 1;

            if value > best_value {
                best_value = value;
                best_plot = g.unit_get_path_end_turn_plot(u);
                best_assault_plot = Some(lp);
            }
        }

        if let (Some(bp), Some(bap)) = (best_plot, best_assault_plot) {
            let (bx, by) = g.map.xy(bp);
            let (ax, ay) = g.map.xy(bap);
            if bp == bap || g.step_distance(bx, by, ax, ay) == 1 {
                if g.unit_at_plot(u, bap) {
                    g.group_unload_all(grp); // XXX is this dangerous (not pushing a mission...) XXX air units?
                    return true;
                } else {
                    push_move(g, u, bap, 0, Some(MissionAIType::MISSIONAI_ASSAULT), Some(bap));
                    return true;
                }
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_ASSAULT), Some(bap));
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_settlerSeaTransport()` (CvUnitAI.cpp:13803-14050)
    pub fn unit_ai_settler_sea_transport(&self, g: &mut Game, u: UnitId) -> bool {
        if !g.unit_can_cargo_all_move(u) {
            return false;
        }

        // New logic should allow some new tricks like unloading settlers when a better site
        // opens up locally and delivering settlers to inland sites.

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);
        let here = g.unit_plot(u);
        let unit_area = g.map.plots[here].area;

        let mut settler_unit: Option<UnitId> = None;
        for &lu in &g.map.plots[here].units {
            if g.units[lu].transport_unit == Some(u) {
                if g.units[lu].ai_type == Some(UnitAIType::UNITAI_SETTLE) {
                    settler_unit = Some(lu);
                    break;
                }
            }
        }

        let mut area_best_found_value = 0;
        let mut area_best_plot: Option<PlotIdx> = None;

        let mut other_area_best_found_value = 0;
        let mut other_area_best_plot: Option<PlotIdx> = None;

        let num_sites = self.player_ai_get_num_city_sites(g, owner);
        for i in 0..num_sites {
            let Some(site) = self.player_ai_get_city_site(g, owner, i) else { continue };
            if self.player_ai_plot_target_mission_ais(g, owner, site, MissionAIType::MISSIONAI_FOUND, Some(grp), 0) == 0 {
                let value = g.plot_get_found_value(site, owner);

                // BETTER_BTS_AI_MOD 01/13/09 jdog5000 (Settler AI): only count city sites we can get to
                // PORT ADAPT: C++ asserts a settler is aboard (and would crash without one when a
                // site is in this area); without one the site counts as not reachable.
                let reachable = g.map.plots[site].area == unit_area
                    && settler_unit.map_or(false, |s| g.unit_generate_path(s, site, MOVE_SAFE_TERRITORY, true, None));
                if reachable {
                    if value > area_best_found_value {
                        area_best_found_value = value;
                        area_best_plot = Some(site);
                    }
                } else if value > other_area_best_found_value {
                    other_area_best_found_value = value;
                    other_area_best_plot = Some(site);
                }
            }
        }
        if 0 == area_best_found_value && 0 == other_area_best_found_value {
            return false;
        }

        if area_best_found_value > other_area_best_found_value {
            // let the settler walk.
            g.unit_unload_all(u);
            return true;
        }

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_found_plot: Option<PlotIdx> = None;

        for i in 0..num_sites {
            let Some(site) = self.player_ai_get_city_site(g, owner, i) else { continue };
            if !g.plot_is_visible_enemy_unit_for_unit(site, u) {
                if self.player_ai_plot_target_mission_ais(g, owner, site, MissionAIType::MISSIONAI_FOUND, Some(grp), 4) == 0 {
                    if let Some(turns) = path_turns(g, u, site, 0) {
                        let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                        let mut value = g.plot_get_found_value(site, owner);
                        value *= 1000;
                        value /= 2 + turns;

                        if value > best_value {
                            best_value = value;
                            best_plot = end_turn_plot;
                            best_found_plot = Some(site);
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bfp)) = (best_plot, best_found_plot) {
            let (bx, by) = g.map.xy(bp);
            let (fx, fy) = g.map.xy(bfp);
            if bp == bfp || g.step_distance(bx, by, fx, fy) == 1 {
                if g.unit_at_plot(u, bfp) {
                    g.unit_unload_all(u); // XXX is this dangerous (not pushing a mission...) XXX air units?
                    return true;
                } else {
                    push_move(g, u, bfp, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                    return true;
                }
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            }
        }

        // Try original logic
        // (sometimes new logic breaks)
        let here = g.unit_plot(u);

        best_value = 0;
        best_plot = None;
        best_found_plot = None;

        let min_found_value = self.player_ai_get_min_found_value(g, owner);

        for lp in g.map.plot_ids() {
            if g.plot_is_coastal_land(lp, -1) {
                let value = g.plot_get_found_value(lp, owner);

                if value > best_value && value >= min_found_value {
                    let mut valid = false;

                    for &lu in &g.map.plots[here].units {
                        if g.units[lu].transport_unit == Some(u) {
                            if g.unit_can_found(lu, lp, false) {
                                valid = true;
                                break;
                            }
                        }
                    }

                    if valid {
                        if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                            if self.player_ai_plot_target_mission_ais(g, owner, lp, MissionAIType::MISSIONAI_FOUND, Some(grp), 4) == 0 {
                                if g.unit_generate_path(u, lp, 0, true, None) {
                                    best_value = value;
                                    best_plot = g.unit_get_path_end_turn_plot(u);
                                    best_found_plot = Some(lp);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(bp), Some(bfp)) = (best_plot, best_found_plot) {
            let (bx, by) = g.map.xy(bp);
            let (fx, fy) = g.map.xy(bfp);
            if bp == bfp || g.step_distance(bx, by, fx, fy) == 1 {
                if g.unit_at_plot(u, bfp) {
                    g.unit_unload_all(u); // XXX is this dangerous (not pushing a mission...) XXX air units?
                    return true;
                } else {
                    push_move(g, u, bfp, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                    return true;
                }
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(bfp));
                return true;
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_settlerSeaFerry()` (CvUnitAI.cpp:14054-14120)
    pub fn unit_ai_settler_sea_ferry(&self, g: &mut Game, u: UnitId) -> bool {
        if !g.unit_can_cargo_all_move(u) {
            return false;
        }

        let owner = g.units[u].owner;
        let grp = grp_of(g, u);

        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for c in g.players[owner].cities.clone() {
            let mut value = self.city_ai_get_workers_needed(g, c);
            if value > 0 {
                let cplot = g.city_plot(c);
                value -= self.player_ai_plot_target_mission_ais(g, owner, cplot, MissionAIType::MISSIONAI_FOUND, Some(grp), 0);
                if value > 0 {
                    if let Some(turns) = path_turns(g, u, cplot, 0) {
                        let carea = g.city_area(c);
                        value += 0.max(self.player_ai_needed_workers(g, owner, carea) - self.player_ai_total_area_unit_ais(g, owner, carea, UnitAIType::UNITAI_WORKER));
                        value *= 1000;
                        value /= 4 + turns;
                        if g.unit_at_plot(u, cplot) {
                            value += 100;
                        } else {
                            value += self.rand(100, "AI settler sea ferry");
                        }
                        if value > best_value {
                            best_value = value;
                            best_plot = Some(cplot);
                        }
                    }
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_at_plot(u, bp) {
                g.unit_unload_all(u); // XXX is this dangerous (not pushing a mission...) XXX air units?
                return true;
            } else {
                push_move(g, u, bp, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(bp));
                return true;
            }
        }
        false
    }
}
