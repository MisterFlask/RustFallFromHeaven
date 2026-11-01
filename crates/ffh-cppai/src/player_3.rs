//! Port of CvPlayerAI.cpp:6805-9819 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES: C++ `AI_unitValue(eUnit, eUnitAI, pArea)` is called with `pArea == NULL`
//! (CvPlayer.cpp:1014, CvPlayerAI.cpp:18748 with no city, CvSelectionGroup.cpp:4194); the stub
//! signature takes an `AreaId`, so the NULL case is [`Cai::player_ai_unit_value_opt`] (same
//! function, `Option<AreaId>`): callers porting a NULL area call that one.
//!
//! Rules functions re-implemented privately: `CvPlayer::getTradeDenial` (ffh-core's
//! `player_get_trade_denial` leaves out the AI judgements, which are the point here) and
//! `CvPlayer::getCity(iID)` for trade items.

#![allow(unused_variables, unused_mut, unused_imports, unused_assignments, clippy::all)]

use crate::prelude::*;
use ffh_core::deal::Deal;

/// `GC.getLeaderHeadInfo(GET_PLAYER(p).getPersonalityType())`.
fn leader(g: &Game, p: PlayerId) -> &'static LeaderHeadInfo {
    let d: &'static GameData = g.d;
    &d.leader_heads[g.players[p].personality_type.expect("getPersonalityType() is NO_LEADER")]
}

/// A trade item's `m_iData` as a type index.
fn di<I: Idx>(data: i32) -> I {
    I::from_usize(data.max(0) as usize)
}

/// A trade item's `m_iData` as a team.
fn dt(data: i32) -> TeamId {
    TeamId(data.max(0) as u8)
}

/// C++ `GET_PLAYER(p).getCity(iID)`: the city with this id if `p` owns it (NULL otherwise).
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

/// C++ `AttitudeTypes` thresholds compared as ints (NO_ATTITUDE = -1).
fn att_i(a: Option<AttitudeType>) -> i32 {
    opt_to_i32(a)
}

/// Sephi's BETTER AI diplomacy bonus: +1 when the two players share an alignment or a
/// favorite religion.
fn same_alignment_or_religion_mod(g: &Game, a: PlayerId, b: PlayerId) -> i32 {
    if g.players[a].alignment == g.players[b].alignment || g.players[a].favorite_religion == g.players[b].favorite_religion {
        1
    } else {
        0
    }
}

/// C++ `CvPlayer::getTradeDenial(eWhoTo, item)` with the AI judgements (CvPlayer.cpp:4556);
/// ffh-core's `player_get_trade_denial` reads the AI parts as no denial.
fn trade_denial(cai: &Cai, g: &Game, p: PlayerId, who_to: PlayerId, item: &TradeData) -> Option<DenialType> {
    let team = g.players[p].team;
    let to_team = g.players[who_to].team;
    match item.item_type {
        TradeableItem::TRADE_TECHNOLOGIES => cai.team_ai_tech_trade(g, team, di::<TechType>(item.data), to_team),
        TradeableItem::TRADE_RESOURCES => cai.player_ai_bonus_trade(g, p, di::<BonusType>(item.data), who_to),
        TradeableItem::TRADE_CITIES => match player_city(g, p, item.data) {
            Some(c) => cai.player_ai_city_trade(g, p, c, who_to),
            None => None,
        },
        TradeableItem::TRADE_GOLD | TradeableItem::TRADE_GOLD_PER_TURN => None,
        TradeableItem::TRADE_MAPS => cai.team_ai_map_trade(g, team, to_team),
        TradeableItem::TRADE_SURRENDER => cai.team_ai_surrender_trade(g, team, to_team, 140),
        TradeableItem::TRADE_VASSAL => cai.team_ai_vassal_trade(g, team, to_team),
        TradeableItem::TRADE_PEACE => cai.team_ai_make_peace_trade(g, team, dt(item.data), to_team),
        // C++ default bConsiderPower = true.
        TradeableItem::TRADE_WAR => cai.team_ai_declare_war_trade(g, team, dt(item.data), to_team, true),
        TradeableItem::TRADE_EMBARGO => cai.player_ai_stop_trading_trade(g, p, dt(item.data), who_to),
        TradeableItem::TRADE_CIVIC => cai.player_ai_civic_trade(g, p, di::<CivicType>(item.data), who_to),
        TradeableItem::TRADE_RELIGION => cai.player_ai_religion_trade(g, p, di::<ReligionType>(item.data), who_to),
        TradeableItem::TRADE_OPEN_BORDERS => cai.team_ai_open_borders_trade(g, team, to_team),
        TradeableItem::TRADE_DEFENSIVE_PACT => cai.team_ai_defensive_pact_trade(g, team, to_team),
        TradeableItem::TRADE_PERMANENT_ALLIANCE => cai.team_ai_permanent_alliance_trade(g, team, to_team),
        TradeableItem::TRADE_PEACE_TREATY => None,
        _ => None,
    }
}

impl Cai {
    /// C++ `int CvPlayerAI::AI_dealVal(PlayerTypes ePlayer, const CLinkList<TradeData>* pList, bool bIgnoreAnnual = false, int iChange = 1) const` (CvPlayerAI.cpp:6805-6892)
    pub fn player_ai_deal_val(&self, g: &Game, p: PlayerId, player: PlayerId, list: &[TradeData], ignore_annual: bool, change: i32) -> i32 {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        let mut value = 0;

        if g.at_war(team, their_team) {
            value += self.team_ai_end_war_val(g, team, their_team);
        }

        for node in list {
            match node.item_type {
                TradeableItem::TRADE_TECHNOLOGIES => {
                    value += self.team_ai_tech_trade_val(g, team, di::<TechType>(node.data), their_team);
                }
                TradeableItem::TRADE_RESOURCES => {
                    if !ignore_annual {
                        value += self.player_ai_bonus_trade_val(g, p, di::<BonusType>(node.data), player, change);
                    }
                }
                TradeableItem::TRADE_CITIES => {
                    if let Some(c) = player_city(g, player, node.data) {
                        value += self.player_ai_city_trade_val(g, p, c);
                    }
                }
                TradeableItem::TRADE_GOLD => {
                    value += (node.data * self.player_ai_gold_trade_value_percent(g, p)) / 100;
                }
                TradeableItem::TRADE_GOLD_PER_TURN => {
                    if !ignore_annual {
                        value += self.player_ai_gold_per_turn_trade_val(g, p, node.data);
                    }
                }
                TradeableItem::TRADE_MAPS => {
                    value += self.team_ai_map_trade_val(g, team, their_team);
                }
                TradeableItem::TRADE_SURRENDER => {
                    if !ignore_annual {
                        value += self.team_ai_surrender_trade_val(g, team, their_team);
                    }
                }
                TradeableItem::TRADE_VASSAL => {
                    if !ignore_annual {
                        value += self.team_ai_vassal_trade_val(g, team, their_team);
                    }
                }
                TradeableItem::TRADE_OPEN_BORDERS => {
                    value += self.team_ai_open_borders_trade_val(g, team, their_team);
                }
                TradeableItem::TRADE_DEFENSIVE_PACT => {
                    value += self.team_ai_defensive_pact_trade_val(g, team, their_team);
                }
                TradeableItem::TRADE_PEACE => {
                    value += self.team_ai_make_peace_trade_val(g, team, dt(node.data), their_team);
                }
                TradeableItem::TRADE_WAR => {
                    value += self.team_ai_declare_war_trade_val(g, team, dt(node.data), their_team);
                }
                TradeableItem::TRADE_EMBARGO => {
                    value += self.player_ai_stop_trading_trade_val(g, p, dt(node.data), player);
                }
                TradeableItem::TRADE_CIVIC => {
                    value += self.player_ai_civic_trade_val(g, p, di::<CivicType>(node.data), player);
                }
                TradeableItem::TRADE_RELIGION => {
                    value += self.player_ai_religion_trade_val(g, p, di::<ReligionType>(node.data), player);
                }
                _ => {}
            }
        }

        value
    }

    /// C++ `bool CvPlayerAI::AI_goldDeal(const CLinkList<TradeData>* pList) const` (CvPlayerAI.cpp:6895-6913)
    pub fn player_ai_gold_deal(&self, g: &Game, p: PlayerId, list: &[TradeData]) -> bool {
        list.iter().any(|n| n.item_type == TradeableItem::TRADE_GOLD || n.item_type == TradeableItem::TRADE_GOLD_PER_TURN)
    }

