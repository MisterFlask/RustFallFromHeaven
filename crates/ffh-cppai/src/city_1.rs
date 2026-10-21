//! Port of CvCityAI.cpp:69-2746 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvCityAI::AI_init()` (CvCityAI.cpp:69-80)
    pub fn city_ai_init(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_init")
    }

    /// C++ `void CvCityAI::AI_reset()` (CvCityAI.cpp:91-190)
    pub fn city_ai_reset(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_reset")
    }

    /// C++ `void CvCityAI::AI_doTurn()` (CvCityAI.cpp:193-255)
    pub fn city_ai_do_turn(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_doTurn")
    }

    /// C++ `void CvCityAI::AI_assignWorkingPlots()` (CvCityAI.cpp:258-416)
    pub fn city_ai_assign_working_plots(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_assignWorkingPlots")
    }

    /// C++ `void CvCityAI::AI_updateAssignWork()` (CvCityAI.cpp:419-425)
    pub fn city_ai_update_assign_work(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_updateAssignWork")
    }

    /// C++ `bool CvCityAI::AI_stopGrowth()` (CvCityAI.cpp:432-486)
    pub fn city_ai_stop_growth(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_stopGrowth")
    }

    /// C++ `bool CvCityAI::AI_avoidGrowth()` (CvCityAI.cpp:491-552)
    pub fn city_ai_avoid_growth(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_avoidGrowth")
    }

    /// C++ `bool CvCityAI::AI_ignoreGrowth()` (CvCityAI.cpp:555-568)
    pub fn city_ai_ignore_growth(&self, g: &Game, c: CityId) -> bool {
        todo!("CvCityAI::AI_ignoreGrowth")
    }

    /// C++ `int CvCityAI::AI_specialistValue(SpecialistTypes eSpecialist, bool bAvoidGrowth, bool bRemove)` (CvCityAI.cpp:571-743)
    pub fn city_ai_specialist_value(&self, g: &Game, c: CityId, specialist: SpecialistType, avoid_growth: bool, remove: bool) -> i32 {
        todo!("CvCityAI::AI_specialistValue")
    }

    /// C++ `void CvCityAI::AI_chooseProduction()` (CvCityAI.cpp:746-2263)
    pub fn city_ai_choose_production(&self, g: &mut Game, c: CityId) {
        todo!("CvCityAI::AI_chooseProduction")
    }

    /// C++ `UnitTypes CvCityAI::AI_bestUnit(bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR, UnitAITypes* peBestUnitAI = NULL)` (CvCityAI.cpp:2266-2526)
    pub fn city_ai_best_unit(&self, g: &Game, c: CityId, async_: bool, ignore_advisor: Option<AdvisorType>, best_unit_ai: &mut Option<UnitAIType>) -> Option<UnitType> {
        todo!("CvCityAI::AI_bestUnit")
    }

    /// C++ `UnitTypes CvCityAI::AI_bestUnitAI(UnitAITypes eUnitAI, bool bAsync = false, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR)` (CvCityAI.cpp:2529-2746)
    pub fn city_ai_best_unit_ai(&self, g: &Game, c: CityId, unit_ai: UnitAIType, async_: bool, ignore_advisor: Option<AdvisorType>) -> Option<UnitType> {
        todo!("CvCityAI::AI_bestUnitAI")
    }

}
