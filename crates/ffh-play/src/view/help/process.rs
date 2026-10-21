//! What a process does: C++ `CvGameTextMgr::setProcessHelp`.

use ffh_data::*;

/// What a process `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, p: ProcessType) -> String {
    let _ = (d, p);
    String::new()
}
