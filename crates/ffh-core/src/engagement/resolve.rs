//! The sampled round model (plan 6.4 step 5 to 6.6): opening volleys, rounds, withdrawal, the
//! end, with every draw from the RNG passed in, and the per-round event log the battle report
//! and the battle UI replay.
//!
//! **What a battle does.** Each fighter's volleys are rolled (`first_strikes` + 0..=chance). Volley
//! round k: every engaged fighter with at least k volleys hits (first-strike-immune fighters are
//! not volley targets; with none left the volley is lost), the others do not hit. Then up to
//! `max_rounds` rounds in which every engaged fighter hits. A hit, in the interleaved slot order
//! (A0, D0, A1, D1, ...): the hitter takes the first class of its order with an available target
//! over the round-start state ([`targets`]: engaged enemies of that kind; back fighters are behind
//! the shield while an enemy front fighter is engaged, unless the hitter ignores it; an attacker
//! with a combat limit skips targets at its limit), a target uniformly in it ("Engagement
//! target", drawn only when there are two or more) and r ("Engagement hit"):
//! `max(1, mean x r / 100)` centi-HP, an attacker's hit cut so the target stays at its limit.
//! Damage applies at the end of the round, all at once; the hit that took a target to 0 in the
//! accumulation order has the kill. Then withdrawal: in the interleaved order, each fighter that
//! has not rolled, has a chance and ended the round below `withdraw_hp_pct` (or dead) rolls once
//! ("Engagement withdraw"); on a success it leaves alive, and if the round killed it, with its
//! HP from before the round (the round's hits on it and the kill are cancelled). An attacker with
//! a combat limit and no target left but ones at its limit leaves at the end of the round (melee
//! rounds; in a volley round its volley is lost). The battle ends when a side has no engaged
//! fighter (both at once: mutual), and after `max_rounds` rounds in a draw. Volley rounds have the
//! same end of round (damage, withdrawal, end) as rounds.
//!
//! **RNG.** In this order: volley counts (attackers, then defenders, slot order, only when the
//! chance is above 0); per volley round and round, per hitter in the interleaved order, a target
//! draw when there are two or more candidates, then r; the withdrawal rolls in the interleaved
//! order. Nothing else draws; the log records every roll, never a seed.

use super::battle::{Battle, BattleEffect, FighterIdx, Side, TargetClass};
use crate::rng::Rng;

/// The synchronized RNG, as the round model draws from it. The rules back it with the game's
/// `soren_rand` (the same draws as `soren_rand_num`); tests use a seeded [`Rng`].
pub trait BattleRng {
    /// A draw in `0..n` (`n >= 1`); `label` names it, as `getSorenRandNum`'s log string.
    fn rand(&mut self, n: i32, label: &'static str) -> i32;
}

impl BattleRng for Rng {
    fn rand(&mut self, n: i32, label: &'static str) -> i32 {
        self.get_i(n, label)
    }
}

/// How an engagement ended (plan 6.6).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EngageResult {
    /// The defender has no engaged fighter left; the attacker has.
    AttackerWon,
    /// The attacker has no engaged fighter left; the defender has.
    DefenderHeld,
    /// `max_rounds` rounds fought (or nobody fought): both sides disengage.
    Draw,
    /// Neither side has an engaged fighter left (the last of both fell or left in one round).
    Mutual,
}

/// When something happened: the opening (battle effects before the volleys), volley round k, or
/// round n (both from 1).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase {
    Opening,
    Volley(i32),
    Round(i32),
}

/// What became of a fighter.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Engaged at the end (the winner's survivors, and both sides after a draw).
    Survived,
    /// Killed; `by` had the killing hit (a damage effect without a fighter: `None`).
    Killed { by: Option<FighterIdx>, phase: Phase },
    /// Left alive: by its withdrawal roll, or (`capped`) as an attacker whose every target was at
    /// its combat limit.
    Withdrew { phase: Phase, capped: bool },
    /// Failed the morale check (`Fighter::sat_out`): never fought.
    SatOut,
}

/// Why a hitter did not hit.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NoTarget {
    /// A volley round, and every available target is first-strike immune: the volley is lost.
    Immune,
    /// An attacker with a combat limit: every target it may hit is at its limit (it leaves at the
    /// end of a melee round).
    Capped,
    /// No enemy in any class of its order.
    None,
}

/// One hit (the battle UI's "who hit whom, the damage and why").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// A volley (`Phase::Volley`) or a round's hit.
    pub phase: Phase,
    pub hitter: FighterIdx,
    pub target: FighterIdx,
    /// The class of the hitter's order it hit in.
    pub class: TargetClass,
    /// The candidates of that class (the target was drawn uniformly among them).
    pub candidates: Vec<FighterIdx>,
    /// Whether the enemy's back fighters were behind the shield for this hitter.
    pub shield_up: bool,
    /// The hitter's A and the target's Dv for the pair (x100), and the mean damage they give.
    pub a: i32,
    pub dv: i32,
    pub mean: i32,
    /// r, in percent.
    pub r: i32,
    /// The damage dealt, in centi-HP (after a combat-limit cut).
    pub dmg: i32,
    /// Cut by the hitter's combat limit.
    pub capped: bool,
    /// This hit took the target to 0 in the accumulation order (the kill credit, unless a
    /// fatal-round withdrawal cancels it).
    pub lethal: bool,
}

/// A death at the end of a round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Death {
    pub fighter: FighterIdx,
    pub killer: Option<FighterIdx>,
    /// The other fighters that damaged it in the battle, in fighter order.
    pub assists: Vec<FighterIdx>,
}

