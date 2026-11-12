//! The city screen (`city {city}`, design §4.6): the human's own cities only (C++ opens the city
//! screen for the active player's cities; debug reveal opens any city).
//!
//! Building a city screen leaves the save bytes as they were (tests/purity.rs): ffh-core's
//! `&self` functions, the happiness reads included, write nothing.

use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::query::QueryErr;
use crate::view::{city_name, production, unit_json, Viewer};

fn all<I: Idx>(n: usize) -> impl Iterator<Item = I> {
    (0..n).map(I::from_usize)
}

/// The buildable list: units, buildings, projects and processes the city screen shows
/// (`canTrain/canConstruct/canCreate(.., bTestVisible = true)`), `enabled` when they can be
/// started now, with cost (production needed) and turns. A governed city (docs/GOVERNORS.md)
/// lists only what may be an empire order, `enabled` when the city can take it as one now (the
/// governor builds the rest).
pub fn buildable(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let d = g.d;
    let governed = g.city_is_governed(c);
    let empire = |order: OrderType, data: usize| !governed || g.order_may_be_empire(order, data as i32);
    let mut out = Vec::new();
    for u in d.units.keys() {
        if g.city_offers_unit(c, u) && g.city_can_train(c, u, false, true, false, false) {
            let mut j = crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_TRAIN), "type" => Json::idx(u),
                "enabled" => g.city_can_train(c, u, false, false, false, false),
                "cost" => g.city_production_needed_unit(c, u), "turns" => g.city_production_turns_left_unit(c, u, 0),
            };
            set_national(g, c, u, &mut j);
            out.push(j);
        }
    }
    for b in d.buildings.keys() {
        if empire(OrderType::ORDER_CONSTRUCT, b.idx()) && g.city_can_construct(c, b, false, true, false) {
            let mut j = crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_CONSTRUCT), "type" => Json::idx(b),
                "enabled" => g.city_can_construct(c, b, false, false, false),
                "cost" => g.city_production_needed_building(c, b), "turns" => g.city_production_turns_left_building(c, b, 0),
            };
            set_tier(g, c, b, &mut j);
            out.push(j);
        }
    }
    for p in d.projects.keys() {
        if g.city_can_create(c, p, false, true) || crate::view::rituals::ritual_needs_tech(g, c, p).is_some() {
            let enabled = g.city_can_create(c, p, false, false);
            let mut j = crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_CREATE), "type" => Json::idx(p),
                "enabled" => enabled,
                "cost" => g.city_production_needed_project(c, p), "turns" => g.city_production_turns_left_project(c, p, 0),
            };
            // World rituals (docs/CARDS.md): the rules line and why it cannot be chosen now.
            crate::view::rituals::buildable_fields(g, c, p, enabled, &mut j);
            out.push(j);
        }
    }
    for p in d.processes.keys() {
        if !governed && g.city_can_maintain(c, p, false) {
            out.push(crate::jobj! { "order" => Json::idx(OrderType::ORDER_MAINTAIN), "type" => Json::idx(p), "enabled" => true, "cost" => Json::Null, "turns" => Json::Null });
        }
    }
    Json::Arr(out)
}

/// A national unit whose limit is a price (docs/PORTING.md, "Changes from FFH2"): how many its
/// owner has, how many it trains at the normal cost, and the cost percent past them.
fn set_national(g: &ffh_core::game::Game, c: CityId, u: UnitType, j: &mut Json) {
    let d = g.d;
    let Some(k) = d.units[u].unit_class_type.filter(|&k| ffh_core::utils::is_soft_limited_unit_class(d, k)) else { return };
    let owner = g.cities[c].owner;
    j.set("national_have", g.players[owner].unit_class_count[k]);
    j.set("national_normal", d.unit_classes[k].max_player_instances);
    j.set("national_over_percent", d.defines.national_unit_over_limit_cost_percent);
}

