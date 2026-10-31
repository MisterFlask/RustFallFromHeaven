//! This game's armies as the human sees them (docs/ARMIES.md): the view's `armies` (the human's
//! armies, their supply and the reinforcements on their way), the Armies screen (`armies` query)
//! and the recruitment choices of the city screen.

use ffh_core::army::RecruitDest;
use ffh_core::ids::*;

use crate::command::recruit_dest_json;
use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A marching recruit (shown as the old units on their way, `transit`): its type, where it comes
/// from and goes, and how long until it arrives (`null`: no way there now).
fn march_json(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let m = g.unit_march(u).expect("a marching unit");
    crate::jobj! {
        "id" => u,
        "unit" => Json::idx(g.units[u].unit_type),
        "from" => g.cities.contains(m.from_city).then(|| city_name(g, m.from_city)),
        "dest" => recruit_dest_json(Some(m.dest)),
        "to" => g.recruit_dest_name(m.dest),
        "x" => g.units[u].x,
        "y" => g.units[u].y,
        "turns_left" => g.unit_march_eta(u),
        "stalled" => m.stalled,
    }
}

/// The human's armies (formation order): name, size, where, in supply, target size, the units on
/// their way, whether it marches to a garrison (disbanded).
pub fn armies(v: &Viewer) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return Json::Null;
    }
    let h = v.h;
    let out = g.players[h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| {
        let army = &g.armies[a];
        let plot = g.army_plot(a);
        let units = g.army_units(a);
        let transit: Vec<Json> = g.army_marching(a).into_iter().map(|u| march_json(v, u)).collect();
        // Upgrade all: the units with an upgrade (given the gold) and its total price.
        let upgrades: Vec<(UnitId, ffh_data::UnitType)> = units.iter().filter_map(|&u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
        crate::jobj! {
            "id" => a,
            "name" => army.name.as_str(),
            "group" => army.group,
            "units" => units,
            "x" => plot.map(|p| g.map.xy(p).0),
            "y" => plot.map(|p| g.map.xy(p).1),
            "in_supply" => g.army_in_supply(a),
            "target_size" => army.target_size,
            "transit" => Json::Arr(transit),
            "disband_to" => army.disband_to.filter(|&c| g.cities.contains(c)).map(|c| city_name(g, c)),
            "upgrades" => upgrades.len(),
            "upgrade_price" => upgrades.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum::<i32>(),
        }
    });
    Json::Arr(out.collect())
}

/// The units on their way to garrisons.
pub fn garrison_transit(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    Json::Arr(g.players[h].units.iter().copied().filter(|&u| g.unit_march(u).is_some_and(|m| matches!(m.dest, RecruitDest::Garrison(_)))).map(|u| march_json(v, u)).collect())
}

/// `armies`: the Armies screen. The armies (as the view's), each city's garrison with the units
/// on their way to it, the supply range and how many turns a reinforcement waits out of supply
/// before it goes to a garrison instead.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return crate::jobj! { "on" => false };
    }
    let h = v.h;
    let garrisons = g.players[h].cities.iter().map(|&c| {
        let transit: Vec<Json> = g.garrison_marching(c).into_iter().map(|u| march_json(v, u)).collect();
        crate::jobj! {
            "city" => c,
            "name" => city_name(g, c),
            "units" => g.city_garrison_units(c),
            "transit" => Json::Arr(transit),
        }
    });
    crate::jobj! {
        "on" => true,
        "armies" => armies(v),
        "garrisons" => Json::Arr(garrisons.collect()),
        "supply_range" => g.d.defines.army_supply_range,
        "stall_turns" => g.d.defines.army_transit_stall_turns,
    }
}

/// The city screen's recruitment: the city's default destination and the destinations it may
/// choose (its own and the other cities' garrisons, the armies); `null` without armies.
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
    for &a in &g.players[h].armies {
        if g.armies.contains(a) && g.recruit_dest_valid(h, RecruitDest::Army(a)) {
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
    }
}
