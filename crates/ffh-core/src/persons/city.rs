//! What a House does to its cities (docs/PERSONS.md, "The first build": A head's trait,
//! Maintenance).
//!
//! **A head's trait.** The trait a House's cities have ([`Game::city_house_trait`], the saved
//! `City::house_trait`, kept by [`Game::city_house_refresh`]) is its head's
//! [`Game::person_house_trait`]. It applies in that city as if the owner had it, for the effects a
//! city can carry, each added where the city reads its owner's value: health and happiness
//! (`city_good_health`, `city_happy_level`, ...), the buildings' `happiness_traits`
//! (`city_update_extra_building_happiness`, `city_building_happiness`), `commerce_change`
//! (`city_base_commerce_rate_times100`), `commerce_modifier` (`city_total_commerce_rate_modifier`),
//! `extra_yield_threshold` (`plot_calculate_yield`, by the plot's working city),
//! `trade_yield_modifier` (`city_calculate_trade_yield`), `great_people_rate_modifier`
//! (`city_total_great_people_rate_modifier`), the three wonder production modifiers and
//! `production_traits` (`city_production_modifier_unit`/`_building`), and the free promotions of
//! the units it trains (`city_add_production_experience`). `city_has_trait` answers for it too.
//! The rest of a trait (upkeep, anarchy, the great general rate, pillage gold, upgrade cost,
//! summons, its flags) stays its owner's.
//!
//! **Maintenance.** A city of a House with a head pays `PERSON_HOUSE_MAINTENANCE_PERCENT` of its
//! number-of-cities maintenance (`city_calculate_num_cities_maintenance_times100`).

use ffh_data::*;

use super::*;

impl Game {
    /// The trait city `c` has from its House's head (`None`: the Crown's, an empty seat, a head
    /// without a trait the owner lacks). The saved value, kept by [`Game::city_house_refresh`].
    pub fn city_house_trait(&self, c: CityId) -> Option<TraitType> {
        self.cities.get(c)?.house_trait
    }

    /// The info of city `c`'s House trait, for the city's own reads.
    pub(crate) fn city_house_trait_info(&self, c: CityId) -> Option<&'static TraitInfo> {
        let d: &'static GameData = self.d;
        self.cities[c].house_trait.map(|t| &d.traits[t])
    }

    /// The trait city `c` should have by the rule now: its House's head's
    /// [`Game::person_house_trait`].
    fn city_house_trait_by_rule(&self, c: CityId) -> Option<TraitType> {
        let h = self.cities[c].house.filter(|&h| self.houses.contains(h))?;
        let x = self.houses[h].head.filter(|&x| self.persons.contains(x))?;
        self.person_house_trait(x)
    }

    /// Whether city `c` pays the House share of its number-of-cities maintenance: a city of a
    /// House with a head.
    pub(crate) fn city_house_maintenance_applies(&self, c: CityId) -> bool {
        self.cities[c].house.filter(|&h| self.houses.contains(h)).is_some_and(|h| self.houses[h].head.is_some_and(|x| self.persons.contains(x)))
    }

    /// City `c` after a change of its House or its head (a grant, a head leaving or arriving, a
    /// House ending or seceding, its owner's traits): its House trait taken anew, with every
    /// cache that reads it (yields, commerce, building happiness, trade routes), and its
    /// maintenance.
    pub fn city_house_refresh(&mut self, c: CityId) {
        if !self.cities.contains(c) {
            return;
        }
        let new = self.city_house_trait_by_rule(c);
        let old = self.cities[c].house_trait;
        if new != old {
            let d = self.d;
            self.cities[c].house_trait = new;
            self.city_update_extra_building_happiness(c);
            self.city_update_yield(c);
            self.city_update_commerces(c);
            let trades = |t: Option<TraitType>| t.is_some_and(|t| d.traits[t].trade_yield_modifier.iter().any(|&v| v != 0));
            if trades(old) || trades(new) {
                let owner = self.cities[c].owner;
                self.player_update_trade_routes(owner);
            }
            self.cities[c].ai_assign_work_dirty = true;
        }
        self.city_update_maintenance(c);
    }

    /// Every city of House `h` refreshed ([`Game::city_house_refresh`]).
    pub(crate) fn house_refresh_cities(&mut self, h: HouseId) {
        if !self.houses.contains(h) {
            return;
        }
        for c in self.houses[h].cities.clone() {
            self.city_house_refresh(c);
        }
    }

    /// Player `p`'s cities whose House trait or House share could be stale (its turn processing:
    /// its own traits may have changed).
    pub(crate) fn player_persons_cities(&mut self, p: PlayerId) {
        for c in self.players[p].cities.clone() {
            if self.cities[c].house.is_some() || self.cities[c].house_trait.is_some() {
                self.city_house_refresh(c);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Maintenance read
    // -----------------------------------------------------------------------------------------

    /// The maintenance (hundredths of gold a turn) city `c` pays less as a city of a House with a
    /// head than as the Crown's, at its present size and modifiers (whether or not it is a
    /// House's now): what a grant of it saves.
    pub fn city_house_maintenance_saved_times100(&self, c: CityId) -> i32 {
        let Some(city) = self.cities.get(c) else { return 0 };
        if self.city_is_disorder(c) || city.we_love_the_king_day {
            return 0;
        }
        self.city_house_maintenance_saved_lasting_times100(c)
    }

    /// What a grant of city `c` saves for good: [`Game::city_house_maintenance_saved_times100`]
    /// without the turns the city pays no city-count maintenance at all (disorder, We Love the
    /// King Day), which pass. A grant preview shows this one, with the condition.
    pub fn city_house_maintenance_saved_lasting_times100(&self, c: CityId) -> i32 {
        let Some(city) = self.cities.get(c) else { return 0 };
        if city.population <= 0 || city.outpost {
            return 0;
        }
        let full = self.city_calculate_num_cities_maintenance_full_times100(c);
        let share = full * self.d.defines.person_house_maintenance_percent / 100;
        (full - share) * (city.maintenance_modifier + 100).max(0) / 100
    }

    /// The maintenance (hundredths of gold a turn) House `h` saves its owner in all (0 with an
    /// empty seat).
    pub fn house_maintenance_saved_times100(&self, h: HouseId) -> i32 {
        let Some(house) = self.houses.get(h) else { return 0 };
        if !house.head.is_some_and(|x| self.persons.contains(x)) {
            return 0;
        }
        house.cities.iter().filter(|&&c| self.cities.contains(c)).map(|&c| self.city_house_maintenance_saved_times100(c)).sum()
    }
}
