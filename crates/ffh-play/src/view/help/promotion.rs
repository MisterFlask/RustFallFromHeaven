//! What a promotion does: C++ `CvGameTextMgr::parsePromotionHelp` (its `pcNewline`, each line's
//! start, is the help's line break here, as in the pedia), then what it takes to choose it on a
//! level-up, which FFH2's help leaves to the pedia's buttons (Python
//! `CvPediaPromotion.placePrereqs` and `placeUnitGroups`; the rules: C++
//! `CvUnit::canAcquirePromotion`).

use ffh_data::*;

use super::{finish, req, txt, Arg, NEWLINE};
use crate::names::{self, display};

/// One line of the help: C++ `szBuffer.append(pcNewline); szBuffer.append(gDLL->getText(..))`.
fn line(b: &mut String, d: &GameData, key: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&txt(d, key, args));
}

/// A stat's name in the help (docs/UNIT_STATS.md).
pub fn stat_name(s: StatType) -> &'static str {
    match s {
        StatType::Hp => "HP",
        StatType::Attack => "Attack",
        StatType::Defense => "Defense",
        StatType::Magic => "Magic",
        StatType::MagicDefense => "Magic defense",
        StatType::Accuracy => "Accuracy",
        StatType::Evasion => "Evasion",
        StatType::Crit => "Crit",
        StatType::Speed => "Speed",
    }
}

/// The battle magic of a unit type or promotion (docs/CARDS.md, "New data keys"), a line each:
/// what its spells, presence and hits do in a formation battle. Bare rules text.
pub(super) fn magic_lines(b: &mut String, d: &GameData, m: &BattleMagic) {
    let BattleMagic {
        spell_power,
        damage_only,
        spell_pattern,
        front_caster,
        hex_ward,
        end_charms,
        presence,
        revive,
        dispel,
        damage_vs_summons,
        spell,
        strike_spell,
        opening_cast,
        death_recall,
        familiar,
    } = m;
    let mut say = |text: String| {
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]In battle: ");
        b.push_str(&text);
    };
    if *spell_power != 0 {
        say(format!("{spell_power:+}% damage of its damage spells, the magic bolt included"));
    }
    if *damage_only {
        say("it casts no charms, hexes or heals".to_string());
    }
    if let Some(p) = spell_pattern {
        say(format!("its damage spells hit with the {} pattern", p.token().to_lowercase()));
    }
    if *front_caster {
        say("from the front row it casts as many spells as from its best row".to_string());
    }
    if *hex_ward {
        say("while it is on the field, enemy hexes have no effect on its side".to_string());
    }
    if *end_charms {
        say("while it is on the field, the enemy side's charms end at the start of each round after the first".to_string());
    }
    match presence {
        Some(Presence::Grounding) => say("while it is in the line at the start, enemy charges are strikes for the battle (front of a lane, no first-round bonus)".to_string()),
        Some(Presence::Madness) => say("while it is in the line at the start, the enemy unit with the highest attack (not a Sentinel) hits its own side in round 1".to_string()),
        None => {}
    }
    if *revive > 0 {
        say(format!("once per battle, its heal also brings back the first unit of its side that fell, at {revive}% HP, in its cell if free, else the rear cell of its lane (not a Sentinel)"));
    }
    if *dispel {
        say("each hit it lands ends the target's charms and its dispellable promotions' battle effects for the battle".to_string());
    }
    if *damage_vs_summons != 0 {
        say(format!("{damage_vs_summons:+}% damage to summoned units and illusions"));
    }
    if let Some(s) = spell {
        say(format!("knows the battle spell {}", display(d, *s)));
    }
    if let Some(s) = strike_spell {
        say(format!("casts {} in place of its first strike each round, with half its attack as its magic", display(d, *s)));
    }
    if *opening_cast {
        say("when its plot is attacked, it casts its damage spell before the first round; the promotion is then lost".to_string());
    }
    if *death_recall {
        say("when a hit would kill it, it leaves the battle and returns to the capital with 1 HP; the promotion is then lost".to_string());
    }
    if *familiar {
        say("bound to a caster, it takes the first spell or shot aimed at that caster; the caster gains 1 XP a turn while they share a plot".to_string());
    }
}

