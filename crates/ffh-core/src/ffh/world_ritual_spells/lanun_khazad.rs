//! The Lanun's and the Khazad's world spells (docs/CARDS.md, "World rituals"; docs/PORTING.md,
//! "Changes from FFH2"):
//!
//! | Spell | Cast | Effect |
//! |---|---|---|
//! | Raging Seas (Lanun, world spell) | at a water plot the caster's team has revealed | a storm over the plots within 2 plots of it for 15 turns |
//! | Steer the Storm (Lanun, world action) | at a water plot within 2 plots of the storm's centre | the storm's centre moves there, once a turn |
//! | The Delve (Khazad, world spell, from a selection) | by a land unit on a hill in its borders, at another within 10 plots | a tunnel joins the two hills for good |
//! | Enter the Delve (any land unit) | on a tunnel's end, with all its moves | the unit moves to the other end and ends its turn |
//!
//! The storm is an area effect (`Game::area_effects`, `AreaEffectKind::RagingSeas`): at the start
//! of each of its owner's turns ([`Game::raging_seas_strike`], from `player_set_turn_active`) every
//! unit of another team in it, on water or on land next to water, takes 20 cold damage through
//! `unit_do_damage` (its resistances count); a land unit is not taken beyond 75% damage, a ship can
//! sink. One storm per player at a time. The tunnels are `Game::delves`; a tunnel's ends outlive
//! any change of the plots' owners, and anyone who meets Enter the Delve's conditions travels.
//!
//! As in the cards' batches, a spell cast at a distance has a predicate for one target (`*_ok`):
//! its `targets*` callback lists what passes it, and its result checks the target again.

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::{AreaEffect, AreaEffectKind, SpellTarget};
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellRagingSeas" => spell_raging_seas,
        "spellSteerTheStorm" => spell_steer_the_storm,
        "spellTheDelve" => spell_the_delve,
        "spellEnterTheDelve" => spell_enter_the_delve,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqRagingSeas" => req_raging_seas,
        "reqSteerTheStorm" => req_steer_the_storm,
        "reqTheDelve" => req_the_delve,
        "reqEnterTheDelve" => req_enter_the_delve,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsRagingSeas" => targets_raging_seas,
        "targetsSteerTheStorm" => targets_steer_the_storm,
        "targetsTheDelve" => targets_the_delve,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions (none: the plots' own names serve).
pub fn find_target_label(_name: &str) -> Option<TargetLabelFn> {
    None
}

// ---------------------------------------------------------------------------------------------
// Numbers.

/// Raging Seas: the radius of the storm around its centre.
pub const STORM_RADIUS: i32 = 2;
/// Raging Seas: the turns the storm holds, the turn of the cast included.
pub const STORM_TURNS: i32 = 15;
/// Raging Seas: the cold damage each unit in the storm takes at the start of its owner's turns.
pub const STORM_DAMAGE: i32 = 20;
/// Raging Seas: the most damage, in percent of its hit points, the storm deals a unit that is not
/// of the sea domain.
pub const STORM_LAND_LIMIT_PERCENT: i32 = 75;
/// Steer the Storm: how far the storm's centre moves at most.
pub const STEER_RANGE: i32 = 2;
/// The Delve: how far apart the two ends of a tunnel may be.
pub const DELVE_RANGE: i32 = 10;

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

fn distance(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let (ax, ay) = g.map.xy(a);
    let (bx, by) = g.map.xy(b);
    g.plot_distance(ax, ay, bx, by)
}

// ---------------------------------------------------------------------------------------------
// Raging Seas and Steer the Storm.

impl Game {
    /// The index in `area_effects` of player `p`'s storm (Raging Seas) that holds this turn.
    pub fn player_storm(&self, p: PlayerId) -> Option<usize> {
        let turn = self.game_turn();
        self.area_effects.iter().position(|e| e.kind == AreaEffectKind::RagingSeas && e.owner == p && e.until > turn)
    }

    /// Does a storm strike the units on plot `t` (water, or land next to water)?
    pub fn plot_storm_strikes(&self, t: PlotIdx) -> bool {
        self.plot_is_water(t) || self.plot_is_adjacent_to_water(t)
    }

    /// Raging Seas, at the start of player `p`'s turn: each unit of another team in its storm, on
    /// a plot the storm strikes, takes [`STORM_DAMAGE`] cold damage (`unit_do_damage`, with its
    /// resistances); a unit that is not of the sea domain to at most
    /// [`STORM_LAND_LIMIT_PERCENT`]% damage.
    pub fn raging_seas_strike(&mut self, p: PlayerId) {
        let Some(i) = self.player_storm(p) else { return };
        let d = self.d;
        let storm = self.area_effects[i];
        let team = self.players[p].team;
        let cold = d.t::<DamageType>("DAMAGE_COLD");
        let mut struck = 0;
        for t in plots_within(self, storm.plot, storm.radius) {
            if !self.plot_storm_strikes(t) {
                continue;
            }
            for u in self.map.plots[t].units.clone() {
                if !self.units.contains(u) || self.unit_get_team(u) == team {
                    continue;
                }
                let max = self.unit_max_hit_points(u);
                let limit = if self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA { max } else { max * STORM_LAND_LIMIT_PERCENT / 100 };
                self.unit_do_damage(u, STORM_DAMAGE, limit, None, Some(cold), false);
                struck += 1;
            }
        }
        let turn = self.game_turn();
        let at = self.map.xy(storm.plot);
        self.log.msg_at(turn, Some(p), at, || format!("Raging Seas: {struck} units struck"));
    }
}

/// May `caster` raise a storm centred on `t` (a water plot its team has revealed; no storm of its
/// owner holds)?
pub fn raging_seas_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    g.plot_is_water(t) && g.map.plots[t].revealed[g.unit_get_team(caster)] && g.player_storm(g.units[caster].owner).is_none()
}