/// Building tiers (docs/SUITABILITY.md): a building's tier in the city (`tier`, the
/// `BuildingTierType` index), and for a wonder the cost its tier gives it there (`cost_percent`).
/// Nothing for a building without a tier.
fn set_tier(g: &ffh_core::game::Game, c: CityId, b: BuildingType, j: &mut Json) {
    let d = g.d;
    if let Some(t) = d.buildings[b].tier {
        j.set("tier", Json::idx(t));
    } else if g.city_wonder_cost_percent(c, b) != 100 || d.buildings[b].building_class_type.is_some_and(|k| d.building_classes[k].rolls_tier) {
        let t = g.city_building_tier(c, b);
        j.set("tier", Json::idx(t));
        j.set("cost_percent", g.city_wonder_cost_percent(c, b));
    }
}

/// The city's hand (docs/SUITABILITY.md): its building for each class it rolled a tier other than
/// Standard for (its own civilization's, at its tier; a wonder class's wonder), best tiers first.
fn tier_hand(g: &ffh_core::game::Game, c: CityId) -> Json {
    let d = g.d;
    let mut hand: Vec<(BuildingTierType, BuildingType)> = Vec::new();
    for k in d.building_classes.keys() {
        let t = g.city_tier(c, k);
        if !d.building_classes[k].rolls_tier || t == BuildingTierType::STANDARD {
            continue;
        }
        if let Some(b) = g.city_building_for_class(c, k) {
            hand.push((t, b));
        }
    }
    hand.sort_by_key(|&(t, b)| (std::cmp::Reverse(t), b));
    Json::Arr(
        hand.into_iter()
            .map(|(t, b)| {
                let mut j = crate::jobj! { "building" => Json::idx(b), "tier" => Json::idx(t) };
                if d.buildings[b].tier.is_none() {
                    j.set("cost_percent", g.city_wonder_cost_percent(c, b));
                }
                j
            })
            .collect(),
    )
}

/// The production queue with each order's turns (from what is stored, for the head order).
fn queue(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let city = &g.cities[c];
    let out = city.order_queue.iter().enumerate().map(|(i, o)| {
        let k = o.data1;
        let ok = |n: usize| k >= 0 && (k as usize) < n;
        let d = g.d;
        let turns = match o.order_type {
            OrderType::ORDER_TRAIN if ok(d.units.len()) => Json::from(g.city_production_turns_left_unit(c, UnitType::from_usize(k as usize), i as i32)),
            OrderType::ORDER_CONSTRUCT if ok(d.buildings.len()) => Json::from(g.city_production_turns_left_building(c, BuildingType::from_usize(k as usize), i as i32)),
            OrderType::ORDER_CREATE if ok(d.projects.len()) => Json::from(g.city_production_turns_left_project(c, ProjectType::from_usize(k as usize), i as i32)),
            _ => Json::Null,
        };
        let mut j = crate::jobj! { "order" => Json::idx(o.order_type), "type" => k, "save" => o.save, "turns" => turns, "empire" => o.empire };
        // This game's armies: where a trained land combat unit goes.
        if o.order_type == OrderType::ORDER_TRAIN && ok(d.units.len()) && g.unit_type_is_army_unit(UnitType::from_usize(k as usize)) {
            j.set("dest", crate::command::recruit_dest_json(o.dest));
            j.set("dest_name", o.dest.map_or_else(|| "the city's default".to_string(), |x| g.recruit_dest_name(x)));
            j.set("requisition", o.requisition);
        }
        j
    });
    Json::Arr(out.collect())
}

/// One line of a city's happiness or health: what it comes from and how much (a positive count).
type Part = (String, i32);

/// Adds `n` to the part labelled `label` (a new part when there is none), if `n` > 0.
fn add_part(parts: &mut Vec<Part>, label: &str, n: i32) {
    if n <= 0 {
        return;
    }
    match parts.iter_mut().find(|(l, _)| l == label) {
        Some(p) => p.1 += n,
        None => parts.push((label.to_string(), n)),
    }
}

