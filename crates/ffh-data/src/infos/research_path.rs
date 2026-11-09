//! This game's own info class for research paths (not FFH2's; docs/PORTING.md, "Changes from
//! FFH2"): `research_paths.jsonc`. A player commits to one path at a time and researches its
//! techs faster (`RESEARCH_PATH_PERCENT`).

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::idx::IdxVec;
use crate::registry::Rd;

define_idx!(/// A research path (`RESEARCHPATH_*`, research_paths.jsonc).
ResearchPathType);

/// One research path (this game's `ResearchPathInfo`).
#[derive(Debug, Clone, Default)]
pub struct ResearchPathInfo {
    pub base: InfoBase,
    /// The techs the commitment speeds up. A tech may be on several paths.
    pub techs: Vec<TechType>,
    /// The tech each alignment adds to the path (the Faith path: Righteousness for the good,
    /// Commune with Nature for the neutral, Malevolent Designs for the evil). It follows the
    /// player's alignment of the moment.
    pub alignment_techs: IdxVec<AlignmentType, Option<TechType>>,
    /// The path's late techs: a civilization may commit to the path only if one of them (or the
    /// path's tech for its alignment) unlocks a unit it can train, unless none of them unlocks a
    /// unit for any civilization (a path of civics and buildings).
    pub capstones: Vec<TechType>,
    /// The resource the commitment grants at the capital, kept after the path is complete. It
    /// counts only once the team knows the techs that reveal it and let it be traded
    /// (`BonusInfo::tech_reveal`, `tech_city_trade`).
    pub bonus: Option<BonusType>,
}

pub fn read_research_path(rd: &Rd) -> ResearchPathInfo {
    ResearchPathInfo {
        base: InfoBase::read(rd),
        techs: rd.ref_list("techs"),
        alignment_techs: rd.pairs_ref("alignment_techs"),
        capstones: rd.ref_list("capstones"),
        bonus: rd.rf("bonus"),
    }
}
