//! The batch "battle more" on `Game` (docs/CARDS.md; `more_cards.rs`): each card's effect on a
//! battle's set-up and on the rules it changes, in a world built by hand.

use super::*;
use crate::engagement::engage::{EngageOrder, Setup};
use crate::engagement::pick::WarTest;
use crate::engagement::report::EngageResult;
use crate::engagement::stats::HitCtx;
use crate::setup::GameSetup;
use crate::structs::SpellTarget;

/// A duel map, players 0 and 1 met and at war; `from` and `to` two unowned flat grassland plots
/// side by side; both teams know every card of this batch.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

const CARDS: [&str; 13] = [
    "TECH_CARD_SANCTIFIED_STEEL",
    "TECH_CARD_SHIELDBEARER",
    "TECH_CARD_FIRE_ARROWS",
    "TECH_CARD_WAR_DRUMS",
    "TECH_CARD_LEVIATHAN_HARPOONS",
    "TECH_CARD_RUNEGUARD",
    "TECH_CARD_WITCH_HUNTER",
    "TECH_CARD_GRYPHON_RIDERS",
    "TECH_CARD_GILDED_LEGION",
    "TECH_CARD_CAIRN_GOLEM",
    "TECH_CARD_AUTOMATONS",
    "TECH_CARD_BASTION",
    "TECH_CARD_LANTERN_OF_LUGUS",
];

fn world() -> World {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && !g.plot_is_peak(p) && !g.plot_is_hills(p) && plot.units.is_empty() && plot.owner.is_none() && plot.feature_type.is_none() && plot.improvement_type.is_none()
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((p, q))
        })
        .expect("two flat plots side by side");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for p in [from, to] {
        g.map.plots[p].terrain_type = Some(grass);
    }
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.team_declare_war(t0, t1, false, None);
    for card in CARDS {
        let tech = d.t::<TechType>(card);
        for t in [t0, t1] {
            g.team_set_has_tech(t, tech, true, None, false, false);
        }
    }
    World { g, p0, p1, from, to }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    /// The set-up of player 0's attack from `from` on `to`.
    fn setup(&self) -> Setup {
        let g = &self.g;
        let ctx = HitCtx::new(self.from, self.to);
        let slots = g.engage_slots(&ctx);
        let pool = g.engage_attackers(self.p0, self.from, self.to, WarTest::AtWar);
        let att = g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
        let force: Vec<UnitId> = att.iter().map(|p| p.unit).collect();
        let candidates = g.engage_defenders(self.to, self.p0, &force, WarTest::AtWar);
        g.engage_setup(ctx, slots, att, candidates, true).expect("a battle")
    }

    fn promote(&mut self, u: UnitId, name: &str) {
        let p = self.g.d.t::<PromotionType>(name);
        self.g.unit_set_has_promotion(u, p, true);
    }
}

/// The Bastion: its side fights with two Sentinels grown by its `battle_sentinel_defense`, in the
/// front row, at no slot; they keep nothing and hold no plot.
#[test]
fn a_bastion_brings_two_grown_sentinels_that_hold_no_plot() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    for _ in 0..4 {
        w.unit(p0, "UNIT_CHAMPION", from);
    }
    let bastion = w.unit(p1, "UNIT_BASTION", to);
    let s = w.setup();
    assert_eq!(s.sentinel_range().len(), 2, "two Sentinels from the field");
    assert!(s.city_sentinel_range().is_empty() && s.sentinel_city.is_none());
    let grow = w.g.d.units[w.g.units[bastion].unit_type].kit.sentinel_defense;
    assert_eq!(grow, 100);
    for i in s.sentinel_range() {
        assert!(s.fighters[i].fixed && s.places[i].1 == 0, "a Sentinel of the front row");
        assert_eq!(s.fighters[i].max_hp, w.g.field_sentinel_max_hp(grow));
        assert_eq!(s.fighters[i].max_hp, w.g.sentinel_stat(StatType::Hp) * 2);
        assert!(matches!(s.sentinel_source(i), Some(SentinelSource::Field { owner, grow: 100, .. }) if owner == p1));
    }
    assert_eq!((s.slots.field_sentinels, s.slots.sentinels), (2, 0));
    assert_eq!(s.slots.att(), w.g.engage_slots(&HitCtx::new(from, to)).att(), "they cost the attacker no slot");
    // A battle: the Bastion falls only to the whole force; with it alone, its Sentinels still fight.
    let out = w.g.engage(EngageOrder { p: p0, from, to, launcher: None, formation: None, forced: None, log: Some(true), surprise: false }).expect("a battle");
    let r = w.g.battle_report(out.report).expect("a report");
    assert_eq!(r.outcome.sentinels, 2);
    if !w.g.units.contains(bastion) || w.g.unit_is_dead(bastion) {
        assert_eq!(out.result, EngageResult::Taken, "no unit left: the plot falls, whatever the Sentinels did");
    }
}

