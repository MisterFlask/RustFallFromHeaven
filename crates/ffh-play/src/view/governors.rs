//! This game's governors as the human sees them (docs/GOVERNORS.md): the city screen's
//! `governance`, the Governors screen (`governors` query: who runs the cities, the empire
//! priorities, the diagnoses, a role card per governed city) and the two steps of an empire order
//! (`empire_items`: what may be ordered, and how many governed cities can build each;
//! `empire_cities`: for one item, every governed city able to build it, with a suggestion). (An
//! army's requisition interrupts a governed city as any city: the Armies screen, `view::armies`.)

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::governors::GovDiagnosis;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::query::QueryErr;
use crate::view::{city_name, Viewer};

/// The human's government civic (its civic of `CIVICOPTION_GOVERNMENT`), if any.
fn government(g: &Game, h: PlayerId) -> Option<CivicType> {
    let d = g.d;
    d.civic_options.keys().find(|&o| d.civic_options[o].base.type_ == "CIVICOPTION_GOVERNMENT").and_then(|o| g.players[h].civics[o])
}

/// A governor: name, trait, the city it holds.
fn governor_json(v: &Viewer, g: GovernorId) -> Json {
    let gv = &v.g.governors[g];
    crate::jobj! {
        "id" => g,
        "name" => gv.name.as_str(),
        "trait" => gv.trait_.map(Json::idx),
        "city" => gv.city,
        "since" => gv.since_turn,
        "born" => gv.born_turn,
    }
}

/// Order `i` of city `c`'s queue: `index`, `order`, `type`, `turns` (left to finish it there),
/// `empire`, `requisition`, `dest_name` (a unit's destination; null for the city's own garrison
/// or default).
pub fn order_json(g: &Game, c: CityId, i: usize) -> Json {
    let o = &g.cities[c].order_queue[i];
    crate::jobj! {
        "index" => i,
        "order" => Json::idx(o.order_type),
        "type" => o.data1,
        "turns" => g.city_order_turns_left(c, i),
        "empire" => o.empire,
        "requisition" => o.requisition,
        "dest_name" => o.dest.map(|x| g.recruit_dest_name(x)),
    }
}

