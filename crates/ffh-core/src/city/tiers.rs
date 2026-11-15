//! Retired building-tier scaffolding. New cities use Standard buildings (city specialties).
//! Family lookups remain shared by building prerequisites and code effects.

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

impl Game {
    /// Initializes every class to Standard without consuming randomness.
    pub fn city_roll_building_tiers(&mut self, c: CityId) {
        self.cities[c].building_tiers = IdxVec::from_elem(BuildingTierType::STANDARD, self.d.building_classes.len());
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
