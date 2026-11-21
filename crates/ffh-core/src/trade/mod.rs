//! Trade (this game's own rules, not FFH2's; docs/TRADE.md): routes, caravans, pay.
//!
//! **Routes** (`Game::routes`, [`Route`]): a standing link from an origin city of its owner to a
//! destination city, carried by two caravans. A player has `Player::trade_routes` plus the game's
//! `trade_routes` routes ([`Game::player_route_limit`]), plus `coastal_trade_routes` that only a
//! coastal city can be the origin of; a city is the origin of 1 plus its `extra_trade_routes`
//! ([`Game::city_origin_limit`]). A trained Caravan in a city of its owner establishes one
//! ([`Game::unit_establish_route`]) to a destination that passes
//! [`Game::route_destination_refusal`]; it then travels by itself and the route pays from its
//! arrival, when the second caravan forms in the origin.
//!
//! **Two computations.** The path and the open/closed check ([`Game::route_path`],
//! [`Game::player_update_routes`]) run at the start of the owner's turn and when war is declared
//! ([`Game::routes_on_war`]). The pay ([`Game::trade_refresh_pay`]) is a pure recomputation from
//! the stored route state, run whenever something it reads changes; a city's `trade_yield` is the
//! hundredths of its routes rounded to the nearest whole, through `city_calculate_trade_yield`.
//!
//! **Caravans** move at the end of their owner's turn ([`Game::player_move_caravans`]) by the
//! rules' one-step move (`group_move`), never by movement points, and take no order. A caravan
//! killed by anything but these rules ([`Game::route_caravan_lost`], hooked from `unit_kill`)
//! gives plunder to its killer, halves the route's growth and re-forms after
//! `CARAVAN_REFORM_TURNS` turns of its owner.

use std::sync::OnceLock;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

mod caravans;
mod pay;
mod path;

/// The caravans a route has.
pub const CARAVAN_SLOTS: usize = 2;

/// Which of a route's cities a caravan is bound for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leg {
    /// Bound for the destination.
    Out,
    /// Bound for the origin.
    Back,
}

/// One of a route's two caravans (docs/TRADE.md, 4.1-4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaravanSlot {
    /// The caravan unit while it lives; `None` while it waits to form.
    pub unit: Option<UnitId>,
    /// Turns of its owner until it forms again after it was killed (0: it forms when the route
    /// is open; for the second caravan, also once the route has arrived).
    pub reform_turns: i32,
    /// The city it is bound for.
    pub bound: Leg,
    /// Its cargo (docs/TRADE.md, section 5; not built: always `None`).
    pub cargo: Option<BonusType>,
    /// The turn it was last killed (-1: never), and by whom (`None`: no killer).
    pub lost_turn: i32,
    pub lost_to: Option<PlayerId>,
    /// What it pays its owner a turn, in hundredths, as last computed (0 while dead or the route
    /// pays nothing).
    pub pay_x100: i32,
}

impl CaravanSlot {
    fn empty() -> CaravanSlot {
        CaravanSlot { unit: None, reform_turns: 0, bound: Leg::Out, cargo: None, lost_turn: -1, lost_to: None, pay_x100: 0 }
    }
}

/// A trade route (docs/TRADE.md, section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub id: RouteId,
    pub owner: PlayerId,
    pub origin: CityId,
    pub destination: CityId,
    /// The game turn it was established.
    pub established: i32,
    /// Its first caravan has reached the destination: it pays from then.
    pub arrived: bool,
    /// Growth (6.2), in percent.
    pub growth: i32,
    pub slots: [CaravanSlot; CARAVAN_SLOTS],
    /// Closed, and why (`None`: open).
    pub closed: Option<RouteRefusal>,
    /// The path traced at the last check: the origin's plot first, the destination's last
    /// (empty while there is none).
    pub path: Vec<PlotIdx>,
    /// The path's plots, the origin's left out.
    pub length: i32,
    /// What the destination city gets a turn, in hundredths, as last computed.
    pub dest_pay_x100: i32,
}

impl Route {
    /// Whether it is open.
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }

    /// Its living caravans.
    pub fn caravans(&self) -> Vec<UnitId> {
        self.slots.iter().filter_map(|s| s.unit).collect()
    }

    /// What its owner gets a turn, in hundredths, as last computed.
    pub fn owner_pay_x100(&self) -> i32 {
        self.slots.iter().map(|s| s.pay_x100).sum()
    }
}

