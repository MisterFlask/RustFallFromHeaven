//! Armies (this game's own rules; docs/ARMIES.md; docs/AI.md "Armies").
//!
//! The AI's groups become armies by themselves (the rules' army sync names every group of army
//! units), so its movement logic drives each army unchanged: a city-attack stack is an army, a
//! city's defenders are its garrison. A trained unit stands in the city that trained it, as any
//! unit; the AI's own movement takes the attackers to the field army (`campaign.rs` commits every
//! stack near the war's target and gathers them; `military::army` sends an idle one to the main
//! army, [`join_main_army`]). What an expert adds is the army's reinforcements:
//!
//! - **Target size.** At war, or preparing one, the main field army (the largest army in supply,
//!   nearest the enemy on ties) is to be the army the war needs (`diplomacy::army_needed_for_war`):
//!   `Army::target_size`, the AI's bookkeeping (the rules do not read it).
//! - **Orders for the army** ([`order_cities`]). While the main army is short of that
//!   ([`shortfall`]), the AI places empire unit orders (ahead of the governor's choice) in the
//!   governed cities an expert would draw on: near the front (a unit that joins within
//!   [`ORDER_HORIZON`] turns, its training and its march by land), not a city that needs its
//!   production for its own defence ([`can_spare`]), not a city with an empire order already
//!   queued (one at a time); as many cities as the army lacks units, the soonest to deliver first.
//!   The unit is [`reinforcement_unit`]: siege up to a third of the army, then the best attacker
//!   against cities. Nothing is withdrawn: an order beyond the need completes, and a city in
//!   danger drops its empire orders for a defender (`governor::yield_to_defense`). Directly ruled
//!   cities get no such orders: they choose by `production.rs`.
//! - **Defenders in the garrison.** A city defender that walked to another of our cities (an
//!   army of its own on the way) joins that city's garrison when it arrives.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::OrderData;
use ffh_data::*;

use crate::production::{self as prod, Role};

/// The unit roles that go to the field army.
const FIELD_ROLES: [UnitAIType; 4] = [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL, UnitAIType::UNITAI_COUNTER];

/// Whether `o` trains a field unit (an army unit in one of [`FIELD_ROLES`]).
fn is_field_order(g: &Game, o: &OrderData) -> bool {
    o.order_type == OrderType::ORDER_TRAIN && o.data1 >= 0 && opt_from_i32::<UnitAIType>(o.data2).is_some_and(|r| FIELD_ROLES.contains(&r)) && g.unit_type_is_army_unit(UnitType::from_usize(o.data1 as usize))
}

/// Whether `o` is an empire order for a field unit: what [`order_cities`] places for the main
/// army (and the governed empire's ladder orders, `governor::place_empire_orders`). Such a unit
/// is the army's, not its city's.
pub(crate) fn is_army_order(g: &Game, o: &OrderData) -> bool {
    o.empire && is_field_order(g, o)
}

/// The team `p` is at war with or prepares a war against (the preparation first), if any.
pub(crate) fn war_team(g: &Game, p: PlayerId) -> Option<TeamId> {
    let team = g.players[p].team;
    crate::diplomacy::preparing_target(g, team).or_else(|| g.team_ids().find(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)))
}

