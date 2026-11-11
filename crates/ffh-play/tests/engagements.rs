//! Engagement combat through the human's protocol (docs/ENGAGEMENTS.md, formation battles; docs/HUMAN.md,
//! "Engagement combat"): `ffh-play stage engagement`'s attack fought through `engage` (in the auto
//! formation and in one the player orders), its report and battle log in the server's shapes
//! (`view/battles.rs`), the plan with its forecast, the war an attack declares asked first and
//! the view's fields; and with the option off, the command refused and the view without them.

mod common;

use std::sync::Arc;

use ffh_play::command::{CmdErr, CmdOk, Command, Confirm};
use ffh_play::json::Json;
use ffh_core::ids::UnitId;
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

/// The keys of object `j`, sorted.
fn keys(j: &Json) -> Vec<String> {
    let mut k: Vec<String> = j.as_obj().unwrap_or_else(|| panic!("not an object: {j:?}")).iter().map(|(k, _)| k.clone()).collect();
    k.sort();
    k
}

fn sorted(list: &[&str]) -> Vec<String> {
    let mut k: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    k.sort();
    k
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("no array `{key}` in {j:?}")).to_vec()
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("no integer `{key}` in {j:?}"))
}

// The shapes of the battle log (`view/battles.rs`; `assets/app/battle_log.js`'s header is the
// screens' contract).
const LOG: &[&str] = &[
    "v", "id", "turn", "viewer_side", "plot", "from", "ground", "params", "slots", "lanes", "tactics", "sides", "fighters", "pairs", "events", "after", "forecast", "outcome", "summary",
];
const SIDE: &[&str] = &["player", "shown_player", "civ", "label", "lanes", "ordered"];
const FIGHTER: &[&str] = &[
    "b", "unit", "sentinel", "illusion", "summoner", "on_kill", "unit_type", "name", "owner", "side", "lane", "row", "large", "level", "xp", "hero", "promotions", "max_hp", "hp_start", "hp_battle", "speed",
    "first_strikes", "chance_first_strikes", "rolled_first_strikes", "actions", "bonus_action", "spell", "accuracy", "evasion", "crit", "anti_flank", "flanker", "withdraw_pct",
    "combat_limit", "collateral_limit", "splash", "def_strike", "immune", "fear", "a_x100", "dv_x100", "ma_x100", "md_x100", "a", "dv", "mag", "mdef", "fate", "dealt", "taken", "healed",
];
const STAT: &[&str] = &["base_x100", "terms", "total_x100"];
const PAIR: &[&str] = &["h", "t", "a_x100", "dv_x100", "mean", "ma_x100", "md_x100", "spell_pct", "magic_mean", "a_extra", "dv_extra"];
const GROUND: &[&str] = &["city", "forest", "hills", "flanking", "rules"];
const PARAMS: &[&str] = &["bolt_pct", "splash_pct", "first_strike_speed"];
const OUTCOME: &[&str] = &["result", "rounds", "plot_taken", "city_captured", "moved_in", "defenders_left", "lines_down", "sides"];
const LINE: &[&str] = &["committed", "sat_out", "fallen", "escaped", "standing"];
const SUMMARY: &[&str] = &["id", "turn", "x", "y", "from", "place", "city", "viewer_side", "result", "rounds", "plot_taken", "city_captured", "defenders_left", "has_log", "sides"];
const SUMMARY_SIDE: &[&str] = &["player", "civ", "label", "slots", "lanes", "used", "committed", "sat_out", "lost", "lost_types", "escaped", "standing", "shore"];
const PLAN: &[&str] = &[
    "plot", "from", "ground", "params", "slots", "lanes", "sides", "fighters", "formation", "auto_formation", "reserve", "def_formation", "def_reserve", "summons", "tactics",
    "starving", "shore", "shore_unseen", "forecast", "declare_war", "errors",
];
const FORECAST: &[&str] = &["runs", "line_wiped_permille", "plot_falls_permille", "attackers_wiped_permille", "lost_x100", "escaped_x100", "per"];
const HIT: &[&str] = &["target", "dmg", "expected", "r", "crit", "miss", "killed", "withdrew", "splash"];
/// The action kinds as the screens name them.
const KINDS: &[&str] = &["Strike", "Charge", "Stab", "Shoot", "Spell", "Bombard", "Heal", "None"];
/// Each event kind of `events` with its members (`kind` included; an action's `kind` is its
/// action kind).
const EVENTS: &[(&str, &[&str])] = &[
    ("Afraid", &["round", "pass", "kind", "actor", "target"]),
    ("DefStrike", &["round", "pass", "kind", "actor", "target", "hits", "chance", "roll", "pct"]),
    ("Cast", &["round", "pass", "kind", "actor", "spell", "targets", "effect", "hits"]),
    ("Starvation", &["round", "pass", "kind", "units", "dealt_pct"]),
];
const ACTION: &[&str] = &["round", "pass", "actor", "kind", "spell", "target", "hits", "heal", "knocked", "note", "entered"];
/// Each kind of `after` with its members.
const AFTER: &[(&str, &[&str])] = &[
    ("Heal", &["kind", "unit", "amount", "source"]),
    ("Xp", &["kind", "unit", "amount", "from"]),
    ("KillEffect", &["kind", "killer", "victim", "effect"]),
    ("Flee", &["kind", "unit", "to"]),
    ("MoveIn", &["kind", "units", "city"]),
    ("Cast", &["kind", "actor", "spell", "targets", "effect", "hits"]),
];

