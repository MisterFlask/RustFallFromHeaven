//! Gifts and demands (this game's own rules, not FFH2's; docs/HUMAN.md, "Deals"). A proposed deal
//! in which the proposer gives and gets nothing is a gift; one in which it gets and gives nothing
//! is a demand. The receiver of a gift and the payer of a demand record it (`Player::tributes`)
//! with its value in their own trade points (`Controller::trade_value`) for [`TRIBUTE_TURNS`]
//! turns. While recorded, gifts raise the receiver's relations with the giver by a step per
//! [`Game::gift_tier`] of value (at most [`GIFT_MAX_RELATIONS`]), and each demand paid lowers the
//! payer's by one. A player that paid a demand pays no other demand of the same player for
//! [`DEMAND_PAUSE_TURNS`] turns. The relations are the AI's (ffh-ai's `relation_parts`); whether an
//! AI pays a demand is its judgement (`Controller::judge_offer`).
//!
//! Only deals proposed through [`Game::implement_proposal`] (the human's diplomacy screen) record
//! anything: the AI's own one-sided deals (help to an ally, a vassal's offer) and those of events
//! and loans do not.

use crate::game::Game;
use crate::ids::*;
use crate::structs::{TradeData, Tribute};
use ffh_data::Idx;

/// The turns a gift or a paid demand is remembered.
pub const TRIBUTE_TURNS: i32 = 30;
/// The turns after a paid demand in which the payer pays no other demand of the same player.
pub const DEMAND_PAUSE_TURNS: i32 = 10;
/// The most relations gifts can add.
pub const GIFT_MAX_RELATIONS: i32 = 3;

impl Game {
    /// The value of one gift tier: 150 trade points in the first era, 150 more per later era of
    /// the game (`current_era`).
    pub fn gift_tier(&self) -> i32 {
        150 * (self.current_era().map_or(0, |e| e.idx() as i32) + 1)
    }

    /// The relations a total value of gifts gives: a step per [`Game::gift_tier`], at most
    /// [`GIFT_MAX_RELATIONS`].
    pub fn gift_relations(&self, value: i32) -> i32 {
        (value / self.gift_tier().max(1)).clamp(0, GIFT_MAX_RELATIONS)
    }

