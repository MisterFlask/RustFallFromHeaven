//! City production (C++ `CvCityAI::AI_chooseProduction`, FfH's `CvGameUtils.AI_chooseProduction`
//! Python): a priority list -- garrison, emergency defense, the first worker, settlers while
//! good sites remain, minimal army, more workers, buildings by marginal value per turn of
//! production, army to its target, casters, missionaries -- then a process. Whatever happens,
//! a city is never left without an order (`fallback`).

use std::sync::OnceLock;

use ffh_core::defs::{CITY_HOME_PLOT, NUM_CITY_PLOTS};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::expansion::SettlerPlan;
use crate::sites;
use crate::view::type_attack_power;
use crate::Ai;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Train(UnitType, UnitAIType),
    Construct(BuildingType),
    Create(ProjectType),
    Maintain(ProcessType),
}

pub fn choose(ai: &Ai, g: &mut Game, c: CityId) {
    let p = g.cities[c].owner;
    if !g.cities[c].order_queue.is_empty() {
        // Asked again with an order queued (production made dirty, or the head is a process
        // the rules re-ask about every turn): a process gives way to a fresh choice; anything
        // else keeps being built.
        if g.city_is_production_process(c) {
            g.city_clear_order_queue(c);
        } else {
            return;
        }
    }
    if g.player_is_barbarian(p) {
        // FfH barbarian cities pick from a fixed list (CustomFunctions.BarbCityProduction).
        if g.cf_barb_city_production(c) {
            return;
        }
    }
    // An undefended city with one of our fighters standing in it (the starting warrior, an
    // escort): that unit becomes the garrison instead of training a new one.
    let plot = g.city_plot(c);
    if !g.player_is_barbarian(p) && garrison(g, p, plot).0 == 0 {
        let spare = g.map.plots[plot].units.iter().copied().find(|&u| {
            let unit = &g.units[u];
            unit.owner == p && g.d.units[unit.unit_type].combat > 0 && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && matches!(unit.ai_type, Some(UnitAIType::UNITAI_ATTACK) | Some(UnitAIType::UNITAI_COUNTER) | Some(UnitAIType::UNITAI_RESERVE))
        });
        if let Some(u) = spare {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_CITY_DEFENSE);
        }
    }
    if let Some((choice, why)) = decide(ai, g, c) {
        crate::trace!(g, p, "{} size {} builds {}", g.cities[c].name, g.cities[c].population, choice_name(g, choice));
        if let (true, Choice::Construct(b)) = (crate::trace_on(p), choice) {
            let coded = crate::code_effects::building_value(g, c, b, 0);
            if coded != 0 {
                crate::trace!(g, p, "{}: {} worth {} for effects in the rules' code", g.cities[c].name, g.d.buildings[b].base.type_, coded);
            }
        }
        g.tel_decision(p, "production", type_name(g, choice), why, 0, String::new);
        push(g, c, choice);
    }
    if g.cities[c].order_queue.is_empty() {
        // Never leave a city idle: anything it can make.
        if let Some(choice) = fallback(g, c) {
            crate::trace!(g, p, "{} size {} falls back to {}", g.cities[c].name, g.cities[c].population, choice_name(g, choice));
            g.tel_decision(p, "production", type_name(g, choice), "fallback", 0, String::new);
            push(g, c, choice);
        }
    }
}

/// The type name of what a choice makes (telemetry).
fn type_name(g: &Game, choice: Choice) -> &'static str {
    let d = g.d;
    match choice {
        Choice::Train(u, _) => d.units[u].base.type_.as_str(),
        Choice::Construct(b) => d.buildings[b].base.type_.as_str(),
        Choice::Create(pr) => d.projects[pr].base.type_.as_str(),
        Choice::Maintain(pr) => d.processes[pr].base.type_.as_str(),
    }
}

pub fn choice_name(g: &Game, choice: Choice) -> String {
    match choice {
        Choice::Train(u, ai) => format!("{} ({:?})", g.d.units[u].base.type_, ai),
        Choice::Construct(b) => g.d.buildings[b].base.type_.clone(),
        Choice::Create(pr) => g.d.projects[pr].base.type_.clone(),
        Choice::Maintain(pr) => g.d.processes[pr].base.type_.clone(),
    }
}

pub fn push(g: &mut Game, c: CityId, choice: Choice) {
    match choice {
        Choice::Train(u, unit_ai) => g.city_push_order(c, OrderType::ORDER_TRAIN, u.i(), unit_ai.i(), false, false, false, false),
        Choice::Construct(b) => g.city_push_order(c, OrderType::ORDER_CONSTRUCT, b.i(), -1, false, false, false, false),
        Choice::Create(pr) => g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, false),
        Choice::Maintain(pr) => g.city_push_order(c, OrderType::ORDER_MAINTAIN, pr.i(), -1, false, false, false, false),
    }
}

/// Once per turn (C++ `CvCityAI::AI_doTurn` sets production dirty on danger): a city under
/// threat that is not building a defender switches to one (the progress on the old item is
/// kept by the rules). The switch clears the queue and asks [`decide`] again, whose garrison rule
/// has the last word: with switch `EMERGENCY_DECIDE` (on), a city switches only when `decide` would
/// put a fighter first. Before, a city whose garrison `decide` found full (`needed + 2` defenders)
/// was switched every turn and re-picked the same building: an Illian city with six defenders
/// chose its Temple of the Hand seven times in ten turns, losing the rest of its queue each time.
pub fn emergency(ai: &Ai, g: &mut Game, p: PlayerId) {
    // A settler waiting in port for a ship that nobody builds: the port builds one now.
    if let Some(q) = crate::naval::settler_waiting(g, p) {
        if let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities[c].owner == p) {
            let building_ship = g.city_production_unit(c).is_some_and(|u| crate::naval::is_transport_type(&g.d.units[u]));
            if !building_ship && crate::naval::num_transports(g, p) == 0 && local_threat(g, p, q, 2) == 0 {
                if let Some(u) = best_unit(g, c, Role::Transport) {
                    crate::trace!(g, p, "{}: a ship for the waiting settler", g.cities[c].name);
                    g.city_push_order(c, OrderType::ORDER_TRAIN, u.i(), UnitAIType::UNITAI_SETTLER_SEA.i(), false, false, false, false);
                }
            }
        }
    }
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) || g.cities[c].order_queue.is_empty() {
            continue;
        }
        let plot = g.city_plot(c);
        let threat = local_threat(g, p, plot, 2);
        let (defenders, defense) = garrison(g, p, plot);
        let building_fighter = g.city_production_unit(c).is_some_and(|u| g.d.units[u].combat > 0);
        if threat == 0 {
            // A long build in a city that should have a second defender by now (decide puts
            // the defender first).
            let exposed = g.game_turn() > 30 && defenders < 2 && second_defender_wanted(g, p, c, plot, defenders, local_threat(g, p, plot, 3), defense);
            if exposed && !building_fighter && g.city_production_turns_left(c) > 3 && !g.player_is_barbarian(p) && best_unit(g, c, Role::Defense).is_some() && switch_gets_fighter(ai, g, c) {
                crate::trace!(g, p, "{}: second defender first", g.cities[c].name);
                g.city_clear_order_queue(c);
                choose(ai, g, c);
            }
            continue;
        }
        if building_fighter || defense * 3 / 2 >= threat && defenders > 0 {
            continue;
        }
        if !switch_gets_fighter(ai, g, c) {
            crate::trace!(g, p, "{} under threat {} (defense {}, {} defenders): its garrison holds, the build goes on", g.cities[c].name, threat, defense, defenders);
            continue;
        }
        crate::trace!(g, p, "{} under threat {} (defense {}): switching to defense", g.cities[c].name, threat, defense);
        g.city_clear_order_queue(c);
        choose(ai, g, c);
    }
}

/// Would clearing city `c`'s queue get a fighter: does [`decide`] put one first? (It does not read
/// the queue.) Always true with switch `EMERGENCY_DECIDE` off: the switch then clears the queue
/// whatever comes next.
fn switch_gets_fighter(ai: &Ai, g: &Game, c: CityId) -> bool {
    if !crate::switches::EMERGENCY_DECIDE.on(g, g.cities[c].owner) {
        return true;
    }
    matches!(decide(ai, g, c), Some((Choice::Train(u, _), _)) if g.d.units[u].combat > 0)
}

/// Last resort when the priority list found nothing: the cheapest useful building, else the
/// strongest land unit per hammer, else any process.
fn fallback(g: &Game, c: CityId) -> Option<Choice> {
    let d = g.d;
    let mut best: Option<(BuildingType, i32)> = None;
    for b in d.buildings.keys() {
        if !building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0);
        if best.map_or(true, |(_, t)| turns < t) {
            best = Some((b, turns));
        }
    }
    if let Some((b, _)) = best {
        return Some(Choice::Construct(b));
    }
    // Nothing left to build: workers (they pay for themselves) before more soldiers.
    let p = g.cities[c].owner;
    if count_ai(g, p, UnitAIType::UNITAI_WORKER) < 2 * g.players[p].cities.len() as i32 {
        if let Some(u) = best_unit(g, c, Role::Worker) {
            return Some(Choice::Train(u, UnitAIType::UNITAI_WORKER));
        }
    }
    // Field units (they hunt and campaign) rather than garrisons the treasury cannot carry.
    // When the treasury cannot carry more, the strongest one (support costs the same for any
    // unit, and the weakest are the ones disbanded): the hammers buy quality, not numbers.
    let at_war = civ_enemies_at_war(g, p);
    let unit = if crate::economy::can_afford_unit(g, p, true, at_war) { best_unit(g, c, Role::Attack) } else { strongest_unit(g, c, Role::Attack) };
    if let Some(u) = unit {
        return Some(Choice::Train(u, UnitAIType::UNITAI_ATTACK));
    }
    if let Some(u) = best_unit(g, c, Role::Defense) {
        return Some(Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE));
    }
    d.processes.keys().find(|&pr| g.city_can_maintain(c, pr, false)).map(Choice::Maintain)
}

