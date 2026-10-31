//! Engagement combat's slots, who can fight and the pick (GAMEOPTION_ENGAGEMENTS, phase 6 step 5;
//! `engagement/slots.rs`, `engagement/pick.rs`): each side's slots by plot, city defense,
//! bombardment, the minimum and the river (Q9); the attack count with Blitz and bombards, the
//! combat limit and marching units in `unit_can_engage` and the gates it shares with
//! `unit_can_move_into`; the defender's candidates; the stances each side fights with; each
//! stance's pick, wounded units and Preserve Veterans' holds; force validation.

use ffh_core::army::{March, RecruitDest};
use ffh_core::engagement::report::{SlotChange, SlotSource};
use ffh_core::engagement::{HitCtx, Side, Slots, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map with armies and engagements, players 0 and 1 at war (both AI), `from` and `to` two
/// unowned flat grassland plots side by side with no feature, improvement or river crossing.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world_with(engagements: bool, war: bool) -> World {
    world_of(engagements, war, 2, "WORLDSIZE_DUEL")
}

/// [`world_with`] with `civs` players on a map of size `size`; players 0 and 1 at war if `war`.
fn world_of(engagements: bool, war: bool, civs: usize, size: &str) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, civs, size, 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = engagements;
    let mut g = Game::new(d, setup);
    g.game_init();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p)
            && !g.plot_is_peak(p)
            && !g.plot_is_hills(p)
            && plot.units.is_empty()
            && plot.owner.is_none()
            && plot.feature_type.is_none()
            && plot.improvement_type.is_none()
            && g.plot_defense_modifier(p, None, false) == 0
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
    g.map.plots[from].terrain_type = Some(grass);
    g.map.plots[to].terrain_type = Some(grass);
    if war {
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_declare_war(t0, t1, false, None);
    }
    World { g, p0, p1, from, to }
}

fn world() -> World {
    world_with(true, true)
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        (0..n).map(|_| self.unit(owner, name, at)).collect()
    }

    fn promote(&mut self, u: UnitId, name: &str) {
        let p = self.g.d.t::<PromotionType>(name);
        self.g.unit_set_has_promotion(u, p, true);
    }

    fn ctx(&self) -> HitCtx {
        HitCtx::new(self.from, self.to)
    }

    fn slots(&self) -> Slots {
        self.g.engage_slots(&self.ctx())
    }

    fn stance(&self, name: &str) -> StanceType {
        self.g.d.t::<StanceType>(name)
    }

    fn pick(&self, pool: &[UnitId], stance: &str, side: Side) -> Vec<UnitId> {
        self.g.engage_auto_pick(pool, self.stance(stance), &self.slots(), side, &self.ctx())
    }

    /// A city of `owner` on `at` whose defense modifier (no bombardment) is `pct`.
    fn city(&mut self, owner: PlayerId, at: PlotIdx, pct: i32) -> CityId {
        let (x, y) = self.g.map.xy(at);
        let c = self.g.player_init_city(owner, x, y, true, true);
        let total = self.g.city_total_defense(c, false);
        self.g.cities[c].building_defense += pct - total;
        assert_eq!(self.g.city_defense_modifier(c, false), pct);
        c
    }

    /// How many of `units` are of unit type `name`.
    fn count(&self, units: &[UnitId], name: &str) -> usize {
        let ty = self.g.d.t::<UnitType>(name);
        units.iter().filter(|&&u| self.g.units[u].unit_type == ty).count()
    }

    fn role(&self, u: UnitId) -> String {
        self.g.d.combat_roles[self.g.unit_combat_role(u).unwrap()].base.type_.clone()
    }

    fn roles(&self, units: &[UnitId], role: &str) -> usize {
        units.iter().filter(|&&u| self.role(u) == role).count()
    }
}

/// `base` plus the changes give the total, on both sides.
fn check_sums(s: &Slots) {
    for side in &s.sides {
        assert_eq!(side.base + side.changes.iter().map(|c| c.delta).sum::<i32>(), side.total, "{side:?}");
    }
    assert_eq!(s.sides[1].river_only, 0);
}

fn src(s: &Slots, side: Side) -> Vec<(SlotSource, i32)> {
    s.sides[side.index()].changes.iter().map(|SlotChange { src, delta }| (src.clone(), *delta)).collect()
}

