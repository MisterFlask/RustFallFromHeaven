//! Cutting an invader's supply line (docs/SUPPLY.md, "Second pass", C; docs/AI.md, "Supply
//! lines"): an expert facing an invading army puts a unit on its road.
//!
//! - **The armies** ([`turn`]): each turn at most two enemy armies at war with us that we see
//!   within [`RANGE`] plots of one of our cities, those our cutters already watch first, then the
//!   nearest. For each, its known line (`Game::plot_supply_line_known`, our team as viewer): none,
//!   it is cut already and nothing is sent.
//! - **A cut plot** is a plot of that line, not next to the army and not a city, such that with a
//!   unit of ours standing on it the known search would find no line. The rules' search takes no
//!   hypothetical blocker, so a plot is found in two steps. Before: the line's plots are tested
//!   from the source end toward the army, at most [`MAX_TESTS`], by a local search
//!   ([`bypassed`]): with the plot closed, is there a way around it within [`BYPASS_RADIUS`]
//!   plots of it, over the plots as we know them, that keeps the line within its budget? A plot
//!   with no such way is a candidate. After: once our unit stands there, the known search itself
//!   says whether the line is gone. If it is not, the unit moves once to a candidate of the line
//!   that bends around it (the plot it stands on counted open, as it would be once it leaves),
//!   and holds wherever it is after that.
//! - **The unit**: a spare of a garrison (a fighter alone in its group, with no orders, in a city
//!   of ours that keeps three fighters without it), a recon or mounted unit first, that reaches
//!   the cut plot within two turns, and that the danger map does not show attacked there by more
//!   than its defence on that plot. It fortifies there. One unit per enemy army; it holds while
//!   that army is within the plots, at war and in sight, then goes back to the nearest city of
//!   ours. Its job is the group's mission AI (saved with the game): [`JOB`] (the first plot) or
//!   [`JOB_RETRY`] (moved once already), the cut plot, and the army's unit it watches.
//!   `units::play` leaves such a group to this module.
//! - **Cost a turn**: at most two known-line searches (one per army; a holding unit's check is
//!   that same search), at most six local searches per army (each within the 7x7 plots around a
//!   plot), at most six two-turn floods per army that has no unit on it yet, one for a unit that
//!   moves on, and the rules' path search of each move.
//!
//! Switch `SUPPLY_CUT` (`FFH_AI_SUPPLY_CUT_OFF`). Telemetry `supply_cut`: `fired` (a unit sent;
//! why `first` or `retry`), `arrived` (on its plot, the known line gone), `not_cut` (on its plot,
//! the known line still there), `hungry` (the army watched has its first turn of Hunger while
//! the unit holds; why `within_3` when that is within 3 of our turns of its arrival, else
//! `later`), `holding` (each turn on its plot; its unit and plot as the alternatives, for the
//! events file), `home` (the job ended; why `army_gone`, `away`, `no_path`), and `no_plot` /
//! `no_unit` (an army with a known line and no candidate plot / no unit to send).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use ffh_core::army::SupplyLine;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::units::alive;
use crate::view::{defense_power, View};

/// An enemy army is watched within this many plots of one of our cities.
pub const RANGE: i32 = 6;
/// The enemy armies looked at a turn.
pub const MAX_ARMIES: usize = 2;
/// The line's plots tested a turn, per army.
pub const MAX_TESTS: usize = 6;
/// The local search for a way around a closed plot stays within this many plots of it.
pub const BYPASS_RADIUS: i32 = 3;
/// The spares flooded per army.
const MAX_SPARES: usize = 6;
/// A unit is sent only to a plot it reaches on its first two turns (the flood's turns 0 and 1).
const REACH_TURNS: i32 = 1;
/// The mission AI of a cutter on its first plot.
pub const JOB: MissionAIType = MissionAIType::MISSIONAI_GUARD_TRADE_NET;
/// The mission AI of a cutter that has moved on once (it holds where it is next).
pub const JOB_RETRY: MissionAIType = MissionAIType::MISSIONAI_GUARD_BONUS;

