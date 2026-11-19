//! A House's requests (docs/PERSONS.md, "The third build", rule 2: the second build's demands,
//! called requests in every text the player reads; the code keeps the word "demand").
//!
//! One standing request a House, checked and made in its liege's turn processing. What a House
//! requests depends on its leader's ambition (the Land, War and Faith lists; Land and Works: the
//! Land list). Met: `Request met: +3 (60 turns)`. Lapsed at its term: `Request refused: -3 (60
//! turns)`. Made impossible by an act of the liege (the city granted to another House or razed by
//! the liege, peace made with the civilization of a city to be taken): refused at once. Made
//! impossible otherwise: replaced, without a grievance. The next request comes
//! `PERSON_DEMAND_REST` turns after one was met or refused.

use super::*;

impl Game {
    // -----------------------------------------------------------------------------------------
    // Text
    // -----------------------------------------------------------------------------------------

    /// A city named by its plot: its name, else the plot.
    pub(crate) fn demand_city_name(&self, plot: PlotIdx) -> String {
        match self.persons_city_at(plot) {
            Some(c) => self.persons_city_name(c),
            None => {
                let (x, y) = self.map.xy(plot);
                format!("the city at ({x}, {y})")
            }
        }
    }

    /// A city named by its plot with its owner when it is not `p`'s: `Torrin (Sheaim)`.
    fn demand_city_owned(&self, p: PlayerId, plot: PlotIdx) -> String {
        let name = self.demand_city_name(plot);
        match self.persons_city_at(plot) {
            Some(c) if self.cities[c].owner != p => format!("{name} ({})", self.player_get_civilization_short_description(self.cities[c].owner)),
            _ => name,
        }
    }

    /// The text of request `kind` of House `h`: `Torrin (Sheaim) taken and granted to House
    /// Ashledger.`
    pub fn demand_kind_text(&self, h: HouseId, kind: DemandKind) -> String {
        let p = self.houses[h].owner;
        let house = self.house_label(h);
        match kind {
            DemandKind::GrantCity(plot) => match self.persons_city_at(plot) {
                Some(c) if self.cities[c].owner == p => format!("{} granted to {house}.", self.demand_city_name(plot)),
                _ => format!("{} taken and granted to {house}.", self.demand_city_owned(p, plot)),
            },
            DemandKind::TakeCity(plot) => format!("{} taken.", self.demand_city_owned(p, plot)),
            DemandKind::War(q) => format!("War with the {}.", self.player_get_civilization_short_description(q)),
            DemandKind::AdoptReligion => "A state religion adopted.".into(),
            DemandKind::SpreadReligion(plot) => match self.player_get_state_religion(p) {
                Some(r) => format!("{} in {}.", self.d.text.name(&self.d.religions[r].base.description), self.demand_city_name(plot)),
                None => format!("The state religion in {}.", self.demand_city_name(plot)),
            },
        }
    }

    /// The short form for the record: `Torrin granted`, `war with the Sheaim`.
    fn demand_kind_short(&self, h: HouseId, kind: DemandKind) -> String {
        let p = self.houses[h].owner;
        match kind {
            DemandKind::GrantCity(plot) => format!("{} granted", self.demand_city_name(plot)),
            DemandKind::TakeCity(plot) => format!("{} taken", self.demand_city_name(plot)),
            DemandKind::War(q) => format!("war with the {}", self.player_get_civilization_short_description(q)),
            DemandKind::AdoptReligion => "a state religion".into(),
            DemandKind::SpreadReligion(plot) => match self.player_get_state_religion(p) {
                Some(r) => format!("{} in {}", self.d.text.name(&self.d.religions[r].base.description), self.demand_city_name(plot)),
                None => format!("the state religion in {}", self.demand_city_name(plot)),
            },
        }
    }

    // -----------------------------------------------------------------------------------------
    // The lists
    // -----------------------------------------------------------------------------------------

    fn plot_dist(&self, a: PlotIdx, b: PlotIdx) -> i32 {
        let (ax, ay) = self.map.xy(a);
        let (bx, by) = self.map.xy(b);
        self.plot_distance(ax, ay, bx, by)
    }

    /// Whether team `a` has more power than team `b` (the rules' own measure, vassals included).
    pub(crate) fn team_stronger(&self, a: TeamId, b: TeamId) -> bool {
        self.team_get_power(a, true) > self.team_get_power(b, true)
    }

    /// Whether teams `a` and `b` are master and vassal (either way).
    fn teams_vassal_bound(&self, a: TeamId, b: TeamId) -> bool {
        self.teams[a].vassal[b] || self.teams[b].vassal[a]
    }

