//! Cultural channels (this game's own rules, docs/CULTURE.md; docs/AI.md "Cultural actions").
//!
//! What an expert does with the standing channels, once a turn, after the diplomacy (so the war
//! state is current). A channel is always the civilization's own decision, in a governed city too:
//! governors never set one (the owner's decision, 2026-10-30).
//!
//! 1. **Defence** ([`defend`]). Revolt pressure aimed at one of our cities is held off by its
//!    garrison, each defender resisting `CULTURE_REVOLT_GARRISON` a turn: [`defenders_needed`]
//!    is how many keep the pressure, as it would settle under the channels aimed at the city now,
//!    short of disorder (past it the city makes no culture and the pressure runs on to the flip),
//!    at most six. A directly ruled city short of them trains one first; a governed
//!    city's governor chooses again (`governor::decide` puts them first). A rival whose pressure is
//!    two thirds of the way to taking one of our cities provokes the war logic ([`flips_against`],
//!    read by `diplomacy::territory_check`).
//! 2. **Foment Revolt** ([`channel`]). Cities whose culture is worth spending (not the cities
//!    carrying a cultural victory we pursue; a point counts in full while the city's borders still
//!    grow to its working radius, half after) channel revolt at a rival when it pays. Per target
//!    city, the pressure our channels would build is simulated over [`HORIZON`] turns as the rules
//!    run it (the inflow, less the city's resistance as we reckon it: its culture and its
//!    stationed defenders, [`reckoned_resistance`]; faded): a city taken (never a capital) is
//!    worth its people, buildings, wonders and holy cities, amortized over the turns it takes and
//!    five more; disorder in a rival's city its lost production and commerce, unrest little; the outcome
//!    counts only as far as our channels change it. The rival's lost output counts in full at war,
//!    while we prepare one, against the barbarians or the civilization racing us to a victory,
//!    less at peace (60% if we dislike them, 40% if Cautious), and a city taken at peace three
//!    quarters (it invites war). Against that: the culture spent, and at peace the target's
//!    memory of it (a cost a turn, larger the better they think of us). Spared: civilizations we
//!    are Pleased or Friendly with, vassals and masters, defensive pact partners, and at peace any
//!    whose defensive power is above 125% of ours (150% for a channel that stands). The leader's
//!    `iCulturalAggression` scales the value, halfway from 100%, a conqueror's
//!    (`campaign::conqueror`: Aggressive leaders, the warlike civilizations) by another quarter.
//!    Our cities' channels at one civilization add up, so each target is
//!    weighed with the group of our cities that would channel at it (the strongest first, at most
//!    `CULTURE_AI_MAX_HOSTILE_PER_TARGET`, twice that at war or against the barbarians), and the
//!    groups are chosen greedily, best first.
//! 3. **Spread Religion** ([`religion_value`]). A city spreads our state religion, or a religion
//!    whose shrine we hold, into a civilization whose cities lack it: the chance a turn (from the
//!    rules' reach) times what a spread is worth, against the culture spent. Our own cities first
//!    (happiness, the religion's buildings), then friends and neutrals (a shrine's gold, a
//!    religion-less civilization drawn toward ours, a co-religionist's friendship), into a
//!    civilization with another state religion only where we dislike it and a shrine pays (the
//!    spread is remembered as hostile).
//!
//! **Stability.** Stopping a revolt channel lets the pressure it built fade, so a standing channel
//! is kept while it is worth [`KEEP_PCT`] of what it costs (in a group, the weakest dropped first),
//! a new one must be worth a quarter more than its cost (`Caster::entry`), and a city spreading a
//! religion moves to another channel only when that is clearly better (by half its cost).
//!
//! A human's governed cities ([`govern`], through `Controller::govern_cities`): the governor
//! trains defenders against revolt pressure (production is its job); the channels are the
//! player's.
//!
//! Switch `CULTURE` (`FFH_AI_CULTURE_OFF`): off, a civilization channels nothing and its cultural
//! defence is only the governors' (`governor::decide`).

use ffh_core::culture::ChannelReach;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// Turns over which a revolt channel's outcome is weighed.
const HORIZON: i32 = 40;
/// A standing channel is kept while it is worth this share (percent) of what it costs: stopping
/// lets the pressure it built fade, so a channel is not given up for a small swing.
const KEEP_PCT: i64 = 70;
/// Most defenders the cultural defence asks for in one city.
const MAX_DEFENDERS: i32 = 6;

/// A channel: action, target civilization, religion.
type Order = (CulturalActionType, PlayerId, Option<ReligionType>);

/// The AI's cultural play, once a turn.
pub fn turn(g: &mut Game, p: PlayerId) {
    let on = crate::switches::CULTURE.on(g, p);
    turn_switched(g, p, on);
}

/// [`turn`] with switch `CULTURE` read.
fn turn_switched(g: &mut Game, p: PlayerId, on: bool) {
    if !on || !g.culture_actions_on() || g.player_is_barbarian(p) || g.players[p].cities.is_empty() {
        return;
    }
    defend(g, p, false);
    channel(g, p);
}

/// The governors of a human player's governed cities train defenders against the revolt pressure
/// aimed at them (docs/GOVERNORS.md); they never set a channel.
pub fn govern(g: &mut Game, p: PlayerId) {
    if !g.culture_actions_on() || g.player_is_barbarian(p) || g.players[p].cities.is_empty() {
        return;
    }
    defend(g, p, true);
}

/// The cultural action with `effect`, if the data has one.
fn action(g: &Game, effect: CultureEffect) -> Option<CulturalActionType> {
    g.d.cultural_actions.keys().find(|&a| g.d.cultural_actions[a].effect == effect)
}

/// Whether revolt pressure can take city `c` (never a capital; not with
/// `GAMEOPTION_NO_CITY_FLIPPING`).
fn can_flip(g: &Game, c: CityId) -> bool {
    !g.city_is_capital(c) && !g.is_option(GameOptionType::GAMEOPTION_NO_CITY_FLIPPING)
}

// ----------------------------------------------------------------------------
// Defence
// ----------------------------------------------------------------------------

