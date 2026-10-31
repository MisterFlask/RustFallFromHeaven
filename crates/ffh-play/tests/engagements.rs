//! Engagement combat through the human's protocol (docs/ENGAGEMENTS.md; docs/HUMAN.md, "Engagement
//! combat"): `ffh-play stage engagement`'s attack fought through `engage`, its report and battle
//! log in the shapes the battle screens read (the header of `assets/app/battle_log.js` lists every
//! field), the war an attack declares asked first, the stance commands and the view's fields; and
//! with the option off, the commands refused and the view without them.

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

// The shapes battle_log.js's header lists.
const LOG: &[&str] = &["v", "id", "turn", "viewer_side", "plot", "from", "slots", "sides", "fighters", "pairs", "events", "forecast", "outcome", "summary"];
const SIDE: &[&str] = &["player", "shown_player", "civ", "label", "army", "garrison_of", "stance", "stance_known", "auto_pick", "picked_by"];
const FIGHTER: &[&str] = &[
    "b", "unit", "unit_type", "name", "owner", "side", "slot", "level", "xp", "hero", "role", "pos", "screen", "ignores_shield", "assassin_profile", "guardsman", "targets", "promotions", "max_hp",
    "hp_start", "volleys", "volley_chance", "withdraw_pct", "def_strike", "collateral", "combat_limit", "immune", "fear", "a", "dv", "fate",
];
const STAT: &[&str] = &["base_x100", "role_pct", "terms", "total_x100"];
const PAIR: &[&str] = &["h", "t", "a_x100", "dv_x100", "mean", "a_extra", "dv_extra"];
const OUTCOME: &[&str] = &["result", "volley_rounds", "rounds", "plot_taken", "city_captured", "moved_in", "defenders_left"];
const SUMMARY: &[&str] = &[
    "id", "turn", "x", "y", "from", "place", "city", "viewer_side", "result", "volley_rounds", "rounds", "plot_taken", "city_captured", "defenders_left", "has_log", "sides",
];
const SUMMARY_SIDE: &[&str] = &["player", "civ", "label", "stance", "slots", "committed", "lost", "lost_types", "withdrew", "sat_out", "fled"];
/// Each event kind with its members (`kind` included).
const EVENTS: &[(&str, &[&str])] = &[
    ("Phase", &["kind", "phase"]),
    ("Morale", &["kind", "unit", "source", "chance", "roll", "passed"]),
    ("SatOut", &["kind", "unit", "why"]),
    ("VolleyRoll", &["kind", "unit", "chance", "roll", "total"]),
    ("DefStrike", &["kind", "striker", "target", "rolled", "chance", "roll", "pct", "dmg"]),
    ("Collateral", &["kind", "source", "target", "pct", "limit_pct", "protection_pct", "dmg"]),
    ("Hit", &["kind", "h", "t", "rule", "step", "blocked", "eligible", "r", "dmg", "capped"]),
    ("Withdraw", &["kind", "unit", "hp_pct", "chance", "roll", "ok", "fatal", "hp_kept"]),
    ("RoundEnd", &["kind", "deaths"]),
    ("ShieldDown", &["kind", "side"]),
    ("End", &["kind", "result", "volley_rounds", "rounds"]),
    ("Heal", &["kind", "unit", "amount", "source"]),
    ("Xp", &["kind", "unit", "amount", "from"]),
    ("KillEffect", &["kind", "killer", "victim", "effect"]),
    ("Flee", &["kind", "unit", "to"]),
    ("Rout", &["kind", "units", "to"]),
    ("MoveIn", &["kind", "units", "city"]),
    ("Cast", &["kind", "caster", "spell", "targets", "effect"]),
];

