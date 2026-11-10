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
//! A trait changes what the governor does with its role, not how well: a cautious governor keeps
//! its trait's defenders on top of the garrison quota, an ambitious one ranks buildings by score
//! times production cost. [`plan`] is the governor's next buildings in that order.
//!
//! **The AI's governed empire** ([`turn`], once a turn): its priorities (the war roles while a
//! civilization at war threatens its cities, else the roles of its leader's flavors), and its
//! empire orders: in its capital and its three most productive other governed cities, the AI's
//! own production ladder (`production::decide`) is asked, and what it picks becomes an empire
//! order when the governor would not build it (settlers, wonders, rituals, heroes, field units,
//! ships...); the ladder's fillers wait for the governor's next choice, and there an ordinary
//! building the ladder names gives way to a wonder it ranks nearly as high. A city whose garrison
//! is missing or overwhelmed drops its empire orders for the governor's defender.
//!
//! Cultural channels are the civilization's, not its governors' (culture.rs plans them in
//! governed cities too).

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::{self as prod, Choice, Role};
use crate::Ai;

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
    // A cautious governor keeps more (its trait's defenders, alongside the role's).
    needed += g.city_governor_trait(c).map_or(0, |t| d.governor_traits[t].defenders);
    if prod::barbarian_lair_near(g, p, plot, 3) || !held && prod::barbarians_near(g, p, plot, 3) {
        needed += 1;
    }
    if !crate::economy::can_afford_unit(g, p, true, prod::civ_enemies_at_war(g, p)) && threat == 0 && !crate::probes::keeps_garrison(g, p) {
        needed = needed.min(1);
    }
    // A probe's garrison (probes.rs).
    let needed = crate::probes::garrison_need(g, p, needed, true);
    let overwhelmed = threat > 0 && defense * 3 / 2 < threat && defenders < needed + 2;
    let second = prod::second_defender_wanted(g, p, c, plot, defenders, threat, defense) && !crate::probes::skips_second_defender(g, p);
    if defenders == 0 || overwhelmed || defenders < needed || second {
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

    // A probe's army and buildings (probes.rs).
    if let Some((choice, why)) = crate::probes::production(g, c, threat) {
        return Some((choice, why, None));
    }

    // 3. The best building for the role (an ambitious governor: the costliest it may build).
    if let Some(&(b, _)) = ranked_buildings(g, c, threat).first() {
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

/// The buildings city `c`'s governor may choose, best first by its score (ties: the first
/// building type): the city AI's value (`building_value`) per turn of production, up to half again
/// for a building all of whose flavor its role weighs 5; an ambitious governor's score is that
/// times the building's production cost, so it prefers the costliest. Not a wonder or a palace
/// (the player's empire orders), nothing of no value, nothing taking more than 50 turns.
fn ranked_buildings(g: &Game, c: CityId, threat: i32) -> Vec<(BuildingType, i64)> {
    let d = g.d;
    let profile = g.city_role(c).map(|k| &d.city_roles[k]);
    let ambitious = g.city_governor_trait(c).is_some_and(|t| d.governor_traits[t].ambitious);
    let flavors: Vec<FlavorType> = (0..d.reg.count::<FlavorType>()).map(FlavorType::from_usize).collect();
    let mut out: Vec<(BuildingType, i64)> = Vec::new();
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
        // The role's lean: up to half again for a building all of whose flavor it weighs 5.
        let bf = &d.buildings[b].flavor_value;
        let total: i32 = flavors.iter().map(|&f| bf[f].max(0)).sum();
        let lean = match profile {
            Some(pr) if total > 0 => 10 * flavors.iter().map(|&f| pr.flavor_value[f] * bf[f].max(0)).sum::<i32>() / total,
            _ => 0,
        };
        let mut score = value as i64 * (100 + lean as i64) / (turns as i64 + 3);
        if ambitious {
            score *= g.city_production_needed_building(c, b).max(1) as i64;
        }
        out.push((b, score));
    }
    out.sort_by_key(|&(b, score)| (std::cmp::Reverse(score), b));
    out
}

/// The next `n` buildings city `c`'s governor means to build, in the order [`decide`] would pick
/// them (the client shows them as the governor's plan; docs/GOVERNORS.md): its candidates ranked
/// once by the same score, without simulating what each one built would change.
pub fn plan(g: &Game, c: CityId, n: usize) -> Vec<BuildingType> {
    let threat = prod::local_threat(g, g.cities[c].owner, g.city_plot(c), 3);
    ranked_buildings(g, c, threat).into_iter().take(n).map(|(b, _)| b).collect()
}

// -------------------------------------------------------------------------------------------
// The AI's governed empire: priorities and empire orders
// -------------------------------------------------------------------------------------------

/// How many cities the AI places its empire orders in: its capital and its most productive
/// governed cities (the directly ruled set before governors v2).
const EMPIRE_CITIES: usize = 4;

/// The AI's governed empire, once a turn (before the armies' turn, so that the attackers it
/// orders go to the main army the same turn): its priorities ([`set_priorities`]); an empire
/// order in a city whose garrison is missing or overwhelmed gives way to the governor's defender
/// ([`yield_to_defense`]); and the empire orders of its own production choice in its capital and
/// most productive governed cities ([`place_empire_orders`]). Switch `GOVERNORS` off: no empire
/// orders (the priorities are still set).
pub fn turn(ai: &Ai, g: &mut Game, p: PlayerId) {
    if g.player_is_barbarian(p) || g.players[p].human || g.player_is_crusade_host(p) {
        return;
    }
    set_priorities(g, p);
    if g.player_rules_directly(p) || !crate::switches::GOVERNORS.on(g, p) {
        return;
    }
    for c in g.players[p].cities.clone() {
        if g.cities.contains(c) && g.city_is_governed(c) {
            yield_to_defense(ai, g, c);
        }
    }
    place_empire_orders(ai, g, p);
}

/// The cities `p` places empire orders in: its capital (if governed) and its most productive other
/// governed cities, [`EMPIRE_CITIES`] in all (ties: the older city); no settlement.
pub(crate) fn empire_cities(g: &Game, p: PlayerId) -> Vec<CityId> {
    let ok = |c: CityId| g.cities.contains(c) && g.cities[c].owner == p && g.city_is_governed(c) && !g.cities[c].settlement;
    let capital = g.players[p].capital_city.filter(|&c| ok(c));
    let mut rest: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| ok(c) && Some(c) != capital).collect();
    rest.sort_by_key(|&c| (std::cmp::Reverse(g.city_yield_rate(c, YieldType::YIELD_PRODUCTION)), c));
    capital.into_iter().chain(rest).take(crate::probes::empire_cities(g, p, EMPIRE_CITIES)).collect()
}

