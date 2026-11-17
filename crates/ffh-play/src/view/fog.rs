//! Fog rules (design §4.4): what the human may learn. These are the only visibility predicates
//! of the crate: the views and queries ask a [`Viewer`] (P3).
//!
//! A [`Viewer`] answers for the human's team, or with the true state under debug reveal:
//! * plots: revealed (`Plot::revealed[team]`), visible (`plot_is_visible`); a revealed plot
//!   shows its current terrain and feature, the bonus only when the team's techs reveal it
//!   (`plot_get_bonus_type(p, team)`), and the improvement, route and owner the team last saw
//!   (`revealed_improvement_type`, `revealed_route_type`, `revealed_owner`);
//! * units: own units always; others only on visible plots and when not invisible to the team
//!   (`unit_is_invisible(u, team, true)`: C++'s default `bCheckCargo`, so a foreign transport's
//!   cargo is hidden), with their owner as `unit_get_visual_owner` shows it (hidden nationality);
//! * cities: own cities; foreign cities the team has revealed (`city_is_revealed`); their size
//!   and defence only while the city's plot is visible;
//! * players: the team's own players and the players of teams it has met.
//!
//! The log is not filtered here: ffh-core records who receives each entry where it logs it (the
//! C++ condition of its `addMessage`, evaluated then: `LogEntry::audience`), and the session
//! passes the human the entries that reach it (`LogEntry::reaches`).

use ffh_core::ids::*;
use ffh_data::*;

use crate::view::Viewer;

impl Viewer<'_> {
    /// The plot is on the human's map (revealed).
    pub fn plot_revealed(&self, p: PlotIdx) -> bool {
        self.reveal || self.g.map.plots[p].revealed[self.team]
    }

    /// The human's team sees the plot now.
    pub fn plot_visible(&self, p: PlotIdx) -> bool {
        self.reveal || self.g.plot_is_visible(p, self.team)
    }

    /// The plot's bonus as the human sees it: on a revealed plot, when its techs reveal it.
    pub fn plot_bonus(&self, p: PlotIdx) -> Option<BonusType> {
        if self.reveal {
            return self.g.map.plots[p].bonus_type;
        }
        if !self.plot_revealed(p) {
            return None;
        }
        self.g.plot_get_bonus_type(p, Some(self.team))
    }

    /// The improvement the human's team last saw on the plot.
    pub fn plot_improvement(&self, p: PlotIdx) -> Option<ImprovementType> {
        let pl = &self.g.map.plots[p];
        if self.reveal {
            pl.improvement_type
        } else {
            pl.revealed_improvement_type[self.team]
        }
    }

    /// The route the human's team last saw on the plot.
    pub fn plot_route(&self, p: PlotIdx) -> Option<RouteType> {
        let pl = &self.g.map.plots[p];
        if self.reveal {
            pl.route_type
        } else {
            pl.revealed_route_type[self.team]
        }
    }

    /// The owner the human's team last saw for the plot.
    pub fn plot_owner(&self, p: PlotIdx) -> Option<PlayerId> {
        let pl = &self.g.map.plots[p];
        if self.reveal {
            pl.owner
        } else {
            pl.revealed_owner[self.team]
        }
    }

    /// The human sees unit `u` (C++: on a visible plot and `!isInvisible(team)`).
    pub fn unit_visible(&self, u: UnitId) -> bool {
        let g = self.g;
        let Some(unit) = g.units.get(u) else { return false };
        if self.reveal || unit.owner == self.h {
            return true;
        }
        let Some(p) = g.map.plot_xy(unit.x, unit.y) else { return false };
        self.plot_visible(p) && !g.unit_is_invisible(u, self.team, true)
    }

    /// The owner of `u` as the human sees it: hidden nationality shows foreign units as
    /// barbarians (`getVisualOwner`).
    pub fn unit_owner(&self, u: UnitId) -> PlayerId {
        let g = self.g;
        if self.reveal || g.units[u].owner == self.h {
            return g.units[u].owner;
        }
        g.unit_get_visual_owner(u, Some(self.team))
    }

    /// The city is on the human's map.
    pub fn city_revealed(&self, c: CityId) -> bool {
        let g = self.g;
        g.cities.get(c).is_some_and(|city| self.reveal || city.owner == self.h || g.city_is_revealed(c, self.team, false))
    }

    /// The human sees the city now (its plot is visible).
    pub fn city_visible(&self, c: CityId) -> bool {
        let g = self.g;
        g.cities.get(c).is_some_and(|city| city.owner == self.h || self.plot_visible(g.city_plot(c)))
    }

    /// The human's team is `t` or has met it.
    pub fn team_met(&self, t: TeamId) -> bool {
        self.reveal || t == self.team || self.g.teams[self.team].has_met[t]
    }

    /// The human knows player `p` (its team met `p`'s).
    pub fn player_met(&self, p: PlayerId) -> bool {
        self.team_met(self.g.players[p].team)
    }
}