/// Are some of our field units standing next to a hostile city (a siege under way)?
fn besieging(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    g.players[p].units.iter().any(|&u| {
        let unit = &g.units[u];
        if !matches!(unit.ai_type, Some(UnitAIType::UNITAI_ATTACK) | Some(UnitAIType::UNITAI_ATTACK_CITY)) {
            return false;
        }
        (0..DirectionType::NUM).any(|dir| {
            g.plot_direction(unit.x, unit.y, DirectionType::from_usize(dir))
                .and_then(|q| g.map.plots[q].plot_city)
                .is_some_and(|c| g.at_war(team, g.players[g.cities[c].owner].team))
        })
    })
}

/// Our units on the city plot able to defend it, and their summed defense power.
pub fn garrison(g: &Game, p: PlayerId, plot: PlotIdx) -> (i32, i32) {
    let mut n = 0;
    let mut power = 0;
    for &u in &g.map.plots[plot].units {
        let unit = &g.units[u];
        if unit.owner != p {
            continue;
        }
        let info = &g.d.units[unit.unit_type];
        if info.combat_defense <= 0 && info.combat <= 0 {
            continue;
        }
        if matches!(unit.ai_type, Some(UnitAIType::UNITAI_CITY_DEFENSE) | Some(UnitAIType::UNITAI_CITY_COUNTER) | Some(UnitAIType::UNITAI_CITY_SPECIAL)) || info.only_defensive {
            n += 1;
            power += crate::view::defense_power(g, u);
        }
    }
    (n, power)
}

/// Summed attack power of enemy units within `range` plots, fog included
/// ([`crate::view::known_threat`]).
pub fn local_threat(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> i32 {
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    let mut threat = 0;
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            for &u in &g.map.plots[q].units {
                if !crate::view::known_threat(g, p, u) {
                    continue;
                }
                let ut = g.units[u].owner;
                let their_team = g.players[ut].team;
                // Animals cannot enter owned land: no threat to a city.
                if g.d.units[g.units[u].unit_type].animal {
                    continue;
                }
                if their_team != team && g.at_war(team, their_team) {
                    // Nearer units weigh more (distance 1: full, 3: a third).
                    let dist = dx.abs().max(dy.abs()).max(1);
                    threat += crate::view::attack_power(g, u) * (range + 1 - dist) / range;
                }
            }
        }
    }
    threat
}

/// A revealed barbarian lair or a (non-animal) barbarian unit within `range` plots.
pub fn barbarians_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_player();
    // Barbarians we are at peace with (the Clan of Embers, Charadon's Doviello) count too: their
    // garrison term holds the defenders that keep the Clan's cities from its rivals (docs/AI_LOG.md,
    // "Allied barbarians no longer counted as a threat"). Raiders in sight count, not only lairs:
    // a quota from lairs and towns alone lost cities to the barbarians, and one from a per-city
    // record of the raiders seen cost research for no city saved (docs/AI_LOG.md, "Garrison
    // quota from lasting threats only", "Garrison quota from a record of threats seen").
    let (x, y) = g.map.xy(plot);
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let pl = &g.map.plots[q];
            if pl.revealed[team] && pl.improvement_type.is_some_and(|i| g.d.improvements[i].spawn_unit_type.is_some_and(|u| !g.d.units[u].animal)) {
                return true;
            }
            // Units: fog included (`view::known_threat`).
            if pl.units.iter().any(|&u| g.units[u].owner == barb && !g.d.units[g.units[u].unit_type].animal && crate::view::known_threat(g, p, u)) {
                return true;
            }
        }
    }
    false
}

/// The peacetime army target of `decide` (step 8): attackers for `n_cities` cities and the
/// leader's military flavour.
fn army_target_for(n_cities: i32, mil_flavor: i32) -> i32 {
    n_cities * (2 + mil_flavor / 5) / 2 + 1
}

/// Player `p`'s peacetime army target (step 8 of `decide`, before war, finances and
/// affordability adjust it).
pub(crate) fn army_target(g: &Game, p: PlayerId) -> i32 {
    let d = g.d;
    let mil_flavor = match (g.players[p].personality_type, d.ty::<FlavorType>("FLAVOR_MILITARY")) {
        (Some(l), Some(f)) => d.leader_heads[l].flavor_value[f],
        _ => 0,
    };
    army_target_for(g.players[p].cities.len() as i32, mil_flavor)
}

/// A barbarian lair (an improvement that spawns non-animal units) within `range` plots: the
/// lasting part of [`barbarians_near`].
pub fn barbarian_lair_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let pl = &g.map.plots[q];
            if pl.revealed[team] && pl.improvement_type.is_some_and(|i| g.d.improvements[i].spawn_unit_type.is_some_and(|u| !g.d.units[u].animal)) {
                return true;
            }
        }
    }
    false
}

/// Units trained are kept (`finance::disband_military`): does the garrison
/// of an interior city hold the barbarians it sees, when one more defender would cost gold?
/// At war with the barbarians, the visible threat within 3 plots (`threat`, [`local_threat`]) no
/// more than the garrison's defense (`defense`), nothing in sight included; no unit of a
/// civilization at war with us within 3 plots; no city of another civilization within 8 plots;
/// no free unit slot left. Then a raider in sight adds no defender to the city's quota: under
/// More Naval AI's rule a defender trained against a raider the garrison holds is kept and paid
/// for long after the raider has gone (docs/AI_LOG.md, "More Naval AI's disband rule"). A border
/// city keeps the reactive defender: the defenders trained against raiders are also the ones
/// that hold it when a neighbour attacks (the same rule in every city lost civ cities to
/// rivals). While units are free a defender costs nothing to keep. Lairs still count, and so do
/// the barbarians of a civilization at peace with them (the Clan of Embers' garrison term,
/// docs/AI_LOG.md, "Allied barbarians no longer counted as a threat").
pub(crate) fn garrison_holds(g: &Game, p: PlayerId, plot: PlotIdx, threat: i32, defense: i32) -> bool {
    g.at_war(g.players[p].team, g.barbarian_team())
        && threat <= defense
        && crate::economy::marginal_unit_cost_x100(g, p, true) > 0
        && !civ_enemy_near(g, p, plot, 3)
        && !rival_city_near(g, p, plot, 8)
}

/// A unit of a civilization we are at war with within `range` plots, fog included
/// ([`crate::view::known_threat`]).
fn civ_enemy_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    let (x, y) = g.map.xy(plot);
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            if g.map.plots[q].units.iter().any(|&u| {
                let t = g.players[g.units[u].owner].team;
                t != team && t != barb && g.at_war(team, t) && crate::view::known_threat(g, p, u)
            }) {
                return true;
            }
        }
    }
    false
}

/// A city of another civilization (not a vassal of ours, not the barbarians) that we have seen,
/// within `range` plots: a border city.
fn rival_city_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    g.player_ids().any(|o| {
        let t = g.players[o].team;
        t != team
            && !g.player_is_barbarian(o)
            && g.players[o].alive
            && !g.teams[t].vassal[team]
            && g.players[o].cities.iter().any(|&c| {
                let city = &g.cities[c];
                g.plot_distance(x, y, city.x, city.y) <= range && g.map.plots[g.city_plot(c)].revealed[team]
            })
    })
}

/// The second defender after turn 30: in the capital, and in a city near barbarians (a lair
/// within 4 plots; barbarian units in sight there unless the garrison holds them,
/// [`garrison_holds`]).
fn second_defender_wanted(g: &Game, p: PlayerId, c: CityId, plot: PlotIdx, defenders: i32, threat: i32, defense: i32) -> bool {
    g.game_turn() > 30
        && defenders < 2
        && (g.city_is_capital(c) || barbarian_lair_near(g, p, plot, 4) || !garrison_holds(g, p, plot, threat, defense) && barbarians_near(g, p, plot, 4))
}

pub fn count_ai(g: &Game, p: PlayerId, unit_ai: UnitAIType) -> i32 {
    let pl = &g.players[p];
    pl.ai_num_ai_units[unit_ai] + pl.ai_num_train_ai_units[unit_ai]
}

/// The workers our land needs (step 6 of `decide`): one a city up to three, one more for every
/// three worked plots without an improvement, at most two a city.
pub(crate) fn worker_target(g: &Game, p: PlayerId) -> i32 {
    let n_cities = g.players[p].cities.len() as i32;
    (n_cities.min(3) + (unimproved_worked_plots(g, p) + 1) / 3).min(n_cities * 2).max(1)
}

/// Can our workers build anything useful yet (an improvement or route whose tech we have)?
pub fn has_worker_builds(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    g.d.builds.iter().any(|b| (b.improvement.is_some() || b.route.is_some()) && b.tech_prereq.map_or(true, |t| g.team_is_has_tech(team, Some(t))))
}

