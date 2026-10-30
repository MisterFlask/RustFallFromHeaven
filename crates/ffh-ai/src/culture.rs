//! Cultural actions (this game's own rules, docs/CULTURE.md; docs/AI.md "Cultural actions").
//!
//! Once a turn, before the cities' other business:
//!
//! 1. **Defence.** A hostile channel aimed at one of our cities (or a pact partner's) is
//!    answered with a festival there (it resists hostile actions and adds a happy face), from the
//!    city itself or a neighbour that can reach it, unless a festival already covers the city
//!    until the channel ends; against a revolt or a flip the city also trains a defender first
//!    while it has fewer than three. A flip channel is a provocation the war logic reads
//!    (`flips_against`, `diplomacy::territory_check`).
//! 2. **Use.** Every city with an empty queue weighs the actions it can afford by what they are
//!    worth for the culture they cost (expected value × E ÷ R·D, E being in the rules' preview)
//!    and queues the best one worth at least its culture, or leaves the queue empty (the culture
//!    then grows the city's borders, as without the system). Culture counts in full while the
//!    city's borders still grow to its working radius, half after; the cities carrying a
//!    cultural victory we pursue spend none:
//!    - a **trade fair** with a civilization we are Pleased or Friendly with: the extra
//!      commerce of the routes it lifts, ours and half of theirs, and a little goodwill with a
//!      Pleased one (the fair is remembered: a step towards Friendly);
//!    - **spreading a religion**, on the missionaries' priorities (`units::missionary`): our own
//!      cities lacking our state religion first, then, once its shrine pays for every city with
//!      it (`units::abroad_religion`), foreign cities whose rules admit it, never as a hostile act
//!      (into a city whose owner has another state religion) unless we dislike them anyway;
//!    - **inciting unrest** in a city of a civilization at war with us, or one we are Annoyed or
//!      Furious with, at most `CULTURE_AI_MAX_HOSTILE_PER_TARGET` channels against one civilization
//!      at a time; weighed by the city's size and the leader's cultural aggression, and most where
//!      our culture is strong and theirs weak (a city they took from us);
//!    - **siphoning knowledge** from anyone but a Pleased or Friendly civilization (research is
//!      worth what commerce is; a minor offence);
//!    - **fomenting revolt** in a rival's city: its output for the disorder, the hurt garrison, and
//!      a third of the city's worth when one of our cities could then flip it;
//!    - **flipping** a rival's city (after a revolt): its worth times the chance; at peace only
//!      when our power is at least 80% of their defence (a flip is a provocation).
//!
//! Switch `CULTURE` (`FFH_AI_CULTURE_OFF`): off, a civilization takes no cultural actions.

use ffh_core::culture::{CultureCheck, CulturePreview, CultureTarget};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// One possible order and what it is worth.
struct Option_ {
    city: CityId,
    action: CulturalActionType,
    target: CityId,
    religion: Option<ReligionType>,
    /// Expected value in culture-equivalent points (a commerce or a culture is 1).
    value: i32,
    hostile: bool,
    why: &'static str,
}

pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.culture_actions_on() || !crate::switches::CULTURE.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    defend(g, p);
    use_culture(g, p);
}

/// The festival action, if the data has one.
fn find(g: &Game, effect: CultureEffect) -> Option<CulturalActionType> {
    g.d.cultural_actions.keys().find(|&a| g.d.cultural_actions[a].effect == effect)
}

