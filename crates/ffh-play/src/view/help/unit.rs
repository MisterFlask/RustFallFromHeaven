//! What a unit does: C++ `CvGameTextMgr::setUnitHelp` for a unit type (`pCity` NULL, no active
//! player) and the `setBasicUnitHelp` it calls, as the static help the page shows beside the unit's
//! name and strategy text.
//!
//! C++ writes the type's help in two modes: the tooltip (`bCivilopediaText` false: the strength and
//! moves line and the requirements) and the Civilopedia's (`bCivilopediaText` true: the speed-ups of
//! leader traits and resources as their own lines, every invisible type it sees, the units that
//! replace it). This is both, without what needs a city or a player (cost, turns, "N left").
//! Its order is C++'s, except that the strength and moves line comes first, with the unit combat
//! of C++'s name line (the page shows this text in place of its own numbers), and the XML help comes
//! last (C++ puts it before the requirements).
//!
//! FfH2's unit help says nothing of a few things its rules check or its pedia's panels show; these
//! lines are ffh-play's, marked as such below: the civilization of `PrereqCiv` when the help does
//! not name it, the religion and state religion a unit needs, the spells it can cast as it is
//! created (through its free promotions, or its own with its civilization's or religion's
//! prerequisite), the promotion an equipment is carried as, and the units it upgrades to (the pedia's
//! "Upgrades To" panel).

use ffh_data::*;

use super::{color, finish, list_help, txt, Arg, NEWLINE};
use crate::names::display;

/// A line of help: `NEWLINE` and the text of `key` (`fallback` when the key has no text).
fn add(d: &GameData, b: &mut String, key: &str, fallback: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&txt(d, key, fallback, args));
}

/// C++ `setListHelp` over a whole list: `start` and the first item, then `sep` and each other one.
fn list(b: &mut String, start: &str, items: &[String], sep: &str) {
    for (i, item) in items.iter().enumerate() {
        list_help(b, start, item, sep, i == 0);
    }
}

/// A "Requires X" line (C++ `TXT_KEY_UNIT_REQUIRES_STRING`).
fn requires(d: &GameData, b: &mut String, what: &str) {
    add(d, b, "TXT_KEY_UNIT_REQUIRES_STRING", "[COLOR_WARNING_TEXT]Requires %s1_Text[COLOR_REVERT]", &[Arg::S(what)]);
}

/// "Requires A or B" / "Requires A and B" (C++ `TXT_KEY_REQUIRES` + `setListHelp` + `ENDCOLR`);
/// nothing for an empty list.
fn requires_list(d: &GameData, b: &mut String, items: &[String], sep_key: &str, sep_fallback: &str) {
    if items.is_empty() {
        return;
    }
    let start = format!("{NEWLINE}{}", txt(d, "TXT_KEY_REQUIRES", "[COLOR_WARNING_TEXT]Requires[SPACE]", &[]));
    list(b, &start, items, &txt(d, sep_key, sep_fallback, &[]));
    b.push_str("[COLOR_REVERT]");
}

