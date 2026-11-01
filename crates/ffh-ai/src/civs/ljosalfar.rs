//! Ljosalfar: FfH's research plan (`tech_goals`), nothing else yet. Their `MaintainFeatures` keeps the forest under every improvement they
//! build on their own land (`CvPlot::setImprovementType` with FfH's check), and workers only build
//! there, so they never clear forest.

use ffh_core::game::Game;
use ffh_core::ids::*;

use super::CivPlay;

pub struct Ljosalfar;

impl CivPlay for Ljosalfar {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { early_military: Some("TECH_HUNTING"), religion: Some("TECH_WAY_OF_THE_FORESTS"), beelines: vec!["TECH_BOWYERS"] }
    }
}
