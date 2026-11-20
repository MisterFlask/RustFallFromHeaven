//! The civilization's cultural channel (docs/CULTURE.md; docs/PERSONS.md, "The fourth build", rule
//! 9; docs/AI.md "Cultural actions"). A rules change is not gated on the AI (CLAUDE.md): reasonable
//! first choices, not measured ones.
//!
//! Once a turn, after the diplomacy (so the war state is current), the AI chooses its one channel.
//! It is always the civilization's own decision: governors never set one.
//!
//! 1. **Nothing** while it pursues a cultural victory (its cities keep their culture).
//! 2. **Guard** ([`guard_choice`]) a House of its own whose strongest rival favour is past half its
//!    requirement, or would reach it within [`GUARD_HORIZON`] turns at the inflows now (without
//!    our guard): the most pressed one (the soonest, then the nearest to its requirement).
//! 3. **Court** ([`court_value`]) the House whose cities it would gain are worth most for the
//!    time: each city of the split ([`ffh_core::persons::SplitView::going`], those the hold contest
//!    keeps left out) at [`city_worth`] (the revolt AI's values), over the turns to the
//!    requirement plus five (at most [`COURT_HORIZON`] turns: the rules' own prediction,
//!    `house_courtship_at`), scaled by the leader's `iCulturalAggression` (halfway from 100%, a
//!    conqueror's a quarter higher). Spared (the revolt AI's rules, [`spared`]): a liege we are
//!    Pleased or Friendly with, vassals and masters, defensive pact partners, and at peace a liege
//!    whose defensive power is above 125% of ours (150% for a channel that stands).
//! 4. **Spread Religion** ([`religion_value`]) as the per-city channels did: the state religion (or
//!    one whose shrine we hold) into our own cities first, then to friends and neutrals; into a
//!    civilization with another state religion only where we dislike it and a shrine pays.
//!
//! Against the value stands the culture spent (half our cities' culture, worth all of a commerce
//! while a city's borders still grow, half after: [`cost`]): a new channel must beat it by a
//! quarter of it. **Stability**: a standing channel is kept while it is worth [`KEEP_PCT`] of the
//! best and of its cost; a new one must beat the standing one by a quarter; a channel stands
//! [`MIN_TURNS`] turns at least (a guard of a pressed House aside), and a guard is kept while its
//! House is still pressed (a quarter of the requirement, or 40 turns).
//!
//! The AI gives no gifts to court (left out). A House accepts any gift (`diplomacy::judge_offer`).

use ffh_core::culture::{ChannelOrder, ChannelReach, ChannelTarget};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// A House of ours is guarded when a rival's favour would reach its requirement within this many
/// turns at the inflows now.
pub const GUARD_HORIZON: i32 = 20;
/// A House is courted only when its split would come within this many turns.
pub const COURT_HORIZON: i32 = 60;
/// A standing channel is kept while it is worth this share (percent) of the best and of its cost.
const KEEP_PCT: i64 = 70;
/// A channel stands at least this many turns while the rules allow it (but for a guard).
const MIN_TURNS: i32 = 5;

/// The AI's cultural channel, once a turn.
pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.player_has_houses(p) || g.players[p].cities.is_empty() {
        return;
    }
    let choice = choose(g, p);
    apply(g, p, choice);
}

/// Sets `choice` as `p`'s channel, where it changes (telemetry and the trace say why).
fn apply(g: &mut Game, p: PlayerId, choice: Option<Choice>) {
    let current = g.players[p].culture_channel.as_ref().map(|ch| ch.order());
    let order = choice.as_ref().map(|c| c.order);
    if order == current {
        return;
    }
    if g.player_set_culture_channel(p, order).is_err() {
        return;
    }
    match choice {
        Some(c) => {
            let name = g.d.cultural_actions[c.order.action].base.type_.clone();
            if crate::trace_on(p) {
                crate::trace!(g, p, "culture: channel {} ({}, net {} a turn)", g.culture_channel_name(&c.order), c.why, c.net / 100);
            }
            let alts = c.alts;
            g.tel_decision(p, "culture", &name, c.why, c.net / 100, || ffh_core::telemetry::tel_alts(alts.iter().map(|(s, v)| (s.as_str(), *v))));
        }
        None => {
            crate::trace!(g, p, "culture: the channel stops");
            g.tel_decision(p, "culture", "stop", "worthless", 0, String::new);
        }
    }
}

