//! This game's armies as the human sees them (docs/ARMIES.md): the view's `armies` (the human's
//! armies and fleets, their supply), the Armies screen (`armies` query: its armies, its fleets,
//! docs/NAVAL.md section 7), the city screen's garrison and the cities' waypoints.

use ffh_core::army::{ArmyDomain, EscortTarget, Station, SupplyLine, SupplySource, SupplyState, Waypoint};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::Idx;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// The name of a supply line's source: the city's, or "a Trireme" for a ship.
fn source_name(g: &Game, s: SupplySource) -> String {
    match s {
        SupplySource::City(c) => city_name(g, c),
        SupplySource::Ship(u) => format!("a {}", crate::names::display(g.d, g.units[u].unit_type)),
        // At sea, in range of a port (docs/NAVAL.md, 3.3).
        SupplySource::Sea(c) => city_name(g, c),
    }
}

/// A supply line (docs/SUPPLY.md) as the human's view has it: `path` (`[x, y]` from the army's
/// plot to the source's), `cost`, `budget` (`SUPPLY_LINE_BUDGET`), `source` (its name) and
/// `source_kind` (`city`, `ship`, or `sea`: an army in boats in range of the port `source`).
pub fn supply_line_json(g: &Game, line: &SupplyLine) -> Json {
    crate::jobj! {
        "path" => Json::Arr(line.path.iter().map(|&q| { let (x, y) = g.map.xy(q); crate::jarr![x, y] }).collect()),
        "cost" => line.cost,
        "budget" => g.d.defines.supply_line_budget,
        "source" => source_name(g, line.source),
        "source_kind" => match line.source { SupplySource::City(_) => "city", SupplySource::Ship(_) => "ship", SupplySource::Sea(_) => "sea" },
    }
}

/// Why army `a` may not leave its city (docs/SUPPLY.md, 3.2): held in an invested city of its
/// team, it moves only to attack until the siege lifts. `None`: free to move.
pub fn siege_hold_text(g: &Game, a: ArmyId) -> Option<String> {
    let c = g.army_units(a).iter().find_map(|&u| g.unit_siege_hold(u))?;
    Some(format!("Besieged in {}: moves only to attack until the siege lifts.", city_name(g, c)))
}

