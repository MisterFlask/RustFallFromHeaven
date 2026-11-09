//! This game's own info class for battle arrays (not FFH2's; docs/ARRAYS.md):
//! `battle_arrays.jsonc`. An army's (or a city garrison's) standing array decides which of its
//! units fight when the slots are few and where the kinds of units stand, in every battle it
//! fights. The knobs ([`ArrayKnobs`]) are what the rules' auto formation reads
//! (`formation::auto_arrange_around` in ffh-core).

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;
use super::engagement::ActionKind;

define_idx!(/// A battle array (`ARRAY_*`, battle_arrays.jsonc).
BattleArrayType);

/// A unit's kind as the auto formation sorts it: the action of the row it prefers, a striker with
/// an `anti_flank` promotion (Formation) apart. The discriminants are Line's pick order
/// (docs/ENGAGEMENTS.md, "The auto formation"): anti-flanking strikers, strikers, shooters,
/// chargers, casters, healers, bombards, stabbers, then units that do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArrayKind {
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

impl ArrayKind {
    /// Every kind, in Line's pick order.
    pub const ALL: [ArrayKind; 9] = [
        ArrayKind::AntiFlank,
        ArrayKind::Strike,
        ArrayKind::Shoot,
        ArrayKind::Charge,
        ArrayKind::Spell,
        ArrayKind::Heal,
        ArrayKind::Bombard,
        ArrayKind::Stab,
        ArrayKind::None,
    ];

    /// The kind of a unit whose preferred row's action is `kind` (`anti_flank`: it holds an
    /// `anti_flank` promotion; only a striker's kind changes with it).
    pub fn of(kind: ActionKind, anti_flank: bool) -> ArrayKind {
        match kind {
            ActionKind::Strike if anti_flank => ArrayKind::AntiFlank,
            ActionKind::Strike => ArrayKind::Strike,
            ActionKind::Shoot => ArrayKind::Shoot,
            ActionKind::Charge => ArrayKind::Charge,
            ActionKind::Spell => ArrayKind::Spell,
            ActionKind::Heal => ArrayKind::Heal,
            ActionKind::Bombard => ArrayKind::Bombard,
            ActionKind::Stab => ArrayKind::Stab,
            ActionKind::None => ArrayKind::None,
        }
    }

    /// Its place in Line's pick order (1 first).
    pub fn pick_order(self) -> i32 {
        self as i32
    }

    /// The kind of Line's pick order `n`, if it is one.
    pub fn from_pick_order(n: i32) -> Option<ArrayKind> {
        ArrayKind::ALL.into_iter().find(|k| k.pick_order() == n)
    }

    /// A guard: a strike or a charge (anti-flanking strikers, strikers, chargers). Every other
    /// kind is support.
    pub fn guard(self) -> bool {
        matches!(self, ArrayKind::AntiFlank | ArrayKind::Strike | ArrayKind::Charge)
    }

    /// The data's token.
    pub fn token(self) -> &'static str {
        match self {
            ArrayKind::AntiFlank => "ANTI_FLANK",
            ArrayKind::Strike => "STRIKE",
            ArrayKind::Shoot => "SHOOT",
            ArrayKind::Charge => "CHARGE",
            ArrayKind::Spell => "SPELL",
            ArrayKind::Heal => "HEAL",
            ArrayKind::Bombard => "BOMBARD",
            ArrayKind::Stab => "STAB",
            ArrayKind::None => "NONE",
        }
    }

    pub fn parse(s: &str) -> Option<ArrayKind> {
        ArrayKind::ALL.into_iter().find(|k| k.token() == s.trim())
    }
}

/// How an array places the units it chose (docs/ARRAYS.md, section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayPlacement {
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

impl ArrayPlacement {
    pub fn token(self) -> &'static str {
        match self {
            ArrayPlacement::Line => "LINE",
            ArrayPlacement::Square => "SQUARE",
            ArrayPlacement::Vanguard => "VANGUARD",
        }
    }

    pub fn parse(s: &str) -> Option<ArrayPlacement> {
        [ArrayPlacement::Line, ArrayPlacement::Square, ArrayPlacement::Vanguard].into_iter().find(|p| p.token() == s.trim())
    }
}

/// What an array changes in the auto formation (docs/ARRAYS.md, section 5). The default is
/// Line's: today's auto formation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayKnobs {
    /// Selection: the kinds in the order they take the slots; kinds left out come after, in
    /// Line's order. Empty: Line's order.
    pub pick_order: Vec<ArrayKind>,
    /// One unit of each kind before the rest (Line's rule).
    pub one_of_each: bool,
    /// Guards taken before anything else, per lane of the side with a free front cell (a large
    /// guard screens two).
    pub screen_per_lane: i32,
    pub placement: ArrayPlacement,
    /// Line placement only: kinds placed before the others.
    pub place_first: Vec<ArrayKind>,
}

impl ArrayKnobs {
    /// Line's knobs: today's auto formation.
    pub fn line() -> ArrayKnobs {
        ArrayKnobs { pick_order: Vec::new(), one_of_each: true, screen_per_lane: 0, placement: ArrayPlacement::Line, place_first: Vec::new() }
    }
}

impl Default for ArrayKnobs {
    fn default() -> ArrayKnobs {
        ArrayKnobs::line()
    }
}

/// One battle array (this game's `BattleArrayInfo`).
#[derive(Debug, Clone, Default)]
pub struct BattleArrayInfo {
    pub base: InfoBase,
    pub knobs: ArrayKnobs,
}

fn read_kinds(rd: &Rd, key: &str) -> Vec<ArrayKind> {
    let mut out: Vec<ArrayKind> = Vec::new();
    for t in rd.string_list(key) {
        match ArrayKind::parse(&t) {
            Some(k) if !out.contains(&k) => out.push(k),
            Some(_) => rd.cx.warn(format!("{key}: {t} twice")),
            None => rd.cx.warn(format!("{key}: unknown kind {t:?}")),
        }
    }
    out
}

pub fn read_battle_array(rd: &Rd) -> BattleArrayInfo {
    let base = InfoBase::read(rd);
    let placement = if rd.has("placement") {
        let t = rd.string("placement");
        ArrayPlacement::parse(&t).unwrap_or_else(|| {
            rd.cx.warn(format!("{}: unknown placement {t:?}", base.type_));
            ArrayPlacement::Line
        })
    } else {
        ArrayPlacement::Line
    };
    let knobs = ArrayKnobs {
        pick_order: read_kinds(rd, "pick_order"),
        one_of_each: rd.bool_or("one_of_each", true),
        screen_per_lane: rd.int_or("screen_per_lane", 0).max(0),
        placement,
        place_first: read_kinds(rd, "place_first"),
    };
    BattleArrayInfo { base, knobs }
}