/// In each of [`empire_cities`] with no empire order queued, the AI's own production choice (the
/// directly ruled city's ladder, `production::decide`) becomes an empire order when it is one
/// ([`empire_order`]): a settler, a wonder, a ritual, a hero, a field unit, a ship... Whatever the
/// governor does itself (the garrison, an ordinary building, a process) is left to it.
fn place_empire_orders(ai: &Ai, g: &mut Game, p: PlayerId) {
    for c in empire_cities(g, p) {
        if g.cities[c].order_queue.iter().any(|o| o.empire) {
            continue;
        }
        if let Some((choice, why)) = prod::decide(ai, g, c) {
            if FILLERS.contains(&why) && !at_choice_point(g, c) {
                crate::trace!(g, p, "governors: {} keeps its governor's build ({why} waits for its next choice)", g.d.text.name(&g.cities[c].name));
                continue;
            }
            if let Choice::Construct(b) = choice {
                if empire_order(g, choice).is_none() && at_choice_point(g, c) {
                    if let Some(w) = wonder_beside(g, c, b) {
                        place(g, c, Choice::Construct(w), "wonder_beside");
                        continue;
                    }
                }
            }
            place(g, c, choice, why);
        }
    }
}

/// When the AI's ladder names an ordinary building `best` for governed city `c` at its governor's
/// choice point ([`at_choice_point`]): the wonder (world, team or national; not a palace) it would
/// rank nearest, if it scores at least three quarters as much by the ladder's measure (value per
/// turn of production, its filters: 50 turns at most, a slow world wonder only in a city among the
/// quickest to finish it), and the city is safe (threat at most half its defense). A directly
/// ruled city builds `best` and turns to the wonder at its next choice; in a governed city the
/// governor builds the ordinary buildings in its own order, so the ladder could keep naming `best`
/// for many turns and the wonder would never come (measured: 8 seeds, 300 turns, Heroic and
/// National Epics built 10 and 8 times without this, 24 and 24 in the base; with a bar of half and
/// at any turn, 38 and 42, the world wonders half again the base's and population 9% under it).
fn wonder_beside(g: &Game, c: CityId, best: BuildingType) -> Option<BuildingType> {
    let d = g.d;
    let p = g.cities[c].owner;
    let plot = g.city_plot(c);
    let threat = prod::local_threat(g, p, plot, 3);
    if threat * 2 > prod::garrison(g, p, plot).1 {
        return None;
    }
    let score = |b: BuildingType| {
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        (turns, prod::building_value(g, c, b, threat) * 100 / (turns + 3))
    };
    let bar = score(best).1 * 3 / 4;
    let quick = 2 * g.players[p].cities.len() as i32 + 1;
    let mut found: Option<(BuildingType, i32)> = None;
    for b in d.buildings.keys() {
        if d.buildings[b].capital || !g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i()) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let (turns, s) = score(b);
        if turns > 50 || s <= 0 || s < bar {
            continue;
        }
        if turns > quick && prod::is_world_wonder(d, b) && crate::switches::TOP_CITIES.on(g, p) && !prod::top_wonder_city(g, c, b) {
            continue;
        }
        if found.map_or(true, |(_, f)| s > f) {
            found = Some((b, s));
        }
    }
    found.map(|(b, _)| b)
}

