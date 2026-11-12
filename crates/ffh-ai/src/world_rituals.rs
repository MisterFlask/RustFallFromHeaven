//! The AI's world ritual (docs/CARDS.md, "World rituals"): the project whose completion leaves a
//! charge of the civilization's world spell (when to cast it: `world_spells.rs`). Once a turn,
//! after the empire decisions, a player with no charge and no ritual under way starts it in its
//! city with the most production, when it has at least two cities (one if it may hold no more),
//! no city in peril, and that city completes it within [`MAX_TURNS`] turns. The AI never sets a
//! ritual under way aside: nothing it queues goes in front of one ([`locked`]), so only losing the
//! city ends it. A civilization's own condition ([`civ_gate`]) can hold a ritual back.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::view::View;

/// The most turns of its best city's production a world ritual may take for the AI to start it.
pub const MAX_TURNS: i32 = 20;

/// Is city `c` building a world ritual with production in it (the AI puts nothing in front of
/// it: that would abandon it)?
pub fn locked(g: &Game, c: CityId) -> bool {
    g.city_world_ritual_in_progress(c).is_some()
}

/// Once a turn: start the world ritual where and when it is worth it (module doc).
pub fn turn(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(pr) = g.player_world_ritual(p) else { return };
    if g.player_world_ritual_charge_city(p, pr).is_some() || g.player_world_ritual_city(p, pr).is_some() {
        return;
    }
    if !civ_gate(g, p, pr) {
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