/// Why a route cannot be established, or why it is closed (docs/TRADE.md, 3.2-3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteRefusal {
    /// The unit is not a Caravan.
    NotACaravan,
    /// The unit already carries a route.
    InRoute,
    /// The unit does not stand in a city of its owner.
    NotInOwnCity,
    /// The origin is an outpost.
    OriginOutpost,
    /// Every route of the player is used.
    NoRouteFree,
    /// The routes left can only start in a coastal city.
    CoastalOnly,
    /// The city is the origin of as many routes as it can be.
    OriginFull,
    /// The destination is the origin.
    SameCity,
    /// The destination no longer exists.
    DestinationGone,
    /// The owner has not revealed the destination.
    NotRevealed,
    /// The destination is an outpost.
    DestinationOutpost,
    /// At war with the destination's owner.
    AtWar(PlayerId),
    /// No open borders with the destination's owner.
    NoOpenBorders(PlayerId),
    /// A civic of one side forbids foreign trade (the other player named).
    NoForeignTrade(PlayerId),
    /// No path (3.5).
    NoPath,
    /// A route of the owner already links the two cities.
    AlreadyLinked,
    /// A cargo was given: cargo is not built yet (docs/TRADE.md, section 5).
    CargoNotBuilt,
}

impl RouteRefusal {
    /// The bare reason (docs/TRADE.md, section 9).
    pub fn text(self, g: &Game) -> String {
        let civ = |p: PlayerId| g.player_get_civilization_short_description(p);
        match self {
            RouteRefusal::NotACaravan => "Only a Caravan establishes a route.".to_string(),
            RouteRefusal::InRoute => "This caravan already carries a route.".to_string(),
            RouteRefusal::NotInOwnCity => "A route starts in one of your cities.".to_string(),
            RouteRefusal::OriginOutpost => "An outpost is not the origin of a route.".to_string(),
            RouteRefusal::NoRouteFree => "No route free.".to_string(),
            RouteRefusal::CoastalOnly => "The routes left start only in a coastal city.".to_string(),
            RouteRefusal::OriginFull => "This city is the origin of as many routes as it can be.".to_string(),
            RouteRefusal::SameCity => "A route links two cities.".to_string(),
            RouteRefusal::DestinationGone => "The destination no longer exists.".to_string(),
            RouteRefusal::NotRevealed => "The destination is not revealed.".to_string(),
            RouteRefusal::DestinationOutpost => "An outpost is not the destination of a route.".to_string(),
            RouteRefusal::AtWar(p) => format!("At war with {}.", civ(p)),
            RouteRefusal::NoOpenBorders(p) => format!("No open borders with {}.", civ(p)),
            RouteRefusal::NoForeignTrade(p) => format!("No foreign trade with {}.", civ(p)),
            RouteRefusal::NoPath => "No path.".to_string(),
            RouteRefusal::AlreadyLinked => "A route already links these cities.".to_string(),
            RouteRefusal::CargoNotBuilt => "Cargo is not built yet.".to_string(),
        }
    }

    /// A short key for telemetry.
    pub fn key(self) -> &'static str {
        match self {
            RouteRefusal::NotACaravan => "not_a_caravan",
            RouteRefusal::InRoute => "in_route",
            RouteRefusal::NotInOwnCity => "not_in_own_city",
            RouteRefusal::OriginOutpost => "origin_outpost",
            RouteRefusal::NoRouteFree => "no_route_free",
            RouteRefusal::CoastalOnly => "coastal_only",
            RouteRefusal::OriginFull => "origin_full",
            RouteRefusal::SameCity => "same_city",
            RouteRefusal::DestinationGone => "destination_gone",
            RouteRefusal::NotRevealed => "not_revealed",
            RouteRefusal::DestinationOutpost => "destination_outpost",
            RouteRefusal::AtWar(_) => "war",
            RouteRefusal::NoOpenBorders(_) => "no_open_borders",
            RouteRefusal::NoForeignTrade(_) => "no_foreign_trade",
            RouteRefusal::NoPath => "no_path",
            RouteRefusal::AlreadyLinked => "already_linked",
            RouteRefusal::CargoNotBuilt => "cargo_not_built",
        }
    }
}

