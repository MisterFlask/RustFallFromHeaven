//! Observed games and flags (docs/HUMAN.md, "Watching the AI"): an AI-only game watched by a
//! spectator stops at turn boundaries, takes only the commands that play turns, follows any
//! civilization, shows every player's messages, saves, loads and replays as observed; a flag
//! pins the save its replay starts from, and that save played to the flagged turn gives the
//! checksum the flag recorded (so does the game replayed from its settings).

mod common;

use std::sync::Arc;

use common::TempDir;
use ffh_data::*;
use ffh_play::human::AdvisorAnswerer;
use ffh_play::journal;
use ffh_play::save::read_bundle;
use ffh_play::session::{replay, Phase};
use ffh_play::{CmdErr, CmdOk, Command, Json, NewGame, Session, SessionOptions, Stop};

fn observed(seed: u32) -> NewGame {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, seed);
    ng.opponents = 3;
    ng.world = d.t::<WorldSizeType>("WORLDSIZE_TINY");
    ng.observe = true;
    ng
}

fn session(ng: &NewGame, saves: Option<std::path::PathBuf>) -> Session {
    let opts = SessionOptions { saves_dir: saves, ..SessionOptions::default() };
    Session::new_game(ng, Arc::new(AdvisorAnswerer), opts).expect("new game")
}

/// Plays `turns` turns one at a time (each stops at the next turn boundary).
fn play(s: &mut Session, turns: i32) {
    for _ in 0..turns {
        let t = s.g.game_turn();
        assert!(matches!(s.execute(&Command::AutoPlay { turns: 1 }), Ok(CmdOk::Applied { .. })));
        assert_eq!(s.advance(&mut |_| {}), Stop::Human);
        assert!(s.g.at_turn_boundary(), "an observed game stops at turn boundaries");
        assert!(s.g.game_turn() == t + 1 || (t == s.g.start_turn && s.g.game_turn() == t), "one turn a step");
    }
}

fn checksum_hex(s: &Session) -> String {
    format!("{:08x}", s.g.calculate_sync_checksum() as u32)
}

#[test]
fn observed_games_stop_at_boundaries_take_only_turns_and_follow_any_civ() {
    let d = ffh_data::global();
    let mut s = session(&observed(3), None);
    assert_eq!(s.advance(&mut |_| {}), Stop::Human, "a new observed game stops at its start");
    assert_eq!(s.phase(), Phase::Human(ffh_core::ids::PlayerId(0)));
    assert_eq!(s.observe_state(), Some(0));
    assert_eq!(s.g.num_human_players(), 0, "nobody is human");
    let st = s.query_json(&ffh_play::jobj! { "q" => "status" }).unwrap();
    assert_eq!(st.get("observe").and_then(Json::as_bool), Some(true));
    assert_eq!(st.get("reveal").and_then(Json::as_bool), Some(true), "the observer sees the whole map");
    assert_eq!(st.get("phase").and_then(Json::as_str), Some("human"));

    // Nothing but turns can be played.
    let research = Command::from_json(d, &ffh_play::jobj! { "c" => "research", "tech" => Json::Null }).unwrap();
    assert!(matches!(s.execute(&research), Err(CmdErr::Cannot(_))));
    assert!(matches!(s.execute(&Command::AutoPlay { turns: 0 }), Err(CmdErr::Invalid(_))));
    // end_turn plays one turn: the first ends at the first boundary.
    assert!(s.execute(&Command::EndTurn { force: false }).is_ok());
    assert_eq!(s.query_json(&ffh_play::jobj! { "q" => "status" }).unwrap().get("phase").and_then(Json::as_str), Some("running"));
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    assert!(s.g.at_turn_boundary());
    let t0 = s.g.game_turn();
    // autoplay N plays N turns before the next stop.
    assert!(s.execute(&Command::AutoPlay { turns: 4 }).is_ok());
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    assert_eq!(s.g.game_turn(), t0 + 4);
    play(&mut s, 10);

    // The view follows a civilization; another one can be followed (the game is not changed).
    let view = s.query_json(&ffh_play::jobj! { "q" => "view" }).unwrap();
    assert_eq!(view.get("me").and_then(|m| m.get("player")).and_then(Json::as_i64), Some(0));
    let before = checksum_hex(&s);
    let other = s.g.civ_player_ids().find(|&p| p.0 != 0 && s.g.players[p].alive).expect("another civ");
    s.set_viewer(other).unwrap();
    let view = s.query_json(&ffh_play::jobj! { "q" => "view" }).unwrap();
    assert_eq!(view.get("me").and_then(|m| m.get("player")).and_then(Json::as_i64), Some(other.0 as i64));
    assert_eq!(checksum_hex(&s), before);
    assert!(s.pending() == ffh_play::Pending::default(), "nothing waits for an observer");

    // Every player's messages reach the observer; combat messages say where they happened.
    let players: std::collections::BTreeSet<_> = s.messages().iter().filter_map(|e| e.player).collect();
    eprintln!("messages from players {players:?}");
    for e in s.messages() {
        if e.text.contains(" died attacking ") || e.text.contains(" withdrew from ") {
            assert!(e.at.is_some(), "a combat message has its plot: {e:?}");
        }
    }
}

#[test]
fn human_games_follow_no_one_else() {
    let mut ng = observed(4);
    ng.observe = false;
    let mut s = session(&ng, None);
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    assert_eq!(s.observe_state(), None);
    assert!(s.set_viewer(ffh_core::ids::PlayerId(1)).is_err());
}

