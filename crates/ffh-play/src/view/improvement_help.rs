//! What a unique improvement does, as Civ4 markup for the page (`effects` on the unique
//! `ImprovementType` entries of `data`; the map tooltip shows it).
//!
//! First the lines of C++ `CvGameTextMgr::setImprovementHelp` that describe a placed improvement
//! (its yields, defence, happiness, heal rate, the strength it gives around it, visibility, the
//! barbarians it spawns and its XML help; not what concerns building it or its upgrade), then
//! what FFH2 does in Python and never describes: the resource it puts on its plot, the spells cast
//! on it and its on-move and at-range callbacks. Those descriptions are written here from the
//! ports in `ffh-core/src/ffh/`; a test checks that every callback a unique improvement's XML
//! names has one (or is listed as having nothing to say), so a new one is not left out silently.

use ffh_data::*;

use crate::names;
use crate::view::help::{one_line, txt, Arg};

/// What a Python callback named by a unique improvement, or by a spell cast on one, does:
/// `Some(line)`, `Some("")` when there is nothing to say (worldbuilder scenarios only), `None` for
/// a callback not listed here.
fn python_effect(name: &str) -> Option<&'static str> {
    Some(match name {
        // `at_range_guardian`: once, while the plot is empty, for a non-barbarian unit.
        "atRangeGuardian" => "[ICON_BULLET]Three Gargoyles awaken when a unit first comes next to it (not in the game's first 20 turns)",
        // `on_move_letum_frigus`: human players only, then the plot's Python is off.
        "onMoveLetumFrigus" => "[ICON_BULLET]Your first unit to enter it brings the Letum Frigus event, once per game (one choice grants the Aggressive trait)",
        // `on_move_maelstrom`: `rand(100) <= 25` sinks; else an ocean plot, unowned first, damage 25.
        "onMoveMaelstrom" => "[ICON_BULLET]A ship that enters it sinks (26%) or is flung to a random ocean tile at 75% health",
        // The improvement's help names the other three cures.
        "onMovePoolOfTears" => "[ICON_BULLET]Cures Withered",
        // `spell_explore_lair_epic`: `rand(100) + level < 54` is a big bad result, else a big good one.
        "spellExploreLairEpic" => "a unit here may explore it, for a great reward or a great danger (54% danger, 1% less per level of the unit)",
        // `spellSacrificePyre`: the spell's help (`TXT_KEY_SPELL_SACRIFICE_HELP`, research) is
        // another spell's.
        "spellSacrificePyre" => "the unit gives its life for a Ring of Flames and a Fire Elemental (seven for a world unit)",
        "onMoveMirrorOfHeaven" => "",
        _ => return None,
    })
}

/// The name of a Python call (`onMoveMaelstrom(pCaster, pPlot)` -> `onMoveMaelstrom`).
fn call_name(src: &str) -> &str {
    src.split('(').next().unwrap_or("").trim()
}

