//! Trade (docs/TRADE.md, phase 1): the limits of 3.1, the refusals of 3.3, the path (costs, entry,
//! ties), establishing and the first trip, movement, pay and growth, raids on land and at sea,
//! closing and reopening, seizure, the route's end, dissolving, a save; on situations built by
//! hand. Numbers are read from the defines, not assumed.

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_core::trade::{Leg, RouteRefusal};
use ffh_data::*;

/// A three-player game (no barbarians) on a 40x16 map of coast: the home island (x 1..=12,
/// y 2..=13) and the far island (x 22..=33, y 2..=13), all revealed. Player 0's capital at (6, 7),
/// its port at (12, 10); player 1's port at (22, 10). Seas cut by hand: Gap (x 13..=21) and Rim.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    p2: PlayerId,
    capital: CityId,
    port: CityId,
    far: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 3, "WORLDSIZE_SMALL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 16, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=13).contains(&y) && ((1..=12).contains(&x) || (22..=33).contains(&x));
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let gap: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.plot_is_water(q) && (13..=21).contains(&g.map.xy(q).0)).collect();
    let rim: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.plot_is_water(q) && !(13..=21).contains(&g.map.xy(q).0)).collect();
    g.map_cut_sea("Gap", &gap);
    g.map_cut_sea("Rim", &rim);
    let (p0, p1, p2) = (PlayerId(0), PlayerId(1), PlayerId(2));
    for (a, b) in [(p0, p1), (p0, p2), (p1, p2)] {
        let (ta, tb) = (g.players[a].team, g.players[b].team);
        g.team_meet(ta, tb, false);
    }
    g.players[p0].turn_active = true;
    let capital = g.player_init_city(p0, 6, 7, true, true);
    let port = g.player_init_city(p0, 12, 10, true, true);
    let far = g.player_init_city(p1, 22, 10, true, true);
    reveal(&mut g);
    World { g, p0, p1, p2, capital, port, far }
}

/// Every plot revealed to every team (the cities on them too).
fn reveal(g: &mut Game) {
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, false, false, None, false);
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn caravan_at(&mut self, c: CityId) -> UnitId {
        reveal(&mut self.g);
        let (x, y, owner) = (self.g.cities[c].x, self.g.cities[c].y, self.g.cities[c].owner);
        self.unit(owner, "UNIT_CARAVAN", x, y)
    }

    fn team(&self, p: PlayerId) -> TeamId {
        self.g.players[p].team
    }

    fn open_borders(&mut self, a: PlayerId, b: PlayerId, on: bool) {
        let (ta, tb) = (self.team(a), self.team(b));
        self.g.team_set_open_borders(ta, tb, on);
        self.g.team_set_open_borders(tb, ta, on);
    }

    fn war(&mut self, a: PlayerId, b: PlayerId) {
        let (ta, tb) = (self.team(a), self.team(b));
        self.g.team_declare_war(ta, tb, false, None);
    }

    /// A route of the origin's owner from `origin` to `dest`, by a Caravan trained there.
    fn establish(&mut self, origin: CityId, dest: CityId) -> RouteId {
        let u = self.caravan_at(origin);
        self.g.unit_establish_route(u, dest, [None, None]).expect("the route is established")
    }

    /// Turn ends of the route's owner until its first caravan arrives (at most 30).
    fn arrive(&mut self, r: RouteId) -> i32 {
        let p = self.g.routes[r].owner;
        for turn in 1..=30 {
            self.g.player_move_caravans(p);
            if self.g.routes[r].arrived {
                return turn;
            }
        }
        panic!("the first caravan never arrived");
    }

    fn logged(&self, text: &str) -> usize {
        self.g.log.entries.iter().filter(|m| m.text.contains(text)).count()
    }

    fn consistent(&self) {
        let stale = self.g.trade_stale();
        assert!(stale.is_empty(), "the trade state is consistent: {stale:?}");
    }
}

/// The sum of the route costs of the plots `path` enters (the first is not entered).
fn path_cost(g: &Game, team: TeamId, path: &[PlotIdx], dest: PlotIdx) -> i32 {
    let d = &g.d.defines;
    path[1..]
        .iter()
        .map(|&q| {
            g.plot_route_cost(team, q).unwrap_or_else(|| {
                assert_eq!(q, dest, "only the destination may be no plot to pass");
                if g.map.plots[q].route_type.is_some() {
                    d.caravan_cost_road
                } else {
                    d.caravan_cost_land
                }
            })
        })
        .sum()
}

// ---- What a Caravan is ----

#[test]
fn the_caravan_unit() {
    let w = world();
    let d = w.g.d;
    let ty = d.t::<UnitType>("UNIT_CARAVAN");
    let info = &d.units[ty];
    assert_eq!(info.production_cost, 60);
    assert!(info.prereq_and_tech.is_none());
    assert_eq!(info.domain_type, Some(DomainType::DOMAIN_LAND));
    assert!(info.only_defensive && info.no_experience);
    assert_eq!((info.attack_rating(), info.defense_rating()), (0, 30), "strength 3: DEF 30, no attack");
    assert!(!w.g.unit_type_is_army_unit(ty), "not an army unit");
    assert_eq!(info.power_value, 0);
    assert!(w.g.city_can_train(w.capital, ty, false, false, false, false), "every civilization trains it");
    for n in 2..=4 {
        let p = d.t::<PromotionType>(&format!("PROMOTION_CARAVAN_GUARD_{n}"));
        assert_eq!(d.promotions[p].min_level, -1, "not obtainable by level");
    }
}

#[test]
fn a_caravan_in_a_route_takes_no_order() {
    let mut w = world();
    let r = w.establish(w.capital, w.port);
    let u = w.g.routes[r].slots[0].unit.unwrap();
    let g = w.g.units[u].group.unwrap();
    assert!(!w.g.group_ready_to_move(g, false) && !w.g.group_ready_to_move(g, true), "never waiting for orders");
    assert!(!w.g.player_has_ready_unit(w.p0, true));
    assert!(!w.g.group_can_start_mission(g, MissionType::MISSION_MOVE_TO, 8, 8, None, false));
    assert!(!w.g.group_can_start_mission(g, MissionType::MISSION_SKIP, -1, -1, None, false));
    assert!(!w.g.unit_can_do_command(u, CommandType::COMMAND_DELETE, -1, -1, false, false), "not even delete");
    assert!(w.g.unit_is_immune_to_capture(u), "never captured");
    // Another unit cannot join its group.
    let other = w.caravan_at(w.capital);
    assert!(!w.g.unit_can_join_group_opt(other, Some(w.g.city_plot(w.capital)), g));
    // A second Caravan is an ordinary unit until it has a route.
    let og = w.g.units[other].group.unwrap();
    assert!(w.g.group_ready_to_move(og, false));
    w.consistent();
}

// ---- Limits (3.1) ----

#[test]
fn the_limits() {
    let mut w = world();
    let d = w.g.d;
    assert_eq!(w.g.players[w.p0].trade_routes, d.defines.initial_trade_routes);
    assert_eq!(w.g.player_route_limit(w.p0), w.g.players[w.p0].trade_routes + w.g.trade_routes);
    assert_eq!(w.g.city_origin_limit(w.capital), 1);
    // A third city to fill the routes.
    let third = w.g.player_init_city(w.p0, 4, 11, true, true);
    w.g.players[w.p0].trade_routes = 2;
    w.establish(w.capital, w.port);
    // The capital is the origin of one route.
    let u = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_establish_refusal(u, third, [None, None]), Some(RouteRefusal::OriginFull));
    // Its extra_trade_routes (a building) allow another.
    w.g.city_change_extra_trade_routes(w.capital, 1);
    assert_eq!(w.g.city_origin_limit(w.capital), 2);
    assert_eq!(w.g.unit_establish_refusal(u, third, [None, None]), None);
    w.g.unit_establish_route(u, third, [None, None]).unwrap();
    assert_eq!((w.g.player_routes_used(w.p0), w.g.city_origin_used(w.capital)), (2, 2));
    // All used: none free.
    let v = w.caravan_at(w.port);
    assert_eq!(w.g.unit_establish_refusal(v, third, [None, None]), Some(RouteRefusal::NoRouteFree));
    // A coastal route: only a coastal city can use it.
    w.g.players[w.p0].coastal_trade_routes = 1;
    assert!(w.g.city_is_route_coastal(w.port));
    assert_eq!(w.g.unit_establish_refusal(v, third, [None, None]), None, "the port is coastal");
    assert!(!w.g.city_is_route_coastal(w.capital), "the capital is inland");
    w.g.city_change_extra_trade_routes(w.capital, 1);
    let x = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_establish_refusal(x, w.far, [None, None]), Some(RouteRefusal::CoastalOnly));
    // The game's routes (the votes) count for everyone.
    w.g.trade_routes = 1;
    assert_eq!(w.g.player_route_limit(w.p0), 3);
}

