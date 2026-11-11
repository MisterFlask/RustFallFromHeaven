//! Fleets: the armies' container at sea (this game's own rules; docs/NAVAL.md, section 7).
//!
//! A fleet is an [`Army`](super::Army) whose `domain` is [`ArmyDomain::Sea`]: a player's warships
//! and ships with cargo room (not animals, not summons, not the barbarians') serve in named fleets
//! ("1st Fleet", numbered apart from the armies), kept in one group by the same army sync. A lone
//! such ship outside a city is a one-ship fleet; a ship in a city in no fleet is simply a ship.
//! What a fleet shares with an army: the sync, form / split / join / merge / rename / disband, a
//! target size, requisition (from coastal cities: the ship sails to the fleet as a recruit
//! marches), upgrade all. What it has not: a supply line, a battle tactic, a garrison.
//!
//! **Standing orders** (`Army::station`, 7.2), carried out by the rules:
//!
//! - **Patrol** of a sea: at the start of its owner's turn ([`Game::player_fleet_stations`]) a
//!   fleet not in that sea sails (a move order over water in range, several turns if need be) to
//!   the nearest plot of it outside a city that it can reach, and takes the Patrol order there
//!   (`MISSION_PLUNDER`, docs/NAVAL.md 4.2); in the sea outside a city it takes it where it is.
//! - **Escort** of an army or a group of its owner: whenever a group of the owner ends a step on a
//!   water plot where the target now stands, the fleet moves there if it can reach it this turn
//!   ([`Game::group_escort_step`], called by `group_move`; moves made by it start no further
//!   escort, so two fleets escorting each other cannot loop). At the start of its owner's turn a
//!   fleet away from its target at sea sails toward it. A target on land (landed) or gone: the
//!   fleet waits where it is (a target gone ends the order).
//! - A move order (`MISSION_MOVE_TO`, `MISSION_MOVE_TO_UNIT`, `MISSION_ROUTE_TO`) given to the
//!   fleet's group by its owner, human or AI, ends the standing order
//!   ([`Game::group_order_ends_station`], called by `group_push_mission`); the rules' own moves
//!   for the order keep it.

use std::cmp::Reverse;

use ffh_data::*;

use super::{ArmyResult, RecruitDest};
use crate::defs::MOVE_SEA_RANGE;
use crate::game::Game;
use crate::ids::*;

/// What an army is: land units (an army) or ships (a fleet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArmyDomain {
    Land,
    Sea,
}

/// What a fleet escorts: an army or fleet of its owner, or a group (a ship or a unit in boats in
/// no army).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscortTarget {
    Army(ArmyId),
    Group(GroupId),
}

/// A fleet's standing order (docs/NAVAL.md, 7.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Station {
    /// Under way, or waiting.
    #[default]
    None,
    /// Patrol of a named sea.
    Patrol(SeaId),
    /// Escort of an army or a group.
    Escort(EscortTarget),
}

/// The moves that end a fleet's standing order when its owner gives them.
fn is_move_order(mission: MissionType) -> bool {
    matches!(mission, MissionType::MISSION_MOVE_TO | MissionType::MISSION_MOVE_TO_UNIT | MissionType::MISSION_ROUTE_TO)
}

impl Game {
    /// Whether units of type `t` serve in fleets: sea units with a strength or cargo room, not
    /// animals and not heroes.
    pub fn unit_type_is_fleet_unit(&self, t: UnitType) -> bool {
        let info = &self.d.units[t];
        info.domain_type == Some(DomainType::DOMAIN_SEA) && !info.animal && (info.combat > 0 || info.cargo_space > 0) && !super::hero_promotion(self.d).is_some_and(|h| info.free_promotions[h])
    }

    /// Whether unit `u` serves in fleets: a fleet unit type, not a summon, not a hero, not the
    /// barbarians'.
    pub fn unit_is_fleet_unit(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        !self.player_is_barbarian(unit.owner) && unit.duration == 0 && self.unit_type_is_fleet_unit(unit.unit_type) && !self.unit_is_animal(u) && !self.unit_is_hero(u)
    }

