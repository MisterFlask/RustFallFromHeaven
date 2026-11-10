//! This game's own info class for battle tactics (not FFH2's; docs/TACTICS.md):
//! `battle_tactics.jsonc`. An army's (or a city garrison's) standing tactic decides which of its
//! units fight when the slots are few and where the kinds of units stand, in every battle it
//! fights. The knobs ([`TacticKnobs`]) are what the rules' auto formation reads
//! (`formation::auto_arrange_around` in ffh-core).

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;
use super::engagement::ActionKind;

define_idx!(/// A battle tactic (`TACTIC_*`, battle_tactics.jsonc).
BattleTacticType);

/// A unit's kind as the auto formation sorts it: the action of the row it prefers, a striker with
/// an `anti_flank` promotion (Formation) apart. The discriminants are Line's pick order
/// (docs/ENGAGEMENTS.md, "The auto formation"): anti-flanking strikers, strikers, shooters,
/// chargers, casters, healers, bombards, stabbers, then units that do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TacticKind {
    AntiFlank = 1,
    Strike = 2,
    Shoot = 3,
    Charge = 4,
    Spell = 5,
    Heal = 6,
    Bombard = 7,
    Stab = 8,
    None = 9,
}

impl TacticKind {
    /// Every kind, in Line's pick order.
    pub const ALL: [TacticKind; 9] = [
        TacticKind::AntiFlank,
        TacticKind::Strike,
        TacticKind::Shoot,
        TacticKind::Charge,
        TacticKind::Spell,
        TacticKind::Heal,
        TacticKind::Bombard,
        TacticKind::Stab,
        TacticKind::None,
    ];

    /// The kind of a unit whose preferred row's action is `kind` (`anti_flank`: it holds an
    /// `anti_flank` promotion; only a striker's kind changes with it).
    pub fn of(kind: ActionKind, anti_flank: bool) -> TacticKind {
        match kind {
            ActionKind::Strike if anti_flank => TacticKind::AntiFlank,
            ActionKind::Strike => TacticKind::Strike,
            ActionKind::Shoot => TacticKind::Shoot,
            ActionKind::Charge => TacticKind::Charge,
            ActionKind::Spell => TacticKind::Spell,
            ActionKind::Heal => TacticKind::Heal,
            ActionKind::Bombard => TacticKind::Bombard,
            ActionKind::Stab => TacticKind::Stab,
            ActionKind::None => TacticKind::None,
        }
    }

    /// Its place in Line's pick order (1 first).
    pub fn pick_order(self) -> i32 {
        self as i32
    }

    /// The kind of Line's pick order `n`, if it is one.
    pub fn from_pick_order(n: i32) -> Option<TacticKind> {
        TacticKind::ALL.into_iter().find(|k| k.pick_order() == n)
    }

    /// A guard: a strike or a charge (anti-flanking strikers, strikers, chargers). Every other
    /// kind is support.
    pub fn guard(self) -> bool {
        matches!(self, TacticKind::AntiFlank | TacticKind::Strike | TacticKind::Charge)
    }

    /// The data's token.
    pub fn token(self) -> &'static str {
        match self {
            TacticKind::AntiFlank => "ANTI_FLANK",
            TacticKind::Strike => "STRIKE",
            TacticKind::Shoot => "SHOOT",
            TacticKind::Charge => "CHARGE",
            TacticKind::Spell => "SPELL",
            TacticKind::Heal => "HEAL",
            TacticKind::Bombard => "BOMBARD",
            TacticKind::Stab => "STAB",
            TacticKind::None => "NONE",
        }
    }

    pub fn parse(s: &str) -> Option<TacticKind> {
        TacticKind::ALL.into_iter().find(|k| k.token() == s.trim())
    }
}

/// How a tactic places the units it chose (docs/TACTICS.md, section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TacticPlacement {
    /// Line's: large units first, each unit in the first row of its rank order with room, the
    /// lanes nearest the centre first (`place_first` kinds before the others).
    #[default]
    Line,
    /// Lane by lane from the centre out: a guard in front, a support unit in the middle, a guard
    /// in the rear.
    Square,
    /// Line's, with each veteran guard in the middle row behind a guard that is no veteran.
    Vanguard,
}

impl TacticPlacement {
    pub fn token(self) -> &'static str {
        match self {
            TacticPlacement::Line => "LINE",
            TacticPlacement::Square => "SQUARE",
            TacticPlacement::Vanguard => "VANGUARD",
        }
    }

    pub fn parse(s: &str) -> Option<TacticPlacement> {
        [TacticPlacement::Line, TacticPlacement::Square, TacticPlacement::Vanguard].into_iter().find(|p| p.token() == s.trim())
    }
}

/// What a tactic changes in the auto formation (docs/TACTICS.md, section 5). The default is
/// Line's: today's auto formation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacticKnobs {
    /// Selection: the kinds in the order they take the slots; kinds left out come after, in
    /// Line's order. Empty: Line's order.
    pub pick_order: Vec<TacticKind>,
    /// One unit of each kind before the rest (Line's rule).
    pub one_of_each: bool,
    /// Guards taken before anything else, per lane of the side with a free front cell (a large
    /// guard screens two).
    pub screen_per_lane: i32,
    pub placement: TacticPlacement,
    /// Line placement only: kinds placed before the others.
    pub place_first: Vec<TacticKind>,
}

impl TacticKnobs {
    /// Line's knobs: today's auto formation.
    pub fn line() -> TacticKnobs {
        TacticKnobs { pick_order: Vec::new(), one_of_each: true, screen_per_lane: 0, placement: TacticPlacement::Line, place_first: Vec::new() }
    }
}

impl Default for TacticKnobs {
    fn default() -> TacticKnobs {
        TacticKnobs::line()
    }
}

/// One battle tactic (this game's `BattleTacticInfo`).
#[derive(Debug, Clone, Default)]
pub struct BattleTacticInfo {
    pub base: InfoBase,
    pub knobs: TacticKnobs,
}

fn read_kinds(rd: &Rd, key: &str) -> Vec<TacticKind> {
    let mut out: Vec<TacticKind> = Vec::new();
    for t in rd.string_list(key) {
        match TacticKind::parse(&t) {
            Some(k) if !out.contains(&k) => out.push(k),
            Some(_) => rd.cx.warn(format!("{key}: {t} twice")),
            None => rd.cx.warn(format!("{key}: unknown kind {t:?}")),
        }
    }
    out
}

pub fn read_battle_tactic(rd: &Rd) -> BattleTacticInfo {
    let base = InfoBase::read(rd);
    let placement = if rd.has("placement") {
        let t = rd.string("placement");
        TacticPlacement::parse(&t).unwrap_or_else(|| {
            rd.cx.warn(format!("{}: unknown placement {t:?}", base.type_));
            TacticPlacement::Line
        })
    } else {
        TacticPlacement::Line
    };
    let knobs = TacticKnobs {
        pick_order: read_kinds(rd, "pick_order"),
        one_of_each: rd.bool_or("one_of_each", true),
        screen_per_lane: rd.int_or("screen_per_lane", 0).max(0),
        placement,
        place_first: read_kinds(rd, "place_first"),
    };
    BattleTacticInfo { base, knobs }
}
