//! Opinion, state and happiness (docs/HOUSES.md, rules 7 and 8). Nothing is stored: every read
//! sums the parts from the game as it is (or as a [`HouseAct`] would leave it, for previews).
//! The happiness reads are on the city's hot path, so the sum allocates nothing.

use ffh_data::*;

use super::*;

/// What a leader's opinion depends on: a House's, or a candidate's before it leads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Leaning {
    pub alignment: Option<AlignmentType>,
    pub religion: Option<ReligionType>,
    pub sympathy: Option<PlayerId>,
    pub outsider: bool,
}

/// An act whose effect on the owner's Houses is previewed (rule 12). A pact is a defensive pact.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HouseAct {
    /// The owner declares war on team `t` (and so on `t`'s vassals); its open borders and pact
    /// with them end.
    DeclareWar(TeamId),
    /// The owner makes peace with team `t`.
    MakePeace(TeamId),
    /// Open borders with team `t` signed (`true`) or ended.
    OpenBorders(TeamId, bool),
    /// A defensive pact with team `t` signed (`true`) or ended.
    DefensivePact(TeamId, bool),
    /// A new state religion (`None`: none); the owner's alignment moves into its range, as
    /// `player_convert` moves it.
    Convert(Option<ReligionType>),
}

/// The four parts of an opinion (rule 7); an outsider has (a) and (b) only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct OpinionParts {
    /// (a) alignment against the owner's.
    pub alignment: i32,
    /// (b) religion against the owner's state religion.
    pub religion: i32,
    /// (c) the owner's relations with the civilization the leader favours.
    pub sympathy: i32,
    /// (d) the owner's wars, by the enemies' alignment.
    pub wars: i32,
}

impl OpinionParts {
    #[inline]
    pub fn total(&self) -> i32 {
        self.alignment + self.religion + self.sympathy + self.wars
    }
}

/// The relations of the owner a preview may change, read through the act.
pub(crate) struct View<'a> {
    g: &'a Game,
    team: TeamId,
    act: Option<HouseAct>,
}

impl View<'_> {
    #[inline]
    fn at_war(&self, t: TeamId) -> bool {
        match self.act {
            Some(HouseAct::DeclareWar(x)) if t == x || self.g.teams[t].vassal[x] => t != self.team,
            Some(HouseAct::MakePeace(x)) if t == x => false,
            _ => self.g.teams[self.team].at_war[t],
        }
    }

    #[inline]
    fn defensive_pact(&self, t: TeamId) -> bool {
        match self.act {
            Some(HouseAct::DeclareWar(_)) if self.at_war(t) => false,
            Some(HouseAct::DefensivePact(x, on)) if t == x => on,
            _ => self.g.teams[self.team].defensive_pact[t],
        }
    }

    #[inline]
    fn open_borders(&self, t: TeamId) -> bool {
        match self.act {
            Some(HouseAct::DeclareWar(_)) if self.at_war(t) => false,
            Some(HouseAct::OpenBorders(x, on)) if t == x => on,
            _ => self.g.teams[self.team].open_borders[t],
        }
    }
}

impl Game {
    /// Player `p`'s state religion, as act `act` would leave it.
    #[inline]
    fn house_state_religion(&self, p: PlayerId, act: Option<HouseAct>) -> Option<ReligionType> {
        match act {
            Some(HouseAct::Convert(r)) => r,
            _ => self.player_get_state_religion(p),
        }
    }

    /// Player `p`'s alignment, as act `act` would leave it (a conversion moves it into the new
    /// religion's range: `player_convert`).
    pub(crate) fn house_owner_alignment(&self, p: PlayerId, act: Option<HouseAct>) -> Option<AlignmentType> {
        let mut a = self.players[p].alignment;
        if let Some(HouseAct::Convert(Some(r))) = act {
            let info = &self.d.religions[r];
            if a < info.alignment_best {
                a = info.alignment_best;
            }
            if a > info.alignment_worst {
                a = info.alignment_worst;
            }
        }
        a
    }