/// The battle log, in resolution order. Every `Hit` of a round comes before that round's
/// `Withdraw`s and its `RoundEnd`; HP changes only at `Effect` and `RoundEnd`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BattleEvent {
    /// A `BattleEffect::Damage` (index into `Battle::effects`) applied before the volleys.
    Effect { effect: u16, fighter: FighterIdx, dmg: i32, hp: i32 },
    /// A fighter's volleys: `first_strikes` plus the roll.
    Volleys { fighter: FighterIdx, first_strikes: i32, rolled: i32 },
    /// A volley round or a round begins.
    RoundStart { phase: Phase },
    Hit(Hit),
    NoTarget { hitter: FighterIdx, why: NoTarget },
    /// A withdrawal roll at the end of a round (`roll < chance` succeeds). `fatal`: the round
    /// would have killed it, and on a success it keeps its HP from before the round.
    Withdraw { fighter: FighterIdx, chance: i32, roll: i32, ok: bool, fatal: bool },
    /// The round's damage applied: every fighter's HP after it (centi-HP; the dead at 0 or
    /// below), its deaths in kill order, and the attackers that left at their combat limit.
    RoundEnd { phase: Phase, hp: Vec<i32>, deaths: Vec<Death>, left: Vec<FighterIdx> },
    End { result: EngageResult, volley_rounds: i32, rounds: i32 },
}

/// A kill, in kill order (round by round, in the accumulation order of the killing hits).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Kill {
    pub victim: FighterIdx,
    pub killer: Option<FighterIdx>,
    pub phase: Phase,
}

/// A fighter at the end of the battle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FighterEnd {
    /// HP left, in centi-HP (0 or below: dead).
    pub hp: i32,
    pub fate: Fate,
    pub volleys: i32,
    /// Hits it dealt and the damage they did (centi-HP, overkill excluded: a hit counts only
    /// down to the target's 0).
    pub hits: i32,
    pub dealt: i32,
    /// Damage it took (centi-HP, overkill excluded: its HP at the start less its HP left, 0 for
    /// the dead).
    pub taken: i32,
}

/// The outcome of [`resolve_sampled`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleLog {
    pub result: EngageResult,
    pub volley_rounds: i32,
    pub rounds: i32,
    pub fighters: Vec<FighterEnd>,
    pub kills: Vec<Kill>,
    /// `n * n`: damage dealt by `h` to `t` (centi-HP; the shares for battle XP), overkill
    /// excluded: a hit counts only down to the target's 0, so one landing on a target another hit
    /// of the round already took to 0 counts nothing.
    dealt: Vec<i32>,
    /// Empty when resolved quietly ([`resolve_sampled_quiet`]).
    pub events: Vec<BattleEvent>,
}

impl BattleLog {
    /// The damage `h` dealt `t` in the battle (centi-HP), cancelled hits and overkill excluded.
    pub fn dealt(&self, h: usize, t: usize) -> i32 {
        self.dealt[h * self.fighters.len() + t]
    }

    /// Fighters lost (killed) on each side, (attacker, defender).
    pub fn losses(&self, b: &Battle) -> (usize, usize) {
        let mut l = (0, 0);
        for (f, e) in b.fighters.iter().zip(&self.fighters) {
            if matches!(e.fate, Fate::Killed { .. }) {
                if f.side == Side::Attacker {
                    l.0 += 1
                } else {
                    l.1 += 1
                }
            }
        }
        l
    }
}

/// The round-start state targeting reads.
pub struct TargetState<'a> {
    /// Alive, not withdrawn, not sat out.
    pub engaged: &'a [bool],
    /// HP (centi-HP), for the combat-limit skip.
    pub hp: &'a [i32],
}

/// The class and candidates of hitter `h`'s next hit, or why it has none.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Targets {
    Class(TargetClass),
    Miss(NoTarget),
}

/// Hitter `h`'s targets over the round-start state (plan 6.5 step 1): the first class of its
/// order (made disjoint, [`Battle`]) holding an available enemy: engaged, not behind the shield
/// (a back fighter while an enemy front fighter is engaged, unless `h` ignores the shield), not
/// first-strike immune in a volley round, not at `h`'s combat limit. The candidates go to `out`.
/// The estimator reads the same class table, shield and limit rules over its weights.
pub fn targets(b: &Battle, h: usize, st: &TargetState, volley: bool, out: &mut Vec<FighterIdx>) -> Targets {
    let enemy = b.fighters[h].side.other();
    let shield_up = !b.fighters[h].ignores_shield && b.front(enemy).iter().any(|&j| st.engaged[j as usize]);
    let (mut saw_capped, mut saw_immune) = (false, false);
    for entry in b.classes(h) {
        out.clear();
        for &j in &entry.members {
            let ju = j as usize;
            if !st.engaged[ju] || (shield_up && b.fighters[ju].back) {
                continue;
            }
            if volley && b.fighters[ju].first_strike_immune {
                saw_immune = true;
                continue;
            }
            if let Some(floor) = b.cap_floor(h, ju) {
                if st.hp[ju] <= floor {
                    saw_capped = true;
                    continue;
                }
            }
            out.push(j);
        }
        if !out.is_empty() {
            return Targets::Class(entry.class);
        }
    }
    out.clear();
    Targets::Miss(if saw_capped {
        NoTarget::Capped
    } else if saw_immune {
        NoTarget::Immune
    } else {
        NoTarget::None
    })
}

/// Resolves the battle with the RNG `rng`, recording every event.
pub fn resolve_sampled(b: &Battle, rng: &mut impl BattleRng) -> BattleLog {
    Run::new(b, true).run(rng)
}

