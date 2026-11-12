//! Frozen Passage, Sea Ice and Icebreak, in a world built by hand (`illians.rs`).

use super::*;
use crate::map::{Map, MapInitData};
use crate::setup::GameSetup;

const US: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Every name the data uses resolves.
#[test]
fn registered_names_resolve() {
    for name in ["spellFrozenPassage", "spellIcebreak"] {
        assert!(find_spell(name).is_some(), "{name}");
        assert!(crate::ffh::world_ritual_spells::find_spell(name).is_some(), "{name}");
    }
    for name in ["targetsFrozenPassage", "targetsIcebreak"] {
        assert!(find_targets(name).is_some(), "{name}");
    }
    let d = ffh_data::global();
    assert!(d.ty::<SpellType>("SPELL_STASIS").is_none(), "Stasis is gone");
    assert!(d.ty::<ProjectType>("PROJECT_RITUAL_STASIS").is_none());
    let ritual = d.t::<ProjectType>("PROJECT_RITUAL_FROZEN_PASSAGE");
    assert_eq!(d.projects[ritual].production_cost, 150);
    let spell = d.t::<SpellType>("SPELL_FROZEN_PASSAGE");
    assert!(d.spells[spell].global && d.spells[spell].ritual_charge == Some(ritual));
    assert_eq!(d.spells[d.t::<SpellType>("SPELL_ICEBREAK")].delay, 2);
}

