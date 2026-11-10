//! Supply lines (this game's own rules, not FFH2's; docs/SUPPLY.md).
//!
//! **The line.** Every army needs a supply line: the cheapest path over land from its plot to a
//! source of its owner ([`Game::plot_supply_line`]): a city of its owner's team not in
//! occupation, or a ship of its owner with room for land units in a water area on which the
//! owner's team has a coastal city (the line then ends on a land plot next to the ship). Entering
//! a plot costs `SUPPLY_COST_ROAD` with a route (anyone's), `SUPPLY_COST_OWN` in the owner's
//! team's territory without one, `SUPPLY_COST_LAND` anywhere else; the line cannot enter water,
//! impassable plots, a plot an enemy unit stands on (seen or not), an enemy city, a plot next to a
//! fort an enemy unit holds, or territory its owner's borders bar. The army is supplied when the
//! cheapest path costs at most `SUPPLY_LINE_BUDGET`; an army in a city of its owner's team is
//! supplied whatever the city's state. Ties: the lower cost, the fewer plots, then the lower plot
//! index at the first difference. A crusade Host (docs/CRUSADE.md), which has no cities of its
//! own at first, counts its lenders' and the victim's cities and land as its own team's.
//!
//! **Hunger and Starvation.** `Army::unsupplied_turns`, traced at the start of its owner's turn
//! before its units heal ([`Game::player_update_supply`]): supplied sets it to 0, cut adds 1.
//! From 1 to `SUPPLY_HUNGER_TURNS` the army is hungry (its units do not heal, recruits bound for
//! it wait); above, it starves: as Hunger, and each unit loses `STARVATION_DAMAGE_PERCENT` of its
//! full HP at each of its owner's turn starts, never below `STARVATION_FLOOR_PERCENT`, and deals
//! `STARVATION_DAMAGE_DEALT_PERCENT` damage in battles (the engagement's pre-battle hook).

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// Where a supply line ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SupplySource {
    /// A city of the army's owner's team.
    City(CityId),
    /// A ship of the army's owner; the line ends on a land plot next to it (or on the ship's
    /// plot itself for an army aboard it).
    Ship(UnitId),
}

/// A supply line ([`Game::plot_supply_line`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyLine {
    /// The plots from the army's plot (first) to the source's plot (last; for a ship, the land
    /// plot next to it). One plot when the army stands on its source.
    pub path: Vec<PlotIdx>,
    /// The sum of the costs of the plots entered (the army's own plot costs nothing).
    pub cost: i32,
    pub source: SupplySource,
}

/// An army's supply state ([`Game::army_supply_state`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyState {
    /// Its counter is 0.
    Supplied,
    /// Its line was cut at the start of `turn` of its owner's turns in a row, of the `of`
    /// (`SUPPLY_HUNGER_TURNS`) turns of Hunger: no healing, no recruits.
    Hunger { turn: i32, of: i32 },
    /// Cut for more turns than Hunger lasts.
    Starvation,
}

impl SupplyState {
    /// The owner's turn starts with the line cut before Starvation sets in (1: the next one).
    /// `None` unless hungry.
    pub fn turns_to_starvation(self) -> Option<i32> {
        match self {
            SupplyState::Hunger { turn, of } => Some(of - turn + 1),
            _ => None,
        }
    }
}

/// A plot's best label in the search: its cost, its number of plots, the plot it is entered from.
#[derive(Clone, Copy)]
struct Label {
    cost: i32,
    len: i32,
    from: Option<PlotIdx>,
    done: bool,
}

impl Game {
    /// The improvements that count as forts for supply lines: `IMPROVEMENT_FORT` and what it
    /// upgrades to (a castle, a citadel).
    fn supply_fort_types(&self) -> Vec<ImprovementType> {
        let mut out = Vec::new();
        let mut next = self.d.ty::<ImprovementType>("IMPROVEMENT_FORT");
        while let Some(i) = next {
            if out.contains(&i) {
                break;
            }
            out.push(i);
            next = self.d.improvements[i].improvement_upgrade;
        }
        out
    }

    /// Whether `other`'s cities and territory are home to `team`'s supply lines: its own team's,
    /// and for a crusade Host (docs/CRUSADE.md) its lenders' and the victim's, where it heals as
    /// in friendly land.
    fn supply_home(&self, team: TeamId, other: TeamId) -> bool {
        other == team || self.crusade_host_heals_in(team, Some(other))
    }

