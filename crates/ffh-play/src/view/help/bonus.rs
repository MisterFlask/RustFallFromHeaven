//! What a resource does, for its tooltip: only what a player acts on. How it is had (the tech
//! that lets it be connected, the improvements a worker can build on it; the buildings that
//! provide it when no improvement does), what it gives (C++ `CvGameTextMgr::setBonusHelp`: health,
//! happiness, FfH's modifiers), what it does to units (a free promotion, weapons, mutation,
//! affinity in one line) and to buildings (their yields with it, by building and not by tier;
//! what it builds faster).
//!
//! Left out on purpose (the owner's call, 2026-11-12): the plot's yields (the plot's totals are
//! under the tooltip) and the revealing tech (a resource that is seen is revealed), the palaces
//! and wonders that also provide a resource found on the map, the unique features that hold it,
//! the units with an affinity by name, what requires it, where the map places it.

use ffh_data::*;

use super::{changes_help, color, finish, txt, Arg, NEWLINE, YIELD_ICONS};
use crate::names::{self, display};

/// The building `bl` is a tier of (docs/SUITABILITY.md), itself when it is no variant.
fn standard(d: &GameData, bl: BuildingType) -> BuildingType {
    d.buildings[bl].variant_of.unwrap_or(bl)
}

/// More names than this are counted, not listed.
const LIST_MAX: usize = 4;

/// Pushes `item` unless it is listed already.
fn push_new(list: &mut Vec<String>, item: String) {
    if !list.contains(&item) {
        list.push(item);
    }
}

