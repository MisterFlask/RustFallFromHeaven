//! Trade (docs/TRADE.md): caravans: raids (4.3), closing and ending (3.4), the turn start, movement (4.1).

use crate::game::Game;

use super::*;

impl Game {
    // ---------------------------------------------------------------------------------------
    // Raids (4.3)

    /// The hook of `unit_kill`: caravan `u` of route `r` is killed (by `killer`'s side, `None`
    /// when no one killed it). Its route is no longer its own.
    pub(crate) fn unit_route_killed(&mut self, u: UnitId, r: RouteId, killer: Option<PlayerId>) {
        self.units[u].route = None;
        if !self.routes.contains(r) {
            return;
        }
        let Some(slot) = self.routes[r].slots.iter().position(|s| s.unit == Some(u)) else { return };
        self.route_caravan_lost(r, slot, killer);
    }

    /// Caravan `slot` of route `r` is lost (4.3): `killer`'s owner gains `TRADE_PLUNDER_TURNS`
    /// times its last turn's pay (none to the barbarians or with no killer), the route's growth
    /// is halved, the slot re-forms after `CARAVAN_REFORM_TURNS` turns of its owner. The unit
    /// itself is the caller's: this only unlinks it.
    pub fn route_caravan_lost(&mut self, r: RouteId, slot: usize, killer: Option<PlayerId>) {
        let d = self.d;
        let turn = self.game_turn();
        let owner = self.routes[r].owner;
        let u = self.routes[r].slots[slot].unit.take();
        let at_plot = u.filter(|&u| self.units.contains(u)).and_then(|u| self.unit_plot_opt(u));
        if let Some(u) = u.filter(|&u| self.units.contains(u)) {
            self.units[u].route = None;
        }
        let pay = self.routes[r].slots[slot].pay_x100;
        let plunder = killer.filter(|&k| !self.player_is_barbarian(k)).map_or(0, |_| round_x100(d.defines.trade_plunder_turns * pay).max(0));
        {
            let route = &mut self.routes[r];
            route.growth /= 2;
            let s = &mut route.slots[slot];
            s.reform_turns = d.defines.caravan_reform_turns.max(1);
            s.bound = Leg::Out;
            s.lost_turn = turn;
            s.lost_to = killer;
        }
        if let Some(k) = killer {
            if plunder > 0 {
                self.players[k].gold += plunder;
                self.tel_trade(k, "trade_plunder", "gold", plunder as i64);
            }
        }
        // Telemetry: who and where.
        let owner_team = self.players[owner].team;
        let by = match killer {
            None => "other",
            Some(k) if self.player_is_barbarian(k) => "barbarians",
            Some(k) if self.teams[owner_team].at_war[self.players[k].team] => "war",
            Some(_) => "hidden",
        };
        let place = match at_plot {
            Some(q) if self.plot_is_water(q) => "water",
            Some(q) => match self.plot_get_team(q) {
                None => "unowned",
                Some(t) if t == owner_team => "own",
                Some(_) => "foreign",
            },
            None => "other",
        };
        self.tel_trade(owner, "trade_caravan_lost_by", by, 1);
        self.tel_trade(owner, "trade_caravan_lost_at", place, 1);
        let (on, dn) = (self.city_name_or(self.routes[r].origin), self.city_name_or(self.routes[r].destination));
        let growth = self.routes[r].growth;
        let who = match killer {
            None => String::new(),
            Some(k) if self.player_is_barbarian(k) => " to the barbarians".to_string(),
            Some(k) => format!(" to {}", self.player_get_civilization_short_description(k)),
        };
        let at = at_plot.map(|q| self.map.xy(q)).unwrap_or_else(|| self.route_origin_xy(r));
        self.log.msg_at(turn, Some(owner), at, || format!("Caravan of {on} to {dn} lost{who}. Growth: {growth}%."));
        if let Some(k) = killer.filter(|&k| k != owner && plunder > 0) {
            self.log.msg_at(turn, Some(k), at, || format!("Caravan of {on} to {dn} plundered: +{plunder} gold."));
        }
        self.trade_refresh_pay();
    }

    pub(crate) fn city_name_or(&self, c: CityId) -> String {
        if self.cities.contains(c) {
            self.cities[c].name.clone()
        } else {
            "a lost city".to_string()
        }
    }

    pub(crate) fn route_origin_xy(&self, r: RouteId) -> (i32, i32) {
        let o = self.routes[r].origin;
        if self.cities.contains(o) {
            (self.cities[o].x, self.cities[o].y)
        } else {
            (-1, -1)
        }
    }

    // ---------------------------------------------------------------------------------------
    // Closing, ending (3.4)

