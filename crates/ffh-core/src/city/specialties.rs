//! Permanent city specialties and saved, deterministic pick-one-of-three offers.
use crate::{game::Game, ids::*};
use ffh_data::*;

pub const CLASS_XP: u8 = 0;
pub const WONDERS: u8 = 3;
pub const SETTLERS: u8 = 6;
pub const GREAT_PEOPLE: u8 = 7;
pub const SAGES: u8 = 8;
pub const CULTURE: u8 = 9;
pub const RELIGIONS: u8 = 11;
pub const NAVAL: u8 = 13;
pub const HEALING: u8 = 15;
pub const MOVEMENT: u8 = 16;
pub const FOOD: u8 = 17;
pub const POP_RUSH: u8 = 18;
pub const DRAFT: u8 = 19;
pub const GOLD_RUSH: u8 = 20;
// Retired ids are not reused: 1, 2, 4, 5, 10, 12 and 14.
pub const MANA: u8 = 21;
pub const DOUBLE_GROWTH: u8 = 22;
pub const SPECIALTIES: [u8; 16] = [CLASS_XP, WONDERS, SETTLERS, GREAT_PEOPLE, SAGES,
    CULTURE, RELIGIONS, NAVAL, HEALING, MOVEMENT, FOOD, POP_RUSH, DRAFT, GOLD_RUSH,
    MANA, DOUBLE_GROWTH];

impl Game {
    pub fn city_has_specialty(&self, c: CityId, s: u8) -> bool { self.cities[c].specialty == Some(s) }

    pub fn specialty_text(&self, s: u8, class: Option<UnitCombatType>, mana: Option<BonusType>) -> String {
        if s == CLASS_XP {
            let name = class.map(|k| {
                let key = &self.d.unit_combats[k].base.description;
                let text = self.d.text.name(key);
                if text.starts_with("TXT_KEY_") { self.d.unit_combats[k].base.type_.trim_start_matches("UNITCOMBAT_") } else { text }
            }).unwrap_or("Class");
            return format!("{name} units trained here get +3 XP.");
        }
        if s == MANA {
            let name = mana.map(|b| self.d.text.name(&self.d.bonuses[b].base.description)).unwrap_or("mana");
            return format!("This city provides 1 {name}.");
        }
        match s {
            DOUBLE_GROWTH => "When this city grows, it gains 2 population, up to its happiness limit.",
            WONDERS => "Wonders are built here at 1.5x speed.",
            SETTLERS => "Settlers trained here do not stop growth, and the cities they found start at size 2.",
            GREAT_PEOPLE => "Great Person points are doubled here.",
            SAGES => "Each specialist in this city gives +1 research.",
            CULTURE => "1.5x culture in this city.",
            RELIGIONS => "Each religion in this city gives +2 culture and +1 gold.",
            NAVAL => "Naval units trained here get +1 movement.",
            HEALING => "Friendly units in this city heal fully each turn while supplied.",
            MOVEMENT => "Friendly units move at double speed in this city's cultural radius.",
            FOOD => "This city gains +5 food each turn.",
            POP_RUSH => "This city can always rush production with population, and rushing here causes no unhappiness.",
            DRAFT => "This city can draft one unit every 10 turns with no unhappiness, in addition to civic drafts.",
            GOLD_RUSH => "Rushing production with gold costs half here and requires no civic.",
            _ => "",
        }.into()
    }

    pub fn city_specialty_classes(&self, c: CityId) -> Vec<UnitCombatType> {
        let mut classes = Vec::new();
        for u in self.d.units.keys() {
            if let Some(k) = self.d.units[u].unit_combat_type {
                if !classes.contains(&k) && self.city_can_train(c, u, false, false, false, false) { classes.push(k); }
            }
        }
        classes.sort();
        classes
    }

    pub fn city_specialty_eligible(&self, c: CityId, s: u8) -> bool {
        match s {
            CLASS_XP => !self.city_specialty_classes(c).is_empty(),
            NAVAL => self.city_is_coastal(c, 0),
            MANA => !self.city_specialty_manas().is_empty(),
            _ => SPECIALTIES.contains(&s),
        }
    }

