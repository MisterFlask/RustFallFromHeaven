//! Batch B7's cards, each in a world built by hand (`world_spells_2.rs`).

use super::*;
use crate::map::{Map, MapInitData};
use crate::setup::GameSetup;

const US: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Every name the batch's data uses resolves.
#[test]
fn registered_names_resolve() {
    for name in [
        "spellSingTheStones",
        "spellDragonsOath",
        "spellCallTheLeviathan",
        "spellJudgmentOfJunil",
        "spellEarthshapersDecree",
        "spellPlagiaristsLens",
        "spellWeather",
        "spellTidecall",
    ] {
        assert!(find_spell(name).is_some(), "{name}");
    }
    for name in ["reqSingTheStones", "reqDragonsOath", "reqEarthshapersDecree"] {
        assert!(find_req(name).is_some(), "{name}");
    }
    for name in ["targetsCallTheLeviathan", "targetsJudgmentOfJunil", "targetsPlagiaristsLens", "targetsWeather", "targetsTidecall"] {
        assert!(find_targets(name).is_some(), "{name}");
    }
    for name in ["labelCallTheLeviathan", "labelJudgmentOfJunil"] {
        assert!(find_target_label(name).is_some(), "{name}");
    }
}

/// A 20x12 map, grassland at x 2..=17, y 3..=8 and coast around it, for `n` players with no units
/// or cities, players 0 and 1 met; every plot revealed to every team; no barbarians.
fn game_of(n: usize) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, n, "WORLDSIZE_DUEL", 7);
    for (slot, name) in ["CIVILIZATION_BANNOR", "CIVILIZATION_LJOSALFAR", "CIVILIZATION_HIPPUS"].iter().take(n).enumerate() {
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
    g
}

fn game() -> Game {
    game_of(2)
}

/// `p`'s team learns the card of `spell`.
fn learn(g: &mut Game, p: PlayerId, spell: &str) -> SpellType {
    let s = g.d.t::<SpellType>(spell);
    let card = g.d.spells[s].tech_prereq.expect("a card spell needs its card");
    assert!(g.d.techs[card].card.is_some(), "{spell}: its tech is a card");
    let team = g.players[p].team;
    g.team_set_has_tech(team, card, true, Some(p), false, false);
    s
}

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, None, None)
}

fn plot(g: &Game, x: i32, y: i32) -> PlotIdx {
    g.map.plot_xy(x, y).expect("on the map")
}

fn war(g: &mut Game, a: PlayerId, b: PlayerId) {
    let (ta, tb) = (g.players[a].team, g.players[b].team);
    g.team_declare_war(ta, tb, false, None);
    assert!(g.at_war(ta, tb));
}

// ---------------------------------------------------------------------------------------------

