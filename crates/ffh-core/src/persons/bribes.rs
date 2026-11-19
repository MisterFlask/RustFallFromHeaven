//! Side deals with Houses (docs/PERSONS.md, "The second build", rule 9).
//!
//! A House is a party on the diplomacy screen for anyone who has met it. It trades only this: it
//! takes gold (a lump sum only, the owner's decision of 2026-11-18: gold per turn could be paid
//! for a turn and ended by a war on the seceded House), and gives **Secede** (`TRADE_SECEDE`: it
//! breaks away at once) or
//! **Become your vassal** (`TRADE_HOUSE_VASSAL`: it breaks away and becomes the buyer's House, and
//! by the vassal rules the buyer shares its war with the old liege). War, peace, techs and the rest
//! are its liege's to trade (`player_can_trade_item`). It accepts when it is Cautious or worse, the
//! buyer's courtship of it has been open `PERSON_COURTSHIP_TURNS` turns (the third build, rule 7:
//! `courtship.rs`), and the offer covers its price ([`Game::house_bribe_price`], the one rules
//! function every controller reads): `(loyalty + 6) x P x its cities` gold, P a share of the world
//! unit (the third build, rule 8: [`Game::house_price_step`]; one step at least when Furious).
//! Besides its side deals a House takes gifts: gold and resources, from a civilization that may
//! court it, nothing asked back (the courtship's gifts; it accepts any).
//!
//! The liege learns of an accepted offer ([`Game::house_note_offer`], where the deal takes effect:
//! `house_trade`, from `deal_start_trade`): it remembers it (`MEMORY_BOUGHT_HOUSE` toward the
//! buyer, which fades; the House itself was paid and is not angered), the House's loyalty list
//! shows `Offered 300 gold by the Hippus` (no amount, [`OFFER_NOTED_TURNS`] turns), and the liege
//! is told (an alert). A refused offer leaves no trace (the owner's decision, 2026-11-18): no
//! memory, no entry, no alert, no telemetry; the trade table
//! says before an offer is made whether it will be accepted. [`Game::player_offer_house`] is the
//! one way to propose it that the client and the AI share.

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

/// The worth of an offer to a House: its gold (a House takes nothing else).
pub fn house_offer_value(items: &[TradeData]) -> i32 {
    items.iter().filter(|t| t.item_type == TradeableItem::TRADE_GOLD).map(|t| t.data.max(0)).sum()
}

impl Game {
    /// House `h`'s price to secede or become another's vassal (gold): `(loyalty + 6) x P x its
    /// cities`, P [`Game::house_price_step`] (a share of the world unit), one step at least when
    /// Furious; `None` when it refuses any offer (Pleased or Friendly: [`Game::house_bribe_refusal`]
    /// says so).
    pub fn house_bribe_price(&self, h: HouseId) -> Option<i32> {
        let house = self.houses.get(h)?;
        if !self.players[house.player].alive || self.house_state(h) < LoyaltyState::Cautious {
            return None;
        }
        let cities = self.house_cities(h).len() as i32;
        // At least one step even when Furious (until its ultimatum at its liege's next turn): a
        // rival cannot take a House for nothing (docs/PERSONS.md, "The third build", rule 8).
        Some((self.house_loyalty(h) + 6).max(1) * self.house_price_step() * cities)
    }

    /// Why House `h` does not give `kind` to `buyer` now (`None`: it would, at its price): the
    /// screen's words (`House Arne is Pleased with the Bannor.`, `Court House Arne first: gifts
    /// worth 180 (you have given 140).`, `Offers possible from turn 140.`): its terms
    /// ([`Game::house_bribe_terms_refusal`]), then the buyer's courtship (the third build, rule 7:
    /// [`Game::house_courtship_refusal`]).
    pub fn house_bribe_refusal(&self, h: HouseId, buyer: PlayerId, kind: BribeKind) -> Option<String> {
        self.house_bribe_terms_refusal(h, buyer, kind).or_else(|| self.house_courtship_refusal(h, buyer))
    }