    /// Only chosen specialties reserve an entry. Exhaustion permits repeats so every city gets
    /// three choices; choices themselves are always distinct. Unchosen offers reserve nothing.
    pub fn city_offer_specialty(&mut self, c: CityId) {
        if self.cities[c].outpost || self.cities[c].specialty.is_some() { return; }
        if self.cities[c].culture_level.is_none_or(|l| l.idx() < 3) { return; }
        let owner = self.cities[c].owner;
        let used: Vec<u8> = self.players[owner].cities.iter().filter(|&&o| o != c).filter_map(|&o| self.cities[o].specialty).collect();
        let eligible: Vec<u8> = SPECIALTIES.into_iter().filter(|&s| self.city_specialty_eligible(c, s)).collect();
        let mut pool: Vec<u8> = eligible.iter().copied().filter(|s| !used.contains(s)).collect();
        let unused = pool.len();
        let mut repeats: Vec<_> = eligible.into_iter().filter(|s| used.contains(s)).collect();
        // A previously offered option may have been chosen in another city. Keep all three
        // offers stable until that happens, including across saves and repeated queries.
        if self.cities[c].specialty_options.len() == 3 && self.cities[c].specialty_options.iter().all(|(s, _, _)| SPECIALTIES.contains(s) && (!used.contains(s) || unused < 3)) {
            if !self.players[owner].human { self.city_choose_specialty(c, 0); }
            return;
        }
        let mut options = Vec::new();
        while options.len() < 3 && (!pool.is_empty() || !repeats.is_empty()) {
            if pool.is_empty() { pool.append(&mut repeats); }
            let i = self.soren_rand_num(pool.len() as i32, "City specialty") as usize;
            let s = pool.remove(i);
            let class = if s == CLASS_XP {
                let classes = self.city_specialty_classes(c);
                Some(classes[self.soren_rand_num(classes.len() as i32, "Specialty class") as usize])
            } else { None };
            let mana = if s == MANA {
                let manas = self.city_specialty_manas();
                Some(manas[self.soren_rand_num(manas.len() as i32, "Specialty mana") as usize])
            } else { None };
            options.push((s, class, mana));
        }
        self.cities[c].specialty_options = options;
        if !self.players[owner].human {
            // A modest local choice: favor food, specialists and existing military production.
            let pick = self.cities[c].specialty_options.iter().enumerate().max_by_key(|(_, (s, _, _))| match *s { FOOD => 100, GREAT_PEOPLE => 70 + self.cities[c].population, CLASS_XP => 60, _ => 40 }).map(|(i, _)| i).unwrap_or(0);
            self.city_choose_specialty(c, pick);
        }
    }

    pub fn city_choose_specialty(&mut self, c: CityId, option: usize) -> bool {
        if self.cities[c].specialty.is_some() { return false; }
        let Some(&(s, class, mana)) = self.cities[c].specialty_options.get(option) else { return false; };
        if !SPECIALTIES.contains(&s) || (s == MANA && !mana.is_some_and(|b| self.city_specialty_manas().contains(&b))) { return false; }
        self.cities[c].specialty = Some(s);
        self.cities[c].specialty_class = class;
        self.cities[c].specialty_mana = mana;
        if s == MANA { self.city_change_free_bonus(c, mana.unwrap(), 1); }
        self.cities[c].specialty_options.clear();
        self.city_update_yield(c);
        self.city_update_commerces(c);
        self.cities[c].ai_assign_work_dirty = true;
        let owner = self.cities[c].owner;
        // Refresh conflicting offers immediately; no command can choose a stale duplicate.
        for o in self.players[owner].cities.clone() { if o != c && !self.cities[o].specialty_options.is_empty() { self.city_offer_specialty(o); } }
        true
    }

    /// Typed mana resources, excluding raw mana, in deterministic data order.
    pub fn city_specialty_manas(&self) -> Vec<BonusType> {
        self.d.bonuses.keys().filter(|&b| self.d.bonuses[b].bonus_class_type
            .is_some_and(|k| k.i() == self.d.defines.bonusclass_mana)).collect()
    }

    /// One food threshold buys one growth event; the specialty adds at most two citizens.
    pub fn city_population_growth(&self, c: CityId) -> i32 {
        if !self.city_has_specialty(c, DOUBLE_GROWTH) { return 1; }
        let happy = self.city_happy_level(c);
        let healthy = self.city_good_health(c);
        (1..=2).take_while(|&extra| {
            self.city_unhappy_level(c, extra) <= happy
                && (!self.city_ai_is_emphasize_avoid_unhealthy_citizens(c)
                    || self.city_bad_health(c, false, extra) <= healthy)
                && (!self.cities[c].outpost || self.cities[c].population + extra <= self.d.defines.outpost_max_pop)
        }).count() as i32
    }

    pub fn specialty_fast_plot(&self, p: PlotIdx, team: TeamId) -> bool {
        self.cities.iter().any(|(_, c)| c.specialty == Some(MOVEMENT) && self.players[c.owner].team == team && self.map.plots[p].owner.is_some_and(|o| self.players[o].team == team) && self.plot_distance(c.x, c.y, self.map.xy(p).0, self.map.xy(p).1) <= c.culture_level.map_or(0, |l| l.idx() as i32))
    }
}

impl Game {
    pub fn city_specialty_can_hurry(&self, c: CityId, h: HurryType) -> bool {
        (self.city_has_specialty(c, POP_RUSH) && self.d.hurries[h].production_per_population > 0)
            || (self.city_has_specialty(c, GOLD_RUSH) && self.d.hurries[h].gold_per_production > 0)
    }
    pub fn city_specialty_draft_ready(&self, c: CityId) -> bool {
        self.city_has_specialty(c, DRAFT) && self.game_turn() - self.cities[c].specialty_draft_turn >= 10
    }
}
