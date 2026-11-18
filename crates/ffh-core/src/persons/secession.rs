//! Breaking away, a commander's secession, the civil war of direct rule (docs/PERSONS.md, "The
//! second build", rules 7 and 11).
//!
//! - **A House breaks away** ([`Game::house_break_away`]) at the end of its Furious countdown, in
//!   its liege's turn processing (Houses before commanders), when its liege comes under a
//!   direct-rule civic (every House at once: the civil war), or when it accepts an offer to secede
//!   (`bribes.rs`): first every city of the House in which an army of the liege stands goes back
//!   to the liege (as a trade: its buildings and garrison; `Censura returns to you: an army of
//!   yours stands in it.`), then the vassalage ends (the record ends, its head leaves the cast: they lead the
//!   civilization), and the House's team declares war on the liege's team, [`GRUDGE`] both ways.
//!   Everyone who has met the liege is told. Houses breaking away the same turn are not merged.
//! - **A commander's secession** ([`Game::commander_secede`]): their sworn units, but those
//!   standing in a city of the owner with more of the owner's other land combat units than sworn
//!   units there, (1) join a House that broke away this turn whose head is not the commander's
//!   rival (they become its player's units in its capital), else (2) seize the nearest city of the
//!   owner (not the capital) on the commander's land area whose garrison (the owner's land combat
//!   units not sworn) is smaller than they are, which becomes a new civilization led by the
//!   commander, "<civilization> of <city>", else (4) the commander resigns and the units stay.
//!   (3, going over to a rival, is stage 4.)
//! - **Direct rule** ([`Game::player_persons_civil_war`]): a player with Houses that comes under
//!   a direct-rule civic, by any means, has every House break away at once; the counter on the
//!   map holds.

use ffh_data::*;

use super::*;
use crate::ffh::cards::world::{self, Birth};
use crate::log::Audience;

/// The attitude modifier between a civilization that broke away or seceded and its former liege,
/// both ways (FFH's scale, 4 a step; the Sundering's grudge).
pub const GRUDGE: i32 = -4;

