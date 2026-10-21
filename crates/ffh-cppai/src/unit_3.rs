//! Port of CvUnitAI.cpp:5695-8506 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvUnitAI::AI_exploreSeaMove()` (CvUnitAI.cpp:5695-5835)
    pub fn unit_ai_explore_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_exploreSeaMove")
    }

    /// C++ `void CvUnitAI::AI_assaultSeaMove()` (CvUnitAI.cpp:5838-6100)
    pub fn unit_ai_assault_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_assaultSeaMove")
    }

    /// C++ `void CvUnitAI::AI_missionarySeaMove()` (CvUnitAI.cpp:6102-6167)
    pub fn unit_ai_missionary_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_missionarySeaMove")
    }

    /// C++ `void CvUnitAI::AI_settlerSeaMove()` (CvUnitAI.cpp:6169-6378)
    pub fn unit_ai_settler_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_settlerSeaMove")
    }

    /// C++ `void CvUnitAI::AI_spySeaMove()` (CvUnitAI.cpp:6381-6435)
    pub fn unit_ai_spy_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_spySeaMove")
    }

    /// C++ `void CvUnitAI::AI_carrierSeaMove()` (CvUnitAI.cpp:6438-6524)
    pub fn unit_ai_carrier_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_carrierSeaMove")
    }

    /// C++ `void CvUnitAI::AI_missileCarrierSeaMove()` (CvUnitAI.cpp:6527-6581)
    pub fn unit_ai_missile_carrier_sea_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_missileCarrierSeaMove")
    }

    /// C++ `void CvUnitAI::AI_attackAirMove()` (CvUnitAI.cpp:6584-6686)
    pub fn unit_ai_attack_air_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_attackAirMove")
    }

    /// C++ `void CvUnitAI::AI_defenseAirMove()` (CvUnitAI.cpp:6689-6765)
    pub fn unit_ai_defense_air_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_defenseAirMove")
    }

    /// C++ `void CvUnitAI::AI_carrierAirMove()` (CvUnitAI.cpp:6768-6848)
    pub fn unit_ai_carrier_air_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_carrierAirMove")
    }

    /// C++ `void CvUnitAI::AI_missileAirMove()` (CvUnitAI.cpp:6851-6950)
    pub fn unit_ai_missile_air_move(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_missileAirMove")
    }

    /// C++ `void CvUnitAI::AI_networkAutomated()` (CvUnitAI.cpp:6953-7025)
    pub fn unit_ai_network_automated(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_networkAutomated")
    }

    /// C++ `void CvUnitAI::AI_cityAutomated()` (CvUnitAI.cpp:7028-7075)
    pub fn unit_ai_city_automated(&self, g: &mut Game, u: UnitId) {
        todo!("CvUnitAI::AI_cityAutomated")
    }

    /// C++ `int CvUnitAI::AI_promotionValue(PromotionTypes ePromotion)` (CvUnitAI.cpp:7079-7839)
    pub fn unit_ai_promotion_value(&self, g: &Game, u: UnitId, promotion: PromotionType) -> i32 {
        todo!("CvUnitAI::AI_promotionValue")
    }

    /// C++ `bool CvUnitAI::AI_shadow(UnitAITypes eUnitAI, int iMax = -1, int iMaxRatio = -1, bool bWithCargoOnly = true)` (CvUnitAI.cpp:7843-7920)
    pub fn unit_ai_shadow(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max: i32, max_ratio: i32, with_cargo_only: bool) -> bool {
        todo!("CvUnitAI::AI_shadow")
    }

    /// C++ `bool CvUnitAI::AI_group(UnitAITypes eUnitAI, int iMaxGroup = -1, int iMaxOwnUnitAI = -1, int iMinUnitAI = -1, bool bIgnoreFaster = false, bool bIgnoreOwnUnitType = false, bool bStackOfDoom = false, int iMaxPath = MAX_INT, bool bAllowRegrouping = false)` (CvUnitAI.cpp:7924-8059)
    pub fn unit_ai_group(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max_group: i32, max_own_unit_ai: i32, min_unit_ai: i32, ignore_faster: bool, ignore_own_unit_type: bool, stack_of_doom: bool, max_path: i32, allow_regrouping: bool) -> bool {
        todo!("CvUnitAI::AI_group")
    }

    /// C++ `bool CvUnitAI::AI_groupMergeRange(UnitAITypes eUnitAI, int iMaxRange, bool bBiggerOnly = true, bool bAllowRegrouping = false, bool bIgnoreFaster = false)` (CvUnitAI.cpp:8061-8187)
    pub fn unit_ai_group_merge_range(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max_range: i32, bigger_only: bool, allow_regrouping: bool, ignore_faster: bool) -> bool {
        todo!("CvUnitAI::AI_groupMergeRange")
    }

    /// C++ `bool CvUnitAI::AI_load(UnitAITypes eUnitAI, MissionAITypes eMissionAI, UnitAITypes eTransportedUnitAI = NO_UNITAI, int iMinCargo = -1, int iMinCargoSpace = -1, int iMaxCargoSpace = -1, int iMaxCargoOurUnitAI = -1, int iFlags = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:8190-8364)
    pub fn unit_ai_load(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, mission_ai: MissionAIType, transported_unit_ai: Option<UnitAIType>, min_cargo: i32, min_cargo_space: i32, max_cargo_space: i32, max_cargo_our_unit_ai: i32, flags: i32, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_load")
    }

    /// C++ `bool CvUnitAI::AI_guardCityBestDefender()` (CvUnitAI.cpp:8368-8389)
    pub fn unit_ai_guard_city_best_defender(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_guardCityBestDefender")
    }

    /// C++ `bool CvUnitAI::AI_guardCityMinDefender(bool bSearch = true)` (CvUnitAI.cpp:8391-8506)
    pub fn unit_ai_guard_city_min_defender(&self, g: &mut Game, u: UnitId, search: bool) -> bool {
        todo!("CvUnitAI::AI_guardCityMinDefender")
    }

}
