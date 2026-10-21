//! Port of CvPlayerAI.cpp:142-3366 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvPlayerAI::AI_init()` (CvPlayerAI.cpp:142-157)
    pub fn player_ai_init(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_init")
    }

    /// C++ `void CvPlayerAI::AI_reset(bool bConstructor)` (CvPlayerAI.cpp:168-293)
    pub fn player_ai_reset(&self, g: &mut Game, p: PlayerId, constructor: bool) {
        todo!("CvPlayerAI::AI_reset")
    }

    /// C++ `int CvPlayerAI::AI_getFlavorValue(FlavorTypes eFlavor) const` (CvPlayerAI.cpp:296-301)
    pub fn player_ai_get_flavor_value(&self, g: &Game, p: PlayerId, flavor: FlavorType) -> i32 {
        todo!("CvPlayerAI::AI_getFlavorValue")
    }

    /// C++ `void CvPlayerAI::AI_doTurnPre()` (CvPlayerAI.cpp:304-372)
    pub fn player_ai_do_turn_pre(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doTurnPre")
    }

    /// C++ `void CvPlayerAI::AI_doTurnPost()` (CvPlayerAI.cpp:375-412)
    pub fn player_ai_do_turn_post(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doTurnPost")
    }

    /// C++ `void CvPlayerAI::AI_doTurnUnitsPre()` (CvPlayerAI.cpp:415-440)
    pub fn player_ai_do_turn_units_pre(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doTurnUnitsPre")
    }

    /// C++ `void CvPlayerAI::AI_doTurnUnitsPost()` (CvPlayerAI.cpp:443-723)
    pub fn player_ai_do_turn_units_post(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doTurnUnitsPost")
    }

    /// C++ `void CvPlayerAI::AI_doPeace()` (CvPlayerAI.cpp:726-1048)
    pub fn player_ai_do_peace(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doPeace")
    }

    /// C++ `void CvPlayerAI::AI_updateFoundValues(bool bStartingLoc = false) const` (CvPlayerAI.cpp:1051-1127)
    pub fn player_ai_update_found_values(&self, g: &mut Game, p: PlayerId, starting_loc: bool) {
        todo!("CvPlayerAI::AI_updateFoundValues")
    }

    /// C++ `void CvPlayerAI::AI_updateAreaTargets()` (CvPlayerAI.cpp:1130-1149)
    pub fn player_ai_update_area_targets(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_updateAreaTargets")
    }

    /// C++ `int CvPlayerAI::AI_movementPriority(CvSelectionGroup* pGroup) const` (CvPlayerAI.cpp:1153-1256)
    pub fn player_ai_movement_priority(&self, g: &Game, p: PlayerId, group: GroupId) -> i32 {
        todo!("CvPlayerAI::AI_movementPriority")
    }

    /// C++ `void CvPlayerAI::AI_unitUpdate()` (CvPlayerAI.cpp:1259-1372)
    pub fn player_ai_unit_update(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_unitUpdate")
    }

    /// C++ `void CvPlayerAI::AI_makeAssignWorkDirty()` (CvPlayerAI.cpp:1375-1384)
    pub fn player_ai_make_assign_work_dirty(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_makeAssignWorkDirty")
    }

    /// C++ `void CvPlayerAI::AI_assignWorkingPlots()` (CvPlayerAI.cpp:1387-1396)
    pub fn player_ai_assign_working_plots(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_assignWorkingPlots")
    }

    /// C++ `void CvPlayerAI::AI_updateAssignWork()` (CvPlayerAI.cpp:1399-1408)
    pub fn player_ai_update_assign_work(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_updateAssignWork")
    }

    /// C++ `void CvPlayerAI::AI_makeProductionDirty()` (CvPlayerAI.cpp:1411-1422)
    pub fn player_ai_make_production_dirty(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_makeProductionDirty")
    }

    /// C++ `void CvPlayerAI::AI_conquerCity(CvCity* pCity)` (CvPlayerAI.cpp:1425-1545)
    pub fn player_ai_conquer_city(&self, g: &Game, p: PlayerId, city: CityId) -> bool {
        todo!("CvPlayerAI::AI_conquerCity")
    }

    /// C++ `bool CvPlayerAI::AI_acceptUnit(CvUnit* pUnit) const` (CvPlayerAI.cpp:1548-1578)
    pub fn player_ai_accept_unit(&self, g: &Game, p: PlayerId, unit: UnitId) -> bool {
        todo!("CvPlayerAI::AI_acceptUnit")
    }

    /// C++ `bool CvPlayerAI::AI_captureUnit(UnitTypes eUnit, CvPlot* pPlot) const` (CvPlayerAI.cpp:1581-1610)
    pub fn player_ai_capture_unit(&self, g: &Game, p: PlayerId, unit: UnitType, plot: PlotIdx) -> bool {
        todo!("CvPlayerAI::AI_captureUnit")
    }

    /// C++ `DomainTypes CvPlayerAI::AI_unitAIDomainType(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:1613-1695)
    pub fn player_ai_unit_ai_domain_type(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> Option<DomainType> {
        todo!("CvPlayerAI::AI_unitAIDomainType")
    }

    /// C++ `int CvPlayerAI::AI_yieldWeight(YieldTypes eYield) const` (CvPlayerAI.cpp:1698-1706)
    pub fn player_ai_yield_weight(&self, g: &Game, p: PlayerId, yield_: YieldType) -> i32 {
        todo!("CvPlayerAI::AI_yieldWeight")
    }

    /// C++ `int CvPlayerAI::AI_commerceWeight(CommerceTypes eCommerce, CvCity* pCity = NULL) const` (CvPlayerAI.cpp:1709-1844)
    pub fn player_ai_commerce_weight(&self, g: &Game, p: PlayerId, commerce: CommerceType, city: Option<CityId>) -> i32 {
        todo!("CvPlayerAI::AI_commerceWeight")
    }

    /// C++ `int CvPlayerAI::AI_foundValue(int iX, int iY, int iMinRivalRange = -1, bool bStartingLoc = false) const` (CvPlayerAI.cpp:1847-2993)
    pub fn player_ai_found_value(&self, g: &Game, p: PlayerId, x: i32, y: i32, min_rival_range: i32, starting_loc: bool) -> i32 {
        todo!("CvPlayerAI::AI_foundValue")
    }

    /// C++ `bool CvPlayerAI::AI_isAreaAlone(CvArea* pArea) const` (CvPlayerAI.cpp:2996-2999)
    pub fn player_ai_is_area_alone(&self, g: &Game, p: PlayerId, area: AreaId) -> bool {
        todo!("CvPlayerAI::AI_isAreaAlone")
    }

    /// C++ `bool CvPlayerAI::AI_isCapitalAreaAlone() const` (CvPlayerAI.cpp:3002-3014)
    pub fn player_ai_is_capital_area_alone(&self, g: &Game, p: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_isCapitalAreaAlone")
    }

    /// C++ `bool CvPlayerAI::AI_isPrimaryArea(CvArea* pArea) const` (CvPlayerAI.cpp:3017-3042)
    pub fn player_ai_is_primary_area(&self, g: &Game, p: PlayerId, area: AreaId) -> bool {
        todo!("CvPlayerAI::AI_isPrimaryArea")
    }

    /// C++ `int CvPlayerAI::AI_militaryWeight(CvArea* pArea) const` (CvPlayerAI.cpp:3045-3048)
    pub fn player_ai_military_weight(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_militaryWeight")
    }

    /// C++ `int CvPlayerAI::AI_targetCityValue(CvCity* pCity, bool bRandomize, bool bIgnoreAttackers = false) const` (CvPlayerAI.cpp:3051-3131)
    pub fn player_ai_target_city_value(&self, g: &Game, p: PlayerId, city: CityId, randomize: bool, ignore_attackers: bool) -> i32 {
        todo!("CvPlayerAI::AI_targetCityValue")
    }

    /// C++ `CvCity* CvPlayerAI::AI_findTargetCity(CvArea* pArea) const` (CvPlayerAI.cpp:3134-3170)
    pub fn player_ai_find_target_city(&self, g: &Game, p: PlayerId, area: AreaId) -> Option<CityId> {
        todo!("CvPlayerAI::AI_findTargetCity")
    }

    /// C++ `bool CvPlayerAI::AI_isCommercePlot(CvPlot* pPlot) const` (CvPlayerAI.cpp:3173-3176)
    pub fn player_ai_is_commerce_plot(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
        todo!("CvPlayerAI::AI_isCommercePlot")
    }

    /// C++ `int CvPlayerAI::AI_getPlotDanger(CvPlot* pPlot, int iRange = -1, bool bTestMoves = true) const` (CvPlayerAI.cpp:3179-3271)
    pub fn player_ai_get_plot_danger(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> i32 {
        todo!("CvPlayerAI::AI_getPlotDanger")
    }

    /// C++ `int CvPlayerAI::AI_getUnitDanger(CvUnit* pUnit, int iRange = -1, bool bTestMoves = true, bool bAnyDanger = true) const` (CvPlayerAI.cpp:3274-3366)
    pub fn player_ai_get_unit_danger(&self, g: &Game, p: PlayerId, unit: UnitId, range: i32, test_moves: bool, any_danger: bool) -> i32 {
        todo!("CvPlayerAI::AI_getUnitDanger")
    }

}
