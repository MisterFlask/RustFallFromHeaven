//! This game's own info classes for cultural actions (not FFH2's; docs/CULTURE.md):
//! `Gameinfo/CIV4CulturalActionInfos.xml`.

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;

define_idx!(/// A cultural action (`CULTURALACTION_*`, Gameinfo/CIV4CulturalActionInfos.xml).
CulturalActionType);

/// What a cultural action may be aimed at (XML `Target`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CultureTargetKind {
    /// A city of the caster's team, or of a team it has a defensive pact with.
    #[default]
    OwnOrAllyCity,
    /// A city of another team (or of the barbarians).
    ForeignCity,
    /// Any city.
    AnyCity,
}

/// When an action counts as hostile to the target's owner (XML `Hostile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CultureHostility {
    #[default]
    Never,
    Always,
    /// Only when the target's owner has a state religion other than the one spread.
    DifferentStateReligion,
}

/// What completing the action does (XML `Effect`); the magnitudes are `CULTURE_*` defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CultureEffect {
    /// +happiness and resistance to hostile actions for a while.
    #[default]
    Festival,
    /// Trade routes between the target and the caster's cities yield more for a while.
    TradeFair,
    /// A chance to spread a religion of the casting city to the target.
    SpreadReligion,
    /// +unhappiness in the target for a while.
    InciteUnrest,
}

/// One cultural action (this game's `CvCulturalActionInfo`).
#[derive(Debug, Clone, Default)]
pub struct CulturalActionInfo {
    pub base: InfoBase,
    /// 1-3: the casting city's culture level, the range and the minimum share it needs.
    pub tier: i32,
    /// Culture diverted a turn while channelling (R).
    pub rate: i32,
    /// Turns of channelling at normal speed (D).
    pub duration: i32,
    pub target: CultureTargetKind,
    pub hostile: CultureHostility,
    pub effect: CultureEffect,
    /// The magnitude or chance is multiplied by the effectiveness E (foreign targets only).
    pub scaled_by_effect: bool,
    /// For the future threat score; nothing reads it yet.
    pub threat_weight: i32,
    /// Memory points the target's owner keeps when the action completes.
    pub memory_weight: i32,
}

pub fn read_cultural_action(rd: &Rd) -> CulturalActionInfo {
    let base = InfoBase::read(rd);
    let target = match rd.string("Target").as_str() {
        "OWN_OR_ALLY_CITY" => CultureTargetKind::OwnOrAllyCity,
        "FOREIGN_CITY" => CultureTargetKind::ForeignCity,
        "ANY_CITY" => CultureTargetKind::AnyCity,
        other => {
            rd.cx.warn(format!("{}: unknown Target {other:?}", base.type_));
            CultureTargetKind::OwnOrAllyCity
        }
    };
    let hostile = match rd.string("Hostile").as_str() {
        "NEVER" | "" => CultureHostility::Never,
        "ALWAYS" => CultureHostility::Always,
        "DIFFERENT_STATE_RELIGION" => CultureHostility::DifferentStateReligion,
        other => {
            rd.cx.warn(format!("{}: unknown Hostile {other:?}", base.type_));
            CultureHostility::Never
        }
    };
    let effect = match rd.string("Effect").as_str() {
        "FESTIVAL" => CultureEffect::Festival,
        "TRADE_FAIR" => CultureEffect::TradeFair,
        "SPREAD_RELIGION" => CultureEffect::SpreadReligion,
        "INCITE_UNREST" => CultureEffect::InciteUnrest,
        other => {
            rd.cx.warn(format!("{}: unknown Effect {other:?}", base.type_));
            CultureEffect::Festival
        }
    };
    CulturalActionInfo {
        base,
        tier: rd.int("iTier").clamp(1, 3),
        rate: rd.int("iRate").max(0),
        duration: rd.int("iDuration").max(1),
        target,
        hostile,
        effect,
        scaled_by_effect: rd.bool("bScaledByEffect"),
        threat_weight: rd.int("iThreatWeight"),
        memory_weight: rd.int("iMemoryWeight"),
    }
}
