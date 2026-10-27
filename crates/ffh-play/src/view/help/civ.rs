//! What a civilization, a leader and a trait bring (the new-game screen's help): C++
//! `CvGameTextMgr::parseCivInfos`, `parseLeaderTraits` and `parseTraits`, as the civilization
//! picker shows them (not the Dawn of Man screen's short form).

use ffh_data::*;

use super::{finish, list_help, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};
use crate::names::display;

/// C++ `isWorldUnitClass`.
fn world_unit_class(d: &GameData, c: UnitClassType) -> bool {
    d.unit_classes[c].max_global_instances != -1
}

/// C++ `isWorldWonderClass`.
fn world_wonder_class(d: &GameData, c: BuildingClassType) -> bool {
    d.building_classes[c].max_global_instances != -1
}

/// Unit `class`'s unit for `civ` (the class default without a civilization).
fn civ_unit(d: &GameData, civ: Option<CivilizationType>, class: UnitClassType) -> Option<UnitType> {
    let default = d.unit_classes[class].default_unit_index;
    match civ {
        Some(c) => d.civilizations[c].civilization_units(class, default),
        None => default,
    }
}

fn civ_building(d: &GameData, civ: Option<CivilizationType>, class: BuildingClassType) -> Option<BuildingType> {
    match civ {
        Some(c) => d.civilizations[c].civilization_buildings(class, &d.building_classes),
        None => d.building_classes[class].default_building_index,
    }
}

