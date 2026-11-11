//! Raging Seas, Steer the Storm, The Delve and Enter the Delve, each in a world built by hand.

use super::*;
use crate::map::{Map, MapInitData};
use crate::setup::GameSetup;

const US: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Every name the data uses resolves.
#[test]
fn registered_names_resolve() {
    for name in ["spellRagingSeas", "spellSteerTheStorm", "spellTheDelve", "spellEnterTheDelve"] {
        assert!(find_spell(name).is_some(), "{name}");
    }
    for name in ["reqRagingSeas", "reqSteerTheStorm", "reqTheDelve", "reqEnterTheDelve"] {
        assert!(find_req(name).is_some(), "{name}");
    }
    for name in ["targetsRagingSeas", "targetsSteerTheStorm", "targetsTheDelve"] {
        assert!(find_targets(name).is_some(), "{name}");
    }
    let d = ffh_data::global();
    assert!(d.ty::<SpellType>("SPELL_MOTHER_LODE").is_none());
    assert!(d.ty::<ProjectType>("PROJECT_RITUAL_MOTHER_LODE").is_none());
    assert!(d.spells[d.t::<SpellType>("SPELL_STEER_THE_STORM")].world_action);
    assert!(d.spells[d.t::<SpellType>("SPELL_THE_DELVE")].cast_from_selection);
}

/// A 20x12 map, grassland at x 2..=17, y 3..=8 and coast around it, for the players of `civs`
/// with no units or cities, players 0 and 1 met; every plot revealed to every team; no
/// barbarians; the log on.
fn game_of(civs: &[&str]) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, civs.len(), "WORLDSIZE_DUEL", 7);
    for (slot, name) in civs.iter().enumerate() {
        let civ = d.t::<CivilizationType>(name);
        setup.players[slot].civ = Some(civ);
        setup.players[slot].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
    }
    let mut g = Game::new(d, setup);
    g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
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
    let (t0, t1) = (g.players[US].team, g.players[THEM].team);
    g.team_meet(t0, t1, false);
    g.log.enabled = true;
    g
}

fn learn(g: &mut Game, p: PlayerId, tech: &str) {
    let t = g.d.t::<TechType>(tech);
    let team = g.players[p].team;
    g.team_set_has_tech(team, t, true, Some(p), false, false);
}

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, None, None)
}

fn plot(g: &Game, x: i32, y: i32) -> PlotIdx {
    g.map.plot_xy(x, y).expect("on the map")
}

fn spell(g: &Game, name: &str) -> SpellType {
    g.d.t::<SpellType>(name)
}

fn war(g: &mut Game, a: PlayerId, b: PlayerId) {
    let (ta, tb) = (g.players[a].team, g.players[b].team);
    g.team_declare_war(ta, tb, false, None);
    assert!(g.at_war(ta, tb));
}

// ---------------------------------------------------------------------------------------------
// Raging Seas.

/// The Lanun (with Optics, a city at (4, 4) holding a charge) and the Bannor; a Lanun warrior in
/// the city.
fn seas_world() -> (Game, UnitId) {
    let mut g = game_of(&["CIVILIZATION_LANUN", "CIVILIZATION_BANNOR"]);
    learn(&mut g, US, "TECH_OPTICS");
    g.player_init_city(US, 4, 4, true, true);
    assert!(g.player_grant_world_ritual_charge(US).is_some());
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 4);
    (g, caster)
}

/// Casts Raging Seas at `(x, y)`.
fn raise_storm(g: &mut Game, caster: UnitId, x: i32, y: i32) {
    let seas = spell(g, "SPELL_RAGING_SEAS");
    let t = SpellTarget::Plot(plot(g, x, y));
    assert!(g.unit_can_cast_at(caster, seas, t));
    g.unit_cast_at(caster, seas, t);
}

