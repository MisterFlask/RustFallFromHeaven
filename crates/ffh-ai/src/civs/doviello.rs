//! Doviello. They keep casters, their Shamans (the Doviello Adept, no Mage Guild needed):
//! [`super::unique_casters`], the owner's rule that civilizations use their unique units (CLAUDE.md
//! "AI changes"). Their War Machine and Bison Riders come from the generic building-gated units
//! (`production::unit_building`); the War Machine (Hero promotion) is trained as a hero.
//!
//! Slave -> Beastman (`SPELL_UPGRADE_DOVIELLO_WARRIOR_SLAVE`; the rules' AI gate: at war): cast
//! by the generic casting, and the Beastmen fight (`units::fighters_off_worker_ai`: the cast keeps
//! the slave's AI type). (The Worker's cast went with the workers, docs/WORKS.md.)
//!
//! The Cull (their world spell, docs/CARDS.md): with a charge, cast on the plot with the most of
//! our units where the weaker half that dies are all of the lowest tier we can build or older,
//! leaving 2 or more units in a city; it waits for such a plot of 4 or more units ([`cull`]).
//!
//! Tried and not kept (docs/AI_LOG.md): holding Wild Hunt (FFH2's world spell, a wolf for every
//! unit of ours) for the strike, a city in peril or 20 turns of war, with the wolves made army
//! units in the stacks; no
//! Worker -> Beastman conversion unless the worker is about to be captured; a gold reserve for the
//! upgrade spells; a Siege Workshop and Stables for the War Machine and the Bison Rider (never
//! reached in the measured games; generic now).

use ffh_core::game::Game;
use ffh_core::ids::*;

use ffh_core::ffh::world_ritual_spells::small::cull_units;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use super::CivPlay;
use crate::production::Choice;
use crate::view::View;
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

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        cull(g, v.p);
    }
}

/// The fewest units on a plot for The Cull.
const CULL_MIN_UNITS: usize = 4;

/// The lowest tier among the land combat units player `p` can train now (`None`: none).
fn lowest_buildable_tier(g: &Game, p: PlayerId) -> Option<i32> {
    let d = g.d;
    d.units
        .keys()
        .filter(|&ut| {
            let i = &d.units[ut];
            i.combat > 0 && i.domain_type == Some(DomainType::DOMAIN_LAND) && i.production_cost > 0 && g.player_can_train(p, ut, false, false, false)
        })
        .map(|ut| d.units[ut].tier)
        .min()
}

/// The Cull, with a charge: the plot with the most of our units (ties: the lower plot) where the
/// weaker half are all of the lowest tier we can build or older, 4 or more units, and 2 or more
/// left in a city.
fn cull(g: &mut Game, p: PlayerId) {
    const SPELL: &str = "SPELL_THE_CULL";
    let Some((caster, spell)) = super::world_spell::caster(g, p, SPELL) else { return };
    let Some(tier) = lowest_buildable_tier(g, p) else { return };
    let d = g.d;
    let mut best: Option<(usize, PlotIdx)> = None;
    for t in g.unit_spell_targets(caster, spell) {
        let SpellTarget::Plot(q) = t else { continue };
        let units = cull_units(g, p, q);
        let n = units.len();
        let killed = &units[..n / 2];
        if n < CULL_MIN_UNITS || killed.iter().any(|&u| d.units[g.units[u].unit_type].tier > tier) {
            continue;
        }
        if g.map.plots[q].plot_city.is_some() && n - n / 2 < 2 {
            continue;
        }
        if best.map_or(true, |(bn, bq)| n > bn || (n == bn && q < bq)) {
            best = Some((n, q));
        }
    }
    if let Some((_, q)) = best {
        super::world_spell::cast_at(g, p, SPELL, SpellTarget::Plot(q));
    }
}