    /// Route `r` closes for `why`: its caravans are taken off the map and wait in the origin.
    pub(crate) fn route_close(&mut self, r: RouteId, why: RouteRefusal) {
        let was_open = self.routes[r].is_open();
        self.routes[r].closed = Some(why);
        self.routes[r].path.clear();
        self.routes[r].length = 0;
        for slot in 0..CARAVAN_SLOTS {
            self.caravan_remove(r, slot);
        }
        if was_open {
            let owner = self.routes[r].owner;
            let (on, dn) = (self.city_name_or(self.routes[r].origin), self.city_name_or(self.routes[r].destination));
            let text = why.text(self);
            let turn = self.game_turn();
            let at = self.route_origin_xy(r);
            self.log.msg_at(turn, Some(owner), at, || format!("Route {on} to {dn} closed: {text}"));
        }
        self.trade_refresh_pay();
    }

    /// Route `r` ends (its slot is free, its caravans gone): dissolved, its origin lost, its
    /// destination destroyed.
    pub(crate) fn route_end(&mut self, r: RouteId, why: &str) {
        if !self.routes.contains(r) {
            return;
        }
        for slot in 0..CARAVAN_SLOTS {
            self.caravan_remove(r, slot);
        }
        let owner = self.routes[r].owner;
        let (on, dn) = (self.city_name_or(self.routes[r].origin), self.city_name_or(self.routes[r].destination));
        let turn = self.game_turn();
        let at = self.route_origin_xy(r);
        let why = why.to_string();
        self.log.msg_at(turn, Some(owner), at, || format!("Route {on} to {dn} ended: {why}"));
        self.cargo_undo_route(r);
        self.routes.remove(r);
        self.trade_refresh_pay();
    }

    /// Its owner dissolves route `r` (3.4): it ends. False if there is no such route.
    pub fn route_dissolve(&mut self, r: RouteId) -> bool {
        if !self.routes.contains(r) {
            return false;
        }
        self.route_end(r, "dissolved.");
        true
    }

    /// City `c` is destroyed or leaves its owner's hands (`city_kill`): the routes from it end;
    /// those into it end too unless it is changing hands (`changing_hands`: the routes are moved
    /// to the new city by [`Game::routes_city_changed_hands`]).
    pub(crate) fn routes_on_city_gone(&mut self, c: CityId, changing_hands: bool) {
        for r in self.routes.ids() {
            if !self.routes.contains(r) {
                continue;
            }
            let (o, d) = (self.routes[r].origin, self.routes[r].destination);
            if o == c {
                self.route_end(r, "the origin is lost.");
            } else if d == c && !changing_hands {
                self.route_end(r, "the destination is destroyed.");
            } else if d == c {
                // Its caravans standing in the city go back to wait in the origin.
                for slot in 0..CARAVAN_SLOTS {
                    if let Some(u) = self.routes[r].slots[slot].unit {
                        if self.units.contains(u) && self.unit_plot_opt(u) == Some(self.city_plot(c)) {
                            self.caravan_remove(r, slot);
                        }
                    }
                }
            }
        }
    }

    /// City `old` changed hands and is now `new` (`player_acquire_city`): the routes into it keep
    /// it as their destination, open or closed by 3.3 at their owner's next turn start.
    pub(crate) fn routes_city_changed_hands(&mut self, old: CityId, new: CityId) {
        for r in self.routes.ids() {
            if self.routes[r].destination == old {
                self.routes[r].destination = new;
            }
        }
        self.trade_refresh_pay();
    }

    /// War between teams `t` and `other` (3.4): each route of their players is checked; a route
    /// between the two loses each caravan standing on land the enemy owns to that land's owner
    /// (seizure), its growth goes to 0 and it closes.
    pub fn routes_on_war(&mut self, t: TeamId, other: TeamId) {
        for r in self.routes.ids() {
            if !self.routes.contains(r) {
                continue;
            }
            let owner = self.routes[r].owner;
            let team = self.players[owner].team;
            if team != t && team != other {
                continue;
            }
            let dest = self.routes[r].destination;
            if !self.cities.contains(dest) {
                continue;
            }
            let dest_team = self.city_team(dest);
            let enemy = if team == t { other } else { t };
            if dest_team == enemy {
                self.routes[r].growth = 0;
                for slot in 0..CARAVAN_SLOTS {
                    let Some(u) = self.routes[r].slots[slot].unit else { continue };
                    if !self.units.contains(u) {
                        continue;
                    }
                    let Some(q) = self.unit_plot_opt(u) else { continue };
                    let Some(land) = self.map.plots[q].owner.filter(|&o| self.players[o].team == enemy) else { continue };
                    self.route_caravan_lost(r, slot, Some(land));
                    let (turn, at) = (self.game_turn(), self.map.xy(q));
                    self.log.msg_at(turn, Some(land), at, || "Caravan seized.".to_string());
                    if self.units.contains(u) {
                        self.unit_kill(u, false, None);
                    }
                }
                self.routes[r].growth = 0;
            }
            if self.routes[r].is_open() {
                if let Err(why) = self.route_check(r) {
                    self.route_close(r, why);
                }
            }
        }
        self.trade_refresh_pay();
    }