/// [`resolve_sampled`] without the event log (the same draws and outcome): the calibration
/// battery and anything else that only needs the outcome.
pub fn resolve_sampled_quiet(b: &Battle, rng: &mut impl BattleRng) -> BattleLog {
    Run::new(b, false).run(rng)
}

struct Run<'a> {
    b: &'a Battle,
    n: usize,
    record: bool,
    hp: Vec<i32>,
    engaged: Vec<bool>,
    rolled: Vec<bool>,
    fate: Vec<Option<Fate>>,
    volleys: Vec<i32>,
    hits: Vec<i32>,
    dealt: Vec<i32>,
    kills: Vec<Kill>,
    events: Vec<BattleEvent>,
    // Per-round scratch.
    pending: Vec<i32>,
    killer: Vec<Option<(FighterIdx, u32)>>,
    /// The round's hits: hitter, target, effective damage (overkill excluded).
    round_hits: Vec<(FighterIdx, FighterIdx, i32)>,
    leaving: Vec<bool>,
    cand: Vec<FighterIdx>,
}

impl<'a> Run<'a> {
    fn new(b: &'a Battle, record: bool) -> Run<'a> {
        let n = b.fighters.len();
        Run {
            b,
            n,
            record,
            hp: b.fighters.iter().map(|f| f.hp).collect(),
            engaged: b.fighters.iter().map(|f| !f.sat_out).collect(),
            rolled: vec![false; n],
            fate: b.fighters.iter().map(|f| f.sat_out.then_some(Fate::SatOut)).collect(),
            volleys: vec![0; n],
            hits: vec![0; n],
            dealt: vec![0; n * n],
            kills: Vec::new(),
            events: Vec::new(),
            pending: vec![0; n],
            killer: vec![None; n],
            round_hits: Vec::new(),
            leaving: vec![false; n],
            cand: Vec::with_capacity(n),
        }
    }

    fn push(&mut self, e: BattleEvent) {
        if self.record {
            self.events.push(e);
        }
    }

    fn run(mut self, rng: &mut impl BattleRng) -> BattleLog {
        let b = self.b;
        // Battle effects: damage before the volleys (phase 7's opening spells).
        for (i, e) in b.effects.iter().enumerate() {
            if let BattleEffect::Damage { fighter, pct, by } = *e {
                let f = fighter as usize;
                if self.hp[f] <= 0 {
                    continue;
                }
                let dmg = (pct.max(0) as i64 * b.fighters[f].max_hp as i64).min(i32::MAX as i64) as i32;
                if let Some(h) = by {
                    self.dealt[h as usize * self.n + f] += dmg.min(self.hp[f]);
                }
                self.hp[f] -= dmg;
                self.push(BattleEvent::Effect { effect: i as u16, fighter, dmg, hp: self.hp[f] });
                if self.hp[f] <= 0 {
                    self.engaged[f] = false;
                    self.fate[f] = Some(Fate::Killed { by, phase: Phase::Opening });
                    self.kills.push(Kill { victim: fighter, killer: by, phase: Phase::Opening });
                }
            }
        }
        // Volley counts: attackers, then defenders, in slot order.
        for side in [Side::Attacker, Side::Defender] {
            for f in 0..self.n {
                let fi = &b.fighters[f];
                if fi.side != side || !self.engaged[f] {
                    continue;
                }
                let fs = fi.first_strikes.max(0);
                let rolled = if fi.chance_first_strikes > 0 { rng.rand(fi.chance_first_strikes + 1, "First Strike") } else { 0 };
                self.volleys[f] = fs + rolled;
                if fs + rolled > 0 || fi.chance_first_strikes > 0 {
                    self.push(BattleEvent::Volleys { fighter: f as FighterIdx, first_strikes: fs, rolled });
                }
            }
        }
        let (mut volley_rounds, mut rounds) = (0, 0);
        let mut result = self.decided();
        if result.is_none() {
            let max_v = self.volleys.iter().copied().max().unwrap_or(0);
            for k in 1..=max_v {
                if !(0..self.n).any(|f| self.engaged[f] && self.volleys[f] >= k) {
                    break;
                }
                volley_rounds = k;
                result = self.round(rng, Phase::Volley(k), Some(k));
                if result.is_some() {
                    break;
                }
            }
        }
        if result.is_none() {
            for r in 1..=b.params.max_rounds {
                rounds = r;
                result = self.round(rng, Phase::Round(r), None);
                if result.is_some() {
                    break;
                }
            }
        }
        let result = result.unwrap_or(EngageResult::Draw);
        self.push(BattleEvent::End { result, volley_rounds, rounds });
        let fighters = (0..self.n)
            .map(|f| FighterEnd {
                hp: self.hp[f],
                fate: self.fate[f].unwrap_or(Fate::Survived),
                volleys: self.volleys[f],
                hits: self.hits[f],
                dealt: (0..self.n).map(|t| self.dealt[f * self.n + t]).sum(),
                taken: b.fighters[f].hp.max(0) - self.hp[f].max(0),
            })
            .collect();
        BattleLog { result, volley_rounds, rounds, fighters, kills: self.kills, dealt: self.dealt, events: self.events }
    }

    /// The result if a side has no engaged fighter (nobody on either side: a draw before the
    /// first round, a mutual end after one).
    fn decided(&self) -> Option<EngageResult> {
        let mut alive = [false; 2];
        for f in 0..self.n {
            if self.engaged[f] {
                alive[self.b.fighters[f].side.index()] = true;
            }
        }
        match alive {
            [true, true] => None,
            [true, false] => Some(EngageResult::AttackerWon),
            [false, true] => Some(EngageResult::DefenderHeld),
            [false, false] => Some(if self.kills.is_empty() && !self.fate.iter().any(|f| matches!(f, Some(Fate::Withdrew { .. }))) {
                EngageResult::Draw
            } else {
                EngageResult::Mutual
            }),
        }
    }

    /// One volley round (`volley = Some(k)`: only fighters with k volleys hit) or round.
    fn round(&mut self, rng: &mut impl BattleRng, phase: Phase, volley: Option<i32>) -> Option<EngageResult> {
        let b = self.b;
        let p = &b.params;
        self.push(BattleEvent::RoundStart { phase });
        self.pending.iter_mut().for_each(|x| *x = 0);
        self.killer.iter_mut().for_each(|x| *x = None);
        self.leaving.iter_mut().for_each(|x| *x = false);
        self.round_hits.clear();
        let mut seq = 0u32;
        for &h in b.hit_order() {
            let hu = h as usize;
            if !self.engaged[hu] || volley.is_some_and(|k| self.volleys[hu] < k) {
                continue;
            }
            let st = TargetState { engaged: &self.engaged, hp: &self.hp };
            let class = match targets(b, hu, &st, volley.is_some(), &mut self.cand) {
                Targets::Class(c) => c,
                Targets::Miss(why) => {
                    if why == NoTarget::Capped && volley.is_none() {
                        self.leaving[hu] = true;
                    }
                    self.push(BattleEvent::NoTarget { hitter: h, why });
                    continue;
                }
            };
            let shield_up = !b.fighters[hu].ignores_shield && b.front(b.fighters[hu].side.other()).iter().any(|&j| self.engaged[j as usize]);
            let nc = self.cand.len();
            let t = if nc > 1 { self.cand[rng.rand(nc as i32, "Engagement target") as usize] } else { self.cand[0] };
            let tu = t as usize;
            let r = p.rand_min + rng.rand(p.rand_max - p.rand_min + 1, "Engagement hit");
            let mean = b.mean_damage(hu, tu);
            let mut dmg = ((mean as i64 * r as i64) / 100).max(1).min(i32::MAX as i64) as i32;
            let mut capped = false;
            if let Some(floor) = b.cap_floor(hu, tu) {
                let room = self.hp[tu] - self.pending[tu] - floor;
                if dmg > room {
                    dmg = room.max(0);
                    capped = true;
                }
            }
            let before = self.hp[tu] - self.pending[tu];
            self.pending[tu] += dmg;
            let lethal = before > 0 && before - dmg <= 0;
            if lethal {
                self.killer[tu] = Some((h, seq));
            }
            seq += 1;
            self.hits[hu] += 1;
            self.round_hits.push((h, t, dmg.min(before.max(0))));
            if self.record {
                let pr = b.pair(hu, tu);
                self.events.push(BattleEvent::Hit(Hit {
                    phase,
                    hitter: h,
                    target: t,
                    class,
                    candidates: self.cand.clone(),
                    shield_up,
                    a: pr.a,
                    dv: pr.dv,
                    mean,
                    r,
                    dmg,
                    capped,
                    lethal,
                }));
            }
        }
        // Damage, then the withdrawal rolls.
        let mut new_hp: Vec<i32> = (0..self.n).map(|f| self.hp[f] - self.pending[f]).collect();
        let mut withdrew = vec![false; self.n];
        let mut saved = vec![false; self.n];
        for &f in b.hit_order() {
            let fu = f as usize;
            let fi = &b.fighters[fu];
            if !self.engaged[fu] || self.rolled[fu] || fi.withdraw_pct <= 0 {
                continue;
            }
            if new_hp[fu] > 0 && new_hp[fu] >= fi.withdraw_threshold(p) {
                continue;
            }
            self.rolled[fu] = true;
            let roll = rng.rand(100, "Engagement withdraw");
            let ok = roll < fi.withdraw_pct;
            let fatal = new_hp[fu] <= 0;
            if ok {
                withdrew[fu] = true;
                if fatal {
                    new_hp[fu] = self.hp[fu];
                    saved[fu] = true;
                }
            }
            self.push(BattleEvent::Withdraw { fighter: f, chance: fi.withdraw_pct, roll, ok, fatal });
        }
        for &(h, t, dmg) in &self.round_hits {
            if !saved[t as usize] {
                self.dealt[h as usize * self.n + t as usize] += dmg;
            }
        }
        self.hp = new_hp;
        let mut dead: Vec<(u32, usize)> = Vec::new();
        for f in 0..self.n {
            if self.engaged[f] && self.hp[f] <= 0 {
                dead.push((self.killer[f].map_or(u32::MAX, |k| k.1), f));
            }
        }
        dead.sort_unstable();
        let mut deaths = Vec::new();
        for &(_, f) in &dead {
            let by = self.killer[f].map(|k| k.0);
            self.engaged[f] = false;
            self.fate[f] = Some(Fate::Killed { by, phase });
            self.kills.push(Kill { victim: f as FighterIdx, killer: by, phase });
            if self.record {
                let assists = (0..self.n).filter(|&h| Some(h as FighterIdx) != by && self.dealt[h * self.n + f] > 0).map(|h| h as FighterIdx).collect();
                deaths.push(Death { fighter: f as FighterIdx, killer: by, assists });
            }
        }
        let mut left = Vec::new();
        for f in 0..self.n {
            if !self.engaged[f] {
                continue;
            }
            if withdrew[f] {
                self.engaged[f] = false;
                self.fate[f] = Some(Fate::Withdrew { phase, capped: false });
            } else if self.leaving[f] {
                self.engaged[f] = false;
                self.fate[f] = Some(Fate::Withdrew { phase, capped: true });
                left.push(f as FighterIdx);
            }
        }
        if self.record {
            self.events.push(BattleEvent::RoundEnd { phase, hp: self.hp.clone(), deaths, left });
        }
        self.decided()
    }
}

#[cfg(test)]
mod tests {
    use super::super::battle::{Fighter, Pair, Params};
    use super::*;