#[test]
fn raging_seas_raises_a_storm_over_water_and_spends_the_charge() {
    let (mut g, caster) = seas_world();
    let seas = spell(&g, "SPELL_RAGING_SEAS");
    let project = g.d.t::<ProjectType>("PROJECT_RITUAL_RAGING_SEAS");
    let targets = g.unit_spell_targets(caster, seas);
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 10, 2))), "a water plot");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 10, 4))), "not land");
    // A water plot the team has not revealed is no target.
    let hidden = plot(&g, 19, 11);
    let team = g.players[US].team;
    g.map.plots[hidden].revealed[team] = false;
    assert!(!g.unit_spell_targets(caster, seas).contains(&SpellTarget::Plot(hidden)));

    let turn = g.game_turn();
    raise_storm(&mut g, caster, 10, 2);
    assert!(g.player_world_ritual_charge_city(US, project).is_none(), "the charge is spent");
    let i = g.player_storm(US).expect("a storm");
    let storm = g.area_effects[i];
    assert_eq!((storm.kind, storm.owner, storm.plot, storm.radius, storm.until, storm.moved), (AreaEffectKind::RagingSeas, US, plot(&g, 10, 2), STORM_RADIUS, turn + STORM_TURNS, -1));

    // No second storm while one holds, even with a new charge.
    g.player_grant_world_ritual_charge(US);
    assert!(!g.unit_can_cast_untargeted(caster, seas, false));
    assert!(g.unit_spell_targets(caster, seas).is_empty());
}

#[test]
fn the_storm_strikes_other_teams_at_sea_and_on_the_shore() {
    let (mut g, caster) = seas_world();
    raise_storm(&mut g, caster, 10, 2);
    let shore = unit(&mut g, THEM, "UNIT_WARRIOR", 10, 3);
    let inland = unit(&mut g, THEM, "UNIT_WARRIOR", 10, 4);
    let outside = unit(&mut g, THEM, "UNIT_WARRIOR", 14, 3);
    let ours = unit(&mut g, US, "UNIT_WARRIOR", 11, 3);
    let ship = unit(&mut g, THEM, "UNIT_TRIREME", 9, 1);
    let limit = g.unit_max_hit_points(shore) * STORM_LAND_LIMIT_PERCENT / 100;
    for _ in 0..40 {
        g.raging_seas_strike(US);
        assert!(g.units[shore].damage <= limit, "a land unit keeps a quarter of its health");
    }
    assert_eq!(g.units[shore].damage, limit, "struck to the limit");
    assert_eq!(g.units[inland].damage, 0, "inland, not next to water");
    assert_eq!(g.units[outside].damage, 0, "outside the storm");
    assert_eq!(g.units[ours].damage, 0, "the owner's team");
    assert!(!g.units.contains(ship) || g.unit_is_dead(ship), "a ship sinks");
}

#[test]
fn the_storm_strikes_at_the_start_of_its_owners_turn() {
    let (mut g, caster) = seas_world();
    raise_storm(&mut g, caster, 10, 2);
    let shore = unit(&mut g, THEM, "UNIT_WARRIOR", 10, 3);
    g.elapsed_game_turns = 1;
    for _ in 0..10 {
        g.players[US].turn_active = false;
        g.player_set_turn_active(US, true, true);
    }
    assert!(g.units[shore].damage > 0);
}

#[test]
fn the_storm_holds_fifteen_turns() {
    let (mut g, caster) = seas_world();
    let turn = g.game_turn();
    raise_storm(&mut g, caster, 10, 2);
    let shore = unit(&mut g, THEM, "UNIT_WARRIOR", 10, 3);
    g.set_game_turn(turn + STORM_TURNS - 1);
    assert!(g.player_storm(US).is_some(), "the cast's turn and the next 14");
    g.set_game_turn(turn + STORM_TURNS);
    assert!(g.player_storm(US).is_none());
    g.raging_seas_strike(US);
    assert_eq!(g.units[shore].damage, 0);
    g.area_effects_do_turn();
    assert!(g.area_effects.is_empty());
}