/// What unit `u` does (Civ4 markup, lines joined by `[NEWLINE]`): its strength and moves, what it
/// can do, its bonuses, what it requires. Every field of the unit is either described here or named
/// as not shown, so that a new field cannot go unnoticed (the build fails until it is listed).
pub fn effects(d: &GameData, u: UnitType) -> String {
    let UnitInfo {
        base,
        unit_class_type,
        special_unit_type,
        // Read by the rules only (the class a captured unit becomes); C++'s help leaves it out.
        unit_capture_class_type: _,
        unit_combat_type,
        domain_type,
        // AI.
        default_unit_ai_type: _,
        invisible_type,
        see_invisible_types,
        // Read by the rules only; C++'s help leaves them out (an animal says so by its unit combat).
        animal: _,
        food_production: _,
        no_bad_goodies,
        only_defensive,
        no_capture,
        // UI: whether combat is animated.
        quick_combat: _,
        rival_territory,
        military_happiness,
        // Read by the rules only (upkeep, production modifiers); C++'s help leaves them out.
        military_support: _,
        military_production: _,
        pillage,
        // BtS espionage flags, read by the rules only; C++'s help leaves them out.
        spy: _,
        sabotage: _,
        destroy: _,
        steal_plans: _,
        investigate,
        counter_spy,
        found,
        golden_age,
        invisible,
        first_strike_immune,
        no_defensive_bonus,
        // Read by the rules only; C++'s help leaves them out.
        ignore_building_defense: _,
        can_move_impassable,
        can_move_all_terrain: _,
        flat_movement_cost,
        ignore_terrain_cost,
        // Read by the rules only; C++'s help leaves them out (no FfH2 unit sets the two prereqs).
        nuke_immune: _,
        prereq_bonuses: _,
        is_prereq_religion: _,
        mech_unit: _,
        suicide: _,
        line_of_sight: _,
        hidden_nationality,
        always_hostile,
        // Read by the rules only; C++'s help leaves it out.
        no_reveal_map: _,
        upgrade_unit_class,
        target_unit_class,
        target_unit_combat,
        defender_unit_class,
        defender_unit_combat,
        flanking_strike_unit_class,
        // AI.
        unit_ai_type: _,
        not_unit_ai_type: _,
        builds,
        religion_spreads,
        corporation_spreads,
        terrain_passable_tech,
        feature_passable_tech,
        great_peoples,
        buildings,
        force_buildings,
        holy_city,
        religion_type,
        state_religion,
        prereq_religion,
        // Read by the rules only (FfH2 has no corporations); C++'s help leaves it out.
        prereq_corporation: _,
        prereq_building,
        prereq_and_tech,
        prereq_and_techs,
        prereq_and_bonus,
        prereq_or_bonuses,
        production_traits,
        // AI.
        flavor_value: _,
        ai_weight: _,
        // The cost line, which needs a player (`getProductionNeeded`): left out.
        production_cost: _,
        // Read by the rules only (hurrying, advanced start, the area a ship needs).
        hurry_cost_modifier: _,
        advanced_start_cost: _,
        advanced_start_cost_increase: _,
        min_area_size: _,
        moves,
        air_range,
        // Read by the rules only; C++'s help leaves them out.
        air_unit_cap: _,
        drop_range,
        nuke_range,
        work_rate: _,
        base_discover,
        discover_multiplier,
        base_hurry,
        hurry_multiplier,
        base_trade,
        trade_multiplier,
        great_work_culture,
        espionage_points,
        terrain_impassable,
        feature_impassable,
        combat,
        combat_limit,
        air_combat,
        // Read by the rules only (air combat, experience); C++'s help leaves them out.
        air_combat_limit: _,
        xp_value_attack: _,
        xp_value_defense: _,
        first_strikes,
        chance_first_strikes,
        interception_probability,
        evasion_probability,
        withdrawal_probability,
        collateral_damage,
        collateral_damage_limit,
        // Read by the rules only; C++'s help leaves it out.
        collateral_damage_max_units: _,
        city_attack_modifier,
        city_defense_modifier,
        animal_combat_modifier,
        hills_attack_modifier,
        hills_defense_modifier,
        // Read by the rules only; C++'s help leaves them out.
        terrain_native: _,
        feature_native: _,
        terrain_attack_modifier,
        terrain_defense_modifier,
        feature_attack_modifier,
        feature_defense_modifier,
        unit_class_attack_modifier,
        unit_class_defense_modifier,
        unit_combat_modifier,
        unit_combat_collateral_immune,
        domain_modifier,
        bonus_production_modifier,
        bomb_rate,
        bombard_rate,
        special_cargo,
        // Read by the rules only (C++'s help names the special cargo only).
        domain_cargo: _,
        cargo_space,
        // Read by the rules only (drafting, garrison culture); C++'s help leaves them out.
        conscription_value: _,
        culture_garrison_value: _,
        extra_cost,
        // Score and AI.
        asset_value: _,
        power_value: _,
        // The names given to the units made (great people): not what the type does.
        unit_names: _,
        free_promotions,
        leader_promotion,
        leader_experience,
        abandon,
        // Read by the rules only; FfH2's help leaves them out.
        auto_raze: _,
        disable_upgrade_to,
        explode_in_combat: _,
        free_xp: _,
        immortal,
        immune_to_defensive_strike,
        // Read by the rules only; FfH2's help leaves it out.
        never_obsolete: _,
        no_war_weariness,
        combat_defense,
        defensive_strike_chance,
        defensive_strike_damage,
        // Read by the rules only; FfH2's help leaves it out.
        duration_from_combat: _,
        enslavement_chance,
        free_promotion_pick,
        gold_from_combat,
        min_level,
        // Read by the rules only; FfH2's help leaves it out.
        miscast_chance: _,
        modify_global_counter,
        diplo_vote_type,
        equipment_promotion,
        prereq_alignment,
        prereq_building_class,
        prereq_civ,
        prereq_civic,
        prereq_global_counter,
        promotion_from_combat,
        // Read by the rules (the unit's place in its line, for conversions: Python
        // `getUnholyVersion`) and the AI; FfH2's help leaves it out.
        tier: _,
        unit_convert_from_combat,
        unit_convert_from_combat_chance,
        unit_create_from_combat,
        unit_create_from_combat_chance,
        // Read by the rules only (which civilization's units may upgrade to it); FfH2's help leaves it out.
        upgrade_civ: _,
        weapon_tier,
        // Read by the rules only; FfH2's help leaves it out.
        withdrawl_prob_defensive: _,
        // Python callbacks: C++ shows only the XML help for them.
        py_post_combat_lost: _,
        py_post_combat_won: _,
        damage_type_combat,
        bonus_affinity,
        // AI.
        ai_block_explore: _,
        ai_block_perm_defense: _,
        ai_block_patrol: _,
    } = &d.units[u];
    let class = *unit_class_type;
    let mut b = String::new();

    // ---- The strength and moves line (C++ `setBasicUnitHelp` without `bCivilopediaText`) ----
    if *domain_type == Some(DomainType::DOMAIN_AIR) {
        if *air_combat > 0 {
            b.push_str(&format!("{air_combat}[ICON_STRENGTH], "));
        }
    } else if *combat > 0 || *combat_defense > 0 {
        // FfH: units defend with `iCombatDefense`. C++ shows `getCombat()` only here (the type's
        // help was not updated; its unit help writes `att/def`), which misleads for the 52 units
        // whose defence differs (the Archer's 3/5): written as the unit help writes it.
        if combat_defense == combat {
            b.push_str(&format!("{combat}[ICON_STRENGTH], "));
        } else {
            b.push_str(&format!("{combat}/{combat_defense}[ICON_STRENGTH], "));
        }
    }
    b.push_str(&format!("{moves}[ICON_MOVES]"));
    if *air_range > 0 {
        b.push_str(", ");
        b.push_str(&txt(d, "TXT_KEY_UNIT_AIR_RANGE", "%d1_Num Range", &[Arg::N(*air_range)]));
    }
    // The unit combat C++'s name line shows after the name, " (Melee)".
    if let Some(uc) = unit_combat_type {
        b.push_str(&format!(" ({})", display(d, *uc)));
    }

    // ---- setUnitHelp: unique unit, limited unit ----
    let default = class.and_then(|c| d.unit_classes[c].default_unit_index);
    let civ_unit = |civ: CivilizationType| class.and_then(|c| d.civilizations[civ].civilization_units(c, default));
    // C++ quirk: FfH calls a class unique when more than 19 civilizations (of FfH2's 23) cannot
    // have it, and then names the civilizations whose unit of the class this is.
    let unique = d.civilizations.keys().filter(|&c| civ_unit(c).is_none()).count() > 19;
    // The civilizations it belongs to (for its spells below). C++ also names the setup's "Random"
    // civilization (the High Priest of Winter's), which never leads a player: left out.
    let random = opt_from_i32::<CivilizationType>(d.defines.random_civilization);
    let mut civs: Vec<CivilizationType> = Vec::new();
    if unique || default.is_some_and(|x| x != u) {
        for c in d.civilizations.keys().filter(|&c| civ_unit(c) == Some(u) && Some(c) != random) {
            add(d, &mut b, "TXT_KEY_UNIQUE_UNIT", "Unique Unit for [LINK=literal]%s1_civ[\\LINK]", &[Arg::S(&display(d, c))]);
            civs.push(c);
        }
        if let Some(x) = default.filter(|_| !unique) {
            add(d, &mut b, "TXT_KEY_REPLACES_UNIT", "Replaces [LINK=literal]%s1_default_unit[\\LINK]", &[Arg::S(&display(d, x))]);
        }
    }
    // ffh-play's: `PrereqCiv` (CvCity::canTrain) when the lines above do not name it (the class
    // is open to every civilization: the Angel, the Priest of Winter, two barbarian units).
    if let Some(c) = prereq_civ.filter(|c| !civs.contains(c)) {
        add(d, &mut b, "TXT_KEY_UNIQUE_UNIT", "Unique Unit for [LINK=literal]%s1_civ[\\LINK]", &[Arg::S(&display(d, c))]);
        civs.push(c);
    }
    if let Some(c) = class {
        let ci = &d.unit_classes[c];
        // Without a city: how many are allowed (with one: how many are left).
        if ci.max_global_instances != -1 {
            add(d, &mut b, "TXT_KEY_UNIT_WORLD_UNIT_ALLOWED", "[ICON_BULLET]World Unit (%d1_Num Allowed)", &[Arg::N(ci.max_global_instances)]);
        }
        if ci.max_team_instances != -1 {
            add(d, &mut b, "TXT_KEY_UNIT_TEAM_UNIT_ALLOWED", "[ICON_BULLET]Team Unit (%d1_Num Allowed)", &[Arg::N(ci.max_team_instances)]);
        }
        if ci.max_player_instances != -1 {
            add(d, &mut b, "TXT_KEY_UNIT_NATIONAL_UNIT_ALLOWED", "[ICON_BULLET]National Unit (%d1_Num Allowed)", &[Arg::N(ci.max_player_instances)]);
        }
        if ci.instance_cost_modifier != 0 {
            add(d, &mut b, "TXT_KEY_UNIT_INSTANCE_COST_MOD", "Each Unit in Play Increases Base Cost by %d1%%", &[Arg::N(ci.instance_cost_modifier)]);
        }
    }

    // ---- setBasicUnitHelp ----
    if *golden_age {
        add(d, &mut b, "TXT_KEY_UNIT_GOLDEN_AGE", "[ICON_BULLET]Can Start a Golden Age", &[]);
    }
    if *leader_experience > 0 {
        if d.defines.warlord_extra_experience_per_unit_percent == 0 {
            add(d, &mut b, "TXT_KEY_UNIT_LEADER", "[ICON_BULLET]Can Provide %d1_Num Experience to Units in the Same Tile", &[Arg::N(*leader_experience)]);
        } else {
            add(d, &mut b, "TXT_KEY_UNIT_LEADER_EXPERIENCE", "[ICON_BULLET]Can Provide up to %d1 XP to Each Unit in the Same Tile", &[Arg::N(*leader_experience)]);
        }
    }
    if let Some(p) = leader_promotion {
        b.push_str(&format!("{NEWLINE}[ICON_BULLET]{}", txt(d, "TXT_KEY_PROMOTION_WHEN_LEADING", "When attached to a unit:", &[])));
        // C++ `parsePromotionHelp(.., L"\n   ")`: the promotion's lines, indented.
        for l in super::promotion::effects(d, *p).split(NEWLINE).filter(|l| !l.is_empty()) {
            b.push_str(&format!("{NEWLINE}[SPACE][SPACE][SPACE]{l}"));
        }
    }
    if *base_discover > 0 || *discover_multiplier > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_DISCOVER_TECH", "[ICON_BULLET]Can Discover a Technology", &[]);
    }
    if *base_hurry > 0 || *hurry_multiplier > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_HURRY_PRODUCTION", "[ICON_BULLET]Can Hurry Production", &[]);
    }
    if *base_trade > 0 || *trade_multiplier > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_TRADE_MISSION", "[ICON_BULLET]Can Conduct a Trade Mission", &[]);
    }
    // C++ scales these two by the game speed's `getUnitGreatWorkPercent`, unknown before the game:
    // as at Normal speed (100%).
    if *great_work_culture > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_GREAT_WORK", "[ICON_BULLET]Can Create a Great Work (%D1_Change[ICON_CULTURE])", &[Arg::N(*great_work_culture)]);
    }
    if *espionage_points > 0 {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_ESPIONAGE_MISSION",
            "[ICON_BULLET]Can Infiltrate another player's City (%D1_Change[ICON_ESPIONAGE])",
            &[Arg::N(*espionage_points)],
        );
    }
    let start = |key: &str, fallback: &str| format!("{NEWLINE}{}", txt(d, key, fallback, &[]));
    let religions: Vec<String> = religion_spreads.iter_enumerated().filter(|(_, &n)| n > 0).map(|(r, _)| display(d, r).into_owned()).collect();
    list(&mut b, &start("TXT_KEY_UNIT_CAN_SPREAD", "[ICON_BULLET]Can Spread[SPACE]"), &religions, ", ");
    let corporations: Vec<String> = corporation_spreads.iter_enumerated().filter(|(_, &n)| n > 0).map(|(c, _)| display(d, c).into_owned()).collect();
    list(&mut b, &start("TXT_KEY_UNIT_CAN_EXPAND", "[ICON_BULLET]Can Expand[SPACE]"), &corporations, ", ");
    let specialists: Vec<String> = great_peoples.iter_enumerated().filter(|(_, &x)| x).map(|(s, _)| display(d, s).into_owned()).collect();
    list(&mut b, &start("TXT_KEY_UNIT_CAN_JOIN", "[ICON_BULLET]Can Join City as[SPACE]"), &specialists, ", ");
    let constructs: Vec<String> = d.buildings.keys().filter(|&bl| buildings[bl] || force_buildings[bl]).map(|bl| display(d, bl).into_owned()).collect();
    list(&mut b, &start("TXT_KEY_UNIT_CAN_CONSTRUCT", "[ICON_BULLET]Can Construct[SPACE]"), &constructs, ", ");
    if *cargo_space > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_CARGO_SPACE", "[ICON_BULLET]Cargo Space: %d1_Num", &[Arg::N(*cargo_space)]);
        if let Some(su) = special_cargo {
            b.push_str(&txt(d, "TXT_KEY_UNIT_CARRIES", "[SPACE](Carries %s1_Name)", &[Arg::S(&display(d, *su))]));
        }
    }
    let until = |what: &str, tech: Option<TechType>| match tech {
        None => what.to_string(),
        Some(t) => txt(d, "TXT_KEY_TERRAIN_UNTIL_TECH", "[LINK=literal]%s1_terrain[\\LINK] (until %s2_technology)", &[Arg::S(what), Arg::S(&display(d, t))]),
    };
    let mut cannot_enter: Vec<String> = terrain_impassable.iter_enumerated().filter(|(_, &x)| x).map(|(t, _)| until(&display(d, t), terrain_passable_tech[t])).collect();
    // C++ reads `getTerrainPassableTech` with the feature's index here (a slip that would name the
    // wrong tech): the feature's own `getFeaturePassableTech`.
    cannot_enter.extend(feature_impassable.iter_enumerated().filter(|(_, &x)| x).map(|(f, _)| until(&display(d, f), feature_passable_tech[f])));
    list(&mut b, &format!("{} ", start("TXT_KEY_UNIT_CANNOT_ENTER", "[ICON_BULLET]Cannot Enter")), &cannot_enter, ", ");
    if *invisible {
        add(d, &mut b, "TXT_KEY_UNIT_INVISIBLE_ALL", "[ICON_BULLET]Invisible to All Units", &[]);
    } else if invisible_type.is_some() {
        add(d, &mut b, "TXT_KEY_UNIT_INVISIBLE_MOST", "[ICON_BULLET]Invisible to Most Units", &[]);
    }
    // Every type it sees, as the Civilopedia lists them (the tooltip skips its own invisible type).
    for &i in see_invisible_types {
        add(d, &mut b, "TXT_KEY_UNIT_SEE_INVISIBLE", "[ICON_BULLET]Can See %s1_Name", &[Arg::S(&display(d, i))]);
    }
    for (has, key, fallback) in [
        (*investigate, "TXT_KEY_UNIT_INVESTIGATE_CITY", "[ICON_BULLET]Can Investigate Rival City"),
        (*can_move_impassable, "TXT_KEY_UNIT_CAN_MOVE_IMPASSABLE", "[ICON_BULLET]Can Move through Impassable Terrain"),
        (*no_bad_goodies, "TXT_KEY_UNIT_NO_BAD_GOODIES", "[ICON_BULLET]Better Results from Tribal Villages"),
        (*hidden_nationality, "TXT_KEY_UNIT_HIDDEN_NATIONALITY", "[ICON_BULLET]Hidden Nationality"),
        (*always_hostile, "TXT_KEY_UNIT_ALWAYS_HOSTILE", "[ICON_BULLET]Can Attack without Declaring War"),
        (*only_defensive, "TXT_KEY_UNIT_ONLY_DEFENSIVE", "[ICON_BULLET]Can Only Defend"),
        (*no_capture, "TXT_KEY_UNIT_CANNOT_CAPTURE", "[ICON_BULLET]Unit cannot capture"),
        (!*pillage, "TXT_KEY_UNIT_CANNOT_PILLAGE", "[ICON_BULLET]Unit cannot pillage"),
        (!*military_happiness, "TXT_KEY_UNIT_CANNOT_PROTECT_CITY", "[ICON_BULLET]Unit doesn't provide military happiness"),
        (*rival_territory, "TXT_KEY_UNIT_EXPLORE_RIVAL", "[ICON_BULLET]Can Explore Rival Territory"),
        (*found, "TXT_KEY_UNIT_FOUND_CITY", "[ICON_BULLET]Can Found a [LINK=CONCEPT_CITIES]New City[\\LINK]"),
    ] {
        if has {
            add(d, &mut b, key, fallback, &[]);
        }
    }
    // The builds: all of them in a word when it has more than three quarters.
    let my_builds: Vec<String> = builds.iter_enumerated().filter(|(_, &x)| x).map(|(x, _)| display(d, x).into_owned()).collect();
    if my_builds.len() > d.builds.len() * 3 / 4 {
        add(d, &mut b, "TXT_KEY_UNIT_IMPROVE_PLOTS", "[ICON_BULLET]Can Improve Tiles", &[]);
    } else {
        list(&mut b, &format!("{} ", start("TXT_KEY_UNIT_CAN", "[ICON_BULLET]Can[SPACE]")), &my_builds, ", ");
    }
    if *nuke_range != -1 {
        add(d, &mut b, "TXT_KEY_UNIT_CAN_NUKE", "[ICON_BULLET]Can [LINK=CONCEPT_NUKES]Nuke[\\LINK] Enemy Lands", &[]);
    }
    if *counter_spy {
        add(d, &mut b, "TXT_KEY_UNIT_EXPOSE_SPIES", "[ICON_BULLET]Helps Thwart Rival Spies", &[]);
    }
    if first_strikes + chance_first_strikes > 0 {
        if *chance_first_strikes == 0 {
            if *first_strikes == 1 {
                add(d, &mut b, "TXT_KEY_UNIT_ONE_FIRST_STRIKE", "[ICON_BULLET]1 First Strike", &[]);
            } else {
                add(d, &mut b, "TXT_KEY_UNIT_NUM_FIRST_STRIKES", "[ICON_BULLET]%d1_Num First Strikes", &[Arg::N(*first_strikes)]);
            }
        } else {
            add(
                d,
                &mut b,
                "TXT_KEY_UNIT_FIRST_STRIKE_CHANCES",
                "[ICON_BULLET]%d1_Num1-%d2_Num2 First Strikes",
                &[Arg::N(*first_strikes), Arg::N(first_strikes + chance_first_strikes)],
            );
        }
    }
    if *first_strike_immune {
        add(d, &mut b, "TXT_KEY_UNIT_IMMUNE_FIRST_STRIKES", "[ICON_BULLET]Immune to First Strikes", &[]);
    }
    // FfH.
    if *defensive_strike_chance > 0 {
        add(
            d,
            &mut b,
            "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_PEDIA",
            "[ICON_BULLET]+%d1% chance to do +%d2% damage to units attacking this unit's stack",
            &[Arg::N(*defensive_strike_chance), Arg::N(*defensive_strike_damage)],
        );
    }
    if *immune_to_defensive_strike {
        add(d, &mut b, "TXT_KEY_PROMOTION_IMMUNE_TO_DEFENSIVE_STRIKE_PEDIA", "[ICON_BULLET]Immune to Defensive Strikes", &[]);
    }
    for (has, key, fallback) in [
        (*no_defensive_bonus, "TXT_KEY_UNIT_NO_DEFENSE_BONUSES", "[ICON_BULLET]Doesn't Receive Defensive Bonuses"),
        (*flat_movement_cost, "TXT_KEY_UNIT_FLAT_MOVEMENT", "[ICON_BULLET]Flat Movement Costs"),
        (*ignore_terrain_cost, "TXT_KEY_UNIT_IGNORE_TERRAIN", "[ICON_BULLET]Ignores Terrain Movement Costs"),
    ] {
        if has {
            add(d, &mut b, key, fallback, &[]);
        }
    }
    for (n, key, fallback) in [
        (*interception_probability, "TXT_KEY_UNIT_INTERCEPT_AIRCRAFT", "[ICON_BULLET]Can Intercept Aircraft (%d1_Amount%% Chance)"),
        (*evasion_probability, "TXT_KEY_UNIT_EVADE_INTERCEPTION", "[ICON_BULLET]Can Evade Interception (%d1_Amount%% Chance)"),
        (*withdrawal_probability, "TXT_KEY_UNIT_WITHDRAWL_PROBABILITY", "[ICON_BULLET]Can Withdraw from Combat (%d1_Amount%% Chance)"),
    ] {
        if n > 0 {
            add(d, &mut b, key, fallback, &[Arg::N(n)]);
        }
    }
    let hp = d.defines.max_hit_points.max(1);
    if *combat_limit < hp && *combat > 0 && !*only_defensive {
        add(d, &mut b, "TXT_KEY_UNIT_COMBAT_LIMIT", "[ICON_BULLET]Maximum %d1%% damage to enemy on attack", &[Arg::N(100 * combat_limit / hp)]);
    }
    if *collateral_damage > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_COLLATERAL_DAMAGE", "[ICON_BULLET]Causes Collateral Damage (max. %d1%%)", &[Arg::N(100 * collateral_damage_limit / hp)]);
    }
    for (uc, &n) in unit_combat_collateral_immune.iter_enumerated() {
        if n != 0 {
            add(d, &mut b, "TXT_KEY_UNIT_COLLATERAL_IMMUNE", "[ICON_BULLET]Immune to collateral damage from [LINK=literal]%s1_units[\\LINK]", &[Arg::S(&display(d, uc))]);
        }
    }
    // A modifier for attack and defence: one "Strength" line when they are equal, else each.
    let strength_or_each = |b: &mut String, attack: i32, defense: i32, both: (&str, &str), att: (&str, &str), def: (&str, &str), what: &str, defense_first: bool| {
        if attack == defense {
            if attack != 0 {
                add(d, b, both.0, both.1, &[Arg::N(attack), Arg::S(what)]);
            }
            return;
        }
        let mut lines = [(attack, att), (defense, def)];
        if defense_first {
            lines.swap(0, 1);
        }
        for (n, (key, fallback)) in lines {
            if n != 0 {
                add(d, b, key, fallback, &[Arg::N(n), Arg::S(what)]);
            }
        }
    };
    strength_or_each(
        &mut b,
        *city_attack_modifier,
        *city_defense_modifier,
        ("TXT_KEY_UNIT_CITY_STRENGTH_MOD", "[ICON_BULLET]%D1_Change%% City Strength"),
        ("TXT_KEY_UNIT_CITY_ATTACK_MOD", "[ICON_BULLET]%D1_Change%% City Attack"),
        ("TXT_KEY_UNIT_CITY_DEFENSE_MOD", "[ICON_BULLET]%D1_Change%% City Defense"),
        "",
        false,
    );
    if *animal_combat_modifier != 0 {
        add(d, &mut b, "TXT_KEY_UNIT_ANIMAL_COMBAT_MOD", "[ICON_BULLET]%D1_Change%% vs. Animals", &[Arg::N(*animal_combat_modifier)]);
    }
    if *drop_range > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_PARADROP_RANGE", "[ICON_BULLET]Can perform paradrops (Range=%d1_range)", &[Arg::N(*drop_range)]);
    }
    strength_or_each(
        &mut b,
        *hills_attack_modifier,
        *hills_defense_modifier,
        ("TXT_KEY_UNIT_HILLS_STRENGTH", "[ICON_BULLET]%D1_Change%% Hills Strength"),
        ("TXT_KEY_UNIT_HILLS_ATTACK", "[ICON_BULLET]%D1_Change%% Hills Attack"),
        ("TXT_KEY_UNIT_HILLS_DEFENSE", "[ICON_BULLET]%D1_Change%% Hills Defense"),
        "",
        false,
    );
    let terrain_keys = (
        ("TXT_KEY_UNIT_STRENGTH", "[ICON_BULLET]%D1_Change%% %s2_TerrOrFeat Strength"),
        ("TXT_KEY_UNIT_ATTACK", "[ICON_BULLET]%D1_Change%% %s2_TerrOrFeat Attack"),
        ("TXT_KEY_UNIT_DEFENSE", "[ICON_BULLET]%D1_Change%% %s2_TerrOrFeat Defense"),
    );
    // C++ writes the defence before the attack for terrains and features.
    for t in d.terrains.keys() {
        strength_or_each(&mut b, terrain_attack_modifier[t], terrain_defense_modifier[t], terrain_keys.0, terrain_keys.1, terrain_keys.2, &display(d, t), true);
    }
    // C++ names the terrain of the feature's index in the "Strength" line (a slip that would name
    // the wrong place): the feature.
    for f in d.features.keys() {
        strength_or_each(&mut b, feature_attack_modifier[f], feature_defense_modifier[f], terrain_keys.0, terrain_keys.1, terrain_keys.2, &display(d, f), true);
    }
    for c in d.unit_classes.keys() {
        strength_or_each(
            &mut b,
            unit_class_attack_modifier[c],
            unit_class_defense_modifier[c],
            ("TXT_KEY_UNIT_MOD_VS_TYPE", "[ICON_BULLET]%D1_Change%% vs. [LINK=literal]%s2_Type[\\LINK]"),
            ("TXT_KEY_UNIT_ATTACK_MOD_VS_CLASS", "[ICON_BULLET]%D1_Change%% Attack vs. [LINK=literal]%s2_Class[\\LINK]"),
            ("TXT_KEY_UNIT_DEFENSE_MOD_VS_CLASS", "[ICON_BULLET]%D1_Change%% Defense vs. [LINK=literal]%s2_Type[\\LINK]"),
            &display(d, c),
            false,
        );
    }
    for (uc, &n) in unit_combat_modifier.iter_enumerated() {
        if n != 0 {
            add(d, &mut b, "TXT_KEY_UNIT_MOD_VS_TYPE", "[ICON_BULLET]%D1_Change%% vs. [LINK=literal]%s2_Type[\\LINK]", &[Arg::N(n), Arg::S(&display(d, uc))]);
        }
    }
    for (dom, &n) in domain_modifier.iter_enumerated() {
        if n != 0 {
            add(d, &mut b, "TXT_KEY_UNIT_MOD_VS_TYPE_NO_LINK", "[ICON_BULLET]%D1_Change%% vs. %s2_Type", &[Arg::N(n), Arg::S(&display(d, dom))]);
        }
    }
    // The classes and unit combats it attacks or defends first, and flanks.
    let targets = "[ICON_BULLET]Targets any %s1_unit_list first in combat outside cities";
    let defends = "[ICON_BULLET]Defends first against %s1_unit_list";
    let class_list = |of: &IdxVec<UnitClassType, bool>| -> String { d.unit_classes.keys().filter(|&c| of[c]).map(|c| display(d, c).into_owned()).collect::<Vec<_>>().join(", ") };
    let combat_list = |of: &IdxVec<UnitCombatType, bool>| -> String { d.unit_combats.keys().filter(|&c| of[c]).map(|c| display(d, c).into_owned()).collect::<Vec<_>>().join(", ") };
    let flanks = d.unit_classes.keys().filter(|&c| flanking_strike_unit_class[c] > 0).map(|c| display(d, c).into_owned()).collect::<Vec<_>>().join(", ");
    for (names, key, fallback) in [
        (class_list(target_unit_class), "TXT_KEY_UNIT_TARGETS_UNIT_FIRST", targets),
        (class_list(defender_unit_class), "TXT_KEY_UNIT_DEFENDS_UNIT_FIRST", defends),
        (combat_list(target_unit_combat), "TXT_KEY_UNIT_TARGETS_UNIT_FIRST", targets),
        (combat_list(defender_unit_combat), "TXT_KEY_UNIT_DEFENDS_UNIT_FIRST", defends),
        (flanks, "TXT_KEY_UNIT_FLANKING_STRIKES", "[ICON_BULLET][LINK=CONCEPT_FLANK_ATTACK]Flank attack[\\LINK] against %s1_unitlist"),
    ] {
        if !names.is_empty() {
            add(d, &mut b, key, fallback, &[Arg::S(&names)]);
        }
    }
    let city_damage = d.defines.max_city_defense_damage.max(1);
    if *bomb_rate > 0 {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_BOMB_RATE",
            "[ICON_BULLET]Can Destroy Tile Improvements and Bomb City Defenses (-%d1_Mod%%/Turn)",
            &[Arg::N(bomb_rate * 100 / city_damage)],
        );
    }
    if *bombard_rate > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_BOMBARD_RATE", "[ICON_BULLET]Can Bombard City Defenses (-%d1_Mod%%/Turn)", &[Arg::N(bombard_rate * 100 / city_damage)]);
    }
    let promotions: Vec<String> = free_promotions.iter_enumerated().filter(|(_, &x)| x).map(|(p, _)| display(d, p).into_owned()).collect();
    list(&mut b, &start("TXT_KEY_UNIT_STARTS_WITH", "[ICON_BULLET]Starts with[SPACE]"), &promotions, ", ");
    if *extra_cost != 0 {
        add(d, &mut b, "TXT_KEY_UNIT_EXTRA_COST", "[ICON_BULLET]%D1_Change[ICON_GOLD] Cost", &[Arg::N(*extra_cost)]);
    }
    // The Civilopedia's: the leader traits that build it faster.
    for (t, &n) in production_traits.iter_enumerated() {
        if n == 100 {
            add(d, &mut b, "TXT_KEY_DOUBLE_SPEED_TRAIT", "[ICON_BULLET]Double production speed for %s1_trait leaders", &[Arg::S(&display(d, t))]);
        } else if n != 0 {
            add(d, &mut b, "TXT_KEY_PRODUCTION_MODIFIER_TRAIT", "[ICON_BULLET]%D1_change%% production speed for %s2_trait leaders", &[Arg::N(n), Arg::S(&display(d, t))]);
        }
    }
    // FfH.
    for (dt, &n) in damage_type_combat.iter_enumerated() {
        if n != 0 {
            add(d, &mut b, "TXT_KEY_DAMAGE_TYPE_COMBAT", "[ICON_BULLET]%D1 %s2 combat", &[Arg::N(n), Arg::S(&display(d, dt))]);
        }
    }
    for (bonus, &n) in bonus_affinity.iter_enumerated() {
        if n != 0 {
            add(d, &mut b, "TXT_KEY_BONUS_AFFINITY", "[ICON_BULLET]%D1: Affinity for %s2", &[Arg::N(n), Arg::S(&display(d, bonus))]);
        }
    }
    if *min_level > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_CANT_BUILD", "[ICON_BULLET]Unit cannot be built", &[]);
        add(d, &mut b, "TXT_KEY_UNIT_MIN_LEVEL", "[ICON_BULLET]Must be upgraded from a unit of level %D1 or higher", &[Arg::N(*min_level)]);
    }
    if *disable_upgrade_to {
        add(d, &mut b, "TXT_KEY_UNIT_DISABLE_UPGRADE_TO", "[ICON_BULLET]Cannot Upgrade to this Unit Type", &[]);
    }
    if *free_promotion_pick > 0 {
        add(d, &mut b, "TXT_KEY_UNIT_FREE_PROMOTION_PICK", "[ICON_BULLET]Unit starts with %D1 free promotion(s)", &[Arg::N(*free_promotion_pick)]);
    }
    if *gold_from_combat != 0 {
        add(d, &mut b, "TXT_KEY_UNIT_GOLD_FROM_COMBAT", "[ICON_BULLET]Combat victories produce %D1 gold", &[Arg::N(*gold_from_combat)]);
    }
    if *modify_global_counter != 0 {
        add(d, &mut b, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER", "[ICON_BULLET]%D1 to the Armageddon Counter", &[Arg::N(*modify_global_counter)]);
    }
    if let Some(x) = unit_convert_from_combat {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_CONVERT_FROM_COMBAT",
            "[ICON_BULLET]%D1% chance of becoming a %s2_unit if successful in combat",
            &[Arg::N(*unit_convert_from_combat_chance), Arg::S(&display(d, *x))],
        );
    }
    if let Some(x) = unit_create_from_combat {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_CREATE_FROM_COMBAT",
            "[ICON_BULLET]%D1% chance of creating a %s2_unit if successful in combat",
            &[Arg::N(*unit_create_from_combat_chance), Arg::S(&display(d, *x))],
        );
    }
    if *enslavement_chance != 0 {
        add(d, &mut b, "TXT_KEY_UNIT_ENSLAVEMENT_CHANCE", "[ICON_BULLET]%D1% chance to create a slave from combat", &[Arg::N(*enslavement_chance)]);
    }
    if let Some(p) = promotion_from_combat {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_PROMOTION_FROM_COMBAT",
            "[ICON_BULLET]Gains the %s1_promotion promotion after defeating a living unit in combat",
            &[Arg::S(&display(d, *p))],
        );
    }
    if *weapon_tier > 0 {
        // C++ quirk: the line break comes first, so a tier above 3 (none in FfH2) leaves an empty line.
        b.push_str(NEWLINE);
        match weapon_tier {
            1 => b.push_str(&txt(d, "TXT_KEY_UNIT_WEAPON_TIER_1", "[ICON_BULLET]Unit can use Bronze weapons.", &[])),
            2 => b.push_str(&txt(d, "TXT_KEY_UNIT_WEAPON_TIER_2", "[ICON_BULLET]Unit can use Bronze and Iron weapons.", &[])),
            3 => b.push_str(&txt(d, "TXT_KEY_UNIT_WEAPON_TIER_3", "[ICON_BULLET]Unit can use Bronze, Iron and Mithril weapons.", &[])),
            _ => {}
        }
    }
    if *immortal {
        add(d, &mut b, "TXT_KEY_PROMOTION_IMMORTAL_PEDIA", "[ICON_BULLET]Resurrected in the civilization's capital if killed", &[]);
    }
    if *no_war_weariness {
        add(d, &mut b, "TXT_KEY_UNIT_NO_WAR_WEARINESS", "[ICON_BULLET]Death won't cause War Weariness", &[]);
    }
    if let Some(v) = diplo_vote_type {
        add(d, &mut b, "TXT_KEY_UNIT_DIPLO_VOTE_TYPE", "[ICON_BULLET]+1 vote in the %s1", &[Arg::S(&display(d, *v))]);
    }
    if *abandon {
        add(d, &mut b, "TXT_KEY_UNIT_ABANDON", "[ICON_BULLET]Unit will abandon civilizations that don't maintain its Civic or Religion requirements", &[]);
    }
    // Its own spells: those of the unit or its class, without a civilization or religion
    // prerequisite (C++ checks the two independently; no FfH2 spell has both).
    let ability = |b: &mut String, s: SpellType| add(d, b, "TXT_KEY_UNIT_SPELL_ABILITY", "[ICON_BULLET]Unit can use the [LINK=literal]%s1_spell[\\LINK] ability", &[Arg::S(&display(d, s))]);
    let of_class = |s: &SpellInfo| s.unit_class_prereq.is_some() && s.unit_class_prereq == class;
    let listed = |s: &SpellInfo| s.civilization_prereq.is_none() && s.religion_prereq.is_none() && (s.unit_prereq == Some(u) || of_class(s));
    for s in d.spells.keys() {
        let si = &d.spells[s];
        if si.civilization_prereq.is_none() && si.religion_prereq.is_none() {
            if si.unit_prereq == Some(u) {
                ability(&mut b, s);
            }
            if of_class(si) {
                ability(&mut b, s);
            }
        }
    }
    // ffh-play's: the other spells a new unit of this type can cast (CvUnit::canCast's checks of the
    // caster): through its free promotions, or its own with a prerequisite civilization (one it
    // belongs to) or religion (its own). What the spell needs of the place, the player and the
    // game is the spell's help.
    let has = |p: Option<PromotionType>| p.is_none_or(|p| free_promotions[p]);
    for s in d.spells.keys() {
        let si = &d.spells[s];
        let tied = si.promotion_prereq1.is_some_and(|p| free_promotions[p]) || si.unit_prereq == Some(u) || of_class(si);
        let caster = has(si.promotion_prereq1)
            && has(si.promotion_prereq2)
            && si.unit_prereq.is_none_or(|x| x == u)
            && si.unit_class_prereq.is_none_or(|c| Some(c) == class)
            && si.unit_combat_prereq.is_none_or(|c| Some(c) == *unit_combat_type)
            && si.civilization_prereq.is_none_or(|c| civs.contains(&c))
            && si.religion_prereq.is_none_or(|r| Some(r) == *religion_type)
            // An equipment casts only the spells of its class.
            && (equipment_promotion.is_none() || of_class(si));
        if tied && caster && !listed(si) {
            ability(&mut b, s);
        }
    }
    // ffh-play's: what an equipment is when a unit carries it.
    if let Some(p) = equipment_promotion {
        b.push_str(NEWLINE);
        b.push_str(&super::fill("[ICON_BULLET]Equipment: carried as the [LINK=literal]%s1[\\LINK] promotion", &[Arg::S(&display(d, *p))]));
    }
    // ffh-play's: the units it upgrades to (FfH2's pedia panel "Upgrades To": the classes' default
    // units, none for a world unit, not those that cannot be upgraded to).
    if !class.is_some_and(|c| d.unit_classes[c].max_global_instances != -1) {
        let upgrades: Vec<String> = upgrade_unit_class
            .iter_enumerated()
            .filter(|(_, &x)| x)
            .filter_map(|(c, _)| d.unit_classes[c].default_unit_index)
            .filter(|&x| !d.units[x].disable_upgrade_to)
            .map(|x| display(d, x).into_owned())
            .collect();
        list(&mut b, &format!("{NEWLINE}[ICON_BULLET]{}: ", txt(d, "TXT_KEY_PEDIA_UPGRADES_TO", "Upgrades To", &[])), &upgrades, ", ");
    }

    // ---- setUnitHelp: what it requires (without a city, all of it) ----
    if let Some(r) = holy_city {
        // `%F1` is the religion's font icon: its name stands in for it.
        add(d, &mut b, "TXT_KEY_UNIT_REQUIRES_HOLY_CITY", "[COLOR_WARNING_TEXT]Requires %F1_RelIcon Holy City[COLOR_REVERT]", &[Arg::S(&display(d, *r))]);
    }
    if let Some(su) = special_unit_type {
        let projects: Vec<String> = d.projects.keys().filter(|&p| d.projects[p].everyone_special_unit == Some(*su)).map(|p| display(d, p).into_owned()).collect();
        requires_list(d, &mut b, &projects, "TXT_KEY_OR", "[SPACE]or[SPACE]");
    }
    if *nuke_range != -1 {
        let mut allow: Vec<String> = d.projects.keys().filter(|&p| d.projects[p].allows_nukes).map(|p| display(d, p).into_owned()).collect();
        allow.extend(d.buildings.keys().filter(|&bl| d.buildings[bl].allows_nukes).map(|bl| display(d, bl).into_owned()));
        requires_list(d, &mut b, &allow, "TXT_KEY_OR", "[SPACE]or[SPACE]");
    }
    if let Some(bl) = prereq_building {
        requires(d, &mut b, &display(d, *bl));
    }
    // FfH.
    if let Some(bc) = prereq_building_class {
        requires(d, &mut b, &display(d, *bc));
    }
    if let Some(a) = prereq_alignment {
        add(d, &mut b, "TXT_KEY_UNIT_PREREQ_ALIGNMENT", "[COLOR_WARNING_TEXT]Requires %s1 alignment[COLOR_REVERT]", &[Arg::S(&display(d, *a))]);
    }
    if let Some(c) = prereq_civic {
        requires(d, &mut b, &display(d, *c));
    }
    if *prereq_global_counter != 0 {
        add(
            d,
            &mut b,
            "TXT_KEY_UNIT_PREREQ_GLOBAL_COUNTER",
            "[COLOR_WARNING_TEXT]Requires an Armageddon Count of %D1 or higher[COLOR_REVERT]",
            &[Arg::N(*prereq_global_counter)],
        );
    }
    // ffh-play's: the religion of the city (CvPlot::canTrain) and the player's state religion
    // (CvPlayer::canTrain); FfH2's pedia shows the first in its "Requires" panel.
    if let Some(r) = prereq_religion {
        requires(d, &mut b, &display(d, *r));
    }
    if let Some(r) = state_religion {
        b.push_str(NEWLINE);
        b.push_str(&color("COLOR_WARNING_TEXT", &txt(d, "TXT_KEY_TECH_PREREQ_RELIGION", "Requires: %s1_religion state religion", &[Arg::S(&display(d, *r))])));
    }
    if let Some(t) = prereq_and_tech {
        requires(d, &mut b, &display(d, *t));
    }
    let techs: Vec<String> = prereq_and_techs.iter().map(|&t| display(d, t).into_owned()).collect();
    requires_list(d, &mut b, &techs, "TXT_KEY_AND", "[SPACE]and[SPACE]");
    if let Some(bonus) = prereq_and_bonus {
        requires(d, &mut b, &display(d, *bonus));
    }
    let bonuses: Vec<String> = prereq_or_bonuses.iter().map(|&x| display(d, x).into_owned()).collect();
    requires_list(d, &mut b, &bonuses, "TXT_KEY_OR", "[SPACE]or[SPACE]");

    // The resources that speed it up, as the Civilopedia writes them (the tooltip puts them after
    // its cost line).
    for (bonus, &n) in bonus_production_modifier.iter_enumerated() {
        if n == 0 {
            continue;
        }
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]");
        b.push_str(&if n == 100 {
            // BtS's English leaves the link open: closed here.
            txt(d, "TXT_KEY_UNIT_DOUBLE_SPEED", "Double Production Speed with [LINK=literal]%s1_Bonus[\\LINK]", &[Arg::S(&display(d, bonus))])
        } else {
            txt(d, "TXT_KEY_UNIT_BUILDS_FASTER", "Builds %d1_Change%% Faster with [LINK=literal]%s2_Bonus[\\LINK]", &[Arg::N(n), Arg::S(&display(d, bonus))])
        });
    }
    // The Civilopedia's: the other civilizations' units of its class, when it is the class's default.
    // C++ also names the graphical-only ones, which the Civilopedia hides (the barbarians' Scorpion
    // Clan copies: "Archer" replaced by "Archer"): left out.
    if default == Some(u) {
        for x in d.units.keys().filter(|&x| x != u && d.units[x].unit_class_type == class && !d.units[x].base.graphical_only) {
            add(d, &mut b, "TXT_KEY_REPLACED_BY_UNIT", "Replaced by [LINK=literal]%s1_default_unit[\\LINK]", &[Arg::S(&display(d, x))]);
        }
    }
    // The XML help (no FfH2 unit has one).
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

    fn help(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<UnitType>(name))
    }

    /// Whether help `h` holds each of `parts`, reading `[SPACE]` as a space.
    fn has(h: &str, parts: &[&str]) {
        let h = h.replace("[SPACE]", " ");
        for part in parts {
            assert!(h.contains(part), "{part:?} missing from: {h}");
        }
    }

    #[test]
    fn melee_units_say_their_strength_and_bonuses() {
        let w = help("UNIT_WARRIOR");
        assert!(w.starts_with("3[ICON_STRENGTH], 1[ICON_MOVES] (Melee Units)"), "{w}");
        has(&w, &["[ICON_BULLET]+25% City Defense", "[ICON_BULLET]Unit can use Bronze weapons.", "Upgrades To: ", "Axeman", "Replaced by"]);
        let a = help("UNIT_AXEMAN");
        assert!(a.starts_with("4[ICON_STRENGTH], 1[ICON_MOVES] (Melee Units)"), "{a}");
        has(&a, &["[ICON_BULLET]+10% City Attack", "[ICON_BULLET]Unit can use Bronze, Iron and Mithril weapons.", "Requires Bronze Working", "Requires Training Yard"]);
        let c = help("UNIT_CHAMPION");
        has(&c, &["[ICON_BULLET]+25% vs. [LINK=literal]Melee Units[\\LINK]", "Requires Iron Working"]);
        // FfH's defence strength; not "replaced by" its graphical-only barbarian copy.
        let archer = help("UNIT_ARCHER");
        assert!(archer.starts_with("3/5[ICON_STRENGTH], 1[ICON_MOVES] (Archery Units)"), "{archer}");
        has(&archer, &["Replaced by [LINK=literal]Dwarven Slinger[\\LINK]"]);
        assert!(!archer.contains("Replaced by [LINK=literal]Archer[\\LINK]"), "{archer}");
        assert!(help("UNIT_MUD_GOLEM").starts_with("0/2[ICON_STRENGTH]"), "{}", help("UNIT_MUD_GOLEM"));
    }

    #[test]
    fn workers_scouts_and_great_people_say_what_they_can_do() {
        // 15 of the 35 builds: listed (more than three quarters would be "Can Improve Tiles").
        let w = help("UNIT_WORKER");
        assert!(w.starts_with("2[ICON_MOVES]"), "{w}");
        has(
            &w,
            &[
                "[ICON_BULLET]Can  Build a [LINK=CONCEPT_MOVEMENT]Road[\\LINK], Build a [LINK=IMPROVEMENT_FARM]Farm[\\LINK], ",
                "[ICON_BULLET]Unit cannot pillage",
                "[ICON_BULLET]Unit doesn't provide military happiness",
                // The Luchuirp's worker.
                "Replaced by [LINK=literal]Mud Golem[\\LINK]",
            ],
        );
        assert!(!w.contains("Can Improve Tiles"), "{w}");
        let s = help("UNIT_SCOUT");
        assert!(s.starts_with("2[ICON_STRENGTH], 2[ICON_MOVES] (Recon Units)"), "{s}");
        has(&s, &["[ICON_BULLET]Better Results from Tribal Villages", "Upgrades To: "]);
        let p = help("UNIT_PROPHET");
        has(
            &p,
            &[
                "[ICON_BULLET]Can Start a Golden Age",
                "[ICON_BULLET]Can Discover a Technology",
                "[ICON_BULLET]Can Join City as ",
                "[ICON_BULLET]Can Construct ",
                "[ICON_BULLET]Can Only Defend",
            ],
        );
        let settler = help("UNIT_SETTLER");
        has(&settler, &["[ICON_BULLET]Can Found a [LINK=CONCEPT_CITIES]New City[\\LINK]", "production speed for"]);
    }

    #[test]
    fn heroes_ships_and_casters_say_their_ffh_abilities() {
        let d = ffh_data::global();
        let donal = help("UNIT_DONAL");
        has(&donal, &["Unique Unit for", "Bannor", "[ICON_BULLET]World Unit (", "[ICON_BULLET]Starts with ", "Hero", "Requires Fanaticism"]);
        // The spells of its free promotions (Spirit II: Hope; Recruiter: Recruit).
        has(&donal, &[&format!("[LINK=literal]{}[\\LINK] ability", display(d, d.t::<SpellType>("SPELL_HOPE")))]);
        let galley = help("UNIT_GALLEY");
        assert!(galley.starts_with("5[ICON_STRENGTH], 3[ICON_MOVES] (Naval Units)"), "{galley}");
        has(&galley, &["[ICON_BULLET]Cannot Enter Ocean"]);
        let barge = help("UNIT_ARCANE_BARGE");
        has(&barge, &["[ICON_BULLET]Cargo Space: 1", &format!("[LINK=literal]{}[\\LINK] ability", display(d, d.t::<SpellType>("SPELL_FIREBALL"))), "Requires Astronomy", "Requires Sorcery"]);
        let adept = help("UNIT_ADEPT");
        has(&adept, &["[ICON_BULLET]Unit starts with +1 free promotion(s)", "Upgrades To: Mage", "Requires Knowledge of the Ether"]);
        // The Malakim's Summon Sand Lion is not every mage's.
        let sand_lion = format!("[LINK=literal]{}[\\LINK] ability", display(d, d.t::<SpellType>("SPELL_SUMMON_SAND_LION")));
        assert!(!help("UNIT_MAGE").contains(&sand_lion), "{}", help("UNIT_MAGE"));
        let archmage = help("UNIT_ARCHMAGE");
        has(&archmage, &["[ICON_BULLET]Unit cannot be built", "[ICON_BULLET]Must be upgraded from a unit of level +6 or higher"]);
        // A priest: its religion's spells only, and what it requires.
        let priest = help("UNIT_PRIEST_OF_LEAVES");
        let leaves = display(d, d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES"));
        has(&priest, &[&format!("Requires {leaves}"), &format!("Requires: {leaves} state religion"), &format!("[LINK=literal]{}[\\LINK] ability", display(d, d.t::<SpellType>("SPELL_BLOOM")))]);
        assert!(!priest.contains(&format!("[LINK=literal]{}[\\LINK] ability", display(d, d.t::<SpellType>("SPELL_BLESS")))), "{priest}");
        has(&help("UNIT_DRUID"), &["Requires Neutral alignment"]);
        has(&help("EQUIPMENT_ATHAME"), &["[ICON_BULLET]Equipment: carried as the "]);
        // A unique unit of one civilization (not of the setup's "Random" one), and the
        // `PrereqCiv` of a class open to all.
        let winter = help("UNIT_HIGH_PRIEST_OF_WINTER");
        let illians = display(d, d.t::<CivilizationType>("CIVILIZATION_ILLIANS"));
        assert_eq!(winter.matches("Unique Unit for").count(), 1, "{winter}");
        has(&winter, &[&format!("Unique Unit for [LINK=literal]{illians}[\\LINK]")]);
        has(&help("UNIT_PRIEST_OF_WINTER"), &[&format!("Unique Unit for [LINK=literal]{illians}[\\LINK]")]);
    }

    #[test]
    fn every_unit_has_its_line_and_no_unfilled_placeholder() {
        let d = ffh_data::global();
        for u in d.units.keys() {
            let h = effects(d, u);
            let name = crate::names::name(d, u);
            assert!(h.contains("[ICON_MOVES]"), "{name}: {h}");
            let b = h.as_bytes();
            assert!(!b.windows(3).any(|w| w[0] == b'%' && matches!(w[1], b's' | b'd' | b'D' | b'F') && w[2].is_ascii_digit()), "{name}: {h}");
            assert!(!h.contains("TXT_KEY_") && !h.contains("[NUM"), "{name}: {h}");
            assert!(!h.contains("[NEWLINE][NEWLINE]") && !h.ends_with(NEWLINE), "{name}: {h}");
            // Links and colours are closed.
            assert_eq!(h.matches("[LINK=").count(), h.matches("[\\LINK]").count(), "{name}: {h}");
            assert_eq!(h.matches("[COLOR_").count() - h.matches("[COLOR_REVERT]").count(), h.matches("[COLOR_REVERT]").count(), "{name}: {h}");
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        for u in ["UNIT_WARRIOR", "UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_WORKER", "UNIT_SCOUT", "UNIT_PROPHET", "UNIT_DONAL", "UNIT_GALLEY", "UNIT_ADEPT", "UNIT_ARCANE_BARGE", "UNIT_PRIEST_OF_LEAVES", "UNIT_ARCHMAGE", "UNIT_ANGEL", "EQUIPMENT_ATHAME", "UNIT_ACHERON"] {
            println!("== {u}\n{}\n", help(u).replace("[NEWLINE]", "\n"));
        }
    }
}