/// A 24x12 map: two grassland continents, x 2..=8 and x 15..=21 at y 3..=8, coast around them;
/// player 0 the Illians, player 1 the Bannor, met; every plot revealed to every team; no
/// barbarians, no units or cities.
fn game() -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 7);
    for (slot, name) in ["CIVILIZATION_ILLIANS", "CIVILIZATION_BANNOR"].iter().enumerate() {
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
    g.map = Map::new(d, &MapInitData { grid_w: 24, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for p in g.map.plot_ids() {
        let (x, y) = g.map.xy(p);
        let land = (3..=8).contains(&y) && ((2..=8).contains(&x) || (15..=21).contains(&x));
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
    g
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

/// Freezes the strait's row y = 5, x 9..=14: a bridge of Sea Ice between the two continents.
fn bridge(g: &mut Game) -> Vec<PlotIdx> {
    let row: Vec<PlotIdx> = (9..=14).map(|x| plot(g, x, 5)).collect();
    g.plot_freeze_sea_ice(&row);
    row
}

/// The areas agree with the plots: each plot's area exists and has its kind (water or land), the
/// tiles counted are the plots, and two plots next to each other (diagonals for land, sides for
/// water) of the same kind share one.
fn assert_areas_consistent(g: &Game) {
    let mut tiles = std::collections::BTreeMap::new();
    for p in g.map.plot_ids() {
        let a = g.map.plots[p].area.expect("every plot has an area");
        assert!(g.map.areas.contains(a));
        assert_eq!(g.map.areas[a].water, g.plot_is_water(p), "{:?}", g.map.xy(p));
        *tiles.entry(a).or_insert(0) += 1;
        let (x, y) = g.map.xy(p);
        for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
            let Some(q) = g.map.plot_xy(x + dx, y + dy) else { continue };
            let diagonal = dx != 0 && dy != 0;
            if g.plot_is_water(p) == g.plot_is_water(q) && (!diagonal || !g.plot_is_water(p)) {
                assert_eq!(g.map.plots[q].area, Some(a), "{:?} and {:?}", (x, y), g.map.xy(q));
            }
        }
    }
    for (a, n) in tiles {
        assert_eq!(g.map.areas[a].num_tiles, n);
    }
}

#[test]
fn frozen_passage_freezes_the_water_within_two_and_spends_the_charge() {
    let mut g = game();
    let d = g.d;
    let fp = spell(&g, "SPELL_FROZEN_PASSAGE");
    let city = g.player_init_city(US, 7, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 7, 5);
    assert!(!g.unit_can_cast(caster, fp, false), "no charge");
    assert!(g.player_grant_world_ritual_pending(US).is_some());
    let _ = city;
    assert!(g.unit_can_cast(caster, fp, false));
    let target = plot(&g, 11, 5);
    let before: Vec<bool> = g.map.plot_ids().map(|p| g.plot_is_water(p)).collect();
    let expected: Vec<PlotIdx> = g
        .map
        .plot_ids()
        .filter(|&p| {
            let (x, y) = g.map.xy(p);
            g.plot_is_water(p) && g.plot_distance(x, y, 11, 5) <= 2
        })
        .collect();
    assert_eq!(expected.len(), 21, "the whole radius is water");
    g.unit_cast_at(caster, fp, SpellTarget::Plot(target));
    let snow = d.t::<TerrainType>("TERRAIN_SNOW");
    for p in g.map.plot_ids() {
        let frozen = expected.contains(&p);
        assert_eq!(g.map.plots[p].sea_ice, frozen, "{:?}", g.map.xy(p));
        if frozen {
            assert_eq!(g.map.plots[p].plot_type, PlotType::PLOT_LAND);
            assert_eq!(g.map.plots[p].terrain_type, Some(snow));
        } else {
            assert_eq!(g.plot_is_water(p), before[p.idx()], "{:?}: nothing else changed", g.map.xy(p));
        }
    }
    assert!(!g.player_world_ritual_pending(US, g.player_world_ritual(US).unwrap()), "the charge is spent");
    assert!(!g.unit_can_cast(caster, fp, false));
    assert_areas_consistent(&g);
}

#[test]
fn frozen_passage_targets_seen_water_near_the_borders_or_the_ice() {
    let mut g = game();
    let fp = spell(&g, "SPELL_FROZEN_PASSAGE");
    g.player_init_city(US, 7, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 7, 5);
    g.player_grant_world_ritual_pending(US);
    let targets = |g: &Game| g.unit_spell_targets(caster, fp);
    // The borders at founding: x 6..=8, y 4..=6.
    assert!(targets(&g).contains(&SpellTarget::Plot(plot(&g, 13, 5))), "5 plots from the borders");
    assert!(!targets(&g).contains(&SpellTarget::Plot(plot(&g, 14, 5))), "6 plots");
    assert!(!targets(&g).contains(&SpellTarget::Plot(plot(&g, 7, 5))), "land");
    // Unrevealed water is refused.
    let team = g.players[US].team;
    let hidden = plot(&g, 12, 5);
    g.plot_set_revealed(hidden, team, false, false, None, false);
    assert!(!targets(&g).contains(&SpellTarget::Plot(hidden)), "not seen");
    assert!(!frozen_passage_ok(&g, caster, hidden));
    // Sea Ice anchors it too: (14, 0) is 6 from the borders, 4 from the ice at (12, 3).
    let far = plot(&g, 14, 0);
    assert!(!targets(&g).contains(&SpellTarget::Plot(far)));
    let ice = vec![plot(&g, 12, 3)];
    g.plot_freeze_sea_ice(&ice);
    assert!(targets(&g).contains(&SpellTarget::Plot(far)), "near Sea Ice");
    assert!(!targets(&g).contains(&SpellTarget::Plot(plot(&g, 20, 0))), "far from both");
    // Every target is water, seen, and in range.
    for t in targets(&g) {
        let SpellTarget::Plot(t) = t else { panic!("a plot") };
        assert!(frozen_passage_ok(&g, caster, t));
    }
}

#[test]
fn a_land_unit_walks_across_and_a_ship_cannot_enter() {
    let mut g = game();
    let row = bridge(&mut g);
    assert_areas_consistent(&g);
    assert_eq!(g.map.plots[plot(&g, 2, 5)].area, g.map.plots[plot(&g, 21, 5)].area, "one landmass now");
    let warrior = unit(&mut g, US, "UNIT_WARRIOR", 8, 5);
    let den = g.d.defines.move_denominator;
    assert_eq!(g.plot_movement_cost(row[0], warrior, plot(&g, 8, 5)), den, "a cost of 1");
    assert!(g.unit_can_move_into(warrior, row[0], false, false, false));
    let grp = g.units[warrior].group.unwrap();
    g.players[US].turn_active = true;
    g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 15, 5, 0, false, false, None, None, None);
    for _ in 0..10 {
        if (g.units[warrior].x, g.units[warrior].y) == (15, 5) {
            break;
        }
        g.units[warrior].moves = 0;
        g.group_auto_mission(grp);
    }
    assert_eq!((g.units[warrior].x, g.units[warrior].y), (15, 5), "across the strait on foot");
    let ship = unit(&mut g, THEM, "UNIT_TRIREME", 11, 4);
    for &q in &row {
        assert!(!g.unit_can_move_into(ship, q, false, false, false), "{:?}", g.map.xy(q));
    }
    assert!(g.unit_can_move_into(ship, plot(&g, 12, 4), false, false, false), "water still");
}

#[test]
fn sea_ice_yields_nothing_and_takes_no_city_or_improvement() {
    let mut g = game();
    let d = g.d;
    let row = bridge(&mut g);
    let ice = row[2];
    let (x, y) = g.map.xy(ice);
    for yt in [YieldType::YIELD_FOOD, YieldType::YIELD_PRODUCTION, YieldType::YIELD_COMMERCE] {
        assert_eq!(g.plot_calculate_yield(ice, yt), 0);
        assert_eq!(g.map.plots[ice].yields[yt.idx()], 0);
    }
    assert!(!g.player_can_found(US, x, y, false), "no city");
    assert!(d.builds.keys().all(|b| !g.plot_can_build(ice, b, US, false)), "no improvement or route");
    assert!(d.improvements.keys().all(|i| !g.plot_can_have_improvement(ice, i, None, false)));
    g.plot_set_improvement_type(ice, Some(d.t::<ImprovementType>("IMPROVEMENT_FARM")));
    g.plot_set_route_type(ice, Some(d.t::<RouteType>("ROUTE_ROAD")), true);
    g.plot_set_feature_type(ice, Some(d.t::<FeatureType>("FEATURE_FOREST")));
    g.plot_set_terrain_type(ice, d.t::<TerrainType>("TERRAIN_GRASS"), true);
    let pl = &g.map.plots[ice];
    assert_eq!((pl.improvement_type, pl.route_type, pl.feature_type), (None, None, None));
    assert_eq!(pl.terrain_type, Some(d.t::<TerrainType>("TERRAIN_SNOW")));
    // A resource under the water stays, unused.
    let fish = d.t::<BonusType>("BONUS_FISH");
    let q = plot(&g, 11, 8);
    g.plot_set_bonus_type(q, Some(fish));
    g.plot_freeze_sea_ice(&[q]);
    assert_eq!(g.map.plots[q].bonus_type, Some(fish));
    assert_eq!(g.map.plots[q].yields, [0; YieldType::NUM]);
}

#[test]
fn a_ship_the_water_froze_under_stays_and_leaves_to_the_water() {
    let mut g = game();
    let ship = unit(&mut g, US, "UNIT_TRIREME", 11, 5);
    let cargo = unit(&mut g, US, "UNIT_WARRIOR", 11, 5);
    g.unit_set_transport_unit(cargo, Some(ship));
    let row = bridge(&mut g);
    let at = plot(&g, 11, 5);
    assert!(g.units.contains(ship) && g.unit_plot(ship) == at, "the ship stays");
    assert!(g.plot_is_valid_domain_for_location(at, ship));
    g.plot_verify_unit_valid_plot(at);
    assert_eq!(g.unit_plot(ship), at, "not bumped");
    assert!(g.unit_can_unload(cargo), "its cargo may unload onto the ice");
    g.unit_unload(cargo);
    assert!(!g.unit_is_cargo(cargo));
    let grp = g.units[ship].group.unwrap();
    g.players[US].turn_active = true;
    g.units[ship].moves = 0;
    assert!(g.unit_can_move_into(ship, plot(&g, 11, 4), false, false, false));
    g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 11, 3, 0, false, false, None, None, None);
    assert_eq!((g.units[ship].x, g.units[ship].y), (11, 3), "out to the water");
    assert!(!g.unit_can_move_into(ship, row[2], false, false, false), "and not back");
}

#[test]
fn icebreak_takes_two_turns_and_thaws_to_coast_or_ocean() {
    let mut g = game();
    let d = g.d;
    let ib = spell(&g, "SPELL_ICEBREAK");
    let row = bridge(&mut g);
    // Any civilization's ship: a Bannor galley.
    let boat = unit(&mut g, THEM, "UNIT_GALLEY", 11, 4);
    let soldier = unit(&mut g, US, "UNIT_WARRIOR", 12, 5);
    let targets = g.unit_spell_targets(boat, ib);
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 10, 5))) && targets.contains(&SpellTarget::Plot(plot(&g, 11, 5))));
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 12, 5))), "a unit stands on it");
    assert!(!targets.contains(&SpellTarget::Plot(plot(&g, 13, 5))), "not adjacent");
    let warrior_targets = g.unit_spell_targets(soldier, ib);
    assert!(warrior_targets.is_empty(), "ships only");
    g.unit_cast_at(boat, ib, SpellTarget::Plot(plot(&g, 11, 5)));
    assert_eq!(g.unit_get_immobile_timer(boat), 2, "held two turns");
    assert!(g.map.plots[plot(&g, 11, 5)].sea_ice);
    g.unit_change_immobile_timer(boat, -1);
    assert!(g.map.plots[plot(&g, 11, 5)].sea_ice, "after one turn: still ice");
    g.unit_change_immobile_timer(boat, -1);
    let mid = plot(&g, 11, 5);
    assert!(g.plot_is_water(mid) && !g.map.plots[mid].sea_ice, "after two: water");
    assert_eq!(g.map.plots[mid].terrain_type, Some(d.t::<TerrainType>("TERRAIN_OCEAN")), "no land but ice next to it");
    assert_eq!(g.units[boat].delayed_spell, None);
    assert_areas_consistent(&g);
    assert_ne!(g.map.plots[plot(&g, 2, 5)].area, g.map.plots[plot(&g, 21, 5)].area, "the bridge is cut");
    // Next to the land: Coast.
    let boat2 = unit(&mut g, THEM, "UNIT_GALLEY", 9, 4);
    g.unit_cast_at(boat2, ib, SpellTarget::Plot(row[0]));
    g.unit_change_immobile_timer(boat2, -2);
    assert_eq!(g.map.plots[row[0]].terrain_type, Some(d.t::<TerrainType>("TERRAIN_COAST")));
    assert_areas_consistent(&g);
    // A unit that steps on the plot meanwhile keeps it frozen.
    let boat3 = unit(&mut g, THEM, "UNIT_GALLEY", 10, 6);
    let target = plot(&g, 10, 5);
    g.unit_cast_at(boat3, ib, SpellTarget::Plot(target));
    let w2 = unit(&mut g, US, "UNIT_WARRIOR", 10, 5);
    g.unit_change_immobile_timer(boat3, -2);
    assert!(g.map.plots[target].sea_ice, "occupied at the end: no thaw");
    assert!(g.units.contains(w2));
}

