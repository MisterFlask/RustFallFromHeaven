//! Governed cities (this game's own rules, `GAMEOPTION_GOVERNORS`; docs/GOVERNORS.md; docs/AI.md
//! "Governors"): what a governor builds, and how the AI runs its own governed cities.
//!
//! **Production.** A governed city's production is the governor's, for the human's cities as
//! for the AI's: the same choice ([`decide`]), reading its governor's profile. In order: a
//! garrison the city's danger asks for (the city AI's garrison quota, plus the profile's
//! defenders); the first worker and workers to the empire's need; work boats; the best building
//! by the city AI's value per turn of production, weighted by the profile's flavors against the
//! building's; a missionary for a zealot whose religion misses our cities; then the process the
//! profile prefers. World wonders, rituals, palaces and settlers are not for governed cities (the
//! rules forbid them). An army's requisition (docs/ARMIES.md) goes ahead of the governor's choice:
//! the unit is [`requisition_unit`], the AI's `Controller::requisition_unit`.
//!
//! **The AI's governed cities.** Its direct slots go to its most productive cities (a swap when a
//! governed city makes half again as much as the weakest direct one, both past their cooldown); a
//! vacant post takes the best-matching candidate (the rules do it). Cultural channels are the
//! civilization's, not its governors' (culture.rs plans them in governed cities too).
//!
//! (The feed mode, in which the governed cities nearest the enemy fed the main army, is gone with
//! the rules' requisition: which cities the AI's armies requisition from is not decided yet.)
//!
//! Switch `GOVERNORS` (`FFH_AI_GOVERNORS_OFF`): off, the AI keeps its first cities direct.

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
    let profile = g.city_governor_profile(c).map(|k| &d.governor_profiles[k]);
    let weight = |name: &str| match (profile, d.ty::<FlavorType>(name)) {
        (Some(pr), Some(f)) => pr.flavor_value[f],
        _ => 0,
    };
    let (defenders, defense) = prod::garrison(g, p, plot);
    let threat = prod::local_threat(g, p, plot, 3);
    let held = prod::garrison_holds(g, p, plot, threat, defense);
    let safe = threat * 2 <= defense;

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

    // 2. Workers, while the land needs improving.
    let workers = prod::count_ai(g, p, UnitAIType::UNITAI_WORKER);
    if safe && prod::has_worker_builds(g, p) && city.population >= 2 && (workers == 0 || workers < prod::worker_target(g, p)) {
        if let Some(u) = prod::best_unit(g, c, Role::Worker) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_WORKER), "governor_worker", None));
        }
    }
    // Work boats for sea resources nobody improves yet.
    if safe && prod::sea_resources_unimproved(g, c) > 0 {
        let need: i32 = g.players[p].cities.iter().map(|&o| prod::sea_resources_unimproved(g, o)).sum();
        if prod::count_ai(g, p, UnitAIType::UNITAI_WORKER_SEA) < need.min(g.players[p].cities.len() as i32) {
            if let Some(u) = prod::best_unit(g, c, Role::SeaWorker) {
                return Some((Choice::Train(u, UnitAIType::UNITAI_WORKER_SEA), "governor_work_boat", None));
            }
        }
    }

    // 3. The best building for the profile.
    let flavors: Vec<FlavorType> = (0..d.reg.count::<FlavorType>()).map(FlavorType::from_usize).collect();
    let mut best: Option<(BuildingType, i64)> = None;
    for b in d.buildings.keys() {
        if !prod::building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
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

/// The unit city `c` trains for army `a`'s requisition (the AI's `Controller::requisition_unit`):
/// siege while it is less than a third of the army (its recruits marching to it and its
/// requisition orders included), else the best attacker against cities, else any attacker; only
/// units that serve in armies (a Ranger or an Assassin would stay in the city, and the army would
/// never fill).
pub fn requisition_unit(g: &Game, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
    let mut siege = 0;
    let mut total = 0;
    for u in g.army_units(a).into_iter().chain(g.army_marching(a)) {
        total += 1;
        if matches!(g.units[u].ai_type, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL)) {
            siege += 1;
        }
    }
    for (oc, i) in g.army_requisition_orders(a) {
        total += 1;
        if matches!(opt_from_i32::<UnitAIType>(g.cities[oc].order_queue[i].data2), Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL)) {
            siege += 1;
        }
    }
    let army_unit = |u: UnitType| g.unit_type_is_army_unit(u);
    if siege * 3 < total {
        if let Some(u) = prod::best_unit_where(g, c, Role::Siege, army_unit) {
            return Some((u, UnitAIType::UNITAI_ATTACK_CITY));
        }
    }
    prod::best_unit_where(g, c, Role::CityAttack, army_unit).or_else(|| prod::best_unit_where(g, c, Role::Attack, army_unit)).map(|u| (u, UnitAIType::UNITAI_ATTACK))
}

/// The AI's governed cities, once a turn: the direct slots on its most productive cities.
pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.governors_on() || !crate::switches::GOVERNORS.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    swap_direct(g, p);
}

/// A governed city making half again as much as the weakest directly ruled city other than the
/// capital takes its slot (both past their cooldown); a free slot goes to the most productive.
fn swap_direct(g: &mut Game, p: PlayerId) {
    let production = |g: &Game, c: CityId| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
    let governed: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.city_is_governed(c) && g.city_rule_change_ready(c)).collect();
    let Some(best) = governed.iter().copied().max_by_key(|&c| (production(g, c), std::cmp::Reverse(c))) else { return };
    if g.player_direct_cities(p).len() < g.governor_direct_slots() {
        let _ = g.player_set_direct_rule(p, best, None);
        return;
    }
    let weakest = g.player_direct_cities(p).into_iter().filter(|&c| !g.city_is_capital(c) && g.city_rule_change_ready(c)).min_by_key(|&c| (production(g, c), c));
    if let Some(w) = weakest {
        if production(g, best) * 2 > production(g, w) * 3 && production(g, best) >= 6 {
            if g.player_set_direct_rule(p, best, Some(w)).is_ok() {
                crate::trace!(g, p, "governors: {} ruled directly instead of {}", g.d.text.name(&g.cities[best].name), g.d.text.name(&g.cities[w].name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The AI's requisition unit for an army: siege while it is short of a third, then
    /// attackers; only army units.
    #[test]
    fn requisition_takes_siege_then_attackers() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        let p0 = PlayerId(0);
        let c = g.player_init_city(p0, 5, 5, true, true);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 8, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &units).unwrap();
        let (u, role) = requisition_unit(&g, c, a).expect("a unit");
        assert!(g.unit_type_is_army_unit(u));
        // No siege the city can train at the start: an attacker.
        if !g.unit_type_is_siege(u) {
            assert_eq!(role, UnitAIType::UNITAI_ATTACK);
        }
    }
}
