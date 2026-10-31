//! The attack preview (phase 6 step 7, plan 15 item 2; docs/ENGAGEMENTS.md, "The preview"): a
//! function of its inputs that leaves the game's RNG and state alone, its fields, the forecast
//! frozen into the battle log, the sampled win chance against the moment estimator on a battery
//! of real unit battles (the calibration check of plan 11.4), and its time (`--ignored`).

use std::time::Instant;

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::{AltKind, ForecastSource};
use ffh_core::engagement::{Attackers, DefPick, EstimateInput, HitCtx, PreviewInput, Side, WarTest};
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

    /// A unit of the attacker's group (its army's).
    fn launcher(&self) -> Option<GroupId> {
        self.g.map.plots[self.from].units.first().and_then(|&u| self.g.units[u].group)
    }
}

fn state(g: &Game) -> (Rng, Rng, Vec<u8>) {
    (g.soren_rand.clone(), g.map_rand.clone(), g.save())
}

/// A mixed attack on a mixed defense with a reserve: the preview and its What-if list are the
/// same twice, and the game's RNGs and state are untouched.
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
    let alts = w.g.engage_preview_alternatives(&inp).unwrap();
    assert_eq!(alts, w.g.engage_preview_alternatives(&inp).unwrap());
    assert!(state(&w.g) == before, "the preview changed the game");
    // What it holds.
    let f = &a.forecast;
    assert_eq!(f.runs, w.g.d.defines.engagement_preview_runs);
    assert_eq!(f.per.len(), a.force.len() + a.assumed_pick.len());
    assert!(f.per.iter().all(|x| (0..=100).contains(&x.fall_pct) && (0..=100).contains(&x.withdraw_pct) && x.fall_pct + x.withdraw_pct <= 100));
    assert!(f.rounds.0 <= f.rounds.1 && f.rounds.1 <= f.rounds.2);
    assert!(f.win_permille + f.draw_permille <= 1000);
    assert_eq!(f.curve[0].len(), f.curve[1].len());
    assert!(!f.curve[0].is_empty());
    assert!(f.curve.iter().all(|c| c.windows(2).all(|x| x[1] <= x[0])), "HP never rises: {:?}", f.curve);
    assert_eq!(a.force, a.auto_pick);
    assert!(a.reserve.iter().all(|u| a.pool.contains(u) && !a.force.contains(u)));
    assert_eq!(a.declare_war, None);
    // The What-if list: the stance's own pick, swaps, the other stances' picks.
    assert!(alts.iter().any(|x| x.kind == AltKind::StancePick && x.stance == a.stance));
    assert!(alts.iter().any(|x| x.kind == AltKind::Swap), "{alts:?}");
    assert_eq!(alts.iter().filter(|x| x.kind == AltKind::OtherStance).count(), w.g.d.stances.len() - 1);
    for alt in &alts {
        assert!(alt.force.iter().all(|u| a.pool.contains(u)));
        assert!(w.g.engage_validate_force(&a.pool, &alt.force, &a.slots).is_ok(), "{alt:?}");
    }
    // A different force is a different plan.
    let mut other = a.force.clone();
    let swap_in = a.reserve[0];
    other[0] = swap_in;
    let c = w.g.engage_preview(&PreviewInput { force: Some(&other), ..inp }).unwrap();
    assert_eq!(c.force, other);
    assert_eq!(c.auto_pick, a.auto_pick);
    assert!(state(&w.g) == before);
    // The preview's RNG is its own (seeded from the plan): the game's RNGs elsewhere, the same
    // numbers.
    w.g.soren_rand = Rng::new(0x5eed_0001);
    w.g.map_rand = Rng::new(0x5eed_0002);
    assert_eq!(w.g.engage_preview(&inp).unwrap(), a, "the preview depends on the game's RNG");
    assert_eq!(w.g.engage_preview_alternatives(&inp).unwrap(), alts, "the What-if list depends on the game's RNG");
}

