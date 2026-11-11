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
//! - **Stranded overseas** ([`fall_back`] on a landmass without a city or settler of ours): the
//!   army walks to the port nearest our nearest city that it may enter (a coastal city of another
//!   team on a sea one of our ports is on: open borders, a vassal or its master) and waits there for
//!   a ship (the group's mission AI `MISSIONAI_PICKUP` on its plot: the request, as a settler waiting
//!   in port asks for one); a transport of ours on its plot is boarded, and with nothing left to
//!   fight on that landmass it is boarded even while the ship supplies the army, or beside it at
//!   sea (a land unit steps onto water, docs/NAVAL.md, 3.2). Before asking for a ship, an army goes
//!   home in boats when the way over water is in our range and no enemy warship we see is within 3
//!   plots (`naval::walk_home_in_boats`, docs/NAVAL.md, section 8). The transport's side is
//!   `naval::pickup`; a port of ours with no transport builds one (`production::decide`,
//!   `army_ship`).
//! - **The last resort**: an army that has starved for [`STRANDED_TURNS`] of its owner's turns in a
//!   row with no path over land to a city of ours, or overseas with no transport of ours on its way
//!   to it, is disbanded unit by unit (`unit_scrap`; heroes and units the rules will not scrap
//!   stay).

use ffh_core::army::SupplyState;
use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::units::{alive, move_to, skip};
use crate::view::View;

/// Turns of Starvation in a row after which an army that cannot get home is disbanded (the last
/// resort: docs/SUPPLY.md, "Second pass", B).
pub(crate) const STRANDED_TURNS: i32 = 10;

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

