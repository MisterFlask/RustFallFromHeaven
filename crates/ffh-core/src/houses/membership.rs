//! Membership (docs/HOUSES.md, rule 4): a city gained joins a House at once; the unhoused cities
//! join or form Houses in their owner's turn processing; a city lost or destroyed leaves its
//! House, and a House with no city ends.

use super::*;

impl Game {
    /// The House of city `c`'s owner and origin it would join now (rule 4): fewer than
    /// `HOUSE_MAX_CITIES` cities, its seat nearest `c` and within `HOUSE_JOIN_RANGE` plots (a tie
    /// to the lower House id).
    pub fn city_house_to_join(&self, c: CityId) -> Option<HouseId> {
        let city = &self.cities[c];
        let origin = self.city_origin(c)?;
        let x = &self.d.defines;
        let mut best: Option<(i32, HouseId)> = None;
        for &h in &self.players[city.owner].houses {
            let Some(house) = self.houses.get(h) else { continue };
            if house.origin != origin || house.cities.len() >= x.house_max_cities as usize || house.cities.contains(&c) {
                continue;
            }
            let Some(seat) = house.seat() else { continue };
            let dist = self.plot_distance(city.x, city.y, self.cities[seat].x, self.cities[seat].y);
            if dist > x.house_join_range {
                continue;
            }
            if best.map_or(true, |(bd, bh)| dist < bd || (dist == bd && h < bh)) {
                best = Some((dist, h));
            }
        }
        best.map(|(_, h)| h)
    }

    /// City `c` joins House `h`.
    fn house_add_city(&mut self, h: HouseId, c: CityId, tell: bool) {
        self.houses[h].cities.push(c);
        self.cities[c].house = Some(h);
        self.city_house_perk_changed(c);
        if tell {
            let name = self.d.text.name(&self.cities[c].name).to_string();
            let label = self.house_label(h);
            self.house_msg(h, || format!("{name} joins {label}."));
        }
    }

    /// Whether city `c` can be in a House: not an outpost, of a player that has Houses.
    fn city_can_be_housed(&self, c: CityId) -> bool {
        let city = &self.cities[c];
        !city.outpost && city.house.is_none() && self.player_has_houses(city.owner)
    }

    /// A city gained (founded, captured, traded, flipped, born, raised from an outpost): it
    /// joins a House at once where one takes it (rule 4).
    pub fn house_city_gained(&mut self, c: CityId) {
        if !self.cities.contains(c) || !self.city_can_be_housed(c) {
            return;
        }
        if let Some(h) = self.city_house_to_join(c) {
            self.house_add_city(h, c, true);
        }
    }

    /// A city about to change owner or be destroyed leaves its House (rule 4); the seat passes
    /// to the next city, and a House with no city ends.
    pub fn house_city_lost(&mut self, c: CityId) {
        let Some(h) = self.cities.get(c).and_then(|city| city.house) else { return };
        self.cities[c].house = None;
        self.city_house_perk_changed(c);
        let Some(house) = self.houses.get_mut(h) else { return };
        house.cities.retain(|&x| x != c);
        if house.cities.is_empty() {
            self.house_end(h);
        }
    }

    /// House `h` ends: it holds no city; its leader is free again.
    fn house_end(&mut self, h: HouseId) {
        let label = self.house_label(h);
        let owner = self.houses[h].owner;
        self.house_msg(h, || format!("{label} ends: it holds no city."));
        self.tel_house(owner, "ended", "no_city");
        for &c in &self.houses[h].cities.clone() {
            if self.cities.contains(c) {
                self.cities[c].house = None;
                self.city_house_perk_changed(c);
            }
        }
        self.houses.remove(h);
        self.players[owner].houses.retain(|&x| x != h);
    }

    /// Whether player `p` has a home House (of its own civilization).
    pub fn player_has_home_house(&self, p: PlayerId) -> bool {
        let civ = self.player_get_civilization_type(p);
        self.players[p].houses.iter().any(|&h| self.houses.get(h).is_some_and(|house| Some(house.origin) == civ))
    }

