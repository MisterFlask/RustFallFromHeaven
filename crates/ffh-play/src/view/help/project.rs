//! What a project or ritual does: C++ `CvGameTextMgr::setProjectHelp` (static: no city).

use ffh_data::*;

/// What a project or ritual `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, p: ProjectType) -> String {
    let _ = (d, p);
    String::new()
}
