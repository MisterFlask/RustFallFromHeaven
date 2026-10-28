//! Spellcasting (`magic.rs`): constructed, deterministic scenarios on a small hand-made map --
//! a mage next to an enemy city casts Fireball before our stack attacks it, an adept that knows
//! Enchanted Blade and Haste blesses its stack rather than hasting it at the end of a quiet turn,
//! a fresh summon attacks only a fight it has a chance in unless it is about to expire and covers
//! no one, Donal recruits once in a big city, a caster builds a mana node on our raw mana and
//! Govannon teaches his spheres; a Fireball does nothing to a Fire Elemental, and the barrage
//! softens the stack at our gates before our units move.
//! They fix no unit's stats, only what the spells are for.

use std::sync::Arc;

use ffh_ai::magic;
use ffh_ai::view::View;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// One block of grassland (x 2..=17, y 3..=8) in an ocean, revealed to both players: a fresh
/// two-player AI game whose starting units are gone.
fn one_landmass_game(seed: u32) -> Game {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", seed));
    g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (w, h) = (20, 12);
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: w, grid_h: h, ..Default::default() }, np, nt);
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    let coast = d.t::<TerrainType>("TERRAIN_COAST");
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
    g.update_plot_groups();
    g
}

fn give(g: &mut Game, u: UnitId, names: &[&str]) {
    for name in names {
        let pr = g.d.t::<PromotionType>(name);
        g.unit_set_has_promotion(u, pr, true);
    }
}

#[test]
fn a_mage_fireballs_the_city_before_the_strike() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(11);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) {
        // FFH_AI_MAGIC_OFF names our civilization: nothing to test.
        return;
    }
    g.player_found(us, 4, 5);
    g.player_found(them, 12, 5);
    let target = g.players[them].capital_city.expect("the enemy founded a city");
    let tp = g.city_plot(target);
    for _ in 0..2 {
        g.player_init_unit(them, d.t::<UnitType>("UNIT_ARCHER"), 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    }
    // Our stack next to the city: a mage of the Fire sphere and two axemen about to attack.
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 11, 5, Some(UnitAIType::UNITAI_MAGE), None);
    give(&mut g, mage, &["PROMOTION_FIRE1", "PROMOTION_FIRE2"]);
    let axes: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(us, d.t::<UnitType>("UNIT_AXEMAN"), 11, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    assert!(g.at_war(ut, tt));
    let fireball = d.t::<SpellType>("SPELL_FIREBALL");
    assert!(g.unit_can_cast(mage, fireball, false), "the mage can cast Fireball next to the city");
    assert!(magic::attack_value(&g, mage, fireball, tp, &axes) > 0, "a Fireball on the defenders is worth casting");
    let hp_before: i32 = g.map.plots[tp].units.iter().map(|&t| g.unit_curr_hit_points(t)).sum();
    let defenders_before = g.map.plots[tp].units.len();
    let v = View::new(&g, us);
    magic::before_attack(&mut g, &v, tp, &axes);
    assert!(g.units[mage].has_casted, "the mage cast before the strike");
    let hp_after: i32 = g.map.plots[tp].units.iter().map(|&t| g.unit_curr_hit_points(t)).sum();
    assert!(
        hp_after < hp_before || g.map.plots[tp].units.len() < defenders_before,
        "the Fireball hurt the defenders ({hp_before} -> {hp_after} hit points)"
    );
}

