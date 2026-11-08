//! The battle arms cards on `Game` (docs/CARDS.md; `arms.rs`): each card's effect on a battle's
//! set-up, on the slots, on a bombard and on its promotions, in a world built by hand.

use super::*;
use crate::engagement::engage::EngageOrder;
use crate::engagement::formation::Note;
use crate::engagement::pick::WarTest;
use crate::engagement::stats::HitCtx;
use crate::setup::GameSetup;

/// A duel map, players 0 and 1 at war; `west`, `from` and `to` three unowned flat grassland plots
/// in a row, a city of player 1 on `to`; both teams know every card of this batch.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    west: PlotIdx,
    from: PlotIdx,
    to: PlotIdx,
    city: CityId,
}

const CARDS: [&str; 13] = [
    "TECH_CARD_THUNDERHEAD_TOWER",
    "TECH_CARD_ARCANE_FORTRESS",
    "TECH_CARD_LIVING_WALLS",
    "TECH_CARD_MOAT",
    "TECH_CARD_TRENCH",
    "TECH_CARD_SIEGE_BALLISTA",
    "TECH_CARD_SIEGE_COLOSSUS",
    "TECH_CARD_LAST_STAND_HYMN",
    "TECH_CARD_WARDANCE",
    "TECH_CARD_ELEMENTAL_FORGE",
    "TECH_CARD_HERALD_OF_AGARES",
    "TECH_CARD_TEMPLAR",
    "TECH_CARD_UNCONQUERED_CITY",
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
    let (west, from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            let r = g.map.plot_xy(x - 1, y)?;
            (flat(&g, q) && flat(&g, r) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((r, p, q))
        })
        .expect("three flat plots in a row");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for p in [west, from, to] {
        g.map.plots[p].terrain_type = Some(grass);
    }
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (x, y) = g.map.xy(to);
    let city = g.player_init_city(p1, x, y, true, true);
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    for card in CARDS {
        let tech = d.t::<TechType>(card);
        for t in [t0, t1] {
            g.team_set_has_tech(t, tech, true, None, false, false);
        }
    }
    World { g, p0, p1, west, from, to, city }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    fn build(&mut self, name: &str) {
        let b = self.g.d.t::<BuildingType>(name);
        if let Some(class) = self.g.d.buildings[b].building_class_type {
            self.g.cities[self.city].building_tiers[class] = BuildingTierType::STANDARD;
        }
        self.g.city_set_num_real_building(self.city, b, 1);
        self.g.city_sync_sentinels(self.city);
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

    fn index(s: &Setup, u: UnitId) -> usize {
        s.units.iter().position(|&x| x == u).expect("committed")
    }
}

/// Thunderhead Tower: each attacker in the rear row loses 30% of its max HP before the battle,
/// less its lightning resistance, never below 1 HP; the front row and those that sat out are
/// spared.
#[test]
fn the_thunderhead_tower_strikes_the_rear_row() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.build("BUILDING_THUNDERHEAD_TOWER");
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let catapult = w.unit(p0, "UNIT_CATAPULT", from);
    let wounded = w.unit(p0, "UNIT_CATAPULT", from);
    w.g.units[wounded].damage = 90;
    w.unit(p1, "UNIT_WARRIOR", to);
    let s = w.setup();
    let (ia, ic, iw) = (World::index(&s, axeman), World::index(&s, catapult), World::index(&s, wounded));
    assert_eq!((s.places[ia].1, s.places[ic].1, s.places[iw].1), (0, 2, 2), "the axeman in front, the catapults behind");
    let before: Vec<i32> = s.fighters.iter().map(|f| f.hp).collect();
    let none = vec![false; s.units.len()];
    let mut hp = before.clone();
    w.g.engage_arms_opening(&s, &none, &mut hp, None);
    assert_eq!(hp[ia], before[ia], "the front row is not struck");
    assert_eq!(before[ic] - hp[ic], 30 * s.fighters[ic].max_hp, "30% of its max HP");
    assert_eq!(hp[iw], 100, "never below 1 HP");
    // Lightning resistance cuts it.
    let immune = w.g.d.t::<PromotionType>("PROMOTION_IMMUNE_LIGHTNING");
    w.g.unit_set_has_promotion(catapult, immune, true);
    let mut hp = before.clone();
    w.g.engage_arms_opening(&s, &none, &mut hp, None);
    assert_eq!(hp[ic], before[ic], "immune to lightning");
    // A unit that sat out is not struck.
    let mut sat = none.clone();
    sat[iw] = true;
    let mut hp = before.clone();
    w.g.engage_arms_opening(&s, &sat, &mut hp, None);
    assert_eq!(hp[iw], before[iw]);
}

