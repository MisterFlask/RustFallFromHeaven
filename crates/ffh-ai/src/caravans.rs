//! Caravans and trade routes (docs/TRADE.md, section 8; a rules change's AI, kept small:
//! docs/AI.md, "Caravans").
//!
//! - **What an offer is worth** ([`worth`]): what the route would pay us a turn, in hundredths:
//!   its owner's pay, plus the destination's share when the destination is ours. Offers whose
//!   path crosses a sea where an enemy patrols, as we see it (`sea_state_known(..).theirs > 0`,
//!   as the outposts' lanes read it), are skipped ([`offers`]).
//! - **Production** ([`production`]): a safe city trains a Caravan when the player has at least
//!   [`MIN_CITIES`] cities, no Caravan is in training, the routes free outnumber the route-less
//!   Caravans alive, and this city is, among the cities that can be an origin now, the one with
//!   the best offer (ties: the lower city id), worth enough to repay the Caravan within
//!   [`REPAY_TURNS`] turns (one hammer for one commerce). With no route free, it trains one to
//!   **replace** our least-paying route when its best offer from a city that could be its origin
//!   once that route is dissolved is worth at least [`REPLACE_FACTOR`] times what that route pays.
//! - **The Caravan with no route** ([`caravan`]): in the origin of the best offer (on its
//!   landmass) it establishes it, dissolving the route it replaces in the same decision; elsewhere
//!   it walks there (an ordinary land move, re-planned each turn); with nothing to do it waits in
//!   the nearest city of ours, deciding again each turn.
//! - **Caravans in a route** take no orders and are left alone by every AI loop
//!   (`units::play`, the garrison count `production::garrison`).
//! - **Raids**: an enemy caravan is a hunt target ([`plunder`] adds to its value in
//!   `military::find_targets`) and a warship's target at sea (`warships::targets`); units of hidden
//!   nationality attack the caravans of every team that is not ours, our vassal, our master or one
//!   we are Pleased with ([`hidden_target`], [`raid`]).
//! - **Open borders** ([`open_borders_reason`]): one attitude step toward open borders with a
//!   civilization one of whose cities would give us a better route than our best domestic one.
//! - **Value** ([`route_worth_x100`]): the data keys `trade_routes`, `coastal_trade_routes` and
//!   `trade_route_modifier` are valued at what one of our routes pays now.
//!
//! The memo ([`Memo`]) keeps each city's offers for the turn while nothing they depend on changed
//! (our routes, the cities, our wars and open borders): it saves path searches and changes no
//! decision.

use std::collections::BTreeMap;

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::trade::RouteOffer;
use ffh_data::*;

use crate::production::Choice;
use crate::units::{alive, move_to, push, skip};
use crate::view::View;
use crate::Ai;

/// The cities we hold before we train a Caravan.
pub const MIN_CITIES: i32 = 2;
/// A Caravan must repay its hammers within this many turns of its route's pay.
pub const REPAY_TURNS: i32 = 30;
/// A route is replaced by an offer worth at least this many times what it pays.
pub const REPLACE_FACTOR: i32 = 2;
/// The worth of one route (hundredths of commerce a turn) when we have none open to read it from.
pub const DEFAULT_ROUTE_X100: i32 = 300;
/// The value of a caravan as a target per gold of plunder (`military::find_targets`).
pub const PLUNDER_VALUE: i32 = 5;

// ----------------------------------------------------------------------------
// The memo
// ----------------------------------------------------------------------------

/// What a player's cities' offers depend on, besides the map: the turn, its routes, the cities,
/// its team's wars and open borders.
#[derive(Clone, PartialEq, Eq, Default)]
struct Key {
    turn: i32,
    routes: Vec<RouteId>,
    cities: Vec<CityId>,
    war: Vec<bool>,
    open: Vec<bool>,
}

/// Each player's offers per origin city (module comment).
#[derive(Default)]
pub struct Memo {
    by_player: BTreeMap<PlayerId, (Key, BTreeMap<CityId, Vec<RouteOffer>>)>,
}

