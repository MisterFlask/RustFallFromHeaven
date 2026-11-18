//! Previews (pure): what a choice does to each holder before it is made, for the client and the
//! AI (docs/PERSONS.md, UI: "Previews at the decision"), and a countdown's stated outcome.

use ffh_data::*;

use super::houses::Claimant;
use super::loyalty::Holder;
use super::*;

/// One line of a preview: who, what, and the state it leaves them in when that changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLine {
    /// Who: `House Arne`, `Hrolf, 2nd Army`.
    pub subject: String,
    /// What: `Granted Torrin: +1 (30 turns)`, `Demand met: +3 (60 turns)`.
    pub text: String,
    /// The state after the choice, on the subject's last line, only when it changes.
    pub state_after: Option<LoyaltyState>,
}

/// One choice of the grant list ([`Game::grant_candidates`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCandidate {
    pub to: GrantTo,
    /// `House Arne`, `Hrolf` (founds a House), `Great Prophet` (draws the founder).
    pub label: String,
    /// A claimant to the city.
    pub claimant: bool,
    /// What the grant does, grantee first, then each snubbed claimant.
    pub lines: Vec<PreviewLine>,
    /// Why it cannot be chosen, if it cannot (`No person of note is left to found a House.`).
    pub refusal: Option<&'static str>,
}

/// An act whose effect on the cast is previewed ([`Game::persons_preview`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonsAct<'a> {
    DeclareWar(TeamId),
    MakePeace(TeamId),
    /// A new state religion (`None`: none).
    Convert(Option<ReligionType>),
    /// Civics adopted (each replaces the civic of its option).
    Civics(&'a [CivicType]),
    /// One of the player's cities razed by it.
    Raze(CityId),
}

/// Lines of one subject before their states are worked out: (text, change of loyalty).
struct Pending {
    subject: String,
    holder: Option<Holder>,
    /// Their loyalty now (for a holder that does not exist yet: 0).
    total: i32,
    state: LoyaltyState,
    lines: Vec<(String, i32)>,
}

fn amount_text(label: &str, (amount, turns): (i32, i32)) -> String {
    let sign = if amount > 0 { "+" } else { "" };
    format!("{label}: {sign}{amount} ({turns} turns)")
}

impl Game {
    fn pending_for(&self, hd: Holder) -> Pending {
        let total = self.holder_loyalty(hd);
        Pending { subject: self.holder_label(hd), holder: Some(hd), total, state: self.holder_state(hd), lines: Vec::new() }
    }

    fn pending_lines(&self, list: Vec<Pending>) -> Vec<PreviewLine> {
        let mut out = Vec::new();
        for pd in list {
            let delta: i32 = pd.lines.iter().map(|l| l.1).sum();
            let after = match pd.holder {
                Some(hd) => self.holder_state_at(hd, pd.total + delta),
                None => LoyaltyState::of(pd.total + delta),
            };
            let n = pd.lines.len();
            for (i, (text, _)) in pd.lines.into_iter().enumerate() {
                let state_after = (i + 1 == n && after != pd.state).then_some(after);
                out.push(PreviewLine { subject: pd.subject.clone(), text, state_after });
            }
        }
        out
    }

    /// Adds `(text, delta)` to the subject `hd`, creating it in `list` when new.
    fn pending_add(&self, list: &mut Vec<Pending>, hd: Holder, text: String, delta: i32) {
        if let Some(pd) = list.iter_mut().find(|pd| pd.holder == Some(hd)) {
            pd.lines.push((text, delta));
            return;
        }
        let mut pd = self.pending_for(hd);
        pd.lines.push((text, delta));
        list.push(pd);
    }

