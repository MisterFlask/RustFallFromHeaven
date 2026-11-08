//! Engagement combat's slots, who can fight and where they stand
//! (`engagement/slots.rs`, `engagement/pick.rs`, `engagement/arrange.rs`): each side's slots by
//! plot, city defense, bombardment, the minimum and the river (Q9), and its lanes; the attack
//! count with Blitz and bombards, the combat limit and marching units in `unit_can_engage` and the
//! gates it shares with `unit_can_move_into`; the defender's candidates; the rules' auto formation
//! (one of each role first, healthy and strong first, home rows, large units, the river rule) and
//! an ordered formation's check.

use ffh_core::army::{March, RecruitDest};
use ffh_core::engagement::report::{SlotChange, SlotSource};
use ffh_core::engagement::{HitCtx, Place, Side, Slots, WarTest};
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

fn world_with(war: bool) -> World {
    world_of(war, 2, "WORLDSIZE_DUEL")
}

/// [`world_with`] with `civs` players on a map of size `size`; players 0 and 1 at war if `war`.
fn world_of(war: bool, civs: usize, size: &str) -> World {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, civs, size, 5);
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
    world_with(true)
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

    /// The rules' auto formation of `pool` on side `side`.
    fn auto(&self, pool: &[UnitId], side: Side) -> Vec<Place> {
        self.g.engage_auto_formation(pool, side, &self.slots(), &self.ctx())
    }

    /// The units of the auto formation of `pool` on side `side`.
    fn pick(&self, pool: &[UnitId], side: Side) -> Vec<UnitId> {
        self.auto(pool, side).iter().map(|p| p.unit).collect()
    }

    /// A city of `owner` on `at` whose defense modifier (no bombardment) is `pct`.
    fn city(&mut self, owner: PlayerId, at: PlotIdx, pct: i32) -> CityId {
        let (x, y) = self.g.map.xy(at);
        let c = self.g.player_init_city(owner, x, y, true, true);
        // Standard Walls (two Sentinels) whatever the city's building tiers rolled
        // (docs/SUITABILITY.md).
        let walls = self.g.d.t::<BuildingType>("BUILDING_WALLS");
        let class = self.g.d.buildings[walls].building_class_type.unwrap();
        self.g.cities[c].building_tiers[class] = BuildingTierType::STANDARD;
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

    /// A unit's kind as the auto formation sorts it: `ANTI_FLANK` for an anti-flanking striker,
    /// else the token of what it does from the row it prefers (`STRIKE`, `SHOOT`, ...).
    fn kind(&self, u: UnitId) -> String {
        if self.g.unit_engage_anti_flank(u) {
            "ANTI_FLANK".to_string()
        } else {
            self.g.unit_engage_kind(u).token().to_string()
        }
    }

    fn kinds(&self, units: &[UnitId], kind: &str) -> usize {
        units.iter().filter(|&&u| self.kind(u) == kind).count()
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
fn a_walled_citys_sentinels_cost_both_sides_slots_and_the_minimum_holds() {
    // docs/SIEGES.md: a city's defence costs no slots; each standing Sentinel costs both sides one.
    let mut w = world();
    let (p1, to) = (w.p1, w.to);
    let grass = w.g.d.t::<TerrainType>("TERRAIN_GRASS");
    let c = w.city(p1, to, 60);
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.def(), s.sentinels), (10, 10, 0), "no walls: the defence costs nothing");

    // Walls: two Sentinels.
    let walls = w.g.d.t::<BuildingType>("BUILDING_WALLS");
    w.g.city_set_num_real_building(c, walls, 1);
    assert_eq!(w.g.city_sentinel_count(c), 2);
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.def(), s.sentinels), (8, 8, 2));
    let want = vec![(SlotSource::Terrain(grass), 2), (SlotSource::Sentinels(2), -2)];
    assert_eq!((src(&s, Side::Attacker), src(&s, Side::Defender)), (want.clone(), want));
    // The defender's grid holds its soldiers' slots and its Sentinels.
    assert_eq!(w.g.engage_lanes(&s), [4, 4]);
    assert_eq!(w.g.engage_sentinel_cells(&s), vec![(1, 0), (2, 0)]);

    // One breached: one slot back to each side.
    w.g.city_set_sentinel_damage(c, 0, 100);
    let s = w.slots();
    assert_eq!((s.att(), s.def(), s.sentinels), (9, 9, 1));
    w.g.city_set_sentinel_damage(c, 0, 0);

    // Forested hills: 4 a side; the Sentinels would leave 2: raised to SLOTS_MIN (3).
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.def()), (3, 3));
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
    let auto = w.auto(&pool, Side::Attacker);
    let pick: Vec<UnitId> = auto.iter().map(|p| p.unit).collect();
    assert_eq!((pick.len(), w.count(&pick, "UNIT_AXEMAN"), w.count(&pick, "UNIT_LIZARDMAN")), (10, 8, 2));
    assert_eq!(w.g.engage_validate_formation(&pool, &auto, &s), Ok(()));
    // The defender has no river slots.
    assert_eq!(w.pick(&axemen, Side::Defender).len(), 10);

    // Near the minimum the river takes only what is above it: forested hills, 4 - 1.
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    let s = w.slots();
    check_sums(&s);
    assert_eq!((s.att(), s.att_river()), (3, 1));
}