/// The Thunderhead Tower's strike in a real battle: logged before the battle, and the HP the
/// rear row starts the rounds with.
#[test]
fn the_thunderhead_tower_strike_is_logged() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.build("BUILDING_THUNDERHEAD_TOWER");
    w.unit(p0, "UNIT_AXEMAN", from);
    let catapult = w.unit(p0, "UNIT_CATAPULT", from);
    w.unit(p1, "UNIT_WARRIOR", to);
    let out = w.g.engage(EngageOrder { p: p0, from, to, launcher: None, formation: None, forced: None, log: Some(true) }).expect("a battle");
    let log = w.g.battle_report(out.report).and_then(|r| r.log.clone()).expect("a log");
    let b = log.fighters.iter().position(|f| f.unit == catapult).expect("in the log") as FighterIdx;
    let strike = log.events.iter().find_map(|e| match e {
        LogEvent::BuildingStrike { hits, .. } => Some(hits.clone()),
        _ => None,
    });
    let f = &log.fighters[b as usize];
    assert_eq!(strike, Some(vec![(b, 30 * f.max_hp)]));
    assert_eq!(f.hp_start - f.hp_battle, 30 * f.max_hp);
}

/// Arcane Fortress: its Sentinels take half from map bombards, and resist shots and bombards in
/// battle.
#[test]
fn the_arcane_fortress_halves_bombards_on_its_sentinels() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    w.build("BUILDING_WALLS");
    w.build("BUILDING_ARCANE_FORTRESS");
    let catapult = w.unit(p0, "UNIT_CATAPULT", from);
    let c = w.city;
    assert_eq!(w.g.city_sentinel_ranged_resist(c), 50);
    w.g.city_bombard_damage(c, catapult, 30);
    assert_eq!(w.g.cities[c].sentinel_damage, vec![15, 15]);
    let s = w.setup();
    assert!(s.sentinel_range().len() == 2 && s.sentinel_range().all(|i| s.fighters[i].arms.ranged_resist == 50));
    assert!(s.of(Side::Attacker).all(|i| s.fighters[i].arms.ranged_resist == 0));
}

/// Living Walls: the Sentinels fight with the Treant's attack and HP, grown by the city's
/// defence; one cell each.
#[test]
fn living_walls_give_the_sentinels_a_treants_attack_and_hp() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    w.build("BUILDING_WALLS");
    let c = w.city;
    let def = w.g.city_total_defense(c, false);
    assert_eq!(w.g.city_sentinel_max_hp(c), 100 * (100 + def) / 100);
    w.build("BUILDING_LIVING_WALLS");
    assert_eq!(w.g.city_total_defense(c, false), def);
    assert_eq!(w.g.city_sentinel_max_hp(c), 180 * (100 + def) / 100);
    assert_eq!(w.g.city_sentinel_a_x100(c), 1000);
    w.unit(p0, "UNIT_AXEMAN", from);
    let s = w.setup();
    let n = s.fighters.len();
    assert_eq!(s.sentinel_range().len(), 2);
    for i in s.sentinel_range() {
        let f = &s.fighters[i];
        assert!(!f.large && f.fixed);
        assert_eq!((f.a, f.max_hp), (1000, 180 * (100 + def) / 100));
        assert_eq!(s.pairs[i * n].a, 1000, "its hits on the attacker");
        assert_eq!(f.actions[0].kind, ActionKind::Strike, "it strikes from the front row, every round");
    }
}