/// Festivals against the hostile channels aimed at our cities and our partners'.
fn defend(g: &mut Game, p: PlayerId) {
    let Some(festival) = find(g, CultureEffect::Festival) else { return };
    let team = g.players[p].team;
    // Threatened cities: (city, turns until the earliest hostile channel there completes).
    let mut threatened: Vec<(CityId, i32)> = Vec::new();
    for ch in g.culture_channels() {
        let CultureTarget::City(t) = ch.order.target;
        let owner = g.cities[t].owner;
        if !g.culture_is_ally(p, owner) || !g.culture_channel_visible(team, ch.caster) {
            continue;
        }
        if !g.culture_is_hostile(ch.order.action, ch.caster, t, ch.order.religion) {
            continue;
        }
        let left = (ch.duration - ch.order.progress).max(0);
        match threatened.iter_mut().find(|(c, _)| *c == t) {
            Some(e) => e.1 = e.1.min(left),
            None => threatened.push((t, left)),
        }
    }
    threatened.sort();
    // Against a revolt or a flip, defenders too: each resists it, and a revolt hurts fewer of
    // them when there are more (expert play: festival and garrison).
    for &(t, _) in &threatened {
        let owner = g.cities[t].owner;
        if owner != p {
            continue;
        }
        let grave = g.culture_hostile_channels_at(t).iter().any(|ch| matches!(g.d.cultural_actions[ch.order.action].effect, CultureEffect::FomentRevolt | CultureEffect::FlipCity));
        if !grave || g.culture_garrison_count(t) >= 3 || g.city_production_unit(t).is_some_and(|u| g.d.units[u].combat > 0) {
            continue;
        }
        if let Some(u) = crate::production::best_unit(g, t, crate::production::Role::Defense) {
            crate::trace!(g, p, "culture: a defender first in {} against a revolt or flip", g.d.text.name(&g.cities[t].name));
            g.city_push_order(t, OrderType::ORDER_TRAIN, u.i(), UnitAIType::UNITAI_CITY_DEFENSE.i(), false, false, false, false);
            g.tel_decision(p, "culture", "garrison", "defend", 0, String::new);
        }
    }
    for (t, left) in threatened {
        // Already covered: a festival lasting past the channel's end, or one on its way.
        let covered = g.cities[t].culture_marks.iter().any(|k| k.effect == CultureEffect::Festival && k.turns > left);
        let coming = g.culture_channels().iter().any(|ch| ch.order.action == festival && ch.order.target == CultureTarget::City(t) && g.culture_is_ally(ch.caster, g.cities[t].owner));
        if covered || coming {
            continue;
        }
        // Who can hold it: the city itself (put first) or our nearest idle neighbour in reach.
        let mut casters: Vec<(i32, bool, CityId)> = Vec::new();
        for &c in &g.players[p].cities {
            if g.culture_check(c, festival, CultureTarget::City(t), None, CultureCheck::Queue).is_err() {
                continue;
            }
            let idle = g.cities[c].culture_queue.is_empty();
            if !idle && c != t {
                continue;
            }
            let dist = g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[t].x, g.cities[t].y);
            casters.push((dist, !idle, c));
        }
        casters.sort();
        if let Some(&(_, busy, c)) = casters.first() {
            if g.city_push_culture_order(c, festival, CultureTarget::City(t), None, false, busy).is_ok() {
                crate::trace!(g, p, "culture: festival in {} against a hostile channel ({left} turns left)", g.d.text.name(&g.cities[t].name));
                g.tel_decision(p, "culture", "CULTURALACTION_FESTIVAL", "defend", left as i64, String::new);
            }
        }
    }
}

/// Our hostile channels running against each civilization (by its owner).
fn hostile_count(g: &Game, p: PlayerId, owner: PlayerId) -> i32 {
    g.culture_channels()
        .iter()
        .filter(|ch| {
            let CultureTarget::City(t) = ch.order.target;
            ch.caster == p && g.cities[t].owner == owner && g.culture_is_hostile(ch.order.action, p, t, ch.order.religion)
        })
        .count() as i32
}

/// Whether we may act against `owner`'s cities: at war, or Annoyed or Furious with them.
fn rival(g: &Game, p: PlayerId, owner: PlayerId) -> bool {
    if g.player_is_barbarian(owner) {
        return g.at_war(g.players[p].team, g.players[owner].team);
    }
    g.at_war(g.players[p].team, g.players[owner].team) || crate::diplomacy::attitude(g, p, owner).i() <= AttitudeType::ATTITUDE_ANNOYED.i()
}