fn key(g: &Game, p: PlayerId) -> Key {
    let team = g.players[p].team;
    Key {
        turn: g.game_turn(),
        routes: g.player_routes(p),
        cities: g.cities.ids(),
        war: g.team_ids().map(|t| g.teams[team].at_war[t]).collect(),
        open: g.team_ids().map(|t| g.team_is_open_borders(team, t)).collect(),
    }
}

/// Whether `offer`'s path crosses a sea where an enemy patrols that `team` knows of.
fn patrolled(g: &Game, team: TeamId, offer: &RouteOffer) -> bool {
    offer.seas.iter().any(|&s| g.sea_state_known(s, team).theirs > 0)
}

/// The offers of a route from city `c` (`Game::route_offers`), those through a patrolled sea
/// left out (module comment).
pub fn offers(ai: &Ai, g: &Game, c: CityId) -> Vec<RouteOffer> {
    let p = g.cities[c].owner;
    let k = key(g, p);
    {
        let memo = ai.caravans.lock().unwrap();
        if let Some((mk, m)) = memo.by_player.get(&p) {
            if *mk == k {
                if let Some(v) = m.get(&c) {
                    return v.clone();
                }
            }
        }
    }
    let team = g.players[p].team;
    let all = g.route_offers(c);
    let skipped = all.iter().filter(|o| patrolled(g, team, o)).count();
    if skipped > 0 {
        g.tel_decision(p, "caravan", "offer_skipped", "patrolled_sea", skipped as i64, String::new);
    }
    let kept: Vec<RouteOffer> = all.into_iter().filter(|o| !patrolled(g, team, o)).collect();
    let mut memo = ai.caravans.lock().unwrap();
    let entry = memo.by_player.entry(p).or_insert_with(|| (k.clone(), BTreeMap::new()));
    if entry.0 != k {
        *entry = (k, BTreeMap::new());
    }
    entry.1.insert(c, kept.clone());
    kept
}

// ----------------------------------------------------------------------------
// Worth
// ----------------------------------------------------------------------------

/// What `offer` would pay `p` a turn, in hundredths (module comment).
pub fn worth(offer: &RouteOffer) -> i32 {
    offer.owner_pay_x100 + if offer.foreign { 0 } else { offer.dest_pay_x100 }
}

/// What route `r` pays its owner a turn now, in hundredths, the destination's share included when
/// it is the owner's: 0 while closed (`Game` keeps no turn of closing, so a closed route counts 0
/// at once); before its first caravan arrives, what it will pay then (its stored length, growth 0).
pub fn route_pays(g: &Game, r: RouteId) -> i32 {
    let route = &g.routes[r];
    if !route.is_open() || !g.cities.contains(route.origin) || !g.cities.contains(route.destination) {
        return 0;
    }
    let domestic = g.cities[route.destination].owner == route.owner;
    if route.arrived {
        return route.owner_pay_x100() + if domestic { route.dest_pay_x100 } else { 0 };
    }
    let water = route.path.iter().any(|&q| g.plot_is_water(q));
    let total = g.route_pay_total_x100(route.owner, route.origin, route.destination, route.length, water, route.growth);
    total + if domestic { total * g.d.defines.trade_dest_percent / 100 } else { 0 }
}

/// Our least-paying route (ties: the lower id) and what it pays.
pub fn least_route(g: &Game, p: PlayerId) -> Option<(RouteId, i32)> {
    g.player_routes(p).into_iter().map(|r| (r, route_pays(g, r))).min_by_key(|&(r, pay)| (pay, r))
}

/// What one route of `p` is worth a turn, in hundredths: the mean of its open routes' pay
/// ([`route_pays`]), [`DEFAULT_ROUTE_X100`] with none. For the value functions: no path search.
pub fn route_worth_x100(g: &Game, p: PlayerId) -> i32 {
    let pays: Vec<i32> = g.player_routes(p).into_iter().filter(|&r| g.routes[r].is_open()).map(|r| route_pays(g, r)).collect();
    if pays.is_empty() {
        DEFAULT_ROUTE_X100
    } else {
        (pays.iter().sum::<i32>() / pays.len() as i32).max(1)
    }
}

/// Whether `worth` repays a Caravan of `cost` hammers within [`REPAY_TURNS`] turns.
fn repays(worth: i32, cost: i32) -> bool {
    worth as i64 * REPAY_TURNS as i64 >= cost as i64 * 100
}