    /// [`Game::house_bribe_refusal`], the courtship aside: who may buy it (who may court it,
    /// [`Game::house_courter_refusal`]), whether the buyer may take a vassal (Become your vassal),
    /// and its state (Cautious or worse).
    pub fn house_bribe_terms_refusal(&self, h: HouseId, buyer: PlayerId, kind: BribeKind) -> Option<String> {
        if let Some(why) = self.house_courter_refusal(h, buyer) {
            return Some(why);
        }
        let liege = self.houses[h].owner;
        let label = self.house_label(h);
        if kind == BribeKind::Vassal {
            if let Some(why) = self.team_vassal_refusal(self.players[buyer].team) {
                return Some(why.to_string());
            }
            if !self.player_has_houses(buyer) {
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
    /// is, or the ordinary rules decide). A House gives only Secede and Become your vassal (no
    /// tech: techs, like war, are its liege's to trade); it takes gold (no gold per turn) and
    /// resources (the third build, rule 7: a resource's trade by the ordinary rules, a trade network
    /// and a spare source), from a civilization that may court it ([`Game::house_courter_refusal`]).
    pub(crate) fn house_trade_item_allowed(&self, p: PlayerId, to: PlayerId, item: &TradeData) -> Option<bool> {
        if let Some(kind) = house_item(item.item_type) {
            return Some(self.house_can_give(p, to, kind));
        }
        let (from_house, to_house) = (self.player_house(p), self.player_house(to));
        if from_house.is_none() && to_house.is_none() {
            return None;
        }
        let Some(h) = to_house.filter(|_| from_house.is_none()) else { return Some(false) };
        if self.house_courter_refusal(h, p).is_some() {
            return Some(false);
        }
        match item.item_type {
            TradeableItem::TRADE_GOLD => Some(self.players[p].gold >= item.data),
            TradeableItem::TRADE_RESOURCES => None,
            _ => Some(false),
        }
    }

    /// The liege of House player `hp` learns of an accepted offer of `give` by `buyer` (called
    /// where the deal takes effect, `house_trade`; a refused offer is not noted): it remembers it
    /// (`MEMORY_BOUGHT_HOUSE` toward the buyer, which fades; the House was paid and is not
    /// angered), `Offered N gold by the <buyer>` in the House's loyalty list, and an alert to the
    /// liege.
    pub fn house_note_offer(&mut self, buyer: PlayerId, hp: PlayerId, give: &[TradeData]) {
        let Some(h) = self.player_house(hp) else { return };
        let liege = self.houses[h].owner;
        let gold = house_offer_value(give);
        let by = self.player_get_civilization_short_description(buyer);
        self.player_add_memory(liege, buyer, MemoryType::MEMORY_BOUGHT_HOUSE);
        let until = Some(self.game_turn() + OFFER_NOTED_TURNS);
        self.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::Offered { gold, by: by.clone() }, amount: 0, until });
        let label = self.house_label(h);
        self.house_msg(h, || format!("The {by} offered {label} {gold} gold to secede."));
        self.tel_house(liege, "bribe", "offered");
    }

    /// `buyer` proposes to House player `hp` a deal of `give` (from the buyer: gold) for `get`
    /// (from the House: Secede or Become your vassal), or a gift (`get` empty: gold or resources,
    /// the third build's courtship): the House's controller answers
    /// (`Controller::consider_offer`), and an accepted deal is implemented, where the liege
    /// learns of it (`house_trade`; a gift may open a courtship). Returns whether it was accepted.
    /// A refused offer, or one the rules refuse at its implementation, moves nothing: the liege does
    /// not learn of it. The one way the client and the AI propose a side deal or a gift to a House.
    pub fn player_offer_house(&mut self, buyer: PlayerId, hp: PlayerId, give: &[TradeData], get: &[TradeData]) -> bool {
        let gift = get.is_empty() && !give.is_empty();
        if self.player_house(hp).is_none() || !(gift || get.iter().any(|t| house_item(t.item_type).is_some())) {
            return false;
        }
        let accept = self.controller(hp).consider_offer(self, hp, buyer, give, get);
        if accept {
            self.implement_deal(buyer, hp, give, get, false);
        }
        accept
    }

    /// The trade item of House player `hp` given to `buyer` takes effect (`deal_start_trade`):
    /// the accepted offer is noted (the only place an offer is), then the House breaks away (rule
    /// 7); for Become your vassal it then becomes the buyer's vassal (a House of the buyer's).
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