    /// The Land list of House `h`: (1) the Crown's city nearest its seat (not the capital)
    /// granted to it; (2) else a city of another owner within [`LAND_RANGE`] plots of a city of
    /// the House, the barbarians' first, else of a weaker civilization that is not the liege's
    /// team, vassal or master, the nearest, taken and granted.
    fn demand_land(&self, h: HouseId) -> Option<DemandKind> {
        let p = self.houses[h].owner;
        let team = self.players[p].team;
        let seat = self.house_seat(h)?;
        let seat_plot = self.city_plot(seat);
        let crown = self.players[p]
            .cities
            .iter()
            .copied()
            .filter(|&c| !self.city_is_capital(c) && !self.cities[c].outpost)
            .min_by_key(|&c| (self.plot_dist(self.city_plot(c), seat_plot), c));
        if let Some(c) = crown {
            return Some(DemandKind::GrantCity(self.city_plot(c)));
        }
        let own: Vec<PlotIdx> = self.house_cities(h).iter().map(|&c| self.city_plot(c)).collect();
        let barbarian = self.barbarian_player();
        let mut best: Option<((bool, i32, CityId), PlotIdx)> = None;
        for q in self.player_ids() {
            if q == p || !self.players[q].alive {
                continue;
            }
            let qt = self.players[q].team;
            let is_barb = q == barbarian;
            if !is_barb && (qt == team || self.teams_vassal_bound(team, qt) || !self.team_stronger(team, qt) || self.player_is_minor_civ(q) || self.player_is_crusade_host(q)) {
                continue;
            }
            for &c in &self.players[q].cities {
                if !self.city_is_revealed(c, team, false) {
                    continue;
                }
                let plot = self.city_plot(c);
                let Some(dist) = own.iter().map(|&o| self.plot_dist(o, plot)).min() else { continue };
                if dist > LAND_RANGE {
                    continue;
                }
                let key = (!is_barb, dist, c);
                if best.map_or(true, |(k, _)| key < k) {
                    best = Some((key, plot));
                }
            }
        }
        best.map(|(_, plot)| DemandKind::GrantCity(plot))
    }

    /// The War list of House `h`: (1) while its liege's team is at war with a civilization, its
    /// city nearest the House's seat, taken; (2) else war with a civilization the liege has met,
    /// is at peace with and may declare war on, weaker than it, of the alignment opposite the
    /// House's leader's first, then the nearest capital.
    fn demand_war(&self, h: HouseId) -> Option<DemandKind> {
        let p = self.houses[h].owner;
        let team = self.players[p].team;
        let from = self.city_plot(self.house_seat(h)?);
        let barbarian = self.barbarian_player();
        let mut best: Option<((i32, CityId), PlotIdx)> = None;
        for q in self.player_ids() {
            if q == barbarian || !self.players[q].alive || !self.teams[team].at_war[self.players[q].team] {
                continue;
            }
            for &c in &self.players[q].cities {
                if !self.city_is_revealed(c, team, false) {
                    continue;
                }
                let plot = self.city_plot(c);
                let key = (self.plot_dist(from, plot), c);
                if best.map_or(true, |(k, _)| key < k) {
                    best = Some((key, plot));
                }
            }
        }
        if let Some((_, plot)) = best {
            return Some(DemandKind::TakeCity(plot));
        }
        let alignment = self.house_alignment(h);
        let target = self
            .player_ids()
            .filter(|&q| {
                let qt = self.players[q].team;
                q != p
                    && q != barbarian
                    && self.players[q].alive
                    && !self.player_is_minor_civ(q)
                    && !self.player_is_crusade_host(q)
                    && qt != team
                    && self.teams[team].has_met[qt]
                    && !self.teams[team].at_war[qt]
                    && !self.teams_vassal_bound(team, qt)
                    && self.team_can_declare_war(team, qt)
                    && self.team_stronger(team, qt)
            })
            .min_by_key(|&q| {
                let opposed = self.alignments_opposed(alignment, self.players[q].alignment);
                let dist = self.player_get_capital_city(q).map_or(i32::MAX, |c| self.plot_dist(from, self.city_plot(c)));
                (!opposed, dist, q)
            });
        target.map(DemandKind::War)
    }