// ---- Destinations (3.3) ----

#[test]
fn each_refusal() {
    let mut w = world();
    let u = w.caravan_at(w.capital);
    let none = [None, None];
    assert_eq!(w.g.unit_establish_refusal(u, w.capital, none), Some(RouteRefusal::SameCity));
    // Foreign: no open borders.
    assert_eq!(w.g.unit_establish_refusal(u, w.far, none), Some(RouteRefusal::NoOpenBorders(w.p1)));
    w.open_borders(w.p0, w.p1, true);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, none), None);
    // A no-foreign-trade civic on either side.
    w.g.players[w.p1].no_foreign_trade_count += 1;
    assert_eq!(w.g.unit_establish_refusal(u, w.far, none), Some(RouteRefusal::NoForeignTrade(w.p1)));
    w.g.players[w.p1].no_foreign_trade_count -= 1;
    // Cargo is not built.
    let gems = w.g.d.t::<BonusType>("BONUS_GEMS");
    assert_eq!(w.g.unit_establish_refusal(u, w.far, [Some(gems), None]), Some(RouteRefusal::CargoNoCopy(gems)));
    assert_eq!(w.g.unit_establish_route(u, w.far, [Some(gems), None]), Err(RouteRefusal::CargoNoCopy(gems)));
    // Not revealed.
    let t0 = w.team(w.p0);
    let fp = w.g.city_plot(w.far);
    w.g.plot_set_revealed(fp, t0, false, false, None, false);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, none), Some(RouteRefusal::NotRevealed));
    w.g.plot_set_revealed(fp, t0, true, false, None, false);
    // An outpost.
    let colonist = w.unit(w.p0, "UNIT_COLONIST", 33, 7);
    assert!(w.g.unit_found_outpost(colonist));
    let outpost = w.g.map.plots[w.at(33, 7)].plot_city.unwrap();
    assert_eq!(w.g.unit_establish_refusal(u, outpost, none), Some(RouteRefusal::DestinationOutpost));
    let v = w.caravan_at(outpost);
    assert_eq!(w.g.unit_establish_refusal(v, w.capital, none), Some(RouteRefusal::OriginOutpost));
    // No path: the destination walled in by peaks.
    let walled = w.g.player_init_city(w.p0, 3, 4, true, true);
    for (x, y) in [(2, 3), (3, 3), (4, 3), (2, 4), (4, 4), (2, 5), (3, 5), (4, 5)] {
        let q = w.at(x, y);
        w.g.plot_set_plot_type(q, PlotType::PLOT_PEAK, false);
        assert!(w.g.plot_is_impassable(q));
    }
    reveal(&mut w.g);
    // (3, 4) touches no water: (1..) land, the west coast is at x 0.
    assert_eq!(w.g.unit_establish_refusal(u, walled, none), Some(RouteRefusal::NoPath));
    // Already linked, in either direction.
    w.g.unit_establish_route(u, w.port, none).unwrap();
    let x = w.caravan_at(w.port);
    assert_eq!(w.g.unit_establish_refusal(x, w.capital, none), Some(RouteRefusal::AlreadyLinked));
    // Not a Caravan; not in a city.
    let axe = w.unit(w.p0, "UNIT_AXEMAN", 6, 7);
    assert_eq!(w.g.unit_establish_refusal(axe, w.port, none), Some(RouteRefusal::NotACaravan));
    let y = w.unit(w.p0, "UNIT_CARAVAN", 8, 8);
    assert_eq!(w.g.unit_establish_refusal(y, w.port, none), Some(RouteRefusal::NotInOwnCity));
    // At war.
    w.war(w.p0, w.p1);
    let z = w.caravan_at(w.capital);
    assert_eq!(w.g.route_destination_refusal(w.p0, w.capital, w.far), Some(RouteRefusal::AtWar(w.p1)));
    let _ = z;
    // Every refusal has bare text.
    assert_eq!(RouteRefusal::NoOpenBorders(w.p1).text(&w.g), format!("No open borders with {}.", w.g.player_get_civilization_short_description(w.p1)));
}

// ---- The path (3.5) ----

#[test]
fn the_path_costs_and_entry() {
    let mut w = world();
    let d = w.g.d;
    let t0 = w.team(w.p0);
    let road = d.t::<RouteType>("ROUTE_ROAD");
    let (open, water) = (w.at(8, 8), w.at(16, 8));
    assert_eq!(w.g.plot_route_cost(t0, open), Some(d.defines.caravan_cost_land));
    w.g.plot_set_route_type(open, Some(road), false);
    assert_eq!(w.g.plot_route_cost(t0, open), Some(d.defines.caravan_cost_road));
    // Water: in range only (the Gap's middle is past a range of 8 from the port alone).
    let far_water = w.at(21, 8);
    assert_eq!(w.g.plot_route_cost(t0, w.at(14, 10)), Some(d.defines.caravan_cost_water));
    assert_eq!(w.g.plot_in_sea_range(t0, far_water), w.g.plot_route_cost(t0, far_water).is_some());
    let _ = water;
    // Land of a team with no open borders: closed; open borders or war: open.
    let p1_land = w.at(23, 10);
    assert_eq!(w.g.map.plots[p1_land].owner, Some(w.p1));
    assert_eq!(w.g.plot_route_cost(t0, p1_land), None);
    w.open_borders(w.p0, w.p1, true);
    assert_eq!(w.g.plot_route_cost(t0, p1_land), Some(d.defines.caravan_cost_land));
    // At war: neither its land nor its city.
    w.open_borders(w.p0, w.p1, false);
    w.war(w.p0, w.p1);
    assert_eq!(w.g.plot_route_cost(t0, p1_land), None);
    assert_eq!(w.g.plot_route_cost(t0, w.g.city_plot(w.far)), None);
    // Impassable.
    let peak = w.at(9, 4);
    w.g.plot_set_plot_type(peak, PlotType::PLOT_PEAK, false);
    assert_eq!(w.g.plot_route_cost(t0, peak), None);
}

#[test]
fn the_path_is_the_cheapest_then_the_shortest_then_the_lowest() {
    let mut w = world();
    let d = w.g.d;
    let t0 = w.team(w.p0);
    let a = w.g.player_init_city(w.p0, 4, 4, true, true);
    let b = w.g.player_init_city(w.p0, 7, 5, true, true);
    let path = w.g.route_path(w.p0, a, b).unwrap();
    // Three steps (dx 3, dy 1); among those, each plot entered from the lower plot index.
    assert_eq!(path, vec![w.at(4, 4), w.at(5, 3), w.at(6, 4), w.at(7, 5)]);
    assert_eq!(path_cost(&w.g, t0, &path, w.at(7, 5)), 2 * d.defines.caravan_cost_land + w.g.plot_route_cost(t0, w.at(7, 5)).unwrap());
    // A road elsewhere makes a longer path cheaper when it is.
    let road = d.t::<RouteType>("ROUTE_ROAD");
    for (x, y) in [(5, 5), (6, 6)] {
        let q = w.at(x, y);
        w.g.plot_set_route_type(q, Some(road), false);
    }
    let path = w.g.route_path(w.p0, a, b).unwrap();
    let cost = path_cost(&w.g, t0, &path, w.at(7, 5));
    let straight = vec![w.at(4, 4), w.at(5, 3), w.at(6, 4), w.at(7, 5)];
    assert!(cost <= path_cost(&w.g, t0, &straight, w.at(7, 5)));
    if 2 * d.defines.caravan_cost_road < 2 * d.defines.caravan_cost_land {
        assert_eq!(path, vec![w.at(4, 4), w.at(5, 5), w.at(6, 6), w.at(7, 5)], "the road");
    }
}

// ---- Establishing and the first trip (3.2, 4.1) ----

