//! The calibration battery (plan 8.2): a fixed synthetic set of battles (mixed roles, strengths,
//! terrain-like defense bonuses, wounds, volleys, first-strike immunity, withdrawal, siege combat
//! limits), each fought many times by [`resolve_sampled_quiet`] and once by [`resolve_expected`].
//!
//! - `engage_calibrate` (ignored; `cargo test --release -p ffh-core -- --ignored engage_calibrate
//!   --nocapture`): 2000 battles x 200 runs; prints [`WIN_TABLE`] (the realized win rate per size
//!   bucket at each knot of the margin) to paste into `expect.rs`, and the estimator's errors.
//! - `estimator_calibration_bounds`: a different 300-battle battery (200 runs each) against the
//!   checked-in table: expected losses within 0.25 units mean absolute on each side, the win
//!   chance within 60 per mille mean absolute and 100 in every bucket with 30 or more battles.

use super::battle::{apply_pct, Battle, Fighter, Pair, Params, Side, TargetClass};
use super::expect::{resolve_expected, resolve_expected_tuned, size_bucket, win_from, Expectation, Tuning, WIN_KNOTS, WIN_TABLE};
use super::resolve::{resolve_sampled_quiet, EngageResult};
use crate::rng::Rng;

/// A role of the spec's table: A %, Dv %, HP, back, screen, ignores the shield, target order.
struct Role {
    a: i32,
    dv: i32,
    hp: i32,
    back: bool,
    screen: bool,
    ignores: bool,
    order: &'static [TargetClass],
}

use TargetClass::{Back, Front, Screen};

const FRONTLINE: Role = Role { a: 100, dv: 100, hp: 120, back: false, screen: false, ignores: false, order: &[Front, Back] };
const SCREEN: Role = Role { a: 100, dv: 110, hp: 120, back: false, screen: true, ignores: false, order: &[Front, Back] };
const BACKLINE: Role = Role { a: 100, dv: 60, hp: 80, back: true, screen: false, ignores: false, order: &[Front, Back] };
const FLANKER: Role = Role { a: 110, dv: 80, hp: 100, back: false, screen: false, ignores: true, order: &[Screen, Back, Front] };
const ASSASSIN: Role = Role { a: 120, dv: 60, hp: 60, back: false, screen: false, ignores: true, order: &[Back, Front] };
const SUPPORT: Role = Role { a: 100, dv: 50, hp: 70, back: true, screen: false, ignores: false, order: &[Front, Back] };

/// A hash of two words (splitmix-style), to seed independent runs.
pub(crate) fn mix(a: u32, b: u32) -> u32 {
    let mut z = (a as u64) << 32 | b as u64;
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as u32
}

fn r(rng: &mut Rng, n: i32) -> i32 {
    rng.get_i(n, "battery")
}

