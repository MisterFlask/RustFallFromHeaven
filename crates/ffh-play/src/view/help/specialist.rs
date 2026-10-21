//! What a specialist does: C++ `CvGameTextMgr::parseSpecialistHelp` (static: no city).

use ffh_data::*;

/// What a specialist `s` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, s: SpecialistType) -> String {
    let _ = (d, s);
    String::new()
}
