//! The AI's battle estimator on the rules' own battle code (docs/CALIBRATION.md, "The battle
//! forecast"): a thin layer over [`Game::engage_forecast_if`] and [`Game::engage_forecast_waves`],
//! which play the real engagement (formations, morale, the opening, the resolver) many times on a
//! private RNG for a battle that need not be on the board yet, or for the battles a turn's
//! attacks fight one after another.
//!
//! - [`attack`]: one battle, our units attacking a plot from a plot next to it (the engagement
//!   one attack fights): the chance the plot falls, our line is wiped out or repulsed, and the
//!   losses on each side.
//! - [`attack_turn`], [`assault`]: the battles our units next to a plot (a city) fight in a
//!   turn, each plot's pool and then its reserve, until it falls.
//! - [`holds`]: the defensive question for one of our cities: each enemy stack we see that can
//!   reach it within [`HOLD_STRIKE_TURNS`] turns (and the union of one player's stacks), its
//!   attacks on the city as the garrison stands and with the units of ours that can get there
//!   first.
//!
//! What it reads is what the AI may read: an attack is forecast only on a plot whose units the
//! AI knows ([`crate::view::sees_units`], the fog rule of its attacks), against the defenders the
//! rules let the attack meet (never an invisible unit); a defense sees the enemy units the AI's
//! threat assessment sees ([`crate::view::known_threat`]).
//!
//! Pure and deterministic. The AI's attack decisions read it (switch `FORECAST`: the field rule
//! in `military.rs`, the strike in `campaign.rs`, through [`decide_turn`]), and every estimate is
//! recorded beside `combat_sim`'s in the prediction ledger (`predict.rs`). The
//! [`Cache`] (one per player and turn, kept by the controller and lent to the [`crate::view::View`])
//! remembers the forecasts of the turn by their inputs and a fingerprint of the state they read.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use ffh_core::engagement::{Forecast, ForecastSight, ForecastTurn, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::reach::Flood;

/// Battles played per forecast of an attack (the rules' preview plays
/// `ENGAGEMENT_PREVIEW_RUNS`, 200; a 60-run estimate of a 50% chance is within 6.5 points 1 time
/// in 1.5, ample for a decision and a third of the cost).
pub const RUNS: i32 = 60;
/// Battles played per forecast of a defense ([`holds`]: several per city).
pub const HOLD_RUNS: i32 = 40;
/// Enemy stacks within this many plots of the city count (the radius of `local_threat`, `peril`).
pub const HOLD_RADIUS: i32 = 3;
/// A stack counts if it can stand next to the city within this many of its turns (0: it can
/// attack on its next turn).
pub const HOLD_STRIKE_TURNS: i32 = 2;
/// At most this many stacks, the soonest first.
pub const HOLD_MAX_STACKS: usize = 6;
/// Reinforcements: our units within this many plots of the city ...
pub const REINFORCE_RADIUS: i32 = 6;
/// ... the nearest this many of them, each counted if it reaches the city before the stack can
/// strike.
pub const REINFORCE_K: usize = 6;

/// An attack's forecast, in per mille (chances) and hundredths (units).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Est {
    /// The plot falls (no defender left on it: the city is taken, the stack cleared).
    pub p_take: i32,
    /// The defender's line is wiped out (the plot may still hold a reserve).
    pub p_wipe: i32,
    /// Our line is wiped out.
    pub p_repulsed: i32,
    /// Expected units fallen: ours, theirs (x100).
    pub own_lost_x100: i32,
    pub enemy_lost_x100: i32,
    /// Committed fighters: ours, theirs (Sentinels not counted).
    pub n_att: i32,
    pub n_def: i32,
}

impl Est {
    pub fn of(f: &Forecast) -> Est {
        let n = |s: Side| f.per.iter().filter(|x| x.side == s).count() as i32;
        Est {
            p_take: f.falls_permille,
            p_wipe: f.wiped_permille,
            p_repulsed: f.repulsed_permille,
            own_lost_x100: f.lost_x100[0],
            enemy_lost_x100: f.lost_x100[1],
            n_att: n(Side::Attacker),
            n_def: n(Side::Defender),
        }
    }
}