/// A channel chosen ([`choose`]).
#[derive(Clone, Debug)]
pub struct Choice {
    pub order: ChannelOrder,
    pub why: &'static str,
    /// Value less cost, hundredths of a commerce a turn.
    pub net: i64,
    /// The other options weighed (telemetry), in commerce a turn.
    pub alts: Vec<(String, i64)>,
}

/// What the planning knows of player `p` this turn.
struct Ctx {
    p: PlayerId,
    team: TeamId,
    court: Option<CulturalActionType>,
    guard: Option<CulturalActionType>,
    spread: Option<CulturalActionType>,
    /// Percent the value of a city taken counts: halfway from 100 to the leader's
    /// `iCulturalAggression`, a conqueror's a quarter more.
    aggression: i64,
    /// It pursues a cultural victory.
    culture_victory: bool,
    state: Option<ReligionType>,
    /// The inflow now, hundredths of culture a turn.
    inflow: i32,
}

impl Ctx {
    fn new(g: &Game, p: PlayerId) -> Ctx {
        let team = g.players[p].team;
        let plan = crate::victory::plan(g, p);
        // Half the leader's lean either way (40-180 gives 70-140%): a peaceful leader still takes
        // a House that clearly pays.
        let leader = (100 + g.players[p].personality_type.map_or(100, |l| g.d.leader_heads[l].cultural_aggression) as i64) / 2;
        let aggression = if crate::campaign::conqueror(g, p) { leader * 5 / 4 } else { leader };
        Ctx {
            p,
            team,
            court: g.culture_action(CultureEffect::Court),
            guard: g.culture_action(CultureEffect::Guard),
            spread: g.culture_action(CultureEffect::SpreadReligion),
            aggression,
            culture_victory: plan.goal() == Some(crate::victory::Goal::Culture),
            state: g.player_get_state_religion(p),
            inflow: g.player_culture_inflow_now(p),
        }
    }
}

/// What the channel's culture costs us a turn, in hundredths of a commerce: each city's share at
/// all of a commerce a point while its borders still grow to its working radius
/// (`CULTURELEVEL_FLEDGLING`), half after.
pub fn cost(g: &Game, p: PlayerId) -> i64 {
    let fledgling = g.d.ty::<CultureLevelType>("CULTURELEVEL_FLEDGLING");
    let mut out = 0i64;
    for &c in &g.players[p].cities {
        if !g.cities.contains(c) || g.city_is_disorder(c) {
            continue;
        }
        let share = g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE).max(0) as i64 * g.d.defines.culture_channel_percent as i64 / 100;
        let grown = matches!((g.cities[c].culture_level, fledgling), (Some(l), Some(f)) if l >= f);
        out += share * if grown { 50 } else { 100 } / 100;
    }
    out
}

/// What a new channel must be worth beyond its cost (hundredths a turn): a quarter of it, at least
/// a commerce.
fn entry(cost: i64) -> i64 {
    (cost / 4).max(100)
}

// ----------------------------------------------------------------------------
// Guard
// ----------------------------------------------------------------------------

/// How pressed House `h` of ours is by its strongest courtship, without our guard: `Some((turns,
/// 1000 - permille of the requirement))` (smaller is more pressed) when some rival's favour is past
/// half its requirement or would reach it within [`GUARD_HORIZON`] turns at the inflows now.
fn pressed(g: &Game, h: HouseId) -> Option<(i32, i64)> {
    pressed_by(g, h, 2, GUARD_HORIZON)
}

/// Whether a standing guard of House `h` is kept: some rival favour is past a quarter of its
/// requirement, or would reach it within twice [`GUARD_HORIZON`] turns without our guard (a
/// guard that stopped as soon as the favour fell below half would start again the turn after).
fn still_pressed(g: &Game, h: HouseId) -> bool {
    pressed_by(g, h, 4, 2 * GUARD_HORIZON).is_some()
}