    /// The opinion parts of a leader of `leaning` toward its ruler `owner`, the game as act
    /// `act` would leave it (`None`: as it is). Allocation-free.
    pub fn house_opinion_of(&self, owner: PlayerId, leaning: &Leaning, act: Option<HouseAct>) -> OpinionParts {
        let x = &self.d.defines;
        let mut parts = OpinionParts::default();
        // (a) Alignment.
        if let Some(steps) = alignment_steps(leaning.alignment, self.house_owner_alignment(owner, act)) {
            parts.alignment = if steps == 0 { x.house_opinion_alignment_same } else { x.house_opinion_alignment_step * steps };
        }
        // (b) Religion.
        if let (Some(r), Some(s)) = (leaning.religion, self.house_state_religion(owner, act)) {
            parts.religion = if r == s { x.house_opinion_religion_same } else { x.house_opinion_religion_other };
        }
        if leaning.outsider {
            return parts;
        }
        let view = View { g: self, team: self.players[owner].team, act };
        // (c) Sympathy, toward a living civilization.
        let favoured = leaning.sympathy.filter(|&s| self.players[s].alive);
        if let Some(s) = favoured {
            let t = self.players[s].team;
            parts.sympathy = if t != view.team && view.at_war(t) {
                x.house_opinion_sympathy_war
            } else if t == view.team || view.defensive_pact(t) || self.teams[t].vassal[view.team] || self.teams[view.team].vassal[t] {
                // A permanent alliance joins the two in one team.
                x.house_opinion_sympathy_pact
            } else if view.open_borders(t) {
                x.house_opinion_sympathy_open_borders
            } else {
                0
            };
        }
        // (d) Wars and alignment: each enemy civilization with a city, `X` of (c) left out.
        let mut wars = 0;
        for q in self.player_ids() {
            let pl = &self.players[q];
            if Some(q) == favoured || !pl.alive || pl.team == view.team || pl.cities.is_empty() || !self.player_has_houses(q) || !view.at_war(pl.team) {
                continue;
            }
            match alignment_steps(leaning.alignment, pl.alignment) {
                Some(0) => wars += x.house_opinion_war_same_alignment,
                Some(2) => wars += x.house_opinion_war_opposed_alignment,
                _ => {}
            }
        }
        let limit = x.house_opinion_wars_limit;
        parts.wars = wars.clamp(-limit, limit);
        parts
    }

    /// House `h`'s opinion parts now.
    #[inline]
    pub fn house_opinion_split(&self, h: HouseId) -> OpinionParts {
        let house = &self.houses[h];
        self.house_opinion_of(house.owner, &house.leaning(), None)
    }

    /// House `h`'s opinion of its ruler now (the sum of its parts). Allocation-free.
    #[inline]
    pub fn house_opinion(&self, h: HouseId) -> i32 {
        self.house_opinion_split(h).total()
    }

    /// House `h`'s state now.
    #[inline]
    pub fn house_state(&self, h: HouseId) -> HouseState {
        HouseState::of(self.d, self.house_opinion(h))
    }

    /// The happiness a House's state gives each of its cities, and the outsider's (0 or
    /// `HOUSE_OUTSIDER_HAPPY`), for `leaning` under `owner` (rule 8).
    #[inline]
    pub fn house_happiness_terms_of(&self, owner: PlayerId, leaning: &Leaning, act: Option<HouseAct>) -> (i32, i32) {
        let state = HouseState::of(self.d, self.house_opinion_of(owner, leaning, act).total());
        (state.happiness(self.d), if leaning.outsider { self.d.defines.house_outsider_happy } else { 0 })
    }

    /// The happiness in each city of House `h` (rule 8): its state's plus the outsider's.
    pub fn house_happiness(&self, h: HouseId) -> i32 {
        let house = &self.houses[h];
        let (state, outsider) = self.house_happiness_terms_of(house.owner, &house.leaning(), None);
        state + outsider
    }

    /// City `c`'s happiness terms from its House: (its state's, the outsider's); (0, 0) in no
    /// House. The happy and unhappy levels each take the side of their sign. Hot path.
    #[inline]
    pub fn city_house_happiness_terms(&self, c: CityId) -> (i32, i32) {
        let Some(h) = self.cities[c].house else { return (0, 0) };
        let Some(house) = self.houses.get(h) else { return (0, 0) };
        self.house_happiness_terms_of(house.owner, &house.leaning(), None)
    }

    /// City `c`'s happiness from its House (both terms).
    pub fn city_house_happiness(&self, c: CityId) -> i32 {
        let (a, b) = self.city_house_happiness_terms(c);
        a + b
    }
}
