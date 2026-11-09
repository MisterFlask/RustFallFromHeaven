//! City works (docs/WORKS.md): this game's own rules, not FFH2's. There are no worker units.
//! Each city has work crews that improve its land, lay its roads and launch its fishing boats:
//! every turn the city adds its work rate ([`Game::city_crew_rate`]) to one build on one plot
//! ([`City::crew_build`](crate::city::City)), which it keeps until the build is done or cannot go on.
//! What it works on next is its owner's controller's choice ([`Controller::choose_crew_build`],
//! default [`Game::city_choose_crew_build_default`]).
//!
//! Where a city works ([`Game::city_crews_can_build`]): the plots of its owner that it is the work city
//! of ([`Game::plot_crew_city`]: the city that works the plot, else the owner's nearest city), and
//! for a road the unowned land next to its owner's land or to a road, within
//! `CITY_WORK_ROAD_RANGE`: a road between two cities advances plot by plot from each end. The
//! crews stay off a plot an enemy unit stands on, and outside the borders off a plot next to one.
//!
//! Roads, forts and the cards' field works are also built by army units where they stand
//! (`BuildInfo::field`, `FIELD_WORK_RATE`; [`Game::unit_can_build`]).

use crate::defs::*;
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// How far from a city its crews look for a plot of their owner that no city works (a resource
/// inside the borders, outside every city's plots).
const OUTLYING_RANGE: i32 = 8;

impl Game {
    /// The build progress city `c`'s crews add each turn: `CITY_WORK_RATE`, more for each unit of
    /// its owner in the city that helps them (`UnitInfo::crew_work`: a Slave), with its owner's work
    /// rate modifiers (civics, techs, buildings, traits; its civilization's own), the AI
    /// handicap's as for FFH2's workers; 0 in disorder.
    pub fn city_crew_rate(&self, c: CityId) -> i32 {
        let city = &self.cities[c];
        if city.occupation_timer > 0 {
            return 0;
        }
        let owner = city.owner;
        let civ = self.player_get_civilization_type(owner).map_or(0, |civ| self.d.civilizations[civ].work_rate_modifier);
        let plot = self.city_plot(c);
        let helpers: i32 = self.map.plots[plot].units.iter().filter(|&&u| self.units[u].owner == owner).map(|&u| self.d.units[self.units[u].unit_type].crew_work).sum();
        let mut rate = self.d.defines.city_work_rate + helpers;
        rate *= std::cmp::max(0, self.players[owner].worker_speed_modifier + civ + 100);
        rate /= 100;
        if !self.players[owner].human && !self.player_is_barbarian(owner) {
            rate *= std::cmp::max(0, self.d.handicaps[self.handicap].ai_work_rate_modifier + 100);
            rate /= 100;
        }
        rate
    }

    /// The city of `p` whose crews work `plot`, a plot of `p`: the city that works the plot,
    /// else `p`'s nearest city (the lower id of two as near).
    pub fn plot_crew_city(&self, plot: PlotIdx, p: PlayerId) -> Option<CityId> {
        if let Some(c) = self.map.plots[plot].working_city.filter(|&c| self.cities[c].owner == p) {
            return Some(c);
        }
        let (x, y) = self.map.xy(plot);
        self.players[p].cities.iter().copied().min_by_key(|&c| (self.plot_distance(x, y, self.cities[c].x, self.cities[c].y), c))
    }

    /// Whether a unit at war with `p` that `p` sees stands on `plot` (`near`: or next to it).
    fn plot_crews_threatened(&self, plot: PlotIdx, p: PlayerId, near: bool) -> bool {
        if self.plot_is_visible_enemy_unit(plot, p) {
            return true;
        }
        if !near {
            return false;
        }
        let (x, y) = self.map.xy(plot);
        (0..DirectionType::NUM).filter_map(|dir| self.plot_direction(x, y, DirectionType::from_usize(dir))).any(|q| self.plot_is_visible_enemy_unit(q, p))
    }