/// The best offer of city `c` (ties: the lower destination id).
fn best_of(ai: &Ai, g: &Game, c: CityId) -> Option<RouteOffer> {
    offers(ai, g, c).into_iter().max_by(|a, b| worth(a).cmp(&worth(b)).then(b.destination.cmp(&a.destination)))
}

/// A plan for a Caravan: establish `offer` from `origin`, dissolving `replaces` first.
#[derive(Clone, Debug)]
pub struct Plan {
    pub origin: CityId,
    pub offer: RouteOffer,
    pub worth: i32,
    pub replaces: Option<RouteId>,
}

/// Whether city `c` could be the origin of one more route of its owner once route `r` is
/// dissolved (`Game::city_origin_refusal` with `r` left out).
fn origin_after_dissolving(g: &Game, c: CityId, r: RouteId) -> bool {
    let p = g.cities[c].owner;
    if g.cities[c].outpost {
        return false;
    }
    let old = g.routes[r].origin;
    let freed = i32::from(old == c);
    if g.city_origin_used(c) - freed >= g.city_origin_limit(c) {
        return false;
    }
    if !g.city_is_route_coastal(c) {
        let inland = g.player_routes(p).into_iter().filter(|&o| !(g.cities.contains(g.routes[o].origin) && g.city_is_route_coastal(g.routes[o].origin))).count() as i32;
        let freed_inland = i32::from(!(g.cities.contains(old) && g.city_is_route_coastal(old)));
        if inland - freed_inland >= g.player_route_limit(p) {
            return false;
        }
    }
    true
}

/// The best plan of `p` now (module comment), among origins `keep` accepts: a free route's, else
/// a replacement's. The worth test against the Caravan's cost is the caller's.
pub fn plan(ai: &Ai, g: &Game, p: PlayerId, keep: &dyn Fn(CityId) -> bool) -> Option<Plan> {
    let mut cities: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c) && keep(c)).collect();
    cities.sort();
    let free = g.player_routes_used(p) < g.player_route_limit(p) + g.player_coastal_route_extra(p);
    let mut best: Option<Plan> = None;
    if free {
        for &c in &cities {
            if g.city_origin_refusal(c).is_some() {
                continue;
            }
            let Some(o) = best_of(ai, g, c) else { continue };
            let w = worth(&o);
            if best.as_ref().is_none_or(|b| w > b.worth) {
                best = Some(Plan { origin: c, offer: o, worth: w, replaces: None });
            }
        }
        return best;
    }
    let (least, pays) = least_route(g, p)?;
    for &c in &cities {
        if !origin_after_dissolving(g, c, least) {
            continue;
        }
        let Some(o) = best_of(ai, g, c) else { continue };
        let w = worth(&o);
        if w < REPLACE_FACTOR * pays || w <= pays {
            continue;
        }
        if best.as_ref().is_none_or(|b| w > b.worth) {
            best = Some(Plan { origin: c, offer: o, worth: w, replaces: Some(least) });
        }
    }
    best
}

// ----------------------------------------------------------------------------
// Production
// ----------------------------------------------------------------------------

/// Our Caravans with no route alive, and in training.
pub fn idle_caravans(g: &Game, p: PlayerId) -> (Vec<UnitId>, i32) {
    let alive: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.unit_is_caravan(u) && !g.unit_in_route(u)).collect();
    let Some(t) = g.caravan_unit_type() else { return (alive, 0) };
    let training = g.players[p]
        .cities
        .iter()
        .filter(|&&c| g.cities.contains(c))
        .flat_map(|&c| g.cities[c].order_queue.iter())
        .filter(|o| o.order_type == OrderType::ORDER_TRAIN && o.data1 == t.i() as i32)
        .count() as i32;
    (alive, training)
}