/// A destination a route from a city could have (docs/TRADE.md, 3.3 and 9; [`Game::route_offers`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteOffer {
    pub destination: CityId,
    /// The path's plots, the origin's left out.
    pub length: i32,
    /// The path, the origin's plot first.
    pub path: Vec<PlotIdx>,
    /// What the route would pay its owner a turn, in hundredths, at growth 0 with two caravans.
    pub owner_pay_x100: i32,
    /// What it would pay the destination a turn, in hundredths, the same way.
    pub dest_pay_x100: i32,
    /// The destination's owner is not the route's owner.
    pub foreign: bool,
    /// The named seas its path crosses, in the order it enters them.
    pub seas: Vec<SeaId>,
}

/// The pay of a route (6.1), in hundredths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoutePay {
    /// Each caravan's pay to the owner (0 for a dead one).
    pub caravan_x100: [i32; CARAVAN_SLOTS],
    /// The destination's share.
    pub dest_x100: i32,
}

/// The data types the rules name, looked up once.
pub(crate) struct TradeTypes {
    pub(crate) caravan: Option<UnitType>,
    pub(crate) guards: [Option<PromotionType>; 3],
}

pub(crate) fn types(d: &GameData) -> &'static TradeTypes {
    static T: OnceLock<TradeTypes> = OnceLock::new();
    T.get_or_init(|| TradeTypes {
        caravan: d.ty::<UnitType>("UNIT_CARAVAN"),
        guards: [d.ty::<PromotionType>("PROMOTION_CARAVAN_GUARD_2"), d.ty::<PromotionType>("PROMOTION_CARAVAN_GUARD_3"), d.ty::<PromotionType>("PROMOTION_CARAVAN_GUARD_4")],
    })
}

/// Rounds hundredths to the nearest whole (half up; a negative sum rounds toward 0 as well).
pub fn round_x100(x100: i32) -> i32 {
    if x100 >= 0 {
        (x100 + 50) / 100
    } else {
        -((-x100 + 50) / 100)
    }
}

impl Game {
    // ---------------------------------------------------------------------------------------
    // What a caravan is

    /// `UNIT_CARAVAN`, if the data has it.
    pub fn caravan_unit_type(&self) -> Option<UnitType> {
        types(self.d).caravan
    }

    /// Whether unit `u` is a Caravan (in a route or not).
    pub fn unit_is_caravan(&self, u: UnitId) -> bool {
        self.units.contains(u) && Some(self.units[u].unit_type) == types(self.d).caravan
    }

    /// Whether unit `u` is a caravan of a route (it takes no order; docs/TRADE.md, 4.1).
    pub fn unit_in_route(&self, u: UnitId) -> bool {
        self.units.contains(u) && self.units[u].route.is_some()
    }

    /// Whether group `g` holds a caravan of a route.
    pub fn group_has_route_caravan(&self, g: GroupId) -> bool {
        self.groups.contains(g) && self.groups[g].units.iter().any(|&u| self.unit_in_route(u))
    }

    // ---------------------------------------------------------------------------------------
    // Limits (3.1)

    /// The routes player `p` may have from any city: `Player::trade_routes` plus the game's
    /// `trade_routes` (at least 0).
    pub fn player_route_limit(&self, p: PlayerId) -> i32 {
        (self.players[p].trade_routes + self.trade_routes).max(0)
    }

    /// The routes player `p` may have more from a coastal city: `coastal_trade_routes`.
    pub fn player_coastal_route_extra(&self, p: PlayerId) -> i32 {
        self.players[p].coastal_trade_routes.max(0)
    }

    /// The routes player `p` owns.
    pub fn player_routes_used(&self, p: PlayerId) -> i32 {
        self.routes.iter().filter(|(_, r)| r.owner == p).count() as i32
    }

    /// The routes of player `p` from a city that is not coastal.
    pub(crate) fn player_inland_routes_used(&self, p: PlayerId) -> i32 {
        self.routes.iter().filter(|(_, r)| r.owner == p && !(self.cities.contains(r.origin) && self.city_is_route_coastal(r.origin))).count() as i32
    }

    /// Whether city `c` is coastal for `coastal_trade_routes` (as Civ4's trade routes counted it).
    pub fn city_is_route_coastal(&self, c: CityId) -> bool {
        self.city_is_coastal(c, self.d.defines.min_water_size_for_ocean)
    }

    /// The routes city `c` may be the origin of: 1 plus its `extra_trade_routes` (at least 0).
    pub fn city_origin_limit(&self, c: CityId) -> i32 {
        (1 + self.cities[c].extra_trade_routes).max(0)
    }