#[test]
fn slots_follow_the_defended_plot() {
    let mut w = world();
    let d = w.g.d;
    let (grass, marsh) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_MARSH"));
    let forest = d.t::<FeatureType>("FEATURE_FOREST");
    let to = w.to;
    // (terrain, hills, feature) -> slots a side (plan 2.4).
    for (terrain, hills, feature, expect) in [
        (grass, false, None, 10),
        (grass, false, Some(forest), 6),
        (grass, true, None, 6),
        (grass, true, Some(forest), 4),
        (marsh, false, None, 8),
    ] {
        w.g.map.plots[to].terrain_type = Some(terrain);
        w.g.map.plots[to].plot_type = if hills { PlotType::PLOT_HILLS } else { PlotType::PLOT_LAND };
        w.g.map.plots[to].feature_type = feature;
        let s = w.slots();
        check_sums(&s);
        assert_eq!((s.att(), s.att_river(), s.def()), (expect, 0, expect), "{terrain:?} hills {hills} {feature:?}");
        assert_eq!(src(&s, Side::Attacker), src(&s, Side::Defender));
    }
    // The last plot: forested hills list the hills and the forest, not the terrain.
    w.g.map.plots[to].terrain_type = Some(grass);
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[to].feature_type = Some(forest);
    assert_eq!(src(&w.slots(), Side::Defender), vec![(SlotSource::Hills, -2), (SlotSource::Feature(forest), -2)]);
}

#[test]
fn a_city_costs_the_attacker_slots_a_bombard_returns_them_and_the_minimum_holds() {
    let mut w = world();
    let (p1, to) = (w.p1, w.to);
    let c = w.city(p1, to, 60);
    // 60% walls: -2 for the attacker (one per 25%), none for the defender.
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.def()), (8, 10));
    assert_eq!(src(&s, Side::Attacker), vec![(SlotSource::Terrain(w.g.d.t::<TerrainType>("TERRAIN_GRASS")), 2), (SlotSource::CityDefense { pct: 60, next_at: 75 }, -2)]);

    // Bombarded to 30%: one slot back.
    w.g.cities[c].defense_damage = 50;
    assert_eq!(w.g.city_defense_modifier(c, false), 30);
    let s = w.slots();
    check_sums(&s);
    assert_eq!(s.att(), 9);
    assert_eq!(src(&s, Side::Attacker)[2], (SlotSource::Bombarded { pct: 30 }, 1));

    // The estimator's planned bombard replaces the current defense.
    let planned = |pct| HitCtx { city_defense: Some(pct), ..w.ctx() };
    assert_eq!(w.g.engage_slots(&planned(0)).att(), 10);
    assert_eq!(w.g.engage_slots(&planned(24)).att(), 10);
    assert_eq!(w.g.engage_slots(&planned(25)).att(), 9);
    w.g.cities[c].defense_damage = 0;

    // Forested hills: 4 a side; the walls would leave the attacker 2: raised to SLOTS_MIN (3).
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.def()), (3, 4));
    assert_eq!(src(&s, Side::Attacker).last(), Some(&(SlotSource::Minimum, 1)));
}

#[test]
fn a_river_crossing_keeps_slots_for_units_that_cross_rivers() {
    let mut w = world();
    let (p0, from, to) = (w.p0, w.from, w.to);
    w.g.map.plots[from].river_crossing[DirectionType::DIRECTION_EAST.idx()] = true;
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.att_river(), s.def()), (8, 2, 10));
    assert_eq!(src(&s, Side::Attacker).last(), Some(&(SlotSource::River, -2)));
    // No approach plot (a hypothetical estimate): no river.
    assert_eq!(w.g.engage_slots(&HitCtx { from: None, ..w.ctx() }).att(), 10);

    // Ten strong Axemen and three Lizardmen (bRiver): eight Axemen, the river slots to two
    // Lizardmen.
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 10);
    for &a in &axemen {
        w.promote(a, "PROMOTION_COMBAT1");
    }
    let lizards = w.units(p0, "UNIT_LIZARDMAN", from, 3);
    assert!(w.g.unit_is_river(lizards[0]) && !w.g.unit_is_river(axemen[0]));
    let pool: Vec<UnitId> = axemen.iter().chain(&lizards).copied().collect();
    let pick = w.pick(&pool, "STANCE_BALANCED", Side::Attacker);
    assert_eq!((pick.len(), w.count(&pick, "UNIT_AXEMAN"), w.count(&pick, "UNIT_LIZARDMAN")), (10, 8, 2));
    assert_eq!(w.g.engage_validate_force(&pool, &pick, &s), Ok(()));
    // The defender has no river slots.
    assert_eq!(w.pick(&axemen, "STANCE_BALANCED", Side::Defender).len(), 10);

    // Near the minimum the river takes only what is above it: forested hills, 4 - 1.
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.att_river()), (3, 1));
}

