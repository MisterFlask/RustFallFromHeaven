//! World rituals (`world_rituals.rs`), each in a world built by hand.

use super::*;
use crate::map::{Map, MapInitData};
use crate::setup::GameSetup;
use crate::structs::OrderKind;

const US: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// A 20x12 map, grassland at x 2..=17, y 3..=8 and ocean around it, for the players of `civs`
/// with no units or cities; the log on; no barbarians.
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
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    let ocean = d.t::<TerrainType>("TERRAIN_COAST");
    for p in g.map.plot_ids() {
        let (x, y) = g.map.xy(p);
        let land = (3..=8).contains(&y) && (2..=17).contains(&x);
        g.plot_set_plot_type(p, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(p, if land { grass } else { ocean }, false);
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    g.log.enabled = true;
    g
}

/// The Hippus (Warcry: no tech needed) and the Bannor, each with two cities.
fn game() -> (Game, [CityId; 2], [CityId; 2]) {
    let mut g = game_of(&["CIVILIZATION_HIPPUS", "CIVILIZATION_BANNOR", "CIVILIZATION_LJOSALFAR"]);
    let ours = [g.player_init_city(US, 4, 5, true, true), g.player_init_city(US, 8, 5, true, true)];
    let theirs = [g.player_init_city(THEM, 13, 5, true, true), g.player_init_city(THEM, 16, 7, true, true)];
    g.player_init_city(PlayerId(2), 14, 3, true, true);
    for c in ours.iter().chain(theirs.iter()) {
        g.city_clear_order_queue(*c);
    }
    (g, ours, theirs)
}

fn warcry(g: &Game) -> (ProjectType, SpellType) {
    (g.d.t::<ProjectType>("PROJECT_RITUAL_WARCRY"), g.d.t::<SpellType>("SPELL_WARCRY"))
}

fn queue(g: &mut Game, c: CityId, pr: ProjectType) {
    g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, false);
}

/// The log entries containing `text`, by player.
fn told(g: &Game, text: &str) -> Vec<Option<u8>> {
    g.log.entries.iter().filter(|e| e.text.contains(text)).map(|e| e.player).collect()
}

/// Completes the ritual at the head of `c`'s queue as a turn's production would.
fn complete(g: &mut Game, c: CityId) {
    let needed = g.city_production_needed(c);
    let have = g.city_production(c);
    g.city_add_turn_production(c, needed - have);
    g.city_pop_order(c, 0, true, false);
}

#[test]
fn every_world_spell_has_a_world_ritual_of_its_civilization() {
    let d = ffh_data::global();
    let mut n = 0;
    for s in d.spells.keys().filter(|&s| d.spells[s].global) {
        let pr = d.spells[s].ritual_charge.expect("a world spell spends a ritual's charge");
        assert!(d.projects[pr].world_ritual && d.projects[pr].max_global_instances == -1 && d.projects[pr].max_team_instances == -1);
        assert!(d.projects[pr].prereq_civilization.is_some(), "{}", d.projects[pr].base.type_);
        assert_eq!(d.projects[pr].repeat_cost_percent, 200);
        n += 1;
    }
    assert_eq!(n, 21);
}

#[test]
fn grow_cost_doubles_without_a_cap_on_the_count() {
    assert_eq!(grow_cost(100, 200, 0), 100);
    assert_eq!(grow_cost(100, 200, 3), 800);
    assert_eq!(grow_cost(100, 200, 40), WORLD_RITUAL_MAX_COST);
    assert_eq!(grow_cost(100, 200, -2), 100);
}

#[test]
fn the_cost_doubles_per_completion_and_a_refund_undoes_one() {
    let (mut g, _, _) = game();
    let (pr, _) = warcry(&g);
    assert_eq!(g.player_world_ritual(US), Some(pr));
    assert!(g.project_is_world_ritual(pr));
    let base = g.player_get_production_needed_project(US, pr);
    let once = g.player_project_production_needed_at(US, pr, 1);
    assert!(once >= 2 * base - 1 && once <= 2 * base + 1, "{base} -> {once}");
    g.players[US].world_ritual_completions = 1;
    assert_eq!(g.player_get_production_needed_project(US, pr), once);
    g.players[US].world_ritual_completions = 30;
    let big = g.player_get_production_needed_project(US, pr);
    assert!(big > 0, "no overflow: {big}");
    g.players[US].world_ritual_completions = 1;
    g.players[US].world_ritual_refunds = 1;
    assert_eq!(g.player_get_production_needed_project(US, pr), base);
    assert_eq!(g.player_world_ritual_net_completions(US), 0);
    // Birthright Regained (which undoes a doubling) needs a net completion.
    let birthright = g.d.t::<ProjectType>("PROJECT_BIRTHRIGHT_REGAINED");
    let c = g.players[US].cities[0];
    assert!(cannot_create(&g, c, birthright, false));
    g.players[US].world_ritual_completions = 2;
    assert!(!cannot_create(&g, c, birthright, false));
    g.ev_project_built(c, birthright);
    assert_eq!(g.players[US].world_ritual_refunds, 2);
}