/// One random battle of the battery's shapes.
pub(crate) fn random_battle(rng: &mut Rng) -> Battle {
    let sizes = [1 + r(rng, 10), 1 + r(rng, 10)];
    // Each side's typical strength (x100). Two battles in three are near balance (the square law:
    // n x strength^2 alike, within a factor 0.6-1.6 of strength), the third is anything.
    let level0 = 200 + r(rng, 1000);
    let level1 = if r(rng, 3) == 0 {
        200 + r(rng, 1000)
    } else {
        let ratio_x100 = ((sizes[0] as u64 * 10_000) / sizes[1] as u64).isqrt() as i32;
        (level0 * ratio_x100 / 100 * (60 + r(rng, 100)) / 100).max(50)
    };
    let level = [level0, level1];
    let mut fighters = Vec::new();
    // (base A x100, base Dv x100, A modifier %, Dv modifier %, mounted)
    let mut stats: Vec<(i32, i32, i32, i32, bool)> = Vec::new();
    for (si, side) in [Side::Attacker, Side::Defender].into_iter().enumerate() {
        // A side's shape: how much of it is front line.
        let mix_front = 30 + r(rng, 60);
        for _ in 0..sizes[si] {
            let x = r(rng, 100);
            let (role, siege) = if x < mix_front {
                (if r(rng, 100) < 20 { &SCREEN } else { &FRONTLINE }, false)
            } else {
                match r(rng, 100) {
                    0..=34 => (&BACKLINE, side == Side::Attacker && r(rng, 100) < 30),
                    35..=69 => (&FLANKER, false),
                    70..=84 => (&ASSASSIN, false),
                    _ => (&SUPPORT, false),
                }
            };
            let str_x100 = level[si] * (60 + r(rng, 81)) / 100;
            let mut f = Fighter::new(side, role.hp);
            f.back = role.back;
            f.screen = role.screen;
            f.ignores_shield = role.ignores;
            f.order = role.order.to_vec();
            if r(rng, 100) < 30 {
                f.hp = f.max_hp * (100 - 10 - r(rng, 60));
            }
            let ranged = std::ptr::eq(role, &BACKLINE) || std::ptr::eq(role, &ASSASSIN);
            if (ranged || std::ptr::eq(role, &FLANKER)) && r(rng, 100) < 50 {
                f.first_strikes = 1;
            }
            if r(rng, 100) < 15 {
                f.chance_first_strikes = 1 + r(rng, 2);
            }
            f.first_strike_immune = std::ptr::eq(role, &ASSASSIN) || (std::ptr::eq(role, &FLANKER) && r(rng, 100) < 30);
            if std::ptr::eq(role, &FLANKER) {
                f.withdraw_pct = [10, 20, 25, 35][r(rng, 4) as usize];
            } else if siege {
                f.withdraw_pct = 80;
                f.combat_limit = 75;
            } else if r(rng, 100) < 5 {
                f.withdraw_pct = [10, 50, 100][r(rng, 3) as usize];
            }
            if side == Side::Attacker && !siege && r(rng, 100) < 3 {
                f.combat_limit = 90;
            }
            let (amod, dmod) = match side {
                Side::Attacker => (r(rng, 70) - 20, r(rng, 30)),
                Side::Defender => (r(rng, 30), r(rng, 120)),
            };
            stats.push((str_x100 * role.a / 100, str_x100 * role.dv / 100, amod, dmod, std::ptr::eq(role, &FLANKER)));
            f.tag = fighters.len() as u32;
            fighters.push(f);
        }
    }
    let screens: Vec<bool> = fighters.iter().map(|f| f.screen).collect();
    let bonus: Vec<i32> = (0..fighters.len() * fighters.len()).map(|_| if r(rng, 100) < 8 { 25 } else { 0 }).collect();
    let n = fighters.len();
    Battle::new(Params::default(), fighters, |h, t| {
        let (a, _, amod, _, _) = stats[h];
        let (_, dv, _, dmod, mounted) = stats[t];
        let vs = bonus[h * n + t] + if screens[h] && mounted { 40 } else { 0 };
        Pair { a: apply_pct(a.max(1), amod + vs), dv: apply_pct(dv.max(1), dmod) }
    })
}

/// A battle's sampled outcome over `runs` runs: win rate (per mille), mean losses per side
/// (x1000), draw rate (per mille), the largest rounds and volley rounds seen.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Sampled {
    pub win: i64,
    pub att_loss: i64,
    pub def_loss: i64,
    pub draw: i64,
    pub max_rounds: i32,
    pub max_volley_rounds: i32,
}

pub(crate) fn sample(b: &Battle, runs: u32, seed: u32) -> Sampled {
    let mut s = Sampled::default();
    for k in 0..runs {
        let mut rng = Rng::new(mix(seed, k));
        let log = resolve_sampled_quiet(b, &mut rng);
        let (la, ld) = log.losses(b);
        s.win += (log.result == EngageResult::AttackerWon) as i64;
        s.draw += (log.result == EngageResult::Draw) as i64;
        s.att_loss += la as i64;
        s.def_loss += ld as i64;
        s.max_rounds = s.max_rounds.max(log.rounds);
        s.max_volley_rounds = s.max_volley_rounds.max(log.volley_rounds);
    }
    let n = runs.max(1) as i64;
    Sampled { win: s.win * 1000 / n, att_loss: s.att_loss * 1000 / n, def_loss: s.def_loss * 1000 / n, draw: s.draw * 1000 / n, ..s }
}

