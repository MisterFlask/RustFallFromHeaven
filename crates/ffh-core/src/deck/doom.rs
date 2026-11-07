//! The Armageddon deck (docs/DECKS.md, "The Armageddon deck as built"): FFH2's fixed Armageddon
//! events become cards, dealt face down to the counter's slots at the start of the game.
//!
//! - **Slots:** 10, 20, ... 90, and 100 for the ending: one card each, or none.
//! - **The deal:** on the first turn, slot by slot from the lowest, one card from those whose
//!   first threshold is the slot. A chain's later stages take the later slots they name; a chain
//!   is dealt only where those slots are free.
//! - **Reveal:** a card is shown to every player once the counter (`get_global_counter`) is within
//!   5 of its slot. A card whose condition fails then (`DoomCard::valid`) is dealt again from its
//!   slot's cards, or the slot stays empty.
//! - **Firing:** once, when the counter first reaches the slot.
//!
//! The cards are code ([`CARDS`]): a name (text `TXT_KEY_DOOM_<KEY>`, its effect
//! `TXT_KEY_DOOM_<KEY>_HELP`), its thresholds, and what each stage does. A slot holds a card by
//! its key, so a save does not depend on the table's order.

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// A card of the Armageddon deck.
pub struct DoomCard {
    /// `DOOM_<NAME>`: its text is `TXT_KEY_<key>` (the name) and `TXT_KEY_<key>_HELP` (what it
    /// does, one line per stage).
    pub key: &'static str,
    /// The thresholds of its stages: the first is the slot it is dealt to, the others the slots
    /// its later stages take (a chain).
    pub stages: &'static [i32],
    /// Fires stage `i`.
    pub fire: fn(&mut Game, usize),
    /// Whether the card can be played (checked at the reveal; `None`: always).
    pub valid: Option<fn(&Game) -> bool>,
}

/// The slots, in counter order.
pub const SLOTS: [i32; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

/// How far below its slot a card is revealed.
pub const REVEAL_AHEAD: i32 = 5;

/// A slot of the dealt deck.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoomSlot {
    /// Its threshold (a percentage of the counter's limit, as `get_global_counter`).
    pub threshold: i32,
    /// The card (its key) and the stage of it this slot plays; `None`: an empty slot.
    pub card: Option<(String, u8)>,
    /// Shown to every player.
    pub revealed: bool,
    /// Played.
    pub fired: bool,
}

/// The Armageddon deck as dealt (`Game::doom`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doom {
    /// The slots have been dealt (on the first turn).
    pub dealt: bool,
    pub slots: Vec<DoomSlot>,
}

/// FFH2's Armageddon event `name`, for every civilization (as FFH2's triggers fired: not the
/// barbarians, minor civs or crusade Hosts).
fn ffh2_event_all(g: &mut Game, name: &str) {
    for p in g.player_ids() {
        if g.players[p].alive && !g.player_is_barbarian(p) && !g.player_is_minor_civ(p) && !g.player_is_crusade_host(p) {
            crate::ffh::random_events_a::armageddon_effect(g, name, p);
        }
    }
}

fn fire_blight(g: &mut Game, _: usize) {
    ffh2_event_all(g, "doArmageddonBlight");
}

fn fire_horsemen(g: &mut Game, stage: usize) {
    let name = ["doArmageddonStephanos", "doArmageddonBuboes", "doArmageddonYersinia", "doArmageddonArs"][stage.min(3)];
    crate::ffh::random_events_a::armageddon_effect(g, name, PlayerId(0));
}

fn fire_hellfire(g: &mut Game, _: usize) {
    // FFH2's Hellfire runs once, for player 0.
    crate::ffh::random_events_a::armageddon_effect(g, "doArmageddonHellfire", PlayerId(0));
}

fn fire_wrath(g: &mut Game, _: usize) {
    ffh2_event_all(g, "doArmageddonWrath");
}

fn fire_apocalypse(g: &mut Game, _: usize) {
    ffh2_event_all(g, "doArmageddonApocalypse");
}