#[test]
fn the_lanes_follow_the_slots() {
    let mut w = world();
    let to = w.to;
    // Open flat land: 10 a side, 4 lanes; a forest: 6, 3 lanes; forested hills: 4, 2 lanes.
    assert_eq!(w.g.engage_lanes(&w.slots()), [4, 4]);
    w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
    assert_eq!(w.g.engage_lanes(&w.slots()), [3, 3]);
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    assert_eq!(w.g.engage_lanes(&w.slots()), [2, 2]);
    // A walled city's two Sentinels take a slot of each side and stand in the defender's grid
    // (docs/SIEGES.md): on hills 6 - 2 = 4 a side, the defender's 4 + 2 cells: 2 and 3 lanes.
    w.g.map.plots[to].plot_type = PlotType::PLOT_LAND;
    let p1 = w.p1;
    let c = w.city(p1, to, 75);
    w.g.city_set_num_real_building(c, w.g.d.t::<BuildingType>("BUILDING_WALLS"), 1);
    w.g.map.plots[to].feature_type = None;
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    let s = w.slots();
    assert_eq!((s.max(Side::Attacker), s.def(), s.sentinels), (4, 4, 2));
    assert_eq!(w.g.engage_lanes(&s), [2, 3]);
}

#[test]
fn formations_are_validated() {
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    w.g.map.plots[from].river_crossing[DirectionType::DIRECTION_EAST.idx()] = true;
    let s = w.slots();
    assert_eq!((s.att(), s.att_river(), w.g.engage_lanes(&s)[0]), (8, 2, 4));
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 10);
    let lizard = w.unit(p0, "UNIT_LIZARDMAN", from);
    let giant = w.unit(p0, "UNIT_HILL_GIANT", from);
    assert!(w.g.unit_engage_large(giant) && w.g.unit_engage_size(giant) == 2);
    let outsider = w.unit(p0, "UNIT_AXEMAN", w.to);
    let pool: Vec<UnitId> = axemen.iter().copied().chain([lizard, giant]).collect();
    let at = |u: UnitId, lane: u8, row: u8| Place { unit: u, lane, row };
    // The first n axemen in the front row, then the middle, then the rear (4 lanes).
    let rows = |n: usize| -> Vec<Place> { (0..n).map(|i| at(axemen[i], (i % 4) as u8, (i / 4) as u8)).collect() };
    let v = |f: &[Place]| w.g.engage_validate_formation(&pool, f, &s);
    assert!(v(&[]).is_err(), "empty");
    assert_eq!(v(&rows(8)), Ok(()));
    assert!(v(&[at(axemen[0], 0, 0), at(axemen[0], 1, 0)]).is_err(), "twice");
    assert!(v(&[at(axemen[0], 0, 0), at(outsider, 1, 0)]).is_err(), "not in the pool");
    assert!(v(&[at(axemen[0], 4, 0)]).is_err(), "outside the lanes");
    assert!(v(&[at(axemen[0], 0, 3)]).is_err(), "outside the rows");
    assert!(v(&[at(axemen[0], 1, 0), at(axemen[1], 1, 0)]).is_err(), "two in one cell");
    // A large unit fills its lane and the next: not at the last lane, not over another unit.
    assert_eq!(v(&[at(giant, 2, 0), at(axemen[0], 1, 0)]), Ok(()));
    assert!(v(&[at(giant, 3, 0)]).is_err(), "a large unit needs two lanes");
    assert!(v(&[at(giant, 1, 0), at(axemen[0], 2, 0)]).is_err(), "its second cell is taken");
    // Nine without bRiver in eight ordinary slots; the river slot to the Lizardman is fine; a
    // large unit counts two.
    assert!(v(&rows(9)).is_err(), "nine without bRiver in eight ordinary slots");
    let mut f = rows(8);
    f.push(at(lizard, 0, 2));
    assert_eq!(v(&f), Ok(()));
    let mut f = rows(7);
    f.push(at(giant, 0, 2));
    assert!(v(&f).is_err(), "seven and a giant fill nine ordinary slots");
    let mut f = rows(6);
    f.push(at(giant, 2, 1));
    assert_eq!(v(&f), Ok(()));
    let mut more: Vec<UnitId> = w.units(p0, "UNIT_LIZARDMAN", from, 2);
    more.extend(pool.iter().copied());
    let mut f = rows(8);
    f.extend([at(more[0], 0, 2), at(more[1], 1, 2), at(lizard, 2, 2)]);
    assert!(w.g.engage_validate_formation(&more, &f, &s).is_err(), "more than the slots");
}

