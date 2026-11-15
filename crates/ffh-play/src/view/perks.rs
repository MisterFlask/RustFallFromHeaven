//! City-perk presentation shared by saved offers, the city screen and the cities list.
use ffh_core::{game::Game, ids::CityId, city::specialties::*};
use ffh_data::*;
use crate::json::Json;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Perk {
    pub title: String,
    /// The existing interface icon name (the mana entry uses its rolled sphere).
    pub icon: String,
    pub text: String,
}

impl Perk {
    pub fn to_json(&self) -> Json { crate::jobj! {
        "title" => self.title.as_str(), "icon" => self.icon.as_str(), "text" => self.text.as_str(),
    } }
}

pub fn describe(g: &Game, s: u8, class: Option<UnitCombatType>, mana: Option<BonusType>) -> Perk {
    let (title, icon) = match s {
        CLASS_XP => {
            let name = class.map(|k| {
                let key = &g.d.unit_combats[k].base.description;
                let text = g.d.text.name(key);
                if text.starts_with("TXT_KEY_") { g.d.unit_combats[k].base.type_.trim_start_matches("UNITCOMBAT_") } else { text }
            }).unwrap_or("Military");
            (format!("{name} Academy"), "xp".into())
        },
        MANA => {
            let name = mana.map(|b| g.d.text.name(&g.d.bonuses[b].base.description)).unwrap_or("Mana");
            let icon = mana.map(|b| format!("sphere_{}", g.d.bonuses[b].base.type_.trim_start_matches("BONUS_MANA_").to_lowercase())).unwrap_or("mana".into());
            (format!("{name} Source"), icon)
        },
        _ => {
            let (title, icon) = match s {
                WONDERS => ("Master Builders", "construct"),
                SETTLERS => ("Pioneers", "found"),
                GREAT_PEOPLE => ("Great Minds", "greatpeople"),
                SAGES => ("Scholars' Guild", "research"),
                CULTURE => ("City of Arts", "culture"),
                RELIGIONS => ("Sacred Crossroads", "religion"),
                NAVAL => ("Fleet Academy", "seapatrol"),
                HEALING => ("Sanctuary", "heal"),
                MOVEMENT => ("Swift Roads", "road"),
                FOOD => ("Abundance", "food"),
                POP_RUSH => ("Willing Hands", "hurry"),
                DRAFT => ("Citizen Militia", "strength"),
                GOLD_RUSH => ("Shrewd Buyers", "gold"),
                DOUBLE_GROWTH => ("Growing Together", "population"),
                _ => ("City perk", "help"),
            };
            (title.into(), icon.into())
        },
    };
    Perk { title, icon, text: g.specialty_text(s, class, mana) }
}

pub fn city(g: &Game, c: CityId) -> Option<Perk> {
    let city = &g.cities[c];
    city.specialty.map(|s| describe(g, s, city.specialty_class, city.specialty_mana))
}
