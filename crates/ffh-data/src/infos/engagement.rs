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

/// Whom else an action hits besides its target (an `engage_rows` row's `pattern`; docs/UNIT_ROLES.md,
/// "Attack patterns"). The extra hits land with the target's (no roll of their own), at the
/// pattern's percentage of a hit (`ENGAGEMENT_*_PERCENT`), never crit, and take the place of a
/// collateral splash (a known spell's splash stays the spell's).
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
    pub const ALL: [AttackPattern; 6] =
        [AttackPattern::Single, AttackPattern::Volley, AttackPattern::Pierce, AttackPattern::Sweep, AttackPattern::Cleave, AttackPattern::Trample];

    /// The data's token of the pattern.
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

/// A unit type's attack patterns, by row (each `engage_rows` row's optional `pattern`;
/// `SINGLE` without it). An unknown pattern warns and reads as `SINGLE`.
pub fn read_engage_patterns(rd: &Rd, unit_type: &str) -> [AttackPattern; ROWS] {
    let mut out = [AttackPattern::Single; ROWS];
    if let Some(list) = rd.child("engage_rows") {
        for (i, row) in list.children().enumerate().take(ROWS) {
            if row.has("pattern") {
                let token = row.string("pattern");
                match AttackPattern::parse(&token) {
                    Some(p) => out[i] = p,
                    None => rd.cx.warn(format!("{unit_type}: engage_rows, {} row: unknown pattern {token:?}", ROW_TOKENS[i])),
                }
            }
        }
    }
    out
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


/// A unit type's `round_percent` (`units.jsonc`): its hits' damage in each round, in percent (the
/// last value for later rounds); 100 each without the key. A list that is not three numbers of 0
/// or more warns and reads as 100 each.
pub fn read_round_percent(rd: &Rd, unit_type: &str) -> [i32; ROWS] {
    if !rd.has("round_percent") {
        return [100; ROWS];
    }
    let v: Vec<i32> = rd.child("round_percent").map(|l| l.children().map(|c| c.scalar_text().parse::<i32>().unwrap_or(-1)).collect()).unwrap_or_default();
    if v.len() != ROWS || v.iter().any(|&x| x < 0) {
        rd.cx.warn(format!("{unit_type}: round_percent {v:?} is not three percentages"));
        return [100; ROWS];
    }
    [v[0], v[1], v[2]]
}

/// A battle charm or hex (docs/ENGAGEMENTS.md, "Battle spells"): a spell's `battle_charm`, cast with
/// one of its caster's first actions and lasting the battle: on its own side (a charm) or the
/// enemy's (a hex), on the front row or the whole side; percent more damage dealt and taken, and
/// points of accuracy, evasion, crit and speed (each may be below 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattleCharm {
    pub enemy: bool,
    pub whole_side: bool,
    /// `"rows": "REAR"`: the rear row (else the front row, or the whole side).
    pub rear: bool,
    pub damage: i32,
    pub taken: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
    pub speed: i32,
    /// `skip`: actions each unit it lands on skips, its next ones (docs/CARDS.md, the Lullaby).
    pub skip: i32,
}

/// A spell's `battle_charm`: `{"side": "OWN"|"ENEMY", "rows": "FRONT"|"REAR"|"ALL", "damage",
/// "taken", "accuracy", "evasion", "crit", "speed", "skip"}`; an unknown side or rows warns and
/// reads as none.
pub fn read_battle_charm(rd: &Rd, spell: &str) -> Option<BattleCharm> {
    let c = rd.child("battle_charm")?;
    let side = c.string("side");
    let rows = c.string("rows");
    let enemy = match side.as_str() {
        "OWN" => false,
        "ENEMY" => true,
        _ => {
            rd.cx.warn(format!("{spell}: battle_charm side {side:?} is not OWN or ENEMY"));
            return None;
        }
    };
    let (whole_side, rear) = match rows.as_str() {
        "FRONT" => (false, false),
        "REAR" => (false, true),
        "ALL" => (true, false),
        _ => {
            rd.cx.warn(format!("{spell}: battle_charm rows {rows:?} is not FRONT, REAR or ALL"));
            return None;
        }
    };
    Some(BattleCharm {
        enemy,
        whole_side,
        rear,
        damage: c.int("damage"),
        taken: c.int("taken"),
        accuracy: c.int("accuracy"),
        evasion: c.int("evasion"),
        crit: c.int("crit"),
        speed: c.int("speed"),
        skip: c.int("skip"),
    })
}

/// A caster's battle spells (`GameData::engage_battle_spells`): the damage spell it casts with its
/// spell actions, the heal it casts in their place when an ally is hurt, and its charms and hexes,
/// cast first (docs/ENGAGEMENTS.md, "Battle spells").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSpells<S> {
    pub attack: Option<S>,
    pub heal: Option<S>,
    pub charms: Vec<S>,
    /// Its illusions (`battle_illusions`, docs/CARDS.md), cast with its first spell action.
    pub illusions: Option<S>,
}

impl<S> Default for BattleSpells<S> {
    fn default() -> Self {
        BattleSpells { attack: None, heal: None, charms: Vec::new(), illusions: None }
    }
}

impl<S> BattleSpells<S> {
    pub fn any(&self) -> bool {
        self.attack.is_some() || self.heal.is_some() || !self.charms.is_empty() || self.illusions.is_some()
    }
}