#[test]
fn sing_the_stones_refounds_the_ruins_near_the_borders() {
    let mut g = game();
    let d = g.d;
    let spell = learn(&mut g, US, "SPELL_SING_THE_STONES");
    g.player_init_city(US, 3, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
    let ruins = d.t::<ImprovementType>("IMPROVEMENT_CITY_RUINS");
    assert!(!g.unit_can_cast(caster, spell, false), "no ruins");
    // Near: 3 plots past the city's borders (radius 1 at founding: x 2..=4), so x 7. Far: x 14.
    let (near, far) = (plot(&g, 7, 5), plot(&g, 14, 5));
    g.plot_set_improvement_type(near, Some(ruins));
    g.plot_set_improvement_type(far, Some(ruins));
    assert_eq!(sing_the_stones_ruins(&g, US), vec![near]);
    assert!(g.unit_can_cast(caster, spell, false));
    g.unit_cast(caster, spell);
    let c = g.map.plots[near].plot_city.expect("a city on the near ruins");
    assert_eq!(g.cities[c].owner, US);
    assert_eq!(g.cities[c].population, STONES_POPULATION);
    assert_ne!(g.map.plots[near].improvement_type, Some(ruins), "the ruins are gone");
    assert!(g.map.plots[far].plot_city.is_none() && g.map.plots[far].improvement_type == Some(ruins), "the far ruins stay");
    assert!(g.players[US].once_spells.contains(&spell));
}

#[test]
fn sing_the_stones_needs_room_for_a_city() {
    let mut g = game();
    let d = g.d;
    learn(&mut g, US, "SPELL_SING_THE_STONES");
    g.player_init_city(US, 3, 5, true, true);
    let ruins = d.t::<ImprovementType>("IMPROVEMENT_CITY_RUINS");
    // Next to the city: too close to found.
    let close = plot(&g, 4, 6);
    g.plot_set_improvement_type(close, Some(ruins));
    // In another civilization's land.
    let theirs = plot(&g, 7, 4);
    g.plot_set_improvement_type(theirs, Some(ruins));
    assert_eq!(sing_the_stones_ruins(&g, US), vec![theirs]);
    g.plot_set_owner(theirs, Some(THEM), false, false);
    assert!(sing_the_stones_ruins(&g, US).is_empty());
}

#[test]
fn dragons_oath_brings_sylvarith_to_the_capital_once_in_the_world() {
    let mut g = game();
    let d = g.d;
    let spell = learn(&mut g, US, "SPELL_DRAGONS_OATH");
    let capital = g.player_init_city(US, 4, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 10, 5);
    assert!(g.unit_can_cast(caster, spell, false), "cast by any unit of ours");
    g.unit_cast(caster, spell);
    let sylvarith = d.t::<UnitType>("UNIT_SYLVARITH");
    let s = g.players[US].units.iter().copied().find(|&u| g.units[u].unit_type == sylvarith).expect("Sylvarith");
    assert_eq!(g.unit_plot(s), g.city_plot(capital));
    let info = &d.units[sylvarith];
    assert!(info.large && info.moves == 3 && info.can_move_impassable);
    assert_eq!((d.unit_stat(sylvarith, StatType::Hp), d.unit_stat(sylvarith, StatType::Attack), d.unit_stat(sylvarith, StatType::Defense)), (180, 150, 120));
    assert!(d.promotions.keys().all(|p| !(g.units[s].has_promotion[p] && d.promotions[p].fear)), "no Fear");
    let breath = d.t::<SpellType>("SPELL_SYLVARITH_BREATH_FIRE");
    assert!(g.unit_can_cast(s, breath, true), "Breath Fire");
    // Another civilization with the card cannot call a second one while it lives.
    let theirs = learn(&mut g, THEM, "SPELL_DRAGONS_OATH");
    g.player_init_city(THEM, 14, 5, true, true);
    let their_caster = unit(&mut g, THEM, "UNIT_WARRIOR", 14, 5);
    assert!(!g.unit_can_cast(their_caster, theirs, false));
    g.unit_kill(s, false, None);
    assert!(g.unit_can_cast(their_caster, theirs, false), "once it is dead, it may be called again");
}

#[test]
fn call_the_leviathan_brings_a_kraken_while_the_leviathan_lives() {
    let mut g = game();
    let d = g.d;
    let spell = learn(&mut g, US, "SPELL_CALL_THE_LEVIATHAN");
    let coastal = g.player_init_city(US, 2, 5, true, true);
    let inland = g.player_init_city(US, 9, 6, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 9, 6);
    let targets = g.unit_spell_targets(caster, spell);
    assert!(targets.contains(&SpellTarget::City(coastal)) && !targets.contains(&SpellTarget::City(inland)));
    assert_eq!(g.spell_target_label(caster, spell, SpellTarget::City(coastal)), format!("{}: Leviathan", d.text.name(&g.cities[coastal].name)));
    g.unit_cast_at(caster, spell, SpellTarget::City(coastal));
    let leviathan = d.t::<UnitType>("UNIT_LEVIATHAN");
    let beast = g.players[US].units.iter().copied().find(|&u| g.units[u].unit_type == leviathan).expect("the Leviathan");
    assert_eq!(g.unit_plot(beast), g.city_plot(coastal));
    // The other civilization calls a Kraken while it lives.
    let theirs = learn(&mut g, THEM, "SPELL_CALL_THE_LEVIATHAN");
    let their_city = g.player_init_city(THEM, 17, 5, true, true);
    let their_caster = unit(&mut g, THEM, "UNIT_WARRIOR", 17, 5);
    assert_eq!(leviathan_beast(&g), Some(d.t::<UnitType>("UNIT_KRAKEN")));
    g.unit_cast_at(their_caster, theirs, SpellTarget::City(their_city));
    let kraken = d.t::<UnitType>("UNIT_KRAKEN");
    assert!(g.players[THEM].units.iter().any(|&u| g.units[u].unit_type == kraken));
}

#[test]
fn judgment_of_junil_ends_the_war_and_holds_the_peace_twenty_turns() {
    let mut g = game();
    let spell = learn(&mut g, US, "SPELL_JUDGMENT_OF_JUNIL");
    g.player_init_city(US, 3, 5, true, true);
    g.player_init_city(THEM, 15, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
    war(&mut g, US, THEM);
    let (t0, t1) = (g.players[US].team, g.players[THEM].team);
    let target = SpellTarget::Player(THEM);
    assert_eq!(g.unit_spell_targets(caster, spell), vec![target]);
    assert!(g.spell_target_label(caster, spell, target).ends_with(": at war: the war ends"));
    g.unit_cast_at(caster, spell, target);
    assert!(!g.at_war(t0, t1), "the war ends");
    assert!(g.team_has_truce(t0, t1) && g.team_has_truce(t1, t0));
    assert_eq!(g.teams[t0].truce_until[t1], g.game_turn() + TRUCE_TURNS);
    // Neither declares, and a declaration does nothing.
    assert!(!g.team_can_declare_war(t0, t1) && !g.team_can_declare_war(t1, t0));
    g.team_declare_war(t1, t0, false, None);
    assert!(!g.at_war(t0, t1));
    // Once it ends, war is possible again.
    g.teams[t0].truce_until[t1] = g.game_turn();
    g.teams[t1].truce_until[t0] = g.game_turn();
    assert!(g.team_can_declare_war(t1, t0));
}

#[test]
fn judgment_of_junil_holds_against_a_defensive_pact() {
    let mut g = game_of(3);
    let other = PlayerId(2);
    let (t0, t1, t2) = (g.players[US].team, g.players[THEM].team, g.players[other].team);
    for (a, b) in [(t0, t2), (t1, t2)] {
        g.team_meet(a, b, false);
    }
    g.player_init_city(US, 3, 5, true, true);
    g.player_init_city(THEM, 15, 5, true, true);
    g.player_init_city(other, 9, 8, true, true);
    // The third civilization has a pact with THEM, and a truce with US.
    g.team_set_defensive_pact(t1, t2, true);
    g.team_set_defensive_pact(t2, t1, true);
    impose_truce(&mut g, t0, t2);
    assert!(!g.team_wars_after_declaring(t0, t1)[t2], "the preview knows the pact will not reach");
    g.team_declare_war(t0, t1, false, None);
    assert!(g.at_war(t0, t1));
    assert!(!g.at_war(t0, t2), "the pact does not start the war the truce forbids");
}

#[test]
fn judgment_of_junil_keeps_hidden_units_from_attacking() {
    let mut g = game();
    let (t0, t1) = (g.players[US].team, g.players[THEM].team);
    let hn = g.d.t::<PromotionType>("PROMOTION_HIDDEN_NATIONALITY");
    let ours = unit(&mut g, US, "UNIT_WARRIOR", 5, 5);
    g.unit_set_has_promotion(ours, hn, true);
    let theirs = unit(&mut g, THEM, "UNIT_WARRIOR", 6, 5);
    assert!(g.unit_is_enemy(ours, Some(t1), None), "hidden nationality: an enemy of all");
    impose_truce(&mut g, t0, t1);
    assert!(!g.unit_is_enemy(ours, Some(t1), None));
    assert!(!g.unit_can_move_into(ours, g.unit_plot(theirs), true, false, false), "no attack");
}

#[test]
fn earthshapers_decree_improves_the_unimproved_land_and_keeps_forests() {
    let mut g = game();
    let d = g.d;
    let spell = learn(&mut g, US, "SPELL_EARTHSHAPERS_DECREE");
    let team = g.players[US].team;
    for t in d.techs.keys().filter(|&t| d.techs[t].card.is_none()) {
        g.team_set_has_tech(team, t, true, Some(US), false, false);
    }
    let c = g.player_init_city(US, 6, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 6, 5);
    let (hill, forest, done, outside) = (plot(&g, 7, 5), plot(&g, 5, 5), plot(&g, 6, 6), plot(&g, 12, 5));
    g.plot_set_plot_type(hill, PlotType::PLOT_HILLS, true);
    let forest_type = d.t::<FeatureType>("FEATURE_FOREST");
    g.plot_set_feature_type(forest, Some(forest_type));
    let cottage = d.t::<ImprovementType>("IMPROVEMENT_COTTAGE");
    g.plot_set_improvement_type(done, Some(cottage));
    assert!(g.unit_can_cast(caster, spell, false));
    let planned = decree_plots(&g, US);
    assert!(planned.iter().all(|&(t, _)| g.map.plots[t].owner == Some(US) && t != g.city_plot(c) && t != done));
    g.unit_cast(caster, spell);
    let yields = |t: PlotIdx, yt: YieldType| g.map.plots[t].improvement_type.map_or(0, |i| g.plot_calculate_improvement_yield_change(t, i, yt, Some(US), false, false));
    assert!(yields(hill, YieldType::YIELD_PRODUCTION) > 0, "the hill gets a mine");
    assert_eq!(g.map.plots[forest].feature_type, Some(forest_type), "the forest stays");
    assert_eq!(g.map.plots[done].improvement_type, Some(cottage), "an improved plot keeps its improvement");
    assert!(g.map.plots[outside].improvement_type.is_none(), "outside the borders");
    for (t, i) in planned {
        assert_eq!(g.map.plots[t].improvement_type, Some(i));
    }
}

#[test]
fn the_plagiarists_lens_copies_a_card_a_met_civilization_knows() {
    let mut g = game();
    let d = g.d;
    let spell = learn(&mut g, US, "SPELL_PLAGIARISTS_LENS");
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
    assert!(g.unit_spell_targets(caster, spell).is_empty(), "no card to copy");
    let card = d.t::<TechType>("TECH_CARD_ASTRAL_LEAP");
    let t1 = g.players[THEM].team;
    g.team_set_has_tech(t1, card, true, Some(THEM), false, false);
    let choice = SpellTarget::Choice(card.idx() as i32);
    assert_eq!(g.unit_spell_targets(caster, spell), vec![choice]);
    assert_eq!(g.spell_target_label(caster, spell, choice), d.text.name(&d.techs[card].base.description));
    g.unit_cast_at(caster, spell, choice);
    assert!(g.team_is_has_tech(g.players[US].team, Some(card)));
    assert_eq!(g.players[THEM].ai_attitude_extra[US], LENS_ATTITUDE);
}

/// Player 0 knows the Weather Engine, with a city at (6, 5) holding a charge and a unit in it.
fn weather_world() -> (Game, CityId, UnitId) {
    let mut g = game();
    learn(&mut g, US, "SPELL_WEATHER_STORM");
    let c = g.player_init_city(US, 6, 5, true, true);
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 6, 5);
    (g, c, caster)
}

#[test]
fn the_weather_engine_ritual_charges_its_city_once() {
    let (mut g, c, caster) = weather_world();
    let d = g.d;
    let project = d.t::<ProjectType>("PROJECT_WEATHER_ENGINE");
    let storm = d.t::<SpellType>("SPELL_WEATHER_STORM");
    assert!(g.city_can_create(c, project, false, false));
    assert!(!g.unit_can_cast_untargeted(caster, storm, true), "no charge: not even shown");
    project_built(&mut g, c, project);
    assert_eq!(g.cities[c].ritual_charges, vec![project]);
    assert!(!g.city_can_create(c, project, false, false), "not again while charged");
    assert!(g.unit_can_cast_untargeted(caster, storm, false));
    let targets = g.unit_spell_targets(caster, storm);
    assert!(targets.contains(&SpellTarget::Plot(plot(&g, 16, 5))) && !targets.contains(&SpellTarget::Plot(plot(&g, 17, 5))), "within 10 plots");
    // A unit outside the city cannot spend it.
    let outside = unit(&mut g, US, "UNIT_WARRIOR", 7, 5);
    assert!(!g.unit_can_cast_untargeted(outside, storm, true));
    g.unit_cast_at(caster, storm, SpellTarget::Plot(plot(&g, 12, 5)));
    assert!(g.cities[c].ritual_charges.is_empty(), "spent");
    assert!(g.city_can_create(c, project, false, false));
    assert_eq!(g.area_effects.len(), 1);
    assert!(g.plot_has_weather(plot(&g, 17, 5), AreaEffectKind::Storm) && !g.plot_has_weather(plot(&g, 18, 5), AreaEffectKind::Storm), "5 plots around its centre");
    // It lasts the rest of the turn and the next 5.
    let until = g.area_effects[0].until;
    assert_eq!(until, g.game_turn() + WEATHER_TURNS + 1);
}

#[test]
fn a_blizzard_stops_healing_and_takes_a_move() {
    let (mut g, _, _) = weather_world();
    let them = unit(&mut g, THEM, "UNIT_HORSEMAN", 12, 5);
    g.units[them].damage = 30;
    let at = plot(&g, 12, 5);
    let (rate, moves) = (g.unit_heal_rate(them, at), g.unit_turn_moves_at(them, at));
    assert!(rate > 0);
    let until = g.game_turn() + 3;
    g.area_effects.push(AreaEffect { kind: AreaEffectKind::Blizzard, owner: US, plot: plot(&g, 13, 6), radius: WEATHER_RADIUS, until });
    assert_eq!(g.unit_heal_rate(them, at), 0);
    assert_eq!(g.unit_turn_moves_at(them, at), moves - g.d.defines.move_denominator);
    g.unit_do_turn(them);
    assert_eq!(g.units[them].damage, 30, "no healing");
    // Ended: as before.
    g.area_effects[0].until = g.game_turn();
    assert_eq!(g.unit_heal_rate(them, at), rate);
}

#[test]
fn fog_hides_units_from_a_team_with_none_near() {
    let (mut g, _, _) = weather_world();
    let t0 = g.players[US].team;
    let theirs = unit(&mut g, THEM, "UNIT_WARRIOR", 12, 5);
    unit(&mut g, US, "UNIT_SCOUT", 10, 5);
    assert!(!g.unit_is_invisible(theirs, t0, false));
    let until = g.game_turn() + 3;
    g.area_effects.push(AreaEffect { kind: AreaEffectKind::Fog, owner: THEM, plot: plot(&g, 12, 5), radius: WEATHER_RADIUS, until });
    assert!(g.unit_is_invisible(theirs, t0, false), "two plots away in fog");
    unit(&mut g, US, "UNIT_SCOUT", 11, 5);
    assert!(!g.unit_is_invisible(theirs, t0, false), "a unit next to it sees it");
}

/// The set-up of `p0`'s attack from `from` on `to`.
fn battle_setup(g: &Game, p0: PlayerId, from: PlotIdx, to: PlotIdx) -> crate::engagement::engage::Setup {
    use crate::engagement::formation::Side;
    use crate::engagement::pick::WarTest;
    use crate::engagement::stats::HitCtx;
    let ctx = HitCtx::new(from, to);
    let slots = g.engage_slots(&ctx);
    let pool = g.engage_attackers(p0, from, to, WarTest::AtWar);
    let att = g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
    let force: Vec<UnitId> = att.iter().map(|p| p.unit).collect();
    let candidates = g.engage_defenders(to, p0, &force, WarTest::AtWar);
    g.engage_setup(ctx, slots, att, candidates, true, &[]).expect("a battle")
}

#[test]
fn storm_halves_shots_and_fog_holds_them_in_round_one() {
    let (mut g, _, _) = weather_world();
    let (from, to) = (plot(&g, 11, 5), plot(&g, 12, 5));
    unit(&mut g, US, "UNIT_AXEMAN", 11, 5);
    unit(&mut g, THEM, "UNIT_ARCHER", 12, 5);
    war(&mut g, US, THEM);
    g.player_army_sync(US);
    let s = battle_setup(&g, US, from, to);
    assert!(s.fighters.iter().all(|f| f.arms.ranged_resist == 0 && f.arms.hold_shot_rounds == 0), "clear weather");
    let until = g.game_turn() + 3;
    g.area_effects.push(AreaEffect { kind: AreaEffectKind::Storm, owner: US, plot: to, radius: WEATHER_RADIUS, until });
    let s = battle_setup(&g, US, from, to);
    assert!(s.fighters.iter().all(|f| f.arms.ranged_resist == STORM_RANGED_PERCENT));
    g.area_effects[0].kind = AreaEffectKind::Fog;
    let s = battle_setup(&g, US, from, to);
    assert!(s.fighters.iter().all(|f| f.arms.hold_shot_rounds == 1 && f.arms.ranged_resist == 0));
}

#[test]
fn the_tidecall_drowns_a_plot_by_the_water() {
    let mut g = game();
    let d = g.d;
    learn(&mut g, US, "SPELL_TIDECALL_OF_DANALIN");
    let project = d.t::<ProjectType>("PROJECT_TIDECALL_OF_DANALIN");
    let spell = d.t::<SpellType>("SPELL_TIDECALL_OF_DANALIN");
    let coastal = g.player_init_city(US, 4, 3, true, true);
    let inland = g.player_init_city(US, 10, 6, true, true);
    assert!(g.city_can_create(coastal, project, false, false));
    assert!(!g.city_can_create(inland, project, false, false), "coastal cities only");
    let caster = unit(&mut g, US, "UNIT_WARRIOR", 4, 3);
    project_built(&mut g, coastal, project);
    let shore = plot(&g, 5, 3);
    let inner = plot(&g, 5, 5);
    let targets = g.unit_spell_targets(caster, spell);
    assert!(targets.contains(&SpellTarget::Plot(shore)), "flat land by the water");
    assert!(!targets.contains(&SpellTarget::Plot(inner)), "not next to water");
    assert!(!targets.contains(&SpellTarget::Plot(g.city_plot(coastal))), "not a city");
    let soldier = unit(&mut g, US, "UNIT_WARRIOR", 5, 3);
    g.unit_cast_at(caster, spell, SpellTarget::Plot(shore));
    assert!(g.plot_is_water(shore));
    assert_eq!(g.map.plots[shore].terrain_type, Some(d.t::<TerrainType>("TERRAIN_COAST")));
    assert!(g.units.contains(soldier) && !g.plot_is_water(g.unit_plot(soldier)), "the land unit moved to land");
    assert!(g.cities[coastal].ritual_charges.is_empty());
}

#[test]
fn the_pillar_of_dawn_heals_nearby_units_and_doubles_its_cost() {
    let mut g = game();
    let d = g.d;
    let card = d.t::<TechType>("TECH_CARD_PILLAR_OF_DAWN");
    let team = g.players[US].team;
    g.team_set_has_tech(team, card, true, Some(US), false, false);
    let project = d.t::<ProjectType>("PROJECT_PILLAR_OF_DAWN");
    let c = g.player_init_city(US, 4, 5, true, true);
    assert!(g.city_can_create(c, project, false, false));
    let cost = g.player_get_production_needed_project(US, project);
    g.team_change_project_count(team, project, 1);
    assert_eq!(g.player_get_production_needed_project(US, project), 2 * cost, "twice the last");
    g.team_change_project_count(team, project, 1);
    assert_eq!(g.player_get_production_needed_project(US, project), 4 * cost);
    project_built(&mut g, c, project);
    let near = unit(&mut g, US, "UNIT_WARRIOR", 9, 5);
    let far = unit(&mut g, US, "UNIT_WARRIOR", 10, 5);
    let theirs = unit(&mut g, THEM, "UNIT_WARRIOR", 8, 6);
    for u in [near, far, theirs] {
        g.units[u].damage = 60;
    }
    for u in [near, far, theirs] {
        g.unit_do_turn(u);
    }
    assert_eq!(g.units[near].damage, 0, "within 5 plots: healed fully");
    assert!(g.units[far].damage > 0, "6 plots away");
    assert!(g.units[theirs].damage > 0, "another player's unit");
}

#[test]
fn the_second_sky_reveals_the_sites_when_built() {
    let mut g = game();
    let d = g.d;
    let team = g.players[US].team;
    let c = g.player_init_city(US, 4, 5, true, true);
    let hut = plot(&g, 15, 7);
    let goody = d.t::<ImprovementType>("IMPROVEMENT_GOODY_HUT");
    g.plot_set_improvement_type(hut, Some(goody));
    let empty = plot(&g, 15, 4);
    for t in [hut, empty] {
        g.plot_set_revealed(t, team, false, false, None, false);
    }
    let sky = d.t::<BuildingType>("BUILDING_SECOND_SKY");
    g.city_set_num_real_building(c, sky, 1);
    g.ev_building_built(c, sky);
    assert!(g.map.plots[hut].revealed[team], "the goody hut is shown");
    assert!(!g.map.plots[empty].revealed[team], "nothing else");
    // Recon units trained there see one plot farther.
    let promotion = d.t::<PromotionType>("PROMOTION_SECOND_SKY");
    assert_eq!(d.buildings[sky].free_promotion, Some(promotion));
    assert!(d.promotions[promotion].unit_combat[d.t::<UnitCombatType>("UNITCOMBAT_RECON")]);
}