/// Army `a`'s supply (docs/SUPPLY.md, section 5), the human's own: `state` (`supplied`, `hunger`
/// or `starvation`, from its counter), `turn` and `of` (Hunger's turn of `SUPPLY_HUNGER_TURNS`),
/// `turns_to_starvation` (Hunger: the turn starts with the line cut before Starvation; else
/// null), `line` (the line as it is traced now, [`supply_line_json`]; null: cut), `raiding` (it
/// holds a Raid order: supplied whatever its line, docs/RAIDS.md section 3), `home` (it stands in
/// its owner's territory: supplied with no line, `line` null) and `text`, the
/// state in the bare words of the Armies screen and the unit panel.
pub fn supply_json(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    let d = g.d;
    let state = g.army_supply_state(a);
    // Within its owner's borders an army needs no line (docs/SUPPLY.md, section 2): none is drawn.
    let home = g.army_supply_home(a);
    let line = if home { None } else { g.army_supply_line(a) };
    let budget = d.defines.supply_line_budget;
    let supplied = |l: &SupplyLine| match l.source {
        // At sea, in range of a port (docs/NAVAL.md, 3.3 and 10).
        SupplySource::Sea(c) => format!("at sea, in range of {}.", crate::view::sea::port_name(v, c)),
        _ => format!("{} of {budget}, from {}.", l.cost, source_name(g, l.source)),
    };
    // No line: why a supply ship of the human's on or by the army's plot is no source
    // (docs/NAVAL.md, 4.6 and 10: `the transport is in Sirens' Water, held by Lanun.`).
    let sea_why = if line.is_none() && !home { g.army_plot(a).and_then(|q| crate::view::seas::transport_refusal(v, q)) } else { None };
    // Raids (docs/RAIDS.md, section 3): an army holding the Raid order lives off the land.
    let raiding = g.army_raid_city(a).is_some();
    let mut text = match state {
        SupplyState::Supplied if raiding => "Supplied: raiding, no supply line needed.".to_string(),
        SupplyState::Supplied if home => "Supplied: in your territory, no supply line needed.".to_string(),
        SupplyState::Supplied => match (&line, &sea_why) {
            (Some(l), _) => format!("Supplied: {}", supplied(l)),
            (None, Some(why)) => format!("No supply line: {why} Hunger from your next turn."),
            (None, None) => "No supply line: Hunger from your next turn.".to_string(),
        },
        SupplyState::Hunger { turn, of } => format!("Hunger: turn {turn} of {of}. No healing."),
        SupplyState::Starvation => format!(
            "Starvation: -{}% HP a turn (to {}%), {:+}% damage dealt.",
            d.defines.starvation_damage_percent, d.defines.starvation_floor_percent, d.defines.starvation_damage_dealt_percent
        ),
    };
    if state != SupplyState::Supplied {
        if home {
            text.push_str(" In your territory: supplied from your next turn.");
        } else if let Some(l) = &line {
            text.push_str(&format!(" Supply line again: {} Supplied from your next turn.", supplied(l)));
        }
    }
    if let (Some(why), true) = (&sea_why, state != SupplyState::Supplied) {
        text.push_str(&format!(" No supply line: {why}"));
    }
    let (name, turn, of) = match state {
        SupplyState::Supplied => ("supplied", Json::Null, Json::Null),
        SupplyState::Hunger { turn, of } => ("hunger", Json::from(turn), Json::from(of)),
        SupplyState::Starvation => ("starvation", Json::Null, Json::Null),
    };
    crate::jobj! {
        "state" => name,
        "turn" => turn,
        "of" => of,
        "turns_to_starvation" => state.turns_to_starvation(),
        "line" => line.as_ref().map(|l| supply_line_json(g, l)),
        "raiding" => raiding,
        "home" => home,
        "text" => text,
    }
}

/// The supply lines of the other players' armies the human sees (docs/SUPPLY.md, section 5 and
/// "Second pass", A), each `{owner, x, y, segments}`: the army's plot and its line as the human can
/// work it out (`Game::plot_supply_line_known` from the army's plot, the human's team as viewer):
/// it runs over plots the human has revealed only and bends around the enemy units the human sees,
/// not the hidden ones, so it can differ from the army's true line. `segments` holds that one path
/// (`[[[x, y], ...]]`, from the army toward its source; the client draws runs). An army counts as
/// seen when its plot is visible to the human and one of its units is visible and shows its true
/// owner (hidden nationality hides the army); an army with no known line has no entry. Nothing
/// else of the army's supply is told (its counter, the cost, the source).
pub fn foreign_supply_lines(v: &Viewer) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for p in g.player_ids() {
        if p == v.h || !g.players[p].alive || g.player_is_barbarian(p) {
            continue;
        }
        for &a in &g.players[p].armies {
            // (A fleet has no supply line: docs/NAVAL.md, 7.1.)
            if !g.armies.contains(a) || g.army_is_fleet(a) {
                continue;
            }
            let Some(plot) = g.army_plot(a) else { continue };
            if !v.plot_visible(plot) || !g.army_units(a).iter().any(|&u| v.unit_visible(u) && v.unit_owner(u) == p) {
                continue;
            }
            // Within its owner's borders an army needs no line: none to draw.
            if g.plot_supply_home(p, plot) {
                continue;
            }
            let Some(line) = g.plot_supply_line_known(p, plot, v.team) else { continue };
            let path: Vec<Json> = line.path.iter().map(|&q| { let (x, y) = g.map.xy(q); crate::jarr![x, y] }).collect();
            let (x, y) = g.map.xy(plot);
            out.push(crate::jobj! { "owner" => p, "x" => x, "y" => y, "segments" => crate::jarr![Json::Arr(path)] });
        }
    }
    Json::Arr(out)
}

