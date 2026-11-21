//! The memory of wrongs (the owner's decisions of 2026-11-18; docs/PORTING.md, "Changes from
//! FFH2"): a city taken in battle is remembered by the player that lost it; a declared war is
//! remembered once by each player declared on, a vassal pulled in included. Every situation is
//! built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

const TOOK: MemoryType = MemoryType::MEMORY_TOOK_CITY;
const WAR: MemoryType = MemoryType::MEMORY_DECLARED_WAR;

/// Three civilizations on one block of grassland, all met, no barbarians: the Bannor (player 0)
/// with its capital at (3, 4) and city `own` at (3, 7); the Elohim (player 1), the Bannor's vassal
/// (by a deal) with cities `a` (9, 4) and `b` (9, 7); the Ljosalfar (player 2) with a city at
/// (15, 7).
struct World {
    g: Game,
    own: CityId,
    a: CityId,
    b: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 3, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    for (slot, (civ, leader)) in [("CIVILIZATION_BANNOR", "LEADER_SABATHIEL"), ("CIVILIZATION_ELOHIM", "LEADER_EINION"), ("CIVILIZATION_LJOSALFAR", "LEADER_ARENDEL")].into_iter().enumerate() {
        setup.players[slot].civ = Some(d.t::<CivilizationType>(civ));
        setup.players[slot].leader = Some(d.t::<LeaderHeadType>(leader));
    }
    let mut g = Game::new(d, setup);
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 20, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (3..=8).contains(&y) && (2..=17).contains(&x);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    let teams: Vec<TeamId> = [P0, P1, P2].iter().map(|&p| g.players[p].team).collect();
    for &t in &teams {
        for &o in &teams {
            if t != o && !g.teams[t].has_met[o] {
                g.team_meet(t, o, false);
            }
        }
    }
    g.player_init_city(P0, 3, 4, true, true);
    let own = g.player_init_city(P0, 3, 7, true, true);
    let a = g.player_init_city(P1, 9, 4, true, true);
    let b = g.player_init_city(P1, 9, 7, true, true);
    g.player_init_city(P2, 15, 7, true, true);
    g.team_assign_vassal(teams[0], teams[1], false);
    World { g, own, a, b }
}

fn memory(g: &Game, p: PlayerId, of: PlayerId, m: MemoryType) -> i32 {
    g.players[p].ai_memory_count[of][m]
}

#[test]
fn a_city_taken_in_battle_is_remembered_by_the_player_that_lost_it() {
    let World { mut g, own, a, b } = world();
    // The vassal's city: the vassal remembers it, its master does not.
    g.player_acquire_city(P2, a, true, false, true);
    assert_eq!(memory(&g, P1, P2, TOOK), 1, "the vassal");
    assert_eq!(memory(&g, P0, P2, TOOK), 0, "not its master");
    assert_eq!(memory(&g, P2, P1, TOOK) + memory(&g, P2, P0, TOOK), 0, "not the taker");
    // The master's own city.
    g.player_acquire_city(P2, own, true, false, true);
    assert_eq!(memory(&g, P0, P2, TOOK), 1);
    assert_eq!(memory(&g, P1, P2, TOOK), 1, "a vassal does not take up its master's wrongs");
    // A city traded is no conquest.
    g.player_acquire_city(P2, b, false, true, true);
    assert_eq!((memory(&g, P1, P2, TOOK), memory(&g, P0, P2, TOOK)), (1, 1));
}

#[test]
fn a_declared_war_is_remembered_once_by_each_player_declared_on() {
    let World { mut g, .. } = world();
    let (t0, t1, t2) = (g.players[P0].team, g.players[P1].team, g.players[P2].team);
    g.team_declare_war(t2, t0, false, None);
    assert!(g.teams[t2].at_war[t1], "the vassal is pulled in by the vassal rules");
    assert_eq!(memory(&g, P0, P2, WAR), 1, "the master, once: not again for its vassal");
    assert_eq!(memory(&g, P1, P2, WAR), 1, "the vassal");
    assert_eq!(memory(&g, P2, P0, WAR) + memory(&g, P2, P1, WAR), 0, "not the declarer");
}
