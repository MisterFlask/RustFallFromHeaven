//! Supply lines (docs/SUPPLY.md, section 6; docs/AI.md, "Supply lines"): where our armies go
//! given the rules' supply line (`Game::plot_supply_line`, a budget-limited search: called at most
//! once or twice per army per turn here, never per candidate plot).
//!
//! - **The move gate** ([`may_move`]): an army does not take a move whose plot at the end of this
//!   turn (the rules' path) would have no line, unless an enemy city it marches on is within two
//!   turns of that plot (two turns of its slowest unit's moves, in plots). A move into a city of
//!   ours is always allowed (it ends at a source), and so is staying. Gated in `military.rs` (the
//!   hunt, gathering into a bigger stack) and `campaign.rs` (`march`, every operation move).
//! - **Falling back** ([`fall_back`]): a hungry army goes on only while the city it attacks (its
//!   operation's target, else an enemy city at war) is within two plots; otherwise, and always
//!   when starving, it goes back toward the nearest city of ours on its landmass it has a path to
//!   (else the reachable plot of ours, or nearest that city, within three turns; the source of
//!   its last line, which the AI does not remember) until it stands where it has a line again.
//!   A hungry army does not stop to heal in the field (it cannot), and an army whose line is cut
//!   is re-planned each turn (`units::play`), so no multi-turn order walks it on.
//!
//! Switch `SUPPLY` (`FFH_AI_SUPPLY_OFF`): off, the AI ignores supply lines (as before them).

use ffh_core::army::SupplyState;
use ffh_core::game::Game;
use ffh_core::ids::*;

use crate::units::{alive, move_to, skip};
use crate::view::View;

/// The army the group's head belongs to (an AI army of a player that is no barbarian, and not a
/// marching recruit).
fn army_of(g: &Game, grp: GroupId) -> Option<ArmyId> {
    let head = g.group_get_head_unit(grp)?;
    let a = g.units[head].army.filter(|&a| g.armies.contains(a))?;
    if g.player_is_barbarian(g.armies[a].owner) || g.units[head].march.is_some() {
        return None;
    }
    Some(a)
}

fn on(g: &Game, p: PlayerId) -> bool {
    crate::switches::SUPPLY.on(g, p)
}

/// Whether the group is an army whose line is cut (hungry or starving), with the rule on.
pub(crate) fn cut(g: &Game, grp: GroupId) -> bool {
    army_of(g, grp).is_some_and(|a| g.armies[a].unsupplied_turns > 0 && on(g, g.armies[a].owner))
}

/// Plots the group covers in two turns: twice its slowest unit's moves (at least 2).
fn two_turns(g: &Game, grp: GroupId) -> i32 {
    let pace = g.groups[grp].units.iter().map(|&u| g.unit_base_moves(u)).min().unwrap_or(1).max(1);
    (2 * pace).max(2)
}

fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
    g.plot_distance(ax, ay, bx, by)
}

/// May the group take a move toward `dest` this turn, marching on the enemy city on `city` (if
/// any)? Not an army, the rule off: yes. Else: yes into a city of ours, when it stays where it is,
/// when `city` is within two turns of the plot it reaches this turn, when that plot is in our
/// territory, or when it has a supply line. At most one path search and one line search.
pub(crate) fn may_move(g: &Game, grp: GroupId, dest: PlotIdx, city: Option<PlotIdx>) -> bool {
    let Some(a) = army_of(g, grp) else { return true };
    let p = g.armies[a].owner;
    if !on(g, p) {
        return true;
    }
    let team = g.players[p].team;
    if g.map.plots[dest].plot_city.is_some_and(|c| g.cities.contains(c) && g.players[g.cities[c].owner].team == team) {
        return true;
    }
    let Some(here) = g.group_plot(grp) else { return true };
    let end = if dist(g, here, dest) <= 1 {
        dest
    } else {
        match g.group_find_path(grp, here, dest, 0) {
            Some(path) => path.end_turn_plot.unwrap_or(dest),
            // No path: the rules will not move it.
            None => return true,
        }
    };
    if end == here || city.is_some_and(|c| dist(g, end, c) <= two_turns(g, grp)) {
        return true;
    }
    // Our own territory carries the line (1 a plot from a city of ours: it is cut there only when
    // enemy units close every way home), so no search for it.
    if g.plot_get_team(end) == Some(team) || g.plot_supply_line(p, end).is_some() {
        return true;
    }
    crate::trace!(g, p, "army {:?} does not move to {:?}: no supply line there", grp, g.map.xy(end));
    g.tel_decision(p, "supply", "move_refused", "no_line", 1, String::new);
    false
}

