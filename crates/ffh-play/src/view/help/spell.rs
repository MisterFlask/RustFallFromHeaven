//! What a spell does: C++ `CvGameTextMgr::parseSpellHelp` (FfH).

use ffh_data::*;

/// What a spell `s` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, s: SpellType) -> String {
    let _ = (d, s);
    String::new()
}
