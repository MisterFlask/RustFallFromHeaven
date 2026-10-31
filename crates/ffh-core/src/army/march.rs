//! Marching recruits (this game's own rules, `GAMEOPTION_ARMIES`; docs/ARMIES.md,
//! "Recruitment").
//!
//! A finished land combat unit bound elsewhere (an army, another city's garrison) is created in
//! the city that trained it as any unit is (build effects, experience, promotions, upkeep, unit
//! class limits) and marches to its destination (`Unit::march`): the rules move it, not its
//! owner, once a turn at the end of its owner's turn ([`Game::player_do_march`], the start of its
//! turn processing, before the army sync, the requisition and the cities' production), with the
//! moves it kept through the turn, along the rules' path to where the destination is after its
//! owner moved (an army's plot, recomputed every turn; a garrison's city). Through the turn it
//! holds ([`Game::player_march_hold`], as its owner's units get their moves): its owner does not
//! move it, and it waits for no orders. A recruit trained this turn first moves at the end of the
//! next, as any new unit first moves in the next turn. It never attacks: a step that would be an
//! attack is not taken, and the path goes around visible enemy units when it can. It is a real
//! unit on the way (it can be attacked, defends itself, can die), in no army and no garrison, and
//! it takes no orders: [`Game::unit_march`] says it marches and [`Game::unit_march_refusal`] why
//! it takes none.
//!
//! - **Arrival.** On its destination's plot it arrives at once: the garrison of its city, or the
//!   army (the army's moves this turn are over, so a recruit that joins with its moves spent holds
//!   nothing back; with the army moving every turn, a recruit faster than it catches up). An army
//!   that came to it during the turn takes it as its march begins. [`Game::player_march_join`]
//!   then takes any recruit left on its destination's plot, before the army sync.
//! - **Supply gates progress**: toward an army out of supply it waits where it is.
//! - **Stalls.** A turn without progress (the army out of supply, no path, the way blocked, it
//!   cannot move) counts; after `ARMY_TRANSIT_STALL_TURNS` such turns in a row it marches to the
//!   nearest city of its owner that it can reach (another than the garrison it stalled on) and
//!   joins its garrison there. A recruit diverted once that stalls again for as long stops
//!   marching: an ordinary unit where it stands (so no recruit stays out of its owner's hands for
//!   ever).
//! - **Destination gone**: its army destroyed or its city lost, it goes at once to the nearest
//!   garrison it can reach; its army disbanded, to the city the army marches to; its army merged
//!   into another, to that army. With no city it can reach it stops marching where it stands.
//!
//! Each step is a message to its owner (setting out, waiting, diverted, arrived, stopped, killed)
//! and a telemetry count (`march`: `sent`, `arrived`, `diverted`, `stopped`, `killed`).

use ffh_data::*;

use super::RecruitDest;
use crate::game::Game;
use crate::ids::*;

/// A recruit on its way to its destination (`Unit::march`): the rules move it, not its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct March {
    /// Where it goes.
    pub dest: RecruitDest,
    /// Turns in a row without progress (the army out of supply, no way there, the way blocked).
    pub stalled: i32,
    /// The game turn it set out.
    pub left_turn: i32,
    /// The city that trained it.
    pub from_city: CityId,
    /// Sent to the nearest garrison instead of its first destination (gone, or out of reach for
    /// too long): a second such stall makes it stop marching.
    pub diverted: bool,
}

/// "a Warrior", "an Axeman".
pub(crate) fn with_article(name: &str) -> String {
    let vowel = name.chars().next().is_some_and(|c| "AEIOUaeiou".contains(c));
    format!("{} {name}", if vowel { "an" } else { "a" })
}

impl Game {
    /// Unit `u`'s march, if it is a marching recruit (it takes no orders until it arrives).
    pub fn unit_march(&self, u: UnitId) -> Option<&March> {
        self.units.get(u).and_then(|unit| unit.march.as_ref())
    }