/// Why a House breaks away (its telemetry and its message).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Why {
    /// A Furious countdown's end.
    Countdown,
    /// A direct-rule civic adopted.
    CivilWar,
    /// An offer accepted: Secede (`None`) or Become your vassal (the buyer).
    Bribe { buyer: PlayerId, vassal: bool },
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
            let in_city = self.unit_plot_opt(u).is_some_and(|plot| plots.contains(&plot)) && self.unit_goes_with_city(u, p) && self.units[u].army.is_none() && self.units[u].march.is_none();
            if in_city || units.contains(&u) {
                let name = self.persons[x].name.clone();
                self.person_add_deed(x, format!("Went over to {to}."));
                self.person_msg(x, || format!("{name} goes over to {to}."));
                self.person_leave(x);
            }
        }
    }

    /// A new civilization born of player `p` from `cities` (its capital first), led by `leader`,
    /// named `description`: played by the game's AI, met with everyone `p` has met, at war with
    /// `p`, [`GRUDGE`] both ways. Returns it.
    fn persons_birth(&mut self, p: PlayerId, leader: LeaderHeadType, description: String, cities: Vec<CityId>) -> PlayerId {
        let civ = self.player_get_civilization_type(p).expect("a civilization");
        let mut b = Birth::wild(civ, leader);
        b.parent = Some(p);
        b.description = Some(description);
        b.release = cities;
        b.parent_attitude = GRUDGE;
        let np = world::birth(self, &b);
        // Played by the game's AI, not the founder's controller.
        let barbarian = self.barbarian_player();
        self.controllers[np] = self.controllers[barbarian].clone();
        self.players[p].ai_attitude_extra[np] += GRUDGE;
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
        self.team_declare_war(nt, pt, false, None);
        np
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
    // A House breaking away
    // -----------------------------------------------------------------------------------------

    /// House `h` breaks away (`why`): its cities an army of its liege holds go back to the liege,
    /// the vassalage ends, and it declares war on its liege (with `Bribe { vassal: true }`, it
    /// then becomes the buyer's vassal). Returns its player, alive.
    pub(crate) fn house_break_away(&mut self, h: HouseId, why: Why) -> Option<PlayerId> {
        let p = self.houses[h].owner;
        let hp = self.houses[h].player;
        let label = self.house_label(h);
        let head = self.houses[h].head.filter(|&x| self.persons.contains(x));
        let civ_name = self.player_get_civilization_short_description(p);
        let (going, held) = self.house_cities_held(h);
        let turn = self.game_turn();
        // The counter on the map: the held cities go back to the liege, as a trade.
        for &c in &held {
            let plot = self.city_plot(c);
            let city = self.persons_city_name(c);
            for a in self.players[p].armies.clone() {
                if let Some(x) = self.armies.get(a).filter(|_| self.army_plot(a) == Some(plot)).and_then(|army| army.commander).filter(|&x| self.persons.contains(x)) {
                    self.person_add_deed(x, format!("Held {city} when {label} broke away."));
                }
            }
            self.persons_move_city(c, p);
            let at = self.map.xy(plot);
            self.log.msg_marked(turn, Some(p), at, LogMark::Person, LogTag::None, || format!("{city} returns to you: an army of yours stands in it."));
        }
        if let Some(x) = head {
            self.person_add_deed(x, format!("Broke away from the {civ_name} with {label}."));
        }
        // The vassalage ends: the record ends, the head leaves the cast.
        let (ht, pt) = (self.players[hp].team, self.players[p].team);
        self.team_end_vassalage(ht, pt);
        if self.houses.contains(h) {
            self.house_record_end(h);
        }
        self.player_verify_alive(hp);
        let alive = self.players[hp].alive;
        if alive {
            self.players[hp].ai_attitude_extra[p] += GRUDGE;
            self.players[p].ai_attitude_extra[hp] += GRUDGE;
            self.team_declare_war(ht, pt, false, None);
        }
        let names = self.city_names(&going.iter().copied().filter(|&c| self.cities.contains(c)).collect::<Vec<_>>());
        let at = self.player_get_capital_city(hp).map(|c| (self.cities[c].x, self.cities[c].y));
        // The news names the cities a House takes after "taking" (after "with", a name reads as a
        // person's); the buyer of a bribe reads `you`.
        let news = |buyer: Option<&str>| match (why, alive) {
            (_, false) => format!("{label} ends: every city of it stands held by an army of the {civ_name}."),
            (Why::CivilWar, _) => format!("Civil war: {label} breaks away from the {civ_name}, taking {names}: war."),
            (Why::Countdown, _) => format!("{label} breaks away from the {civ_name}, taking {names}: war."),
            (Why::Bribe { vassal: false, .. }, _) => format!("{label} breaks away from the {civ_name}, taking {names}. Bought by {}: war.", buyer.unwrap_or("you")),
            (Why::Bribe { vassal: true, .. }, _) => match buyer {
                Some(b) => format!("{label} breaks away from the {civ_name}, taking {names}, as a vassal of {b}: war."),
                None => format!("{label} breaks away from the {civ_name}, taking {names}, as your vassal: war."),
            },
        };
        // The buyer's own message is in the second person; everyone else who met the liege reads
        // the buyer's name.
        let buyer = match why {
            Why::Bribe { buyer, .. } if alive => Some(buyer),
            _ => None,
        };
        let buyer_name = buyer.map(|b| format!("the {}", self.player_get_civilization_short_description(b)));
        let (others, own) = match (self.audience_met(pt), buyer) {
            (Audience::Players(list), Some(b)) => {
                let own = list.contains(&b).then(|| Audience::Players(vec![b]));
                (Audience::Players(list.into_iter().filter(|&q| q != b).collect()), own)
            }
            (to, _) => (to, None),
        };
        let text = news(buyer_name.as_deref());
        let own_text = own.as_ref().map(|_| news(None));
        match at {
            Some(at) => self.log.msg_to_at(turn, others, at, LogMark::Person, || text),
            None => self.log.msg_to(turn, others, || text),
        }
        if let (Some(own), Some(own_text)) = (own, own_text) {
            match at {
                Some(at) => self.log.msg_to_at(turn, own, at, LogMark::Person, || own_text),
                None => self.log.msg_to(turn, own, || own_text),
            }
        }
        let tel = match why {
            Why::CivilWar => "civil_war",
            Why::Countdown => "countdown",
            Why::Bribe { vassal: false, .. } => "bribe",
            Why::Bribe { vassal: true, .. } => "bribe_vassal",
        };
        self.tel_person(p, "seceded", if alive { tel } else { "held" });
        alive.then_some(hp)
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

    /// Commander `x`'s countdown ends (`broke`: the Houses that broke away this turn, their
    /// players and their heads).
    fn commander_secede(&mut self, x: PersonId, broke: &[(PlayerId, Option<PersonId>)]) {
        let p = self.persons[x].owner;
        let name = self.persons[x].name.clone();
        let subject = self.person_subject(x);
        let civ_name = self.player_get_civilization_short_description(p);
        let turn = self.game_turn();
        let to = self.audience_met(self.players[p].team);
        let (going, _) = self.commander_sworn_going(x);
        // 1. A House that broke away this turn whose head is not their rival.
        let rival = self.persons[x].rival;
        let join = broke.iter().find(|&&(np, head)| self.players[np].alive && self.player_get_capital_city(np).is_some() && (head.is_none() || head != rival)).map(|&(np, _)| np);
        if let (Some(np), false) = (join, going.is_empty()) {
            let over = self.player_house_name(np);
            let n = going.len();
            self.persons_leave_with(p, &[], &going, &over);
            if self.persons.contains(x) {
                self.person_leave(x);
            }
            for &u in &going {
                self.persons_unit_over(u, p, np);
            }
            self.log.msg_to(turn, to, || format!("{subject} goes over to {over} with {n} units."));
            self.tel_person(p, "seceded", "commander_house");
            return;
        }
        // 2. A city seized.
        if let Some(c) = self.commander_seize_target(x, &going) {
            let leader = match self.persons[x].leader {
                Some(l) => Some(l),
                None => self.player_get_civilization_type(p).and_then(|civ| crate::ffh::cards::sundering::choose_faction_leader(self, civ, &[])),
            };
            if let Some(leader) = leader {
                let city = self.persons_city_name(c);
                let n = going.len();
                let description = format!("{civ_name} of {city}");
                let plot = self.city_plot(c);
                self.persons_leave_with(p, &[plot], &going, &description);
                if self.persons.contains(x) {
                    self.person_leave(x);
                }
                let np = self.persons_birth(p, leader, description, vec![c]);
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
        self.person_leave(x);
        self.tel_person(p, "seceded", "commander_resigned");
    }

    // -----------------------------------------------------------------------------------------
    // The turn, the civil war
    // -----------------------------------------------------------------------------------------

    /// Player `p`'s countdowns that ran out (its turn processing): Houses first, then
    /// commanders.
    pub(crate) fn player_persons_secessions(&mut self, p: PlayerId) {
        let due = self.persons_countdowns_due(p);
        if due.is_empty() {
            return;
        }
        let mut broke: Vec<(PlayerId, Option<PersonId>)> = Vec::new();
        for &d in &due {
            if let CountdownDue::House(h) = d {
                if self.houses.get(h).is_some_and(|house| house.standing.countdown.is_some()) {
                    let head = self.houses[h].head;
                    if let Some(np) = self.house_break_away(h, Why::Countdown) {
                        broke.push((np, head));
                    }
                }
            }
        }
        for &d in &due {
            if let CountdownDue::Commander(x) = d {
                if self.persons.get(x).is_some_and(|person| person.owner == p && matches!(person.post, Post::Commander(_)) && person.standing.countdown.is_some()) {
                    self.commander_secede(x, &broke);
                }
            }
        }
    }

    /// Player `p` adopting direct rule with Houses: every House breaks away at once
    /// (docs/PERSONS.md, rule 11). Called when its civics change into direct rule
    /// (`player_set_civics`); a vassal taken later under direct rule is a House and stays.
    pub fn player_persons_civil_war(&mut self, p: PlayerId) {
        if self.players[p].houses.is_empty() || !self.player_rules_directly(p) {
            return;
        }
        let mut houses = self.players[p].houses.clone();
        houses.sort();
        for h in houses {
            if self.houses.contains(h) {
                self.house_break_away(h, Why::CivilWar);
            }
            // (Whatever could not break away ends too.)
            if self.houses.contains(h) {
                self.house_record_end(h);
            }
        }
        self.players[p].houses.clear();
    }
}