/// The forecasts of one player's turn (module comment), with the counts telemetry reports.
#[derive(Default)]
pub struct Cache {
    /// The game turn the entries belong to.
    turn: Cell<i32>,
    /// Inputs -> (fingerprint of the state read, the forecast or `None` when refused).
    map: RefCell<BTreeMap<Vec<u32>, (u64, Option<Forecast>)>>,
    /// Forecasts asked for, and played (not found in the cache).
    pub calls: Cell<i64>,
    pub played: Cell<i64>,
    /// The game turn the incoming-attack estimate was last recorded (`predict::incoming`: once
    /// per player-turn).
    pub incoming_turn: Cell<Option<i32>>,
}

impl Cache {
    /// This cache for game turn `turn`: emptied when it belonged to another turn.
    pub fn for_turn(self, turn: i32) -> Cache {
        if self.turn.get() != turn {
            self.map.borrow_mut().clear();
            self.turn.set(turn);
        }
        self
    }
}

/// FNV-1a, 64 bits.
fn fnv(h: &mut u64, v: i64) {
    for b in v.to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// What a forecast reads that changes within a turn: every unit involved (and every unit on the
/// target): where it is, its health, moves, attacks, experience, level, promotions and
/// fortification; the target's owner and its city's defence damage.
fn fingerprint(g: &Game, units: &[UnitId], target: PlotIdx) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    fnv(&mut h, g.game_turn() as i64);
    let on_target = g.map.plots[target].units.iter().copied();
    for u in units.iter().copied().chain(on_target) {
        fnv(&mut h, u.0 as i64);
        let Some(x) = g.units.get(u) else {
            fnv(&mut h, -1);
            continue;
        };
        for v in [x.x, x.y, x.damage, x.moves, x.attacks_made, x.made_attack as i32, x.experience, x.level, x.fortify_turns, x.owner.0 as i32] {
            fnv(&mut h, v as i64);
        }
        fnv(&mut h, g.d.promotions.keys().filter(|&p| x.has_promotion[p]).count() as i64);
    }
    let plot = &g.map.plots[target];
    fnv(&mut h, plot.owner.map_or(-1, |o| o.0 as i64));
    fnv(&mut h, plot.plot_city.map_or(-1, |c| g.cities.get(c).map_or(-2, |c| c.defense_damage as i64)));
    h
}

/// Attacks on one plot: units and the plot next to it each group attacks from.
pub type Attacks = Vec<(Vec<UnitId>, PlotIdx)>;

/// The rules' forecast of `attackers` attacking `target` as if from `from` against what stands
/// there plus `extras`: one battle ([`Game::engage_forecast_if`]), no sight test (the callers
/// apply the AI's own), through `cache` when given. `None`: refused (no attacker could, nobody
/// to fight).
pub fn forecast(g: &Game, cache: Option<&Cache>, attackers: &[UnitId], from: PlotIdx, target: PlotIdx, extras: &[UnitId], runs: i32, when: ForecastTurn) -> Option<Forecast> {
    forecast_in(g, cache, false, &[(attackers.to_vec(), from)], target, extras, runs, when, WarTest::AtWar)
}

/// The rules' forecast of the battles `attacks` fight on `target` one after another in a turn
/// (each group's pool, then its reserve: [`Game::engage_forecast_waves`]), against what stands
/// there plus `extras`; as [`forecast`] otherwise.
pub fn forecast_turn(g: &Game, cache: Option<&Cache>, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, extras: &[UnitId], runs: i32, when: ForecastTurn) -> Option<Forecast> {
    forecast_in(g, cache, true, attacks, target, extras, runs, when, WarTest::AtWar)
}

