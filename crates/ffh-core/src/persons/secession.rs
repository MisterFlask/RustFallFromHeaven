//! Secession, leagues, the civil war of direct rule, release as a vassal, and the Disloyal
//! head's unit (docs/PERSONS.md, "The first build": Secession, Direct rule, Release).
//!
//! - **A House's secession** ([`Game::house_secede`]), at the end of its countdown, in its
//!   owner's turn processing (Houses before commanders): its cities that no army holds
//!   ([`Game::house_cities_held`]) become a new civilization of the owner's kind
//!   (`world::birth` with `release`: buildings, culture and garrisons go with them), led by its
//!   head (a hero head: a faction leader as the Sundering chooses one), named "<civilization> of
//!   House <name>", its parent the owner, played by the game's AI, at war with its former owner
//!   (`team_declare_war`), `ai_attitude_extra` [`GRUDGE`] both ways. Every other House of the
//!   owner whose countdown runs joins it (a league), unless its head is the rival of a head
//!   already in it, or the reverse; the league is led by the head of the House with the most
//!   cities (ties: the lower id). The held cities are the Crown's. The heads leave the cast: a
//!   head whose House kept a city goes with it, one whose House had every city held is removed
//!   with their unit. Without a leader to be had there is no secession: the head resigns.
//! - **A commander's secession** ([`Game::commander_secede`]): their sworn units, but those
//!   standing in a city of the owner with more of the owner's other land combat units than sworn
//!   units there, (1) join a league formed this turn whose leader is not the commander's rival,
//!   else (2) seize the nearest city of the owner (not the capital) on the commander's land area
//!   whose garrison (the owner's land combat units not sworn) is smaller than they are, which
//!   becomes a new civilization led by the commander, "<civilization> of <city>", else (4) the
//!   commander resigns and the units stay. (3, going over to a rival, is stage 4.)
//! - **Direct rule** ([`Game::player_persons_civil_war`]): a player with Houses that comes under
//!   a direct-rule civic, by any means, loses them all at once as one league, whatever their
//!   states and rivalries; the counter on the map holds.
//! - **Release** ([`Game::player_release_house`]): a House becomes its owner's vassal.
//! - **A Disloyal head's unit** ([`Game::unit_held_by_house`]): from the turn the countdown
//!   begins it marches to the House's largest city (`Unit::march`, docs/ARMIES.md) and is sent
//!   back there whenever it is elsewhere, until the countdown stops.

use std::cmp::Reverse;

use ffh_data::*;

use super::*;
use crate::army::{March, RecruitDest};
use crate::ffh::cards::world::{self, Birth};

/// The attitude modifier between a seceded civilization and its former owner, both ways (FFH's
/// scale, 4 a step; the Sundering's grudge).
pub const GRUDGE: i32 = -4;