pub fn turn(g: &mut Game, p: PlayerId) {
    if g.player_is_barbarian(p) {
        return;
    }
    // A defender that walked into one of our cities to hold it serves in the city's garrison, not
    // as an army of its own (an expert's garrison: what holds the city, in one place).
    let mut garrisoned = false;
    for a in g.players[p].armies.clone() {
        // (Fleets are not armies here: the AI's navy is `naval.rs`'s, docs/NAVAL.md section 8.)
        if g.army_is_fleet(a) {
            continue;
        }
        let units = g.army_units(a);
        let in_city = g.army_plot(a).and_then(|plot| g.map.plots[plot].plot_city).is_some_and(|c| g.cities.contains(c) && g.cities[c].owner == p);
        if in_city && g.armies[a].disband_to.is_none() && !units.is_empty() && units.iter().all(|&u| g.units[u].ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE)) {
            for u in units {
                g.units[u].army = None;
            }
            garrisoned = true;
        }
    }
    if garrisoned {
        g.player_army_sync(p);
    }
    let target = war_team(g, p);
    let main = target.and_then(|t| main_army(g, p, t));
    if let (Some(a), Some(t)) = (main, target) {
        g.armies[a].target_size = crate::diplomacy::army_needed_for_war(g, p, t);
    }
    // Only the main army has a target size (at peace, none). Through a turn without a main army
    // while the war lasts (out of supply, at sea), the last one keeps its target: it is the main
    // one again when it is back in supply (`main_army` reads it).
    if main.is_some() || target.is_none() {
        for a in g.players[p].armies.clone() {
            if Some(a) != main && g.armies[a].disband_to.is_none() && !g.army_is_fleet(a) {
                g.armies[a].target_size = 0;
            }
        }
    }
    let Some(a) = main else { return };
    let short = shortfall(g, p, a);
    if crate::trace_on(p) {
        let (units, orders) = coming(g, p, a);
        let home = units.iter().filter(|&&u| g.map.plots[g.unit_plot(u)].plot_city.is_some_and(|c| g.cities[c].owner == p)).count();
        crate::trace!(g, p, "armies: the {} (size {} of {}, short {}): {} field units outside it ({} in our cities), {} orders", g.armies[a].name, g.army_units(a).len(), g.armies[a].target_size, short, units.len(), home, orders.len());
    }
    if short <= 0 {
        return;
    }
    let picked = order_cities(g, p, a, short as usize);
    if crate::trace_on(p) {
        let names: Vec<String> = picked.iter().map(|&(c, ..)| g.d.text.name(&g.cities[c].name).to_string()).collect();
        crate::trace!(g, p, "armies: the {} orders units in [{}]", g.armies[a].name, names.join(", "));
    }
    for (c, unit, role) in picked {
        match g.city_push_empire_order(p, c, OrderType::ORDER_TRAIN, unit.i(), role.i()) {
            Ok(()) => g.tel_decision(p, "empire_order", g.d.units[unit].base.type_.as_str(), "main_army", 0, String::new),
            Err(e) => crate::trace!(g, p, "armies: no order in {} for {}: {e}", g.d.text.name(&g.cities[c].name), g.d.units[unit].base.type_),
        }
    }
}

/// Turns within which the unit a city trains for the main army should join it (its production
/// turns there and its march by land): a city farther from the front than that is not drawn on
/// (its unit would come after the fighting it is for).
const ORDER_HORIZON: i32 = 20;

/// The field units on their way to main army `a` of `p`, or to be trained for it: every field unit
/// of ours on the army's landmass outside it (`military::is_army_unit`: the campaign commits and
/// gathers them, the idle ones walk to it, [`join_main_army`]), and every field order in our
/// cities' queues (the empire orders, [`is_army_order`], and a directly ruled city's own field
/// unit at the head of its queue).
fn coming(g: &Game, p: PlayerId, a: ArmyId) -> (Vec<UnitId>, Vec<(CityId, usize)>) {
    let area = g.army_plot(a).map(|at| g.map.plots[at].area);
    let units = g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u) && g.units[u].army != Some(a) && crate::military::is_army_unit(g, u) && !g.unit_is_cargo(u) && g.unit_plot_opt(u).is_some_and(|q| Some(g.map.plots[q].area) == area))
        .collect();
    let mut orders = Vec::new();
    for &c in &g.players[p].cities {
        let governed = g.city_is_governed(c);
        for (i, o) in g.cities[c].order_queue.iter().enumerate() {
            if is_army_order(g, o) || i == 0 && !governed && !o.empire && is_field_order(g, o) {
                orders.push((c, i));
            }
        }
    }
    (units, orders)
}

/// How many units main army `a` of `p` lacks: its target size less its units, the field units of
/// ours outside it on its landmass, and the field orders in our queues ([`coming`]).
pub(crate) fn shortfall(g: &Game, p: PlayerId, a: ArmyId) -> i32 {
    let (units, orders) = coming(g, p, a);
    g.armies[a].target_size - g.army_units(a).len() as i32 - units.len() as i32 - orders.len() as i32
}

/// Whether city `c` can spare its production for an army: it holds against what threatens its
/// owner there (a defender at least, and the defense `production::emergency` would not switch to
/// a defender for), and it is not training a defender for itself.
fn can_spare(g: &Game, c: CityId) -> bool {
    let p = g.cities[c].owner;
    let plot = g.city_plot(c);
    let (defenders, defense) = crate::production::garrison(g, p, plot);
    let threat = crate::production::local_threat(g, p, plot, 2);
    if defenders == 0 || defense * 3 / 2 < threat {
        return false;
    }
    !prod::own_order(g, c).is_some_and(|o| o.order_type == OrderType::ORDER_TRAIN && matches!(opt_from_i32::<UnitAIType>(o.data2), Some(UnitAIType::UNITAI_CITY_DEFENSE) | Some(UnitAIType::UNITAI_CITY_COUNTER)))
}

