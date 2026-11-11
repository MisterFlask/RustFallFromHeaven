//! The sea, phase 2 (this game's own rules, not FFH2's; docs/NAVAL.md, section 4): named seas and
//! who holds them.
//!
//! **Seas** ([`Game::map_generate_seas`], at the start of the starting items, after the map's
//! water is final): every water area of at least `SEA_MIN_PLOTS` plots is cut into
//! `max(1, round(plots / SEA_TARGET_PLOTS))` seas, through the map RNG: the first seed a plot of
//! the area drawn at random, each next the plot farthest (water steps, 8 directions, inside the
//! area) from the seeds so far (ties: the lower plot index); each plot goes to the seed nearest
//! over water (ties: the lower sea index). Names are drawn without repeats from
//! `sea_names.jsonc` (the `lake` lists for an area that is one sea and touches no map edge), then
//! "Sea N". `Plot::sea` and `Map::seas` are saved. A plot that becomes water later joins the sea of
//! a neighbouring water plot (the lowest sea index), or none; one that becomes land leaves its sea.
//!
//! **Patrol** (4.2): Civ4's plunder mission and blockading state (`Unit::blockading`), renamed:
//! a player's warship on a water plot of a sea, outside a city ([`Game::unit_can_plunder`]).
//!
//! **Control** (4.3): a team's strength in a sea is the strength of its warships on Patrol there
//! ([`Game::unit_sea_strength_x100`]: as the duel counts it, damage and wear included), summed and
//! rounded down. A sea is closed to team A while a team E at war with A (a ship of hidden
//! nationality: at war with every other team) has strength above 0 there and at least
//! `SEA_HOLD_PERCENT` of A's ([`Game::sea_closed_to`]). [`Game::sea_state`] is the summary a team
//! is shown, [`Game::sea_state_known`] the same from the ships a viewer sees.
//!
//! **The stored blockade.** Control is a pure function of the map; for speed each sea stores the
//! teams it is closed to (`Sea::closed_to`) and each of its plots the blockade
//! (`Plot::blockaded_count`, 1 for a team the plot's sea is closed to, else 0), which the trade
//! network and the citizens read. [`Game::sea_refresh`] recomputes a sea and, on a change, the
//! plots and the trade network: when a ship starts or ends Patrol, is damaged or promoted, dies,
//! changes owner or nationality, when war or peace is made (all through `unit_update_plunder`),
//! at each player's turn start for the seas it patrols (wear) and for every sea at the start of
//! each game turn. [`Game::sea_control_stale`] checks it (the stress harness).

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// A sea (docs/NAVAL.md, 4.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sea {
    pub name: String,
    /// Its plots, in index order.
    pub plots: Vec<PlotIdx>,
    /// The teams the sea is closed to (stored for speed, module comment), in team order.
    pub closed_to: Vec<TeamId>,
}

/// A party on Patrol in a sea as a viewer can tell it: a team, or ships of hidden nationality
/// whose team the viewer cannot tell (`None`).
pub type SeaParty = Option<TeamId>;

/// Who holds a sea, for a team (docs/NAVAL.md, 4.3 and 10).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SeaControl {
    /// It holds the sea against every enemy that patrols there, and has strength.
    HeldByUs,
    /// Closed to it: held by that party.
    HeldBy(SeaParty),
    /// It and an enemy (the strongest) both have strength; neither holds.
    Contested(SeaParty),
    /// No enemy patrols, it does not, and no other team does.
    Open,
    /// Patrolled only by a team it is not at war with (the strongest).
    PatrolledBy(SeaParty),
}

/// A sea's state for a team: who holds it and the strengths (`ours`: the team's; `theirs`: the
/// party the control names, 0 for `Open` and for `HeldByUs` with no enemy).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SeaState {
    pub control: SeaControl,
    pub ours: i32,
    pub theirs: i32,
}

impl SeaState {
    /// Whether the sea is closed to the team.
    pub fn closed(&self) -> bool {
        matches!(self.control, SeaControl::HeldBy(_))
    }
}

/// A coastal city's harbour (docs/NAVAL.md, 4.4 point 1; [`Game::city_harbour`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Harbour {
    /// The city touches no sea.
    None,
    /// Some sea it touches is not closed to its team.
    Open,
    /// Every sea it touches is closed to its team: each sea with its holder.
    Blockaded(Vec<(SeaId, TeamId)>),
}

/// The ships of one party in a sea: total strength (x100) and the part of hidden nationality.
#[derive(Copy, Clone, Debug, Default)]
struct Force {
    all: i32,
    hidden: i32,
}

/// No sea yet / unreached, in the generator's arrays.
const FAR: u32 = u32::MAX;

impl Game {
    // ---------------------------------------------------------------------------------------
    // Seas (4.1)

