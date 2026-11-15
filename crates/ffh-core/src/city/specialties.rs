//! Permanent city specialties and saved, deterministic pick-one-of-three offers.
use crate::{game::Game, ids::*};
use ffh_data::*;

pub const CLASS_XP: u8 = 0;
pub const MOBILITY: u8 = 1;
pub const NO_UPKEEP: u8 = 2;
pub const WONDERS: u8 = 3;
pub const MINES: u8 = 4;
pub const FARMS: u8 = 5;
pub const SETTLERS: u8 = 6;
pub const GREAT_PEOPLE: u8 = 7;
pub const SAGES: u8 = 8;
pub const CULTURE: u8 = 9;
pub const MISSIONARIES: u8 = 10;
pub const RELIGIONS: u8 = 11;
pub const WATER: u8 = 12;
pub const NAVAL: u8 = 13;
pub const SEA_TRADE: u8 = 14;
pub const HEALING: u8 = 15;
pub const MOVEMENT: u8 = 16;
pub const FOOD: u8 = 17;
pub const POP_RUSH: u8 = 18;
pub const DRAFT: u8 = 19;
pub const GOLD_RUSH: u8 = 20;
pub const COUNT: u8 = 21;

impl Game {
    pub fn city_has_specialty(&self, c: CityId, s: u8) -> bool { self.cities[c].specialty == Some(s) }

    pub fn specialty_text(&self, s: u8, class: Option<UnitCombatType>) -> String {
        if s == CLASS_XP {
            let name = class.map(|k| {
                let key = &self.d.unit_combats[k].base.description;
                let text = self.d.text.name(key);
                if text.starts_with("TXT_KEY_") { self.d.unit_combats[k].base.type_.trim_start_matches("UNITCOMBAT_") } else { text }
            }).unwrap_or("Class");
            return format!("{name} units trained here get +3 XP.");
        }
        match s {
            MOBILITY => "Units trained here start with Mobility I.",
            NO_UPKEEP => "Units trained here cost no upkeep.",
            WONDERS => "Wonders are built here at 1.5x speed.",
            MINES => "Mines and quarries worked by this city give +1 production.",
            FARMS => "Farms worked by this city give +1 food.",
            SETTLERS => "Settlers trained here do not stop growth, and the cities they found start at size 2.",
            GREAT_PEOPLE => "Great Person points are doubled here.",
            SAGES => "Each specialist in this city gives +1 research.",
            CULTURE => "1.5x culture in this city.",
            MISSIONARIES => "Missionaries trained here never fail to spread their religion.",
            RELIGIONS => "Each religion in this city gives +2 culture and +1 gold.",
            WATER => "Water plots worked by this city give +1 food.",
            NAVAL => "Naval units trained here get +1 movement.",
            SEA_TRADE => "Trade routes with other coastal cities yield double here.",
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
            WATER | NAVAL | SEA_TRADE => self.city_is_coastal(c, 0),
            MINES | FARMS => (0..crate::defs::NUM_CITY_PLOTS).filter_map(|i| self.plot_city(self.cities[c].x, self.cities[c].y, i)).any(|p| {
                if s == MINES { ["IMPROVEMENT_MINE", "IMPROVEMENT_QUARRY"].iter().any(|name| self.d.ty::<ImprovementType>(name).is_some_and(|i| self.map.plots[p].improvement_type == Some(i) || self.plot_can_have_improvement(p, i, Some(self.city_team(c)), true))) }
                else { self.d.ty::<ImprovementType>("IMPROVEMENT_FARM").is_some_and(|farm| self.map.plots[p].improvement_type == Some(farm) || self.plot_can_have_improvement(p, farm, Some(self.city_team(c)), true)) }
            }),
            _ => s < COUNT,
        }
    }

    /// Only chosen specialties reserve an entry. Exhaustion permits repeats so every city gets
    /// three choices; choices themselves are always distinct. Unchosen offers reserve nothing.
    pub fn city_offer_specialty(&mut self, c: CityId) {
        if self.cities[c].outpost || self.cities[c].specialty.is_some() { return; }
        if self.cities[c].culture_level.is_none_or(|l| l.idx() < 3) { return; }
        let owner = self.cities[c].owner;
        let used: Vec<u8> = self.players[owner].cities.iter().filter(|&&o| o != c).filter_map(|&o| self.cities[o].specialty).collect();
        let eligible: Vec<u8> = (0..COUNT).filter(|&s| self.city_specialty_eligible(c, s)).collect();
        let mut pool: Vec<u8> = eligible.iter().copied().filter(|s| !used.contains(s)).collect();
        let unused = pool.len();
        let mut repeats: Vec<_> = eligible.into_iter().filter(|s| used.contains(s)).collect();
        // A previously offered option may have been chosen in another city. Keep all three
        // offers stable until that happens, including across saves and repeated queries.
        if self.cities[c].specialty_options.len() == 3 && self.cities[c].specialty_options.iter().all(|(s, _)| !used.contains(s) || unused < 3) {
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
            options.push((s, class));
        }
        self.cities[c].specialty_options = options;
        if !self.players[owner].human {
            // A modest local choice: favor food, specialists and existing military production.
            let pick = self.cities[c].specialty_options.iter().enumerate().max_by_key(|(_, (s, _))| match *s { FOOD => 100, GREAT_PEOPLE => 70 + self.cities[c].population, CLASS_XP | MOBILITY | NO_UPKEEP => 60, _ => 40 }).map(|(i, _)| i).unwrap_or(0);
            self.city_choose_specialty(c, pick);
        }
    }

    pub fn city_choose_specialty(&mut self, c: CityId, option: usize) -> bool {
        if self.cities[c].specialty.is_some() { return false; }
        let Some(&(s, class)) = self.cities[c].specialty_options.get(option) else { return false; };
        self.cities[c].specialty = Some(s);
        self.cities[c].specialty_class = class;
        self.cities[c].specialty_options.clear();
        self.city_update_yield(c);
        self.city_update_commerces(c);
        self.city_update_trade_routes(c);
        self.cities[c].ai_assign_work_dirty = true;
        let owner = self.cities[c].owner;
        // Refresh conflicting offers immediately; no command can choose a stale duplicate.
        for o in self.players[owner].cities.clone() { if o != c && !self.cities[o].specialty_options.is_empty() { self.city_offer_specialty(o); } }
        true
    }

    pub fn city_specialty_plot_yield(&self, c: CityId, p: PlotIdx, y: YieldType) -> i32 {
        let s = self.cities[c].specialty;
        let imp = self.map.plots[p].improvement_type.map(|i| self.d.improvements[i].base.type_.as_str()).unwrap_or("");
        i32::from((s == Some(WATER) && y == YieldType::YIELD_FOOD && self.plot_is_water(p))
            || (s == Some(FARMS) && y == YieldType::YIELD_FOOD && imp == "IMPROVEMENT_FARM")
            || (s == Some(MINES) && y == YieldType::YIELD_PRODUCTION && matches!(imp, "IMPROVEMENT_MINE" | "IMPROVEMENT_QUARRY")))
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
