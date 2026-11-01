//! Armies (this game's own rules, `GAMEOPTION_ARMIES`; docs/ARMIES.md; docs/AI.md "Armies").
//!
//! The AI's groups become armies by themselves (the rules' army sync names every group of army
//! units), so its movement logic drives each army unchanged: a city-attack stack is an army, a
//! city's defenders are its garrison. What an expert adds is logistics:
//!
//! - **Target size.** At war, or preparing one, the main field army (the largest army in supply,
//!   nearest the enemy on ties) is to be the army the war needs (`diplomacy::army_needed_for_war`).
//! - **Requisition** ([`requisition_cities`]). While the main army is short of that, it
//!   requisitions soldiers from the cities an expert would draw on: near the front (a unit that
//!   joins within [`REQUISITION_HORIZON`] turns, its training and its march by land), not a city
//!   that needs its production for its own defence, governed cities first (their production is
//!   the governor's development), a directly ruled city only when what it would interrupt is worth
//!   less ([`interruptible`]: not a wonder, a ritual, a settler or a worker, not the last turns of
//!   a building); as many cities as the army lacks units, the soonest to deliver first. A city
//!   whose order is under way keeps it while it can still serve. At peace, no requisition; through
//!   a turn without a main army (out of supply, at sea) while the war lasts, the orders under way
//!   are kept where the city can still spare them and no new ones placed. The unit is [`requisition_unit`] (the AI's
//!   `Controller::requisition_unit`): siege up to a third of the army, then the best attacker
//!   against cities.
//! - **Directly ruled cities' attackers.** The attack units the directly ruled cities train by
//!   their own choice (attack, city attack, collateral, counter roles) go to the main army too:
//!   they march to it (the rules move them). The rules count such an order at the head of a queue
//!   against the army's shortfall, and a city training one is not requisitioned on top of it, so
//!   the two do not count twice. Defenders and everything else stay in their city's garrison.
//! - **Defenders in the garrison.** A city defender that walked to another of our cities (an
//!   army of its own on the way) joins that city's garrison when it arrives.
//!
//! Marching recruits are the rules' to move, not the AI's (docs/ARMIES.md): `units::play` gives
//! them no orders, and the AI's stacks, escorts, transports, garrison counts and casts leave them
//! out (`military::is_army_unit`, `production::garrison`).
//!
//! Switch `ARMIES` (`FFH_AI_ARMIES_OFF`): off, every unit stays in its city's garrison when
//! trained and no army requisitions (the armies still form from the groups).

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::{self as prod, Role};

/// The unit roles that go to the field army.
const FIELD_ROLES: [UnitAIType; 4] = [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL, UnitAIType::UNITAI_COUNTER];

pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.armies_on() || !crate::switches::ARMIES.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    // A defender that walked into one of our cities to hold it serves in the city's garrison, not
    // as an army of its own (an expert's garrison: what holds the city, in one place).
    let mut garrisoned = false;
    for a in g.players[p].armies.clone() {
        let units = g.army_units(a);
        let in_city = g.army_plot(a).and_then(|plot| g.map.plots[plot].plot_city).is_some_and(|c| g.cities.contains(c) && g.cities[c].owner == p);
        let city = g.army_plot(a).and_then(|plot| g.map.plots[plot].plot_city);
        if in_city && g.armies[a].disband_to.is_none() && !units.is_empty() && units.iter().all(|&u| g.units[u].ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
            // What was on its way to the army goes to the garrison it joins.
            if let Some(c) = city {
                for u in g.army_marching(a) {
                    if let Some(m) = g.units[u].march.as_mut() {
                        m.dest = RecruitDest::Garrison(c);
                    }
                }
            }
            for u in units {
                g.units[u].army = None;
            }
            garrisoned = true;
        }
    }
    if garrisoned {
        g.player_army_sync(p);
    }
    let team = g.players[p].team;
    let target = crate::diplomacy::preparing_target(g, team).or_else(|| g.team_ids().find(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)));
    let main = target.and_then(|t| main_army(g, p, t));
    if let (Some(a), Some(t)) = (main, target) {
        let need = crate::diplomacy::army_needed_for_war(g, p, t);
        if g.armies[a].target_size != need {
            let _ = g.army_set_target_size(p, a, need);
        }
    }
    // Only the main army has a target size (no other army requisitions; at peace, none). Through
    // a turn without a main army while the war lasts (out of supply, at sea), the last one keeps
    // its target: it is the main one again when it is back in supply.
    if main.is_some() || target.is_none() {
        for a in g.players[p].armies.clone() {
            if Some(a) != main && g.armies[a].target_size != 0 && g.armies[a].disband_to.is_none() {
                let _ = g.army_set_target_size(p, a, 0);
            }
        }
    }
    for c in g.players[p].cities.clone() {
        for i in 0..g.cities[c].order_queue.len() {
            let o = &g.cities[c].order_queue[i];
            if o.order_type != OrderType::ORDER_TRAIN || o.dest.is_some() {
                continue;
            }
            let unit = UnitType::from_usize(o.data1 as usize);
            let field = opt_from_i32::<UnitAIType>(o.data2).is_some_and(|r| FIELD_ROLES.contains(&r));
            if !field || !g.unit_type_is_army_unit(unit) {
                continue;
            }
            if let Some(a) = main {
                g.cities[c].order_queue[i].dest = Some(RecruitDest::Army(a));
                crate::trace!(g, p, "armies: {} trains for the {}", g.d.text.name(&g.cities[c].name), g.armies[a].name);
            }
        }
    }
    if crate::switches::REQUISITION.on(g, p) {
        requisition(g, p, main, target.is_some());
    }
}

/// Turns within which the unit a city trains for the main army should join it (its production
/// turns there and its march by land): a city farther from the front than that is not drawn on
/// (its unit would come after the fighting it is for).
const REQUISITION_HORIZON: i32 = 20;

/// The armies' requisition lists, once a turn after their target sizes (see the module comment):
/// the main army draws on [`requisition_cities`]; at war without a main army this turn (out of
/// supply, at sea) every army keeps only the cities whose orders are under way and that can still
/// spare their production ([`can_spare`]: a threatened city trains its own defender); otherwise (at
/// peace, or another army is the main one) an army requisitions from none. A city taken off a
/// list loses its order at once; the rules place the new ones at the end of the turn
/// (`player_do_requisition`), before the cities produce.
fn requisition(g: &mut Game, p: PlayerId, main: Option<ArmyId>, war: bool) {
    for a in g.players[p].armies.clone() {
        if !g.armies.contains(a) || g.armies[a].disband_to.is_some() {
            continue;
        }
        let list: Vec<CityId> = if Some(a) == main {
            requisition_cities(g, p, a)
        } else if war && main.is_none() {
            let busy = ordered_cities(g, a);
            g.armies[a].requisition.iter().copied().filter(|&c| busy.contains(&c) && can_spare(g, p, c)).collect()
        } else {
            Vec::new()
        };
        if g.armies[a].requisition == list {
            continue;
        }
        if crate::trace_on(p) {
            let names: Vec<String> = list.iter().map(|&c| g.d.text.name(&g.cities[c].name).to_string()).collect();
            // The cities released with an order under way, and why.
            let released: Vec<String> = ordered_cities(g, a)
                .into_iter()
                .filter(|c| !list.contains(c))
                .map(|c| {
                    let why = if !can_spare(g, p, c) {
                        "needs its production"
                    } else if Some(a) != main {
                        "not the main army"
                    } else {
                        "?"
                    };
                    format!("{} ({why})", g.d.text.name(&g.cities[c].name))
                })
                .collect();
            crate::trace!(g, p, "armies: the {} (size {} of {}, short {}, {} marching) requisitions from [{}]; released [{}]", g.armies[a].name, g.army_units(a).len(), g.armies[a].target_size, g.army_shortfall(a), g.army_marching(a).len(), names.join(", "), released.join(", "));
        }
        let _ = g.army_set_requisition(p, a, &list);
    }
}

