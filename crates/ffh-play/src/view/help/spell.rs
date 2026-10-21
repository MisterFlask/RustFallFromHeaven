//! What a spell does: C++ `CvGameTextMgr::parseSpellHelp` (FfH; its `pcNewline`, each line's
//! start, is the help's line break here), with the promotions the caster needs first, which
//! FFH2's help leaves to the pedia's buttons (Python `CvPediaSpell.placePrereqs`; the rules: C++
//! `CvUnit::canCast`).

use ffh_data::*;

use super::{finish, txt, Arg, NEWLINE};
use crate::names::{self, display};

/// One line of the help: C++ `szBuffer.append(pcNewline); szBuffer.append(gDLL->getText(..))`.
fn line(b: &mut String, d: &GameData, key: &str, fallback: &str, args: &[Arg]) {
    b.push_str(NEWLINE);
    b.push_str(&txt(d, key, fallback, args));
}

/// What spell `s` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is nothing to
/// say). Static: no caster and no player. Every field of the spell is either described here or
/// named as not shown, so that a new field cannot go unnoticed (the build fails until it is
/// listed).
pub fn effects(d: &GameData, s: SpellType) -> String {
    let SpellInfo {
        base,
        promotion_prereq1,
        promotion_prereq2,
        unit_prereq,
        unit_combat_prereq,
        unit_class_prereq,
        unit_in_stack_prereq,
        building_prereq,
        building_class_owned_prereq,
        civilization_prereq,
        corporation_prereq,
        feature_or_prereq1,
        feature_or_prereq2,
        improvement_prereq,
        promotion_in_stack_prereq,
        religion_prereq,
        state_religion_prereq,
        tech_prereq,
        // Whether the AI may cast it, and its weights.
        allow_ai: _,
        ai_weight: _,
        ai_weight_city: _,
        ai_weight_war: _,
        // UI: automatic casting (Sephi's addon), the greyed button when it cannot be cast, the
        // warning before a cast that may start a war (the unit panel shows it).
        allow_auto: _,
        allow_automate_terrain: _,
        display_when_disabled: _,
        causes_war: _,
        adjacent_to_water_only,
        // Read by the rules only, never described by FFH2: a living caster, a caster that is not
        // a temporary summon, the caster's one cast per turn (and the spells that ignore it), the
        // kind of its damage (resistances), an ability (cast even while spellcasting is
        // disabled), the units it pushes away, the population it adds (its XML help says so).
        caster_must_be_alive: _,
        caster_no_duration: _,
        has_casted: _,
        ignore_has_casted: _,
        damage_type: _,
        ability: _,
        push: _,
        change_population: _,
        global,
        in_borders_only,
        in_city_only,
        prereq_slave_trade,
        resistable,
        caster_min_level,
        range,
        resist_modify,
        damage,
        damage_limit,
        add_promotion_type1,
        add_promotion_type2,
        add_promotion_type3,
        remove_promotion_type1,
        remove_promotion_type2,
        remove_promotion_type3,
        buff_caster_only,
        convert_unit_type,
        create_building_type,
        create_feature_type,
        create_improvement_type,
        spread_religion,
        create_unit_type,
        create_unit_num,
        copy_casters_promotions,
        permanent_unit_create,
        create_unit_promotion,
        immune_team,
        immune_neutral,
        immune_enemy,
        immune_flying,
        immune_not_alive,
        dispel,
        remove_has_casted,
        sacrifice_caster,
        cost,
        delay,
        immobile_turns,
        miscast_chance,
        // Python callbacks: C++ shows only the XML help for them.
        py_miscast: _,
        py_result: _,
        py_requirement: _,
    } = &d.spells[s];
    let b = &mut String::new();

    // ffh-play's: the promotions the caster needs (C++ `canCast` checks each one that is set).
    // C++ names the spells a promotion allows in that promotion's help instead.
    match (promotion_prereq1, promotion_prereq2) {
        (Some(a), Some(c)) => {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]Requires the {} and {} promotions", display(d, *a), display(d, *c)));
        }
        (Some(a), None) | (None, Some(a)) => {
            b.push_str(NEWLINE);
            b.push_str(&format!("[ICON_BULLET]Requires the {} promotion", display(d, *a)));
        }
        (None, None) => {}
    }

    if *global {
        line(b, d, "TXT_KEY_SPELL_GLOBAL", "[ICON_BULLET]Can only be used once per game", &[]);
    }
    if *prereq_slave_trade {
        line(b, d, "TXT_KEY_SPELL_PREREQ_SLAVE_TRADE", "[ICON_BULLET]Player must have Slave Trade vote passed in the Undercouncil to use this ability", &[]);
    }
    if *caster_min_level != 0 {
        line(b, d, "TXT_KEY_SPELL_CASTER_MIN_LEVEL", "[ICON_BULLET]Caster must be at least %d1 level", &[Arg::N(*caster_min_level)]);
    }
    if let Some(x) = building_prereq {
        line(b, d, "TXT_KEY_SPELL_BUILDING_PREREQ", "[ICON_BULLET]Requires a [LINK=literal]%s1_building[\\LINK] building in the caster's city", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = building_class_owned_prereq {
        line(b, d, "TXT_KEY_SPELL_BUILDINGCLASS_OWNED_PREREQ", "[ICON_BULLET]The player must own the [LINK=literal]%s1_building[\\LINK] to use this spell", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = civilization_prereq {
        line(b, d, "TXT_KEY_SPELL_CIVILIZATION_PREREQ", "[ICON_BULLET]Can only be used by the [LINK=literal]%s1_civ[\\LINK] civilization", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = corporation_prereq {
        // No text has this key (no FFH2 spell needs a corporation): the line is written here, as
        // the rules check it (the corporation in the caster's city).
        line(b, d, "TXT_KEY_SPELL_CORPORATION_PREREQ", "[ICON_BULLET]Requires the [LINK=literal]%s1_corporation[\\LINK] corporation in the caster's city", &[Arg::S(&display(d, *x))]);
    }
    if let Some(f) = feature_or_prereq1 {
        match feature_or_prereq2 {
            None => line(b, d, "TXT_KEY_SPELL_FEATURE_PREREQ", "[ICON_BULLET]Requires a [LINK=literal]%s1_feature[\\LINK] in the casters tile", &[Arg::S(&display(d, *f))]),
            Some(g) => line(
                b,
                d,
                "TXT_KEY_SPELL_FEATURE2_PREREQ",
                "[ICON_BULLET]Requires a [LINK=literal]%s1_feature[\\LINK] or a [LINK=literal]%s2_feature[\\LINK] in the casters tile",
                &[Arg::S(&display(d, *f)), Arg::S(&display(d, *g))],
            ),
        }
    }
    if let Some(x) = improvement_prereq {
        line(b, d, "TXT_KEY_SPELL_IMPROVEMENT_PREREQ", "[ICON_BULLET]Requires a [LINK=literal]%s1_improvement[\\LINK] in the casters tile", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = promotion_in_stack_prereq {
        line(b, d, "TXT_KEY_SPELL_PROMOTION_IN_STACK_PREREQ", "[ICON_BULLET]Requires a unit with [LINK=literal]%s1_prom[\\LINK] promotion in the casters stack", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = religion_prereq {
        line(b, d, "TXT_KEY_SPELL_RELIGION_PREREQ", "[ICON_BULLET]Caster must have the [LINK=literal]%s1_religion[\\LINK] religion", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = state_religion_prereq {
        line(b, d, "TXT_KEY_SPELL_STATE_RELIGION_PREREQ", "[ICON_BULLET]Player must have the [LINK=literal]%s1_religion[\\LINK] state religion", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = tech_prereq {
        line(b, d, "TXT_KEY_SPELL_TECH_PREREQ", "[ICON_BULLET]Requires the [LINK=literal]%s1_tech[\\LINK] technology", &[Arg::S(&display(d, *x))]);
    }
    // The unit, unit combat or unit class that casts it: C++ says all three in the unit's words
    // ("Can only be used by a Naval Units.").
    const UNIT_PREREQ: &str = "[ICON_BULLET]Can only be used by a [LINK=literal]%s1_unit[\\LINK].";
    if let Some(x) = unit_prereq {
        line(b, d, "TXT_KEY_SPELL_UNIT_PREREQ", UNIT_PREREQ, &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = unit_combat_prereq {
        // C++ reuses the unit's key ("by a Naval Units."): a unit combat is plural.
        b.push_str(NEWLINE);
        b.push_str(&format!("[ICON_BULLET]Can only be used by [LINK=literal]{}[\\LINK].", display(d, *x)));
    }
    if let Some(x) = unit_class_prereq {
        line(b, d, "TXT_KEY_SPELL_UNIT_PREREQ", UNIT_PREREQ, &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = unit_in_stack_prereq {
        line(b, d, "TXT_KEY_SPELL_UNIT_IN_STACK_PREREQ", "[ICON_BULLET]Can only be used if there is a [LINK=literal]%s1_unit[\\LINK] unit in the stack.", &[Arg::S(&display(d, *x))]);
    }

    // What it does.
    if let Some(u) = create_unit_type {
        let unit = display(d, *u);
        if *create_unit_num == 1 {
            line(b, d, "TXT_KEY_SPELL_SUMMON_UNIT", "[ICON_BULLET]Summons a [LINK=literal]%s1_unit[\\LINK].", &[Arg::S(&unit)]);
        } else {
            line(b, d, "TXT_KEY_SPELL_SUMMON_UNIT_MULTIPLE", "[ICON_BULLET]Summons %D1 [LINK=literal]%s2_unitname[\\LINK](s).", &[Arg::N(*create_unit_num), Arg::S(&unit)]);
        }
        if *permanent_unit_create {
            // On the same line.
            b.push_str(&txt(d, "TXT_KEY_SPELL_SUMMON_UNIT_PERMANENT", " (permanent)", &[]));
        }
        if *copy_casters_promotions {
            line(b, d, "TXT_KEY_SPELL_COPY_CASTERS_PROMOTIONS", "[ICON_BULLET]Unit gains all the promotions of the caster", &[]);
        }
    }
    if let Some(x) = create_unit_promotion {
        line(b, d, "TXT_KEY_SPELL_CREATE_UNIT_PROMOTION", "[ICON_BULLET]Summoned units start with the [LINK=literal]%s1_promotion[\\LINK] promotion", &[Arg::S(&display(d, *x))]);
    }
    if *damage != 0 {
        line(b, d, "TXT_KEY_SPELL_DAMAGE", "[ICON_BULLET]Damages units about %D1%, to a limit of %D2% damage.", &[Arg::N(*damage), Arg::N(*damage_limit)]);
    }
    // The promotions it adds and removes (the second and third only with the first), on the
    // caster alone, its stack, or the units in range.
    if let Some(first) = add_promotion_type1 {
        for q in [Some(*first), *add_promotion_type2, *add_promotion_type3].into_iter().flatten() {
            let name = display(d, q);
            if *buff_caster_only {
                line(b, d, "TXT_KEY_SPELL_ADD_PROMOTION_CASTER", "[ICON_BULLET]Adds the [LINK=literal]%s1_promotion[\\LINK] promotion to the caster", &[Arg::S(&name)]);
            } else if *range == 0 {
                line(b, d, "TXT_KEY_SPELL_ADD_PROMOTION", "[ICON_BULLET]Adds the [LINK=literal]%s1_promotion[\\LINK] promotion to all units in the caster's stack", &[Arg::S(&name)]);
            } else {
                line(b, d, "TXT_KEY_SPELL_ADD_PROMOTION_AT_RANGE", "[ICON_BULLET]Adds the [LINK=literal]%s1_promotion[\\LINK] promotion to units within %d2 range", &[Arg::S(&name), Arg::N(*range)]);
            }
        }
    }
    if let Some(first) = remove_promotion_type1 {
        for q in [Some(*first), *remove_promotion_type2, *remove_promotion_type3].into_iter().flatten() {
            let name = display(d, q);
            if *buff_caster_only {
                line(b, d, "TXT_KEY_SPELL_REMOVE_PROMOTION_CASTER", "[ICON_BULLET]Removes the [LINK=literal]%s1_promotion[\\LINK] promotion from the caster", &[Arg::S(&name)]);
            } else {
                line(b, d, "TXT_KEY_SPELL_REMOVE_PROMOTION", "[ICON_BULLET]Removes the [LINK=literal]%s1_promotion[\\LINK] promotion", &[Arg::S(&name)]);
            }
        }
    }
    // The range, when the promotions it adds did not say it.
    if *range != 0 && add_promotion_type1.is_none() {
        line(b, d, "TXT_KEY_SPELL_RANGE", "[ICON_BULLET]Range: %D1", &[Arg::N(*range)]);
    }
    if let Some(x) = convert_unit_type {
        line(b, d, "TXT_KEY_SPELL_CONVERT_UNIT", "[ICON_BULLET]Transforms the caster into a %s1_unit", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = create_building_type {
        line(b, d, "TXT_KEY_SPELL_CREATE_BUILDING", "[ICON_BULLET]Creates a [LINK=literal]%s1_building[\\LINK] in the city", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = create_feature_type {
        line(b, d, "TXT_KEY_SPELL_CREATE_FEATURE", "[ICON_BULLET]Creates a [LINK=literal]%s1_feature[\\LINK] in the tile", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = create_improvement_type {
        line(b, d, "TXT_KEY_SPELL_CREATE_IMPROVEMENT", "[ICON_BULLET]Creates a [LINK=literal]%s1_improvement[\\LINK] in the tile", &[Arg::S(&display(d, *x))]);
    }
    if let Some(x) = spread_religion {
        line(b, d, "TXT_KEY_SPELL_SPREAD_RELIGION", "[ICON_BULLET]Spreads [LINK=literal]%s1_religion[\\LINK] religion", &[Arg::S(&display(d, *x))]);
    }
    if *dispel {
        line(b, d, "TXT_KEY_SPELL_DISPEL", "[ICON_BULLET]Removes beneficial spell effects from enemy units, and negative spell effects from allied units", &[]);
    }
    // The XML help (in the middle, as C++ puts it).
    let help = names::text(d, &base.help);
    if !base.help.is_empty() && help != base.help && !help.trim().is_empty() {
        b.push_str(NEWLINE);
        b.push_str(help);
    }

    // Who it affects and where it can be cast.
    if *resistable {
        if *resist_modify != 0 {
            line(b, d, "TXT_KEY_SPELL_RESIST_WITH_MODIFY", "[ICON_BULLET]Can be resisted (%D1%)", &[Arg::N(*resist_modify)]);
        } else {
            line(b, d, "TXT_KEY_SPELL_RESIST", "[ICON_BULLET]Can be resisted", &[]);
        }
    }
    if *adjacent_to_water_only {
        line(b, d, "TXT_KEY_SPELL_ADJACENT_TO_WATER_ONLY", "[ICON_BULLET]Can only be cast next to water tiles", &[]);
    }
    if *in_borders_only && *in_city_only {
        line(b, d, "TXT_KEY_SPELL_IN_BORDERS_AND_CITY_ONLY", "[ICON_BULLET]Can only be cast in cities you own", &[]);
    } else {
        if *in_borders_only {
            line(b, d, "TXT_KEY_SPELL_IN_BORDERS_ONLY", "[ICON_BULLET]Can only be cast within your civilizations borders", &[]);
        }
        if *in_city_only {
            line(b, d, "TXT_KEY_SPELL_IN_CITY_ONLY", "[ICON_BULLET]Can only be cast in cities", &[]);
        }
    }
    let immune = match (*immune_team, *immune_neutral, *immune_enemy) {
        (true, false, false) => Some(("TXT_KEY_SPELL_IMMUNE_TEAM", "[ICON_BULLET]Won't affect units on your team")),
        (false, true, false) => Some(("TXT_KEY_SPELL_IMMUNE_NEUTRAL", "[ICON_BULLET]Won't affect neutral units")),
        (false, false, true) => Some(("TXT_KEY_SPELL_IMMUNE_ENEMY", "[ICON_BULLET]Won't affect enemy units")),
        (true, true, false) => Some(("TXT_KEY_SPELL_IMMUNE_TEAM_NEUTRAL", "[ICON_BULLET]Only affects enemy units")),
        (true, false, true) => Some(("TXT_KEY_SPELL_IMMUNE_TEAM_ENEMY", "[ICON_BULLET]Only affects neutral units")),
        (false, true, true) => Some(("TXT_KEY_SPELL_IMMUNE_NEUTRAL_ENEMY", "[ICON_BULLET]Only affects units on your team")),
        (true, true, true) => Some(("TXT_KEY_SPELL_IMMUNE_TEAM_NEUTRAL_ENEMY", "[ICON_BULLET]Won't affect team, neutral or enemy units")),
        (false, false, false) => None,
    };
    if let Some((key, fallback)) = immune {
        line(b, d, key, fallback, &[]);
    }
    if *immune_flying {
        line(b, d, "TXT_KEY_SPELL_IMMUNE_FLYING", "[ICON_BULLET]Won't affect flying units", &[]);
    }
    if *immune_not_alive {
        line(b, d, "TXT_KEY_SPELL_IMMUNE_NOT_ALIVE", "[ICON_BULLET]Only affects living targets", &[]);
    }
    if *remove_has_casted {
        line(b, d, "TXT_KEY_SPELL_REMOVE_HAS_CASTED", "[ICON_BULLET]Restores a caster's arcane energy", &[]);
    }
    // Its price: C++ adds the active player's upgrade cost modifier to a conversion's cost; no
    // player here, so the base cost.
    if *cost != 0 {
        line(b, d, "TXT_KEY_SPELL_COST", "[ICON_BULLET]Costs %d1 gold", &[Arg::N(*cost)]);
    }
    if *delay != 0 {
        line(b, d, "TXT_KEY_SPELL_DELAY", "[ICON_BULLET]Takes %d1 turns to cast", &[Arg::N(*delay)]);
    }
    if *miscast_chance != 0 {
        line(b, d, "TXT_KEY_SPELL_MISCAST_CHANCE", "[ICON_BULLET]%D1% to be miscast", &[Arg::N(*miscast_chance)]);
    }
    if *immobile_turns != 0 {
        line(b, d, "TXT_KEY_SPELL_IMMOBILE_TURNS", "[ICON_BULLET]Units become immobile for %d1 turns", &[Arg::N(*immobile_turns)]);
    }
    if *sacrifice_caster {
        line(b, d, "TXT_KEY_SPELL_SACRIFICE_CASTER", "[ICON_BULLET]Casting this spell kills the caster", &[]);
    }
    finish(std::mem::take(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<SpellType>(name))
    }

    fn has_placeholder(t: &str) -> bool {
        t.as_bytes().windows(3).any(|w| w[0] == b'%' && matches!(w[1], b'd' | b'D' | b's' | b'F') && w[2].is_ascii_digit())
    }

    #[test]
    fn known_spells_say_what_they_do() {
        let haste = of("SPELL_HASTE");
        assert!(haste.starts_with("[ICON_BULLET]Requires the Body I and Channeling I promotions[NEWLINE]"), "{haste}");
        for part in [
            "[ICON_BULLET]Adds the [LINK=literal]Hasted[\\LINK] promotion to all units in the caster's stack",
            "[ICON_BULLET]Removes the [LINK=literal]Slow[\\LINK] promotion",
            "[ICON_BULLET]Only affects units on your team",
            "[ICON_BULLET]Only affects living targets",
        ] {
            assert!(haste.contains(part), "{part}: {haste}");
        }

        let fireball = of("SPELL_FIREBALL");
        assert_eq!(fireball, "[ICON_BULLET]Requires the Fire II promotion[NEWLINE][ICON_BULLET]Summons a [LINK=literal]Fireball[\\LINK].");
        // A permanent summon says so on its line.
        let skeleton = of("SPELL_RAISE_SKELETON");
        assert!(skeleton.contains("[ICON_BULLET]Summons a [LINK=literal]Skeleton[\\LINK]. (permanent)"), "{skeleton}");
        let balor = of("SPELL_SUMMON_BALOR");
        assert!(balor.contains("Requires the Channeling III and Divine promotions"), "{balor}");
        assert!(balor.contains("Caster must have the [LINK=literal]"), "{balor}");

        // A world spell: once, one civilization, a tech, then its XML help.
        let seas = of("SPELL_RAGING_SEAS");
        for part in [
            "[ICON_BULLET]Can only be used once per game",
            "[ICON_BULLET]Can only be used by the [LINK=literal]Lanun[\\LINK] civilization",
            "[ICON_BULLET]Requires the [LINK=literal]Optics[\\LINK] technology",
        ] {
            assert!(seas.contains(part), "{part}: {seas}");
        }
        let d = ffh_data::global();
        let help = names::text(d, &d.spells[d.t::<SpellType>("SPELL_RAGING_SEAS")].base.help);
        assert!(seas.ends_with(help), "{seas}");

        let maelstrom = of("SPELL_MAELSTROM");
        assert!(maelstrom.contains("[ICON_BULLET]Damages units about +15%, to a limit of +30% damage."), "{maelstrom}");
        assert!(maelstrom.contains("[ICON_BULLET]Range: +2"), "{maelstrom}");
    }

    #[test]
    fn every_spell_reads_without_placeholders_and_says_something() {
        let d = ffh_data::global();
        for s in d.spells.keys() {
            let e = effects(d, s);
            let name = names::name(d, s);
            assert!(!has_placeholder(&e), "{name}: {e}");
            assert!(!e.starts_with(NEWLINE) && !e.contains("TXT_KEY_"), "{name}: {e}");
            // Every spell has a requirement, an effect or an XML help to show.
            assert!(!e.is_empty(), "{name} says nothing");
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        for name in ["SPELL_HASTE", "SPELL_FIREBALL", "SPELL_RAISE_SKELETON", "SPELL_SUMMON_BALOR", "SPELL_RAGING_SEAS", "SPELL_MAELSTROM", "SPELL_DISPEL_MAGIC"] {
            println!("{name}:\n{}\n", of(name));
        }
    }
}
