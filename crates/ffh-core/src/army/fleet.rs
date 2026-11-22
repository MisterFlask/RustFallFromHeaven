//! Fleets: the armies' container at sea (this game's own rules; docs/NAVAL.md, section 7).
//!
//! A fleet is an [`Army`](super::Army) whose `domain` is [`ArmyDomain::Sea`]: a player's warships
//! and ships with cargo room (not animals, not summons, not the barbarians') serve in named fleets
//! ("1st Fleet", numbered apart from the armies), kept in one group by the same army sync. A lone
//! such ship outside a city is a one-ship fleet; a ship in a city in no fleet is simply a ship.
//! What a fleet shares with an army: the sync, form / split / join / merge / rename / disband, a
//! target size, requisition (from coastal cities: the ship sails to the fleet as a recruit
//! marches), upgrade all. What it has not: a supply line, a garrison.
//!
//! **Standing orders** (`Army::station`, 7.2), carried out by the rules:
//!
//! - **Patrol** of a sea: at the end of its owner's turn ([`Game::player_fleet_stations`], with the
//!   moves its owner left it, so that an order the owner gave in the turn is not undone) a
//!   fleet not in that sea sails (a move order over water in range, several turns if need be) to
//!   the nearest plot of it outside a city that it can reach, and takes the Patrol order there
//!   (`MISSION_PLUNDER`, docs/NAVAL.md 4.2); in the sea outside a city it takes it where it is.
//! - **Escort** of an army or a group of its owner: whenever a group of the owner ends a step on a
//!   water plot where the target now stands, the fleet moves there if it can reach it this turn
//!   ([`Game::group_escort_step`], called by `group_move`; moves made by it start no further
//!   escort, so two fleets escorting each other cannot loop). At the end of its owner's turn a
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
/// no army); or what a fleet or a land army escorts: a caravan of a route of its owner, by its
/// slot (docs/TRADE.md, 4.4: the slot, not the unit, so the escort follows a re-formed caravan).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscortTarget {
    Army(ArmyId),
    Group(GroupId),
    Caravan(RouteId, u8),
}