/// The cities with a requisition order for army `a` under way, in its owner's city order.
fn ordered_cities(g: &Game, a: ArmyId) -> Vec<CityId> {
    let mut out: Vec<CityId> = g.army_requisition_orders(a).into_iter().map(|(c, _)| c).collect();
    out.dedup();
    out
}

/// Whether city `c` of `p` can spare its production for an army: it holds against what threatens
/// it (a defender at least, and the defense `production::emergency` would not switch to a
/// defender for), it is not short of the defenders that keep revolt pressure off, and it is not
/// training a defender for itself.
fn can_spare(g: &Game, p: PlayerId, c: CityId) -> bool {
    let plot = g.city_plot(c);
    let (defenders, defense) = crate::production::garrison(g, p, plot);
    let threat = crate::production::local_threat(g, p, plot, 2);
    if defenders == 0 || defense * 3 / 2 < threat || crate::governor::culture_grave(g, c) {
        return false;
    }
    !prod::own_order(g, c).is_some_and(|o| {
        o.order_type == OrderType::ORDER_TRAIN
            && o.dest.is_none()
            && matches!(opt_from_i32::<UnitAIType>(o.data2), Some(UnitAIType::UNITAI_CITY_DEFENSE) | Some(UnitAIType::UNITAI_CITY_COUNTER))
    })
}

/// Whether directly ruled city `c` may be requisitioned for army `a`: what it would interrupt is
/// worth less than a soldier -- not a wonder (world, team or national), a project (rituals,
/// world projects), a settler, a worker, a limited unit (heroes) or a ship, not a building or
/// unit in its last 3 turns, and not a unit already bound for the army (it counts toward the
/// army's target as it is). Nothing queued, a process or a long build gives way.
fn interruptible(g: &Game, c: CityId, a: ArmyId) -> bool {
    let d = g.d;
    let Some((i, o)) = g.cities[c].order_queue.iter().enumerate().find(|(_, o)| !o.requisition) else { return true };
    let index = i as i32;
    match o.order_type {
        OrderType::ORDER_MAINTAIN => true,
        OrderType::ORDER_CONSTRUCT => {
            let b = BuildingType::from_usize(o.data1.max(0) as usize);
            let wonder = d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_limited_wonder_class(d, k));
            !wonder && g.city_production_turns_left_building(c, b, index) > 3
        }
        OrderType::ORDER_TRAIN => {
            let u = UnitType::from_usize(o.data1.max(0) as usize);
            let info = &d.units[u];
            let limited = info.unit_class_type.is_some_and(|k| ffh_core::utils::is_limited_unit_class(d, k));
            let civilian = info.found || info.builds.iter().any(|&b| b) || info.domain_type != Some(DomainType::DOMAIN_LAND);
            let for_army = g.city_recruit_dest(c, o.dest, u) == Some(RecruitDest::Army(a));
            !limited && !civilian && !for_army && g.city_production_turns_left_unit(c, u, index) > 3
        }
        _ => false,
    }
}

