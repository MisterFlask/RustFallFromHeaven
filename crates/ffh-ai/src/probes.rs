//! Probes: the standard AI with a few of its priorities overridden so that it pushes one strategy
//! as hard as it can (CLAUDE.md, "Probes: AIs that go all in on one strategy"; docs/PROBES.md).
//! They are measuring instruments -- opponents unlike the standard AI, and tests of whether a
//! strategy is over- or underpowered -- not a second AI: every override below is consulted at one
//! of the standard AI's own decision points, and everything else plays as standard.
//!
//! A probe is chosen per civilization, as the off-by-default switches are (`switches.rs`):
//! `FFH_AI_PROBE_RUSH_ONLY`, `FFH_AI_PROBE_HORDE_ONLY`, `FFH_AI_PROBE_TURTLE_ONLY` and
//! `FFH_AI_PROBE_EXPAND_ONLY` list the civilizations that play it (or `ALL`). Two set for one
//! civilization: the first in [`Probe`]'s order. Barbarians never probe. With no variable set
//! every hook answers "standard" and the AI plays exactly as without this module.
//!
//! - **Rush** ([`Probe::Rush`]): kill the nearest neighbour early. With [`RUSH_MIN_CITIES`]
//!   cities it picks the nearest land-reachable civilization it can declare on and prepares the war
//!   (no settlers from then on), trains the best attacker in every city with one defender each
//!   while [`ARMY_RESEARCH_FLOOR_PCT`] research stays affordable, builds only buildings that make
//!   units faster or stronger, researches the cheapest path to a stronger land attacker, and
//!   declares once its army near the target covers the target's nearest city with a reserve
//!   (`combat_sim::covers`). No peace while it is not losing and the enemy has a city on our
//!   landmass the army takes. Then the next nearest.
//! - **Horde** ([`Probe::Horde`]): the standard opening and economy, but from
//!   [`HORDE_MIN_CITIES`] cities on a fixed share of the cities' decisions goes to army, and it is
//!   always at war or preparing one against the weakest reachable neighbour by the rules' defensive
//!   power, declared on the standard readiness test.
//! - **Turtle** ([`Probe::Turtle`]): never declares war, takes every peace, keeps a garrison of
//!   [`TURTLE_GARRISON_MULT`] times the standard quota (at least [`TURTLE_GARRISON_MIN`]) of the
//!   best defenders, builds defensive buildings first, no field army beyond the small force that
//!   clears raiders; the rest is the standard economy.
//! - **Expand** ([`Probe::Expand`]): settlers whenever a site exists (a lower bar on sites, more
//!   settlers at a time, from smaller cities), one defender per city while sites remain, no wars
//!   declared, every peace taken; with no site left it plays standard.
//!
//! Hooks (each a call into this module; nothing else in the AI knows about probes):
//! - `lib.rs` `telemetry_intents`: the intent `probe` ([`name`]), so game records say who was one;
//! - `production.rs` `decide` (a directly ruled city): the garrison quota ([`garrison_need`],
//!   [`keeps_garrison`]), the second defender ([`skips_second_defender`]), settlers
//!   ([`settlers_allowed`]), the army and buildings ([`production`], after the garrison quota), the
//!   field army's size and the campaign's force ([`small_army`]);
//! - `governor.rs` `decide` (a governed city: most of the AI's cities): the same garrison hooks and
//!   [`production`] after the garrison; `empire_cities`: the cities that take the ladder's choices
//!   (settlers among them) as empire orders ([`empire_cities`]: every one for the Expand);
//! - `expansion.rs` `settler_choice`: settlers at a time and the city size for one
//!   ([`settler_limits`]); `sites.rs` `compute`: the lowest site value ([`site_min_value`]);
//! - `research.rs` `choose`: the Rush's research ([`research`]);
//! - `diplomacy.rs` `manage`: peace ([`peace_stance`]) and wars ([`manage_war`]); `judge_offer`:
//!   peace offered ([`accept_peace`]), war bribes ([`declares_wars`]);
//! - `finance.rs` `disband_military`: the army is not scrapped for its upkeep ([`keeps_army`]: the
//!   army is trained only while [`ARMY_RESEARCH_FLOOR_PCT`] research stays affordable, and the
//!   sliders pay for it below that);
//! - `units.rs` `garrison_quota`: the Turtle's defenders stay in their cities ([`garrison_need`]).
//!
//! No unit, tech, building or civilization is named and no turn number is fixed: attackers and
//! defenders are what `production::best_unit` picks by the rules' strengths, targets come from
//! the reach and power helpers of `diplomacy.rs`, readiness from `combat_sim`.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::{best_unit, count_ai, Choice, Role};
use crate::switches::Switch;