    /// Player `p`'s unhoused home cities (outposts left out), in its city order.
    pub fn player_unhoused_home_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.players[p].cities.iter().copied().filter(|&c| !self.cities[c].outpost && self.cities[c].house.is_none() && self.city_is_home(c)).collect()
    }

    /// The Houses pass of player `p`'s turn processing (rules 4, 5, 11): each unhoused city in
    /// its city order joins a House, or waits (the first home House), or forms one; then each
    /// House's change of state is told.
    pub fn player_houses_turn(&mut self, p: PlayerId) {
        if !self.player_has_houses(p) {
            return;
        }
        self.players[p].house_turn = self.game_turn();
        let x_first = self.d.defines.house_first_cities.max(1) as usize;
        let x_max = self.d.defines.house_max_cities.max(1) as usize;
        let range = self.d.defines.house_join_range;
        for c in self.players[p].cities.clone() {
            if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_can_be_housed(c) {
                continue;
            }
            if let Some(h) = self.city_house_to_join(c) {
                self.house_add_city(h, c, true);
                continue;
            }
            if self.city_is_home(c) && !self.player_has_home_house(p) {
                let waiting = self.player_unhoused_home_cities(p);
                if waiting.len() < x_first {
                    continue;
                }
                // The first House: the capital as its seat (if it is an unhoused home city, else
                // the first of them), with the unhoused home cities in range, nearest first.
                let capital = self.player_get_capital_city(p).filter(|k| waiting.contains(k));
                let seat = capital.unwrap_or(waiting[0]);
                let (sx, sy) = (self.cities[seat].x, self.cities[seat].y);
                let mut others: Vec<(i32, CityId)> =
                    waiting.iter().filter(|&&k| k != seat).map(|&k| (self.plot_distance(sx, sy, self.cities[k].x, self.cities[k].y), k)).filter(|&(dist, _)| dist <= range).collect();
                others.sort();
                let mut members = vec![seat];
                members.extend(others.into_iter().map(|(_, k)| k).take(x_max - 1));
                self.house_form(p, members);
                // `c` may be out of the first House's range: it joins or forms like any other.
                if self.cities.contains(c) && self.city_can_be_housed(c) {
                    if let Some(h) = self.city_house_to_join(c) {
                        self.house_add_city(h, c, true);
                    } else if self.player_has_home_house(p) {
                        self.house_form(p, vec![c]);
                    }
                }
                continue;
            }
            self.house_form(p, vec![c]);
        }
        self.player_houses_tell_states(p);
    }

    /// Each House of `p` whose state changed since it was last told: `House Aldric is now
    /// Furious: its perk is off.` / `House Aldric is now Annoyed.` The perk switches here, and
    /// only here (rule 9): off when the House is Furious now, on when it is not; its cities'
    /// stored values follow.
    pub fn player_houses_tell_states(&mut self, p: PlayerId) {
        for h in self.players[p].houses.clone() {
            if !self.houses.contains(h) {
                continue;
            }
            let state = self.house_state(h);
            let on = state != HouseState::Furious;
            if on != self.houses[h].perk_on {
                self.houses[h].perk_on = on;
                for c in self.houses[h].cities.clone() {
                    self.city_house_perk_changed(c);
                }
            }
            if state == self.houses[h].told_state {
                continue;
            }
            self.houses[h].told_state = state;
            let label = self.house_label(h);
            self.house_msg(h, || {
                if state == HouseState::Furious {
                    format!("{label} is now Furious: its perk is off.")
                } else {
                    format!("{label} is now {}.", state.name())
                }
            });
            self.tel_house(p, "state", state.name());
        }
    }

    /// A House of `p` forms with `cities` (the seat first; every one unhoused, of `p`, of one
    /// origin): candidates rolled, the pick through `p`'s controller, the record made (rules 5,
    /// 6, 9, 11). `None` when no leader is free (it does not form this turn).
    pub fn house_form(&mut self, p: PlayerId, cities: Vec<CityId>) -> Option<HouseId> {
        let seat = *cities.first()?;
        let origin = self.city_origin(seat)?;
        let tier = self.house_tier_now();
        let candidates = self.house_roll_candidates(p, origin, &cities, tier);
        if candidates.is_empty() {
            return None;
        }
        let name = self.house_new_name(p, origin);
        let offer = self.house_offer(p, name.clone(), origin, cities.clone(), tier, &candidates);
        let mut pick = self.controller(p).choose_house_leader(self, p, &offer);
        if pick >= candidates.len() {
            pick = 0;
        }
        // The controller's call may not touch the game, but a city may be gone in tests of
        // odd controllers: keep the ones still unhoused and of `p`.
        let cities: Vec<CityId> = cities.into_iter().filter(|&c| self.cities.contains(c) && self.cities[c].owner == p && self.cities[c].house.is_none()).collect();
        if cities.is_empty() {
            return None;
        }
        let chosen = candidates[pick].clone();
        let turn = self.game_turn();
        let leaning = Leaning { alignment: chosen.alignment, religion: chosen.religion, sympathy: chosen.sympathy, outsider: chosen.outsider };
        let state = HouseState::of(self.d, self.house_opinion_of(p, &leaning, None).total());
        let id = self.houses.next_id();
        let house = House {
            id,
            owner: p,
            name: name.clone(),
            leader: chosen.leader,
            origin,
            alignment: chosen.alignment,
            religion: chosen.religion,
            sympathy: chosen.sympathy,
            perk: chosen.perk,
            outsider: chosen.outsider,
            formed_turn: turn,
            cities: Vec::new(),
            told_state: state,
            // On unless it is Furious as it forms (rule 9).
            perk_on: state != HouseState::Furious,
        };
        let h = self.houses.insert(house);
        debug_assert_eq!(h, id);
        self.players[p].houses.push(h);
        self.players[p].house_names.push(name);
        for &c in &cities {
            self.house_add_city(h, c, false);
        }
        debug_assert_eq!(self.houses[h].told_state, self.house_state(h));
        let label = self.house_label(h);
        let leader = leader_name(self.d, chosen.leader);
        let names: Vec<String> = cities.iter().map(|&c| self.d.text.name(&self.cities[c].name).to_string()).collect();
        self.house_msg(h, || format!("{label} forms: {leader} leads {}.", and_list(&names)));
        let foreign = Some(origin) != self.player_get_civilization_type(p);
        self.tel_house(p, "formed", if foreign { "foreign" } else { "home" });
        if chosen.outsider {
            self.tel_house(p, "formed", "outsider");
        }
        Some(h)
    }
}