/// The city the group attacks, if it is within two plots: its operation's target, else an enemy
/// city at war (a city of a team we are at war with, as `military::opportune` takes).
fn attacking_near(g: &Game, v: &View, grp: GroupId, here: PlotIdx) -> bool {
    if let Some(op) = v.campaign.op_of(grp).map(|i| &v.campaign.ops[i]) {
        return dist(g, here, op.plot) <= 2;
    }
    let (x, y) = g.map.xy(here);
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities.contains(c)) else { continue };
            let t = g.players[g.cities[c].owner].team;
            if t != v.team && g.at_war(v.team, t) {
                return true;
            }
        }
    }
    false
}

/// The rule for an army whose line is cut (module doc). True if the group went back (its turn
/// is over).
pub(crate) fn fall_back(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let Some(a) = army_of(g, grp) else { return false };
    let p = v.p;
    if !on(g, p) {
        return false;
    }
    let state = g.army_supply_state(a);
    let here = g.unit_plot(u);
    match state {
        SupplyState::Supplied => return false,
        SupplyState::Hunger { .. } => {
            crate::campaign::plan(g, v);
            if attacking_near(g, v, grp, here) {
                g.tel_decision(p, "supply", "hungry_goes_on", "city_near", 1, String::new);
                return false;
            }
        }
        SupplyState::Starvation => {}
    }
    // Where it stands it has a line again: its counter goes back to 0 at the next turn start.
    if g.plot_supply_line(p, here).is_some() {
        return false;
    }
    // Toward the nearest of our cities on its landmass it has a path to (of the three nearest);
    // else to the plot it reaches within three turns that is ours (else the nearest to that city).
    let area = g.map.plots[here].area;
    let (x, y) = g.map.xy(here);
    let mut cities: Vec<(i32, CityId)> = g.players[p].cities.iter().copied().filter(|&c| g.map.plots[g.city_plot(c)].area == area).map(|c| (g.plot_distance(x, y, g.cities[c].x, g.cities[c].y), c)).collect();
    cities.sort();
    let Some(&(_, nearest)) = cities.first() else { return false };
    let mut dest = cities.iter().take(3).map(|&(_, c)| g.city_plot(c)).find(|&q| g.group_find_path(grp, here, q, 0).is_some());
    if dest.is_none() {
        let home = g.city_plot(nearest);
        let reach = v.flood.run(g, u, 3, false);
        dest = reach
            .iter()
            .filter(|r| r.plot != here)
            .min_by_key(|r| (g.plot_get_team(r.plot) != Some(v.team), dist(g, r.plot, home), r.turns, r.plot))
            .map(|r| r.plot)
            .filter(|&q| g.plot_get_team(q) == Some(v.team) || dist(g, q, home) < dist(g, here, home));
    }
    let Some(dest) = dest else { return false };
    let why = if state == SupplyState::Starvation { "starving" } else { "hungry" };
    crate::trace!(g, p, "army {:?} ({} units, {}) falls back from {:?} toward {:?}", grp, g.groups[grp].units.len(), why, g.map.xy(here), g.map.xy(dest));
    g.tel_decision(p, "supply", "fall_back", why, 1, String::new);
    move_to(g, grp, dest, 0);
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
    true
}

