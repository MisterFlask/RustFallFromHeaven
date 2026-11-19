//! Courtship and a House's price (docs/PERSONS.md, "The third build", rules 7 and 8).
//!
//! **Courtship.** A civilization that has met a House (not of its liege's team, not a House
//! itself: [`Game::house_courter_refusal`]) courts it with gifts in the trade table: gold, a lump
//! sum recorded on the House (`House::gifts`, counted [`COURTSHIP_GIFT_TURNS`] turns), or a
//! resource, a trade it keeps giving (its live deals with the House's player). Its courtship's value
//! ([`Game::house_courtship_value`]): that gold plus, for each resource it gives now, the
//! requirement divided by [`COURTSHIP_RESOURCE_SHARE`] (a third). The requirement
//! ([`Game::courtship_requirement`]): [`COURTSHIP_UNITS`] turns of the world unit. The courtship
//! opens the turn its value reaches the requirement (`House::courtships`: the turn, per giver) and
//! closes when it falls below; it is checked where a gift arrives or a resource's trade starts or
//! ends (`deal.rs`: [`Game::house_deal_done`]) and in the liege's turn processing (gifts age, the
//! world unit moves: [`Game::player_persons_courtships`]). A House takes an offer (Secede, Become
//! your vassal) only from a civilization whose courtship has been open `PERSON_COURTSHIP_TURNS`
//! turns or more ([`Game::house_courtship_refusal`], read by `house_bribe_refusal`). When a
//! courtship opens, the liege is told (an alert: `The Bannor court House Aldric: offers possible
//! from turn 140.`) and remembers it (`MEMORY_COURTED_HOUSE` toward the giver, which fades); the
//! House, given gifts, is not angered. The House accepts any gift (its controller answers; the AI
//! accepts every one).
//!
//! **The price** ([`Game::house_price_step`]): `(loyalty + 6) x P x its cities`, P
//! [`PRICE_PERCENT_OF_UNIT`] percent of the world unit (25 around turn 150), at least 1.

use std::collections::BTreeSet;

use ffh_data::*;

use super::*;

