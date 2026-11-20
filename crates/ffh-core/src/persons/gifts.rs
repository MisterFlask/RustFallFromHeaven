//! Gifts to a House (docs/PERSONS.md, "The fourth build", rule 3; the second build's side deals
//! and the third build's courtship by gifts are retired).
//!
//! A House is a party on the diplomacy screen for anyone who has met it, but it trades only gifts:
//! it takes gold (a lump sum, no gold per turn) and resources (a trade by the ordinary rules: a
//! trade network and a spare source), from a civilization that may give it gifts
//! ([`Game::house_gift_refusal`]: one that has met it, not of its liege's team, not a House),
//! nothing asked back; it gives nothing (war, peace, techs and the rest are its liege's to trade:
//! `player_can_trade_item`). A gift is an ordinary gift (`tribute.rs`: recorded on the House by
//! [`Game::implement_proposal`]): it raises the House leader's attitude toward the giver (the AI's
//! `gifts` relation), which lowers a courtship's requirement (`favour.rs`). Nothing else counts
//! it. [`Game::player_offer_house`] is the one way the client and the AI give a House a gift.
//!
//! The trade table with a House states what a better attitude would bring:
//! [`Game::house_requirement_steps`], a courtship's requirement at the attitude now and at each
//! better one up to Friendly ([`Requirement::at`]).

use ffh_data::*;

use super::*;
use crate::structs::TradeData;

/// The attitudes on the requirement's scale, from Furious (−2) to Friendly (+2).
const STEPS: [AttitudeType; 5] = [AttitudeType::ATTITUDE_FURIOUS, AttitudeType::ATTITUDE_ANNOYED, AttitudeType::ATTITUDE_CAUTIOUS, AttitudeType::ATTITUDE_PLEASED, AttitudeType::ATTITUDE_FRIENDLY];

impl Requirement {
    /// The requirement (culture) at attitude step `attitude` (−2 Furious to +2 Friendly), its
    /// loyalty and world unit as they are (`total` is `at(attitude)`).
    pub fn at(&self, attitude: i32) -> i32 {
        super::favour::requirement_total(self.loyalty, attitude, self.unit)
    }
}

impl Game {
    /// The requirement of `courter`'s courtship of House `h` (culture) at its leader's attitude
    /// toward the courter now and at each better attitude up to Friendly, the House's loyalty and
    /// the world unit as they are (pure; the trade table with a House: what a gift that raises the
    /// attitude would bring): `[(attitude, requirement)]`, the first the attitude now.
    pub fn house_requirement_steps(&self, h: HouseId, courter: PlayerId) -> Vec<(AttitudeType, i32)> {
        let r = self.house_requirement(h, courter);
        STEPS.iter().zip(-2..=2).filter(|&(_, a)| a >= r.attitude).map(|(&t, a)| (t, r.at(a))).collect()
    }

    /// Why `giver` may not give House `h` gifts, in the screen's words (`None`: it may): a
    /// civilization (not the barbarians, a minor civilization, a crusade Host or a House), not of
    /// its liege's team, that has met it.
    pub fn house_gift_refusal(&self, h: HouseId, giver: PlayerId) -> Option<String> {
        let Some(house) = self.houses.get(h) else { return Some("Not a House.".into()) };
        let label = self.house_label(h);
        if !self.players[giver].alive || self.player_is_barbarian(giver) || self.player_is_minor_civ(giver) || self.player_is_crusade_host(giver) || self.players[giver].house.is_some() {
            return Some(format!("{label} trades with civilizations only."));
        }
        let (gt, lt) = (self.players[giver].team, self.players[house.owner].team);
        if gt == lt {
            return Some(format!("{label} pays its liege by tribute."));
        }
        if !self.teams[gt].has_met[self.players[house.player].team] {
            return Some(format!("You have not met {label}."));
        }
        None
    }

    /// Whether player `p` may give `item` to player `to` when either is a House (`None`: neither
    /// is, or the ordinary rules decide). A House gives nothing; it takes gold (no gold per turn)
    /// and resources (by the ordinary rules) from a civilization that may give it gifts.
    pub(crate) fn house_trade_item_allowed(&self, p: PlayerId, to: PlayerId, item: &TradeData) -> Option<bool> {
        let (from_house, to_house) = (self.player_house(p), self.player_house(to));
        if from_house.is_none() && to_house.is_none() {
            return None;
        }
        let Some(h) = to_house.filter(|_| from_house.is_none()) else { return Some(false) };
        if self.house_gift_refusal(h, p).is_some() {
            return Some(false);
        }
        match item.item_type {
            TradeableItem::TRADE_GOLD => Some(self.players[p].gold >= item.data),
            TradeableItem::TRADE_RESOURCES => None,
            _ => Some(false),
        }
    }

    /// `giver` gives House player `hp` the gift `give` (gold or resources, nothing asked back): the
    /// House's controller answers (`Controller::consider_offer`; the AI accepts any gift), and an
    /// accepted gift is implemented as a proposed deal ([`Game::implement_proposal`]: recorded on
    /// the House as a gift, which raises its leader's attitude toward the giver). Returns whether it
    /// was accepted; anything but a gift is refused. The one way the client and the AI give a House
    /// a gift.
    pub fn player_offer_house(&mut self, giver: PlayerId, hp: PlayerId, give: &[TradeData], get: &[TradeData]) -> bool {
        if self.player_house(hp).is_none() || !get.is_empty() || give.is_empty() {
            return false;
        }
        let accept = self.controller(hp).consider_offer(self, hp, giver, give, get);
        if accept {
            self.implement_proposal(giver, hp, give, get);
        }
        accept
    }
}