/// What safe city `c` trains for a route (module comment), if anything.
pub fn production(ai: &Ai, g: &Game, c: CityId) -> Option<Choice> {
    let p = g.cities[c].owner;
    if g.player_is_barbarian(p) || g.player_num_real_cities(p) < MIN_CITIES {
        return None;
    }
    let t = g.caravan_unit_type()?;
    if !g.city_can_train(c, t, false, false, false, false) {
        return None;
    }
    let (alive, training) = idle_caravans(g, p);
    if training > 0 {
        return None;
    }
    let limit = g.player_route_limit(p) + g.player_coastal_route_extra(p);
    let free = limit - g.player_routes_used(p);
    // A route free that no Caravan of ours claims; with none free, one replacement at a time.
    if (free > 0 && alive.len() as i32 >= free) || (free <= 0 && !alive.is_empty()) || limit <= 0 {
        return None;
    }
    let plan = plan(ai, g, p, &|_| true)?;
    let cost = g.player_get_production_needed_unit(p, t);
    crate::trace!(g, p, "caravans: best origin {} to {} worth {} (cost {}, replaces {:?})", g.cities[plan.origin].name, g.cities[plan.offer.destination].name, plan.worth, cost, plan.replaces);
    if plan.origin != c || !repays(plan.worth, cost) {
        return None;
    }
    g.tel_decision(p, "caravan", if plan.replaces.is_some() { "ask_replace" } else { "ask" }, if plan.offer.foreign { "foreign" } else { "domestic" }, plan.worth as i64, String::new);
    Some(Choice::Train(t, UnitAIType::UNITAI_UNKNOWN))
}

// ----------------------------------------------------------------------------
// The Caravan with no route
// ----------------------------------------------------------------------------

/// Whether group `grp` (head `u`) is a Caravan of ours with no route, whose orders are
/// re-planned every turn.
pub fn replans(g: &Game, u: UnitId) -> bool {
    g.unit_is_caravan(u) && !g.unit_in_route(u)
}

/// A Caravan of ours with no route (module comment). False if `u` is none; true: its turn is
/// decided.
pub fn caravan(ai: &Ai, g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || !g.units.contains(u) || !g.unit_is_caravan(u) || g.unit_in_route(u) {
        return false;
    }
    let p = v.p;
    let here = g.unit_plot(u);
    let area = g.map.plots[here].area;
    let on_land = |c: CityId| g.map.plots[g.city_plot(c)].area == area;
    let cost = g.caravan_unit_type().map_or(0, |t| g.player_get_production_needed_unit(p, t));
    let plan = plan(ai, g, p, &on_land).filter(|pl| repays(pl.worth, cost));
    let Some(plan) = plan else {
        // Nothing worth a route: wait in the nearest city of ours, deciding again each turn.
        let in_ours = g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p);
        if !in_ours {
            if let Some(c) = crate::units::nearest_city(g, p, here).filter(|&c| on_land(c)) {
                move_to(g, grp, g.city_plot(c), 0);
            }
        }
        if alive(g, grp) && g.group_ready_to_move(grp, true) {
            skip(g, grp);
        }
        return true;
    };
    let origin_plot = g.city_plot(plan.origin);
    if here != origin_plot {
        crate::trace!(g, p, "caravan {:?} walks to {} for {}", u, g.cities[plan.origin].name, g.cities[plan.offer.destination].name);
        move_to(g, grp, origin_plot, 0);
        if alive(g, grp) && g.units.contains(u) && g.group_ready_to_move(grp, true) {
            skip(g, grp);
        }
        return true;
    }
    let foreign = plan.offer.foreign;
    if let Some(r) = plan.replaces {
        // The destination must still pass with the route gone (the path, the borders).
        if g.route_destination_refusal(p, plan.origin, plan.offer.destination).is_some() {
            skip(g, grp);
            return true;
        }
        g.tel_decision(p, "caravan", "replace", if foreign { "foreign" } else { "domestic" }, plan.worth as i64, String::new);
        g.route_dissolve(r);
    }
    match g.unit_establish_route(u, plan.offer.destination, [None, None]) {
        Ok(_) => {
            crate::trace!(g, p, "caravan {:?} establishes {} to {} (worth {})", u, g.cities[plan.origin].name, g.cities[plan.offer.destination].name, plan.worth);
            g.tel_decision(p, "caravan", "establish", if foreign { "foreign" } else { "domestic" }, plan.worth as i64, String::new);
        }
        Err(why) => {
            crate::trace!(g, p, "caravan {:?} could not establish: {:?}", u, why);
            g.tel_decision(p, "caravan", if plan.replaces.is_some() { "replace_failed" } else { "establish_failed" }, why.key(), 0, String::new);
            if alive(g, grp) && g.group_ready_to_move(grp, true) {
                skip(g, grp);
            }
        }
    }
    true
}