#[test]
fn establishing_the_first_trip_and_the_second_caravan() {
    let mut w = world();
    let d = w.g.d;
    let r = w.establish(w.capital, w.port);
    let route = w.g.routes[r].clone();
    assert!(route.is_open() && !route.arrived);
    assert_eq!(route.length, route.path.len() as i32 - 1);
    assert_eq!(route.path.first(), Some(&w.g.city_plot(w.capital)));
    assert_eq!(route.path.last(), Some(&w.g.city_plot(w.port)));
    assert!(route.slots[1].unit.is_none(), "one caravan until it arrives");
    assert_eq!(w.g.route_pay(r).caravan_x100, [0, 0], "nothing paid before the arrival");
    assert_eq!(w.logged("Route established"), 1);
    // The expected turns: the path's costs walked with CARAVAN_MOVE a turn.
    let t0 = w.team(w.p0);
    let mut turns = 1;
    let mut left = d.defines.caravan_move;
    for &q in &route.path[1..] {
        let c = w.g.plot_route_cost(t0, q).unwrap();
        if c > left {
            turns += 1;
            left = d.defines.caravan_move;
        }
        left -= c;
    }
    let u = route.slots[0].unit.unwrap();
    let moves = w.g.units[u].moves;
    assert_eq!(w.arrive(r), turns);
    assert_eq!(w.g.units[u].moves, moves, "no movement points are spent");
    let route = w.g.routes[r].clone();
    assert!(route.arrived);
    assert_eq!(route.slots[0].bound, Leg::Back, "it turns around");
    let second = route.slots[1].unit.expect("the second caravan forms");
    assert_eq!(w.g.unit_plot(second), w.g.city_plot(w.capital), "in the origin");
    assert_eq!(w.g.units[second].route, Some(r));
    assert!(w.g.route_pay(r).caravan_x100.iter().all(|&x| x > 0), "both pay");
    w.consistent();
    // The second caravan moves on the next turn end, not on the one it formed.
    w.g.player_move_caravans(w.p0);
    assert_ne!(w.g.unit_plot(second), w.g.city_plot(w.capital));
    w.consistent();
}

// ---- Movement (4.1) ----

/// Plots a fresh caravan of `r` walks in one turn end.
fn one_turn(w: &mut World, r: RouteId) -> usize {
    let u = w.g.routes[r].slots[0].unit.unwrap();
    let path = w.g.routes[r].path.clone();
    let before = path.iter().position(|&q| q == w.g.unit_plot(u)).unwrap();
    w.g.player_move_caravans(w.g.routes[r].owner);
    path.iter().position(|&q| q == w.g.unit_plot(u)).unwrap() - before
}

#[test]
fn movement_on_road_land_and_water() {
    let d = ffh_data::global();
    let m = &d.defines;
    // Open land: (2, 7) to (12, 7) is ten plots of grass.
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    assert_eq!(one_turn(&mut w, r) as i32, m.caravan_move / m.caravan_cost_land);
    // Road.
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let road = d.t::<RouteType>("ROUTE_ROAD");
    for x in 2..=11 {
        let q = w.at(x, 3);
        w.g.plot_set_route_type(q, Some(road), false);
    }
    let r = w.establish(a, b);
    assert_eq!(one_turn(&mut w, r) as i32, m.caravan_move / m.caravan_cost_road);
    // Water: a domestic route across the Gap to a city of its own on the far island.
    let mut w = world();
    let across = w.g.player_init_city(w.p0, 22, 4, true, true);
    let r = w.establish(w.port, across);
    let path = w.g.routes[r].path.clone();
    assert!(path.iter().any(|&q| w.g.plot_is_water(q)));
    let u = w.g.routes[r].slots[0].unit.unwrap();
    // Walk to the water first.
    for _ in 0..3 {
        if w.g.plot_is_water(w.g.unit_plot(u)) {
            break;
        }
        w.g.player_move_caravans(w.p0);
    }
    let on_water = path.iter().position(|&q| q == w.g.unit_plot(u)).unwrap();
    assert!(path[on_water + 1..].iter().take((m.caravan_move / m.caravan_cost_water) as usize + 1).all(|&q| w.g.plot_is_water(q)), "open water ahead");
    assert_eq!(one_turn(&mut w, r) as i32, m.caravan_move / m.caravan_cost_water);
    assert!(!w.g.unit_is_in_boats(u), "a caravan is not in boats");
    w.consistent();
}

#[test]
fn a_caravan_waits_behind_an_enemy_unit() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let path = w.g.routes[r].path.clone();
    let block = path[2];
    let (bx, by) = w.g.map.xy(block);
    w.war(w.p0, w.p2);
    w.unit(w.p2, "UNIT_AXEMAN", bx, by);
    let u = w.g.routes[r].slots[0].unit.unwrap();
    for _ in 0..3 {
        w.g.player_move_caravans(w.p0);
    }
    assert_eq!(w.g.unit_plot(u), path[1], "it waits in front of the enemy");
    assert!(w.g.units.contains(u));
}

// ---- Pay (6.1-6.3) ----

fn expected_total(g: &Game, r: RouteId) -> i32 {
    let m = &g.d.defines;
    let route = &g.routes[r];
    let (o, dst) = (route.origin, route.destination);
    let full = m.trade_length_full;
    let l = route.length;
    let eff = if l <= full { l } else { full + (l.min(m.trade_length_cap) - full) / 2 };
    let base = eff * (g.cities[o].population + g.cities[dst].population) * m.trade_pay_percent / 100;
    let mut pct = 100 + g.cities[o].trade_route_modifier + route.growth;
    if g.cities[dst].owner != route.owner {
        pct += m.trade_foreign_percent + g.cities[o].foreign_trade_route_modifier;
    }
    if route.path.iter().any(|&q| g.plot_is_water(q)) {
        pct += m.overseas_trade_modifier;
    }
    base * pct / 100
}

fn check_pay(w: &World, r: RouteId) {
    let g = &w.g;
    let total = expected_total(g, r);
    let halves = [total - total / 2, total / 2];
    let route = &g.routes[r];
    let pay = g.route_pay(r);
    let mut dest = 0;
    for i in 0..2 {
        let want = if route.slots[i].unit.is_some() { halves[i] } else { 0 };
        assert_eq!(pay.caravan_x100[i], want, "caravan {i}");
        dest += want * g.d.defines.trade_dest_percent / 100;
    }
    assert_eq!(pay.dest_x100, dest);
    // The cities' trade yield: the hundredths rounded to the nearest, through the modifier.
    let y = YieldType::YIELD_COMMERCE;
    for c in [route.origin, route.destination] {
        let mut sum = 0;
        for (id, other) in g.routes.iter() {
            let p = g.route_pay(id);
            if other.origin == c {
                sum += p.caravan_x100.iter().sum::<i32>();
            }
            if other.destination == c {
                sum += p.dest_x100;
            }
        }
        let m = g.players[g.cities[c].owner].trade_yield_modifier[y.idx()];
        assert_eq!(g.cities[c].trade_yield[y.idx()], ((sum + 50) / 100) * m / 100, "{}", g.cities[c].name);
    }
    w.consistent();
}

#[test]
fn the_effective_length() {
    let g = world().g;
    let m = &g.d.defines;
    let (full, cap) = (m.trade_length_full, m.trade_length_cap);
    assert_eq!(g.route_effective_length(full), full);
    assert_eq!(g.route_effective_length(full + 2), full + 1);
    assert_eq!(g.route_effective_length(cap), full + (cap - full) / 2);
    assert_eq!(g.route_effective_length(cap + 10), g.route_effective_length(cap), "nothing beyond the cap");
}

#[test]
fn pay_with_each_term_and_the_split() {
    let mut w = world();
    for c in [w.capital, w.port, w.far] {
        w.g.city_set_population(c, 6);
    }
    let r = w.establish(w.capital, w.port);
    w.arrive(r);
    check_pay(&w, r);
    assert!(w.g.route_pay(r).caravan_x100[0] > 0);
    // The origin's buildings.
    w.g.city_change_trade_route_modifier(w.capital, 50);
    check_pay(&w, r);
    // Growth.
    w.g.routes[r].growth = 20;
    w.g.trade_refresh_pay();
    check_pay(&w, r);
    // Population.
    w.g.city_set_population(w.port, 9);
    check_pay(&w, r);
    // One caravan dead: only the other's half.
    w.g.routes[r].growth = 0;
    let u = w.g.routes[r].slots[1].unit.unwrap();
    w.g.unit_kill(u, false, None);
    check_pay(&w, r);
    assert_eq!(w.g.route_pay(r).caravan_x100[1], 0);
    // Foreign, over water, with the foreign modifier: a route from the port to player 1.
    w.open_borders(w.p0, w.p1, true);
    w.g.city_change_extra_trade_routes(w.port, 0);
    let f = w.establish(w.port, w.far);
    assert!(w.g.routes[f].path.iter().any(|&q| w.g.plot_is_water(q)));
    w.arrive(f);
    w.g.city_change_foreign_trade_route_modifier(w.port, 25);
    check_pay(&w, f);
    // The destination's share enters player 1's city.
    let y = YieldType::YIELD_COMMERCE;
    let m = w.g.players[w.p1].trade_yield_modifier[y.idx()];
    assert_eq!(w.g.cities[w.far].trade_yield[y.idx()], ((w.g.route_pay(f).dest_x100 + 50) / 100) * m / 100);
    // An offer gives the same numbers at growth 0 with two caravans.
    let offers = w.g.route_offers(w.capital);
    let o = offers.iter().find(|o| o.destination == w.port).is_none();
    assert!(o, "already linked: not offered");
}

