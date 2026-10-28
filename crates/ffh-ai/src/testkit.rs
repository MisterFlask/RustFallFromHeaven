//! Test fixtures shared by the modules' unit tests.

use std::sync::Arc;

use ffh_core::game::Game;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// One block of grassland (x 2..=17, y 3..=8) in an ocean, revealed to both players: a fresh
/// two-player AI game whose starting units are gone (as `tests/magic.rs`).
pub(crate) fn one_landmass_game(seed: u32) -> Game {
    one_landmass_game_with(seed, &[])
}

/// [`one_landmass_game`] with the first players' civilizations given (each with its first
/// leader).
pub(crate) fn one_landmass_game_with(seed: u32, civs: &[&str]) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", seed);
    for (slot, name) in civs.iter().enumerate() {
        let civ = d.t::<CivilizationType>(name);
        setup.players[slot].civ = Some(civ);
        setup.players[slot].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
    }
    let mut g = Game::new(d, setup);
    g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = crate::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 20, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for p in g.map.plot_ids() {
        let (x, y) = g.map.xy(p);
        let land = (3..=8).contains(&y) && (2..=17).contains(&x);
        g.plot_set_plot_type(p, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(p, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    for t in g.team_ids() {
        for p in g.map.plot_ids() {
            g.plot_set_revealed(p, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    g
}
