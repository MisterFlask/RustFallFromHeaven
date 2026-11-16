//! Terra's geographic and resource contracts hold after complete start normalization and load.
use ffh_core::Game;
use ffh_data::*;
use ffh_play::setup::{build_game, build_observed_game};
use ffh_play::{Json, NewGame};

fn check(g: &Game) {
    let mut land: Vec<_> = g
        .map
        .areas
        .iter()
        .filter(|(_, a)| !a.water)
        .map(|(a, _)| {
            let habitable = g
                .map
                .plot_ids()
                .filter(|&p| g.map.plots[p].area == Some(a) && !g.plot_is_impassable(p))
                .count();
            (a, habitable)
        })
        .collect();
    land.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    assert_eq!(land.len(), 2, "exactly two continents");
    let (old, old_size) = land[0];
    let (new, new_size) = land[1];
    let bounds = |area| {
        let xs: Vec<_> = g
            .map
            .plot_ids()
            .filter(|&p| g.map.plots[p].area == Some(area))
            .map(|p| g.map.xy(p).0)
            .collect();
        (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
    };
    let a = bounds(old);
    let b = bounds(new);
    let (left, right) = if a.1 < b.0 { (a.1, b.0) } else { (b.1, a.0) };
    assert!(
        right - left - 1 >= 12,
        "the ocean gap is between the continents"
    );
    let ocean = g.d.t::<TerrainType>("TERRAIN_OCEAN");
    for x in left + 2..right - 1 {
        for y in 0..g.map.grid_height {
            assert_eq!(
                g.map.plots[g.map.plot_idx(x, y).unwrap()].terrain_type,
                Some(ocean),
                "deep water prevents a coastal stepping-stone route"
            );
        }
    }

    assert!(
        new_size * 100 >= old_size * 26 && new_size * 100 <= old_size * 44,
        "habitable size {old_size}/{new_size}"
    );
    let mut starts = Vec::new();
    for p in g
        .player_ids()
        .filter(|&p| g.players[p].alive && p != g.setup.barbarian)
    {
        let start = g.player_get_starting_plot(p).unwrap();
        assert_eq!(
            g.map.plots[start].area,
            Some(old),
            "all civilizations start in the Old World"
        );
        assert!(!g.plot_is_impassable(start));
        assert!(!starts.contains(&start));
        starts.push(start);
    }
    for name in ["BONUS_GEMS", "BONUS_SILK", "BONUS_INCENSE", "BONUS_MITHRIL"] {
        let b = g.d.t::<BonusType>(name);
        let copies: Vec<_> = g
            .map
            .plot_ids()
            .filter(|&p| g.map.plots[p].bonus_type == Some(b))
            .collect();
        assert!(copies.len() >= 2, "guaranteed overseas {name}");
        for p in copies {
            assert_eq!(
                g.map.plots[p].area,
                Some(new),
                "no {name} leaks into the Old World"
            );
            assert!(!g.plot_is_impassable(p));
            // Check physical compatibility independently of the resource occupancy guard.
            let info = &g.d.bonuses[b];
            let plot = &g.map.plots[p];
            if let Some(f) = plot.feature_type {
                assert!(info.feature[f] && info.feature_terrain[plot.terrain_type.unwrap()]);
            } else {
                assert!(info.terrain[plot.terrain_type.unwrap()]);
            }
        }
    }
    assert!(!g.map.wrap_x && !g.map.wrap_y);
}

#[test]
fn terra_selection_is_validated_and_recorded_for_human_and_observed_games() {
    let d = ffh_data::global();
    let ng =
        NewGame::from_json(d, &Json::parse(r#"{"map_script":"ErebusTerra"}"#).unwrap()).unwrap();
    assert_eq!(NewGame::from_json(d, &ng.to_json()).unwrap(), ng);
    assert_eq!(build_game(d, &ng).unwrap().map_script.name(), "ErebusTerra");
    assert_eq!(
        build_observed_game(d, &ng).unwrap().map_script.name(),
        "ErebusTerra"
    );
}

#[test]
fn populated_old_world_empty_new_world_and_exclusives_survive_generation_and_loading() {
    let d = ffh_data::global();
    for (size, civs) in [
        ("WORLDSIZE_DUEL", 2),
        ("WORLDSIZE_SMALL", 4),
        ("WORLDSIZE_STANDARD", 6),
    ] {
        for seed in 1..=12 {
            let mut ng = NewGame::new(d, seed);
            ng.map_script = "ErebusTerra".into();
            ng.world = d.t(size);
            ng.opponents = civs - 1;
            let mut g = build_game(d, &ng).unwrap();
            g.game_init();
            check(&g);
            let bytes = g.save();
            let loaded = Game::load(d, &bytes).unwrap();
            assert_eq!(loaded.map_script.name(), "ErebusTerra");
            assert_eq!(loaded.save(), bytes);
            check(&loaded);
        }
    }
}

#[test]
fn climate_sea_level_and_observer_settings_preserve_the_terra_contract() {
    let d = ffh_data::global();
    for (climate, sea) in [
        ("CLIMATE_COLD", "SEALEVEL_LOW"),
        ("CLIMATE_ARID", "SEALEVEL_HIGH"),
        ("CLIMATE_TROPICAL", "SEALEVEL_MEDIUM"),
    ] {
        let mut ng = NewGame::new(d, 31);
        ng.map_script = "ErebusTerra".into();
        ng.world = d.t("WORLDSIZE_DUEL");
        ng.opponents = 1;
        ng.climate = d.t(climate);
        ng.sea_level = d.t(sea);
        let mut g = build_observed_game(d, &ng).unwrap();
        g.game_init();
        check(&g);
    }
}
