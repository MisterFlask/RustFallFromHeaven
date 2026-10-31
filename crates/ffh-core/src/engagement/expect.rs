//! The estimator's integrator (plan 8.2): the round model in moments, no RNG. A run at mean
//! damage is not the mean outcome (deaths are thresholds, and the dead stop hitting), so per
//! fighter it keeps its alive weight `a` (the chance it is alive and engaged), and the mean `mu`
//! and variance `v` of the damage it has taken while engaged. Integer only: weights in 1/4096,
//! damage moments in centi-HP (x4096 fixed point), `Phi` from a table, `sqrt` by `u64::isqrt`,
//! so native and wasm agree. A float prototype of the method (scratchpad `p6rev/moment2.py`) is
//! the reference it was checked against (`tests::matches_the_float_prototype`).
//!
//! **A round.** Hitter `h`, with weight `a_h` (times `P(volleys >= k)` in volley round k), goes
//! down its class order ([`Battle`]'s disjoint classes, the same shield and limit rules as
//! [`super::targets`]): it reaches class `C` when the classes before it are empty (each with
//! probability `prod (1 - a_j)` over its available members, independence assumed). Given target
//! `t` is alive, `t` gets `q = a_h x P(earlier empty) x s_t / (1 + sum_{j in C, j != t} avail_j)`
//! hits, where `s_t` is 1, or for a fighter behind the shield the chance that the enemy front left
//! in play is empty (and then only the other back members compete). `mu_t += q E[hit]`,
//! `v_t += q E[hit^2] - (q E[hit])^2`, with E[hit] and E[hit^2] over r's discrete uniform
//! (`(41^2 - 1) / 12` = 140 %^2 for 80..=120). An attacker with a combat limit gives nothing to a
//! target whose expected damage reached its limit and cuts its expected hit at it; in a melee round
//! it leaves with the chance that it finds no target while the enemy is not all gone.
//!
//! **After the round**, simultaneously: `S_t = P(D < hp)`, `W_t = P(D >= hp - threshold)`
//! (normal approximation, both kept monotone). Of the fighter's population, a share `c` (its
//! withdrawal chance) of each newly wounded mass leaves alive (the one roll, the fatal round
//! included), the rest stays and dies by `S`: engaged `(1 - c) S + c (1 - W)`, withdrawn `c W`,
//! dead `(1 - c)(1 - S)`; the fighter's weight follows that population, scaled by what left by
//! its combat limit. Fear: a fighter starts with weight `1 - fear_fail`. Battle effects: `Mod`s
//! are in the pair table, `Damage` lowers HP before the volleys (as in the sampled run).
//!
//! **The end**: when a side's alive weight is below [`STOP`] (that side lost: its residual weight
//! counts as lost, since the losing side loses every committed unit that did not withdraw), or
//! after `max_rounds` rounds (a draw). The margin `M = Pa_end / Pa_start - Pd_end / Pd_start`,
//! with `P = sum a_i x E[hp left]_i x power_i` (power: the fighter's mean damage per hit against
//! the enemy), is the share of its fighting power each side keeps; [`win_permille`] reads the
//! calibrated table at it.

use super::battle::{Battle, BattleEffect, Side};
use super::phi_table::{PARTIAL, PHI};

/// Weights and probabilities: 1/4096.
pub const ONE: i64 = 4096;

/// The end rule (crate-internal, so the calibration battery can compare variants): a side whose
/// alive weight is below `stop` has lost, and with `residual` its remaining alive weight counts
/// as lost.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tuning {
    pub stop: i64,
    pub residual: bool,
}

/// The end rule in use.
pub(crate) const TUNING: Tuning = Tuning { stop: STOP, residual: true };

/// A side whose alive weight is below this (0.05) has lost.
pub const STOP: i64 = 205;

/// The standard normal CDF at `z_x1024 / 1024`, in 1/4096 (linear in the table, clamped).
pub fn phi(z_x1024: i64) -> i64 {
    table_at(&PHI, z_x1024, |_| 0, |_| ONE)
}

/// `z Phi(z) + phi(z)` at `z_x1024 / 1024`, in 1/4096: `E[(z - X)+]` for a standard normal X.
fn partial(z_x1024: i64) -> i64 {
    table_at(&PARTIAL, z_x1024, |_| 0, |z| z * 4)
}

fn table_at(t: &[u16; 257], z: i64, below: impl Fn(i64) -> i64, above: impl Fn(i64) -> i64) -> i64 {
    let pos = z + 4096;
    if pos < 0 {
        return below(z);
    }
    if pos >= 8192 {
        return above(z);
    }
    let (i, frac) = ((pos / 32) as usize, pos % 32);
    let (lo, hi) = (t[i] as i64, t[i + 1] as i64);
    lo + (hi - lo) * frac / 32
}

/// A fighter's state carried in from an earlier engagement (an assault's defenders): alive
/// weight (1/4096) and the mean and variance of the damage taken (centi-HP, centi-HP^2), on top
/// of `Fighter::hp`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MomentStart {
    pub alive: i32,
    pub mu: i64,
    pub var: i64,
}

