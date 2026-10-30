//! Cultural channels (this game's own rules, docs/CULTURE.md; docs/AI.md "Cultural actions").
//!
//! STUB during the redesign to standing channels (2026-10-30): the AI sets no channel yet.
//!
//! Switch `CULTURE` (`FFH_AI_CULTURE_OFF`): off, a civilization channels nothing.

use ffh_core::game::Game;
use ffh_core::ids::*;

pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.culture_actions_on() || !crate::switches::CULTURE.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
}

/// The governors of a human player's governed cities set channels on their own
/// (docs/GOVERNORS.md); hostile ones only where the player allowed them.
pub fn govern(g: &mut Game, p: PlayerId) {
    if !g.culture_actions_on() || g.player_is_barbarian(p) {
        return;
    }
}

/// Whether `other`'s revolt pressure is close to taking one of `p`'s cities (a provocation:
/// `diplomacy::territory_check`).
pub(crate) fn flips_against(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let flip = 100 * g.d.defines.culture_revolt_flip;
    g.players[p].cities.iter().any(|&c| g.cities[c].culture_pressure[other] * 3 >= flip * 2)
}