    // ---------------------------------------------------------------------------------------
    // The turn start (3.4, 4.2, 4.3, 6.2)

    /// The start of `p`'s turn, after the seas' refresh (docs/TRADE.md, 3.4): each route of `p`
    /// is checked; re-form timers count down; an open route's waiting caravans form in the
    /// origin, its guard is set, and with both caravans alive its growth rises; a closed one
    /// keeps its caravans off the map. Messages on a change; telemetry of route-turns.
    pub fn player_update_routes(&mut self, p: PlayerId) {
        let routes = self.player_routes(p);
        if routes.is_empty() {
            // (Resources and networks change between turns: every route's cargo and pay are
            // refreshed at every turn start.)
            self.trade_refresh_pay();
            return;
        }
        let d = self.d;
        let turn = self.game_turn();
        for r in routes {
            if !self.routes.contains(r) {
                continue;
            }
            // An origin no longer its owner's ends the route (a defensive check: city_kill ends it).
            let origin = self.routes[r].origin;
            if !self.cities.contains(origin) || self.cities[origin].owner != p {
                self.route_end(r, "the origin is lost.");
                continue;
            }
            for s in self.routes[r].slots.iter_mut() {
                if s.unit.is_none() && s.reform_turns > 0 {
                    s.reform_turns -= 1;
                }
            }
            match self.route_check(r) {
                Ok(path) => {
                    let reopened = !self.routes[r].is_open();
                    {
                        let route = &mut self.routes[r];
                        route.length = path.len() as i32 - 1;
                        route.path = path;
                        route.closed = None;
                    }
                    if reopened {
                        let (on, dn) = (self.city_name_or(origin), self.city_name_or(self.routes[r].destination));
                        let at = self.route_origin_xy(r);
                        self.log.msg_at(turn, Some(p), at, || format!("Route {on} to {dn} open again."));
                    }
                    for slot in 0..CARAVAN_SLOTS {
                        let s = &self.routes[r].slots[slot];
                        let waiting = s.unit.is_none() && s.reform_turns == 0 && (slot == 0 || self.routes[r].arrived);
                        if !waiting {
                            continue;
                        }
                        let was_lost = s.lost_turn >= 0;
                        if self.caravan_form(r, slot).is_some() && was_lost && !reopened {
                            let (on, dn) = (self.city_name_or(origin), self.city_name_or(self.routes[r].destination));
                            let at = self.route_origin_xy(r);
                            self.log.msg_at(turn, Some(p), at, || format!("Caravan of {on} to {dn} re-formed."));
                        }
                    }
                    for u in self.routes[r].caravans() {
                        if self.units.contains(u) {
                            self.caravan_set_guard(r, u);
                        }
                    }
                    if self.routes[r].slots.iter().all(|s| s.unit.is_some()) {
                        let route = &mut self.routes[r];
                        route.growth = (route.growth + d.defines.trade_growth_percent).min(d.defines.trade_growth_max.max(0));
                    }
                    self.tel_trade(p, "trade_route_turn", "open", 1);
                }
                Err(why) => {
                    if self.routes[r].is_open() {
                        self.route_close(r, why);
                    } else {
                        self.routes[r].closed = Some(why);
                    }
                    if matches!(why, RouteRefusal::AtWar(_)) {
                        self.routes[r].growth = 0;
                    }
                    self.tel_trade(p, "trade_route_turn", why.key(), 1);
                }
            }
        }
        self.trade_refresh_pay();
        self.cargo_telemetry(p);
    }

    // ---------------------------------------------------------------------------------------
    // Movement (4.1)

    /// Whether a unit `team` is at war with stands on `plot` (a caravan waits behind it).
    pub(crate) fn route_enemy_on(&self, team: TeamId, plot: PlotIdx) -> bool {
        self.map.plots[plot].units.iter().any(|&o| self.units.contains(o) && self.unit_is_enemy(o, Some(team), Some(plot)))
    }

