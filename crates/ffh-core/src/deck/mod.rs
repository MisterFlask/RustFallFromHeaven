//! The decks (docs/DECKS.md, "The deck as built"): the great-person deck of secret techs
//! ([`Game::deck_draw`], [`Game::unit_discover_card`]) and the Armageddon deck ([`doom`]).
//!
//! **Great-person cards.** A card is a tech with a `card_suit` (`TechInfo::card`): it is never
//! researched or traded. A great person of a suit's class (Sage `UNITCLASS_SCIENTIST`, Prophet
//! `UNITCLASS_PROPHET`, Engineer `UNITCLASS_ENGINEER`, Merchant `UNITCLASS_MERCHANT`, Bard
//! `UNITCLASS_ARTIST`) is born holding one (`Unit::card`), drawn from the cards still in the deck.
//! Once its owner knows the card's own prerequisite techs (the card's "needs"), the great person
//! can discover it (`SPELL_DISCOVER_CARD`): its team learns the card, the content that needs the
//! card (units, buildings, promotions, spells, civics, ...) opens, and the great person is used up.
//!
//! There is one deck for the world, and it keeps no state of its own: a card is in the deck while
//! no team knows it and no living unit holds it. A card whose great person dies, or is used
//! another way, is back in the deck.

pub mod doom;
pub mod doom_cards;

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// The band a card belongs to, or a player is in: its era, Medieval and later as one.
fn band(e: EraType) -> usize {
    e.idx().min(2)
}

impl Game {
    /// Is `class` a suit of the deck (some card is drawn by its great people)?
    pub fn deck_is_suit(&self, class: UnitClassType) -> bool {
        self.d.techs.iter().any(|t| t.card.as_ref().is_some_and(|c| c.suit == class))
    }

    /// Is `card` in the deck: no team knows it and no living unit holds it?
    pub fn deck_has_card(&self, card: TechType) -> bool {
        self.d.techs[card].card.is_some()
            && !self.teams.iter().any(|t| t.has_tech[card])
            && !self.units.iter().any(|(_, u)| u.card == Some(card))
    }

    /// The cards a great person of class `suit` born to `p` may be dealt now, in id order:
    /// the cards of the suit still in the deck whose alignment (if any) is the player's, from
    /// the band of the player's era (I Ancient, II Classical, III Medieval and later); if that
    /// band is empty, from the nearest band below it, else the nearest above.
    pub fn deck_candidates(&self, p: PlayerId, suit: UnitClassType) -> Vec<TechType> {
        let d = self.d;
        let alignment = self.players[p].alignment;
        let open: Vec<(TechType, usize)> = d
            .techs
            .keys()
            .filter_map(|t| {
                let c = d.techs[t].card.as_ref()?;
                if c.suit != suit || c.alignment.is_some_and(|a| Some(a) != alignment) {
                    return None;
                }
                Some((t, d.techs[t].era.map_or(0, band)))
            })
            .filter(|&(t, _)| self.deck_has_card(t))
            .collect();
        let mine = band(self.players[p].current_era);
        let order = (0..=mine).rev().chain(mine + 1..3);
        for b in order {
            let cards: Vec<TechType> = open.iter().filter(|&&(_, cb)| cb == b).map(|&(t, _)| t).collect();
            if !cards.is_empty() {
                return cards;
            }
        }
        Vec::new()
    }

    /// Deals a card to a great person of class `suit` born to `p` (`None`: not a suit, or the
    /// suit's cards are all out).
    pub fn deck_draw(&mut self, p: PlayerId, suit: UnitClassType) -> Option<TechType> {
        if !self.deck_is_suit(suit) {
            return None;
        }
        let cards = self.deck_candidates(p, suit);
        if cards.is_empty() {
            return None;
        }
        let k = self.soren_rand_num(cards.len() as i32, "Great person card") as usize;
        Some(cards[k])
    }

    /// Gives a new unit its card if it is a great person of a suit (from `player_init_unit`).
    pub fn deck_deal_to_unit(&mut self, u: UnitId) {
        let p = self.units[u].owner;
        if self.player_is_barbarian(p) {
            return;
        }
        let class = self.unit_get_unit_class_type(u);
        if let Some(card) = self.deck_draw(p, class) {
            self.units[u].card = Some(card);
        }
    }