/// The ladder's steps that fill a city's production between its buildings (the peacetime
/// interleave of army and buildings, the army to its target, the elite sink): the ladder takes
/// them where a city chooses its next build, so they become empire orders only there
/// ([`at_choice_point`]). Asked every turn instead, the interleave would put an attacker ahead of
/// the governor's building every third turn (measured: 8 seeds, 300 turns, 62% of the empire
/// orders were `army_interleave` and the world wonders fell by a quarter against the base).
const FILLERS: [&str; 4] = ["army_interleave", "army", "attacker", "elite"];

/// Whether governed city `c` is where a directly ruled city would choose its next build: its
/// governor's own order (behind the requisition and empire orders) is none, a process, or one not
/// yet started (no production in it: chosen at the end of the last turn).
fn at_choice_point(g: &Game, c: CityId) -> bool {
    let city = &g.cities[c];
    match prod::own_order(g, c) {
        None => true,
        Some(o) => match o.order_type {
            OrderType::ORDER_CONSTRUCT => city.building_production[BuildingType::from_usize(o.data1 as usize)] == 0,
            OrderType::ORDER_TRAIN => city.unit_production[UnitType::from_usize(o.data1 as usize)] == 0,
            _ => true,
        },
    }
}

/// The production choice `choice` (made for `why`) becomes an empire order in governed city `c`
/// if it is one ([`empire_order`]; a unit keeps its unit AI, and goes where the rules send an
/// order without a destination: its garrison, or the main army for a field unit, `armies::turn`).
/// Whether it was placed.
pub(crate) fn place(g: &mut Game, c: CityId, choice: Choice, why: &'static str) -> bool {
    let p = g.cities[c].owner;
    let Some((order, data1, data2)) = empire_order(g, choice) else {
        crate::trace!(g, p, "governors: {} leaves {} to its governor ({why})", g.d.text.name(&g.cities[c].name), prod::choice_name(g, choice));
        return false;
    };
    match g.city_push_empire_order(p, c, order, data1, data2, None) {
        Ok(()) => {
            crate::trace!(g, p, "governors: empire order in {}: {} ({why})", g.d.text.name(&g.cities[c].name), prod::choice_name(g, choice));
            g.tel_decision(p, "empire_order", prod::type_name(g, choice), why, 0, String::new);
            true
        }
        Err(e) => {
            crate::trace!(g, p, "governors: no empire order in {} for {}: {e}", g.d.text.name(&g.cities[c].name), prod::choice_name(g, choice));
            false
        }
    }
}

