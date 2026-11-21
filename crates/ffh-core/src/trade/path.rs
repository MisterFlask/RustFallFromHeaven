//! Trade (docs/TRADE.md): the route's path (3.5) and its destinations (3.3).

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::game::Game;

use super::*;

/// A plot's best label in the route search: cost, plots, the plot it is entered from.
#[derive(Clone, Copy)]
pub(crate) struct Label {
    cost: i32,
    len: i32,
    from: Option<PlotIdx>,
    done: bool,
}

/// The labels of a route search, per plot.
pub(crate) struct Tree {
    pub(crate) labels: Vec<Option<Label>>,
}

impl Tree {
    pub(crate) fn path_to(&self, q: PlotIdx) -> Option<Vec<PlotIdx>> {
        self.labels[q.idx()]?;
        let mut v = vec![q];
        let mut at = q;
        while let Some(f) = self.labels[at.idx()].and_then(|l| l.from) {
            v.push(f);
            at = f;
        }
        v.reverse();
        Some(v)
    }
}

impl Game {
    // ---------------------------------------------------------------------------------------
    // The path (3.5)

    /// Whether `team` may enter the land of `other` for a route: its own or a friend's, open
    /// borders, at war, the barbarians'.
    pub(crate) fn route_may_enter_land(&self, team: TeamId, other: TeamId) -> bool {
        other == team || self.team_is_friendly_territory(team, Some(other)) || self.team_is_open_borders(team, other) || self.teams[team].at_war[other] || self.team_is_barbarian(other)
    }

    /// Whether `team` may pass through city `c` on a route: a city of its own team, a friend's
    /// or one it has open borders with (never one it is at war with).
    pub(crate) fn route_may_pass_city(&self, team: TeamId, c: CityId) -> bool {
        let other = self.city_team(c);
        if self.teams[team].at_war[other] || self.team_is_barbarian(other) {
            return false;
        }
        other == team || self.team_is_friendly_territory(team, Some(other)) || self.team_is_open_borders(team, other)
    }