/// `[[label, n], ..]`.
fn parts_json(parts: &[Part]) -> Json {
    Json::Arr(parts.iter().map(|(l, n)| crate::jarr![l.as_str(), *n]).collect())
}

/// The name of the one feature of the city's plots whose health is `good` (> 0) or bad (< 0),
/// else "features" (C++ `setGoodHealthHelp` / `setBadHealthHelp`).
fn feature_label(g: &ffh_core::game::Game, c: CityId, good: bool) -> String {
    let d = g.d;
    let mut only: Option<FeatureType> = None;
    for i in 0..g.city_num_city_plots(c).max(0) as usize {
        let Some(p) = g.city_index_plot(c, i) else { continue };
        let Some(f) = g.map.plots[p].feature_type else { continue };
        let hp = d.features[f].health_percent;
        if (good && hp > 0) || (!good && hp < 0) {
            match only {
                None => only = Some(f),
                Some(o) if o != f => return "features".to_string(),
                _ => {}
            }
        }
    }
    only.map_or_else(|| "features".to_string(), |f| crate::names::display(d, f).into_owned())
}

/// What makes city `c` happy: the terms of C++ `CvCity::happyLevel`, which they sum to (C++
/// `setHappyHelp`, with the city's and its owner's extra happiness as one term, as the rules
/// count them).
pub fn happy_parts(g: &ffh_core::game::Game, c: CityId) -> Vec<Part> {
    let city = &g.cities[c];
    let owner = city.owner;
    let player = &g.players[owner];
    let mut out = Vec::new();
    add_part(&mut out, "largest cities", g.city_largest_city_happiness(c));
    add_part(&mut out, "military units", g.city_military_happiness(c));
    add_part(&mut out, "vassals", g.city_vassal_happiness(c));
    add_part(&mut out, "state religion", g.city_current_state_religion_happiness(c));
    add_part(&mut out, "buildings", city.building_good_happiness.max(0) + city.extra_building_good_happiness.max(0));
    add_part(&mut out, "features", city.feature_good_happiness);
    add_part(&mut out, "resources", city.bonus_good_happiness);
    add_part(&mut out, "religions", city.religion_good_happiness);
    add_part(&mut out, "luxury", g.city_commerce_happiness(c));
    add_part(&mut out, "buildings", g.map.areas[g.city_area(c)].building_happiness[owner]);
    add_part(&mut out, "buildings", player.building_happiness);
    add_part(&mut out, "other", city.extra_happiness + player.extra_happiness);
    add_part(&mut out, "difficulty", g.d.handicaps[g.city_handicap_type(c)].happy_bonus);
    if city.happiness_timer > 0 {
        add_part(&mut out, &format!("past events ({} turns)", city.happiness_timer), g.d.defines.temp_happy);
    }
    out
}

