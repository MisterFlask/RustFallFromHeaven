//! Incoming attacks: which hostile forces we see will attack which of our cities, when, and with
//! what (docs/CALIBRATION.md, "Incoming attacks (`threat.rs`)"). The battle itself is forecast
//! well (`forecast::holds`); this is the other half a defender reads off the map: an army
//! gathering and coming, several turns ahead, and roughly where it is going.
//!
//! - **Stacks** ([`Threats::of`]): land units of the teams at war with us (the barbarians
//!   included) that the AI's threat rules let it see ([`crate::view::known_threat`]: through fog,
//!   never through stealth), able to attack, not animals, not garrisons in their own cities;
//!   grouped by owner, units on the same or adjacent plots merged into one stack. Each stack floods
//!   as its slowest unit moves (the rules' costs, [`Flood`]) for [`THREAT_TURNS`] turns; the
//!   cities of ours it reaches are its candidates, each with the plot next to it the stack attacks
//!   from and the turns to stand there (0: it can attack on its next turn).
//! - **Forces**: an owner's stacks are one army: for each city of ours its stacks reach (the
//!   soonest [`MAX_CANDIDATES`]), the rules' forecast of the army gathered there by each arrival
//!   turn in turn ([`ForecastTurn::Next`], the garrison as it stands, every unit that has arrived
//!   from the approach of the biggest stack), until it takes the city with at least [`TAKE_BAR`]:
//!   `take_turns`, the turn the owner can take it.
//! - **The rule** (transparent; the levels in percent are the chance the owner attacks the city
//!   within a horizon of `h` turns): the city the owner can take soonest (ties: the better chance,
//!   then the nearer city) is its target. A target it can take at turn `w < h`:
//!   [`P_TARGET`] x [`DECAY`]^`w` (the sooner, the surer); another city it can take (a stack that
//!   takes one city walks on to the next): [`P_NEXT`] x [`DECAY`]^`w`. A city beyond the forecast
//!   ones (not among the soonest [`MAX_CANDIDATES`]) of an owner with a target counts as one it
//!   can take, from the turn it reaches it or takes the target, the later. Either after the
//!   horizon (`w >= h`): [`P_LATE`]. A city it reaches within `h` and cannot take: [`P_REACH`];
//!   later: 0. A barbarian owner: [`P_BARBARIAN`] for a city a stack reaches within `h`
//!   (barbarians seldom assault a city, whatever they could take), else 0, with no forecast. A
//!   city's chance is that of one owner at least attacking it (owners independent). The levels
//!   were read off the attacks of 16 games by category (docs/CALIBRATION.md).
//! - [`incoming`]: the city's estimate ([`Incoming`]): the chance it is attacked within each of
//!   [`HORIZONS`], the turns until the soonest expected attack, the expected force (the owners'
//!   armies that can take it, by the turn they can; failing those, every stack that reaches it),
//!   and the forecast of that force against the garrison as it stands and with our units that can
//!   get there before it (`p_lost_as_is`, `p_lost_reinforced`).
//!
//! No memory: only what is in sight this turn (state would have to live in `Game`). Pure and
//! deterministic; nothing decides on it yet: it is recorded in the prediction ledger
//! (`predict::incoming`).

use std::collections::BTreeMap;

use ffh_core::engagement::ForecastTurn;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::forecast::{forecast_turn, Cache};
use crate::reach::Flood;

/// A stack counts if it can stand next to one of our cities within this many of its turns.
pub const THREAT_TURNS: i32 = 6;
/// The horizons of [`Incoming::p_attacked_within`] (turns from now).
pub const HORIZONS: [i32; 4] = [1, 3, 6, 10];
/// The soonest this many cities an owner's stacks reach get a forecast (the others: none).
pub const MAX_CANDIDATES: usize = 4;
/// At most this many forecasts of an owner's army gathering at one city (the arrival turns).
pub const MAX_STEPS: usize = 3;
/// Battles played per forecast of an army against a candidate (the target choice).
pub const TARGET_RUNS: i32 = 20;
/// Battles played per forecast of a city's expected force (`p_lost_*`).
pub const CITY_RUNS: i32 = 40;
/// An owner can take a city when its forecast is at least this (per mille).
pub const TAKE_BAR: i32 = 500;
/// Plots a unit can cover per move at best (roads): bounds where a stack can come from.
pub const PLOTS_PER_MOVE: i32 = 3;
/// Reinforcements: the nearest this many of our units that could reach the city in time.
pub const REINFORCE_K: usize = 8;