/// A battery row: the battle, its expectation and its sampled outcome.
pub(crate) type Row = (Battle, Expectation, Sampled);

/// The battery: `count` battles from `seed`, each with its expectation and sampled outcome.
/// Contested battles (expected margin within 0.3) are all kept, a fifth of the others, so the
/// step of the win chance is well covered.
pub(crate) fn battery(seed: u32, count: usize, runs: u32) -> Vec<Row> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let b = random_battle(&mut rng);
        let e = resolve_expected(&b);
        if e.margin.abs() >= 1229 && r(&mut rng, 5) != 0 {
            continue;
        }
        let s = sample(&b, runs, mix(seed, out.len() as u32));
        out.push((b, e, s));
    }
    out
}

/// The estimator's errors over a battery: mean absolute loss errors per side (x1000), the mean
/// absolute win error (per mille), and per bucket (count, mean absolute win error).
pub(crate) struct Errors {
    pub att_loss: i64,
    pub def_loss: i64,
    pub max_loss: i64,
    pub win: i64,
    pub buckets: [(i64, i64); 4],
}

pub(crate) fn errors(rows: &[Row], table: &[[i16; 15]; 4]) -> Errors {
    let n = rows.len().max(1) as i64;
    let mut e = Errors { att_loss: 0, def_loss: 0, max_loss: 0, win: 0, buckets: [(0, 0); 4] };
    for (_, x, s) in rows {
        let (da, dd) = ((x.att_loss_x1000 as i64 - s.att_loss).abs(), (x.def_loss_x1000 as i64 - s.def_loss).abs());
        e.att_loss += da;
        e.def_loss += dd;
        e.max_loss = e.max_loss.max(da).max(dd);
        let bucket = size_bucket(x.n_att, x.n_def);
        let dw = (win_from(table, x.margin, bucket) as i64 - s.win).abs();
        e.win += dw;
        let bk = &mut e.buckets[bucket];
        bk.0 += 1;
        bk.1 += dw;
    }
    e.att_loss /= n;
    e.def_loss /= n;
    e.win /= n;
    for bk in e.buckets.iter_mut() {
        if bk.0 > 0 {
            bk.1 /= bk.0;
        }
    }
    e
}

/// Fits the win table: per bucket, the battles' sampled win rates binned at the nearest knot of
/// their margin, made monotone (pooled adjacent violators, weighted by count), empty knots
/// interpolated, clamped to 20..=980.
pub(crate) fn fit_table(rows: &[Row]) -> [[i16; 15]; 4] {
    let mut sum = [[0i64; 15]; 4];
    let mut cnt = [[0i64; 15]; 4];
    for (_, x, s) in rows {
        let bk = size_bucket(x.n_att, x.n_def);
        let k = (0..15).min_by_key(|&k| (WIN_KNOTS[k] - x.margin).abs()).unwrap();
        sum[bk][k] += s.win;
        cnt[bk][k] += 1;
    }
    let mut out = [[0i16; 15]; 4];
    for bk in 0..4 {
        // Pool adjacent violators over the knots that have data.
        let mut blocks: Vec<(i64, i64, Vec<usize>)> = Vec::new(); // (sum, count, knots)
        for k in 0..15 {
            if cnt[bk][k] == 0 {
                continue;
            }
            blocks.push((sum[bk][k], cnt[bk][k], vec![k]));
            while blocks.len() >= 2 {
                let l = blocks.len();
                let (s1, c1) = (blocks[l - 2].0, blocks[l - 2].1);
                let (s2, c2) = (blocks[l - 1].0, blocks[l - 1].1);
                if s1 * c2 <= s2 * c1 {
                    break;
                }
                let last = blocks.pop().unwrap();
                let prev = blocks.last_mut().unwrap();
                prev.0 += last.0;
                prev.1 += last.1;
                prev.2.extend(last.2);
            }
        }
        let mut val: [Option<i64>; 15] = [None; 15];
        for (s, c, ks) in &blocks {
            for &k in ks {
                val[k] = Some(s / c);
            }
        }
        // Ends: 20 below the lowest data, 980 above the highest; interpolate the gaps.
        if val[0].is_none() {
            val[0] = Some(20);
        }
        if val[14].is_none() {
            val[14] = Some(980);
        }
        let mut k = 0;
        while k < 15 {
            if val[k].is_some() {
                k += 1;
                continue;
            }
            let lo = k - 1;
            let hi = (k..15).find(|&j| val[j].is_some()).unwrap();
            let (y0, y1) = (val[lo].unwrap(), val[hi].unwrap());
            let (x0, x1) = (WIN_KNOTS[lo] as i64, WIN_KNOTS[hi] as i64);
            for j in k..hi {
                val[j] = Some(y0 + (y1 - y0) * (WIN_KNOTS[j] as i64 - x0) / (x1 - x0));
            }
            k = hi;
        }
        for k in 0..15 {
            out[bk][k] = val[k].unwrap().clamp(20, 980) as i16;
        }
        // Monotone after clamping and interpolation.
        for k in 1..15 {
            out[bk][k] = out[bk][k].max(out[bk][k - 1]);
        }
    }
    out
}

