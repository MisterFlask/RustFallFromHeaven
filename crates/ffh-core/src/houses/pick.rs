//! The pick (docs/HOUSES.md, rules 5, 6, 9 and 14): the candidates drawn and rolled with the game
//! RNG, the offer built from them (a pure function, so the same struct serves the controller, the
//! human's decision and the screens), and the default pick.

use ffh_data::*;

use super::*;

/// One candidate as rolled (rule 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseCandidate {
    pub leader: LeaderHeadType,
    /// The leader's own alignment.
    pub alignment: Option<AlignmentType>,
    pub religion: Option<ReligionType>,
    pub sympathy: Option<PlayerId>,
    pub perk: TraitType,
    /// The outsider of a foreign House's pick (rule 5): from the owner's civilization.
    pub outsider: bool,
}

impl HouseCandidate {
    pub fn leaning(&self) -> Leaning {
        Leaning { alignment: self.alignment, religion: self.religion, sympathy: self.sympathy, outsider: self.outsider }
    }
}

/// A candidate's card (rule 5, "Each candidate shows"): what it was rolled with and what its
/// opinion and happiness would be now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseCard {
    pub candidate: HouseCandidate,
    /// `Tovin Cinderwright`.
    pub name: String,
    /// `The Auditor of Oaths`, when the leader's civilopedia gives one.
    pub epithet: Option<String>,
    pub opinion: OpinionParts,
    pub state: HouseState,
    /// The happiness it would give each of the forming cities now (its state's plus the
    /// outsider's).
    pub happiness: i32,
    /// The card's lines, bare: `Alignment: Neutral`, `Religion: Runes of Kilmorph`, `Sympathy:
    /// the Balseraphs`, `Perk: Horsemasters I`, the outsider's line, `Opinion now: Annoyed (-3)`,
    /// its parts (`Alignment: -2`, ...), `Happiness: -2 in 3 cities.`.
    pub lines: Vec<String>,
    /// The perk's effects, bare, one line each (`Mounted Units trained: Flanking I.`).
    pub perk_lines: Vec<String>,
}

/// A House's forming offered to its owner (rule 5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseOffer {
    pub owner: PlayerId,
    /// The name it takes, without "House".
    pub name: String,
    pub origin: CivilizationType,
    /// Of another civilization than the owner's: the last card is the outsider's, if any.
    pub foreign: bool,
    /// The forming cities, the seat first.
    pub cities: Vec<CityId>,
    /// The perk tier rolled.
    pub tier: i32,
    /// The plot culture in the forming cities' work radius by player, percent of the total
    /// there, highest first (ties by player id; players with none left out). The sympathy roll's
    /// shares: `Culture here: the Balseraphs 62%, yours 30%, the Lanun 8%.`
    pub culture: Vec<(PlayerId, i32)>,
    pub cards: Vec<HouseCard>,
}

impl HouseOffer {
    /// The dialog's head (bare): `House Aldric forms with Ventry, Corona and Astrum.` and the
    /// culture there, `Culture here: the Balseraphs 62%, yours 30%, the Lanun 8%.`
    pub fn head(&self, g: &Game) -> Vec<String> {
        let names: Vec<String> = self.cities.iter().map(|&c| g.d.text.name(&g.cities[c].name).to_string()).collect();
        let mut out = vec![format!("House {} forms with {}.", self.name, and_list(&names))];
        if !self.culture.is_empty() {
            let parts: Vec<String> = self.culture.iter().map(|&(q, pct)| if q == self.owner { format!("yours {pct}%") } else { format!("{} {pct}%", g.house_civ_name(q)) }).collect();
            out.push(format!("Culture here: {}.", parts.join(", ")));
        }
        out
    }
}

/// The default pick's score of a card (rule 14): `HOUSE_PICK_PERK_VALUE` (0 where it would be
/// Furious now) plus `HOUSE_PICK_HAPPY_VALUE` times the happiness it would give now.
pub fn house_pick_score(d: &GameData, card: &HouseCard) -> i32 {
    let perk = if card.state == HouseState::Furious { 0 } else { d.defines.house_pick_perk_value };
    perk + d.defines.house_pick_happy_value * card.happiness
}

/// The default pick (rule 14): the card with the highest [`house_pick_score`], a tie to the
/// first. Every controller without its own answer picks by it.
pub fn house_pick_default(d: &GameData, offer: &HouseOffer) -> usize {
    let mut best = 0;
    let mut best_score = i32::MIN;
    for (i, card) in offer.cards.iter().enumerate() {
        let s = house_pick_score(d, card);
        if s > best_score {
            best = i;
            best_score = s;
        }
    }
    best
}

