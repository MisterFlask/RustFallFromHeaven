//! Engagement combat's slots and who can fight (GAMEOPTION_ENGAGEMENTS, phase 6 step 5;
//! `engagement/slots.rs`, `engagement/pick.rs`): each side's slots by plot, city defense,
//! bombardment, the minimum and the river (Q9); the attack count with Blitz and bombards, the
//! combat limit and marching units in `unit_can_engage` and the gates it shares with
//! `unit_can_move_into`; the defender's candidates.

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
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
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

    /// A city of `owner` on `at` whose defense modifier (no bombardment) is `pct`.
    fn city(&mut self, owner: PlayerId, at: PlotIdx, pct: i32) -> CityId {
        let (x, y) = self.g.map.xy(at);
        let c = self.g.player_init_city(owner, x, y, true, true);
        let total = self.g.city_total_defense(c, false);
        self.g.cities[c].building_defense += pct - total;
        assert_eq!(self.g.city_defense_modifier(c, false), pct);
        c
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