impl Default for MomentStart {
    fn default() -> MomentStart {
        MomentStart { alive: ONE as i32, mu: 0, var: 0 }
    }
}

/// A fighter at the end of the expected battle.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FighterExpect {
    /// Alive and engaged at the end (1/4096); 0 on the losing side.
    pub alive: i32,
    /// Left alive: withdrawn, or at its combat limit (1/4096).
    pub withdrawn: i32,
    /// Sat out (fear, 1/4096).
    pub sat_out: i32,
    /// Lost (1/4096): killed, plus the losing side's residual alive weight.
    pub lost: i32,
    /// Mean and variance of the damage it took while engaged (centi-HP, centi-HP^2).
    pub mu: i64,
    pub var: i64,
    /// Expected HP left if alive (centi-HP).
    pub hp_alive: i32,
    /// Expected HP left over all its outcomes (alive, withdrawn and sat out; dead count 0).
    pub hp_left: i32,
}

/// The outcome of [`resolve_expected`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Expectation {
    pub fighters: Vec<FighterExpect>,
    pub volley_rounds: i32,
    pub rounds: i32,
    /// Ran to `max_rounds`.
    pub draw: bool,
    /// Expected fighters lost and withdrawn per side, x1000.
    pub att_loss_x1000: i32,
    pub def_loss_x1000: i32,
    pub att_withdrawn_x1000: i32,
    pub def_withdrawn_x1000: i32,
    /// Expected HP left per side (centi-HP), every outcome but death.
    pub att_hp_left: i64,
    pub def_hp_left: i64,
    /// The share of fighting power kept, attacker minus defender (1/4096, -4096..=4096).
    pub margin: i32,
    /// The attacker's chance to win the engagement (per mille, [`win_permille`]).
    pub win_permille: i32,
    /// Fighters per side that can fight (not sat out).
    pub n_att: usize,
    pub n_def: usize,
}

/// The win table's size bucket: `min(n_att, n_def)` in {1, 2-3, 4-7, 8+}.
pub fn size_bucket(n_att: usize, n_def: usize) -> usize {
    match n_att.min(n_def) {
        0 | 1 => 0,
        2..=3 => 1,
        4..=7 => 2,
        _ => 3,
    }
}

/// The margins (1/4096) at which [`WIN_TABLE`] is given.
pub const WIN_KNOTS: [i32; 15] = [-4096, -2048, -1229, -819, -614, -410, -205, 0, 205, 410, 614, 819, 1229, 2048, 4096];

/// The realized win rate (per mille) at each knot, per size bucket: made by the calibration
/// battery (`engagement::calibrate`, `cargo test --release -p ffh-core -- --ignored
/// engage_calibrate --nocapture`: 4000 battles x 200 sampled runs), which prints this table.
/// A tie (M = 0) wins less than half the time in small battles: both sides often fall in the
/// same round (mutual, not a win). Recalibrate whenever the round model or the margin changes.
pub const WIN_TABLE: [[i16; 15]; 4] = [
    [20, 20, 20, 20, 74, 74, 81, 299, 621, 718, 783, 932, 980, 980, 980],
    [20, 20, 20, 60, 87, 209, 331, 443, 625, 745, 827, 909, 977, 980, 980],
    [20, 20, 20, 52, 109, 217, 372, 465, 625, 770, 865, 951, 980, 980, 980],
    [20, 20, 20, 37, 111, 200, 335, 509, 632, 784, 900, 971, 980, 980, 980],
];

/// The attacker's chance to win at margin `m` (1/4096) in size bucket `bucket`: the table,
/// linear between knots, clamped to 20..=980.
pub fn win_permille(m: i32, bucket: usize) -> i32 {
    win_from(&WIN_TABLE, m, bucket)
}

/// [`win_permille`] read from `table`.
pub(crate) fn win_from(table: &[[i16; 15]; 4], m: i32, bucket: usize) -> i32 {
    let row = &table[bucket.min(3)];
    let m = m.clamp(WIN_KNOTS[0], WIN_KNOTS[14]);
    let mut i = 0;
    while i + 2 < WIN_KNOTS.len() && m > WIN_KNOTS[i + 1] {
        i += 1;
    }
    let (x0, x1) = (WIN_KNOTS[i] as i64, WIN_KNOTS[i + 1] as i64);
    let (y0, y1) = (row[i] as i64, row[i + 1] as i64);
    let v = y0 + (y1 - y0) * (m as i64 - x0) / (x1 - x0);
    (v as i32).clamp(20, 980)
}