fn on(g: &Game, p: PlayerId) -> bool {
    crate::switches::SUPPLY_CUT.on(g, p)
}

fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
    g.plot_distance(ax, ay, bx, by)
}

/// Whether `grp` is one of our cutters (its mission AI is [`JOB`] or [`JOB_RETRY`]).
pub(crate) fn is_cutter(g: &Game, grp: GroupId) -> bool {
    matches!(g.groups[grp].ai_mission_ai_type, Some(JOB) | Some(JOB_RETRY))
}

/// A cutter's job: its group and unit, the plot it holds, the army's unit it watches, retried.
#[derive(Clone, Copy, Debug)]
struct Job {
    grp: GroupId,
    unit: UnitId,
    plot: PlotIdx,
    watched: Option<UnitId>,
    retry: bool,
}

fn job_of(g: &Game, grp: GroupId) -> Option<Job> {
    if !alive(g, grp) || !is_cutter(g, grp) {
        return None;
    }
    let unit = g.group_get_head_unit(grp)?;
    let plot = g.group_ai_get_mission_ai_plot(grp)?;
    Some(Job { grp, unit, plot, watched: g.group_ai_get_mission_ai_unit(grp), retry: g.groups[grp].ai_mission_ai_type == Some(JOB_RETRY) })
}

/// A cutter's group outside [`turn`] (`units::play`): it keeps its orders; left ready to move,
/// it holds where it is, keeping its job (a skip pushed for it would end the job).
pub(crate) fn wait(g: &mut Game, grp: GroupId) {
    g.groups[grp].force_update = false;
    if let Some(job) = job_of(g, grp) {
        if g.group_ready_to_move(grp, false) {
            hold(g, &job);
        }
    }
}

/// Push `mission` for the group keeping its job (the mission AI is set by every push).
fn order(g: &mut Game, job: &Job, mission: MissionType, d1: i32, d2: i32) {
    let ai = if job.retry { JOB_RETRY } else { JOB };
    g.group_push_mission(job.grp, mission, d1, d2, 0, false, false, Some(ai), Some(job.plot), job.watched);
}

/// Fortify (else sleep, else skip) on its plot, keeping the job.
fn hold(g: &mut Game, job: &Job) {
    for m in [MissionType::MISSION_FORTIFY, MissionType::MISSION_SLEEP, MissionType::MISSION_SKIP] {
        if g.group_can_start_mission(job.grp, m, -1, -1, None, false) || m == MissionType::MISSION_SKIP {
            order(g, job, m, -1, -1);
            break;
        }
    }
    if alive(g, job.grp) {
        g.groups[job.grp].force_update = false;
    }
}

/// Toward its plot, keeping the job; holds when it stands there. False when the rules found no
/// way (it did not move).
fn go(g: &mut Game, job: &Job) -> bool {
    let start = g.unit_plot(job.unit);
    if start == job.plot {
        hold(g, job);
        return true;
    }
    let (x, y) = g.map.xy(job.plot);
    order(g, job, MissionType::MISSION_MOVE_TO, x, y);
    if !alive(g, job.grp) || !g.units.contains(job.unit) {
        return true;
    }
    g.groups[job.grp].force_update = false;
    if g.unit_plot(job.unit) == job.plot {
        hold(g, job);
        return true;
    }
    !(g.groups[job.grp].mission_queue.is_empty() && g.unit_plot(job.unit) == start)
}