#[test]
fn offers_list_the_destinations_with_their_pay() {
    let mut w = world();
    w.open_borders(w.p0, w.p1, true);
    let offers = w.g.route_offers(w.port);
    let to_capital = offers.iter().find(|o| o.destination == w.capital).expect("the capital is offered");
    assert!(!to_capital.foreign && to_capital.seas.is_empty());
    let to_far = offers.iter().find(|o| o.destination == w.far).expect("player 1's port is offered");
    assert!(to_far.foreign && !to_far.seas.is_empty());
    assert_eq!(to_far.length, to_far.path.len() as i32 - 1);
    // The offer's pay is the route's at growth 0 once it runs with both caravans.
    let r = w.establish(w.port, w.far);
    w.arrive(r);
    let pay = w.g.route_pay(r);
    assert_eq!(to_far.owner_pay_x100, pay.caravan_x100.iter().sum::<i32>());
    assert_eq!(to_far.dest_pay_x100, pay.dest_x100);
}

// ---- Growth (6.2) ----

#[test]
fn growth_rises_is_halved_and_zero_after_war() {
    let mut w = world();
    let m = &w.g.d.defines;
    let (step, max) = (m.trade_growth_percent, m.trade_growth_max);
    w.open_borders(w.p0, w.p1, true);
    let r = w.establish(w.port, w.far);
    w.g.player_update_routes(w.p0);
    assert_eq!(w.g.routes[r].growth, 0, "not with one caravan");
    w.arrive(r);
    w.g.player_update_routes(w.p0);
    assert_eq!(w.g.routes[r].growth, step);
    for _ in 0..100 {
        w.g.player_update_routes(w.p0);
    }
    assert_eq!(w.g.routes[r].growth, max, "to the most");
    let u = w.g.routes[r].slots[0].unit.unwrap();
    w.g.unit_kill(u, false, Some(w.p2));
    assert_eq!(w.g.routes[r].growth, max / 2, "halved on a loss");
    w.war(w.p0, w.p1);
    assert_eq!(w.g.routes[r].growth, 0, "war between the two owners");
    assert!(!w.g.routes[r].is_open());
    w.consistent();
}

// ---- Raids (4.3) ----

#[test]
fn a_killed_caravan_pays_plunder_and_re_forms() {
    let mut w = world();
    let m = &w.g.d.defines;
    let (reform, turns) = (m.caravan_reform_turns, m.trade_plunder_turns);
    let r = w.establish(w.capital, w.port);
    w.arrive(r);
    w.g.routes[r].growth = 10;
    w.g.trade_refresh_pay();
    let u = w.g.routes[r].slots[0].unit.unwrap();
    let pay = w.g.routes[r].slots[0].pay_x100;
    let gold = w.g.players[w.p2].gold;
    w.g.unit_kill(u, false, Some(w.p2));
    assert_eq!(w.g.players[w.p2].gold - gold, (turns * pay + 50) / 100, "plunder");
    let route = w.g.routes[r].clone();
    assert_eq!(route.growth, 5);
    assert!(route.slots[0].unit.is_none());
    assert_eq!(route.slots[0].lost_to, Some(w.p2));
    assert_eq!(w.logged("lost to"), 1);
    w.consistent();
    for i in 1..=reform {
        w.g.player_update_routes(w.p0);
        assert_eq!(w.g.routes[r].slots[0].unit.is_some(), i == reform, "turn {i}");
    }
    let back = w.g.routes[r].slots[0].unit.unwrap();
    assert_eq!(w.g.unit_plot(back), w.g.city_plot(w.capital), "in the origin");
    assert_eq!(w.logged("re-formed"), 1);
    // The barbarians and no killer gain nothing; growth still halves.
    let u = w.g.routes[r].slots[1].unit.unwrap();
    let bp = w.g.barbarian_player();
    let bgold = w.g.players[bp].gold;
    w.g.unit_kill(u, false, Some(bp));
    assert_eq!(w.g.players[bp].gold, bgold);
    w.consistent();
}

#[test]
fn a_caravan_killed_in_a_battle_on_land() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    w.g.routes[r].arrived = true;
    w.g.trade_refresh_pay();
    w.g.player_move_caravans(w.p0);
    let u = w.g.routes[r].slots[0].unit.unwrap();
    let here = w.g.unit_plot(u);
    assert!(!w.g.plot_is_water(here) && w.g.map.plots[here].plot_city.is_none());
    w.g.unit_set_damage(u, 99, None);
    w.war(w.p0, w.p2);
    let (x, y) = w.g.map.xy(here);
    let champ = w.unit(w.p2, "UNIT_CHAMPION", x, y + 1);
    let pay = w.g.routes[r].slots[0].pay_x100;
    let gold = w.g.players[w.p2].gold;
    let grp = w.g.units[champ].group.unwrap();
    let (attacked, _) = w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    assert!(attacked);
    assert!(!w.g.units.contains(u) || w.g.unit_is_dead(u), "a 1 HP caravan falls");
    assert!(w.g.routes[r].slots[0].unit.is_none());
    assert_eq!(w.g.players[w.p2].gold - gold, (w.g.d.defines.trade_plunder_turns * pay + 50) / 100);
    w.consistent();
}

#[test]
fn a_caravan_intercepted_at_sea() {
    let mut w = world();
    let across = w.g.player_init_city(w.p0, 22, 4, true, true);
    let r = w.establish(w.port, across);
    w.g.routes[r].arrived = true;
    w.g.trade_refresh_pay();
    let u = w.g.routes[r].slots[0].unit.unwrap();
    w.g.unit_set_damage(u, 99, None);
    w.war(w.p0, w.p2);
    // A trireme of player 2 on Patrol in the Gap.
    let ship = w.unit(w.p2, "UNIT_TRIREME", 17, 1);
    assert!(w.g.unit_can_plunder(ship, w.at(17, 1), false));
    assert!(w.g.unit_plunder(ship));
    // One of player 0's own on Patrol there too: the sea is contested, not closed to it.
    let own = w.unit(w.p0, "UNIT_TRIREME", 17, 14);
    assert!(w.g.unit_plunder(own));
    let gap = w.g.map.plots[w.at(17, 1)].sea.unwrap();
    assert!(!w.g.sea_is_closed_to(gap, w.team(w.p0)));
    let pay = w.g.routes[r].slots[0].pay_x100;
    let gold = w.g.players[w.p2].gold;
    for _ in 0..6 {
        if !w.g.units.contains(u) || w.g.units[u].route.is_none() {
            break;
        }
        w.g.player_move_caravans(w.p0);
    }
    assert!(w.g.routes[r].slots[0].unit.is_none(), "the patrol sank it");
    assert_eq!(w.g.players[w.p2].gold - gold, (w.g.d.defines.trade_plunder_turns * pay + 50) / 100);
    assert_eq!(w.g.routes[r].slots[0].reform_turns, w.g.d.defines.caravan_reform_turns);
    w.consistent();
}

#[test]
fn a_hidden_nationality_attacker_needs_no_war() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    w.g.player_move_caravans(w.p0);
    let u = w.g.routes[r].slots[0].unit.unwrap();
    let here = w.g.unit_plot(u);
    let (x, y) = w.g.map.xy(here);
    let t2 = w.team(w.p2);
    assert!(!w.g.teams[w.team(w.p0)].at_war[t2]);
    let champ = w.unit(w.p2, "UNIT_CHAMPION", x, y + 1);
    let normal = w.g.unit_can_move_into(champ, here, true, false, false);
    assert!(!normal, "no attack without war");
    let hn = w.g.d.t::<PromotionType>("PROMOTION_HIDDEN_NATIONALITY");
    w.g.unit_set_has_promotion(champ, hn, true);
    assert!(w.g.unit_is_hidden_nationality(champ));
    assert!(w.g.unit_can_move_into(champ, here, true, false, false), "a hidden-nationality unit may attack it");
    w.g.unit_set_damage(u, 99, None);
    let grp = w.g.units[champ].group.unwrap();
    let (attacked, _) = w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    assert!(attacked);
    assert!(w.g.routes[r].slots[0].unit.is_none());
    assert!(!w.g.teams[w.team(w.p0)].at_war[t2], "still no war");
    w.consistent();
}

