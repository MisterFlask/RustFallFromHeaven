//! What a building does: C++ `CvGameTextMgr::setBuildingHelp` (static: no city, no player) and `buildBuildingRequiresString`.

use ffh_data::*;

/// What a building `b` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, b: BuildingType) -> String {
    let _ = (d, b);
    String::new()
}