/// The job ends: the unit goes back to the nearest city of ours (from a city, it stays and takes
/// its usual orders this turn).
fn end(g: &mut Game, p: PlayerId, job: &Job, why: &'static str) {
    g.tel_decision(p, "supply_cut", "home", why, 1, String::new);
    crate::trace!(g, p, "supply cut: {:?} at {:?} goes home ({why})", job.unit, g.map.xy(g.unit_plot(job.unit)));
    g.group_ai_set_mission_ai(job.grp, None, None, None);
    g.group_clear_mission_queue(job.grp);
    g.group_set_activity_type(job.grp, ActivityType::ACTIVITY_AWAKE);
    let here = g.unit_plot(job.unit);
    if g.map.plots[here].plot_city.is_some_and(|c| g.cities.contains(c) && g.cities[c].owner == p) {
        g.groups[job.grp].force_update = true;
        return;
    }
    if let Some(c) = crate::units::nearest_city(g, p, here) {
        crate::units::move_to(g, job.grp, g.city_plot(c), 0);
    }
    if alive(g, job.grp) {
        g.groups[job.grp].force_update = false;
    }
}

/// The plot an enemy army stands on, if it is one we look at: an army of a civilization at war
/// with us that we see (its plot visible, its head unit not invisible to us), within [`RANGE`] of
/// one of our cities; with that distance.
fn watched_army(g: &Game, v: &View, a: ArmyId) -> Option<(i32, PlotIdx)> {
    if !g.armies.contains(a) {
        return None;
    }
    let owner = g.armies[a].owner;
    if !v.civ_enemies.contains(&g.players[owner].team) {
        return None;
    }
    let grp = g.armies[a].group.filter(|&grp| alive(g, grp))?;
    let head = g.group_get_head_unit(grp)?;
    let plot = g.group_plot(grp)?;
    if !g.plot_is_visible(plot, v.team) || g.unit_is_invisible(head, v.team, false) {
        return None;
    }
    let (x, y) = g.map.xy(plot);
    let d = g.players[v.p].cities.iter().map(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y)).min()?;
    (d <= RANGE).then_some((d, plot))
}

/// The cost for a line of team `them` to enter `q` as team `us` knows it (the rules' costs,
/// docs/SUPPLY.md section 2, over what `us` knows: "Second pass", A), for the local search only.
/// `None`: it cannot. Left out (the search after arrival has the last word): forts and ships.
/// `open`: a plot counted free of units (the one our unit would leave).
fn known_cost(g: &Game, them: TeamId, us: TeamId, q: PlotIdx, open: Option<PlotIdx>) -> Option<i32> {
    let plot = &g.map.plots[q];
    if !plot.revealed[us] || g.plot_is_water(q) || g.plot_is_impassable(q) {
        return None;
    }
    if Some(q) != open && g.plot_is_visible(q, us) && plot.units.iter().any(|&u| g.units.contains(u) && g.unit_is_enemy(u, Some(them), Some(q)) && !g.unit_is_invisible(u, us, true)) {
        return None;
    }
    if let Some(c) = plot.plot_city.filter(|&c| g.cities.contains(c) && g.city_is_revealed(c, us, false)) {
        if g.teams[them].at_war[g.players[g.cities[c].owner].team] {
            return None;
        }
    }
    let owner = g.plot_get_team(q);
    if let Some(t) = owner.filter(|&t| t != them) {
        let may = g.team_is_friendly_territory(them, Some(t)) || g.teams[them].at_war[t] || g.team_is_open_borders(them, t) || g.team_is_barbarian(t) || g.team_is_barbarian(them);
        if !may {
            return None;
        }
    }
    let d = &g.d.defines;
    Some(if plot.route_type.is_some() {
        d.supply_cost_road
    } else if owner == Some(them) {
        d.supply_cost_own
    } else {
        d.supply_cost_land
    })
}

/// The cost of the line up to and including each of its plots, by [`known_cost`] (the army's
/// own plot 0; a plot the local costs refuse counts as open land).
fn prefix_costs(g: &Game, them: TeamId, us: TeamId, line: &SupplyLine, open: Option<PlotIdx>) -> Vec<i32> {
    let mut cum = vec![0; line.path.len()];
    for k in 1..line.path.len() {
        cum[k] = cum[k - 1] + known_cost(g, them, us, line.path[k], open).unwrap_or(g.d.defines.supply_cost_land);
    }
    cum
}