/// Water resources in this city's radius (our borders) that a work boat could improve now.
pub fn sea_resources_unimproved(g: &Game, c: CityId) -> i32 {
    let p = g.cities[c].owner;
    let team = g.players[p].team;
    let d = g.d;
    let mut n = 0;
    for i in 0..NUM_CITY_PLOTS {
        let Some(q) = g.city_index_plot(c, i) else { continue };
        let plot = &g.map.plots[q];
        if plot.owner != Some(p) || plot.improvement_type.is_some() || !g.plot_is_water(q) {
            continue;
        }
        let Some(bonus) = plot.bonus_type else { continue };
        if d.bonuses[bonus].tech_reveal.is_some_and(|t| !g.team_is_has_tech(team, Some(t))) {
            continue;
        }
        let ok = d.builds.iter_enumerated().any(|(b, info)| {
            info.improvement.is_some_and(|imp| d.improvements[imp].improvement_bonus[bonus].bonus_trade) && g.player_can_build(p, q, b, false, false)
        });
        if ok {
            n += 1;
        }
    }
    n
}

/// Plots worked by our cities that have no improvement (a measure of worker demand). Plots no
/// known build improves yet count too: the worker that waits for the tech is the one ready the
/// turn it comes (docs/AI_LOG.md, "Worker demand only where a build is possible now").
pub fn unimproved_worked_plots(g: &Game, p: PlayerId) -> i32 {
    let mut n = 0;
    for &c in &g.players[p].cities {
        for i in 0..NUM_CITY_PLOTS {
            if i == CITY_HOME_PLOT || !g.cities[c].working_plot[i] {
                continue;
            }
            if let Some(q) = g.city_index_plot(c, i) {
                let plot = &g.map.plots[q];
                if plot.improvement_type.is_none() && plot.plot_type != PlotType::PLOT_OCEAN {
                    n += 1;
                }
            }
        }
    }
    n
}

/// Buildings the AI never builds by itself: a new palace (moves the capital), buildings with
/// no effect but a cost the generic valuation cannot see the downside of.
fn building_allowed(g: &Game, c: CityId, b: BuildingType) -> bool {
    let info = &g.d.buildings[b];
    if info.capital {
        // Only if we have lost our palace.
        return g.players[g.cities[c].owner].capital_city.map_or(true, |cap| !g.cities.contains(cap));
    }
    true
}

fn civ_enemies_at_war(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t))
}

/// Is `c` in the top third of our cities by production (the best one at least), among those
/// for which `eligible` holds (the cities that could build the item at all: a hero ship needs a
/// coast)? More Naval AI builds world wonders and heroes only there
/// (`CvCityAI::AI_chooseProduction`: `iProductionRank <= max(1, iNumCities / 3)`); the rules let
/// one city at a time build each, so the first empty queue otherwise locks it in. Ties go to the
/// older city, as in `victory::ranked_by_production`.
pub(crate) fn top_production_city(g: &Game, c: CityId, eligible: impl Fn(CityId) -> bool) -> bool {
    let p = g.cities[c].owner;
    let mine = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
    let mut n = 0;
    let mut ahead = 0;
    for &o in &g.players[p].cities {
        if o != c && !eligible(o) {
            continue;
        }
        n += 1;
        let prod = g.city_yield_rate(o, YieldType::YIELD_PRODUCTION);
        if prod > mine || prod == mine && o < c {
            ahead += 1;
        }
    }
    ahead < (n / 3).max(1)
}

/// A world wonder (one in the world, `isWorldWonderClass`).
pub(crate) fn is_world_wonder(d: &GameData, b: BuildingType) -> bool {
    d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_world_wonder_class(d, k))
}

/// The owner cannot pay for more: research slider pushed down to cover costs.
pub fn financial_trouble(g: &Game, p: PlayerId) -> bool {
    g.player_is_commerce_flexible(p, CommerceType::COMMERCE_RESEARCH) && crate::economy::spend_pct(g, p) < 40 && g.players[p].cities.len() > 1
}

/// City state used by the decision.
struct Ctx {
    p: PlayerId,
    n_cities: i32,
    capital: bool,
    pop: i32,
    threat: i32,
    defenders: i32,
    defense: i32,
    food_surplus: i32,
    production: i32,
    happy_room: i32,
    at_war: bool,
    broke: bool,
    /// One more military unit is affordable (see `economy::can_afford_unit`).
    afford: bool,
}