/// Every card of the deck. FFH2's events first; the cards of docs/DECKS_ROUND8.md section B
/// follow (`deck/doom_cards.rs`).
pub const CARDS: &[DoomCard] = &[
    DoomCard { key: "DOOM_BLIGHT", stages: &[30], fire: fire_blight, valid: None },
    DoomCard { key: "DOOM_FOUR_HORSEMEN", stages: &[40, 50, 60, 70], fire: fire_horsemen, valid: None },
    DoomCard { key: "DOOM_HELLFIRE", stages: &[80], fire: fire_hellfire, valid: None },
    DoomCard { key: "DOOM_WRATH", stages: &[90], fire: fire_wrath, valid: None },
    DoomCard { key: "DOOM_APOCALYPSE", stages: &[100], fire: fire_apocalypse, valid: None },
];

/// The cards of the deck: FFH2's and the new ones.
pub fn all_cards() -> impl Iterator<Item = &'static DoomCard> {
    CARDS.iter().chain(crate::deck::doom_cards::CARDS.iter())
}

/// The card with this key, among all.
pub fn find(key: &str) -> Option<&'static DoomCard> {
    all_cards().find(|c| c.key == key)
}

impl Game {
    /// The keys of the cards dealt to some slot.
    fn doom_dealt_keys(&self) -> Vec<&str> {
        self.doom.slots.iter().filter_map(|s| s.card.as_ref().map(|(k, _)| k.as_str())).collect()
    }

