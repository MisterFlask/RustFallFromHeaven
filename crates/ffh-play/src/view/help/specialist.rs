//! What a specialist does: C++ `CvGameTextMgr::parseSpecialistHelp`, static (no city, no active
//! player: the yields and commerces of the info itself), without its name line (the page shows
//! the name); then ffh-play's line for the great person its points go to.

use ffh_data::*;

use super::{changes_help, color, finish, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};
use crate::names::{self, display};

/// What a specialist `s` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing
/// to say). Every field of the specialist is either described here or named as not shown.
pub fn effects(d: &GameData, s: SpecialistType) -> String {
    let SpecialistInfo {
        base,
        great_people_unit_class,
        great_people_rate_change,
        experience,
        yield_change,
        commerce_change,
        // AI weights.
        flavor_value: _,
    } = &d.specialists[s];
    let mut b = String::new();
    changes_help(&mut b, "", "", "", yield_change, &YIELD_ICONS, false, true);
    changes_help(&mut b, "", "", "", commerce_change, &COMMERCE_ICONS, false, true);
    if *experience > 0 {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_SPECIALIST_EXPERIENCE", &[Arg::N(*experience)]));
    }
    if *great_people_rate_change != 0 {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_SPECIALIST_BIRTH_RATE", &[Arg::N(*great_people_rate_change)]));
        // ffh-play's: the great person the points count toward (C++ `CvCity::processSpecialist`
        // adds them to its civilization's unit of the class, and `doGreatPeople` draws the born
        // great person by those points); without a civilization, the class's default unit.
        if let Some(u) = great_people_unit_class.and_then(|c| d.unit_classes[c].default_unit_index) {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]Its [ICON_GREATPEOPLE] points count toward a {}", color("COLOR_UNIT_TEXT", &display(d, u))));
        }
    }
    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        b.push_str(NEWLINE);
        b.push_str(help);
    }
    finish(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a value's `Debug` text sets anything: a number other than 0, `true` or `Some`
    /// (field names, which may end in a digit, are words with letters).
    fn sets_anything(debug: &str) -> bool {
        debug.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|w| w == "true" || w == "Some" || (!w.is_empty() && w.bytes().all(|c| c.is_ascii_digit()) && w.bytes().any(|c| c != b'0')))
    }

    #[test]
    fn specialists_say_what_they_produce() {
        let d = ffh_data::global();
        let sage = effects(d, d.t::<SpecialistType>("SPECIALIST_SCIENTIST"));
        assert!(sage.starts_with("[ICON_BULLET]+3[ICON_RESEARCH]"), "{sage}");
        assert!(sage.contains("[ICON_BULLET]+3[ICON_GREATPEOPLE] Birth Rate"), "{sage}");
        assert!(sage.contains("count toward a [COLOR_UNIT_TEXT]"), "{sage}");
        let priest = effects(d, d.t::<SpecialistType>("SPECIALIST_PRIEST"));
        assert!(priest.contains("[ICON_PRODUCTION]") && priest.contains("[ICON_GOLD]"), "{priest}");
        // A great specialist settled in a city: its yields, no birth rate.
        let great = effects(d, d.t::<SpecialistType>("SPECIALIST_GREAT_ENGINEER"));
        assert!(great.contains("+3[ICON_PRODUCTION]") && !great.contains("Birth Rate"), "{great}");
        for sp in d.specialists.keys() {
            let t = effects(d, sp);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{t}");
            let mut x = d.specialists[sp].clone();
            x.base = InfoBase::default();
            x.flavor_value = IdxVec::new();
            if sets_anything(&format!("{x:?}")) {
                assert!(!t.is_empty(), "{}", names::name(d, sp));
            }
        }
    }

    #[test]
    fn debug_texts_that_set_something() {
        assert!(sets_anything("X { a1: 0, b: Some(T(0)) }"));
        assert!(sets_anything("X { a1: 0, b: [0, -2, 0] }"));
        assert!(sets_anything("X { a1: true }"));
        assert!(!sets_anything("X { rand_appearance1: 0, b: None, c: [0, 0], d: false, e: \"\" }"));
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for sp in d.specialists.keys() {
            println!("{}:\n  {}\n", crate::names::name(d, sp), effects(d, sp).replace(NEWLINE, "\n  "));
        }
    }
}