    /// The routes city `c` is the origin of.
    pub fn city_origin_used(&self, c: CityId) -> i32 {
        self.routes.iter().filter(|(_, r)| r.origin == c).count() as i32
    }

    /// The routes from city `c`, in id order.
    pub fn city_routes_from(&self, c: CityId) -> Vec<RouteId> {
        self.routes.iter().filter(|(_, r)| r.origin == c).map(|(id, _)| id).collect()
    }

    /// The routes into city `c` (it is their destination), in id order.
    pub fn city_routes_into(&self, c: CityId) -> Vec<RouteId> {
        self.routes.iter().filter(|(_, r)| r.destination == c).map(|(id, _)| id).collect()
    }

    /// The routes of player `p`, in id order.
    pub fn player_routes(&self, p: PlayerId) -> Vec<RouteId> {
        self.routes.iter().filter(|(_, r)| r.owner == p).map(|(id, _)| id).collect()
    }

    /// Why city `c` cannot be the origin of one more route of its owner now, `None` when it can.
    pub fn city_origin_refusal(&self, c: CityId) -> Option<RouteRefusal> {
        let p = self.cities[c].owner;
        if self.cities[c].outpost {
            return Some(RouteRefusal::OriginOutpost);
        }
        if self.city_origin_used(c) >= self.city_origin_limit(c) {
            return Some(RouteRefusal::OriginFull);
        }
        let (base, extra) = (self.player_route_limit(p), self.player_coastal_route_extra(p));
        if self.player_routes_used(p) >= base + extra {
            return Some(RouteRefusal::NoRouteFree);
        }
        if !self.city_is_route_coastal(c) && self.player_inland_routes_used(p) >= base {
            return Some(RouteRefusal::CoastalOnly);
        }
        None
    }

    // ---------------------------------------------------------------------------------------
    // Establishing (3.2)

    /// Whether unit `u` can take the order Establish route now (to some destination): a Caravan
    /// in no route, in a city of its owner that can be the origin of one more route.
    pub fn unit_can_establish_route(&self, u: UnitId) -> bool {
        self.unit_establish_origin_refusal(u).is_none()
    }