#[test]
fn an_adept_blesses_its_stack_rather_than_hasting_it() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(12);
    let us = PlayerId(0);
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 4, 5);
    // An adept that knows Enchanted Blade and Haste, with three axemen, in our city; no enemy
    // anywhere near.
    let adept = g.player_init_unit(us, d.t::<UnitType>("UNIT_ADEPT"), 4, 5, Some(UnitAIType::UNITAI_MAGE), None);
    give(&mut g, adept, &["PROMOTION_ENCHANTMENT1", "PROMOTION_BODY1"]);
    let axes: Vec<UnitId> = (0..3).map(|_| g.player_init_unit(us, d.t::<UnitType>("UNIT_AXEMAN"), 4, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
    let blade = d.t::<SpellType>("SPELL_ENCHANTED_BLADE");
    let haste = d.t::<SpellType>("SPELL_HASTE");
    assert!(g.unit_can_cast(adept, blade, false) && g.unit_can_cast(adept, haste, false));
    let v = View::new(&g, us);
    let (spell, _) = magic::choose_turn_spell(&g, &v, adept).expect("the adept casts something");
    assert_eq!(d.spells[spell].base.type_, "SPELL_ENCHANTED_BLADE", "a lasting blessing for the stack, not a move it will not use");
    magic::cast(&mut g, adept, spell, "test", 0);
    let eb = d.t::<PromotionType>("PROMOTION_ENCHANTED_BLADE");
    assert!(axes.iter().all(|&a| g.units[a].has_promotion[eb]), "every axeman on the plot is blessed");
    // Next quiet turn: nothing worth casting by what it does (Haste is gone before our units
    // would use it; `casting::end_turn` then leaves the cast to FfH's own choice).
    g.units[adept].has_casted = false;
    let v = View::new(&g, us);
    let next = magic::choose_turn_spell(&g, &v, adept);
    assert!(next.map_or(true, |(s, _)| s != haste), "Haste is worth nothing at the end of a quiet turn");
}

#[test]
fn a_summon_attacks_only_at_fair_odds() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(13);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 3, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // Two enemies in the open, well apart: a Champion and a Warrior.
    let champion = g.player_init_unit(them, d.t::<UnitType>("UNIT_CHAMPION"), 8, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let warrior = g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 12, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    // Fresh summons of ours in suicide mode, as the end of the turn makes them: a Puppet next to
    // the Champion (Loki's puppets threw themselves at a Spectre every turn and left him to it),
    // an Earth Elemental next to the Warrior.
    let newest = g.players[us].units.iter().copied().max();
    let summon = |g: &mut Game, name: &str, x: i32| {
        let n = g.player_init_unit(us, d.t::<UnitType>(name), x, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        g.units[n].ai_suicide_summon = true;
        n
    };
    let puppet = summon(&mut g, "UNIT_PUPPET", 7);
    summon(&mut g, "UNIT_EARTH_ELEMENTAL", 11);
    let v = View::new(&g, us);
    ffh_ai::casting::summons_attack(&mut g, &v, newest);
    assert!(g.units.contains(puppet) && !g.units[puppet].made_attack, "the Puppet stays out of a fight it cannot win");
    assert_eq!(g.units[champion].damage, 0);
    assert!(!g.units.contains(warrior) || g.units[warrior].damage > 0, "the Earth Elemental attacks the Warrior");
}

#[test]
fn a_summon_with_two_moves_reaches_an_enemy_two_plots_away() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(14);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 3, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    let warrior = g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 9, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    g.units[warrior].damage = 60;
    // A fresh Spectre (two moves) two plots away, nothing next to it.
    let newest = g.players[us].units.iter().copied().max();
    let spectre = g.player_init_unit(us, d.t::<UnitType>("UNIT_SPECTRE"), 7, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    g.units[spectre].ai_suicide_summon = true;
    let v = View::new(&g, us);
    ffh_ai::casting::summons_attack(&mut g, &v, newest);
    assert!(!g.units.contains(warrior) || g.units[warrior].damage > 60, "the Spectre walked up to the Warrior and attacked");

    // The barbarians' summons attack only next to them (with this reach they took more cities).
    let barbarian = g.barbarian_player();
    assert!(g.at_war(g.players[barbarian].team, ut));
    let axeman = g.player_init_unit(us, d.t::<UnitType>("UNIT_AXEMAN"), 14, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    g.units[axeman].damage = 60;
    let newest = g.players[barbarian].units.iter().copied().max();
    let barbarian_spectre = g.player_init_unit(barbarian, d.t::<UnitType>("UNIT_SPECTRE"), 14, 7, Some(UnitAIType::UNITAI_ATTACK), None);
    g.units[barbarian_spectre].ai_suicide_summon = true;
    // (Its orders run in the barbarians' turn.)
    g.players[us].turn_active = false;
    g.players[barbarian].turn_active = true;
    let v = View::new(&g, barbarian);
    ffh_ai::casting::summons_attack(&mut g, &v, newest);
    assert!(g.units.contains(axeman) && g.units[axeman].damage == 60, "the barbarian Spectre stays out of reach");
}

#[test]
fn a_summon_is_worth_only_the_fights_it_would_take() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(15);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 3, 5);
    g.player_found(them, 12, 5);
    for _ in 0..2 {
        g.player_init_unit(them, d.t::<UnitType>("UNIT_CHAMPION"), 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    }
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // A mage that knows Summon Spectre next to a city its Spectre could not hurt at 5% odds.
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 11, 5, Some(UnitAIType::UNITAI_MAGE), None);
    give(&mut g, mage, &["PROMOTION_DEATH1", "PROMOTION_DEATH2"]);
    let spectre = d.t::<SpellType>("SPELL_SUMMON_SPECTRE");
    assert!(g.unit_can_cast(mage, spectre, false));
    let weight = d.spells[spectre].ai_weight;
    let v = View::new(&g, us);
    // (Even one that spends itself at bad odds, `SUMMON_SPEND`, leaves a city alone at the end of
    // the turn: its defenders heal before we strike.)
    let at_city = magic::turn_value(&g, &v, mage, spectre).expect("a summon is valued");
    assert_eq!(at_city, weight, "a Spectre that would not attack is worth nothing");
    // A wounded Warrior two plots away: the Spectre walks up and takes it on.
    let warrior = g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 9, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    g.units[warrior].damage = 60;
    let v = View::new(&g, us);
    assert!(magic::turn_value(&g, &v, mage, spectre).is_some_and(|value| value > at_city), "the Warrior in reach is worth a Spectre");
}

#[test]
fn donal_recruits_once_in_a_big_city() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(14);
    let us = PlayerId(0);
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 4, 5);
    g.player_found(us, 10, 5);
    let (big, small) = (g.players[us].cities[0], g.players[us].cities[1]);
    g.city_set_population(big, 9);
    g.city_set_population(small, 2);
    let recruit = d.t::<SpellType>("SPELL_RECRUIT");
    let donal = |g: &mut Game, x: i32| g.player_init_unit(us, d.t::<UnitType>("UNIT_DONAL"), x, 5, Some(UnitAIType::UNITAI_HERO), None);
    // In the small city: a single recruit, while the big city gives four.
    let elsewhere = donal(&mut g, 10);
    assert!(g.unit_can_cast(elsewhere, recruit, false), "Donal recruits in any city of ours");
    let v = View::new(&g, us);
    assert!(magic::turn_value(&g, &v, elsewhere, recruit).unwrap_or(0) <= 0, "not for one recruit");
    // In the big city: population 9 / 3 + 1 = 4 units, once.
    let home = donal(&mut g, 4);
    let v = View::new(&g, us);
    let (spell, value) = magic::choose_turn_spell(&g, &v, home).expect("Donal casts something");
    assert_eq!(spell, recruit, "the recruits beat Donal's other spells (value {value})");
    let before = g.players[us].units.len();
    magic::cast(&mut g, home, spell, "test", value);
    assert_eq!(g.players[us].units.len(), before + 4, "four recruits");
    g.units[home].has_casted = false;
    assert!(!g.unit_can_cast(home, recruit, false), "the Recruiter promotion is spent");
}

