//! Luchuirp: Gifts of Nantosuelta leaves a Golden Hammer in each city, which can become a free
//! Engineer there (`SPELL_GIVE_HAMMER_TO_CRAFTSMAN`) or +1 strength on a unit that takes it
//! (`SPELL_TAKE_GOLDEN_HAMMER_UNIT`). FfH's end-of-turn spell choice valued the take higher, so
//! the garrison warriors took nearly every hammer; units in our own cities now leave it, and the
//! hammer gives itself.
//!
//! Tried, not kept (docs/AI_LOG.md): building the Sculptor's Studio (a specific building,
//! `PrereqBuilding`, that the generic production never builds) where it opens the Wood Golem or
//! the Gargoyle. With both golems, Wood Golems replaced the Dwarven Slingers and the Luchuirp
//! fell behind (world share at turn 300 -91); for the Gargoyle only, no gain (full games: win
//! 25% -> 20%, world share at the end -86).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;

pub struct Luchuirp;

impl CivPlay for Luchuirp {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`): Construction early (it counts
    /// as the military tier 2), then Elementalism and Sorcery once Trade is known.
    fn tech_goals(&self, g: &Game, p: PlayerId) -> super::TechGoals {
        let mut beelines = vec!["TECH_CONSTRUCTION"];
        if g.d.ty::<TechType>("TECH_TRADE").is_some_and(|t| g.team_is_has_tech(g.players[p].team, Some(t))) {
            beelines.extend(["TECH_ELEMENTALISM", "TECH_SORCERY"]);
        }
        super::TechGoals { early_military: Some("TECH_CONSTRUCTION"), beelines, ..Default::default() }
    }

    fn spell_value(&self, g: &Game, u: UnitId, spell: SpellType, value: i32) -> i32 {
        let here = g.unit_plot(u);
        let in_own_city = g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == g.units[u].owner);
        if in_own_city && g.d.ty::<SpellType>("SPELL_TAKE_GOLDEN_HAMMER_UNIT") == Some(spell) {
            return 0;
        }
        value
    }
}