/// `reqRagingSeas(pCaster)`: no storm of the caster's owner holds.
fn req_raging_seas(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.player_storm(g.units[caster].owner).is_none()
}

/// `targetsRagingSeas(pCaster)`: the water plots the caster's team has revealed.
fn targets_raging_seas(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    if g.player_storm(g.units[caster].owner).is_some() {
        return Vec::new();
    }
    let team = g.unit_get_team(caster);
    g.map.plot_ids().filter(|&t| g.plot_is_water(t) && g.map.plots[t].revealed[team]).map(SpellTarget::Plot).collect()
}

/// `spellRagingSeas(pCaster)`: the storm, from this turn to the end of the 14th after it.
fn spell_raging_seas(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !raging_seas_ok(g, caster, t) {
        return;
    }
    let p = g.units[caster].owner;
    let turn = g.game_turn();
    g.area_effects.push(AreaEffect { kind: AreaEffectKind::RagingSeas, owner: p, plot: t, radius: STORM_RADIUS, until: turn + STORM_TURNS, moved: -1 });
    let (x, y) = g.map.xy(t);
    // Every player is told (a world spell).
    g.log.msg_at(turn, None, (x, y), || format!("Raging Seas: a storm within {STORM_RADIUS} plots of ({x}, {y}) for {STORM_TURNS} turns"));
}

/// May the owner of `caster` steer its storm to `t` (water its team has revealed, within
/// [`STEER_RANGE`] plots of the storm's centre, not the centre; not moved yet this turn)?
pub fn steer_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    let Some(i) = g.player_storm(g.units[caster].owner) else { return false };
    let storm = &g.area_effects[i];
    storm.moved != g.game_turn()
        && t != storm.plot
        && g.plot_is_water(t)
        && g.map.plots[t].revealed[g.unit_get_team(caster)]
        && distance(g, storm.plot, t) <= STEER_RANGE
}

/// `reqSteerTheStorm(pCaster)`: the owner's storm holds and has not moved this turn.
fn req_steer_the_storm(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.player_storm(g.units[caster].owner).is_some_and(|i| g.area_effects[i].moved != g.game_turn())
}

/// `targetsSteerTheStorm(pCaster)`: where the storm may move.
fn targets_steer_the_storm(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(i) = g.player_storm(g.units[caster].owner) else { return Vec::new() };
    plots_within(g, g.area_effects[i].plot, STEER_RANGE).into_iter().filter(|&t| steer_ok(g, caster, t)).map(SpellTarget::Plot).collect()
}

/// `spellSteerTheStorm(pCaster)`: the storm's centre moves to the target.
fn spell_steer_the_storm(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !steer_ok(g, caster, t) {
        return;
    }
    let p = g.units[caster].owner;
    let Some(i) = g.player_storm(p) else { return };
    let turn = g.game_turn();
    g.area_effects[i].plot = t;
    g.area_effects[i].moved = turn;
    let (x, y) = g.map.xy(t);
    g.log.msg_at(turn, Some(p), (x, y), || format!("Raging Seas: the storm moves to ({x}, {y})"));
}