/// [`pressed`] with the share of the requirement (`1 / part`) and the horizon.
fn pressed_by(g: &Game, h: HouseId, part: i64, horizon: i32) -> Option<(i32, i64)> {
    let mut best: Option<(i32, i64)> = None;
    for cv in g.house_courtships(h) {
        let need = 100 * cv.requirement.total as i64;
        let turns = g.favour_turns(cv.favour, cv.gain, 0, cv.requirement.total);
        let past = part * cv.favour as i64 > need;
        let soon = turns.is_some_and(|t| t <= horizon);
        if !(past || soon) {
            continue;
        }
        let key = (turns.unwrap_or(i32::MAX), 1000 - cv.favour as i64 * 1000 / need.max(1));
        if best.is_none_or(|b| key < b) {
            best = Some(key);
        }
    }
    best
}

/// The House of ours to guard, if any: see the module comment. A standing guard is kept while its
/// House is still pressed ([`still_pressed`]: a quarter of the requirement, or 40 turns).
pub fn guard_choice(g: &Game, p: PlayerId) -> Option<HouseId> {
    let mut houses: Vec<HouseId> = g.players[p].houses.iter().copied().filter(|&h| g.houses.contains(h)).collect();
    houses.sort();
    if let Some(ChannelTarget::House(h)) = g.players[p].culture_channel.as_ref().filter(|ch| g.culture_effect(ch.action) == CultureEffect::Guard).map(|ch| ch.target) {
        if houses.contains(&h) && still_pressed(g, h) {
            return Some(h);
        }
    }
    houses.into_iter().filter_map(|h| pressed(g, h).map(|k| (k, h))).min().map(|(_, h)| h)
}

// ----------------------------------------------------------------------------
// Court
// ----------------------------------------------------------------------------

/// What a city is worth to whoever holds it, in commerce-equivalent points: its people, its
/// buildings, its wonders and holy cities (the revolt AI's values).
pub fn city_worth(g: &Game, t: CityId) -> i64 {
    let city = &g.cities[t];
    let holy = g.d.religions.keys().filter(|&r| g.get_holy_city(r) == Some(t)).count() as i64;
    150 + 50 * city.population as i64 + 20 * city.num_buildings as i64 + 150 * city.num_world_wonders as i64 + 200 * holy
}

/// Why `p` does not court a House of `liege` (`Some`: spared): the revolt AI's rules. `keeping`:
/// for a channel that stands, which a power swing does not end at once (a stronger liege is
/// spared above 125% of our power for a new channel, 150% for a standing one).
pub fn spared(g: &Game, p: PlayerId, liege: PlayerId, keeping: bool) -> Option<&'static str> {
    let (team, lt) = (g.players[p].team, g.players[liege].team);
    if lt == team {
        return Some("own_team");
    }
    if g.teams[team].vassal[lt] || g.teams[lt].vassal[team] {
        return Some("vassal");
    }
    if g.teams[team].defensive_pact[lt] {
        return Some("pact");
    }
    if g.at_war(team, lt) {
        return None;
    }
    if crate::diplomacy::attitude(g, p, liege).i() >= AttitudeType::ATTITUDE_PLEASED.i() {
        return Some("friend");
    }
    let power = g.team_get_power(team, true).max(1) as i64;
    if g.team_get_defensive_power(lt) as i64 * 100 > power * if keeping { 150 } else { 125 } {
        return Some("stronger");
    }
    None
}

/// What courting House `h` is worth to `p` a turn (hundredths of a commerce, before its cost), at
/// the inflow now: the cities its split would take over the turns to the requirement plus five,
/// scaled by the leader's aggression; `None` when the split would not come within
/// [`COURT_HORIZON`] turns or would take nothing.
fn court_value(g: &Game, ctx: &Ctx, h: HouseId) -> Option<i64> {
    g.culture_house_reach(ctx.p, h)?;
    let cv = g.house_courtship_at(h, ctx.p, ctx.inflow);
    let turns = cv.turns.filter(|&t| t <= COURT_HORIZON)?;
    let going = cv.split.map(|s| s.going).unwrap_or_default();
    if going.is_empty() {
        return None;
    }
    let worth: i64 = going.iter().map(|&c| city_worth(g, c)).sum();
    Some(100 * worth / (turns + 5) as i64 * ctx.aggression / 100)
}

// ----------------------------------------------------------------------------
// Spread Religion
// ----------------------------------------------------------------------------

