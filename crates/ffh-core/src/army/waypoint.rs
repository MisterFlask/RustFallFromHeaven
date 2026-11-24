//! Waypoints (this game's own rules; docs/ARMIES.md).
//!
//! A city may have a waypoint (`City::waypoint`): a plot, or an army or fleet of its owner. A unit
//! trained there that follows it ([`Game::unit_follows_waypoint`]) walks there under an ordinary
//! go-to order (`MISSION_MOVE_TO`), with `Unit::waypoint` set while it is on its way. It is an
//! ordinary unit in every respect (its owner can order it; it attacks and is attacked), except that
//! while on its way it is in no army and no garrison (the army sync leaves it out,
//! `unit_garrison_city` is `None`, its squad body is loose).
//!
//! - **Any order from its owner ends the waypoint** for that unit: a mission pushed through the
//!   public `group_push_mission` (move, fortify, sleep, sentry, ...), joining a group that holds
//!   other units, the commands that change what it does (wake, cancel, automate, load, unload), a
//!   direct attack (`group_attack_force`), an army command on it. The rules' own pushes for the
//!   waypoint go through `group_push_mission_keep_waypoints`, and the rules' continuation of a
//!   mission (`group_auto_mission`, `group_continue_mission`) leaves it.
//! - **Once a turn** ([`Game::player_waypoints_turn`], in its owner's turn processing after the
//!   army sync): on the target's plot it arrives; an army target whose plot is not its go-to's
//!   gets a new go-to to where the army stands (no path: the waypoint ends); a plot target whose
//!   go-to the rules dropped ends.
//! - **Arrival** (also checked in every army sync): the unit joins its army by the explicit join
//!   (never taking the army over), or, on a plot target outside its owner's cities, the oldest army
//!   of its owner and domain standing there (else it stays alone: a one-unit army by the sync). In
//!   one of its owner's cities it is in the garrison by the ordinary rule.
//! - **Target gone**: an army destroyed or disbanded clears the waypoints naming it (one message per
//!   city) and ends the units' ways there; an army merged into another hands the other its
//!   waypoints; a city that leaves its owner has none.
//!
//! The AI sets no waypoints; the rules have no AI-only branch here.

use ffh_data::*;

use super::{ArmyDomain, ArmyResult};
use crate::game::Game;
use crate::ids::*;

/// Where a city's new units go, and where a unit on its way goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Waypoint {
    /// A plot of the map.
    Plot(PlotIdx),
    /// An army or a fleet of the city's owner.
    Army(ArmyId),
}

/// The path flags of a waypoint's go-to (the path check, the order, the estimate): an ordinary
/// go-to's.
const WAYPOINT_FLAGS: i32 = 0;

impl Game {
    // ---------------------------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------------------------

    /// The plot waypoint `wp` of player `p` is at now: a plot on the map, or where the army
    /// stands (`None`: the army is gone, not `p`'s, disbanding or not on the map).
    pub fn waypoint_plot(&self, p: PlayerId, wp: Waypoint) -> Option<PlotIdx> {
        match wp {
            Waypoint::Plot(q) => (q.idx() < self.map.plots.len()).then_some(q),
            Waypoint::Army(a) => self.armies.get(a).filter(|ar| ar.owner == p && ar.disband_to.is_none()).and_then(|_| self.army_plot(a)),
        }
    }

    /// A waypoint's name in messages and views ("the 2nd Army", "plot 14, 9").
    pub fn waypoint_name(&self, wp: Waypoint) -> String {
        match wp {
            Waypoint::Army(a) => self.army_display_name(a),
            Waypoint::Plot(q) => {
                let (x, y) = self.map.xy(q);
                format!("plot {x}, {y}")
            }
        }
    }

    /// The cities whose waypoint is army `a`.
    pub fn army_waypoint_cities(&self, a: ArmyId) -> Vec<CityId> {
        let Some(army) = self.armies.get(a) else { return Vec::new() };
        self.players[army.owner].cities.iter().copied().filter(|&c| self.cities[c].waypoint == Some(Waypoint::Army(a))).collect()
    }

    /// The units on their way to army `a`.
    pub fn army_waypoint_units(&self, a: ArmyId) -> Vec<UnitId> {
        let Some(army) = self.armies.get(a) else { return Vec::new() };
        self.players[army.owner].units.iter().copied().filter(|&u| self.units[u].waypoint == Some(Waypoint::Army(a))).collect()
    }

    /// Player `p`'s units on their way to a waypoint.
    pub fn player_waypoint_units(&self, p: PlayerId) -> Vec<UnitId> {
        self.players[p].units.iter().copied().filter(|&u| self.units[u].waypoint.is_some()).collect()
    }

