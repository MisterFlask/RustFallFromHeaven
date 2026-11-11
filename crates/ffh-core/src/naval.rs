//! The sea, phase 1 (this game's own rules, not FFH2's; docs/NAVAL.md, section 3): range, boats,
//! wear.
//!
//! **Range.** A team's ports are the cities next to water of its own players, of its vassals and
//! its master, and of the teams it has open borders with (none at war with it). The range field
//! ([`Game::plot_sea_range`]) is, for each water plot, the fewest steps over passable water
//! (8 directions) from a water plot next to one of those ports (a plot next to a port is at 1);
//! the plot is in range when that is at most the team's range ([`Game::team_sea_range`]:
//! `SEA_RANGE` plus each known tech's `sea_range_change`). At the start of its owner's turn a
//! sea unit or a unit in boats on a water plot out of range loses `SEA_ATTRITION_PERCENT` of its
//! full HP (this kills) and does not heal ([`Game::player_update_sea`]). Exempt: the barbarians,
//! animals, units carried by a ship, and a team with no city (range is not traced at all).
//!
//! **Boats.** A land unit of a player (not the barbarians, not an animal) may step onto a water
//! plot; not carried by a ship there, it is in boats ([`Game::unit_is_in_boats`]): each water plot
//! it enters costs its whole turn split `EMBARKED_MOVES` + its civilization's
//! `embarked_moves_change` ways (`Game::plot_movement_cost`), it defends with strength
//! `EMBARKED_STRENGTH` and no modifier, it cannot attack or enter a plot holding an enemy unit or
//! city, and it does nothing but move, wait or be deleted (no healing, fortifying, casting,
//! building, pillaging, founding; sight 1). Barbarians and animals never use boats.
//!
//! **Wear.** A warship of a player counts its turn starts outside a port city of its team
//! (`Unit::sea_turns`); beyond `SEA_WEAR_FREE_TURNS` its strength falls `SEA_WEAR_PERCENT` a turn,
//! to at most `SEA_WEAR_MAX_PERCENT` ([`Game::unit_sea_wear_percent`], a term of its strength).
//!
//! **The cache.** The field is derived state: computed from the map, cached per team in
//! `Game::sea_range` (not saved) and checked on every read against what it was computed from
//! (which teams' cities are the team's ports, which team each player is on); the cities and the
//! plots' water and passability clear it when they change (`Game::sea_range_invalidate`, called
//! by the city and plot setters). A read from the cache equals a fresh computation, so the cache
//! cannot change a game's course.

use std::cell::RefCell;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// No distance / no port in a [`SeaField`].
const NONE16: u16 = u16::MAX;
const NONE32: u32 = u32::MAX;

/// One team's range field (module comment) and what it was computed from.
#[derive(Clone, Debug)]
struct SeaField {
    /// Per team: whether its cities were this team's ports.
    port_teams: Vec<bool>,
    /// Per player: its team.
    player_teams: Vec<TeamId>,
    /// Per plot: the steps from the nearest port (`NONE16`: land, or unreachable).
    dist: Vec<u16>,
    /// Per plot: the nearest port's city id (ties to the lower id; `NONE32`: none).
    port: Vec<u32>,
}

/// The cached range fields, per team (module comment). Not game state: not saved.
#[derive(Clone, Debug, Default)]
pub struct SeaRangeCache {
    fields: Vec<Option<SeaField>>,
    /// The techs with a `sea_range_change` (from the data, which never changes).
    range_techs: Option<Vec<(TechType, i32)>>,
}

impl SeaRangeCache {
    pub fn new() -> SeaRangeCache {
        SeaRangeCache::default()
    }
}

impl Game {
    /// Drops the cached range fields (module comment): a city founded or gone, a plot's water or
    /// passability changed, players added or teams merged.
    pub fn sea_range_invalidate(&mut self) {
        self.sea_range.get_mut().fields.clear();
    }

