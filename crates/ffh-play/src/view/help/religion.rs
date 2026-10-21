//! What a religion does: C++ `CvGameTextMgr::setReligionHelp`, and what requires the religion.

use ffh_data::*;

/// What a religion `r` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, r: ReligionType) -> String {
    let _ = (d, r);
    String::new()
}
