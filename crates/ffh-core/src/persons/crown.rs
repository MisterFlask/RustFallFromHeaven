//! The Crown's limit (docs/PERSONS.md, "The second build", rule 14 (a) to (d); the owner's
//! request of 2026-11-18: "the typical state should be the player managing 2-5 Houses depending on
//! empire size", and its revision of the same day).
//!
//! - **The Crown's limit** ([`Game::player_crown_limit`]): under every governing civic the Crown
//!   holds `PERSON_CROWN_CITIES` (3) cities at the normal city-count maintenance. The Crown's
//!   cities are the player's own cities but outposts, the capital first, then its full cities, then
//!   its settlements, each in the player's city order ([`Game::player_crown_cities`]); those beyond
//!   the limit pay [`CROWN_OVER_FACTOR`] times their city-count maintenance
//!   (`city_calculate_num_cities_maintenance_times100`). There is no limit where no city can be
//!   granted (direct rule, a vassal, No Vassal States, a civilization without Houses): the remedy
//!   is a grant.
//! - **The city cap** (the Kuriotates' `max_cities`, rule 14 as revised 2026-11-20): a player's own
//!   full cities count against it ([`Game::player_full_cities`]); a city that comes to a House with
//!   a cap is a settlement ([`Game::player_new_city_is_settlement`]), and a full city cannot be
//!   granted to one. A city moving within a realm otherwise keeps what it was.

use super::*;

/// The Crown's cities beyond its limit pay this many times their city-count maintenance.
pub const CROWN_OVER_FACTOR: i32 = 3;

impl Game {
    /// The cities player `p`'s Crown holds at the normal city-count maintenance (`None`: no
    /// limit, where it cannot grant cities): `PERSON_CROWN_CITIES`.
    pub fn player_crown_limit(&self, p: PlayerId) -> Option<i32> {
        if self.player_check_grants(p).is_err() {
            return None;
        }
        Some(self.d.defines.person_crown_cities)
    }

    /// Player `p`'s full cities (neither a settlement nor an outpost), `without` left out. A city
    /// cap (`max_cities`, the Kuriotates) counts them.
    pub fn player_full_cities(&self, p: PlayerId, without: Option<CityId>) -> i32 {
        self.players[p].cities.iter().filter(|&&c| Some(c) != without && self.cities.get(c).is_some_and(|k| !k.settlement && !k.outpost)).count() as i32
    }

    /// Whether city `c`, coming to player `p` now (founded, captured, raised from an outpost), is
    /// a settlement: `p` has a city cap and is a House (a capped House holds only settlements:
    /// docs/PERSONS.md, rule 14 as revised 2026-11-20), or its other full cities reach the cap.
    pub fn player_new_city_is_settlement(&self, p: PlayerId, c: CityId) -> bool {
        let max_cities = self.players[p].max_cities;
        max_cities != -1 && (self.player_house(p).is_some() || self.player_full_cities(p, Some(c)) >= max_cities)
    }

    /// City `c`, which moved to a House or back from one (a grant, a founding, a break-away's
    /// held city), keeps what it was before (`settlement`): a full city stays full, a
    /// settlement stays one, whatever its new owner's count, where its new owner has a city cap
    /// (a player without one has no settlements). (A grant never brings a full city to a House
    /// with a cap: `player_check_grant`.)
    pub(crate) fn city_keep_settlement(&mut self, c: CityId, settlement: bool) {
        let d = self.d;
        let owner = self.cities[c].owner;
        if self.players[owner].max_cities == -1 || self.cities[c].outpost {
            return;
        }
        let changed = self.cities[c].settlement != settlement;
        self.cities[c].settlement = settlement;
        // Its radius as a full city or a settlement, or its radius building's.
        let building = d.buildings.keys().filter(|&b| d.buildings[b].plot_radius != 0 && self.city_num_active_building(c, b) > 0).map(|b| d.buildings[b].plot_radius).max().unwrap_or(0);
        let radius = self.city_base_plot_radius(c).max(building);
        self.city_set_plot_radius(c, radius);
        if changed {
            self.player_update_maintenance(owner);
        }
    }

