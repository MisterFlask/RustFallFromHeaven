//! Port of CvTeamAI.cpp:2219-4403, CvSelectionGroupAI.cpp:28-1003, CvGameAI.cpp:24-110 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES (for the integrator):
//! - `team_ai_set_war_plan` takes `WarPlanType`, so it cannot express `NO_WARPLAN`; this file
//!   calls `g.team_ai_set_war_plan(t, other, None, true)` (ffh-core, the same C++) for that.
//!   Suggest `new_value: Option<WarPlanType>`.
//! - `group_ai_set_mission_ai` takes `MissionAIType`, `PlotIdx`, `UnitId`, so callers cannot pass
//!   `NO_MISSIONAI` / NULL; suggest `Option<..>` for all three (it delegates to
//!   `g.group_ai_set_mission_ai`, which already takes options).
//! - `plot_ai_sum_strength` (py_2.rs) takes `owner: PlayerId`, but `AI_compareStacks` passes
//!   `NO_PLAYER`; this file uses a private port (`plot_ai_sum_strength_any`) with an optional
//!   owner. Suggest `owner: Option<PlayerId>` there.
//! - ffh-core already runs `team_ai_do_counter` and its own `team_ai_update_worst_enemy` in
//!   `CvTeam::doTurn` (team/tech.rs): calling `team_ai_do_counter` again from `AI_doTurnPre`
//!   would advance the counters twice per turn. `team_ai_update_worst_enemy` here uses this
//!   crate's attitude values (ffh-core's uses attitude indices), so calling it from
//!   `AI_doTurnPost` overwrites the core value with the C++ one (intended).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// `GC.getLeaderHeadInfo(GET_PLAYER(p).getPersonalityType())`.
// PORT ADAPT: a player without a personality (C++ would index NO_LEADER) is skipped by callers.
fn leader_info(g: &Game, p: PlayerId) -> Option<&'static LeaderHeadInfo> {
    let d: &'static GameData = g.d;
    g.players[p].personality_type.map(|l| &d.leader_heads[l])
}

/// C++ `eAttitude <= threshold` with an `AttitudeTypes` threshold that may be `NO_ATTITUDE` (-1).
fn attitude_at_most(attitude: AttitudeType, threshold: Option<AttitudeType>) -> bool {
    attitude.i() <= threshold.map_or(-1, |a| a.i())
}

/// The team average of a leader-head value (`AI_maxWarRand` and its siblings): the sum over
/// alive members, divided by the member count when it is above `min_count_to_divide`.
fn team_leader_average(g: &Game, t: TeamId, min_count_to_divide: i32, f: impl Fn(&LeaderHeadInfo) -> i32) -> i32 {
    let mut rand = 0;
    let mut count = 0;
    for p in g.player_ids() {
        if g.players[p].alive {
            if g.players[p].team == t {
                if let Some(li) = leader_info(g, p) {
                    rand += f(li);
                    count += 1;
                }
            }
        }
    }

    if count > min_count_to_divide {
        rand /= count;
    }

    rand
}

/// CvCity::isHolyCity() (any religion), not in ffh-core.
fn city_is_holy_city_any(g: &Game, c: CityId) -> bool {
    g.d.religions.keys().any(|r| g.city_is_holy_city(c, r))
}

/// CvCity::isHeadquarters() (any corporation), not in ffh-core.
fn city_is_headquarters_any(g: &Game, c: CityId) -> bool {
    g.d.corporations.keys().any(|corp| g.city_is_headquarters(c, corp))
}

/// C++ `nextUnitNode` of the group list: the unit after `u` (None when `u` is last or gone).
fn next_group_unit(g: &Game, grp: GroupId, u: UnitId) -> Option<UnitId> {
    let units = &g.groups[grp].units;
    let i = units.iter().position(|&x| x == u)?;
    units.get(i + 1).copied()
}

/// C++ `CvPlot::AI_sumStrength(eOwner, eAttackingPlayer, eDomainType, bDefensiveBonuses,
/// bTestAtWar, bTestPotentialEnemy)` with `eOwner` allowed to be `NO_PLAYER` (`AI_compareStacks`
/// passes NO_PLAYER; py_2's port takes a `PlayerId`).
#[allow(clippy::too_many_arguments)]
fn plot_ai_sum_strength_any(
    g: &Game,
    plot: PlotIdx,
    owner: Option<PlayerId>,
    attacking_player: Option<PlayerId>,
    domain_type: Option<DomainType>,
    defensive_bonuses: bool,
    test_at_war: bool,
    test_potential_enemy: bool,
) -> i32 {
    let mut str_sum = 0;
    let attacking_team = attacking_player.map(|p| g.players[p].team);

    for &loop_unit in &g.map.plots[plot].units {
        if owner.is_none() || Some(g.units[loop_unit].owner) == owner {
            if attacking_team.map_or(true, |at| !g.unit_is_invisible(loop_unit, at, false)) {
                if !test_at_war || attacking_team.map_or(true, |at| g.at_war(at, g.unit_get_team(loop_unit))) {
                    if !test_potential_enemy || attacking_team.map_or(true, |at| g.unit_is_potential_enemy(loop_unit, Some(at), Some(plot))) {
                        // we may want to be more sophisticated about domains
                        // somewhere we need to check to see if this is a city, if so, only land units can defend here, etc
                        if domain_type.is_none() || Some(g.unit_get_domain_type(loop_unit)) == domain_type {
                            let p = if defensive_bonuses { Some(plot) } else { None };

                            str_sum += g.unit_curr_effective_str(loop_unit, p, None, None);
                        }
                    }
                }
            }
        }
    }

    str_sum
}

