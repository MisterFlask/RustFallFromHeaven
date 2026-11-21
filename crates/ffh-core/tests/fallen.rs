//! Heroes return after they die, Wounded; Resurrection brings back another dead unit (this game's
//! rules, not FFH2's: `unit/fallen.rs`, docs/PORTING.md, "Changes from FFH2").

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::SpellTarget;
use ffh_core::unit::fallen::{HERO_RECOVERY_TURNS, HERO_RETURN_DAMAGE, RESURRECTION_TURNS};
use ffh_data::*;

/// A small map after `game_init`, the starting units gone, player 0 (the Bannor) with one city
/// (its capital) and every tech.
fn game() -> (Game, PlayerId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    let p0 = PlayerId(0);
    for u in g.players[p0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let q = g.map.plot_ids().find(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].owner.is_none()).expect("land");
    let (x, y) = g.map.xy(q);
    let c = g.player_init_city(p0, x, y, true, true);
    let team = g.players[p0].team;
    for t in d.techs.keys() {
        g.team_set_has_tech(team, t, true, Some(p0), false, false);
    }
    (g, p0, c)
}

fn add(g: &mut Game, p: PlayerId, c: CityId, unit: &str) -> UnitId {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let t = g.d.t::<UnitType>(unit);
    g.player_init_unit(p, t, x, y, None, None)
}

/// Kills `u` by harm: its HP to 0, then its delayed death.
fn kill_by_harm(g: &mut Game, u: UnitId) {
    g.unit_set_damage(u, g.d.defines.max_hit_points, Some(PlayerId(1)));
    if g.units.contains(u) {
        g.unit_do_delayed_death(u);
    }
    assert!(!g.units.contains(u));
}

fn heroes_of(g: &Game, p: PlayerId, t: UnitType) -> Vec<UnitId> {
    g.players[p].units.iter().copied().filter(|&u| g.units[u].unit_type == t).collect()
}

#[test]
fn a_dead_hero_returns_to_the_capital_wounded_with_its_promotions() {
    let (mut g, p, c) = game();
    let d = g.d;
    let donal = add(&mut g, p, c, "UNIT_DONAL");
    let t = g.units[donal].unit_type;
    let combat1 = d.t::<PromotionType>("PROMOTION_COMBAT1");
    g.unit_set_has_promotion(donal, combat1, true);
    g.unit_set_experience(donal, 30, -1);
    g.unit_set_level(donal, 4);
    let attack_pct = g.units[donal].stat_pct[StatType::Attack.idx()];
    let start = g.game_turn();

    kill_by_harm(&mut g, donal);
    assert_eq!(g.players[p].recovering.len(), 1);
    assert!(g.players[p].fallen.is_empty(), "a hero is not Resurrection's");
    let back = g.players[p].recovering[0].returns;
    assert_eq!(back, start + ffh_core::crusade::speed_turns(&g, HERO_RECOVERY_TURNS));
    assert!(g.hero_class_alive(d.units[t].unit_class_type.unwrap()), "recovering counts as alive");

    // Not before its turn.
    g.set_game_turn(back - 1);
    g.player_do_fallen(p);
    assert!(heroes_of(&g, p, t).is_empty());
    g.set_game_turn(back);
    g.player_do_fallen(p);
    let back_units = heroes_of(&g, p, t);
    assert_eq!(back_units.len(), 1);
    let u = back_units[0];
    assert!(g.players[p].recovering.is_empty());
    assert_eq!((g.units[u].x, g.units[u].y), (g.cities[c].x, g.cities[c].y), "in the capital");
    assert!(g.units[u].has_promotion[combat1]);
    assert_eq!(g.units[u].level, 4);
    assert_eq!(g.units[u].experience, 30);
    assert_eq!(g.units[u].damage, HERO_RETURN_DAMAGE);
    assert_eq!(g.units[u].wounds, 1);
    let wounded = d.t::<PromotionType>("PROMOTION_WOUNDED");
    assert!(g.units[u].has_promotion[wounded]);
    let wound = d.promotions[wounded].stat_changes[StatType::Attack.idx()].percent;
    assert!(wound < 0);
    assert_eq!(g.units[u].stat_pct[StatType::Attack.idx()], attack_pct + wound);

    // A second death: two Wounds, each counted.
    kill_by_harm(&mut g, u);
    let back = g.players[p].recovering[0].returns;
    g.set_game_turn(back);
    g.player_do_fallen(p);
    let u = heroes_of(&g, p, t)[0];
    assert_eq!(g.units[u].wounds, 2);
    assert_eq!(g.units[u].stat_pct[StatType::Attack.idx()], attack_pct + 2 * wound);

    // Losing the promotion heals them all.
    g.unit_set_has_promotion(u, wounded, false);
    assert_eq!(g.units[u].wounds, 0);
    assert_eq!(g.units[u].stat_pct[StatType::Attack.idx()], attack_pct);
}

