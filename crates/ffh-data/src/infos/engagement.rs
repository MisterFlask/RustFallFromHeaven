//! This game's own info classes for engagement combat (not FFH2's; phase 6 of the owner's spec,
//! `GAMEOPTION_ENGAGEMENTS`, docs/COMBAT_INVENTORY.md): `Units/CIV4CombatRoleInfos.xml` and
//! `Units/CIV4StanceInfos.xml`. The role of each unit type is derived at load
//! (`GameData::unit_roles`, lib.rs).

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::registry::Rd;

define_idx!(/// An engagement role (`COMBATROLE_*`, Units/CIV4CombatRoleInfos.xml).
CombatRoleType);
define_idx!(/// An engagement stance (`STANCE_*`, Units/CIV4StanceInfos.xml).
StanceType);

/// A class of enemy units a role hits, in its priority order (XML `TargetOrder/Target`).
/// A class holds the living, committed, not withdrawn enemies of its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetClass {
    /// `FRONT`: the front-position enemies (every role without `bBack`, screens included).
    Front,
    /// `BACK`: the back-position enemies (`bBack`); empty while an enemy front-position unit
    /// lives, unless the hitter's role ignores the shield.
    Back,
    /// `SCREEN`: the enemy screens (`bScreen`).
    Screen,
    /// `HERO_SUPPORT`: enemy heroes and support units (the assassin's first class from phase 7 on).
    HeroOrSupport,
}

impl TargetClass {
    /// The XML token of the class.
    pub fn token(self) -> &'static str {
        match self {
            TargetClass::Front => "FRONT",
            TargetClass::Back => "BACK",
            TargetClass::Screen => "SCREEN",
            TargetClass::HeroOrSupport => "HERO_SUPPORT",
        }
    }

    fn parse(s: &str) -> Option<TargetClass> {
        match s.trim() {
            "FRONT" => Some(TargetClass::Front),
            "BACK" => Some(TargetClass::Back),
            "SCREEN" => Some(TargetClass::Screen),
            "HERO_SUPPORT" => Some(TargetClass::HeroOrSupport),
            _ => None,
        }
    }
}

/// An engagement role (this game's `CombatRoleInfo`): the unit's multipliers, hit points,
/// position and targets in an engagement.
#[derive(Debug, Clone, Default)]
pub struct CombatRoleInfo {
    pub base: InfoBase,
    /// `iAttackPercent`: the unit's `iCombat` is multiplied by this to give its attack A (100 = 1.0).
    pub attack_percent: i32,
    /// `iDefensePercent`: the unit's `iCombatDefense` is multiplied by this to give its defence Dv.
    pub defense_percent: i32,
    /// `iMaxHP`: the unit's hit points in an engagement (its damage stays a percentage of them).
    pub max_hp: i32,
    /// `bBack`: back position, shielded by its side's front-position units.
    pub back: bool,
    /// `bScreen`: a screen (front position; flankers hit screens first).
    pub screen: bool,
    /// `bIgnoresShield`: may hit back-position units while front-position ones live.
    pub ignores_shield: bool,
    /// `TargetOrder/Target`: the classes it hits, the first non-empty one first.
    pub targets: Vec<TargetClass>,
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

pub fn read_combat_role(rd: &Rd) -> CombatRoleInfo {
    let base = InfoBase::read(rd);
    let mut targets = Vec::new();
    for tok in rd.string_list("TargetOrder") {
        match TargetClass::parse(&tok) {
            Some(t) => targets.push(t),
            None => rd.cx.warn(format!("{}: unknown Target {tok:?}", base.type_)),
        }
    }
    CombatRoleInfo {
        attack_percent: rd.int("iAttackPercent"),
        defense_percent: rd.int("iDefensePercent"),
        max_hp: rd.int("iMaxHP"),
        back: rd.bool("bBack"),
        screen: rd.bool("bScreen"),
        ignores_shield: rd.bool("bIgnoresShield"),
        targets,
        default_unit_combats: rd.ref_list("DefaultUnitCombats"),
        default_free_promotion: rd.rf("DefaultFreePromotion"),
        from_role: rd.rf("FromRole"),
        from_promotion: rd.rf("FromPromotion"),
        default: rd.bool("bDefault"),
        base,
    }
}

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
