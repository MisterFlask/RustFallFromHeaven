//! This game's own info classes for cultural channels (not FFH2's; docs/CULTURE.md):
//! `Gameinfo/CIV4CulturalActionInfos.xml`.

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;

define_idx!(/// What a city may channel its culture into (`CULTURALACTION_*`, Gameinfo/CIV4CulturalActionInfos.xml).
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
    /// Revolt pressure on the target's cities: unhappiness, disorder, then the city joins the
    /// caster.
    #[default]
    FomentRevolt,
    /// A chance a turn that a religion of the channelling city spreads to one of the target's
    /// cities.
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
        "FOMENT_REVOLT" => CultureEffect::FomentRevolt,
        "SPREAD_RELIGION" => CultureEffect::SpreadReligion,
        other => {
            rd.cx.warn(format!("{}: unknown Effect {other:?}", base.type_));
            CultureEffect::FomentRevolt
        }
    };
    CulturalActionInfo { base, hostile, effect }
}