// ----------------------------------------------------------------------------
// Raids
// ----------------------------------------------------------------------------

/// The plunder (gold) that killing caravan `e` of a route would give: `TRADE_PLUNDER_TURNS` x
/// its pay, rounded; 0 for a unit in no route.
pub fn plunder(g: &Game, e: UnitId) -> i32 {
    let Some(r) = g.units[e].route.filter(|&r| g.routes.contains(r)) else { return 0 };
    let pay = g.routes[r].slots.iter().find(|s| s.unit == Some(e)).map_or(0, |s| s.pay_x100);
    ffh_core::trade::round_x100(g.d.defines.trade_plunder_turns * pay).max(0)
}

/// Whether a unit of hidden nationality of `p` may attack the caravans of `owner`: not ours, our
/// vassal or master, nor one we are Pleased or better with.
pub fn hidden_target(g: &Game, p: PlayerId, owner: PlayerId) -> bool {
    let (team, ot) = (g.players[p].team, g.players[owner].team);
    if ot == team || g.player_is_barbarian(owner) || g.teams[team].vassal[ot] || g.teams[ot].vassal[team] {
        return false;
    }
    crate::diplomacy::attitude(g, p, owner) < AttitudeType::ATTITUDE_PLEASED
}

/// Whether plot `q` holds, as `team` (of player `p`) sees it, a caravan of a route a unit of `p`
/// may attack: of a team at war with ours, or, for an attacker of hidden nationality, one
/// [`hidden_target`] accepts. Returns the caravan with the most plunder.
pub fn caravan_on(g: &Game, p: PlayerId, q: PlotIdx, hidden: bool) -> Option<UnitId> {
    let team = g.players[p].team;
    if !g.plot_is_visible(q, team) {
        return None;
    }
    g.map.plots[q]
        .units
        .iter()
        .copied()
        .filter(|&e| g.unit_in_route(e) && !g.unit_is_invisible(e, team, false))
        .filter(|&e| {
            let owner = g.units[e].owner;
            g.at_war(team, g.players[owner].team) || hidden && hidden_target(g, p, owner)
        })
        .max_by_key(|&e| (plunder(g, e), std::cmp::Reverse(e)))
}

/// Records an attack of ours on a plot holding a caravan (telemetry `caravan.attack`).
pub fn note_attack(g: &Game, p: PlayerId, q: PlotIdx, hidden: bool) {
    let team = g.players[p].team;
    if let Some(e) = g.map.plots[q].units.iter().copied().find(|&e| g.unit_in_route(e)) {
        let war = g.at_war(team, g.unit_get_team(e));
        g.tel_decision(p, "caravan", "attack", if war && !hidden { "war" } else { "hidden" }, plunder(g, e) as i64, String::new);
    }
}

/// The odds (per mille) a raid on a caravan needs.
pub const RAID_ODDS: i32 = crate::warships::ODDS_BAR;

