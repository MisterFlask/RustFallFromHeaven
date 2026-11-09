//! Doviello. They keep casters, their Shamans (the Doviello Adept, no Mage Guild needed):
//! [`super::unique_casters`], the owner's rule that civilizations use their unique units (CLAUDE.md
//! "AI changes"). Their War Machine and Bison Riders come from the generic building-gated units
//! (`production::unit_building`); the War Machine (Hero promotion) is trained as a hero.
//!
//! Slave -> Beastman (`SPELL_UPGRADE_DOVIELLO_WARRIOR_SLAVE`; the rules' AI gate: at war): cast
//! by the generic casting, and the Beastmen fight (`units::fighters_off_worker_ai`: the cast keeps
//! the slave's AI type). (The Worker's cast went with the workers, docs/WORKS.md.)
//!
//! Tried and not kept (docs/AI_LOG.md): holding Wild Hunt (a wolf for every unit of ours) for the
//! strike, a city in peril or 20 turns of war, with the wolves made army units in the stacks; no
//! Worker -> Beastman conversion unless the worker is about to be captured; a gold reserve for the
//! upgrade spells; a Siege Workshop and Stables for the War Machine and the Bison Rider (never
//! reached in the measured games; generic now).

use ffh_core::game::Game;
use ffh_core::ids::*;

use super::CivPlay;
use crate::production::Choice;
use crate::Ai;

pub struct Doviello;

impl CivPlay for Doviello {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }

    fn production(&self, _ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        super::unique_casters(g, c, threat)
    }
}