    /// The sea of `plot`, if any.
    pub fn plot_sea(&self, plot: PlotIdx) -> Option<SeaId> {
        self.map.plots[plot].sea
    }

    /// The seas, by id.
    pub fn sea_ids(&self) -> impl Iterator<Item = SeaId> + 'static {
        (0..self.map.seas.len()).map(|i| SeaId(i as u32))
    }

    /// Cuts the map's water into seas (module comment), replacing any there were. Draws from the
    /// map RNG.
    pub fn map_generate_seas(&mut self) {
        let d = self.d;
        let n = self.map.num_plots();
        for q in self.map.plot_ids() {
            self.map.plots[q].sea = None;
        }
        self.map.seas.clear();
        let min = d.defines.sea_min_plots.max(1) as usize;
        let target = d.defines.sea_target_plots.max(1) as usize;
        let (w, h) = (self.map.grid_width, self.map.grid_height);
        let (wrap_x, wrap_y) = (self.map.wrap_x, self.map.wrap_y);
        // The names, in data order: the seas' and the lakes'.
        let mut sea_names: Vec<String> = Vec::new();
        let mut lake_names: Vec<String> = Vec::new();
        for info in d.sea_names.iter() {
            if info.lake {
                lake_names.extend(info.names.iter().cloned());
            } else {
                sea_names.extend(info.names.iter().cloned());
            }
        }
        // The water areas, in id order, with their plots in index order.
        let mut areas: Vec<(AreaId, Vec<PlotIdx>)> = self.map.areas.iter().filter(|(_, a)| a.water).map(|(id, _)| (id, Vec::new())).collect();
        for q in self.map.plot_ids() {
            if !self.plot_is_water(q) {
                continue;
            }
            if let Some(a) = self.map.plots[q].area {
                if let Some(e) = areas.iter_mut().find(|(id, _)| *id == a) {
                    e.1.push(q);
                }
            }
        }
        let mut in_area = vec![false; n];
        let mut dist = vec![FAR; n];
        let mut label = vec![FAR; n];
        for (_, plots) in areas {
            if plots.len() < min {
                continue;
            }
            for &q in &plots {
                in_area[q.idx()] = true;
                dist[q.idx()] = FAR;
            }
            let count = ((plots.len() + target / 2) / target).max(1);
            // The seeds: the first at random, each next the farthest from those so far.
            let first = plots[self.map_rand_num(plots.len() as i32, "Seas") as usize];
            let mut seeds: Vec<PlotIdx> = Vec::with_capacity(count);
            let add_seed = |g: &Game, s: PlotIdx, dist: &mut Vec<u32>| {
                dist[s.idx()] = 0;
                let mut layer = vec![s];
                let mut k = 0u32;
                while !layer.is_empty() {
                    let mut next = Vec::new();
                    for &q in &layer {
                        let (x, y) = g.map.xy(q);
                        for i in 0..DirectionType::NUM {
                            let Some(o) = g.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                            if in_area[o.idx()] && dist[o.idx()] > k + 1 {
                                dist[o.idx()] = k + 1;
                                next.push(o);
                            }
                        }
                    }
                    layer = next;
                    k += 1;
                }
            };
            add_seed(self, first, &mut dist);
            seeds.push(first);
            while seeds.len() < count {
                let mut best: Option<PlotIdx> = None;
                for &q in &plots {
                    let dq = dist[q.idx()];
                    if dq == 0 || dq == FAR {
                        continue;
                    }
                    if best.is_none_or(|b| dq > dist[b.idx()]) {
                        best = Some(q);
                    }
                }
                let Some(s) = best else { break };
                add_seed(self, s, &mut dist);
                seeds.push(s);
            }
            // Each plot to its nearest seed, ties to the lower sea index.
            let base = self.map.seas.len() as u32;
            for &q in &plots {
                dist[q.idx()] = FAR;
            }
            let mut layer: Vec<PlotIdx> = Vec::new();
            for (k, &s) in seeds.iter().enumerate() {
                dist[s.idx()] = 0;
                label[s.idx()] = base + k as u32;
                layer.push(s);
            }
            layer.sort();
            let mut k = 0u32;
            while !layer.is_empty() {
                let mut next = Vec::new();
                for &q in &layer {
                    let (x, y) = self.map.xy(q);
                    let from = label[q.idx()];
                    for i in 0..DirectionType::NUM {
                        let Some(o) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                        if !in_area[o.idx()] {
                            continue;
                        }
                        if dist[o.idx()] == FAR {
                            dist[o.idx()] = k + 1;
                            label[o.idx()] = from;
                            next.push(o);
                        } else if dist[o.idx()] == k + 1 && from < label[o.idx()] {
                            label[o.idx()] = from;
                        }
                    }
                }
                next.sort();
                layer = next;
                k += 1;
            }
            // The seas, named.
            let edge = |q: PlotIdx| {
                let (x, y) = (q.idx() as i32 % w.max(1), q.idx() as i32 / w.max(1));
                (!wrap_x && (x == 0 || x == w - 1)) || (!wrap_y && (y == 0 || y == h - 1))
            };
            let lake = seeds.len() == 1 && !plots.iter().any(|&q| edge(q));
            for k in 0..seeds.len() {
                let index = self.map.seas.len();
                let name = {
                    let pool = if lake && !lake_names.is_empty() { &mut lake_names } else { &mut sea_names };
                    if pool.is_empty() {
                        format!("Sea {}", index + 1)
                    } else {
                        let i = self.map_rand_num(pool.len() as i32, "Sea names") as usize;
                        pool.remove(i)
                    }
                };
                let sea_plots: Vec<PlotIdx> = plots.iter().copied().filter(|q| label[q.idx()] == base + k as u32).collect();
                self.map.seas.push(Sea { name, plots: sea_plots, closed_to: Vec::new() });
            }
            for &q in &plots {
                in_area[q.idx()] = false;
                if label[q.idx()] != FAR {
                    self.map.plots[q].sea = Some(SeaId(label[q.idx()]));
                }
                label[q.idx()] = FAR;
            }
        }
        // No blockade yet (nobody patrols): every plot open.
        self.seas_refresh_all();
    }

    /// Adds a sea of `plots` (water plots with no sea yet), named `name`: for tests and tools that
    /// build a map by hand. Returns its id.
    pub fn map_cut_sea(&mut self, name: &str, plots: &[PlotIdx]) -> SeaId {
        let id = SeaId(self.map.seas.len() as u32);
        let mut plots: Vec<PlotIdx> = plots.iter().copied().filter(|&q| self.plot_is_water(q) && self.map.plots[q].sea.is_none()).collect();
        plots.sort();
        plots.dedup();
        for &q in &plots {
            self.map.plots[q].sea = Some(id);
        }
        self.map.seas.push(Sea { name: name.to_string(), plots, closed_to: Vec::new() });
        self.sea_refresh(id);
        id
    }

    /// `plot` changed between water and land (`plot_set_plot_type`): water joins the sea of a
    /// neighbouring water plot (the lowest sea index) or none; land leaves its sea.
    pub(crate) fn plot_update_sea_on_type_change(&mut self, p: PlotIdx) {
        if let Some(s) = self.map.plots[p].sea.take() {
            self.map.seas[s.idx()].plots.retain(|&q| q != p);
        }
        if self.plot_is_water(p) {
            let (x, y) = self.map.xy(p);
            let near = (0..DirectionType::NUM)
                .filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i)))
                .filter(|&o| self.plot_is_water(o))
                .filter_map(|o| self.map.plots[o].sea)
                .min();
            if let Some(s) = near {
                self.map.plots[p].sea = Some(s);
                let plots = &mut self.map.seas[s.idx()].plots;
                let at = plots.partition_point(|&q| q < p);
                plots.insert(at, p);
            }
        }
        // The plot's blockade follows its sea (none on land).
        let closed: Vec<TeamId> = self.map.plots[p].sea.map(|s| self.map.seas[s.idx()].closed_to.clone()).unwrap_or_default();
        let mut changed = false;
        for t in self.team_ids() {
            let v = closed.contains(&t) as i32;
            if self.map.plots[p].blockaded_count[t] != v {
                self.map.plots[p].blockaded_count[t] = v;
                changed = true;
            }
        }
        if changed {
            self.update_plot_groups();
        }
    }

    // ---------------------------------------------------------------------------------------
    // Strength and control (4.3)

    /// Unit `u`'s strength in a sea, x100: its strength as the duel counts it with no plot and no
    /// opponent (damage and wear included).
    pub fn unit_sea_strength_x100(&self, u: UnitId) -> i32 {
        self.unit_curr_combat_str(u, None, None, None).max(0)
    }

    /// The warships on Patrol in sea `s` (in plot order, then the plot's unit order).
    pub fn sea_patrollers(&self, s: SeaId) -> Vec<UnitId> {
        let mut out = Vec::new();
        for &q in &self.map.seas[s.idx()].plots {
            for &u in &self.map.plots[q].units {
                if self.units.contains(u) && self.units[u].blockading && self.unit_is_warship(u) && !self.unit_is_barbarian(u) {
                    out.push(u);
                }
            }
        }
        out
    }

    /// Whether `viewer` sees unit `u` (its own team's always).
    fn sea_sees(&self, viewer: TeamId, u: UnitId) -> bool {
        self.unit_get_team(u) == viewer || self.unit_plot_opt(u).is_some_and(|q| self.plot_is_visible(q, viewer) && !self.unit_is_invisible(u, viewer, false))
    }

    /// The forces on Patrol in sea `s` by team (x100), from `ships` (the truth: every patroller).
    fn sea_forces(&self, ships: &[UnitId]) -> Vec<(TeamId, Force)> {
        let mut out: Vec<(TeamId, Force)> = Vec::new();
        for &u in ships {
            let t = self.unit_get_team(u);
            let x = self.unit_sea_strength_x100(u);
            let hidden = self.unit_is_hidden_nationality(u);
            let i = match out.iter().position(|e| e.0 == t) {
                Some(i) => i,
                None => {
                    out.push((t, Force::default()));
                    out.len() - 1
                }
            };
            out[i].1.all += x;
            if hidden {
                out[i].1.hidden += x;
            }
        }
        out.sort_by_key(|e| e.0);
        out
    }

    /// `team`'s strength in sea `s` (docs/NAVAL.md, 4.3): its warships on Patrol there, rounded
    /// down. The truth.
    pub fn sea_strength(&self, s: SeaId, team: TeamId) -> i32 {
        self.sea_patrollers(s).into_iter().filter(|&u| self.unit_get_team(u) == team).map(|u| self.unit_sea_strength_x100(u)).sum::<i32>() / 100
    }

    /// The state from parties: `ours` (the team's strength), the hostile parties' strengths
    /// against it and the others' (each rounded down, x1).
    fn sea_state_from(&self, ours: i32, hostile: &[(SeaParty, i32)], friendly: &[(SeaParty, i32)]) -> SeaState {
        let hold = self.d.defines.sea_hold_percent.max(0) as i64;
        // The strongest first, ties to the lower team (hidden last).
        let key = |p: &SeaParty| p.map_or(u32::MAX, |t| t.0 as u32);
        let best = |v: &[(SeaParty, i32)], f: &dyn Fn(i32) -> bool| v.iter().filter(|e| e.1 > 0 && f(e.1)).min_by(|a, b| b.1.cmp(&a.1).then(key(&a.0).cmp(&key(&b.0)))).copied();
        if let Some((p, x)) = best(hostile, &|x| x as i64 * 100 >= hold * ours as i64) {
            return SeaState { control: SeaControl::HeldBy(p), ours, theirs: x };
        }
        if let Some((p, x)) = best(hostile, &|_| true) {
            let we_hold = ours > 0 && hostile.iter().all(|e| ours as i64 * 100 >= hold * e.1 as i64);
            let control = if we_hold { SeaControl::HeldByUs } else { SeaControl::Contested(p) };
            return SeaState { control, ours, theirs: x };
        }
        if ours > 0 {
            return SeaState { control: SeaControl::HeldByUs, ours, theirs: 0 };
        }
        match best(friendly, &|_| true) {
            Some((p, x)) => SeaState { control: SeaControl::PatrolledBy(p), ours, theirs: x },
            None => SeaState { control: SeaControl::Open, ours, theirs: 0 },
        }
    }

    /// The truth's state of sea `s` for `team`, with `extra` ships counted as on Patrol there
    /// (a hypothetical patrol; empty for the real state).
    fn sea_state_truth(&self, s: SeaId, team: TeamId, extra: &[UnitId]) -> SeaState {
        let mut ships = self.sea_patrollers(s);
        let more: Vec<UnitId> = extra.iter().copied().filter(|u| !ships.contains(u)).collect();
        ships.extend(more);
        let forces = self.sea_forces(&ships);
        let ours = forces.iter().find(|e| e.0 == team).map_or(0, |e| e.1.all) / 100;
        let mut hostile: Vec<(SeaParty, i32)> = Vec::new();
        let mut friendly: Vec<(SeaParty, i32)> = Vec::new();
        for &(t, f) in &forces {
            if t == team {
                continue;
            }
            if self.at_war(t, team) {
                hostile.push((Some(t), f.all / 100));
            } else {
                if f.hidden > 0 {
                    hostile.push((Some(t), f.hidden / 100));
                }
                if f.all - f.hidden > 0 {
                    friendly.push((Some(t), (f.all - f.hidden) / 100));
                }
            }
        }
        self.sea_state_from(ours, &hostile, &friendly)
    }

    /// The state of sea `s` for `team` (docs/NAVAL.md, 4.3; module comment): the truth.
    pub fn sea_state(&self, s: SeaId, team: TeamId) -> SeaState {
        self.sea_state_truth(s, team, &[])
    }

    /// The state of sea `s` for `team` as team `viewer` can work it out: only ships `viewer` sees
    /// count (its own always), and a ship of hidden nationality of another team than the viewer's
    /// is of an unknown party (`None`), hostile to `team`. `extra`: ships counted as on Patrol
    /// there (a hypothetical patrol).
    pub fn sea_state_known_if(&self, s: SeaId, team: TeamId, viewer: TeamId, extra: &[UnitId]) -> SeaState {
        let mut ships: Vec<UnitId> = self.sea_patrollers(s).into_iter().filter(|&u| self.sea_sees(viewer, u)).collect();
        let more: Vec<UnitId> = extra.iter().copied().filter(|u| !ships.contains(u)).collect();
        ships.extend(more);
        let mut ours = 0;
        let mut parties: Vec<(SeaParty, bool, i32)> = Vec::new();
        for u in ships {
            let t = self.unit_get_team(u);
            let x = self.unit_sea_strength_x100(u);
            if t == team && (t == viewer || !self.unit_is_hidden_nationality(u)) {
                ours += x;
                continue;
            }
            let (party, hostile) = if self.unit_is_hidden_nationality(u) && t != viewer { (None, true) } else { (Some(t), self.at_war(t, team) || self.unit_is_hidden_nationality(u)) };
            match parties.iter_mut().find(|e| e.0 == party && e.1 == hostile) {
                Some(e) => e.2 += x,
                None => parties.push((party, hostile, x)),
            }
        }
        let hostile: Vec<(SeaParty, i32)> = parties.iter().filter(|e| e.1).map(|e| (e.0, e.2 / 100)).collect();
        let friendly: Vec<(SeaParty, i32)> = parties.iter().filter(|e| !e.1).map(|e| (e.0, e.2 / 100)).collect();
        self.sea_state_from(ours / 100, &hostile, &friendly)
    }

    /// [`Game::sea_state_known_if`] with no hypothetical ship: what `viewer` can work out of sea
    /// `s` for its own team.
    pub fn sea_state_known(&self, s: SeaId, viewer: TeamId) -> SeaState {
        self.sea_state_known_if(s, viewer, viewer, &[])
    }

    /// The state sea `s` would have for `viewer`'s team, as the viewer can work it out, if
    /// `ships` (its own) were on Patrol there: the result of a patrol order (docs/NAVAL.md, 10:
    /// `38 to 17. Held by you.`).
    pub fn sea_state_if_patrol(&self, s: SeaId, viewer: TeamId, ships: &[UnitId]) -> SeaState {
        self.sea_state_known_if(s, viewer, viewer, ships)
    }

    /// The team holding sea `s` against `team` (the strongest party that closes it; a hidden
    /// party is reported as its team), `None` when it is not closed to `team`. The truth,
    /// computed (the stored blockade is [`Game::sea_is_closed_to`]).
    pub fn sea_closed_to(&self, s: SeaId, team: TeamId) -> Option<TeamId> {
        match self.sea_state(s, team).control {
            SeaControl::HeldBy(p) => p,
            _ => None,
        }
    }

    /// Whether sea `s` is closed to `team`, as stored (module comment): equal to
    /// [`Game::sea_closed_to`] between events.
    pub fn sea_is_closed_to(&self, s: SeaId, team: TeamId) -> bool {
        self.map.seas[s.idx()].closed_to.contains(&team)
    }

    /// Whether the sea of `plot` is closed to `team`, as stored; false off the seas.
    pub fn plot_sea_closed(&self, plot: PlotIdx, team: TeamId) -> bool {
        self.map.plots[plot].sea.is_some_and(|s| self.sea_is_closed_to(s, team))
    }

    /// Whether sea `s` is closed to `team` as `viewer` can work it out, and by which party.
    pub fn sea_closed_known(&self, s: SeaId, team: TeamId, viewer: TeamId) -> Option<SeaParty> {
        match self.sea_state_known_if(s, team, viewer, &[]).control {
            SeaControl::HeldBy(p) => Some(p),
            _ => None,
        }
    }

    // ---------------------------------------------------------------------------------------
    // The stored blockade (module comment)

    /// The teams sea `s` is closed to now (the truth), in team order.
    fn sea_closed_now(&self, s: SeaId) -> Vec<TeamId> {
        let ships = self.sea_patrollers(s);
        if ships.is_empty() {
            return Vec::new();
        }
        self.team_ids().filter(|&t| self.sea_state_truth(s, t, &[]).closed()).collect()
    }

    /// Recomputes sea `s`'s control and, when it changed, its plots' blockade, the trade network
    /// and the messages to the players with a port on it (module comment). True if it changed.
    pub fn sea_refresh(&mut self, s: SeaId) -> bool {
        if !self.sea_refresh_quiet(s) {
            return false;
        }
        self.update_plot_groups();
        true
    }

    /// [`Game::sea_refresh`] without the trade network's update.
    fn sea_refresh_quiet(&mut self, s: SeaId) -> bool {
        let now = self.sea_closed_now(s);
        if now == self.map.seas[s.idx()].closed_to {
            return false;
        }
        let before = std::mem::replace(&mut self.map.seas[s.idx()].closed_to, now.clone());
        for q in self.map.seas[s.idx()].plots.clone() {
            let mut dirty = false;
            for t in self.team_ids() {
                let v = now.contains(&t) as i32;
                if self.map.plots[q].blockaded_count[t] != v {
                    self.map.plots[q].blockaded_count[t] = v;
                    dirty = true;
                }
            }
            if dirty {
                if let Some(c) = self.map.plots[q].working_city.filter(|&c| self.cities.contains(c)) {
                    self.cities[c].ai_assign_work_dirty = true;
                }
            }
        }
        self.sea_messages(s, &before, &now);
        true
    }

    /// Every sea refreshed ([`Game::sea_refresh`]), the trade network once.
    pub fn seas_refresh_all(&mut self) {
        let mut changed = false;
        for s in self.sea_ids() {
            changed |= self.sea_refresh_quiet(s);
        }
        // Plots on no sea are never blockaded.
        for q in self.map.plot_ids() {
            if self.map.plots[q].sea.is_none() && self.map.plots[q].blockaded_count.iter().any(|&v| v != 0) {
                for t in self.team_ids() {
                    self.map.plots[q].blockaded_count[t] = 0;
                }
                changed = true;
            }
        }
        if changed {
            self.update_plot_groups();
        }
    }

    /// The sea of `plot` refreshed, if it has one.
    pub fn plot_sea_refresh(&mut self, plot: PlotIdx) {
        if let Some(s) = self.map.plots[plot].sea {
            self.sea_refresh(s);
        }
    }

    /// The seas whose stored control (or whose plots' blockade) differs from a fresh computation:
    /// a check for tools (empty when it holds; module comment).
    pub fn sea_control_stale(&self) -> Vec<SeaId> {
        let mut out = Vec::new();
        for s in self.sea_ids() {
            let sea = &self.map.seas[s.idx()];
            let fresh = self.sea_closed_now(s);
            let plots_ok = sea.plots.iter().all(|&q| self.map.plots[q].sea == Some(s) && self.team_ids().all(|t| self.map.plots[q].blockaded_count[t] == sea.closed_to.contains(&t) as i32));
            if fresh != sea.closed_to || !plots_ok {
                out.push(s);
            }
        }
        out
    }

    /// The messages of a change of sea `s` (docs/NAVAL.md, 10): to each player with a port on it,
    /// whose team it closed or opened.
    fn sea_messages(&mut self, s: SeaId, before: &[TeamId], now: &[TeamId]) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let name = self.map.seas[s.idx()].name.clone();
        for t in self.team_ids() {
            let (was, is) = (before.contains(&t), now.contains(&t));
            if was == is {
                continue;
            }
            let state = self.sea_state(s, t);
            let ports: Vec<CityId> = self.player_ids().filter(|&q| self.players[q].team == t).flat_map(|q| self.players[q].cities.clone()).filter(|&c| self.cities.contains(c) && self.city_seas(c).contains(&s)).collect();
            if ports.is_empty() {
                continue;
            }
            let text = if is {
                let holder = match state.control {
                    SeaControl::HeldBy(Some(e)) => self.team_label(e),
                    _ => "An enemy".to_string(),
                };
                let mut text = format!("{holder} holds {name} ({} to {}).", state.theirs, state.ours);
                for &c in &ports {
                    if matches!(self.city_harbour(c), Harbour::Blockaded(_)) {
                        text.push_str(&format!(" {} is blockaded.", self.cities[c].name));
                    }
                }
                text
            } else {
                format!("{name} is no longer closed to you ({} to {}).", state.ours, state.theirs)
            };
            let at = self.map.xy(self.city_plot(ports[0]));
            for q in self.player_ids().filter(|&q| self.players[q].team == t && self.players[q].alive) {
                let text = text.clone();
                self.log.msg_at(turn, Some(q), at, || text);
            }
        }
    }

    /// A team's name for messages: its leader's civilization's short description.
    pub fn team_label(&self, t: TeamId) -> String {
        self.team_get_leader_id(t).map_or_else(|| format!("Team {}", t.0), |p| self.player_get_civilization_short_description(p))
    }

    // ---------------------------------------------------------------------------------------
    // Harbours (4.4 point 1)

    /// The seas city `c` touches (its neighbouring water plots' seas), in id order.
    pub fn city_seas(&self, c: CityId) -> Vec<SeaId> {
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        let mut out: Vec<SeaId> = (0..DirectionType::NUM)
            .filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i)))
            .filter(|&w| self.plot_is_water(w))
            .filter_map(|w| self.map.plots[w].sea)
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// City `c`'s harbour (docs/NAVAL.md, 4.4 point 1): blockaded when every sea it touches is
    /// closed to its team (with each sea's holder), open when one is not, none when it touches no
    /// sea. The truth (as stored).
    pub fn city_harbour(&self, c: CityId) -> Harbour {
        let team = self.city_team(c);
        let seas = self.city_seas(c);
        if seas.is_empty() {
            return Harbour::None;
        }
        if seas.iter().any(|&s| !self.sea_is_closed_to(s, team)) {
            return Harbour::Open;
        }
        Harbour::Blockaded(seas.into_iter().map(|s| (s, self.sea_closed_to(s, team).unwrap_or(team))).collect())
    }

    /// City `c`'s harbour if team `holder` held sea `s` against its team (the sea panel's "if X
    /// holds it"): blockaded when every other sea it touches is closed to its team now.
    pub fn city_harbour_if_held(&self, c: CityId, s: SeaId) -> bool {
        let team = self.city_team(c);
        let seas = self.city_seas(c);
        seas.contains(&s) && seas.iter().all(|&o| o == s || self.sea_is_closed_to(o, team))
    }

    /// The cities of `team` that would be blockaded if sea `s` were closed to it (the sea panel's
    /// "If X holds it": its ports there whose other seas are closed to it), in id order.
    pub fn sea_ports_blockaded_if_held(&self, s: SeaId, team: TeamId) -> Vec<CityId> {
        let mut out: Vec<CityId> = self
            .player_ids()
            .filter(|&q| self.players[q].team == team)
            .flat_map(|q| self.players[q].cities.clone())
            .filter(|&c| self.cities.contains(c) && self.city_harbour_if_held(c, s))
            .collect();
        out.sort();
        out
    }

    /// The supply ships of `team` (sea units with room for land units, not the barbarians') at sea
    /// whose lane (as `viewer` can work it out, [`Game::sea_lane_known`]) starts in or passes
    /// through sea `s`, with the lane: the sea panel's lanes that would be cut "if X holds it". In
    /// unit id order.
    pub fn sea_lanes_through(&self, s: SeaId, team: TeamId, viewer: TeamId) -> Vec<(UnitId, crate::naval::SeaLane)> {
        let mut out = Vec::new();
        for q in self.player_ids().filter(|&q| self.players[q].team == team) {
            for &u in &self.players[q].units {
                if !self.units.contains(u) || self.units[u].cargo_capacity <= 0 || self.unit_get_domain_type(u) != DomainType::DOMAIN_SEA {
                    continue;
                }
                let Some(at) = self.unit_plot_opt(u).filter(|&w| self.plot_is_water(w)) else { continue };
                let Some(lane) = self.sea_lane_known(team, at, viewer) else { continue };
                if lane.path.iter().any(|&w| self.map.plots[w].sea == Some(s)) {
                    out.push((u, lane));
                }
            }
        }
        out.sort_by_key(|e| e.0);
        out
    }

    /// The ports of `team` on sea `s` (its cities that touch it), in id order.
    pub fn sea_ports(&self, s: SeaId, team: TeamId) -> Vec<CityId> {
        let mut out: Vec<CityId> = self
            .player_ids()
            .filter(|&q| self.players[q].team == team)
            .flat_map(|q| self.players[q].cities.clone())
            .filter(|&c| self.cities.contains(c) && self.city_seas(c).contains(&s))
            .collect();
        out.sort();
        out
    }

    // ---------------------------------------------------------------------------------------
    // Interception (4.4 point 4)

    /// The ship that would intercept group `g` arriving on `plot` for each enemy with ships on
    /// Patrol in its sea (docs/NAVAL.md, 4.4 point 4), in team order: its strongest ship on Patrol
    /// there that has not intercepted this turn and whose odds against the plot's best defender
    /// are at least `INTERCEPT_MIN_ODDS`. Empty off the seas. Pure; the path preview's "will be
    /// intercepted" when the group stands on `plot` (or [`Game::sea_interceptors_for`] before it
    /// moves).
    pub fn sea_interceptors(&self, g: GroupId, plot: PlotIdx) -> Vec<UnitId> {
        let Some(head) = self.group_get_head_unit(g) else { return Vec::new() };
        self.sea_interceptors_for(head, plot, None)
    }

    /// [`Game::sea_interceptors`] for the group led by `head` on `plot`, the best defender given
    /// (`defender`: a unit that would stand there; `None`: the plot's own best defender against
    /// each ship).
    pub fn sea_interceptors_for(&self, head: UnitId, plot: PlotIdx, defender: Option<UnitId>) -> Vec<UnitId> {
        let Some(s) = self.map.plots[plot].sea else { return Vec::new() };
        let team = self.unit_get_team(head);
        let min = self.d.defines.intercept_min_odds.clamp(0, 100) * 10;
        let mut best: Vec<(TeamId, UnitId, i32)> = Vec::new();
        for ship in self.sea_patrollers(s) {
            let st = self.unit_get_team(ship);
            if st == team || self.units[ship].intercepted {
                continue;
            }
            if !self.unit_is_enemy(ship, Some(team), Some(plot)) && !self.unit_is_enemy(head, Some(st), Some(plot)) {
                continue;
            }
            let owner = self.units[ship].owner;
            let Some(def) = defender.or_else(|| self.plot_get_best_defender(plot, None, Some(owner), Some(ship), true, false, false)) else { continue };
            if self.get_combat_odds(ship, def) < min {
                continue;
            }
            let x = self.unit_sea_strength_x100(ship);
            match best.iter_mut().find(|e| e.0 == st) {
                Some(e) => {
                    if x > e.2 || (x == e.2 && ship < e.1) {
                        e.1 = ship;
                        e.2 = x;
                    }
                }
                None => best.push((st, ship, x)),
            }
        }
        best.sort_by_key(|e| e.0);
        best.into_iter().map(|e| e.1).collect()
    }

    /// Group `g` completed a step onto `plot` (`group_move`, every player alike): each enemy with
    /// ships on Patrol in the plot's sea intercepts it once ([`Game::sea_interceptors`]; a duel
    /// by the rules' own combat code, the ship not moving), one ship per enemy, each ship once a
    /// turn; it stops when no unit of the group is left on the plot. Messages to both sides.
    pub(crate) fn sea_intercept_step(&mut self, g: GroupId, plot: PlotIdx) {
        if self.map.plots[plot].sea.is_none() || self.map.seas.is_empty() {
            return;
        }
        let d = self.d;
        let turn = self.game_turn();
        let at = self.map.xy(plot);
        let sea_name = self.map.plots[plot].sea.map(|s| self.map.seas[s.idx()].name.clone()).unwrap_or_default();
        for ship in self.sea_interceptors(g, plot) {
            // The mover still there?
            if !self.groups.contains(g) || !self.groups[g].units.iter().any(|&u| self.units.contains(u) && self.unit_plot_opt(u) == Some(plot)) {
                return;
            }
            if !self.units.contains(ship) || self.units[ship].intercepted {
                continue;
            }
            let owner = self.units[ship].owner;
            let Some(def) = self.plot_get_best_defender(plot, None, Some(owner), Some(ship), true, false, false) else { continue };
            let (def_owner, def_type, ship_type) = (self.units[def].owner, self.units[def].unit_type, self.units[ship].unit_type);
            let name = |t: UnitType| d.text.name(&d.units[t].base.description).to_string();
            let (sn, dn) = (name(ship_type), name(def_type));
            self.units[ship].intercepted = true;
            self.tel_ctx_push("intercept");
            self.unit_intercept_attack(ship, plot);
            self.tel_ctx_pop();
            let result = if !self.units.contains(def) || self.unit_is_dead(def) {
                "sunk"
            } else if !self.units.contains(ship) || self.unit_is_dead(ship) {
                "repelled; the patrol was lost"
            } else {
                "repelled"
            };
            let them = self.player_get_civilization_short_description(owner);
            let sea = sea_name.clone();
            self.log.msg_at(turn, Some(def_owner), at, || format!("Intercepted in {sea} by {them}'s {sn}: {dn} {result}."));
            let sea = sea_name.clone();
            self.log.msg_at(turn, Some(owner), at, || format!("{sn} intercepted {dn} in {sea}: {result}."));
        }
    }

    // ---------------------------------------------------------------------------------------
    // The turn start

    /// The start of `p`'s turn (after wear, before healing): its units' interception and shore
    /// fire are ready again; the seas its ships patrol are refreshed (wear changed their
    /// strength).
    pub fn player_update_seas(&mut self, p: PlayerId) {
        let mut seas: Vec<SeaId> = Vec::new();
        for u in self.players[p].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            self.units[u].intercepted = false;
            self.units[u].shore_fired = false;
            if self.units[u].blockading {
                if let Some(s) = self.unit_plot_opt(u).and_then(|q| self.map.plots[q].sea) {
                    if !seas.contains(&s) {
                        seas.push(s);
                    }
                }
            }
        }
        seas.sort();
        let mut changed = false;
        for s in seas {
            changed |= self.sea_refresh_quiet(s);
        }
        if changed {
            self.update_plot_groups();
        }
    }
}