/// An army's or a fleet's standing order (docs/NAVAL.md, 7.2; docs/RAIDS.md, section 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Station {
    /// Under way, or waiting.
    #[default]
    None,
    /// A fleet's: Patrol of a named sea.
    Patrol(SeaId),
    /// A fleet's: Escort of an army or a group; a fleet's or a land army's: Escort of a caravan
    /// (docs/TRADE.md, 4.4).
    Escort(EscortTarget),
    /// A land army's: the Raid of an enemy city from its countryside (docs/RAIDS.md;
    /// `raid.rs`). Whether it still holds: [`Game::army_raid_city`].
    Raid(CityId),
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
        info.domain_type == Some(DomainType::DOMAIN_SEA) && !info.animal && (info.strength > 0 || info.cargo_space > 0) && !super::hero_promotion(self.d).is_some_and(|h| info.free_promotions[h])
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
        if let Some(t) = candidates.iter().copied().filter(|&t| d.units[t].strength > 0).max_by_key(|&t| (d.units[t].strength, Reverse(d.units[t].production_cost), Reverse(t))) {
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
            // (`None` while the caravan is dead or waits in its origin.)
            EscortTarget::Caravan(r, slot) => self.routes.get(r).and_then(|route| route.slots.get(slot as usize)).and_then(|s| s.unit).filter(|&u| self.units.contains(u)).and_then(|u| self.unit_plot_opt(u)),
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
            EscortTarget::Caravan(r, slot) => match self.routes.get(r) {
                Some(route) => format!("Caravan: {} to {} ({})", self.city_name_or(route.origin), self.city_name_or(route.destination), slot as usize + 1),
                None => "a caravan".to_string(),
            },
        }
    }

    /// Sets fleet `a`'s standing order (docs/NAVAL.md, 7.2), and carries it out at once (the
    /// fleet sails, takes the Patrol order, or joins its target if it can this turn); or a land
    /// army's Raid order (docs/RAIDS.md, section 3: [`Game::army_raid_refusal`]; its moves end,
    /// its units sleep). `Station::None` ends either.
    pub fn army_set_station(&mut self, p: PlayerId, a: ArmyId, station: Station) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        let fleet = self.armies[a].domain == ArmyDomain::Sea;
        match station {
            Station::None => {}
            Station::Raid(_) if fleet => return Err("a fleet does not raid"),
            Station::Raid(_) => {}
            // (A land army's only Escort: a caravan's, docs/TRADE.md 4.4.)
            Station::Escort(EscortTarget::Caravan(..)) => {}
            _ if !fleet => return Err("only a fleet takes this standing order"),
            _ => {}
        }
        if self.armies[a].disband_to.is_some() && station != Station::None {
            return Err(if fleet { "the fleet is disbanding" } else { "the army is disbanding" });
        }
        match station {
            Station::None => {
                if let Station::Raid(c) = self.armies[a].station {
                    self.armies[a].station = Station::None;
                    self.raid_order_ended(a, c, "its order");
                    return Ok(());
                }
            }
            Station::Raid(c) => {
                if let Some(why) = self.army_raid_refusal(a, c) {
                    return Err(why);
                }
                self.army_take_raid(a, c);
                return Ok(());
            }
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
            Station::Escort(EscortTarget::Caravan(r, slot)) => {
                if !self.routes.contains(r) || self.routes[r].owner != p || slot as usize >= crate::trade::CARAVAN_SLOTS {
                    return Err("no such caravan to escort");
                }
            }
        }
        self.armies[a].station = station;
        if let Station::Escort(target @ EscortTarget::Caravan(..)) = station {
            let (turn, at, name, what) = (self.game_turn(), self.army_xy(a), self.armies[a].name.clone(), self.escort_target_name(target));
            self.log.msg_at(turn, Some(p), at, || format!("{name} escorts {what}."));
        }
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
    /// order of the army or fleet whose group it is (a Raid with its messages).
    pub(crate) fn group_order_ends_station(&mut self, g: GroupId, mission: MissionType) {
        if !is_move_order(mission) || !self.groups.contains(g) {
            return;
        }
        let Some(head) = self.group_get_head_unit(g) else { return };
        if let Some(a) = self.units[head].army.filter(|&a| self.armies.contains(a) && self.armies[a].group == Some(g)) {
            let was = self.armies[a].station;
            self.armies[a].station = Station::None;
            if let Station::Raid(c) = was {
                self.raid_order_ended(a, c, "a move order");
            }
            if let Station::Escort(target @ EscortTarget::Caravan(..)) = was {
                self.caravan_escort_ended(a, target, "a move order");
            }
        }
    }

    /// Army `a`'s plot as message coordinates ((-1, -1) off the map).
    fn army_xy(&self, a: ArmyId) -> (i32, i32) {
        self.army_plot(a).map_or((-1, -1), |q| self.map.xy(q))
    }

    /// Army or fleet `a`'s Escort of a caravan ended (`why`: "a move order", "the route ended").
    pub(crate) fn caravan_escort_ended(&mut self, a: ArmyId, target: EscortTarget, why: &str) {
        let (turn, at, name, what, owner) = (self.game_turn(), self.army_xy(a), self.armies[a].name.clone(), self.escort_target_name(target), self.armies[a].owner);
        let why = why.to_string();
        self.log.msg_at(turn, Some(owner), at, || format!("{name}: escort of {what} ended: {why}."));
    }

    /// The armies and fleets of route `r`'s owner holding the Escort of its slot `slot`, in id
    /// order.
    pub fn route_slot_escorts(&self, r: RouteId, slot: u8) -> Vec<ArmyId> {
        let Some(route) = self.routes.get(r) else { return Vec::new() };
        let mut out: Vec<ArmyId> = self.players[route.owner].armies.iter().copied().filter(|&a| self.armies.get(a).is_some_and(|ar| ar.station == Station::Escort(EscortTarget::Caravan(r, slot)))).collect();
        out.sort();
        out
    }

    /// The caravans army or fleet `a` could escort now (docs/TRADE.md, 4.4): the slots of its
    /// owner's routes; a land army's only where the route's path has a land plot besides its two
    /// cities, a fleet's only where it has water. In route id order.
    pub fn army_escort_caravans(&self, a: ArmyId) -> Vec<(RouteId, u8)> {
        let Some(army) = self.armies.get(a) else { return Vec::new() };
        let fleet = army.domain == ArmyDomain::Sea;
        let mut out = Vec::new();
        for r in self.player_routes(army.owner) {
            let path = &self.routes[r].path;
            let fits = if fleet {
                path.iter().any(|&q| self.plot_is_water(q))
            } else {
                path.len() > 2 && path[1..path.len() - 1].iter().any(|&q| !self.plot_is_water(q))
            };
            if fits {
                for slot in 0..crate::trade::CARAVAN_SLOTS as u8 {
                    out.push((r, slot));
                }
            }
        }
        out
    }

    /// The escorts of route `r` wait: its caravan of `slot` (every slot: `None`) is lost or the
    /// route closed (`why`).
    pub(crate) fn caravan_escorts_wait(&mut self, r: RouteId, slot: Option<u8>, why: &str) {
        let turn = self.game_turn();
        for s in 0..crate::trade::CARAVAN_SLOTS as u8 {
            if slot.is_some_and(|x| x != s) {
                continue;
            }
            for a in self.route_slot_escorts(r, s) {
                let (at, name, what, owner) = (self.army_xy(a), self.armies[a].name.clone(), self.escort_target_name(EscortTarget::Caravan(r, s)), self.armies[a].owner);
                let why = why.to_string();
                self.log.msg_at(turn, Some(owner), at, || format!("{name} waits: {what}: {why}"));
            }
        }
    }

    /// Route `r` ends: the Escorts of its caravans end.
    pub(crate) fn caravan_escorts_route_ended(&mut self, r: RouteId) {
        for s in 0..crate::trade::CARAVAN_SLOTS as u8 {
            for a in self.route_slot_escorts(r, s) {
                let target = EscortTarget::Caravan(r, s);
                self.caravan_escort_ended(a, target, "the route ended");
                self.armies[a].station = Station::None;
            }
        }
    }

    /// Whether an escort of caravan slot `slot` of route `r` standing with it on `here` holds it
    /// back from the step to `next` (docs/TRADE.md, 4.4): a step within one domain that the
    /// escort cannot take this turn (no moves, or it may not enter).
    pub(crate) fn caravan_held_by_escort(&self, r: RouteId, slot: u8, here: PlotIdx, next: PlotIdx) -> bool {
        let water = self.plot_is_water(here);
        if water != self.plot_is_water(next) {
            return false;
        }
        for a in self.route_slot_escorts(r, slot) {
            let army = &self.armies[a];
            if (army.domain == ArmyDomain::Sea) != water {
                continue;
            }
            let Some(grp) = army.group.filter(|&gr| self.groups.contains(gr)) else { continue };
            if self.group_plot(grp) != Some(here) {
                continue;
            }
            let units = &self.groups[grp].units;
            if units.is_empty() || !units.iter().all(|&u| self.unit_can_move(u)) || !self.group_can_move_into(grp, next, false) || self.group_is_busy(grp) {
                return true;
            }
        }
        false
    }

    /// Player `p`'s escorts of caravans carry out their order at the end of its turn, after its
    /// caravans moved (`player_move_caravans`): see [`Game::caravan_escort_step`].
    pub fn player_caravan_escorts(&mut self, p: PlayerId) {
        for a in self.players[p].armies.clone() {
            if self.armies.get(a).is_some_and(|ar| matches!(ar.station, Station::Escort(EscortTarget::Caravan(..)))) {
                self.caravan_escort_step(a);
            }
        }
    }

    /// Army or fleet `a` escorting a caravan: the route gone ends the order; the caravan dead or
    /// waiting, or on a plot of the other domain: the escort waits where it is, asleep; else it
    /// joins it if it can this turn, else moves toward it within its own domain (a fleet over
    /// water in range; an army over land, never in boats).
    pub fn caravan_escort_step(&mut self, a: ArmyId) {
        let Some(army) = self.armies.get(a) else { return };
        let Station::Escort(target @ EscortTarget::Caravan(r, _)) = army.station else { return };
        let fleet = army.domain == ArmyDomain::Sea;
        if !self.routes.contains(r) {
            self.caravan_escort_ended(a, target, "the route ended");
            self.armies[a].station = Station::None;
            return;
        }
        let Some(grp) = army.group.filter(|&g| self.groups.contains(g)) else { return };
        if self.group_is_busy(grp) {
            return;
        }
        if !self.groups[grp].mission_queue.is_empty() {
            self.group_clear_mission_queue(grp);
        }
        if let Some(to) = self.escort_target_plot(target) {
            if self.plot_is_water(to) == fleet && self.group_plot(grp) != Some(to) {
                let flags = if fleet { MOVE_SEA_RANGE } else { 0 };
                self.escort_walk(grp, to, flags);
            }
        }
        if self.groups.contains(grp) && self.groups[grp].mission_queue.is_empty() {
            self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
        }
    }

    /// Group `grp` walks toward `to` with the moves it has this turn, by the rules' path (search
    /// `flags`), every step onto a plot of `to`'s domain (land or water). The steps start no other
    /// escort.
    fn escort_walk(&mut self, grp: GroupId, to: PlotIdx, flags: i32) {
        let water = self.plot_is_water(to);
        let was = self.escorting;
        self.escorting = true;
        let mut steps = 0;
        while self.groups.contains(grp) && steps < 64 {
            steps += 1;
            let Some(at) = self.group_plot(grp) else { break };
            if at == to || !self.groups[grp].units.iter().all(|&u| self.unit_can_move(u)) {
                break;
            }
            let Some(step) = self.group_find_path(grp, at, to, flags).and_then(|path| path.first_plot) else { break };
            if step == at || self.plot_is_water(step) != water || !self.group_can_move_into(grp, step, false) {
                break;
            }
            self.group_move(grp, step, false, None, false);
        }
        self.escorting = was;
    }

    /// Group `g` attacks (`group_attack_force`): an attack ends the Raid order of the army whose
    /// group it is (docs/RAIDS.md, section 3: an attack is a move order).
    pub(crate) fn group_attack_ends_raid(&mut self, g: GroupId) {
        if !self.groups.contains(g) {
            return;
        }
        let Some(head) = self.group_get_head_unit(g) else { return };
        if let Some(a) = self.units[head].army.filter(|&a| self.armies.contains(a) && self.armies[a].group == Some(g)) {
            if let Station::Raid(c) = self.armies[a].station {
                self.armies[a].station = Station::None;
                self.raid_order_ended(a, c, "an attack");
            }
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

    /// Player `p`'s fleets carry out their standing orders at the end of its turn (its turn
    /// processing, with the moves its own orders left them): a target gone ends an Escort; see
    /// [`Game::army_station_step`].
    pub fn player_fleet_stations(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        for a in self.players[p].armies.clone() {
            if !self.armies.contains(a) {
                continue;
            }
            match self.armies[a].station {
                Station::None => {}
                // A land army's Raid (docs/RAIDS.md): ended when it no longer holds.
                Station::Raid(_) => self.army_raid_step(a),
                // A caravan's escort: after the caravans move (`player_caravan_escorts`).
                Station::Escort(EscortTarget::Caravan(..)) => {}
                _ => self.army_station_step(a),
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
            Station::Raid(_) => self.army_raid_step(a),
            Station::Escort(EscortTarget::Caravan(..)) => self.caravan_escort_step(a),
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
            // (Every step onto a plot of the target's domain: a land army escorting a caravan
            // never takes to boats.)
            if step == at || !self.group_can_move_into(grp, step, false) || self.plot_is_water(step) != self.plot_is_water(to) {
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
        if self.escorting || !self.groups.contains(g) {
            return;
        }
        // On land only a caravan's land escorts follow (docs/TRADE.md, 4.4).
        let water = self.plot_is_water(plot);
        if !water && !self.players[self.groups[g].owner].armies.iter().any(|&a| self.armies.get(a).is_some_and(|ar| matches!(ar.station, Station::Escort(EscortTarget::Caravan(..))))) {
            return;
        }
        let p = self.groups[g].owner;
        if self.player_is_barbarian(p) {
            return;
        }
        for f in self.players[p].armies.clone() {
            let Some(army) = self.armies.get(f) else { continue };
            let Station::Escort(target) = army.station else { continue };
            // In its own domain only: a fleet on water, a land army (a caravan's escort) on land.
            if (army.domain == ArmyDomain::Sea) != water {
                continue;
            }
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
