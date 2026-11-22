//! Trade (docs/TRADE.md, section 9), as the human sees it: its routes (the Trade screen's rows),
//! the routes of others into its cities, its limits, the map's lines (the Trade layer: trade.js),
//! each city's routes with the pay to that city (the city panel), each visible caravan's route and
//! plunder (the unit panel, the plot tooltip, the attack preview), the Establish action and the
//! Establish dialog's offers (the `route_offers` query, computed on demand: one path search per
//! destination). Text is bare (CLAUDE.md, "Rules text and tooltips").
//!
//! What it never shows: a path over plots the human has not revealed (every path is clipped to
//! revealed plots, its own routes' too), a city it has not revealed ("a city"), the pay of a route
//! that is neither its own nor into one of its cities.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::trade::{round_x100, CargoClass, CargoState, Leg, Route, RouteRefusal, CARAVAN_SLOTS};
use ffh_data::BonusType;

use ffh_core::army::{EscortTarget, Station};

use crate::command::Command;
use crate::json::Json;
use crate::query::QueryErr;
use crate::view::Viewer;

/// Hundredths of commerce as people read them: one decimal (`2.8`), rounded half up.
pub fn pay_text(x100: i32) -> String {
    let tenths = if x100 >= 0 { (x100 + 5) / 10 } else { -((-x100 + 5) / 10) };
    let sign = if tenths < 0 { "-" } else { "" };
    format!("{sign}{}.{}", tenths.abs() / 10, tenths.abs() % 10)
}

/// A city's name as the human may name it ("a city" when not revealed).
fn name(v: &Viewer, c: CityId) -> String {
    if v.g.cities.contains(c) && v.city_revealed(c) {
        crate::view::city_name(v.g, c)
    } else {
        "a city".to_string()
    }
}

/// A player as the human names it: "you", "the barbarians", or its civilization.
fn player_name(v: &Viewer, p: PlayerId) -> String {
    let g = v.g;
    if p == v.h {
        "you".to_string()
    } else if g.player_is_barbarian(p) {
        "the barbarians".to_string()
    } else {
        g.player_get_civilization_short_description(p)
    }
}

fn civ(g: &Game, p: PlayerId) -> String {
    g.player_get_civilization_short_description(p)
}

/// The plunder a killer of caravan `slot` of route `r` gains (docs/TRADE.md, 4.3): its pay as
/// last computed x `TRADE_PLUNDER_TURNS`, rounded as the rules round it.
pub fn plunder(g: &Game, r: RouteId, slot: usize) -> i32 {
    round_x100(g.d.defines.trade_plunder_turns * g.routes[r].slots[slot].pay_x100).max(0)
}

/// `Plunder: 14 gold.`
pub fn plunder_line(gold: i32) -> String {
    format!("Plunder: {gold} gold.")
}

/// The route and slot of caravan `u`, if it carries one.
fn caravan_slot(g: &Game, u: UnitId) -> Option<(RouteId, usize)> {
    let r = g.units.get(u)?.route.filter(|&r| g.routes.contains(r))?;
    let slot = g.routes[r].slots.iter().position(|s| s.unit == Some(u))?;
    Some((r, slot))
}

/// The city a caravan bound `leg` on route `route` goes to.
fn bound_city(route: &Route, leg: Leg) -> CityId {
    match leg {
        Leg::Out => route.destination,
        Leg::Back => route.origin,
    }
}

/// The `caravan` field of a visible unit in a route (null otherwise): `{route, mine, text, bound,
/// plunder, plunder_text}` (`Caravan: Ithralia to Vallus.`, `Bound for Vallus.`, `Plunder: 14
/// gold.`), the city names as the human may name them.
pub fn unit_caravan(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    if !v.unit_visible(u) {
        return Json::Null;
    }
    let Some((r, slot)) = caravan_slot(g, u) else { return Json::Null };
    let route = &g.routes[r];
    let gold = plunder(g, r, slot);
    let to = bound_city(route, route.slots[slot].bound);
    crate::jobj! {
        "route" => r,
        "mine" => route.owner == v.h,
        "text" => format!("Caravan: {} to {}.", name(v, route.origin), name(v, route.destination)),
        "bound" => format!("Bound for {}.", name(v, to)),
        // The rules give the barbarians nothing; the human is never the barbarians.
        "plunder" => gold,
        "plunder_text" => plunder_line(gold),
        // Its cargo (docs/TRADE.md, 5): a raider chooses by it. Shown with the caravan only.
        "cargo_text" => route.slots[slot].cargo.map(|b| format!("Cargo: {}.", bonus_name(g, b))),
    }
}

/// A resource's name.
fn bonus_name(g: &Game, b: BonusType) -> String {
    g.d.text.name(&g.d.bonuses[b].base.description).to_string()
}

/// Slot `slot` of route `r`'s cargo as the rules read it (`route_slot_cargo`), except that a
/// caravan waiting in the origin of a closed route (not killed) reads `Route closed.`: the rules
/// name any absent caravan `Caravan lost.` first.
fn slot_cargo(g: &Game, r: RouteId, slot: usize) -> ffh_core::trade::CargoView {
    let mut c = g.route_slot_cargo(r, slot);
    let s = &g.routes[r].slots[slot];
    if c.state == CargoState::CaravanLost && !g.routes[r].is_open() && s.reform_turns == 0 {
        c.state = CargoState::RouteClosed;
    }
    c
}

