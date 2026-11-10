//! Battle tactics (docs/TACTICS.md) and supply lines (docs/SUPPLY.md) through the human's protocol
//! (docs/HUMAN.md): the `army_tactic` and `garrison_tactic` commands with the views that show them
//! (the Armies data, the city's garrison, the static data's tactics), and each army's supply in
//! the view (its state, its line, its words), the path query's supply readout and the lines of
//! other players' armies the human sees, clipped to what it has revealed. Built on `ffh-play stage
//! supply`.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command};
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

/// Command `line` as the transports take it: refused when malformed, else executed.
fn cmd(s: &mut Session, line: &str) -> Result<CmdOk, CmdErr> {
    Command::from_json(ffh_data::global(), &Json::parse(line).unwrap()).and_then(|c| s.execute(&c))
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

fn tactic(name: &str) -> i64 {
    ffh_data::global().t::<BattleTacticType>(name).idx() as i64
}

#[test]
fn the_tactic_commands_set_the_next_tactic_and_the_views_show_both() {
    let dir = common::TempDir::new("tactics-commands");
    let st = ffh_play::stage::supply(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let h = s.human;
    let a = st.supplied;
    let army = view_army(&s, a);
    assert_eq!((int(&army, "tactic"), int(&army, "tactic_next")), (tactic("TACTIC_LINE"), tactic("TACTIC_LINE")), "an army starts in Line");

    // By name or by index; the change is pending until the human's next turn starts.
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_tactic","army":{},"tactic":"TACTIC_CHARGE"}}"#, a.0)), Ok(CmdOk::Applied { .. })));
    let army = view_army(&s, a);
    assert_eq!((int(&army, "tactic"), int(&army, "tactic_next")), (tactic("TACTIC_LINE"), tactic("TACTIC_CHARGE")));
    // Refusals: an unknown tactic, an army not the human's.
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_tactic","army":{},"tactic":"TACTIC_SQUARE"}}"#, a.0)), Err(CmdErr::Invalid(_))));
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_tactic","army":{},"tactic":99}}"#, a.0)), Err(CmdErr::Invalid(_))));
    let (ai, ai_army, _) = ai_army(&mut s.g, h);
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_tactic","army":{},"tactic":"TACTIC_LINE"}}"#, ai_army.0)), Err(CmdErr::NotOwner)));
    // The turn start applies it.
    s.g.player_apply_tactics(h);
    let army = view_army(&s, a);
    assert_eq!((int(&army, "tactic"), int(&army, "tactic_next")), (tactic("TACTIC_CHARGE"), tactic("TACTIC_CHARGE")));

    // The garrison's: on the city query's recruitment part.
    let c = st.capital;
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"garrison_tactic","city":{},"tactic":"TACTIC_VOLLEY"}}"#, c.0)), Ok(CmdOk::Applied { .. })));
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, c.0));
    let rc = city.get("recruit").expect("recruit");
    assert_eq!((int(rc, "garrison_tactic"), int(rc, "garrison_tactic_next")), (tactic("TACTIC_LINE"), tactic("TACTIC_VOLLEY")));
    let ai_city = s.g.players[ai].cities[0];
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"garrison_tactic","city":{},"tactic":"TACTIC_LINE"}}"#, ai_city.0)), Err(CmdErr::NotOwner)));

    // The commands round-trip through the protocol (journal and replay take their JSON).
    for c in [Command::ArmyTactic { army: a, tactic: BattleTacticType::from_usize(2) }, Command::GarrisonTactic { city: c, tactic: BattleTacticType::from_usize(1) }] {
        assert_eq!(Command::from_json(ffh_data::global(), &c.to_json()).unwrap(), c);
    }
}

#[test]
fn the_static_data_lists_the_four_tactics_with_their_help() {
    let d = ffh_data::global();
    let data = ffh_play::view::data::data(d);
    let list = data.get("tables").and_then(|t| t.get("BattleTacticType")).and_then(Json::as_arr).expect("the tactics' table");
    let names: Vec<String> = list.iter().map(|t| text(t, "name")).collect();
    assert_eq!(names, ["Line", "Charge", "Volley", "Vanguard"]);
    for t in list {
        let help = text(t, "help");
        assert!(help.contains("[ICON_BULLET]") && !help.contains('%'), "{help}");
    }
    let vanguard = text(&list[3], "help");
    assert!(vanguard.contains(&format!("level {} or above", d.defines.tactic_veteran_level)), "{vanguard}");
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
}

/// The lines of other players' armies: an AI army the human sees next to the AI's city has its
/// line, every point of it a plot the human has revealed; an AI army the human does not see has
/// none; the view never names their counters or sources.
#[test]
fn the_lines_of_other_players_armies_the_human_sees_are_clipped_to_its_map() {
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
