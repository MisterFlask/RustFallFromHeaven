//! This game's Houses (docs/HOUSES.md, the owner's design of 2026-11-21): a civilization's cities
//! in groups of up to four under a minor leader with an alignment, a religion, a sympathy for
//! another civilization and a perk. The cities stay their owner's in every respect; the leader's
//! opinion of the ruler is happiness in the House's cities, and a Furious House's perk is off.
//!
//! - `mod.rs`: the record, who has Houses, a city's origin, names and the messages (rule 11).
//! - `membership.rs`: joining, leaving and forming (rule 4); the owner's turn pass.
//! - `pick.rs`: the candidates and their rolls (rules 5, 6, 9), the offer and the default pick
//!   (rule 14).
//! - `opinion.rs`: opinion, state and happiness (rules 7, 8), allocation-free for the happiness
//!   reads.
//! - `read.rs`: the reads for the screens (rule 12): labelled parts, `*_calc` lines, summaries,
//!   previews.

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::log::{LogMark, LogTag};

mod membership;
mod opinion;
mod perks;
mod pick;
mod read;

pub use opinion::{HouseAct, Leaning, OpinionParts};
pub use pick::{house_pick_default, house_pick_score, HouseCandidate, HouseCard, HouseOffer};
pub use read::{perk_lines, HousePreview, HouseSummary, OpinionPart, Unhoused};

/// A House (rule 2): a record of its owner, never a player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct House {
    pub id: HouseId,
    pub owner: PlayerId,
    /// Its name without "House" (`Aldric`); [`Game::house_label`] is `House Aldric`.
    pub name: String,
    /// A minor leader (`LeaderHeadInfo::minor_leader_of`), for good.
    pub leader: LeaderHeadType,
    /// The civilization of the city it formed with (rule 3); it takes only cities of it.
    pub origin: CivilizationType,
    /// The leader's own alignment, from the data.
    pub alignment: Option<AlignmentType>,
    pub religion: Option<ReligionType>,
    /// The civilization (player) it favours.
    pub sympathy: Option<PlayerId>,
    /// A House perk (`TraitInfo::is_house_perk`).
    pub perk: TraitType,
    /// Led by the outsider of a foreign House's pick (rule 5).
    pub outsider: bool,
    pub formed_turn: i32,
    /// Its cities in join order; the first is its seat.
    pub cities: Vec<CityId>,
    /// The state its owner was last told (rule 11): a change is told in the owner's turn.
    pub told_state: HouseState,
    /// Whether its perk is on (rule 9): set when it forms (on unless it is Furious then) and
    /// changed only in its owner's turn pass that tells a change of state (off when it is
    /// Furious then, on when it is not), so the cities' stored values change at one moment.
    pub perk_on: bool,
}

impl House {
    /// What its opinion depends on.
    #[inline]
    pub fn leaning(&self) -> Leaning {
        Leaning { alignment: self.alignment, religion: self.religion, sympathy: self.sympathy, outsider: self.outsider }
    }

    /// Its seat: its first city.
    pub fn seat(&self) -> Option<CityId> {
        self.cities.first().copied()
    }
}

/// A House's state (rule 7), by its opinion.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HouseState {
    Friendly,
    Pleased,
    Cautious,
    Annoyed,
    Furious,
}

impl HouseState {
    pub const ALL: [HouseState; 5] = [HouseState::Friendly, HouseState::Pleased, HouseState::Cautious, HouseState::Annoyed, HouseState::Furious];

    /// The state of opinion `n` (`HOUSE_OPINION_*`: Friendly 3 or more, Pleased 1 to 2, Cautious
    /// -2 to 0, Annoyed -5 to -3, Furious -6 or less).
    #[inline]
    pub fn of(d: &GameData, n: i32) -> HouseState {
        let x = &d.defines;
        if n >= x.house_opinion_friendly {
            HouseState::Friendly
        } else if n >= x.house_opinion_pleased {
            HouseState::Pleased
        } else if n >= x.house_opinion_cautious {
            HouseState::Cautious
        } else if n >= x.house_opinion_annoyed {
            HouseState::Annoyed
        } else {
            HouseState::Furious
        }
    }

    /// The happiness it gives each city of the House (`HOUSE_HAPPY_*`), the outsider's apart.
    #[inline]
    pub fn happiness(self, d: &GameData) -> i32 {
        let x = &d.defines;
        match self {
            HouseState::Friendly => x.house_happy_friendly,
            HouseState::Pleased => x.house_happy_pleased,
            HouseState::Cautious => x.house_happy_cautious,
            HouseState::Annoyed => x.house_happy_annoyed,
            HouseState::Furious => x.house_happy_furious,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            HouseState::Friendly => "Friendly",
            HouseState::Pleased => "Pleased",
            HouseState::Cautious => "Cautious",
            HouseState::Annoyed => "Annoyed",
            HouseState::Furious => "Furious",
        }
    }
}

/// Alignment steps between two alignments (Good - Neutral - Evil); `None` when either is unset.
#[inline]
pub fn alignment_steps(a: Option<AlignmentType>, b: Option<AlignmentType>) -> Option<i32> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.idx() as i32 - b.idx() as i32).abs()),
        _ => None,
    }
}

/// Whether religion `r` admits alignment `a` (rule 6): `a` lies between its `alignment_best`
/// and `alignment_worst` (an unset bound admits every alignment on its side).
pub fn religion_admits(d: &GameData, r: ReligionType, a: Option<AlignmentType>) -> bool {
    let info = &d.religions[r];
    let Some(a) = a else { return info.alignment_best.is_none() && info.alignment_worst.is_none() };
    info.alignment_best.map_or(true, |b| a >= b) && info.alignment_worst.map_or(true, |w| a <= w)
}

