//! Caravans and trade routes (docs/TRADE.md, section 8; a rules change's AI, kept small:
//! docs/AI.md, "Caravans").
//!
//! - **What an offer is worth** ([`worth`], [`best_of`]): what the route would pay us a turn, in
//!   hundredths: its owner's pay, plus the destination's share when the destination is ours, plus
//!   on a foreign route its cargo's premium ([`cargo_worth`]: each loaded caravan's half x its
//!   premium / 100). Offers whose path crosses a sea where an enemy patrols, as we see it
//!   (`sea_state_known(..).theirs > 0`, as the outposts' lanes read it), are skipped ([`offers`]).
//! - **Cargo** ([`choose_cargo`]): on a foreign route each caravan takes a luxury of which we have
//!   a spare copy (never an only copy), the highest premium first, the two different; a strategic
//!   or mana resource the same way only to our vassal or master or a team at war with a
//!   civilization we are at war with. Never a refit.
//! - **Asking for a Caravan** ([`ask`], [`turn`]): on a plan turn ([`plan_turn`]: every
//!   [`FREE_EVERY`] turns with a route free, every [`REPLACE_EVERY`] with none, staggered by player
//!   id), with at least [`MIN_CITIES`] cities, none in training, a route free that no route-less
//!   Caravan claims, and the best plan ([`plan_all`]: the best offer among the cities that can be
//!   an origin now, ties to the lower city id) repaying the Caravan within [`REPAY_TURNS`] turns
//!   (one hammer for one commerce), the Caravan is put first in the origin's queue (an empire
//!   order in a governed city); when the origin cannot train it, is unsafe (no defender, or a
//!   threat above half its defence) or holds a ritual in progress, in the nearest city of its landmass within [`STAND_IN_DIST`] plots that can.
//! - **Replacing**: with no route free and no route-less Caravan, the plan is the best offer from a
//!   city that could be the origin once our least-paying route is dissolved, when it is worth at
//!   least [`REPLACE_FACTOR`] times what that route pays ([`route_pays`]: a closed route counts what
//!   it paid until [`CLOSED_TURNS`] turns after closing, 0 from then or at once when closed by war
//!   with the destination's owner).
//! - **The Caravan with no route** ([`caravan`]): on plan turns, in the origin of the best plan it
//!   can walk to it establishes it with its cargo, dissolving the route it replaces in the same
//!   decision; elsewhere it walks there (an ordinary land move, on its landmass, only to an origin
//!   it has a unit path to); with no plan it waits in the nearest city of ours.
//! - **Caravans in a route** take no orders and are left alone by every AI loop
//!   (`units::play`, the garrison count `production::garrison`).
//! - **Raids**: an enemy caravan is a hunt target ([`plunder`] adds to its value in
//!   `military::find_targets`) and a warship's target at sea (`warships::targets`); units of hidden
//!   nationality attack the caravans of every team that is not ours, our vassal, our master or one
//!   we are Pleased with ([`hidden_target`], [`raid`]).
//! - **Open borders** ([`open_borders_why`]): one attitude step ([`OPEN_BORDERS_STEP`]) toward open
//!   borders when a route of ours to their cities would beat our alternative, or a route of theirs
//!   into our cities would pay our city at least [`INBOUND_X100`] a turn; the deal is then kept
//!   whatever the attitude (`diplomacy::keep_deal`) and offered again right after a peace.
//! - **Value** ([`route_worth_x100`]): the data keys `trade_routes`, `coastal_trade_routes` and
//!   `trade_route_modifier` are valued at what one of our routes pays now.
//!
//! The memo ([`Memo`]) keeps each city's offers and the player's plan for the turn while nothing
//! they depend on changed (our routes, the cities, our wars and open borders): it saves path
//! searches and changes no decision.

