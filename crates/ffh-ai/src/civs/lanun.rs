//! Lanun. Their Pirate Cove (+2 commerce on an empty coast plot, growing into a Pirate Harbor and
//! a Pirate Port, 3/2/6) is a build of their cities' work crews (`BUILD_PIRATE_COVE`,
//! docs/WORKS.md; FFH2: a Work Boat's spell), valued with the other builds
//! (`workers::choose_crew_build`: an improvement that grows counts its grown yields). Black Wind,
//! their hero ship, is trained in a coastal city (`production`'s hero step, `Role::SeaHero`) and
//! serves as a transport.
//!
//! Tried, not kept (docs/AI_LOG.md): a Siege Workshop where it opens the War Tortoise
//! (`PrereqBuilding`): 0.3 Tortoises a game, no effect.

use ffh_core::game::Game;
use ffh_core::ids::*;

use super::CivPlay;

pub struct Lanun;

impl CivPlay for Lanun {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }
}