    /// The cards that may be dealt to the slot at `threshold` now: first threshold `threshold`,
    /// not dealt elsewhere, every later stage's slot free (empty and unrevealed), valid now if
    /// `check_valid`.
    fn doom_slot_candidates(&self, threshold: i32, check_valid: bool) -> Vec<&'static DoomCard> {
        let dealt = self.doom_dealt_keys();
        all_cards()
            .filter(|c| c.stages.first() == Some(&threshold) && !dealt.contains(&c.key))
            .filter(|c| {
                c.stages[1..].iter().all(|&t| self.doom.slots.iter().any(|s| s.threshold == t && s.card.is_none() && !s.fired))
            })
            .filter(|c| !check_valid || c.valid.map_or(true, |f| f(self)))
            .collect()
    }

    /// Deals `c` to its first slot and its later stages to theirs.
    fn doom_place(&mut self, c: &'static DoomCard) {
        for (i, &t) in c.stages.iter().enumerate() {
            if let Some(s) = self.doom.slots.iter_mut().find(|s| s.threshold == t) {
                s.card = Some((c.key.to_string(), i as u8));
            }
        }
    }

    /// Deals the deck (once, on the first turn).
    pub fn doom_deal(&mut self) {
        self.doom.slots = SLOTS.iter().map(|&threshold| DoomSlot { threshold, ..Default::default() }).collect();
        self.doom.dealt = true;
        for &t in SLOTS.iter() {
            if self.doom.slots.iter().any(|s| s.threshold == t && s.card.is_some()) {
                continue;
            }
            let cands = self.doom_slot_candidates(t, false);
            if cands.is_empty() {
                continue;
            }
            let k = self.soren_rand_num(cands.len() as i32, "Armageddon deal") as usize;
            self.doom_place(cands[k]);
        }
    }

    /// The card of a slot and its stage.
    pub fn doom_slot_card(&self, i: usize) -> Option<(&'static DoomCard, usize)> {
        let (key, stage) = self.doom.slots.get(i)?.card.as_ref()?;
        Some((find(key)?, *stage as usize))
    }

    /// Reveals and fires the slots the counter has reached (each game turn).
    pub fn doom_do_turn(&mut self) {
        if self.is_option(GameOptionType::GAMEOPTION_NO_GLOBAL_COUNTER) {
            return;
        }
        if !self.doom.dealt {
            self.doom_deal();
        }
        let counter = self.get_global_counter();
        for i in 0..self.doom.slots.len() {
            let threshold = self.doom.slots[i].threshold;
            if !self.doom.slots[i].revealed && counter >= threshold - REVEAL_AHEAD {
                self.doom_reveal(i);
            }
            if self.doom.slots[i].revealed && !self.doom.slots[i].fired && counter >= threshold {
                self.doom.slots[i].fired = true;
                if let Some((c, stage)) = self.doom_slot_card(i) {
                    let turn = self.game_turn();
                    let name = self.d.text.get(&format!("TXT_KEY_{}", c.key)).to_string();
                    self.log.msg(turn, None, || format!("Armageddon {threshold}: {name}."));
                    (c.fire)(self, stage);
                }
            }
        }
    }

    /// Reveals slot `i`: a first stage whose card cannot be played is dealt again from the
    /// slot's cards.
    fn doom_reveal(&mut self, i: usize) {
        self.doom.slots[i].revealed = true;
        let threshold = self.doom.slots[i].threshold;
        if let Some((c, 0)) = self.doom_slot_card(i) {
            if !c.valid.map_or(true, |f| f(self)) {
                // Take the card (all its stages) out, and deal another.
                for s in self.doom.slots.iter_mut() {
                    if s.card.as_ref().is_some_and(|(k, _)| k == c.key) {
                        s.card = None;
                    }
                }
                let cands: Vec<&'static DoomCard> =
                    self.doom_slot_candidates(threshold, true).into_iter().filter(|x| x.key != c.key).collect();
                if !cands.is_empty() {
                    let k = self.soren_rand_num(cands.len() as i32, "Armageddon redeal") as usize;
                    self.doom_place(cands[k]);
                }
            }
        }
        if let Some((c, _)) = self.doom_slot_card(i) {
            let turn = self.game_turn();
            let name = self.d.text.get(&format!("TXT_KEY_{}", c.key)).to_string();
            self.log.msg(turn, None, || format!("Armageddon card revealed at {threshold}: {name}."));
        }
    }

    /// Is slot `i` known to `p`: revealed to everyone, or among the next `extra` unrevealed
    /// slots for a player who sees further (the Orrery of Ceridwen: 2).
    pub fn doom_slot_known(&self, p: PlayerId, i: usize) -> bool {
        let Some(slot) = self.doom.slots.get(i) else { return false };
        if slot.revealed {
            return true;
        }
        let extra = self.player_doom_sight(p);
        if extra == 0 {
            return false;
        }
        let ahead: Vec<usize> = (0..self.doom.slots.len()).filter(|&j| !self.doom.slots[j].revealed).take(extra).collect();
        ahead.contains(&i)
    }

    /// How many unrevealed slots ahead `p` sees (0; cards that show more add to it).
    pub fn player_doom_sight(&self, p: PlayerId) -> usize {
        crate::deck::doom_cards::doom_sight(self, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    #[test]
    fn every_card_has_its_text_and_ascending_slots() {
        let d = ffh_data::global();
        for c in all_cards() {
            assert!(SLOTS.contains(&c.stages[0]), "{}: its first threshold is a slot", c.key);
            assert!(c.stages.windows(2).all(|w| w[0] < w[1]), "{}: its stages ascend", c.key);
            assert!(c.stages.iter().all(|t| SLOTS.contains(t)), "{}: every stage is a slot", c.key);
            let name = format!("TXT_KEY_{}", c.key);
            assert_ne!(d.text.get(&name), name, "{}: has a name", c.key);
            let help = format!("TXT_KEY_{}_HELP", c.key);
            assert_ne!(d.text.get(&help), help, "{}: has a help line", c.key);
        }
        let mut keys: Vec<&str> = all_cards().map(|c| c.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), all_cards().count(), "card keys are unique");
    }

    #[test]
    fn the_deal_fills_slots_with_whole_chains_and_reveals_before_firing() {
        let mut g = game();
        g.doom_deal();
        assert_eq!(g.doom.slots.len(), SLOTS.len());
        // Each dealt card's stages sit in their own slots, in order.
        for s in &g.doom.slots {
            if let Some((key, stage)) = &s.card {
                let c = find(key).expect("a dealt card exists");
                assert_eq!(c.stages[*stage as usize], s.threshold);
                for (i, &t) in c.stages.iter().enumerate() {
                    let other = g.doom.slots.iter().find(|x| x.threshold == t).unwrap();
                    assert_eq!(other.card.as_ref(), Some(&(key.clone(), i as u8)));
                }
            }
        }
        // Within 5 of a slot: revealed, not fired; at it: fired.
        g.global_counter_limit = 100;
        g.global_counter = 26;
        g.doom_do_turn();
        let s30 = g.doom.slots.iter().find(|s| s.threshold == 30).unwrap();
        assert!(s30.revealed && !s30.fired);
        assert!(g.doom.slots.iter().filter(|s| s.threshold < 30).all(|s| s.fired));
    }
}
