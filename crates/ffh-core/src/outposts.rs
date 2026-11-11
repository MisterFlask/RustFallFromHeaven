//! The sea, phase 4 (this game's own rules, not FFH2's; docs/NAVAL.md, section 9): far shores,
//! outposts, their lanes.
//!
//! **Far shores** ([`Game::map_place_far_shores`], after the starting plots are normalized,
//! through the map RNG): a far shore is a land area with no starting plot and at least
//! `FAR_SHORE_MIN_PLOTS` plots. When the map has one, each bonus flagged `far_shore` on a land
//! area with a starting plot (outside every starting plot's city radius) moves to a plot of a far
//! shore that can hold it (`plot_can_have_bonus`, no improvement),
//! drawn at random; one that fits nowhere stays. Each far shore then gets one raw mana node per
//! `FAR_SHORE_MANA_PLOTS` of its plots, at least one, on plots that can hold it.
//!
//! **Outposts** (`City::outpost`): founded by the Colonist (`found_outpost`) or by any unit that
//! founds cities, by its second action (`MISSION_FOUND` with data1 [`FOUND_OUTPOST`]), on a plot
//! where a city can be founded, next to water of the ocean's minimum size, on a land area other
//! than its owner's capital's ([`Game::player_outpost_site_refusal`]). An outpost has radius 1
//! (the 8 plots around it, borders at culture level 1 at most), grows to `OUTPOST_MAX_POP`,
//! builds nothing, has no governor, pays no maintenance, is never a capital and is not counted
//! in any number-of-cities cost; the rules that check these test `City::outpost` where they
//! stand. Raising it ([`Game::player_raise_outpost`], `OUTPOST_RAISE_COST` gold scaled by the
//! game speed's `construct_percent`) makes it a city.
//!
//! **Lanes** (9.3): an outpost's lane runs over water in its team's range to the water by the
//! nearest home port, a city of its owner that is not an outpost ([`Game::outpost_lane`]). At the
//! start of its owner's turn ([`Game::player_update_lanes`]): open, `lane_cut_turns` is 0 and the
//! path is stored (`City::lane`); cut or missing, `lane_cut_turns` + 1 and no path. The trade
//! network reads the stored paths: the plots of an open lane are network terrain for the team
//! (`plot_is_network_terrain`), and the outpost's plots are off the network while its lane is not
//! open ([`Game::plot_outpost_cut_off`]). Raiding gold: a cut lane pays `LANE_RAID_GOLD` to the
//! players of the holding team with ships on Patrol in the cutting sea; an open lane through a
//! sea where an enemy patrols without holding it pays `LANE_RAID_GOLD_CONTESTED` to the strongest
//! such enemy's players with ships on Patrol there (docs/NAVAL.md, 4.4 point 5). Above
//! `LANE_STARVE_TURNS` the outpost starves: -1 population a turn (to 1) and no port.

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::naval::SeaLane;

/// `MISSION_FOUND`'s data1 that founds an outpost instead of a city.
pub const FOUND_OUTPOST: i32 = 1;

/// Why a plot is no site for an outpost of a player (docs/NAVAL.md, 9.2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OutpostSiteRefusal {
    /// No city can be founded here.
    NoCity,
    /// Not next to water of the ocean's minimum size.
    NotCoastal,
    /// The player has no capital.
    NoCapital,
    /// On the landmass of the player's capital.
    CapitalLandmass,
}

impl OutpostSiteRefusal {
    /// The bare reason (docs/NAVAL.md, 10).
    pub fn text(self) -> &'static str {
        match self {
            OutpostSiteRefusal::NoCity => "A city cannot be founded here.",
            OutpostSiteRefusal::NotCoastal => "An outpost needs a coast on the ocean.",
            OutpostSiteRefusal::NoCapital => "An outpost needs a capital.",
            OutpostSiteRefusal::CapitalLandmass => "An outpost must be on another landmass than your capital's.",
        }
    }
}

/// The state of an outpost's lane (docs/NAVAL.md, 9.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaneState {
    /// Open: the lane, its home port and its plots.
    Open(SeaLane),
    /// Cut in that sea, held by that team.
    Cut(SeaLane),
    /// No lane to a home port in range.
    NoLane,
}

