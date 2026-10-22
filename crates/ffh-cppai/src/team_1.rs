//! Port of CvTeamAI.cpp:80-2216 (FFH2 0.41 C++ AI).
//!
//! The CvTeamAI members (worst enemy, counters, war success, enemy peacetime values, war plans)
//! are ffh-core `Team` fields; `AI_doCounter` and `AI_updateWorstEnemy` also run in the rules'
//! `CvTeam::doTurn` (`team_do_turn`).
//!
//! PORT ADAPT notes (see each function): `AI_getAttitudeVal` averages the C++ raw attitude
//! values of the members played by this AI and a raw-scale equivalent of the controller's
//! attitude for members played by another controller; a few late-game `int` products are
//! computed in i64 (`// FIX:`).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use std::sync::Arc;

use crate::prelude::*;

/// Alive members of team `t` (C++ `for (iI = 0; iI < MAX_PLAYERS; iI++) if (isAlive && getTeam() == getID())`).
fn members(g: &Game, t: TeamId) -> Vec<PlayerId> {
    g.player_ids().filter(|&p| g.players[p].alive && g.players[p].team == t).collect()
}

/// `GC.getLeaderHeadInfo(GET_PLAYER(p).getPersonalityType())`.
fn leader_info(g: &Game, p: PlayerId) -> &'static LeaderHeadInfo {
    let d: &'static GameData = g.d;
    let leader = g.players[p].personality_type.expect("getPersonalityType() is NO_LEADER");
    &d.leader_heads[leader]
}

/// C++ `eAttitude <= threshold` with an `AttitudeTypes` threshold that may be NO_ATTITUDE (-1).
fn att_le(a: AttitudeType, threshold: Option<AttitudeType>) -> bool {
    a.i() <= opt_to_i32(threshold)
}

/// C++ `eAttitude > threshold` with an `AttitudeTypes` threshold that may be NO_ATTITUDE (-1).
fn att_gt(a: AttitudeType, threshold: Option<AttitudeType>) -> bool {
    a.i() > opt_to_i32(threshold)
}

/// C++ `iValue -= (iValue % DIPLOMACY_VALUE_REMAINDER); return isHuman() ? max(iValue, REMAINDER) : iValue;`
fn diplomacy_value(g: &Game, t: TeamId, mut value: i32) -> i32 {
    let rem = g.d.defines.diplomacy_value_remainder;
    if rem != 0 {
        value -= value % rem;
    }
    if g.team_is_human(t) {
        value.max(rem)
    } else {
        value
    }
}