// ---- Closing, seizure, ending (3.4) ----

#[test]
fn closing_and_reopening_on_open_borders() {
    let mut w = world();
    w.open_borders(w.p0, w.p1, true);
    let r = w.establish(w.port, w.far);
    w.arrive(r);
    w.g.player_update_routes(w.p0);
    assert!(w.g.routes[r].is_open());
    let growth = w.g.routes[r].growth;
    w.open_borders(w.p0, w.p1, false);
    w.g.player_update_routes(w.p0);
    let route = w.g.routes[r].clone();
    assert_eq!(route.closed, Some(RouteRefusal::NoOpenBorders(w.p1)));
    assert!(route.slots.iter().all(|s| s.unit.is_none()), "the caravans wait in the origin");
    assert_eq!(route.growth, growth, "growth is kept");
    assert_eq!(w.g.route_pay(r).caravan_x100, [0, 0]);
    assert_eq!(w.logged("closed: No open borders"), 1);
    w.consistent();
    w.open_borders(w.p0, w.p1, true);
    w.g.player_update_routes(w.p0);
    let route = w.g.routes[r].clone();
    assert!(route.is_open());
    assert!(route.slots.iter().all(|s| s.unit.is_some()), "both form in the origin");
    for s in &route.slots {
        assert_eq!(w.g.unit_plot(s.unit.unwrap()), w.g.city_plot(w.port));
    }
    assert_eq!(w.logged("open again"), 1);
    w.consistent();
}

#[test]
fn seizure_at_a_war_declaration() {
    let mut w = world();
    w.open_borders(w.p0, w.p1, true);
    let r = w.establish(w.port, w.far);
    w.arrive(r);
    // Walk the second caravan until the first stands on player 1's land, if it does.
    let first = w.g.routes[r].slots[0].unit.unwrap();
    // The first caravan is in player 1's city now (it arrived): player 1's land.
    let here = w.g.unit_plot(first);
    assert_eq!(w.g.map.plots[here].owner, Some(w.p1));
    let pay = w.g.routes[r].slots[0].pay_x100;
    let gold = w.g.players[w.p1].gold;
    w.war(w.p1, w.p0);
    assert!(w.g.routes[r].slots[0].unit.is_none());
    assert!(!w.g.units.contains(first));
    assert_eq!(w.g.routes[r].slots[0].lost_to, Some(w.p1), "seized by the land's owner");
    assert_eq!(w.g.players[w.p1].gold - gold, (w.g.d.defines.trade_plunder_turns * pay + 50) / 100);
    assert_eq!(w.g.routes[r].growth, 0);
    assert_eq!(w.g.routes[r].closed, Some(RouteRefusal::AtWar(w.p1)));
    assert_eq!(w.logged("Caravan seized"), 1);
    w.consistent();
}

#[test]
fn the_route_ends_when_the_origin_is_lost_and_follows_a_captured_destination() {
    let mut w = world();
    w.open_borders(w.p0, w.p1, true);
    let r = w.establish(w.port, w.far);
    w.arrive(r);
    let s = w.establish(w.capital, w.port);
    let caravans = w.g.routes[s].caravans();
    // Player 1 takes the port: route s's destination changes hands, route r's origin is lost.
    w.g.player_acquire_city(w.p1, w.port, false, true, true);
    assert!(!w.g.routes.contains(r), "the origin is lost: it ends");
    assert!(w.g.routes.contains(s), "the destination changed hands: it stays");
    let new_port = w.g.map.plots[w.at(12, 10)].plot_city.unwrap();
    assert_eq!(w.g.routes[s].destination, new_port);
    assert!(caravans.iter().all(|&u| w.g.units.contains(u) || w.g.routes[s].caravans().iter().all(|&v| v != u)));
    assert_eq!(w.logged("the origin is lost"), 1);
    w.consistent();
    // Its owner's next turn start: no open borders with player 1 (p0 and p1 have them): still open.
    w.g.player_update_routes(w.p0);
    w.consistent();
    // Destroyed destination: the route ends.
    w.g.city_kill(new_port, true);
    assert!(!w.g.routes.contains(s));
    w.consistent();
}

#[test]
fn dissolving() {
    let mut w = world();
    let r = w.establish(w.capital, w.port);
    w.arrive(r);
    let units = w.g.routes[r].caravans();
    assert_eq!(units.len(), 2);
    assert!(w.g.route_dissolve(r));
    assert!(!w.g.routes.contains(r));
    assert!(units.iter().all(|&u| !w.g.units.contains(u)));
    assert_eq!(w.g.player_routes_used(w.p0), 0);
    assert!(w.g.cities[w.capital].trade_yield.iter().all(|&y| y == 0));
    assert!(!w.g.route_dissolve(r));
    w.consistent();
}

// ---- Saves ----

#[test]
fn routes_survive_a_save() {
    let mut w = world();
    w.open_borders(w.p0, w.p1, true);
    let r = w.establish(w.port, w.far);
    w.arrive(r);
    let s = w.establish(w.capital, w.port);
    w.g.player_move_caravans(w.p0);
    let u = w.g.routes[r].slots[1].unit.unwrap();
    w.g.unit_kill(u, false, Some(w.p2));
    // A route closed by a war (a refusal with a player) and one with a lost caravan.
    w.war(w.p0, w.p1);
    assert_eq!(w.g.routes[r].closed, Some(RouteRefusal::AtWar(w.p1)));
    assert_eq!(w.g.routes[r].slots[1].lost_to, Some(w.p2));
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.routes[r], w.g.routes[r]);
    assert_eq!(h.routes[s], w.g.routes[s]);
    for c in w.g.routes[s].caravans() {
        assert_eq!(h.units[c].route, Some(s));
    }
    assert!(h.trade_stale().is_empty());
    assert_eq!(h.save(), bytes);
}

// ---- Through the rules' own turn ----

#[test]
fn a_route_runs_through_the_owner_s_turns() {
    let mut w = world();
    for c in [w.capital, w.port] {
        w.g.city_set_population(c, 6);
    }
    let r = w.establish(w.capital, w.port);
    let y = YieldType::YIELD_COMMERCE;
    let mut arrived_at = None;
    for turn in 0..40 {
        w.g.player_do_turn_units(w.p0);
        w.g.player_do_turn(w.p0);
        w.consistent();
        if arrived_at.is_none() && w.g.routes[r].arrived {
            arrived_at = Some(turn);
        }
    }
    assert!(arrived_at.is_some(), "the first caravan arrived");
    let route = w.g.routes[r].clone();
    assert!(route.is_open() && route.slots.iter().all(|s| s.unit.is_some()), "both caravans alive");
    assert!(route.growth > 0);
    // The pay reaches the origin's commerce.
    let trade = w.g.cities[w.capital].trade_yield[y.idx()];
    assert!(trade > 0, "the origin's trade yield");
    assert!(w.g.city_yield_rate(w.capital, y) >= trade);
    // The caravans shuttle: each has turned around at least once in 40 turns.
    let bounds: Vec<Leg> = route.slots.iter().map(|s| s.bound).collect();
    let plots: Vec<PlotIdx> = route.caravans().iter().map(|&u| w.g.unit_plot(u)).collect();
    assert!(plots.iter().all(|q| route.path.contains(q)), "on the path: {bounds:?}");
    assert!(!w.g.player_has_ready_unit(w.p0, true), "no caravan waits for orders");
}

// ---- The path avoids enemy land (3.5) ----

