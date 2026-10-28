//! What a project or ritual does: C++ `CvGameTextMgr::setProjectHelp`, static (no city, no active
//! player), as a city's tooltip shows it without the name line (the page shows the name) and the
//! cost line (the city shows its own cost: the game speed and its modifiers change it). Of the
//! Civilopedia's text it takes the victory requirement and the resource speed-ups as bulleted
//! lines. FfH2's rituals act in Python (`onProjectBuilt`): their strategy text says what they do.

use ffh_data::*;

use super::{color, finish, list_help, req, txt, Arg, NEWLINE};
use crate::names::{self, display};

/// What a project or ritual `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is
/// nothing to say). Every field of the project is either described here or named as not shown.
pub fn effects(d: &GameData, p: ProjectType) -> String {
    let ProjectInfo {
        base,
        victory_prereq,
        // C++ shows no tech line: a city offers only the projects its owner has the tech for, and
        // the tech's help lists the projects it allows (`setTechHelp`, "Can Create").
        tech_prereq: _,
        anyone_project_prereq,
        max_global_instances,
        max_team_instances,
        // The city shows the cost it pays (C++ `getProductionNeeded`).
        production_cost: _,
        nuke_interception,
        tech_share,
        everyone_special_unit,
        everyone_special_building,
        // The BtS space race (a part's victory delay and launch success): no FFH2 project sets
        // them, and C++ shows them only on the spaceship screen.
        victory_delay_percent: _,
        success_rate: _,
        spaceship: _,
        allows_nukes,
        // AI weight.
        ai_weight: _,
        modify_global_counter,
        prereq_civilization,
        prereq_global_counter,
        bonus_production_modifier,
        victory_threshold,
        victory_min_threshold,
        projects_needed,
    } = &d.projects[p];
    let mut b = String::new();
    let line = |b: &mut String, s: String| {
        b.push_str(NEWLINE);
        b.push_str(&s);
    };
    // C++ `isWorldProject`, `isTeamProject`.
    if *max_global_instances != -1 {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_WORLD_NUM_ALLOWED", &[Arg::N(*max_global_instances)]));
    }
    if *max_team_instances != -1 {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_TEAM_NUM_ALLOWED", &[Arg::N(*max_team_instances)]));
    }
    if *nuke_interception != 0 {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_CHANCE_INTERCEPT_NUKES", &[Arg::N(*nuke_interception)]));
    }
    if *tech_share != 0 {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_TECH_SHARE", &[Arg::N(*tech_share)]));
    }
    if *allows_nukes {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_ENABLES_NUKES", &[]));
    }
    let enables_special = |name: &str| txt(d, "TXT_KEY_PROJECT_ENABLES_SPECIAL", &[Arg::S(name)]);
    if let Some(su) = everyone_special_unit {
        line(&mut b, enables_special(&display(d, *su)));
    }
    if let Some(sb) = everyone_special_building {
        line(&mut b, enables_special(&display(d, *sb)));
    }
    for v in d.victories.keys() {
        let (max, min) = (victory_threshold[v], victory_min_threshold[v]);
        if max > 0 {
            let n = if max == min { max.to_string() } else { format!("{min}-{max}") };
            line(&mut b, txt(d, "TXT_KEY_PROJECT_REQUIRED_FOR_VICTORY", &[Arg::S(&n), Arg::S(&display(d, v))]));
        }
    }
    // The projects it unlocks (C++ checks each against the city; without one, all).
    let mut first = true;
    for o in d.projects.keys().filter(|&o| d.projects[o].anyone_project_prereq == Some(p)) {
        let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_PROJECT_REQUIRED_TO_CREATE_ANYONE", &[]));
        list_help(&mut b, &start, &color("COLOR_PROJECT_TEXT", &display(d, o)), ", ", first);
        first = false;
    }
    let mut first = true;
    for o in d.projects.keys().filter(|&o| d.projects[o].projects_needed[p] > 0) {
        let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_PROJECT_REQUIRED_TO_CREATE", &[]));
        list_help(&mut b, &start, &color("COLOR_PROJECT_TEXT", &display(d, o)), ", ", first);
        first = false;
    }
    // FfH.
    if *modify_global_counter != 0 {
        line(&mut b, txt(d, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER", &[Arg::N(*modify_global_counter)]));
    }
    if let Some(civ) = prereq_civilization {
        line(&mut b, txt(d, "TXT_KEY_IMPROVEMENT_PREREQ_CIVILIZATION", &[Arg::S(&display(d, *civ))]));
    }
    if *prereq_global_counter != 0 {
        line(&mut b, req(d, "TXT_KEY_UNIT_PREREQ_GLOBAL_COUNTER", &[Arg::N(*prereq_global_counter)]));
    }
    // What it requires (C++: without a city, every requirement; the banned-nukes line needs one),
    // uncoloured (`super::unwarned`).
    if let Some(o) = anyone_project_prereq {
        line(&mut b, req(d, "TXT_KEY_PROJECT_REQUIRES_ANYONE", &[Arg::S(&display(d, *o))]));
    }
    for o in d.projects.keys() {
        let n = projects_needed[o];
        if n > 0 {
            line(&mut b, req(d, "TXT_KEY_PROJECT_REQUIRES_NO_CITY", &[Arg::S(&display(d, o)), Arg::N(n)]));
        }
    }
    // The Civilopedia's (`bCivilopediaText`) line: the city's tooltip leaves it out.
    if let Some(v) = victory_prereq {
        line(&mut b, txt(d, "TXT_KEY_PROJECT_REQUIRES_STRING_VICTORY", &[Arg::S(&display(d, *v))]));
    }
    // The resources that speed it up, as the Civilopedia writes them (a bulleted line each; the
    // tooltip appends them to the cost line, which is not here).
    for bonus in d.bonuses.keys() {
        let n = bonus_production_modifier[bonus];
        if n != 0 {
            let name = display(d, bonus);
            line(
                &mut b,
                format!(
                    "[ICON_BULLET]{}",
                    if n == 100 {
                        txt(d, "TXT_KEY_PROJECT_DOUBLE_SPEED_WITH", &[Arg::S(&name)])
                    } else {
                        txt(d, "TXT_KEY_PROJECT_BUILDS_FASTER_WITH", &[Arg::N(n), Arg::S(&name)])
                    }
                ),
            );
        }
    }
    // C++ shows no XML help here; kept at the end so that the tooltip, which shows `effects` in
    // place of the help, does not lose it (no FFH2 project has one).
    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        line(&mut b, help.to_string());
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

    fn project(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<ProjectType>(name))
    }

    #[test]
    fn rituals_say_what_they_need() {
        let genesis = project("PROJECT_GENESIS");
        assert_eq!(genesis, "[ICON_BULLET]World Project (1 Allowed)[NEWLINE][ICON_BULLET]Double Production Speed with [LINK=literal]Nature Mana[\\LINK]");
        let ascension = project("PROJECT_ASCENSION");
        for part in ["World Project (1 Allowed)", "Requires Illians civilization", "Requires Someone to Create [LINK=literal]The Draw[\\LINK]"] {
            assert!(ascension.contains(part), "{part}: {ascension}");
        }
        // The Draw unlocks Ascension.
        let draw = project("PROJECT_THE_DRAW");
        assert!(draw.contains("Required for Anyone to Create[SPACE][COLOR_PROJECT_TEXT]Ascension[COLOR_REVERT]"), "{draw}");
        assert!(draw.contains("[ICON_BULLET]+10 to the Armageddon Counter"), "{draw}");
        // Not a world project: no instance line.
        let elegy = project("PROJECT_ELEGY_OF_THE_SHEAIM");
        assert!(!elegy.contains("Allowed") && elegy.contains("+5 to the Armageddon Counter") && elegy.contains("Sheaim"), "{elegy}");
        let bane = project("PROJECT_BANE_DIVINE");
        assert!(bane.contains("Requires an Armageddon Count of +70 or higher") && bane.contains("Death Mana") && bane.contains("Entropy Mana"), "{bane}");
    }

    #[test]
    fn every_project_reads_well() {
        let d = ffh_data::global();
        for p in d.projects.keys() {
            let t = effects(d, p);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{}: {t}", names::name(d, p));
            assert!(!t.starts_with(NEWLINE) && !t.contains("[NEWLINE][NEWLINE]"), "{t}");
            let mut x = d.projects[p].clone();
            x.base = InfoBase::default();
            x.tech_prereq = None;
            x.production_cost = 0;
            x.ai_weight = 0;
            // -1 is "unlimited".
            x.max_global_instances = x.max_global_instances.max(0);
            x.max_team_instances = x.max_team_instances.max(0);
            if sets_anything(&format!("{x:?}")) {
                assert!(!t.is_empty(), "{}", names::name(d, p));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for p in d.projects.keys() {
            println!("{}:\n  {}\n", names::name(d, p), effects(d, p).replace(NEWLINE, "\n  "));
        }
    }
}