/// With `line.path[i]` closed, is there a way around it within [`BYPASS_RADIUS`] plots of it
/// that keeps the line within the budget? A cheapest-path search over the plots within the
/// radius from the line's plots before it (at their cost along the line) to the line's plots
/// after it (then along the line to the source).
fn bypassed(g: &Game, them: TeamId, us: TeamId, line: &SupplyLine, cum: &[i32], i: usize, open: Option<PlotIdx>) -> bool {
    let closed = line.path[i];
    let budget = g.d.defines.supply_line_budget;
    let total = *cum.last().unwrap_or(&0);
    let near = |q: PlotIdx| dist(g, q, closed) <= BYPASS_RADIUS;
    let after: BTreeMap<PlotIdx, usize> = (i + 1..line.path.len()).filter(|&j| near(line.path[j])).map(|j| (line.path[j], j)).collect();
    let mut best: BTreeMap<PlotIdx, i32> = BTreeMap::new();
    let mut heap: BinaryHeap<Reverse<(i32, PlotIdx)>> = BinaryHeap::new();
    for j in 0..i {
        let q = line.path[j];
        if near(q) && best.get(&q).is_none_or(|&c| cum[j] < c) {
            best.insert(q, cum[j]);
            heap.push(Reverse((cum[j], q)));
        }
    }
    while let Some(Reverse((c, q))) = heap.pop() {
        if best.get(&q).is_some_and(|&b| c > b) {
            continue;
        }
        if let Some(&j) = after.get(&q) {
            if c + total - cum[j] <= budget {
                return true;
            }
        }
        let (x, y) = g.map.xy(q);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(n) = g.plot_xy(x, y, dx, dy) else { continue };
                if n == closed || n == line.path[0] || !near(n) {
                    continue;
                }
                let Some(step) = known_cost(g, them, us, n, open) else { continue };
                let c2 = c + step;
                if c2 > budget || best.get(&n).is_some_and(|&b| b <= c2) {
                    continue;
                }
                best.insert(n, c2);
                heap.push(Reverse((c2, n)));
            }
        }
    }
    false
}

/// The candidate cut plots of `line` (an army of team `them` on its first plot), in the order
/// tested: from the source end toward the army, at most [`MAX_TESTS`] tested, not next to the
/// army, not a city, never the source's own plot.
fn candidates(g: &Game, them: TeamId, us: TeamId, line: &SupplyLine, open: Option<PlotIdx>) -> Vec<PlotIdx> {
    let n = line.path.len();
    if n < 3 {
        return Vec::new();
    }
    let army = line.path[0];
    let cum = prefix_costs(g, them, us, line, open);
    let mut out = Vec::new();
    let mut tested = 0;
    for i in (1..n - 1).rev() {
        let q = line.path[i];
        if dist(g, q, army) <= 1 || g.map.plots[q].plot_city.is_some() {
            continue;
        }
        if tested == MAX_TESTS {
            break;
        }
        tested += 1;
        if !bypassed(g, them, us, line, &cum, i, open) {
            out.push(q);
        }
    }
    out
}

/// Whether unit `u` of ours may hold `q` this turn: the danger map's attack power there is no
/// more than its defence on that plot.
fn safe(g: &Game, v: &View, u: UnitId, q: PlotIdx) -> bool {
    let def = defense_power(g, u) * (100 + g.plot_defense_modifier(q, Some(v.team), false).max(0)) / 100;
    v.danger_at(q) <= def
}