/// The commerce a fair lifts a turn between city `t` and our cities, in hundredths: our routes to
/// it, and half of its routes to us (a friend's gain is worth something to us).
fn fair_gain(g: &Game, p: PlayerId, t: CityId, pct: i32) -> i32 {
    let max_routes = g.d.defines.max_trade_routes.max(0) as usize;
    let mut ours = 0;
    for &c in &g.players[p].cities {
        for i in 0..max_routes {
            if g.city_trade_city(c, i) == Some(t) {
                ours += g.city_calculate_trade_profit(c, t);
            }
        }
    }
    let mut theirs = 0;
    for i in 0..max_routes {
        if let Some(o) = g.city_trade_city(t, i) {
            if g.cities[o].owner == p {
                theirs += g.city_calculate_trade_profit(t, o);
            }
        }
    }
    (2 * ours + theirs) * pct / 2
}

/// What an order is worth (culture-equivalent points), or `None` when we would not do it.
fn worth(g: &Game, p: PlayerId, effect: CultureEffect, t: CityId, religion: Option<ReligionType>, pv: &CulturePreview) -> Option<(i32, &'static str)> {
    let owner = g.cities[t].owner;
    let own_team = g.players[owner].team == g.players[p].team;
    let pop = g.cities[t].population;
    match effect {
        // Festivals are for defence only (`defend`).
        CultureEffect::Festival => None,
        CultureEffect::TradeFair => {
            if g.player_is_barbarian(owner) || crate::diplomacy::attitude(g, p, owner).i() < AttitudeType::ATTITUDE_PLEASED.i() {
                return None;
            }
            let gain = fair_gain(g, p, t, pv.magnitude) * pv.effect_turns / 100;
            if gain <= 0 {
                return None;
            }
            // The goodwill: a fair is remembered, a step towards Friendly (what a Pleased
            // neighbour's friendship brings is beyond counting here; a few points).
            let pleased = crate::diplomacy::attitude(g, p, owner) == AttitudeType::ATTITUDE_PLEASED;
            Some((gain + if pleased { 8 } else { 0 }, "fair"))
        }
        CultureEffect::SpreadReligion => {
            let r = religion?;
            if own_team {
                // Our state religion in our own city: its happiness, its buildings.
                if owner != p || g.player_get_state_religion(p) != Some(r) {
                    return None;
                }
                return Some(((30 + 5 * pop) * pv.magnitude / 100, "religion_home"));
            }
            if pv.hostile && !rival(g, p, owner) {
                return None;
            }
            if crate::units::abroad_religion(g, p) != Some(r) {
                return None;
            }
            let none = g.city_religion_count(t) == 0;
            Some(((if none { 25 } else { 12 } + 3 * pop) * pv.magnitude / 100, "religion_abroad"))
        }
        CultureEffect::InciteUnrest => {
            if own_team || !rival(g, p, owner) {
                return None;
            }
            if hostile_count(g, p, owner) >= g.d.defines.culture_ai_max_hostile_per_target {
                return None;
            }
            let war = g.at_war(g.players[p].team, g.players[owner].team);
            let aggression = g.players[p].personality_type.map_or(100, |l| g.d.leader_heads[l].cultural_aggression);
            // An unhappy face costs a citizen's work (2-3 yields) once the city is at its cap;
            // a bigger city is likelier at it.
            let mut v = pv.magnitude * pv.effect_turns * (2 + pop / 3);
            if war {
                v *= 2;
            }
            Some((v * aggression / 100, if war { "unrest_war" } else { "unrest_rival" }))
        }
        CultureEffect::SiphonKnowledge => {
            // Research is worth what commerce is; a minor offence, not against a friend.
            if !g.player_is_barbarian(owner) && crate::diplomacy::attitude(g, p, owner).i() >= AttitudeType::ATTITUDE_PLEASED.i() {
                return None;
            }
            Some((pv.magnitude, "siphon"))
        }
        CultureEffect::FomentRevolt => {
            if own_team || !rival(g, p, owner) || hostile_count(g, p, owner) >= g.d.defines.culture_ai_max_hostile_per_target {
                return None;
            }
            let war = g.at_war(g.players[p].team, g.players[owner].team);
            let aggression = g.players[p].personality_type.map_or(100, |l| g.d.leader_heads[l].cultural_aggression);
            // Disorder costs the city its whole output for the turns; the garrison is hurt; and a
            // revolt opens the way to flipping the city, when one of our cities could.
            let garrison = g.culture_garrison_count(t);
            let mut v = pv.effect_turns * (3 * pop + 4) + garrison * 6;
            if can_flip_later(g, p, t) {
                v += city_worth(g, t) / 3;
            }
            if war {
                v = v * 3 / 2;
            }
            Some((v * pv.magnitude / 100 * aggression / 100, if war { "revolt_war" } else { "revolt_rival" }))
        }
        CultureEffect::FlipCity => {
            if own_team || !rival(g, p, owner) {
                return None;
            }
            // A city taken without a war; at peace it is a provocation, so only when we could
            // hold our own in the war it may bring.
            let (team, ot) = (g.players[p].team, g.players[owner].team);
            let war = g.at_war(team, ot);
            if !war && (g.team_get_power(team, true) as i64) * 100 < g.team_get_defensive_power(ot) as i64 * 80 {
                return None;
            }
            let aggression = g.players[p].personality_type.map_or(100, |l| g.d.leader_heads[l].cultural_aggression);
            Some((city_worth(g, t) * pv.magnitude / 100 * aggression / 100, if war { "flip_war" } else { "flip_rival" }))
        }
    }
}