/// Moat: in the first battle at the city each turn the attacker's melee is held in round 1.
#[test]
fn the_moat_holds_the_first_battle_of_the_turn() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.build("BUILDING_MOAT");
    // A warrior against two axemen: the city does not fall in this battle.
    w.unit(p0, "UNIT_WARRIOR", from);
    w.unit(p1, "UNIT_AXEMAN", to);
    w.unit(p1, "UNIT_AXEMAN", to);
    let s = w.setup();
    assert!(s.of(Side::Attacker).all(|i| s.fighters[i].arms.hold_melee_rounds == 1));
    assert!(s.of(Side::Defender).all(|i| s.fighters[i].arms.hold_melee_rounds == 0));
    assert!(w.g.city_first_battle_this_turn(w.city));
    let out = w.g.engage(EngageOrder { p: p0, from, to, launcher: None, formation: None, forced: None, log: Some(true) }).expect("a battle");
    let log = w.g.battle_report(out.report).and_then(|r| r.log.clone()).expect("a log");
    assert!(log.events.iter().any(|e| matches!(e, LogEvent::Action(a) if a.round == 1 && a.note == Some(Note::Held))), "the warrior was held in round 1");
    assert_eq!(w.g.cities[w.city].owner, p1);
    assert!(log.events.iter().all(|e| !matches!(e, LogEvent::Action(a) if a.round == 2 && a.note == Some(Note::Held))));
    // A battle there this turn: the next one is not held.
    assert!(!w.g.city_first_battle_this_turn(w.city));
    w.unit(p0, "UNIT_WARRIOR", from);
    let s = w.setup();
    assert!(s.of(Side::Attacker).all(|i| s.fighters[i].arms.hold_melee_rounds == 0));
}

/// Trench: the attacker has 3 fewer slots on its plot; the defender's are unchanged; every
/// worker can dig it.
#[test]
fn a_trench_narrows_the_attacker() {
    let mut w = world();
    let (from, to) = (w.from, w.to);
    // The trench on `from`, attacked from the city's plot.
    let ctx = HitCtx::new(to, from);
    let open = w.g.engage_slots(&ctx);
    let trench = w.g.d.t::<ImprovementType>("IMPROVEMENT_TRENCH");
    w.g.map.plots[from].improvement_type = Some(trench);
    let dug = w.g.engage_slots(&ctx);
    assert_eq!(dug.att(), open.att() - 3);
    assert_eq!(dug.def(), open.def());
    assert!(dug.sides[0].changes.iter().any(|c| c.src == SlotSource::Improvement(trench) && c.delta == -3));
    let d = w.g.d;
    let build = d.t::<BuildType>("BUILD_TRENCH");
    assert!(d.units[d.t::<UnitType>("UNIT_WORKER")].builds[build]);
    assert!(!d.units[d.t::<UnitType>("UNIT_AXEMAN")].builds[build]);
}

/// The Siege Colossus in battle: a first-round bombard, then trampling strikes from the front.
#[test]
fn the_siege_colossus_bombards_first_then_tramples() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.build("BUILDING_WALLS");
    let colossus = w.unit(p0, "UNIT_SIEGE_COLOSSUS", from);
    w.unit(p1, "UNIT_WARRIOR", to);
    let s = w.setup();
    let i = World::index(&s, colossus);
    let f = &s.fighters[i];
    assert_eq!(f.arms.first_round, Some((ActionKind::Bombard, AttackPattern::Single)));
    assert!(f.large && f.patterns[0] == AttackPattern::Trample);
    assert_eq!(s.places[i].1, 0, "it stands in front");
    assert_eq!(f.actions[0].count, 3, "two strikes in front, and tier 4 one more");
}