#[test]
fn a_second_city_cannot_queue_it() {
    let (mut g, [a, b], _) = game();
    let (pr, _) = warcry(&g);
    assert!(g.city_can_create(a, pr, false, false) && g.city_can_create(b, pr, false, false));
    queue(&mut g, a, pr);
    assert!(!g.city_can_create(b, pr, false, false));
    assert!(g.city_can_create(b, pr, false, true), "shown");
    assert!(g.city_can_create(a, pr, true, false), "a continues");
    let theirs = g.players[THEM].cities[0];
    assert!(!g.city_can_create(theirs, pr, false, false), "another civilization's");
}

#[test]
fn putting_it_behind_or_taking_it_off_loses_its_production() {
    let (mut g, [a, _], _) = game();
    let (pr, _) = warcry(&g);
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    queue(&mut g, a, pr);
    g.city_add_turn_production(a, 6);
    assert_eq!(g.cities[a].project_production[pr], 6);
    assert_eq!(g.city_world_ritual_in_progress(a), Some(pr));
    // Appended behind it: kept.
    g.city_push_order(a, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, true, false);
    assert_eq!(g.cities[a].project_production[pr], 6);
    // An army's requisition does not go in front of it.
    assert_eq!(g.city_push_order_kind(a, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, false, OrderKind::Requisition), None);
    // Put behind a new order: abandoned.
    g.city_push_order(a, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, false);
    assert_eq!(g.cities[a].project_production[pr], 0);
    assert_eq!(told(&g, "abandoned Ritual: Warcry in").len(), 3, "every player told");
    // Back at the front with production, then taken off: abandoned.
    g.city_clear_order_queue(a);
    queue(&mut g, a, pr);
    g.city_add_turn_production(a, 4);
    g.city_pop_order(a, 0, false, false);
    assert_eq!(g.cities[a].project_production[pr], 0);
    assert_eq!(told(&g, "abandoned Ritual: Warcry in").len(), 6);
}

#[test]
fn it_cannot_be_hurried_or_given_production_at_once() {
    let (mut g, [a, _], _) = game();
    let (pr, _) = warcry(&g);
    let gold = g.d.t::<HurryType>("HURRY_GOLD");
    g.players[US].hurry_count[gold] += 1;
    g.players[US].gold = 10_000;
    let granary = g.d.t::<BuildingType>("BUILDING_GRANARY");
    g.city_push_order(a, OrderType::ORDER_CONSTRUCT, granary.i(), -1, false, false, false, true);
    assert!(g.city_can_hurry(a, gold, false), "a building is hurried");
    g.city_clear_order_queue(a);
    queue(&mut g, a, pr);
    assert!(!g.city_can_hurry(a, gold, false) && !g.city_can_hurry(a, gold, true));
    g.city_hurry(a, gold);
    assert_eq!(g.cities[a].project_production[pr], 0);
    g.city_change_production(a, 50);
    assert_eq!(g.cities[a].project_production[pr], 0, "no production in one step");
    g.city_add_turn_production(a, 3);
    assert_eq!(g.cities[a].project_production[pr], 3, "the turn's production");
}

#[test]
fn the_start_is_told_to_every_player_once() {
    let (mut g, [a, _], _) = game();
    let (pr, _) = warcry(&g);
    queue(&mut g, a, pr);
    assert!(told(&g, "began").is_empty(), "queued: nothing put into it yet");
    g.city_add_turn_production(a, 2);
    g.city_add_turn_production(a, 2);
    let mut to = told(&g, "Hippus began Ritual: Warcry in ");
    to.sort();
    assert_eq!(to, vec![Some(0), Some(1), Some(2)]);
    let at = g.log.entries.iter().find(|e| e.text.contains(" began ")).and_then(|e| e.at);
    assert_eq!(at, Some((4, 5)), "at the city");
    complete(&mut g, a);
    assert_eq!(told(&g, "Hippus completed Ritual: Warcry in ").len(), 3);
}