/// A courtship of a House ([`Game::house_courtships`]): a civilization with gifts that count now,
/// or whose courtship is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourtshipView {
    pub giver: PlayerId,
    /// The gold it gave in the last [`COURTSHIP_GIFT_TURNS`] turns.
    pub gold: i32,
    /// The resources it gives the House now (its live deals).
    pub resources: Vec<BonusType>,
    /// Its value (the gold, and each resource at `resource_worth`) and the requirement.
    pub value: i32,
    pub requirement: i32,
    pub resource_worth: i32,
    /// The game turn it opened (`None`: not open), and the first game turn the House takes its offer.
    pub opened: Option<i32>,
    pub offers_from: Option<i32>,
    /// `Courted by the Bannor (gifts: 120 gold, Wine). Offers possible from turn 140.`, or while it
    /// is not open `Courted by the Bannor (gifts: 120 gold). Not open: 120 of 180.`
    pub line: String,
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // The numbers
    // -----------------------------------------------------------------------------------------

    /// A House's price per step of loyalty and per city (rule 8): [`PRICE_PERCENT_OF_UNIT`] percent
    /// of the world unit, at least 1.
    pub fn house_price_step(&self) -> i32 {
        (self.world_commerce_unit() * PRICE_PERCENT_OF_UNIT / 100).max(1)
    }

    /// A courtship's requirement (rule 7): [`COURTSHIP_UNITS`] turns of the world unit.
    pub fn courtship_requirement(&self) -> i32 {
        (COURTSHIP_UNITS * self.world_commerce_unit()).max(1)
    }

    /// What a resource given counts toward a courtship: the requirement divided by
    /// [`COURTSHIP_RESOURCE_SHARE`].
    pub fn courtship_resource_worth(&self) -> i32 {
        self.courtship_requirement() / COURTSHIP_RESOURCE_SHARE
    }

    // -----------------------------------------------------------------------------------------
    // Who courts, and with what
    // -----------------------------------------------------------------------------------------

    /// Why `giver` may not court House `h` (give it gifts, or make it offers), in the screen's words
    /// (`None`: it may): a civilization (not the barbarians, a minor civilization, a crusade Host or
    /// a House), not of its liege's team, that has met it.
    pub fn house_courter_refusal(&self, h: HouseId, giver: PlayerId) -> Option<String> {
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

    /// The gold `giver` gave House `h` in the last [`COURTSHIP_GIFT_TURNS`] turns (a gift of turn
    /// `t` counts through turn `t + 19`).
    pub fn house_gift_gold(&self, h: HouseId, giver: PlayerId) -> i32 {
        let turn = self.game_turn();
        self.houses.get(h).map_or(0, |house| house.gifts.iter().filter(|g| g.giver == giver && g.turn > turn - COURTSHIP_GIFT_TURNS).map(|g| g.gold).sum())
    }

    /// The resources `giver` gives House `h` now: those of its live deals with the House's player,
    /// in deal order.
    pub fn house_gift_resources(&self, h: HouseId, giver: PlayerId) -> Vec<BonusType> {
        let Some(house) = self.houses.get(h) else { return Vec::new() };
        let hp = house.player;
        let mut out = Vec::new();
        for (_, deal) in self.deals.iter() {
            let list = if deal.first_player == giver && deal.second_player == hp {
                &deal.first_trades
            } else if deal.second_player == giver && deal.first_player == hp {
                &deal.second_trades
            } else {
                continue;
            };
            out.extend(list.iter().filter(|t| t.item_type == TradeableItem::TRADE_RESOURCES).map(|t| BonusType::from_usize(t.data as usize)));
        }
        out
    }

    /// The value of `giver`'s courtship of House `h`: the gold it gave in the last
    /// [`COURTSHIP_GIFT_TURNS`] turns plus [`Game::courtship_resource_worth`] for each resource it
    /// gives now.
    pub fn house_courtship_value(&self, h: HouseId, giver: PlayerId) -> i32 {
        self.house_gift_gold(h, giver) + self.house_gift_resources(h, giver).len() as i32 * self.courtship_resource_worth()
    }

    /// The game turn `giver`'s courtship of House `h` opened, while it is open: recorded, its value
    /// still at the requirement, and `giver` still one who may court it.
    pub fn house_courtship_opened(&self, h: HouseId, giver: PlayerId) -> Option<i32> {
        let opened = self.houses.get(h)?.courtships.iter().find(|c| c.giver == giver)?.opened;
        (self.house_courter_refusal(h, giver).is_none() && self.house_courtship_value(h, giver) >= self.courtship_requirement()).then_some(opened)
    }

    /// The first game turn House `h` takes an offer from `giver` by its courtship: the turn it
    /// opened plus `PERSON_COURTSHIP_TURNS` (`None`: not open).
    pub fn house_courtship_offers_from(&self, h: HouseId, giver: PlayerId) -> Option<i32> {
        self.house_courtship_opened(h, giver).map(|t| t + self.d.defines.person_courtship_turns)
    }

    /// Why `buyer`'s courtship of House `h` does not let it make an offer now, in the screen's
    /// words (`None`: it does): `Court House Aldric first: gifts worth 180 (you have given 140).`,
    /// `Offers possible from turn 140.`
    pub fn house_courtship_refusal(&self, h: HouseId, buyer: PlayerId) -> Option<String> {
        match self.house_courtship_offers_from(h, buyer) {
            None => Some(format!("Court {} first: gifts worth {} (you have given {}).", self.house_label(h), self.courtship_requirement(), self.house_courtship_value(h, buyer))),
            Some(t) if t > self.game_turn() => Some(format!("Offers possible from turn {t}.")),
            Some(_) => None,
        }
    }

    // -----------------------------------------------------------------------------------------
    // The read
    // -----------------------------------------------------------------------------------------

    /// The civilizations that court House `h`: those whose gifts count now or whose courtship is
    /// recorded open, in player order.
    fn house_courters(&self, h: HouseId) -> Vec<PlayerId> {
        let Some(house) = self.houses.get(h) else { return Vec::new() };
        let turn = self.game_turn();
        let mut out: BTreeSet<PlayerId> = house.gifts.iter().filter(|g| g.turn > turn - COURTSHIP_GIFT_TURNS).map(|g| g.giver).collect();
        out.extend(house.courtships.iter().map(|c| c.giver));
        let hp = house.player;
        for (_, deal) in self.deals.iter() {
            for (from, to, list) in [(deal.first_player, deal.second_player, &deal.first_trades), (deal.second_player, deal.first_player, &deal.second_trades)] {
                if to == hp && from != hp && list.iter().any(|t| t.item_type == TradeableItem::TRADE_RESOURCES) {
                    out.insert(from);
                }
            }
        }
        out.into_iter().filter(|&g| self.house_courter_refusal(h, g).is_none()).collect()
    }

    /// A resource's name (`Wine`).
    fn bonus_name(&self, b: BonusType) -> String {
        self.d.text.name(&self.d.bonuses[b].base.description).to_string()
    }

    /// `giver`'s courtship of House `h` (pure).
    pub fn house_courtship_view(&self, h: HouseId, giver: PlayerId) -> CourtshipView {
        let gold = self.house_gift_gold(h, giver);
        let resources = self.house_gift_resources(h, giver);
        let value = self.house_courtship_value(h, giver);
        let requirement = self.courtship_requirement();
        let opened = self.house_courtship_opened(h, giver);
        let offers_from = opened.map(|t| t + self.d.defines.person_courtship_turns);
        let mut gifts: Vec<String> = Vec::new();
        if gold > 0 {
            gifts.push(format!("{gold} gold"));
        }
        gifts.extend(resources.iter().map(|&b| self.bonus_name(b)));
        let by = self.player_get_civilization_short_description(giver);
        let head = if gifts.is_empty() { format!("Courted by the {by}.") } else { format!("Courted by the {by} (gifts: {}).", gifts.join(", ")) };
        let line = match offers_from {
            Some(t) => format!("{head} Offers possible from turn {t}."),
            None => format!("{head} Not open: {value} of {requirement}."),
        };
        CourtshipView { giver, gold, resources, value, requirement, resource_worth: self.courtship_resource_worth(), opened, offers_from, line }
    }

    /// Every courtship of House `h`, open or not yet, in player order (the House card's and the
    /// diplomacy list's lines).
    pub fn house_courtships(&self, h: HouseId) -> Vec<CourtshipView> {
        self.house_courters(h).into_iter().map(|g| self.house_courtship_view(h, g)).collect()
    }

    /// The courter's line in its trade table with House `h`: `Courting House Aldric: 140 of 180
    /// given (Wine counts 60).`; `None` when `giver` gives it nothing that counts and has no
    /// courtship open (the offer's refusal says what to do).
    pub fn house_courting_line(&self, h: HouseId, giver: PlayerId) -> Option<String> {
        let value = self.house_courtship_value(h, giver);
        if value <= 0 && self.house_courtship_opened(h, giver).is_none() {
            return None;
        }
        let worth = self.courtship_resource_worth();
        let counts: Vec<String> = self.house_gift_resources(h, giver).into_iter().map(|b| format!("{} counts {worth}", self.bonus_name(b))).collect();
        let tail = if counts.is_empty() { String::new() } else { format!(" ({})", counts.join(", ")) };
        Some(format!("Courting {}: {value} of {} given{tail}.", self.house_label(h), self.courtship_requirement()))
    }

    // -----------------------------------------------------------------------------------------
    // Gifts and the courtships' turn
    // -----------------------------------------------------------------------------------------

    /// `gold` given by `giver` to player `to` in a deal (`deal_start_trade`): when `to` is a House
    /// that `giver` may court, a gift recorded (its courtships are checked once the deal is in
    /// place, [`Game::house_deal_done`]).
    pub(crate) fn house_gift_received(&mut self, giver: PlayerId, to: PlayerId, gold: i32) {
        let Some(h) = self.player_house(to) else { return };
        if gold <= 0 || self.house_courter_refusal(h, giver).is_some() {
            return;
        }
        let turn = self.game_turn();
        self.houses[h].gifts.push(Gift { giver, gold, turn });
    }

    /// A deal between `a` and `b` started or ended: the courtships of whichever is a House are
    /// checked (a gift of gold arrived, a resource's trade began or stopped).
    pub(crate) fn house_deal_done(&mut self, a: PlayerId, b: PlayerId) {
        for p in [a, b] {
            if let Some(h) = self.player_house(p) {
                self.house_update_courtships(h);
            }
        }
    }

    /// Player `p`'s Houses' courtships checked in its turn processing (gifts age; the world unit,
    /// and with it the requirement, moves).
    pub(crate) fn player_persons_courtships(&mut self, p: PlayerId) {
        for h in self.player_houses(p) {
            self.house_update_courtships(h);
        }
    }

    /// House `h`'s courtships checked: gold older than [`COURTSHIP_GIFT_TURNS`] turns drops out;
    /// a courtship whose value reaches the requirement opens (its liege is told and remembers the
    /// giver: `MEMORY_COURTED_HOUSE`), one that falls below closes; in player order.
    pub(crate) fn house_update_courtships(&mut self, h: HouseId) {
        if !self.houses.contains(h) {
            return;
        }
        let turn = self.game_turn();
        self.houses[h].gifts.retain(|g| g.turn > turn - COURTSHIP_GIFT_TURNS);
        let requirement = self.courtship_requirement();
        let mut givers: BTreeSet<PlayerId> = self.house_courters(h).into_iter().collect();
        givers.extend(self.houses[h].courtships.iter().map(|c| c.giver));
        for giver in givers {
            let open = self.houses[h].courtships.iter().any(|c| c.giver == giver);
            let reaches = self.house_courter_refusal(h, giver).is_none() && self.house_courtship_value(h, giver) >= requirement;
            if reaches && !open {
                self.house_courtship_opens(h, giver);
            } else if !reaches && open {
                self.houses[h].courtships.retain(|c| c.giver != giver);
            }
        }
    }

    /// `giver`'s courtship of House `h` opens this turn: recorded, the liege told (`The Bannor court
    /// House Aldric: offers possible from turn 140.`) and remembering it, a line in the House's
    /// record.
    fn house_courtship_opens(&mut self, h: HouseId, giver: PlayerId) {
        let turn = self.game_turn();
        let courtships = &mut self.houses[h].courtships;
        courtships.push(Courtship { giver, opened: turn });
        courtships.sort_by_key(|c| c.giver);
        let liege = self.houses[h].owner;
        let by = self.player_get_civilization_short_description(giver);
        let label = self.house_label(h);
        let from = turn + self.d.defines.person_courtship_turns;
        self.player_add_memory(liege, giver, MemoryType::MEMORY_COURTED_HOUSE);
        self.house_add_deed(h, format!("Courted by the {by}."));
        self.house_msg(h, || format!("The {by} court {label}: offers possible from turn {from}."));
        self.tel_house(liege, "courtship", "opened");
    }
}