/// The empire order (order type, data1, data2) a production choice makes, if it is one the
/// governor does not do itself: any unit but a city defender (the governor's garrison), with its
/// unit AI; a wonder or a palace; a project. `None` for a city defender, an ordinary building, a
/// process.
pub(crate) fn empire_order(g: &Game, choice: Choice) -> Option<(OrderType, i32, i32)> {
    let (order, data1, data2) = match choice {
        Choice::Train(_, UnitAIType::UNITAI_CITY_DEFENSE) | Choice::Maintain(_) => return None,
        Choice::Train(u, unit_ai) => (OrderType::ORDER_TRAIN, u.i(), unit_ai.i()),
        Choice::Construct(b) => (OrderType::ORDER_CONSTRUCT, b.i(), -1),
        Choice::Create(pr) => (OrderType::ORDER_CREATE, pr.i(), -1),
    };
    g.order_may_be_empire(order, data1).then_some((order, data1, data2))
}

/// Governed city `c` without a defender, or whose garrison the threat around it overwhelms (the
/// condition the ladder's and the governor's garrison rules share): its empire orders are taken off
/// (what was put into them stays with each item) and its governor chooses again, which puts the
/// defender first. Only when the city can train a defender.
fn yield_to_defense(ai: &Ai, g: &mut Game, c: CityId) {
    if !g.cities[c].order_queue.iter().any(|o| o.empire) {
        return;
    }
    let p = g.cities[c].owner;
    let plot = g.city_plot(c);
    let (defenders, defense) = prod::garrison(g, p, plot);
    let threat = prod::local_threat(g, p, plot, 3);
    let endangered = defenders == 0 || threat > 0 && defense * 3 / 2 < threat;
    if !endangered || prod::best_unit(g, c, Role::Defense).is_none() {
        return;
    }
    while let Some(i) = g.cities.contains(c).then(|| g.cities[c].order_queue.iter().position(|o| o.empire)).flatten() {
        let o = &g.cities[c].order_queue[i];
        let what = match o.order_type {
            OrderType::ORDER_TRAIN => g.d.units[UnitType::from_usize(o.data1 as usize)].base.type_.as_str(),
            OrderType::ORDER_CONSTRUCT => g.d.buildings[BuildingType::from_usize(o.data1 as usize)].base.type_.as_str(),
            OrderType::ORDER_CREATE => g.d.projects[ProjectType::from_usize(o.data1 as usize)].base.type_.as_str(),
            _ => "?",
        };
        crate::trace!(g, p, "governors: {} (threat {threat}, defense {defense}, {defenders} defenders): empire order {what} gives way to the garrison", g.d.text.name(&g.cities[c].name));
        if g.city_remove_empire_order(p, c, i).is_err() {
            return;
        }
    }
    if !g.cities.contains(c) {
        return;
    }
    prod::clear_own_orders(g, c);
    prod::choose(ai, g, c);
}

/// The AI's empire priorities (at most `GOVERNOR_MAX_PRIORITIES`), set when they change
/// ([`wanted_priorities`]). The AI does not override its cities' proposed roles.
pub(crate) fn set_priorities(g: &mut Game, p: PlayerId) {
    let wanted = wanted_priorities(g, p);
    if g.players[p].governor_priorities != wanted {
        let names: Vec<&str> = wanted.iter().map(|&r| g.d.city_roles[r].base.type_.as_str()).collect();
        crate::trace!(g, p, "governors: priorities [{}]", names.join(", "));
        let _ = g.player_set_governor_priorities(p, wanted);
    }
}

