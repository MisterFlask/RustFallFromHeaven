//! Investment is public state; previews and confirmations leave the game untouched.
use std::sync::Arc;
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::{Session, SessionOptions, Json};
use ffh_play::command::{Command, CmdOk, Confirm, Ctx, Plan};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;

#[test]
fn a_withdrawal_previews_and_confirms_starvation_without_changing_the_game() {
    let root = std::env::temp_dir().join(format!("ffh-siege-confirm-{}", std::process::id()));
    let fixture = ffh_play::stage::siege(&root, "siege-attacker").unwrap();
    let bytes = std::fs::read(fixture.req::<String>("save").unwrap()).unwrap();
    let mut s = Session::load(&bytes, None, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    let city = fixture.req::<CityId>("city").unwrap();
    let unit = fixture.req::<UnitId>("unit").unwrap();
    let to = fixture.req::<Vec<i32>>("withdraw").unwrap();
    let (x, y) = (to[0], to[1]);
    assert_eq!(s.g.cities[city].siege_turns, 3);
    let before = s.g.save();
    let preview = s.query(&Query::Path { units: vec![unit], x, y, flags: 0 }).unwrap();
    assert_eq!(preview.get("ok").and_then(Json::as_bool), Some(true));
    let ends = preview.get("ends_sieges").and_then(Json::as_arr).unwrap();
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].get("city"), Some(&Json::from(city)));
    assert!(ends[0].req::<String>("text").unwrap().contains("turn 3, Starvation"));
    let move_cmd = |confirmed| Command::Move { units: vec![unit], x, y, append: false, declare_war: None, confirm_end_siege: confirmed };
    let reply = s.execute(&move_cmd(false)).unwrap();
    assert!(matches!(reply, CmdOk::NeedsConfirm(Confirm::EndSiege { .. })));
    let mission = Command::Mission { units: vec![unit], mission: MissionType::MISSION_MOVE_TO, data1: x, data2: y, flags: 0, append: false, confirm_end_siege: false };
    assert!(matches!(s.execute(&mission).unwrap(), CmdOk::NeedsConfirm(Confirm::EndSiege { .. })));
    assert_eq!(s.g.save(), before, "neither preview nor unanswered confirmation changes state or RNG");
    s.g.cities[city].siege_turns = 2;
    assert!(matches!(ffh_play::command::validate(&s.g, s.human, &move_cmd(false), &Ctx { events: &[] }).unwrap(), Plan::Apply(_)), "Hunger alone asks no confirmation");
    s.g.cities[city].siege_turns = 3;
    assert!(matches!(s.execute(&move_cmd(true)).unwrap(), CmdOk::Applied { .. }));
    assert!(!s.g.city_is_invested(city));
    assert_eq!(s.g.cities[city].siege_turns, 3, "withdrawal does not immediately clear Starvation");
    let owner = s.g.cities[city].owner;
    s.g.player_update_supply(owner);
    let v = ffh_play::view::Viewer::new(&s.g, s.human, false);
    let state = ffh_play::view::sieges::city(&v, city);
    assert_eq!(state.get("relief_turns"), Some(&Json::from(1)));
    assert_eq!(state.get("state").and_then(Json::as_str), Some("starvation"));
    s.g.player_update_supply(owner);
    assert_eq!(s.g.cities[city].siege_turns, 0);
    std::fs::remove_dir_all(root).unwrap();
}