/// The revolt pressure a turn (hundredths) each caster's channels put on each of `owner`'s cities
/// now: (city, caster, inflow).
fn inflows_on(g: &Game, owner: PlayerId) -> Vec<(CityId, PlayerId, i32)> {
    let team = g.players[owner].team;
    let mut out: Vec<(CityId, PlayerId, i32)> = Vec::new();
    for ch in g.culture_channels() {
        if ch.channel.target != owner || g.players[ch.caster].team == team || g.d.cultural_actions[ch.channel.action].effect != CultureEffect::FomentRevolt {
            continue;
        }
        for x in g.culture_channel_reach(ch.city, ch.channel.action, owner, None) {
            match out.iter_mut().find(|e| e.0 == x.city && e.1 == ch.caster) {
                Some(e) => e.2 += x.amount,
                None => out.push((x.city, ch.caster, x.amount)),
            }
        }
    }
    out
}

/// Where revolt pressure settles under a steady `inflow` against `resist` (both hundredths a
/// turn): the net inflow over the fade.
fn steady(g: &Game, inflow: i32, resist: i32) -> i64 {
    let fade = g.d.defines.culture_revolt_fade_percent.max(1) as i64;
    (inflow - resist).max(0) as i64 * 100 / fade
}

/// Defenders city `c` needs against the revolt pressure aimed at it now: enough that all of it, as
/// it would settle, stays short of disorder, at most [`MAX_DEFENDERS`]; 0 when nothing presses it.
/// That also keeps the city from being taken: a city in revolt makes no culture (disorder stops its
/// commerce), so past disorder only its garrison resists and the pressure runs on to the flip.
pub(crate) fn defenders_needed(g: &Game, c: CityId) -> i32 {
    if !g.culture_actions_on() {
        return 0;
    }
    needed_with(g, c, &inflows_on(g, g.cities[c].owner))
}

/// [`defenders_needed`] with the inflows on its owner's cities ([`inflows_on`]).
fn needed_with(g: &Game, c: CityId, inflows: &[(CityId, PlayerId, i32)]) -> i32 {
    let defines = &g.d.defines;
    let per = 100 * defines.culture_revolt_garrison;
    let mine: Vec<i32> = inflows.iter().filter(|x| x.0 == c).map(|x| x.2).collect();
    if mine.is_empty() || per <= 0 {
        return 0;
    }
    // The city's own part of its resistance (its culture), without its present garrison.
    let own = g.city_revolt_resistance(c) - per * g.culture_garrison_count(c);
    let disorder = 100 * defines.culture_revolt_disorder as i64;
    (0..=MAX_DEFENDERS).find(|&n| mine.iter().map(|&i| steady(g, i, own + per * n)).sum::<i64>() < disorder).unwrap_or(MAX_DEFENDERS)
}

/// Our cities short of the defenders that hold off the revolt pressure aimed at them train one
/// first: a directly ruled city (the AI's own play) puts one at the front of its queue, a governed
/// city's governor chooses again (its choice then puts them first). `only_governed`: a human's
/// governed cities alone.
fn defend(g: &mut Game, p: PlayerId, only_governed: bool) {
    let inflows = inflows_on(g, p);
    if inflows.is_empty() {
        return;
    }
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) || !inflows.iter().any(|x| x.0 == c) {
            continue;
        }
        let governed = g.city_is_governed(c);
        if only_governed && !governed {
            continue;
        }
        let (need, have) = (needed_with(g, c, &inflows), g.culture_garrison_count(c));
        if have >= need || trains_a_defender(g, c) {
            continue;
        }
        let Some(u) = crate::production::best_unit(g, c, crate::production::Role::Defense) else { continue };
        let name = g.d.text.name(&g.cities[c].name).to_string();
        if governed {
            crate::trace!(g, p, "culture: the governor of {name} trains defenders against revolt pressure ({have} of {need})");
            g.city_clear_order_queue(c);
            g.city_choose_production(c, None, None, None, false, false);
        } else {
            crate::trace!(g, p, "culture: a defender first in {name} against revolt pressure ({have} of {need})");
            g.city_push_order(c, OrderType::ORDER_TRAIN, u.i(), UnitAIType::UNITAI_CITY_DEFENSE.i(), false, false, false, false);
        }
        g.tel_decision(p, "culture", "garrison", "defend", (need - have) as i64, String::new);
    }
}

/// Whether city `c` is training a defender that will stay in it: a city defender whose order
/// leaves it in the city's own garrison (an attacker, or a unit bound for an army, adds no lasting
/// resistance).
fn trains_a_defender(g: &Game, c: CityId) -> bool {
    let Some(o) = g.cities[c].order_queue.first() else { return false };
    if o.order_type != OrderType::ORDER_TRAIN || g.city_production_unit_ai(c) != Some(UnitAIType::UNITAI_CITY_DEFENSE) {
        return false;
    }
    match o.dest.or(g.cities[c].recruit_default) {
        None => true,
        Some(ffh_core::army::RecruitDest::Garrison(to)) => to == c,
        Some(ffh_core::army::RecruitDest::Army(_)) => false,
    }
}

/// Whether `other`'s revolt pressure is close to taking one of `p`'s cities (two thirds of the
/// way, in a city it can take): a provocation (`diplomacy::territory_check`).
pub(crate) fn flips_against(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let flip = 100 * g.d.defines.culture_revolt_flip;
    g.players[p].cities.iter().any(|&c| g.cities[c].culture_pressure[other] * 3 >= flip * 2 && can_flip(g, c))
}

// ----------------------------------------------------------------------------
// The channels
// ----------------------------------------------------------------------------

/// What the planning knows of player `p` this turn.
struct Ctx {
    p: PlayerId,
    team: TeamId,
    revolt: Option<CulturalActionType>,
    spread: Option<CulturalActionType>,
    /// Percent the value of a revolt counts: halfway from 100 to the leader's
    /// `iCulturalAggression`, a conqueror's a quarter more.
    aggression: i64,
    /// Our cities carrying a cultural victory we pursue: they keep their culture.
    carriers: Vec<CityId>,
    state: Option<ReligionType>,
    /// Our team's power (with its vassals), at least 1.
    power: i64,
    /// The team racing us to a victory.
    rival: Option<TeamId>,
    fledgling: Option<CultureLevelType>,
}

