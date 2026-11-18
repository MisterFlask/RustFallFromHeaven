//! Houses, grants, claims and heirs; the quota's accounting (docs/PERSONS.md, "The first
//! build": Houses and grants, Claims, Heirs, Quota).
//!
//! A city gained is the Crown's (`City::house` `None`) until its owner grants it, for good: to a
//! House, to a person (without a post, or a commander, whose command ends), who founds one, or to
//! a great person unit, which is consumed and draws the founder. Never the capital, never under
//! direct rule. A city leaves its House only by leaving its owner; a House without cities ends.

use super::loyalty::Holder;
use super::*;

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
    fn house_new_color(&self, p: PlayerId) -> u8 {
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
            if house.cities.iter().any(|&o| o != c && self.cities.contains(o) && self.plot_distance(x, y, self.cities[o].x, self.cities[o].y) <= range) {
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
        if self.cities[c].house.is_some() {
            return Err("The city is a House's. A grant is for good.");
        }
        Ok(())
    }

    /// Whether player `p` may grant city `c` to `to` (else why not).
    pub fn player_check_grant(&self, p: PlayerId, c: CityId, to: GrantTo) -> Result<(), &'static str> {
        self.player_check_grant_city(p, c)?;
        match to {
            GrantTo::House(h) => {
                if !self.houses.contains(h) || self.houses[h].owner != p {
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

    /// Person `x` founds a House of their owner with city `c` (granted). Their own loyalty
    /// modifiers move to the House; a commander's command ends.
    fn person_found_house(&mut self, x: PersonId) -> HouseId {
        let p = self.persons[x].owner;
        let turn = self.game_turn();
        let name = self.house_new_name(p);
        let color = self.house_new_color(p);
        self.players[p].house_names.push(name.clone());
        let mut standing = std::mem::take(&mut self.persons[x].standing);
        standing.sworn.clear();
        let h = self.houses.insert_with(|id| House { id, owner: p, name, head: Some(x), seat: None, cities: Vec::new(), standing, founded_turn: turn, color, orders: Vec::new() });
        self.players[p].houses.push(h);
        if let Post::Commander(a) = self.persons[x].post {
            if let Some(army) = self.armies.get_mut(a) {
                if army.commander == Some(x) {
                    army.commander = None;
                }
                if army.commander_named == Some(x) {
                    army.commander_named = None;
                }
            }
        }
        self.persons[x].post = Post::Head(h);
        self.tel_person(p, "posted", "head");
        h
    }

    /// Player `p` grants city `c` to `to`, for good (docs/PERSONS.md): the grantee's `Granted
    /// <city>: +1 (30 turns)`, each other claimant's `<city> granted to <House>` (-1, -2 for the
    /// commander who took it) and the grantee's head as their rival; the demands it meets are met
    /// at once, those it makes impossible are refused.
    pub fn player_grant_city(&mut self, p: PlayerId, c: CityId, to: GrantTo) -> Result<HouseId, &'static str> {
        self.player_check_grant(p, c, to)?;
        let city = self.persons_city_name(c);
        let plot = self.city_plot(c);
        let claimants = self.city_claimants(c);
        let h = match to {
            GrantTo::House(h) => h,
            GrantTo::Person(x) => {
                let h = self.person_found_house(x);
                let (name, house) = (self.persons[x].name.clone(), self.house_label(h));
                self.person_add_deed(x, format!("Founded {house} at {city}."));
                self.person_msg(x, || format!("{name} founds {house} at {city}."));
                h
            }
            GrantTo::GreatPerson(u) => {
                let l = self.persons_draw(p).expect("checked: a person to draw");
                let name = entry::leader_name(self.d, l);
                let ambition = ambition_of_leader(self.d, l);
                let unit_name = self.d.text.name(&self.d.units[self.units[u].unit_type].base.description).to_string();
                self.unit_kill(u, false, None);
                let house_name = self.house_new_name(p);
                let line = format!("{unit_name}, House {house_name} at {city}");
                let x = self.person_new(p, PersonOrigin::GreatPerson, Some(l), None, name, ambition, &line);
                let h = self.person_found_house(x);
                let house = self.house_label(h);
                self.person_add_deed(x, format!("Founded {house} at {city}."));
                h
            }
        };
        // The city joins the House: its trait and maintenance.
        self.cities[c].house = Some(h);
        self.houses[h].cities.push(c);
        if self.houses[h].seat.is_none() {
            self.houses[h].seat = Some(c);
        }
        self.city_house_refresh(c);
        let house = self.house_label(h);
        self.holder_add_mod(Holder::House(h), LoyaltyCause::Granted { city: city.clone() }, GRANTED);
        let head = self.houses[h].head;
        if let Some(x) = head {
            self.person_add_deed(x, format!("Granted {city}."));
        }
        // The snubbed claimants.
        for cl in claimants {
            let (hd, person, amount) = self.claimant_holder(cl);
            if hd == Holder::House(h) || (person.is_some() && person == head) {
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
        if let Some(x) = head {
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
    // A city leaving, a House ending, heirs
    // -----------------------------------------------------------------------------------------

    /// Hook of `city_kill`: city `c` leaves its owner (captured, traded, flipped, razed, released);
    /// it leaves its House, which ends without cities, and takes a new seat when it was the seat.
    pub(crate) fn city_persons_lost(&mut self, c: CityId) {
        let Some(h) = self.cities[c].house.take() else { return };
        if !self.houses.contains(h) {
            return;
        }
        self.houses[h].cities.retain(|&o| o != c);
        if self.houses[h].cities.is_empty() {
            self.house_end(h);
        } else if self.houses[h].seat == Some(c) {
            self.house_reseat(h, Some(c));
        }
    }

    /// House `h` takes a new seat (`gone`: the old seat, still in the slab): its most populous
    /// city, ties the lower id.
    fn house_reseat(&mut self, h: HouseId, gone: Option<CityId>) {
        let seat = self.houses[h].cities.iter().copied().filter(|&o| Some(o) != gone && self.cities.contains(o)).max_by_key(|&o| (self.cities[o].population, std::cmp::Reverse(o)));
        self.houses[h].seat = seat;
    }

    /// House `h` ends (it has no city): its head is a person without a post.
    fn house_end(&mut self, h: HouseId) {
        let label = self.house_label(h);
        let p = self.houses[h].owner;
        self.house_msg(h, || format!("{label} has ended: it holds no city."));
        if let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) {
            self.persons[x].post = Post::None;
            self.persons[x].standing = Standing::default();
            self.person_add_deed(x, format!("{label} ended."));
        }
        self.houses.remove(h);
        self.players[p].houses.retain(|&o| o != h);
        self.tel_person(p, "house", "ended");
    }

    /// The heir candidates of player `p`: its persons without a post, in cast order.
    pub fn player_heir_candidates(&self, p: PlayerId) -> Vec<PersonId> {
        self.players[p].persons.iter().copied().filter(|&x| self.persons.get(x).is_some_and(|person| person.post == Post::None)).collect()
    }

    /// House `h`'s seat was just vacated: an owner with exactly one person without a post has
    /// that one succeed at once; with several the owner names one; with none the seat waits.
    pub(crate) fn house_seat_vacated(&mut self, h: HouseId) {
        let p = self.houses[h].owner;
        let candidates = self.player_heir_candidates(p);
        let label = self.house_label(h);
        match candidates.len() {
            1 => self.house_succeed(h, candidates[0]),
            0 => self.house_msg(h, || format!("{label} has no head. The first person without a post to enter the court succeeds.")),
            _ => self.house_msg(h, || format!("{label} needs an heir.")),
        }
    }

    /// Person `x` succeeds as head of House `h`: their own modifiers move to the House (whose
    /// own stay with it).
    fn house_succeed(&mut self, h: HouseId, x: PersonId) {
        let p = self.houses[h].owner;
        self.houses[h].head = Some(x);
        self.persons[x].post = Post::Head(h);
        let own = std::mem::take(&mut self.persons[x].standing);
        self.houses[h].standing.mods.extend(own.mods);
        self.house_refresh_cities(h);
        let label = self.house_label(h);
        let name = self.persons[x].name.clone();
        self.person_add_deed(x, format!("Succeeded as head of {label}."));
        self.house_msg(h, || format!("{name} succeeds as head of {label}."));
        self.tel_person(p, "posted", "heir");
    }

    /// Player `p` names person `x` heir of House `h`, whose seat is empty.
    pub fn player_name_heir(&mut self, p: PlayerId, h: HouseId, x: PersonId) -> Result<(), &'static str> {
        if !self.houses.contains(h) || self.houses[h].owner != p {
            return Err("Not a House of yours.");
        }
        if self.houses[h].head.is_some() {
            return Err("The seat is not empty.");
        }
        if !self.persons.contains(x) || self.persons[x].owner != p {
            return Err("Not a person of yours.");
        }
        if self.persons[x].post != Post::None {
            return Err("The heir must hold no post.");
        }
        self.house_succeed(h, x);
        Ok(())
    }

    /// Player `p`'s Houses in its turn processing: dead cities dropped, seats taken anew, orders
    /// past the window pruned; an empty seat with exactly one person without a post takes them.
    pub(crate) fn player_persons_houses(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        let window = self.d.defines.person_levy_turns;
        for h in self.players[p].houses.clone() {
            if !self.houses.contains(h) {
                self.players[p].houses.retain(|&o| o != h);
                continue;
            }
            let cities = &self.cities;
            self.houses[h].cities.retain(|&c| cities.contains(c) && cities[c].owner == p && cities[c].house == Some(h));
            if self.houses[h].cities.is_empty() {
                self.house_end(h);
                continue;
            }
            if self.houses[h].seat.map_or(true, |s| !self.houses[h].cities.contains(&s)) {
                self.house_reseat(h, None);
            }
            self.houses[h].orders.retain(|&t| t + window > turn);
            if self.houses[h].head.is_some_and(|x| !self.persons.contains(x)) {
                self.houses[h].head = None;
            }
            if self.houses[h].head.is_none() {
                let candidates = self.player_heir_candidates(p);
                if candidates.len() == 1 {
                    self.house_succeed(h, candidates[0]);
                }
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // The quota: empire orders (`city_check_empire_order`, `city_push_empire_order`) and the
    // requisition (`player_do_requisition`) ask and record it
    // -----------------------------------------------------------------------------------------

    /// `CivicInfo::house_orders` of player `p`'s government civic (a civic without the key: 100).
    pub fn player_house_orders(&self, p: PlayerId) -> i32 {
        let civic = names(self.d).government.and_then(|o| self.players[p].civics[o]);
        match civic.map(|c| self.d.civics[c].house_orders) {
            Some(n) if n > 0 => n,
            _ => 100,
        }
    }

    /// The orders a Loyal House `h` may take in `PERSON_LEVY_TURNS` turns: the larger of 1 and
    /// its cities times the government's `house_orders` / 100.
    pub fn house_orders_total(&self, h: HouseId) -> i32 {
        let p = self.houses[h].owner;
        (self.houses[h].cities.len() as i32 * self.player_house_orders(p) / 100).max(1)
    }

    /// The orders House `h` takes now: `None` no limit (Devoted, an empty seat); Restless and
    /// Disloyal none; Loyal its total less those placed in the window.
    pub fn house_orders_left(&self, h: HouseId) -> Option<i32> {
        if self.houses[h].head.is_none() {
            return None;
        }
        match self.house_state(h) {
            LoyaltyState::Devoted => None,
            LoyaltyState::Restless | LoyaltyState::Disloyal => Some(0),
            LoyaltyState::Loyal => {
                let turn = self.game_turn();
                let window = self.d.defines.person_levy_turns;
                let placed = self.houses[h].orders.iter().filter(|&&t| t + window > turn).count() as i32;
                Some((self.house_orders_total(h) - placed).max(0))
            }
        }
    }

    /// The turns until House `h`'s oldest order in the window frees (`None`: none placed).
    pub fn house_order_frees_in(&self, h: HouseId) -> Option<i32> {
        let turn = self.game_turn();
        let window = self.d.defines.person_levy_turns;
        self.houses[h].orders.iter().filter(|&&t| t + window > turn).min().map(|&t| t + window - turn)
    }

    /// An empire or requisition order was placed in a city of House `h` this turn.
    pub fn house_record_order(&mut self, h: HouseId) {
        let turn = self.game_turn();
        self.houses[h].orders.push(turn);
    }

    /// Whether city `c` takes an empire or requisition order now ([`Game::city_house_takes_order`]
    /// without the reason's text).
    pub fn city_takes_order(&self, c: CityId) -> bool {
        self.city_house(c).map_or(true, |h| self.house_orders_left(h) != Some(0))
    }

    /// Whether city `c` takes an empire or requisition order now (else why not): a Crown city
    /// always; a House's city within its quota.
    pub fn city_house_takes_order(&self, c: CityId) -> Result<(), String> {
        let Some(h) = self.cities[c].house.filter(|&h| self.houses.contains(h)) else { return Ok(()) };
        let label = self.house_label(h);
        match self.house_orders_left(h) {
            None => Ok(()),
            Some(n) if n > 0 => Ok(()),
            Some(_) => match self.house_state(h) {
                state @ (LoyaltyState::Restless | LoyaltyState::Disloyal) => Err(format!("{label} is {}. It takes no orders.", state.name())),
                _ => {
                    let turns = self.house_order_frees_in(h).unwrap_or(0);
                    Err(format!("{label} takes no more orders for {turns} turns."))
                }
            },
        }
    }

    // -----------------------------------------------------------------------------------------
    // The counter on the map
    // -----------------------------------------------------------------------------------------

    /// Whether an army of city `c`'s owner whose commander is not Disloyal stands in it.
    pub fn city_held_by_army(&self, c: CityId) -> bool {
        let p = self.cities[c].owner;
        let plot = self.city_plot(c);
        self.players[p].armies.iter().any(|&a| {
            self.armies.get(a).is_some_and(|army| {
                army.domain == crate::army::ArmyDomain::Land
                    && self.army_plot(a) == Some(plot)
                    && army.commander.filter(|&x| self.persons.contains(x)).map_or(true, |x| self.person_state(x) != LoyaltyState::Disloyal)
            })
        })
    }

    /// House `h`'s cities that would secede now (`going`) and those an army holds (`held`: an army
    /// of the owner stands in it whose commander is not Disloyal), each in the House's order. The
    /// secession of step 2 and the previews share it.
    pub fn house_cities_held(&self, h: HouseId) -> (Vec<CityId>, Vec<CityId>) {
        let (mut going, mut held) = (Vec::new(), Vec::new());
        for &c in &self.houses[h].cities {
            if !self.cities.contains(c) {
                continue;
            }
            if self.city_held_by_army(c) {
                held.push(c);
            } else {
                going.push(c);
            }
        }
        (going, held)
    }
}