/// A temporary summon on its last turn (`SUMMON_SPEND`) throws itself at a fight it cannot win,
/// unless it is its stack's cover: an enemy can strike the plot next turn and the units it stands
/// with defend worse than it does.
#[test]
fn a_summon_on_its_last_turn_spends_itself_unless_it_is_cover() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(16);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) || !ffh_ai::switches::SUMMON_SPEND.on(&g, us) {
        return;
    }
    g.player_found(us, 3, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    let champion = g.player_init_unit(them, d.t::<UnitType>("UNIT_CHAMPION"), 8, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let summon = |g: &mut Game, x: i32| {
        let n = g.player_init_unit(us, d.t::<UnitType>("UNIT_PUPPET"), x, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        g.units[n].ai_suicide_summon = true;
        g.units[n].duration = 1;
        n
    };
    // Next to the Champion with a Worker to shield: it stays and defends.
    let newest = g.players[us].units.iter().copied().max();
    let guard = summon(&mut g, 7);
    g.player_init_unit(us, d.t::<UnitType>("UNIT_WORKER"), 7, 5, Some(UnitAIType::UNITAI_WORKER), None);
    let v = View::new(&g, us);
    ffh_ai::casting::summons_attack(&mut g, &v, newest);
    assert!(g.units.contains(guard) && !g.units[guard].made_attack, "the Puppet covers the Worker");
    // Alone on the other side of the Champion: it attacks whatever the odds.
    let newest = g.players[us].units.iter().copied().max();
    let alone = summon(&mut g, 9);
    let v = View::new(&g, us);
    ffh_ai::casting::summons_attack(&mut g, &v, newest);
    let fought = !g.units.contains(alone) || g.units[alone].made_attack || g.units[alone].damage > 0 || g.units[champion].damage > 0;
    assert!(fought, "the Puppet spends itself on the Champion");
}

/// Mana nodes (`MANA_NODES`): raw mana in our city's radius is a job for a caster, which goes to
/// build the node type worth most.
#[test]
fn a_caster_builds_a_mana_node_on_our_raw_mana() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(17);
    let us = PlayerId(0);
    if !ffh_ai::switches::MANA_NODES.on(&g, us) {
        return;
    }
    g.player_found(us, 5, 5);
    let team = g.players[us].team;
    for name in ["TECH_MYSTICISM", "TECH_KNOWLEDGE_OF_THE_ETHER", "TECH_DIVINATION"] {
        let t = d.t::<TechType>(name);
        g.team_set_has_tech(team, t, true, Some(us), false, false);
    }
    let raw = g.map.plot_xy(6, 6).unwrap();
    g.plot_set_bonus_type(raw, Some(d.t::<BonusType>("BONUS_MANA")));
    assert_eq!(g.map.plots[raw].owner, Some(us));
    let jobs = ffh_ai::mana::jobs(&g, us);
    assert_eq!(jobs.len(), 1, "one raw node");
    assert_eq!(jobs[0].0, raw);
    assert!(d.builds[jobs[0].1].base.type_.starts_with("BUILD_MANA_"));
    assert!(jobs[0].2 > 0, "worth something");
    let adept = g.player_init_unit(us, d.t::<UnitType>("UNIT_ADEPT"), 5, 5, Some(UnitAIType::UNITAI_MANA_UPGRADE), None);
    let grp = g.units[adept].group.unwrap();
    let mut v = View::new(&g, us);
    assert!(ffh_ai::mana::build_node(&mut g, &mut v, grp, adept), "the adept goes to build it");
    let build = g.groups[grp].mission_queue.iter().find(|m| m.mission_type == MissionType::MISSION_BUILD).map(|m| m.data1);
    assert_eq!(build, Some(jobs[0].1.i()), "the node's build is queued");
}

/// Govannon (`TEACH_SPELLS`) teaches his first-level spheres to the units he stands with.
#[test]
fn govannon_teaches_his_spheres_to_the_garrison() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(19);
    let us = PlayerId(0);
    if !ffh_ai::switches::TEACH_SPELLS.on(&g, us) {
        return;
    }
    g.player_found(us, 5, 5);
    let warriors: Vec<UnitId> = (0..3).map(|_| g.player_init_unit(us, d.t::<UnitType>("UNIT_WARRIOR"), 5, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None)).collect();
    let gov = g.player_init_unit(us, d.t::<UnitType>("UNIT_GOVANNON"), 5, 5, Some(UnitAIType::UNITAI_HERO), None);
    give(&mut g, gov, &["PROMOTION_ENCHANTMENT1"]);
    let here = g.unit_plot(gov);
    assert!(magic::teach_value(&g, gov, here) >= 3 * 40, "Enchanted Blade for three warriors");
    let grp = g.units[gov].group.unwrap();
    let mut v = View::new(&g, us);
    assert!(ffh_ai::casters::govannon(&mut g, &mut v, grp, gov));
    let ench = d.t::<PromotionType>("PROMOTION_ENCHANTMENT1");
    assert!(warriors.iter().all(|&w| g.units[w].has_promotion[ench]), "every warrior learned Enchantment I");
    assert_eq!(magic::teach_value(&g, gov, here), 0, "nothing left to teach here");
}

