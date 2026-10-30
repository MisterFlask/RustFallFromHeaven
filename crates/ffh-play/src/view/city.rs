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
/// started now, with cost (production needed) and turns.
pub fn buildable(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let d = g.d;
    let mut out = Vec::new();
    for u in d.units.keys() {
        if g.city_offers_unit(c, u) && g.city_can_train(c, u, false, true, false, false) {
            out.push(crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_TRAIN), "type" => Json::idx(u),
                "enabled" => g.city_can_train(c, u, false, false, false, false),
                "cost" => g.city_production_needed_unit(c, u), "turns" => g.city_production_turns_left_unit(c, u, 0),
            });
        }
    }
    for b in d.buildings.keys() {
        if g.city_can_construct(c, b, false, true, false) {
            out.push(crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_CONSTRUCT), "type" => Json::idx(b),
                "enabled" => g.city_can_construct(c, b, false, false, false),
                "cost" => g.city_production_needed_building(c, b), "turns" => g.city_production_turns_left_building(c, b, 0),
            });
        }
    }
    for p in d.projects.keys() {
        if g.city_can_create(c, p, false, true) {
            out.push(crate::jobj! {
                "order" => Json::idx(OrderType::ORDER_CREATE), "type" => Json::idx(p),
                "enabled" => g.city_can_create(c, p, false, false),
                "cost" => g.city_production_needed_project(c, p), "turns" => g.city_production_turns_left_project(c, p, 0),
            });
        }
    }
    for p in d.processes.keys() {
        if g.city_can_maintain(c, p, false) {
            out.push(crate::jobj! { "order" => Json::idx(OrderType::ORDER_MAINTAIN), "type" => Json::idx(p), "enabled" => true, "cost" => Json::Null, "turns" => Json::Null });
        }
    }
    Json::Arr(out)
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
        let mut j = crate::jobj! { "order" => Json::idx(o.order_type), "type" => k, "save" => o.save, "turns" => turns };
        // This game's armies: where a trained land combat unit goes.
        if g.armies_on() && o.order_type == OrderType::ORDER_TRAIN && ok(d.units.len()) && g.unit_type_is_army_unit(UnitType::from_usize(k as usize)) {
            j.set("dest", crate::command::recruit_dest_json(o.dest));
            j.set("dest_name", o.dest.map_or_else(|| "the city's default".to_string(), |x| g.recruit_dest_name(x)));
            j.set("reinforce_to", o.reinforce_to);
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
    add_part(&mut out, "festival", g.city_culture_happiness(c));
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
    step(&mut out, "espionage", None, city.espionage_happiness_counter.max(0));
    step(&mut out, "incited unrest", None, g.city_culture_unhappiness(c));
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
    let commerces = all::<CommerceType>(CommerceType::NUM).map(|k| crate::jobj! { "commerce" => Json::idx(k), "rate" => g.city_commerce_rate(c, k) });
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
        .map(|b| crate::jobj! { "building" => Json::idx(b), "real" => city.num_real_building[b], "free" => city.num_free_building[b] })
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
    Ok(crate::jobj! {
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
        "maintenance" => g.city_maintenance(c),
        "crime" => g.city_crime(c),
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
        "citizens_automated" => city.citizens_automated,
        "emphasize" => emphasize,
        "extra_population" => g.city_extra_population(c),
        "yields" => Json::Arr(yields.collect()),
        "commerces" => Json::Arr(commerces.collect()),
        "happiness" => happiness,
        "health" => health,
        "great_people" => crate::jobj! { "progress" => city.great_people_progress, "rate" => g.city_great_people_rate(c) },
        "plots" => Json::Arr(plots),
        "specialists" => Json::Arr(specialists.collect()),
        "free_specialists" => g.city_total_free_specialists(c),
        "buildings" => Json::Arr(buildings),
        "religions" => Json::Arr(religions),
        "garrison" => Json::Arr(garrison),
        "queue" => queue(v, c),
        "hurries" => Json::Arr(hurries),
        "can_conscript" => g.city_can_conscript(c),
        "buildable" => buildable(v, c),
        "cultural" => cultural(v, c),
        "recruit" => crate::view::armies::recruit(v, c),
        "governance" => crate::view::governors::city(v, c),
    })
}

