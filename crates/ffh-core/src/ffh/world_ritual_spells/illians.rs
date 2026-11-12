//! The Illians' world spell, Frozen Passage, and the Sea Ice it leaves (the owner's decision,
//! 2026-11-11: it replaces FFH2's Stasis; docs/PORTING.md, "Changes from FFH2").
//!
//! - **Frozen Passage** (`SPELL_FROZEN_PASSAGE`, a world spell spending a charge of
//!   `PROJECT_RITUAL_FROZEN_PASSAGE`): cast at a water plot the caster's team has revealed, within
//!   [`FROZEN_PASSAGE_RANGE`] plots of a plot its owner owns or of Sea Ice. Every water plot
//!   within [`FROZEN_PASSAGE_RADIUS`] plots of it becomes Sea Ice, for good.
//! - **Sea Ice** (`Plot::sea_ice`): a flat land plot of `TERRAIN_SNOW`. It loses its feature,
//!   improvement and route and keeps its resource. It yields nothing and no city works it
//!   (`plot_calculate_yield`, `plot_is_unworkable`); no city is founded on it
//!   (`player_can_found`), nothing is built on it (`plot_can_build`, `plot_is_unimprovable`), it
//!   takes no feature and its terrain does not change (the plot setters). Land units cross it at
//!   a cost of 1 (`plot_movement_cost`); ships do not enter it (it is land), but a ship the water
//!   froze under stays (`plot_is_valid_domain_for_location`) and leaves to the water next to it.
//! - **Icebreak** (`SPELL_ICEBREAK`): any ship, at an adjacent Sea Ice plot with no unit on it (or
//!   the Sea Ice it stands on, alone). The spell's `delay` holds the ship 2 turns
//!   (`Unit::delayed_spell_target` keeps the plot); then, if no other unit stands there, the plot
//!   is water again: Coast next to land that is not Sea Ice, else Ocean.
//!
//! The water and land change goes through `plot_set_plot_type_with` without erasing the plot
//! (its units and resource stay) and without the per-plot area update; the areas are recalculated
//! once per cast ([`Game::plot_freeze_sea_ice`], [`Game::plot_thaw_sea_ice`]).

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// How far from its caster's owner's plots, or from Sea Ice, Frozen Passage's target may be.
pub const FROZEN_PASSAGE_RANGE: i32 = 5;
/// The radius around its target that Frozen Passage freezes.
pub const FROZEN_PASSAGE_RADIUS: i32 = 2;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellFrozenPassage" => spell_frozen_passage,
        "spellIcebreak" => spell_icebreak,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions (none: the targets gate both spells).