/// The battle stats a promotion changes, a line each: "+20% Attack", "+10 Defense", "-1 Speed".
/// The battle levers of a unit type or promotion (docs/UNIT_ROLES.md): damage against and
/// resistance to unit combats, armor, armor piercing, pursuit; one line each.
pub(super) fn lever_lines(
    b: &mut String,
    d: &GameData,
    damage_vs: &IdxVec<UnitCombatType, i32>,
    resist_vs: &IdxVec<UnitCombatType, i32>,
    armor: i32,
    piercing: bool,
    pursuit: bool,
) {
    for (c, &v) in damage_vs.iter_enumerated() {
        if v != 0 {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]In battle: {v:+}% damage to {} units", display(d, c)));
        }
    }
    for (c, &v) in resist_vs.iter_enumerated() {
        if v != 0 {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]In battle: {:+}% damage from {} units", -v, display(d, c)));
        }
    }
    if armor != 0 {
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]In battle: armor {armor} (HP off each physical hit taken)"));
    }
    if piercing {
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]In battle: its hits ignore armor");
    }
    if pursuit {
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]In battle: its hits cannot be withdrawn from");
    }
}

fn stat_lines(b: &mut String, changes: &[StatChange; StatType::NUM]) {
    for s in StatType::ALL {
        let c = changes[s.idx()];
        if c.flat != 0 {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]{:+} {}", c.flat, stat_name(s)));
        }
        if c.percent != 0 {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]{:+}% {}", c.percent, stat_name(s)));
        }
    }
}

