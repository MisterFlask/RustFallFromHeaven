//! The sea, phase 1 (docs/NAVAL.md, 3.2): in boats a unit moves, waits or sleeps; the client's
//! selection offers it no other mission (it checks the selection as the rules check a group).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::selection;

/// A duel game on a block of grassland (y 2..=9) between two bands of coast, player 0 human with
/// a city on (5, 2).
fn world() -> Game {
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
        let land = (2..=9).contains(&y) && (1..=38).contains(&x);
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
    g.players[PlayerId(0)].human = true;
    g.players[PlayerId(0)].turn_active = true;
    g.player_init_city(PlayerId(0), 5, 2, true, true);
    g
}

/// The missions the selection offers `u` where it stands (tested as visible: the buttons shown).
fn offered(g: &Game, u: UnitId) -> Vec<MissionType> {
    let plot = g.unit_plot(u);
    (0..MissionType::NUM)
        .map(MissionType::from_usize)
        .filter(|&m| selection::is_player_mission(m) && m != MissionType::MISSION_MOVE_TO && m != MissionType::MISSION_ROUTE_TO)
        .filter(|&m| selection::can_start_mission(g, &[u], m, -1, -1, plot, true))
        .collect()
}

#[test]
fn in_boats_the_selection_offers_only_waiting() {
    let mut g = world();
    let ty = |name: &str| g.d.t::<UnitType>(name);
    let mut on_land = Vec::new();
    for name in ["UNIT_PROPHET", "UNIT_ARTIST", "UNIT_MERCHANT", "UNIT_ENGINEER", "UNIT_SLAVE", "UNIT_WARRIOR"] {
        let t = ty(name);
        let u = g.player_init_unit(PlayerId(0), t, 10, 2, None, None);
        on_land.extend(offered(&g, u));
        g.unit_move(u, g.map.plot_xy(10, 1).unwrap());
        assert!(g.unit_is_in_boats(u), "{name} in boats");
        let left: Vec<MissionType> = offered(&g, u)
            .into_iter()
            .filter(|m| !matches!(*m, MissionType::MISSION_SKIP | MissionType::MISSION_SLEEP | MissionType::MISSION_SENTRY))
            .collect();
        assert!(left.is_empty(), "{name} in boats is offered {left:?}");
        g.unit_kill(u, false, None);
    }
    // (The test means something: on land the great people have their missions.)
    assert!(on_land.iter().any(|m| matches!(*m, MissionType::MISSION_DISCOVER | MissionType::MISSION_GOLDEN_AGE | MissionType::MISSION_HURRY)), "{on_land:?}");
}
