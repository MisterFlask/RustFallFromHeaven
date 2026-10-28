//! Malakim: FfH's research beeline (`tech_goals`), nothing else kept. Tried (docs/AI_LOG.md): Religious Fervor held for a war (win 30% ->
//! 22.5%, share at the end -100), the Citadel of Light valued in frontier cities at war (never
//! buildable: the AI does not research Sorcery). Religious Fervor stays with the generic play,
//! cast the first turn the AI-only gate (5 cities with the state religion) opens.

use ffh_core::game::Game;
use ffh_core::ids::*;

use super::CivPlay;

pub struct Malakim;

impl CivPlay for Malakim {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_PRIESTHOOD"], ..Default::default() }
    }
}