/// A strategy pushed to its limit.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    Rush,
    Horde,
    Turtle,
    Expand,
}

static RUSH: Switch = Switch::off_by_default("PROBE_RUSH");
static HORDE: Switch = Switch::off_by_default("PROBE_HORDE");
static TURTLE: Switch = Switch::off_by_default("PROBE_TURTLE");
static EXPAND: Switch = Switch::off_by_default("PROBE_EXPAND");

// ---------------------------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------------------------

/// Rush: the cities we have before the first target is chosen (the standard war gate's own minimum,
/// `diplomacy::manage`); from then on no more settlers.
pub const RUSH_MIN_CITIES: usize = 2;
/// Rush: an attacker is worth researching toward when its city-attack strength is at least this
/// percent of our best trainable one's.
pub const RUSH_ATTACK_STEP_PCT: i64 = 120;
/// Rush: the share (percent) of a city's decisions that may go to a building that makes units
/// faster or stronger, when one is available; the rest are attackers.
pub const RUSH_BUILDING_SHARE_PCT: i32 = 25;
/// Rush: the longest such a building may take, in turns of the city's production.
pub const RUSH_BUILDING_MAX_TURNS: i32 = 12;
/// Rush: our army units count toward the declaration when they stand at most this many plots
/// farther from the target city than our nearest city does (the rally city, where the campaign
/// gathers them while the war is prepared).
pub const RUSH_STAGE_SLACK: i32 = 3;
/// Rush and Horde: one more unit is trained for the army only while its upkeep leaves this research
/// share (percent) affordable. The standard asks for 60%, 30% at war; at 0% a rush trained warriors
/// until it was bankrupt and never reached the techs of a better attacker.
pub const ARMY_RESEARCH_FLOOR_PCT: i32 = 20;
/// Horde: the army takes this share (percent) of the cities' production decisions.
pub const HORDE_ARMY_SHARE_PCT: i32 = 67;
/// Horde: the army share starts once we have this many cities (the standard opening before it),
/// and so does the first war's preparation.
pub const HORDE_MIN_CITIES: usize = 3;
/// Turtle: the garrison is this multiple of the standard quota of each city...
pub const TURTLE_GARRISON_MULT: i32 = 3;
/// ... and never below this many defenders.
pub const TURTLE_GARRISON_MIN: i32 = 3;
/// Turtle: the longest a defensive building (one that adds city defense) may take, in turns.
pub const TURTLE_DEFENSE_MAX_TURNS: i32 = 30;
/// Expand: sites down to this percent of the standard lowest site value are settled.
pub const EXPAND_SITE_VALUE_PCT: i32 = 75;
/// Expand: a city of this size may build a settler (the standard asks 3 or 4 while it can grow).
pub const EXPAND_MIN_POP: i32 = 2;

// ---------------------------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------------------------

/// The probe player `p` plays, if any (`FFH_AI_PROBE_<NAME>_ONLY`, read once).
pub fn probe(g: &Game, p: PlayerId) -> Option<Probe> {
    if g.player_is_barbarian(p) || g.players[p].human {
        return None;
    }
    let civ = g.player_get_civilization_type(p)?;
    [(Probe::Rush, &RUSH), (Probe::Horde, &HORDE), (Probe::Turtle, &TURTLE), (Probe::Expand, &EXPAND)].into_iter().find(|(_, s)| s.civ_on(g.d, civ)).map(|(pr, _)| pr)
}

/// The probe's name (telemetry).
pub fn name(pr: Probe) -> &'static str {
    match pr {
        Probe::Rush => "rush",
        Probe::Horde => "horde",
        Probe::Turtle => "turtle",
        Probe::Expand => "expand",
    }
}

/// Does `p` declare wars at all (a Turtle or an Expand probe does not)?
pub fn declares_wars(g: &Game, p: PlayerId) -> bool {
    !matches!(probe(g, p), Some(Probe::Turtle | Probe::Expand))
}

/// At war with a civilization, or preparing a war.
fn war_footing(g: &Game, p: PlayerId) -> bool {
    crate::production::civ_enemies_at_war(g, p) || crate::diplomacy::preparing_target(g, g.players[p].team).is_some()
}