    /// Whether city `c`'s crews can work on `build` at `plot` now (the module docs).
    pub fn city_crews_can_build(&self, c: CityId, plot: PlotIdx, build: BuildType) -> bool {
        let info = &self.d.builds[build];
        if !info.city {
            return false;
        }
        let city = &self.cities[c];
        let owner = city.owner;
        match self.map.plots[plot].owner {
            Some(o) if o == owner => {
                if self.plot_crew_city(plot, owner) != Some(c) || self.plot_crews_threatened(plot, owner, false) {
                    return false;
                }
            }
            Some(_) => return false,
            None => {
                // A road beyond the borders: from our land or from a road, plot by plot.
                if info.route.is_none() || info.improvement.is_some() || self.plot_is_water(plot) {
                    return false;
                }
                let (x, y) = self.map.xy(plot);
                if self.plot_distance(x, y, city.x, city.y) > self.d.defines.city_work_road_range {
                    return false;
                }
                let linked = (0..DirectionType::NUM)
                    .filter_map(|dir| self.plot_direction(x, y, DirectionType::from_usize(dir)))
                    .any(|q| self.map.plots[q].owner == Some(owner) || self.map.plots[q].route_type.is_some());
                if !linked || self.plot_crews_threatened(plot, owner, true) {
                    return false;
                }
            }
        }
        self.player_can_build(owner, plot, build, false, false)
    }

    /// Sets what city `c`'s crews work on (`None`: they choose again in the city's turn); false,
    /// changing nothing, when they cannot work on it ([`Game::city_crews_can_build`]).
    pub fn city_set_crew_build(&mut self, c: CityId, job: Option<(PlotIdx, BuildType)>) -> bool {
        if let Some((plot, build)) = job {
            if !self.city_crews_can_build(c, plot, build) {
                return false;
            }
        }
        self.cities[c].crew_build = job;
        true
    }

    /// The turns city `c`'s crews need to finish `build` at `plot` at their rate now (`None` at
    /// rate 0).
    pub fn city_crew_turns_left(&self, c: CityId, plot: PlotIdx, build: BuildType) -> Option<i32> {
        let rate = self.city_crew_rate(c);
        if rate <= 0 {
            return None;
        }
        let left = self.plot_get_build_time(plot, build) - self.map.plots[plot].build_progress[build];
        Some(std::cmp::max(1, (left + rate - 1) / rate))
    }

    /// City `c`'s turn of works: its crews go on with their build, or with the one its owner's
    /// controller chooses when they have none they can work on. The barbarians' cities have no
    /// crews (their cities had no workers).
    pub fn city_do_crews(&mut self, c: CityId) {
        let owner = self.cities[c].owner;
        if self.player_is_barbarian(owner) {
            return;
        }
        let rate = self.city_crew_rate(c);
        if rate <= 0 {
            return;
        }
        let mut job = self.cities[c].crew_build.filter(|&(plot, build)| self.city_crews_can_build(c, plot, build));
        if job.is_none() {
            let controller = self.controller(owner);
            job = controller.choose_crew_build(self, c).filter(|&(plot, build)| self.city_crews_can_build(c, plot, build));
        }
        self.cities[c].crew_build = job;
        let Some((plot, build)) = job else { return };

        let cost = self.player_get_build_cost(owner, plot, build);
        self.players[owner].gold -= cost;
        let team = self.players[owner].team;
        if self.plot_change_build_progress(plot, build, rate, team) {
            if self.telemetry.on() {
                self.tel_crew_build(owner, build);
            }
            if self.d.builds[build].improvement.is_some_and(|i| self.d.improvements[i].claim_range > 0) {
                self.plot_claim(plot, owner);
            }
            if self.cities.contains(c) {
                self.cities[c].crew_build = None;
            }
        }
    }