/// How a secession came about (its telemetry and its message).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Why {
    /// A countdown's end.
    Countdown,
    /// A direct-rule civic adopted.
    CivilWar,
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // Shared steps
    // -----------------------------------------------------------------------------------------

    /// Whether unit `u` of player `p` on a released city's plot goes with the city
    /// (`world::birth`'s release: the owner's land units that can fight, not heroes).
    fn unit_goes_with_city(&self, u: UnitId, p: PlayerId) -> bool {
        self.units.contains(u)
            && self.units[u].owner == p
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && self.d.units[self.units[u].unit_type].combat > 0
            && !self.unit_is_hero(u)
    }

    /// The persons of `p` whose units leave with the cities at `plots` or with `units`: they
    /// leave the cast first, so that the units' change of owner is not told as a death.
    fn persons_leave_with(&mut self, p: PlayerId, plots: &[PlotIdx], units: &[UnitId], to: &str) {
        for x in self.players[p].persons.clone() {
            let Some(u) = self.person_unit(x) else { continue };
            let in_city = self.unit_plot_opt(u).is_some_and(|plot| plots.contains(&plot)) && self.unit_goes_with_city(u, p);
            if in_city || units.contains(&u) {
                let name = self.persons[x].name.clone();
                self.person_add_deed(x, format!("Went over to {to}."));
                self.person_msg(x, || format!("{name} goes over to {to}."));
                self.persons_depart(x);
            }
        }
    }

    /// Person `x` leaves the cast without a word, their post empty (no heir is asked for: their
    /// House, if any, is gone or ends at once).
    fn persons_depart(&mut self, x: PersonId) {
        if !self.persons.contains(x) {
            return;
        }
        if let Post::Head(h) = self.persons[x].post {
            if self.houses.contains(h) && self.houses[h].head == Some(x) {
                self.houses[h].head = None;
            }
            self.persons[x].post = Post::None;
        }
        self.person_leave(x);
    }

    /// House `h` is gone (seceded, released or dissolved): its cities left in its owner's hands
    /// are the Crown's again, refreshed.
    fn house_remove(&mut self, h: HouseId) {
        let p = self.houses[h].owner;
        let cities = self.houses[h].cities.clone();
        self.houses.remove(h);
        self.players[p].houses.retain(|&o| o != h);
        for c in cities {
            if self.cities.contains(c) && self.cities[c].house == Some(h) {
                self.cities[c].house = None;
                self.city_house_refresh(c);
            }
        }
    }

    /// A new civilization born of player `p` from `cities` (its capital first), led by `leader`,
    /// named `description`: played by the game's AI, met with everyone `p` has met, at war with
    /// `p` (`at_war`) or `p`'s vassal (`vassal`), [`GRUDGE`] both ways unless a vassal. Returns it.
    fn persons_birth(&mut self, p: PlayerId, leader: LeaderHeadType, description: String, cities: Vec<CityId>, vassal: bool) -> PlayerId {
        let civ = self.player_get_civilization_type(p).expect("a civilization");
        let mut b = if vassal { Birth::new(civ, leader, p) } else { Birth::wild(civ, leader) };
        b.parent = Some(p);
        b.description = Some(description);
        b.release = cities;
        b.vassal = vassal;
        if !vassal {
            b.parent_attitude = GRUDGE;
        }
        let np = world::birth(self, &b);
        // Played by the game's AI, not the founder's controller.
        let barbarian = self.barbarian_player();
        self.controllers[np] = self.controllers[barbarian].clone();
        if !vassal {
            self.players[p].ai_attitude_extra[np] += GRUDGE;
        }
        let (pt, nt) = (self.players[p].team, self.players[np].team);
        for t in self.civ_team_ids() {
            if t != nt && t != pt && self.teams[pt].has_met[t] && !self.teams[nt].has_met[t] {
                self.team_meet(nt, t, true);
            }
        }
        if self.teams[nt].barbarian_ally {
            let barbarians = self.players[barbarian].team;
            self.teams[nt].at_war[barbarians] = false;
            self.teams[barbarians].at_war[nt] = false;
        }
        if !vassal {
            self.team_declare_war(nt, pt, false, None);
        }
        np
    }

    /// The leader of a new civilization led by person `x` (`None`: a faction leader of `p`'s
    /// civilization, as the Sundering chooses one; `None` again when none is left).
    fn persons_new_leader(&mut self, p: PlayerId, x: Option<PersonId>) -> Option<LeaderHeadType> {
        if let Some(l) = x.and_then(|x| self.persons.get(x)).and_then(|person| person.leader) {
            return Some(l);
        }
        let civ = self.player_get_civilization_type(p)?;
        crate::ffh::cards::sundering::choose_faction_leader(self, civ, &[])
    }

    /// Unit `u` (still `p`'s) goes to player `np`'s capital as its unit.
    fn persons_unit_over(&mut self, u: UnitId, p: PlayerId, np: PlayerId) {
        if !self.units.contains(u) || self.units[u].owner != p {
            return;
        }
        let Some(cap) = self.player_get_capital_city(np) else { return };
        let at = (self.cities[cap].x, self.cities[cap].y);
        self.crusade_change_owner(u, np, Some(at));
    }

    fn city_names(&self, cities: &[CityId]) -> String {
        and_list(&cities.iter().map(|&c| self.persons_city_name(c)).collect::<Vec<_>>())
    }

    // -----------------------------------------------------------------------------------------
    // A House's secession
    // -----------------------------------------------------------------------------------------

    /// The Houses that secede with House `h` now (`h` first): every other House of its owner whose
    /// countdown runs, unless its head is the rival of a head already in, or the reverse; with
    /// `all` (the civil war) every House.
    pub fn house_league(&self, h: HouseId, all: bool) -> Vec<HouseId> {
        let p = self.houses[h].owner;
        let mut out = vec![h];
        for &o in &self.players[p].houses {
            if o == h || !self.houses.contains(o) {
                continue;
            }
            if all {
                out.push(o);
                continue;
            }
            if self.houses[o].standing.countdown.is_none() {
                continue;
            }
            let head = self.houses[o].head.filter(|&x| self.persons.contains(x));
            let rivals = out.iter().filter_map(|&m| self.houses[m].head.filter(|&y| self.persons.contains(y))).any(|y| {
                head.is_some_and(|x| self.persons[x].rival == Some(y) || self.persons[y].rival == Some(x))
            });
            if !rivals {
                out.push(o);
            }
        }
        out
    }

    /// House `h` secedes (with its league; `why`). Returns the new civilization and the league's
    /// leading head, if a civilization was born.
    fn house_secede(&mut self, h: HouseId, why: Why) -> Option<(PlayerId, Option<PersonId>)> {
        let p = self.houses[h].owner;
        let members = self.house_league(h, why == Why::CivilWar);
        // The league's leader: the head of the House with the most cities (ties: the lower id).
        let lead = members
            .iter()
            .copied()
            .filter(|&m| self.houses[m].head.is_some_and(|x| self.persons.contains(x)))
            .max_by_key(|&m| (self.houses[m].cities.len(), Reverse(m)))
            .unwrap_or(h);
        let lead_head = self.houses[lead].head.filter(|&x| self.persons.contains(x));
        let mut going: Vec<CityId> = Vec::new();
        let mut held: Vec<CityId> = Vec::new();
        let mut kept: Vec<HouseId> = Vec::new();
        for &m in &members {
            let (g, hd) = self.house_cities_held(m);
            if !g.is_empty() {
                kept.push(m);
            }
            going.extend(g);
            held.extend(hd);
        }
        // Its capital: the leading House's seat when it goes, else the most populous city going.
        let seat = self.houses[lead].seat.filter(|c| going.contains(c));
        let capital = seat.or_else(|| going.iter().copied().max_by_key(|&c| (self.cities[c].population, Reverse(c))));
        if let Some(cap) = capital {
            going.retain(|&c| c != cap);
            going.insert(0, cap);
        }
        let leader = if going.is_empty() { None } else { self.persons_new_leader(p, lead_head) };
        let labels: Vec<String> = members.iter().map(|&m| self.house_label(m)).collect();
        let civ_name = self.player_get_civilization_short_description(p);
        let turn = self.game_turn();
        let to = self.audience_met(self.players[p].team);
        let tel = match (why, members.len()) {
            (Why::CivilWar, _) => "civil_war",
            (_, 1) => "house",
            _ => "league",
        };

        // Commanders whose armies held a city: a deed.
        for &c in &held {
            let plot = self.city_plot(c);
            let city = self.persons_city_name(c);
            for &a in &self.players[p].armies.clone() {
                if let Some(x) = self.armies.get(a).filter(|_| self.army_plot(a) == Some(plot)).and_then(|army| army.commander).filter(|&x| self.persons.contains(x)) {
                    self.person_add_deed(x, format!("Held {city} at the secession of {}.", and_list(&labels)));
                }
            }
        }

        let Some(leader) = leader else {
            if going.is_empty() {
                // Every city held: the Houses end, their heads leave the cast with their units.
                for &m in &members {
                    let label = self.house_label(m);
                    self.house_msg(m, || format!("{label} ends: every city is held."));
                    self.house_dissolve(m, true);
                }
                self.tel_person(p, "seceded", "held");
                return None;
            }
            // No leader to be had: no secession.
            if why == Why::CivilWar {
                for &m in &members {
                    let label = self.house_label(m);
                    self.house_msg(m, || format!("{label} is dissolved: no leader can be found."));
                    self.house_dissolve(m, false);
                }
            } else {
                self.houses[h].standing.countdown = None;
                if let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) {
                    let (name, label) = (self.persons[x].name.clone(), self.house_label(h));
                    self.house_msg(h, || format!("{name} resigns as head of {label}: no leader can be found."));
                    self.persons_depart(x);
                    self.house_refresh_cities(h);
                    self.house_seat_vacated(h);
                }
            }
            self.tel_person(p, "seceded", "no_leader");
            return None;
        };

        let names = self.city_names(&going);
        let lead_label = self.house_label(lead);
        let description = format!("{civ_name} of {lead_label}");
        // The heads: those whose House keeps a city go with it; the others are removed.
        let mut heads_over: Vec<UnitId> = Vec::new();
        for &m in &members {
            let Some(x) = self.houses[m].head.filter(|&x| self.persons.contains(x)) else { continue };
            let unit = self.person_unit(x);
            if kept.contains(&m) {
                heads_over.extend(unit);
                self.person_add_deed(x, format!("Seceded with {}.", self.house_label(m)));
                self.persons_depart(x);
            } else {
                let label = self.house_label(m);
                self.house_msg(m, || format!("{label} ends: every city is held."));
                self.persons_depart(x);
                if let Some(u) = unit.filter(|&u| self.units.contains(u)) {
                    self.unit_kill(u, false, None);
                }
            }
        }
        let plots: Vec<PlotIdx> = going.iter().map(|&c| self.city_plot(c)).collect();
        self.persons_leave_with(p, &plots, &[], &description);
        for &m in &members {
            self.house_remove(m);
        }
        let np = self.persons_birth(p, leader, description, going, false);
        for u in heads_over {
            self.persons_unit_over(u, p, np);
        }
        let text = match (why, labels.len()) {
            (Why::CivilWar, _) => format!("Civil war: {} secede from the {civ_name} with {names}.", and_list(&labels)),
            (_, 1) => format!("{} secedes from the {civ_name} with {names}.", labels[0]),
            _ => format!("{} secede from the {civ_name} with {names}.", and_list(&labels)),
        };
        self.log.msg_to(turn, to, || text);
        self.tel_person(p, "seceded", tel);
        Some((np, lead_head))
    }

    /// House `h` ends without a secession: its cities are the Crown's, its head leaves the cast
    /// (with `kill`, their unit is removed).
    fn house_dissolve(&mut self, h: HouseId, kill: bool) {
        if let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) {
            let unit = self.person_unit(x);
            self.persons_depart(x);
            if kill {
                if let Some(u) = unit.filter(|&u| self.units.contains(u)) {
                    self.unit_kill(u, false, None);
                }
            }
        }
        self.house_remove(h);
    }

    // -----------------------------------------------------------------------------------------
    // A commander's secession
    // -----------------------------------------------------------------------------------------

    /// Commander `x`'s sworn units that go at the countdown's end and those that stay (standing
    /// in a city of the owner with more of the owner's other land combat units than sworn units
    /// there).
    pub fn commander_sworn_going(&self, x: PersonId) -> (Vec<UnitId>, Vec<UnitId>) {
        let p = self.persons[x].owner;
        let sworn: Vec<UnitId> = self.persons[x].standing.sworn.iter().copied().filter(|&u| self.units.contains(u) && self.units[u].owner == p).collect();
        let (mut going, mut staying) = (Vec::new(), Vec::new());
        for &u in &sworn {
            let Some(plot) = self.unit_plot_opt(u) else {
                going.push(u);
                continue;
            };
            let ours = self.persons_city_at(plot).is_some_and(|c| self.cities[c].owner == p);
            if ours {
                let units = &self.map.plots[plot].units;
                let sworn_here = units.iter().filter(|o| sworn.contains(o)).count();
                let others = units.iter().filter(|&&o| !sworn.contains(&o) && self.unit_goes_with_city(o, p)).count();
                if others > sworn_here {
                    staying.push(u);
                    continue;
                }
            }
            going.push(u);
        }
        (going, staying)
    }

    /// The city commander `x`'s going units would seize: the nearest city of the owner (not the
    /// capital) on the commander's land area whose garrison (the owner's land combat units not
    /// sworn) is smaller than they are (ties: the lower id).
    pub fn commander_seize_target(&self, x: PersonId, going: &[UnitId]) -> Option<CityId> {
        let p = self.persons[x].owner;
        let from = self.person_unit(x).filter(|u| going.contains(u)).or_else(|| going.first().copied()).and_then(|u| self.unit_plot_opt(u))?;
        let area = self.map.plots[from].area;
        let (fx, fy) = self.map.xy(from);
        let sworn = &self.persons[x].standing.sworn;
        self.players[p]
            .cities
            .iter()
            .copied()
            .filter(|&c| {
                let plot = self.city_plot(c);
                !self.city_is_capital(c)
                    && self.map.plots[plot].area == area
                    && self.map.plots[plot].units.iter().filter(|&&o| !sworn.contains(&o) && self.unit_goes_with_city(o, p)).count() < going.len()
            })
            .min_by_key(|&c| (self.plot_distance(fx, fy, self.cities[c].x, self.cities[c].y), c))
    }

    /// Commander `x`'s countdown ends (`leagues`: the civilizations born of a House's secession
    /// this turn, with their leading heads).
    fn commander_secede(&mut self, x: PersonId, leagues: &[(PlayerId, Option<PersonId>)]) {
        let p = self.persons[x].owner;
        let name = self.persons[x].name.clone();
        let subject = self.person_subject(x);
        let civ_name = self.player_get_civilization_short_description(p);
        let turn = self.game_turn();
        let to = self.audience_met(self.players[p].team);
        let (going, _) = self.commander_sworn_going(x);
        // 1. A league formed this turn whose leader is not their rival.
        let rival = self.persons[x].rival;
        let league = leagues.iter().find(|&&(np, head)| self.players[np].alive && self.player_get_capital_city(np).is_some() && (head.is_none() || head != rival)).map(|&(np, _)| np);
        if let (Some(np), false) = (league, going.is_empty()) {
            let over = self.player_get_civilization_description(np);
            let n = going.len();
            self.persons_leave_with(p, &[], &going, &over);
            self.persons_depart(x);
            for &u in &going {
                self.persons_unit_over(u, p, np);
            }
            self.log.msg_to(turn, to, || format!("{subject} goes over to {over} with {n} units."));
            self.tel_person(p, "seceded", "commander_league");
            return;
        }
        // 2. A city seized.
        if let Some(c) = self.commander_seize_target(x, &going) {
            if let Some(leader) = self.persons_new_leader(p, Some(x)) {
                let city = self.persons_city_name(c);
                let n = going.len();
                let description = format!("{civ_name} of {city}");
                let plot = self.city_plot(c);
                self.persons_leave_with(p, &[plot], &going, &description);
                self.persons_depart(x);
                let np = self.persons_birth(p, leader, description, vec![c], false);
                for &u in &going {
                    self.persons_unit_over(u, p, np);
                }
                self.log.msg_to(turn, to, || format!("{subject} seizes {city} and secedes from the {civ_name} with {n} units."));
                self.tel_person(p, "seceded", "commander_city");
                return;
            }
        }
        // 4. They resign; the units stay.
        self.person_add_deed(x, "Resigned.".to_string());
        self.person_msg(x, || format!("{name} resigns the command and leaves the court."));
        self.persons_depart(x);
        self.tel_person(p, "seceded", "commander_resigned");
    }

    // -----------------------------------------------------------------------------------------
    // The turn, the civil war, release
    // -----------------------------------------------------------------------------------------

    /// Player `p`'s countdowns that ran out (its turn processing): Houses first, then
    /// commanders.
    pub(crate) fn player_persons_secessions(&mut self, p: PlayerId) {
        let due = self.persons_countdowns_due(p);
        if due.is_empty() {
            return;
        }
        let mut leagues: Vec<(PlayerId, Option<PersonId>)> = Vec::new();
        for &d in &due {
            if let CountdownDue::House(h) = d {
                // (A House already gone with an earlier league, or whose countdown stopped.)
                if self.houses.get(h).is_some_and(|house| house.standing.countdown.is_some()) {
                    if let Some(league) = self.house_secede(h, Why::Countdown) {
                        leagues.push(league);
                    }
                }
            }
        }
        for &d in &due {
            if let CountdownDue::Commander(x) = d {
                if self.persons.get(x).is_some_and(|person| person.owner == p && matches!(person.post, Post::Commander(_)) && person.standing.countdown.is_some()) {
                    self.commander_secede(x, &leagues);
                }
            }
        }
    }

    /// Player `p` under direct rule with Houses: every House secedes at once as one league
    /// (docs/PERSONS.md, "Direct rule"). Called where the government's change is noticed
    /// (`player_governance_sync`).
    pub fn player_persons_civil_war(&mut self, p: PlayerId) {
        if self.players[p].houses.is_empty() || !self.player_rules_directly(p) {
            return;
        }
        let first = self.players[p].houses.iter().copied().find(|&h| self.houses.contains(h));
        if let Some(h) = first {
            self.house_secede(h, Why::CivilWar);
        }
        // Whatever could not secede is gone too: no House stands under direct rule.
        for h in self.players[p].houses.clone() {
            if self.houses.contains(h) {
                self.house_dissolve(h, false);
            }
        }
        self.players[p].houses.clear();
    }

    /// Whether player `p` may release House `h` as its vassal (else why not).
    pub fn player_check_release_house(&self, p: PlayerId, h: HouseId) -> Result<(), &'static str> {
        if !self.houses.contains(h) || self.houses[h].owner != p {
            return Err("Not a House of yours.");
        }
        if self.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) {
            return Err("Game option: no vassal states.");
        }
        if self.team_is_a_vassal(self.players[p].team) {
            return Err("A vassal cannot have vassals.");
        }
        let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) else {
            return Err("The seat is empty.");
        };
        if self.house_state(h) == LoyaltyState::Disloyal {
            return Err("A Disloyal House cannot be released.");
        }
        if self.persons[x].leader.is_none() {
            let civ = self.player_get_civilization_type(p).ok_or("This civilization has no Houses.")?;
            if crate::ffh::cards::sundering::faction_leader(self, civ, &[]).is_none() {
                return Err("No leader can be found.");
            }
        }
        Ok(())
    }

    /// Player `p` releases House `h`: its cities become `p`'s vassal civilization, led by its
    /// head, who leaves the cast (docs/PERSONS.md, "Release"). Returns the vassal.
    pub fn player_release_house(&mut self, p: PlayerId, h: HouseId) -> Result<PlayerId, &'static str> {
        self.player_check_release_house(p, h)?;
        let x = self.houses[h].head.expect("checked: a head");
        let leader = self.persons_new_leader(p, Some(x)).ok_or("No leader can be found.")?;
        let mut cities: Vec<CityId> = self.houses[h].cities.iter().copied().filter(|&c| self.cities.contains(c)).collect();
        if let Some(seat) = self.houses[h].seat.filter(|c| cities.contains(c)) {
            cities.retain(|&c| c != seat);
            cities.insert(0, seat);
        }
        let label = self.house_label(h);
        let civ_name = self.player_get_civilization_short_description(p);
        let names = self.city_names(&cities);
        let description = format!("{civ_name} of {label}");
        let unit = self.person_unit(x);
        let plots: Vec<PlotIdx> = cities.iter().map(|&c| self.city_plot(c)).collect();
        self.persons_depart(x);
        self.persons_leave_with(p, &plots, &[], &description);
        self.house_remove(h);
        let np = self.persons_birth(p, leader, description, cities, true);
        if let Some(u) = unit {
            self.persons_unit_over(u, p, np);
        }
        let turn = self.game_turn();
        let to = self.audience_met(self.players[p].team);
        self.log.msg_to(turn, to, || format!("{label} is released from the {civ_name} as a vassal with {names}."));
        self.tel_person(p, "seceded", "released");
        Ok(np)
    }

    // -----------------------------------------------------------------------------------------
    // A Disloyal head's unit
    // -----------------------------------------------------------------------------------------

    /// The House whose countdown holds unit `u`: the unit of a head whose House's Disloyal
    /// countdown runs (it goes to the House's largest city and takes no orders).
    pub fn unit_held_by_house(&self, u: UnitId) -> Option<HouseId> {
        let owner = self.units.get(u)?.owner;
        if self.players[owner].houses.is_empty() {
            return None;
        }
        let x = self.unit_person(u)?;
        let h = self.person_house(x)?;
        self.houses[h].standing.countdown.is_some().then_some(h)
    }

    /// Why unit `u` takes no orders, if a House holds it: `House Arne is Disloyal. Its head's unit
    /// takes no orders.`
    pub fn unit_house_refusal(&self, u: UnitId) -> Option<String> {
        let h = self.unit_held_by_house(u)?;
        Some(format!("{} is Disloyal. Its head's unit takes no orders.", self.house_label(h)))
    }

    /// House `h`'s largest city (ties: the lower id).
    fn house_largest_city(&self, h: HouseId) -> Option<CityId> {
        self.houses[h].cities.iter().copied().filter(|&c| self.cities.contains(c)).max_by_key(|&c| (self.cities[c].population, Reverse(c)))
    }

    /// Player `p`'s Disloyal heads' units go to their Houses' largest cities (its turn
    /// processing).
    pub(crate) fn player_persons_held_heads(&mut self, p: PlayerId) {
        for h in self.players[p].houses.clone() {
            if !self.houses.contains(h) || self.houses[h].standing.countdown.is_none() {
                continue;
            }
            let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) else { continue };
            let Some(u) = self.person_unit(x) else { continue };
            let Some(c) = self.house_largest_city(h) else { continue };
            let Some(here) = self.unit_plot_opt(u) else { continue };
            if here == self.city_plot(c) || self.units[u].march.is_some() || self.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
                continue;
            }
            if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
                self.group_clear_mission_queue(grp);
            }
            let turn = self.game_turn();
            self.units[u].army = None;
            self.units[u].march = Some(March { dest: RecruitDest::Garrison(c), stalled: 0, left_turn: turn, from_city: c, diverted: false });
            let (name, city, label) = (self.persons[x].name.clone(), self.persons_city_name(c), self.house_label(h));
            self.person_msg(x, || format!("{name} goes to {city}: {label} is Disloyal. The unit takes no orders."));
        }
    }
}