// ---------------------------------------------------------------------------------------------
// Production
// ---------------------------------------------------------------------------------------------

/// Player `p`'s garrison quota for one city, given the standard one (`decide`'s or the placement's).
/// The Turtle's floor is trained only while one more unit is affordable at
/// [`ARMY_RESEARCH_FLOOR_PCT`] (beyond it the rules disband units for the deficit), and kept in
/// place once trained (`production == false`: the placement's quota).
pub fn garrison_need(g: &Game, p: PlayerId, standard: i32, production: bool) -> i32 {
    match probe(g, p) {
        Some(Probe::Turtle) if !production || army_affordable(g, p) => (standard * TURTLE_GARRISON_MULT).max(TURTLE_GARRISON_MIN),
        Some(Probe::Rush) => 1,
        Some(Probe::Expand) if expanding(g, p) => 1,
        _ => standard,
    }
}

/// Does `p` keep its whole garrison when it is broke or cannot afford one more unit at the
/// standard's research share (the standard cuts it to one a city)? The Turtle does while one more
/// unit is affordable at [`ARMY_RESEARCH_FLOOR_PCT`].
pub fn keeps_garrison(g: &Game, p: PlayerId) -> bool {
    probe(g, p) == Some(Probe::Turtle) && army_affordable(g, p)
}

/// How many governed cities take the AI's own production ladder as empire orders (settlers come
/// only so), given the standard's number: the Expand, every one.
pub fn empire_cities(g: &Game, p: PlayerId, standard: usize) -> usize {
    if probe(g, p) == Some(Probe::Expand) && expanding(g, p) {
        usize::MAX
    } else {
        standard
    }
}

/// No second defender in the capital and near barbarians: the Rush, and the Expand while it settles.
pub fn skips_second_defender(g: &Game, p: PlayerId) -> bool {
    match probe(g, p) {
        Some(Probe::Rush) => true,
        Some(Probe::Expand) => expanding(g, p),
        _ => false,
    }
}

/// May `p` train settlers? The Rush stops once it has a target: at war or preparing one, or with
/// [`RUSH_MIN_CITIES`] cities and a civilization it can reach by land (between its wars too: a
/// settler in production when the first target is chosen is the most it adds).
pub fn settlers_allowed(g: &Game, p: PlayerId) -> bool {
    probe(g, p) != Some(Probe::Rush) || !(war_footing(g, p) || g.players[p].cities.len() >= RUSH_MIN_CITIES && nearest_target(g, p).is_some())
}

/// An Expand probe with sites left on its landmasses (`naval::home_is_full` is the standard's own
/// test of that).
fn expanding(g: &Game, p: PlayerId) -> bool {
    !crate::naval::home_is_full(g, p)
}

/// The field army is the small force that clears raiders (the Turtle): no army target beyond it,
/// no campaign force.
pub fn small_army(g: &Game, p: PlayerId) -> bool {
    probe(g, p) == Some(Probe::Turtle)
}

/// The army and buildings of a probe's city, after the standard ladder's garrison, settlers,
/// civilization items and unit-enabling buildings: `None` lets the standard ladder go on.
pub fn production(g: &Game, c: CityId, threat: i32) -> Option<(Choice, &'static str)> {
    let p = g.cities[c].owner;
    match probe(g, p)? {
        Probe::Rush => rush_production(g, c),
        Probe::Horde => horde_production(g, c),
        Probe::Turtle => defensive_building(g, c, threat).map(|b| (Choice::Construct(b), "probe_turtle_defense")),
        Probe::Expand => None,
    }
}

/// A deterministic per-city, per-turn draw in 0..100 (no RNG: the AI draws none).
fn slot(g: &Game, c: CityId) -> i32 {
    (g.game_turn().wrapping_mul(37) + c.0 as i32 * 61).rem_euclid(100)
}

/// Our army units alive or in training: attackers (all army roles) and siege engines.
fn army_counts(g: &Game, p: PlayerId) -> (i32, i32) {
    let attackers = count_ai(g, p, UnitAIType::UNITAI_ATTACK) + count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL);
    let siege = count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL);
    (attackers, siege)
}

