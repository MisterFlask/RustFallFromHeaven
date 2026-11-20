//! This game's own info classes for cultural channels (not FFH2's; docs/CULTURE.md):
//! `cultural_actions.jsonc`.

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;

define_idx!(/// What a civilization may channel its culture into (`CULTURALACTION_*`, cultural_actions.jsonc).
CulturalActionType);

/// When a channel counts as hostile to its target (XML `Hostile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CultureHostility {
    #[default]
    Never,
    Always,
    /// Only when the target has a state religion other than the one spread.
    DifferentStateReligion,
}

/// What a channel does (XML `Effect`); the magnitudes are `CULTURE_*` defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CultureEffect {
    /// A House of another realm: its favour toward the caster, which splits it at the requirement
    /// (docs/PERSONS.md, "The fourth build", rules 2 to 4).
    #[default]
    Court,
    /// One of the caster's own Houses: every courter's favour at it falls (rule 5).
    Guard,
    /// A chance a turn that a religion of the caster's cities spreads to one of the target's
    /// cities (rule 6).
    SpreadReligion,
}

/// One kind of cultural channel (this game's `CvCulturalActionInfo`).
#[derive(Debug, Clone, Default)]
pub struct CulturalActionInfo {
    pub base: InfoBase,
    pub hostile: CultureHostility,
    pub effect: CultureEffect,
}

pub fn read_cultural_action(rd: &Rd) -> CulturalActionInfo {
    let base = InfoBase::read(rd);
    let hostile = match rd.string("hostile").as_str() {
        "NEVER" | "" => CultureHostility::Never,
        "ALWAYS" => CultureHostility::Always,
        "DIFFERENT_STATE_RELIGION" => CultureHostility::DifferentStateReligion,
        other => {
            rd.cx.warn(format!("{}: unknown Hostile {other:?}", base.type_));
            CultureHostility::Never
        }
    };
    let effect = match rd.string("effect").as_str() {
        "COURT" => CultureEffect::Court,
        "GUARD" => CultureEffect::Guard,
        "SPREAD_RELIGION" => CultureEffect::SpreadReligion,
        other => {
            rd.cx.warn(format!("{}: unknown Effect {other:?}", base.type_));
            CultureEffect::Court
        }
    };
    CulturalActionInfo { base, hostile, effect }
}