    /// C++ `bool CvPlayerAI::AI_considerOffer(PlayerTypes ePlayer, const CLinkList<TradeData>* pTheirList, const CLinkList<TradeData>* pOurList, int iChange = 1) const` (CvPlayerAI.cpp:6916-7058)
    pub fn player_ai_consider_offer(&self, g: &Game, p: PlayerId, player: PlayerId, their_list: &[TradeData], our_list: &[TradeData], change: i32) -> bool {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        if self.player_ai_gold_deal(g, p, their_list) && self.player_ai_gold_deal(g, p, our_list) {
            return false;
        }

        if change > -1 {
            for node in our_list {
                if trade_denial(self, g, p, player, node).is_some() {
                    return false;
                }
            }
        }

        if their_team == team {
            return true;
        }

        // BETTER_BTS_AI_MOD 10/23/09 jdog5000: don't always accept giving deals, TRADE_VASSAL
        // and TRADE_SURRENDER come with strings attached.
        let mut vassal_trade = false;
        for node in their_list {
            if node.item_type == TradeableItem::TRADE_VASSAL {
                vassal_trade = true;

                for it in g.civ_team_ids() {
                    if g.team_is_alive(it) {
                        if it != team && it != their_team && g.at_war(their_team, it) && !g.at_war(team, it) {
                            if self.team_ai_declare_war_trade(g, team, it, their_team, false).is_some() {
                                return false;
                            }
                        }
                    }
                }
            } else if node.item_type == TradeableItem::TRADE_SURRENDER {
                vassal_trade = true;

                if !self.team_ai_accept_surrender(g, team, their_team) {
                    return false;
                }
            }
        }

        if !vassal_trade {
            if our_list.is_empty() && !their_list.is_empty() {
                return true;
            }
        }

        // GET_PLAYER(ePlayer).AI_dealVal(..): a valuation of the other player (our port).
        let mut our_value = self.player_ai_deal_val(g, player, p, our_list, false, change);
        let their_value = self.player_ai_deal_val(g, p, player, their_list, false, change);

        if our_value > 0 && their_list.is_empty() && their_value == 0 {
            if g.teams[team].vassal[their_team] && Deal::is_vassal_tribute_deal(our_list) {
                if self.player_ai_get_attitude(g, p, player, false).i() <= att_i(leader(g, p).vassal_refuse_attitude_threshold)
                    && g.team_get_at_war_count(team, true) == 0
                    && g.team_get_defensive_pact_count(their_team, None) == 0
                {
                    our_value *= g.team_get_power(team, false) + 10;
                    our_value /= g.team_get_power(their_team, false) + 10;
                } else {
                    return true;
                }
            } else {
                if self.player_ai_get_attitude(g, p, player, true) < AttitudeType::ATTITUDE_PLEASED {
                    if g.team_get_power(team, false) > (g.team_get_power(their_team, false) * 4) / 3 {
                        return false;
                    }
                }

                if self.player_ai_get_memory_count(g, p, player, MemoryType::MEMORY_MADE_DEMAND_RECENT) > 0 {
                    return false;
                }
            }

            let mut threshold = self.team_ai_get_has_met_counter(g, team, their_team) + 50;

            threshold *= 2;

            if self.team_ai_is_land_target(g, their_team, team) {
                threshold *= 3;
            }

            threshold *= g.team_get_power(their_team, false) + 100;
            threshold /= g.team_get_power(team, false) + 100;

            threshold -= self.player_ai_get_peacetime_grant_value(g, player, p);

            return our_value < threshold;
        }

        if change < 0 {
            return their_value * 110 >= our_value * 100;
        }

        their_value >= our_value
    }