#[test]
fn forces_are_validated() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    w.g.map.plots[from].river_crossing[DirectionType::DIRECTION_EAST.idx()] = true;
    let s = w.slots();
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 10);
    let lizard = w.unit(p0, "UNIT_LIZARDMAN", from);
    let outsider = w.unit(p0, "UNIT_AXEMAN", w.to);
    let pool: Vec<UnitId> = axemen.iter().copied().chain([lizard]).collect();
    let v = |force: &[UnitId]| w.g.engage_validate_force(&pool, force, &s);
    assert!(v(&[]).is_err(), "empty");
    assert!(v(&[axemen[0], axemen[0]]).is_err(), "twice");
    assert!(v(&[axemen[0], outsider]).is_err(), "not in the pool");
    assert!(v(&axemen[..9]).is_err(), "nine without bRiver in eight ordinary slots");
    assert_eq!(v(&[&axemen[..8], &[lizard]].concat()), Ok(()));
    let mut more = w.units(p0, "UNIT_LIZARDMAN", from, 2);
    more.extend(pool.iter().copied());
    let many: Vec<UnitId> = more.iter().copied().filter(|u| !axemen[8..].contains(u)).collect();
    assert_eq!(many.len(), 11);
    assert!(w.g.engage_validate_force(&more, &many, &s).is_err(), "more than the slots");
}

#[test]
fn each_stance_picks_its_quotas_from_a_mixed_pool() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
    let screen = w.unit(p0, "UNIT_AXEMAN", from);
    w.promote(screen, "PROMOTION_FORMATION");
    let archers = w.units(p0, "UNIT_ARCHER", from, 8);
    let horsemen = w.units(p0, "UNIT_HORSEMAN", from, 4);
    let assassins = w.units(p0, "UNIT_ASSASSIN", from, 2);
    let adepts = w.units(p0, "UNIT_ADEPT", from, 2);
    let pool: Vec<UnitId> = [axemen.clone(), vec![screen], archers, horsemen, assassins, adepts].concat();
    assert_eq!(w.role(screen), "COMBATROLE_SCREEN");
    assert_eq!(w.slots().att(), 10);

    let roles = |pick: &[UnitId]| {
        ["COMBATROLE_FRONTLINE", "COMBATROLE_SCREEN", "COMBATROLE_ARCHER", "COMBATROLE_FLANKER", "COMBATROLE_ASSASSIN", "COMBATROLE_ARCANE"].map(|r| w.roles(pick, r))
    };
    // Balanced: a screen, the front to half the slots, then the archers fill the rest.
    let pick = w.pick(&pool, "STANCE_BALANCED", Side::Attacker);
    assert_eq!(roles(&pick), [4, 1, 5, 0, 0, 0]);
    assert_eq!(pick[0], screen, "the screen step comes first");
    // Shield wall: every front unit, then archers.
    assert_eq!(roles(&w.pick(&pool, "STANCE_SHIELD_WALL", Side::Attacker)), [6, 1, 3, 0, 0, 0]);
    // Ranged line: three front units, then archers.
    let pick = w.pick(&pool, "STANCE_RANGED_LINE", Side::Attacker);
    assert_eq!(roles(&pick)[2], 7);
    assert_eq!(roles(&pick)[0] + roles(&pick)[1], 3);
    // Mobile: every flanker and assassin, then the front.
    let pick = w.pick(&pool, "STANCE_MOBILE", Side::Attacker);
    assert_eq!(roles(&pick)[2..], [0, 4, 2, 0]);
    assert_eq!(roles(&pick)[0] + roles(&pick)[1], 4);
    // Support units only come in through the fill: the whole pool on a plot with more slots
    // than units takes them.
    let small: Vec<UnitId> = pool.iter().copied().filter(|u| w.role(*u) != "COMBATROLE_ARCHER").collect();
    assert_eq!(small.len(), 15);
    let pick = w.pick(&small[5..], "STANCE_SHIELD_WALL", Side::Attacker);
    assert_eq!(pick.len(), 10, "fewer candidates than slots: all of them");
    assert_eq!(roles(&pick)[5], 2);

    // Deterministic and the same for both sides' counts; no unit twice.
    for stance in ["STANCE_BALANCED", "STANCE_SHIELD_WALL", "STANCE_RANGED_LINE", "STANCE_MOBILE", "STANCE_PRESERVE_VETERANS"] {
        for side in [Side::Attacker, Side::Defender] {
            let a = w.pick(&pool, stance, side);
            assert_eq!(a, w.pick(&pool, stance, side));
            assert_eq!(a.len(), 10);
            let mut sorted = a.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), 10);
        }
    }
}

