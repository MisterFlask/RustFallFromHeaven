//! Amurites: the arcane civilization (both leaders Arcane; the palace gives Fire, Body and
//! Metamagic mana; Wizards are Mages with a free Spellstaff; the research plan beelines Sorcery).
//! They keep casters so their Wizards come (a Wizard is an Adept upgraded at level 4):
//! [`super::unique_casters`], the owner's rule that civilizations use their unique units
//! (CLAUDE.md "AI changes").
//!
//! First tried in round 5 (commit 5c6fdbe; docs/AI_LOG.md), before casters cast in fights:
//! development seeds 1-40 win 5.0% -> 7.5% (2/1), share at the end +108 (t 1.06); fresh seeds
//! 5001-5064 win 4.7% -> 1.6% (0/2), share at the end -25 (t -0.31). Back since 2026-10-20 under
//! the owner's rule.
//!
//! Wellspring (their world spell, docs/CARDS.md): with its cast pending, placed on the valid plot nearest
//! the capital, for the mana type they own the fewest of ([`wellspring`]).

use ffh_core::game::Game;
use ffh_core::ids::*;

use ffh_core::ffh::world_ritual_spells::small::{wellspring_choice, wellspring_decode, wellspring_manas};
use ffh_core::structs::SpellTarget;
use ffh_data::SpellType;

use super::CivPlay;
use crate::production::Choice;
use crate::view::View;
use crate::Ai;

pub struct Amurites;

impl CivPlay for Amurites {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_BOWYERS", "TECH_SORCERY"], ..Default::default() }
    }

    fn production(&self, _ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        super::unique_casters(g, c, threat)
    }

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        wellspring(g, v.p);
    }
}

/// Wellspring, with its cast pending: the valid plot nearest the capital (ties: the lower plot), with
/// the mana type we own the fewest of (`player_get_num_available_bonuses`; ties in data order).
/// Placed by the player (`Game::player_place`): no unit casts it.
fn wellspring(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_WELLSPRING") else { return };
    let targets = g.player_place_targets(p, spell);
    let Some(cap) = g.players[p].capital_city.filter(|&c| g.cities.contains(c)) else { return };
    let (cx, cy) = (g.cities[cap].x, g.cities[cap].y);
    let plot = targets
        .iter()
        .filter_map(|&t| if let SpellTarget::Choice(ch) = t { wellspring_decode(g, ch).map(|(q, _)| q) } else { None })
        .min_by_key(|&q| {
            let (x, y) = g.map.xy(q);
            (g.plot_distance(cx, cy, x, y), q)
        });
    let Some(q) = plot else { return };
    let d = g.d;
    let Some(mana) = wellspring_manas(d).into_iter().map(|(b, _)| b).min_by_key(|&b| (g.player_get_num_available_bonuses(p, b), b)) else { return };
    let choice = SpellTarget::Choice(wellspring_choice(d, q, mana));
    if targets.contains(&choice) {
        crate::trace!(g, p, "world spell SPELL_WELLSPRING placed at {:?}", choice);
        g.player_place(p, spell, choice);
    }
}