/// The staged engagement (the smoke test's): the plan with its forecast, the attack through
/// `engage`, its report in the command's reply, the battle list, the message's link and the battle
/// log in the server's shapes.
#[test]
fn the_staged_engagement_is_fought_through_engage_and_its_log_has_the_screens_fields() {
    let dir = common::TempDir::new("stage-engagement");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some("engagement"));
    let mut s = load(&st.save);
    let (army, x, y) = (st.army.0, st.x, st.y);

    // The view: no battle yet; each army unit's rank preference, A, Dv, battle HP, speed and
    // actions per row.
    let view = q(&s, r#"{"q":"view"}"#);
    assert_eq!(view.get("battles").and_then(Json::as_arr).map(<[Json]>::len), Some(0));
    for u in arr(&view, "units").iter().filter(|u| st.units.iter().any(|&id| id.0 as i64 == int(u, "id"))) {
        assert!(int(u, "engage_attack") > 0 && int(u, "engage_defense") > 0 && int(u, "engage_max_hp") > 0, "{u:?}");
        assert_eq!(arr(u, "engage_row_order").len(), 3, "{u:?}");
        assert!(int(u, "engage_speed") > 0 && arr(u, "engage_actions").len() == 3, "{u:?}");
        assert!(u.get("engage_large").and_then(Json::as_bool).is_some());
    }

    // The `combat` query: an engagement, its pool and defenders.
    let units: Vec<String> = st.units.iter().map(|u| u.0.to_string()).collect();
    let c = q(&s, &format!(r#"{{"q":"combat","units":[{}],"x":{x},"y":{y}}}"#, units.join(",")));
    assert_eq!(c.get("engagement").and_then(Json::as_bool), Some(true), "{c:?}");
    assert_eq!(c.get("valid").and_then(Json::as_bool), Some(true));
    assert_eq!(int(&c, "pool"), st.units.len() as i64);
    assert_eq!(int(&c, "defenders"), st.defenders.len() as i64);

    // The plan: the auto formation in its lanes, the defenders', the forecast.
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}}}}"#));
    assert_eq!(keys(&pv), sorted(PLAN));
    assert!(arr(&pv, "errors").is_empty(), "{pv:?}");
    let formation = arr(&pv, "formation");
    assert!(!formation.is_empty());
    assert_eq!(formation, arr(&pv, "auto_formation"), "no formation given: the auto formation");
    assert_eq!(arr(&pv, "fighters").len(), st.units.len() + st.defenders.len());
    assert_eq!(arr(&pv, "slots").len(), 2);
    assert_eq!(keys(pv.get("ground").unwrap()), sorted(GROUND));
    assert_eq!(keys(pv.get("params").unwrap()), sorted(PARAMS));
    for f in arr(&pv, "fighters") {
        assert_eq!(keys(&f), sorted(FIGHTER));
        for a in arr(&f, "actions") {
            assert!(KINDS.contains(&a.get("kind").and_then(Json::as_str).unwrap_or("?")), "{a:?}");
            assert!(int(&a, "n") >= 0);
        }
    }
    let fc = pv.get("forecast").unwrap();
    assert_eq!(keys(fc), sorted(FORECAST));
    assert!(int(fc, "runs") > 0 && arr(fc, "per").len() == formation.len() + arr(&pv, "def_formation").len());
    // A formation that breaks the grid: the plan in the auto formation, no forecast, and why.
    let u0 = int(&formation[0], "unit");
    let bad = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}},"formation":[{{"unit":{u0},"lane":9,"row":0}}]}}"#));
    assert!(!arr(&bad, "errors").is_empty(), "{bad:?}");
    assert!(bad.get("forecast").is_some_and(Json::is_null), "{bad:?}");
    assert_eq!(arr(&bad, "formation"), formation, "the refused formation's plan is the auto formation's");

    // An attack on a plot that is not next to the army is refused, and changes nothing.
    let far = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{},"y":{y}}}}}"#, x + 3));
    assert!(matches!(far, Err(CmdErr::Cannot(_))), "{far:?}");
    assert!(s.g.battle_reports.is_empty());
    // A formation is refused: an army fights in its tactic (docs/TACTICS.md).
    let off = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{x},"y":{y}}},"formation":[{{"unit":{u0},"lane":9,"row":0}}]}}"#));
    assert!(matches!(&off, Err(CmdErr::Cannot(r)) if r == "An army fights in its tactic."), "{off:?}");
    assert!(s.g.battle_reports.is_empty());

    // The attack: its report in the reply, the message tagged with it.
    let r = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{x},"y":{y}}}}}"#)).unwrap();
    let CmdOk::Applied { battle: Some(id), log, .. } = r else { panic!("no battle: {r:?}") };
    assert_eq!(s.g.battle_reports.len(), 1);
    assert!(log.iter().any(|e| ffh_play::report::battle_report_ref(&e.text) == Some(id)), "the battle's message names its report: {log:?}");
    let msgs = q(&s, r#"{"q":"log","since":0}"#);
    let tagged = arr(&msgs, "entries").into_iter().find(|e| e.get("kind").and_then(Json::as_str) == Some("engagement")).expect("a message of kind engagement");
    assert_eq!(int(&tagged, "report"), id as i64);

    // The battle lists: `battle_reports` and the view's `battles`, as summaries.
    let list = q(&s, r#"{"q":"battle_reports"}"#);
    let reports = arr(&list, "reports");
    assert_eq!(reports.len(), 1);
    assert_eq!(keys(&reports[0]), sorted(SUMMARY));
    for side in arr(&reports[0], "sides") {
        assert_eq!(keys(&side), sorted(SUMMARY_SIDE));
    }
    assert_eq!(int(&reports[0], "viewer_side"), 0);
    let view = q(&s, r#"{"q":"view"}"#);
    assert_eq!(arr(&view, "battles").len(), 1);

    // The battle log, every member.
    let l = q(&s, &format!(r#"{{"q":"battle_log","report":{id}}}"#));
    assert_eq!(keys(&l), sorted(LOG));
    assert_eq!(int(&l, "id"), id as i64);
    for side in arr(&l, "sides") {
        assert_eq!(keys(&side), sorted(SIDE));
    }
    assert_eq!(keys(l.get("ground").unwrap()), sorted(GROUND));
    assert_eq!(keys(l.get("params").unwrap()), sorted(PARAMS));
    let fighters = arr(&l, "fighters");
    assert_eq!(fighters.len(), st.units.len() + st.defenders.len(), "every unit of both sides is in the log");
    for (b, f) in fighters.iter().enumerate() {
        assert_eq!(keys(f), sorted(FIGHTER));
        assert_eq!(int(f, "b"), b as i64);
        assert_eq!(keys(f.get("a").unwrap()), sorted(STAT));
        assert_eq!(keys(f.get("dv").unwrap()), sorted(STAT));
        assert_eq!(keys(f.get("mag").unwrap()), sorted(STAT));
        assert_eq!(keys(f.get("mdef").unwrap()), sorted(STAT));
        assert_eq!(int(f, "a_x100"), int(f.get("a").unwrap(), "total_x100"));
        assert_eq!(int(f, "dv_x100"), int(f.get("dv").unwrap(), "total_x100"));
        assert_eq!(int(f, "md_x100"), int(f.get("mdef").unwrap(), "total_x100"));
    }
    for p in arr(&l, "pairs") {
        assert_eq!(keys(&p), sorted(PAIR));
    }
    let outcome = l.get("outcome").unwrap();
    assert_eq!(keys(outcome), sorted(OUTCOME));
    let lines = arr(outcome, "sides");
    assert_eq!(lines.len(), 2);
    for (side, line) in lines.iter().enumerate() {
        assert_eq!(keys(line), sorted(LINE));
        let sum = int(line, "sat_out") + int(line, "fallen") + int(line, "escaped") + int(line, "standing");
        assert_eq!(sum, int(line, "committed"), "side {side}: {line:?}");
    }
    // Names for people, not text keys.
    let city = l.get("plot").and_then(|p| p.get("city")).and_then(|c| c.get("name")).and_then(Json::as_str).expect("the defended city's name");
    assert!(!city.is_empty() && !city.contains("TXT_KEY"), "{city}");
    for side in arr(&l, "sides") {
        let label = side.get("label").and_then(Json::as_str).unwrap();
        assert!(!label.contains("TXT_KEY"), "{label}");
    }
    // The A and Dv parts: the strength before the role's percent, the role's, the total.
    for f in &fighters {
        for k in ["a", "dv", "mag", "mdef"] {
            let s = f.get(k).unwrap();
            let (base, total) = (int(s, "base_x100"), int(s, "total_x100"));
            let terms: i64 = arr(s, "terms").iter().map(|t| int(t, "pct")).sum();
            let want = if terms >= 0 { base * (100 + terms) / 100 } else { base * 100 / (100 - terms) }.max(1);
            assert!((want - total).abs() <= 2, "{k} of {f:?}: {base} x (100 + {terms})% is not {total}");
        }
    }
    let n = fighters.len() as i64;
    let in_range = |e: &Json, k: &str| e.get(k).and_then(Json::as_i64).is_none_or(|b| (0..n).contains(&b));
    let events = arr(&l, "events");
    let kind = |e: &Json| e.get("kind").and_then(Json::as_str).unwrap_or("?").to_string();
    assert!(events.iter().any(|e| KINDS.contains(&kind(e).as_str())), "the battle has actions");
    let mut last = (0, 0);
    for e in &events {
        let k = kind(e);
        let members: &[&str] = if KINDS.contains(&k.as_str()) {
            ACTION
        } else {
            EVENTS.iter().find(|(x, _)| *x == k).unwrap_or_else(|| panic!("an event kind the screens do not know: {e:?}")).1
        };
        assert_eq!(keys(e), sorted(members), "{e:?}");
        for key in ["actor", "target"] {
            assert!(in_range(e, key), "{key} out of range: {e:?}");
        }
        for h in e.get("hits").and_then(Json::as_arr).unwrap_or(&[]) {
            if members == ACTION {
                assert_eq!(keys(h), sorted(HIT));
            }
            assert!(in_range(h, "target"));
        }
        // In order: the opening (round 0, pass 0), then the rounds and their passes.
        let at = (int(e, "round"), int(e, "pass"));
        assert!(at >= last, "{e:?} after round {} pass {}", last.0, last.1);
        assert_eq!(at.0 == 0, !members.eq(ACTION), "only the opening is at round 0: {e:?}");
        last = at;
    }
    for a in arr(&l, "after") {
        let k = kind(&a);
        let (_, members) = AFTER.iter().find(|(x, _)| *x == k).unwrap_or_else(|| panic!("an after-battle kind the screens do not know: {a:?}"));
        assert_eq!(keys(&a), sorted(members), "{a:?}");
        for key in ["unit", "killer", "victim"] {
            assert!(in_range(&a, key), "{key} out of range: {a:?}");
        }
    }
    // A report that is not kept, or not the viewer's, is gone.
    assert!(matches!(s.query_json(&Json::parse(r#"{"q":"battle_log","report":4321}"#).unwrap()), Err(ffh_play::query::QueryErr::Gone)));
}

/// The `combat` query on a battle the selection cannot launch now keeps the duel's answer (the
/// hover's combat lines and the path's odds chip read `valid` and `odds`) and says it is a battle
/// on arrival: the city two plots away (a go-to that ends in an attack), and next to the army
/// once its units have used their moves. `pool` counts the selected units that could fight. And
/// `engage_preview` on a plot that is not next to the attackers, or that the human does not see,
/// answers its errors alone: no plot, slots or fighters (G5).
#[test]
fn the_combat_query_previews_a_battle_on_arrival() {
    let dir = common::TempDir::new("stage-engagement-arrival");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let (x, y) = (st.x, st.y);
    let combat = |s: &Session, units: &[UnitId]| {
        let ids: Vec<String> = units.iter().map(|u| u.0.to_string()).collect();
        q(s, &format!(r#"{{"q":"combat","units":[{}],"x":{x},"y":{y}}}"#, ids.join(",")))
    };
    let flag = |j: &Json, k: &str| j.get(k).and_then(Json::as_bool).unwrap_or_else(|| panic!("no boolean `{k}` in {j:?}"));

    // Next to the city with moves left: a battle now.
    let c = combat(&s, &st.units);
    assert!(flag(&c, "engagement") && flag(&c, "valid") && !flag(&c, "on_arrival"), "{c:?}");

    // Next to it with no moves left: on arrival, the duel's odds kept.
    let mut spent = load(&st.save);
    for &u in &st.units {
        spent.g.unit_finish_moves(u);
    }
    let c = combat(&spent, &st.units);
    assert!(flag(&c, "engagement") && flag(&c, "on_arrival"), "{c:?}");
    assert!(flag(&c, "valid") && c.get("odds").and_then(Json::as_i64).is_some(), "the duel's odds stay: {c:?}");
    assert_eq!(int(&c, "pool"), st.units.len() as i64, "every selected unit could fight there");

    // Two plots away (the warrior stays next to the city, so it is still seen): on arrival.
    let g = &s.g;
    let at = g.unit_plot(st.units[0]);
    let (ax, ay) = g.map.xy(at);
    let city_plot = g.map.plot_xy(x, y).unwrap();
    let back = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
        .filter_map(|(dx, dy)| g.map.plot_xy(ax + dx, ay + dy))
        .find(|&p| {
            let (px, py) = g.map.xy(p);
            g.step_distance(px, py, x, y) == 2 && !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.map.plots[p].units.is_empty() && g.map.plots[p].plot_city.is_none()
        })
        .expect("a plot two steps from the city next to the army");
    let warrior = ffh_data::global().t::<ffh_data::UnitType>("UNIT_WARRIOR");
    let moved: Vec<UnitId> = st.units.iter().copied().filter(|&u| s.g.units[u].unit_type != warrior).collect();
    let (bx, by) = s.g.map.xy(back);
    for &u in &moved {
        s.g.unit_set_xy(u, bx, by, false);
    }
    assert!(s.g.plot_is_visible(city_plot, s.g.players[s.human].team), "the city is still seen");
    let c = combat(&s, &moved);
    assert!(flag(&c, "engagement") && flag(&c, "on_arrival"), "{c:?}");
    assert!(flag(&c, "valid") && c.get("odds").and_then(Json::as_i64).is_some(), "the duel's odds stay: {c:?}");
    assert_eq!(int(&c, "pool"), moved.len() as i64);
    assert_eq!(int(&c, "defenders"), st.defenders.len() as i64);

    // The plan of an attack on a plot that is not next to the attackers: its errors alone.
    let ids: Vec<String> = moved.iter().map(|u| u.0.to_string()).collect();
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","units":[{}],"to":{{"x":{x},"y":{y}}}}}"#, ids.join(",")));
    assert_eq!(keys(&pv), sorted(&["declare_war", "errors"]), "{pv:?}");
    assert!(!arr(&pv, "errors").is_empty());

    // ... and on a plot the human does not see (from a plot next to it).
    let team = s.g.players[s.human].team;
    let unseen = s.g.map.plot_ids().find(|&p| !s.g.plot_is_visible(p, team) && !s.g.plot_is_water(p)).expect("a plot the human does not see");
    let (ux, uy) = s.g.map.xy(unseen);
    let from = s.g.map.plot_xy(ux + 1, uy).or_else(|| s.g.map.plot_xy(ux - 1, uy)).expect("a neighbour");
    let (fx, fy) = s.g.map.xy(from);
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","from":{{"x":{fx},"y":{fy}}},"to":{{"x":{ux},"y":{uy}}}}}"#));
    assert_eq!(keys(&pv), sorted(&["declare_war", "errors"]), "{pv:?}");
    assert!(arr(&pv, "errors").iter().any(|e| e.as_str().is_some_and(|t| t.contains("cannot see"))), "{pv:?}");
}

/// Battle tactics (docs/TACTICS.md): `engage` refuses a formation (even the plan's own), and the
/// attack fights in the formation the plan shows: the army's tactic's (Line, the army's first),
/// named on both sides of the plan and the log, not ordered.
#[test]
fn an_army_fights_in_its_tactic_not_in_a_formation_given() {
    let dir = common::TempDir::new("stage-engagement-formation");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let (army, x, y) = (st.army.0, st.x, st.y);
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}}}}"#));
    let line = ffh_data::global().t::<ffh_data::BattleTacticType>("TACTIC_LINE");
    let tactics: Vec<i64> = arr(&pv, "tactics").iter().filter_map(Json::as_i64).collect();
    assert_eq!(tactics, vec![ffh_data::Idx::idx(line) as i64; 2], "{pv:?}");
    let cells: Vec<(i64, i64, i64)> = arr(&pv, "formation").iter().map(|p| (int(p, "unit"), int(p, "lane"), int(p, "row"))).collect();
    assert!(!cells.is_empty());
    let list: Vec<String> = cells.iter().map(|(u, l, r)| format!(r#"{{"unit":{u},"lane":{l},"row":{r}}}"#)).collect();
    let refused = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{x},"y":{y}}},"formation":[{}]}}"#, list.join(",")));
    assert!(matches!(&refused, Err(CmdErr::Cannot(r)) if r == "An army fights in its tactic."), "{refused:?}");
    assert!(s.g.battle_reports.is_empty(), "a refused attack changes nothing");
    let r = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{x},"y":{y}}}}}"#)).unwrap();
    let CmdOk::Applied { battle: Some(id), .. } = r else { panic!("no battle: {r:?}") };
    let l = q(&s, &format!(r#"{{"q":"battle_log","report":{id}}}"#));
    assert_eq!(arr(&l, "tactics").iter().filter_map(Json::as_i64).collect::<Vec<_>>(), tactics);
    let side = arr(&l, "sides").into_iter().next().unwrap();
    assert_eq!(side.get("ordered").and_then(Json::as_bool), Some(false), "{side:?}");
    for (u, lane, row) in cells {
        let f = arr(&l, "fighters").into_iter().find(|f| int(f, "unit") == u).expect("in the log");
        assert_eq!((int(&f, "lane"), int(&f, "row")), (lane, row), "the plan's cell: {f:?}");
    }
}

/// An attack that would declare war asks first, as `move` does: the confirmation, a "no" that
/// changes nothing, then the war and the battle.
#[test]
fn an_engage_that_declares_war_asks_first() {
    let dir = common::TempDir::new("stage-engagement-war");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let my = s.g.players[s.human].team;
    let city_owner = s.g.cities[st.city].owner;
    let their = s.g.players[city_owner].team;
    s.g.team_make_peace(my, their, false);
    assert!(!s.g.teams[my].at_war[their]);
    let line = |dw: &str| format!(r#"{{"c":"engage","army":{},"to":{{"x":{},"y":{}}}{dw}}}"#, st.army.0, st.x, st.y);
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{},"to":{{"x":{},"y":{}}}}}"#, st.army.0, st.x, st.y));
    assert_eq!(pv.get("declare_war").and_then(Json::as_i64), Some(their.0 as i64), "the plan says the attack declares war");
    match cmd(&mut s, &line("")) {
        Ok(CmdOk::NeedsConfirm(Confirm::DeclareWar { team, .. })) => assert_eq!(team, their),
        other => panic!("{other:?}"),
    }
    assert!(matches!(cmd(&mut s, &line(r#","declare_war":false"#)), Ok(CmdOk::Applied { battle: None, .. })));
    assert!(!s.g.teams[my].at_war[their] && s.g.battle_reports.is_empty(), "a no changes nothing");
    let r = cmd(&mut s, &line(r#","declare_war":true"#));
    assert!(matches!(r, Ok(CmdOk::Applied { battle: Some(_), .. })), "{r:?}");
    assert!(s.g.teams[my].at_war[their]);
}

/// Card: False Colours (docs/CARDS.md): an army under false colours attacks a civ at peace
/// without the question: the plan shows the battle, and the war is declared with it.
#[test]
fn an_engage_under_false_colours_asks_nothing_and_the_war_comes_with_the_battle() {
    let dir = common::TempDir::new("stage-engagement-colours");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let my = s.g.players[s.human].team;
    let their = s.g.players[s.g.cities[st.city].owner].team;
    s.g.team_make_peace(my, their, false);
    let colours = s.g.d.t::<ffh_data::PromotionType>("PROMOTION_FALSE_COLOURS");
    for &u in &st.units {
        s.g.unit_set_has_promotion(u, colours, true);
    }
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{},"to":{{"x":{},"y":{}}}}}"#, st.army.0, st.x, st.y));
    assert_eq!(pv.get("declare_war"), Some(&Json::Null), "no question");
    assert!(arr(&pv, "errors").is_empty() && !arr(&pv, "fighters").is_empty(), "a plan: {pv:?}");
    let r = cmd(&mut s, &format!(r#"{{"c":"engage","army":{},"to":{{"x":{},"y":{}}}}}"#, st.army.0, st.x, st.y));
    assert!(matches!(r, Ok(CmdOk::Applied { battle: Some(_), .. })), "{r:?}");
    assert!(s.g.teams[my].at_war[their], "the war is declared at the battle");
}
