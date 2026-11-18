//! Side deals with Houses (docs/PERSONS.md, "The second build", rule 9).
//!
//! A House is a party on the diplomacy screen for anyone who has met it. It trades only this: it
//! takes gold and gold per turn, and gives **Secede** (`TRADE_SECEDE`: it breaks away at once) or
//! **Become your vassal** (`TRADE_HOUSE_VASSAL`: it breaks away and becomes the buyer's House, and
//! by the vassal rules the buyer shares its war with the old liege). War, peace and the rest are
//! its liege's to trade (`player_can_trade_item`). It accepts when it is Cautious or worse and the
//! offer covers its price ([`Game::house_bribe_price`], the one rules function every controller
//! reads): `(loyalty + 6) x 25 x its cities` gold (0 when Furious), gold per turn counted ten
//! times.
//!
//! The liege learns of every offer ([`Game::house_note_offer`]): an AI liege's attitude modifier
//! toward the buyer ([`BRIBE_GRUDGE`]), the House's loyalty list shows `Offered 300 gold by the
//! Hippus` (no amount, [`OFFER_NOTED_TURNS`] turns), and the liege is told (an alert). An accepted
//! offer is noted where the deal is implemented (`deal_start_trade`); a refused one where it is
//! proposed ([`Game::player_offer_house`], the one way to propose it that the client and the AI
//! share).

use ffh_data::*;

use super::secession::Why;
use super::*;
use crate::structs::TradeData;

/// What a House gives in a side deal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BribeKind {
    /// `TRADE_SECEDE`: it breaks away at once.
    Secede,
    /// `TRADE_HOUSE_VASSAL`: it breaks away and becomes the buyer's vassal.
    Vassal,
}

/// The House item of a trade item, if it is one.
pub fn house_item(item: TradeableItem) -> Option<BribeKind> {
    match item {
        TradeableItem::TRADE_SECEDE => Some(BribeKind::Secede),
        TradeableItem::TRADE_HOUSE_VASSAL => Some(BribeKind::Vassal),
        _ => None,
    }
}

impl BribeKind {
    /// Its trade item.
    pub fn item(self) -> TradeableItem {
        match self {
            BribeKind::Secede => TradeableItem::TRADE_SECEDE,
            BribeKind::Vassal => TradeableItem::TRADE_HOUSE_VASSAL,
        }
    }

    /// Its name on the diplomacy screen.
    pub fn name(self) -> &'static str {
        match self {
            BribeKind::Secede => "Secede",
            BribeKind::Vassal => "Become your vassal",
        }
    }
}

/// The worth of an offer to a House: its gold, and its gold per turn counted
/// [`BRIBE_GOLD_PER_TURN_TURNS`] times.
pub fn house_offer_value(items: &[TradeData]) -> i32 {
    items
        .iter()
        .map(|t| match t.item_type {
            TradeableItem::TRADE_GOLD => t.data.max(0),
            TradeableItem::TRADE_GOLD_PER_TURN => t.data.max(0) * BRIBE_GOLD_PER_TURN_TURNS,
            _ => 0,
        })
        .sum()
}

impl Game {
    /// House `h`'s price to secede or become another's vassal (gold; gold per turn counts ten
    /// times): `(loyalty + 6) x 25 x its cities`, 0 when Furious; `None` when it refuses any
    /// offer (Pleased or Friendly: [`Game::house_bribe_refusal`] says so).
    pub fn house_bribe_price(&self, h: HouseId) -> Option<i32> {
        let house = self.houses.get(h)?;
        if !self.players[house.player].alive || self.house_state(h) < LoyaltyState::Cautious {
            return None;
        }
        let cities = self.house_cities(h).len() as i32;
        Some((self.house_loyalty(h) + 6).max(0) * BRIBE_PER_CITY * cities)
    }

    /// Why House `h` does not give `kind` to `buyer` now (`None`: it would, at its price): the
    /// screen's words (`House Arne is Pleased with the Bannor.`).
    pub fn house_bribe_refusal(&self, h: HouseId, buyer: PlayerId, kind: BribeKind) -> Option<String> {
        let Some(house) = self.houses.get(h) else { return Some("Not a House.".into()) };
        let (hp, liege) = (house.player, house.owner);
        let label = self.house_label(h);
        if !self.players[buyer].alive || self.player_is_barbarian(buyer) || self.player_is_minor_civ(buyer) || self.player_is_crusade_host(buyer) || self.players[buyer].house.is_some() {
            return Some(format!("{label} trades with civilizations only."));
        }
        let (bt, lt) = (self.players[buyer].team, self.players[liege].team);
        if bt == lt {
            return Some(format!("{label} pays its liege by tribute."));
        }
        if !self.teams[bt].has_met[self.players[hp].team] {
            return Some(format!("You have not met {label}."));
        }
        if kind == BribeKind::Vassal {
            if let Some(why) = self.team_vassal_refusal(bt) {
                return Some(why.to_string());
            }
            if !self.player_has_persons(buyer) {
                return Some("This civilization has no Houses.".into());
            }
        }
        let state = self.house_state(h);
        if state < LoyaltyState::Cautious {
            return Some(format!("{label} is {} with the {}.", state.name(), self.player_get_civilization_short_description(liege)));
        }
        None
    }