/// Caravan `slot` of route `r`'s cargo line (docs/TRADE.md, 9): `Cargo: Gems. Delivered. Premium
/// +150%.` / `Cargo: Gems. Caravan lost.` / `Cargo: none.`
pub fn cargo_line(g: &Game, r: RouteId, slot: usize) -> String {
    let c = slot_cargo(g, r, slot);
    match c.cargo {
        None => "Cargo: none.".to_string(),
        Some(b) if c.state == CargoState::Delivering && c.premium_percent > 0 => format!("Cargo: {}. {} Premium +{}%.", bonus_name(g, b), c.state.text(), c.premium_percent),
        Some(b) => format!("Cargo: {}. {}", bonus_name(g, b), c.state.text()),
    }
}

/// The escorts of caravan `slot` of route `r` (docs/TRADE.md, 4.4): `[{army, name}]`.
fn escorts_json(g: &Game, r: RouteId, slot: usize) -> Json {
    Json::Arr(g.route_slot_escorts(r, slot as u8).into_iter().map(|a| crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str() }).collect())
}

/// The unit panel's lines for caravan `u` of the human's (null otherwise): the route, where it is
/// bound, its state, the plunder, and why it takes no order.
pub fn unit_panel(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    if !g.unit_is_caravan(u) {
        return Json::Null;
    }
    let Some((r, slot)) = caravan_slot(g, u) else {
        return crate::jobj! { "route" => Json::Null, "lines" => vec!["No route. Establish one from a city of yours.".to_string()] };
    };
    let route = &g.routes[r];
    let mut lines = vec![format!("Route: {} to {}.", name(v, route.origin), name(v, route.destination))];
    lines.push(caravan_state(v, route, slot));
    lines.push(cargo_line(g, r, slot));
    lines.push(plunder_line(plunder(g, r, slot)));
    for a in g.route_slot_escorts(r, slot as u8) {
        lines.push(format!("Escort: {}.", g.armies[a].name));
    }
    lines.push("A caravan in a route takes no orders.".to_string());
    crate::jobj! { "route" => r, "lines" => lines }
}

/// What caravan `slot` of `route` does now (docs/TRADE.md, section 9): `On the road: plot 7 of 17,
/// bound for Vallus.` / `Lost on turn 112 to the barbarians. Re-forms in 6 turns.` / `Waits in
/// Ithralia: the route is closed.` / `Forms in Ithralia when the first caravan arrives.`
fn caravan_state(v: &Viewer, route: &Route, slot: usize) -> String {
    let g = v.g;
    let s = &route.slots[slot];
    let origin = name(v, route.origin);
    match s.unit.filter(|&u| g.units.contains(u)) {
        Some(u) => {
            let to = name(v, bound_city(route, s.bound));
            let at = g.unit_plot(u);
            let len = route.length.max(0);
            match route.path.iter().position(|&q| q == at) {
                Some(0) => format!("In {origin}, bound for {to}."),
                Some(i) if i as i32 >= len => format!("In {}, bound for {to}.", name(v, route.destination)),
                Some(i) => {
                    let step = if s.bound == Leg::Out { i as i32 } else { len - i as i32 };
                    format!("On the road: plot {step} of {len}, bound for {to}.")
                }
                None => format!("Off the path, bound for {to}."),
            }
        }
        None if s.lost_turn >= 0 && s.reform_turns > 0 => {
            let by = s.lost_to.map_or_else(String::new, |k| format!(" to {}", player_name(v, k)));
            let n = s.reform_turns;
            format!("Lost on turn {}{by}. Re-forms in {n} turn{}.", s.lost_turn, if n == 1 { "" } else { "s" })
        }
        None if !route.is_open() => format!("Waits in {origin}: the route is closed."),
        None if !route.arrived => format!("Forms in {origin} when the first caravan arrives."),
        None => format!("Forms in {origin} next turn."),
    }
}

/// The lines of a route's pay terms (6.1): the effective length, the two populations, the base,
/// then each percent term that applies with its value.
pub fn pay_terms(g: &Game, owner: PlayerId, origin: CityId, dest: CityId, length: i32, water: bool, growth: i32) -> Vec<String> {
    let d = &g.d.defines;
    let mut out = Vec::new();
    if !g.cities.contains(origin) || !g.cities.contains(dest) {
        return out;
    }
    let eff = g.route_effective_length(length);
    out.push(if eff == length { format!("Length: {length} plots.") } else { format!("Length: {length} plots, counted {eff}.") });
    let (po, pd) = (g.cities[origin].population, g.cities[dest].population);
    out.push(format!("Population: {po} + {pd}."));
    let base = eff as i64 * (po + pd) as i64 * d.trade_pay_percent as i64 / 100;
    // `TRADE_PAY_PERCENT` is in hundredths of a percent of a commerce: 500 is 0.05 a plot and a
    // citizen.
    let p = d.trade_pay_percent.max(0);
    out.push(format!("Base: {eff} x {} x {}.{:02} = {}.", po + pd, p / 10000, (p % 10000) / 100, pay_text(base as i32)));
    let foreign = g.cities[dest].owner != owner;
    let pct = |n: i32| format!("{}{n}%", if n >= 0 { "+" } else { "" });
    if foreign && d.trade_foreign_percent != 0 {
        out.push(format!("Foreign: {}.", pct(d.trade_foreign_percent)));
    }
    if water && d.overseas_trade_modifier != 0 {
        out.push(format!("Over water: {}.", pct(d.overseas_trade_modifier)));
    }
    let m = g.cities[origin].trade_route_modifier;
    if m != 0 {
        out.push(format!("Buildings: {}.", pct(m)));
    }
    let f = g.cities[origin].foreign_trade_route_modifier;
    if foreign && f != 0 {
        out.push(format!("Foreign trade buildings: {}.", pct(f)));
    }
    if growth != 0 {
        out.push(format!("Growth: {}.", pct(growth)));
    }
    out
}

