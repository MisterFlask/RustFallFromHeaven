//! The odds of exploring a lair or entering a goody hut, for the plot query (`explore`): what
//! each result is, its chance for the selected unit, and what happens to the lair after it
//! (`ffh_core::ffh::lairs`). Labels follow the rules-text rule: bare, mechanical.

use ffh_core::ffh::lairs::{LairTier, BIG_BAD_MONSTER};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::names;
use crate::view::Viewer;

/// The `explore` member of the plot query for plot `p`, or `None` when it holds neither a lair
/// the viewer knows of nor a goody hut. `units`: the selection; the first of them that belongs
/// to the viewer is the unit the odds are for (it explores, or enters, first).
pub fn explore_json(v: &Viewer, p: PlotIdx, units: &[UnitId]) -> Option<Json> {
    let g = v.g;
    // What the viewer knows is there (a fogged plot as last seen).
    let known = v.plot_improvement(p)?;
    let mine: Vec<UnitId> = units.iter().copied().filter(|&u| g.units.get(u).is_some_and(|unit| unit.owner == v.h)).collect();
    if g.d.improvements[known].goody {
        // (The hut's odds read the unit, the player and the cities near the plot, not the plot.)
        return Some(hut_json(v, p, mine.first().copied()));
    }
    let epic = g.lair_explore_kind_of(known)?;
    // The outcome lists read the plot as it is: only while the viewer sees it. A fogged lair
    // gets the tiers, which depend on the unit alone.
    let current = v.plot_visible(p) && g.map.plots[p].improvement_type == Some(known);
    Some(lair_json(v, p, epic, &mine, current))
}

fn ppm_json(ppm: i64) -> Json {
    Json::from(ppm)
}

fn tiers_json(tiers: &[(LairTier, i64)]) -> Json {
    Json::Arr(tiers.iter().map(|&(t, ppm)| crate::jobj! { "tier" => t.key(), "ppm" => ppm_json(ppm) }).collect())
}

fn lair_json(v: &Viewer, p: PlotIdx, epic: bool, units: &[UnitId], current: bool) -> Json {
    let g = v.g;
    let d = g.d;
    let mut j = crate::jobj! { "kind" => "lair", "epic" => epic, "in_view" => current };
    let Some(&first) = units.first() else {
        // No unit: the tiers for a level-1 unit, without the unit's lists.
        j.set("unit", Json::Null);
        j.set("level", 1);
        j.set("tiers", tiers_json(&level_tiers(1, epic)));
        return j;
    };
    if !current {
        j.set("unit", first);
        j.set("unit_type", Json::idx(g.units[first].unit_type));
        j.set("level", g.units[first].level);
        j.set("tiers", tiers_json(&level_tiers(g.units[first].level, epic)));
        return j;
    }
    let odds = g.lair_explore_odds(first, p).expect("an explorable lair");
    j.set("unit", first);
    j.set("unit_type", Json::idx(g.units[first].unit_type));
    j.set("level", odds.level);
    j.set("tiers", tiers_json(&odds.tiers));
    j.set("lair_survives_ppm", ppm_json(odds.lair_survives_ppm));
    j.set("grace_ppm", ppm_json(odds.grace_ppm));
    let outcomes: Vec<Json> = odds
        .outcomes
        .iter()
        .map(|o| {
            crate::jobj! {
                "tier" => o.tier.key(),
                "key" => o.key,
                "label" => lair_label(g, v.h, o.tier, o.key),
                "ppm" => ppm_json(o.ppm),
                "destroy_pct" => o.destroy_pct,
                "replaces_lair" => ffh_core::ffh::lairs::lair_outcome_replaces_lair(o.tier, o.key),
            }
        })
        .collect();
    j.set("outcomes", Json::Arr(outcomes));
    let names_of = |list: &[&'static str], unit: bool| -> Json {
        Json::Arr(
            list.iter()
                .map(|n| if unit { Json::from(names::display(d, d.t::<UnitType>(n)).into_owned()) } else { Json::from(names::display(d, d.t::<PromotionType>(n)).into_owned()) })
                .collect(),
        )
    };
    j.set(
        "big_bad",
        crate::jobj! {
            "monsters" => names_of(&odds.big_bad.monsters, true),
            "henchmen" => names_of(&odds.big_bad.henchmen, true),
            "promotions" => names_of(&odds.big_bad.promotions, false),
        },
    );
    // The other selected units, each its own roll (every selected unit casts it in turn).
    let others: Vec<Json> = units[1..]
        .iter()
        .take(8)
        .map(|&u| crate::jobj! { "unit" => u, "unit_type" => Json::idx(g.units[u].unit_type), "level" => g.units[u].level, "tiers" => tiers_json(&level_tiers(g.units[u].level, epic)) })
        .collect();
    j.set("others", Json::Arr(others));
    j
}

