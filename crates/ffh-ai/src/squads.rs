//! The AI's squads (docs/ENGAGEMENTS.md, "Squads"): every squad of an AI player stays Auto, and
//! a body (an army, a garrison, the loose units on a plot) whose squads could be fewer (units that
//! arrived formed squads of one) is auto-sorted, at the start of the player's turn (for its
//! attacks) and at its end (for the battles it defends before its next turn).

use ffh_core::game::Game;
use ffh_core::ids::PlayerId;

/// Settles player `p`'s squads and auto-sorts the bodies whose squads could be fewer.
pub fn turn(g: &mut Game, p: PlayerId) {
    g.player_army_sync(p);
    for list in g.squad_bodies_to_sort(p) {
        let _ = g.squad_auto_sort(p, &list);
    }
}