/// Sanctuary and March of the Trees wait for a war we are losing (`WS_LOSING_WAR`, off by
/// default; the rule itself is tested here).
#[test]
fn defensive_world_spells_wait_for_a_losing_war() {
    let mut g = one_landmass_game(19);
    let (us, them) = (PlayerId(0), PlayerId(1));
    g.player_found(us, 3, 5);
    g.player_found(them, 16, 5);
    g.players[us].power = 100;
    g.players[them].power = 200;
    assert!(!ffh_ai::world_spells::losing_war(&g, us), "at peace");
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    g.players[us].power = 100;
    g.players[them].power = 200;
    assert!(ffh_ai::world_spells::losing_war(&g, us), "their power is twice ours");
    g.players[them].power = 140;
    assert!(!ffh_ai::world_spells::losing_war(&g, us), "1.4 times ours is a war we can hold");
}

#[test]
fn an_apprentice_stays_home_and_casters_step_only_beside_our_units() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(17);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::school(&g, us) {
        // FFH_AI_CASTER_SCHOOL_OFF names our civilization: nothing to test.
        return;
    }
    g.player_found(us, 4, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // An Adept is an apprentice until it is a Mage, also at the Mage's level (it still casts an
    // Adept's spells); a Mage is not.
    let adept = g.player_init_unit(us, d.t::<UnitType>("UNIT_ADEPT"), 4, 5, Some(UnitAIType::UNITAI_MAGE), None);
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 4, 5, Some(UnitAIType::UNITAI_MAGE), None);
    assert!(magic::apprentice(&g, adept));
    assert!(!magic::apprentice(&g, mage));
    g.units[adept].level = d.units[d.t::<UnitType>("UNIT_MAGE")].min_level;
    assert!(magic::apprentice(&g, adept), "an Adept at the Mage's level waits for the upgrade at home");
    // An enemy Champion two plots east of our city, seen by a Scout of ours beside it (too weak
    // to escort a Mage): the plots around the city are in danger. The Mage heading east stays in
    // the city until a soldier of ours stands on the way.
    g.player_init_unit(us, d.t::<UnitType>("UNIT_SCOUT"), 5, 4, Some(UnitAIType::UNITAI_EXPLORE), None);
    g.player_init_unit(them, d.t::<UnitType>("UNIT_CHAMPION"), 6, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let east = g.map.plot_xy(12, 5).unwrap();
    let next = g.map.plot_xy(5, 5).unwrap();
    let here = g.unit_plot(mage);
    let mut v = View::new(&g, us);
    assert!(v.danger_at(next) > 0, "the Champion threatens the plot east of the city");
    assert!(!magic::sheltered(&g, &v, mage, next));
    assert_eq!(magic::safe_step(&g, &mut v, mage, east), Some(here), "the Mage waits in the city");
    g.player_init_unit(us, d.t::<UnitType>("UNIT_AXEMAN"), 5, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let mut v = View::new(&g, us);
    assert!(magic::sheltered(&g, &v, mage, next));
    assert_eq!(magic::safe_step(&g, &mut v, mage, east), Some(next), "the Mage steps beside the Axeman");
}

#[test]
fn a_fireball_does_nothing_to_a_fire_elemental() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(17);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) || !ffh_ai::switches::BARRAGE.on(&g, us) {
        return;
    }
    g.player_found(us, 3, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // A Fire Elemental next to our mage, in the open: immune to fire, and a Fireball is all fire.
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 8, 5, Some(UnitAIType::UNITAI_MAGE), None);
    give(&mut g, mage, &["PROMOTION_FIRE1", "PROMOTION_FIRE2"]);
    let elemental = g.player_init_unit(them, d.t::<UnitType>("UNIT_FIRE_ELEMENTAL"), 9, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let fireball = d.t::<SpellType>("SPELL_FIREBALL");
    assert!(g.unit_can_cast(mage, fireball, false));
    let ep = g.unit_plot(elemental);
    assert_eq!(magic::summon_fight(&g, us, d.t::<UnitType>("UNIT_FIREBALL"), ep), None, "a Fireball cannot fight a Fire Elemental");
    // A Warrior instead: the Fireball takes hit points from it.
    g.unit_kill(elemental, false, None);
    let warrior = g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 9, 5, Some(UnitAIType::UNITAI_ATTACK), None);
    let wp = g.unit_plot(warrior);
    assert!(magic::summon_fight(&g, us, d.t::<UnitType>("UNIT_FIREBALL"), wp).is_some_and(|(_, hp)| hp > 0));
}

