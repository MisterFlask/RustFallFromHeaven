//! What a process does: C++ `CvGameTextMgr::setProcessHelp`, without its name line (the page
//! shows the name): the share of the city's production it turns into each commerce.

use ffh_data::*;

use super::{finish, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};

/// What a process `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say). Every field of the process is either described here or named as not shown.
pub fn effects(d: &GameData, p: ProcessType) -> String {
    let ProcessInfo {
        base,
        // C++ shows no tech line: a city offers only the processes its owner can run, and the
        // tech's help lists the processes it allows (`setTechHelp`, "Can Build").
        tech_prereq: _,
        production_to_commerce_modifier,
    } = &d.processes[p];
    let mut b = String::new();
    for k in 0..CommerceType::NUM {
        let n = production_to_commerce_modifier[k];
        if n != 0 {
            b.push_str(NEWLINE);
            b.push_str(&txt(
                d,
                "TXT_KEY_PROCESS_CONVERTS",
                "[ICON_BULLET]Converts %d1_Mod%% of %F2_Type1 to %F3_Type2",
                &[Arg::N(n), Arg::S(YIELD_ICONS[YieldType::YIELD_PRODUCTION.idx()]), Arg::S(COMMERCE_ICONS[k])],
            ));
        }
    }
    // C++ shows no XML help here; kept at the end so that the tooltip, which shows `effects` in
    // place of the help, does not lose it (no FFH2 process has one).
    let help = crate::names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        b.push_str(NEWLINE);
        b.push_str(help);
    }
    finish(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_say_what_they_convert() {
        let d = ffh_data::global();
        let wealth = effects(d, d.t::<ProcessType>("PROCESS_WEALTH"));
        assert_eq!(wealth, "[ICON_BULLET]Converts 50% of [ICON_PRODUCTION] to [ICON_GOLD]");
        let research = effects(d, d.t::<ProcessType>("PROCESS_RESEARCH"));
        assert!(research.contains("[ICON_PRODUCTION] to [ICON_RESEARCH]"), "{research}");
        for p in d.processes.keys() {
            let t = effects(d, p);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{t}");
            // Every process converts something.
            if d.processes[p].production_to_commerce_modifier.iter().any(|&n| n != 0) {
                assert!(t.contains("[ICON_BULLET]Converts"), "{}: {t}", crate::names::name(d, p));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for p in d.processes.keys() {
            println!("{}:\n  {}\n", crate::names::name(d, p), effects(d, p).replace(NEWLINE, "\n  "));
        }
    }
}