/// What the far-shore pass did (docs/NAVAL.md, 9.1; [`Game::far_shore_report`]): for tools.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FarShoreReport {
    /// The far shores and their plots.
    pub far_shores: Vec<(AreaId, i32)>,
    /// Each flagged bonus moved: its type, from, to.
    pub moved: Vec<(BonusType, PlotIdx, PlotIdx)>,
    /// Each flagged bonus that stayed on a starting landmass because no far-shore plot could hold
    /// it.
    pub stayed: Vec<(BonusType, PlotIdx)>,
    /// Each flagged bonus kept because it is inside a starting plot's city radius.
    pub kept_by_start: Vec<(BonusType, PlotIdx)>,
    /// The mana nodes added.
    pub mana: Vec<PlotIdx>,
    /// The mana nodes the far shores' sizes asked for.
    pub mana_wanted: i32,
}

/// The outposts' share of the trade network, by plot (outposts.rs module comment): a bit per
/// team. Derived from the cities; cached with the sea's range fields (`SeaRangeCache`).
#[derive(Clone, Debug, Default)]
pub struct OutpostNet {
    /// Per plot: the teams with an open outpost lane over it (empty: no outpost lane at all).
    lane: Vec<u64>,
    /// Per plot: the teams for which the plot is cut off (an outpost of theirs is there or next
    /// to it, its lane not open; empty: none).
    cut: Vec<u64>,
}

impl Game {
    // ---------------------------------------------------------------------------------------
    // What an outpost is (9.2)

    /// Whether city `c` is an outpost.
    pub fn city_is_outpost(&self, c: CityId) -> bool {
        self.cities[c].outpost
    }

    /// C++-style count: the outposts of player `p`.
    pub fn player_get_num_outposts(&self, p: PlayerId) -> i32 {
        self.players[p].cities.iter().filter(|&&c| self.cities.contains(c) && self.cities[c].outpost).count() as i32
    }

    /// The cities of player `p` that are not outposts.
    pub fn player_num_real_cities(&self, p: PlayerId) -> i32 {
        self.players[p].num_cities() - self.player_get_num_outposts(p)
    }

    /// Whether outpost `c` has reached `OUTPOST_MAX_POP` (false for a city).
    pub fn city_outpost_full(&self, c: CityId) -> bool {
        self.cities[c].outpost && self.cities[c].population >= self.d.defines.outpost_max_pop
    }

    /// Whether outpost `c` is starved: its lane cut or missing for more than `LANE_STARVE_TURNS`
    /// turns (docs/NAVAL.md, 9.3). False for a city.
    pub fn city_outpost_starved(&self, c: CityId) -> bool {
        self.cities.contains(c) && self.cities[c].outpost && self.cities[c].lane_cut_turns > self.d.defines.lane_starve_turns
    }

    /// The gold raising outpost `c` costs: `OUTPOST_RAISE_COST` scaled by the game speed's
    /// `construct_percent`.
    pub fn outpost_raise_cost(&self) -> i32 {
        self.d.defines.outpost_raise_cost * self.d.game_speeds[self.game_speed_type()].construct_percent / 100
    }