    /// Whether `other`'s cities are ports of `team` (docs/NAVAL.md, section 2): its own team, its
    /// vassals and master, a team it has open borders with; never one at war with it.
    pub fn team_is_port_team(&self, team: TeamId, other: TeamId) -> bool {
        if other == team {
            return true;
        }
        if self.teams[team].at_war[other] || self.team_is_barbarian(other) {
            return false;
        }
        self.teams[team].vassal[other] || self.teams[other].vassal[team] || self.team_is_open_borders(team, other)
    }

    /// Whether city `c` is a port: a water plot is next to it.
    pub fn city_is_port(&self, c: CityId) -> bool {
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        (0..DirectionType::NUM).any(|i| self.plot_direction(x, y, DirectionType::from_usize(i)).is_some_and(|w| self.plot_is_water(w)))
    }

    /// Whether `team` has no range rule at all (module comment): the barbarians, or a team none
    /// of whose players has a city.
    pub fn team_sea_exempt(&self, team: TeamId) -> bool {
        self.team_is_barbarian(team) || !self.player_ids().any(|q| self.players[q].team == team && !self.players[q].cities.is_empty())
    }

    /// `team`'s range at sea: `SEA_RANGE` plus the `sea_range_change` of each tech it knows.
    pub fn team_sea_range(&self, team: TeamId) -> i32 {
        let mut cache = self.sea_range.borrow_mut();
        let techs = cache.range_techs.get_or_insert_with(|| {
            self.d.techs.iter_enumerated().filter(|(_, info)| info.sea_range_change != 0).map(|(t, info)| (t, info.sea_range_change)).collect()
        });
        self.d.defines.sea_range + techs.iter().filter(|&&(t, _)| self.teams[team].has_tech[t]).map(|&(_, c)| c).sum::<i32>()
    }

    /// A water plot the range field crosses: water, not impassable.
    fn sea_passable(&self, q: PlotIdx) -> bool {
        self.plot_is_water(q) && !self.plot_is_impassable(q)
    }

    /// Whether `f` was computed from what the game holds now for `team`.
    fn sea_field_fresh(&self, team: TeamId, f: &SeaField) -> bool {
        f.dist.len() == self.map.num_plots()
            && f.port_teams.len() == self.teams.len()
            && f.player_teams.len() == self.players.len()
            && self.team_ids().all(|t| f.port_teams[t.idx()] == self.team_is_port_team(team, t))
            && self.player_ids().all(|q| f.player_teams[q.idx()] == self.players[q].team)
    }

    /// Computes `team`'s range field (module comment): a breadth-first search over passable water
    /// from the water plots next to its ports, layer by layer, each plot keeping the lowest city
    /// id among the nearest ports.
    fn sea_field_compute(&self, team: TeamId) -> SeaField {
        let n = self.map.num_plots();
        let port_teams: Vec<bool> = self.team_ids().map(|t| self.team_is_port_team(team, t)).collect();
        let player_teams: Vec<TeamId> = self.player_ids().map(|q| self.players[q].team).collect();
        let mut dist = vec![NONE16; n];
        let mut port = vec![NONE32; n];
        let mut layer: Vec<PlotIdx> = Vec::new();
        for q in self.player_ids() {
            if !port_teams[self.players[q].team.idx()] {
                continue;
            }
            for &c in &self.players[q].cities {
                if !self.cities.contains(c) {
                    continue;
                }
                let (x, y) = (self.cities[c].x, self.cities[c].y);
                for i in 0..DirectionType::NUM {
                    let Some(w) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                    if !self.sea_passable(w) {
                        continue;
                    }
                    if dist[w.idx()] == NONE16 {
                        dist[w.idx()] = 1;
                        port[w.idx()] = c.0;
                        layer.push(w);
                    } else if c.0 < port[w.idx()] {
                        port[w.idx()] = c.0;
                    }
                }
            }
        }
        layer.sort();
        let mut d: u16 = 1;
        while !layer.is_empty() {
            let mut next: Vec<PlotIdx> = Vec::new();
            for &q in &layer {
                let (x, y) = self.map.xy(q);
                let from = port[q.idx()];
                for i in 0..DirectionType::NUM {
                    let Some(w) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                    if !self.sea_passable(w) {
                        continue;
                    }
                    if dist[w.idx()] == NONE16 {
                        dist[w.idx()] = d.saturating_add(1);
                        port[w.idx()] = from;
                        next.push(w);
                    } else if dist[w.idx()] == d.saturating_add(1) && from < port[w.idx()] {
                        port[w.idx()] = from;
                    }
                }
            }
            next.sort();
            layer = next;
            d = d.saturating_add(1);
        }
        SeaField { port_teams, player_teams, dist, port }
    }