/// A fleet's standing order (docs/NAVAL.md, 7.2) for the views: `kind` (`none`, `patrol`,
/// `escort`), `sea` and `army` / `group` (its target), and `text`, the order in bare words.
pub fn station_view(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    match g.armies[a].station {
        Station::None => crate::jobj! { "kind" => "none", "text" => "Standing order: none." },
        Station::Patrol(s) => {
            let name = g.map.seas.get(s.idx()).map_or("a sea", |sea| sea.name.as_str());
            crate::jobj! { "kind" => "patrol", "sea" => s.idx(), "text" => format!("Standing order: Patrol of {name}.") }
        }
        Station::Escort(t) => {
            let (army, group) = match t {
                EscortTarget::Army(b) => (Some(b), None),
                EscortTarget::Group(gr) => (None, Some(gr)),
                // A caravan (docs/TRADE.md 4.4): its own line, with why it waits.
                EscortTarget::Caravan(..) => return crate::view::trade::escort_station(v, a),
            };
            crate::jobj! { "kind" => "escort", "army" => army, "group" => group, "text" => format!("Standing order: Escort of {}.", g.escort_target_name(t)) }
        }
        Station::Raid(c) => {
            let name = g.cities.get(c).map_or_else(|| "a city".to_string(), |city| g.d.text.name(&city.name).to_owned());
            crate::jobj! { "kind" => "raid", "city" => c, "text" => format!("Raiding {name}.") }
        }
    }
}

/// A fleet's wear (docs/NAVAL.md, 3.4): its ships' highest, in percent of strength lost.
fn fleet_wear(v: &Viewer, units: &[UnitId]) -> i32 {
    units.iter().map(|&u| v.g.unit_sea_wear_percent(u)).max().unwrap_or(0)
}

/// One army or fleet of the human's: name, units, where, in supply, whether it marches to a
/// garrison (disbanded), Upgrade all; `domain` (`land`, `sea`). A fleet has no supply (`supply`
/// null); it has its standing order (`station`, [`station_view`]) and its `wear`. The Armies
/// screen's (`screen`) adds the army's squads.
fn army_json(v: &Viewer, a: ArmyId, screen: bool) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    if army.domain == ArmyDomain::Sea {
        return fleet_json(v, a);
    }
    let plot = g.army_plot(a);
    let units = g.army_units(a);
    // Upgrade all: the units with an upgrade (given the gold) and its total price.
    let upgrades: Vec<(UnitId, ffh_data::UnitType)> = units.iter().filter_map(|&u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
    let mut j = crate::jobj! {
        "id" => a,
        "name" => army.name.as_str(),
        "group" => army.group,
        "units" => units,
        // Held in an invested city (docs/SUPPLY.md, 3.2): why it moves only to attack.
        "held" => siege_hold_text(g, a),
        "x" => plot.map(|p| g.map.xy(p).0),
        "y" => plot.map(|p| g.map.xy(p).1),
        "in_supply" => g.army_in_supply(a),
        "supply" => supply_json(v, a),
        "disband_to" => army.disband_to.filter(|&c| g.cities.contains(c)).map(|c| city_name(g, c)),
        "upgrades" => upgrades.len(),
        "upgrade_price" => upgrades.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum::<i32>(),
    };
    j.set("domain", "land");
    army_waypoint_fields(v, a, &mut j);
    // Its Raid order (docs/RAIDS.md, section 8): `kind` none or raid, its `text`.
    // Or its Escort of a caravan (docs/TRADE.md 4.4): `kind` escort, `route`, `slot`, `text`, `wait`.
    let escort = crate::view::trade::escort_station(v, a);
    j.set("station", if escort.is_null() { crate::view::raids::station(v, a) } else { escort });
    j.set("escort_caravans", crate::view::trade::escort_choices(v, a));
    if screen {
        // Squads (docs/ENGAGEMENTS.md, "Squads"): the army's squads, for the squad board.
        j.set("squads", plot.map_or(Json::Arr(Vec::new()), |p| crate::view::squads::plot_squads(v, p, Some(ffh_core::army::SquadBody::Army(a)))));
    }
    j
}

