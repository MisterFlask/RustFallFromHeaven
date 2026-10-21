//! Port of CvCityAI.cpp:2749-5256 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `BuildingTypes CvCityAI::AI_bestBuilding(int iFocusFlags = 0, int iMaxTurns = 0, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2749-2752)
    pub fn city_ai_best_building(&self, g: &Game, c: CityId, focus_flags: i32, max_turns: i32, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<BuildingType> {
        todo!("CvCityAI::AI_bestBuilding")
    }

    /// C++ `BuildingTypes CvCityAI::AI_bestBuildingThreshold(int iFocusFlags = 0, int iMaxTurns = 0, int iMinThreshold = 0, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2754-2928)
    pub fn city_ai_best_building_threshold(&self, g: &Game, c: CityId, focus_flags: i32, max_turns: i32, min_threshold: i32, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<BuildingType> {
        todo!("CvCityAI::AI_bestBuildingThreshold")
    }

    /// C++ `int CvCityAI::AI_buildingValue(BuildingTypes eBuilding, int iFocusFlags = 0)` (CvCityAI.cpp:2931-2934)
    pub fn city_ai_building_value(&self, g: &Game, c: CityId, building: BuildingType, focus_flags: i32) -> i32 {
        todo!("CvCityAI::AI_buildingValue")
    }

    /// C++ `int CvCityAI::AI_buildingValueThreshold(BuildingTypes eBuilding, int iFocusFlags = 0, int iThreshold = 0)` (CvCityAI.cpp:2938-4384)
    pub fn city_ai_building_value_threshold(&self, g: &Game, c: CityId, building: BuildingType, focus_flags: i32, threshold: i32) -> i32 {
        todo!("CvCityAI::AI_buildingValueThreshold")
    }

    /// C++ `ProjectTypes CvCityAI::AI_bestProject()` (CvCityAI.cpp:4387-4468)
    pub fn city_ai_best_project(&self, g: &Game, c: CityId) -> Option<ProjectType> {
        todo!("CvCityAI::AI_bestProject")
    }

    /// C++ `int CvCityAI::AI_projectValue(ProjectTypes eProject)` (CvCityAI.cpp:4471-4512)
    pub fn city_ai_project_value(&self, g: &Game, c: CityId, project: ProjectType) -> i32 {
        todo!("CvCityAI::AI_projectValue")
    }

    /// C++ `ProcessTypes CvCityAI::AI_bestProcess()` (CvCityAI.cpp:4515-4518)
    pub fn city_ai_best_process(&self, g: &Game, c: CityId) -> Option<ProcessType> {
        todo!("CvCityAI::AI_bestProcess")
    }

    /// C++ `ProcessTypes CvCityAI::AI_bestProcess(CommerceTypes eCommerceType)` (CvCityAI.cpp:4520-4545)
    pub fn city_ai_best_process_2(&self, g: &Game, c: CityId, commerce_type: CommerceType) -> Option<ProcessType> {
        todo!("CvCityAI::AI_bestProcess")
    }

    /// C++ `int CvCityAI::AI_processValue(ProcessTypes eProcess)` (CvCityAI.cpp:4548-4551)
    pub fn city_ai_process_value(&self, g: &Game, c: CityId, process: ProcessType) -> i32 {
        todo!("CvCityAI::AI_processValue")
    }

    /// C++ `int CvCityAI::AI_processValue(ProcessTypes eProcess, CommerceTypes eCommerceType)` (CvCityAI.cpp:4553-4590)
    pub fn city_ai_process_value_2(&self, g: &Game, c: CityId, process: ProcessType, commerce_type: CommerceType) -> i32 {
        todo!("CvCityAI::AI_processValue")
    }

    /// C++ `int CvCityAI::AI_neededSeaWorkers()` (CvCityAI.cpp:4593-4657)
    pub fn city_ai_needed_sea_workers(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_neededSeaWorkers")
    }

    /// C++ `bool CvCityAI::AI_isDefended(int iExtra = 0)` (CvCityAI.cpp:4660-4665)
    pub fn city_ai_is_defended(&self, g: &Game, c: CityId, extra: i32) -> bool {
        todo!("CvCityAI::AI_isDefended")
    }

    /// C++ `bool CvCityAI::AI_isAirDefended(int iExtra = 0)` (CvCityAI.cpp:4668-4673)
    pub fn city_ai_is_air_defended(&self, g: &Game, c: CityId, extra: i32) -> bool {
        todo!("CvCityAI::AI_isAirDefended")
    }

    /// C++ `int CvCityAI::AI_neededDefenders()` (CvCityAI.cpp:4676-4755)
    pub fn city_ai_needed_defenders(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_neededDefenders")
    }

    /// C++ `int CvCityAI::AI_minDefenders()` (CvCityAI.cpp:4757-4771)
    pub fn city_ai_min_defenders(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_minDefenders")
    }

    /// C++ `int CvCityAI::AI_neededFloatingDefenders()` (CvCityAI.cpp:4773-4780)
    pub fn city_ai_needed_floating_defenders(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_neededFloatingDefenders")
    }

    /// C++ `void CvCityAI::AI_updateNeededFloatingDefenders()` (CvCityAI.cpp:4782-4796)
    pub fn city_ai_update_needed_floating_defenders(&self, g: &Game, c: CityId) {
        todo!("CvCityAI::AI_updateNeededFloatingDefenders")
    }

    /// C++ `int CvCityAI::AI_neededAirDefenders()` (CvCityAI.cpp:4798-4846)
    pub fn city_ai_needed_air_defenders(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_neededAirDefenders")
    }

    /// C++ `bool CvCityAI::AI_isDanger()` (CvCityAI.cpp:4849-4852)
    pub fn city_ai_is_danger(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isDanger")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidGrowthCount()` (CvCityAI.cpp:4855-4858)
    pub fn city_ai_get_emphasize_avoid_growth_count(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeAvoidGrowthCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidGrowth()` (CvCityAI.cpp:4861-4864)
    pub fn city_ai_is_emphasize_avoid_growth(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isEmphasizeAvoidGrowth")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidAngryCitizensCount()` (CvCityAI.cpp:4871-4874)
    pub fn city_ai_get_emphasize_avoid_angry_citizens_count(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeAvoidAngryCitizensCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidAngryCitizens()` (CvCityAI.cpp:4876-4879)
    pub fn city_ai_is_emphasize_avoid_angry_citizens(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isEmphasizeAvoidAngryCitizens")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeAvoidUnhealthyCitizensCount()` (CvCityAI.cpp:4881-4884)
    pub fn city_ai_get_emphasize_avoid_unhealthy_citizens_count(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeAvoidUnhealthyCitizensCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeAvoidUnhealthyCitizens()` (CvCityAI.cpp:4886-4889)
    pub fn city_ai_is_emphasize_avoid_unhealthy_citizens(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isEmphasizeAvoidUnhealthyCitizens")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeGreatPeopleCount()` (CvCityAI.cpp:4894-4897)
    pub fn city_ai_get_emphasize_great_people_count(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeGreatPeopleCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeGreatPeople()` (CvCityAI.cpp:4900-4903)
    pub fn city_ai_is_emphasize_great_people(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isEmphasizeGreatPeople")
    }

    /// C++ `bool CvCityAI::AI_isAssignWorkDirty()` (CvCityAI.cpp:4906-4909)
    pub fn city_ai_is_assign_work_dirty(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isAssignWorkDirty")
    }

    /// C++ `void CvCityAI::AI_setAssignWorkDirty(bool bNewValue)` (CvCityAI.cpp:4912-4915)
    pub fn city_ai_set_assign_work_dirty(&self, g: &mut Game, c: CityId, new_value: bool) {
        todo!("CvCityAI::AI_setAssignWorkDirty")
    }

    /// C++ `bool CvCityAI::AI_isChooseProductionDirty()` (CvCityAI.cpp:4918-4921)
    pub fn city_ai_is_choose_production_dirty(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_isChooseProductionDirty")
    }

    /// C++ `void CvCityAI::AI_setChooseProductionDirty(bool bNewValue)` (CvCityAI.cpp:4924-4927)
    pub fn city_ai_set_choose_production_dirty(&self, g: &mut Game, c: CityId, new_value: bool) {
        todo!("CvCityAI::AI_setChooseProductionDirty")
    }

    /// C++ `CvCity* CvCityAI::AI_getRouteToCity() const` (CvCityAI.cpp:4930-4933)
    pub fn city_ai_get_route_to_city(&self, g: &Game, c: CityId) -> Option<CityId> {
        todo!("CvCityAI::AI_getRouteToCity")
    }

    /// C++ `void CvCityAI::AI_updateRouteToCity()` (CvCityAI.cpp:4936-4984)
    pub fn city_ai_update_route_to_city(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_updateRouteToCity")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeYieldCount(YieldTypes eIndex)` (CvCityAI.cpp:4987-4992)
    pub fn city_ai_get_emphasize_yield_count(&self, g: &Game, c: CityId, index: YieldType) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeYieldCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeYield(YieldTypes eIndex)` (CvCityAI.cpp:4995-4998)
    pub fn city_ai_is_emphasize_yield(&self, g: &Game, c: CityId, index: YieldType) -> bool {
        todo!("CvCityAI::AI_isEmphasizeYield")
    }

    /// C++ `int CvCityAI::AI_getEmphasizeCommerceCount(CommerceTypes eIndex)` (CvCityAI.cpp:5001-5006)
    pub fn city_ai_get_emphasize_commerce_count(&self, g: &Game, c: CityId, index: CommerceType) -> i32 {
        todo!("CvCityAI::AI_getEmphasizeCommerceCount")
    }

    /// C++ `bool CvCityAI::AI_isEmphasizeCommerce(CommerceTypes eIndex)` (CvCityAI.cpp:5009-5012)
    pub fn city_ai_is_emphasize_commerce(&self, g: &Game, c: CityId, index: CommerceType) -> bool {
        todo!("CvCityAI::AI_isEmphasizeCommerce")
    }

    /// C++ `bool CvCityAI::AI_isEmphasize(EmphasizeTypes eIndex)` (CvCityAI.cpp:5015-5021)
    pub fn city_ai_is_emphasize(&self, g: &Game, c: CityId, index: EmphasizeType) -> bool {
        todo!("CvCityAI::AI_isEmphasize")
    }

    /// C++ `void CvCityAI::AI_setEmphasize(EmphasizeTypes eIndex, bool bNewValue)` (CvCityAI.cpp:5024-5091)
    pub fn city_ai_set_emphasize(&self, g: &mut Game, c: CityId, index: EmphasizeType, new_value: bool) {
        todo!("CvCityAI::AI_setEmphasize")
    }

    /// C++ `void CvCityAI::AI_forceEmphasizeCulture(bool bNewValue)` (CvCityAI.cpp:5093-5102)
    pub fn city_ai_force_emphasize_culture(&self, g: &mut Game, c: CityId, new_value: bool) {
        todo!("CvCityAI::AI_forceEmphasizeCulture")
    }

    /// C++ `int CvCityAI::AI_getBestBuildValue(int iIndex)` (CvCityAI.cpp:5105-5110)
    pub fn city_ai_get_best_build_value(&self, g: &Game, c: CityId, index: i32) -> i32 {
        todo!("CvCityAI::AI_getBestBuildValue")
    }

    /// C++ `int CvCityAI::AI_totalBestBuildValue(CvArea* pArea)` (CvCityAI.cpp:5113-5145)
    pub fn city_ai_total_best_build_value(&self, g: &Game, c: CityId, area: AreaId) -> i32 {
        todo!("CvCityAI::AI_totalBestBuildValue")
    }

    /// C++ `int CvCityAI::AI_clearFeatureValue(int iIndex)` (CvCityAI.cpp:5147-5214)
    pub fn city_ai_clear_feature_value(&self, g: &Game, c: CityId, index: i32) -> i32 {
        todo!("CvCityAI::AI_clearFeatureValue")
    }

    /// C++ `BuildTypes CvCityAI::AI_getBestBuild(int iIndex)` (CvCityAI.cpp:5216-5221)
    pub fn city_ai_get_best_build(&self, g: &Game, c: CityId, index: i32) -> Option<BuildType> {
        todo!("CvCityAI::AI_getBestBuild")
    }

    /// C++ `int CvCityAI::AI_countBestBuilds(CvArea* pArea)` (CvCityAI.cpp:5224-5256)
    pub fn city_ai_count_best_builds(&self, g: &Game, c: CityId, area: AreaId) -> i32 {
        todo!("CvCityAI::AI_countBestBuilds")
    }

}