use std::collections::BTreeMap;

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::trade::{CargoClass, RouteOffer, RouteRefusal, CARAVAN_SLOTS};
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
/// A city within this many plots of the origin, on its landmass, trains the Caravan when the origin
/// cannot (a long walk alone gets the Caravan killed).
pub const STAND_IN_DIST: i32 = 5;
/// A closed route counts as paying nothing for the replace rule once closed this many turns.
pub const CLOSED_TURNS: i32 = 10;
/// With a route free, production looks for a plan once every this many turns (staggered by player
/// id): a plan costs a route search per city that can be an origin.
pub const FREE_EVERY: i32 = 5;
/// With every route used, the replace rule is looked at once every this many turns (staggered
/// by player id).
pub const REPLACE_EVERY: i32 = 10;
/// A route of theirs into one of our cities that would pay our city this much a turn (hundredths)
/// is a reason for open borders with them.
pub const INBOUND_X100: i32 = 100;
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
    /// Each player's plan over all its cities (`plan_all`), for the same key.
    plans: BTreeMap<PlayerId, (Key, Option<Plan>)>,
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
/// it is the owner's. Closed: 0 once closed [`CLOSED_TURNS`] turns or more, or at once when closed
/// by war with the destination's owner; before that what it paid open (its growth, the plot
/// distance as its length: a closed route keeps no path). Before its first caravan arrives, what it
/// will pay then.
pub fn route_pays(g: &Game, r: RouteId) -> i32 {
    let route = &g.routes[r];
    if !g.cities.contains(route.origin) || !g.cities.contains(route.destination) {
        return 0;
    }
    let domestic = g.cities[route.destination].owner == route.owner;
    let share = |total: i32| total + if domestic { total * g.d.defines.trade_dest_percent / 100 } else { 0 };
    if !route.is_open() {
        if matches!(route.closed, Some(RouteRefusal::AtWar(_))) || g.game_turn() - route.closed_since >= CLOSED_TURNS {
            return 0;
        }
        let (o, d) = (&g.cities[route.origin], &g.cities[route.destination]);
        let length = g.plot_distance(o.x, o.y, d.x, d.y);
        return share(g.route_pay_total_x100(route.owner, route.origin, route.destination, length, false, route.growth));
    }
    if route.arrived {
        return route.owner_pay_x100() + if domestic { route.dest_pay_x100 } else { 0 };
    }
    let water = route.path.iter().any(|&q| g.plot_is_water(q));
    share(g.route_pay_total_x100(route.owner, route.origin, route.destination, route.length, water, route.growth))
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

// ----------------------------------------------------------------------------
// Cargo (docs/TRADE.md, section 5)
// ----------------------------------------------------------------------------

/// A cargo the AI would load: the resource, its class and the premium it would earn (percent).
pub type Cargo = (BonusType, CargoClass, i32);

/// Whether `p` may send a strategic or mana resource to `rec`: its vassal or master, or a team at
/// war with a civilization we are at war with (the barbarians do not count).
fn strategic_ok(g: &Game, p: PlayerId, rec: PlayerId) -> bool {
    let (team, rt) = (g.players[p].team, g.players[rec].team);
    if g.teams[team].vassal[rt] || g.teams[rt].vassal[team] {
        return true;
    }
    g.team_ids().any(|t| t != team && t != rt && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t) && g.at_war(rt, t))
}

/// The cargos `p` loads on a route from `origin` to foreign `dest` (module comment): luxuries of
/// which it has a spare copy (never an only copy), the highest premium first (ties: the lower
/// resource), the two caravans different; a strategic or mana resource the same way only to a
/// recipient [`strategic_ok`] accepts. None on a domestic route.
pub fn choose_cargo(g: &Game, p: PlayerId, origin: CityId, dest: CityId) -> [Option<Cargo>; CARAVAN_SLOTS] {
    let mut out = [None; CARAVAN_SLOTS];
    if !g.cities.contains(dest) || g.cities[dest].owner == p {
        return out;
    }
    let rec = g.cities[dest].owner;
    let strategic = strategic_ok(g, p, rec);
    let mut opts: Vec<Cargo> = g
        .route_cargo_options(p, origin, dest, None)
        .into_iter()
        .filter(|o| !o.only_copy && o.free_copies > 0 && (o.class == CargoClass::Luxury || strategic))
        .map(|o| (o.bonus, o.class, o.premium_percent))
        .collect();
    opts.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    for (i, c) in opts.into_iter().take(CARAVAN_SLOTS).enumerate() {
        out[i] = Some(c);
    }
    out
}

/// What `cargo` adds to a route paying its owner `owner_pay_x100` (both halves): each loaded
/// caravan's half x its premium / 100.
pub fn cargo_worth(owner_pay_x100: i32, cargo: &[Option<Cargo>; CARAVAN_SLOTS]) -> i32 {
    let halves = [owner_pay_x100 - owner_pay_x100 / 2, owner_pay_x100 / 2];
    (0..CARAVAN_SLOTS).map(|i| cargo[i].map_or(0, |c| halves[i] * c.2 / 100)).sum()
}

