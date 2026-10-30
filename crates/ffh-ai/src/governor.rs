//! Governed cities (this game's own rules, `GAMEOPTION_GOVERNORS`; docs/GOVERNORS.md; docs/AI.md
//! "Governors"): what a governor builds, and how the AI runs its own governed cities.
//!
//! **Production.** A governed city's production is the governor's, for the human's cities as
//! for the AI's: the same choice ([`decide`]), reading the city's mode and its governor's
//! profile. In order: a garrison the city's danger asks for (the city AI's garrison quota, plus
//! the profile's defenders); feeding its army, when it feeds one that is short of its target size
//! (the best unit for what the army lacks: a third in siege, the rest attackers; sent in
//! transit); the first worker and workers to the empire's need; work boats; the best building by
//! the city AI's value per turn of production, weighted by the profile's flavors against the
//! building's; a missionary for a zealot whose religion misses our cities; then the process the
//! profile prefers. World wonders, rituals, palaces and settlers are not for governed cities (the
//! rules forbid them).
//!
//! **The AI's governed cities.** Its direct slots go to its most productive cities (a swap when a
//! governed city makes half again as much as the weakest direct one, both past their cooldown);
//! at war or preparing one, the governed cities nearest the enemy feed its main army until it
//! has the army the war needs, the others develop; a vacant post takes the best-matching
//! candidate (the rules do it); its governors may queue hostile cultural actions (its cultural
//! play decides which are worth it).
//!
//! Switch `GOVERNORS` (`FFH_AI_GOVERNORS_OFF`): off, the AI keeps its first cities direct and every
//! governed city develops.

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

    // Against a revolt or a flip aimed at it, defenders up to three: each resists it (the AI's
    // cultural defense, culture.rs, re-chooses a threatened governed city's production).
    if culture_grave(g, c) && g.culture_garrison_count(c) < 3 {
        if let Some(u) = prod::best_unit(g, c, Role::Defense) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), "governor_culture_defense", None));
        }
    }

    // 2. Feeding its army.
    if let Some(a) = city.feed_army.filter(|&a| g.recruit_dest_valid(p, RecruitDest::Army(a))) {
        if army_short(g, a) {
            if let Some((u, role)) = feeding_unit(g, c, a) {
                return Some((Choice::Train(u, role), "governor_feed", Some(RecruitDest::Army(a))));
            }
        }
    }

    // 3. Workers, while the land needs improving.
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

    // 4. The best building for the profile.
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

    // 5. A zealot spreads the state religion to our cities without it.
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

    // 6. The process the profile prefers: research, gold or culture.
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

/// Whether a hostile Foment Revolt or Flip City is aimed at city `c`.
pub(crate) fn culture_grave(g: &Game, c: CityId) -> bool {
    g.culture_hostile_channels_at(c).iter().any(|ch| matches!(g.d.cultural_actions[ch.order.action].effect, CultureEffect::FomentRevolt | CultureEffect::FlipCity))
}

/// Whether army `a` is short of its target size, the units on their way and those its owner's
/// cities are training for it counted (no target: it always takes more): a feeding city starts a
/// unit for it only then, so several feeders do not overshoot.
pub fn army_short(g: &Game, a: ArmyId) -> bool {
    let target = g.armies[a].target_size;
    if target <= 0 {
        return true;
    }
    let owner = g.armies[a].owner;
    let queued = g.players[owner]
        .cities
        .iter()
        .flat_map(|&c| g.cities[c].order_queue.iter())
        .filter(|o| o.order_type == OrderType::ORDER_TRAIN && o.dest == Some(RecruitDest::Army(a)))
        .count() as i32;
    g.army_units(a).len() as i32 + g.army_transit_count(a) + queued < target
}

