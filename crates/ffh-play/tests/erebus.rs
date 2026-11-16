//! Erebus is a real new-game choice, including observed games and persisted setup.

use ffh_core::Game;
use ffh_play::setup::{build_game, build_observed_game, setup_lists};
use ffh_play::{Json, NewGame};

#[test]
fn generator_selection_round_trips_and_rejects_unknown_names() {
    let d = ffh_data::global();
    let old = NewGame::from_json(d, &Json::parse(r#"{"seed":2}"#).unwrap()).unwrap();
    assert_eq!(old.map_script, "Fractal");
    let ng = NewGame::from_json(
        d,
        &Json::parse(r#"{"map_script":"Erebus","seed":2}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(NewGame::from_json(d, &ng.to_json()).unwrap(), ng);
    assert_eq!(NewGame::from_json(d, &ng.to_json_names(d)).unwrap(), ng);
    assert_eq!(build_game(d, &ng).unwrap().map_script.name(), "Erebus");
    assert_eq!(
        build_observed_game(d, &ng).unwrap().map_script.name(),
        "Erebus"
    );
    let error =
        NewGame::from_json(d, &Json::parse(r#"{"map_script":"NotAMap"}"#).unwrap()).unwrap_err();
    assert_eq!(error.key, "map_script");
    let lists = setup_lists(d);
    let names: Vec<_> = lists
        .req_arr("map_scripts")
        .unwrap()
        .iter()
        .map(|j| j.req_str("type").unwrap())
        .collect();
    assert_eq!(names, ["Fractal", "Erebus", "ErebusTerra"]);
}

#[test]
fn erebus_games_have_valid_starts_and_keep_the_selected_script_on_load() {
    let d = ffh_data::global();
    for (world, players, seed) in [
        ("WORLDSIZE_DUEL", 2, 1),
        ("WORLDSIZE_SMALL", 4, 7),
        ("WORLDSIZE_STANDARD", 6, 19),
    ] {
        let mut ng = NewGame::new(d, seed);
        ng.map_script = "Erebus".into();
        ng.world = d.t(world);
        ng.opponents = players - 1;
        let mut g = build_game(d, &ng).unwrap();
        g.game_init();
        assert!(!g.map.wrap_x && !g.map.wrap_y);
        assert_eq!(g.map.grid_width, g.map.grid_height);
        let mut starts = Vec::new();
        for p in g
            .player_ids()
            .filter(|&p| p != g.setup.barbarian && g.players[p].alive)
        {
            let start = g
                .player_get_starting_plot(p)
                .expect("every player has a start");
            assert!(!g.plot_is_water(start) && !g.plot_is_impassable(start));
            assert!(!starts.contains(&start), "distinct starting plots");
            starts.push(start);
        }
        assert_eq!(starts.len(), players);
        assert!(
            g.map.plots.iter().any(|p| p.n_of_river || p.w_of_river),
            "valleys have rivers"
        );
        let saved = g.save();
        let loaded = Game::load(d, &saved).unwrap();
        assert_eq!(loaded.setup.map_script, "Erebus");
        assert_eq!(loaded.map_script.name(), "Erebus");
        assert_eq!(loaded.save(), saved, "loading keeps game state unchanged");
    }
}

#[test]
fn sea_level_sizes_and_climate_settings_reach_erebus() {
    let d = ffh_data::global();
    let mut sizes = Vec::new();
    for level in ["SEALEVEL_LOW", "SEALEVEL_MEDIUM", "SEALEVEL_HIGH"] {
        let mut ng = NewGame::new(d, 13);
        ng.map_script = "Erebus".into();
        ng.world = d.t("WORLDSIZE_DUEL");
        ng.opponents = 1;
        ng.sea_level = d.t(level);
        ng.climate = d.t("CLIMATE_COLD");
        let mut g = build_game(d, &ng).unwrap();
        g.game_init();
        sizes.push(g.map.grid_width);
        assert!(g
            .map
            .plots
            .iter()
            .any(|p| p.terrain_type == Some(d.t("TERRAIN_SNOW"))));
    }
    assert!(
        sizes[0] < sizes[1] && sizes[1] < sizes[2],
        "sea levels retain room for playable valleys"
    );
}