/// The priorities `p` wants now: at war with a civilization whose units threaten one of our
/// cities (the capital, a border city or one they reached), the garrison and production roles,
/// kept for as long as a war with a civilization lasts; otherwise the roles whose flavors match
/// the leader's strongest flavors (the sum over flavors of the role's weight times the leader's;
/// ties: the first role; none that matches nothing).
pub(crate) fn wanted_priorities(g: &Game, p: PlayerId) -> Vec<CityRoleType> {
    let d = g.d;
    let max = d.defines.governor_max_priorities.max(0) as usize;
    let war_roles: Vec<CityRoleType> = ["ROLE_GARRISON", "ROLE_PRODUCTION"].iter().filter_map(|n| d.ty::<CityRoleType>(n)).take(max).collect();
    let at_war = prod::civ_enemies_at_war(g, p);
    if at_war && !war_roles.is_empty() && (g.players[p].governor_priorities == war_roles || g.players[p].cities.iter().any(|&c| civ_threat(g, p, c))) {
        return war_roles;
    }
    let Some(leader) = g.players[p].personality_type.map(|l| &d.leader_heads[l]) else { return Vec::new() };
    let mut scored: Vec<(i32, CityRoleType)> = d
        .city_roles
        .keys()
        .map(|r| (d.city_roles[r].flavor_value.iter().zip(leader.flavor_value.iter()).map(|(&a, &b)| a * b).sum::<i32>(), r))
        .filter(|&(s, _)| s > 0)
        .collect();
    scored.sort_by_key(|&(s, r)| (std::cmp::Reverse(s), r));
    scored.into_iter().take(max).map(|(_, r)| r).collect()
}