/// What trait `t` gives (C++ `parseTraits` with `bDawnOfMan` false): its name, then its effects,
/// each on its own indented line; `civ` picks the units and buildings it speeds up. Every field of
/// the trait is either described here or named as not shown, so that a new field cannot go
/// unnoticed (the build fails until it is listed).
pub fn trait_help(d: &GameData, t: TraitType, civ: Option<CivilizationType>) -> String {
    let TraitInfo {
        base,
        short_description: _,
        health,
        happiness,
        max_anarchy,
        upkeep_modifier,
        level_experience_modifier,
        great_people_rate_modifier,
        great_general_rate_modifier,
        domestic_great_general_rate_modifier,
        max_global_building_production_modifier: global_wonders,
        max_team_building_production_modifier: team_wonders,
        max_player_building_production_modifier: national_wonders,
        adaptive,
        agnostic,
        assimilation,
        barbarian_ally,
        ignore_food,
        insane,
        // Whether the Adaptive trait may pick it: not what the trait does.
        selectable: _,
        sprawling,
        free_xp_from_combat,
        max_cities,
        pillaging_gold,
        starting_gold,
        summon_duration,
        upgrade_cost_modifier,
        extra_yield_threshold,
        trade_yield_modifier,
        commerce_change,
        commerce_modifier,
        free_promotion,
        free_promotion_unit_combat,
    } = &d.traits[t];
    let mut b = String::new();
    b.push_str(NEWLINE);
    b.push_str(&format!("[COLOR_ALT_HIGHLIGHT_TEXT]{}[COLOR_REVERT]", display(d, t)));
    let help = crate::names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help {
        b.push_str(help);
    }
    let add = |b: &mut String, key: &str, fallback: &str, args: &[Arg]| b.push_str(&txt(d, key, fallback, args));
    if *health != 0 {
        add(&mut b, "TXT_KEY_TRAIT_HEALTH", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1[ICON_HEALTHY]/City", &[Arg::N(*health)]);
    }
    if *happiness != 0 {
        add(&mut b, "TXT_KEY_TRAIT_HAPPINESS", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1[ICON_HAPPY]/City", &[Arg::N(*happiness)]);
    }
    if *max_anarchy != -1 {
        if *max_anarchy == 0 {
            add(&mut b, "TXT_KEY_TRAIT_NO_ANARCHY", "[NEWLINE][SPACE][SPACE][ICON_BULLET]No Anarchy", &[]);
        } else {
            add(&mut b, "TXT_KEY_TRAIT_MAX_ANARCHY", "[NEWLINE][SPACE][SPACE][ICON_BULLET]Max %d1 [NUM1:Turn:Turns] of Anarchy", &[Arg::N(*max_anarchy)]);
        }
    }
    if *upkeep_modifier != 0 {
        add(&mut b, "TXT_KEY_TRAIT_CIVIC_UPKEEP_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% Civic Upkeep", &[Arg::N(*upkeep_modifier)]);
    }
    if *level_experience_modifier != 0 {
        add(&mut b, "TXT_KEY_TRAIT_CIVIC_LEVEL_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% XP Needed for Unit Promotions", &[Arg::N(*level_experience_modifier)]);
    }
    if *great_people_rate_modifier != 0 {
        add(&mut b, "TXT_KEY_TRAIT_GREAT_PEOPLE_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% [ICON_GREATPEOPLE] Birth Rate", &[Arg::N(*great_people_rate_modifier)]);
    }
    if *great_general_rate_modifier != 0 {
        add(&mut b, "TXT_KEY_TRAIT_GREAT_GENERAL_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% Great General Emergence", &[Arg::N(*great_general_rate_modifier)]);
    }
    if *domestic_great_general_rate_modifier != 0 {
        add(
            &mut b,
            "TXT_KEY_TRAIT_DOMESTIC_GREAT_GENERAL_MODIFIER",
            "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% Great General Emergence inside Cultural Borders",
            &[Arg::N(*domestic_great_general_rate_modifier)],
        );
    }
    if *global_wonders != 0 || *team_wonders != 0 || *national_wonders != 0 {
        if global_wonders == team_wonders && global_wonders == national_wonders {
            add(&mut b, "TXT_KEY_TRAIT_WONDER_PRODUCTION_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% Wonder Production", &[Arg::N(*global_wonders)]);
        } else {
            if *global_wonders != 0 {
                add(&mut b, "TXT_KEY_TRAIT_WORLD_WONDER_PRODUCTION_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% World Wonder Production", &[Arg::N(*global_wonders)]);
            }
            if *team_wonders != 0 {
                add(&mut b, "TXT_KEY_TRAIT_TEAM_WONDER_PRODUCTION_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% Team Wonder Production", &[Arg::N(*team_wonders)]);
            }
            if *national_wonders != 0 {
                add(&mut b, "TXT_KEY_TRAIT_NATIONAL_WONDER_PRODUCTION_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% National Wonder Production", &[Arg::N(*national_wonders)]);
            }
        }
    }
    for y in 0..YieldType::NUM {
        if extra_yield_threshold[y] > 0 {
            add(
                &mut b,
                "TXT_KEY_TRAIT_EXTRA_YIELD_THRESHOLDS",
                "[NEWLINE][SPACE][SPACE][ICON_BULLET]+1%F1_yield on Plots with %d2%F3_yield",
                &[Arg::S(YIELD_ICONS[y]), Arg::N(extra_yield_threshold[y]), Arg::S(YIELD_ICONS[y])],
            );
        }
        if trade_yield_modifier[y] != 0 {
            add(&mut b, "TXT_KEY_TRAIT_TRADE_YIELD_MODIFIERS", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%F2_yield from [ICON_TRADE]", &[Arg::N(trade_yield_modifier[y]), Arg::S(YIELD_ICONS[y])]);
        }
    }
    for k in 0..CommerceType::NUM {
        if commerce_change[k] != 0 {
            add(&mut b, "TXT_KEY_TRAIT_COMMERCE_CHANGES", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%F2_commerce/City", &[Arg::N(commerce_change[k]), Arg::S(COMMERCE_ICONS[k])]);
        }
        if commerce_modifier[k] != 0 {
            add(&mut b, "TXT_KEY_TRAIT_COMMERCE_MODIFIERS", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1%% %F2_commerce", &[Arg::N(commerce_modifier[k]), Arg::S(COMMERCE_ICONS[k])]);
        }
    }
    let promos: Vec<String> = d.promotions.keys().filter(|&p| free_promotion[p]).map(|p| display(d, p).into_owned()).collect();
    if !promos.is_empty() {
        add(&mut b, "TXT_KEY_TRAIT_FREE_PROMOTIONS", "[NEWLINE][SPACE][SPACE][ICON_BULLET]Free Promotion (%s1)", &[Arg::S(&promos.join(", "))]);
        for uc in d.unit_combats.keys().filter(|&uc| free_promotion_unit_combat[uc]) {
            b.push_str(&format!("{NEWLINE}[SPACE][SPACE][SPACE][SPACE][ICON_BULLET]{}", display(d, uc)));
        }
    }
    for o in d.civic_options.keys().filter(|&o| d.civic_options[o].trait_no_upkeep[t]) {
        add(&mut b, "TXT_KEY_TRAIT_NO_UPKEEP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]No Upkeep Costs for %s1", &[Arg::S(&display(d, o))]);
    }
    // The units and buildings it builds faster, grouped by the speed-up.
    let speed = |n: i32| {
        if n == 100 {
            txt(d, "TXT_KEY_TRAIT_DOUBLE_SPEED", "[NEWLINE][SPACE][SPACE][ICON_BULLET]Double Production Speed of ", &[])
        } else {
            txt(d, "TXT_KEY_TRAIT_PRODUCTION_MODIFIER", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%d1%% Faster Production of ", &[Arg::N(n)])
        }
    };
    let mut last = 0;
    for su in d.special_units.keys() {
        let n = d.special_units[su].production_traits[t];
        if n != 0 {
            list_help(&mut b, &speed(n), &display(d, su), ", ", n != last);
            last = n;
        }
    }
    let mut last = 0;
    for class in d.unit_classes.keys().filter(|&c| !world_unit_class(d, c)) {
        let Some(u) = civ_unit(d, civ, class) else { continue };
        let n = d.units[u].production_traits[t];
        if n != 0 {
            list_help(&mut b, &speed(n), &display(d, u), ", ", n != last);
            last = n;
        }
    }
    let mut last = 0;
    for sb in d.special_buildings.keys() {
        let n = d.special_buildings[sb].production_traits[t];
        if n != 0 {
            list_help(&mut b, &speed(n), &display(d, sb), ", ", n != last);
            last = n;
        }
    }
    let mut last = 0;
    for class in d.building_classes.keys().filter(|&c| !world_wonder_class(d, c)) {
        let Some(bl) = civ_building(d, civ, class) else { continue };
        let n = d.buildings[bl].production_traits[t];
        if n != 0 {
            list_help(&mut b, &speed(n), &display(d, bl), ", ", n != last);
            last = n;
        }
    }
    let mut last = 0;
    for class in d.building_classes.keys().filter(|&c| !world_wonder_class(d, c)) {
        let Some(bl) = civ_building(d, civ, class) else { continue };
        let n = d.buildings[bl].happiness_traits[t];
        if n != 0 {
            let icon = if n > 0 { "[ICON_HAPPY]" } else { "[ICON_UNHAPPY]" };
            let start = txt(d, "TXT_KEY_TRAIT_BUILDING_HAPPINESS", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1_Change%F2_HappOrUnhapp from ", &[Arg::N(n.abs()), Arg::S(icon)]);
            list_help(&mut b, &start, &display(d, bl), ", ", n != last);
            last = n;
        }
    }
    // FfH.
    for (has, key) in [
        (*adaptive, "TXT_KEY_TRAIT_ADAPTIVE_HELP"),
        (*agnostic, "TXT_KEY_TRAIT_AGNOSTIC_HELP"),
        (*assimilation, "TXT_KEY_TRAIT_ASSIMILATION_HELP"),
        (*barbarian_ally, "TXT_KEY_TRAIT_BARBARIAN_ALLY_HELP"),
        (*ignore_food, "TXT_KEY_TRAIT_IGNORE_FOOD_HELP"),
        (*insane, "TXT_KEY_TRAIT_INSANE_HELP"),
        (*sprawling, "TXT_KEY_TRAIT_SPRAWLING_HELP"),
    ] {
        if has {
            add(&mut b, key, key, &[]);
        }
    }
    if *max_cities != -1 {
        // C++ adds the world size's `getMaxCitiesMod`, unknown before the game: said in words.
        b.push_str(&txt(d, "TXT_KEY_TRAIT_MAX_CITIES_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1 Maximum Cities", &[Arg::N(*max_cities)]));
        b.push_str(" (plus the world size's)");
    }
    for (n, key, fallback) in [
        (*free_xp_from_combat, "TXT_KEY_TRAIT_FREE_XP_FROM_COMBAT_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1 XP from combat"),
        (*pillaging_gold, "TXT_KEY_TRAIT_PILLAGING_GOLD_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1% gold from pillaging"),
        (*starting_gold, "TXT_KEY_TRAIT_STARTING_GOLD_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1 to starting gold"),
        (*summon_duration, "TXT_KEY_TRAIT_SUMMON_DURATION_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1 duration to summoned units"),
        (*upgrade_cost_modifier, "TXT_KEY_TRAIT_UPGRADE_COST_MODIFIER_HELP", "[NEWLINE][SPACE][SPACE][ICON_BULLET]%D1% to upgrade costs"),
    ] {
        if n != 0 {
            add(&mut b, key, fallback, &[Arg::N(n)]);
        }
    }
    finish(b)
}

/// What a leader brings (C++ `parseLeaderTraits` for the civilization picker): its alignment,
/// its civilization's hero and world spell, and each of its traits.
pub fn leader_help(d: &GameData, l: LeaderHeadType, civ: Option<CivilizationType>) -> String {
    let info = &d.leader_heads[l];
    let mut b = format!("[COLOR_HIGHLIGHT_TEXT]{}[COLOR_REVERT]", display(d, l));
    if let Some(a) = info.alignment {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_ALIGNMENT", "Alignment: %s1", &[Arg::S(&display(d, a))]));
    }
    if let Some(c) = civ {
        if let Some(hero) = d.civilizations[c].hero {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_HERO", "Hero: %s1", &[Arg::S(&display(d, hero))]));
        }
        for s in d.spells.keys().filter(|&s| d.spells[s].global && d.spells[s].civilization_prereq == Some(c)) {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_GLOBAL_SPELL", "World Spell: %s1", &[Arg::S(&display(d, s))]));
        }
    }
    for t in d.traits.keys().filter(|&t| info.has_trait[t]) {
        b.push_str(NEWLINE);
        b.push_str(&trait_help(d, t, civ));
    }
    b
}

/// What a civilization brings (C++ `parseCivInfos` for the civilization picker): its starting
/// techs, trait, hero, world spell, terrain yields, unique and blocked units and buildings, and
/// its special abilities (its XML help).
pub fn civ_help(d: &GameData, c: CivilizationType) -> String {
    let info = &d.civilizations[c];
    let head = |b: &mut String, key: &str, fallback: &str| {
        b.push_str(NEWLINE);
        b.push_str(&format!("[COLOR_ALT_HIGHLIGHT_TEXT]{}[COLOR_REVERT]", txt(d, key, fallback, &[])));
    };
    let item = |b: &mut String, s: &str| b.push_str(&format!("{NEWLINE}[SPACE][SPACE][ICON_BULLET]{s}"));
    let none = |b: &mut String, key: &str, fallback: &str| b.push_str(&format!("{NEWLINE}[SPACE][SPACE]{}", txt(d, key, fallback, &[])));
    let mut b = format!("[COLOR_HIGHLIGHT_TEXT]{}[COLOR_REVERT]", display(d, c));

    head(&mut b, "TXT_KEY_FREE_TECHS", "Starting Technologies");
    let techs: Vec<TechType> = d.techs.keys().filter(|&t| info.civilization_free_techs[t]).collect();
    for &t in &techs {
        item(&mut b, &display(d, t));
    }
    if techs.is_empty() {
        none(&mut b, "TXT_KEY_FREE_TECHS_NO", "No Starting Technologies");
    }
    if let Some(t) = info.civ_trait {
        head(&mut b, "TXT_KEY_CIV_TRAIT", "Civilization Trait");
        item(&mut b, &display(d, t));
    }
    if let Some(hero) = info.hero {
        head(&mut b, "TXT_KEY_HERO_PEDIA", "Hero");
        item(&mut b, &display(d, hero));
    }
    head(&mut b, "TXT_KEY_GLOBAL_SPELL_PEDIA", "World Spell");
    for s in d.spells.keys().filter(|&s| d.spells[s].global && d.spells[s].civilization_prereq == Some(c)) {
        item(&mut b, &display(d, s));
    }
    let mut first = true;
    for terrain in d.terrains.keys().filter(|&t| d.terrains[t].civilization_yield_type == Some(c)) {
        for y in 0..YieldType::NUM {
            let n = d.terrains[terrain].civilization_yield_change[y];
            if n != 0 {
                if first {
                    head(&mut b, "TXT_KEY_MISC_CIV_TERRAIN", "Terrain Modifier");
                    first = false;
                }
                item(&mut b, &txt(d, "TXT_KEY_MISC_CIV_TERRAIN_MOD", "%D1 %F2_yield from %s3_Terrain tiles", &[Arg::N(n), Arg::S(YIELD_ICONS[y]), Arg::S(&display(d, terrain))]));
            }
        }
    }
    // Unique units: the civilization's own in place of a class default ("Ours - (Default)"), or
    // a unit only it can build.
    head(&mut b, "TXT_KEY_FREE_UNITS", "Unique Units");
    let mut found = false;
    for class in d.unit_classes.keys() {
        let (Some(ours), Some(default)) = (civ_unit(d, Some(c), class), d.unit_classes[class].default_unit_index) else { continue };
        if ours != default {
            item(&mut b, &format!("{} - ({})", display(d, ours), display(d, default)));
            found = true;
        } else if !world_unit_class(d, class) && d.units[default].prereq_civ == Some(c) {
            item(&mut b, &display(d, ours));
            found = true;
        }
    }
    if !found {
        none(&mut b, "TXT_KEY_FREE_UNITS_NO", "No Unique Units");
    }
    head(&mut b, "TXT_KEY_MISC_CIV_BLOCKED_UNITS", "Blocked Units");
    let mut found = false;
    for class in d.unit_classes.keys() {
        let default = d.unit_classes[class].default_unit_index;
        if let Some(def) = default.filter(|&u| civ_unit(d, Some(c), class).is_none() && !world_unit_class(d, class) && d.units[u].prereq_civ.is_none()) {
            item(&mut b, &display(d, def));
            found = true;
        }
    }
    if !found {
        none(&mut b, "TXT_KEY_MISC_CIV_BLOCKED_UNITS_NO", "No Blocked Units");
    }
    head(&mut b, "TXT_KEY_UNIQUE_BUILDINGS", "Unique Buildings");
    let mut found = false;
    for class in d.building_classes.keys() {
        let Some(ours) = civ_building(d, Some(c), class) else { continue };
        let default = d.building_classes[class].default_building_index;
        if (default.is_some() || d.buildings[ours].prereq_civ == Some(c)) && Some(ours) != default {
            match default.filter(|_| d.buildings[ours].prereq_civ != Some(c)) {
                Some(def) => item(&mut b, &format!("{} - ({})", display(d, ours), display(d, def))),
                None => item(&mut b, &display(d, ours)),
            }
            found = true;
        }
    }
    if !found {
        none(&mut b, "TXT_KEY_UNIQUE_BUILDINGS_NO", "No Unique Buildings");
    }
    head(&mut b, "TXT_KEY_MISC_CIV_BLOCKED_BUILDINGS", "Blocked Buildings");
    let mut found = false;
    for class in d.building_classes.keys() {
        if let Some(def) = d.building_classes[class].default_building_index.filter(|_| civ_building(d, Some(c), class).is_none()) {
            item(&mut b, &display(d, def));
            found = true;
        }
    }
    if !found {
        none(&mut b, "TXT_KEY_MISC_CIV_BLOCKED_BUILDINGS_NO", "No Blocked Buildings");
    }
    let help = crate::names::text(d, &info.base.help);
    if !info.base.help.is_empty() && help != info.base.help {
        head(&mut b, "TXT_KEY_MISC_CIV_SPECIAL_ABILITIES", "Special Abilities");
        item(&mut b, help);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits_leaders_and_civilizations_say_what_they_bring() {
        let d = ffh_data::global();
        let bannor = d.t::<CivilizationType>("CIVILIZATION_BANNOR");
        let civ = civ_help(d, bannor);
        for part in ["Starting Technologies", "Civilization Trait", "Hero", "World Spell", "Unique Units", "Unique Buildings", "Crusade"] {
            assert!(civ.contains(part), "{part}: {civ}");
        }
        // Every trait a playable leader has says more than its name.
        for l in d.leader_heads.keys() {
            for t in d.traits.keys().filter(|&t| d.leader_heads[l].has_trait[t]) {
                let h = trait_help(d, t, None);
                assert!(h.matches("[ICON_BULLET]").count() >= 1, "{}: {h}", crate::names::name(d, t));
            }
        }
        let fin = trait_help(d, d.t::<TraitType>("TRAIT_FINANCIAL"), None);
        assert!(fin.contains("[ICON_BULLET]"), "{fin}");
        assert!(!civ.contains("%s") && !civ.contains("%d") && !fin.contains("%D"), "{civ}{fin}");
    }
}