/// What a city is worth to whoever holds it, in culture-equivalent points (its people and its
/// buildings).
fn city_worth(g: &Game, t: CityId) -> i32 {
    80 + 30 * g.cities[t].population + 12 * g.cities[t].num_buildings
}

/// Whether one of our cities could channel Flip City on `t` once it has revolted: tier 3 there,
/// in reach, with the share it needs.
fn can_flip_later(g: &Game, p: PlayerId, t: CityId) -> bool {
    let Some(flip) = find(g, CultureEffect::FlipCity) else { return false };
    let tier = g.d.cultural_actions[flip].tier;
    if g.city_is_capital(t) || g.culture_share(t, p) < g.culture_tier_min_share(tier) {
        return false;
    }
    let level = g.culture_tier_level(tier);
    let range = g.culture_tier_range(tier);
    g.players[p].cities.iter().any(|&c| {
        level.map_or(true, |l| g.cities[c].culture_level.is_some_and(|x| x >= l))
            && g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[t].x, g.cities[t].y) <= range
    })
}

/// Whether `other` channels Flip City against one of `p`'s cities (a provocation:
/// `diplomacy::territory_check`).
pub(crate) fn flips_against(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    g.culture_channels().iter().any(|ch| {
        let CultureTarget::City(t) = ch.order.target;
        ch.caster == other && g.cities.contains(t) && g.cities[t].owner == p && g.d.cultural_actions[ch.order.action].effect == CultureEffect::FlipCity
    })
}

/// What a point of city `c`'s culture is worth to us, in percent of a commerce: all of it while
/// the city's borders still grow to its working radius (`CULTURELEVEL_FLEDGLING`), half after
/// (it then only presses on contested borders), and nothing to spare in the cities that carry a
/// cultural victory we pursue (`None`).
fn culture_weight(g: &Game, c: CityId, carriers: &[CityId]) -> Option<i32> {
    if carriers.contains(&c) {
        return None;
    }
    let fledgling = g.d.ty::<CultureLevelType>("CULTURELEVEL_FLEDGLING");
    let grown = match (g.cities[c].culture_level, fledgling) {
        (Some(l), Some(f)) => l >= f,
        _ => false,
    };
    Some(if grown { 50 } else { 100 })
}

