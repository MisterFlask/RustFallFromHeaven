//! What a resource does: C++ `CvGameTextMgr::setBonusHelp`, static (no active player: the buildings
//! are their classes' defaults), with the tooltip's yields and revealing tech but not its name
//! line (the page shows the name) or the player's count of it. Then ffh-play's lines for what the
//! Civilopedia shows in its own panels or FfH2 does without a word: the tech that lets it be
//! connected and the one that obsoletes it, how it is had (the improvements that connect it or make it, the
//! buildings that provide it), the units it strengthens (affinity, weapons, a free promotion,
//! mutation), the buildings it boosts and the items it speeds up, and what requires it.

use ffh_data::*;

use super::{changes_help, color, finish, txt, Arg, NEWLINE, YIELD_ICONS};
use crate::names::{self, display};

/// The civilizations that build `bl` when it is not (nearly) everyone's: a building that replaces
/// its class's default, or a default that most civilizations replace (the Bannor's palace, the
/// default of the palaces). Only the civilizations with leaders count, not the random pick
/// (`RANDOM_CIVILIZATION`).
fn owners(d: &GameData, bl: BuildingType) -> Vec<String> {
    let Some(class) = d.buildings[bl].building_class_type else { return Vec::new() };
    let of = |c: CivilizationType| d.civilizations[c].civilization_buildings(class, &d.building_classes);
    let random = opt_from_i32::<CivilizationType>(d.defines.random_civilization);
    let civs: Vec<CivilizationType> = d.civilizations.keys().filter(|&c| Some(c) != random && d.civilizations[c].leaders.iter().any(|&l| l)).collect();
    let users: Vec<CivilizationType> = civs.iter().copied().filter(|&c| of(c) == Some(bl)).collect();
    let with_class = civs.iter().filter(|&&c| of(c).is_some()).count();
    if d.building_classes[class].default_building_index == Some(bl) && users.len() * 2 > with_class {
        return Vec::new();
    }
    users.into_iter().map(|c| display(d, c).into_owned()).collect()
}

