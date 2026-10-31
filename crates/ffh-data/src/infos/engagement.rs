//! This game's own info classes for engagement combat (not FFH2's; phase 6 of the owner's spec,
//! `GAMEOPTION_ENGAGEMENTS`, docs/ENGAGEMENTS.md): `Units/CIV4CombatRoleInfos.xml` (the roles of
//! formation battles, revision 4) and `Units/CIV4StanceInfos.xml` (the round model's stances,
//! which go with it). The role of each unit type is derived at load (`GameData::unit_roles`,
//! lib.rs).

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::registry::Rd;

define_idx!(/// An engagement role (`COMBATROLE_*`, Units/CIV4CombatRoleInfos.xml).
CombatRoleType);
define_idx!(/// An engagement stance (`STANCE_*`, Units/CIV4StanceInfos.xml).
StanceType);

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
    /// extra action.
    pub fn best_row(&self) -> usize {
        let mut best = 0;
        for (i, r) in self.rows.iter().enumerate() {
            if r.count > self.rows[best].count {
                best = i;
            }
        }
        best
    }

    /// What a unit of the role does from each row, with `extra` more actions in its best row
    /// (tier 4 units and heroes get `ENGAGEMENT_EXTRA_ACTIONS`); a row without an action stays
    /// without.
    pub fn actions(&self, extra: i32) -> [RowActions; ROWS] {
        let mut rows = self.rows;
        let best = self.best_row();
        if rows[best].kind != ActionKind::None {
            rows[best].count += extra;
        }
        rows
    }

    /// A flanker: one of its rows charges (anti-flank units in its reach are taken first).
    pub fn flanks(&self) -> bool {
        self.rows.iter().any(|r| r.kind == ActionKind::Charge)
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

// The round model's stances (phase 6 before revision 4). Formation battles have none: they go
// with the round model (docs/ENGAGEMENTS.md, "What is left of the round model").

/// One step of a stance's pick (XML `Steps/Step`): take the best units of `roles` until the pick
/// holds `max(count, ceil(S x percent / 100))` units of those roles, S the side's slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StanceStep {
    /// `Roles/Role`.
    pub roles: Vec<CombatRoleType>,
    /// `iCount`: at least this many.
    pub count: i32,
    /// `iPercent`: at least this share of the slots (100: as many as there are).
    pub percent: i32,
}

/// An engagement stance (this game's `StanceInfo`): which units a side commits, by role.
#[derive(Debug, Clone, Default)]
pub struct StanceInfo {
    /// `Description` names the stance, `Help` says what it does.
    pub base: InfoBase,
    /// `bDefault`: the stance of new armies and garrisons and of units in no army.
    pub default: bool,
    /// `iMaxLevel` (-1: none): units above this level are held (committed only when the slots
    /// would otherwise go empty).
    pub max_level: i32,
    /// `bHoldHeroes`: heroes are held too.
    pub hold_heroes: bool,
    /// `Steps/Step`, in order.
    pub steps: Vec<StanceStep>,
    /// `Fill/Role`: the order the slots left after the steps are filled in.
    pub fill: Vec<CombatRoleType>,
}

pub fn read_stance(rd: &Rd) -> StanceInfo {
    let base = InfoBase::read(rd);
    let mut steps = Vec::new();
    if let Some(list) = rd.child("Steps") {
        for s in list.children_named("Step") {
            let step = StanceStep { roles: s.ref_list("Roles"), count: s.int("iCount"), percent: s.int("iPercent") };
            if step.roles.is_empty() || step.count < 0 || !(0..=100).contains(&step.percent) {
                rd.cx.warn(format!("{}: bad Step {step:?}", base.type_));
            }
            steps.push(step);
        }
    }
    StanceInfo {
        default: rd.bool("bDefault"),
        max_level: rd.int_or("iMaxLevel", -1),
        hold_heroes: rd.bool("bHoldHeroes"),
        steps,
        fill: rd.ref_list("Fill"),
        base,
    }
}