// ---------------------------------------------------------------------------------------------
// The Delve and Enter the Delve.

impl Game {
    /// The other end of the tunnel with an end on plot `t`, if one.
    pub fn plot_delve_end(&self, t: PlotIdx) -> Option<PlotIdx> {
        self.delves.iter().find_map(|&(a, b)| if a == t { Some(b) } else if b == t { Some(a) } else { None })
    }
}

/// Is `t` a hill player `p` owns that is no tunnel's end (a possible end of a new tunnel)?
pub fn delve_hill_ok(g: &Game, p: PlayerId, t: PlotIdx) -> bool {
    g.plot_is_hills(t) && g.map.plots[t].owner == Some(p) && g.plot_delve_end(t).is_none()
}

/// Can `caster` dig a tunnel from where it stands (a land unit on a hill of its owner that is no
/// tunnel's end)?
pub fn delve_from_ok(g: &Game, caster: UnitId) -> bool {
    let Some(at) = g.unit_plot_opt(caster) else { return false };
    g.unit_get_domain_type(caster) == DomainType::DOMAIN_LAND && delve_hill_ok(g, g.units[caster].owner, at)
}

/// May `caster` dig a tunnel to `t` (another hill of its owner within [`DELVE_RANGE`] plots, no
/// tunnel's end)?
pub fn delve_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    if !delve_from_ok(g, caster) {
        return false;
    }
    let at = g.unit_plot(caster);
    t != at && delve_hill_ok(g, g.units[caster].owner, t) && distance(g, at, t) <= DELVE_RANGE
}

/// `reqTheDelve(pCaster)`.
fn req_the_delve(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    delve_from_ok(g, caster)
}

/// `targetsTheDelve(pCaster)`: the hills the tunnel may lead to, in plot order.
fn targets_the_delve(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(at) = g.unit_plot_opt(caster) else { return Vec::new() };
    if !delve_from_ok(g, caster) {
        return Vec::new();
    }
    plots_within(g, at, DELVE_RANGE).into_iter().filter(|&t| delve_ok(g, caster, t)).map(SpellTarget::Plot).collect()
}

/// `spellTheDelve(pCaster)`: the tunnel from the caster's plot to the target.
fn spell_the_delve(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !delve_ok(g, caster, t) {
        return;
    }
    let at = g.unit_plot(caster);
    g.delves.push((at, t));
    let p = g.units[caster].owner;
    let turn = g.game_turn();
    let (ax, ay) = g.map.xy(at);
    let (x, y) = g.map.xy(t);
    g.log.msg_at(turn, Some(p), (ax, ay), || format!("The Delve: a tunnel from ({ax}, {ay}) to ({x}, {y})"));
}

/// Where unit `u` would travel by Enter the Delve, if it may now: a land unit on a tunnel's end,
/// with all its moves, the other end holding no unit at war with it (either way), and a plot it
/// could enter without attacking (`unit_can_move_into`: open borders, an enemy city, ...).
pub fn enter_the_delve_to(g: &Game, u: UnitId) -> Option<PlotIdx> {
    let at = g.unit_plot_opt(u)?;
    if g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || g.units[u].moves != 0 {
        return None;
    }
    let to = g.plot_delve_end(at)?;
    let team = g.unit_get_team(u);
    let enemy = g.map.plots[to].units.iter().any(|&o| {
        let other = g.unit_get_team(o);
        other != team && (g.unit_is_enemy(o, Some(team), Some(to)) || g.unit_is_enemy(u, Some(other), Some(to)))
    });
    (!enemy && g.unit_can_move_into(u, to, false, false, true)).then_some(to)
}

/// `reqEnterTheDelve(pCaster)`.
fn req_enter_the_delve(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    enter_the_delve_to(g, caster).is_some()
}

/// `spellEnterTheDelve(pCaster)`: the unit moves to the tunnel's other end, out of its group, with
/// no moves left.
fn spell_enter_the_delve(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(to) = enter_the_delve_to(g, caster) else { return };
    let (x, y) = g.map.xy(to);
    g.unit_set_xy(caster, x, y, false);
    if g.units.contains(caster) {
        g.unit_finish_moves(caster);
    }
}

#[cfg(test)]
mod tests;