/// The staged engagement (the smoke test's): the plan, the stances, the attack through `engage`,
/// its report in the command's reply, the battle list, the message's link and the battle log in
/// the shapes the screens read.
#[test]
fn the_staged_engagement_is_fought_through_engage_and_its_log_has_the_screens_fields() {
    let dir = common::TempDir::new("stage-engagement");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some("engagement"));
    let mut s = load(&st.save);
    let (army, x, y) = (st.army.0, st.x, st.y);

    // The view: engagements on, no battle yet; the army's stance, the cities' garrison stance,
    // each army unit's role, A, Dv and battle HP.
    let view = q(&s, r#"{"q":"view"}"#);
    assert_eq!(view.get("engagements").and_then(Json::as_bool), Some(true));
    assert_eq!(view.get("battles").and_then(Json::as_arr).map(<[Json]>::len), Some(0));
    let armies = arr(&view, "armies");
    let a = armies.iter().find(|a| int(a, "id") == army as i64).expect("the staged army in the view");
    assert!(a.get("stance").and_then(Json::as_i64).is_some(), "the army's stance");
    assert!(arr(&view, "cities").iter().filter(|c| c.get("owner").and_then(Json::as_i64) == Some(s.human.0 as i64)).all(|c| c.get("garrison_stance").and_then(Json::as_i64).is_some()));
    for u in arr(&view, "units").iter().filter(|u| st.units.iter().any(|&id| id.0 as i64 == int(u, "id"))) {
        assert!(int(u, "engage_attack") > 0 && int(u, "engage_defense") > 0 && int(u, "engage_max_hp") > 0, "{u:?}");
        assert!(u.get("engage_role").and_then(Json::as_i64).is_some());
    }

    // The `combat` query: an engagement, its pool and defenders.
    let units: Vec<String> = st.units.iter().map(|u| u.0.to_string()).collect();
    let c = q(&s, &format!(r#"{{"q":"combat","units":[{}],"x":{x},"y":{y}}}"#, units.join(",")));
    assert_eq!(c.get("engagement").and_then(Json::as_bool), Some(true), "{c:?}");
    assert_eq!(c.get("valid").and_then(Json::as_bool), Some(true));
    assert_eq!(int(&c, "pool"), st.units.len() as i64);
    assert_eq!(int(&c, "defenders"), st.defenders.len() as i64);

    // The plan: the stance's pick and its slots; the estimate not built yet.
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}}}}"#));
    assert!(arr(&pv, "errors").is_empty(), "{pv:?}");
    assert!(!arr(&pv, "force").is_empty());
    assert_eq!(arr(&pv, "fighters").len(), st.units.len() + st.defenders.len());
    assert_eq!(arr(&pv, "slots").len(), 2);
    assert!(pv.get("forecast").is_some_and(Json::is_null));
    assert_eq!(pv.get("estimate_pending").and_then(Json::as_bool), Some(true));

    // The stances: the army's and the capital garrison's are set; a stance out of range is not.
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_stance","army":{army},"stance":"STANCE_MOBILE"}}"#)), Ok(CmdOk::Applied { .. })));
    let mobile = ffh_data::global().t::<ffh_data::StanceType>("STANCE_MOBILE");
    assert_eq!(s.g.armies[st.army].stance, mobile);
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"garrison_stance","city":{},"stance":"STANCE_SHIELD_WALL"}}"#, st.capital.0)), Ok(CmdOk::Applied { .. })));
    assert_eq!(s.g.cities[st.capital].garrison_stance, ffh_data::global().t::<ffh_data::StanceType>("STANCE_SHIELD_WALL"));
    assert!(matches!(cmd(&mut s, &format!(r#"{{"c":"army_stance","army":{army},"stance":99}}"#)), Err(CmdErr::Invalid(_))));
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}}}}"#));
    assert_eq!(pv.get("stance").and_then(Json::as_i64), Some(mobile.0 as i64), "the plan takes the army's stance");

    // An attack on a plot that is not next to the army is refused, and changes nothing.
    let far = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{},"y":{y}}}}}"#, x + 3));
    assert!(matches!(far, Err(CmdErr::Cannot(_))), "{far:?}");
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
    assert_eq!(list.get("on").and_then(Json::as_bool), Some(true));
    let reports = arr(&list, "reports");
    assert_eq!(reports.len(), 1);
    assert_eq!(keys(&reports[0]), sorted(SUMMARY));
    for side in arr(&reports[0], "sides") {
        assert_eq!(keys(&side), sorted(SUMMARY_SIDE));
    }
    assert_eq!(int(&reports[0], "viewer_side"), 0);
    let view = q(&s, r#"{"q":"view"}"#);
    assert_eq!(arr(&view, "battles").len(), 1);

    // The battle log, every member the screens read.
    let l = q(&s, &format!(r#"{{"q":"battle_log","report":{id}}}"#));
    assert_eq!(keys(&l), sorted(LOG));
    assert_eq!(int(&l, "id"), id as i64);
    for side in arr(&l, "sides") {
        assert_eq!(keys(&side), sorted(SIDE));
    }
    let fighters = arr(&l, "fighters");
    assert_eq!(fighters.len(), st.units.len() + st.defenders.len(), "every unit of both sides is in the log");
    for (b, f) in fighters.iter().enumerate() {
        assert_eq!(keys(f), sorted(FIGHTER));
        assert_eq!(int(f, "b"), b as i64);
        assert_eq!(keys(f.get("a").unwrap()), sorted(STAT));
        assert_eq!(keys(f.get("dv").unwrap()), sorted(STAT));
    }
    for p in arr(&l, "pairs") {
        assert_eq!(keys(&p), sorted(PAIR));
    }
    assert_eq!(keys(l.get("outcome").unwrap()), sorted(OUTCOME));
    // Names for people, not text keys.
    let city = l.get("plot").and_then(|p| p.get("city")).and_then(|c| c.get("name")).and_then(Json::as_str).expect("the defended city's name");
    assert!(!city.is_empty() && !city.contains("TXT_KEY"), "{city}");
    for side in arr(&l, "sides") {
        let label = side.get("label").and_then(Json::as_str).unwrap();
        assert!(!label.contains("TXT_KEY"), "{label}");
    }
    // The A and Dv parts: the strength before the role's percent, the role's, the total.
    for f in &fighters {
        for k in ["a", "dv"] {
            let s = f.get(k).unwrap();
            let (base, role, total) = (int(s, "base_x100"), int(s, "role_pct"), int(s, "total_x100"));
            let terms: i64 = arr(s, "terms").iter().map(|t| int(t, "pct")).sum();
            let want = base * role / 100 * (100 + terms) / 100;
            assert!((want - total).abs() <= 2, "{k} of {f:?}: {base} x {role}% x (100 + {terms})% is not {total}");
        }
    }
    let n = fighters.len() as i64;
    let in_range = |e: &Json, k: &str| e.get(k).and_then(Json::as_i64).is_none_or(|b| (0..n).contains(&b));
    let events = arr(&l, "events");
    assert!(events.iter().any(|e| e.get("kind").and_then(Json::as_str) == Some("Hit")), "the battle has hits");
    assert!(events.iter().any(|e| e.get("kind").and_then(Json::as_str) == Some("End")));
    for e in &events {
        let kind = e.get("kind").and_then(Json::as_str).unwrap_or("?");
        let (_, members) = EVENTS.iter().find(|(k, _)| *k == kind).unwrap_or_else(|| panic!("an event kind the screens do not know: {e:?}"));
        let mut want = sorted(members);
        // (Phase's `n`: volleys and rounds only.)
        if kind == "Phase" && e.get("n").is_some() {
            want = sorted(&["kind", "phase", "n"]);
        }
        assert_eq!(keys(e), want, "{e:?}");
        for k in ["h", "t", "unit", "striker", "target", "source", "killer", "victim", "caster"] {
            assert!(in_range(e, k), "{k} out of range: {e:?}");
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

/// The stance picked in the plan is the attack's: `engage` with a stance and the plan's force
/// fights with that stance, the battle's log records it (not the army's standing order, which
/// stays) and the force as that stance's own pick.
#[test]
fn the_plan_s_stance_is_the_attack_s() {
    let dir = common::TempDir::new("stage-engagement-stance");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let mut s = load(&st.save);
    let (army, x, y) = (st.army.0, st.x, st.y);
    let mobile = ffh_data::global().t::<ffh_data::StanceType>("STANCE_MOBILE");
    let standing = s.g.armies[st.army].stance;
    assert_ne!(standing, mobile);
    let pv = q(&s, &format!(r#"{{"q":"engage_preview","army":{army},"to":{{"x":{x},"y":{y}}},"stance":"STANCE_MOBILE"}}"#));
    assert_eq!(int(&pv, "stance"), mobile.0 as i64);
    let fighters = arr(&pv, "fighters");
    let force: Vec<String> = arr(&pv, "force").iter().map(|b| int(&fighters[b.as_i64().unwrap() as usize], "unit").to_string()).collect();
    let r = cmd(&mut s, &format!(r#"{{"c":"engage","army":{army},"to":{{"x":{x},"y":{y}}},"force":[{}],"stance":"STANCE_MOBILE"}}"#, force.join(","))).unwrap();
    let CmdOk::Applied { battle: Some(id), .. } = r else { panic!("no battle: {r:?}") };
    let l = q(&s, &format!(r#"{{"q":"battle_log","report":{id}}}"#));
    let side = arr(&l, "sides").into_iter().next().unwrap();
    assert_eq!(int(&side, "stance"), mobile.0 as i64, "the stance picked is the one recorded: {side:?}");
    assert_eq!(side.get("picked_by").and_then(|p| p.get("kind")).and_then(Json::as_str), Some("Auto"), "the plan's force is the stance's own pick: {side:?}");
    let summary = l.get("summary").unwrap();
    assert_eq!(int(&arr(summary, "sides")[0], "stance"), mobile.0 as i64);
    if let Some(a) = s.g.armies.get(st.army) {
        assert_eq!(a.stance, standing, "the army's standing order is unchanged");
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

/// With the option off (armies on, engagements off: `stage armies`), the stance commands and
/// `engage` are refused, the queries say so, and the view has none of the fields.
#[test]
fn without_engagements_the_commands_are_refused_and_the_view_has_none_of_it() {
    let dir = common::TempDir::new("stage-no-engagements");
    let st = ffh_play::stage::armies(&dir.0).expect("staged");
    let mut s = load(&st.save);
    assert!(s.g.armies_on() && !s.g.engagements_on());
    let off = |r: Result<CmdOk, CmdErr>| matches!(r, Err(CmdErr::Cannot(ref m)) if m.contains("engagements are off"));
    assert!(off(cmd(&mut s, &format!(r#"{{"c":"army_stance","army":{},"stance":0}}"#, st.army.0))));
    assert!(off(cmd(&mut s, &format!(r#"{{"c":"garrison_stance","city":{},"stance":0}}"#, st.capital.0))));
    assert!(off(cmd(&mut s, &format!(r#"{{"c":"engage","army":{},"to":{{"x":0,"y":0}}}}"#, st.army.0))));
    let list = q(&s, r#"{"q":"battle_reports"}"#);
    assert_eq!(list.get("on").and_then(Json::as_bool), Some(false));
    assert!(arr(&list, "reports").is_empty());
    assert!(s.query_json(&Json::parse(&format!(r#"{{"q":"engage_preview","army":{},"to":{{"x":0,"y":0}}}}"#, st.army.0)).unwrap()).is_err());
    let view = q(&s, r#"{"q":"view"}"#);
    assert_eq!(view.get("engagements").and_then(Json::as_bool), Some(false));
    assert!(view.get("battles").is_some_and(Json::is_null));
    for u in arr(&view, "units") {
        assert!(u.get("engage_role").is_none(), "{u:?}");
    }
    for a in arr(&view, "armies") {
        assert!(a.get("stance").is_none(), "{a:?}");
    }
    for c in arr(&view, "cities") {
        assert!(c.get("garrison_stance").is_none(), "{c:?}");
    }
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, st.capital.0));
    assert!(city.get("garrison_stance").is_some_and(Json::is_null));
}