/// The governed cities of `p` that take an order for main army `a` this turn, at most `slots`,
/// with the unit each trains: cities on the army's landmass whose unit can reach it by land, that
/// can spare their production ([`can_spare`]) and have no empire order queued (nor a ritual under
/// way), the soonest to
/// deliver first (training plus march, within [`ORDER_HORIZON`]; the march guessed from the
/// distance to rank them, then found along the land).
pub fn order_cities(g: &Game, p: PlayerId, a: ArmyId, slots: usize) -> Vec<(CityId, UnitType, UnitAIType)> {
    let Some(at) = g.army_plot(a) else { return Vec::new() };
    let area = g.map.plots[at].area;
    // The candidates by their training time and a quick guess at the march (two plots a move: a
    // road on most of the way), then the march along the land, nearest first, until the slots
    // are filled: the land path is searched for a few cities a turn, not for every one.
    let siege = wants_siege(g, p, a);
    let (ax, ay) = g.map.xy(at);
    let mut cands: Vec<(i32, CityId, UnitType, UnitAIType, i32)> = Vec::new();
    for &c in &g.players[p].cities {
        // (Not a city whose ritual is under way: an empire order does not go ahead of it.)
        if !g.city_is_governed(c) || g.map.plots[g.city_plot(c)].area != area || g.cities[c].order_queue.iter().any(|o| o.empire) || g.city_ritual_in_progress(c).is_some() || !can_spare(g, c) {
            continue;
        }
        let Some((unit, role)) = unit_for(g, c, siege) else { continue };
        let production = g.city_production_turns_left_unit(c, unit, 0);
        let guess = g.plot_distance(g.cities[c].x, g.cities[c].y, ax, ay) / (2 * g.d.units[unit].moves.max(1));
        if production + guess <= ORDER_HORIZON {
            cands.push((production + guess, c, unit, role, production));
        }
    }
    cands.sort_by_key(|&(guess, c, ..)| (guess, c));
    let mut picked: Vec<(i32, CityId, UnitType, UnitAIType)> = Vec::new();
    for (_, c, unit, role, production) in cands {
        if picked.len() >= slots {
            break;
        }
        let Some(march) = g.march_turns(p, unit, g.city_plot(c), at) else { continue };
        if production + march <= ORDER_HORIZON {
            crate::trace!(g, p, "armies: the {} picks {} ({} in {} turns, {} to march)", g.armies[a].name, g.d.text.name(&g.cities[c].name), g.d.units[unit].base.type_, production, march);
            picked.push((production + march, c, unit, role));
        }
    }
    picked.sort_by_key(|&(turns, c, ..)| (turns, c));
    picked.into_iter().map(|(_, c, unit, role)| (c, unit, role)).collect()
}

/// The unit city `c` trains for main army `a` of its owner: siege while it is less than a third
/// of the army ([`wants_siege`]), else the best attacker against cities, else any attacker; only
/// units that serve in armies (a Ranger or an Assassin would stay in the city, and the army would
/// never fill).
pub fn reinforcement_unit(g: &Game, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
    unit_for(g, c, wants_siege(g, g.cities[c].owner, a))
}

/// Whether main army `a` of `p` wants siege: less than a third of it is siege (city attackers and
/// collateral units by their AI role), the field units and orders coming to it included
/// ([`coming`]).
fn wants_siege(g: &Game, p: PlayerId, a: ArmyId) -> bool {
    let siege_role = |r: Option<UnitAIType>| matches!(r, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL));
    let (units, orders) = coming(g, p, a);
    let mut siege = 0;
    let mut total = 0;
    for u in g.army_units(a).into_iter().chain(units) {
        total += 1;
        siege += i32::from(siege_role(g.units[u].ai_type));
    }
    for (oc, i) in orders {
        total += 1;
        siege += i32::from(siege_role(opt_from_i32::<UnitAIType>(g.cities[oc].order_queue[i].data2)));
    }
    siege * 3 < total
}

/// The army unit city `c` trains for the main army: siege when `siege` (and it can), else the
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

