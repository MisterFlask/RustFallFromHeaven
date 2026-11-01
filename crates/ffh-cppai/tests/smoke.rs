//! The C++ AI port keeps playing: a mixed game (C++ AI on the even slots, ffh-ai on the odd ones,
//! as `scripts/h2h.py` plays) runs without a panic, and the C++ AI's units take the BETTER AI
//! group flags its unit AI depends on (without them every military unit only skips its turns).
//! A benchmark guard (docs/CPPAI.md), not a check of the port's fidelity.

use std::sync::Arc;

use ffh_core::defs::GROUPFLAG_NONE;
use ffh_core::game::Game;
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_data::GameStateType;

fn mixed_game(seed: u32) -> Game {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 4, "WORLDSIZE_SMALL", seed));
    g.map_script = Arc::new(FractalMapScript::default());
    let ffh = ffh_ai::ai_controller();
    let cpp = ffh_cppai::cppai_controller(seed);
    let barb = g.barbarian_player();
    for p in g.player_ids() {
        g.controllers[p] = if p != barb && p.0 % 2 == 0 { cpp.clone() } else { ffh.clone() };
    }
    g.autorun = true;
    g.game_init();
    g
}

#[test]
fn mixed_game_plays_and_cpp_units_get_group_flags() {
    let mut g = mixed_game(3);
    let mut flagged = 0;
    while g.game_state == GameStateType::GAMESTATE_ON && g.game_turn() < 60 {
        let turn = g.game_turn();
        for _ in 0..200_000 {
            g.update();
            if g.game_turn() != turn || g.game_state != GameStateType::GAMESTATE_ON {
                break;
            }
        }
        assert!(g.game_turn() != turn || g.game_state != GameStateType::GAMESTATE_ON, "turn {turn} does not end");
        for p in g.player_ids() {
            if g.controller(p).name() == "cppai" {
                flagged += g.players[p].units.iter().filter(|&&u| g.units[u].ai_groupflag != GROUPFLAG_NONE).count();
            }
        }
    }
    assert!(g.game_turn() >= 60, "the game ended at turn {}", g.game_turn());
    assert!(flagged > 0, "no C++ AI unit ever took a group flag");
}
