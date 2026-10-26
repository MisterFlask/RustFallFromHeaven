//! Telemetry (ffh_core::telemetry, docs/TELEMETRY.md) only observes: a game played with it on
//! plays out exactly like the same game with it off (same `calculate_sync_checksum` every turn),
//! and it records what happened.
//!
//! Lives in ffh-ai because it needs the AI that drives the rules (as `save_resume.rs`).

use std::io::Write;
use std::sync::{Arc, Mutex};

use ffh_core::game::Game;
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_core::telemetry::Telemetry;
use ffh_data::GameStateType;

struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn new_game(seed: u32, civs: usize, world: &str, telemetry: Option<Arc<Mutex<Vec<u8>>>>) -> Game {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, civs, world, seed);
    let mut g = Game::new(d, setup);
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    if let Some(buf) = telemetry {
        g.telemetry = Telemetry::new(Some(Box::new(Shared(buf))));
    }
    g.game_init();
    g
}

fn play_turn(g: &mut Game) {
    for _ in 0..200_000 {
        g.update();
        if g.at_turn_boundary() || g.game_state != GameStateType::GAMESTATE_ON {
            return;
        }
    }
    panic!("turn {} does not end", g.game_turn());
}

#[test]
fn telemetry_does_not_change_play() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut off = new_game(1, 4, "WORLDSIZE_SMALL", None);
    let mut on = new_game(1, 4, "WORLDSIZE_SMALL", Some(events.clone()));
    for _ in 0..90 {
        play_turn(&mut off);
        play_turn(&mut on);
        assert_eq!(off.calculate_sync_checksum(), on.calculate_sync_checksum(), "turn {}", off.game_turn());
    }
    assert_eq!(off.save(), on.save(), "the saves differ");
    assert!(off.telemetry_json().is_none());
    let json = on.telemetry_json().expect("telemetry is on");
    for field in ["\"mission\"", "\"MISSION_MOVE_TO\"", "\"built_unit\"", "\"tech\"", "\"production\"", "\"research\"", "\"wars\""] {
        assert!(json.contains(field), "{field} missing from {json}");
    }
    let events = String::from_utf8(events.lock().unwrap().clone()).unwrap();
    assert!(events.lines().count() > 50, "{} events", events.lines().count());
    assert!(events.lines().all(|l| l.starts_with("{\"t\":") && l.ends_with('}')));
    for kind in ["\"e\":\"tech\"", "\"e\":\"found\"", "\"e\":\"decision\""] {
        assert!(events.contains(kind), "no {kind} event");
    }
}
