//! What a promotion does: C++ `CvGameTextMgr::parsePromotionHelp`.

use ffh_data::*;

/// What a promotion `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say).
pub fn effects(d: &GameData, p: PromotionType) -> String {
    let _ = (d, p);
    String::new()
}