#[test]
fn a_route_whose_only_way_crosses_enemy_land_closes() {
    let mut w = world();
    // Player 2 holds column x 7 of the home island; the capital (6, 7) is inland.
    for y in 1..=14 {
        let q = w.at(7, y);
        if !w.g.plot_is_water(q) {
            w.g.plot_set_owner(q, Some(w.p2), false, true);
        }
    }
    let east = w.g.player_init_city(w.p0, 10, 7, true, true);
    let t0 = w.team(w.p0);
    assert!(!w.g.city_is_route_coastal(w.capital));
    let u = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_establish_refusal(u, east, [None, None]), Some(RouteRefusal::NoPath), "no open borders with player 2");
    w.open_borders(w.p0, w.p2, true);
    let r = w.g.unit_establish_route(u, east, [None, None]).expect("through player 2's land");
    // Unowned land stays enterable at war; player 2's does not.
    w.war(w.p0, w.p2);
    assert_eq!(w.g.plot_route_cost(t0, w.at(7, 7)), None, "enemy land");
    assert_eq!(w.g.plot_route_cost(t0, w.at(30, 3)), Some(w.g.d.defines.caravan_cost_land), "unowned land");
    assert_eq!(w.g.routes[r].closed, Some(RouteRefusal::NoPath));
    w.consistent();
}

// ---- The Great Merchant ----

#[test]
fn the_trade_mission_counts_a_share_of_a_caravan() {
    let w = world();
    let pct = w.g.d.defines.trade_mission_percent;
    let (a, b) = (&w.g.cities[w.far], &w.g.cities[w.capital]);
    let length = w.g.plot_distance(a.x, a.y, b.x, b.y);
    let total = w.g.route_pay_total_x100(w.p1, w.far, w.capital, length, false, 0);
    let half = total - total / 2;
    assert_eq!(w.g.trade_mission_profit(w.far, w.capital), (half * pct / 100 + 50) / 100);
}

// ---- Cargo (section 5) ----

fn gems(w: &World) -> BonusType {
    w.g.d.t::<BonusType>("BONUS_GEMS")
}

impl World {
    fn give(&mut self, c: CityId, b: BonusType, n: i32) {
        self.g.city_change_free_bonus(c, b, n);
        self.g.trade_refresh_pay();
    }
}

/// Player 0's route from its port to player 1's port with `cargo`, arrived.
fn cargo_route(w: &mut World, cargo: [Option<BonusType>; 2]) -> RouteId {
    w.open_borders(w.p0, w.p1, true);
    let u = w.caravan_at(w.port);
    let r = w.g.unit_establish_route(u, w.far, cargo).expect("established");
    w.arrive(r);
    r
}

#[test]
fn cargo_refusals() {
    let mut w = world();
    let g = gems(&w);
    let silk = w.g.d.t::<BonusType>("BONUS_SILK");
    w.open_borders(w.p0, w.p1, true);
    let u = w.caravan_at(w.port);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, [Some(g), None]), Some(RouteRefusal::CargoNoCopy(g)), "no copy");
    w.give(w.capital, g, 1);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, [Some(g), None]), None);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, [Some(g), Some(g)]), Some(RouteRefusal::CargoSame));
    w.give(w.capital, silk, 1);
    assert_eq!(w.g.unit_establish_refusal(u, w.far, [Some(g), Some(silk)]), None);
    // Domestic.
    let v = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_establish_refusal(v, w.port, [Some(g), None]), Some(RouteRefusal::CargoDomestic));
    // A copy another caravan carries is not free.
    w.g.unit_establish_route(u, w.far, [Some(g), None]).unwrap();
    let far2 = w.g.player_init_city(w.p1, 30, 10, true, true);
    let x = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_establish_refusal(x, far2, [Some(g), None]), Some(RouteRefusal::CargoNoCopy(g)));
    assert_eq!(w.g.unit_establish_refusal(x, far2, [Some(silk), None]), None);
    // The options say the same.
    let opts = w.g.route_cargo_options(w.p0, w.capital, far2, None);
    assert!(opts.iter().all(|o| o.bonus != g));
    let s = opts.iter().find(|o| o.bonus == silk).expect("silk is offered");
    assert!(s.only_copy && s.free_copies == 1);
    assert_eq!(s.class, ffh_core::trade::CargoClass::Luxury);
    assert!(w.g.route_cargo_options(w.p0, w.capital, w.port, None).is_empty(), "domestic: none");
    w.consistent();
}

#[test]
fn delivery_moves_the_only_copy() {
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 1);
    let happy = w.g.city_happy_level(w.far);
    assert!(w.g.city_has_bonus(w.capital, g) && !w.g.city_has_bonus(w.far, g));
    let r = cargo_route(&mut w, [Some(g), None]);
    assert_eq!(w.g.routes[r].slots[0].delivering, Some((w.p1, g)));
    assert_eq!(w.g.route_slot_cargo(r, 0).state, ffh_core::trade::CargoState::Delivering);
    assert!(w.g.city_has_bonus(w.far, g), "the recipient's cities have it");
    assert!(w.g.city_happy_level(w.far) > happy, "a happiness resource raises happiness there");
    assert!(!w.g.city_has_bonus(w.capital, g), "the owner's only copy is gone");
    assert_eq!(w.g.player_get_num_tradeable_bonuses(w.p1, g), 0, "it cannot be traded on");
    assert_eq!(w.g.players[w.p0].bonus_export[g], 1);
    assert_eq!(w.g.players[w.p1].bonus_import[g], 1);
    w.consistent();
}

#[test]
fn a_spare_copy_stays_with_the_owner() {
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 2);
    let r = cargo_route(&mut w, [Some(g), None]);
    assert!(w.g.routes[r].slots[0].delivering.is_some());
    assert!(w.g.city_has_bonus(w.capital, g));
    w.consistent();
}

#[test]
fn delivery_stops_and_resumes() {
    use ffh_core::trade::CargoState;
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 1);
    let r = cargo_route(&mut w, [Some(g), None]);
    // The copy lost, then back.
    w.give(w.capital, g, -1);
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::NoCopy);
    assert!(!w.g.city_has_bonus(w.far, g));
    w.consistent();
    w.give(w.capital, g, 1);
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::Delivering);
    assert!(w.logged("delivery stopped") >= 1);
    w.consistent();
    // The route closed, then open again.
    w.open_borders(w.p0, w.p1, false);
    w.g.player_update_routes(w.p0);
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::RouteClosed, "its caravan waits in the origin");
    assert!(!w.g.city_has_bonus(w.far, g) && w.g.city_has_bonus(w.capital, g), "the copy is back with its owner");
    w.consistent();
    w.open_borders(w.p0, w.p1, true);
    w.g.player_update_routes(w.p0);
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::Delivering);
    w.consistent();
    // The caravan killed: stops; re-formed: resumes.
    let u = w.g.routes[r].slots[0].unit.unwrap();
    w.g.unit_kill(u, false, Some(w.p2));
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::CaravanLost);
    assert!(!w.g.city_has_bonus(w.far, g));
    w.consistent();
    for _ in 0..w.g.d.defines.caravan_reform_turns {
        w.g.player_update_routes(w.p0);
    }
    assert_eq!(w.g.route_slot_cargo(r, 0).state, CargoState::Delivering, "it re-forms with its cargo");
    w.consistent();
}

#[test]
fn both_premiums_and_their_end() {
    let mut w = world();
    let g = gems(&w);
    let (low, high) = (w.g.d.defines.trade_cargo_percent, w.g.d.defines.trade_cargo_new_percent);
    w.give(w.capital, g, 1);
    let r = cargo_route(&mut w, [Some(g), None]);
    assert_eq!(w.g.routes[r].slots[0].premium, high, "the recipient has none");
    // The premium on the caravan's half only; the destination's share before it.
    let total = expected_total(&w.g, r);
    let halves = [total - total / 2, total / 2];
    let pay = w.g.route_pay(r);
    assert_eq!(pay.caravan_x100[0], halves[0] * (100 + high) / 100);
    assert_eq!(pay.caravan_x100[1], halves[1]);
    let dest_pct = w.g.d.defines.trade_dest_percent;
    assert_eq!(pay.dest_x100, halves[0] * dest_pct / 100 + halves[1] * dest_pct / 100);
    // The recipient's own copy: the low premium.
    w.give(w.far, g, 1);
    assert_eq!(w.g.routes[r].slots[0].premium, low);
    assert!(w.logged("now has") >= 1);
    w.give(w.far, g, -1);
    assert_eq!(w.g.routes[r].slots[0].premium, high);
    // A deal's import counts as having it.
    let p2city = w.g.player_init_city(w.p2, 30, 7, true, true);
    w.give(p2city, g, 1);
    let item = ffh_core::structs::TradeData { item_type: TradeableItem::TRADE_RESOURCES, data: g.0 as i32, offering: false, hidden: false };
    w.g.implement_deal(w.p2, w.p1, &[item], &[], true);
    assert_eq!(w.g.players[w.p1].bonus_import[g], 2, "the deal's and the caravan's");
    assert_eq!(w.g.routes[r].slots[0].premium, low);
    w.consistent();
}

