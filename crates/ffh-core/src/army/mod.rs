//! Armies, garrisons, requisition, marching recruits and supply (this game's own rules, not
//! FFH2's; `GAMEOPTION_ARMIES`; docs/ARMIES.md).
//!
//! **Armies.** An army is a persistent, named container of land combat units (`Army`,
//! `Unit::army`): it moves as one, as a selection group does (its units are kept in one group by
//! [`Game::player_army_sync`]), and several armies may share a plot. Heroes, mages (arcane
//! units), recon units, summons and non-combat units are not army units: they move on their own or
//! along with an army's group. An army unit on its own (after a split, a move out of a city) is
//! a one-unit army.
//!
//! **Garrisons.** An army unit in one of its owner's cities, in no army and not marching, is in
//! that city's garrison. The garrison does not move: a unit that leaves the city leaves the
//! garrison (and becomes, or joins, an army). Units go from the garrison into an army on the
//! city's plot and back freely; the garrison and every army on the plot defend the city together
//! (the rules' defender choice already takes every unit on the plot).
//!
//! **Army sync.** Units change groups for many reasons (a split, a join, the AI's own grouping),
//! so the army of each group is settled by [`Game::player_army_sync`] after every human command,
//! after each army command and in the player's turn processing (`player_do_turn`, Civ4's doTurn
//! at the end of its turn): each group of army units keeps one army, the oldest group holding
//! units of an army keeps that army (a split-off part becomes a new army), a group whose units had
//! no army gets a new one, and an army without units is gone. Marching recruits are left out (they
//! are in no army until they arrive). An army whose units all went to another army's group (Civ4's
//! Group button) hands it its recruits, requisition and production orders and city defaults; a
//! destroyed army's orders fall back to their city's default, its requisition orders are
//! withdrawn and its recruits on the way go to the nearest garrison.
//!
//! **Recruitment** (`requisition.rs`, `march.rs`). An army requisitions units from the cities its
//! owner picks for it (`Army::requisition`) while it is short of its target size: an order at the
//! front of the city's queue, which interrupts what the city was building. A finished land combat
//! unit goes to its order's destination (`OrderData::dest`, else the city's default
//! `City::recruit_default`, else the city's own garrison): to the garrison it stays, as without
//! armies; to another city's garrison or to an army it is created in the city as any unit and
//! *marches* there (`Unit::march`): the rules move it once a turn, before its owner plays, and it
//! takes no orders until it arrives.
//!
//! **Supply.** An army is in supply within `ARMY_SUPPLY_RANGE` plots of a plot connected to one of
//! its owner's cities (the owner's plot groups, which decide trade and resource connections). Out
//! of supply, only reinforcement pauses: its recruits wait where they are.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::cmp::Reverse;
use std::sync::OnceLock;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

mod march;
mod requisition;

pub use march::March;

/// A persistent army (`Game::armies`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Army {
    pub id: ArmyId,
    pub owner: PlayerId,
    pub name: String,
    /// The group its units are in (set by `player_army_sync`).
    pub group: Option<GroupId>,
    /// The size it is to be kept at: its requisition fills it (0 = none: it requisitions nothing).
    pub target_size: i32,
    /// A hero leading it (none yet: commanders come with the engagement rules).
    pub commander: Option<UnitId>,
    /// Disbanded: it marches to this city, whose garrison its units join on arrival.
    pub disband_to: Option<CityId>,
    /// The game turn it was formed.
    pub formed_turn: i32,
    /// The cities it requisitions units from, in the order its owner picked them (only its
    /// owner's cities: a city lost, traded, razed or flipped leaves every list).
    pub requisition: Vec<CityId>,
}

/// Where a recruited unit goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecruitDest {
    /// A city's garrison (the producing city's own is the default).
    Garrison(CityId),
    /// An army.
    Army(ArmyId),
}

/// Why an army command is refused.
pub type ArmyResult = Result<(), &'static str>;