/// A building's coloured name, with the civilizations that build it (see [`owners`]).
fn building_name(d: &GameData, bl: BuildingType) -> String {
    let civs = owners(d, bl);
    let name = color("COLOR_BUILDING_TEXT", &display(d, bl));
    if civs.is_empty() {
        name
    } else {
        format!("{name} ({})", civs.join(", "))
    }
}

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
        tech_reveal,
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
        yield_change,
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

    // The tooltip's (`!bCivilopediaText`): the plot's yields, the tech that reveals it.
    changes_help(&mut b, "", "", "", yield_change, &YIELD_ICONS, false, true);
    if let Some(t) = tech_reveal {
        line(&mut b, txt(d, "TXT_KEY_BONUS_REVEALED_BY", &[Arg::S(&display(d, *t))]));
    }
    // ffh-play's: the tech without which a team gets nothing from its plots (C++
    // `CvPlot::updatePlotGroupBonus`; the copies buildings provide do not need it; a disabled
    // tech, as raw mana's, is never known), and the one that obsoletes it
    // (`CvTeam::isBonusObsolete`).
    if let Some(t) = tech_city_trade {
        line(&mut b, if d.techs[*t].disable { "[ICON_BULLET]Cannot be connected as it is".to_string() } else { format!("[ICON_BULLET]Can be connected once you know {}", tech(*t)) });
    }
    if let Some(t) = tech_obsolete {
        line(&mut b, txt(d, "TXT_KEY_BUILDING_OBSOLETE_WITH", &[Arg::S(&display(d, *t))]));
    }

    // ffh-play's: the improvements that connect it (C++ `isImprovementBonusTrade`), with what they
    // add on it (`getImprovementBonusYield`); those that make it from another resource
    // (FfH `getBonusConvert`: a node on raw mana) and those that turn it into another.
    let mut connect = Vec::new();
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
                push_new(&mut connect, if y.is_empty() { name } else { format!("{name} ({y})") });
            }
            _ => {}
        }
    }
    if !connect.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Connected by {}", connect.join(", ")));
    }
    for imp in d.improvements.keys().filter(|&i| d.improvements[i].bonus_convert == Some(bonus)) {
        let from: Vec<String> = d.bonuses.keys().filter(|&x| x != bonus && d.improvements[imp].improvement_bonus[x].bonus_makes_valid).map(|x| display(d, x).into_owned()).collect();
        if !from.is_empty() {
            line(&mut b, format!("[ICON_BULLET]Made from {} by a {}", from.join(", "), color("COLOR_HIGHLIGHT_TEXT", &display(d, imp))));
        }
    }
    if !convert_away.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Turned into another resource by {}", convert_away.join(", ")));
    }
    // The buildings that provide it (C++ `getFreeBonus`, FfH `getFreeBonus2/3`).
    let mut providers = Vec::new();
    for bl in d.buildings.keys() {
        let bi = &d.buildings[bl];
        if [bi.free_bonus, bi.free_bonus2, bi.free_bonus3].contains(&Some(bonus)) {
            push_new(&mut providers, building_name(d, bl));
        }
    }
    if !providers.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Provided by {}", providers.join(", ")));
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
    let mut affinity: Vec<(i32, Vec<String>)> = Vec::new();
    let mut add_affinity = |n: i32, name: String| match affinity.iter_mut().find(|(m, _)| *m == n) {
        Some((_, list)) => push_new(list, name),
        None => affinity.push((n, vec![name])),
    };
    for u in d.units.keys().filter(|&u| d.units[u].bonus_affinity[bonus] != 0) {
        add_affinity(d.units[u].bonus_affinity[bonus], color("COLOR_UNIT_TEXT", &display(d, u)));
    }
    for p in d.promotions.keys().filter(|&p| d.promotions[p].bonus_affinity[bonus] != 0) {
        add_affinity(d.promotions[p].bonus_affinity[bonus], format!("{} (promotion)", color("COLOR_HIGHLIGHT_TEXT", &display(d, p))));
    }
    for (n, list) in &affinity {
        line(&mut b, format!("[ICON_BULLET]{n:+}{strength} for each one you have: {}", list.join(", ")));
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
    // defaults).
    for class in d.building_classes.keys() {
        let Some(bl) = d.building_classes[class].default_building_index else { continue };
        let bi = &d.buildings[bl];
        let n = bi.bonus_happiness_changes[bonus];
        if n != 0 {
            line(&mut b, format!("[ICON_BULLET]{}{}", happy(n), with(&display(d, bl))));
        }
        let n = bi.bonus_health_changes[bonus];
        if n != 0 {
            line(&mut b, format!("[ICON_BULLET]{}{}", healthy(n), with(&display(d, bl))));
        }
    }

    // ffh-play's: the buildings whose yields it raises (C++ `getBonusYieldModifier`) and what it
    // builds faster (`getBonusProductionModifier` of buildings, units and projects), every
    // civilization's.
    let mut seen = Vec::new();
    for bl in d.buildings.keys() {
        let mut y = String::new();
        changes_help(&mut y, &building_name(d, bl), ": ", "", &d.buildings[bl].bonus_yield_modifier[bonus], &YIELD_ICONS, true, true);
        if !y.is_empty() && !seen.contains(&y) {
            b.push_str(&y);
            seen.push(y);
        }
    }
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
        add_faster(d.buildings[bl].bonus_production_modifier[bonus], building_name(d, bl));
    }
    for u in d.units.keys() {
        add_faster(d.units[u].bonus_production_modifier[bonus], color("COLOR_UNIT_TEXT", &display(d, u)));
    }
    for p in d.projects.keys() {
        add_faster(d.projects[p].bonus_production_modifier[bonus], color("COLOR_PROJECT_TEXT", &display(d, p)));
    }
    for (n, list) in &faster {
        let start = if *n == 100 { "Double Production Speed of ".to_string() } else { format!("{n:+}% Production of ") };
        line(&mut b, format!("[ICON_BULLET]{start}{}", list.join(", ")));
    }

    // ffh-play's: what requires it (the Civilopedia's "Allows"): the units, buildings and
    // promotions that need it (C++ `getPrereqAndBonus`, `getBonusPrereq`) and those that need it
    // or another of their resources (`getPrereqOrBonuses`).
    let mut needs = Vec::new();
    let mut needs_one_of = Vec::new();
    for u in d.units.keys() {
        let ui = &d.units[u];
        let name = color("COLOR_UNIT_TEXT", &display(d, u));
        if ui.prereq_and_bonus == Some(bonus) {
            push_new(&mut needs, name);
        } else if ui.prereq_or_bonuses.contains(&bonus) {
            push_new(&mut needs_one_of, name);
        }
    }
    for bl in d.buildings.keys() {
        let bi = &d.buildings[bl];
        if bi.prereq_and_bonus == Some(bonus) {
            push_new(&mut needs, building_name(d, bl));
        } else if bi.prereq_or_bonuses.contains(&Some(bonus)) {
            push_new(&mut needs_one_of, building_name(d, bl));
        }
    }
    for p in d.promotions.keys().filter(|&p| d.promotions[p].bonus_prereq == Some(bonus)) {
        push_new(&mut needs, color("COLOR_HIGHLIGHT_TEXT", &display(d, p)));
    }
    if !needs.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Required for {}", needs.join(", ")));
    }
    if !needs_one_of.is_empty() {
        line(&mut b, format!("[ICON_BULLET]Required (or an alternative) for {}", needs_one_of.join(", ")));
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
            "[ICON_BULLET]+1[ICON_COMMERCE]",
            "[ICON_BULLET]Can be connected once you know [COLOR_TECH_TEXT]Crafting[COLOR_REVERT]",
            "[ICON_BULLET]Happy: +1[ICON_HAPPY] (with Winery)",
            "[ICON_BULLET]Connected by [COLOR_HIGHLIGHT_TEXT]Winery[COLOR_REVERT] (+1[ICON_FOOD], +2[ICON_COMMERCE])",
        ] {
            assert!(wine.contains(part), "{part}: {wine}");
        }
        let iron = bonus("BONUS_IRON");
        for part in [
            "[ICON_BULLET]Revealed by [COLOR_TECH_TEXT]Smelting[COLOR_REVERT]",
            "[ICON_BULLET]Can be connected once you know [COLOR_TECH_TEXT]Iron Working[COLOR_REVERT]",
            "[ICON_BULLET]Units able to use [COLOR_HIGHLIGHT_TEXT]Iron Weapons[COLOR_REVERT] get them",
            "Connected by [COLOR_HIGHLIGHT_TEXT]Mine[COLOR_REVERT] (+1[ICON_PRODUCTION])",
            "[ICON_BULLET]Required (or an alternative) for [COLOR_UNIT_TEXT]",
            "Provided by [COLOR_BUILDING_TEXT]",
        ] {
            assert!(iron.contains(part), "{part}: {iron}");
        }
        // Wheat: health with a farm, and the granary's.
        let wheat = bonus("BONUS_WHEAT");
        assert!(wheat.contains("[ICON_BULLET]Healthy: +1[ICON_HEALTHY] (with Farm)") && wheat.contains("(with Granary)"), "{wheat}");
        // Marble speeds up wonders and the gargoyles.
        let marble = bonus("BONUS_MARBLE");
        assert!(marble.contains("[ICON_BULLET]Double Production Speed of [COLOR_BUILDING_TEXT]") && marble.contains("[ICON_BULLET]+25% Production of [COLOR_UNIT_TEXT]"), "{marble}");
        // Mana: raw mana becomes a sphere by a node; a sphere strengthens its elementals.
        let mana = bonus("BONUS_MANA");
        assert!(mana.contains("[ICON_BULLET]Cannot be connected as it is") && mana.contains("[ICON_BULLET]Turned into another resource by [COLOR_HIGHLIGHT_TEXT]"), "{mana}");
        let fire = bonus("BONUS_MANA_FIRE");
        for part in [
            "[ICON_BULLET]+1 Fire combat for each one you have: ",
            "Fire Elemental",
            "(promotion)",
            "[ICON_BULLET]Made from Mana by a [COLOR_HIGHLIGHT_TEXT]Fire Node",
            "[COLOR_HIGHLIGHT_TEXT]Fire I[COLOR_REVERT]",
        ] {
            assert!(fire.contains(part), "{part}: {fire}");
        }
        assert!(bonus("BONUS_MANA_NATURE").contains("Double Production Speed of [COLOR_PROJECT_TEXT]Genesis[COLOR_REVERT]"));
        let body = bonus("BONUS_MANA_BODY");
        assert!(body.contains("[ICON_BULLET]+5% to heal rate of friendly units within your borders") && body.contains("Benefits of this resource are cumulative"), "{body}");
        assert!(bonus("BONUS_MANA_CHAOS").contains("+2% chance that living units built in a city with it start mutated"));
        assert!(bonus("BONUS_NIGHTMARE").contains("[ICON_BULLET]Free [COLOR_HIGHLIGHT_TEXT]"));
        let d = ffh_data::global();
        let brewery = display(d, d.t::<BuildingType>("BUILDING_BREWERY")).into_owned();
        assert!(bonus("BONUS_ALE").contains(&format!("Provided by [COLOR_BUILDING_TEXT]{brewery}[COLOR_REVERT]")));
        // A palace of one civilization says whose it is, the default (Bannor) one too.
        let law = bonus("BONUS_MANA_LAW");
        assert!(law.contains("Palace[COLOR_REVERT] (Bannor)") && law.contains("Palace[COLOR_REVERT] (Khazad)"), "{law}");
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