/// Per-fighter integrator state. Damage moments are x4096 (`mu_q = mu x 4096`).
#[derive(Clone, Debug)]
struct St {
    /// HP at the start (centi-HP), after the damage effects.
    hp: i64,
    /// The weight alive at the start (1 unless carried in), the weight of it that fights (not
    /// sat out), and the alive and engaged weight.
    alive0: i64,
    fight: i64,
    a: i64,
    /// Withdrawn (or left at the limit) weight, and its HP x weight.
    w: i64,
    w_hp: i64,
    mu_q: i64,
    v_q: i64,
    /// Survival and wounded probabilities of the damage, the engaged share of the population.
    s: i64,
    wnd: i64,
    pe: i64,
    /// Withdrawal chance (1/4096) and threshold (centi-HP).
    c: i64,
    thr: i64,
    /// Hit by a hitter without a combat limit: until then its damage is capped and cannot kill.
    unlimited: bool,
}

/// `P(D < x)` and `E[(x - D)+]` (centi-HP) for the damage `D ~ N(mu, v)` (x4096 moments).
fn below(x: i64, mu_q: i64, v_q: i64) -> (i64, i64) {
    let x_q = x * ONE;
    if v_q <= 0 {
        return if mu_q < x_q { (ONE, (x_q - mu_q) / ONE) } else { (0, 0) };
    }
    // sq = sqrt(v) x 1024 (v = v_q / 4096).
    let sq = (v_q as u64).saturating_mul(256).isqrt() as i64;
    if sq == 0 {
        return if mu_q < x_q { (ONE, (x_q - mu_q) / ONE) } else { (0, 0) };
    }
    let z = ((x_q - mu_q) as i128 * 256 / sq as i128).clamp(-(1 << 40), 1 << 40) as i64;
    let p = phi(z);
    let e = (sq as i128 * partial(z) as i128 / (1024 * ONE) as i128) as i64;
    (p, e.max(0))
}

/// Resolves the battle in moments (no RNG).
pub fn resolve_expected(b: &Battle) -> Expectation {
    resolve_expected_from(b, &[])
}

/// [`resolve_expected`] with fighters' states carried in (`start[i]` for fighter `i`; missing
/// entries start fresh).
pub fn resolve_expected_from(b: &Battle, start: &[MomentStart]) -> Expectation {
    resolve_expected_tuned(b, start, &TUNING)
}

