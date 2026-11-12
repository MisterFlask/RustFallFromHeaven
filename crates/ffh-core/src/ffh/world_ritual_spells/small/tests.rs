//! The Tithe, The Cull, Upgrade and Wellspring (`small.rs`), each in a world built by hand.

use super::*;
use crate::map::{Map, MapInitData};
use crate::setup::GameSetup;

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

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, None, None)
}

fn spell(g: &Game, name: &str) -> SpellType {
    g.d.t::<SpellType>(name)
}

fn queue(g: &mut Game, c: CityId, pr: ProjectType) {
    g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, false);
}

/// Completes the order at the head of `c`'s queue as a turn's production would.
fn complete(g: &mut Game, c: CityId) {
    let needed = g.city_production_needed(c);
    let have = g.city_production(c);
    g.city_add_turn_production(c, needed - have);
    g.city_pop_order(c, 0, true, false);
}

fn told(g: &Game, text: &str) -> Vec<Option<u8>> {
    g.log.entries.iter().filter(|e| e.text.contains(text)).map(|e| e.player).collect()
}

fn in_queue(g: &Game, c: CityId, pr: ProjectType) -> bool {
    g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == pr.i())
}

// ---------------------------------------------------------------------------------------------
// The Tithe

/// The Calabim (one city, a Warrior, a vampire) and the Bannor (cities of 5, 2 and 3), met.
fn tithe_game() -> (Game, CityId, [CityId; 3], UnitId, UnitId) {
    let mut g = game_of(&["CIVILIZATION_CALABIM", "CIVILIZATION_BANNOR", "CIVILIZATION_LJOSALFAR"]);
    let ours = g.player_init_city(US, 4, 5, true, true);
    let theirs = [g.player_init_city(THEM, 12, 4, true, true), g.player_init_city(THEM, 16, 5, true, true), g.player_init_city(THEM, 13, 8, true, true)];
    for (c, pop) in theirs.iter().zip([5, 2, 3]) {
        g.city_set_population(*c, pop);
    }
    g.player_init_city(PlayerId(2), 8, 8, true, true);
    for c in g.cities.ids() {
        g.city_clear_order_queue(c);
    }
    let (t, o) = (g.players[US].team, g.players[THEM].team);
    g.team_meet(t, o, false);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    let vampire = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    let promo = g.d.t::<PromotionType>("PROMOTION_VAMPIRE");
    g.unit_set_has_promotion(vampire, promo, true);
    (g, ours, theirs, caster, vampire)
}

fn name_the_tithe(g: &mut Game, caster: UnitId, rival: PlayerId) {
    let s = spell(g, "SPELL_NAME_THE_TITHE");
    assert!(g.unit_can_cast_at(caster, s, SpellTarget::Player(rival)), "may name {rival:?}");
    g.unit_cast_at(caster, s, SpellTarget::Player(rival));
}

#[test]
fn naming_the_tithe_declares_war_and_opens_the_ritual() {
    let (mut g, ours, _, caster, _) = tithe_game();
    let pr = tithe_project(g.d).unwrap();
    assert_eq!(g.player_world_ritual(US), Some(pr));
    assert_eq!(g.world_ritual_spell(pr), None, "no charge: it acts at completion");
    assert!(!g.city_can_create(ours, pr, false, false), "no ritual without a named rival");
    assert!(g.city_can_create(ours, pr, false, true), "listed all the same");
    let s = spell(&g, "SPELL_NAME_THE_TITHE");
    let targets = g.unit_spell_targets(caster, s);
    assert_eq!(targets, vec![SpellTarget::Player(THEM)], "only the rival met");
    name_the_tithe(&mut g, caster, THEM);
    let (t, o) = (g.players[US].team, g.players[THEM].team);
    assert!(g.teams[t].at_war[o]);
    assert_eq!(g.players[US].tithe_target, Some(THEM));
    assert!(g.city_can_create(ours, pr, false, false));
    // While it has production, no new name.
    queue(&mut g, ours, pr);
    g.city_add_turn_production(ours, 5);
    assert!(g.city_world_ritual_in_progress(ours).is_some());
    assert!(!g.unit_can_cast_untargeted(caster, s, false));
}

