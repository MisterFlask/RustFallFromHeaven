//! This game's own info classes for engagement combat (not FFH2's; phase 6 of the owner's spec,
//! docs/ENGAGEMENTS.md): the actions of formation battles and what a unit type does from each row
//! of its side's grid (`engage_rows` and `engage_row_order` in units.jsonc, read here), and the
//! stat sheets (docs/UNIT_STATS.md). There are no roles (the owner's decision, 2026-11-06): each
//! unit type has its own rows and rank preference.

use crate::registry::Rd;

/// What a unit does with one action of a formation battle (an `action` of its `engage_rows`;
/// docs/ENGAGEMENTS.md, "Action kinds and reach").
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

    /// The XML token of the kind (also the battle log's).
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

/// The rows of a side's formation grid, front (0), middle (1) and rear (2).
pub const ROWS: usize = 3;

/// The data's tokens of the rows (`engage_row_order`), front first.
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

/// `rows` with `extra` more actions in their best row ([`rows_best_row`]); a row without an action
/// stays without.
pub fn rows_with_extra_actions(mut rows: [RowActions; ROWS], extra: i32) -> [RowActions; ROWS] {
    let best = rows_best_row(&rows);
    if rows[best].kind != ActionKind::None {
        rows[best].count += extra;
    }
    rows
}

/// `rows` of a unit with magic that knows a battle spell (docs/ENGAGEMENTS.md, "Battle spells"):
/// its spell is the action it takes in place of each attack (strike, charge, stab, shoot, bombard;
/// the same count), in every row. A healer's rows (one heals) stay as they are.
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

/// What a unit with `rows` and rank preference `order` does from the row it prefers (the first of
/// `order`): its kind, which the auto formation sorts by (`formation::kind_pick_order`).
pub fn rows_kind(rows: &[RowActions; ROWS], order: &[usize; ROWS]) -> ActionKind {
    rows[order[0].min(ROWS - 1)].kind
}

/// Whether one of `rows` charges: a flanker's rows.
pub fn rows_charge(rows: &[RowActions; ROWS]) -> bool {
    rows.iter().any(|r| r.kind == ActionKind::Charge)
}

/// What is wrong with a unit type's rows, if anything: every row has a count above 0
/// exactly when its kind is not NONE, and some row acts.
pub fn rows_problem(rows: &[RowActions; ROWS]) -> Option<&'static str> {
    if rows.iter().any(|row| row.count < 0 || (row.kind == ActionKind::None) != (row.count == 0)) {
        Some("a row has a count above 0 exactly when its action is not NONE")
    } else if rows.iter().all(|row| row.count == 0) {
        Some("no row has an action")
    } else {
        None
    }
}

/// A unit's battle numbers (docs/UNIT_STATS.md): each land unit type's `stats` sheet in
/// units.jsonc, and what promotions change of them (`stat_changes` in promotions.jsonc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatType {
    /// Hit points in a battle (the unit's damage stays a percentage of them).
    Hp,
    /// The hitter's side of STRIKE, CHARGE, STAB, SHOOT and BOMBARD.
    Attack,
    /// The target's side of those actions.
    Defense,
    /// The caster's side of SPELL: 0 casts nothing.
    Magic,
    /// The target's side of SPELL.
    MagicDefense,
    /// The hitter's chance to hit, in percent, before the target's evasion.
    Accuracy,
    /// Taken off a physical hitter's chance to hit.
    Evasion,
    /// The chance to crit on STRIKE, CHARGE, STAB and SHOOT, in percent.
    Crit,
    /// The order in each pass of a round (first strikes add to it).
    Speed,
}

impl StatType {
    pub const NUM: usize = 9;
    pub const ALL: [StatType; StatType::NUM] = [
        StatType::Hp,
        StatType::Attack,
        StatType::Defense,
        StatType::Magic,
        StatType::MagicDefense,
        StatType::Accuracy,
        StatType::Evasion,
        StatType::Crit,
        StatType::Speed,
    ];

    /// Its key in the data (`stats`, `stat_changes`).
    pub fn key(self) -> &'static str {
        match self {
            StatType::Hp => "hp",
            StatType::Attack => "attack",
            StatType::Defense => "defense",
            StatType::Magic => "magic",
            StatType::MagicDefense => "magic_defense",
            StatType::Accuracy => "accuracy",
            StatType::Evasion => "evasion",
            StatType::Crit => "crit",
            StatType::Speed => "speed",
        }
    }

    /// Its short name (HP, ATK, DEF, MAG, MDEF, ACC, EVA, CRIT, SPD).
    pub fn abbr(self) -> &'static str {
        match self {
            StatType::Hp => "HP",
            StatType::Attack => "ATK",
            StatType::Defense => "DEF",
            StatType::Magic => "MAG",
            StatType::MagicDefense => "MDEF",
            StatType::Accuracy => "ACC",
            StatType::Evasion => "EVA",
            StatType::Crit => "CRIT",
            StatType::Speed => "SPD",
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }

    /// The values it may hold in a sheet.
    pub fn valid(self) -> std::ops::RangeInclusive<i32> {
        match self {
            StatType::Hp => 1..=i32::MAX,
            StatType::Accuracy => 0..=200,
            StatType::Evasion | StatType::Crit => 0..=95,
            _ => 0..=i32::MAX,
        }
    }
}

