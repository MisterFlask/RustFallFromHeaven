//! The bump (`Game::unit_jump_to_nearest_valid_plot`, docs/SUPPLY.md "Second pass" B): a land
//! unit of a player with a city on its landmass goes only to a plot connected over land to a city
//! of its owner, else to that city; a player without a city there is bumped as before. Situations
//! built by hand on a flat map.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians, the teams met and at peace, no open borders) on one flat block of
/// grassland (x 1..=38, y 2..=9) in an ocean, revealed to both teams; with `split`, column x = 10
/// is ocean, which cuts the block in two landmasses. Player 0 has a city on (5, 5), player 1 one
/// on (19, 8). Player 1 owns `p1_owns` plots; every other plot without a city is nobody's.
/// Player 0's warrior stands on `at`. Returns the game and the warrior.
fn world(split: bool, at: (i32, i32), p1_owns: impl Fn(i32, i32) -> bool) -> (Game, UnitId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=9).contains(&y) && (1..=38).contains(&x) && !(split && x == 10);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    g.player_init_city(p0, 5, 5, true, true);
    g.player_init_city(p1, 19, 8, true, true);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let u = g.player_init_unit(p0, warrior, at.0, at.1, None, None);
    for q in g.map.plot_ids() {
        if g.map.plots[q].plot_city.is_none() {
            let (x, y) = g.map.xy(q);
            let owner = if p1_owns(x, y) { Some(p1) } else { None };
            g.plot_set_owner(q, owner, false, false);
        }
    }
    assert!(!g.at_war(t0, t1) && !g.team_is_open_borders(t0, t1));
    (g, u)
}

/// Player 1 owns the block x 18..=24 (all of y 2..=9) but for a hole on (21, 5).
fn square_with_hole(x: i32, y: i32) -> bool {
    (18..=24).contains(&x) && (x, y) != (21, 5)
}

fn xy(g: &Game, u: UnitId) -> (i32, i32) {
    (g.units[u].x, g.units[u].y)
}

/// The hole is the nearest valid plot but a sealed pocket; the plots west of player 1's block
/// connect to player 0's city: the warrior goes there.
#[test]
fn a_bump_skips_a_sealed_pocket() {
    let (mut g, u) = world(false, (21, 4), square_with_hole);
    assert!(g.unit_jump_to_nearest_valid_plot(u));
    let (x, _) = xy(&g, u);
    assert_eq!(x, 17, "{:?}", xy(&g, u));
}

/// Player 1 owns every plot but the hole on (30, 5): the only plot connected to player 0's city is
/// the city's own; the warrior goes there.
#[test]
fn a_bump_without_a_connected_plot_goes_to_the_city() {
    let (mut g, u) = world(false, (29, 5), |x, y| (x, y) != (30, 5));
    assert!(g.unit_jump_to_nearest_valid_plot(u));
    assert_eq!(xy(&g, u), (5, 5));
}

/// Player 0's city is on another landmass (column 10 is ocean): the warrior is bumped by distance
/// alone, into the hole.
#[test]
fn a_bump_without_a_city_on_the_landmass_is_by_distance() {
    let (mut g, u) = world(true, (21, 4), square_with_hole);
    assert_ne!(g.map.plots[g.unit_plot(u)].area, g.map.plots[g.map.plot_xy(5, 5).unwrap()].area);
    assert!(g.unit_jump_to_nearest_valid_plot(u));
    assert_eq!(xy(&g, u), (21, 5));
}