const CALIBRATION_SEED: u32 = 20261031;
const BOUNDS_SEED: u32 = 777;

fn describe(b: &Battle) -> String {
    let mut out = String::new();
    for side in [Side::Attacker, Side::Defender] {
        let tags: Vec<String> = b
            .fighters
            .iter()
            .filter(|f| f.side == side)
            .map(|f| {
                let role = match (f.back, f.screen, f.ignores_shield, f.max_hp) {
                    (true, _, _, 80) => "B",
                    (true, _, _, _) => "S",
                    (_, true, _, _) => "Sc",
                    (_, _, true, 100) => "Fl",
                    (_, _, true, _) => "As",
                    _ => "F",
                };
                format!("{role}{}{}", if f.combat_limit < 100 { "!" } else { "" }, if f.withdraw_pct > 0 { "w" } else { "" })
            })
            .collect();
        out += &format!("[{}] ", tags.join(" "));
    }
    out
}

fn report(name: &str, rows: &[Row], table: &[[i16; 15]; 4]) {
    let e = errors(rows, table);
    println!(
        "{name}: {} battles: losses mean abs err att {} def {} (x1000), max {}; win mean abs err {} per mille; buckets (n, err) {:?}",
        rows.len(),
        e.att_loss,
        e.def_loss,
        e.max_loss,
        e.win,
        e.buckets
    );
}

/// Battles in the calibration battery (the plan's 2000 left the one-unit bucket thin near a
/// margin of 0).
const CALIBRATION_BATTLES: usize = 4000;