/// A Bastion on the attacking side brings nothing: it cannot attack, and its Sentinels stand where
/// it stands.
#[test]
fn a_bastion_does_not_attack() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let bastion = w.unit(p0, "UNIT_BASTION", from);
    w.unit(p0, "UNIT_AXEMAN", from);
    w.unit(p1, "UNIT_AXEMAN", to);
    assert!(!w.g.unit_can_attack(bastion));
    let s = w.setup();
    assert!(!s.units.contains(&bastion));
    assert!(s.sentinel_range().is_empty());
}

/// The Cairn: the defender's units have +50% defence on it, and fight with a Sentinel grown by
/// 50%; the attacker's do not.
#[test]
fn a_cairn_gives_its_defenders_defence_and_a_sentinel() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let att = w.unit(p0, "UNIT_AXEMAN", from);
    let def = w.unit(p1, "UNIT_AXEMAN", to);
    let ctx = HitCtx::new(from, to);
    let plain = w.g.engage_hit(att, def, Side::Attacker, &ctx);
    let cairn = w.g.d.t::<ImprovementType>("IMPROVEMENT_CAIRN");
    w.g.map.plots[to].improvement_type = Some(cairn);
    let held = w.g.engage_hit(att, def, Side::Attacker, &ctx);
    assert_eq!(held.dv, plain.dv * 150 / 100, "the defender's Dv +50%");
    assert_eq!(held.a, plain.a, "the attacker's A as before");
    let back = w.g.engage_hit(def, att, Side::Defender, &ctx);
    let back_plain = {
        w.g.map.plots[to].improvement_type = None;
        let x = w.g.engage_hit(def, att, Side::Defender, &ctx);
        w.g.map.plots[to].improvement_type = Some(cairn);
        x
    };
    assert_eq!(back.a, back_plain.a, "not on the defender's A");
    let s = w.setup();
    assert_eq!(s.sentinel_range().len(), 1);
    let i = s.sentinel_range().start;
    assert_eq!(s.fighters[i].max_hp, w.g.field_sentinel_max_hp(50));
    assert_eq!(s.pair(0, i).dv, w.g.field_sentinel_def_x100(50, false));
}

/// War Drums: a side with a Drummer makes no morale roll against a Fear unit; without one it does.
#[test]
fn drums_keep_their_side_from_fear() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let a = w.unit(p0, "UNIT_AXEMAN", from);
    let fear = w.unit(p1, "UNIT_AXEMAN", to);
    w.promote(fear, "PROMOTION_FEAR");
    let units = vec![a, fear];
    let mut rng = crate::rng::Rng::default();
    let mut sat = vec![false; 2];
    let rolls = w.g.engage_morale_rolls(&units, 1, &mut sat, &mut rng);
    assert_eq!(rolls.len(), 1, "the axeman rolls against the Fear unit");
    let drummer = w.unit(p0, "UNIT_WAR_DRUMMER", from);
    let units = vec![a, drummer, fear];
    let mut sat = vec![false; 3];
    let rolls = w.g.engage_morale_rolls(&units, 2, &mut sat, &mut rng);
    assert!(rolls.is_empty(), "the drummed side rolls nothing: {rolls:?}");
    let s = {
        let mut w2 = w;
        w2.g.units[drummer].damage = 0;
        w2.setup()
    };
    let i = s.units.iter().position(|&u| u == drummer).expect("the drummer fights");
    assert_eq!(s.fighters[i].kit.drums, 1);
}