    /// What granting city `c` of player `p` to `to` does (pure).
    pub fn grant_preview(&self, p: PlayerId, c: CityId, to: GrantTo) -> Vec<PreviewLine> {
        let city = self.persons_city_name(c);
        let plot = self.city_plot(c);
        let met = amount_text("Demand met", DEMAND_MET);
        let refused = amount_text("Demand refused", DEMAND_REFUSED);
        let mut list: Vec<Pending> = Vec::new();
        // The grantee.
        let (house, head, grantee) = match to {
            GrantTo::House(h) => (self.house_label(h), self.houses[h].head, Some(Holder::House(h))),
            GrantTo::Person(x) => (format!("House {}", self.house_new_name(p)), Some(x), Some(Holder::Person(x))),
            GrantTo::GreatPerson(_) => (format!("House {}", self.house_new_name(p)), None, None),
        };
        let mut first = match grantee {
            Some(hd) => {
                let mut pd = self.pending_for(hd);
                pd.subject = house.clone();
                pd
            }
            None => Pending { subject: house.clone(), holder: None, total: 0, state: LoyaltyState::of(0), lines: Vec::new() },
        };
        if let GrantTo::Person(x) = to {
            let name = &self.persons[x].name;
            first.lines.push((format!("Founded by {name} at {city}: a civilization of its own, your vassal"), 0));
            if let Some(a) = self.person_army(x) {
                first.lines.push((format!("Command of the {} ends", self.armies[a].name), 0));
            }
            if self.person_unit(x).is_some() {
                first.lines.push((format!("{name} leaves the map"), 0));
            }
        }
        if let GrantTo::GreatPerson(u) = to {
            first.lines.push((format!("A person of note is drawn. The {} is consumed", self.unit_get_name(u)), 0));
        }
        if !matches!(to, GrantTo::House(_)) {
            // A new House pays the realm's default tribute (a standing part of its loyalty).
            let pct = self.players[p].house_tribute;
            let part = super::loyalty::tribute_part(pct);
            first.lines.push((format!("Tribute {pct}%: {}{part}", if part > 0 { "+" } else { "" }), part));
        }
        first.lines.push((amount_text(&format!("Granted {city}"), GRANTED), GRANTED.0));
        // A House's size (rule 14): the standing part changes with one great city more (a
        // settlement does not count). A grant moves a city within the realm: the threshold
        // stays.
        if let GrantTo::House(h) = to {
            let n = self.house_great_cities(h);
            let more = n + i32::from(!self.cities[c].settlement);
            let t = self.house_size_threshold(h);
            let (before, after) = (Self::size_part(n, t), Self::size_part(more, t));
            if after != before {
                let label = self.house_size_label(h, more);
                let text = if before == 0 { format!("{label}: {after}") } else { format!("{label}: {} ({after} in all)", after - before) };
                first.lines.push((text, after - before));
            }
        }
        if let Some(x) = head {
            if let Some(dm) = self.persons[x].demand {
                let meets = match dm.kind {
                    DemandKind::GrantCity(q) => q == plot,
                    DemandKind::Post | DemandKind::House => matches!(to, GrantTo::Person(_)),
                    _ => false,
                };
                if meets {
                    first.lines.push((met.clone(), DEMAND_MET.0));
                }
            }
        }
        list.push(first);
        // The snubbed claimants.
        for cl in self.city_claimants(c) {
            let (hd, person, amount) = self.claimant_holder(cl);
            if Some(hd) == grantee || (person.is_some() && person == head) {
                continue;
            }
            let label = amount_text(&format!("{city} granted to {house}"), amount);
            self.pending_add(&mut list, hd, label, amount.0);
            if let (Some(y), Some(hx)) = (person, head) {
                if self.persons.contains(y) && self.persons.contains(hx) {
                    self.pending_add(&mut list, hd, format!("Becomes the rival of {}", self.persons[hx].name), 0);
                }
            }
        }
        // Demands the grant makes impossible.
        for &y in &self.players[p].persons {
            let Some(dm) = self.persons.get(y).and_then(|person| person.demand) else { continue };
            if dm.kind == DemandKind::GrantCity(plot) && Some(y) != head {
                let hd = self.person_holder(y);
                self.pending_add(&mut list, hd, refused.clone(), DEMAND_REFUSED.0);
            }
        }
        self.pending_lines(list)
    }

