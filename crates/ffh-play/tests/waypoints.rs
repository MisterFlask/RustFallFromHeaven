//! Waypoints through the human's command layer and views (docs/ARMIES.md, the protocol in
//! docs/HUMAN.md), on `ffh-play stage waypoints`: `city_waypoint` sets, refuses and clears a
//! city's waypoint; the city screen's, the Armies screen's and the units' waypoint fields; a unit
//! on its way takes orders, and an order ends its waypoint.

mod common;

use std::sync::Arc;

use ffh_core::army::Waypoint;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan, WaypointTo};
use ffh_play::json::Json;
use ffh_play::stage::StagedWaypoints;
use ffh_play::view::{self, Viewer};

/// The staged save, loaded: its session and what the stage named.
fn staged(name: &str) -> (ffh_play::Session, StagedWaypoints, common::TempDir) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::waypoints(&dir.0).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
    let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    (s, st, dir)
}

/// Validates `cmd` for the human and applies it.
fn run(g: &mut Game, h: PlayerId, cmd: &Command) -> Result<(), CmdErr> {
    match command::validate(g, h, cmd, &Ctx { events: &[] })? {
        Plan::Apply(a) => {
            command::apply(g, h, &a);
            Ok(())
        }
        other => panic!("{cmd:?}: {other:?}"),
    }
}