#[test]
fn two_routes_with_one_resource() {
    let mut w = world();
    let g = gems(&w);
    let (low, high) = (w.g.d.defines.trade_cargo_percent, w.g.d.defines.trade_cargo_new_percent);
    w.give(w.capital, g, 2);
    let older = cargo_route(&mut w, [Some(g), None]);
    let far2 = w.g.player_init_city(w.p1, 30, 10, true, true);
    w.g.routes[older].established -= 1;
    let u = w.caravan_at(w.capital);
    let younger = w.g.unit_establish_route(u, far2, [Some(g), None]).unwrap();
    w.arrive(younger);
    assert_eq!(w.g.routes[older].slots[0].premium, high, "the older route's");
    assert_eq!(w.g.routes[younger].slots[0].premium, low);
    w.consistent();
    // The older one's caravan dead: the younger has the high premium.
    let k = w.g.routes[older].slots[0].unit.unwrap();
    w.g.unit_kill(k, false, None);
    assert_eq!(w.g.routes[younger].slots[0].premium, high);
    w.consistent();
}

#[test]
fn plunder_includes_the_premium() {
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 1);
    let r = cargo_route(&mut w, [Some(g), None]);
    let pay = w.g.routes[r].slots[0].pay_x100;
    let total = expected_total(&w.g, r);
    assert!(pay > total - total / 2, "the premium is in the stored pay");
    let gold = w.g.players[w.p2].gold;
    let u = w.g.routes[r].slots[0].unit.unwrap();
    w.g.unit_kill(u, false, Some(w.p2));
    assert_eq!(w.g.players[w.p2].gold - gold, (w.g.d.defines.trade_plunder_turns * pay + 50) / 100);
    w.consistent();
}

#[test]
fn refit_keeps_growth_and_uses_the_unit() {
    let mut w = world();
    let g = gems(&w);
    let r = cargo_route(&mut w, [None, None]);
    w.g.routes[r].growth = 14;
    w.give(w.capital, g, 1);
    let u = w.caravan_at(w.port);
    let elsewhere = w.caravan_at(w.capital);
    assert_eq!(w.g.unit_refit_refusal(elsewhere, r, [Some(g), None]), Some(RouteRefusal::NotInOrigin));
    assert_eq!(w.g.unit_refit_refusal(u, RouteId(999), [Some(g), None]), Some(RouteRefusal::RouteGone));
    w.g.unit_refit_route(u, r, [Some(g), None]).unwrap();
    assert!(!w.g.units.contains(u), "used up");
    assert_eq!(w.g.routes[r].growth, 14);
    assert_eq!(w.g.routes[r].slots[0].cargo, Some(g));
    assert!(w.g.routes[r].slots[0].delivering.is_some());
    assert_eq!(w.logged("refitted"), 1);
    // Another refit may put the same copy on the other caravan (the route's own copies are free for it).
    let v = w.caravan_at(w.port);
    assert_eq!(w.g.unit_refit_refusal(v, r, [None, Some(g)]), None);
    w.consistent();
}

#[test]
fn the_import_follows_the_destination_to_its_new_owner() {
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 1);
    let r = cargo_route(&mut w, [Some(g), None]);
    w.open_borders(w.p0, w.p2, true);
    w.g.player_acquire_city(w.p2, w.far, false, true, true);
    let far = w.g.map.plots[w.at(22, 10)].plot_city.unwrap();
    assert_eq!(w.g.routes[r].destination, far);
    assert_eq!(w.g.players[w.p1].bonus_import[g], 0, "the old owner's import is undone");
    // Its caravan in the city went back to wait in the origin; it forms there at the owner's next
    // turn start and delivers to player 2.
    w.consistent();
    w.g.player_update_routes(w.p0);
    let delivering_to_p2 = w.g.routes[r].slots.iter().any(|s| s.delivering == Some((w.p2, g)));
    assert!(delivering_to_p2, "player 2 receives it");
    assert_eq!(w.g.players[w.p2].bonus_import[g], 1);
    w.consistent();
}

#[test]
fn cargo_survives_a_save() {
    let mut w = world();
    let g = gems(&w);
    w.give(w.capital, g, 1);
    let r = cargo_route(&mut w, [Some(g), None]);
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.routes[r], w.g.routes[r]);
    assert_eq!(h.players[w.p1].bonus_import[g], 1);
    assert!(h.trade_stale().is_empty());
    assert_eq!(h.save(), bytes);
}

// ---- Escorts (4.4) ----

use ffh_core::army::{EscortTarget, Station};

impl World {
    /// The owner's turn end for the trade: every unit of player 0 fresh, the caravans move, then
    /// their escorts.
    fn trade_turn(&mut self) {
        for u in self.g.players[self.p0].units.clone() {
            self.g.units[u].moves = 0;
        }
        self.g.player_move_caravans(self.p0);
        self.g.player_caravan_escorts(self.p0);
        self.consistent();
    }

    fn army_of(&mut self, name: &str, x: i32, y: i32) -> (ArmyId, UnitId) {
        let u = self.unit(self.p0, name, x, y);
        let a = self.g.army_form(self.p0, &[u]).expect("an army");
        (a, u)
    }

    fn escort(&mut self, a: ArmyId, r: RouteId, slot: u8) {
        self.g.army_set_station(self.p0, a, Station::Escort(EscortTarget::Caravan(r, slot))).expect("the escort order");
    }
}

#[test]
fn an_army_escorts_a_caravan_over_land_at_its_pace() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let c = w.g.routes[r].slots[0].unit.unwrap();
    let (army, man) = w.army_of("UNIT_AXEMAN", 2, 3);
    assert!(w.g.army_escort_caravans(army).contains(&(r, 0)), "a land path: a land army may escort it");
    w.escort(army, r, 0);
    assert_eq!(w.g.route_slot_escorts(r, 0), vec![army]);
    assert_eq!(w.g.escort_target_name(EscortTarget::Caravan(r, 0)), format!("Caravan: {} to {} (1)", w.g.cities[a].name, w.g.cities[b].name));
    assert_eq!(w.logged("escorts Caravan"), 1);
    let path = w.g.routes[r].path.clone();
    let moves = w.g.unit_max_moves(man) / w.g.d.defines.move_denominator;
    let free = (w.g.d.defines.caravan_move / w.g.d.defines.caravan_cost_land) as i32;
    for turn in 1..=3 {
        w.trade_turn();
        let at = path.iter().position(|&q| q == w.g.unit_plot(c)).unwrap() as i32;
        assert_eq!(w.g.unit_plot(man), w.g.unit_plot(c), "turn {turn}: together");
        assert_eq!(at, turn * moves.min(free), "turn {turn}: held to the army's pace");
    }
    // Apart: the caravan is not held; the army follows at the turn's end.
    let (other, _) = w.army_of("UNIT_AXEMAN", 2, 3);
    w.g.army_set_station(w.p0, army, Station::None).unwrap();
    let before = path.iter().position(|&q| q == w.g.unit_plot(c)).unwrap() as i32;
    w.escort(other, r, 0);
    w.trade_turn();
    let after = path.iter().position(|&q| q == w.g.unit_plot(c)).unwrap() as i32;
    assert_eq!(after - before, free.min(path.len() as i32 - 1 - before), "an escort elsewhere does not slow it");
    assert!(w.g.army_plot(other) != Some(w.g.city_plot(a)), "the escort walks toward it");
}

#[test]
fn a_fleet_escorts_the_water_leg_and_an_army_waits_at_the_shore() {
    let mut w = world();
    let across = w.g.player_init_city(w.p0, 22, 4, true, true);
    let r = w.establish(w.port, across);
    let c = w.g.routes[r].slots[0].unit.unwrap();
    let (fleet, _) = w.army_of("UNIT_TRIREME", 13, 10);
    let (army, _) = w.army_of("UNIT_AXEMAN", 12, 10);
    assert!(w.g.army_escort_caravans(fleet).contains(&(r, 0)), "water on the path: a fleet may escort it");
    w.escort(fleet, r, 0);
    w.escort(army, r, 0);
    let mut was_on_water = false;
    let mut landed_after_water = false;
    for _ in 0..30 {
        w.trade_turn();
        let here = w.g.unit_plot(c);
        let fp = w.g.army_plot(fleet).unwrap();
        let ap = w.g.army_plot(army).unwrap();
        assert!(w.g.plot_is_water(fp), "the fleet stays on water");
        assert!(!w.g.plot_is_water(ap), "the army never takes to boats");
        if w.g.plot_is_water(here) {
            was_on_water = true;
            assert_eq!(fp, here, "the fleet is with the caravan at sea");
        } else if was_on_water {
            landed_after_water = true;
        }
        if w.g.routes[r].slots[0].bound == ffh_core::trade::Leg::Back && landed_after_water && w.g.plot_is_water(here) {
            assert_eq!(fp, here, "it follows again on the way back");
            break;
        }
    }
    assert!(was_on_water && landed_after_water);
}