/// What a resource `bonus` does (Civ4 markup, lines joined by `[NEWLINE]`; empty when there is
/// nothing to say). Every field of the resource is either described here or named as not shown.
pub fn effects(d: &GameData, bonus: BonusType) -> String {
    let BonusInfo {
        base,
        // Map generation (resources of a class keep apart).
        bonus_class_type: _,
        // A resource that is seen is revealed.
        tech_reveal: _,
        tech_city_trade,
        tech_obsolete,
        // AI weights.
        ai_trade_modifier: _,
        ai_objective: _,
        health,
        happiness,
        // Map generation: where and how often it is placed.
        min_area_size: _,
        min_latitude: _,
        max_latitude: _,
        placement_order: _,
        const_appearance: _,
        rand_appearance1: _,
        rand_appearance2: _,
        rand_appearance3: _,
        rand_appearance4: _,
        percent_per_player: _,
        tiles_per: _,
        min_land_percent: _,
        unique_range: _,
        group_range: _,
        group_rand: _,
        one_area: _,
        hills: _,
        flatlands: _,
        no_river_side: _,
        normalize: _,
        modifier_per_bonus,
        // Read by the AI only: the C++ attitude ("You use X") and city site values; ffh-ai's
        // relation score does not use it.
        bad_attitude: _,
        damage_type,
        discover_rand_modifier,
        free_promotion,
        great_people_rate_modifier,
        heal_change,
        heal_change_enemy,
        maintenance_modifier,
        mutate_chance,
        research_modifier,
        // Map generation (docs/NAVAL.md, 9.1); the plot query's `bonus_note` tells a far shore's.
        far_shore: _,
        // The plot's totals are under the tooltip.
        yield_change: _,
        // Map generation: the terrains and features it is placed on.
        terrain: _,
        feature: _,
        feature_terrain: _,
    } = &d.bonuses[bonus];
    let mut b = String::new();
    let line = |b: &mut String, s: String| {
        b.push_str(NEWLINE);
        b.push_str(&s);
    };
    let tech = |t: TechType| color("COLOR_TECH_TEXT", &display(d, t));

    // The tech without which a team gets nothing from its plots (C++
    // `CvPlot::updatePlotGroupBonus`; a disabled tech, as raw mana's, is never known), and the one
    // that obsoletes it (`CvTeam::isBonusObsolete`).
    if let Some(t) = tech_city_trade {
        line(&mut b, if d.techs[*t].disable { "[ICON_BULLET]Cannot be connected as it is".to_string() } else { format!("[ICON_BULLET]Can be connected once you know {}", tech(*t)) });
    }
    if let Some(t) = tech_obsolete {
        line(&mut b, txt(d, "TXT_KEY_BUILDING_OBSOLETE_WITH", &[Arg::S(&display(d, *t))]));
    }

    // The improvements that connect it (C++ `isImprovementBonusTrade`), with what they add on it
    // (`getImprovementBonusYield`): those a worker builds, the unique features only when nothing
    // else does. Those that turn it into another resource (FfH `getBonusConvert`: a node on raw
    // mana).
    let mut connect = Vec::new();
    let mut connect_unique = Vec::new();
    let mut convert_away = Vec::new();
    for imp in d.improvements.keys() {
        let ii = &d.improvements[imp];
        let ib = &ii.improvement_bonus[bonus];
        let name = color("COLOR_HIGHLIGHT_TEXT", &display(d, imp));
        match ii.bonus_convert {
            Some(other) if other != bonus => {
                if ib.bonus_makes_valid || ib.bonus_trade {
                    push_new(&mut convert_away, name);
                }
            }
            _ if ib.bonus_trade => {
                let mut y = String::new();
                changes_help(&mut y, "", "", "", &ib.yield_change, &YIELD_ICONS, false, false);
                push_new(if ii.unique { &mut connect_unique } else { &mut connect }, if y.is_empty() { name } else { format!("{name} ({y})") });
            }
            _ => {}
        }
    }
    if connect.is_empty() {
        connect = connect_unique;
    }
    if !connect.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Connected by {}", connect.join(", ")));
    }
    if convert_away.len() > LIST_MAX {
        line(&mut b, "[ICON_BULLET]A node built on it turns it into the node's mana".to_string());
    } else if !convert_away.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Turned into another resource by {}", convert_away.join(", ")));
    }
    // A resource no improvement connects: the buildings that provide it (C++ `getFreeBonus`, FfH
    // `getFreeBonus2/3`), a building's tiers once.
    if connect.is_empty() {
        let mut providers = Vec::new();
        for bl in d.buildings.keys() {
            let bi = &d.buildings[bl];
            if [bi.free_bonus, bi.free_bonus2, bi.free_bonus3].contains(&Some(bonus)) {
                push_new(&mut providers, color("COLOR_BUILDING_TEXT", &display(d, standard(d, bl))));
            }
        }
        if !providers.is_empty() {
            line(&mut b, format!("[ICON_BULLET]Provided by {}", providers.join(", ")));
        }
    }

    // C++ names the first improvement that the resource makes valid as the one that gives it.
    let improvement = d.improvements.keys().find(|&i| d.improvements[i].improvement_bonus[bonus].bonus_makes_valid);
    // BtS writes these lines without a bullet and the "(with ..)" right after the icon: here they
    // read as the other lines do.
    let with = |name: &str| format!(" {}", txt(d, "TXT_KEY_BONUS_WITH_IMPROVEMENT", &[Arg::S(name)]));
    let healthy = |n: i32| {
        if n > 0 {
            txt(d, "TXT_KEY_BONUS_HEALTHY", &[Arg::N(n)])
        } else {
            txt(d, "TXT_KEY_BONUS_UNHEALTHY", &[Arg::N(-n)])
        }
    };
    let happy = |n: i32| {
        if n > 0 {
            txt(d, "TXT_KEY_BONUS_HAPPY", &[Arg::N(n)])
        } else {
            txt(d, "TXT_KEY_BONUS_UNHAPPY", &[Arg::N(-n)])
        }
    };
    let with_improvement = improvement.map(|i| with(&display(d, i))).unwrap_or_default();
    if *health != 0 {
        line(&mut b, format!("[ICON_BULLET]{}{with_improvement}", healthy(*health)));
    }
    if *happiness != 0 {
        line(&mut b, format!("[ICON_BULLET]{}{with_improvement}", happy(*happiness)));
    }

    // FfH.
    if *discover_rand_modifier > 0 {
        line(&mut b, txt(d, "TXT_KEY_BONUS_DISCOVER_RAND_MODIFIER_INCREASE", &[]));
    } else if *discover_rand_modifier < 0 {
        line(&mut b, txt(d, "TXT_KEY_BONUS_DISCOVER_RAND_MODIFIER_DECREASE", &[]));
    }
    for (n, key) in [
        (*great_people_rate_modifier, "TXT_KEY_BONUS_GREAT_PEOPLE_RATE_MODIFIER"),
        (*heal_change, "TXT_KEY_BONUS_HEAL_CHANGE"),
        (*heal_change_enemy, "TXT_KEY_BONUS_HEAL_CHANGE_ENEMY"),
        (*maintenance_modifier, "TXT_KEY_BONUS_MAINTENANCE_MODIFIER"),
        (*research_modifier, "TXT_KEY_BONUS_RESEARCH_MODIFIER"),
    ] {
        if n != 0 {
            line(&mut b, txt(d, key, &[Arg::N(n)]));
        }
    }
    if *modifier_per_bonus {
        line(&mut b, txt(d, "TXT_KEY_BONUS_MODIFIER_PER_BONUS", &[]));
    }
    // ffh-play's: what it does to units. A free promotion for the units a city with it trains
    // (C++ `CvCity::processBonus`, `addProductionExperience`: those of the promotion's unit
    // combats) and a chance for its living units to start mutated (`changeMutateChance`).
    if let Some(p) = free_promotion {
        line(&mut b, format!("[ICON_BULLET]Free {} Promotion for Units Built in a City with it", color("COLOR_HIGHLIGHT_TEXT", &display(d, *p))));
    }
    if *mutate_chance != 0 {
        line(&mut b, format!("[ICON_BULLET]{mutate_chance:+}% chance that living units built in a city with it start mutated"));
    }
    // Affinity (C++ `CvUnit::updateBonusAffinity`): each one the player has adds the affinity to
    // the strength of the units (and of the holders of the promotions) with one, or to their
    // combat of its damage type.
    let strength = match damage_type {
        Some(t) => format!(" {} combat", display(d, *t)),
        None => "[ICON_STRENGTH]".to_string(),
    };
    // One line, without the units' names: the least and the most a unit or a promotion has.
    let affinity: Vec<i32> = d.units.keys().map(|u| d.units[u].bonus_affinity[bonus]).chain(d.promotions.keys().map(|p| d.promotions[p].bonus_affinity[bonus])).filter(|&n| n != 0).collect();
    if let (Some(&lo), Some(&hi)) = (affinity.iter().min(), affinity.iter().max()) {
        let n = if lo == hi { format!("{lo:+}") } else { format!("{lo:+} to {hi:+}") };
        line(&mut b, format!("[ICON_BULLET]Units with an affinity for it: {n}{strength} for each one you have"));
    }
    // FfH's weapons (C++ `CvUnit::setWeapons`, the `WEAPON_*_TIER*` defines): units able to use a
    // tier get its promotion in their owner's cities that have its resource.
    for (req, promotion) in [
        (d.defines.weapon_req_bonus_tier1, d.defines.weapon_promotion_tier1),
        (d.defines.weapon_req_bonus_tier2, d.defines.weapon_promotion_tier2),
        (d.defines.weapon_req_bonus_tier3, d.defines.weapon_promotion_tier3),
    ] {
        if let (Some(req), Some(p)) = (opt_from_i32::<BonusType>(req), opt_from_i32::<PromotionType>(promotion)) {
            if req == bonus {
                line(&mut b, format!("[ICON_BULLET]Units able to use {} get them in your cities with it", color("COLOR_HIGHLIGHT_TEXT", &display(d, p))));
            }
        }
    }

    // The buildings' happiness and health with it (C++; without a civilization, the classes'
    // defaults), the buildings of one amount on one line.
    let mut moods: Vec<(String, Vec<String>)> = Vec::new();
    for class in d.building_classes.keys() {
        let Some(bl) = d.building_classes[class].default_building_index else { continue };
        let bi = &d.buildings[bl];
        let (h, s) = (bi.bonus_happiness_changes[bonus], bi.bonus_health_changes[bonus]);
        for text in [(h != 0).then(|| happy(h)), (s != 0).then(|| healthy(s))].into_iter().flatten() {
            let name = display(d, standard(d, bl)).into_owned();
            match moods.iter_mut().find(|(t, _)| *t == text) {
                Some((_, list)) => push_new(list, name),
                None => moods.push((text, vec![name])),
            }
        }
    }
    for (text, list) in &moods {
        line(&mut b, format!("[ICON_BULLET]{text}{}", with(&list.join(", "))));
    }

    // The buildings whose yields it raises (C++ `getBonusYieldModifier`): a building once, with
    // the least and the most of its tiers.
    let mut raised: Vec<(BuildingType, [(i32, i32); YieldType::NUM])> = Vec::new();
    for bl in d.buildings.keys() {
        let m = &d.buildings[bl].bonus_yield_modifier[bonus];
        if m.iter().all(|&n| n == 0) {
            continue;
        }
        let of = standard(d, bl);
        match raised.iter_mut().find(|(s, _)| *s == of) {
            Some((_, r)) => {
                for (r, &n) in r.iter_mut().zip(m.iter()) {
                    *r = (r.0.min(n), r.1.max(n));
                }
            }
            None => raised.push((of, std::array::from_fn(|y| (m[y], m[y])))),
        }
    }
    for (bl, r) in &raised {
        let parts: Vec<String> = r.iter().zip(YIELD_ICONS.iter()).filter(|((lo, hi), _)| *lo != 0 || *hi != 0).map(|((lo, hi), icon)| if lo == hi { format!("{lo:+}%{icon}") } else { format!("{lo:+}% to {hi:+}%{icon}") }).collect();
        line(&mut b, format!("[ICON_BULLET]{}: {}", color("COLOR_BUILDING_TEXT", &display(d, *bl)), parts.join(", ")));
    }
    // What it builds faster (`getBonusProductionModifier` of buildings, units and projects): the
    // names of a few, the count of many.
    let mut faster: Vec<(i32, Vec<String>)> = Vec::new();
    let mut add_faster = |n: i32, name: String| {
        if n != 0 {
            match faster.iter_mut().find(|(m, _)| *m == n) {
                Some((_, list)) => push_new(list, name),
                None => faster.push((n, vec![name])),
            }
        }
    };
    for bl in d.buildings.keys() {
        add_faster(d.buildings[bl].bonus_production_modifier[bonus], color("COLOR_BUILDING_TEXT", &display(d, standard(d, bl))));
    }
    for u in d.units.keys() {
        add_faster(d.units[u].bonus_production_modifier[bonus], color("COLOR_UNIT_TEXT", &display(d, u)));
    }
    for p in d.projects.keys() {
        add_faster(d.projects[p].bonus_production_modifier[bonus], color("COLOR_PROJECT_TEXT", &display(d, p)));
    }
    for (n, list) in &faster {
        let start = if *n == 100 { "Double Production Speed of ".to_string() } else { format!("{n:+}% Production of ") };
        let what = if list.len() > LIST_MAX { format!("{} buildings, units or rituals", list.len()) } else { list.join(", ") };
        line(&mut b, format!("[ICON_BULLET]{start}{what}"));
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

    fn bonus(name: &str) -> String {
        let d = ffh_data::global();
        effects(d, d.t::<BonusType>(name))
    }

    #[test]
    fn known_resources_say_what_they_give() {
        let wine = bonus("BONUS_WINE");
        for part in [
            "[ICON_BULLET]Can be connected once you know [COLOR_TECH_TEXT]Crafting[COLOR_REVERT]",
            "[ICON_BULLET]Happy: +1[ICON_HAPPY] (with Winery)",
            "[ICON_BULLET]Connected by [COLOR_HIGHLIGHT_TEXT]Winery[COLOR_REVERT] (+1[ICON_FOOD], +2[ICON_COMMERCE])",
        ] {
            assert!(wine.contains(part), "{part}: {wine}");
        }
        let iron = bonus("BONUS_IRON");
        for part in [
            "[ICON_BULLET]Can be connected once you know [COLOR_TECH_TEXT]Iron Working[COLOR_REVERT]",
            "[ICON_BULLET]Units able to use [COLOR_HIGHLIGHT_TEXT]Iron Weapons[COLOR_REVERT] get them",
            "Connected by [COLOR_HIGHLIGHT_TEXT]Mine[COLOR_REVERT] (+1[ICON_PRODUCTION])",
            // A building's tiers are one line.
            "[ICON_BULLET][COLOR_BUILDING_TEXT]Dwarven Smithy[COLOR_REVERT]: +5% to +20%[ICON_PRODUCTION]",
        ] {
            assert!(iron.contains(part), "{part}: {iron}");
        }
        // Wheat: health with a farm, and the granary's.
        let wheat = bonus("BONUS_WHEAT");
        assert!(wheat.contains("[ICON_BULLET]Healthy: +1[ICON_HEALTHY] (with Farm)") && wheat.contains("(with Granary)"), "{wheat}");
        // Marble speeds up wonders (counted) and the gargoyles (named).
        let marble = bonus("BONUS_MARBLE");
        assert!(marble.contains("[ICON_BULLET]Double Production Speed of 9 buildings, units or rituals") && marble.contains("[ICON_BULLET]+25% Production of [COLOR_UNIT_TEXT]"), "{marble}");
        // Mana: raw mana becomes a sphere by a node; a sphere strengthens the units of its affinity.
        let mana = bonus("BONUS_MANA");
        assert!(mana.contains("[ICON_BULLET]Cannot be connected as it is") && mana.contains("[ICON_BULLET]A node built on it turns it into the node's mana"), "{mana}");
        let fire = bonus("BONUS_MANA_FIRE");
        for part in ["[ICON_BULLET]Units with an affinity for it: +1 Fire combat for each one you have", "[ICON_BULLET]Connected by [COLOR_HIGHLIGHT_TEXT]Fire Node[COLOR_REVERT]"] {
            assert!(fire.contains(part), "{part}: {fire}");
        }
        assert!(bonus("BONUS_MANA_SUN").contains("Units with an affinity for it: +1 to +2 Fire combat for each one you have"));
        assert!(bonus("BONUS_MANA_NATURE").contains("Double Production Speed of [COLOR_PROJECT_TEXT]Genesis[COLOR_REVERT]"));
        let body = bonus("BONUS_MANA_BODY");
        assert!(body.contains("[ICON_BULLET]+5% to heal rate of friendly units within your borders") && body.contains("Benefits of this resource are cumulative"), "{body}");
        assert!(bonus("BONUS_MANA_CHAOS").contains("+2% chance that living units built in a city with it start mutated"));
        assert!(bonus("BONUS_NIGHTMARE").contains("[ICON_BULLET]Free [COLOR_HIGHLIGHT_TEXT]"));
        // Only a resource the map does not hold names the buildings that provide it.
        let d = ffh_data::global();
        let brewery = display(d, d.t::<BuildingType>("BUILDING_BREWERY")).into_owned();
        assert!(bonus("BONUS_ALE").contains(&format!("Provided by [COLOR_BUILDING_TEXT]{brewery}[COLOR_REVERT]")));
        // What the tooltip leaves out: who else provides it, the units by name, what requires it,
        // a unique feature beside the node.
        for bn in d.bonuses.keys() {
            let t = effects(d, bn);
            assert!(!t.contains("Required") && !t.contains("Revealed by") && !t.contains("Palace"), "{}: {t}", names::name(d, bn));
        }
        let earth = bonus("BONUS_MANA_EARTH");
        assert!(!earth.contains("Standing Stones") && !earth.contains("Provided by") && !earth.contains("Djinn"), "{earth}");
    }

    #[test]
    fn every_resource_reads_well() {
        let d = ffh_data::global();
        for bn in d.bonuses.keys() {
            let t = effects(d, bn);
            assert!(!t.contains("%s") && !t.contains("%d") && !t.contains("%D") && !t.contains("%F"), "{}: {t}", names::name(d, bn));
            assert!(!t.starts_with(NEWLINE) && !t.contains("[NEWLINE][NEWLINE]"), "{t}");
            let i = &d.bonuses[bn];
            let mut x = i.clone();
            x.base = InfoBase::default();
            x.bonus_class_type = None;
            (x.ai_trade_modifier, x.ai_objective, x.bad_attitude) = (0, 0, 0);
            (x.min_area_size, x.min_latitude, x.max_latitude, x.placement_order, x.const_appearance) = (0, 0, 0, 0, 0);
            (x.rand_appearance1, x.rand_appearance2, x.rand_appearance3, x.rand_appearance4) = (0, 0, 0, 0);
            (x.percent_per_player, x.tiles_per, x.min_land_percent, x.unique_range, x.group_range, x.group_rand) = (0, 0, 0, 0, 0, 0);
            (x.one_area, x.hills, x.flatlands, x.no_river_side, x.normalize) = (false, false, false, false, false);
            (x.terrain, x.feature, x.feature_terrain) = (IdxVec::new(), IdxVec::new(), IdxVec::new());
            if sets_anything(&format!("{x:?}")) {
                assert!(!t.is_empty(), "{}", names::name(d, bn));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        for bn in d.bonuses.keys() {
            println!("{}:\n  {}\n", names::name(d, bn), effects(d, bn).replace(NEWLINE, "\n  "));
        }
    }
}
