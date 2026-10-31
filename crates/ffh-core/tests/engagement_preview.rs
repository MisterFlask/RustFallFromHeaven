//! The attack preview (docs/ENGAGEMENTS.md, "The attack preview"): a function of its inputs that
//! leaves the game's RNG and state alone, its fields (both formations, the ground, the forecast),
//! the forecast frozen into the battle log and the battle fought as planned, and its time
//! (`--ignored`).

use std::time::Instant;

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::{EngagePreview, HitCtx, Place, PreviewInput, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 (at war when `war`), `from` and `to` two unowned flat grassland
/// plots side by side with no feature, improvement or river crossing.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world_with(war: bool) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let (p0, p1) = (PlayerId(0), PlayerId(1));
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
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        (0..n).map(|_| self.unit(owner, name, at)).collect()
    }

    fn forest(&mut self, at: PlotIdx) {
        self.g.map.plots[at].feature_type = Some(self.g.d.t::<FeatureType>("FEATURE_FOREST"));
    }

    fn input(&self) -> PreviewInput<'static> {
        PreviewInput::new(self.p0, self.from, self.to)
    }
}

fn state(g: &Game) -> (Rng, Rng, Vec<u8>) {
    (g.soren_rand.clone(), g.map_rand.clone(), g.save())
}

/// A mixed attack on a mixed defense with a reserve: the preview is the same twice, the game's
/// RNGs and state are untouched, and its forecast holds together.
#[test]
fn the_preview_is_a_function_of_its_inputs_and_changes_nothing() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.forest(to);
    w.units(p0, "UNIT_AXEMAN", from, 5);
    w.units(p0, "UNIT_HORSEMAN", from, 2);
    w.units(p0, "UNIT_CATAPULT", from, 2);
    w.units(p0, "UNIT_ARCHER", from, 2);
    w.units(p1, "UNIT_ARCHER", to, 4);
    w.units(p1, "UNIT_AXEMAN", to, 4);
    w.units(p1, "UNIT_WARRIOR", to, 2);
    let before = state(&w.g);
    let inp = w.input();
    let a = w.g.engage_preview(&inp).expect("a preview");
    let b = w.g.engage_preview(&inp).unwrap();
    assert_eq!(a, b);
    assert!(state(&w.g) == before, "the preview changed the game");
    // What it holds: the forest's six slots and three lanes a side, its rules, both formations.
    assert_eq!((a.slots.att(), a.slots.def(), a.lanes), (6, 6, [3, 3]));
    assert!(a.ground.forest && !a.ground.flanking());
    assert_eq!(a.rules, a.ground.rules(&ffh_core::engagement::formation::Params::from_defines(&w.g.d.defines)));
    assert_eq!(a.formation, a.auto, "no formation given: the auto formation");
    assert_eq!(a.formation.len(), 6);
    assert_eq!(a.def_formation, w.g.engage_auto_formation(&a.defenders, Side::Defender, &a.slots, &HitCtx::new(from, to)));
    assert_eq!(a.def_formation.len() + a.def_reserve.len(), a.defenders.len());
    assert!(a.reserve.iter().all(|u| a.pool.contains(u) && !a.formation.iter().any(|p| p.unit == *u)));
    assert_eq!(a.declare_war, None);
    let f = &a.forecast;
    assert_eq!(f.runs, w.g.d.defines.engagement_preview_runs);
    assert_eq!(f.per.len(), a.formation.len() + a.def_formation.len());
    assert!(f.per.iter().all(|x| (0..=100).contains(&x.fall_pct) && (0..=100).contains(&x.escape_pct) && x.fall_pct + x.escape_pct <= 100));
    assert!(f.falls_permille <= f.wiped_permille && f.wiped_permille <= 1000 && f.repulsed_permille <= 1000);
    assert!(f.wiped_permille + f.repulsed_permille <= 1000, "a battle ends when one line is gone");
    assert_eq!(f.falls_permille, 0, "a reserve of four: the plot never falls in one battle");
    let side_lost = |s: Side| f.per.iter().filter(|x| x.side == s).map(|x| x.fall_pct as i64).sum::<i64>();
    for (i, s) in [Side::Attacker, Side::Defender].into_iter().enumerate() {
        // The per-fighter percentages (rounded down) add up to at most the side's expected losses.
        assert!(side_lost(s) <= f.lost_x100[i] as i64, "{s:?}: {} > {}", side_lost(s), f.lost_x100[i]);
        assert!(f.lost_x100[i] + f.escaped_x100[i] <= 100 * f.per.iter().filter(|x| x.side == s).count() as i32);
    }
    // A different formation is a different plan: the axemen behind, the archers in front.
    let mut other = a.formation.clone();
    let (i, j) = (0, other.len() - 1);
    let (ci, cj) = ((other[i].lane, other[i].row), (other[j].lane, other[j].row));
    (other[i].lane, other[i].row) = cj;
    (other[j].lane, other[j].row) = ci;
    let c = w.g.engage_preview(&PreviewInput { formation: Some(&other), ..inp }).unwrap();
    assert_eq!(c.formation, other);
    assert_eq!(c.auto, a.auto);
    assert!(state(&w.g) == before);
    // The preview's RNG is its own (seeded from the plan): the game's RNGs elsewhere, the same
    // numbers.
    w.g.soren_rand = Rng::new(0x5eed_0001);
    w.g.map_rand = Rng::new(0x5eed_0002);
    assert_eq!(w.g.engage_preview(&inp).unwrap(), a, "the preview depends on the game's RNG");
}