/// The chance a turn of our spreading religion `r` into civilization `t`, times what a spread is
/// worth ([`spread_value`]), over the cities we know of (hundredths of a commerce a turn).
fn religion_value(g: &Game, ctx: &Ctx, r: ReligionType, t: PlayerId) -> i64 {
    let reach: Vec<ChannelReach> = g.culture_spread_reach(ctx.p, t, r, ctx.inflow).into_iter().filter(|x| x.amount > 0 && g.cities[x.city].revealed[ctx.team]).collect();
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

/// The religions we may spread: our state religion, and any whose shrine we hold, that one of our
/// cities has.
fn religions(g: &Game, ctx: &Ctx) -> Vec<ReligionType> {
    g.d.religions.keys().filter(|&r| (ctx.state == Some(r) || g.team_has_shrine(ctx.team, Some(r))) && g.players[ctx.p].cities.iter().any(|&c| g.cities[c].has_religion[r])).collect()
}

/// The civilizations we spread religion `r` into: ours and our team's, and those at peace with us
/// that we have met (a hostile spread only into one we dislike).
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
            let o = ChannelOrder { action: spread, target: ChannelTarget::Player(t), religion: Some(r) };
            if g.culture_is_hostile(ctx.p, &o) && crate::diplomacy::attitude(g, ctx.p, t).i() > AttitudeType::ATTITUDE_ANNOYED.i() {
                continue;
            }
        }
        out.push(t);
    }
    out
}

// ----------------------------------------------------------------------------
// The choice
// ----------------------------------------------------------------------------

/// What order `o` is worth to `p` a turn (hundredths of a commerce, before its cost), if it may be
/// set now (`keeping`: it stands, see [`spared`]); `None` when it may not or brings nothing.
fn value_of(g: &Game, ctx: &Ctx, o: &ChannelOrder, keeping: bool) -> Option<(i64, &'static str)> {
    let standing = g.players[ctx.p].culture_channel.as_ref().is_some_and(|ch| ch.order() == *o);
    if !standing && g.culture_channel_check(ctx.p, o).is_err() {
        return None;
    }
    match (g.culture_effect(o.action), o.target) {
        (CultureEffect::Court, ChannelTarget::House(h)) => {
            let liege = g.houses.get(h)?.owner;
            if spared(g, ctx.p, liege, keeping).is_some() {
                return None;
            }
            court_value(g, ctx, h).map(|v| (v, "court"))
        }
        (CultureEffect::SpreadReligion, ChannelTarget::Player(t)) => {
            let r = o.religion?;
            let v = religion_value(g, ctx, r, t);
            let why = if g.players[t].team == ctx.team {
                "religion_home"
            } else if g.culture_is_hostile(ctx.p, o) {
                "religion_hostile"
            } else {
                "religion_abroad"
            };
            (v > 0).then_some((v, why))
        }
        _ => None,
    }
}

/// The channel `p` sets this turn (`None`: none): see the module comment.
pub fn choose(g: &Game, p: PlayerId) -> Option<Choice> {
    choose_in(g, &Ctx::new(g, p))
}