/// A land unit of hidden nationality (module comment): it attacks a caravan it may attack
/// ([`caravan_on`]) next to a plot it reaches this turn, at [`RAID_ODDS`] or better, the most
/// plunder first. True if it did.
pub fn raid(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if v.barbarian || !g.unit_is_hidden_nationality(u) || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || g.unit_moves_left(u) <= 0 || !g.unit_can_attack(u) || crate::view::attack_power(g, u) <= 0 {
        return false;
    }
    let reach = v.flood.run(g, u, 0, false);
    let mut best: Option<(i32, PlotIdx, PlotIdx, i32)> = None; // (plunder, target, from, odds)
    for r in reach.iter().filter(|r| r.turns == 0) {
        if g.map.plots[r.plot].units.iter().any(|&o| g.units[o].owner != p) {
            continue;
        }
        let (x, y) = g.map.xy(r.plot);
        for dir in 0..DirectionType::NUM {
            let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
            if g.plot_is_water(q) || g.map.plots[q].plot_city.is_some() {
                continue;
            }
            let Some(e) = caravan_on(g, p, q, true) else { continue };
            if !g.unit_can_move_into(u, q, true, false, false) {
                continue;
            }
            let Some(def) = g.plot_get_best_defender(q, None, Some(p), Some(u), false, false, false) else { continue };
            // Only a plot the caravans hold alone: no fight with their escort.
            if g.map.plots[q].units.iter().any(|&o| !g.unit_is_caravan(o) && g.unit_can_defend(o, None)) {
                continue;
            }
            let odds = g.get_combat_odds(u, def);
            let loot = plunder(g, e);
            if odds >= RAID_ODDS && best.is_none_or(|b| (loot, odds) > (b.0, b.3)) {
                best = Some((loot, q, r.plot, odds));
            }
        }
    }
    let Some((_, q, from, odds)) = best else { return false };
    if from != g.unit_plot(u) {
        move_to(g, grp, from, 0);
        if !alive(g, grp) || !g.units.contains(u) || g.unit_plot(u) != from || g.unit_moves_left(u) <= 0 {
            return true;
        }
    }
    crate::trace!(g, p, "{:?} of hidden nationality raids a caravan at {:?} at odds {}", u, g.map.xy(q), odds);
    note_attack(g, p, q, true);
    let (x, y) = g.map.xy(q);
    push(g, grp, MissionType::MISSION_MOVE_TO, x, y, MOVE_DIRECT_ATTACK, false);
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
    true
}

// ----------------------------------------------------------------------------
// Open borders
// ----------------------------------------------------------------------------

/// Whether a route to a city of `other` would be worth more to `p`, were borders open, than its
/// best domestic route (with a route free) or its least-paying route (with none): estimated by
/// plot distance as the length, over land, at growth 0 (`Game::route_pay_total_x100`; no path
/// search). Never with the team we prepare a war on or our worst enemy.
pub fn open_borders_reason(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let (team, ot) = (g.players[p].team, g.players[other].team);
    if team == ot || g.player_is_barbarian(p) || g.player_is_barbarian(other) || crate::diplomacy::preparing_target(g, team) == Some(ot) || g.teams[team].ai_worst_enemy == Some(ot) {
        return false;
    }
    let limit = g.player_route_limit(p) + g.player_coastal_route_extra(p);
    if limit <= 0 {
        return false;
    }
    let ours: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c) && !g.cities[c].outpost).collect();
    let theirs: Vec<CityId> = g.players[other].cities.iter().copied().filter(|&c| g.cities.contains(c) && !g.cities[c].outpost && g.city_is_revealed(c, team, false)).collect();
    let dist = |a: CityId, b: CityId| g.plot_distance(g.cities[a].x, g.cities[a].y, g.cities[b].x, g.cities[b].y);
    let foreign = ours.iter().flat_map(|&a| theirs.iter().map(move |&b| (a, b))).map(|(a, b)| g.route_pay_total_x100(p, a, b, dist(a, b), false, 0)).max().unwrap_or(0);
    if foreign <= 0 {
        return false;
    }
    let bar = if g.player_routes_used(p) < limit {
        let pct = g.d.defines.trade_dest_percent;
        let linked = |a: CityId, b: CityId| g.player_routes(p).into_iter().any(|r| (g.routes[r].origin, g.routes[r].destination) == (a, b) || (g.routes[r].origin, g.routes[r].destination) == (b, a));
        ours.iter()
            .flat_map(|&a| ours.iter().map(move |&b| (a, b)))
            .filter(|&(a, b)| a != b && !linked(a, b))
            .map(|(a, b)| {
                let t = g.route_pay_total_x100(p, a, b, dist(a, b), false, 0);
                t + t * pct / 100
            })
            .max()
            .unwrap_or(0)
    } else {
        least_route(g, p).map_or(0, |(_, pay)| pay)
    };
    foreign > bar
}

/// The attitude steps (on `diplomacy::relation`'s scale, where 2 is a step) the route reason adds
/// toward open borders between `p` and `other`.
pub fn open_borders_bonus(g: &Game, p: PlayerId, other: PlayerId) -> i32 {
    if open_borders_reason(g, p, other) {
        2
    } else {
        0
    }
}