/// The army's next unit from city `c`: what the campaign's force is missing (siege engines for
/// walls), else the best attacker (against cities at war or while preparing one).
fn army_unit(g: &Game, c: CityId, why: &'static str) -> Option<(Choice, &'static str)> {
    let p = g.cities[c].owner;
    let (attackers, siege) = army_counts(g, p);
    if let Some(ch) = crate::campaign::army_choice(g, c, attackers - siege, siege, true) {
        return Some((ch, why));
    }
    let role = if war_footing(g, p) { Role::CityAttack } else { Role::Attack };
    best_unit(g, c, role).or_else(|| best_unit(g, c, Role::Attack)).map(|u| (Choice::Train(u, UnitAIType::UNITAI_ATTACK), why))
}

/// Can `p` pay for one more unit with research at [`ARMY_RESEARCH_FLOOR_PCT`]? (The probes that
/// live by their army spend most of the research slider on it, as a human rusher does.)
fn army_affordable(g: &Game, p: PlayerId) -> bool {
    crate::economy::can_afford_unit_at(g, p, true, ARMY_RESEARCH_FLOOR_PCT)
}

fn rush_production(g: &Game, c: CityId) -> Option<(Choice, &'static str)> {
    let p = g.cities[c].owner;
    if !army_affordable(g, p) {
        return None;
    }
    if slot(g, c) < RUSH_BUILDING_SHARE_PCT {
        if let Some(b) = military_building(g, c) {
            return Some((Choice::Construct(b), "probe_rush_building"));
        }
    }
    army_unit(g, c, "probe_rush_army")
}

fn horde_production(g: &Game, c: CityId) -> Option<(Choice, &'static str)> {
    let p = g.cities[c].owner;
    if g.players[p].cities.len() < HORDE_MIN_CITIES || slot(g, c) >= HORDE_ARMY_SHARE_PCT || !army_affordable(g, p) {
        return None;
    }
    army_unit(g, c, "probe_horde_army")
}

/// Does building `b` make units faster or stronger: more production (yield, military production),
/// experience or promotions for the units trained, or better units from its tiers?
fn military_effect(info: &BuildingInfo) -> bool {
    let prod = YieldType::YIELD_PRODUCTION.idx();
    info.yield_change[prod] > 0
        || info.yield_modifier[prod] > 0
        || info.military_production_modifier > 0
        || info.free_experience > 0
        || info.unit_combat_free_experience.iter().any(|&x| x > 0)
        || info.domain_free_experience[DomainType::DOMAIN_LAND] > 0
        || info.unlocked_unit_experience > 0
        || info.unlocked_unit_production_modifier > 0
        || info.unlocked_unit_promotion.is_some()
        || info.extra_free_promotion.is_some()
}