/// [`choose`] with what the planning knows.
fn choose_in(g: &Game, ctx: &Ctx) -> Option<Choice> {
    let p = ctx.p;
    if ctx.culture_victory {
        return None;
    }
    // A channel set less than MIN_TURNS ago stands while the rules allow it (a guard of a pressed
    // House aside): a courter that left a guarded House and a liege that stopped its guard as soon
    // as the courter left would otherwise chase each other every turn.
    let young = g.players[p].culture_channel.as_ref().map(|ch| (ch.order(), g.game_turn() - ch.since_turn)).filter(|&(o, age)| age < MIN_TURNS && g.culture_channel_check(p, &o).is_ok()).map(|(o, _)| o);
    let keep = |o: ChannelOrder| Some(Choice { order: o, why: "standing", net: 0, alts: Vec::new() });
    // Guard first.
    if let (Some(guard), Some(h)) = (ctx.guard, guard_choice(g, p)) {
        let order = ChannelOrder { action: guard, target: ChannelTarget::House(h), religion: None };
        if let Some(o) = young.filter(|o| g.culture_effect(o.action) == CultureEffect::Guard) {
            return keep(o);
        }
        if g.culture_channel_check(p, &order).is_ok() {
            return Some(Choice { order, why: "guard", net: 0, alts: Vec::new() });
        }
    }
    if let Some(o) = young {
        return keep(o);
    }
    let cost = cost(g, p);
    // Every option: the Houses we may court, the religions we may spread.
    let mut options: Vec<(ChannelOrder, i64, &'static str)> = Vec::new();
    if let Some(court) = ctx.court {
        let mut houses: Vec<HouseId> = g.houses.iter().map(|(h, _)| h).collect();
        houses.sort();
        for h in houses {
            let o = ChannelOrder { action: court, target: ChannelTarget::House(h), religion: None };
            if let Some((v, why)) = value_of(g, ctx, &o, false) {
                options.push((o, v, why));
            }
        }
    }
    if let Some(spread) = ctx.spread {
        for r in religions(g, ctx) {
            for t in religion_targets(g, ctx, r) {
                let o = ChannelOrder { action: spread, target: ChannelTarget::Player(t), religion: Some(r) };
                if let Some((v, why)) = value_of(g, ctx, &o, false) {
                    options.push((o, v, why));
                }
            }
        }
    }
    let alts: Vec<(String, i64)> = if g.telemetry.on() { options.iter().map(|(o, v, _)| (format!("{}@{:?}", g.d.cultural_actions[o.action].base.type_, o.target), (v - cost) / 100)).collect() } else { Vec::new() };
    // The best (ties: the first found, Houses before religions, in id order).
    let best = options.iter().fold(None::<&(ChannelOrder, i64, &'static str)>, |b, x| if b.is_none_or(|b| x.1 > b.1) { Some(x) } else { b });
    // The standing channel, as it is worth now (a power swing does not end it at once).
    let standing = g.players[p].culture_channel.as_ref().map(|ch| ch.order()).filter(|o| g.culture_effect(o.action) != CultureEffect::Guard);
    let standing_value = standing.and_then(|o| value_of(g, ctx, &o, true).map(|(v, why)| (o, v, why)));
    if let Some((o, v, why)) = standing_value {
        let best_v = best.map_or(0, |b| b.1);
        let worth_best = v * 100 >= KEEP_PCT * best_v || best_v * 100 <= v * 125;
        if worth_best && v * 100 >= KEEP_PCT * cost {
            return Some(Choice { order: o, why, net: v - cost, alts });
        }
    }
    let (o, v, why) = *best?;
    (v - cost > entry(cost)).then_some(Choice { order: o, why, net: v - cost, alts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::persons::{Favour, LoyaltyCause, LoyaltyMod};

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);
    const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;

    fn set_rate(g: &mut Game, c: CityId, rate: i32) {
        g.cities[c].commerce_rate[CULTURE.idx()] = 100 * rate;
    }

    /// A two-player game, the Bannor (player 0) and the Doviello (player 1, the Aggressive
    /// Charadon, a conqueror): player 0's capital at (3, 4) and a House of player 0 at (9, 4) with
    /// a second city at (6, 7); player 1's capital at (15, 5), 6 plots from the House's seat. Met,
    /// revealed, as strong as each other; player 0 under Aristocracy. Returns the House.
    fn duel() -> (Game, HouseId, CityId) {
        let mut g = crate::testkit::one_landmass_game_with(4, &["CIVILIZATION_BANNOR", "CIVILIZATION_DOVIELLO"]);
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_meet(t0, t1, false);
        g.player_init_city(P0, 3, 4, true, true);
        let seat = g.player_init_city(P0, 9, 4, true, true);
        let second = g.player_init_city(P0, 6, 7, true, true);
        let theirs = g.player_init_city(P1, 15, 5, true, true);
        let civic = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
        let option = g.d.civics[civic].civic_option_type.expect("a civic option");
        g.player_set_civics(P0, option, Some(civic));
        let civ = g.player_get_civilization_type(P0).unwrap();
        let leader = g.house_leader_draw(civ, P0).expect("a free leader");
        let (h, now) = g.player_found_house(P0, leader, &[seat, second]).expect("founded");
        let t = g.game_turn();
        let house = &mut g.houses[h];
        house.demand = None;
        house.demand_rest_until = t + 1000;
        house.standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -1, until: Some(t + 1000) }];
        house.tribute = 0;
        house.tribute_now = 0;
        let ht = g.players[g.houses[h].player].team;
        g.team_meet(t1, ht, false);
        g.players[P0].power = g.players[P1].power;
        g.players[g.houses[h].player].power = 0;
        let _ = now;
        (g, h, theirs)
    }

    /// Player 1 makes 40 culture a turn in its capital; player 0 and the House make 2 a city.
    fn cultures(g: &mut Game, h: HouseId, theirs: CityId) {
        set_rate(g, theirs, 40);
        for c in g.players[P0].cities.clone().into_iter().chain(g.house_cities(h)) {
            set_rate(g, c, 2);
        }
    }

    /// A rival that can split a House within the horizon courts it (its cities are worth the
    /// culture): the Doviello court the Bannor's House.
    #[test]
    fn the_ai_courts_a_house_when_it_pays() {
        let (mut g, h, theirs) = duel();
        cultures(&mut g, h, theirs);
        let c = choose(&g, P1).expect("a channel");
        assert_eq!(g.culture_effect(c.order.action), CultureEffect::Court);
        assert_eq!(c.order.target, ChannelTarget::House(h));
        turn(&mut g, P1);
        assert!(g.players[P1].culture_channel.as_ref().is_some_and(|ch| ch.target == ChannelTarget::House(h)));
    }

    /// A liege the AI is Friendly with is spared: no Court at its House.
    #[test]
    fn a_friendly_liege_is_spared() {
        let (mut g, h, theirs) = duel();
        cultures(&mut g, h, theirs);
        g.players[P0].feat_accomplished[FeatType::FEAT_TRUST] = true;
        g.players[P1].culture_memory[P0] = [0, 8];
        let neutral = g.d.t::<AlignmentType>("ALIGNMENT_NEUTRAL");
        g.players[P0].alignment = Some(neutral);
        g.players[P1].alignment = Some(neutral);
        assert!(crate::diplomacy::attitude(&g, P1, P0).i() >= AttitudeType::ATTITUDE_PLEASED.i());
        assert_eq!(spared(&g, P1, P0, false), Some("friend"));
        assert!(choose(&g, P1).is_none_or(|c| g.culture_effect(c.order.action) != CultureEffect::Court), "no Court of a friend's House");
    }

    /// A House of ours whose rival favour is past half its requirement is guarded.
    #[test]
    fn the_ai_guards_a_pressed_house() {
        let (mut g, h, theirs) = duel();
        cultures(&mut g, h, theirs);
        assert_eq!(guard_choice(&g, P0), None, "no one courts it");
        let need = 100 * g.house_requirement(h, P1).total;
        g.houses[h].favour = vec![Favour { courter: P1, amount: need * 2 / 3, warned: false }];
        assert_eq!(guard_choice(&g, P0), Some(h));
        let c = choose(&g, P0).expect("a channel");
        assert_eq!((g.culture_effect(c.order.action), c.order.target), (CultureEffect::Guard, ChannelTarget::House(h)));
        turn(&mut g, P0);
        assert!(g.house_guard_view(h).is_some(), "the guard runs");
    }

    /// A courtship building fast (it would split within 20 turns at the inflow now) is guarded
    /// against too, before it is past half.
    #[test]
    fn the_ai_guards_against_a_fast_courtship() {
        let (mut g, h, theirs) = duel();
        cultures(&mut g, h, theirs);
        set_rate(&mut g, theirs, 400);
        let court = g.culture_action(CultureEffect::Court).unwrap();
        g.player_set_culture_channel(P1, Some(ChannelOrder { action: court, target: ChannelTarget::House(h), religion: None })).expect("courted");
        assert_eq!(guard_choice(&g, P0), Some(h));
    }

    /// Nothing while the AI pursues a cultural victory: a standing channel stops.
    #[test]
    fn a_cultural_victory_keeps_its_culture() {
        let (mut g, h, theirs) = duel();
        cultures(&mut g, h, theirs);
        turn(&mut g, P1);
        assert!(g.players[P1].culture_channel.is_some());
        let mut ctx = Ctx::new(&g, P1);
        ctx.culture_victory = true;
        let choice = choose_in(&g, &ctx);
        assert!(choice.is_none());
        apply(&mut g, P1, choice);
        assert!(g.players[P1].culture_channel.is_none(), "it stopped");
    }
}
