//! Previews (pure): what a choice does to each House before it is made, for the client and the AI
//! (docs/PERSONS.md, UI: "Previews at the decision"), and a Furious House's stated outcome (its
//! ultimatum; the third build, rule 5). A contract's floor counts (rule 6): a House under one is
//! at least Annoyed after any act.

use ffh_data::*;

use super::*;

/// One line of a preview: which House, what, and the state it leaves the House in when that
/// changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLine {
    /// Who: `House Arne`.
    pub subject: String,
    /// What: `Granted Torrin: +1 (30 turns)`, `Request met: +3 (60 turns)`.
    pub text: String,
    /// The state after the choice, on the subject's last line, only when it changes.
    pub state_after: Option<LoyaltyState>,
}

/// One choice of the grant list ([`Game::grant_candidates`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantCandidate {
    pub to: GrantTo,
    /// `House Arne`; a founding: the name the new House takes (`House Bertram`).
    pub label: String,
    /// A claimant to the city.
    pub claimant: bool,
    /// What the grant does, grantee first, then each snubbed claimant.
    pub lines: Vec<PreviewLine>,
    /// Why it cannot be chosen, if it cannot (`No leader is free to found a House.`).
    pub refusal: Option<&'static str>,
}

/// An act whose effect on the Houses is previewed ([`Game::persons_preview`]).
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
    house: Option<HouseId>,
    /// Its loyalty now as its parts make it, without a contract's floor (for a House that does not
    /// exist yet: 0).
    total: i32,
    state: LoyaltyState,
    lines: Vec<(String, i32)>,
}

fn amount_text(label: &str, (amount, turns): (i32, i32)) -> String {
    let sign = if amount > 0 { "+" } else { "" };
    format!("{label}: {sign}{amount} ({turns} turns)")
}

impl Game {
    fn pending_for(&self, h: HouseId) -> Pending {
        Pending { subject: self.house_label(h), house: Some(h), total: self.house_loyalty_raw(h), state: self.house_state(h), lines: Vec::new() }
    }

    fn pending_lines(&self, list: Vec<Pending>) -> Vec<PreviewLine> {
        let mut out = Vec::new();
        for pd in list {
            let delta: i32 = pd.lines.iter().map(|l| l.1).sum();
            let after = LoyaltyState::of(match pd.house {
                Some(h) => self.house_loyalty_floored(h, pd.total + delta),
                None => pd.total + delta,
            });
            let n = pd.lines.len();
            for (i, (text, _)) in pd.lines.into_iter().enumerate() {
                let state_after = (i + 1 == n && after != pd.state).then_some(after);
                out.push(PreviewLine { subject: pd.subject.clone(), text, state_after });
            }
        }
        out
    }

    /// Adds `(text, delta)` to House `h`, creating its subject in `list` when new.
    fn pending_add(&self, list: &mut Vec<Pending>, h: HouseId, text: String, delta: i32) {
        if let Some(pd) = list.iter_mut().find(|pd| pd.house == Some(h)) {
            pd.lines.push((text, delta));
            return;
        }
        let mut pd = self.pending_for(h);
        pd.lines.push((text, delta));
        list.push(pd);
    }

    /// What granting city `c` of player `p` to `to` does (pure).
    pub fn grant_preview(&self, p: PlayerId, c: CityId, to: GrantTo) -> Vec<PreviewLine> {
        let city = self.persons_city_name(c);
        let plot = self.city_plot(c);
        let met = amount_text("Request met", DEMAND_MET);
        let refused = amount_text("Request refused", DEMAND_REFUSED);
        let mut list: Vec<Pending> = Vec::new();
        // The grantee.
        let (house, grantee) = match to {
            GrantTo::House(h) => (self.house_label(h), Some(h)),
            GrantTo::NewHouse => (format!("House {}", self.house_new_name(p)), None),
        };
        let mut first = match grantee {
            Some(h) => self.pending_for(h),
            None => Pending { subject: house.clone(), house: None, total: 0, state: LoyaltyState::of(0), lines: Vec::new() },
        };
        if to == GrantTo::NewHouse {
            if let Some(line) = self.house_founding_line(p) {
                first.lines.push((line.trim_end_matches('.').to_string(), 0));
            }
            // A new House pays the realm's default tribute (a standing part of its loyalty).
            let pct = self.players[p].house_tribute;
            let part = tribute_part(pct);
            first.lines.push((format!("Tribute {pct}%: {}{part}", if part > 0 { "+" } else { "" }), part));
        }
        first.lines.push((amount_text(&format!("Granted {city}"), GRANTED), GRANTED.0));
        if let Some(h) = grantee {
            if self.houses[h].demand.is_some_and(|dm| dm.kind == DemandKind::GrantCity(plot)) {
                first.lines.push((met.clone(), DEMAND_MET.0));
            }
        }
        list.push(first);
        // The snubbed claimants.
        for o in self.city_claimants(c) {
            if Some(o) == grantee {
                continue;
            }
            self.pending_add(&mut list, o, amount_text(&format!("{city} granted to {house}"), SNUB_HOUSE), SNUB_HOUSE.0);
        }
        // Requests the grant makes impossible.
        for o in self.player_houses(p) {
            if Some(o) != grantee && self.houses[o].demand.is_some_and(|dm| dm.kind == DemandKind::GrantCity(plot)) {
                self.pending_add(&mut list, o, refused.clone(), DEMAND_REFUSED.0);
            }
        }
        self.pending_lines(list)
    }