#[test]
fn a_hero_removed_another_way_is_not_remembered() {
    let (mut g, p, c) = game();
    let donal = add(&mut g, p, c, "UNIT_DONAL");
    g.unit_kill(donal, false, None);
    assert!(g.players[p].recovering.is_empty());
    assert!(g.players[p].fallen.is_empty());
}

#[test]
fn a_hero_gains_no_experience_by_waiting() {
    let (mut g, p, c) = game();
    let donal = add(&mut g, p, c, "UNIT_DONAL");
    let xp = g.units[donal].experience;
    for _ in 0..5 {
        g.unit_do_turn(donal);
    }
    assert_eq!(g.units[donal].experience, xp);
}

#[test]
fn resurrection_brings_back_a_chosen_dead_unit_within_its_reach() {
    let (mut g, p, c) = game();
    let d = g.d;
    let champion = add(&mut g, p, c, "UNIT_CHAMPION");
    let combat1 = d.t::<PromotionType>("PROMOTION_COMBAT1");
    g.unit_set_has_promotion(champion, combat1, true);
    g.unit_set_experience(champion, 12, -1);
    g.unit_set_level(champion, 3);
    let t = g.units[champion].unit_type;
    let start = g.game_turn();
    kill_by_harm(&mut g, champion);
    assert_eq!(g.players[p].fallen.len(), 1);

    // A Life III caster offers it as Resurrection's choice.
    let caster = add(&mut g, p, c, "UNIT_ADEPT");
    g.unit_set_has_promotion(caster, d.t::<PromotionType>("PROMOTION_LIFE3"), true);
    let spell = d.t::<SpellType>("SPELL_RESSURECTION");
    let targets = g.unit_spell_targets(caster, spell);
    assert_eq!(targets, vec![SpellTarget::Choice(champion.0 as i32)]);
    assert!(g.resurrect_fallen(caster, champion.0 as i32));
    let back: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units[u].unit_type == t).collect();
    assert_eq!(back.len(), 1);
    assert!(g.units[back[0]].has_promotion[combat1]);
    assert_eq!((g.units[back[0]].level, g.units[back[0]].experience), (3, 12));
    assert!(g.players[p].fallen.is_empty());
    assert!(g.unit_spell_targets(caster, spell).is_empty(), "nothing left to bring back");

    // Out of reach after RESURRECTION_TURNS.
    kill_by_harm(&mut g, back[0]);
    let reach = ffh_core::crusade::speed_turns(&g, RESURRECTION_TURNS);
    g.set_game_turn(start + reach);
    g.player_do_fallen(p);
    assert_eq!(g.players[p].fallen.len(), 1);
    g.set_game_turn(start + reach + 1);
    g.player_do_fallen(p);
    assert!(g.players[p].fallen.is_empty());
}

#[test]
fn the_dead_survive_a_save() {
    let (mut g, p, c) = game();
    let donal = add(&mut g, p, c, "UNIT_DONAL");
    kill_by_harm(&mut g, donal);
    let warrior = add(&mut g, p, c, "UNIT_WARRIOR");
    kill_by_harm(&mut g, warrior);
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).expect("load");
    assert_eq!(h.players[p].recovering, g.players[p].recovering);
    assert_eq!(h.players[p].fallen, g.players[p].fallen);
}