#[test]
fn steering_moves_the_storm_two_plots_over_water_once_a_turn() {
    let (mut g, caster) = seas_world();
    let steer = spell(&g, "SPELL_STEER_THE_STORM");
    // Any Lanun unit, wherever it stands: not without a storm.
    let pilot = unit(&mut g, US, "UNIT_WARRIOR", 8, 6);
    assert!(!g.unit_can_cast_untargeted(pilot, steer, false));
    raise_storm(&mut g, caster, 10, 1);
    let targets = g.unit_spell_targets(pilot, steer);
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 12, 1))));
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 10, 0))));
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 13, 1))), "3 plots away");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 10, 3))), "land");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 10, 1))), "where it is");
    // Another civilization's unit cannot.
    let other = unit(&mut g, THEM, "UNIT_WARRIOR", 14, 6);
    assert!(!g.unit_can_cast_untargeted(other, steer, false));

    let hidden = g.d.t::<PromotionType>("PROMOTION_HIDDEN");
    g.unit_set_has_promotion(pilot, hidden, true);
    let to = SpellTarget::Plot(plot(&g, 12, 1));
    assert!(g.unit_can_cast_at(pilot, steer, to));
    g.unit_cast_at(pilot, steer, to);
    let i = g.player_storm(US).unwrap();
    assert_eq!(g.area_effects[i].plot, plot(&g, 12, 1));
    assert_eq!(g.units[pilot].moves, 0, "the caster keeps its moves");
    assert!(!g.units[pilot].has_casted);
    assert!(g.units[pilot].has_promotion[hidden], "and stays hidden");
    // Once a turn.
    assert!(!g.unit_can_cast_untargeted(pilot, steer, false));
    assert!(!g.unit_can_cast_untargeted(caster, steer, false));
    g.set_game_turn(g.game_turn() + 1);
    assert!(g.unit_can_cast_at(pilot, steer, SpellTarget::Plot(plot(&g, 14, 1))));
}

// ---------------------------------------------------------------------------------------------
// The Delve.

/// The Khazad (with Mining, a city at (3, 4) holding a charge) and the Bannor. Hills at (5, 5),
/// (12, 5), (14, 5) and (16, 5), the Khazad's; a Bannor hill at (7, 5); a flat Khazad plot at
/// (8, 5). A Khazad warrior on (5, 5).
fn delve_world() -> (Game, UnitId) {
    let mut g = game_of(&["CIVILIZATION_KHAZAD", "CIVILIZATION_BANNOR"]);
    learn(&mut g, US, "TECH_MINING");
    g.player_init_city(US, 3, 4, true, true);
    assert!(g.player_grant_world_ritual_charge(US).is_some());
    for (x, y, owner, hill) in [(5, 5, US, true), (12, 5, US, true), (14, 5, US, true), (16, 5, US, true), (7, 5, THEM, true), (8, 5, US, false)] {
        let t = plot(&g, x, y);
        if hill {
            g.plot_set_plot_type(t, PlotType::PLOT_HILLS, false);
        }
        g.plot_set_owner(t, Some(owner), false, false);
    }
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 5, 5);
    (g, caster)
}

/// Digs the tunnel from `caster`'s plot to `(x, y)`.
fn delve(g: &mut Game, caster: UnitId, x: i32, y: i32) {
    let s = spell(g, "SPELL_THE_DELVE");
    let t = SpellTarget::Plot(plot(g, x, y));
    assert!(g.unit_can_cast_at(caster, s, t));
    g.unit_cast_at(caster, s, t);
}

#[test]
fn the_delve_joins_two_hills_of_the_caster_within_ten_plots() {
    let (mut g, caster) = delve_world();
    let s = spell(&g, "SPELL_THE_DELVE");
    let project = g.d.t::<ProjectType>("PROJECT_RITUAL_THE_DELVE");
    let targets = g.unit_spell_targets(caster, s);
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 14, 5))), "9 plots away");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 16, 5))), "11 plots away");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 7, 5))), "another's hill");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 8, 5))), "flat");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 5, 5))), "its own plot");
    // Not from a flat plot, nor from another's hill.
    let flat = unit(&mut g, US, "UNIT_WARRIOR", 8, 5);
    assert!(!g.unit_can_cast_untargeted(flat, s, false));
    let foreign = unit(&mut g, US, "UNIT_WARRIOR", 7, 5);
    assert!(!g.unit_can_cast_untargeted(foreign, s, false));

    delve(&mut g, caster, 14, 5);
    assert_eq!(g.delves, vec![(plot(&g, 5, 5), plot(&g, 14, 5))]);
    assert!(g.player_world_ritual_charge_city(US, project).is_none(), "the charge is spent");
    assert_eq!(g.plot_delve_end(plot(&g, 14, 5)), Some(plot(&g, 5, 5)));

    // Neither end is the end of another tunnel.
    g.player_grant_world_ritual_charge(US);
    assert!(!g.unit_can_cast_untargeted(caster, s, false), "from an end");
    let other = unit(&mut g, US, "UNIT_WARRIOR", 12, 5);
    assert!(g.unit_can_cast_untargeted(other, s, false));
    let targets = g.unit_spell_targets(other, s);
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 14, 5))) && !targets.contains(&SpellTarget::Plot(plot(&g, 5, 5))), "to an end");
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 16, 5))));

    // The tunnel outlives a change of owner.
    g.plot_set_owner(plot(&g, 14, 5), Some(THEM), false, false);
    assert_eq!(g.plot_delve_end(plot(&g, 5, 5)), Some(plot(&g, 14, 5)));
}

