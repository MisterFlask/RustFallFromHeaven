//! What a building does and what it requires: C++ `CvGameTextMgr::setBuildingHelp` and
//! `buildBuildingRequiresString`, as static help (no city, no player).
//!
//! The lines are the tooltip's (`bCivilopediaText` false, `pCity` NULL, no active player) without
//! its name and cost lines, which the page shows itself: the numbers of the name line (happiness,
//! health, yields, commerces, their modifiers, great people) go on a line of their own, and the
//! resources that speed it up on lines of their own (the Civilopedia's form; the tooltip appends
//! them to the cost). The Civilopedia's own lines (`bCivilopediaText`) that need no player are
//! added where C++ writes them: the traits that build it faster, the great person it makes likelier,
//! the free start era, and the victory, start era and teams it requires. Not its "Replaced by"
//! list: each unique building says what it replaces, and the Palace's would name every
//! civilization's.
//!
//! Where C++ looks a class's unit or building up in the active player's civilization (the units
//! it is required for, the free building, the great person), it takes the classes' defaults with
//! no player; a unique building takes its own civilization's instead, the only one that can have
//! it.
//!
//! The requirements come without the warning colour C++ gives them (its tooltip lists only those
//! the city or player does not meet; here nothing is known to be unmet), names in the highlight
//! colour. FFH2's help leaves out some requirements its rules check (`CvCity::canConstruct`,
//! `CvPlayer::canConstruct`, `CvCity::isValidBuildingLocation`): the religion in the city, the
//! state religion, the coast, the river and the land size, and the civilizations of the
//! civilization-only buildings. ffh-play's lines add them ("ffh-play's" below).

use ffh_data::*;

use super::{changes_help, color, finish, list_help, req, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};
use crate::names::{self, display};

/// A name in the highlight colour: requirements, and religions and corporations where C++ writes
/// their icon (the page has no markup for those icons).
fn highlight(name: &str) -> String {
    color("COLOR_HIGHLIGHT_TEXT", name)
}

/// C++ writes a resource's icon (`getChar()`) in brackets after its name ("Provides 1 Mithril
/// (icon)"); the page has no markup for it, so it is filled in empty and the brackets go.
fn without_icon(t: String) -> String {
    t.replace("[SPACE]()", "").replace(" ()", "")
}

/// C++ `CvGame::getNumFreeBonuses`: the building's number, or with -1 the world size's
/// (`getNumFreeBuildingBonuses`), unknown before the game: then the range over the world sizes.
fn free_bonus_count(d: &GameData, n: i32) -> String {
    if n != -1 {
        return n.to_string();
    }
    let sizes: Vec<i32> = d.worlds.keys().map(|w| d.worlds[w].num_free_building_bonuses).collect();
    let (lo, hi) = (sizes.iter().copied().min().unwrap_or(0), sizes.iter().copied().max().unwrap_or(0));
    if lo == hi {
        lo.to_string()
    } else {
        format!("{lo}-{hi}")
    }
}

fn happy_icon(n: i32) -> &'static str {
    if n > 0 {
        "[ICON_HAPPY]"
    } else {
        "[ICON_UNHAPPY]"
    }
}

fn health_icon(n: i32) -> &'static str {
    if n > 0 {
        "[ICON_HEALTHY]"
    } else {
        "[ICON_UNHEALTHY]"
    }
}

