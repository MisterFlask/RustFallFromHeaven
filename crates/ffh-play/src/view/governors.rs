//! This game's governors as the human sees them (docs/GOVERNORS.md): the city screen's
//! `governance` and the Governors screen (`governors` query): who runs each city, its role and
//! governor, the empire priorities and diagnoses. (An army's requisition interrupts a governed
//! city as any city: the Armies screen, `view::armies`.)
//!
//! (Interim after governors v2, 2026-11-10: the data the rules give, without commands yet.)

use ffh_core::governors::GovDiagnosis;
use ffh_core::ids::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A governor: name, trait, the city it holds.
fn governor_json(v: &Viewer, g: GovernorId) -> Json {
    let gv = &v.g.governors[g];
    crate::jobj! {
        "id" => g,
        "name" => gv.name.as_str(),
        "trait" => gv.trait_.map(Json::idx),
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
    let suitability: Vec<Json> = g.d.city_roles.keys().map(|r| Json::from(g.city_role_suitability(c, r))).collect();
    crate::jobj! {
        "city" => c,
        "name" => city_name(g, c),
        "capital" => g.city_is_capital(c),
        "governed" => governed,
        "governor" => city.governor.filter(|&x| g.governors.contains(x)).map(|x| governor_json(v, x)),
        "role" => g.city_role(c).map(Json::idx),
        "role_overridden" => city.role_overridden,
        "proposal" => Json::idx(g.city_role_proposal(c)),
        "suitability" => Json::Arr(suitability),
        "production" => g.city_yield_rate(c, ffh_data::YieldType::YIELD_PRODUCTION),
        "population" => city.population,
        // This game's armies: the armies that requisition from the city (their names).
        "requisitioned_by" => g.players[v.h].armies.iter().filter(|&&a| g.armies.contains(a) && g.armies[a].requisition.contains(&c)).map(|&a| Json::from(g.armies[a].name.as_str())).collect::<Vec<_>>(),
    }
}

/// The city screen's `governance`: the city's rule, role and governor.
pub fn city(v: &Viewer, c: CityId) -> Json {
    if v.g.cities[c].owner != v.h {
        return Json::Null;
    }
    city_json(v, c)
}

/// A diagnosis.
fn diagnosis_json(d: &GovDiagnosis) -> Json {
    match *d {
        GovDiagnosis::RoleUnfilled(r) => crate::jobj! { "kind" => "role_unfilled", "role" => Json::idx(r) },
        GovDiagnosis::PoorFit { city, role, suitability } => crate::jobj! { "kind" => "poor_fit", "city" => city, "role" => Json::idx(role), "suitability" => suitability },
        GovDiagnosis::SlowOrder { city, turns } => crate::jobj! { "kind" => "slow_order", "city" => city, "turns" => turns },
    }
}

/// `governors`: the Governors screen. Every city of the human's, whether its civics rule them
/// directly, the priorities and the diagnoses.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let mut cities: Vec<CityId> = g.players[h].cities.clone();
    cities.sort_by_key(|&c| (!g.city_is_capital(c), c));
    crate::jobj! {
        "direct_rule" => g.player_rules_directly(h),
        "min_suitability" => g.d.defines.governor_role_min_suitability,
        "max_priorities" => g.d.defines.governor_max_priorities,
        "priorities" => Json::Arr(g.players[h].governor_priorities.iter().map(|&r| Json::idx(r)).collect()),
        "diagnoses" => Json::Arr(g.player_governor_diagnoses(h).iter().map(diagnosis_json).collect()),
        "cities" => Json::Arr(cities.iter().map(|&c| city_json(v, c)).collect()),
    }
}
