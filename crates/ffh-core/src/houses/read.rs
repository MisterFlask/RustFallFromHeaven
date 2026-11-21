//! The reads for the screens (docs/HOUSES.md, rule 12): a House's labelled parts, state and
//! `*_calc` lines, its happiness by city, its perk and whether it is on, a summary of every House
//! of a player, the cities in no House and why, and the previews of an act. All pure.

use ffh_data::*;

use super::pick::signed;
use super::*;

/// One labelled part of an opinion: `Alignment` with its detail and value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpinionPart {
    /// `alignment`, `religion`, `sympathy`, `wars`.
    pub key: &'static str,
    /// The line, bare: `Alignment: Neutral, yours Good: -2`.
    pub label: String,
    pub value: i32,
}

/// Everything a screen shows of one House.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseSummary {
    pub id: HouseId,
    pub owner: PlayerId,
    /// `House Aldric`.
    pub label: String,
    pub leader: LeaderHeadType,
    pub leader_name: String,
    pub epithet: Option<String>,
    pub origin: CivilizationType,
    pub foreign: bool,
    pub outsider: bool,
    pub alignment: Option<AlignmentType>,
    pub religion: Option<ReligionType>,
    pub sympathy: Option<PlayerId>,
    pub perk: TraitType,
    pub perk_name: String,
    /// The perk's effects, bare (`Mounted Units trained: Flanking I.`).
    pub perk_lines: Vec<String>,
    /// False while the House is Furious.
    pub perk_on: bool,
    pub formed_turn: i32,
    /// Its cities in join order, the seat first.
    pub cities: Vec<CityId>,
    pub opinion: i32,
    pub state: HouseState,
    pub parts: Vec<OpinionPart>,
    /// The happiness in each of its cities (its state's plus the outsider's).
    pub happiness: i32,
    /// The card's lines (as a candidate's card: alignment, religion, sympathy, perk, outsider).
    pub lines: Vec<String>,
    /// How its opinion and happiness are worked out.
    pub calc: Vec<String>,
}

/// A city of a player that has Houses in no House, and why (`No House yet: the first forms at 3
/// cities (2 of 3).`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unhoused {
    pub city: CityId,
    pub reason: String,
}

/// A House whose state an act would change (rule 12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HousePreview {
    pub house: HouseId,
    pub before: HouseState,
    pub after: HouseState,
    pub opinion_before: i32,
    pub opinion_after: i32,
    /// The happiness in each of its cities, before and after.
    pub happiness_before: i32,
    pub happiness_after: i32,
    pub cities: usize,
    /// `House Aldric: Pleased to Annoyed. Happiness -3 in 4 cities.`, then `Its perk goes off.`
    /// (or `Its perk is on again.`) where that changes.
    pub lines: Vec<String>,
}

impl Game {
    /// A House's or a candidate's lines (bare): alignment, religion, sympathy, perk, and the
    /// outsider's line.
    pub fn house_leaning_lines(&self, leaning: &Leaning, perk: TraitType) -> Vec<String> {
        let d = self.d;
        let mut lines = vec![format!("Alignment: {}", alignment_name(d, leaning.alignment)), format!("Religion: {}", religion_name(d, leaning.religion))];
        if leaning.outsider {
            lines.push(format!("Outsider: {} happiness in this House's cities. No sympathy.", d.defines.house_outsider_happy));
        } else {
            lines.push(format!("Sympathy: {}", leaning.sympathy.map_or_else(|| "none".to_string(), |s| self.house_civ_name(s))));
        }
        lines.push(format!("Perk: {}", perk_name(d, perk)));
        lines
    }