    /// C++ `bool CvPlayerAI::AI_counterPropose(PlayerTypes ePlayer, const CLinkList<TradeData>* pTheirList, const CLinkList<TradeData>* pOurList, CLinkList<TradeData>* pTheirInventory, CLinkList<TradeData>* pOurInventory, CLinkList<TradeData>* pTheirCounter, CLinkList<TradeData>* pOurCounter) const` (CvPlayerAI.cpp:7061-7610)
    pub fn player_ai_counter_propose(&self, g: &Game, p: PlayerId, player: PlayerId, their_list: &[TradeData], our_list: &[TradeData], their_inventory: &mut Vec<TradeData>, our_inventory: &mut Vec<TradeData>, their_counter: &mut Vec<TradeData>, our_counter: &mut Vec<TradeData>) -> bool {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        let their_gold_deal = self.player_ai_gold_deal(g, p, their_list);
        let our_gold_deal = self.player_ai_gold_deal(g, p, our_list);

        if our_gold_deal && their_gold_deal {
            return false;
        }

        let mut bonus_deal = vec![false; g.d.bonuses.len()];

        // Indices into the inventory being searched (C++ node pointers).
        let mut gold_per_turn_node: Option<usize> = None;
        let mut gold_node: Option<usize> = None;

        let mut human_deal_weight = self.player_ai_deal_val(g, p, player, their_list, false, 1);
        let mut ai_deal_weight = self.player_ai_deal_val(g, player, p, our_list, false, 1);

        let gold_value_percent = self.player_ai_gold_trade_value_percent(g, p);

        their_counter.clear();
        our_counter.clear();

        if ai_deal_weight > human_deal_weight {
            if g.at_war(team, their_team) {
                let mut best_value = 0;
                let mut best_weight = 0;
                let mut best_node: Option<usize> = None;

                for i in 0..their_inventory.len() {
                    if !(ai_deal_weight > human_deal_weight) {
                        break;
                    }
                    let node = their_inventory[i].clone();
                    if !node.offering && !node.hidden {
                        if node.item_type == TradeableItem::TRADE_CITIES {
                            if trade_denial(self, g, player, p, &node).is_none() {
                                if let Some(c) = player_city(g, player, node.data) {
                                    let weight = self.player_ai_city_trade_val(g, p, c);

                                    if weight > 0 {
                                        let value = self.player_ai_target_city_value(g, p, c, false, false);

                                        if value > best_value {
                                            best_value = value;
                                            best_weight = weight;
                                            best_node = Some(i);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(i) = best_node {
                    human_deal_weight += best_weight;
                    their_counter.push(their_inventory[i].clone());
                }
            }

            for i in 0..their_inventory.len() {
                if !(ai_deal_weight > human_deal_weight) {
                    break;
                }
                let node = their_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if trade_denial(self, g, player, p, &node).is_none() {
                        match node.item_type {
                            TradeableItem::TRADE_GOLD => {
                                if !our_gold_deal {
                                    gold_node = Some(i);
                                }
                            }
                            TradeableItem::TRADE_GOLD_PER_TURN => {
                                if !our_gold_deal {
                                    gold_per_turn_node = Some(i);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            let mut gold_weight = ai_deal_weight - human_deal_weight;

            if gold_weight > 0 {
                if let Some(gi) = gold_node {
                    let mut gold_data = gold_weight * 100;
                    gold_data /= gold_value_percent;
                    // C++ quirk: compares iGoldData * percent (not / 100) with the weight.
                    if (gold_data * gold_value_percent) < gold_weight {
                        gold_data += 1;
                    }
                    if self.player_ai_max_gold_trade(g, player, p) >= gold_data {
                        their_inventory[gi].data = gold_data;
                        human_deal_weight += (gold_data * gold_value_percent) / 100;
                        their_counter.push(their_inventory[gi].clone());
                        gold_node = None;
                    }
                }
            }

            for node in our_list {
                if node.item_type == TradeableItem::TRADE_RESOURCES {
                    bonus_deal[node.data.max(0) as usize] = true;
                }
            }

            for i in 0..their_inventory.len() {
                if !(ai_deal_weight > human_deal_weight) {
                    break;
                }
                let node = their_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if trade_denial(self, g, player, p, &node).is_none() {
                        let mut weight = 0;

                        match node.item_type {
                            TradeableItem::TRADE_TECHNOLOGIES => {
                                weight += self.team_ai_tech_trade_val(g, team, di::<TechType>(node.data), their_team);
                            }
                            TradeableItem::TRADE_RESOURCES => {
                                let bonus = di::<BonusType>(node.data);
                                if !bonus_deal[bonus.idx()] {
                                    if g.player_get_num_tradeable_bonuses(player, bonus) > 1 {
                                        if g.player_ai_corporation_bonus_val(player, bonus) == 0 {
                                            weight += self.player_ai_bonus_trade_val(g, p, bonus, player, 1);
                                            bonus_deal[bonus.idx()] = true;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        if weight > 0 {
                            human_deal_weight += weight;
                            their_counter.push(node);
                        }
                    }
                }
            }

            for i in 0..their_inventory.len() {
                if !(ai_deal_weight > human_deal_weight) {
                    break;
                }
                let node = their_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if node.item_type == TradeableItem::TRADE_MAPS {
                        if trade_denial(self, g, player, p, &node).is_none() {
                            let weight = self.team_ai_map_trade_val(g, team, their_team);

                            if weight > 0 {
                                human_deal_weight += weight;
                                their_counter.push(node);
                            }
                        }
                    }
                }
            }

            gold_weight = ai_deal_weight - human_deal_weight;

            if gold_weight > 0 {
                if let Some(gi) = gold_node {
                    let mut gold_data = gold_weight * 100;
                    gold_data /= gold_value_percent;

                    if (gold_weight * 100) > (gold_data * gold_value_percent) {
                        gold_data += 1;
                    }

                    gold_data = gold_data.min(self.player_ai_max_gold_trade(g, player, p));

                    if gold_data > 0 {
                        their_inventory[gi].data = gold_data;
                        human_deal_weight += (gold_data * gold_value_percent) / 100;
                        their_counter.push(their_inventory[gi].clone());
                        gold_node = None;
                    }
                }
            }

            if ai_deal_weight > human_deal_weight {
                if let Some(gi) = gold_per_turn_node {
                    let mut gold_data = 0;

                    // FIX: guard the loop against a non-positive per-turn value (C++ would
                    // loop forever; AI_goldTradeValuePercent is >= 200, so it never fires).
                    if self.player_ai_gold_per_turn_trade_val(g, p, 1) > 0 {
                        while self.player_ai_gold_per_turn_trade_val(g, p, gold_data) < (ai_deal_weight - human_deal_weight) {
                            gold_data += 1;
                        }
                    }

                    gold_data = gold_data.min(self.player_ai_max_gold_per_turn_trade(g, player, p));

                    if gold_data > 0 {
                        their_inventory[gi].data = gold_data;
                        human_deal_weight += self.player_ai_gold_per_turn_trade_val(g, p, their_inventory[gi].data);
                        their_counter.push(their_inventory[gi].clone());
                        gold_per_turn_node = None;
                    }
                }
            }

            for i in 0..their_inventory.len() {
                if !(ai_deal_weight > human_deal_weight) {
                    break;
                }
                let node = their_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if node.item_type == TradeableItem::TRADE_RESOURCES {
                        if trade_denial(self, g, player, p, &node).is_none() {
                            let mut weight = 0;
                            let bonus = di::<BonusType>(node.data);

                            if !bonus_deal[bonus.idx()] {
                                if g.player_get_num_tradeable_bonuses(player, bonus) > 0 {
                                    weight += self.player_ai_bonus_trade_val(g, p, bonus, player, 1);
                                    bonus_deal[bonus.idx()] = true;
                                }
                            }

                            if weight > 0 {
                                human_deal_weight += weight;
                                their_counter.push(node);
                            }
                        }
                    }
                }
            }
        } else if human_deal_weight > ai_deal_weight {
            if g.at_war(team, their_team) {
                let mut surrender = false;
                for i in 0..our_inventory.len() {
                    let node = our_inventory[i].clone();
                    if !node.offering && !node.hidden {
                        if node.item_type == TradeableItem::TRADE_SURRENDER {
                            if trade_denial(self, g, p, player, &node).is_none() {
                                ai_deal_weight += self.team_ai_surrender_trade_val(g, their_team, team);
                                our_counter.push(node);
                                surrender = true;
                            }
                            break;
                        }
                    }
                }

                if !surrender {
                    for i in 0..our_inventory.len() {
                        let node = our_inventory[i].clone();
                        if !node.offering && !node.hidden {
                            if node.item_type == TradeableItem::TRADE_PEACE_TREATY {
                                our_counter.push(node);
                                break;
                            }
                        }
                    }
                }

                let mut best_value = 0;
                let mut best_weight = 0;
                let mut best_node: Option<usize> = None;

                for i in 0..our_inventory.len() {
                    if !(human_deal_weight > ai_deal_weight) {
                        break;
                    }
                    let node = our_inventory[i].clone();
                    if !node.offering && !node.hidden {
                        if node.item_type == TradeableItem::TRADE_CITIES {
                            if trade_denial(self, g, p, player, &node).is_none() {
                                if let Some(c) = player_city(g, p, node.data) {
                                    let weight = self.player_ai_city_trade_val(g, player, c);

                                    if weight > 0 {
                                        let value = self.player_ai_target_city_value(g, player, c, false, false);

                                        if value > best_value {
                                            if human_deal_weight >= (ai_deal_weight + weight) {
                                                best_value = value;
                                                best_weight = weight;
                                                best_node = Some(i);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(i) = best_node {
                    ai_deal_weight += best_weight;
                    our_counter.push(our_inventory[i].clone());
                }
            }

            for i in 0..our_inventory.len() {
                if !(human_deal_weight > ai_deal_weight) {
                    break;
                }
                let node = our_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if trade_denial(self, g, p, player, &node).is_none() {
                        match node.item_type {
                            TradeableItem::TRADE_GOLD => {
                                if !their_gold_deal {
                                    gold_node = Some(i);
                                }
                            }
                            TradeableItem::TRADE_GOLD_PER_TURN => {
                                if !their_gold_deal {
                                    gold_per_turn_node = Some(i);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            let mut gold_weight = human_deal_weight - ai_deal_weight;

            if gold_weight > 0 {
                if let Some(gi) = gold_node {
                    let mut gold_data = gold_weight * 100;
                    gold_data /= gold_value_percent;

                    if self.player_ai_max_gold_trade(g, p, player) >= gold_data {
                        our_inventory[gi].data = gold_data;
                        ai_deal_weight += (gold_data * gold_value_percent) / 100;
                        our_counter.push(our_inventory[gi].clone());
                        gold_node = None;
                    }
                }
            }

            for node in their_list {
                if node.item_type == TradeableItem::TRADE_RESOURCES {
                    bonus_deal[node.data.max(0) as usize] = true;
                }
            }

            for i in 0..our_inventory.len() {
                if !(human_deal_weight > ai_deal_weight) {
                    break;
                }
                let node = our_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if trade_denial(self, g, p, player, &node).is_none() {
                        let mut weight = 0;

                        match node.item_type {
                            TradeableItem::TRADE_TECHNOLOGIES => {
                                weight += self.team_ai_tech_trade_val(g, their_team, di::<TechType>(node.data), team);
                            }
                            TradeableItem::TRADE_RESOURCES => {
                                let bonus = di::<BonusType>(node.data);
                                if !bonus_deal[bonus.idx()] {
                                    if g.player_get_num_tradeable_bonuses(p, bonus) > 1 {
                                        weight += self.player_ai_bonus_trade_val(g, player, bonus, p, 1);
                                        bonus_deal[bonus.idx()] = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if weight > 0 {
                            if human_deal_weight >= (ai_deal_weight + weight) {
                                ai_deal_weight += weight;
                                our_counter.push(node);
                            }
                        }
                    }
                }
            }

            for i in 0..our_inventory.len() {
                if !(human_deal_weight > ai_deal_weight) {
                    break;
                }
                let node = our_inventory[i].clone();
                if !node.offering && !node.hidden {
                    if node.item_type == TradeableItem::TRADE_MAPS {
                        if trade_denial(self, g, p, player, &node).is_none() {
                            let weight = self.team_ai_map_trade_val(g, their_team, team);

                            if weight > 0 {
                                if human_deal_weight >= (ai_deal_weight + weight) {
                                    ai_deal_weight += weight;
                                    our_counter.push(node);
                                }
                            }
                        }
                    }
                }
            }

            gold_weight = human_deal_weight - ai_deal_weight;
            if gold_weight > 0 {
                if let Some(gi) = gold_node {
                    let mut gold_data = gold_weight * 100;
                    gold_data /= self.player_ai_gold_trade_value_percent(g, p);

                    gold_data = gold_data.min(self.player_ai_max_gold_trade(g, p, player));

                    if gold_data > 0 {
                        our_inventory[gi].data = gold_data;
                        ai_deal_weight += (gold_data * self.player_ai_gold_trade_value_percent(g, p)) / 100;
                        our_counter.push(our_inventory[gi].clone());
                        gold_node = None;
                    }
                }
            }

            if human_deal_weight > ai_deal_weight {
                if let Some(gi) = gold_per_turn_node {
                    let mut gold_data = 0;

                    // FIX: guard against a non-positive per-turn value (see above).
                    if self.player_ai_gold_per_turn_trade_val(g, player, 1) > 0 {
                        while self.player_ai_gold_per_turn_trade_val(g, player, gold_data + 1) <= (human_deal_weight - ai_deal_weight) {
                            gold_data += 1;
                        }
                    }

                    gold_data = gold_data.min(self.player_ai_max_gold_per_turn_trade(g, p, player));

                    if gold_data > 0 {
                        our_inventory[gi].data = gold_data;
                        ai_deal_weight += self.player_ai_gold_per_turn_trade_val(g, player, our_inventory[gi].data);
                        our_counter.push(our_inventory[gi].clone());
                        gold_per_turn_node = None;
                    }
                }
            }
        }

        (ai_deal_weight <= human_deal_weight) && (!our_list.is_empty() || !our_counter.is_empty() || !their_counter.is_empty())
    }

    /// C++ `int CvPlayerAI::AI_maxGoldTrade(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:7613-7654)
    pub fn player_ai_max_gold_trade(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let mut max_gold;

        if g.players[p].human || g.players[player].team == g.players[p].team {
            max_gold = g.players[p].gold;

            // FfH: Added by Kael 01/26/2010
            if !g.players[p].human {
                max_gold -= self.player_ai_get_gold_reserve(g, p);
            }
        } else {
            max_gold = g.players[p].total_population;

            max_gold *= self.team_ai_get_has_met_counter(g, g.players[p].team, g.players[player].team) + 10;

            max_gold *= leader(g, p).max_gold_trade_percent;
            max_gold /= 100;
            max_gold -= self.player_ai_get_gold_traded_to(g, p, player);

            let mut research_buffer = -g.player_calculate_gold_rate(p) * 12;
            research_buffer *= g.d.game_speeds[g.game_speed_type()].research_percent;
            research_buffer /= 100;

            max_gold = max_gold.min(g.players[p].gold - research_buffer);

            max_gold = max_gold.min(g.players[p].gold);

            let rem = g.d.defines.diplomacy_value_remainder;
            if rem != 0 {
                max_gold -= max_gold % rem;
            }
        }

        max_gold.max(0)
    }

    /// C++ `int CvPlayerAI::AI_maxGoldPerTurnTrade(PlayerTypes ePlayer) const` (CvPlayerAI.cpp:7657-7678)
    pub fn player_ai_max_gold_per_turn_trade(&self, g: &Game, p: PlayerId, player: PlayerId) -> i32 {
        let mut max_gold_per_turn;

        if g.players[p].human || g.players[player].team == g.players[p].team {
            let length = g.d.defines.peace_treaty_length;
            max_gold_per_turn = g.player_calculate_gold_rate(p) + if length != 0 { g.players[p].gold / length } else { 0 };
        } else {
            max_gold_per_turn = g.players[p].total_population;

            max_gold_per_turn *= leader(g, p).max_gold_per_turn_trade_percent;
            max_gold_per_turn /= 100;

            max_gold_per_turn += g.players[p].gold_per_turn_by_player[player].min(0);
        }

        max_gold_per_turn.min(g.player_calculate_gold_rate(p)).max(0)
    }

    /// C++ `int CvPlayerAI::AI_goldPerTurnTradeVal(int iGoldPerTurn) const` (CvPlayerAI.cpp:7681-7688)
    pub fn player_ai_gold_per_turn_trade_val(&self, g: &Game, p: PlayerId, gold_per_turn: i32) -> i32 {
        let mut value = gold_per_turn * g.d.defines.peace_treaty_length;
        value *= self.player_ai_gold_trade_value_percent(g, p);
        value /= 100;

        value
    }

    /// C++ `int CvPlayerAI::AI_bonusVal(BonusTypes eBonus, int iChange = 1) const` (CvPlayerAI.cpp:7690-7716)
    pub fn player_ai_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType, change: i32) -> i32 {
        // Ported in ffh-core (found_value.rs) because the rules need it.
        g.player_ai_bonus_val(p, bonus, change)
    }

    /// C++ `int CvPlayerAI::AI_baseBonusVal(BonusTypes eBonus) const` (CvPlayerAI.cpp:7719-8038)
    pub fn player_ai_base_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
        // Ported in ffh-core (found_value.rs), cached in `Player::ai_bonus_value`.
        g.player_ai_base_bonus_val(p, bonus)
    }

    /// C++ `int CvPlayerAI::AI_corporationBonusVal(BonusTypes eBonus) const` (CvPlayerAI.cpp:8040-8086)
    pub fn player_ai_corporation_bonus_val(&self, g: &Game, p: PlayerId, bonus: BonusType) -> i32 {
        // Ported in ffh-core (found_value.rs).
        g.player_ai_corporation_bonus_val(p, bonus)
    }

    /// C++ `int CvPlayerAI::AI_bonusTradeVal(BonusTypes eBonus, PlayerTypes ePlayer, int iChange) const` (CvPlayerAI.cpp:8089-8109)
    pub fn player_ai_bonus_trade_val(&self, g: &Game, p: PlayerId, bonus: BonusType, player: PlayerId, change: i32) -> i32 {
        let mut value = self.player_ai_bonus_val(g, p, bonus, change);

        value *= (g.players[p].num_cities().min(g.players[player].num_cities()) + 3) * 30;
        value /= 100;

        value *= (g.d.bonuses[bonus].ai_trade_modifier + 100).max(0);
        value /= 100;

        let their_team = g.players[player].team;
        if g.teams[their_team].vassal[g.players[p].team] && !g.teams[their_team].capitulated {
            value /= 2;
        }

        value * g.d.defines.peace_treaty_length
    }

    /// C++ `DenialTypes CvPlayerAI::AI_bonusTrade(BonusTypes eBonus, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8112-8270)
    pub fn player_ai_bonus_trade(&self, g: &Game, p: PlayerId, bonus: BonusType, player: PlayerId) -> Option<DenialType> {
        let team = g.players[p].team;
        let their_team = g.players[player].team;
        let human = g.players[p].human;

        if human && g.players[player].human {
            return None;
        }

        if g.teams[team].vassal[their_team] {
            return None;
        }

        if g.at_war(team, their_team) {
            return None;
        }

        if their_team == team {
            return None;
        }

        if g.player_get_num_available_bonuses(player, bonus) > 0 && g.player_ai_corporation_bonus_val(player, bonus) <= 0 {
            return Some(if g.players[player].human { DenialType::DENIAL_JOKING } else { DenialType::DENIAL_NO_GAIN });
        }

        if human {
            return None;
        }

        if self.team_ai_get_worst_enemy(g, team) == Some(their_team) {
            return Some(DenialType::DENIAL_WORST_ENEMY);
        }

        if self.player_ai_corporation_bonus_val(g, p, bonus) > 0 {
            return Some(DenialType::DENIAL_JOKING);
        }

        let mut strategic = false;

        for ui in g.d.units.iter() {
            if ui.prereq_and_bonus == Some(bonus) {
                strategic = true;
            }

            if ui.prereq_or_bonuses.contains(&bonus) {
                strategic = true;
            }
        }

        for bi in g.d.buildings.iter() {
            if bi.prereq_and_bonus == Some(bonus) {
                strategic = true;
            }

            if bi.prereq_or_bonuses.contains(&Some(bonus)) {
                strategic = true;
            }
        }

        // XXX marble and stone???

        let attitude = self.player_ai_get_attitude(g, p, player, true).i();
        let lh = leader(g, p);
        let bi = &g.d.bonuses[bonus];

        if strategic {
            // BETTER AI (Better Diplomatics) Sephi
            let imod = same_alignment_or_religion_mod(g, player, p);
            if attitude + imod <= att_i(lh.strategic_bonus_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }

        if bi.happiness > 0 {
            // BETTER AI (Better Diplomatics) Sephi
            let imod = same_alignment_or_religion_mod(g, player, p);
            if attitude + imod <= att_i(lh.happiness_bonus_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }

        if bi.health > 0 {
            // BETTER AI (Better Diplomatics) Sephi
            let imod = same_alignment_or_religion_mod(g, player, p);
            if attitude + imod <= att_i(lh.health_bonus_refuse_attitude_threshold) {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_cityTradeVal(CvCity* pCity) const` (CvPlayerAI.cpp:8273-8318)
    pub fn player_ai_city_trade_val(&self, g: &Game, p: PlayerId, city: CityId) -> i32 {
        let team = g.players[p].team;
        let cty = &g.cities[city];

        let mut value = 300;

        value += cty.population * 50;

        value += opt_to_i32(cty.culture_level) * 200;

        value += ((((cty.population * 50) + g.elapsed_game_turns + 100) * 4) * g.plot_calculate_culture_percent(g.city_plot(city), cty.owner)) / 100;

        for i in 0..NUM_CITY_PLOTS {
            if let Some(loop_plot) = g.plot_city(cty.x, cty.y, i) {
                if let Some(b) = g.plot_get_bonus_type(loop_plot, Some(team)) {
                    value += self.player_ai_bonus_val(g, p, b, 1) * 10;
                }
            }
        }

        if !cty.ever_owned[p] {
            value *= 3;
            value /= 2;
        }

        let rem = g.d.defines.diplomacy_value_remainder;
        if rem != 0 {
            value -= value % rem;
        }

        if g.players[p].human {
            value.max(rem)
        } else {
            value
        }
    }

    /// C++ `DenialTypes CvPlayerAI::AI_cityTrade(CvCity* pCity, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8321-8374)
    pub fn player_ai_city_trade(&self, g: &Game, p: PlayerId, city: CityId, player: PlayerId) -> Option<DenialType> {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        if g.city_liberation_player(city, false) == Some(player) {
            return None;
        }

        if !g.players[player].human {
            if their_team != team {
                if g.plot_calculate_culture_percent(g.city_plot(city), player) == 0 && !g.cities[city].ever_owned[player] && g.players[player].num_cities() > 3 {
                    if self.player_ai_is_financial_trouble(g, player) {
                        return Some(DenialType::DENIAL_UNKNOWN);
                    }

                    let (cx, cy) = (g.cities[city].x, g.cities[city].y);
                    let nearest = g.map_find_city(cx, cy, Some(player), None, true, false, None, None, Some(city));

                    match nearest {
                        None => return Some(DenialType::DENIAL_UNKNOWN),
                        Some(n) => {
                            if g.plot_distance(cx, cy, g.cities[n].x, g.cities[n].y) > 9 {
                                return Some(DenialType::DENIAL_UNKNOWN);
                            }
                        }
                    }
                }
            }
        }

        if g.players[p].human {
            return None;
        }

        if g.at_war(team, their_team) {
            return None;
        }

        if their_team != team {
            return Some(DenialType::DENIAL_NEVER);
        }

        if g.city_calculate_culture_percent(city, p) > 50 {
            return Some(DenialType::DENIAL_TOO_MUCH);
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_stopTradingTradeVal(TeamTypes eTradeTeam, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8377-8471)
    pub fn player_ai_stop_trading_trade_val(&self, g: &Game, p: PlayerId, trade_team: TeamId, player: PlayerId) -> i32 {
        let their_team = g.players[player].team;

        let mut value = 50 + (g.game_turn() / 2);
        value += g.teams[trade_team].num_cities * 5;

        let mut modifier = 0;

        // GET_TEAM(their team).AI_getAttitude(eTradeTeam): another team's attitude (its controller).
        let attitude = g.team_ai_get_attitude(their_team, trade_team, true);
        match attitude {
            AttitudeType::ATTITUDE_FURIOUS => {}
            AttitudeType::ATTITUDE_ANNOYED => modifier += 25,
            AttitudeType::ATTITUDE_CAUTIOUS => modifier += 50,
            AttitudeType::ATTITUDE_PLEASED => modifier += 100,
            AttitudeType::ATTITUDE_FRIENDLY => modifier += 200,
            _ => {}
        }

        value *= (modifier + 100).max(0);
        value /= 100;

        if g.team_is_open_borders(their_team, trade_team) {
            value *= 2;
        }

        if g.teams[their_team].defensive_pact[trade_team] {
            value *= 3;
        }

        for (d, deal) in g.deals.iter() {
            if g.deal_is_cancelable(d, Some(p)) && !g.deal_is_peace_deal(d) {
                if g.players[deal.first_player].team == their_team {
                    if !deal.second_trades.is_empty() {
                        value += self.player_ai_deal_val(g, deal.first_player, deal.second_player, &deal.second_trades, false, 1) * if deal.first_trades.is_empty() { 2 } else { 1 };
                    }
                }

                if g.players[deal.second_player].team == their_team {
                    if !deal.first_trades.is_empty() {
                        value += self.player_ai_deal_val(g, deal.second_player, deal.first_player, &deal.first_trades, false, 1) * if deal.second_trades.is_empty() { 2 } else { 1 };
                    }
                }
            }
        }

        if g.teams[their_team].vassal[g.players[p].team] {
            value /= 2;
        }

        let rem = g.d.defines.diplomacy_value_remainder;
        if rem != 0 {
            value -= value % rem;
        }

        if g.players[p].human {
            value.max(rem)
        } else {
            value
        }
    }

    /// C++ `DenialTypes CvPlayerAI::AI_stopTradingTrade(TeamTypes eTradeTeam, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8474-8564)
    pub fn player_ai_stop_trading_trade(&self, g: &Game, p: PlayerId, trade_team: TeamId, player: PlayerId) -> Option<DenialType> {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        if g.players[p].human {
            return None;
        }

        if g.teams[team].vassal[their_team] {
            return None;
        }

        if g.teams[team].vassal[trade_team] {
            return Some(DenialType::DENIAL_POWER_THEM);
        }

        let attitude = self.team_ai_get_attitude(g, team, their_team, true).i();

        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == team {
                    // BETTER AI (Better Diplomatics) Sephi
                    let imod = same_alignment_or_religion_mod(g, i, p);
                    if attitude + imod <= att_i(leader(g, i).stop_trading_refuse_attitude_threshold) {
                        return Some(DenialType::DENIAL_ATTITUDE);
                    }
                }
            }
        }

        let attitude_them = self.team_ai_get_attitude(g, team, trade_team, true).i();

        for i in g.player_ids() {
            if g.players[i].alive {
                if g.players[i].team == team {
                    // BETTER AI (Better Diplomatics) Sephi.
                    // C++ quirk: iMod is computed but not used in this comparison.
                    let _imod = same_alignment_or_religion_mod(g, i, p);
                    if attitude_them > att_i(leader(g, i).stop_trading_them_refuse_attitude_threshold) {
                        return Some(DenialType::DENIAL_ATTITUDE_THEM);
                    }
                }
            }
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_civicTradeVal(CivicTypes eCivic, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8567-8599)
    pub fn player_ai_civic_trade_val(&self, g: &Game, p: PlayerId, civic: CivicType, player: PlayerId) -> i32 {
        let mut value = 2 * (g.players[p].total_population + g.players[player].total_population); // XXX

        // GET_PLAYER(ePlayer).AI_bestCivic / AI_civicValue: valuations of the other player (our port).
        let best_civic = match g.d.civics[civic].civic_option_type {
            Some(opt) => self.player_ai_best_civic(g, player, opt),
            None => None,
        };

        if let Some(best) = best_civic {
            if best != civic {
                value += ((self.player_ai_civic_value(g, player, best) - self.player_ai_civic_value(g, player, civic)) * 2).max(0);
            }
        }

        if g.teams[g.players[player].team].vassal[g.players[p].team] {
            value /= 2;
        }

        let rem = g.d.defines.diplomacy_value_remainder;
        if rem != 0 {
            value -= value % rem;
        }

        if g.players[p].human {
            value.max(rem)
        } else {
            value
        }
    }

    /// C++ `DenialTypes CvPlayerAI::AI_civicTrade(CivicTypes eCivic, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8602-8666)
    pub fn player_ai_civic_trade(&self, g: &Game, p: PlayerId, civic: CivicType, player: PlayerId) -> Option<DenialType> {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        if g.players[p].human {
            return None;
        }

        if g.teams[team].vassal[their_team] {
            return None;
        }

        if g.at_war(team, their_team) {
            return None;
        }

        if their_team == team {
            return None;
        }

        let option = g.d.civics[civic].civic_option_type;

        if let Some(opt) = option {
            // PORT ADAPT: C++ indexes with getCivics(option), always set in a live game.
            let current_anger = match g.players[p].civics[opt] {
                Some(cur) => g.player_get_civic_percent_anger(p, cur, true),
                None => 0,
            };
            if current_anger > g.player_get_civic_percent_anger(p, civic, false) {
                return Some(DenialType::DENIAL_ANGER_CIVIC);
            }
        }

        let lh = leader(g, p);
        if let Some(favorite) = lh.favorite_civic {
            if g.player_is_civic(p, favorite) && option == g.d.civics[favorite].civic_option_type {
                return Some(DenialType::DENIAL_FAVORITE_CIVIC);
            }
        }

        if let (Some(civ), Some(opt)) = (g.player_get_civilization_type(p), option) {
            if g.d.civilizations[civ].civilization_initial_civics[opt] == Some(civic) {
                return Some(DenialType::DENIAL_JOKING);
            }
        }

        // BETTER AI (Better Diplomatics) Sephi.
        let imod = same_alignment_or_religion_mod(g, player, p);

        // C++ quirk: Sephi's `if (attitude + iMod <= threshold)` has no body of its own, so the
        // original check below is its body: both must hold.
        let threshold = att_i(lh.adopt_civic_refuse_attitude_threshold);
        if self.player_ai_get_attitude(g, p, player, true).i() + imod <= threshold {
            if self.player_ai_get_attitude(g, p, player, true).i() <= threshold {
                return Some(DenialType::DENIAL_ATTITUDE);
            }
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_religionTradeVal(ReligionTypes eReligion, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8669-8701)
    pub fn player_ai_religion_trade_val(&self, g: &Game, p: PlayerId, religion: ReligionType, player: PlayerId) -> i32 {
        let mut value = 3 * (g.players[p].total_population + g.players[player].total_population); // XXX

        // GET_PLAYER(ePlayer).AI_bestReligion / AI_religionValue: valuations of the other player.
        let best_religion = self.player_ai_best_religion(g, player);

        if let Some(best) = best_religion {
            if best != religion {
                value += (self.player_ai_religion_value(g, player, best) - self.player_ai_religion_value(g, player, religion)).max(0);
            }
        }

        if g.teams[g.players[player].team].vassal[g.players[p].team] {
            value /= 2;
        }

        let rem = g.d.defines.diplomacy_value_remainder;
        if rem != 0 {
            value -= value % rem;
        }

        if g.players[p].human {
            value.max(rem)
        } else {
            value
        }
    }

    /// C++ `DenialTypes CvPlayerAI::AI_religionTrade(ReligionTypes eReligion, PlayerTypes ePlayer) const` (CvPlayerAI.cpp:8704-8740)
    pub fn player_ai_religion_trade(&self, g: &Game, p: PlayerId, religion: ReligionType, player: PlayerId) -> Option<DenialType> {
        let team = g.players[p].team;
        let their_team = g.players[player].team;

        if g.players[p].human {
            return None;
        }

        if g.teams[team].vassal[their_team] {
            return None;
        }

        if g.at_war(team, their_team) {
            return None;
        }

        if their_team == team {
            return None;
        }

        if let Some(state) = g.player_get_state_religion(p) {
            let counts = &g.players[p].has_religion_count;
            if counts[religion] < (counts[state] - 1).min(g.players[p].num_cities() / 2) {
                return Some(DenialType::DENIAL_MINORITY_RELIGION);
            }
        }

        if self.player_ai_get_attitude(g, p, player, true).i() <= att_i(leader(g, p).convert_religion_refuse_attitude_threshold) {
            return Some(DenialType::DENIAL_ATTITUDE);
        }

        None
    }

    /// C++ `int CvPlayerAI::AI_unitImpassableCount(UnitTypes eUnit) const` (CvPlayerAI.cpp:8743-8771)
    pub fn player_ai_unit_impassable_count(&self, g: &Game, p: PlayerId, unit: UnitType) -> i32 {
        let team = g.players[p].team;
        let ui = &g.d.units[unit];
        let mut count = 0;

        for t in g.d.terrains.keys() {
            if ui.terrain_impassable[t] {
                let tech = ui.terrain_passable_tech[t];
                if tech.is_none() || !g.team_is_has_tech(team, tech) {
                    count += 1;
                }
            }
        }

        for f in g.d.features.keys() {
            if ui.feature_impassable[f] {
                let tech = ui.feature_passable_tech[f];
                if tech.is_none() || !g.team_is_has_tech(team, tech) {
                    count += 1;
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_unitValue(UnitTypes eUnit, UnitAITypes eUnitAI, CvArea* pArea) const` (CvPlayerAI.cpp:8774-9701)
    pub fn player_ai_unit_value(&self, g: &Game, p: PlayerId, unit: UnitType, unit_ai: UnitAIType, area: AreaId) -> i32 {
        self.player_ai_unit_value_opt(g, p, unit, unit_ai, Some(area))
    }

    /// C++ `CvPlayerAI::AI_unitValue(eUnit, eUnitAI, pArea)` with a possibly NULL `pArea`
    /// (`None`), as several C++ callers pass (see the module comment).
    pub fn player_ai_unit_value_opt(&self, g: &Game, p: PlayerId, unit: UnitType, unit_ai: UnitAIType, area: Option<AreaId>) -> i32 {
        let ui = &g.d.units[unit];
        let team = g.players[p].team;

        if ui.domain_type != self.player_ai_unit_ai_domain_type(g, p, unit_ai) {
            if unit_ai != UnitAIType::UNITAI_ICBM {
                // XXX
                return 0;
            }
        }

        if ui.not_unit_ai_type[unit_ai] {
            return 0;
        }

        let mut valid = ui.unit_ai_type[unit_ai];

        if !valid {
            match unit_ai {
                UnitAIType::UNITAI_UNKNOWN => {}

                UnitAIType::UNITAI_ANIMAL => {
                    if ui.animal {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_SETTLE => {
                    if ui.found {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_WORKER => {
                    if ui.builds.iter().any(|&b| b) {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_ATTACK => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_ATTACK_CITY => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            if !ui.no_capture {
                                valid = true;
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_COLLATERAL => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            if ui.collateral_damage > 0 {
                                valid = true;
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_PILLAGE => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_RESERVE => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_COUNTER => {
                    if ui.combat > 0 {
                        if !ui.only_defensive {
                            if ui.interception_probability > 0 {
                                valid = true;
                            }

                            if !valid {
                                for uc in g.d.unit_classes.keys() {
                                    if ui.unit_class_attack_modifier[uc] > 0 {
                                        valid = true;
                                        break;
                                    }

                                    if ui.target_unit_class[uc] {
                                        valid = true;
                                        break;
                                    }
                                }
                            }

                            if !valid {
                                for ucb in g.d.unit_combats.keys() {
                                    if ui.unit_combat_modifier[ucb] > 0 {
                                        valid = true;
                                        break;
                                    }

                                    if ui.target_unit_combat[ucb] {
                                        valid = true;
                                        break;
                                    }
                                }
                            }

                            if !valid {
                                for other in g.d.units.iter() {
                                    if let Some(uc) = ui.unit_class_type {
                                        if other.defender_unit_class[uc] {
                                            valid = true;
                                            break;
                                        }
                                    }

                                    if let Some(ucb) = ui.unit_combat_type {
                                        if other.defender_unit_combat[ucb] {
                                            valid = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_CITY_DEFENSE => {
                    if ui.combat > 0 {
                        if !ui.no_defensive_bonus {
                            if ui.city_defense_modifier > 0 {
                                valid = true;
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_CITY_COUNTER => {
                    if ui.combat > 0 {
                        if !ui.no_defensive_bonus {
                            if ui.interception_probability > 0 {
                                valid = true;
                            }

                            if !valid {
                                for uc in g.d.unit_classes.keys() {
                                    if ui.unit_class_defense_modifier[uc] > 0 {
                                        valid = true;
                                        break;
                                    }
                                }
                            }

                            if !valid {
                                for ucb in g.d.unit_combats.keys() {
                                    if ui.unit_combat_modifier[ucb] > 0 {
                                        valid = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_CITY_SPECIAL => {}

                UnitAIType::UNITAI_PARADROP => {
                    if ui.drop_range > 0 {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_EXPLORE => {
                    if ui.combat > 0 {
                        if self.player_ai_unit_impassable_count(g, p, unit) == 0 {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_MISSIONARY => {
                    if let Some(a) = area {
                        for r in g.d.religions.keys() {
                            if ui.religion_spreads[r] > 0 {
                                let needed_missionaries = self.player_ai_needed_missionaries(g, p, a, r);

                                if needed_missionaries > 0 {
                                    if needed_missionaries > g.player_count_religion_spread_units(p, a, r) {
                                        valid = true;
                                        break;
                                    }
                                }
                            }
                        }

                        for corp in g.d.corporations.keys() {
                            if ui.corporation_spreads[corp] > 0 {
                                let needed_missionaries = self.player_ai_needed_executives(g, p, a, corp);

                                if needed_missionaries > 0 {
                                    if needed_missionaries > g.player_count_corporation_spread_units(p, a, corp) {
                                        valid = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_PROPHET
                | UnitAIType::UNITAI_ARTIST
                | UnitAIType::UNITAI_SCIENTIST
                | UnitAIType::UNITAI_GENERAL
                | UnitAIType::UNITAI_MERCHANT
                | UnitAIType::UNITAI_ENGINEER
                | UnitAIType::UNITAI_SPY => {}

                // BETTER AI (New UNITAI) Sephi
                UnitAIType::UNITAI_MAGE | UnitAIType::UNITAI_WARWIZARD | UnitAIType::UNITAI_TERRAFORMER | UnitAIType::UNITAI_MANA_UPGRADE => {
                    valid = true;
                }

                UnitAIType::UNITAI_ICBM => {
                    if ui.nuke_range != -1 {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_WORKER_SEA => {
                    if ui.builds.iter().any(|&b| b) {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_ATTACK_SEA => {
                    if ui.combat > 0 {
                        // FfH: Modified by Kael 11/22/2008 (no cargo space test)
                        if !ui.invisible && ui.invisible_type.is_none() {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_RESERVE_SEA => {
                    if ui.combat > 0 {
                        // FfH: Modified by Kael 11/22/2008 (no cargo space test)
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_ESCORT_SEA => {
                    if ui.combat > 0 {
                        // FfH: Modified by Kael 11/22/2008 (no cargo space test)
                        if self.player_ai_unit_impassable_count(g, p, unit) == 0 {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_EXPLORE_SEA => {
                    if ui.cargo_space <= 1 {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_ASSAULT_SEA | UnitAIType::UNITAI_SETTLER_SEA => {
                    if ui.cargo_space > 0 {
                        if ui.special_cargo.is_none() {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_MISSIONARY_SEA | UnitAIType::UNITAI_SPY_SEA | UnitAIType::UNITAI_CARRIER_SEA | UnitAIType::UNITAI_MISSILE_CARRIER_SEA => {
                    if ui.cargo_space > 0 {
                        if let Some(special) = ui.special_cargo {
                            // C++ quirk: the loop over NUM_UNITAI_TYPES repeats one test.
                            if g.d.special_units[special].carrier_unit_ai_type[unit_ai] {
                                valid = true;
                            }
                        }
                    }
                }

                UnitAIType::UNITAI_PIRATE_SEA => {
                    let hidden_promotion = opt_from_i32::<PromotionType>(g.d.defines.hidden_nationality_promotion).map_or(false, |pr| ui.free_promotions[pr]);
                    // FfH: Added by Kael 11/22/2008 (the free hidden nationality promotion)
                    if (ui.always_hostile && ui.hidden_nationality) || hidden_promotion {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_ATTACK_AIR => {
                    if ui.air_combat > 0 {
                        if !ui.suicide {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_DEFENSE_AIR => {
                    if ui.interception_probability > 0 {
                        valid = true;
                    }
                }

                UnitAIType::UNITAI_CARRIER_AIR => {
                    if ui.air_combat > 0 {
                        if ui.interception_probability > 0 {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_MISSILE_AIR => {
                    if ui.air_combat > 0 {
                        if ui.suicide {
                            valid = true;
                        }
                    }
                }

                UnitAIType::UNITAI_ATTACK_CITY_LEMMING => {
                    valid = false;
                }

                _ => {
                    // not all new UNITAIs added Sephi
                }
            }
        }

        if !valid {
            return 0;
        }

        // FfH: Modified by Kael 05/07/2008 (was GC.getGameINLINE().AI_combatValue(eUnit))
        let combat_value = self.player_ai_combat_value(g, p, unit) * 3;

        let mut value = 1;

        value += ui.ai_weight;

        match unit_ai {
            UnitAIType::UNITAI_UNKNOWN | UnitAIType::UNITAI_ANIMAL => {}

            UnitAIType::UNITAI_SETTLE => {
                value += ui.moves * 100;
            }

            UnitAIType::UNITAI_WORKER => {
                for &b in ui.builds.iter() {
                    if b {
                        value += 50;
                    }
                }
                value += ui.moves * 100;
            }

            UnitAIType::UNITAI_ATTACK => {
                let fast_mover_multiplier = if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FASTMOVERS) { 2 } else { 1 };

                value += combat_value;
                value += (combat_value * ui.moves * fast_mover_multiplier) / 2;
                value += (combat_value * ui.withdrawal_probability) / 100;
                if ui.combat_limit < 100 {
                    value -= (combat_value * (125 - ui.combat_limit)) / 100;
                }
            }

            UnitAIType::UNITAI_ATTACK_CITY => {
                let fast_mover_multiplier = if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FASTMOVERS) { 4 } else { 1 };

                let temp_value = ((combat_value * combat_value) / 75) + (combat_value / 2);
                value += temp_value;
                if ui.no_defensive_bonus {
                    value -= temp_value / 2;
                }
                if ui.drop_range > 0 {
                    value -= temp_value / 2;
                }
                if ui.first_strike_immune {
                    value += (temp_value * 8) / 100;
                }
                value += (combat_value * ui.city_attack_modifier) / 100;
                value += (combat_value * ui.collateral_damage) / 400;
                value += (combat_value * ui.moves * fast_mover_multiplier) / 4;
                value += (combat_value * ui.withdrawal_probability) / 100;
                if !self.player_ai_is_do_strategy(g, p, AI_STRATEGY_AIR_BLITZ) {
                    let mut bombard_value = ui.bombard_rate * 4;
                    if bombard_value > 0 {
                        // Percentage change
                        // Total Bombard == 0 : 600%
                        // Total Bombard == 100 : 200%
                        // Total Bombard == 200: 100%
                        // Total Bombard == 300: 50%
                        let total_bombard_rate = self.player_ai_calculate_total_bombard(g, p, DomainType::DOMAIN_LAND);
                        if total_bombard_rate < 100 {
                            bombard_value *= 4 * (200 - total_bombard_rate);
                            bombard_value /= 100;
                        } else {
                            bombard_value *= 100;
                            bombard_value /= total_bombard_rate.min(400);
                        }
                        value += bombard_value;
                    }
                }
            }

            UnitAIType::UNITAI_COLLATERAL => {
                value += combat_value;
                value += (combat_value * ui.collateral_damage) / 50;
                value += (combat_value * ui.moves) / 4;
                value += (combat_value * ui.withdrawal_probability) / 25;
                value -= (combat_value * ui.city_attack_modifier) / 100;
            }

            UnitAIType::UNITAI_PILLAGE => {
                value += combat_value;
                value += combat_value * ui.moves;
            }

            UnitAIType::UNITAI_RESERVE => {
                value += combat_value;
                value -= (combat_value * ui.collateral_damage) / 200;
                for ucb in g.d.unit_combats.keys() {
                    value += (combat_value * ui.unit_combat_modifier[ucb] * self.player_ai_get_unit_combat_weight(g, p, ucb)) / 12000;
                }
                value += (combat_value * ui.moves) / 2;
            }

            UnitAIType::UNITAI_COUNTER => {
                value += combat_value / 2;
                for uc in g.d.unit_classes.keys() {
                    value += (combat_value * ui.unit_class_attack_modifier[uc] * self.player_ai_get_unit_class_weight(g, p, uc)) / 7500;
                    value += (combat_value * if ui.target_unit_class[uc] { 50 } else { 0 }) / 100;
                }
                for ucb in g.d.unit_combats.keys() {
                    value += (combat_value * ui.unit_combat_modifier[ucb] * self.player_ai_get_unit_combat_weight(g, p, ucb)) / 10000;
                    value += (combat_value * if ui.target_unit_combat[ucb] { 50 } else { 0 }) / 100;
                }
                for other in g.d.units.iter() {
                    if let Some(uc) = ui.unit_class_type {
                        if other.defender_unit_class[uc] {
                            value += (50 * combat_value) / 100;
                        }
                    }

                    if let Some(ucb) = ui.unit_combat_type {
                        if other.defender_unit_combat[ucb] {
                            value += (50 * combat_value) / 100;
                        }
                    }
                }

                value += (combat_value * ui.moves) / 2;
                value += (combat_value * ui.withdrawal_probability) / 100;
                value += ui.interception_probability * 2;
            }

            UnitAIType::UNITAI_CITY_DEFENSE => {
                value += (combat_value * 2) / 3;
                value += (combat_value * ui.city_defense_modifier) / 75;
            }

            // BETTER AI (New UNITAI) Sephi
            UnitAIType::UNITAI_MAGE | UnitAIType::UNITAI_TERRAFORMER | UnitAIType::UNITAI_MANA_UPGRADE | UnitAIType::UNITAI_WARWIZARD => {
                value += combat_value;
            }

            UnitAIType::UNITAI_CITY_COUNTER | UnitAIType::UNITAI_CITY_SPECIAL | UnitAIType::UNITAI_PARADROP => {
                value += combat_value / 2;
                value += (combat_value * ui.city_defense_modifier) / 100;
                for uc in g.d.unit_classes.keys() {
                    value += (combat_value * ui.unit_class_attack_modifier[uc] * self.player_ai_get_unit_class_weight(g, p, uc)) / 10000;
                    value += (combat_value * if ui.defender_unit_class[uc] { 50 } else { 0 }) / 100;
                }
                for ucb in g.d.unit_combats.keys() {
                    value += (combat_value * ui.unit_combat_modifier[ucb] * self.player_ai_get_unit_combat_weight(g, p, ucb)) / 10000;
                    value += (combat_value * if ui.defender_unit_combat[ucb] { 50 } else { 0 }) / 100;
                }
                value += ui.interception_probability * 3;
            }

            UnitAIType::UNITAI_EXPLORE => {
                value += combat_value / 2;
                value += ui.moves * 200;
                if ui.no_bad_goodies {
                    value += 100;
                }
            }

            UnitAIType::UNITAI_MISSIONARY => {
                value += ui.moves * 100;
                let state_religion = g.player_get_state_religion(p);
                if let Some(sr) = state_religion {
                    if ui.religion_spreads[sr] > 0 {
                        value += (5 * ui.religion_spreads[sr]) / 2;
                    }
                }
                for r in g.d.religions.keys() {
                    if ui.religion_spreads[r] != 0 && g.player_has_holy_city(p, r) {
                        value += 80;
                        break;
                    }
                }
                if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                    let mut temp_value = 0;
                    for r in g.d.religions.keys() {
                        if ui.religion_spreads[r] != 0 {
                            temp_value += (50 * g.players[p].num_cities()) / (1 + g.players[p].has_religion_count[r]);
                        }
                    }
                    value += temp_value;
                }
                for corp in g.d.corporations.keys() {
                    if g.player_has_headquarters(p, corp) {
                        if ui.corporation_spreads[corp] > 0 {
                            value += (5 * ui.corporation_spreads[corp]) / 2;
                            // UNOFFICIAL_PATCH 06/03/09 jdog5000: fix potential crash.
                            if let Some(a) = area {
                                value += 300 / g.area_count_has_corporation(a, corp, Some(p)).max(1);
                            }
                        }
                    }
                }
            }

            UnitAIType::UNITAI_PROPHET
            | UnitAIType::UNITAI_ARTIST
            | UnitAIType::UNITAI_SCIENTIST
            | UnitAIType::UNITAI_GENERAL
            | UnitAIType::UNITAI_MERCHANT
            | UnitAIType::UNITAI_ENGINEER => {}

            UnitAIType::UNITAI_SPY => {
                value += ui.moves * 100;
                if ui.sabotage {
                    value += 50;
                }
                if ui.destroy {
                    value += 50;
                }
                if ui.counter_spy {
                    value += 100;
                }
            }

            UnitAIType::UNITAI_ICBM => {
                if ui.nuke_range != -1 {
                    let temp_value = 40 + (ui.nuke_range * 40);
                    if ui.air_range == 0 {
                        value += temp_value;
                    } else {
                        value += (temp_value * ui.air_range.min(10)) / 10;
                    }
                    value += (temp_value * (60 + ui.evasion_probability)) / 100;
                }
            }

            UnitAIType::UNITAI_WORKER_SEA => {
                for &b in ui.builds.iter() {
                    if b {
                        value += 50;
                    }
                }
                value += ui.moves * 100;
            }

            UnitAIType::UNITAI_ATTACK_SEA => {
                value += combat_value;
                value += (combat_value * ui.moves) / 2;
                value += ui.bombard_rate * 4;
            }

            UnitAIType::UNITAI_RESERVE_SEA => {
                value += combat_value;
                value += combat_value * ui.moves;
            }

            UnitAIType::UNITAI_ESCORT_SEA => {
                value += combat_value;
                value += combat_value * ui.moves;
                value += ui.interception_probability * 3;
                if !ui.see_invisible_types.is_empty() {
                    value += 200;
                }
            }

            UnitAIType::UNITAI_EXPLORE_SEA => {
                let mut explore_value = 100;
                if let Some(a) = area {
                    if g.map.areas[a].water {
                        if g.map.areas[a].units_per_player[g.barbarian_player()] > 0 {
                            explore_value += 2 * combat_value;
                        }
                    }
                }
                value += ui.moves * explore_value;
                if ui.always_hostile {
                    value /= 2;
                }
                value /= 1 + self.player_ai_unit_impassable_count(g, p, unit);
            }

            UnitAIType::UNITAI_ASSAULT_SEA | UnitAIType::UNITAI_SETTLER_SEA | UnitAIType::UNITAI_MISSIONARY_SEA | UnitAIType::UNITAI_SPY_SEA => {
                value += combat_value / 2;
                value += ui.moves * 200;
                value += ui.cargo_space * 300;
            }

            UnitAIType::UNITAI_CARRIER_SEA => {
                value += combat_value;
                value += ui.moves * 50;
                value += ui.cargo_space * 400;
            }

            UnitAIType::UNITAI_MISSILE_CARRIER_SEA => {
                value += combat_value * ui.moves;
                value += (25 + combat_value) * (3 + ui.cargo_space);
            }

            UnitAIType::UNITAI_PIRATE_SEA => {
                value += combat_value;
                value += combat_value * ui.moves;
            }

            UnitAIType::UNITAI_ATTACK_AIR => {
                value += combat_value;
                value += (ui.collateral_damage * combat_value) / 100;
                value += 4 * ui.bomb_rate;
                value += (combat_value * (100 + 2 * ui.collateral_damage) * ui.air_range) / 100;
            }

            UnitAIType::UNITAI_DEFENSE_AIR => {
                value += combat_value;
                value += ui.interception_probability * 3;
                value += ui.air_range * combat_value;
            }

            UnitAIType::UNITAI_CARRIER_AIR => {
                value += combat_value;
                value += ui.interception_probability * 2;
                value += ui.air_range * combat_value;
            }

            UnitAIType::UNITAI_MISSILE_AIR => {
                value += combat_value;
                value += 4 * ui.bomb_rate;
                value += ui.air_range * combat_value;
            }

            UnitAIType::UNITAI_ATTACK_CITY_LEMMING => {
                value += combat_value;
            }

            _ => {}
        }

        if combat_value > 0 && (unit_ai == UnitAIType::UNITAI_ATTACK || unit_ai == UnitAIType::UNITAI_ATTACK_CITY) {
            if let Some(a) = area {
                let area_ai = g.map.areas[a].area_ai_type[team];
                if area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_MASSING) {
                    for pr in g.d.promotions.keys() {
                        if ui.free_promotions[pr] {
                            if g.d.promotions[pr].amphib {
                                value *= 133;
                                value /= 100;
                                break;
                            }
                        }
                    }
                }
            }
        }

        value.max(0)
    }

    /// C++ `int CvPlayerAI::AI_totalUnitAIs(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9704-9707)
    pub fn player_ai_total_unit_ais(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> i32 {
        g.players[p].ai_num_train_ai_units[unit_ai] + g.players[p].ai_num_ai_units[unit_ai]
    }

    /// C++ `int CvPlayerAI::AI_totalAreaUnitAIs(CvArea* pArea, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9710-9713)
    pub fn player_ai_total_area_unit_ais(&self, g: &Game, p: PlayerId, area: AreaId, unit_ai: UnitAIType) -> i32 {
        let a = &g.map.areas[area];
        a.num_train_ai_units[p][unit_ai] + a.num_ai_units[p][unit_ai]
    }

    /// C++ `int CvPlayerAI::AI_totalWaterAreaUnitAIs(CvArea* pArea, UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9716-9746)
    pub fn player_ai_total_water_area_unit_ais(&self, g: &Game, p: PlayerId, area: AreaId, unit_ai: UnitAIType) -> i32 {
        let mut count = self.player_ai_total_area_unit_ais(g, p, area, unit_ai);

        for i in g.player_ids() {
            if g.players[i].alive {
                for &c in g.players[i].cities.iter() {
                    if g.city_water_area(c) == Some(area) {
                        count += g.plot_plot_count(g.city_plot(c), Some(Game::puf_is_unit_ai_type), unit_ai.i(), -1, Some(p), None, None, -1, -1);

                        if g.cities[c].owner == p {
                            count += g.city_num_train_unit_ai(c, unit_ai);
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_countCargoSpace(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:9749-9766)
    pub fn player_ai_count_cargo_space(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> i32 {
        let mut count = 0;

        for &u in g.players[p].units.iter() {
            if g.units[u].ai_type == Some(unit_ai) {
                // CvUnit::cargoSpace() = m_iCargoCapacity.
                count += g.units[u].cargo_capacity;
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_neededExplorers(CvArea* pArea) const` (CvPlayerAI.cpp:9769-9819)
    pub fn player_ai_needed_explorers(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        let team = g.players[p].team;
        let num_cities = g.players[p].num_cities();
        let water = g.map.areas[area].water;
        let mut needed = 0;

        if water {
            needed = (needed + (g.area_get_num_unrevealed_tiles(area, team) / 400)).min(2.min((num_cities / 2) + 1));
        } else {
            needed = (needed + (g.area_get_num_unrevealed_tiles(area, team) / 150)).min(3.min((num_cities / 3) + 2));
        }

        if needed == 0 {
            if (g.count_civ_teams_alive() - 1) > g.team_get_has_met_civ_count(team, true) {
                if water {
                    if g.map.find_biggest_area(true) == Some(area) {
                        needed += 1;
                    }
                } else {
                    if let Some(capital) = g.player_get_capital_city(p) {
                        if g.city_area(capital) == area {
                            for other in g.civ_player_ids() {
                                if g.players[other].alive && g.players[other].team != team {
                                    if !g.teams[team].has_met[g.players[other].team] {
                                        if g.map.areas[area].cities_per_player[other] > 0 {
                                            needed += 1;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        needed
    }
}
