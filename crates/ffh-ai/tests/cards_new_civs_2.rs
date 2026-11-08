//! Batch B8's civilizations under the AI (docs/CARDS.md, section 6): each is born on a generated
//! world as its card makes it (the land it needs painted by hand), then the AI plays every
//! civilization for some turns of a steady game (always peace, no barbarians). Each born
//! civilization keeps its cities, produces, and its turns run without a fault.

use std::sync::Arc;

use ffh_core::ffh::cards::new_civs_2 as nc;
use ffh_core::ffh::cards::world;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// The steady options (ffh-play's tests' `STEADY`): a game nobody can lose in a few turns.
const STEADY: [GameOptionType; 6] = [
    GameOptionType::GAMEOPTION_ALWAYS_PEACE,
    GameOptionType::GAMEOPTION_NO_BARBARIANS,
    GameOptionType::GAMEOPTION_NO_LAIRS,
    GameOptionType::GAMEOPTION_NO_GOODY_HUTS,
    GameOptionType::GAMEOPTION_NO_UNIQUE_IMPROVEMENTS,
    GameOptionType::GAMEOPTION_NO_ORTHUS,
];

/// How many turns the AI plays after the births.
const TURNS: i32 = 15;

/// A steady 3-civilization game on a small Fractal world, the AI on every slot, each civilization
/// with a capital on its starting plot.
fn new_game(seed: u32) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 3, "WORLDSIZE_SMALL", seed);
    for o in STEADY {
        setup.options[o] = true;
    }
    let mut g = Game::new(d, setup);
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    g.game_init();
    let civs: Vec<PlayerId> = world::civ_players(&g);
    for p in civs {
        if let Some(start) = g.player_get_starting_plot(p) {
            if world::free_site(&g, start, Some(p)) || g.map.plots[start].units.iter().all(|&u| g.units[u].owner == p) {
                let (x, y) = g.map.xy(start);
                g.player_init_city(p, x, y, true, true);
            }
        }
    }
    g
}

/// Plays to the next turn boundary.
fn play_turn(g: &mut Game) {
    for _ in 0..200_000 {
        g.update();
        if g.at_turn_boundary() || g.game_state != GameStateType::GAMESTATE_ON {
            return;
        }
    }
    panic!("turn {} does not end", g.game_turn());
}

/// A `cols` x `rows` block of unowned land away from every city (no peak, unit or city), flat and
/// cleared of resources and improvements; `None` when the world has none.
fn block(g: &mut Game, cols: i32, rows: i32, avoid: &[PlotIdx]) -> Option<Vec<PlotIdx>> {
    let open = |g: &Game, q: PlotIdx| {
        let p = &g.map.plots[q];
        !g.plot_is_water(q) && !g.plot_is_peak(q) && p.owner.is_none() && p.units.is_empty() && p.plot_city.is_none() && !avoid.contains(&q) && !g.plot_is_city_radius(q)
    };
    let found = g.map.plot_ids().find_map(|q| {
        let (x, y) = g.map.xy(q);
        let plots: Option<Vec<PlotIdx>> = (0..cols * rows).map(|i| g.map.plot_xy(x + i % cols, y + i / cols).filter(|&t| open(g, t))).collect();
        plots.filter(|p| {
            let mut s = p.clone();
            s.sort();
            s.dedup();
            s.len() == (cols * rows) as usize
        })
    })?;
    for &q in &found {
        g.plot_set_bonus_type(q, None);
        g.plot_set_improvement_type(q, None);
        if g.plot_is_hills(q) {
            g.plot_set_plot_type(q, PlotType::PLOT_LAND, true);
        }
    }
    Some(found)
}

/// `plots` and those within 3 of them.
fn margin(g: &Game, plots: &[PlotIdx]) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for &q in plots {
        let (x, y) = g.map.xy(q);
        out.extend((0..49).filter_map(|i| g.map.plot_xy(x + i % 7 - 3, y + i / 7 - 3)));
    }
    out
}

/// What a civilization has made: its units and buildings, and the hammers in its cities' current
/// orders.
fn made(g: &Game, p: PlayerId) -> usize {
    let buildings: i32 = g.players[p].cities.iter().map(|&c| g.d.buildings.keys().map(|b| g.cities[c].num_real_building[b]).sum::<i32>()).sum();
    let progress: i32 = g.players[p].cities.iter().map(|&c| g.city_production(c)).sum();
    g.players[p].units.len() + (buildings + progress) as usize
}

/// The AI plays `TURNS` turns; each of `born` stays alive with its cities and produces.
fn play_and_check(g: &mut Game, born: &[(PlayerId, &str)]) {
    let before: Vec<usize> = born.iter().map(|&(p, _)| made(g, p)).collect();
    let cities: Vec<usize> = born.iter().map(|&(p, _)| g.players[p].cities.len()).collect();
    let start = g.game_turn();
    for _ in 0..TURNS * 100 {
        if g.game_turn() >= start + TURNS || g.game_state != GameStateType::GAMESTATE_ON {
            break;
        }
        play_turn(g);
    }
    assert!(g.game_turn() >= start + TURNS, "the turns are played");
    for (i, &(p, name)) in born.iter().enumerate() {
        assert!(g.players[p].alive, "{name} lives");
        assert!(g.players[p].cities.len() >= cities[i], "{name} keeps its cities");
        assert!(made(g, p) > before[i] || g.players[p].cities.len() > cities[i], "{name} produces");
    }
}