/// The plots of `path` the human has revealed, split into runs where it has not (as
/// `outposts::lanes` clips another's lane), each as `[[x, y], ...]` of 2 plots or more.
fn segments(v: &Viewer, path: &[PlotIdx]) -> Json {
    let g = v.g;
    let mut out: Vec<Json> = Vec::new();
    let mut run: Vec<Json> = Vec::new();
    let flush = |run: &mut Vec<Json>, out: &mut Vec<Json>| {
        if run.len() >= 2 {
            out.push(Json::Arr(std::mem::take(run)));
        } else {
            run.clear();
        }
    };
    for &q in path {
        if v.plot_revealed(q) {
            let (x, y) = g.map.xy(q);
            run.push(crate::jarr![x, y]);
        } else {
            flush(&mut run, &mut out);
        }
    }
    flush(&mut run, &mut out);
    Json::Arr(out)
}

/// The named seas a path crosses that the human has revealed a plot of on it, in the order it
/// enters them: `[{id, name, state, text}]` (`text`: `Bay of Lorda: contested.`, the state as the
/// Sea layer names it).
fn seas(v: &Viewer, path: &[PlotIdx]) -> Vec<Json> {
    let g = v.g;
    let revealed: Vec<PlotIdx> = path.iter().copied().filter(|&q| v.plot_revealed(q)).collect();
    g.route_path_seas(&revealed)
        .into_iter()
        .map(|s| {
            let sm = crate::view::seas::summary(v, s);
            let sea = g.map.seas[s.0 as usize].name.clone();
            let state = lower_first(&sm.control);
            crate::jobj! { "id" => s.0 as i64, "name" => sea.as_str(), "state" => sm.state, "text" => format!("{sea}: {state}.") }
        })
        .collect()
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A city as a route's row names it: `{id, name, x, y, owner}` (null coordinates and owner when
/// not revealed).
fn city_ref(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if g.cities.contains(c) && v.city_revealed(c) {
        crate::jobj! { "id" => c, "name" => name(v, c), "x" => g.cities[c].x, "y" => g.cities[c].y, "owner" => g.cities[c].owner }
    } else {
        crate::jobj! { "id" => Json::Null, "name" => "a city", "x" => Json::Null, "y" => Json::Null, "owner" => Json::Null }
    }
}

/// A route's title: `Ithralia to Vallus (Lanun).` (the destination's owner when foreign).
fn title(v: &Viewer, route: &Route) -> String {
    let g = v.g;
    let foreign = g.cities.contains(route.destination) && g.cities[route.destination].owner != route.owner && v.city_revealed(route.destination);
    let owner = if foreign { format!(" ({})", civ(g, g.cities[route.destination].owner)) } else { String::new() };
    format!("{} to {}{owner}.", name(v, route.origin), name(v, route.destination))
}

/// One of the human's routes (the Trade screen's row): `{id, origin, destination, foreign, title,
/// open, state, length, pay_x100, pay, terms, dest_pay_x100, dest_pay, growth, growth_max,
/// growth_text, established, arrived, caravans: [{slot, unit, x, y, state, text, plunder, cargo}],
/// segments}`.
pub fn route_json(v: &Viewer, r: RouteId) -> Json {
    let g = v.g;
    let route = &g.routes[r];
    let d = &g.d.defines;
    let water = route.path.iter().any(|&q| g.plot_is_water(q));
    let foreign = g.cities.contains(route.destination) && g.cities[route.destination].owner != route.owner;
    let state = match route.closed {
        Some(why) => format!("Closed: {}", lower_first(&why.text(g))),
        None if !route.arrived => "Open. Pays from the first caravan's arrival.".to_string(),
        None => "Open.".to_string(),
    };
    let caravans: Vec<Json> = (0..route.slots.len())
        .map(|i| {
            let s = &route.slots[i];
            let unit = s.unit.filter(|&u| g.units.contains(u));
            let (x, y) = unit.map_or((Json::Null, Json::Null), |u| (Json::from(g.units[u].x), Json::from(g.units[u].y)));
            let kind = match unit {
                Some(_) => "road",
                None if s.lost_turn >= 0 && s.reform_turns > 0 => "lost",
                None if !route.is_open() => "waiting",
                None => "forming",
            };
            crate::jobj! {
                "slot" => i,
                "unit" => unit,
                "x" => x,
                "y" => y,
                "state" => kind,
                "text" => caravan_state(v, route, i),
                "plunder" => unit.map(|_| plunder(g, r, i)),
                // Cargo (docs/TRADE.md, section 5) and the escorts (4.4).
                "cargo" => s.cargo.map(|b| Json::idx(b)),
                "cargo_text" => cargo_line(g, r, i),
                "cargo_state" => cargo_state_key(slot_cargo(g, r, i).state),
                "escorts" => escorts_json(g, r, i),
            }
        })
        .collect();
    let pay = route.owner_pay_x100();
    crate::jobj! {
        "id" => r,
        "origin" => city_ref(v, route.origin),
        "destination" => city_ref(v, route.destination),
        "foreign" => foreign,
        "title" => title(v, route),
        "open" => route.is_open(),
        "closed_key" => route.closed.map(RouteRefusal::key),
        "state" => state,
        "length" => route.length,
        "pay_x100" => pay,
        "pay" => format!("Pay: {} a turn.", pay_text(pay)),
        // The terms of the pay now (none while closed or before the first arrival: it pays nothing).
        "terms" => if route.is_open() && route.arrived { pay_terms(g, route.owner, route.origin, route.destination, route.length, water, route.growth) } else { Vec::new() },
        "dest_pay_x100" => route.dest_pay_x100,
        "dest_pay" => if foreign { format!("{} gets {} a turn.", name(v, route.destination), pay_text(route.dest_pay_x100)) } else { format!("{}: {} a turn.", name(v, route.destination), pay_text(route.dest_pay_x100)) },
        "growth" => route.growth,
        "growth_max" => d.trade_growth_max,
        "growth_text" => format!("Growth: +{}% of {}%.", route.growth, d.trade_growth_max),
        "established" => route.established,
        "arrived" => route.arrived,
        "caravans" => Json::Arr(caravans),
        "segments" => segments(v, &route.path),
    }
}

/// A route of another player into one of the human's cities: `{id, owner, origin, destination,
/// text, pay_x100, pay}` (`From Ithralia (Lanun) to Vallus: 1.4 a turn.`).
fn into_json(v: &Viewer, r: RouteId) -> Json {
    let g = v.g;
    let route = &g.routes[r];
    // What it brings: the resources its caravans deliver to the human (docs/TRADE.md, 9).
    let brought: Vec<String> = route.slots.iter().filter_map(|s| s.delivering.filter(|&(p, _)| p == v.h).map(|(_, b)| bonus_name(g, b))).collect();
    let pay = route.dest_pay_x100;
    let who = player_name(v, route.owner);
    crate::jobj! {
        "id" => r,
        "owner" => route.owner,
        "origin" => city_ref(v, route.origin),
        "destination" => city_ref(v, route.destination),
        "open" => route.is_open(),
        "text" => format!("From {} ({who}) to {}.", name(v, route.origin), name(v, route.destination)),
        "pay_x100" => pay,
        "pay" => format!("{} a turn to {}.", pay_text(pay), name(v, route.destination)),
        "brings" => (!brought.is_empty()).then(|| format!("Brings: {}.", brought.join(", "))),
    }
}

fn cargo_state_key(s: CargoState) -> &'static str {
    match s {
        CargoState::NoCargo => "none",
        CargoState::Delivering => "delivering",
        CargoState::CaravanLost => "lost",
        CargoState::RouteClosed => "closed",
        CargoState::NotArrived => "not_arrived",
        CargoState::Domestic => "domestic",
        CargoState::NoCopy => "no_copy",
    }
}