#[test]
fn the_auto_formation_puts_recon_units_in_the_middle_row() {
    // Recon units fight from any row and prefer the middle (2026-11-06): with room, a Hunter
    // stands in the middle row, where it shoots; the Axeman in the front.
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let hunter = w.unit(p0, "UNIT_HUNTER", from);
    let axeman = w.unit(p0, "UNIT_AXEMAN", from);
    let auto = w.auto(&[hunter, axeman], Side::Attacker);
    let row_of = |u: UnitId| auto.iter().find(|p| p.unit == u).map(|p| p.row);
    assert_eq!((row_of(hunter), row_of(axeman)), (Some(1), Some(0)));
    assert_eq!(w.kind(hunter), "SHOOT");
    // With Formation it is not anti-flanking: it does not strike from the row it prefers.
    w.promote(hunter, "PROMOTION_FORMATION");
    assert!(!w.g.unit_engage_anti_flank(hunter));
}

#[test]
fn the_auto_formation_takes_one_of_each_kind_first_and_puts_each_in_the_row_it_prefers() {
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
    assert_eq!(w.kind(screen), "ANTI_FLANK");
    assert_eq!((w.slots().att(), w.g.engage_lanes(&w.slots())), (10, [4, 4]));

    let kinds = |pick: &[UnitId]| ["STRIKE", "ANTI_FLANK", "SHOOT", "CHARGE", "STAB", "SPELL"].map(|k| w.kinds(pick, k));
    // One of each kind first (anti-flank, strike, shoot, charge, spell, stab: six), then the
    // rest in the kinds' order: four more strikers.
    let auto = w.auto(&pool, Side::Attacker);
    let pick: Vec<UnitId> = auto.iter().map(|p| p.unit).collect();
    assert_eq!(kinds(&pick), [5, 1, 1, 1, 1, 1]);
    assert_eq!(w.g.engage_validate_formation(&pool, &auto, &w.slots()), Ok(()));
    // The rows they prefer: the anti-flanker and the horseman in the front row; the archer and
    // the assassin in the middle; the adept in the rear; the front's four cells full, the other
    // strikers in the next row of their rank order with room.
    let row_of = |u: UnitId| auto.iter().find(|p| p.unit == u).map(|p| p.row);
    let of_kind = |k: &str| pick.iter().copied().filter(|&u| w.kind(u) == k).collect::<Vec<_>>();
    assert_eq!(row_of(screen), Some(0));
    assert_eq!(of_kind("CHARGE").iter().map(|&u| row_of(u)).collect::<Vec<_>>(), [Some(0)]);
    assert_eq!(of_kind("SHOOT").iter().map(|&u| row_of(u)).collect::<Vec<_>>(), [Some(1)]);
    assert_eq!(of_kind("STAB").iter().map(|&u| row_of(u)).collect::<Vec<_>>(), [Some(1)]);
    assert_eq!(of_kind("SPELL").iter().map(|&u| row_of(u)).collect::<Vec<_>>(), [Some(2)]);
    assert_eq!(auto.iter().filter(|p| p.row == 0).count(), 4, "the front row is full");
    // Centre lanes first: the anti-flanker, picked first, stands in lane 1 (lanes 1 and 2 are the
    // centre; ties go to the lower).
    assert_eq!(auto.iter().find(|p| p.unit == screen).map(|p| p.lane), Some(1));

    // Deterministic, the same for both sides, no unit twice; fewer candidates than slots: all.
    for side in [Side::Attacker, Side::Defender] {
        let a = w.auto(&pool, side);
        assert_eq!(a, w.auto(&pool, side));
        assert_eq!(a.len(), 10);
        let mut units: Vec<UnitId> = a.iter().map(|p| p.unit).collect();
        units.sort();
        units.dedup();
        assert_eq!(units.len(), 10);
    }
    assert_eq!(w.pick(&pool[..8], Side::Attacker).len(), 8);

    // A large unit counts two slots and is placed first: a Hill Giant takes the centre of the
    // front row.
    let giant = w.unit(p0, "UNIT_HILL_GIANT", from);
    let mut pool2 = pool.clone();
    pool2.push(giant);
    let auto = w.auto(&pool2, Side::Attacker);
    let g = auto.iter().find(|p| p.unit == giant).expect("the giant is strong enough to be taken");
    assert_eq!((g.lane, g.row), (1, 0));
    assert_eq!(auto.iter().map(|p| w.g.unit_engage_size(p.unit)).sum::<i32>(), 10);
    assert_eq!(w.g.engage_validate_formation(&pool2, &auto, &w.slots()), Ok(()));
}

