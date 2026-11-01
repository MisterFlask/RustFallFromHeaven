//! What a religion does: C++ `CvGameTextMgr::setReligionHelp`, static (no city, no active player:
//! the founder's free unit is its class's default), without its name line (the page shows the
//! name). FfH2's religions matter mostly through what they unlock, which its help leaves out, so
//! ffh-play adds, from the data: its alignment and what adopting it does to the leader's, its
//! FfH2 traits (hidden, sneak attack, sight of its cities, the Armageddon Counter), its heroes,
//! and the units, buildings, techs, promotions, civics and spells that require it, grouped by
//! what they require (it as the state religion, in the city, its holy city, or a caster of it).

use ffh_data::*;

use super::{changes_help, color, finish, txt, Arg, COMMERCE_ICONS, NEWLINE};
use crate::names::{self, display};

/// What a religion requires of an item.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Need {
    /// It as state religion, and in the city.
    StateInCity,
    /// It in the city.
    InCity,
    /// It as state religion, and its holy city.
    StateHolyCity,
    /// Its holy city.
    HolyCity,
    /// It as state religion.
    State,
    /// A caster of the religion (FfH spells: the unit's religion).
    Caster,
    /// A caster of the religion, with it as state religion.
    StateCaster,
}

impl Need {
    const ALL: [Need; 7] = [Need::StateInCity, Need::InCity, Need::StateHolyCity, Need::HolyCity, Need::State, Need::Caster, Need::StateCaster];