/// One fleet of the human's (docs/NAVAL.md, section 7): as [`army_json`], with its standing order
/// and wear in place of supply.
fn fleet_json(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    let plot = g.army_plot(a);
    let units = g.army_units(a);
    let upgrades: Vec<(UnitId, ffh_data::UnitType)> = units.iter().filter_map(|&u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
    let wear = fleet_wear(v, &units);
    let mut j = crate::jobj! {
        "id" => a,
        "name" => army.name.as_str(),
        "domain" => "sea",
        "group" => army.group,
        "units" => units,
        "x" => plot.map(|p| g.map.xy(p).0),
        "y" => plot.map(|p| g.map.xy(p).1),
        "in_supply" => true,
        "supply" => Json::Null,
        "station" => station_view(v, a),
        "escort_caravans" => crate::view::trade::escort_choices(v, a),
        "wear" => wear,
        "wear_text" => if wear > 0 { format!("Wear: -{wear}% strength. Enter a port to clear.") } else { "Wear: none.".to_string() },
        "sea" => plot.and_then(|q| g.map.plots[q].sea).map(|s| g.map.seas[s.idx()].name.clone()),
        "disband_to" => army.disband_to.filter(|&c| g.cities.contains(c)).map(|c| city_name(g, c)),
        "upgrades" => upgrades.len(),
        "upgrade_price" => upgrades.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum::<i32>(),
    };
    army_waypoint_fields(v, a, &mut j);
    j
}

/// Waypoints (docs/ARMIES.md): army `a`'s `waypoint_cities` (the cities whose waypoint it is:
/// `{city, name}`) and `waypoint_units` (the units on their way to it: ids).
fn army_waypoint_fields(v: &Viewer, a: ArmyId, j: &mut Json) {
    let g = v.g;
    let cities: Vec<Json> = g.army_waypoint_cities(a).into_iter().map(|c| crate::jobj! { "city" => c, "name" => city_name(g, c) }).collect();
    j.set("waypoint_cities", Json::Arr(cities));
    j.set("waypoint_units", Json::from(g.army_waypoint_units(a)));
}

/// A waypoint for the views: `kind` (`none`, `army`, `plot`), `army` (its id) or `x`, `y`, and
/// `name` ("the 2nd Army", "plot 14, 9"; null for none).
pub fn waypoint_json(g: &Game, wp: Option<Waypoint>) -> Json {
    match wp {
        None => crate::jobj! { "kind" => "none", "name" => Json::Null },
        Some(Waypoint::Army(a)) => crate::jobj! { "kind" => "army", "army" => a, "name" => g.waypoint_name(Waypoint::Army(a)) },
        Some(Waypoint::Plot(q)) => {
            let (x, y) = g.map.xy(q);
            crate::jobj! { "kind" => "plot", "x" => x, "y" => y, "name" => g.waypoint_name(Waypoint::Plot(q)) }
        }
    }
}

/// The city screen's waypoint of city `c` (null for another's city): the waypoint
/// ([`waypoint_json`]), `governed` (only the units of empire orders follow it) and `choices`, the
/// armies and fleets it may name now (`{army, name, fleet}`; a fleet for a coastal city only).
pub fn city_waypoint(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if g.cities[c].owner != v.h {
        return Json::Null;
    }
    let choices: Vec<Json> = g.players[v.h]
        .armies
        .iter()
        .copied()
        .filter(|&a| g.armies.contains(a) && g.city_check_waypoint(v.h, c, Some(Waypoint::Army(a))).is_ok())
        .map(|a| crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str(), "fleet" => g.army_is_fleet(a) })
        .collect();
    let mut j = waypoint_json(g, g.cities[c].waypoint);
    j.set("governed", g.city_is_governed(c));
    j.set("choices", Json::Arr(choices));
    j
}

/// The human's armies and fleets (formation order; each with its `domain`), as the view has them.
pub fn armies(v: &Viewer) -> Json {
    let g = v.g;
    Json::Arr(g.players[v.h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| army_json(v, a, false)).collect())
}

