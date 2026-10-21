//! Port of CvPlayerAI.cpp:6805-9819 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `int CvPlayerAI::AI_dealVal(PlayerTypes ePlayer, const CLinkList<TradeData>* pList, bool bIgnoreAnnual = false, int iChange = 1) const` (CvPlayerAI.cpp:6805-6892)
    pub fn player_ai_deal_val(&self, g: &Game, p: PlayerId, player: PlayerId, list: &[TradeData], ignore_annual: bool, change: i32) -> i32 {
        todo!("CvPlayerAI::AI_dealVal")
    }

    /// C++ `bool CvPlayerAI::AI_goldDeal(const CLinkList<TradeData>* pList) const` (CvPlayerAI.cpp:6895-6913)
    pub fn player_ai_gold_deal(&self, g: &Game, p: PlayerId, list: &[TradeData]) -> bool {
        todo!("CvPlayerAI::AI_goldDeal")
    }

    /// C++ `bool CvPlayerAI::AI_considerOffer(PlayerTypes ePlayer, const CLinkList<TradeData>* pTheirList, const CLinkList<TradeData>* pOurList, int iChange = 1) const` (CvPlayerAI.cpp:6916-7058)
    pub fn player_ai_consider_offer(&self, g: &Game, p: PlayerId, player: PlayerId, their_list: &[TradeData], our_list: &[TradeData], change: i32) -> bool {
        todo!("CvPlayerAI::AI_considerOffer")
    }

    /// C++ `bool CvPlayerAI::AI_counterPropose(PlayerTypes ePlayer, const CLinkList<TradeData>* pTheirList, const CLinkList<TradeData>* pOurList, CLinkList<TradeData>* pTheirInventory, CLinkList<TradeData>* pOurInventory, CLinkList<TradeData>* pTheirCounter, CLinkList<TradeData>* pOurCounter) const` (CvPlayerAI.cpp:7061-7610)
    pub fn player_ai_counter_propose(&self, g: &Game, p: PlayerId, player: PlayerId, their_list: &[TradeData], our_list: &[TradeData], their_inventory: &mut Vec<TradeData>, our_inventory: &mut Vec<TradeData>, their_counter: &mut Vec<TradeData>, our_counter: &mut Vec<TradeData>) -> bool {
        todo!("CvPlayerAI::AI_counterPropose")
    }

    /// C++ `int CvPlayerAI::AI_maxGoldTrade(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:7613-7654)
    pub fn player_ai_max_gold_trade(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_maxGoldTrade")
    }

    /// C++ `int CvPlayerAI::AI_maxGoldPerTurnTrade(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:7657-7678)
    pub fn player_ai_max_gold_per_turn_trade(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_maxGoldPerTurnTrade")
    }

    /// C++ `int CvPlayerAI::AI_goldPerTurnTradeVal(int iGoldPerTurn) const` (CvPlayerAI.cpp:7681-7688)
    pub fn player_ai_gold_per_turn_trade_val(&self, g: &Game, p: PlayerId, gold_per_turn: i32) -> i32 {
        todo!("CvPlayerAI::AI_goldPerTurnTradeVal")
    }

    /// C++ `int CvPlayerAI::AI_bonusVal(BonusTypes eBonus, int iChange = 1) const` (CvPlayerAI.cpp:7690-7716)
    pub fn player_ai_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType, change: i32) -> i32 {
        todo!("CvPlayerAI::AI_bonusVal")
    }

    /// C++ `int CvPlayerAI::AI_baseBonusVal(BonusTypes eBonus) const` (CvPlayerAI.cpp:7719-8038)
    pub fn player_ai_base_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
        todo!("CvPlayerAI::AI_baseBonusVal")
    }

    /// C++ `int CvPlayerAI::AI_corporationBonusVal(BonusTypes eBonus) const` (CvPlayerAI.cpp:8040-8086)
    pub fn player_ai_corporation_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
        todo!("CvPlayerAI::AI_corporationBonusVal")
    }

    /// C++ `int CvPlayerAI::AI_bonusTradeVal(BonusTypes eBonus, PlayerTypes ePlayer, int iChange) const` (CvPlayerAI.cpp:8089-8109)
    pub fn player_ai_bonus_trade_val(&self, g: &Game, p: PlayerId, bonus: BonusType, player: PlayerId, change: i32) -> i32 {
        todo!("CvPlayerAI::AI_bonusTradeVal")
    }

    /// C++ `DenialTypes CvPlayerAI::AI_bonusTrade(BonusTypes eBonus, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8112-8270)
    pub fn player_ai_bonus_trade(&self, g: &Game, p: PlayerId, bonus: BonusType, player: PlayerId) -> Option<DenialType> {
        todo!("CvPlayerAI::AI_bonusTrade")
    }

    /// C++ `int CvPlayerAI::AI_cityTradeVal(CvCity* pCity) const` (CvPlayerAI.cpp:8273-8318)
    pub fn player_ai_city_trade_val(&self, g: &Game, p: PlayerId, city: CityId) -> i32 {
        todo!("CvPlayerAI::AI_cityTradeVal")
    }

    /// C++ `DenialTypes CvPlayerAI::AI_cityTrade(CvCity* pCity, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8321-8374)
    pub fn player_ai_city_trade(&self, g: &Game, p: PlayerId, city: CityId, player: PlayerId) -> Option<DenialType> {
        todo!("CvPlayerAI::AI_cityTrade")
    }

    /// C++ `int CvPlayerAI::AI_stopTradingTradeVal(TeamTypes eTradeTeam, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8377-8471)
    pub fn player_ai_stop_trading_trade_val(&self, g: &Game, p: PlayerId, trade_team: TeamId, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_stopTradingTradeVal")
    }

    /// C++ `DenialTypes CvPlayerAI::AI_stopTradingTrade(TeamTypes eTradeTeam, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8474-8564)
    pub fn player_ai_stop_trading_trade(&self, g: &Game, p: PlayerId, trade_team: TeamId, player: PlayerId) -> Option<DenialType> {
        todo!("CvPlayerAI::AI_stopTradingTrade")
    }

    /// C++ `int CvPlayerAI::AI_civicTradeVal(CivicTypes eCivic, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8567-8599)
    pub fn player_ai_civic_trade_val(&self, g: &Game, p: PlayerId, civic: CivicType, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_civicTradeVal")
    }

    /// C++ `DenialTypes CvPlayerAI::AI_civicTrade(CivicTypes eCivic, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8602-8666)
    pub fn player_ai_civic_trade(&self, g: &Game, p: PlayerId, civic: CivicType, player: PlayerId) -> Option<DenialType> {
        todo!("CvPlayerAI::AI_civicTrade")
    }

    /// C++ `int CvPlayerAI::AI_religionTradeVal(ReligionTypes eReligion, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8669-8701)
    pub fn player_ai_religion_trade_val(&self, g: &Game, p: PlayerId, religion: ReligionType, player: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_religionTradeVal")
    }

    /// C++ `DenialTypes CvPlayerAI::AI_religionTrade(ReligionTypes eReligion, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8704-8740)
    pub fn player_ai_religion_trade(&self, g: &Game, p: PlayerId, religion: ReligionType, player: PlayerId) -> Option<DenialType> {
        todo!("CvPlayerAI::AI_religionTrade")
    }

    /// C++ `int CvPlayerAI::AI_unitImpassableCount(UnitTypes eUnit) const` (CvPlayerAI.cpp:8743-8771)
    pub fn player_ai_unit_impassable_count(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        todo!("CvPlayerAI::AI_unitImpassableCount")
    }

    /// C++ `int CvPlayerAI::AI_unitValue(UnitTypes eUnit, UnitAITypes eUnitAI, CvArea* pArea) const` (CvPlayerAI.cpp:8774-9701)
    pub fn player_ai_unit_value(&self, g: &Game, p: PlayerId, unit: UnitType, unit_ai: UnitAIType, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_unitValue")
    }

    /// C++ `int CvPlayerAI::AI_totalUnitAIs(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9704-9707)
    pub fn player_ai_total_unit_ais(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_totalUnitAIs")
    }

    /// C++ `int CvPlayerAI::AI_totalAreaUnitAIs(CvArea* pArea, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9710-9713)
    pub fn player_ai_total_area_unit_ais(&self, g: &Game, p: PlayerId, area: AreaId, unit_ai: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_totalAreaUnitAIs")
    }

    /// C++ `int CvPlayerAI::AI_totalWaterAreaUnitAIs(CvArea* pArea, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9716-9746)
    pub fn player_ai_total_water_area_unit_ais(&self, g: &Game, p: PlayerId, area: AreaId, unit_ai: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_totalWaterAreaUnitAIs")
    }

    /// C++ `int CvPlayerAI::AI_countCargoSpace(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9749-9766)
    pub fn player_ai_count_cargo_space(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> i32 {
        todo!("CvPlayerAI::AI_countCargoSpace")
    }

    /// C++ `int CvPlayerAI::AI_neededExplorers(CvArea* pArea) const` (CvPlayerAI.cpp:9769-9819)
    pub fn player_ai_needed_explorers(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        todo!("CvPlayerAI::AI_neededExplorers")
    }

}