/// The largest premium a cargo earns (percent): the bound [`best_of`] prunes with.
fn max_premium(g: &Game) -> i32 {
    g.d.defines.trade_cargo_new_percent.max(g.d.defines.trade_cargo_percent).max(0)
}

/// Whether `worth` repays a Caravan of `cost` hammers within [`REPAY_TURNS`] turns, counted at
/// the game's speed: a slower game trains for more hammers and lasts as many more turns, so the
/// turns scale with the speed's `train_percent` (on Marathon no route repaid a Caravan otherwise).
fn repays(g: &Game, worth: i32, cost: i32) -> bool {
    let turns = REPAY_TURNS as i64 * g.d.game_speeds[g.game_speed_type()].train_percent.max(1) as i64 / 100;
    worth as i64 * turns >= cost as i64 * 100
}

/// The best offer of city `c`, its cargo counted (ties: the lower destination id), with its worth
/// and cargo. A foreign offer's cargo is chosen only while its worth with the largest premium on
/// both halves could still beat the best so far.
fn best_of(ai: &Ai, g: &Game, c: CityId) -> Option<(RouteOffer, i32, [Option<Cargo>; CARAVAN_SLOTS])> {
    let p = g.cities[c].owner;
    let mut all = offers(ai, g, c);
    let top = max_premium(g);
    let bound = |o: &RouteOffer| worth(o) + if o.foreign { o.owner_pay_x100 * top / 100 } else { 0 };
    all.sort_by(|a, b| bound(b).cmp(&bound(a)).then(a.destination.cmp(&b.destination)));
    let mut best: Option<(RouteOffer, i32, [Option<Cargo>; CARAVAN_SLOTS])> = None;
    for o in all {
        if best.as_ref().is_some_and(|b| bound(&o) < b.1) {
            break;
        }
        let cargo = if o.foreign { choose_cargo(g, p, c, o.destination) } else { [None; CARAVAN_SLOTS] };
        let w = worth(&o) + cargo_worth(o.owner_pay_x100, &cargo);
        if best.as_ref().is_none_or(|b| w > b.1 || w == b.1 && o.destination < b.0.destination) {
            best = Some((o, w, cargo));
        }
    }
    best
}

/// A plan for a Caravan: establish `offer` from `origin`, dissolving `replaces` first.
#[derive(Clone, Debug)]
pub struct Plan {
    pub origin: CityId,
    pub offer: RouteOffer,
    /// Its worth, the cargo's premium included.
    pub worth: i32,
    pub cargo: [Option<Cargo>; CARAVAN_SLOTS],
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
    // (`keep` may search a path: it is asked after the cheap tests.)
    let mut cities: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c)).collect();
    cities.sort();
    let free = g.player_routes_used(p) < g.player_route_limit(p) + g.player_coastal_route_extra(p);
    let mut best: Option<Plan> = None;
    if free {
        for &c in &cities {
            if g.city_origin_refusal(c).is_some() || !keep(c) {
                continue;
            }
            let Some((o, w, cargo)) = best_of(ai, g, c) else { continue };
            if best.as_ref().is_none_or(|b| w > b.worth) {
                best = Some(Plan { origin: c, offer: o, worth: w, cargo, replaces: None });
            }
        }
        return best;
    }
    let (least, pays) = least_route(g, p)?;
    for &c in &cities {
        if !origin_after_dissolving(g, c, least) || !keep(c) {
            continue;
        }
        let Some((o, w, cargo)) = best_of(ai, g, c) else { continue };
        if w < REPLACE_FACTOR * pays || w <= pays {
            continue;
        }
        if best.as_ref().is_none_or(|b| w > b.worth) {
            best = Some(Plan { origin: c, offer: o, worth: w, cargo, replaces: Some(least) });
        }
    }
    best
}

