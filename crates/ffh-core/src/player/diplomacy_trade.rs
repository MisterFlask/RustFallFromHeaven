//! CvPlayer: diplomacy and trade -- contact, diplomacy event consequences, tradeable
//! items and denials, deals, bonuses (availability, export/import), trade routes with
//! other players, forced peace, and the
//! CvPlayerAI peacetime trade/grant bookkeeping that deals update.
//!
//! Trade denials (`getTradeDenial`) are the C++ AI's `AI_*Trade` judgements. Only the checks
//! that follow from the rules (war/peace feasibility of vassal deals, peace/war with third
//! parties) are ported; the AI's own judgement (attitudes, valuations, power) is not, and
//! reads as "no denial": whether an AI accepts a deal is `Controller::consider_offer`.


use crate::defs::NUM_CITY_PLOTS;
use crate::game::Game;
use crate::ids::*;
use crate::path::FinderKind;
use crate::structs::*;
use crate::utils::set_trade_item;
use ffh_data::*;

/// C++ `DiploEventTypes` (CvEnums.h; not generated since no info table uses it).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiploEvent {
    /// C++ `DIPLOEVENT_CONTACT`
    Contact,
    /// C++ `DIPLOEVENT_AI_CONTACT`
    AiContact,
    /// C++ `DIPLOEVENT_FAILED_CONTACT`
    FailedContact,
    /// C++ `DIPLOEVENT_GIVE_HELP`
    GiveHelp,
    /// C++ `DIPLOEVENT_REFUSED_HELP`
    RefusedHelp,
    /// C++ `DIPLOEVENT_ACCEPT_DEMAND`
    AcceptDemand,
    /// C++ `DIPLOEVENT_REJECTED_DEMAND`
    RejectedDemand,
    /// C++ `DIPLOEVENT_DEMAND_WAR`
    DemandWar,
    /// C++ `DIPLOEVENT_CONVERT`
    Convert,
    /// C++ `DIPLOEVENT_NO_CONVERT`
    NoConvert,
    /// C++ `DIPLOEVENT_REVOLUTION`
    Revolution,
    /// C++ `DIPLOEVENT_NO_REVOLUTION`
    NoRevolution,
    /// C++ `DIPLOEVENT_JOIN_WAR` (data1 = target team)
    JoinWar,
    /// C++ `DIPLOEVENT_NO_JOIN_WAR`
    NoJoinWar,
    /// C++ `DIPLOEVENT_STOP_TRADING` (data1 = target team)
    StopTrading,
    /// C++ `DIPLOEVENT_NO_STOP_TRADING`
    NoStopTrading,
    /// C++ `DIPLOEVENT_ASK_HELP`
    AskHelp,
    /// C++ `DIPLOEVENT_MADE_DEMAND`
    MadeDemand,
    /// C++ `DIPLOEVENT_RESEARCH_TECH` (data1 = tech)
    ResearchTech,
    /// C++ `DIPLOEVENT_TARGET_CITY` (data1 = city owner, data2 = city id)
    TargetCity,
    /// C++ `DIPLOEVENT_MADE_DEMAND_VASSAL`
    MadeDemandVassal,
}

impl Game {
    // ------------------------------------------------------------------ contact and diplomacy

    /// C++ `CvPlayer::canContact(ePlayer)`.
    pub fn player_can_contact(&self, p: PlayerId, other: PlayerId) -> bool {
        if other == p {
            return false;
        }

        if !self.players[p].alive || !self.players[other].alive {
            return false;
        }

        if self.player_is_barbarian(p) || self.player_is_barbarian(other) {
            return false;
        }

        if self.player_is_minor_civ(p) || self.player_is_minor_civ(other) {
            return false;
        }

        // Not in C++: a crusade Host has no diplomacy (docs/CRUSADE.md).
        if self.player_is_crusade_host(p) || self.player_is_crusade_host(other) {
            return false;
        }

        let team = self.players[p].team;
        let other_team = self.players[other].team;

        if team != other_team {
            if !self.teams[team].has_met[other_team] {
                return false;
            }

            if self.at_war(team, other_team) {
                //FfH: Added by Kael 08/14/2007
                if self.player_is_no_diplomacy_with_enemies(p) {
                    return false;
                }
                //FfH: End Add

                // C++ default `bAllowVassal = false`.
                if !self.team_can_change_war_peace(team, other_team, false) {
                    return false;
                }
            }

            if (self.players[p].human || self.players[other].human) && self.is_option(GameOptionType::GAMEOPTION_ALWAYS_WAR) {
                return false;
            }
        }

        //FfH: Added by Kael 11/06/2007
        if self.players[p].temp_player_timer > 0 {
            return false;
        }
        //FfH: End Add

        true
    }

    /// C++ `CvPlayer::handleDiploEvent(eDiploEvent, ePlayer, iData1, iData2)`: rules
    /// consequences of a diplomacy exchange between `p` (the AI side) and `other`
    /// (memories, first contact, forced peace, war, conversion, revolution, embargo, ...).
    pub fn player_handle_diplo_event(&mut self, p: PlayerId, event: DiploEvent, other: PlayerId, data1: i32, data2: i32) {
        debug_assert!(other != p, "shouldn't call this function on ourselves");

        match event {
            DiploEvent::Contact => {
                // AI_setFirstContact (AI bookkeeping)
                self.players[p].ai_first_contact[other] = true;
                self.players[other].ai_first_contact[p] = true;
            }

            DiploEvent::AiContact => {}

            DiploEvent::FailedContact => {
                self.players[p].ai_first_contact[other] = true;
                self.players[other].ai_first_contact[p] = true;
            }

            DiploEvent::GiveHelp => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_GIVE_HELP, 1);
                self.player_force_peace(p, other);
            }