/// The defender's pick as the attacker knows it, a refused force, a declare-war preview.
#[test]
fn the_preview_fields() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.forest(to);
    w.units(p0, "UNIT_AXEMAN", from, 4);
    let defs: Vec<UnitId> = ["UNIT_ARCHER", "UNIT_AXEMAN", "UNIT_HORSEMAN", "UNIT_WARRIOR", "UNIT_CATAPULT"]
        .iter()
        .flat_map(|n| w.units(p1, n, to, 2))
        .collect();
    let a = w.g.engage_preview(&w.input()).unwrap();
    assert!(defs.len() as i32 > a.slots.def());
    assert_eq!(a.def_stance, None, "an enemy's stance is unseen");
    assert_eq!(a.def_stance_assumed, w.g.d.default_stance);
    match &a.defender_pick {
        DefPick::Likely { sure, maybe } => {
            assert!(!maybe.is_empty(), "the stances differ on {} candidates", defs.len());
            assert!(sure.iter().all(|u| a.assumed_pick.contains(u)));
            assert!(sure.iter().chain(maybe).all(|u| defs.contains(u)));
        }
        other => panic!("{other:?}"),
    }
    // Fewer candidates than slots: every one fights, whatever the stance.
    let mut w2 = world();
    w2.units(w2.p0, "UNIT_AXEMAN", w2.from, 3);
    let few = w2.units(w2.p1, "UNIT_ARCHER", w2.to, 2);
    let b = w2.g.engage_preview(&w2.input()).unwrap();
    assert_eq!(b.defender_pick, DefPick::Known(b.assumed_pick.clone()));
    assert_eq!(b.assumed_pick.len(), few.len());
    // A force the rules refuse.
    let twice = [a.force[0], a.force[0]];
    assert!(w.g.engage_preview(&PreviewInput { force: Some(&twice), ..w.input() }).is_err());
    let theirs = [defs[0]];
    assert!(w.g.engage_preview(&PreviewInput { force: Some(&theirs), ..w.input() }).is_err());
    // At peace: nothing to fight unless the move declares war.
    let mut w3 = world_with(false);
    w3.units(w3.p0, "UNIT_AXEMAN", w3.from, 3);
    w3.units(w3.p1, "UNIT_WARRIOR", w3.to, 2);
    assert!(w3.g.engage_preview(&w3.input()).is_err());
    let t1 = w3.g.players[w3.p1].team;
    let c = w3.g.engage_preview(&PreviewInput { war: WarTest::DeclareWar(t1), ..w3.input() }).unwrap();
    assert_eq!(c.declare_war, Some(t1));
    assert_eq!(c.assumed_pick.len(), 2);
    let _ = (p0, from);
}

/// A human's attack freezes the preview it saw into its battle log; an AI's, the moment
/// estimate.
#[test]
fn the_battle_log_keeps_the_plan() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p0].human = true;
    let att = w.units(p0, "UNIT_AXEMAN", from, 5);
    w.units(p0, "UNIT_ARCHER", from, 2);
    w.units(p1, "UNIT_AXEMAN", to, 3);
    w.units(p1, "UNIT_ARCHER", to, 2);
    let inp = PreviewInput { launcher: w.launcher(), ..w.input() };
    let pv = w.g.engage_preview(&inp).unwrap();
    let alts = w.g.engage_preview_alternatives(&inp).unwrap();
    let grp = w.g.units[att[0]].group.unwrap();
    let before = w.g.battle_report_next;
    let (x, y) = w.g.map.xy(to);
    // The plan's Attack: the `engage` command's force.
    w.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(pv.force.clone())).expect("the attack");
    let r = w.g.battle_reports.iter().find(|r| r.id >= before).expect("an engagement");
    let log = r.log.as_ref().expect("a human's battle is logged");
    let fc = log.forecast.as_ref().expect("the plan is frozen");
    assert_eq!(fc.source, ForecastSource::Sampled { runs: pv.forecast.runs });
    assert_eq!((fc.win_permille, fc.draw_permille, fc.rounds, fc.losses_if_win_x10), (pv.forecast.win_permille, pv.forecast.draw_permille, pv.forecast.rounds, pv.forecast.losses_if_win_x10));
    assert_eq!(fc.curve, pv.forecast.curve);
    assert_eq!(fc.per.len(), pv.forecast.per.len());
    for (frozen, seen) in fc.per.iter().zip(&pv.forecast.per) {
        assert_eq!(log.fighters[frozen.b as usize].unit, seen.unit);
        assert_eq!((frozen.fall_pct, frozen.withdraw_pct, frozen.death_round), (seen.fall_pct, seen.withdraw_pct, seen.death_round));
    }
    assert_eq!(fc.alternatives.len(), alts.len());
    for (frozen, seen) in fc.alternatives.iter().zip(&alts) {
        let units: Vec<UnitId> = frozen.force.iter().map(|&b| log.fighters[b as usize].unit).collect();
        assert_eq!((frozen.kind, frozen.stance, &units, frozen.win_permille), (seen.kind, seen.stance, &seen.force, seen.win_permille));
    }
    // An AI's attack on a human: the moment estimate.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    let att = w.units(p0, "UNIT_AXEMAN", from, 4);
    w.units(p1, "UNIT_ARCHER", to, 2);
    let grp = w.g.units[att[0]].group.unwrap();
    let (x, y) = w.g.map.xy(to);
    w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    let r = w.g.battle_reports.last().unwrap();
    let fc = r.log.as_ref().unwrap().forecast.as_ref().expect("the AI's plan");
    assert_eq!(fc.source, ForecastSource::Expected);
    assert!(fc.alternatives.is_empty() && fc.per.len() == 6);
}