    /// Why unit `u` takes no orders, if it is a marching recruit: "marching to the 2nd Army".
    pub fn unit_march_refusal(&self, u: UnitId) -> Option<String> {
        self.unit_march(u).map(|m| format!("marching to {}", self.recruit_dest_name(m.dest)))
    }

    /// The recruits of `p` marching to `dest`.
    pub fn player_marching_to(&self, p: PlayerId, dest: RecruitDest) -> Vec<UnitId> {
        self.players[p].units.iter().copied().filter(|&u| self.units[u].march.as_ref().is_some_and(|m| m.dest == dest)).collect()
    }

    /// The recruits marching to army `a`.
    pub fn army_marching(&self, a: ArmyId) -> Vec<UnitId> {
        match self.armies.get(a) {
            Some(army) => self.player_marching_to(army.owner, RecruitDest::Army(a)),
            None => Vec::new(),
        }
    }

    /// The recruits marching to the garrison of city `c`.
    pub fn garrison_marching(&self, c: CityId) -> Vec<UnitId> {
        match self.cities.get(c) {
            Some(city) => self.player_marching_to(city.owner, RecruitDest::Garrison(c)),
            None => Vec::new(),
        }
    }

    /// The turns marching unit `u` needs to reach its destination along the rules' path (as a
    /// go-to order would take it; 0: there), `None` when it has no way there now.
    pub fn unit_march_eta(&self, u: UnitId) -> Option<i32> {
        let m = self.unit_march(u)?;
        let to = self.recruit_dest_plot(self.units[u].owner, m.dest)?;
        let here = self.unit_plot_opt(u)?;
        if here == to {
            return Some(0);
        }
        let grp = self.units[u].group.filter(|&g| self.groups.contains(g))?;
        self.group_find_path(grp, here, to, 0).map(|path| path.turns)
    }

    /// A recruit's name in messages: "Ithralia's Axeman" (the city that trained it), else "The
    /// Axeman".
    fn recruit_name(&self, u: UnitId, from: Option<CityId>) -> String {
        let unit = self.d.text.name(&self.d.units[self.units[u].unit_type].base.description);
        match from.filter(|&c| self.cities.contains(c)) {
            Some(c) => format!("{}'s {unit}", self.d.text.name(&self.cities[c].name)),
            None => format!("The {unit}"),
        }
    }

    /// Unit `u` joins army `a`: its group and `Unit::army`.
    fn unit_join_army(&mut self, u: UnitId, a: ArmyId) {
        if let Some(grp) = self.armies.get(a).and_then(|ar| ar.group).filter(|&g| self.groups.contains(g)) {
            self.unit_join_group(u, Some(grp), false);
            self.units[u].army = Some(a);
        }
    }