/// What building `b` does and requires (Civ4 markup, lines joined by `[NEWLINE]`; empty when there
/// is nothing to say). Every field of the building is either described here or named as not
/// shown, so that a new field cannot go unnoticed (the build fails until it is listed).
pub fn effects(d: &GameData, b: BuildingType) -> String {
    let BuildingInfo {
        // The XML help (the name, pedia, strategy and button are the page's).
        base,
        building_class_type,
        victory_prereq,
        free_start_era,
        max_start_era,
        obsolete_tech,
        prereq_and_tech,
        no_bonus,
        power_bonus,
        free_bonus,
        num_free_bonuses,
        free_building_class,
        free_promotion,
        civic_option,
        special_building_type,
        holy_city,
        religion_type,
        state_religion,
        prereq_religion,
        // Not described by C++ (FFH2 has no corporations).
        prereq_corporation: _,
        founds_corporation,
        global_religion_commerce,
        global_corporation_commerce,
        prereq_and_bonus,
        great_people_unit_class,
        vote_source_type,
        // The AI's.
        ai_weight: _,
        // The cost line, which the page shows itself (the data's `cost`).
        production_cost: _,
        // Not described by C++: the extra cost of hurrying it.
        hurry_cost_modifier: _,
        hurry_anger_modifier,
        // Advanced starts only.
        advanced_start_cost: _,
        advanced_start_cost_increase: _,
        min_area_size,
        num_cities_prereq,
        num_teams_prereq,
        unit_level_prereq,
        min_latitude,
        max_latitude,
        great_people_rate_modifier,
        great_general_rate_modifier,
        domestic_great_general_rate_modifier,
        global_great_people_rate_modifier,
        anarchy_modifier,
        golden_age_modifier,
        global_hurry_modifier,
        free_experience,
        global_free_experience,
        food_kept,
        airlift,
        air_modifier,
        air_unit_capacity,
        nuke_modifier,
        nuke_explosion_rand,
        free_specialist,
        area_free_specialist,
        global_free_specialist,
        happiness,
        area_happiness,
        global_happiness,
        state_religion_happiness,
        worker_speed_modifier,
        military_production_modifier,
        space_production_modifier,
        global_space_production_modifier,
        trade_routes,
        coastal_trade_routes,
        global_trade_routes,
        trade_route_modifier,
        foreign_trade_route_modifier,
        // The score's and the AI's measures of wealth and military power.
        asset_value: _,
        power_value: _,
        great_people_rate_change,
        // Not described by C++: the chance it survives the city's capture.
        conquest_probability: _,
        maintenance_modifier,
        war_weariness_modifier,
        global_war_weariness_modifier,
        enemy_war_weariness_modifier,
        heal_rate_change,
        health,
        area_health,
        global_health,
        global_population_change,
        free_techs,
        defense_modifier,
        bombard_defense_modifier,
        all_city_defense_modifier,
        espionage_defense_modifier,
        // Not described by C++: whether its effects reach the team.
        team_share: _,
        water,
        river,
        power,
        dirty_power,
        area_clean_power,
        area_border_obstacle,
        force_team_vote_eligible,
        capital,
        government_center,
        golden_age,
        map_centering,
        no_unhappiness,
        no_unhealthy_population,
        building_only_healthy,
        // Not described by C++: destroyed when the city is captured; safe from nukes.
        never_capture: _,
        nuke_immune: _,
        is_prereq_religion,
        // Art: drawn in the city's centre.
        center_in_city: _,
        is_state_religion,
        allows_nukes,
        // ---- FfH ----
        apply_free_promotion_on_move,
        // Not described by C++: razing the city drops the equipment unit that holds it.
        equipment: _,
        hide_units,
        no_civic_anger,
        requires_caster,
        see_invisible,
        unhappy_production,
        crime,
        free_promotion_pick,
        free_bonus2,
        free_bonus3,
        global_resist_enemy_modify,
        global_resist_modify,
        modify_global_counter,
        plot_radius,
        // Never set (C++ quirk, see the field) and the XML value is never applied: the
        // civilizations come from their unique buildings (the "Unique Building for" lines).
        prereq_civ: _,
        prereq_civ_xml: _,
        prereq_trait,
        remove_promotion,
        resist_magic,
        specialist_commerce_change,
        // The AI's (Better AI).
        victory_building: _,
        prereq_and_techs,
        prereq_or_bonuses,
        production_traits,
        happiness_traits,
        sea_plot_yield_change,
        river_plot_yield_change,
        global_sea_plot_yield_change,
        yield_change,
        yield_modifier,
        power_yield_modifier,
        area_yield_modifier,
        global_yield_modifier,
        commerce_change,
        obsolete_safe_commerce_change,
        // Not described by C++: the turns after which its commerce doubles.
        commerce_change_double_time: _,
        commerce_modifier,
        global_commerce_modifier,
        specialist_extra_commerce,
        state_religion_commerce,
        commerce_happiness,
        religion_change,
        specialist_count,
        free_specialist_count,
        bonus_health_changes,
        bonus_happiness_changes,
        bonus_production_modifier,
        unit_combat_free_experience,
        domain_free_experience,
        domain_production_modifier,
        building_happiness_changes,
        prereq_num_of_building_class,
        // The AI's.
        flavor_value: _,
        improvement_free_specialist,
        commerce_flexible,
        // Not described by C++: whether the commerce stays with the builder.
        commerce_change_original_owner: _,
        building_class_needed_in_city,
        specialist_yield_change,
        bonus_yield_modifier,
    } = &d.buildings[b];
    let mut h = String::new();
    let add = |h: &mut String, key: &str, args: &[Arg]| {
        h.push_str(NEWLINE);
        h.push_str(&txt(d, key, args));
    };
    let add_req = |h: &mut String, key: &str, args: &[Arg]| {
        h.push_str(NEWLINE);
        h.push_str(&req(d, key, args));
    };
    let class = *building_class_type;
    let default = class.and_then(|c| d.building_classes[c].default_building_index);
    // The civilizations that have it for its class.
    let civs: Vec<CivilizationType> = match class {
        Some(c) => d.civilizations.keys().filter(|&civ| d.civilizations[civ].civilization_buildings(c, &d.building_classes) == Some(b)).collect(),
        None => Vec::new(),
    };
    // C++ takes units and buildings of a class from the active player's civilization, with none
    // the classes' defaults. A unique building's civilization is the only player who can have it,
    // so its units and buildings are taken (C++ slip fixed: the Luchuirp's Sculptor's Studio was
    // "Required to Train" the default Axeman, which the Luchuirp train as a Wood Golem).
    let owner = match civs[..] {
        [civ] if default != Some(b) => Some(civ),
        _ => None,
    };
    let civ_unit = |c: UnitClassType| {
        let default = d.unit_classes[c].default_unit_index;
        owner.map_or(default, |civ| d.civilizations[civ].civilization_units(c, default))
    };
    let civ_building = |c: BuildingClassType| owner.map_or(d.building_classes[c].default_building_index, |civ| d.civilizations[civ].civilization_buildings(c, &d.building_classes));

    // The tooltip's name line ("Library, +25%[ICON_RESEARCH]") without the name.
    let mut stats = String::new();
    if *happiness != 0 {
        stats.push_str(&format!(", +{}{}", happiness.abs(), happy_icon(*happiness)));
    }
    if *health != 0 {
        stats.push_str(&format!(", +{}{}", health.abs(), health_icon(*health)));
    }
    changes_help(&mut stats, ", ", "", "", yield_change, &YIELD_ICONS, false, false);
    let commerces: Vec<i32> = (0..CommerceType::NUM).map(|k| commerce_change[k] + obsolete_safe_commerce_change[k]).collect();
    changes_help(&mut stats, ", ", "", "", &commerces, &COMMERCE_ICONS, false, false);
    changes_help(&mut stats, ", ", "", "", yield_modifier, &YIELD_ICONS, true, false);
    changes_help(&mut stats, ", ", "", "", commerce_modifier, &COMMERCE_ICONS, true, false);
    let great_person = great_people_unit_class.and_then(civ_unit);
    if *great_people_rate_change != 0 {
        let sign = if *great_people_rate_change > 0 { "+" } else { "" };
        stats.push_str(&format!(", {sign}{great_people_rate_change}[ICON_GREATPEOPLE]"));
        if let Some(u) = great_person {
            stats.push_str(&format!(" ({})", display(d, u)));
        }
    }
    if let Some(s) = stats.strip_prefix(", ") {
        h.push_str(NEWLINE);
        h.push_str("[ICON_BULLET]");
        h.push_str(s);
    }

    // A unique building: the civilizations that have it in place of the class's default.
    if let Some(c) = class {
        let unique = |h: &mut String| {
            for &civ in &civs {
                add(h, "TXT_KEY_UNIQUE_BUILDING", &[Arg::S(&display(d, civ))]);
            }
        };
        match default {
            Some(def) if def != b => {
                unique(&mut h);
                add(&mut h, "TXT_KEY_REPLACES_UNIT", &[Arg::S(&display(d, def))]);
            }
            // ffh-play's: FFH2's civilization-only buildings share classes without a default
            // (`BUILDINGCLASS_CIV_BUILDING1`, ...), which C++ passes over.
            None => unique(&mut h),
            Some(_) => {}
        }
        let ci = &d.building_classes[c];
        if ci.max_global_instances != -1 {
            add(&mut h, "TXT_KEY_BUILDING_WORLD_WONDER_ALLOWED", &[Arg::N(ci.max_global_instances)]);
        }
        if ci.max_team_instances != -1 {
            add(&mut h, "TXT_KEY_BUILDING_TEAM_WONDER_ALLOWED", &[Arg::N(ci.max_team_instances)]);
        }
        if ci.max_player_instances != -1 {
            add(&mut h, "TXT_KEY_BUILDING_NATIONAL_WONDER_ALLOWED", &[Arg::N(ci.max_player_instances)]);
        }
    }

    if let Some(r) = *global_religion_commerce {
        let end = txt(d, "TXT_KEY_BUILDING_PER_CITY_WITH", &[Arg::S(&highlight(&display(d, r)))]);
        changes_help(&mut h, "", "", &end, &d.religions[r].global_religion_commerce, &COMMERCE_ICONS, false, true);
    }
    if let Some(c) = *founds_corporation {
        add(&mut h, "TXT_KEY_FOUNDS_CORPORATION", &[Arg::S(&display(d, c))]);
    }
    if let Some(c) = *global_corporation_commerce {
        let end = txt(d, "TXT_KEY_BUILDING_PER_CITY_WITH", &[Arg::S(&highlight(&display(d, c)))]);
        changes_help(&mut h, "", "", &end, &d.corporations[c].headquarter_commerce, &COMMERCE_ICONS, false, true);
    }
    if let Some(x) = *no_bonus {
        h.push_str(NEWLINE);
        h.push_str(&without_icon(txt(
            d,
            "TXT_KEY_BUILDING_DISABLES",
            &[Arg::S(&display(d, x)), Arg::S("")],
        )));
    }
    // The free resources (FfH: up to three), each with the health and happiness it brings.
    let count = free_bonus_count(d, *num_free_bonuses);
    for x in [*free_bonus, *free_bonus2, *free_bonus3].into_iter().flatten() {
        h.push_str(NEWLINE);
        h.push_str(&without_icon(txt(
            d,
            "TXT_KEY_BUILDING_PROVIDES",
            &[Arg::S(&count), Arg::S(&display(d, x)), Arg::S("")],
        )));
        let bonus = &d.bonuses[x];
        if bonus.health != 0 {
            h.push_str(&format!(", +{}{}", bonus.health.abs(), health_icon(bonus.health)));
        }
        if bonus.happiness != 0 {
            h.push_str(&format!(", +{}{}", bonus.happiness.abs(), happy_icon(bonus.happiness)));
        }
    }

    // FfH.
    if *crime != 0 {
        add(&mut h, "TXT_KEY_BUILDING_CRIME", &[Arg::N(*crime)]);
    }
    if *free_promotion_pick > 0 {
        add(&mut h, "TXT_KEY_BUILDING_FREE_PROMOTION_PICK", &[Arg::N(*free_promotion_pick)]);
    }
    if *modify_global_counter != 0 {
        add(&mut h, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER", &[Arg::N(*modify_global_counter)]);
    }
    if let Some(p) = *remove_promotion {
        add(&mut h, "TXT_KEY_BUILDING_REMOVE_PROMOTION", &[Arg::S(&display(d, p))]);
    }
    if *resist_magic != 0 {
        add(&mut h, "TXT_KEY_BUILDING_RESIST_MAGIC", &[Arg::N(*resist_magic)]);
    }
    if *global_resist_enemy_modify != 0 {
        add(&mut h, "TXT_KEY_BUILDING_GLOBAL_RESIST_ENEMY_MODIFY", &[Arg::N(*global_resist_enemy_modify)]);
    }
    if *global_resist_modify != 0 {
        add(&mut h, "TXT_KEY_BUILDING_GLOBAL_RESIST_MODIFY", &[Arg::N(*global_resist_modify)]);
    }
    for (has, key) in [
        (*hide_units, "TXT_KEY_BUILDING_HIDE_UNITS"),
        (*unhappy_production, "TXT_KEY_BUILDING_UNHAPPY_PRODUCTION"),
        (*requires_caster, "TXT_KEY_BUILDING_REQUIRES_CASTER"),
        (*see_invisible, "TXT_KEY_BUILDING_SEE_INVISIBLE"),
    ] {
        if has {
            add(&mut h, key, &[]);
        }
    }
    let from_specialist =
        |s: SpecialistType| txt(d, "TXT_KEY_BUILDING_FROM_IN_ALL_CITIES", &[Arg::S(&display(d, s))]);
    for s in d.specialists.keys() {
        changes_help(&mut h, "", "", &from_specialist(s), &specialist_commerce_change[s], &COMMERCE_ICONS, false, true);
    }
    if *no_civic_anger {
        add(&mut h, "TXT_KEY_BUILDING_NO_CIVIC_ANGER", &[]);
    }
    if *plot_radius != 0 {
        // The text signs the radius ("to +3"): passed as text, it reads "to 3".
        add(&mut h, "TXT_KEY_BUILDING_PLOT_RADIUS", &[Arg::S(&plot_radius.to_string())]);
    }

    if let Some(fb) = free_building_class.and_then(civ_building) {
        add(&mut h, "TXT_KEY_BUILDING_FREE_IN_CITY", &[Arg::S(&display(d, fb))]);
    }
    if let Some(p) = *free_promotion {
        add(
            &mut h,
            "TXT_KEY_BUILDING_FREE_PROMOTION",
            &[Arg::S(&display(d, p))],
        );
        if *apply_free_promotion_on_move {
            h.push_str(&txt(d, "TXT_KEY_BUILDING_APPLY_FREE_PROMOTION_ON_MOVE", &[]));
        }
    }
    if let Some(o) = *civic_option {
        add(&mut h, "TXT_KEY_BUILDING_ENABLES_CIVICS", &[Arg::S(&display(d, o))]);
    }
    let dirty = d.defines.dirty_power_health_change;
    let dirty_health = format!(" (+{}{})", dirty.abs(), health_icon(dirty));
    if *power {
        add(&mut h, "TXT_KEY_BUILDING_PROVIDES_POWER", &[]);
        if *dirty_power && dirty != 0 {
            h.push_str(&dirty_health);
        }
    }
    if *area_clean_power {
        add(&mut h, "TXT_KEY_BUILDING_PROVIDES_AREA_CLEAN_POWER", &[]);
    }
    if *area_border_obstacle {
        add(&mut h, "TXT_KEY_BUILDING_BORDER_OBSTACLE", &[]);
    }
    if let Some(v) = *vote_source_type {
        add(&mut h, "TXT_KEY_BUILDING_DIPLO_VOTE", &[Arg::S(&display(d, v))]);
    }
    for (has, key) in [
        (*force_team_vote_eligible, "TXT_KEY_BUILDING_ELECTION_ELIGIBILITY"),
        (*capital, "TXT_KEY_BUILDING_CAPITAL"),
        (*government_center, "TXT_KEY_BUILDING_REDUCES_MAINTENANCE"),
        (*golden_age, "TXT_KEY_BUILDING_GOLDEN_AGE"),
        (*allows_nukes, "TXT_KEY_PROJECT_ENABLES_NUKES"),
        (*map_centering, "TXT_KEY_BUILDING_CENTERS_MAP"),
        (*no_unhappiness, "TXT_KEY_BUILDING_NO_UNHAPPY"),
        (*no_unhealthy_population, "TXT_KEY_BUILDING_NO_UNHEALTHY_POP"),
        (*building_only_healthy, "TXT_KEY_BUILDING_NO_UNHEALTHY_BUILDINGS"),
    ] {
        if has {
            add(&mut h, key, &[]);
        }
    }
    for (n, key) in [
        (*great_people_rate_modifier, "TXT_KEY_BUILDING_BIRTH_RATE_MOD"),
        (*great_general_rate_modifier, "TXT_KEY_BUILDING_GENERAL_RATE_MOD"),
        (*domestic_great_general_rate_modifier, "TXT_KEY_DOMESTIC_GREAT_GENERAL_MODIFIER"),
        (*global_great_people_rate_modifier, "TXT_KEY_BUILDING_BIRTH_RATE_MOD_ALL_CITIES"),
    ] {
        if n != 0 {
            add(&mut h, key, &[Arg::N(n)]);
        }
    }
    if *anarchy_modifier == -100 {
        add(&mut h, "TXT_KEY_BUILDING_NO_ANARCHY", &[]);
    } else if *anarchy_modifier != 0 {
        add(&mut h, "TXT_KEY_BUILDING_ANARCHY_MOD", &[Arg::N(*anarchy_modifier)]);
        add(&mut h, "TXT_KEY_BUILDING_ANARCHY_TIMER_MOD", &[Arg::N(*anarchy_modifier)]);
    }
    for (n, key) in [
        (*golden_age_modifier, "TXT_KEY_BUILDING_GOLDENAGE_MOD"),
        (*global_hurry_modifier, "TXT_KEY_BUILDING_HURRY_MOD"),
        (*free_experience, "TXT_KEY_BUILDING_FREE_XP_UNITS"),
        (*global_free_experience, "TXT_KEY_BUILDING_FREE_XP_ALL_CITIES"),
    ] {
        if n != 0 {
            add(&mut h, key, &[Arg::N(n)]);
        }
    }
    if *food_kept > 0 {
        add(&mut h, "TXT_KEY_BUILDING_STORES_FOOD", &[Arg::N(*food_kept)]);
    }
    if *airlift > 0 {
        add(&mut h, "TXT_KEY_BUILDING_AIRLIFT", &[Arg::N(*airlift)]);
    }
    for (n, key) in [
        (*air_modifier, "TXT_KEY_BUILDING_AIR_DAMAGE_MOD"),
        (*air_unit_capacity, "TXT_KEY_BUILDING_AIR_UNIT_CAPACITY"),
        (*nuke_modifier, "TXT_KEY_BUILDING_NUKE_DAMAGE_MOD"),
        (*nuke_explosion_rand, "TXT_KEY_BUILDING_NUKE_EXPLOSION_CHANCE"),
        (*free_specialist, "TXT_KEY_BUILDING_FREE_SPECIALISTS"),
        (*area_free_specialist, "TXT_KEY_BUILDING_FREE_SPECIALISTS_CONT"),
        (*global_free_specialist, "TXT_KEY_BUILDING_FREE_SPECIALISTS_ALL_CITIES"),
        (*maintenance_modifier, "TXT_KEY_BUILDING_MAINT_MOD"),
        (*hurry_anger_modifier, "TXT_KEY_BUILDING_HURRY_ANGER_MOD"),
        (*war_weariness_modifier, "TXT_KEY_BUILDING_WAR_WEAR_MOD"),
        (*global_war_weariness_modifier, "TXT_KEY_BUILDING_WAR_WEAR_MOD_ALL_CITIES"),
        (*enemy_war_weariness_modifier, "TXT_KEY_BUILDING_ENEMY_WAR_WEAR"),
        (*heal_rate_change, "TXT_KEY_BUILDING_HEAL_MOD"),
    ] {
        if n != 0 {
            add(&mut h, key, &[Arg::N(n)]);
        }
    }
    if *area_health != 0 {
        add(&mut h, "TXT_KEY_BUILDING_HEALTH_CHANGE_CONT", &[Arg::N(area_health.abs()), Arg::S(health_icon(*area_health))]);
    }
    if *global_health != 0 {
        add(&mut h, "TXT_KEY_BUILDING_HEALTH_CHANGE_ALL_CITIES", &[Arg::N(global_health.abs()), Arg::S(health_icon(*global_health))]);
    }
    // C++ quirk: only a gain is said (a loss of happiness here or in all cities is not).
    if *area_happiness > 0 {
        add(&mut h, "TXT_KEY_BUILDING_HAPPY_CHANGE_CONT", &[Arg::N(*area_happiness), Arg::S(happy_icon(*area_happiness))]);
    }
    if *global_happiness > 0 {
        add(&mut h, "TXT_KEY_BUILDING_HAPPY_CHANGE_ALL_CITIES", &[Arg::N(*global_happiness), Arg::S(happy_icon(*global_happiness))]);
    }
    if let Some(r) = religion_type.filter(|_| *state_religion_happiness > 0) {
        add(
            &mut h,
            "TXT_KEY_BUILDING_RELIGION_HAPPINESS",
            &[Arg::N(*state_religion_happiness), Arg::S(happy_icon(*state_religion_happiness)), Arg::S(&highlight(&display(d, r)))],
        );
    }
    for (n, key) in [
        (*worker_speed_modifier, "TXT_KEY_BUILDING_WORKER_MOD"),
        (*military_production_modifier, "TXT_KEY_BUILDING_MILITARY_MOD"),
        (*space_production_modifier, "TXT_KEY_BUILDING_SPACESHIP_MOD"),
        (*global_space_production_modifier, "TXT_KEY_BUILDING_SPACESHIP_MOD_ALL_CITIES"),
        (*trade_routes, "TXT_KEY_BUILDING_TRADE_ROUTES"),
        (*coastal_trade_routes, "TXT_KEY_BUILDING_COASTAL_TRADE_ROUTES"),
        (*global_trade_routes, "TXT_KEY_BUILDING_TRADE_ROUTES_ALL_CITIES"),
        (*trade_route_modifier, "TXT_KEY_BUILDING_TRADE_ROUTE_MOD"),
        (*foreign_trade_route_modifier, "TXT_KEY_BUILDING_FOREIGN_TRADE_ROUTE_MOD"),
        (*global_population_change, "TXT_KEY_BUILDING_GLOBAL_POP"),
    ] {
        if n != 0 {
            add(&mut h, key, &[Arg::N(n)]);
        }
    }
    if *free_techs == 1 {
        add(&mut h, "TXT_KEY_BUILDING_FREE_TECH", &[]);
    } else if *free_techs != 0 {
        add(&mut h, "TXT_KEY_BUILDING_FREE_TECHS", &[Arg::N(*free_techs)]);
    }
    if *defense_modifier != 0 {
        add(&mut h, "TXT_KEY_BUILDING_DEFENSE_MOD", &[Arg::N(*defense_modifier)]);
    }
    if *bombard_defense_modifier != 0 {
        add(&mut h, "TXT_KEY_BUILDING_BOMBARD_DEFENSE_MOD", &[Arg::N(-*bombard_defense_modifier)]);
    }
    if *all_city_defense_modifier != 0 {
        add(&mut h, "TXT_KEY_BUILDING_DEFENSE_MOD_ALL_CITIES", &[Arg::N(*all_city_defense_modifier)]);
    }
    if *espionage_defense_modifier != 0 {
        add(&mut h, "TXT_KEY_BUILDING_ESPIONAGE_DEFENSE_MOD", &[Arg::N(*espionage_defense_modifier)]);
        add(&mut h, "TXT_KEY_UNIT_EXPOSE_SPIES", &[]);
    }

    // Plot yields and the modifiers elsewhere.
    changes_help(&mut h, &txt(d, "TXT_KEY_BUILDING_WATER_PLOTS", &[]), ": ", "", sea_plot_yield_change, &YIELD_ICONS, false, true);
    changes_help(&mut h, &txt(d, "TXT_KEY_BUILDING_RIVER_PLOTS", &[]), ": ", "", river_plot_yield_change, &YIELD_ICONS, false, true);
    changes_help(&mut h, &txt(d, "TXT_KEY_BUILDING_WATER_PLOTS_ALL_CITIES", &[]), ": ", "", global_sea_plot_yield_change, &YIELD_ICONS, false, true);
    changes_help(&mut h, "", "", &txt(d, "TXT_KEY_BUILDING_WITH_POWER", &[]), power_yield_modifier, &YIELD_ICONS, true, true);
    changes_help(&mut h, "", "", &txt(d, "TXT_KEY_BUILDING_ALL_CITIES_THIS_CONTINENT", &[]), area_yield_modifier, &YIELD_ICONS, true, true);
    let all_cities = txt(d, "TXT_KEY_BUILDING_ALL_CITIES", &[]);
    changes_help(&mut h, "", "", &all_cities, global_yield_modifier, &YIELD_ICONS, true, true);
    changes_help(&mut h, "", "", &all_cities, global_commerce_modifier, &COMMERCE_ICONS, true, true);
    changes_help(&mut h, "", "", &txt(d, "TXT_KEY_BUILDING_PER_SPECIALIST_ALL_CITIES", &[]), specialist_extra_commerce, &COMMERCE_ICONS, false, true);
    // With no player, any state religion's buildings.
    changes_help(&mut h, "", "", &txt(d, "TXT_KEY_BUILDING_STATE_REL_BUILDINGS", &[]), state_religion_commerce, &COMMERCE_ICONS, false, true);
    for k in 0..CommerceType::NUM {
        let n = commerce_happiness[k];
        if n != 0 {
            add(&mut h, "TXT_KEY_BUILDING_PER_LEVEL", &[Arg::S(happy_icon(n)), Arg::N(100 / n), Arg::S(COMMERCE_ICONS[k])]);
        }
        if commerce_flexible[k] {
            add(&mut h, "TXT_KEY_BUILDING_ADJUST_COMM_RATE", &[Arg::S(COMMERCE_ICONS[k])]);
        }
    }
    for s in d.specialists.keys() {
        changes_help(&mut h, "", "", &from_specialist(s), &specialist_yield_change[s], &YIELD_ICONS, false, true);
    }
    for x in d.bonuses.keys() {
        let end = txt(d, "TXT_KEY_BUILDING_WITH_BONUS", &[Arg::S(&display(d, x))]);
        changes_help(&mut h, "", "", &end, &bonus_yield_modifier[x], &YIELD_ICONS, true, true);
    }
    for r in d.religions.keys().filter(|&r| religion_change[r] > 0) {
        add(&mut h, "TXT_KEY_BUILDING_SPREADS_RELIGION", &[Arg::S(&highlight(&display(d, r)))]);
    }
    for s in d.specialists.keys() {
        let n = specialist_count[s];
        if n == 1 {
            add(&mut h, "TXT_KEY_BUILDING_TURN_CITIZEN_INTO", &[Arg::S(&display(d, s))]);
        } else if n > 1 {
            add(
                &mut h,
                "TXT_KEY_BUILDING_TURN_CITIZENS_INTO",
                &[Arg::N(n), Arg::S(&display(d, s))],
            );
        }
        if free_specialist_count[s] > 0 {
            add(
                &mut h,
                "TXT_KEY_BUILDING_FREE_SPECIALIST",
                &[Arg::N(free_specialist_count[s]), Arg::S(&display(d, s))],
            );
        }
    }

    // Lists grouped by their number (C++ `setListHelp`: a new line when the number changes).
    let mut last = 0;
    for i in d.improvements.keys() {
        let n = improvement_free_specialist[i];
        if n > 0 {
            let start = txt(d, "TXT_KEY_BUILDING_IMPROVEMENT_FREE_SPECIALISTS", &[Arg::N(n)]);
            list_help(&mut h, &format!("{NEWLINE}{start}"), &display(d, i), ", ", n != last);
            last = n;
        }
    }
    let from = |n: i32, icon: &str| txt(d, "TXT_KEY_BUILDING_HEALTH_HAPPINESS_CHANGE", &[Arg::N(n.abs()), Arg::S(icon)]);
    let with = |n: i32, icon: &str| txt(d, "TXT_KEY_BUILDING_CIVIC_HEALTH_HAPPINESS_CHANGE", &[Arg::N(n.abs()), Arg::S(icon)]);
    let mut last = 0;
    for x in d.bonuses.keys() {
        let n = bonus_health_changes[x];
        if n != 0 {
            list_help(&mut h, &format!("{NEWLINE}{}", from(n, health_icon(n))), &display(d, x), ", ", n != last);
            last = n;
        }
    }
    if let Some(c) = class {
        let mut last = 0;
        for civic in d.civics.keys() {
            let n = d.civics[civic].building_health_changes[c];
            if n != 0 {
                list_help(&mut h, &format!("{NEWLINE}{}", with(n, health_icon(n))), &display(d, civic), ", ", n != last);
                last = n;
            }
        }
    }
    let mut last = 0;
    for x in d.bonuses.keys() {
        let n = bonus_happiness_changes[x];
        if n != 0 {
            list_help(&mut h, &format!("{NEWLINE}{}", from(n, happy_icon(n))), &display(d, x), ", ", n != last);
            last = n;
        }
    }
    if let Some(c) = class {
        let mut last = 0;
        for civic in d.civics.keys() {
            let n = d.civics[civic].building_happiness_changes[c];
            if n != 0 {
                list_help(&mut h, &format!("{NEWLINE}{}", with(n, happy_icon(n))), &display(d, civic), ", ", n != last);
                last = n;
            }
        }
    }
    for uc in d.unit_combats.keys() {
        let n = unit_combat_free_experience[uc];
        if n != 0 {
            add(&mut h, "TXT_KEY_BUILDING_FREE_XP", &[Arg::S(&display(d, uc)), Arg::N(n)]);
        }
    }
    for dom in d.domains.keys() {
        let n = domain_free_experience[dom];
        if n != 0 {
            add(&mut h, "TXT_KEY_BUILDING_FREE_XP", &[Arg::S(&display(d, dom)), Arg::N(n)]);
        }
    }
    for dom in d.domains.keys() {
        let n = domain_production_modifier[dom];
        if n != 0 {
            add(&mut h, "TXT_KEY_BUILDING_BUILDS_FASTER_DOMAIN", &[Arg::S(&display(d, dom)), Arg::N(n)]);
        }
    }

    // The units that need it (FfH: the civilization's unit of each class; the building or its
    // class), and those that can build it.
    let to_train = format!("{NEWLINE}{}", txt(d, "TXT_KEY_BUILDING_REQUIRED_TO_TRAIN", &[]));
    let mut first = true;
    for uc in d.unit_classes.keys() {
        let Some(u) = civ_unit(uc) else { continue };
        let unit = &d.units[u];
        if unit.prereq_building == Some(b) || (class.is_some() && unit.prereq_building_class == class) {
            list_help(&mut h, &to_train, &color("COLOR_UNIT_TEXT", &display(d, u)), ", ", first);
            first = false;
        }
    }
    let built_by = format!("{NEWLINE}{}", txt(d, "TXT_KEY_UNIT_REQUIRED_TO_BUILD", &[]));
    let mut first = true;
    for u in d.units.keys().filter(|&u| d.units[u].buildings[b] || d.units[u].force_buildings[b]) {
        list_help(&mut h, &built_by, &color("COLOR_UNIT_TEXT", &display(d, u)), ", ", first);
        first = false;
    }
    let mut last = 0;
    for c in d.building_classes.keys() {
        let n = building_happiness_changes[c];
        // C++ slip fixed: for a civilization without a building of the class it lists an empty
        // name.
        if n != 0 && (owner.is_none() || civ_building(c).is_some()) {
            let start = txt(d, "TXT_KEY_BUILDING_HAPPINESS_CHANGE", &[Arg::N(n), Arg::S(happy_icon(n))]);
            list_help(&mut h, &format!("{NEWLINE}{start}"), &display(d, c), ", ", n != last);
            last = n;
        }
    }
    if let Some(x) = *power_bonus {
        add(&mut h, "TXT_KEY_BUILDING_PROVIDES_POWER_WITH", &[Arg::S(&display(d, x))]);
        if *dirty_power && dirty != 0 {
            h.push_str(&dirty_health);
        }
    }
    // The buildings that need it in their city (the civilization's building of each class).
    if let Some(c) = class {
        let to_build = format!("{NEWLINE}{}", txt(d, "TXT_KEY_BUILDING_REQUIRED_TO_BUILD", &[]));
        let mut first = true;
        for other in d.building_classes.keys().filter_map(civ_building) {
            if d.buildings[other].building_class_needed_in_city[c] {
                list_help(&mut h, &to_build, &color("COLOR_BUILDING_TEXT", &display(d, other)), ", ", first);
                first = false;
            }
        }
    }

    // The Civilopedia's: the traits that build it faster or make it happier, the great person it
    // makes likelier (when the first line did not name it), the start era that gives it free.
    for t in d.traits.keys() {
        let n = production_traits[t];
        if n == 100 {
            add(&mut h, "TXT_KEY_DOUBLE_SPEED_TRAIT", &[Arg::S(&display(d, t))]);
        } else if n != 0 {
            add(&mut h, "TXT_KEY_PRODUCTION_MODIFIER_TRAIT", &[Arg::N(n), Arg::S(&display(d, t))]);
        }
    }
    for t in d.traits.keys() {
        let n = happiness_traits[t];
        if n != 0 {
            add(&mut h, "TXT_KEY_BUILDING_HAPPINESS_TRAIT", &[Arg::N(n), Arg::S(&display(d, t))]);
        }
    }
    if let Some(u) = great_person.filter(|_| *great_people_rate_change == 0) {
        add(&mut h, "TXT_KEY_BUILDING_LIKELY_TO_GENERATE", &[Arg::S(&display(d, u))]);
    }
    if let Some(e) = *free_start_era {
        add(&mut h, "TXT_KEY_BUILDING_FREE_START_ERA", &[Arg::S(&display(d, e))]);
    }

    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        h.push_str(NEWLINE);
        h.push_str(help);
    }

    // C++ `buildBuildingRequiresString`, which with no city writes every requirement.
    let requires = format!("{NEWLINE}{}", req(d, "TXT_KEY_REQUIRES", &[]));
    let and = txt(d, "TXT_KEY_AND", &[]);
    let or = txt(d, "TXT_KEY_OR", &[]);
    if let Some(r) = *holy_city {
        // FFH2's religion names have no article ("Order"): "in the Order Holy City".
        add_req(&mut h, "TXT_KEY_ACTION_ONLY_HOLY_CONSTRUCT", &[Arg::S(&highlight(&display(d, r)))]);
    }
    if let Some(sb) = *special_building_type {
        let mut first = true;
        for p in d.projects.keys().filter(|&p| d.projects[p].everyone_special_building == Some(sb)) {
            list_help(&mut h, &requires, &highlight(&display(d, p)), &or, first);
            first = false;
        }
        if let Some(t) = d.special_buildings[sb].tech_prereq_anyone {
            add_req(
                &mut h,
                "TXT_KEY_REQUIRES_TECH_ANYONE",
                &[Arg::S(&display(d, t))],
            );
        }
    }
    for c in d.building_classes.keys() {
        // The civilization's building of the class (C++ fails on a class without one when counting
        // and passes it over when it is needed in the city: the class's name here). C++ with no
        // player writes the count of the XML; a player's scales with the world size.
        let name = || highlight(&civ_building(c).map_or_else(|| display(d, c), |x| display(d, x)));
        let n = prereq_num_of_building_class[c];
        if n > 0 {
            add_req(&mut h, "TXT_KEY_BUILDING_REQUIRES_NUM_SPECIAL_BUILDINGS_NO_CITY", &[Arg::S(&name()), Arg::N(n)]);
        }
        // C++ slip fixed: an `else if`, so that a class both counted and needed in the city (the
        // Great Library's libraries) said only the count. After the count, ffh-play's line says
        // where the other one stands.
        if building_class_needed_in_city[c] && n > 0 {
            h.push_str(&format!("{NEWLINE}Requires {} in the city", name()));
        } else if building_class_needed_in_city[c] {
            add_req(&mut h, "TXT_KEY_BUILDING_REQUIRES_STRING", &[Arg::S(&name())]);
        }
    }
    if *is_state_religion {
        add_req(&mut h, "TXT_KEY_REQUIRES_STATE_RELIGION", &[]);
    }
    // ffh-play's: the religions FFH2's `canConstruct` asks for.
    if let Some(r) = *prereq_religion {
        h.push_str(&format!("{NEWLINE}Requires the {} religion in the city", highlight(&display(d, r))));
    }
    if let Some(r) = *state_religion {
        h.push_str(&format!("{NEWLINE}Requires the {} state religion", highlight(&display(d, r))));
    }
    if *is_prereq_religion {
        h.push_str(&format!("{NEWLINE}Requires a city without a religion"));
    }
    if *num_cities_prereq > 0 {
        add_req(&mut h, "TXT_KEY_BUILDING_REQUIRES_NUM_CITIES", &[Arg::N(*num_cities_prereq)]);
    }
    if *unit_level_prereq > 0 {
        add_req(&mut h, "TXT_KEY_BUILDING_REQUIRES_UNIT_LEVEL", &[Arg::N(*unit_level_prereq)]);
    }
    if *min_latitude > 0 {
        add_req(&mut h, "TXT_KEY_MIN_LATITUDE", &[Arg::N(*min_latitude)]);
    }
    if *max_latitude < 90 {
        add_req(&mut h, "TXT_KEY_MAX_LATITUDE", &[Arg::N(*max_latitude)]);
    }
    // ffh-play's: where it may stand (C++ `CvCity::isValidBuildingLocation`: the coast with water of
    // at least the area size, or a river when both are set; else the land's size and a river).
    if *water {
        let mut coast = "Requires a coastal city".to_string();
        if *min_area_size > 0 {
            coast.push_str(&format!(" (water of at least {min_area_size} tiles)"));
        }
        if *river {
            coast.push_str(" or a river");
        }
        h.push_str(&format!("{NEWLINE}{coast}"));
    } else {
        if *min_area_size > 0 {
            h.push_str(&format!("{NEWLINE}Requires a landmass of at least {min_area_size} tiles"));
        }
        if *river {
            h.push_str(&format!("{NEWLINE}Requires a river"));
        }
    }
    // The Civilopedia's.
    if let Some(v) = *victory_prereq {
        add(&mut h, "TXT_KEY_BUILDING_REQUIRES_VICTORY", &[Arg::S(&display(d, v))]);
    }
    if let Some(e) = *max_start_era {
        add(&mut h, "TXT_KEY_BUILDING_MAX_START_ERA", &[Arg::S(&display(d, e))]);
    }
    if *num_teams_prereq > 0 {
        add(&mut h, "TXT_KEY_BUILDING_REQUIRES_NUM_TEAMS", &[Arg::N(*num_teams_prereq)]);
    }
    // The tooltip's: techs and resources.
    if let Some(t) = *prereq_and_tech {
        add_req(&mut h, "TXT_KEY_BUILDING_REQUIRES_STRING", &[Arg::S(&highlight(&display(d, t)))]);
    }
    let mut first = true;
    for &t in prereq_and_techs.iter().flatten() {
        list_help(&mut h, &requires, &highlight(&display(d, t)), &and, first);
        first = false;
    }
    if let Some(x) = *prereq_and_bonus {
        add_req(&mut h, "TXT_KEY_UNIT_REQUIRES_STRING", &[Arg::S(&highlight(&display(d, x)))]);
    }
    // FfH: every one of these resources ("and"; BtS asks for one of them). C++ builds the list
    // aside and writes it after the trait.
    let mut bonuses = String::new();
    let mut first = true;
    for &x in prereq_or_bonuses.iter().flatten() {
        list_help(&mut bonuses, &requires, &highlight(&display(d, x)), &and, first);
        first = false;
    }
    if let Some(t) = *prereq_trait {
        add_req(&mut h, "TXT_KEY_UNIT_REQUIRES_STRING", &[Arg::S(&highlight(&display(d, t)))]);
    }
    h.push_str(&bonuses);
    if let Some(c) = *founds_corporation {
        let mut first = true;
        for &x in &d.corporations[c].prereq_bonus {
            list_help(&mut h, &requires, &highlight(&display(d, x)), &or, first);
            first = false;
        }
    }

    // The resources that speed it up (the Civilopedia's lines; the tooltip appends them to the
    // cost), then when it goes obsolete.
    for x in d.bonuses.keys() {
        let n = bonus_production_modifier[x];
        if n != 0 {
            h.push_str(NEWLINE);
            h.push_str("[ICON_BULLET]");
            h.push_str(&if n == 100 {
                txt(d, "TXT_KEY_BUILDING_DOUBLE_SPEED_WITH", &[Arg::S(&display(d, x))])
            } else {
                txt(d, "TXT_KEY_BUILDING_BUILDS_FASTER_WITH", &[Arg::N(n), Arg::S(&display(d, x))])
            });
        }
    }
    let obsolete =
        |h: &mut String, t: TechType| add(h, "TXT_KEY_BUILDING_OBSOLETE_WITH", &[Arg::S(&display(d, t))]);
    if let Some(t) = *obsolete_tech {
        obsolete(&mut h, t);
        if *defense_modifier != 0 || *bombard_defense_modifier != 0 {
            h.push_str(&txt(d, "TXT_KEY_BUILDING_OBSOLETE_EXCEPT", &[]));
        }
    }
    if let Some(t) = special_building_type.and_then(|sb| d.special_buildings[sb].obsolete_tech) {
        obsolete(&mut h, t);
    }
    finish(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> &'static GameData {
        ffh_data::global()
    }

    fn help(name: &str) -> String {
        let d = data();
        effects(d, d.t::<BuildingType>(name))
    }

    /// The text as read: colours and links dropped, `[SPACE]` a space, icons and line breaks kept.
    fn read(t: &str) -> String {
        let mut out = String::new();
        let mut rest = t;
        while let Some(at) = rest.find('[') {
            out.push_str(&rest[..at]);
            let tail = &rest[at..];
            let Some(end) = tail.find(']') else { break };
            let tag = &tail[1..end];
            if tag == "SPACE" {
                out.push(' ');
            } else if !(tag.starts_with("COLOR_") || tag.starts_with("LINK") || tag == "\\LINK") {
                out.push_str(&tail[..=end]);
            }
            rest = &tail[end + 1..];
        }
        out.push_str(rest);
        out
    }

    fn says(name: &str, parts: &[&str]) -> String {
        let h = read(&help(name));
        for p in parts {
            assert!(h.contains(p), "{name} should say \"{p}\":\n{}", h.replace("[NEWLINE]", "\n"));
        }
        h
    }

    #[test]
    fn known_buildings_say_what_they_do_and_require() {
        // CIV4BuildingInfos.xml: iFoodKept 20; +1 health with Corn, Rice and Wheat; Expansive
        // leaders build it at double speed; PrereqTech Agriculture.
        says("BUILDING_GRANARY", &["Stores 20% of [ICON_FOOD] after Growth", "+1[ICON_HEALTHY] from Corn, Rice, Wheat", "Double production speed for Expansive leaders", "Requires Agriculture"]);
        // iDefense 10, Defender leaders at double speed, no tech.
        let palisade = says("BUILDING_PALISADE", &["10% Defense", "Double production speed for Defender leaders"]);
        assert!(!palisade.contains("Requires"), "{palisade}");
        // iHealth 1, iHealRateChange 10, +1 health with Reagents.
        says("BUILDING_HERBALIST", &["[ICON_BULLET]+1[ICON_HEALTHY]", "Heals Units Extra 10% Damage/Turn", "+1[ICON_HEALTHY] from Reagents", "Requires Agriculture"]);
        // The research building: +25% research, a Sage (scientist) slot, Philosophical, Writing.
        let library = says("BUILDING_LIBRARY", &["Can turn 1 Citizen into Sage", "Double production speed for Philosophical leaders", "Requires Writing"]);
        assert!(library.starts_with("[ICON_BULLET]+25%[ICON_RESEARCH]"), "{library}");
        // A world wonder: one allowed; 2 great people points (great scientist); a free Sage;
        // three libraries and one in the city; Writing; faster with Marble.
        let great_library = says(
            "BUILDING_GREAT_LIBRARY",
            &["World Wonder (1 Allowed)", "+2[ICON_GREATPEOPLE]", "+1 Free Sage", "Requires Library (3 Total)", "Requires Library in the city", "Requires Writing", "Double Production Speed with Marble"],
        );
        assert!(great_library.contains("[ICON_RESEARCH] from Sage in All Cities"), "{great_library}");
        // A religious temple: the Order in the city, Orders from Heaven, two priests, +1 happiness
        // with Incense, +10% military production, -5 crime.
        says(
            "BUILDING_TEMPLE_OF_THE_ORDER",
            &[
                "+10% Military Unit Production",
                "-5 to the crime rate",
                "Can turn 2 Citizens into Priest",
                "+1[ICON_HAPPY] from Incense",
                "Requires the Order religion in the city",
                "Requires Orders from Heaven",
            ],
        );
        // The Demon's Altar: the Ashen Veil as state religion, a Temple of the Veil in the city.
        says("BUILDING_DEMONS_ALTAR", &["Requires the Ashen Veil state religion", "Requires Temple of the Veil", "Allows you to sacrifice"]);
        // A coastal building, a victory's, a civilization's own.
        says("BUILDING_HARBOR", &["+50% Trade Route Yield", "Requires a coastal city (water of at least 10 tiles)"]);
        says("BUILDING_TOWER_OF_MASTERY", &["Tower of Mastery Victory must be Enabled", "Requires Tower of Alteration (1 Total)"]);
        says("BUILDING_ADVENTURERS_GUILD", &["Unique Building for Grigori"]);
        says("BUILDING_HARBOR_LANUN", &["Unique Building for Lanun", "Replaces Harbor"]);
        // A unique building's units are its civilization's: the Luchuirp's Training Yard trains
        // their golems (UnitInfos: Wood Golem, PrereqBuilding Sculptor's Studio), not Axemen; the
        // default building lists the default units.
        let studio = says("BUILDING_SCULPTORS_STUDIO", &["Required to Train", "Wood Golem", "Gargoyle"]);
        assert!(!studio.contains("Axeman"), "{studio}");
        says("BUILDING_TRAINING_YARD", &["Required to Train", "Axeman"]);
        says("BUILDING_GOVERNORS_MANOR", &["Required to Train Vampire"]);
    }

    /// No placeholder left, no text key, no empty icon brackets, and no colour that runs on past its
    /// line (a closing tag with none open is harmless).
    fn check_markup(name: &str, h: &str) {
        assert!(!h.as_bytes().windows(3).any(|w| w[0] == b'%' && w[1].is_ascii_alphabetic() && w[2].is_ascii_digit()), "{name}: {h}");
        assert!(!h.contains("TXT_KEY_") && !h.starts_with(NEWLINE) && !h.contains("()"), "{name}: {h}");
        for line in h.split(NEWLINE) {
            let mut depth = 0usize;
            let mut rest = line;
            while let Some(at) = rest.find("[COLOR_") {
                rest = &rest[at + 1..];
                if rest.starts_with("COLOR_REVERT]") {
                    depth = depth.saturating_sub(1);
                } else {
                    depth += 1;
                }
            }
            assert_eq!(depth, 0, "{name}: {line}");
        }
    }

    #[test]
    fn every_building_text_is_filled_and_closes_its_colours() {
        let d = data();
        for b in d.buildings.keys() {
            check_markup(names::name(d, b), &effects(d, b));
        }
    }

    /// The fields no FFH2 building sets, on a copy of the data's Monument.
    #[test]
    fn fields_ffh2_leaves_unset_read_as_civ4_writes_them() {
        let mut d = GameData::load(&ffh_data::default_data_dir()).expect("game data");
        let tech = |d: &GameData, n: &str| d.t::<TechType>(n);
        let (writing, math) = (tech(&d, "TECH_WRITING"), tech(&d, "TECH_MATHEMATICS"));
        let (iron, copper) = (d.t::<BonusType>("BONUS_IRON"), d.t::<BonusType>("BONUS_COPPER"));
        let industrious = d.t::<TraitType>("TRAIT_INDUSTRIOUS");
        let order = d.t::<ReligionType>("RELIGION_THE_ORDER");
        let (ancient, classical) = (d.t::<EraType>("ERA_ANCIENT"), d.t::<EraType>("ERA_CLASSICAL"));
        let government = d.t::<CivicOptionType>("CIVICOPTION_GOVERNMENT");
        let sage = d.t::<UnitClassType>("UNITCLASS_SCIENTIST");
        let b = d.t::<BuildingType>("BUILDING_MONUMENT");
        let info = &mut d.buildings[b];
        info.happiness = -2;
        info.prereq_and_techs = vec![Some(writing), Some(math), None];
        info.prereq_or_bonuses = vec![Some(iron), Some(copper), None, None, None];
        info.prereq_trait = Some(industrious);
        info.no_bonus = Some(iron);
        info.free_bonus = Some(copper);
        info.num_free_bonuses = -1;
        info.free_start_era = Some(classical);
        info.max_start_era = Some(ancient);
        info.num_teams_prereq = 2;
        info.anarchy_modifier = -100;
        info.free_techs = 2;
        info.area_happiness = -1;
        info.min_latitude = 30;
        info.max_latitude = 60;
        (info.water, info.river, info.min_area_size) = (true, true, 20);
        info.bombard_defense_modifier = 25;
        info.obsolete_tech = Some(math);
        info.commerce_happiness[CommerceType::COMMERCE_CULTURE.idx()] = 10;
        info.commerce_flexible[CommerceType::COMMERCE_ESPIONAGE.idx()] = true;
        info.espionage_defense_modifier = 50;
        (info.power, info.dirty_power) = (true, true);
        info.civic_option = Some(government);
        (info.religion_type, info.state_religion_happiness) = (Some(order), 2);
        info.production_traits[industrious] = 50;
        info.happiness_traits[industrious] = 1;
        info.is_prereq_religion = true;
        info.holy_city = Some(order);
        info.global_yield_modifier[YieldType::YIELD_PRODUCTION.idx()] = 10;
        info.specialist_extra_commerce[CommerceType::COMMERCE_GOLD.idx()] = 1;
        info.great_people_unit_class = Some(sage);
        let h = effects(&d, b);
        check_markup("BUILDING_MONUMENT", &h);
        let h = read(&h);
        for part in [
            "[ICON_BULLET]+2[ICON_UNHAPPY], +2[ICON_CULTURE]",
            "Removes access to Iron from the city.",
            "Provides 1-7 Copper",
            "Enables all Government Civics",
            "Provides Power ([ICON_POWER])",
            "No Anarchy",
            "+2[ICON_HAPPY] if Order is State [ICON_RELIGION]",
            "2 Free Technologies",
            "-25% Damage to Defenses from Bombardment",
            "+50% Defense against Espionage[NEWLINE][ICON_BULLET]Helps Thwart Rival Spies",
            "+10%[ICON_PRODUCTION] All Cities",
            "+1[ICON_GOLD] per Specialist in All Cities",
            "+1[ICON_HAPPY] per 10% [ICON_CULTURE] Rate",
            "Can Adjust [ICON_ESPIONAGE] Rate",
            "+50% production speed for Industrious leaders",
            "+1[ICON_HAPPY] for Industrious leaders",
            "City more likely to generate Great Sage",
            "Available for free on Classical and later starts",
            "Can only be Constructed in the Order Holy City.",
            "Requires a city without a religion",
            "Can build at a minimum latitude of 30 degrees",
            "Can build at a maximum latitude of 60 degrees",
            "Requires a coastal city (water of at least 20 tiles) or a river",
            "Can only be Built on Ancient and Earlier Starts",
            "Requires at least 2 Teams",
            "Requires Writing and Mathematics",
            // C++ order: the trait, then the resources it built aside.
            "Requires Industrious[NEWLINE]Requires Iron and Copper",
            "Obsolete with Mathematics (Except defensive bonus)",
        ] {
            assert!(h.contains(part), "should say \"{part}\":\n{}", h.replace("[NEWLINE]", "\n"));
        }
        // C++ quirk: a loss of happiness on the continent is not said.
        assert!(!h.contains("on this Continent"), "{h}");
    }

    /// The buildings whose XML sets a value the help describes, read from the XML itself (any tag
    /// but the ones below, with a value other than 0, -1, NONE or nothing; a latitude below 90),
    /// so that a field the struct or the help leaves out still counts.
    fn buildings_with_described_xml() -> Vec<String> {
        // Names, texts, art, sound, AI, the cost (the page's) and what C++ does not describe.
        const NOT_DESCRIBED: &[&str] = &[
            "Type",
            "BuildingClass",
            "Description",
            "Civilopedia",
            "Strategy",
            "Help",
            "Advisor",
            "ArtDefineTag",
            "MovieDefineTag",
            "ConstructSound",
            "HotKey",
            "bAltDown",
            "bShiftDown",
            "bCtrlDown",
            "iHotKeyPriority",
            "fVisibilityPriority",
            "bGraphicalOnly",
            "bCenterInCity",
            "iAIWeight",
            "FlavorType",
            "iFlavor",
            "iAsset",
            "iPower",
            "iAdvancedStartCost",
            "iAdvancedStartCostIncrease",
            "iCost",
            "iHurryCostModifier",
            "iConquestProb",
            "bTeamShare",
            "bNeverCapture",
            "bNukeImmune",
            "bEquipment",
            "PrereqCiv",
            "bVictoryBuilding",
            "CommerceChangeDoubleTimes",
            "CommerceChangeOriginalOwners",
            // A pair's key: its value is the number or the flag beside it.
            "BonusType",
            "BuildingClassType",
            "SpecialistType",
            "UnitCombatType",
            "DomainType",
            "ImprovementType",
            "ProductionTraitType",
            "HappinessTraitType",
            "ReligionType",
        ];
        let path = ffh_data::default_data_dir().join("xml").join("Buildings").join("CIV4BuildingInfos.xml");
        let xml = std::fs::read_to_string(path).expect("building XML");
        let mut out = Vec::new();
        for block in xml.split("<BuildingInfo>").skip(1) {
            let block = block.split("</BuildingInfo>").next().unwrap_or("");
            let mut ty = String::new();
            let mut described = false;
            let mut rest = block;
            while let Some(open) = rest.find('<') {
                rest = &rest[open + 1..];
                let Some(close) = rest.find('>') else { break };
                let tag = &rest[..close];
                rest = &rest[close + 1..];
                if tag.starts_with('/') || tag.starts_with('!') || tag.ends_with('/') {
                    continue;
                }
                let Some(end) = rest.find(&format!("</{tag}>")) else { continue };
                let value = &rest[..end];
                if value.contains('<') {
                    continue;
                }
                let value = value.trim();
                if tag == "Type" {
                    ty = value.to_string();
                } else if !NOT_DESCRIBED.contains(&tag) && !matches!(value, "0" | "-1" | "NONE" | "") && !(tag == "iMaxLatitude" && value == "90") {
                    described = true;
                }
            }
            if described {
                out.push(ty);
            }
        }
        out
    }

    #[test]
    fn every_building_with_a_described_value_says_something() {
        let d = data();
        let with = buildings_with_described_xml();
        assert!(with.len() > 100, "{}", with.len());
        for name in with {
            let b = d.t::<BuildingType>(&name);
            assert!(!effects(d, b).is_empty(), "{name} sets a value the help describes but says nothing");
        }
    }

    /// Prints the help of a few buildings: `-- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_samples() {
        let d = data();
        let names: Vec<String> = match std::env::var("FFH_BUILDINGS") {
            Ok(v) => v.split(',').map(str::to_string).collect(),
            Err(_) => ["BUILDING_LIBRARY", "BUILDING_GREAT_LIBRARY", "BUILDING_TEMPLE_OF_THE_ORDER", "BUILDING_DEMONS_ALTAR", "BUILDING_HARBOR_LANUN", "BUILDING_TOWER_OF_MASTERY"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        for n in names {
            let b = d.t::<BuildingType>(&n);
            println!("== {n}\n{}\n", effects(d, b).replace(NEWLINE, "\n"));
        }
    }
}
