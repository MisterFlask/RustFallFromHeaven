//! Illians: their ritual projects raise their unique units (the owner's rule: civilizations use
//! their unique mechanics, CLAUDE.md "AI changes"). Wilboman (Hero promotion, an attacker's default
//! AI) is trained as a hero (`production::role_ok`); as an attacker he never won on cost.
//!
//! - **Ritual projects** (`projects.jsonc`, effects in `ffh/events.rs`): The White Hand
//!   (Philosophy, 400 hammers: three Priests of Winter, strength 5 + 2 cold, Medic, Channeling
//!   II, Ice) and Stir from Slumber (Divine Essence, 900 hammers: Drifa, a dragon; the rules
//!   allow it once a player has been eliminated). The generic AI reaches projects only when no
//!   building has any value left, so The White Hand came very late or never, and it never
//!   builds Stir from Slumber. Our most productive city builds them as soon as it can (not while
//!   threatened or broke).
//! - **Frozen Passage** (world spell, replacing FFH2's Stasis: the water within 2 plots of a
//!   target becomes Sea Ice, `ffh_core::ffh::world_ritual_spells::illians`) is cast the turn a
//!   cast is pending ([`frozen_passage`]): toward the nearest city of another civilization on
//!   another landmass than our capital's (a passage for our armies), else by the capital. Ad hoc
//!   (a rules change, CLAUDE.md "AI changes").
//!
//! First tried in round 5 (commit 76c1ffd; docs/AI_LOG.md): development seeds 1-40 win 12.5% ->
//! 17.5% (2/0), share at the end +269 (t 1.90); fresh seeds 5001-5064 win 15.6% -> 14.1% (2/3),
//! share at the end -40 (t -0.56): The White Hand came at turns 325-372, when Philosophy came.
//! Back since 2026-10-20 under the owner's rule.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;
use crate::production::Choice;
use crate::view::View;
use crate::Ai;
use ffh_core::ffh::world_ritual_spells::illians as ice;
use ffh_core::structs::SpellTarget;

pub struct Illians;

impl CivPlay for Illians {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }

    fn production(&self, _ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        if threat > 0 {
            return None;
        }
        ritual(g, c)
    }

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        frozen_passage(g, v.p);
    }
}

/// Frozen Passage's target for player `p` among `targets`: the one closest to the nearest city of
/// another civilization (one we have seen) on another landmass than our capital's, else the one
/// closest to our capital (ties: the lower plot).
pub fn frozen_passage_target(g: &Game, p: PlayerId, targets: &[PlotIdx]) -> Option<PlotIdx> {
    let capital = g.players[p].capital_city.filter(|&c| g.cities.contains(c))?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let home = g.map.plots[g.city_plot(capital)].area;
    let team = g.players[p].team;
    let barbarian = g.barbarian_player();
    let rival = g
        .cities
        .ids()
        .into_iter()
        .filter(|&c| {
            let o = g.cities[c].owner;
            let at = g.city_plot(c);
            o != p && o != barbarian && g.players[o].team != team && g.map.plots[at].area != home && g.map.plots[at].revealed[team]
        })
        .min_by_key(|&c| (g.plot_distance(cx, cy, g.cities[c].x, g.cities[c].y), c));
    let (ax, ay) = rival.map_or((cx, cy), |c| (g.cities[c].x, g.cities[c].y));
    targets.iter().copied().min_by_key(|&t| {
        let (x, y) = g.map.xy(t);
        (g.plot_distance(x, y, ax, ay), t)
    })
}

/// Casts Frozen Passage when `p`'s cast is pending (module doc).
fn frozen_passage(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_FROZEN_PASSAGE") else { return };
    let Some(u) = g.players[p].units.iter().copied().find(|&u| g.units.contains(u) && g.unit_march(u).is_none() && g.unit_can_cast_untargeted(u, spell, false)) else {
        return;
    };
    let targets: Vec<PlotIdx> = g.unit_spell_targets(u, spell).into_iter().filter_map(|t| if let SpellTarget::Plot(q) = t { Some(q) } else { None }).collect();
    let Some(t) = frozen_passage_target(g, p, &targets) else { return };
    let (x, y) = g.map.xy(t);
    crate::trace!(g, p, "Frozen Passage at ({x}, {y}): {} plots", ice::frozen_passage_plots(g, t).len());
    g.tel_cast_reason(u, "world", 0);
    g.unit_cast_at(u, spell, SpellTarget::Plot(t));
}

/// The White Hand and Stir from Slumber in our most productive city as soon as it can build
/// them.
fn ritual(g: &Game, c: CityId) -> Option<Choice> {
    let p = g.cities[c].owner;
    let best = g.players[p].cities.iter().copied().max_by_key(|&o| (g.city_yield_rate(o, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(o)));
    if best != Some(c) || crate::production::financial_trouble(g, p) {
        return None;
    }
    for name in ["PROJECT_THE_WHITE_HAND", "PROJECT_STIR_FROM_SLUMBER"] {
        let Some(pr) = g.d.ty::<ProjectType>(name) else { continue };
        if g.city_can_create(c, pr, false, false) {
            return Some(Choice::Create(pr));
        }
    }
    None
}