/// Whether a civilization at war with `p` (not the barbarians) has combat units we know of within
/// 3 plots of our city `c`.
fn civ_threat(g: &Game, p: PlayerId, c: CityId) -> bool {
    let team = g.players[p].team;
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    for dx in -3..=3 {
        for dy in -3..=3 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            for &u in &g.map.plots[q].units {
                let owner = g.units[u].owner;
                let their = g.players[owner].team;
                if their != team && !g.player_is_barbarian(owner) && g.at_war(team, their) && g.d.units[g.units[u].unit_type].combat > 0 && crate::view::known_threat(g, p, u) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    /// The one-landmass game with player 0 under Aristocracy (every city governed) holding cities
    /// at `spots` (the first its capital), each queue empty.
    fn governed(spots: &[(i32, i32)]) -> (Game, Vec<CityId>) {
        let mut g = crate::testkit::one_landmass_game(4);
        let cities: Vec<CityId> = spots.iter().map(|&(x, y)| g.player_init_city(P0, x, y, true, true)).collect();
        let civic = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
        let option = g.d.civics[civic].civic_option_type.expect("a civic option");
        g.player_set_civics(P0, option, Some(civic));
        for &c in &cities {
            assert!(g.city_is_governed(c));
            g.city_clear_order_queue(c);
        }
        (g, cities)
    }

    fn warrior(g: &Game) -> UnitType {
        g.d.t::<UnitType>("UNIT_WARRIOR")
    }

    fn defenders(g: &mut Game, c: CityId, n: usize) {
        let w = warrior(g);
        for _ in 0..n {
            g.player_init_unit(P0, w, g.cities[c].x, g.cities[c].y, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
    }

    fn set_trait(g: &mut Game, c: CityId, name: Option<&str>) {
        let t = name.map(|n| g.d.t::<GovernorTraitType>(n));
        let gov = g.cities[c].governor.expect("a governor");
        g.governors[gov].trait_ = t;
    }

    /// What the governor builds itself is never an empire order: a city defender, an ordinary
    /// building, a process; every other unit (with its unit AI), a wonder and a project are.
    #[test]
    fn empire_orders_are_what_the_governor_does_not_build() {
        let (g, _) = governed(&[(4, 5)]);
        let d = g.d;
        let w = warrior(&g);
        assert_eq!(empire_order(&g, Choice::Train(w, UnitAIType::UNITAI_CITY_DEFENSE)), None);
        assert_eq!(empire_order(&g, Choice::Train(w, UnitAIType::UNITAI_ATTACK)), Some((OrderType::ORDER_TRAIN, w.i(), UnitAIType::UNITAI_ATTACK.i())));
        let ordinary = d.buildings.keys().find(|&b| !g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i())).expect("an ordinary building");
        assert_eq!(empire_order(&g, Choice::Construct(ordinary)), None);
        let wonder = d.buildings.keys().find(|&b| prod::is_world_wonder(d, b)).expect("a world wonder");
        assert_eq!(empire_order(&g, Choice::Construct(wonder)), Some((OrderType::ORDER_CONSTRUCT, wonder.i(), -1)));
        let project = d.projects.keys().next().expect("a project");
        assert_eq!(empire_order(&g, Choice::Create(project)), Some((OrderType::ORDER_CREATE, project.i(), -1)));
        let process = d.processes.keys().next().expect("a process");
        assert_eq!(empire_order(&g, Choice::Maintain(process)), None);
    }

    /// The AI places empire orders in its capital and its three most productive other governed
    /// cities; under Despotism (direct rule) in none.
    #[test]
    fn the_capital_and_the_most_productive_cities_place_empire_orders() {
        let (mut g, cities) = governed(&[(3, 4), (3, 7), (7, 4), (7, 7), (11, 4), (11, 7)]);
        let set = empire_cities(&g, P0);
        assert_eq!(set.len(), EMPIRE_CITIES);
        assert_eq!(set[0], cities[0], "the capital first");
        let rate = |c: CityId| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
        for &c in &cities {
            if !set.contains(&c) {
                assert!(set[1..].iter().all(|&s| rate(s) >= rate(c)), "a less productive city left out");
            }
        }
        let despotism = g.d.t::<CivicType>("CIVIC_DESPOTISM");
        let option = g.d.civics[despotism].civic_option_type.unwrap();
        g.player_set_civics(P0, option, Some(despotism));
        assert!(empire_cities(&g, P0).is_empty());
    }

    /// A field unit the AI's ladder picks is placed as an empire order ahead of the governor's own
    /// order, with its unit AI; an ordinary building is left to the governor.
    #[test]
    fn a_choice_the_governor_would_not_make_becomes_an_empire_order() {
        let (mut g, cities) = governed(&[(4, 5)]);
        let c = cities[0];
        defenders(&mut g, c, 2);
        let ai = crate::Ai::new();
        prod::choose(&ai, &mut g, c);
        let own = g.cities[c].order_queue.len();
        assert!(own > 0 && !g.cities[c].order_queue[0].empire, "the governor's order");
        let w = warrior(&g);
        assert!(place(&mut g, c, Choice::Train(w, UnitAIType::UNITAI_ATTACK), "test"));
        let q = &g.cities[c].order_queue;
        assert_eq!(q.len(), own + 1);
        assert!(q[0].empire && q[0].order_type == OrderType::ORDER_TRAIN && q[0].data1 == w.i());
        assert_eq!(q[0].data2, UnitAIType::UNITAI_ATTACK.i());
        assert!(!q[1].empire, "the governor's order resumes after it");
        let ordinary = g.d.buildings.keys().find(|&b| !g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i()) && g.city_can_construct(c, b, false, false, false)).expect("a building");
        assert!(!place(&mut g, c, Choice::Construct(ordinary), "test"));
        assert_eq!(g.cities[c].order_queue.len(), own + 1);
    }

    /// The ladder's fillers wait for the governor's next choice: a city whose governor's build has
    /// not started is at a choice point, one with production in it is not.
    #[test]
    fn fillers_wait_for_the_governors_next_choice() {
        let (mut g, cities) = governed(&[(4, 5)]);
        let c = cities[0];
        assert!(at_choice_point(&g, c), "an empty queue");
        defenders(&mut g, c, 4);
        let ai = crate::Ai::new();
        prod::choose(&ai, &mut g, c);
        let Some(b) = g.city_production_building(c) else { panic!("the governor builds") };
        assert!(at_choice_point(&g, c), "not started");
        g.cities[c].building_production[b] = 5;
        assert!(!at_choice_point(&g, c), "under way");
    }

    /// A governed city without a defender drops its empire order, and its governor trains the
    /// garrison; with defenders the empire order stays.
    #[test]
    fn an_undefended_city_drops_its_empire_order_for_the_garrison() {
        let (mut g, cities) = governed(&[(4, 5), (12, 5)]);
        let (bare, held) = (cities[0], cities[1]);
        defenders(&mut g, held, 2);
        let ai = crate::Ai::new();
        let w = warrior(&g);
        for &c in &cities {
            g.city_push_empire_order(P0, c, OrderType::ORDER_TRAIN, w.i(), UnitAIType::UNITAI_ATTACK.i(), None).unwrap();
            yield_to_defense(&ai, &mut g, c);
        }
        assert!(g.cities[held].order_queue[0].empire, "a held city keeps it");
        let q = &g.cities[bare].order_queue;
        assert!(q.iter().all(|o| !o.empire), "the empire order gone");
        assert!(q[0].order_type == OrderType::ORDER_TRAIN && q[0].data2 == UnitAIType::UNITAI_CITY_DEFENSE.i(), "the governor's defender");
    }

    /// A cautious governor keeps its trait's defenders more than a governor without a trait.
    #[test]
    fn a_cautious_governor_keeps_more_defenders() {
        let (mut g, cities) = governed(&[(4, 5)]);
        let c = cities[0];
        set_trait(&mut g, c, None);
        let garrison = |g: &Game| matches!(decide(g, c), Some((Choice::Train(_, UnitAIType::UNITAI_CITY_DEFENSE), ..)));
        let mut n = 0;
        while garrison(&g) {
            defenders(&mut g, c, 1);
            n += 1;
            assert!(n < 10, "the quota is reached");
        }
        set_trait(&mut g, c, Some("GOVERNORTRAIT_CAUTIOUS"));
        assert!(garrison(&g), "a cautious governor wants another defender ({n} held)");
        let more = g.d.governor_traits[g.d.t::<GovernorTraitType>("GOVERNORTRAIT_CAUTIOUS")].defenders as usize;
        defenders(&mut g, c, more);
        assert!(!garrison(&g));
    }

    /// The plan is the governor's ranking: its first building is what it builds, and an ambitious
    /// governor's choice costs at least as much as the plain governor's.
    #[test]
    fn the_plan_is_the_governors_order_and_ambition_builds_big() {
        let (mut g, cities) = governed(&[(4, 5)]);
        let c = cities[0];
        defenders(&mut g, c, 4);
        set_trait(&mut g, c, None);
        let plain = plan(&g, c, 5);
        assert!(!plain.is_empty() && plain.len() <= 5);
        let mut distinct = plain.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), plain.len());
        assert!(matches!(decide(&g, c), Some((Choice::Construct(b), ..)) if b == plain[0]));
        assert!(plain.iter().all(|&b| !g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, b.i())), "no wonder in the plan");
        set_trait(&mut g, c, Some("GOVERNORTRAIT_AMBITIOUS"));
        let big = plan(&g, c, 5);
        assert!(matches!(decide(&g, c), Some((Choice::Construct(b), ..)) if b == big[0]));
        let cost = |b: BuildingType| g.city_production_needed_building(c, b);
        assert!(cost(big[0]) >= cost(plain[0]));
    }

    /// At peace the priorities are the roles of the leader's strongest flavors; a civilization at
    /// war whose unit stands near our city sets the war roles, kept while the war lasts and dropped
    /// at the peace.
    #[test]
    fn priorities_follow_the_leader_and_the_war() {
        let (mut g, cities) = governed(&[(4, 5)]);
        let d = g.d;
        let leader = &d.leader_heads[g.players[P0].personality_type.unwrap()];
        let score = |r: CityRoleType| d.city_roles[r].flavor_value.iter().zip(leader.flavor_value.iter()).map(|(&a, &b)| a * b).sum::<i32>();
        set_priorities(&mut g, P0);
        let peace = g.players[P0].governor_priorities.clone();
        assert!(peace.len() <= d.defines.governor_max_priorities as usize);
        for r in d.city_roles.keys().filter(|r| !peace.contains(r)) {
            assert!(peace.iter().all(|&s| score(s) >= score(r)), "the strongest flavors");
        }
        let war_roles = vec![d.t::<CityRoleType>("ROLE_GARRISON"), d.t::<CityRoleType>("ROLE_PRODUCTION")];
        g.player_init_city(P1, 15, 5, true, true);
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        set_priorities(&mut g, P0);
        assert_eq!(g.players[P0].governor_priorities, peace, "at war, no city threatened");
        let raider = g.player_init_unit(P1, warrior(&g), g.cities[cities[0]].x + 2, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        set_priorities(&mut g, P0);
        assert_eq!(g.players[P0].governor_priorities, war_roles);
        g.unit_kill(raider, false, None);
        set_priorities(&mut g, P0);
        assert_eq!(g.players[P0].governor_priorities, war_roles, "kept while the war lasts");
        g.team_make_peace(t0, t1, false);
        set_priorities(&mut g, P0);
        assert_eq!(g.players[P0].governor_priorities, peace);
    }
}