    /// The turns unit `u` needs to reach its waypoint along the rules' path (as its go-to takes
    /// it; 0: there), `None` when it has no waypoint or no way there now.
    pub fn unit_waypoint_eta(&self, u: UnitId) -> Option<i32> {
        let unit = self.units.get(u)?;
        let to = self.waypoint_plot(unit.owner, unit.waypoint?)?;
        let here = self.unit_plot_opt(u)?;
        if here == to {
            return Some(0);
        }
        let grp = unit.group.filter(|&g| self.groups.contains(g))?;
        self.group_find_path(grp, here, to, WAYPOINT_FLAGS).map(|path| path.turns)
    }

    /// An estimate of the turns a unit of `unit_type` trained in city `c` takes to its waypoint
    /// ([`Game::march_turns`]), `None` when the city has no waypoint or there is no way.
    pub fn city_waypoint_turns(&self, c: CityId, unit_type: UnitType) -> Option<i32> {
        let city = self.cities.get(c)?;
        let to = self.waypoint_plot(city.owner, city.waypoint?)?;
        self.march_turns(city.owner, unit_type, self.city_plot(c), to)
    }

    /// Whether unit `u` follows waypoint `wp`: to an army, a unit that can serve in it; to a plot,
    /// a unit with a combat strength whose domain matches the plot (land units to land, ships to
    /// water; never air units). Settlers, workers and missionaries follow none.
    pub fn unit_follows_waypoint(&self, u: UnitId, wp: Waypoint) -> bool {
        match wp {
            Waypoint::Army(a) => self.armies.get(a).is_some_and(|ar| self.unit_army_domain(u) == Some(ar.domain)),
            Waypoint::Plot(q) => {
                // (A combat strength: an attack, as an army unit's; a caravan has none.)
                if q.idx() >= self.map.plots.len() || self.d.units[self.units[u].unit_type].attack_rating() <= 0 {
                    return false;
                }
                match self.unit_get_domain_type(u) {
                    DomainType::DOMAIN_LAND => !self.plot_is_water(q),
                    DomainType::DOMAIN_SEA => self.plot_is_water(q),
                    _ => false,
                }
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Setting it
    // ---------------------------------------------------------------------------------------

    /// Whether player `p` may give city `c` the waypoint `wp` (`None`: none), else why not.
    pub fn city_check_waypoint(&self, p: PlayerId, c: CityId, wp: Option<Waypoint>) -> ArmyResult {
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("not your city");
        }
        match wp {
            None => Ok(()),
            Some(Waypoint::Plot(q)) => {
                if q.idx() >= self.map.plots.len() {
                    return Err("that plot is not on the map");
                }
                if !self.map.plots[q].revealed[self.players[p].team] {
                    return Err("that plot is not revealed");
                }
                Ok(())
            }
            Some(Waypoint::Army(a)) => {
                let Some(army) = self.armies.get(a).filter(|ar| ar.owner == p) else { return Err("no such army") };
                let fleet = army.domain == ArmyDomain::Sea;
                if army.disband_to.is_some() {
                    return Err(if fleet { "that fleet is disbanding" } else { "that army is disbanding" });
                }
                if fleet && !self.city_is_port(c) {
                    return Err("a fleet is a waypoint only for a coastal city");
                }
                Ok(())
            }
        }
    }

    /// Player `p` gives city `c` the waypoint `wp` (`None`: none) for the units it trains from
    /// now on ([`Game::city_check_waypoint`]).
    pub fn city_set_waypoint(&mut self, p: PlayerId, c: CityId, wp: Option<Waypoint>) -> ArmyResult {
        self.city_check_waypoint(p, c, wp)?;
        self.cities[c].waypoint = wp;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------
    // Setting out (the train hook in `city_pop_order`)
    // ---------------------------------------------------------------------------------------

    /// Unit `u`, just trained by city `c` (`empire`: by an empire order), sets out for the city's
    /// waypoint if it follows it: on the target's plot already it arrives at once; with a path
    /// there it gets the go-to; without, it stays where it is, an ordinary unit (its owner is
    /// told). In a governed city only the units of empire orders follow.
    pub(crate) fn city_send_to_waypoint(&mut self, c: CityId, u: UnitId, empire: bool) {
        let Some(wp) = self.cities[c].waypoint else { return };
        if self.city_is_governed(c) && !empire {
            return;
        }
        if !self.unit_follows_waypoint(u, wp) {
            return;
        }
        let p = self.units[u].owner;
        let Some(here) = self.unit_plot_opt(u) else { return };
        let target = self.waypoint_plot(p, wp);
        if target == Some(here) {
            self.units[u].waypoint = Some(wp);
            self.tel_waypoint(p, "sent");
            self.unit_waypoint_arrive(u);
            return;
        }
        if let Some(to) = target {
            if self.unit_waypoint_go(u, to) {
                self.units[u].waypoint = Some(wp);
                self.tel_waypoint(p, "sent");
                return;
            }
        }
        let (turn, at, name, to) = (self.game_turn(), self.map.xy(here), self.unit_waypoint_label(u), self.waypoint_name(wp));
        self.log.msg_at(turn, Some(p), at, || format!("{name}: no way to {to}."));
    }

    /// Gives unit `u`'s group a go-to to `to` (the rules' own push: the waypoint stays) if the
    /// group has a path there. Returns whether its head mission is now that go-to.
    fn unit_waypoint_go(&mut self, u: UnitId, to: PlotIdx) -> bool {
        let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) else { return false };
        let Some(here) = self.unit_plot_opt(u) else { return false };
        if self.group_find_path(grp, here, to, WAYPOINT_FLAGS).is_none() {
            return false;
        }
        let (x, y) = self.map.xy(to);
        self.group_push_mission_keep_waypoints(grp, MissionType::MISSION_MOVE_TO, x, y, WAYPOINT_FLAGS, false, false, None, None, None);
        self.units.contains(u) && self.unit_waypoint_goes_to(u, to)
    }

    /// Whether unit `u`'s group's head mission is a go-to to `to`.
    fn unit_waypoint_goes_to(&self, u: UnitId, to: PlotIdx) -> bool {
        let (x, y) = self.map.xy(to);
        self.units[u]
            .group
            .and_then(|g| self.groups.get(g))
            .and_then(|grp| grp.mission_queue.first())
            .is_some_and(|m| m.mission_type == MissionType::MISSION_MOVE_TO && m.data1 == x && m.data2 == y)
    }

    /// A unit's name in waypoint messages ("Axeman").
    fn unit_waypoint_label(&self, u: UnitId) -> String {
        self.d.text.name(&self.d.units[self.units[u].unit_type].base.description).to_string()
    }

    // ---------------------------------------------------------------------------------------
    // On the way
    // ---------------------------------------------------------------------------------------

    /// Unit `u`'s waypoint ends because its owner gave it an order (no message: the order is its
    /// owner's).
    pub(crate) fn unit_clear_waypoint(&mut self, u: UnitId) {
        if self.units.get(u).is_some_and(|x| x.waypoint.is_some()) {
            self.units[u].waypoint = None;
            let p = self.units[u].owner;
            self.tel_waypoint(p, "ended");
        }
    }

    /// The waypoints of group `g`'s units end ([`Game::unit_clear_waypoint`]): its owner gave the
    /// group an order.
    pub(crate) fn group_clear_waypoints(&mut self, g: GroupId) {
        let Some(grp) = self.groups.get(g) else { return };
        if !grp.units.iter().any(|&u| self.units.get(u).is_some_and(|x| x.waypoint.is_some())) {
            return;
        }
        for u in self.groups[g].units.clone() {
            self.unit_clear_waypoint(u);
        }
    }

    /// Unit `u`'s waypoint ends without its arriving (`why`: "no way to the 2nd Army", "the 2nd
    /// Army is gone"): its missions are cleared, an ordinary unit that waits for orders.
    fn unit_waypoint_end(&mut self, u: UnitId, why: &str) {
        let p = self.units[u].owner;
        self.units[u].waypoint = None;
        if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
            if !self.groups[grp].mission_queue.is_empty() {
                self.group_clear_mission_queue(grp);
            }
        }
        self.tel_waypoint(p, "ended");
        let (turn, name, why) = (self.game_turn(), self.unit_waypoint_label(u), why.to_string());
        let at = self.unit_plot_opt(u).map_or((-1, -1), |q| self.map.xy(q));
        self.log.msg_at(turn, Some(p), at, || format!("{name}: {why}. It waits for orders."));
    }

    /// Player `p`'s units with a waypoint, once a turn (its turn processing, after the army sync):
    /// on the target's plot they arrive; toward an army whose plot is not their go-to's they get a
    /// new go-to (no path: the waypoint ends); toward a plot whose go-to the rules dropped the
    /// waypoint ends. The armies are settled again when anything changed.
    pub fn player_waypoints_turn(&mut self, p: PlayerId) {
        let mut changed = false;
        for u in self.players[p].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            let Some(wp) = self.units[u].waypoint else { continue };
            let Some(here) = self.unit_plot_opt(u) else { continue };
            let target = self.waypoint_plot(p, wp);
            if target == Some(here) {
                self.unit_waypoint_arrive(u);
                changed = true;
                continue;
            }
            let no_way = format!("no way to {}", self.waypoint_name(wp));
            match (wp, target) {
                (_, None) => {
                    self.unit_waypoint_end(u, &no_way);
                    changed = true;
                }
                (Waypoint::Army(_), Some(to)) => {
                    // (A group still busy, its move shown, takes no new order: next turn.)
                    if self.units[u].group.is_some_and(|g| self.groups.contains(g) && self.group_is_busy(g)) {
                        continue;
                    }
                    if !self.unit_waypoint_goes_to(u, to) && !self.unit_waypoint_go(u, to) {
                        if self.units.contains(u) {
                            self.unit_waypoint_end(u, &no_way);
                        }
                        changed = true;
                    }
                }
                (Waypoint::Plot(_), Some(to)) => {
                    if !self.unit_waypoint_goes_to(u, to) {
                        self.unit_waypoint_end(u, &no_way);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.player_army_sync(p);
        }
    }

    /// Player `p`'s units with a waypoint that stand on its target's plot arrive (part of the army
    /// sync, once the armies are settled with the groups). Returns whether any arrived.
    pub(crate) fn player_waypoint_arrivals(&mut self, p: PlayerId) -> bool {
        let mut any = false;
        for u in self.players[p].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            let Some(wp) = self.units[u].waypoint else { continue };
            let here = self.unit_plot_opt(u);
            if here.is_some() && self.waypoint_plot(p, wp) == here {
                self.unit_waypoint_arrive(u);
                any = true;
            }
        }
        any
    }

    /// Unit `u` arrives at its waypoint: the waypoint is cleared; to an army it joins that army
    /// (the explicit join: the army keeps its name, order and counters); on a plot outside its
    /// owner's cities it joins the oldest army of its owner and domain there, if any (else it
    /// stays alone); in one of its owner's cities it is in the garrison by the ordinary rule.
    fn unit_waypoint_arrive(&mut self, u: UnitId) {
        let Some(wp) = self.units[u].waypoint else { return };
        let p = self.units[u].owner;
        self.units[u].waypoint = None;
        self.tel_waypoint(p, "arrived");
        let Some(here) = self.unit_plot_opt(u) else { return };
        // The go-to (when the unit arrived otherwise) is done.
        if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
            if self.groups[grp].units.len() == 1 && !self.groups[grp].mission_queue.is_empty() {
                self.group_clear_mission_queue(grp);
            }
        }
        let into = match wp {
            Waypoint::Army(a) => Some(a),
            Waypoint::Plot(_) => {
                let own_city = self.map.plots[here].plot_city.is_some_and(|c| self.cities.contains(c) && self.cities[c].owner == p);
                match self.unit_army_domain(u) {
                    Some(domain) if !own_city => self.players[p]
                        .armies
                        .iter()
                        .copied()
                        .filter(|&a| self.armies[a].domain == domain && self.armies[a].disband_to.is_none() && self.army_plot(a) == Some(here))
                        .min_by_key(|&a| (self.armies[a].formed_turn, a)),
                    _ => None,
                }
            }
        };
        let Some(a) = into else { return };
        if self.army_join_units(p, &[u], a).is_ok() && self.units.contains(u) && self.units[u].army == Some(a) && self.units[u].group == self.armies[a].group {
            let (turn, name, to) = (self.game_turn(), self.unit_waypoint_label(u), self.army_display_name(a));
            self.log.msg_at(turn, Some(p), self.map.xy(here), || format!("{name} joined {to}."));
        }
    }

    // ---------------------------------------------------------------------------------------
    // Target gone
    // ---------------------------------------------------------------------------------------

    /// Army `a` is destroyed or disbanded: the cities whose waypoint it is have none (one message
    /// each), and the units on their way to it wait for orders.
    pub(crate) fn army_waypoints_gone(&mut self, a: ArmyId) {
        let Some(army) = self.armies.get(a) else { return };
        let (p, name) = (army.owner, army.name.clone());
        let turn = self.game_turn();
        for c in self.army_waypoint_cities(a) {
            self.cities[c].waypoint = None;
            let city = self.d.text.name(&self.cities[c].name).to_string();
            let at = (self.cities[c].x, self.cities[c].y);
            let name = name.clone();
            self.log.msg_at(turn, Some(p), at, || format!("{city}: waypoint cleared. The {name} is gone."));
        }
        for u in self.army_waypoint_units(a) {
            self.unit_waypoint_end(u, &format!("the {name} is gone"));
        }
    }

    /// Army `from` of `p` merged into army `to`: the cities and units naming `from` name `to`.
    pub(crate) fn army_waypoints_redirect(&mut self, p: PlayerId, from: ArmyId, to: ArmyId) {
        for c in self.players[p].cities.clone() {
            if self.cities[c].waypoint == Some(Waypoint::Army(from)) {
                self.cities[c].waypoint = Some(Waypoint::Army(to));
            }
        }
        for u in self.players[p].units.clone() {
            if self.units[u].waypoint == Some(Waypoint::Army(from)) {
                self.units[u].waypoint = Some(Waypoint::Army(to));
            }
        }
    }
}
