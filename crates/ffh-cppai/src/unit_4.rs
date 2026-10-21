//! Port of CvUnitAI.cpp:8508-11297 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `bool CvUnitAI::AI_guardCity(bool bLeave = false, bool bSearch = false, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:8509-8758)
    pub fn unit_ai_guard_city(&self, g: &mut Game, u: UnitId, leave: bool, search: bool, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_guardCity")
    }

    /// C++ `bool CvUnitAI::AI_guardCityAirlift()` (CvUnitAI.cpp:8762-8826)
    pub fn unit_ai_guard_city_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_guardCityAirlift")
    }

    /// C++ `bool CvUnitAI::AI_guardBonus(int iMinValue = 0)` (CvUnitAI.cpp:8830-8935)
    pub fn unit_ai_guard_bonus(&self, g: &mut Game, u: UnitId, min_value: i32) -> bool {
        todo!("CvUnitAI::AI_guardBonus")
    }

    /// C++ `int CvUnitAI::AI_getPlotDefendersNeeded(CvPlot* pPlot, int iExtra)` (CvUnitAI.cpp:8937-8994)
    pub fn unit_ai_get_plot_defenders_needed(&self, g: &Game, u: UnitId, plot: PlotIdx, extra: i32) -> i32 {
        todo!("CvUnitAI::AI_getPlotDefendersNeeded")
    }

    /// C++ `bool CvUnitAI::AI_guardFort(bool bSearch = true)` (CvUnitAI.cpp:8996-9085)
    pub fn unit_ai_guard_fort(&self, g: &mut Game, u: UnitId, search: bool) -> bool {
        todo!("CvUnitAI::AI_guardFort")
    }

    /// C++ `bool CvUnitAI::AI_guardCitySite()` (CvUnitAI.cpp:9087-9137)
    pub fn unit_ai_guard_city_site(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_guardCitySite")
    }

    /// C++ `bool CvUnitAI::AI_guardSpy(int iRandomPercent)` (CvUnitAI.cpp:9142-9225)
    pub fn unit_ai_guard_spy(&self, g: &mut Game, u: UnitId, random_percent: i32) -> bool {
        todo!("CvUnitAI::AI_guardSpy")
    }

    /// C++ `bool CvUnitAI::AI_destroySpy()` (CvUnitAI.cpp:9229-9333)
    pub fn unit_ai_destroy_spy(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_destroySpy")
    }

    /// C++ `bool CvUnitAI::AI_sabotageSpy()` (CvUnitAI.cpp:9337-9440)
    pub fn unit_ai_sabotage_spy(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_sabotageSpy")
    }

    /// C++ `bool CvUnitAI::AI_pickupTargetSpy()` (CvUnitAI.cpp:9443-9507)
    pub fn unit_ai_pickup_target_spy(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_pickupTargetSpy")
    }

    /// C++ `bool CvUnitAI::AI_chokeDefend()` (CvUnitAI.cpp:9511-9545)
    pub fn unit_ai_choke_defend(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_chokeDefend")
    }

    /// C++ `bool CvUnitAI::AI_heal(int iDamagePercent = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:9549-9691)
    pub fn unit_ai_heal(&self, g: &mut Game, u: UnitId, damage_percent: i32, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_heal")
    }

    /// C++ `bool CvUnitAI::AI_afterAttack()` (CvUnitAI.cpp:9695-9756)
    pub fn unit_ai_after_attack(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_afterAttack")
    }

    /// C++ `bool CvUnitAI::AI_goldenAge()` (CvUnitAI.cpp:9760-9769)
    pub fn unit_ai_golden_age(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_goldenAge")
    }

    /// C++ `bool CvUnitAI::AI_spreadReligion()` (CvUnitAI.cpp:9773-10019)
    pub fn unit_ai_spread_religion(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_spreadReligion")
    }

    /// C++ `bool CvUnitAI::AI_spreadCorporation()` (CvUnitAI.cpp:10023-10162)
    pub fn unit_ai_spread_corporation(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_spreadCorporation")
    }

    /// C++ `bool CvUnitAI::AI_spreadReligionAirlift()` (CvUnitAI.cpp:10164-10278)
    pub fn unit_ai_spread_religion_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_spreadReligionAirlift")
    }

    /// C++ `bool CvUnitAI::AI_spreadCorporationAirlift()` (CvUnitAI.cpp:10280-10364)
    pub fn unit_ai_spread_corporation_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_spreadCorporationAirlift")
    }

    /// C++ `bool CvUnitAI::AI_discover(bool bThisTurnOnly = false, bool bFirstResearchOnly = false)` (CvUnitAI.cpp:10367-10418)
    pub fn unit_ai_discover(&self, g: &mut Game, u: UnitId, this_turn_only: bool, first_research_only: bool) -> bool {
        todo!("CvUnitAI::AI_discover")
    }

    /// C++ `bool CvUnitAI::AI_lead(std::vector<UnitAITypes>& aeUnitAITypes)` (CvUnitAI.cpp:10421-10528)
    pub fn unit_ai_lead(&self, g: &mut Game, u: UnitId, unit_ai_types: &mut Vec<i32>) -> bool {
        todo!("CvUnitAI::AI_lead")
    }

    /// C++ `bool CvUnitAI::AI_join(int iMaxCount = MAX_INT)` (CvUnitAI.cpp:10532-10610)
    pub fn unit_ai_join(&self, g: &mut Game, u: UnitId, max_count: i32) -> bool {
        todo!("CvUnitAI::AI_join")
    }

    /// C++ `bool CvUnitAI::AI_construct(int iMaxCount = MAX_INT, int iMaxSingleBuildingCount = MAX_INT, int iThreshold = 15)` (CvUnitAI.cpp:10615-10722)
    pub fn unit_ai_construct(&self, g: &mut Game, u: UnitId, max_count: i32, max_single_building_count: i32, threshold: i32) -> bool {
        todo!("CvUnitAI::AI_construct")
    }

    /// C++ `bool CvUnitAI::AI_switchHurry()` (CvUnitAI.cpp:10726-10789)
    pub fn unit_ai_switch_hurry(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_switchHurry")
    }

    /// C++ `bool CvUnitAI::AI_hurry()` (CvUnitAI.cpp:10793-10874)
    pub fn unit_ai_hurry(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_hurry")
    }

    /// C++ `bool CvUnitAI::AI_greatWork()` (CvUnitAI.cpp:10878-10939)
    pub fn unit_ai_great_work(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_greatWork")
    }

    /// C++ `bool CvUnitAI::AI_offensiveAirlift()` (CvUnitAI.cpp:10943-11034)
    pub fn unit_ai_offensive_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_offensiveAirlift")
    }

    /// C++ `bool CvUnitAI::AI_paradrop(int iRange)` (CvUnitAI.cpp:11038-11142)
    pub fn unit_ai_paradrop(&self, g: &mut Game, u: UnitId, range: i32) -> bool {
        todo!("CvUnitAI::AI_paradrop")
    }

    /// C++ `bool CvUnitAI::AI_protect(int iOddsThreshold)` (CvUnitAI.cpp:11146-11221)
    pub fn unit_ai_protect(&self, g: &mut Game, u: UnitId, odds_threshold: i32) -> bool {
        todo!("CvUnitAI::AI_protect")
    }

    /// C++ `bool CvUnitAI::AI_patrol()` (CvUnitAI.cpp:11225-11297)
    pub fn unit_ai_patrol(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_patrol")
    }

}
