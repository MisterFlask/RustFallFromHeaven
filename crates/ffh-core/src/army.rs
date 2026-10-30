//! Armies, garrisons, recruitment in transit and supply (this game's own rules, not FFH2's;
//! `GAMEOPTION_ARMIES`; docs/ARMIES.md).
//!
//! **Armies.** An army is a persistent, named container of land combat units (`Army`,
//! `Unit::army`): it moves as one, as a selection group does (its units are kept in one group by
//! [`Game::player_army_sync`]), and several armies may share a plot. Heroes, mages (arcane
//! units), recon units, summons and non-combat units are not army units: they move on their own or
//! along with an army's group. An army unit on its own (after a split, a move out of a city) is
//! a one-unit army.
//!
//! **Garrisons.** An army unit in one of its owner's cities and in no army is in that city's
//! garrison. The garrison does not move: a unit that leaves the city leaves the garrison (and
//! becomes, or joins, an army). Units go from the garrison into an army on the city's plot and
//! back freely; the garrison and every army on the plot defend the city together (the rules'
//! defender choice already takes every unit on the plot).
//!
//! **Army sync.** Units change groups for many reasons (a split, a join, the AI's own grouping),
//! so the army of each group is settled by [`Game::player_army_sync`] after every human command,
//! after each army command and in the player's turn processing (`player_do_turn`, Civ4's doTurn
//! at the end of its turn): each group of army units keeps one army, the oldest group holding
//! units of an army keeps that army (a split-off part becomes a new army), a group whose units had
//! no army gets a new one, and an army without units is gone. An army whose units all went to
//! another army's group (Civ4's Group button) hands it its reinforcements, production orders and
//! city defaults; a destroyed army's orders fall back to their city's default.
//!
//! **Recruitment.** A finished land combat unit goes to its order's destination (`OrderData::
//! dest`, else the city's default `City::recruit_default`, else the city's own garrison): to the
//! garrison it appears at once, as without armies; to another city's garrison or to an army it
//! travels off the map (`Transit`): its ETA is the path from the producing city to the destination
//! over land at the unit's movement (terrain and roads, other units ignored), recomputed every turn;
//! its journey advances only while the destination is in supply. It never gets lost: when the army
//! or the city is gone or a garrison has no land path, it diverts at once to the garrison of the
//! nearest city of its owner; when the army is out of supply or out of reach by land (aboard
//! ships), it waits, and diverts after `ARMY_TRANSIT_STALL_TURNS` such turns. An army standing in
//! the producing city takes the unit at once. Units in transit pay upkeep and count against unit
//! class limits.
//!
//! **Supply.** An army is in supply within `ARMY_SUPPLY_RANGE` plots of a plot connected to one of
//! its owner's cities (the owner's plot groups, which decide trade and resource connections). Out
//! of supply, only reinforcement pauses.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::cmp::Reverse;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// A persistent army (`Game::armies`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Army {
    pub id: ArmyId,
    pub owner: PlayerId,
    pub name: String,
    /// The group its units are in (set by `player_army_sync`).
    pub group: Option<GroupId>,
    /// The size it is to be kept at: reinforcement orders ("Reinforce to N") and, later, the
    /// governed cities that feed it; 0 = none.
    pub target_size: i32,
    /// A hero leading it (none yet: commanders come with the engagement rules).
    pub commander: Option<UnitId>,
    /// Disbanded: it marches to this city, whose garrison its units join on arrival.
    pub disband_to: Option<CityId>,
    /// The game turn it was formed.
    pub formed_turn: i32,
}

/// Where a recruited unit goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecruitDest {
    /// A city's garrison (the producing city's own is the default).
    Garrison(CityId),
    /// An army.
    Army(ArmyId),
}

/// A recruited unit on its way to its destination (`Player::transit`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transit {
    pub unit_type: UnitType,
    /// Its UnitAI (the order's).
    pub unit_ai: Option<UnitAIType>,
    /// The city that trained it (its build effects come from there if it is still ours).
    pub from_city: CityId,
    /// Where it left from: the producing city's plot.
    pub from_plot: PlotIdx,
    pub dest: RecruitDest,
    /// Turns travelled (advances only while the destination is in supply).
    pub progress: i32,
    /// The journey's length in turns as last computed (-1: not yet).
    pub eta: i32,
    /// Consecutive turns the destination has been out of supply.
    pub stalled: i32,
    /// The game turn it left.
    pub left_turn: i32,
}

