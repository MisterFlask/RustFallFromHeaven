//! The Crown's limit (docs/PERSONS.md, "The second build", rule 14 (a) to (d); the owner's
//! request of 2026-11-18: "the typical state should be the player managing 2-5 Houses depending on
//! empire size", and its revision of the same day).
//!
//! - **The Crown's limit** ([`Game::player_crown_limit`]): under every governing civic the Crown
//!   holds `PERSON_CROWN_CITIES` (3) cities at the normal city-count maintenance, plus its traits'
//!   `crown_cities_change` (the Kuriotates' Sprawling: -2, the capital alone), never below 1. The
//!   Crown's cities are the player's own great cities (not a settlement, not an outpost), the
//!   capital first, then in the player's city order ([`Game::player_crown_cities`]); those beyond
//!   the limit pay [`CROWN_OVER_FACTOR`] times their city-count maintenance
//!   (`city_calculate_num_cities_maintenance_times100`). There is no limit where no city can be
//!   granted (direct rule, a vassal, No Vassal States, a civilization without Houses): the remedy
//!   is a grant.
//! - **The realm's great cities** ([`Game::player_realm_great_cities`]): a player with a city cap
//!   (the Kuriotates' `max_cities`) counts its liege's and every House's of that liege together
//!   against it, so a House founding or receiving cities does not raise the realm's number of
//!   great cities. A city granted keeps what it was (a great city or a settlement).

use super::*;

/// The Crown's cities beyond its limit pay this many times their city-count maintenance.
pub const CROWN_OVER_FACTOR: i32 = 3;

impl Game {
    /// The cities player `p`'s Crown holds at the normal city-count maintenance (`None`: no
    /// limit, where it cannot grant cities): `PERSON_CROWN_CITIES` plus its traits'
    /// `crown_cities_change`, never below 1.
    pub fn player_crown_limit(&self, p: PlayerId) -> Option<i32> {
        if self.player_check_grants(p).is_err() {
            return None;
        }
        let d = self.d;
        let change: i32 = d.traits.keys().filter(|&t| self.players[p].traits[t]).map(|t| d.traits[t].crown_cities_change).sum();
        Some((d.defines.person_crown_cities + change).max(1))
    }

    /// The great cities (no settlement, no outpost) of player `p`'s realm: its liege's (or its
    /// own, for a player that is no House) and those of every House of that liege. A player's
    /// city cap (`max_cities`, the Kuriotates) counts them.
    pub fn player_realm_great_cities(&self, p: PlayerId) -> i32 {
        let head = self.player_house(p).map_or(p, |h| self.houses[h].owner);
        let great = |q: PlayerId| self.players[q].num_cities() - self.player_get_num_settlements(q) - self.player_get_num_outposts(q);
        great(head) + self.players[head].houses.iter().filter_map(|&h| self.houses.get(h)).map(|house| great(house.player)).sum::<i32>()
    }

    /// City `c`, which moved to a House or back from one (a grant, a founding, a break-away's
    /// held city), keeps what it was before (`settlement`): a great city stays great, a
    /// settlement stays one, whatever its new owner's count, where its new owner has a city cap
    /// (a player without one has no settlements).
    pub(crate) fn city_keep_settlement(&mut self, c: CityId, settlement: bool) {
        let d = self.d;
        let owner = self.cities[c].owner;
        if self.players[owner].max_cities == -1 || self.cities[c].outpost {
            return;
        }
        let changed = self.cities[c].settlement != settlement;
        self.cities[c].settlement = settlement;
        // Its radius as a great city or a settlement, or its radius building's.
        let building = d.buildings.keys().filter(|&b| d.buildings[b].plot_radius != 0 && self.city_num_active_building(c, b) > 0).map(|b| d.buildings[b].plot_radius).max().unwrap_or(0);
        let radius = self.city_base_plot_radius(c).max(building);
        self.city_set_plot_radius(c, radius);
        if changed {
            self.player_update_maintenance(owner);
        }
    }

    /// Whether city `c` counts in its Crown (a great city: no settlement, no
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