impl Ctx {
    fn new(g: &Game, p: PlayerId) -> Ctx {
        let team = g.players[p].team;
        let plan = crate::victory::plan(g, p);
        let mut carriers: Vec<CityId> = Vec::new();
        if plan.goal() == Some(crate::victory::Goal::Culture) {
            // A cultural victory needs `iNumCultureCities` legendary cities: our best ones keep
            // their culture.
            let n = g.d.victories.keys().find(|&v| g.d.victories[v].num_culture_cities > 0).map_or(3, |v| g.d.victories[v].num_culture_cities) as usize;
            let mut by_culture: Vec<(i32, CityId)> = g.players[p].cities.iter().map(|&c| (-g.cities[c].culture[p], c)).collect();
            by_culture.sort();
            carriers = by_culture.into_iter().take(n).map(|(_, c)| c).collect();
        }
        // Half the leader's lean either way (40-180 gives 70-140%): a peaceful leader still takes
        // a city that clearly pays.
        let leader = (100 + g.players[p].personality_type.map_or(100, |l| g.d.leader_heads[l].cultural_aggression) as i64) / 2;
        let aggression = if crate::campaign::conqueror(g, p) {
            leader * 5 / 4
        } else {
            leader
        };
        Ctx {
            p,
            team,
            revolt: action(g, CultureEffect::FomentRevolt),
            spread: action(g, CultureEffect::SpreadReligion),
            aggression,
            carriers,
            state: g.player_get_state_religion(p),
            power: g.team_get_power(team, true).max(1) as i64,
            rival: crate::victory::rival_team(g, &plan),
            fledgling: g.d.ty::<CultureLevelType>("CULTURELEVEL_FLEDGLING"),
        }
    }
}

/// One of our cities as the planning sees it.
struct Caster {
    city: CityId,
    /// The culture a turn it would channel.
    amount: i32,
    /// What a point of its culture is worth to us, in percent of a commerce: all of it while the
    /// city's borders still grow to its working radius (`CULTURELEVEL_FLEDGLING`), half after; 0
    /// in a city carrying a cultural victory we pursue (it spends none).
    weight: i64,
    /// Its channel now.
    current: Option<Order>,
}

impl Caster {
    /// What its channel costs a turn, in hundredths of a commerce.
    fn cost(&self) -> i64 {
        self.amount as i64 * self.weight
    }

    /// What a better channel must beat its present religion channel by (hundredths a turn).
    fn margin(&self) -> i64 {
        (self.cost() / 2).max(100)
    }

    /// What a new channel must be worth beyond its cost (hundredths a turn): a quarter of it, and
    /// at least a commerce, so that one at the edge does not start and stop with every swing.
    fn entry(&self) -> i64 {
        (self.cost() / 4).max(100)
    }
}

fn casters(g: &Game, ctx: &Ctx) -> Vec<Caster> {
    let mut out = Vec::new();
    for &c in &g.players[ctx.p].cities {
        let city = &g.cities[c];
        let current = city.culture_channel.as_ref().map(|ch| (ch.action, ch.target, ch.religion));
        let weight = if ctx.carriers.contains(&c) {
            0
        } else {
            let grown = match (city.culture_level, ctx.fledgling) {
                (Some(l), Some(f)) => l >= f,
                _ => false,
            };
            if grown { 50 } else { 100 }
        };
        out.push(Caster { city: c, amount: g.city_channel_amount(c), weight, current });
    }
    out
}

/// How our revolt channels against one civilization count.
#[derive(Clone, Copy, Debug)]
struct Stance {
    /// Percent of a rival city's lost output that counts for us.
    harm: i64,
    /// Percent of a city's worth that counts when we take it (at peace a flip invites war).
    take: i64,
    /// The diplomatic cost a turn of each hostile channel (hundredths of a commerce): the target
    /// remembers it.
    diplo: i64,
    /// At war (or against the barbarians): no memory to spare, a larger group.
    war: bool,
}

/// Whether we foment revolt against `t`, and how it counts; `Err` names why it is spared.
/// `keeping`: for a channel that stands, which a power swing does not end at once (a stronger
/// civilization is spared above 125% of our power for a new channel, 150% for a standing one).
fn revolt_stance(g: &Game, ctx: &Ctx, t: PlayerId, keeping: bool) -> Result<Stance, &'static str> {
    let tp = &g.players[t];
    if !tp.alive || tp.cities.is_empty() {
        return Err("gone");
    }
    let (team, ot) = (ctx.team, tp.team);
    if ot == team {
        return Err("own_team");
    }
    if g.player_is_barbarian(t) {
        return Ok(Stance { harm: 100, take: 100, diplo: 0, war: true });
    }
    if !g.teams[team].has_met[ot] {
        return Err("not_met");
    }
    if g.teams[team].vassal[ot] || g.teams[ot].vassal[team] {
        return Err("vassal");
    }
    if g.teams[team].defensive_pact[ot] {
        return Err("pact");
    }
    if g.at_war(team, ot) {
        return Ok(Stance { harm: 100, take: 100, diplo: 0, war: true });
    }
    let attitude = crate::diplomacy::attitude(g, ctx.p, t);
    if attitude.i() >= AttitudeType::ATTITUDE_PLEASED.i() {
        return Err("friend");
    }
    // A far stronger civilization we would provoke (a city close to joining us is a provocation
    // for its war logic, as for ours).
    if g.team_get_defensive_power(ot) as i64 * 100 > ctx.power * if keeping { 150 } else { 125 } {
        return Err("stronger");
    }
    if crate::diplomacy::preparing_target(g, team) == Some(ot) {
        return Ok(Stance { harm: 100, take: 100, diplo: 50, war: false });
    }
    // How much their losses count reads what we think of them; what the channel costs in their
    // memory, what they think of us (more to lose the better it is).
    let harm = match attitude {
        AttitudeType::ATTITUDE_FURIOUS | AttitudeType::ATTITUDE_ANNOYED => 60,
        _ => 40,
    };
    let diplo = match crate::diplomacy::attitude(g, t, ctx.p) {
        AttitudeType::ATTITUDE_FURIOUS => 50,
        AttitudeType::ATTITUDE_ANNOYED => 100,
        _ => 250,
    };
    Ok(Stance { harm: if ctx.rival == Some(ot) { 100 } else { harm }, take: 75, diplo, war: false })
}

/// What a city is worth to whoever holds it, in commerce-equivalent points: its people, its
/// buildings, its wonders and holy cities.
fn city_worth(g: &Game, t: CityId) -> i64 {
    let city = &g.cities[t];
    let holy = g.d.religions.keys().filter(|&r| g.get_holy_city(r) == Some(t)).count() as i64;
    150 + 50 * city.population as i64 + 20 * city.num_buildings as i64 + 150 * city.num_world_wonders as i64 + 200 * holy
}