fn decide(ai: &Ai, g: &Game, c: CityId) -> Option<(Choice, &'static str)> {
    let d = g.d;
    let city = &g.cities[c];
    let p = city.owner;
    let pl = &g.players[p];
    let plot = g.city_plot(c);
    let barbarian = g.player_is_barbarian(p);
    let leader = pl.personality_type.map(|l| &d.leader_heads[l]);
    let (defenders, defense) = garrison(g, p, plot);
    let ctx = Ctx {
        p,
        n_cities: pl.cities.len() as i32,
        capital: g.city_is_capital(c),
        pop: city.population,
        threat: local_threat(g, p, plot, 3),
        defenders,
        defense,
        food_surplus: g.city_yield_rate(c, YieldType::YIELD_FOOD) - g.city_food_consumption(c, false, 0),
        production: g.city_yield_rate(c, YieldType::YIELD_PRODUCTION),
        happy_room: g.city_happy_level(c) - g.city_unhappy_level(c, 0),
        at_war: civ_enemies_at_war(g, p) || crate::diplomacy::preparing_target(g, pl.team).is_some(),
        broke: financial_trouble(g, p),
        afford: false,
    };
    let ctx = Ctx { afford: crate::economy::can_afford_unit(g, p, true, ctx.at_war), ..ctx };
    crate::trace!(g, p, "{}: threat {} defenders {} ({}) food {:+} prod {} happy {}", city.name, ctx.threat, ctx.defenders, ctx.defense, ctx.food_surplus, ctx.production, ctx.happy_room);
    if ctx.threat > 0 && crate::trace_on(p) {
        let (x, y) = g.map.xy(plot);
        for dx in -3..=3 {
            for dy in -3..=3 {
                let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                for &u in &g.map.plots[q].units {
                    if g.units[u].owner != p {
                        crate::trace!(g, p, "   threat unit {} owner {:?} at ({},{}) impr {:?} visible {}", d.units[g.units[u].unit_type].base.type_, g.units[u].owner, dx, dy, g.map.plots[q].improvement_type.map(|i| d.improvements[i].base.type_.clone()), g.plot_is_visible(q, pl.team));
                    }
                }
            }
        }
    }
    let flavor = |name: &str| match (leader, d.ty::<FlavorType>(name)) {
        (Some(l), Some(f)) => l.flavor_value[f],
        _ => 0,
    };
    let mil_flavor = flavor("FLAVOR_MILITARY");

    // Garrison quota: one, the capital and threatened cities more, cities near barbarian lairs
    // or barbarians more.
    // A raider in sight the garrison holds adds nothing (`garrison_holds`).
    let held = garrison_holds(g, p, plot, ctx.threat, ctx.defense);
    let mut needed = 1 + i32::from(ctx.capital && (ctx.n_cities > 1 || g.game_turn() > 30)) + i32::from(ctx.threat > 0 && !held);
    if barbarian_lair_near(g, p, plot, 3) || !held && barbarians_near(g, p, plot, 3) {
        needed += 1;
    }
    // The rival civilizations near this city (war, hostility, power; defense.rs): at war the
    // front holds more and the interior fewer than a flat one more in every city (the rule before
    // round 7e, switch `RIVAL_DEFENSE` off).
    let rival = crate::defense::rival_defenders(g, c);
    needed += rival;
    if let Some(l) = leader {
        if l.ai_value_defense > 100 && ctx.n_cities > 3 {
            needed += 1;
        }
    }

    // 1. Missing or overwhelmed garrison.
    // Fortified defenders in a city fight at about 1.5x their strength.
    if ctx.defenders == 0 || (ctx.threat > 0 && ctx.defense * 3 / 2 < ctx.threat && ctx.defenders < needed + 2) {
        if let Some(u) = best_unit(g, c, Role::Defense) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), "garrison"));
        }
    }
    if barbarian {
        return best_unit(g, c, Role::Attack).map(|u| (Choice::Train(u, UnitAIType::UNITAI_ATTACK), "barbarian"));
    }

    let workers = count_ai(g, p, UnitAIType::UNITAI_WORKER);
    let can_improve = has_worker_builds(g, p);

    // Threats the garrison can hold (lair guards next door, say) do not stop expansion.
    let safe = ctx.threat * 2 <= ctx.defense;

    // 2. The first worker, once workers have something to do.
    if workers == 0 && can_improve && safe && (ctx.pop >= 2 || ctx.food_surplus <= 1) {
        if let Some(u) = best_unit(g, c, Role::Worker) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_WORKER), "first_worker"));
        }
    }

    // A second defender before more settlers once the barbarians are about.
    if second_defender_wanted(g, p, c, plot, ctx.defenders, ctx.threat, ctx.defense) {
        if let Some(u) = best_unit(g, c, Role::Defense) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), "second_defender"));
        }
    }

    // A victory a few turns away comes before expansion (victory.rs).
    if safe {
        if let Some(choice) = crate::victory::production(g, c, true) { return Some((choice, "victory_soon")); } // victory-pursuit
    }

    // An explorer while much of our landmass is unknown (sites, goody huts, contact with
    // the neighbours).
    if ctx.capital && ctx.threat == 0 && (5..150).contains(&g.game_turn()) && count_ai(g, p, UnitAIType::UNITAI_EXPLORE) == 0 {
        let area = g.map.plots[plot].area;
        if let Some(a) = area {
            let unknown = g.area_get_num_unrevealed_tiles(a, pl.team);
            if unknown * 5 > g.map.areas[a].num_tiles && unknown > 20 {
                if let Some(u) = best_unit(g, c, Role::Explore).or_else(|| best_unit(g, c, Role::Attack)) {
                    if g.city_production_turns_left_unit(c, u, 0) <= 10 {
                        return Some((Choice::Train(u, UnitAIType::UNITAI_EXPLORE), "explorer"));
                    }
                }
            }
        }
    }

    // 3. Settlers while good sites remain in reach.
    let settler = crate::expansion::settler_choice(ai, g, c, ctx.p, ctx.threat, ctx.defense); // early-expansion
    if let SettlerPlan::Now(choice) = settler {
        return Some((choice, "settler"));
    }
    if let Some(choice) = crate::civs::production(ai, g, c, ctx.threat) { return Some((choice, "civ_mechanics")); } // civ-mechanics
    if let Some(choice) = crate::traits::production(g, c, ctx.threat) { return Some((choice, "trait_play")); } // trait-play
    // An assault fleet (transport + attackers) for an overseas enemy city we can take.
    if safe && crate::naval::is_coastal_city(g, c) {
        if let Some(choice) = crate::naval::assault_production(g, c) {
            return Some((choice, "assault_fleet"));
        }
    }
    // A ship for a settler waiting in port; a sea explorer while our land is full and
    // nothing is known across the water.
    if safe && crate::naval::is_coastal_city(g, c) {
        if crate::naval::settler_waiting(g, p).is_some() && crate::naval::num_transports(g, p) == 0 {
            if let Some(u) = best_unit(g, c, Role::Transport) {
                return Some((Choice::Train(u, UnitAIType::UNITAI_SETTLER_SEA), "settler_ship"));
            }
        }
        if count_ai(g, p, UnitAIType::UNITAI_EXPLORE_SEA) == 0
            && crate::naval::home_is_full(g, p)
            && sites::overseas_sites(ai, g, p).is_empty()
            && g.game_turn() % 20 < 10
            && ctx.afford
        {
            let unknown = g.map.num_plots() as i32 - g.map.areas.iter().map(|(_, a)| a.num_revealed_tiles[pl.team]).sum::<i32>();
            if unknown * 5 > g.map.num_plots() as i32 {
                if let Some(u) = best_unit(g, c, Role::SeaExplore) {
                    return Some((Choice::Train(u, UnitAIType::UNITAI_EXPLORE_SEA), "sea_explorer"));
                }
            }
        }
    }
    // Work boats for sea resources nobody improves yet.
    if safe && sea_resources_unimproved(g, c) > 0 {
        let need: i32 = pl.cities.iter().map(|&o| sea_resources_unimproved(g, o)).sum();
        if count_ai(g, p, UnitAIType::UNITAI_WORKER_SEA) < need.min(ctx.n_cities) {
            if let Some(u) = best_unit(g, c, Role::SeaWorker) {
                return Some((Choice::Train(u, UnitAIType::UNITAI_WORKER_SEA), "work_boat"));
            }
        }
    }
    // Growing toward a settler: nothing that ties the city up for long.
    let max_turns = if settler == SettlerPlan::Soon { 12 } else { 50 };

    // FfH tier-2 units need a building in the training city (Archery Range for archers,
    // Training Yard for axemen, ...): build it when it opens a clearly better unit.
    if safe && ctx.pop >= 2 {
        if let Some(b) = enabler_building(g, c, ctx.threat) {
            return Some((Choice::Construct(b), "unit_enabler"));
        }
    }

    // 4. Extra defenders up to the quota (a broke empire keeps one per city unless threatened).
    if ctx.broke && ctx.threat == 0 {
        needed = needed.min(1 + i32::from(ctx.capital));
    }
    if !ctx.afford && ctx.threat == 0 {
        needed = needed.min(1 + i32::from(ctx.capital));
    }
    needed += crate::victory::guard(g, c); // victory-pursuit
    if ctx.defenders < needed {
        if let Some(u) = best_unit(g, c, Role::Defense) {
            let why = if ctx.defenders >= needed - rival { "rival_defender" } else { "defender_quota" };
            return Some((Choice::Train(u, UnitAIType::UNITAI_CITY_DEFENSE), why));
        }
    }

    // 5. A small mobile force to clear barbarians and animals near our lands.
    let attackers = count_ai(g, p, UnitAIType::UNITAI_ATTACK) + count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL);
    let min_army = if ctx.broke { 1 + ctx.n_cities / 4 } else { 1 + ctx.n_cities / 2 };
    if attackers < min_army && (ctx.production >= 3 || settler == SettlerPlan::Soon) && (ctx.afford || attackers == 0) {
        if let Some(u) = best_unit(g, c, Role::Attack) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_ATTACK), "barbarian_force"));
        }
    }

    // 6. More workers, as the land we work needs improving.
    if can_improve && safe {
        let target = worker_target(g, p);
        if workers < target && ctx.pop >= 2 {
            if let Some(u) = best_unit(g, c, Role::Worker) {
                return Some((Choice::Train(u, UnitAIType::UNITAI_WORKER), "workers"));
            }
        }
    }

    // Victory pursuit (victory.rs): the towers in our best cities, culture in the cities
    // nearest legendary -- after the garrison, the first army and the workers.
    if safe {
        if let Some(choice) = crate::victory::production(g, c, false) { return Some((choice, "victory")); } // victory-pursuit
    }

    // Mana nodes (MANA_NODES) and the caster corps (casters.rs): a caster to convert our raw mana
    // when none of ours can, and the corps' casters (mana.rs, casters.rs).
    if safe && ctx.afford && !ctx.broke {
        if let Some(choice) = crate::mana::production(g, c) {
            return Some((choice, "mana_builder"));
        }
        if let Some(choice) = crate::casters::production(g, c, ctx.at_war, attackers) {
            return Some((choice, "caster_corps"));
        }
    }

    // A wonder slot in our most productive cities at peace (More Naval AI's quick wonder).
    if safe && settler != SettlerPlan::Soon {
        if let Some(b) = wonder_slot(g, c, ctx.at_war) {
            return Some((Choice::Construct(b), "wonder_slot"));
        }
    }

    // 7. Heroes are unique and strong (but not at the cost of the first expansion); a coastal city
    // trains a hero ship (the Lanun's Black Wind, which serves as a transport). Only the top third
    // of our cities by production that can train it builds one (`TOP_CITIES`).
    let top_cities = crate::switches::TOP_CITIES.on(g, p);
    if let Some(u) = best_unit(g, c, Role::Hero) {
        let best_city = !top_cities || top_production_city(g, c, |o| g.city_can_train(o, u, false, false, false, false));
        if best_city && (g.city_production_turns_left_unit(c, u, 0) <= max_turns.min(25) || ctx.n_cities >= 3) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_HERO), "hero"));
        }
    }
    if let Some(u) = best_unit(g, c, Role::SeaHero) {
        let best_city = !top_cities || top_production_city(g, c, |o| g.city_can_train(o, u, false, false, false, false));
        if best_city && (g.city_production_turns_left_unit(c, u, 0) <= max_turns.min(25) || ctx.n_cities >= 3) {
            return Some((Choice::Train(u, d.units[u].default_unit_ai_type.unwrap_or(UnitAIType::UNITAI_ATTACK_SEA)), "hero_ship"));
        }
    }

    // 8. Army target (larger at war).
    let mut army_target = army_target_for(ctx.n_cities, mil_flavor);
    if ctx.at_war {
        army_target = army_target * 2 + 2;
    } else if ctx.broke {
        army_target = min_army;
    }
    if !ctx.afford {
        army_target = army_target.min(attackers);
    }
    let army_short = attackers < army_target;
    // At war (or preparing): a third of the army in siege engines. A stack camped next to a
    // city it cannot take (a barbarian town, say) asks for a couple as well.
    let siege = count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL);
    if let Some(choice) = crate::campaign::army_choice(g, c, attackers - siege, siege, ctx.afford) { return Some((choice, "campaign_army")); } // siege-campaigns
    if (ctx.at_war && army_short && siege * 3 < attackers + 1) || (siege < 2 && besieging(g, p) && ctx.afford) {
        if let Some(u) = best_unit(g, c, Role::Siege) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_ATTACK_CITY), "siege"));
        }
    }
    // 9. Buildings by marginal value per turn of production. Computed BEFORE the surplus-army
    // interleave below so a high-value economy building (a research/commerce multiplier) is not
    // starved while the army is short: at war the interleave would otherwise take every turn and
    // this loop was never reached (measured: libraries in ~1 of 6-8 cities, research = commerce x
    // slider with no multiplier). The operation's real strike force (army_choice above) and every
    // garrison are upstream, so only the non-converting attacker tail beyond the army target is
    // deferred -- a few turns per city, until the economy buildings are up.
    let mut best_building: Option<(BuildingType, i32)> = None;
    let mut best_economy: Option<(BuildingType, i32)> = None;
    // With `TOP_CITIES`, a city outside the top third of ours by production takes a world wonder
    // only if it finishes it as quickly as the wonder slot asks (2 x cities + 1 turns): the rules
    // let one city at a time build it, and a slow city that starts one first locks it away from
    // the cities that would finish it sooner.
    let mut wonder_city: Option<bool> = if top_cities { None } else { Some(true) };
    let quick_wonder = 2 * ctx.n_cities + 1;
    for b in d.buildings.keys() {
        if !building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let value = building_value(g, c, b, ctx.threat);
        if value <= 0 {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        // Nothing that ties a city up for ages (wonders belong in productive cities).
        if turns > max_turns {
            continue;
        }
        if turns > quick_wonder && is_world_wonder(d, b) && !*wonder_city.get_or_insert_with(|| top_production_city(g, c, |_| true)) {
            continue;
        }
        let score = value * 100 / (turns + 3);
        if best_building.map_or(true, |(_, s)| score > s) {
            best_building = Some((b, score));
        }
        if is_economy_building(g, c, b, ctx.threat) && best_economy.map_or(true, |(_, s)| score > s) {
            best_economy = Some((b, score));
        }
    }
    // A high-value economy building comes ahead of the surplus-army interleave, but ONLY for a
    // civ whose finances are actually strained by upkeep (support-heavy: unit support >= a tenth
    // of commerce with research already pushed below its target; or a structural deficit). Those
    // are the research-suppressed laggards the de-starvation is for; a healthy civ (research at or
    // above target) keeps building its army, so conquest is preserved. (Measured: gating on any
    // available economy building instead raised research but cut captures ~25% and shrank the
    // biggest empire -- it dampened healthy conquerors too; this condition keeps the research gain
    // where economy is the real bind without trading away the conquest.)
    if army_short {
        if let Some((b, score)) = best_economy {
            if score >= ECON_BUILD_BAR {
                let f = crate::finance::state(g, p);
                if f.support_heavy() || f.structural() {
                    return Some((Choice::Construct(b), "economy_building_strained"));
                }
            }
        }
    }

    // Interleave army and buildings in peacetime (deterministic: no RNG draw from the AI).
    if army_short && (ctx.at_war || (g.game_turn() + c.0 as i32) % 3 == 0) {
        if let Some(u) = best_unit(g, c, if ctx.at_war { Role::CityAttack } else { Role::Attack }) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_ATTACK), "army_interleave"));
        }
    }
    // A missionary for a foreign city while our shrine pays a gold a turn for every city with the
    // religion (MISSIONARY_ABROAD): weighed like a building worth that gold (`building_value`'s
    // 80 a gold, less the odds of failing on the way), against the best building.
    if let Some(r) = crate::units::abroad_religion(g, p).filter(|&r| g.player_get_state_religion(p) == Some(r)) {
        if count_ai(g, p, UnitAIType::UNITAI_MISSIONARY) == 0 && abroad_targets(g, p, c, r) {
            if let Some(u) = best_missionary(g, c, r) {
                let score = MISSIONARY_ABROAD_VALUE * 100 / (g.city_production_turns_left_unit(c, u, 0).max(1) + 3);
                if best_building.map_or(true, |(_, s)| score > s) && (score >= ECON_BUILD_BAR || !army_short) {
                    return Some((Choice::Train(u, UnitAIType::UNITAI_MISSIONARY), "missionary_abroad"));
                }
            }
        }
    }
    if let Some((b, score)) = best_building {
        if score >= ECON_BUILD_BAR || !army_short {
            return Some((Choice::Construct(b), "building_interleave"));
        }
    }
    if settler == SettlerPlan::Soon && ctx.afford {
        // Cheap units while the city grows: they explore, escort and hunt.
        if let Some(u) = best_unit(g, c, Role::Attack) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_ATTACK), "army"));
        }
    }

    // 10. Casters and missionaries.
    let mages = count_ai(g, p, UnitAIType::UNITAI_MAGE) + count_ai(g, p, UnitAIType::UNITAI_WARWIZARD);
    let mage_target = (ctx.n_cities + 2) / 4 + leader.map_or(0, |l| i32::from(l.ai_value_mage > 100));
    if mages < mage_target && ctx.afford {
        if let Some(u) = best_unit(g, c, Role::Mage) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_MAGE), "caster"));
        }
    }
    if let Some(r) = g.player_get_state_religion(p) {
        let missing = pl.cities.iter().filter(|&&cc| !g.cities[cc].has_religion[r]).count() as i32;
        if missing > 0 && count_ai(g, p, UnitAIType::UNITAI_MISSIONARY) == 0 {
            if let Some(u) = best_missionary(g, c, r) {
                return Some((Choice::Train(u, UnitAIType::UNITAI_MISSIONARY), "missionary"));
            }
        }
    }

    if army_short {
        if let Some(u) = best_unit(g, c, Role::Attack) {
            return Some((Choice::Train(u, UnitAIType::UNITAI_ATTACK), "attacker"));
        }
    }
    if let Some((b, _)) = best_building {
        return Some((Choice::Construct(b), "building"));
    }
    if let Some(pr) = best_project(g, c) {
        return Some((Choice::Create(pr), "project"));
    }
    // 11. Convert production to research (else gold).
    let mut best_process: Option<(ProcessType, i32)> = None;
    for pr in d.processes.keys() {
        if !g.city_can_maintain(c, pr, false) {
            continue;
        }
        let m = &d.processes[pr].production_to_commerce_modifier;
        let v = m[CommerceType::COMMERCE_RESEARCH.idx()] * 2 + m[CommerceType::COMMERCE_GOLD.idx()] * 2 + m[CommerceType::COMMERCE_CULTURE.idx()];
        if best_process.map_or(true, |(_, s)| v > s) {
            best_process = Some((pr, v));
        }
    }
    best_process.map(|(pr, _)| (Choice::Maintain(pr), "process"))
}