    /// The grant list of city `c` of player `p` (pure): every House, every person who heads none
    /// (a commander's command ends), every great person unit, each with whether it claims the
    /// city and what the grant does. Empty when the city cannot be granted.
    pub fn grant_candidates(&self, p: PlayerId, c: CityId) -> Vec<GrantCandidate> {
        if self.player_check_grant_city(p, c).is_err() {
            return Vec::new();
        }
        let claimants = self.city_claimants(c);
        let mut out = Vec::new();
        for &h in &self.players[p].houses {
            if !self.houses.contains(h) {
                continue;
            }
            let claimant = claimants.iter().any(|&cl| match cl {
                Claimant::House(o) => o == h,
                Claimant::Commander(x) => self.houses[h].head == Some(x),
            });
            let to = GrantTo::House(h);
            out.push(GrantCandidate { to, label: self.house_label(h), claimant, lines: self.grant_preview(p, c, to), refusal: None });
        }
        for &x in &self.players[p].persons {
            if !self.persons.contains(x) || self.person_house(x).is_some() {
                continue;
            }
            let to = GrantTo::Person(x);
            let claimant = claimants.contains(&Claimant::Commander(x));
            let refusal = self.persons[x].leader.is_none().then_some("A hero cannot head a House.");
            out.push(GrantCandidate { to, label: self.persons[x].name.clone(), claimant, lines: self.grant_preview(p, c, to), refusal });
        }
        let pool_empty = self.persons_pool(p).is_empty();
        for &u in &self.players[p].units {
            if !self.unit_is_great_person(u) {
                continue;
            }
            let to = GrantTo::GreatPerson(u);
            let refusal = pool_empty.then_some("No person of note is left to found a House.");
            let label = self.unit_get_name(u);
            out.push(GrantCandidate { to, label, claimant: false, lines: self.grant_preview(p, c, to), refusal });
        }
        out
    }

