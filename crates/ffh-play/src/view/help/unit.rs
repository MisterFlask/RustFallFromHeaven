//! What a unit does: C++ `CvGameTextMgr::setBasicUnitHelp` and `setUnitHelp` (static: no city, no player).

use ffh_data::*;

/// What a unit `u` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, u: UnitType) -> String {
    let _ = (d, u);
    String::new()
}