impl Game {
    /// `the Balseraphs`: a civilization as the Houses' lines name it.
    pub fn house_civ_name(&self, p: PlayerId) -> String {
        format!("the {}", self.player_get_civilization_short_description(p))
    }

    /// Whether minor leader `l` leads a living player.
    fn leader_leads_living(&self, l: LeaderHeadType) -> bool {
        self.player_ids().any(|p| self.players[p].alive && self.player_get_leader_type(p) == Some(l))
    }

    /// Whether `l` is a free minor leader (rule 5): a minor leader leading no living player and
    /// no House in the game.
    pub fn house_leader_is_free(&self, l: LeaderHeadType) -> bool {
        self.d.leader_heads[l].minor_leader_of.is_some() && !self.leader_leads_living(l) && !self.houses.iter().any(|(_, h)| h.leader == l)
    }

    /// The alignments of `civ`'s major leaders: the leaders it lists that are no minor leader
    /// (Decius, listed under three civilizations, does not count).
    fn civ_major_alignments(&self, civ: CivilizationType) -> Vec<AlignmentType> {
        let d = self.d;
        let decius = d.ty::<LeaderHeadType>("LEADER_DECIUS");
        let mut out: Vec<AlignmentType> = d
            .leader_heads
            .keys()
            .filter(|&l| d.civilizations[civ].leaders[l] && d.leader_heads[l].minor_leader_of.is_none() && Some(l) != decius)
            .filter_map(|l| d.leader_heads[l].alignment)
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The free minor leaders a House of civilization `origin` draws from, in leader id order:
    /// (own) `origin`'s, and (others) other civilizations' whose alignment is within one step of
    /// a major leader of `origin`.
    pub fn house_leader_pools(&self, origin: CivilizationType) -> (Vec<LeaderHeadType>, Vec<LeaderHeadType>) {
        let d = self.d;
        let majors = self.civ_major_alignments(origin);
        let mut own = Vec::new();
        let mut others = Vec::new();
        for l in d.leader_heads.keys() {
            let Some(civ) = d.leader_heads[l].minor_leader_of else { continue };
            if !self.house_leader_is_free(l) {
                continue;
            }
            if civ == origin {
                own.push(l);
            } else if majors.iter().any(|&m| alignment_steps(Some(m), d.leader_heads[l].alignment).is_some_and(|s| s <= 1)) {
                others.push(l);
            }
        }
        (own, others)
    }

    /// Whether a House of civilization `origin` would find a candidate now.
    pub fn house_can_find_leader(&self, origin: CivilizationType) -> bool {
        let (own, others) = self.house_leader_pools(origin);
        !own.is_empty() || !others.is_empty()
    }

    /// Draws `n` from `pool` without replacement (the game RNG; a pool of one needs no draw).
    fn house_draw(&mut self, pool: &mut Vec<LeaderHeadType>, n: usize, out: &mut Vec<LeaderHeadType>) {
        while out.len() < n && !pool.is_empty() {
            let i = if pool.len() == 1 { 0 } else { self.soren_rand_num(pool.len() as i32, "Houses: candidate") as usize };
            out.push(pool.remove(i));
        }
    }

    /// The plots in the work radius of `cities`, each once, in plot order.
    fn house_radius_plots(&self, cities: &[CityId]) -> Vec<PlotIdx> {
        let mut plots = Vec::new();
        for &c in cities {
            for i in 0..self.city_num_city_plots(c).max(0) as usize {
                if let Some(q) = self.city_index_plot(c, i) {
                    plots.push(q);
                }
            }
        }
        plots.sort();
        plots.dedup();
        plots
    }

    /// The plot culture of each player over the work radius of `cities`, and its total.
    fn house_radius_culture(&self, cities: &[CityId]) -> (Vec<i64>, i64) {
        let mut by = vec![0i64; self.num_players()];
        for q in self.house_radius_plots(cities) {
            for (i, &v) in self.map.plots[q].culture.iter().enumerate() {
                by[i] += v.max(0) as i64;
            }
        }
        let total = by.iter().sum();
        (by, total)
    }

    /// The culture shares of the offer's head ([`HouseOffer::culture`]).
    fn house_culture_shares(&self, cities: &[CityId]) -> Vec<(PlayerId, i32)> {
        let (by, total) = self.house_radius_culture(cities);
        if total == 0 {
            return Vec::new();
        }
        let mut out: Vec<(PlayerId, i32)> = self.player_ids().filter(|q| by[q.idx()] > 0).map(|q| (q, (by[q.idx()] * 100 / total) as i32)).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// A weighted draw among `weights` (the game RNG); the first on a zero total.
    fn house_weighted(&mut self, weights: &[i64], what: &str) -> usize {
        let total: i64 = weights.iter().sum();
        if total <= 0 {
            return 0;
        }
        let mut r = self.soren_rand_num(total.min(i32::MAX as i64) as i32, what) as i64;
        for (i, &w) in weights.iter().enumerate() {
            if r < w {
                return i;
            }
            r -= w;
        }
        weights.len() - 1
    }

    /// Rolls a candidate's religion (rule 6): each religion admitting `alignment` 10, plus 30
    /// where one of `cities` has it; none 20.
    fn house_roll_religion(&mut self, alignment: Option<AlignmentType>, cities: &[CityId]) -> Option<ReligionType> {
        let d = self.d;
        let x = &d.defines;
        let mut options: Vec<Option<ReligionType>> = vec![None];
        let mut weights: Vec<i64> = vec![x.house_religion_none_weight as i64];
        for r in d.religions.keys() {
            if !religion_admits(d, r, alignment) {
                continue;
            }
            let present = cities.iter().any(|&c| self.cities[c].has_religion[r]);
            options.push(Some(r));
            weights.push((x.house_religion_weight + if present { x.house_religion_present_weight } else { 0 }) as i64);
        }
        let i = self.house_weighted(&weights, "Houses: religion");
        options[i]
    }

    /// The civilizations a leader of `alignment` under `owner` may favour (rule 6): the living
    /// players `owner` has met, of another team, that have Houses and are not two alignment steps
    /// from it, in player order.
    pub fn house_sympathy_candidates(&self, owner: PlayerId, alignment: Option<AlignmentType>) -> Vec<PlayerId> {
        let team = self.players[owner].team;
        self.player_ids()
            .filter(|&q| {
                let pl = &self.players[q];
                pl.alive && pl.team != team && self.teams[team].has_met[pl.team] && self.player_has_houses(q) && alignment_steps(alignment, pl.alignment).map_or(true, |s| s < 2)
            })
            .collect()
    }

    /// The sympathy roll's weights (rule 6), none first then each candidate:
    /// `20 + 100·C(owner)/T` and `5 + 100·C(X)/T` (`T` zero: 20 and 5).
    pub fn house_sympathy_weights(&self, owner: PlayerId, candidates: &[PlayerId], cities: &[CityId]) -> Vec<i64> {
        let x = &self.d.defines;
        let (by, total) = self.house_radius_culture(cities);
        let share = |q: PlayerId| if total == 0 { 0 } else { x.house_sympathy_culture_weight as i64 * by[q.idx()] / total };
        let mut weights = vec![x.house_sympathy_none_weight as i64 + share(owner)];
        weights.extend(candidates.iter().map(|&q| x.house_sympathy_weight as i64 + share(q)));
        weights
    }

    fn house_roll_sympathy(&mut self, owner: PlayerId, alignment: Option<AlignmentType>, cities: &[CityId]) -> Option<PlayerId> {
        let candidates = self.house_sympathy_candidates(owner, alignment);
        let weights = self.house_sympathy_weights(owner, &candidates, cities);
        let i = self.house_weighted(&weights, "Houses: sympathy");
        if i == 0 {
            None
        } else {
            Some(candidates[i - 1])
        }
    }

    /// The perks of `tier` a House forming with `cities` can use (rule 9): the coastal family
    /// only where one of them is coastal; in table order.
    pub fn house_perk_options(&self, tier: i32, cities: &[CityId]) -> Vec<TraitType> {
        let d = self.d;
        let coastal = cities.iter().any(|&c| self.city_is_coastal(c, d.defines.min_water_size_for_ocean));
        d.house_perks(tier).iter().copied().filter(|&t| coastal || !d.traits[t].house_requires_coastal).collect()
    }

    fn house_roll_perk(&mut self, tier: i32, cities: &[CityId], taken: &[TraitType]) -> TraitType {
        let d = self.d;
        let all = self.house_perk_options(tier, cities);
        let mut options: Vec<TraitType> = all.iter().copied().filter(|&t| taken.iter().all(|&o| d.traits[o].house_perk_family != d.traits[t].house_perk_family)).collect();
        if options.is_empty() {
            options = all;
        }
        let i = if options.len() == 1 { 0 } else { self.soren_rand_num(options.len() as i32, "Houses: perk") as usize };
        options[i]
    }

    /// Draws and rolls the candidates of a House of `owner` forming with `cities` (seat first)
    /// of civilization `origin` (rules 5, 6, 9), with the game RNG in the contract's order: the
    /// candidates, the outsider, then for each in order religion, sympathy, perk. Empty when no
    /// leader is free.
    pub fn house_roll_candidates(&mut self, owner: PlayerId, origin: CivilizationType, cities: &[CityId], tier: i32) -> Vec<HouseCandidate> {
        let d = self.d;
        let n = d.defines.house_candidates.max(1) as usize;
        let (mut own, mut others) = self.house_leader_pools(origin);
        let mut leaders = Vec::new();
        self.house_draw(&mut own, n, &mut leaders);
        self.house_draw(&mut others, n, &mut leaders);
        if leaders.is_empty() {
            return Vec::new();
        }
        // The outsider of a foreign House: the free minor leader of the owner's civilization
        // whose alignment is nearest the owner's (a tie by the game RNG).
        let own_civ = self.player_get_civilization_type(owner);
        let mut outsider = None;
        if own_civ.is_some_and(|c| c != origin) {
            let a = self.players[owner].alignment;
            let pool: Vec<LeaderHeadType> = d.leader_heads.keys().filter(|&l| d.leader_heads[l].minor_leader_of == own_civ && !leaders.contains(&l) && self.house_leader_is_free(l)).collect();
            let dist = |l: LeaderHeadType| alignment_steps(a, d.leader_heads[l].alignment).unwrap_or(0);
            if let Some(best) = pool.iter().map(|&l| dist(l)).min() {
                let nearest: Vec<LeaderHeadType> = pool.into_iter().filter(|&l| dist(l) == best).collect();
                let i = if nearest.len() == 1 { 0 } else { self.soren_rand_num(nearest.len() as i32, "Houses: outsider") as usize };
                outsider = Some(nearest[i]);
            }
        }
        let mut out: Vec<HouseCandidate> = Vec::new();
        let mut taken: Vec<TraitType> = Vec::new();
        for l in leaders {
            let alignment = d.leader_heads[l].alignment;
            let religion = self.house_roll_religion(alignment, cities);
            let sympathy = self.house_roll_sympathy(owner, alignment, cities);
            let perk = self.house_roll_perk(tier, cities, &taken);
            taken.push(perk);
            out.push(HouseCandidate { leader: l, alignment, religion, sympathy, perk, outsider: false });
        }
        if let Some(l) = outsider {
            let alignment = d.leader_heads[l].alignment;
            let religion = self.player_get_state_religion(owner).filter(|&r| religion_admits(d, r, alignment));
            let perk = self.house_roll_perk(tier, cities, &taken);
            out.push(HouseCandidate { leader: l, alignment, religion, sympathy: None, perk, outsider: true });
        }
        out
    }

    /// A candidate's card under `owner` for `cities`, as the game is now (pure).
    pub fn house_card(&self, owner: PlayerId, candidate: &HouseCandidate, cities: usize) -> HouseCard {
        let d = self.d;
        let leaning = candidate.leaning();
        let opinion = self.house_opinion_of(owner, &leaning, None);
        let state = HouseState::of(d, opinion.total());
        let (s, o) = self.house_happiness_terms_of(owner, &leaning, None);
        let happiness = s + o;
        let mut lines = self.house_leaning_lines(&leaning, candidate.perk);
        lines.push(format!("Opinion now: {} ({})", state.name(), opinion.total()));
        lines.extend(self.house_opinion_part_lines(owner, &leaning, &opinion));
        lines.push(format!("Happiness: {} in {cities} {}.", signed(happiness), if cities == 1 { "city" } else { "cities" }));
        HouseCard {
            candidate: candidate.clone(),
            name: leader_name(d, candidate.leader),
            epithet: leader_epithet(d, candidate.leader),
            opinion,
            state,
            happiness,
            lines,
            perk_lines: self.house_perk_lines(candidate.perk),
        }
    }

    /// The offer of a House of `owner` forming with `cities` (seat first) of `origin`, named
    /// `name`, from candidates already rolled (pure).
    pub fn house_offer(&self, owner: PlayerId, name: String, origin: CivilizationType, cities: Vec<CityId>, tier: i32, candidates: &[HouseCandidate]) -> HouseOffer {
        let cards = candidates.iter().map(|c| self.house_card(owner, c, cities.len())).collect();
        HouseOffer {
            owner,
            name,
            origin,
            foreign: Some(origin) != self.player_get_civilization_type(owner),
            culture: self.house_culture_shares(&cities),
            cities,
            tier,
            cards,
        }
    }
}

/// `+2`, `0`, `-3`.
pub(crate) fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}
