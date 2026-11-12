//! The AI's Icebreak (the rules: `ffh_core::ffh::world_ritual_spells::illians`): once its units
//! have moved, a ship of ours that ends its turn idle (moves left, no orders) next to Sea Ice (or
//! on it) within [`ICEBREAK_RANGE`] plots of one of our cities breaks it, so the ice does not shut
//! our coast. Ad hoc (a rules change, CLAUDE.md "AI changes").

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

/// How far from one of our cities Sea Ice is broken.
pub const ICEBREAK_RANGE: i32 = 4;

/// The turn's Icebreaks of player `p`.
pub fn end_turn(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_ICEBREAK") else { return };
    // Nothing to break while the map has no Sea Ice (the usual case: one pass over the plots).
    if !g.map.plot_ids().any(|q| g.map.plots[q].sea_ice) {
        return;
    }
    let cities: Vec<(i32, i32)> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
    if cities.is_empty() {
        return;
    }
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.unit_get_domain_type(u) != DomainType::DOMAIN_SEA || !g.unit_can_move(u) || g.unit_is_cargo(u) {
            continue;
        }
        let busy = g.units[u].group.and_then(|grp| g.groups.get(grp)).is_some_and(|grp| !grp.mission_queue.is_empty());
        if busy || !g.unit_can_cast_untargeted(u, spell, false) {
            continue;
        }
        let near = |t: PlotIdx| {
            let (x, y) = g.map.xy(t);
            cities.iter().map(|&(cx, cy)| g.plot_distance(x, y, cx, cy)).min().is_some_and(|d| d <= ICEBREAK_RANGE)
        };
        let target = g.unit_spell_targets(u, spell).into_iter().find(|&t| matches!(t, SpellTarget::Plot(q) if near(q)));
        if let Some(t) = target {
            crate::trace!(g, p, "Icebreak at {:?}", t);
            g.tel_cast_reason(u, "icebreak", 0);
            g.unit_cast_at(u, spell, t);
        }
    }
}