/// The levels of the rule (module comment), percent: the target, taken on the owner's next turn ...
pub const P_TARGET: i32 = 40;
/// ... another city it takes ...
pub const P_NEXT: i32 = 25;
/// ... each turn later multiplies by this (numerator, denominator) ...
pub const DECAY: (i32, i32) = (2, 3);
/// ... either, taken only after the horizon ...
pub const P_LATE: i32 = 1;
/// ... a city reached within the horizon that the owner cannot take ...
pub const P_REACH: i32 = 1;
/// ... and any city a barbarian stack reaches within it.
pub const P_BARBARIAN: i32 = 1;

/// One of our cities a stack can reach: the plot next to it it attacks from, and the turns to
/// stand there (0: it can attack on its next turn).
#[derive(Clone, Debug)]
pub struct Cand {
    pub city: CityId,
    pub from: PlotIdx,
    pub turns: i32,
}

/// A hostile stack (module comment).
#[derive(Clone, Debug)]
pub struct Stack {
    pub owner: PlayerId,
    pub units: Vec<UnitId>,
    /// The plot of its slowest unit (where it floods from).
    pub at: PlotIdx,
    /// Some unit of it stands on water (a landing from ships).
    pub at_sea: bool,
    /// Its candidates, the soonest first.
    pub cands: Vec<Cand>,
}

/// What one owner's army can do to one of our cities.
#[derive(Clone, Debug)]
pub struct OwnerCity {
    pub city: CityId,
    /// The soonest of its stacks to stand next to the city.
    pub turns: i32,
    /// The turn its army gathered there takes the city ([`TAKE_BAR`]); `None`: not within the
    /// stacks in reach, or not forecast.
    pub take_turns: Option<i32>,
    /// The forecast at `take_turns` (else the last one made; -1: none made), per mille.
    pub p_take: i32,
    /// Forecast at all (one of the soonest [`MAX_CANDIDATES`], not barbarian).
    pub forecast: bool,
}

/// One owner's stacks in reach of our cities, and its cities.
#[derive(Clone, Debug)]
pub struct Force {
    pub owner: PlayerId,
    pub barbarian: bool,
    /// Indices into [`Threats::stacks`].
    pub stacks: Vec<usize>,
    /// The cities its stacks reach, the soonest first.
    pub cities: Vec<OwnerCity>,
    /// The index in `cities` of its target, if any.
    pub target: Option<usize>,
}

/// The category of an owner and city under the rule (module comment).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Target,
    Next,
    Reach,
    Barbarian,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Target => "target",
            Kind::Next => "next",
            Kind::Reach => "reach",
            Kind::Barbarian => "barbarian",
        }
    }
}

impl Force {
    /// The rule's category of city `i` of this owner.
    pub fn kind(&self, i: usize) -> Kind {
        if self.barbarian {
            Kind::Barbarian
        } else if self.target == Some(i) {
            Kind::Target
        } else if self.when(i).is_some() {
            Kind::Next
        } else {
            Kind::Reach
        }
    }

    /// The turn this owner can take its city `i`: forecast, or for a city beyond the forecast
    /// ones of an owner with a target, the later of the turn it reaches it and the target's.
    pub fn when(&self, i: usize) -> Option<i32> {
        let c = &self.cities[i];
        if c.forecast {
            return c.take_turns;
        }
        let t = self.target?;
        Some(c.turns.max(self.cities[t].take_turns.unwrap_or(0)))
    }

