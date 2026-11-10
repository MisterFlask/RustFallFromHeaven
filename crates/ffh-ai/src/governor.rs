//! Governed cities (this game's own rules; docs/GOVERNORS.md; docs/AI.md
//! "Governors"): what a governor builds.
//!
//! **Production.** A governed city's production is the governor's, for the human's cities as
//! for the AI's: the same choice ([`decide`]), reading the city's role. In order: a garrison the
//! city's danger asks for (the city AI's garrison quota, plus the role's defenders); the best
//! building by the city AI's value per turn of production, weighted by the role's flavors against
//! the building's; a missionary for a role that weighs religion while its religion misses our
//! cities; then the process the role prefers. Wonders, palaces, rituals and settlers are the
//! player's empire orders, not the governor's choice. An army's requisition and the player's
//! empire orders (docs/ARMIES.md, docs/GOVERNORS.md) go ahead of the governor's choice, which
//! resumes after them (the rules do not ask the governor while they lead the queue).
//!
//! (Interim after governors v2, 2026-11-10: the AI places no empire orders yet, and the
//! traits are not read.)
//!
//! Cultural channels are the civilization's, not its governors' (culture.rs plans them in
//! governed cities too).

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::{self as prod, Choice, Role};

/// What a governed city builds now, why, and where a unit goes (`None`: its own garrison).
pub fn decide(g: &Game, c: CityId) -> Option<(Choice, &'static str, Option<RecruitDest>)> {
    let d = g.d;
    let city = &g.cities[c];
    let p = city.owner;
    let plot = g.city_plot(c);
    let profile = g.city_role(c).map(|k| &d.city_roles[k]);
    let weight = |name: &str| match (profile, d.ty::<FlavorType>(name)) {
        (Some(pr), Some(f)) => pr.flavor_value[f],
        _ => 0,
    };
    let (defenders, defense) = prod::garrison(g, p, plot);
    let threat = prod::local_threat(g, p, plot, 3);
    let held = prod::garrison_holds(g, p, plot, threat, defense);

    // 1. The garrison the city's danger asks for.
    let mut needed = 1 + i32::from(threat > 0 && !held) + crate::defense::rival_defenders(g, c) + profile.map_or(0, |pr| pr.defenders);
    if prod::barbarian_lair_near(g, p, plot, 3) || !held && prod::barbarians_near(g, p, plot, 3) {
        needed += 1;
    }
    if !crate::economy::can_afford_unit(g, p, true, prod::civ_enemies_at_war(g, p)) && threat == 0 {
        needed = needed.min(1);
    }
    let overwhelmed = threat > 0 && defense * 3 / 2 < threat && defenders < needed + 2;
    if defenders == 0 || overwhelmed || defenders < needed || prod::second_defender_wanted(g, p, c, plot, defenders, threat, defense) {
        if let Some(u) = prod::best_unit(g, c, Role::Defense) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), "governor_garrison", None));
        }
    }

    // Against revolt pressure that would bring disorder (and then take the city), the defenders
    // that hold it off: each resists it (`culture::defenders_needed`).
    if culture_grave(g, c) {
        if let Some(u) = prod::best_unit(g, c, Role::Defense) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), "governor_culture_defense", None));
        }
    }


    // 3. The best building for the profile.
    let flavors: Vec<FlavorType> = (0..d.reg.count::<FlavorType>()).map(FlavorType::from_usize).collect();
    let mut best: Option<(BuildingType, i64)> = None;
    for b in d.buildings.keys() {
        if !prod::building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        // Wonders and palaces are the player's empire orders (docs/GOVERNORS.md), not the
        // governor's choice.
        if g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i()) {
            continue;
        }
        let value = prod::building_value(g, c, b, threat);
        if value <= 0 {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        if turns > 50 {
            continue;
        }
        // The profile's lean: up to half again for a building all of whose flavor it weighs 5.
        let bf = &d.buildings[b].flavor_value;
        let total: i32 = flavors.iter().map(|&f| bf[f].max(0)).sum();
        let lean = match profile {
            Some(pr) if total > 0 => 10 * flavors.iter().map(|&f| pr.flavor_value[f] * bf[f].max(0)).sum::<i32>() / total,
            _ => 0,
        };
        let score = value as i64 * (100 + lean as i64) / (turns as i64 + 3);
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((b, score));
        }
    }
    if let Some((b, _)) = best {
        return Some((Choice::Construct(b), "governor_building", None));
    }

    // 4. A zealot spreads the state religion to our cities without it.
    if weight("FLAVOR_RELIGION") >= 3 {
        if let Some(r) = g.player_get_state_religion(p) {
            let missing = g.players[p].cities.iter().any(|&o| !g.cities[o].has_religion[r]);
            if missing && prod::count_ai(g, p, UnitAIType::UNITAI_MISSIONARY) == 0 {
                if let Some(u) = prod::best_missionary(g, c, r) {
                    return Some((Choice::Train(u, UnitAIType::UNITAI_MISSIONARY), "governor_missionary", None));
                }
            }
        }
    }

    // 5. The process the profile prefers: research, gold or culture.
    let (w_r, w_g, w_c) = (2 + weight("FLAVOR_SCIENCE"), 2 + weight("FLAVOR_GOLD"), 1 + weight("FLAVOR_CULTURE"));
    let mut best_process: Option<(ProcessType, i32)> = None;
    for pr in d.processes.keys() {
        if !g.city_can_maintain(c, pr, false) {
            continue;
        }
        let m = &d.processes[pr].production_to_commerce_modifier;
        let v = m[CommerceType::COMMERCE_RESEARCH.idx()] * w_r + m[CommerceType::COMMERCE_GOLD.idx()] * w_g + m[CommerceType::COMMERCE_CULTURE.idx()] * w_c;
        if best_process.map_or(true, |(_, s)| v > s) {
            best_process = Some((pr, v));
        }
    }
    best_process.map(|(pr, _)| (Choice::Maintain(pr), "governor_process", None))
}

/// Whether city `c` has fewer defenders than hold off the revolt pressure aimed at it
/// (`culture::defenders_needed`: the pressure, as it would settle, short of disorder; past it the
/// city makes no culture and the pressure runs on to the flip).
pub(crate) fn culture_grave(g: &Game, c: CityId) -> bool {
    g.culture_garrison_count(c) < crate::culture::defenders_needed(g, c)
}

/// The next `n` buildings city `c`'s governor means to build, in order (the client shows them as
/// the governor's plan; docs/GOVERNORS.md).
pub fn plan(g: &Game, c: CityId, n: usize) -> Vec<BuildingType> {
    let _ = (g, c, n);
    Vec::new()
}
