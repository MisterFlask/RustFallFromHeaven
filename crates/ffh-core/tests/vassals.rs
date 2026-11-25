//! Vassals cost their master nothing (docs/PORTING.md, "Changes from FFH2"): a vassal's cities do
//! not count in the master's number-of-cities maintenance; on a situation built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

#[test]
fn a_vassal_adds_nothing_to_its_masters_maintenance() {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 16, ..Default::default() }, np, nt);
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for q in g.map.plot_ids() {
        g.plot_set_plot_type(q, PlotType::PLOT_LAND, false);
        g.plot_set_terrain_type(q, grass, false);
    }
    g.map_recalculate_areas();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    let capital = g.player_init_city(p0, 4, 7, true, true);
    let other = g.player_init_city(p0, 10, 7, true, true);
    for x in [22, 28, 34] {
        g.player_init_city(p1, x, 7, true, true);
    }
    for c in [capital, other] {
        g.city_update_maintenance(c);
    }

    let read = |g: &Game| {
        ([capital, other].map(|c| g.city_calculate_num_cities_maintenance_times100(c)), [capital, other].map(|c| g.cities[c].maintenance), g.player_get_total_maintenance(p0))
    };
    let before = read(&g);
    assert!(before.0.iter().all(|&n| n > 0), "the master's cities pay a city-count part ({:?})", before.0);

    g.team_set_vassal(t1, t0, true, false);
    assert!(g.teams[t1].vassal[t0], "player 1 is player 0's vassal");
    for c in [capital, other] {
        g.city_update_maintenance(c);
    }
    assert_eq!(read(&g), before, "three vassal cities change nothing in the master's maintenance");
}