#[test]
fn a_wounded_unit_is_taken_only_when_no_healthy_one_of_its_roles_is_left() {
    let mut w = world();
    let (p0, from, to) = (w.p0, w.from, w.to);
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    assert_eq!(w.slots().att(), 6);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 7);
    let archer = w.unit(p0, "UNIT_ARCHER", from);
    // The strongest Axeman at 40% HP.
    let hurt = axemen[0];
    w.promote(hurt, "PROMOTION_COMBAT1");
    w.g.units[hurt].damage = 60;
    let pool: Vec<UnitId> = axemen.iter().copied().chain([archer]).collect();
    let pick = w.pick(&pool, "STANCE_SHIELD_WALL", Side::Attacker);
    assert!(!pick.contains(&hurt), "six healthy Axemen fill the front");
    assert_eq!(w.count(&pick, "UNIT_AXEMAN"), 6);
    // At exactly half its HP it counts as healthy, and its strength puts it first.
    w.g.units[hurt].damage = 50;
    assert_eq!(w.pick(&pool, "STANCE_SHIELD_WALL", Side::Attacker)[0], hurt);
    // One healthy Axeman fewer: the wounded one is taken before the healthy archer (the front
    // step is for its roles).
    w.g.units[hurt].damage = 60;
    let pool: Vec<UnitId> = axemen[..6].iter().copied().chain([archer]).collect();
    let pick = w.pick(&pool, "STANCE_SHIELD_WALL", Side::Attacker);
    assert!(pick.contains(&hurt) && !pick.contains(&archer));
}

#[test]
fn preserve_veterans_holds_veterans_and_heroes_back_until_the_slots_would_go_empty() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
    let archers = w.units(p0, "UNIT_ARCHER", from, 5);
    // The army's best Axeman is a level-5 veteran.
    let vet = axemen[0];
    w.promote(vet, "PROMOTION_COMBAT1");
    w.promote(vet, "PROMOTION_COMBAT2");
    w.g.units[vet].level = 5;
    let pool: Vec<UnitId> = [axemen.clone(), archers.clone()].concat();
    assert!(w.pick(&pool, "STANCE_BALANCED", Side::Attacker).contains(&vet));
    let pick = w.pick(&pool, "STANCE_PRESERVE_VETERANS", Side::Attacker);
    assert!(!pick.contains(&vet), "a veteran never displaces a non-veteran");
    assert_eq!(pick.len(), 10);
    // Ten candidates for ten slots: the veteran comes in last.
    let pool: Vec<UnitId> = [axemen.clone(), archers[..4].to_vec()].concat();
    let pick = w.pick(&pool, "STANCE_PRESERVE_VETERANS", Side::Attacker);
    assert_eq!(pick.len(), 10);
    assert_eq!(pick.last(), Some(&vet));
    // A level-2 hero is held too; Balanced commits it.
    w.g.units[vet].level = 2;
    w.promote(vet, "PROMOTION_HERO");
    assert!(w.g.unit_is_hero(vet));
    let pool: Vec<UnitId> = [axemen.clone(), archers.clone()].concat();
    assert!(w.g.engage_is_held(vet, w.stance("STANCE_PRESERVE_VETERANS")));
    assert!(!w.g.engage_is_held(vet, w.stance("STANCE_BALANCED")));
    assert!(!w.pick(&pool, "STANCE_PRESERVE_VETERANS", Side::Attacker).contains(&vet));
    assert!(w.pick(&pool, "STANCE_BALANCED", Side::Attacker).contains(&vet));
    // The single swaps never bring a held unit in.
    let preserve = w.stance("STANCE_PRESERVE_VETERANS");
    let pick = w.pick(&pool, "STANCE_PRESERVE_VETERANS", Side::Attacker);
    let swaps = w.g.engage_single_swaps(&pool, &pick, preserve, &w.slots());
    assert!(swaps.is_empty(), "{swaps:?}: the only unit left out is the hero");
    let balanced = w.stance("STANCE_BALANCED");
    let swaps = w.g.engage_single_swaps(&pool, &pick, balanced, &w.slots());
    assert_eq!(swaps.len(), 10);
    assert!(swaps.iter().all(|&(out, inn)| inn == vet && pick.contains(&out)));
    // With no swap to make (the only unit left out is held), the proposal is the quota pick.
    assert_eq!(w.g.engage_propose_force(&pool, preserve, &w.slots(), &w.ctx()), pick);
}