    /// The container unit `u` serves in: an army (land), a fleet (sea), or none.
    pub fn unit_army_domain(&self, u: UnitId) -> Option<ArmyDomain> {
        if self.unit_is_army_unit(u) {
            Some(ArmyDomain::Land)
        } else if self.unit_is_fleet_unit(u) {
            Some(ArmyDomain::Sea)
        } else {
            None
        }
    }

    /// The container units of type `t` serve in.
    pub fn unit_type_army_domain(&self, t: UnitType) -> Option<ArmyDomain> {
        if self.unit_type_is_army_unit(t) {
            Some(ArmyDomain::Land)
        } else if self.unit_type_is_fleet_unit(t) {
            Some(ArmyDomain::Sea)
        } else {
            None
        }
    }

    /// Whether army `a` is a fleet.
    pub fn army_is_fleet(&self, a: ArmyId) -> bool {
        self.armies.get(a).is_some_and(|ar| ar.domain == ArmyDomain::Sea)
    }

    /// The nearest port of `p` (a city next to water) for a fleet on `plot`: on the plot's water
    /// area first (plot distance; ties: the older city), else any.
    pub fn fleet_nearest_port(&self, p: PlayerId, plot: PlotIdx) -> Option<CityId> {
        let (x, y) = self.map.xy(plot);
        let area = self.map.plots[plot].area;
        self.players[p]
            .cities
            .iter()
            .copied()
            .filter(|&c| self.cities.contains(c) && self.city_is_port(c))
            .min_by_key(|&c| (!area.is_some_and(|ar| self.plot_is_adjacent_to_area(self.city_plot(c), ar)), self.plot_distance(x, y, self.cities[c].x, self.cities[c].y), c))
    }

    /// Whether fleet `a` of `p` may requisition from city `c` (else why not): a coastal city.
    pub(crate) fn fleet_check_requisition_city(&self, a: ArmyId, c: CityId) -> ArmyResult {
        if self.armies[a].domain == ArmyDomain::Sea && !self.city_is_port(c) {
            return Err("a fleet requisitions only from coastal cities");
        }
        Ok(())
    }

    /// The rules' choice of the ship city `c` trains for fleet `a`'s requisition (the default of
    /// `Controller::requisition_unit` for a fleet, the human's): the strongest warship the city
    /// can train, no world or national unit (ties: the cheaper, then the first); else the ship with
    /// the most cargo room.
    pub(crate) fn fleet_requisition_unit_default(&self, c: CityId) -> Option<(UnitType, UnitAIType)> {
        let d = self.d;
        let ok = |t: UnitType| self.unit_type_is_fleet_unit(t) && !d.units[t].unit_class_type.is_some_and(|k| crate::utils::is_limited_unit_class(d, k)) && self.city_can_train(c, t, false, false, false, false);
        let candidates: Vec<UnitType> = d.units.keys().filter(|&t| ok(t)).collect();
        if let Some(t) = candidates.iter().copied().filter(|&t| d.units[t].combat > 0).max_by_key(|&t| (d.units[t].combat, Reverse(d.units[t].production_cost), Reverse(t))) {
            return Some((t, UnitAIType::UNITAI_ATTACK_SEA));
        }
        candidates.into_iter().max_by_key(|&t| (d.units[t].cargo_space, Reverse(d.units[t].production_cost), Reverse(t))).map(|t| (t, UnitAIType::UNITAI_ASSAULT_SEA))
    }

    /// Whether units of `unit` may join army `a` (an army unit type for an army, a fleet unit
    /// type for a fleet).
    pub fn unit_type_fits_army(&self, unit: UnitType, a: ArmyId) -> bool {
        self.armies.get(a).is_some_and(|ar| self.unit_type_army_domain(unit) == Some(ar.domain))
    }

    // ---------------------------------------------------------------------------------------
    // Standing orders
    // ---------------------------------------------------------------------------------------

