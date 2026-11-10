//! The prediction ledger (docs/CALIBRATION.md): every estimate the AI acts on (or declines to act
//! on) recorded as a prediction about something the rules decide, resolved by the rules, scored.
//!
//! A child module of `telemetry` (it lives in the same `Tel`): it is on only when telemetry is,
//! it draws no random number and changes no game state, and it is not saved.
//!
//! - **Recording:** [`Game::tel_predict`] (or [`Game::tel_prediction`] with a raw `score` or
//!   whether the AI acted on it): a kind (`"assault"`), an [`Outcome`] the rules can resolve, a
//!   horizon in turns, and the predicted chance (0..=100) that the outcome happens within it.
//! - **De-duplication:** one pending prediction per player, kind and subject (the outcome's city,
//!   unit, plot or teams): the first one made stays until it resolves, so an estimate the AI makes
//!   again every turn is one sample, not ten. One exception: on the turn it was made, a
//!   prediction the AI declined to act on gives way to the same one acted on (the decision of that
//!   turn is the one scored).
//! - **Resolution:** at the events themselves (a city changes hands, a combat or an engagement is
//!   fought), at the end of the predicting player's turn (this turn's predictions: a plot cleared,
//!   an attack that was not fought) and at the end of each game turn (horizons run out, cities
//!   lost). Each resolved prediction writes one `prediction` event and adds to the aggregates
//!   (`predictions` in [`Game::telemetry_json`]). A prediction whose subject vanished in a way
//!   that makes the question meaningless (the attacker never fought; the victim was destroyed by
//!   someone else) is `void`: counted, not scored.

use std::collections::BTreeMap;

use super::{ev, Obj, Tel};
use crate::game::Game;
use crate::ids::*;

/// What a prediction says will happen, in terms the rules resolve without any AI code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A player of `team` owns `city` (its plot: a captured city is a new city) within the horizon.
    CityCapturedBy { city: CityId, team: TeamId },
    /// The owner of `city` when the prediction is made no longer owns it within the horizon
    /// (captured, traded, flipped, razed or destroyed).
    CityLost { city: CityId },
    /// The next attack `attacker` makes wins: a duel it wins (the defender dies) or an
    /// engagement on its side that takes or clears the plot. False if it falls, or its side is
    /// repulsed; void if it withdraws, the engagement is held, or it fights no attack within the
    /// horizon.
    AttackWon { attacker: UnitId },
    /// No unit of a team at war with `team` able to defend stands on `plot` when the horizon ends
    /// (at the end of the predicting player's turn, for horizon 0).
    StackClears { team: TeamId, plot: PlotIdx },
    /// `team` conquers at least one city of `victim` within the horizon. Void if `victim` is
    /// destroyed by someone else first.
    AnyCityCapturedFrom { team: TeamId, victim: TeamId },
}

/// The horizon of a prediction about this turn: resolved at the end of the predicting player's
/// turn (or the game turn).
pub const HORIZON_THIS_TURN: i32 = 0;

/// A prediction to record (see [`Game::tel_prediction`]).
#[derive(Copy, Clone, Debug)]
pub struct Prediction {
    pub kind: &'static str,
    pub outcome: Outcome,
    /// Turns after the turn it is made within which the outcome counts.
    pub horizon: i32,
    /// The predicted chance, 0..=100, that the outcome happens within the horizon (a yes/no
    /// estimator: 100 or 0).
    pub pct: i32,
    /// The estimator's raw margin, when it has one (a power ratio, a margin of strength).
    pub score: Option<i64>,
    /// Whether the AI acted on the estimate (attacked, declared war, held a defender back).
    pub acted: Option<bool>,
}

impl Prediction {
    pub fn new(kind: &'static str, outcome: Outcome, horizon: i32, pct: i32) -> Prediction {
        Prediction { kind, outcome, horizon, pct, score: None, acted: None }
    }

    pub fn score(self, score: i64) -> Prediction {
        Prediction { score: Some(score), ..self }
    }

    pub fn acted(self, acted: bool) -> Prediction {
        Prediction { acted: Some(acted), ..self }
    }
}