/// The city's cultural queue and what it could channel (docs/CULTURE.md): `null` when the game
/// has no cultural actions.
fn cultural(v: &Viewer, c: CityId) -> Json {
    use ffh_core::culture::CultureTarget;
    let g = v.g;
    let d = g.d;
    if !g.culture_actions_on() {
        return Json::Null;
    }
    let city = &g.cities[c];
    let queue: Vec<Json> = city
        .culture_queue
        .iter()
        .map(|o| {
            let CultureTarget::City(t) = o.target;
            let mut j = crate::jobj! {
                "action" => Json::idx(o.action),
                "target" => t,
                "target_name" => if g.cities.contains(t) { Json::from(city_name(g, t)) } else { Json::Null },
                "religion" => Json::opt_idx(o.religion),
                "repeat" => o.repeat,
                "progress" => o.progress,
                "duration" => g.culture_action_duration(o.action),
                "stalled" => o.stalled,
                "started" => o.started_turn >= 0,
            };
            // Why it would pause now (its check this turn).
            if let Err(r) = g.culture_check(c, o.action, o.target, o.religion, if o.started_turn >= 0 { ffh_core::culture::CultureCheck::Channel } else { ffh_core::culture::CultureCheck::Start }) {
                j.set("paused", r.describe(d));
            }
            j
        })
        .collect();
    // Every action, with the targets in its reach: possible ones with their preview, the others
    // with the reason (only cities the human has seen).
    let (cx, cy) = (city.x, city.y);
    let team = g.players[v.h].team;
    let actions: Vec<Json> = d
        .cultural_actions
        .keys()
        .map(|a| {
            let info = &d.cultural_actions[a];
            let range = g.culture_tier_range(info.tier);
            let religions: Vec<Option<ReligionType>> = if info.effect == CultureEffect::SpreadReligion {
                d.religions.keys().filter(|&r| city.has_religion[r]).map(Some).collect()
            } else {
                vec![None]
            };
            let mut targets = Vec::new();
            for (t, tc) in g.cities.iter() {
                if g.plot_distance(cx, cy, tc.x, tc.y) > range || (tc.owner != v.h && !g.city_is_revealed(t, team, false)) {
                    continue;
                }
                for &r in &religions {
                    // A city that has the religion is no target for spreading it.
                    if r.is_some_and(|r| tc.has_religion[r]) {
                        continue;
                    }
                    let mut j = crate::jobj! { "city" => t, "name" => city_name(g, t), "owner" => tc.owner, "religion" => Json::opt_idx(r) };
                    match g.culture_check(c, a, CultureTarget::City(t), r, ffh_core::culture::CultureCheck::Queue) {
                        Ok(p) => {
                            j.set("ok", true);
                            j.set("effect", p.effectiveness);
                            j.set("hostile", p.hostile);
                            j.set("magnitude", p.magnitude);
                            j.set("effect_turns", p.effect_turns);
                        }
                        Err(e) => {
                            // Not the kind of city the action is for: not listed.
                            if matches!(e, ffh_core::culture::CultureRefusal::Target(_)) {
                                continue;
                            }
                            j.set("ok", false);
                            j.set("why", e.describe(d));
                        }
                    }
                    targets.push(j);
                }
            }
            crate::jobj! {
                "action" => Json::idx(a),
                "tier" => info.tier,
                "rate" => info.rate,
                "duration" => g.culture_action_duration(a),
                "range" => range,
                "targets" => Json::Arr(targets),
            }
        })
        .collect();
    let pressure: Vec<Json> = g
        .culture_hostile_channels_at(c)
        .into_iter()
        .filter(|ch| g.culture_channel_visible(team, ch.caster))
        .map(|ch| crate::view::culture::channel_json(v, &ch))
        .collect();
    crate::jobj! {
        "rate" => g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE),
        "diverted" => city.culture_diverted,
        "queue" => Json::Arr(queue),
        "queue_max" => d.defines.culture_queue_max,
        "actions" => Json::Arr(actions),
        "pressure" => Json::Arr(pressure),
        "marks" => crate::view::culture::marks_json(v, c),
        // The last successful Foment Revolt here (-1: none), which Flip City needs.
        "revolt_turn" => city.culture_revolt_turn,
    }
}
