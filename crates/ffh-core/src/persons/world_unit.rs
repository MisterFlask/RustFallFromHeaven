//! The world unit (docs/PERSONS.md, "The third build", rule 9): the mean total commerce a turn,
//! every commerce type, of the living civilizations that are not Houses, barbarians, minor
//! civilizations or crusade Hosts. It scales what passes in gold between a realm and the world
//! (the courtship's requirement, a House's price, an ultimatum's gold: part 2). Total commerce,
//! not gold: the gold slider is a choice, and a civilization at full research shows little gold
//! however rich.

use ffh_data::*;

use super::*;

impl Game {
    /// Player `p`'s total commerce a turn: the sum of its commerce rates of every type.
    pub fn player_total_commerce_rate(&self, p: PlayerId) -> i32 {
        (0..CommerceType::NUM).map(|i| self.player_get_commerce_rate(p, CommerceType::from_usize(i))).sum()
    }

    /// The world unit (pure, integer): the mean of [`Game::player_total_commerce_rate`] over the
    /// civilizations that can have Houses ([`Game::player_has_houses`]: living, no House, no
    /// barbarians, minor civilization or crusade Host), rounded down; at least 1.
    pub fn world_commerce_unit(&self) -> i32 {
        let (mut sum, mut n) = (0i64, 0i64);
        for p in self.player_ids() {
            if self.player_has_houses(p) {
                sum += self.player_total_commerce_rate(p) as i64;
                n += 1;
            }
        }
        if n == 0 {
            return 1;
        }
        ((sum / n) as i32).max(1)
    }
}