pub(crate) fn resolve_expected_tuned(b: &Battle, start: &[MomentStart], tu: &Tuning) -> Expectation {
    let n = b.fighters.len();
    let p = &b.params;
    let mut hp0: Vec<i64> = b.fighters.iter().map(|f| f.hp as i64).collect();
    for e in &b.effects {
        if let BattleEffect::Damage { fighter, pct, .. } = *e {
            hp0[fighter as usize] -= pct.max(0) as i64 * b.fighters[fighter as usize].max_hp as i64;
        }
    }
    let mut st: Vec<St> = (0..n)
        .map(|i| {
            let f = &b.fighters[i];
            let s0 = start.get(i).copied().unwrap_or_default();
            let alive0 = s0.alive.clamp(0, ONE as i32) as i64;
            let fight = if f.sat_out { 0 } else { (ONE - f.fear_fail.clamp(0, ONE as i32) as i64) * alive0 / ONE };
            let dead = hp0[i] <= 0;
            let (mu_q, v_q) = (s0.mu.max(0) * ONE, s0.var.max(0) * ONE);
            // Survival so far under the carried damage (1 when fresh); the alive weight carried
            // in already counts it, so the rounds scale from here. Every engagement gives its
            // fighters a new withdrawal roll.
            let s = if dead { 0 } else { below(hp0[i], mu_q, v_q).0 };
            let c = f.withdraw_pct.clamp(0, 100) as i64 * ONE / 100;
            St {
                hp: hp0[i],
                alive0,
                fight,
                a: if dead { 0 } else { fight },
                w: 0,
                w_hp: 0,
                mu_q,
                v_q,
                s,
                wnd: 0,
                pe: ((ONE - c) * s + c * ONE) / ONE,
                c,
                thr: f.withdraw_threshold(p) as i64,
                unlimited: mu_q > 0 || v_q > 0,
            }
        })
        .collect();
    // Fighting power: mean damage per hit against the enemy, averaged.
    let power: Vec<i64> = (0..n)
        .map(|h| {
            let es: Vec<usize> = (0..n).filter(|&t| b.fighters[t].side != b.fighters[h].side).collect();
            if es.is_empty() {
                1
            } else {
                (es.iter().map(|&t| b.mean_damage(h, t) as i64).sum::<i64>() / es.len() as i64).max(1)
            }
        })
        .collect();
    let power_of = |st: &[St], side: Side, start: bool| -> i64 {
        (0..n)
            .filter(|&i| b.fighters[i].side == side)
            .map(|i| {
                let s = &st[i];
                if start {
                    s.a * (s.hp - s.mu_q / ONE).max(0) / ONE * power[i]
                } else {
                    s.a * hp_alive(s) / ONE * power[i]
                }
            })
            .sum()
    };
    let p_start = [power_of(&st, Side::Attacker, true), power_of(&st, Side::Defender, true)];
    let er_milli = (p.rand_min + p.rand_max) as i64 * 5; // E[r] / 100, x1000
    let span = (p.rand_max - p.rand_min + 1).max(1) as i64;
    let er2_milli = (p.rand_min..=p.rand_max).map(|r| (r as i64) * (r as i64)).sum::<i64>() / span / 10; // E[r^2] / 10000, x1000

    let mut volley_rounds = 0;
    let mut rounds = 0;
    let mut stopped = side_alive(b, &st).iter().any(|&x| x < tu.stop);
    let max_v = b
        .fighters
        .iter()
        .zip(&st)
        .filter(|(_, s)| s.a > 0)
        .map(|(f, _)| f.first_strikes.max(0) + f.chance_first_strikes.max(0))
        .max()
        .unwrap_or(0);
    let mut scratch = Scratch::new(n);
    if !stopped {
        for k in 1..=max_v {
            volley_rounds = k;
            round(b, &mut st, &mut scratch, Some(k), er_milli, er2_milli);
            if side_alive(b, &st).iter().any(|&x| x < tu.stop) {
                stopped = true;
                break;
            }
        }
    }
    if !stopped {
        for r in 1..=p.max_rounds {
            rounds = r;
            round(b, &mut st, &mut scratch, None, er_milli, er2_milli);
            if side_alive(b, &st).iter().any(|&x| x < tu.stop) {
                stopped = true;
                break;
            }
        }
    }
    let alive = side_alive(b, &st);
    let p_end = [power_of(&st, Side::Attacker, false), power_of(&st, Side::Defender, false)];
    let share = |i: usize| if p_start[i] > 0 { (p_end[i] as i128 * ONE as i128 / p_start[i] as i128) as i64 } else { 0 };
    let margin = (share(0) - share(1)).clamp(-ONE, ONE) as i32;
    let mut out = Expectation { volley_rounds, rounds, draw: !stopped, margin, ..Expectation::default() };
    // Per side, in 1/4096: lost, withdrawn.
    let mut sums = [[0i64; 2]; 2];
    for (i, s) in st.iter().enumerate() {
        let f = &b.fighters[i];
        let side = f.side.index();
        let loser = tu.residual && stopped && alive[side] < tu.stop;
        let dead = (s.fight - s.a - s.w).max(0);
        let sat_out = (s.alive0 - s.fight).max(0);
        let fe = FighterExpect {
            alive: if loser { 0 } else { s.a as i32 },
            withdrawn: s.w as i32,
            sat_out: sat_out as i32,
            lost: (dead + if loser { s.a } else { 0 }) as i32,
            mu: s.mu_q / ONE,
            var: s.v_q / ONE,
            hp_alive: hp_alive(s) as i32,
            hp_left: ((if loser { 0 } else { s.a * hp_alive(s) } + s.w_hp + sat_out * (s.hp - s.mu_q / ONE).max(0)) / ONE) as i32,
        };
        sums[side][0] += fe.lost as i64;
        sums[side][1] += fe.withdrawn as i64;
        if s.fight > 0 {
            if f.side == Side::Attacker {
                out.n_att += 1
            } else {
                out.n_def += 1
            }
        }
        if f.side == Side::Attacker {
            out.att_hp_left += fe.hp_left as i64;
        } else {
            out.def_hp_left += fe.hp_left as i64;
        }
        out.fighters.push(fe);
    }
    let x1000 = |v: i64| ((v * 1000 + ONE / 2) / ONE) as i32;
    out.att_loss_x1000 = x1000(sums[0][0]);
    out.att_withdrawn_x1000 = x1000(sums[0][1]);
    out.def_loss_x1000 = x1000(sums[1][0]);
    out.def_withdrawn_x1000 = x1000(sums[1][1]);
    out.win_permille = win_permille(out.margin, size_bucket(out.n_att, out.n_def));
    out
}

/// Expected HP left if alive (centi-HP): `E[hp - D | D < hp]`.
fn hp_alive(s: &St) -> i64 {
    if !s.unlimited {
        return (s.hp - s.mu_q / ONE).max(0);
    }
    let (pa, e) = below(s.hp, s.mu_q, s.v_q);
    if pa <= 0 {
        0
    } else {
        (e * ONE / pa).min(s.hp.max(0))
    }
}

fn side_alive(b: &Battle, st: &[St]) -> [i64; 2] {
    let mut a = [0; 2];
    for (f, s) in b.fighters.iter().zip(st) {
        a[f.side.index()] += s.a;
    }
    a
}

struct Scratch {
    inc_mu: Vec<i64>,
    inc_v: Vec<i64>,
    leave: Vec<i64>,
    /// Hit this round by a hitter without a combat limit.
    unlim: Vec<bool>,
    /// `absent[j] == hitter stamp`: an available member of one of the hitter's earlier classes
    /// (absent, given those classes are empty).
    absent: Vec<u32>,
    /// `eff[j] == class stamp`: an available member of the class being read.
    eff: Vec<u32>,
    stamp: u32,
}

impl Scratch {
    fn new(n: usize) -> Scratch {
        Scratch { inc_mu: vec![0; n], inc_v: vec![0; n], leave: vec![0; n], unlim: vec![false; n], absent: vec![0; n], eff: vec![0; n], stamp: 0 }
    }