    /// The seas `p` may send a fleet to patrol: those with a plot `p`'s team has revealed that is
    /// in its range.
    pub fn fleet_patrol_seas(&self, p: PlayerId) -> Vec<SeaId> {
        let team = self.players[p].team;
        (0..self.map.seas.len())
            .map(SeaId::from_usize)
            .filter(|s| self.map.seas[s.idx()].plots.iter().any(|&w| self.map.plots[w].revealed[team] && self.plot_in_sea_range(team, w)))
            .collect()
    }

    /// Whether fleet `a` has a warship (one that can take the Patrol order).
    fn fleet_has_warship(&self, a: ArmyId) -> bool {
        self.army_units(a).iter().any(|&u| self.unit_is_warship(u))
    }

    /// The plot an escort target stands on now (`None`: gone or off the map).
    pub fn escort_target_plot(&self, target: EscortTarget) -> Option<PlotIdx> {
        match target {
            EscortTarget::Army(b) => self.armies.get(b).and_then(|_| self.army_plot(b)),
            EscortTarget::Group(gr) => self.groups.get(gr).and_then(|_| self.group_plot(gr)),
        }
    }

    /// A target's name in messages and the views ("the 2nd Army", "a Galleon").
    pub fn escort_target_name(&self, target: EscortTarget) -> String {
        match target {
            EscortTarget::Army(b) => self.recruit_dest_name(RecruitDest::Army(b)),
            EscortTarget::Group(gr) => match self.groups.get(gr).and_then(|_| self.group_get_head_unit(gr)) {
                Some(u) => super::march::with_article(self.d.text.name(&self.d.units[self.units[u].unit_type].base.description)),
                None => "a group".to_string(),
            },
        }
    }

