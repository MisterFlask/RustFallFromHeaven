//! Kuriotates (`CvCity::init`): the Sprawling trait limits them to 3 full cities on a standard
//! map (radius 3); past the limit a new city is a *settlement*: a city of radius 1 under every
//! other rule (changed from FFH2 2026-11-20: docs/PORTING.md). The ported C++ `AI_foundValue` already scores
//! settlement-sized sites (9 plots) for a sprawling player at its limit. What the AI does:
//! - [`Kuriotates::production`] builds one settler at a time for a settlement site on the city's
//!   landmass, from a full city of size 5+ (or at its happy cap) at peace;
//! - [`Kuriotates::site_value`]: at the limit, no settlement site within 4 plots of a full city.
//!   A plot is worked by the city nearest to it (`CvPlot::updateWorkingCity`), so a settlement
//!   inside a full city's radius-3 ring takes plots the full city could work (measured with FFH2's
//!   radius-2 settlements: full cities stayed smaller, population at turn 300 37 against 41
//!   without settlements, commerce 117 against 159); 5 plots out, a radius-1 settlement shares
//!   none;
//! - after losing a full city, the next city founded is a full one, but the generic settler step
//!   counts settlements towards the limit and stops: a settler for a full city then.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;
use crate::production::{best_unit, count_ai, Choice, Role};
use crate::Ai;

pub struct Kuriotates;

impl CivPlay for Kuriotates {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_STIRRUPS"], ..Default::default() }
    }

    fn production(&self, ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        full_city_settler(ai, g, c, threat).or_else(|| settlement_settler(ai, g, c, threat))
    }

    fn site_value(&self, g: &Game, p: PlayerId, q: PlotIdx, value: i32) -> i32 {
        let pl = &g.players[p];
        let full = pl.cities.len() as i32 - g.player_get_num_settlements(p);
        if pl.max_cities <= 0 || full < pl.max_cities {
            return value;
        }
        // A full city's radius (3) plus a settlement's (1): closer, they share plots.
        let (x, y) = g.map.xy(q);
        if pl.cities.iter().any(|&c| !g.cities[c].settlement && g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= 4) {
            return 0;
        }
        value
    }
}

/// Below the limit of full cities while settlements make up the rest (a full city was lost): the
/// next city founded is a full one (`CvCity::init` counts full cities), but the generic settler
/// step counts settlements too and stops. One settler at a time from a city of size 4+ (or at
/// its happy cap) with a site on its landmass.
fn full_city_settler(ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
    let p = g.cities[c].owner;
    let pl = &g.players[p];
    if pl.max_cities <= 0 || threat > 0 || g.cities[c].settlement || g.city_is_occupation(c) {
        return None;
    }
    let settlements = g.player_get_num_settlements(p);
    let full = pl.cities.len() as i32 - settlements;
    if settlements == 0 || full >= pl.max_cities || count_ai(g, p, UnitAIType::UNITAI_SETTLE) > 0 {
        return None;
    }
    if crate::production::financial_trouble(g, p) && crate::economy::spend_pct(g, p) < 20 {
        return None;
    }
    let happy_room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
    if g.cities[c].population < 4 && happy_room > 0 {
        return None;
    }
    let area = g.map.plots[g.city_plot(c)].area;
    if !crate::sites::sites(ai, g, p).iter().any(|&(q, _)| g.map.plots[q].area == area) {
        return None;
    }
    let u = best_unit(g, c, Role::Settler)?;
    if g.city_production_turns_left_unit(c, u, 0) > 25 {
        return None;
    }
    crate::trace!(g, p, "{}: settler for a full city ({} full of {})", g.cities[c].name, full, pl.max_cities);
    Some(Choice::Train(u, UnitAIType::UNITAI_SETTLE))
}

/// Kuriotates at their city limit: one settler at a time for a settlement site on this
/// city's landmass, from a city of size 5+ (or at its happy cap) at peace.
fn settlement_settler(ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
    let p = g.cities[c].owner;
    let pl = &g.players[p];
    if pl.max_cities <= 0 || threat > 0 || g.cities[c].settlement || g.city_is_occupation(c) {
        return None;
    }
    let full = pl.cities.len() as i32 - g.player_get_num_settlements(p);
    if full < pl.max_cities || count_ai(g, p, UnitAIType::UNITAI_SETTLE) > 0 {
        return None;
    }
    if crate::production::financial_trouble(g, p) {
        return None;
    }
    let team = pl.team;
    let barb = g.barbarian_team();
    if g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t)) {
        return None;
    }
    let happy_room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
    if g.cities[c].population < 5 && happy_room > 0 {
        return None;
    }
    let area = g.map.plots[g.city_plot(c)].area;
    if !crate::sites::sites(ai, g, p).iter().any(|&(q, _)| g.map.plots[q].area == area) {
        return None;
    }
    let u = best_unit(g, c, Role::Settler)?;
    if g.city_production_turns_left_unit(c, u, 0) > 20 {
        return None;
    }
    Some(Choice::Train(u, UnitAIType::UNITAI_SETTLE))
}