    /// From whether an item requires it as state religion, in the city and its holy city.
    fn of(state: bool, in_city: bool, holy_city: bool) -> Option<Need> {
        Some(match (state, in_city, holy_city) {
            (true, _, true) => Need::StateHolyCity,
            (false, _, true) => Need::HolyCity,
            (true, true, false) => Need::StateInCity,
            (false, true, false) => Need::InCity,
            (true, false, false) => Need::State,
            (false, false, false) => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Need::StateInCity => "With it as state religion and in the city",
            Need::InCity => "With it in the city",
            Need::StateHolyCity => "With it as state religion, in its holy city",
            Need::HolyCity => "In its holy city",
            Need::State => "With it as state religion",
            Need::Caster => "For units of this religion",
            Need::StateCaster => "For units of this religion, with it as state religion",
        }
    }
}

/// The kinds of items listed, in their order.
const KINDS: [&str; 6] = ["units", "buildings", "techs", "promotions", "civics", "spells"];

/// What a religion `r` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing
/// to say). Every field of the religion is either described here or named as not shown.
pub fn effects(d: &GameData, r: ReligionType) -> String {
    let ReligionInfo {
        base,
        tech_prereq,
        free_unit_class,
        num_free_units,
        // The weight of its own spread to nearby cities (C++ `CvCity::doReligion`), against the
        // other religions': C++ shows no line for it.
        spread_factor: _,
        // What a shrine earns per city with the religion: the shrine's help says it (C++
        // `setBuildingHelp`, `getGlobalReligionCommerce`).
        global_religion_commerce: _,
        holy_city_commerce,
        state_religion_commerce,
        // For game messages.
        adjective_key: _,
        hidden,
        sneak_attack,
        update_sight,
        global_counter_found,
        global_counter_spread,
        alignment,
        alignment_best,
        alignment_worst,
        religion_hero1,
        religion_hero2,
        // Better AI: the techs its followers' AI favours.
        religion_tech1: _,
        religion_tech2: _,
    } = &d.religions[r];
    let mut b = String::new();
    let line = |b: &mut String, s: String| {
        b.push_str(NEWLINE);
        b.push_str(&s);
    };
    changes_help(&mut b, &txt(d, "TXT_KEY_RELIGION_HOLY_CITY", &[]), ": ", "", holy_city_commerce, &COMMERCE_ICONS, false, true);
    changes_help(&mut b, &txt(d, "TXT_KEY_RELIGION_ALL_CITIES", &[]), ": ", "", state_religion_commerce, &COMMERCE_ICONS, false, true);
    if let Some(t) = tech_prereq {
        line(&mut b, txt(d, "TXT_KEY_RELIGION_FOUNDED_FIRST", &[Arg::S(&display(d, *t))]));
    }
    if let Some(u) = free_unit_class.and_then(|c| d.unit_classes[c].default_unit_index) {
        if *num_free_units > 1 {
            line(
                &mut b,
                txt(
                    d,
                    "TXT_KEY_RELIGION_FOUNDER_RECEIVES_NUM",
                    &[Arg::S(&display(d, u)), Arg::N(*num_free_units)],
                ),
            );
        } else if *num_free_units > 0 {
            line(&mut b, txt(d, "TXT_KEY_RELIGION_FOUNDER_RECEIVES", &[Arg::S(&display(d, u))]));
        }
    }

    // ffh-play's, from here on.
    if let Some(a) = alignment {
        line(&mut b, format!("[ICON_BULLET]{}", txt(d, "TXT_KEY_ALIGNMENT", &[Arg::S(&display(d, *a))])));
    }
    // C++ `CvPlayer::convert`: the leader's alignment is pulled up to the best and down to the
    // worst (alignments order good < neutral < evil).
    let mut moves: Vec<(AlignmentType, Vec<String>)> = Vec::new();
    for a in d.alignments.keys() {
        let mut to = Some(a);
        if to < *alignment_best {
            to = *alignment_best;
        }
        if to > *alignment_worst {
            to = *alignment_worst;
        }
        if let Some(to) = to.filter(|&t| t != a) {
            match moves.iter_mut().find(|(t, _)| *t == to) {
                Some((_, from)) => from.push(display(d, a).into_owned()),
                None => moves.push((to, vec![display(d, a).into_owned()])),
            }
        }
    }
    if !moves.is_empty() {
        let parts: Vec<String> = moves.iter().map(|(to, from)| format!("{} leaders become {}", from.join(" and "), display(d, *to))).collect();
        line(&mut b, format!("[ICON_BULLET]Adopting it as state religion: {}", parts.join("; ")));
    }
    // C++ `CvPlayer::canSeeReligion`, `CvUnit::canEnterTerritory` (while declaring war),
    // `CvPlot::updateSight`, `CvGame::setHolyCity` and `CvCity::setHasReligion`.
    if *hidden {
        line(&mut b, "[ICON_BULLET]Hidden: rivals do not see it in your cities unless it is their state religion".to_string());
    }
    if *sneak_attack {
        line(&mut b, "[ICON_BULLET]As state religion: declaring war does not expel your units from the enemy's lands".to_string());
    }
    if *update_sight {
        line(&mut b, "[ICON_BULLET]The owner of its holy city, with it as state religion, sees around every city it has spread to".to_string());
    }
    let counter = |n: i32| txt(d, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER", &[Arg::N(n)]);
    if *global_counter_found != 0 {
        line(&mut b, counter(*global_counter_found) + " when founded");
    }
    if *global_counter_spread != 0 {
        line(&mut b, counter(*global_counter_spread) + " for each city it spreads to");
    }

    // Its heroes (Better AI's `getReligionHero1/2`), then what requires it.
    let heroes: Vec<UnitClassType> = [*religion_hero1, *religion_hero2].into_iter().flatten().collect();
    let hero_names: Vec<String> = heroes.iter().filter_map(|&c| d.unit_classes[c].default_unit_index).map(|u| color("COLOR_UNIT_TEXT", &display(d, u))).collect();
    if !hero_names.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Heroes: {}", hero_names.join(", ")));
    }
    // (need, kind, coloured name), deduplicated by name.
    let mut items: Vec<(Need, usize, String)> = Vec::new();
    let mut add = |need: Option<Need>, kind: usize, name: String| {
        if let Some(need) = need {
            if !items.iter().any(|(n, k, s)| *n == need && *k == kind && *s == name) {
                items.push((need, kind, name));
            }
        }
    };
    // C++ `CvPlayer::canTrain` (`getStateReligion`), `CvPlot::canTrain` (`getPrereqReligion`,
    // `getHolyCity`).
    for u in d.units.keys() {
        let ui = &d.units[u];
        if ui.unit_class_type.is_some_and(|c| heroes.contains(&c)) {
            continue;
        }
        let need = Need::of(ui.state_religion == Some(r), ui.prereq_religion == Some(r), ui.holy_city == Some(r));
        add(need, 0, color("COLOR_UNIT_TEXT", &display(d, u)));
    }
    // C++ `CvPlayer::canConstruct` (`getStateReligion`), `CvCity::canConstruct`
    // (`getPrereqReligion`, `getHolyCity`).
    for bl in d.buildings.keys() {
        let bi = &d.buildings[bl];
        let need = Need::of(bi.state_religion == Some(r), bi.prereq_religion == Some(r), bi.holy_city == Some(r));
        add(need, 1, color("COLOR_BUILDING_TEXT", &display(d, bl)));
    }
    // FfH: techs (`CvPlayer::canResearch`), promotions (`CvUnit::canAcquirePromotion`), civics
    // (`CvPlayer::canDoCivics`) and spells (`CvUnit::canCast`) of a state religion; spells of a
    // caster of the religion.
    for t in d.techs.keys().filter(|&t| d.techs[t].prereq_religion == Some(r)) {
        add(Some(Need::State), 2, color("COLOR_TECH_TEXT", &display(d, t)));
    }
    for p in d.promotions.keys().filter(|&p| d.promotions[p].state_religion_prereq == Some(r)) {
        add(Some(Need::State), 3, color("COLOR_HIGHLIGHT_TEXT", &display(d, p)));
    }
    for c in d.civics.keys().filter(|&c| d.civics[c].prereq_religion == Some(r)) {
        add(Some(Need::State), 4, color("COLOR_HIGHLIGHT_TEXT", &display(d, c)));
    }
    for s in d.spells.keys() {
        let si = &d.spells[s];
        let need = match (si.state_religion_prereq == Some(r), si.religion_prereq == Some(r)) {
            (true, true) => Some(Need::StateCaster),
            (true, false) => Some(Need::State),
            (false, true) => Some(Need::Caster),
            (false, false) => None,
        };
        add(need, 5, color("COLOR_HIGHLIGHT_TEXT", &display(d, s)));
    }
    for need in Need::ALL {
        let parts: Vec<String> = (0..KINDS.len())
            .filter_map(|k| {
                let list: Vec<&str> = items.iter().filter(|(n, kind, _)| *n == need && *kind == k).map(|(_, _, s)| s.as_str()).collect();
                (!list.is_empty()).then(|| format!("{} {}", KINDS[k], list.join(", ")))
            })
            .collect();
        if !parts.is_empty() {
            line(&mut b, format!("[ICON_BULLET]{}: {}", need.label(), parts.join("; ")));
        }
    }
    // C++ shows no XML help here; kept at the end so that the tooltip, which shows `effects` in
    // place of the help, does not lose it (no FFH2 religion has one).
    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        line(&mut b, help.to_string());
    }
    finish(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn religion(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<ReligionType>(name))
    }