#[test]
fn blitz_allows_one_more_attack_and_every_attack_test_agrees() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.unit(p1, "UNIT_WARRIOR", to);
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let blitz = w.unit(p0, "UNIT_HORSEMAN", from);
    w.promote(blitz, "PROMOTION_BLITZ");
    let g = &mut w.g;
    let agree = |g: &Game, u: UnitId| {
        let engage = g.unit_can_engage(u, to, WarTest::AtWar);
        let grp = g.units[u].group.unwrap();
        assert_eq!(engage, g.unit_can_move_into(u, to, true, false, false), "unit_can_move_into");
        assert_eq!(engage, g.group_can_move_into(grp, to, true), "group_can_move_into");
        assert_eq!(engage, g.group_ai_get_best_group_attacker(grp, to, false, false, false).0 == Some(u), "the best group attacker");
        assert_eq!(engage, g.engage_attackers(p0, from, to, WarTest::AtWar).contains(&u), "the pool");
        engage
    };
    for (made, axe, horse) in [(0, true, true), (1, false, true), (2, false, false)] {
        for u in [axeman, blitz] {
            g.units[u].attacks_made = made;
            g.units[u].made_attack = made > 0;
        }
        assert_eq!((agree(g, axeman), agree(g, blitz)), (axe, horse), "after {made} attacks");
    }
    // Another attack this turn (a duel: boarding a ship) counts as one.
    g.units[axeman].attacks_made = 0;
    g.units[axeman].made_attack = true;
    assert!(!agree(g, axeman));
    g.units[axeman].made_attack = false;
    assert!(agree(g, axeman));

    // Off, the duel's rule: made_attack, Blitz ignoring the counter.
    let mut off = world_with(false, true);
    let (from, to) = (off.from, off.to);
    off.unit(PlayerId(1), "UNIT_WARRIOR", to);
    let a = off.unit(PlayerId(0), "UNIT_AXEMAN", from);
    assert!(!off.g.unit_can_engage(a, to, WarTest::AtWar));
    assert!(off.g.unit_can_move_into(a, to, true, false, false));
    off.g.units[a].attacks_made = 5;
    assert!(off.g.unit_can_move_into(a, to, true, false, false));
    off.g.units[a].made_attack = true;
    assert!(!off.g.unit_can_move_into(a, to, true, false, false));
}

#[test]
fn a_bombard_spends_the_attack_and_blitz_leaves_one() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.city(p1, to, 50);
    w.unit(p1, "UNIT_WARRIOR", to);
    let cat = w.unit(p0, "UNIT_CATAPULT", from);
    let fast = w.unit(p0, "UNIT_CATAPULT", from);
    w.promote(fast, "PROMOTION_BLITZ");
    let g = &mut w.g;
    for u in [cat, fast] {
        assert!(g.unit_can_bombard(u, from) && g.unit_can_engage(u, to, WarTest::AtWar));
        assert!(g.unit_bombard(u));
        assert_eq!((g.units[u].attacks_made, g.units[u].made_attack), (1, true));
        g.units[u].moves = 0; // moves to spare, to see the attack count alone
    }
    assert!(!g.unit_can_bombard(cat, from) && !g.unit_can_engage(cat, to, WarTest::AtWar));
    assert!(g.unit_can_bombard(fast, from) && g.unit_can_engage(fast, to, WarTest::AtWar));
    assert!(g.unit_bombard(fast));
    g.units[fast].moves = 0;
    assert!(!g.unit_can_bombard(fast, from) && !g.unit_can_engage(fast, to, WarTest::AtWar));
}

