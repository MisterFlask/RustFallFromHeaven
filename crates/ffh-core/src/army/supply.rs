//! Supply lines (this game's own rules, not FFH2's; docs/SUPPLY.md).
//!
//! **The line.** Every army needs a supply line: the cheapest path over land from its plot to a
//! source of its owner ([`Game::plot_supply_line`]): a city of its owner's team not in
//! occupation, or a ship of its owner with room for land units standing in a sea not closed to
//! its team with an open lane to a port of the team (docs/NAVAL.md, 4.6; the line then ends on a
//! land plot next to the ship). Entering
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
use std::collections::{BTreeMap, BinaryHeap};

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::naval::SeaLane;

/// Where a supply line ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SupplySource {
    /// A city of the army's owner's team.
    City(CityId),
    /// A ship of the army's owner; the line ends on a land plot next to it (or on the ship's
    /// plot itself for an army aboard it).
    Ship(UnitId),
    /// An army in boats on water in its team's range (docs/NAVAL.md, 3.3): the line is its own
    /// plot; the city is the nearest port by the range field (`Game::plot_sea_port`).
    Sea(CityId),
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

/// Why a supply ship is no source by the sea ([`Game::supply_ship_refused`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyShipRefusal {
    /// The sea it stands in is closed to its team: the sea and its holder.
    SeaClosed(SeaId, TeamId),
    /// Its lane is cut: the sea and its holder.
    LaneCut(SeaId, TeamId),
    /// No lane: no path over water in range to a port of its team.
    NoLane,
}