    /// The plots city `c`'s crews may have work on: its own plots that it is the work city of,
    /// its owner's resource plots near it that no city works, and the plots of its owner's road
    /// plan (`roads`, [`Game::player_road_plan`]) in its reach.
    pub fn city_crew_plots(&self, c: CityId, roads: &[(PlotIdx, i32)]) -> Vec<PlotIdx> {
        let city = &self.cities[c];
        let owner = city.owner;
        let mut out: Vec<PlotIdx> = Vec::new();
        for i in 0..self.city_num_city_plots(c).max(0) as usize {
            if i == CITY_HOME_PLOT {
                continue;
            }
            let Some(q) = self.city_index_plot(c, i) else { continue };
            if self.map.plots[q].owner == Some(owner) && self.plot_crew_city(q, owner) == Some(c) {
                out.push(q);
            }
        }
        for dx in -OUTLYING_RANGE..=OUTLYING_RANGE {
            for dy in -OUTLYING_RANGE..=OUTLYING_RANGE {
                let Some(q) = self.plot_xy(city.x, city.y, dx, dy) else { continue };
                let plot = &self.map.plots[q];
                if plot.owner == Some(owner) && plot.bonus_type.is_some() && plot.plot_city.is_none() && !out.contains(&q) && self.plot_crew_city(q, owner) == Some(c) {
                    out.push(q);
                }
            }
        }
        let range = self.d.defines.city_work_road_range;
        for &(q, _) in roads {
            let (x, y) = self.map.xy(q);
            if !out.contains(&q) && self.plot_distance(x, y, city.x, city.y) <= range {
                out.push(q);
            }
        }
        out
    }

    /// The roads worth building for player `p` now, (plot, value): the next plot without a road
    /// on the way between two of its cities that no route links, at most `CITY_WORK_ROAD_RANGE`
    /// apart, from each end (through its land and unowned land); and the plots of its improved
    /// resources that no route links to their city.
    pub fn player_road_plan(&self, p: PlayerId) -> Vec<(PlotIdx, i32)> {
        let d = self.d;
        let team = self.players[p].team;
        let can_road = d.builds.iter().any(|b| b.city && b.route.is_some() && b.improvement.is_none() && b.tech_prereq.map_or(true, |t| self.team_is_has_tech(team, Some(t))));
        if !can_road {
            return Vec::new();
        }
        let range = d.defines.city_work_road_range;
        let cities = &self.players[p].cities;
        let mut out: Vec<(PlotIdx, i32)> = Vec::new();
        let add = |out: &mut Vec<(PlotIdx, i32)>, q: PlotIdx, v: i32| {
            if let Some(e) = out.iter_mut().find(|(o, _)| *o == q) {
                e.1 = e.1.max(v);
            } else {
                out.push((q, v));
            }
        };
        // Greedy step from `from` toward `to` along land we may build on: the first plot without a
        // route.
        let next_road = |from: PlotIdx, to: PlotIdx| -> Option<PlotIdx> {
            let (tx, ty) = self.map.xy(to);
            let mut cur = from;
            for _ in 0..(range + 4) {
                let (x, y) = self.map.xy(cur);
                let mut best: Option<(PlotIdx, i32)> = None;
                for dir in 0..DirectionType::NUM {
                    let Some(q) = self.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
                    if self.plot_is_water(q) || self.plot_is_impassable(q) {
                        continue;
                    }
                    let owner = self.map.plots[q].owner;
                    if owner.is_some_and(|o| o != p) {
                        continue;
                    }
                    let (qx, qy) = self.map.xy(q);
                    let dist = self.plot_distance(qx, qy, tx, ty) * 10 - i32::from(self.map.plots[q].route_type.is_some()) * 3;
                    if best.map_or(true, |(bq, bd)| dist < bd || dist == bd && q < bq) {
                        best = Some((q, dist));
                    }
                }
                let (q, _) = best?;
                if q == to {
                    return None;
                }
                if self.map.plots[q].route_type.is_none() {
                    return Some(q);
                }
                cur = q;
            }
            None
        };
        for (i, &a) in cities.iter().enumerate() {
            let pa = self.city_plot(a);
            for &b in cities.iter().skip(i + 1) {
                let pb = self.city_plot(b);
                if self.map.plots[pa].area != self.map.plots[pb].area {
                    continue;
                }
                let (ax, ay) = self.map.xy(pa);
                let (bx, by) = self.map.xy(pb);
                if self.plot_distance(ax, ay, bx, by) > range || self.city_is_connected_to(a, b) {
                    continue;
                }
                let v = 60 + 8 * (self.cities[a].population + self.cities[b].population).min(20);
                if let Some(q) = next_road(pa, pb) {
                    add(&mut out, q, v);
                }
                if let Some(q) = next_road(pb, pa) {
                    add(&mut out, q, v);
                }
            }
            // Improved resources not yet linked to this city.
            for k in 0..NUM_CITY_PLOTS {
                if k == CITY_HOME_PLOT {
                    continue;
                }
                let Some(q) = self.city_index_plot(a, k) else { continue };
                let plot = &self.map.plots[q];
                if plot.owner != Some(p) || plot.working_city != Some(a) {
                    continue;
                }
                let (Some(bonus), Some(imp)) = (plot.bonus_type, plot.improvement_type) else { continue };
                if !d.improvements[imp].improvement_bonus[bonus].bonus_trade || self.plot_is_connected_to(q, a) {
                    continue;
                }
                if plot.route_type.is_none() && !self.plot_is_water(q) {
                    add(&mut out, q, 90);
                } else if let Some(r) = next_road(q, pa) {
                    add(&mut out, r, 90);
                }
            }
        }
        out
    }

