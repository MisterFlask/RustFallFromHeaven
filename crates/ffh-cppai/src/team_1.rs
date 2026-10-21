//! Port of CvTeamAI.cpp:80-2216 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvTeamAI::AI_init()` (CvTeamAI.cpp:80-86)
    pub fn team_ai_init(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_init")
    }

    /// C++ `void CvTeamAI::AI_reset(bool bConstructor)` (CvTeamAI.cpp:94-131)
    pub fn team_ai_reset(&self, g: &mut Game, t: TeamId, constructor: bool) {
        todo!("CvTeamAI::AI_reset")
    }

    /// C++ `void CvTeamAI::AI_doTurnPre()` (CvTeamAI.cpp:134-152)
    pub fn team_ai_do_turn_pre(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_doTurnPre")
    }

    /// C++ `void CvTeamAI::AI_doTurnPost()` (CvTeamAI.cpp:155-177)
    pub fn team_ai_do_turn_post(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_doTurnPost")
    }

    /// C++ `void CvTeamAI::AI_makeAssignWorkDirty()` (CvTeamAI.cpp:180-194)
    pub fn team_ai_make_assign_work_dirty(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_makeAssignWorkDirty")
    }

    /// C++ `void CvTeamAI::AI_updateAreaStragies(bool bTargets = true)` (CvTeamAI.cpp:197-216)
    pub fn team_ai_update_area_stragies(&self, g: &mut Game, t: TeamId, targets: bool) {
        todo!("CvTeamAI::AI_updateAreaStragies")
    }

    /// C++ `void CvTeamAI::AI_updateAreaTargets()` (CvTeamAI.cpp:219-233)
    pub fn team_ai_update_area_targets(&self, g: &mut Game, t: TeamId) {
        todo!("CvTeamAI::AI_updateAreaTargets")
    }

    /// C++ `int CvTeamAI::AI_countFinancialTrouble() const` (CvTeamAI.cpp:236-257)
    pub fn team_ai_count_financial_trouble(&self, g: &Game, t: TeamId) -> i32 {
        todo!("CvTeamAI::AI_countFinancialTrouble")
    }

    /// C++ `int CvTeamAI::AI_countMilitaryWeight(CvArea* pArea) const` (CvTeamAI.cpp:260-278)
    pub fn team_ai_count_military_weight(&self, g: &Game, t: TeamId, area: AreaId) -> i32 {
        todo!("CvTeamAI::AI_countMilitaryWeight")
    }

    /// C++ `bool CvTeamAI::AI_isAnyCapitalAreaAlone() const` (CvTeamAI.cpp:281-299)
    pub fn team_ai_is_any_capital_area_alone(&self, g: &Game, t: TeamId) -> bool {
        todo!("CvTeamAI::AI_isAnyCapitalAreaAlone")
    }

    /// C++ `bool CvTeamAI::AI_isPrimaryArea(CvArea* pArea) const` (CvTeamAI.cpp:302-320)
    pub fn team_ai_is_primary_area(&self, g: &Game, t: TeamId, area: AreaId) -> bool {
        todo!("CvTeamAI::AI_isPrimaryArea")
    }

    /// C++ `bool CvTeamAI::AI_hasCitiesInPrimaryArea(TeamTypes eTeam) const` (CvTeamAI.cpp:323-342)
    pub fn team_ai_has_cities_in_primary_area(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        todo!("CvTeamAI::AI_hasCitiesInPrimaryArea")
    }

    /// C++ `AreaAITypes CvTeamAI::AI_calculateAreaAIType(CvArea* pArea, bool bPreparingTotal = false) const` (CvTeamAI.cpp:345-523)
    pub fn team_ai_calculate_area_ai_type(&self, g: &Game, t: TeamId, area: AreaId, preparing_total: bool) -> Option<AreaAIType> {
        todo!("CvTeamAI::AI_calculateAreaAIType")
    }

    /// C++ `int CvTeamAI::AI_calculateAdjacentLandPlots(TeamTypes eTeam) const` (CvTeamAI.cpp:525-549)
    pub fn team_ai_calculate_adjacent_land_plots(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_calculateAdjacentLandPlots")
    }

    /// C++ `int CvTeamAI::AI_calculatePlotWarValue(TeamTypes eTeam) const` (CvTeamAI.cpp:552-578)
    pub fn team_ai_calculate_plot_war_value(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_calculatePlotWarValue")
    }

    /// C++ `int CvTeamAI::AI_calculateCapitalProximity(TeamTypes eTeam) const` (CvTeamAI.cpp:581-639)
    pub fn team_ai_calculate_capital_proximity(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_calculateCapitalProximity")
    }

    /// C++ `bool CvTeamAI::AI_isWarPossible() const` (CvTeamAI.cpp:642-660)
    pub fn team_ai_is_war_possible(&self, g: &Game, t: TeamId) -> bool {
        todo!("CvTeamAI::AI_isWarPossible")
    }

    /// C++ `bool CvTeamAI::AI_isLandTarget(TeamTypes eTeam) const` (CvTeamAI.cpp:663-676)
    pub fn team_ai_is_land_target(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        todo!("CvTeamAI::AI_isLandTarget")
    }

    /// C++ `bool CvTeamAI::AI_isAllyLandTarget(TeamTypes eTeam) const` (CvTeamAI.cpp:679-697)
    pub fn team_ai_is_ally_land_target(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        todo!("CvTeamAI::AI_isAllyLandTarget")
    }

    /// C++ `bool CvTeamAI::AI_shareWar(TeamTypes eTeam) const` (CvTeamAI.cpp:700-719)
    pub fn team_ai_share_war(&self, g: &Game, t: TeamId, team: TeamId) -> bool {
        todo!("CvTeamAI::AI_shareWar")
    }

    /// C++ `AttitudeTypes CvTeamAI::AI_getAttitude(TeamTypes eTeam, bool bForced = true) const` (CvTeamAI.cpp:722-761)
    pub fn team_ai_get_attitude(&self, g: &Game, t: TeamId, team: TeamId, forced: bool) -> AttitudeType {
        todo!("CvTeamAI::AI_getAttitude")
    }

    /// C++ `int CvTeamAI::AI_getAttitudeVal(TeamTypes eTeam, bool bForced = true) const` (CvTeamAI.cpp:764-800)
    pub fn team_ai_get_attitude_val(&self, g: &Game, t: TeamId, team: TeamId, forced: bool) -> i32 {
        todo!("CvTeamAI::AI_getAttitudeVal")
    }

    /// C++ `int CvTeamAI::AI_getMemoryCount(TeamTypes eTeam, MemoryTypes eMemory) const` (CvTeamAI.cpp:803-839)
    pub fn team_ai_get_memory_count(&self, g: &Game, t: TeamId, team: TeamId, memory: MemoryType) -> i32 {
        todo!("CvTeamAI::AI_getMemoryCount")
    }

    /// C++ `int CvTeamAI::AI_chooseElection(const VoteSelectionData& kVoteSelectionData) const` (CvTeamAI.cpp:842-895)
    pub fn team_ai_choose_election(&self, g: &Game, t: TeamId, vote_selection_data: &VoteSelectionData) -> i32 {
        todo!("CvTeamAI::AI_chooseElection")
    }

    /// C++ `int CvTeamAI::AI_startWarVal(TeamTypes eTeam) const` (CvTeamAI.cpp:898-988)
    pub fn team_ai_start_war_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_startWarVal")
    }

    /// C++ `int CvTeamAI::AI_endWarVal(TeamTypes eTeam) const` (CvTeamAI.cpp:992-1171)
    pub fn team_ai_end_war_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_endWarVal")
    }

    /// C++ `int CvTeamAI::AI_techTradeVal(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:1174-1224)
    pub fn team_ai_tech_trade_val(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_techTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_techTrade(TechTypes eTech, TeamTypes eTeam) const` (CvTeamAI.cpp:1227-1395)
    pub fn team_ai_tech_trade(&self, g: &Game, t: TeamId, tech: TechType, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_techTrade")
    }

    /// C++ `int CvTeamAI::AI_mapTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1398-1442)
    pub fn team_ai_map_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_mapTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_mapTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:1445-1489)
    pub fn team_ai_map_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_mapTrade")
    }

    /// C++ `int CvTeamAI::AI_vassalTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1492-1497)
    pub fn team_ai_vassal_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_vassalTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_vassalTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:1500-1553)
    pub fn team_ai_vassal_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_vassalTrade")
    }

    /// C++ `int CvTeamAI::AI_surrenderTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:1556-1561)
    pub fn team_ai_surrender_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_surrenderTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_surrenderTrade(TeamTypes eTeam, int iPowerMultiplier = 100) const` (CvTeamAI.cpp:1564-1827)
    pub fn team_ai_surrender_trade(&self, g: &Game, t: TeamId, team: TeamId, power_multiplier: i32) -> Option<DenialType> {
        todo!("CvTeamAI::AI_surrenderTrade")
    }

    /// C++ `int CvTeamAI::AI_makePeaceTradeVal(TeamTypes ePeaceTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1830-1887)
    pub fn team_ai_make_peace_trade_val(&self, g: &Game, t: TeamId, peace_team: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_makePeaceTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_makePeaceTrade(TeamTypes ePeaceTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1890-1929)
    pub fn team_ai_make_peace_trade(&self, g: &Game, t: TeamId, peace_team: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_makePeaceTrade")
    }

    /// C++ `int CvTeamAI::AI_declareWarTradeVal(TeamTypes eWarTeam, TeamTypes eTeam) const` (CvTeamAI.cpp:1932-2064)
    pub fn team_ai_declare_war_trade_val(&self, g: &Game, t: TeamId, war_team: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_declareWarTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_declareWarTrade(TeamTypes eWarTeam, TeamTypes eTeam, bool bConsiderPower = true) const` (CvTeamAI.cpp:2067-2158)
    pub fn team_ai_declare_war_trade(&self, g: &Game, t: TeamId, war_team: TeamId, team: TeamId, consider_power: bool) -> Option<DenialType> {
        todo!("CvTeamAI::AI_declareWarTrade")
    }

    /// C++ `int CvTeamAI::AI_openBordersTradeVal(TeamTypes eTeam) const` (CvTeamAI.cpp:2161-2164)
    pub fn team_ai_open_borders_trade_val(&self, g: &Game, t: TeamId, team: TeamId) -> i32 {
        todo!("CvTeamAI::AI_openBordersTradeVal")
    }

    /// C++ `DenialTypes CvTeamAI::AI_openBordersTrade(TeamTypes eTeam) const` (CvTeamAI.cpp:2167-2216)
    pub fn team_ai_open_borders_trade(&self, g: &Game, t: TeamId, team: TeamId) -> Option<DenialType> {
        todo!("CvTeamAI::AI_openBordersTrade")
    }

}
