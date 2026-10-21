//! Port of CvPlayerAI.cpp:16742-19674 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvPlayerAI::AI_nowHasTech(TechTypes eTech)` (CvPlayerAI.cpp:16742-16764)
    pub fn player_ai_now_has_tech(&self, g: &mut Game, p: PlayerId, tech: TechType) {
        todo!("CvPlayerAI::AI_nowHasTech")
    }

    /// C++ `int CvPlayerAI::AI_countDeadlockedBonuses(CvPlot* pPlot) const` (CvPlayerAI.cpp:16767-16825)
    pub fn player_ai_count_deadlocked_bonuses(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        todo!("CvPlayerAI::AI_countDeadlockedBonuses")
    }

    /// C++ `int CvPlayerAI::AI_getOurPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves) const` (CvPlayerAI.cpp:16827-16889)
    pub fn player_ai_get_our_plot_strength(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool) -> i32 {
        todo!("CvPlayerAI::AI_getOurPlotStrength")
    }

    /// C++ `int CvPlayerAI::AI_getEnemyPlotStrength(CvPlot* pPlot, int iRange, bool bDefensiveBonuses, bool bTestMoves) const` (CvPlayerAI.cpp:16891-16956)
    pub fn player_ai_get_enemy_plot_strength(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, defensive_bonuses: bool, test_moves: bool) -> i32 {
        todo!("CvPlayerAI::AI_getEnemyPlotStrength")
    }

    /// C++ `int CvPlayerAI::AI_goldToUpgradeAllUnits(int iExpThreshold = 0) const` (CvPlayerAI.cpp:16958-17067)
    pub fn player_ai_gold_to_upgrade_all_units(&self, g: &Game, p: PlayerId, exp_threshold: i32) -> i32 {
        todo!("CvPlayerAI::AI_goldToUpgradeAllUnits")
    }

    /// C++ `int CvPlayerAI::AI_goldTradeValuePercent() const` (CvPlayerAI.cpp:17069-17078)
    pub fn player_ai_gold_trade_value_percent(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_goldTradeValuePercent")
    }

    /// C++ `int CvPlayerAI::AI_averageYieldMultiplier(YieldTypes eYield) const` (CvPlayerAI.cpp:17080-17092)
    pub fn player_ai_average_yield_multiplier(&self, g: &Game, p: PlayerId, yield_: YieldType) -> i32 {
        todo!("CvPlayerAI::AI_averageYieldMultiplier")
    }

    /// C++ `int CvPlayerAI::AI_averageCommerceMultiplier(CommerceTypes eCommerce) const` (CvPlayerAI.cpp:17094-17105)
    pub fn player_ai_average_commerce_multiplier(&self, g: &Game, p: PlayerId, commerce: CommerceType) -> i32 {
        todo!("CvPlayerAI::AI_averageCommerceMultiplier")
    }

    /// C++ `int CvPlayerAI::AI_averageGreatPeopleMultiplier() const` (CvPlayerAI.cpp:17107-17114)
    pub fn player_ai_average_great_people_multiplier(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_averageGreatPeopleMultiplier")
    }

    /// C++ `int CvPlayerAI::AI_averageCommerceExchange(CommerceTypes eCommerce) const` (CvPlayerAI.cpp:17117-17128)
    pub fn player_ai_average_commerce_exchange(&self, g: &Game, p: PlayerId, commerce: CommerceType) -> i32 {
        todo!("CvPlayerAI::AI_averageCommerceExchange")
    }

    /// C++ `void CvPlayerAI::AI_calculateAverages() const` (CvPlayerAI.cpp:17130-17239)
    pub fn player_ai_calculate_averages(&self, g: &Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_calculateAverages")
    }

    /// C++ `void CvPlayerAI::AI_convertUnitAITypesForCrush()` (CvPlayerAI.cpp:17241-17285)
    pub fn player_ai_convert_unit_ai_types_for_crush(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_convertUnitAITypesForCrush")
    }

    /// C++ `int CvPlayerAI::AI_playerCloseness(PlayerTypes eIndex, int iMaxDistance) const` (CvPlayerAI.cpp:17287-17301)
    pub fn player_ai_player_closeness(&self, g: &Game, p: PlayerId, index: PlayerId, max_distance: i32) -> i32 {
        todo!("CvPlayerAI::AI_playerCloseness")
    }

    /// C++ `int CvPlayerAI::AI_getTotalAreaCityThreat(CvArea* pArea) const` (CvPlayerAI.cpp:17304-17320)
    pub fn player_ai_get_total_area_city_threat(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_getTotalAreaCityThreat")
    }

    /// C++ `int CvPlayerAI::AI_countNumAreaHostileUnits(CvArea* pArea, bool bPlayer, bool bTeam, bool bNeutral, bool bHostile) const` (CvPlayerAI.cpp:17322-17342)
    pub fn player_ai_count_num_area_hostile_units(&self, g: &Game, p: PlayerId, area: AreaId, player: bool, team: bool, neutral: bool, hostile: bool) -> i32 {
        todo!("CvPlayerAI::AI_countNumAreaHostileUnits")
    }

    /// C++ `int CvPlayerAI::AI_getTotalFloatingDefendersNeeded(CvArea* pArea) const` (CvPlayerAI.cpp:17345-17415)
    pub fn player_ai_get_total_floating_defenders_needed(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_getTotalFloatingDefendersNeeded")
    }

    /// C++ `int CvPlayerAI::AI_getTotalFloatingDefenders(CvArea* pArea) const` (CvPlayerAI.cpp:17417-17429)
    pub fn player_ai_get_total_floating_defenders(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_getTotalFloatingDefenders")
    }

    /// C++ `RouteTypes CvPlayerAI::AI_bestAdvancedStartRoute(CvPlot* pPlot, int* piYieldValue = NULL) const` (CvPlayerAI.cpp:17431-17471)
    pub fn player_ai_best_advanced_start_route(&self, g: &Game, p: PlayerId, plot: PlotIdx, yield_value: &mut i32) -> Option<RouteType> {
        todo!("CvPlayerAI::AI_bestAdvancedStartRoute")
    }

    /// C++ `UnitTypes CvPlayerAI::AI_bestAdvancedStartUnitAI(CvPlot* pPlot, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:17473-17577)
    pub fn player_ai_best_advanced_start_unit_ai(&self, g: &Game, p: PlayerId, plot: PlotIdx, unit_ai: UnitAIType) -> Option<UnitType> {
        todo!("CvPlayerAI::AI_bestAdvancedStartUnitAI")
    }

    /// C++ `CvPlot* CvPlayerAI::AI_advancedStartFindCapitalPlot() const` (CvPlayerAI.cpp:17579-17680)
    pub fn player_ai_advanced_start_find_capital_plot(&self, g: &Game, p: PlayerId) -> Option<PlotIdx> {
        todo!("CvPlayerAI::AI_advancedStartFindCapitalPlot")
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartPlaceExploreUnits(bool bLand)` (CvPlayerAI.cpp:17683-17765)
    pub fn player_ai_advanced_start_place_explore_units(&self, g: &mut Game, p: PlayerId, land: bool) -> bool {
        todo!("CvPlayerAI::AI_advancedStartPlaceExploreUnits")
    }

    /// C++ `void CvPlayerAI::AI_advancedStartRevealRadius(CvPlot* pPlot, int iRadius)` (CvPlayerAI.cpp:17767-17790)
    pub fn player_ai_advanced_start_reveal_radius(&self, g: &mut Game, p: PlayerId, plot: PlotIdx, radius: i32) {
        todo!("CvPlayerAI::AI_advancedStartRevealRadius")
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartPlaceCity(CvPlot* pPlot)` (CvPlayerAI.cpp:17792-17899)
    pub fn player_ai_advanced_start_place_city(&self, g: &mut Game, p: PlayerId, plot: PlotIdx) -> bool {
        todo!("CvPlayerAI::AI_advancedStartPlaceCity")
    }

    /// C++ `bool CvPlayerAI::AI_advancedStartDoRoute(CvPlot* pFromPlot, CvPlot* pToPlot)` (CvPlayerAI.cpp:17902-17937)
    pub fn player_ai_advanced_start_do_route(&self, g: &mut Game, p: PlayerId, from_plot: PlotIdx, to_plot: PlotIdx) -> bool {
        todo!("CvPlayerAI::AI_advancedStartDoRoute")
    }

    /// C++ `void CvPlayerAI::AI_advancedStartRouteTerritory()` (CvPlayerAI.cpp:17939-18139)
    pub fn player_ai_advanced_start_route_territory(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_advancedStartRouteTerritory")
    }

    /// C++ `void CvPlayerAI::AI_doAdvancedStart(bool bNoExit = false)` (CvPlayerAI.cpp:18142-18459)
    pub fn player_ai_do_advanced_start(&self, g: &mut Game, p: PlayerId, no_exit: bool) {
        todo!("CvPlayerAI::AI_doAdvancedStart")
    }

    /// C++ `void CvPlayerAI::AI_recalculateFoundValues(int iX, int iY, int iInnerRadius, int iOuterRadius) const` (CvPlayerAI.cpp:18462-18516)
    pub fn player_ai_recalculate_found_values(&self, g: &Game, p: PlayerId, x: i32, y: i32, inner_radius: i32, outer_radius: i32) {
        todo!("CvPlayerAI::AI_recalculateFoundValues")
    }

    /// C++ `int CvPlayerAI::AI_getMinFoundValue() const` (CvPlayerAI.cpp:18519-18545)
    pub fn player_ai_get_min_found_value(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getMinFoundValue")
    }

    /// C++ `void CvPlayerAI::AI_updateCitySites(int iMinFoundValueThreshold, int iMaxSites) const` (CvPlayerAI.cpp:18547-18596)
    pub fn player_ai_update_city_sites(&self, g: &Game, p: PlayerId, min_found_value_threshold: i32, max_sites: i32) {
        todo!("CvPlayerAI::AI_updateCitySites")
    }

    /// C++ `void CvPlayerAI::AI_invalidateCitySites(int iMinFoundValueThreshold) const` (CvPlayerAI.cpp:18598-18601)
    pub fn player_ai_invalidate_city_sites(&self, g: &Game, p: PlayerId, min_found_value_threshold: i32) {
        todo!("CvPlayerAI::AI_invalidateCitySites")
    }

    /// C++ `int CvPlayerAI::AI_getNumCitySites() const` (CvPlayerAI.cpp:18603-18607)
    pub fn player_ai_get_num_city_sites(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getNumCitySites")
    }

    /// C++ `bool CvPlayerAI::AI_isPlotCitySite(CvPlot* pPlot) const` (CvPlayerAI.cpp:18609-18623)
    pub fn player_ai_is_plot_city_site(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
        todo!("CvPlayerAI::AI_isPlotCitySite")
    }

    /// C++ `int CvPlayerAI::AI_getNumAreaCitySites(int iAreaID, int& iBestValue) const` (CvPlayerAI.cpp:18625-18641)
    pub fn player_ai_get_num_area_city_sites(&self, g: &Game, p: PlayerId, area_id: i32, best_value: &mut i32) -> i32 {
        todo!("CvPlayerAI::AI_getNumAreaCitySites")
    }

    /// C++ `int CvPlayerAI::AI_getNumAdjacentAreaCitySites(int iWaterAreaID, int iExcludeArea, int& iBestValue) const` (CvPlayerAI.cpp:18643-18664)
    pub fn player_ai_get_num_adjacent_area_city_sites(&self, g: &Game, p: PlayerId, water_area_id: i32, exclude_area: i32, best_value: &mut i32) -> i32 {
        todo!("CvPlayerAI::AI_getNumAdjacentAreaCitySites")
    }

    /// C++ `CvPlot* CvPlayerAI::AI_getCitySite(int iIndex) const` (CvPlayerAI.cpp:18666-18670)
    pub fn player_ai_get_city_site(&self, g: &Game, p: PlayerId, index: i32) -> Option<PlotIdx> {
        todo!("CvPlayerAI::AI_getCitySite")
    }

    /// C++ `int CvPlayerAI::AI_bestAreaUnitAIValue(UnitAITypes eUnitAI, CvArea* pArea, UnitTypes* peBestUnitType = NULL) const` (CvPlayerAI.cpp:18672-18725)
    pub fn player_ai_best_area_unit_ai_value(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, area: AreaId, best_unit_type: &mut Option<UnitType>) -> i32 {
        todo!("CvPlayerAI::AI_bestAreaUnitAIValue")
    }

    /// C++ `int CvPlayerAI::AI_bestCityUnitAIValue(UnitAITypes eUnitAI, CvCity* pCity, UnitTypes* peBestUnitType = NULL) const` (CvPlayerAI.cpp:18727-18763)
    pub fn player_ai_best_city_unit_ai_value(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, city: CityId, best_unit_type: &mut Option<UnitType>) -> i32 {
        todo!("CvPlayerAI::AI_bestCityUnitAIValue")
    }

    /// C++ `int CvPlayerAI::AI_calculateTotalBombard(DomainTypes eDomain) const` (CvPlayerAI.cpp:18765-18794)
    pub fn player_ai_calculate_total_bombard(&self, g: &Game, p: PlayerId, domain: DomainType) -> i32 {
        todo!("CvPlayerAI::AI_calculateTotalBombard")
    }

    /// C++ `void CvPlayerAI::AI_updateBonusValue(BonusTypes eBonus)` (CvPlayerAI.cpp:18796-18802)
    pub fn player_ai_update_bonus_value(&self, g: &mut Game, p: PlayerId, bonus: BonusType) {
        todo!("CvPlayerAI::AI_updateBonusValue")
    }

    /// C++ `void CvPlayerAI::AI_updateBonusValue()` (CvPlayerAI.cpp:18805-18815)
    pub fn player_ai_update_bonus_value_2(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_updateBonusValue")
    }

    /// C++ `int CvPlayerAI::AI_getUnitClassWeight(UnitClassTypes eUnitClass) const` (CvPlayerAI.cpp:18817-18820)
    pub fn player_ai_get_unit_class_weight(&self, g: &Game, p: PlayerId, unit_class: UnitClassType) -> i32 {
        todo!("CvPlayerAI::AI_getUnitClassWeight")
    }

    /// C++ `int CvPlayerAI::AI_getUnitCombatWeight(UnitCombatTypes eUnitCombat) const` (CvPlayerAI.cpp:18822-18825)
    pub fn player_ai_get_unit_combat_weight(&self, g: &Game, p: PlayerId, unit_combat: UnitCombatType) -> i32 {
        todo!("CvPlayerAI::AI_getUnitCombatWeight")
    }

    /// C++ `void CvPlayerAI::AI_doEnemyUnitData()` (CvPlayerAI.cpp:18827-18970)
    pub fn player_ai_do_enemy_unit_data(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doEnemyUnitData")
    }

    /// C++ `int CvPlayerAI::AI_calculateUnitAIViability(UnitAITypes eUnitAI, DomainTypes eDomain) const` (CvPlayerAI.cpp:18972-18998)
    pub fn player_ai_calculate_unit_ai_viability(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, domain: DomainType) -> i32 {
        todo!("CvPlayerAI::AI_calculateUnitAIViability")
    }

    /// C++ `ReligionTypes CvPlayerAI::AI_chooseReligion()` (CvPlayerAI.cpp:19000-19023)
    pub fn player_ai_choose_religion(&self, g: &Game, p: PlayerId) -> Option<ReligionType> {
        todo!("CvPlayerAI::AI_chooseReligion")
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeWeight(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19025-19048)
    pub fn player_ai_get_attitude_weight(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getAttitudeWeight")
    }

    /// C++ `int CvPlayerAI::AI_getPlotAirbaseValue(CvPlot* pPlot) const` (CvPlayerAI.cpp:19050-19172)
    pub fn player_ai_get_plot_airbase_value(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        todo!("CvPlayerAI::AI_getPlotAirbaseValue")
    }

    /// C++ `int CvPlayerAI::AI_getPlotCanalValue(CvPlot* pPlot) const` (CvPlayerAI.cpp:19174-19226)
    pub fn player_ai_get_plot_canal_value(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
        todo!("CvPlayerAI::AI_getPlotCanalValue")
    }

    /// C++ `int CvPlayerAI::AI_getHappinessWeight(int iHappy, int iExtraPop) const` (CvPlayerAI.cpp:19231-19273)
    pub fn player_ai_get_happiness_weight(&self, g: &Game, p: PlayerId, happy: i32, extra_pop: i32) -> i32 {
        todo!("CvPlayerAI::AI_getHappinessWeight")
    }

    /// C++ `int CvPlayerAI::AI_getHealthWeight(int iHealth, int iExtraPop) const` (CvPlayerAI.cpp:19275-19315)
    pub fn player_ai_get_health_weight(&self, g: &Game, p: PlayerId, health: i32, extra_pop: i32) -> i32 {
        todo!("CvPlayerAI::AI_getHealthWeight")
    }

    /// C++ `void CvPlayerAI::AI_invalidateCloseBordersAttitudeCache()` (CvPlayerAI.cpp:19317-19323)
    pub fn player_ai_invalidate_close_borders_attitude_cache(&self, g: &Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_invalidateCloseBordersAttitudeCache")
    }

    /// C++ `bool CvPlayerAI::AI_isPlotThreatened(CvPlot* pPlot, int iRange = -1, bool bTestMoves = true) const` (CvPlayerAI.cpp:19325-19375)
    pub fn player_ai_is_plot_threatened(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> bool {
        todo!("CvPlayerAI::AI_isPlotThreatened")
    }

    /// C++ `bool CvPlayerAI::AI_isFirstTech(TechTypes eTech) const` (CvPlayerAI.cpp:19377-19400)
    pub fn player_ai_is_first_tech(&self, g: &Game, p: PlayerId, tech: TechType) -> bool {
        todo!("CvPlayerAI::AI_isFirstTech")
    }

    /// C++ `int CvPlayerAI::AI_getAlignmentAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19403-19452)
    pub fn player_ai_get_alignment_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getAlignmentAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getBadBonusAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19454-19465)
    pub fn player_ai_get_bad_bonus_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getBadBonusAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteWonderAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19467-19478)
    pub fn player_ai_get_favorite_wonder_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getFavoriteWonderAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getGenderAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19480-19500)
    pub fn player_ai_get_gender_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getGenderAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getTrustAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19502-19510)
    pub fn player_ai_get_trust_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getTrustAttitude")
    }

    /// C++ `int CvPlayerAI::AI_trueCombatValue(UnitTypes eUnit) const` (CvPlayerAI.cpp:19512-19558)
    pub fn player_ai_true_combat_value(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        todo!("CvPlayerAI::AI_trueCombatValue")
    }

    /// C++ `int CvPlayerAI::AI_combatValue(UnitTypes eUnit) const` (CvPlayerAI.cpp:19560-19567)
    pub fn player_ai_combat_value(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        todo!("CvPlayerAI::AI_combatValue")
    }

    /// C++ `int CvPlayerAI::AI_getCivicShareAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:19569-19580)
    pub fn player_ai_get_civic_share_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getCivicShareAttitude")
    }

    /// C++ `CvPlot* CvPlayerAI::AI_getExpansionPlot() const` (CvPlayerAI.cpp:19583-19593)
    pub fn player_ai_get_expansion_plot(&self, g: &Game, p: PlayerId) -> Option<PlotIdx> {
        todo!("CvPlayerAI::AI_getExpansionPlot")
    }

    /// C++ `void CvPlayerAI::AI_setExpansionPlot(CvPlot* newplot)` (CvPlayerAI.cpp:19595-19607)
    pub fn player_ai_set_expansion_plot(&self, g: &mut Game, p: PlayerId, newplot: PlotIdx) {
        todo!("CvPlayerAI::AI_setExpansionPlot")
    }

    /// C++ `void CvPlayerAI::AI_doExpansion()` (CvPlayerAI.cpp:19609-19674)
    pub fn player_ai_do_expansion(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doExpansion")
    }

}
