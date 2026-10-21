//! Port of CvPlayerAI.cpp:12386-16739 (FFH2 0.41 C++ AI).
//!
//! `AI_doDiplo`, `AI_eventValue`, `AI_chooseEvent`, `AI_doSplit`, `AI_launch`,
//! `AI_doCheckFinancialTrouble`, `AI_disbandUnit`, the culture victory functions and
//! `AI_getStrategyHash` (`read`/`write` are not ported: the AI state is not saved).
//!
//! Trade denials: the C++ `canTradeItem(eWhoTo, item, true)` asks `CvPlayer::getTradeDenial`,
//! which is the C++ AI's `AI_*Trade` judgement of the giving player; ffh-core's
//! `player_get_trade_denial` leaves those judgements out, so this file has its own
//! [`trade_denial`] / [`can_trade_item`] built on the ported `AI_*Trade` functions.
//!
//! Deals with other AI players: the C++ implements them at once (the other side's consent is
//! the denial test of its `canTradeItem`, judged by our port of its AI). PORT ADAPT: a player
//! played by another controller (not the C++ AI) is asked `consider_offer` first
//! ([`implement_ai_deal`]).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use std::collections::BTreeMap;

use crate::prelude::*;

// --------------------------------------------------------------------------------------------
// Private helpers
// --------------------------------------------------------------------------------------------

/// Attitude as the C++ `int` (`ATTITUDE_FURIOUS` = 0 ...).
fn att(a: AttitudeType) -> i32 {
    a.0 as i32
}

/// An attitude threshold (`NO_ATTITUDE` = -1).
fn att_opt(a: Option<AttitudeType>) -> i32 {
    a.map_or(-1, |a| a.0 as i32)
}

fn team_of(g: &Game, p: PlayerId) -> TeamId {
    g.players[p].team
}

/// `GET_TEAM(getTeam()).getLeaderID() == getID()`.
fn is_team_leader(g: &Game, p: PlayerId) -> bool {
    g.team_get_leader_id(g.players[p].team) == Some(p)
}

/// `GC.getLeaderHeadInfo(GET_PLAYER(p).getPersonalityType())`.
fn leader_of(g: &Game, p: PlayerId) -> Option<&'static LeaderHeadInfo> {
    let d: &'static GameData = g.d;
    g.players[p].personality_type.map(|l| &d.leader_heads[l])
}

/// `GC.getCivilizationInfo(getCivilizationType()).getCivilizationUnits(eClass)`.
fn civ_unit(g: &Game, p: PlayerId, class: UnitClassType) -> Option<UnitType> {
    g.d.civ_unit(g.player_get_civilization_type(p)?, class)
}

/// `GC.getCivilizationInfo(getCivilizationType()).getCivilizationBuildings(eClass)`.
fn civ_building(g: &Game, p: PlayerId, class: BuildingClassType) -> Option<BuildingType> {
    g.d.civ_building(g.player_get_civilization_type(p)?, class)
}

/// `CvPlayer::getCity(iID)`: the city with that id if `p` owns it.
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

/// A team index stored in `TradeData::m_iData`.
fn data_team(data: i32) -> Option<TeamId> {
    if data < 0 {
        None
    } else {
        Some(TeamId(data as u8))
    }
}

/// C++ `CvPlayer::getTradeDenial(eWhoTo, item)` with the C++ AI's `AI_*Trade` judgements
/// (None = NO_DENIAL). ffh-core's `player_get_trade_denial` leaves those judgements out.
fn trade_denial(cai: &Cai, g: &Game, p: PlayerId, who_to: PlayerId, item: &TradeData) -> Option<DenialType> {
    let team = g.players[p].team;
    let to_team = g.players[who_to].team;

    match item.item_type {
        TradeableItem::TRADE_TECHNOLOGIES => {
            let tech: Option<TechType> = opt_from_i32(item.data);
            tech.and_then(|t| cai.team_ai_tech_trade(g, team, t, to_team))
        }
        TradeableItem::TRADE_RESOURCES => {
            let bonus: Option<BonusType> = opt_from_i32(item.data);
            bonus.and_then(|b| cai.player_ai_bonus_trade(g, p, b, who_to))
        }
        TradeableItem::TRADE_CITIES => player_city(g, p, item.data).and_then(|c| cai.player_ai_city_trade(g, p, c, who_to)),
        TradeableItem::TRADE_GOLD | TradeableItem::TRADE_GOLD_PER_TURN => None,
        TradeableItem::TRADE_MAPS => cai.team_ai_map_trade(g, team, to_team),
        TradeableItem::TRADE_SURRENDER => cai.team_ai_surrender_trade(g, team, to_team, 140),
        TradeableItem::TRADE_VASSAL => cai.team_ai_vassal_trade(g, team, to_team),
        TradeableItem::TRADE_PEACE => data_team(item.data).and_then(|t| cai.team_ai_make_peace_trade(g, team, t, to_team)),
        // C++ default bConsiderPower = true.
        TradeableItem::TRADE_WAR => data_team(item.data).and_then(|t| cai.team_ai_declare_war_trade(g, team, t, to_team, true)),
        TradeableItem::TRADE_EMBARGO => data_team(item.data).and_then(|t| cai.player_ai_stop_trading_trade(g, p, t, who_to)),
        TradeableItem::TRADE_CIVIC => {
            let civic: Option<CivicType> = opt_from_i32(item.data);
            civic.and_then(|c| cai.player_ai_civic_trade(g, p, c, who_to))
        }
        TradeableItem::TRADE_RELIGION => {
            let religion: Option<ReligionType> = opt_from_i32(item.data);
            religion.and_then(|r| cai.player_ai_religion_trade(g, p, r, who_to))
        }
        TradeableItem::TRADE_OPEN_BORDERS => cai.team_ai_open_borders_trade(g, team, to_team),
        TradeableItem::TRADE_DEFENSIVE_PACT => cai.team_ai_defensive_pact_trade(g, team, to_team),
        TradeableItem::TRADE_PERMANENT_ALLIANCE => cai.team_ai_permanent_alliance_trade(g, team, to_team),
        _ => None,
    }
}

/// C++ `GET_PLAYER(p).canTradeItem(eWhoTo, item, bTestDenial)` with the C++ AI's denials.
/// (The C++ tests the denial first; both tests are const, so the cheaper validity test runs
/// first here.)
fn can_trade_item(cai: &Cai, g: &Game, p: PlayerId, who_to: PlayerId, item: &TradeData, test_denial: bool) -> bool {
    if !g.player_can_trade_item(p, who_to, item, false) {
        return false;
    }
    !test_denial || trade_denial(cai, g, p, who_to, item).is_none()
}

/// `GC.getGameINLINE().implementDeal(p, eOther, &ourList, &theirList)` for a deal the C++ AI
/// makes with another player without a diplomacy screen. PORT ADAPT: the C++ relies on the
/// other side's trade denials (tested before, with our port of its AI); a player played by
/// another controller is asked `consider_offer` first.
fn implement_ai_deal(cai: &Cai, g: &mut Game, p: PlayerId, other: PlayerId, our_list: &[TradeData], their_list: &[TradeData]) {
    let c = g.controller(other);
    if c.name() != "cppai" && !c.consider_offer(g, other, p, our_list, their_list) {
        return;
    }
    g.implement_deal(p, other, our_list, their_list, false);
}

/// The diplomacy screen opened for a human (`gDLL->beginDiplomacy`, omitted): the contact
/// timer and `abContacted` bookkeeping that come with it.
fn human_contact(cai: &Cai, g: &mut Game, p: PlayerId, i: PlayerId, contacted: &mut [bool], contact: Option<ContactType>) {
    let ti = g.players[i].team.idx();
    if !contacted[ti] {
        if let Some(contact) = contact {
            let delay = leader_of(g, p).map_or(0, |lh| lh.contact_delay[contact]);
            cai.player_ai_change_contact_timer(g, p, i, contact, delay);
        }
        contacted[ti] = true;
    }
}

const AI_FLAVOR_MILITARY: FlavorType = FlavorType(0);
const AI_FLAVOR_RELIGION: FlavorType = FlavorType(1);
const AI_FLAVOR_PRODUCTION: FlavorType = FlavorType(2);
const AI_FLAVOR_GOLD: FlavorType = FlavorType(3);
const AI_FLAVOR_SCIENCE: FlavorType = FlavorType(4);
const AI_FLAVOR_CULTURE: FlavorType = FlavorType(5);
const AI_FLAVOR_GROWTH: FlavorType = FlavorType(6);