/// What makes city `c` unhappy: the terms of C++ `CvCity::unhappyLevel`, which they sum to
/// (C++ `setAngerHelp`: each percent anger turns into citizens by the difference it makes to the
/// running total, so that the rounding of the whole is shared out and the parts add up).
pub fn unhappy_parts(g: &ffh_core::game::Game, c: CityId) -> Vec<Part> {
    let d = g.d;
    let mut out = Vec::new();
    if g.city_is_no_unhappiness(c) {
        return out;
    }
    let city = &g.cities[c];
    let owner = city.owner;
    let player = &g.players[owner];
    let pop = city.population;
    let div = d.defines.percent_anger_divisor;
    let (mut old_pct, mut new_pct, mut old, mut new) = (0, 0, 0, 0);
    let mut step = |out: &mut Vec<Part>, label: &str, pct: Option<i32>, flat: i32| {
        match pct {
            Some(p) => {
                new_pct += p;
                new += (new_pct * pop) / div - (old_pct * pop) / div;
                old_pct = new_pct;
            }
            None => new += flat,
        }
        add_part(out, label, (new - old) + old.min(0));
        old = new;
    };
    step(&mut out, "population", Some(g.city_overcrowding_percent_anger(c, 0)), 0);
    step(&mut out, "no military protection", Some(g.city_no_military_percent_anger(c)), 0);
    step(&mut out, "foreign culture", Some(g.city_culture_percent_anger(c)), 0);
    step(&mut out, "war on its religion", Some(g.city_religion_percent_anger(c)), 0);
    step(&mut out, "hurried production", Some(g.city_hurry_percent_anger(c, 0)), 0);
    step(&mut out, "draft", Some(g.city_conscript_percent_anger(c, 0)), 0);
    step(&mut out, "defied resolution", Some(g.city_defy_resolution_percent_anger(c, 0)), 0);
    step(&mut out, "war weariness", Some(g.city_war_weariness_percent_anger(c)), 0);
    step(&mut out, "being a vassal", None, g.city_vassal_unhappiness(c).max(0));
    step(&mut out, "revolt pressure", None, g.city_culture_unhappiness(c));
    for civic in d.civics.keys() {
        let label = format!("wants {}", crate::names::display(d, civic));
        step(&mut out, &label, Some(g.player_get_civic_percent_anger(owner, civic, false)), 0);
    }
    step(&mut out, "largest cities", None, -g.city_largest_city_happiness(c).min(0));
    step(&mut out, "military units", None, -g.city_military_happiness(c).min(0));
    step(&mut out, "state religion", None, -g.city_current_state_religion_happiness(c).min(0));
    step(&mut out, "buildings", None, -city.building_bad_happiness.min(0) - city.extra_building_bad_happiness.min(0));
    step(&mut out, "features", None, -city.feature_bad_happiness.min(0));
    step(&mut out, "resources", None, -city.bonus_bad_happiness.min(0));
    step(&mut out, "religions", None, -city.religion_bad_happiness.min(0));
    step(&mut out, "luxury", None, -g.city_commerce_happiness(c).min(0));
    step(&mut out, "buildings", None, -g.map.areas[g.city_area(c)].building_happiness[owner].min(0));
    step(&mut out, "buildings", None, -player.building_happiness.min(0));
    step(&mut out, "other", None, -(city.extra_happiness + player.extra_happiness).min(0));
    step(&mut out, "difficulty", None, -d.handicaps[g.city_handicap_type(c)].happy_bonus.min(0));
    out
}

/// What keeps city `c` healthy: the terms of C++ `CvCity::goodHealth`, which they sum to.
pub fn good_health_parts(g: &ffh_core::game::Game, c: CityId) -> Vec<Part> {
    let city = &g.cities[c];
    let mut out = Vec::new();
    add_part(&mut out, "fresh water", city.fresh_water_good_health);
    if city.feature_good_health > 0 {
        add_part(&mut out, &feature_label(g, c, true), city.feature_good_health);
    }
    add_part(&mut out, "power", city.power_good_health);
    add_part(&mut out, "resources", city.bonus_good_health);
    add_part(&mut out, "buildings", g.city_total_good_building_health(c));
    add_part(&mut out, "other", g.players[city.owner].extra_health + city.extra_health);
    add_part(&mut out, "difficulty", g.d.handicaps[g.city_handicap_type(c)].health_bonus);
    out
}

/// What makes city `c` unhealthy: the terms of C++ `CvCity::badHealth`, which they sum to (its
/// population first; the extra building bad health, which the rules count twice, is in
/// "buildings").
pub fn bad_health_parts(g: &ffh_core::game::Game, c: CityId) -> Vec<Part> {
    let city = &g.cities[c];
    let mut out = Vec::new();
    add_part(&mut out, "population", g.city_unhealthy_population(c, false, 0));
    add_part(&mut out, "blight", city.espionage_health_counter);
    add_part(&mut out, "fresh water", -city.fresh_water_bad_health);
    if city.feature_bad_health < 0 {
        add_part(&mut out, &feature_label(g, c, false), -city.feature_bad_health);
    }
    add_part(&mut out, "power", -city.power_bad_health);
    add_part(&mut out, "resources", -city.bonus_bad_health);
    add_part(&mut out, "buildings", -g.city_total_bad_building_health(c).min(0) - city.extra_building_bad_health.min(0));
    add_part(&mut out, "other", -(g.players[city.owner].extra_health + city.extra_health));
    add_part(&mut out, "difficulty", -g.d.handicaps[g.city_handicap_type(c)].health_bonus);
    out
}