/// The Siege Colossus on the map: it bombards a city two plots away (a Catapult cannot), and the
/// city counts as besieged until its turn: no repair then.
#[test]
fn the_siege_colossus_bombards_from_two_plots_and_besieges() {
    let mut w = world();
    let (p0, west, from) = (w.p0, w.west, w.from);
    let c = w.city;
    w.build("BUILDING_WALLS");
    // A worker next to the city: it sees the city and besieges nothing.
    w.unit(p0, "UNIT_WORKER", from);
    let catapult = w.unit(p0, "UNIT_CATAPULT", west);
    let colossus = w.unit(p0, "UNIT_SIEGE_COLOSSUS", west);
    assert!(!w.g.unit_can_bombard(catapult, west), "a catapult reaches the next plot only");
    assert!(w.g.unit_can_bombard(colossus, west));
    assert!(!w.g.city_is_besieged(c), "nobody that fights is next to it");
    assert!(w.g.unit_bombard(colossus));
    let damage = w.g.cities[c].sentinel_damage.clone();
    assert!(damage.iter().all(|&d| d > 0), "{damage:?}");
    assert!(w.g.city_is_besieged(c), "the colossus's bombard besieges it");
    w.g.city_do_turn(c);
    assert_eq!(w.g.cities[c].sentinel_damage, damage, "no repair under the bombard's siege");
    assert!(!w.g.city_is_besieged(c), "until the city's turn");
}

/// Last Stand Hymn: the defender's units (not the Sentinels) carry the last stand.
#[test]
fn the_last_stand_hymn_arms_the_defenders() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.build("BUILDING_WALLS");
    w.build("BUILDING_LAST_STAND_HYMN");
    w.unit(p0, "UNIT_AXEMAN", from);
    w.unit(p1, "UNIT_WARRIOR", to);
    w.unit(p1, "UNIT_WARRIOR", to);
    let s = w.setup();
    assert_eq!(s.of(Side::Defender).len(), 2);
    assert!(s.of(Side::Defender).all(|i| s.fighters[i].arms.last_stand_pct == 100));
    assert!(s.sentinel_range().all(|i| s.fighters[i].arms.last_stand_pct == 0));
    assert!(s.of(Side::Attacker).all(|i| s.fighters[i].arms.last_stand_pct == 0));
}

/// Wardance: melee units of level 5 or more take it; it gives the first melee pattern.
#[test]
fn wardance_is_for_veteran_melee_units() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let wardance = w.g.d.t::<PromotionType>("PROMOTION_WARDANCE");
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let archer = w.unit(p0, "UNIT_ARCHER", from);
    w.g.units[axeman].level = 4;
    assert!(!w.g.unit_can_acquire_promotion(axeman, wardance), "level 4");
    w.g.units[axeman].level = 5;
    w.g.units[archer].level = 5;
    assert!(w.g.unit_can_acquire_promotion(axeman, wardance));
    assert!(!w.g.unit_can_acquire_promotion(archer, wardance), "not melee");
    w.g.unit_set_has_promotion(axeman, wardance, true);
    w.unit(p1, "UNIT_WARRIOR", to);
    let s = w.setup();
    assert_eq!(s.fighters[World::index(&s, axeman)].arms.first_melee_pattern, Some(AttackPattern::Cleave));
    assert_eq!(s.fighters[World::index(&s, archer)].arms.first_melee_pattern, None);
}

/// Templar: Crusaders alone take it; it heals the unit behind 20 HP and is immune to fear.
#[test]
fn the_templar_is_for_crusaders() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let templar = w.g.d.t::<PromotionType>("PROMOTION_TEMPLAR");
    let crusader = w.unit(p0, "UNIT_CRUSADER", from);
    let priest = w.unit(p0, "UNIT_PRIEST_OF_THE_ORDER", from);
    assert!(w.g.unit_can_acquire_promotion(crusader, templar));
    assert!(!w.g.unit_can_acquire_promotion(priest, templar), "a disciple of another class");
    assert!(!w.g.unit_is_immune_to_fear(crusader));
    w.g.unit_set_has_promotion(crusader, templar, true);
    assert!(w.g.unit_is_immune_to_fear(crusader));
    w.unit(p1, "UNIT_WARRIOR", to);
    let s = w.setup();
    assert_eq!(s.fighters[World::index(&s, crusader)].arms.heal_behind, 2000);
    assert_eq!(s.fighters[World::index(&s, priest)].arms.heal_behind, 0);
}

