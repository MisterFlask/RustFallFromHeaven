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

/// Renown for the first Great Person, and how much more each later one needs.
pub const RENOWN_FIRST: i32 = 45;
pub const RENOWN_STEP: i32 = 30;

/// Tempo Burst amounts by suit.
pub const BURST_SAGE_LORE: i32 = 90;
pub const BURST_SAGE_MANA: i32 = 20;
pub const BURST_PROPHET_STRAIN: i32 = -6;
pub const BURST_PROPHET_LOYALTY: i32 = 2;
pub const BURST_ENGINEER_INDUSTRY: i32 = 140;
pub const BURST_MERCHANT_GOLD: i32 = 220;
pub const BURST_MERCHANT_CARAVAN_SLOTS: i32 = 1;
pub const BURST_BARD_CULTURE: i32 = 120;
pub const BURST_COMMANDER_HEAL_PCT: i32 = 50;
pub const BURST_COMMANDER_XP: i32 = 15;
/// A Lord appointment adds this much lasting Loyalty to the House.
pub const LORD_LOYALTY: i32 = 2;
/// Discovering a Secret Tech whose effect is a one-time Strain surge also adds this Mana.
pub const SECRET_SURGE_MANA: i32 = 40;

/// Computes the Renown threshold for the next Great Person (`n` already born).
pub fn renown_threshold(born_so_far: i32) -> i32 {
    RENOWN_FIRST + born_so_far * RENOWN_STEP
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