/// [`forecast_turn`] under `war` ([`WarTest::DeclareWar`]: the battles once that war is declared,
/// for an attack weighed before the declaration: `invasion.rs`).
pub fn forecast_turn_war(g: &Game, cache: Option<&Cache>, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, extras: &[UnitId], runs: i32, when: ForecastTurn, war: WarTest) -> Option<Forecast> {
    forecast_in(g, cache, true, attacks, target, extras, runs, when, war)
}

#[allow(clippy::too_many_arguments)]
fn forecast_in(g: &Game, cache: Option<&Cache>, waves: bool, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, extras: &[UnitId], runs: i32, when: ForecastTurn, war: WarTest) -> Option<Forecast> {
    // Canonical inputs: the same units in any order are the same battle.
    let mut seen: Vec<UnitId> = Vec::new();
    let mut att: Vec<(Vec<UnitId>, PlotIdx)> = Vec::new();
    for (units, from) in attacks {
        let mut u: Vec<UnitId> = units.iter().copied().filter(|x| !seen.contains(x)).collect();
        u.sort();
        u.dedup();
        seen.extend(&u);
        if !u.is_empty() {
            att.push((u, *from));
        }
    }
    if att.is_empty() {
        return None;
    }
    let mut ext: Vec<UnitId> = extras.iter().copied().filter(|u| !seen.contains(u)).collect();
    ext.sort();
    ext.dedup();
    let play = || {
        if waves {
            g.engage_forecast_waves_war(&att, target, &ext, runs, ForecastSight::Defender, when, war).ok()
        } else {
            g.engage_forecast_if(&att[0].0, att[0].1, target, &ext, runs, ForecastSight::Defender, when).ok()
        }
    };
    let Some(cache) = cache else { return play() };
    cache.calls.set(cache.calls.get() + 1);
    let mut key: Vec<u32> = Vec::with_capacity(seen.len() + ext.len() + 2 * att.len() + 4);
    key.extend([waves as u32 + 2 * (when == ForecastTurn::Next) as u32, target.0, runs as u32]);
    if let WarTest::DeclareWar(t) = war {
        key.extend([u32::MAX - 2, t.0 as u32]);
    }
    for (u, from) in &att {
        key.push(from.0);
        key.extend(u.iter().map(|x| x.0));
        key.push(u32::MAX);
    }
    key.push(u32::MAX - 1);
    key.extend(ext.iter().map(|u| u.0));
    let involved: Vec<UnitId> = seen.iter().chain(&ext).copied().collect();
    let fp = fingerprint(g, &involved, target);
    if let Some((f, out)) = cache.map.borrow().get(&key) {
        if *f == fp {
            return out.clone();
        }
    }
    cache.played.set(cache.played.get() + 1);
    let out = play();
    cache.map.borrow_mut().insert(key, (fp, out.clone()));
    out
}

/// Our `units` next to `target` grouped by the plot they stand on, the plot with the most first
/// (then the lower plot index): the battles a strike from there fights, in that order.
pub fn by_plot(g: &Game, units: &[UnitId], target: PlotIdx) -> Attacks {
    let (tx, ty) = g.map.xy(target);
    let mut at: BTreeMap<PlotIdx, Vec<UnitId>> = BTreeMap::new();
    for &u in units {
        let Some(q) = g.unit_plot_opt(u) else { continue };
        let (x, y) = g.map.xy(q);
        if g.step_distance(x, y, tx, ty) == 1 {
            at.entry(q).or_default().push(u);
        }
    }
    let mut out: Attacks = at.into_iter().map(|(q, u)| (u, q)).collect();
    out.sort_by_key(|(u, q)| (std::cmp::Reverse(u.len()), *q));
    out
}

/// Our units attacking `target` this turn in the battles `attacks` fight one after another
/// ([`forecast_turn`]): [`Est`] (`p_take`: the plot falls by the last of them). `None` as
/// [`attack`].
pub fn attack_turn(g: &Game, cache: Option<&Cache>, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx) -> Option<Est> {
    let &u = attacks.iter().flat_map(|a| a.0.iter()).next()?;
    let team = g.players[g.units[u].owner].team;
    if !crate::view::sees_units(g, team, target) {
        return None;
    }
    forecast_turn(g, cache, attacks, target, &[], RUNS, ForecastTurn::Now).map(|f| Est::of(&f))
}

