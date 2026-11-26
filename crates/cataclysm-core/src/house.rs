//! Regional Houses: internal politics replacing Civ4's Happiness, Health, and Maintenance.
//!
//! Every 1..=4 cities of the same origin group into a House led by a named minor-leader portrait
//! with an alignment, religion, sympathy for a rival culture, and a rolled House Perk.
//! Loyalty directly scales city output and shuts off the House Perk when Furious (<= -3).

use crate::data::{Alignment, HousePerk, Religion};
use crate::ids::{CityId, HouseId, PlayerId};

pub const HOUSE_MAX_CITIES: usize = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HouseMood {
    Loyal,
    Content,
    Restless,
    Furious,
}

impl HouseMood {
    pub const fn name(self) -> &'static str {
        match self {
            HouseMood::Loyal => "Loyal",
            HouseMood::Content => "Content",
            HouseMood::Restless => "Restless",
            HouseMood::Furious => "Furious",
        }
    }

    /// Output multiplier in percent for Gold/Industry/Lore from cities in this House.
    pub const fn output_pct(self) -> i32 {
        match self {
            HouseMood::Loyal => 115,
            HouseMood::Content => 100,
            HouseMood::Restless => 85,
            HouseMood::Furious => 60,
        }
    }
}

#[derive(Clone, Debug)]
pub struct House {
    pub id: HouseId,
    pub owner: PlayerId,
    pub origin: PlayerId,
    pub name: String,
    pub lord_name: String,
    pub lord_portrait: &'static str,
    pub alignment: Alignment,
    pub religion: Option<Religion>,
    /// Rival player whose culture had strong influence here (going to war with them hurts loyalty).
    pub sympathy: Option<PlayerId>,
    pub perk: HousePerk,
    pub cities: Vec<CityId>,
    /// Persistent loyalty modifiers (from Great People, Armageddon crises, etc.).
    pub bonus_loyalty: i32,
}

impl House {
    pub fn compute_loyalty(
        &self,
        ruler_alignment: Alignment,
        ruler_religion: Option<Religion>,
        at_war_with_sympathy: bool,
        temples_in_house: i32,
        empire_loyalty_bonus: i32,
        total_houses_owned: usize,
    ) -> (i32, Vec<String>) {
        let mut score = 1 + empire_loyalty_bonus + self.bonus_loyalty;
        let mut parts = Vec::new();
        parts.push("Base: +1".into());
        if empire_loyalty_bonus != 0 {
            parts.push(format!("Empire bonuses: {:+}", empire_loyalty_bonus));
        }
        if self.bonus_loyalty != 0 {
            parts.push(format!("Decrees & Omens: {:+}", self.bonus_loyalty));
        }
        if self.alignment == ruler_alignment {
            score += 1;
            parts.push("Shared alignment: +1".into());
        } else if (self.alignment == Alignment::Good && ruler_alignment == Alignment::Evil)
            || (self.alignment == Alignment::Evil && ruler_alignment == Alignment::Good)
        {
            score -= 2;
            parts.push("Opposed alignment: -2".into());
        }
        if let (Some(hr), Some(rr)) = (self.religion, ruler_religion) {
            if hr == rr {
                score += 1;
                parts.push(format!("Shared faith ({}): +1", hr.name()));
            } else {
                score -= 1;
                parts.push(format!("Clashing faith ({}): -1", hr.name()));
            }
        }
        if at_war_with_sympathy {
            score -= 3;
            parts.push("War against sympathetic rival: -3".into());
        }
        if temples_in_house > 0 {
            score += temples_in_house.min(2);
            parts.push(format!("Temples: +{}", temples_in_house.min(2)));
        }
        if total_houses_owned > 2 {
            let stretch = (total_houses_owned as i32 - 2).min(3);
            score -= stretch;
            parts.push(format!("Overextension ({} Houses): -{}", total_houses_owned, stretch));
        }
        (score, parts)
    }

    pub fn mood_of(loyalty: i32) -> HouseMood {
        if loyalty >= 3 {
            HouseMood::Loyal
        } else if loyalty >= 0 {
            HouseMood::Content
        } else if loyalty >= -2 {
            HouseMood::Restless
        } else {
            HouseMood::Furious
        }
    }

    pub fn perk_active(loyalty: i32) -> bool {
        Self::mood_of(loyalty) != HouseMood::Furious
    }
}
