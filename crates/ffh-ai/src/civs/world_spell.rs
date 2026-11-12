//! The civilizations' world-spell helpers: whether a spell is a given one, war or its preparation,
//! and a targeted cast through one of the player's units ([`cast_at`]). (A world spell is no
//! longer held for a moment: an untargeted one takes effect at its ritual's completion, whose
//! start the AI times, `world_spells::ready_for_ritual`.)

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// True if `spell` is world spell `name`.
pub fn is(g: &Game, spell: SpellType, name: &str) -> bool {
    g.d.ty::<SpellType>(name) == Some(spell)
}

/// At war with a civilization (not the barbarians), or preparing a war.
pub fn war_or_preparing(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t))
        || crate::diplomacy::preparing_target(g, team).is_some()
}

/// Casts targeted spell `name` at `target` with one of the player's units that can (a city
/// defender first); true if cast.
pub fn cast_at(g: &mut Game, p: PlayerId, name: &str, target: ffh_core::structs::SpellTarget) -> bool {
    let Some(spell) = g.d.ty::<SpellType>(name) else { return false };
    let units: Vec<UnitId> = g.players[p].units.clone();
    let caster = units
        .iter()
        .copied()
        .filter(|&u| g.unit_march(u).is_none() && g.unit_can_cast_at(u, spell, target))
        .min_by_key(|&u| (g.units[u].ai_type != Some(UnitAIType::UNITAI_CITY_DEFENSE), u));
    let Some(u) = caster else { return false };
    crate::trace!(g, p, "world spell {} by {} at {:?}", name, g.d.units[g.units[u].unit_type].base.type_, target);
    g.tel_cast_reason(u, "world_spell", 0);
    g.unit_cast_at(u, spell, target);
    true
}

/// A unit of player `p` that can cast `name` now (its targets aside), for listing its targets.
pub fn caster(g: &Game, p: PlayerId, name: &str) -> Option<(UnitId, SpellType)> {
    let spell = g.d.ty::<SpellType>(name)?;
    let u = g.players[p].units.iter().copied().find(|&u| g.unit_march(u).is_none() && g.unit_can_cast_untargeted(u, spell, false))?;
    Some((u, spell))
}