/// The tier odds of a unit of `level` (the lists aside).
fn level_tiers(level: i32, epic: bool) -> Vec<(LairTier, i64)> {
    let mut out: Vec<(LairTier, i64)> = Vec::new();
    for r in 0..100 {
        let tier = LairTier::of_roll(r + level, epic);
        match out.iter_mut().find(|t| t.0 == tier) {
            Some(t) => t.1 += 10_000,
            None => out.push((tier, 10_000)),
        }
    }
    out.sort_by_key(|t| t.0);
    out
}

fn hut_json(v: &Viewer, p: PlotIdx, unit: Option<UnitId>) -> Json {
    let g = v.g;
    let odds = g.goody_hut_odds(v.h, p, unit);
    let outcomes: Vec<Json> = odds
        .outcomes
        .iter()
        .map(|o| {
            crate::jobj! {
                "goody" => Json::idx(o.goody),
                "label" => goody_label(g, v.h, o.goody),
                "ppm" => ppm_json(o.ppm),
                "slots" => o.slots,
                "allowed" => o.allowed,
            }
        })
        .collect();
    crate::jobj! {
        "kind" => "hut",
        "unit" => unit.map_or(Json::Null, Json::from),
        "unit_type" => unit.map_or(Json::Null, |u| Json::idx(g.units[u].unit_type)),
        "outcomes" => Json::Arr(outcomes),
        "slots_total" => odds.slots_total,
        "attempts" => odds.attempts,
        "nothing_ppm" => ppm_json(odds.nothing_ppm),
    }
}

/// What a goody gives, in a line: `Gold: 20-60`, `Unit: Scout`, `Barbarians: 2+ Warrior`.
pub fn goody_label(g: &Game, p: PlayerId, goody: GoodyType) -> String {
    let d = g.d;
    // The unit a class gives a player (`receiveGoody`: the civilization's own unit of the class).
    let civ_unit = |player: PlayerId, class: UnitClassType| g.player_get_civilization_type(player).and_then(|c| d.civ_unit(c, class));
    let unit_name = |unit: Option<UnitType>, class: UnitClassType| unit.map_or_else(|| names::display(d, class).into_owned(), |u| names::display(d, u).into_owned());
    let info = &d.goodies[goody];
    let mut parts: Vec<String> = Vec::new();
    if let Some((min, max)) = g.goody_gold_range(goody) {
        parts.push(format!("Gold: {min}-{max}"));
    }
    if info.map_range > 0 {
        parts.push(format!("Map: reveals {}% of the plots within {} of a point up to {} away", info.map_prob, info.map_range, info.map_offset));
    }
    if info.tech {
        parts.push("Tech: a random one you can research".to_string());
    }
    if let Some(class) = info.unit_class_type {
        let name = unit_name(civ_unit(p, class).or(d.unit_classes[class].default_unit_index), class);
        let kind = if d.unit_classes[class].base.type_.starts_with("EQUIPMENTCLASS_") { "Item" } else { "Unit" };
        parts.push(format!("{kind}: {name}"));
    }
    if let Some(class) = info.barbarian_unit_class {
        let name = unit_name(civ_unit(g.barbarian_player(), class), class);
        let min = info.min_barbarians.max(1);
        parts.push(format!("Barbarians: {min}+ {name}"));
    }
    if info.experience > 0 {
        parts.push(format!("Experience: +{}", info.experience));
    }
    if info.healing > 0 {
        parts.push(format!("Heals {}% (unit damaged {}% or more)", info.healing, info.damage_prereq));
    }
    if parts.is_empty() {
        return names::display(d, goody).into_owned();
    }
    parts.join("; ")
}