/// A leader's name.
pub fn leader_name(d: &GameData, l: LeaderHeadType) -> String {
    d.text.name(&d.leader_heads[l].base.description).to_string()
}

/// A minor leader's epithet: the first paragraph of their civilopedia text when it is a short
/// title (`The Auditor of Oaths`); `None` when the text starts with prose.
pub fn leader_epithet(d: &GameData, l: LeaderHeadType) -> Option<String> {
    let text = d.text.get(&d.leader_heads[l].base.civilopedia);
    let (first, rest) = text.split_once("[NEWLINE][NEWLINE]")?;
    let first = first.trim();
    if first.is_empty() || rest.is_empty() || first.len() > 60 || first.contains('.') {
        return None;
    }
    Some(first.to_string())
}

/// An alignment's name (`Neutral`; `none` when unset).
pub fn alignment_name(d: &GameData, a: Option<AlignmentType>) -> String {
    a.map_or_else(|| "none".to_string(), |a| d.text.name(&d.alignments[a].base.description).to_string())
}

/// A religion's name (`Runes of Kilmorph`; `none`).
pub fn religion_name(d: &GameData, r: Option<ReligionType>) -> String {
    r.map_or_else(|| "none".to_string(), |r| d.text.name(&d.religions[r].base.description).to_string())
}

/// A perk's name (`Horsemasters I`).
pub fn perk_name(d: &GameData, t: TraitType) -> String {
    d.text.name(&d.traits[t].base.description).to_string()
}

/// `a`, `a and b`, `a, b and c`.
pub(crate) fn and_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

impl Game {
    /// Whether player `p` has Houses (rule 1): every living civilization, a vassal included; not
    /// the barbarians, a minor civilization or a crusade Host.
    pub fn player_has_houses(&self, p: PlayerId) -> bool {
        self.players[p].alive && !self.player_is_barbarian(p) && !self.player_is_minor_civ(p) && !self.player_is_crusade_host(p)
    }

    /// Player `p`'s Houses, in the order they formed.
    pub fn player_houses(&self, p: PlayerId) -> &[HouseId] {
        &self.players[p].houses
    }

    /// `House Aldric`.
    pub fn house_label(&self, h: HouseId) -> String {
        format!("House {}", self.houses[h].name)
    }

    /// House `h`'s seat (its first city).
    pub fn house_seat(&self, h: HouseId) -> Option<CityId> {
        self.houses[h].seat()
    }

    /// City `c`'s origin (rule 3): the civilization of its original owner; the current owner's
    /// own where that is the barbarians or the owner's own civilization (a home city).
    pub fn city_origin(&self, c: CityId) -> Option<CivilizationType> {
        let city = &self.cities[c];
        let own = self.player_get_civilization_type(city.owner);
        let original = city.original_owner;
        if self.player_is_barbarian(original) {
            return own;
        }
        self.player_get_civilization_type(original).or(own)
    }

    /// Whether city `c` is a home city of its owner (rule 3).
    pub fn city_is_home(&self, c: CityId) -> bool {
        self.city_origin(c) == self.player_get_civilization_type(self.cities[c].owner)
    }

    /// Whether House `h` is foreign: of another civilization than its owner's.
    pub fn house_is_foreign(&self, h: HouseId) -> bool {
        Some(self.houses[h].origin) != self.player_get_civilization_type(self.houses[h].owner)
    }

    /// The perk in effect in city `c` (rule 9): its House's while the House's switch is on
    /// (`House::perk_on`); `None` for a city in no House or whose House's perk is off.
    pub fn city_house_perk(&self, c: CityId) -> Option<TraitType> {
        let h = self.cities.get(c)?.house?;
        let house = self.houses.get(h)?;
        house.perk_on.then_some(house.perk)
    }

    /// The perk tier a House forming now rolls (rule 9): 1 before `HOUSE_PERK_TIER_2_TURN`, 2
    /// from it, 3 from `HOUSE_PERK_TIER_3_TURN` (Normal speed; scaled as `crusade::speed_turns`).
    pub fn house_tier_now(&self) -> i32 {
        let turn = self.game_turn();
        let x = &self.d.defines;
        if turn >= crate::crusade::speed_turns(self, x.house_perk_tier_3_turn) {
            3
        } else if turn >= crate::crusade::speed_turns(self, x.house_perk_tier_2_turn) {
            2
        } else {
            1
        }
    }

    /// The name a new House of player `p` of civilization `origin` takes (without "House"): the
    /// first of `origin`'s governor-name list, then of the generic list, not taken by a House of
    /// `p` in this game; else the number of its Houses so far plus one.
    pub fn house_new_name(&self, p: PlayerId, origin: CivilizationType) -> String {
        let d = self.d;
        let used = &self.players[p].house_names;
        let lists = [
            d.governor_names.keys().find(|&k| d.governor_names[k].civilization == Some(origin)),
            d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_none()),
        ];
        for list in lists.into_iter().flatten() {
            if let Some(n) = d.governor_names[list].names.iter().find(|n| !used.contains(n)) {
                return n.clone();
            }
        }
        format!("{}", used.len() + 1)
    }

    /// A message to House `h`'s owner about it (rule 11), marked for the client's alerts (at its
    /// seat).
    pub(crate) fn house_msg(&mut self, h: HouseId, f: impl FnOnce() -> String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let p = self.houses[h].owner;
        let at = self.house_seat(h).map(|c| (self.cities[c].x, self.cities[c].y));
        self.log.msg_tagged(turn, Some(p), at, LogMark::House, LogTag::House(h), f);
    }
}