/// Our `units` attacking plot `target` from `from` (next to it): [`Est`]. `None` when the AI does
/// not know the units there ([`crate::view::sees_units`]) or the rules refuse the battle.
pub fn attack(g: &Game, cache: Option<&Cache>, units: &[UnitId], from: PlotIdx, target: PlotIdx) -> Option<Est> {
    attack_full(g, cache, units, from, target).map(|f| Est::of(&f))
}

/// [`attack`] with the whole forecast (each fighter's chance to fall).
pub fn attack_full(g: &Game, cache: Option<&Cache>, units: &[UnitId], from: PlotIdx, target: PlotIdx) -> Option<Forecast> {
    let &u = units.first()?;
    let team = g.players[g.units[u].owner].team;
    if !crate::view::sees_units(g, team, target) {
        return None;
    }
    forecast(g, cache, units, from, target, &[], RUNS, ForecastTurn::Now)
}

/// Our units assaulting `city` this turn ([`attack_turn`] on its plot; `p_take`: the city
/// taken), each group of `attacks` from its plot next to it.
pub fn assault(g: &Game, cache: Option<&Cache>, attacks: &[(Vec<UnitId>, PlotIdx)], city: CityId) -> Option<Est> {
    attack_turn(g, cache, attacks, g.city_plot(city))
}

/// Battles played in a decision's first look ([`decide_turn`]): a third of [`RUNS`]. The
/// forecast's seed does not depend on the runs, so these are the first runs of the full one.
pub const FAST_RUNS: i32 = 20;
/// A first look whose `p_take` is this close to one of the decision's bars (per mille; about two
/// standard errors of a 20-run estimate of a 50% chance) is played again with [`RUNS`].
pub const RECHECK_MARGIN: i32 = 200;

/// The forecast an attack decision reads (switch `FORECAST`): the battles `attacks` fight on
/// `target` this turn ([`attack_turn`]'s, with the whole forecast), played with [`FAST_RUNS`]
/// first and again with [`RUNS`] when the chance the plot falls is within [`RECHECK_MARGIN`] of
/// one of `bars` (per mille). `None` as [`attack_turn`]. With telemetry on, `checked` gets the
/// full forecast's `p_take` too (to count the first looks the full one would have overturned).
pub fn decide_turn(g: &Game, cache: Option<&Cache>, attacks: &[(Vec<UnitId>, PlotIdx)], target: PlotIdx, bars: &[i32], checked: Option<&Cell<i32>>) -> Option<Forecast> {
    let &u = attacks.iter().flat_map(|a| a.0.iter()).next()?;
    let team = g.players[g.units[u].owner].team;
    if !crate::view::sees_units(g, team, target) {
        return None;
    }
    let fast = forecast_turn(g, cache, attacks, target, &[], FAST_RUNS, ForecastTurn::Now)?;
    let near = bars.iter().any(|&b| (fast.falls_permille - b).abs() < RECHECK_MARGIN);
    if near || checked.is_some() {
        let full = forecast_turn(g, cache, attacks, target, &[], RUNS, ForecastTurn::Now);
        if let (Some(c), Some(f)) = (checked, full.as_ref()) {
            c.set(f.falls_permille);
        }
        if near {
            return full;
        }
    }
    Some(fast)
}

/// The defensive forecast of one of our cities ([`holds`]). Chances in per mille.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// An enemy stack can strike within [`HOLD_STRIKE_TURNS`].
    pub threat: bool,
    /// The city falls: the worst of the single stacks and the union, as the garrison stands ...
    pub p_fall_now: i32,
    /// ... and with the reinforcements that arrive in time.
    pub p_fall_reinforced: i32,
    /// The parts: the worst single stack, the union of one player's stacks (each, now and
    /// reinforced).
    pub single_now: i32,
    pub union_now: i32,
    pub single_reinforced: i32,
    pub union_reinforced: i32,
    /// Our units that would have to come (those that arrive before the soonest stack strikes).
    pub reinforcements: Vec<UnitId>,
    /// Turns until the soonest stack can attack (0: on its next turn).
    pub turns: i32,
    /// Stacks counted and their units.
    pub stacks: i32,
    pub enemy_units: i32,
    /// Forecasts made for this answer.
    pub forecasts: i32,
    /// The players whose stacks count, in order.
    pub owners: Vec<PlayerId>,
}

