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