/// `int` expression `a * b / c` computed in i64 (clamped back to i32; `c == 0` divides by 1).
fn mul_div(a: i32, b: i32, c: i32) -> i32 {
    let c = if c == 0 { 1 } else { c };
    let v = (a as i64) * (b as i64) / (c as i64);
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

impl Cai {
    /// True if player `p` is played by this controller (so its C++ AI state lives here).
    fn plays(&self, g: &Game, p: PlayerId) -> bool {
        let c = g.controller(p);
        std::ptr::eq(Arc::as_ptr(&c) as *const u8, self as *const Cai as *const u8)
    }

    /// C++ `void CvTeamAI::AI_init()` (CvTeamAI.cpp:80-86)
    pub fn team_ai_init(&self, g: &mut Game, t: TeamId) {
        self.team_ai_reset(g, t, false);

        //--------------------------------
        // Init other game data
    }

    /// C++ `void CvTeamAI::AI_reset(bool bConstructor)` (CvTeamAI.cpp:94-131)
    pub fn team_ai_reset(&self, g: &mut Game, t: TeamId, constructor: bool) {
        // AI_uninit(): nothing.

        g.teams[t].ai_worst_enemy = None;

        for o in g.team_ids() {
            {
                let k = &mut g.teams[t];
                k.ai_war_plan_state_counter[o] = 0;
                k.ai_at_war_counter[o] = 0;
                k.ai_at_peace_counter[o] = 0;
                k.ai_has_met_counter[o] = 0;
                k.ai_open_borders_counter[o] = 0;
                k.ai_defensive_pact_counter[o] = 0;
                k.ai_share_war_counter[o] = 0;
                k.ai_war_success[o] = 0;
                k.ai_enemy_peacetime_trade_value[o] = 0;
                k.ai_enemy_peacetime_grant_value[o] = 0;
                k.ai_war_plan[o] = None;
            }

            if !constructor {
                let k = &mut g.teams[o];
                k.ai_war_plan_state_counter[t] = 0;
                k.ai_at_war_counter[t] = 0;
                k.ai_at_peace_counter[t] = 0;
                k.ai_has_met_counter[t] = 0;
                k.ai_open_borders_counter[t] = 0;
                k.ai_defensive_pact_counter[t] = 0;
                k.ai_share_war_counter[t] = 0;
                k.ai_war_success[t] = 0;
                k.ai_enemy_peacetime_trade_value[t] = 0;
                k.ai_enemy_peacetime_grant_value[t] = 0;
                k.ai_war_plan[t] = None;
            }
        }
    }

    /// C++ `void CvTeamAI::AI_doTurnPre()` (CvTeamAI.cpp:134-152)
    pub fn team_ai_do_turn_pre(&self, g: &mut Game, t: TeamId) {
        // AI_doCounter() is ffh-core's `team_ai_do_counter`, already run by the rules
        // (`team_do_turn`): not run a second time here. The rest of the C++ body is early
        // returns (human, barbarian, minor civ) with nothing after them.
        if g.team_is_human(t) {
            return;
        }

        if g.team_is_barbarian(t) {
            return;
        }

        if g.team_is_minor_civ(t) {
            return;
        }
    }

    /// C++ `void CvTeamAI::AI_doTurnPost()` (CvTeamAI.cpp:155-177)
    pub fn team_ai_do_turn_post(&self, g: &mut Game, t: TeamId) {
        // The rules (`team_do_turn`) ran ffh-core's AI_updateWorstEnemy just before this hook;
        // running the AI's own port again is harmless (same state).
        self.team_ai_update_worst_enemy(g, t);

        self.team_ai_update_area_stragies(g, t, false);

        if g.team_is_human(t) {
            return;
        }

        if g.team_is_barbarian(t) {
            return;
        }

        if g.team_is_minor_civ(t) {
            return;
        }

        self.team_ai_do_war(g, t);
    }

    /// C++ `void CvTeamAI::AI_makeAssignWorkDirty()` (CvTeamAI.cpp:180-194)
    pub fn team_ai_make_assign_work_dirty(&self, g: &mut Game, t: TeamId) {
        // Same loop as the C++ (alive members' AI_makeAssignWorkDirty).
        g.team_ai_make_assign_work_dirty(t);
    }

    /// C++ `void CvTeamAI::AI_updateAreaStragies(bool bTargets = true)` (CvTeamAI.cpp:197-216)
    pub fn team_ai_update_area_stragies(&self, g: &mut Game, t: TeamId, targets: bool) {
        if !g.final_initialized {
            return;
        }

        for a in g.map.areas.ids() {
            if !g.map.areas.contains(a) {
                continue;
            }
            let v = self.team_ai_calculate_area_ai_type(g, t, a, false);
            g.map.areas[a].area_ai_type[t] = v;
        }

        if targets {
            self.team_ai_update_area_targets(g, t);
        }
    }

    /// C++ `void CvTeamAI::AI_updateAreaTargets()` (CvTeamAI.cpp:219-233)
    pub fn team_ai_update_area_targets(&self, g: &mut Game, t: TeamId) {
        for p in g.player_ids() {
            if g.players[p].alive {
                if g.players[p].team == t {
                    self.player_ai_update_area_targets(g, p);
                }
            }
        }
    }

    /// C++ `int CvTeamAI::AI_countFinancialTrouble() const` (CvTeamAI.cpp:236-257)
    pub fn team_ai_count_financial_trouble(&self, g: &Game, t: TeamId) -> i32 {
        let mut count = 0;

        for p in members(g, t) {
            if self.player_ai_is_financial_trouble(g, p) {
                count += 1;
            }
        }
        count
    }

    /// C++ `int CvTeamAI::AI_countMilitaryWeight(CvArea* pArea) const` (CvTeamAI.cpp:260-278)
    pub fn team_ai_count_military_weight(&self, g: &Game, t: TeamId, area: AreaId) -> i32 {
        let mut count = 0;

        for p in members(g, t) {
            count += self.player_ai_military_weight(g, p, area);
        }
        count
    }

    /// C++ `bool CvTeamAI::AI_isAnyCapitalAreaAlone() const` (CvTeamAI.cpp:281-299)
    pub fn team_ai_is_any_capital_area_alone(&self, g: &Game, t: TeamId) -> bool {
        for p in members(g, t) {
            if self.player_ai_is_capital_area_alone(g, p) {
                return true;
            }
        }
        false
    }

    /// C++ `bool CvTeamAI::AI_isPrimaryArea(CvArea* pArea) const` (CvTeamAI.cpp:302-320)
    pub fn team_ai_is_primary_area(&self, g: &Game, t: TeamId, area: AreaId) -> bool {
        for p in members(g, t) {
            if self.player_ai_is_primary_area(g, p, area) {
                return true;
            }
        }
        false
    }

    /// C++ `bool CvTeamAI::AI_hasCitiesInPrimaryArea(TeamTypes eTeam) const` (CvTeamAI.cpp:323-342)
    pub fn team_ai_has_cities_in_primary_area(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        for (a, _) in g.map.areas.iter() {
            if self.team_ai_is_primary_area(g, t, a) {
                if g.team_count_num_cities_by_area(team, a) != 0 {
                    return true;
                }
            }
        }

        false
    }

    /// C++ `AreaAITypes CvTeamAI::AI_calculateAreaAIType(CvArea* pArea, bool bPreparingTotal = false) const` (CvTeamAI.cpp:345-523)
    pub fn team_ai_calculate_area_ai_type(&self, g: &Game, t: TeamId, area: AreaId, preparing_total: bool) -> Option<AreaAIType> {
        let ar = &g.map.areas[area];

        if !ar.water {
            let attack_units = |g: &Game| {
                g.team_count_num_ai_units_by_area(t, area, UnitAIType::UNITAI_ATTACK)
                    + g.team_count_num_ai_units_by_area(t, area, UnitAIType::UNITAI_ATTACK_CITY)
                    + g.team_count_num_ai_units_by_area(t, area, UnitAIType::UNITAI_PILLAGE)
                    + g.team_count_num_ai_units_by_area(t, area, UnitAIType::UNITAI_ATTACK_AIR)
            };

            if g.team_is_barbarian(t) {
                if (ar.num_cities - ar.cities_per_player[g.barbarian_player()]) == 0 {
                    return Some(AreaAIType::AREAAI_ASSAULT);
                }
                if attack_units(g) > ((self.team_ai_count_military_weight(g, t, area) * 20) / 100) + 1 {
                    return Some(AreaAIType::AREAAI_OFFENSIVE); // XXX does this ever happen?
                }

                return Some(AreaAIType::AREAAI_MASSING);
            }

            let mut recent_attack = false;
            let mut targets = false;
            let mut chosen_targets = false;
            let mut declared_targets = false;

            let mut assault = false;
            let mut preparing_assault = false;

            let offensive_threshold = if preparing_total { 25 } else { 20 };

            let area_cities = g.team_count_num_cities_by_area(t, area);

            for o in g.civ_team_ids() {
                if g.team_is_alive(o) {
                    if g.teams[t].ai_war_plan[o].is_some() {
                        if g.teams[t].ai_war_plan[o] == Some(WarPlanType::WARPLAN_ATTACKED_RECENT) {
                            recent_attack = true;
                        }

                        if g.team_count_num_cities_by_area(o, area) > 0 || g.team_count_num_units_by_area(o, area) > 4 {
                            targets = true;

                            if g.team_ai_is_chosen_war(t, o) {
                                chosen_targets = true;

                                let declared = if g.teams[t].at_war[o] {
                                    g.teams[t].ai_at_war_counter[o] < 10
                                } else {
                                    g.team_ai_is_sneak_attack_ready(t, o)
                                };
                                if declared {
                                    declared_targets = true;
                                }
                            }
                        } else {
                            assault = true;
                            if g.team_ai_is_sneak_attack_preparing(t, o) {
                                preparing_assault = true;
                            }
                        }
                    }
                }
            }

            if declared_targets {
                return Some(AreaAIType::AREAAI_OFFENSIVE);
            }

            if targets {
                if attack_units(g) > ((self.team_ai_count_military_weight(g, t, area) * offensive_threshold) / 100) + 1 {
                    return Some(AreaAIType::AREAAI_OFFENSIVE);
                }
            }

            if targets {
                // C++ quirk: the loop reads GET_PLAYER(iI) where iI is the finished team loop's
                // counter (MAX_CIV_TEAMS == BARBARIAN_PLAYER), not iPlayer: kPlayer is always the
                // barbarian player, never on our (non-barbarian) team, so this never returns.
                let k = g.barbarian_player();
                for pl in g.civ_player_ids() {
                    if g.players[k].alive {
                        if g.players[k].team == t {
                            if self.player_ai_is_do_strategy(g, k, AI_STRATEGY_DAGGER) || self.player_ai_is_do_strategy(g, k, AI_STRATEGY_FINAL_WAR) {
                                if ar.cities_per_player[pl] > 0 {
                                    return Some(AreaAIType::AREAAI_MASSING);
                                }
                            }
                        }
                    }
                }
                if recent_attack {
                    let power = g.team_count_power_by_area(t, area);
                    let enemy_power = g.team_count_enemy_power_by_area(t, area);
                    if power > enemy_power {
                        return Some(AreaAIType::AREAAI_MASSING);
                    }
                    return Some(AreaAIType::AREAAI_DEFENSIVE);
                }
            }

            if area_cities > 0 {
                if g.team_count_enemy_danger_by_area(t, area) > area_cities {
                    return Some(AreaAIType::AREAAI_DEFENSIVE);
                }
            }

            if chosen_targets {
                return Some(AreaAIType::AREAAI_MASSING);
            }

            if targets {
                if area_cities > (g.teams[t].num_members * 3) {
                    if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI)
                        || g.is_option(GameOptionType::GAMEOPTION_ALWAYS_WAR)
                        || (g.team_count_power_by_area(t, area) > ((g.team_count_enemy_power_by_area(t, area) * 3) / 2))
                    {
                        return Some(AreaAIType::AREAAI_MASSING);
                    }
                }
                return Some(AreaAIType::AREAAI_DEFENSIVE);
            } else {
                if assault {
                    if self.team_ai_is_primary_area(g, t, area) {
                        if preparing_assault {
                            return Some(AreaAIType::AREAAI_ASSAULT_MASSING);
                        }
                    } else if g.team_count_num_cities_by_area(t, area) > 0 {
                        return Some(AreaAIType::AREAAI_ASSAULT_ASSIST);
                    }

                    return Some(AreaAIType::AREAAI_ASSAULT);
                }
            }
        }

        Some(AreaAIType::AREAAI_NEUTRAL)
    }

    /// C++ `int CvTeamAI::AI_calculateAdjacentLandPlots(TeamTypes eTeam) const` (CvTeamAI.cpp:525-549)
    pub fn team_ai_calculate_adjacent_land_plots(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let mut count = 0;

        for pl in g.map.plot_ids() {
            if !g.plot_is_water(pl) {
                if g.plot_get_team(pl) == Some(team) && g.plot_is_adjacent_team(pl, t, true) {
                    count += 1;
                }
            }
        }

        count
    }

    /// C++ `int CvTeamAI::AI_calculatePlotWarValue(TeamTypes eTeam) const` (CvTeamAI.cpp:552-578)
    pub fn team_ai_calculate_plot_war_value(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let mut value = 0;

        for pl in g.map.plot_ids() {
            if g.plot_get_team(pl) == Some(team) {
                if !g.plot_is_water(pl) && g.plot_is_adjacent_team(pl, t, true) {
                    value += 4;
                }

                if let Some(bonus) = g.plot_get_bonus_type(pl, Some(t)) {
                    value += 40 * g.d.bonuses[bonus].ai_objective;
                }
            }
        }

        value
    }

    /// C++ `int CvTeamAI::AI_calculateCapitalProximity(TeamTypes eTeam) const` (CvTeamAI.cpp:581-639)
    pub fn team_ai_calculate_capital_proximity(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let mut total_distance = 0;
        let mut count = 0;

        let mut min_distance = i32::MAX;
        let mut max_distance = 0;
        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == t {
                    if let Some(our_capital) = g.player_get_capital_city(i) {
                        for j in g.player_ids() {
                            if g.players[j].alive {
                                if g.players[j].team != t {
                                    if let Some(their_capital) = g.player_get_capital_city(j) {
                                        let (oc, tc) = (&g.cities[our_capital], &g.cities[their_capital]);
                                        let mult = if g.city_area(our_capital) != g.city_area(their_capital) { 3 } else { 2 };
                                        let distance = g.plot_distance(oc.x, oc.y, tc.x, tc.y) * mult;
                                        if g.players[j].team == team {
                                            total_distance += distance;
                                            count += 1;
                                        }
                                        min_distance = distance.min(min_distance);
                                        max_distance = distance.max(max_distance);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if count > 0 {
            // FIX: C++ only asserts iMaxDistance > 0 before dividing by it.
            let max_distance = max_distance.max(1);
            return (g.map.max_plot_distance() * (max_distance - ((total_distance / count) - min_distance))) / max_distance;
        }

        0
    }

    /// C++ `bool CvTeamAI::AI_isWarPossible() const` (CvTeamAI.cpp:642-660)
    pub fn team_ai_is_war_possible(&self, g: &Game, t: TeamId) -> bool {
        if g.team_get_at_war_count(t, false) > 0 {
            return true;
        }

        if g.is_option(GameOptionType::GAMEOPTION_ALWAYS_WAR) {
            return true;
        }

        if !g.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE) && !g.is_option(GameOptionType::GAMEOPTION_NO_CHANGING_WAR_PEACE) {
            return true;
        }

        false
    }

    /// C++ `bool CvTeamAI::AI_isLandTarget(TeamTypes eTeam) const` (CvTeamAI.cpp:663-676)
    pub fn team_ai_is_land_target(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        if !self.team_ai_has_cities_in_primary_area(g, t, team) {
            return false;
        }

        if self.team_ai_calculate_adjacent_land_plots(g, t, team) < 8 {
            return false;
        }

        true
    }

    /// C++ `bool CvTeamAI::AI_isAllyLandTarget(TeamTypes eTeam) const` (CvTeamAI.cpp:679-697)
    /// (this determines if eTeam or any of its allies are land targets of us)
    pub fn team_ai_is_ally_land_target(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        for o in g.civ_team_ids() {
            if o != t {
                if o == team || g.teams[o].vassal[team] || g.teams[team].vassal[o] || g.teams[o].defensive_pact[team] {
                    if self.team_ai_is_land_target(g, t, o) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// C++ `bool CvTeamAI::AI_shareWar(TeamTypes eTeam) const` (CvTeamAI.cpp:700-719)
    pub fn team_ai_share_war(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        // Same body as the C++ (ffh-core keeps it for the rules' counters).
        g.team_ai_share_war(t, team)
    }

    /// C++ `AttitudeTypes CvTeamAI::AI_getAttitude(TeamTypes eTeam, bool bForced = true) const` (CvTeamAI.cpp:722-761)
    pub fn team_ai_get_attitude(&self, g: &Game, t: TeamId, team: TeamId, forced: bool) -> AttitudeType {
        // Same averaging as the C++; each member's attitude comes from its controller (for the
        // players of this AI: `player_ai_get_attitude`, after the rules' bForced overrides).
        g.team_ai_get_attitude(t, team, forced)
    }

    /// C++ `int CvTeamAI::AI_getAttitudeVal(TeamTypes eTeam, bool bForced = true) const` (CvTeamAI.cpp:764-800)
    pub fn team_ai_get_attitude_val(&self, g: &Game, t: TeamId, team: TeamId, forced: bool) -> i32 {
        // PORT ADAPT: ffh-core's `team_ai_get_attitude_val` averages AttitudeType indices (the
        // controllers only give an AttitudeType), but the C++ callers need the raw value scale
        // (`AI_getAttitudeFromValue(AI_getAttitudeVal(..) + modifier)` in AI_surrenderTrade,
        // comparisons in AI_updateWorstEnemy and CvPlayerAI). Members played by this AI use the
        // C++ `CvPlayerAI::AI_getAttitudeVal`; members played by another controller use a raw
        // value equivalent to their controller's attitude (FURIOUS -10, ANNOYED -5, CAUTIOUS 0,
        // PLEASED 5, FRIENDLY 10: `AI_getAttitudeFromValue` maps each back to the attitude).
        let mut attitude_val = 0;
        let mut count = 0;
        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == t {
                    let ours = self.plays(g, i);
                    for j in g.player_ids() {
                        if g.players[j].alive {
                            if g.players[j].team == team {
                                attitude_val += if ours {
                                    self.player_ai_get_attitude_val(g, i, j, forced)
                                } else {
                                    match g.player_ai_get_attitude(i, j, forced) {
                                        AttitudeType::ATTITUDE_FURIOUS => -10,
                                        AttitudeType::ATTITUDE_ANNOYED => -5,
                                        AttitudeType::ATTITUDE_PLEASED => 5,
                                        AttitudeType::ATTITUDE_FRIENDLY => 10,
                                        _ => 0,
                                    }
                                };
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        if count > 0 {
            return attitude_val / count;
        }

        0
    }

    /// C++ `int CvTeamAI::AI_getMemoryCount(TeamTypes eTeam, MemoryTypes eMemory) const` (CvTeamAI.cpp:803-839)
    pub fn team_ai_get_memory_count(&self, g: &Game, t: TeamId, team: TeamId, memory: MemoryType) -> i32 {
        let mut memory_count = 0;
        let mut count = 0;
        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == t {
                    for j in g.player_ids() {
                        if g.players[j].alive {
                            if g.players[j].team == team {
                                // CvPlayerAI::AI_getMemoryCount: the rules' memory counts.
                                memory_count += g.players[i].ai_memory_count[j][memory];
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        if count > 0 {
            return memory_count / count;
        }

        0
    }

    /// C++ `int CvTeamAI::AI_chooseElection(const VoteSelectionData& kVoteSelectionData) const` (CvTeamAI.cpp:842-895)
    pub fn team_ai_choose_election(&self, g: &Game, t: TeamId, vote_selection_data: &VoteSelectionData) -> i32 {
        let vote_source = vote_selection_data.vote_source;

        let mut best_vote = -1;
        let mut best_value = 0;

        for (i, option) in vote_selection_data.vote_options.iter().enumerate() {
            let vote = option.vote;

            let mut valid = true;

            if !g.is_team_vote(vote) {
                for j in g.player_ids() {
                    if g.players[j].alive {
                        if g.players[j].team == t {
                            // A member of this (AI) team: its own AI_diploVote.
                            let player_vote = self.player_ai_diplo_vote(g, j, option, vote_source, true);

                            // FIX: C++ reads getVoteOutcome((VoteTypes)iI) with the option index
                            // iI instead of the option's vote type (wrong vote, or out of bounds).
                            let outcome = g.vote_outcome.get(vote).copied().unwrap_or(PlayerVote::NO_PLAYER_VOTE);
                            if player_vote != PlayerVote::PLAYER_VOTE_YES || player_vote == outcome {
                                valid = false;
                                break;
                            }
                        }
                    }
                }
            }

            if valid {
                let value = 1 + self.rand(10000, "AI Choose Vote");

                if value > best_value {
                    best_value = value;
                    best_vote = i as i32;
                }
            }
        }

        best_vote
    }

    /// C++ `int CvTeamAI::AI_startWarVal(TeamTypes eTeam) const` (CvTeamAI.cpp:898-988)
    pub fn team_ai_start_war_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let aggressive = g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI);

        let mut value = self.team_ai_calculate_plot_war_value(g, t, team);

        value += (3 * self.team_ai_calculate_capital_proximity(g, t, team)) / if value > 0 { 2 } else { 3 };

        let closeness_value = self.team_ai_team_closeness(g, t, team, -1);
        if closeness_value == 0 {
            value /= if aggressive { 4 } else { 2 };
        }
        value += closeness_value;

        let our_victory_countdown = self.team_ai_get_lowest_victory_countdown(g, t);
        let their_victory_countdown = self.team_ai_get_lowest_victory_countdown(g, team);

        if their_victory_countdown != -1 && (our_victory_countdown == -1 || their_victory_countdown < our_victory_countdown) {
            value += 1;
            value *= 8;
        }

        //Domination...
        let our_land_percent = g.team_get_total_land(t, true) * 100 / g.map.land_plots.max(1);
        let mut percent_of_domination = 0;
        for v in g.d.victories.keys() {
            if g.d.victories[v].land_percent > 0 {
                percent_of_domination = 100 * our_land_percent / g.adjusted_land_percent(v).max(1);
            }
        }

        if percent_of_domination > 75 {
            value *= if aggressive { 6 } else { 4 };
        }

        match self.team_ai_get_attitude(g, t, team, true) {
            AttitudeType::ATTITUDE_FURIOUS => value *= 16,
            AttitudeType::ATTITUDE_ANNOYED => value *= 8,
            AttitudeType::ATTITUDE_CAUTIOUS => value *= 4,
            AttitudeType::ATTITUDE_PLEASED => value *= 2,
            AttitudeType::ATTITUDE_FRIENDLY => value *= 1,
            _ => {}
        }

        let mut max_culture_victory_adjustment = 1;
        for p in members(g, t) {
            if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3) {
                max_culture_victory_adjustment = max_culture_victory_adjustment.max(8);
            } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                max_culture_victory_adjustment = max_culture_victory_adjustment.max(3);
            }
        }
        value /= max_culture_victory_adjustment;

        value
    }

    /// C++ `int CvTeamAI::AI_endWarVal(TeamTypes eTeam) const` (CvTeamAI.cpp:992-1171)
    /// (XXX this should consider area power...)
    pub fn team_ai_end_war_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let mut value = 100;

        value += g.teams[t].num_cities * 3;
        value += g.teams[team].num_cities * 3;

        value += g.team_get_total_population(t, true);
        value += g.team_get_total_population(team, true);

        value += g.teams[team].ai_war_success[t] * 20;

        let our_power = g.team_get_power(t, true).max(1);
        let their_power = g.team_get_defensive_power(team).max(1);

        // FIX: the power products below are computed in i64 (C++ int overflows with late-game
        // power values).
        //FfH: Modified by Kael 04/23/2009
        //	iValue /= std::max(1, iOurPower + iTheirPower + 10);
        value = mul_div(value, their_power + 10, (our_power + 10).max(1));
        if (our_power as i64 * 100) > (their_power as i64 * 120) {
            value /= 5;
        }
        //FfH: End Modify

        let war_plan = g.teams[t].ai_war_plan[team];

        // if we not human, do we want to continue war for strategic reasons?
        // only check if our power is at least 120% of theirs
        if !g.team_is_human(t) && our_power > ((120 * their_power) / 100) {
            let mut dagger = false;

            let mut any_financial_trouble = false;
            for p in members(g, t) {
                if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_DAGGER) {
                    dagger = true;
                }

                if self.player_ai_is_financial_trouble(g, p) {
                    any_financial_trouble = true;
                }
            }
            // if dagger, value peace at 90% * power ratio
            if dagger {
                value = mul_div(value, 9 * their_power, 10 * our_power);
            }

            // for now, we will always do the land mass check for domination
            // if we have more than half the land, then value peace at 90% * land ratio
            let land_ratio = (g.team_get_total_land(t, true) * 100) / g.team_get_total_land(team, true).max(1);
            if land_ratio > 120 {
                value = mul_div(value, 9 * 100, 10 * land_ratio);
            }

            // if in financial trouble, warmongers will continue the fight to make more money
            if any_financial_trouble {
                let peace_ratio = |v: i32| mul_div(v, 7 * (5 * their_power), 10 * (our_power + (4 * their_power)));
                match war_plan {
                    Some(WarPlanType::WARPLAN_TOTAL) => {
                        // if we total warmonger, value peace at 70% * power ratio factor
                        if dagger || self.team_ai_max_war_rand(g, t) < 100 {
                            value = peace_ratio(value);
                        }
                    }
                    Some(WarPlanType::WARPLAN_LIMITED) => {
                        // if we limited warmonger, value peace at 70% * power ratio factor
                        if self.team_ai_limited_war_rand(g, t) < 100 {
                            value = peace_ratio(value);
                        }
                    }
                    Some(WarPlanType::WARPLAN_DOGPILE) => {
                        // if we dogpile warmonger, value peace at 70% * power ratio factor
                        if self.team_ai_dogpile_war_rand(g, t) < 100 {
                            value = peace_ratio(value);
                        }
                    }
                    _ => {}
                }
            }
        }

        // XXX count units in enemy territory...

        if (!g.team_is_human(t) && war_plan == Some(WarPlanType::WARPLAN_TOTAL))
            || (!g.team_is_human(team) && g.teams[team].ai_war_plan[t] == Some(WarPlanType::WARPLAN_TOTAL))
        {
            value *= 2;
        } else if (!g.team_is_human(t) && war_plan == Some(WarPlanType::WARPLAN_DOGPILE) && g.team_get_at_war_count(team, true) > 1)
            || (!g.team_is_human(team) && g.teams[team].ai_war_plan[t] == Some(WarPlanType::WARPLAN_DOGPILE) && g.team_get_at_war_count(t, true) > 1)
        {
            value *= 3;
            value /= 2;
        }

        if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
            let mut our_endangered_cities = 0;
            let mut their_endangered_cities = 0;

            for p in g.civ_player_ids() {
                if g.players[p].team == team {
                    for c in g.players[p].cities.iter().copied() {
                        if self.city_ai_is_danger(g, c) {
                            their_endangered_cities += 1;
                        }
                    }
                }

                if g.players[p].team == t {
                    for c in g.players[p].cities.iter().copied() {
                        if self.city_ai_is_danger(g, c) {
                            our_endangered_cities += 1;
                        }
                    }
                }
            }

            if their_endangered_cities > our_endangered_cities {
                value /= 3;
            }
        }

        diplomacy_value(g, t, value)
    }

    /// C++ `int CvTeamAI::AI_techTradeVal(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:1174-1224)
    pub fn team_ai_tech_trade_val(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> i32 {
        let cost = (g.team_get_research_cost(t, tech) - g.team_get_research_progress(t, Some(tech))).max(0);

        let mut value = (cost * 3) / 2;

        let mut known_count = 0;
        let mut possible_known_count = 0;

        for o in g.civ_team_ids() {
            if g.team_is_alive(o) {
                if o != t {
                    if g.teams[t].has_met[o] {
                        if g.team_is_has_tech(o, Some(tech)) {
                            known_count += 1;
                        }

                        possible_known_count += 1;
                    }
                }
            }
        }

        // FIX: C++ divides by iPossibleKnownCount unguarded (0 when we have met nobody alive).
        if possible_known_count > 0 {
            value += ((cost / 2) * (possible_known_count - known_count)) / possible_known_count;
        }

        value = mul_div(value, (g.d.techs[tech].ai_trade_modifier + 100).max(0), 100);

        diplomacy_value(g, t, value)
    }

    /// C++ `DenialTypes CvTeamAI::AI_techTrade(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:1227-1395)
    pub fn team_ai_tech_trade(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> Option<DenialType> {
        let d = g.d;

        if g.is_option(GameOptionType::GAMEOPTION_NO_TECH_BROKERING) {
            if !g.team_is_has_tech(team, Some(tech)) {
                if !g.team_is_human(team) {
                    if 2 * g.team_get_research_progress(team, Some(tech)) > g.team_get_research_cost(team, tech) {
                        return Some(DenialType::DENIAL_NO_GAIN);
                    }
                }
            }
        }

        if g.team_is_human(t) {
            return None;
        }

        if g.teams[t].vassal[team] {
            return None;
        }

        if g.teams[t].at_war[team] {
            return None;
        }

        if g.teams[t].ai_worst_enemy == Some(team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in members(g, t) {
            if att_le(attitude, leader_info(g, p).tech_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }
        if attitude < AttitudeType::ATTITUDE_FRIENDLY {
            let half = g.count_civ_teams_ever_alive() / 2;
            if g.team_rank[t] < half || g.team_rank[team] < half {
                let mut no_tech_trade_threshold = self.team_ai_no_tech_trade_threshold(g, t);

                no_tech_trade_threshold *= (d.handicaps[g.team_get_handicap_type(team)].no_tech_trade_modifier + 100).max(0);
                no_tech_trade_threshold /= 100;

                if self.team_ai_get_memory_count(g, t, team, MemoryType::MEMORY_RECEIVED_TECH_FROM_ANY) > no_tech_trade_threshold {
                    return Some(DenialType::DENIAL_TECH_WHORE);
                }
            }

            let mut known_count = 0;
            let mut possible_known_count = 0;
            for o in g.civ_team_ids() {
                if g.team_is_alive(o) {
                    if o != t && o != team {
                        if g.teams[t].has_met[o] {
                            if g.team_is_has_tech(o, Some(tech)) {
                                known_count += 1;
                            }
                            possible_known_count += 1;
                        }
                    }
                }
            }
            let mut tech_trade_known_percent = self.team_ai_tech_trade_known_percent(g, t);
            tech_trade_known_percent *= (d.handicaps[g.team_get_handicap_type(team)].tech_trade_known_modifier + 100).max(0);
            tech_trade_known_percent /= 100;
            tech_trade_known_percent = mul_div(tech_trade_known_percent, self.team_ai_get_tech_monopoly_value(g, t, tech, team), 100);
            let monopoly = if possible_known_count > 0 {
                ((known_count * 100) / possible_known_count) < tech_trade_known_percent
            } else {
                tech_trade_known_percent > 0
            };
            if monopoly {
                return Some(DenialType::DENIAL_TECH_MONOPOLY);
            }
        }

        for unit in d.units.keys() {
            if is_tech_required_for_unit(d, tech, unit) {
                if let Some(class) = d.units[unit].unit_class_type {
                    if is_world_unit_class(d, class) {
                        if g.team_get_unit_class_making(t, class) > 0 {
                            return Some(DenialType::DENIAL_MYSTERY);
                        }
                    }
                }
            }
        }

        for building in d.buildings.keys() {
            if is_tech_required_for_building(d, tech, building) {
                if let Some(class) = d.buildings[building].building_class_type {
                    if is_world_wonder_class(d, class) {
                        if g.team_get_building_class_making(t, class) > 0 {
                            return Some(DenialType::DENIAL_MYSTERY);
                        }
                    }
                }
            }
        }

        for project in d.projects.keys() {
            if d.projects[project].tech_prereq == Some(tech) {
                if is_world_project(d, project) {
                    if g.teams[t].project_making[project] > 0 {
                        return Some(DenialType::DENIAL_MYSTERY);
                    }
                }

                for v in d.victories.keys() {
                    if g.is_victory_valid(v) {
                        if d.projects[project].victory_threshold[v] != 0 {
                            return Some(DenialType::DENIAL_VICTORY);
                        }
                    }
                }
            }
        }

        None
    }

    /// C++ `int CvTeamAI::AI_mapTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1398-1442)
    pub fn team_ai_map_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        let mut value = 0;

        for pl in g.map.plot_ids() {
            let plot = &g.map.plots[pl];
            if !plot.revealed[t] && plot.revealed[team] {
                if g.plot_is_water(pl) {
                    value += 1;
                } else {
                    value += 5;
                }
            }
        }

        value /= 10;

        if g.teams[team].vassal[t] {
            value /= 2;
        }

        diplomacy_value(g, t, value)
    }

    /// C++ `DenialTypes CvTeamAI::AI_mapTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:1445-1489)
    pub fn team_ai_map_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        if g.team_is_human(t) {
            return None;
        }

        if g.teams[t].vassal[team] {
            return None;
        }

        if g.teams[t].at_war[team] {
            return None;
        }

        if g.teams[t].ai_worst_enemy == Some(team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in members(g, t) {
            if att_le(attitude, leader_info(g, p).map_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }
        None
    }

    /// C++ `int CvTeamAI::AI_vassalTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1492-1497)
    pub fn team_ai_vassal_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        self.team_ai_surrender_trade_val(g, t, team)
    }

    /// C++ `DenialTypes CvTeamAI::AI_vassalTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:1500-1553)
    pub fn team_ai_vassal_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        let master = team;

        for o in g.team_ids() {
            if g.team_is_alive(o) && o != t && o != master {
                if !g.teams[o].at_war[master] && g.teams[o].at_war[t] {
                    if g.teams[master].force_peace[o] || !g.team_can_change_war_peace(master, o, false) {
                        if !g.team_is_a_vassal(o) {
                            return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_YOU);
                        }
                    }

                    if !g.team_is_human(master) {
                        // The master's denial, predicted with this AI's port.
                        let war_denial = self.team_ai_declare_war_trade(g, master, o, t, true);
                        if war_denial.is_some() {
                            return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_YOU);
                        }
                    }
                } else if g.teams[o].at_war[master] && !g.teams[o].at_war[t] {
                    if !g.team_can_change_war_peace(master, o, false) {
                        if !g.team_is_a_vassal(o) {
                            return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_YOU);
                        }
                    }

                    if !g.team_is_human(master) {
                        let peace_denial = self.team_ai_make_peace_trade(g, master, o, t);
                        if peace_denial.is_some() {
                            return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_YOU);
                        }
                    }
                }
            }
        }

        self.team_ai_surrender_trade(g, t, team, 100)
    }

    /// C++ `int CvTeamAI::AI_surrenderTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1556-1561)
    pub fn team_ai_surrender_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        0
    }

    /// C++ `DenialTypes CvTeamAI::AI_surrenderTrade(TeamTypes eTeam, int iPowerMultiplier = 100) const` (CvTeamAI.cpp:1564-1827)
    pub fn team_ai_surrender_trade(&self, g: &Game, t: TeamId, team: TeamId, power_multiplier: i32) -> Option<DenialType> {
        let master = team;

        for o in g.team_ids() {
            if g.team_is_alive(o) && o != t && o != master {
                if g.teams[o].at_war[master] && !g.teams[o].at_war[t] {
                    if g.teams[t].force_peace[o] || !g.team_can_change_war_peace(t, o, false) {
                        if !g.team_is_a_vassal(o) {
                            return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_US);
                        }
                    }
                } else if !g.teams[o].at_war[master] && g.teams[o].at_war[t] {
                    if !g.team_can_change_war_peace(t, o, false) {
                        if !g.team_is_a_vassal(o) {
                            return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_US);
                        }
                    }
                }
            }
        }

        if g.team_is_human(t) {
            return None;
        }

        let mut attitude_modifier = 0;

        if !g.team_is_parent(team, t) {
            let mut personality_modifier = 0;
            let mut num_members = 0;
            for p in members(g, t) {
                personality_modifier += leader_info(g, p).vassal_power_modifier;
                num_members += 1;
            }

            let total_power = g.count_total_civ_power();
            let mut average_power = total_power / g.count_civ_teams_alive().max(1);
            let mut master_power = g.team_get_power(team, false);
            let our_power = g.team_get_power(t, true);
            let vassal_power = mul_div(our_power, power_multiplier + personality_modifier / num_members.max(1), 100);

            // FIX: i64 intermediates here and below (C++ int overflows with late-game power).
            if g.teams[t].at_war[team] {
                let their_success = g.teams[team].ai_war_success[t].max(10);
                let mut our_success = g.teams[t].ai_war_success[team].max(10);
                let mut others_best_success = 0;
                for o in g.civ_team_ids() {
                    if o != team && o != t {
                        if g.team_is_alive(o) && g.teams[o].at_war[t] {
                            let success = g.teams[o].ai_war_success[t];
                            if success > others_best_success {
                                others_best_success = success;
                            }
                        }
                    }
                }

                // Discourage capitulation to a team that has not done the most damage
                if their_success < others_best_success {
                    our_success += others_best_success - their_success;
                }

                master_power = mul_div(2 * master_power, their_success, their_success + our_success);

                if g.teams[t].ai_worst_enemy == Some(team) {
                    master_power *= 3;
                    master_power /= 4;
                }
            } else {
                if !self.team_ai_is_land_target(g, team, t) {
                    master_power /= 2;
                }
            }

            for o in g.civ_team_ids() {
                if o != t {
                    if g.team_is_alive(o) {
                        if self.team_ai_is_land_target(g, o, t) {
                            if o != team {
                                let loop_power = g.team_get_power(o, true);
                                if loop_power > our_power {
                                    if g.teams[o].at_war[team] && !g.teams[o].at_war[t] {
                                        return Some(DenialType::DENIAL_POWER_YOUR_ENEMIES);
                                    }

                                    average_power = mul_div(2 * average_power, loop_power, (loop_power + our_power).max(1));

                                    attitude_modifier += (3 * loop_power) / our_power.max(1) - 2;
                                }

                                if !g.teams[o].at_war[team] && g.teams[o].at_war[t] {
                                    average_power = mul_div(average_power, our_power + g.team_get_power(team, false), our_power.max(1));
                                }
                            }
                        }

                        if !g.teams[t].at_war[team] {
                            if g.teams[o].at_war[team] && !g.teams[o].at_war[t] {
                                let denial = self.team_ai_declare_war_trade(g, t, o, team, false);
                                if denial.is_some() {
                                    return denial;
                                }
                            }
                        }
                    }
                }
            }

            if !g.teams[t].vassal[team] && g.team_can_vassal_revolt(t, team) {
                return Some(DenialType::DENIAL_POWER_US);
            }

            if vassal_power > average_power || 3 * (vassal_power as i64) > 2 * (master_power as i64) {
                return Some(DenialType::DENIAL_POWER_US);
            }

            let is_a_vassal = g.team_is_a_vassal(t);
            for v in g.d.victories.keys() {
                let pop_percent = g.adjusted_population_percent(v);
                let land_percent = g.adjusted_land_percent(v);

                let mut population_threat = true;
                if pop_percent > 0 {
                    population_threat = false;

                    let threshold = g.total_population as i64 * pop_percent as i64;
                    if 400 * g.team_get_total_population(t, !is_a_vassal) as i64 > 3 * threshold {
                        return Some(DenialType::DENIAL_VICTORY);
                    }

                    if !g.teams[t].at_war[team] {
                        if 400 * (g.team_get_total_population(t, is_a_vassal) + g.team_get_total_population(team, true)) as i64 > 3 * threshold {
                            population_threat = true;
                        }
                    }
                }

                let mut land_threat = true;
                if land_percent > 0 {
                    land_threat = false;

                    let threshold = g.map.land_plots as i64 * land_percent as i64;
                    if 400 * g.team_get_total_land(t, !is_a_vassal) as i64 > 3 * threshold {
                        return Some(DenialType::DENIAL_VICTORY);
                    }

                    if !g.teams[t].at_war[team] {
                        if 400 * (g.team_get_total_land(t, is_a_vassal) + g.team_get_total_land(team, true)) as i64 > 3 * threshold {
                            land_threat = true;
                        }
                    }
                }

                if pop_percent > 0 || land_percent > 0 {
                    if land_threat && population_threat {
                        return Some(DenialType::DENIAL_POWER_YOU);
                    }
                }
            }
        }

        if !g.teams[t].at_war[team] {
            if !g.team_is_parent(team, t) {
                if g.teams[t].ai_worst_enemy == Some(team) {
                    return Some(DenialType::DENIAL_WORST_ENEMY);
                }

                if !self.team_ai_has_cities_in_primary_area(g, t, team) && self.team_ai_calculate_adjacent_land_plots(g, t, team) == 0 {
                    return Some(DenialType::DENIAL_TOO_FAR);
                }
            }

            let attitude = self.team_ai_get_attitude(g, t, team, false);

            let attitude_val = self.team_ai_get_attitude_val(g, t, team, false);
            let modified_attitude = self.player_ai_get_attitude_from_value(g, attitude_val + attitude_modifier);
            for p in members(g, t) {
                if attitude <= AttitudeType::ATTITUDE_FURIOUS {
                    return Some(DenialType::DENIAL_ATTITUDE);
                }

                if att_le(modified_attitude, leader_info(g, p).vassal_refuse_attitude_threshold) {
                    return Some(DenialType::DENIAL_ATTITUDE);
                }
            }
        } else {
            if g.teams[t].ai_war_success[team] + 4 * g.d.defines.war_success_city_capturing > g.teams[team].ai_war_success[t] {
                return Some(DenialType::DENIAL_JOKING);
            }
        }

        None
    }

    /// C++ `int CvTeamAI::AI_makePeaceTradeVal(TeamTypes ePeaceTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1830-1887)
    pub fn team_ai_make_peace_trade_val(&self, g: &Game, t: TeamId, peace_team: TeamId, team: TeamId) -> i32 {
        let mut value = 50 + g.game_turn();
        value += (g.teams[team].num_cities + g.teams[peace_team].num_cities) * 8;

        let mut modifier = 0;

        let att = (self.team_ai_get_attitude(g, team, peace_team, true).i() + self.team_ai_get_attitude(g, peace_team, team, true).i()) / 2;
        match AttitudeType::from_usize(att.max(0) as usize) {
            AttitudeType::ATTITUDE_FURIOUS => modifier += 400,
            AttitudeType::ATTITUDE_ANNOYED => modifier += 200,
            AttitudeType::ATTITUDE_CAUTIOUS => modifier += 100,
            AttitudeType::ATTITUDE_PLEASED => modifier += 50,
            AttitudeType::ATTITUDE_FRIENDLY => {}
            _ => {}
        }

        value *= (modifier + 100).max(0);
        value /= 100;

        value *= 40;
        value /= g.teams[team].ai_at_war_counter[peace_team] + 10;

        diplomacy_value(g, t, value)
    }

    /// C++ `DenialTypes CvTeamAI::AI_makePeaceTrade(TeamTypes ePeaceTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1890-1929)
    pub fn team_ai_make_peace_trade(&self, g: &Game, t: TeamId, peace_team: TeamId, team: TeamId) -> Option<DenialType> {
        if g.team_is_human(peace_team) {
            return Some(DenialType::DENIAL_CONTACT_THEM);
        }

        if g.team_is_a_vassal(peace_team) {
            return Some(DenialType::DENIAL_VASSAL);
        }

        if g.team_is_human(t) {
            return None;
        }

        if !g.team_can_change_war_peace(t, peace_team, false) {
            return Some(DenialType::DENIAL_VASSAL);
        }

        // The peace team's end-war value is predicted with this AI's port.
        if self.team_ai_end_war_val(g, t, peace_team) > (self.team_ai_end_war_val(g, peace_team, t) * 2) {
            return Some(DenialType::DENIAL_CONTACT_THEM);
        }

        let land_ratio = (g.team_get_total_land(t, true) * 100) / g.team_get_total_land(team, true).max(20);
        if land_ratio > 250 {
            return Some(DenialType::DENIAL_VICTORY);
        }

        None
    }

    /// C++ `int CvTeamAI::AI_declareWarTradeVal(TeamTypes eWarTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1932-2064)
    pub fn team_ai_declare_war_trade_val(&self, g: &Game, t: TeamId, war_team: TeamId, team: TeamId) -> i32 {
        let mut value = 0;
        value += g.teams[war_team].num_cities * 10;
        value += g.team_get_total_population(war_team, true) * 2;

        let mut modifier = 0;

        match self.team_ai_get_attitude(g, team, war_team, true) {
            AttitudeType::ATTITUDE_FURIOUS => {}
            AttitudeType::ATTITUDE_ANNOYED => modifier += 25,
            AttitudeType::ATTITUDE_CAUTIOUS => modifier += 50,
            AttitudeType::ATTITUDE_PLEASED => modifier += 150,
            AttitudeType::ATTITUDE_FRIENDLY => modifier += 400,
            _ => {}
        }

        value *= (modifier + 100).max(0);
        value /= 100;

        let their_power = g.team_get_power(team, true);
        let war_team_power = g.team_get_power(war_team, true);

        value = mul_div(value, 50 + mul_div(100, war_team_power, their_power + war_team_power + 1), 100);

        if !self.team_ai_is_ally_land_target(g, team, war_team) {
            value *= 2;
        }

        if !g.teams[t].at_war[war_team] {
            value *= 3;
        } else {
            value *= 150;
            value /= 100 + ((50 * (mul_div(100, g.teams[t].ai_war_success[war_team], 8 + g.team_get_total_population(t, false))).min(100)) / 100);
        }

        value += g.teams[team].num_cities * 20;
        value += g.team_get_total_population(team, true) * 15;

        if g.teams[t].at_war[war_team] {
            match self.team_ai_get_attitude(g, team, t, true) {
                AttitudeType::ATTITUDE_FURIOUS | AttitudeType::ATTITUDE_ANNOYED | AttitudeType::ATTITUDE_CAUTIOUS => value *= 100,
                AttitudeType::ATTITUDE_PLEASED => value *= (100 - g.team_get_at_war_count(t, true) * 10).max(75),
                AttitudeType::ATTITUDE_FRIENDLY => value *= (100 - g.team_get_at_war_count(t, true) * 20).max(50),
                _ => {}
            }
            value /= 100;
        }

        value += g.team_get_num_nuke_units(war_team) * 250; //Don't want to get nuked
        value += g.team_get_num_nuke_units(team) * 150; //Don't want to use nukes on another's behalf

        if g.team_get_at_war_count(war_team, false) == 0 {
            value *= 2;

            for o in g.civ_team_ids() {
                if g.team_is_alive(o) {
                    if o != t && o != war_team && o != team {
                        if g.teams[war_team].defensive_pact[o] {
                            value += g.teams[o].num_cities * 30;
                            value += g.team_get_total_population(o, true) * 20;
                        }
                    }
                }
            }
        }

        value = mul_div(value, 60 + (140 * g.game_turn()) / g.estimate_end_turn.max(1), 100);

        diplomacy_value(g, t, value)
    }

    /// C++ `DenialTypes CvTeamAI::AI_declareWarTrade(TeamTypes eWarTeam, TeamTypes eTeam, bool bConsiderPower = true) const` (CvTeamAI.cpp:2067-2158)
    pub fn team_ai_declare_war_trade(&self, g: &Game, t: TeamId, war_team: TeamId, team: TeamId, consider_power: bool) -> Option<DenialType> {
        if g.teams[war_team].vassal[team] || g.teams[war_team].defensive_pact[team] {
            return Some(DenialType::DENIAL_JOKING);
        }

        if g.team_is_human(t) {
            return None;
        }

        if !g.team_can_declare_war(t, war_team) {
            return Some(DenialType::DENIAL_VASSAL);
        }

        if g.team_get_any_war_plan_count(t, true) > 0 {
            return Some(DenialType::DENIAL_TOO_MANY_WARS);
        }

        if consider_power {
            let land_target = self.team_ai_is_ally_land_target(g, t, war_team);

            let their_power = g.team_get_defensive_power(war_team) / if land_target { 2 } else { 1 };
            let our_power = g.team_get_power(t, true) + if g.teams[war_team].at_war[team] { g.team_get_power(team, true) } else { 0 };
            if their_power > our_power {
                if land_target {
                    return Some(DenialType::DENIAL_POWER_THEM);
                } else {
                    return Some(DenialType::DENIAL_NO_GAIN);
                }
            }
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in members(g, t) {
            if att_le(attitude, leader_info(g, p).declare_war_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }

        let attitude_them = self.team_ai_get_attitude(g, t, war_team, true);
        for p in members(g, t) {
            if att_gt(attitude_them, leader_info(g, p).declare_war_them_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE_THEM);
            }
        }

        if !g.teams[war_team].at_war[team] {
            if g.team_get_num_nuke_units(war_team) > 0 {
                return Some(DenialType::DENIAL_JOKING);
            }
        }

        None
    }

    /// C++ `int CvTeamAI::AI_openBordersTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:2161-2164)
    pub fn team_ai_open_borders_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        g.teams[t].num_cities + g.teams[team].num_cities
    }

    /// C++ `DenialTypes CvTeamAI::AI_openBordersTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2167-2216)
    pub fn team_ai_open_borders_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        if g.team_is_human(t) {
            return None;
        }

        if g.teams[t].vassal[team] {
            return None;
        }

        if self.team_ai_share_war(g, t, team) {
            return None;
        }

        if self.team_ai_get_memory_count(g, t, team, MemoryType::MEMORY_CANCELLED_OPEN_BORDERS) > 0 {
            return Some(DenialType::DENIAL_RECENT_CANCEL);
        }

        if g.teams[t].ai_worst_enemy == Some(team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        let attitude = self.team_ai_get_attitude(g, t, team, true);
        for p in members(g, t) {
            if att_le(attitude, leader_info(g, p).open_borders_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }
        None
    }
}