/// An enemy stack near the city: its units, owner, where it attacks from and when.
struct Stack {
    units: Vec<UnitId>,
    owner: PlayerId,
    from: PlotIdx,
    turns: i32,
}

/// Can enemy unit `u` take part in an attack on our city (as `p` sees it)? A unit we see, of a
/// team at war with us, a land unit able to attack, not an animal (animals do not enter owned
/// land), not a garrison staying in its own city.
pub(crate) fn raider(g: &Game, p: PlayerId, u: UnitId) -> bool {
    let team = g.players[p].team;
    let unit = &g.units[u];
    let their = g.players[unit.owner].team;
    if their == team || !g.at_war(team, their) || !crate::view::known_threat(g, p, u) {
        return false;
    }
    let info = &g.d.units[unit.unit_type];
    if info.animal || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || !g.unit_can_attack(u) {
        return false;
    }
    let at = g.unit_plot(u);
    !(g.map.plots[at].plot_city.is_some() && unit.ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE))
}

/// The stacks that threaten the city on `cp` (module comment): enemy units within
/// [`HOLD_RADIUS`] grouped by plot and owner, each with the first plot next to the city its
/// slowest unit reaches (the reach flood, as that unit moves, with full moves on its turn) and
/// the turns it takes; the soonest [`HOLD_MAX_STACKS`].
fn stacks_near(g: &Game, flood: &mut Flood, p: PlayerId, cp: PlotIdx) -> Vec<Stack> {
    let (cx, cy) = g.map.xy(cp);
    let mut groups: BTreeMap<(PlotIdx, PlayerId), Vec<UnitId>> = BTreeMap::new();
    for dx in -HOLD_RADIUS..=HOLD_RADIUS {
        for dy in -HOLD_RADIUS..=HOLD_RADIUS {
            let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
            if q == cp {
                continue;
            }
            for &u in &g.map.plots[q].units {
                if raider(g, p, u) {
                    groups.entry((q, g.units[u].owner)).or_default().push(u);
                }
            }
        }
    }
    let next_to = |q: PlotIdx| {
        let (x, y) = g.map.xy(q);
        g.step_distance(x, y, cx, cy) == 1 && !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none()
    };
    let mut out: Vec<Stack> = Vec::new();
    for ((q, owner), mut units) in groups {
        units.sort();
        let (from, turns) = if next_to(q) {
            (q, 0)
        } else {
            let Some(&slow) = units.iter().min_by_key(|&&u| (g.unit_max_moves(u), u)) else { continue };
            let moves = g.unit_max_moves(slow).max(1);
            let Some(r) = flood.run_moves(g, slow, moves, 0, HOLD_STRIKE_TURNS, false).into_iter().find(|r| next_to(r.plot)) else { continue };
            (r.plot, r.turns)
        };
        out.push(Stack { units, owner, from, turns });
    }
    out.sort_by_key(|s| (s.turns, s.from, s.owner));
    out.truncate(HOLD_MAX_STACKS);
    out
}

