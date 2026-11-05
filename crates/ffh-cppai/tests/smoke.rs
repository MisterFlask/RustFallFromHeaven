//! The C++ AI port keeps playing: a mixed game (C++ AI on the even slots, ffh-ai on the odd ones,
//! as `scripts/h2h.py` plays) runs without a panic, and the C++ AI's units take the BETTER AI
//! group flags its unit AI depends on (without them every military unit only skips its turns).
//! A benchmark guard (docs/CPPAI.md), not a check of the port's fidelity.

use std::sync::Arc;

use ffh_core::defs::GROUPFLAG_NONE;
use ffh_core::game::Game;
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_data::{GameOptionType, GameStateType};

/// A 4-civ game that cannot end within the test's turns: always peace and no barbarian units (the
/// options of `common::STEADY` in `crates/ffh-play/tests`), so the test does not depend on how the
/// seed's game goes.
fn mixed_game(seed: u32) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 4, "WORLDSIZE_SMALL", seed);
    for o in [
        GameOptionType::GAMEOPTION_ALWAYS_PEACE,
        GameOptionType::GAMEOPTION_NO_BARBARIANS,
        GameOptionType::GAMEOPTION_NO_LAIRS,
        GameOptionType::GAMEOPTION_NO_GOODY_HUTS,
        GameOptionType::GAMEOPTION_NO_UNIQUE_IMPROVEMENTS,
        GameOptionType::GAMEOPTION_NO_ORTHUS,
    ] {
        setup.options[o] = true;
    }
    let mut g = Game::new(d, setup);
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
