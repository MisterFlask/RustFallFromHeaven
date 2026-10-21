//! Port of CvCityAI.cpp:7887-10755 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvCityAI::AI_bestPlotBuild(CvPlot* pPlot, int* piBestValue, BuildTypes* peBestBuild, int iFoodPriority, int iProductionPriority, int iCommercePriority, bool bChop, int iHappyAdjust, int iHealthAdjust, int iFoodChange)` (CvCityAI.cpp:7888-8705)
    pub fn city_ai_best_plot_build(&self, g: &Game, c: CityId, plot: PlotIdx, best_value: &mut i32, best_build: &mut Option<BuildType>, food_priority: i32, production_priority: i32, commerce_priority: i32, chop: bool, happy_adjust: i32, health_adjust: i32, food_change: i32) {
        todo!("CvCityAI::AI_bestPlotBuild")
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry)` (CvCityAI.cpp:8707-8710)
    pub fn city_ai_get_happy_from_hurry(&self, g: &Game, c: CityId, hurry: HurryType) -> i32 {
        todo!("CvCityAI::AI_getHappyFromHurry")
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(int iHurryPopulation)` (CvCityAI.cpp:8712-8727)
    pub fn city_ai_get_happy_from_hurry_2(&self, g: &Game, c: CityId, hurry_population: i32) -> i32 {
        todo!("CvCityAI::AI_getHappyFromHurry")
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry, UnitTypes eUnit, bool bIgnoreNew)` (CvCityAI.cpp:8729-8732)
    pub fn city_ai_get_happy_from_hurry_3(&self, g: &Game, c: CityId, hurry: HurryType, unit: UnitType, ignore_new: bool) -> i32 {
        todo!("CvCityAI::AI_getHappyFromHurry")
    }

    /// C++ `int CvCityAI::AI_getHappyFromHurry(HurryTypes eHurry, BuildingTypes eBuilding, bool bIgnoreNew)` (CvCityAI.cpp:8734-8737)
    pub fn city_ai_get_happy_from_hurry_4(&self, g: &Game, c: CityId, hurry: HurryType, building: BuildingType, ignore_new: bool) -> i32 {
        todo!("CvCityAI::AI_getHappyFromHurry")
    }

    /// C++ `int CvCityAI::AI_cityValue() const` (CvCityAI.cpp:8740-8758)
    pub fn city_ai_city_value(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_cityValue")
    }

    /// C++ `bool CvCityAI::AI_doPanic()` (CvCityAI.cpp:8760-8848)
    pub fn city_ai_do_panic(&self, g: &mut Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_doPanic")
    }

    /// C++ `int CvCityAI::AI_calculateCulturePressure(bool bGreatWork = false)` (CvCityAI.cpp:8850-8929)
    pub fn city_ai_calculate_culture_pressure(&self, g: &Game, c: CityId, great_work: bool) -> i32 {
        todo!("CvCityAI::AI_calculateCulturePressure")
    }

    /// C++ `void CvCityAI::AI_buildGovernorChooseProduction()` (CvCityAI.cpp:8932-9103)
    pub fn city_ai_build_governor_choose_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_buildGovernorChooseProduction")
    }

    /// C++ `int CvCityAI::AI_calculateWaterWorldPercent()` (CvCityAI.cpp:9105-9145)
    pub fn city_ai_calculate_water_world_percent(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_calculateWaterWorldPercent")
    }

    /// C++ `int CvCityAI::AI_getYieldMagicValue(const int* piYieldsTimes100, bool bHealthy)` (CvCityAI.cpp:9148-9159)
    pub fn city_ai_get_yield_magic_value(&self, g: &Game, c: CityId, yields_times100: &[i32], healthy: bool) -> i32 {
        todo!("CvCityAI::AI_getYieldMagicValue")
    }

    /// C++ `int CvCityAI::AI_getPlotMagicValue(CvPlot* pPlot, bool bHealthy, bool bWorkerOptimization = false)` (CvCityAI.cpp:9168-9208)
    pub fn city_ai_get_plot_magic_value(&self, g: &Game, c: CityId, plot: PlotIdx, healthy: bool, worker_optimization: bool) -> i32 {
        todo!("CvCityAI::AI_getPlotMagicValue")
    }

    /// C++ `int CvCityAI::AI_countGoodTiles(bool bHealthy, bool bUnworkedOnly, int iThreshold = 50, bool bWorkerOptimization = false)` (CvCityAI.cpp:9213-9238)
    pub fn city_ai_count_good_tiles(&self, g: &Game, c: CityId, healthy: bool, unworked_only: bool, threshold: i32, worker_optimization: bool) -> i32 {
        todo!("CvCityAI::AI_countGoodTiles")
    }

    /// C++ `int CvCityAI::AI_calculateTargetCulturePerTurn()` (CvCityAI.cpp:9240-9279)
    pub fn city_ai_calculate_target_culture_per_turn(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_calculateTargetCulturePerTurn")
    }

    /// C++ `int CvCityAI::AI_countGoodSpecialists(bool bHealthy)` (CvCityAI.cpp:9281-9309)
    pub fn city_ai_count_good_specialists(&self, g: &Game, c: CityId, healthy: bool) -> i32 {
        todo!("CvCityAI::AI_countGoodSpecialists")
    }

    /// C++ `int CvCityAI::AI_getCityImportance(bool bEconomy, bool bMilitary)` (CvCityAI.cpp:9312-9336)
    pub fn city_ai_get_city_importance(&self, g: &Game, c: CityId, economy: bool, military: bool) -> i32 {
        todo!("CvCityAI::AI_getCityImportance")
    }

    /// C++ `void CvCityAI::AI_stealPlots()` (CvCityAI.cpp:9338-9397)
    pub fn city_ai_steal_plots(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_stealPlots")
    }

    /// C++ `int CvCityAI::AI_buildingSpecialYieldChangeValue(BuildingTypes eBuilding, YieldTypes eYield)` (CvCityAI.cpp:9407-9454)
    pub fn city_ai_building_special_yield_change_value(&self, g: &Game, c: CityId, building: BuildingType, yield_: YieldType) -> i32 {
        todo!("CvCityAI::AI_buildingSpecialYieldChangeValue")
    }

    /// C++ `int CvCityAI::AI_yieldMultiplier(YieldTypes eYield)` (CvCityAI.cpp:9457-9476)
    pub fn city_ai_yield_multiplier(&self, g: &Game, c: CityId, yield_: YieldType) -> i32 {
        todo!("CvCityAI::AI_yieldMultiplier")
    }

    /// C++ `void CvCityAI::AI_updateSpecialYieldMultiplier()` (CvCityAI.cpp:9481-9584)
    pub fn city_ai_update_special_yield_multiplier(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_updateSpecialYieldMultiplier")
    }

    /// C++ `int CvCityAI::AI_specialYieldMultiplier(YieldTypes eYield)` (CvCityAI.cpp:9586-9589)
    pub fn city_ai_special_yield_multiplier(&self, g: &Game, c: CityId, yield_: YieldType) -> i32 {
        todo!("CvCityAI::AI_specialYieldMultiplier")
    }

    /// C++ `int CvCityAI::AI_countNumBonuses(BonusTypes eBonus, bool bIncludeOurs, bool bIncludeNeutral, int iOtherCultureThreshold, bool bLand = true, bool bWater = true)` (CvCityAI.cpp:9592-9635)
    pub fn city_ai_count_num_bonuses(&self, g: &Game, c: CityId, bonus: BonusType, include_ours: bool, include_neutral: bool, other_culture_threshold: i32, land: bool, water: bool) -> i32 {
        todo!("CvCityAI::AI_countNumBonuses")
    }

    /// C++ `int CvCityAI::AI_playerCloseness(PlayerTypes eIndex, int iMaxDistance)` (CvCityAI.cpp:9637-9649)
    pub fn city_ai_player_closeness(&self, g: &Game, c: CityId, index: PlayerId, max_distance: i32) -> i32 {
        todo!("CvCityAI::AI_playerCloseness")
    }

    /// C++ `void CvCityAI::AI_cachePlayerCloseness(int iMaxDistance)` (CvCityAI.cpp:9651-9728)
    pub fn city_ai_cache_player_closeness(&self, g: &Game, c: CityId, max_distance: i32) {
        todo!("CvCityAI::AI_cachePlayerCloseness")
    }

    /// C++ `int CvCityAI::AI_cityThreat(bool bDangerPercent = false)` (CvCityAI.cpp:9730-9814)
    pub fn city_ai_city_threat(&self, g: &Game, c: CityId, danger_percent: bool) -> i32 {
        todo!("CvCityAI::AI_cityThreat")
    }

    /// C++ `int CvCityAI::AI_getWorkersHave()` (CvCityAI.cpp:9820-9823)
    pub fn city_ai_get_workers_have(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getWorkersHave")
    }

    /// C++ `int CvCityAI::AI_getWorkersNeeded()` (CvCityAI.cpp:9825-9828)
    pub fn city_ai_get_workers_needed(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getWorkersNeeded")
    }

    /// C++ `void CvCityAI::AI_changeWorkersHave(int iChange)` (CvCityAI.cpp:9830-9835)
    pub fn city_ai_change_workers_have(&self, g: &mut Game, c: CityId, change: i32) {
        todo!("CvCityAI::AI_changeWorkersHave")
    }

    /// C++ `void CvCityAI::AI_updateWorkersNeededHere()` (CvCityAI.cpp:9838-10029)
    pub fn city_ai_update_workers_needed_here(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_updateWorkersNeededHere")
    }

    /// C++ `BuildingTypes CvCityAI::AI_bestAdvancedStartBuilding(int iPass)` (CvCityAI.cpp:10031-10060)
    pub fn city_ai_best_advanced_start_building(&self, g: &Game, c: CityId, pass: i32) -> Option<BuildingType> {
        todo!("CvCityAI::AI_bestAdvancedStartBuilding")
    }

    /// C++ `void CvCityAI::AI_calculateNeededPermDefense()` (CvCityAI.cpp:10068-10146)
    pub fn city_ai_calculate_needed_perm_defense(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_calculateNeededPermDefense")
    }

    /// C++ `void CvCityAI::AI_calculateNeededPatrol()` (CvCityAI.cpp:10149-10211)
    pub fn city_ai_calculate_needed_patrol(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_calculateNeededPatrol")
    }

    /// C++ `void CvCityAI::AI_calculateNeededCityDefenseProduction()` (CvCityAI.cpp:10214-10340)
    pub fn city_ai_calculate_needed_city_defense_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_calculateNeededCityDefenseProduction")
    }

    /// C++ `void CvCityAI::AI_calculateNeededCityPatrolProduction()` (CvCityAI.cpp:10343-10454)
    pub fn city_ai_calculate_needed_city_patrol_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_calculateNeededCityPatrolProduction")
    }

    /// C++ `int CvCityAI::AI_neededPermDefense(int flag)` (CvCityAI.cpp:10458-10464)
    pub fn city_ai_needed_perm_defense(&self, g: &Game, c: CityId, flag: i32) -> i32 {
        todo!("CvCityAI::AI_neededPermDefense")
    }

    /// C++ `void CvCityAI::AI_setCityPatrolOrigin(CvPlot* newpatrolorigin)` (CvCityAI.cpp:10466-10478)
    pub fn city_ai_set_city_patrol_origin(&self, g: &mut Game, c: CityId, newpatrolorigin: PlotIdx) {
        todo!("CvCityAI::AI_setCityPatrolOrigin")
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityPatrolOrigin()` (CvCityAI.cpp:10480-10490)
    pub fn city_ai_get_city_patrol_origin(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        todo!("CvCityAI::AI_getCityPatrolOrigin")
    }

    /// C++ `int CvCityAI::AI_neededPatrol(int flag)` (CvCityAI.cpp:10492-10498)
    pub fn city_ai_needed_patrol(&self, g: &Game, c: CityId, flag: i32) -> i32 {
        todo!("CvCityAI::AI_neededPatrol")
    }

    /// C++ `int CvCityAI::AI_CityDangerValue()` (CvCityAI.cpp:10500-10561)
    pub fn city_ai_city_danger_value(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_CityDangerValue")
    }

    /// C++ `void CvCityAI::AI_setCityDefenseProduction(CvPlot* newplot)` (CvCityAI.cpp:10563-10575)
    pub fn city_ai_set_city_defense_production(&self, g: &mut Game, c: CityId, newplot: PlotIdx) {
        todo!("CvCityAI::AI_setCityDefenseProduction")
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityDefenseProduction()` (CvCityAI.cpp:10577-10587)
    pub fn city_ai_get_city_defense_production(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        todo!("CvCityAI::AI_getCityDefenseProduction")
    }

    /// C++ `void CvCityAI::AI_CityChooseDefenseProduction()` (CvCityAI.cpp:10589-10640)
    pub fn city_ai_city_choose_defense_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_CityChooseDefenseProduction")
    }

    /// C++ `int CvCityAI::AI_neededCityDefenseProduction(int type)` (CvCityAI.cpp:10642-10645)
    pub fn city_ai_needed_city_defense_production(&self, g: &Game, c: CityId, type_: i32) -> i32 {
        todo!("CvCityAI::AI_neededCityDefenseProduction")
    }

    /// C++ `bool CvCityAI::AI_isCityDefenseProduction()` (CvCityAI.cpp:10647-10659)
    pub fn city_ai_is_city_defense_production(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isCityDefenseProduction")
    }

    /// C++ `void CvCityAI::AI_setCityPatrolProduction(CvPlot* newplot)` (CvCityAI.cpp:10661-10673)
    pub fn city_ai_set_city_patrol_production(&self, g: &mut Game, c: CityId, newplot: PlotIdx) {
        todo!("CvCityAI::AI_setCityPatrolProduction")
    }

    /// C++ `CvPlot* CvCityAI::AI_getCityPatrolProduction()` (CvCityAI.cpp:10675-10685)
    pub fn city_ai_get_city_patrol_production(&self, g: &Game, c: CityId) -> Option<PlotIdx> {
        todo!("CvCityAI::AI_getCityPatrolProduction")
    }

    /// C++ `void CvCityAI::AI_CityChoosePatrolProduction()` (CvCityAI.cpp:10688-10735)
    pub fn city_ai_city_choose_patrol_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_CityChoosePatrolProduction")
    }

    /// C++ `int CvCityAI::AI_neededCityPatrolProduction(int type)` (CvCityAI.cpp:10738-10741)
    pub fn city_ai_needed_city_patrol_production(&self, g: &Game, c: CityId, type_: i32) -> i32 {
        todo!("CvCityAI::AI_neededCityPatrolProduction")
    }

    /// C++ `bool CvCityAI::AI_isCityPatrolProduction()` (CvCityAI.cpp:10743-10755)
    pub fn city_ai_is_city_patrol_production(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isCityPatrolProduction")
    }

}
