//! This game's armies as the human sees them (docs/ARMIES.md): the view's `armies` (the human's
//! armies, their supply and the reinforcements on their way), the Armies screen (`armies` query)
//! and the recruitment choices of the city screen.

use ffh_core::army::{RecruitDest, Transit};
use ffh_core::ids::*;

use crate::command::recruit_dest_json;
use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A unit on its way: its type, where it comes from and goes, and how long until it arrives.
fn transit_json(v: &Viewer, t: &Transit) -> Json {
    let g = v.g;
    crate::jobj! {
        "unit" => Json::idx(t.unit_type),
        "from" => g.cities.contains(t.from_city).then(|| city_name(g, t.from_city)),
        "dest" => recruit_dest_json(Some(t.dest)),
        "to" => g.recruit_dest_name(t.dest),
        "turns_left" => if t.eta < 0 { Json::Null } else { Json::from((t.eta - t.progress).max(0)) },
        "stalled" => t.stalled,
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
        let transit: Vec<Json> = g.players[h].transit.iter().filter(|t| t.dest == RecruitDest::Army(a)).map(|t| transit_json(v, t)).collect();
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
    Json::Arr(g.players[v.h].transit.iter().filter(|t| matches!(t.dest, RecruitDest::Garrison(_))).map(|t| transit_json(v, t)).collect())
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
        let transit: Vec<Json> = g.players[h].transit.iter().filter(|t| t.dest == RecruitDest::Garrison(c)).map(|t| transit_json(v, t)).collect();
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