/// What our revolt pressure on a city does over [`HORIZON`] turns.
#[derive(Clone, Copy, Debug, Default)]
struct Sim {
    /// The turn it joins us.
    flip: Option<i32>,
    /// Turns in disorder (all casters' pressure together), before a flip.
    disorder: i32,
    /// Points of unhappiness times turns, before a flip.
    unhappy: i32,
}

/// The revolt pressure city `t` resists a turn as we reckon it (hundredths): its own culture and
/// the defenders stationed there, those that did not move last turn (fortifying): an army passing
/// through resists while it is there, but does not stay (a city whose stack came and went every
/// turn had a channel set and ended in turn). As (its culture's part, its garrison's).
fn reckoned_resistance(g: &Game, t: CityId) -> (i32, i32) {
    let team = g.city_team(t);
    let plot = g.city_plot(t);
    let passing = g.map.plots[plot]
        .units
        .iter()
        .filter(|&&u| {
            g.units[u].fortify_turns == 0
                && g.unit_get_team(u) == team
                && g.d.units[g.units[u].unit_type].domain_type == Some(DomainType::DOMAIN_LAND)
                && g.unit_can_defend(u, Some(plot))
        })
        .count() as i32;
    let per = 100 * g.d.defines.culture_revolt_garrison;
    let garrison = per * (g.culture_garrison_count(t) - passing);
    (g.city_revolt_resistance(t) - per * g.culture_garrison_count(t), garrison)
}

/// Our pressure on city `t` under an `inflow` a turn (hundredths), as the rules run it: added in
/// our cities' turns, then resisted ([`reckoned_resistance`]) and faded in its own; the other
/// casters' pressure as it is. A city in revolt makes no culture (disorder stops its commerce), so
/// from the turn after its pressure reaches disorder only its garrison resists.
fn simulate(g: &Game, p: PlayerId, t: CityId, inflow: i32) -> Sim {
    let defines = &g.d.defines;
    let (own, garrison) = reckoned_resistance(g, t);
    let fade = defines.culture_revolt_fade_percent.max(0);
    let flip = 100 * defines.culture_revolt_flip;
    let disorder = 100 * defines.culture_revolt_disorder;
    let step = 100 * defines.culture_revolt_unhappy_step.max(1);
    let most = defines.culture_revolt_unhappy_max.max(0);
    let flips = can_flip(g, t);
    let mut ours = g.cities[t].culture_pressure[p];
    let others = g.city_revolt_pressure(t) - ours;
    let mut in_revolt = g.city_revolt_pressure(t) >= disorder;
    let mut sim = Sim::default();
    for n in 1..=HORIZON {
        ours += inflow;
        if ours > 0 {
            let resist = if in_revolt { garrison } else { own + garrison };
            ours = (ours - resist - (ours * fade / 100).max(100)).max(0);
        }
        if flips && ours >= flip {
            sim.flip = Some(n);
            return sim;
        }
        let total = ours + others;
        in_revolt = total >= disorder;
        if in_revolt {
            sim.disorder += 1;
        }
        sim.unhappy += (total / step).min(most);
    }
    sim
}

/// What our pressure on city `t` under `inflow` is worth a turn (hundredths of a commerce): the
/// city taken, amortized over the turns it takes and five more (a city is not put to use at once;
/// with no floor at all a flip a turn sooner is worth a fortune, with a hard floor nothing, and a
/// group's members dropped in turn as each looked redundant), and its lost output while in
/// disorder; else the disorder and the unrest over the horizon.
fn city_rate(g: &Game, st: &Stance, p: PlayerId, t: CityId, inflow: i32) -> (i64, Sim) {
    let sim = simulate(g, p, t, inflow);
    let output = (g.city_yield_rate(t, YieldType::YIELD_PRODUCTION) + g.city_yield_rate(t, YieldType::YIELD_COMMERCE)).max(0) as i64;
    // A turn of disorder costs the city its output; a point of unhappiness about half a commerce.
    let lesser = (100 * output * sim.disorder as i64 + 50 * sim.unhappy as i64) * st.harm / 100;
    let rate = match sim.flip {
        Some(n) => (100 * city_worth(g, t) * st.take / 100 + lesser) / (n + 5) as i64,
        None => lesser / HORIZON as i64,
    };
    (rate, sim)
}

/// What a group of our channels at one civilization does beyond what is there without them.
#[derive(Clone, Copy, Debug, Default)]
struct Eval {
    /// Hundredths of a commerce a turn (before the leader's aggression).
    value: i64,
    flips: i32,
    revolts: i32,
}

impl Eval {
    fn why(&self) -> &'static str {
        if self.flips > 0 {
            "revolt_flip"
        } else if self.revolts > 0 {
            "revolt_disorder"
        } else {
            "revolt_unrest"
        }
    }
}

/// What the revolt channels of `members` (indices into `reach`) at one civilization do.
fn group_value(g: &Game, p: PlayerId, st: &Stance, reach: &[&[ChannelReach]], members: &[usize]) -> Eval {
    let mut inflow: Vec<(CityId, i32)> = Vec::new();
    for &m in members {
        for x in reach[m] {
            match inflow.iter_mut().find(|e| e.0 == x.city) {
                Some(e) => e.1 += x.amount,
                None => inflow.push((x.city, x.amount)),
            }
        }
    }
    let mut ev = Eval::default();
    for (t, amount) in inflow {
        let (with, sim) = city_rate(g, st, p, t, amount);
        let (without, base) = city_rate(g, st, p, t, 0);
        ev.value += with - without;
        if sim.flip.is_some() && base.flip.is_none() {
            ev.flips += 1;
        } else if sim.disorder > base.disorder {
            ev.revolts += 1;
        }
    }
    ev
}

/// For the trace: each target city the group reaches, with its resistance, our pressure, the
/// group's inflow and what it would do.
fn group_detail(g: &Game, p: PlayerId, reach: &[&[ChannelReach]], members: &[usize]) -> String {
    let mut inflow: Vec<(CityId, i32)> = Vec::new();
    for &m in members {
        for x in reach[m] {
            match inflow.iter_mut().find(|e| e.0 == x.city) {
                Some(e) => e.1 += x.amount,
                None => inflow.push((x.city, x.amount)),
            }
        }
    }
    let parts: Vec<String> = inflow
        .iter()
        .map(|&(t, a)| {
            let sim = simulate(g, p, t, a);
            format!(
                "{} (resist {}, {} defenders, ours {}, all {}, inflow {}: flip {:?}, disorder {})",
                g.d.text.name(&g.cities[t].name),
                {
                    let (own, garrison) = reckoned_resistance(g, t);
                    (own + garrison) / 100
                },
                g.culture_garrison_count(t),
                g.cities[t].culture_pressure[p] / 100,
                g.city_revolt_pressure(t) / 100,
                a / 100,
                sim.flip,
                sim.disorder
            )
        })
        .collect();
    parts.join("; ")
}