/// [`plan`] over all of `p`'s cities, kept for the turn while nothing the offers depend on changed
/// (the memo's key): the cities that decide their production in a turn share it.
pub fn plan_all(ai: &Ai, g: &Game, p: PlayerId) -> Option<Plan> {
    let k = key(g, p);
    if let Some((mk, pl)) = ai.caravans.lock().unwrap().plans.get(&p) {
        if *mk == k {
            return pl.clone();
        }
    }
    let pl = plan(ai, g, p, &|_| true);
    ai.caravans.lock().unwrap().plans.insert(p, (k, pl.clone()));
    pl
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

/// Whether city `c` of `p` is safe enough to train a Caravan: a defender in it (a governed city
/// without one drops the empire order for its garrison) and the threat near it at most half its
/// garrison's defence (`production::decide`'s `safe`).
fn safe(g: &Game, p: PlayerId, c: CityId) -> bool {
    let plot = g.city_plot(c);
    let (n, defense) = crate::production::garrison(g, p, plot);
    n > 0 && crate::production::local_threat(g, p, plot, 3) * 2 <= defense
}

/// Where `p` trains a Caravan now, and for what plan (module comment), if anywhere: on a plan turn
/// ([`plan_turn`]), with at least [`MIN_CITIES`] cities, none in training, a route free that no
/// route-less Caravan claims (or, with none free and none route-less, a replacement), and a plan
/// that repays it: the origin, else (it cannot train one, is unsafe, or a governed city with a
/// ritual in progress) the nearest city of its landmass within
/// [`STAND_IN_DIST`] plots that can.
pub fn ask(ai: &Ai, g: &Game, p: PlayerId) -> Option<(CityId, Plan)> {
    if g.player_is_barbarian(p) || g.player_num_real_cities(p) < MIN_CITIES || !plan_turn(g, p) {
        return None;
    }
    let t = g.caravan_unit_type()?;
    let (alive, training) = idle_caravans(g, p);
    if training > 0 {
        return None;
    }
    let limit = g.player_route_limit(p) + g.player_coastal_route_extra(p);
    let free = limit - g.player_routes_used(p);
    if limit <= 0 || (free > 0 && alive.len() as i32 >= free) || (free <= 0 && !alive.is_empty()) {
        return None;
    }
    let plan = plan_all(ai, g, p)?;
    let cost = g.player_get_production_needed_unit(p, t);
    crate::trace!(g, p, "caravans: best origin {} to {} worth {} (cost {}, replaces {:?}, cargo {:?})", g.cities[plan.origin].name, g.cities[plan.offer.destination].name, plan.worth, cost, plan.replaces, plan.cargo);
    if !repays(g, plan.worth, cost) {
        return None;
    }
    // (A ritual in progress would lose its production behind the Caravan, or refuse the empire order.)
    let fits = |c: CityId| g.city_can_train(c, t, false, false, false, false) && g.city_ritual_in_progress(c).is_none() && safe(g, p, c);
    if fits(plan.origin) {
        return Some((plan.origin, plan));
    }
    let (o, area) = (&g.cities[plan.origin], g.map.plots[g.city_plot(plan.origin)].area);
    let mut near: Vec<(i32, CityId)> = g.players[p]
        .cities
        .iter()
        .copied()
        .filter(|&c| c != plan.origin && g.cities.contains(c) && g.map.plots[g.city_plot(c)].area == area)
        .map(|c| (g.plot_distance(o.x, o.y, g.cities[c].x, g.cities[c].y), c))
        .filter(|&(d, _)| d <= STAND_IN_DIST)
        .collect();
    near.sort();
    near.into_iter().map(|(_, c)| c).find(|&c| fits(c)).map(|c| (c, plan))
}

/// Once a turn (module comment): the Caravan [`ask`] names is put first in that city's queue (an
/// empire order in a governed city); the build it interrupts keeps its production.
pub fn turn(ai: &Ai, g: &mut Game, p: PlayerId) {
    let Some((c, plan)) = ask(ai, g, p) else { return };
    let Some(t) = g.caravan_unit_type() else { return };
    let choice = Choice::Train(t, UnitAIType::UNITAI_UNKNOWN);
    let placed = if g.city_is_governed(c) {
        crate::governor::place(g, c, choice, "caravan")
    } else {
        g.city_push_order(c, OrderType::ORDER_TRAIN, t.i() as i32, UnitAIType::UNITAI_UNKNOWN.i() as i32, false, false, false, false);
        g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_TRAIN && o.data1 == t.i() as i32)
    };
    crate::trace!(g, p, "caravans: {} at {:?} trains a Caravan ({})", g.cities[c].name, (g.cities[c].x, g.cities[c].y), if placed { "placed" } else { "refused" });
    if placed {
        g.tel_decision(p, "caravan", if plan.replaces.is_some() { "ask_replace" } else { "ask" }, if plan.offer.foreign { "foreign" } else { "domestic" }, plan.worth as i64, String::new);
    } else {
        g.tel_decision(p, "caravan", "ask_refused", if g.city_is_governed(c) { "governed" } else { "own" }, 0, String::new);
    }
}