/// What lair result `key` of `tier` does, in a line.
fn lair_label(g: &Game, p: PlayerId, tier: LairTier, key: &str) -> String {
    let d = g.d;
    if let Some(goody) = Game::lair_outcome_goody(tier, key) {
        return goody_label(g, p, d.t::<GoodyType>(goody));
    }
    let promo = |name: &str| names::display(d, d.t::<PromotionType>(name)).into_owned();
    let event = |trigger: &str| {
        let t = d.t::<EventTriggerType>(trigger);
        let name = names::display(d, t);
        format!("Event: {} ({} choices)", name.trim_start_matches("Explore Lair "), d.event_triggers[t].events.len())
    };
    match (tier, key) {
        (LairTier::BigBad, BIG_BAD_MONSTER) => "Barbarian monster with a promotion, and 0-4 henchmen, next to the unit".to_string(),
        (LairTier::Bad, "DEATH") => "The unit dies".to_string(),
        (LairTier::Bad, "COLLAPSE") => "Collapse: 50% damage (up to 90%)".to_string(),
        (LairTier::Bad, "DEMONIC_POSSESSION") => format!("Promotions: {}, {}, {}", promo("PROMOTION_ENRAGED"), promo("PROMOTION_CRAZED"), promo("PROMOTION_DEMON")),
        (LairTier::Bad, "POISONED") => format!("Promotion: {}; 25% damage (up to 90%)", promo("PROMOTION_POISONED")),
        (LairTier::Bad, "RUSTED") => format!("Promotion: {}; weapon promotions removed", promo("PROMOTION_RUSTED")),
        (LairTier::Bad, k @ ("CRAZED" | "DISEASED" | "ENRAGED" | "PLAGUED" | "WITHERED")) => format!("Promotion: {}", promo(&format!("PROMOTION_{k}"))),
        (LairTier::Neutral, "NOTHING") => "Nothing".to_string(),
        (LairTier::Neutral, "CAGE") => "Cage: the lair becomes a cage; units on the plot are held".to_string(),
        (LairTier::Neutral, "DEPTHS") => event("EVENTTRIGGER_EXPLORE_LAIR_DEPTHS"),
        (LairTier::Neutral, "DWARF_VS_LIZARDMEN") => event("EVENTTRIGGER_EXPLORE_LAIR_DWARF_VS_LIZARDMEN"),
        (LairTier::Neutral, "PORTAL") => event("EVENTTRIGGER_EXPLORE_LAIR_PORTAL"),
        (LairTier::Neutral, "MUTATED") => format!("Promotion: {}", promo("PROMOTION_MUTATED")),
        (LairTier::Good, "TREASURE") => format!("{} placed on a random empty land plot", names::display(d, d.t::<UnitType>("EQUIPMENT_TREASURE"))),
        (LairTier::Good, k @ ("SPIRIT_GUIDE" | "ENCHANTED_BLADE" | "SPELLSTAFF" | "POISONED_BLADE" | "FLAMING_ARROWS" | "SHIELD_OF_FAITH" | "BRONZE_WEAPONS" | "IRON_WEAPONS" | "MITHRIL_WEAPONS")) => {
            format!("Promotion: {}", promo(&format!("PROMOTION_{k}")))
        }
        (LairTier::BigGood, "GOLDEN_AGE") => format!("Golden age: {} turns", g.golden_age_length()),
        (LairTier::BigGood, k) if k.starts_with("BONUS_") => format!("Resource on this plot: {}", names::display(d, d.t::<BonusType>(k))),
        _ => names::humanize(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_tiers_sum_to_one() {
        for (level, epic) in [(1, false), (5, false), (40, false), (1, true), (60, true)] {
            let t = level_tiers(level, epic);
            assert_eq!(t.iter().map(|x| x.1).sum::<i64>(), 1_000_000);
        }
        assert_eq!(level_tiers(1, false)[0], (LairTier::BigBad, 130_000));
    }

    #[test]
    fn every_goody_has_a_label() {
        let d = ffh_data::global();
        let mut g = Game::new(d, ffh_core::setup::GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 1));
        g.game_init();
        let p = g.player_ids().next().expect("a player");
        for goody in d.goodies.keys() {
            let label = goody_label(&g, p, goody);
            assert!(!label.is_empty() && !label.contains("TXT_KEY"), "{}: {label}", d.goodies[goody].base.type_);
        }
        // The barbarians' own unit of a class: their hunters are Lizardmen.
        assert!(goody_label(&g, p, d.t::<GoodyType>("GOODY_EXPLORE_LAIR_SPAWN_LIZARDMAN")).contains("Lizard"));
    }
}