pub fn find_req(_name: &str) -> Option<ReqFn> {
    None
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsFrozenPassage" => targets_frozen_passage,
        "targetsIcebreak" => targets_icebreak,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions (none).
pub fn find_target_label(_name: &str) -> Option<TargetLabelFn> {
    None
}

/// The plots within `range` plots (plot distance) of plot `t`, each once, in plot order.
pub fn plots_within(g: &Game, t: PlotIdx, range: i32) -> Vec<PlotIdx> {
    let (x, y) = g.map.xy(t);
    let mut out = Vec::new();
    for dy in -range..=range {
        for dx in -range..=range {
            if let Some(q) = g.plot_xy(x, y, dx, dy) {
                let (qx, qy) = g.map.xy(q);
                if g.plot_distance(x, y, qx, qy) <= range {
                    out.push(q);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Does plot `q` anchor Frozen Passage for player `p`: owned by it, or Sea Ice?
fn anchors(g: &Game, p: PlayerId, q: PlotIdx) -> bool {
    let plot = &g.map.plots[q];
    plot.sea_ice || plot.owner == Some(p)
}

/// May `caster` cast Frozen Passage at `t`: water its team has revealed, within
/// [`FROZEN_PASSAGE_RANGE`] plots of a plot its owner owns or of Sea Ice? (The charge and the
/// civilization are the spell's own checks.)
pub fn frozen_passage_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    let p = g.units[caster].owner;
    g.plot_is_water(t) && g.map.plots[t].revealed[g.unit_get_team(caster)] && plots_within(g, t, FROZEN_PASSAGE_RANGE).into_iter().any(|q| anchors(g, p, q))
}

/// `targetsFrozenPassage(pCaster)`: every valid target, in plot order (each anchor marks the
/// plots in range of it once).
fn targets_frozen_passage(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let p = g.units[caster].owner;
    let team = g.unit_get_team(caster);
    let mut near = vec![false; g.map.num_plots()];
    for q in g.map.plot_ids() {
        if anchors(g, p, q) {
            for t in plots_within(g, q, FROZEN_PASSAGE_RANGE) {
                near[t.idx()] = true;
            }
        }
    }
    g.map.plot_ids().filter(|&t| near[t.idx()] && g.plot_is_water(t) && g.map.plots[t].revealed[team]).map(SpellTarget::Plot).collect()
}

/// The water plots Frozen Passage at `t` freezes: within [`FROZEN_PASSAGE_RADIUS`] plots.
pub fn frozen_passage_plots(g: &Game, t: PlotIdx) -> Vec<PlotIdx> {
    plots_within(g, t, FROZEN_PASSAGE_RADIUS).into_iter().filter(|&q| g.plot_is_water(q)).collect()
}

/// `spellFrozenPassage(pCaster)`.
fn spell_frozen_passage(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !frozen_passage_ok(g, caster, t) {
        return;
    }
    let plots = frozen_passage_plots(g, t);
    g.plot_freeze_sea_ice(&plots);
    let turn = g.game_turn();
    let (x, y) = g.map.xy(t);
    let n = plots.len();
    // Every player is told (it reshapes the map).
    g.log.msg_at(turn, None, (x, y), || format!("Frozen Passage: {n} water plots within {FROZEN_PASSAGE_RADIUS} plots of ({x}, {y}) become Sea Ice"));
}

/// The Icebreak targets of unit `u` standing on plot `at`: the adjacent Sea Ice plots with no
/// unit on them, and `at` itself when it is Sea Ice with no other unit on it.
pub fn icebreak_targets_at(g: &Game, u: UnitId, at: PlotIdx) -> Vec<PlotIdx> {
    if g.unit_get_domain_type(u) != DomainType::DOMAIN_SEA {
        return Vec::new();
    }
    plots_within(g, at, 1)
        .into_iter()
        .filter(|&q| {
            let plot = &g.map.plots[q];
            plot.sea_ice && if q == at { plot.units.iter().all(|&o| o == u) } else { plot.units.is_empty() }
        })
        .collect()
}

/// `targetsIcebreak(pCaster)`.
fn targets_icebreak(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(at) = g.unit_plot_opt(caster) else { return Vec::new() };
    icebreak_targets_at(g, caster, at).into_iter().map(SpellTarget::Plot).collect()
}

/// `spellIcebreak(pCaster)`: cast at the end of its delay (`SpellInfo::delay`); the plot thaws if
/// it is still Sea Ice and no unit but the caster stands on it.
fn spell_icebreak(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !g.map.plots[t].sea_ice || g.map.plots[t].units.iter().any(|&o| o != caster) {
        return;
    }
    g.plot_thaw_sea_ice(t);
    let p = g.units.get(caster).map(|u| u.owner);
    let turn = g.game_turn();
    let (x, y) = g.map.xy(t);
    g.log.msg_at(turn, p, (x, y), || format!("Icebreak: ({x}, {y}) is water again"));
}

impl Game {
    /// Is plot `t` Sea Ice?
    #[inline]
    pub fn plot_is_sea_ice(&self, t: PlotIdx) -> bool {
        self.map.plots[t].sea_ice
    }

    /// The water plots among `plots` become Sea Ice (module doc): flat land of snow, without its
    /// feature, improvement or route, keeping its units and resource. The areas are recalculated
    /// once, and the cities that worked them place their citizens elsewhere.
    pub fn plot_freeze_sea_ice(&mut self, plots: &[PlotIdx]) {
        let snow = self.d.t::<TerrainType>("TERRAIN_SNOW");
        let mut cities: Vec<CityId> = Vec::new();
        let mut changed = false;
        for &t in plots {
            if !self.plot_is_water(t) {
                continue;
            }
            changed = true;
            self.plot_set_feature_type(t, None);
            self.plot_set_improvement_type(t, None);
            self.plot_set_route_type(t, None, true);
            // A temporary terrain would come back when its timer ends.
            self.map.plots[t].real_terrain_type = None;
            self.map.plots[t].temp_terrain_timer = 0;
            self.plot_set_plot_type_with(t, PlotType::PLOT_LAND, false, false);
            self.plot_set_terrain_type(t, snow, false);
            self.map.plots[t].sea_ice = true;
            self.plot_update_yield(t);
            if let Some(c) = self.map.plots[t].working_city.filter(|&c| self.cities.contains(c)) {
                if !cities.contains(&c) {
                    cities.push(c);
                }
            }
        }
        if !changed {
            return;
        }
        self.map_recalculate_areas();
        for c in cities {
            if self.cities.contains(c) {
                self.city_verify_working_plots(c);
            }
        }
    }

    /// Sea Ice plot `t` is water again: Coast when a plot next to it is land that is not Sea Ice,
    /// else Ocean. Its units stay (the caller checks that only a ship is there); the areas are
    /// recalculated.
    pub fn plot_thaw_sea_ice(&mut self, t: PlotIdx) {
        if !self.map.plots[t].sea_ice {
            return;
        }
        let d = self.d;
        let by_land = plots_within(self, t, 1).into_iter().any(|q| q != t && !self.plot_is_water(q) && !self.map.plots[q].sea_ice);
        let terrain = d.t::<TerrainType>(if by_land { "TERRAIN_COAST" } else { "TERRAIN_OCEAN" });
        self.map.plots[t].sea_ice = false;
        self.plot_set_plot_type_with(t, PlotType::PLOT_OCEAN, false, false);
        self.plot_set_terrain_type(t, terrain, false);
        self.plot_update_yield(t);
        self.map_recalculate_areas();
    }
}

#[cfg(test)]
mod tests;