    /// Runs `f` on `team`'s range field, from the cache when it is fresh.
    fn with_sea_field<R>(&self, team: TeamId, f: impl FnOnce(&SeaField) -> R) -> R {
        {
            let cache = self.sea_range.borrow();
            if let Some(field) = cache.fields.get(team.idx()).and_then(|o| o.as_ref()) {
                if self.sea_field_fresh(team, field) {
                    return f(field);
                }
            }
        }
        let field = self.sea_field_compute(team);
        let mut cache = self.sea_range.borrow_mut();
        if cache.fields.len() <= team.idx() {
            cache.fields.resize(team.idx() + 1, None);
        }
        cache.fields[team.idx()] = Some(field);
        f(cache.fields[team.idx()].as_ref().expect("just stored"))
    }

    /// The teams whose cached range field differs from a fresh computation (a check for tools:
    /// the cache must never change a game's course; empty when it holds).
    pub fn sea_range_cache_stale(&self) -> Vec<TeamId> {
        let cache = self.sea_range.borrow();
        let mut out = Vec::new();
        for (i, f) in cache.fields.iter().enumerate() {
            let Some(f) = f else { continue };
            let team = TeamId(i as u8);
            if i >= self.teams.len() || !self.sea_field_fresh(team, f) {
                continue;
            }
            let fresh = self.sea_field_compute(team);
            if fresh.dist != f.dist || fresh.port != f.port {
                out.push(team);
            }
        }
        out
    }

    /// The range field of `team` at `plot` (docs/NAVAL.md, 3.1): the fewest water steps from a
    /// water plot next to one of its ports (1 next to a port), `None` on land or where no port's
    /// water reaches. The truth, whoever asks.
    pub fn plot_sea_range(&self, team: TeamId, plot: PlotIdx) -> Option<i32> {
        if !self.plot_is_water(plot) {
            return None;
        }
        self.with_sea_field(team, |f| {
            let d = f.dist[plot.idx()];
            (d != NONE16).then_some(d as i32)
        })
    }

    /// The nearest port of `team` to water plot `plot` by the range field (ties: the lower city
    /// id), `None` where [`Game::plot_sea_range`] is `None`.
    pub fn plot_sea_port(&self, team: TeamId, plot: PlotIdx) -> Option<CityId> {
        if !self.plot_is_water(plot) {
            return None;
        }
        self.with_sea_field(team, |f| {
            let c = f.port[plot.idx()];
            (c != NONE32).then_some(CityId(c))
        })
    }

    /// Whether `plot` is in `team`'s range: always on land and for an exempt team
    /// ([`Game::team_sea_exempt`]); on water, its range field is at most [`Game::team_sea_range`].
    pub fn plot_in_sea_range(&self, team: TeamId, plot: PlotIdx) -> bool {
        if !self.plot_is_water(plot) || self.team_sea_exempt(team) {
            return true;
        }
        self.plot_sea_range(team, plot).is_some_and(|d| d <= self.team_sea_range(team))
    }

    // ---------------------------------------------------------------------------------------
    // Boats (3.2)

    /// Whether land unit `u` may use boats at all: a land unit of a player, not an animal.
    pub fn unit_can_use_boats(&self, u: UnitId) -> bool {
        self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && !self.unit_is_barbarian(u) && !self.unit_is_animal(u)
    }

    /// Whether unit `u` on `plot` would be at sea as a land unit: a land unit, `plot` water it
    /// does not walk or fly over (Causeway, Ferries, flying, water walking). Carried or in boats.
    pub fn unit_needs_boats(&self, u: UnitId, plot: PlotIdx) -> bool {
        self.plot_is_water(plot)
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && !self.unit_can_move_all_terrain(u)
            && !self.unit_walks_on_water(u, plot)
    }