/// Why an army command is refused.
pub type ArmyResult = Result<(), &'static str>;

/// English ordinal ("1st", "2nd", "11th", "23rd").
fn ordinal(n: i32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

impl Game {
    /// Whether this game has armies.
    #[inline]
    pub fn armies_on(&self) -> bool {
        self.is_option(GameOptionType::GAMEOPTION_ARMIES)
    }

    /// Whether a unit of type `unit_type` is an army unit: a land combat unit that is not a mage
    /// (arcane unit) or a recon unit.
    pub fn unit_type_is_army_unit(&self, unit_type: UnitType) -> bool {
        let info = &self.d.units[unit_type];
        if info.domain_type != Some(DomainType::DOMAIN_LAND) || info.combat <= 0 {
            return false;
        }
        match info.unit_combat_type {
            Some(c) => {
                let name = self.d.reg.type_name(c);
                name != "UNITCOMBAT_ADEPT" && name != "UNITCOMBAT_RECON"
            }
            None => false,
        }
    }

    /// Whether unit `u` belongs in armies and garrisons: an army unit type, not a hero, not a
    /// summon, not the barbarians'.
    pub fn unit_is_army_unit(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        if self.player_is_barbarian(unit.owner) || unit.duration > 0 || !self.unit_type_is_army_unit(unit.unit_type) || !self.unit_can_fight(u) {
            return false;
        }
        if let Some(hero) = self.d.ty::<PromotionType>("PROMOTION_HERO") {
            if unit.has_promotion[hero] {
                return false;
            }
        }
        true
    }

    /// The city whose garrison unit `u` is in, if any: an army unit in no army, in one of its
    /// owner's cities.
    pub fn unit_garrison_city(&self, u: UnitId) -> Option<CityId> {
        let unit = &self.units[u];
        if unit.army.is_some() || !self.unit_is_army_unit(u) {
            return None;
        }
        let plot = self.unit_plot_opt(u)?;
        self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == unit.owner)
    }

    /// The units of army `a`.
    pub fn army_units(&self, a: ArmyId) -> Vec<UnitId> {
        let owner = self.armies[a].owner;
        self.players[owner].units.iter().copied().filter(|&u| self.units[u].army == Some(a)).collect()
    }

    /// The garrison of city `c`: its owner's army units on its plot in no army.
    pub fn city_garrison_units(&self, c: CityId) -> Vec<UnitId> {
        let owner = self.cities[c].owner;
        let plot = self.city_plot(c);
        self.map.plots[plot].units.iter().copied().filter(|&u| self.units[u].owner == owner && self.unit_garrison_city(u) == Some(c)).collect()
    }

    /// The plot army `a` stands on.
    pub fn army_plot(&self, a: ArmyId) -> Option<PlotIdx> {
        let g = self.armies[a].group.filter(|&g| self.groups.contains(g))?;
        self.group_plot(g)
    }

    /// The units on their way to army `a`.
    pub fn army_transit_count(&self, a: ArmyId) -> i32 {
        let owner = self.armies[a].owner;
        self.players[owner].transit.iter().filter(|t| t.dest == RecruitDest::Army(a)).count() as i32
    }

    /// A new army of `p`, named "Nth Army" with the lowest N none of its armies has.
    fn army_create(&mut self, p: PlayerId) -> ArmyId {
        self.players[p].army_count += 1;
        let taken: BTreeSet<String> = self.players[p].armies.iter().map(|&a| self.armies[a].name.clone()).collect();
        let name = (1..).map(|n| format!("{} Army", ordinal(n))).find(|n| !taken.contains(n)).expect("a free name");
        let turn = self.game_turn();
        let a = self.armies.insert_with(|id| Army { id, owner: p, name, group: None, target_size: 0, commander: None, disband_to: None, formed_turn: turn });
        self.players[p].armies.push(a);
        a
    }

    /// Army `a` is gone. Its units belong to no army; production orders and city defaults naming
    /// it fall back to the city's default (its units in transit divert on their next step).
    fn army_delete(&mut self, a: ArmyId) {
        let owner = self.armies[a].owner;
        self.armies.remove(a);
        self.players[owner].armies.retain(|&x| x != a);
        for &u in &self.players[owner].units.clone() {
            if self.units[u].army == Some(a) {
                self.units[u].army = None;
            }
        }
        for c in self.players[owner].cities.clone() {
            let city = &mut self.cities[c];
            if city.recruit_default == Some(RecruitDest::Army(a)) {
                city.recruit_default = None;
            }
            for o in city.order_queue.iter_mut() {
                if o.dest == Some(RecruitDest::Army(a)) {
                    o.dest = None;
                    o.reinforce_to = 0;
                }
            }
        }
    }

    /// Everything of `p` bound for army `from` goes to army `to` instead (`from` was merged into
    /// it): the units in transit, the production orders, the cities' defaults.
    fn army_redirect(&mut self, p: PlayerId, from: ArmyId, to: ArmyId) {
        for t in self.players[p].transit.iter_mut() {
            if t.dest == RecruitDest::Army(from) {
                t.dest = RecruitDest::Army(to);
            }
        }
        for c in self.players[p].cities.clone() {
            let city = &mut self.cities[c];
            if city.recruit_default == Some(RecruitDest::Army(from)) {
                city.recruit_default = Some(RecruitDest::Army(to));
            }
            for o in city.order_queue.iter_mut() {
                if o.dest == Some(RecruitDest::Army(from)) {
                    o.dest = Some(RecruitDest::Army(to));
                }
            }
        }
    }

    /// Settles player `p`'s armies with its groups (see the module comment): every group of
    /// army units outside a garrison has one army; the oldest group with units of an army keeps it;
    /// a disbanded army in the city it marched to joins its garrison; armies without units go.
    pub fn player_army_sync(&mut self, p: PlayerId) {
        if !self.armies_on() || self.player_is_barbarian(p) {
            return;
        }
        // Units that are no army units (a hero now, a summon) belong to no army; nor does a unit
        // to an army that is gone or not its owner's.
        for u in self.players[p].units.clone() {
            if let Some(a) = self.units[u].army {
                if !self.unit_is_army_unit(u) || !self.armies.contains(a) || self.armies[a].owner != p {
                    self.units[u].army = None;
                }
            }
        }
        let mut claimed: BTreeSet<ArmyId> = BTreeSet::new();
        // Armies whose units another army's group took (Civ4's grouping joined them): the first
        // army that took any.
        let mut absorbed: BTreeMap<ArmyId, ArmyId> = BTreeMap::new();
        for grp in self.players[p].groups.clone() {
            if !self.groups.contains(grp) {
                continue;
            }
            let members: Vec<UnitId> = self.groups[grp].units.iter().copied().filter(|&u| self.units.contains(u) && self.unit_is_army_unit(u)).collect();
            let Some(plot) = self.group_plot(grp) else { continue };
            if members.is_empty() {
                continue;
            }
            let own_city = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == p);
            // A group of garrison units stays the garrison.
            if own_city.is_some() && members.iter().all(|&u| self.units[u].army.is_none()) {
                continue;
            }
            // Its army: the first not yet claimed among its units', else a new one.
            let a = match members.iter().filter_map(|&u| self.units[u].army).find(|a| self.armies.contains(*a) && !claimed.contains(a)) {
                Some(a) => a,
                None => self.army_create(p),
            };
            claimed.insert(a);
            for &u in &members {
                if let Some(b) = self.units[u].army.filter(|&b| b != a) {
                    absorbed.entry(b).or_insert(a);
                }
                self.units[u].army = Some(a);
            }
            self.armies[a].group = Some(grp);
            // A disbanded army in its city: into the garrison.
            if let Some(c) = self.armies[a].disband_to {
                if own_city == Some(c) {
                    for &u in &members {
                        self.units[u].army = None;
                    }
                    claimed.remove(&a);
                }
            }
        }
        for a in self.players[p].armies.clone() {
            if !claimed.contains(&a) {
                if let Some(&to) = absorbed.get(&a).filter(|to| claimed.contains(to)) {
                    self.army_redirect(p, a, to);
                }
                self.army_delete(a);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Commands (the human's buttons; the AI may use them too)
    // ---------------------------------------------------------------------------------------

    /// The plot all of `units` stand on, if they are `p`'s army units on one plot.
    pub fn army_units_plot(&self, p: PlayerId, units: &[UnitId]) -> Result<PlotIdx, &'static str> {
        let mut plot = None;
        for &u in units {
            if !self.units.contains(u) || self.units[u].owner != p {
                return Err("not your unit");
            }
            if !self.unit_is_army_unit(u) {
                return Err("heroes, mages, recon units and summons are not in armies");
            }
            let q = self.unit_plot_opt(u).ok_or("a unit is not on the map")?;
            if plot.is_some_and(|x| x != q) {
                return Err("the units are not on one plot");
            }
            plot = Some(q);
        }
        plot.ok_or("no units")
    }

    /// Forms a new army of `units` (army units of `p` on one plot), in one group of their own.
    pub fn army_form(&mut self, p: PlayerId, units: &[UnitId]) -> Result<ArmyId, &'static str> {
        if !self.armies_on() {
            return Err("armies are off in this game");
        }
        self.army_units_plot(p, units)?;
        let a = self.army_create(p);
        let first = units[0];
        self.unit_join_group(first, None, true);
        let grp = self.units[first].group.expect("joined a new group");
        for &u in &units[1..] {
            self.unit_join_group(u, Some(grp), false);
        }
        for &u in units {
            self.units[u].army = Some(a);
        }
        self.armies[a].group = Some(grp);
        self.player_army_sync(p);
        Ok(a)
    }

    /// Splits `units` (of one army) off into a new army.
    pub fn army_split(&mut self, p: PlayerId, units: &[UnitId]) -> Result<ArmyId, &'static str> {
        self.army_units_plot(p, units)?;
        self.army_form(p, units)
    }

    /// `units` (army units of `p`, from armies or a garrison) join army `into` on their plot. An
    /// army left without units is gone, and the reinforcements on their way to it follow.
    pub fn army_join(&mut self, p: PlayerId, units: &[UnitId], into: ArmyId) -> ArmyResult {
        if !self.armies.contains(into) || self.armies[into].owner != p {
            return Err("no such army");
        }
        let plot = self.army_units_plot(p, units)?;
        if self.army_plot(into) != Some(plot) {
            return Err("the army is not on their plot");
        }
        let grp = self.armies[into].group.ok_or("the army has no group")?;
        let mut left: Vec<ArmyId> = Vec::new();
        for &u in units {
            if let Some(a) = self.units[u].army.filter(|&a| a != into) {
                if !left.contains(&a) {
                    left.push(a);
                }
            }
            self.unit_join_group(u, Some(grp), false);
            self.units[u].army = Some(into);
        }
        for a in left {
            if self.army_units(a).is_empty() {
                self.army_redirect(p, a, into);
            }
        }
        self.player_army_sync(p);
        Ok(())
    }

    /// Merges army `from` into army `into` (both `p`'s, on one plot).
    pub fn army_merge(&mut self, p: PlayerId, from: ArmyId, into: ArmyId) -> ArmyResult {
        if from == into || !self.armies.contains(from) || self.armies[from].owner != p {
            return Err("no such armies");
        }
        let units = self.army_units(from);
        self.army_join(p, &units, into)
    }

    /// Moves `units` (army units of `p` in one of its cities) into that city's garrison.
    pub fn army_to_garrison(&mut self, p: PlayerId, units: &[UnitId]) -> ArmyResult {
        let plot = self.army_units_plot(p, units)?;
        let c = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == p).ok_or("not in one of your cities")?;
        // The garrison's own group (any group of garrison units there), else a new one.
        let garrison_group = self.city_garrison_units(c).into_iter().find(|u| !units.contains(u)).and_then(|u| self.units[u].group);
        for (i, &u) in units.iter().enumerate() {
            self.units[u].army = None;
            match garrison_group {
                Some(gr) => self.unit_join_group(u, Some(gr), false),
                None if i == 0 => self.unit_join_group(u, None, true),
                None => {
                    let gr = self.units[units[0]].group;
                    self.unit_join_group(u, gr, false);
                }
            }
        }
        self.player_army_sync(p);
        Ok(())
    }

    /// Renames army `a`.
    pub fn army_rename(&mut self, p: PlayerId, a: ArmyId, name: &str) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err("a name of 1 to 40 characters");
        }
        self.armies[a].name = name.to_string();
        Ok(())
    }

    /// Sets the size army `a` is to be kept at (0: none).
    pub fn army_set_target_size(&mut self, p: PlayerId, a: ArmyId, size: i32) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        self.armies[a].target_size = size.clamp(0, 99);
        Ok(())
    }

    /// Disbands army `a`: its units join the garrison of their city, or march to the nearest city
    /// of `p` and join its garrison there. Reinforcements on their way go to that garrison.
    pub fn army_disband(&mut self, p: PlayerId, a: ArmyId) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        let plot = self.army_plot(a).ok_or("the army is not on the map")?;
        let here = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == p);
        let target = match here {
            Some(c) => Some(c),
            None => self.army_nearest_city(p, plot),
        };
        let Some(c) = target else { return Err("you have no city to go to") };
        for t in self.players[p].transit.iter_mut() {
            if t.dest == RecruitDest::Army(a) {
                t.dest = RecruitDest::Garrison(c);
            }
        }
        if here.is_some() {
            let units = self.army_units(a);
            return self.army_to_garrison(p, &units);
        }
        self.armies[a].disband_to = Some(c);
        if let Some(grp) = self.armies[a].group {
            let (x, y) = (self.cities[c].x, self.cities[c].y);
            self.group_push_mission(grp, MissionType::MISSION_MOVE_TO, x, y, 0, false, false, None, None, None);
        }
        Ok(())
    }

    /// Unit `u`'s best upgrade (the strongest, then the dearest): one it can afford now, or with
    /// `test_visible` one it could upgrade to with the gold.
    pub fn unit_best_upgrade(&self, u: UnitId, test_visible: bool) -> Option<UnitType> {
        self.d.units.keys().filter(|&t| self.unit_can_upgrade(u, Some(t), test_visible)).max_by_key(|&t| (self.d.units[t].combat, self.d.units[t].production_cost, Reverse(t)))
    }

    /// Army `a`'s units in the order Upgrade all takes them: the most experienced first.
    pub fn army_upgrade_order(&self, a: ArmyId) -> Vec<UnitId> {
        let mut units = self.army_units(a);
        units.sort_by_key(|&u| (Reverse(self.units[u].experience), Reverse(self.units[u].level), u));
        units
    }

    /// Upgrades the units of army `a`, the most experienced first, each to its best upgrade while
    /// the gold lasts. Returns how many were upgraded.
    pub fn army_upgrade_all(&mut self, p: PlayerId, a: ArmyId) -> Result<i32, &'static str> {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        let mut done = 0;
        for u in self.army_upgrade_order(a) {
            if !self.units.contains(u) {
                continue;
            }
            if let Some(t) = self.unit_best_upgrade(u, false) {
                self.unit_upgrade(u, Some(t));
                done += 1;
            }
        }
        self.player_army_sync(p);
        Ok(done)
    }

    /// The nearest city of `p` to `plot` (plot distance; ties: the older city).
    pub fn army_nearest_city(&self, p: PlayerId, plot: PlotIdx) -> Option<CityId> {
        let (x, y) = self.map.xy(plot);
        self.players[p].cities.iter().copied().min_by_key(|&c| (self.plot_distance(x, y, self.cities[c].x, self.cities[c].y), c))
    }

    // ---------------------------------------------------------------------------------------
    // Supply
    // ---------------------------------------------------------------------------------------

    /// Whether `plot` is in `p`'s supply: within `ARMY_SUPPLY_RANGE` plots of a plot in the same
    /// plot group (trade network) as one of `p`'s cities.
    pub fn plot_in_supply(&self, p: PlayerId, plot: PlotIdx) -> bool {
        let groups: BTreeSet<PlotGroupId> = self.players[p].cities.iter().filter_map(|&c| self.plot_get_plot_group(self.city_plot(c), p)).collect();
        if groups.is_empty() {
            return false;
        }
        let range = self.d.defines.army_supply_range.max(0);
        let (x, y) = self.map.xy(plot);
        for dy in -range..=range {
            for dx in -range..=range {
                let Some(q) = self.plot_xy(x, y, dx, dy) else { continue };
                if self.plot_distance(x, y, x + dx, y + dy) > range {
                    continue;
                }
                if self.plot_get_plot_group(q, p).is_some_and(|g| groups.contains(&g)) {
                    return true;
                }
            }
        }
        false
    }

    /// Whether army `a` is in supply.
    pub fn army_in_supply(&self, a: ArmyId) -> bool {
        self.army_plot(a).is_some_and(|q| self.plot_in_supply(self.armies[a].owner, q))
    }

    // ---------------------------------------------------------------------------------------
    // Recruitment and transit
    // ---------------------------------------------------------------------------------------

    /// The plot of a destination, if it exists and is `p`'s.
    fn recruit_dest_plot(&self, p: PlayerId, dest: RecruitDest) -> Option<PlotIdx> {
        match dest {
            RecruitDest::Garrison(c) => (self.cities.contains(c) && self.cities[c].owner == p).then(|| self.city_plot(c)),
            RecruitDest::Army(a) => (self.armies.contains(a) && self.armies[a].owner == p && self.armies[a].disband_to.is_none()).then(|| self.army_plot(a)).flatten(),
        }
    }

    /// Whether a destination is valid for a city of `p` to recruit to.
    pub fn recruit_dest_valid(&self, p: PlayerId, dest: RecruitDest) -> bool {
        self.recruit_dest_plot(p, dest).is_some()
    }

    /// The cost (in `MOVE_DENOMINATOR`ths of a move) for a unit of `unit_type` with `base_moves`
    /// of `team` to step from `from` to `to`: the rules' terrain and route costs without the
    /// unit's promotions (C++ `CvPlot::movementCost`, unit-free).
    fn transit_step_cost(&self, team: TeamId, base_moves: i32, from: PlotIdx, to: PlotIdx) -> i32 {
        let d = self.d;
        let den = d.defines.move_denominator;
        let terrain = self.map.plots[to].terrain_type;
        let mut regular = match self.map.plots[to].feature_type {
            None => terrain.map_or(1, |t| d.terrains[t].movement_cost),
            Some(f) => d.features[f].movement_cost,
        };
        if self.plot_is_hills(to) {
            regular += d.defines.hills_extra_movement;
        }
        let regular = regular.clamp(1, base_moves.max(1)) * den;
        match (self.map.plots[from].route_type, self.map.plots[to].route_type) {
            (Some(a), Some(b)) => {
                let t = &self.teams[team];
                let route = (d.routes[a].movement_cost + t.route_change[a]).max(d.routes[b].movement_cost + t.route_change[b]);
                let flat = (d.routes[a].flat_movement_cost * base_moves).max(d.routes[b].flat_movement_cost * base_moves);
                regular.min(route.min(flat)).max(1)
            }
            _ => regular,
        }
    }

    /// The journey in turns of a unit of `unit_type` of `p` from `from` to `to` over land (terrain
    /// and roads; other units, borders and war ignored), or `None` when there is no land path.
    pub fn transit_turns(&self, p: PlayerId, unit_type: UnitType, from: PlotIdx, to: PlotIdx) -> Option<i32> {
        if from == to {
            return Some(0);
        }
        let team = self.players[p].team;
        let base_moves = self.d.units[unit_type].moves.max(1);
        let per_turn = base_moves * self.d.defines.move_denominator;
        // Dijkstra over land plots; ties by plot index (deterministic).
        let mut best: BTreeMap<PlotIdx, i32> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        best.insert(from, 0);
        heap.push(Reverse((0i32, from)));
        while let Some(Reverse((cost, q))) = heap.pop() {
            if q == to {
                // Whole turns: a journey ends when the moves of its last turn run out.
                return Some((cost + per_turn - 1) / per_turn);
            }
            if best.get(&q).is_some_and(|&b| b < cost) {
                continue;
            }
            let (x, y) = self.map.xy(q);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let Some(n) = self.plot_xy(x, y, dx, dy) else { continue };
                    if self.plot_is_water(n) || self.plot_is_impassable(n) {
                        continue;
                    }
                    let c = cost + self.transit_step_cost(team, base_moves, q, n);
                    if best.get(&n).map_or(true, |&b| c < b) {
                        best.insert(n, c);
                        heap.push(Reverse((c, n)));
                    }
                }
            }
        }
        None
    }

    /// Where a unit of city `c`'s order goes (`order_dest`, else the city's default, else its own
    /// garrison); `None` = here, at once.
    pub fn city_recruit_dest(&self, c: CityId, order_dest: Option<RecruitDest>, unit_type: UnitType) -> Option<RecruitDest> {
        if !self.armies_on() || !self.unit_type_is_army_unit(unit_type) {
            return None;
        }
        let owner = self.cities[c].owner;
        let dest = order_dest.or(self.cities[c].recruit_default)?;
        if !self.recruit_dest_valid(owner, dest) {
            return None;
        }
        let plot = self.city_plot(c);
        match dest {
            RecruitDest::Garrison(x) if x == c => None,
            // An army standing in the city takes the unit at once.
            RecruitDest::Army(a) if self.army_plot(a) == Some(plot) => Some(dest),
            _ => Some(dest),
        }
    }

    /// City `c` finished a unit of `unit_type` for destination `dest`: an army in the city takes
    /// it at once (returns false: create it here and add it to the army); otherwise it leaves in
    /// transit (returns true).
    pub fn city_send_in_transit(&mut self, c: CityId, unit_type: UnitType, unit_ai: Option<UnitAIType>, dest: RecruitDest) -> bool {
        let owner = self.cities[c].owner;
        let plot = self.city_plot(c);
        if let RecruitDest::Army(a) = dest {
            if self.army_plot(a) == Some(plot) {
                return false;
            }
        }
        let turn = self.game_turn();
        let mut t = Transit { unit_type, unit_ai, from_city: c, from_plot: plot, dest, progress: 0, eta: -1, stalled: 0, left_turn: turn };
        t.eta = self.recruit_dest_plot(owner, dest).and_then(|to| self.transit_turns(owner, unit_type, plot, to)).unwrap_or(-1);
        // It still counts as being made (the unit class limits) until it arrives.
        if let Some(class) = self.d.units[unit_type].unit_class_type {
            self.players[owner].unit_class_making[class] += 1;
        }
        self.tel_transit(owner, "sent", matches!(dest, RecruitDest::Army(_)), t.eta);
        self.players[owner].transit.push(t);
        let name = self.d.text.name(&self.d.units[unit_type].base.description).to_string();
        let to = self.recruit_dest_name(dest);
        let city = self.d.text.name(&self.cities[c].name).to_string();
        self.log.msg_at(turn, Some(owner), self.map.xy(plot), || format!("{city} trained a {name}; it marches to {to}."));
        true
    }

    /// A destination's name for messages ("the 2nd Army", "the garrison of Ithralia").
    pub fn recruit_dest_name(&self, dest: RecruitDest) -> String {
        match dest {
            RecruitDest::Army(a) if self.armies.contains(a) => format!("the {}", self.armies[a].name),
            RecruitDest::Army(_) => "an army".to_string(),
            RecruitDest::Garrison(c) if self.cities.contains(c) => format!("the garrison of {}", self.d.text.name(&self.cities[c].name)),
            RecruitDest::Garrison(_) => "a garrison".to_string(),
        }
    }

    /// Player `p`'s units in transit, in its turn processing: each advances while its
    /// destination is in supply and arrives when its journey is done. A lost destination or a
    /// garrison out of reach by land diverts it to the nearest city's garrison at once; an army out
    /// of supply or out of reach by land (aboard ships, across the water) makes it wait, and after
    /// `ARMY_TRANSIT_STALL_TURNS` such turns diverts it too.
    pub fn player_do_transit(&mut self, p: PlayerId) {
        if self.players[p].transit.is_empty() {
            return;
        }
        let stall_turns = self.d.defines.army_transit_stall_turns.max(1);
        let turn = self.game_turn();
        let mut i = 0;
        while i < self.players[p].transit.len() {
            let t = self.players[p].transit[i].clone();
            let mut dest = t.dest;
            let mut diverted = false;
            let mut dest_plot = self.recruit_dest_plot(p, dest);
            let mut eta = dest_plot.and_then(|to| self.transit_turns(p, t.unit_type, t.from_plot, to));
            let in_supply = match dest {
                RecruitDest::Garrison(_) => true,
                RecruitDest::Army(a) => self.armies.contains(a) && self.army_in_supply(a),
            };
            let to_army = matches!(dest, RecruitDest::Army(_));
            let stalled = if dest_plot.is_some() && to_army && (eta.is_none() || !in_supply) { t.stalled + 1 } else { 0 };
            if dest_plot.is_none() || (eta.is_none() && !to_army) || stalled >= stall_turns {
                // To the nearest city's garrison (one reachable by land first).
                let near = dest_plot.unwrap_or(t.from_plot);
                let (x, y) = self.map.xy(near);
                let mut cities: Vec<CityId> = self.players[p].cities.clone();
                cities.sort_by_key(|&c| (self.plot_distance(x, y, self.cities[c].x, self.cities[c].y), c));
                let pick = cities.iter().copied().find(|&c| self.transit_turns(p, t.unit_type, t.from_plot, self.city_plot(c)).is_some()).or(cities.first().copied());
                let Some(c) = pick else {
                    // No city at all: it waits.
                    i += 1;
                    continue;
                };
                dest = RecruitDest::Garrison(c);
                dest_plot = Some(self.city_plot(c));
                eta = self.transit_turns(p, t.unit_type, t.from_plot, self.city_plot(c)).or(Some(0));
                diverted = true;
            }
            let waiting = !diverted && stalled > 0;
            let progress = if waiting { t.progress } else { t.progress + 1 };
            {
                let tr = &mut self.players[p].transit[i];
                tr.dest = dest;
                tr.eta = eta.unwrap_or(-1);
                tr.stalled = if diverted { 0 } else { stalled };
                tr.progress = progress;
            }
            if waiting && t.stalled == 0 {
                let name = self.d.text.name(&self.d.units[t.unit_type].base.description).to_string();
                let to = self.recruit_dest_name(dest);
                let why = if eta.is_none() { "no way there by land" } else { "the army is out of supply" };
                self.log.msg(turn, Some(p), || format!("A {name} on its way to {to} waits: {why}."));
            }
            if diverted {
                self.tel_transit(p, "diverted", matches!(t.dest, RecruitDest::Army(_)), t.progress);
                let name = self.d.text.name(&self.d.units[t.unit_type].base.description).to_string();
                let to = self.recruit_dest_name(dest);
                let was = self.recruit_dest_name(t.dest);
                self.log.msg(turn, Some(p), || format!("A {name} on its way to {was} goes to {to} instead."));
            }
            if !waiting && eta.is_some_and(|e| progress >= e) {
                let tr = self.players[p].transit.remove(i);
                self.transit_arrive(p, tr, dest_plot.expect("a destination"));
                continue;
            }
            i += 1;
        }
    }

    /// A unit in transit arrives at `plot` (its destination's): it is created there, with the
    /// build effects of the city that trained it if that city is still `p`'s, and joins its army.
    fn transit_arrive(&mut self, p: PlayerId, t: Transit, plot: PlotIdx) {
        self.tel_transit(p, "arrived", matches!(t.dest, RecruitDest::Army(_)), self.game_turn() - t.left_turn);
        if let Some(class) = self.d.units[t.unit_type].unit_class_type {
            self.players[p].unit_class_making[class] -= 1;
        }
        let (x, y) = self.map.xy(plot);
        let u = self.player_init_unit(p, t.unit_type, x, y, t.unit_ai, None);
        if self.cities.contains(t.from_city) && self.cities[t.from_city].owner == p {
            self.city_apply_build_effects(t.from_city, u);
            if self.units.contains(u) {
                // Not the producing city's rally plot: it is where it should be.
                if let Some(grp) = self.units[u].group {
                    self.group_clear_mission_queue(grp);
                }
                self.ev_unit_built(t.from_city, u);
            }
        } else {
            self.unit_finish_moves(u);
        }
        if !self.units.contains(u) {
            return;
        }
        if let RecruitDest::Army(a) = t.dest {
            if let Some(grp) = self.armies.get(a).and_then(|ar| ar.group).filter(|&g| self.groups.contains(g)) {
                self.unit_join_group(u, Some(grp), false);
                self.units[u].army = Some(a);
            }
        }
        let name = self.d.text.name(&self.d.units[t.unit_type].base.description).to_string();
        let to = self.recruit_dest_name(t.dest);
        let turn = self.game_turn();
        self.log.msg_at(turn, Some(p), (x, y), || format!("A {name} joined {to}."));
        self.player_army_sync(p);
    }
}