    /// Does `team` know every prerequisite of `card` (its "needs")?
    pub fn team_meets_card_needs(&self, team: TeamId, card: TechType) -> bool {
        let info = &self.d.techs[card];
        info.prereq_and_techs.iter().all(|&t| self.team_is_has_tech(team, Some(t)))
            && (info.prereq_or_techs.is_empty() || info.prereq_or_techs.iter().any(|&t| self.team_is_has_tech(team, Some(t))))
    }

    /// Can `u` discover the card it holds now (`SPELL_DISCOVER_CARD`'s requirement)?
    pub fn unit_can_discover_card(&self, u: UnitId) -> bool {
        let Some(card) = self.units[u].card else {
            return false;
        };
        let team = self.unit_get_team(u);
        !self.teams[team].has_tech[card] && self.team_meets_card_needs(team, card)
    }

    /// Discovers the card `u` holds (`SPELL_DISCOVER_CARD`'s result; the spell uses the great
    /// person up): its team learns it, and a card with a faith spreads it to three of the
    /// owner's cities without it (the most populous first).
    pub fn unit_discover_card(&mut self, u: UnitId) {
        let Some(card) = self.units[u].card.take() else {
            return;
        };
        let p = self.units[u].owner;
        let team = self.players[p].team;
        self.team_set_has_tech(team, card, true, Some(p), true, true);
        if let Some(religion) = self.d.techs[card].card.as_ref().and_then(|c| c.religion) {
            self.player_spread_card_religion(p, religion, 3);
        }
        let name = self.d.text.get(&self.d.techs[card].base.description).to_string();
        let turn = self.game_turn();
        self.log.msg(turn, Some(p), || format!("Card discovered: {name}."));
    }

    /// Spreads `religion` to `n` of `p`'s cities that lack it, the most populous first (ties: the
    /// lower city id).
    pub fn player_spread_card_religion(&mut self, p: PlayerId, religion: ReligionType, n: usize) {
        let mut cities: Vec<CityId> =
            self.players[p].cities.iter().copied().filter(|&c| !self.cities[c].has_religion[religion]).collect();
        cities.sort_by_key(|&c| (std::cmp::Reverse(self.cities[c].population), c));
        for c in cities.into_iter().take(n) {
            self.city_set_has_religion(c, religion, true, true, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::ids::*;
    use crate::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 5));
        g.game_init();
        g
    }

    #[test]
    fn every_card_has_a_suit_needs_and_no_research() {
        let d = ffh_data::global();
        for (t, info) in d.techs.iter_enumerated() {
            let Some(card) = &info.card else { continue };
            assert!(info.disable && !info.trade, "{}: a card is neither researched nor traded", info.base.type_);
            assert!(!info.prereq_and_techs.is_empty() || !info.prereq_or_techs.is_empty(), "{}: a card needs a tech", info.base.type_);
            assert!(info.era.is_some(), "{}: a card has a band (its era)", info.base.type_);
            let _ = (t, card);
        }
    }

    #[test]
    fn a_great_person_is_dealt_a_card_of_its_suit_and_it_returns_to_the_deck() {
        let d = ffh_data::global();
        let Some((card, info)) = d.techs.iter_enumerated().find(|(_, i)| i.card.is_some()) else {
            return;
        };
        let suit = info.card.as_ref().unwrap().suit;
        let Some(unit) = d.unit_classes[suit].default_unit_index else { return };
        let mut g = game();
        let p = PlayerId(0);
        let plot = g.map.plot_ids().find(|&t| !g.plot_is_water(t) && !g.plot_is_impassable(t)).expect("land");
        let (x, y) = g.map.xy(plot);
        assert!(g.deck_has_card(card));
        let mut held = None;
        for _ in 0..64 {
            let u = g.player_init_unit(p, unit, x, y, None, None);
            let c = g.units[u].card.expect("a great person of a suit is dealt a card");
            assert_eq!(d.techs[c].card.as_ref().unwrap().suit, suit);
            assert!(!g.deck_has_card(c), "a held card is out of the deck");
            if c == card {
                held = Some(u);
                break;
            }
            g.unit_kill(u, false, None);
            assert!(g.deck_has_card(c), "a dead great person's card is back in the deck");
        }
        if let Some(u) = held {
            let team = g.players[p].team;
            for &t in d.techs[card].prereq_and_techs.iter().chain(d.techs[card].prereq_or_techs.iter()) {
                g.team_set_has_tech(team, t, true, Some(p), false, false);
            }
            assert!(g.unit_can_discover_card(u));
            g.unit_discover_card(u);
            assert!(g.teams[team].has_tech[card]);
            assert!(!g.deck_has_card(card), "a discovered card has left the deck");
        }
    }
}