    const EQUAL: Pair = Pair { a: 400, dv: 400 };

    fn att(hp: i32) -> Fighter {
        Fighter::new(Side::Attacker, hp)
    }

    fn def(hp: i32) -> Fighter {
        Fighter::new(Side::Defender, hp)
    }

    fn back(mut f: Fighter) -> Fighter {
        f.back = true;
        f
    }

    fn battle(fs: Vec<Fighter>) -> Battle {
        Battle::new(Params::default(), fs, |_, _| EQUAL)
    }

    fn hits(log: &BattleLog) -> impl Iterator<Item = (&Hit, Phase)> {
        let mut phase = Phase::Opening;
        log.events.iter().filter_map(move |e| match e {
            BattleEvent::RoundStart { phase: p } => {
                phase = *p;
                None
            }
            BattleEvent::Hit(h) => Some((h, phase)),
            _ => None,
        })
    }

    /// An RNG that records the labels of its draws.
    struct Labels(Rng, Vec<&'static str>);

    impl BattleRng for Labels {
        fn rand(&mut self, n: i32, label: &'static str) -> i32 {
            self.1.push(label);
            self.0.rand(n, label)
        }
    }

    #[test]
    fn equal_strength_hits_ten_hp() {
        let b = battle(vec![att(120), def(120)]);
        assert_eq!(b.mean_damage(0, 1), 1000);
        let (mut sum, mut n) = (0i64, 0i64);
        for seed in 0..500 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            for (h, _) in hits(&log) {
                assert!((80..=120).contains(&h.r), "r {}", h.r);
                assert_eq!(h.dmg, h.mean * h.r / 100);
                sum += h.dmg as i64;
                n += 1;
            }
        }
        let mean = sum / n;
        assert!((990..=1010).contains(&mean), "mean hit {mean} centi-HP");
        // A hopeless hitter still scratches: at least 1 centi-HP.
        let b = Battle::new(Params::default(), vec![att(120), def(120)], |h, _| if h == 0 { Pair { a: 1, dv: 1_000_000 } } else { EQUAL });
        assert_eq!(b.mean_damage(0, 1), 0);
        let log = resolve_sampled(&b, &mut Rng::new(1));
        assert!(hits(&log).filter(|(h, _)| h.hitter == 0).all(|(h, _)| h.dmg == 1));
    }

