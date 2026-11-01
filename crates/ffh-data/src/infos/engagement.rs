//! This game's own info classes for engagement combat (not FFH2's; phase 6 of the owner's spec,
//! `GAMEOPTION_ENGAGEMENTS`, docs/ENGAGEMENTS.md): `Units/CIV4CombatRoleInfos.xml` (the roles of
//! formation battles, revision 4). The role of each unit type is derived at load
//! (`GameData::unit_roles`, lib.rs).

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::registry::Rd;

define_idx!(/// An engagement role (`COMBATROLE_*`, Units/CIV4CombatRoleInfos.xml).
CombatRoleType);

/// What a unit does with one action of a formation battle (XML `Action` of a role's rows;
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

/// The XML tokens of the rows (`HomeRow`), front first.
pub const ROW_TOKENS: [&str; ROWS] = ["FRONT", "MIDDLE", "REAR"];

/// What a unit of a role does from one row: `count` actions of `kind` each round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowActions {
    pub kind: ActionKind,
    pub count: i32,
}

/// An engagement role (this game's `CombatRoleInfo`): the unit's multipliers, hit points, speed
/// and what it does from each row of its side's formation (docs/ENGAGEMENTS.md, "Roles").
#[derive(Debug, Clone, Default)]
pub struct CombatRoleInfo {
    pub base: InfoBase,
    /// `iAttackPercent`: the unit's `iCombat` is multiplied by this to give its attack A (100 = 1.0).
    pub attack_percent: i32,
    /// `iDefensePercent`: the unit's `iCombatDefense` is multiplied by this to give its defence Dv.
    pub defense_percent: i32,
    /// `iMaxHP`: the unit's hit points in an engagement (its damage stays a percentage of them).
    pub max_hp: i32,
    /// `iSpeed`: faster units act first in a pass (first strikes add to it).
    pub speed: i32,
    /// `iDodge`: points of miss chance added to every physical action on a unit of the role.
    pub dodge: i32,
    /// `iCritBonus`: points added to its own crit chance.
    pub crit_bonus: i32,
    /// `bAntiFlank`: whatever an enemy flanker can reach, units of this role in its reach are
    /// taken first.
    pub anti_flank: bool,
    /// `FrontRow`, `MiddleRow`, `RearRow` (each an `Action` and its `iActions`): what it does
    /// from each row, front first.
    pub rows: [RowActions; ROWS],
    /// `HomeRow` (`FRONT`, `MIDDLE`, `REAR`): the row the auto formation puts it in.
    pub home_row: usize,
    /// `iAutoPick`: the auto formation's role order (lowest first): it takes one unit of each
    /// role in this order, then the rest in it.
    pub auto_pick: i32,
    /// `DefaultUnitCombats/UnitCombat`: unit types of these unit combats take this role.
    pub default_unit_combats: Vec<UnitCombatType>,
    /// `DefaultFreePromotion`: ... only when the unit type has this promotion free (such roles
    /// are tried before the others).
    pub default_free_promotion: Option<PromotionType>,
    /// `FromRole` with `FromPromotion`: a unit of `from_role` holding `from_promotion` takes this
    /// role instead (`GameData::screen_role`).
    pub from_role: Option<CombatRoleType>,
    pub from_promotion: Option<PromotionType>,
    /// `bDefault`: the role of fighting land units no other role claims (no unit combat:
    /// monstrous units, summons).
    pub default: bool,
}

impl CombatRoleInfo {
    /// The row with the most actions (the first such): where a tier 4 unit or a hero gets its
    /// extra action. A unit type with its own `EngageRows` has its own best row
    /// (`GameData::engage_best_row`).
    pub fn best_row(&self) -> usize {
        rows_best_row(&self.rows)
    }

    /// What a unit of the role does from each row, with `extra` more actions in its best row
    /// (tier 4 units and heroes get `ENGAGEMENT_EXTRA_ACTIONS`); a row without an action stays
    /// without. A unit type with its own `EngageRows` acts from those (`GameData::engage_actions`).
    pub fn actions(&self, extra: i32) -> [RowActions; ROWS] {
        rows_with_extra_actions(self.rows, extra)
    }