/// More Naval AI's quick wonder (`CvCityAI::AI_chooseProduction`, `BUILDINGFOCUS_WORLDWONDER`):
/// at peace, a city in the top third of ours by production builds the world wonder of most value
/// per turn that it finishes within 2 x cities + 1 turns (four times that in a city making more
/// than three hammers a citizen at size 6+). FfH's wonders carry culture, the cultural victory's
/// currency; hammers taken from the army cost cities (docs/AI.md, "Looked promising on
/// development seeds, failed on fresh ones": buildings valued for more effects), so the slot is
/// only for the best cities and only at peace.
fn wonder_slot(g: &Game, c: CityId, at_war: bool) -> Option<BuildingType> {
    let p = g.cities[c].owner;
    if at_war || !crate::switches::WONDER_SLOT.on(g, p) || !top_production_city(g, c, |_| true) {
        return None;
    }
    let d = g.d;
    let horizon = wonder_horizon(g, c);
    let mut best: Option<(BuildingType, i32)> = None;
    for b in d.buildings.keys() {
        if !is_world_wonder(d, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        if turns > horizon {
            continue;
        }
        let value = building_value(g, c, b, 0);
        if value <= 0 {
            continue;
        }
        let score = value * 100 / (turns + 3);
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((b, score));
        }
    }
    best.map(|(b, _)| b)
}

/// The wonder slot's horizon in city `c`: 2 x cities + 1 turns, four times that in a city of size
/// 6+ making more than three hammers a citizen (More Naval AI's `iWonderTime`, "SuperCity").
fn wonder_horizon(g: &Game, c: CityId) -> i32 {
    let p = g.cities[c].owner;
    let pop = g.cities[c].population;
    let super_city = pop > 5 && g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) > pop * 3;
    (2 * g.players[p].cities.len() as i32 + 1) * if super_city { 4 } else { 1 }
}

/// With `WONDER_SLOT`, once a turn: a city in the top third of ours by production pushing a
/// world wonder home at peace -- done within the wonder slot's horizon ([`wonder_horizon`]) --
/// emphasizes production (More Naval AI's special yield multiplier,
/// `CvCityAI::AI_updateSpecialYieldMultiplier`: production +50% while building a wonder; here
/// the player's `EMPHASIZE_PRODUCTION`, which the governor weighs the same), and drops it once
/// the wonder is done, switched or at war. On every world wonder for the whole build, as More
/// Naval AI does, it cost growth and commerce for few hammers (seeds 1-48, 300 turns: population
/// -1.3% and commerce -1.3% at turn 200, research -4.3% at turn 300, against -0.1%, -0.2% and
/// -1.4% for the slot without it, about the same wonders). The AI sets this emphasis for nothing
/// else.
pub fn wonder_emphasis(g: &mut Game, p: PlayerId) {
    static EMPHASIZE: OnceLock<Option<EmphasizeType>> = OnceLock::new();
    let Some(e) = *EMPHASIZE.get_or_init(|| g.d.ty::<EmphasizeType>("EMPHASIZE_PRODUCTION")) else { return };
    if !crate::switches::WONDER_SLOT.on(g, p) {
        return;
    }
    let peace = !civ_enemies_at_war(g, p) && crate::diplomacy::preparing_target(g, g.players[p].team).is_none();
    for c in g.players[p].cities.clone() {
        let want = peace
            && g.city_production_building(c).is_some_and(|b| is_world_wonder(g.d, b))
            && g.city_production_turns_left(c) <= wonder_horizon(g, c)
            && top_production_city(g, c, |_| true);
        if g.cities[c].ai_emphasize[e] != want {
            crate::trace!(g, p, "{}: production emphasis {}", g.cities[c].name, if want { "on (wonder)" } else { "off" });
            g.city_ai_set_emphasize(c, e, want);
        }
    }
}