/// The units we may send: fighters of a garrison, alone in their group and with no orders, in a
/// city of ours that keeps three fighters without them; recon and mounted units first, then the
/// nearest to `toward`. At most [`MAX_SPARES`].
fn spares(g: &Game, v: &View, toward: PlotIdx) -> Vec<UnitId> {
    let p = v.p;
    let d = g.d;
    let fast = [d.ty::<UnitCombatType>("UNITCOMBAT_RECON"), d.ty::<UnitCombatType>("UNITCOMBAT_MOUNTED")];
    let mut out: Vec<(bool, i32, UnitId)> = Vec::new();
    for &c in &g.players[p].cities {
        let cp = g.city_plot(c);
        if dist(g, cp, toward) > 8 {
            continue;
        }
        let fighters: Vec<UnitId> = g.map.plots[cp].units.iter().copied().filter(|&u| g.units[u].owner == p && g.unit_is_army_unit(u) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND).collect();
        if fighters.len() <= 3 {
            continue;
        }
        for &u in &fighters {
            let unit = &g.units[u];
            let Some(grp) = unit.group.filter(|&grp| alive(g, grp)) else { continue };
            if unit.army.is_some() || g.unit_march(u).is_some() || g.unit_is_automated(u) || g.unit_is_cargo(u) || g.groups[grp].units.len() != 1 || !g.groups[grp].mission_queue.is_empty() || is_cutter(g, grp) || g.unit_moves_left(u) <= 0 || defense_power(g, u) <= 0 {
                continue;
            }
            let combat = d.units[unit.unit_type].unit_combat_type;
            let is_fast = combat.is_some() && fast.contains(&combat);
            out.push((!is_fast, dist(g, cp, toward), u));
        }
    }
    out.sort();
    out.into_iter().take(MAX_SPARES).map(|(_, _, u)| u).collect()
}

/// The nearest of `plots` unit `u` reaches within two turns and may hold: (turns, its index in
/// `plots`, the plot); and how many of `plots` it reaches but may not hold (telemetry).
fn reach(g: &Game, v: &mut View, u: UnitId, plots: &[PlotIdx]) -> (Option<(i32, usize, PlotIdx)>, usize) {
    let reached = v.flood.run(g, u, REACH_TURNS, false);
    let mut best: Option<(i32, usize, PlotIdx)> = None;
    let mut unsafe_plots = 0;
    for r in reached {
        if r.turns > REACH_TURNS {
            continue;
        }
        let Some(i) = plots.iter().position(|&q| q == r.plot) else { continue };
        if !safe(g, v, u, r.plot) {
            unsafe_plots += 1;
            continue;
        }
        if best.is_none_or(|b| (r.turns, i) < (b.0, b.1)) {
            best = Some((r.turns, i, r.plot));
        }
    }
    (best, unsafe_plots)
}

/// One turn of line cutting for `v.p` (module doc), before its units take their orders.
pub fn turn(g: &mut Game, v: &mut View) {
    let p = v.p;
    if v.barbarian || !on(g, p) {
        return;
    }
    let jobs: Vec<Job> = g.players[p].groups.clone().into_iter().filter_map(|grp| job_of(g, grp)).collect();
    if jobs.is_empty() && v.civ_enemies.is_empty() {
        return;
    }
    // The armies our units watch (one unit each; a second unit on the same army goes home).
    let mut watched: Vec<(ArmyId, Job)> = Vec::new();
    for job in jobs {
        let army = job.watched.and_then(|w| g.units[w].army).filter(|&a| g.armies.contains(a));
        let Some(a) = army else {
            end(g, p, &job, "army_gone");
            continue;
        };
        if watched.iter().any(|&(b, _)| b == a) || watched.len() == MAX_ARMIES {
            end(g, p, &job, "away");
            continue;
        }
        if watched_army(g, v, a).is_none() {
            end(g, p, &job, "away");
            continue;
        }
        watched.push((a, job));
    }
    // The armies to look at: the watched ones, then the nearest others.
    let mut others: Vec<(i32, ArmyId, PlotIdx)> = Vec::new();
    if watched.len() < MAX_ARMIES {
        for &t in &v.civ_enemies {
            for q in g.player_ids().filter(|&q| g.players[q].alive && g.players[q].team == t) {
                for &a in &g.players[q].armies {
                    if watched.iter().any(|&(b, _)| b == a) {
                        continue;
                    }
                    if let Some((d, plot)) = watched_army(g, v, a) {
                        others.push((d, a, plot));
                    }
                }
            }
        }
        others.sort();
        others.truncate(MAX_ARMIES - watched.len());
    }
    for (a, job) in watched {
        keep(g, v, a, job);
    }
    for (_, a, plot) in others {
        send(g, v, a, plot);
    }
}