    /// A flanker: one of its rows charges (anti-flank units in its reach are taken first).
    pub fn flanks(&self) -> bool {
        rows_charge(&self.rows)
    }
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

/// Whether one of `rows` charges: a flanker's rows.
pub fn rows_charge(rows: &[RowActions; ROWS]) -> bool {
    rows.iter().any(|r| r.kind == ActionKind::Charge)
}

/// What is wrong with a role's or a unit type's rows, if anything: every row has a count above 0
/// exactly when its kind is not NONE, and some row acts.
pub fn rows_problem(rows: &[RowActions; ROWS]) -> Option<&'static str> {
    if rows.iter().any(|row| row.count < 0 || (row.kind == ActionKind::None) != (row.count == 0)) {
        Some("a row has a count above 0 exactly when its Action is not NONE")
    } else if rows.iter().all(|row| row.count == 0) {
        Some("no row has an action")
    } else {
        None
    }
}

fn read_row(rd: &Rd, base: &InfoBase, tag: &str) -> RowActions {
    let Some(row) = rd.child(tag) else {
        rd.cx.warn(format!("{}: no {tag}", base.type_));
        return RowActions::default();
    };
    let token = row.string("Action");
    let kind = ActionKind::parse(&token).unwrap_or_else(|| {
        rd.cx.warn(format!("{}: {tag}: unknown Action {token:?}", base.type_));
        ActionKind::None
    });
    RowActions { kind, count: row.int("iActions") }
}

pub fn read_combat_role(rd: &Rd) -> CombatRoleInfo {
    let base = InfoBase::read(rd);
    let rows = [read_row(rd, &base, "FrontRow"), read_row(rd, &base, "MiddleRow"), read_row(rd, &base, "RearRow")];
    let home = rd.string("HomeRow");
    let home_row = ROW_TOKENS.iter().position(|&t| t == home.trim()).unwrap_or_else(|| {
        rd.cx.warn(format!("{}: unknown HomeRow {home:?}", base.type_));
        0
    });
    CombatRoleInfo {
        attack_percent: rd.int("iAttackPercent"),
        defense_percent: rd.int("iDefensePercent"),
        max_hp: rd.int("iMaxHP"),
        speed: rd.int("iSpeed"),
        dodge: rd.int("iDodge"),
        crit_bonus: rd.int("iCritBonus"),
        anti_flank: rd.bool("bAntiFlank"),
        rows,
        home_row,
        auto_pick: rd.int("iAutoPick"),
        default_unit_combats: rd.ref_list("DefaultUnitCombats"),
        default_free_promotion: rd.rf("DefaultFreePromotion"),
        from_role: rd.rf("FromRole"),
        from_promotion: rd.rf("FromPromotion"),
        default: rd.bool("bDefault"),
        base,
    }
}

/// A unit type's own value in place of its role's (`iEngageSpeed`, `iEngageDodge`,
/// `iEngageCritBonus` in `CIV4UnitInfos.xml`): `None` at -1, the default (the role's value), else
/// the value, which must lie in `valid`; one outside both warns and reads as the role's.
pub fn read_engage_override(rd: &Rd, tag: &str, unit_type: &str, valid: std::ops::RangeInclusive<i32>) -> Option<i32> {
    let v = rd.int_or(tag, -1);
    if v == -1 {
        return None;
    }
    if !valid.contains(&v) {
        rd.cx.warn(format!("{unit_type}: {tag} {v} is neither -1 (the role's) nor in {}..={}", valid.start(), valid.end()));
        return None;
    }
    Some(v)
}

/// A unit type's `EngageRows` (`CIV4UnitInfos.xml`): three `Row` elements, front, middle and rear,
/// each an `Action` and its `iCount`, which replace its role's rows. `None` without the tag: the
/// role's rows. Rows that are not exactly three, an unknown `Action`, or rows the roles' checks
/// refuse ([`rows_problem`]) warn, and the unit keeps its role's rows.
pub fn read_engage_rows(rd: &Rd, unit_type: &str) -> Option<[RowActions; ROWS]> {
    let list = rd.child("EngageRows")?;
    let rows: Vec<Rd> = list.children_named("Row").collect();
    if rows.len() != ROWS {
        rd.cx.warn(format!("{unit_type}: EngageRows has {} Row elements ({ROWS} expected: front, middle, rear)", rows.len()));
        return None;
    }
    let mut out = [RowActions::default(); ROWS];
    for (i, row) in rows.iter().enumerate() {
        let token = row.string("Action");
        let Some(kind) = ActionKind::parse(&token) else {
            rd.cx.warn(format!("{unit_type}: EngageRows, {} row: unknown Action {token:?}", ROW_TOKENS[i]));
            return None;
        };
        out[i] = RowActions { kind, count: row.int("iCount") };
    }
    if let Some(problem) = rows_problem(&out) {
        rd.cx.warn(format!("{unit_type}: EngageRows: {problem}"));
        return None;
    }
    Some(out)
}
