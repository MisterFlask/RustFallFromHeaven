//! Port of CvPlayerAI.cpp:12386-16739 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `void CvPlayerAI::AI_doDiplo()` (CvPlayerAI.cpp:12386-14222)
    pub fn player_ai_do_diplo(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doDiplo")
    }

    /// C++ `int CvPlayerAI::AI_eventValue(EventTypes eEvent, const EventTriggeredData& kTriggeredData) const` (CvPlayerAI.cpp:14393-14934)
    pub fn player_ai_event_value(&self, g: &Game, p: PlayerId, event: EventType, triggered_data: &EventTriggeredData) -> i32 {
        todo!("CvPlayerAI::AI_eventValue")
    }

    /// C++ `EventTypes CvPlayerAI::AI_chooseEvent(int iTriggeredId) const` (CvPlayerAI.cpp:14936-14969)
    pub fn player_ai_choose_event(&self, g: &Game, p: PlayerId, triggered: &EventTriggeredData) -> Option<EventType> {
        todo!("CvPlayerAI::AI_chooseEvent")
    }

    /// C++ `void CvPlayerAI::AI_doSplit()` (CvPlayerAI.cpp:14971-15024)
    pub fn player_ai_do_split(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doSplit")
    }

    /// C++ `void CvPlayerAI::AI_launch(VictoryTypes eVictory)` (CvPlayerAI.cpp:15026-15084)
    pub fn player_ai_launch(&self, g: &mut Game, p: PlayerId, victory: VictoryType) {
        todo!("CvPlayerAI::AI_launch")
    }

    /// C++ `void CvPlayerAI::AI_doCheckFinancialTrouble()` (CvPlayerAI.cpp:15086-15108)
    pub fn player_ai_do_check_financial_trouble(&self, g: &mut Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_doCheckFinancialTrouble")
    }

    /// C++ `bool CvPlayerAI::AI_disbandUnit(int iExpThreshold, bool bObsolete)` (CvPlayerAI.cpp:15110-15283)
    pub fn player_ai_disband_unit(&self, g: &mut Game, p: PlayerId, exp_threshold: i32, obsolete: bool) -> bool {
        todo!("CvPlayerAI::AI_disbandUnit")
    }

    /// C++ `int CvPlayerAI::AI_cultureVictoryTechValue(TechTypes eTech) const` (CvPlayerAI.cpp:15285-15360)
    pub fn player_ai_culture_victory_tech_value(&self, g: &Game, p: PlayerId, tech: TechType) -> i32 {
        todo!("CvPlayerAI::AI_cultureVictoryTechValue")
    }

    /// C++ `int CvPlayerAI::AI_getCultureVictoryStage() const` (CvPlayerAI.cpp:15362-15493)
    pub fn player_ai_get_culture_victory_stage(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getCultureVictoryStage")
    }

    /// C++ `bool CvPlayerAI::AI_isDoStrategy(int iStrategy) const` (CvPlayerAI.cpp:15496-15503)
    pub fn player_ai_is_do_strategy(&self, g: &Game, p: PlayerId, strategy: i32) -> bool {
        todo!("CvPlayerAI::AI_isDoStrategy")
    }

    /// C++ `void CvPlayerAI::AI_forceUpdateStrategies()` (CvPlayerAI.cpp:15505-15509)
    pub fn player_ai_force_update_strategies(&self, g: &Game, p: PlayerId) {
        todo!("CvPlayerAI::AI_forceUpdateStrategies")
    }

    /// C++ `int CvPlayerAI::AI_getStrategyHash() const` (CvPlayerAI.cpp:15511-16739)
    pub fn player_ai_get_strategy_hash(&self, g: &Game, p: PlayerId) -> i32 {
        todo!("CvPlayerAI::AI_getStrategyHash")
    }

}