/// A cutter on its way or holding (module doc).
fn keep(g: &mut Game, v: &mut View, a: ArmyId, job: Job) {
    let p = v.p;
    let owner = g.armies[a].owner;
    let them = g.players[owner].team;
    let Some(ap) = g.army_plot(a) else { return };
    let here = g.unit_plot(job.unit);
    if here != job.plot {
        if !go(g, &job) {
            end(g, p, &job, "no_path");
        }
        return;
    }
    let arrived = g.groups[job.grp].activity_type != ActivityType::ACTIVITY_SLEEP; // (fortified units sleep)
    let line = g.plot_supply_line_known(owner, ap, v.team);
    if arrived {
        g.tel_decision(p, "supply_cut", if line.is_none() { "arrived" } else { "not_cut" }, if job.retry { "retry" } else { "first" }, 1, String::new);
    } else {
        let (x, y) = g.map.xy(here);
        g.tel_decision(p, "supply_cut", "holding", "", 1, || format!("{{\"u\":{},\"x\":{x},\"y\":{y},\"army\":{}}}", job.unit.0, a.0));
        if g.armies[a].unsupplied_turns == 1 {
            let why = if g.units[job.unit].fortify_turns <= 2 { "within_3" } else { "later" };
            g.tel_decision(p, "supply_cut", "hungry", why, 1, String::new);
        }
    }
    match line {
        Some(line) if !job.retry => {
            // Not a cut: once to a plot of the line that bends around this one.
            let cands = candidates(g, them, v.team, &line, Some(here));
            if let Some((_, _, q)) = (!cands.is_empty()).then(|| reach(g, v, job.unit, &cands).0).flatten() {
                crate::trace!(g, p, "supply cut: {:?} moves on from {:?} to {:?}", job.unit, g.map.xy(here), g.map.xy(q));
                g.tel_decision(p, "supply_cut", "fired", "retry", 1, String::new);
                let next = Job { plot: q, retry: true, ..job };
                if !go(g, &next) {
                    end(g, p, &next, "no_path");
                }
                return;
            }
            let held = Job { retry: true, ..job };
            hold(g, &held);
        }
        _ => hold(g, &job),
    }
}