    /// Whether unit `u` would be in boats on `plot`: a unit that may use boats
    /// ([`Game::unit_can_use_boats`]), at sea as a land unit there ([`Game::unit_needs_boats`]) and
    /// not carried by a ship (its current state).
    pub fn unit_in_boats_at(&self, u: UnitId, plot: PlotIdx) -> bool {
        self.plot_is_water(plot) && !self.unit_is_cargo(u) && self.unit_needs_boats(u, plot) && self.unit_can_use_boats(u)
    }

    /// Whether unit `u` is in boats now (docs/NAVAL.md, 3.2): a land unit on a water plot, not
    /// carried by a ship.
    pub fn unit_is_in_boats(&self, u: UnitId) -> bool {
        self.units.contains(u) && self.unit_plot_opt(u).is_some_and(|p| self.unit_in_boats_at(u, p))
    }

    /// The water plots unit `u` enters a turn in boats: `EMBARKED_MOVES` plus its owner's
    /// civilization's `embarked_moves_change`, at least 1.
    pub fn unit_embarked_moves(&self, u: UnitId) -> i32 {
        let civ = self.player_get_civilization_type(self.units[u].owner).map_or(0, |c| self.d.civilizations[c].embarked_moves_change);
        (self.d.defines.embarked_moves + civ).max(1)
    }

    /// The moves a water plot costs unit `u` in boats: its full moves split
    /// [`Game::unit_embarked_moves`] ways (rounded up, at least 1).
    pub fn unit_embarked_step_cost(&self, u: UnitId) -> i32 {
        let n = self.unit_embarked_moves(u);
        let max = self.unit_max_moves(u).max(1);
        ((max + n - 1) / n).max(1)
    }