/// Telemetry only (reads, changes nothing): each of `p`'s armies' supply counter this turn
/// (`supply_state`: `supplied`, or `cut_NN`, the counter, capped at 30), for the measures of
/// docs/SUPPLY.md ("Measures"): the episodes that reach `NN` turns are the count of `cut_NN`.
pub fn turn(g: &Game, p: PlayerId) {
    if !g.telemetry.on() || g.player_is_barbarian(p) {
        return;
    }
    for &a in &g.players[p].armies {
        if !g.armies.contains(a) || g.army_plot(a).is_none() {
            continue;
        }
        let n = g.armies[a].unsupplied_turns;
        let choice = if n <= 0 { "supplied".to_string() } else { format!("cut_{:02}", n.min(30)) };
        g.tel_decision(p, "supply_state", &choice, "", 1, String::new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_data::*;

    /// Player 0's city on (3, 5) and player 1's on (16, 5) of the testkit's landmass (x 2..=17),
    /// at war; nothing owns the plots between them and there is no road.
    fn war_game() -> (Game, CityId, CityId) {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 3, 5, true, true);
        let theirs = g.player_init_city(p1, 16, 5, true, true);
        for q in g.map.plot_ids() {
            if g.map.plots[q].plot_city.is_none() {
                g.plot_set_owner(q, None, false, false);
                g.plot_set_route_type(q, None, false);
            }
        }
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        (g, ours, theirs)
    }

    fn army(g: &mut Game, x: i32, y: i32, n: usize) -> (ArmyId, GroupId) {
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let units: Vec<UnitId> = (0..n).map(|_| g.player_init_unit(PlayerId(0), w, x, y, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let grp = g.units[units[0]].group.unwrap();
        for &u in &units[1..] {
            g.unit_join_group(u, Some(grp), false);
        }
        let a = g.army_form(PlayerId(0), &units).unwrap();
        (a, g.units[units[0]].group.unwrap())
    }

    /// The gate: a move to a plot with a line, into our city, or ending within two turns of the
    /// city it marches on is taken; a move ending in open country beyond the line is not.
    #[test]
    fn an_army_does_not_move_beyond_its_line() {
        let (mut g, ours, theirs) = war_game();
        let p0 = PlayerId(0);
        // Seven plots of open country (3 each) and the city's own plot (1): the line reaches
        // x = 11; x = 12 is cut.
        assert!(g.plot_supply_line(p0, g.map.plot_xy(11, 5).unwrap()).is_some());
        assert!(g.plot_supply_line(p0, g.map.plot_xy(12, 5).unwrap()).is_none());
        let (_, grp) = army(&mut g, 11, 5, 2);
        let map = g.map.clone();
        let to = |x| map.plot_xy(x, 5).unwrap();
        assert!(may_move(&g, grp, to(10), None), "back toward home");
        assert!(!may_move(&g, grp, to(12), None), "one plot beyond the line");
        assert!(may_move(&g, grp, g.city_plot(ours), None), "into our city");
        // Marching on their city (16, 5): from x = 12 it is four plots away, more than the two
        // turns (2 plots) of Warriors; from x = 14 it is two.
        let tp = g.city_plot(theirs);
        assert!(!may_move(&g, grp, to(12), Some(tp)));
        let (_, near) = army(&mut g, 13, 6, 2);
        assert!(g.plot_supply_line(p0, to(14)).is_none());
        assert!(may_move(&g, near, to(14), Some(tp)), "the city within two turns of the plot");
        // Units in no army are not gated.
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let lone = g.player_init_unit(p0, w, 11, 6, Some(UnitAIType::UNITAI_ATTACK), None);
        let lone_grp = g.units[lone].group.unwrap();
        g.units[lone].army = None;
        assert!(may_move(&g, lone_grp, to(12), None));
    }

    /// A hungry army far from any city it attacks goes back toward our city; next to an enemy
    /// city it goes on; a starving one goes back even there.
    #[test]
    fn a_hungry_army_falls_back_unless_its_city_is_near() {
        let (mut g, ours, _theirs) = war_game();
        let p0 = PlayerId(0);
        g.players[p0].turn_active = true;
        // Out of the line, far from their city: hungry, it goes back.
        let (a, grp) = army(&mut g, 13, 4, 2);
        g.armies[a].unsupplied_turns = 1;
        let u = g.groups[grp].units[0];
        let mut v = View::new(&g, p0);
        v.analyze(&g);
        assert!(fall_back(&mut g, &mut v, grp, u), "it falls back");
        let (x, _) = g.map.xy(g.unit_plot(u));
        assert!(x < 13, "toward {}: now at x = {x}", g.cities[ours].name);
        // Two plots from their city: it goes on.
        let (b, near) = army(&mut g, 14, 5, 2);
        g.armies[b].unsupplied_turns = 2;
        let un = g.groups[near].units[0];
        let mut v = View::new(&g, p0);
        v.analyze(&g);
        assert!(!fall_back(&mut g, &mut v, near, un), "the city it attacks is within two plots");
        assert_eq!(g.map.xy(g.unit_plot(un)), (14, 5));
        // Starving: back, whatever is near.
        g.armies[b].unsupplied_turns = 3;
        assert_eq!(g.army_supply_state(b), SupplyState::Starvation);
        assert!(fall_back(&mut g, &mut v, near, un));
        assert!(g.map.xy(g.unit_plot(un)).0 < 14);
        // Supplied: nothing.
        let (c, home) = army(&mut g, 6, 5, 2);
        assert_eq!(g.armies[c].unsupplied_turns, 0);
        let uh = g.groups[home].units[0];
        assert!(!fall_back(&mut g, &mut v, home, uh));
        // Hungry but standing where it has a line again: nothing (the counter resets next turn).
        g.armies[c].unsupplied_turns = 1;
        assert!(!fall_back(&mut g, &mut v, home, uh));
    }
}