/// The building of most value per turn in city `c` among those that make units faster or stronger
/// ([`military_effect`]), done within [`RUSH_BUILDING_MAX_TURNS`]; no world wonder.
fn military_building(g: &Game, c: CityId) -> Option<BuildingType> {
    let d = g.d;
    let mut best: Option<(BuildingType, i32)> = None;
    for b in d.buildings.keys() {
        if !military_effect(&d.buildings[b]) || crate::production::is_world_wonder(d, b) || !crate::production::building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        if turns > RUSH_BUILDING_MAX_TURNS {
            continue;
        }
        let value = crate::production::building_value(g, c, b, 0);
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

/// The Turtle's walls: the building adding the most city defense per turn that city `c` can build
/// within [`TURTLE_DEFENSE_MAX_TURNS`] (6 under threat).
fn defensive_building(g: &Game, c: CityId, threat: i32) -> Option<BuildingType> {
    let d = g.d;
    let max_turns = if threat > 0 { 6 } else { TURTLE_DEFENSE_MAX_TURNS };
    let mut best: Option<(BuildingType, i32)> = None;
    for b in d.buildings.keys() {
        let info = &d.buildings[b];
        if info.defense_modifier <= 0 || crate::production::is_world_wonder(d, b) || !crate::production::building_allowed(g, c, b) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        let turns = g.city_production_turns_left_building(c, b, 0).max(1);
        if turns > max_turns {
            continue;
        }
        let score = info.defense_modifier * 100 / (turns + 3);
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((b, score));
        }
    }
    best.map(|(b, _)| b)
}

// ---------------------------------------------------------------------------------------------
// Expansion
// ---------------------------------------------------------------------------------------------

/// Settlers at a time and the smallest city that builds one, given the standard's (`max_settlers`,
/// `min_pop`) and the sites on the city's landmass.
pub fn settler_limits(g: &Game, p: PlayerId, max_settlers: i32, min_pop: i32, area_sites: i32) -> (i32, i32) {
    if probe(g, p) != Some(Probe::Expand) {
        return (max_settlers, min_pop);
    }
    // One settler for every site in reach, as many as we have cities.
    let n_cities = g.players[p].cities.len() as i32;
    (max_settlers.max(n_cities.max(1).min(area_sites)), min_pop.min(EXPAND_MIN_POP))
}

/// The lowest value of a site `p` settles, given the standard's.
pub fn site_min_value(g: &Game, p: PlayerId, standard: i32) -> i32 {
    if probe(g, p) == Some(Probe::Expand) {
        (standard * EXPAND_SITE_VALUE_PCT / 100).max(1)
    } else {
        standard
    }
}

// ---------------------------------------------------------------------------------------------
// Research
// ---------------------------------------------------------------------------------------------

/// A land attacker of `p`'s civilization the army may train (not a siege engine, hero, limited
/// unit or a religion's unit): its city-attack strength (x100: strength with its city attack
/// modifier, as production's city-attack role weighs one), or `None`.
fn attacker_strength(g: &Game, p: PlayerId, u: UnitType) -> Option<i64> {
    let d = g.d;
    let info = &d.units[u];
    let civ = g.player_get_civilization_type(p)?;
    let class = info.unit_class_type?;
    if d.civ_unit(civ, class) != Some(u) || info.domain_type != Some(DomainType::DOMAIN_LAND) || info.combat <= 0 || info.only_defensive || info.found || info.production_cost <= 0 {
        return None;
    }
    if !(info.unit_ai_type[UnitAIType::UNITAI_ATTACK] || info.unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY]) || crate::production::siege_engine(info) {
        return None;
    }
    if info.prereq_religion.is_some() || info.state_religion.is_some() || info.min_level > 0 || info.default_unit_ai_type == Some(UnitAIType::UNITAI_HERO) {
        return None;
    }
    let ci = &d.unit_classes[class];
    if ci.max_global_instances > 0 || ci.max_team_instances > 0 || ci.max_player_instances > 0 {
        return None;
    }
    if info.prereq_building.is_some_and(|b| d.buildings[b].prereq_religion.is_some() || d.buildings[b].state_religion.is_some()) {
        return None;
    }
    Some(((info.combat + info.damage_type_combat.iter().sum::<i32>()) * (100 + info.city_attack_modifier.max(-50))) as i64)
}

/// Do we have (or own a revealed plot with) the resources unit `u` needs?
fn has_unit_resources(g: &Game, p: PlayerId, u: UnitType) -> bool {
    let info = &g.d.units[u];
    let team = g.players[p].team;
    let have = |bn: BonusType| {
        g.player_has_bonus(p, bn) || (0..g.map.num_plots()).any(|i| g.map.plots[i].owner == Some(p) && g.plot_get_bonus_type(PlotIdx(i as u32), Some(team)) == Some(bn))
    };
    info.prereq_and_bonus.map_or(true, have) && (info.prereq_or_bonuses.is_empty() || info.prereq_or_bonuses.iter().any(|&bn| have(bn)))
}