/// Where idle field stack `grp` of `p` (nothing else to do this turn: `military::army`'s last
/// resort) goes to join the main army, at war or preparing one: the main army's plot, on the
/// stack's landmass, when the stack is not part of it. (The campaign gathers the stacks near the
/// war's target; this takes the ones it leaves, such as a unit just trained in a city far from it.)
pub(crate) fn join_main_army(g: &Game, p: PlayerId, grp: GroupId) -> Option<PlotIdx> {
    let head = g.group_get_head_unit(grp)?;
    let a = main_army(g, p, war_team(g, p)?)?;
    let at = g.army_plot(a)?;
    let here = g.unit_plot(head);
    if g.units[head].army == Some(a) || at == here || g.map.plots[at].area != g.map.plots[here].area {
        return None;
    }
    Some(at)
}

/// Our main field army against `target`: the largest army (at least two units) in supply and not
/// disbanding, the nearest to `target`'s cities on ties -- but the main army so far (the one with
/// a target size: [`turn`] gives it to the main army alone) stays the main one while it is at
/// least two thirds the size of the largest; out of supply, there is no main army this turn
/// (its reinforcements wait for it) unless an army in supply is as good a choice by that measure.
/// An expert keeps reinforcing the stack he fights with: switching whenever another stack was a
/// unit larger for a turn (the AI's stacks split for strikes and merge again) or the field army
/// crossed out of supply (a pair of units at home became the main army) withdrew a third of the
/// requisition orders placed in the first games (the rules' requisition, removed 2026-11).
pub(crate) fn main_army(g: &Game, p: PlayerId, target: TeamId) -> Option<ArmyId> {
    let mut best: Option<(i32, i32, ArmyId)> = None;
    let mut current: Option<(i32, bool, ArmyId)> = None;
    for &a in &g.players[p].armies {
        if !g.armies.contains(a) || g.armies[a].disband_to.is_some() || g.army_is_fleet(a) {
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

    /// At war, the main field army's target size is the war's; a city's attack unit is trained
    /// in its city as any (no destination), and the target size goes back to 0 at peace.
    #[test]
    fn at_war_the_main_army_has_the_war_s_target_size() {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        g.player_init_city(p0, 4, 5, true, true);
        g.player_init_city(p1, 14, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        // Three plots out of the city (no roads: its plot is the network), so in supply.
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 7, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        turn(&mut g, p0);
        assert_eq!(g.armies[a].target_size, 0, "no target at peace");
        g.team_declare_war(t0, t1, false, None);
        turn(&mut g, p0);
        assert_eq!(g.armies[a].target_size, crate::diplomacy::army_needed_for_war(&g, p0, t1));
        g.team_make_peace(t0, t1, false);
        turn(&mut g, p0);
        assert_eq!(g.armies[a].target_size, 0);
    }

    /// The AI's unit for the main army: siege while it is short of a third, then attackers; only
    /// army units.
    #[test]
    fn reinforcement_takes_siege_then_attackers() {
        let mut g = crate::testkit::one_landmass_game(4);
        let p0 = PlayerId(0);
        let c = g.player_init_city(p0, 5, 5, true, true);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 8, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &units).unwrap();
        let (u, role) = reinforcement_unit(&g, c, a).expect("a unit");
        assert!(g.unit_type_is_army_unit(u));
        // No siege the city can train at the start: an attacker.
        if !g.unit_type_is_siege(u) {
            assert_eq!(role, UnitAIType::UNITAI_ATTACK);
        }
    }

    /// Seven cities of player 0 west of player 1's one (the first four ruled directly, the other
    /// three governed: set by hand, as no government mixes the two), a defender in each, and a
    /// main army of two warriors between them; the cities' queues empty, and whatever they would
    /// train for the army one turn from done (so every city is near enough to the front).
    fn army_game() -> (Game, Vec<CityId>, ArmyId) {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let cities: Vec<CityId> = [(3, 4), (3, 7), (6, 4), (6, 7), (9, 4), (9, 7), (12, 4)].iter().map(|&(x, y)| g.player_init_city(p0, x, y, true, true)).collect();
        g.player_init_city(p1, 16, 6, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        for &c in &cities[4..] {
            g.cities[c].direct_rule = false;
            g.cities[c].production_automated = true;
        }
        for &c in &cities {
            g.player_init_unit(p0, w, g.cities[c].x, g.cities[c].y, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
            g.city_clear_order_queue(c);
        }
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 13, 6, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        assert!(g.army_in_supply(a), "two plots from the nearest city");
        for &c in &cities {
            let (u, _) = reinforcement_unit(&g, c, a).expect("a unit to train");
            g.cities[c].unit_production[u] = g.city_production_needed_unit(c, u) - 1;
        }
        assert_eq!(cities.iter().map(|&c| g.city_is_governed(c)).collect::<Vec<_>>(), [false, false, false, false, true, true, true]);
        (g, cities, a)
    }

    /// The empire orders of city `c` that train a field unit for the army.
    fn army_orders(g: &Game, c: CityId) -> usize {
        g.cities[c].order_queue.iter().filter(|o| is_army_order(g, o)).count()
    }

    /// At war, while the main army is short, the AI places one empire unit order in each governed
    /// city that can spare its production (not the threatened one), none in the directly ruled
    /// cities; the orders count against the shortfall, and a city with one gets no second.
    #[test]
    fn a_short_main_army_orders_units_in_governed_cities() {
        let (mut g, c, a) = army_game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        // An enemy stack next to the last governed city: it keeps its production.
        for _ in 0..3 {
            g.player_init_unit(p1, w, 13, 3, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        // At peace: no orders.
        turn(&mut g, p0);
        assert!(c.iter().all(|&x| g.cities[x].order_queue.iter().all(|o| !o.empire)), "no orders at peace");
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_declare_war(t0, t1, false, None);
        assert!(!can_spare(&g, c[6]), "threatened");
        turn(&mut g, p0);
        let need = crate::diplomacy::army_needed_for_war(&g, p0, t1);
        assert_eq!(g.armies[a].target_size, need);
        assert!(need - 2 >= 3, "short of more than the two cities can give: {need}");
        for &x in &c[4..6] {
            assert_eq!(army_orders(&g, x), 1, "one order in {x:?}");
            let o = &g.cities[x].order_queue[0];
            assert!(o.empire && o.order_type == OrderType::ORDER_TRAIN, "at the head of the queue");
        }
        assert!(c[..4].iter().chain([&c[6]]).all(|&x| g.cities[x].order_queue.iter().all(|o| !o.empire)), "none in the directly ruled cities or the threatened one");
        assert_eq!(shortfall(&g, p0, a), need - 2 - 2, "the army's two units and the two orders counted");
        // Still short, but every city that can spare its production has its order.
        turn(&mut g, p0);
        for &x in &c[4..6] {
            assert_eq!(army_orders(&g, x), 1, "no second order in {x:?}");
        }
        // An attacker of ours outside the army counts toward it.
        g.player_init_unit(p0, w, 6, 4, Some(UnitAIType::UNITAI_ATTACK), None);
        assert_eq!(shortfall(&g, p0, a), need - 2 - 2 - 1);
    }

    /// At war, a field unit trained in a home city far from the main army (no operation this
    /// turn: the enemy city is unseen) walks to the army; the city's defender stays.
    #[test]
    fn an_idle_attacker_at_home_walks_to_the_main_army() {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let home = g.player_init_city(p0, 3, 5, true, true);
        g.player_init_city(p0, 10, 7, true, true);
        let theirs = g.player_init_city(p1, 17, 8, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        g.plot_set_revealed(g.city_plot(theirs), t0, false, false, None, false);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        for (x, y) in [(3, 5), (10, 7)] {
            g.player_init_unit(p0, w, x, y, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 11, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        let recruit = g.player_init_unit(p0, w, 3, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        g.player_army_sync(p0);
        assert_eq!(main_army(&g, p0, t1), Some(a));
        assert_eq!(join_main_army(&g, p0, g.units[recruit].group.unwrap()), g.army_plot(a));
        assert_eq!(join_main_army(&g, p0, g.units[army[0]].group.unwrap()), None, "the army itself");
        let defender = g.map.plots[g.city_plot(home)].units.iter().copied().find(|&u| g.units[u].ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE)).unwrap();
        let ai = crate::Ai::new();
        let mut v = crate::view::View::new(&g, p0);
        v.analyze(&g);
        crate::units::play(&ai, &mut g, &mut v);
        assert!(g.units[recruit].x > 3, "on its way to the army: {:?}", (g.units[recruit].x, g.units[recruit].y));
        assert_eq!(g.unit_plot(defender), g.city_plot(home), "the defender stays");
    }
}