fn class_name(c: CargoClass) -> &'static str {
    match c {
        CargoClass::Luxury => "luxury",
        CargoClass::Mana => "mana",
        CargoClass::Strategic => "strategic",
    }
}

/// The cargo selectors of a route of `owner` from `origin` to `dest` (docs/TRADE.md, 5 and 9; a
/// refit names its route), null for a domestic destination: `{options: [{bonus, name, class,
/// premium, only_copy, text, warn}], preselect: [bonus | null, bonus | null], halves: [x100, x100]}`.
/// `halves` are the two caravans' halves of the pay before premiums (the client shows the pay with
/// the chosen cargos: each half x (100 + its option's premium) / 100, the rules' rule in
/// `route_pay`). The preselection (the spec's): the luxuries the recipient lacks (the new
/// premium), not an only copy, the highest premium first, a different one for each caravan; never a
/// strategic or mana resource.
pub fn cargo_json(v: &Viewer, origin: CityId, dest: CityId, route: Option<RouteId>, total_x100: i32) -> Json {
    let g = v.g;
    if !g.cities.contains(dest) || g.cities[dest].owner == v.h {
        return Json::Null;
    }
    let rec = civ(g, g.cities[dest].owner);
    let new_pct = g.d.defines.trade_cargo_new_percent;
    let opts = g.route_cargo_options(v.h, origin, dest, route);
    let options: Vec<Json> = opts
        .iter()
        .map(|o| {
            let n = bonus_name(g, o.bonus);
            let has = if o.premium_percent >= new_pct { format!("{rec} has none.") } else { format!("{rec} has it.") };
            let text = format!("{n} ({}): {has} +{}% on this caravan.", class_name(o.class), o.premium_percent);
            crate::jobj! {
                "bonus" => Json::idx(o.bonus),
                "name" => n.as_str(),
                "class" => class_name(o.class),
                "premium" => o.premium_percent,
                "only_copy" => o.only_copy,
                "text" => text,
                "warn" => o.only_copy.then(|| format!("Your only copy: your cities lose {n}.")),
            }
        })
        .collect();
    let mut pick: Vec<&ffh_core::trade::CargoOption> = opts.iter().filter(|o| o.class == CargoClass::Luxury && !o.only_copy && o.premium_percent >= new_pct).collect();
    pick.sort_by_key(|o| (std::cmp::Reverse(o.premium_percent), o.bonus));
    let mut preselect = [Json::Null, Json::Null];
    for (i, o) in pick.iter().take(CARAVAN_SLOTS).enumerate() {
        preselect[i] = Json::idx(o.bonus);
    }
    // A refit starts from the route's cargos.
    if let Some(r) = route.filter(|&r| g.routes.contains(r)) {
        for (i, p) in preselect.iter_mut().enumerate() {
            *p = g.routes[r].slots[i].cargo.map_or(Json::Null, |b| Json::idx(b));
        }
    }
    let halves = [total_x100 - total_x100 / 2, total_x100 / 2];
    crate::jobj! { "options" => Json::Arr(options), "preselect" => Json::Arr(preselect.to_vec()), "halves" => crate::jarr![halves[0], halves[1]] }
}