/// The kit on the fighters: the Shieldbearer's shield, Fire Arrows (and their damage to walls), the
/// Gryphon's charge, the Runeguard's, Witch-Hunter's and Automaton's immunities.
#[test]
fn the_kit_reaches_the_fighters() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let champion = w.unit(p0, "UNIT_CHAMPION", from);
    w.promote(champion, "PROMOTION_SHIELDBEARER");
    let archer = w.unit(p0, "UNIT_ARCHER", from);
    let walls_before = w.g.engage_pair_pct(archer, None);
    w.promote(archer, "PROMOTION_FIRE_ARROWS");
    assert_eq!(w.g.engage_pair_pct(archer, None), walls_before * 2, "+100% to Sentinels");
    let gryphon = w.unit(p0, "UNIT_GRYPHON_RIDER", from);
    let rune = w.unit(p1, "UNIT_RUNEGUARD", to);
    let hunter = w.unit(p1, "UNIT_WITCH_HUNTER", to);
    let auto = w.unit(p1, "UNIT_AUTOMATON", to);
    let s = w.setup();
    let f = |u: UnitId| &s.fighters[s.units.iter().position(|&x| x == u).expect("committed")].kit;
    assert_eq!(f(champion).shield, 50);
    assert!(f(archer).shots_at_walls);
    assert!(f(gryphon).charge_anywhere);
    assert!(f(rune).immune_spells && f(rune).immune_charms && f(rune).immune_hexes);
    assert!(!f(hunter).immune_spells && !f(hunter).immune_charms && f(hunter).immune_hexes);
    assert!(!f(auto).immune_spells && f(auto).immune_charms && f(auto).immune_hexes);
    assert!(w.g.unit_is_immune_to_fear(auto), "an Automaton is not alive");
}

/// The Lantern of Lugus: the Dawnbringer's shots are holy (the undead take more), and it sees every
/// invisible type within its sight of 2.
#[test]
fn the_dawnbringer_shoots_holy_and_sees_the_hidden() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let dawn = w.unit(p0, "UNIT_DAWNBRINGER", from);
    let archer = w.unit(p0, "UNIT_ARCHER", from);
    let skeleton = w.unit(p1, "UNIT_SKELETON", to);
    let holy = w.g.d.t::<DamageType>("DAMAGE_HOLY");
    assert_eq!(w.g.unit_attack_damage_type(dawn), Some(holy));
    assert_eq!(w.g.unit_attack_damage_type(archer), None);
    let ctx = HitCtx::new(from, to);
    let vs_undead = w.g.engage_hit(dawn, skeleton, Side::Attacker, &ctx).a;
    let alone = w.g.engage_a_alone(dawn, Side::Attacker, &ctx);
    assert!(vs_undead > alone, "the undead's weakness to holy: {vs_undead} against {alone}");
    assert_eq!(w.g.unit_get_num_see_invisible_types(dawn), 3);
    assert_eq!(w.g.unit_visibility_range(dawn), w.g.unit_visibility_range(archer) + 1);
}

/// Automatons gain no experience, and heal only in a city of their team with a Forge.
#[test]
fn automatons_gain_no_xp_and_heal_at_a_forge() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let auto = w.unit(p0, "UNIT_AUTOMATON", from);
    w.g.unit_change_experience(auto, 10, -1, true, false, false);
    assert_eq!(w.g.units[auto].experience, 0);
    w.g.units[auto].damage = 50;
    assert_eq!(w.g.unit_heal_rate(auto, from), 0, "no heal in the field");
    let (x, y) = w.g.map.xy(from);
    let city = w.g.player_init_city(p0, x, y, true, true);
    assert_eq!(w.g.unit_heal_rate(auto, from), 0, "none in a city without a Forge");
    let forge = w.g.d.t::<BuildingType>("BUILDING_FORGE");
    if let Some(class) = w.g.d.buildings[forge].building_class_type {
        w.g.cities[city].building_tiers[class] = BuildingTierType::STANDARD;
    }
    w.g.city_set_num_real_building(city, forge, 1);
    assert!(w.g.unit_heal_rate(auto, from) > 0, "it heals at a Forge");
    let axe = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.unit_change_experience(axe, 10, -1, true, false, false);
    assert!(w.g.units[axe].experience > 0, "other units do");
}

