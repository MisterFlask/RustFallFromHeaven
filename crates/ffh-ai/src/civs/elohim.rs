//! Elohim: no civilization-specific play but FfH's research beeline (`tech_goals`). Corlindale (a hero of strength 0, Channeling III) is
//! trained as a hero and moves as a caster (`production::role_ok`, `units::role_of`); before, no
//! Elohim trained him.
//!
//! Tried and not kept (docs/AI_LOG.md): holding Sanctuary (30 turns in which no foreign unit can
//! enter our land; AI gate: two civ wars) until enemy units stand in our land or a city is in
//! peril. The gate opens in few games and the measured change was within noise, slightly
//! negative.

use ffh_core::game::Game;
use ffh_core::ids::*;

use super::CivPlay;

pub struct Elohim;

impl CivPlay for Elohim {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_PRIESTHOOD"], ..Default::default() }
    }
}