#[test]
fn the_tunnels_and_the_storm_are_saved() {
    let (mut g, caster) = delve_world();
    delve(&mut g, caster, 14, 5);
    g.area_effects.push(AreaEffect { kind: AreaEffectKind::RagingSeas, owner: US, plot: plot(&g, 10, 1), radius: STORM_RADIUS, until: 30, moved: 4 });
    let loaded = Game::load(g.d, &g.save()).expect("loads");
    assert_eq!(loaded.delves, g.delves);
    assert_eq!(loaded.area_effects, g.area_effects);
}

#[test]
fn entering_the_delve_takes_a_fresh_unit_to_the_other_end_and_ends_its_turn() {
    let (mut g, caster) = delve_world();
    let enter = spell(&g, "SPELL_ENTER_THE_DELVE");
    assert!(!g.unit_can_cast(caster, enter, false), "no tunnel yet");
    delve(&mut g, caster, 14, 5);
    let far = plot(&g, 14, 5);
    // A unit that has moved cannot.
    g.units[caster].moves = 1;
    assert!(!g.unit_can_cast(caster, enter, false));
    g.units[caster].moves = 0;
    assert!(g.unit_can_cast(caster, enter, false));
    g.unit_cast(caster, enter);
    assert_eq!(g.unit_plot(caster), far);
    assert!(!g.unit_can_move(caster), "its turn is over");
    // And back, next turn.
    g.units[caster].moves = 0;
    assert!(g.unit_can_cast(caster, enter, false));
}

#[test]
fn an_enemy_on_the_far_end_blocks_the_tunnel() {
    let (mut g, caster) = delve_world();
    let enter = spell(&g, "SPELL_ENTER_THE_DELVE");
    delve(&mut g, caster, 14, 5);
    let them = unit(&mut g, THEM, "UNIT_WARRIOR", 14, 5);
    g.units[caster].moves = 0;
    assert!(g.unit_can_cast(caster, enter, false), "at peace: no enemy");
    war(&mut g, US, THEM);
    assert!(!g.unit_can_cast(caster, enter, false), "at war: blocked");
    g.unit_kill(them, false, None);
    assert!(g.unit_can_cast(caster, enter, false));
}

#[test]
fn another_players_unit_travels_where_it_may_stand() {
    let (mut g, caster) = delve_world();
    let enter = spell(&g, "SPELL_ENTER_THE_DELVE");
    delve(&mut g, caster, 14, 5);
    let (t0, t1) = (g.players[US].team, g.players[THEM].team);
    g.team_set_open_borders(t0, t1, true);
    g.team_set_open_borders(t1, t0, true);
    let guest = unit(&mut g, THEM, "UNIT_WARRIOR", 5, 5);
    assert_eq!(g.unit_plot(guest), plot(&g, 5, 5));
    g.units[guest].moves = 0;
    assert!(g.unit_can_cast(guest, enter, false));
    // Without open borders it may not enter the Khazad's land at the far end.
    g.teams[t1].open_borders[t0] = false;
    g.teams[t0].open_borders[t1] = false;
    assert!(!g.unit_can_cast(guest, enter, false), "no open borders");
    g.teams[t1].open_borders[t0] = true;
    g.teams[t0].open_borders[t1] = true;
    g.unit_cast(guest, enter);
    assert_eq!(g.unit_plot(guest), plot(&g, 14, 5));
}