/// The limits line: `Routes: 1 of 2.` with ` +1 from a coastal city.` when the player has
/// coastal routes (docs/TRADE.md, 3.1).
pub fn limits_text(g: &Game, p: PlayerId) -> String {
    let (used, limit, coastal) = (g.player_routes_used(p), g.player_route_limit(p), g.player_coastal_route_extra(p));
    let mut s = format!("Routes: {used} of {}.", limit + coastal);
    if coastal > 0 {
        s.push_str(&format!(" {coastal} of them only from a coastal city."));
    }
    s
}

/// `This city: 0 of 1.`
fn city_limit_text(g: &Game, c: CityId) -> String {
    format!("This city: {} of {}.", g.city_origin_used(c), g.city_origin_limit(c))
}

/// The view's `trade`: `{limits: {used, limit, coastal, text}, routes: [route_json], into: [routes
/// of others into the human's cities], lines: [{route, owner, mine, open, segments}]}`. The lines:
/// the human's open routes and the other players' open routes, each over the plots the human has
/// revealed.
pub fn trade_json(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let mut routes = Vec::new();
    let mut into = Vec::new();
    let mut lines = Vec::new();
    for (r, route) in g.routes.iter() {
        let mine = route.owner == h;
        if mine {
            routes.push(route_json(v, r));
        } else if g.cities.contains(route.destination) && g.cities[route.destination].owner == h {
            into.push(into_json(v, r));
        }
        if route.is_open() {
            let segs = segments(v, &route.path);
            if segs.as_arr().is_some_and(|a| !a.is_empty()) {
                lines.push(crate::jobj! { "route" => r, "owner" => route.owner, "mine" => mine, "segments" => segs });
            }
        }
    }
    crate::jobj! {
        "limits" => crate::jobj! {
            "used" => g.player_routes_used(h),
            "limit" => g.player_route_limit(h),
            "coastal" => g.player_coastal_route_extra(h),
            "text" => limits_text(g, h),
        },
        "routes" => Json::Arr(routes),
        "into" => Json::Arr(into),
        "lines" => Json::Arr(lines),
    }
}

/// The city panel's routes of city `c` (the human's own; null for another's): `{from: [{route,
/// text, pay_x100}], into: [...], limit}`. Each with the pay to this city: a route from it pays its
/// owner here; a route into it pays this city its share.
pub fn city(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if !g.cities.contains(c) || g.cities[c].owner != v.h {
        return Json::Null;
    }
    let from: Vec<Json> = g
        .city_routes_from(c)
        .into_iter()
        .map(|r| {
            let route = &g.routes[r];
            let pay = route.owner_pay_x100();
            let closed = route.closed.map_or_else(String::new, |why| format!(" Closed: {}", lower_first(&why.text(g))));
            crate::jobj! { "route" => r, "pay_x100" => pay, "text" => format!("To {}: {} a turn.{closed}", dest_name(v, route), pay_text(pay)) }
        })
        .collect();
    let into: Vec<Json> = g
        .city_routes_into(c)
        .into_iter()
        .map(|r| {
            let route = &g.routes[r];
            let pay = route.dest_pay_x100;
            let who = if route.owner == v.h { String::new() } else { format!(" ({})", player_name(v, route.owner)) };
            crate::jobj! { "route" => r, "pay_x100" => pay, "text" => format!("From {}{who}: {} a turn.", name(v, route.origin), pay_text(pay)) }
        })
        .collect();
    crate::jobj! { "from" => Json::Arr(from), "into" => Json::Arr(into), "limit" => city_limit_text(g, c) }
}

/// The destination's name with its owner when foreign: `Vallus (Lanun)`.
fn dest_name(v: &Viewer, route: &Route) -> String {
    let g = v.g;
    let n = name(v, route.destination);
    if g.cities.contains(route.destination) && g.cities[route.destination].owner != route.owner && v.city_revealed(route.destination) {
        format!("{n} ({})", civ(g, g.cities[route.destination].owner))
    } else {
        n
    }
}

/// The unit panel's Establish action of a selection holding a Caravan of the human's in no route
/// (none otherwise): `{kind: "trade", trade: "establish", unit, label, enabled, why, help}`; its
/// `cmd` is null (the client opens the Establish dialog: the `route_offers` query).
pub fn establish_action(v: &Viewer, units: &[UnitId]) -> Option<Json> {
    let g = v.g;
    let &u = units.iter().find(|&&u| g.unit_is_caravan(u) && g.units[u].owner == v.h && !g.unit_in_route(u))?;
    let why = g.unit_establish_origin_refusal(u);
    let mut j = crate::jobj! {
        "kind" => "trade",
        "trade" => "establish",
        "unit" => u,
        "label" => "Establish route",
        "enabled" => why.is_none(),
        "help" => "Choose a destination. The caravan travels there by itself; the route pays from its arrival.",
        "cmd" => Json::Null,
    };
    if let Some(r) = why {
        j.set("why", r.text(g));
    }
    Some(j)
}