    /// Sets fleet `a`'s standing order (docs/NAVAL.md, 7.2), and carries it out at once (the
    /// fleet sails, takes the Patrol order, or joins its target if it can this turn).
    pub fn army_set_station(&mut self, p: PlayerId, a: ArmyId, station: Station) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such fleet");
        }
        if self.armies[a].domain != ArmyDomain::Sea {
            return Err("only a fleet takes a standing order");
        }
        if self.armies[a].disband_to.is_some() && station != Station::None {
            return Err("the fleet is disbanding");
        }
        match station {
            Station::None => {}
            Station::Patrol(s) => {
                if s.idx() >= self.map.seas.len() {
                    return Err("no such sea");
                }
                if !self.fleet_has_warship(a) {
                    return Err("a fleet without a warship cannot patrol");
                }
                if !self.fleet_patrol_seas(p).contains(&s) {
                    return Err("that sea is out of your range");
                }
            }
            Station::Escort(EscortTarget::Army(b)) => {
                if b == a || !self.armies.contains(b) || self.armies[b].owner != p {
                    return Err("no such army to escort");
                }
            }
            Station::Escort(EscortTarget::Group(gr)) => {
                if !self.groups.contains(gr) || self.groups[gr].owner != p || self.armies[a].group == Some(gr) || self.groups[gr].units.is_empty() {
                    return Err("no such group to escort");
                }
            }
        }
        self.armies[a].station = station;
        self.army_station_step(a);
        Ok(())
    }

    /// The fleets of `p` escorting army `from` escort `to` instead (it merged into `to`), or, with
    /// `to` none, have no order any more (it is gone; they wait where they are).
    pub(crate) fn fleet_target_gone(&mut self, p: PlayerId, from: ArmyId, to: Option<ArmyId>) {
        for f in self.players[p].armies.clone() {
            if self.armies.get(f).is_some_and(|ar| ar.station == Station::Escort(EscortTarget::Army(from))) {
                self.armies[f].station = match to {
                    Some(t) if t != f => Station::Escort(EscortTarget::Army(t)),
                    _ => Station::None,
                };
            }
        }
    }

    /// A mission is pushed for group `g` (`group_push_mission`): a move order ends the standing
    /// order of the fleet whose group it is.
    pub(crate) fn group_order_ends_station(&mut self, g: GroupId, mission: MissionType) {
        if !is_move_order(mission) || !self.groups.contains(g) {
            return;
        }
        let Some(head) = self.group_get_head_unit(g) else { return };
        if let Some(a) = self.units[head].army.filter(|&a| self.armies.contains(a) && self.armies[a].group == Some(g)) {
            self.armies[a].station = Station::None;
        }
    }

    /// Pushes a mission for fleet `a`'s group as part of its standing order (which the push would
    /// otherwise end).
    fn fleet_push(&mut self, a: ArmyId, grp: GroupId, mission: MissionType, x: i32, y: i32, flags: i32, append: bool) {
        let station = self.armies[a].station;
        self.group_push_mission(grp, mission, x, y, flags, append, false, None, None, None);
        if self.armies.contains(a) {
            self.armies[a].station = station;
        }
    }

    /// Player `p`'s fleets carry out their standing orders as its turn begins (its units have
    /// their moves): a target gone ends an Escort; see [`Game::army_station_step`].
    pub fn player_fleet_stations(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        for a in self.players[p].armies.clone() {
            if self.armies.contains(a) && self.armies[a].station != Station::None {
                self.army_station_step(a);
            }
        }
    }

    /// Fleet `a` carries out its standing order now: on Patrol in its sea, or on its way there;
    /// beside its escort target at sea when it can reach it this turn, else on its way to it.
    pub fn army_station_step(&mut self, a: ArmyId) {
        if !self.armies.contains(a) {
            return;
        }
        let Some(grp) = self.armies[a].group.filter(|&g| self.groups.contains(g)) else { return };
        let Some(here) = self.group_plot(grp) else { return };
        if self.group_is_busy(grp) {
            return;
        }
        match self.armies[a].station {
            Station::None => {}
            Station::Patrol(s) => self.fleet_patrol_step(a, grp, here, s),
            Station::Escort(target) => {
                let Some(to) = self.escort_target_plot(target) else {
                    self.armies[a].station = Station::None;
                    return;
                };
                // (An escort is not on Patrol.)
                if self.groups[grp].activity_type == ActivityType::ACTIVITY_PLUNDER {
                    self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
                }
                // A target on land (landed): the fleet waits where it is.
                if to == here || !self.plot_is_water(to) {
                    if self.groups[grp].mission_queue.is_empty() {
                        self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
                    }
                    return;
                }
                if !self.fleet_follow(grp, to) && self.groups.contains(grp) && self.groups[grp].mission_queue.is_empty() {
                    // Out of reach this turn: it sails toward the target.
                    let (x, y) = self.map.xy(to);
                    if self.group_find_path(grp, here, to, MOVE_SEA_RANGE).is_some() {
                        self.fleet_push(a, grp, MissionType::MISSION_MOVE_TO, x, y, MOVE_SEA_RANGE, false);
                    }
                }
                if self.groups.contains(grp) && self.groups[grp].mission_queue.is_empty() {
                    self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
                }
            }
        }
    }

    /// Patrol of sea `s` for fleet `a` (group `grp` on `here`): see the module comment.
    fn fleet_patrol_step(&mut self, a: ArmyId, grp: GroupId, here: PlotIdx, s: SeaId) {
        if s.idx() >= self.map.seas.len() {
            self.armies[a].station = Station::None;
            return;
        }
        let in_sea = self.map.plots[here].sea == Some(s) && self.map.plots[here].plot_city.is_none();
        if in_sea {
            // (The Patrol order taken with no moves left sets the activity without patrolling: it
            // is taken again with the moves of the turn.)
            let patrolling = self.groups[grp].units.iter().all(|&u| !self.unit_is_warship(u) || self.units[u].blockading);
            if self.groups[grp].activity_type == ActivityType::ACTIVITY_PLUNDER && patrolling {
                return;
            }
            if self.groups[grp].activity_type == ActivityType::ACTIVITY_PLUNDER {
                self.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
            }
            if !self.groups[grp].mission_queue.is_empty() {
                self.group_clear_mission_queue(grp);
            }
            if self.group_can_start_mission(grp, MissionType::MISSION_PLUNDER, -1, -1, None, false) {
                self.fleet_push(a, grp, MissionType::MISSION_PLUNDER, -1, -1, 0, false);
            }
            return;
        }
        // On its way (its own move order, or the station's).
        if !self.groups[grp].mission_queue.is_empty() {
            return;
        }
        let team = self.players[self.armies[a].owner].team;
        let (hx, hy) = self.map.xy(here);
        let mut plots: Vec<(i32, PlotIdx)> = self.map.seas[s.idx()]
            .plots
            .iter()
            .copied()
            .filter(|&w| self.map.plots[w].plot_city.is_none() && !self.plot_is_impassable(w) && self.plot_in_sea_range(team, w))
            .map(|w| {
                let (x, y) = self.map.xy(w);
                (self.plot_distance(hx, hy, x, y), w)
            })
            .collect();
        plots.sort();
        // The nearest plot it can reach: of the four nearest by distance, the fewest turns.
        let best = plots.iter().take(4).filter_map(|&(_, w)| self.group_find_path(grp, here, w, MOVE_SEA_RANGE).map(|path| (path.turns, w))).min();
        let Some((_, w)) = best else { return };
        let (x, y) = self.map.xy(w);
        self.fleet_push(a, grp, MissionType::MISSION_MOVE_TO, x, y, MOVE_SEA_RANGE, false);
        if self.groups.contains(grp) && self.armies.contains(a) && self.armies[a].group == Some(grp) {
            // Arrived this turn: the Patrol order there; else on arrival (the queue's next mission).
            if self.groups[grp].mission_queue.is_empty() {
                if self.group_plot(grp).is_some_and(|q| self.map.plots[q].sea == Some(s)) && self.group_can_start_mission(grp, MissionType::MISSION_PLUNDER, -1, -1, None, false) {
                    self.fleet_push(a, grp, MissionType::MISSION_PLUNDER, -1, -1, 0, false);
                }
            } else {
                self.fleet_push(a, grp, MissionType::MISSION_PLUNDER, -1, -1, 0, true);
            }
        }
    }

    /// A fleet's group `grp` moves to `to` if it can reach it this turn. True if it is there.
    fn fleet_follow(&mut self, grp: GroupId, to: PlotIdx) -> bool {
        let Some(here) = self.group_plot(grp) else { return false };
        if here == to {
            return true;
        }
        if !self.group_find_path(grp, here, to, 0).is_some_and(|path| path.turns <= 1) {
            return false;
        }
        let was = self.escorting;
        self.escorting = true;
        let mut steps = 0;
        while self.groups.contains(grp) && steps < 64 {
            steps += 1;
            let Some(at) = self.group_plot(grp) else { break };
            if at == to || !self.groups[grp].units.iter().any(|&u| self.unit_can_move(u)) {
                break;
            }
            let Some(step) = self.group_find_path(grp, at, to, 0).and_then(|path| path.first_plot) else { break };
            if step == at || !self.group_can_move_into(grp, step, false) {
                break;
            }
            self.group_move(grp, step, false, None, false);
        }
        self.escorting = was;
        self.groups.contains(grp) && self.group_plot(grp) == Some(to)
    }

    /// Group `g` of a player completed a step onto water plot `plot` (`group_move`): the fleets
    /// escorting a target that now stands there move to it when they can reach it this turn
    /// (not for moves made by an escort: no chains, no loops).
    pub(crate) fn group_escort_step(&mut self, g: GroupId, plot: PlotIdx) {
        if self.escorting || !self.groups.contains(g) || !self.plot_is_water(plot) {
            return;
        }
        let p = self.groups[g].owner;
        if self.player_is_barbarian(p) {
            return;
        }
        for f in self.players[p].armies.clone() {
            let Some(army) = self.armies.get(f) else { continue };
            let Station::Escort(target) = army.station else { continue };
            if army.group == Some(g) || self.escort_target_plot(target) != Some(plot) {
                continue;
            }
            let Some(grp) = army.group.filter(|&gr| self.groups.contains(gr)) else { continue };
            if self.group_plot(grp) == Some(plot) || self.group_is_busy(grp) {
                continue;
            }
            self.fleet_follow(grp, plot);
            if self.groups.contains(grp) && self.groups[grp].mission_queue.is_empty() {
                self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
            }
        }
    }
}