#[test]
fn a_unit_with_a_combat_limit_needs_a_defender_below_it() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let cat = w.unit(p0, "UNIT_CATAPULT", from);
    let limit = w.g.units[cat].combat_limit;
    assert!(limit < 100);
    let a = w.unit(p1, "UNIT_WARRIOR", to);
    let b = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[a].damage = limit;
    w.g.units[b].damage = limit + 5;
    assert!(!w.g.unit_can_engage(cat, to, WarTest::AtWar), "every defender at the cap");
    assert!(!w.g.unit_can_move_into(cat, to, true, false, false));
    w.g.units[b].damage = limit - 1;
    assert!(w.g.unit_can_engage(cat, to, WarTest::AtWar), "one defender below it");
    assert!(w.g.unit_can_move_into(cat, to, true, false, false));
    // An Axeman has no limit.
    w.g.units[b].damage = limit;
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    assert!(w.g.unit_can_engage(axeman, to, WarTest::AtWar));
    assert_eq!(w.g.engage_attackers(p0, from, to, WarTest::AtWar), vec![axeman]);
}

/// The rules change: the duel's best defender already prefers a unit the siege unit can hurt, but
/// a Marksman's (bTargetWeakestUnit) picks the capped one, so today it may not attack; under
/// engagements it attacks while any defender is below its limit.
#[test]
fn a_siege_unit_attacks_while_any_defender_is_below_its_limit_whoever_the_best_defender_is() {
    for engagements in [true, false] {
        let mut w = world_with(engagements, true);
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        let cat = w.unit(p0, "UNIT_CATAPULT", from);
        w.promote(cat, "PROMOTION_MARKSMAN");
        assert!(w.g.unit_is_target_weakest_unit(cat));
        let limit = w.g.units[cat].combat_limit;
        let axeman = w.unit(p1, "UNIT_AXEMAN", to);
        let warrior = w.unit(p1, "UNIT_WARRIOR", to);
        w.g.units[axeman].damage = limit;
        w.g.units[warrior].damage = limit - 1;
        let owner = Some(p0);
        assert_eq!(w.g.plot_get_best_defender(to, None, owner, Some(cat), true, false, false), Some(axeman), "the capped unit is the duel's best defender");
        assert_eq!(w.g.unit_can_move_into(cat, to, true, false, false), engagements);
        assert_eq!(w.g.unit_can_engage(cat, to, WarTest::AtWar), engagements);
        w.g.units[warrior].damage = limit;
        assert!(!w.g.unit_can_move_into(cat, to, true, false, false), "every defender at the cap");
        assert!(!w.g.unit_can_engage(cat, to, WarTest::AtWar));
    }
}

/// As today: a siege unit attacks no plot from the water (a non-combatant on it is enough to
/// refuse it), and from land it needs a unit below its limit even when the plot holds only
/// non-combatants (the attack captures them).
#[test]
fn a_siege_unit_does_not_attack_from_the_water_even_onto_workers() {
    for engagements in [true, false] {
        let mut w = world_with(engagements, true);
        let (p0, p1) = (w.p0, w.p1);
        // A plot of land with no units, next to the water.
        let g = &w.g;
        let (sea, coast) = g
            .map
            .plot_ids()
            .filter(|&p| !g.plot_is_water(p) && !g.plot_is_peak(p) && g.map.plots[p].units.is_empty() && g.map.plots[p].owner.is_none())
            .find_map(|p| {
                let (x, y) = g.map.xy(p);
                let q = g.map.plot_xy(x - 1, y)?;
                (g.plot_is_water(q) && g.map.plots[q].units.is_empty()).then_some((q, p))
            })
            .expect("a coast");
        let worker = w.unit(p1, "UNIT_WORKER", coast);
        let aboard = |w: &mut World, name: &str| {
            let ship = w.unit(p0, "UNIT_TRIREME", sea);
            let u = w.unit(p0, name, sea);
            w.g.unit_set_transport_unit(u, Some(ship));
            w.g.units[u].moves = 0;
            assert!(w.g.unit_is_cargo(u));
            u
        };
        let cat = aboard(&mut w, "UNIT_CATAPULT");
        let axeman = aboard(&mut w, "UNIT_AXEMAN");
        assert!(w.g.unit_can_move_into(axeman, coast, true, false, false), "an amphibious attack");
        assert!(!w.g.unit_can_move_into(cat, coast, true, false, false), "no amphibious artillery");
        if engagements {
            assert!(w.g.unit_can_engage(axeman, coast, WarTest::AtWar));
            assert!(!w.g.unit_can_engage(cat, coast, WarTest::AtWar));
            assert!(w.g.engage_defenders(coast, p0, &[axeman], WarTest::AtWar).is_empty(), "a worker does not defend");
        }
        // From land: the worker below the limit lets it capture, at the limit it does not.
        let (x, y) = w.g.map.xy(coast);
        let land = w
            .g
            .map
            .plot_ids()
            .find(|&p| {
                let (px, py) = w.g.map.xy(p);
                w.g.step_distance(px, py, x, y) == 1 && !w.g.plot_is_water(p) && !w.g.plot_is_peak(p) && w.g.map.plots[p].units.is_empty()
            })
            .expect("land next to the coast");
        let siege = w.unit(p0, "UNIT_CATAPULT", land);
        assert!(w.g.unit_can_move_into(siege, coast, true, false, false));
        w.g.units[worker].damage = w.g.units[siege].combat_limit;
        assert!(!w.g.unit_can_move_into(siege, coast, true, false, false));
    }
}

