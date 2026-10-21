//! What a resource does: C++ `CvGameTextMgr::setBonusHelp` (static).

use ffh_data::*;

/// What a resource `b` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, b: BonusType) -> String {
    let _ = (d, b);
    String::new()
}
