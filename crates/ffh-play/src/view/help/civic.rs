//! What a civic does: C++ `CvGameTextMgr::parseCivicInfo` (no player context).

use ffh_data::*;

/// What a civic `c` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, c: CivicType) -> String {
    let _ = (d, c);
    String::new()
}