/// With `TROUBLE_REBUILD`, once a turn while the empire cannot research at its target share
/// (`finance::Finance::deficit`): More Naval AI re-decides every city's production when the
/// empire runs into financial trouble (`CvPlayerAI::AI_doCheckFinancialTrouble`,
/// `CvCityAI::AI_chooseProduction`), keeping items within 3 turns of done, wonders, world
/// projects and heroes. Here a city in the middle of a longer build re-runs the ladder
/// ([`decide`]) and switches only when it now picks an economy building (commerce, research,
/// specialists, trade or maintenance savings make up most of its value: Market, Library,
/// Courthouse...) -- the building it needs now. Also kept: units without combat (settlers,
/// workers, ships for them, great people), a fighter for a city without defenders or under
/// threat, any fighter at war, an economy building already under way. Each city is looked at
/// every fifth turn (the ladder is the costly part).
pub fn trouble_rebuild(ai: &Ai, g: &mut Game, p: PlayerId) {
    if !crate::switches::TROUBLE_REBUILD.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    let f = crate::finance::state(g, p);
    if !f.flexible || f.deficit() == 0 {
        return;
    }
    let d = g.d;
    let turn = g.game_turn();
    for c in g.players[p].cities.clone() {
        if (turn + c.0 as i32) % 5 != 0 || g.cities[c].order_queue.is_empty() || g.city_is_production_process(c) || g.city_production_turns_left(c) <= 3 {
            continue;
        }
        let plot = g.city_plot(c);
        let threat = local_threat(g, p, plot, 3);
        if let Some(u) = g.city_production_unit(c) {
            let info = &d.units[u];
            let limited = info.unit_class_type.is_some_and(|k| ffh_core::utils::is_limited_unit_class(d, k));
            if info.combat == 0 || limited || f.at_war || threat > 0 || garrison(g, p, plot).0 == 0 {
                continue;
            }
        } else if let Some(b) = g.city_production_building(c) {
            let limited = d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_limited_wonder_class(d, k));
            if limited || is_finance_building(g, c, b, threat) {
                continue;
            }
        } else {
            // A project (rituals, world projects): kept.
            continue;
        }
        let Some((Choice::Construct(b), why)) = decide(ai, g, c) else { continue };
        let limited = d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_limited_wonder_class(d, k));
        if limited || Some(b) == g.city_production_building(c) || !is_finance_building(g, c, b, threat) {
            continue;
        }
        let current = g.city_production_unit(c).map(|u| d.units[u].base.type_.as_str()).or_else(|| g.city_production_building(c).map(|b| d.buildings[b].base.type_.as_str())).unwrap_or("?");
        crate::trace!(g, p, "{}: financial trouble (deficit {}, costs {}%): {} with {} turns left gives way to {}", g.cities[c].name, f.deficit(), f.cost_share(), current, g.city_production_turns_left(c), d.buildings[b].base.type_);
        // Alternatives: what it replaces (turns left), and the ladder step that chose the building.
        let left = g.city_production_turns_left(c) as i64;
        g.tel_decision(p, "production", &d.buildings[b].base.type_, "trouble_rebuild", f.deficit() as i64, || ffh_core::telemetry::tel_alts([(current, left), (why, 0)]));
        g.city_clear_order_queue(c);
        push(g, c, Choice::Construct(b));
    }
}

/// A building whose value is mostly economy -- commerce, research, specialists, great people,
/// trade routes ([`economy_value`]) or maintenance savings -- as `building_value` weighs them.
fn is_finance_building(g: &Game, c: CityId, b: BuildingType, threat: i32) -> bool {
    let info = &g.d.buildings[b];
    let p = g.cities[c].owner;
    let gold_value = if financial_trouble(g, p) { 160 } else { 80 };
    let savings = -info.maintenance_modifier * g.cities[c].maintenance.max(0) / 100 * gold_value / 100;
    let ev = economy_value(g, c, b) + savings.max(0);
    ev > 0 && ev * 2 >= building_value(g, c, b, threat).max(1)
}

/// Projects worth building by default (positive AI weight, not a world-ending or self-harming
/// ritual the generic logic cannot judge).
fn best_project(g: &Game, c: CityId) -> Option<ProjectType> {
    let d = g.d;
    let mut best: Option<(ProjectType, i32)> = None;
    for pr in d.projects.keys() {
        let info = &d.projects[pr];
        if info.ai_weight <= 0 || !g.city_can_create(c, pr, false, false) {
            continue;
        }
        let name = info.base.type_.as_str();
        // Rituals with large side effects: left to dedicated logic.
        if matches!(name, "PROJECT_SAMHAIN" | "PROJECT_THE_DRAW" | "PROJECT_STIR_FROM_SLUMBER" | "PROJECT_BANE_DIVINE" | "PROJECT_GLORY_EVERLASTING" | "PROJECT_PURGE_THE_UNFAITHFUL") {
            continue;
        }
        let turns = g.city_production_turns_left_project(c, pr, 0);
        if turns > 40 {
            continue;
        }
        let v = info.ai_weight * 100 / (turns + 5);
        if best.map_or(true, |(_, b)| v > b) {
            best = Some((pr, v));
        }
    }
    best.map(|(pr, _)| pr)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    Defense,
    Attack,
    Worker,
    Settler,
    Hero,
    Mage,
    Explore,
    Siege,
    /// Attackers for a war: strength against cities (city attack bonus, first strikes), not
    /// mobility.
    CityAttack,
    /// Work boats (sea resources).
    SeaWorker,
    /// Ships that carry land units.
    Transport,
    /// Ships that reveal the seas.
    SeaExplore,
    /// Hero ships (the Lanun's Black Wind).
    SeaHero,
}

impl Role {
    fn sea(self) -> bool {
        matches!(self, Role::SeaWorker | Role::Transport | Role::SeaExplore | Role::SeaHero)
    }
}

/// A hero: the hero AI, or the Hero promotion for free. Wilboman and the War Machine have an
/// attacker's default AI, and as attackers (one unit for 360 and 240 hammers) they never won on
/// cost: 1 Wilboman in 47 Illian games, where he could be trained in 74% (docs/AI.md, "Unique
/// units").
fn is_hero(d: &GameData, info: &UnitInfo) -> bool {
    static HERO: OnceLock<Option<PromotionType>> = OnceLock::new();
    let hero = *HERO.get_or_init(|| d.ty::<PromotionType>("PROMOTION_HERO"));
    info.default_unit_ai_type == Some(UnitAIType::UNITAI_HERO) || hero.is_some_and(|h| info.free_promotions[h])
}

/// A spellcaster from birth (Channeling I for free): a hero of strength 0 is still worth training
/// (Corlindale, never trained before in 37 Elohim games).
fn born_caster(d: &GameData, info: &UnitInfo) -> bool {
    static CHANNELING: OnceLock<Option<PromotionType>> = OnceLock::new();
    let channeling = *CHANNELING.get_or_init(|| d.ty::<PromotionType>("PROMOTION_CHANNELING1"));
    channeling.is_some_and(|c| info.free_promotions[c])
}

/// A siege engine (catapults, trebuchets, cannons: default AI city attack or collateral, with
/// bombard or collateral damage). Trained for the siege role only (`campaign::army_choice` asks
/// for a few per target): as the army's attackers the Khazad fielded about 30 Dwarven Cannons a
/// game once their Siege Workshops came.
pub(crate) fn siege_engine(info: &UnitInfo) -> bool {
    matches!(info.default_unit_ai_type, Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL)) && (info.bombard_rate > 0 || info.collateral_damage > 0)
}

fn role_ok(d: &GameData, info: &UnitInfo, role: Role) -> bool {
    let has = |a: UnitAIType| info.unit_ai_type[a];
    let fighter = info.combat > 0 && !info.found && !is_hero(d, info) && info.default_unit_ai_type != Some(UnitAIType::UNITAI_MAGE);
    match role {
        Role::Defense => {
            fighter && (has(UnitAIType::UNITAI_CITY_DEFENSE) || has(UnitAIType::UNITAI_CITY_COUNTER) || has(UnitAIType::UNITAI_ATTACK) || has(UnitAIType::UNITAI_COUNTER))
                && info.default_unit_ai_type != Some(UnitAIType::UNITAI_EXPLORE)
        }
        Role::Attack | Role::CityAttack => fighter && !info.only_defensive && (has(UnitAIType::UNITAI_ATTACK) || has(UnitAIType::UNITAI_ATTACK_CITY)) && !siege_engine(info),
        Role::Worker => has(UnitAIType::UNITAI_WORKER) && info.combat == 0,
        Role::Settler => has(UnitAIType::UNITAI_SETTLE) && info.found,
        Role::Hero => is_hero(d, info) && (info.combat > 0 || born_caster(d, info)),
        Role::SeaHero => is_hero(d, info) && info.combat > 0,
        Role::Mage => info.default_unit_ai_type == Some(UnitAIType::UNITAI_MAGE) || has(UnitAIType::UNITAI_WARWIZARD),
        Role::Explore => has(UnitAIType::UNITAI_EXPLORE),
        Role::SeaWorker => has(UnitAIType::UNITAI_WORKER_SEA) && info.combat == 0,
        Role::Transport => info.cargo_space > 0 && info.domain_cargo == Some(DomainType::DOMAIN_LAND) && info.special_cargo.is_none(),
        Role::SeaExplore => info.combat > 0 && (has(UnitAIType::UNITAI_EXPLORE_SEA) || has(UnitAIType::UNITAI_ATTACK_SEA)),
        Role::Siege => fighter && (has(UnitAIType::UNITAI_ATTACK_CITY) || has(UnitAIType::UNITAI_COLLATERAL)) && (info.bombard_rate > 0 || info.collateral_damage > 0),
    }
}