#[test]
fn the_tithe_drains_the_rivals_large_cities_and_feeds_the_vampires() {
    let (mut g, ours, theirs, caster, vampire) = tithe_game();
    let pr = tithe_project(g.d).unwrap();
    name_the_tithe(&mut g, caster, THEM);
    let (xp_caster, xp_vampire) = (g.units[caster].experience, g.units[vampire].experience);
    queue(&mut g, ours, pr);
    complete(&mut g, ours);
    let pops: Vec<i32> = theirs.iter().map(|&c| g.cities[c].population).collect();
    assert_eq!(pops, vec![3, 2, 1], "2 from each city above 2");
    assert_eq!(g.units[vampire].experience, xp_vampire + 2, "D 4: 2 experience");
    assert_eq!(g.units[caster].experience, xp_caster, "not a vampire");
    assert_eq!(g.players[US].tithe_target, None, "the name is spent");
    assert_eq!(g.players[US].world_ritual_completions, 1);
    assert!(!told(&g, "The Tithe: ").is_empty());
    assert!(told(&g, "loses 4 population").contains(&Some(THEM.0)));
    assert!(!g.city_can_create(ours, pr, false, false), "a new name is needed");
}

#[test]
fn peace_or_the_rivals_death_cancels_the_tithe_without_a_doubling() {
    for death in [false, true] {
        let (mut g, ours, _, caster, _) = tithe_game();
        let pr = tithe_project(g.d).unwrap();
        name_the_tithe(&mut g, caster, THEM);
        let cost = g.player_get_production_needed_project(US, pr);
        queue(&mut g, ours, pr);
        g.city_add_turn_production(ours, 5);
        assert!(g.cities[ours].project_production[pr] > 0);
        let (t, o) = (g.players[US].team, g.players[THEM].team);
        if death {
            g.player_set_alive(THEM, false);
        } else {
            g.team_make_peace(t, o, false);
        }
        assert_eq!(g.players[US].tithe_target, None, "death {death}");
        assert!(!in_queue(&g, ours, pr), "the order is gone");
        assert_eq!(g.cities[ours].project_production[pr], 0, "its production lost");
        assert_eq!(g.players[US].world_ritual_completions, 0);
        assert_eq!(g.player_get_production_needed_project(US, pr), cost, "no doubling");
        assert!(!told(&g, "abandoned").is_empty());
    }
}

// ---------------------------------------------------------------------------------------------
// The Cull

fn cull_game() -> (Game, UnitId) {
    let mut g = game_of(&["CIVILIZATION_DOVIELLO", "CIVILIZATION_BANNOR"]);
    g.player_init_city(US, 4, 5, true, true);
    g.player_grant_world_ritual_pending(US).expect("a charge");
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    (g, caster)
}

#[test]
fn the_cull_kills_the_weaker_half_and_shares_their_experience() {
    let (mut g, caster) = cull_game();
    let s = spell(&g, "SPELL_THE_CULL");
    // Five units on (8, 5): strength, experience.
    let specs = [(3, 4), (2, 10), (3, 1), (5, 0), (4, 7)];
    let units: Vec<UnitId> = specs
        .iter()
        .map(|&(st, xp)| {
            let u = unit(&mut g, US, "UNIT_WARRIOR", 8, 5);
            g.units[u].base_combat = st;
            g.units[u].experience = xp;
            u
        })
        .collect();
    let settler = unit(&mut g, US, "UNIT_SETTLER", 8, 5);
    let q = g.map.plot_xy(8, 5).unwrap();
    assert_eq!(cull_units(&g, US, q), vec![units[1], units[2], units[0], units[4], units[3]], "by strength, then experience");
    assert!(g.unit_can_cast_at(caster, s, SpellTarget::Plot(q)));
    g.unit_cast_at(caster, s, SpellTarget::Plot(q));
    // Killed: the 2/10 and the 3/1. Pool 10 + 1 + 2 * 5 = 21 over 3: 7 each.
    assert!(g.units[units[1]].death_delay && g.units[units[2]].death_delay);
    assert!(!g.units[settler].death_delay, "a unit without strength is not counted");
    assert_eq!([0, 4, 3].map(|i| g.units[units[i]].experience), [4 + 7, 7 + 7, 7]);
    assert!(!g.player_world_ritual_pending(US, g.player_world_ritual(US).unwrap()), "the charge is spent");
}