#[test]
fn the_barrage_softens_the_stack_at_our_gates_before_our_units_move() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(19);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) || !ffh_ai::switches::BARRAGE.on(&g, us) {
        return;
    }
    g.player_found(us, 4, 5);
    g.player_found(them, 16, 5);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // Our city mage of the Fire sphere, and three enemy Warriors at the gates.
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 4, 5, Some(UnitAIType::UNITAI_MAGE), None);
    give(&mut g, mage, &["PROMOTION_FIRE1", "PROMOTION_FIRE2"]);
    g.player_init_unit(us, d.t::<UnitType>("UNIT_AXEMAN"), 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    let warriors: Vec<UnitId> = (0..3).map(|_| g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 5, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
    let hp = |g: &Game| warriors.iter().filter(|&&w| g.units.contains(w)).map(|&w| g.unit_curr_hit_points(w)).sum::<i32>();
    let before = hp(&g);
    let v = View::new(&g, us);
    if magic::choose_turn_spell(&g, &v, mage).map_or(true, |(s, _)| d.spells[s].base.type_ != "SPELL_FIREBALL") {
        // A civilization whose mage prefers a spell of its own (the Balseraphs' puppets): it
        // keeps it for the end of the turn.
        return;
    }
    ffh_ai::casting::barrage(&mut g, &v);
    assert!(g.units[mage].has_casted, "the mage casts at the start of the turn");
    assert!(hp(&g) < before, "the Fireball hurt the Warriors before our units move ({before} -> {})", hp(&g));
}

#[test]
fn the_force_estimate_counts_the_mages_fireballs() {
    let d = ffh_data::global();
    let mut g = one_landmass_game(20);
    let (us, them) = (PlayerId(0), PlayerId(1));
    if !magic::enabled(&g, us) {
        return;
    }
    g.player_found(us, 4, 5);
    g.player_found(them, 12, 5);
    let target = g.players[them].capital_city.expect("the enemy founded a city");
    let tp = g.city_plot(target);
    let defs: Vec<UnitId> = (0..3).map(|_| g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None)).collect();
    let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 11, 5, Some(UnitAIType::UNITAI_WARWIZARD), None);
    let (ut, tt) = (g.players[us].team, g.players[them].team);
    g.team_declare_war(ut, tt, true, None);
    // Without a damage sphere the mage softens nothing; with Fire II its Fireball takes hit points.
    assert_eq!(magic::planned_soften(&g, tp, &defs, &[mage]).iter().sum::<i32>(), 0);
    give(&mut g, mage, &["PROMOTION_FIRE1", "PROMOTION_FIRE2"]);
    let soften = magic::planned_soften(&g, tp, &defs, &[mage, mage]);
    assert_eq!(soften.len(), defs.len());
    assert!(soften.iter().sum::<i32>() > 0, "the Fireball's fight and collateral damage count: {soften:?}");
    // A caster counts once, however often it is listed.
    assert_eq!(soften, magic::planned_soften(&g, tp, &defs, &[mage]));
}