/// The building a city needs to train a unit (C++ `CvPlot::canTrain`): `PrereqBuilding`, a
/// building type (siege engines, Knights, Druids and most building-gated unique units:
/// Hornguard, Trebuchet, War Machine, Myconid...), or `PrereqBuildingClass`, the civilization's
/// building of that class (archers, melee, Camel Archers...).
pub(crate) fn unit_building(d: &GameData, civ: CivilizationType, info: &UnitInfo) -> Option<BuildingType> {
    info.prereq_building.or_else(|| info.prereq_building_class.and_then(|bc| d.civ_building(civ, bc)))
}

/// A building that would let this city train a defender or attacker at least a third better than
/// its best now ([`unit_building`]), cheap enough to build soon: a third stronger, or a third
/// stronger per hammer. By strength alone the Luchuirp's Sculptor's Studio (Wood Golems, 6) came
/// before the Archery Range, which then looked no gain for the Dwarven Slingers (5 and a first
/// strike, +25% in cities: 14% stronger, but at two thirds of the price), and golems held their
/// cities. (By strength squared per hammer, as [`best_unit`] weighs a choice, a Warrior beat an
/// Axeman and the Training Yards were no longer built.)
fn enabler_building(g: &Game, c: CityId, threat: i32) -> Option<BuildingType> {
    let d = g.d;
    let p = g.cities[c].owner;
    let civ = g.player_get_civilization_type(p)?;
    let best = |role: Role, power: &dyn Fn(UnitType) -> i64| best_unit(g, c, role).map(|u| (power(u), d.units[u].production_cost.max(1) as i64)).filter(|&(strength, _)| strength > 0);
    let cur_def = best(Role::Defense, &|u| type_city_defense(&d.units[u]));
    let cur_att = best(Role::Attack, &|u| type_attack_power(d, u) as i64);
    let better = |power: i64, cost: i64, cur: Option<(i64, i64)>| cur.map_or(0, |(cp, cc)| (power * 100 / cp).max(power * cc * 100 / (cp * cost.max(1))));
    let mut best: Option<(BuildingType, i64)> = None;
    for u in d.units.keys() {
        let info = &d.units[u];
        let Some(b) = unit_building(d, civ, info) else { continue };
        let Some(bc) = d.buildings[b].building_class_type else { continue };
        if info.domain_type != Some(DomainType::DOMAIN_LAND) || g.city_is_has_building_class(c, bc) {
            continue;
        }
        let Some(class) = info.unit_class_type else { continue };
        if d.civ_unit(civ, class) != Some(u) || !g.player_can_train(p, u, false, false, false) {
            continue;
        }
        if info.prereq_and_bonus.is_some_and(|bn| !g.city_has_bonus(c, bn)) {
            continue;
        }
        if !info.prereq_or_bonuses.is_empty() && !info.prereq_or_bonuses.iter().any(|&bn| g.city_has_bonus(c, bn)) {
            continue;
        }
        if !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        // About half the cities get one (the others train what they can).
        let have = g.players[p].cities.iter().filter(|&&o| g.city_is_has_building_class(o, bc)).count();
        if have > g.players[p].cities.len() / 2 {
            continue;
        }
        let cost = info.production_cost as i64;
        let gain = if role_ok(d, info, Role::Defense) { better(type_city_defense(info), cost, cur_def) } else { 0 }
            .max(if role_ok(d, info, Role::Attack) { better(type_attack_power(d, u) as i64, cost, cur_att) } else { 0 });
        if gain < 133 {
            continue;
        }
        // Under some threat, only a quick build.
        let turns = g.city_production_turns_left_building(c, b, 0);
        if turns > if threat > 0 { 6 } else { 20 } {
            continue;
        }
        if best.map_or(true, |(_, s)| gain > s) {
            best = Some((b, gain));
        }
    }
    best.map(|(b, _)| b)
}

/// Defense strength of a unit type in a city (FfH units have a separate defense value).
pub(crate) fn type_city_defense(info: &UnitInfo) -> i64 {
    let dmg: i32 = info.damage_type_combat.iter().sum();
    let base = if info.combat_defense > 0 { info.combat_defense } else { info.combat };
    let mut v = ((base + dmg) * 100) as i64 * (100 + info.city_defense_modifier.max(-50)) as i64 / 100;
    v += v * (info.first_strikes * 2 + info.chance_first_strikes) as i64 / 20;
    v
}

/// Best unit this city can train for `role`, by power per production turn. Upkeep in the price
/// and a peacetime army only from the cities that train good units were measured and reverted
/// (docs/AI_LOG.md, "Unit choices and an upgrade reserve").
pub fn best_unit(g: &Game, c: CityId, role: Role) -> Option<UnitType> {
    let d = g.d;
    let mut best: Option<(UnitType, i64)> = None;
    for u in d.units.keys() {
        let info = &d.units[u];
        let domain = if role.sea() { DomainType::DOMAIN_SEA } else { DomainType::DOMAIN_LAND };
        if info.domain_type != Some(domain) || !role_ok(d, info, role) {
            continue;
        }
        if !g.city_offers_unit(c, u) || !g.city_can_train(c, u, false, false, false, false) {
            continue;
        }
        let turns = g.city_production_turns_left_unit(c, u, 0).max(1) as i64;
        let power = match role {
            Role::Defense => type_city_defense(info),
            Role::Attack | Role::Hero | Role::SeaHero | Role::Siege => {
                let mut v = type_attack_power(d, u) as i64;
                v += v * (info.moves.max(1) as i64 - 1) / 4;
                v += v * (info.first_strikes * 2 + info.chance_first_strikes) as i64 / 20;
                v += v * info.withdrawal_probability as i64 / 200;
                v += info.collateral_damage as i64;
                if role == Role::Siege {
                    v += (info.bombard_rate * 20) as i64 + info.collateral_damage as i64 * 2;
                }
                v
            }
            Role::CityAttack => {
                let mut v = type_attack_power(d, u) as i64 * (100 + info.city_attack_modifier.max(-50)) as i64 / 100;
                v += v * (info.first_strikes * 2 + info.chance_first_strikes) as i64 / 20;
                v += info.collateral_damage as i64 * 2;
                v
            }
            Role::Worker => 100 + info.work_rate as i64,
            Role::SeaWorker => 100,
            Role::Transport => info.cargo_space as i64 * 100 + info.moves as i64 * 30 + info.combat as i64 * 5,
            Role::SeaExplore => info.moves as i64 * 60 + info.combat as i64 * 10 + 50 * i64::from(info.unit_ai_type[UnitAIType::UNITAI_EXPLORE_SEA]),
            Role::Settler => 100,
            Role::Mage | Role::Explore => 100 + info.moves as i64 * 10 + info.tier as i64 * 50,
        } + info.ai_weight as i64;
        if power <= 0 {
            continue;
        }
        // Strength matters more than cost: weaker but cheaper units only win by a margin.
        let score = match role {
            Role::Worker | Role::Settler | Role::SeaWorker | Role::Transport | Role::SeaExplore => power * 1000 / turns,
            _ => power * power / 100 * 1000 / (turns + 4),
        };
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((u, score));
        }
    }
    best.map(|(u, _)| u)
}

/// The strongest unit this city can train for `role`, whatever it costs.
fn strongest_unit(g: &Game, c: CityId, role: Role) -> Option<UnitType> {
    let d = g.d;
    d.units
        .keys()
        .filter(|&u| d.units[u].domain_type == Some(DomainType::DOMAIN_LAND) && role_ok(d, &d.units[u], role) && g.city_offers_unit(c, u) && g.city_can_train(c, u, false, false, false, false))
        .max_by_key(|&u| (type_attack_power(d, u), d.units[u].production_cost, std::cmp::Reverse(u)))
}

/// Is there a foreign city a missionary of `r` from city `c` could convert: one we may visit
/// (`units::abroad_city`), revealed to us, on `c`'s landmass within 15 plots.
fn abroad_targets(g: &Game, p: PlayerId, c: CityId, r: ReligionType) -> bool {
    let team = g.players[p].team;
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let area = g.map.plots[g.city_plot(c)].area;
    g.civ_player_ids().any(|q| {
        q != p
            && g.players[q].alive
            && g.players[q].cities.iter().any(|&oc| {
                let op = g.city_plot(oc);
                g.map.plots[op].area == area
                    && g.map.plots[op].revealed[team]
                    && g.plot_distance(x, y, g.cities[oc].x, g.cities[oc].y) <= 15
                    && crate::units::abroad_city(g, p, oc, r)
            })
    })
}

fn best_missionary(g: &Game, c: CityId, r: ReligionType) -> Option<UnitType> {
    g.d.units.keys().filter(|&u| {
        let info = &g.d.units[u];
        info.unit_ai_type[UnitAIType::UNITAI_MISSIONARY] && info.religion_spreads[r] > 0
    }).find(|&u| g.city_offers_unit(c, u) && g.city_can_train(c, u, false, false, false, false))
}