    /// Unit `u`, just trained by city `c`, goes to `dest` (not the city's own garrison): an army
    /// standing in the city takes it at once; otherwise it marches there.
    pub fn unit_recruit_to(&mut self, u: UnitId, c: CityId, dest: RecruitDest) {
        let p = self.units[u].owner;
        let here = self.unit_plot(u);
        // The city's rally plot is not where it goes.
        if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
            self.group_clear_mission_queue(grp);
        }
        if let RecruitDest::Army(a) = dest {
            if self.army_plot(a) == Some(here) {
                self.unit_join_army(u, a);
                return;
            }
        }
        let turn = self.game_turn();
        self.units[u].army = None;
        self.units[u].march = Some(March { dest, stalled: 0, left_turn: turn, from_city: c, diverted: false });
        let unit_type = self.units[u].unit_type;
        let eta = self.recruit_dest_plot(p, dest).and_then(|to| self.march_turns(p, unit_type, here, to)).unwrap_or(-1);
        self.tel_march(p, "sent", matches!(dest, RecruitDest::Army(_)), eta);
        let name = self.recruit_name(u, Some(c));
        let to = self.recruit_dest_name(dest);
        self.log.msg_at(turn, Some(p), self.map.xy(here), || format!("{name} marches to {to}."));
    }

    /// Player `p`'s marching recruits as its turn begins (its units' turn, after their groups'
    /// turn woke them and gave them their moves): each is alone in its group and holds, so that
    /// its owner does not move it and it waits for no orders through the turn (it marches at the
    /// turn's end, [`Game::player_do_march`]).
    pub fn player_march_hold(&mut self, p: PlayerId) {
        if !self.armies_on() {
            return;
        }
        for u in self.players[p].units.clone() {
            if self.units.contains(u) && self.units[u].march.is_some() {
                self.unit_march_detach(u);
                if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
                    self.group_set_activity_type(grp, ActivityType::ACTIVITY_HOLD);
                }
            }
        }
    }

    /// Player `p`'s marching recruits, once a turn at the end of its turn (the start of its turn
    /// processing, before the army sync): each marches with the moves it kept, toward where its
    /// destination is after its owner moved, and arrives on its destination's plot.
    pub fn player_do_march(&mut self, p: PlayerId) {
        if !self.armies_on() {
            return;
        }
        let marching: Vec<UnitId> = self.players[p].units.iter().copied().filter(|&u| self.units[u].march.is_some()).collect();
        for u in marching {
            if self.units.contains(u) && self.units[u].march.is_some() {
                self.unit_march_step(u);
            }
        }
    }

    /// The end of player `p`'s turn (its turn processing, after the march and before the army
    /// sync): its recruits standing on their destination's plot arrive (the march's catch-all:
    /// one that reached it is in already).
    pub fn player_march_join(&mut self, p: PlayerId) {
        if !self.armies_on() {
            return;
        }
        for u in self.players[p].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            let Some(dest) = self.units[u].march.as_ref().map(|m| m.dest) else { continue };
            let here = self.unit_plot_opt(u);
            if here.is_some() && self.recruit_dest_plot(p, dest) == here {
                self.unit_march_arrive(u);
            }
        }
    }

    /// Marching unit `u` alone in its group, without missions and in no army: the rules move it.
    fn unit_march_detach(&mut self, u: UnitId) {
        if self.units[u].group.filter(|&g| self.groups.contains(g)).is_some_and(|g| self.groups[g].units.len() > 1) {
            self.unit_join_group(u, None, true);
        }
        if let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) {
            if !self.groups[grp].mission_queue.is_empty() {
                self.group_clear_mission_queue(grp);
            }
        }
        self.units[u].army = None;
    }

    /// The plot marching unit `u` goes to now: its destination's, after following a disbanded
    /// army to its city or going to the nearest garrison when the destination is gone. `None`: it
    /// stopped marching (no city it can reach) or arrived on the way.
    fn unit_march_target(&mut self, u: UnitId) -> Option<PlotIdx> {
        let p = self.units[u].owner;
        let dest = self.units[u].march.as_ref()?.dest;
        if let RecruitDest::Army(a) = dest {
            // Its army disbanded: to the garrison its units go to.
            if let Some(c) = self.armies.get(a).filter(|ar| ar.owner == p).and_then(|ar| ar.disband_to) {
                if let Some(m) = self.units[u].march.as_mut() {
                    m.dest = RecruitDest::Garrison(c);
                }
            }
        }
        let dest = self.units[u].march.as_ref()?.dest;
        if let Some(to) = self.recruit_dest_plot(p, dest) {
            return Some(to);
        }
        let why = match dest {
            RecruitDest::Army(_) => "its army is gone",
            RecruitDest::Garrison(_) => "the city is lost",
        };
        if !self.unit_march_divert(u, why, None) {
            return None;
        }
        let dest = self.units.get(u)?.march.as_ref()?.dest;
        self.recruit_dest_plot(p, dest)
    }

    /// Marching unit `u`'s march at the end of its owner's turn: it moves toward its destination
    /// with the moves it kept (see the module comment), arriving there if it gets there.
    fn unit_march_step(&mut self, u: UnitId) {
        let p = self.units[u].owner;
        self.unit_march_detach(u);
        let Some(to) = self.unit_march_target(u) else { return };
        if !self.units.contains(u) || self.units[u].march.is_none() {
            return;
        }
        let start = self.unit_plot(u);
        if start == to {
            self.unit_march_arrive(u);
            return;
        }
        let dest = self.units[u].march.as_ref().expect("marching").dest;
        let mut why: Option<&'static str> = None;
        if let RecruitDest::Army(a) = dest {
            if !self.army_in_supply(a) {
                why = Some("the army is out of supply");
            }
        }
        if why.is_none() && !self.unit_can_move(u) {
            why = Some("it cannot move");
        }
        if why.is_none() {
            while self.units.contains(u) && self.unit_can_move(u) {
                let here = self.unit_plot(u);
                if here == to {
                    break;
                }
                let Some(grp) = self.units[u].group.filter(|&g| self.groups.contains(g)) else { break };
                let Some(step) = self.group_find_path(grp, here, to, 0).and_then(|path| path.first_plot) else {
                    why = Some("no way there by land");
                    break;
                };
                // Never an attack (nor a war declared): a step it cannot simply move into waits.
                if step == here || !self.unit_can_move_into(u, step, false, false, false) {
                    why = Some("the way is blocked");
                    break;
                }
                self.unit_move(u, step);
            }
        }
        // (A move can kill it: an FfH feature or an event on the plot.)
        if !self.units.contains(u) || self.units[u].march.is_none() {
            return;
        }
        let here = self.unit_plot(u);
        // There: the garrison, or the army, whose moves this turn are over.
        if here == to {
            self.unit_march_arrive(u);
            return;
        }
        let moved = here != start;
        let stalled = {
            let m = self.units[u].march.as_mut().expect("marching");
            m.stalled = if moved { 0 } else { m.stalled + 1 };
            m.stalled
        };
        if !moved {
            let why = why.unwrap_or("the way is blocked");
            let turn = self.game_turn();
            let m = self.units[u].march.clone().expect("marching");
            if stalled == 1 {
                let name = self.recruit_name(u, Some(m.from_city));
                let to = self.recruit_dest_name(m.dest);
                self.log.msg_at(turn, Some(p), self.map.xy(here), || format!("{name} on its way to {to} waits: {why}."));
            }
            if stalled >= self.d.defines.army_transit_stall_turns.max(1) {
                if m.diverted {
                    self.unit_march_stop(u, why);
                    return;
                }
                let exclude = match m.dest {
                    RecruitDest::Garrison(c) => Some(c),
                    RecruitDest::Army(_) => None,
                };
                if !self.unit_march_divert(u, why, exclude) {
                    return;
                }
            }
        }
        // It holds until its owner's next turn wakes it (and `player_march_hold` holds it again).
        if let Some(grp) = self.units.get(u).filter(|x| x.march.is_some()).and_then(|x| x.group).filter(|&g| self.groups.contains(g)) {
            self.group_set_activity_type(grp, ActivityType::ACTIVITY_HOLD);
        }
    }

    /// Marching unit `u` arrives: it joins its army (on the army's plot) or the garrison (in the
    /// city), an ordinary unit from then on.
    fn unit_march_arrive(&mut self, u: UnitId) {
        let p = self.units[u].owner;
        let Some(m) = self.units[u].march.clone() else { return };
        let name = self.recruit_name(u, Some(m.from_city));
        let to = self.recruit_dest_name(m.dest);
        self.units[u].march = None;
        match m.dest {
            RecruitDest::Army(a) => self.unit_join_army(u, a),
            RecruitDest::Garrison(_) => self.units[u].army = None,
        }
        let turn = self.game_turn();
        self.tel_march(p, "arrived", matches!(m.dest, RecruitDest::Army(_)), turn - m.left_turn);
        let at = self.map.xy(self.unit_plot(u));
        self.log.msg_at(turn, Some(p), at, || format!("{name} joined {to}."));
    }

    /// Marching unit `u` goes to the garrison of the nearest city of its owner it can reach
    /// instead (not `exclude`), because `why`; with none it stops marching. Returns whether it
    /// still marches (or arrived: the city it stands in).
    pub(crate) fn unit_march_divert(&mut self, u: UnitId, why: &str, exclude: Option<CityId>) -> bool {
        let p = self.units[u].owner;
        let Some(m) = self.units[u].march.clone() else { return false };
        let Some(here) = self.unit_plot_opt(u) else { return false };
        let (x, y) = self.map.xy(here);
        let mut cities: Vec<CityId> = self.players[p].cities.iter().copied().filter(|&c| Some(c) != exclude).collect();
        cities.sort_by_key(|&c| (self.plot_distance(x, y, self.cities[c].x, self.cities[c].y), c));
        let grp = self.units[u].group.filter(|&g| self.groups.contains(g));
        let pick = cities.into_iter().find(|&c| {
            let to = self.city_plot(c);
            to == here || grp.is_some_and(|g| self.group_find_path(g, here, to, 0).is_some())
        });
        let Some(c) = pick else {
            self.unit_march_stop(u, why);
            return false;
        };
        let dest = RecruitDest::Garrison(c);
        if let Some(mm) = self.units[u].march.as_mut() {
            mm.dest = dest;
            mm.stalled = 0;
            mm.diverted = true;
        }
        let turn = self.game_turn();
        self.tel_march(p, "diverted", matches!(m.dest, RecruitDest::Army(_)), turn - m.left_turn);
        let name = self.recruit_name(u, Some(m.from_city));
        let (was, to) = (self.recruit_dest_name(m.dest), self.recruit_dest_name(dest));
        self.log.msg_at(turn, Some(p), (x, y), || format!("{name} on its way to {was} goes to {to} instead: {why}."));
        if self.city_plot(c) == here {
            self.unit_march_arrive(u);
        }
        true
    }

    /// Marching unit `u` stops marching where it stands (no city it can reach): an ordinary unit
    /// again, its owner's to command.
    fn unit_march_stop(&mut self, u: UnitId, why: &str) {
        let p = self.units[u].owner;
        let Some(m) = self.units[u].march.clone() else { return };
        let name = self.recruit_name(u, Some(m.from_city));
        let was = self.recruit_dest_name(m.dest);
        self.units[u].march = None;
        let turn = self.game_turn();
        self.tel_march(p, "stopped", matches!(m.dest, RecruitDest::Army(_)), turn - m.left_turn);
        let at = self.map.xy(self.unit_plot(u));
        self.log.msg_at(turn, Some(p), at, || format!("{name} on its way to {was} stops where it stands ({why}, and no city of yours can be reached): it takes your orders."));
    }

    /// Marching unit `u` dies (`unit_kill`): its owner is told, telemetry counts it.
    pub(crate) fn unit_march_lost(&mut self, u: UnitId) {
        let Some(m) = self.units[u].march.clone() else { return };
        let p = self.units[u].owner;
        let name = self.recruit_name(u, Some(m.from_city));
        let to = self.recruit_dest_name(m.dest);
        self.units[u].march = None;
        let turn = self.game_turn();
        self.tel_march(p, "killed", matches!(m.dest, RecruitDest::Army(_)), turn - m.left_turn);
        let at = self.unit_plot_opt(u).map(|q| self.map.xy(q));
        match at {
            Some(at) => self.log.msg_at(turn, Some(p), at, || format!("{name} was killed on its way to {to}.")),
            None => self.log.msg(turn, Some(p), || format!("{name} was killed on its way to {to}.")),
        }
    }
}
