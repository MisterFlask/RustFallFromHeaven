//! Port of CvPlayerAI.cpp:3368-6803 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `int CvPlayerAI::AI_getWaterDanger(CvPlot* pPlot, int iRange, bool bTestMoves = true) const` (CvPlayerAI.cpp:3368-3424)
    pub fn player_ai_get_water_danger(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> i32 {
        todo!("CvPlayerAI::AI_getWaterDanger")
    }

    /// C++ `bool CvPlayerAI::AI_avoidScience() const` (CvPlayerAI.cpp:3427-3444)
    pub fn player_ai_avoid_science(&self, g: &Game, p: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_avoidScience")
    }

    /// C++ `bool CvPlayerAI::AI_isFinancialTrouble() const` (CvPlayerAI.cpp:3448-3505)
    pub fn player_ai_is_financial_trouble(&self, g: &Game, p: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_isFinancialTrouble")
    }

    /// C++ `int CvPlayerAI::AI_goldTarget() const` (CvPlayerAI.cpp:3507-3558)
    pub fn player_ai_gold_target(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_goldTarget")
    }

    /// C++ `int CvPlayerAI::AI_getGoldReserve() const` (CvPlayerAI.cpp:3566-3590)
    pub fn player_ai_get_gold_reserve(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getGoldReserve")
    }

    /// C++ `TechTypes CvPlayerAI::AI_bestTech(int iMaxPathLength = 1, bool bIgnoreCost = false, bool bAsync = false, TechTypes eIgnoreTech = NO_TECH, AdvisorTypes eIgnoreAdvisor = NO_ADVISOR) const` (CvPlayerAI.cpp:3595-5202)
    pub fn player_ai_best_tech(&self, g: &Game, p: PlayerId, max_path_length: i32, ignore_cost: bool, async_: bool, ignore_tech: Option<TechType>, ignore_advisor: Option<AdvisorType>) -> Option<TechType> {
        todo!("CvPlayerAI::AI_bestTech")
    }

    /// C++ `void CvPlayerAI::AI_chooseFreeTech()` (CvPlayerAI.cpp:5205-5228)
    pub fn player_ai_choose_free_tech(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        todo!("CvPlayerAI::AI_chooseFreeTech")
    }

    /// C++ `void CvPlayerAI::AI_chooseResearch()` (CvPlayerAI.cpp:5231-5298)
    pub fn player_ai_choose_research(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        todo!("CvPlayerAI::AI_chooseResearch")
    }

    /// C++ `bool CvPlayerAI::AI_isWillingToTalk(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5334-5382)
    pub fn player_ai_is_willing_to_talk(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_isWillingToTalk")
    }

    /// C++ `bool CvPlayerAI::AI_demandRebukedSneak(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5387-5404)
    pub fn player_ai_demand_rebuked_sneak(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_demandRebukedSneak")
    }

    /// C++ `bool CvPlayerAI::AI_demandRebukedWar(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5409-5430)
    pub fn player_ai_demand_rebuked_war(&self, g: &Game, p: PlayerId, player: PlayerId) -> bool {
        todo!("CvPlayerAI::AI_demandRebukedWar")
    }

    /// C++ `bool CvPlayerAI::AI_hasTradedWithTeam(TeamTypes eTeam) const` (CvPlayerAI.cpp:5434-5453)
    pub fn player_ai_has_traded_with_team(&self, g: &Game, p: PlayerId, team: TeamId) -> bool {
        todo!("CvPlayerAI::AI_hasTradedWithTeam")
    }

    /// C++ `AttitudeTypes CvPlayerAI::AI_getAttitudeFromValue(int iAttitudeVal)` (CvPlayerAI.cpp:5456-5478)
    pub fn player_ai_get_attitude_from_value(&self, g: &Game, attitude_val: i32) -> AttitudeType {
        todo!("CvPlayerAI::AI_getAttitudeFromValue")
    }

    /// C++ `AttitudeTypes CvPlayerAI::AI_getAttitude(PlayerTypes ePlayer, bool bForced = true) const` (CvPlayerAI.cpp:5480-5487)
    pub fn player_ai_get_attitude(&self, g: &Game, p: PlayerId, player: PlayerId, forced: bool) -> AttitudeType {
        todo!("CvPlayerAI::AI_getAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeVal(PlayerTypes ePlayer, bool bForced = true) const` (CvPlayerAI.cpp:5490-5588)
    pub fn player_ai_get_attitude_val(&self, g: &Game, p: PlayerId, player: PlayerId, forced: bool) -> i32 {
        todo!("CvPlayerAI::AI_getAttitudeVal")
    }

    /// C++ `int CvPlayerAI::AI_calculateStolenCityRadiusPlots(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5591-5617)
    pub fn player_ai_calculate_stolen_city_radius_plots(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_calculateStolenCityRadiusPlots")
    }

    /// C++ `int CvPlayerAI::AI_getCloseBordersAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5620-5642)
    pub fn player_ai_get_close_borders_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getCloseBordersAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getWarAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5645-5664)
    pub fn player_ai_get_war_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getWarAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getPeaceAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5667-5678)
    pub fn player_ai_get_peace_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getPeaceAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getSameReligionAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5681-5705)
    pub fn player_ai_get_same_religion_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getSameReligionAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getDifferentReligionAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5708-5740)
    pub fn player_ai_get_different_religion_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getDifferentReligionAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getBonusTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5743-5757)
    pub fn player_ai_get_bonus_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getBonusTradeAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getOpenBordersAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5760-5774)
    pub fn player_ai_get_open_borders_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getOpenBordersAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getDefensivePactAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5777-5796)
    pub fn player_ai_get_defensive_pact_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getDefensivePactAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getRivalDefensivePactAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5799-5814)
    pub fn player_ai_get_rival_defensive_pact_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getRivalDefensivePactAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getRivalVassalAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5817-5832)
    pub fn player_ai_get_rival_vassal_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getRivalVassalAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getShareWarAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5835-5857)
    pub fn player_ai_get_share_war_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getShareWarAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteCivicAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5860-5883)
    pub fn player_ai_get_favorite_civic_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getFavoriteCivicAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5886-5890)
    pub fn player_ai_get_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getTradeAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getRivalTradeAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5893-5897)
    pub fn player_ai_get_rival_trade_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getRivalTradeAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getMemoryAttitude(PlayerTypes ePlayer, MemoryTypes eMemory) const` (CvPlayerAI.cpp:5900-5903)
    pub fn player_ai_get_memory_attitude(&self, g: &Game, p: PlayerId, player: PlayerId, memory: MemoryType) -> i32 {
        todo!("CvPlayerAI::AI_getMemoryAttitude")
    }

    /// C++ `int CvPlayerAI::AI_getColonyAttitude(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:5905-5915)
    pub fn player_ai_get_colony_attitude(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getColonyAttitude")
    }

    /// C++ `PlayerVoteTypes CvPlayerAI::AI_diploVote(const VoteSelectionSubData& kVoteData, VoteSourceTypes eVoteSource, bool bPropose)` (CvPlayerAI.cpp:5919-6803)
    pub fn player_ai_diplo_vote(&self, g: &Game, p: PlayerId, vote_data: &VoteSelectionSubData, vote_source: VoteSourceType, propose: bool) -> PlayerVote {
        todo!("CvPlayerAI::AI_diploVote")
    }

}