/// The routes Caravan `u` of the human's (in no route) could refit where it stands: those of the
/// human's whose origin is its city (docs/TRADE.md, 5, "Fixed").
fn refit_routes(v: &Viewer, u: UnitId) -> Vec<RouteId> {
    let g = v.g;
    let Some(c) = g.unit_plot_opt(u).and_then(|q| g.map.plots[q].plot_city).filter(|&c| g.cities.contains(c) && g.cities[c].owner == v.h) else { return Vec::new() };
    g.city_routes_from(c).into_iter().filter(|&r| g.routes[r].owner == v.h).collect()
}

/// The unit panel's Refit actions of a selection holding a Caravan of the human's in no route in
/// a city that is the origin of a route of its (docs/TRADE.md, 5): one per route, `{kind: "trade",
/// trade: "refit", unit, route, label, enabled, why, help}`; the dialog sends `refit_route`.
pub fn refit_actions(v: &Viewer, units: &[UnitId]) -> Vec<Json> {
    let g = v.g;
    let Some(&u) = units.iter().find(|&&u| g.unit_is_caravan(u) && g.units[u].owner == v.h && !g.unit_in_route(u)) else { return Vec::new() };
    let routes = refit_routes(v, u);
    let many = routes.len() > 1;
    routes
        .into_iter()
        .map(|r| {
            let route = &g.routes[r];
            // A refit with no cargo is refused only where the route is gone: the cargo choice is the dialog's.
            let why = g.unit_refit_refusal(u, r, [None, None]);
            let domestic = g.cities.contains(route.destination) && g.cities[route.destination].owner == route.owner;
            let why = why.map(|w| w.text(g)).or_else(|| domestic.then(|| RouteRefusal::CargoDomestic.text(g)));
            let label = if many { format!("Refit route to {}", name(v, route.destination)) } else { "Refit route".to_string() };
            let mut j = crate::jobj! {
                "kind" => "trade",
                "trade" => "refit",
                "unit" => u,
                "route" => r,
                "label" => label,
                "enabled" => why.is_none(),
                "help" => REFIT_LINE,
                "cmd" => Json::Null,
            };
            if let Some(w) = why {
                j.set("why", w);
            }
            j
        })
        .collect()
}

/// The Refit dialog's rule line.
pub const REFIT_LINE: &str = "The route keeps its growth. This Caravan is used up.";

/// `route_offers {unit}`: the Establish dialog of Caravan `unit` of the human's (docs/TRADE.md,
/// 3.3 and 9): `{unit, origin: {id, name} | null, enabled, why, limits, offers: [{city, name,
/// owner, civ, foreign, length, pay_x100, dest_pay_x100, seas, text, segments, cmd}]}`, the offers
/// by pay (the highest first, then the city id). Each offer is one path search.
pub fn offers(v: &Viewer, u: UnitId) -> Result<Json, QueryErr> {
    let g = v.g;
    match g.units.get(u) {
        None => return Err(QueryErr::Gone),
        Some(unit) if unit.owner != v.h => return Err(QueryErr::NotOwner),
        Some(_) => {}
    }
    if !g.unit_is_caravan(u) {
        return Err(QueryErr::Invalid(format!("unit {} is not a Caravan", u.0)));
    }
    let why = g.unit_establish_origin_refusal(u);
    let origin = g.unit_plot_opt(u).and_then(|q| g.map.plots[q].plot_city).filter(|&c| g.cities.contains(c) && g.cities[c].owner == v.h);
    let mut limits = limits_text(g, v.h);
    if let Some(c) = origin {
        limits.push(' ');
        limits.push_str(&city_limit_text(g, c));
    }
    let mut list = match (why, origin) {
        (None, Some(c)) => g.route_offers(c),
        _ => Vec::new(),
    };
    list.sort_by_key(|o| (std::cmp::Reverse(o.owner_pay_x100), o.destination));
    let rows: Vec<Json> = list
        .iter()
        .map(|o| {
            let c = o.destination;
            let owner = g.cities[c].owner;
            let n = name(v, c);
            let crossed = seas(v, &o.path);
            let mut text = if o.foreign { format!("{n} ({}). ", civ(g, owner)) } else { format!("{n}. ") };
            text.push_str(&format!("{} plots. {} a turn.", o.length, pay_text(o.owner_pay_x100)));
            let passes: Vec<String> = crossed.iter().filter_map(|s| s.get("text").and_then(Json::as_str).map(|t| t.trim_end_matches('.').to_string())).collect();
            if !passes.is_empty() {
                text.push_str(&format!(" Passes {}.", passes.join(", ")));
            }
            let cmd = Command::EstablishRoute { unit: u, city: c, cargo: [None, None] }.to_json();
            crate::jobj! {
                "city" => c,
                "name" => n,
                "owner" => owner,
                "civ" => if o.foreign { Json::from(civ(g, owner)) } else { Json::Null },
                "foreign" => o.foreign,
                "length" => o.length,
                "pay_x100" => o.owner_pay_x100,
                "pay" => pay_text(o.owner_pay_x100),
                "dest_pay_x100" => o.dest_pay_x100,
                "seas" => Json::Arr(crossed),
                "text" => text,
                // The preview's line (the plots the human has revealed).
                "segments" => segments(v, &o.path),
                // Cargo (docs/TRADE.md, section 5): each caravan's selector (null: a domestic route).
                "cargo" => match origin {
                    Some(oc) if o.foreign => cargo_json(v, oc, c, None, o.owner_pay_x100),
                    _ => Json::Null,
                },
                "cmd" => cmd,
            }
        })
        .collect();
    // The routes it could refit here (docs/TRADE.md, 5), each with its cargo selectors.
    let refits: Vec<Json> = refit_routes(v, u)
        .into_iter()
        .map(|r| {
            let route = &g.routes[r];
            let water = route.path.iter().any(|&q| g.plot_is_water(q));
            let total = g.route_pay_total_x100(route.owner, route.origin, route.destination, route.length, water, route.growth);
            let why = g.unit_refit_refusal(u, r, [None, None]).map(|w| w.text(g));
            crate::jobj! {
                "route" => r,
                "title" => title(v, route),
                "line" => REFIT_LINE,
                "why" => why,
                "cargo" => cargo_json(v, route.origin, route.destination, Some(r), total),
                "cmd" => Command::RefitRoute { unit: u, route: r, cargo: [None, None] }.to_json(),
            }
        })
        .collect();
    Ok(crate::jobj! {
        "unit" => u,
        "refits" => Json::Arr(refits),
        "origin" => origin.map_or(Json::Null, |c| crate::jobj! { "id" => c, "name" => name(v, c), "x" => g.cities[c].x, "y" => g.cities[c].y }),
        "enabled" => why.is_none(),
        "why" => why.map(|r| r.text(g)),
        "limits" => limits,
        "offers" => Json::Arr(rows),
        "empty" => if why.is_none() && list.is_empty() { Json::from("No destination: a city must be revealed, reachable, and at peace with open borders when foreign.") } else { Json::Null },
    })
}