/// City `c`'s commerce `k` by its sources, times 100 (C++ `getBaseCommerceRateTimes100`,
/// `updateCommerce`): [from the city's commerce at the owner's rate, specialists, buildings,
/// religions and corporations, the empire's free commerce, processes].
fn commerce_parts(g: &ffh_core::game::Game, c: CityId, k: CommerceType) -> [i32; 6] {
    let i = k.idx();
    let city = &g.cities[c];
    let player = &g.players[city.owner];
    let mut specialists = 100 * city.specialist_commerce[i];
    for s in g.d.specialists.keys() {
        specialists += 100 * (city.specialist_count[s] + city.free_specialist_count[s]) * (player.specialist_extra_commerce[i] + player.specialist_type_extra_commerce[s][i]);
    }
    [
        g.city_commerce_from_percent(c, k, g.city_yield_rate(c, YieldType::YIELD_COMMERCE) * 100),
        specialists,
        100 * city.building_commerce[i],
        100 * (city.religion_commerce[i] + city.corporation_commerce[i]),
        100 * player.free_city_commerce[i],
        g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) * city.production_to_commerce_modifier[i],
    ]
}

/// `city {city}`: the full city screen.
pub fn city(v: &Viewer, c: CityId) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    let Some(city) = g.cities.get(c) else { return Err(QueryErr::Gone) };
    if city.owner != v.h && !v.reveal {
        return Err(QueryErr::NotOwner);
    }
    // City plots: yields, worked here or by another city.
    let mut plots = Vec::new();
    for i in 0..g.city_num_city_plots(c).max(0) as usize {
        let Some(p) = g.city_index_plot(c, i) else { continue };
        let pl = &g.map.plots[p];
        plots.push(crate::jobj! {
            "i" => i,
            "x" => pl.x,
            "y" => pl.y,
            "revealed" => v.plot_revealed(p),
            "owner" => v.plot_owner(p),
            "worked" => g.city_is_working_plot(c, p),
            // A click on it works it or stops working it (`alterWorkingPlot`); else, on our
            // plot, it takes the plot from the city working it (the working override).
            "can_work" => g.city_can_work(c, p),
            "worked_by" => pl.working_city.filter(|&w| w != c && v.city_revealed(w)),
            "override" => pl.working_city_override,
            "yields" => if v.plot_visible(p) { Json::from(pl.yields.to_vec()) } else { Json::Null },
        });
    }
    let specialists = d.specialists.keys().map(|s| {
        // The +/- buttons as the city screen shows them (what `do_task CHANGE_SPECIALIST`
        // accepts).
        let (plus, minus) = crate::command::specialist_buttons(g, c, s);
        crate::jobj! {
            "specialist" => Json::idx(s),
            "count" => city.specialist_count[s],
            "forced" => city.force_specialist_count[s],
            "free" => city.free_specialist_count[s],
            "max" => city.max_specialist_count[s],
            "can_add" => g.city_is_specialist_valid(c, s, 1),
            "plus" => plus,
            "minus" => minus,
        }
    });
    let emphasize: Vec<bool> = d.emphasizes.keys().map(|e| city.ai_emphasize[e]).collect();
    let yields = all::<YieldType>(YieldType::NUM).map(|y| {
        crate::jobj! { "yield" => Json::idx(y), "rate" => g.city_yield_rate(c, y), "base" => city.base_yield_rate[y.idx()], "modifier" => city.yield_rate_modifier[y.idx()] }
    });
    let commerces = all::<CommerceType>(CommerceType::NUM).map(|k| {
        // Where the commerce comes from (`getBaseCommerceRateTimes100`, `updateCommerce`): the
        // city's commerce split by the owner's rates, then the specialists', buildings', religions'
        // and the empire's own, the processes' share of production, all times the modifier. The
        // parts are whole points (rounded down), so they can fall short of the rate by a point.
        let mut parts = commerce_parts(g, c, k);
        if k == CommerceType::COMMERCE_CULTURE {
            let spy = commerce_parts(g, c, CommerceType::COMMERCE_ESPIONAGE);
            for (a, b) in parts.iter_mut().zip(spy) {
                *a += b;
            }
        }
        crate::jobj! {
            "commerce" => Json::idx(k),
            "rate" => g.city_commerce_rate(c, k),
            "percent" => g.players[city.owner].commerce_percent[k.idx()],
            "rates" => parts[0] / 100,
            "specialists" => parts[1] / 100,
            "buildings" => parts[2] / 100,
            "religions" => parts[3] / 100,
            "empire" => parts[4] / 100,
            "process" => parts[5] / 100,
            "modifier" => g.city_total_commerce_rate_modifier(c, k) - 100,
        }
    });
    // What a click on a plot the city does not work takes (`alterWorkingPlot`): a citizen without
    // work, else the one the governor would free (`city_worst_citizen`).
    let next_worker = if g.city_extra_population(c) > 0 {
        crate::jobj! { "idle" => true }
    } else {
        match g.city_worst_citizen(c, None) {
            Some(ffh_core::governor::WorstCitizen::Specialist(s)) => crate::jobj! { "specialist" => Json::idx(s) },
            Some(ffh_core::governor::WorstCitizen::Plot(i)) => crate::jobj! { "plot" => i },
            None => Json::Null,
        }
    };
    let happiness = crate::jobj! {
        "happy" => g.city_happy_level(c),
        "unhappy" => g.city_unhappy_level(c, 0),
        "angry" => g.city_angry_population(c, 0),
        "buildings" => city.building_good_happiness + city.extra_building_good_happiness,
        "buildings_bad" => city.building_bad_happiness + city.extra_building_bad_happiness,
        "religions" => city.religion_good_happiness,
        "religions_bad" => city.religion_bad_happiness,
        "bonuses" => city.bonus_good_happiness,
        "bonuses_bad" => city.bonus_bad_happiness,
        "features" => city.feature_good_happiness,
        "features_bad" => city.feature_bad_happiness,
        "military" => g.city_military_happiness(c),
        "largest_city" => g.city_largest_city_happiness(c),
        "commerce" => g.city_commerce_happiness(c),
        "extra" => city.extra_happiness,
        "war_weariness_percent_anger" => g.city_war_weariness_percent_anger(c),
        "hurry_anger_timer" => city.hurry_anger_timer,
        "conscript_anger_timer" => city.conscript_anger_timer,
        // The terms the totals are made of (labels and counts, each list sums to its total).
        "happy_parts" => parts_json(&happy_parts(g, c)),
        "unhappy_parts" => parts_json(&unhappy_parts(g, c)),
    };
    let health = crate::jobj! {
        "good" => g.city_good_health(c),
        "bad" => g.city_bad_health(c, false, 0),
        "unhealthy_population" => g.city_unhealthy_population(c, false, 0),
        "fresh_water" => city.fresh_water_good_health,
        "fresh_water_bad" => city.fresh_water_bad_health,
        "features" => city.feature_good_health,
        "features_bad" => city.feature_bad_health,
        "buildings" => city.building_good_health,
        "buildings_bad" => city.building_bad_health,
        "power" => city.power_good_health,
        "power_bad" => city.power_bad_health,
        "bonuses" => city.bonus_good_health,
        "bonuses_bad" => city.bonus_bad_health,
        "extra" => city.extra_health,
        "good_parts" => parts_json(&good_health_parts(g, c)),
        "bad_parts" => parts_json(&bad_health_parts(g, c)),
    };
    let buildings: Vec<Json> = d
        .buildings
        .keys()
        .filter(|&b| city.num_real_building[b] > 0 || city.num_free_building[b] > 0)
        .map(|b| {
            let mut j = crate::jobj! { "building" => Json::idx(b), "real" => city.num_real_building[b], "free" => city.num_free_building[b] };
            set_tier(g, c, b, &mut j);
            j
        })
        .collect();
    let religions: Vec<Json> = d
        .religions
        .keys()
        .filter(|&r| city.has_religion[r])
        .map(|r| crate::jobj! { "religion" => Json::idx(r), "holy" => g.city_is_holy_city(c, r) })
        .collect();
    let plot = g.city_plot(c);
    let garrison: Vec<Json> = g.map.plots[plot].units.iter().filter(|&&u| v.unit_visible(u)).map(|&u| unit_json(v, u)).collect();
    let hurries: Vec<Json> = d
        .hurries
        .keys()
        .filter(|&hu| g.city_can_hurry(c, hu, true))
        .map(|hu| crate::jobj! {
            "hurry" => Json::idx(hu), "enabled" => g.city_can_hurry(c, hu, false),
            "gold" => g.city_hurry_gold(c, hu), "population" => g.city_hurry_population(c, hu),
        })
        .collect();
    let mut out = crate::jobj! {
        "id" => c,
        "name" => city_name(g, c),
        "owner" => city.owner,
        "x" => city.x,
        "y" => city.y,
        "size" => city.population,
        "capital" => g.city_is_capital(c),
        "founded" => city.game_turn_founded,
        "culture_level" => Json::opt_idx(city.culture_level),
        "culture" => g.city_culture(c, city.owner),
        "culture_threshold" => g.city_culture_threshold(c),
        "defense" => g.city_defense_modifier(c, false),
        "defense_damage" => city.defense_damage,
        // Sieges (docs/SIEGES.md): besieged (by how many units), and each Sentinel's HP left in
        // percent with its max HP in a battle.
        "besieged" => g.city_is_besieged(c),
        "besiegers" => g.city_besiegers(c).len(),
        // Named seas (docs/NAVAL.md, 4.4 and 10): the harbour, open or blockaded (null on no sea).
        "harbour" => crate::view::seas::harbour_json(v, c),
        "sentinels" => sentinels_json(g, c),
        "sentinel_max_hp" => g.city_sentinel_max_hp(c),
        "maintenance" => g.city_maintenance(c),
        "trade_routes" => g.city_trade_routes(c),
        "disorder" => g.city_is_disorder(c),
        "occupation" => city.occupation_timer,
        "food" => crate::jobj! {
            "stored" => city.food,
            "threshold" => g.city_growth_threshold(c),
            "difference" => g.city_food_difference(c, true),
            "consumption" => g.city_food_consumption(c, false, 0),
            "turns" => g.city_food_turns_left(c),
        },
        "production" => crate::jobj! {
            "current" => production(g, c),
            "stored" => g.city_production(c),
            "needed" => if city.order_queue.is_empty() { Json::Null } else { Json::from(g.city_production_needed(c)) },
            "rate" => g.city_current_production_difference(c, false, true),
            "automated" => city.production_automated,
        },
        // This game's city works (docs/WORKS.md): the crews' work rate and their build.
        "crews" => crate::jobj! {
            "rate" => g.city_crew_rate(c),
            "build" => match city.crew_build {
                Some((q, b)) => crate::jobj! { "build" => Json::idx(b), "x" => g.map.xy(q).0, "y" => g.map.xy(q).1, "turns" => g.city_crew_turns_left(c, q, b).map_or(Json::Null, Json::from) },
                None => Json::Null,
            },
        },
        "citizens_automated" => city.citizens_automated,
        "emphasize" => emphasize,
        "extra_population" => g.city_extra_population(c),
        "next_worker" => next_worker,
        "yields" => Json::Arr(yields.collect()),
        "commerces" => Json::Arr(commerces.collect()),
        "happiness" => happiness,
        "health" => health,
        "great_people" => crate::jobj! { "progress" => city.great_people_progress, "rate" => g.city_great_people_rate(c) },
        "plots" => Json::Arr(plots),
        "specialists" => Json::Arr(specialists.collect()),
        "free_specialists" => g.city_total_free_specialists(c),
        "buildings" => Json::Arr(buildings),
        "tier_hand" => tier_hand(g, c),
        "religions" => Json::Arr(religions),
        "garrison" => Json::Arr(garrison),
        "queue" => queue(v, c),
        "hurries" => Json::Arr(hurries),
        "can_conscript" => g.city_can_conscript(c),
        "buildable" => buildable(v, c),
        "cultural" => cultural(v, c),
        "recruit" => crate::view::armies::recruit(v, c),
        "governance" => crate::view::governors::city(v, c),
        // This game's outposts (docs/NAVAL.md, 9 and 10): the lane, starving, raising; null for a city.
        "outpost" => crate::view::outposts::city(v, c),
    };
    // World rituals (docs/CARDS.md): the ritual in progress at the head of the queue (asked before
    // it is taken off) and the hurry buttons' note.
    crate::view::rituals::city_fields(g, c, &mut out);
    Ok(out)
}