impl Cai {
    /// C++ `int CvTeamAI::AI_defensivePactTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:2219-2222)
    pub fn team_ai_defensive_pact_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        (g.teams[t].num_cities + g.teams[team].num_cities) * 3
    }

    /// C++ `DenialTypes CvTeamAI::AI_defensivePactTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2225-2265)
    pub fn team_ai_defensive_pact_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        debug_assert!(team != t, "shouldn't call this function on ourselves");

        if g.team_is_human(t) {
            return None;
        }

        if g.count_civ_teams_alive() == 2 {
            return Some(DenialType::DENIAL_NO_GAIN);
        }

        if self.team_ai_get_worst_enemy(g, t) == Some(team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in g.player_ids() {
            if g.players[p].alive {
                if g.players[p].team == t {
                    if let Some(li) = leader_info(g, p) {
                        if attitude_at_most(attitude, li.defensive_pact_refuse_attitude_threshold) {
                            return Some(DenialType::DENIAL_ATTITUDE);
                        }
                    }
                }
            }
        }

        None
    }

    /// C++ `DenialTypes CvTeamAI::AI_permanentAllianceTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2268-2320)
    pub fn team_ai_permanent_alliance_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        debug_assert!(team != t, "shouldn't call this function on ourselves");

        if g.team_is_human(t) {
            return None;
        }

        if self.team_ai_get_worst_enemy(g, t) == Some(team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        if (g.team_get_power(t, true) + g.team_get_power(team, true)) > (g.count_total_civ_power() / 2) {
            if g.team_get_power(t, true) > g.team_get_power(team, true) {
                return Some(DenialType::DENIAL_POWER_US);
            } else {
                return Some(DenialType::DENIAL_POWER_YOU);
            }
        }

        if (self.team_ai_get_defensive_pact_counter(g, t, team) + self.team_ai_get_share_war_counter(g, t, team)) < 40 {
            return Some(DenialType::DENIAL_NOT_ALLIED);
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in g.player_ids() {
            if g.players[p].alive {
                if g.players[p].team == t {
                    if let Some(li) = leader_info(g, p) {
                        if attitude_at_most(attitude, li.permanent_alliance_refuse_attitude_threshold) {
                            return Some(DenialType::DENIAL_ATTITUDE);
                        }
                    }
                }
            }
        }

        None
    }

    /// C++ `TeamTypes CvTeamAI::AI_getWorstEnemy() const` (CvTeamAI.cpp:2323-2326)
    pub fn team_ai_get_worst_enemy(&self, g: &Game, t: TeamId) -> Option<TeamId> {
        g.teams[t].ai_worst_enemy
    }

    /// C++ `void CvTeamAI::AI_updateWorstEnemy()` (CvTeamAI.cpp:2329-2361)
    ///
    /// Not delegated to `g.team_ai_update_worst_enemy`: that one ranks by attitude indices
    /// (ffh-core deviation); this uses the C++ attitude values (`team_ai_get_attitude_val`).
    pub fn team_ai_update_worst_enemy(&self, g: &mut Game, t: TeamId) {
        let mut best_team = None;
        let mut best_value = MAX_INT;

        for loop_team in g.civ_team_ids() {
            if g.team_is_alive(loop_team) {
                if loop_team != t && !g.teams[loop_team].vassal[t] {
                    if g.teams[t].has_met[loop_team] {
                        if self.team_ai_get_attitude(g, t, loop_team, true) < AttitudeType::ATTITUDE_CAUTIOUS {
                            let value = self.team_ai_get_attitude_val(g, t, loop_team, true);
                            if value < best_value {
                                best_value = value;
                                best_team = Some(loop_team);
                            }
                        }
                    }
                }
            }
        }

        g.teams[t].ai_worst_enemy = best_team;
    }

    /// C++ `int CvTeamAI::AI_getWarPlanStateCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2364-2369)
    pub fn team_ai_get_war_plan_state_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_war_plan_state_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setWarPlanStateCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2372-2378)
    pub fn team_ai_set_war_plan_state_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_war_plan_state_counter[index] = new_value;
        // FAssert: self.team_ai_get_war_plan_state_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeWarPlanStateCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2381-2384)
    pub fn team_ai_change_war_plan_state_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_war_plan_state_counter(g, t, index) + change;
        self.team_ai_set_war_plan_state_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getAtWarCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2387-2392)
    pub fn team_ai_get_at_war_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_at_war_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setAtWarCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2395-2401)
    pub fn team_ai_set_at_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_at_war_counter[index] = new_value;
        // FAssert: self.team_ai_get_at_war_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeAtWarCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2404-2407)
    pub fn team_ai_change_at_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_at_war_counter(g, t, index) + change;
        self.team_ai_set_at_war_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getAtPeaceCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2410-2415)
    pub fn team_ai_get_at_peace_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_at_peace_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setAtPeaceCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2418-2424)
    pub fn team_ai_set_at_peace_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_at_peace_counter[index] = new_value;
        // FAssert: self.team_ai_get_at_peace_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeAtPeaceCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2427-2430)
    pub fn team_ai_change_at_peace_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_at_peace_counter(g, t, index) + change;
        self.team_ai_set_at_peace_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getHasMetCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2433-2438)
    pub fn team_ai_get_has_met_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_has_met_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setHasMetCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2441-2447)
    pub fn team_ai_set_has_met_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_has_met_counter[index] = new_value;
        // FAssert: self.team_ai_get_has_met_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeHasMetCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2450-2453)
    pub fn team_ai_change_has_met_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_has_met_counter(g, t, index) + change;
        self.team_ai_set_has_met_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getOpenBordersCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2456-2461)
    pub fn team_ai_get_open_borders_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_open_borders_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setOpenBordersCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2464-2470)
    pub fn team_ai_set_open_borders_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_open_borders_counter[index] = new_value;
        // FAssert: self.team_ai_get_open_borders_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeOpenBordersCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2473-2476)
    pub fn team_ai_change_open_borders_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_open_borders_counter(g, t, index) + change;
        self.team_ai_set_open_borders_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getDefensivePactCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2479-2484)
    pub fn team_ai_get_defensive_pact_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_defensive_pact_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setDefensivePactCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2487-2493)
    pub fn team_ai_set_defensive_pact_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_defensive_pact_counter[index] = new_value;
        // FAssert: self.team_ai_get_defensive_pact_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeDefensivePactCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2496-2499)
    pub fn team_ai_change_defensive_pact_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_defensive_pact_counter(g, t, index) + change;
        self.team_ai_set_defensive_pact_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getShareWarCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2502-2507)
    pub fn team_ai_get_share_war_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_share_war_counter[index]
    }

    /// C++ `void CvTeamAI::AI_setShareWarCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2510-2516)
    pub fn team_ai_set_share_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_share_war_counter[index] = new_value;
        // FAssert: self.team_ai_get_share_war_counter(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeShareWarCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2519-2522)
    pub fn team_ai_change_share_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_share_war_counter(g, t, index) + change;
        self.team_ai_set_share_war_counter(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getWarSuccess(TeamTypes eIndex) const` (CvTeamAI.cpp:2525-2530)
    pub fn team_ai_get_war_success(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_war_success[index]
    }

    /// C++ `void CvTeamAI::AI_setWarSuccess(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2533-2539)
    pub fn team_ai_set_war_success(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_war_success[index] = new_value;
        // FAssert: self.team_ai_get_war_success(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeWarSuccess(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2542-2545)
    pub fn team_ai_change_war_success(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_war_success(g, t, index) + change;
        self.team_ai_set_war_success(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getEnemyPeacetimeTradeValue(TeamTypes eIndex) const` (CvTeamAI.cpp:2548-2553)
    pub fn team_ai_get_enemy_peacetime_trade_value(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_enemy_peacetime_trade_value[index]
    }

    /// C++ `void CvTeamAI::AI_setEnemyPeacetimeTradeValue(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2556-2562)
    pub fn team_ai_set_enemy_peacetime_trade_value(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_enemy_peacetime_trade_value[index] = new_value;
        // FAssert: self.team_ai_get_enemy_peacetime_trade_value(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeEnemyPeacetimeTradeValue(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2565-2568)
    pub fn team_ai_change_enemy_peacetime_trade_value(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_enemy_peacetime_trade_value(g, t, index) + change;
        self.team_ai_set_enemy_peacetime_trade_value(g, t, index, v);
    }

    /// C++ `int CvTeamAI::AI_getEnemyPeacetimeGrantValue(TeamTypes eIndex) const` (CvTeamAI.cpp:2571-2576)
    pub fn team_ai_get_enemy_peacetime_grant_value(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        g.teams[t].ai_enemy_peacetime_grant_value[index]
    }

    /// C++ `void CvTeamAI::AI_setEnemyPeacetimeGrantValue(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2579-2585)
    pub fn team_ai_set_enemy_peacetime_grant_value(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        g.teams[t].ai_enemy_peacetime_grant_value[index] = new_value;
        // FAssert: self.team_ai_get_enemy_peacetime_grant_value(g, t, index) >= 0
    }

    /// C++ `void CvTeamAI::AI_changeEnemyPeacetimeGrantValue(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2588-2591)
    pub fn team_ai_change_enemy_peacetime_grant_value(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        let v = self.team_ai_get_enemy_peacetime_grant_value(g, t, index) + change;
        self.team_ai_set_enemy_peacetime_grant_value(g, t, index, v);
    }

    /// C++ `WarPlanTypes CvTeamAI::AI_getWarPlan(TeamTypes eIndex) const` (CvTeamAI.cpp:2594-2600)
    pub fn team_ai_get_war_plan(&self, g: &Game, t: TeamId, index: TeamId) -> Option<WarPlanType> {
        debug_assert!(index != t || g.teams[t].ai_war_plan[index].is_none());
        g.teams[t].ai_war_plan[index]
    }

    /// C++ `bool CvTeamAI::AI_isChosenWar(TeamTypes eIndex) const` (CvTeamAI.cpp:2603-2621)
    pub fn team_ai_is_chosen_war(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        match self.team_ai_get_war_plan(g, t, index) {
            Some(WarPlanType::WARPLAN_ATTACKED_RECENT) | Some(WarPlanType::WARPLAN_ATTACKED) => false,
            Some(WarPlanType::WARPLAN_PREPARING_LIMITED)
            | Some(WarPlanType::WARPLAN_PREPARING_TOTAL)
            | Some(WarPlanType::WARPLAN_LIMITED)
            | Some(WarPlanType::WARPLAN_TOTAL)
            | Some(WarPlanType::WARPLAN_DOGPILE) => true,
            _ => false,
        }
    }

    /// C++ `bool CvTeamAI::AI_isSneakAttackPreparing(TeamTypes eIndex) const` (CvTeamAI.cpp:2624-2627)
    pub fn team_ai_is_sneak_attack_preparing(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        let plan = self.team_ai_get_war_plan(g, t, index);
        plan == Some(WarPlanType::WARPLAN_PREPARING_LIMITED) || plan == Some(WarPlanType::WARPLAN_PREPARING_TOTAL)
    }

    /// C++ `bool CvTeamAI::AI_isSneakAttackReady(TeamTypes eIndex) const` (CvTeamAI.cpp:2630-2633)
    pub fn team_ai_is_sneak_attack_ready(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        self.team_ai_is_chosen_war(g, t, index) && !self.team_ai_is_sneak_attack_preparing(g, t, index)
    }

    /// C++ `void CvTeamAI::AI_setWarPlan(TeamTypes eIndex, WarPlanTypes eNewValue, bool bWar = true)` (CvTeamAI.cpp:2636-2667)
    ///
    /// Delegates to ffh-core's port (same C++: state counter reset, `AI_updateAreaStragies`
    /// through the team leader's controller hook, `AI_makeProductionDirty` for AI members).
    pub fn team_ai_set_war_plan(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: WarPlanType, war: bool) {
        g.team_ai_set_war_plan(t, index, Some(new_value), war);
    }

    /// C++ `int CvTeamAI::AI_teamCloseness(TeamTypes eIndex, int iMaxDistance = -1) const` (CvTeamAI.cpp:2671-2704)
    //if this number is over 0 the teams are "close"
    //this may be expensive to run, kinda O(N^2)...
    pub fn team_ai_team_closeness(&self, g: &Game, t: TeamId, index: TeamId, max_distance: i32) -> i32 {
        let mut max_distance = max_distance;
        if max_distance == -1 {
            max_distance = DEFAULT_PLAYER_CLOSENESS;
        }

        debug_assert!(index != t);
        let mut value = 0;
        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == t {
                    for j in g.player_ids() {
                        if g.players[j].alive {
                            if g.players[j].team == index {
                                value += self.player_ai_player_closeness(g, i, j, max_distance);
                            }
                        }
                    }
                }
            }
        }

        value
    }

    /// C++ `int CvTeamAI::AI_noTechTradeThreshold() const` (CvTeamAI.cpp:2754-2780)
    pub fn team_ai_no_tech_trade_threshold(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.no_tech_trade_threshold)
    }

    /// C++ `int CvTeamAI::AI_techTradeKnownPercent() const` (CvTeamAI.cpp:2783-2809)
    pub fn team_ai_tech_trade_known_percent(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.tech_trade_known_percent)
    }

    /// C++ `int CvTeamAI::AI_maxWarRand() const` (CvTeamAI.cpp:2812-2838)
    pub fn team_ai_max_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.max_war_rand)
    }

    /// C++ `int CvTeamAI::AI_maxWarNearbyPowerRatio() const` (CvTeamAI.cpp:2841-2867)
    pub fn team_ai_max_war_nearby_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        // C++ quirk: divides only when iCount > 1 (same result).
        team_leader_average(g, t, 1, |li| li.max_war_nearby_power_ratio)
    }

    /// C++ `int CvTeamAI::AI_maxWarDistantPowerRatio() const` (CvTeamAI.cpp:2870-2896)
    pub fn team_ai_max_war_distant_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        // C++ quirk: divides only when iCount > 1 (same result).
        team_leader_average(g, t, 1, |li| li.max_war_distant_power_ratio)
    }

    /// C++ `int CvTeamAI::AI_maxWarMinAdjacentLandPercent() const` (CvTeamAI.cpp:2899-2925)
    pub fn team_ai_max_war_min_adjacent_land_percent(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.max_war_min_adjacent_land_percent)
    }

    /// C++ `int CvTeamAI::AI_limitedWarRand() const` (CvTeamAI.cpp:2928-2954)
    pub fn team_ai_limited_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.limited_war_rand)
    }

    /// C++ `int CvTeamAI::AI_limitedWarPowerRatio() const` (CvTeamAI.cpp:2957-2984)
    pub fn team_ai_limited_war_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.limited_war_power_ratio)
    }

    /// C++ `int CvTeamAI::AI_dogpileWarRand() const` (CvTeamAI.cpp:2987-3013)
    pub fn team_ai_dogpile_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.dogpile_war_rand)
    }

    /// C++ `int CvTeamAI::AI_makePeaceRand() const` (CvTeamAI.cpp:3016-3042)
    pub fn team_ai_make_peace_rand(&self, g: &Game, t: TeamId) -> i32 {
        team_leader_average(g, t, 0, |li| li.make_peace_rand)
    }

    /// C++ `int CvTeamAI::AI_noWarAttitudeProb(AttitudeTypes eAttitude) const` (CvTeamAI.cpp:3045-3071)
    pub fn team_ai_no_war_attitude_prob(&self, g: &Game, t: TeamId, attitude: AttitudeType) -> i32 {
        // C++ quirk: divides only when iCount > 1 (same result).
        team_leader_average(g, t, 1, |li| li.no_war_attitude_prob[attitude])
    }

    /// C++ `void CvTeamAI::AI_doCounter()` (CvTeamAI.cpp:3074-3127)
    ///
    /// Delegates to ffh-core's port (same C++, `AI_shareWar` included). Note ffh-core's
    /// `CvTeam::doTurn` already calls it once per turn.
    pub fn team_ai_do_counter(&self, g: &mut Game, t: TeamId) {
        g.team_ai_do_counter(t);
    }

    /// C++ `void CvTeamAI::AI_doWar()` (CvTeamAI.cpp:3130-3793)
    pub fn team_ai_do_war(&self, g: &mut Game, t: TeamId) {
        // FAssert: !g.team_is_human(t)
        // FAssert: !g.team_is_barbarian(t)
        // FAssert: !g.team_is_minor_civ(t)

        if g.team_is_a_vassal(t) {
            return;
        }

        // allow python to handle it: CvGameUtils.py AI_doWar returns False (CvGameUtils.py:2996),
        // so the C++ always goes on.

        for i in g.civ_team_ids() {
            if g.team_is_alive(i) {
                if self.team_ai_get_war_plan(g, t, i).is_some() {
                    let mut time_modifier = 100;
                    {
                        let our_power = g.team_get_power(t, true);
                        let their_power = g.team_get_power(i, true);

                        // ADDON (Fix Division by zero) Sephi
                        // Orig: int iRatio = (1 + iOurPower) / (1 + iTheirPower);
                        let mut divisor = their_power;
                        if divisor == 0 {
                            divisor += 1;
                        }
                        // FIX: Sephi's fix computes iDivisor but still divides by iTheirPower
                        // (a division by zero when it is 0); divide by iDivisor.
                        let ratio = (1 + our_power) / divisor;

                        if ratio > 200 {
                            time_modifier *= 100;
                            time_modifier /= ratio - 100;
                        }

                        time_modifier *= 50 + g.d.game_speeds[g.game_speed_type()].train_percent;
                        time_modifier /= 150;
                        // FAssert: time_modifier > 0
                    }

                    if self.team_ai_get_war_plan(g, t, i) == Some(WarPlanType::WARPLAN_ATTACKED_RECENT) {
                        // FAssert: g.teams[t].at_war[i]

                        let limit = if self.team_ai_is_land_target(g, i, t) { 9 } else { 3 };
                        if self.team_ai_get_at_war_counter(g, t, i) > limit {
                            self.team_ai_set_war_plan(g, t, i, WarPlanType::WARPLAN_ATTACKED, true);
                        }
                    } else if self.team_ai_get_war_plan(g, t, i) == Some(WarPlanType::WARPLAN_PREPARING_LIMITED) {
                        // FAssert: g.team_can_declare_war(t, i)

                        if self.team_ai_get_war_plan_state_counter(g, t, i) > ((5 * time_modifier) / 100) {
                            self.team_ai_set_war_plan(g, t, i, WarPlanType::WARPLAN_LIMITED, true);
                        }
                    } else if self.team_ai_get_war_plan(g, t, i) == Some(WarPlanType::WARPLAN_PREPARING_TOTAL) {
                        // FAssert: g.team_can_declare_war(t, i)

                        if self.team_ai_get_war_plan_state_counter(g, t, i) > ((10 * time_modifier) / 100) {
                            let mut area_valid = true;
                            let mut share_valid = false;

                            for area in g.map.areas.ids() {
                                if self.team_ai_is_primary_area(g, t, area) {
                                    if g.team_count_num_cities_by_area(i, area) > 0 {
                                        share_valid = true;

                                        let mut offensive_valid = false;

                                        if self.team_ai_calculate_area_ai_type(g, t, area, true) == Some(AreaAIType::AREAAI_OFFENSIVE) {
                                            offensive_valid = true;
                                        }

                                        if !offensive_valid {
                                            area_valid = false;
                                        }
                                    }
                                }
                            }

                            if area_valid || !share_valid {
                                self.team_ai_set_war_plan(g, t, i, WarPlanType::WARPLAN_TOTAL, true);
                            } else if self.team_ai_get_war_plan_state_counter(g, t, i) > ((20 * time_modifier) / 100) {
                                // AI_setWarPlan(NO_WARPLAN)
                                g.team_ai_set_war_plan(t, i, None, true);
                            }
                        }
                    }
                }
            }
        }

        for p in g.civ_player_ids() {
            if g.players[p].alive {
                if g.players[p].team == t {
                    self.player_ai_do_peace(g, p);
                }
            }
        }

        let num_members = g.teams[t].num_members;
        let mut high_unit_spending_percent = 0;
        let mut low_unit_spending_percent = 0;
        for p in g.player_ids() {
            if g.players[p].alive {
                if g.players[p].team == t {
                    let unit_spending_percent = (g.player_calculate_unit_cost(p) * 100) / g.player_calculate_pre_inflated_costs(p).max(1);
                    high_unit_spending_percent += (unit_spending_percent - 7).max(0) / 2;
                    low_unit_spending_percent += unit_spending_percent;
                }
            }
        }

        // PORT ADAPT: max(1) guards a team without members (C++ divides by getNumMembers()).
        high_unit_spending_percent /= num_members.max(1);
        low_unit_spending_percent /= num_members.max(1);

        let aggressive_option = g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI);

        // if at war, check for making peace
        if g.team_get_at_war_count(t, true) > 0 {
            // XXX
            let make_peace_rand = self.team_ai_make_peace_rand(g, t);
            if self.rand(make_peace_rand, "AI Make Peace") == 0 {
                for i in g.civ_team_ids() {
                    if g.team_is_alive(i) {
                        if i != t {
                            if !g.team_is_human(i) {
                                if g.team_can_contact(t, i) {
                                    // FAssert: !g.team_is_minor_civ(i)

                                    if g.teams[t].at_war[i] {
                                        if self.team_ai_is_chosen_war(g, t, i) {
                                            if self.team_ai_get_at_war_counter(g, t, i) > 20 {
                                                // removed this, it prevents conquest and domination victories
                                                // have to make sure diplomacy begs for mercy properly in AI-AI relations
                                                //if (AI_getAtWarCounter((TeamTypes)iI) > 50)
                                                //{
                                                //	makePeace((TeamTypes)iI);
                                                //	break;
                                                //}

                                                let limit = if self.team_ai_get_war_plan(g, t, i) == Some(WarPlanType::WARPLAN_TOTAL) { 40 } else { 30 };
                                                if self.team_ai_get_at_war_counter(g, t, i) > limit {
                                                    let our_value = self.team_ai_end_war_val(g, t, i);
                                                    let their_value = self.team_ai_end_war_val(g, i, t);
                                                    if (our_value > (their_value / 2)) && (their_value > (our_value / 2)) {
                                                        g.team_make_peace(t, i, true);
                                                        break;
                                                    }
                                                }

                                                if self.team_ai_get_war_plan(g, t, i) == Some(WarPlanType::WARPLAN_DOGPILE) {
                                                    if g.team_get_at_war_count(i, true) == 1 {
                                                        g.team_make_peace(t, i, true);
                                                        break;
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
        }
        // if no war plans, consider starting one!
        else if g.team_get_any_war_plan_count(t, true) == 0 {
            let mut aggressive = aggressive_option;

            let mut financial_trouble_count = 0;
            let mut dagger_count = 0;
            let mut get_better_units_count = 0;
            let mut final_war = false;
            for p in g.player_ids() {
                if g.players[p].alive {
                    if g.players[p].team == t {
                        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_DAGGER) {
                            dagger_count += 1;
                            aggressive = true;
                        }
                        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_GET_BETTER_UNITS) {
                            get_better_units_count += 1;
                        }

                        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FINAL_WAR) {
                            final_war = true;
                        }

                        if self.player_ai_is_financial_trouble(g, p) {
                            financial_trouble_count += 1;
                        }
                    }
                }
            }

            // if random in this range is 0, we go to war of this type (so lower numbers are higher probablity)
            // average of everyone on our team
            let mut max_war_rand = self.team_ai_max_war_rand(g, t);
            let mut limited_war_rand = self.team_ai_limited_war_rand(g, t);
            let mut dogpile_war_rand = self.team_ai_dogpile_war_rand(g, t);

            let num_vassals = g.team_get_vassal_count(t, None);

            max_war_rand *= num_members;
            max_war_rand /= num_members + num_vassals;

            if final_war {
                max_war_rand /= 4;
            }

            limited_war_rand *= num_members;
            limited_war_rand /= num_members + num_vassals;

            dogpile_war_rand *= num_members;
            dogpile_war_rand /= num_members + num_vassals;

            let mut war_rand_threshold = high_unit_spending_percent * (if aggressive_option { 4 } else { 2 });
            war_rand_threshold /= 3;
            war_rand_threshold += if aggressive_option { 1 } else { 0 };

            // we oppose war if half the non-dagger teammates in financial trouble
            let mut finances_oppose_war = false;
            if (financial_trouble_count - dagger_count) >= (g.teams[t].num_members / 2).max(1) {
                // this can be overridden by by the pro-war booleans
                finances_oppose_war = true;
            }

            // if agressive, we may start a war to get money
            let mut finances_pro_max_war = false;
            let mut finances_pro_limited_war = false;
            let mut finances_pro_dogpile_war = false;
            if financial_trouble_count > 0 {
                // do we like all out wars?
                if dagger_count > 0 || max_war_rand < 100 {
                    finances_pro_max_war = true;
                }

                // do we like limited wars?
                if limited_war_rand < 100 {
                    finances_pro_limited_war = true;
                }

                // do we like dogpile wars?
                if dogpile_war_rand < 100 {
                    finances_pro_dogpile_war = true;
                }
            }
            let financial_pro_war = finances_pro_max_war || finances_pro_limited_war || finances_pro_dogpile_war;

            // overall war check (quite frequently true)
            let mut make_war_checks = false;
            if get_better_units_count * 3 < num_members * 2 {
                if financial_pro_war || !finances_oppose_war {
                    // random overall war chance (at noble+ difficulties this is 100%)
                    if self.rand(100, "AI Declare War 1") < g.d.handicaps[g.handicap].ai_declare_war_prob {
                        // if non-agressive, random based on number of cities (the more cities, the less war)
                        let random_ok = if aggressive || financial_pro_war { true } else { self.rand(4, "AI Declare War 2") != 0 };
                        if random_ok {
                            make_war_checks = true;
                        }
                    }
                }
            }

            if make_war_checks {
                let mut our_power = g.team_get_power(t, true);

                if aggressive_option {
                    our_power *= 4;
                    our_power /= 3;
                }

                if (finances_pro_max_war || !finances_oppose_war) && (self.rand(max_war_rand, "AI Maximum War") <= war_rand_threshold) {
                    let mut no_war_roll = self.rand(100, "AI No War") - 0;
                    no_war_roll = range(no_war_roll + (if aggressive { 10 } else { 0 }) + (if finances_pro_max_war { 10 } else { 0 }), 0, 99);

                    let war_plan = if dagger_count > 0 { WarPlanType::WARPLAN_TOTAL } else { WarPlanType::WARPLAN_PREPARING_TOTAL };
                    self.do_war_max_war_passes(g, t, no_war_roll, our_power, war_plan);
                }
                // C++ quirk: the limited war test uses bFinancesProMaxWar, not bFinancesProLimitedWar.
                else if (finances_pro_max_war || !finances_oppose_war) && (self.rand(limited_war_rand, "AI Limited War") == 0) {
                    let mut no_war_roll = self.rand(100, "AI No War") - 10;
                    no_war_roll = range(no_war_roll + (if aggressive { 10 } else { 0 }) + (if finances_pro_limited_war { 10 } else { 0 }), 0, 99);

                    let mut best_value = 0;
                    let mut best_team = None;

                    for i in g.civ_team_ids() {
                        if g.team_is_alive(i) {
                            if i != t {
                                if g.teams[t].has_met[i] {
                                    if g.team_can_declare_war(t, i) {
                                        if no_war_roll >= self.team_ai_no_war_attitude_prob(g, t, self.team_ai_get_attitude(g, t, i, true)) {
                                            if self.team_ai_is_land_target(g, t, i)
                                                || (self.team_ai_is_any_capital_area_alone(g, t) && self.team_ai_is_any_capital_area_alone(g, i))
                                            {
                                                if g.team_get_defensive_power(i) < ((our_power * self.team_ai_limited_war_power_ratio(g, t)) / 100) {
                                                    let value = self.team_ai_start_war_val(g, t, i);

                                                    if value > best_value {
                                                        // FAssert: !self.team_ai_share_war(g, t, i)
                                                        best_value = value;
                                                        best_team = Some(i);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(best_team) = best_team {
                        let plan = if dagger_count > 0 { WarPlanType::WARPLAN_LIMITED } else { WarPlanType::WARPLAN_PREPARING_LIMITED };
                        self.team_ai_set_war_plan(g, t, best_team, plan, true);
                    }
                } else if (finances_pro_dogpile_war || !finances_oppose_war) && (self.rand(dogpile_war_rand, "AI Dogpile War") == 0) {
                    let mut no_war_roll = self.rand(100, "AI No War") - 20;
                    no_war_roll = range(no_war_roll + (if aggressive { 10 } else { 0 }) + (if finances_pro_dogpile_war { 10 } else { 0 }), 0, 99);

                    let mut best_value = 0;
                    let mut best_team = None;

                    for i in g.civ_team_ids() {
                        if g.team_is_alive(i) {
                            if i != t {
                                if g.teams[t].has_met[i] {
                                    if g.team_can_declare_war(t, i) {
                                        if no_war_roll >= self.team_ai_no_war_attitude_prob(g, t, self.team_ai_get_attitude(g, t, i, true)) {
                                            if g.team_get_at_war_count(i, true) > 0 {
                                                if self.team_ai_is_land_target(g, t, i) {
                                                    let mut dogpile_power = our_power;

                                                    for j in g.civ_team_ids() {
                                                        if g.team_is_alive(j) {
                                                            if j != i {
                                                                if g.at_war(j, i) {
                                                                    dogpile_power += g.team_get_power(j, false);
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if ((g.team_get_defensive_power(i) * 3) / 2) < dogpile_power {
                                                        let value = self.team_ai_start_war_val(g, t, i);

                                                        if value > best_value {
                                                            // FAssert: !self.team_ai_share_war(g, t, i)
                                                            best_value = value;
                                                            best_team = Some(i);
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

                    if let Some(best_team) = best_team {
                        self.team_ai_set_war_plan(g, t, best_team, WarPlanType::WARPLAN_DOGPILE, true);
                    }
                }
            }
        }
        // BETTER AI (in Conquestmode AI more willing to declare War) Sephi
        // C++ quirk: the same condition as the branch above, so this block never runs.
        else if g.team_get_any_war_plan_count(t, true) == 0 {
            let mut conquest_mode = 0;
            for p in g.player_ids() {
                if g.players[p].alive {
                    if g.players[p].team == t {
                        if g.players[p].conquest_mode {
                            conquest_mode += 1;
                        }
                    }
                }
            }

            if conquest_mode == num_members {
                let dagger_count = 0;
                let final_war = true;
                // if random in this range is 0, we go to war of this type (so lower numbers are higher probablity)
                // average of everyone on our team
                let mut max_war_rand = self.team_ai_max_war_rand(g, t);

                let num_vassals = g.team_get_vassal_count(t, None);

                max_war_rand *= num_members;
                max_war_rand /= num_members + num_vassals;

                if final_war {
                    max_war_rand /= 4;
                }

                let mut war_rand_threshold = high_unit_spending_percent * (if aggressive_option { 4 } else { 2 });
                war_rand_threshold /= 3;
                war_rand_threshold += if aggressive_option { 1 } else { 0 };

                let make_war_checks = true;

                if make_war_checks {
                    let mut our_power = g.team_get_power(t, true);

                    if aggressive_option {
                        our_power *= 4;
                        our_power /= 3;
                    }

                    if self.rand(max_war_rand, "AI Maximum War") <= war_rand_threshold {
                        let no_war_roll = self.rand(100, "AI No War") + 50;

                        let war_plan = if dagger_count > 0 { WarPlanType::WARPLAN_TOTAL } else { WarPlanType::WARPLAN_PREPARING_TOTAL };
                        self.do_war_max_war_passes(g, t, no_war_roll, our_power, war_plan);
                    }
                }
            }
        }
    }

    /// The three passes of `AI_doWar`'s "maximum war" target choice (the same code in both of
    /// its maximum war blocks): the best `AI_startWarVal` target gets `war_plan`.
    fn do_war_max_war_passes(&self, g: &mut Game, t: TeamId, no_war_roll: i32, our_power: i32, war_plan: WarPlanType) {
        let mut best_value = 0;
        let mut best_team = None;

        for pass in 0..3 {
            for i in g.civ_team_ids() {
                if g.team_is_alive(i) {
                    if i != t {
                        if g.teams[t].has_met[i] {
                            if g.team_can_declare_war(t, i) {
                                if no_war_roll >= self.team_ai_no_war_attitude_prob(g, t, self.team_ai_get_attitude(g, t, i, true)) {
                                    let defensive_power = (g.team_get_defensive_power(i) * 2) / 3;

                                    let ratio = if pass > 1 { self.team_ai_max_war_distant_power_ratio(g, t) } else { self.team_ai_max_war_nearby_power_ratio(g, t) };
                                    if defensive_power < ((our_power * ratio) / 100) {
                                        // XXX make sure they share an area....

                                        // FAssert: !g.team_is_barbarian(i), "Expected to not be declaring war on the barb civ"

                                        if (pass > 1) || (self.team_ai_is_land_target(g, t, i) || self.team_ai_is_any_capital_area_alone(g, t)) {
                                            if (pass > 0)
                                                || (self.team_ai_calculate_adjacent_land_plots(g, t, i)
                                                    >= ((g.team_get_total_land(t, true) * self.team_ai_max_war_min_adjacent_land_percent(g, t)) / 100))
                                            {
                                                let value = self.team_ai_start_war_val(g, t, i);

                                                if value > best_value {
                                                    best_value = value;
                                                    best_team = Some(i);
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

            if let Some(best_team) = best_team {
                self.team_ai_set_war_plan(g, t, best_team, war_plan, true);
                break;
            }
        }
    }

    /// C++ `bool CvTeamAI::AI_performNoWarRolls(TeamTypes eTeam)` (CvTeamAI.cpp:3796-3812)
    //returns true if war is veto'd by rolls.
    pub fn team_ai_perform_no_war_rolls(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        if self.rand(100, "AI Declare War 1") > g.d.handicaps[g.handicap].ai_declare_war_prob {
            return true;
        }

        if self.rand(100, "AI No War") <= self.team_ai_no_war_attitude_prob(g, t, self.team_ai_get_attitude(g, t, team, true)) {
            return true;
        }

        false
    }

    /// C++ `int CvTeamAI::AI_getAttitudeWeight(TeamTypes eTeam)` (CvTeamAI.cpp:3814-3837)
    pub fn team_ai_get_attitude_weight(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        match self.team_ai_get_attitude(g, t, team, true) {
            AttitudeType::ATTITUDE_FURIOUS => -100,
            AttitudeType::ATTITUDE_ANNOYED => -40,
            AttitudeType::ATTITUDE_CAUTIOUS => -5,
            AttitudeType::ATTITUDE_PLEASED => 50,
            AttitudeType::ATTITUDE_FRIENDLY => 100,
            _ => 0,
        }
    }

    /// C++ `int CvTeamAI::AI_getLowestVictoryCountdown() const` (CvTeamAI.cpp:3839-3855)
    pub fn team_ai_get_lowest_victory_countdown(&self, g: &Game, t: TeamId) -> i32 {
        let mut best_victory_countdown = MAX_INT;
        for victory in g.d.victories.keys() {
            let countdown = g.teams[t].victory_countdown[victory];
            if countdown > 0 {
                best_victory_countdown = best_victory_countdown.min(countdown);
            }
        }
        if MAX_INT == best_victory_countdown {
            best_victory_countdown = -1;
        }
        best_victory_countdown
    }

    /// C++ `int CvTeamAI::AI_getTechMonopolyValue(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:3857-4039)
    pub fn team_ai_get_tech_monopoly_value(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> i32 {
        let d = g.d;
        let mut value = 0;

        // C++ quirk: getAnyWarPlanCount(bool bIgnoreMinors) is passed eTeam (true unless team 0).
        let war_plan = g.team_get_any_war_plan_count(t, team.idx() != 0) > 0;

        for unit_class in d.unit_classes.keys() {
            let Some(loop_unit) = d.unit_classes[unit_class].default_unit_index else {
                continue;
            };

            if is_tech_required_for_unit(d, tech, loop_unit) {
                if is_world_unit_class(d, unit_class) {
                    value += 50;
                }

                let info = &d.units[loop_unit];
                if info.prereq_and_tech == Some(tech) {
                    let mut naval_value = 0;

                    let combat_ratio = (info.combat * 100) / g.best_land_unit_combat().max(1);
                    if combat_ratio > 50 {
                        value += ((if war_plan { 100 } else { 50 }) * (combat_ratio - 40)) / 50;
                    }

                    match info.default_unit_ai_type {
                        Some(UnitAIType::UNITAI_ATTACK) | Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL) => {
                            value += if war_plan { 50 } else { 20 };
                        }

                        Some(UnitAIType::UNITAI_PILLAGE)
                        | Some(UnitAIType::UNITAI_RESERVE)
                        | Some(UnitAIType::UNITAI_COUNTER)
                        | Some(UnitAIType::UNITAI_PARADROP)
                        | Some(UnitAIType::UNITAI_CITY_DEFENSE)
                        | Some(UnitAIType::UNITAI_CITY_COUNTER)
                        | Some(UnitAIType::UNITAI_CITY_SPECIAL) => {
                            value += if war_plan { 40 } else { 15 };
                        }

                        Some(UnitAIType::UNITAI_ICBM) => {
                            value += if war_plan { 80 } else { 40 };
                        }

                        Some(UnitAIType::UNITAI_ATTACK_SEA) => {
                            naval_value += 50;
                        }

                        Some(UnitAIType::UNITAI_RESERVE_SEA) | Some(UnitAIType::UNITAI_ESCORT_SEA) => {
                            naval_value += 30;
                        }

                        Some(UnitAIType::UNITAI_EXPLORE_SEA) => {
                            value += if g.circumnavigation_available() { 100 } else { 0 };
                        }

                        Some(UnitAIType::UNITAI_ASSAULT_SEA) => {
                            naval_value += 60;
                        }

                        Some(UnitAIType::UNITAI_CARRIER_SEA) | Some(UnitAIType::UNITAI_MISSILE_CARRIER_SEA) => {
                            naval_value += 40;
                        }

                        Some(UnitAIType::UNITAI_PIRATE_SEA) => {
                            naval_value += 20;
                        }

                        Some(UnitAIType::UNITAI_ATTACK_AIR) | Some(UnitAIType::UNITAI_DEFENSE_AIR) => {
                            value += if war_plan { 60 } else { 30 };
                        }

                        Some(UnitAIType::UNITAI_CARRIER_AIR) => {
                            naval_value += 40;
                        }

                        Some(UnitAIType::UNITAI_MISSILE_AIR) => {
                            value += if war_plan { 40 } else { 20 };
                        }

                        // UNKNOWN, ANIMAL, SETTLE, WORKER, EXPLORE, MISSIONARY, great people,
                        // SPY, WORKER_SEA, SETTLER_SEA, MISSIONARY_SEA, SPY_SEA and the FfH unit
                        // AIs add nothing.
                        _ => {}
                    }

                    if naval_value > 0 {
                        if self.team_ai_is_any_capital_area_alone(g, t) {
                            value += naval_value / 2;
                        }
                        if war_plan && !self.team_ai_is_land_target(g, t, team) {
                            value += naval_value / 2;
                        }
                    }
                }
            }
        }

        for building in d.buildings.keys() {
            if is_tech_required_for_building(d, tech, building) {
                let loop_building = &d.buildings[building];
                if loop_building.religion_type.is_none() {
                    value += 30;
                }
                if let Some(building_class) = loop_building.building_class_type {
                    if is_world_wonder_class(d, building_class) {
                        if !g.is_building_class_maxed_out(building_class, 0) {
                            value += 50;
                        }
                    }
                }
            }
        }

        for project in d.projects.keys() {
            if d.projects[project].tech_prereq == Some(tech) {
                if is_world_project(d, project) {
                    if !g.is_project_maxed_out(project, 0) {
                        value += 100;
                    }
                } else {
                    value += 50;
                }
            }
        }

        value
    }

    /// C++ `bool CvTeamAI::AI_isWaterAreaRelevant(CvArea* pArea)` (CvTeamAI.cpp:4041-4091)
    pub fn team_ai_is_water_area_relevant(&self, g: &Game, t: TeamId, area: AreaId) -> bool {
        let mut team_cities = 0;
        let mut other_team_cities = 0;

        if g.map.find_biggest_area(true) == Some(area) {
            return true;
        }

        //An area is deemed relevant if it has at least 2 cities of our and different teams.

        for player in g.civ_player_ids() {
            let player_team = g.players[player].team;

            if (team_cities < 2 && (player_team == t)) || (other_team_cities < 2 && (player_team != t)) {
                for &loop_city in &g.players[player].cities {
                    if g.plot_is_adjacent_to_area(g.city_plot(loop_city), area) {
                        if player_team == t {
                            team_cities += 1;
                            if team_cities >= 2 {
                                break;
                            }
                        } else {
                            other_team_cities += 1;
                            if other_team_cities >= 2 {
                                break;
                            }
                        }
                    }
                }
            }
            if team_cities >= 2 && other_team_cities >= 2 {
                return true;
            }
        }
        false
    }

    /// C++ `bool CvTeamAI::AI_acceptSurrender(TeamTypes eSurrenderTeam)` (CvTeamAI.cpp:4099-4356)
    pub fn team_ai_accept_surrender(&self, g: &Game, t: TeamId, surrender_team: TeamId) -> bool {
        // BBAI TODO:  Need better impending space victory detector ... I think this only catches launched spaceships
        // Is surrender team going to win by spaceship?
        let their_victory_counter = self.team_ai_get_lowest_victory_countdown(g, surrender_team);
        let our_victory_counter = self.team_ai_get_lowest_victory_countdown(g, t);
        if (their_victory_counter >= 0 && our_victory_counter < 0) || (their_victory_counter < our_victory_counter) {
            return false;
        }

        // Is surrender team going for cultural victory?  Capturing and holding original capital will halt those ambitions
        for i in g.civ_player_ids() {
            if g.players[i].alive {
                if g.players[i].team == surrender_team {
                    if self.player_ai_get_culture_victory_stage(g, i) > 2 {
                        return false;
                    }
                }
            }
        }

        // Check for whether team can surrender to anyone else
        for i in g.civ_player_ids() {
            if g.players[i].alive {
                let i_team = g.players[i].team;
                if i_team != t && !g.teams[i_team].vassal[t] {
                    if g.teams[surrender_team].at_war[i_team] {
                        if self.team_ai_get_at_war_counter(g, surrender_team, i_team) >= 10 {
                            let item = set_trade_item(TradeableItem::TRADE_SURRENDER, 0);

                            // PORT ADAPT: a team without a leader (C++ GET_PLAYER(NO_PLAYER)) cannot trade.
                            if let Some(leader) = g.team_get_leader_id(surrender_team) {
                                if g.player_can_trade_item(leader, i, &item, true) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut valuable_cities = 0;
        let mut cities_threatened_by_us = 0;
        let mut valuable_cities_threatened_by_us = 0;
        let mut cities_threatened_by_others = 0;

        let our_leader = g.team_get_leader_id(t);

        for i in g.civ_player_ids() {
            if g.players[i].team == surrender_team && g.players[i].alive {
                for &loop_city in &g.players[i].cities {
                    let city_area = g.city_area(loop_city);
                    let city_plot = g.city_plot(loop_city);
                    let mut valuable = false;

                    if city_is_holy_city_any(g, loop_city) {
                        valuable = true;
                    } else if city_is_headquarters_any(g, loop_city) {
                        valuable = true;
                    } else if g.city_has_active_world_wonder(loop_city) {
                        valuable = true;
                    } else if self.team_ai_is_primary_area(g, t, city_area) && (g.team_count_num_cities_by_area(surrender_team, city_area) < 3) {
                        valuable = true;
                    } else if g.city_is_capital(loop_city)
                        && (g.teams[surrender_team].num_cities > g.teams[surrender_team].num_members || g.team_count_num_cities_by_area(t, city_area) > 0)
                    {
                        valuable = true;
                    } else {
                        // Valuable terrain bonuses
                        let (cx, cy) = (g.cities[loop_city].x, g.cities[loop_city].y);
                        for j in 0..NUM_CITY_PLOTS {
                            if let Some(loop_plot) = g.plot_city(cx, cy, j) {
                                if let Some(bonus) = g.plot_get_non_obsolete_bonus_type(loop_plot, t) {
                                    // GET_PLAYER(getLeaderID()).AI_bonusVal(eBonus): CvPlayerAI default iChange = 1.
                                    if let Some(leader) = our_leader {
                                        if self.player_ai_bonus_val(g, leader, bonus, 1) > 15 {
                                            valuable = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let owner_power = self.player_ai_get_our_plot_strength(g, i, city_plot, 2, true, false);
                    let our_power = self.team_ai_get_our_plot_strength(g, t, city_plot, 2, false, false, true);
                    let other_power = self.player_ai_get_enemy_plot_strength(g, i, city_plot, 2, false, false) - our_power;

                    if other_power > owner_power {
                        cities_threatened_by_others += 1;
                    }

                    if our_power > owner_power {
                        cities_threatened_by_us += 1;
                        if valuable {
                            valuable_cities += 1;
                            valuable_cities_threatened_by_us += 1;
                            continue;
                        }
                    }

                    if valuable && g.cities[loop_city].highest_population < 5 {
                        valuable = false;
                    }

                    if valuable {
                        if self.team_ai_is_primary_area(g, t, city_area) {
                            valuable_cities += 1;
                        } else {
                            for j in g.player_ids() {
                                if g.players[j].alive {
                                    if g.players[j].team == t {
                                        // CvCity::AI_playerCloseness default iMaxDistance = 7.
                                        if self.city_ai_player_closeness(g, loop_city, j, 7) > 5 {
                                            valuable_cities += 1;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if valuable_cities_threatened_by_us > 0 {
            // Press for capture of valuable city
            return false;
        }

        if cities_threatened_by_others > (1 + cities_threatened_by_us / 2) {
            // Keep others from capturing spoils, but let it go if surrender civ is too small
            // to care about
            if 6 * (valuable_cities + g.teams[surrender_team].num_cities) > g.teams[t].num_cities {
                return true;
            }
        }

        // If we're low on the totem poll, accept so enemies don't drag anyone else into war with us
        // Top rank is 0, second is 1, etc.
        let team_rank = g.team_rank[t];
        if team_rank > (1 + g.count_civ_teams_alive() / 3) {
            return true;
        }

        let our_war_success_ratio = self.team_ai_get_war_success_capitulation_ratio(g, t);
        if our_war_success_ratio < -30 {
            // We're doing badly overall, need to be done with this war and gain an ally
            return true;
        }

        let mut war_count = 0;
        for i in g.civ_team_ids() {
            if g.team_is_alive(i) && !g.team_is_minor_civ(i) {
                if i != surrender_team && !g.teams[i].vassal[surrender_team] {
                    if g.teams[t].at_war[i] {
                        if self.team_ai_get_war_success(g, i, t) > 5 * g.d.defines.war_success_attacking {
                            war_count += 1;
                        }
                    }
                }
            }
        }

        if war_count > 0 && our_war_success_ratio < 50 {
            // Accept if we have other wars to fight
            return true;
        }

        // War weariness
        let mut weariness_threshold = if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) { 300 } else { 240 };
        weariness_threshold += 10 * valuable_cities + 20 * cities_threatened_by_us;

        for i in g.civ_player_ids() {
            if g.players[i].alive {
                if g.players[i].team == t {
                    let mut war_weariness_percent_anger =
                        (g.teams[t].war_weariness[surrender_team] * (100 + g.teams[surrender_team].enemy_war_weariness_modifier).max(0)) / 10000;
                    war_weariness_percent_anger = g.player_get_modified_war_weariness_percent_anger(i, war_weariness_percent_anger);

                    // Significant war weariness from eSurrenderTeam, 1000 = 100%
                    if war_weariness_percent_anger > 50 {
                        if g.player_get_war_weariness_percent_anger(i) > weariness_threshold {
                            return true;
                        }
                    }
                }
            }
        }

        if (valuable_cities + cities_threatened_by_us) >= (self.team_ai_max_war_rand(g, t) / 100) {
            // Continue conquest
            return false;
        }

        if g.teams[surrender_team].num_cities < (g.teams[t].num_cities / 4 - (self.team_ai_max_war_rand(g, t) / 100)) {
            // Too small to bother leaving alive
            return false;
        }

        true
    }

    /// C++ `int CvTeamAI::AI_getOurPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves, bool bIncludeVassals)` (CvTeamAI.cpp:4359-4376)
    // Find plot strength of teammates and potentially vassals
    pub fn team_ai_get_our_plot_strength(&self, g: &Game, t: TeamId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool, include_vassals: bool) -> i32 {
        let mut plot_strength = 0;

        for i in g.player_ids() {
            if g.players[i].alive {
                let i_team = g.players[i].team;
                if i_team == t || (include_vassals && g.teams[i_team].vassal[t]) {
                    plot_strength += self.player_ai_get_our_plot_strength(g, i, plot, range, defensive_bonuses, test_moves);
                }
            }
        }

        plot_strength
    }

    /// C++ `int CvTeamAI::AI_getWarSuccessCapitulationRatio() const` (CvTeamAI.cpp:4382-4403)
    /// \brief Compute how close our wars are to capitulation.
    ///
    /// At -99, this means we're losing on net enough to capitulate.  At +99, we're winning enough
    /// to perhaps have another player capitulate to us.
    pub fn team_ai_get_war_success_capitulation_ratio(&self, g: &Game, t: TeamId) -> i32 {
        let mut sum_war_success = 0;

        for i in g.civ_team_ids() {
            if i != t {
                if g.team_is_alive(i) && g.teams[t].at_war[i] {
                    sum_war_success += self.team_ai_get_war_success(g, t, i);
                    sum_war_success -= self.team_ai_get_war_success(g, i, t);
                }
            }
        }

        let divisor = 25.max(g.teams[t].num_cities.min(4) * g.d.defines.war_success_city_capturing);

        sum_war_success = range((100 * sum_war_success) / divisor, -99, 99);

        sum_war_success
    }

    /// C++ `void CvSelectionGroupAI::AI_init()` (CvSelectionGroupAI.cpp:28-34)
    pub fn group_ai_init(&self, g: &mut Game, grp: GroupId) {
        self.group_ai_reset(g, grp);

        //--------------------------------
        // Init other game data
    }

    /// C++ `void CvSelectionGroupAI::AI_reset()` (CvSelectionGroupAI.cpp:42-58)
    pub fn group_ai_reset(&self, g: &mut Game, grp: GroupId) {
        // AI_uninit() is empty.
        let group = &mut g.groups[grp];

        group.ai_mission_ai_x = INVALID_PLOT_COORD;
        group.ai_mission_ai_y = INVALID_PLOT_COORD;

        group.ai_force_separate = false;

        group.ai_mission_ai_type = None;

        group.ai_mission_ai_unit = None;

        group.ai_group_attack = false;
        group.ai_group_attack_x = -1;
        group.ai_group_attack_y = -1;
    }

    /// C++ `void CvSelectionGroupAI::AI_separate()` (CvSelectionGroupAI.cpp:61-79)
    pub fn group_ai_separate(&self, g: &mut Game, grp: GroupId) {
        let team = g.group_get_team(grp);
        for loop_unit in g.groups[grp].units.clone() {
            if !g.units.contains(loop_unit) {
                continue;
            }

            g.unit_join_group(loop_unit, None, true);
            if g.plot_get_team(g.unit_plot(loop_unit)) == Some(team) {
                if let Some(new_group) = g.units[loop_unit].group {
                    g.group_push_mission(new_group, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
                }
            }
        }
    }

    /// C++ `void CvSelectionGroupAI::AI_seperateNonAI(UnitAITypes eUnitAI)` (CvSelectionGroupAI.cpp:81-101)
    pub fn group_ai_seperate_non_ai(&self, g: &mut Game, grp: GroupId, unit_ai: UnitAIType) {
        let team = g.group_get_team(grp);
        for loop_unit in g.groups[grp].units.clone() {
            if !g.units.contains(loop_unit) {
                continue;
            }

            if g.units[loop_unit].ai_type != Some(unit_ai) {
                g.unit_join_group(loop_unit, None, true);
                if g.plot_get_team(g.unit_plot(loop_unit)) == Some(team) {
                    if let Some(new_group) = g.units[loop_unit].group {
                        g.group_push_mission(new_group, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
                    }
                }
            }
        }
    }

    /// C++ `void CvSelectionGroupAI::AI_seperateAI(UnitAITypes eUnitAI)` (CvSelectionGroupAI.cpp:103-123)
    pub fn group_ai_seperate_ai(&self, g: &mut Game, grp: GroupId, unit_ai: UnitAIType) {
        let team = g.group_get_team(grp);
        for loop_unit in g.groups[grp].units.clone() {
            if !g.units.contains(loop_unit) {
                continue;
            }

            if g.units[loop_unit].ai_type == Some(unit_ai) {
                g.unit_join_group(loop_unit, None, true);
                // C++ quirk: tests the team of the group's plot (its new head unit's), not the unit's.
                // FIX: C++ dereferences NULL when the group is now empty; use the unit's plot then.
                let plot = g.group_plot(grp).unwrap_or_else(|| g.unit_plot(loop_unit));
                if g.plot_get_team(plot) == Some(team) {
                    if let Some(new_group) = g.units[loop_unit].group {
                        g.group_push_mission(new_group, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
                    }
                }
            }
        }
    }

    /// C++ `bool CvSelectionGroupAI::AI_update()` (CvSelectionGroupAI.cpp:125-281)
    // Returns true if the group has become busy...
    pub fn group_ai_update(&self, g: &mut Game, grp: GroupId) -> bool {
        //FfH: Modified by Kael 12/28/2008
        if !self.group_ai_is_controlled(g, grp) && !g.group_is_ai_control(grp) {
            return false;
        }

        if g.groups[grp].units.is_empty() {
            return false;
        }

        if g.groups[grp].force_update {
            g.group_clear_mission_queue(grp); // XXX ???
            g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
            g.groups[grp].force_update = false;

            // if we are in the middle of attacking with a stack, cancel it
            self.group_ai_cancel_group_attack(g, grp);
        }

        let mut temp_hack = 0; // XXX

        let mut dead = false;
        let mut failed_already_fighting = false;
        loop {
            // PORT ADAPT: a group deleted meanwhile (C++ would hold a dangling pointer) ends the
            // update as a dead group.
            if !g.groups.contains(grp) {
                return true;
            }
            if !((g.groups[grp].ai_group_attack && !failed_already_fighting) || g.group_ready_to_move(grp, false)) {
                break;
            }

            temp_hack += 1;
            if temp_hack > 100 {
                // "Unit stuck in loop" (logging omitted)
                if let Some(head_unit) = g.group_get_head_unit(grp) {
                    g.unit_finish_moves(head_unit);
                }
                break;
            }

            // if we want to force the group to attack, force another attack
            if g.groups[grp].ai_group_attack {
                g.groups[grp].ai_group_attack = false;

                let (x, y) = (g.groups[grp].ai_group_attack_x, g.groups[grp].ai_group_attack_y);
                let (_, failed) = g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
                failed_already_fighting = failed;
            }
            // else pick AI action
            else {
                let Some(head_unit) = g.group_get_head_unit(grp) else {
                    break;
                };
                if g.units[head_unit].death_delay {
                    break;
                }

                g.group_reset_path(grp);

                if self.unit_ai_update(g, head_unit) {
                    // AI_update returns true when we should abort the loop and wait until next slice
                    break;
                }
            }

            if !g.groups.contains(grp) {
                return true;
            }

            if g.group_do_delayed_death(grp) {
                dead = true;
                break;
            }

            // if no longer group attacking, and force separate is true, then bail, decide what to do after group is split up
            // (UnitAI of head unit may have changed)
            if !g.groups[grp].ai_group_attack && self.group_ai_is_force_separate(g, grp) {
                self.group_ai_separate(g, grp); // pointers could become invalid...
                return true;
            }
        }

        if !dead {
            // PORT ADAPT: see above.
            if !g.groups.contains(grp) {
                return true;
            }

            if !g.group_is_human(grp) {
                let mut follow = false;

                // if we not group attacking, then check for follow action
                if !g.groups[grp].ai_group_attack {
                    let mut entity_node = g.groups[grp].units.first().copied();

                    while let Some(loop_unit) = entity_node {
                        if !g.groups.contains(grp) || !g.group_ready_to_move(grp, true) {
                            break;
                        }
                        // PORT ADAPT: a prefetched unit that left the group or died ends the loop.
                        if !g.units.contains(loop_unit) || g.units[loop_unit].group != Some(grp) {
                            break;
                        }
                        entity_node = next_group_unit(g, grp, loop_unit);

                        if g.unit_can_move(loop_unit) {
                            g.group_reset_path(grp);

                            if self.unit_ai_follow(g, loop_unit) {
                                follow = true;
                                break;
                            }
                        }
                    }
                }

                if !g.groups.contains(grp) {
                    return true;
                }

                if g.group_do_delayed_death(grp) {
                    dead = true;
                }

                if !dead {
                    if !follow && g.group_ready_to_move(grp, true) {
                        g.group_push_mission(grp, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
                    }
                }
            }
        }

        if dead {
            return true;
        }

        if !g.groups.contains(grp) {
            return true;
        }

        g.group_is_busy(grp) || g.group_is_cargo_busy(grp)
    }

    /// C++ `int CvSelectionGroupAI::AI_attackOdds(const CvPlot* pPlot, bool bPotentialEnemy) const` (CvSelectionGroupAI.cpp:285-305)
    // Returns attack odds out of 100 (the higher, the better...)
    pub fn group_ai_attack_odds(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool) -> i32 {
        let owner = g.groups[grp].owner;

        if g.plot_get_best_defender(plot, None, Some(owner), None, !potential_enemy, potential_enemy, false).is_none() {
            return 100;
        }

        let mut odds = 0;
        let attacker = self.group_ai_get_best_group_attacker(g, grp, plot, potential_enemy, &mut odds, false, false);

        if attacker.is_none() {
            return 0;
        }

        odds
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getBestGroupAttacker(const CvPlot* pPlot, bool bPotentialEnemy, int& iUnitOdds, bool bForce = false, bool bNoBlitz = false) const` (CvSelectionGroupAI.cpp:308-417)
    ///
    /// Delegates to ffh-core's port (same C++, BETTER AI odds blocks included; its
    /// `AI_attackOdds` is ffh-core's port of `CvUnitAI::AI_attackOdds`).
    pub fn group_ai_get_best_group_attacker(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, unit_odds: &mut i32, force: bool, no_blitz: bool) -> Option<UnitId> {
        let (best_unit, best_odds) = g.group_ai_get_best_group_attacker(grp, plot, potential_enemy, force, no_blitz);
        *unit_odds = best_odds;
        best_unit
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getBestGroupSacrifice(const CvPlot* pPlot, bool bPotentialEnemy, bool bForce = false, bool bNoBlitz = false) const` (CvSelectionGroupAI.cpp:419-505)
    ///
    /// Delegates to ffh-core's port (same C++, BETTER AI blocks included).
    pub fn group_ai_get_best_group_sacrifice(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, force: bool, no_blitz: bool) -> Option<UnitId> {
        g.group_ai_get_best_group_sacrifice(grp, plot, potential_enemy, force, no_blitz)
    }

    /// C++ `int CvSelectionGroupAI::AI_compareStacks(const CvPlot* pPlot, bool bPotentialEnemy, bool bCheckCanAttack = false, bool bCheckCanMove = false) const` (CvSelectionGroupAI.cpp:509-540)
    // Returns ratio of strengths of stacks times 100
    // (so 100 is even ratio, numbers over 100 mean this group is more powerful than the stack on a plot)
    pub fn group_ai_compare_stacks(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, check_can_attack: bool, check_can_move: bool) -> i32 {
        let mut domain_type = g.group_get_domain_type(grp);

        // if not aircraft, then choose based on the plot, not the head unit (mainly for transport carried units)
        if domain_type != Some(DomainType::DOMAIN_AIR) {
            if g.plot_is_water(plot) {
                domain_type = Some(DomainType::DOMAIN_SEA);
            } else {
                domain_type = Some(DomainType::DOMAIN_LAND);
            }
        }

        let mut compare_ratio = self.group_ai_sum_strength(g, grp, Some(plot), domain_type, check_can_attack, check_can_move);
        compare_ratio *= 100;

        let owner = g.groups[grp].owner;

        let defender_sum = plot_ai_sum_strength_any(g, plot, None, Some(owner), domain_type, true, !potential_enemy, potential_enemy);
        compare_ratio /= defender_sum.max(1);

        compare_ratio
    }

    /// C++ `int CvSelectionGroupAI::AI_sumStrength(const CvPlot* pAttackedPlot = NULL, DomainTypes eDomainType = NO_DOMAIN, bool bCheckCanAttack = false, bool bCheckCanMove = false) const` (CvSelectionGroupAI.cpp:542-574)
    pub fn group_ai_sum_strength(&self, g: &Game, grp: GroupId, attacked_plot: Option<PlotIdx>, domain_type: Option<DomainType>, check_can_attack: bool, check_can_move: bool) -> i32 {
        let mut str_sum = 0;

        for &loop_unit in &g.groups[grp].units {
            if !g.unit_is_dead(loop_unit) {
                let unit_domain = g.unit_get_domain_type(loop_unit);
                let can_attack = if unit_domain == DomainType::DOMAIN_AIR { g.unit_can_air_attack(loop_unit) } else { g.unit_can_attack(loop_unit) };

                if !check_can_attack || can_attack {
                    if !check_can_move || g.unit_can_move(loop_unit) {
                        if !check_can_move || attacked_plot.map_or(true, |p| g.unit_can_move_into(loop_unit, p, true, true, false)) {
                            if domain_type.is_none() || Some(unit_domain) == domain_type {
                                // C++ quirk: the unit is passed as its own attacker.
                                str_sum += g.unit_curr_effective_str(loop_unit, attacked_plot, Some(loop_unit), None);
                            }
                        }
                    }
                }
            }
        }

        str_sum
    }

    /// C++ `void CvSelectionGroupAI::AI_queueGroupAttack(int iX, int iY)` (CvSelectionGroupAI.cpp:576-582)
    pub fn group_ai_queue_group_attack(&self, g: &mut Game, grp: GroupId, x: i32, y: i32) {
        g.group_ai_queue_group_attack(grp, x, y);
    }

    /// C++ `inline void CvSelectionGroupAI::AI_cancelGroupAttack()` (CvSelectionGroupAI.cpp:584-587)
    pub fn group_ai_cancel_group_attack(&self, g: &mut Game, grp: GroupId) {
        g.groups[grp].ai_group_attack = false;
    }

    /// C++ `inline bool CvSelectionGroupAI::AI_isGroupAttack()` (CvSelectionGroupAI.cpp:589-592)
    pub fn group_ai_is_group_attack(&self, g: &Game, grp: GroupId) -> bool {
        g.groups[grp].ai_group_attack
    }

    /// C++ `bool CvSelectionGroupAI::AI_isControlled()` (CvSelectionGroupAI.cpp:594-597)
    pub fn group_ai_is_controlled(&self, g: &Game, grp: GroupId) -> bool {
        g.group_ai_is_controlled(grp)
    }

    /// C++ `bool CvSelectionGroupAI::AI_isDeclareWar(const CvPlot* pPlot = NULL)` (CvSelectionGroupAI.cpp:600-730)
    ///
    /// Delegates to ffh-core's port (same C++, FfH unit AIs included).
    pub fn group_ai_is_declare_war(&self, g: &Game, grp: GroupId, plot: Option<PlotIdx>) -> bool {
        g.group_ai_is_declare_war(grp, plot)
    }

    /// C++ `CvPlot* CvSelectionGroupAI::AI_getMissionAIPlot()` (CvSelectionGroupAI.cpp:733-736)
    pub fn group_ai_get_mission_ai_plot(&self, g: &Game, grp: GroupId) -> Option<PlotIdx> {
        g.group_ai_get_mission_ai_plot(grp)
    }

    /// C++ `bool CvSelectionGroupAI::AI_isForceSeparate()` (CvSelectionGroupAI.cpp:739-742)
    pub fn group_ai_is_force_separate(&self, g: &Game, grp: GroupId) -> bool {
        g.groups[grp].ai_force_separate
    }

    /// C++ `void CvSelectionGroupAI::AI_makeForceSeparate()` (CvSelectionGroupAI.cpp:745-748)
    pub fn group_ai_make_force_separate(&self, g: &mut Game, grp: GroupId) {
        g.groups[grp].ai_force_separate = true;
    }

    /// C++ `MissionAITypes CvSelectionGroupAI::AI_getMissionAIType()` (CvSelectionGroupAI.cpp:751-754)
    pub fn group_ai_get_mission_ai_type(&self, g: &Game, grp: GroupId) -> Option<MissionAIType> {
        g.groups[grp].ai_mission_ai_type
    }

    /// C++ `void CvSelectionGroupAI::AI_setMissionAI(MissionAITypes eNewMissionAI, CvPlot* pNewPlot, CvUnit* pNewUnit)` (CvSelectionGroupAI.cpp:757-780)
    ///
    /// Delegates to ffh-core's port. See the SIGNATURE NOTES: NULL / NO_MISSIONAI cannot be
    /// passed through this signature (`g.group_ai_set_mission_ai` takes options).
    pub fn group_ai_set_mission_ai(&self, g: &mut Game, grp: GroupId, new_mission_ai: MissionAIType, new_plot: PlotIdx, new_unit: UnitId) {
        g.group_ai_set_mission_ai(grp, Some(new_mission_ai), Some(new_plot), Some(new_unit));
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getMissionAIUnit()` (CvSelectionGroupAI.cpp:783-786)
    pub fn group_ai_get_mission_ai_unit(&self, g: &Game, grp: GroupId) -> Option<UnitId> {
        g.group_ai_get_mission_ai_unit(grp)
    }

    /// C++ `bool CvSelectionGroupAI::AI_isFull()` (CvSelectionGroupAI.cpp:788-848)
    pub fn group_ai_is_full(&self, g: &Game, grp: GroupId) -> bool {
        let units = &g.groups[grp].units;

        if !units.is_empty() {
            let unit_ai = g.group_get_head_unit_ai(grp);
            // do two passes, the first pass, we ignore units with speical cargo
            let mut special_cargo_count = 0;
            let mut cargo_count = 0;

            // first pass, count but ignore special cargo units
            for &loop_unit in units {
                if g.units[loop_unit].ai_type == unit_ai {
                    // cargoSpace()
                    if g.units[loop_unit].cargo_capacity > 0 {
                        cargo_count += 1;
                    }

                    if g.unit_special_cargo(loop_unit).is_some() {
                        special_cargo_count += 1;
                    } else if !g.unit_is_full(loop_unit) {
                        return false;
                    }
                }
            }

            // if every unit in the group has special cargo, then check those, otherwise, consider ourselves full
            if special_cargo_count >= cargo_count {
                for &loop_unit in units {
                    if g.units[loop_unit].ai_type == unit_ai {
                        if !g.unit_is_full(loop_unit) {
                            return false;
                        }
                    }
                }
            }

            return true;
        }

        false
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_ejectBestDefender(CvPlot* pDefendPlot)` (CvSelectionGroupAI.cpp:851-895)
    pub fn group_ai_eject_best_defender(&self, g: &mut Game, grp: GroupId, defend_plot: PlotIdx) -> Option<UnitId> {
        let team = g.group_get_team(grp);

        let mut best_unit = None;
        let mut best_unit_value = 0;

        for &loop_unit in &g.groups[grp].units {
            if !g.unit_no_defensive_bonus(loop_unit) {
                let mut value = g.unit_curr_effective_str(loop_unit, Some(defend_plot), None, None) * 100;

                if g.plot_is_city(defend_plot, true, Some(team)) {
                    value *= 100 + g.unit_city_defense_modifier(loop_unit);
                    value /= 100;
                }

                value *= 100;
                let divisor = 100 + g.unit_city_attack_modifier(loop_unit) + g.units[loop_unit].extra_city_attack_percent;
                // FIX: C++ divides by zero when the city attack modifiers sum to -100.
                if divisor != 0 {
                    value /= divisor;
                }

                value /= 2 + g.units[loop_unit].level;

                if value > best_unit_value {
                    best_unit_value = value;
                    best_unit = Some(loop_unit);
                }
            }
        }

        if let Some(best) = best_unit {
            if g.groups[grp].units.len() > 1 {
                g.unit_join_group(best, None, true);
            }
        }

        best_unit
    }

    /// C++ `int CvSelectionGroupAI::AI_getGroupflag() const` (CvSelectionGroupAI.cpp:952-955)
    pub fn group_ai_get_groupflag(&self, g: &Game, grp: GroupId) -> i32 {
        g.group_ai_get_groupflag(grp)
    }

    /// C++ `void CvSelectionGroupAI::AI_setGroupflag(int newflag)` (CvSelectionGroupAI.cpp:957-966)
    pub fn group_ai_set_groupflag(&self, g: &mut Game, grp: GroupId, newflag: i32) {
        g.group_ai_set_groupflag(grp, newflag);
    }

    /// C++ `bool CvSelectionGroupAI::isHeadUnit(CvUnit* pUnit)` (CvSelectionGroupAI.cpp:968-975)
    pub fn group_ai_is_head_unit(&self, g: &Game, grp: GroupId, unit: UnitId) -> bool {
        g.group_get_head_unit(grp) == Some(unit)
    }

    /// C++ `int CvSelectionGroupAI::AI_GroupPower(CvPlot* pPlot, bool bDefensiveBonuses) const` (CvSelectionGroupAI.cpp:977-1003)
    // uses the same scale as CvPlayerAI::AI_getOurPlotStrength
    pub fn group_ai_group_power(&self, g: &Game, grp: GroupId, plot: PlotIdx, defensive_bonuses: bool) -> i32 {
        let team = g.group_get_team(grp);
        let mut value = 0;

        for &loop_unit in &g.groups[grp].units {
            if (defensive_bonuses && g.unit_can_defend(loop_unit, None)) || g.unit_can_attack(loop_unit) {
                if !g.unit_is_invisible(loop_unit, team, false) {
                    if g.unit_at_plot(loop_unit, plot)
                        || g.unit_can_move_into(loop_unit, plot, false, false, false)
                        || g.unit_can_move_into(loop_unit, plot, true, false, false)
                    {
                        value += g.unit_curr_effective_str(loop_unit, if defensive_bonuses { Some(plot) } else { None }, None, None);
                    }
                }
            }
        }
        value
    }

    /// C++ `void CvGameAI::AI_init()` (CvGameAI.cpp:24-30)
    pub fn game_ai_init(&self, g: &mut Game) {
        self.game_ai_reset(g);

        //--------------------------------
        // Init other game data
    }

    /// C++ `void CvGameAI::AI_reset()` (CvGameAI.cpp:38-43)
    pub fn game_ai_reset(&self, g: &mut Game) {
        // AI_uninit() is empty; m_iPad = 0 (unused padding, not kept).
    }

    /// C++ `void CvGameAI::AI_makeAssignWorkDirty()` (CvGameAI.cpp:46-57)
    pub fn game_ai_make_assign_work_dirty(&self, g: &mut Game) {
        for p in g.player_ids() {
            if g.players[p].alive {
                self.player_ai_make_assign_work_dirty(g, p);
            }
        }
    }

    /// C++ `void CvGameAI::AI_updateAssignWork()` (CvGameAI.cpp:60-72)
    pub fn game_ai_update_assign_work(&self, g: &mut Game) {
        for p in g.player_ids() {
            let team = g.players[p].team;
            if g.team_is_human(team) && g.players[p].alive {
                self.player_ai_update_assign_work(g, p);
            }
        }
    }

    /// C++ `int CvGameAI::AI_combatValue(UnitTypes eUnit)` (CvGameAI.cpp:76-97)
    //FfH: CvGameAI::AI_combatValue has been obsoleted by the FfH function CvPlayerAI::AI_combatValue(UnitTypes eUnit)
    pub fn game_ai_combat_value(&self, g: &Game, unit: UnitType) -> i32 {
        let info = &g.d.units[unit];
        let mut value = 100;

        if info.domain_type == Some(DomainType::DOMAIN_AIR) {
            value *= info.air_combat;
        } else {
            value *= info.combat;

            value *= ((info.first_strikes * 2) + info.chance_first_strikes) * (g.d.defines.combat_damage / 5) + 100;
            value /= 100;
        }

        value /= g.best_land_unit_combat();

        value
    }

    /// C++ `int CvGameAI::AI_turnsPercent(int iTurns, int iPercent)` (CvGameAI.cpp:100-110)
    pub fn game_ai_turns_percent(&self, g: &Game, turns: i32, percent: i32) -> i32 {
        debug_assert!(percent > 0);
        let mut turns = turns;
        if turns != MAX_INT {
            turns *= percent;
            turns /= 100;
        }

        turns.max(1)
    }
}