// ----------------------------------------------------------------------------
// The Caravan with no route
// ----------------------------------------------------------------------------

/// Whether this is a turn on which player `p` looks for a plan: once every [`FREE_EVERY`] turns
/// with a route free, every [`REPLACE_EVERY`] with none (staggered by player id).
pub fn plan_turn(g: &Game, p: PlayerId) -> bool {
    let free = g.player_routes_used(p) < g.player_route_limit(p) + g.player_coastal_route_extra(p);
    (g.game_turn() + p.0 as i32) % if free { FREE_EVERY } else { REPLACE_EVERY } == 0
}

/// Whether `u` is a Caravan of ours with no route whose orders are re-planned this turn (on its
/// owner's plan turns: [`plan_turn`]; between them it keeps walking where it was sent).
pub fn replans(g: &Game, u: UnitId) -> bool {
    g.unit_is_caravan(u) && !g.unit_in_route(u) && plan_turn(g, g.units[u].owner)
}

/// A Caravan of ours with no route (module comment). False if `u` is none; true: its turn is
/// decided.
pub fn caravan(ai: &Ai, g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || !g.units.contains(u) || !g.unit_is_caravan(u) || g.unit_in_route(u) {
        return false;
    }
    let p = v.p;
    let here = g.unit_plot(u);
    // Between plan turns it waits (one that walks goes on where it was sent: `replans`).
    if !plan_turn(g, p) {
        skip(g, grp);
        return true;
    }
    let area = g.map.plots[here].area;
    let on_land = |c: CityId| g.map.plots[g.city_plot(c)].area == area;
    // (An origin the Caravan can walk to: the route's path may cross land the unit may not enter.)
    let reachable = |c: CityId| on_land(c) && (g.city_plot(c) == here || g.group_find_path(grp, here, g.city_plot(c), 0).is_some());
    let cost = g.caravan_unit_type().map_or(0, |t| g.player_get_production_needed_unit(p, t));
    // (The plan over all cities, when its origin is one it can walk to, is the plan over those.)
    let plan = match plan_all(ai, g, p) {
        Some(pl) if reachable(pl.origin) => Some(pl),
        Some(_) => plan(ai, g, p, &reachable),
        None => None,
    }
    .filter(|pl| repays(g, pl.worth, cost));
    let Some(plan) = plan else {
        crate::trace!(g, p, "caravan {:?} at {:?} waits: no plan it repays and walks to", u, g.map.xy(here));
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
        crate::trace!(g, p, "caravan {:?} at {:?} walks to {} at {:?} for {}", u, g.map.xy(here), g.cities[plan.origin].name, g.map.xy(origin_plot), g.cities[plan.offer.destination].name);
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
    // The cargo chosen now (the copies may have changed since the plan); refused, none.
    let cargo = choose_cargo(g, p, plan.origin, plan.offer.destination);
    let bonuses = [cargo[0].map(|c| c.0), cargo[1].map(|c| c.0)];
    let mut done = g.unit_establish_route(u, plan.offer.destination, bonuses);
    let loaded = done.is_ok();
    if matches!(done, Err(RouteRefusal::CargoDomestic | RouteRefusal::CargoNoCopy(_) | RouteRefusal::CargoSame)) {
        done = g.unit_establish_route(u, plan.offer.destination, [None, None]);
    }
    match done {
        Ok(_) => {
            crate::trace!(g, p, "caravan {:?} establishes {} to {} (worth {}, cargo {:?})", u, g.cities[plan.origin].name, g.cities[plan.offer.destination].name, plan.worth, bonuses);
            g.tel_decision(p, "caravan", "establish", if foreign { "foreign" } else { "domestic" }, plan.worth as i64, String::new);
            if loaded {
                for c in cargo.iter().flatten() {
                    g.tel_decision(p, "caravan", "cargo", cargo_key(g, c), c.2 as i64, String::new);
                }
            }
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

/// The telemetry key of a cargo loaded: its class and premium level (`new`: the premium for a
/// resource the recipient lacks; `cargo`: the ordinary one).
fn cargo_key(g: &Game, c: &Cargo) -> &'static str {
    let new = c.2 >= g.d.defines.trade_cargo_new_percent;
    match (c.1, new) {
        (CargoClass::Luxury, true) => "luxury_new",
        (CargoClass::Luxury, false) => "luxury_cargo",
        (CargoClass::Mana, true) => "mana_new",
        (CargoClass::Mana, false) => "mana_cargo",
        (CargoClass::Strategic, true) => "strategic_new",
        (CargoClass::Strategic, false) => "strategic_cargo",
    }
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

/// Why open borders between `p` and `other` would bring `p` routes (module comment), as `p`
/// decides it, `None` when they would not: `"own"`, a route of ours to one of their cities would
/// pay us more than our best domestic pair (with a route free) or our least-paying route (with
/// none); `"inbound"`, a route of theirs into one of our cities would pay our city at least
/// [`INBOUND_X100`] a turn (the destination's share: commerce for nothing). Estimated by plot
/// distance as the length, over land, at growth 0 (`Game::route_pay_total_x100`; no path search).
/// Never with the team we prepare a war on or our worst enemy.
pub fn open_borders_why(g: &Game, p: PlayerId, other: PlayerId) -> Option<&'static str> {
    let (team, ot) = (g.players[p].team, g.players[other].team);
    if team == ot || g.player_is_barbarian(p) || g.player_is_barbarian(other) || crate::diplomacy::preparing_target(g, team) == Some(ot) || g.teams[team].ai_worst_enemy == Some(ot) {
        return None;
    }
    let cities_of = |q: PlayerId, seen_by: TeamId| -> Vec<CityId> { g.players[q].cities.iter().copied().filter(|&c| g.cities.contains(c) && !g.cities[c].outpost && (g.players[q].team == seen_by || g.city_is_revealed(c, seen_by, false))).collect() };
    let ours = cities_of(p, team);
    let theirs = cities_of(other, team);
    let dist = |a: CityId, b: CityId| g.plot_distance(g.cities[a].x, g.cities[a].y, g.cities[b].x, g.cities[b].y);
    let pct = g.d.defines.trade_dest_percent;
    if own_reason(g, p, &ours, &theirs, &dist) {
        return Some("own");
    }
    // Theirs into ours: what our city would get, by the best pair of their routes' limits.
    if g.player_route_limit(other) + g.player_coastal_route_extra(other) > 0 {
        let inbound = theirs.iter().flat_map(|&a| ours.iter().map(move |&b| (a, b))).map(|(a, b)| g.route_pay_total_x100(other, a, b, dist(a, b), false, 0) * pct / 100).max().unwrap_or(0);
        if inbound >= INBOUND_X100 {
            return Some("inbound");
        }
    }
    None
}

/// The "own" side of [`open_borders_why`].
fn own_reason(g: &Game, p: PlayerId, ours: &[CityId], theirs: &[CityId], dist: &dyn Fn(CityId, CityId) -> i32) -> bool {
    let limit = g.player_route_limit(p) + g.player_coastal_route_extra(p);
    if limit <= 0 {
        return false;
    }
    let foreign = ours.iter().flat_map(|&a| theirs.iter().map(move |&b| (a, b))).map(|(a, b)| g.route_pay_total_x100(p, a, b, dist(a, b), false, 0)).max().unwrap_or(0);
    if foreign <= 0 {
        return false;
    }
    let bar = if g.player_routes_used(p) < limit {
        let pct = g.d.defines.trade_dest_percent;
        let routes = g.player_routes(p);
        let linked = |a: CityId, b: CityId| routes.iter().any(|&r| (g.routes[r].origin, g.routes[r].destination) == (a, b) || (g.routes[r].origin, g.routes[r].destination) == (b, a));
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

/// Whether [`open_borders_why`] gives a reason.
pub fn open_borders_reason(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    open_borders_why(g, p, other).is_some()
}

/// The attitude step a trade-route reason is worth toward open borders, on `diplomacy::relation`'s
/// scale (where 2 is a step).
pub const OPEN_BORDERS_STEP: i32 = 2;

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
        let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
        for &x in &[a, b, c] {
            g.city_set_population(x, 6);
            let (cx, cy) = (g.cities[x].x, g.cities[x].y);
            g.player_init_unit(p, warrior, cx, cy, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
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
        assert!(repays(&g, plan.worth, g.player_get_production_needed_unit(p, t)), "a route over 6 or 7 plots between cities of 6 repays a Caravan");
        assert_eq!(ask(&ai, &g, p).map(|(c, pl)| (c, pl.origin)), Some((plan.origin, plan.origin)), "the best origin trains one");
        // The turn step puts it first in the origin's queue, and asks no second one.
        let mut g = g;
        turn(&ai, &mut g, p);
        let head = g.cities[plan.origin].order_queue.first().cloned().expect("an order");
        assert_eq!((head.order_type, head.data1), (OrderType::ORDER_TRAIN, t.i() as i32));
        assert!(ask(&ai, &g, p).is_none(), "one in training at a time");
        let _ = cities;
    }

    #[test]
    fn no_caravan_with_one_city_or_one_already_claiming_the_free_routes() {
        let ai = Ai::new();
        let mut g = one_landmass_game(31);
        let p = PlayerId(0);
        let a = g.player_init_city(p, 3, 5, true, true);
        let _ = a;
        assert!(ask(&ai, &g, p).is_none(), "one city: no route to have");
        let (mut g, p, cities) = three_cities();
        let free = g.player_route_limit(p) + g.player_coastal_route_extra(p);
        for _ in 0..free {
            caravan_at(&mut g, p, cities[0]);
        }
        assert!(ask(&ai, &g, p).is_none(), "every free route claimed");
        let _ = cities;
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
        assert_eq!(ask(&ai, &g, p).map(|x| x.0), Some(pl.origin));
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
        assert_eq!(crate::production::garrison(&g, p, g.city_plot(plan.origin)).0, 1, "the warrior defends, not the caravan");
        let _ = cities;
    }

    /// Player 0's two cities and player 1's one, open borders between them.
    fn foreign_world() -> (Game, PlayerId, PlayerId, [CityId; 3]) {
        let mut g = one_landmass_game(33);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let a = g.player_init_city(p0, 3, 5, true, true);
        let b = g.player_init_city(p0, 6, 5, true, true);
        let f = g.player_init_city(p1, 16, 5, true, true);
        for &c in &[a, b, f] {
            g.city_set_population(c, 6);
        }
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_set_open_borders(t0, t1, true);
        g.team_set_open_borders(t1, t0, true);
        for &c in &[a, b, f] {
            g.cities[c].revealed[t0] = true;
            g.cities[c].revealed[t1] = true;
        }
        g.trade_refresh_pay();
        (g, p0, p1, [a, b, f])
    }

    #[test]
    fn cargo_is_spare_luxuries_the_highest_premium_first_and_strategic_only_to_a_friend() {
        let (mut g, p0, p1, [a, _b, f]) = foreign_world();
        let (gems, silk) = (g.d.t::<BonusType>("BONUS_GEMS"), g.d.t::<BonusType>("BONUS_SILK"));
        let strat = g.d.bonuses.keys().find(|&b| g.bonus_cargo_class(b) == CargoClass::Strategic).expect("a strategic resource");
        assert_eq!(choose_cargo(&g, p0, a, f), [None, None], "no copies: nothing");
        // One copy of gems is the only copy: never loaded.
        g.city_change_free_bonus(a, gems, 1);
        g.trade_refresh_pay();
        assert_eq!(choose_cargo(&g, p0, a, f), [None, None], "an only copy stays home");
        // Spare gems and silk: both, different; a spare strategic resource: not to a stranger.
        g.city_change_free_bonus(a, gems, 1);
        g.city_change_free_bonus(a, silk, 2);
        g.city_change_free_bonus(a, strat, 2);
        g.trade_refresh_pay();
        let c = choose_cargo(&g, p0, a, f);
        let picked: Vec<BonusType> = c.iter().flatten().map(|x| x.0).collect();
        assert_eq!(picked.len(), 2);
        assert!(picked.contains(&gems) && picked.contains(&silk), "{picked:?}");
        assert!(c[0].unwrap().2 >= c[1].unwrap().2, "the higher premium first");
        // No luxury spare, the recipient our vassal: the strategic resource goes.
        g.city_change_free_bonus(a, gems, -2);
        g.city_change_free_bonus(a, silk, -2);
        g.trade_refresh_pay();
        assert_eq!(choose_cargo(&g, p0, a, f), [None, None], "strategic: not to a stranger");
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.teams[t1].vassal[t0] = true;
        assert_eq!(choose_cargo(&g, p0, a, f)[0].map(|x| x.0), Some(strat), "strategic to our vassal");
        // Domestic: never.
        assert_eq!(choose_cargo(&g, p0, a, _b), [None, None]);
    }

    #[test]
    fn the_cargo_premium_counts_in_an_offers_worth() {
        let lux = |b: usize, pct: i32| Some((BonusType::from_usize(b), CargoClass::Luxury, pct));
        // Halves 501 and 500: 501 x 150% + 500 x 50%.
        assert_eq!(cargo_worth(1001, &[lux(0, 150), lux(1, 50)]), 501 * 150 / 100 + 500 * 50 / 100);
        assert_eq!(cargo_worth(1001, &[None, lux(1, 50)]), 250);
        // The plan's worth of a foreign offer with spare luxuries is above the bare offer's.
        let (mut g, p0, _p1, [a, _b, f]) = foreign_world();
        let ai = Ai::new();
        let bare = best_of(&ai, &g, a).unwrap();
        g.city_change_free_bonus(a, g.d.t::<BonusType>("BONUS_GEMS"), 2);
        g.trade_refresh_pay();
        let ai = Ai::new();
        let with = best_of(&ai, &g, a).unwrap();
        if with.0.destination == f {
            assert!(with.1 > worth(&with.0), "cargo adds to the worth");
            assert!(with.2[0].is_some());
        }
        assert!(with.1 >= bare.1);
        let _ = p0;
    }

    #[test]
    fn a_closed_route_counts_its_pay_until_ten_turns_or_a_war() {
        let (mut g, p, cities) = three_cities();
        let u = caravan_at(&mut g, p, cities[0]);
        let r = g.unit_establish_route(u, cities[1], [None, None]).unwrap();
        // Closed by a lost path this turn: what it paid open (estimated), not 0.
        let turn = g.game_turn();
        g.routes[r].closed = Some(RouteRefusal::NoPath);
        g.routes[r].closed_since = turn;
        assert!(route_pays(&g, r) > 0, "closed this turn: counted as paying");
        g.routes[r].closed_since = turn - (CLOSED_TURNS - 1);
        assert!(route_pays(&g, r) > 0);
        g.routes[r].closed_since = turn - CLOSED_TURNS;
        assert_eq!(route_pays(&g, r), 0, "closed ten turns: nothing");
        // Closed by war with the destination's owner: nothing at once.
        g.routes[r].closed = Some(RouteRefusal::AtWar(PlayerId(1)));
        g.routes[r].closed_since = turn;
        assert_eq!(route_pays(&g, r), 0, "war: nothing at once");
    }

    #[test]
    fn the_open_borders_reason_has_two_sides() {
        let (mut g, p0, p1, [_a, _b, f]) = foreign_world();
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_set_open_borders(t0, t1, false);
        g.team_set_open_borders(t1, t0, false);
        // Our two cities 3 plots apart, theirs 10 to 13 away: our own route reason.
        assert_eq!(open_borders_why(&g, p0, p1), Some("own"));
        // With no routes of our own, their route into our cities is the reason when it pays our
        // city enough.
        g.players[p0].trade_routes = -g.player_route_limit(p0) - 10;
        assert_eq!(g.player_route_limit(p0), 0);
        let inbound = g.route_pay_total_x100(p1, f, _a, g.plot_distance(16, 5, 3, 5), false, 0) * g.d.defines.trade_dest_percent / 100;
        assert_eq!(open_borders_why(&g, p0, p1), (inbound >= INBOUND_X100).then_some("inbound"));
        // Small cities: their route would pay our city too little.
        for c in g.players[p0].cities.clone().into_iter().chain([f]) {
            g.city_set_population(c, 1);
        }
        let small = g.route_pay_total_x100(p1, f, _b, g.plot_distance(16, 5, 6, 5), false, 0) * g.d.defines.trade_dest_percent / 100;
        assert!(small < INBOUND_X100, "the test's small cities pay {small}");
        assert_eq!(open_borders_why(&g, p0, p1), None);
        // Never with our worst enemy.
        g.city_set_population(f, 12);
        g.teams[t0].ai_worst_enemy = Some(t1);
        assert_eq!(open_borders_why(&g, p0, p1), None);
    }

}