    /// The Faith list of House `h`: (1) with no state religion in its liege's realm and a
    /// religion in one of the liege's cities that it may adopt, a state religion adopted; (2) else
    /// the state religion in a city of the liege's without it (the nearest to the House's seat).
    fn demand_faith(&self, h: HouseId) -> Option<DemandKind> {
        let p = self.houses[h].owner;
        match self.player_get_state_religion(p) {
            None => {
                let any = self.d.religions.keys().any(|r| self.players[p].cities.iter().any(|&c| self.cities[c].has_religion[r]) && self.player_can_convert(p, Some(r)));
                any.then_some(DemandKind::AdoptReligion)
            }
            Some(r) => {
                let from = self.house_seat(h).map(|c| self.city_plot(c));
                self.players[p]
                    .cities
                    .iter()
                    .copied()
                    .filter(|&c| !self.cities[c].has_religion[r])
                    .min_by_key(|&c| (from.map_or(0, |f| self.plot_dist(f, self.city_plot(c))), c))
                    .map(|c| DemandKind::SpreadReligion(self.city_plot(c)))
            }
        }
    }

    /// The request House `h` would make now: the list of its ambition (War, Faith; Land and
    /// Works: none), then the Land list.
    pub fn house_next_demand(&self, h: HouseId) -> Option<DemandKind> {
        if !self.houses.contains(h) {
            return None;
        }
        let first = match self.house_ambition(h) {
            Ambition::Land | Ambition::Works => None,
            Ambition::War => self.demand_war(h),
            Ambition::Faith => self.demand_faith(h),
        };
        first.or_else(|| self.demand_land(h))
    }

    // -----------------------------------------------------------------------------------------
    // Met, impossible
    // -----------------------------------------------------------------------------------------