#[test]
fn the_cull_gives_the_remainder_to_the_strongest() {
    let (mut g, caster) = cull_game();
    let s = spell(&g, "SPELL_THE_CULL");
    // Four units: (2, 0) and (3, 0) die: pool 10 over 2... make it uneven with three survivors.
    let specs = [(2, 0), (6, 0), (4, 0), (5, 0), (3, 0)];
    let units: Vec<UnitId> = specs
        .iter()
        .map(|&(st, xp)| {
            let u = unit(&mut g, US, "UNIT_WARRIOR", 9, 6);
            g.units[u].base_combat = st;
            g.units[u].experience = xp;
            u
        })
        .collect();
    let q = g.map.plot_xy(9, 6).unwrap();
    g.unit_cast_at(caster, s, SpellTarget::Plot(q));
    // Killed: strengths 2 and 3; pool 10 over 3 survivors: 4 to the strongest (6), 3 and 3.
    assert_eq!([1, 3, 2].map(|i| g.units[units[i]].experience), [4, 3, 3]);
}

#[test]
fn the_cull_refuses_a_plot_without_two_units() {
    let (mut g, caster) = cull_game();
    let s = spell(&g, "SPELL_THE_CULL");
    unit(&mut g, US, "UNIT_WARRIOR", 10, 5);
    unit(&mut g, US, "UNIT_SETTLER", 10, 5);
    unit(&mut g, THEM, "UNIT_WARRIOR", 12, 5);
    unit(&mut g, US, "UNIT_WARRIOR", 12, 5);
    let lone = g.map.plot_xy(10, 5).unwrap();
    let mixed = g.map.plot_xy(12, 5).unwrap();
    assert!(!g.unit_can_cast_at(caster, s, SpellTarget::Plot(lone)));
    assert!(!g.unit_can_cast_at(caster, s, SpellTarget::Plot(mixed)), "another player's units do not count");
    // Without its cast pending, not at all.
    unit(&mut g, US, "UNIT_WARRIOR", 10, 5);
    assert!(g.unit_can_cast_at(caster, s, SpellTarget::Plot(lone)));
    g.players[US].world_ritual_pending = None;
    assert!(!g.unit_can_cast_at(caster, s, SpellTarget::Plot(lone)));
}

// ---------------------------------------------------------------------------------------------
// Upgrade

#[test]
fn upgrade_reinforces_the_golems_and_stacks() {
    let mut g = game_of(&["CIVILIZATION_LUCHUIRP", "CIVILIZATION_BANNOR"]);
    let c = g.player_init_city(US, 4, 5, true, true);
    g.city_clear_order_queue(c);
    let golem = unit(&mut g, US, "UNIT_WOOD_GOLEM", 4, 5);
    let warrior = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    let pr = g.d.t::<ProjectType>("PROJECT_RITUAL_UPGRADE");
    assert_eq!(g.player_world_ritual(US), Some(pr));
    assert_eq!(g.world_ritual_spell(pr), None);
    let base = g.unit_engage_max_hp(golem);
    // The ritual needs Masonry.
    let masonry = g.d.t::<TechType>("TECH_MASONRY");
    let team = g.players[US].team;
    g.team_set_has_tech(team, masonry, true, Some(US), false, false);
    assert!(g.city_can_create(c, pr, false, false));
    queue(&mut g, c, pr);
    complete(&mut g, c);
    assert_eq!((g.units[golem].reinforced, g.units[warrior].reinforced), (1, 0));
    assert!(told(&g, "Upgrade: 1 golems reinforced.").contains(&Some(US.0)));
    assert_eq!(g.unit_engage_max_hp(golem), base * 110 / 100);
    queue(&mut g, c, pr);
    complete(&mut g, c);
    assert_eq!(g.units[golem].reinforced, 2, "a second completion stacks");
    assert_eq!(g.unit_engage_max_hp(golem), base * 120 / 100);
    // A fighter in an engagement, with its damage carried over in proportion.
    g.units[golem].damage = 50;
    let from = g.unit_plot(golem);
    let to = g.map.plot_xy(5, 5).unwrap();
    let ctx = crate::engagement::stats::HitCtx::new(from, to);
    let params = crate::engagement::formation::Params::from_defines(&g.d.defines);
    let f = g.engage_fighter(golem, crate::engagement::formation::Side::Attacker, (0, 0), false, &ctx, &params);
    assert_eq!(f.max_hp, base * 120 / 100);
    assert_eq!(f.hp, 50 * f.max_hp);
    // Saved and loaded.
    let h = Game::load(g.d, &g.save()).expect("a save of this build");
    assert_eq!(h.units[golem].reinforced, 2);
}

