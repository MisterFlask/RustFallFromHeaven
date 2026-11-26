//! Shared data-free battle primitives used by both `ffh-battle` and game-data crates:
//! action kinds, attack patterns, row action tables, battle charms/hexes, and pre-battle presences.

/// What a unit does with one action of a formation battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ActionKind {
    /// `NONE`: no action from this row.
    #[default]
    None,
    /// `STRIKE`: melee on the front unit of an enemy lane; a crit knocks back.
    Strike,
    /// `CHARGE`: the flanker's charge: where flanking works, anyone on the field (the highest
    /// threat for its HP left), harder in the first round; elsewhere a strike.
    Charge,
    /// `STAB`: any enemy.
    Stab,
    /// `SHOOT`: the backmost unit of an enemy lane; misses more.
    Shoot,
    /// `SPELL`: the backmost unit of an enemy lane: the caster's strongest damage spell (with a
    /// splash), else a magic bolt; never misses, never crits.
    Spell,
    /// `BOMBARD`: the backmost unit of an enemy lane; misses most, never crits.
    Bombard,
    /// `HEAL`: the most hurt ally.
    Heal,
}

impl ActionKind {
    pub const ALL: [ActionKind; 8] = [
        ActionKind::None,
        ActionKind::Strike,
        ActionKind::Charge,
        ActionKind::Stab,
        ActionKind::Shoot,
        ActionKind::Spell,
        ActionKind::Bombard,
        ActionKind::Heal,
    ];

    /// The token of the kind (also used in battle logs and data files).
    pub fn token(self) -> &'static str {
        match self {
            ActionKind::None => "NONE",
            ActionKind::Strike => "STRIKE",
            ActionKind::Charge => "CHARGE",
            ActionKind::Stab => "STAB",
            ActionKind::Shoot => "SHOOT",
            ActionKind::Spell => "SPELL",
            ActionKind::Bombard => "BOMBARD",
            ActionKind::Heal => "HEAL",
        }
    }

    pub fn parse(s: &str) -> Option<ActionKind> {
        ActionKind::ALL.into_iter().find(|k| k.token() == s.trim())
    }

    /// A physical action: it can miss (strike, charge, stab, shoot, bombard).
    pub fn physical(self) -> bool {
        matches!(self, ActionKind::Strike | ActionKind::Charge | ActionKind::Stab | ActionKind::Shoot | ActionKind::Bombard)
    }

    /// A physical single-target action: it can crit (strike, charge, stab, shoot).
    pub fn can_crit(self) -> bool {
        matches!(self, ActionKind::Strike | ActionKind::Charge | ActionKind::Stab | ActionKind::Shoot)
    }

    /// A melee action: its crit knocks the target back (strike, charge).
    pub fn melee(self) -> bool {
        matches!(self, ActionKind::Strike | ActionKind::Charge)
    }

    /// An action that falls on the backmost unit of a lane (shoot, spell, bombard).
    pub fn ranged(self) -> bool {
        matches!(self, ActionKind::Shoot | ActionKind::Spell | ActionKind::Bombard)
    }
}

/// Whom else an action hits besides its primary target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum AttackPattern {
    /// `SINGLE`: the target alone.
    #[default]
    Single,
    /// `VOLLEY`: also the unit its kind would hit in each lane beside the target's.
    Volley,
    /// `PIERCE`: also every other unit in the target's lanes, front to back.
    Pierce,
    /// `SWEEP`: also every other unit in the target's row. A heal with it heals every hurt ally in
    /// its target's row.
    Sweep,
    /// `CLEAVE`: also the front unit of each lane beside the target's.
    Cleave,
    /// `TRAMPLE`: also the unit behind the target in its lanes.
    Trample,
}

impl AttackPattern {
    pub const ALL: [AttackPattern; 6] = [
        AttackPattern::Single,
        AttackPattern::Volley,
        AttackPattern::Pierce,
        AttackPattern::Sweep,
        AttackPattern::Cleave,
        AttackPattern::Trample,
    ];

    /// The data token of the pattern.
    pub fn token(self) -> &'static str {
        match self {
            AttackPattern::Single => "SINGLE",
            AttackPattern::Volley => "VOLLEY",
            AttackPattern::Pierce => "PIERCE",
            AttackPattern::Sweep => "SWEEP",
            AttackPattern::Cleave => "CLEAVE",
            AttackPattern::Trample => "TRAMPLE",
        }
    }

    pub fn parse(s: &str) -> Option<AttackPattern> {
        AttackPattern::ALL.into_iter().find(|k| k.token() == s.trim())
    }
}

/// The rows of a side's formation grid: front (0), middle (1), and rear (2).
pub const ROWS: usize = 3;

/// The data tokens of the rows, front first.
pub const ROW_TOKENS: [&str; ROWS] = ["FRONT", "MIDDLE", "REAR"];

/// What a unit does from one row: `count` actions of `kind` each round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowActions {
    pub kind: ActionKind,
    pub count: i32,
}

/// The row of `rows` with the most actions (the first such).
pub fn rows_best_row(rows: &[RowActions; ROWS]) -> usize {
    let mut best = 0;
    for (i, r) in rows.iter().enumerate() {
        if r.count > rows[best].count {
            best = i;
        }
    }
    best
}

/// `rows` with `extra` more actions in their best row; a row without an action stays without.
pub fn rows_with_extra_actions(mut rows: [RowActions; ROWS], extra: i32) -> [RowActions; ROWS] {
    let best = rows_best_row(&rows);
    if rows[best].kind != ActionKind::None {
        rows[best].count += extra;
    }
    rows
}

/// `rows` of a unit with magic that knows a battle spell: its spell replaces physical attacks.
pub fn rows_with_battle_spell(mut rows: [RowActions; ROWS]) -> [RowActions; ROWS] {
    if rows.iter().any(|r| r.kind == ActionKind::Heal) {
        return rows;
    }
    for r in &mut rows {
        if matches!(r.kind, ActionKind::Strike | ActionKind::Charge | ActionKind::Stab | ActionKind::Shoot | ActionKind::Bombard) {
            r.kind = ActionKind::Spell;
        }
    }
    rows
}

/// What a unit with `rows` and rank preference `order` does from the row it prefers.
pub fn rows_kind(rows: &[RowActions; ROWS], order: &[usize; ROWS]) -> ActionKind {
    rows[order[0].min(ROWS - 1)].kind
}

/// A battle charm or hex lasting the battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattleCharm {
    pub enemy: bool,
    pub whole_side: bool,
    /// `"rows": "REAR"`: the side's rearmost row in use (else the front row, or the whole side).
    pub rear: bool,
    pub damage: i32,
    pub taken: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
    pub speed: i32,
    /// `skip`: actions each unit it lands on skips, its next ones.
    pub skip: i32,
}

/// A pre-battle hex of a unit's presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    /// `GROUNDING`: for the battle, the enemy's charges are strikes (front reach, no first-round
    /// bonus).
    Grounding,
    /// `MADNESS`: in round 1 the enemy's unit with the highest attack (not a Sentinel) hits its
    /// own side.
    Madness,
}

impl Presence {
    pub const ALL: [Presence; 2] = [Presence::Grounding, Presence::Madness];

    pub fn token(self) -> &'static str {
        match self {
            Presence::Grounding => "GROUNDING",
            Presence::Madness => "MADNESS",
        }
    }

    pub fn parse(s: &str) -> Option<Presence> {
        Presence::ALL.into_iter().find(|p| p.token() == s.trim())
    }
}
