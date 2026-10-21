//! Port of CvUnitAI.cpp:11300-14120 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `bool CvUnitAI::AI_defend()` (CvUnitAI.cpp:11301-11366)
    pub fn unit_ai_defend(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_defend")
    }

    /// C++ `bool CvUnitAI::AI_safety()` (CvUnitAI.cpp:11370-11482)
    pub fn unit_ai_safety(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_safety")
    }

    /// C++ `bool CvUnitAI::AI_hide()` (CvUnitAI.cpp:11486-11618)
    pub fn unit_ai_hide(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_hide")
    }

    /// C++ `bool CvUnitAI::AI_goody(int iRange)` (CvUnitAI.cpp:11622-11690)
    pub fn unit_ai_goody(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_goody")
    }

    /// C++ `bool CvUnitAI::AI_explore()` (CvUnitAI.cpp:11694-11801)
    pub fn unit_ai_explore(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_explore")
    }

    /// C++ `bool CvUnitAI::AI_exploreRange(int iRange)` (CvUnitAI.cpp:11805-11963)
    pub fn unit_ai_explore_range(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_exploreRange")
    }

    /// C++ `bool CvUnitAI::AI_targetCity(int iFlags = 0)` (CvUnitAI.cpp:11967-12118)
    pub fn unit_ai_target_city(&self, g: &mut Game, u: UnitId, flags: i32) -> bool {
        todo!("CvUnitAI::AI_targetCity")
    }

    /// C++ `bool CvUnitAI::AI_targetBarbCity()` (CvUnitAI.cpp:12127-12234)
    pub fn unit_ai_target_barb_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_targetBarbCity")
    }

    /// C++ `bool CvUnitAI::AI_bombardCity()` (CvUnitAI.cpp:12241-12312)
    pub fn unit_ai_bombard_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_bombardCity")
    }

    /// C++ `bool CvUnitAI::AI_cityAttack(int iRange, int iOddsThreshold, bool bFollow = false)` (CvUnitAI.cpp:12316-12385)
    pub fn unit_ai_city_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, follow: bool) -> bool {
        todo!("CvUnitAI::AI_cityAttack")
    }

    /// C++ `bool CvUnitAI::AI_anyAttack(int iRange, int iOddsThreshold, int iMinStack = 0, bool bFollow = false)` (CvUnitAI.cpp:12389-12500)
    pub fn unit_ai_any_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, min_stack: i32, follow: bool) -> bool {
        todo!("CvUnitAI::AI_anyAttack")
    }

    /// C++ `bool CvUnitAI::AI_rangeAttack(int iRange)` (CvUnitAI.cpp:12503-12552)
    pub fn unit_ai_range_attack(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_rangeAttack")
    }

    /// C++ `bool CvUnitAI::AI_leaveAttack(int iRange, int iOddsThreshold, int iStrengthThreshold)` (CvUnitAI.cpp:12554-12636)
    pub fn unit_ai_leave_attack(&self, g: &mut Game, u: UnitId, range: i32, odds_threshold: i32, strength_threshold: i32) -> bool {
        todo!("CvUnitAI::AI_leaveAttack")
    }

    /// C++ `bool CvUnitAI::AI_blockade()` (CvUnitAI.cpp:12639-12754)
    pub fn unit_ai_blockade(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_blockade")
    }

    /// C++ `bool CvUnitAI::AI_pirateBlockade()` (CvUnitAI.cpp:12758-12979)
    pub fn unit_ai_pirate_blockade(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_pirateBlockade")
    }

    /// C++ `bool CvUnitAI::AI_seaBombardRange(int iMaxRange)` (CvUnitAI.cpp:12982-13104)
    pub fn unit_ai_sea_bombard_range(&self, g: &mut Game, u: UnitId, max_range: i32) -> bool {
        todo!("CvUnitAI::AI_seaBombardRange")
    }

    /// C++ `bool CvUnitAI::AI_pillage(int iBonusValueThreshold = 0)` (CvUnitAI.cpp:13109-13227)
    pub fn unit_ai_pillage(&self, g: &mut Game, u: UnitId, bonus_value_threshold: i32) -> bool {
        todo!("CvUnitAI::AI_pillage")
    }

    /// C++ `bool CvUnitAI::AI_canPillage(CvPlot& kPlot) const` (CvUnitAI.cpp:13229-13265)
    pub fn unit_ai_can_pillage(&self, g: &Game, u: UnitId, plot: PlotIdx) -> bool {
        todo!("CvUnitAI::AI_canPillage")
    }

    /// C++ `bool CvUnitAI::AI_pillageRange(int iRange, int iBonusValueThreshold = 0)` (CvUnitAI.cpp:13269-13376)
    pub fn unit_ai_pillage_range(&self, g: &mut Game, u: UnitId, range: i32, bonus_value_threshold: i32) -> bool {
        todo!("CvUnitAI::AI_pillageRange")
    }

    /// C++ `bool CvUnitAI::AI_found()` (CvUnitAI.cpp:13380-13486)
    pub fn unit_ai_found(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_found")
    }

    /// C++ `bool CvUnitAI::AI_foundRange(int iRange, bool bFollow = false)` (CvUnitAI.cpp:13490-13563)
    pub fn unit_ai_found_range(&self, g: &mut Game, u: UnitId, range: i32, follow: bool) -> bool {
        todo!("CvUnitAI::AI_foundRange")
    }

    /// C++ `bool CvUnitAI::AI_assaultSeaTransport(bool bBarbarian = false)` (CvUnitAI.cpp:13567-13799)
    pub fn unit_ai_assault_sea_transport(&self, g: &mut Game, u: UnitId, barbarian: bool) -> bool {
        todo!("CvUnitAI::AI_assaultSeaTransport")
    }

    /// C++ `bool CvUnitAI::AI_settlerSeaTransport()` (CvUnitAI.cpp:13803-14050)
    pub fn unit_ai_settler_sea_transport(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_settlerSeaTransport")
    }

    /// C++ `bool CvUnitAI::AI_settlerSeaFerry()` (CvUnitAI.cpp:14054-14120)
    pub fn unit_ai_settler_sea_ferry(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_settlerSeaFerry")
    }

}