/// What a supply-line search knows: the truth (the rules' line, [`Game::plot_supply_line`]), or
/// what a team knows ([`Game::plot_supply_line_known`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SupplyView {
    Truth,
    Known(TeamId),
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

    /// Whether `view` sees unit `u` on `plot`: always for the truth; for a team, its plot is
    /// visible to the team ([`Game::plot_is_visible`]) and the unit not invisible to it
    /// ([`Game::unit_is_invisible`]).
    fn supply_sees_unit(&self, view: SupplyView, u: UnitId, plot: PlotIdx) -> bool {
        match view {
            SupplyView::Truth => true,
            SupplyView::Known(t) => self.plot_is_visible(plot, t) && !self.unit_is_invisible(u, t, true),
        }
    }

    /// Whether `view` knows of city `c`: always for the truth; for a team, it has revealed it
    /// ([`Game::city_is_revealed`]).
    fn supply_knows_city(&self, view: SupplyView, c: CityId) -> bool {
        match view {
            SupplyView::Truth => true,
            SupplyView::Known(t) => self.city_is_revealed(c, t, false),
        }
    }

    /// Whether a unit at war with `team` that `view` sees stands on `plot` (the truth: any).
    fn supply_enemy_on(&self, team: TeamId, plot: PlotIdx, view: SupplyView) -> bool {
        self.map.plots[plot].units.iter().any(|&u| self.units.contains(u) && self.unit_is_enemy(u, Some(team), Some(plot)) && self.supply_sees_unit(view, u, plot))
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
        self.supply_cost(team, plot, &self.supply_fort_types(), SupplyView::Truth)
    }

    /// [`Game::plot_supply_cost`] as `view` knows it: for a team, a plot it has not revealed
    /// cannot be entered, and units, cities and manned forts count only when it sees them.
    fn supply_cost(&self, team: TeamId, plot: PlotIdx, forts: &[ImprovementType], view: SupplyView) -> Option<i32> {
        if let SupplyView::Known(t) = view {
            if !self.map.plots[plot].revealed[t] {
                return None;
            }
        }
        if self.plot_is_water(plot) || self.plot_is_impassable(plot) {
            return None;
        }
        if self.supply_enemy_on(team, plot, view) {
            return None;
        }
        let plot_team = self.plot_get_team(plot);
        if let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.supply_knows_city(view, c)) {
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
                    if self.map.plots[q].improvement_type.is_some_and(|i| forts.contains(&i)) && self.supply_enemy_on(team, q, view) {
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
    /// (`cargo_capacity > 0`, land cargo, no special cargo) on a water plot whose sea is not
    /// closed to `p`'s team, with an open lane to a port of the team (docs/NAVAL.md, 4.6 and
    /// section 5: [`Game::sea_lane`]). The lowest unit id per plot. For a team's `view`: only
    /// ships it sees, and the control it can work out ([`Game::sea_closed_known`],
    /// [`Game::sea_lane_known`]).
    fn supply_ships(&self, p: PlayerId, view: SupplyView) -> BTreeMap<PlotIdx, UnitId> {
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
        for u in candidates {
            let Some(plot) = self.unit_plot_opt(u) else { continue };
            if !self.supply_sees_unit(view, u, plot) {
                continue;
            }
            if self.plot_is_water(plot) && self.supply_ship_open(team, plot, view) {
                let e = ships.entry(plot).or_insert(u);
                if u < *e {
                    *e = u;
                }
            }
        }
        ships
    }

    /// Whether a supply ship of `team` on water plot `plot` is a source by the sea (docs/NAVAL.md,
    /// 4.6): its sea not closed to the team and its lane to a port of the team open, as `view`
    /// knows them.
    fn supply_ship_open(&self, team: TeamId, plot: PlotIdx, view: SupplyView) -> bool {
        match view {
            SupplyView::Truth => !self.plot_sea_closed(plot, team) && self.sea_lane(team, plot).is_some_and(|l| !l.is_cut()),
            SupplyView::Known(v) => {
                self.map.plots[plot].sea.is_none_or(|s| self.sea_closed_known(s, team, v).is_none()) && self.sea_lane_known(team, plot, v).is_some_and(|l| !l.is_cut())
            }
        }
    }

    /// Why a supply ship of `team` on `plot` is no source by the sea, as the truth has it
    /// (docs/NAVAL.md, 4.6; the client's "No supply line: the transport is in X, held by Y"): the
    /// sea it stands in closed (with its holder), else its lane cut (where, by whom), else no lane
    /// in range. `None`: it is a source by the sea.
    pub fn supply_ship_refused(&self, team: TeamId, plot: PlotIdx) -> Option<SupplyShipRefusal> {
        if let Some(s) = self.map.plots[plot].sea.filter(|&s| self.sea_is_closed_to(s, team)) {
            return Some(SupplyShipRefusal::SeaClosed(s, self.sea_closed_to(s, team).unwrap_or_else(|| self.barbarian_team())));
        }
        match self.sea_lane(team, plot) {
            None => Some(SupplyShipRefusal::NoLane),
            Some(SeaLane { cut_in: Some((s, t)), .. }) => Some(SupplyShipRefusal::LaneCut(s, t)),
            Some(_) => None,
        }
    }

    /// The source a supply line of `p`'s ends at on `plot` (`team`'s, `ships` from
    /// [`Game::supply_ships`]): a city of the team not in occupation, else a land plot next to a
    /// ship (the lowest unit id). For a team's `view`: only a city it has revealed.
    fn supply_source_at(&self, team: TeamId, plot: PlotIdx, ships: &BTreeMap<PlotIdx, UnitId>, view: SupplyView) -> Option<SupplySource> {
        if let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.supply_knows_city(view, c)) {
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
    /// deterministic; nothing is cached. It bends around every enemy unit, seen or not.
    pub fn plot_supply_line(&self, p: PlayerId, start: PlotIdx) -> Option<SupplyLine> {
        self.supply_line_search(p, start, SupplyView::Truth)
    }

    /// The supply line of an army of `p` on `plot` as team `viewer` can work it out
    /// (docs/SUPPLY.md, "Second pass", A): [`Game::plot_supply_line`]'s search, costs, budget and
    /// tie rule with the viewer's knowledge in place of the truth. It enters only plots the viewer
    /// has revealed (`Plot::revealed`); a unit blocks only if the viewer sees it now (its plot
    /// visible to the viewer, [`Game::plot_is_visible`], and the unit not invisible to it,
    /// [`Game::unit_is_invisible`]); a fort denies its neighbours only if the viewer sees a unit
    /// in it; a source is a city the viewer has revealed ([`Game::city_is_revealed`]) or a ship
    /// it sees. Who is an enemy, the plots' owners, routes and improvements, and a city's
    /// occupation are the truth. `viewer` may be `p`'s own team. Pure and deterministic.
    pub fn plot_supply_line_known(&self, p: PlayerId, start: PlotIdx, viewer: TeamId) -> Option<SupplyLine> {
        self.supply_line_search(p, start, SupplyView::Known(viewer))
    }

    /// The search of [`Game::plot_supply_line`] and [`Game::plot_supply_line_known`].
    fn supply_line_search(&self, p: PlayerId, start: PlotIdx, view: SupplyView) -> Option<SupplyLine> {
        let team = self.players[p].team;
        // In a city of its owner's team, whatever its state.
        if let Some(c) = self.map.plots[start].plot_city.filter(|&c| self.cities.contains(c) && self.supply_knows_city(view, c)) {
            if self.supply_home(team, self.players[self.cities[c].owner].team) {
                return Some(SupplyLine { path: vec![start], cost: 0, source: SupplySource::City(c) });
            }
        }
        let ships = self.supply_ships(p, view);
        // Aboard (on the plot of) one of its source ships.
        if let Some(&s) = ships.get(&start) {
            return Some(SupplyLine { path: vec![start], cost: 0, source: SupplySource::Ship(s) });
        }
        // At sea, in boats (docs/NAVAL.md, 3.3): supplied while its plot is in range, from the
        // nearest port; cut out of range. (Both views read the range field as it is: the ports
        // are cities of the army's own side.)
        if self.supply_at_sea(team, start) {
            if self.team_sea_exempt(team) {
                return None;
            }
            let in_range = self.plot_sea_range(team, start).is_some_and(|d| d <= self.team_sea_range(team));
            return match self.plot_sea_port(team, start) {
                Some(c) if in_range => Some(SupplyLine { path: vec![start], cost: 0, source: SupplySource::Sea(c) }),
                _ => None,
            };
        }
        if let Some(source) = self.supply_source_at(team, start, &ships, view) {
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
                if let Some(source) = self.supply_source_at(team, q, &ships, view) {
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
                    let step = *costs.entry(n).or_insert_with(|| self.supply_cost(team, n, &forts, view));
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

    /// Whether a supply line of `team`'s from `start` is at sea (docs/NAVAL.md, 3.3): a water
    /// plot that is not walked on as land (a Causeway, a strait for a Ferries holder).
    fn supply_at_sea(&self, team: TeamId, start: PlotIdx) -> bool {
        self.plot_is_water(start) && !self.plot_is_land_bridge(start) && !(self.team_strait_crossing(team) && self.plot_is_strait(start))
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
        // A player with no city (the starting units before the capital, a crusade Host without
        // lenders' cities) has no supply rule at all: its armies' counters stay 0.
        let team = self.players[p].team;
        let has_city = self.player_ids().any(|q| self.players[q].alive && !self.players[q].cities.is_empty() && self.supply_home(team, self.players[q].team));
        if !has_city {
            for a in self.players[p].armies.clone() {
                if self.armies.contains(a) {
                    self.armies[a].unsupplied_turns = 0;
                }
            }
            return;
        }
        let hunger = self.d.defines.supply_hunger_turns;
        let turn = self.game_turn();
        for a in self.players[p].armies.clone() {
            if !self.armies.contains(a) {
                continue;
            }
            // A fleet has no supply line (docs/NAVAL.md, 7.1: range and wear instead).
            if self.armies[a].domain == super::ArmyDomain::Sea {
                self.armies[a].unsupplied_turns = 0;
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
