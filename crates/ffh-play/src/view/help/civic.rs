//! What a civic does: C++ `CvGameTextMgr::parseCivicInfo` with `bCivilopediaText` false,
//! `bPlayerContext` false and `bSkipName` true (the requirement lines are written, the name is not:
//! the page shows it), then ffh-play's lines for what FfH2's help leaves out: a hidden civic, the
//! units it allows and the councils it votes in.
//!
//! Without a player, the state-religion lines speak of "the state religion" (C++'s generic
//! `RELIGION_CHAR` branch), and the numbers C++ takes from the game's world size (the cities
//! counted as the largest, the units drafted) are given for every world size.

use ffh_data::*;

use super::{changes_help, color, finish, list_help, req, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};
use crate::names::{self, display};

/// The lowest and the highest value of a number over the world sizes (C++ takes the game's world
/// size, unknown to the static help).
fn world_size_range(d: &GameData, f: impl Fn(&WorldInfo) -> i32) -> (i32, i32) {
    let v: Vec<i32> = d.worlds.iter().map(f).collect();
    (v.iter().copied().min().unwrap_or(0), v.iter().copied().max().unwrap_or(0))
}

/// Such a range as a `getText` argument: the number when every world size gives it, else the
/// range ("4-6", kept in `text`), with the note to add after the line.
fn range_arg(lo: i32, hi: i32, text: &mut String) -> (Arg<'_>, &'static str) {
    if lo == hi {
        (Arg::N(lo), "")
    } else {
        *text = format!("{lo}-{hi}");
        (Arg::S(text), " (by world size)")
    }
}

/// What a civic `c` does and requires (Civ4 markup, lines joined by `[NEWLINE]`; empty when there
/// is nothing to say).
pub fn effects(d: &GameData, c: CivicType) -> String {
    help(d, c, true)
}

/// What civic `c` does, without what it requires: the civics screen's, which says which of the
/// requirements its player does not meet (C++ with `bPlayerContext` writes only those).
pub fn effects_without_requirements(d: &GameData, c: CivicType) -> String {
    help(d, c, false)
}