/// A stat sheet, by [`StatType::idx`].
pub type StatSheet = [i32; StatType::NUM];

/// What something changes of one stat: points added, and a percentage summed with the stat's
/// other percentages (`x (100 + m) / 100` above 0, `x 100 / (100 - m)` below).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatChange {
    pub flat: i32,
    pub percent: i32,
}

/// A unit type's `stats` sheet: every stat's value (`None` without the key: the unit takes no
/// part in land battles). A missing or out-of-range value warns and reads as its range's
/// lowest.
pub fn read_stats(rd: &Rd, unit_type: &str) -> Option<StatSheet> {
    let sheet = rd.child("stats")?;
    let mut out = [0; StatType::NUM];
    for s in StatType::ALL {
        let valid = s.valid();
        let v = if sheet.has(s.key()) {
            sheet.int(s.key())
        } else {
            rd.cx.warn(format!("{unit_type}: stats has no {:?}", s.key()));
            *valid.start()
        };
        out[s.idx()] = if valid.contains(&v) {
            v
        } else {
            rd.cx.warn(format!("{unit_type}: stats {:?} {v} is not in {}..={}", s.key(), valid.start(), valid.end()));
            *valid.start()
        };
    }
    Some(out)
}

/// A promotion's `stat_changes`: an object from stat keys to `{flat, percent}`.
pub fn read_stat_changes(rd: &Rd) -> [StatChange; StatType::NUM] {
    let mut out = [StatChange::default(); StatType::NUM];
    if let Some(changes) = rd.child("stat_changes") {
        for s in StatType::ALL {
            if let Some(c) = changes.child(s.key()) {
                out[s.idx()] = StatChange { flat: c.int("flat"), percent: c.int("percent") };
            }
        }
    }
    out
}

/// A unit type's `engage_rows` (`units.jsonc`): three rows, front, middle and rear, each an
/// `action` and its `count`. `None` without the key. Rows that are not exactly three, an unknown
/// action, or rows [`rows_problem`] refuses warn and read as `None` (the unit then does nothing
/// in a battle; the loader warns for a land unit without rows).
pub fn read_engage_rows(rd: &Rd, unit_type: &str) -> Option<[RowActions; ROWS]> {
    let list = rd.child("engage_rows")?;
    let rows: Vec<Rd> = list.children().collect();
    if rows.len() != ROWS {
        rd.cx.warn(format!("{unit_type}: engage_rows has {} rows ({ROWS} expected: front, middle, rear)", rows.len()));
        return None;
    }
    let mut out = [RowActions::default(); ROWS];
    for (i, row) in rows.iter().enumerate() {
        let (token, count) = (row.string("action"), row.int("count"));
        let Some(kind) = ActionKind::parse(&token) else {
            rd.cx.warn(format!("{unit_type}: engage_rows, {} row: unknown action {token:?}", ROW_TOKENS[i]));
            return None;
        };
        out[i] = RowActions { kind, count };
    }
    if let Some(problem) = rows_problem(&out) {
        rd.cx.warn(format!("{unit_type}: engage_rows: {problem}"));
        return None;
    }
    Some(out)
}

/// A unit type's `engage_row_order` (`units.jsonc`): its rank preference, the three rows (`FRONT`,
/// `MIDDLE`, `REAR`) in the order the auto formation tries them. `None` without the key. A list
/// that is not the three rows once each warns and reads as `None`.
pub fn read_engage_row_order(rd: &Rd, unit_type: &str) -> Option<[usize; ROWS]> {
    if !rd.has("engage_row_order") {
        return None;
    }
    let tokens: Vec<String> = rd.string_list("engage_row_order");
    let rows: Vec<usize> = tokens.iter().filter_map(|t| ROW_TOKENS.iter().position(|&r| r == t.trim())).collect();
    let mut seen = [false; ROWS];
    for &r in &rows {
        seen[r] = true;
    }
    if tokens.len() != ROWS || rows.len() != ROWS || seen.iter().any(|&s| !s) {
        rd.cx.warn(format!("{unit_type}: engage_row_order {tokens:?} is not FRONT, MIDDLE and REAR once each"));
        return None;
    }
    Some([rows[0], rows[1], rows[2]])
}

