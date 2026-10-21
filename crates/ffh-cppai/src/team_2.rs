//! Port of CvTeamAI.cpp:2219-4403, CvSelectionGroupAI.cpp:28-1003, CvGameAI.cpp:24-110 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `int CvTeamAI::AI_defensivePactTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:2219-2222)
    pub fn team_ai_defensive_pact_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_defensivePactTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_defensivePactTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2225-2265)
    pub fn team_ai_defensive_pact_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_defensivePactTrade")
    }

    /// C++ `DenialTypes CvTeamAI::AI_permanentAllianceTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2268-2320)
    pub fn team_ai_permanent_alliance_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_permanentAllianceTrade")
    }

    /// C++ `TeamTypes CvTeamAI::AI_getWorstEnemy() const` (CvTeamAI.cpp:2323-2326)
    pub fn team_ai_get_worst_enemy(&self, g: &Game, t: TeamId) -> Option<TeamId> {
        todo!("CvTeamAI::AI_getWorstEnemy")
    }

    /// C++ `void CvTeamAI::AI_updateWorstEnemy()` (CvTeamAI.cpp:2329-2361)
    pub fn team_ai_update_worst_enemy(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_updateWorstEnemy")
    }

    /// C++ `int CvTeamAI::AI_getWarPlanStateCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2364-2369)
    pub fn team_ai_get_war_plan_state_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getWarPlanStateCounter")
    }

    /// C++ `void CvTeamAI::AI_setWarPlanStateCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2372-2378)
    pub fn team_ai_set_war_plan_state_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setWarPlanStateCounter")
    }

    /// C++ `void CvTeamAI::AI_changeWarPlanStateCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2381-2384)
    pub fn team_ai_change_war_plan_state_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeWarPlanStateCounter")
    }

    /// C++ `int CvTeamAI::AI_getAtWarCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2387-2392)
    pub fn team_ai_get_at_war_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getAtWarCounter")
    }

    /// C++ `void CvTeamAI::AI_setAtWarCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2395-2401)
    pub fn team_ai_set_at_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setAtWarCounter")
    }

    /// C++ `void CvTeamAI::AI_changeAtWarCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2404-2407)
    pub fn team_ai_change_at_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeAtWarCounter")
    }

    /// C++ `int CvTeamAI::AI_getAtPeaceCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2410-2415)
    pub fn team_ai_get_at_peace_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getAtPeaceCounter")
    }

    /// C++ `void CvTeamAI::AI_setAtPeaceCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2418-2424)
    pub fn team_ai_set_at_peace_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setAtPeaceCounter")
    }

    /// C++ `void CvTeamAI::AI_changeAtPeaceCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2427-2430)
    pub fn team_ai_change_at_peace_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeAtPeaceCounter")
    }

    /// C++ `int CvTeamAI::AI_getHasMetCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2433-2438)
    pub fn team_ai_get_has_met_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getHasMetCounter")
    }

    /// C++ `void CvTeamAI::AI_setHasMetCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2441-2447)
    pub fn team_ai_set_has_met_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setHasMetCounter")
    }

    /// C++ `void CvTeamAI::AI_changeHasMetCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2450-2453)
    pub fn team_ai_change_has_met_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeHasMetCounter")
    }

    /// C++ `int CvTeamAI::AI_getOpenBordersCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2456-2461)
    pub fn team_ai_get_open_borders_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getOpenBordersCounter")
    }

    /// C++ `void CvTeamAI::AI_setOpenBordersCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2464-2470)
    pub fn team_ai_set_open_borders_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setOpenBordersCounter")
    }

    /// C++ `void CvTeamAI::AI_changeOpenBordersCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2473-2476)
    pub fn team_ai_change_open_borders_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeOpenBordersCounter")
    }

    /// C++ `int CvTeamAI::AI_getDefensivePactCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2479-2484)
    pub fn team_ai_get_defensive_pact_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getDefensivePactCounter")
    }

    /// C++ `void CvTeamAI::AI_setDefensivePactCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2487-2493)
    pub fn team_ai_set_defensive_pact_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setDefensivePactCounter")
    }

    /// C++ `void CvTeamAI::AI_changeDefensivePactCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2496-2499)
    pub fn team_ai_change_defensive_pact_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeDefensivePactCounter")
    }

    /// C++ `int CvTeamAI::AI_getShareWarCounter(TeamTypes eIndex) const` (CvTeamAI.cpp:2502-2507)
    pub fn team_ai_get_share_war_counter(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getShareWarCounter")
    }

    /// C++ `void CvTeamAI::AI_setShareWarCounter(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2510-2516)
    pub fn team_ai_set_share_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setShareWarCounter")
    }

    /// C++ `void CvTeamAI::AI_changeShareWarCounter(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2519-2522)
    pub fn team_ai_change_share_war_counter(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeShareWarCounter")
    }

    /// C++ `int CvTeamAI::AI_getWarSuccess(TeamTypes eIndex) const` (CvTeamAI.cpp:2525-2530)
    pub fn team_ai_get_war_success(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getWarSuccess")
    }

    /// C++ `void CvTeamAI::AI_setWarSuccess(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2533-2539)
    pub fn team_ai_set_war_success(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setWarSuccess")
    }

    /// C++ `void CvTeamAI::AI_changeWarSuccess(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2542-2545)
    pub fn team_ai_change_war_success(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeWarSuccess")
    }

    /// C++ `int CvTeamAI::AI_getEnemyPeacetimeTradeValue(TeamTypes eIndex) const` (CvTeamAI.cpp:2548-2553)
    pub fn team_ai_get_enemy_peacetime_trade_value(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getEnemyPeacetimeTradeValue")
    }

    /// C++ `void CvTeamAI::AI_setEnemyPeacetimeTradeValue(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2556-2562)
    pub fn team_ai_set_enemy_peacetime_trade_value(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setEnemyPeacetimeTradeValue")
    }

    /// C++ `void CvTeamAI::AI_changeEnemyPeacetimeTradeValue(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2565-2568)
    pub fn team_ai_change_enemy_peacetime_trade_value(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeEnemyPeacetimeTradeValue")
    }

    /// C++ `int CvTeamAI::AI_getEnemyPeacetimeGrantValue(TeamTypes eIndex) const` (CvTeamAI.cpp:2571-2576)
    pub fn team_ai_get_enemy_peacetime_grant_value(&self, g: &Game, t: TeamId, index: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getEnemyPeacetimeGrantValue")
    }

    /// C++ `void CvTeamAI::AI_setEnemyPeacetimeGrantValue(TeamTypes eIndex, int iNewValue)` (CvTeamAI.cpp:2579-2585)
    pub fn team_ai_set_enemy_peacetime_grant_value(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: i32) {
        todo!("CvTeamAI::AI_setEnemyPeacetimeGrantValue")
    }

    /// C++ `void CvTeamAI::AI_changeEnemyPeacetimeGrantValue(TeamTypes eIndex, int iChange)` (CvTeamAI.cpp:2588-2591)
    pub fn team_ai_change_enemy_peacetime_grant_value(&self, g: &mut Game, t: TeamId, index: TeamId, change: i32) {
        todo!("CvTeamAI::AI_changeEnemyPeacetimeGrantValue")
    }

    /// C++ `WarPlanTypes CvTeamAI::AI_getWarPlan(TeamTypes eIndex) const` (CvTeamAI.cpp:2594-2600)
    pub fn team_ai_get_war_plan(&self, g: &Game, t: TeamId, index: TeamId) -> Option<WarPlanType> {
        todo!("CvTeamAI::AI_getWarPlan")
    }

    /// C++ `bool CvTeamAI::AI_isChosenWar(TeamTypes eIndex) const` (CvTeamAI.cpp:2603-2621)
    pub fn team_ai_is_chosen_war(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        todo!("CvTeamAI::AI_isChosenWar")
    }

    /// C++ `bool CvTeamAI::AI_isSneakAttackPreparing(TeamTypes eIndex) const` (CvTeamAI.cpp:2624-2627)
    pub fn team_ai_is_sneak_attack_preparing(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        todo!("CvTeamAI::AI_isSneakAttackPreparing")
    }

    /// C++ `bool CvTeamAI::AI_isSneakAttackReady(TeamTypes eIndex) const` (CvTeamAI.cpp:2630-2633)
    pub fn team_ai_is_sneak_attack_ready(&self, g: &Game, t: TeamId, index: TeamId) -> bool {
        todo!("CvTeamAI::AI_isSneakAttackReady")
    }

    /// C++ `void CvTeamAI::AI_setWarPlan(TeamTypes eIndex, WarPlanTypes eNewValue, bool bWar = true)` (CvTeamAI.cpp:2636-2667)
    pub fn team_ai_set_war_plan(&self, g: &mut Game, t: TeamId, index: TeamId, new_value: WarPlanType, war: bool) {
        todo!("CvTeamAI::AI_setWarPlan")
    }

    /// C++ `int CvTeamAI::AI_teamCloseness(TeamTypes eIndex, int iMaxDistance = -1) const` (CvTeamAI.cpp:2671-2704)
    pub fn team_ai_team_closeness(&self, g: &Game, t: TeamId, index: TeamId, max_distance: i32) -> i32 {
        todo!("CvTeamAI::AI_teamCloseness")
    }

    /// C++ `int CvTeamAI::AI_noTechTradeThreshold() const` (CvTeamAI.cpp:2754-2780)
    pub fn team_ai_no_tech_trade_threshold(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_noTechTradeThreshold")
    }

    /// C++ `int CvTeamAI::AI_techTradeKnownPercent() const` (CvTeamAI.cpp:2783-2809)
    pub fn team_ai_tech_trade_known_percent(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_techTradeKnownPercent")
    }

    /// C++ `int CvTeamAI::AI_maxWarRand() const` (CvTeamAI.cpp:2812-2838)
    pub fn team_ai_max_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_maxWarRand")
    }

    /// C++ `int CvTeamAI::AI_maxWarNearbyPowerRatio() const` (CvTeamAI.cpp:2841-2867)
    pub fn team_ai_max_war_nearby_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_maxWarNearbyPowerRatio")
    }

    /// C++ `int CvTeamAI::AI_maxWarDistantPowerRatio() const` (CvTeamAI.cpp:2870-2896)
    pub fn team_ai_max_war_distant_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_maxWarDistantPowerRatio")
    }

    /// C++ `int CvTeamAI::AI_maxWarMinAdjacentLandPercent() const` (CvTeamAI.cpp:2899-2925)
    pub fn team_ai_max_war_min_adjacent_land_percent(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_maxWarMinAdjacentLandPercent")
    }

    /// C++ `int CvTeamAI::AI_limitedWarRand() const` (CvTeamAI.cpp:2928-2954)
    pub fn team_ai_limited_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_limitedWarRand")
    }

    /// C++ `int CvTeamAI::AI_limitedWarPowerRatio() const` (CvTeamAI.cpp:2957-2984)
    pub fn team_ai_limited_war_power_ratio(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_limitedWarPowerRatio")
    }

    /// C++ `int CvTeamAI::AI_dogpileWarRand() const` (CvTeamAI.cpp:2987-3013)
    pub fn team_ai_dogpile_war_rand(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_dogpileWarRand")
    }

    /// C++ `int CvTeamAI::AI_makePeaceRand() const` (CvTeamAI.cpp:3016-3042)
    pub fn team_ai_make_peace_rand(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_makePeaceRand")
    }

    /// C++ `int CvTeamAI::AI_noWarAttitudeProb(AttitudeTypes eAttitude) const` (CvTeamAI.cpp:3045-3071)
    pub fn team_ai_no_war_attitude_prob(&self, g: &Game, t: TeamId, attitude: AttitudeType) -> i32 {
        todo!("CvTeamAI::AI_noWarAttitudeProb")
    }

    /// C++ `void CvTeamAI::AI_doCounter()` (CvTeamAI.cpp:3074-3127)
    pub fn team_ai_do_counter(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_doCounter")
    }

    /// C++ `void CvTeamAI::AI_doWar()` (CvTeamAI.cpp:3130-3793)
    pub fn team_ai_do_war(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_doWar")
    }

    /// C++ `bool CvTeamAI::AI_performNoWarRolls(TeamTypes eTeam)` (CvTeamAI.cpp:3796-3812)
    pub fn team_ai_perform_no_war_rolls(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        todo!("CvTeamAI::AI_performNoWarRolls")
    }

    /// C++ `int CvTeamAI::AI_getAttitudeWeight(TeamTypes eTeam)` (CvTeamAI.cpp:3814-3837)
    pub fn team_ai_get_attitude_weight(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getAttitudeWeight")
    }

    /// C++ `int CvTeamAI::AI_getLowestVictoryCountdown() const` (CvTeamAI.cpp:3839-3855)
    pub fn team_ai_get_lowest_victory_countdown(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getLowestVictoryCountdown")
    }

    /// C++ `int CvTeamAI::AI_getTechMonopolyValue(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:3857-4039)
    pub fn team_ai_get_tech_monopoly_value(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getTechMonopolyValue")
    }

    /// C++ `bool CvTeamAI::AI_isWaterAreaRelevant(CvArea* pArea)` (CvTeamAI.cpp:4041-4091)
    pub fn team_ai_is_water_area_relevant(&self, g: &Game, t: TeamId, area: AreaId) -> bool {
        todo!("CvTeamAI::AI_isWaterAreaRelevant")
    }

    /// C++ `bool CvTeamAI::AI_acceptSurrender(TeamTypes eSurrenderTeam)` (CvTeamAI.cpp:4099-4356)
    pub fn team_ai_accept_surrender(&self, g: &Game, t: TeamId, surrender_team: TeamId) -> bool {
        todo!("CvTeamAI::AI_acceptSurrender")
    }

    /// C++ `int CvTeamAI::AI_getOurPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves, bool bIncludeVassals)` (CvTeamAI.cpp:4359-4376)
    pub fn team_ai_get_our_plot_strength(&self, g: &Game, t: TeamId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool, include_vassals: bool) -> i32 {
        todo!("CvTeamAI::AI_getOurPlotStrength")
    }

    /// C++ `int CvTeamAI::AI_getWarSuccessCapitulationRatio() const` (CvTeamAI.cpp:4382-4403)
    pub fn team_ai_get_war_success_capitulation_ratio(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_getWarSuccessCapitulationRatio")
    }

    /// C++ `void CvSelectionGroupAI::AI_init()` (CvSelectionGroupAI.cpp:28-34)
    pub fn group_ai_init(&self, g: &mut Game, grp: GroupId) {
        todo!("CvSelectionGroupAI::AI_init")
    }

    /// C++ `void CvSelectionGroupAI::AI_reset()` (CvSelectionGroupAI.cpp:42-58)
    pub fn group_ai_reset(&self, g: &mut Game, grp: GroupId) {
        todo!("CvSelectionGroupAI::AI_reset")
    }

    /// C++ `void CvSelectionGroupAI::AI_separate()` (CvSelectionGroupAI.cpp:61-79)
    pub fn group_ai_separate(&self, g: &mut Game, grp: GroupId) {
        todo!("CvSelectionGroupAI::AI_separate")
    }

    /// C++ `void CvSelectionGroupAI::AI_seperateNonAI(UnitAITypes eUnitAI)` (CvSelectionGroupAI.cpp:81-101)
    pub fn group_ai_seperate_non_ai(&self, g: &mut Game, grp: GroupId, unit_ai: UnitAIType) {
        todo!("CvSelectionGroupAI::AI_seperateNonAI")
    }

    /// C++ `void CvSelectionGroupAI::AI_seperateAI(UnitAITypes eUnitAI)` (CvSelectionGroupAI.cpp:103-123)
    pub fn group_ai_seperate_ai(&self, g: &mut Game, grp: GroupId, unit_ai: UnitAIType) {
        todo!("CvSelectionGroupAI::AI_seperateAI")
    }

    /// C++ `bool CvSelectionGroupAI::AI_update()` (CvSelectionGroupAI.cpp:125-281)
    pub fn group_ai_update(&self, g: &mut Game, grp: GroupId) -> bool {
        todo!("CvSelectionGroupAI::AI_update")
    }

    /// C++ `int CvSelectionGroupAI::AI_attackOdds(const CvPlot* pPlot, bool bPotentialEnemy) const` (CvSelectionGroupAI.cpp:285-305)
    pub fn group_ai_attack_odds(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool) -> i32 {
        todo!("CvSelectionGroupAI::AI_attackOdds")
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getBestGroupAttacker(const CvPlot* pPlot, bool bPotentialEnemy, int& iUnitOdds, bool bForce = false, bool bNoBlitz = false) const` (CvSelectionGroupAI.cpp:308-417)
    pub fn group_ai_get_best_group_attacker(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, unit_odds: &mut i32, force: bool, no_blitz: bool) -> Option<UnitId> {
        todo!("CvSelectionGroupAI::AI_getBestGroupAttacker")
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getBestGroupSacrifice(const CvPlot* pPlot, bool bPotentialEnemy, bool bForce = false, bool bNoBlitz = false) const` (CvSelectionGroupAI.cpp:419-505)
    pub fn group_ai_get_best_group_sacrifice(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, force: bool, no_blitz: bool) -> Option<UnitId> {
        todo!("CvSelectionGroupAI::AI_getBestGroupSacrifice")
    }

    /// C++ `int CvSelectionGroupAI::AI_compareStacks(const CvPlot* pPlot, bool bPotentialEnemy, bool bCheckCanAttack = false, bool bCheckCanMove = false) const` (CvSelectionGroupAI.cpp:509-540)
    pub fn group_ai_compare_stacks(&self, g: &Game, grp: GroupId, plot: PlotIdx, potential_enemy: bool, check_can_attack: bool, check_can_move: bool) -> i32 {
        todo!("CvSelectionGroupAI::AI_compareStacks")
    }

    /// C++ `int CvSelectionGroupAI::AI_sumStrength(const CvPlot* pAttackedPlot = NULL, DomainTypes eDomainType = NO_DOMAIN, bool bCheckCanAttack = false, bool bCheckCanMove = false) const` (CvSelectionGroupAI.cpp:542-574)
    pub fn group_ai_sum_strength(&self, g: &Game, grp: GroupId, attacked_plot: Option<PlotIdx>, domain_type: Option<DomainType>, check_can_attack: bool, check_can_move: bool) -> i32 {
        todo!("CvSelectionGroupAI::AI_sumStrength")
    }

    /// C++ `void CvSelectionGroupAI::AI_queueGroupAttack(int iX, int iY)` (CvSelectionGroupAI.cpp:576-582)
    pub fn group_ai_queue_group_attack(&self, g: &mut Game, grp: GroupId, x: i32, y: i32) {
        todo!("CvSelectionGroupAI::AI_queueGroupAttack")
    }

    /// C++ `inline void CvSelectionGroupAI::AI_cancelGroupAttack()` (CvSelectionGroupAI.cpp:584-587)
    pub fn group_ai_cancel_group_attack(&self, g: &mut Game, grp: GroupId) {
        todo!("CvSelectionGroupAI::AI_cancelGroupAttack")
    }

    /// C++ `inline bool CvSelectionGroupAI::AI_isGroupAttack()` (CvSelectionGroupAI.cpp:589-592)
    pub fn group_ai_is_group_attack(&self, g: &Game, grp: GroupId) -> bool {
        todo!("CvSelectionGroupAI::AI_isGroupAttack")
    }

    /// C++ `bool CvSelectionGroupAI::AI_isControlled()` (CvSelectionGroupAI.cpp:594-597)
    pub fn group_ai_is_controlled(&self, g: &Game, grp: GroupId) -> bool {
        todo!("CvSelectionGroupAI::AI_isControlled")
    }

    /// C++ `bool CvSelectionGroupAI::AI_isDeclareWar(const CvPlot* pPlot = NULL)` (CvSelectionGroupAI.cpp:600-730)
    pub fn group_ai_is_declare_war(&self, g: &Game, grp: GroupId, plot: Option<PlotIdx>) -> bool {
        todo!("CvSelectionGroupAI::AI_isDeclareWar")
    }

    /// C++ `CvPlot* CvSelectionGroupAI::AI_getMissionAIPlot()` (CvSelectionGroupAI.cpp:733-736)
    pub fn group_ai_get_mission_ai_plot(&self, g: &Game, grp: GroupId) -> Option<PlotIdx> {
        todo!("CvSelectionGroupAI::AI_getMissionAIPlot")
    }

    /// C++ `bool CvSelectionGroupAI::AI_isForceSeparate()` (CvSelectionGroupAI.cpp:739-742)
    pub fn group_ai_is_force_separate(&self, g: &Game, grp: GroupId) -> bool {
        todo!("CvSelectionGroupAI::AI_isForceSeparate")
    }

    /// C++ `void CvSelectionGroupAI::AI_makeForceSeparate()` (CvSelectionGroupAI.cpp:745-748)
    pub fn group_ai_make_force_separate(&self, g: &mut Game, grp: GroupId) {
        todo!("CvSelectionGroupAI::AI_makeForceSeparate")
    }

    /// C++ `MissionAITypes CvSelectionGroupAI::AI_getMissionAIType()` (CvSelectionGroupAI.cpp:751-754)
    pub fn group_ai_get_mission_ai_type(&self, g: &Game, grp: GroupId) -> Option<MissionAIType> {
        todo!("CvSelectionGroupAI::AI_getMissionAIType")
    }

    /// C++ `void CvSelectionGroupAI::AI_setMissionAI(MissionAITypes eNewMissionAI, CvPlot* pNewPlot, CvUnit* pNewUnit)` (CvSelectionGroupAI.cpp:757-780)
    pub fn group_ai_set_mission_ai(&self, g: &mut Game, grp: GroupId, new_mission_ai: MissionAIType, new_plot: PlotIdx, new_unit: UnitId) {
        todo!("CvSelectionGroupAI::AI_setMissionAI")
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_getMissionAIUnit()` (CvSelectionGroupAI.cpp:783-786)
    pub fn group_ai_get_mission_ai_unit(&self, g: &Game, grp: GroupId) -> Option<UnitId> {
        todo!("CvSelectionGroupAI::AI_getMissionAIUnit")
    }

    /// C++ `bool CvSelectionGroupAI::AI_isFull()` (CvSelectionGroupAI.cpp:788-848)
    pub fn group_ai_is_full(&self, g: &Game, grp: GroupId) -> bool {
        todo!("CvSelectionGroupAI::AI_isFull")
    }

    /// C++ `CvUnit* CvSelectionGroupAI::AI_ejectBestDefender(CvPlot* pDefendPlot)` (CvSelectionGroupAI.cpp:851-895)
    pub fn group_ai_eject_best_defender(&self, g: &mut Game, grp: GroupId, defend_plot: PlotIdx) -> Option<UnitId> {
        todo!("CvSelectionGroupAI::AI_ejectBestDefender")
    }

    /// C++ `int CvSelectionGroupAI::AI_getGroupflag() const` (CvSelectionGroupAI.cpp:952-955)
    pub fn group_ai_get_groupflag(&self, g: &Game, grp: GroupId) -> i32 {
        todo!("CvSelectionGroupAI::AI_getGroupflag")
    }

    /// C++ `void CvSelectionGroupAI::AI_setGroupflag(int newflag)` (CvSelectionGroupAI.cpp:957-966)
    pub fn group_ai_set_groupflag(&self, g: &mut Game, grp: GroupId, newflag: i32) {
        todo!("CvSelectionGroupAI::AI_setGroupflag")
    }

    /// C++ `bool CvSelectionGroupAI::isHeadUnit(CvUnit* pUnit)` (CvSelectionGroupAI.cpp:968-975)
    pub fn group_ai_is_head_unit(&self, g: &Game, grp: GroupId, unit: UnitId) -> bool {
        todo!("CvSelectionGroupAI::isHeadUnit")
    }

    /// C++ `int CvSelectionGroupAI::AI_GroupPower(CvPlot* pPlot, bool bDefensiveBonuses) const` (CvSelectionGroupAI.cpp:977-1003)
    pub fn group_ai_group_power(&self, g: &Game, grp: GroupId, plot: PlotIdx, defensive_bonuses: bool) -> i32 {
        todo!("CvSelectionGroupAI::AI_GroupPower")
    }

    /// C++ `void CvGameAI::AI_init()` (CvGameAI.cpp:24-30)
    pub fn game_ai_init(&self, g: &mut Game) {
        todo!("CvGameAI::AI_init")
    }

    /// C++ `void CvGameAI::AI_reset()` (CvGameAI.cpp:38-43)
    pub fn game_ai_reset(&self, g: &mut Game) {
        todo!("CvGameAI::AI_reset")
    }

    /// C++ `void CvGameAI::AI_makeAssignWorkDirty()` (CvGameAI.cpp:46-57)
    pub fn game_ai_make_assign_work_dirty(&self, g: &mut Game) {
        todo!("CvGameAI::AI_makeAssignWorkDirty")
    }

    /// C++ `void CvGameAI::AI_updateAssignWork()` (CvGameAI.cpp:60-72)
    pub fn game_ai_update_assign_work(&self, g: &mut Game) {
        todo!("CvGameAI::AI_updateAssignWork")
    }

    /// C++ `int CvGameAI::AI_combatValue(UnitTypes eUnit)` (CvGameAI.cpp:76-97)
    pub fn game_ai_combat_value(&self, g: &Game, unit: UnitType) -> i32 {
        todo!("CvGameAI::AI_combatValue")
    }

    /// C++ `int CvGameAI::AI_turnsPercent(int iTurns, int iPercent)` (CvGameAI.cpp:100-110)
    pub fn game_ai_turns_percent(&self, g: &Game, turns: i32, percent: i32) -> i32 {
        todo!("CvGameAI::AI_turnsPercent")
    }

}