/// The Unconquered City: its defenders deal +50% while it has never changed hands.
#[test]
fn the_unconquered_city_arms_its_defenders_until_it_is_lost() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.unit(p0, "UNIT_AXEMAN", from);
    w.unit(p1, "UNIT_WARRIOR", to);
    let plain = w.setup();
    w.build("BUILDING_UNCONQUERED_CITY");
    assert!(w.g.city_never_lost(w.city));
    let s = w.setup();
    let n = s.fighters.len();
    let (a, dfd) = (s.of(Side::Attacker).start, s.of(Side::Defender).start);
    assert_eq!(s.pairs[dfd * n + a].pct, plain.pairs[dfd * n + a].pct * 150 / 100, "the defender's hits");
    assert_eq!(s.pairs[a * n + dfd].pct, plain.pairs[a * n + dfd].pct, "the attacker's are unchanged");
    w.g.cities[w.city].never_lost = false;
    let lost = w.setup();
    assert_eq!(lost.pairs[dfd * n + a].pct, plain.pairs[dfd * n + a].pct, "lost once, gone for good");
}

/// Elemental Forge: the two forges exclude each other; a Fireforged unit is immune to fire and
/// its attack is cut by its target's fire resistance.
#[test]
fn the_elemental_forge_turns_attacks_to_its_element() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.g.d;
    let c = w.city;
    let (fire, storm) = (d.t::<BuildingType>("BUILDING_FIRE_FORGE"), d.t::<BuildingType>("BUILDING_STORM_FORGE"));
    w.g.city_change_free_bonus(c, d.t::<BonusType>("BONUS_MANA_FIRE"), 1);
    w.g.city_change_free_bonus(c, d.t::<BonusType>("BONUS_MANA_AIR"), 1);
    assert!(w.g.city_can_construct(c, fire, false, false, false));
    assert!(w.g.city_can_construct(c, storm, false, false, false));
    w.build("BUILDING_FIRE_FORGE");
    assert!(!w.g.city_can_construct(c, storm, false, false, false), "one forge or the other");
    let forged = d.t::<PromotionType>("PROMOTION_FIREFORGED");
    let fire_dt = d.t::<DamageType>("DAMAGE_FIRE");
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let plain = w.unit(p1, "UNIT_WARRIOR", to);
    let immune = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.unit_set_has_promotion(immune, d.t::<PromotionType>("PROMOTION_IMMUNE_FIRE"), true);
    let ctx = HitCtx::new(from, to);
    let a_before = w.g.engage_hit(axeman, immune, Side::Attacker, &ctx).a;
    w.g.unit_set_has_promotion(axeman, forged, true);
    assert_eq!(w.g.unit_get_damage_type_resist(axeman, fire_dt), 100, "immune to fire");
    assert_eq!(w.g.unit_attack_damage_type(axeman), Some(fire_dt));
    assert_eq!(w.g.engage_hit(axeman, plain, Side::Attacker, &ctx).a, a_before, "a target without fire resistance takes the whole attack");
    assert!(w.g.engage_hit(axeman, immune, Side::Attacker, &ctx).a <= 1, "nothing gets through fire immunity");
    assert_eq!(w.g.engage_arms_attack(axeman, None, 400), 400, "against nobody in particular");
}

/// The Herald of Agares has Fear and is one a player; the Siege Ballista's bombards pierce.
#[test]
fn the_herald_has_fear_and_the_ballista_pierces() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let d = w.g.d;
    let herald = w.unit(p0, "UNIT_HERALD_OF_AGARES", from);
    assert!(w.g.unit_is_fear(herald));
    let class = d.units[d.t::<UnitType>("UNIT_HERALD_OF_AGARES")].unit_class_type.unwrap();
    assert!(d.unit_classes[class].hard_limit && d.unit_classes[class].max_player_instances == 1);
    let ballista = &d.units[d.t::<UnitType>("UNIT_SIEGE_BALLISTA")];
    assert_eq!(ballista.engage_patterns, [AttackPattern::Single, AttackPattern::Pierce, AttackPattern::Pierce]);
    assert_eq!((ballista.production_cost, ballista.tier), (100, 2));
}