/// Every field of the civic is either described here or named as not shown, so that a new field
/// cannot go unnoticed (the build fails until it is listed). `requirements`: with the lines of what
/// it requires.
fn help(d: &GameData, c: CivicType, requirements: bool) -> String {
    let CivicInfo {
        base,
        civic_option_type,
        // The civics screen shows what a switch costs in anarchy and upkeep (`view::player`, the
        // data's `anarchy` and `upkeep`); C++ has no line for them here.
        anarchy_length: _,
        upkeep: _,
        // AI weight.
        ai_weight: _,
        great_people_rate_modifier,
        great_general_rate_modifier,
        domestic_great_general_rate_modifier,
        state_religion_great_people_rate_modifier,
        distance_maintenance_modifier,
        num_cities_maintenance_modifier,
        corporation_maintenance_modifier,
        extra_health,
        free_experience,
        worker_speed_modifier,
        improvement_upgrade_rate_modifier,
        military_production_modifier,
        base_free_units,
        base_free_military_units,
        free_units_population_percent,
        free_military_units_population_percent,
        gold_per_unit,
        gold_per_military_unit,
        happy_per_military_unit,
        largest_city_happiness,
        war_weariness_modifier,
        free_specialist,
        trade_routes,
        tech_prereq,
        civic_percent_anger,
        max_conscript,
        state_religion_happiness,
        non_state_religion_happiness,
        state_religion_unit_production_modifier,
        state_religion_building_production_modifier,
        state_religion_free_experience,
        exp_in_border_modifier,
        military_food_production,
        no_unhealthy_population,
        building_only_healthy,
        no_foreign_trade,
        no_corporations,
        no_foreign_corporations,
        state_religion,
        no_non_state_religion_spread,
        // The text of the celebration message.
        we_love_the_king: _,
        hidden,
        no_diplomacy_with_enemies,
        prereq_war,
        // Read only by the C++ AI's attitude (`AI_getCivicShareAttitude`: "You are a fellow
        // council member"); ffh-ai's relation score does not use it.
        attitude_share_mod: _,
        coastal_trade_routes,
        enslavement_chance,
        food_consumption_per_population,
        block_alignment,
        prereq_alignment,
        prereq_civilization,
        prereq_religion,
        yield_modifier,
        capital_yield_modifier,
        trade_yield_modifier,
        commerce_modifier,
        capital_commerce_modifier,
        specialist_extra_commerce,
        building_happiness_changes,
        building_health_changes,
        feature_happiness_changes,
        hurry,
        special_building_not_required,
        specialist_valid,
        improvement_yield_changes,
    } = &d.civics[c];
    let mut b = String::new();
    let line = |b: &mut String, s: String| {
        b.push_str(NEWLINE);
        b.push_str(&s);
    };
    let happy = |n: i32| if n > 0 { "[ICON_HAPPY]" } else { "[ICON_UNHAPPY]" };
    let healthy = |n: i32| if n > 0 { "[ICON_HEALTHY]" } else { "[ICON_UNHEALTHY]" };
    // The requirements, uncoloured (`super::unwarned`): C++ colours them only when the player
    // does not meet them; the civics screen says which a player does not meet.
    let requires = |name: &str| req(d, "TXT_KEY_CIVIC_REQUIRES", &[Arg::S(name)]);

    if let Some(t) = tech_prereq.filter(|_| requirements) {
        line(&mut b, requires(&display(d, t)));
    }
    for sb in d.special_buildings.keys().filter(|&sb| special_building_not_required[sb]) {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_BUILD_MISSIONARIES", &[Arg::S(&display(d, sb))]));
    }
    let mut first = true;
    for s in d.specialists.keys().filter(|&s| specialist_valid[s]) {
        let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_CIVIC_UNLIMTED", &[]));
        list_help(&mut b, &start, &display(d, s), ", ", first);
        first = false;
    }
    if *great_people_rate_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_GREAT_PEOPLE_MOD", &[Arg::N(*great_people_rate_modifier)]));
    }
    if *great_general_rate_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_GREAT_GENERAL_MOD", &[Arg::N(*great_general_rate_modifier)]));
    }
    if *domestic_great_general_rate_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_DOMESTIC_GREAT_GENERAL_MODIFIER", &[Arg::N(*domestic_great_general_rate_modifier)]));
    }
    if *state_religion_great_people_rate_modifier != 0 {
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_GREAT_PEOPLE_MOD_STATE_RELIGION",
                &[Arg::N(*state_religion_great_people_rate_modifier), Arg::S("[ICON_RELIGION]")],
            ),
        );
    }
    for (n, none_key, mod_key) in [
        (
            *distance_maintenance_modifier,
            "TXT_KEY_CIVIC_DISTANCE_MAINT",
            "TXT_KEY_CIVIC_DISTANCE_MAINT_MOD",
        ),
        (
            *num_cities_maintenance_modifier,
            "TXT_KEY_CIVIC_NO_MAINT_NUM_CITIES",
            "TXT_KEY_CIVIC_NO_MAINT_NUM_CITIES_MOD",
        ),
        (
            *corporation_maintenance_modifier,
            "TXT_KEY_CIVIC_NO_MAINT_CORPORATION",
            "TXT_KEY_CIVIC_NO_MAINT_CORPORATION_MOD",
        ),
    ] {
        if n != 0 {
            line(&mut b, if n <= -100 { txt(d, none_key, &[]) } else { txt(d, mod_key, &[Arg::N(n)]) });
        }
    }
    if *extra_health != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_EXTRA_HEALTH", &[Arg::N(extra_health.abs()), Arg::S(healthy(*extra_health))]));
    }
    if *free_experience != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_FREE_XP", &[Arg::N(*free_experience)]));
    }
    if *worker_speed_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_WORKER_SPEED", &[Arg::N(*worker_speed_modifier)]));
    }
    if *improvement_upgrade_rate_modifier != 0 {
        let mut first = true;
        for imp in d.improvements.keys().filter(|&i| d.improvements[i].improvement_upgrade.is_some()) {
            let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_CIVIC_IMPROVEMENT_UPGRADE", &[Arg::N(*improvement_upgrade_rate_modifier)]));
            list_help(&mut b, &start, &display(d, imp), ", ", first);
            first = false;
        }
    }
    if *military_production_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_MILITARY_PRODUCTION", &[Arg::N(*military_production_modifier)]));
    }
    // Without a player C++ says the support is lower, not how many units it pays for.
    if *base_free_units != 0 || *free_units_population_percent != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_UNIT_SUPPORT", &[]));
    }
    if *base_free_military_units != 0 || *free_military_units_population_percent != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_MILITARY_UNIT_SUPPORT", &[]));
    }
    if *happy_per_military_unit != 0 {
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_UNIT_HAPPINESS",
                &[Arg::N(*happy_per_military_unit), Arg::S(happy(*happy_per_military_unit))],
            ),
        );
    }
    if *military_food_production {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_MILITARY_FOOD", &[]));
    }
    // C++ `getWorldSizeMaxConscript`: the civic's number scaled by the world size's modifier.
    if *max_conscript != 0 {
        let (lo, hi) = world_size_range(d, |w| max_conscript * (w.max_conscript_modifier + 100).max(0) / 100);
        let mut range = String::new();
        let (n, note) = range_arg(lo, hi, &mut range);
        line(&mut b, txt(d, "TXT_KEY_CIVIC_CONSCRIPTION", &[n]) + note);
    }
    if *no_unhealthy_population {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_NO_POP_UNHEALTHY", &[]));
    }
    if *building_only_healthy {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_NO_BUILDING_UNHEALTHY", &[]));
    }
    if *exp_in_border_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_EXPERIENCE_IN_BORDERS", &[Arg::N(*exp_in_border_modifier)]));
    }
    if *war_weariness_modifier != 0 {
        line(
            &mut b,
            if *war_weariness_modifier <= -100 {
                txt(d, "TXT_KEY_CIVIC_NO_WAR_WEARINESS", &[])
            } else {
                txt(d, "TXT_KEY_CIVIC_EXTRA_WAR_WEARINESS", &[Arg::N(*war_weariness_modifier)])
            },
        );
    }
    if *free_specialist != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_FREE_SPECIALISTS", &[Arg::N(*free_specialist)]));
    }
    if *trade_routes != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_TRADE_ROUTES", &[Arg::N(*trade_routes)]));
    }
    for (on, key) in [
        (*no_foreign_trade, "TXT_KEY_CIVIC_NO_FOREIGN_TRADE"),
        (*no_corporations, "TXT_KEY_CIVIC_NO_CORPORATIONS"),
        (*no_foreign_corporations, "TXT_KEY_CIVIC_NO_FOREIGN_CORPORATIONS"),
    ] {
        if on {
            line(&mut b, txt(d, key, &[]));
        }
    }
    if *civic_percent_anger != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_FREEDOM_ANGER", &[Arg::S(&display(d, c))]));
    }
    // A civic without a state religion where another of its option has one.
    if !*state_religion && d.civics.iter().any(|o| o.civic_option_type == *civic_option_type && o.state_religion) {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_NO_STATE_RELIGION", &[]));
    }
    if *state_religion_happiness != 0 {
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_RELIGION_HAPPINESS",
                &[Arg::N(state_religion_happiness.abs()), Arg::S(happy(*state_religion_happiness))],
            ),
        );
    }
    if *non_state_religion_happiness != 0 {
        // C++ slip, fixed (no FFH2 civic sets it): C++ writes "per religion" for a civic with a
        // state religion and "per non-state religion" for one without, the other way round from
        // what the rules do (`getReligionHappiness`: every religion but the state one), and gives
        // the first text no arguments.
        let args = [Arg::N(non_state_religion_happiness.abs()), Arg::S(happy(*non_state_religion_happiness))];
        line(
            &mut b,
            if *state_religion {
                txt(d, "TXT_KEY_CIVIC_NON_STATE_REL_HAPPINESS_WITH_STATE", &args)
            } else {
                txt(d, "TXT_KEY_CIVIC_NON_STATE_REL_HAPPINESS_NO_STATE", &args)
            },
        );
    }
    if *state_religion_unit_production_modifier != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_STATE_REL_TRAIN_BONUS", &[Arg::N(*state_religion_unit_production_modifier)]));
    }
    if *state_religion_building_production_modifier != 0 {
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_STATE_REL_BUILDING_BONUS",
                &[Arg::N(*state_religion_building_production_modifier)],
            ),
        );
    }
    if *state_religion_free_experience != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_STATE_REL_FREE_XP", &[Arg::N(*state_religion_free_experience)]));
    }
    if *no_non_state_religion_spread {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_NO_NON_STATE_SPREAD", &[]));
    }
    let all_cities = txt(d, "TXT_KEY_CIVIC_IN_ALL_CITIES", &[]);
    let capital = txt(d, "TXT_KEY_CIVIC_IN_CAPITAL", &[]);
    let from_trade = txt(d, "TXT_KEY_CIVIC_FROM_TRADE_ROUTES", &[]);
    let per_specialist = txt(d, "TXT_KEY_CIVIC_PER_SPECIALIST", &[]);
    changes_help(&mut b, "", "", &all_cities, yield_modifier, &YIELD_ICONS, true, true);
    changes_help(&mut b, "", "", &capital, capital_yield_modifier, &YIELD_ICONS, true, true);
    changes_help(&mut b, "", "", &from_trade, trade_yield_modifier, &YIELD_ICONS, true, true);
    changes_help(&mut b, "", "", &all_cities, commerce_modifier, &COMMERCE_ICONS, true, true);
    changes_help(&mut b, "", "", &capital, capital_commerce_modifier, &COMMERCE_ICONS, true, true);
    changes_help(&mut b, "", "", &per_specialist, specialist_extra_commerce, &COMMERCE_ICONS, false, true);
    if *largest_city_happiness != 0 {
        let (lo, hi) = world_size_range(d, |w| w.target_num_cities);
        let mut range = String::new();
        let (n, note) = range_arg(lo, hi, &mut range);
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_LARGEST_CITIES_HAPPINESS",
                &[Arg::N(*largest_city_happiness), Arg::S(happy(*largest_city_happiness)), n],
            ) + note,
        );
    }
    // The improvements' yields, one list per yield and change.
    for y in 0..YieldType::NUM {
        let mut last = 0;
        for imp in d.improvements.keys() {
            let n = improvement_yield_changes[imp][y];
            if n != 0 {
                let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_CIVIC_IMPROVEMENT_YIELD_CHANGE", &[Arg::N(n), Arg::S(YIELD_ICONS[y])]));
                list_help(&mut b, &start, &display(d, imp), ", ", n != last);
                last = n;
            }
        }
    }
    // Without a player C++ names the building class (the same key for health).
    for class in d.building_classes.keys() {
        let n = building_happiness_changes[class];
        if n != 0 {
            line(&mut b, txt(d, "TXT_KEY_CIVIC_BUILDING_HAPPINESS", &[Arg::N(n), Arg::S(happy(n)), Arg::S(&display(d, class))]));
        }
        let n = building_health_changes[class];
        if n != 0 {
            line(&mut b, txt(d, "TXT_KEY_CIVIC_BUILDING_HAPPINESS", &[Arg::N(n), Arg::S(healthy(n)), Arg::S(&display(d, class))]));
        }
    }
    let mut last = 0;
    for f in d.features.keys() {
        let n = feature_happiness_changes[f];
        if n != 0 {
            let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_CIVIC_FEATURE_HAPPINESS", &[Arg::N(n), Arg::S(happy(n))]));
            list_help(&mut b, &start, &display(d, f), ", ", n != last);
            last = n;
        }
    }
    for h in d.hurries.keys().filter(|&h| hurry[h]) {
        line(&mut b, format!("[ICON_BULLET]{}", display(d, h)));
    }
    // FfH. Without a player, every requirement is written.
    if *prereq_war && requirements {
        line(&mut b, req(d, "TXT_KEY_CIVIC_PREREQ_WAR", &[]));
    }
    if let Some(a) = block_alignment.filter(|_| requirements) {
        line(&mut b, req(d, "TXT_KEY_CIVIC_BLOCKS", &[Arg::S(&display(d, a))]));
    }
    if let Some(a) = prereq_alignment.filter(|_| requirements) {
        line(&mut b, requires(&display(d, a)));
    }
    if let Some(civ) = prereq_civilization.filter(|_| requirements) {
        line(&mut b, requires(&display(d, civ)));
    }
    if let Some(r) = prereq_religion.filter(|_| requirements) {
        line(&mut b, requires(&display(d, r)));
    }
    if *no_diplomacy_with_enemies {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_NO_DIPLOMACY_WITH_ENEMIES", &[]));
    }
    if *coastal_trade_routes != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_COASTAL_TRADE_ROUTES", &[Arg::N(*coastal_trade_routes)]));
    }
    if *enslavement_chance != 0 {
        line(&mut b, txt(d, "TXT_KEY_UNIT_ENSLAVEMENT_CHANCE", &[Arg::N(*enslavement_chance)]));
    }
    if *food_consumption_per_population != 0 {
        line(
            &mut b,
            txt(
                d,
                "TXT_KEY_CIVIC_FOOD_CONSUMPTION_PER_POPULATION",
                &[Arg::N(*food_consumption_per_population), Arg::N(d.defines.food_consumption_per_population)],
            ),
        );
    }
    // C++ slip, fixed (no FFH2 civic sets them): C++ passes `getGoldPerUnit() > 0`, so the line
    // read +1 (or +0) whatever the cost; here the cost.
    if *gold_per_unit != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_SUPPORT_COSTS", &[Arg::N(*gold_per_unit), Arg::S(COMMERCE_ICONS[0])]));
    }
    if *gold_per_military_unit != 0 {
        line(&mut b, txt(d, "TXT_KEY_CIVIC_MILITARY_SUPPORT_COSTS", &[Arg::N(*gold_per_military_unit), Arg::S(COMMERCE_ICONS[0])]));
    }
    // ffh-play's (FfH2's help says nothing of these): a hidden civic (C++ `canSeeCivic`: only a player
    // who follows it sees it), the units that need it (`CvPlayer::canTrain`, `getPrereqCivic`) and the
    // councils whose votes it gives (`CvPlayer::getVotes`, the vote source's `getCivic`).
    if *hidden {
        line(&mut b, "[ICON_BULLET]Hidden: only those who follow it see that you do".to_string());
    }
    let mut first = true;
    for u in d.units.keys().filter(|&u| d.units[u].prereq_civic == Some(c)) {
        let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_TECH_CAN_TRAIN", &[]));
        list_help(&mut b, &start, &color("COLOR_UNIT_TEXT", &display(d, u)), ", ", first);
        first = false;
    }
    for v in d.vote_sources.keys().filter(|&v| d.vote_sources[v].civic == Some(c)) {
        line(&mut b, format!("[ICON_BULLET]Votes in the {}", color("COLOR_HIGHLIGHT_TEXT", &display(d, v))));
    }
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

    fn civic(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<CivicType>(name))
    }

    #[test]
    fn known_civics_say_what_they_do() {
        let god_king = civic("CIVIC_GOD_KING");
        assert!(god_king.starts_with("Reqs [LINK=literal]Mysticism[\\LINK][NEWLINE]"), "{god_king}");
        assert!(god_king.contains("[ICON_BULLET]+10% Maintenance Costs from Distance to Palace"), "{god_king}");
        assert!(god_king.contains("+50%[ICON_PRODUCTION][SPACE]in Capital"), "{god_king}");

        let republic = civic("CIVIC_REPUBLIC");
        assert!(republic.contains("[ICON_BULLET]+25%[ICON_GREATPEOPLE] Birth Rate"), "{republic}");
        assert!(republic.contains("[ICON_BULLET]+3[ICON_HAPPY] in 4-6 Largest Cities (by world size)"), "{republic}");
        assert!(republic.contains("Penalty for Civs without [LINK=literal]Republic[\\LINK]"), "{republic}");
        assert!(republic.contains("[SPACE]in All Cities"), "{republic}");

        // Farms: +1 food, -1 production; +1 health.
        let agrarianism = civic("CIVIC_AGRARIANISM");
        assert!(agrarianism.contains("[ICON_BULLET]+1[ICON_HEALTHY] in All Cities"), "{agrarianism}");
        assert!(agrarianism.contains("[ICON_BULLET]+1[ICON_FOOD] from[SPACE]Farm"), "{agrarianism}");
        assert!(agrarianism.contains("[ICON_BULLET]-1[ICON_PRODUCTION] from[SPACE]Farm"), "{agrarianism}");

        let overcouncil = civic("CIVIC_OVERCOUNCIL");
        assert!(overcouncil.contains("Unavailable to Evil Leaders"), "{overcouncil}");
        assert!(overcouncil.contains("[ICON_BULLET]Votes in the [COLOR_HIGHLIGHT_TEXT]Overcouncil[COLOR_REVERT]"), "{overcouncil}");
        assert!(civic("CIVIC_UNDERCOUNCIL").contains("Hidden"));

        let crusade = civic("CIVIC_CRUSADE");
        for part in ["Must be at War", "Reqs [LINK=literal]Bannor", "No diplomacy with enemies", "Lower Unit Support Costs", "Train Units +25% Faster", "Can Train[SPACE][COLOR_UNIT_TEXT]"] {
            assert!(crusade.contains(part), "{part}: {crusade}");
        }
        // The civics screen's: the same without the requirements (the screen says those unmet).
        let d = ffh_data::global();
        let screen = effects_without_requirements(d, d.t::<CivicType>("CIVIC_CRUSADE"));
        assert!(!screen.contains("Reqs") && !screen.contains("Must be at War"), "{screen}");
        assert!(screen.starts_with("[ICON_BULLET]Lower Unit Support Costs") && screen.contains("No diplomacy with enemies"), "{screen}");
        assert!(!effects_without_requirements(d, d.t::<CivicType>("CIVIC_OVERCOUNCIL")).contains("Unavailable"));
        // Military State drafts 3 units on a standard map: 1 to 5 over the world sizes.
        assert!(civic("CIVIC_MILITARY_STATE").contains("Draft[\\LINK] 1-5 Units per Turn (by world size)"));
        let slavery = civic("CIVIC_SLAVERY");
        assert!(slavery.contains("Sacrifice Population") && slavery.contains("+25% chance to create a slave"), "{slavery}");
        assert!(civic("CIVIC_ARETE").contains("Reqs [LINK=literal]Runes of Kilmorph"));
        assert!(civic("CIVIC_THEOCRACY").contains("[ICON_BULLET]Unlimited[SPACE]Priest"));
        assert!(civic("CIVIC_SACRIFICE_THE_WEAK").contains("+1 food consumed per population (instead of +2)"));
    }

    #[test]
    fn every_civic_reads_well() {
        let d = ffh_data::global();
        for c in d.civics.keys() {
            let t = effects(d, c);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{}: {t}", names::name(d, c));
            assert!(!t.starts_with(NEWLINE) && !t.contains("[NEWLINE][NEWLINE]"), "{t}");
            // Something to say for every civic with a gameplay field set (beyond those not shown).
            let mut x = d.civics[c].clone();
            x.base = InfoBase::default();
            x.civic_option_type = None;
            x.anarchy_length = 0;
            x.upkeep = None;
            x.ai_weight = 0;
            x.we_love_the_king.clear();
            x.attitude_share_mod = 0;
            if sets_anything(&format!("{x:?}")) {
                assert!(!t.is_empty(), "{}", names::name(d, c));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for c in d.civics.keys() {
            println!("{}:\n  {}\n", names::name(d, c), effects(d, c).replace(NEWLINE, "\n  "));
        }
    }
}
