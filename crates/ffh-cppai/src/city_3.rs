//! Port of CvCityAI.cpp:5259-7883 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvCityAI::AI_updateBestBuild()` (CvCityAI.cpp:5260-5754)
    pub fn city_ai_update_best_build(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_updateBestBuild")
    }

    /// C++ `void CvCityAI::AI_doDraft(bool bForce = false)` (CvCityAI.cpp:5759-5814)
    pub fn city_ai_do_draft(&self, g: &mut Game, c: CityId, force: bool) {
        todo!("CvCityAI::AI_doDraft")
    }

    /// C++ `void CvCityAI::AI_doHurry(bool bForce = false)` (CvCityAI.cpp:5817-6306)
    pub fn city_ai_do_hurry(&self, g: &mut Game, c: CityId, force: bool) {
        todo!("CvCityAI::AI_doHurry")
    }

    /// C++ `void CvCityAI::AI_doEmphasize()` (CvCityAI.cpp:6310-6435)
    pub fn city_ai_do_emphasize(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_doEmphasize")
    }

    /// C++ `bool CvCityAI::AI_chooseUnit(UnitAITypes eUnitAI = NO_UNITAI)` (CvCityAI.cpp:6438-6458)
    pub fn city_ai_choose_unit(&self, g: &mut Game, c: CityId, unit_ai: Option<UnitAIType>) -> bool {
        todo!("CvCityAI::AI_chooseUnit")
    }

    /// C++ `bool CvCityAI::AI_chooseUnit(UnitTypes eUnit, UnitAITypes eUnitAI)` (CvCityAI.cpp:6460-6469)
    pub fn city_ai_choose_unit_2(&self, g: &mut Game, c: CityId, unit: UnitType, unit_ai: UnitAIType) -> bool {
        todo!("CvCityAI::AI_chooseUnit")
    }

    /// C++ `bool CvCityAI::AI_chooseDefender()` (CvCityAI.cpp:6472-6496)
    pub fn city_ai_choose_defender(&self, g: &mut Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_chooseDefender")
    }

    /// C++ `bool CvCityAI::AI_chooseLeastRepresentedUnit(UnitTypeWeightArray & allowedTypes)` (CvCityAI.cpp:6498-6524)
    pub fn city_ai_choose_least_represented_unit(&self, g: &mut Game, c: CityId, allowed_types: &mut Vec<(UnitAIType, i32)>) -> bool {
        todo!("CvCityAI::AI_chooseLeastRepresentedUnit")
    }

    /// C++ `bool CvCityAI::AI_bestSpreadUnit(bool bMissionary, bool bExecutive, int iBaseChance, UnitTypes* eBestSpreadUnit, int* iBestSpreadUnitValue)` (CvCityAI.cpp:6526-6682)
    pub fn city_ai_best_spread_unit(&self, g: &Game, c: CityId, missionary: bool, executive: bool, base_chance: i32, best_spread_unit: &mut Option<UnitType>, best_spread_unit_value: &mut i32) -> bool {
        todo!("CvCityAI::AI_bestSpreadUnit")
    }

    /// C++ `bool CvCityAI::AI_chooseBuilding(int iFocusFlags = 0, int iMaxTurns = MAX_INT, int iMinThreshold = 0)` (CvCityAI.cpp:6684-6697)
    pub fn city_ai_choose_building(&self, g: &mut Game, c: CityId, focus_flags: i32, max_turns: i32, min_threshold: i32) -> bool {
        todo!("CvCityAI::AI_chooseBuilding")
    }

    /// C++ `bool CvCityAI::AI_chooseProject()` (CvCityAI.cpp:6700-6713)
    pub fn city_ai_choose_project(&self, g: &mut Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_chooseProject")
    }

    /// C++ `bool CvCityAI::AI_chooseProcess(CommerceTypes eCommerceType = NO_COMMERCE)` (CvCityAI.cpp:6716-6729)
    pub fn city_ai_choose_process(&self, g: &mut Game, c: CityId, commerce_type: Option<CommerceType>) -> bool {
        todo!("CvCityAI::AI_chooseProcess")
    }

    /// C++ `bool CvCityAI::AI_addBestCitizen(bool bWorkers, bool bSpecialists, int* piBestPlot = NULL, SpecialistTypes* peBestSpecialist = NULL)` (CvCityAI.cpp:6733-6878)
    pub fn city_ai_add_best_citizen(&self, g: &mut Game, c: CityId, workers: bool, specialists: bool, best_plot: &mut i32, best_specialist: &mut Option<SpecialistType>) -> bool {
        todo!("CvCityAI::AI_addBestCitizen")
    }

    /// C++ `bool CvCityAI::AI_removeWorstCitizen(SpecialistTypes eIgnoreSpecialist = NO_SPECIALIST)` (CvCityAI.cpp:6882-7003)
    pub fn city_ai_remove_worst_citizen(&self, g: &mut Game, c: CityId, ignore_specialist: Option<SpecialistType>) -> bool {
        todo!("CvCityAI::AI_removeWorstCitizen")
    }

    /// C++ `void CvCityAI::AI_juggleCitizens()` (CvCityAI.cpp:7006-7084)
    pub fn city_ai_juggle_citizens(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_juggleCitizens")
    }

    /// C++ `bool CvCityAI::AI_potentialPlot(short* piYields)` (CvCityAI.cpp:7087-7110)
    pub fn city_ai_potential_plot(&self, g: &Game, c: CityId, yields: &[i32]) -> bool {
        todo!("CvCityAI::AI_potentialPlot")
    }

    /// C++ `bool CvCityAI::AI_foodAvailable(int iExtra = 0)` (CvCityAI.cpp:7113-7208)
    pub fn city_ai_food_available(&self, g: &Game, c: CityId, extra: i32) -> bool {
        todo!("CvCityAI::AI_foodAvailable")
    }

    /// C++ `int CvCityAI::AI_yieldValue(short* piYields, short* piCommerceYields, bool bAvoidGrowth, bool bRemove, bool bIgnoreFood = false, bool bIgnoreGrowth = false, bool bIgnoreStarvation = false, bool bWorkerOptimization = false)` (CvCityAI.cpp:7211-7778)
    pub fn city_ai_yield_value(&self, g: &Game, c: CityId, yields: &[i32], commerce_yields: &[i32], avoid_growth: bool, remove: bool, ignore_food: bool, ignore_growth: bool, ignore_starvation: bool, worker_optimization: bool) -> i32 {
        todo!("CvCityAI::AI_yieldValue")
    }

    /// C++ `int CvCityAI::AI_plotValue(CvPlot* pPlot, bool bAvoidGrowth, bool bRemove, bool bIgnoreFood = false, bool bIgnoreGrowth = false, bool bIgnoreStarvation = false)` (CvCityAI.cpp:7781-7862)
    pub fn city_ai_plot_value(&self, g: &Game, c: CityId, plot: PlotIdx, avoid_growth: bool, remove: bool, ignore_food: bool, ignore_growth: bool, ignore_starvation: bool) -> i32 {
        todo!("CvCityAI::AI_plotValue")
    }

    /// C++ `int CvCityAI::AI_experienceWeight()` (CvCityAI.cpp:7865-7868)
    pub fn city_ai_experience_weight(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_experienceWeight")
    }

    /// C++ `int CvCityAI::AI_buildUnitProb()` (CvCityAI.cpp:7871-7883)
    pub fn city_ai_build_unit_prob(&self, g: &Game, c: CityId) -> i32 {
        todo!("CvCityAI::AI_buildUnitProb")
    }

}