/// The Rush's research: toward the cheapest (in research left) land attacker at least
/// [`RUSH_ATTACK_STEP_PCT`] as strong as our best trainable one, the next tech on its path we can
/// research now. `None`: the standard choice (no probe, or no such attacker).
pub fn research(g: &Game, p: PlayerId) -> Option<TechType> {
    if probe(g, p) != Some(Probe::Rush) {
        return None;
    }
    let d = g.d;
    let team = g.players[p].team;
    let current = d.units.keys().filter(|&u| g.player_can_train(p, u, false, false, false)).filter_map(|u| attacker_strength(g, p, u)).max().unwrap_or(0);
    let mut best: Option<(i64, UnitType, Vec<TechType>)> = None;
    for u in d.units.keys() {
        let Some(strength) = attacker_strength(g, p, u) else { continue };
        if strength * 100 < current * RUSH_ATTACK_STEP_PCT || g.player_can_train(p, u, false, false, false) || !has_unit_resources(g, p, u) {
            continue;
        }
        let info = &d.units[u];
        let mut goals: Vec<TechType> = info.prereq_and_tech.into_iter().chain(info.prereq_and_techs.iter().copied()).collect();
        if let Some(b) = g.player_get_civilization_type(p).and_then(|civ| crate::production::unit_building(d, civ, info)) {
            goals.extend(d.buildings[b].prereq_and_tech);
            goals.extend(d.buildings[b].prereq_and_techs.iter().flatten().copied());
        }
        let mut techs: Vec<TechType> = Vec::new();
        let mut reachable = true;
        for t in goals {
            if g.team_is_has_tech(team, Some(t)) {
                continue;
            }
            match crate::tech_plan::path(g, p, t) {
                Some(path) => {
                    for x in path.techs {
                        if !techs.contains(&x) {
                            techs.push(x);
                        }
                    }
                }
                None => reachable = false,
            }
        }
        if !reachable || techs.is_empty() {
            continue;
        }
        let cost: i64 = techs.iter().map(|&t| crate::tech_plan::effective_left(g, p, t)).sum();
        if best.as_ref().map_or(true, |(bc, bu, _)| (cost, u) < (*bc, *bu)) {
            best = Some((cost, u, techs));
        }
    }
    let (cost, u, techs) = best?;
    let next = techs.iter().copied().filter(|&t| g.player_can_research(p, t, false)).min_by_key(|&t| (crate::tech_plan::effective_left(g, p, t), t))?;
    crate::trace!(g, p, "probe rush: research {} toward {} ({} beakers left)", d.techs[next].base.type_, d.units[u].base.type_, cost);
    g.tel_decision(p, "research", &d.techs[next].base.type_, "probe_rush_attacker", cost, String::new);
    Some(next)
}

// ---------------------------------------------------------------------------------------------
// Wars and peace
// ---------------------------------------------------------------------------------------------

/// What `p` does about peace with `enemy` in `diplomacy::manage`: `Some(true)` seeks it (the
/// Turtle and the Expand), `Some(false)` fights on (the Rush while it can take a city), `None` as
/// standard.
pub fn peace_stance(g: &Game, p: PlayerId, enemy: TeamId) -> Option<bool> {
    match probe(g, p)? {
        Probe::Turtle | Probe::Expand => Some(true),
        Probe::Rush => rush_fights_on(g, p, enemy).then_some(false),
        Probe::Horde => None,
    }
}

/// Does `p` take a peace `their_team` offers? `Some(true)`: the Turtle and the Expand take every
/// one; `Some(false)`: the Rush refuses while it can take a city; `None`: as standard.
pub fn accept_peace(g: &Game, p: PlayerId, their_team: TeamId) -> Option<bool> {
    peace_stance(g, p, their_team)
}

/// The Rush fights on against `enemy` while it is not losing (the standard's test: they score more
/// and are not weaker) and `enemy` has a city on a landmass we share that our army takes: our best
/// trainable attacker can take it (`diplomacy::can_take_city`: production rebuilds the force), or
/// our army on its landmass covers it at the strike's odds ([`army_takes`] without a reserve).
fn rush_fights_on(g: &Game, p: PlayerId, enemy: TeamId) -> bool {
    let team = g.players[p].team;
    let ours = g.team_get_power(team, true).max(1) as i64;
    let theirs = g.team_get_power(enemy, true).max(1) as i64;
    let losing = g.teams[enemy].ai_war_success[team] > g.teams[team].ai_war_success[enemy] + 10 && ours * 10 < theirs * 12;
    if losing {
        return false;
    }
    let areas: Vec<Option<AreaId>> = g.players[p].cities.iter().map(|&c| g.map.plots[g.city_plot(c)].area).collect();
    g.player_ids().filter(|&q| g.players[q].alive && g.players[q].team == enemy).any(|q| {
        g.players[q].cities.iter().any(|&c| {
            let cp = g.city_plot(c);
            areas.contains(&g.map.plots[cp].area) && g.map.plots[cp].revealed[team] && (crate::diplomacy::can_take_city(g, p, c) || army_takes(g, p, c, i32::MAX, false).is_some())
        })
    })
}