    /// Why unit `u` cannot establish a route from where it stands (the destination aside).
    pub fn unit_establish_origin_refusal(&self, u: UnitId) -> Option<RouteRefusal> {
        if !self.unit_is_caravan(u) {
            return Some(RouteRefusal::NotACaravan);
        }
        if self.units[u].route.is_some() {
            return Some(RouteRefusal::InRoute);
        }
        let owner = self.units[u].owner;
        let Some(plot) = self.unit_plot_opt(u) else { return Some(RouteRefusal::NotInOwnCity) };
        let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == owner) else {
            return Some(RouteRefusal::NotInOwnCity);
        };
        self.city_origin_refusal(c)
    }

    /// Why unit `u` cannot establish a route to `dest` with `cargo`, `None` when it can.
    pub fn unit_establish_refusal(&self, u: UnitId, dest: CityId, cargo: [Option<BonusType>; CARAVAN_SLOTS]) -> Option<RouteRefusal> {
        if let Some(r) = self.unit_establish_origin_refusal(u) {
            return Some(r);
        }
        let owner = self.units[u].owner;
        let origin = self.map.plots[self.unit_plot(u)].plot_city.expect("in a city");
        if let Some(r) = self.route_destination_refusal(owner, origin, dest) {
            return Some(r);
        }
        if cargo.iter().any(|c| c.is_some()) {
            return Some(RouteRefusal::CargoNotBuilt);
        }
        None
    }

    /// Caravan `u` establishes a route from the city it stands in to `dest` (docs/TRADE.md, 3.2):
    /// it becomes the route's first caravan, bound for the destination; it sets out at the end
    /// of its owner's turn. `cargo` must be `[None, None]` (cargo is not built).
    pub fn unit_establish_route(&mut self, u: UnitId, dest: CityId, cargo: [Option<BonusType>; CARAVAN_SLOTS]) -> Result<RouteId, RouteRefusal> {
        if let Some(why) = self.unit_establish_refusal(u, dest, cargo) {
            return Err(why);
        }
        let owner = self.units[u].owner;
        let origin = self.map.plots[self.unit_plot(u)].plot_city.expect("in a city");
        let path = self.route_path(owner, origin, dest).expect("a path");
        let turn = self.game_turn();
        let length = path.len() as i32 - 1;
        let r = self.routes.insert_with(|id| Route {
            id,
            owner,
            origin,
            destination: dest,
            established: turn,
            arrived: false,
            growth: 0,
            slots: [CaravanSlot::empty(), CaravanSlot::empty()],
            closed: None,
            path,
            length,
            dest_pay_x100: 0,
        });
        self.routes[r].slots[0].cargo = cargo[0];
        self.routes[r].slots[1].cargo = cargo[1];
        self.caravan_join_route(u, r, 0);
        let foreign = self.cities[dest].owner != owner;
        let (on, dn) = (self.cities[origin].name.clone(), self.cities[dest].name.clone());
        let at = (self.cities[origin].x, self.cities[origin].y);
        self.log.msg_at(turn, Some(owner), at, || format!("Route established: {on} to {dn}. {length} plots."));
        self.tel_trade(owner, "trade_route", if foreign { "foreign" } else { "domestic" }, 1);
        self.trade_refresh_pay();
        Ok(r)
    }

    /// Unit `u` becomes caravan `slot` of route `r`: alone in its group, asleep, no orders, bound
    /// for the destination, its guard set (4.2).
    pub(crate) fn caravan_join_route(&mut self, u: UnitId, r: RouteId, slot: usize) {
        // Off any ship it was loaded on (it is never cargo in a route).
        if self.units[u].transport_unit.is_some() {
            self.unit_set_transport_unit(u, None);
        }
        self.units[u].route = Some(r);
        let s = &mut self.routes[r].slots[slot];
        s.unit = Some(u);
        s.reform_turns = 0;
        s.bound = Leg::Out;
        self.unit_join_group(u, None, true);
        if let Some(g) = self.units[u].group {
            self.group_clear_mission_queue(g);
            self.group_set_activity_type(g, ActivityType::ACTIVITY_SLEEP);
        }
        self.caravan_set_guard(r, u);
    }

    /// The guard tier of route `r` (4.2): the highest tier of an army unit its origin can train
    /// (1 when none).
    pub fn route_guard_tier(&self, r: RouteId) -> i32 {
        let origin = self.routes[r].origin;
        if !self.cities.contains(origin) {
            return 1;
        }
        let d = self.d;
        let mut best = 1;
        for ty in d.units.keys() {
            let tier = d.units[ty].tier;
            if tier > best && self.unit_type_is_army_unit(ty) && self.city_can_train(origin, ty, false, false, false, false) {
                best = tier;
            }
        }
        best
    }

    /// Caravan `u` of route `r` holds `PROMOTION_CARAVAN_GUARD_2` to `_4` up to the route's guard
    /// tier, and no higher one.
    pub(crate) fn caravan_set_guard(&mut self, r: RouteId, u: UnitId) {
        let tier = self.route_guard_tier(r);
        let guards = types(self.d).guards;
        for (i, g) in guards.iter().enumerate() {
            let Some(p) = *g else { continue };
            let want = tier >= i as i32 + 2;
            if self.units[u].has_promotion[p] != want {
                self.unit_set_has_promotion(u, p, want);
            }
        }
    }

    /// A caravan of route `r` forms in its origin city in slot `slot`, free, bound for the
    /// destination.
    pub(crate) fn caravan_form(&mut self, r: RouteId, slot: usize) -> Option<UnitId> {
        let ty = self.caravan_unit_type()?;
        let (owner, origin) = (self.routes[r].owner, self.routes[r].origin);
        if !self.cities.contains(origin) {
            return None;
        }
        let (x, y) = (self.cities[origin].x, self.cities[origin].y);
        let u = self.player_init_unit(owner, ty, x, y, Some(UnitAIType::UNITAI_UNKNOWN), None);
        if !self.units.contains(u) {
            return None;
        }
        self.caravan_join_route(u, r, slot);
        Some(u)
    }

    /// Takes caravan `slot` of route `r` off the map by rule (no killer, nothing lost): the slot
    /// waits to form again in the origin.
    pub(crate) fn caravan_remove(&mut self, r: RouteId, slot: usize) {
        let Some(u) = self.routes[r].slots[slot].unit.take() else { return };
        self.routes[r].slots[slot].bound = Leg::Out;
        if self.units.contains(u) {
            self.units[u].route = None;
            // (A removal by rule, not a death: telemetry counts it under its own cause.)
            self.tel_ctx_push("trade");
            self.unit_kill(u, false, None);
            self.tel_ctx_pop();
        }
    }

}
