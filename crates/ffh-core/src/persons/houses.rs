//! Grants and claims (docs/PERSONS.md, "The third build", rule 3, on "The second build", rule 3).
//!
//! A city gained is the Crown's until its owner grants it, for good: to a House, whose player
//! takes it, or, while the realm has no House, to a new House founded with it
//! ([`GrantTo::NewHouse`]): a new player born of the liege (`world::birth` with `release` and
//! `vassal`), its leader drawn from the free minor leaders ([`Game::house_leader_draw`]). Never
//! the capital, never under direct rule. Either way the city keeps its buildings and culture, its
//! garrison goes with it, and the liege's units in an army or marching stay the liege's.

use ffh_data::*;

use super::*;
use crate::ffh::cards::world::{self, Birth};

impl Game {
    // -----------------------------------------------------------------------------------------
    // Names and colours
    // -----------------------------------------------------------------------------------------

    /// The name a new House of player `p` takes (without "House"): the first of its
    /// civilization's list, then of the generic list, not used by a House of `p` in this game;
    /// else the number of Houses it founded plus one.
    pub fn house_new_name(&self, p: PlayerId) -> String {
        let d = self.d;
        let civ = self.player_get_civilization_type(p);
        let used = &self.players[p].house_names;
        let lists = [
            d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_some() && d.governor_names[k].civilization == civ),
            d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_none()),
        ];
        for list in lists.into_iter().flatten() {
            if let Some(n) = d.governor_names[list].names.iter().find(|n| !used.contains(n)) {
                return n.clone();
            }
        }
        format!("{}", used.len() + 1)
    }

    /// The colour a new House of player `p` takes: the smallest index no other House of `p` has
    /// (with every colour taken, the number of its Houses modulo [`HOUSE_COLORS`]).
    pub(crate) fn house_new_color(&self, p: PlayerId) -> u8 {
        let taken: Vec<u8> = self.players[p].houses.iter().filter_map(|&h| self.houses.get(h)).map(|h| h.color).collect();
        (0..HOUSE_COLORS).find(|c| !taken.contains(c)).unwrap_or((self.players[p].houses.len() % HOUSE_COLORS as usize) as u8)
    }

    // -----------------------------------------------------------------------------------------
    // Claims
    // -----------------------------------------------------------------------------------------

    /// The Houses that claim city `c`: each House of its owner with a city within
    /// `PERSON_CLAIM_RANGE` plots, in the owner's House order.
    pub fn city_claimants(&self, c: CityId) -> Vec<HouseId> {
        let p = self.cities[c].owner;
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        let range = self.d.defines.person_claim_range;
        self.players[p]
            .houses
            .iter()
            .copied()
            .filter(|&h| {
                self.houses.get(h).is_some_and(|house| {
                    self.players[house.player].cities.iter().any(|&o| o != c && self.cities.contains(o) && self.plot_distance(x, y, self.cities[o].x, self.cities[o].y) <= range)
                })
            })
            .collect()
    }

    // -----------------------------------------------------------------------------------------
    // Grants
    // -----------------------------------------------------------------------------------------

    /// Whether player `p` may grant cities at all (else why not).
    pub fn player_check_grants(&self, p: PlayerId) -> Result<(), &'static str> {
        if !self.player_has_houses(p) {
            return Err("This civilization has no Houses.");
        }
        if self.player_rules_directly(p) {
            return Err("Government: direct rule. No city can be granted.");
        }
        if self.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) {
            return Err("Game option: no vassal states.");
        }
        if self.team_is_a_vassal(self.players[p].team) {
            return Err("A vassal cannot have vassals.");
        }
        Ok(())
    }

    /// Whether player `p` may grant city `c` to anyone (else why not).
    pub fn player_check_grant_city(&self, p: PlayerId, c: CityId) -> Result<(), &'static str> {
        self.player_check_grants(p)?;
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("Not a city of yours.");
        }
        if self.city_is_capital(c) {
            return Err("The capital is the Crown's.");
        }
        if self.cities[c].outpost {
            return Err("An outpost cannot be granted.");
        }
        Ok(())
    }

    /// Whether player `p` may found a House now (else why not): its realm has no House, and a
    /// leader is free ([`Game::house_leader_pool`]).
    pub fn player_check_found_house(&self, p: PlayerId) -> Result<(), &'static str> {
        self.player_check_grants(p)?;
        if !self.player_houses(p).is_empty() {
            return Err("A realm with a House grants to its Houses.");
        }
        let Some(civ) = self.player_get_civilization_type(p) else { return Err("No leader is free to found a House.") };
        if self.house_leader_pool(civ, p).leaders.is_empty() {
            return Err("No leader is free to found a House.");
        }
        Ok(())
    }

    /// Whether player `p` may grant city `c` to `to` (else why not).
    pub fn player_check_grant(&self, p: PlayerId, c: CityId, to: GrantTo) -> Result<(), &'static str> {
        self.player_check_grant_city(p, c)?;
        match to {
            GrantTo::House(h) => {
                if !self.houses.contains(h) || self.houses[h].owner != p || !self.players[self.houses[h].player].alive {
                    return Err("Not a House of yours.");
                }
            }
            GrantTo::NewHouse => self.player_check_found_house(p)?,
        }
        Ok(())
    }

    /// What a grant of city `c` does to the units in it (pure, the grant list's preview): the
    /// garrison that goes with it (`going`: [`Game::persons_move_city`]'s units) and its owner's
    /// units that would go but stay because they are in an army or marching (`staying`).
    pub fn city_grant_garrison(&self, c: CityId) -> (Vec<UnitId>, Vec<UnitId>) {
        let going = world::release_garrison(self, c, None, true);
        let staying = world::release_garrison(self, c, None, false).into_iter().filter(|u| !going.contains(u)).collect();
        (going, staying)
    }

    /// City `c` goes to player `to` as a trade would, with its buildings, culture and garrison:
    /// the land combat units of its owner in it, but those in an army or marching (they stay
    /// their owner's: a vassal's land is friendly to its master). The city it becomes.
    pub(crate) fn persons_move_city(&mut self, c: CityId, to: PlayerId) -> Option<CityId> {
        let settlement = self.cities[c].settlement;
        let moved = world::release_city(self, to, c, None, true)?;
        self.city_keep_settlement(moved, settlement);
        Some(moved)
    }

    /// Player `p` founds a House with city `c`, led by `leader`: a new player born of `p` (its
    /// civilization, `p`'s techs), `p`'s vassal, played by the game's AI, named `House <name>`
    /// (docs/PERSONS.md, "The third build", rule 3). `cities`: the cities it is born with, its
    /// capital first. Returns the House and the cities as they are now (a city changing hands has a
    /// new id).
    pub fn player_found_house(&mut self, p: PlayerId, leader: LeaderHeadType, cities: &[CityId]) -> Option<(HouseId, Vec<CityId>)> {
        let civ = self.player_get_civilization_type(p)?;
        self.player_found_house_as(p, civ, leader, cities, HouseOrigin::Founded)
    }

    /// [`Game::player_found_house`] of civilization `civ` with origin `origin`: a founding
    /// (`Founded`, the liege's civilization) or a split's cadet (`Split`, its parent's civilization;
    /// rule 4: its cities may be a House's, released from it as from the liege). The record is the
    /// vassal rules' (`house_begin`), its origin set here, its starting modifier none.
    pub fn player_found_house_as(&mut self, p: PlayerId, civ: CivilizationType, leader: LeaderHeadType, cities: &[CityId], origin: HouseOrigin) -> Option<(HouseId, Vec<CityId>)> {
        let plots: Vec<(PlotIdx, bool)> = cities.iter().map(|&c| (self.city_plot(c), self.cities[c].settlement)).collect();
        let name = self.house_new_name(p);
        self.players[p].house_names.push(name.clone());
        let mut b = Birth::new(civ, leader, p);
        b.description = Some(format!("House {name}"));
        b.release = cities.to_vec();
        b.vassal = true;
        b.keep_armies = true;
        let np = world::birth(self, &b);
        // Played by the game's AI, not the liege's controller.
        let barbarian = self.barbarian_player();
        self.controllers[np] = self.controllers[barbarian].clone();
        // The vassal rules made the record (`team_set_vassal`).
        let h = match self.player_house(np) {
            Some(h) => h,
            None => self.house_begin(np, p, origin)?,
        };
        let house = &mut self.houses[h];
        house.origin = origin;
        house.name = name;
        house.standing.mods.retain(|m| m.cause != LoyaltyCause::SoughtProtection);
        let state = self.house_state(h);
        self.houses[h].standing.state = state;
        // A great city stays great, a settlement stays one (rule 14: the realm's count).
        let mut now = Vec::new();
        for (plot, settlement) in plots {
            if let Some(c) = self.persons_city_at(plot) {
                self.city_keep_settlement(c, settlement);
                now.push(c);
            }
        }
        Some((h, now))
    }

    /// Player `p` grants city `c` to `to`, for good (docs/PERSONS.md, rule 3): the grantee's
    /// `Granted <city>: +1 (30 turns)`, each other claimant House's `<city> granted to <House>`
    /// (-1); the requests it meets are met at once, those it makes impossible are refused. The
    /// city moves to the House's player (a founding: a new one, its leader drawn now).
    pub fn player_grant_city(&mut self, p: PlayerId, c: CityId, to: GrantTo) -> Result<HouseId, &'static str> {
        self.player_check_grant(p, c, to)?;
        let city = self.persons_city_name(c);
        let plot = self.city_plot(c);
        let claimants = self.city_claimants(c);
        let h = match to {
            GrantTo::House(h) => {
                let hp = self.houses[h].player;
                self.persons_move_city(c, hp).ok_or("The city cannot be granted.")?;
                h
            }
            GrantTo::NewHouse => {
                let civ = self.player_get_civilization_type(p).ok_or("No leader is free to found a House.")?;
                let leader = self.house_leader_draw(civ, p).ok_or("No leader is free to found a House.")?;
                let (h, _) = self.player_found_house(p, leader, &[c]).ok_or("The House cannot be founded.")?;
                let (house, name) = (self.house_label(h), leader_name(self.d, leader));
                self.house_add_deed(h, format!("Founded at {city}, led by {name}."));
                self.house_msg(h, || format!("{house} is founded at {city}, led by {name}."));
                h
            }
        };
        let house = self.house_label(h);
        self.house_add_mod(h, LoyaltyCause::Granted { city: city.clone() }, GRANTED);
        self.house_add_deed(h, format!("Granted {city}."));
        // The snubbed claimants.
        for o in claimants {
            if o == h || !self.houses.contains(o) {
                continue;
            }
            self.house_add_mod(o, LoyaltyCause::GrantedElsewhere { city: city.clone(), house: house.clone() }, SNUB_HOUSE);
            self.house_add_deed(o, format!("Passed over: {city} granted to {house}."));
        }
        self.house_msg(h, || format!("{city} granted to {house}."));
        self.tel_house(p, "granted", if matches!(to, GrantTo::House(_)) { "house" } else { "founded" });
        // Requests: those it makes impossible are refused at once; the grantee's met at once.
        for o in self.player_houses(p) {
            let Some(dm) = self.houses[o].demand else { continue };
            if o != h && dm.kind == DemandKind::GrantCity(plot) {
                self.house_demand_refused(o);
            }
        }
        if let Some(dm) = self.houses[h].demand {
            if self.demand_is_met(h, dm.kind) {
                self.house_demand_met(h);
            }
        }
        Ok(h)
    }

    // -----------------------------------------------------------------------------------------
    // The counter on the map: a contest of numbers (rule 7)
    // -----------------------------------------------------------------------------------------

    /// Whether unit `u` counts for `liege` in the contest for a House's city: a land unit of
    /// `liege` in one of its land armies.
    pub fn unit_holds_for_liege(&self, u: UnitId, liege: PlayerId) -> bool {
        let Some(unit) = self.units.get(u) else { return false };
        if unit.owner != liege || self.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return false;
        }
        unit.army.and_then(|a| self.armies.get(a)).is_some_and(|army| army.domain == crate::army::ArmyDomain::Land)
    }

    /// The contest for city `c` of a House when the House breaks away (rule 7): the liege's army
    /// units standing in it ([`Game::unit_holds_for_liege`]) against the House's own land combat
    /// units in it (those that go with a released city). `None` for a city of no House.
    pub fn city_hold(&self, c: CityId) -> Option<CityHold> {
        let h = self.city_house(c)?;
        let (liege, hp) = (self.houses[h].owner, self.houses[h].player);
        let plot = self.city_plot(c);
        let (mut units, mut armies, mut house) = (0, Vec::new(), 0);
        for &u in &self.map.plots[plot].units {
            if self.unit_holds_for_liege(u, liege) {
                units += 1;
                let a = self.units[u].army;
                if !armies.contains(&a) {
                    armies.push(a);
                }
            } else if self.unit_goes_with_city(u, hp) {
                house += 1;
            }
        }
        Some(CityHold { liege: units, armies: armies.len() as i32, house })
    }

    /// Whether city `c` of a House goes back to its liege when the House breaks away: the liege's
    /// army units in it outnumber the House's own (a tie goes to the House).
    pub fn city_held_by_army(&self, c: CityId) -> bool {
        self.city_hold(c).is_some_and(|hold| hold.held())
    }

    /// House `h`'s cities that would go with it if it broke away now (`going`) and those that
    /// would go back to its liege (`held`: [`Game::city_held_by_army`]), each in its player's
    /// order. The break-away and the previews share it.
    pub fn house_cities_held(&self, h: HouseId) -> (Vec<CityId>, Vec<CityId>) {
        let (mut going, mut held) = (Vec::new(), Vec::new());
        for c in self.house_cities(h) {
            if self.city_held_by_army(c) {
                held.push(c);
            } else {
                going.push(c);
            }
        }
        (going, held)
    }

    /// The contest for city `c` as its liege reads it, when a unit of an army of the liege counts
    /// in it (`None` otherwise): `Corona returns to you: your army's 4 units outnumber its 2.`, or,
    /// not held, `Corona goes with it: its 3 units against your army's 2.` (`house`: how the House
    /// is named; `it` after a line that names it).
    pub fn city_hold_line(&self, c: CityId, house: &str) -> Option<String> {
        let hold = self.city_hold(c)?;
        if hold.liege == 0 {
            return None;
        }
        let city = self.persons_city_name(c);
        let units = |n: i32| format!("{n} {}", if n == 1 { "unit" } else { "units" });
        let yours = if hold.armies > 1 { "your armies'" } else { "your army's" };
        Some(if hold.held() {
            let verb = if hold.liege == 1 { "outnumbers" } else { "outnumber" };
            format!("{city} returns to you: {yours} {} {verb} its {}.", units(hold.liege), hold.house)
        } else {
            format!("{city} goes with {house}: its {} against {yours} {}.", units(hold.house), hold.liege)
        })
    }

    /// House `h`'s contest lines ([`Game::city_hold_line`], the House named `it`), each after a
    /// space, in its player's city order; empty when no army of its liege stands in its cities.
    pub fn house_hold_text(&self, h: HouseId) -> String {
        self.house_cities(h).into_iter().filter_map(|c| self.city_hold_line(c, "it")).map(|l| format!(" {l}")).collect()
    }
}

/// The contest for a House's city when the House breaks away ([`Game::city_hold`]; docs/PERSONS.md,
/// rule 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CityHold {
    /// The liege's army units standing in it (land units of its land armies).
    pub liege: i32,
    /// The liege's armies those units are in.
    pub armies: i32,
    /// The House's own land combat units in it.
    pub house: i32,
}

impl CityHold {
    /// The liege's army units outnumber the House's: the city goes back to the liege.
    pub fn held(&self) -> bool {
        self.liege > self.house
    }
}