/// The city of ours `city` against the enemy stacks near it (module comment, [`Hold`]).
pub fn holds(g: &Game, cache: Option<&Cache>, flood: &mut Flood, city: CityId) -> Hold {
    let p = g.cities[city].owner;
    let cp = g.city_plot(city);
    let stacks = stacks_near(g, flood, p, cp);
    if stacks.is_empty() {
        return Hold::default();
    }
    let turns = stacks.iter().map(|s| s.turns).min().unwrap_or(0);
    let last = stacks.iter().map(|s| s.turns).max().unwrap_or(0);
    // Reinforcements: our units near the city that can defend, the nearest first; each with the
    // turns it takes to stand in the city (this turn: 0).
    let (cx, cy) = g.map.xy(cp);
    let mut near: Vec<(i32, UnitId)> = g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u))
        .filter_map(|u| {
            let unit = &g.units[u];
            let at = g.unit_plot_opt(u)?;
            if at == cp || unit.march.is_some() || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || !g.unit_can_defend(u, None) {
                return None;
            }
            let info = &g.d.units[unit.unit_type];
            if info.combat_defense <= 0 && info.combat <= 0 {
                return None;
            }
            let (x, y) = g.map.xy(at);
            let d = g.step_distance(x, y, cx, cy);
            (d <= REINFORCE_RADIUS).then_some((d, u))
        })
        .collect();
    near.sort();
    near.truncate(REINFORCE_K);
    let mut arrive: Vec<(UnitId, i32)> = Vec::new();
    for &(_, u) in &near {
        let r = flood.run_until(g, u, last, false, |r| r.plot == cp);
        if let Some(r) = r.last().filter(|r| r.plot == cp) {
            arrive.push((u, r.turns));
        }
    }
    let by = |t: i32| -> Vec<UnitId> { arrive.iter().filter(|&&(_, at)| at <= t).map(|&(u, _)| u).collect() };
    let mut forecasts = 0;
    // Each attack: its battles one after another (the stack's pool, then its reserve).
    let mut fall = |attacks: &[(Vec<UnitId>, PlotIdx)], extras: &[UnitId]| -> Option<i32> {
        forecasts += 1;
        forecast_turn(g, cache, attacks, cp, extras, HOLD_RUNS, ForecastTurn::Next).map(|f| f.falls_permille)
    };
    let (mut single_now, mut single_reinforced, mut any) = (0, 0, false);
    for s in &stacks {
        let attack = [(s.units.clone(), s.from)];
        let Some(now) = fall(&attack, &[]) else { continue };
        any = true;
        single_now = single_now.max(now);
        let help = by(s.turns);
        let reinforced = if help.is_empty() { now } else { fall(&attack, &help).unwrap_or(now) };
        single_reinforced = single_reinforced.max(reinforced);
    }
    if !any {
        return Hold { forecasts, ..Hold::default() };
    }
    // The union of one player's stacks: each from its own approach, the soonest first, once the
    // last has come (the reinforcements that arrive by then).
    let (mut union_now, mut union_reinforced) = (single_now, single_reinforced);
    let owners: Vec<PlayerId> = {
        let mut o: Vec<PlayerId> = stacks.iter().map(|s| s.owner).collect();
        o.sort();
        o.dedup();
        o
    };
    for o in owners {
        let mine: Vec<&Stack> = stacks.iter().filter(|s| s.owner == o).collect();
        if mine.len() < 2 {
            continue;
        }
        let attacks: Attacks = mine.iter().map(|s| (s.units.clone(), s.from)).collect();
        let help = by(mine.iter().map(|s| s.turns).max().unwrap_or(0));
        if let Some(now) = fall(&attacks, &[]) {
            union_now = union_now.max(now);
            let reinforced = if help.is_empty() { now } else { fall(&attacks, &help).unwrap_or(now) };
            union_reinforced = union_reinforced.max(reinforced);
        }
    }
    Hold {
        threat: true,
        p_fall_now: single_now.max(union_now),
        p_fall_reinforced: single_reinforced.max(union_reinforced),
        single_now,
        union_now,
        single_reinforced,
        union_reinforced,
        reinforcements: by(turns),
        turns,
        stacks: stacks.len() as i32,
        enemy_units: stacks.iter().map(|s| s.units.len() as i32).sum(),
        owners: {
            let mut o: Vec<PlayerId> = stacks.iter().map(|s| s.owner).collect();
            o.sort();
            o.dedup();
            o
        },
        forecasts,
    }
}