/// The defender's formation as the attacker sees it, a refused formation, a declare-war preview,
/// the plot that falls when no defender is left.
#[test]
fn the_preview_fields() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.forest(to);
    let att = w.units(p0, "UNIT_AXEMAN", from, 4);
    let defs: Vec<UnitId> = ["UNIT_ARCHER", "UNIT_AXEMAN", "UNIT_HORSEMAN", "UNIT_WARRIOR", "UNIT_CATAPULT"].iter().flat_map(|n| w.units(p1, n, to, 2)).collect();
    // One of them hidden from the attacker: not a defender it plans against.
    let hidden = defs[7];
    w.g.unit_set_has_promotion(hidden, w.g.d.t::<PromotionType>("PROMOTION_HIDDEN"), true);
    let a = w.g.engage_preview(&w.input()).unwrap();
    assert!(!a.defenders.contains(&hidden));
    assert_eq!(a.defenders.len(), defs.len() - 1);
    assert!(a.defenders.len() as i32 > a.slots.def());
    assert_eq!(a.def_formation.len() as i32, a.slots.def());
    // Formations the rules refuse.
    let twice = [Place { unit: att[0], lane: 0, row: 0 }, Place { unit: att[0], lane: 1, row: 0 }];
    assert!(w.g.engage_preview(&PreviewInput { formation: Some(&twice), ..w.input() }).is_err());
    let theirs = [Place { unit: defs[0], lane: 0, row: 0 }];
    assert!(w.g.engage_preview(&PreviewInput { formation: Some(&theirs), ..w.input() }).is_err());
    let off = [Place { unit: att[0], lane: 3, row: 0 }];
    assert!(w.g.engage_preview(&PreviewInput { formation: Some(&off), ..w.input() }).is_err(), "three lanes in a forest");
    // Fewer defenders than slots and a crushing attack: the plot falls in most battles.
    let mut w2 = world();
    let strong = w2.units(w2.p0, "UNIT_AXEMAN", w2.from, 8);
    for &u in &strong {
        w2.g.units[u].extra_combat_percent = 500;
    }
    for u in w2.units(w2.p1, "UNIT_WARRIOR", w2.to, 2) {
        w2.g.units[u].damage = 80;
    }
    let b = w2.g.engage_preview(&w2.input()).unwrap();
    assert!(b.def_reserve.is_empty());
    assert!(b.forecast.falls_permille > 900 && b.forecast.falls_permille == b.forecast.wiped_permille, "{:?}", b.forecast);
    assert_eq!(b.forecast.lost_x100[1], 200 - b.forecast.escaped_x100[1] - 0, "both warriors fall or escape: {:?}", b.forecast);
    // At peace: nothing to fight unless the move declares war.
    let mut w3 = world_with(false);
    w3.units(w3.p0, "UNIT_AXEMAN", w3.from, 3);
    w3.units(w3.p1, "UNIT_WARRIOR", w3.to, 2);
    assert!(w3.g.engage_preview(&w3.input()).is_err());
    let t1 = w3.g.players[w3.p1].team;
    let c = w3.g.engage_preview(&PreviewInput { war: WarTest::DeclareWar(t1), ..w3.input() }).unwrap();
    assert_eq!(c.declare_war, Some(t1));
    assert_eq!(c.def_formation.len(), 2);
    let _ = (p0, from);
}