/// The unit governed city `c` trains for army `a`: siege while it is less than a third of the
/// army (on its way included), else the best attacker against cities, else any attacker; only
/// units that serve in armies (a Ranger or an Assassin would stay in the city, and the army would
/// never fill).
fn feeding_unit(g: &Game, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
    let owner = g.armies[a].owner;
    let mut siege = 0;
    let mut total = 0;
    for u in g.army_units(a) {
        total += 1;
        if matches!(g.units[u].ai_type, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL)) {
            siege += 1;
        }
    }
    for t in g.players[owner].transit.iter().filter(|t| t.dest == RecruitDest::Army(a)) {
        total += 1;
        if matches!(t.unit_ai, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL)) {
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

/// The AI's governed cities, once a turn: the direct slots on its most productive cities, the
/// feeding cities while it prepares or wages a war.
pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.governors_on() || !crate::switches::GOVERNORS.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    swap_direct(g, p);
    set_modes(g, p);
    // Its cultural play (culture.rs) weighs hostile actions itself: its governors may queue them.
    for c in g.players[p].cities.clone() {
        if g.city_is_governed(c) && !g.cities[c].governor_hostile_culture {
            let _ = g.city_set_hostile_culture(p, c, true);
        }
    }
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

/// At war or preparing one, with a main army: the governed cities nearest the enemy feed it
/// while it is short of its target (those on its landmass: a recruit travels by land); every
/// other governed city develops. The ARMIES switch off sends no unit in transit, so none feeds.
fn set_modes(g: &mut Game, p: PlayerId) {
    let team = g.players[p].team;
    let target = crate::diplomacy::preparing_target(g, team).or_else(|| g.team_ids().find(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)));
    let main = if g.armies_on() && crate::switches::ARMIES.on(g, p) { target.and_then(|t| crate::armies::main_army(g, p, t)) } else { None };
    let area = main.and_then(|a| g.army_plot(a)).and_then(|plot| g.map.plots[plot].area);
    let governed: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.city_is_governed(c)).collect();
    let reachable: Vec<CityId> = governed.iter().copied().filter(|&c| Some(g.city_area(c)) == area).collect();
    let feeders: Vec<CityId> = match (main, target) {
        // (The units in production not counted: a city training the last one keeps feeding.)
        (Some(a), Some(t)) if g.armies[a].target_size <= 0 || (g.army_units(a).len() as i32 + g.army_transit_count(a)) < g.armies[a].target_size => {
            // The half of our governed cities nearest the enemy's cities.
            let mut by_dist: Vec<(i32, CityId)> = reachable
                .iter()
                .map(|&c| {
                    let (x, y) = (g.cities[c].x, g.cities[c].y);
                    let dist = g
                        .player_ids()
                        .filter(|&q| g.players[q].alive && g.players[q].team == t)
                        .flat_map(|q| g.players[q].cities.iter().map(|&o| g.plot_distance(x, y, g.cities[o].x, g.cities[o].y)).collect::<Vec<_>>())
                        .min()
                        .unwrap_or(i32::MAX / 2);
                    (dist, c)
                })
                .collect();
            by_dist.sort();
            by_dist.into_iter().take(governed.len().div_ceil(2)).map(|(_, c)| c).collect()
        }
        _ => Vec::new(),
    };
    for c in governed {
        let want = if feeders.contains(&c) { main } else { None };
        if g.cities[c].feed_army != want {
            let _ = g.city_set_feed_army(p, c, want);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A governed city feeding an army trains for it while the army (with the units on their way)
    /// is short of its target size, and develops once it is not.
    #[test]
    fn feeding_stops_at_the_target_size() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        g.setup.options[GameOptionType::GAMEOPTION_GOVERNORS] = true;
        let p0 = PlayerId(0);
        let cities: Vec<CityId> = [3, 6, 9, 12, 15].iter().map(|&x| g.player_init_city(p0, x, 5, true, true)).collect();
        let fed = cities[4];
        assert!(g.city_is_governed(fed), "the fifth city is governed");
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        // A garrison that needs nothing more.
        for _ in 0..3 {
            g.player_init_unit(p0, w, 15, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 14, 7, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &units).unwrap();
        g.army_set_target_size(p0, a, 3).unwrap();
        g.city_set_feed_army(p0, fed, Some(a)).unwrap();
        // The mode takes effect at once: the city trains for the army, and that unit counts.
        let o = g.cities[fed].order_queue.first().expect("an order");
        assert_eq!((o.order_type, o.dest), (OrderType::ORDER_TRAIN, Some(RecruitDest::Army(a))));
        assert!(!army_short(&g, a), "two and one in production");
        g.city_clear_order_queue(fed);
        let (_, why, dest) = decide(&g, fed).expect("a choice");
        assert_eq!((why, dest), ("governor_feed", Some(RecruitDest::Army(a))));
        // One on its way: the army has its three.
        assert!(g.city_send_in_transit(cities[0], w, Some(UnitAIType::UNITAI_ATTACK), RecruitDest::Army(a)));
        assert!(!army_short(&g, a));
        let (_, why, _) = decide(&g, fed).expect("a choice");
        assert_ne!(why, "governor_feed", "the army is full: the city develops");
    }
}
