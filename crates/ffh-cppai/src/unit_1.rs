//! Port of CvUnitAI.cpp:41-2953 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvUnitAI::AI_init(UnitAITypes eUnitAI)` (CvUnitAI.cpp:41-52)
    pub fn unit_ai_init(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType) {
        todo!("CvUnitAI::AI_init")
    }

    /// C++ `void CvUnitAI::AI_reset(UnitAITypes eUnitAI = NO_UNITAI)` (CvUnitAI.cpp:60-87)
    pub fn unit_ai_reset(&self, g: &mut Game, u: UnitId, unit_ai: Option<UnitAIType>) {
        todo!("CvUnitAI::AI_reset")
    }

    /// C++ `bool CvUnitAI::AI_update()` (CvUnitAI.cpp:90-732)
    pub fn unit_ai_update(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_update")
    }

    /// C++ `bool CvUnitAI::AI_follow()` (CvUnitAI.cpp:736-774)
    pub fn unit_ai_follow(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_follow")
    }

    /// C++ `void CvUnitAI::AI_upgrade()` (CvUnitAI.cpp:779-825)
    pub fn unit_ai_upgrade(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_upgrade")
    }

    /// C++ `void CvUnitAI::AI_promote()` (CvUnitAI.cpp:828-895)
    pub fn unit_ai_promote(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_promote")
    }

    /// C++ `int CvUnitAI::AI_groupFirstVal()` (CvUnitAI.cpp:898-1116)
    pub fn unit_ai_group_first_val(&self, g: &Game, u: UnitId) -> i32 {
        todo!("CvUnitAI::AI_groupFirstVal")
    }

    /// C++ `int CvUnitAI::AI_groupSecondVal()` (CvUnitAI.cpp:1119-1122)
    pub fn unit_ai_group_second_val(&self, g: &Game, u: UnitId) -> i32 {
        todo!("CvUnitAI::AI_groupSecondVal")
    }

    /// C++ `int CvUnitAI::AI_attackOdds(const CvPlot* pPlot, bool bPotentialEnemy) const` (CvUnitAI.cpp:1127-1227)
    pub fn unit_ai_attack_odds(&self, g: &Game, u: UnitId, plot: PlotIdx, potential_enemy: bool) -> i32 {
        todo!("CvUnitAI::AI_attackOdds")
    }

    /// C++ `bool CvUnitAI::AI_bestCityBuild(CvCity* pCity, CvPlot** ppBestPlot = NULL, BuildTypes* peBestBuild = NULL, CvPlot* pIgnorePlot = NULL, CvUnit* pUnit = NULL)` (CvUnitAI.cpp:1231-1373)
    pub fn unit_ai_best_city_build(&self, g: &mut Game, u: UnitId, city: CityId, best_plot: &mut Option<PlotIdx>, best_build: &mut Option<BuildType>, ignore_plot: Option<PlotIdx>, unit: Option<UnitId>) -> bool {
        todo!("CvUnitAI::AI_bestCityBuild")
    }

    /// C++ `bool CvUnitAI::AI_isCityAIType() const` (CvUnitAI.cpp:1376-1382)
    pub fn unit_ai_is_city_ai_type(&self, g: &Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_isCityAIType")
    }

    /// C++ `int CvUnitAI::AI_getBirthmark() const` (CvUnitAI.cpp:1385-1388)
    pub fn unit_ai_get_birthmark(&self, g: &Game, u: UnitId) -> i32 {
        todo!("CvUnitAI::AI_getBirthmark")
    }

    /// C++ `void CvUnitAI::AI_setBirthmark(int iNewValue)` (CvUnitAI.cpp:1391-1422)
    pub fn unit_ai_set_birthmark(&self, g: &mut Game, u: UnitId, new_value: i32) {
        todo!("CvUnitAI::AI_setBirthmark")
    }

    /// C++ `UnitAITypes CvUnitAI::AI_getUnitAIType() const` (CvUnitAI.cpp:1425-1428)
    pub fn unit_ai_get_unit_ai_type(&self, g: &Game, u: UnitId) -> Option<UnitAIType> {
        todo!("CvUnitAI::AI_getUnitAIType")
    }

    /// C++ `void CvUnitAI::AI_setUnitAIType(UnitAITypes eNewValue)` (CvUnitAI.cpp:1432-1448)
    pub fn unit_ai_set_unit_ai_type(&self, g: &mut Game, u: UnitId, new_value: UnitAIType) {
        todo!("CvUnitAI::AI_setUnitAIType")
    }

    /// C++ `int CvUnitAI::AI_sacrificeValue(const CvPlot* pPlot) const` (CvUnitAI.cpp:1450-1495)
    pub fn unit_ai_sacrifice_value(&self, g: &Game, u: UnitId, plot: PlotIdx) -> i32 {
        todo!("CvUnitAI::AI_sacrificeValue")
    }

    /// C++ `void CvUnitAI::AI_animalMove()` (CvUnitAI.cpp:1499-1536)
    pub fn unit_ai_animal_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_animalMove")
    }

    /// C++ `void CvUnitAI::AI_settleMove()` (CvUnitAI.cpp:1539-1909)
    pub fn unit_ai_settle_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_settleMove")
    }

    /// C++ `void CvUnitAI::AI_workerMove()` (CvUnitAI.cpp:1912-2269)
    pub fn unit_ai_worker_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_workerMove")
    }

    /// C++ `void CvUnitAI::AI_barbAttackMove()` (CvUnitAI.cpp:2272-2670)
    pub fn unit_ai_barb_attack_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_barbAttackMove")
    }

    /// C++ `void CvUnitAI::AI_attackMove()` (CvUnitAI.cpp:2673-2953)
    pub fn unit_ai_attack_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_attackMove")
    }

}