/// What a pending prediction is about, as resolution needs it: cities by plot and owner (a city
/// that changes hands is a new city with a new id).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Subject {
    CityCapturedBy { plot: PlotIdx, team: TeamId },
    CityLost { plot: PlotIdx, owner: PlayerId },
    AttackWon { unit: UnitId },
    StackClears { team: TeamId, plot: PlotIdx },
    AnyCityCapturedFrom { team: TeamId, victim: TeamId },
}

struct Pending {
    turn: i32,
    horizon: i32,
    pct: i32,
    score: Option<i64>,
    acted: Option<bool>,
    ctx: String,
}

/// Per player and kind.
#[derive(Default)]
struct Stat {
    n: i64,
    void: i64,
    hits: i64,
    sum_pct: i64,
    /// Sum of (pct - 100 * outcome)^2: the Brier score is this / n / 10000.
    brier: i64,
    /// By predicted pct (0-9, 10-19, .., 90-100): [count, hits].
    bins: [[i64; 2]; 10],
}

type Key = (PlayerId, &'static str, Subject);

#[derive(Default)]
pub(super) struct Ledger {
    pending: BTreeMap<Key, Pending>,
    stats: BTreeMap<(PlayerId, &'static str), Stat>,
}

/// A verdict: `Some(true)`/`Some(false)` resolved, `None` void.
type Verdict = Option<bool>;

impl Ledger {
    /// The aggregates, by player and kind: `{"<p>": {"<kind>": {n, void, hits, sum_pct, brier,
    /// bins, open}}}`.
    pub(super) fn json(&self) -> String {
        let mut open: BTreeMap<(PlayerId, &'static str), i64> = BTreeMap::new();
        for &(p, kind, _) in self.pending.keys() {
            *open.entry((p, kind)).or_insert(0) += 1;
        }
        let mut keys: Vec<(PlayerId, &'static str)> = self.stats.keys().copied().collect();
        keys.extend(open.keys().copied().filter(|k| !self.stats.contains_key(k)));
        keys.sort();
        let mut by_player: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
        let empty = Stat::default();
        for k in keys {
            let s = self.stats.get(&k).unwrap_or(&empty);
            let bins: Vec<String> = s.bins.iter().map(|b| format!("[{},{}]", b[0], b[1])).collect();
            let o = Obj::new()
                .i("n", s.n)
                .i("void", s.void)
                .i("hits", s.hits)
                .i("sum_pct", s.sum_pct)
                .i("brier", s.brier)
                .raw("bins", &format!("[{}]", bins.join(",")))
                .i("open", open.get(&k).copied().unwrap_or(0))
                .end();
            let mut key = String::new();
            super::push_json_str(&mut key, k.1);
            by_player.entry(k.0).or_default().push(format!("{key}:{o}"));
        }
        let pl: Vec<String> = by_player.into_iter().map(|(p, v)| format!("\"{}\":{{{}}}", p.0, v.join(","))).collect();
        format!("{{{}}}", pl.join(","))
    }
}

impl Tel {
    /// Resolves the pending prediction `key` (if still pending): its event and its aggregates.
    fn resolve(&mut self, key: &Key, verdict: Verdict, turn: i32, civ: &str) {
        let Some(pd) = self.ledger.pending.remove(key) else { return };
        let (p, kind, _) = *key;
        let st = self.ledger.stats.entry((p, kind)).or_default();
        match verdict {
            None => st.void += 1,
            Some(hit) => {
                let o = if hit { 100 } else { 0 };
                st.n += 1;
                st.hits += hit as i64;
                st.sum_pct += pd.pct as i64;
                st.brier += ((pd.pct - o) as i64).pow(2);
                let bin = &mut st.bins[(pd.pct / 10).clamp(0, 9) as usize];
                bin[0] += 1;
                bin[1] += hit as i64;
            }
        }
        self.event(|| {
            let mut o = ev(turn, "prediction", p).s("kind", kind).s("civ", civ).i("turn_made", pd.turn as i64).i("turn_resolved", turn as i64).i("horizon", pd.horizon as i64).i("pct", pd.pct as i64);
            if let Some(s) = pd.score {
                o = o.i("score", s);
            }
            if let Some(a) = pd.acted {
                o = o.b("acted", a);
            }
            o = match verdict {
                Some(hit) => o.b("outcome", hit),
                None => o.raw("outcome", "null").b("void", true),
            };
            o.raw("ctx", if pd.ctx.is_empty() { "{}" } else { &pd.ctx }).end()
        });
    }
}

impl Game {
    /// Records a prediction by player `p` (only when telemetry is on): `kind` names the estimator,
    /// `outcome` what it predicts, `horizon_turns` the turns after this one within which it
    /// counts, `pct` its chance (0..=100). `ctx` builds a JSON object of facts to slice by later
    /// (called only when events are written, and only for a prediction kept: see the
    /// de-duplication rule in the module docs).
    pub fn tel_predict(&self, p: PlayerId, kind: &'static str, outcome: Outcome, horizon_turns: i32, pct: i32, ctx: impl FnOnce() -> String) {
        self.tel_prediction(p, Prediction::new(kind, outcome, horizon_turns, pct), ctx);
    }

    /// [`Game::tel_predict`] with the options of [`Prediction`] (a raw score, acted or not).
    pub fn tel_prediction(&self, p: PlayerId, pr: Prediction, ctx: impl FnOnce() -> String) {
        if !self.telemetry.on() {
            return;
        }
        let subject = match pr.outcome {
            Outcome::CityCapturedBy { city, team } => {
                let Some(c) = self.cities.get(city) else { return };
                let Some(plot) = self.map.plot_xy(c.x, c.y) else { return };
                Subject::CityCapturedBy { plot, team }
            }
            Outcome::CityLost { city } => {
                let Some(c) = self.cities.get(city) else { return };
                let Some(plot) = self.map.plot_xy(c.x, c.y) else { return };
                Subject::CityLost { plot, owner: c.owner }
            }
            Outcome::AttackWon { attacker } => {
                if !self.units.contains(attacker) {
                    return;
                }
                Subject::AttackWon { unit: attacker }
            }
            Outcome::StackClears { team, plot } => Subject::StackClears { team, plot },
            Outcome::AnyCityCapturedFrom { team, victim } => Subject::AnyCityCapturedFrom { team, victim },
        };
        let turn = self.game_turn();
        let key: Key = (p, pr.kind, subject);
        let keep = self.telemetry.with(|t| match t.ledger.pending.get(&key) {
            None => true,
            // A same-turn prediction declined gives way to the same one acted on.
            Some(old) => old.turn == turn && old.acted == Some(false) && pr.acted == Some(true),
        });
        if keep != Some(true) {
            return;
        }
        let has_events = self.telemetry.with(|t| t.events.is_some()) == Some(true);
        let ctx = if has_events { ctx() } else { String::new() };
        let pending = Pending { turn, horizon: pr.horizon.max(0), pct: pr.pct.clamp(0, 100), score: pr.score, acted: pr.acted, ctx };
        self.telemetry.with(|t| {
            t.ledger.pending.insert(key, pending);
        });
    }

    fn tel_predict_civ(&self, p: PlayerId) -> &'static str {
        self.player_get_civilization_type(p).map_or("?", |c| self.d.civilizations[c].base.type_.as_str())
    }

    /// Resolves the verdicts found (pending keys and what they resolve to).
    fn tel_predict_apply(&self, verdicts: Vec<(Key, Verdict)>) {
        if verdicts.is_empty() {
            return;
        }
        let turn = self.game_turn();
        let civs: Vec<&'static str> = verdicts.iter().map(|(k, _)| self.tel_predict_civ(k.0)).collect();
        self.telemetry.with(|t| {
            for ((key, verdict), civ) in verdicts.iter().zip(civs) {
                t.resolve(key, *verdict, turn, civ);
            }
        });
    }

    /// The pending keys (a copy, so the game can be read while judging them).
    fn tel_predict_pending(&self) -> Vec<(Key, i32, i32)> {
        self.telemetry.with(|t| t.ledger.pending.iter().map(|(k, pd)| (*k, pd.turn, pd.horizon)).collect()).unwrap_or_default()
    }

    /// City `c` is about to pass to `new_owner` (`player_acquire_city`, before the transfer).
    pub(super) fn tel_predict_city_changed(&self, new_owner: PlayerId, c: CityId, conquest: bool) {
        let plot = self.city_plot(c);
        let old = self.cities[c].owner;
        let (tn, to) = (self.players[new_owner].team, self.players[old].team);
        let mut verdicts = Vec::new();
        for (key, _, _) in self.tel_predict_pending() {
            let v = match key.2 {
                Subject::CityLost { plot: q, owner } if q == plot && owner == old => Some(true),
                Subject::CityCapturedBy { plot: q, team } if q == plot && team == tn => Some(true),
                Subject::AnyCityCapturedFrom { team, victim } if conquest && team == tn && victim == to => Some(true),
                _ => continue,
            };
            verdicts.push((key, v));
        }
        self.tel_predict_apply(verdicts);
    }

    /// A duel fought by `att` as the attacker (`tel_combat`).
    pub(super) fn tel_predict_duel(&self, att: UnitId, att_dead: bool, def_dead: bool) {
        let mut verdicts = Vec::new();
        for (key, _, _) in self.tel_predict_pending() {
            if key.2 == (Subject::AttackWon { unit: att }) {
                verdicts.push((key, if att_dead { Some(false) } else if def_dead { Some(true) } else { None }));
            }
        }
        self.tel_predict_apply(verdicts);
    }

    /// An engagement: each attacker that fought (`units`, with whether it fell) and whether the
    /// attacking side took or cleared the plot (`won`), was repulsed (`lost`) or neither.
    pub(super) fn tel_predict_engagement(&self, attackers: &[(UnitId, bool)], won: bool, lost: bool) {
        let pending = self.tel_predict_pending();
        if pending.is_empty() {
            return;
        }
        let mut verdicts = Vec::new();
        for (key, _, _) in pending {
            let Subject::AttackWon { unit } = key.2 else { continue };
            let Some(&(_, fell)) = attackers.iter().find(|&&(u, _)| u == unit) else { continue };
            verdicts.push((key, if fell || lost { Some(false) } else if won { Some(true) } else { None }));
        }
        self.tel_predict_apply(verdicts);
    }

    /// No unit of a team at war with `team` that can defend stands on `plot`.
    fn tel_predict_cleared(&self, team: TeamId, plot: PlotIdx) -> bool {
        !self.map.plots[plot].units.iter().any(|&u| {
            let ut = self.unit_get_team(u);
            ut != team && self.at_war(team, ut) && self.unit_can_defend(u, Some(plot))
        })
    }

    /// The end of player `p`'s turn: its predictions about this turn resolve (a plot cleared or
    /// not; an attack not fought is void).
    pub fn tel_predict_player_turn_end(&self, p: PlayerId) {
        if !self.telemetry.on() {
            return;
        }
        let turn = self.game_turn();
        let mut verdicts = Vec::new();
        for (key, made, horizon) in self.tel_predict_pending() {
            if key.0 != p || made + horizon > turn {
                continue;
            }
            match key.2 {
                Subject::StackClears { team, plot } => verdicts.push((key, Some(self.tel_predict_cleared(team, plot)))),
                Subject::AttackWon { .. } => verdicts.push((key, None)),
                _ => {}
            }
        }
        self.tel_predict_apply(verdicts);
    }

    /// The end of the game turn: what the state now settles, and the horizons that ran out.
    pub fn tel_predict_turn_end(&self) {
        if !self.telemetry.on() {
            return;
        }
        let turn = self.game_turn();
        let mut verdicts = Vec::new();
        for (key, made, horizon) in self.tel_predict_pending() {
            let expired = made + horizon <= turn;
            let city_at = |plot: PlotIdx| self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c));
            let v = match key.2 {
                Subject::CityLost { plot, owner } => match city_at(plot) {
                    Some(c) if self.cities[c].owner == owner => (expired).then_some(Some(false)),
                    _ => Some(Some(true)),
                },
                Subject::CityCapturedBy { plot, team } => match city_at(plot) {
                    Some(c) if self.players[self.cities[c].owner].team == team => Some(Some(true)),
                    None => Some(Some(false)),
                    Some(_) => expired.then_some(Some(false)),
                },
                Subject::AnyCityCapturedFrom { team, victim } => {
                    if !self.team_is_alive(victim) {
                        Some(None)
                    } else if !self.team_is_alive(team) {
                        Some(Some(false))
                    } else {
                        expired.then_some(Some(false))
                    }
                }
                Subject::StackClears { team, plot } => expired.then(|| Some(self.tel_predict_cleared(team, plot))),
                Subject::AttackWon { unit } => (expired || !self.units.contains(unit)).then_some(None),
            };
            if let Some(v) = v {
                verdicts.push((key, v));
            }
        }
        self.tel_predict_apply(verdicts);
    }
}
