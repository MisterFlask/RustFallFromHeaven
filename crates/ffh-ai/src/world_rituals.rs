//! The AI's world ritual (docs/CARDS.md, "World rituals"): the project whose completion brings the
//! civilization's world spell (an untargeted one takes effect at completion, so the ritual is
//! started when its moment holds: `world_spells::ready_for_ritual`; a targeted one is cast as soon
//! as it has a target). Once a turn, after the empire decisions, a player with no cast pending and
//! no ritual under way starts it in its city with the most production, when it has at least two cities (one if it may hold no more),
//! no city in peril, and that city completes it within [`MAX_TURNS`] turns. The AI never sets a
//! ritual under way aside (any project: every project is a ritual, ffh-core's `ffh::rituals`):
//! nothing it queues goes in front of one ([`locked`]), so only losing the city ends it. It never
//! tries a gold hurry on one (the rules refuse it); its population hurries are for urgent units
//! only (`empire::hurry`), as for any other build. A civilization's own condition ([`civ_gate`]) can hold a ritual back.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::view::View;

/// The most turns of its best city's production a world ritual may take for the AI to start it.
pub const MAX_TURNS: i32 = 20;

/// Is city `c` building a ritual (any project) with production in it (the AI puts nothing in
/// front of it: that would lose its production)? Not while enemy units stand within 2 plots of
/// the city: a defender is worth more than the ritual's progress.
pub fn locked(g: &Game, c: CityId) -> bool {
    if g.city_ritual_in_progress(c).is_none() {
        return false;
    }
    let city = &g.cities[c];
    let Some(plot) = g.map.plot_xy(city.x, city.y) else { return true };
    crate::production::local_threat(g, city.owner, plot, 2) == 0
}

/// Once a turn: start the world ritual where and when it is worth it (module doc).
pub fn turn(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(pr) = g.player_world_ritual(p) else { return };
    if g.player_world_ritual_pending(p, pr) || g.player_world_ritual_city(p, pr).is_some() {
        return;
    }
    if !civ_gate(g, p, pr) {
        return;
    }
    // An untargeted world spell takes effect at the ritual's completion (docs/CARDS.md): the
    // ritual starts when the spell's moment holds (`world_spells::ready`), not the cast.
    if g.world_ritual_spell(pr).is_some_and(|s| !g.world_spell_targeted(s) && !crate::world_spells::ready_for_ritual(g, p, s)) {
        return;
    }
    let n = g.players[p].num_cities();
    let needed = if g.players[p].max_cities == 1 { 1 } else { 2 };
    if n < needed || !v.peril.is_empty() {
        return;
    }
    let Some(c) = g.players[p].cities.iter().copied().max_by_key(|&c| (g.city_current_production_difference(c, false, false), std::cmp::Reverse(c))) else { return };
    if !g.city_can_create(c, pr, false, false) || g.city_production_turns_left_project(c, pr, 0) > MAX_TURNS {
        return;
    }
    // (An army's requisition or an empire order stays at the head of the queue.)
    if g.cities[c].order_queue.first().is_some_and(|o| o.requisition || o.empire) || locked(g, c) {
        return;
    }
    let d = g.d;
    crate::trace!(g, p, "world ritual {} in {}", d.projects[pr].base.type_, g.cities[c].name);
    g.tel_decision(p, "production", &d.projects[pr].base.type_, "world_ritual", 0, String::new);
    g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, false);
}

/// A civilization's own condition to start its ritual `pr`: Upgrade (the Luchuirp's: a stack of
/// Reinforced on each golem) only with 4 or more golems.
fn civ_gate(g: &Game, p: PlayerId, pr: ProjectType) -> bool {
    let d = g.d;
    match d.projects[pr].base.type_.as_str() {
        "PROJECT_RITUAL_UPGRADE" => {
            let golem = d.t::<PromotionType>("PROMOTION_GOLEM");
            g.players[p].units.iter().filter(|&&u| g.units.contains(u) && g.units[u].has_promotion[golem]).count() >= 4
        }
        _ => true,
    }
}