/// A move that declares war previews the battle its attack fights once the war is declared: the
/// archers' defensive strikes are in its opening. So its forecast is that of the preview made
/// after the declaration, and the attack freezes the numbers the player saw.
#[test]
fn a_declare_war_preview_is_the_battle_after_the_declaration() {
    let mut w = world_with(false);
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.forest(to);
    w.g.players[p0].human = true;
    let cats = w.units(p0, "UNIT_CATAPULT", from, 2);
    w.units(p0, "UNIT_AXEMAN", from, 6);
    w.units(p0, "UNIT_HORSEMAN", from, 2);
    w.units(p1, "UNIT_ARCHER", to, 4);
    w.units(p1, "UNIT_AXEMAN", to, 3);
    let (t0, t1) = (w.g.players[p0].team, w.g.players[p1].team);
    let declare = PreviewInput { war: WarTest::DeclareWar(t1), ..w.input() };
    let proposed = w.g.engage_preview(&declare).expect("a declare-war preview");
    // The plan: four of the auto formation's other units where it put them, and both catapults
    // in free cells of the rear row.
    let mut formation: Vec<Place> = proposed.auto.iter().copied().filter(|p| !cats.contains(&p.unit)).take(4).collect();
    for &c in &cats {
        let lane = (0..proposed.lanes[0]).find(|&l| !formation.iter().any(|p| p.row == 2 && p.lane == l)).expect("a free rear cell");
        formation.push(Place { unit: c, lane, row: 2 });
    }
    let with_cats = PreviewInput { formation: Some(&formation), ..declare };
    let before = w.g.engage_preview(&with_cats).unwrap();
    assert_eq!(before.declare_war, Some(t1));
    assert_eq!(before.formation, formation);

    // Declared: the same numbers.
    w.g.team_declare_war(t0, t1, false, None);
    let at_war = PreviewInput { war: WarTest::AtWar, ..declare };
    assert_eq!(w.g.engage_preview(&at_war).unwrap().auto, proposed.auto, "the auto formation");
    let after = w.g.engage_preview(&PreviewInput { formation: Some(&formation), ..at_war }).unwrap();
    assert_eq!(EngagePreview { declare_war: None, ..before.clone() }, after);

    // The attack freezes the plan the player saw before declaring.
    let grp = w.g.units[cats[0]].group.unwrap();
    let next = w.g.battle_report_next;
    let (x, y) = w.g.map.xy(to);
    w.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(formation.clone())).expect("the attack");
    let r = w.g.battle_reports.iter().find(|r| r.id >= next).expect("an engagement");
    let fc = r.log.as_ref().unwrap().forecast.as_ref().expect("the plan is frozen");
    let f = &before.forecast;
    assert_eq!((fc.wiped_permille, fc.falls_permille, fc.repulsed_permille, fc.lost_x100), (f.wiped_permille, f.falls_permille, f.repulsed_permille, f.lost_x100));
}