    /// Whether city `c` counts in its Crown (any city but an outpost).
    fn city_counts_for_crown(&self, c: CityId) -> bool {
        self.cities.get(c).is_some_and(|city| !city.outpost)
    }

    /// Player `p`'s Crown cities, the capital first, then its full cities, then its settlements,
    /// each in its city order; `without` left out (a grant's preview).
    pub(crate) fn player_crown_cities_without(&self, p: PlayerId, without: Option<CityId>) -> Vec<CityId> {
        let capital = self.player_get_capital_city(p);
        let mut out: Vec<CityId> = Vec::new();
        if let Some(cap) = capital.filter(|&k| Some(k) != without && self.city_counts_for_crown(k)) {
            out.push(cap);
        }
        let rest: Vec<CityId> = self.players[p].cities.iter().copied().filter(|&k| Some(k) != capital && Some(k) != without && self.city_counts_for_crown(k)).collect();
        out.extend(rest.iter().copied().filter(|&k| !self.cities[k].settlement));
        out.extend(rest.iter().copied().filter(|&k| self.cities[k].settlement));
        out
    }

    /// Player `p`'s Crown cities: the capital first, then its full cities, then its settlements
    /// (outposts left out).
    pub fn player_crown_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.player_crown_cities_without(p, None)
    }

    /// Whether city `c` is beyond its Crown's limit (`without`: were that city granted away).
    pub(crate) fn city_crown_over_without(&self, c: CityId, without: Option<CityId>) -> bool {
        let p = self.cities[c].owner;
        let Some(limit) = self.player_crown_limit(p) else { return false };
        if !self.city_counts_for_crown(c) {
            return false;
        }
        self.player_crown_cities_without(p, without).iter().position(|&k| k == c).is_some_and(|i| i as i32 >= limit)
    }

    /// Player `p`'s Crown cities beyond its limit, in the Crown's order.
    pub fn player_crown_over(&self, p: PlayerId) -> Vec<CityId> {
        let Some(limit) = self.player_crown_limit(p) else { return Vec::new() };
        self.player_crown_cities(p).into_iter().skip(limit.max(0) as usize).collect()
    }

    /// The gold a turn (hundredths, inflation included) player `p` pays for its Crown cities
    /// beyond the limit above the normal rate: their city-count maintenance times
    /// `CROWN_OVER_FACTOR - 1` (0 in disorder or We Love the King Day, as a city pays none then).
    pub fn player_crown_over_cost_times100(&self, p: PlayerId) -> i32 {
        self.player_crown_over_cost_without_times100(p, None, false)
    }

    /// [`Game::player_crown_over_cost_times100`] were city `without` granted to a House (a
    /// grant's preview; `None`: as it is). `lasting`: a city that pays none for a while (disorder,
    /// We Love the King Day) is counted at what it pays once that ends.
    pub fn player_crown_over_cost_without_times100(&self, p: PlayerId, without: Option<CityId>, lasting: bool) -> i32 {
        let Some(limit) = self.player_crown_limit(p) else { return 0 };
        let mut extra = 0;
        for c in self.player_crown_cities_without(p, without).into_iter().skip(limit.max(0) as usize) {
            let city = &self.cities[c];
            if city.population <= 0 || (!lasting && (self.city_is_disorder(c) || city.we_love_the_king_day)) {
                continue;
            }
            let base = self.city_num_cities_base_times100(c, without);
            extra += base * (CROWN_OVER_FACTOR - 1) * (city.maintenance_modifier + 100).max(0) / 100;
        }
        let inflation = (self.player_calculate_inflation_rate(p) + 100).max(0);
        extra * inflation / 100
    }
}