    #[test]
    fn damage_is_simultaneous_and_both_can_die() {
        let mut a = att(120);
        a.hp = 500;
        let mut d = def(120);
        d.hp = 500;
        let b = battle(vec![a, d]);
        let log = resolve_sampled(&b, &mut Rng::new(3));
        assert_eq!(log.result, EngageResult::Mutual);
        assert_eq!(log.rounds, 1);
        assert_eq!(log.fighters[0].fate, Fate::Killed { by: Some(1), phase: Phase::Round(1) });
        assert_eq!(log.fighters[1].fate, Fate::Killed { by: Some(0), phase: Phase::Round(1) });
        assert_eq!(log.kills.len(), 2);
    }

    #[test]
    fn kill_credit_goes_to_the_hit_that_crosses_zero() {
        // A0 hits first (A0, D0, A1): never enough alone, so A1's hit is the kill.
        let mut d = def(120);
        d.hp = 1500;
        let b = battle(vec![att(120), d, att(120)]);
        for seed in 0..50 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            assert_eq!(log.result, EngageResult::AttackerWon);
            assert_eq!(log.kills, vec![Kill { victim: 1, killer: Some(2), phase: Phase::Round(1) }]);
            assert!(log.dealt(0, 1) > 0 && log.dealt(2, 1) > 0);
            let deaths: Vec<&Death> = log.events.iter().filter_map(|e| if let BattleEvent::RoundEnd { deaths, .. } = e { Some(deaths) } else { None }).flatten().collect();
            assert_eq!(deaths, vec![&Death { fighter: 1, killer: Some(2), assists: vec![0] }]);
        }
    }