/// The chance a turn of city `c`'s spreading religion `r` into civilization `t`, times what a
/// spread is worth ([`spread_value`]), over the cities we know of (hundredths of a commerce a
/// turn).
fn religion_value(g: &Game, ctx: &Ctx, c: CityId, r: ReligionType, t: PlayerId) -> i64 {
    let Some(spread) = ctx.spread else { return 0 };
    let reach: Vec<ChannelReach> = g.culture_channel_reach(c, spread, t, Some(r)).into_iter().filter(|x| x.amount > 0 && g.cities[x.city].revealed[ctx.team]).collect();
    let total: i64 = reach.iter().map(|x| x.amount as i64).sum();
    if total == 0 {
        return 0;
    }
    let chance = g.culture_spread_chance(&reach) as i64;
    let avg = reach.iter().map(|x| x.amount as i64 * spread_value(g, ctx, x.city, r)).sum::<i64>() / total;
    chance * avg / 10000
}

/// What religion `r` spreading to city `t` is worth to us (hundredths of a commerce): our state
/// religion in our own city its happiness and buildings; a shrine of ours a gold a turn for every
/// city with its religion; our state religion abroad draws a civilization without one toward it
/// (the AI adopts the religion most of its cities have: the first city with ours may bring it to
/// our faith, and a shared state religion is a step of attitude both ways), pleases a
/// co-religionist (it remembers the spread as friendship), and in a civilization with another
/// state religion is remembered as hostile.
fn spread_value(g: &Game, ctx: &Ctx, t: CityId, r: ReligionType) -> i64 {
    let owner = g.cities[t].owner;
    let shrine = if g.team_has_shrine(ctx.team, Some(r)) { 2500 } else { 0 };
    let state = ctx.state == Some(r);
    if owner == ctx.p {
        return if state { 4000 + 600 * g.cities[t].population as i64 } else { shrine };
    }
    if !state {
        return shrine;
    }
    if g.players[owner].team == ctx.team {
        return shrine + 2000;
    }
    match g.player_get_state_religion(owner) {
        Some(s) if s != r => shrine + 300 - 1500,
        Some(_) => shrine + 1500,
        None if g.players[owner].has_religion_count[r] == 0 => shrine + 5000,
        None => shrine + 2000,
    }
}

/// The religions city `c` may spread: our state religion, and any whose shrine we hold.
fn religions(g: &Game, ctx: &Ctx, c: CityId) -> Vec<ReligionType> {
    g.d.religions.keys().filter(|&r| g.cities[c].has_religion[r] && (ctx.state == Some(r) || g.team_has_shrine(ctx.team, Some(r)))).collect()
}

/// The civilizations we spread religion `r` into: ours and our team's, and those at peace with
/// us that we have met (a hostile spread only into one we dislike).
fn religion_targets(g: &Game, ctx: &Ctx, r: ReligionType) -> Vec<PlayerId> {
    let Some(spread) = ctx.spread else { return Vec::new() };
    let mut out = Vec::new();
    for t in g.player_ids() {
        let tp = &g.players[t];
        if !tp.alive || tp.cities.is_empty() || g.player_is_barbarian(t) {
            continue;
        }
        if tp.team != ctx.team {
            if !g.teams[ctx.team].has_met[tp.team] || g.at_war(ctx.team, tp.team) {
                continue;
            }
            if g.culture_is_hostile(spread, ctx.p, t, Some(r)) && crate::diplomacy::attitude(g, ctx.p, t).i() > AttitudeType::ATTITUDE_ANNOYED.i() {
                continue;
            }
        }
        out.push(t);
    }
    out
}

/// City `k`'s best religion channel: (value less cost, religion, target).
fn best_religion(g: &Game, ctx: &Ctx, k: &Caster, alts: &mut Vec<(String, i64)>) -> Option<(i64, ReligionType, PlayerId)> {
    let spread = ctx.spread?;
    if k.amount <= 0 || k.weight == 0 {
        return None;
    }
    let mut best: Option<(i64, ReligionType, PlayerId)> = None;
    for r in religions(g, ctx, k.city) {
        for t in religion_targets(g, ctx, r) {
            if g.culture_channel_check(k.city, spread, t, Some(r)).is_err() {
                continue;
            }
            let net = religion_value(g, ctx, k.city, r, t) - k.cost();
            if g.telemetry.on() {
                alts.push((format!("{}@p{}", g.d.religions[r].base.type_, t.0), net / 100));
            }
            if best.map_or(true, |b| net > b.0) {
                best = Some((net, r, t));
            }
        }
    }
    best
}

/// A decision of this turn's planning, applied at the end.
struct Change {
    order: Option<Order>,
    why: &'static str,
    /// Value less cost, hundredths of a commerce a turn.
    net: i64,
    alts: Vec<(String, i64)>,
    /// For the trace.
    detail: String,
}