/// A human's attack freezes the preview it saw into its battle log, and the battle fought is the
/// preview's: the same formations, cell for cell. An AI's attack freezes none.
#[test]
fn the_battle_log_keeps_the_plan() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p0].human = true;
    let att = w.units(p0, "UNIT_AXEMAN", from, 5);
    w.units(p0, "UNIT_ARCHER", from, 2);
    w.units(p1, "UNIT_AXEMAN", to, 3);
    w.units(p1, "UNIT_ARCHER", to, 2);
    let inp = w.input();
    let pv = w.g.engage_preview(&inp).unwrap();
    let grp = w.g.units[att[0]].group.unwrap();
    let before = w.g.battle_report_next;
    let (x, y) = w.g.map.xy(to);
    // The plan's Attack: the `engage` command's formation.
    w.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(pv.formation.clone())).expect("the attack");
    let r = w.g.battle_reports.iter().find(|r| r.id >= before).expect("an engagement");
    let log = r.log.as_ref().expect("a human's battle is logged");
    assert!(log.ordered[0]);
    let fc = log.forecast.as_ref().expect("the plan is frozen");
    assert_eq!(fc.runs, pv.forecast.runs);
    assert_eq!((fc.wiped_permille, fc.falls_permille, fc.repulsed_permille, fc.lost_x100), (pv.forecast.wiped_permille, pv.forecast.falls_permille, pv.forecast.repulsed_permille, pv.forecast.lost_x100));
    assert_eq!(fc.per.len(), pv.forecast.per.len());
    for (frozen, seen) in fc.per.iter().zip(&pv.forecast.per) {
        assert_eq!(log.fighters[frozen.b as usize].unit, seen.unit);
        assert_eq!((frozen.fall_pct, frozen.escape_pct), (seen.fall_pct, seen.escape_pct));
    }
    for p in pv.formation.iter().chain(&pv.def_formation) {
        let lf = log.fighters.iter().find(|lf| lf.unit == p.unit).expect("in the battle");
        assert_eq!(lf.place, Some((p.lane, p.row)), "the battle stands as the plan did");
    }
    assert_eq!(log.lanes, pv.lanes);
    assert_eq!(log.rules, pv.rules);
    // An AI's attack on a human: logged, no plan frozen.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    let att = w.units(p0, "UNIT_AXEMAN", from, 4);
    w.units(p1, "UNIT_ARCHER", to, 2);
    let grp = w.g.units[att[0]].group.unwrap();
    let (x, y) = w.g.map.xy(to);
    w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    let r = w.g.battle_reports.last().unwrap();
    assert!(r.log.as_ref().expect("a human's battle").forecast.is_none());
}

/// The time of a preview, 6 v 6, 8 v 8 and 10 v 10 (the attacker's slots cut to the size by a
/// city's defense, the defender's at ten so all of its units fight), and of the auto formation.
/// `cargo test --release -p ffh-core --test engagement_preview -- --ignored --nocapture
/// preview_timing`.
#[test]
#[ignore]
fn preview_timing() {
    let time = |reps: u32, f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..reps {
            f();
        }
        t.elapsed().as_secs_f64() * 1000.0 / reps as f64
    };
    for size in [6usize, 8, 10] {
        let mut w = world();
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        let (x, y) = w.g.map.xy(to);
        let c = w.g.player_init_city(p1, x, y, true, true);
        let total = w.g.city_total_defense(c, false);
        w.g.cities[c].building_defense += (10 - size as i32) * 25 - total;
        let mix = ["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_SWORDSMAN"];
        let pool: Vec<UnitId> = (0..size + 3).map(|i| w.unit(p0, mix[i % mix.len()], from)).collect();
        for i in 0..size {
            w.unit(p1, mix[(i + 1) % mix.len()], to);
        }
        let ctx = HitCtx::new(from, to);
        let slots = w.g.engage_slots(&ctx);
        assert_eq!((slots.att() as usize, slots.def()), (size, 10));
        let inp = w.input();
        let g = &w.g;
        let plan_ms = time(5, &mut || {
            g.engage_preview(&inp).unwrap();
        });
        let auto_ms = time(50, &mut || {
            g.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
        });
        println!("{size} v {size}: the plan {plan_ms:.1} ms ({} runs), the auto formation {auto_ms:.3} ms", g.d.defines.engagement_preview_runs);
    }
}