/// Marginal value of building `b` in city `c` (roughly: value points per turn, a hammer being
/// worth 15 as in the governor).
/// The marginal score (value per turn, see step 9 of `decide`) a building must clear to be built
/// while the army is short -- the ladder's existing army-short bar, shared by the economy gate and
/// step 9 (lifted from a literal so the two stay identical).
const ECON_BUILD_BAR: i32 = 40;

/// What a missionary sent abroad is worth in [`building_value`] points: the shrine's gold a turn
/// for the city it converts (80), at the odds of reaching it and spreading (a city without a
/// religion takes it almost surely; one with another religion about four times in five).
const MISSIONARY_ABROAD_VALUE: i32 = 60;

/// The economy portion of [`building_value`] -- its gold/research commerce terms plus specialists,
/// great people and trade routes -- using the identical weights. A literal subset of
/// `building_value`'s own lines (keep the two in sync); keyed only off `CommerceType`/`YieldType`
/// constants and `BuildingInfo` fields, never a building name.
fn economy_value(g: &Game, c: CityId, b: BuildingType) -> i32 {
    let d = g.d;
    let info = &d.buildings[b];
    let p = g.cities[c].owner;
    let n_cities = g.players[p].cities.len() as i32;
    let pop = g.cities[c].population;
    let yw = [20, 15, 7];
    let cw = [8, 9, 4, 1];
    let mut v = 0;
    // Commerce YIELD (building_value's yield block, commerce column = index 2 only).
    let ci = 2usize; // YieldType::YIELD_COMMERCE
    let comm = g.city_yield_rate(c, YieldType::YIELD_COMMERCE).max(0);
    v += info.yield_change[ci] * yw[ci] * 10;
    v += info.yield_modifier[ci] * comm * yw[ci] / 10;
    v += info.global_yield_modifier[ci] * comm * yw[ci] * n_cities.min(8) / 20;
    // GOLD and RESEARCH commerce (building_value's commerce block, those columns only).
    for i in [CommerceType::COMMERCE_GOLD.idx(), CommerceType::COMMERCE_RESEARCH.idx()] {
        let rate = g.city_commerce_rate(c, CommerceType::from_usize(i)).max(0);
        v += (info.commerce_change[i] + info.obsolete_safe_commerce_change[i]) * cw[i] * 10;
        v += info.commerce_modifier[i] * rate * cw[i] / 10;
        v += info.global_commerce_modifier[i] * rate * cw[i] * n_cities.min(8) / 20;
    }
    // Specialists, great people, trade routes (building_value's own terms).
    let slots: i32 = info.specialist_count.iter().sum();
    v += slots * 15 * (pop - 4).clamp(0, 8);
    v += info.great_people_rate_change * 40 + info.great_people_rate_modifier * pop / 4;
    v += info.free_specialist * 120;
    v += info.trade_routes * 60;
    v
}

/// A building whose value is majority economy (research/commerce), so it should not wait behind the
/// surplus-army interleave in `decide`. A ratio of the same valuation, not a magic threshold.
fn is_economy_building(g: &Game, c: CityId, b: BuildingType, threat: i32) -> bool {
    let ev = economy_value(g, c, b);
    ev > 0 && ev * 2 >= building_value(g, c, b, threat).max(1)
}

/// What building `b` is worth in city `c`: the generic estimate, then the civilization's own
/// (civs/, e.g. a unique building whose effect lives in Python).
pub fn building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let v = crate::civs::building_value(g, c, b, generic_building_value(g, c, b, threat)); // civ-mechanics
    crate::traits::building_value(g, c, b, v) // trait-play
}

/// The generic part of [`building_value`].
fn generic_building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let d = g.d;
    let info = &d.buildings[b];
    let city = &g.cities[c];
    let p = city.owner;
    let n_cities = g.players[p].cities.len() as i32;
    let pop = city.population;
    let food = g.city_yield_rate(c, YieldType::YIELD_FOOD).max(0);
    let prod = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION).max(0);
    let comm = g.city_yield_rate(c, YieldType::YIELD_COMMERCE).max(0);
    // Value of one point per turn of each yield / commerce type.
    let yw = [20, 15, 7];
    let rates = [food, prod, comm];
    let mut v = 0;
    for i in 0..YieldType::NUM {
        v += info.yield_change[i] * yw[i] * 10;
        v += info.yield_modifier[i] * rates[i] * yw[i] / 10;
        v += info.global_yield_modifier[i] * rates[i] * yw[i] * n_cities.min(8) / 20;
    }
    let cw = [8, 9, 4, 1];
    for i in 0..CommerceType::NUM {
        let rate = g.city_commerce_rate(c, CommerceType::from_usize(i)).max(0);
        v += (info.commerce_change[i] + info.obsolete_safe_commerce_change[i]) * cw[i] * 10;
        v += info.commerce_modifier[i] * rate * cw[i] / 10;
        v += info.global_commerce_modifier[i] * rate * cw[i] * n_cities.min(8) / 20;
    }
    // Culture: border growth matters in a new city (cheap culture buildings only).
    let culture = info.commerce_change[CommerceType::COMMERCE_CULTURE.idx()];
    if culture > 0 && info.production_cost <= 120 && city.culture_level.map_or(true, |l| l.i() <= 1) {
        v += culture.min(3) * 150;
    }

    // Happiness and health, worth a citizen when the city is at its cap.
    let happy_room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
    let mut happy = info.happiness + info.area_happiness + info.global_happiness + info.state_religion_happiness * i32::from(info.state_religion.is_some());
    for bonus in d.bonuses.keys() {
        if info.bonus_happiness_changes[bonus] != 0 && g.city_has_bonus(c, bonus) {
            happy += info.bonus_happiness_changes[bonus];
        }
    }
    if happy != 0 {
        let per = if happy_room <= 0 { 180 } else if happy_room <= 1 { 100 } else { 35 };
        v += happy * per;
    }
    v += (info.area_happiness * 30 + info.global_happiness * 50) * (n_cities - 1).max(0);
    let health_room = g.city_good_health(c) - g.city_bad_health(c, false, 0);
    let mut health = info.health + info.area_health + info.global_health;
    for bonus in d.bonuses.keys() {
        if info.bonus_health_changes[bonus] != 0 && g.city_has_bonus(c, bonus) {
            health += info.bonus_health_changes[bonus];
        }
    }
    if health != 0 {
        let per = if health_room <= 0 { 120 } else if health_room <= 2 { 60 } else { 20 };
        v += health * per;
    }
    // Specialists slots in a big city.
    let slots: i32 = info.specialist_count.iter().sum();
    v += slots * 15 * (pop - 4).clamp(0, 8);
    v += info.great_people_rate_change * 40 + info.great_people_rate_modifier * pop / 4;
    v += info.free_specialist * 120;
    v += info.defense_modifier * if threat > 0 { 6 } else { 1 } / 2;
    // Experience matters in cities that train units.
    let xp: i32 = info.free_experience + info.unit_combat_free_experience.iter().sum::<i32>().min(4) + info.domain_free_experience[DomainType::DOMAIN_LAND];
    v += xp * prod.min(15) * 4;
    v += info.military_production_modifier * prod / 4;
    v += info.trade_routes * 60;
    // Granary-like food storage: worth more while the city grows.
    v += info.food_kept * food.min(12) / 4;
    v += info.worker_speed_modifier / 2;
    // Maintenance savings are gold (80 per gold per turn, more when short of gold).
    let gold_value = if financial_trouble(g, p) { 160 } else { 80 };
    v += -info.maintenance_modifier * g.cities[c].maintenance.max(0) / 100 * gold_value / 100;
    v += info.heal_rate_change * 2;
    v += info.ai_weight;
    // Effects the rules implement in code, not in the data (the Warrens' copies, the Planar
    // Gate's creatures, ...).
    v += crate::code_effects::building_value(g, c, b, threat);
    if let Some(l) = g.players[p].personality_type.map(|l| &d.leader_heads[l]) {
        if info.building_class_type.is_some() && (l.favorite_early_wonder == info.building_class_type || l.favorite_late_wonder == info.building_class_type) {
            v += 400;
        }
        if l.favorite_wonder == Some(b) {
            v += 300;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heroes_are_trained_as_heroes() {
        let d = ffh_data::global();
        let info = |name: &str| &d.units[d.t::<UnitType>(name)];
        // The Hero promotion makes a hero whatever the default AI.
        for name in ["UNIT_WILBOMAN", "UNIT_WAR_MACHINE"] {
            assert!(role_ok(d, info(name), Role::Hero), "{name} is trained as a hero");
            assert!(!role_ok(d, info(name), Role::Attack), "{name} is no army attacker");
        }
        // So does a spellcaster of strength 0 with the hero AI, and a hero ship at sea.
        assert!(role_ok(d, info("UNIT_CORLINDALE"), Role::Hero));
        assert!(role_ok(d, info("UNIT_BLACK_WIND"), Role::SeaHero) && Role::SeaHero.sea());
        assert!(role_ok(d, info("UNIT_DONAL"), Role::Hero));
        assert!(!role_ok(d, info("UNIT_AXEMAN"), Role::Hero) && !role_ok(d, info("UNIT_ADEPT"), Role::Hero));
    }
}