/// Queues the best action of each city with an empty queue.
fn use_culture(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let cities = g.players[p].cities.clone();
    // A cultural victory needs `iNumCultureCities` legendary cities: our best ones keep theirs.
    let mut carriers: Vec<CityId> = Vec::new();
    if crate::victory::plan_ours(g, p).goal() == Some(crate::victory::Goal::Culture) {
        let n = g.d.victories.keys().find(|&v| g.d.victories[v].num_culture_cities > 0).map_or(3, |v| g.d.victories[v].num_culture_cities) as usize;
        let mut by_culture: Vec<(i32, CityId)> = cities.iter().map(|&c| (-g.cities[c].culture[p], c)).collect();
        by_culture.sort();
        carriers = by_culture.into_iter().take(n).map(|(_, c)| c).collect();
    }
    for c in cities {
        if !g.cities.contains(c) || !g.cities[c].culture_queue.is_empty() {
            continue;
        }
        let rate = g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE);
        if rate <= 0 {
            continue;
        }
        let Some(weight) = culture_weight(g, c, &carriers) else { continue };
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        // (score, option): the best value per culture spent.
        let mut best: Option<(i32, Option_)> = None;
        let mut alts: Vec<(String, i64)> = Vec::new();
        for action in d.cultural_actions.keys() {
            let info = &d.cultural_actions[action];
            if info.rate > rate || info.effect == CultureEffect::Festival {
                continue;
            }
            let range = g.culture_tier_range(info.tier);
            let cost = (info.rate * g.culture_action_duration(action) * weight / 100).max(1);
            let religions: Vec<Option<ReligionType>> = if info.effect == CultureEffect::SpreadReligion {
                d.religions.keys().filter(|&r| g.cities[c].has_religion[r]).map(Some).collect()
            } else {
                vec![None]
            };
            for (t, city) in g.cities.iter() {
                if g.plot_distance(cx, cy, city.x, city.y) > range {
                    continue;
                }
                for &religion in &religions {
                    let Ok(pv) = g.culture_check(c, action, CultureTarget::City(t), religion, CultureCheck::Queue) else { continue };
                    let Some((value, why)) = worth(g, p, info.effect, t, religion, &pv) else { continue };
                    // Worth its culture: the value per point of culture spent, in percent.
                    let score = value * 100 / cost;
                    if g.telemetry.on() {
                        alts.push((format!("{}@{}", info.base.type_, t.0), score as i64));
                    }
                    if score >= 100 && best.as_ref().map_or(true, |(s, _)| score > *s) {
                        best = Some((score, Option_ { city: c, action, target: t, religion, value, hostile: pv.hostile, why }));
                    }
                }
            }
        }
        let Some((_, o)) = best else { continue };
        if g.city_push_culture_order(o.city, o.action, CultureTarget::City(o.target), o.religion, false, false).is_ok() {
            let name = d.cultural_actions[o.action].base.type_.clone();
            crate::trace!(g, p, "culture: {} from {} on {} ({}, value {}{})", name, d.text.name(&g.cities[c].name), d.text.name(&g.cities[o.target].name), o.why, o.value, if o.hostile { ", hostile" } else { "" });
            let value = o.value as i64;
            g.tel_decision(p, "culture", &name, o.why, value, || ffh_core::telemetry::tel_alts(alts.iter().map(|(s, v)| (s.as_str(), *v))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::culture::CultureOrder;

    /// An enemy channelling Flip City at one of our cities is answered with a festival there and
    /// a defender first in its production (docs/CULTURE.md, phase 3).
    #[test]
    fn an_incoming_flip_gets_a_festival_and_a_defender() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_CULTURE_ACTIONS] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 4, 5, true, true);
        let theirs = g.player_init_city(p1, 9, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        // Their flip channel, started, aimed at our city.
        let flip = g.d.t::<CulturalActionType>("CULTURALACTION_FLIP_CITY");
        g.cities[theirs].culture_queue.push(CultureOrder { action: flip, target: CultureTarget::City(ours), religion: None, repeat: false, progress: 2, stalled: 0, started_turn: 1 });
        // Our city can afford a festival.
        g.cities[ours].commerce_rate[CommerceType::COMMERCE_CULTURE.idx()] = 1000;
        let festival = g.d.t::<CulturalActionType>("CULTURALACTION_FESTIVAL");
        assert!(g.culture_hostile_channels_at(ours).len() == 1);
        turn(&mut g, p0);
        let q = &g.cities[ours].culture_queue;
        assert!(q.first().is_some_and(|o| o.action == festival && o.target == CultureTarget::City(ours)), "a festival first: {q:?}");
        let first = g.cities[ours].order_queue.first().expect("an order");
        assert_eq!(first.order_type, OrderType::ORDER_TRAIN);
        assert!(g.d.units[UnitType::from_usize(first.data1 as usize)].combat > 0, "a defender first");
    }
}