    /// Why player `p` cannot raise city `c` now, `None` when it can.
    pub fn player_raise_outpost_refusal(&self, p: PlayerId, c: CityId) -> Option<&'static str> {
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Some("Not your city.");
        }
        if !self.cities[c].outpost {
            return Some("Not an outpost.");
        }
        if self.players[p].gold < self.outpost_raise_cost() {
            return Some("Not enough gold.");
        }
        None
    }

    /// Player `p` raises outpost `c` to a city (docs/NAVAL.md, 9.2): pays the cost; from now every
    /// rule of cities applies (its radius, borders, free buildings, governor, maintenance).
    /// False if it cannot.
    pub fn player_raise_outpost(&mut self, p: PlayerId, c: CityId) -> bool {
        if self.player_raise_outpost_refusal(p, c).is_some() {
            return false;
        }
        let d = self.d;
        self.players[p].gold -= self.outpost_raise_cost();
        self.cities[c].outpost = false;
        self.cities[c].lane_cut_turns = 0;
        self.cities[c].lane.clear();
        // A player with a city limit (the Kuriotates): a city beyond it is a settlement.
        let max_cities = self.players[p].max_cities;
        if max_cities != -1 && self.players[p].num_cities() - self.player_get_num_settlements(p) - self.player_get_num_outposts(p) - 1 >= max_cities {
            self.cities[c].settlement = true;
        }
        let radius = self.city_base_plot_radius(c);
        self.city_set_plot_radius(c, radius);
        self.city_update_culture_level(c, false);
        for building in d.buildings.keys() {
            if self.player_is_building_free(p, building) {
                self.city_set_num_free_building(c, building, 1);
            }
        }
        self.city_governance_init(c, true);
        self.cities[c].ai_choose_production_dirty = true;
        self.player_update_maintenance(p);
        self.sea_range_invalidate();
        self.update_plot_groups();
        let turn = self.game_turn();
        let at = (self.cities[c].x, self.cities[c].y);
        let name = self.cities[c].name.clone();
        self.log.msg_at(turn, Some(p), at, || format!("{name} is raised to a city."));
        true
    }

    // ---------------------------------------------------------------------------------------
    // Founding (9.2)

    /// Why `plot` is no outpost site for player `p` (docs/NAVAL.md, 9.2), `None` when it is one:
    /// a city can be founded there, it is next to water of the ocean's minimum size
    /// (`MIN_WATER_SIZE_FOR_OCEAN`), and it is on another land area than `p`'s capital.
    pub fn player_outpost_site_refusal(&self, p: PlayerId, plot: PlotIdx, test_visible: bool) -> Option<OutpostSiteRefusal> {
        let (x, y) = self.map.xy(plot);
        if !self.player_can_found(p, x, y, test_visible) {
            return Some(OutpostSiteRefusal::NoCity);
        }
        if !self.plot_is_coastal_land(plot, self.d.defines.min_water_size_for_ocean) {
            return Some(OutpostSiteRefusal::NotCoastal);
        }
        let Some(capital) = self.player_get_capital_city(p) else { return Some(OutpostSiteRefusal::NoCapital) };
        if self.map.plots[self.city_plot(capital)].area == self.map.plots[plot].area {
            return Some(OutpostSiteRefusal::CapitalLandmass);
        }
        None
    }

    /// Whether player `p` can found an outpost at (`x`, `y`).
    pub fn player_can_found_outpost(&self, p: PlayerId, x: i32, y: i32, test_visible: bool) -> bool {
        self.map.plot_xy(x, y).is_some_and(|plot| self.player_outpost_site_refusal(p, plot, test_visible).is_none())
    }

    /// Whether unit `u` founds outposts at all: a Colonist (`found_outpost`) or a unit that
    /// founds cities.
    pub fn unit_founds_outposts(&self, u: UnitId) -> bool {
        let info = &self.d.units[self.units[u].unit_type];
        info.found_outpost || info.found
    }

    /// Whether unit `u` can found an outpost on `plot` (as `unit_can_found` for a city): it
    /// founds outposts, is not in boats, has not cast this turn, the plot is not disabled for
    /// founding and it is an outpost site of its owner.
    pub fn unit_can_found_outpost(&self, u: UnitId, plot: PlotIdx, test_visible: bool) -> bool {
        self.unit_found_outpost_refusal(u, plot, test_visible).is_none() && self.unit_founds_outposts(u)
    }

    /// Why unit `u` cannot found an outpost on `plot` (the site's reason when that is it);
    /// `None` when it can (or when it does not found outposts at all: check
    /// [`Game::unit_founds_outposts`]).
    pub fn unit_found_outpost_refusal(&self, u: UnitId, plot: PlotIdx, test_visible: bool) -> Option<&'static str> {
        if self.unit_in_boats_at(u, plot) {
            return Some("Not from boats.");
        }
        if self.map.plots[plot].found_disabled || self.units[u].has_casted {
            return Some("A city cannot be founded here.");
        }
        self.player_outpost_site_refusal(self.units[u].owner, plot, test_visible).map(|r| r.text())
    }

    /// Unit `u` founds an outpost where it stands (as `unit_found`). True if it did.
    pub fn unit_found_outpost(&mut self, u: UnitId) -> bool {
        let plot = self.unit_plot(u);
        if !self.unit_can_found_outpost(u, plot, false) {
            return false;
        }
        let owner = self.units[u].owner;
        let (x, y) = (self.units[u].x, self.units[u].y);
        if self.player_found_outpost(owner, x, y).is_none() {
            return false;
        }
        if self.units.contains(u) {
            self.unit_kill(u, true, None);
        }
        true
    }

    /// Player `p` founds an outpost at (`x`, `y`) (as `player_found` for a city): the city is an
    /// outpost from its start; its lane is traced at once. `None` when the site is no outpost
    /// site.
    pub fn player_found_outpost(&mut self, p: PlayerId, x: i32, y: i32) -> Option<CityId> {
        if !self.player_can_found_outpost(p, x, y, false) {
            return None;
        }
        let city = self.player_init_city_as(p, x, y, true, true, None, true);
        if self.telemetry.on() {
            self.tel_found(p, city);
        }
        self.city_trace_lane(city);
        self.city_do_found_message(city);
        self.ev_city_built(city);
        Some(city)
    }

    // ---------------------------------------------------------------------------------------
    // Lanes (9.3)

    /// The home ports of outpost `c`: its owner's cities that are not outposts, in id order.
    pub fn outpost_home_ports(&self, c: CityId) -> Vec<CityId> {
        let p = self.cities[c].owner;
        let mut out: Vec<CityId> = self.players[p].cities.iter().copied().filter(|&o| self.cities.contains(o) && !self.cities[o].outpost).collect();
        out.sort();
        out
    }

    /// The path and home port of outpost `c`'s lane (docs/NAVAL.md, 5 and 9.3): the shortest path
    /// over passable water, every plot in its team's range, from a water plot next to it to a
    /// water plot next to one of its home ports; ties: fewer plots, then the lower plot index at
    /// the first difference; the port: the lowest city id next to the last plot. `None`: no lane
    /// (the barbarians and a team with no range have none).
    fn outpost_lane_path(&self, c: CityId) -> Option<(Vec<PlotIdx>, CityId)> {
        let team = self.city_team(c);
        if self.player_is_barbarian(self.cities[c].owner) || self.team_sea_exempt(team) {
            return None;
        }
        let ports = self.outpost_home_ports(c);
        if ports.is_empty() {
            return None;
        }
        let n = self.map.num_plots();
        let range = self.team_sea_range(team);
        let field = self.sea_range_dists(team);
        let ok = |q: PlotIdx| self.plot_is_water(q) && !self.plot_is_impassable(q) && field[q.idx()] != u16::MAX && (field[q.idx()] as i32) <= range;
        const FAR: u32 = u32::MAX;
        let mut dist = vec![FAR; n];
        let mut port = vec![FAR; n];
        let mut layer: Vec<PlotIdx> = Vec::new();
        for &h in &ports {
            let (x, y) = (self.cities[h].x, self.cities[h].y);
            for i in 0..DirectionType::NUM {
                let Some(w) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                if !ok(w) {
                    continue;
                }
                if dist[w.idx()] == FAR {
                    dist[w.idx()] = 1;
                    port[w.idx()] = h.0;
                    layer.push(w);
                } else if h.0 < port[w.idx()] {
                    port[w.idx()] = h.0;
                }
            }
        }
        layer.sort();
        let mut d = 1u32;
        while !layer.is_empty() {
            let mut next = Vec::new();
            for &q in &layer {
                let (x, y) = self.map.xy(q);
                for i in 0..DirectionType::NUM {
                    let Some(w) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                    if dist[w.idx()] == FAR && ok(w) {
                        dist[w.idx()] = d + 1;
                        next.push(w);
                    }
                }
            }
            next.sort();
            layer = next;
            d += 1;
        }
        // From the outpost: the best of its water plots' paths.
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        let mut starts: Vec<PlotIdx> = (0..DirectionType::NUM).filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i))).filter(|&w| dist[w.idx()] != FAR).collect();
        starts.sort();
        starts.dedup();
        let mut best: Option<Vec<PlotIdx>> = None;
        for s in starts {
            let mut path = vec![s];
            let mut cur = s;
            while dist[cur.idx()] > 1 {
                let (cx, cy) = self.map.xy(cur);
                let want = dist[cur.idx()] - 1;
                let next = (0..DirectionType::NUM).filter_map(|i| self.plot_direction(cx, cy, DirectionType::from_usize(i))).filter(|&w| dist[w.idx()] == want).min();
                let Some(nx) = next else { break };
                path.push(nx);
                cur = nx;
            }
            if dist[cur.idx()] != 1 {
                continue;
            }
            if best.as_ref().is_none_or(|b| (path.len(), &path) < (b.len(), b)) {
                best = Some(path);
            }
        }
        let path = best?;
        let last = *path.last()?;
        // The port: the lowest id among the home ports next to the last plot.
        let (lx, ly) = self.map.xy(last);
        let by_last = ports.iter().copied().filter(|&h| {
            let (hx, hy) = (self.cities[h].x, self.cities[h].y);
            self.plot_distance(lx, ly, hx, hy) <= 1
        });
        let home = by_last.min().or_else(|| (port[last.idx()] != FAR).then(|| CityId(port[last.idx()])))?;
        Some((path, home))
    }

    /// Outpost `c`'s lane (docs/NAVAL.md, 9.3), `None` when it has none or is no outpost: cut in
    /// the first sea along it, from the outpost, that is closed to its team (as stored). The
    /// truth.
    pub fn outpost_lane(&self, c: CityId) -> Option<SeaLane> {
        if !self.cities[c].outpost {
            return None;
        }
        let team = self.city_team(c);
        let (path, port) = self.outpost_lane_path(c)?;
        let mut cut_in = None;
        for &q in &path {
            if let Some(s) = self.map.plots[q].sea {
                if self.sea_is_closed_to(s, team) {
                    cut_in = Some((s, self.sea_closed_to(s, team).unwrap_or_else(|| self.barbarian_team())));
                    break;
                }
            }
        }
        Some(SeaLane { path, port, cut_in })
    }

    /// Outpost `c`'s lane as team `viewer` can work it out: the same path, cut where the control
    /// the viewer can work out closes a sea (only ships it sees count).
    pub fn outpost_lane_known(&self, c: CityId, viewer: TeamId) -> Option<SeaLane> {
        if !self.cities[c].outpost {
            return None;
        }
        let team = self.city_team(c);
        if team == viewer {
            return self.outpost_lane(c);
        }
        let (path, port) = self.outpost_lane_path(c)?;
        let mut cut_in = None;
        let mut seen: Vec<SeaId> = Vec::new();
        for &q in &path {
            if let Some(s) = self.map.plots[q].sea {
                if seen.contains(&s) {
                    continue;
                }
                seen.push(s);
                if let Some(holder) = self.sea_closed_known(s, team, viewer) {
                    cut_in = Some((s, holder.unwrap_or_else(|| self.barbarian_team())));
                    break;
                }
            }
        }
        Some(SeaLane { path, port, cut_in })
    }

    /// The state of outpost `c`'s lane as `viewer` can work it out (its own team: the truth).
    pub fn outpost_lane_state(&self, c: CityId, viewer: TeamId) -> LaneState {
        match self.outpost_lane_known(c, viewer) {
            None => LaneState::NoLane,
            Some(l) if l.is_cut() => LaneState::Cut(l),
            Some(l) => LaneState::Open(l),
        }
    }

    /// The outposts of `team` whose lane (as `viewer` can work it out) passes through sea `s`,
    /// with the lane, in city id order: the sea panel's lanes.
    pub fn sea_outpost_lanes_through(&self, s: SeaId, team: TeamId, viewer: TeamId) -> Vec<(CityId, SeaLane)> {
        let mut out = Vec::new();
        for q in self.player_ids().filter(|&q| self.players[q].team == team) {
            for &c in &self.players[q].cities {
                if !self.cities.contains(c) || !self.cities[c].outpost {
                    continue;
                }
                let Some(lane) = self.outpost_lane_known(c, viewer) else { continue };
                if lane.path.iter().any(|&w| self.map.plots[w].sea == Some(s)) {
                    out.push((c, lane));
                }
            }
        }
        out.sort_by_key(|e| e.0);
        out
    }

    /// Traces outpost `c`'s lane now and stores its path (`City::lane`; empty when cut or
    /// missing), without counting a turn: on founding and capture. The trade network follows.
    pub fn city_trace_lane(&mut self, c: CityId) {
        let path = match self.outpost_lane(c) {
            Some(l) if !l.is_cut() => l.path,
            _ => Vec::new(),
        };
        if path != self.cities[c].lane {
            self.cities[c].lane = path;
            self.sea_range_invalidate();
            self.update_plot_groups();
        }
    }

    /// The players of `team` with a ship on Patrol in sea `s`, in id order.
    fn sea_patrolling_players(&self, s: SeaId, team: TeamId) -> Vec<PlayerId> {
        let mut out: Vec<PlayerId> = self.sea_patrollers(s).into_iter().filter(|&u| self.unit_get_team(u) == team).map(|u| self.units[u].owner).collect();
        out.sort();
        out.dedup();
        out
    }

    /// Shares `gold` among `players` (the remainder to the lowest ids first: one each), with a
    /// message each.
    fn lane_raid_pay(&mut self, players: &[PlayerId], gold: i32, sea: SeaId, outpost: CityId) {
        if players.is_empty() || gold <= 0 {
            return;
        }
        let n = players.len() as i32;
        let turn = self.game_turn();
        let at = (self.cities[outpost].x, self.cities[outpost].y);
        let sea_name = self.map.seas[sea.idx()].name.clone();
        let city_name = self.cities[outpost].name.clone();
        for (i, &q) in players.iter().enumerate() {
            let share = gold / n + i32::from((i as i32) < gold % n);
            if share <= 0 {
                continue;
            }
            self.players[q].gold += share;
            let (sn, cn) = (sea_name.clone(), city_name.clone());
            self.log.msg_at(turn, Some(q), at, || format!("Lane of {cn} raided in {sn}: +{share} gold."));
        }
    }

    /// The strongest team at war with `team` (or patrolling it with hidden ships) on Patrol in
    /// sea `s`, with its strength: ties to the lower team. `None` when no enemy patrols.
    fn sea_strongest_enemy(&self, s: SeaId, team: TeamId) -> Option<TeamId> {
        let mut forces: Vec<(TeamId, i32)> = Vec::new();
        for u in self.sea_patrollers(s) {
            let t = self.unit_get_team(u);
            if t == team || !(self.at_war(t, team) || self.unit_is_hidden_nationality(u)) {
                continue;
            }
            let x = self.unit_sea_strength_x100(u);
            match forces.iter_mut().find(|e| e.0 == t) {
                Some(e) => e.1 += x,
                None => forces.push((t, x)),
            }
        }
        forces.into_iter().filter(|e| e.1 / 100 > 0).min_by(|a, b| (b.1 / 100).cmp(&(a.1 / 100)).then(a.0.cmp(&b.0))).map(|e| e.0)
    }

    /// The start of `p`'s turn, after the seas' refresh (docs/NAVAL.md, 9.3 and 4.4 point 5): each
    /// outpost's lane is traced; open, its count goes to 0 (and a sea it passes through where an
    /// enemy patrols without holding it pays that enemy `LANE_RAID_GOLD_CONTESTED`); cut or
    /// missing, its count goes up (a cut pays the holder `LANE_RAID_GOLD`); above
    /// `LANE_STARVE_TURNS` it loses 1 population (to 1). Messages on a change. The trade network
    /// and the range fields follow.
    pub fn player_update_lanes(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        let mut outposts: Vec<CityId> = self.players[p].cities.iter().copied().filter(|&c| self.cities.contains(c) && self.cities[c].outpost).collect();
        if outposts.is_empty() {
            return;
        }
        outposts.sort();
        let d = self.d;
        let team = self.players[p].team;
        let turn = self.game_turn();
        let mut net_changed = false;
        let mut ports_changed = false;
        for c in outposts {
            if !self.cities.contains(c) {
                continue;
            }
            let was_starved = self.city_outpost_starved(c);
            let at = (self.cities[c].x, self.cities[c].y);
            let name = self.cities[c].name.clone();
            let lane = self.outpost_lane(c);
            let starve = d.defines.lane_starve_turns;
            let new_path = match &lane {
                Some(l) if !l.is_cut() => {
                    if self.cities[c].lane_cut_turns > 0 {
                        let port = self.cities[l.port].name.clone();
                        self.log.msg_at(turn, Some(p), at, || format!("{name}: lane open again, to {port}."));
                    }
                    self.cities[c].lane_cut_turns = 0;
                    // Raiding (4.4 point 5): each sea along it where an enemy patrols, not holding it.
                    let mut seas: Vec<SeaId> = Vec::new();
                    for &q in &l.path {
                        if let Some(s) = self.map.plots[q].sea {
                            if !seas.contains(&s) {
                                seas.push(s);
                            }
                        }
                    }
                    for s in seas {
                        if let Some(e) = self.sea_strongest_enemy(s, team) {
                            let players = self.sea_patrolling_players(s, e);
                            self.lane_raid_pay(&players, d.defines.lane_raid_gold_contested, s, c);
                        }
                    }
                    l.path.clone()
                }
                Some(l) => {
                    let (s, holder) = l.cut_in.expect("a cut lane");
                    self.cities[c].lane_cut_turns += 1;
                    if self.cities[c].lane_cut_turns == 1 {
                        let (sea, who) = (self.map.seas[s.idx()].name.clone(), self.team_label(holder));
                        self.log.msg_at(turn, Some(p), at, || format!("{name}: lane cut in {sea} by {who}. Starves after {starve} turns."));
                    }
                    let players = self.sea_patrolling_players(s, holder);
                    self.lane_raid_pay(&players, d.defines.lane_raid_gold, s, c);
                    Vec::new()
                }
                None => {
                    self.cities[c].lane_cut_turns += 1;
                    if self.cities[c].lane_cut_turns == 1 {
                        self.log.msg_at(turn, Some(p), at, || format!("{name}: no lane to a home port in range. Starves after {starve} turns."));
                    }
                    Vec::new()
                }
            };
            if new_path != self.cities[c].lane {
                self.cities[c].lane = new_path;
                net_changed = true;
            }
            let starved = self.city_outpost_starved(c);
            if starved != was_starved {
                ports_changed = true;
            }
            if starved {
                if !was_starved {
                    self.log.msg_at(turn, Some(p), at, || format!("{name} is starved: -1 population a turn, not a port."));
                }
                if self.cities[c].population > 1 {
                    self.city_change_population(c, -1);
                }
            }
        }
        if net_changed || ports_changed {
            self.sea_range_invalidate();
        }
        if net_changed {
            self.update_plot_groups();
        }
    }

    // ---------------------------------------------------------------------------------------
    // The trade network (9.3)

    /// The outposts' network bits of `plot`: (open lanes, cut off), a bit per team.
    fn outpost_net_bits(&self, plot: PlotIdx) -> (u64, u64) {
        if let Ok(cache) = self.sea_range.try_borrow() {
            if let Some(net) = &cache.outpost_net {
                return net.bits(plot);
            }
        }
        let net = self.outpost_net_build();
        let out = net.bits(plot);
        if let Ok(mut cache) = self.sea_range.try_borrow_mut() {
            cache.outpost_net = Some(net);
        }
        out
    }

    /// Builds the outposts' network from the cities (module comment).
    fn outpost_net_build(&self) -> OutpostNet {
        let n = self.map.num_plots();
        let mut net = OutpostNet::default();
        for p in self.player_ids() {
            let team = self.players[p].team;
            if team.idx() >= 64 {
                continue;
            }
            let bit = 1u64 << team.idx();
            for &c in &self.players[p].cities {
                if !self.cities.contains(c) || !self.cities[c].outpost {
                    continue;
                }
                if self.cities[c].lane.is_empty() {
                    if net.cut.is_empty() {
                        net.cut = vec![0; n];
                    }
                    let (x, y) = (self.cities[c].x, self.cities[c].y);
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            if let Some(q) = self.plot_xy(x, y, dx, dy) {
                                net.cut[q.idx()] |= bit;
                            }
                        }
                    }
                } else {
                    if net.lane.is_empty() {
                        net.lane = vec![0; n];
                    }
                    for &q in &self.cities[c].lane {
                        if q.idx() < n {
                            net.lane[q.idx()] |= bit;
                        }
                    }
                }
            }
        }
        net
    }

    /// Whether `plot` is on an open lane of an outpost of `team` (it carries the team's trade).
    pub fn plot_on_outpost_lane(&self, plot: PlotIdx, team: TeamId) -> bool {
        team.idx() < 64 && self.outpost_net_bits(plot).0 & (1u64 << team.idx()) != 0
    }

    /// Whether `plot` is cut off from `team`'s trade network: an outpost of `team` whose lane is
    /// not open stands on it or next to it.
    pub fn plot_outpost_cut_off(&self, plot: PlotIdx, team: TeamId) -> bool {
        team.idx() < 64 && self.outpost_net_bits(plot).1 & (1u64 << team.idx()) != 0
    }

    // ---------------------------------------------------------------------------------------
    // Far shores (9.1)

    /// The far shores of the map (docs/NAVAL.md, 9.1): land areas with no starting plot and at
    /// least `FAR_SHORE_MIN_PLOTS` plots, in id order. Empty when no area has a starting plot.
    pub fn map_far_shores(&self) -> Vec<AreaId> {
        if !self.map.areas.iter().any(|(_, a)| !a.water && a.num_starting_plots > 0) {
            return Vec::new();
        }
        let min = self.d.defines.far_shore_min_plots;
        self.map.areas.iter().filter(|(_, a)| !a.water && a.num_starting_plots == 0 && a.num_tiles >= min).map(|(id, _)| id).collect()
    }

    /// Whether far-shore plot `q` may take bonus `b`: it can hold it (the rules' own test) and has
    /// no improvement (a lair, a goody hut).
    fn far_shore_fits(&self, q: PlotIdx, b: BonusType) -> bool {
        self.map.plots[q].improvement_type.is_none() && self.plot_can_have_bonus(q, Some(b), false)
    }

    /// The far-shore pass (docs/NAVAL.md, 9.1; module comment), through the map RNG. Nothing
    /// changes on a map with no far shore. Returns what it did (also kept for tools:
    /// [`Game::far_shore_report`]).
    pub fn map_place_far_shores(&mut self) -> FarShoreReport {
        let d = self.d;
        let mut report = FarShoreReport::default();
        let shores = self.map_far_shores();
        if shores.is_empty() {
            self.sea_range.get_mut().far_shores = Some(report.clone());
            return report;
        }
        report.far_shores = shores.iter().map(|&a| (a, self.map.areas[a].num_tiles)).collect();
        let starts: Vec<PlotIdx> = self.player_ids().filter_map(|p| self.player_get_starting_plot(p)).collect();
        let shore_plots: Vec<PlotIdx> = self.map.plot_ids().filter(|&q| self.map.plots[q].area.is_some_and(|a| shores.contains(&a))).collect();
        // The flagged bonuses on the starting landmasses, in plot order.
        let flagged: Vec<(PlotIdx, BonusType)> = self
            .map
            .plot_ids()
            .filter_map(|q| self.map.plots[q].bonus_type.map(|b| (q, b)))
            .filter(|&(q, b)| d.bonuses[b].far_shore && !self.plot_is_water(q) && self.map.plots[q].area.is_some_and(|a| self.map.areas[a].num_starting_plots > 0))
            .collect();
        for (q, b) in flagged {
            let (x, y) = self.map.xy(q);
            if starts.iter().any(|&s| {
                let (sx, sy) = self.map.xy(s);
                crate::utils::plot_city_xy_radius(self.dx_wrap(x - sx), self.dy_wrap(y - sy), 2).is_some()
            }) {
                report.kept_by_start.push((b, q));
                continue;
            }
            // `plot_can_have_bonus` refuses a plot holding a bonus: take it off first.
            self.plot_set_bonus_type(q, None);
            let fits: Vec<PlotIdx> = shore_plots.iter().copied().filter(|&o| self.far_shore_fits(o, b)).collect();
            if fits.is_empty() {
                self.plot_set_bonus_type(q, Some(b));
                report.stayed.push((b, q));
                continue;
            }
            let to = fits[self.map_rand_num(fits.len() as i32, "Far shore bonus") as usize];
            self.plot_set_bonus_type(to, Some(b));
            report.moved.push((b, q, to));
        }
        // One more raw mana node per FAR_SHORE_MANA_PLOTS of each far shore, at least one.
        if let Some(mana) = opt_from_i32::<BonusType>(d.defines.bonus_mana) {
            let per = d.defines.far_shore_mana_plots.max(1);
            for &a in &shores {
                let count = (self.map.areas[a].num_tiles / per).max(1);
                report.mana_wanted += count;
                for _ in 0..count {
                    let fits: Vec<PlotIdx> = shore_plots.iter().copied().filter(|&o| self.map.plots[o].area == Some(a) && self.far_shore_fits(o, mana)).collect();
                    if fits.is_empty() {
                        break;
                    }
                    let to = fits[self.map_rand_num(fits.len() as i32, "Far shore mana") as usize];
                    self.plot_set_bonus_type(to, Some(mana));
                    report.mana.push(to);
                }
            }
        }
        self.sea_range.get_mut().far_shores = Some(report.clone());
        report
    }

    /// What the far-shore pass did in this game (`None` after a load or in a game whose map was
    /// not generated): for tools.
    pub fn far_shore_report(&self) -> Option<FarShoreReport> {
        self.sea_range.borrow().far_shores.clone()
    }

    /// Whether player `p` has a city on a land area holding a plot it has revealed with bonus `b`
    /// (the plot tooltip's `Not on your landmasses.` when false; docs/NAVAL.md, 10).
    pub fn player_bonus_on_own_landmass(&self, p: PlayerId, b: BonusType) -> bool {
        let team = self.players[p].team;
        let areas: Vec<AreaId> = self.players[p].cities.iter().filter(|&&c| self.cities.contains(c)).filter_map(|&c| self.map.plots[self.city_plot(c)].area).collect();
        self.map.plot_ids().any(|q| self.map.plots[q].area.is_some_and(|a| areas.contains(&a)) && self.map.plots[q].revealed[team] && self.plot_get_bonus_type(q, Some(team)) == Some(b))
    }
}

impl OutpostNet {
    fn bits(&self, plot: PlotIdx) -> (u64, u64) {
        let i = plot.idx();
        (self.lane.get(i).copied().unwrap_or(0), self.cut.get(i).copied().unwrap_or(0))
    }
}
