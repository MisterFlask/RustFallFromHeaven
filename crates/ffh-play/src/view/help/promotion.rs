//! What a promotion does: C++ `CvGameTextMgr::parsePromotionHelp` (its `pcNewline`, each line's
//! start, is the help's line break here, as in the pedia), then what it takes to choose it on a
//! level-up, which FFH2's help leaves to the pedia's buttons (Python
//! `CvPediaPromotion.placePrereqs` and `placeUnitGroups`; the rules: C++
//! `CvUnit::canAcquirePromotion`).

use ffh_data::*;

use super::{finish, req, txt, Arg, NEWLINE};
use crate::names::{self, display};

/// One line of the help: C++ `szBuffer.append(pcNewline); szBuffer.append(gDLL->getText(..))`.
fn line(b: &mut String, d: &GameData, key: &str, fallback: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&txt(d, key, fallback, args));
}

/// A requirement line: `line` without the warning colour (`super::unwarned`).
fn line_req(b: &mut String, d: &GameData, key: &str, fallback: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&req(d, key, fallback, args));
}

/// What promotion `p` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say). Every field of the promotion is either described here or named as not shown, so that a
/// new field cannot go unnoticed (the build fails until it is listed).
pub fn effects(d: &GameData, p: PromotionType) -> String {
    let PromotionInfo {
        base,
        prereq_promotion,
        prereq_or_promotion1,
        prereq_or_promotion2,
        tech_prereq,
        state_religion_prereq,
        visibility_change,
        moves_change,
        move_discount_change,
        air_range_change,
        intercept_change,
        evasion_change,
        withdrawal_change,
        cargo_change,
        collateral_damage_change,
        bombard_rate_change,
        first_strikes_change,
        chance_first_strikes_change,
        enemy_heal_change,
        neutral_heal_change,
        friendly_heal_change,
        same_tile_heal_change,
        adjacent_tile_heal_change,
        combat_percent,
        city_attack_percent,
        city_defense_percent,
        hills_attack_percent,
        hills_defense_percent,
        revolt_protection,
        collateral_damage_protection,
        pillage_change,
        upgrade_discount,
        experience_percent,
        kamikaze_percent,
        // The Warlords Great General's promotion (it also hides the promotion from the pedia);
        // FfH2 has none.
        leader: _,
        blitz,
        amphib,
        river,
        enemy_route,
        always_heal,
        hills_double_move,
        immune_to_first_strikes,
        terrain_attack_percent,
        terrain_defense_percent,
        feature_attack_percent,
        feature_defense_percent,
        unit_combat_modifier_percent,
        domain_modifier_percent,
        terrain_double_move,
        feature_double_move,
        unit_combat,
        // ---- FfH ----
        // Read by the rules only, never described by FFH2: the unit is played by the AI
        // (Enraged: its XML help says so), cannot attack (Charmed: likewise), Dispel Magic
        // removes it, a mutation may grant it, an upgrade to a unit combat it is not for drops
        // it, the strength the Armageddon Counter gives (Stigmata: its XML help says so).
        ai_control: _,
        only_defensive: _,
        dispellable: _,
        mutation: _,
        validate: _,
        combat_percent_global_counter: _,
        // Never applied by the rules (C++ reads it, no unit uses it; the AI weighs it).
        combat_percent_defense: _,
        // Equipment and races: no unit chooses them (no requirements below).
        equipment,
        race,
        // Only a living unit may have it (a requirement below).
        prereq_alive,
        boarding,
        double_fortify_bonus,
        fear,
        flying,
        held,
        hidden_nationality,
        ignore_building_defense,
        immortal,
        immune_to_capture,
        immune_to_defensive_strike,
        immune_to_fear,
        immune_to_magic,
        invisible,
        not_alive,
        removed_by_casting,
        removed_by_combat,
        removed_when_healed,
        see_invisible,
        target_weakest_unit,
        target_weakest_unit_counter,
        twincast,
        water_walking,
        // AI weight.
        ai_weight: _,
        betrayal_chance,
        better_defender_than_percent,
        caster_resist_modify,
        combat_capture_percent,
        combat_heal_percent,
        combat_limit,
        combat_percent_in_borders,
        defensive_strike_chance,
        defensive_strike_damage,
        expire_chance,
        extra_combat_str,
        extra_combat_defense,
        free_xp_per_turn,
        free_xp_from_combat,
        gold_from_combat,
        min_level,
        modify_global_counter,
        modify_global_counter_on_combat,
        resist_magic,
        spell_caster_xp,
        spell_damage_modify,
        work_rate_modify,
        capture_unit_combat,
        promotion_combat_apply,
        promotion_immune1,
        promotion_immune2,
        promotion_immune3,
        promotion_random_apply,
        promotion_summon_perk,
        bonus_prereq,
        promotion_prereq_or3,
        promotion_prereq_or4,
        promotion_prereq_and,
        // UI: the unit panel shows only the highest of a line of promotions (Python
        // `CvMainInterface`).
        promotion_next_level: _,
        // A Python callback (Crown of Brilliance): its XML help says what it does.
        py_per_turn: _,
        promotion_combat_type,
        promotion_combat_mod,
        bonus_affinity,
        damage_type_combat,
        damage_type_resist,
    } = &d.promotions[p];
    let b = &mut String::new();

    for (has, key, fallback) in [
        (*blitz, "TXT_KEY_PROMOTION_BLITZ_TEXT", "[ICON_BULLET]Can Attack Multiple Times per Turn"),
        (*amphib, "TXT_KEY_PROMOTION_AMPHIB_TEXT", "[ICON_BULLET]No Combat Penalty for Attacking from Sea"),
        (*river, "TXT_KEY_PROMOTION_RIVER_ATTACK_TEXT", "[ICON_BULLET]No Combat Penalty for Crossing River"),
        (*enemy_route, "TXT_KEY_PROMOTION_ENEMY_ROADS_TEXT", "[ICON_BULLET]Can Use Enemy Roads"),
        (*always_heal, "TXT_KEY_PROMOTION_ALWAYS_HEAL_TEXT", "[ICON_BULLET]Can Heal while Moving"),
        (*hills_double_move, "TXT_KEY_PROMOTION_HILLS_MOVE_TEXT", "[ICON_BULLET]Double Movement in Hills"),
        (*immune_to_first_strikes, "TXT_KEY_PROMOTION_IMMUNE_FIRST_STRIKES_TEXT", "[ICON_BULLET]Immune to First Strikes"),
    ] {
        if has {
            line(b, d, key, fallback, &[]);
        }
    }
    const DOUBLE_MOVE: &str = "[ICON_BULLET]Double Movement in %s1_TerrFeatType";
    for t in d.terrains.keys().filter(|&t| terrain_double_move[t]) {
        line(b, d, "TXT_KEY_PROMOTION_DOUBLE_MOVE_TEXT", DOUBLE_MOVE, &[Arg::S(&display(d, t))]);
    }
    for f in d.features.keys().filter(|&f| feature_double_move[f]) {
        line(b, d, "TXT_KEY_PROMOTION_DOUBLE_MOVE_TEXT", DOUBLE_MOVE, &[Arg::S(&display(d, f))]);
    }
    for (n, key, fallback) in [
        (*visibility_change, "TXT_KEY_PROMOTION_VISIBILITY_TEXT", "[ICON_BULLET]%D1_Change Visibility Range"),
        (*moves_change, "TXT_KEY_PROMOTION_MOVE_TEXT", "[ICON_BULLET]%D1_Change Movement Range"),
        // C++ passes the discount negated: a change of the cost.
        (-move_discount_change, "TXT_KEY_PROMOTION_MOVE_DISCOUNT_TEXT", "[ICON_BULLET]%D1_Discount Terrain Movement Cost"),
        (*air_range_change, "TXT_KEY_PROMOTION_AIR_RANGE_TEXT", "[ICON_BULLET]%D1_Change Operational Range"),
        (*intercept_change, "TXT_KEY_PROMOTION_INTERCEPT_TEXT", "[ICON_BULLET]%D1_Change%% Interception Chance"),
        (*evasion_change, "TXT_KEY_PROMOTION_EVASION_TEXT", "[ICON_BULLET]%D1_Change%% Evasion Chance"),
        (*withdrawal_change, "TXT_KEY_PROMOTION_WITHDRAWAL_TEXT", "[ICON_BULLET]%D1_Change%% Withdrawal Chance"),
        (*cargo_change, "TXT_KEY_PROMOTION_CARGO_TEXT", "[ICON_BULLET]%D1_Change Cargo Space"),
        (*collateral_damage_change, "TXT_KEY_PROMOTION_COLLATERAL_DAMAGE_TEXT", "[ICON_BULLET]%D1_DmgChange%% Collateral Damage"),
        (*bombard_rate_change, "TXT_KEY_PROMOTION_BOMBARD_TEXT", "[ICON_BULLET]%D1_DmgChange%% City Bombard Damage"),
    ] {
        if n != 0 {
            line(b, d, key, fallback, &[Arg::N(n)]);
        }
    }
    if *first_strikes_change == 1 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKE_TEXT", "[ICON_BULLET]%d1_Num Extra First Strike", &[Arg::N(*first_strikes_change)]);
    } else if *first_strikes_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKES_TEXT", "[ICON_BULLET]%D1_Change First Strikes", &[Arg::N(*first_strikes_change)]);
    }
    if *chance_first_strikes_change == 1 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKE_CHANCE_TEXT", "[ICON_BULLET]%d1_Num Extra First Strike Chance", &[Arg::N(*chance_first_strikes_change)]);
    } else if *chance_first_strikes_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKES_CHANCE_TEXT", "[ICON_BULLET]%D1_Change First Strike Chances", &[Arg::N(*chance_first_strikes_change)]);
    }
    // Healing: one line when it is the same everywhere (FfH BUG addon, Denev), else one per kind of
    // land.
    let heals = |b: &mut String, n: i32, where_key: &str, where_fallback: &str| {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_PROMOTION_HEALS_EXTRA_TEXT", "[ICON_BULLET]Heals Extra %d1_Amount%%[SPACE]", &[Arg::N(n)]));
        b.push_str(&txt(d, where_key, where_fallback, &[]));
    };
    if enemy_heal_change == neutral_heal_change && enemy_heal_change == friendly_heal_change {
        if *enemy_heal_change != 0 {
            heals(b, *enemy_heal_change, "TXT_KEY_PROMOTION_DAMAGE_TURN_TEXT", "Damage/Turn");
        }
    } else {
        if *enemy_heal_change != 0 {
            heals(b, *enemy_heal_change, "TXT_KEY_PROMOTION_ENEMY_LANDS_TEXT", "Damage/Turn in Enemy Lands");
        }
        if *neutral_heal_change != 0 {
            heals(b, *neutral_heal_change, "TXT_KEY_PROMOTION_NEUTRAL_LANDS_TEXT", "Damage/Turn in Neutral Lands");
        }
        if *friendly_heal_change != 0 {
            heals(b, *friendly_heal_change, "TXT_KEY_PROMOTION_FRIENDLY_LANDS_TEXT", "Damage/Turn in Friendly Lands");
        }
    }
    let per_turn = txt(d, "TXT_KEY_PROMOTION_DAMAGE_TURN_TEXT", "Damage/Turn", &[]);
    if *same_tile_heal_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_HEALS_SAME_TEXT", "[ICON_BULLET]Heals Units in Same Tile Extra %d1_Amount%%[SPACE]", &[Arg::N(*same_tile_heal_change)]);
        b.push_str(&per_turn);
    }
    if *adjacent_tile_heal_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_HEALS_ADJACENT_TEXT", "[ICON_BULLET]Heals Units in Adjacent Tiles Extra %d1_Heals%%[SPACE]", &[Arg::N(*adjacent_tile_heal_change)]);
        b.push_str(&per_turn);
    }
    if *combat_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_STRENGTH_TEXT", "[ICON_BULLET]%D1_Bonus%% Strength", &[Arg::N(*combat_percent)]);
    }
    // Attack and defence: one "strength" line when they are equal (FfH BUG addon, Denev).
    if city_attack_percent == city_defense_percent {
        if *city_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_CITY_STRENGTH_MOD", "[ICON_BULLET]%D1_Change%% City Strength", &[Arg::N(*city_attack_percent)]);
        }
    } else {
        if *city_attack_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_CITY_ATTACK_TEXT", "[ICON_BULLET]%D1_Bonus%% City Attack", &[Arg::N(*city_attack_percent)]);
        }
        if *city_defense_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_CITY_DEFENSE_TEXT", "[ICON_BULLET]%D1_Bonus%% City Defense", &[Arg::N(*city_defense_percent)]);
        }
    }
    if hills_attack_percent == hills_defense_percent {
        if *hills_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_HILLS_STRENGTH", "[ICON_BULLET]%D1_Change%% Hills Strength", &[Arg::N(*hills_attack_percent)]);
        }
    } else {
        if *hills_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_HILLS_ATTACK", "[ICON_BULLET]%D1_Change%% Hills Attack", &[Arg::N(*hills_attack_percent)]);
        }
        if *hills_defense_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_HILLS_DEFENSE_TEXT", "[ICON_BULLET]%D1_Bonus%% Hills Defense", &[Arg::N(*hills_defense_percent)]);
        }
    }
    for (n, key, fallback) in [
        (*revolt_protection, "TXT_KEY_PROMOTION_REVOLT_PROTECTION_TEXT", "[ICON_BULLET]Garrisoned City %d1_percent%% less likely to Revolt"),
        (*collateral_damage_protection, "TXT_KEY_PROMOTION_COLLATERAL_PROTECTION_TEXT", "[ICON_BULLET]Suffers %d1_percent%% less Collateral Damage"),
        (*pillage_change, "TXT_KEY_PROMOTION_PILLAGE_CHANGE_TEXT", "[ICON_BULLET]Yields %D1_percent%% [ICON_GOLD] from Pillaging"),
    ] {
        if n != 0 {
            line(b, d, key, fallback, &[Arg::N(n)]);
        }
    }
    if *upgrade_discount == 100 {
        line(b, d, "TXT_KEY_PROMOTION_UPGRADE_DISCOUNT_FREE_TEXT", "[ICON_BULLET]Free upgrades", &[]);
    } else if *upgrade_discount != 0 {
        line(b, d, "TXT_KEY_PROMOTION_UPGRADE_DISCOUNT_TEXT", "[ICON_BULLET]%d1_percent%% less [ICON_GOLD] to Upgrade", &[Arg::N(*upgrade_discount)]);
    }
    if *experience_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FASTER_EXPERIENCE_TEXT", "[ICON_BULLET]Gains %D1_percent%% Experience from Combat", &[Arg::N(*experience_percent)]);
    }
    if *kamikaze_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_KAMIKAZE_TEXT", "[ICON_BULLET]Sacrificed in Combat in Exchange for %D1_percent%% [ICON_STRENGTH]", &[Arg::N(*kamikaze_percent)]);
    }
    // Terrain and feature strength: one line when attack and defence are equal (FfH BUG addon).
    let place_strength = |b: &mut String, attack: i32, defense: i32, place: &str| {
        if attack == defense {
            if attack != 0 {
                line(b, d, "TXT_KEY_UNIT_STRENGTH", "[ICON_BULLET]%D1_Change%% %s2_TerrOrFeat Strength", &[Arg::N(attack), Arg::S(place)]);
            }
        } else {
            if attack != 0 {
                line(b, d, "TXT_KEY_PROMOTION_ATTACK_TEXT", "[ICON_BULLET]%D1_Bonus%% %s2_TerrFeat Attack", &[Arg::N(attack), Arg::S(place)]);
            }
            if defense != 0 {
                line(b, d, "TXT_KEY_PROMOTION_DEFENSE_TEXT", "[ICON_BULLET]%D1_Bonus%% %s2_TerrFeat Defense", &[Arg::N(defense), Arg::S(place)]);
            }
        }
    };
    for t in d.terrains.keys() {
        place_strength(b, terrain_attack_percent[t], terrain_defense_percent[t], &display(d, t));
    }
    for f in d.features.keys() {
        place_strength(b, feature_attack_percent[f], feature_defense_percent[f], &display(d, f));
    }
    const VERSUS: &str = "[ICON_BULLET]%D1_Mod%% vs. [LINK=literal]%s2_Against[\\LINK]";
    for uc in d.unit_combats.keys().filter(|&uc| unit_combat_modifier_percent[uc] != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", VERSUS, &[Arg::N(unit_combat_modifier_percent[uc]), Arg::S(&display(d, uc))]);
    }
    for dom in d.domains.keys().filter(|&dom| domain_modifier_percent[dom] != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", VERSUS, &[Arg::N(domain_modifier_percent[dom]), Arg::S(&display(d, dom))]);
    }

    // FfH.
    if *race {
        line(b, d, "TXT_KEY_PROMOTION_RACE_PEDIA", "[ICON_BULLET]Race: %s1_promotion", &[Arg::S(&display(d, p))]);
    }
    for dt in d.damages.keys() {
        let damage = display(d, dt);
        if damage_type_combat[dt] != 0 {
            line(b, d, "TXT_KEY_DAMAGE_TYPE_COMBAT", "[ICON_BULLET]%D1 %s2 combat", &[Arg::N(damage_type_combat[dt]), Arg::S(&damage)]);
        }
        if damage_type_resist[dt] == 100 {
            line(b, d, "TXT_KEY_PROMOTION_DAMAGE_TYPE_IMMUNE", "[ICON_BULLET]Immune to %s1 damage", &[Arg::S(&damage)]);
        } else if damage_type_resist[dt] != 0 {
            line(b, d, "TXT_KEY_PROMOTION_DAMAGE_TYPE_RESIST", "[ICON_BULLET]%D1% Resistance to %s2 damage", &[Arg::N(damage_type_resist[dt]), Arg::S(&damage)]);
        }
    }
    for bonus in d.bonuses.keys().filter(|&x| bonus_affinity[x] != 0) {
        line(b, d, "TXT_KEY_BONUS_AFFINITY", "[ICON_BULLET]%D1: Affinity for %s2", &[Arg::N(bonus_affinity[bonus]), Arg::S(&display(d, bonus))]);
    }
    if *defensive_strike_chance == 0 {
        if *defensive_strike_damage > 0 {
            line(b, d, "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_DAMAGE_PEDIA", "[ICON_BULLET]Defensive Strikes do +%d1% more damage", &[Arg::N(*defensive_strike_damage)]);
        }
    } else if *defensive_strike_damage > 0 {
        line(
            b,
            d,
            "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_PEDIA",
            "[ICON_BULLET]+%d1% chance to do +%d2% damage to units attacking this unit's stack",
            &[Arg::N(*defensive_strike_chance), Arg::N(*defensive_strike_damage)],
        );
    } else {
        line(b, d, "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_CHANCE_PEDIA", "[ICON_BULLET]+%d1% chance to launch Defensive Strikes", &[Arg::N(*defensive_strike_chance)]);
    }
    if *immune_to_defensive_strike {
        line(b, d, "TXT_KEY_PROMOTION_IMMUNE_TO_DEFENSIVE_STRIKE_PEDIA", "[ICON_BULLET]Immune to Defensive Strikes", &[]);
    }
    if *combat_percent_in_borders != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_PERCENT_IN_BORDERS", "[ICON_BULLET]%D1% strength when within the player's borders", &[Arg::N(*combat_percent_in_borders)]);
    }
    if extra_combat_str == extra_combat_defense {
        if *extra_combat_str != 0 {
            line(b, d, "TXT_KEY_PROMOTION_EXTRA_COMBAT_STR", "[ICON_BULLET]%D1 Strength", &[Arg::N(*extra_combat_str)]);
        }
    } else {
        if *extra_combat_str != 0 {
            line(b, d, "TXT_KEY_PROMOTION_EXTRA_COMBAT_STR_ATTACK", "[ICON_BULLET]%D1 Attack Strength", &[Arg::N(*extra_combat_str)]);
        }
        if *extra_combat_defense != 0 {
            line(b, d, "TXT_KEY_PROMOTION_EXTRA_COMBAT_STR_DEFENSE", "[ICON_BULLET]%D1 Defense Strength", &[Arg::N(*extra_combat_defense)]);
        }
    }
    if *target_weakest_unit {
        line(b, d, "TXT_KEY_PROMOTION_TARGET_WEAKEST_UNIT", "[ICON_BULLET]Attacks the weakest unit in a stack", &[]);
    }
    if *target_weakest_unit_counter {
        line(b, d, "TXT_KEY_PROMOTION_TARGET_WEAKEST_UNIT_COUNTER", "[ICON_BULLET]Guards against enemy Marksman units", &[]);
    }
    if *better_defender_than_percent < 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETTER_DEFENDER_THAN_PERCENT_LESS", "[ICON_BULLET]Reduced chance to defend the stack", &[]);
    } else if *better_defender_than_percent > 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETTER_DEFENDER_THAN_PERCENT_MORE", "[ICON_BULLET]Increased chance to defend the stack", &[]);
    }
    if let Some(q) = promotion_combat_apply {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_APPLY", "[ICON_BULLET]Passes the %s1_promotion promotion to enemies in combat", &[Arg::S(&display(d, *q))]);
    }
    if let Some(q) = promotion_random_apply {
        line(b, d, "TXT_KEY_PROMOTION_RANDOM_APPLY", "[ICON_BULLET]Randomly grants the %s1_promotion promotion", &[Arg::S(&display(d, *q))]);
    }
    if *boarding {
        line(b, d, "TXT_KEY_PROMOTION_BOARDING_PEDIA", "[ICON_BULLET]Allows the unit to attack ships", &[]);
    }
    if let Some(uc) = capture_unit_combat {
        line(b, d, "TXT_KEY_PROMOTION_CAPTURE_UNITCOMBAT", "[ICON_BULLET]Captures defeated %s1_unitcombat", &[Arg::S(&display(d, *uc))]);
    }
    if *combat_capture_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_CAPTURE_PERCENT", "[ICON_BULLET]%D1% chance of converting defeated living units", &[Arg::N(*combat_capture_percent)]);
    }
    if *combat_heal_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_HEAL_PERCENT", "[ICON_BULLET]Heals %D1% after combat", &[Arg::N(*combat_heal_percent)]);
    }
    if *combat_limit != 0 {
        let limit = 100 * combat_limit / d.defines.max_hit_points.max(1);
        line(b, d, "TXT_KEY_UNIT_COMBAT_LIMIT", "[ICON_BULLET]Maximum %d1%% damage to enemy on attack", &[Arg::N(limit)]);
    }
    if *fear {
        line(b, d, "TXT_KEY_PROMOTION_FEAR_PEDIA", "[ICON_BULLET]Causes fear, scattering units in tiles it attacks", &[]);
    }
    if *free_xp_per_turn != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FREE_XP_PER_TURN", "[ICON_BULLET]%D1 XP per turn until %D2 XP", &[Arg::N(*free_xp_per_turn), Arg::N(d.defines.free_xp_max)]);
    }
    for (n, key, fallback) in [
        (*free_xp_from_combat, "TXT_KEY_PROMOTION_FREE_XP_FROM_COMBAT", "[ICON_BULLET]%D1 XP per combat"),
        (*gold_from_combat, "TXT_KEY_UNIT_GOLD_FROM_COMBAT", "[ICON_BULLET]Combat victories produce %D1 gold"),
        (*modify_global_counter, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER", "[ICON_BULLET]%D1 to the Armageddon Counter"),
        (*modify_global_counter_on_combat, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER_ON_COMBAT", "[ICON_BULLET]%D1 to the Armageddon Counter when a living unit is killed in combat"),
    ] {
        if n != 0 {
            line(b, d, key, fallback, &[Arg::N(n)]);
        }
    }
    if *spell_caster_xp != 0 {
        line(b, d, "TXT_KEY_PROMOTION_SPELL_CASTER_XP", "[ICON_BULLET]Increases the rate spellcasters gain xp", &[]);
    }
    for (has, key, fallback) in [
        (*flying, "TXT_KEY_PROMOTION_FLYING_PEDIA", "[ICON_BULLET]Grants the unit the ability to fly"),
        (*held, "TXT_KEY_PROMOTION_HELD_PEDIA", "[ICON_BULLET]Keeps the unit from being able to move"),
        (*hidden_nationality, "TXT_KEY_PROMOTION_HIDDEN_NATIONALITY_PEDIA", "[ICON_BULLET]Hides the unit's civilization (will appear as a barbarian unit to all civilizations except its own)"),
        (*ignore_building_defense, "TXT_KEY_PROMOTION_IGNORE_BUILDING_DEFENSE", "[ICON_BULLET]Ignores building defense when attacking cities"),
        (*immortal, "TXT_KEY_PROMOTION_IMMORTAL_PEDIA", "[ICON_BULLET]Resurrected in the civilization's capital if killed"),
        (*immune_to_capture, "TXT_KEY_PROMOTION_IMMUNE_TO_CAPTURE_PEDIA", "[ICON_BULLET]Immune to Capture"),
        (*immune_to_magic, "TXT_KEY_PROMOTION_IMMUNE_TO_MAGIC_PEDIA", "[ICON_BULLET]Immune to Magic"),
        (*immune_to_fear, "TXT_KEY_PROMOTION_IMMUNE_TO_FEAR_PEDIA", "[ICON_BULLET]Immune to Fear"),
    ] {
        if has {
            line(b, d, key, fallback, &[]);
        }
    }
    for q in [promotion_immune1, promotion_immune2, promotion_immune3].into_iter().flatten() {
        line(b, d, "TXT_KEY_PROMOTION_IMMUNE", "[ICON_BULLET]Immune to the %s1_promotion promotion", &[Arg::S(&display(d, *q))]);
    }
    for (has, key, fallback) in [
        (*invisible, "TXT_KEY_PROMOTION_INVISIBLE_PEDIA", "[ICON_BULLET]Makes the unit invisible"),
        (*see_invisible, "TXT_KEY_PROMOTION_SEE_INVISIBLE_PEDIA", "[ICON_BULLET]Allows the unit to see invisible units"),
        (*double_fortify_bonus, "TXT_KEY_PROMOTION_DOUBLE_FORTIFY_BONUS", "[ICON_BULLET]Doubles the unit's fortification bonus"),
        (*twincast, "TXT_KEY_PROMOTION_TWINCAST_PEDIA", "[ICON_BULLET]Summon spells cast by the unit are doubled"),
        (*water_walking, "TXT_KEY_PROMOTION_WATER_WALKING_PEDIA", "[ICON_BULLET]Allows the unit to travel over water tiles"),
        (*not_alive, "TXT_KEY_PROMOTION_NOT_ALIVE", "[ICON_BULLET]Units with this promotion are not alive"),
    ] {
        if has {
            line(b, d, key, fallback, &[]);
        }
    }
    for (n, key, fallback) in [
        (*resist_magic, "TXT_KEY_PROMOTION_RESIST", "[ICON_BULLET]%D1% to resist spells"),
        (*spell_damage_modify, "TXT_KEY_PROMOTION_SPELL_DAMAGE_MODIFY", "[ICON_BULLET]%D1 to spell damage"),
        (*caster_resist_modify, "TXT_KEY_PROMOTION_CASTER_RESIST_MODIFY", "[ICON_BULLET]Modifies targets' magic resistance by %D1%"),
    ] {
        if n != 0 {
            line(b, d, key, fallback, &[Arg::N(n)]);
        }
    }
    // The spells it allows (those whose first promotion it is; not the pedia's hidden ones).
    for s in d.spells.keys() {
        let spell = &d.spells[s];
        if spell.promotion_prereq1 != Some(p) || spell.base.graphical_only {
            continue;
        }
        match spell.promotion_prereq2 {
            None => line(b, d, "TXT_KEY_PROMOTION_ALLOWS_TEXT", "[ICON_BULLET]Allows: %s1", &[Arg::S(&display(d, s))]),
            Some(q) => line(b, d, "TXT_KEY_PROMOTION_ALLOWS_WITH_TEXT", "[ICON_BULLET]Allows: %s1 (with %s2)", &[Arg::S(&display(d, s)), Arg::S(&display(d, q))]),
        }
    }
    if *expire_chance != 0 {
        line(b, d, "TXT_KEY_PROMOTION_EXPIRE_CHANCE", "[ICON_BULLET]%D1% chance of wearing off each turn", &[Arg::N(*expire_chance)]);
    }
    for (has, key, fallback) in [
        (*removed_by_casting, "TXT_KEY_PROMOTION_REMOVED_BY_CASTING", "[ICON_BULLET]Removed by casting spells or using special abilities"),
        (*removed_by_combat, "TXT_KEY_PROMOTION_REMOVED_BY_COMBAT", "[ICON_BULLET]Removed after combat"),
        (*removed_when_healed, "TXT_KEY_PROMOTION_REMOVED_WHEN_HEALED", "[ICON_BULLET]Removed when the unit is fully healed"),
    ] {
        if has {
            line(b, d, key, fallback, &[]);
        }
    }
    if let Some(q) = promotion_summon_perk {
        line(b, d, "TXT_KEY_PROMOTION_SUMMON_PERK", "[ICON_BULLET]Grants the %s1_promotion promotion to summoned units", &[Arg::S(&display(d, *q))]);
    }
    // C++ reads the promotion it fights better against without checking it is set (none is
    // missing in FFH2's data): nothing then.
    if let Some(q) = promotion_combat_type.filter(|_| *promotion_combat_mod != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", VERSUS, &[Arg::N(*promotion_combat_mod), Arg::S(&display(d, q))]);
    }
    if *betrayal_chance != 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETRAYAL_CHANCE", "[ICON_BULLET]%D1% to become a barbarian unit each turn", &[Arg::N(*betrayal_chance)]);
    }
    if *work_rate_modify != 0 {
        line(b, d, "TXT_KEY_PROMOTION_WORK_RATE_MODIFY", "[ICON_BULLET]%D1% to work rate", &[Arg::N(*work_rate_modify)]);
    }
    // The XML help.
    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        b.push_str(NEWLINE);
        b.push_str(help);
    }

    // ffh-play's: what it takes to choose it on a level-up (C++ `CvUnit::canAcquirePromotion` and
    // `isPromotionValid`, as the pedia's "Requires" and "Available To" panels show it), in the
    // words C++ `setUnitHelp` uses for a unit's requirements. Nothing for the promotions no unit
    // can choose: races, equipment, the effects of spells and the like (`iMinLevel` -1), those
    // behind a tech nobody can research (Never) and those for no unit combat (only a unit that
    // starts with one may have it, and it has it then).
    let chosen_by_none = *min_level == -1 || *race || *equipment || tech_prereq.is_some_and(|t| d.techs[t].disable) || !unit_combat.iter().any(|&c| c);
    if chosen_by_none {
        return finish(std::mem::take(b));
    }
    let and_ = txt(d, "TXT_KEY_AND", "[SPACE]and[SPACE]", &[]);
    let or_ = txt(d, "TXT_KEY_OR", "[SPACE]or[SPACE]", &[]);
    let join = |ps: &[PromotionType], sep: &str| ps.iter().map(|&q| display(d, q).into_owned()).collect::<Vec<_>>().join(sep);
    // Both "and" promotions, and one of the "or" ones (counted only with the first of them, as the
    // rules count them).
    let all: Vec<PromotionType> = [*prereq_promotion, *promotion_prereq_and].into_iter().flatten().collect();
    let any: Vec<PromotionType> = match prereq_or_promotion1 {
        Some(first) => [Some(*first), *prereq_or_promotion2, *promotion_prereq_or3, *promotion_prereq_or4].into_iter().flatten().collect(),
        None => Vec::new(),
    };
    let mut needs = join(&all, &and_);
    if !any.is_empty() {
        if all.is_empty() {
            needs = join(&any, &or_);
        } else if any.len() == 1 {
            needs = format!("{needs}{and_}{}", join(&any, &or_));
        } else {
            needs = format!("{needs}{and_}({})", join(&any, &or_));
        }
    }
    if !needs.is_empty() {
        b.push_str(NEWLINE);
        b.push_str(&req(d, "TXT_KEY_REQUIRES", "[COLOR_WARNING_TEXT]Requires[SPACE]", &[]));
        b.push_str(&needs);
    }
    const REQUIRES: &str = "[COLOR_WARNING_TEXT]Requires %s1_Text[COLOR_REVERT]";
    if let Some(t) = tech_prereq {
        line_req(b, d, "TXT_KEY_UNIT_REQUIRES_STRING", REQUIRES, &[Arg::S(&display(d, *t))]);
    }
    if let Some(r) = state_religion_prereq {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_TECH_PREREQ_RELIGION", "Requires: %s1_religion state religion", &[Arg::S(&display(d, *r))]));
    }
    // The player must have the resource (C++ `CvPlayer::hasBonus`).
    if let Some(bonus) = bonus_prereq {
        line_req(b, d, "TXT_KEY_UNIT_REQUIRES_STRING", REQUIRES, &[Arg::S(&display(d, *bonus))]);
    }
    if *min_level > 0 {
        line_req(b, d, "TXT_KEY_BUILDING_REQUIRES_UNIT_LEVEL", "[COLOR_WARNING_TEXT]Requires a Unit of Level %d1_Num Experience[COLOR_REVERT]", &[Arg::N(*min_level)]);
    }
    if *prereq_alive {
        b.push_str(NEWLINE);
        b.push_str("Requires a living unit");
    }
    let combats: Vec<String> = d.unit_combats.keys().filter(|&uc| unit_combat[uc]).map(|uc| display(d, uc).into_owned()).collect();
    if !combats.is_empty() {
        b.push_str(NEWLINE);
        b.push_str(&format!("{}: {}", txt(d, "TXT_KEY_PEDIA_PROMOTION_UNITS", "Available To", &[]), combats.join(", ")));
    }
    finish(std::mem::take(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<PromotionType>(name)).replace("[SPACE]", " ")
    }

    fn has_placeholder(t: &str) -> bool {
        t.as_bytes().windows(3).any(|w| w[0] == b'%' && matches!(w[1], b'd' | b'D' | b's' | b'F') && w[2].is_ascii_digit())
    }

    #[test]
    fn known_promotions_say_what_they_do() {
        let combat1 = of("PROMOTION_COMBAT1");
        for part in ["[ICON_BULLET]+20% Strength", "[ICON_BULLET]+5 to spell damage", "Grants the Empower I promotion to summoned units", "Available To: Recon Units, Archery Units, ", "Beast Units"] {
            assert!(combat1.contains(part), "{part}: {combat1}");
        }
        // Nothing to choose it after, and no tech.
        assert!(!combat1.contains("Requires"), "{combat1}");
        assert!(combat1.starts_with("[ICON_BULLET]"), "{combat1}");

        let shock = of("PROMOTION_SHOCK");
        assert!(shock.contains("[ICON_BULLET]+40% vs. [LINK=literal]Melee Units[\\LINK]"), "{shock}");
        assert!(shock.contains("Requires Combat I"), "{shock}");
        // A requirement is not coloured as unmet: nothing is known of the unit.
        assert!(!shock.contains("COLOR_WARNING_TEXT"), "{shock}");

        // Given, never chosen: no requirements.
        let hero = of("PROMOTION_HERO");
        assert!(hero.contains("[ICON_BULLET]+1 XP per turn until +100 XP"), "{hero}");
        assert!(hero.contains("[ICON_BULLET]+20% to resist spells"), "{hero}");
        assert!(!hero.contains("Requires") && !hero.contains("Available To"), "{hero}");
        // For no unit combat: no unit chooses it either.
        let medic3 = of("PROMOTION_MEDIC3");
        assert!(medic3.contains("[ICON_BULLET]Heals Units in Adjacent Tiles Extra 20% Damage/Turn"), "{medic3}");
        assert!(medic3.contains("[ICON_BULLET]Allows: Heal") && !medic3.contains("Requires"), "{medic3}");

        // A sphere: its spells, its channeling, its mana.
        let fire1 = of("PROMOTION_FIRE1");
        for part in ["Allows: Blaze (with Channeling I)", "Requires Channeling I[NEWLINE]", "Requires Fire Mana", "Available To: Arcane Units"] {
            assert!(fire1.contains(part), "{part}: {fire1}");
        }
        let fire2 = of("PROMOTION_FIRE2");
        assert!(fire2.contains("Requires Fire I and Channeling II"), "{fire2}");
        assert!(fire2.contains("Allows: Fireball"), "{fire2}");

        // A race.
        let elf = of("PROMOTION_ELF");
        for part in ["Race: Elven", "Double Movement in Forest", "+10% Forest Attack", "-25% Resistance to Cold damage", "-20% to work rate"] {
            assert!(elf.contains(part), "{part}: {elf}");
        }
        let undead = of("PROMOTION_UNDEAD");
        for part in ["Immune to Death damage", "-50% Resistance to Holy damage", "Immune to Fear", "Units with this promotion are not alive"] {
            assert!(undead.contains(part), "{part}: {undead}");
        }

        // "And" with a group of "or", the same healing everywhere, a state religion.
        let fear = of("PROMOTION_FEAR");
        assert!(fear.contains("Requires Combat V and (Demon or Undead or Nightmare)"), "{fear}");
        let march = of("PROMOTION_MARCH");
        assert!(march.contains("[ICON_BULLET]Heals Extra 10% Damage/Turn"), "{march}");
        assert!(march.contains("Requires Combat III or Medic I"), "{march}");
        assert!(!march.contains("Enemy Lands"), "{march}");
        let command4 = of("PROMOTION_COMMAND4");
        assert!(command4.contains("Requires Command III and Channeling III"), "{command4}");
        assert!(command4.contains("Requires: Order state religion"), "{command4}");
    }

    #[test]
    fn every_promotion_reads_without_placeholders_and_says_something() {
        let d = ffh_data::global();
        for p in d.promotions.keys() {
            let e = effects(d, p);
            let name = names::name(d, p);
            assert!(!has_placeholder(&e), "{name}: {e}");
            assert!(!e.starts_with(NEWLINE) && !e.contains("TXT_KEY_"), "{name}: {e}");
            // Every promotion says what it does, but for pieces of equipment that do nothing in
            // the data (their buildings and Python do it: the Dragon's Hoard, Syliven's Lyre).
            let info = &d.promotions[p];
            if e.is_empty() {
                assert!(info.equipment && info.base.help.is_empty(), "{name} says nothing");
            }
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        for name in ["PROMOTION_COMBAT1", "PROMOTION_SHOCK", "PROMOTION_HERO", "PROMOTION_FIRE1", "PROMOTION_ELF", "PROMOTION_FEAR", "PROMOTION_MARCH", "PROMOTION_COMMAND4"] {
            println!("{name}:\n{}\n", of(name));
        }
    }
}