    /// Whether request `kind` of House `h` is met now.
    pub fn demand_is_met(&self, h: HouseId, kind: DemandKind) -> bool {
        let p = self.houses[h].owner;
        let team = self.players[p].team;
        match kind {
            DemandKind::GrantCity(plot) => self.persons_city_at(plot).is_some_and(|c| self.cities[c].owner == self.houses[h].player),
            DemandKind::TakeCity(plot) => self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == team),
            DemandKind::War(q) => self.teams[team].at_war[self.players[q].team],
            DemandKind::AdoptReligion => self.player_get_state_religion(p).is_some(),
            DemandKind::SpreadReligion(plot) => match (self.player_get_state_religion(p), self.persons_city_at(plot)) {
                (Some(r), Some(c)) => self.cities[c].owner == p && self.cities[c].has_religion[r],
                _ => false,
            },
        }
    }

    /// Whether request `kind` of House `h` (not met) can no longer be met.
    fn demand_is_impossible(&self, h: HouseId, kind: DemandKind) -> bool {
        let p = self.houses[h].owner;
        let team = self.players[p].team;
        match kind {
            DemandKind::GrantCity(plot) => {
                let Some(c) = self.persons_city_at(plot) else { return true };
                let owner = self.cities[c].owner;
                if owner == p {
                    self.city_is_capital(c) || self.cities[c].outpost
                } else {
                    let ot = self.players[owner].team;
                    ot == team || self.teams_vassal_bound(team, ot)
                }
            }
            DemandKind::TakeCity(plot) => match self.persons_city_at(plot) {
                None => true,
                Some(c) => !self.teams[team].at_war[self.players[self.cities[c].owner].team],
            },
            DemandKind::War(q) => {
                let qt = self.players[q].team;
                !self.players[q].alive || (!self.teams[team].at_war[qt] && !self.team_can_declare_war(team, qt))
            }
            DemandKind::AdoptReligion => !self.d.religions.keys().any(|r| self.players[p].cities.iter().any(|&c| self.cities[c].has_religion[r]) && self.player_can_convert(p, Some(r))),
            DemandKind::SpreadReligion(plot) => self.player_get_state_religion(p).is_none() || self.persons_city_at(plot).map_or(true, |c| self.cities[c].owner != p),
        }
    }

    /// Whether request `kind` names the city at `plot`.
    pub(crate) fn demand_names_plot(kind: DemandKind, plot: PlotIdx) -> bool {
        match kind {
            DemandKind::GrantCity(q) | DemandKind::TakeCity(q) | DemandKind::SpreadReligion(q) => q == plot,
            _ => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Outcomes
    // -----------------------------------------------------------------------------------------

    /// House `h`'s request is made: `kind`, for `PERSON_DEMAND_TURNS` turns.
    fn house_issue_demand(&mut self, h: HouseId, kind: DemandKind) {
        let turn = self.game_turn();
        let turns = self.d.defines.person_demand_turns;
        self.houses[h].demand = Some(Demand { kind, issued: turn, deadline: turn + turns });
        let subject = self.house_label(h);
        let text = self.demand_kind_text(h, kind);
        self.house_msg(h, || format!("{subject} requests: {text} {turns} turns."));
        self.tel_house(self.houses[h].owner, "request", "issued");
    }

    /// House `h`'s request is met: `Request met: +3 (60 turns)`.
    pub(crate) fn house_demand_met(&mut self, h: HouseId) {
        let Some(dm) = self.houses[h].demand.take() else { return };
        let turn = self.game_turn();
        self.houses[h].demand_rest_until = turn + self.d.defines.person_demand_rest;
        let short = self.demand_kind_short(h, dm.kind);
        self.house_add_mod(h, LoyaltyCause::DemandMet, DEMAND_MET);
        self.house_add_deed(h, format!("Request met: {short}."));
        let subject = self.house_label(h);
        self.house_msg(h, || format!("{subject}: request met ({short}): +{} ({} turns).", DEMAND_MET.0, DEMAND_MET.1));
        self.tel_house(self.houses[h].owner, "request", "met");
    }

    /// House `h`'s request is refused (lapsed, or made impossible by an act of its liege):
    /// `Request refused: -3 (60 turns)`.
    pub(crate) fn house_demand_refused(&mut self, h: HouseId) {
        let Some(dm) = self.houses[h].demand.take() else { return };
        // Asked again by an ultimatum while its `Request refused` runs (the third build, rule 5 (a)).
        self.houses[h].refused = Some(dm.kind);
        let turn = self.game_turn();
        self.houses[h].demand_rest_until = turn + self.d.defines.person_demand_rest;
        let short = self.demand_kind_short(h, dm.kind);
        self.house_add_mod(h, LoyaltyCause::DemandRefused, DEMAND_REFUSED);
        self.house_add_deed(h, format!("Request refused: {short}."));
        let subject = self.house_label(h);
        self.house_msg(h, || format!("{subject}: request refused ({short}): {} ({} turns).", DEMAND_REFUSED.0, DEMAND_REFUSED.1));
        self.tel_house(self.houses[h].owner, "request", "refused");
    }

    /// Player `p`'s Houses' requests in its turn processing: met, impossible (replaced), lapsed;
    /// then new ones for those without one whose rest is over.
    pub(crate) fn player_persons_demands(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for h in self.player_houses(p) {
            if let Some(dm) = self.houses[h].demand {
                if self.demand_is_met(h, dm.kind) {
                    self.house_demand_met(h);
                } else if self.demand_is_impossible(h, dm.kind) {
                    // Replaced, without a grievance (a new one below, at once).
                    self.houses[h].demand = None;
                } else if turn >= dm.deadline {
                    self.house_demand_refused(h);
                } else if dm.deadline - turn == DEMAND_WARNING_TURNS {
                    let subject = self.house_label(h);
                    let text = self.demand_kind_text(h, dm.kind);
                    self.house_msg(h, || format!("{subject} requests: {text} {DEMAND_WARNING_TURNS} turns left."));
                }
            }
            if self.houses[h].demand.is_none() && turn >= self.houses[h].demand_rest_until {
                if let Some(kind) = self.house_next_demand(h) {
                    if !self.demand_is_met(h, kind) {
                        self.house_issue_demand(h, kind);
                    }
                }
            }
        }
    }

    /// The requests of player `p`'s Houses that name the city at `plot` are refused at once (the
    /// city razed by `p`). Called before the city goes.
    pub(crate) fn persons_city_razed(&mut self, p: PlayerId, plot: PlotIdx) {
        for h in self.player_houses(p) {
            if self.houses[h].demand.is_some_and(|dm| Self::demand_names_plot(dm.kind, plot)) {
                self.house_demand_refused(h);
            }
        }
    }

    /// Teams `t` and `other` made peace: requests of their Houses to take a city of the other side
    /// are refused at once.
    pub(crate) fn persons_peace_made(&mut self, t: TeamId, other: TeamId) {
        for p in self.player_ids() {
            let mine = self.players[p].team;
            let them = if mine == t {
                other
            } else if mine == other {
                t
            } else {
                continue;
            };
            for h in self.player_houses(p) {
                let Some(DemandKind::TakeCity(plot)) = self.houses[h].demand.map(|dm| dm.kind) else { continue };
                if self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == them) {
                    self.house_demand_refused(h);
                }
            }
        }
    }

    /// The turns left of House `h`'s request (0 at its last turn).
    pub fn house_demand_turns_left(&self, h: HouseId) -> Option<i32> {
        self.houses[h].demand.map(|d| (d.deadline - self.game_turn()).max(0))
    }
}