    /// Applies `change` to unit `u` with its sight taken off its plot before and put back after
    /// (as `unit_change_extra_visibility_range` does): for a change its sight range depends on.
    pub(crate) fn unit_resight(&mut self, u: UnitId, change: impl FnOnce(&mut Game)) {
        let plot = self.units.contains(u).then(|| self.unit_plot_opt(u)).flatten();
        let Some(plot) = plot else {
            change(self);
            return;
        };
        let team = self.unit_get_team(u);
        let before = self.unit_visibility_range(u);
        change(self);
        let after = self.unit_visibility_range(u);
        if before != after {
            self.plot_change_adjacent_sight(plot, team, before, false, Some(u), true);
            self.plot_change_adjacent_sight(plot, team, after, true, Some(u), true);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Attrition and wear (3.1, 3.4)

    /// Whether the range rule applies to unit `u` where it stands: a sea unit not carried, or a
    /// unit in boats, on a water plot; not an animal, not the barbarians', its team not exempt.
    pub fn unit_sea_range_applies(&self, u: UnitId) -> bool {
        let Some(plot) = self.unit_plot_opt(u) else { return false };
        if !self.plot_is_water(plot) || self.unit_is_animal(u) || self.unit_is_barbarian(u) || self.unit_is_cargo(u) {
            return false;
        }
        let sea = self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA;
        (sea || self.unit_in_boats_at(u, plot)) && !self.team_sea_exempt(self.unit_get_team(u))
    }

    /// Whether unit `u` stands out of its team's range (the rule applies to it and its plot is
    /// not in range): it takes attrition at its owner's turn start and does not heal.
    pub fn unit_is_out_of_sea_range(&self, u: UnitId) -> bool {
        self.unit_sea_range_applies(u) && !self.plot_in_sea_range(self.unit_get_team(u), self.unit_plot(u))
    }

    /// Whether unit `u` does not heal on `plot` (docs/NAVAL.md, 3.1 and 3.2): in boats there, or a
    /// sea unit (not carried, not an animal, not the barbarians') on water out of its team's range.
    pub fn unit_sea_no_heal_at(&self, u: UnitId, plot: PlotIdx) -> bool {
        if !self.plot_is_water(plot) {
            return false;
        }
        if self.unit_in_boats_at(u, plot) {
            return true;
        }
        self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA
            && !self.unit_is_cargo(u)
            && !self.unit_is_animal(u)
            && !self.unit_is_barbarian(u)
            && !self.plot_in_sea_range(self.unit_get_team(u), plot)
    }

    /// Whether unit `u` is a warship (docs/NAVAL.md, section 2): a sea unit with a strength, not
    /// an animal.
    pub fn unit_is_warship(&self, u: UnitId) -> bool {
        self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && self.unit_can_fight(u) && !self.unit_is_animal(u)
    }

    /// Unit `u`'s wear (docs/NAVAL.md, 3.4), in percent of strength lost: `SEA_WEAR_PERCENT` per
    /// turn start above `SEA_WEAR_FREE_TURNS`, at most `SEA_WEAR_MAX_PERCENT`; 0 for anything but
    /// a player's warship.
    pub fn unit_sea_wear_percent(&self, u: UnitId) -> i32 {
        let d = &self.d.defines;
        let over = self.units[u].sea_turns - d.sea_wear_free_turns;
        if over <= 0 || !self.unit_is_warship(u) || self.unit_is_barbarian(u) {
            return 0;
        }
        (over * d.sea_wear_percent).clamp(0, d.sea_wear_max_percent.max(0))
    }

    /// Whether `plot` is a port city of `team`'s (a city of one of its port teams): a warship
    /// that starts its turn there loses its wear.
    pub fn plot_is_home_port(&self, team: TeamId, plot: PlotIdx) -> bool {
        self.map.plots[plot].plot_city.is_some_and(|c| self.cities.contains(c) && self.team_is_port_team(team, self.city_team(c)))
    }

    /// The start of `p`'s turn, before its units heal (docs/NAVAL.md, 3.1 and 3.4): each warship's
    /// wear count (0 in a port city of its team, else +1; a message when wear starts), then each
    /// sea unit or unit in boats out of range loses `SEA_ATTRITION_PERCENT` of its full HP (a
    /// message on its first turn out of range, and when it is lost).
    pub fn player_update_sea(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        let team = self.players[p].team;
        let turn = self.game_turn();
        let d = self.d;
        let exempt = self.team_sea_exempt(team);
        for u in self.players[p].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            let Some(plot) = self.unit_plot_opt(u) else { continue };
            let at = self.map.xy(plot);
            let ut = self.units[u].unit_type;
            let name = move || d.text.name(&d.units[ut].base.description).to_string();
            // Wear (3.4).
            if self.unit_is_warship(u) {
                if self.plot_is_home_port(team, plot) {
                    self.units[u].sea_turns = 0;
                } else {
                    self.units[u].sea_turns += 1;
                    if self.units[u].sea_turns == d.defines.sea_wear_free_turns + 1 {
                        let (n, pct) = (name(), d.defines.sea_wear_percent);
                        self.log.msg_at(turn, Some(p), at, || format!("{n}: wear, -{pct}% strength a turn at sea. Enter a port to clear."));
                    }
                }
            }
            // Attrition (3.1).
            if exempt || !self.unit_sea_range_applies(u) {
                self.units[u].sea_out_of_range = false;
                continue;
            }
            if self.plot_in_sea_range(team, plot) {
                self.units[u].sea_out_of_range = false;
                continue;
            }
            let pct = d.defines.sea_attrition_percent;
            if !self.units[u].sea_out_of_range {
                let n = name();
                self.log.msg_at(turn, Some(p), at, || format!("{n}: out of range at sea, -{pct}% HP a turn."));
            }
            self.units[u].sea_out_of_range = true;
            let loss = self.unit_max_hit_points(u) * pct.max(0) / 100;
            if loss <= 0 {
                continue;
            }
            let n = name();
            self.tel_ctx_push("sea_attrition");
            self.unit_change_damage(u, loss, None);
            self.tel_ctx_pop();
            if !self.units.contains(u) || self.unit_is_dead(u) {
                self.log.msg_at(turn, Some(p), at, || format!("{n} lost at sea."));
            }
        }
    }
}

/// Whether a unit in boats may take `mission` (docs/NAVAL.md, 3.2: it moves, waits or sleeps):
/// move, move to a unit, skip, sleep, sentry. The rules' group check
/// (`Game::group_can_start_mission`) and a client's selection check both ask this.
pub fn mission_allowed_in_boats(mission: MissionType) -> bool {
    matches!(
        mission,
        MissionType::MISSION_MOVE_TO | MissionType::MISSION_MOVE_TO_UNIT | MissionType::MISSION_SKIP | MissionType::MISSION_SLEEP | MissionType::MISSION_SENTRY
    )
}

/// The empty cache a new or loaded game starts with.
pub(crate) fn new_cache() -> RefCell<SeaRangeCache> {
    RefCell::new(SeaRangeCache::new())
}

// -------------------------------------------------------------------------------------------
// Lanes (docs/NAVAL.md, section 5; phase 2)

/// A lane ([`Game::sea_lane`]): the path from its plot (first) to a water plot next to a home
/// port (last), the port, and the first sea along it, counted from its plot, that is closed to
/// the team with who holds it (the barbarian team for ships of hidden nationality a viewer cannot
/// name). A lane with `cut_in` is cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeaLane {
    pub path: Vec<PlotIdx>,
    pub port: CityId,
    pub cut_in: Option<(SeaId, TeamId)>,
}