    /// What an act of player `p` does to its cast (pure).
    pub fn persons_preview(&self, p: PlayerId, act: &PersonsAct) -> Vec<PreviewLine> {
        let met = amount_text("Demand met", DEMAND_MET);
        let refused = amount_text("Demand refused", DEMAND_REFUSED);
        let mut list: Vec<Pending> = Vec::new();
        let demands: Vec<(PersonId, DemandKind)> = self.players[p].persons.iter().filter_map(|&x| self.persons.get(x).and_then(|person| person.demand).map(|dm| (x, dm.kind))).collect();
        match *act {
            PersonsAct::DeclareWar(t) => {
                for &(x, kind) in &demands {
                    if let DemandKind::War(q) = kind {
                        if self.players[q].team == t {
                            self.pending_add(&mut list, self.person_holder(x), met.clone(), DEMAND_MET.0);
                        }
                    }
                }
            }
            PersonsAct::MakePeace(t) => {
                for &(x, kind) in &demands {
                    if let DemandKind::TakeCity(plot) = kind {
                        if self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == t) {
                            self.pending_add(&mut list, self.person_holder(x), refused.clone(), DEMAND_REFUSED.0);
                        }
                    }
                }
            }
            PersonsAct::Convert(religion) => {
                for &(x, kind) in &demands {
                    if kind == DemandKind::AdoptReligion && religion.is_some() {
                        self.pending_add(&mut list, self.person_holder(x), met.clone(), DEMAND_MET.0);
                    }
                }
                let now = self.player_get_state_religion(p);
                for hd in self.player_holders(p) {
                    let Some(x) = self.holder_person(hd) else { continue };
                    let (before, after) = (self.person_religion_part(x, now), self.person_religion_part(x, religion));
                    if before != after {
                        let text = if after < 0 { format!("State religion opposed: {after}") } else { format!("State religion opposed ends: +{}", -before) };
                        self.pending_add(&mut list, hd, text, after - before);
                    }
                }
            }
            PersonsAct::Civics(civics) => {
                let d = self.d;
                let mut set: Vec<Option<CivicType>> = self.players[p].civics.iter().copied().collect();
                for &c in civics {
                    let option = d.civics[c].civic_option_type;
                    if let Some(o) = option {
                        set[o.idx()] = Some(c);
                    }
                }
                let direct_after = set.iter().flatten().any(|&c| d.civics[c].direct_rule);
                if direct_after && !self.player_rules_directly(p) {
                    let mut out = Vec::new();
                    for &h in &self.players[p].houses {
                        if !self.houses.contains(h) {
                            continue;
                        }
                        let (going, held) = self.house_cities_held(h);
                        let mut text = if going.is_empty() { "Every city is held by an army: the House ends.".to_string() } else { format!("Breaks away, taking {}: war.", self.city_names_of(&going)) };
                        text.push_str(&self.held_cities_text(&held));
                        out.push(PreviewLine { subject: self.house_label(h), text, state_after: None });
                    }
                    return out;
                }
            }
            PersonsAct::Raze(c) => {
                let plot = self.city_plot(c);
                for &(x, kind) in &demands {
                    if Self::demand_names_plot(kind, plot) {
                        self.pending_add(&mut list, self.person_holder(x), refused.clone(), DEMAND_REFUSED.0);
                    }
                }
            }
        }
        self.pending_lines(list)
    }

    /// City names joined: `Prespur, Elam and Torrin`.
    pub(crate) fn city_names_of(&self, cities: &[CityId]) -> String {
        and_list(&cities.iter().map(|&c| self.persons_city_name(c)).collect::<Vec<_>>())
    }

    /// What a break-away does with the cities an army of the liege holds (`held`), for the liege
    /// to read, each after a space: ` Torrin returns to you: an army of yours stands in it.`
    /// (empty without one).
    pub(crate) fn held_cities_text(&self, held: &[CityId]) -> String {
        held.iter().map(|&c| format!(" {} returns to you: an army of yours stands in it.", self.persons_city_name(c))).collect()
    }

    /// The stated outcome of House `h`'s countdown, for its liege: `House Ashledger will break
    /// away in 8 turns, taking Prespur and Elam: war. Torrin returns to you: an army of yours
    /// stands in it.` (without a countdown: what would happen now, `would break away, taking
    /// ...`).
    pub fn house_breakaway_outcome(&self, h: HouseId) -> String {
        let label = self.house_label(h);
        let (going, held) = self.house_cities_held(h);
        let cities = self.city_names_of(&going);
        let stays = self.held_cities_text(&held);
        match self.houses[h].standing.countdown {
            Some(end) => {
                let t = (end - self.game_turn()).max(0);
                let turns = format!("{t} {}", if t == 1 { "turn" } else { "turns" });
                if going.is_empty() {
                    format!("{label} will end in {turns}: every city is held.")
                } else {
                    format!("{label} will break away in {turns}, taking {cities}: war.{stays}")
                }
            }
            None => {
                if going.is_empty() {
                    format!("{label} would end: every city is held.")
                } else {
                    format!("{label} would break away, taking {cities}: war.{stays}")
                }
            }
        }
    }

    /// The stated outcome of commander `x`'s countdown, from where the sworn units stand now
    /// (`commander_sworn_going`, `commander_seize_target`: what the secession would do this turn):
    /// `Hrolf will seize Vallus with 5 units in 8 turns. 1 stays in Torrin.`, `Hrolf will join
    /// House Arne's secession with 5 units in 8 turns.`, `Hrolf will resign in 8 turns: no city to
    /// seize.` Without a countdown: the same in the conditional (`Hrolf would seize ...`).
    pub fn commander_secession_outcome(&self, x: PersonId) -> String {
        let name = &self.persons[x].name;
        let p = self.persons[x].owner;
        let end = self.persons[x].standing.countdown;
        let when = match end {
            Some(end) => {
                let t = (end - self.game_turn()).max(0);
                format!(" in {t} {}", if t == 1 { "turn" } else { "turns" })
            }
            None => String::new(),
        };
        let will = if end.is_some() { "will" } else { "would" };
        let units = |n: usize| format!("{n} {}", if n == 1 { "unit" } else { "units" });
        let (going, staying) = self.commander_sworn_going(x);
        // The units that stay, by the city they stand in.
        let mut stays: Vec<(String, usize)> = Vec::new();
        for &u in &staying {
            let Some(c) = self.unit_plot_opt(u).and_then(|plot| self.persons_city_at(plot)) else { continue };
            let city = self.persons_city_name(c);
            match stays.iter_mut().find(|(n, _)| *n == city) {
                Some((_, k)) => *k += 1,
                None => stays.push((city, 1)),
            }
        }
        let stay = stays
            .iter()
            .map(|(city, k)| format!(" {k} {} in {city}.", if *k == 1 { "stays" } else { "stay" }))
            .collect::<String>();
        if going.is_empty() {
            return format!("{name} {will} resign{when}: every sworn unit stands in a garrisoned city.");
        }
        // A House whose countdown ends the same turn, its head not the commander's rival: its
        // break-away takes the sworn units (`commander_secede`, step 1).
        let rival = self.persons[x].rival;
        if let Some(end) = end {
            let league = self.players[p].houses.iter().copied().find(|&h| {
                self.houses.get(h).is_some_and(|house| house.standing.countdown == Some(end) && (house.head.is_none() || house.head != rival))
            });
            if let Some(h) = league {
                return format!("{name} {will} join {} as it breaks away{when}, with {}.{stay}", self.house_label(h), units(going.len()));
            }
        }
        match self.commander_seize_target(x, &going) {
            Some(c) => format!("{name} {will} seize {}{when} with {}.{stay}", self.persons_city_name(c), units(going.len())),
            None => format!("{name} {will} resign{when}: no city to seize.{stay}"),
        }
    }
}
