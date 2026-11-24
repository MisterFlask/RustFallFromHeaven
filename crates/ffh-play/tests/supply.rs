//! Supply lines (docs/SUPPLY.md) through the human's protocol (docs/HUMAN.md): each army's supply
//! in the view (its state, its line, its words), the path query's supply readout and the lines of
//! other players' armies the human sees, both as the human knows them (docs/SUPPLY.md, "Second
//! pass", A: over revealed plots, around the units it sees), while its own armies' lines stay the
//! truth. Built on `ffh-play stage supply`.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::{Session, SessionOptions};

fn load(save: &std::path::Path) -> Session {
    let (bytes, sidecar) = ffh_play::save::read_bundle(save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap()
}

fn q(s: &Session, line: &str) -> Json {
    s.query_json(&Json::parse(line).unwrap()).unwrap_or_else(|e| panic!("{line}: {e:?}"))
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("no array `{key}` in {j:?}")).to_vec()
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("no integer `{key}` in {j:?}"))
}

fn text(j: &Json, key: &str) -> String {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("no string `{key}` in {j:?}")).to_string()
}

/// The view's army `a`.
fn view_army(s: &Session, a: ArmyId) -> Json {
    let view = q(s, r#"{"q":"view"}"#);
    arr(&view, "armies").into_iter().find(|j| int(j, "id") == a.0 as i64).expect("the army in the view")
}

/// An AI with a city, and a new army of two of its warriors on a free land plot next to that
/// city: `(ai, army, (x, y))`.
fn ai_army(g: &mut ffh_core::game::Game, h: PlayerId) -> (PlayerId, ArmyId, (i32, i32)) {
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    let ai = g.player_ids().find(|&p| p != h && !g.player_is_barbarian(p) && !g.players[p].cities.is_empty()).expect("an AI with a city");
    let city = g.players[ai].cities[0];
    let (cx, cy) = (g.cities[city].x, g.cities[city].y);
    let q = DIRS.iter().filter_map(|&(dx, dy)| free(g, cx + dx, cy + dy)).next().expect("land next to the AI city");
    let (x, y) = g.map.xy(q);
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(ai, w, x, y, None, None)).collect();
    let army = g.army_form(ai, &units).expect("an AI army");
    (ai, army, (x, y))
}

const DIRS: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)];

/// A free land plot at (x, y): no water, peak, city or unit.
fn free(g: &ffh_core::game::Game, x: i32, y: i32) -> Option<PlotIdx> {
    g.map.plot_xy(x, y).filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none() && g.map.plots[q].units.is_empty())
}