    /// What city `c`'s crews work on next when its owner's controller has no view of its own
    /// ([`Controller::choose_crew_build`]'s default): the build worth the most for its time, where
    /// an improvement on a plot without one counts the yields it adds (twice on a plot being
    /// worked), more when it connects a resource, and a road counts its value in
    /// [`Game::player_road_plan`]. It clears no feature for its production alone and replaces no
    /// improvement.
    pub fn city_choose_crew_build_default(&self, c: CityId) -> Option<(PlotIdx, BuildType)> {
        let d = self.d;
        let owner = self.cities[c].owner;
        let team = self.players[owner].team;
        let rate = self.city_crew_rate(c).max(1);
        let roads = self.player_road_plan(owner);
        let builds: Vec<BuildType> = d.builds.keys().filter(|&b| d.builds[b].city && (d.builds[b].improvement.is_some() || d.builds[b].route.is_some())).collect();
        let mut best: Option<(i64, PlotIdx, BuildType)> = None;
        for q in self.city_crew_plots(c, &roads) {
            let plot = &self.map.plots[q];
            for &b in &builds {
                let info = &d.builds[b];
                let value = if let Some(imp) = info.improvement {
                    if plot.improvement_type.is_some() {
                        continue;
                    }
                    let yields: i32 = (0..YieldType::NUM)
                        .map(|y| self.plot_calculate_improvement_yield_change(q, imp, YieldType::from_usize(y), Some(owner), false, false))
                        .sum();
                    let connects = self.plot_get_bonus_type(q, Some(team)).is_some_and(|bn| d.improvements[imp].improvement_bonus[bn].bonus_trade);
                    let mut value = 10 * yields + if connects { 100 } else { 0 };
                    if self.plot_is_being_worked(q) {
                        value *= 2;
                    }
                    value
                } else {
                    if plot.route_type.is_some() {
                        continue;
                    }
                    roads.iter().find(|(r, _)| *r == q).map_or(0, |&(_, v)| v)
                };
                if value <= 0 || !self.city_crews_can_build(c, q, b) {
                    continue;
                }
                let turns = (self.plot_get_build_time(q, b) - plot.build_progress[b]).max(1) / rate;
                let score = value as i64 * 100 / (turns as i64 + 3);
                if best.map_or(true, |(s, ..)| score > s) {
                    best = Some((score, q, b));
                }
            }
        }
        best.map(|(_, q, b)| (q, b))
    }
}
