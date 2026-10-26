//! What researching a tech brings the human: C++ `CvGameTextMgr::setTechHelp` with its
//! `bPlayerContext` (the active player's civilization's own units and buildings; only what the
//! player cannot do yet; the free unit, free techs and religion only while nobody knows the tech)
//! and `bTreeInfo` ("Leads to"), without the name, the cost line and Sid's tips, which the screens
//! show themselves.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::{changes_help, color, finish, list_help, txt, Arg, COMMERCE_ICONS, NEWLINE, YIELD_ICONS};
use crate::names;

/// What tech `t` gives player `p` that it cannot have yet, as the help lists it: the player's own
/// units and buildings ("Can Train", "Can Construct"), the worker's builds ("Can ..."), the civics
/// ("Enables") and the religions the first to discover it founds (which the help leaves unnamed
/// with "pick religion"). The research screens draw them as icons.
pub struct Unlocks {
    pub units: Vec<UnitType>,
    pub buildings: Vec<BuildingType>,
    pub builds: Vec<BuildType>,
    pub civics: Vec<CivicType>,
    pub religions: Vec<ReligionType>,
}

/// The player's unit of a class (C++ `getCivilizationUnits` of the active civ).
fn civ_unit(g: &Game, p: PlayerId, class: UnitClassType) -> Option<UnitType> {
    let d = g.d;
    match g.player_get_civilization_type(p) {
        Some(c) => d.civilizations[c].civilization_units(class, d.unit_classes[class].default_unit_index),
        None => d.unit_classes[class].default_unit_index,
    }
}

/// The player's building of a class (C++ `getCivilizationBuildings` of the active civ).
fn civ_building(g: &Game, p: PlayerId, class: BuildingClassType) -> Option<BuildingType> {
    let d = g.d;
    match g.player_get_civilization_type(p) {
        Some(c) => d.civilizations[c].civilization_buildings(class, &d.building_classes),
        None => d.building_classes[class].default_building_index,
    }
}

/// [`Unlocks`] of tech `t` for player `p`.
pub fn unlocks(g: &Game, p: PlayerId, t: TechType) -> Unlocks {
    let d = g.d;
    let ours = |c: Option<CivilizationType>| c.is_none() || c == g.player_get_civilization_type(p);
    let units = d
        .unit_classes
        .keys()
        .filter(|&class| !g.player_is_production_maxed_unit_class(p, class))
        .filter_map(|class| civ_unit(g, p, class))
        .filter(|&u| {
            let ui = &d.units[u];
            !g.player_can_train(p, u, false, false, false) && (ui.prereq_and_tech == Some(t) || ui.prereq_and_techs.contains(&t))
        })
        .collect();
    let buildings = d
        .building_classes
        .keys()
        .filter(|&class| !g.player_is_production_maxed_building_class(p, class, false))
        .filter_map(|class| civ_building(g, p, class))
        .filter(|&bl| {
            let bi = &d.buildings[bl];
            !g.player_can_construct(p, bl, false, true, false) && (bi.prereq_and_tech == Some(t) || bi.prereq_and_techs.contains(&Some(t)))
        })
        .collect();
    // C++ `buildImprovementString`: a build without its own tech is allowed by its feature techs
    // (the chop of a forest).
    let builds = d
        .builds
        .keys()
        .filter(|&build| {
            let bi = &d.builds[build];
            match bi.tech_prereq {
                None => bi.feature_tech.iter().any(|&f| f == Some(t)),
                Some(x) => x == t,
            }
        })
        .collect();
    let civics = d.civics.keys().filter(|&x| d.civics[x].tech_prereq == Some(t) && ours(d.civics[x].prereq_civilization)).collect();
    let religions = if g.count_known_tech_num_teams(t) == 0 {
        d.religions.keys().filter(|&r| !g.religion_slot_taken[r] && d.religions[r].tech_prereq == Some(t)).collect()
    } else {
        Vec::new()
    };
    Unlocks { units, buildings, builds, civics, religions }
}

/// The help of tech `t` for player `p` (Civ4 markup, lines joined by `[NEWLINE]`; empty when the
/// tech brings nothing to list).
pub fn tech_help(g: &Game, p: PlayerId, t: TechType) -> String {
    tech_help_with(g, p, t, &unlocks(g, p, t))
}