    /// The plots caravan `u` of route `r` walks this turn, in order, toward the city it is bound
    /// for: along the stored path, or first to the nearest plot of it (4.1: off the path).
    pub(crate) fn caravan_walk(&self, r: RouteId, u: UnitId) -> Vec<PlotIdx> {
        let route = &self.routes[r];
        let Some(here) = self.unit_plot_opt(u) else { return Vec::new() };
        let slot = route.slots.iter().position(|s| s.unit == Some(u)).expect("a caravan of the route");
        let path = &route.path;
        if path.is_empty() {
            return Vec::new();
        }
        let toward = |i: usize| -> Vec<PlotIdx> {
            match route.slots[slot].bound {
                Leg::Out => path[i + 1..].to_vec(),
                Leg::Back => path[..i].iter().rev().copied().collect(),
            }
        };
        if let Some(i) = path.iter().position(|&q| q == here) {
            return toward(i);
        }
        let team = self.players[route.owner].team;
        let Some(rejoin) = self.route_rejoin(team, here, path) else { return Vec::new() };
        let j = path.iter().position(|&q| Some(&q) == rejoin.last()).expect("on the path");
        let mut walk: Vec<PlotIdx> = rejoin[1..].to_vec();
        walk.extend(toward(j));
        walk
    }

    /// The end of `p`'s turn (4.1): each caravan of an open route steps along its path toward the
    /// city it is bound for, spending `CARAVAN_MOVE` on the path's costs; a step that costs more
    /// than is left, or onto a plot an enemy unit stands on, waits. Entering the city it is bound
    /// for ends its turn and turns it around; the first arrival at the destination makes the
    /// route pay and forms the second caravan in the origin. The move is the rules' own step
    /// (`group_move`: sight, interception, entering a city), with no movement points spent.
    pub fn player_move_caravans(&mut self, p: PlayerId) {
        let routes = self.player_routes(p);
        if routes.is_empty() {
            return;
        }
        let team = self.players[p].team;
        let budget0 = self.d.defines.caravan_move;
        // Who moves this turn: the caravans living now (one formed meanwhile waits a turn).
        let movers: Vec<(RouteId, UnitId)> = routes.iter().filter(|&&r| self.routes.contains(r) && self.routes[r].is_open()).flat_map(|&r| self.routes[r].caravans().into_iter().map(move |u| (r, u))).collect();
        let mut changed = false;
        for (r, u) in movers {
            if !self.routes.contains(r) || !self.units.contains(u) || self.units[u].route != Some(r) || !self.routes[r].is_open() {
                continue;
            }
            let walk = self.caravan_walk(r, u);
            let mut budget = budget0;
            for next in walk {
                let Some(here) = self.unit_plot_opt(u) else { break };
                let goal = self.caravan_target_plot(r, u);
                let Some(cost) = self.route_step_cost(team, here, next, Some(next) == goal) else { break };
                if cost > budget || self.route_enemy_on(team, next) {
                    break;
                }
                budget -= cost;
                let Some(g) = self.units[u].group else { break };
                let moves = self.units[u].moves;
                self.group_move(g, next, false, Some(u), false);
                if !self.units.contains(u) || self.units[u].route != Some(r) {
                    break;
                }
                self.units[u].moves = moves;
                if let Some(g) = self.units[u].group {
                    self.group_set_activity_type(g, ActivityType::ACTIVITY_SLEEP);
                }
                if Some(next) == goal {
                    self.caravan_arrive(r, u);
                    changed = true;
                    break;
                }
            }
        }
        if changed {
            self.trade_refresh_pay();
        }
    }

    /// The plot of the city caravan `u` of route `r` is bound for.
    pub(crate) fn caravan_target_plot(&self, r: RouteId, u: UnitId) -> Option<PlotIdx> {
        let route = &self.routes[r];
        let slot = route.slots.iter().position(|s| s.unit == Some(u))?;
        let c = match route.slots[slot].bound {
            Leg::Out => route.destination,
            Leg::Back => route.origin,
        };
        self.cities.contains(c).then(|| self.city_plot(c))
    }

    /// Caravan `u` of route `r` entered the city it was bound for: it turns around; the first
    /// arrival at the destination makes the route pay and forms the second caravan.
    pub(crate) fn caravan_arrive(&mut self, r: RouteId, u: UnitId) {
        let Some(slot) = self.routes[r].slots.iter().position(|s| s.unit == Some(u)) else { return };
        let out = self.routes[r].slots[slot].bound == Leg::Out;
        self.routes[r].slots[slot].bound = if out { Leg::Back } else { Leg::Out };
        if out && !self.routes[r].arrived {
            self.routes[r].arrived = true;
            let other = 1 - slot;
            if self.routes[r].slots[other].unit.is_none() && self.routes[r].slots[other].reform_turns == 0 {
                self.caravan_form(r, other);
            }
            let owner = self.routes[r].owner;
            let (on, dn) = (self.city_name_or(self.routes[r].origin), self.city_name_or(self.routes[r].destination));
            let (turn, at) = (self.game_turn(), self.route_origin_xy(r));
            self.log.msg_at(turn, Some(owner), at, || format!("Route {on} to {dn}: the first caravan arrived. The route pays."));
        }
    }

}