/// The unit type whose figure a unit is drawn with when it is not its own (docs/TRADE.md, 2: a
/// caravan on a water plot is drawn as a merchant ship, the Galley's figure): `None` for its own.
pub fn unit_art(g: &Game, u: UnitId) -> Option<ffh_data::UnitType> {
    if !g.unit_in_route(u) || !g.unit_plot_opt(u).is_some_and(|q| g.plot_is_water(q)) {
        return None;
    }
    g.d.ty::<ffh_data::UnitType>("UNIT_GALLEY").or_else(|| g.d.ty::<ffh_data::UnitType>("UNIT_TRIREME"))
}

/// The rule said where an Escort of a caravan is offered (docs/TRADE.md, 4.4).
pub const ESCORT_LINE: &str = "A caravan standing with its escort moves only as far as the escort can.";

/// An Escort target's name, as `escort_target_name` words it with the city names resolved
/// (`Caravan: Ithralia to Vallus (1)`).
fn caravan_name(v: &Viewer, r: RouteId, slot: u8) -> String {
    match v.g.routes.get(r) {
        Some(route) => format!("Caravan: {} to {} ({})", name(v, route.origin), name(v, route.destination), slot as usize + 1),
        None => "a caravan".to_string(),
    }
}

/// The caravans army or fleet `a` could escort, for its selector: `[{route, slot, name}]`
/// (`army_escort_caravans`, named by `escort_target_name`).
pub fn escort_choices(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    Json::Arr(
        g.army_escort_caravans(a)
            .into_iter()
            .map(|(r, slot)| crate::jobj! { "route" => r, "slot" => slot as i32, "name" => caravan_name(v, r, slot) })
            .collect(),
    )
}

/// Why army or fleet `a`, escorting caravan `slot` of route `r`, waits now, null when it does not
/// (derived from the route's slot, as the rules' messages say it): `caravan lost`, `route closed`,
/// `the caravan waits in Ithralia`, `the caravan is at sea` (a land army), `the caravan is on land`
/// (a fleet).
fn escort_wait(v: &Viewer, a: ArmyId, r: RouteId, slot: u8) -> Option<String> {
    let g = v.g;
    let route = g.routes.get(r)?;
    let s = route.slots.get(slot as usize)?;
    let fleet = g.army_is_fleet(a);
    match s.unit.filter(|&u| g.units.contains(u)) {
        None if s.lost_turn >= 0 && s.reform_turns > 0 => Some("caravan lost".to_string()),
        None if !route.is_open() => Some("route closed".to_string()),
        None => Some(format!("the caravan waits in {}", name(v, route.origin))),
        Some(u) => {
            let q = g.unit_plot(u);
            let water = g.plot_is_water(q);
            if !fleet && water {
                Some("the caravan is at sea".to_string())
            } else if fleet && !water && g.map.plots[q].plot_city.is_none() {
                Some("the caravan is on land".to_string())
            } else {
                None
            }
        }
    }
}

/// The station of army or fleet `a` while it escorts a caravan (docs/TRADE.md, 4.4), null
/// otherwise: `{kind: "escort", route, slot, text, wait}` (`Escorting Caravan: Ithralia to Vallus
/// (1).`; `wait`: `Waits: caravan lost.`).
pub fn escort_station(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    let Station::Escort(EscortTarget::Caravan(r, slot)) = g.armies[a].station else { return Json::Null };
    crate::jobj! {
        "kind" => "escort",
        "route" => r,
        "slot" => slot as i32,
        "army" => Json::Null,
        "group" => Json::Null,
        "text" => format!("Escorting {}.", caravan_name(v, r, slot)),
        "wait" => escort_wait(v, a, r, slot).map(|w| format!("Waits: {w}.")),
    }
}