/// An army no unit of ours watches yet: its known line, a cut plot and a unit for it.
fn send(g: &mut Game, v: &mut View, a: ArmyId, ap: PlotIdx) {
    let p = v.p;
    let owner = g.armies[a].owner;
    let them = g.players[owner].team;
    let Some(line) = g.plot_supply_line_known(owner, ap, v.team) else { return };
    let cands = candidates(g, them, v.team, &line, None);
    if cands.is_empty() {
        let budget = g.d.defines.supply_line_budget;
        g.tel_decision(p, "supply_cut", "no_plot", "", 1, || format!("{{\"cost\":{},\"len\":{},\"slack\":{}}}", line.cost, line.path.len(), budget - line.cost));
        return;
    }
    let mut best: Option<(bool, i32, usize, UnitId, PlotIdx)> = None;
    let fast = [g.d.ty::<UnitCombatType>("UNITCOMBAT_RECON"), g.d.ty::<UnitCombatType>("UNITCOMBAT_MOUNTED")];
    let sent = spares(g, v, cands[0]);
    let mut unsafe_plots = 0;
    for &u in &sent {
        let combat = g.d.units[g.units[u].unit_type].unit_combat_type;
        let slow = !(combat.is_some() && fast.contains(&combat));
        if best.is_some_and(|b| !b.0 && slow) {
            break;
        }
        let (found, risky) = reach(g, v, u, &cands);
        unsafe_plots += risky;
        if let Some((turns, i, q)) = found {
            if best.is_none_or(|b| (slow, turns, i, u) < (b.0, b.1, b.2, b.3)) {
                best = Some((slow, turns, i, u, q));
            }
        }
    }
    let Some((_, _, _, u, q)) = best else {
        g.tel_decision(p, "supply_cut", "no_unit", "", 1, || format!("{{\"plots\":{},\"spares\":{},\"unsafe\":{unsafe_plots}}}", cands.len(), sent.len()));
        return;
    };
    let Some(grp) = g.units[u].group else { return };
    let watched = g.armies[a].group.and_then(|ag| g.group_get_head_unit(ag));
    crate::trace!(g, p, "supply cut: {:?} from {:?} to {:?} on the line of {:?} at {:?}", u, g.map.xy(g.unit_plot(u)), g.map.xy(q), a, g.map.xy(ap));
    g.tel_decision(p, "supply_cut", "fired", "first", 1, || format!("{{\"u\":{},\"army\":{},\"slack\":{}}}", u.0, a.0, g.d.defines.supply_line_budget - line.cost));
    let job = Job { grp, unit: u, plot: q, watched, retry: false };
    if !go(g, &job) {
        g.group_ai_set_mission_ai(grp, None, None, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Player 0's city on (10, 3) and player 1's on (16, 5), at war; a road on y = 5 from x = 5
    /// to their city; column x = 12 is water but for (12, 5) (with `isthmus`; else all land).
    /// Player 1's army of two Warriors on (7, 6); our city holds four Warriors and a Horseman,
    /// and a Warrior of ours on (6, 6) sees the army. Nothing owns the plots between.
    fn staged(isthmus: bool) -> (Game, ArmyId, UnitId) {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let cities = [g.player_init_city(p0, 10, 3, true, true), g.player_init_city(p1, 16, 5, true, true)];
        for c in cities {
            for t in g.team_ids() {
                g.cities[c].revealed[t] = true;
            }
        }
        let road = g.d.t::<RouteType>("ROUTE_ROAD");
        let ocean = g.d.t::<TerrainType>("TERRAIN_COAST");
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            if g.map.plots[q].plot_city.is_none() {
                g.plot_set_owner(q, None, false, false);
                g.plot_set_route_type(q, if y == 5 && (5..16).contains(&x) { Some(road) } else { None }, false);
            }
            if isthmus && x == 12 && y != 5 && (3..=8).contains(&y) {
                g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, false);
                g.plot_set_terrain_type(q, ocean, false);
            }
        }
        g.map_recalculate_areas();
        g.update_plot_groups();
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p1, w, 7, 6, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let grp = g.units[units[0]].group.unwrap();
        g.unit_join_group(units[1], Some(grp), false);
        let a = g.army_form(p1, &units).unwrap();
        for _ in 0..4 {
            g.player_init_unit(p0, w, 10, 3, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let horse = g.player_init_unit(p0, g.d.t::<UnitType>("UNIT_HORSEMAN"), 10, 3, Some(UnitAIType::UNITAI_ATTACK), None);
        g.player_init_unit(p0, w, 6, 6, Some(UnitAIType::UNITAI_ATTACK), None);
        g.players[p0].turn_active = true;
        (g, a, horse)
    }

    fn view(g: &Game) -> View {
        View::new(g, PlayerId(0))
    }

    /// The only plot whose closing leaves no way around is the isthmus: the Horseman goes there,
    /// fortifies, and the known line is gone; the Warriors stay. Our units' orders then leave it
    /// there.
    #[test]
    fn a_spare_rider_cuts_the_line_at_the_isthmus() {
        let (mut g, a, horse) = staged(true);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let t0 = g.players[p0].team;
        let ap = g.army_plot(a).unwrap();
        let line = g.plot_supply_line_known(p1, ap, t0).expect("a known line");
        let isthmus = g.map.plot_xy(12, 5).unwrap();
        assert!(line.path.contains(&isthmus));
        let t1 = g.players[p1].team;
        assert_eq!(candidates(&g, t1, t0, &line, None), vec![isthmus]);
        let mut v = view(&g);
        turn(&mut g, &mut v);
        assert_eq!(g.unit_plot(horse), isthmus, "the rider reaches it this turn");
        let grp = g.units[horse].group.unwrap();
        assert!(is_cutter(&g, grp));
        assert_eq!(g.group_ai_get_mission_ai_plot(grp), Some(isthmus));
        assert!(g.plot_supply_line_known(p1, ap, t0).is_none(), "the known line is gone");
        let city = g.map.plot_xy(10, 3).unwrap();
        assert_eq!(g.map.plots[city].units.iter().filter(|&&u| g.units[u].owner == p0).count(), 4);
        // The turn's other orders do not move it.
        let ai = crate::Ai::new();
        crate::units::play(&ai, &mut g, &mut v);
        assert_eq!(g.unit_plot(horse), isthmus);
        assert!(is_cutter(&g, grp));
    }

    /// Open land around every plot of the line: no plot cuts it, nothing is sent.
    #[test]
    fn no_unit_goes_where_the_line_bends_around_it() {
        let (mut g, a, horse) = staged(false);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let t0 = g.players[p0].team;
        let ap = g.army_plot(a).unwrap();
        let line = g.plot_supply_line_known(p1, ap, t0).unwrap();
        assert!(candidates(&g, g.players[p1].team, t0, &line, None).is_empty());
        let mut v = view(&g);
        let start = g.unit_plot(horse);
        turn(&mut g, &mut v);
        assert_eq!(g.unit_plot(horse), start);
        assert!(!is_cutter(&g, g.units[horse].group.unwrap()));
    }

    /// An enemy stack that can strike the isthmus next turn with more than the rider holds: no
    /// unit is sent.
    #[test]
    fn no_unit_goes_where_it_would_be_overwhelmed() {
        let (mut g, _a, horse) = staged(true);
        let p1 = PlayerId(1);
        let axe = g.d.t::<UnitType>("UNIT_AXEMAN");
        for _ in 0..4 {
            g.player_init_unit(p1, axe, 14, 7, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        let mut v = view(&g);
        let isthmus = g.map.plot_xy(12, 5).unwrap();
        assert!(!safe(&g, &v, horse, isthmus));
        let start = g.unit_plot(horse);
        turn(&mut g, &mut v);
        assert_eq!(g.unit_plot(horse), start);
    }

    /// Once the army is beyond the plots of our cities, the unit goes back toward our city and
    /// its job ends.
    #[test]
    fn the_cutter_goes_home_when_the_army_leaves() {
        let (mut g, a, horse) = staged(true);
        let mut v = view(&g);
        turn(&mut g, &mut v);
        let grp = g.units[horse].group.unwrap();
        assert!(is_cutter(&g, grp));
        // The army steps back beyond 6 plots of our city (it stays in sight of a unit of ours);
        // a new turn's moves for the rider.
        let ag = g.armies[a].group.unwrap();
        let head = g.groups[ag].units[0];
        g.unit_set_xy(head, 17, 7, true);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        g.player_init_unit(PlayerId(0), w, 17, 8, Some(UnitAIType::UNITAI_ATTACK), None);
        g.units[horse].moves = 0;
        let mut v = view(&g);
        turn(&mut g, &mut v);
        assert!(!is_cutter(&g, g.units[horse].group.unwrap()));
        let (x, _) = g.map.xy(g.unit_plot(horse));
        assert!(x < 12, "back toward our city: now at x = {x}");
    }
}