impl Cai {
    /// C++ `void CvPlayerAI::AI_doDiplo()` (CvPlayerAI.cpp:12386-14222)
    pub fn player_ai_do_diplo(&self, g: &mut Game, p: PlayerId) {
        // FAssert(!isHuman()); FAssert(!isMinorCiv()); FAssert(!isBarbarian());

        // Python AI_doDiplo (CvGameUtils.py) always returns False: nothing to skip.

        let d: &'static GameData = g.d;
        let Some(lh) = leader_of(g, p) else {
            return;
        };
        let contact_rand = |c: ContactType| lh.contact_rand[c];

        let gold_value_percent = self.player_ai_gold_trade_value_percent(g, p);

        let mut contacted = vec![false; g.teams.len()];

        for pass in 0..2 {
            'players: for i in g.civ_player_ids() {
                if !g.players[i].alive {
                    continue;
                }
                if g.players[i].human != (pass == 1) {
                    continue;
                }
                if i == p {
                    continue;
                }

                if team_of(g, i) != team_of(g, p) {
                    for deal in g.deals.ids() {
                        if !g.deals.contains(deal) {
                            continue;
                        }
                        if !g.deal_is_cancelable(deal, Some(p)) {
                            continue;
                        }
                        if (g.game_turn() - g.deals[deal].initial_game_turn) < (d.defines.peace_treaty_length * 2) {
                            continue;
                        }

                        let mut cancel_deal = false;
                        let first_player = g.deals[deal].first_player;
                        let second_player = g.deals[deal].second_player;
                        let first_trades = g.deals[deal].first_trades.clone();
                        let second_trades = g.deals[deal].second_trades.clone();

                        if first_player == p && second_player == i {
                            if g.players[i].human {
                                if !self.player_ai_consider_offer(g, p, i, &second_trades, &first_trades, -1) {
                                    cancel_deal = true;
                                }
                            } else {
                                for t in &first_trades {
                                    if trade_denial(self, g, p, i, t).is_some() {
                                        cancel_deal = true;
                                        break;
                                    }
                                }
                            }
                        } else if first_player == i && second_player == p {
                            if g.players[i].human {
                                if !self.player_ai_consider_offer(g, p, i, &first_trades, &second_trades, -1) {
                                    cancel_deal = true;
                                }
                            } else {
                                for t in &second_trades {
                                    if trade_denial(self, g, p, i, t).is_some() {
                                        cancel_deal = true;
                                        break;
                                    }
                                }
                            }
                        }

                        if cancel_deal {
                            if g.player_can_contact(p, i) && self.player_ai_is_willing_to_talk(g, p, i) {
                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_NO_VASSAL / AI_DIPLOCOMMENT_CANCEL_DEAL
                                    // (beginDiplomacy omitted).
                                    let ti = team_of(g, i).idx();
                                    contacted[ti] = true;
                                }
                            }

                            g.deal_kill(deal, true); // XXX test this for AI...
                        }
                    }
                }

                if !(g.player_can_contact(p, i) && self.player_ai_is_willing_to_talk(g, p, i)) {
                    continue;
                }

                if team_of(g, i) == team_of(g, p) || g.teams[team_of(g, i)].vassal[team_of(g, p)] {
                    // XXX will it cancel this deal if it loses it's first resource???

                    let mut best_value = 0;
                    let mut best_give_bonus: Option<BonusType> = None;

                    for b in d.bonuses.keys() {
                        if g.player_get_num_tradeable_bonuses(p, b) > 1 {
                            if self.player_ai_bonus_trade_val(g, i, b, p, 1) > 0
                                && self.player_ai_bonus_val(g, i, b, 1) > self.player_ai_bonus_val(g, p, b, -1)
                            {
                                let item = set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32);
                                if can_trade_item(self, g, p, i, &item, true) {
                                    let value = 1 + self.rand(10000, "AI Bonus Trading #1");
                                    if value > best_value {
                                        best_value = value;
                                        best_give_bonus = Some(b);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(b) = best_give_bonus {
                        let our_list = vec![set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32)];
                        if g.players[i].human {
                            // AI_DIPLOCOMMENT_GIVE_HELP
                            human_contact(self, g, p, i, &mut contacted, None);
                        } else {
                            implement_ai_deal(self, g, p, i, &our_list, &[]);
                        }
                    }
                }

                if team_of(g, i) != team_of(g, p) && g.teams[team_of(g, i)].vassal[team_of(g, p)] {
                    let mut best_value = 0;
                    let mut best_give_tech: Option<TechType> = None;

                    for tech in d.techs.keys() {
                        if self.team_ai_tech_trade(g, team_of(g, p), tech, team_of(g, i)).is_none() {
                            let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                            if can_trade_item(self, g, p, i, &item, true) {
                                let value = 1 + self.rand(10000, "AI Vassal Tech gift");
                                if value > best_value {
                                    best_value = value;
                                    best_give_tech = Some(tech);
                                }
                            }
                        }
                    }

                    if let Some(tech) = best_give_tech {
                        let our_list = vec![set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32)];
                        if g.players[i].human {
                            // AI_DIPLOCOMMENT_GIVE_HELP
                            human_contact(self, g, p, i, &mut contacted, None);
                        } else {
                            implement_ai_deal(self, g, p, i, &our_list, &[]);
                        }
                    }
                }

                if !(team_of(g, i) != team_of(g, p)
                    && !g.team_is_human(team_of(g, p))
                    && (g.players[i].human || !g.team_is_human(team_of(g, i))))
                {
                    continue;
                }

                if g.teams[team_of(g, i)].vassal[team_of(g, p)] {
                    let mut best_value = 0;
                    let mut best_give_bonus: Option<BonusType> = None;

                    for b in d.bonuses.keys() {
                        if g.player_get_num_tradeable_bonuses(i, b) > 0 && g.player_get_num_available_bonuses(p, b) == 0 {
                            let value = self.player_ai_bonus_trade_val(g, p, b, i, 1);
                            if value > best_value {
                                best_value = value;
                                best_give_bonus = Some(b);
                            }
                        }
                    }

                    if let Some(b) = best_give_bonus {
                        let their_list = vec![set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32)];
                        if g.players[i].human {
                            // BUTTONPOPUP_VASSAL_GRANT_TRIBUTE popup (omitted)
                            human_contact(self, g, p, i, &mut contacted, None);
                        } else {
                            implement_ai_deal(self, g, p, i, &[], &their_list);
                        }
                    }
                }

                if g.teams[team_of(g, p)].at_war[team_of(g, i)] {
                    continue;
                }

                if att(self.player_ai_get_attitude(g, p, i, true)) >= att(AttitudeType::ATTITUDE_CAUTIOUS) {
                    for c in g.players[p].cities.clone() {
                        if !g.cities.contains(c) || g.cities[c].owner != p {
                            continue;
                        }
                        if g.cities[c].previous_owner != Some(i) {
                            if ((g.cities[c].game_turn_acquired + 4) % 20) == (g.game_turn() % 20) {
                                let mut count = 0;
                                let mut possible_count = 0;

                                let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                                for j in 0..NUM_CITY_PLOTS {
                                    if let Some(pl) = g.plot_city(cx, cy, j) {
                                        if g.map.plots[pl].owner == Some(i) {
                                            count += 1;
                                        }
                                        possible_count += 1;
                                    }
                                }

                                if count >= (possible_count / 2) {
                                    let item = set_trade_item(TradeableItem::TRADE_CITIES, c.0 as i32);
                                    if can_trade_item(self, g, p, i, &item, true) {
                                        let our_list = vec![item];
                                        if g.players[i].human {
                                            // AI_DIPLOCOMMENT_OFFER_CITY (the C++ does not test abContacted here)
                                            let ti = team_of(g, i).idx();
                                            contacted[ti] = true;
                                        } else {
                                            implement_ai_deal(self, g, p, i, &our_list, &[]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_team_leader(g, p) {
                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_PERMANENT_ALLIANCE) == 0 {
                        let mut offered = false;
                        if self.rand(contact_rand(ContactType::CONTACT_PERMANENT_ALLIANCE), "AI Diplo Alliance") == 0 {
                            let item = set_trade_item(TradeableItem::TRADE_PERMANENT_ALLIANCE, 0);

                            if can_trade_item(self, g, p, i, &item, true) && can_trade_item(self, g, i, p, &item, true) {
                                let our_list = vec![item.clone()];
                                let their_list = vec![item.clone()];

                                offered = true;

                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_OFFER_DEAL
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_PERMANENT_ALLIANCE));
                                } else {
                                    implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                    // C++ quirk: "move on to next player since we are on the same team
                                    // now", but the break leaves the whole player loop of this pass.
                                    break 'players;
                                }
                            }
                        }

                        if !offered {
                            let item = set_trade_item(TradeableItem::TRADE_VASSAL, 0);

                            if can_trade_item(self, g, p, i, &item, true) {
                                let our_list = vec![item];

                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_OFFER_VASSAL
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_PERMANENT_ALLIANCE));
                                } else {
                                    let mut accepted = true;
                                    let master_team = team_of(g, i);
                                    let our_team = team_of(g, p);
                                    for t in g.civ_team_ids() {
                                        if g.team_is_alive(t) {
                                            if t != our_team && t != master_team && g.at_war(our_team, t) && !g.at_war(master_team, t) {
                                                if self.team_ai_declare_war_trade(g, master_team, t, our_team, false).is_some() {
                                                    accepted = false;
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    if accepted {
                                        implement_ai_deal(self, g, p, i, &our_list, &[]);
                                    }
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) && !g.teams[team_of(g, p)].vassal[team_of(g, i)] {
                    // BETTER AI (Better Diplomatics) Sephi: only the favorite religion.
                    let state_religion = g.player_get_state_religion(p);
                    if state_religion.is_some() && state_religion == g.players[p].favorite_religion {
                        if g.player_can_convert(i, state_religion) {
                            if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_RELIGION_PRESSURE) == 0 {
                                if self.rand(contact_rand(ContactType::CONTACT_RELIGION_PRESSURE), "AI Diplo Religion Pressure") == 0 {
                                    // AI_DIPLOCOMMENT_RELIGION_PRESSURE
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_RELIGION_PRESSURE));
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) && !g.teams[team_of(g, p)].vassal[team_of(g, i)] {
                    if let Some(favorite_civic) = lh.favorite_civic {
                        if g.player_is_civic(p, favorite_civic) {
                            if g.player_can_do_civics(i, Some(favorite_civic)) && !g.player_is_civic(i, favorite_civic) {
                                if g.player_can_revolution(i, None) {
                                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_CIVIC_PRESSURE) == 0 {
                                        if self.rand(contact_rand(ContactType::CONTACT_CIVIC_PRESSURE), "AI Diplo Civic Pressure") == 0 {
                                            // AI_DIPLOCOMMENT_CIVIC_PRESSURE
                                            human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_CIVIC_PRESSURE));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if g.players[p].ai_memory_count[i][MemoryType::MEMORY_DECLARED_WAR] == 0
                        && g.players[p].ai_memory_count[i][MemoryType::MEMORY_HIRED_WAR_ALLY] == 0
                    {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_JOIN_WAR) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_JOIN_WAR), "AI Diplo Join War") == 0 {
                                let mut best_value = 0;
                                let mut best_team: Option<TeamId> = None;

                                for t in g.civ_team_ids() {
                                    if g.team_is_alive(t) {
                                        if g.at_war(t, team_of(g, p)) && !g.at_war(t, team_of(g, i)) {
                                            if g.teams[team_of(g, i)].has_met[t] {
                                                if g.team_can_declare_war(team_of(g, i), t) {
                                                    let value = 1 + self.rand(10000, "AI Joining War");
                                                    if value > best_value {
                                                        best_value = value;
                                                        best_team = Some(t);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if best_team.is_some() {
                                    // AI_DIPLOCOMMENT_JOIN_WAR
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_JOIN_WAR));
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) && !g.teams[team_of(g, p)].vassal[team_of(g, i)] {
                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_STOP_TRADING) == 0 {
                        if self.rand(contact_rand(ContactType::CONTACT_STOP_TRADING), "AI Diplo Stop Trading") == 0 {
                            let best_team = g.teams[team_of(g, p)].ai_worst_enemy;

                            if let Some(bt) = best_team {
                                if g.teams[team_of(g, i)].has_met[bt] && !g.teams[bt].vassal[team_of(g, i)] {
                                    if g.player_can_stop_trading_with_team(i, bt, false) {
                                        // AI_DIPLOCOMMENT_STOP_TRADING
                                        human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_STOP_TRADING));
                                    }
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if g.team_get_assets(team_of(g, i)) < (g.team_get_assets(team_of(g, p)) / 2) {
                        let threshold = leader_of(g, i).map_or(-1, |l| att_opt(l.no_give_help_attitude_threshold));
                        if att(self.player_ai_get_attitude(g, p, i, true)) > threshold {
                            if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_GIVE_HELP) == 0 {
                                if self.rand(contact_rand(ContactType::CONTACT_GIVE_HELP), "AI Diplo Give Help") == 0 {
                                    // XXX maybe do gold instead???

                                    let mut best_value = 0;
                                    let mut best_give_tech: Option<TechType> = None;

                                    for tech in d.techs.keys() {
                                        let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                        if can_trade_item(self, g, p, i, &item, true) {
                                            let value = 1 + self.rand(10000, "AI Giving Help");
                                            if value > best_value {
                                                best_value = value;
                                                best_give_tech = Some(tech);
                                            }
                                        }
                                    }

                                    if best_give_tech.is_some() {
                                        // AI_DIPLOCOMMENT_GIVE_HELP
                                        human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_GIVE_HELP));
                                    }
                                }
                            }
                        }
                    }
                }

                // BETTER AI (Better Diplomatics) Sephi: give techs to friendly AIs.
                if !g.players[i].human && is_team_leader(g, p) {
                    if g.team_get_assets(team_of(g, i)) < (g.team_get_assets(team_of(g, p)) / 2) {
                        let mut imod = 0;
                        if g.players[i].alignment == g.players[p].alignment {
                            imod += 1;
                        }
                        if g.players[i].favorite_religion == g.players[p].favorite_religion {
                            imod += 1;
                        }
                        let threshold = leader_of(g, i).map_or(-1, |l| att_opt(l.no_give_help_attitude_threshold));
                        if att(self.player_ai_get_attitude(g, p, i, true)) + imod > threshold {
                            if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_GIVE_HELP) == 0 {
                                if self.rand(contact_rand(ContactType::CONTACT_GIVE_HELP), "AI Diplo Give Help") == 0 {
                                    // XXX maybe do gold instead???

                                    let mut best_value = 0;
                                    let mut best_give_tech: Option<TechType> = None;

                                    for tech in d.techs.keys() {
                                        let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                        if can_trade_item(self, g, p, i, &item, true) {
                                            let value = 1 + self.rand(10000, "AI Giving Help");
                                            if value > best_value {
                                                best_value = value;
                                                best_give_tech = Some(tech);
                                            }
                                        }
                                    }

                                    if let Some(tech) = best_give_tech {
                                        let our_list = vec![set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32)];
                                        implement_ai_deal(self, g, p, i, &our_list, &[]);
                                    }
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if g.team_get_assets(team_of(g, i)) > (g.team_get_assets(team_of(g, p)) / 2) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_ASK_FOR_HELP) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_ASK_FOR_HELP), "AI Diplo Ask For Help") == 0 {
                                let mut best_value = 0;
                                let mut best_receive_tech: Option<TechType> = None;

                                for tech in d.techs.keys() {
                                    let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                    if can_trade_item(self, g, i, p, &item, true) {
                                        let value = 1 + self.rand(10000, "AI Asking For Help");
                                        if value > best_value {
                                            best_value = value;
                                            best_receive_tech = Some(tech);
                                        }
                                    }
                                }

                                if best_receive_tech.is_some() {
                                    // AI_DIPLOCOMMENT_ASK_FOR_HELP
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_ASK_FOR_HELP));
                                }
                            }
                        }
                    }
                }

                // UNOFFICIAL_PATCH 05/06/09: the demands below test AI_isChosenWar (BTS:
                // AI_isSneakAttackPreparing).
                if g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, false) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                let mut receive_gold = (g.players[i].gold - 50).max(0).min(self.player_ai_gold_target(g, i));

                                let remainder = d.defines.diplomacy_value_remainder;
                                if remainder != 0 {
                                    receive_gold -= receive_gold % remainder;
                                }

                                if receive_gold > 50 {
                                    // AI_DIPLOCOMMENT_DEMAND_TRIBUTE
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_DEMAND_TRIBUTE));
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, false) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                if self.team_ai_map_trade_val(g, team_of(g, p), team_of(g, i)) > 100 {
                                    // AI_DIPLOCOMMENT_DEMAND_TRIBUTE
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_DEMAND_TRIBUTE));
                                }
                            }
                        }
                    }
                }

                // BETTER AI (Better Diplomatics) Sephi: demand maps from hostile AIs.
                if !g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, true) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                if self.team_ai_map_trade_val(g, team_of(g, p), team_of(g, i)) > 100 {
                                    let their_list = vec![set_trade_item(TradeableItem::TRADE_MAPS, 0)];
                                    implement_ai_deal(self, g, p, i, &[], &their_list);
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, false) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                let best_receive_tech = self.demand_tribute_tech(g, p, i);
                                if best_receive_tech.is_some() {
                                    // AI_DIPLOCOMMENT_DEMAND_TRIBUTE
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_DEMAND_TRIBUTE));
                                }
                            }
                        }
                    }
                }

                // BETTER AI (Better Diplomatics) Sephi: demand techs from hostile AIs.
                if !g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, true) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                if let Some(tech) = self.demand_tribute_tech(g, p, i) {
                                    let their_list = vec![set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32)];
                                    implement_ai_deal(self, g, p, i, &[], &their_list);
                                }
                            }
                        }
                    }
                }

                if g.players[i].human && is_team_leader(g, p) {
                    if self.can_demand_tribute(g, p, i, false) {
                        if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEMAND_TRIBUTE) == 0 {
                            if self.rand(contact_rand(ContactType::CONTACT_DEMAND_TRIBUTE), "AI Diplo Demand Tribute") == 0 {
                                let mut best_value = 0;
                                let mut best_receive_bonus: Option<BonusType> = None;

                                for b in d.bonuses.keys() {
                                    if g.player_get_num_tradeable_bonuses(i, b) > 1 {
                                        if self.player_ai_bonus_trade_val(g, p, b, i, 1) > 0 {
                                            let item = set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32);
                                            if can_trade_item(self, g, i, p, &item, true) {
                                                let value = 1 + self.rand(10000, "AI Demanding Tribute (Bonus)");
                                                if value > best_value {
                                                    best_value = value;
                                                    best_receive_bonus = Some(b);
                                                }
                                            }
                                        }
                                    }
                                }

                                if best_receive_bonus.is_some() {
                                    // AI_DIPLOCOMMENT_DEMAND_TRIBUTE
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_DEMAND_TRIBUTE));
                                }
                            }
                        }
                    }
                }

                if is_team_leader(g, p) {
                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_OPEN_BORDERS) == 0 {
                        if self.rand(contact_rand(ContactType::CONTACT_OPEN_BORDERS), "AI Diplo Open Borders") == 0 {
                            let item = set_trade_item(TradeableItem::TRADE_OPEN_BORDERS, 0);

                            if can_trade_item(self, g, p, i, &item, true) && can_trade_item(self, g, i, p, &item, true) {
                                let our_list = vec![item.clone()];
                                let their_list = vec![item.clone()];

                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_OFFER_DEAL
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_OPEN_BORDERS));
                                } else {
                                    implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                }
                            }
                        }
                    }
                }

                if is_team_leader(g, p) {
                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_DEFENSIVE_PACT) == 0 {
                        if self.rand(contact_rand(ContactType::CONTACT_DEFENSIVE_PACT), "AI Diplo Defensive Pact") == 0 {
                            let item = set_trade_item(TradeableItem::TRADE_DEFENSIVE_PACT, 0);

                            if can_trade_item(self, g, p, i, &item, true) && can_trade_item(self, g, i, p, &item, true) {
                                let our_list = vec![item.clone()];
                                let their_list = vec![item.clone()];

                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_OFFER_DEAL
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_DEFENSIVE_PACT));
                                } else {
                                    implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                }
                            }
                        }
                    }
                }

                if !g.players[i].human || is_team_leader(g, p) {
                    if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_TRADE_TECH) == 0 {
                        if self.rand(contact_rand(ContactType::CONTACT_TRADE_TECH), "AI Diplo Trade Tech") == 0 {
                            let mut best_value = 0;
                            let mut best_receive_tech: Option<TechType> = None;

                            for tech in d.techs.keys() {
                                let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                if can_trade_item(self, g, i, p, &item, true) {
                                    let value = 1 + self.rand(10000, "AI Tech Trading #1");
                                    if value > best_value {
                                        best_value = value;
                                        best_receive_tech = Some(tech);
                                    }
                                }
                            }

                            if let Some(receive_tech) = best_receive_tech {
                                let mut best_value = 0;
                                let mut best_give_tech: Option<TechType> = None;

                                for tech in d.techs.keys() {
                                    let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                    if can_trade_item(self, g, p, i, &item, true) {
                                        let value = 1 + self.rand(10000, "AI Tech Trading #2");
                                        if value > best_value {
                                            best_value = value;
                                            best_give_tech = Some(tech);
                                        }
                                    }
                                }

                                let mut our_value = self.team_ai_tech_trade_val(g, team_of(g, p), receive_tech, team_of(g, i));
                                let mut their_value = match best_give_tech {
                                    Some(tech) => self.team_ai_tech_trade_val(g, team_of(g, i), tech, team_of(g, p)),
                                    None => 0,
                                };

                                let mut receive_gold = 0;
                                let mut give_gold = 0;

                                if their_value > our_value {
                                    let gold_weight = their_value - our_value;
                                    let mut gold_data = (gold_weight * 100) / gold_value_percent;
                                    if gold_weight * 100 > gold_data * gold_value_percent {
                                        gold_data += 1;
                                    }

                                    let gold = gold_data.min(self.player_ai_max_gold_trade(g, i, p));

                                    if gold > 0 {
                                        let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);
                                        if can_trade_item(self, g, i, p, &item, true) {
                                            receive_gold = gold;
                                            our_value += (gold * gold_value_percent) / 100;
                                        }
                                    }
                                } else if our_value > their_value {
                                    let gold_data = ((our_value - their_value) * 100) / gold_value_percent;
                                    let gold = gold_data.min(self.player_ai_max_gold_trade(g, p, i));

                                    if gold > 0 {
                                        let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);
                                        if can_trade_item(self, g, p, i, &item, true) {
                                            give_gold = gold;
                                            their_value += (gold * gold_value_percent) / 100;
                                        }
                                    }
                                }

                                if !g.players[i].human || our_value >= their_value {
                                    if (our_value > ((their_value * 2) / 3)) && (their_value > ((our_value * 2) / 3)) {
                                        let mut our_list = Vec::new();
                                        let mut their_list = Vec::new();

                                        if let Some(tech) = best_give_tech {
                                            our_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32));
                                        }

                                        their_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, receive_tech.0 as i32));

                                        if give_gold != 0 {
                                            our_list.push(set_trade_item(TradeableItem::TRADE_GOLD, give_gold));
                                        }

                                        if receive_gold != 0 {
                                            their_list.push(set_trade_item(TradeableItem::TRADE_GOLD, receive_gold));
                                        }

                                        if g.players[i].human {
                                            // AI_DIPLOCOMMENT_OFFER_DEAL
                                            human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_TRADE_TECH));
                                        } else {
                                            implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_TRADE_BONUS) == 0 {
                    if self.rand(contact_rand(ContactType::CONTACT_TRADE_BONUS), "AI Diplo Trade Bonus") == 0 {
                        let mut best_value = 0;
                        let mut best_receive_bonus: Option<BonusType> = None;

                        for b in d.bonuses.keys() {
                            if g.player_get_num_tradeable_bonuses(i, b) > 1 {
                                if self.player_ai_corporation_bonus_val(g, i, b) == 0 {
                                    if self.player_ai_bonus_trade_val(g, p, b, i, 1) > 0 {
                                        let item = set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32);
                                        if can_trade_item(self, g, i, p, &item, true) {
                                            let value = 1 + self.rand(10000, "AI Bonus Trading #1");
                                            if value > best_value {
                                                best_value = value;
                                                best_receive_bonus = Some(b);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(receive_bonus) = best_receive_bonus {
                            let mut best_value = 0;
                            let mut best_give_bonus: Option<BonusType> = None;

                            for b in d.bonuses.keys() {
                                if b != receive_bonus {
                                    if g.player_get_num_tradeable_bonuses(p, b) > 1 {
                                        if self.player_ai_bonus_trade_val(g, i, b, p, 1) > 0 {
                                            let item = set_trade_item(TradeableItem::TRADE_RESOURCES, b.0 as i32);
                                            if can_trade_item(self, g, p, i, &item, true) {
                                                let value = 1 + self.rand(10000, "AI Bonus Trading #2");
                                                if value > best_value {
                                                    best_value = value;
                                                    best_give_bonus = Some(b);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(give_bonus) = best_give_bonus {
                                if !g.players[i].human
                                    || (self.player_ai_bonus_trade_val(g, p, receive_bonus, i, -1) >= self.player_ai_bonus_trade_val(g, i, give_bonus, p, 1))
                                {
                                    let our_list = vec![set_trade_item(TradeableItem::TRADE_RESOURCES, give_bonus.0 as i32)];
                                    let their_list = vec![set_trade_item(TradeableItem::TRADE_RESOURCES, receive_bonus.0 as i32)];

                                    if g.players[i].human {
                                        // AI_DIPLOCOMMENT_OFFER_DEAL
                                        human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_TRADE_BONUS));
                                    } else {
                                        implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                    }
                                }
                            }
                        }
                    }
                }

                if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_TRADE_MAP) == 0 {
                    if self.rand(contact_rand(ContactType::CONTACT_TRADE_MAP), "AI Diplo Trade Map") == 0 {
                        let item = set_trade_item(TradeableItem::TRADE_MAPS, 0);

                        if can_trade_item(self, g, i, p, &item, true) && can_trade_item(self, g, p, i, &item, true) {
                            if !g.players[i].human
                                || (self.team_ai_map_trade_val(g, team_of(g, p), team_of(g, i)) >= self.team_ai_map_trade_val(g, team_of(g, i), team_of(g, p)))
                            {
                                let our_list = vec![set_trade_item(TradeableItem::TRADE_MAPS, 0)];
                                let their_list = vec![set_trade_item(TradeableItem::TRADE_MAPS, 0)];

                                if g.players[i].human {
                                    // AI_DIPLOCOMMENT_OFFER_DEAL
                                    human_contact(self, g, p, i, &mut contacted, Some(ContactType::CONTACT_TRADE_MAP));
                                } else {
                                    implement_ai_deal(self, g, p, i, &our_list, &their_list);
                                }
                            }
                        }
                    }
                }

                if !g.players[i].human || !contacted[team_of(g, i).idx()] {
                    let mut declare_war_trade_rand = lh.declare_war_trade_rand;
                    let mut min_at_war_counter = i32::MAX;
                    let our_team = team_of(g, p);
                    for t in g.civ_team_ids() {
                        if g.team_is_alive(t) {
                            if g.at_war(t, our_team) {
                                let mut at_war_counter = g.teams[our_team].ai_at_war_counter[t];
                                if g.teams[our_team].ai_war_plan[t] == Some(WarPlanType::WARPLAN_DOGPILE) {
                                    at_war_counter *= 2;
                                    at_war_counter += 5;
                                }
                                min_at_war_counter = at_war_counter.min(min_at_war_counter);
                            }
                        }
                    }

                    if min_at_war_counter < 10 {
                        declare_war_trade_rand *= min_at_war_counter;
                        declare_war_trade_rand /= 10;
                        declare_war_trade_rand += 1;
                    }

                    if min_at_war_counter < 4 {
                        declare_war_trade_rand /= 4;
                        declare_war_trade_rand += 1;
                    }

                    // BETTER AI (Diplomatics) Sephi: more willing to ask for a war in conquest mode.
                    if g.players[p].conquest_mode {
                        declare_war_trade_rand /= 2;
                        declare_war_trade_rand += 1;
                    }

                    if self.rand(declare_war_trade_rand, "AI Diplo Declare War Trade") == 0 {
                        let mut best_value = 0;
                        let mut best_team: Option<TeamId> = None;

                        for t in g.civ_team_ids() {
                            if g.team_is_alive(t) {
                                if g.at_war(t, team_of(g, p)) && !g.at_war(t, team_of(g, i)) {
                                    if g.team_get_at_war_count(t, true) < 2.max(g.count_civ_teams_alive() / 2) {
                                        let item = set_trade_item(TradeableItem::TRADE_WAR, t.0 as i32);
                                        if can_trade_item(self, g, i, p, &item, true) {
                                            let mut value = 1 + self.rand(1000, "AI Declare War Trading");

                                            value *= 101 + self.team_ai_get_attitude_weight(g, t, team_of(g, p));
                                            value /= 100;

                                            if value > best_value {
                                                best_value = value;
                                                best_team = Some(t);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(best_team) = best_team {
                            let mut best_value = 0;
                            let mut best_give_tech: Option<TechType> = None;

                            for tech in d.techs.keys() {
                                let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                if can_trade_item(self, g, p, i, &item, true) {
                                    let mut value = 1 + self.rand(100, "AI Tech Trading #2");
                                    value = value.wrapping_mul(g.team_get_research_left(best_team, tech));
                                    if value > best_value {
                                        best_value = value;
                                        best_give_tech = Some(tech);
                                    }
                                }
                            }

                            let mut our_value = self.team_ai_declare_war_trade_val(g, team_of(g, p), best_team, team_of(g, i));
                            let mut their_value = match best_give_tech {
                                Some(tech) => self.team_ai_tech_trade_val(g, team_of(g, i), tech, team_of(g, p)),
                                None => 0,
                            };

                            let mut best_value2 = 0;
                            let mut best_give_tech2: Option<TechType> = None;

                            if their_value < our_value {
                                if let Some(give_tech) = best_give_tech {
                                    for tech in d.techs.keys() {
                                        if tech != give_tech {
                                            let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
                                            if can_trade_item(self, g, p, i, &item, true) {
                                                let mut value = 1 + self.rand(100, "AI Tech Trading #2");
                                                value = value.wrapping_mul(g.team_get_research_left(best_team, tech));
                                                // C++ quirk: compared with iBestValue (the first tech's), so
                                                // the last tech above it wins.
                                                if value > best_value {
                                                    best_value2 = value;
                                                    best_give_tech2 = Some(tech);
                                                }
                                            }
                                        }
                                    }

                                    if let Some(tech2) = best_give_tech2 {
                                        their_value += self.team_ai_tech_trade_val(g, team_of(g, i), tech2, team_of(g, p));
                                    }
                                }
                            }

                            let mut receive_gold = 0;
                            let mut give_gold = 0;

                            if their_value > our_value {
                                let gold = (((their_value - our_value) * 100) / gold_value_percent).min(self.player_ai_max_gold_trade(g, i, p));

                                if gold > 0 {
                                    let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);
                                    if can_trade_item(self, g, i, p, &item, true) {
                                        receive_gold = gold;
                                        our_value += (gold * gold_value_percent) / 100;
                                    }
                                }
                            } else if our_value > their_value {
                                let gold = (((our_value - their_value) * 100) / gold_value_percent).min(self.player_ai_max_gold_trade(g, p, i));

                                if gold > 0 {
                                    let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);
                                    if can_trade_item(self, g, p, i, &item, true) {
                                        give_gold = gold;
                                        their_value += (gold * gold_value_percent) / 100;
                                    }
                                }
                            }

                            // BETTER AI (Diplomatics) Sephi: more willing to join a war in conquest mode.
                            if g.players[i].conquest_mode {
                                their_value = (their_value * 3) / 2;
                            }

                            if their_value > (our_value * 3 / 4) {
                                let mut our_list = Vec::new();
                                let mut their_list = Vec::new();

                                if let Some(tech) = best_give_tech {
                                    our_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32));
                                }

                                if let Some(tech) = best_give_tech2 {
                                    our_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32));
                                }

                                their_list.push(set_trade_item(TradeableItem::TRADE_WAR, best_team.0 as i32));

                                if give_gold != 0 {
                                    our_list.push(set_trade_item(TradeableItem::TRADE_GOLD, give_gold));
                                }

                                if receive_gold != 0 {
                                    their_list.push(set_trade_item(TradeableItem::TRADE_GOLD, receive_gold));
                                }

                                implement_ai_deal(self, g, p, i, &our_list, &their_list);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The common test of the tribute demands of `AI_doDiplo`: we could declare war on them
    /// (and have not chosen to), they are weaker than us and we do not like them. `ai_target`:
    /// the BETTER AI demands from AI players (half their defensive power, attitude raised by a
    /// shared alignment or favorite religion).
    fn can_demand_tribute(&self, g: &Game, p: PlayerId, i: PlayerId, ai_target: bool) -> bool {
        let our_team = team_of(g, p);
        let their_team = team_of(g, i);
        if !(g.team_can_declare_war(our_team, their_team) && !g.team_ai_is_chosen_war(our_team, their_team)) {
            return false;
        }

        let their_defensive_power = g.team_get_defensive_power(their_team);
        let their_defensive_power = if ai_target { their_defensive_power / 2 } else { their_defensive_power };
        if !(their_defensive_power < g.team_get_power(our_team, true)) {
            return false;
        }

        let mut imod = 0;
        if ai_target {
            if g.players[i].alignment == g.players[p].alignment || g.players[i].favorite_religion == g.players[p].favorite_religion {
                imod += 1;
            }
        }
        let threshold = leader_of(g, i).map_or(-1, |l| att_opt(l.demand_tribute_attitude_threshold));
        att(self.player_ai_get_attitude(g, p, i, true)) + imod <= threshold
    }

    /// The tech tribute demand of `AI_doDiplo`: a random tech they can give us that another
    /// team knows too.
    fn demand_tribute_tech(&self, g: &Game, p: PlayerId, i: PlayerId) -> Option<TechType> {
        let mut best_value = 0;
        let mut best_receive_tech: Option<TechType> = None;

        for tech in g.d.techs.keys() {
            let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.0 as i32);
            if can_trade_item(self, g, i, p, &item, true) {
                if g.count_known_tech_num_teams(tech) > 1 {
                    let value = 1 + self.rand(10000, "AI Demanding Tribute (Tech)");
                    if value > best_value {
                        best_value = value;
                        best_receive_tech = Some(tech);
                    }
                }
            }
        }

        best_receive_tech
    }

    /// C++ `int CvPlayerAI::AI_eventValue(EventTypes eEvent, const EventTriggeredData& kTriggeredData) const` (CvPlayerAI.cpp:14393-14934)
    pub fn player_ai_event_value(&self, g: &Game, p: PlayerId, event: EventType, triggered_data: &EventTriggeredData) -> i32 {
        let d: &'static GameData = g.d;
        let kev = &d.events[event];

        let num_cities = g.players[p].cities.len() as i32;
        let city = triggered_data.city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == p);
        let plot = g.map.plot_xy(triggered_data.plot_x, triggered_data.plot_y);
        let unit = triggered_data.unit.filter(|&u| g.units.contains(u) && g.units[u].owner == p);
        let other_player = triggered_data.other_player;
        let team = g.players[p].team;

        let mut happy = 0;
        let mut health = 0;
        let mut yields = [0i32; YieldType::NUM];
        let mut commerce_yields = [0i32; CommerceType::NUM];

        if let Some(other) = other_player {
            if kev.declare_war {
                let attitude = self.player_ai_get_attitude(g, p, other, true);
                if attitude == AttitudeType::ATTITUDE_FURIOUS
                    || attitude == AttitudeType::ATTITUDE_ANNOYED
                    || attitude == AttitudeType::ATTITUDE_CAUTIOUS
                {
                    if g.team_get_defensive_power(team) < g.team_get_power(g.players[other].team, true) {
                        return -i32::MAX + 1;
                    }
                } else if attitude == AttitudeType::ATTITUDE_PLEASED || attitude == AttitudeType::ATTITUDE_FRIENDLY {
                    return -i32::MAX + 1;
                }
            }
        }

        // Proportional to #turns in the game...
        // (AI evaluation will generally assume proper game speed scaling!)
        let speed = &d.game_speeds[g.game_speed_type()];
        let game_speed_percent = speed.victory_delay_percent;

        let mut value = self.rand(kev.ai_value, "AI Event choice");
        value += (g.player_get_event_cost(p, event, other_player, false) + g.player_get_event_cost(p, event, other_player, true)) / 2;

        value += kev.espionage_points;

        if let Some(tech) = kev.tech {
            value += (g.team_get_research_cost(team, tech) * kev.tech_percent) / 100;
        }

        if let Some(class) = kev.unit_class {
            if let Some(eunit) = civ_unit(g, p, class) {
                // Altough AI_unitValue compares well within units, the value is somewhat independent of cost
                let mut unit_value = d.units[eunit].production_cost;
                if unit_value > 0 {
                    unit_value *= 2;
                } else if unit_value == -1 {
                    unit_value = 200; // Great Person?
                }

                unit_value *= speed.train_percent;
                value += kev.num_units * unit_value;
            }
        }

        if kev.disband_unit {
            if let Some(u) = unit {
                let mut unit_value = d.units[g.units[u].unit_type].production_cost;
                if unit_value > 0 {
                    unit_value *= 2;
                } else if unit_value == -1 {
                    unit_value = 200; // Great Person?
                }

                unit_value *= speed.train_percent;
                value -= unit_value;
            }
        }

        if let Some(class) = kev.building_class {
            if let Some(building) = civ_building(g, p, class) {
                if city.is_some() {
                    // iValue += kEvent.getBuildingChange() * pCity->AI_buildingValue(eBuilding);
                    let mut building_value = d.buildings[building].production_cost;
                    if building_value > 0 {
                        building_value *= 2;
                    } else if building_value == -1 {
                        building_value = 300; // Shrine?
                    }

                    building_value *= speed.construct_percent;
                    value += kev.building_change * building_value;
                }
            }
        }

        let mut best_tech: Option<TechType> = None;
        let mut best_value = 0;
        for tech in d.techs.keys() {
            if g.player_can_research(p, tech, false) {
                if other_player.map_or(true, |o| g.teams[g.players[o].team].has_tech[tech]) {
                    let mut tech_value = 0;
                    for (f, &v) in kev.tech_flavor_value.iter_enumerated() {
                        tech_value += v * d.techs[tech].flavor_value.get(f).copied().unwrap_or(0);
                    }

                    if tech_value > best_value {
                        best_tech = Some(tech);
                        best_value = tech_value;
                    }
                }
            }
        }

        if let Some(tech) = best_tech {
            value += (g.team_get_research_cost(team, tech) * kev.tech_percent) / 100;
        }

        if kev.golden_age {
            value += self.player_ai_calculate_golden_age_value(g, p);
        }

        {
            // Yield and other changes
            if !kev.building_yield_changes.is_empty() {
                for bc in d.building_classes.keys() {
                    for y in 0..YieldType::NUM {
                        yields[y] += kev.building_yield_change(bc, YieldType(y as u16));
                    }
                }
            }

            if !kev.building_commerce_changes.is_empty() {
                for bc in d.building_classes.keys() {
                    for c in 0..CommerceType::NUM {
                        commerce_yields[c] += kev.building_commerce_change(bc, CommerceType(c as u16));
                    }
                }
            }

            if !kev.building_happy_changes.is_empty() {
                for bc in d.building_classes.keys() {
                    happy += kev.building_happy_change(bc);
                }
            }

            if !kev.building_health_changes.is_empty() {
                for bc in d.building_classes.keys() {
                    health += kev.building_health_change(bc);
                }
            }
        }

        if kev.city_effect {
            let mut city_population = -1;
            let mut city_turn_value = 0;
            if let Some(c) = city {
                city_population = g.cities[c].population;
                for s in d.specialists.keys() {
                    if kev.free_specialist_count.get(s).copied().unwrap_or(0) > 0 {
                        city_turn_value += self.city_ai_specialist_value(g, c, s, false, false) / 300;
                    }
                }
            }

            if city_population == -1 {
                // What is going on here?
                city_population = 5;
            }

            city_turn_value += (happy + kev.happy) * 8;
            city_turn_value += (health + kev.health) * 6;

            city_turn_value += yields[YieldType::YIELD_FOOD.idx()] * 5;
            city_turn_value += yields[YieldType::YIELD_PRODUCTION.idx()] * 5;
            city_turn_value += yields[YieldType::YIELD_COMMERCE.idx()] * 3;

            city_turn_value += commerce_yields[CommerceType::COMMERCE_RESEARCH.idx()] * 3;
            city_turn_value += commerce_yields[CommerceType::COMMERCE_GOLD.idx()] * 3;
            city_turn_value += commerce_yields[CommerceType::COMMERCE_CULTURE.idx()] * 1;
            city_turn_value += commerce_yields[CommerceType::COMMERCE_ESPIONAGE.idx()] * 2;

            value += (city_turn_value * 20 * game_speed_percent) / 100;

            value += kev.food;
            value += kev.food_percent / 4;
            value += kev.population_change * 30;
            value -= kev.revolt_turns * (12 + city_population * 16);
            value -= (kev.hurry_anger * 6 * d.defines.hurry_anger_divisor * speed.hurry_conscript_anger_percent) / 100;
            value += kev.happy_turns * 10;
            value += kev.culture / 2;
        } else if !kev.other_player_city_effect {
            let mut per_turn_value = 0;
            per_turn_value += num_cities * ((happy * 4) + (kev.happy * 8));
            per_turn_value += num_cities * ((health * 3) + (kev.health * 6));

            value += (per_turn_value * 20 * game_speed_percent) / 100;

            value += kev.food * num_cities;
            value += (kev.food_percent * num_cities) / 4;
            value += kev.population_change * num_cities * 40;
            value -= (num_cities * kev.hurry_anger * 6 * d.defines.hurry_anger_divisor * speed.hurry_conscript_anger_percent) / 100;
            value += num_cities * kev.happy_turns * 10;
            value += num_cities * kev.culture / 2;
        }

        let mut bonus_value = 0;
        if let Some(bonus) = kev.bonus {
            bonus_value = self.player_ai_bonus_val(g, p, bonus, 1);
        }

        if let Some(pl) = plot {
            if kev.feature_change != 0 {
                let mut old_feature_value = 0;
                let mut new_feature_value = 0;
                if let Some(feature) = g.map.plots[pl].feature_type {
                    // *grumble* who tied feature production to builds rather than the feature...
                    old_feature_value = d.features[feature].health_percent;

                    if kev.feature_change > 0 {
                        if let Some(new_feature) = kev.feature {
                            new_feature_value = d.features[new_feature].health_percent;
                        }
                    }

                    value += ((new_feature_value - old_feature_value) * game_speed_percent) / 100;
                }
            }

            if kev.improvement_change > 0 {
                value += (30 * game_speed_percent) / 100;
            } else if kev.improvement_change < 0 {
                value -= (30 * game_speed_percent) / 100;
            }

            if kev.route_change > 0 {
                value += (10 * game_speed_percent) / 100;
            } else if kev.route_change < 0 {
                value -= (10 * game_speed_percent) / 100;
            }

            if kev.bonus_change > 0 {
                value += (bonus_value * 15 * game_speed_percent) / 100;
            } else if kev.bonus_change < 0 {
                value -= (bonus_value * 15 * game_speed_percent) / 100;
            }

            for y in 0..YieldType::NUM {
                if kev.plot_extra_yield[y] != 0 {
                    if g.map.plots[pl].working_city.is_some() {
                        // FAssertMsg(pPlot->getWorkingCity()->getOwner() == getID(), "Event creates a boni for another player?");
                        // C++ quirk: added to aiYields after the city values were summed (unused).
                        yields[y] += kev.plot_extra_yield[y];
                    } else {
                        value += (20 * 8 * kev.plot_extra_yield[y] * game_speed_percent) / 100;
                    }
                }
            }
        }

        if kev.bonus_revealed.is_some() {
            value += (bonus_value * 10 * game_speed_percent) / 100;
        }

        if let Some(u) = unit {
            value += (2 * g.unit_base_combat_str(u) * kev.unit_experience * speed.train_percent) / 100;

            value -= 10 * kev.unit_immobile_turns;
        }

        {
            let mut promotion_value = 0;

            for (uc, promotion) in kev.unit_combat_promotion.iter_enumerated() {
                if let Some(promotion) = *promotion {
                    for &u in &g.players[p].units {
                        if g.unit_get_unit_combat_type(u) == Some(uc) {
                            if !g.units[u].has_promotion[promotion] {
                                promotion_value += 5 * g.unit_base_combat_str(u);
                            }
                        }
                    }

                    promotion_value += num_cities * 50;
                }
            }

            value += (promotion_value * game_speed_percent) / 100;
        }

        if kev.free_unit_support != 0 {
            value += (20 * kev.free_unit_support * game_speed_percent) / 100;
        }

        if kev.inflation_modifier != 0 {
            value -= (20i32
                .wrapping_mul(kev.inflation_modifier)
                .wrapping_mul(g.player_calculate_pre_inflated_costs(p))
                .wrapping_mul(game_speed_percent))
                / 100;
        }

        if kev.space_production_modifier != 0 {
            value += ((20 + num_cities) * g.players[p].space_production_modifier * game_speed_percent) / 100;
        }

        let mut other_player_attitude_weight = 0;
        if let Some(other) = other_player {
            other_player_attitude_weight = self.player_ai_get_attitude_weight(g, p, other);
            other_player_attitude_weight += 10 - self.rand(20, "AI event value attitude");
        }

        // Religion
        if let Some(religion) = triggered_data.religion {
            let mut religion_value = 15;

            if g.player_get_state_religion(p) == Some(religion) {
                religion_value += 15;
            }
            if g.player_has_holy_city(p, religion) {
                religion_value += 15;
            }

            value += (kev.convert_own_cities * religion_value * game_speed_percent) / 100;

            if kev.convert_other_cities > 0 {
                // Don't like them much = fairly indifferent, hate them = negative.
                value += (kev.convert_other_cities * (other_player_attitude_weight + 50) * religion_value * game_speed_percent) / 15000;
            }
        }

        if let Some(other) = other_player {
            let mut diplo_value = 0;
            // if we like this player then value positive attitude, if however we really hate them then
            // actually value negative attitude.
            diplo_value += ((other_player_attitude_weight + 50) * kev.attitude_modifier * g.players[other].power) / g.players[p].power.max(1);

            if kev.their_enemy_attitude_modifier != 0 {
                // Oh wow this sure is mildly complicated.
                let worst_enemy = g.teams[g.players[other].team].ai_worst_enemy;

                if let Some(worst_enemy) = worst_enemy {
                    if worst_enemy != team {
                        let third_party_attitude_weight = self.team_ai_get_attitude_weight(g, team, worst_enemy);

                        // If we like both teams, we want them to get along.
                        // If we like otherPlayer but not enemy (or vice-verca), we don't want them to get along.
                        // If we don't like either, we don't want them to get along.
                        // Also just value stirring up trouble in general.

                        let mut third_party_diplo_value = 50 * kev.their_enemy_attitude_modifier;
                        third_party_diplo_value *= third_party_attitude_weight - 10;
                        third_party_diplo_value *= other_player_attitude_weight - 10;
                        third_party_diplo_value /= 10000;

                        if other_player_attitude_weight < 0 && third_party_attitude_weight < 0 {
                            third_party_diplo_value *= -1;
                        }

                        third_party_diplo_value /= 2;

                        diplo_value += third_party_diplo_value;
                    }
                }
            }

            diplo_value *= game_speed_percent;
            diplo_value /= 100;

            if let Some(bonus_gift) = kev.bonus_gift {
                let mut gift_bonus_value = -self.player_ai_bonus_val(g, p, bonus_gift, -1);
                gift_bonus_value += (other_player_attitude_weight - 40) * self.player_ai_bonus_val(g, other, bonus_gift, 1);
                // Positive for friends, negative for enemies.
                diplo_value += (gift_bonus_value * d.defines.peace_treaty_length) / 60;
            }

            if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
                // What is this "relationships" thing?
                diplo_value /= 2;
            }

            if kev.gold_to_player {
                // If the gold goes to another player instead of the void, then this is a positive
                // thing if we like the player, otherwise it's a negative thing.
                let mut gift_value = (g.player_get_event_cost(p, event, other_player, false) + g.player_get_event_cost(p, event, other_player, true)) / 2;
                gift_value *= -other_player_attitude_weight;
                gift_value /= 110;

                value += gift_value;
            }

            // C++ quirk: the war value of isDeclareWar() and the pillage value of getMaxPillage()
            // are computed but never added (no side effects): omitted.

            value += (diplo_value * game_speed_percent) / 100;
        }

        let this_event_value = value;
        // XXX THIS IS VULNERABLE TO NON-TRIVIAL RECURSIONS!
        // Event A effects Event B, Event B effects Event A
        for e in d.events.keys() {
            let additional = kev.additional_event_chance.get(e).copied().unwrap_or(0);
            if additional > 0 {
                if e == event {
                    // Infinite recursion is not our friend.
                    // Fortunately we have the event value for this event - sans values of other events
                    // disabled or cleared. Hopefully no events will be that complicated...
                    // Double the value since it's recursive.
                    value += (additional * this_event_value) / 50;
                } else {
                    value += (additional * self.player_ai_event_value(g, p, e, triggered_data)) / 100;
                }
            }

            let clear = kev.clear_event_chance.get(e).copied().unwrap_or(0);
            if clear > 0 {
                if e == event {
                    value -= (clear * this_event_value) / 50;
                } else {
                    value -= (clear * self.player_ai_event_value(g, p, e, triggered_data)) / 100;
                }
            }
        }

        value = value.wrapping_mul(100 + self.rand(20, "AI Event choice"));
        value /= 100;

        value
    }

    /// C++ `EventTypes CvPlayerAI::AI_chooseEvent(int iTriggeredId) const` (CvPlayerAI.cpp:14936-14969)
    pub fn player_ai_choose_event(&self, g: &Game, p: PlayerId, triggered: &EventTriggeredData) -> Option<EventType> {
        let trigger = &g.d.event_triggers[triggered.trigger];

        let mut best_value = -i32::MAX;
        let mut best_event: Option<EventType> = None;

        for &event in &trigger.events {
            let mut value = -i32::MAX;
            if g.player_can_do_event(p, event, triggered) {
                value = self.player_ai_event_value(g, p, event, triggered);
            }

            if value > best_value {
                best_value = value;
                best_event = Some(event);
            }
        }

        best_event
    }

    /// C++ `void CvPlayerAI::AI_doSplit()` (CvPlayerAI.cpp:14971-15024)
    pub fn player_ai_do_split(&self, g: &mut Game, p: PlayerId) {
        if !g.player_can_split_empire(p) {
            return;
        }

        let mut area_values: BTreeMap<AreaId, i32> = BTreeMap::new();

        for (a, _) in g.map.areas.iter() {
            area_values.insert(a, 0);
        }

        for c in g.players[p].cities.clone() {
            let a = g.city_area(c);
            let v = self.city_ai_city_value(g, c);
            *area_values.entry(a).or_insert(0) += v;
        }

        for (&area, &v) in area_values.iter() {
            if v < 0 {
                if g.player_can_split_area(p, area) {
                    g.player_split_empire(p, area);

                    for u in g.players[p].units.clone() {
                        if !g.units.contains(u) {
                            continue;
                        }
                        if g.unit_area(u) == area {
                            let plot_team = g.plot_get_team(g.unit_plot(u));

                            if let Some(plot_team) = plot_team {
                                let our_team = g.players[p].team;
                                if g.teams[plot_team].vassal[our_team] && g.team_is_parent(our_team, plot_team) {
                                    g.unit_gift(u, true);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_launch(VictoryTypes eVictory)` (CvPlayerAI.cpp:15026-15084)
    pub fn player_ai_launch(&self, g: &mut Game, p: PlayerId, victory: VictoryType) {
        let team = g.players[p].team;
        if g.team_is_human(team) {
            return;
        }

        if !g.teams[team].can_launch[victory] {
            return;
        }

        let mut launch = true;

        let mut best_arrival = i32::MAX;
        let mut best_team: Option<TeamId> = None;

        for t in g.civ_team_ids() {
            if t != team {
                if g.team_is_alive(t) {
                    let countdown = g.teams[t].victory_countdown[victory];
                    if countdown > 0 {
                        if countdown < best_arrival {
                            best_arrival = countdown;
                            best_team = Some(t);
                        }

                        if countdown < g.team_get_victory_delay(team, victory) && g.team_get_launch_success_rate(t, victory) == 100 {
                            launch = false;
                            break;
                        }
                    }
                }
            }
        }

        if launch {
            if best_team.is_none() || best_arrival > g.team_get_victory_delay(team, victory) {
                if g.team_get_launch_success_rate(team, victory) < 100 {
                    launch = false;
                }
            }
        }

        if launch {
            g.player_launch(p, victory);
        }
    }

    /// C++ `void CvPlayerAI::AI_doCheckFinancialTrouble()` (CvPlayerAI.cpp:15086-15108)
    pub fn player_ai_do_check_financial_trouble(&self, g: &mut Game, p: PlayerId) {
        // if we just ran into financial trouble this turn
        let financial_trouble = self.player_ai_is_financial_trouble(g, p);
        let was = self.s().player(p).was_financial_trouble;
        if financial_trouble != was {
            if financial_trouble {
                let game_turn = g.game_turn();

                // only reset at most every 10 turns
                let last = self.s().player(p).turn_last_production_dirty;
                if game_turn > last + 10 {
                    // redeterimine the best things to build in each city
                    self.player_ai_make_production_dirty(g, p);

                    self.s().player(p).turn_last_production_dirty = game_turn;
                }
            }

            self.s().player(p).was_financial_trouble = financial_trouble;
        }
    }

    /// C++ `bool CvPlayerAI::AI_disbandUnit(int iExpThreshold, bool bObsolete)` (CvPlayerAI.cpp:15110-15283)
    pub fn player_ai_disband_unit(&self, g: &mut Game, p: PlayerId, exp_threshold: i32, obsolete: bool) -> bool {
        let d: &'static GameData = g.d;
        let mut best_value = i32::MAX;
        let mut best_unit: Option<UnitId> = None;

        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            let info = &d.units[g.units[u].unit_type];
            //FfH: Added by Kael 12/03/2007 (isMilitarySupport)
            if !g.unit_has_cargo(u) && info.military_support {
                if !g.unit_is_golden_age(u) {
                    if info.production_cost > 0 {
                        if exp_threshold == -1 || (g.unit_can_fight(u) && g.units[u].experience <= exp_threshold) {
                            let plot = g.unit_plot(u);
                            if !g.unit_is_military_happiness(u)
                                || !g.plot_is_city(plot, false, None)
                                || g.plot_plot_count(plot, Some(Game::puf_is_military_happiness), -1, -1, Some(p), None, None, -1, -1) > 2
                            {
                                let mut value = 10000 + self.rand(1000, "Disband Unit");

                                value = value.wrapping_mul(100 + info.production_cost * 3);
                                value /= 100;

                                value = value.wrapping_mul(100 + g.units[u].experience * 10);
                                value /= 100;

                                value = value.wrapping_mul(100 + g.units[u].level * 25);
                                value /= 100;

                                if g.unit_can_defend(u, None) && g.plot_is_city(plot, false, None) {
                                    value = value.wrapping_mul(2);
                                }

                                if g.plot_get_team(plot) == Some(g.unit_get_team(u)) {
                                    value = value.wrapping_mul(3);
                                }

                                let factor = match g.units[u].ai_type {
                                    Some(UnitAIType::UNITAI_UNKNOWN) | Some(UnitAIType::UNITAI_ANIMAL) => 1,
                                    Some(UnitAIType::UNITAI_SETTLE) => 20,
                                    Some(UnitAIType::UNITAI_WORKER) => {
                                        let mut f = 1;
                                        if (g.game_turn() - g.units[u].game_turn_created) > 10 {
                                            if g.plot_is_city(plot, false, None) {
                                                if let Some(c) = g.map.plots[plot].plot_city {
                                                    if self.city_ai_get_workers_needed(g, c) == 0 {
                                                        f = 10;
                                                    }
                                                }
                                            }
                                        }
                                        f
                                    }
                                    Some(UnitAIType::UNITAI_ATTACK)
                                    | Some(UnitAIType::UNITAI_ATTACK_CITY)
                                    | Some(UnitAIType::UNITAI_COLLATERAL)
                                    | Some(UnitAIType::UNITAI_PILLAGE)
                                    | Some(UnitAIType::UNITAI_RESERVE)
                                    | Some(UnitAIType::UNITAI_COUNTER) => 2,
                                    Some(UnitAIType::UNITAI_CITY_DEFENSE)
                                    | Some(UnitAIType::UNITAI_CITY_COUNTER)
                                    | Some(UnitAIType::UNITAI_CITY_SPECIAL)
                                    | Some(UnitAIType::UNITAI_PARADROP) => 6,
                                    Some(UnitAIType::UNITAI_EXPLORE) => 15,
                                    Some(UnitAIType::UNITAI_MISSIONARY) => 8,
                                    Some(UnitAIType::UNITAI_PROPHET)
                                    | Some(UnitAIType::UNITAI_ARTIST)
                                    | Some(UnitAIType::UNITAI_SCIENTIST)
                                    | Some(UnitAIType::UNITAI_GENERAL)
                                    | Some(UnitAIType::UNITAI_MERCHANT)
                                    | Some(UnitAIType::UNITAI_ENGINEER) => 1,
                                    Some(UnitAIType::UNITAI_SPY) => 12,
                                    Some(UnitAIType::UNITAI_ICBM) => 4,
                                    Some(UnitAIType::UNITAI_WORKER_SEA) => 18,
                                    Some(UnitAIType::UNITAI_ATTACK_SEA)
                                    | Some(UnitAIType::UNITAI_RESERVE_SEA)
                                    | Some(UnitAIType::UNITAI_ESCORT_SEA) => 1,
                                    Some(UnitAIType::UNITAI_EXPLORE_SEA) => 25,
                                    Some(UnitAIType::UNITAI_ASSAULT_SEA)
                                    | Some(UnitAIType::UNITAI_SETTLER_SEA)
                                    | Some(UnitAIType::UNITAI_MISSIONARY_SEA)
                                    | Some(UnitAIType::UNITAI_SPY_SEA)
                                    | Some(UnitAIType::UNITAI_CARRIER_SEA)
                                    | Some(UnitAIType::UNITAI_MISSILE_CARRIER_SEA) => 5,
                                    Some(UnitAIType::UNITAI_PIRATE_SEA) | Some(UnitAIType::UNITAI_ATTACK_AIR) => 1,
                                    Some(UnitAIType::UNITAI_DEFENSE_AIR)
                                    | Some(UnitAIType::UNITAI_CARRIER_AIR)
                                    | Some(UnitAIType::UNITAI_MISSILE_AIR) => 3,
                                    // FAssert(false); new UnitAIs not added modified Sephi
                                    _ => 1,
                                };
                                value = value.wrapping_mul(factor);

                                if info.extra_cost > 0 {
                                    value /= info.extra_cost + 1;
                                }

                                if value < best_value {
                                    best_value = value;
                                    best_unit = Some(u);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(u) = best_unit {
            g.unit_kill(u, false, None);
            return true;
        }
        false
    }

    /// C++ `int CvPlayerAI::AI_cultureVictoryTechValue(TechTypes eTech) const` (CvPlayerAI.cpp:15285-15360)
    pub fn player_ai_culture_victory_tech_value(&self, g: &Game, p: PlayerId, tech: TechType) -> i32 {
        let d: &'static GameData = g.d;
        let tinfo = &d.techs[tech];

        let mut value = 0;

        if tinfo.defensive_pact_trading {
            value += 50;
        }

        if tinfo.commerce_flexible[CommerceType::COMMERCE_CULTURE.idx()] {
            value += 100;
        }

        // units
        let any_warplan = g.team_get_any_war_plan_count(g.players[p].team, true) > 0;
        let mut best_unit_value = 0;
        for class in d.unit_classes.keys() {
            if let Some(eunit) = civ_unit(g, p, class) {
                if is_tech_required_for_unit(d, tech, eunit) {
                    let mut temp_value = (d.units[eunit].combat * 100) / g.best_land_unit_combat().max(1);
                    temp_value *= if any_warplan { 2 } else { 1 };

                    value += temp_value / 3;
                    best_unit_value = best_unit_value.max(temp_value);
                }
            }
        }
        value += (best_unit_value - 15).max(0);

        // cultural things
        for class in d.building_classes.keys() {
            if let Some(building) = civ_building(g, p, class) {
                if is_tech_required_for_building(d, tech, building) {
                    let kb = &d.buildings[building];

                    if d.building_classes[class].default_building_index != Some(building) {
                        // UB
                        value += 100;
                    }

                    value += (150 * kb.commerce_change[CommerceType::COMMERCE_CULTURE.idx()]) * 20;
                    value += kb.commerce_modifier[CommerceType::COMMERCE_CULTURE.idx()] * 2;
                }
            }
        }

        // important civics
        for civic in d.civics.keys() {
            if d.civics[civic].tech_prereq == Some(tech) {
                value += d.civics[civic].commerce_modifier[CommerceType::COMMERCE_CULTURE.idx()] * 2;
            }
        }

        value
    }

    /// C++ `int CvPlayerAI::AI_getCultureVictoryStage() const` (CvPlayerAI.cpp:15362-15493)
    pub fn player_ai_get_culture_victory_stage(&self, g: &Game, p: PlayerId) -> i32 {
        let d: &'static GameData = g.d;

        if !g.cultural_victory_valid() {
            return 0;
        }

        let Some(capital) = g.player_get_capital_city(p) else {
            return 0;
        };

        if g.cities[capital].game_turn_founded > (10 + g.start_turn) {
            // the loss of the capital indicates it might be a good idea to abort any culture victory
            return 0;
        }

        if g.start_era().0 as i32 > 1 {
            return 0;
        }

        let mut value = 0;

        if g.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE) {
            value += 4;
        }

        for tr in d.traits.keys() {
            if g.players[p].traits[tr] {
                let ti = &d.traits[tr];
                // spiritual +4
                value += if ti.max_anarchy == 0 { 4 } else { 0 };

                // creative +4
                value += ti.commerce_change[CommerceType::COMMERCE_CULTURE.idx()] * 2;

                // (none in default xml), set so 25% boost = 4
                value += ti.commerce_modifier[CommerceType::COMMERCE_CULTURE.idx()] / 6;

                // philisophical +4
                value += ti.great_people_rate_modifier / 25;

                // financial +4
                value += ti.extra_yield_threshold[YieldType::YIELD_COMMERCE.idx()] * 2 * d.defines.extra_yield;

                // (none in default xml), set so 100% boost = 4
                value += ti.trade_yield_modifier[YieldType::YIELD_COMMERCE.idx()] / 25;

                // industrious +4
                value += ti.max_global_building_production_modifier / 25;
            }
        }

        value += if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) { -3 } else { 0 };

        let favorite_civic = leader_of(g, p).and_then(|l| l.favorite_civic);
        if let Some(civic) = favorite_civic {
            if d.civics[civic].no_non_state_religion_spread {
                value -= 3;
            }
        }

        let num_cities = g.players[p].cities.len() as i32;

        value += g.player_count_holy_cities(p);
        if g.team_is_a_vassal(g.players[p].team) && num_cities > 5 {
            let religion_count = g.player_count_total_has_religion(p);
            if ((religion_count * 100) / num_cities) > 250 {
                value += 1;
                value += ((2 * religion_count) + 1) / num_cities;
            }
        }

        // atleast I'm honest
        let mut nonsense = 0;
        nonsense += g.cities[capital].x;
        nonsense += g.cities[capital].y;

        value += nonsense % 7;

        if value < 10 {
            return 0;
        }

        let current_era = g.players[p].current_era.0 as i32;
        let num_eras = d.eras.len() as i32;

        if current_era >= (num_eras - (2 + nonsense % 2)) {
            if g.player_count_total_has_religion(p) >= num_cities * 3 {
                return 3;
            }

            let mut high_culture_count = 0;

            let victory_level = g.cultural_victory_culture_level().map_or(-1, |l| l.0 as i32);
            for &c in &g.players[p].cities {
                // is culture rate over 100?
                if g.city_base_commerce_rate(c, CommerceType::COMMERCE_CULTURE) > 100 {
                    high_culture_count += 1;
                }
                // or is the culture level 1 less than required to win?
                else if g.cities[c].culture_level.map_or(-1, |l| l.0 as i32) >= (victory_level - 1) {
                    high_culture_count += 1;
                }
            }

            // if we have enough high culture cities, go to stage 3
            if high_culture_count >= g.num_culture_victory_cities {
                return 3;
            }
        }

        if current_era >= ((num_eras / 3) + nonsense % 2) {
            2
        } else {
            1
        }
    }

    /// C++ `bool CvPlayerAI::AI_isDoStrategy(int iStrategy) const` (CvPlayerAI.cpp:15496-15503)
    pub fn player_ai_is_do_strategy(&self, g: &Game, p: PlayerId, strategy: i32) -> bool {
        if g.players[p].human {
            return false;
        }
        (strategy & self.player_ai_get_strategy_hash(g, p)) != 0
    }

    /// C++ `void CvPlayerAI::AI_forceUpdateStrategies()` (CvPlayerAI.cpp:15505-15509)
    pub fn player_ai_force_update_strategies(&self, g: &Game, p: PlayerId) {
        // this forces a recache.
        self.s().player(p).strategy_hash_cache_turn = -1;
    }

    /// C++ `int CvPlayerAI::AI_getStrategyHash() const` (CvPlayerAI.cpp:15511-16739)
    ///
    /// FfH (Kael 05/08/2008) rewrote it; the BTS body (15515-16195) is commented out in the
    /// C++. The C++ writes `m_iStrategyHash` as it goes (functions called during the analysis
    /// may read it): the partial hash is stored in the AI state after every change.
    pub fn player_ai_get_strategy_hash(&self, g: &Game, p: PlayerId) -> i32 {
        let d: &'static GameData = g.d;
        let game_turn = g.game_turn();
        {
            let (hash, cache_turn) = {
                let mut s = self.s();
                let pa = s.player(p);
                (pa.strategy_hash, pa.strategy_hash_cache_turn)
            };
            if hash != 0 && cache_turn == game_turn {
                return hash;
            }
        }

        let mut hash = AI_DEFAULT_STRATEGY;
        let store = |h: i32| {
            self.s().player(p).strategy_hash = h;
        };
        {
            let mut s = self.s();
            let pa = s.player(p);
            pa.strategy_hash = hash;
            pa.strategy_hash_cache_turn = game_turn;
        }

        // 0, 2, 5 or 10 in default xml [augustus 5, frederick 10, huayna 2, jc 2, chinese leader 2, qin 5, ramsess 2, roosevelt 5, stalin 2]
        if self.player_ai_get_flavor_value(g, p, AI_FLAVOR_PRODUCTION) >= 2 {
            hash |= AI_STRATEGY_PRODUCTION;
            store(hash);
        }

        let Some(capital) = g.player_get_capital_city(p) else {
            return hash;
        };

        let mut nonsense = 0;
        nonsense += g.cities[capital].x;
        nonsense += g.cities[capital].y;

        let team = g.players[p].team;
        let met_count = g.team_get_has_met_civ_count(team, true);
        let mut paranoia = 0;

        // peace
        if met_count == 0 {
            paranoia = 0;
        } else {
            for i in g.player_ids() {
                if i != p && g.players[i].alive {
                    if !g.team_is_a_vassal(g.players[i].team) {
                        // int iCloseness = AI_playerCloseness((PlayerTypes)iI, 5 + iCurrentEra);
                        let closeness = self.player_ai_player_closeness(g, p, i, 5);
                        if closeness > 0 {
                            let war_memory = self.player_ai_get_memory_attitude(g, i, p, MemoryType::MEMORY_DECLARED_WAR);
                            if war_memory > 0 {
                                // they are a snake
                                paranoia += 50 + 20 * war_memory;
                                break;
                            }

                            if att(self.player_ai_get_attitude(g, p, i, true)) < att(AttitudeType::ATTITUDE_PLEASED) {
                                paranoia += 30;
                            }

                            // perform some personality profiling
                            for j in g.player_ids() {
                                // XXX the alive check means it doesn't count murder victims, should it?
                                // PORT ADAPT: the C++ tests isHasMet((TeamTypes)iJ), a player index used as a
                                // team index; here player iJ's team (the intent).
                                if j != p
                                    && i != j
                                    && g.players[j].alive
                                    && g.teams[team].has_met[g.players[j].team]
                                    && !g.team_is_a_vassal(g.players[j].team)
                                {
                                    if self.player_ai_get_memory_attitude(g, i, j, MemoryType::MEMORY_DECLARED_WAR) > 0 {
                                        // Do we not hate the victim?
                                        if att(self.player_ai_get_attitude(g, p, j, true)) >= att(AttitudeType::ATTITUDE_ANNOYED) {
                                            paranoia += if closeness < 15 { 60 } else { 100 };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // iParanoia -= iCurrentEra * 10;

        if paranoia < (if met_count > 1 { 100 } else { 50 }) {
            hash |= AI_STRATEGY_PEACE;
            store(hash);
        }

        // Unit Analysis
        let mut best_slow_unit_combat = -1;
        let mut best_fast_unit_combat = -1;
        let mut has_mobile_artillery = false;
        let mut attack_unit_count = 0;
        for class in d.unit_classes.keys() {
            if let Some(eunit) = civ_unit(g, p, class) {
                if g.player_can_train(p, eunit, false, false, false) {
                    let ku = &d.units[eunit];
                    let is_uu = d.unit_classes[class].default_unit_index != Some(eunit);
                    if ku.unit_ai_type[UnitAIType::UNITAI_RESERVE]
                        || ku.unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY]
                        || ku.unit_ai_type[UnitAIType::UNITAI_COUNTER]
                        || ku.unit_ai_type[UnitAIType::UNITAI_PILLAGE]
                        || ku.unit_ai_type[UnitAIType::UNITAI_ATTACK]
                    {
                        attack_unit_count += 1;
                        if is_uu {
                            if ku.unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY]
                                || (ku.unit_ai_type[UnitAIType::UNITAI_ATTACK] && !ku.unit_ai_type[UnitAIType::UNITAI_CITY_DEFENSE])
                            {
                                attack_unit_count += 1;
                            }
                        }
                        let combat = self.player_ai_true_combat_value(g, p, eunit);
                        let moves = ku.moves;
                        if moves == 1 {
                            best_slow_unit_combat = best_slow_unit_combat.max(combat);
                        } else if moves > 1 {
                            best_fast_unit_combat = best_fast_unit_combat.max(combat);
                            if is_uu {
                                best_fast_unit_combat += 1;
                            }
                        }
                    }
                    if ku.moves > 1 {
                        if ku.bombard_rate > 10 {
                            has_mobile_artillery = true;
                        }
                    }
                }
            }
        }

        if attack_unit_count <= 1 {
            hash |= AI_STRATEGY_GET_BETTER_UNITS;
            store(hash);
        }
        if best_fast_unit_combat > best_slow_unit_combat {
            hash |= AI_STRATEGY_FASTMOVERS;
            if has_mobile_artillery {
                hash |= AI_STRATEGY_LAND_BLITZ;
            }
            store(hash);
        }

        // missionary
        {
            if let Some(state_religion) = g.player_get_state_religion(p) {
                let holy_city_count = g.player_count_holy_cities(p);
                if holy_city_count > 0 && g.player_has_holy_city(p, state_religion) {
                    let mut missionary = 0;
                    // Missionary
                    missionary += self.player_ai_get_flavor_value(g, p, AI_FLAVOR_GROWTH) * 2; // up to 10
                    missionary += self.player_ai_get_flavor_value(g, p, AI_FLAVOR_CULTURE) * 4; // up to 40
                    missionary += self.player_ai_get_flavor_value(g, p, AI_FLAVOR_RELIGION) * 6; // up to 60

                    let favorite_civic = leader_of(g, p).and_then(|l| l.favorite_civic);
                    if let Some(civic) = favorite_civic {
                        if d.civics[civic].no_non_state_religion_spread {
                            missionary += 20;
                        }
                    }
                    missionary += (holy_city_count - 1) * 5;
                    missionary += met_count * 7;
                    for i in g.player_ids() {
                        if g.players[i].alive {
                            if i != p {
                                if g.team_is_open_borders(team, g.players[i].team) {
                                    missionary += if g.player_count_holy_cities(i) == 0 { 12 } else { 4 };
                                }
                                if g.player_get_state_religion(i) == Some(state_religion) {
                                    missionary += 10;
                                }
                            }
                        }
                    }
                    missionary += (nonsense % 7) * 3;
                    if missionary > 100 {
                        hash |= AI_STRATEGY_MISSIONARY;
                        store(hash);
                    }
                }
            }
        }

        // dagger
        if (hash & AI_STRATEGY_MISSIONARY) == 0 && (met_count > 0 && paranoia > 0) {
            let lh = leader_of(g, p);
            let max_war_rand = lh.map_or(0, |l| l.max_war_rand);
            let mut dagger = 0;
            dagger += 12000 / 100.max(50 + max_war_rand);
            dagger *= nonsense % 11;
            dagger /= 10;
            dagger += 5 * 8.min(self.player_ai_get_flavor_value(g, p, AI_FLAVOR_MILITARY));
            for class in d.unit_classes.keys() {
                let eunit = civ_unit(g, p, class);
                if let Some(eunit) = eunit {
                    if d.units[eunit].combat > 0 {
                        let default_unit = d.unit_classes[class].default_unit_index;
                        if default_unit != Some(eunit) {
                            let is_defensive = d.units[eunit].unit_ai_type[UnitAIType::UNITAI_CITY_DEFENSE]
                                && !d.units[eunit].unit_ai_type[UnitAIType::UNITAI_RESERVE];
                            dagger += if is_defensive { -10 } else { 0 };
                            if g.player_can_train(p, eunit, false, false, false) {
                                dagger += if is_defensive { 10 } else { 40 };
                                let uu_str = self.player_ai_true_combat_value(g, p, eunit);
                                // PORT ADAPT: a class without a default unit counts as strength 0.
                                let normal_str = default_unit.map_or(0, |du| self.player_ai_true_combat_value(g, p, du));
                                dagger += 20 * (uu_str - normal_str).clamp(0, 2);
                            }
                        }
                    }
                }
            }
            for &u in &g.players[p].units {
                dagger += (g.units[u].level - 1) * 5;
            }
            if !g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
                dagger += (100 - d.handicaps[g.handicap].ai_train_percent).clamp(0, 15);
            }
            let area_ai = g.map.areas[g.city_area(capital)].area_ai_type[team];
            if area_ai == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE) {
                dagger += if attack_unit_count > 0 { 40 } else { 20 };
            }
            if dagger >= AI_DAGGER_THRESHOLD {
                hash |= AI_STRATEGY_DAGGER;
                store(hash);
            }
        }

        let culture_stage = self.player_ai_get_culture_victory_stage(g, p);
        if culture_stage > 0 {
            hash |= AI_STRATEGY_CULTURE1;
            if culture_stage > 1 {
                hash |= AI_STRATEGY_CULTURE2;
                if culture_stage > 2 {
                    hash |= AI_STRATEGY_CULTURE3;
                    store(hash);
                    let tech_value = match g.player_get_current_research(p) {
                        Some(tech) => self.player_ai_culture_victory_tech_value(g, p, tech),
                        None => 0,
                    };
                    if tech_value < 100 {
                        hash |= AI_STRATEGY_CULTURE4;
                    }
                }
            }
            store(hash);
        }

        // turn off dagger if culture gets to 2
        if (hash & AI_STRATEGY_CULTURE2) != 0 {
            hash &= !AI_STRATEGY_DAGGER;
            store(hash);
        }

        {
            // Crush
            let mut war_count = 0;
            let mut crush_value = 0;

            crush_value += nonsense % 4;

            if (hash & AI_STRATEGY_DAGGER) != 0 {
                crush_value += 3;
            }
            if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
                crush_value += 3;
            }
            for t in g.civ_team_ids() {
                if g.team_is_alive(t) && t != team {
                    let war_plan = g.teams[team].ai_war_plan[t];
                    if war_plan.is_some() {
                        if !g.team_is_a_vassal(t) {
                            if self.team_ai_team_closeness(g, team, t, -1) > 0 {
                                war_count += 1;
                            }
                        }
                        if war_plan == Some(WarPlanType::WARPLAN_PREPARING_TOTAL) {
                            crush_value += 6;
                        } else if war_plan == Some(WarPlanType::WARPLAN_TOTAL) && g.teams[team].ai_war_plan_state_counter[t] < 20 {
                            crush_value += 6;
                        }
                        if war_plan == Some(WarPlanType::WARPLAN_DOGPILE) && g.teams[team].ai_war_plan_state_counter[t] < 20 {
                            for j in g.team_ids() {
                                // C++ quirk: compares the team index iJ with the player id.
                                if j != t && j.idx() != p.idx() {
                                    if g.at_war(t, j) && !g.team_is_a_vassal(t) {
                                        crush_value += 4;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if war_count <= 1 && crush_value >= 10 {
                hash |= AI_STRATEGY_CRUSH;
                store(hash);
            }
        }

        let our_victory_countdown = self.team_ai_get_lowest_victory_countdown(g, team);
        let mut their_victory_countdown = i32::MAX;
        for t in g.civ_team_ids() {
            if g.team_is_alive(t) && t != team {
                their_victory_countdown = their_victory_countdown.min(self.team_ai_get_lowest_victory_countdown(g, t));
            }
        }
        if their_victory_countdown == i32::MAX {
            their_victory_countdown = -1;
        }
        if our_victory_countdown >= 0 && our_victory_countdown <= their_victory_countdown {
            hash |= AI_STRATEGY_LAST_STAND;
            store(hash);
        }
        if their_victory_countdown > 0 && their_victory_countdown < our_victory_countdown {
            hash |= AI_STRATEGY_FINAL_WAR;
            store(hash);
        }
        if our_victory_countdown < 0 {
            if g.player_is_current_research_repeat(p) {
                let mut stronger = 0;
                let mut alive = 1;
                for t in g.civ_team_ids() {
                    if t != team {
                        if g.team_is_alive(t) {
                            alive += 1;
                            if g.team_get_power(team, true) < g.team_get_power(t, true) {
                                stronger += 1;
                            }
                        }
                    }
                }
                if stronger <= 1 || stronger <= alive / 4 {
                    hash |= AI_STRATEGY_FINAL_WAR;
                    store(hash);
                }
            }
        }

        if g.player_is_current_research_repeat(p) {
            let mut total_victories = 0;
            let mut achieve_victories = 0;
            let mut war_victories = 0;
            let threshold = 1.max((g.count_civ_teams_alive() + 1) / 4);
            let team_score = |t: TeamId| g.team_score.raw.get(t.idx()).copied().unwrap_or(0);
            for v in d.victories.keys() {
                let kv = &d.victories[v];
                if g.is_victory_valid(v) {
                    total_victories += 1;
                    if kv.diplo_vote {
                        //
                    } else if kv.end_score {
                        let mut higher_count = 0;
                        let mut weaker_count = 0;
                        for t in g.civ_team_ids() {
                            if t != team {
                                if g.team_is_alive(t) {
                                    if team_score(team) < ((team_score(t) * 90) / 100) {
                                        higher_count += 1;
                                        if g.team_get_power(team, true) > g.team_get_power(t, true) {
                                            weaker_count += 1;
                                        }
                                    }
                                }
                            }
                        }

                        if higher_count > 0 {
                            if weaker_count == higher_count {
                                war_victories += 1;
                            }
                        }
                    } else if kv.city_culture.map_or(false, |l| l.0 > 0) {
                        if (hash & AI_STRATEGY_CULTURE1) != 0 {
                            achieve_victories += 1;
                        }
                    } else if kv.min_land_percent > 0 || kv.land_percent > 0 {
                        let mut larger_count = 0;
                        for t in g.civ_team_ids() {
                            if t != team {
                                if g.team_is_alive(t) {
                                    if g.team_get_total_land(team, true) < g.team_get_total_land(t, true) {
                                        larger_count += 1;
                                    }
                                }
                            }
                        }
                        if larger_count <= threshold {
                            war_victories += 1;
                        }
                    } else if kv.conquest {
                        let mut stronger_count = 0;
                        for t in g.civ_team_ids() {
                            if t != team {
                                if g.team_is_alive(t) {
                                    if g.team_get_power(team, true) < g.team_get_power(t, true) {
                                        stronger_count += 1;
                                    }
                                }
                            }
                        }
                        if stronger_count <= threshold {
                            war_victories += 1;
                        }
                    } else {
                        if g.teams[team].victory_countdown[v] > 0 {
                            achieve_victories += 1;
                        }
                    }
                }
            }

            if achieve_victories == 0 {
                if war_victories > 0 {
                    hash |= AI_STRATEGY_FINAL_WAR;
                    store(hash);
                }
            }
        }

        // Espionage
        if !g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE) {
            let mut temp_value = 0;
            if g.players[p].commerce_percent[CommerceType::COMMERCE_ESPIONAGE.idx()] == 0 {
                temp_value += 4;
            }

            if g.team_get_any_war_plan_count(team, true) == 0 {
                temp_value += 3;
            }

            temp_value += (100 - self.player_ai_get_espionage_weight(g, p)) / 10;

            temp_value += nonsense % 12;

            if temp_value > 10 {
                hash |= AI_STRATEGY_BIG_ESPIONAGE;
                store(hash);
            }
        }
        // Turn off inappropriate strategies.
        if g.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE) {
            hash &= !AI_STRATEGY_DAGGER;
            hash &= !AI_STRATEGY_CRUSH;
            // m_iStrategyHash &= ~AI_STRATEGY_OWABWNW;
            hash &= !AI_STRATEGY_FASTMOVERS;
            hash &= !AI_STRATEGY_FINAL_WAR;
        }

        store(hash);
        hash
    }
}
