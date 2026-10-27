//! Sidar. Into the Mist (`SPELL_INTO_THE_MIST`, world spell; for the AI only with 40+ units) gives
//! every unit `PROMOTION_HIDDEN`: invisible to other teams (except in our own cities) and +1 first
//! strike, until the unit fights or casts. Units trained later do not get it. The end-of-turn
//! spell choice cast it the first turn the gate opened, usually at peace, and the army lost it
//! against barbarians before any war. The AI holds it for a war: it casts it when it is at war
//! with a civilization or preparing one, so the army marches unseen (the enemy cannot attack or
//! count it) and opens with an extra first strike; with no war by then it casts it once it has
//! half again the gate's 40 units, for the protection of workers and settlers.
//!
//! **Wane** (FfH kept it from its AI, `bAllowAI` 0, `unflagged.rs`): a living unit of level 6 or
//! more becomes a Shade (at most 4), a great person (artist, engineer, merchant, scientist) that
//! joins a city as a super specialist. A strong player retires veterans the army does not need
//! while at peace: [`wane`] retires one a turn, while no war is on or prepared, from a unit in one
//! of our cities that its garrison can spare (heroes, units of a limited class and casters stay:
//! a caster's levels are its spheres), and the
//! Shade then goes where great people go (`units::great_person`).
//!
//! Measured (full games, seeds 1-40, Sidar in slot 0, against the generic play): win 5.0% ->
//! 7.5% (1 seed won only by the new side, 0 by the base), world share at the end 1333 -> 1407
//! (t 0.94), at turn 300 1531 -> 1529 (t -0.41: the spell mostly falls after turn 300).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::{world_spell, CivPlay};
use crate::view::View;
use crate::Ai;

pub struct Sidar;

const SPELL: &str = "SPELL_INTO_THE_MIST";

impl CivPlay for Sidar {
    fn spell_value(&self, g: &Game, _u: UnitId, spell: SpellType, value: i32) -> i32 {
        if world_spell::is(g, spell, SPELL) {
            return 0;
        }
        value
    }

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        wane(g, v);
        let p = v.p;
        if g.players[p].feat_accomplished[FeatType::FEAT_GLOBAL_SPELL] {
            return;
        }
        if world_spell::war_or_preparing(g, p) || g.players[p].num_units() >= 60 {
            world_spell::cast(g, p, SPELL);
        }
    }
}

/// Wane: one veteran (level 6+) a turn in one of our cities, at peace with no war prepared, that
/// the city's garrison can spare and that is no hero or unit of a limited class, retires as a
/// Shade, which is then handled as a great person (see the module doc).
fn wane(g: &mut Game, v: &View) {
    let d = g.d;
    let p = v.p;
    let Some(spell) = d.ty::<SpellType>("SPELL_WANE") else { return };
    if world_spell::war_or_preparing(g, p) {
        return;
    }
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.units[u].level < 6 || !g.unit_can_cast(u, spell, false) {
            continue;
        }
        let info = &d.units[g.units[u].unit_type];
        let limited = info.unit_class_type.is_some_and(|k| d.unit_classes[k].max_global_instances > 0 || d.unit_classes[k].max_player_instances > 0);
        let caster = d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT").is_some_and(|a| info.unit_combat_type == Some(a));
        if limited || caster || info.default_unit_ai_type == Some(UnitAIType::UNITAI_HERO) {
            continue;
        }
        let plot = g.unit_plot(u);
        let Some(c) = g.map.plots[plot].plot_city.filter(|&c| g.cities[c].owner == p) else { continue };
        if g.unit_can_defend(u, Some(plot)) {
            let others = g.map.plots[plot].units.iter().filter(|&&o| o != u && g.units[o].owner == p && g.unit_can_defend(o, Some(plot))).count() as i32;
            if others < crate::units::garrison_quota(g, v, c) {
                continue;
            }
        }
        let before: Vec<UnitId> = g.map.plots[plot].units.clone();
        crate::trace!(g, p, "{}: {} (level {}) wanes into a Shade", g.cities[c].name, info.base.type_, g.units[u].level);
        g.unit_cast(u, spell);
        // The Shade is a new unit (`CvUnit::convert`): a great person from now on.
        let shade = d.ty::<UnitClassType>("UNITCLASS_SHADE");
        let new: Vec<UnitId> = g.map.plots[plot].units.iter().copied().filter(|o| !before.contains(o)).collect();
        for s in new {
            if g.units[s].owner == p && d.units[g.units[s].unit_type].unit_class_type == shade {
                g.unit_ai_set_unit_ai_type(s, UnitAIType::UNITAI_SCIENTIST);
            }
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::tests::one_landmass_game_with;

    /// At peace, a level-6 Archer the garrison can spare wanes into a Shade, which then acts as
    /// a great person; a level-6 Archer the garrison needs stays.
    #[test]
    fn a_spare_veteran_wanes_into_a_shade() {
        let d = ffh_data::global();
        let mut g = one_landmass_game_with(7, &["CIVILIZATION_SIDAR", "CIVILIZATION_MALAKIM"]);
        let us = PlayerId(0);
        g.player_found(us, 4, 5);
        let archer = d.t::<UnitType>("UNIT_ARCHER");
        let vet = g.player_init_unit(us, archer, 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        g.unit_set_level(vet, 6);
        let shade = d.t::<UnitClassType>("UNITCLASS_SHADE");
        let shades = |g: &Game| g.players[us].units.iter().filter(|&&u| d.units[g.units[u].unit_type].unit_class_type == Some(shade)).count();
        // Alone in the city: the garrison needs it.
        let v = View::new(&g, us);
        wane(&mut g, &v);
        assert!(g.units.contains(vet) && shades(&g) == 0, "the only defender stays");
        for _ in 0..3 {
            g.player_init_unit(us, archer, 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let v = View::new(&g, us);
        wane(&mut g, &v);
        assert_eq!(shades(&g), 1, "the spare veteran became a Shade");
        let s = g.players[us].units.iter().copied().find(|&u| d.units[g.units[u].unit_type].unit_class_type == Some(shade)).unwrap();
        assert_eq!(g.units[s].ai_type, Some(UnitAIType::UNITAI_SCIENTIST));
    }
}