#[test]
fn a_ship_alone_on_sea_ice_may_break_its_own_plot() {
    let mut g = game();
    let ib = spell(&g, "SPELL_ICEBREAK");
    let ship = unit(&mut g, US, "UNIT_TRIREME", 11, 5);
    bridge(&mut g);
    let at = plot(&g, 11, 5);
    assert!(g.unit_spell_targets(ship, ib).contains(&SpellTarget::Plot(at)));
    let other = unit(&mut g, US, "UNIT_WARRIOR", 11, 5);
    assert!(!g.unit_spell_targets(ship, ib).contains(&SpellTarget::Plot(at)), "not with another unit there");
    g.unit_kill(other, false, None);
    g.unit_cast_at(ship, ib, SpellTarget::Plot(at));
    g.unit_change_immobile_timer(ship, -2);
    assert!(g.plot_is_water(at) && g.units.contains(ship) && g.unit_plot(ship) == at);
    assert_areas_consistent(&g);
}

#[test]
fn a_frozen_plot_and_a_pending_icebreak_survive_save_and_load() {
    let mut g = game();
    let ib = spell(&g, "SPELL_ICEBREAK");
    bridge(&mut g);
    let boat = unit(&mut g, THEM, "UNIT_GALLEY", 11, 4);
    let at = plot(&g, 11, 5);
    g.unit_cast_at(boat, ib, SpellTarget::Plot(at));
    let mut h = Game::load(g.d, &g.save()).expect("load");
    assert!(h.map.plots[at].sea_ice && !h.plot_is_water(at));
    assert_eq!(h.units[boat].delayed_spell_target, Some(SpellTarget::Plot(at)));
    h.unit_change_immobile_timer(boat, -2);
    assert!(h.plot_is_water(at), "the loaded game finishes the icebreak");
}
