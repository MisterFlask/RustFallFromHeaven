//! Demands (docs/PERSONS.md, "The first build": Demands; a head makes no Works demand since the
//! second build: a head of Works ambition demands land).
//!
//! One standing demand a person, checked and made in the owner's turn processing. What a person
//! demands depends on their post and ambition (the Land, War and Faith lists). Met: `Demand
//! met: +3 (60 turns)`. Lapsed at its term: `Demand refused: -3 (60 turns)`. Made impossible by an
//! act of the owner (the city granted to another House or razed by the owner, peace made with the
//! civilization of a city to be taken): refused at once. Made impossible otherwise: replaced,
//! without a grievance. The next demand comes `PERSON_DEMAND_REST` turns after one was met or
//! refused.

use ffh_data::*;

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

    /// The text of demand `kind` of person `x`, as the doc words it: `Torrin (Sheaim) taken and
    /// granted to House Ashledger.`
    pub fn demand_kind_text(&self, x: PersonId, kind: DemandKind) -> String {
        let p = self.persons[x].owner;
        let house = || match self.person_house(x) {
            Some(h) => self.house_label(h),
            None => "their House".to_string(),
        };
        match kind {
            DemandKind::Post => "A post: a command or a House.".into(),
            DemandKind::House => "A House.".into(),
            DemandKind::GrantCity(plot) => match self.persons_city_at(plot) {
                Some(c) if self.cities[c].owner == p => format!("{} granted to {}.", self.demand_city_name(plot), house()),
                _ => format!("{} taken and granted to {}.", self.demand_city_owned(p, plot), house()),
            },
            DemandKind::TakeCity(plot) => format!("{} taken.", self.demand_city_owned(p, plot)),
            DemandKind::War(q) => format!("War with the {}.", self.player_get_civilization_short_description(q)),
            DemandKind::AdoptReligion => "A state religion adopted.".into(),
            DemandKind::SpreadReligion(plot) => match self.player_get_state_religion(p) {
                Some(r) => format!("{} in {}.", self.d.text.name(&self.d.religions[r].base.description), self.demand_city_name(plot)),
                None => format!("The state religion in {}.", self.demand_city_name(plot)),
            },
            DemandKind::Wonder { city, building } => format!("{} built in {}.", self.d.text.name(&self.d.buildings[building].base.description), self.demand_city_name(city)),
        }
    }

    /// The short form for the record: `Torrin granted`, `war with the Sheaim`.
    fn demand_kind_short(&self, x: PersonId, kind: DemandKind) -> String {
        let p = self.persons[x].owner;
        match kind {
            DemandKind::Post => "a post".into(),
            DemandKind::House => "a House".into(),
            DemandKind::GrantCity(plot) => format!("{} granted", self.demand_city_name(plot)),
            DemandKind::TakeCity(plot) => format!("{} taken", self.demand_city_name(plot)),
            DemandKind::War(q) => format!("war with the {}", self.player_get_civilization_short_description(q)),
            DemandKind::AdoptReligion => "a state religion".into(),
            DemandKind::SpreadReligion(plot) => match self.player_get_state_religion(p) {
                Some(r) => format!("{} in {}", self.d.text.name(&self.d.religions[r].base.description), self.demand_city_name(plot)),
                None => format!("the state religion in {}", self.demand_city_name(plot)),
            },
            DemandKind::Wonder { city, building } => format!("{} in {}", self.d.text.name(&self.d.buildings[building].base.description), self.demand_city_name(city)),
        }
    }

    // -----------------------------------------------------------------------------------------
    // The lists
    // -----------------------------------------------------------------------------------------

    /// The plot a person's demands are measured from: a head's seat, else their unit's.
    fn person_reference_plot(&self, x: PersonId) -> Option<PlotIdx> {
        if let Some(c) = self.person_house(x).and_then(|h| self.house_seat(h)) {
            return Some(self.city_plot(c));
        }
        self.person_unit(x).and_then(|u| self.unit_plot_opt(u))
    }

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

    /// The Land list of person `x`, head of House `h`: (1) the Crown's city nearest their seat (not
    /// the capital) granted to their House; (2) else a city of another owner within
    /// [`LAND_RANGE`] plots of a city of their House, the barbarians' first, else of a weaker
    /// civilization that is not our team, vassal or master, the nearest, taken and granted.
    fn demand_land(&self, x: PersonId, h: HouseId) -> Option<DemandKind> {
        let p = self.persons[x].owner;
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

    /// The War list of person `x`: (1) while our team is at war with a civilization, its city
    /// nearest the person (a head: their seat), taken; (2) else war with a civilization we have
    /// met, are at peace with and may declare war on, weaker than us, of the alignment opposite
    /// the person's first, then the nearest capital.
    fn demand_war(&self, x: PersonId) -> Option<DemandKind> {
        let p = self.persons[x].owner;
        let team = self.players[p].team;
        let from = self.person_reference_plot(x)?;
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
        let alignment = self.person_alignment(x);
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

    /// The Faith list of person `x`: (1) with no state religion and a religion in one of our
    /// cities that we may adopt, a state religion adopted; (2) else the state religion in a city
    /// of ours without it (the nearest to their House's seat or their unit).
    fn demand_faith(&self, x: PersonId) -> Option<DemandKind> {
        let p = self.persons[x].owner;
        match self.player_get_state_religion(p) {
            None => {
                let any = self.d.religions.keys().any(|r| self.players[p].cities.iter().any(|&c| self.cities[c].has_religion[r]) && self.player_can_convert(p, Some(r)));
                any.then_some(DemandKind::AdoptReligion)
            }
            Some(r) => {
                let from = self.person_reference_plot(x);
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

    /// The demand person `x` would make now: without a post, a post; a commander, the list of
    /// their ambition (War, Faith; Land and Works: a House); a head, the list of their ambition
    /// (War, Faith; Land and Works: none), then the Land list.
    pub fn person_next_demand(&self, x: PersonId) -> Option<DemandKind> {
        let ambition = self.persons[x].ambition;
        match self.persons[x].post {
            Post::None => Some(DemandKind::Post),
            Post::Commander(_) => match ambition {
                Ambition::War => self.demand_war(x),
                Ambition::Faith => self.demand_faith(x),
                Ambition::Land | Ambition::Works => Some(DemandKind::House),
            },
            Post::Head(h) => {
                if !self.houses.contains(h) {
                    return None;
                }
                let first = match ambition {
                    Ambition::Land | Ambition::Works => None,
                    Ambition::War => self.demand_war(x),
                    Ambition::Faith => self.demand_faith(x),
                };
                first.or_else(|| self.demand_land(x, h))
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Met, impossible, paused
    // -----------------------------------------------------------------------------------------

    /// Whether demand `kind` of person `x` is met now.
    pub fn demand_is_met(&self, x: PersonId, kind: DemandKind) -> bool {
        let p = self.persons[x].owner;
        let team = self.players[p].team;
        match kind {
            DemandKind::Post => self.persons[x].post != Post::None,
            DemandKind::House => self.person_house(x).is_some(),
            DemandKind::GrantCity(plot) => self.persons_city_at(plot).is_some_and(|c| self.person_house(x).is_some_and(|h| self.cities[c].owner == self.houses[h].player)),
            DemandKind::TakeCity(plot) => self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == team),
            DemandKind::War(q) => self.teams[team].at_war[self.players[q].team],
            DemandKind::AdoptReligion => self.player_get_state_religion(p).is_some(),
            DemandKind::SpreadReligion(plot) => match (self.player_get_state_religion(p), self.persons_city_at(plot)) {
                (Some(r), Some(c)) => self.cities[c].owner == p && self.cities[c].has_religion[r],
                _ => false,
            },
            DemandKind::Wonder { city, building } => self.persons_city_at(city).is_some_and(|c| self.cities[c].owner == p && self.cities[c].num_real_building[building] > 0),
        }
    }

    /// Whether wonder `building` is in the queue of the city at `plot`.
    fn wonder_queued(&self, plot: PlotIdx, building: BuildingType) -> bool {
        self.persons_city_at(plot).is_some_and(|c| self.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CONSTRUCT && o.data1 == building.idx() as i32))
    }

    /// Whether the term of demand `kind` is paused now (a Works wonder in the seat's queue).
    pub fn demand_is_paused(&self, kind: DemandKind) -> bool {
        match kind {
            DemandKind::Wonder { city, building } => self.wonder_queued(city, building),
            _ => false,
        }
    }

    /// Whether demand `kind` of person `x` (not met) can no longer be met.
    fn demand_is_impossible(&self, x: PersonId, kind: DemandKind) -> bool {
        let p = self.persons[x].owner;
        let team = self.players[p].team;
        match kind {
            DemandKind::Post | DemandKind::House => false,
            DemandKind::GrantCity(plot) => {
                let Some(c) = self.persons_city_at(plot) else { return true };
                if self.person_house(x).is_none() {
                    return true;
                }
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
            DemandKind::Wonder { city, building } => match self.persons_city_at(city) {
                None => true,
                Some(c) => self.cities[c].owner != p || (!self.wonder_queued(city, building) && !self.city_can_construct(c, building, false, false, false)),
            },
        }
    }

    /// Whether demand `kind` names the city at `plot`.
    pub(crate) fn demand_names_plot(kind: DemandKind, plot: PlotIdx) -> bool {
        match kind {
            DemandKind::GrantCity(q) | DemandKind::TakeCity(q) | DemandKind::SpreadReligion(q) => q == plot,
            DemandKind::Wonder { city, .. } => city == plot,
            _ => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Outcomes
    // -----------------------------------------------------------------------------------------

    /// Person `x`'s demand is made: `kind`, for `PERSON_DEMAND_TURNS` turns.
    fn person_issue_demand(&mut self, x: PersonId, kind: DemandKind) {
        let turn = self.game_turn();
        let turns = self.d.defines.person_demand_turns;
        self.persons[x].demand = Some(Demand { kind, issued: turn, deadline: turn + turns });
        let subject = self.persons[x].name.clone();
        let text = self.demand_kind_text(x, kind);
        self.person_msg(x, || format!("{subject} demands: {text} {turns} turns."));
        self.tel_person(self.persons[x].owner, "demand", "issued");
    }

    /// Person `x`'s demand is met: `Demand met: +3 (60 turns)`.
    pub(crate) fn person_demand_met(&mut self, x: PersonId) {
        let Some(dm) = self.persons[x].demand.take() else { return };
        let turn = self.game_turn();
        self.persons[x].demand_rest_until = turn + self.d.defines.person_demand_rest;
        let short = self.demand_kind_short(x, dm.kind);
        self.holder_add_mod(self.person_holder(x), LoyaltyCause::DemandMet, DEMAND_MET);
        self.person_add_deed(x, format!("Demand met: {short}."));
        let subject = self.persons[x].name.clone();
        self.person_msg(x, || format!("{subject}: demand met ({short}): +{} ({} turns).", DEMAND_MET.0, DEMAND_MET.1));
        self.tel_person(self.persons[x].owner, "demand", "met");
    }

    /// Person `x`'s demand is refused (lapsed, or made impossible by an act of the owner):
    /// `Demand refused: -3 (60 turns)`.
    pub(crate) fn person_demand_refused(&mut self, x: PersonId) {
        let Some(dm) = self.persons[x].demand.take() else { return };
        let turn = self.game_turn();
        self.persons[x].demand_rest_until = turn + self.d.defines.person_demand_rest;
        let short = self.demand_kind_short(x, dm.kind);
        self.holder_add_mod(self.person_holder(x), LoyaltyCause::DemandRefused, DEMAND_REFUSED);
        self.person_add_deed(x, format!("Demand refused: {short}."));
        let subject = self.persons[x].name.clone();
        self.person_msg(x, || format!("{subject}: demand refused ({short}): {} ({} turns).", DEMAND_REFUSED.0, DEMAND_REFUSED.1));
        self.tel_person(self.persons[x].owner, "demand", "refused");
    }

    /// Player `p`'s demands in its turn processing: met, impossible (replaced), lapsed; then new
    /// ones for those without one whose rest is over.
    pub(crate) fn player_persons_demands(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for x in self.players[p].persons.clone() {
            if !self.persons.contains(x) {
                continue;
            }
            if let Some(dm) = self.persons[x].demand {
                if self.demand_is_met(x, dm.kind) {
                    self.person_demand_met(x);
                } else if self.demand_is_impossible(x, dm.kind) {
                    // Replaced, without a grievance (a new one below, at once).
                    self.persons[x].demand = None;
                } else {
                    if self.demand_is_paused(dm.kind) {
                        if let Some(d) = self.persons[x].demand.as_mut() {
                            d.deadline += 1;
                        }
                    }
                    let deadline = self.persons[x].demand.map_or(turn, |d| d.deadline);
                    if turn >= deadline {
                        self.person_demand_refused(x);
                    } else if deadline - turn == DEMAND_WARNING_TURNS {
                        let subject = self.persons[x].name.clone();
                        let text = self.demand_kind_text(x, dm.kind);
                        self.person_msg(x, || format!("{subject} demands: {text} {DEMAND_WARNING_TURNS} turns left."));
                    }
                }
            }
            if self.persons[x].demand.is_none() && turn >= self.persons[x].demand_rest_until {
                // A head's House must stand.
                if let Some(kind) = self.person_next_demand(x) {
                    if !self.demand_is_met(x, kind) {
                        self.person_issue_demand(x, kind);
                    }
                }
            }
        }
    }

    /// The demands of player `p`'s persons that name the city at `plot` are refused at once (the
    /// city razed by `p`). Called before the city goes.
    pub(crate) fn persons_city_razed(&mut self, p: PlayerId, plot: PlotIdx) {
        for x in self.players[p].persons.clone() {
            if self.persons.get(x).and_then(|person| person.demand).is_some_and(|dm| Self::demand_names_plot(dm.kind, plot)) {
                self.person_demand_refused(x);
            }
        }
    }

    /// Teams `t` and `other` made peace: demands of their persons to take a city of the other
    /// side are refused at once.
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
            for x in self.players[p].persons.clone() {
                let Some(DemandKind::TakeCity(plot)) = self.persons.get(x).and_then(|person| person.demand).map(|dm| dm.kind) else { continue };
                if self.persons_city_at(plot).is_some_and(|c| self.players[self.cities[c].owner].team == them) {
                    self.person_demand_refused(x);
                }
            }
        }
    }

    /// The turns left of person `x`'s demand (0 at its last turn).
    pub fn person_demand_turns_left(&self, x: PersonId) -> Option<i32> {
        self.persons[x].demand.map(|d| (d.deadline - self.game_turn()).max(0))
    }
}