    /// The rule's chance (percent) that this owner attacks its city `i` within `h` turns.
    pub fn p_attack(&self, i: usize, h: i32) -> i32 {
        let decay = |p: i32, w: i32| (0..w.max(0)).fold(p, |p, _| p * DECAY.0 / DECAY.1.max(1));
        let reached = self.cities[i].turns < h;
        match self.kind(i) {
            Kind::Barbarian => if reached { P_BARBARIAN } else { 0 },
            Kind::Reach => if reached { P_REACH } else { 0 },
            k => {
                let w = self.when(i).unwrap_or(0);
                if w >= h {
                    P_LATE
                } else if k == Kind::Target {
                    decay(P_TARGET, w).max(P_LATE)
                } else {
                    decay(P_NEXT, w).max(P_LATE)
                }
            }
        }
    }
}

/// The hostile stacks and forces of one player's turn.
#[derive(Clone, Debug, Default)]
pub struct Threats {
    pub p: PlayerId,
    pub stacks: Vec<Stack>,
    pub forces: Vec<Force>,
    /// Hostile units seen that reach none of our cities within [`THREAT_TURNS`], by owner.
    pub far: BTreeMap<PlayerId, i32>,
    /// Floods run and forecasts made (the cost).
    pub floods: i32,
    pub forecasts: i32,
}

/// The army of `units` (one owner's, wherever they stand) attacking `city` as if gathered on
/// `from`: the forecast's chance it falls (per mille), the garrison as it stands plus `extras`; -1
/// when the rules refuse the battle.
fn army_takes(g: &Game, cache: Option<&Cache>, units: &[UnitId], from: PlotIdx, city: CityId, extras: &[UnitId], runs: i32) -> i32 {
    let attack = [(units.to_vec(), from)];
    forecast_turn(g, cache, &attack, g.city_plot(city), extras, runs, ForecastTurn::Next).map_or(-1, |f| f.falls_permille)
}

