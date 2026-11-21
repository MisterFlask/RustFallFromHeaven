//! What a perk does (docs/HOUSES.md, rule 9): its effects at the scope of one city, read by the
//! rules where the same kind of modifier from a building or a civic enters, and the refresh of
//! the city's stored values when its perk changes.
//!
//! A perk is in effect in a city while its House's switch (`House::perk_on`) is on; the switch
//! changes only in the owner's turn pass (`player_houses_tell_states`), so the stored values
//! that a perk enters (the commerce rates, through the yield and commerce modifiers, and the
//! maintenance) change at known moments: [`Game::city_house_perk_changed`].

use ffh_data::*;

use super::pick::signed;
use super::*;

impl Game {
    /// The trait row of the perk in effect in city `c`. Hot path: `city.house` -> the House's
    /// switch -> the row.
    #[inline]
    pub fn city_house_perk_info(&self, c: CityId) -> Option<&'static TraitInfo> {
        let h = self.cities[c].house?;
        let house = self.houses.get(h)?;
        let d: &'static GameData = self.d;
        house.perk_on.then(|| &d.traits[house.perk])
    }

    /// The perk's yield rate modifier in city `c` (percent; 0 with no perk).
    #[inline]
    pub fn city_house_yield_modifier(&self, c: CityId, y: YieldType) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_yield_modifier[y.idx()])
    }

    /// The perk's commerce rate modifier in city `c` (percent).
    #[inline]
    pub fn city_house_commerce_modifier(&self, c: CityId, k: CommerceType) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_commerce_modifier[k.idx()])
    }

    /// The perk's great people rate modifier in city `c` (percent).
    #[inline]
    pub fn city_house_great_people_rate_modifier(&self, c: CityId) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_great_people_rate_modifier)
    }

    /// The perk's modifier of city `c`'s distance maintenance (percent).
    #[inline]
    pub fn city_house_distance_maintenance_modifier(&self, c: CityId) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_distance_maintenance_modifier)
    }

    /// The perk's modifier of city `c`'s city-count maintenance (percent).
    #[inline]
    pub fn city_house_num_cities_maintenance_modifier(&self, c: CityId) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_num_cities_maintenance_modifier)
    }

    /// The perk's experience for a unit of type `unit` trained in city `c` (every unit's, and
    /// its combat's); with `unit` `None`, every unit's alone.
    pub fn city_house_unit_experience(&self, c: CityId, unit: Option<UnitType>) -> i32 {
        let Some(t) = self.city_house_perk_info(c) else { return 0 };
        let combat = unit.and_then(|u| self.d.units[u].unit_combat_type).map_or(0, |k| t.house_unit_combat_free_experience[k]);
        t.house_free_experience + combat
    }

    /// The perk's production modifier of unit type `unit` in city `c` (its combat's).
    #[inline]
    pub fn city_house_unit_production_modifier(&self, c: CityId, unit: UnitType) -> i32 {
        let Some(t) = self.city_house_perk_info(c) else { return 0 };
        self.d.units[unit].unit_combat_type.map_or(0, |k| t.house_unit_combat_production_modifier[k])
    }

    /// The perk's production modifier of building `b` in city `c`: a world or national wonder's.
    pub fn city_house_building_production_modifier(&self, c: CityId, b: BuildingType) -> i32 {
        let Some(t) = self.city_house_perk_info(c) else { return 0 };
        if t.house_wonder_production_modifier == 0 {
            return 0;
        }
        let d = self.d;
        let wonder = d.buildings[b].building_class_type.is_some_and(|k| crate::utils::is_world_wonder_class(d, k) || crate::utils::is_national_wonder_class(d, k));
        if wonder {
            t.house_wonder_production_modifier
        } else {
            0
        }
    }

    /// The perk's production modifier of a project (a ritual) in city `c`.
    #[inline]
    pub fn city_house_project_production_modifier(&self, c: CityId, _project: ProjectType) -> i32 {
        self.city_house_perk_info(c).map_or(0, |t| t.house_ritual_production_modifier)
    }

    /// The promotions the perk gives a unit of combat `k` trained in city `c`, in their order.
    pub fn city_house_free_promotions(&self, c: CityId, k: UnitCombatType) -> &'static [PromotionType] {
        self.city_house_perk_info(c).map_or(&[], |t| t.house_unit_combat_free_promotions[k].as_slice())
    }

    /// The perk's part in a calculation, labelled (`House Aldric (Scholars II)`): `None` where no
    /// perk is in effect in city `c`.
    pub fn city_house_perk_label(&self, c: CityId) -> Option<String> {
        let h = self.cities[c].house?;
        let house = self.houses.get(h)?;
        house.perk_on.then(|| format!("{} ({})", self.house_label(h), perk_name(self.d, house.perk)))
    }

    /// A percent part of a calculation from city `c`'s perk: `House Aldric (Scholars II): +25%`;
    /// `None` where the perk gives `n` = 0 or none is in effect.
    pub fn city_house_percent_part(&self, c: CityId, n: i32) -> Option<String> {
        if n == 0 {
            return None;
        }
        Some(format!("{}: {}%", self.city_house_perk_label(c)?, signed(n)))
    }

    /// City `c`'s perk changed (its House's switch flipped, it joined or left a House, its House
    /// formed or ended): the stored values a perk enters are worked out again, through the
    /// functions a building or a civic that changes the same values uses. The yield rate and
    /// commerce rate modifiers enter the stored commerce rates (`city_update_commerces`: commerce
    /// from the commerce yield, and from production through the processes); the maintenance
    /// modifiers the stored maintenance (`city_update_maintenance`). Every other value a perk
    /// enters (the yield rates, the great people rate, the production modifiers, the experience)
    /// is worked out when asked; the citizens are assigned again.
    pub fn city_house_perk_changed(&mut self, c: CityId) {
        if !self.cities.contains(c) {
            return;
        }
        self.city_update_commerces(c);
        self.city_update_maintenance(c);
        self.cities[c].ai_assign_work_dirty = true;
    }

    /// Gives a unit trained in city `c` the perk's promotions for its combat, in order, where
    /// its combat may hold each (as a building's free promotions are given). It keeps them.
    pub(crate) fn city_house_give_promotions(&mut self, c: CityId, unit: UnitId) {
        let Some(k) = self.unit_get_unit_combat_type(unit) else { return };
        let promotions = self.city_house_free_promotions(c, k);
        for &promotion in promotions {
            if !self.units.contains(unit) {
                return;
            }
            if self.d.promotions[promotion].unit_combat[k] {
                self.unit_set_has_promotion(unit, promotion, true);
            }
        }
    }
}