#[test]
fn reinforced_stays_only_on_a_golem_through_a_conversion() {
    let mut g = game_of(&["CIVILIZATION_LUCHUIRP", "CIVILIZATION_BANNOR"]);
    g.player_init_city(US, 4, 5, true, true);
    let golem = unit(&mut g, US, "UNIT_WOOD_GOLEM", 4, 5);
    g.units[golem].reinforced = 3;
    let other = unit(&mut g, US, "UNIT_CLOCKWORK_GOLEM", 4, 5);
    g.unit_convert(other, golem);
    assert_eq!(g.units[other].reinforced, 3);
    // A unit that is no golem (stacks set by hand) keeps none.
    let warrior = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    g.units[warrior].reinforced = 2;
    let axeman = unit(&mut g, US, "UNIT_AXEMAN", 4, 5);
    g.unit_convert(axeman, warrior);
    assert_eq!(g.units[axeman].reinforced, 0);
}

// ---------------------------------------------------------------------------------------------
// Wellspring

/// The Amurites (a city at (6, 5), a charge of their ritual) and the Bannor (a city at (14, 5));
/// no unit of either.
fn wellspring_game() -> Game {
    let mut g = game_of(&["CIVILIZATION_AMURITES", "CIVILIZATION_BANNOR"]);
    g.player_init_city(US, 6, 5, true, true);
    g.player_init_city(THEM, 14, 5, true, true);
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    g.player_grant_world_ritual_pending(US).expect("a charge");
    g
}

#[test]
fn wellspring_raises_a_mana_node_on_a_bare_plot_of_ours() {
    let mut g = wellspring_game();
    assert!(g.players[US].units.is_empty(), "no unit at all: the player places it");
    let s = spell(&g, "SPELL_WELLSPRING");
    let d = g.d;
    assert!(d.spells[s].place_on_charge);
    let manas = wellspring_manas(d);
    assert_eq!(manas.len(), 17, "the typed mana with a node");
    let fire = d.t::<BonusType>("BONUS_MANA_FIRE");
    let (_, node) = *manas.iter().find(|&&(b, _)| b == fire).unwrap();
    // Valid: an owned bare land plot. Invalid: the city, a resource, an improvement, a peak,
    // water, a foreign plot.
    let at = |g: &Game, x, y| g.map.plot_xy(x, y).unwrap();
    let bare = at(&g, 7, 5);
    assert_eq!(g.map.plots[bare].owner, Some(US));
    let with_bonus = at(&g, 5, 5);
    g.plot_set_bonus_type(with_bonus, Some(d.t::<BonusType>("BONUS_COW")));
    let improved = at(&g, 6, 4);
    g.plot_set_improvement_type(improved, Some(d.t::<ImprovementType>("IMPROVEMENT_FARM")));
    let peak = at(&g, 7, 6);
    g.plot_set_plot_type(peak, PlotType::PLOT_PEAK, true);
    let water = at(&g, 6, 2);
    let foreign = at(&g, 14, 4);
    let ok = |g: &Game, q| wellspring_plot_ok(g, US, q);
    assert!(ok(&g, bare));
    for q in [at(&g, 6, 5), with_bonus, improved, peak, water, foreign] {
        assert!(!ok(&g, q), "{:?}", g.map.xy(q));
    }
    assert!(g.player_can_place(US, s));
    assert!(!g.player_can_place(THEM, s), "not the Bannor's");
    let target = SpellTarget::Choice(wellspring_choice(d, bare, fire));
    let targets = g.player_place_targets(US, s);
    assert!(targets.contains(&target));
    assert!(!targets.contains(&SpellTarget::Choice(wellspring_choice(d, peak, fire))));
    assert!(!targets.contains(&SpellTarget::Choice(wellspring_choice(d, foreign, fire))));
    assert_eq!(wellspring_label(&g, wellspring_choice(d, bare, fire)), d.text.name(&d.bonuses[fire].base.description), "the mana alone: the plot is picked on the map");
    assert_eq!(g.spell_choice_plot(s, wellspring_choice(d, bare, fire)), Some(bare));
    // An invalid target places nothing and keeps the charge.
    assert!(!g.player_place(US, s, SpellTarget::Choice(wellspring_choice(d, peak, fire))));
    assert!(g.player_world_ritual_pending(US, g.player_world_ritual(US).unwrap()));
    assert!(g.player_place(US, s, target));
    assert_eq!(g.map.plots[bare].bonus_type, Some(fire));
    assert_eq!(g.map.plots[bare].improvement_type, Some(node));
    assert!(!g.player_world_ritual_pending(US, g.player_world_ritual(US).unwrap()), "the charge is spent");
    assert_eq!(g.players[US].world_spell_casts, 1);
    assert!(!told(&g, "Wellspring: ").is_empty());
    assert!(!g.player_can_place(US, s));
    assert!(g.player_place_targets(US, s).is_empty(), "no charge, no placement");
    assert!(!g.player_place(US, s, SpellTarget::Choice(wellspring_choice(d, at(&g, 8, 5), fire))));
}