    /// The cost for a route of `team` to enter `plot` (3.5), `None` when it cannot. A city plot
    /// is entered only if it may pass the city (the destination is the search's own matter).
    pub fn plot_route_cost(&self, team: TeamId, plot: PlotIdx) -> Option<i32> {
        if self.plot_is_impassable(plot) {
            return None;
        }
        let d = &self.d.defines;
        if self.plot_is_water(plot) {
            if !self.plot_in_sea_range(team, plot) || self.plot_sea_closed(plot, team) {
                return None;
            }
            return Some(d.caravan_cost_water);
        }
        if let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c)) {
            if !self.route_may_pass_city(team, c) {
                return None;
            }
        }
        if let Some(t) = self.plot_get_team(plot) {
            if !self.route_may_enter_land(team, t) {
                return None;
            }
        }
        Some(if self.map.plots[plot].route_type.is_some() { d.caravan_cost_road } else { d.caravan_cost_land })
    }

    /// The step cost from `from` to `to` for a route of `team`, `None` when it cannot: the
    /// plot's cost, and no diagonal step over water between two land plots (as ships).
    pub(crate) fn route_step_cost(&self, team: TeamId, from: PlotIdx, to: PlotIdx, goal_city: bool) -> Option<i32> {
        if self.plot_is_water(from) && self.plot_is_water(to) {
            let ((fx, fy), (tx, ty)) = (self.map.xy(from), self.map.xy(to));
            if fx != tx && fy != ty {
                let c1 = self.map.plot_xy(fx, ty);
                let c2 = self.map.plot_xy(tx, fy);
                if c1.is_some_and(|q| !self.plot_is_water(q)) && c2.is_some_and(|q| !self.plot_is_water(q)) {
                    return None;
                }
            }
        }
        match self.plot_route_cost(team, to) {
            Some(c) => Some(c),
            // The destination city itself: entered as land (a road where it has one).
            None if goal_city && !self.plot_is_impassable(to) => {
                let d = &self.d.defines;
                Some(if self.map.plots[to].route_type.is_some() { d.caravan_cost_road } else { d.caravan_cost_land })
            }
            None => None,
        }
    }

    /// The route search of `team` from `start` (3.5): the cheapest path to every plot, by plot
    /// entered; ties: the lower cost, the fewer plots, the lower plot index at the first
    /// difference. A plot `goal` names is entered even when it is no plot to pass (the
    /// destination city) and never left; `stop` ends the search once the best path to it is
    /// final.
    pub(crate) fn route_search(&self, team: TeamId, start: PlotIdx, goal: &dyn Fn(PlotIdx) -> bool, stop: &dyn Fn(PlotIdx) -> bool) -> Tree {
        let n = self.map.num_plots();
        let mut tree = Tree { labels: vec![None; n] };
        let mut heap: BinaryHeap<Reverse<(i32, i32, PlotIdx)>> = BinaryHeap::new();
        tree.labels[start.idx()] = Some(Label { cost: 0, len: 1, from: None, done: false });
        heap.push(Reverse((0, 1, start)));
        while let Some(Reverse((cost, len, q))) = heap.pop() {
            let Some(label) = tree.labels[q.idx()] else { continue };
            if label.done || (label.cost, label.len) != (cost, len) {
                continue;
            }
            tree.labels[q.idx()].as_mut().expect("labelled").done = true;
            if q != start && stop(q) {
                break;
            }
            // A goal city that is no plot to pass is never left.
            if q != start && goal(q) && self.plot_route_cost(team, q).is_none() {
                continue;
            }
            let (x, y) = self.map.xy(q);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let Some(nq) = self.plot_xy(x, y, dx, dy) else { continue };
                    if nq == start {
                        continue;
                    }
                    let Some(step) = self.route_step_cost(team, q, nq, goal(nq)) else { continue };
                    let (c, l) = (cost + step, len + 1);
                    let better = match tree.labels[nq.idx()] {
                        None => true,
                        Some(old) if old.done => false,
                        Some(old) => match (c, l).cmp(&(old.cost, old.len)) {
                            Ordering::Less => true,
                            Ordering::Greater => false,
                            Ordering::Equal => old.from.is_some_and(|f| tree.path_to(q) < tree.path_to(f)),
                        },
                    };
                    if better {
                        let new_key = tree.labels[nq.idx()].is_none_or(|old| (old.cost, old.len) != (c, l));
                        tree.labels[nq.idx()] = Some(Label { cost: c, len: l, from: Some(q), done: false });
                        if new_key {
                            heap.push(Reverse((c, l, nq)));
                        }
                    }
                }
            }
        }
        tree
    }

    /// The path of a route of `owner` from `origin` to `destination` (docs/TRADE.md, 3.5): the
    /// origin's plot first, the destination's last; `None` when there is none. Pure: it ignores
    /// units and is not the units' path finder.
    pub fn route_path(&self, owner: PlayerId, origin: CityId, destination: CityId) -> Option<Vec<PlotIdx>> {
        if !self.cities.contains(origin) || !self.cities.contains(destination) || origin == destination {
            return None;
        }
        let team = self.players[owner].team;
        let (from, to) = (self.city_plot(origin), self.city_plot(destination));
        let tree = self.route_search(team, from, &|q| q == to, &|q| q == to);
        tree.path_to(to).filter(|p| p.len() > 1)
    }

    /// The path from `from` to the nearest plot of `path` (4.1: a caravan off its path), by the
    /// route search; `from` first. `None` when none is reachable.
    pub(crate) fn route_rejoin(&self, team: TeamId, from: PlotIdx, path: &[PlotIdx]) -> Option<Vec<PlotIdx>> {
        let on = |q: PlotIdx| path.contains(&q);
        let tree = self.route_search(team, from, &on, &on);
        // The first goal settled is the nearest (the search stops there).
        let best = path.iter().copied().filter(|&q| tree.labels[q.idx()].is_some_and(|l| l.done)).min_by(|&a, &b| {
            let (la, lb) = (tree.labels[a.idx()].unwrap(), tree.labels[b.idx()].unwrap());
            (la.cost, la.len).cmp(&(lb.cost, lb.len)).then_with(|| tree.path_to(a).cmp(&tree.path_to(b)))
        })?;
        tree.path_to(best)
    }

    /// The named seas `path` crosses, in the order it enters them.
    pub fn route_path_seas(&self, path: &[PlotIdx]) -> Vec<SeaId> {
        let mut out: Vec<SeaId> = Vec::new();
        for &q in path {
            if let Some(s) = self.map.plots[q].sea {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    }

    // ---------------------------------------------------------------------------------------
    // Destinations (3.3)

    /// Why `dest` is no destination for a route of `owner` from `origin` (3.3, the path
    /// excluded), `None` when it passes; `except` is a route not counted as linking the two.
    pub(crate) fn route_destination_refusal_no_path(&self, owner: PlayerId, origin: CityId, dest: CityId, except: Option<RouteId>) -> Option<RouteRefusal> {
        if !self.cities.contains(dest) || !self.cities.contains(origin) {
            return Some(RouteRefusal::DestinationGone);
        }
        if dest == origin {
            return Some(RouteRefusal::SameCity);
        }
        let team = self.players[owner].team;
        if !self.city_is_revealed(dest, team, false) {
            return Some(RouteRefusal::NotRevealed);
        }
        if self.cities[dest].outpost {
            return Some(RouteRefusal::DestinationOutpost);
        }
        let other = self.cities[dest].owner;
        if other != owner {
            if let Some(r) = self.route_trade_refusal(owner, other) {
                return Some(r);
            }
        }
        if self.routes.iter().any(|(id, r)| Some(id) != except && r.owner == owner && ((r.origin == origin && r.destination == dest) || (r.origin == dest && r.destination == origin))) {
            return Some(RouteRefusal::AlreadyLinked);
        }
        None
    }

    /// Why `owner` may not trade with `other` (3.3: `player_can_have_trade_routes_with` both
    /// ways), `None` when it may.
    pub(crate) fn route_trade_refusal(&self, owner: PlayerId, other: PlayerId) -> Option<RouteRefusal> {
        if self.player_can_have_trade_routes_with(owner, other) && self.player_can_have_trade_routes_with(other, owner) {
            return None;
        }
        let (t, o) = (self.players[owner].team, self.players[other].team);
        if self.teams[t].at_war[o] {
            return Some(RouteRefusal::AtWar(other));
        }
        if self.team_is_free_trade(t, o) && self.team_is_free_trade(o, t) {
            return Some(RouteRefusal::NoForeignTrade(other));
        }
        Some(RouteRefusal::NoOpenBorders(other))
    }

    /// Why `dest` is no destination for a route of `owner` from `origin` (docs/TRADE.md, 3.3),
    /// `None` when it is one.
    pub fn route_destination_refusal(&self, owner: PlayerId, origin: CityId, dest: CityId) -> Option<RouteRefusal> {
        if let Some(r) = self.route_destination_refusal_no_path(owner, origin, dest, None) {
            return Some(r);
        }
        if self.route_path(owner, origin, dest).is_none() {
            return Some(RouteRefusal::NoPath);
        }
        None
    }

    /// Route `r` checked now (3.4): its path, or why it is closed.
    pub(crate) fn route_check(&self, r: RouteId) -> Result<Vec<PlotIdx>, RouteRefusal> {
        let route = &self.routes[r];
        if let Some(why) = self.route_destination_refusal_no_path(route.owner, route.origin, route.destination, Some(r)) {
            return Err(why);
        }
        self.route_path(route.owner, route.origin, route.destination).ok_or(RouteRefusal::NoPath)
    }

    /// Every destination a route from city `origin` of its owner could have now (3.3), in city id
    /// order, with its path and pay (6.1 at growth 0 with two caravans). One search serves all.
    pub fn route_offers(&self, origin: CityId) -> Vec<RouteOffer> {
        if !self.cities.contains(origin) {
            return Vec::new();
        }
        let owner = self.cities[origin].owner;
        let team = self.players[owner].team;
        let mut cands: Vec<CityId> = self.cities.ids().into_iter().filter(|&c| self.route_destination_refusal_no_path(owner, origin, c, None).is_none()).collect();
        cands.sort();
        if cands.is_empty() {
            return Vec::new();
        }
        let plots: Vec<PlotIdx> = cands.iter().map(|&c| self.city_plot(c)).collect();
        let goal = |q: PlotIdx| plots.contains(&q);
        let tree = self.route_search(team, self.city_plot(origin), &goal, &|_| false);
        let mut out = Vec::new();
        for (&c, &q) in cands.iter().zip(&plots) {
            let Some(path) = tree.path_to(q).filter(|p| p.len() > 1) else { continue };
            let length = path.len() as i32 - 1;
            let water = path.iter().any(|&w| self.plot_is_water(w));
            let total = self.route_pay_total_x100(owner, origin, c, length, water, 0);
            let (h0, h1) = (total - total / 2, total / 2);
            let dest_pct = self.d.defines.trade_dest_percent;
            out.push(RouteOffer {
                destination: c,
                length,
                seas: self.route_path_seas(&path),
                path,
                owner_pay_x100: h0 + h1,
                dest_pay_x100: h0 * dest_pct / 100 + h1 * dest_pct / 100,
                foreign: self.cities[c].owner != owner,
            });
        }
        out
    }

}
