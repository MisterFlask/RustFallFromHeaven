//! End-to-end protocol & adapter tests for the Cataclysm alternate ruleset mode.

use ffh_play::bot::Bot;
use ffh_play::cataclysm::CataclysmSession;
use ffh_play::json::Json;
use ffh_play::transport::{Host, HostOptions, Request};

fn host_call(host: &mut Host, req: &str) -> Json {
    let j = Json::parse(req).expect("valid request JSON");
    let (id, parsed) = Request::from_json(&j);
    let parsed = parsed.unwrap_or_else(|e| panic!("invalid request {req}: {e:?}"));
    let (reply, _) = host.handle(id, parsed);
    assert_eq!(reply.get("ok").and_then(Json::as_bool), Some(true), "request failed: {req} => {reply}");
    reply.get("r").cloned().expect("reply has r")
}

#[test]
fn cataclysm_host_session_lifecycle_and_commands() {
    let mut host = Host::new(HostOptions::default(), Bot::new(1).answerer());

    // 1. Setup query returns all 9 factions with embedded FFH2 art indices.
    let setup = host_call(&mut host, r#"{"k":"q","q":"cataclysm_setup"}"#);
    assert_eq!(setup.get("mode").and_then(Json::as_str), Some("cataclysm"));
    let factions = setup.get("factions").and_then(Json::as_arr).expect("factions arr");
    assert_eq!(factions.len(), 9);
    for f in factions {
        assert!(f.get("civ").and_then(Json::as_i64).is_some(), "civ art missing: {f}");
        assert!(f.get("leader").and_then(Json::as_i64).is_some(), "leader art missing: {f}");
    }

    // 2. Start a Cataclysm game via session new.
    let started = host_call(
        &mut host,
        r#"{"k":"session","op":"new","setup":{"mode":"cataclysm","faction":"amurites","size":"small","opponents":3,"seed":77}}"#,
    );
    assert_eq!(started.get("mode").and_then(Json::as_str), Some("cataclysm"));
    let st0 = host_call(&mut host, r#"{"k":"q","q":"status"}"#);
    assert_eq!(st0.get("mode").and_then(Json::as_str), Some("cataclysm"));
    assert_eq!(st0.get("phase").and_then(Json::as_str), Some("human"));
    assert_eq!(st0.get("turn").and_then(Json::as_i64), Some(1));

    // 3. View & map match HD-2D / ISO schema + native Cataclysm tree.
    let view = host_call(&mut host, r#"{"k":"q","q":"view"}"#);
    assert_eq!(view.get("mode").and_then(Json::as_str), Some("cataclysm"));
    let units = view.get("units").and_then(Json::as_arr).expect("units");
    assert!(!units.is_empty(), "starting squad members should emit map figures");
    assert!(units[0].get("stand").is_some(), "squad figures must emit 3x3 stand positions");

    let cat = view.get("cataclysm").expect("cataclysm payload");
    let squads = cat.get("squads").and_then(Json::as_arr).expect("squads");
    let my_sq = squads.iter().find(|s| s.get("owner").and_then(Json::as_i64) == Some(0)).expect("human squad");
    let sq_id = my_sq.get("id").and_then(Json::as_i64).unwrap();
    let members = my_sq.get("members").and_then(Json::as_arr).unwrap();
    let member_id = members[0].get("id").and_then(Json::as_i64).unwrap();

    let cities = cat.get("cities").and_then(Json::as_arr).expect("cities");
    let my_city = cities.iter().find(|c| c.get("owner").and_then(Json::as_i64) == Some(0)).expect("human capital");
    let city_id = my_city.get("id").and_then(Json::as_i64).unwrap();

    let map = host_call(&mut host, r#"{"k":"q","q":"map"}"#);
    assert_eq!(map.get("w").and_then(Json::as_i64), Some(22));
    assert_eq!(map.get("h").and_then(Json::as_i64), Some(16));

    // 4. Issue native Cataclysm commands: build District, pick Foundation tech, rearrange 3x3 cell, reinforce, end turns.
    host_call(&mut host, &format!(r#"{{"k":"cmd","c":"cat_city_build","city":{city_id},"kind":"district","key":"academy"}}"#));
    host_call(&mut host, r#"{"k":"cmd","c":"cat_research","tech":0}"#);
    host_call(&mut host, &format!(r#"{{"k":"cmd","c":"cat_set_cell","squad":{sq_id},"member":{member_id},"row":1,"lane":1}}"#));
    host_call(&mut host, &format!(r#"{{"k":"cmd","c":"cat_reinforce","squad":{sq_id},"class":"guard"}}"#));

    for t in 2..=6 {
        host_call(&mut host, r#"{"k":"cmd","c":"end_turn"}"#);
        let st = host_call(&mut host, r#"{"k":"q","q":"status"}"#);
        assert_eq!(st.get("turn").and_then(Json::as_i64), Some(t));
    }
}

#[test]
fn cataclysm_battle_log_v2_matches_watch_schema() {
    let setup = Json::parse(r#"{"mode":"cataclysm","faction":"bannor","size":"small","opponents":2,"seed":19}"#).unwrap();
    let mut sess = CataclysmSession::new(&setup).expect("session");

    // Place a neutral goblin squad right next to the human vanguard so combat fires deterministically.
    let human_sq_id = sess.g.player_squads(cataclysm_core::PlayerId(0))[0];
    let from = sess.g.squads[&human_sq_id].plot;
    let target_plot = sess
        .g
        .world
        .neighbors(from)
        .into_iter()
        .find(|&p| sess.g.world.plot(p).kind.is_passable_land() && sess.g.world.plot(p).city.is_none() && sess.g.squads_at(p).is_empty())
        .expect("adjacent land plot");

    let neutral = sess.g.neutral_player;
    let _enemy_sq = sess.g.spawn_squad_internal(
        neutral,
        target_plot,
        &[cataclysm_core::UnitClassId::Guard, cataclysm_core::UnitClassId::Archer],
    );

    let pt = sess.g.world.plot(target_plot);
    let cmd = Json::parse(&format!(r#"{{"c":"cat_attack","squad":{},"x":{},"y":{}}}"#, human_sq_id.0, pt.x, pt.y)).unwrap();
    let reply = sess.execute_json(&cmd).expect("attack succeeds");
    let report_id = reply.get("report").and_then(Json::as_i64).expect("attack returns battle report id") as u32;

    let log = sess.battle_log_json(report_id).expect("battle log v2");
    assert_eq!(log.get("v").and_then(Json::as_i64), Some(2));
    assert!(log.get("fighters").and_then(Json::as_arr).map_or(false, |a| a.len() >= 4));
    assert!(log.get("events").and_then(Json::as_arr).map_or(false, |a| !a.is_empty()));
    assert!(log.get("outcome").and_then(|o| o.get("rounds")).and_then(Json::as_i64).unwrap_or(0) >= 1);
    assert!(log.get("summary").and_then(|s| s.get("has_log")).and_then(Json::as_bool) == Some(true));
}

fn session(faction: &str, seed: u32) -> CataclysmSession {
    let setup = Json::parse(&format!(r#"{{"mode":"cataclysm","faction":"{faction}","size":"small","opponents":3,"seed":{seed}}}"#)).unwrap();
    CataclysmSession::new(&setup).expect("session")
}

fn q(sess: &CataclysmSession, req: &str) -> Json {
    sess.query(&Json::parse(req).unwrap()).expect("query succeeds")
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("{key} array missing in {j}"))
}

#[test]
fn cataclysm_report_carries_from_turn() {
    let sess = session("bannor", 5);
    let rep = q(&sess, r#"{"q":"report"}"#);
    assert_eq!(rep.get("from_turn").and_then(Json::as_i64), rep.get("turn").and_then(Json::as_i64));
    assert!(rep.get("from_turn").and_then(Json::as_i64).is_some());
}

#[test]
fn cataclysm_view_hides_foreign_city_details_and_describes_own_state() {
    let sess = session("bannor", 11);
    sess.set_reveal(true);
    let view = q(&sess, r#"{"q":"view"}"#);
    let cat = view.get("cataclysm").expect("cataclysm payload");

    // A rival's city: public fields only.
    let cities = arr(cat, "cities");
    let foreign = cities.iter().find(|c| c.get("owner").and_then(Json::as_i64) != Some(0)).expect("a rival city under reveal");
    assert_eq!(foreign.get("foreign").and_then(Json::as_bool), Some(true));
    assert!(foreign.get("building").is_none() && foreign.get("gold_rate").is_none() && foreign.get("crew").is_none());

    // Own squads carry their supply state.
    for s in arr(cat, "squads").iter().filter(|s| s.get("owner").and_then(Json::as_i64) == Some(0)) {
        let key = s.get("supply").and_then(|x| x.get("key")).and_then(Json::as_str).expect("supply.key");
        assert!(["supplied", "cut", "hungry", "starving", "recovering"].contains(&key));
        assert!(s.get("max_moves").and_then(Json::as_i64).unwrap() >= s.get("moves_left").and_then(Json::as_i64).unwrap());
    }

    // A Seventh Garrison capital cannot be razed by its founder, and says why.
    let capital = cities
        .iter()
        .find(|c| c.get("owner").and_then(Json::as_i64) == Some(0) && c.get("capital").and_then(Json::as_bool) == Some(true))
        .expect("own capital");
    let raze = capital.get("raze").expect("raze");
    assert_eq!(raze.get("allowed").and_then(Json::as_bool), Some(false));
    assert!(raze.get("why").and_then(Json::as_str).is_some_and(|w| !w.is_empty()));
    assert!(capital.get("crew").and_then(|c| c.get("text")).and_then(Json::as_str).is_some());

    // Standard city list: Sentinels as numbers.
    let std_cities = arr(&view, "cities");
    let own_std = std_cities.iter().find(|c| c.get("owner").and_then(Json::as_i64) == Some(0)).expect("own city");
    let sentinels = arr(own_std, "sentinels");
    assert!(!sentinels.is_empty(), "a capital has a Sentinel");
    assert!(sentinels.iter().all(|s| s.as_i64().is_some()));

    // Rules help for the in-game Help.
    let help = cat.get("rules_help").expect("rules_help");
    for k in ["supply", "squads", "battle", "cities", "houses"] {
        let lines = arr(help, k);
        assert!(!lines.is_empty() && lines.len() <= 8, "{k}: {} lines", lines.len());
    }
}

#[test]
fn cataclysm_rituals_say_why_when_mana_is_short() {
    let mut sess = session("amurites", 3);
    sess.g.players[0].mana = 0;
    let view = q(&sess, r#"{"q":"view"}"#);
    for r in arr(view.get("cataclysm").unwrap(), "rituals") {
        assert_eq!(r.get("can_cast").and_then(Json::as_bool), Some(false));
        assert!(r.get("why").and_then(Json::as_str).is_some_and(|w| w.contains("Mana")), "{r}");
    }
}

#[test]
fn cataclysm_setup_factions_carry_victory_lines() {
    let d = ffh_data::global();
    let setup = ffh_play::cataclysm::setup_json(d);
    for f in arr(&setup, "factions") {
        assert!(!arr(f, "victory_lines").is_empty(), "{f}");
        assert!(f.get("preferred_doctrine").is_none());
    }
}

#[test]
fn cataclysm_status_over_names_the_human_result() {
    let mut sess = session("bannor", 9);
    sess.g.apocalypse.outcome = Some(cataclysm_core::GameOutcome {
        kind: cataclysm_core::VictoryKind::MortalLifeboat,
        winners: vec![cataclysm_core::PlayerId(1), cataclysm_core::PlayerId(0)],
        summary: "test".into(),
    });
    let st = sess.status();
    let over = st.get("over").expect("over");
    assert_eq!(over.get("you_won").and_then(Json::as_bool), Some(true), "the human is a winner though not the first");
    assert_eq!(arr(over, "winner_factions").len(), 2);
    assert!(over.get("your_result").and_then(Json::as_str).is_some());
    assert!(!arr(over, "scores").is_empty());
}

#[test]
fn cataclysm_eliminated_human_sees_the_game_over() {
    let mut sess = session("bannor", 21);
    let me = cataclysm_core::PlayerId(0);
    // What the rules do at the world turn for a civilization with no city and no squad.
    for cid in sess.g.player_cities(me) {
        let plot = sess.g.cities[&cid].plot;
        sess.g.cities.remove(&cid);
        sess.g.world.plot_mut(plot).city = None;
    }
    for sid in sess.g.player_squads(me) {
        sess.g.squads.remove(&sid);
    }
    sess.g.players[0].alive = false;
    assert!(!sess.g.is_over(), "the rules' own outcome is untouched");
    let st = sess.status();
    assert_eq!(st.get("phase").and_then(Json::as_str), Some("over"));
    let over = st.get("over").expect("over");
    assert_eq!(over.get("you_won").and_then(Json::as_bool), Some(false));
    assert_eq!(over.get("cataclysm_kind").and_then(Json::as_str), Some("Eliminated"));
    assert!(arr(over, "winner_factions").is_empty());
    assert!(!arr(over, "scores").is_empty());
}

#[test]
fn cataclysm_diplomacy_states_offers_and_truces() {
    use cataclysm_core::{Command, PlayerId};
    let mut sess = session("bannor", 4);
    let (me, them) = (PlayerId(0), PlayerId(1));
    sess.g.set_war(me, them, true);
    sess.g.apply(them, Command::MakePeace { target: me }).unwrap();
    let view = q(&sess, r#"{"q":"view"}"#);
    let cat = view.get("cataclysm").unwrap();
    assert!(cat.get("peace_effect").and_then(Json::as_str).is_some_and(|t| t.contains("both sides")));
    let rival = arr(cat, "players").iter().find(|p| p.get("id").and_then(Json::as_i64) == Some(1)).unwrap().clone();
    assert_eq!(rival.get("peace_label").and_then(Json::as_str), Some("Accept peace"));
    assert_eq!(rival.get("can_peace").and_then(Json::as_bool), Some(true));
    assert!(rival.get("peace_offer_to_us").and_then(Json::as_i64).is_some());

    sess.g.apply(me, Command::MakePeace { target: them }).unwrap();
    let view = q(&sess, r#"{"q":"view"}"#);
    let rival = arr(view.get("cataclysm").unwrap(), "players").iter().find(|p| p.get("id").and_then(Json::as_i64) == Some(1)).unwrap().clone();
    assert_eq!(rival.get("can_declare_war").and_then(Json::as_bool), Some(false));
    assert!(rival.get("war_why").and_then(Json::as_str).is_some_and(|w| w.starts_with("Truce")), "{rival}");
    assert!(rival.get("truce_turns").and_then(Json::as_i64).unwrap() > 0);
}

#[test]
fn cataclysm_strain_ritual_states_its_recast_time() {
    use cataclysm_core::{Command, PlayerId, RitualKind};
    let mut sess = session("bannor", 6);
    sess.g.players[0].mana = 1000;
    sess.g.apply(PlayerId(0), Command::CastRitual { ritual: RitualKind::BlightStrike }).unwrap();
    let view = q(&sess, r#"{"q":"view"}"#);
    let r = arr(view.get("cataclysm").unwrap(), "rituals").iter().find(|r| r.get("key").and_then(Json::as_str) == Some("blight_strike")).unwrap().clone();
    assert_eq!(r.get("can_cast").and_then(Json::as_bool), Some(false));
    assert!(r.get("why").and_then(Json::as_str).is_some_and(|w| w.starts_with("Ready in")), "{r}");
    assert!(r.get("summary").and_then(Json::as_str).is_some_and(|t| t.contains("turns after each cast")), "{r}");
}