/// The cities the main army `a` of `p` requisitions from this turn: those whose order for it is
/// under way while they can still spare it, and while it is short, as many more as it lacks
/// units, the soonest to deliver first (training plus march, within [`REQUISITION_HORIZON`]; the
/// march guessed from the distance to rank them, then found along the land), governed cities
/// before directly ruled ones, which serve only when what they would interrupt gives way
/// ([`interruptible`]). Only cities on the army's landmass whose unit can reach it by land.
pub fn requisition_cities(g: &Game, p: PlayerId, a: ArmyId) -> Vec<CityId> {
    let Some(at) = g.army_plot(a) else { return Vec::new() };
    let area = g.map.plots[at].area;
    let busy = ordered_cities(g, a);
    // (In the order the army picked them.)
    let mut list: Vec<CityId> = g.armies[a].requisition.iter().copied().filter(|&c| busy.contains(&c) && can_spare(g, p, c)).collect();
    // The shortfall counts every order under way: those of the cities released here are withdrawn
    // with the new list, and their places go to others.
    let released = g.army_requisition_orders(a).iter().filter(|(c, _)| !list.contains(c)).count() as i32;
    let slots = (g.army_shortfall(a) + released).max(0) as usize;
    if slots == 0 {
        return list;
    }
    // The candidates by their training time and a quick guess at the march (two plots a move: a
    // road on most of the way), then the march along the land, nearest first, until the slots
    // are filled: the land path is searched for a few cities a turn, not for every one.
    let siege = wants_siege(g, a);
    let (ax, ay) = g.map.xy(at);
    let mut cands: Vec<(bool, i32, CityId, UnitType, i32)> = Vec::new();
    for &c in &g.players[p].cities {
        if busy.contains(&c) || g.map.plots[g.city_plot(c)].area != area || !can_spare(g, p, c) {
            continue;
        }
        let governed = g.city_is_governed(c);
        if !governed && !interruptible(g, c, a) {
            continue;
        }
        let Some((unit, _)) = unit_for(g, c, siege) else { continue };
        let production = g.city_production_turns_left_unit(c, unit, 0);
        let guess = g.plot_distance(g.cities[c].x, g.cities[c].y, ax, ay) / (2 * g.d.units[unit].moves.max(1));
        if production + guess <= REQUISITION_HORIZON {
            cands.push((!governed, production + guess, c, unit, production));
        }
    }
    cands.sort_by_key(|&(direct, guess, c, ..)| (direct, guess, c));
    let mut picked: Vec<(bool, i32, CityId)> = Vec::new();
    for (direct, _, c, unit, production) in cands {
        if picked.len() >= slots {
            break;
        }
        let Some(march) = g.march_turns(p, unit, g.city_plot(c), at) else { continue };
        if production + march <= REQUISITION_HORIZON {
            picked.push((direct, production + march, c));
            crate::trace!(g, p, "armies: the {} picks {} ({}, {} in {} turns, {} to march)", g.armies[a].name, g.d.text.name(&g.cities[c].name), if direct { "direct" } else { "governed" }, g.d.units[unit].base.type_, production, march);
        }
    }
    picked.sort();
    list.extend(picked.into_iter().map(|(_, _, c)| c));
    list
}

/// The unit city `c` trains for army `a`'s requisition (the AI's `Controller::requisition_unit`;
/// the governors' feeding choice before the requisition replaced it): siege while it is less than
/// a third of the army ([`wants_siege`]), else the best attacker against cities, else any
/// attacker; only units that serve in armies (a Ranger or an Assassin would stay in the city, and
/// the army would never fill).
pub fn requisition_unit(g: &Game, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
    unit_for(g, c, wants_siege(g, a))
}

/// Whether army `a` wants siege: less than a third of it is siege (city attackers and collateral
/// units by their AI role), its recruits marching to it and its requisition orders included.
fn wants_siege(g: &Game, a: ArmyId) -> bool {
    let siege_role = |r: Option<UnitAIType>| matches!(r, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL));
    let mut siege = 0;
    let mut total = 0;
    for u in g.army_units(a).into_iter().chain(g.army_marching(a)) {
        total += 1;
        siege += i32::from(siege_role(g.units[u].ai_type));
    }
    for (oc, i) in g.army_requisition_orders(a) {
        total += 1;
        siege += i32::from(siege_role(opt_from_i32::<UnitAIType>(g.cities[oc].order_queue[i].data2)));
    }
    siege * 3 < total
}

/// The army unit city `c` trains for a requisition: siege when `siege` (and it can), else the
/// best attacker against cities, else any attacker.
fn unit_for(g: &Game, c: CityId, siege: bool) -> Option<(UnitType, UnitAIType)> {
    let army_unit = |u: UnitType| g.unit_type_is_army_unit(u);
    if siege {
        if let Some(u) = prod::best_unit_where(g, c, Role::Siege, army_unit) {
            return Some((u, UnitAIType::UNITAI_ATTACK_CITY));
        }
    }
    prod::best_unit_where(g, c, Role::CityAttack, army_unit).or_else(|| prod::best_unit_where(g, c, Role::Attack, army_unit)).map(|u| (u, UnitAIType::UNITAI_ATTACK))
}