    /// Whether a unit at war with `team` stands on `plot` (any, seen or not).
    fn supply_enemy_on(&self, team: TeamId, plot: PlotIdx) -> bool {
        self.map.plots[plot].units.iter().any(|&u| self.units.contains(u) && self.unit_is_enemy(u, Some(team), Some(plot)))
    }

    /// Whether `team` may enter the territory of `other` for a supply line: its own or a friend's,
    /// at war, open borders, the barbarians'.
    fn supply_may_enter_borders(&self, team: TeamId, other: TeamId) -> bool {
        self.team_is_friendly_territory(team, Some(other))
            || self.teams[team].at_war[other]
            || self.team_is_open_borders(team, other)
            || self.team_is_barbarian(other)
            || self.team_is_barbarian(team)
    }

    /// The cost for a supply line of `p`'s to enter `plot` (docs/SUPPLY.md, section 2), or
    /// `None` when it cannot: water, impassable, an enemy unit on it, an enemy city, next to a
    /// fort an enemy unit holds, territory `p`'s team may not enter.
    pub fn plot_supply_cost(&self, p: PlayerId, plot: PlotIdx) -> Option<i32> {
        let team = self.players[p].team;
        self.supply_cost(team, plot, &self.supply_fort_types())
    }

    fn supply_cost(&self, team: TeamId, plot: PlotIdx, forts: &[ImprovementType]) -> Option<i32> {
        if self.plot_is_water(plot) || self.plot_is_impassable(plot) {
            return None;
        }
        if self.supply_enemy_on(team, plot) {
            return None;
        }
        let plot_team = self.plot_get_team(plot);
        if let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c)) {
            let city_team = self.players[self.cities[c].owner].team;
            if self.teams[team].at_war[city_team] {
                return None;
            }
        }
        if let Some(t) = plot_team.filter(|&t| t != team) {
            if !self.supply_may_enter_borders(team, t) {
                return None;
            }
        }
        if !forts.is_empty() {
            let (x, y) = self.map.xy(plot);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let Some(q) = self.plot_xy(x, y, dx, dy) else { continue };
                    if self.map.plots[q].improvement_type.is_some_and(|i| forts.contains(&i)) && self.supply_enemy_on(team, q) {
                        return None;
                    }
                }
            }
        }
        let d = &self.d.defines;
        Some(if self.map.plots[plot].route_type.is_some() {
            d.supply_cost_road
        } else if plot_team.is_some_and(|t| self.supply_home(team, t)) {
            d.supply_cost_own
        } else {
            d.supply_cost_land
        })
    }

    /// `p`'s ships that are supply sources, by plot: a sea unit with room for land units
    /// (`cargo_capacity > 0`, land cargo, no special cargo) on a water plot of an area on which a
    /// city of `p`'s team stands. The lowest unit id per plot.
    fn supply_ships(&self, p: PlayerId) -> BTreeMap<PlotIdx, UnitId> {
        let team = self.players[p].team;
        let mut ships: BTreeMap<PlotIdx, UnitId> = BTreeMap::new();
        let candidates: Vec<UnitId> = self.players[p]
            .units
            .iter()
            .copied()
            .filter(|&u| {
                let unit = &self.units[u];
                let info = &self.d.units[unit.unit_type];
                info.domain_type == Some(DomainType::DOMAIN_SEA) && unit.cargo_capacity > 0 && self.unit_domain_cargo(u) == Some(DomainType::DOMAIN_LAND) && self.unit_special_cargo(u).is_none()
            })
            .collect();
        if candidates.is_empty() {
            return ships;
        }
        // The water areas the team's cities touch.
        let mut coastal: BTreeSet<Option<AreaId>> = BTreeSet::new();
        for q in self.player_ids().filter(|&q| self.players[q].alive && self.supply_home(team, self.players[q].team)) {
            for &c in &self.players[q].cities {
                let (x, y) = (self.cities[c].x, self.cities[c].y);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if let Some(w) = self.plot_xy(x, y, dx, dy).filter(|&w| self.plot_is_water(w)) {
                            coastal.insert(self.map.plots[w].area);
                        }
                    }
                }
            }
        }
        for u in candidates {
            let Some(plot) = self.unit_plot_opt(u) else { continue };
            if self.plot_is_water(plot) && self.map.plots[plot].area.is_some() && coastal.contains(&self.map.plots[plot].area) {
                let e = ships.entry(plot).or_insert(u);
                if u < *e {
                    *e = u;
                }
            }
        }
        ships
    }

    /// The source a supply line of `p`'s ends at on `plot` (`team`'s, `ships` from
    /// [`Game::supply_ships`]): a city of the team not in occupation, else a land plot next to a
    /// ship (the lowest unit id).
    fn supply_source_at(&self, team: TeamId, plot: PlotIdx, ships: &BTreeMap<PlotIdx, UnitId>) -> Option<SupplySource> {
        if let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c)) {
            if self.supply_home(team, self.players[self.cities[c].owner].team) && self.cities[c].occupation_timer == 0 {
                return Some(SupplySource::City(c));
            }
        }
        if ships.is_empty() || self.plot_is_water(plot) {
            return None;
        }
        let (x, y) = self.map.xy(plot);
        let mut best: Option<UnitId> = None;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let Some(&s) = self.plot_xy(x, y, dx, dy).and_then(|q| ships.get(&q)) {
                    if best.is_none_or(|b| s < b) {
                        best = Some(s);
                    }
                }
            }
        }
        best.map(SupplySource::Ship)
    }

    /// The supply line of an army of `p` on `plot` (docs/SUPPLY.md, section 2): the cheapest path
    /// over land to a source within `SUPPLY_LINE_BUDGET`, or `None` when it is cut. Pure and
    /// deterministic; nothing is cached.
    pub fn plot_supply_line(&self, p: PlayerId, start: PlotIdx) -> Option<SupplyLine> {
        let team = self.players[p].team;
        // In a city of its owner's team, whatever its state.
        if let Some(c) = self.map.plots[start].plot_city.filter(|&c| self.cities.contains(c)) {
            if self.supply_home(team, self.players[self.cities[c].owner].team) {
                return Some(SupplyLine { path: vec![start], cost: 0, source: SupplySource::City(c) });
            }
        }
        let ships = self.supply_ships(p);
        // Aboard (on the plot of) one of its source ships.
        if let Some(&s) = ships.get(&start) {
            return Some(SupplyLine { path: vec![start], cost: 0, source: SupplySource::Ship(s) });
        }
        if let Some(source) = self.supply_source_at(team, start, &ships) {
            return Some(SupplyLine { path: vec![start], cost: 0, source });
        }
        let budget = self.d.defines.supply_line_budget;
        let forts = self.supply_fort_types();
        let mut labels: BTreeMap<PlotIdx, Label> = BTreeMap::new();
        // The path to `q` from the start, through the labels (the start first).
        let path_of = |labels: &BTreeMap<PlotIdx, Label>, q: PlotIdx| -> Vec<PlotIdx> {
            let mut v = vec![q];
            let mut at = q;
            while let Some(f) = labels.get(&at).and_then(|l| l.from) {
                v.push(f);
                at = f;
            }
            v.reverse();
            v
        };
        let mut costs: BTreeMap<PlotIdx, Option<i32>> = BTreeMap::new();
        let mut heap: BinaryHeap<Reverse<(i32, i32, PlotIdx)>> = BinaryHeap::new();
        labels.insert(start, Label { cost: 0, len: 1, from: None, done: false });
        heap.push(Reverse((0, 1, start)));
        // The best source found: its label and plot, and its source.
        let mut found: Option<(i32, i32, PlotIdx, SupplySource)> = None;
        while let Some(Reverse((cost, len, q))) = heap.pop() {
            if found.is_some_and(|(c, l, _, _)| (cost, len) > (c, l)) {
                break;
            }
            let label = labels[&q];
            if label.done || (label.cost, label.len) != (cost, len) {
                continue;
            }
            labels.get_mut(&q).expect("labelled").done = true;
            if q != start {
                if let Some(source) = self.supply_source_at(team, q, &ships) {
                    // Of the sources reached at this cost and length: the lowest path.
                    let better = match found {
                        None => true,
                        Some((_, _, other, _)) => path_of(&labels, q) < path_of(&labels, other),
                    };
                    if better {
                        found = Some((cost, len, q, source));
                    }
                    continue;
                }
            }
            let (x, y) = self.map.xy(q);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let Some(n) = self.plot_xy(x, y, dx, dy) else { continue };
                    if n == start {
                        continue;
                    }
                    let step = *costs.entry(n).or_insert_with(|| self.supply_cost(team, n, &forts));
                    let Some(step) = step else { continue };
                    let (c, l) = (cost + step, len + 1);
                    if c > budget {
                        continue;
                    }
                    let better = match labels.get(&n) {
                        None => true,
                        Some(old) if old.done => false,
                        Some(old) => match (c, l).cmp(&(old.cost, old.len)) {
                            Ordering::Less => true,
                            Ordering::Greater => false,
                            // The same cost and length: the lower path (both entered from plots
                            // whose paths are final).
                            Ordering::Equal => old.from.is_some_and(|f| path_of(&labels, q) < path_of(&labels, f)),
                        },
                    };
                    if better {
                        let new_key = labels.get(&n).is_none_or(|old| (old.cost, old.len) != (c, l));
                        labels.insert(n, Label { cost: c, len: l, from: Some(q), done: false });
                        if new_key {
                            heap.push(Reverse((c, l, n)));
                        }
                    }
                }
            }
        }
        let (cost, _, q, source) = found?;
        Some(SupplyLine { path: path_of(&labels, q), cost, source })
    }

    /// Army `a`'s supply line as it stands now (`None`: cut, or the army is not on the map).
    pub fn army_supply_line(&self, a: ArmyId) -> Option<SupplyLine> {
        let plot = self.army_plot(a)?;
        self.plot_supply_line(self.armies[a].owner, plot)
    }

    /// Army `a`'s supply state, from its counter (`Army::unsupplied_turns`).
    pub fn army_supply_state(&self, a: ArmyId) -> SupplyState {
        let n = self.armies[a].unsupplied_turns;
        let of = self.d.defines.supply_hunger_turns;
        if n <= 0 {
            SupplyState::Supplied
        } else if n <= of {
            SupplyState::Hunger { turn: n, of }
        } else {
            SupplyState::Starvation
        }
    }

    /// Whether army `a` is supplied: its counter is 0 (its recruits march, its units heal).
    pub fn army_in_supply(&self, a: ArmyId) -> bool {
        self.armies[a].unsupplied_turns == 0
    }

    /// Whether unit `u` is in an army whose line is cut (hungry or starving): it does not heal.
    pub fn unit_is_unsupplied(&self, u: UnitId) -> bool {
        self.units[u].army.is_some_and(|a| self.armies.contains(a) && self.armies[a].unsupplied_turns > 0)
    }

    /// Whether unit `u` is in a starving army: it deals `STARVATION_DAMAGE_DEALT_PERCENT` damage
    /// in battles.
    pub fn unit_is_starving(&self, u: UnitId) -> bool {
        self.units.contains(u) && self.units[u].army.is_some_and(|a| self.armies.contains(a) && self.army_supply_state(a) == SupplyState::Starvation)
    }

    /// The start of `p`'s turn, before its units heal (docs/SUPPLY.md, section 3): each army's
    /// line is traced (supplied: the counter to 0; cut: +1), with a message when an army becomes
    /// hungry, starts starving or is supplied again; a starving army's units lose
    /// `STARVATION_DAMAGE_PERCENT` of their full HP, never below `STARVATION_FLOOR_PERCENT`.
    pub fn player_update_supply(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        let hunger = self.d.defines.supply_hunger_turns;
        let turn = self.game_turn();
        for a in self.players[p].armies.clone() {
            if !self.armies.contains(a) {
                continue;
            }
            let Some(plot) = self.army_plot(a) else { continue };
            let before = self.armies[a].unsupplied_turns;
            let after = if self.plot_supply_line(p, plot).is_some() { 0 } else { before + 1 };
            self.armies[a].unsupplied_turns = after;
            let at = self.map.xy(plot);
            let name = self.armies[a].name.clone();
            if before > 0 && after == 0 {
                self.log.msg_at(turn, Some(p), at, || format!("The {name} is supplied again."));
            } else if before == 0 && after > 0 {
                self.log.msg_at(turn, Some(p), at, || format!("The {name} has no supply line: Hunger, turn 1 of {hunger}. No healing, no recruits."));
            } else if after == hunger + 1 {
                self.log.msg_at(turn, Some(p), at, || format!("The {name} is starving: units lose HP each turn, deal less damage."));
            }
            if after > hunger {
                self.army_starve(a);
            }
        }
    }

    /// Starvation's HP loss on army `a`'s units: `STARVATION_DAMAGE_PERCENT` of their full HP,
    /// never below `STARVATION_FLOOR_PERCENT` of it (a unit below it loses nothing). Never kills.
    fn army_starve(&mut self, a: ArmyId) {
        let d = &self.d.defines;
        let (loss_pct, floor_pct) = (d.starvation_damage_percent.max(0), d.starvation_floor_percent.clamp(1, 100));
        for u in self.army_units(a) {
            let max = self.unit_max_hit_points(u);
            let limit = max - (max * floor_pct + 99) / 100;
            let damage = self.units[u].damage;
            let loss = (max * loss_pct / 100).min(limit - damage);
            if loss > 0 {
                self.unit_change_damage(u, loss, None);
            }
        }
    }
}