    /// The tributes `p` recorded about `other` that still count (the last [`TRIBUTE_TURNS`]
    /// turns): gifts received (`demand` false) or demands paid (`demand` true).
    pub fn tributes(&self, p: PlayerId, other: PlayerId, demand: bool) -> impl Iterator<Item = &Tribute> + '_ {
        let since = self.game_turn() - TRIBUTE_TURNS;
        self.players[p].tributes.iter().filter(move |t| t.other == other && t.demand == demand && t.turn > since)
    }

    /// The total value of the gifts `p` received from `from` that still count.
    pub fn gifts_value(&self, p: PlayerId, from: PlayerId) -> i32 {
        self.tributes(p, from, false).map(|t| t.value).sum()
    }

    /// The relations `p` has with `other` from gifts and paid demands: the gifts' tier, less one
    /// per paid demand.
    pub fn tribute_relations(&self, p: PlayerId, other: PlayerId) -> (i32, i32) {
        (self.gift_relations(self.gifts_value(p, other)), -(self.tributes(p, other, true).count() as i32))
    }

    /// The turns before `p` pays another demand of `from` (0: it may now).
    pub fn demand_pause(&self, p: PlayerId, from: PlayerId) -> i32 {
        self.tributes(p, from, true).map(|t| t.turn + DEMAND_PAUSE_TURNS - self.game_turn()).max().unwrap_or(0).max(0)
    }

    /// The total value of the demands of `from` that `p` paid and still counts.
    pub fn demands_value(&self, p: PlayerId, from: PlayerId) -> i32 {
        self.tributes(p, from, true).map(|t| t.value).sum()
    }

    /// A deal `who` proposed to `other` and `other` accepted (C++ `implementDeal`, as the
    /// diplomacy screen sends it): a gift (`who` gets nothing) is recorded on `other`, a demand
    /// (`who` gives nothing) is recorded on `other` as paid, each with its value to `other`,
    /// before the trades move anything; then the deal is implemented.
    pub fn implement_proposal(&mut self, who: PlayerId, other: PlayerId, our_list: &[TradeData], their_list: &[TradeData]) {
        let demand = our_list.is_empty();
        if our_list.is_empty() != their_list.is_empty() {
            let (items, other_gets) = if demand { (their_list, false) } else { (our_list, true) };
            let c = self.controller(other);
            let value: i32 = items.iter().filter_map(|t| c.trade_value(self, other, who, t, other_gets)).sum();
            if value > 0 {
                let turn = self.game_turn();
                let since = turn - TRIBUTE_TURNS;
                let list = &mut self.players[other].tributes;
                list.retain(|t| t.turn > since);
                list.push(Tribute { other: who, turn, value, demand });
            }
        }
        self.implement_deal(who, other, our_list, their_list, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Controller;
    use crate::player::Player;
    use crate::setup::GameSetup;
    use ffh_data::*;

    /// Two freshly reset players, alive on their own teams, met.
    fn two_player_game() -> Game {
        let d = ffh_data::global();
        let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 7);
        let mut g = Game::new(d, setup);
        let (n_players, n_teams) = (g.num_players(), g.num_teams());
        for i in 0..2 {
            let p = PlayerId(i as u8);
            g.players[p] = Player::new(d, p, &g.setup.players[i], n_players, n_teams);
            g.players[p].alive = true;
            g.players[p].ever_alive = true;
            let t = g.players[p].team;
            g.teams[t].num_members = 1;
            g.teams[t].alive_count = 1;
            g.teams[t].ever_alive_count = 1;
        }
        let (ta, tb) = (g.players[PlayerId(0)].team, g.players[PlayerId(1)].team);
        g.teams[ta].has_met[tb] = true;
        g.teams[tb].has_met[ta] = true;
        g
    }

    /// Values every item at 100 points, as a stand-in for an AI's judgement.
    struct Hundred;
    impl Controller for Hundred {
        fn trade_value(&self, _g: &Game, _p: PlayerId, _other: PlayerId, _item: &TradeData, _p_gets: bool) -> Option<i32> {
            Some(100)
        }
    }

    fn gold(n: i32) -> TradeData {
        TradeData { item_type: TradeableItem::TRADE_GOLD, data: n, offering: false, hidden: false }
    }

    /// A gift is recorded on its receiver and a paid demand on its payer, each with the receiver's
    /// (payer's) value; they count for `TRIBUTE_TURNS` turns, give a step of relations per tier,
    /// and a paid demand pauses the next for `DEMAND_PAUSE_TURNS` turns.
    #[test]
    fn gifts_and_demands_are_recorded_and_expire() {
        let mut g = two_player_game();
        let (a, b) = (PlayerId(0), PlayerId(1));
        g.controllers[b] = std::sync::Arc::new(Hundred);
        g.players[a].gold = 100;
        g.players[b].gold = 100;
        let (ta, tb) = (g.players[a].team, g.players[b].team);
        g.teams[ta].gold_trading_count += 1;
        g.teams[tb].gold_trading_count += 1;
        let tier = g.gift_tier();
        assert_eq!(tier, 150, "a tier is 150 points in the first era");
        g.implement_proposal(a, b, &[gold(10), gold(5)], &[]);
        assert_eq!(g.gifts_value(b, a), 200, "the receiver's value of each item");
        assert_eq!(g.tribute_relations(b, a), (1, 0));
        assert_eq!(g.players[a].gold, 85, "the gift was given");
        g.implement_proposal(a, b, &[], &[gold(20)]);
        assert_eq!(g.demands_value(b, a), 100);
        assert_eq!(g.tribute_relations(b, a), (1, -1));
        assert_eq!(g.demand_pause(b, a), DEMAND_PAUSE_TURNS);
        assert_eq!(g.players[a].gold, 105, "the demand was paid");
        // Two-sided deals record nothing.
        g.implement_proposal(a, b, &[gold(1)], &[gold(1)]);
        assert_eq!(g.players[b].tributes.len(), 2);
        g.setup.game_turn += DEMAND_PAUSE_TURNS;
        assert_eq!(g.demand_pause(b, a), 0);
        g.setup.game_turn += TRIBUTE_TURNS - DEMAND_PAUSE_TURNS;
        assert_eq!(g.tribute_relations(b, a), (0, 0), "forgotten after {TRIBUTE_TURNS} turns");
        assert_eq!(g.gifts_value(b, a), 0);
    }
}