/// The unit panel's escort actions of army or fleet `a` (the whole of it selected), each
/// `(enabled, cmd, extra)` as the other army actions: `Stop escorting` while it escorts a caravan
/// (`army: "stop_escort"`), and `Escort Caravan: A to B (1)` for each caravan it could escort
/// (`army: "escort_caravan"`, `route`, `slot`), with the pace rule as `help`.
pub fn escort_actions(v: &Viewer, a: ArmyId) -> Vec<(bool, Json, Json)> {
    let g = v.g;
    let mut out = Vec::new();
    let held = match g.armies[a].station {
        Station::Escort(EscortTarget::Caravan(r, slot)) => Some((r, slot)),
        _ => None,
    };
    if let Some((r, slot)) = held {
        let n = caravan_name(v, r, slot);
        out.push((
            true,
            Command::ArmyStation { army: a, station: Station::None }.to_json(),
            crate::jobj! { "army" => "stop_escort", "label" => "Stop escorting", "help" => format!("The {} ends its escort of {n}.", g.armies[a].name) },
        ));
    }
    if g.armies[a].disband_to.is_some() {
        return out;
    }
    for (r, slot) in g.army_escort_caravans(a) {
        if held == Some((r, slot)) {
            continue;
        }
        let n = caravan_name(v, r, slot);
        out.push((
            true,
            Command::ArmyStation { army: a, station: Station::Escort(EscortTarget::Caravan(r, slot)) }.to_json(),
            crate::jobj! { "army" => "escort_caravan", "label" => format!("Escort {n}"), "help" => ESCORT_LINE, "route" => r, "slot" => slot as i32 },
        ));
    }
    out
}

/// The attack preview's plunder line for plot `plot` (docs/TRADE.md, section 9): what killing the
/// caravans of other players there the human sees would bring (`Plunder: 14 gold.`, `Plunder: 28
/// gold (2 caravans).`); none when there is none.
pub fn attack_plunder(v: &Viewer, plot: PlotIdx) -> Vec<String> {
    let g = v.g;
    let (mut n, mut gold) = (0, 0);
    for &u in &g.map.plots[plot].units {
        if g.units[u].owner == v.h || !v.unit_visible(u) {
            continue;
        }
        if let Some((r, slot)) = caravan_slot(g, u) {
            n += 1;
            gold += plunder(g, r, slot);
        }
    }
    match n {
        0 => Vec::new(),
        1 => vec![plunder_line(gold)],
        _ => vec![format!("Plunder: {gold} gold ({n} caravans).")],
    }
}

/// The help line of a data key of docs/TRADE.md section 7 (techs, civics, buildings), bare:
/// `+1 route.`, `+1 route from a coastal city.`, `This city can be the origin of 1 more route.`,
/// `+50% pay of the route from this city.`, `+25% pay of a foreign route from this city.` The
/// word "route" carries the Route keyword (its reminder says what a route is).
pub fn help_line(key: HelpKey, n: i32) -> String {
    const ROUTE: &str = "[KW=KEYWORD_ROUTE]route[\\KW]";
    const ROUTES: &str = "[KW=KEYWORD_ROUTE]routes[\\KW]";
    let signed = |n: i32| if n >= 0 { format!("+{n}") } else { n.to_string() };
    let noun = |n: i32| if n.abs() == 1 { ROUTE } else { ROUTES };
    let body = match key {
        HelpKey::Routes => format!("{} {}.", signed(n), noun(n)),
        HelpKey::Coastal => format!("{} {} from a coastal city.", signed(n), noun(n)),
        HelpKey::Origin if n >= 0 => format!("This city can be the origin of {n} more {}.", noun(n)),
        HelpKey::Origin => format!("This city can be the origin of {} fewer {}.", -n, noun(n)),
        HelpKey::Modifier => format!("{}% pay of the {ROUTE} from this city.", signed(n)),
        HelpKey::Foreign => format!("{}% pay of a foreign {ROUTE} from this city.", signed(n)),
    };
    format!("[ICON_BULLET]{body}")
}

/// Which data key a [`help_line`] says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpKey {
    /// `trade_routes` of a tech or civic, `global_trade_routes` of a building, a vote's.
    Routes,
    /// `coastal_trade_routes` (a civic, a building).
    Coastal,
    /// A building's `trade_routes`: the city's `extra_trade_routes`.
    Origin,
    /// A building's `trade_route_modifier`.
    Modifier,
    /// A building's `foreign_trade_route_modifier`.
    Foreign,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_reads_with_one_decimal() {
        assert_eq!(pay_text(0), "0.0");
        assert_eq!(pay_text(283), "2.8");
        assert_eq!(pay_text(285), "2.9");
        assert_eq!(pay_text(1420), "14.2");
        assert_eq!(pay_text(4), "0.0");
        assert_eq!(pay_text(5), "0.1");
        assert_eq!(pay_text(-150), "-1.5");
        assert_eq!(plunder_line(14), "Plunder: 14 gold.");
    }

    #[test]
    fn help_lines_are_bare() {
        let plain = |k, n| help_line(k, n).replace("[ICON_BULLET]", "").replace("[KW=KEYWORD_ROUTE]", "").replace("[\\KW]", "");
        assert_eq!(plain(HelpKey::Routes, 1), "+1 route.");
        assert_eq!(plain(HelpKey::Routes, 2), "+2 routes.");
        assert_eq!(plain(HelpKey::Coastal, 1), "+1 route from a coastal city.");
        assert_eq!(plain(HelpKey::Origin, 1), "This city can be the origin of 1 more route.");
        assert_eq!(plain(HelpKey::Modifier, 50), "+50% pay of the route from this city.");
        assert_eq!(plain(HelpKey::Foreign, 25), "+25% pay of a foreign route from this city.");
    }
}