    /// Whether House player `hp` may give `kind` to player `to` in a deal (the rules' check of
    /// `player_can_trade_item`: [`Game::house_bribe_refusal`] says no more).
    pub(crate) fn house_can_give(&self, hp: PlayerId, to: PlayerId, kind: BribeKind) -> bool {
        self.player_house(hp).is_some_and(|h| self.house_bribe_refusal(h, to, kind).is_none())
    }

    /// Whether player `p` may give `item` to player `to` when either is a House (`None`: neither
    /// is). A House gives only Secede and Become your vassal; it takes only gold and gold per turn,
    /// from a civilization that is not its liege.
    pub(crate) fn house_trade_item_allowed(&self, p: PlayerId, to: PlayerId, item: &TradeData) -> Option<bool> {
        if let Some(kind) = house_item(item.item_type) {
            return Some(self.house_can_give(p, to, kind));
        }
        let (from_house, to_house) = (self.player_house(p), self.player_house(to));
        if from_house.is_none() && to_house.is_none() {
            return None;
        }
        let Some(h) = to_house.filter(|_| from_house.is_none()) else { return Some(false) };
        if self.players[self.houses[h].owner].team == self.players[p].team {
            return Some(false);
        }
        Some(match item.item_type {
            TradeableItem::TRADE_GOLD => self.players[p].gold >= item.data,
            TradeableItem::TRADE_GOLD_PER_TURN => true,
            _ => false,
        })
    }

    /// The liege of House player `hp` learns of an offer of `give` by `buyer`: an AI liege's
    /// attitude modifier toward the buyer ([`BRIBE_GRUDGE`]), `Offered N gold by the <buyer>` in
    /// the House's loyalty list, and an alert to the liege.
    pub fn house_note_offer(&mut self, buyer: PlayerId, hp: PlayerId, give: &[TradeData]) {
        let Some(h) = self.player_house(hp) else { return };
        let liege = self.houses[h].owner;
        let gold = house_offer_value(give);
        let by = self.player_get_civilization_short_description(buyer);
        if !self.players[liege].human {
            self.players[liege].ai_attitude_extra[buyer] += BRIBE_GRUDGE;
        }
        let until = Some(self.game_turn() + OFFER_NOTED_TURNS);
        self.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::Offered { gold, by: by.clone() }, amount: 0, until });
        let label = self.house_label(h);
        self.house_msg(h, || format!("The {by} offered {label} {gold} gold to secede."));
        self.tel_person(liege, "bribe", "offered");
    }

    /// `buyer` proposes to House player `hp` a deal of `give` (from the buyer: gold, gold per
    /// turn) for `get` (from the House: Secede or Become your vassal): the liege learns of it, the
    /// House's controller answers (`Controller::consider_offer`), and an accepted deal is
    /// implemented. Returns whether it was accepted. The one way the client and the AI propose a
    /// side deal to a House (an offer the House refuses is noted here; an accepted one where the
    /// deal is implemented).
    pub fn player_offer_house(&mut self, buyer: PlayerId, hp: PlayerId, give: &[TradeData], get: &[TradeData]) -> bool {
        if self.player_house(hp).is_none() || !get.iter().any(|t| house_item(t.item_type).is_some()) {
            return false;
        }
        let accept = self.controller(hp).consider_offer(self, hp, buyer, give, get);
        if accept {
            self.implement_deal(buyer, hp, give, get, false);
            // (A deal the rules refused moved nothing: the offer is still noted.)
            if self.player_house(hp).is_some() {
                self.house_note_offer(buyer, hp, give);
            }
        } else {
            self.house_note_offer(buyer, hp, give);
        }
        accept
    }

    /// The trade item of House player `hp` given to `buyer` takes effect (`deal_start_trade`):
    /// the offer is noted, then the House breaks away (rule 7); for Become your vassal it then
    /// becomes the buyer's vassal (a House of the buyer's).
    pub(crate) fn house_trade(&mut self, hp: PlayerId, buyer: PlayerId, kind: BribeKind, offered: &[TradeData]) {
        let Some(h) = self.player_house(hp) else { return };
        if self.house_bribe_refusal(h, buyer, kind).is_some() {
            return;
        }
        self.house_note_offer(buyer, hp, offered);
        let vassal = kind == BribeKind::Vassal;
        let Some(np) = self.house_break_away(h, Why::Bribe { buyer, vassal }) else { return };
        if vassal {
            let (bt, nt) = (self.players[buyer].team, self.players[np].team);
            if self.team_vassal_refusal(bt).is_none() && !self.teams[nt].vassal[bt] {
                self.team_assign_vassal(bt, nt, false);
            }
        }
    }
}