/// The Gryphon Riders cannot end a turn over water: at the end of their owner's turn they land on
/// the nearest land plot.
#[test]
fn a_gryphon_lands_at_the_end_of_the_turn() {
    let mut w = world();
    let p0 = w.p0;
    let sea = w.g.map.plot_ids().find(|&q| {
        let (x, y) = w.g.map.xy(q);
        w.g.plot_is_water(q) && (-1..=1).any(|dx| (-1..=1).any(|dy| w.g.plot_xy(x, y, dx, dy).is_some_and(|r| !w.g.plot_is_water(r) && !w.g.plot_is_peak(r))))
    });
    let Some(sea) = sea else { return };
    let g = &mut w.g;
    let team = g.players[p0].team;
    for q in g.map.plot_ids().collect::<Vec<_>>() {
        g.map.plots[q].revealed[team] = true;
    }
    let ty = g.d.t::<UnitType>("UNIT_GRYPHON_RIDER");
    let (x, y) = g.map.xy(sea);
    let u = g.player_init_unit(p0, ty, x, y, None, None);
    assert!(g.unit_is_flying(u) && g.unit_plot(u) == sea, "it flies over water");
    assert!(g.unit_must_land(u));
    assert!(g.unit_land_from_water(u));
    assert!(!g.plot_is_water(g.unit_plot(u)) && !g.unit_must_land(u), "it landed");
    let (lx, ly) = g.map.xy(g.unit_plot(u));
    assert_eq!(g.plot_distance(x, y, lx, ly), 1, "on the nearest land");
}

/// The Leviathan Harpoons: double damage in naval combat against the sea monsters and the great
/// warships, not against other ships; land units are untouched.
#[test]
fn harpoons_double_the_damage_against_sea_monsters() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let mut seas = w.g.map.plot_ids().filter(|&q| w.g.plot_is_water(q) && w.g.map.plots[q].units.is_empty());
    let (sea, there, other) = (seas.next().expect("a sea plot"), seas.next().expect("a sea plot"), seas.next().expect("a sea plot"));
    let (x, y) = w.g.map.xy(sea);
    let tri = w.g.d.t::<UnitType>("UNIT_TRIREME");
    let ship = w.g.player_init_unit(p0, tri, x, y, None, None);
    let (kx, ky) = w.g.map.xy(there);
    let kraken = w.g.player_init_unit(p1, w.g.d.t::<UnitType>("UNIT_KRAKEN"), kx, ky, None, None);
    let (gx, gy) = w.g.map.xy(other);
    let galley = w.g.player_init_unit(p1, w.g.d.t::<UnitType>("UNIT_GALLEY"), gx, gy, None, None);
    assert_eq!(w.g.unit_naval_damage_pct(ship, kraken), 100);
    let dealt = |g: &Game, target: UnitId| g.unit_get_defender_combat_values(ship, target, sea, 1000, 1000, None);
    let (monster_before, galley_before) = (dealt(&w.g, kraken), dealt(&w.g, galley));
    w.promote(ship, "PROMOTION_LEVIATHAN_HARPOONS");
    assert_eq!(w.g.unit_naval_damage_pct(ship, kraken), 200);
    assert_eq!(w.g.unit_naval_damage_pct(ship, galley), 100);
    let (monster, plain) = (dealt(&w.g, kraken), dealt(&w.g, galley));
    assert_eq!(monster.their_damage, monster_before.their_damage * 2, "double damage to the Kraken");
    assert_eq!(monster.our_damage, monster_before.our_damage, "the Kraken's own blows as before");
    assert_eq!(plain.their_damage, galley_before.their_damage);
    let beast = w.g.d.t::<UnitCombatType>("UNITCOMBAT_BEAST");
    let harpoons = w.g.d.t::<PromotionType>("PROMOTION_LEVIATHAN_HARPOONS");
    assert_eq!(w.g.d.promotions[harpoons].capture_unit_combat, Some(beast), "it captures the Kraken and the Leviathan");
}

