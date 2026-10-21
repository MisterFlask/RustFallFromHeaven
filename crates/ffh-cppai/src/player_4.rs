//! Port of CvPlayerAI.cpp:9822-12384 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `int CvPlayerAI::AI_neededWorkers(CvArea* pArea) const` (CvPlayerAI.cpp:9822-9856)
    pub fn player_ai_needed_workers(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_neededWorkers")
    }

    /// C++ `int CvPlayerAI::AI_neededMissionaries(CvArea* pArea, ReligionTypes eReligion) const` (CvPlayerAI.cpp:9859-9902)
    pub fn player_ai_needed_missionaries(&self, g: &Game, p: PlayerId, area: AreaId, religion: ReligionType) -> i32 {
        todo!("CvPlayerAI::AI_neededMissionaries")
    }

    /// C++ `int CvPlayerAI::AI_neededExecutives(CvArea* pArea, CorporationTypes eCorporation) const` (CvPlayerAI.cpp:9905-9923)
    pub fn player_ai_needed_executives(&self, g: &Game, p: PlayerId, area: AreaId, corporation: CorporationType) -> i32 {
        todo!("CvPlayerAI::AI_neededExecutives")
    }

    /// C++ `int CvPlayerAI::AI_adjacentPotentialAttackers(CvPlot* pPlot, bool bTestCanMove = false) const` (CvPlayerAI.cpp:9926-9973)
    pub fn player_ai_adjacent_potential_attackers(&self, g: &Game, p: PlayerId, plot: PlotIdx, test_can_move: bool) -> i32 {
        todo!("CvPlayerAI::AI_adjacentPotentialAttackers")
    }

    /// C++ `int CvPlayerAI::AI_totalMissionAIs(MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:9976-9998)
    pub fn player_ai_total_mission_ais(&self, g: &Game, p: PlayerId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_totalMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_missionaryValue(CvArea* pArea, ReligionTypes eReligion, PlayerTypes* peBestPlayer = NULL) const` (CvPlayerAI.cpp:10000-10136)
    pub fn player_ai_missionary_value(&self, g: &Game, p: PlayerId, area: AreaId, religion: ReligionType, best_player: &mut Option<PlayerId>) -> i32 {
        todo!("CvPlayerAI::AI_missionaryValue")
    }

    /// C++ `int CvPlayerAI::AI_executiveValue(CvArea* pArea, CorporationTypes eCorporation, PlayerTypes* peBestPlayer = NULL) const` (CvPlayerAI.cpp:10138-10285)
    pub fn player_ai_executive_value(&self, g: &Game, p: PlayerId, area: AreaId, corporation: CorporationType, best_player: &mut Option<PlayerId>) -> i32 {
        todo!("CvPlayerAI::AI_executiveValue")
    }

    /// C++ `int CvPlayerAI::AI_corporationValue(CorporationTypes eCorporation, CvCity* pCity = NULL) const` (CvPlayerAI.cpp:10288-10335)
    pub fn player_ai_corporation_value(&self, g: &Game, p: PlayerId, corporation: CorporationType, city: Option<CityId>) -> i32 {
        todo!("CvPlayerAI::AI_corporationValue")
    }

    /// C++ `int CvPlayerAI::AI_areaMissionAIs(CvArea* pArea, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10337-10368)
    pub fn player_ai_area_mission_ais(&self, g: &Game, p: PlayerId, area: AreaId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_areaMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10371-10375)
    pub fn player_ai_plot_target_mission_ais(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        todo!("CvPlayerAI::AI_plotTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, int& iClosestTargetRange, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10377-10380)
    pub fn player_ai_plot_target_mission_ais_2(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, closest_target_range: &mut i32, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        todo!("CvPlayerAI::AI_plotTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes* aeMissionAI, int iMissionAICount, int& iClosestTargetRange, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10382-10421)
    pub fn player_ai_plot_target_mission_ais_3(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: &[MissionAIType], closest_target_range: &mut i32, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        todo!("CvPlayerAI::AI_plotTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_unitTargetMissionAIs(CvUnit* pUnit, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10424-10427)
    pub fn player_ai_unit_target_mission_ais(&self, g: &Game, p: PlayerId, unit: UnitId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_unitTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_unitTargetMissionAIs(CvUnit* pUnit, MissionAITypes* aeMissionAI, int iMissionAICount, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10429-10458)
    pub fn player_ai_unit_target_mission_ais_2(&self, g: &Game, p: PlayerId, unit: UnitId, mission_ai: &[MissionAIType], skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_unitTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_enemyTargetMissionAIs(MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10460-10463)
    pub fn player_ai_enemy_target_mission_ais(&self, g: &Game, p: PlayerId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_enemyTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_enemyTargetMissionAIs(MissionAITypes* aeMissionAI, int iMissionAICount, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10465-10497)
    pub fn player_ai_enemy_target_mission_ais_2(&self, g: &Game, p: PlayerId, mission_ai: &[MissionAIType], skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_enemyTargetMissionAIs")
    }

    /// C++ `int CvPlayerAI::AI_wakePlotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10499-10526)
    pub fn player_ai_wake_plot_target_mission_ais(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        todo!("CvPlayerAI::AI_wakePlotTargetMissionAIs")
    }

    /// C++ `CivicTypes CvPlayerAI::AI_bestCivic(CivicOptionTypes eCivicOption) const` (CvPlayerAI.cpp:10528-10570)
    pub fn player_ai_best_civic(&self, g: &Game, p: PlayerId, civic_option: CivicOptionType) -> Option<CivicType> {
        todo!("CvPlayerAI::AI_bestCivic")
    }

    /// C++ `int CvPlayerAI::AI_civicValue(CivicTypes eCivic) const` (CvPlayerAI.cpp:10573-10896)
    pub fn player_ai_civic_value(&self, g: &Game, p: PlayerId, civic: CivicType) -> i32 {
        todo!("CvPlayerAI::AI_civicValue")
    }

    /// C++ `ReligionTypes CvPlayerAI::AI_bestReligion() const` (CvPlayerAI.cpp:10899-10958)
    pub fn player_ai_best_religion(&self, g: &Game, p: PlayerId) -> Option<ReligionType> {
        todo!("CvPlayerAI::AI_bestReligion")
    }

    /// C++ `int CvPlayerAI::AI_religionValue(ReligionTypes eReligion) const` (CvPlayerAI.cpp:10961-11023)
    pub fn player_ai_religion_value(&self, g: &Game, p: PlayerId, religion: ReligionType) -> i32 {
        todo!("CvPlayerAI::AI_religionValue")
    }

    /// C++ `EspionageMissionTypes CvPlayerAI::AI_bestPlotEspionage(CvPlot* pSpyPlot, PlayerTypes& eTargetPlayer, CvPlot*& pPlot, int& iData) const` (CvPlayerAI.cpp:11025-11202)
    pub fn player_ai_best_plot_espionage(&self, g: &Game, p: PlayerId, spy_plot: PlotIdx, target_player: &mut Option<PlayerId>, plot: &mut Option<PlotIdx>, data: &mut i32) -> Option<EspionageMissionType> {
        todo!("CvPlayerAI::AI_bestPlotEspionage")
    }

    /// C++ `int CvPlayerAI::AI_espionageVal(PlayerTypes eTargetPlayer, EspionageMissionTypes eMission, CvPlot* pPlot, int iData) const` (CvPlayerAI.cpp:11204-11474)
    pub fn player_ai_espionage_val(&self, g: &Game, p: PlayerId, target_player: PlayerId, mission: EspionageMissionType, plot: PlotIdx, data: i32) -> i32 {
        todo!("CvPlayerAI::AI_espionageVal")
    }

    /// C++ `int CvPlayerAI::AI_getPeaceWeight() const` (CvPlayerAI.cpp:11476-11479)
    pub fn player_ai_get_peace_weight(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getPeaceWeight")
    }

    /// C++ `void CvPlayerAI::AI_setPeaceWeight(int iNewValue)` (CvPlayerAI.cpp:11482-11485)
    pub fn player_ai_set_peace_weight(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setPeaceWeight")
    }

    /// C++ `int CvPlayerAI::AI_getEspionageWeight() const` (CvPlayerAI.cpp:11487-11494)
    pub fn player_ai_get_espionage_weight(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getEspionageWeight")
    }

    /// C++ `void CvPlayerAI::AI_setEspionageWeight(int iNewValue)` (CvPlayerAI.cpp:11496-11499)
    pub fn player_ai_set_espionage_weight(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setEspionageWeight")
    }

    /// C++ `int CvPlayerAI::AI_getAttackOddsChange() const` (CvPlayerAI.cpp:11502-11505)
    pub fn player_ai_get_attack_odds_change(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getAttackOddsChange")
    }

    /// C++ `void CvPlayerAI::AI_setAttackOddsChange(int iNewValue)` (CvPlayerAI.cpp:11508-11511)
    pub fn player_ai_set_attack_odds_change(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setAttackOddsChange")
    }

    /// C++ `int CvPlayerAI::AI_getCivicTimer() const` (CvPlayerAI.cpp:11514-11517)
    pub fn player_ai_get_civic_timer(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getCivicTimer")
    }

    /// C++ `void CvPlayerAI::AI_setCivicTimer(int iNewValue)` (CvPlayerAI.cpp:11520-11524)
    pub fn player_ai_set_civic_timer(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setCivicTimer")
    }

    /// C++ `void CvPlayerAI::AI_changeCivicTimer(int iChange)` (CvPlayerAI.cpp:11527-11530)
    pub fn player_ai_change_civic_timer(&self, g: &mut Game, p: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeCivicTimer")
    }

    /// C++ `int CvPlayerAI::AI_getReligionTimer() const` (CvPlayerAI.cpp:11533-11536)
    pub fn player_ai_get_religion_timer(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getReligionTimer")
    }

    /// C++ `void CvPlayerAI::AI_setReligionTimer(int iNewValue)` (CvPlayerAI.cpp:11539-11543)
    pub fn player_ai_set_religion_timer(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setReligionTimer")
    }

    /// C++ `void CvPlayerAI::AI_changeReligionTimer(int iChange)` (CvPlayerAI.cpp:11546-11549)
    pub fn player_ai_change_religion_timer(&self, g: &mut Game, p: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeReligionTimer")
    }

    /// C++ `int CvPlayerAI::AI_getExtraGoldTarget() const` (CvPlayerAI.cpp:11551-11554)
    pub fn player_ai_get_extra_gold_target(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getExtraGoldTarget")
    }

    /// C++ `void CvPlayerAI::AI_setExtraGoldTarget(int iNewValue)` (CvPlayerAI.cpp:11556-11559)
    pub fn player_ai_set_extra_gold_target(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setExtraGoldTarget")
    }

    /// C++ `int CvPlayerAI::AI_getNumTrainAIUnits(UnitAITypes eIndex) const` (CvPlayerAI.cpp:11561-11566)
    pub fn player_ai_get_num_train_ai_units(&self, g: &Game, p: PlayerId, index: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_getNumTrainAIUnits")
    }

    /// C++ `void CvPlayerAI::AI_changeNumTrainAIUnits(UnitAITypes eIndex, int iChange)` (CvPlayerAI.cpp:11569-11575)
    pub fn player_ai_change_num_train_ai_units(&self, g: &mut Game, p: PlayerId, index: UnitAIType, change: i32) {
        todo!("CvPlayerAI::AI_changeNumTrainAIUnits")
    }

    /// C++ `int CvPlayerAI::AI_getNumAIUnits(UnitAITypes eIndex) const` (CvPlayerAI.cpp:11578-11583)
    pub fn player_ai_get_num_ai_units(&self, g: &Game, p: PlayerId, index: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_getNumAIUnits")
    }

    /// C++ `void CvPlayerAI::AI_changeNumAIUnits(UnitAITypes eIndex, int iChange)` (CvPlayerAI.cpp:11586-11592)
    pub fn player_ai_change_num_ai_units(&self, g: &mut Game, p: PlayerId, index: UnitAIType, change: i32) {
        todo!("CvPlayerAI::AI_changeNumAIUnits")
    }

    /// C++ `int CvPlayerAI::AI_getSameReligionCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11595-11600)
    pub fn player_ai_get_same_religion_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getSameReligionCounter")
    }

    /// C++ `void CvPlayerAI::AI_changeSameReligionCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11603-11609)
    pub fn player_ai_change_same_religion_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeSameReligionCounter")
    }

    /// C++ `int CvPlayerAI::AI_getDifferentReligionCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11612-11617)
    pub fn player_ai_get_different_religion_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getDifferentReligionCounter")
    }

    /// C++ `void CvPlayerAI::AI_changeDifferentReligionCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11620-11626)
    pub fn player_ai_change_different_religion_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeDifferentReligionCounter")
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteCivicCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11629-11634)
    pub fn player_ai_get_favorite_civic_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getFavoriteCivicCounter")
    }

    /// C++ `void CvPlayerAI::AI_changeFavoriteCivicCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11637-11643)
    pub fn player_ai_change_favorite_civic_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeFavoriteCivicCounter")
    }

    /// C++ `int CvPlayerAI::AI_getBonusTradeCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11646-11651)
    pub fn player_ai_get_bonus_trade_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getBonusTradeCounter")
    }

    /// C++ `void CvPlayerAI::AI_changeBonusTradeCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11654-11660)
    pub fn player_ai_change_bonus_trade_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeBonusTradeCounter")
    }

    /// C++ `int CvPlayerAI::AI_getPeacetimeTradeValue(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11663-11668)
    pub fn player_ai_get_peacetime_trade_value(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getPeacetimeTradeValue")
    }

    /// C++ `void CvPlayerAI::AI_changePeacetimeTradeValue(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11671-11720)
    pub fn player_ai_change_peacetime_trade_value(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changePeacetimeTradeValue")
    }

    /// C++ `int CvPlayerAI::AI_getPeacetimeGrantValue(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11723-11728)
    pub fn player_ai_get_peacetime_grant_value(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getPeacetimeGrantValue")
    }

    /// C++ `void CvPlayerAI::AI_changePeacetimeGrantValue(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11731-11780)
    pub fn player_ai_change_peacetime_grant_value(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changePeacetimeGrantValue")
    }

    /// C++ `int CvPlayerAI::AI_getGoldTradedTo(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11783-11788)
    pub fn player_ai_get_gold_traded_to(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getGoldTradedTo")
    }

    /// C++ `void CvPlayerAI::AI_changeGoldTradedTo(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11791-11797)
    pub fn player_ai_change_gold_traded_to(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeGoldTradedTo")
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeExtra(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11800-11805)
    pub fn player_ai_get_attitude_extra(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getAttitudeExtra")
    }

    /// C++ `void CvPlayerAI::AI_setAttitudeExtra(PlayerTypes eIndex, int iNewValue)` (CvPlayerAI.cpp:11808-11813)
    pub fn player_ai_set_attitude_extra(&self, g: &mut Game, p: PlayerId, index: PlayerId, new_value: i32) {
        todo!("CvPlayerAI::AI_setAttitudeExtra")
    }

    /// C++ `void CvPlayerAI::AI_changeAttitudeExtra(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11816-11819)
    pub fn player_ai_change_attitude_extra(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        todo!("CvPlayerAI::AI_changeAttitudeExtra")
    }

    /// C++ `bool CvPlayerAI::AI_isFirstContact(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11822-11827)
    pub fn player_ai_is_first_contact(&self, g: &Game, p: PlayerId, index: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_isFirstContact")
    }

    /// C++ `void CvPlayerAI::AI_setFirstContact(PlayerTypes eIndex, bool bNewValue)` (CvPlayerAI.cpp:11830-11835)
    pub fn player_ai_set_first_contact(&self, g: &mut Game, p: PlayerId, index: PlayerId, new_value: bool) {
        todo!("CvPlayerAI::AI_setFirstContact")
    }

    /// C++ `int CvPlayerAI::AI_getContactTimer(PlayerTypes eIndex1, ContactTypes eIndex2) const` (CvPlayerAI.cpp:11838-11845)
    pub fn player_ai_get_contact_timer(&self, g: &Game, p: PlayerId, index1: PlayerId, index2: ContactType) -> i32 {
        todo!("CvPlayerAI::AI_getContactTimer")
    }

    /// C++ `void CvPlayerAI::AI_changeContactTimer(PlayerTypes eIndex1, ContactTypes eIndex2, int iChange)` (CvPlayerAI.cpp:11848-11856)
    pub fn player_ai_change_contact_timer(&self, g: &mut Game, p: PlayerId, index1: PlayerId, index2: ContactType, change: i32) {
        todo!("CvPlayerAI::AI_changeContactTimer")
    }

    /// C++ `int CvPlayerAI::AI_getMemoryCount(PlayerTypes eIndex1, MemoryTypes eIndex2) const` (CvPlayerAI.cpp:11859-11866)
    pub fn player_ai_get_memory_count(&self, g: &Game, p: PlayerId, index1: PlayerId, index2: MemoryType) -> i32 {
        todo!("CvPlayerAI::AI_getMemoryCount")
    }

    /// C++ `void CvPlayerAI::AI_changeMemoryCount(PlayerTypes eIndex1, MemoryTypes eIndex2, int iChange)` (CvPlayerAI.cpp:11869-11877)
    pub fn player_ai_change_memory_count(&self, g: &mut Game, p: PlayerId, index1: PlayerId, index2: MemoryType, change: i32) {
        todo!("CvPlayerAI::AI_changeMemoryCount")
    }

    /// C++ `int CvPlayerAI::AI_calculateGoldenAgeValue() const` (CvPlayerAI.cpp:11879-11898)
    pub fn player_ai_calculate_golden_age_value(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_calculateGoldenAgeValue")
    }

    /// C++ `void CvPlayerAI::AI_doCounter()` (CvPlayerAI.cpp:11902-12006)
    pub fn player_ai_do_counter(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doCounter")
    }

    /// C++ `void CvPlayerAI::AI_doMilitary()` (CvPlayerAI.cpp:12009-12027)
    pub fn player_ai_do_military(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doMilitary")
    }

    /// C++ `void CvPlayerAI::AI_doResearch()` (CvPlayerAI.cpp:12030-12039)
    pub fn player_ai_do_research(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doResearch")
    }

    /// C++ `void CvPlayerAI::AI_doCommerce()` (CvPlayerAI.cpp:12042-12294)
    pub fn player_ai_do_commerce(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doCommerce")
    }

    /// C++ `void CvPlayerAI::AI_doCivics()` (CvPlayerAI.cpp:12297-12337)
    pub fn player_ai_do_civics(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doCivics")
    }

    /// C++ `void CvPlayerAI::AI_doReligion()` (CvPlayerAI.cpp:12340-12384)
    pub fn player_ai_do_religion(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doReligion")
    }

}