/// Sets, keeps or ends each city's channel (see the module comment).
fn channel(g: &mut Game, p: PlayerId) {
    let ctx = Ctx::new(g, p);
    let casters = casters(g, &ctx);
    if casters.is_empty() {
        return;
    }
    let mut assign: Vec<Option<Order>> = casters.iter().map(|k| k.current).collect();
    let mut changes: Vec<Option<Change>> = casters.iter().map(|_| None).collect();
    let stop = |changes: &mut Vec<Option<Change>>, assign: &mut Vec<Option<Order>>, i: usize, why: &'static str| {
        assign[i] = None;
        changes[i] = Some(Change { order: None, why, net: 0, alts: Vec::new(), detail: String::new() });
    };

    // The civilizations we may foment revolt against, and our cities' reach on each.
    // (A standing channel's target, `open` when a new channel may be aimed at it too.)
    let mut targets: Vec<(PlayerId, Stance)> = Vec::new();
    let mut open: Vec<bool> = Vec::new();
    let mut spared: Vec<(PlayerId, &'static str)> = Vec::new();
    if ctx.revolt.is_some() {
        for t in g.player_ids() {
            if t == p {
                continue;
            }
            match revolt_stance(g, &ctx, t, true) {
                Ok(st) => {
                    targets.push((t, st));
                    open.push(revolt_stance(g, &ctx, t, false).is_ok());
                }
                Err(why) => spared.push((t, why)),
            }
        }
    }
    let revolt = ctx.revolt;
    let reach: Vec<Vec<Vec<ChannelReach>>> = casters
        .iter()
        .map(|k| {
            targets
                .iter()
                .map(|&(t, _)| match revolt {
                    Some(a) if k.amount > 0 => g.culture_channel_reach(k.city, a, t, None).into_iter().filter(|x| g.cities[x.city].revealed[ctx.team]).collect(),
                    _ => Vec::new(),
                })
                .collect()
        })
        .collect();
    let reach_of = |ti: usize| -> Vec<&[ChannelReach]> { reach.iter().map(|r| r[ti].as_slice()).collect() };
    let members_at = |assign: &[Option<Order>], t: PlayerId| -> Vec<usize> {
        (0..casters.len()).filter(|&i| matches!(assign[i], Some((a, x, _)) if Some(a) == revolt && x == t)).collect()
    };

    // 1. Standing channels that should end: a city that keeps its culture now, a revolt against a
    //    civilization now spared.
    for (i, k) in casters.iter().enumerate() {
        let Some((a, t, _)) = k.current else { continue };
        let why = if k.weight == 0 {
            Some("keep_culture")
        } else if Some(a) == revolt {
            spared.iter().find(|s| s.0 == t).map(|s| s.1)
        } else {
            None
        };
        if let Some(why) = why {
            stop(&mut changes, &mut assign, i, why);
        }
    }

    // 2. Standing revolt channels worth less than KEEP_PCT of their cost end, the weakest first.
    //    A city that channels nothing this turn (in disorder, in anarchy) costs nothing and keeps
    //    its channel: the rules leave it standing.
    for (ti, &(t, st)) in targets.iter().enumerate() {
        let r = reach_of(ti);
        loop {
            let members = members_at(&assign, t);
            if members.is_empty() {
                break;
            }
            let full = group_value(g, p, &st, &r, &members).value;
            let mut worst: Option<(i64, usize, i64)> = None;
            for &m in members.iter().filter(|&&m| casters[m].amount > 0) {
                let without: Vec<usize> = members.iter().copied().filter(|&x| x != m).collect();
                let marginal = (full - group_value(g, p, &st, &r, &without).value) * ctx.aggression / 100;
                let slack = marginal * 100 - KEEP_PCT * (casters[m].cost() + st.diplo);
                if slack < 0 && worst.map_or(true, |w| slack < w.0) {
                    worst = Some((slack, m, marginal));
                }
            }
            match worst {
                Some((_, m, marginal)) => {
                    stop(&mut changes, &mut assign, m, "worthless");
                    if crate::trace_on(p) {
                        let d = group_detail(g, p, &r, &members);
                        if let Some(ch) = changes[m].as_mut() {
                            ch.detail = format!("revolt at p{}: worth {} a turn of its {}; {d}", t.0, marginal / 100, (casters[m].cost() + st.diplo) / 100);
                        }
                    }
                }
                None => break,
            }
        }
    }

    // 3. Standing religion channels: kept while worth KEEP_PCT of their cost; their worth now.
    let mut current_net: Vec<i64> = vec![0; casters.len()];
    for (i, k) in casters.iter().enumerate() {
        let Some((a, t, Some(r))) = assign[i] else { continue };
        if Some(a) != ctx.spread {
            continue;
        }
        let value = religion_value(g, &ctx, k.city, r, t);
        current_net[i] = value - k.cost();
        if value * 100 < KEEP_PCT * k.cost() {
            stop(&mut changes, &mut assign, i, "worthless");
            if let Some(ch) = changes[i].as_mut().filter(|_| crate::trace_on(p)) {
                ch.detail = format!("religion at p{}: worth {} a turn of its {}", t.0, value / 100, k.cost() / 100);
            }
        }
    }

    // 4. Each city's best religion channel, and what it gives up by fomenting revolt
    //    instead: that channel's worth, or its present one's and the margin to leave it.
    let mut rel_best: Vec<Option<(i64, ReligionType, PlayerId)>> = vec![None; casters.len()];
    let mut rel_alts: Vec<Vec<(String, i64)>> = vec![Vec::new(); casters.len()];
    let mut opp: Vec<i64> = vec![0; casters.len()];
    for (i, k) in casters.iter().enumerate() {
        let idle = assign[i].is_none();
        let spreading = matches!(assign[i], Some((a, _, _)) if Some(a) == ctx.spread);
        if !(idle || spreading) {
            continue;
        }
        rel_best[i] = best_religion(g, &ctx, k, &mut rel_alts[i]);
        opp[i] = if spreading { current_net[i] + k.margin() } else { rel_best[i].map_or(0, |b| b.0.max(0)) };
    }

    // 5. Revolt: the best group of our cities at the best target, greedily, while one pays.
    if let Some(revolt) = revolt {
        loop {
            let mut best: Option<(i64, usize, Vec<usize>, Eval)> = None;
            let mut alts: Vec<(String, i64)> = Vec::new();
            for (ti, &(t, st)) in targets.iter().enumerate() {
                if !open[ti] {
                    continue;
                }
                let base = members_at(&assign, t);
                let cap = (g.d.defines.culture_ai_max_hostile_per_target.max(1) * if st.war { 2 } else { 1 }) as usize;
                if base.len() >= cap {
                    continue;
                }
                let mut cands: Vec<(i32, usize)> = (0..casters.len())
                    .filter(|&i| {
                        let k = &casters[i];
                        let open = assign[i].is_none() || matches!(assign[i], Some((a, _, _)) if Some(a) == ctx.spread);
                        open && k.weight > 0 && !reach[i][ti].is_empty()
                    })
                    .map(|i| (reach[i][ti].iter().map(|x| x.amount).max().unwrap_or(0), i))
                    .collect();
                if cands.is_empty() {
                    continue;
                }
                cands.sort_by_key(|&(amount, i)| (std::cmp::Reverse(amount), casters[i].city));
                let r = reach_of(ti);
                let base_value = group_value(g, p, &st, &r, &base).value;
                let mut group = base.clone();
                let mut added: Vec<usize> = Vec::new();
                let mut cost = 0;
                let mut best_here = i64::MIN;
                for &(_, i) in cands.iter().take(cap - base.len()) {
                    group.push(i);
                    added.push(i);
                    cost += casters[i].cost() + st.diplo + opp[i] + casters[i].entry();
                    let ev = group_value(g, p, &st, &r, &group);
                    let net = (ev.value - base_value) * ctx.aggression / 100 - cost;
                    best_here = best_here.max(net);
                    if net > 0 && best.as_ref().map_or(true, |b| net > b.0) {
                        best = Some((net, ti, added.clone(), ev));
                    }
                }
                if g.telemetry.on() {
                    alts.push((format!("revolt@p{}", t.0), best_here / 100));
                }
            }
            let Some((net, ti, added, ev)) = best else { break };
            let (t, st) = targets[ti];
            let share = net / added.len() as i64;
            let detail = if crate::trace_on(p) {
                let mut all = members_at(&assign, t);
                all.extend(added.iter().copied());
                format!("{} of {}: {} to take, {} in revolt, {:?}; {}", added.len(), all.len(), ev.flips, ev.revolts, st, group_detail(g, p, &reach_of(ti), &all))
            } else {
                String::new()
            };
            for i in added {
                assign[i] = Some((revolt, t, None));
                changes[i] = Some(Change { order: assign[i], why: ev.why(), net: share, alts: alts.clone(), detail: detail.clone() });
            }
        }
    }

    // 6. Religion for the cities left: an idle one takes its best channel when it pays, a
    //    spreading one moves only when that is clearly better.
    for (i, k) in casters.iter().enumerate() {
        let Some((net, r, t)) = rel_best[i] else { continue };
        let Some(spread) = ctx.spread else { break };
        let order = Some((spread, t, Some(r)));
        let take = match assign[i] {
            None => net > k.entry(),
            Some(cur) if Some(cur.0) == ctx.spread => order != Some(cur) && net > current_net[i] + k.margin(),
            Some(_) => false,
        };
        if take {
            assign[i] = order;
            let why = if g.players[t].team == ctx.team {
                "religion_home"
            } else if g.culture_is_hostile(spread, p, t, Some(r)) {
                "religion_hostile"
            } else {
                "religion_abroad"
            };
            changes[i] = Some(Change { order, why, net, alts: std::mem::take(&mut rel_alts[i]), detail: String::new() });
        }
    }

    // 7. Apply.
    for (i, k) in casters.iter().enumerate() {
        let Some(ch) = changes[i].take() else { continue };
        if ch.order == k.current {
            continue;
        }
        let c = k.city;
        if g.city_set_culture_channel(c, ch.order).is_err() {
            continue;
        }
        let city = g.d.text.name(&g.cities[c].name).to_string();
        match ch.order {
            Some((a, t, r)) => {
                let name = g.d.cultural_actions[a].base.type_.clone();
                if crate::trace_on(p) {
                    let what = r.map_or(String::new(), |r| format!(" ({})", g.d.religions[r].base.type_));
                    crate::trace!(g, p, "culture: {city} channels {name}{what} at p{} ({}, net {} a turn, {} culture; {})", t.0, ch.why, ch.net / 100, k.amount, ch.detail);
                }
                let alts = ch.alts;
                g.tel_decision(p, "culture", &name, ch.why, ch.net / 100, || ffh_core::telemetry::tel_alts(alts.iter().map(|(s, v)| (s.as_str(), *v))));
            }
            None => {
                crate::trace!(g, p, "culture: {city} stops its channel ({}; {})", ch.why, ch.detail);
                g.tel_decision(p, "culture", "stop", ch.why, 0, String::new);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;

    fn set_rate(g: &mut Game, c: CityId, rate: i32) {
        g.cities[c].commerce_rate[CULTURE.idx()] = 100 * rate;
    }

    /// A two-player game with cultural channels on, the Doviello (player 0) and the Elohim: our
    /// city at (4, 5), their capital at
    /// (15, 5) and their town of 4 people at (8, 5), 4 plots from ours; met, revealed. Our city's
    /// borders have grown (its culture is worth half), theirs make no culture.
    fn duel() -> (Game, CityId, CityId, CityId) {
        // The Doviello's Charadon: Aggressive, a conqueror.
        let mut g = crate::testkit::one_landmass_game_with(4, &["CIVILIZATION_DOVIELLO", "CIVILIZATION_ELOHIM"]);
        g.setup.options[GameOptionType::GAMEOPTION_CULTURE_ACTIONS] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 4, 5, true, true);
        let capital = g.player_init_city(p1, 15, 5, true, true);
        let town = g.player_init_city(p1, 8, 5, true, true);
        g.city_set_population(town, 4);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        for c in [ours, capital, town] {
            g.cities[c].revealed[t0] = true;
            g.cities[c].revealed[t1] = true;
        }
        let developing = g.d.t::<CultureLevelType>("CULTURELEVEL_DEVELOPING");
        g.cities[ours].culture_level = Some(developing);
        // As strong as they are (no units on either side).
        g.players[p0].power = g.players[p1].power;
        (g, ours, capital, town)
    }

    /// Player 0 is Friendly with player 1 (same alignment, the Trust spell, cultural friendship).
    fn befriend(g: &mut Game) {
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let neutral = g.d.t::<AlignmentType>("ALIGNMENT_NEUTRAL");
        g.players[p0].alignment = Some(neutral);
        g.players[p1].alignment = Some(neutral);
        g.players[p0].culture_memory[p1] = [0, 8];
        g.players[p1].feat_accomplished[FeatType::FEAT_TRUST] = true;
        assert_eq!(crate::diplomacy::attitude(g, p0, p1), AttitudeType::ATTITUDE_FRIENDLY);
    }

    fn revolt(g: &Game) -> CulturalActionType {
        g.d.t::<CulturalActionType>("CULTURALACTION_FOMENT_REVOLT")
    }

    /// A strong cultural city next to a weak town of a civilization we dislike channels revolt at
    /// it, and the town joins us within 30 turns; their capital never does.
    #[test]
    fn a_cultural_city_foments_revolt_until_a_weak_town_joins_us() {
        let (mut g, ours, capital, town) = duel();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        g.players[p0].culture_memory[p1] = [8, 0];
        assert!(crate::diplomacy::attitude(&g, p0, p1).i() <= AttitudeType::ATTITUDE_ANNOYED.i());
        let town_plot = g.city_plot(town);
        let mut channelled = false;
        for _ in 0..30 {
            let next = g.game_turn() + 1;
            g.set_game_turn(next);
            set_rate(&mut g, ours, 60);
            set_rate(&mut g, town, 0);
            turn_switched(&mut g, p0, true);
            channelled |= g.cities[ours].culture_channel.as_ref().is_some_and(|ch| ch.action == revolt(&g) && ch.target == p1);
            g.city_do_culture_channel(ours);
            if g.cities.contains(town) {
                g.city_do_revolt_pressure(town);
            }
            if g.map.plots[town_plot].plot_city.is_some_and(|c| g.cities[c].owner == p0) {
                break;
            }
        }
        assert!(channelled, "revolt channelled at them");
        assert!(g.map.plots[town_plot].plot_city.is_some_and(|c| g.cities[c].owner == p0), "the town joined us");
        assert_eq!(g.cities[capital].owner, p1, "a capital never flips");
    }

    /// With switch `CULTURE` off the civilization channels nothing, where on it would.
    #[test]
    fn nothing_with_the_switch_off() {
        let (mut g, ours, _, _) = duel();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        g.players[p0].culture_memory[p1] = [8, 0];
        set_rate(&mut g, ours, 60);
        let mut on = Game::load(g.d, &g.save()).unwrap();
        set_rate(&mut on, ours, 60);
        turn_switched(&mut g, p0, false);
        assert!(g.cities[ours].culture_channel.is_none());
        turn_switched(&mut on, p0, true);
        assert!(on.cities[ours].culture_channel.is_some(), "on, it channels");
    }

    /// A neighbour we are Friendly with is spared, however weak.
    #[test]
    fn a_friendly_neighbour_is_spared() {
        let (mut g, ours, _, _) = duel();
        let p0 = PlayerId(0);
        befriend(&mut g);
        set_rate(&mut g, ours, 60);
        turn_switched(&mut g, p0, true);
        assert!(!g.cities[ours].culture_channel.as_ref().is_some_and(|ch| ch.action == revolt(&g)), "no revolt against a friend");
    }

    /// A city with our state religion spreads it into our own cities lacking it first, though a
    /// friendly neighbour lacks it too.
    #[test]
    fn the_state_religion_goes_home_first() {
        let (mut g, ours, _, _) = duel();
        let p0 = PlayerId(0);
        befriend(&mut g);
        let second = g.player_init_city(p0, 4, 8, true, true);
        let third = g.player_init_city(p0, 7, 3, true, true);
        let r = g.d.religions.keys().next().unwrap();
        g.city_set_has_religion(ours, r, true, false, false);
        g.player_set_last_state_religion(p0, Some(r));
        if g.player_get_state_religion(p0) != Some(r) {
            g.players[p0].state_religion_count += 1;
        }
        assert_eq!(g.player_get_state_religion(p0), Some(r));
        set_rate(&mut g, ours, 20);
        turn_switched(&mut g, p0, true);
        let ch = g.cities[ours].culture_channel.clone().expect("a channel");
        assert_eq!(ch.action, g.d.t::<CulturalActionType>("CULTURALACTION_SPREAD_RELIGION"));
        assert_eq!((ch.target, ch.religion), (p0, Some(r)), "our state religion, at home");
        assert!(!g.cities[second].has_religion[r] && !g.cities[third].has_religion[r]);
    }

    /// A channel is the civilization's own decision: a human's governor never sets one, and an
    /// AI's governed city channels as the AI decides.
    #[test]
    fn governors_never_channel() {
        let (mut g, ours, _, town) = duel();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        g.setup.options[GameOptionType::GAMEOPTION_GOVERNORS] = true;
        g.cities[ours].direct_rule = false;
        g.players[p0].culture_memory[p1] = [8, 0];
        g.city_set_population(town, 8);
        g.players[p0].power = g.players[p1].power;
        set_rate(&mut g, ours, 60);
        assert!(g.city_is_governed(ours));
        govern(&mut g, p0);
        assert!(g.cities[ours].culture_channel.is_none(), "a human's governor sets no channel");
        turn_switched(&mut g, p0, true);
        let ch = g.cities[ours].culture_channel.clone().expect("the AI's own decision, governed or not");
        assert_eq!((ch.action, ch.target), (revolt(&g), p1));
    }

    /// Their channel would take our town: it trains defenders first, and it provokes us.
    #[test]
    fn revolt_pressure_on_our_town_asks_for_defenders_and_provokes() {
        let (mut g, ours, _, town) = duel();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        // Their town is the caster now: player 1 channels revolt at our (non-capital) city.
        let target = g.player_init_city(p0, 11, 7, true, true);
        g.cities[target].revealed[g.players[p1].team] = true;
        set_rate(&mut g, town, 60);
        set_rate(&mut g, target, 0);
        g.city_set_culture_channel(town, Some((revolt(&g), p0, None))).unwrap();
        assert_eq!(defenders_needed(&g, target), MAX_DEFENDERS, "a town without culture, 3 plots from 30 culture a turn");
        // An attacker in the making adds no lasting resistance: a defender goes before it.
        if let Some(a) = crate::production::best_unit(&g, target, crate::production::Role::Attack) {
            g.city_push_order(target, OrderType::ORDER_TRAIN, a.i(), UnitAIType::UNITAI_ATTACK.i(), false, false, false, false);
        }
        defend(&mut g, p0, false);
        let first = g.cities[target].order_queue.first().expect("an order");
        assert_eq!(first.order_type, OrderType::ORDER_TRAIN);
        assert!(g.d.units[UnitType::from_usize(first.data1 as usize)].combat > 0, "a defender first");
        assert_eq!(g.city_production_unit_ai(target), Some(UnitAIType::UNITAI_CITY_DEFENSE));
        let queued = g.cities[target].order_queue.len();
        defend(&mut g, p0, false);
        assert_eq!(g.cities[target].order_queue.len(), queued, "one defender in the making is enough a turn");
        assert!(!flips_against(&g, p0, p1));
        g.cities[target].culture_pressure[p1] = 100 * g.d.defines.culture_revolt_flip * 3 / 4;
        assert!(flips_against(&g, p0, p1), "close to taking it: a provocation");
        g.cities[ours].culture_pressure[p1] = 100 * g.d.defines.culture_revolt_flip;
        g.cities[target].culture_pressure[p1] = 0;
        assert!(!flips_against(&g, p0, p1), "never our capital");
    }
}