/// Sanctified Steel: a melee unit trained in its city gets Demon Slaying and Undead Slaying; an
/// archer gets neither.
#[test]
fn sanctified_steel_blesses_the_melee_it_trains() {
    let mut w = world();
    let p0 = w.p0;
    let (x, y) = w.g.map.xy(w.from);
    let city = w.g.player_init_city(p0, x, y, true, true);
    let steel = w.g.d.t::<BuildingType>("BUILDING_SANCTIFIED_STEEL");
    w.g.city_set_num_real_building(city, steel, 1);
    let axe = w.unit(p0, "UNIT_AXEMAN", w.from);
    let archer = w.unit(p0, "UNIT_ARCHER", w.from);
    w.g.city_add_production_experience(city, axe, false);
    w.g.city_add_production_experience(city, archer, false);
    let demon = w.g.d.t::<PromotionType>("PROMOTION_DEMON_SLAYING");
    let undead = w.g.d.t::<PromotionType>("PROMOTION_UNDEAD_SLAYING");
    assert!(w.g.units[axe].has_promotion[demon] && w.g.units[axe].has_promotion[undead]);
    assert!(!w.g.units[archer].has_promotion[demon] && !w.g.units[archer].has_promotion[undead]);
}

/// The Gilded Legion: once its city chooses a civilization it has met (the spell, at a player of
/// it), the city trains that civilization's units of its own at 150% of their cost; a city without
/// it does not.
#[test]
fn the_gilded_legion_trains_a_chosen_civilizations_units() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let d = w.g.d;
    let luchuirp = d.t::<CivilizationType>("CIVILIZATION_LUCHUIRP");
    let bannor = d.t::<CivilizationType>("CIVILIZATION_BANNOR");
    w.g.setup.players[0].civ = Some(bannor);
    w.g.setup.players[1].civ = Some(luchuirp);
    let (x, y) = w.g.map.xy(w.from);
    let city = w.g.player_init_city(p0, x, y, true, true);
    let legion = d.t::<BuildingType>("BUILDING_GILDED_LEGION");
    w.g.city_set_num_real_building(city, legion, 1);
    // A Luchuirp unit for a class whose Bannor unit differs, and whose techs player 0 knows.
    let class = d.unit_classes.keys().find(|&k| {
        let (l, b) = (d.civ_unit(luchuirp, k), d.civ_unit(bannor, k));
        l.is_some() && l != b && !crate::utils::is_world_unit_class(d, k) && d.units[l.unwrap()].prereq_building.is_none() && d.units[l.unwrap()].prereq_building_class.is_none()
    });
    let class = class.expect("a Luchuirp unit of its own, with no building prerequisite");
    let unit = d.civ_unit(luchuirp, class).expect("a unit");
    let team = w.g.players[p0].team;
    for t in [d.units[unit].prereq_and_tech].into_iter().flatten().chain(d.units[unit].prereq_and_techs.iter().copied()) {
        w.g.team_set_has_tech(team, t, true, None, false, false);
    }
    assert!(!w.g.city_offers_unit(city, unit), "not before the choice");
    let spell = d.t::<SpellType>("SPELL_GILDED_LEGION");
    let caster = w.unit(p0, "UNIT_AXEMAN", w.from);
    assert_eq!(w.g.unit_spell_targets(caster, spell), vec![SpellTarget::Player(p1)]);
    w.g.unit_cast_at(caster, spell, SpellTarget::Player(p1));
    assert_eq!(w.g.cities[city].legion_civ, Some(luchuirp));
    assert!(w.g.city_offers_unit(city, unit));
    assert_eq!(w.g.city_legion_unit_civ(city, unit), Some(luchuirp));
    let base = w.g.player_get_production_needed_unit(p0, unit);
    assert_eq!(w.g.city_production_needed_unit(city, unit), base * 150 / 100);
    assert!(w.g.unit_spell_targets(caster, spell).is_empty(), "the choice is made once");
    w.g.city_set_num_real_building(city, legion, 0);
    assert!(!w.g.city_offers_unit(city, unit), "without the building, nothing");
}