    /// The labelled parts of an opinion of `leaning` toward `owner` (the outsider's (c) and (d)
    /// left out), for parts as `house_opinion_of` gave them.
    pub fn house_opinion_part_list(&self, owner: PlayerId, leaning: &Leaning, parts: &OpinionParts) -> Vec<OpinionPart> {
        let d = self.d;
        let mut out = Vec::new();
        out.push(OpinionPart {
            key: "alignment",
            label: format!("Alignment: {}, yours {}: {}", alignment_name(d, leaning.alignment), alignment_name(d, self.players[owner].alignment), signed(parts.alignment)),
            value: parts.alignment,
        });
        let state = self.player_get_state_religion(owner);
        let religion = match (leaning.religion, state) {
            (Some(r), Some(s)) if r == s => format!("Religion: {}, your state religion: {}", religion_name(d, Some(r)), signed(parts.religion)),
            (Some(r), Some(s)) => format!("Religion: {}, yours {}: {}", religion_name(d, Some(r)), religion_name(d, Some(s)), signed(parts.religion)),
            (Some(r), None) => format!("Religion: {}, you have no state religion: 0", religion_name(d, Some(r))),
            (None, _) => "Religion: none: 0".to_string(),
        };
        out.push(OpinionPart { key: "religion", label: religion, value: parts.religion });
        if leaning.outsider {
            return out;
        }
        let sympathy = match leaning.sympathy {
            None => "Sympathy: none: 0".to_string(),
            Some(s) if !self.players[s].alive => format!("Sympathy: {}, gone: 0", self.house_civ_name(s)),
            Some(s) => {
                let relation = match parts.sympathy {
                    v if v == d.defines.house_opinion_sympathy_war && v != 0 => "at war with you",
                    v if v == d.defines.house_opinion_sympathy_pact && v != 0 => "your ally",
                    v if v == d.defines.house_opinion_sympathy_open_borders && v != 0 => "open borders with you",
                    _ => "no treaty with you",
                };
                format!("Sympathy: {}, {relation}: {}", self.house_civ_name(s), signed(parts.sympathy))
            }
        };
        out.push(OpinionPart { key: "sympathy", label: sympathy, value: parts.sympathy });
        out.push(OpinionPart { key: "wars", label: format!("Your wars: {}", signed(parts.wars)), value: parts.wars });
        out
    }

    /// The part lines of a card (`Alignment: Neutral, yours Good: -2`).
    pub(crate) fn house_opinion_part_lines(&self, owner: PlayerId, leaning: &Leaning, parts: &OpinionParts) -> Vec<String> {
        self.house_opinion_part_list(owner, leaning, parts).into_iter().map(|p| p.label).collect()
    }

    /// House `h`'s labelled opinion parts now.
    pub fn house_opinion_parts(&self, h: HouseId) -> Vec<OpinionPart> {
        let house = &self.houses[h];
        self.house_opinion_part_list(house.owner, &house.leaning(), &self.house_opinion_split(h))
    }

    /// How House `h`'s opinion, state and happiness are worked out (docs/PLAY_DESIGN.md 4.9):
    /// each part, `Opinion: +1 + 2 - 3 + 0 = 0: Cautious.`, `Happiness: 0 (Cautious) - 3
    /// (outsider) = -3 in each of its 3 cities.`
    pub fn house_opinion_calc(&self, h: HouseId) -> Vec<String> {
        let house = &self.houses[h];
        self.house_calc_lines(&self.house_opinion_parts(h), house.outsider, house.cities.len())
    }

    /// How a candidate's opinion and happiness would be worked out now (as
    /// [`Game::house_opinion_calc`] for a House), for `cities` forming cities.
    pub fn house_card_calc(&self, owner: PlayerId, card: &HouseCard, cities: usize) -> Vec<String> {
        let parts = self.house_opinion_part_list(owner, &card.candidate.leaning(), &card.opinion);
        self.house_calc_lines(&parts, card.candidate.outsider, cities)
    }

    /// The calculation's lines from labelled parts: each part, the sum and its state, the states'
    /// bounds, the happiness in `n` cities (the outsider's apart).
    fn house_calc_lines(&self, parts: &[OpinionPart], outsider: bool, n: usize) -> Vec<String> {
        let total: i32 = parts.iter().map(|p| p.value).sum();
        let state = HouseState::of(self.d, total);
        let mut lines: Vec<String> = parts.iter().map(|p| p.label.clone()).collect();
        // `+1 + 2 - 3 + 0`: the first signed, then each with its operator.
        let mut sum = String::new();
        for (i, p) in parts.iter().enumerate() {
            match i {
                0 => sum.push_str(&signed(p.value)),
                _ if p.value < 0 => sum.push_str(&format!(" - {}", -p.value)),
                _ => sum.push_str(&format!(" + {}", p.value)),
            }
        }
        lines.push(format!("Opinion: {sum} = {total}: {}.", state.name()));
        lines.push(format!(
            "States: Friendly {} or more, Pleased {} to {}, Cautious {} to {}, Annoyed {} to {}, Furious {} or less.",
            self.d.defines.house_opinion_friendly,
            self.d.defines.house_opinion_pleased,
            self.d.defines.house_opinion_friendly - 1,
            self.d.defines.house_opinion_cautious,
            self.d.defines.house_opinion_pleased - 1,
            self.d.defines.house_opinion_annoyed,
            self.d.defines.house_opinion_cautious - 1,
            self.d.defines.house_opinion_annoyed - 1,
        ));
        let cities = if n == 1 { "its city".to_string() } else { format!("each of its {n} cities") };
        let st = state.happiness(self.d);
        if outsider {
            let o = self.d.defines.house_outsider_happy;
            lines.push(format!("Happiness: {} ({}) {} (outsider) = {} in {cities}.", signed(st), state.name(), signed(o), signed(st + o)));
        } else {
            lines.push(format!("Happiness: {} ({}) in {cities}.", signed(st), state.name()));
        }
        lines
    }