#[test]
fn the_world_spell_spends_a_charge_held_anywhere() {
    let (mut g, [a, b], _) = game();
    let (pr, spell) = warcry(&g);
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let caster = g.player_init_unit(US, warrior, 6, 6, None, None);
    assert!(!g.unit_can_cast(caster, spell, false) && !g.unit_can_cast(caster, spell, true), "no charge");
    queue(&mut g, b, pr);
    complete(&mut g, b);
    assert_eq!(g.cities[b].ritual_charges, vec![pr]);
    assert_eq!(g.players[US].world_ritual_completions, 1);
    // One charge at most: not again while held, in any city.
    assert!(!g.city_can_create(a, pr, false, false) && !g.city_can_create(b, pr, false, false));
    // Cast from outside any city.
    assert!(g.unit_can_cast(caster, spell, false));
    g.unit_cast(caster, spell);
    assert!(g.cities[b].ritual_charges.is_empty(), "spent");
    assert_eq!(g.players[US].world_spell_casts, 1);
    assert!(g.city_can_create(a, pr, false, false), "built again");
    // The next one costs twice as much.
    let r = g.world_rituals().into_iter().find(|r| r.player == US).expect("ours");
    assert_eq!((r.project, r.spell, r.charged, r.completed, r.city), (pr, Some(spell), false, 1, None));
    assert_eq!(r.cost, g.player_project_production_needed_at(US, pr, 0));
    assert_eq!(r.next_cost, g.player_project_production_needed_at(US, pr, 1));
    // A charge granted for a test.
    assert_eq!(g.player_grant_world_ritual_charge(US), Some(a));
    let r = g.world_rituals().into_iter().find(|r| r.player == US).expect("ours");
    assert!(r.charged && r.city == Some(a));
}

#[test]
fn the_query_shows_a_ritual_in_progress() {
    let (mut g, [a, _], _) = game();
    let (pr, _) = warcry(&g);
    queue(&mut g, a, pr);
    g.city_add_turn_production(a, 5);
    let all = g.world_rituals();
    assert_eq!(all.len(), 3, "each living civilization's");
    let r = all.iter().find(|r| r.player == US).expect("ours");
    assert_eq!((r.city, r.progress, r.charged, r.completed), (Some(a), 5, false, 0));
    assert!(r.turns_left.is_some_and(|t| t >= 1));
    assert_eq!(all.iter().find(|r| r.player == THEM).map(|r| r.project), g.d.ty::<ProjectType>("PROJECT_RITUAL_RALLY"));
}

#[test]
fn the_sanctuary_ritual_waits_while_the_sanctuary_holds() {
    let mut g = game_of(&["CIVILIZATION_ELOHIM", "CIVILIZATION_BANNOR"]);
    let c = g.player_init_city(US, 5, 5, true, true);
    g.city_clear_order_queue(c);
    let pr = g.d.t::<ProjectType>("PROJECT_RITUAL_SANCTUARY");
    assert_eq!(g.player_world_ritual(US), Some(pr));
    assert!(g.city_can_create(c, pr, false, false));
    g.players[US].sanctuary_timer = 5;
    assert!(!g.city_can_create(c, pr, false, false));
    g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, true);
    g.city_add_turn_production(c, 5);
    assert_eq!(g.cities[c].project_production[pr], 0, "no production while it holds");
    g.players[US].sanctuary_timer = 0;
    assert!(g.city_can_create(c, pr, true, false));
}

#[test]
fn the_option_without_world_spells_forbids_the_rituals() {
    let (mut g, [a, _], _) = game();
    let (pr, spell) = warcry(&g);
    g.setup.options[GameOptionType::GAMEOPTION_NO_WORLD_SPELLS] = true;
    assert!(!g.city_can_create(a, pr, false, false) && !g.city_can_create(a, pr, false, true));
    g.player_grant_world_ritual_charge(US);
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let caster = g.player_init_unit(US, warrior, 6, 6, None, None);
    assert!(!g.unit_can_cast(caster, spell, false));
}