#[test]
fn the_armageddon_civilizations_play_under_the_ai() {
    let mut g = new_game(5);
    let d = g.d;
    let mut born = Vec::new();
    // The Scaled Kingdom, in a marsh painted on free land.
    let marsh = d.t::<TerrainType>("TERRAIN_MARSH");
    let swamp = block(&mut g, 7, 5, &[]).expect("free land for a marsh");
    for &q in &swamp {
        g.plot_set_feature_type(q, None);
        g.plot_set_terrain_type(q, marsh, true);
    }
    let kingdom = world::next_player(&g);
    assert!(nc::valid_scaled_kingdom(&g));
    nc::fire_scaled_kingdom(&mut g, 0);
    born.push((kingdom, "the Scaled Kingdom"));
    // The Brood, from a nest in a forest painted on free land (its nests' spawns need barbarians,
    // which this game has none of: its stages are fired by hand).
    let forest = d.t::<FeatureType>("FEATURE_FOREST");
    let away = margin(&g, &swamp);
    if let Some(wood) = block(&mut g, 4, 4, &away) {
        for &q in &wood {
            g.plot_set_feature_type(q, Some(forest));
        }
        nc::fire_brood(&mut g, 0);
        let brood = world::next_player(&g);
        nc::fire_brood(&mut g, 1);
        if g.players[brood].alive {
            born.push((brood, "the Brood"));
        }
    }
    // The Infernals, beside the civilization with the most cities, made evil.
    let civs = world::civ_players(&g);
    let patron = civs.iter().copied().filter(|&p| !g.players[p].cities.is_empty()).find(|&p| !born.iter().any(|&(b, _)| b == p)).unwrap();
    for p in civs {
        g.players[p].alignment = Some(if p == patron { AlignmentType::ALIGNMENT_EVIL } else { AlignmentType::ALIGNMENT_NEUTRAL });
    }
    if nc::valid_infernal_gate(&g) {
        let infernals = world::next_player(&g);
        nc::fire_infernal_gate(&mut g, 0);
        assert!(g.players[infernals].alive);
        // The Infernals start with settlers, not cities: they are checked for living only.
    }
    play_and_check(&mut g, &born);
}

#[test]
fn the_great_person_civilizations_play_under_the_ai() {
    let mut g = new_game(6);
    let d = g.d;
    let p0 = world::civ_players(&g)[0];
    let team = g.players[p0].team;
    // The land each card needs, painted by hand: three ruins on free sites (the Necropolis) and
    // hills (the Hill Clans); the Aifon take the world's coast.
    let land: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| !g.plot_is_water(q)).collect();
    let ruins = d.t::<ImprovementType>("IMPROVEMENT_CITY_RUINS");
    for q in world::best_sites(&g, &land, 3, None, &[]) {
        g.plot_set_improvement_type(q, Some(ruins));
    }
    let taken: Vec<PlotIdx> = g.cities.ids().into_iter().map(|c| g.city_plot(c)).collect();
    let away = margin(&g, &taken);
    let hills = block(&mut g, 5, 5, &away).expect("free land for hills");
    for &q in &hills {
        g.plot_set_plot_type(q, PlotType::PLOT_HILLS, true);
    }
    // Player 0's team knows the three cards: its AI casts their world spells at once.
    let cards = [
        ("TECH_CARD_RITE_OF_THE_NECROPOLIS", "CIVILIZATION_NECROPOLIS"),
        ("TECH_CARD_WAKE_THE_HILL_CLANS", "CIVILIZATION_HILL_CLANS"),
        ("TECH_CARD_CALL_THE_AIFON", "CIVILIZATION_AIFON"),
    ];
    for (card, _) in cards {
        g.team_set_has_tech(team, d.t::<TechType>(card), true, Some(p0), false, false);
    }
    let start = g.game_turn();
    let mut born: Vec<(PlayerId, &str)> = Vec::new();
    for _ in 0..300 {
        if g.game_turn() >= start + 2 || born.len() == cards.len() {
            break;
        }
        play_turn(&mut g);
        born = cards
            .iter()
            .filter_map(|&(_, name)| {
                let civ = d.t::<CivilizationType>(name);
                g.player_ids().find(|&p| g.players[p].alive && g.player_get_civilization_type(p) == Some(civ)).map(|p| (p, name))
            })
            .collect();
    }
    assert_eq!(born.len(), cards.len(), "the AI casts the three births at once: {born:?}");
    play_and_check(&mut g, &born);
}