/// Whether the group is an army whose line is cut (hungry or starving).
pub(crate) fn cut(g: &Game, grp: GroupId) -> bool {
    army_of(g, grp).is_some_and(|a| g.armies[a].unsupplied_turns > 0)
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
/// any)? Not an army: yes. Else: yes into a city of ours, when it stays where it is,
/// when `city` is within two turns of the plot it reaches this turn, when that plot is in our
/// territory, or when it has a supply line. At most one path search and one line search.
pub(crate) fn may_move(g: &Game, grp: GroupId, dest: PlotIdx, city: Option<PlotIdx>) -> bool {
    let Some(a) = army_of(g, grp) else { return true };
    let p = g.armies[a].owner;
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
    let here = g.unit_plot(u);
    let area = g.map.plots[here].area;
    let overseas = !has_city_on(g, p, area);
    // Overseas with nothing left to fight there: a ship of ours next to it takes it home (while the
    // ship stands there the army has a line to it, so this comes before the supplied test).
    if overseas && !war_target_on(g, v, area) && board(g, grp, u, here, "idle") {
        return true;
    }
    let state = g.army_supply_state(a);
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
    if overseas {
        return stranded_overseas(g, v, grp, u, a, here, state);
    }
    // Toward the nearest of our cities on its landmass it has a path to (of the three nearest);
    // else to the plot it reaches within three turns that is ours (else the nearest to that city).
    let (x, y) = g.map.xy(here);
    let mut cities: Vec<(i32, CityId)> = g.players[p].cities.iter().copied().filter(|&c| g.map.plots[g.city_plot(c)].area == area).map(|c| (g.plot_distance(x, y, g.cities[c].x, g.cities[c].y), c)).collect();
    cities.sort();
    // (A settler of ours here and no city yet: nothing to go back to.)
    let Some(&(_, nearest)) = cities.first() else { return false };
    let mut dest = cities.iter().take(3).map(|&(_, c)| g.city_plot(c)).find(|&q| g.group_find_path(grp, here, q, 0).is_some());
    // The last resort: starved long, and no path over land to any city of ours.
    if dest.is_none() && starved_long(g, a) && !cities.iter().skip(3).any(|&(_, c)| g.group_find_path(grp, here, g.city_plot(c), 0).is_some()) && disband(g, grp, a, "land") {
        return true;
    }
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

/// Has the army starved for [`STRANDED_TURNS`] of its owner's turns in a row?
fn starved_long(g: &Game, a: ArmyId) -> bool {
    g.armies[a].unsupplied_turns - g.d.defines.supply_hunger_turns >= STRANDED_TURNS
}

/// Does `p` have a city on `area`, or a settler there (which will found one)?
pub(crate) fn has_city_on(g: &Game, p: PlayerId, area: Option<AreaId>) -> bool {
    g.players[p].cities.iter().any(|&c| g.map.plots[g.city_plot(c)].area == area)
        || g.players[p].units.iter().any(|&o| g.d.units[g.units[o].unit_type].found && !g.unit_is_cargo(o) && g.map.plots[g.unit_plot(o)].area == area)
}

/// Is there something for us to fight on `area`: a city of a team at war with us (barbarians
/// included), or our amphibious assault's target?
pub(crate) fn war_target_on(g: &Game, v: &View, area: Option<AreaId>) -> bool {
    if v.amphibious.as_ref().is_some_and(|amph| g.map.plots[amph.plot].area == area) {
        return true;
    }
    g.cities.iter().any(|(c, city)| {
        let t = g.players[city.owner].team;
        t != v.team && g.at_war(v.team, t) && g.map.plots[g.city_plot(c)].area == area
    })
}

/// An army of `v.p` stands on land plot `q` (no city), on a landmass without a city or settler of
/// ours, with nothing to fight there: a ship next to it should take it home.
pub(crate) fn stranded_at(g: &Game, v: &View, q: PlotIdx) -> bool {
    if g.plot_is_water(q) || g.map.plots[q].plot_city.is_some() {
        return false;
    }
    let p = v.p;
    let army = g.map.plots[q].units.iter().any(|&o| g.units[o].owner == p && !g.unit_is_cargo(o) && g.units[o].army.is_some() && g.units[o].march.is_none());
    if !army {
        return false;
    }
    let area = g.map.plots[q].area;
    !has_city_on(g, p, area) && !war_target_on(g, v, area)
}

/// Is `t` a transport of ours (`p`'s) not on a settler's errand?
fn our_transport(g: &Game, p: PlayerId, t: UnitId) -> bool {
    g.units[t].owner == p && crate::naval::is_transport_type(&g.d.units[g.units[t].unit_type]) && !g.unit_get_cargo_units(t).iter().any(|&c| g.d.units[g.units[c].unit_type].found)
}

/// Board a transport of ours: one on the group's own plot (a city) takes every unit it has room
/// for (as an army boards in port, `naval::board`); else the stack steps onto one on a water plot
/// next to it (in our range: units the ship has no room for are left in boats there, and go home
/// in boats, `naval::in_boats`). True if it did.
fn board(g: &mut Game, grp: GroupId, u: UnitId, here: PlotIdx, why: &'static str) -> bool {
    let p = g.units[u].owner;
    if let Some(t) = g.map.plots[here].units.iter().copied().find(|&t| our_transport(g, p, t) && g.unit_can_load_unit(u, t, here)) {
        crate::trace!(g, p, "army {:?} ({} units) boards a ship at {:?} ({})", grp, g.groups[grp].units.len(), g.map.xy(here), why);
        g.tel_decision(p, "supply", "pickup_board", why, 1, String::new);
        for o in g.groups[grp].units.clone() {
            if g.units.contains(o) && g.unit_can_load_unit(o, t, here) {
                g.unit_load_unit(o, t);
            }
        }
        if alive(g, grp) && g.group_ready_to_move(grp, false) {
            skip(g, grp);
        }
        return true;
    }
    let (x, y) = g.map.xy(here);
    let next = (0..DirectionType::NUM)
        .filter_map(|d| g.plot_direction(x, y, DirectionType::from_usize(d)))
        .find(|&q| g.plot_is_water(q) && g.map.plots[q].units.iter().any(|&t| our_transport(g, p, t)) && g.unit_can_move_into(u, q, false, false, false));
    let Some(q) = next else { return false };
    crate::trace!(g, p, "army {:?} ({} units) boards a ship at {:?} from {:?} ({})", grp, g.groups[grp].units.len(), g.map.xy(q), g.map.xy(here), why);
    g.tel_decision(p, "supply", "pickup_board", why, 1, String::new);
    // (Onto the ship's plot at sea: `MOVE_BOATS` lets the AI's path finder take it there.)
    move_to(g, grp, q, MOVE_IGNORE_DANGER | MOVE_BOATS);
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
    true
}

/// Where an army of ours on that landmass can board a ship: a coastal city of another team, on one
/// of the seas our ports are on (`seas`), that the army may enter (open borders, a vassal or its
/// master) and so may our ships (the rules' `plot_is_friendly_city` for a sea unit asks the same).
/// (Not a coast plot: an AI's land unit cannot step onto a ship at sea, `board`.)
fn pickup_point(g: &Game, u: UnitId, q: PlotIdx, seas: &[AreaId]) -> bool {
    let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities.contains(c)) else { return false };
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    g.players[g.cities[c].owner].team != g.unit_get_team(u)
        && (0..DirectionType::NUM).any(|d| g.plot_direction(cx, cy, DirectionType::from_usize(d)).is_some_and(|w| g.plot_is_water(w) && g.map.plots[w].area.is_some_and(|ar| seas.contains(&ar))))
        && (q == g.unit_plot(u) || g.unit_can_move_into(u, q, false, false, false))
}

/// An army cut on a landmass without a city or settler of ours: board a ship of ours where it
/// stands or beside it; else the last resort; else to the pickup point (`pickup_point`) nearest our
/// nearest city, and wait there for a ship (module doc). True if its turn is over.
fn stranded_overseas(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, a: ArmyId, here: PlotIdx, state: SupplyState) -> bool {
    let p = v.p;
    if board(g, grp, u, here, "cut") {
        return true;
    }
    // In place of asking for a ship (docs/NAVAL.md, section 8): home in boats, over water in our
    // range, when no enemy warship we see is within 3 plots.
    if crate::naval::walk_home_in_boats(g, grp, here) {
        return true;
    }
    if starved_long(g, a) && !crate::naval::transport_coming(g, p, here) && disband(g, grp, a, "overseas") {
        return true;
    }
    let (x, y) = g.map.xy(here);
    let Some(home) = g.players[p].cities.iter().copied().min_by_key(|&c| (g.plot_distance(x, y, g.cities[c].x, g.cities[c].y), c)).map(|c| g.city_plot(c)) else { return false };
    // The seas our ports are on.
    let mut seas: Vec<AreaId> = Vec::new();
    for &c in &g.players[p].cities {
        if !crate::naval::is_coastal_city(g, c) {
            continue;
        }
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        for d in 0..DirectionType::NUM {
            if let Some(w) = g.plot_direction(cx, cy, DirectionType::from_usize(d)).filter(|&w| g.plot_is_water(w)) {
                if let Some(ar) = g.map.plots[w].area.filter(|ar| !seas.contains(ar)) {
                    seas.push(ar);
                }
            }
        }
    }
    let reach = v.flood.run(g, u, 10, false);
    let Some(dest) = reach.iter().filter(|r| pickup_point(g, u, r.plot, &seas)).min_by_key(|r| (dist(g, r.plot, home), r.turns, r.plot)).map(|r| r.plot) else {
        crate::trace!(g, p, "army {:?} cut overseas at {:?}: no port it may enter in reach", grp, g.map.xy(here));
        g.tel_decision(p, "supply", "stranded", "no_port", 1, String::new);
        return false;
    };
    if dest == here {
        let asked = g.groups[grp].ai_mission_ai_type == Some(MissionAIType::MISSIONAI_PICKUP) && g.group_ai_get_mission_ai_plot(grp) == Some(here);
        if !asked {
            crate::trace!(g, p, "army {:?} ({} units) waits for a ship at {:?}", grp, g.groups[grp].units.len(), g.map.xy(here));
            g.tel_decision(p, "supply", "pickup_request", "overseas", 1, String::new);
        }
        // The request: the turn skipped as an army waiting for a ship on this plot.
        g.group_push_mission(grp, MissionType::MISSION_SKIP, -1, -1, 0, false, false, Some(MissionAIType::MISSIONAI_PICKUP), Some(here), None);
        return true;
    }
    let why = if state == SupplyState::Starvation { "starving" } else { "hungry" };
    crate::trace!(g, p, "army {:?} ({} units, {}) cut overseas at {:?}: to the port at {:?}", grp, g.groups[grp].units.len(), why, g.map.xy(here), g.map.xy(dest));
    g.tel_decision(p, "supply", "fall_back", "overseas", 1, String::new);
    move_to(g, grp, dest, 0);
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
    true
}

/// The last resort: scrap the army's units in the group, but heroes (and units of a limited
/// class) and those the rules will not scrap. True if any was scrapped.
fn disband(g: &mut Game, grp: GroupId, a: ArmyId, why: &'static str) -> bool {
    let p = g.armies[a].owner;
    let here = g.group_plot(grp);
    let mut n = 0;
    for o in g.groups[grp].units.clone() {
        if !g.units.contains(o) || g.units[o].army != Some(a) {
            continue;
        }
        let info = &g.d.units[g.units[o].unit_type];
        let hero = g.unit_is_hero(o)
            || info.default_unit_ai_type == Some(UnitAIType::UNITAI_HERO)
            || info.unit_class_type.is_some_and(|c| g.d.unit_classes[c].max_global_instances > 0 || g.d.unit_classes[c].max_player_instances > 0);
        if hero || !g.unit_can_scrap(o) {
            continue;
        }
        g.tel_decision(p, "supply", "disband_unit", why, 1, String::new);
        g.unit_scrap(o);
        n += 1;
    }
    if n > 0 {
        crate::trace!(g, p, "army {:?} stranded ({}) at {:?}, starving {} turns: {} units disbanded", grp, why, here.map(|q| g.map.xy(q)), g.armies.get(a).map_or(0, |ar| ar.unsupplied_turns), n);
        g.tel_decision(p, "supply", "disband_army", why, n, String::new);
    }
    n > 0
}

/// Telemetry only (reads, changes nothing): each of `p`'s armies' supply counter this turn
/// (`supply_state`: `supplied`, or `cut_NN`, the counter, not capped: at most a label per turn of
/// the game), for the measures of
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
        if n <= 0 {
            g.tel_decision(p, "supply_state", "supplied", "", 1, String::new);
        } else {
            // (The army and its plot, for the events file: how many distinct armies were cut, and where.)
            g.tel_decision(p, "supply_state", &format!("cut_{n:02}"), "", 1, || {
                let (x, y) = g.army_plot(a).map_or((-1, -1), |q| g.map.xy(q));
                format!("{{\"army\":{},\"x\":{x},\"y\":{y}}}", a.0)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Player 0's coastal city on (2, 5) of the west landmass; its army of `n` Warriors on the east
    /// landmass (no city of ours there), its line cut for `turns` turns.
    fn overseas_game(turns: i32, n: usize) -> (Game, ArmyId, GroupId) {
        let mut g = crate::testkit::two_landmass_game(4);
        let p0 = PlayerId(0);
        let c = g.player_init_city(p0, 2, 5, true, true);
        assert!(crate::naval::is_coastal_city(&g, c));
        g.players[p0].turn_active = true;
        let (a, grp) = army(&mut g, 15, 5, n);
        assert!(g.plot_supply_line(p0, g.map.plot_xy(15, 5).unwrap()).is_none());
        g.armies[a].unsupplied_turns = turns;
        (g, a, grp)
    }

    fn analyzed(g: &Game) -> View {
        let mut v = View::new(g, PlayerId(0));
        v.analyze(g);
        v
    }

    /// A Galleon of player 0 (room for land units) on water plot (x, y).
    fn ship(g: &mut Game, x: i32, y: i32) -> (UnitId, GroupId) {
        let t = g.d.t::<UnitType>("UNIT_GALLEON");
        assert!(g.d.units[t].cargo_space >= 2);
        let s = g.player_init_unit(PlayerId(0), t, x, y, Some(UnitAIType::UNITAI_SETTLER_SEA), None);
        (s, g.units[s].group.unwrap())
    }

    /// Player 1's coastal city on (11, 5) of the east landmass, open borders both ways.
    fn friendly_port(g: &mut Game) -> PlotIdx {
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let c = g.player_init_city(p1, 11, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_set_open_borders(t0, t1, true);
        g.team_set_open_borders(t1, t0, true);
        g.city_plot(c)
    }

    fn set_army_xy(g: &mut Game, grp: GroupId, x: i32, y: i32) -> GroupId {
        let u = g.groups[grp].units[0];
        for o in g.groups[grp].units.clone() {
            g.unit_set_xy(o, x, y, true);
        }
        g.units[u].group.unwrap()
    }

    /// A barbarian Galley (an enemy warship) on (12, 2), seen by player 0.
    fn enemy_warship(g: &mut Game) -> UnitId {
        let barbarian = g.barbarian_player();
        let t = g.d.t::<UnitType>("UNIT_GALLEY");
        let s = g.player_init_unit(barbarian, t, 12, 2, None, None);
        let (q, t0) = (g.map.plot_xy(12, 2).unwrap(), g.players[PlayerId(0)].team);
        g.plot_change_visibility_count(q, t0, 1, None, false);
        assert!(g.plot_is_visible(q, t0));
        s
    }

    /// Stranded overseas: a cut army walks to the port it may enter nearest our city and asks for
    /// a ship there (an enemy warship near: no way home in boats); an empty transport sails for it
    /// and waits in the port; the army boards it.
    #[test]
    fn an_army_cut_overseas_waits_in_a_port_and_boards_a_ship() {
        let (mut g, _a, grp) = overseas_game(3, 2);
        let p0 = PlayerId(0);
        let port = friendly_port(&mut g);
        let u = g.groups[grp].units[0];
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u), "it heads for the port");
        assert!(g.map.xy(g.unit_plot(u)).0 < 15, "west, toward the port: now at {:?}", g.map.xy(g.unit_plot(u)));
        assert!(crate::naval::armies_waiting(&g, p0).is_empty(), "not there yet");
        // In the port, with an enemy warship near, it waits for a ship.
        let grp = set_army_xy(&mut g, grp, 11, 5);
        let raider = enemy_warship(&mut g);
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert_eq!(g.unit_plot(u), port, "it stays in the port");
        assert_eq!(crate::naval::armies_waiting(&g, p0), vec![port], "the request");
        g.unit_kill(raider, false, None);
        // An empty transport at (1, 5), off our city, sails for it.
        let (t, tg) = ship(&mut g, 1, 5);
        assert!(!crate::naval::transport_coming(&g, p0, port));
        let mut v = analyzed(&g);
        let here = g.unit_plot(t);
        assert!(crate::naval::pickup(&mut g, &mut v, tg, t, here));
        assert!(crate::naval::transport_coming(&g, p0, port), "on its way");
        // In the port: it waits there, and the army boards it.
        let tg = set_army_xy(&mut g, tg, 11, 5);
        let mut v = analyzed(&g);
        assert!(crate::naval::pickup(&mut g, &mut v, tg, t, port), "it waits in the port");
        assert!(fall_back(&mut g, &mut v, grp, u), "it boards");
        assert!(g.groups[grp].units.iter().all(|&o| g.unit_is_cargo(o)), "all aboard");
    }

    /// No port it may enter on that landmass, and the water home out of our range (no way home in
    /// boats): the army asks for nothing, and an empty transport is not sent. A ship of ours beside
    /// it at sea: it steps aboard (docs/NAVAL.md, 3.2).
    #[test]
    fn an_army_overseas_with_no_port_asks_for_no_ship() {
        let (mut g, _a, grp) = overseas_game(3, 2);
        let u = g.groups[grp].units[0];
        let mut v = analyzed(&g);
        assert!(!fall_back(&mut g, &mut v, grp, u));
        assert!(crate::naval::armies_waiting(&g, PlayerId(0)).is_empty());
        // A ship beside it, out of range: it boards it at sea.
        let grp = set_army_xy(&mut g, grp, 11, 4);
        ship(&mut g, 10, 4);
        let sea = g.map.plot_xy(10, 4).unwrap();
        assert!(!g.plot_in_sea_range(g.unit_get_team(u), sea));
        assert!(g.unit_can_move_into(u, sea, false, false, false));
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert!(g.unit_is_cargo(u), "aboard");
    }

    /// Stranded overseas with the way home in our range (a port of a team with open borders) and
    /// no enemy warship near: the army walks home in boats instead of asking for a ship
    /// (docs/NAVAL.md, section 8).
    #[test]
    fn an_army_cut_overseas_walks_home_in_boats() {
        let (mut g, _a, grp) = overseas_game(3, 2);
        friendly_port(&mut g);
        let u = g.groups[grp].units[0];
        let grp = set_army_xy(&mut g, grp, 11, 5);
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert!(g.unit_is_in_boats(u), "on its way in boats: at {:?}", g.map.xy(g.unit_plot(u)));
        assert!(crate::naval::armies_waiting(&g, PlayerId(0)).is_empty(), "no ship asked for");
    }

    /// A supplied army overseas with nothing to fight there boards the ship in the port it stands
    /// in; with an enemy city at war on that landmass it does not.
    #[test]
    fn an_idle_army_overseas_boards_the_ship_in_its_port() {
        let (mut g, a, grp) = overseas_game(0, 2);
        friendly_port(&mut g);
        let u = g.groups[grp].units[0];
        let grp = set_army_xy(&mut g, grp, 11, 5);
        ship(&mut g, 11, 5);
        assert_eq!(g.armies[a].unsupplied_turns, 0);
        // At war with a city on that landmass: it stays.
        let barbarian = g.barbarian_player();
        let theirs = g.player_init_city(barbarian, 16, 5, true, true);
        let mut v = analyzed(&g);
        assert!(!fall_back(&mut g, &mut v, grp, u), "a war target there");
        assert!(!g.unit_is_cargo(u));
        g.city_kill(theirs, true);
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u), "nothing left there: it boards");
        assert!(g.unit_is_cargo(u));
    }

    fn gone(g: &Game, o: UnitId) -> bool {
        !g.units.contains(o) || g.units[o].death_delay
    }

    /// The last resort: starving for `STRANDED_TURNS` turns, overseas with no transport coming, the
    /// army is disbanded; one turn less, or with a transport on its way, it is not.
    #[test]
    fn an_army_stranded_overseas_is_disbanded_as_the_last_resort() {
        let hunger = ffh_data::global().defines.supply_hunger_turns;
        let (mut g, a, grp) = overseas_game(hunger + STRANDED_TURNS - 1, 2);
        let units = g.groups[grp].units.clone();
        let u = units[0];
        let port = friendly_port(&mut g);
        // An enemy warship near: no way home in boats.
        enemy_warship(&mut g);
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert!(units.iter().all(|&o| !gone(&g, o)), "one turn short: kept");
        // A transport on its way to the port it waits in: kept.
        let grp = set_army_xy(&mut g, grp, 11, 5);
        let shore = port;
        g.group_push_mission(grp, MissionType::MISSION_SKIP, -1, -1, 0, false, false, Some(MissionAIType::MISSIONAI_PICKUP), Some(shore), None);
        let (_, tg) = ship(&mut g, 1, 5);
        g.group_push_mission(tg, MissionType::MISSION_SKIP, -1, -1, 0, false, false, Some(MissionAIType::MISSIONAI_PICKUP), Some(shore), None);
        g.armies[a].unsupplied_turns = hunger + STRANDED_TURNS;
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert!(units.iter().all(|&o| !gone(&g, o)), "a ship is coming: kept");
        // No ship coming: disbanded.
        g.group_push_mission(tg, MissionType::MISSION_SKIP, -1, -1, 0, false, false, None, None, None);
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, u));
        assert!(units.iter().all(|&o| gone(&g, o)), "disbanded");
        // With no port at all, the same.
        let (mut g, _, grp) = overseas_game(hunger + STRANDED_TURNS, 2);
        let units = g.groups[grp].units.clone();
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, units[0]));
        assert!(units.iter().all(|&o| gone(&g, o)), "disbanded, no port");
    }

    /// The last resort on its own landmass: starving long with no path over land to a city of ours
    /// (a wall of peaks between), the army is disbanded.
    #[test]
    fn an_army_with_no_way_home_over_land_is_disbanded() {
        let (mut g, _ours, _theirs) = war_game();
        let peak = PlotType::PLOT_PEAK;
        for y in 3..=8 {
            let q = g.map.plot_xy(9, y).unwrap();
            g.plot_set_plot_type(q, peak, false);
        }
        g.map_recalculate_areas();
        g.update_plot_groups();
        g.players[PlayerId(0)].turn_active = true;
        let (a, grp) = army(&mut g, 12, 5, 2);
        let units = g.groups[grp].units.clone();
        assert!(g.plot_supply_line(PlayerId(0), g.unit_plot(units[0])).is_none());
        let hunger = g.d.defines.supply_hunger_turns;
        g.armies[a].unsupplied_turns = hunger + STRANDED_TURNS - 1;
        let mut v = analyzed(&g);
        fall_back(&mut g, &mut v, grp, units[0]);
        assert!(units.iter().all(|&o| !gone(&g, o)), "one turn short: kept");
        let grp = g.units[units[0]].group.unwrap();
        g.armies[a].unsupplied_turns = hunger + STRANDED_TURNS;
        let mut v = analyzed(&g);
        assert!(fall_back(&mut g, &mut v, grp, units[0]));
        assert!(units.iter().all(|&o| gone(&g, o)), "disbanded");
    }
}