impl Threats {
    /// The stacks and forces that can reach a city of `p` within [`THREAT_TURNS`] (module comment).
    pub fn of(g: &Game, cache: Option<&Cache>, flood: &mut Flood, p: PlayerId) -> Threats {
        let mut out = Threats { p, ..Threats::default() };
        let team = g.players[p].team;
        let cities: Vec<(CityId, PlotIdx)> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c)).map(|c| (c, g.city_plot(c))).collect();
        if cities.is_empty() {
            return out;
        }
        // The hostile units by owner and plot.
        let mut by_plot: BTreeMap<(PlayerId, PlotIdx), Vec<UnitId>> = BTreeMap::new();
        for o in g.player_ids() {
            let pl = &g.players[o];
            if !pl.alive || pl.team == team || !g.at_war(team, pl.team) {
                continue;
            }
            for &u in &pl.units {
                if !g.units.contains(u) || g.unit_plot_opt(u).is_none() || !crate::forecast::raider(g, p, u) {
                    continue;
                }
                by_plot.entry((o, g.unit_plot(u))).or_default().push(u);
            }
        }
        // Merge one owner's plots that touch (same or adjacent plots).
        let keys: Vec<(PlayerId, PlotIdx)> = by_plot.keys().copied().collect();
        let mut root: Vec<usize> = (0..keys.len()).collect();
        fn find(r: &mut [usize], i: usize) -> usize {
            let mut i = i;
            while r[i] != i {
                r[i] = r[r[i]];
                i = r[i];
            }
            i
        }
        for i in 0..keys.len() {
            let (oi, qi) = keys[i];
            let (xi, yi) = g.map.xy(qi);
            for j in i + 1..keys.len() {
                let (oj, qj) = keys[j];
                if oj != oi {
                    break;
                }
                let (xj, yj) = g.map.xy(qj);
                if g.step_distance(xi, yi, xj, yj) <= 1 {
                    let (a, b) = (find(&mut root, i), find(&mut root, j));
                    if a != b {
                        root[a.max(b)] = a.min(b);
                    }
                }
            }
        }
        let mut groups: BTreeMap<usize, Vec<UnitId>> = BTreeMap::new();
        for (i, k) in keys.iter().enumerate() {
            let r = find(&mut root, i);
            groups.entry(r).or_default().extend(by_plot[k].iter().copied());
        }
        // The plots that matter: the plots next to our cities.
        let mut near: BTreeMap<PlotIdx, Vec<usize>> = BTreeMap::new();
        for (ci, &(_, cp)) in cities.iter().enumerate() {
            let (cx, cy) = g.map.xy(cp);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(q) = g.plot_xy(cx, cy, dx, dy) {
                        if q != cp && !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none() {
                            near.entry(q).or_default().push(ci);
                        }
                    }
                }
            }
        }
        let denom = g.d.defines.move_denominator.max(1);
        for (_, mut units) in groups {
            units.sort();
            let owner = g.units[units[0]].owner;
            let Some(&slow) = units.iter().min_by_key(|&&u| (g.unit_max_moves(u), u)) else { continue };
            let at = g.unit_plot(slow);
            let at_sea = units.iter().any(|&u| g.plot_is_water(g.unit_plot(u)));
            let moves = g.unit_max_moves(slow).max(1);
            // Where it can come from at best: no city of ours that far, no flood.
            let bound = ((moves + denom - 1) / denom).max(1) * PLOTS_PER_MOVE * (THREAT_TURNS + 1) + 1;
            let (ax, ay) = g.map.xy(at);
            if !cities.iter().any(|&(_, cp)| {
                let (cx, cy) = g.map.xy(cp);
                g.step_distance(ax, ay, cx, cy) <= bound
            }) {
                *out.far.entry(owner).or_insert(0) += units.len() as i32;
                continue;
            }
            out.floods += 1;
            let reached = flood.run_moves(g, slow, moves, 0, THREAT_TURNS, false);
            // Per city: the soonest plot next to it (ties: the lower plot index).
            let mut best: Vec<Option<(i32, PlotIdx)>> = vec![None; cities.len()];
            for r in &reached {
                let Some(cis) = near.get(&r.plot) else { continue };
                for &ci in cis {
                    if best[ci].is_none_or(|b| (r.turns, r.plot) < b) {
                        best[ci] = Some((r.turns, r.plot));
                    }
                }
            }
            let mut cands: Vec<Cand> = best.iter().enumerate().filter_map(|(ci, b)| b.map(|(turns, from)| Cand { city: cities[ci].0, from, turns })).collect();
            if cands.is_empty() {
                *out.far.entry(owner).or_insert(0) += units.len() as i32;
                continue;
            }
            cands.sort_by_key(|c| (c.turns, c.city));
            out.stacks.push(Stack { owner, units, at, at_sea, cands });
        }
        // The forces: each owner's stacks as one army.
        let mut owners: Vec<PlayerId> = out.stacks.iter().map(|s| s.owner).collect();
        owners.sort();
        owners.dedup();
        for owner in owners {
            let barbarian = g.player_is_barbarian(owner);
            let stacks: Vec<usize> = (0..out.stacks.len()).filter(|&i| out.stacks[i].owner == owner).collect();
            let mut cities: BTreeMap<CityId, i32> = BTreeMap::new();
            for &si in &stacks {
                for c in &out.stacks[si].cands {
                    let e = cities.entry(c.city).or_insert(c.turns);
                    *e = (*e).min(c.turns);
                }
            }
            let mut list: Vec<OwnerCity> = cities.into_iter().map(|(city, turns)| OwnerCity { city, turns, take_turns: None, p_take: -1, forecast: false }).collect();
            list.sort_by_key(|c| (c.turns, c.city));
            if !barbarian {
                for oc in list.iter_mut().take(MAX_CANDIDATES) {
                    oc.forecast = true;
                    // Its stacks that reach the city, by arrival.
                    let mut arr: Vec<(i32, usize, PlotIdx)> =
                        stacks.iter().filter_map(|&si| out.stacks[si].cands.iter().find(|c| c.city == oc.city).map(|c| (c.turns, si, c.from))).collect();
                    arr.sort_by_key(|&(t, si, _)| (t, si));
                    let mut turns: Vec<i32> = arr.iter().map(|a| a.0).collect();
                    turns.dedup();
                    let steps: Vec<i32> = if turns.len() <= MAX_STEPS {
                        turns.clone()
                    } else {
                        (0..MAX_STEPS).map(|k| turns[k * (turns.len() - 1) / (MAX_STEPS - 1)]).collect()
                    };
                    for t in steps {
                        let here: Vec<&(i32, usize, PlotIdx)> = arr.iter().filter(|a| a.0 <= t).collect();
                        let units: Vec<UnitId> = here.iter().flat_map(|a| out.stacks[a.1].units.iter().copied()).collect();
                        // From the approach of the biggest stack there (ties: the soonest).
                        let Some(&&(_, _, from)) = here.iter().max_by_key(|a| (out.stacks[a.1].units.len(), std::cmp::Reverse(a.0), std::cmp::Reverse(a.1))) else { continue };
                        out.forecasts += 1;
                        oc.p_take = army_takes(g, cache, &units, from, oc.city, &[], TARGET_RUNS);
                        if oc.p_take >= TAKE_BAR {
                            oc.take_turns = Some(t);
                            break;
                        }
                    }
                }
            }
            let dist = |c: CityId| {
                let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                stacks.iter().map(|&si| {
                    let (x, y) = g.map.xy(out.stacks[si].at);
                    g.step_distance(x, y, cx, cy)
                }).min().unwrap_or(0)
            };
            let target = (0..list.len()).filter(|&i| list[i].take_turns.is_some()).min_by_key(|&i| (list[i].take_turns, -list[i].p_take, dist(list[i].city), list[i].city));
            out.forces.push(Force { owner, barbarian, stacks, cities: list, target });
        }
        out
    }

    /// The (force, city index) pairs about city `c`.
    pub fn about(&self, c: CityId) -> impl Iterator<Item = (&Force, usize)> {
        self.forces.iter().filter_map(move |f| f.cities.iter().position(|x| x.city == c).map(|i| (f, i)))
    }
}