#[test]
fn each_army_s_supply_is_in_the_view_with_its_line_and_words() {
    let dir = common::TempDir::new("supply-view");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let s = load(&st.save);
    let d = ffh_data::global();
    let budget = d.defines.supply_line_budget as i64;

    // The supplied army: its line from its plot to the source, within the budget.
    let army = view_army(&s, st.supplied);
    let sup = army.get("supply").expect("supply");
    assert_eq!(text(sup, "state"), "supplied");
    let line = sup.get("line").expect("a line");
    let path = arr(line, "path");
    assert!(path.len() >= 3, "{line:?}");
    let xy = |p: &Json| (p.as_arr().unwrap()[0].as_i64().unwrap(), p.as_arr().unwrap()[1].as_i64().unwrap());
    assert_eq!(xy(&path[0]), (st.x as i64, st.y as i64), "the line starts at the army");
    let cost = int(line, "cost");
    assert!(cost > 0 && cost <= budget && int(line, "budget") == budget, "{line:?}");
    assert_eq!(text(line, "source_kind"), "city");
    assert_eq!(text(sup, "text"), format!("Supplied: {cost} of {budget}, from {}.", text(line, "source")));
    assert!(army.get("in_supply").and_then(Json::as_bool).unwrap());

    // The hungry army: no line, Hunger turn 1, its words.
    let army = view_army(&s, st.hungry);
    let sup = army.get("supply").expect("supply");
    assert_eq!(text(sup, "state"), "hunger");
    assert!(sup.get("line").is_some_and(Json::is_null), "{sup:?}");
    let of = d.defines.supply_hunger_turns as i64;
    assert_eq!((int(sup, "turn"), int(sup, "of"), int(sup, "turns_to_starvation")), (1, of, of));
    assert_eq!(text(sup, "text"), format!("Hunger: turn 1 of {of}. No healing, no recruits."));
    assert!(!army.get("in_supply").and_then(Json::as_bool).unwrap());

    // The path query's readout, for the units of one army: supplied on its own plot, not where
    // the hungry army stands.
    let units: Vec<String> = s.g.army_units(st.supplied).iter().map(|u| u.0.to_string()).collect();
    let at = |x: i32, y: i32| q(&s, &format!(r#"{{"q":"path","units":[{}],"x":{x},"y":{y}}}"#, units.join(",")));
    let here = at(st.x, st.y);
    let sup = here.get("supply").expect("supply");
    assert!(sup.get("ok").and_then(Json::as_bool).unwrap() && int(sup, "cost") == cost && int(sup, "budget") == budget, "{sup:?}");
    let there = at(st.hx, st.hy);
    let sup = there.get("supply").expect("supply");
    assert_eq!(sup.get("ok").and_then(Json::as_bool), Some(false), "{sup:?}");
    assert!(sup.get("cost").is_some_and(Json::is_null));
    // In the human's territory: supplied with no line, and the readout says so.
    let sup = at(st.ox, st.oy);
    let sup = sup.get("supply").expect("supply");
    assert_eq!((sup.get("ok").and_then(Json::as_bool), sup.get("home").and_then(Json::as_bool)), (Some(true), Some(true)), "{sup:?}");

    // The army in the human's territory: supplied, no line drawn, its words.
    let army = view_army(&s, st.home);
    let sup = army.get("supply").expect("supply");
    assert_eq!(text(sup, "state"), "supplied");
    assert!(sup.get("line").is_some_and(Json::is_null), "{sup:?}");
    assert_eq!(sup.get("home").and_then(Json::as_bool), Some(true));
    assert_eq!(text(sup, "text"), "Supplied: in your territory, no supply line needed.");
}

/// The lines of other players' armies: an AI army the human sees next to the AI's city has its
/// line, every point of it a plot the human has revealed; an AI army the human does not see has
/// none; the view never names their counters or sources.
#[test]
fn the_lines_of_other_players_armies_the_human_sees_are_on_its_map() {
    let dir = common::TempDir::new("supply-foreign");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let h = s.human;
    let g = &mut s.g;
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    let (ai, ai_army, (ax, ay)) = ai_army(g, h);
    // A human warrior next to the AI army: it sees it.
    let wq = DIRS.iter().filter_map(|&(dx, dy)| free(g, ax + dx, ay + dy)).next().expect("land next to the AI army");
    let (wx, wy) = g.map.xy(wq);
    g.player_init_unit(h, w, wx, wy, None, None);
    let team = g.players[h].team;
    let plot = g.map.plot_xy(ax, ay).unwrap();
    // Outside its borders (within them an army needs no line, and none is drawn).
    g.plot_set_owner(plot, None, false, false);
    assert!(!g.plot_supply_home(ai, plot));
    assert!(g.plot_is_visible(plot, team), "the human sees the AI army");
    assert!(g.army_supply_line(ai_army).is_some(), "the AI army next to its city is supplied");

    let view = q(&s, r#"{"q":"view"}"#);
    let lines = arr(&view, "supply_lines");
    let mine = lines.iter().find(|l| (int(l, "x"), int(l, "y")) == (ax as i64, ay as i64)).expect("the AI army's line");
    assert_eq!(int(mine, "owner"), ai.0 as i64);
    let mut keys: Vec<&str> = mine.as_obj().unwrap().iter().map(|(k, _)| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["owner", "segments", "x", "y"], "nothing but the owner, the plot and the revealed segments");
    let segments = arr(mine, "segments");
    assert!(!segments.is_empty());
    for seg in &segments {
        for p in seg.as_arr().unwrap() {
            let (x, y) = (p.as_arr().unwrap()[0].as_i64().unwrap() as i32, p.as_arr().unwrap()[1].as_i64().unwrap() as i32);
            assert!(s.g.map.plots[s.g.map.plot_xy(x, y).unwrap()].revealed[team], "({x}, {y}) is on the human's map");
        }
    }
    // Every listed army stands on a plot the human sees.
    for l in &lines {
        let q = s.g.map.plot_xy(int(l, "x") as i32, int(l, "y") as i32).unwrap();
        assert!(s.g.plot_is_visible(q, team), "{l:?}");
    }
}

/// Barbarian warriors (at war with everyone) on every free land plot next to `plot`: the plots.
fn ring_of_barbarians(g: &mut ffh_core::game::Game, plot: PlotIdx) -> Vec<PlotIdx> {
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    let barb = g.barbarian_player();
    let (x, y) = g.map.xy(plot);
    let ring: Vec<PlotIdx> = DIRS.iter().filter_map(|&(dx, dy)| free(g, x + dx, y + dy)).collect();
    for &q in &ring {
        let (qx, qy) = g.map.xy(q);
        g.player_init_unit(barb, w, qx, qy, None, None);
    }
    ring
}

/// Whether team `t` sees plot `q` now: forced to `on` (the test builds what the human sees).
fn set_seen(g: &mut ffh_core::game::Game, q: PlotIdx, t: TeamId, on: bool) {
    g.map.plots[q].visibility_count[t] = on as i32;
    g.map.plots[q].stolen_visibility_count[t] = 0;
}

/// The readout before a move and the human's own army: hidden enemy units that close every way
/// out of the army's plot do not change the readout (it answers from what the human sees), while
/// the army's own line, the truth, is gone; once the human sees them, the readout says no line.
#[test]
fn the_readout_answers_from_what_the_human_sees_and_the_own_line_is_the_truth() {
    let dir = common::TempDir::new("supply-known-readout");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let h = s.human;
    let team = s.g.players[h].team;
    let plot = s.g.map.plot_xy(st.x, st.y).unwrap();
    let before = s.g.plot_supply_line_known(h, plot, team).expect("a known line before");
    let ring = ring_of_barbarians(&mut s.g, plot);
    assert!(!ring.is_empty());
    for &q in &ring {
        set_seen(&mut s.g, q, team, false);
    }
    assert!(s.g.army_supply_line(st.supplied).is_none(), "the enemy units close every way out");

    let units: Vec<String> = s.g.army_units(st.supplied).iter().map(|u| u.0.to_string()).collect();
    let readout = |s: &Session| {
        let j = q(s, &format!(r#"{{"q":"path","units":[{}],"x":{},"y":{}}}"#, units.join(","), st.x, st.y));
        j.get("supply").expect("supply").clone()
    };
    // Hidden: the readout is the line the human can work out, as before the units came.
    let sup = readout(&s);
    assert_eq!(sup.get("ok").and_then(Json::as_bool), Some(true), "{sup:?}");
    assert_eq!(int(&sup, "cost"), before.cost as i64);
    // The army's own state and line are the truth: no line.
    let army = view_army(&s, st.supplied);
    let own = army.get("supply").expect("supply");
    assert!(own.get("line").is_some_and(Json::is_null), "{own:?}");
    assert_eq!(text(own, "text"), "No supply line: Hunger from your next turn.");

    // Seen: the readout has no line either.
    for &q in &ring {
        set_seen(&mut s.g, q, team, true);
    }
    let sup = readout(&s);
    assert_eq!(sup.get("ok").and_then(Json::as_bool), Some(false), "{sup:?}");
    assert!(sup.get("cost").is_some_and(Json::is_null));
}

/// Another player's army: its drawn line is the one the human can work out. Hidden enemy units
/// around it do not bend it (it is drawn though the army's true line is cut); one the human sees
/// on the line bends it; all of them seen, the army has no entry.
#[test]
fn other_players_lines_bend_only_around_the_units_the_human_sees() {
    let dir = common::TempDir::new("supply-known-foreign");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let h = s.human;
    let team = s.g.players[h].team;
    let g = &mut s.g;
    let ai = g.player_ids().find(|&p| p != h && !g.player_is_barbarian(p) && !g.players[p].cities.is_empty()).expect("an AI with a city");
    let city = g.players[ai].cities[0];
    let (cx, cy) = (g.cities[city].x, g.cities[city].y);
    // The human has revealed the AI city's surroundings (its map, not its sight).
    for q in g.map.plot_ids().collect::<Vec<_>>() {
        let (x, y) = g.map.xy(q);
        if g.plot_distance(x, y, cx, cy) <= 4 {
            g.plot_set_revealed(q, team, true, false, None, false);
        }
    }
    assert!(g.city_is_revealed(city, team, false));
    // An AI army two plots from its city, its neighbours free land or nothing to stand on.
    let calm = |g: &ffh_core::game::Game, q: PlotIdx| {
        let (x, y) = g.map.xy(q);
        DIRS.iter().all(|&(dx, dy)| g.map.plot_xy(x + dx, y + dy).is_none_or(|n| g.plot_is_water(n) || g.plot_is_impassable(n) || free(g, x + dx, y + dy).is_some()))
    };
    let plot = g
        .map
        .plot_ids()
        .find(|&q| {
            let (x, y) = g.map.xy(q);
            g.plot_distance(x, y, cx, cy) == 2 && free(g, x, y).is_some() && calm(g, q) && g.plot_supply_line_known(ai, q, team).is_some_and(|l| l.path.len() >= 3)
        })
        .expect("a plot two from the AI city with a line");
    let (ax, ay) = g.map.xy(plot);
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(ai, w, ax, ay, None, None)).collect();
    let army = g.army_form(ai, &units).expect("an AI army");
    let ring = ring_of_barbarians(g, plot);
    for &q in &ring {
        set_seen(g, q, team, false);
    }
    set_seen(g, plot, team, true);
    assert!(g.army_supply_line(army).is_none(), "the enemy units close every way out");
    let known = g.plot_supply_line_known(ai, plot, team).expect("the known line ignores the hidden units");

    let entry = |s: &Session| {
        let view = q(s, r#"{"q":"view"}"#);
        arr(&view, "supply_lines").into_iter().find(|l| (int(l, "x"), int(l, "y")) == (ax as i64, ay as i64))
    };
    let path_of = |l: &Json| -> Vec<(i32, i32)> {
        let segs = arr(l, "segments");
        assert_eq!(segs.len(), 1, "one run: the known line is on the human's map");
        segs[0].as_arr().unwrap().iter().map(|p| (p.as_arr().unwrap()[0].as_i64().unwrap() as i32, p.as_arr().unwrap()[1].as_i64().unwrap() as i32)).collect()
    };
    // Hidden: drawn, the known line, through a plot a hidden enemy holds.
    let l = entry(&s).expect("the AI army's line is drawn");
    let drawn = path_of(&l);
    assert_eq!(drawn, known.path.iter().map(|&q| s.g.map.xy(q)).collect::<Vec<_>>());
    let step = s.g.map.plot_xy(drawn[1].0, drawn[1].1).unwrap();
    assert!(ring.contains(&step), "the line leaves through a hidden enemy's plot");

    // The one on the line seen: the drawn line (if any) goes around it.
    set_seen(&mut s.g, step, team, true);
    if let Some(l) = entry(&s) {
        assert!(!path_of(&l).contains(&drawn[1]), "the line bends around the seen unit");
    }
    // All seen: no known line, no entry.
    for &q in &ring {
        set_seen(&mut s.g, q, team, true);
    }
    assert!(entry(&s).is_none(), "an army with no known line has no entry");
}

/// An army in an invested city of the human's (docs/SUPPLY.md, 3.2): the army says it is held,
/// and the path query refuses a move out of the city with the same words.
#[test]
fn an_army_held_in_an_invested_city_says_so_in_the_panel_and_the_path() {
    let dir = common::TempDir::new("supply-held");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let h = s.human;
    let g = &mut s.g;
    let city = g.city_plot(st.capital);
    let (cx, cy) = g.map.xy(city);
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, w, cx, cy, None, None)).collect();
    let a = g.army_form(h, &units).expect("an army");
    let ring = ring_of_barbarians(g, city);
    assert!(ring.len() >= 3 && g.city_is_invested(st.capital), "invested");
    let name = ffh_play::view::city_name(g, st.capital);
    let want = format!("Besieged in {name}: moves only to attack until the siege lifts.");
    let army = view_army(&s, a);
    assert_eq!(army.get("held").and_then(Json::as_str), Some(want.as_str()), "{army:?}");
    // A move out of the city (two plots away, past the ring): refused, with the reason.
    let (tx, ty) = (cx + 2, cy);
    let ids: Vec<String> = units.iter().map(|u| u.0.to_string()).collect();
    let path = q(&s, &format!(r#"{{"q":"path","units":[{}],"x":{tx},"y":{ty}}}"#, ids.join(",")));
    assert_eq!(path.get("ok").and_then(Json::as_bool), Some(false), "{path:?}");
    assert_eq!(path.get("held").and_then(Json::as_str), Some(want.as_str()));
    // The army standing outside is not held.
    assert!(view_army(&s, st.supplied).get("held").is_some_and(Json::is_null));
}