/// Ritual: Wellspring needs Knowledge of the Ether (the tech that makes Fire and Death nodes
/// count): no city starts it without; with it, the capital can.
#[test]
fn wellspring_ritual_needs_knowledge_of_the_ether() {
    let mut g = wellspring_game();
    let d = g.d;
    let pr = d.t::<ProjectType>("PROJECT_RITUAL_WELLSPRING");
    let tech = d.t::<TechType>("TECH_KNOWLEDGE_OF_THE_ETHER");
    assert_eq!(d.projects[pr].tech_prereq, Some(tech));
    // The charge held from the fixture would forbid a start: spend it first.
    let fire = d.t::<BonusType>("BONUS_MANA_FIRE");
    let s = spell(&g, "SPELL_WELLSPRING");
    let bare = g.map.plot_xy(7, 5).unwrap();
    assert!(g.player_place(US, s, SpellTarget::Choice(wellspring_choice(d, bare, fire))), "a charge held is placed without the tech");
    let capital = g.player_get_capital_city(US).unwrap();
    let team = g.players[US].team;
    assert!(!g.team_is_has_tech(team, Some(tech)));
    assert!(!g.city_can_create(capital, pr, false, false));
    assert!(!g.city_can_create(capital, pr, false, true), "hidden by the rules' visibility test");
    g.team_set_has_tech(team, tech, true, Some(US), false, false);
    assert!(g.city_can_create(capital, pr, false, false));
}

/// No unit casts Wellspring, an arcane one included: the player places it, whatever units it has.
#[test]
fn wellspring_is_placed_by_the_player_not_cast_by_a_unit() {
    let mut g = wellspring_game();
    let s = spell(&g, "SPELL_WELLSPRING");
    let d = g.d;
    let adept = unit(&mut g, US, "UNIT_ADEPT", 6, 5);
    let warrior = unit(&mut g, US, "UNIT_WARRIOR", 6, 5);
    for u in [adept, warrior] {
        assert!(!g.unit_can_cast_untargeted(u, s, true), "{:?}", g.units[u].unit_type);
    }
    let fire = d.t::<BonusType>("BONUS_MANA_FIRE");
    let bare = g.map.plot_xy(7, 5).unwrap();
    let target = SpellTarget::Choice(wellspring_choice(d, bare, fire));
    assert!(g.player_place_targets(US, s).contains(&target));
    assert!(g.player_place(US, s, target), "with an arcane unit");
    assert_eq!(g.map.plots[bare].bonus_type, Some(fire));
    // Under No World Spells nothing is placed.
    g.player_grant_world_ritual_pending(US);
    g.setup.options[GameOptionType::GAMEOPTION_NO_WORLD_SPELLS] = true;
    assert!(!g.player_can_place(US, s));
}