/// Our main field army against `target`: the largest army (at least two units) in supply and not
/// disbanding, the nearest to `target`'s cities on ties -- but the main army so far (the one with
/// a target size: [`turn`] gives it to the main army alone) stays the main one while it is at
/// least two thirds the size of the largest; out of supply, there is no main army this turn
/// (its reinforcements wait for it) unless an army in supply is as good a choice by that measure.
/// An expert keeps reinforcing the stack he fights with: switching whenever another stack was a
/// unit larger for a turn (the AI's stacks split for strikes and merge again) or the field army
/// crossed out of supply (a pair of units at home became the main army) withdrew a third of the
/// requisition orders placed in the first games.
pub(crate) fn main_army(g: &Game, p: PlayerId, target: TeamId) -> Option<ArmyId> {
    let mut best: Option<(i32, i32, ArmyId)> = None;
    let mut current: Option<(i32, bool, ArmyId)> = None;
    for &a in &g.players[p].armies {
        if !g.armies.contains(a) || g.armies[a].disband_to.is_some() {
            continue;
        }
        let size = g.army_units(a).len() as i32;
        if size < 2 {
            continue;
        }
        let supply = g.army_in_supply(a);
        if g.armies[a].target_size > 0 && current.is_none() {
            current = Some((size, supply, a));
        }
        if !supply {
            continue;
        }
        let Some(plot) = g.army_plot(a) else { continue };
        let (x, y) = g.map.xy(plot);
        let dist = g
            .player_ids()
            .filter(|&q| g.players[q].alive && g.players[q].team == target)
            .flat_map(|q| g.players[q].cities.iter().map(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y)).collect::<Vec<_>>())
            .min()
            .unwrap_or(i32::MAX / 2);
        let key = (size, -dist, a);
        if best.map_or(true, |(s, d, b)| key > (s, d, b)) {
            best = Some(key);
        }
    }
    let best = best.map(|(s, _, a)| (s, a));
    match (current, best) {
        (Some((n, true, c)), Some((size, _))) if n * 3 >= size * 2 => Some(c),
        (Some((n, false, _)), Some((size, a))) => (size * 3 >= n * 2).then_some(a),
        (Some((_, false, _)), None) => None,
        (_, best) => best.map(|(_, a)| a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At war, a city's attack unit goes to the main field army (it marches); a defender stays in
    /// the city's garrison, and the army's target size is the war's.
    #[test]
    fn at_war_attack_units_reinforce_the_main_army() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 4, 5, true, true);
        g.player_init_city(p1, 14, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        // Three plots out of the city (no roads: its plot is the network), so in supply.
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 7, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        let (attack, defend) = (UnitAIType::UNITAI_ATTACK.idx() as i32, UnitAIType::UNITAI_CITY_DEFENSE.idx() as i32);
        g.city_push_order(ours, OrderType::ORDER_TRAIN, w.idx() as i32, attack, false, false, true, false);
        g.city_push_order(ours, OrderType::ORDER_TRAIN, w.idx() as i32, defend, false, false, true, false);
        turn(&mut g, p0);
        let q = &g.cities[ours].order_queue;
        assert_eq!(q[0].dest, Some(RecruitDest::Army(a)), "the attacker to the army");
        assert_eq!(q[1].dest, None, "the defender stays home");
        assert_eq!(g.armies[a].target_size, crate::diplomacy::army_needed_for_war(&g, p0, t1));
    }

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

    /// Seven cities of player 0 west of player 1's one (the first four ruled directly, the other
    /// three governed), a defender in each, and a main army of two warriors between them; the
    /// cities' queues empty, and whatever they would train for the army one turn from done (so
    /// every city is near enough to the front).
    fn requisition_game() -> (Game, Vec<CityId>, ArmyId) {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        g.setup.options[GameOptionType::GAMEOPTION_GOVERNORS] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let cities: Vec<CityId> = [(3, 4), (3, 7), (6, 4), (6, 7), (9, 4), (9, 7), (12, 4)].iter().map(|&(x, y)| g.player_init_city(p0, x, y, true, true)).collect();
        g.player_init_city(p1, 16, 6, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        for &c in &cities {
            g.player_init_unit(p0, w, g.cities[c].x, g.cities[c].y, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
            g.city_clear_order_queue(c);
        }
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 13, 6, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        assert!(g.army_in_supply(a), "two plots from the nearest city");
        for &c in &cities {
            let (u, _) = requisition_unit(&g, c, a).expect("a unit to train");
            g.cities[c].unit_production[u] = g.city_production_needed_unit(c, u) - 1;
        }
        assert_eq!(cities.iter().map(|&c| g.city_is_governed(c)).collect::<Vec<_>>(), [false, false, false, false, true, true, true]);
        (g, cities, a)
    }

    /// At war, the main army requisitions from the cities an expert draws on: the governed ones
    /// first, then directly ruled ones whose build gives way, as many as it lacks units; not a
    /// city under threat, a settler, the last turns of a building, nor a city already training
    /// an attacker for the army (which counts toward its target). The rules then place the orders
    /// at the front of those queues.
    #[test]
    fn the_main_army_requisitions_from_the_right_cities_at_war() {
        let (mut g, c, a) = requisition_game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let d = g.d;
        // Directly ruled: a settler, a building in its last turns, an attacker of its own (to
        // the army: armies' FIELD_ROLES), a long building.
        g.city_set_population(c[0], 3);
        let settler = d.units.keys().find(|&u| d.units[u].found && g.city_can_train(c[0], u, false, false, false, false)).expect("a settler");
        g.city_push_order(c[0], OrderType::ORDER_TRAIN, settler.i(), UnitAIType::UNITAI_SETTLE.i(), false, false, true, false);
        let building = d.buildings.keys().find(|&b| g.city_can_construct(c[1], b, false, false, false) && d.buildings[b].building_class_type.is_some_and(|k| !ffh_core::utils::is_limited_wonder_class(d, k))).expect("a building");
        g.city_push_order(c[1], OrderType::ORDER_CONSTRUCT, building.i(), -1, false, false, true, false);
        g.cities[c[1]].building_production[building] = g.city_production_needed_building(c[1], building) - 1;
        let w = d.t::<UnitType>("UNIT_WARRIOR");
        g.city_push_order(c[2], OrderType::ORDER_TRAIN, w.i(), UnitAIType::UNITAI_ATTACK.i(), false, false, true, false);
        g.city_push_order(c[3], OrderType::ORDER_CONSTRUCT, building.i(), -1, false, false, true, false);
        assert!(g.city_production_turns_left_building(c[3], building, 0) > 3, "a long build");
        // An enemy stack next to the last governed city: it keeps its production.
        for _ in 0..3 {
            g.player_init_unit(p1, w, 13, 3, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        // At peace: no requisition (a list set by hand is cleared).
        g.army_set_requisition(p0, a, &[c[4]]).unwrap();
        turn(&mut g, p0);
        assert!(g.armies[a].requisition.is_empty(), "no requisition at peace");
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_declare_war(t0, t1, false, None);
        turn(&mut g, p0);
        let need = crate::diplomacy::army_needed_for_war(&g, p0, t1);
        assert_eq!(g.armies[a].target_size, need);
        assert_eq!(g.cities[c[2]].order_queue[0].dest, Some(RecruitDest::Army(a)), "the attacker goes to the army");
        // Short of the target: two in the army, one on its way from c[2]'s queue.
        assert!(need - 3 >= 3, "room for three cities");
        let list = g.armies[a].requisition.clone();
        assert_eq!(list[..2].iter().copied().collect::<std::collections::BTreeSet<_>>(), [c[4], c[5]].into_iter().collect(), "the governed cities first");
        assert_eq!(list[2..], [c[3]], "then the directly ruled city whose build gives way");
        // The rules' requisition: an order at the front of each, the long build waiting behind.
        g.player_do_requisition(p0);
        for &x in &list {
            let o = &g.cities[x].order_queue[0];
            assert!(o.requisition && o.dest == Some(RecruitDest::Army(a)), "a requisition order in {x:?}");
        }
        assert_eq!(g.cities[c[3]].order_queue[1].data1, building.i(), "the build waits behind it");
        assert!(c[..3].iter().chain([&c[6]]).all(|&x| g.cities[x].order_queue.iter().all(|o| !o.requisition)));
        // An order under way stays, and nothing more is placed while the army is full.
        turn(&mut g, p0);
        assert_eq!(g.armies[a].requisition, list);
        // Peace: the requisition ends and its orders are withdrawn.
        g.team_make_peace(t0, t1, false);
        turn(&mut g, p0);
        assert!(g.armies[a].requisition.is_empty());
        assert!(g.army_requisition_orders(a).is_empty(), "withdrawn");
    }

    /// At war through a turn without a main army (the army out of supply), the orders under way
    /// stay, except in a city that can no longer spare its production: a threatened city trains
    /// its own defender rather than the army's unit.
    #[test]
    fn without_a_main_army_a_threatened_city_is_released() {
        let (mut g, c, a) = requisition_game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_declare_war(t0, t1, false, None);
        turn(&mut g, p0);
        g.player_do_requisition(p0);
        let busy = ordered_cities(&g, a);
        assert!(busy.contains(&c[4]) && busy.contains(&c[5]), "orders under way in the governed cities: {busy:?}");
        // The army crosses out of supply (five plots beyond the last city): no main army.
        for u in g.army_units(a) {
            g.unit_set_xy(u, 17, 8, true);
        }
        assert!(!g.army_in_supply(a));
        assert_eq!(main_army(&g, p0, t1), None);
        // An enemy stack next to one of the cities with an order.
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        for _ in 0..3 {
            g.player_init_unit(p1, w, 9, 3, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        assert!(!can_spare(&g, p0, c[4]));
        turn(&mut g, p0);
        assert!(!g.armies[a].requisition.contains(&c[4]), "the threatened city is released");
        assert!(g.cities[c[4]].order_queue.iter().all(|o| !o.requisition), "its order withdrawn");
        assert!(g.armies[a].requisition.contains(&c[5]), "the others keep theirs");
        assert!(g.army_requisition_orders(a).iter().any(|&(x, _)| x == c[5]));
    }

    /// A marching recruit is the rules' to move: the AI gives it no orders, no stack of ours
    /// takes it in, and it is no defender of the city it stands in.
    #[test]
    fn a_marching_unit_is_not_moved_or_counted() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let home = g.player_init_city(p0, 4, 5, true, true);
        let far = g.player_init_city(p0, 10, 7, true, true);
        g.player_init_city(p1, 16, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 8, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        // A recruit for the army and a defender bound for the other city's garrison, both just
        // trained in `home`, beside two attackers of ours that would gather into a stack there.
        let recruit = g.player_init_unit(p0, w, 4, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        g.unit_recruit_to(recruit, home, RecruitDest::Army(a));
        let defender = g.player_init_unit(p0, w, 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        g.unit_recruit_to(defender, home, RecruitDest::Garrison(far));
        for _ in 0..2 {
            g.player_init_unit(p0, w, 4, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        assert!(g.unit_march(recruit).is_some() && g.unit_march(defender).is_some());
        assert!(!crate::military::is_army_unit(&g, recruit), "not one of the stacks' units");
        assert_eq!(crate::production::garrison(&g, p0, g.city_plot(home)).0, 0, "no defender of the city it stands in");
        // Ready to move and flagged for an update: the AI still leaves it alone.
        let here = g.unit_plot(recruit);
        for u in [recruit, defender] {
            let grp = g.units[u].group.unwrap();
            g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
            g.groups[grp].force_update = true;
        }
        let ai = crate::Ai::new();
        let mut v = crate::view::View::new(&g, p0);
        v.analyze(&g);
        crate::units::play(&ai, &mut g, &mut v);
        for u in [recruit, defender] {
            assert_eq!(g.unit_plot(u), here, "not moved");
            let grp = g.units[u].group.unwrap();
            assert_eq!(g.groups[grp].units, vec![u], "alone in its group");
            assert!(g.groups[grp].mission_queue.is_empty(), "no orders");
            assert!(g.unit_march(u).is_some(), "still marching");
        }
    }
}