/// The effects of improvement `i` (Civ4 markup, lines joined by `[NEWLINE]`; empty when there are
/// none). Written for the unique improvements: the callbacks of others may go undescribed.
pub fn effects(d: &GameData, i: ImprovementType) -> String {
    const YIELD_ICONS: [&str; YieldType::NUM] = ["[ICON_FOOD]", "[ICON_PRODUCTION]", "[ICON_COMMERCE]"];
    let info = &d.improvements[i];
    let mut lines: Vec<String> = Vec::new();
    // `setYieldChangeHelp` on the title line ("Mirror of Heaven, +3⚒"): a line of its own here.
    let yields: Vec<String> = (0..YieldType::NUM).filter(|&y| info.yield_change[y] != 0).map(|y| format!("{:+}{}", info.yield_change[y], YIELD_ICONS[y])).collect();
    if !yields.is_empty() {
        lines.push(format!("[ICON_BULLET]{}", yields.join(", ")));
    }
    if info.defense_modifier != 0 {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_DEFENSE_MODIFIER", &[Arg::N(info.defense_modifier)]));
    }
    if info.happiness != 0 {
        let icon = if info.happiness > 0 { "[ICON_HAPPY]" } else { "[ICON_UNHAPPY]" };
        lines.push(txt(d, "TXT_KEY_MISC_ICON_CHANGE_NEARBY_CITIES", &[Arg::N(info.happiness), Arg::S(icon)]));
    }
    if info.acts_as_city {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_DEFENSE_MODIFIER_EXTRA", &[]));
    }
    if info.feature_growth_probability > 0 {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_MORE_GROWTH", &[]));
    } else if info.feature_growth_probability < 0 {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_LESS_GROWTH", &[]));
    }
    if info.unique {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_UNIQUE", &[]));
    }
    if let Some(c) = info.prereq_civilization {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_PREREQ_CIVILIZATION", &[Arg::S(&names::display(d, c))]));
    }
    if info.heal_rate_change != 0 {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_HEAL_RATE_CHANGE", &[Arg::N(info.heal_rate_change)]));
    }
    if info.range_defense_modifier != 0 {
        // The text signs the range too ("within +3 tiles"): passed as text, it reads "within 3".
        lines.push(txt(
            d,
            "TXT_KEY_IMPROVEMENT_RANGE_DEFENSE_MODIFIER",
            &[Arg::N(info.range_defense_modifier), Arg::S(&info.range.to_string())],
        ));
    }
    if info.visibility_change != 0 {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_VISIBILITY_CHANGE", &[Arg::N(info.visibility_change)]));
    }
    // A great-person card's key (docs/CARDS.md): the Trench.
    if info.engagement_attacker_slots_change != 0 {
        lines.push(format!("[ICON_BULLET]In battles on this plot: {:+} slots for the attacker (at least the minimum)", info.engagement_attacker_slots_change));
    }
    // A great-person card's keys (docs/CARDS.md): the Cairn.
    if info.engagement_defense != 0 {
        lines.push(format!("[ICON_BULLET]In battles on this plot: {:+}% defense for the defender's units", info.engagement_defense));
    }
    if info.battle_sentinels > 0 {
        lines.push(format!("[ICON_BULLET]In battles on this plot: the defender fights with {} Sentinels (grown by the plot's defense; whole each battle; they hold no plot)", info.battle_sentinels));
    }
    if let Some(u) = info.spawn_unit_type {
        lines.push(txt(d, "TXT_KEY_IMPROVEMENT_SPAWN_UNIT_TYPE", &[Arg::S(&names::display(d, u))]));
    }
    if let Some(u) = info.deep_spawn_unit_type {
        lines.push(format!("[ICON_BULLET]Once The Lairs Deepen has been played: spawns {} instead", names::display(d, u)));
    }
    if !info.base.help.is_empty() {
        let help = names::text(d, &info.base.help);
        if help != info.base.help && !help.trim().is_empty() {
            lines.push(help.to_string());
        }
    }

    // This game's cards (docs/CARDS.md, `ffh_core::ffh::cards::map_rules`).
    if info.land_bridge {
        lines.push("[ICON_BULLET]Built next to land. Land units move onto and over it as over land; ships cannot enter it".to_string());
    }
    if info.canal {
        lines.push("[ICON_BULLET]Built next to water or another canal. Ships can enter it, unless it is in the land of a civ at war with them; ships in it cannot defend it".to_string());
    }
    if info.enemy_city_adjacent {
        lines.push("[ICON_BULLET]Built only next to a city of a civ you are at war with, and then also in that civ's land".to_string());
    }
    if info.requires_route {
        lines.push("[ICON_BULLET]Built only on a plot with a road".to_string());
    }
    if let Some(t) = info.city_heal_tech {
        lines.push(format!("[ICON_BULLET]Units of the civ that knows {} heal here as in a city of their own", names::display(d, t)));
    }
    // Not in FFH2: the Trading Post card (docs/CARDS.md).
    if info.claim_range > 0 {
        lines.push("[ICON_BULLET]Built only outside every civilization's borders".to_string());
    }
    if info.min_city_distance > 0 {
        lines.push(format!("[ICON_BULLET]Built only {} plots or more from every city", info.min_city_distance));
    }
    if info.claim_range > 0 {
        let plots = (2 * info.claim_range + 1) * (2 * info.claim_range + 1) - 1;
        lines.push(format!("[ICON_BULLET]Its plot and the {plots} land plots around it are its builder's territory, whatever the culture, until it is pillaged"));
    }

    // Not in FFH2 (`tech_research_bonus`, applied by `player_calculate_research_modifier`).
    if let Some(t) = info.tech_research_bonus.filter(|_| info.tech_research_percent > 0) {
        lines.push(format!("[ICON_BULLET]Inside your borders: {:+}% [ICON_RESEARCH] on {}", info.tech_research_percent, names::display(d, t)));
    }

    // FFH2 says nothing of the rest. C++ `CvPlot::setImprovementType` puts the bonus on the plot.
    if let Some(b) = info.bonus_convert {
        lines.push(format!("[ICON_BULLET]Provides {}", names::display(d, b)));
    }
    for s in d.spells.keys().filter(|&s| d.spells[s].improvement_prereq == Some(i)) {
        let spell = &d.spells[s];
        let mut line = format!("[ICON_BULLET]{}", names::display(d, s));
        if let Some(p) = spell.promotion_prereq1 {
            line.push_str(&format!(" ({} units)", names::display(d, p)));
        }
        let what = match python_effect(call_name(&spell.py_result)) {
            Some(e) => e.to_string(),
            None if spell.base.help.is_empty() => String::new(),
            None => one_line(names::text(d, &spell.base.help)),
        };
        if !what.is_empty() {
            line.push_str(": ");
            line.push_str(&what);
        }
        lines.push(line);
    }
    for call in [&info.python_on_move, &info.python_at_range] {
        if let Some(e) = python_effect(call_name(call)).filter(|e| !e.is_empty()) {
            lines.push(e.to_string());
        }
    }
    // `ev_improvement_built` (the minimum level to enter is the plot's, in the `plot` query) and
    // Brigit's `postCombatBrigitHeld`.
    if names::name(d, i) == "IMPROVEMENT_RING_OF_CARCER" {
        lines.push("[ICON_BULLET]Brigit is held prisoner here: the unit that defeats her wins her and breaks the Ring".to_string());
    }
    lines.join("[NEWLINE]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> &'static GameData {
        ffh_data::global()
    }

    fn effects_of(name: &str) -> String {
        let d = data();
        effects(d, d.t::<ImprovementType>(name))
    }

    #[test]
    fn every_callback_of_a_unique_improvement_is_described() {
        let d = data();
        let unique = |i: Option<ImprovementType>| i.is_some_and(|i| d.improvements[i].unique);
        for i in d.improvements.keys().filter(|&i| unique(Some(i))) {
            let info = &d.improvements[i];
            for call in [&info.python_on_move, &info.python_at_range] {
                let name = call_name(call);
                assert!(name.is_empty() || python_effect(name).is_some(), "{}: describe {name} in python_effect", names::name(d, i));
            }
        }
        for s in d.spells.keys() {
            let spell = &d.spells[s];
            if unique(spell.improvement_prereq) && spell.base.help.is_empty() {
                let name = call_name(&spell.py_result);
                assert!(python_effect(name).is_some(), "{}: describe {name} in python_effect", names::name(d, s));
            }
        }
    }

    #[test]
    fn unique_improvements_say_what_they_do() {
        let d = data();
        for i in d.improvements.keys().filter(|&i| d.improvements[i].unique) {
            let e = effects(d, i);
            // More than "World Improvement".
            assert!(e.matches("[ICON_BULLET]").count() >= 2, "{}: {e}", names::name(d, i));
            assert!(!e.as_bytes().windows(3).any(|w| w[0] == b'%' && w[1].is_ascii_alphabetic() && w[2].is_ascii_digit()), "{}: {e}", names::name(d, i));
        }
        let mirror = effects_of("IMPROVEMENT_MIRROR_OF_HEAVEN");
        assert!(mirror.starts_with("[ICON_BULLET]+3[ICON_PRODUCTION][NEWLINE]"), "{mirror}");
        for part in ["+25% Tile Defense", "+1[ICON_HAPPY] in nearby cities", "+6 to unit visibility range", "World Improvement", "Provides "] {
            assert!(mirror.contains(part), "{part}: {mirror}");
        }
        assert!(effects_of("IMPROVEMENT_ODIOS_PRISON").contains("+25% strength to all units within 3 tiles"));
        let maelstrom = effects_of("IMPROVEMENT_MAELSTROM");
        assert!(maelstrom.contains("sinks (26%)"), "{maelstrom}");
        let pool = effects_of("IMPROVEMENT_POOL_OF_TEARS");
        assert!(pool.contains("Cures Plague") && pool.contains("Cures Withered") && pool.contains("+50% heal rate"), "{pool}");
        let pyre = effects_of("IMPROVEMENT_PYRE_OF_THE_SERAPHIC");
        assert!(pyre.contains("Explore Lair: a unit here may explore it") && pyre.contains("Fire Elemental"), "{pyre}");
        assert!(effects_of("IMPROVEMENT_SEVEN_PINES").contains("Halves the Armageddon Counter"));
        assert!(effects_of("IMPROVEMENT_RING_OF_CARCER").contains("Brigit"));
    }
}
