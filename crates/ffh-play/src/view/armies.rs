//! This game's armies as the human sees them (docs/ARMIES.md): the view's `armies` (the human's
//! armies, their supply, requisition and the recruits marching to them), each marching unit's
//! `march`, the Armies screen (`armies` query) and the recruitment choices of the city screen.

use ffh_core::army::RecruitDest;
use ffh_core::ids::*;
use ffh_data::{Idx, OrderType, UnitType};

use crate::command::recruit_dest_json;
use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A marching recruit's march (the unit's `march`, and the Armies screen's `recruits`): where it
/// goes (`dest`, `to` its name), where it comes from, the turns it needs along the rules' path
/// (`turns_left`; `null`: no way there now), the turns it has made no progress (`stalled`), and
/// whether it was sent to a garrison instead of its first destination (`diverted`).
pub fn march_json(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let m = g.unit_march(u).expect("a marching unit");
    crate::jobj! {
        "dest" => recruit_dest_json(Some(m.dest)),
        "to" => g.recruit_dest_name(m.dest),
        "from" => g.cities.contains(m.from_city).then(|| city_name(g, m.from_city)),
        "turns_left" => g.unit_march_eta(u),
        "stalled" => m.stalled,
        "diverted" => m.diverted,
        "since" => m.left_turn,
    }
}

/// A recruit on the Armies screen: the unit (`id`, `unit` its type, where it is) and its march.
fn recruit_json(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let mut j = march_json(v, u);
    j.set("id", u);
    j.set("unit", Json::idx(g.units[u].unit_type));
    j.set("x", g.units[u].x);
    j.set("y", g.units[u].y);
    j
}

/// Army `a`'s requisition orders in production: the city, the unit, its place in the queue and
/// its production turns.
fn requisition_orders(v: &Viewer, a: ArmyId) -> Vec<(CityId, Json)> {
    let g = v.g;
    g.army_requisition_orders(a)
        .into_iter()
        .map(|(c, i)| {
            let o = &g.cities[c].order_queue[i];
            let unit = (o.order_type == OrderType::ORDER_TRAIN && o.data1 >= 0 && (o.data1 as usize) < g.d.units.len()).then(|| UnitType::from_usize(o.data1 as usize));
            let j = crate::jobj! {
                "city" => c,
                "name" => city_name(g, c),
                "unit" => unit.map(Json::idx),
                "index" => i,
                "turns" => unit.map(|t| g.city_production_turns_left_unit(c, t, i as i32)),
            };
            (c, j)
        })
        .collect()
}

/// Army `a`'s requisition: the cities it requisitions from (`cities`, in the order picked), its
/// orders in production (`orders`) and how many it lacks of its target size counting its units,
/// its marching recruits and the orders for it (`shortfall`; 0 or less: none). With `candidates`
/// (the Armies screen), every city of the human's that it may requisition from: whether picked,
/// governed, the unit it would train there (the rules' choice for the human), its production
/// turns, its march turns to the army over land (`null`: no way by land), its order in progress.
fn requisition_json(v: &Viewer, a: ArmyId, candidates: bool) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    let orders = requisition_orders(v, a);
    let mut j = crate::jobj! {
        "cities" => army.requisition.iter().copied().filter(|&c| g.cities.contains(c)).collect::<Vec<_>>(),
        "orders" => Json::Arr(orders.iter().map(|(_, o)| o.clone()).collect()),
        "shortfall" => g.army_shortfall(a),
    };
    if candidates {
        let open = g.army_check_requisition(v.h, a).is_ok();
        let list = g.players[v.h].cities.iter().copied().filter(|&c| g.cities.contains(c)).map(|c| {
            let preview = if open { g.army_requisition_preview(c, a) } else { None };
            crate::jobj! {
                "city" => c,
                "name" => city_name(g, c),
                "picked" => army.requisition.contains(&c),
                "governed" => g.city_is_governed(c),
                "unit" => preview.map(|p| Json::idx(p.0)),
                "production_turns" => preview.map(|p| p.2),
                "march_turns" => preview.and_then(|p| p.3),
                "order" => orders.iter().find(|(oc, _)| *oc == c).map(|(_, o)| o.clone()),
            }
        });
        j.set("candidates", Json::Arr(list.collect()));
        j.set("open", open);
    }
    j
}

