//! Building suitability (docs/SUITABILITY.md): this game's own rule, not FFH2's.
//!
//! A city rolls a tier for every building class at founding (`City::building_tiers`, kept for
//! the city's life, also when it changes hands). Each tier of a building is its own
//! `BuildingType` (building_variants.jsonc), and a city holds only its own tier's variant:
//!
//! - it can build only that variant (`city_can_construct`);
//! - a building given to it at another tier becomes its own tier's
//!   (`city_set_num_real_building_timed`, `city_set_num_free_building`), so events, spells,
//!   captures and free buildings need no tier of their own;
//! - "the city has X" asks for X at any tier: [`Game::city_num_family`] and its kin. Code that
//!   loops over every building type reads the exact counts instead.
//!
//! A wonder has no variants: its tier is its cost (`city_production_needed_building`).
//! The civilization buildings whose effects are code read their tier in `ffh/events.rs`.

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

impl Game {
    /// Rolls the city's tier for every class that rolls one (`BuildingClassInfo::rolls_tier`):
    /// one draw per class, each tier weighted by its `weight`.
    pub fn city_roll_building_tiers(&mut self, c: CityId) {
        let d = self.d;
        let total: i32 = d.building_tiers.iter().map(|t| t.weight.max(0)).sum();
        if total <= 0 {
            return;
        }
        for class in d.building_classes.keys() {
            if !d.building_classes[class].rolls_tier {
                continue;
            }
            let mut roll = self.soren_rand_num(total, "Building tier");
            let mut tier = BuildingTierType::STANDARD;
            for t in d.building_tiers.keys() {
                let w = d.building_tiers[t].weight.max(0);
                if roll < w {
                    tier = t;
                    break;
                }
                roll -= w;
            }
            self.cities[c].building_tiers[class] = tier;
        }
    }

    /// The city's tier for `class`.
    pub fn city_tier(&self, c: CityId, class: BuildingClassType) -> BuildingTierType {
        self.cities[c].building_tiers[class]
    }

    /// The city's tier for the class of `b`.
    pub fn city_building_tier(&self, c: CityId, b: BuildingType) -> BuildingTierType {
        match self.d.buildings[b].building_class_type {
            Some(class) => self.city_tier(c, class),
            None => BuildingTierType::STANDARD,
        }
    }

    /// The variant of `b`'s building at the city's tier: the one the city can hold (`b` for a
    /// building without tiers).
    pub fn city_building_variant(&self, c: CityId, b: BuildingType) -> BuildingType {
        if self.d.buildings[b].tier_variants.is_empty() {
            return b;
        }
        self.d.building_variant(b, self.city_building_tier(c, b))
    }

    /// The city's building for `class`: its civilization's building at the city's tier.
    pub fn city_building_for_class(&self, c: CityId, class: BuildingClassType) -> Option<BuildingType> {
        let b = self.d.civ_building(self.city_civilization_type(c), class)?;
        Some(self.city_building_variant(c, b))
    }

    /// `b` is a variant the city may hold: a building without tiers, or its own tier's.
    pub fn city_is_own_tier(&self, c: CityId, b: BuildingType) -> bool {
        let info = &self.d.buildings[b];
        info.tier_variants.is_empty() || info.tier == Some(self.city_building_tier(c, b))
    }

    /// C++ `getNumBuilding` for `b`'s building at any tier.
    pub fn city_num_family(&self, c: CityId, b: BuildingType) -> i32 {
        self.city_num_building(c, self.city_building_variant(c, b))
    }

    /// C++ `getNumRealBuilding` for `b`'s building at any tier.
    pub fn city_num_real_family(&self, c: CityId, b: BuildingType) -> i32 {
        self.cities[c].num_real_building[self.city_building_variant(c, b)]
    }

    /// The tier of `b`'s building in the city, if the city has it (real or free). Standard for
    /// a building without tiers.
    pub fn city_family_tier(&self, c: CityId, b: BuildingType) -> Option<BuildingTierType> {
        let held = self.city_building_variant(c, b);
        (self.city_num_building(c, held) > 0).then(|| self.d.buildings[held].tier.unwrap_or(BuildingTierType::STANDARD))
    }

    /// The tier of `b`'s building in the city if it has it for real (C++ `getNumRealBuilding`),
    /// as the rules' code effects test it.
    pub fn city_real_family_tier(&self, c: CityId, b: BuildingType) -> Option<BuildingTierType> {
        let held = self.city_building_variant(c, b);
        (self.cities[c].num_real_building[held] > 0).then(|| self.d.buildings[held].tier.unwrap_or(BuildingTierType::STANDARD))
    }

    /// `b`'s buildings at any tier in all of `p`'s cities (C++ `countNumBuildings`).
    pub fn player_count_num_family(&self, p: PlayerId, b: BuildingType) -> i32 {
        self.players[p].cities.iter().map(|&c| self.city_num_real_family(c, b)).sum()
    }

    /// A wonder's cost in the city, in percent: its tier's `wonder_cost_percent` (100 for a
    /// building with tiers or a class that rolls none).
    pub fn city_wonder_cost_percent(&self, c: CityId, b: BuildingType) -> i32 {
        let d = self.d;
        let Some(class) = d.buildings[b].building_class_type else { return 100 };
        let info = &d.building_classes[class];
        if !info.rolls_tier || !d.buildings[b].tier_variants.is_empty() {
            return 100;
        }
        d.building_tiers[self.city_tier(c, class)].wonder_cost_percent
    }

    /// What the city's buildings give the units they unlock (`unlocked_unit_*`) when unit type
    /// `u` is trained there: (experience, production modifier, promotions).
    pub fn city_unlocked_unit_bonus(&self, c: CityId, u: UnitType) -> (i32, i32, Vec<PromotionType>) {
        let d = self.d;
        let (mut xp, mut production, mut promotions) = (0, 0, Vec::new());
        for &b in &d.unlocking_buildings {
            if self.city_num_active_building(c, b) > 0 && d.building_unlocks_unit(b, u) {
                let info = &d.buildings[b];
                xp += info.unlocked_unit_experience;
                production += info.unlocked_unit_production_modifier;
                promotions.extend(info.unlocked_unit_promotion);
            }
        }
        (xp, production, promotions)
    }

    /// Percentage points the city's buildings add to its Sentinels' repair.
    pub fn city_sentinel_repair_change(&self, c: CityId) -> i32 {
        let d = self.d;
        d.sentinel_repair_buildings.iter().filter(|&&b| self.city_num_active_building(c, b) > 0).map(|&b| d.buildings[b].sentinel_repair_change).sum()
    }

    /// The city has a building with commerce per citizen (`population_commerce`).
    pub fn city_has_population_commerce(&self, c: CityId) -> bool {
        self.d.population_commerce_buildings.iter().any(|&b| self.city_num_building(c, b) > 0)
    }
}
