//! Fixed mana spheres: generation, connection, pillaging, discoveries and Dispel Magic.

use ffh_core::{Game, GameSetup, PlayerId};
use ffh_core::mana::{is_mana, mana_nodes};
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_data::*;

fn generated(seed: u32) -> Game {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", seed));
    g.map_script = std::sync::Arc::new(FractalMapScript::default());
    g.game_init();
    g
}

fn world() -> Game {
    let mut g = generated(7);
    for u in g.units.iter().map(|(u, _)| u).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    g.map = Map::new(g.d, &MapInitData { grid_w: 12, grid_h: 10, ..Default::default() }, g.num_players(), g.num_teams());
    for q in g.map.plot_ids() {
        g.plot_set_plot_type(q, PlotType::PLOT_LAND, false);
        g.plot_set_terrain_type(q, g.d.t::<TerrainType>("TERRAIN_GRASS"), false);
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    g.player_init_city(PlayerId(0), 5, 5, true, true);
    let team = g.players[PlayerId(0)].team;
    for t in g.d.techs.keys() {
        if !g.d.techs[t].disable {
            g.team_set_has_tech(team, t, true, Some(PlayerId(0)), false, false);
        }
    }
    g
}

#[test]
fn generated_mana_is_typed_before_any_node_is_built() {
    for seed in [1, 7, 21] {
        let g = generated(seed);
        let nodes = mana_nodes(g.d);
        let raw = g.d.t::<BonusType>("BONUS_MANA");
        assert_eq!(g.map.num_bonus[raw], 0);
        for q in g.map.plot_ids() {
            assert_ne!(g.map.plots[q].bonus_type, Some(raw));
            if let Some(b) = g.map.plots[q].bonus_type.filter(|&b| is_mana(g.d, b)) {
                assert!(nodes.iter().any(|&(mana, _)| mana == b) || b == g.d.t::<BonusType>("BONUS_MANA_ICE"));
                if let Some(i) = g.map.plots[q].improvement_type {
                    assert!(!nodes.iter().any(|&(_, node)| node == i), "ordinary resources start unimproved");
                }
            }
        }
    }
    let (a, b) = (generated(7), generated(7));
    assert!(a.map.plot_ids().all(|q| a.map.plots[q].bonus_type == b.map.plots[q].bonus_type));
}

#[test]
fn only_the_matching_node_can_be_built_and_rebuilt() {
    let mut g = world();
    let d = g.d;
    let p = PlayerId(0);
    let q = g.map.plot_xy(6, 5).unwrap();
    g.plot_set_route_type(q, Some(d.t::<RouteType>("ROUTE_ROAD")), true);
    let warrior = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), 6, 5, None, None);
    let nodes = mana_nodes(d);
    assert_eq!(nodes.len(), 17);
    for &(bonus, node) in &nodes {
        g.plot_set_bonus_type(q, Some(bonus));
        for &(other_bonus, other_node) in &nodes {
            assert_eq!(g.plot_can_have_improvement(q, other_node, Some(g.players[p].team), false), bonus == other_bonus);
        }
        assert!(d.improvements[node].bonus_convert.is_none());
        let before = g.player_get_num_available_bonuses(p, bonus);
        g.plot_set_improvement_type(q, Some(node));
        assert_eq!(g.map.plots[q].bonus_type, Some(bonus));
        assert_eq!(g.player_get_num_available_bonuses(p, bonus), before + 1);
        assert!(g.unit_pillage(warrior));
        assert_eq!(g.map.plots[q].bonus_type, Some(bonus));
        assert_eq!(g.player_get_num_available_bonuses(p, bonus), before);
        g.plot_set_improvement_type(q, Some(node));
        assert_eq!(g.map.plots[q].bonus_type, Some(bonus));
        assert_eq!(g.player_get_num_available_bonuses(p, bonus), before + 1);
        g.plot_set_improvement_type(q, None);
    }
}

#[test]
fn discoveries_are_random_typed_resources_and_do_not_reroll_existing_nodes() {
    let mut g = world();
    let raw = g.d.t::<BonusType>("BONUS_MANA");
    let plots: Vec<_> = g.map.plot_ids().filter(|&q| g.map.plots[q].plot_city.is_none()).collect();
    let mut types = std::collections::BTreeSet::new();
    for &q in &plots {
        g.plot_set_bonus_type(q, Some(raw));
        let b = g.map.plots[q].bonus_type.unwrap();
        assert!(is_mana(g.d, b));
        types.insert(b);
        g.plot_set_bonus_type(q, Some(raw));
        assert_eq!(g.map.plots[q].bonus_type, Some(b));
    }
    assert!(types.len() > 1, "new nodes draw different spheres");
    let saved = Game::load(g.d, &g.save()).unwrap();
    assert!(plots.iter().all(|&q| saved.map.plots[q].bonus_type == g.map.plots[q].bonus_type));
}

#[test]
fn dispel_removes_unit_promotions_without_resetting_mana() {
    let mut g = world();
    let d = g.d;
    let p = PlayerId(0);
    let q = g.map.plot_xy(6, 5).unwrap();
    let earth = d.t::<BonusType>("BONUS_MANA_EARTH");
    let node = d.t::<ImprovementType>("IMPROVEMENT_MANA_EARTH");
    g.plot_set_bonus_type(q, Some(earth));
    g.plot_set_improvement_type(q, Some(node));
    let mage = g.player_init_unit(p, d.t::<UnitType>("UNIT_MAGE"), 6, 5, None, None);
    g.unit_set_has_promotion(mage, d.t::<PromotionType>("PROMOTION_METAMAGIC2"), true);
    let spell = d.t::<SpellType>("SPELL_DISPEL_MAGIC");
    assert!(!g.unit_can_cast(mage, spell, false), "a mana node alone is not a Dispel target");
    let withered = d.t::<PromotionType>("PROMOTION_WITHERED");
    let warrior = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), 6, 5, None, None);
    g.unit_set_has_promotion(warrior, withered, true);
    assert!(g.unit_can_cast(mage, spell, false));
    g.unit_cast(mage, spell);
    assert!(!g.units[warrior].has_promotion[withered]);
    assert_eq!(g.map.plots[q].bonus_type, Some(earth));
    assert_eq!(g.map.plots[q].improvement_type, Some(node));
    assert!(d.ty::<SpellType>("SPELL_MANA_GRAFT").is_none());
    assert!(d.ty::<TechType>("TECH_CARD_MANA_GRAFTING").is_none());
}
