//! Port of CvUnitAI.cpp:16969-19726 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `bool CvUnitAI::AI_exploreAir()` (CvUnitAI.cpp:16969-17020)
    pub fn unit_ai_explore_air(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_exploreAir")
    }

    /// C++ `bool CvUnitAI::AI_nuke()` (CvUnitAI.cpp:17024-17073)
    pub fn unit_ai_nuke(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_nuke")
    }

    /// C++ `bool CvUnitAI::AI_nukeRange(int iRange)` (CvUnitAI.cpp:17075-17194)
    pub fn unit_ai_nuke_range(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_nukeRange")
    }

    /// C++ `bool CvUnitAI::AI_trade(int iValueThreshold)` (CvUnitAI.cpp:17196-17267)
    pub fn unit_ai_trade(&self, g: &mut Game, u: UnitId, value_threshold: i32) -> bool {
        todo!("CvUnitAI::AI_trade")
    }

    /// C++ `bool CvUnitAI::AI_infiltrate()` (CvUnitAI.cpp:17269-17341)
    pub fn unit_ai_infiltrate(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_infiltrate")
    }

    /// C++ `bool CvUnitAI::AI_reconSpy(int iRange)` (CvUnitAI.cpp:17343-17443)
    pub fn unit_ai_recon_spy(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_reconSpy")
    }

    /// C++ `int CvUnitAI::AI_getEspionageTargetValue(CvPlot* pPlot, int iMaxPath)` (CvUnitAI.cpp:17445-17492)
    pub fn unit_ai_get_espionage_target_value(&self, g: &Game, u: UnitId, plot: PlotIdx, max_path: i32) -> i32 {
        todo!("CvUnitAI::AI_getEspionageTargetValue")
    }

    /// C++ `bool CvUnitAI::AI_cityOffenseSpy(int iMaxPath)` (CvUnitAI.cpp:17495-17540)
    pub fn unit_ai_city_offense_spy(&self, g: &mut Game, u: UnitId, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_cityOffenseSpy")
    }

    /// C++ `bool CvUnitAI::AI_bonusOffenseSpy(int iRange)` (CvUnitAI.cpp:17542-17583)
    pub fn unit_ai_bonus_offense_spy(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_bonusOffenseSpy")
    }

    /// C++ `bool CvUnitAI::AI_espionageSpy()` (CvUnitAI.cpp:17586-17615)
    pub fn unit_ai_espionage_spy(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_espionageSpy")
    }

    /// C++ `bool CvUnitAI::AI_moveToStagingCity()` (CvUnitAI.cpp:17617-17691)
    pub fn unit_ai_move_to_staging_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_moveToStagingCity")
    }

    /// C++ `bool CvUnitAI::AI_seaRetreatFromCityDanger()` (CvUnitAI.cpp:17693-17718)
    pub fn unit_ai_sea_retreat_from_city_danger(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_seaRetreatFromCityDanger")
    }

    /// C++ `bool CvUnitAI::AI_airRetreatFromCityDanger()` (CvUnitAI.cpp:17720-17739)
    pub fn unit_ai_air_retreat_from_city_danger(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airRetreatFromCityDanger")
    }

    /// C++ `bool CvUnitAI::AI_airAttackDamagedSkip()` (CvUnitAI.cpp:17741-17778)
    pub fn unit_ai_air_attack_damaged_skip(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airAttackDamagedSkip")
    }

    /// C++ `bool CvUnitAI::AI_followBombard()` (CvUnitAI.cpp:17782-17847)
    pub fn unit_ai_follow_bombard(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_followBombard")
    }

    /// C++ `bool CvUnitAI::AI_potentialEnemy(TeamTypes eTeam, const CvPlot* pPlot = NULL)` (CvUnitAI.cpp:17851-17861)
    pub fn unit_ai_potential_enemy(&self, g: &Game, u: UnitId, team: TeamId, plot: Option<PlotIdx>) -> bool {
        todo!("CvUnitAI::AI_potentialEnemy")
    }

    /// C++ `bool CvUnitAI::AI_defendPlot(CvPlot* pPlot)` (CvUnitAI.cpp:17865-17903)
    pub fn unit_ai_defend_plot(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        todo!("CvUnitAI::AI_defendPlot")
    }

    /// C++ `int CvUnitAI::AI_pillageValue(CvPlot* pPlot, int iBonusValueThreshold = 0)` (CvUnitAI.cpp:17906-18017)
    pub fn unit_ai_pillage_value(&self, g: &Game, u: UnitId, plot: PlotIdx, bonus_value_threshold: i32) -> i32 {
        todo!("CvUnitAI::AI_pillageValue")
    }

    /// C++ `int CvUnitAI::AI_nukeValue(CvCity* pCity)` (CvUnitAI.cpp:18020-18089)
    pub fn unit_ai_nuke_value(&self, g: &Game, u: UnitId, city: CityId) -> i32 {
        todo!("CvUnitAI::AI_nukeValue")
    }

    /// C++ `int CvUnitAI::AI_searchRange(int iRange)` (CvUnitAI.cpp:18092-18107)
    pub fn unit_ai_search_range(&self, g: &Game, u: UnitId, range: i32) -> i32 {
        todo!("CvUnitAI::AI_searchRange")
    }

    /// C++ `bool CvUnitAI::AI_plotValid(CvPlot* pPlot)` (CvUnitAI.cpp:18111-18154)
    pub fn unit_ai_plot_valid(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        todo!("CvUnitAI::AI_plotValid")
    }

    /// C++ `int CvUnitAI::AI_finalOddsThreshold(CvPlot* pPlot, int iOddsThreshold)` (CvUnitAI.cpp:18157-18203)
    pub fn unit_ai_final_odds_threshold(&self, g: &Game, u: UnitId, plot: PlotIdx, odds_threshold: i32) -> i32 {
        todo!("CvUnitAI::AI_finalOddsThreshold")
    }

    /// C++ `int CvUnitAI::AI_stackOfDoomExtra()` (CvUnitAI.cpp:18206-18209)
    pub fn unit_ai_stack_of_doom_extra(&self, g: &Game, u: UnitId) -> i32 {
        todo!("CvUnitAI::AI_stackOfDoomExtra")
    }

    /// C++ `bool CvUnitAI::AI_stackAttackCity(int iRange, int iPowerThreshold, bool bFollow = true)` (CvUnitAI.cpp:18211-18279)
    pub fn unit_ai_stack_attack_city(&self, g: &mut Game, u: UnitId, range: i32, power_threshold: i32, follow: bool) -> bool {
        todo!("CvUnitAI::AI_stackAttackCity")
    }

    /// C++ `bool CvUnitAI::AI_moveIntoCity(int iRange)` (CvUnitAI.cpp:18281-18346)
    pub fn unit_ai_move_into_city(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_moveIntoCity")
    }

    /// C++ `bool CvUnitAI::AI_artistCultureVictoryMove()` (CvUnitAI.cpp:18350-18478)
    pub fn unit_ai_artist_culture_victory_move(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_artistCultureVictoryMove")
    }

    /// C++ `bool CvUnitAI::AI_poach()` (CvUnitAI.cpp:18480-18588)
    pub fn unit_ai_poach(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_poach")
    }

    /// C++ `bool CvUnitAI::AI_choke(int iRange = 1)` (CvUnitAI.cpp:18590-18659)
    pub fn unit_ai_choke(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_choke")
    }

    /// C++ `bool CvUnitAI::AI_solveBlockageProblem(CvPlot* pDestPlot, bool bDeclareWar)` (CvUnitAI.cpp:18661-18736)
    pub fn unit_ai_solve_blockage_problem(&self, g: &mut Game, u: UnitId, dest_plot: PlotIdx, declare_war: bool) -> bool {
        todo!("CvUnitAI::AI_solveBlockageProblem")
    }

    /// C++ `int CvUnitAI::AI_calculatePlotWorkersNeeded(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:18738-18763)
    pub fn unit_ai_calculate_plot_workers_needed(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> i32 {
        todo!("CvUnitAI::AI_calculatePlotWorkersNeeded")
    }

    /// C++ `bool CvUnitAI::AI_canGroupWithAIType(UnitAITypes eUnitAI) const` (CvUnitAI.cpp:18765-18805)
    pub fn unit_ai_can_group_with_ai_type(&self, g: &Game, u: UnitId, unit_ai: UnitAIType) -> bool {
        todo!("CvUnitAI::AI_canGroupWithAIType")
    }

    /// C++ `bool CvUnitAI::AI_allowGroup(const CvUnit* pUnit, UnitAITypes eUnitAI) const` (CvUnitAI.cpp:18809-19006)
    pub fn unit_ai_allow_group(&self, g: &Game, u: UnitId, unit: UnitId, unit_ai: UnitAIType) -> bool {
        todo!("CvUnitAI::AI_allowGroup")
    }

    /// C++ `void CvUnitAI::AI_summonAttackMove()` (CvUnitAI.cpp:19071-19092)
    pub fn unit_ai_summon_attack_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_summonAttackMove")
    }

    /// C++ `CvPlot* CvUnitAI::getOriginPlot() const` (CvUnitAI.cpp:19100-19107)
    pub fn unit_ai_get_origin_plot(&self, g: &Game, u: UnitId) -> Option<PlotIdx> {
        todo!("CvUnitAI::getOriginPlot")
    }

    /// C++ `void CvUnitAI::setOriginPlot(CvPlot* newplot)` (CvUnitAI.cpp:19109-19121)
    pub fn unit_ai_set_origin_plot(&self, g: &mut Game, u: UnitId, newplot: PlotIdx) {
        todo!("CvUnitAI::setOriginPlot")
    }

    /// C++ `int CvUnitAI::AI_getGroupflag() const` (CvUnitAI.cpp:19123-19126)
    pub fn unit_ai_get_groupflag(&self, g: &Game, u: UnitId) -> i32 {
        todo!("CvUnitAI::AI_getGroupflag")
    }

    /// C++ `void CvUnitAI::AI_setGroupflag(int newflag)` (CvUnitAI.cpp:19128-19131)
    pub fn unit_ai_set_groupflag(&self, g: &mut Game, u: UnitId, newflag: i32) {
        todo!("CvUnitAI::AI_setGroupflag")
    }

    /// C++ `void CvUnitAI::AI_chooseGroupflag()` (CvUnitAI.cpp:19134-19226)
    pub fn unit_ai_choose_groupflag(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_chooseGroupflag")
    }

    /// C++ `bool CvUnitAI::isUnitAllowedExploreGroup()` (CvUnitAI.cpp:19228-19290)
    pub fn unit_ai_is_unit_allowed_explore_group(&self, g: &Game, u: UnitId) -> bool {
        todo!("CvUnitAI::isUnitAllowedExploreGroup")
    }

    /// C++ `bool CvUnitAI::AI_decide_exploregroup()` (CvUnitAI.cpp:19293-19319)
    pub fn unit_ai_decide_exploregroup(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_decide_exploregroup")
    }

    /// C++ `bool CvUnitAI::AI_decide_permdefensegroup()` (CvUnitAI.cpp:19322-19382)
    pub fn unit_ai_decide_permdefensegroup(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_decide_permdefensegroup")
    }

    /// C++ `bool CvUnitAI::AI_decide_patrolgroup()` (CvUnitAI.cpp:19385-19462)
    pub fn unit_ai_decide_patrolgroup(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_decide_patrolgroup")
    }

    /// C++ `bool CvUnitAI::isUnitAllowedPermDefense()` (CvUnitAI.cpp:19464-19531)
    pub fn unit_ai_is_unit_allowed_perm_defense(&self, g: &Game, u: UnitId) -> bool {
        todo!("CvUnitAI::isUnitAllowedPermDefense")
    }

    /// C++ `bool CvUnitAI::isUnitAllowedPatrolGroup()` (CvUnitAI.cpp:19533-19595)
    pub fn unit_ai_is_unit_allowed_patrol_group(&self, g: &Game, u: UnitId) -> bool {
        todo!("CvUnitAI::isUnitAllowedPatrolGroup")
    }

    /// C++ `bool CvUnitAI::isUnitAllowedDefenseGroup()` (CvUnitAI.cpp:19598-19644)
    pub fn unit_ai_is_unit_allowed_defense_group(&self, g: &Game, u: UnitId) -> bool {
        todo!("CvUnitAI::isUnitAllowedDefenseGroup")
    }

    /// C++ `bool CvUnitAI::AI_groupheal(int iDamagePercent = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:19647-19726)
    pub fn unit_ai_groupheal(&self, g: &mut Game, u: UnitId, damage_percent: i32, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_groupheal")
    }

}