/// A probe's wars, in `diplomacy::manage` once `p` (a team leader) is at peace with every
/// civilization: `true` when the probe has decided (the standard's war preparation is skipped).
pub fn manage_war(g: &mut Game, p: PlayerId) -> bool {
    let Some(pr) = probe(g, p) else { return false };
    let team = g.players[p].team;
    let preparing = crate::diplomacy::preparing_target(g, team);
    match pr {
        Probe::Turtle | Probe::Expand => {
            if let Some(t) = preparing {
                g.team_ai_set_war_plan(team, t, None, true);
            }
            true
        }
        Probe::Rush => {
            match preparing {
                Some(t) => rush_declare(g, p, t),
                None if g.players[p].cities.len() < RUSH_MIN_CITIES => {}
                None => {
                    if let Some(t) = nearest_target(g, p) {
                        crate::trace!(g, p, "probe rush: prepares war on team {:?}, the nearest by land", t);
                        g.tel_war_reason(team, t, || "probe rush: nearest by land".to_string());
                        g.tel_decision(p, "war_prep", &format!("team {}", t.0), "probe_rush", 0, String::new);
                        g.team_ai_set_war_plan(team, t, Some(WarPlanType::WARPLAN_PREPARING_TOTAL), false);
                    }
                }
            }
            true
        }
        Probe::Horde => {
            if g.players[p].cities.len() < HORDE_MIN_CITIES {
                return false;
            }
            match preparing {
                Some(t) => horde_declare(g, p, t),
                None => {
                    if let Some(t) = weakest_target(g, p) {
                        crate::trace!(g, p, "probe horde: prepares war on team {:?}, the weakest reachable", t);
                        g.tel_war_reason(team, t, || "probe horde: weakest reachable".to_string());
                        g.tel_decision(p, "war_prep", &format!("team {}", t.0), "probe_horde", 0, String::new);
                        g.team_ai_set_war_plan(team, t, Some(WarPlanType::WARPLAN_PREPARING_TOTAL), false);
                    }
                }
            }
            true
        }
    }
}

/// The civilization teams `p` could prepare a war against: met, alive, with cities, not
/// barbarians, the rules let us declare on them.
fn war_candidates(g: &Game, p: PlayerId) -> Vec<TeamId> {
    let team = g.players[p].team;
    g.team_ids().filter(|&t| t != team && !g.team_is_barbarian(t) && g.team_is_alive(t) && g.teams[t].num_cities > 0 && g.teams[team].has_met[t] && g.team_can_declare_war(team, t)).collect()
}

/// The Rush's target: the nearest civilization by land (`diplomacy::reach_distance`).
fn nearest_target(g: &Game, p: PlayerId) -> Option<TeamId> {
    let team = g.players[p].team;
    war_candidates(g, p).into_iter().filter_map(|t| crate::diplomacy::reach_distance(g, team, t).map(|d| (d, t))).min().map(|(_, t)| t)
}

/// Reachable by land, or by sea (an overseas city we can ship an army to and take): the standard's
/// test for keeping a prepared war.
fn reachable(g: &Game, p: PlayerId, t: TeamId) -> bool {
    let team = g.players[p].team;
    crate::diplomacy::reach_distance(g, team, t).is_some() || crate::diplomacy::team_leader(g, t).is_some_and(|q| crate::naval::amphibious_city(g, p, q).is_some())
}

/// The Horde's target: the reachable civilization of least defensive power (the rules'
/// `getDefensivePower`), the nearer first on a tie.
fn weakest_target(g: &Game, p: PlayerId) -> Option<TeamId> {
    let team = g.players[p].team;
    war_candidates(g, p)
        .into_iter()
        .filter(|&t| reachable(g, p, t))
        .map(|t| (g.team_get_defensive_power(t), crate::diplomacy::reach_distance(g, team, t).unwrap_or(i32::MAX), t))
        .min()
        .map(|(_, _, t)| t)
}

/// The Rush's declaration on prepared target `t`: dropped when `t` is no longer reachable by land
/// or cannot be declared on; declared once our army units near its nearest city cover it with a
/// reserve ([`rush_ready`]).
fn rush_declare(g: &mut Game, p: PlayerId, t: TeamId) {
    let team = g.players[p].team;
    if crate::diplomacy::reach_distance(g, team, t).is_none() || !g.team_can_declare_war(team, t) {
        g.team_ai_set_war_plan(team, t, None, true);
        return;
    }
    let Some((city, force)) = rush_ready(g, p, t) else { return };
    let waited = g.teams[team].ai_war_plan_state_counter[t];
    crate::trace!(g, p, "probe rush: declares war on team {:?}: {} units cover {} (prepared {} turns)", t, force, g.cities[city].name, waited);
    let name = g.cities[city].name.clone();
    g.tel_war_reason(team, t, || format!("probe rush: {force} units cover {name} after {waited} turns"));
    g.team_declare_war(team, t, true, Some(WarPlanType::WARPLAN_TOTAL));
}