#[test]
fn the_pool_and_the_defenders() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let warrior = w.unit(p1, "UNIT_WARRIOR", to);
    let worker = w.unit(p1, "UNIT_WORKER", to);
    let hidden = w.unit(p1, "UNIT_WARRIOR", to);
    w.promote(hidden, "PROMOTION_HIDDEN");
    let marcher = w.unit(p1, "UNIT_WARRIOR", to);
    // (Only whether it marches matters here: the city is a placeholder.)
    let march = March { dest: RecruitDest::Garrison(CityId(0)), stalled: 0, left_turn: 0, from_city: CityId(0), diverted: false };
    w.g.units[marcher].march = Some(march.clone());
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    assert!(w.g.unit_is_invisible(hidden, w.g.players[p0].team, true));
    assert!(!w.g.unit_can_defend(worker, Some(to)));
    assert_eq!(w.g.engage_defenders(to, p0, &[axeman], WarTest::AtWar), vec![warrior, marcher], "no worker, no invisible unit; the marching unit defends");

    // The pool: no marching unit, no unit out of moves, no missile, nothing not next to `to`.
    let own_marcher = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[own_marcher].march = Some(march);
    let tired = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[tired].moves = w.g.unit_max_moves(tired);
    let worker0 = w.unit(p0, "UNIT_WORKER", from);
    let missile = w.unit(p0, "UNIT_FIREBALL", from);
    assert!(w.g.unit_is_suicide(missile));
    let (fx, fy) = w.g.map.xy(from);
    let far = w.g.map.plot_xy(fx - 1, fy).unwrap();
    let far_unit = w.unit(p0, "UNIT_AXEMAN", far);
    assert!(!w.g.unit_can_engage(far_unit, to, WarTest::AtWar));
    assert_eq!(w.g.engage_attackers(p0, from, to, WarTest::AtWar), vec![axeman]);
    let _ = (worker0, tired);
}

#[test]
fn a_declare_war_preview_counts_the_team_it_would_fight() {
    let mut w = world_with(true, false);
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let warrior = w.unit(p1, "UNIT_WARRIOR", to);
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let t1 = w.g.players[p1].team;
    assert!(!w.g.unit_can_engage(axeman, to, WarTest::AtWar), "at peace");
    assert!(w.g.engage_defenders(to, p0, &[axeman], WarTest::AtWar).is_empty());
    assert!(w.g.unit_can_engage(axeman, to, WarTest::DeclareWar(t1)));
    assert_eq!(w.g.engage_defenders(to, p0, &[axeman], WarTest::DeclareWar(t1)), vec![warrior]);
    assert_eq!(w.g.engage_attackers(p0, from, to, WarTest::DeclareWar(t1)), vec![axeman]);
}

