//! Grants and claims (docs/PERSONS.md, "The second build", rule 3).
//!
//! A city gained is the Crown's until its owner grants it, for good: to a House, whose player
//! takes it, or to a person with a leader (a veteran, or a great person unit, which is consumed
//! and draws the founder), who founds a House: a new player born of the liege
//! (`world::birth` with `release` and `vassal`), led by the person, who leaves the map. Never the
//! capital, never under direct rule. Either way the city keeps its buildings and culture, its
//! garrison goes with it, and the liege's units in an army or marching stay the liege's.

use ffh_data::*;

use super::loyalty::Holder;
use super::*;
use crate::ffh::cards::world::{self, Birth};

/// Who claims a city ([`Game::city_claimants`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Claimant {
    /// A House of the city's owner with a city within `PERSON_CLAIM_RANGE` plots.
    House(HouseId),
    /// The commander whose army took the city in the last [`TAKEN_CLAIM_TURNS`] turns.
    Commander(PersonId),
}

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

    /// The claimants to city `c`: each House of its owner with a city within
    /// `PERSON_CLAIM_RANGE` plots (in the owner's House order), then the commander whose army took
    /// it in the last [`TAKEN_CLAIM_TURNS`] turns, if still in the cast.
    pub fn city_claimants(&self, c: CityId) -> Vec<Claimant> {
        let p = self.cities[c].owner;
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        let range = self.d.defines.person_claim_range;
        let mut out: Vec<Claimant> = Vec::new();
        for &h in &self.players[p].houses {
            let Some(house) = self.houses.get(h) else { continue };
            if self.players[house.player].cities.iter().any(|&o| o != c && self.cities.contains(o) && self.plot_distance(x, y, self.cities[o].x, self.cities[o].y) <= range) {
                out.push(Claimant::House(h));
            }
        }
        if let Some((x, turn)) = self.cities[c].taken_by {
            if self.game_turn() - turn <= TAKEN_CLAIM_TURNS && self.persons.get(x).is_some_and(|person| person.owner == p) {
                // A head claims through their House when it is near; else as the one who took it.
                let via_house = self.person_house(x).is_some_and(|h| out.contains(&Claimant::House(h)));
                if !via_house {
                    out.push(Claimant::Commander(x));
                }
            }
        }
        out
    }

    /// The holder a claimant's grievance goes to, and the person who takes the rival.
    pub(crate) fn claimant_holder(&self, cl: Claimant) -> (Holder, Option<PersonId>, (i32, i32)) {
        match cl {
            Claimant::House(h) => (Holder::House(h), self.houses[h].head, SNUB_HOUSE),
            Claimant::Commander(x) => (self.person_holder(x), Some(x), SNUB_COMMANDER),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Grants
    // -----------------------------------------------------------------------------------------

    /// Whether player `p` may grant cities at all (else why not).
    pub fn player_check_grants(&self, p: PlayerId) -> Result<(), &'static str> {
        if !self.player_has_persons(p) {
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

    /// Whether player `p` may grant city `c` to `to` (else why not).
    pub fn player_check_grant(&self, p: PlayerId, c: CityId, to: GrantTo) -> Result<(), &'static str> {
        self.player_check_grant_city(p, c)?;
        match to {
            GrantTo::House(h) => {
                if !self.houses.contains(h) || self.houses[h].owner != p || !self.players[self.houses[h].player].alive {
                    return Err("Not a House of yours.");
                }
            }
            GrantTo::Person(x) => {
                if !self.persons.contains(x) || self.persons[x].owner != p {
                    return Err("Not a person of yours.");
                }
                if self.person_house(x).is_some() {
                    return Err("A head is granted cities through their House.");
                }
                if self.persons[x].leader.is_none() {
                    return Err("A hero cannot head a House.");
                }
            }
            GrantTo::GreatPerson(u) => {
                if !self.units.contains(u) || self.units[u].owner != p || !self.unit_is_great_person(u) {
                    return Err("Not a great person of yours.");
                }
                if self.persons_pool(p).is_empty() {
                    return Err("No person of note is left to found a House.");
                }
            }
        }
        Ok(())
    }

    /// Whether unit `u` is a great person (the five card suits' classes and the Great
    /// Commander's).
    pub fn unit_is_great_person(&self, u: UnitId) -> bool {
        names(self.d).great.contains(&self.unit_get_unit_class_type(u))
    }

    /// What a grant of city `c` does to the units in it (pure, the grant list's preview): the
    /// garrison that goes with it (`going`: [`Game::persons_move_city`]'s units, but `founder`,
    /// the founder's unit, which leaves the map first) and its owner's units that would go but
    /// stay because they are in an army or marching (`staying`).
    pub fn city_grant_garrison(&self, c: CityId, founder: Option<UnitId>) -> (Vec<UnitId>, Vec<UnitId>) {
        let going = world::release_garrison(self, c, founder, true);
        let staying = world::release_garrison(self, c, founder, false).into_iter().filter(|u| !going.contains(u)).collect();
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

    /// Person `x` (with a leader, no House) founds a House of their owner `p` with city `c`: a new
    /// player born of `p` (its civilization, `p`'s techs), led by `x`'s leader, `p`'s vassal,
    /// played by the game's AI, named `House <name>`. `x` leaves the map (their unit is
    /// removed) and heads it; their own loyalty modifiers move to it; a commander's command ends.
    /// Returns the House and the city as it is now (a city changing hands has a new id).
    fn person_found_house(&mut self, x: PersonId, c: CityId) -> Option<(HouseId, CityId)> {
        let p = self.persons[x].owner;
        let leader = self.persons[x].leader?;
        let civ = self.player_get_civilization_type(p)?;
        let plot = self.city_plot(c);
        let settlement = self.cities[c].settlement;
        // The person leaves the map first (their unit's death is not theirs).
        if let Some(u) = self.person_unit(x) {
            self.persons[x].unit = None;
            self.unit_kill(u, false, None);
        }
        if let Post::Commander(a) = self.persons[x].post {
            if let Some(army) = self.armies.get_mut(a) {
                if army.commander == Some(x) {
                    army.commander = None;
                }
                if army.commander_named == Some(x) {
                    army.commander_named = None;
                }
            }
            self.persons[x].post = Post::None;
        }
        let name = self.house_new_name(p);
        self.players[p].house_names.push(name.clone());
        let mut b = Birth::new(civ, leader, p);
        b.description = Some(format!("House {name}"));
        b.release = vec![c];
        b.vassal = true;
        b.keep_armies = true;
        let np = world::birth(self, &b);
        // Played by the game's AI, not the founder's controller.
        let barbarian = self.barbarian_player();
        self.controllers[np] = self.controllers[barbarian].clone();
        // The vassal rules made the record (`team_set_vassal`), headed by `x` (their leader).
        let h = match self.player_house(np) {
            Some(h) => h,
            None => self.house_begin(np, p, HouseOrigin::Founded)?,
        };
        let house = &mut self.houses[h];
        house.origin = HouseOrigin::Founded;
        house.name = name;
        house.standing.mods.retain(|m| m.cause != LoyaltyCause::SoughtProtection);
        let new_c = self.persons_city_at(plot)?;
        // A great city stays great, a settlement stays one (rule 14: the realm's count).
        self.city_keep_settlement(new_c, settlement);
        Some((h, new_c))
    }

    /// Player `p` grants city `c` to `to`, for good (docs/PERSONS.md, rule 3): the grantee's
    /// `Granted <city>: +1 (30 turns)`, each other claimant's `<city> granted to <House>` (-1, -2
    /// for the commander who took it) and the grantee's head as their rival; the demands it meets
    /// are met at once, those it makes impossible are refused. The city moves to the House's
    /// player (a founding: a new one).
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
            GrantTo::Person(x) => {
                let (h, _) = self.person_found_house(x, c).ok_or("The House cannot be founded.")?;
                let (name, house) = (self.persons[x].name.clone(), self.house_label(h));
                self.person_add_deed(x, format!("Founded {house} at {city}."));
                self.person_msg(x, || format!("{name} founds {house} at {city} and leaves the map."));
                h
            }
            GrantTo::GreatPerson(u) => {
                let l = self.persons_draw(p).ok_or("No person of note is left to found a House.")?;
                let name = entry::leader_name(self.d, l);
                let ambition = ambition_of_leader(self.d, l);
                let unit_name = self.d.text.name(&self.d.units[self.units[u].unit_type].base.description).to_string();
                self.unit_kill(u, false, None);
                let house_name = self.house_new_name(p);
                let line = format!("{unit_name}, House {house_name} at {city}");
                let x = self.person_new(p, PersonOrigin::GreatPerson, Some(l), None, name, ambition, &line);
                let (h, _) = self.person_found_house(x, c).ok_or("The House cannot be founded.")?;
                let house = self.house_label(h);
                self.person_add_deed(x, format!("Founded {house} at {city}."));
                h
            }
        };
        let house = self.house_label(h);
        self.holder_add_mod(Holder::House(h), LoyaltyCause::Granted { city: city.clone() }, GRANTED);
        let head = self.houses[h].head;
        if let Some(x) = head {
            self.person_add_deed(x, format!("Granted {city}."));
        }
        // The snubbed claimants.
        for cl in claimants {
            let (hd, person, amount) = self.claimant_holder(cl);
            if hd == Holder::House(h) || (person.is_some() && person == head) || !self.holder_alive(hd) {
                continue;
            }
            self.holder_add_mod(hd, LoyaltyCause::GrantedElsewhere { city: city.clone(), house: house.clone() }, amount);
            if let Some(y) = person.filter(|&y| self.persons.contains(y)) {
                self.persons[y].rival = head;
                self.person_add_deed(y, format!("Passed over: {city} granted to {house}."));
            }
        }
        self.house_msg(h, || format!("{city} granted to {house}."));
        self.tel_person(p, "granted", if matches!(to, GrantTo::House(_)) { "house" } else { "founded" });
        // Demands: those it makes impossible are refused at once; the grantee's met at once.
        for y in self.players[p].persons.clone() {
            let Some(dm) = self.persons.get(y).and_then(|person| person.demand) else { continue };
            if let DemandKind::GrantCity(q) = dm.kind {
                if q == plot && self.person_house(y) != Some(h) {
                    self.person_demand_refused(y);
                }
            }
        }
        if let Some(x) = head.filter(|&x| self.persons.contains(x)) {
            if let Some(dm) = self.persons[x].demand {
                if self.demand_is_met(x, dm.kind) {
                    self.person_demand_met(x);
                }
            }
        }
        // A commander who founded a House: the army takes another commander.
        self.player_persons_commanders(p);
        Ok(h)
    }

    // -----------------------------------------------------------------------------------------
    // The counter on the map
    // -----------------------------------------------------------------------------------------

    /// Whether an army of the liege of city `c`'s owner (a House) stands in it, whose commander
    /// is not Furious: the city goes back to the liege when its House breaks away.
    pub fn city_held_by_army(&self, c: CityId) -> bool {
        let Some(h) = self.city_house(c) else { return false };
        let p = self.houses[h].owner;
        let plot = self.city_plot(c);
        self.players[p].armies.iter().any(|&a| {
            self.armies.get(a).is_some_and(|army| {
                army.domain == crate::army::ArmyDomain::Land
                    && self.army_plot(a) == Some(plot)
                    && army.commander.filter(|&x| self.persons.contains(x)).map_or(true, |x| self.person_state(x) != LoyaltyState::Furious)
            })
        })
    }

    /// House `h`'s cities that would go with it if it broke away now (`going`) and those that
    /// would go back to its liege (`held`: an army of the liege stands in it whose commander is
    /// not Furious), each in its player's order. The break-away and the previews share it.
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
}