#[test]
fn observed_games_save_load_and_replay_as_observed() {
    let dir = TempDir::new("observe-save");
    let mut s = session(&observed(5), Some(dir.0.clone()));
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    play(&mut s, 12);
    let (bytes, sidecar) = s.save_bytes().unwrap();
    assert!(sidecar.observe && sidecar.setup.is_some());
    let mut l = Session::load(&bytes, Some(&sidecar), Arc::new(AdvisorAnswerer), SessionOptions { journal: false, ..SessionOptions::default() }).unwrap();
    assert_eq!(l.observe_state(), Some(0));
    assert_eq!(l.advance(&mut |_| {}), Stop::Human);
    play(&mut s, 3);
    play(&mut l, 3);
    assert_eq!(checksum_hex(&l), checksum_hex(&s), "a loaded observed game goes on as the uninterrupted one");

    // An AI-only save without a sidecar is observed too.
    let mut bare = Session::load(&bytes, None, Arc::new(AdvisorAnswerer), SessionOptions { journal: false, ..SessionOptions::default() }).unwrap();
    assert!(bare.observe_state().is_some());
    assert_eq!(bare.advance(&mut |_| {}), Stop::Human);

    // The journal replays to the same game.
    let lines = journal::read_file(&s.journal_path().unwrap()).unwrap();
    let r = replay(lines, None, SessionOptions::default()).expect("replay");
    assert_eq!(checksum_hex(&r), checksum_hex(&s));
}

#[test]
fn flags_pin_their_start_save_and_replay_to_their_checksum() {
    let d = ffh_data::global();
    let dir = TempDir::new("observe-flag");
    let ng = observed(6);
    let mut s = session(&ng, Some(dir.0.clone()));
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    play(&mut s, 9);
    let u = s.g.civ_player_ids().filter(|&p| p.0 != 0).flat_map(|p| s.g.players[p].units.clone()).next().expect("a unit");
    let (x, y) = (s.g.units[u].x, s.g.units[u].y);
    let rec = s.flag(x, y, "a test flag").unwrap();
    assert_eq!(rec.get("flag").and_then(Json::as_i64), Some(1));
    assert_eq!(rec.get("observed").and_then(Json::as_bool), Some(true));
    let moved = rec.get("moved_turn").and_then(Json::as_i64).expect("the turn whose moves are shown") as i32;
    assert_eq!(moved, s.g.game_turn(), "observed: the turn that just ended");
    assert_eq!(rec.get("checksum").and_then(Json::as_str), Some(checksum_hex(&s).as_str()));
    assert_eq!(rec.get("build").and_then(Json::as_str), Some(ffh_play::BUILD_COMMIT));
    let units = rec.get("units").and_then(Json::as_arr).unwrap();
    assert!(units.iter().any(|j| j.get("id").and_then(Json::as_i64) == Some(u.0 as i64)), "the flagged unit is listed");
    let repro: Vec<&str> = rec.get("repro").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
    assert!(repro.iter().any(|r| r.contains("ffh-play observe --setup")), "{repro:?}");
    assert!(repro.iter().any(|r| r.contains("ffh-play observe --load")), "{repro:?}");
    assert!(repro.iter().any(|r| r.contains("ffh autoplay")), "{repro:?}");
    let text = rec.get("text").and_then(Json::as_str).unwrap();
    assert!(text.contains("a test flag") && text.contains(&format!("({x}, {y})")), "{text}");
    assert_eq!(ffh_play::flag::list(Some(&dir.0)).len(), 1);

    // The pinned save, played to the flagged turn, is the flagged state.
    let pinned = std::path::PathBuf::from(rec.get("start_save").and_then(Json::as_str).expect("a pinned save"));
    assert!(pinned.starts_with(&dir.0) && pinned.is_file());
    let (bytes, sidecar) = read_bundle(&pinned).unwrap();
    let mut p = Session::load(&bytes, sidecar.as_ref(), Arc::new(AdvisorAnswerer), SessionOptions { journal: false, ..SessionOptions::default() }).unwrap();
    assert_eq!(p.advance(&mut |_| {}), Stop::Human);
    assert_eq!(p.g.game_turn(), moved - 1);
    play(&mut p, 1);
    assert_eq!((p.g.game_turn(), checksum_hex(&p)), (moved, checksum_hex(&s)));

    // So is the game replayed from its settings (the setup the flag quotes).
    let setup = NewGame::from_json(d, rec.get("setup").unwrap()).unwrap();
    assert!(setup.observe);
    let mut f = session(&setup, None);
    assert_eq!(f.advance(&mut |_| {}), Stop::Human);
    while f.g.game_turn() < moved || !f.g.at_turn_boundary() {
        play(&mut f, 1);
    }
    assert_eq!(checksum_hex(&f), checksum_hex(&s));

    // A second flag is numbered on.
    let rec2 = s.flag(x, y, "").unwrap();
    assert_eq!(rec2.get("flag").and_then(Json::as_i64), Some(2));
}

#[test]
fn flags_of_a_human_game_quote_its_journal() {
    let dir = TempDir::new("observe-human-flag");
    let mut ng = observed(7);
    ng.observe = false;
    let mut s = session(&ng, Some(dir.0.clone()));
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let first = s.flag(0, 0, "").unwrap();
    assert!(first.get("moved_turn").unwrap().is_null(), "before any turn, nothing moved");
    assert!(s.execute(&Command::EndTurn { force: true }).is_ok());
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let rec = s.flag(0, 0, "").unwrap();
    assert_eq!(rec.get("observed").and_then(Json::as_bool), Some(false));
    assert_eq!(rec.get("moved_turn").and_then(Json::as_i64), Some((s.g.game_turn() - 1) as i64), "the others moved last turn");
    let repro: Vec<&str> = rec.get("repro").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
    assert!(repro.iter().any(|r| r.contains("ffh-play replay") && r.contains("--to-turn")), "{repro:?}");
}