/// The estimate of incoming attacks on one of our cities ([`incoming`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Incoming {
    /// The chance (percent) the city is attacked within each of [`HORIZONS`].
    pub p_attacked_within: [i32; 4],
    /// Turns until the soonest expected attack: the soonest an owner can take it; failing one,
    /// the soonest a stack stands next to it. `None`: no stack in reach.
    pub turns_until: Option<i32>,
    /// The expected attackers: each stack and the plot it attacks from, with its turns, the
    /// soonest first.
    pub force: Vec<(Vec<UnitId>, PlotIdx, i32)>,
    /// Whether `force` is armies that can take the city (else: every stack that reaches it).
    pub takes: bool,
    /// The forecast of `force` (gathered on the approach of its biggest stack) attacking the
    /// garrison as it stands, and with our units that can arrive before the soonest attack (per
    /// mille; 0 without a force).
    pub p_lost_as_is: i32,
    pub p_lost_reinforced: i32,
    /// Those units.
    pub reinforcements: Vec<UnitId>,
    /// Owners with a stack in reach of the city, stacks in reach, and owners that can take it.
    pub owners: i32,
    pub stacks: i32,
    pub takers: i32,
    /// The rule's category of the owner most likely to attack within 6 turns (`""`: none).
    pub kind: &'static str,
    /// Forecasts and floods run for this answer.
    pub forecasts: i32,
    pub floods: i32,
}