    /// Whether House `h`'s perk is on (it is not Furious).
    pub fn house_perk_on(&self, h: HouseId) -> bool {
        self.house_state(h) != HouseState::Furious
    }

    /// The happiness by city of House `h`: each city and the happiness its House gives it.
    pub fn house_happiness_by_city(&self, h: HouseId) -> Vec<(CityId, i32)> {
        let n = self.house_happiness(h);
        self.houses[h].cities.iter().map(|&c| (c, n)).collect()
    }

    /// City `c`'s happiness lines from its House, for its happiness list: (`House Aldric is
    /// Annoyed`, -2) when its state gives any, (`Ruled by an outsider`, -3) for an outsider.
    pub fn city_house_happiness_parts(&self, c: CityId) -> Vec<(String, i32)> {
        let mut out = Vec::new();
        let Some(h) = self.cities[c].house.filter(|&h| self.houses.contains(h)) else { return out };
        let (state, outsider) = self.city_house_happiness_terms(c);
        if state != 0 {
            out.push((format!("{} is {}", self.house_label(h), self.house_state(h).name()), state));
        }
        if outsider != 0 {
            out.push(("Ruled by an outsider".to_string(), outsider));
        }
        out
    }

    /// City `c`'s Governance line: `House Aldric (Tovin Cinderwright): Pleased, +1 happiness.
    /// Perk: Horsemasters I.` (`Perk: Horsemasters I (off).` while Furious); `None` in no House.
    pub fn city_house_line(&self, c: CityId) -> Option<String> {
        let h = self.cities[c].house.filter(|&h| self.houses.contains(h))?;
        let house = &self.houses[h];
        let d = self.d;
        let off = if self.house_perk_on(h) { "" } else { " (off)" };
        Some(format!(
            "{} ({}): {}, {} happiness. Perk: {}{off}.",
            self.house_label(h),
            leader_name(d, house.leader),
            self.house_state(h).name(),
            signed(self.house_happiness(h)),
            perk_name(d, house.perk)
        ))
    }

    /// A perk's effects, bare, one line each, from its `TraitInfo` keys (rule 9).
    pub fn house_perk_lines(&self, t: TraitType) -> Vec<String> {
        perk_lines(self.d, t)
    }

    /// Everything a screen shows of House `h`.
    pub fn house_summary(&self, h: HouseId) -> HouseSummary {
        let d = self.d;
        let house = &self.houses[h];
        let leaning = house.leaning();
        let opinion = self.house_opinion(h);
        let state = HouseState::of(d, opinion);
        HouseSummary {
            id: h,
            owner: house.owner,
            label: self.house_label(h),
            leader: house.leader,
            leader_name: leader_name(d, house.leader),
            epithet: leader_epithet(d, house.leader),
            origin: house.origin,
            foreign: self.house_is_foreign(h),
            outsider: house.outsider,
            alignment: house.alignment,
            religion: house.religion,
            sympathy: house.sympathy,
            perk: house.perk,
            perk_name: perk_name(d, house.perk),
            perk_lines: self.house_perk_lines(house.perk),
            perk_on: state != HouseState::Furious,
            formed_turn: house.formed_turn,
            cities: house.cities.clone(),
            opinion,
            state,
            parts: self.house_opinion_parts(h),
            happiness: self.house_happiness(h),
            lines: self.house_leaning_lines(&leaning, house.perk),
            calc: self.house_opinion_calc(h),
        }
    }

    /// Player `p`'s Houses, summarized, in the order they formed.
    pub fn player_house_summaries(&self, p: PlayerId) -> Vec<HouseSummary> {
        self.players[p].houses.iter().filter(|&&h| self.houses.contains(h)).map(|&h| self.house_summary(h)).collect()
    }

    /// Player `p`'s cities in no House (its outposts left out), in its city order, and why.
    pub fn player_unhoused_cities(&self, p: PlayerId) -> Vec<Unhoused> {
        if !self.player_has_houses(p) {
            return Vec::new();
        }
        let first = self.d.defines.house_first_cities;
        let waiting = self.player_unhoused_home_cities(p).len();
        let home_house = self.player_has_home_house(p);
        self.players[p]
            .cities
            .iter()
            .filter(|&&c| !self.cities[c].outpost && self.cities[c].house.is_none())
            .map(|&c| {
                let reason = if self.city_is_home(c) && !home_house && (waiting as i32) < first {
                    format!("No House yet: the first forms at {first} cities ({waiting} of {first}).")
                } else if self.city_origin(c).is_some_and(|o| !self.house_can_find_leader(o)) {
                    "No House yet: no leader is free.".to_string()
                } else {
                    "No House yet: it joins or forms one at your next turn.".to_string()
                };
                Unhoused { city: c, reason }
            })
            .collect()
    }