/// A declaration brings in the target's vassals and its defensive-pact partners (and theirs):
/// the preview's defenders are the ones the fight after the declaration meets.
#[test]
fn a_declare_war_preview_counts_every_team_the_declaration_brings_in() {
    let mut w = world_of(true, false, 6, "WORLDSIZE_STANDARD");
    let (p0, from, to) = (w.p0, w.from, w.to);
    let [t1, vassal, pact, pact_vassal, neutral] = [1, 2, 3, 4, 5].map(|i| w.g.players[PlayerId(i)].team);
    w.g.team_set_vassal(vassal, t1, true, false);
    w.g.team_set_vassal(pact_vassal, pact, true, false);
    let dp = [ffh_core::utils::set_trade_item(TradeableItem::TRADE_DEFENSIVE_PACT, 0)];
    w.g.implement_deal(PlayerId(1), PlayerId(3), &dp, &dp, true);
    assert!(w.g.teams[t1].defensive_pact[pact] && w.g.teams[pact].defensive_pact[t1]);
    let defenders: Vec<UnitId> = (1..=5).map(|i| w.unit(PlayerId(i), "UNIT_WARRIOR", to)).collect();
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let t0 = w.g.players[p0].team;

    let wars = w.g.team_wars_after_declaring(t0, t1);
    assert_eq!([t1, vassal, pact, pact_vassal, neutral].map(|t| wars[t]), [true, true, true, true, false]);
    let preview = w.g.engage_defenders(to, p0, &[axeman], WarTest::DeclareWar(t1));
    assert_eq!(preview, defenders[..4].to_vec(), "not the neutral team's warrior");
    assert!(w.g.engage_defenders(to, p0, &[axeman], WarTest::AtWar).is_empty());
    assert!(w.g.unit_can_engage(axeman, to, WarTest::DeclareWar(t1)));

    w.g.team_declare_war(t0, t1, false, None);
    let at_war: Vec<TeamId> = w.g.team_ids().filter(|&t| w.g.teams[t0].at_war[t]).collect();
    assert_eq!(at_war, w.g.team_ids().filter(|&t| wars[t]).collect::<Vec<_>>(), "the cascade as predicted");
    assert_eq!(w.g.engage_defenders(to, p0, &[axeman], WarTest::AtWar), preview);
}

#[test]
fn the_stance_each_side_fights_with() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let (mobile, ranged, wall) = (w.stance("STANCE_MOBILE"), w.stance("STANCE_RANGED_LINE"), w.stance("STANCE_SHIELD_WALL"));
    let default = w.g.d.default_stance;

    // The attacker: the launching group's army.
    let big = w.units(p0, "UNIT_AXEMAN", from, 3);
    let small = w.units(p0, "UNIT_AXEMAN", from, 2);
    let a_big = w.g.army_form(p0, &big).unwrap();
    let a_small = w.g.army_form(p0, &small).unwrap();
    w.g.army_set_stance(p0, a_big, ranged).unwrap();
    w.g.army_set_stance(p0, a_small, wall).unwrap();
    let grp_small = w.g.units[small[0]].group.unwrap();
    let s = w.g.engage_attack_stance(p0, from, Some(grp_small));
    assert_eq!((s.stance, s.army, s.garrison_of), (wall, Some(a_small), None));
    // A unit in no army (a mage): the largest army on the plot.
    let adept = w.unit(p0, "UNIT_ADEPT", from);
    assert!(w.g.units[adept].army.is_none());
    let s = w.g.engage_attack_stance(p0, from, w.g.units[adept].group);
    assert_eq!((s.stance, s.army), (ranged, Some(a_big)));
    assert_eq!(w.g.engage_attack_stance(p0, from, None).stance, ranged);
    // Barbarians: the default.
    let barb = w.g.barbarian_player();
    assert_eq!(w.g.engage_attack_stance(barb, from, None).stance, default);

    // The defender in the field: the largest army among the candidates.
    let def_big = w.units(p1, "UNIT_WARRIOR", to, 3);
    let def_small = w.units(p1, "UNIT_WARRIOR", to, 1);
    let d_big = w.g.army_form(p1, &def_big).unwrap();
    let d_small = w.g.army_form(p1, &def_small).unwrap();
    w.g.army_set_stance(p1, d_big, mobile).unwrap();
    w.g.army_set_stance(p1, d_small, wall).unwrap();
    let cands = w.g.engage_defenders(to, p0, &big, WarTest::AtWar);
    assert_eq!(cands.len(), 4);
    let s = w.g.engage_defense_stance(to, &cands);
    assert_eq!((s.stance, s.army), (mobile, Some(d_big)));
    assert_eq!(w.g.engage_defense_stance(to, &[]).stance, default);

    // A city: its garrison's stance, even with field armies in it; an attack from a city by a
    // garrison unit proposes the garrison's.
    let c = w.city(p1, to, 0);
    w.g.city_set_garrison_stance(p1, c, ranged).unwrap();
    let s = w.g.engage_defense_stance(to, &cands);
    assert_eq!((s.stance, s.army, s.garrison_of), (ranged, None, Some(c)));
    let guard = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.player_army_sync(p1);
    assert_eq!(w.g.unit_garrison_city(guard), Some(c));
    let s = w.g.engage_attack_stance(p1, to, w.g.units[guard].group);
    assert_eq!((s.stance, s.garrison_of), (ranged, Some(c)));
}