/// `armies`: the Armies screen. The armies (as the view's, with their squads), each city's
/// garrison and the supply budget. Its Fleets section (docs/NAVAL.md, section 7): `fleets` (as the
/// armies, with their standing orders and wear), `seas` (the seas a fleet may be sent to patrol:
/// `{id, name}`, those in the human's range) and `escort_targets` (the armies and fleets a fleet
/// may escort: `{army, name, fleet}`).
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let garrisons = g.players[h].cities.iter().map(|&c| {
        crate::jobj! {
            "city" => c,
            "name" => city_name(g, c),
            "governed" => g.city_is_governed(c),
            "units" => g.city_garrison_units(c),
            // Waypoints: the city's waypoint, and whether a fleet may be it (a coastal city).
            "waypoint" => waypoint_json(g, g.cities[c].waypoint),
            "port" => g.city_is_port(c),
        }
    });
    let live: Vec<ArmyId> = g.players[h].armies.iter().copied().filter(|&a| g.armies.contains(a)).collect();
    let seas: Vec<Json> = g.fleet_patrol_seas(h).into_iter().map(|s| crate::jobj! { "id" => s.idx(), "name" => g.map.seas[s.idx()].name.as_str() }).collect();
    let targets: Vec<Json> = live.iter().map(|&a| crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str(), "fleet" => g.army_is_fleet(a) }).collect();
    crate::jobj! {
        "armies" => Json::Arr(live.iter().filter(|&&a| !g.army_is_fleet(a)).map(|&a| army_json(v, a, true)).collect()),
        "fleets" => Json::Arr(live.iter().filter(|&&a| g.army_is_fleet(a)).map(|&a| army_json(v, a, true)).collect()),
        "seas" => Json::Arr(seas),
        "escort_targets" => Json::Arr(targets),
        "garrisons" => Json::Arr(garrisons.collect()),
        "recovering" => recovering_json(v),
        "fallen" => fallen_json(v),
        "supply_budget" => g.d.defines.supply_line_budget,
    }
}

/// The human's heroes that died and will return (ffh_core::unit::fallen), the soonest first: the
/// name, the unit type, the turns left, the Wounds it will have, where it returns (`null`: no
/// city; it waits for one).
pub fn recovering_json(v: &Viewer) -> Json {
    let g = v.g;
    let turn = g.game_turn();
    let home = g.player_get_capital_city(v.h).or_else(|| g.players[v.h].cities.first().copied());
    let mut list: Vec<&ffh_core::unit::fallen::FallenUnit> = g.players[v.h].recovering.iter().collect();
    list.sort_by_key(|r| (r.returns, r.id));
    Json::Arr(
        list.into_iter()
            .map(|r| {
                crate::jobj! {
                    "name" => { let n = r.display_name(g.d); crate::names::text(g.d, &n).to_string() },
                    "type" => Json::idx(r.unit_type),
                    "level" => r.level,
                    "turns" => (r.returns - turn).max(0),
                    "wounds" => r.wounds + 1,
                    "city" => home.map(|c| city_name(g, c)),
                }
            })
            .collect(),
    )
}

/// The human's other dead within Resurrection's reach (ffh_core::unit::fallen), the latest first:
/// the name, the unit type, its level, the turn it died and the turns it stays in reach.
pub fn fallen_json(v: &Viewer) -> Json {
    let g = v.g;
    let turn = g.game_turn();
    let reach = ffh_core::crusade::speed_turns(g, ffh_core::unit::fallen::RESURRECTION_TURNS);
    Json::Arr(
        g.players[v.h]
            .fallen
            .iter()
            .rev()
            .map(|r| {
                crate::jobj! {
                    "name" => { let n = r.display_name(g.d); crate::names::text(g.d, &n).to_string() },
                    "type" => Json::idx(r.unit_type),
                    "level" => r.level,
                    "died" => r.died,
                    "turns" => (r.died + reach - turn).max(0),
                }
            })
            .collect(),
    )
}

/// The city screen's garrison: the units of city `c`'s garrison (unit ids).
pub fn garrison(v: &Viewer, c: CityId) -> Json {
    Json::from(v.g.city_garrison_units(c))
}