fn refused(r: Result<(), CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(s)) | Err(CmdErr::Invalid(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

/// The city screen's `waypoint` of city `c`.
fn city_wp(g: &Game, h: PlayerId, c: CityId) -> Json {
    view::city::city(&Viewer::new(g, h, false), c).unwrap().get("waypoint").cloned().unwrap()
}

/// The Armies screen's entry of army `a`.
fn screen_army(g: &Game, h: PlayerId, a: ArmyId) -> Json {
    let s = view::armies::screen(&Viewer::new(g, h, false));
    arr(&s, "armies").into_iter().find(|j| int(j, "id") == a.0 as i64).expect("the army on the screen")
}

#[test]
fn the_command_sets_refuses_and_clears_a_city_waypoint() {
    let (mut s, st, _dir) = staged("wp-command");
    let h = s.human;
    let g = &mut s.g;
    // The capital has none; its choices name the army (not a fleet: there is none).
    let wp = city_wp(g, h, st.capital);
    assert_eq!(text(&wp, "kind"), "none", "{wp}");
    assert_eq!(wp.get("name"), Some(&Json::Null), "{wp}");
    let choices = arr(&wp, "choices");
    assert!(choices.iter().any(|c| int(c, "army") == st.army.0 as i64), "{wp}");
    // To the army.
    run(g, h, &Command::CityWaypoint { city: st.capital, to: Some(WaypointTo::Army(st.army)) }).unwrap();
    assert_eq!(g.cities[st.capital].waypoint, Some(Waypoint::Army(st.army)));
    let name = g.army_display_name(st.army);
    let wp = city_wp(g, h, st.capital);
    assert_eq!((text(&wp, "kind"), text(&wp, "name")), ("army", name.as_str()), "{wp}");
    assert_eq!(int(&wp, "army"), st.army.0 as i64);
    // To a plot (revealed: the army's).
    let (x, y) = (g.units[g.army_units(st.army)[0]].x, g.units[g.army_units(st.army)[0]].y);
    run(g, h, &Command::CityWaypoint { city: st.capital, to: Some(WaypointTo::Plot(x, y)) }).unwrap();
    let wp = city_wp(g, h, st.capital);
    assert_eq!((text(&wp, "kind"), text(&wp, "name")), ("plot", format!("plot {x}, {y}").as_str()), "{wp}");
    assert_eq!((int(&wp, "x"), int(&wp, "y")), (x as i64, y as i64));
    // Refused: no such army, a plot off the map, a plot not revealed, another's city.
    assert!(refused(run(g, h, &Command::CityWaypoint { city: st.capital, to: Some(WaypointTo::Army(ArmyId(9999))) })).contains("no such army"));
    assert!(matches!(run(g, h, &Command::CityWaypoint { city: st.capital, to: Some(WaypointTo::Plot(-1, 0)) }), Err(CmdErr::Invalid(_))));
    let team = g.players[h].team;
    if let Some(dark) = g.map.plot_ids().find(|&q| !g.map.plots[q].revealed[team]) {
        let (dx, dy) = g.map.xy(dark);
        assert!(refused(run(g, h, &Command::CityWaypoint { city: st.capital, to: Some(WaypointTo::Plot(dx, dy)) })).contains("not revealed"));
    }
    let theirs = g.cities.iter().map(|(c, _)| c).find(|&c| g.cities[c].owner != h);
    if let Some(theirs) = theirs {
        assert!(matches!(run(g, h, &Command::CityWaypoint { city: theirs, to: None }), Err(CmdErr::NotOwner)));
    }
    // Cleared.
    run(g, h, &Command::CityWaypoint { city: st.capital, to: None }).unwrap();
    assert_eq!(g.cities[st.capital].waypoint, None);
    assert_eq!(text(&city_wp(g, h, st.capital), "kind"), "none");
}

#[test]
fn the_protocol_round_trips_city_waypoint() {
    let d = ffh_data::global();
    for (line, want) in [
        (r#"{"c":"city_waypoint","city":2,"to":{"army":3}}"#, Command::CityWaypoint { city: CityId(2), to: Some(WaypointTo::Army(ArmyId(3))) }),
        (r#"{"c":"city_waypoint","city":2,"to":{"plot":[14,9]}}"#, Command::CityWaypoint { city: CityId(2), to: Some(WaypointTo::Plot(14, 9)) }),
        (r#"{"c":"city_waypoint","city":2,"to":null}"#, Command::CityWaypoint { city: CityId(2), to: None }),
    ] {
        assert_eq!(Command::from_json(d, &Json::parse(line).unwrap()).unwrap(), want, "{line}");
    }
    let bad = Json::parse(r#"{"c":"city_waypoint","city":2,"to":{"garrison":1}}"#).unwrap();
    assert!(matches!(Command::from_json(d, &bad), Err(CmdErr::Invalid(_))));
}

#[test]
fn the_armies_screen_lists_an_armys_waypoint_cities_and_units_on_their_way() {
    let (s, st, _dir) = staged("wp-screen");
    let (g, h) = (&s.g, s.human);
    let e = screen_army(g, h, st.army);
    let cities = arr(&e, "waypoint_cities");
    assert_eq!(cities.len(), 1, "{e}");
    assert_eq!(int(&cities[0], "city"), st.city.0 as i64, "{e}");
    assert!(cities[0].get("name").and_then(Json::as_str).is_some(), "{e}");
    assert_eq!(arr(&e, "waypoint_units"), vec![Json::from(st.unit)], "{e}");
    // The view's army has them too (the unit panel's army line).
    let va = view::armies::armies(&Viewer::new(g, h, false));
    let a = va.as_arr().unwrap().iter().find(|j| int(j, "id") == st.army.0 as i64).unwrap().clone();
    assert_eq!(arr(&a, "waypoint_units"), vec![Json::from(st.unit)]);
    // Each garrison entry has its city's waypoint and whether it is a port.
    let scr = view::armies::screen(&Viewer::new(g, h, false));
    let gar = arr(&scr, "garrisons").into_iter().find(|j| int(j, "city") == st.city.0 as i64).unwrap();
    assert_eq!(text(gar.get("waypoint").unwrap(), "kind"), "army", "{gar}");
    assert!(gar.get("port").and_then(Json::as_bool).is_some(), "{gar}");
}

#[test]
fn a_unit_on_its_way_shows_its_waypoint_takes_orders_and_an_order_ends_it() {
    let (mut s, st, _dir) = staged("wp-unit");
    let h = s.human;
    let g = &mut s.g;
    let u = st.unit;
    let name = g.army_display_name(st.army);
    // On its way: in no army and no garrison; its view names the waypoint.
    assert_eq!(g.units[u].waypoint, Some(Waypoint::Army(st.army)));
    assert_eq!(g.units[u].army, None);
    let uj = view::unit_json(&Viewer::new(g, h, false), u);
    assert_eq!(uj.get("waypoint").and_then(Json::as_str), Some(name.as_str()), "{uj}");
    // The unit panel: the line with its turns, and its orders.
    let grp = view::units::group(&Viewer::new(g, h, false), &[u]).unwrap();
    let detail = arr(&grp, "units")[0].clone();
    let line = detail.get("waypoint_line").cloned().unwrap();
    assert!(text(&line, "text").starts_with(&format!("Waypoint: {name} (")), "{line}");
    assert!(int(&line, "eta") > 0, "{line}");
    assert!(!arr(&grp, "actions").is_empty(), "{grp}");
    // Its group is selectable like any other (it has its go-to, so it is not waiting).
    let gid = g.units[u].group.unwrap();
    assert!(!ffh_play::decision::is_ai_group(g, gid));
    // An order ends the waypoint.
    // (A move to a plot next to it: a warrior just trained may not fortify or sleep yet.)
    let (x, y) = (g.units[u].x, g.units[u].y);
    let next = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)]
        .into_iter()
        .map(|(dx, dy)| (x + dx, y + dy))
        .find(|&(nx, ny)| g.map.plot_xy(nx, ny).is_some_and(|q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].plot_city.is_none()))
        .expect("an empty plot next to the unit");
    run(g, h, &Command::Move { units: vec![u], x: next.0, y: next.1, append: false, declare_war: None, confirm_end_siege: false }).unwrap();
    assert_eq!(g.units[u].waypoint, None);
    let grp = view::units::group(&Viewer::new(g, h, false), &[u]).unwrap();
    assert_eq!(arr(&grp, "units")[0].get("waypoint_line"), Some(&Json::Null));
    assert!(g.army_waypoint_units(st.army).is_empty());
}

// ---------------------------------------------------------------------------------------------
// Real turns: the human's own turn loop (commands, end turn, the auto moves), on a game that
// cannot end (`common::STEADY`); every situation is built by hand.
// ---------------------------------------------------------------------------------------------

use ffh_play::command::CmdOk;
use ffh_play::{Phase, Session, SessionOptions, Stop};

/// A steady duel at the human's first control point, its capital founded (the Bannor: their
/// Warrior is the Warrior). Returns the session and the capital.
fn turn_session(seed: u32) -> (Session, CityId) {
    let d = ffh_data::global();
    let mut ng = common::steady_duel(seed);
    ng.civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    let mut s = Session::new_game(&ng, Arc::new(ffh_play::human::AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let settler = s.g.players[s.human].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).expect("a settler");
    ok(&mut s, Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false, confirm_end_siege: false });
    let capital = s.g.players[s.human].cities[0];
    (s, capital)
}

/// Runs `c` through the session as the client does, then the auto moves it leaves due.
fn ok(s: &mut Session, c: Command) {
    match s.execute(&c) {
        Ok(CmdOk::Applied { .. }) => {}
        other => panic!("{c:?} -> {other:?}"),
    }
    if s.auto_moves_due() {
        assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    }
}

/// Ends the human's turn (forced, as the client's End turn) and runs to its next control point.
fn end_turn(s: &mut Session) {
    if s.phase() != Phase::Human(s.human) {
        assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    }
    ok(s, Command::EndTurn { force: true });
    assert_eq!(s.advance(&mut |_| {}), Stop::Human, "turn {}", s.g.game_turn());
}

/// Land plots of the capital's landmass `dist` plots from it, empty, no city, passable, nearest
/// first (ties by position).
fn land_at(g: &Game, c: CityId, dist: std::ops::RangeInclusive<i32>) -> Vec<(i32, i32)> {
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let area = g.map.plots[g.city_plot(c)].area;
    let mut out: Vec<(i32, i32)> = g
        .map
        .plot_ids()
        .filter(|&q| g.map.plots[q].area == area && !g.plot_is_impassable(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].plot_city.is_none())
        .map(|q| g.map.xy(q))
        .filter(|&(x, y)| dist.contains(&g.plot_distance(x, y, cx, cy)))
        .collect();
    out.sort_by_key(|&(x, y)| (g.plot_distance(x, y, cx, cy), y, x));
    out
}

/// An army of two warriors formed by command on (x, y).
fn field_army(s: &mut Session, x: i32, y: i32) -> ArmyId {
    let h = s.human;
    let w = s.g.d.t::<UnitType>("UNIT_WARRIOR");
    let units: Vec<UnitId> = (0..2).map(|_| s.g.player_init_unit(h, w, x, y, None, None)).collect();
    ok(s, Command::ArmyForm { units: units.clone() });
    s.g.units[units[0]].army.expect("an army")
}

/// City `c` is given a Warrior order by command, its production complete: the city trains it as
/// the turn ends. Returns the new unit, at the human's next control point.
fn train_warrior(s: &mut Session, c: CityId) -> UnitId {
    let h = s.human;
    let w = s.g.d.t::<UnitType>("UNIT_WARRIOR");
    ok(s, Command::PushOrder { city: c, order: OrderType::ORDER_TRAIN, data: w.idx() as i32, alt: false, shift: false, ctrl: true });
    s.g.cities[c].unit_production[w] = s.g.city_production_needed_unit(c, w);
    let before = s.g.players[h].units.clone();
    end_turn(s);
    *s.g.players[h].units.iter().find(|u| !before.contains(u) && s.g.units[**u].unit_type == w).expect("the new warrior")
}

fn at(g: &Game, u: UnitId) -> (i32, i32) {
    (g.units[u].x, g.units[u].y)
}

/// Whether unit `u`'s group is among the human's groups waiting for orders.
fn waits(s: &Session, u: UnitId) -> bool {
    s.g.units[u].group.is_some_and(|grp| s.pending().ready_groups.contains(&grp))
}

/// Ends turns while `u` is on its way (at most `max`), each one asserting that it moved without
/// an order and is not waiting for orders; `between` runs after each turn (its index). Returns
/// the turns it took.
fn walk(s: &mut Session, u: UnitId, max: usize, mut between: impl FnMut(&mut Session, usize)) -> usize {
    for i in 0..max {
        if s.g.units[u].waypoint.is_none() {
            return i;
        }
        assert!(!waits(s, u), "turn {}: the unit on its way waits for orders", s.g.game_turn());
        let from = at(&s.g, u);
        end_turn(s);
        if s.g.units[u].waypoint.is_some() {
            assert_ne!(at(&s.g, u), from, "turn {}: the unit on its way did not move", s.g.game_turn());
        }
        between(s, i);
    }
    panic!("the unit did not arrive within {max} turns (at {:?}, waypoint {:?})", at(&s.g, u), s.g.units[u].waypoint);
}

#[test]
fn a_humans_unit_walks_to_its_army_over_real_turns_and_joins_it() {
    let (mut s, capital) = turn_session(3);
    let &(ax, ay) = land_at(&s.g, capital, 5..=6).first().expect("a plot for the army");
    let a = field_army(&mut s, ax, ay);
    let name = s.g.armies[a].name.clone();
    ok(&mut s, Command::CityWaypoint { city: capital, to: Some(WaypointTo::Army(a)) });
    let u = train_warrior(&mut s, capital);
    assert_eq!(s.g.units[u].waypoint, Some(Waypoint::Army(a)), "the new warrior sets out");
    let n = walk(&mut s, u, 12, |_, _| {});
    assert!(n >= 3, "a walk of {n} turns over 5 plots");
    assert_eq!(s.g.units[u].army, Some(a), "it joined the army");
    assert_eq!(s.g.units[u].waypoint, None);
    assert!(s.g.armies.contains(a) && s.g.armies[a].name == name, "the army keeps its id and name");
    assert_eq!(s.g.army_units(a).len(), 3);
}

#[test]
fn a_unit_follows_its_army_when_the_player_moves_it_and_still_joins() {
    let (mut s, capital) = turn_session(3);
    let &(ax, ay) = land_at(&s.g, capital, 6..=7).first().expect("a plot for the army");
    let a = field_army(&mut s, ax, ay);
    let name = s.g.armies[a].name.clone();
    ok(&mut s, Command::CityWaypoint { city: capital, to: Some(WaypointTo::Army(a)) });
    let u = train_warrior(&mut s, capital);
    assert_eq!(s.g.units[u].waypoint, Some(Waypoint::Army(a)));
    // After the first turn on its way, the player moves the army one plot on.
    let mut moved_to = None;
    let n = walk(&mut s, u, 14, |s, i| {
        if i != 0 || s.g.units[u].waypoint.is_none() {
            return;
        }
        let units = s.g.army_units(a);
        let (x, y) = at(&s.g, units[0]);
        let area = s.g.map.plots[s.g.unit_plot(units[0])].area;
        let next = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)]
            .into_iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .find(|&(nx, ny)| s.g.map.plot_xy(nx, ny).is_some_and(|q| s.g.map.plots[q].area == area && !s.g.plot_is_impassable(q) && s.g.map.plots[q].units.is_empty() && s.g.map.plots[q].plot_city.is_none()))
            .expect("a plot next to the army");
        ok(s, Command::Move { units, x: next.0, y: next.1, append: false, declare_war: None, confirm_end_siege: false });
        moved_to = Some(next);
    });
    assert!(n >= 3, "a walk of {n} turns over 6 plots");
    let to = moved_to.expect("the army was moved while the unit was on its way");
    assert_eq!(at(&s.g, s.g.army_units(a)[0]), to);
    assert_eq!(s.g.units[u].army, Some(a), "it followed and joined the army");
    assert_eq!(at(&s.g, u), to);
    assert_eq!(s.g.units[u].waypoint, None);
    assert_eq!(s.g.armies[a].name, name);
}

#[test]
fn a_unit_walks_to_a_plot_waypoint_and_then_waits_for_orders() {
    let (mut s, capital) = turn_session(3);
    let &(px, py) = land_at(&s.g, capital, 4..=5).first().expect("a plot for the waypoint");
    ok(&mut s, Command::CityWaypoint { city: capital, to: Some(WaypointTo::Plot(px, py)) });
    let u = train_warrior(&mut s, capital);
    assert_eq!(s.g.units[u].waypoint, Some(Waypoint::Plot(s.g.map.plot_xy(px, py).unwrap())));
    let n = walk(&mut s, u, 10, |_, _| {});
    assert!(n >= 2, "a walk of {n} turns over 4 plots");
    assert_eq!(at(&s.g, u), (px, py), "it stands on the plot");
    assert_eq!(s.g.units[u].waypoint, None);
    // No army there: it stands alone (a one-unit army by the sync) and waits for orders.
    let a = s.g.units[u].army.expect("a one-unit army");
    assert_eq!(s.g.army_units(a), vec![u]);
    assert!(waits(&s, u), "it waits for orders on arrival");
}
