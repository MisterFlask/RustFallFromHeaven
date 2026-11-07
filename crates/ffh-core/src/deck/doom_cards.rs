//! The Armageddon cards of docs/DECKS_ROUND8.md section B that survived review (C.5, C.7), and
//! the rules other cards add to the deck (the Orrery of Ceridwen's sight).

use crate::deck::doom::DoomCard;
use crate::game::Game;
use crate::ids::*;

/// The new cards (FFH2's are in `doom::CARDS`).
pub const CARDS: &[DoomCard] = &[];

/// How many unrevealed Armageddon slots ahead `p` sees.
pub fn doom_sight(_g: &Game, _p: PlayerId) -> usize {
    0
}
