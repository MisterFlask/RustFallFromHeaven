//! This game's governors as the human sees them (docs/GOVERNORS.md): the city screen's
//! `governance` and the Governors screen (`governors` query): direct rule, each city's governor
//! and mode, the bench.

use ffh_core::army::RecruitDest;
use ffh_core::ids::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A governor: name, profile, traits, the city it holds.
fn governor_json(v: &Viewer, g: GovernorId) -> Json {
    let gv = &v.g.governors[g];
    crate::jobj! {
        "id" => g,
        "name" => gv.name.as_str(),
        "profile" => Json::idx(gv.profile),
        "traits" => Json::Arr(gv.traits.iter().map(|&t| Json::idx(t)).collect()),
        "city" => gv.city,
        "since" => gv.since_turn,
        "born" => gv.born_turn,
    }
}

/// One of the human's cities as the Governors screen and the city screen show it.
fn city_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let city = &g.cities[c];
    let governed = g.city_is_governed(c);
    let cooldown = if city.rule_changed_turn < 0 { 0 } else { (city.rule_changed_turn + g.d.defines.governor_direct_rule_swap_cooldown - g.game_turn()).max(0) };
    crate::jobj! {
        "city" => c,
        "name" => city_name(g, c),
        "capital" => g.city_is_capital(c),
        "governed" => governed,
        "cooldown" => cooldown,
        "governor" => city.governor.filter(|&x| g.governors.contains(x)).map(|x| governor_json(v, x)),
        "profile" => g.city_governor_profile(c).map(Json::idx),
        "feed_army" => city.feed_army,
        "feed_army_name" => city.feed_army.filter(|&a| g.armies.contains(a)).map(|a| g.armies[a].name.clone()),
        "hostile_culture" => city.governor_hostile_culture,
        "penalty_turns" => city.governor_penalty_turns,
        "production" => g.city_yield_rate(c, ffh_data::YieldType::YIELD_PRODUCTION),
        "population" => city.population,
    }
}

/// The armies a governed city may feed: the human's armies that take reinforcements.
fn feedable_armies(v: &Viewer) -> Json {
    let g = v.g;
    if !g.armies_on() {
        return Json::Arr(Vec::new());
    }
    Json::Arr(
        g.players[v.h]
            .armies
            .iter()
            .filter(|&&a| g.armies.contains(a) && g.recruit_dest_valid(v.h, RecruitDest::Army(a)))
            .map(|&a| {
                crate::jobj! {
                    "id" => a,
                    "name" => g.armies[a].name.as_str(),
                    "units" => g.army_units(a).len(),
                    "target_size" => g.armies[a].target_size,
                }
            })
            .collect(),
    )
}

/// The city screen's `governance` (null without governors): the city's rule and governor, the
/// direct slots, the bench with how well each candidate suits the city, the armies it may feed,
/// the directly ruled cities it may take the slot of.
pub fn city(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if !g.governors_on() || g.cities[c].owner != v.h {
        return Json::Null;
    }
    let h = v.h;
    let direct = g.player_direct_cities(h);
    let swappable: Vec<Json> = direct
        .iter()
        .filter(|&&o| o != c && !g.city_is_capital(o) && g.city_rule_change_ready(o))
        .map(|&o| crate::jobj! { "city" => o, "name" => city_name(g, o) })
        .collect();
    let bench: Vec<Json> = g
        .player_bench(h)
        .into_iter()
        .map(|b| {
            let mut j = governor_json(v, b);
            j.set("match", g.governor_match(b, c));
            j
        })
        .collect();
    let mut j = city_json(v, c);
    j.set("slots", g.governor_direct_slots());
    j.set("direct_count", direct.len());
    j.set("ready", g.city_rule_change_ready(c));
    j.set("swappable", Json::Arr(swappable));
    j.set("bench", Json::Arr(bench));
    j.set("armies", feedable_armies(v));
    j
}

/// `governors`: the Governors screen. Every city of the human's (direct ones first), the bench,
/// the direct slots, the armies cities may feed.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    if !g.governors_on() {
        return crate::jobj! { "on" => false };
    }
    let h = v.h;
    let mut cities: Vec<CityId> = g.players[h].cities.clone();
    cities.sort_by_key(|&c| (g.city_is_governed(c), !g.city_is_capital(c), c));
    crate::jobj! {
        "on" => true,
        "slots" => g.governor_direct_slots(),
        "cooldown" => g.d.defines.governor_direct_rule_swap_cooldown,
        "penalty" => g.d.defines.governor_swap_penalty,
        "penalty_turns" => g.d.defines.governor_swap_turns,
        "cities" => Json::Arr(cities.iter().map(|&c| city_json(v, c)).collect()),
        "bench" => Json::Arr(g.player_bench(h).into_iter().map(|b| governor_json(v, b)).collect()),
        "armies" => feedable_armies(v),
    }
}