#[test]
fn an_escort_waits_for_a_lost_caravan_and_ends_with_the_route() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let (army, man) = w.army_of("UNIT_AXEMAN", 2, 3);
    w.escort(army, r, 0);
    w.trade_turn();
    let c = w.g.routes[r].slots[0].unit.unwrap();
    w.g.unit_kill(c, false, Some(w.p2));
    assert_eq!(w.logged("waits"), 1);
    let at = w.g.unit_plot(man);
    w.trade_turn();
    assert_eq!(w.g.unit_plot(man), at, "it waits where it is");
    assert_eq!(w.g.armies[army].station, Station::Escort(EscortTarget::Caravan(r, 0)), "the order stays");
    for _ in 0..w.g.d.defines.caravan_reform_turns {
        w.g.player_update_routes(w.p0);
    }
    let again = w.g.routes[r].slots[0].unit.expect("re-formed");
    w.trade_turn();
    let origin = w.g.city_plot(a);
    let (ox, oy) = w.g.map.xy(origin);
    let (mx, my) = w.g.map.xy(w.g.unit_plot(man));
    let (cx, cy) = w.g.map.xy(w.g.unit_plot(again));
    assert!(w.g.plot_distance(mx, my, cx, cy) < w.g.plot_distance(mx, my, ox, oy) + 2, "it heads for the re-formed caravan");
    assert!(w.g.route_dissolve(r));
    assert_eq!(w.g.armies[army].station, Station::None, "the route ended: so does the order");
    assert_eq!(w.logged("ended: the route ended"), 1);
}

#[test]
fn a_move_order_ends_an_escort() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let (army, _) = w.army_of("UNIT_AXEMAN", 2, 3);
    w.escort(army, r, 0);
    let grp = w.g.armies[army].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 2, 6, 0, false, false, None, None, None);
    assert_eq!(w.g.armies[army].station, Station::None);
    assert_eq!(w.logged("ended: a move order"), 1);
    // A land army escorts nothing else.
    let (other, _) = w.army_of("UNIT_AXEMAN", 4, 4);
    assert!(w.g.army_set_station(w.p0, other, Station::Escort(EscortTarget::Army(army))).is_err());
}

#[test]
fn an_attack_on_an_escorted_caravan_fights_the_army() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let (army, man) = w.army_of("UNIT_AXEMAN", 2, 3);
    w.escort(army, r, 0);
    w.trade_turn();
    let c = w.g.routes[r].slots[0].unit.unwrap();
    let here = w.g.unit_plot(c);
    assert_eq!(w.g.unit_plot(man), here);
    assert!(w.g.map.plots[here].plot_city.is_none());
    w.war(w.p0, w.p2);
    let (x, y) = w.g.map.xy(here);
    let champ = w.unit(w.p2, "UNIT_CHAMPION", x, y + 1);
    let before = w.g.battle_report_next;
    let grp = w.g.units[champ].group.unwrap();
    let (attacked, _) = w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    assert!(attacked);
    let report = w.g.battle_reports.iter().find(|rep| rep.id >= before).expect("a battle report");
    assert!(report.sides[1].committed >= 1);
    // One squad of the plot defends (docs/ENGAGEMENTS.md, "Squads": the best for the defender):
    // the army's axeman, not the caravan.
    let hurt = |u: UnitId| !w.g.units.contains(u) || w.g.units[u].damage > 0;
    assert!(hurt(man), "the army fought");
    assert!(!hurt(c), "the caravan stood in reserve");
    w.consistent();
}

#[test]
fn an_escort_survives_a_save() {
    let mut w = world();
    let across = w.g.player_init_city(w.p0, 22, 4, true, true);
    let r = w.establish(w.port, across);
    let (fleet, _) = w.army_of("UNIT_TRIREME", 13, 10);
    w.escort(fleet, r, 1);
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.armies[fleet].station, Station::Escort(EscortTarget::Caravan(r, 1)));
    assert_eq!(h.save(), bytes);
}

// ---- Retracing (3.5) ----

impl World {
    /// One turn start of player 0's routes, a game turn later.
    fn next_turn_start(&mut self) {
        self.g.setup.game_turn += 1;
        self.g.player_update_routes(self.p0);
        self.consistent();
    }
}

#[test]
fn a_quiet_turn_retraces_nothing() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let n = w.g.d.defines.trade_retrace_turns;
    let before = ffh_core::trade::route_path_calls();
    for _ in 1..n {
        w.next_turn_start();
    }
    assert_eq!(ffh_core::trade::route_path_calls(), before, "no search between retrace turns");
    w.next_turn_start();
    assert_eq!(ffh_core::trade::route_path_calls(), before + 1, "one on the retrace turn");
    assert!(w.g.route_path_valid(r));
    assert_eq!(w.g.routes[r].closed_since, -1);
}

#[test]
fn a_road_beside_the_path_is_taken_within_the_retrace_turns() {
    let mut w = world();
    let a = w.g.player_init_city(w.p0, 2, 3, true, true);
    let b = w.g.player_init_city(w.p0, 11, 3, true, true);
    let r = w.establish(a, b);
    let old = w.g.routes[r].path.clone();
    let road = w.g.d.t::<RouteType>("ROUTE_ROAD");
    for x in 2..=11 {
        for y in [4] {
            let q = w.at(x, y);
            w.g.plot_set_route_type(q, Some(road), false);
        }
    }
    let fresh = w.g.route_path(w.p0, a, b).unwrap();
    assert_ne!(fresh, old, "the road makes another path cheaper");
    let n = w.g.d.defines.trade_retrace_turns;
    let mut taken = None;
    for turn in 1..=n {
        w.next_turn_start();
        if w.g.routes[r].path == fresh {
            taken = Some(turn);
            break;
        }
        assert_eq!(w.g.routes[r].path, old, "kept while it can be walked");
    }
    assert_eq!(taken, Some(n), "taken on the retrace turn");
}

#[test]
fn a_blocked_path_is_retraced_at_the_next_turn_start() {
    let mut w = world();
    // Through player 2's land under open borders, then the borders close.
    for y in 1..=14 {
        let q = w.at(7, y);
        if !w.g.plot_is_water(q) {
            w.g.plot_set_owner(q, Some(w.p2), false, true);
        }
    }
    let east = w.g.player_init_city(w.p0, 10, 7, true, true);
    w.open_borders(w.p0, w.p2, true);
    let r = w.establish(w.capital, east);
    w.open_borders(w.p0, w.p2, false);
    assert!(!w.g.route_path_valid(r), "the stored path crosses closed borders");
    let before = ffh_core::trade::route_path_calls();
    w.next_turn_start();
    assert_eq!(ffh_core::trade::route_path_calls(), before + 1, "traced again at once");
    assert_eq!(w.g.routes[r].closed, Some(RouteRefusal::NoPath));
    assert_eq!(w.g.routes[r].closed_since, w.g.game_turn());
    // A closed route whose cheap tests pass is traced each turn start; it opens with the borders.
    w.open_borders(w.p0, w.p2, true);
    w.next_turn_start();
    assert!(w.g.routes[r].is_open());
    assert_eq!(w.g.routes[r].closed_since, -1);
}

#[test]
fn a_sea_closing_on_the_path_retraces_it() {
    let mut w = world();
    let across = w.g.player_init_city(w.p0, 22, 4, true, true);
    let r = w.establish(w.port, across);
    let old = w.g.routes[r].path.clone();
    // Player 2 closes the Gap to player 0.
    w.war(w.p0, w.p2);
    let ship = w.unit(w.p2, "UNIT_TRIREME", 17, 1);
    assert!(w.g.unit_plunder(ship));
    let gap = w.g.map.plots[w.at(17, 1)].sea.unwrap();
    assert!(w.g.sea_is_closed_to(gap, w.team(w.p0)));
    assert!(!w.g.route_path_valid(r));
    w.next_turn_start();
    let route = &w.g.routes[r];
    assert!(route.closed == Some(RouteRefusal::NoPath) || (route.path != old && w.g.route_path_valid(r)), "retraced: another way or no path");
}