/// Target `t`'s city nearest to ours by land (revealed to us), and its distance from our nearest
/// city.
fn nearest_city(g: &Game, p: PlayerId, t: TeamId) -> Option<(i32, CityId)> {
    let team = g.players[p].team;
    let mut target: Option<(i32, CityId)> = None;
    for &c in &g.players[p].cities {
        let area = g.map.plots[g.city_plot(c)].area;
        for q in g.player_ids().filter(|&q| g.players[q].alive && g.players[q].team == t) {
            for &o in &g.players[q].cities {
                let op = g.city_plot(o);
                if g.map.plots[op].area != area || !g.map.plots[op].revealed[team] {
                    continue;
                }
                let dist = g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[o].x, g.cities[o].y);
                if target.map_or(true, |b| (dist, o) < b) {
                    target = Some((dist, o));
                }
            }
        }
    }
    target
}

/// Do our army units on the landmass of enemy city `city` (within `within` plots of it) take it,
/// against the defenders we know of (`combat_sim::covers`, siege engines bombarding; `spare`: with
/// a reserve)? A city whose garrison we cannot see needs `campaign::rough_need` of what such a city
/// usually holds. The number of units counted, when they do.
fn army_takes(g: &Game, p: PlayerId, city: CityId, within: i32, spare: bool) -> Option<i32> {
    let team = g.players[p].team;
    let tp = g.city_plot(city);
    let (tx, ty) = (g.cities[city].x, g.cities[city].y);
    let area = g.map.plots[tp].area;
    let force: Vec<UnitId> = g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| crate::military::is_army_unit(g, u) && !g.unit_is_cargo(u) && crate::campaign::fights(g, u))
        .filter(|&u| g.map.plots[g.unit_plot(u)].area == area && g.plot_distance(g.units[u].x, g.units[u].y, tx, ty) <= within)
        .collect();
    if force.is_empty() {
        return None;
    }
    let takes = if crate::view::sees_units(g, team, tp) {
        let defs = crate::combat_sim::visible_defenders(g, team, tp);
        crate::combat_sim::covers(g, city, tp, &defs, &force, spare, true)
    } else {
        force.len() as i32 >= crate::campaign::rough_need((2 + g.cities[city].population / 5).min(5))
    };
    takes.then_some(force.len() as i32)
}

/// The Rush's readiness against target `t`: its city nearest to ours ([`nearest_city`]) and the
/// units that take it with a reserve ([`army_takes`]), counting our army units no farther from it
/// than our nearest city plus [`RUSH_STAGE_SLACK`] (where the campaign gathers them while the war
/// is prepared).
fn rush_ready(g: &Game, p: PlayerId, t: TeamId) -> Option<(CityId, i32)> {
    let (dist, city) = nearest_city(g, p, t)?;
    army_takes(g, p, city, dist + RUSH_STAGE_SLACK, true).map(|n| (city, n))
}

/// The Horde's declaration on prepared target `t`, by the standard's readiness test
/// (`diplomacy::manage`): the army at its size for the war and staged at the border after 5
/// turns of preparation, or 40 turns in any case; dropped when `t` is out of reach.
fn horde_declare(g: &mut Game, p: PlayerId, t: TeamId) {
    let team = g.players[p].team;
    if !reachable(g, p, t) || !g.team_can_declare_war(team, t) {
        g.team_ai_set_war_plan(team, t, None, true);
        return;
    }
    let need = crate::diplomacy::army_needed_for_war(g, p, t);
    let army = crate::diplomacy::army_size(g, p);
    let ready = army >= need;
    let staged = crate::diplomacy::staged_army(g, p, t) * 3 >= need * 2;
    let waited = g.teams[team].ai_war_plan_state_counter[t];
    if ready && staged && waited >= 5 || waited >= 40 {
        crate::trace!(g, p, "probe horde: declares war on team {:?} (army {} of {}, prepared {} turns)", t, army, need, waited);
        g.tel_war_reason(team, t, || format!("probe horde: army {army} of {need} after {waited} turns"));
        g.team_declare_war(team, t, true, Some(WarPlanType::WARPLAN_TOTAL));
    }
}

/// Does `p` keep its army whatever its upkeep (`finance::disband_military` skipped)? The probes
/// that live by their army: the Rush, the Horde and the Turtle's garrisons.
pub fn keeps_army(g: &Game, p: PlayerId) -> bool {
    matches!(probe(g, p), Some(Probe::Rush | Probe::Horde | Probe::Turtle))
}