/// The estimate for our city `city` from the turn's threats `t` (module comment).
pub fn incoming(g: &Game, cache: Option<&Cache>, flood: &mut Flood, t: &Threats, city: CityId) -> Incoming {
    let mut out = Incoming::default();
    let pairs: Vec<(&Force, usize)> = t.about(city).collect();
    if pairs.is_empty() {
        return out;
    }
    out.owners = pairs.len() as i32;
    for (k, &h) in HORIZONS.iter().enumerate() {
        let mut none = 1000i64;
        for &(f, i) in &pairs {
            none = none * (100 - f.p_attack(i, h)) as i64 / 100;
        }
        out.p_attacked_within[k] = ((1000 - none) / 10) as i32;
    }
    out.kind = pairs.iter().max_by_key(|(f, i)| (f.p_attack(*i, 6), std::cmp::Reverse(f.owner))).map_or("", |(f, i)| f.kind(*i).name());
    // The force: the owners that can take it, each with its stacks there by then; failing one,
    // every stack that reaches the city.
    let takers: Vec<&(&Force, usize)> = pairs.iter().filter(|(f, i)| !f.barbarian && f.when(*i).is_some()).collect();
    out.takers = takers.len() as i32;
    out.takes = !takers.is_empty();
    let mut force: Vec<(Vec<UnitId>, PlotIdx, i32)> = Vec::new();
    for &(f, i) in &pairs {
        let by = if out.takes {
            match f.when(i) {
                Some(tt) if !f.barbarian => tt,
                _ => continue,
            }
        } else {
            THREAT_TURNS
        };
        for &si in &f.stacks {
            let s = &t.stacks[si];
            if let Some(c) = s.cands.iter().find(|c| c.city == city && c.turns <= by) {
                out.stacks += 1;
                force.push((s.units.clone(), c.from, c.turns));
            }
        }
    }
    force.sort_by_key(|(u, from, turns)| (*turns, *from, u[0]));
    let soonest = if out.takes { takers.iter().filter_map(|(f, i)| f.when(*i)).min().unwrap_or(0) } else { force.first().map_or(0, |x| x.2) };
    out.turns_until = Some(soonest);
    let Some(from) = force.iter().max_by_key(|x| (x.0.len(), std::cmp::Reverse(x.2))).map(|x| x.1) else {
        out.force = force;
        return out;
    };
    // One owner's army at a time (the forecast is of one player's attack): the worst.
    let mut by_owner: BTreeMap<PlayerId, Vec<UnitId>> = BTreeMap::new();
    for (u, _, _) in &force {
        by_owner.entry(g.units[u[0]].owner).or_default().extend(u.iter().copied());
    }
    let cp = g.city_plot(city);
    let worst = |extras: &[UnitId], forecasts: &mut i32| -> i32 {
        by_owner
            .values()
            .map(|units| {
                *forecasts += 1;
                // Each owner from its own biggest stack's approach.
                let own_from = force.iter().filter(|x| units.contains(&x.0[0])).max_by_key(|x| (x.0.len(), std::cmp::Reverse(x.2))).map_or(from, |x| x.1);
                army_takes(g, cache, units, own_from, city, extras, CITY_RUNS).max(0)
            })
            .max()
            .unwrap_or(0)
    };
    let mut forecasts = 0;
    out.p_lost_as_is = worst(&[], &mut forecasts);
    // Reinforcements: our units that could stand in the city before the soonest attack, the
    // nearest [`REINFORCE_K`] of those within the best-case distance, each flooded to it.
    let p = g.cities[city].owner;
    let (cx, cy) = g.map.xy(cp);
    let denom = g.d.defines.move_denominator.max(1);
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
            let reach = ((g.unit_max_moves(u) + denom - 1) / denom).max(1) * PLOTS_PER_MOVE * (soonest + 1);
            (d <= reach).then_some((d, u))
        })
        .collect();
    near.sort();
    near.truncate(REINFORCE_K);
    for &(_, u) in &near {
        out.floods += 1;
        let r = flood.run_until(g, u, soonest, false, |r| r.plot == cp);
        if r.last().is_some_and(|r| r.plot == cp) {
            out.reinforcements.push(u);
        }
    }
    out.p_lost_reinforced = if out.reinforcements.is_empty() { out.p_lost_as_is } else { worst(&out.reinforcements, &mut forecasts) };
    out.forecasts = forecasts;
    out.force = force;
    out
}