impl SeaLane {
    /// Whether the lane is cut.
    pub fn is_cut(&self) -> bool {
        self.cut_in.is_some()
    }
}

impl Game {
    /// `team`'s lane from `from` (docs/NAVAL.md, section 5) with any port of the team as its home
    /// port (4.6): the shortest path over passable water from `from` (a water plot, or the water
    /// plots next to a land plot) to a water plot next to a port of the team, every plot in the
    /// team's range: down the range field (the field is the distance from those ports). Ties:
    /// fewer plots, then the lower plot index at the first difference. `None`: no path in range.
    /// The truth (the stored control).
    pub fn sea_lane(&self, team: TeamId, from: PlotIdx) -> Option<SeaLane> {
        let (path, port) = self.sea_lane_path(team, from)?;
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

    /// [`Game::sea_lane`] as team `viewer` can work it out: the same path, cut where the control
    /// the viewer can work out ([`Game::sea_closed_known`]: only ships it sees count) closes a sea.
    pub fn sea_lane_known(&self, team: TeamId, from: PlotIdx, viewer: TeamId) -> Option<SeaLane> {
        let (path, port) = self.sea_lane_path(team, from)?;
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

    /// The path and port of `team`'s lane from `from` ([`Game::sea_lane`]).
    fn sea_lane_path(&self, team: TeamId, from: PlotIdx) -> Option<(Vec<PlotIdx>, CityId)> {
        if self.team_sea_exempt(team) {
            return None;
        }
        let range = self.team_sea_range(team);
        let starts: Vec<PlotIdx> = if self.plot_is_water(from) {
            vec![from]
        } else {
            let (x, y) = self.map.xy(from);
            let mut v: Vec<PlotIdx> = (0..DirectionType::NUM).filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i))).filter(|&w| self.plot_is_water(w)).collect();
            v.sort();
            v
        };
        let mut best: Option<Vec<PlotIdx>> = None;
        for s in starts {
            let Some(d0) = self.plot_sea_range(team, s) else { continue };
            if d0 > range {
                continue;
            }
            let mut path = vec![s];
            let (mut cur, mut d) = (s, d0);
            while d > 1 {
                let (x, y) = self.map.xy(cur);
                let next = (0..DirectionType::NUM)
                    .filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i)))
                    .filter(|&w| self.plot_sea_range(team, w) == Some(d - 1))
                    .min();
                let Some(n) = next else { break };
                path.push(n);
                cur = n;
                d -= 1;
            }
            if d != 1 {
                continue;
            }
            let better = match &best {
                None => true,
                Some(b) => (path.len(), &path) < (b.len(), b),
            };
            if better {
                best = Some(path);
            }
        }
        let path = best?;
        let port = self.plot_sea_port(team, *path.last()?)?;
        Some((path, port))
    }
}