    #[test]
    fn overkill_is_not_damage_dealt() {
        // A 1-HP defender: A0 (first in the order A0, D0, A1) kills it with a 10-HP hit, 1 HP of
        // which is damage; A1's hit lands on a target already at 0 and counts nothing, so A1 is
        // no assist and A0 has the whole XP share.
        let mut d = def(120);
        d.hp = 100;
        let b = battle(vec![att(120), d, att(120)]);
        for seed in 0..20 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            assert_eq!(log.kills, vec![Kill { victim: 1, killer: Some(0), phase: Phase::Round(1) }]);
            let on_d: Vec<i32> = hits(&log).filter(|(h, _)| h.target == 1).map(|(h, _)| h.dmg).collect();
            assert!(on_d.len() == 2 && on_d.iter().all(|&x| x >= 800), "both hits land in full in the log: {on_d:?}");
            assert_eq!((log.dealt(0, 1), log.dealt(2, 1)), (100, 0));
            assert_eq!((log.fighters[0].dealt, log.fighters[2].dealt, log.fighters[1].taken), (100, 0, 100));
            let deaths: Vec<&Death> = log.events.iter().filter_map(|e| if let BattleEvent::RoundEnd { deaths, .. } = e { Some(deaths) } else { None }).flatten().collect();
            assert_eq!(deaths, vec![&Death { fighter: 1, killer: Some(0), assists: vec![] }]);
        }
        // A damage effect counts down to 0 too.
        let mut d = def(120);
        d.hp = 100;
        let mut b = battle(vec![att(120), d]);
        b.add_effects([BattleEffect::Damage { fighter: 1, pct: 50, by: Some(0) }]);
        let log = resolve_sampled(&b, &mut Rng::new(1));
        assert_eq!((log.dealt(0, 1), log.fighters[1].taken), (100, 100));
    }

    #[test]
    fn kills_are_in_accumulation_order() {
        // Two weak defenders: A1 (second hitter) and A0 each kill one; the order follows the hits.
        let mut d0 = def(120);
        d0.hp = 100;
        let mut d1 = def(120);
        d1.hp = 100;
        let b = battle(vec![att(120), d0, att(120), d1]);
        for seed in 0..50 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            let lethal: Vec<(FighterIdx, FighterIdx)> = hits(&log).filter(|(h, _)| h.lethal).map(|(h, _)| (h.hitter, h.target)).collect();
            let kills: Vec<(FighterIdx, FighterIdx)> = log.kills.iter().map(|k| (k.killer.unwrap(), k.victim)).collect();
            assert_eq!(lethal, kills);
        }
    }

    #[test]
    fn the_front_shields_the_back() {
        // The front defender cannot be hurt; the frontline attacker never reaches the archer.
        let b = Battle::new(Params::default(), vec![att(120), def(120), back(def(80))], |h, t| if h == 0 && t == 1 { Pair { a: 1, dv: 1_000_000 } } else { EQUAL });
        for seed in 0..20 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            for (h, _) in hits(&log).filter(|(h, _)| h.hitter == 0) {
                assert_eq!((h.target, h.class, h.shield_up), (1, TargetClass::Front, true));
            }
        }
        // Once the front is gone, the back is hit.
        let mut front = def(120);
        front.hp = 100;
        let b = battle(vec![att(120), front, back(def(80))]);
        let log = resolve_sampled(&b, &mut Rng::new(5));
        let first_back = hits(&log).find(|(h, _)| h.hitter == 0 && h.target == 2).expect("the archer is reached");
        assert_eq!((first_back.0.class, first_back.1), (TargetClass::Back, Phase::Round(2)));
    }

    #[test]
    fn flankers_hit_screens_first_and_ignore_the_shield() {
        let mut fl = att(100);
        fl.ignores_shield = true;
        fl.order = vec![TargetClass::Screen, TargetClass::Back, TargetClass::Front];
        let mut screen = def(120);
        screen.screen = true;
        let b = battle(vec![fl.clone(), def(120), screen, back(def(80))]);
        let log = resolve_sampled(&b, &mut Rng::new(9));
        let first = hits(&log).find(|(h, _)| h.hitter == 0).unwrap().0;
        assert_eq!((first.target, first.class, &first.candidates[..]), (2, TargetClass::Screen, &[2][..]));
        // No screen: the flanker goes for the back behind a living front.
        let b = battle(vec![fl, def(120), back(def(80))]);
        let log = resolve_sampled(&b, &mut Rng::new(9));
        let first = hits(&log).find(|(h, _)| h.hitter == 0).unwrap().0;
        assert_eq!((first.target, first.class, first.shield_up), (2, TargetClass::Back, false));
        // An assassin (back, then front, ignoring the shield) likewise.
        let mut asn = att(60);
        asn.ignores_shield = true;
        asn.order = vec![TargetClass::Back, TargetClass::Front];
        let b = battle(vec![asn, def(120), back(def(80))]);
        let log = resolve_sampled(&b, &mut Rng::new(9));
        assert_eq!(hits(&log).find(|(h, _)| h.hitter == 0).unwrap().0.target, 2);
    }

    #[test]
    fn targets_are_uniform_within_a_class() {
        let mut p = Params::default();
        p.max_rounds = 1;
        let b = Battle::new(p, vec![att(120), def(120), def(120), def(120)], |_, _| EQUAL);
        let mut count = [0; 4];
        for seed in 0..10_000 {
            let log = resolve_sampled(&b, &mut Rng::new(seed * 7919 + 1));
            count[hits(&log).find(|(h, _)| h.hitter == 0).unwrap().0.target as usize] += 1;
        }
        for &c in &count[1..] {
            assert!((3133..=3533).contains(&c), "{count:?}");
        }
    }

    #[test]
    fn volleys_take_no_return_damage() {
        let mut a = att(120);
        a.first_strikes = 2;
        let b = battle(vec![a, def(120)]);
        let log = resolve_sampled(&b, &mut Rng::new(4));
        assert_eq!(log.volley_rounds, 2);
        let volleys: Vec<&Hit> = hits(&log).filter(|(_, p)| matches!(p, Phase::Volley(_))).map(|(h, _)| h).collect();
        assert_eq!(volleys.len(), 2);
        assert!(volleys.iter().all(|h| h.hitter == 0));
        let after_volleys = log.events.iter().find_map(|e| match e {
            BattleEvent::RoundEnd { phase: Phase::Volley(2), hp, .. } => Some(hp.clone()),
            _ => None,
        });
        let hp = after_volleys.unwrap();
        assert_eq!(hp[0], 12000);
        assert_eq!(hp[1], 12000 - volleys[0].dmg - volleys[1].dmg);
        // Volley counts: the first strikes plus the roll, logged.
        assert_eq!(log.fighters[0].volleys, 2);
    }

    #[test]
    fn first_strike_immune_fighters_are_not_volley_targets() {
        let mut a = att(120);
        a.first_strikes = 1;
        let mut d = def(120);
        d.first_strike_immune = true;
        let b = battle(vec![a, d]);
        let log = resolve_sampled(&b, &mut Rng::new(4));
        assert!(log.events.contains(&BattleEvent::NoTarget { hitter: 0, why: NoTarget::Immune }));
        assert_eq!(hits(&log).filter(|(_, p)| matches!(p, Phase::Volley(_))).count(), 0);
        // An immune fighter next to a hittable one: the volley goes to the other.
        let mut d = def(120);
        d.first_strike_immune = true;
        let mut a = att(120);
        a.first_strikes = 1;
        let b = battle(vec![a, d, def(120)]);
        for seed in 0..20 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            let v = hits(&log).find(|(_, p)| matches!(p, Phase::Volley(_))).unwrap().0;
            assert_eq!((v.target, &v.candidates[..]), (2, &[2][..]));
        }
    }

    #[test]
    fn chance_volleys_are_rolled_first() {
        let mut a = att(120);
        a.chance_first_strikes = 2;
        let mut d = def(120);
        d.chance_first_strikes = 1;
        let b = battle(vec![a, d]);
        let mut rng = Labels(Rng::new(11), Vec::new());
        let log = resolve_sampled(&b, &mut rng);
        assert_eq!(&rng.1[..2], &["First Strike", "First Strike"]);
        let rolled: Vec<(FighterIdx, i32)> = log.events.iter().filter_map(|e| if let BattleEvent::Volleys { fighter, rolled, .. } = e { Some((*fighter, *rolled)) } else { None }).collect();
        assert_eq!(rolled.len(), 2);
        assert!(rolled[0].1 <= 2 && rolled[1].1 <= 1);
        // Then per hit: a target draw only with two or more candidates, then r.
        assert!(rng.1[2..].iter().all(|&l| l == "Engagement hit"));
    }

    #[test]
    fn withdrawal_is_rolled_once() {
        // A 50% withdrawer against a much stronger defender: one roll at most, whatever happens.
        let mut a = att(100);
        a.withdraw_pct = 50;
        let b = Battle::new(Params::default(), vec![a, def(120)], |h, _| if h == 0 { Pair { a: 100, dv: 900 } } else { Pair { a: 900, dv: 100 } });
        let mut outcomes = [0; 2];
        for seed in 0..200 {
            let log = resolve_sampled(&b, &mut Rng::new(seed));
            let rolls: Vec<&BattleEvent> = log.events.iter().filter(|e| matches!(e, BattleEvent::Withdraw { .. })).collect();
            assert_eq!(rolls.len(), 1, "seed {seed}");
            match log.fighters[0].fate {
                Fate::Withdrew { capped: false, .. } => outcomes[0] += 1,
                Fate::Killed { .. } => outcomes[1] += 1,
                f => panic!("{f:?}"),
            }
        }
        assert!(outcomes[0] > 60 && outcomes[1] > 60, "{outcomes:?}");
    }

    #[test]
    fn a_fatal_round_withdrawal_keeps_the_hp_from_before() {
        // Already below 40%: it rolls at the end of the first round, which also kills it.
        let mut a = att(120);
        a.hp = 500;
        a.withdraw_pct = 100;
        let b = battle(vec![a, def(120)]);
        let log = resolve_sampled(&b, &mut Rng::new(2));
        assert_eq!(log.fighters[0].fate, Fate::Withdrew { phase: Phase::Round(1), capped: false });
        assert_eq!(log.fighters[0].hp, 500);
        assert_eq!(log.dealt(1, 0), 0);
        assert!(log.kills.is_empty());
        assert_eq!(log.result, EngageResult::DefenderHeld);
        let ev = log.events.iter().find(|e| matches!(e, BattleEvent::Withdraw { .. })).unwrap();
        assert!(matches!(ev, BattleEvent::Withdraw { fighter: 0, ok: true, fatal: true, .. }));
        // The lethal hit is logged, but the kill is cancelled.
        assert!(hits(&log).any(|(h, _)| h.hitter == 1 && h.lethal));
    }

    #[test]
    fn a_combat_limit_caps_and_the_capped_attacker_leaves() {
        // A catapult (limit 75) that cannot be hurt: the defender ends at exactly 25%.
        let mut cat = back(att(80));
        cat.combat_limit = 75;
        let b = Battle::new(Params::default(), vec![cat, def(120)], |h, _| if h == 0 { Pair { a: 900, dv: 100 } } else { Pair { a: 1, dv: 1_000_000 } });
        let log = resolve_sampled(&b, &mut Rng::new(8));
        assert_eq!(log.fighters[1].hp, 25 * 120);
        assert!(matches!(log.fighters[0].fate, Fate::Withdrew { capped: true, .. }));
        assert_eq!(log.result, EngageResult::DefenderHeld);
        assert!(hits(&log).any(|(h, _)| h.capped));
        assert!(log.events.iter().any(|e| matches!(e, BattleEvent::NoTarget { hitter: 0, why: NoTarget::Capped })));
        // Behind a capped front the back is shielded: the catapult still leaves.
        let mut front = def(120);
        front.hp = 25 * 120;
        let mut cat = back(att(80));
        cat.combat_limit = 75;
        let b = battle(vec![cat, front, back(def(80))]);
        let log = resolve_sampled(&b, &mut Rng::new(8));
        assert!(matches!(log.fighters[0].fate, Fate::Withdrew { phase: Phase::Round(1), capped: true }));
    }

    #[test]
    fn a_defending_siege_unit_has_no_cap() {
        let mut cat = back(def(80));
        cat.combat_limit = 75;
        let b = Battle::new(Params::default(), vec![att(120), cat], |h, _| if h == 1 { Pair { a: 900, dv: 100 } } else { Pair { a: 1, dv: 1_000_000 } });
        let log = resolve_sampled(&b, &mut Rng::new(8));
        assert_eq!(log.result, EngageResult::DefenderHeld);
        assert!(matches!(log.fighters[0].fate, Fate::Killed { by: Some(1), phase: Phase::Round(_) }));
        assert!(hits(&log).all(|(h, _)| !h.capped));
    }

    #[test]
    fn max_rounds_is_a_draw() {
        let b = Battle::new(Params::default(), vec![att(120), def(120)], |_, _| Pair { a: 1, dv: 1_000_000 });
        let log = resolve_sampled(&b, &mut Rng::new(1));
        assert_eq!((log.result, log.rounds), (EngageResult::Draw, 40));
        assert!(log.fighters.iter().all(|f| f.fate == Fate::Survived));
        assert_eq!(log.events.last(), Some(&BattleEvent::End { result: EngageResult::Draw, volley_rounds: 0, rounds: 40 }));
    }

    #[test]
    fn sat_out_fighters_never_fight() {
        let mut a = att(120);
        a.sat_out = true;
        let b = battle(vec![a, att(120), def(120)]);
        let log = resolve_sampled(&b, &mut Rng::new(6));
        assert_eq!(log.fighters[0].fate, Fate::SatOut);
        assert!(hits(&log).all(|(h, _)| h.hitter != 0 && h.target != 0));
        // Every defender sat out: the attacker wins without a round (the rules keep the plot).
        let mut d = def(120);
        d.sat_out = true;
        let log = resolve_sampled(&battle(vec![att(120), d]), &mut Rng::new(6));
        assert_eq!((log.result, log.rounds, log.volley_rounds), (EngageResult::AttackerWon, 0, 0));
    }

    #[test]
    fn effects_apply_before_the_volleys() {
        let mut b = battle(vec![att(120), def(120), def(120)]);
        b.add_effects([
            BattleEffect::Mod { fighter: 0, a_pct: 100, dv_pct: 0 },
            BattleEffect::Damage { fighter: 2, pct: 100, by: Some(0) },
            BattleEffect::Cast { caster: 0, spell: 7 },
        ]);
        assert_eq!(b.pair(0, 1), Pair { a: 800, dv: 400 });
        assert_eq!(b.mean_damage(0, 1), 1333);
        let log = resolve_sampled(&b, &mut Rng::new(6));
        assert_eq!(log.kills[0], Kill { victim: 2, killer: Some(0), phase: Phase::Opening });
        assert!(matches!(log.events[0], BattleEvent::Effect { effect: 1, fighter: 2, dmg: 12000, hp: 0 }));
        assert!(hits(&log).all(|(h, _)| h.target != 2));
    }

    #[test]
    fn the_log_replays_the_hp() {
        let mut rng = Rng::new(99);
        for _ in 0..200 {
            let b = super::super::calibrate::random_battle(&mut rng);
            let log = resolve_sampled(&b, &mut rng);
            let mut hp: Vec<i32> = b.fighters.iter().map(|f| f.hp).collect();
            let mut pending = vec![0; b.len()];
            for e in &log.events {
                match e {
                    BattleEvent::Hit(h) => pending[h.target as usize] += h.dmg,
                    BattleEvent::RoundEnd { hp: after, .. } => {
                        for f in 0..b.len() {
                            let saved = after[f] == hp[f] && pending[f] > 0;
                            assert!(saved || after[f] == hp[f] - pending[f], "fighter {f}");
                        }
                        hp = after.clone();
                        pending.iter_mut().for_each(|x| *x = 0);
                    }
                    _ => {}
                }
            }
            assert_eq!(hp, log.fighters.iter().map(|f| f.hp).collect::<Vec<_>>());
            // The damage dealt to a fighter, overkill excluded, is the damage it took.
            for t in 0..b.len() {
                assert_eq!((0..b.len()).map(|h| log.dealt(h, t)).sum::<i32>(), log.fighters[t].taken, "fighter {t}");
            }
            // The losing side loses every fighter that did not withdraw.
            let loser = match log.result {
                EngageResult::AttackerWon => Some(Side::Defender),
                EngageResult::DefenderHeld => Some(Side::Attacker),
                _ => None,
            };
            for (f, e) in b.fighters.iter().zip(&log.fighters) {
                if Some(f.side) == loser {
                    assert!(!matches!(e.fate, Fate::Survived));
                }
            }
            // The quiet run draws the same.
            let mut r1 = Rng::new(5);
            let mut r2 = Rng::new(5);
            let (a, q) = (resolve_sampled(&b, &mut r1), resolve_sampled_quiet(&b, &mut r2));
            assert_eq!((a.result, a.rounds, &a.fighters, &a.kills), (q.result, q.rounds, &q.fighters, &q.kills));
            assert_eq!(r1, r2);
        }
    }

    #[test]
    fn the_same_seed_replays_the_same_battle() {
        let mut rng = Rng::new(1234);
        let b = super::super::calibrate::random_battle(&mut rng);
        let (l1, l2) = (resolve_sampled(&b, &mut Rng::new(42)), resolve_sampled(&b, &mut Rng::new(42)));
        assert_eq!(l1, l2);
        let others = (43..60).map(|s| resolve_sampled(&b, &mut Rng::new(s))).filter(|l| *l != l1).count();
        assert!(others > 10);
    }
}