            DiploEvent::RefusedHelp => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_REFUSED_HELP, 1);
            }

            DiploEvent::AcceptDemand => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_ACCEPT_DEMAND, 1);
                self.player_force_peace(p, other);
            }

            DiploEvent::RejectedDemand => {
                debug_assert!(self.players[other].team != self.players[p].team, "shouldn't call this function on our own team");

                ai_change_memory_count(self, p, other, MemoryType::MEMORY_REJECTED_DEMAND, 1);

                if ai_demand_rebuked_sneak(self, p, other) {
                    let team = self.players[p].team;
                    let other_team = self.players[other].team;
                    // C++ default `bWar = true`.
                    self.team_ai_set_war_plan(team, other_team, Some(WarPlanType::WARPLAN_PREPARING_LIMITED), true);
                }
            }

            DiploEvent::DemandWar => {
                debug_assert!(self.players[other].team != self.players[p].team, "shouldn't call this function on our own team");

                let team = self.players[p].team;
                let other_team = self.players[other].team;
                self.team_declare_war(team, other_team, false, Some(WarPlanType::WARPLAN_LIMITED));
            }

            DiploEvent::Convert => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_ACCEPTED_RELIGION, 1);
                let religion = self.player_get_state_religion(p);
                self.player_convert(other, religion);
            }

            DiploEvent::NoConvert => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_DENIED_RELIGION, 1);
            }

            DiploEvent::Revolution => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_ACCEPTED_CIVIC, 1);

                let mut new_civics = self.players[other].civics.clone();

                // C++ FAsserts that the leader has a favorite civic and indexes with it unchecked.
                let favorite = self.players[p].personality_type.and_then(|l| self.d.leader_heads[l].favorite_civic);
                if let Some(favorite) = favorite {
                    if let Some(option) = self.d.civics[favorite].civic_option_type {
                        new_civics[option] = Some(favorite);
                    }
                }

                self.player_revolution(other, &new_civics, true);
            }

            DiploEvent::NoRevolution => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_DENIED_CIVIC, 1);
            }

            DiploEvent::JoinWar => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_ACCEPTED_JOIN_WAR, 1);
                let target = data_team(data1);
                let other_team = self.players[other].team;
                self.team_declare_war(other_team, target, false, Some(WarPlanType::WARPLAN_DOGPILE));

                for i in self.player_ids() {
                    if self.players[i].alive && self.players[i].team == target {
                        ai_change_memory_count(self, i, p, MemoryType::MEMORY_HIRED_WAR_ALLY, 1);
                    }
                }
            }

            DiploEvent::NoJoinWar => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_DENIED_JOIN_WAR, 1);
            }

            DiploEvent::StopTrading => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_ACCEPTED_STOP_TRADING, 1);
                let target = data_team(data1);
                self.player_stop_trading_with_team(other, target);

                for i in self.player_ids() {
                    if self.players[i].alive && self.players[i].team == target {
                        ai_change_memory_count(self, i, p, MemoryType::MEMORY_HIRED_TRADE_EMBARGO, 1);
                    }
                }
            }

            DiploEvent::NoStopTrading => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_DENIED_STOP_TRADING, 1);
            }

            DiploEvent::AskHelp => {
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_MADE_DEMAND_RECENT, 1);
            }

            DiploEvent::MadeDemand => {
                if self.players[p].ai_memory_count[other][MemoryType::MEMORY_MADE_DEMAND] < 10 {
                    ai_change_memory_count(self, p, other, MemoryType::MEMORY_MADE_DEMAND, 1);
                }
                ai_change_memory_count(self, p, other, MemoryType::MEMORY_MADE_DEMAND_RECENT, 1);
            }

            DiploEvent::MadeDemandVassal => {}

            DiploEvent::ResearchTech => {
                let tech: TechType = data_index(data1);
                self.player_push_research(p, tech, true);
            }

            DiploEvent::TargetCity => {
                let owner = PlayerId(data1 as u8);
                if let Some(c) = player_city(self, owner, data2) {
                    // CvArea::setTargetCity (AI bookkeeping, plain assignment)
                    let area = self.city_area(c);
                    self.map.areas[area].target_cities[p] = Some(c);
                }
            }
        }
    }

    /// C++ `CvPlayer::isNoDiplomacyWithEnemies` (FFH).
    pub fn player_is_no_diplomacy_with_enemies(&self, p: PlayerId) -> bool {
        self.players[p].no_diplomacy_with_enemies != 0
    }

    /// C++ `CvPlayer::forcePeace(ePlayer)`: implements a peace treaty deal (unless a vassal).
    pub fn player_force_peace(&mut self, p: PlayerId, other: PlayerId) {
        let team = self.players[p].team;
        if !self.team_is_a_vassal(team) {
            // C++ FAssert(GET_TEAM(getTeam()).canChangeWarPeace(GET_PLAYER(ePlayer).getTeam())).
            let trade = set_trade_item(TradeableItem::TRADE_PEACE_TREATY, 0);
            let player_list = vec![trade.clone()];
            let loop_player_list = vec![trade];

            // C++ default `bForce = false`.
            self.implement_deal(p, other, &player_list, &loop_player_list, false);
        }
    }

    // ------------------------------------------------------------------ trade items and deals

    /// C++ `CvPlayer::canReceiveTradeCity` (not for humans in One City Challenge).
    pub fn player_can_receive_trade_city(&self, p: PlayerId) -> bool {
        if self.is_option(GameOptionType::GAMEOPTION_ONE_CITY_CHALLENGE) && self.players[p].human {
            return false;
        }

        true
    }

    /// C++ `CvPlayer::canTradeItem(eWhoTo, item, bTestDenial = false)`.
    pub fn player_can_trade_item(&self, p: PlayerId, who_to: PlayerId, item: &TradeData, test_denial: bool) -> bool {
        if test_denial && self.player_get_trade_denial(p, who_to, item).is_some() {
            return false;
        }

        let team = self.players[p].team;
        let to_team = self.players[who_to].team;

        //FfH: Added by Kael 11/20/2007
        let mut can_trade = true;
        for vote_source in self.d.vote_sources.keys() {
            if self.player_is_full_member(p, vote_source) && self.no_outside_tech_trades[vote_source] && !self.player_is_full_member(who_to, vote_source) {
                can_trade = false;
            }
        }
        //FfH: End Add

        match item.item_type {
            TradeableItem::TRADE_TECHNOLOGIES => {
                if !self.is_option(GameOptionType::GAMEOPTION_NO_TECH_TRADING) {
                    let tech: TechType = data_index(item.data);
                    if self.d.techs[tech].trade {
                        if self.teams[team].has_tech[tech] && !self.teams[team].no_trade_tech[tech]
                        //FfH: Added by Kael 11/20/2007
                            && can_trade
                        //FfH: End Add
                        {
                            if !self.teams[to_team].has_tech[tech] {
                                // (BTS: `if (GET_PLAYER(eWhoTo).isHuman() || (GET_PLAYER(eWhoTo).getCurrentResearch() != item.m_iData))` commented out)
                                if (self.team_is_tech_trading(team) || self.team_is_tech_trading(to_team)) && self.player_can_research(who_to, tech, true) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            TradeableItem::TRADE_RESOURCES => {
                debug_assert!(item.data > -1, "iData is expected to be non-negative");

                if self.player_can_trade_network_with(p, who_to) {
                    let bonus: BonusType = data_index(item.data);
                    if !self.team_is_bonus_obsolete(to_team, bonus) && !self.team_is_bonus_obsolete(team, bonus) {
                        let can_trade_all = self.players[p].human || team == to_team || self.teams[team].vassal[to_team];
                        let keep = if can_trade_all { 0 } else { 1 };
                        if self.player_get_num_tradeable_bonuses(p, bonus) > keep {
                            // (BTS: `if (GET_PLAYER(eWhoTo).getNumAvailableBonuses(eBonus) == 0)` commented out)
                            return true;
                        }
                    }
                }
            }

            TradeableItem::TRADE_CITIES => {
                let city_traded = player_city(self, p, item.data);

                if let Some(c) = city_traded {
                    if self.city_liberation_player(c, false) == Some(who_to) {
                        return true;
                    }
                }

                if self.player_can_receive_trade_city(who_to)
                    && 0 == self.max_city_elimination() && !self.team_is_a_vassal(team) && !self.teams[to_team].vassal[team]
                {
                    // C++ quirk: this branch does not check that the item's city exists or
                    // is ours (only that it is not our capital); CvDeal::startTrade does.
                    if let Some(our_capital) = self.player_get_capital_city(p) {
                        if our_capital.0 as i64 != item.data as i64 {
                            return true;
                        }
                    }
                }
            }

            TradeableItem::TRADE_GOLD => {
                if (self.team_is_gold_trading(team) || self.team_is_gold_trading(to_team)) && self.players[p].gold >= item.data {
                    return true;
                }
            }

            TradeableItem::TRADE_GOLD_PER_TURN => {
                if self.team_is_gold_trading(team) || self.team_is_gold_trading(to_team) {
                    return true;
                }
            }

            TradeableItem::TRADE_MAPS => {
                if team != to_team && (self.team_is_map_trading(team) || self.team_is_map_trading(to_team)) {
                    return true;
                }
            }

            TradeableItem::TRADE_VASSAL | TradeableItem::TRADE_SURRENDER => {
                if !self.players[p].human || self.players[who_to].human {
                    // human can't be vassal of AI
                    let vassal_team = team;
                    let master_team = to_team;
                    if self.team_is_vassal_state_trading(master_team) {
                        // the master must possess the tech
                        if !self.team_is_a_vassal(vassal_team) && !self.team_is_a_vassal(master_team) && team != to_team {
                            if (self.teams[master_team].at_war[team] || item.data == 1) && item.item_type == TradeableItem::TRADE_SURRENDER {
                                return true;
                            }

                            if !self.teams[master_team].at_war[team] && item.item_type == TradeableItem::TRADE_VASSAL {
                                return true;
                            }
                        }
                    }
                }
            }

            TradeableItem::TRADE_PEACE => {
                if !self.team_is_human(team) && !self.team_is_a_vassal(team) {
                    let peace_team = data_team(item.data);
                    if self.teams[team].has_met[peace_team] && self.teams[to_team].has_met[peace_team] && self.at_war(team, peace_team) {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_WAR => {
                if !self.team_is_human(team) && !self.team_is_a_vassal(team) {
                    let war_team = data_team(item.data);
                    if !self.team_is_a_vassal(war_team)
                        && self.teams[team].has_met[war_team] && self.teams[to_team].has_met[war_team] && self.team_can_declare_war(team, war_team)
                    {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_EMBARGO => {
                if !self.team_is_human(team) {
                    let trade_team = data_team(item.data);
                    if self.teams[team].has_met[trade_team] && self.teams[to_team].has_met[trade_team] {
                        // C++ default `bContinueNotTrading = false`.
                        if self.player_can_stop_trading_with_team(p, trade_team, false) {
                            return true;
                        }
                    }
                }
            }

            TradeableItem::TRADE_CIVIC => {
                // UNOFFICIAL_PATCH 10/22/09 (denev & jdog5000): also within a team
                // (original BTS: `if (!(GET_TEAM(getTeam()).isHuman()))`).
                if !self.team_is_human(team) || team == to_team {
                    let civic: CivicType = data_index(item.data);
                    if self.player_is_civic(who_to, civic)
                        && self.player_can_do_civics(p, Some(civic)) && !self.player_is_civic(p, civic) && self.player_can_revolution(p, None)
                    {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_RELIGION => {
                // UNOFFICIAL_PATCH 10/22/09 (denev & jdog5000): also within a team.
                if !self.team_is_human(team) || team == to_team {
                    let religion: Option<ReligionType> = opt_from_i32(item.data);
                    if self.player_get_state_religion(who_to) == religion && self.player_can_convert(p, religion) {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_OPEN_BORDERS => {
                if team != to_team && !self.at_war(team, to_team) {
                    if !self.team_is_open_borders(team, to_team)
                        && (self.team_is_open_borders_trading(team) || self.team_is_open_borders_trading(to_team))
                    {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_DEFENSIVE_PACT => {
                if !self.team_is_a_vassal(team) && !self.team_is_a_vassal(to_team)
                    && team != to_team && !self.teams[to_team].vassal[team] && !self.at_war(team, to_team)
                {
                    if !self.teams[team].defensive_pact[to_team]
                        && (self.team_is_defensive_pact_trading(team) || self.team_is_defensive_pact_trading(to_team))
                    {
                        if self.team_get_at_war_count(team, true) == 0 && self.team_get_at_war_count(to_team, true) == 0
                            && self.team_can_sign_defensive_pact(team, to_team)
                        {
                            return true;
                        }
                    }
                }
            }

            TradeableItem::TRADE_PERMANENT_ALLIANCE => {
                if !self.team_is_a_vassal(team) && !self.team_is_a_vassal(to_team)
                    && team != to_team && !self.teams[to_team].vassal[team] && !self.at_war(team, to_team)
                {
                    if (self.team_is_permanent_alliance_trading(team) || self.team_is_permanent_alliance_trading(to_team))
                        && self.teams[team].num_members == 1 && self.teams[to_team].num_members == 1
                    {
                        return true;
                    }
                }
            }

            TradeableItem::TRADE_PEACE_TREATY => {
                // Not in C++: no separate peace while one's units serve a Host against the other
                // (docs/CRUSADE.md).
                if !self.crusades.is_empty() && (self.crusade_serving_against(p, to_team) || self.crusade_serving_against(who_to, team)) {
                    return false;
                }
                return true;
            }

            _ => {}
        }

        false
    }

    /// C++ `CvPlayer::getTradeDenial(eWhoTo, item)` (None = NO_DENIAL).
    ///
    /// The denials are the C++ AI's `AI_*Trade` judgements. Ported: the checks that follow from
    /// the rules (war/peace feasibility of vassal/surrender deals, peace or war with a third
    /// team). Not ported (`// AI:`, read as no denial): the AI's own judgement -- attitudes,
    /// worst enemy, valuations, power, victory threats. An AI's willingness to accept a deal is
    /// `Controller::consider_offer`.
    pub fn player_get_trade_denial(&self, p: PlayerId, who_to: PlayerId, item: &TradeData) -> Option<DenialType> {
        let team = self.players[p].team;
        let to_team = self.players[who_to].team;

        match item.item_type {
            TradeableItem::TRADE_TECHNOLOGIES => {
                // AI: GET_TEAM(getTeam()).AI_techTrade(eTech, eToTeam) -- the receiving AI's gain under
                // NO_TECH_BROKERING, worst enemy, attitude thresholds, tech whoring/monopoly, wonder
                // races and victory techs (human teams: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_RESOURCES => {
                // AI: AI_bonusTrade(eBonus, eWhoTo) -- the receiver already has it (corporation value),
                // worst enemy, strategic/happiness/health bonus attitude thresholds.
                return None;
            }

            TradeableItem::TRADE_CITIES => {
                if player_city(self, p, item.data).is_some() {
                    // AI: AI_cityTrade(pCity, eWhoTo) -- liberation is always fine; AI receivers refuse
                    // far or culturally foreign cities (financial trouble, nearest city > 9 plots);
                    // AI givers never give cities to other teams in peace (or > 50% culture).
                    return None;
                }
            }

            TradeableItem::TRADE_GOLD | TradeableItem::TRADE_GOLD_PER_TURN => {}

            TradeableItem::TRADE_MAPS => {
                // AI: GET_TEAM(getTeam()).AI_mapTrade(eToTeam) -- worst enemy, attitude threshold
                // (human teams: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_SURRENDER => {
                return ai_surrender_trade(self, team, to_team, 140);
            }

            TradeableItem::TRADE_VASSAL => {
                return ai_vassal_trade(self, team, to_team);
            }

            TradeableItem::TRADE_PEACE => {
                return ai_make_peace_trade(self, team, data_team(item.data), to_team);
            }

            TradeableItem::TRADE_WAR => {
                // C++ default `bConsiderPower = true`.
                return ai_declare_war_trade(self, team, data_team(item.data), to_team, true);
            }

            TradeableItem::TRADE_EMBARGO => {
                // AI: AI_stopTradingTrade(eTradeTeam, eWhoTo) -- vassal relations, attitude
                // thresholds towards both teams (human players: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_CIVIC => {
                // AI: AI_civicTrade(eCivic, eWhoTo) -- civic anger, favorite civic, initial civics,
                // attitude threshold (human players: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_RELIGION => {
                // AI: AI_religionTrade(eReligion, eWhoTo) -- minority religion, attitude threshold
                // (human players: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_OPEN_BORDERS => {
                // AI: GET_TEAM(getTeam()).AI_openBordersTrade(eToTeam) -- recent cancel memory, worst
                // enemy, attitude threshold (human teams: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_DEFENSIVE_PACT => {
                // AI: GET_TEAM(getTeam()).AI_defensivePactTrade(eToTeam) -- two teams left, worst
                // enemy, attitude threshold (human teams: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_PERMANENT_ALLIANCE => {
                // AI: GET_TEAM(getTeam()).AI_permanentAllianceTrade(eToTeam) -- worst enemy, power,
                // alliance counters, attitude threshold (human teams: NO_DENIAL).
                return None;
            }

            TradeableItem::TRADE_PEACE_TREATY => {}

            _ => {}
        }

        None
    }

    /// C++ `CvPlayer::isTradingWithTeam(eTeam, bIncludeCancelable)`.
    pub fn player_is_trading_with_team(&self, p: PlayerId, team: TeamId, include_cancelable: bool) -> bool {
        if team == self.players[p].team {
            return false;
        }

        for (d, deal) in self.deals.iter() {
            if (include_cancelable || self.deal_is_cancelable(d, Some(p))) && !self.deal_is_peace_deal(d) {
                if deal.first_player == p && self.players[deal.second_player].team == team && !deal.first_trades.is_empty() {
                    return true;
                }

                if deal.second_player == p && self.players[deal.first_player].team == team && !deal.second_trades.is_empty() {
                    return true;
                }
            }
        }

        false
    }

    /// C++ `CvPlayer::canStopTradingWithTeam(eTeam, bContinueNotTrading = false)`.
    pub fn player_can_stop_trading_with_team(&self, p: PlayerId, team: TeamId, continue_not_trading: bool) -> bool {
        let our_team = self.players[p].team;

        if team == our_team {
            return false;
        }

        if self.teams[our_team].vassal[team] {
            return false;
        }

        if !self.player_is_trading_with_team(p, team, false) {
            if continue_not_trading && !self.player_is_trading_with_team(p, team, true) {
                return true;
            }

            return false;
        }

        true
    }

    /// C++ `CvPlayer::stopTradingWithTeam(eTeam)`: cancels deals and records the memories.
    pub fn player_stop_trading_with_team(&mut self, p: PlayerId, team: TeamId) {
        debug_assert!(team != self.players[p].team);

        // C++ iterates the game's deal list; a killed deal can kill others (CvDeal::endTeamTrade),
        // which are then no longer visited.
        for d in self.deals.ids() {
            if !self.deals.contains(d) {
                continue;
            }

            if self.deal_is_cancelable(d, Some(p)) && !self.deal_is_peace_deal(d) {
                let first = self.deals[d].first_player;
                let second = self.deals[d].second_player;
                if (first == p && self.players[second].team == team) || (second == p && self.players[first].team == team) {
                    // C++ default `bKillTeam = true`.
                    self.deal_kill(d, true);
                }
            }
        }

        for i in self.player_ids() {
            if self.players[i].alive && self.players[i].team == team {
                ai_change_memory_count(self, i, p, MemoryType::MEMORY_STOPPED_TRADING, 1);
                ai_change_memory_count(self, i, p, MemoryType::MEMORY_STOPPED_TRADING_RECENT, 1);
            }
        }
    }

    /// C++ `CvPlayer::killAllDeals`.
    pub fn player_kill_all_deals(&mut self, p: PlayerId) {
        for d in self.deals.ids() {
            // a killed deal can kill others (CvDeal::endTeamTrade)
            if !self.deals.contains(d) {
                continue;
            }

            if self.deals[d].first_player == p || self.deals[d].second_player == p {
                // C++ default `bKillTeam = true`.
                self.deal_kill(d, true);
            }
        }
    }

    // ------------------------------------------------------------------ bonuses

    /// C++ `CvPlayer::canTradeNetworkWith(ePlayer)` (capitals connected).
    pub fn player_can_trade_network_with(&self, p: PlayerId, other: PlayerId) -> bool {
        if let Some(our_capital) = self.player_get_capital_city(p) {
            if self.city_is_connected_to_capital(our_capital, Some(other)) {
                return true;
            }
        }

        false
    }

    /// C++ `CvPlayer::getNumAvailableBonuses(eBonus)` (capital plot group; FFH: banned by the
    /// first vote source's `isNoBonus` for full members).
    pub fn player_get_num_available_bonuses(&self, p: PlayerId, bonus: BonusType) -> i32 {
        let plot_group = match self.player_get_capital_city(p) {
            Some(capital) => self.plot_get_owner_plot_group(self.city_plot(capital)),
            None => None,
        };

        //FfH: Added by Kael 11/14/2007
        if self.player_is_full_member(p, VoteSourceType(0)) && self.no_bonus[bonus] {
            return 0;
        }
        //FfH: End Add

        if let Some(pg) = plot_group {
            return self.plot_groups[pg].num_bonuses[bonus];
        }

        0
    }

    /// C++ `CvPlayer::getNumTradeableBonuses(eBonus)`.
    pub fn player_get_num_tradeable_bonuses(&self, p: PlayerId, bonus: BonusType) -> i32 {
        self.player_get_num_available_bonuses(p, bonus) - self.players[p].bonus_import[bonus]
    }

    /// C++ `CvPlayer::hasBonus(eBonus)` (any city has it).
    pub fn player_has_bonus(&self, p: PlayerId, bonus: BonusType) -> bool {
        for &c in &self.players[p].cities {
            if self.city_has_bonus(c, bonus) {
                return true;
            }
        }

        false
    }

    /// C++ `CvPlayer::getNumTradeBonusImports(ePlayer)` (resources traded from `other` in deals).
    pub fn player_get_num_trade_bonus_imports(&self, p: PlayerId, other: PlayerId) -> i32 {
        debug_assert!(other != p);

        let mut count = 0;

        for (_, deal) in self.deals.iter() {
            if deal.first_player == p && deal.second_player == other {
                for trade in &deal.second_trades {
                    if trade.item_type == TradeableItem::TRADE_RESOURCES {
                        count += 1;
                    }
                }
            }

            if deal.first_player == other && deal.second_player == p {
                for trade in &deal.first_trades {
                    if trade.item_type == TradeableItem::TRADE_RESOURCES {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    /// C++ `CvPlayer::countOwnedBonuses(eBonus)`.
    pub fn player_count_owned_bonuses(&self, p: PlayerId, bonus: BonusType) -> i32 {
        let team = self.players[p].team;

        let mut count = 0;

        // count bonuses outside city radius
        for pl in self.map.plot_ids() {
            if self.map.plots[pl].owner == Some(p) && !self.plot_is_city_radius(pl) && self.plot_get_bonus_type(pl, Some(team)) == Some(bonus) {
                count += 1;
            }
        }

        // count bonuses inside city radius or easily claimed
        for &c in &self.players[p].cities {
            let include_neutral = self.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) > 0;
            // C++ defaults `bLand = true`, `bWater = true`.
            count += ai_count_num_bonuses(self, c, Some(bonus), true, include_neutral, -1, true, true);
        }

        count
    }

    /// C++ `CvPlayer::countUnimprovedBonuses(pArea, pFromPlot = NULL)`.
    pub fn player_count_unimproved_bonuses(&self, p: PlayerId, area: AreaId, from_plot: Option<PlotIdx>) -> i32 {
        let d = self.d;
        let team = self.players[p].team;

        // The plots reachable from `from_plot` through the player's territory (C++ searches the
        // border finder per plot, reusing the tree; one flood fill gives the same answers).
        let reachable_from: Option<Vec<bool>> = from_plot.map(|from| {
            let (from_x, from_y) = self.map.xy(from);
            self.with_finder(FinderKind::Border, |finder| {
                finder.generate_path(self, from_x, from_y, -1, -1, false, p.i());
                let mut reachable = vec![false; self.map.plots.len()];
                for node in finder.closed_nodes() {
                    if let Some(pl) = self.map.plot_idx(node.x, node.y) {
                        reachable[pl.idx()] = true;
                    }
                }
                reachable
            })
        });

        let mut count = 0;

        for pl in self.map.plot_ids() {
            if self.map.plots[pl].area == Some(area) && self.map.plots[pl].owner == Some(p) && !self.plot_is_city(pl, false, None) {
                if let Some(non_obsolete_bonus) = self.plot_get_non_obsolete_bonus_type(pl, team) {
                    let improvement = self.map.plots[pl].improvement_type;

                    if improvement.map_or(true, |i| !d.improvements[i].improvement_bonus[non_obsolete_bonus].bonus_trade) {
                        let reachable = reachable_from.as_ref().map_or(true, |r| r[pl.idx()]);

                        if reachable {
                            for build in d.builds.keys() {
                                if let Some(build_improvement) = d.builds[build].improvement {
                                    if d.improvements[build_improvement].improvement_bonus[non_obsolete_bonus].bonus_trade {
                                        // C++ defaults `bTestEra = false`, `bTestVisible = false`.
                                        if self.player_can_build(p, pl, build, false) {
                                            count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `CvPlayer::changeBonusExport` (capital plot group bonuses are recounted).
    pub fn player_change_bonus_export(&mut self, p: PlayerId, bonus: BonusType, change: i32) {
        if change != 0 {
            let capital_city = self.player_get_capital_city(p);
            if let Some(capital) = capital_city {
                let plot = self.city_plot(capital);
                self.plot_update_plot_group_bonus(plot, false);
            }
            self.players[p].bonus_export[bonus] += change;
            debug_assert!(self.players[p].bonus_export[bonus] >= 0);
            if let Some(capital) = capital_city {
                let plot = self.city_plot(capital);
                self.plot_update_plot_group_bonus(plot, true);
            }
        }
    }

    /// C++ `CvPlayer::changeBonusImport` (capital plot group bonuses are recounted).
    pub fn player_change_bonus_import(&mut self, p: PlayerId, bonus: BonusType, change: i32) {
        if change != 0 {
            let capital_city = self.player_get_capital_city(p);
            if let Some(capital) = capital_city {
                let plot = self.city_plot(capital);
                self.plot_update_plot_group_bonus(plot, false);
            }
            self.players[p].bonus_import[bonus] += change;
            debug_assert!(self.players[p].bonus_import[bonus] >= 0);
            if let Some(capital) = capital_city {
                let plot = self.city_plot(capital);
                self.plot_update_plot_group_bonus(plot, true);
            }
        }
    }

    // ------------------------------------------------------------------ foreign trade routes

    /// C++ `CvPlayer::countPotentialForeignTradeCities(pIgnoreArea = NULL)`.
    pub fn player_count_potential_foreign_trade_cities(&self, p: PlayerId, ignore_area: Option<AreaId>) -> i32 {
        let team = self.players[p].team;
        let mut count = 0;

        for o in self.civ_team_ids() {
            if self.team_is_alive(o) && o != team && self.team_is_free_trade(team, o) {
                let mut temp_value = self.teams[o].num_cities;

                if let Some(area) = ignore_area {
                    temp_value -= self.team_count_num_cities_by_area(o, area);
                }

                count += temp_value;
            }
        }

        count
    }

    /// C++ `CvPlayer::countPotentialForeignTradeCitiesConnected`.
    pub fn player_count_potential_foreign_trade_cities_connected(&self, p: PlayerId) -> i32 {
        let team = self.players[p].team;
        let mut count = 0;

        if let Some(capital_city) = self.player_get_capital_city(p) {
            let capital_plot_group = self.city_plot_group(capital_city, p);

            for i in self.civ_player_ids() {
                if self.players[i].alive {
                    let other_team = self.players[i].team;
                    if other_team != team && self.team_is_free_trade(team, other_team) {
                        for &c in &self.players[i].cities {
                            debug_assert!(self.cities[c].owner != p);

                            // C++ compares plot group pointers: two NULL groups are equal too.
                            if self.city_plot_group(c, p) == capital_plot_group {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `CvPlayer::canHaveTradeRoutesWith(ePlayer)`.
    pub fn player_can_have_trade_routes_with(&self, p: PlayerId, other: PlayerId) -> bool {
        if !self.players[other].alive {
            return false;
        }

        let team = self.players[p].team;
        let other_team = self.players[other].team;

        if team == other_team {
            return true;
        }

        if self.team_is_free_trade(team, other_team) {
            if self.teams[team].vassal[other_team] {
                return true;
            }

            if self.teams[other_team].vassal[team] {
                return true;
            }

            if !self.player_is_no_foreign_trade(p) && !self.player_is_no_foreign_trade(other) {
                return true;
            }
        }

        false
    }

    // ------------------------------------------------------------------ CvPlayerAI bookkeeping used by deals

    /// C++ `CvPlayerAI::AI_changePeacetimeTradeValue(eIndex, iChange)`: also raises the
    /// grudge (`AI_changeEnemyPeacetimeTradeValue`) of teams whose worst enemy is `p`'s team
    /// and who have met `other`'s team.
    pub fn player_ai_change_peacetime_trade_value(&mut self, p: PlayerId, other: PlayerId, change: i32) {
        if change != 0 {
            self.players[p].ai_peacetime_trade_value[other] += change;
            debug_assert!(self.players[p].ai_peacetime_trade_value[other] >= 0);

            debug_assert!(change > 0);

            if change > 0 {
                let team = self.players[p].team;
                let other_team = self.players[other].team;

                if other_team != team {
                    for o in self.civ_team_ids() {
                        if self.team_is_alive(o) && self.teams[o].ai_worst_enemy == Some(team) {
                            // BETTER AI (Better Diplomatics, Sephi): only if that team has met
                            // the trading partner (orig: unconditional).
                            if self.teams[o].has_met[other_team] {
                                // CvTeamAI::AI_changeEnemyPeacetimeTradeValue
                                self.teams[o].ai_enemy_peacetime_trade_value[other_team] += change;
                            }
                        }
                    }
                }
            }
        }
    }

    /// C++ `CvPlayerAI::AI_changePeacetimeGrantValue(eIndex, iChange)` (same team side effect
    /// with `AI_changeEnemyPeacetimeGrantValue`).
    pub fn player_ai_change_peacetime_grant_value(&mut self, p: PlayerId, other: PlayerId, change: i32) {
        if change != 0 {
            self.players[p].ai_peacetime_grant_value[other] += change;
            debug_assert!(self.players[p].ai_peacetime_grant_value[other] >= 0);

            debug_assert!(change > 0);

            if change > 0 {
                let team = self.players[p].team;
                let other_team = self.players[other].team;

                if other_team != team {
                    for o in self.civ_team_ids() {
                        if self.team_is_alive(o) && self.teams[o].ai_worst_enemy == Some(team) {
                            // BETTER AI (Better Diplomatics, Sephi): only if that team has met
                            // the trading partner (orig: unconditional).
                            if self.teams[o].has_met[other_team] {
                                // CvTeamAI::AI_changeEnemyPeacetimeGrantValue
                                self.teams[o].ai_enemy_peacetime_grant_value[other_team] += change;
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- helpers

/// The typed index a trade item's `m_iData` (or a diplo event's data) holds.
fn data_index<I: Idx>(data: i32) -> I {
    debug_assert!(data >= 0, "invalid index {}", data);
    I::from_usize(data as usize)
}

/// The team a trade item's `m_iData` (or a diplo event's data) holds.
fn data_team(data: i32) -> TeamId {
    debug_assert!(data >= 0, "invalid team {}", data);
    TeamId(data as u8)
}

/// C++ `GET_PLAYER(p).getCity(iID)`: the city with this id if `p` owns it (NULL otherwise;
/// C++ city ids are per player).
fn player_city(g: &Game, p: PlayerId, id: i32) -> Option<CityId> {
    if id < 0 {
        return None;
    }
    let c = CityId(id as u32);
    match g.cities.get(c) {
        Some(city) if city.owner == p => Some(c),
        _ => None,
    }
}

/// C++ `CvPlayerAI::AI_changeMemoryCount(eIndex1, eIndex2, iChange)` (AI bookkeeping).
fn ai_change_memory_count(g: &mut Game, p: PlayerId, other: PlayerId, memory: MemoryType, change: i32) {
    g.players[p].ai_memory_count[other][memory] += change;
    debug_assert!(g.players[p].ai_memory_count[other][memory] >= 0);
}

/// C++ `CvPlayerAI::AI_demandRebukedSneak(ePlayer)`: after `other` rejected our demand, plan a
/// sneak attack? One game RNG draw.
fn ai_demand_rebuked_sneak(g: &mut Game, p: PlayerId, other: PlayerId) -> bool {
    // C++ FAsserts: not human, not a vassal, team not human.
    // Operands of `<` are evaluated left to right: the draw comes first.
    let roll = g.soren_rand_num(100, "AI Demand Rebuked");
    // C++ reads GC.getLeaderHeadInfo(getPersonalityType()) unchecked.
    let probability = g.players[p].personality_type.map_or(0, |l| g.d.leader_heads[l].demand_rebuked_sneak_prob);
    if roll < probability {
        let team = g.players[p].team;
        let other_team = g.players[other].team;
        if g.team_get_power(team, true) > g.team_get_defensive_power(other_team) {
            return true;
        }
    }

    false
}

/// C++ `CvCityAI::AI_countNumBonuses(eBonus, bIncludeOurs, bIncludeNeutral, iOtherCultureThreshold,
/// bLand = true, bWater = true)`: bonuses in the city's radius on its area (or on water) that
/// are worked by it, unowned, or owned by others with a culture lead below the threshold.
/// (`land` is not read by the C++ body.)
#[allow(clippy::too_many_arguments)]
fn ai_count_num_bonuses(
    g: &Game,
    c: CityId,
    bonus: Option<BonusType>,
    include_ours: bool,
    include_neutral: bool,
    other_culture_threshold: i32,
    _land: bool,
    water: bool,
) -> i32 {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let owner = g.cities[c].owner;
    let team = g.city_team(c);
    let area = g.city_area(c);

    let mut count = 0;
    for i in 0..NUM_CITY_PLOTS {
        if let Some(pl) = g.plot_city(x, y, i) {
            if g.map.plots[pl].area == Some(area) || (water && g.plot_is_water(pl)) {
                if let Some(loop_bonus) = g.plot_get_bonus_type(pl, Some(team)) {
                    if bonus.map_or(true, |b| b == loop_bonus) {
                        let plot_owner = g.map.plots[pl].owner;
                        if include_ours && plot_owner == Some(owner) && g.map.plots[pl].working_city == Some(c) {
                            count += 1;
                        } else if include_neutral && plot_owner.is_none() {
                            count += 1;
                        } else if other_culture_threshold > 0 {
                            if let Some(plot_owner) = plot_owner {
                                if plot_owner != owner {
                                    let culture = &g.map.plots[pl].culture;
                                    if culture[plot_owner] - culture[owner] < other_culture_threshold {
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    count
}

/// C++ `CvTeamAI::AI_surrenderTrade(eTeam, iPowerMultiplier = 100)` for team `t` becoming the
/// capitulated vassal of `master`. Ported: `t` must be able to follow `master` into its wars
/// and out of its peace (checked for human teams too). AI: the rest (power and war success,
/// land targets, victory threats, worst enemy, attitude) -- no denial.
fn ai_surrender_trade(g: &Game, t: TeamId, master: TeamId, _power_multiplier: i32) -> Option<DenialType> {
    for o in g.team_ids() {
        if g.team_is_alive(o) && o != t && o != master {
            if g.teams[o].at_war[master] && !g.teams[o].at_war[t] {
                if (g.teams[t].force_peace[o] || !g.team_can_change_war_peace(t, o, false)) && !g.team_is_a_vassal(o) {
                    return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_US);
                }
            } else if !g.teams[o].at_war[master] && g.teams[o].at_war[t] && !g.team_can_change_war_peace(t, o, false) && !g.team_is_a_vassal(o) {
                return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_US);
            }
        }
    }

    if g.team_is_human(t) {
        return None;
    }

    // AI: CvTeamAI::AI_surrenderTrade's judgement for AI teams (vassal power vs. master and
    // average power, war success, AI_isLandTarget, victory threats, worst enemy, distance,
    // attitude) is not ported: no denial.
    None
}

/// C++ `CvTeamAI::AI_vassalTrade(eTeam)` for team `t` becoming the vassal of `master`: the master
/// must be able to take over `t`'s wars and peace, then `AI_surrenderTrade(eTeam)`.
fn ai_vassal_trade(g: &Game, t: TeamId, master: TeamId) -> Option<DenialType> {
    for o in g.team_ids() {
        if g.team_is_alive(o) && o != t && o != master {
            if !g.teams[o].at_war[master] && g.teams[o].at_war[t] {
                if (g.teams[master].force_peace[o] || !g.team_can_change_war_peace(master, o, false)) && !g.team_is_a_vassal(o) {
                    return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_YOU);
                }

                if !g.team_is_human(master) {
                    // C++ default `bConsiderPower = true`.
                    let war_denial = ai_declare_war_trade(g, master, o, t, true);
                    if war_denial.is_some() {
                        return Some(DenialType::DENIAL_WAR_NOT_POSSIBLE_YOU);
                    }
                }
            } else if g.teams[o].at_war[master] && !g.teams[o].at_war[t] {
                if !g.team_can_change_war_peace(master, o, false) && !g.team_is_a_vassal(o) {
                    return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_YOU);
                }

                if !g.team_is_human(master) {
                    let peace_denial = ai_make_peace_trade(g, master, o, t);
                    if peace_denial.is_some() {
                        return Some(DenialType::DENIAL_PEACE_NOT_POSSIBLE_YOU);
                    }
                }
            }
        }
    }

    // C++ default `iPowerMultiplier = 100`.
    ai_surrender_trade(g, t, master, 100)
}

/// C++ `CvTeamAI::AI_makePeaceTrade(ePeaceTeam, eTeam)`: team `t` making peace with `peace_team`
/// for `team`. Ported: humans must be contacted directly, vassals cannot make peace on their
/// own, and `t` must be able to change war/peace. AI: the war values and land ratio.
fn ai_make_peace_trade(g: &Game, t: TeamId, peace_team: TeamId, team: TeamId) -> Option<DenialType> {
    debug_assert!(team != t, "shouldn't call this function on ourselves");
    debug_assert!(peace_team != t, "shouldn't call this function on ourselves");

    if g.team_is_human(peace_team) {
        return Some(DenialType::DENIAL_CONTACT_THEM);
    }

    if g.team_is_a_vassal(peace_team) {
        return Some(DenialType::DENIAL_VASSAL);
    }

    if g.team_is_human(t) {
        return None;
    }

    // C++ default `bAllowVassal = false`.
    if !g.team_can_change_war_peace(t, peace_team, false) {
        return Some(DenialType::DENIAL_VASSAL);
    }

    // AI: `AI_endWarVal(ePeaceTeam) > GET_TEAM(ePeaceTeam).AI_endWarVal(getID()) * 2` ->
    // DENIAL_CONTACT_THEM and a land ratio over 250% -> DENIAL_VICTORY (the AI team's own
    // judgement) are not ported: no denial.
    None
}

/// C++ `CvTeamAI::AI_declareWarTrade(eWarTeam, eTeam, bConsiderPower = true)`: team `t` declaring
/// war on `war_team` for `team`. Ported: not on `team`'s own vassal or defensive pact partner,
/// and `t` must be able to declare war. AI: war plans, power, attitudes, nukes.
fn ai_declare_war_trade(g: &Game, t: TeamId, war_team: TeamId, team: TeamId, _consider_power: bool) -> Option<DenialType> {
    debug_assert!(team != t, "shouldn't call this function on ourselves");
    debug_assert!(war_team != t, "shouldn't call this function on ourselves");

    if g.teams[war_team].vassal[team] || g.teams[war_team].defensive_pact[team] {
        return Some(DenialType::DENIAL_JOKING);
    }

    if g.team_is_human(t) {
        return None;
    }

    if !g.team_can_declare_war(t, war_team) {
        return Some(DenialType::DENIAL_VASSAL);
    }

    // AI: the AI team's own judgement (DENIAL_TOO_MANY_WARS with any war plan, defensive power
    // vs. AI_isAllyLandTarget, attitude thresholds towards both teams, enemy nukes) is not
    // ported: no denial.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deal::Deal;
    use crate::player::Player;
    use crate::setup::GameSetup;

    /// A game whose first `n` slots hold freshly reset players (C++ `reset`), alive on their
    /// own teams.
    fn game(n: usize) -> Game {
        let d = ffh_data::global();
        let setup = GameSetup::standard(d, n, "WORLDSIZE_DUEL", 7);
        let mut g = Game::new(d, setup);
        let (n_players, n_teams) = (g.num_players(), g.num_teams());
        for i in 0..n {
            let p = PlayerId(i as u8);
            g.players[p] = Player::new(d, p, &g.setup.players[i], n_players, n_teams);
            g.players[p].alive = true;
            g.players[p].ever_alive = true;
            let t = g.players[p].team;
            g.teams[t].num_members = 1;
            g.teams[t].alive_count = 1;
            g.teams[t].ever_alive_count = 1;
        }
        g
    }

    fn meet(g: &mut Game, a: PlayerId, b: PlayerId) {
        let (ta, tb) = (g.players[a].team, g.players[b].team);
        g.teams[ta].has_met[tb] = true;
        g.teams[tb].has_met[ta] = true;
    }

    /// A gold-per-turn deal given by `first` (or by `second`), cancelable or signed this turn.
    fn gold_deal(g: &mut Game, first: PlayerId, second: PlayerId, first_gives: bool, cancelable: bool) -> DealId {
        let deal = g.deals.insert_with(|id| Deal::new(id, first, second));
        let item = set_trade_item(TradeableItem::TRADE_GOLD_PER_TURN, 5);
        if first_gives {
            g.deals[deal].first_trades.push(item);
        } else {
            g.deals[deal].second_trades.push(item);
        }
        // C++ turnsToCancel = initial turn + PEACE_TREATY_LENGTH - game turn
        let length = g.d.defines.peace_treaty_length;
        g.deals[deal].initial_game_turn = g.game_turn() - if cancelable { length } else { 0 };
        deal
    }

    #[test]
    fn can_contact_needs_met_living_players_and_ffh_conditions() {
        let mut g = game(2);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        assert_ne!(t0, t1);

        assert!(!g.player_can_contact(p0, p0));
        assert!(!g.player_can_contact(p0, p1), "not met");
        meet(&mut g, p0, p1);
        assert!(g.player_can_contact(p0, p1));

        g.teams[t0].at_war[t1] = true;
        g.teams[t1].at_war[t0] = true;
        assert!(g.player_can_contact(p0, p1));
        // FfH: no diplomacy with enemies
        g.players[p0].no_diplomacy_with_enemies = 1;
        assert!(!g.player_can_contact(p0, p1));
        assert!(g.player_can_contact(p1, p0));
        // permanent war: war and peace cannot change
        g.teams[t0].permanent_war_peace[t1] = true;
        assert!(!g.player_can_contact(p1, p0));
        g.teams[t0].permanent_war_peace[t1] = false;
        // FfH: temporary players cannot contact anyone
        g.players[p1].temp_player_timer = 2;
        assert!(!g.player_can_contact(p1, p0));
        g.players[p1].temp_player_timer = 0;
        // dead players and barbarians
        g.players[p1].alive = false;
        assert!(!g.player_can_contact(p0, p1));
        let barbarian = g.barbarian_player();
        g.players[barbarian].alive = true;
        assert!(!g.player_can_contact(p0, barbarian));
    }

    #[test]
    fn diplo_events_record_first_contact_and_memories() {
        let mut g = game(2);
        let (p0, p1) = (PlayerId(0), PlayerId(1));

        g.player_handle_diplo_event(p0, DiploEvent::Contact, p1, -1, -1);
        assert!(g.players[p0].ai_first_contact[p1] && g.players[p1].ai_first_contact[p0]);

        g.player_handle_diplo_event(p0, DiploEvent::RefusedHelp, p1, -1, -1);
        g.player_handle_diplo_event(p0, DiploEvent::NoConvert, p1, -1, -1);
        g.player_handle_diplo_event(p0, DiploEvent::NoStopTrading, p1, -1, -1);
        let memory = |g: &Game, m: MemoryType| g.players[p0].ai_memory_count[p1][m];
        assert_eq!(memory(&g, MemoryType::MEMORY_REFUSED_HELP), 1);
        assert_eq!(memory(&g, MemoryType::MEMORY_DENIED_RELIGION), 1);
        assert_eq!(memory(&g, MemoryType::MEMORY_DENIED_STOP_TRADING), 1);

        // MADE_DEMAND saturates at 10, MADE_DEMAND_RECENT does not
        for _ in 0..12 {
            g.player_handle_diplo_event(p0, DiploEvent::MadeDemand, p1, -1, -1);
        }
        g.player_handle_diplo_event(p0, DiploEvent::AskHelp, p1, -1, -1);
        g.player_handle_diplo_event(p0, DiploEvent::MadeDemandVassal, p1, -1, -1);
        assert_eq!(memory(&g, MemoryType::MEMORY_MADE_DEMAND), 10);
        assert_eq!(memory(&g, MemoryType::MEMORY_MADE_DEMAND_RECENT), 13);
        assert_eq!(g.players[p1].ai_memory_count[p0][MemoryType::MEMORY_MADE_DEMAND_RECENT], 0);
    }

    #[test]
    fn trading_with_team_stopping_and_killing_deals() {
        let mut g = game(3);
        let (p0, p1, p2) = (PlayerId(0), PlayerId(1), PlayerId(2));
        let (t0, t1, t2) = (g.players[p0].team, g.players[p1].team, g.players[p2].team);
        assert!(!g.player_is_trading_with_team(p0, t1, true));

        // p1 gives gold to p0: p0 gives nothing to p1's team
        let d1 = gold_deal(&mut g, p0, p1, false, true);
        assert!(!g.player_is_trading_with_team(p0, t1, true));
        assert!(g.player_is_trading_with_team(p1, t0, true));
        assert!(!g.player_is_trading_with_team(p1, t1, true), "own team");

        // p0 gives gold to p1 in a deal signed this turn: counts only with uncancelable deals
        let d2 = gold_deal(&mut g, p1, p0, false, false);
        assert!(g.player_is_trading_with_team(p0, t1, true));
        assert!(!g.player_is_trading_with_team(p0, t1, false));
        assert!(!g.player_can_stop_trading_with_team(p0, t1, false));
        assert!(!g.player_can_stop_trading_with_team(p0, t1, true));
        assert!(!g.player_can_stop_trading_with_team(p0, t2, false));
        assert!(g.player_can_stop_trading_with_team(p0, t2, true), "may continue not trading");
        assert!(!g.player_can_stop_trading_with_team(p0, t0, true));

        // once cancelable, stopping kills every cancelable deal with the team and p1 remembers
        g.deals[d2].initial_game_turn = g.game_turn() - g.d.defines.peace_treaty_length;
        assert!(g.player_can_stop_trading_with_team(p0, t1, false));
        let d3 = gold_deal(&mut g, p0, p2, true, true);
        g.player_stop_trading_with_team(p0, t1);
        assert!(!g.deals.contains(d1) && !g.deals.contains(d2));
        assert!(g.deals.contains(d3));
        assert_eq!(g.players[p1].ai_memory_count[p0][MemoryType::MEMORY_STOPPED_TRADING], 1);
        assert_eq!(g.players[p1].ai_memory_count[p0][MemoryType::MEMORY_STOPPED_TRADING_RECENT], 1);
        assert_eq!(g.players[p2].ai_memory_count[p0][MemoryType::MEMORY_STOPPED_TRADING], 0);

        g.player_kill_all_deals(p1);
        assert!(g.deals.contains(d3));
        g.player_kill_all_deals(p2);
        assert!(g.deals.is_empty());
    }

    #[test]
    fn trade_bonus_imports_count_resources_received_in_deals() {
        let mut g = game(2);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let resource = set_trade_item(TradeableItem::TRADE_RESOURCES, 0);
        let gold = set_trade_item(TradeableItem::TRADE_GOLD_PER_TURN, 3);

        let a = g.deals.insert_with(|id| Deal::new(id, p0, p1));
        g.deals[a].first_trades = vec![resource.clone()];
        g.deals[a].second_trades = vec![resource.clone(), resource.clone(), gold.clone()];
        let b = g.deals.insert_with(|id| Deal::new(id, p1, p0));
        g.deals[b].first_trades = vec![resource];
        g.deals[b].second_trades = vec![gold];

        assert_eq!(g.player_get_num_trade_bonus_imports(p0, p1), 3);
        assert_eq!(g.player_get_num_trade_bonus_imports(p1, p0), 1);
    }

    #[test]
    fn peacetime_values_anger_teams_whose_worst_enemy_traded() {
        let mut g = game(3);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let (t0, t1, t2) = (g.players[p0].team, g.players[p1].team, g.players[PlayerId(2)].team);
        g.teams[t2].ai_worst_enemy = Some(t0);

        g.player_ai_change_peacetime_trade_value(p0, p1, 5);
        assert_eq!(g.players[p0].ai_peacetime_trade_value[p1], 5);
        // BETTER AI: only teams that have met the trading partner hold a grudge
        assert_eq!(g.teams[t2].ai_enemy_peacetime_trade_value[t1], 0);

        g.teams[t2].has_met[t1] = true;
        g.player_ai_change_peacetime_trade_value(p0, p1, 3);
        g.player_ai_change_peacetime_grant_value(p0, p1, 4);
        assert_eq!(g.players[p0].ai_peacetime_trade_value[p1], 8);
        assert_eq!(g.players[p0].ai_peacetime_grant_value[p1], 4);
        assert_eq!(g.teams[t2].ai_enemy_peacetime_trade_value[t1], 3);
        assert_eq!(g.teams[t2].ai_enemy_peacetime_grant_value[t1], 4);
        assert_eq!(g.teams[t1].ai_enemy_peacetime_trade_value[t1], 0);
    }
}
