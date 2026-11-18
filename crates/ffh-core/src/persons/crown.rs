//! The Crown's limit and a House's size (docs/PERSONS.md, "The second build", rule 14; the owner's
//! request of 2026-11-18: "the typical state should be the player managing 2-5 Houses depending on
//! empire size").
//!
//! - **The Crown's limit** ([`Game::player_crown_limit`]): each governing civic sets how many
//!   cities the Crown holds at the normal city-count maintenance (`CivicInfo::crown_cities`; 0: no
//!   limit). The Crown's cities are the player's own that pay city-count maintenance (not a
//!   settlement, not an outpost), the capital first, then in the player's city order
//!   ([`Game::player_crown_cities`]); those beyond the limit pay [`CROWN_OVER_FACTOR`] times their
//!   city-count maintenance (`city_calculate_num_cities_maintenance_times100`). There is no limit
//!   where no city can be granted (direct rule, a vassal, No Vassal States, a civilization without
//!   Houses): the remedy is a grant.
//! - **A House's size**: a House holding more than [`HOUSE_CITIES_FREE`] cities has the standing
//!   loyalty part `Holds N cities: -(N - 3)` (`loyalty.rs`), as `State religion opposed` is.

use super::*;

/// The Crown's cities beyond its limit pay this many times their city-count maintenance.
pub const CROWN_OVER_FACTOR: i32 = 3;
/// A House holds this many cities without a grievance; each further one is -1, standing.
pub const HOUSE_CITIES_FREE: i32 = 3;

impl Game {
    /// The cities player `p`'s Crown holds at the normal city-count maintenance (`None`: no
    /// limit): the largest `crown_cities` of its civics, where it may grant cities.
    pub fn player_crown_limit(&self, p: PlayerId) -> Option<i32> {
        if self.player_check_grants(p).is_err() {
            return None;
        }
        let limit = self.players[p].civics.iter().flatten().map(|&c| self.d.civics[c].crown_cities).max().unwrap_or(0);
        (limit > 0).then_some(limit)
    }

    /// Whether city `c` counts in its Crown (it pays city-count maintenance: no settlement, no
    /// outpost).
    fn city_counts_for_crown(&self, c: CityId) -> bool {
        self.cities.get(c).is_some_and(|city| !city.settlement && !city.outpost)
    }

    /// Player `p`'s Crown cities, the capital first, then in its city order; `without` left out
    /// (a grant's preview).
    pub(crate) fn player_crown_cities_without(&self, p: PlayerId, without: Option<CityId>) -> Vec<CityId> {
        let capital = self.player_get_capital_city(p);
        let mut out: Vec<CityId> = Vec::new();
        if let Some(cap) = capital.filter(|&k| Some(k) != without && self.city_counts_for_crown(k)) {
            out.push(cap);
        }
        out.extend(self.players[p].cities.iter().copied().filter(|&k| Some(k) != capital && Some(k) != without && self.city_counts_for_crown(k)));
        out
    }

    /// Player `p`'s Crown cities: the capital first, then in its city order (settlements and
    /// outposts left out).
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
        let mut extra = 0;
        for c in self.player_crown_over(p) {
            let city = &self.cities[c];
            if self.city_is_disorder(c) || city.we_love_the_king_day || city.population <= 0 {
                continue;
            }
            let base = self.city_num_cities_base_times100(c, None);
            extra += base * (CROWN_OVER_FACTOR - 1) * (city.maintenance_modifier + 100).max(0) / 100;
        }
        let inflation = (self.player_calculate_inflation_rate(p) + 100).max(0);
        extra * inflation / 100
    }

    /// The standing loyalty part of House `h`'s size: `-(N - 3)` for more than
    /// [`HOUSE_CITIES_FREE`] cities.
    pub fn house_size_part(&self, h: HouseId) -> i32 {
        Self::size_part(self.house_cities(h).len() as i32)
    }

    /// The standing part of a House of `n` cities.
    pub(crate) fn size_part(n: i32) -> i32 {
        -(n - HOUSE_CITIES_FREE).max(0)
    }
}