/// Prints the win table to check in, the estimator's errors, and how the end rules compare.
#[test]
#[ignore]
fn engage_calibrate() {
    let rows = battery(CALIBRATION_SEED, CALIBRATION_BATTLES, 200);
    let table = fit_table(&rows);
    println!("pub const WIN_TABLE: [[i16; 15]; 4] = [");
    for row in &table {
        println!("    {:?},", row);
    }
    println!("];");
    println!("table checked in: {}", table == WIN_TABLE);
    let mut counts = [[0; 15]; 4];
    for (_, x, _) in &rows {
        counts[size_bucket(x.n_att, x.n_def)][(0..15).min_by_key(|&k| (WIN_KNOTS[k] - x.margin).abs()).unwrap()] += 1;
    }
    println!("battles per knot: {counts:?}");
    report("calibration battery, fitted table", &rows, &table);
    report("calibration battery, checked-in table", &rows, &WIN_TABLE);
    let mut hist = [0; 5];
    for (_, _, s) in &rows {
        hist[match s.win {
            0..=49 => 0,
            50..=199 => 1,
            200..=799 => 2,
            800..=950 => 3,
            _ => 4,
        }] += 1;
    }
    println!("sampled win rates <5% / 5-20% / 20-80% / 80-95% / >95%: {hist:?}");
    let draws = rows.iter().map(|r| r.2.draw).sum::<i64>() / rows.len() as i64;
    println!("sampled draws {draws} per mille");
    let mut worst: Vec<&Row> = rows.iter().collect();
    worst.sort_by_key(|(_, x, s)| -((x.att_loss_x1000 as i64 - s.att_loss).abs().max((x.def_loss_x1000 as i64 - s.def_loss).abs())));
    for (b, x, s) in worst.iter().take(6) {
        println!(
            "  worst: {}loss exp {}/{} sampled {}/{}, win exp {} sampled {}, margin {}, rounds {}",
            describe(b),
            x.att_loss_x1000,
            x.def_loss_x1000,
            s.att_loss,
            s.def_loss,
            x.win_permille,
            s.win,
            x.margin,
            x.rounds
        );
    }
    // The variants: the plan's end rule (0.05, residual counted), the prototype's (0.5, not
    // counted) and between, with and without the presence covariance. Each fits its own table on
    // the calibration battery and is judged on the bounds one.
    let held = battery(BOUNDS_SEED, 300, 200);
    for (stop, residual, presence) in [(205, true, true), (205, true, false), (205, false, true), (1024, true, true), (1024, true, false), (2048, false, false)] {
        let tu = Tuning { stop, residual, presence };
        let re = |rows: &[Row]| -> Vec<Row> { rows.iter().map(|(b, _, s)| (b.clone(), resolve_expected_tuned(b, &[], &tu), *s)).collect() };
        let (cal, val) = (re(&rows), re(&held));
        let t = fit_table(&cal);
        report(&format!("stop {stop} residual {residual} presence {presence}: calibration"), &cal, &t);
        report(&format!("stop {stop} residual {residual} presence {presence}: held out"), &val, &t);
    }
}

/// What a call costs (release build; `--ignored engage_cost --nocapture`): both integrators on
/// the bounds battery and on a full open-land engagement (10 a side).
#[test]
#[ignore]
fn engage_cost() {
    let rows = battery(BOUNDS_SEED, 300, 1);
    let mut b10 = None;
    let mut rng = Rng::new(5);
    while b10.is_none() {
        let b = random_battle(&mut rng);
        if b.side_counts() == (10, 10) {
            b10 = Some(b);
        }
    }
    let b10 = b10.unwrap();
    let time = |name: &str, reps: u32, f: &mut dyn FnMut()| {
        let t = std::time::Instant::now();
        for _ in 0..reps {
            f();
        }
        println!("{name}: {:.1} us a call", t.elapsed().as_secs_f64() * 1e6 / reps as f64);
    };
    let mut sink = 0i64;
    time("resolve_expected, bounds battery", 10, &mut || {
        for (b, _, _) in &rows {
            sink += resolve_expected(b).margin as i64;
        }
    });
    time("resolve_sampled_quiet, bounds battery", 10, &mut || {
        for (i, (b, _, _)) in rows.iter().enumerate() {
            sink += resolve_sampled_quiet(b, &mut Rng::new(i as u32)).rounds as i64;
        }
    });
    time("resolve_expected, 10 v 10", 2000, &mut || sink += resolve_expected(&b10).margin as i64);
    time("resolve_sampled, 10 v 10 (with the log)", 2000, &mut || sink += super::resolve::resolve_sampled(&b10, &mut Rng::new(sink as u32)).rounds as i64);
    println!("(battery calls are per battle: divide the first two by {}) {sink}", rows.len());
}

#[test]
fn estimator_calibration_bounds() {
    let rows = battery(BOUNDS_SEED, 300, 200);
    let e = errors(&rows, &WIN_TABLE);
    assert!(e.att_loss <= 250 && e.def_loss <= 250, "expected losses off by {} / {} (x1000) on average", e.att_loss, e.def_loss);
    assert!(e.win <= 60, "win chance off by {} per mille on average", e.win);
    for (bk, &(c, w)) in e.buckets.iter().enumerate() {
        assert!(c < 30 || w <= 100, "bucket {bk}: {c} battles, win chance off by {w} per mille");
    }
}