/// One army of the human's: name, units, where, in supply, target size, the recruits marching to
/// it (`marching`: unit ids; each unit's `march` says where and how long), its requisition,
/// whether it marches to a garrison (disbanded), Upgrade all. The Armies screen's
/// (`screen`) adds the recruits in full (`recruits`) and the requisition's candidates.
fn army_json(v: &Viewer, a: ArmyId, screen: bool) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    let plot = g.army_plot(a);
    let units = g.army_units(a);
    let marching = g.army_marching(a);
    // Upgrade all: the units with an upgrade (given the gold) and its total price.
    let upgrades: Vec<(UnitId, ffh_data::UnitType)> = units.iter().filter_map(|&u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
    let mut j = crate::jobj! {
        "id" => a,
        "name" => army.name.as_str(),
        "group" => army.group,
        "units" => units,
        "x" => plot.map(|p| g.map.xy(p).0),
        "y" => plot.map(|p| g.map.xy(p).1),
        "in_supply" => g.army_in_supply(a),
        "target_size" => army.target_size,
        "marching" => marching.clone(),
        "requisition" => requisition_json(v, a, screen),
        "disband_to" => army.disband_to.filter(|&c| g.cities.contains(c)).map(|c| city_name(g, c)),
        "upgrades" => upgrades.len(),
        "upgrade_price" => upgrades.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum::<i32>(),
    };
    if screen {
        j.set("recruits", Json::Arr(marching.into_iter().map(|u| recruit_json(v, u)).collect()));
    }
    j
}

/// The human's armies (formation order), as the view has them.
pub fn armies(v: &Viewer) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return Json::Null;
    }
    Json::Arr(g.players[v.h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| army_json(v, a, false)).collect())
}

/// The recruits marching to garrisons (unit ids).
pub fn garrison_marching(v: &Viewer) -> Json {
    let g = v.g;
    Json::from(g.players[v.h].units.iter().copied().filter(|&u| g.unit_march(u).is_some_and(|m| matches!(m.dest, RecruitDest::Garrison(_)))).collect::<Vec<_>>())
}

/// `armies`: the Armies screen. The armies (as the view's, with their recruits in full and the
/// cities each may requisition from), each city's garrison with the recruits marching to it, the
/// supply range and how many turns a recruit waits without progress before it goes to the
/// nearest garrison instead.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return crate::jobj! { "on" => false };
    }
    let h = v.h;
    let garrisons = g.players[h].cities.iter().map(|&c| {
        crate::jobj! {
            "city" => c,
            "name" => city_name(g, c),
            "governed" => g.city_is_governed(c),
            "units" => g.city_garrison_units(c),
            "recruits" => Json::Arr(g.garrison_marching(c).into_iter().map(|u| recruit_json(v, u)).collect()),
        }
    });
    crate::jobj! {
        "on" => true,
        "armies" => Json::Arr(g.players[h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| army_json(v, a, true)).collect()),
        "garrisons" => Json::Arr(garrisons.collect()),
        "supply_range" => g.d.defines.army_supply_range,
        "stall_turns" => g.d.defines.army_transit_stall_turns,
    }
}

/// The city screen's recruitment: the city's default destination and the destinations it may
/// choose (its own and the other cities' garrisons, the armies), its garrison and the armies that
/// requisition from it; `null` without armies.
pub fn recruit(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return Json::Null;
    }
    let h = v.h;
    let mut options = vec![crate::jobj! { "dest" => recruit_dest_json(Some(RecruitDest::Garrison(c))), "name" => "this city's garrison" }];
    for &o in &g.players[h].cities {
        if o != c {
            options.push(crate::jobj! { "dest" => recruit_dest_json(Some(RecruitDest::Garrison(o))), "name" => format!("garrison of {}", city_name(g, o)) });
        }
    }
    let mut requisitioned_by = Vec::new();
    for &a in &g.players[h].armies {
        if !g.armies.contains(a) {
            continue;
        }
        if g.armies[a].requisition.contains(&c) {
            requisitioned_by.push(crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str(), "target_size" => g.armies[a].target_size, "shortfall" => g.army_shortfall(a) });
        }
        if g.recruit_dest_valid(h, RecruitDest::Army(a)) {
            let size = g.army_units(a).len();
            options.push(crate::jobj! {
                "dest" => recruit_dest_json(Some(RecruitDest::Army(a))),
                "name" => format!("{} ({} units{})", g.armies[a].name, size, if g.army_in_supply(a) { "" } else { ", out of supply" }),
            });
        }
    }
    let garrison: Vec<UnitId> = g.city_garrison_units(c);
    crate::jobj! {
        "default" => recruit_dest_json(g.cities[c].recruit_default),
        "options" => Json::Arr(options),
        "garrison" => garrison,
        "requisitioned_by" => Json::Arr(requisitioned_by),
    }
}