    fn next(&mut self) -> u32 {
        self.stamp += 1;
        self.stamp
    }
}

/// `P(volleys >= k)` (1/4096) for `first_strikes + U{0..=chance}` volleys.
fn p_volleys_at_least(fs: i32, chance: i32, k: i32) -> i64 {
    let (fs, chance) = (fs.max(0), chance.max(0));
    if k <= fs {
        ONE
    } else if k > fs + chance {
        0
    } else {
        (fs + chance - k + 1) as i64 * ONE / (chance + 1) as i64
    }
}

/// One volley round (`volley = Some(k)`) or round, all fighters at once.
fn round(b: &Battle, st: &mut [St], sc: &mut Scratch, volley: Option<i32>, er_milli: i64, er2_milli: i64) {
    let n = b.fighters.len();
    sc.inc_mu.iter_mut().for_each(|x| *x = 0);
    sc.inc_v.iter_mut().for_each(|x| *x = 0);
    sc.leave.iter_mut().for_each(|x| *x = 0);
    sc.unlim.iter_mut().for_each(|x| *x = false);
    for h in 0..n {
        let fh = &b.fighters[h];
        let mut wh = st[h].a;
        if let Some(k) = volley {
            wh = wh * p_volleys_at_least(fh.first_strikes, fh.chance_first_strikes, k) / ONE;
        }
        if wh <= 0 {
            continue;
        }
        let enemy = fh.side.other();
        let front = b.front(enemy);
        let hstamp = sc.next();
        let mut cond = ONE;
        for entry in b.classes(h) {
            // The class's available members: alive, not immune to a volley, not at h's limit.
            let cstamp = sc.next();
            let mut any = false;
            let (mut sum_u, mut sum_s) = (0i64, 0i64);
            let (mut prod_u, mut prod_s) = (ONE, ONE);
            for &j in &entry.members {
                let ju = j as usize;
                let s = &st[ju];
                let ok = s.a > 0
                    && !(volley.is_some() && b.fighters[ju].first_strike_immune)
                    && b.cap_floor(h, ju).is_none_or(|floor| s.hp - s.mu_q / ONE > floor as i64);
                if !ok {
                    continue;
                }
                sc.eff[ju] = cstamp;
                any = true;
                if b.shielded(h, ju) {
                    sum_s += s.a;
                    prod_s = prod_s * (ONE - s.a) / ONE;
                } else {
                    sum_u += s.a;
                    prod_u = prod_u * (ONE - s.a) / ONE;
                }
            }
            if !any {
                continue;
            }
            // The enemy front still in play: not known absent from an earlier class. A shielded
            // member is reachable when all of it is gone; the class is empty when its own
            // available members are gone and the rest of the front still shields the others.
            let (mut e_rest, mut e_rest_out) = (ONE, ONE);
            for &j in front {
                let ju = j as usize;
                if sc.absent[ju] == hstamp {
                    continue;
                }
                let q = ONE - st[ju].a;
                e_rest = e_rest * q / ONE;
                if sc.eff[ju] != cstamp {
                    e_rest_out = e_rest_out * q / ONE;
                }
            }
            for &j in &entry.members {
                let t = j as usize;
                if sc.eff[t] != cstamp {
                    continue;
                }
                let at = st[t].a;
                let q = if b.shielded(h, t) {
                    wh * cond / ONE * e_rest / (ONE + sum_s - at)
                } else {
                    wh * cond / (ONE + sum_u - at)
                };
                if q <= 0 {
                    continue;
                }
                let m = b.mean_damage(h, t) as i64;
                let mut e1 = q * m * er_milli / 1000;
                let mut e2 = q * m * m * er2_milli / 1000;
                if let Some(floor) = b.cap_floor(h, t) {
                    let room = ((st[t].hp - floor as i64) * ONE - st[t].mu_q - sc.inc_mu[t]).max(0);
                    if e1 > room {
                        e2 = if e1 > 0 { (e2 as i128 * room as i128 / e1 as i128) as i64 } else { 0 };
                        e1 = room;
                    }
                } else if e1 > 0 {
                    sc.unlim[t] = true;
                }
                sc.inc_mu[t] += e1;
                sc.inc_v[t] += (e2 - e1 * e1 / ONE).max(0);
            }
            // P(class empty): no unshielded member, and no shielded one reachable.
            let e_k = prod_u * (ONE - e_rest_out * (ONE - prod_s) / ONE) / ONE;
            cond = cond * e_k / ONE;
            for &j in &entry.members {
                if sc.eff[j as usize] == cstamp {
                    sc.absent[j as usize] = hstamp;
                }
            }
        }
        if volley.is_none() && fh.side == Side::Attacker && fh.combat_limit < 100 {
            let all_gone = (0..n).filter(|&j| b.fighters[j].side == enemy).fold(ONE, |acc, j| acc * (ONE - st[j].a) / ONE);
            sc.leave[h] = (cond - all_gone).max(0);
        }
    }
    for (t, s) in st.iter_mut().enumerate() {
        if s.a <= 0 {
            continue;
        }
        s.mu_q += sc.inc_mu[t];
        s.v_q += sc.inc_v[t];
        s.unlimited |= sc.unlim[t];
        // Damage from hitters with a combat limit stops at it: it cannot kill.
        let alive = if s.unlimited { below(s.hp, s.mu_q, s.v_q).0 } else { ONE };
        let s_new = s.s.min(alive);
        let w_new = if s.c > 0 { s.wnd.max(ONE - below(s.hp - s.thr, s.mu_q, s.v_q).0) } else { 0 };
        let pe_new = ((ONE - s.c) * s_new + s.c * (ONE - w_new)) / ONE;
        if s.pe > 0 {
            let a_old = s.a;
            s.a = a_old * pe_new / s.pe;
            let dw = a_old * s.c / ONE * (w_new - s.wnd) / s.pe;
            s.w += dw;
            // A fighter that withdraws leaves just below its threshold: three quarters of it.
            s.w_hp += dw * (s.thr * 3 / 4).min(s.hp).max(0);
        }
        s.s = s_new;
        s.wnd = w_new;
        s.pe = pe_new;
        if sc.leave[t] > 0 {
            let dl = s.a * sc.leave[t] / ONE;
            s.a -= dl;
            s.w += dl;
            s.w_hp += dl * hp_alive(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::battle::{Fighter, Pair, Params, TargetClass};
    use super::super::calibrate::{mix, sample};
    use super::super::resolve::resolve_sampled_quiet;
    use super::*;
    use crate::rng::Rng;

    /// The standard normal CDF and partial expectation by Simpson's rule (tests only).
    fn normal(z: f64) -> (f64, f64) {
        let pdf = |x: f64| (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let (lo, steps) = (-10.0, 20000);
        let h = (z - lo) / steps as f64;
        let mut s = pdf(lo) + pdf(z);
        for i in 1..steps {
            s += pdf(lo + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        let cdf = s * h / 3.0;
        (cdf, z * cdf + pdf(z))
    }

    #[test]
    fn tables_match_the_normal() {
        for i in 0..=256 {
            let z = -4.0 + i as f64 / 32.0;
            let (cdf, part) = normal(z);
            assert!((PHI[i] as f64 - cdf * 4096.0).abs() <= 0.51, "Phi({z})");
            assert!((PARTIAL[i] as f64 - part * 4096.0).abs() <= 0.51, "partial({z})");
        }
        assert_eq!(phi(0), 2048);
        assert_eq!(phi(-5000), 0);
        assert_eq!(phi(5000), ONE);
        assert_eq!(phi(16), (PHI[128] as i64 + PHI[129] as i64) / 2);
    }

    /// `n` frontline fighters a side at full HP (120), each dealing `m` centi-HP a hit on average.
    fn uniform(na: usize, nd: usize, ma: i32, md: i32) -> Battle {
        let fs: Vec<Fighter> = (0..na).map(|_| Fighter::new(Side::Attacker, 120)).chain((0..nd).map(|_| Fighter::new(Side::Defender, 120))).collect();
        // mean = 2000 x A / (A + Dv), so A = m, Dv = 2000 - m gives m.
        Battle::new(Params::default(), fs, |h, _| if h < na { Pair { a: ma, dv: 2000 - ma } } else { Pair { a: md, dv: 2000 - md } })
    }

    /// The review's ten cases: (n_att, n_def, attacker's mean hit, defender's, expected losses
    /// x1000 from the float reference with this integrator's rules (scratchpad
    /// `p6res/moment3.py`: the prototype `p6rev/moment2.py` with the stop at 0.05 checked after
    /// each round, r's discrete variance and the loser's residual counted), its rounds, and the
    /// prototype's own losses x1000 (stop at 0.5, residual not counted)).
    const CASES: [(usize, usize, i32, i32, i32, i32, i32, i32, i32); 10] = [
        (4, 4, 1000, 1000, 3899, 3899, 40, 3550, 3550),
        (8, 8, 1000, 1000, 7882, 7882, 40, 7560, 7560),
        (8, 6, 1000, 1000, 125, 6000, 12, 120, 5890),
        (8, 8, 1100, 900, 1794, 8000, 15, 1790, 7940),
        (8, 4, 1000, 1000, 0, 4000, 8, 0, 3530),
        (8, 8, 800, 1200, 8000, 309, 13, 7740, 300),
        (1, 1, 1000, 1000, 1000, 1000, 34, 780, 780),
        (2, 1, 1000, 1000, 0, 1000, 7, 0, 1000),
        (8, 8, 1200, 800, 309, 8000, 13, 300, 7740),
        (6, 3, 700, 1300, 38, 3000, 11, 30, 2800),
    ];

    #[test]
    fn matches_the_float_prototype() {
        let mut worst = (0, 0);
        for &(na, nd, ma, md, la, ld, rounds, pa, pd) in &CASES {
            let e = resolve_expected(&uniform(na, nd, ma, md));
            let d = (e.att_loss_x1000 - la).abs().max((e.def_loss_x1000 - ld).abs());
            assert!(d <= 25, "{na}v{nd} {ma}/{md}: losses {}/{} against the float {la}/{ld}", e.att_loss_x1000, e.def_loss_x1000);
            assert!((e.rounds - rounds).abs() <= 1, "{na}v{nd} {ma}/{md}: {} rounds against {rounds}", e.rounds);
            // The prototype's own end rule (0.5, residual not counted) differs by design; shown.
            let dp = (e.att_loss_x1000 - pa).abs().max((e.def_loss_x1000 - pd).abs());
            println!("{na}v{nd} {ma}/{md}: losses {}/{}, float reference {la}/{ld}, prototype {pa}/{pd}", e.att_loss_x1000, e.def_loss_x1000);
            worst = (worst.0.max(d), worst.1.max(dp));
        }
        println!("largest difference: {} (x1000) from the float reference, {} from the prototype", worst.0, worst.1);
    }

    #[test]
    fn the_estimate_is_deterministic_and_monotone() {
        let b = uniform(6, 6, 1000, 1000);
        assert_eq!(resolve_expected(&b), resolve_expected(&b));
        let mut last = (i32::MIN, 0);
        for ma in [700, 850, 1000, 1150, 1300] {
            let e = resolve_expected(&uniform(6, 6, ma, 1000));
            assert!(e.margin >= last.0 && e.win_permille >= last.1, "{ma}: {} {}", e.margin, e.win_permille);
            last = (e.margin, e.win_permille);
        }
        assert!(resolve_expected(&uniform(6, 6, 1300, 700)).win_permille >= 950);
        assert!(resolve_expected(&uniform(6, 6, 700, 1300)).win_permille <= 50);
    }

    #[test]
    fn withdrawal_takes_weight_out_alive() {
        // A sure withdrawer against a much stronger defender leaves rather than dies.
        let mut a = Fighter::new(Side::Attacker, 100);
        a.withdraw_pct = 100;
        let b = Battle::new(Params::default(), vec![a, Fighter::new(Side::Defender, 120)], |h, _| if h == 0 { Pair { a: 100, dv: 900 } } else { Pair { a: 900, dv: 100 } });
        let e = resolve_expected(&b);
        assert!(e.fighters[0].withdrawn > 3900, "{:?}", e.fighters[0]);
        assert!(e.att_loss_x1000 < 50);
        let s = sample(&b, 200, 1);
        assert_eq!(s.att_loss, 0);
        // At 50%, about half.
        let mut a = Fighter::new(Side::Attacker, 100);
        a.withdraw_pct = 50;
        let b = Battle::new(Params::default(), vec![a, Fighter::new(Side::Defender, 120)], |h, _| if h == 0 { Pair { a: 100, dv: 900 } } else { Pair { a: 900, dv: 100 } });
        let (e, s) = (resolve_expected(&b), sample(&b, 400, 2));
        assert!((e.att_loss_x1000 as i64 - s.att_loss).abs() <= 80, "{} against {}", e.att_loss_x1000, s.att_loss);
    }

    #[test]
    fn siege_alone_never_kills_and_leaves() {
        let mut fs = Vec::new();
        for _ in 0..3 {
            let mut c = Fighter::new(Side::Attacker, 80);
            c.back = true;
            c.combat_limit = 75;
            fs.push(c);
        }
        fs.push(Fighter::new(Side::Defender, 120));
        fs.push(Fighter::new(Side::Defender, 120));
        let b = Battle::new(Params::default(), fs, |h, _| if h < 3 { Pair { a: 900, dv: 100 } } else { Pair { a: 1, dv: 1_000_000 } });
        let e = resolve_expected(&b);
        assert_eq!(e.def_loss_x1000, 0);
        assert!(e.att_withdrawn_x1000 > 2900, "{e:?}");
        assert!(e.rounds < 40);
        // The defenders are left at (about) the limit.
        for f in &e.fighters[3..] {
            assert!((f.hp_alive - 3000).abs() <= 300, "{f:?}");
        }
    }

    #[test]
    fn the_shield_spares_the_back_while_the_front_stands() {
        let mut archer = Fighter::new(Side::Defender, 80);
        archer.back = true;
        let fs = vec![Fighter::new(Side::Attacker, 120), Fighter::new(Side::Attacker, 120), Fighter::new(Side::Defender, 120), archer.clone()];
        let mut p = Params::default();
        p.max_rounds = 3;
        let e = resolve_expected(&Battle::new(p, fs, |_, _| Pair { a: 400, dv: 400 }));
        assert!(e.fighters[2].mu >= 5900 && e.fighters[3].mu <= 10, "{:?}", e.fighters);
        // A flanker ignores it: the archer takes its hits from the start.
        let mut fl = Fighter::new(Side::Attacker, 100);
        fl.ignores_shield = true;
        fl.order = vec![TargetClass::Screen, TargetClass::Back, TargetClass::Front];
        let fs = vec![fl, Fighter::new(Side::Defender, 120), archer];
        p.max_rounds = 1;
        let e = resolve_expected(&Battle::new(p, fs, |_, _| Pair { a: 400, dv: 400 }));
        assert_eq!((e.fighters[1].mu, e.fighters[2].mu), (0, 1000));
    }

    #[test]
    fn volleys_and_fear_weigh_the_hitters() {
        let mut a = Fighter::new(Side::Attacker, 120);
        a.first_strikes = 1;
        a.chance_first_strikes = 1;
        let b = Battle::new(Params::default(), vec![a, Fighter::new(Side::Defender, 120)], |_, _| Pair { a: 400, dv: 400 });
        let mut p = b.params;
        p.max_rounds = 0;
        let e = resolve_expected(&Battle::new(p, b.fighters.clone(), |_, _| Pair { a: 400, dv: 400 }));
        // One sure volley and half a chance one: 1.5 hits of 10 HP, none back.
        assert_eq!((e.volley_rounds, e.fighters[1].mu, e.fighters[0].mu), (2, 1500, 0));
        // A fighter sure to fail its morale check never fights.
        let mut a = Fighter::new(Side::Attacker, 120);
        a.fear_fail = ONE as i32;
        let e = resolve_expected(&Battle::new(Params::default(), vec![a, Fighter::new(Side::Defender, 120)], |_, _| Pair { a: 400, dv: 400 }));
        assert_eq!((e.att_loss_x1000, e.fighters[0].sat_out, e.rounds, e.win_permille), (0, ONE as i32, 0, 20));
    }

    #[test]
    fn a_carried_state_starts_where_it_left() {
        let b = uniform(3, 3, 1000, 1000);
        let fresh = resolve_expected(&b);
        let hurt = resolve_expected_from(&b, &[MomentStart::default(); 3].into_iter().chain([MomentStart { alive: 2048, mu: 6000, var: 4_000_000 }; 3]).collect::<Vec<_>>());
        assert!(hurt.att_loss_x1000 < fresh.att_loss_x1000 && hurt.margin > fresh.margin, "{fresh:?} {hurt:?}");
        assert!(hurt.def_loss_x1000 <= 1500, "half a defender each was already lost: {}", hurt.def_loss_x1000);
    }

    /// A random battle with extreme values: 1-20 a side, strengths 1..=10000 (x100), HP 1-200,
    /// volleys, immunity, withdrawal, and limits from 0 to 100 (all attackers capped at times).
    fn extreme_battle(rng: &mut Rng) -> Battle {
        let mut r = |n: i32| rng.get_i(n, "extreme");
        let mode = r(4);
        let sizes = [1 + r(20), 1 + r(20)];
        let mut fs = Vec::new();
        let mut st = Vec::new();
        for (si, side) in [Side::Attacker, Side::Defender].into_iter().enumerate() {
            for _ in 0..sizes[si] {
                let mut f = Fighter::new(side, 1 + r(200));
                f.hp = 1 + r(f.max_hp * 100);
                f.back = r(3) == 0;
                f.screen = r(5) == 0;
                f.ignores_shield = r(4) == 0;
                f.order = match r(4) {
                    0 => vec![TargetClass::Screen, TargetClass::Back, TargetClass::Front],
                    1 => vec![TargetClass::Back, TargetClass::Front],
                    2 => vec![TargetClass::Front],
                    _ => vec![TargetClass::Front, TargetClass::Back],
                };
                if mode != 3 {
                    f.first_strikes = r(3);
                    f.chance_first_strikes = r(3);
                }
                f.first_strike_immune = mode == 2 || r(5) == 0;
                f.withdraw_pct = if r(3) == 0 { r(101) } else { 0 };
                f.combat_limit = if side == Side::Attacker && (mode == 1 || r(4) == 0) { r(101) } else { 100 };
                f.fear_fail = if r(6) == 0 { r(4097) } else { 0 };
                st.push((1 + r(1_000_000), 1 + r(1_000_000)));
                fs.push(f);
            }
        }
        Battle::new(Params::default(), fs, |h, t| Pair { a: st[h].0, dv: st[t].1 })
    }

    #[test]
    fn every_battle_terminates() {
        let mut rng = Rng::new(31337);
        for i in 0..20_000 {
            let b = extreme_battle(&mut rng);
            let max_v = b.max_volleys();
            let log = resolve_sampled_quiet(&b, &mut Rng::new(mix(i, 1)));
            assert!(log.rounds <= b.params.max_rounds && log.volley_rounds <= max_v, "battle {i}: {} / {}", log.volley_rounds, log.rounds);
            let e = resolve_expected(&b);
            assert!(e.rounds <= b.params.max_rounds && e.volley_rounds <= max_v, "battle {i}: {} / {}", e.volley_rounds, e.rounds);
            for f in &e.fighters {
                assert!(f.alive >= 0 && f.withdrawn >= 0 && f.lost >= 0 && f.alive + f.withdrawn + f.lost + f.sat_out <= ONE as i32 + 8, "battle {i}: {f:?}");
            }
            assert!((20..=980).contains(&e.win_permille));
        }
    }
}
