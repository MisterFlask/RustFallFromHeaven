//! Spells FfH kept from its AI (`bAllowAI` 0: 57 of 267). The rules no longer refuse them to AI
//! players (docs/PORTING.md, "Changes from FFH2"); `casting::castable_spells` keeps them out of the
//! generic casting, and those we have logic for are cast by it alone (docs/AI_ONLY_RULES.md, D):
//! - Sacrifice Altar (a Demon's Altar city, a level 2+ unit: research for the current tech,
//!   10 + level squared) here: a unit we disband anyway is sacrificed first ([`retire`]);
//! - Arena Battle (the Balseraphs' Arena): `civs/balseraphs.rs`;
//! - Wane (the Sidar: a level 6+ unit becomes a Shade, a great person that joins a city):
//!   `civs/sidar.rs`;
//! - Kidnap (the Svartalfar: a recon unit in a rival city takes a settled great person; failing,
//!   war comes): `civs/svartalfar.rs`.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// A unit we are disbanding (finance, bankruptcy) is sacrificed on a Demon's Altar instead when
/// it stands in such a city of ours and can (switch `SACRIFICE`): it goes either way, the
/// sacrifice adds research. True if sacrificed (its death is delayed, as a scrap's is).
pub fn sacrifice(g: &mut Game, u: UnitId) -> bool {
    let p = g.units[u].owner;
    if !crate::switches::SACRIFICE.on(g, p) {
        return false;
    }
    let Some(spell) = g.d.ty::<SpellType>("SPELL_SACRIFICE_ALTAR") else { return false };
    if !g.unit_can_cast(u, spell, false) {
        return false;
    }
    crate::trace!(g, p, "sacrifices {} (level {}) on the Demon's Altar", g.d.units[g.units[u].unit_type].base.type_, g.units[u].level);
    g.unit_cast(u, spell);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game;

    /// A level-3 unit disbanded in a Demon's Altar city goes as a sacrifice: research for the
    /// current tech (10 + 3 x 3); elsewhere it is simply scrapped.
    #[test]
    fn a_disbanded_unit_is_sacrificed_on_the_altar() {
        let d = ffh_data::global();
        let mut g = one_landmass_game(11);
        let us = PlayerId(0);
        g.player_found(us, 4, 5);
        let c = g.players[us].cities[0];
        let tech = d.techs.keys().find(|&t| g.player_can_research(us, t, false)).unwrap();
        g.player_push_research(us, tech, true);
        let team = g.players[us].team;
        let axeman = d.t::<UnitType>("UNIT_AXEMAN");
        let u = g.player_init_unit(us, axeman, 4, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        g.unit_set_level(u, 3);
        assert!(!sacrifice(&mut g, u), "no altar: no sacrifice");
        g.city_set_num_real_building(c, d.t::<BuildingType>("BUILDING_DEMONS_ALTAR"), 1);
        let before = g.team_get_research_progress(team, Some(tech));
        assert!(sacrifice(&mut g, u), "sacrificed on the altar");
        assert_eq!(g.team_get_research_progress(team, Some(tech)) - before, 19);
    }
}