    /// The grant list of city `c` of player `p` (pure): every House, each with whether it claims
    /// the city and what the grant does; with no House, the founding of one (rule 3: its refusal
    /// when no leader is free). Empty when the city cannot be granted.
    pub fn grant_candidates(&self, p: PlayerId, c: CityId) -> Vec<GrantCandidate> {
        if self.player_check_grant_city(p, c).is_err() {
            return Vec::new();
        }
        let claimants = self.city_claimants(c);
        let houses = self.player_houses(p);
        let mut out = Vec::new();
        for &h in &houses {
            let to = GrantTo::House(h);
            out.push(GrantCandidate { to, label: self.house_label(h), claimant: claimants.contains(&h), lines: self.grant_preview(p, c, to), refusal: None });
        }
        if houses.is_empty() {
            let to = GrantTo::NewHouse;
            let refusal = self.player_check_found_house(p).err();
            out.push(GrantCandidate { to, label: format!("House {}", self.house_new_name(p)), claimant: false, lines: self.grant_preview(p, c, to), refusal });
        }
        out
    }

    /// What an act of player `p` does to its Houses (pure).
    pub fn persons_preview(&self, p: PlayerId, act: &PersonsAct) -> Vec<PreviewLine> {
        let met = amount_text("Request met", DEMAND_MET);
        let refused = amount_text("Request refused", DEMAND_REFUSED);
        let mut list: Vec<Pending> = Vec::new();
        let houses = self.player_houses(p);
        let demands: Vec<(HouseId, DemandKind)> = houses.iter().filter_map(|&h| self.houses[h].demand.map(|dm| (h, dm.kind))).collect();
        // A contract that forbids the act (the third build, rule 6): its House's line says so.
        let bound = match *act {
            PersonsAct::DeclareWar(t) => Some(super::ContractAct::DeclareWar(t)),
            PersonsAct::MakePeace(t) => Some(super::ContractAct::MakePeace(t)),
            PersonsAct::Convert(_) => Some(super::ContractAct::ChangeReligion),
            _ => None,
        };
        if let Some(bound) = bound {
            let mut out = Vec::new();
            for &h in &houses {
                let forbids = self.house_contract(h).is_some_and(|c| self.contract_forbids(h, &c, bound));
                if let (true, Some(cv)) = (forbids, self.house_contract_view(h)) {
                    out.push(PreviewLine { subject: self.house_label(h), text: cv.line.trim_end_matches('.').to_string(), state_after: None });
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
        match *act {
            PersonsAct::DeclareWar(t) => {
                for &(h, kind) in &demands {
                    if let DemandKind::War(q) = kind {
                        if self.players[q].team == t {
                            self.pending_add(&mut list, h, met.clone(), DEMAND_MET.0);
                        }
                    }
                }
            }
            PersonsAct::MakePeace(t) => {
                for &(h, kind) in &demands {
                    if let DemandKind::TakeCity(plot) = kind {
                        if self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == t) {
                            self.pending_add(&mut list, h, refused.clone(), DEMAND_REFUSED.0);
                        }
                    }
                }
            }
            PersonsAct::Convert(religion) => {
                for &(h, kind) in &demands {
                    if kind == DemandKind::AdoptReligion && religion.is_some() {
                        self.pending_add(&mut list, h, met.clone(), DEMAND_MET.0);
                    }
                }
                let now = self.player_get_state_religion(p);
                for &h in &houses {
                    let (before, after) = (self.house_religion_part_under(h, now), self.house_religion_part_under(h, religion));
                    if before != after {
                        let text = if after < 0 { format!("State religion opposed: {after}") } else { format!("State religion opposed ends: +{}", -before) };
                        self.pending_add(&mut list, h, text, after - before);
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
                    for &h in &houses {
                        let (going, _) = self.house_cities_held(h);
                        let mut text = if going.is_empty() { "Every city is held by an army: the House ends.".to_string() } else { format!("Breaks away, taking {}: war.", self.city_names_of(&going)) };
                        text.push_str(&self.house_hold_text(h));
                        out.push(PreviewLine { subject: self.house_label(h), text, state_after: None });
                    }
                    return out;
                }
            }
            PersonsAct::Raze(c) => {
                let plot = self.city_plot(c);
                for &(h, kind) in &demands {
                    if Self::demand_names_plot(kind, plot) {
                        self.pending_add(&mut list, h, refused.clone(), DEMAND_REFUSED.0);
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

    /// The stated outcome of a Furious House `h`, for its liege (the third build, rule 5): its
    /// ultimatum at the liege's next turn, and what refusing it does now: `House Ashledger issues
    /// an ultimatum at your next turn. Refused, it breaks away, taking Prespur and Elam: war. Elam
    /// goes with it: its 3 units against your army's 2. Torrin returns to you: your army's 4 units
    /// outnumber its 2.` (the contest for each city where an army of the liege stands,
    /// [`Game::house_hold_text`]); `Refused, it ends: every city is held.` when none would go.
    pub fn house_breakaway_outcome(&self, h: HouseId) -> String {
        let label = self.house_label(h);
        let (going, _) = self.house_cities_held(h);
        let cities = self.city_names_of(&going);
        let contests = self.house_hold_text(h);
        if going.is_empty() {
            format!("{label} issues an ultimatum at your next turn. Refused, it ends: every city is held.{contests}")
        } else {
            format!("{label} issues an ultimatum at your next turn. Refused, it breaks away, taking {cities}: war.{contests}")
        }
    }
}