// ----------------------------------------------------------------------------
// What the data's route keys are worth
// ----------------------------------------------------------------------------

/// What building `info` is worth in city `c` for routes, on `production::building_value`'s scale
/// (20 a commerce): `trade_routes` (one more route from this city) and `coastal_trade_routes` (one
/// more route of the player from a coastal city) at what one of our routes pays
/// ([`route_worth_x100`]); `trade_route_modifier` as its percent of the pay of the routes from
/// this city (one such route when it has none).
pub fn building_value(g: &Game, c: CityId, info: &BuildingInfo) -> i32 {
    let p = g.cities[c].owner;
    let worth = route_worth_x100(g, p);
    let mut v = (info.trade_routes + info.coastal_trade_routes) * worth * 20 / 100;
    if info.trade_route_modifier != 0 {
        let from: i32 = g.city_routes_from(c).into_iter().map(|r| g.routes[r].owner_pay_x100()).sum();
        let base = if from > 0 { from } else { worth };
        v += info.trade_route_modifier * base / 100 * 20 / 100;
    }
    v
}

/// What `routes` more routes of player `p` (and `coastal` more from coastal cities, when it has
/// one) are worth on `empire::civic_value`'s scale (4 a commerce).
pub fn civic_value(g: &Game, p: PlayerId, routes: i32, coastal: i32) -> i32 {
    let has_coast = g.players[p].cities.iter().any(|&c| g.cities.contains(c) && g.city_is_route_coastal(c));
    let n = routes + if has_coast { coastal } else { 0 };
    n * route_worth_x100(g, p) * 4 / 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game;

    /// Player 0 with three cities on the one landmass, at peace and alone (player 1 has none).
    fn three_cities() -> (Game, PlayerId, [CityId; 3]) {
        let mut g = one_landmass_game(31);
        let p = PlayerId(0);
        let a = g.player_init_city(p, 3, 5, true, true);
        let b = g.player_init_city(p, 9, 5, true, true);
        let c = g.player_init_city(p, 16, 5, true, true);
        for &x in &[a, b, c] {
            g.city_set_population(x, 6);
        }
        (g, p, [a, b, c])
    }

    fn caravan_at(g: &mut Game, p: PlayerId, c: CityId) -> UnitId {
        let t = g.caravan_unit_type().unwrap();
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        g.player_init_unit(p, t, x, y, Some(UnitAIType::UNITAI_UNKNOWN), None)
    }

    #[test]
    fn the_best_origin_trains_a_caravan_and_only_there() {
        let ai = Ai::new();
        let (g, p, cities) = three_cities();
        let plan = plan(&ai, &g, p, &|_| true).expect("an offer");
        let t = g.caravan_unit_type().unwrap();
        assert!(repays(plan.worth, g.player_get_production_needed_unit(p, t)), "a route over 6 or 7 plots between cities of 6 repays a Caravan");
        for &c in &cities {
            let choice = production(&ai, &g, c);
            if c == plan.origin {
                assert!(matches!(choice, Some(Choice::Train(u, _)) if u == t), "the best origin trains one");
            } else {
                assert!(choice.is_none(), "another city does not");
            }
        }
    }

    #[test]
    fn no_caravan_with_one_city_or_one_already_claiming_the_free_routes() {
        let ai = Ai::new();
        let mut g = one_landmass_game(31);
        let p = PlayerId(0);
        let a = g.player_init_city(p, 3, 5, true, true);
        assert!(production(&ai, &g, a).is_none(), "one city: no route to have");
        let (mut g, p, cities) = three_cities();
        let free = g.player_route_limit(p) + g.player_coastal_route_extra(p);
        for _ in 0..free {
            caravan_at(&mut g, p, cities[0]);
        }
        assert!(cities.iter().all(|&c| production(&ai, &g, c).is_none()), "every free route claimed");
    }

    #[test]
    fn a_caravan_in_its_origin_establishes_and_elsewhere_walks_there() {
        let ai = Ai::new();
        let (mut g, p, cities) = three_cities();
        let plan = plan(&ai, &g, p, &|_| true).unwrap();
        let elsewhere = *cities.iter().find(|&&c| c != plan.origin).unwrap();
        let u = caravan_at(&mut g, p, elsewhere);
        let grp = g.units[u].group.unwrap();
        let v = View::new(&g, p);
        assert!(caravan(&ai, &mut g, &v, grp, u));
        assert!(!g.unit_in_route(u), "not in the origin: no route yet");
        let m = &g.groups[g.units[u].group.unwrap()];
        let moved = g.unit_plot(u) != g.city_plot(elsewhere) || m.mission_queue.iter().any(|q| q.mission_type == MissionType::MISSION_MOVE_TO);
        assert!(moved, "it walks to the origin");
        let w = caravan_at(&mut g, p, plan.origin);
        let grp = g.units[w].group.unwrap();
        assert!(caravan(&ai, &mut g, &v, grp, w));
        assert!(g.unit_in_route(w), "in the origin it establishes");
        let r = g.units[w].route.unwrap();
        assert_eq!((g.routes[r].origin, g.routes[r].destination), (plan.origin, plan.offer.destination));
    }

    #[test]
    fn a_route_is_replaced_by_one_worth_twice_as_much() {
        let ai = Ai::new();
        let (mut g, p, cities) = three_cities();
        let limit = g.player_route_limit(p) + g.player_coastal_route_extra(p);
        assert_eq!(limit, 2, "the test's three cities hold two routes (INITIAL_TRADE_ROUTES)");
        let mut made = Vec::new();
        for (o, d) in [(cities[0], cities[1]), (cities[1], cities[2])] {
            let u = caravan_at(&mut g, p, o);
            made.push(g.unit_establish_route(u, d, [None, None]).unwrap());
        }
        assert_eq!(g.player_routes_used(p), limit);
        // Route 0 is made to pay nothing (not arrived, length 0): the least-paying.
        let r = made[0];
        g.routes[r].length = 0;
        assert_eq!(least_route(&g, p), Some((r, 0)));
        let pl = plan(&ai, &g, p, &|_| true).expect("a replacement");
        assert_eq!(pl.replaces, Some(r));
        assert!(pl.worth >= REPLACE_FACTOR * route_pays(&g, r) && pl.worth > 0);
        // Production asks for its Caravan in its origin.
        assert!(production(&ai, &g, pl.origin).is_some());
        // Standing in the origin, the Caravan dissolves the route and establishes the new one.
        let u = caravan_at(&mut g, p, pl.origin);
        let grp = g.units[u].group.unwrap();
        let v = View::new(&g, p);
        assert!(caravan(&ai, &mut g, &v, grp, u));
        assert!(!g.routes.contains(r), "the least-paying route is dissolved");
        let nr = g.units[u].route.expect("the new route");
        assert_eq!((g.routes[nr].origin, g.routes[nr].destination), (pl.origin, pl.offer.destination));
        assert_eq!(g.player_routes_used(p), limit);
        // With a route free there is no replacement plan.
        g.route_dissolve(nr);
        assert!(plan(&ai, &g, p, &|_| true).is_none_or(|pl| pl.replaces.is_none()));
    }

    #[test]
    fn caravans_in_a_route_are_left_alone() {
        let ai = Ai::new();
        let (mut g, p, cities) = three_cities();
        let plan = plan(&ai, &g, p, &|_| true).unwrap();
        let u = caravan_at(&mut g, p, plan.origin);
        g.unit_establish_route(u, plan.offer.destination, [None, None]).unwrap();
        let grp = g.units[u].group.unwrap();
        let v = View::new(&g, p);
        assert!(!caravan(&ai, &mut g, &v, grp, u), "the caravan play skips it");
        // A whole AI turn of the player gives it no order and does not count it as a garrison.
        let before = (g.unit_plot(u), g.groups[grp].mission_queue.len());
        let mut v = View::new(&g, p);
        crate::units::play(&ai, &mut g, &mut v);
        assert!(g.units.contains(u) && g.unit_in_route(u));
        assert_eq!((g.unit_plot(u), g.groups[g.units[u].group.unwrap()].mission_queue.len()), before);
        assert_eq!(crate::production::garrison(&g, p, g.city_plot(plan.origin)).0, 0, "not a defender");
        let _ = cities;
    }
}