    /// What act `act` of House owner `p` would do to its Houses (pure): each House whose state
    /// would change, before and after, in the order they formed.
    pub fn house_preview(&self, p: PlayerId, act: HouseAct) -> Vec<HousePreview> {
        let d = self.d;
        let mut out = Vec::new();
        for &h in &self.players[p].houses {
            let Some(house) = self.houses.get(h) else { continue };
            let leaning = house.leaning();
            let opinion_before = self.house_opinion_of(p, &leaning, None).total();
            let opinion_after = self.house_opinion_of(p, &leaning, Some(act)).total();
            let (before, after) = (HouseState::of(d, opinion_before), HouseState::of(d, opinion_after));
            if before == after {
                continue;
            }
            let outsider = if house.outsider { d.defines.house_outsider_happy } else { 0 };
            let (happiness_before, happiness_after) = (before.happiness(d) + outsider, after.happiness(d) + outsider);
            let n = house.cities.len();
            let mut lines = vec![format!(
                "{}: {} to {}. Happiness {} in {n} {}.",
                self.house_label(h),
                before.name(),
                after.name(),
                signed(happiness_after - happiness_before),
                if n == 1 { "city" } else { "cities" }
            )];
            if after == HouseState::Furious {
                lines.push("Its perk goes off.".to_string());
            } else if before == HouseState::Furious {
                lines.push("Its perk is on again.".to_string());
            }
            out.push(HousePreview { house: h, before, after, opinion_before, opinion_after, happiness_before, happiness_after, cities: n, lines });
        }
        out
    }
}

/// A perk's effects, bare, one line each, from its `TraitInfo` keys (rule 9); pure (the data
/// only), for the help's trait text as for the screens.
pub fn perk_lines(d: &GameData, t: TraitType) -> Vec<String> {
    let info = &d.traits[t];
    let name = |key: &str| d.text.name(key).to_string();
    let pct = |n: i32| if n > 0 { format!("+{n}%") } else { format!("{n}%") };
    let mut out = Vec::new();
    for y in d.yields.keys() {
        let n = info.house_yield_modifier[y.idx()];
        if n != 0 {
            out.push(format!("{} {}.", name(&d.yields[y].base.description), pct(n)));
        }
    }
    for k in d.commerces.keys() {
        let n = info.house_commerce_modifier[k.idx()];
        if n != 0 {
            out.push(format!("{} {}.", name(&d.commerces[k].base.description), pct(n)));
        }
    }
    if info.house_great_people_rate_modifier != 0 {
        out.push(format!("Great people rate {}.", pct(info.house_great_people_rate_modifier)));
    }
    if info.house_distance_maintenance_modifier != 0 {
        out.push(format!("Distance maintenance {}.", pct(info.house_distance_maintenance_modifier)));
    }
    if info.house_num_cities_maintenance_modifier != 0 {
        out.push(format!("City-count maintenance {}.", pct(info.house_num_cities_maintenance_modifier)));
    }
    if info.house_free_experience != 0 {
        out.push(format!("Units trained: {} experience.", signed(info.house_free_experience)));
    }
    for c in d.unit_combats.keys() {
        let combat = name(&d.unit_combats[c].base.description);
        let promotions = &info.house_unit_combat_free_promotions[c];
        if !promotions.is_empty() {
            let names: Vec<String> = promotions.iter().map(|&p| name(&d.promotions[p].base.description)).collect();
            out.push(format!("{combat} trained: {}.", names.join(", ")));
        }
        let xp = info.house_unit_combat_free_experience[c];
        if xp != 0 {
            out.push(format!("{combat} trained: {} experience.", signed(xp)));
        }
        let prod = info.house_unit_combat_production_modifier[c];
        if prod != 0 {
            out.push(format!("{combat}: production {}.", pct(prod)));
        }
    }
    if info.house_wonder_production_modifier != 0 {
        out.push(format!("Wonder production {}.", pct(info.house_wonder_production_modifier)));
    }
    if info.house_ritual_production_modifier != 0 {
        out.push(format!("Ritual production {}.", pct(info.house_ritual_production_modifier)));
    }
    out
}