    #[test]
    fn known_religions_say_what_they_bring() {
        let order = religion("RELIGION_THE_ORDER");
        for part in [
            "[ICON_BULLET]Holy City (if State [ICON_RELIGION]): +4[ICON_CULTURE]",
            "[ICON_BULLET]All Cities (if State [ICON_RELIGION]): +1[ICON_CULTURE]",
            "Founded by First to Discover [COLOR_TECH_TEXT]Orders from Heaven[COLOR_REVERT]",
            "[ICON_BULLET]Founder Receives a ",
            "[ICON_BULLET]Alignment: Good",
            "[ICON_BULLET]Adopting it as state religion: Neutral and Evil leaders become Good",
            "[ICON_BULLET]Heroes: [COLOR_UNIT_TEXT]",
            "[ICON_BULLET]With it as state religion and in the city: units [COLOR_UNIT_TEXT]",
            "[ICON_BULLET]With it in the city: units ",
            "[COLOR_BUILDING_TEXT]Temple of the Order[COLOR_REVERT]",
            "[ICON_BULLET]In its holy city: buildings [COLOR_BUILDING_TEXT]",
            "[ICON_BULLET]With it as state religion: ",
            "[COLOR_HIGHLIGHT_TEXT]Social Order[COLOR_REVERT]",
            "[ICON_BULLET]For units of this religion: spells ",
        ] {
            assert!(order.contains(part), "{part}: {order}");
        }
        // The heroes are named once.
        let d = ffh_data::global();
        let valin = display(d, d.t::<UnitType>("UNIT_VALIN")).into_owned();
        assert_eq!(order.matches(valin.as_str()).count(), 1, "{order}");

        let esus = religion("RELIGION_COUNCIL_OF_ESUS");
        for part in ["[ICON_BULLET]Hidden", "declaring war does not expel", "sees around every city", "Good leaders become Neutral"] {
            assert!(esus.contains(part), "{part}: {esus}");
        }
        // Steal needs both a caster of the Council and it as state religion: listed once.
        let steal = display(d, d.t::<SpellType>("SPELL_STEAL")).into_owned();
        assert_eq!(esus.matches(steal.as_str()).count(), 1, "{esus}");
        assert!(esus.contains("[ICON_BULLET]For units of this religion, with it as state religion: spells "), "{esus}");
        let veil = religion("RELIGION_THE_ASHEN_VEIL");
        assert!(veil.contains("[ICON_BULLET]+5 to the Armageddon Counter when founded"), "{veil}");
        assert!(veil.contains("[ICON_BULLET]+1 to the Armageddon Counter for each city it spreads to"), "{veil}");
        assert!(veil.contains("techs [COLOR_TECH_TEXT]Infernal Pact[COLOR_REVERT]"), "{veil}");
        // Fellowship: neutral, and adopting it changes no alignment.
        let leaves = religion("RELIGION_FELLOWSHIP_OF_LEAVES");
        assert!(leaves.contains("Alignment: Neutral") && !leaves.contains("leaders become"), "{leaves}");
    }

    #[test]
    fn every_religion_reads_well() {
        let d = ffh_data::global();
        for r in d.religions.keys() {
            let t = effects(d, r);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{}: {t}", names::name(d, r));
            assert!(!t.starts_with(NEWLINE) && !t.contains("[NEWLINE][NEWLINE]"), "{t}");
            // Every FfH2 religion is founded by a tech and unlocks units.
            assert!(t.contains("Founded by First to Discover") && t.contains("units "), "{t}");
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for r in d.religions.keys() {
            println!("{}:\n  {}\n", names::name(d, r), effects(d, r).replace(NEWLINE, "\n  "));
        }
    }
}