/// The role card of governed city `c`: every role with its suitability and whether the human may
/// choose it (`why` not), the current role's key buildings with the city's tier of each (`none`:
/// its civilization cannot build the class), the governor's plan (`ffh_ai::governor::plan`), the
/// city's empire and requisition orders, and the governor's own order (`governor_order`: the first
/// order behind them, or null).
fn role_card(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let d = g.d;
    let civ = g.city_civilization_type(c);
    let roles: Vec<Json> = d
        .city_roles
        .keys()
        .map(|r| {
            let why = g.player_check_city_role(v.h, c, r).err();
            crate::jobj! { "role" => Json::idx(r), "suitability" => g.city_role_suitability(c, r), "allowed" => why.is_none(), "why" => why }
        })
        .collect();
    let keys: Vec<Json> = g
        .city_role(c)
        .map(|role| {
            d.city_roles[role]
                .key_buildings
                .iter()
                .map(|&k| {
                    let none = d.civ_building(civ, k).is_none();
                    crate::jobj! {
                        "class" => Json::idx(k),
                        "building" => g.city_building_for_class(c, k).map(Json::idx),
                        "tier" => Json::idx(g.city_tier(c, k)),
                        "effect" => d.building_tiers[g.city_tier(c, k)].core_effect_percent,
                        "none" => none,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let lead = g.city_lead_orders(c);
    let q = &g.cities[c].order_queue;
    crate::jobj! {
        "roles" => Json::Arr(roles),
        "key_buildings" => Json::Arr(keys),
        "plan" => Json::Arr(ffh_ai::governor::plan(g, c, 3).into_iter().map(Json::idx).collect()),
        "orders" => Json::Arr((0..lead).map(|i| order_json(g, c, i)).collect()),
        "governor_order" => (lead < q.len()).then(|| order_json(g, c, lead)),
    }
}

/// One of the human's cities as the Governors screen and the city screen show it.
fn city_json(v: &Viewer, c: CityId, card: bool) -> Json {
    let g = v.g;
    let city = &g.cities[c];
    let governed = g.city_is_governed(c);
    let mut j = crate::jobj! {
        "city" => c,
        "name" => city_name(g, c),
        "capital" => g.city_is_capital(c),
        "governed" => governed,
        "governor" => city.governor.filter(|&x| governed && g.governors.contains(x)).map(|x| governor_json(v, x)),
        "role" => g.city_role(c).map(Json::idx),
        "role_overridden" => city.role_overridden,
        "proposal" => governed.then(|| Json::idx(g.city_role_proposal(c))),
        "suitability" => g.city_role(c).map(|r| g.city_role_suitability(c, r)),
        "production" => g.city_yield_rate(c, YieldType::YIELD_PRODUCTION),
        "population" => city.population,
        // This game's armies: the armies that requisition from the city (their names).
        "requisitioned_by" => g.players[v.h].armies.iter().filter(|&&a| g.armies.contains(a) && g.armies[a].requisition.contains(&c)).map(|&a| Json::from(g.armies[a].name.as_str())).collect::<Vec<_>>(),
    };
    if card && governed {
        j.set("card", role_card(v, c));
    }
    j
}

/// The city screen's `governance`: the city's rule, role and governor (null for another's city).
pub fn city(v: &Viewer, c: CityId) -> Json {
    if v.g.cities[c].owner != v.h {
        return Json::Null;
    }
    let mut j = city_json(v, c, false);
    j.set("government", government(v.g, v.h).map(Json::idx));
    j
}

/// A diagnosis, with the names the screen shows (a slow order's order is the city's first empire
/// order).
fn diagnosis_json(g: &Game, d: &GovDiagnosis) -> Json {
    match *d {
        GovDiagnosis::RoleUnfilled(r) => crate::jobj! { "kind" => "role_unfilled", "role" => Json::idx(r) },
        GovDiagnosis::PoorFit { city, role, suitability } => crate::jobj! { "kind" => "poor_fit", "city" => city, "name" => city_name(g, city), "role" => Json::idx(role), "suitability" => suitability },
        GovDiagnosis::SlowOrder { city, turns } => {
            let o = g.cities[city].order_queue.iter().find(|o| o.empire);
            crate::jobj! {
                "kind" => "slow_order", "city" => city, "name" => city_name(g, city), "turns" => turns,
                "order" => o.map(|o| Json::idx(o.order_type)), "type" => o.map(|o| o.data1),
            }
        }
    }
}

/// `governors`: the Governors screen. The government and whether it rules every city directly,
/// the priorities (with their weight) and the diagnoses, and every city of the human's (the
/// capital first; a governed city with its role card).
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let mut cities: Vec<CityId> = g.players[h].cities.clone();
    cities.sort_by_key(|&c| (!g.city_is_capital(c), c));
    crate::jobj! {
        "government" => government(g, h).map(Json::idx),
        "direct_rule" => g.player_rules_directly(h),
        "min_suitability" => g.d.defines.governor_role_min_suitability,
        "max_priorities" => g.d.defines.governor_max_priorities,
        "priority_weight" => g.d.defines.governor_priority_weight,
        "priorities" => Json::Arr(g.players[h].governor_priorities.iter().map(|&r| Json::idx(r)).collect()),
        "diagnoses" => Json::Arr(g.player_governor_diagnoses(h).iter().map(|x| diagnosis_json(g, x)).collect()),
        "cities" => Json::Arr(cities.iter().map(|&c| city_json(v, c, true)).collect()),
    }
}

// ---------------------------------------------------------------------------------------------
// Empire orders: the two steps
// ---------------------------------------------------------------------------------------------

/// The orders an empire order may be: units, then wonders and palaces, then projects (every one
/// [`Game::order_may_be_empire`] takes).
fn empire_kinds(g: &Game) -> Vec<(OrderType, i32)> {
    let d = g.d;
    let mut out: Vec<(OrderType, i32)> = d.units.keys().map(|u| (OrderType::ORDER_TRAIN, u.idx() as i32)).collect();
    out.extend(d.buildings.keys().map(|b| (OrderType::ORDER_CONSTRUCT, b.idx() as i32)).filter(|&(o, b)| g.order_may_be_empire(o, b)));
    out.extend(d.projects.keys().map(|p| (OrderType::ORDER_CREATE, p.idx() as i32)));
    out
}

/// Whether city `c` shows order (`order`, `data`) in its build list (`bTestVisible`).
fn city_shows(g: &Game, c: CityId, order: OrderType, data: i32) -> bool {
    match order {
        OrderType::ORDER_TRAIN => {
            let u = UnitType::from_usize(data as usize);
            g.city_offers_unit(c, u) && g.city_can_train(c, u, false, true, false, false)
        }
        OrderType::ORDER_CONSTRUCT => g.city_can_construct(c, BuildingType::from_usize(data as usize), false, true, false),
        OrderType::ORDER_CREATE => g.city_can_create(c, ProjectType::from_usize(data as usize), false, true),
        _ => false,
    }
}

/// The human's army `a` (`Gone` / `NotOwner` otherwise).
fn army_of(v: &Viewer, a: ArmyId) -> Result<(), QueryErr> {
    match v.g.armies.get(a) {
        None => Err(QueryErr::Gone),
        Some(x) if x.owner != v.h => Err(QueryErr::NotOwner),
        Some(_) => Ok(()),
    }
}

/// `empire_items {army?}`: what the human may order from its governed cities: every unit,
/// wonder, palace and project one of its cities shows in its build list, each with `cities` (how
/// many governed cities can build it now: 0 is unavailable) and, for a unit, `army_unit`. With
/// `army`, only the units that serve in armies. `governed`: how many governed cities the human has.
pub fn empire_items(v: &Viewer, army: Option<ArmyId>) -> Result<Json, QueryErr> {
    let g = v.g;
    if let Some(a) = army {
        army_of(v, a)?;
    }
    let cities: Vec<CityId> = g.players[v.h].cities.iter().copied().filter(|&c| g.cities.contains(c)).collect();
    let governed: Vec<CityId> = cities.iter().copied().filter(|&c| g.city_is_governed(c)).collect();
    let mut items = Vec::new();
    for (order, data) in empire_kinds(g) {
        let army_unit = order == OrderType::ORDER_TRAIN && g.unit_type_is_army_unit(UnitType::from_usize(data as usize));
        if army.is_some() && !army_unit {
            continue;
        }
        if !cities.iter().any(|&c| city_shows(g, c, order, data)) {
            continue;
        }
        let n = governed.iter().filter(|&&c| g.city_check_empire_order(v.h, c, order, data).is_ok()).count();
        let mut j = crate::jobj! { "order" => Json::idx(order), "type" => data, "cities" => n };
        if order == OrderType::ORDER_TRAIN {
            j.set("army_unit", army_unit);
        }
        items.push(j);
    }
    Ok(crate::jobj! { "army" => army, "governed" => governed.len(), "items" => Json::Arr(items) })
}

/// `empire_cities {order, data, army?}`: every governed city of the human's that can build the
/// item (`city_check_empire_order`), each with `turns` (to finish it there, behind the orders
/// ahead of it: `ahead` of them, `ahead_turns` turns), `own_turns` (the item's own), `displaces`
/// (the governor's order it goes ahead of, with its turns; null: none) delayed by `own_turns`,
/// `distance` (plots to the army, with one), `cost_percent` (a wonder's cost there, from its
/// building tier) and one `suggested` (with an army: the nearest, ties to the fewest turns;
/// without: the fewest turns, ties to the first city).
pub fn empire_cities(v: &Viewer, order: OrderType, data: i32, army: Option<ArmyId>) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    let n = match order {
        OrderType::ORDER_TRAIN => d.units.len(),
        OrderType::ORDER_CONSTRUCT => d.buildings.len(),
        OrderType::ORDER_CREATE => d.projects.len(),
        _ => return Err(crate::query::invalid("field `order`: ORDER_TRAIN, ORDER_CONSTRUCT or ORDER_CREATE")),
    };
    if data < 0 || data as usize >= n {
        return Err(crate::query::invalid(format!("field `data`: {data} out of range 0..{n}")));
    }
    if let Some(a) = army {
        army_of(v, a)?;
        if order != OrderType::ORDER_TRAIN {
            return Err(crate::query::invalid("field `army`: only a unit goes to an army"));
        }
    }
    let army_at = army.and_then(|a| g.army_plot(a)).map(|q| g.map.xy(q));
    let own_turns = |c: CityId, num: i32| match order {
        OrderType::ORDER_TRAIN => g.city_production_turns_left_unit(c, UnitType::from_usize(data as usize), num),
        OrderType::ORDER_CONSTRUCT => g.city_production_turns_left_building(c, BuildingType::from_usize(data as usize), num),
        _ => g.city_production_turns_left_project(c, ProjectType::from_usize(data as usize), num),
    };
    let mut rows: Vec<(CityId, i32, Option<i32>, Json)> = Vec::new();
    let mut cities: Vec<CityId> = g.players[v.h].cities.clone();
    cities.sort();
    for c in cities {
        if !g.cities.contains(c) || g.city_check_empire_order(v.h, c, order, data).is_err() {
            continue;
        }
        let lead = g.city_lead_orders(c);
        let ahead_turns: i32 = (0..lead).map(|i| g.city_order_turns_left(c, i)).sum();
        let own = own_turns(c, lead as i32);
        let q = &g.cities[c].order_queue;
        let distance = army_at.map(|(ax, ay)| g.step_distance(g.cities[c].x, g.cities[c].y, ax, ay));
        let mut j = crate::jobj! {
            "city" => c,
            "name" => city_name(g, c),
            "x" => g.cities[c].x,
            "y" => g.cities[c].y,
            "turns" => ahead_turns + own,
            "own_turns" => own,
            "ahead" => lead,
            "ahead_turns" => ahead_turns,
            "displaces" => (lead < q.len()).then(|| order_json(g, c, lead)),
            "distance" => distance,
        };
        if order == OrderType::ORDER_CONSTRUCT {
            let b = BuildingType::from_usize(data as usize);
            if g.city_wonder_cost_percent(c, b) != 100 {
                j.set("cost_percent", g.city_wonder_cost_percent(c, b));
                j.set("tier", Json::idx(g.city_building_tier(c, b)));
            }
        }
        rows.push((c, ahead_turns + own, distance, j));
    }
    let best = rows.iter().enumerate().min_by_key(|(_, (c, turns, distance, _))| (distance.unwrap_or(0), *turns, *c)).map(|(i, _)| i);
    let out: Vec<Json> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (_, _, _, mut j))| {
            j.set("suggested", Some(i) == best);
            j
        })
        .collect();
    Ok(crate::jobj! {
        "order" => Json::idx(order),
        "type" => data,
        "army" => army,
        "dest" => crate::command::recruit_dest_json(army.map(RecruitDest::Army)),
        "cities" => Json::Arr(out),
    })
}

/// The empire orders bound for army `a` in the human's cities (the Armies screen): `city`, `name`,
/// `unit`, `index`, `turns`.
pub fn army_empire_orders(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for &c in &g.players[v.h].cities {
        if !g.cities.contains(c) {
            continue;
        }
        for (i, o) in g.cities[c].order_queue.iter().enumerate() {
            if o.empire && o.order_type == OrderType::ORDER_TRAIN && o.dest == Some(RecruitDest::Army(a)) {
                out.push(crate::jobj! { "city" => c, "name" => city_name(g, c), "unit" => o.data1, "index" => i, "turns" => g.city_order_turns_left(c, i) });
            }
        }
    }
    Json::Arr(out)
}