/// A requirement line: `line` without the warning colour (`super::unwarned`).
fn line_req(b: &mut String, d: &GameData, key: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&req(d, key, args));
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
        stat_changes,
        anti_flank,
        damage_vs,
        resist_vs,
        armor,
        armor_piercing,
        pursuit,
        magic,
        spell_damage_modify,
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
        false_colours,
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
        free_xp_per_turn,
        free_xp_from_combat,
        gold_from_combat,
        min_level,
        modify_global_counter,
        modify_global_counter_on_combat,
        resist_magic,
        spell_caster_xp,
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
        first_melee_pattern,
        heal_behind,
        unit_classes,
        attack_damage_type,
    } = &d.promotions[p];
    let b = &mut String::new();

    for (has, key) in [
        (*blitz, "TXT_KEY_PROMOTION_BLITZ_TEXT"),
        (*amphib, "TXT_KEY_PROMOTION_AMPHIB_TEXT"),
        (*river, "TXT_KEY_PROMOTION_RIVER_ATTACK_TEXT"),
        (*enemy_route, "TXT_KEY_PROMOTION_ENEMY_ROADS_TEXT"),
        (*always_heal, "TXT_KEY_PROMOTION_ALWAYS_HEAL_TEXT"),
        (*hills_double_move, "TXT_KEY_PROMOTION_HILLS_MOVE_TEXT"),
        (*immune_to_first_strikes, "TXT_KEY_PROMOTION_IMMUNE_FIRST_STRIKES_TEXT"),
    ] {
        if has {
            line(b, d, key, &[]);
        }
    }
    for t in d.terrains.keys().filter(|&t| terrain_double_move[t]) {
        line(b, d, "TXT_KEY_PROMOTION_DOUBLE_MOVE_TEXT", &[Arg::S(&display(d, t))]);
    }
    for f in d.features.keys().filter(|&f| feature_double_move[f]) {
        line(b, d, "TXT_KEY_PROMOTION_DOUBLE_MOVE_TEXT", &[Arg::S(&display(d, f))]);
    }
    for (n, key) in [
        (*visibility_change, "TXT_KEY_PROMOTION_VISIBILITY_TEXT"),
        (*moves_change, "TXT_KEY_PROMOTION_MOVE_TEXT"),
        // C++ passes the discount negated: a change of the cost.
        (-move_discount_change, "TXT_KEY_PROMOTION_MOVE_DISCOUNT_TEXT"),
        (*air_range_change, "TXT_KEY_PROMOTION_AIR_RANGE_TEXT"),
        (*intercept_change, "TXT_KEY_PROMOTION_INTERCEPT_TEXT"),
        (*evasion_change, "TXT_KEY_PROMOTION_EVASION_TEXT"),
        (*withdrawal_change, "TXT_KEY_PROMOTION_WITHDRAWAL_TEXT"),
        (*cargo_change, "TXT_KEY_PROMOTION_CARGO_TEXT"),
        (*collateral_damage_change, "TXT_KEY_PROMOTION_COLLATERAL_DAMAGE_TEXT"),
        (*bombard_rate_change, "TXT_KEY_PROMOTION_BOMBARD_TEXT"),
    ] {
        if n != 0 {
            line(b, d, key, &[Arg::N(n)]);
        }
    }
    if *first_strikes_change == 1 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKE_TEXT", &[Arg::N(*first_strikes_change)]);
    } else if *first_strikes_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKES_TEXT", &[Arg::N(*first_strikes_change)]);
    }
    if *chance_first_strikes_change == 1 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKE_CHANCE_TEXT", &[Arg::N(*chance_first_strikes_change)]);
    } else if *chance_first_strikes_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FIRST_STRIKES_CHANCE_TEXT", &[Arg::N(*chance_first_strikes_change)]);
    }
    // Healing: one line when it is the same everywhere (FfH BUG addon, Denev), else one per kind of
    // land.
    let heals = |b: &mut String, n: i32, where_key: &str| {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_PROMOTION_HEALS_EXTRA_TEXT", &[Arg::N(n)]));
        b.push_str(&txt(d, where_key, &[]));
    };
    if enemy_heal_change == neutral_heal_change && enemy_heal_change == friendly_heal_change {
        if *enemy_heal_change != 0 {
            heals(b, *enemy_heal_change, "TXT_KEY_PROMOTION_DAMAGE_TURN_TEXT");
        }
    } else {
        if *enemy_heal_change != 0 {
            heals(b, *enemy_heal_change, "TXT_KEY_PROMOTION_ENEMY_LANDS_TEXT");
        }
        if *neutral_heal_change != 0 {
            heals(b, *neutral_heal_change, "TXT_KEY_PROMOTION_NEUTRAL_LANDS_TEXT");
        }
        if *friendly_heal_change != 0 {
            heals(b, *friendly_heal_change, "TXT_KEY_PROMOTION_FRIENDLY_LANDS_TEXT");
        }
    }
    let per_turn = txt(d, "TXT_KEY_PROMOTION_DAMAGE_TURN_TEXT", &[]);
    if *same_tile_heal_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_HEALS_SAME_TEXT", &[Arg::N(*same_tile_heal_change)]);
        b.push_str(&per_turn);
    }
    if *adjacent_tile_heal_change != 0 {
        line(b, d, "TXT_KEY_PROMOTION_HEALS_ADJACENT_TEXT", &[Arg::N(*adjacent_tile_heal_change)]);
        b.push_str(&per_turn);
    }
    stat_lines(b, stat_changes);
    lever_lines(b, d, damage_vs, resist_vs, *armor, *armor_piercing, *pursuit);
    magic_lines(b, d, magic);
    if *anti_flank {
        // This game's own (docs/ENGAGEMENTS.md, Q7).
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]Anti-flanking in battle (a unit that strikes from the row it prefers): enemy flankers attack it first whenever it is in their reach");
    }
    // The great-person cards' keys (docs/CARDS.md, section 2).
    if let Some(pattern) = first_melee_pattern {
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]In battle: its first Strike or Charge each round has the {} pattern", pattern.token().to_lowercase()));
    }
    if *heal_behind > 0 {
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]In battle: at the end of each round it heals the nearest ally behind it in its lane {heal_behind} HP"));
    }
    if let Some(dt) = attack_damage_type {
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]In battle: its physical attack deals {} damage (the target's resistance to it cuts the attack)", display(d, *dt).to_lowercase()));
    }
    if !unit_classes.is_empty() {
        let names: Vec<String> = unit_classes.iter().map(|&k| display(d, k).to_string()).collect();
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]Only for: {}", names.join(", ")));
    }
    if *spell_damage_modify != 0 {
        line(b, d, "TXT_KEY_PROMOTION_SPELL_DAMAGE_MODIFY", &[Arg::N(*spell_damage_modify)]);
    }
    // Attack and defence: one "strength" line when they are equal (FfH BUG addon, Denev).
    if city_attack_percent == city_defense_percent {
        if *city_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_CITY_STRENGTH_MOD", &[Arg::N(*city_attack_percent)]);
        }
    } else {
        if *city_attack_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_CITY_ATTACK_TEXT", &[Arg::N(*city_attack_percent)]);
        }
        if *city_defense_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_CITY_DEFENSE_TEXT", &[Arg::N(*city_defense_percent)]);
        }
    }
    if hills_attack_percent == hills_defense_percent {
        if *hills_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_HILLS_STRENGTH", &[Arg::N(*hills_attack_percent)]);
        }
    } else {
        if *hills_attack_percent != 0 {
            line(b, d, "TXT_KEY_UNIT_HILLS_ATTACK", &[Arg::N(*hills_attack_percent)]);
        }
        if *hills_defense_percent != 0 {
            line(b, d, "TXT_KEY_PROMOTION_HILLS_DEFENSE_TEXT", &[Arg::N(*hills_defense_percent)]);
        }
    }
    for (n, key) in [
        (*revolt_protection, "TXT_KEY_PROMOTION_REVOLT_PROTECTION_TEXT"),
        (*collateral_damage_protection, "TXT_KEY_PROMOTION_COLLATERAL_PROTECTION_TEXT"),
        (*pillage_change, "TXT_KEY_PROMOTION_PILLAGE_CHANGE_TEXT"),
    ] {
        if n != 0 {
            line(b, d, key, &[Arg::N(n)]);
        }
    }
    if *upgrade_discount == 100 {
        line(b, d, "TXT_KEY_PROMOTION_UPGRADE_DISCOUNT_FREE_TEXT", &[]);
    } else if *upgrade_discount != 0 {
        line(b, d, "TXT_KEY_PROMOTION_UPGRADE_DISCOUNT_TEXT", &[Arg::N(*upgrade_discount)]);
    }
    if *experience_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FASTER_EXPERIENCE_TEXT", &[Arg::N(*experience_percent)]);
    }
    if *kamikaze_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_KAMIKAZE_TEXT", &[Arg::N(*kamikaze_percent)]);
    }
    // Terrain and feature strength: one line when attack and defence are equal (FfH BUG addon).
    let place_strength = |b: &mut String, attack: i32, defense: i32, place: &str| {
        if attack == defense {
            if attack != 0 {
                line(b, d, "TXT_KEY_UNIT_STRENGTH", &[Arg::N(attack), Arg::S(place)]);
            }
        } else {
            if attack != 0 {
                line(b, d, "TXT_KEY_PROMOTION_ATTACK_TEXT", &[Arg::N(attack), Arg::S(place)]);
            }
            if defense != 0 {
                line(b, d, "TXT_KEY_PROMOTION_DEFENSE_TEXT", &[Arg::N(defense), Arg::S(place)]);
            }
        }
    };
    for t in d.terrains.keys() {
        place_strength(b, terrain_attack_percent[t], terrain_defense_percent[t], &display(d, t));
    }
    for f in d.features.keys() {
        place_strength(b, feature_attack_percent[f], feature_defense_percent[f], &display(d, f));
    }
    for uc in d.unit_combats.keys().filter(|&uc| unit_combat_modifier_percent[uc] != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", &[Arg::N(unit_combat_modifier_percent[uc]), Arg::S(&display(d, uc))]);
    }
    for dom in d.domains.keys().filter(|&dom| domain_modifier_percent[dom] != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", &[Arg::N(domain_modifier_percent[dom]), Arg::S(&display(d, dom))]);
    }

    // FfH.
    if *race {
        line(b, d, "TXT_KEY_PROMOTION_RACE_PEDIA", &[Arg::S(&display(d, p))]);
    }
    for dt in d.damages.keys() {
        let damage = display(d, dt);
        if damage_type_combat[dt] != 0 {
            line(b, d, "TXT_KEY_DAMAGE_TYPE_COMBAT", &[Arg::N(damage_type_combat[dt]), Arg::S(&damage)]);
        }
        if damage_type_resist[dt] == 100 {
            line(b, d, "TXT_KEY_PROMOTION_DAMAGE_TYPE_IMMUNE", &[Arg::S(&damage)]);
        } else if damage_type_resist[dt] != 0 {
            line(b, d, "TXT_KEY_PROMOTION_DAMAGE_TYPE_RESIST", &[Arg::N(damage_type_resist[dt]), Arg::S(&damage)]);
        }
    }
    for bonus in d.bonuses.keys().filter(|&x| bonus_affinity[x] != 0) {
        line(b, d, "TXT_KEY_BONUS_AFFINITY", &[Arg::N(bonus_affinity[bonus]), Arg::S(&display(d, bonus))]);
    }
    if *defensive_strike_chance == 0 {
        if *defensive_strike_damage > 0 {
            line(b, d, "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_DAMAGE_PEDIA", &[Arg::N(*defensive_strike_damage)]);
        }
    } else if *defensive_strike_damage > 0 {
        line(
            b,
            d,
            "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_PEDIA",
            &[Arg::N(*defensive_strike_chance), Arg::N(*defensive_strike_damage)],
        );
    } else {
        line(b, d, "TXT_KEY_PROMOTION_DEFENSIVE_STRIKE_CHANCE_PEDIA", &[Arg::N(*defensive_strike_chance)]);
    }
    if *immune_to_defensive_strike {
        line(b, d, "TXT_KEY_PROMOTION_IMMUNE_TO_DEFENSIVE_STRIKE_PEDIA", &[]);
    }
    if *combat_percent_in_borders != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_PERCENT_IN_BORDERS", &[Arg::N(*combat_percent_in_borders)]);
    }
    if *target_weakest_unit {
        line(b, d, "TXT_KEY_PROMOTION_TARGET_WEAKEST_UNIT", &[]);
    }
    if *target_weakest_unit_counter {
        line(b, d, "TXT_KEY_PROMOTION_TARGET_WEAKEST_UNIT_COUNTER", &[]);
    }
    if *better_defender_than_percent < 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETTER_DEFENDER_THAN_PERCENT_LESS", &[]);
    } else if *better_defender_than_percent > 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETTER_DEFENDER_THAN_PERCENT_MORE", &[]);
    }
    if let Some(q) = promotion_combat_apply {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_APPLY", &[Arg::S(&display(d, *q))]);
    }
    if let Some(q) = promotion_random_apply {
        line(b, d, "TXT_KEY_PROMOTION_RANDOM_APPLY", &[Arg::S(&display(d, *q))]);
    }
    if *boarding {
        line(b, d, "TXT_KEY_PROMOTION_BOARDING_PEDIA", &[]);
    }
    if let Some(uc) = capture_unit_combat {
        line(b, d, "TXT_KEY_PROMOTION_CAPTURE_UNITCOMBAT", &[Arg::S(&display(d, *uc))]);
    }
    if *combat_capture_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_CAPTURE_PERCENT", &[Arg::N(*combat_capture_percent)]);
    }
    if *combat_heal_percent != 0 {
        line(b, d, "TXT_KEY_PROMOTION_COMBAT_HEAL_PERCENT", &[Arg::N(*combat_heal_percent)]);
    }
    if *combat_limit != 0 {
        let limit = 100 * combat_limit / d.defines.max_hit_points.max(1);
        line(b, d, "TXT_KEY_UNIT_COMBAT_LIMIT", &[Arg::N(limit)]);
    }
    if *fear {
        line(b, d, "TXT_KEY_PROMOTION_FEAR_PEDIA", &[]);
    }
    if *free_xp_per_turn != 0 {
        line(b, d, "TXT_KEY_PROMOTION_FREE_XP_PER_TURN", &[Arg::N(*free_xp_per_turn), Arg::N(d.defines.free_xp_max)]);
    }
    for (n, key) in [
        (*free_xp_from_combat, "TXT_KEY_PROMOTION_FREE_XP_FROM_COMBAT"),
        (*gold_from_combat, "TXT_KEY_UNIT_GOLD_FROM_COMBAT"),
        (*modify_global_counter, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER"),
        (*modify_global_counter_on_combat, "TXT_KEY_MESSAGE_MODIFY_GLOBAL_COUNTER_ON_COMBAT"),
    ] {
        if n != 0 {
            line(b, d, key, &[Arg::N(n)]);
        }
    }
    if *spell_caster_xp != 0 {
        line(b, d, "TXT_KEY_PROMOTION_SPELL_CASTER_XP", &[]);
    }
    for (has, key) in [
        (*flying, "TXT_KEY_PROMOTION_FLYING_PEDIA"),
        (*held, "TXT_KEY_PROMOTION_HELD_PEDIA"),
        (*hidden_nationality, "TXT_KEY_PROMOTION_HIDDEN_NATIONALITY_PEDIA"),
        (*ignore_building_defense, "TXT_KEY_PROMOTION_IGNORE_BUILDING_DEFENSE"),
        (*immortal, "TXT_KEY_PROMOTION_IMMORTAL_PEDIA"),
        (*immune_to_capture, "TXT_KEY_PROMOTION_IMMUNE_TO_CAPTURE_PEDIA"),
        (*immune_to_magic, "TXT_KEY_PROMOTION_IMMUNE_TO_MAGIC_PEDIA"),
        (*immune_to_fear, "TXT_KEY_PROMOTION_IMMUNE_TO_FEAR_PEDIA"),
    ] {
        if has {
            line(b, d, key, &[]);
        }
    }
    for q in [promotion_immune1, promotion_immune2, promotion_immune3].into_iter().flatten() {
        line(b, d, "TXT_KEY_PROMOTION_IMMUNE", &[Arg::S(&display(d, *q))]);
    }
    for (has, key) in [
        (*invisible, "TXT_KEY_PROMOTION_INVISIBLE_PEDIA"),
        (*see_invisible, "TXT_KEY_PROMOTION_SEE_INVISIBLE_PEDIA"),
        (*double_fortify_bonus, "TXT_KEY_PROMOTION_DOUBLE_FORTIFY_BONUS"),
        (*twincast, "TXT_KEY_PROMOTION_TWINCAST_PEDIA"),
        (*water_walking, "TXT_KEY_PROMOTION_WATER_WALKING_PEDIA"),
        (*not_alive, "TXT_KEY_PROMOTION_NOT_ALIVE"),
    ] {
        if has {
            line(b, d, key, &[]);
        }
    }
    // This game's card (docs/CARDS.md, False Colours).
    if *false_colours {
        b.push_str(NEWLINE);
        b.push_str("[ICON_BULLET]May attack units of a civ you are at peace with, without a declaration: war follows at the battle, whose defenders get no defensive strikes and no fortification. Ends after its first battle, or at the end of the turn");
    }
    for (n, key) in [
        (*resist_magic, "TXT_KEY_PROMOTION_RESIST"),
        (*caster_resist_modify, "TXT_KEY_PROMOTION_CASTER_RESIST_MODIFY"),
    ] {
        if n != 0 {
            line(b, d, key, &[Arg::N(n)]);
        }
    }
    // The spells it allows (those whose first promotion it is; not the pedia's hidden ones).
    for s in d.spells.keys() {
        let spell = &d.spells[s];
        if spell.promotion_prereq1 != Some(p) || spell.base.graphical_only {
            continue;
        }
        match spell.promotion_prereq2 {
            None => line(b, d, "TXT_KEY_PROMOTION_ALLOWS_TEXT", &[Arg::S(&display(d, s))]),
            Some(q) => line(b, d, "TXT_KEY_PROMOTION_ALLOWS_WITH_TEXT", &[Arg::S(&display(d, s)), Arg::S(&display(d, q))]),
        }
    }
    if *expire_chance != 0 {
        line(b, d, "TXT_KEY_PROMOTION_EXPIRE_CHANCE", &[Arg::N(*expire_chance)]);
    }
    for (has, key) in [
        (*removed_by_casting, "TXT_KEY_PROMOTION_REMOVED_BY_CASTING"),
        (*removed_by_combat, "TXT_KEY_PROMOTION_REMOVED_BY_COMBAT"),
        (*removed_when_healed, "TXT_KEY_PROMOTION_REMOVED_WHEN_HEALED"),
    ] {
        if has {
            line(b, d, key, &[]);
        }
    }
    if let Some(q) = promotion_summon_perk {
        line(b, d, "TXT_KEY_PROMOTION_SUMMON_PERK", &[Arg::S(&display(d, *q))]);
    }
    // C++ reads the promotion it fights better against without checking it is set (none is
    // missing in FFH2's data): nothing then.
    if let Some(q) = promotion_combat_type.filter(|_| *promotion_combat_mod != 0) {
        line(b, d, "TXT_KEY_PROMOTION_VERSUS_TEXT", &[Arg::N(*promotion_combat_mod), Arg::S(&display(d, q))]);
    }
    if *betrayal_chance != 0 {
        line(b, d, "TXT_KEY_PROMOTION_BETRAYAL_CHANCE", &[Arg::N(*betrayal_chance)]);
    }
    if *work_rate_modify != 0 {
        line(b, d, "TXT_KEY_PROMOTION_WORK_RATE_MODIFY", &[Arg::N(*work_rate_modify)]);
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
    let chosen_by_none = *min_level == -1 || *race || *equipment || tech_prereq.is_some_and(|t| d.techs[t].disable && d.techs[t].card.is_none()) || !unit_combat.iter().any(|&c| c);
    if chosen_by_none {
        return finish(std::mem::take(b));
    }
    let and_ = txt(d, "TXT_KEY_AND", &[]);
    let or_ = txt(d, "TXT_KEY_OR", &[]);
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
        b.push_str(&req(d, "TXT_KEY_REQUIRES", &[]));
        b.push_str(&needs);
    }
    if let Some(t) = tech_prereq {
        line_req(b, d, "TXT_KEY_UNIT_REQUIRES_STRING", &[Arg::S(&display(d, *t))]);
    }
    if let Some(r) = state_religion_prereq {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_TECH_PREREQ_RELIGION", &[Arg::S(&display(d, *r))]));
    }
    // The player must have the resource (C++ `CvPlayer::hasBonus`).
    if let Some(bonus) = bonus_prereq {
        line_req(b, d, "TXT_KEY_UNIT_REQUIRES_STRING", &[Arg::S(&display(d, *bonus))]);
    }
    if *min_level > 0 {
        line_req(b, d, "TXT_KEY_BUILDING_REQUIRES_UNIT_LEVEL", &[Arg::N(*min_level)]);
    }
    if *prereq_alive {
        b.push_str(NEWLINE);
        b.push_str("Requires a living unit");
    }
    let combats: Vec<String> = d.unit_combats.keys().filter(|&uc| unit_combat[uc]).map(|uc| display(d, uc).into_owned()).collect();
    if !combats.is_empty() {
        b.push_str(NEWLINE);
        b.push_str(&format!("{}: {}", txt(d, "TXT_KEY_PEDIA_PROMOTION_UNITS", &[]), combats.join(", ")));
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
        for part in ["[ICON_BULLET]+20% Attack", "[ICON_BULLET]+20% Defense", "[ICON_BULLET]+10% Magic", "[ICON_BULLET]+20% Magic defense", "[ICON_BULLET]+5 spell damage outside battles", "Grants the Empower I promotion to summoned units", "Available To: Recon Units, Archery Units, ", "Beast Units"] {
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