/// The battery: (attackers, defenders, the defended plot) of real units, near parity.
fn battery() -> Vec<(Vec<(&'static str, usize)>, Vec<(&'static str, usize)>, &'static str)> {
    // Near parity: the same or comparable units on both sides, the attacker one fewer to one
    // more than the defender, on open land, a forest and a 40% city.
    let matchups: [(&[&'static str], &[&'static str]); 8] = [
        (&["UNIT_AXEMAN"], &["UNIT_AXEMAN"]),
        (&["UNIT_WARRIOR"], &["UNIT_WARRIOR"]),
        (&["UNIT_SWORDSMAN"], &["UNIT_AXEMAN"]),
        (&["UNIT_HORSEMAN"], &["UNIT_ARCHER", "UNIT_WARRIOR"]),
        (&["UNIT_AXEMAN", "UNIT_ARCHER"], &["UNIT_AXEMAN", "UNIT_ARCHER"]),
        (&["UNIT_HORSEMAN", "UNIT_AXEMAN", "UNIT_CATAPULT"], &["UNIT_ARCHER", "UNIT_AXEMAN"]),
        (&["UNIT_CHAMPION"], &["UNIT_LONGBOWMAN", "UNIT_PHALANX"]),
        (&["UNIT_KNIGHT", "UNIT_HORSE_ARCHER"], &["UNIT_PHALANX", "UNIT_ARCHER"]),
    ];
    let side = |types: &[&'static str], n: usize| -> Vec<(&'static str, usize)> {
        let mut v: Vec<(&'static str, usize)> = types.iter().map(|&t| (t, 0)).collect();
        for i in 0..n {
            v[i % types.len()].1 += 1;
        }
        v.retain(|e| e.1 > 0);
        v
    };
    let mut out = Vec::new();

    for (att, def) in matchups {
        for nd in [2usize, 3, 5, 8] {
            for na in [nd - 1, nd, nd + 1] {
                for plot in ["flat", "forest", "city"] {
                    out.push((side(att, na), side(def, nd), plot));
                }
            }
        }
    }
    out
}

/// The sampled preview's win chance against the moment estimator's, on real unit battles
/// (plan 11.4's calibration check, before games): the mean absolute difference over all of
/// them and over the contested ones (sampled between 5% and 95%), and the worst, printed with
/// each battle.
#[test]
fn the_preview_against_the_estimator() {
    let (mut sum, mut worst, mut n) = (0i64, 0i32, 0i64);
    let (mut csum, mut cworst, mut cn) = (0i64, 0i32, 0i64);
    let mut lines = Vec::new();
    for (att, def, plot) in battery() {
        let mut w = world();
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        match plot {
            "forest" => w.forest(to),
            "city" => {
                let (x, y) = w.g.map.xy(to);
                let c = w.g.player_init_city(p1, x, y, true, true);
                let total = w.g.city_total_defense(c, false);
                w.g.cities[c].building_defense += 40 - total;
            }
            _ => {}
        }
        for &(name, k) in &att {
            w.units(p0, name, from, k);
        }
        for &(name, k) in &def {
            w.units(p1, name, to, k);
        }
        let pv = w.g.engage_preview(&w.input()).unwrap();
        let mut inp = EstimateInput::new(&w.g, p0, to, Some(from), Attackers::Force(&pv.force), &pv.defenders);
        inp.def_stance = pv.def_stance_assumed;
        let est = w.g.engage_estimate(&inp).unwrap();
        assert_eq!(est.force, pv.force);
        assert_eq!(est.def_force, pv.assumed_pick);
        let sampled = pv.forecast.win_permille;
        let d = (est.win_permille - sampled).abs();
        sum += d as i64;
        worst = worst.max(d);
        n += 1;
        let contested = (50..=950).contains(&sampled);
        if contested {
            csum += d as i64;
            cworst = cworst.max(d);
            cn += 1;
        }
        lines.push(format!("{}{att:?} v {def:?} {plot}: sampled {sampled} estimated {}", if contested { "* " } else { "  " }, est.win_permille));
    }
    println!("{}", lines.join("\n"));
    let (mean, cmean) = (sum / n, csum / cn.max(1));
    println!("preview vs estimator on {n} real battles: mean |win diff| {mean} per mille, worst {worst}; the {cn} contested ones (*): mean {cmean}, worst {cworst}");
    assert!(cn >= 20, "only {cn} contested battles");
    assert!(mean <= 60, "mean {mean}");
    assert!(cmean <= 120, "contested mean {cmean}");
}

/// The time of a preview and of its What-if list, 6 v 6, 8 v 8 and 10 v 10, each with a reserve
/// of three for the swap search and the swaps (the attacker's slots cut to the size by a city's
/// defense, the defender's at ten so all of its units fight). Two pools: mixed types, where many
/// units are alike (the search tries one swap per pair of kinds), and one where every unit is
/// different (each wounded a little differently: no two alike, the search's worst case). Printed:
/// the plan as it opens (the proposal's swap search and the forecast), the What-if list, the
/// two together, the plan re-run after a swap (a force given), and the proposal alone (what an
/// attack with no force given spends). `cargo test --release -p ffh-core --test
/// engagement_preview -- --ignored --nocapture preview_timing`.
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
    for distinct in [false, true] {
        for size in [6usize, 8, 10] {
            let mut w = world();
            let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
            let (x, y) = w.g.map.xy(to);
            let c = w.g.player_init_city(p1, x, y, true, true);
            let total = w.g.city_total_defense(c, false);
            w.g.cities[c].building_defense += (10 - size as i32) * 25 - total;
            let mix = ["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_SWORDSMAN"];
            let pool: Vec<UnitId> = (0..size + 3).map(|i| w.unit(p0, mix[i % mix.len()], from)).collect();
            if distinct {
                for (i, &u) in pool.iter().enumerate() {
                    w.g.units[u].damage = 2 * i as i32;
                }
            }
            for i in 0..size {
                w.unit(p1, mix[(i + 1) % mix.len()], to);
            }
            let ctx = HitCtx::new(from, to);
            let slots = w.g.engage_slots(&ctx);
            assert_eq!((slots.att() as usize, slots.def()), (size, 10));
            let inp = w.input();
            let reps = 5;
            let g = &w.g;
            let plan_ms = time(reps, &mut || {
                g.engage_preview(&inp).unwrap();
            });
            let mut n_alt = 0;
            let alt_ms = time(reps, &mut || n_alt = g.engage_preview_alternatives(&inp).unwrap().len());
            let pv = g.engage_preview(&inp).unwrap();
            assert_eq!((pv.force.len(), pv.assumed_pick.len()), (size, size));
            let mut swapped = pv.force.clone();
            swapped[0] = pv.reserve[0];
            let swap_inp = PreviewInput { force: Some(&swapped), ..inp };
            let swap_ms = time(reps, &mut || {
                g.engage_preview(&swap_inp).unwrap();
            });
            let propose_ms = time(reps, &mut || {
                g.engage_propose_force(&pool, g.d.default_stance, &slots, &ctx);
            });
            println!(
                "{size} v {size} ({}): plan {plan_ms:.1} ms ({} runs), what-if {alt_ms:.1} ms ({n_alt} plans), together {:.1} ms; after a swap {swap_ms:.1} ms; the proposal alone {propose_ms:.1} ms ({} changed)",
                if distinct { "every unit different" } else { "mixed types" },
                g.d.defines.engagement_preview_runs,
                plan_ms + alt_ms,
                pv.force.iter().filter(|u| !g.engage_auto_pick(&pool, g.d.default_stance, &slots, Side::Attacker, &ctx).contains(u)).count(),
            );
        }
    }
}