/// [`tech_help`] with the tech's [`unlocks`] already worked out.
pub fn tech_help_with(g: &Game, p: PlayerId, t: TechType, un: &Unlocks) -> String {
    let d = g.d;
    let pl = &g.players[p];
    let team = pl.team;
    let info = &d.techs[t];
    let mut b = String::new();
    let nobody_knows = g.count_known_tech_num_teams(t) == 0;

    // Obsolete buildings (of the classes the player has), resources, special buildings.
    for class in d.building_classes.keys() {
        if pl.building_class_count[class] > 0 {
            if let Some(bl) = civ_building(g, p, class).filter(|&bl| d.buildings[bl].obsolete_tech == Some(t)) {
                b.push_str(NEWLINE);
                b.push_str(&txt(d, "TXT_KEY_TECH_OBSOLETES", "[COLOR_WARNING_TEXT]Obsoletes [LINK=literal]%s1_Name[\\LINK][COLOR_REVERT]", &[Arg::S(&names::display(d, bl))]));
            }
        }
    }
    for bonus in d.bonuses.keys().filter(|&x| d.bonuses[x].tech_obsolete == Some(t)) {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_TECH_OBSOLETES", "[COLOR_WARNING_TEXT]Obsoletes [LINK=literal]%s1_Name[\\LINK][COLOR_REVERT]", &[Arg::S(&names::display(d, bonus))]));
    }
    for sb in d.special_buildings.keys().filter(|&x| d.special_buildings[x].obsolete_tech == Some(t)) {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_TECH_OBSOLETES_NO_LINK", "[COLOR_WARNING_TEXT]Obsoletes %s1_Name[COLOR_REVERT]", &[Arg::S(&names::display(d, sb))]));
    }

    // C++ `buildMoveString`: the moves a route gains.
    let denom = d.defines.move_denominator;
    for r in d.routes.keys() {
        let cost = d.routes[r].movement_cost + g.teams[team].route_change[r];
        let diff = denom / cost.max(1) - denom / (cost + d.routes[r].tech_movement_change[t]).max(1);
        if diff != 0 {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_UNIT_MOVEMENT", "[ICON_BULLET]%D1_Change %s2_MoveType Movement", &[Arg::N(-diff), Arg::S(&names::display(d, r))]));
        }
    }
    // The free unit and the free techs of the first to discover it.
    if let Some(u) = g.player_get_tech_free_unit(p, t).filter(|_| nobody_knows) {
        b.push_str(NEWLINE);
        b.push_str(&txt(
            d,
            "TXT_KEY_TECH_FIRST_RECEIVES",
            "[ICON_BULLET]First to Discover Receives a [COLOR_UNIT_TEXT][LINK=literal]%s1_UnitName[\\LINK][COLOR_REVERT]",
            &[Arg::S(&names::display(d, u))],
        ));
    }
    if info.feature_production_modifier != 0 {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_FEATURE_PRODUCTION_MODIFIER", "[ICON_BULLET]Workers Produce %D1_Change%% [ICON_PRODUCTION] from Chopping", &[Arg::N(info.feature_production_modifier)]));
    }
    if info.worker_speed_modifier != 0 {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_UNIT_WORKERS_FASTER", "[ICON_BULLET]Workers Build Improvements %D1_Change%% Faster", &[Arg::N(info.worker_speed_modifier)]));
    }
    if info.trade_routes != 0 {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_MISC_TRADE_ROUTES", "[ICON_BULLET]%D1_Change Trade [NUM1:Route:Routes] per City", &[Arg::N(info.trade_routes)]));
    }
    if info.health != 0 {
        let icon = if info.health > 0 { "[ICON_HEALTHY]" } else { "[ICON_UNHEALTHY]" };
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_MISC_HEALTH_ALL_CITIES", "[ICON_BULLET]%D1_Change%F2_HealthOrUn in All Cities", &[Arg::N(info.health.abs()), Arg::S(icon)]));
    }
    if info.happiness != 0 {
        let icon = if info.happiness > 0 { "[ICON_HAPPY]" } else { "[ICON_UNHAPPY]" };
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_MISC_HAPPINESS_ALL_CITIES", "[ICON_BULLET]%D1_Change%F2_HappyOrUn in All Cities", &[Arg::N(info.happiness.abs()), Arg::S(icon)]));
    }
    if info.first_free_techs > 0 && nobody_knows {
        b.push_str(NEWLINE);
        b.push_str(&if info.first_free_techs == 1 {
            txt(d, "TXT_KEY_TECH_FIRST_FREE_TECH", "[ICON_BULLET]First to Discover Receives a [COLOR_TECH_TEXT]Free Technology[COLOR_REVERT]", &[])
        } else {
            txt(d, "TXT_KEY_TECH_FIRST_FREE_TECHS", "[ICON_BULLET]First to Discover Receives [COLOR_TECH_TEXT]%d1_Num Free Technologies[COLOR_REVERT]", &[Arg::N(info.first_free_techs)])
        });
    }

    // What the team gains once (each only while it does not have it yet).
    let flags: [(bool, bool, &str, &str); 10] = [
        (info.extra_water_see_from, g.team_is_extra_water_see_from(team), "TXT_KEY_UNIT_EXTRA_SIGHT", "[ICON_BULLET]+1 Sight Across Water"),
        (info.map_centering, g.teams[team].map_centering, "TXT_KEY_MISC_CENTERS_MAP", "[ICON_BULLET]Centers [LINK=CONCEPT_MAPS]World Map[\\LINK]"),
        (info.map_visible, false, "TXT_KEY_MISC_REVEALS_MAP", "[ICON_BULLET]Reveals [LINK=CONCEPT_MAPS]World Map[\\LINK]"),
        (info.map_trading, g.team_is_map_trading(team), "TXT_KEY_MISC_ENABLES_MAP_TRADING", "[ICON_BULLET]Enables [LINK=CONCEPT_TRADE]Map Trading[\\LINK]"),
        (info.tech_trading, g.team_is_tech_trading(team), "TXT_KEY_MISC_ENABLES_TECH_TRADING", "[ICON_BULLET]Enables [LINK=CONCEPT_TRADE]Technology Trading[\\LINK]"),
        (info.gold_trading, g.team_is_gold_trading(team), "TXT_KEY_MISC_ENABLES_GOLD_TRADING", "[ICON_BULLET]Enables [LINK=CONCEPT_TRADE]Gold Trading[\\LINK] via Diplomacy"),
        (info.open_borders_trading, g.team_is_open_borders_trading(team), "TXT_KEY_MISC_ENABLES_OPEN_BORDERS", "[ICON_BULLET]Enables [LINK=CONCEPT_BORDERS]Open Borders[\\LINK]"),
        (info.defensive_pact_trading, g.team_is_defensive_pact_trading(team), "TXT_KEY_MISC_ENABLES_DEFENSIVE_PACTS", "[ICON_BULLET]Enables Defensive Pacts"),
        // Only with permanent alliances on (C++).
        (
            info.permanent_alliance_trading && g.is_option(GameOptionType::GAMEOPTION_PERMANENT_ALLIANCES),
            g.team_is_permanent_alliance_trading(team),
            "TXT_KEY_MISC_ENABLES_PERM_ALLIANCES",
            "[ICON_BULLET]Enables Permanent Alliances",
        ),
        (info.bridge_building, g.team_is_bridge_building(team), "TXT_KEY_MISC_ENABLES_BRIDGE_BUILDING", "[ICON_BULLET]Enables Bridge Building"),
    ];
    for (gives, has, key, fallback) in flags {
        if gives && !has {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, key, fallback, &[]));
        }
    }
    for (gives, has, key, fallback) in [
        (info.irrigation, g.team_is_irrigation(team), "TXT_KEY_MISC_SPREAD_IRRIGATION", "[ICON_BULLET][LINK=IMPROVEMENT_FARM]Farms[\\LINK] Spread [LINK=CONCEPT_IRRIGATION]Irrigation[\\LINK]"),
        (info.ignore_irrigation, g.team_is_ignore_irrigation(team), "TXT_KEY_MISC_IRRIGATION_ANYWHERE", "[ICON_BULLET]Can Build [LINK=IMPROVEMENT_FARM]Farms[\\LINK] without Irrigation"),
        (info.water_work, g.team_is_water_work(team), "TXT_KEY_MISC_WATER_WORK", "[ICON_BULLET]Can Work Water Tiles"),
        // C++ shows this one only when vassal states are *off* (`isOption(GAMEOPTION_NO_VASSAL_STATES)`,
        // a BtS slip); here it shows when they are on, which is when it is true.
        (
            info.vassal_state_trading && !g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES),
            g.team_is_vassal_state_trading(team),
            "TXT_KEY_MISC_ENABLES_VASSAL_STATES",
            "[ICON_BULLET]Enables Vassal States",
        ),
    ] {
        if gives && !has {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, key, fallback, &[]));
        }
    }

    // C++ `buildImprovementString`: the builds it allows.
    for &build in &un.builds {
        b.push_str(NEWLINE);
        b.push_str(&txt(d, "TXT_KEY_MISC_CAN_BUILD_IMPROVEMENT", "[ICON_BULLET]Can [COLOR_HIGHLIGHT_TEXT]%s1_ImpName[COLOR_REVERT]", &[Arg::S(&names::display(d, build))]));
    }
    for dom in d.domains.keys() {
        let n = info.domain_extra_moves[dom];
        if n != 0 {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_MISC_EXTRA_MOVES", "[ICON_BULLET]%D1_Change Extra Moves for %s2_Name", &[Arg::N(n), Arg::S(&names::display(d, dom))]));
        }
    }
    for k in 0..CommerceType::NUM {
        if info.commerce_flexible[k] && !g.team_is_commerce_flexible(team, CommerceType::from_usize(k)) {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_MISC_ADJUST_COMMERCE_RATE", "[ICON_BULLET]Can Adjust %F1_CmrcIcon Rate", &[Arg::S(COMMERCE_ICONS[k])]));
        }
    }
    for terrain in d.terrains.keys() {
        if info.terrain_trade[terrain] && !g.team_is_terrain_trade(team, terrain) {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_MISC_ENABLES_ON_TERRAIN", "[ICON_BULLET]Enables %F1_Icon on %s2_TerrainName", &[Arg::S("[ICON_TRADE]"), Arg::S(&names::display(d, terrain))]));
        }
    }
    if info.river_trade && !g.team_is_river_trade(team) {
        b.push_str(NEWLINE);
        let rivers = txt(d, "TXT_KEY_MISC_RIVERS", "Rivers", &[]);
        b.push_str(&txt(d, "TXT_KEY_MISC_ENABLES_ON_TERRAIN", "[ICON_BULLET]Enables %F1_Icon on %s2_TerrainName", &[Arg::S("[ICON_TRADE]"), Arg::S(&rivers)]));
    }
    for sb in d.special_buildings.keys() {
        let s = &d.special_buildings[sb];
        if s.tech_prereq == Some(t) {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_MISC_CAN_CONSTRUCT_BUILDING", "[ICON_BULLET]Can Construct [COLOR_BUILDING_TEXT]%s1_BldgName[COLOR_REVERT]", &[Arg::S(&names::display(d, sb))]));
        }
        if s.tech_prereq_anyone == Some(t) {
            b.push_str(NEWLINE);
            b.push_str(&txt(
                d,
                "TXT_KEY_MISC_CAN_CONSTRUCT_BUILDING_ANYONE",
                "[ICON_BULLET]Any player can Construct [COLOR_BUILDING_TEXT][LINK=literal]%s1_BldgName[\\LINK][COLOR_REVERT]",
                &[Arg::S(&names::display(d, sb))],
            ));
        }
    }
    // C++ `buildYieldChangeString`: what it adds to an improvement.
    for imp in d.improvements.keys() {
        changes_help(&mut b, &names::display(d, imp), ": ", "", &d.improvements[imp].tech_yield_changes[t], &YIELD_ICONS, false, true);
    }

    // Lists: resources revealed, civics enabled.
    let mut first = true;
    for bonus in d.bonuses.keys().filter(|&x| d.bonuses[x].tech_reveal == Some(t)) {
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_REVEALS", "[ICON_BULLET]Reveals ", &[]), &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, bonus)), ", ", first);
        first = false;
    }
    let ours = |c: Option<CivilizationType>| c.is_none() || c == g.player_get_civilization_type(p);
    let mut first = true;
    for &civic in &un.civics {
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_ENABLES", "[ICON_BULLET]Enables ", &[]), &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, civic)), ", ", first);
        first = false;
    }

    // The units, buildings and projects it allows that the player cannot have yet.
    for (i, &u) in un.units.iter().enumerate() {
        list_help(&mut b, &format!("{NEWLINE}{}", txt(d, "TXT_KEY_TECH_CAN_TRAIN", "[ICON_BULLET]Can Train ", &[])), &color("COLOR_UNIT_TEXT", &names::display(d, u)), ", ", i == 0);
    }
    for (i, &bl) in un.buildings.iter().enumerate() {
        list_help(&mut b, &format!("{NEWLINE}{}", txt(d, "TXT_KEY_TECH_CAN_CONSTRUCT", "[ICON_BULLET]Can Construct ", &[])), &color("COLOR_BUILDING_TEXT", &names::display(d, bl)), ", ", i == 0);
    }
    let mut first = true;
    for pr in d.projects.keys() {
        let pi = &d.projects[pr];
        if pi.tech_prereq == Some(t) && !g.player_is_production_maxed_project(p, pr) && !g.player_can_create(p, pr, false, true) && ours(pi.prereq_civilization) {
            list_help(&mut b, &format!("{NEWLINE}{}", txt(d, "TXT_KEY_TECH_CAN_CREATE", "[ICON_BULLET]Can Create ", &[])), &color("COLOR_PROJECT_TEXT", &names::display(d, pr)), ", ", first);
            first = false;
        }
    }
    let mut first = true;
    for pr in d.processes.keys().filter(|&x| d.processes[x].tech_prereq == Some(t)) {
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_CAN_BUILD", "[ICON_BULLET]Can Build ", &[]), &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, pr)), ", ", first);
        first = false;
    }
    // The religions the first to discover it founds (those not founded yet).
    let mut first = true;
    for &r in &un.religions {
        if first {
            b.push_str(NEWLINE);
        }
        let item = if g.is_option(GameOptionType::GAMEOPTION_PICK_RELIGION) { txt(d, "TXT_KEY_RELIGION_UNKNOWN", "a Religion", &[]) } else { color("COLOR_HIGHLIGHT_TEXT", &names::display(d, r)) };
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_FIRST_DISCOVER_FOUNDS", "[ICON_BULLET]First to Discover Founds ", &[]), &item, ", ", first);
        first = false;
    }
    let mut first = true;
    for c in d.corporations.keys() {
        if g.is_corporation_founded(c) || d.corporations[c].tech_prereq != Some(t) || !nobody_knows {
            continue;
        }
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_FIRST_DISCOVER_INCORPORATES", "[ICON_BULLET]First to Discover Incorporates ", &[]), &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, c)), ", ", first);
        first = false;
    }
    let mut first = true;
    for pr in d.promotions.keys().filter(|&x| d.promotions[x].tech_prereq == Some(t)) {
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, &txt(d, "TXT_KEY_MISC_ENABLES", "[ICON_BULLET]Enables ", &[]), &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, pr)), ", ", first);
        first = false;
    }
    // ffh-play's: the spells it allows (FfH2's help leaves them out).
    let mut first = true;
    for s in d.spells.keys().filter(|&x| d.spells[x].tech_prereq == Some(t) && ours(d.spells[x].civilization_prereq)) {
        if first {
            b.push_str(NEWLINE);
        }
        list_help(&mut b, "[ICON_BULLET]Enables spell ", &color("COLOR_HIGHLIGHT_TEXT", &names::display(d, s)), ", ", first);
        first = false;
    }

    // C++ `buildSingleLineTechTreeString`: the techs it leads to that the player cannot research yet.
    let mut first = true;
    for o in d.techs.keys() {
        if g.teams[team].has_tech[o] || g.player_can_research(p, o, false) || d.techs[o].disable {
            continue;
        }
        if d.techs[o].prereq_or_techs.contains(&t) || d.techs[o].prereq_and_techs.contains(&t) {
            list_help(&mut b, &txt(d, "TXT_KEY_MISC_LEADS_TO", "[NEWLINE][ICON_BULLET]Leads to ", &[]), &color("COLOR_TECH_TEXT", &names::display(d, o)), ", ", first);
            first = false;
        }
    }

    // The XML help.
    let help = names::text(d, &info.base.help);
    if !info.base.help.is_empty() && help != info.base.help && !help.trim().is_empty() {
        b.push_str(NEWLINE);
        b.push_str(help);
    }

    if g.player_can_research(p, t, false) {
        // The great people who can discover it (the player's own).
        for u in d.units.keys() {
            let ui = &d.units[u];
            let own = ui.unit_class_type.and_then(|c| civ_unit(g, p, c)) == Some(u);
            if own && (ui.base_discover > 0 || ui.discover_multiplier > 0) && g.get_discovery_tech(u, p) == Some(t) {
                b.push_str(NEWLINE);
                b.push_str(&txt(d, "TXT_KEY_TECH_GREAT_PERSON_DISCOVER", "[ICON_BULLET]Can be researched by a [COLOR_UNIT_TEXT]%s1_great_person[COLOR_REVERT]", &[Arg::S(&names::display(d, u))]));
            }
        }
        if let Some(era) = info.era.filter(|&e| pl.current_era < e) {
            b.push_str(NEWLINE);
            b.push_str(&txt(d, "TXT_KEY_TECH_ERA_ADVANCE", "[ICON_BULLET]Advances to the [COLOR_HIGHLIGHT_TEXT]%s1_era[COLOR_REVERT] Era", &[Arg::S(&names::display(d, era))]));
        }
    }
    // FfH: a tech of a state religion.
    if let Some(r) = info.prereq_religion.filter(|&r| g.player_get_state_religion(p) != Some(r)) {
        b.push_str(NEWLINE);
        b.push_str(&color("COLOR_NEGATIVE_TEXT", &txt(d, "TXT_KEY_TECH_PREREQ_RELIGION", "Requires: %s1_religion state religion", &[Arg::S(&names::display(d, r))])));
    }
    finish(b)
}