/// The city's cultural channel and what it could channel (docs/CULTURE.md): `null` when the
/// game has no cultural actions. `options`: every channel it could set, one per civilization the
/// human has met (and the barbarians) and action (and religion of the city), with whether it may,
/// its reach and a spread's chance; `pressure`: the revolt pressure on the city.
fn cultural(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let d = g.d;
    let city = &g.cities[c];
    let team = g.players[v.h].team;
    let channel = g
        .culture_channels()
        .into_iter()
        .find(|ch| ch.city == c)
        .map_or(Json::Null, |ch| crate::view::culture::channel_json(v, &ch));
    let mut options = Vec::new();
    if city.owner == v.h {
        let targets: Vec<PlayerId> = g
            .player_ids()
            .filter(|&t| g.players[t].alive && !g.players[t].cities.is_empty())
            .filter(|&t| t == g.barbarian_player() || g.players[t].team == team || g.teams[team].has_met[g.players[t].team])
            .collect();
        for a in d.cultural_actions.keys() {
            let religions: Vec<Option<ReligionType>> = match d.cultural_actions[a].effect {
                CultureEffect::SpreadReligion => d.religions.keys().filter(|&r| city.has_religion[r]).map(Some).collect(),
                CultureEffect::FomentRevolt => vec![None],
            };
            for &t in &targets {
                if d.cultural_actions[a].effect == CultureEffect::FomentRevolt && g.players[t].team == g.players[city.owner].team {
                    continue;
                }
                for &r in &religions {
                    let o = crate::view::culture::option_json(v, c, a, t, r);
                    // Nothing in reach at all: not an option worth listing.
                    if o.get("reach").and_then(|r| r.as_arr()).is_some_and(|r| r.is_empty()) && o.get("unseen").and_then(|u| u.as_i64()) == Some(0) {
                        continue;
                    }
                    options.push(o);
                }
            }
        }
    }
    crate::jobj! {
        "rate" => g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE),
        "amount" => g.city_channel_amount(c),
        "diverted" => city.culture_diverted,
        "channel" => channel,
        "options" => Json::Arr(options),
        "pressure" => crate::view::culture::pressure_json(v, c),
        "thresholds" => crate::view::culture::thresholds_json(v),
    }
}

/// City `c`'s Sentinels (docs/SIEGES.md): each one's HP left in percent (0: breached).
pub fn sentinels_json(g: &ffh_core::game::Game, c: CityId) -> Json {
    Json::Arr((0..g.city_sentinel_count(c)).map(|k| Json::from(100 - g.city_sentinel_damage(c, k))).collect())
}
