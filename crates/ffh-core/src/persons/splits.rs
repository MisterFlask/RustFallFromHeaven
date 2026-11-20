//! Houses split (docs/PERSONS.md, "The third build", rule 4).
//!
//! In its liege's turn processing, after the tribute, each House holding
//! `PERSON_HOUSE_SPLIT_CITIES` (6) cities or more (outposts left out; settlements count), in id
//! order, splits with a chance of `PERSON_HOUSE_SPLIT_PERCENT` (5): one draw `soren_rand_num(100)`
//! per such House a turn. The parent keeps its capital and the half of its cities nearest it (half
//! rounded up, by plot distance from its capital, ties to the lower city id); the others go to a
//! **cadet House**, born as a founding is ([`Game::player_found_house_as`]: a new player of the
//! parent's civilization, its leader drawn from the free minor leaders, the liege's vassal). Units
//! go with their cities as in a grant (the garrison: the parent's land combat units in them, those
//! in an army or marching left out); units elsewhere stay the parent's. The cadet inherits nothing:
//! no modifiers, no request, no contract, no memory, no ultimatum count; its tribute is its realm's
//! default, in force. When no leader is free, the House does not split that turn.

use super::*;

impl Game {
    /// The cities that count toward House `h`'s split: its cities but outposts.
    pub fn house_split_cities(&self, h: HouseId) -> i32 {
        self.house_cities(h).into_iter().filter(|&c| !self.cities[c].outpost).count() as i32
    }

    /// The House card's split line, when House `h` may split: `Holds 7 cities: 5% a turn to
    /// split.`
    pub fn house_split_line(&self, h: HouseId) -> Option<String> {
        let n = self.house_split_cities(h);
        let d = &self.d.defines;
        (n >= d.person_house_split_cities).then(|| format!("Holds {n} cities: {}% a turn to split.", d.person_house_split_percent))
    }

    /// A grant's split line, when granting city `c` (not an outpost) to House `h` brings it to the
    /// split's threshold or beyond: `House Aldric will hold 6 cities: 5% a turn to split.`
    pub fn house_split_line_after_grant(&self, h: HouseId, c: CityId) -> Option<String> {
        if !self.houses.contains(h) || !self.cities.contains(c) || self.cities[c].outpost {
            return None;
        }
        let n = self.house_split_cities(h) + 1;
        let d = &self.d.defines;
        (n >= d.person_house_split_cities).then(|| format!("{} will hold {n} cities: {}% a turn to split.", self.house_label(h), d.person_house_split_percent))
    }

    /// How House `h` would split (pure): the cities it keeps (its seat and the nearest half,
    /// rounded up, by plot distance from its seat, ties to the lower city id) and the cadet's (its
    /// most populous first, its capital to be; ties: the lower id). `None` with fewer than 2 cities.
    pub fn house_split_division(&self, h: HouseId) -> Option<(Vec<CityId>, Vec<CityId>)> {
        let seat = self.house_seat(h)?;
        let (sx, sy) = (self.cities[seat].x, self.cities[seat].y);
        let mut cities = self.house_cities(h);
        if cities.len() < 2 {
            return None;
        }
        cities.sort_by_key(|&c| (c != seat, self.plot_distance(sx, sy, self.cities[c].x, self.cities[c].y), c));
        let keep = cities.len().div_ceil(2);
        let mut cadet = cities.split_off(keep);
        cadet.sort_by_key(|&c| (std::cmp::Reverse(self.cities[c].population), c));
        Some((cities, cadet))
    }

    /// House `h` splits now (no roll): the cadet House, or `None` when no leader is free (the House
    /// does not split) or it cannot be divided.
    pub fn house_split(&mut self, h: HouseId) -> Option<HouseId> {
        let (_, cadet) = self.house_split_division(h)?;
        let (p, hp) = (self.houses[h].owner, self.houses[h].player);
        let civ = self.player_get_civilization_type(hp)?;
        let Some(leader) = self.house_leader_draw(civ, p) else {
            self.tel_house(p, "split", "no_leader");
            return None;
        };
        let parent = self.house_label(h);
        let names = self.city_names_of(&cadet);
        let (nh, _) = self.player_found_house_as(p, civ, leader, &cadet, HouseOrigin::Split)?;
        let (child, led) = (self.house_label(nh), leader_name(self.d, leader));
        self.house_add_deed(nh, format!("Split from {parent}, led by {led}, with {names}."));
        if self.houses.contains(h) {
            self.house_add_deed(h, format!("Split: {child} took {names}."));
        }
        self.house_msg(nh, || format!("{parent} splits: {child}, led by {led}, takes {names}."));
        self.tel_house(p, "split", "split");
        Some(nh)
    }

    /// Player `p`'s Houses that hold `PERSON_HOUSE_SPLIT_CITIES` great cities or more split with a
    /// chance of `PERSON_HOUSE_SPLIT_PERCENT`, one draw each, in id order (its turn processing,
    /// after the tribute).
    pub(crate) fn player_persons_splits(&mut self, p: PlayerId) {
        let (at, pct) = (self.d.defines.person_house_split_cities, self.d.defines.person_house_split_percent);
        let mut houses = self.player_houses(p);
        houses.sort();
        for h in houses {
            if !self.houses.contains(h) || self.house_split_cities(h) < at {
                continue;
            }
            if self.soren_rand_num(100, "House split") < pct {
                self.house_split(h);
            }
        }
    }
}