#[test]
fn a_wounded_unit_waits_behind_the_healthy_ones_of_its_role() {
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
    let pick = w.pick(&pool, Side::Attacker);
    assert!(!pick.contains(&hurt), "five healthy Axemen and the archer");
    assert!(pick.contains(&archer), "one of each role first");
    assert_eq!(w.count(&pick, "UNIT_AXEMAN"), 5);
    // At exactly half its HP it counts as healthy, and its strength makes it its role's first.
    w.g.units[hurt].damage = 50;
    assert!(w.pick(&pool, Side::Attacker).contains(&hurt));
    // With fewer healthy Axemen than the slots, it is taken after them.
    w.g.units[hurt].damage = 60;
    let pool: Vec<UnitId> = axemen[..5].iter().copied().chain([archer]).collect();
    let pick = w.pick(&pool, Side::Attacker);
    assert!(pick.contains(&hurt) && pick.contains(&archer));
    // Heroes and veterans are not held back: the strongest is taken first.
    let vet = axemen[1];
    w.promote(vet, "PROMOTION_COMBAT1");
    w.promote(vet, "PROMOTION_COMBAT2");
    w.promote(vet, "PROMOTION_HERO");
    w.g.units[vet].level = 5;
    assert!(w.g.unit_is_hero(vet));
    let pool: Vec<UnitId> = axemen.iter().copied().chain([archer]).collect();
    assert!(w.pick(&pool, Side::Attacker).contains(&vet));
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
/// a Marksman's (bTargetWeakestUnit) picks the capped one, so in the duel it could not attack;
/// under engagements it attacks while any defender is below its limit.
#[test]
fn a_siege_unit_attacks_while_any_defender_is_below_its_limit_whoever_the_best_defender_is() {
    let mut w = world_with(true);
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
    assert!(w.g.unit_can_move_into(cat, to, true, false, false));
    assert!(w.g.unit_can_engage(cat, to, WarTest::AtWar));
    w.g.units[warrior].damage = limit;
    assert!(!w.g.unit_can_move_into(cat, to, true, false, false), "every defender at the cap");
    assert!(!w.g.unit_can_engage(cat, to, WarTest::AtWar));
}

/// As today: a siege unit attacks no plot from the water (a non-combatant on it is enough to
/// refuse it), and from land it needs a unit below its limit even when the plot holds only
/// non-combatants (the attack captures them).
#[test]
fn a_siege_unit_does_not_attack_from_the_water_even_onto_workers() {
    let mut w = world_with(true);
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
    assert!(w.g.unit_can_engage(axeman, coast, WarTest::AtWar));
    assert!(!w.g.unit_can_engage(cat, coast, WarTest::AtWar));
    assert!(w.g.engage_defenders(coast, p0, &[axeman], WarTest::AtWar).is_empty(), "a worker does not defend");
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
    let mut w = world_with(false);
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
    let mut w = world_of(false, 6, "WORLDSIZE_STANDARD");
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