/// `PROMOTION_HERO` (looked up once: the army checks run over every unit type and unit).
fn hero_promotion(d: &GameData) -> Option<PromotionType> {
    static HERO: OnceLock<Option<PromotionType>> = OnceLock::new();
    *HERO.get_or_init(|| d.ty::<PromotionType>("PROMOTION_HERO"))
}

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
    /// (arcane unit), a recon unit or a hero (a type with `PROMOTION_HERO` for free, which no unit
    /// of it could lose: [`Game::unit_is_army_unit`] refuses every hero).
    pub fn unit_type_is_army_unit(&self, unit_type: UnitType) -> bool {
        let info = &self.d.units[unit_type];
        if info.domain_type != Some(DomainType::DOMAIN_LAND) || info.combat <= 0 {
            return false;
        }
        if hero_promotion(self.d).is_some_and(|h| info.free_promotions[h]) {
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
        !hero_promotion(self.d).is_some_and(|h| unit.has_promotion[h])
    }

    /// The city whose garrison unit `u` is in, if any: an army unit in no army and not marching,
    /// in one of its owner's cities.
    pub fn unit_garrison_city(&self, u: UnitId) -> Option<CityId> {
        let unit = &self.units[u];
        if unit.army.is_some() || unit.march.is_some() || !self.unit_is_army_unit(u) {
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

    /// A new army of `p`, named "Nth Army" with the lowest N none of its armies has.
    fn army_create(&mut self, p: PlayerId) -> ArmyId {
        self.players[p].army_count += 1;
        let taken: BTreeSet<String> = self.players[p].armies.iter().map(|&a| self.armies[a].name.clone()).collect();
        let name = (1..).map(|n| format!("{} Army", ordinal(n))).find(|n| !taken.contains(n)).expect("a free name");
        let turn = self.game_turn();
        let a = self.armies.insert_with(|id| Army { id, owner: p, name, group: None, target_size: 0, commander: None, disband_to: None, formed_turn: turn, requisition: Vec::new() });
        self.players[p].armies.push(a);
        a
    }

    /// Army `a` is gone. Its units belong to no army; production orders and city defaults naming
    /// it fall back to the city's default, its requisition orders are withdrawn (what was put into
    /// them stays with the unit type), and its recruits on the way go to the nearest garrison.
    fn army_delete(&mut self, a: ArmyId) {
        let owner = self.armies[a].owner;
        let marching = self.army_marching(a);
        // (Not chosen anew here: the city's turn chooses for an emptied automated queue, and an
        // emptied queue of a directly ruled city waits for its owner, as any.)
        for c in self.players[owner].cities.clone() {
            self.city_withdraw_requisitions(c, a, false);
        }
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
                }
            }
        }
        for u in marching {
            self.unit_march_divert(u, "its army is gone", None);
        }
    }

    /// Everything of `p` bound for army `from` goes to army `to` instead (`from` was merged into
    /// it): the recruits on the way, the requisition (its cities join `to`'s list) and production
    /// orders, the cities' defaults.
    fn army_redirect(&mut self, p: PlayerId, from: ArmyId, to: ArmyId) {
        for u in self.army_marching(from) {
            if let Some(m) = self.units[u].march.as_mut() {
                m.dest = RecruitDest::Army(to);
            }
        }
        if self.armies.contains(from) && self.armies.contains(to) {
            for c in self.armies[from].requisition.clone() {
                if !self.armies[to].requisition.contains(&c) {
                    self.armies[to].requisition.push(c);
                }
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
        // to an army that is gone or not its owner's, nor a marching recruit.
        for u in self.players[p].units.clone() {
            if let Some(a) = self.units[u].army {
                if !self.unit_is_army_unit(u) || !self.armies.contains(a) || self.armies[a].owner != p || self.units[u].march.is_some() {
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
            let members: Vec<UnitId> = self.groups[grp].units.iter().copied().filter(|&u| self.units.contains(u) && self.unit_is_army_unit(u) && self.units[u].march.is_none()).collect();
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
            if self.units[u].march.is_some() {
                return Err("a marching recruit takes no orders until it arrives");
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
    /// army left without units is gone, and its recruits, requisition and orders follow.
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

    /// Sets the size army `a` is to be kept at (0: none, it requisitions nothing). Its requisition
    /// follows in its owner's next turn processing.
    pub fn army_set_target_size(&mut self, p: PlayerId, a: ArmyId, size: i32) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        self.armies[a].target_size = size.clamp(0, 99);
        Ok(())
    }

    /// Disbands army `a`: its units join the garrison of their city, or march to the nearest city
    /// of `p` and join its garrison there. Its recruits on the way go to that garrison; its
    /// requisition ends (its orders are withdrawn).
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
        for u in self.army_marching(a) {
            if let Some(m) = self.units[u].march.as_mut() {
                m.dest = RecruitDest::Garrison(c);
            }
        }
        self.armies[a].requisition.clear();
        for o in self.players[p].cities.clone() {
            self.city_withdraw_requisitions(o, a, true);
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
    // Recruitment destinations
    // ---------------------------------------------------------------------------------------

    /// The plot of a destination, if it exists and is `p`'s (an army disbanding is none).
    pub fn recruit_dest_plot(&self, p: PlayerId, dest: RecruitDest) -> Option<PlotIdx> {
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
    fn march_step_cost(&self, team: TeamId, base_moves: i32, from: PlotIdx, to: PlotIdx) -> i32 {
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

    /// An estimate of the march in turns of a unit of `unit_type` of `p` from `from` to `to` over
    /// land (terrain and roads; other units, borders and war ignored), or `None` when there is no
    /// land path. For choosing and showing; the march itself follows the rules' path
    /// ([`Game::unit_march_eta`]).
    pub fn march_turns(&self, p: PlayerId, unit_type: UnitType, from: PlotIdx, to: PlotIdx) -> Option<i32> {
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
                // Whole turns: a march ends when the moves of its last turn run out.
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
                    let c = cost + self.march_step_cost(team, base_moves, q, n);
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
    /// garrison); `None` = it stays here (the city's own garrison, a destination gone, no armies).
    pub fn city_recruit_dest(&self, c: CityId, order_dest: Option<RecruitDest>, unit_type: UnitType) -> Option<RecruitDest> {
        if !self.armies_on() || !self.unit_type_is_army_unit(unit_type) {
            return None;
        }
        let owner = self.cities[c].owner;
        let dest = order_dest.or(self.cities[c].recruit_default)?;
        if !self.recruit_dest_valid(owner, dest) || dest == RecruitDest::Garrison(c) {
            return None;
        }
        Some(dest)
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

    /// City `c` leaves its owner (captured, traded, flipped, razed: the start of `city_kill`): it
    /// leaves every army's requisition, and the recruits marching to its garrison go to the
    /// nearest other one.
    pub fn city_army_lost(&mut self, c: CityId) {
        if !self.armies_on() {
            return;
        }
        let owner = self.cities[c].owner;
        for a in self.players[owner].armies.clone() {
            self.armies[a].requisition.retain(|&x| x != c);
        }
        for u in self.players[owner].units.clone() {
            if self.units[u].march.as_ref().is_some_and(|m| m.dest == RecruitDest::Garrison(c)) {
                self.unit_march_divert(u, "the city is lost", Some(c));
            }
        }
    }
}
