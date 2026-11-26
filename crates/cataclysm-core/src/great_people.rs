//! Empire-wide Great People meters, Court arrival, shared world Secret-Tech Deck, and 3-way choices.

use crate::data::{GreatSuit, SecretTechId, SECRET_TECHS};
use crate::ids::{CityId, GreatPersonId, HouseId, PlayerId, SquadId};
use crate::rng::Rng;

#[derive(Clone, Debug)]
pub struct GreatPerson {
    pub id: GreatPersonId,
    pub owner: PlayerId,
    pub suit: GreatSuit,
    pub name: String,
    pub portrait: &'static str,
    /// The visible Secret Tech card drawn from the shared world deck when this person was born.
    /// If spent on a Tempo Burst or Commissioned, this card returns to the world deck!
    pub secret_tech: Option<SecretTechId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GreatPersonAction {
    DiscoverSecret,
    TempoBurst,
    CommissionHero(SquadId),
    CommissionLord(HouseId),
}

/// Shared world deck of undiscovered Secret Tech cards (one deck for the whole world).
#[derive(Clone, Debug)]
pub struct SecretDeck {
    pub available: Vec<SecretTechId>,
}

impl SecretDeck {
    pub fn new(rng: &mut Rng) -> Self {
        let mut available: Vec<SecretTechId> = SECRET_TECHS.iter().map(|s| s.id).collect();
        rng.shuffle(&mut available);
        Self { available }
    }

    pub fn draw_for_suit(&mut self, suit: GreatSuit) -> Option<SecretTechId> {
        if let Some(pos) = self.available.iter().position(|&id| SECRET_TECHS[id.0 as usize].suit == suit) {
            Some(self.available.remove(pos))
        } else {
            None
        }
    }

    pub fn return_card(&mut self, id: SecretTechId) {
        if !self.available.contains(&id) {
            self.available.push(id);
        }
    }
}

/// Computes the Renown threshold for the next Great Person (`n` already born).
pub fn renown_threshold(born_so_far: i32) -> i32 {
    45 + born_so_far * 30
}

/// Picks the highest-meter suit once Renown threshold is reached.
pub fn dominant_suit(meters: &[i32; 6]) -> GreatSuit {
    let mut best_i = 0;
    for i in 1..6 {
        if meters[i] > meters[best_i] {
            best_i = i;
        }
    }
    GreatSuit::ALL[best_i]
}

/// City target used for Engineer/Bard tempo bursts.
pub fn burst_city(cities: &[CityId]) -> Option<CityId> {
    cities.first().copied()
}
