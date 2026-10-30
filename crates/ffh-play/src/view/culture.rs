//! Cultural channels as the human sees them (docs/CULTURE.md): the `culture` query (the Cultural
//! Pressure panel) and the pieces the city screen and the map's city list use.
//!
//! Every channel is public to the civilizations that have met its caster
//! (`Game::culture_channel_visible`). A channel's reach lists only the cities the human has seen
//! (`unseen` counts the others).

use ffh_core::culture::{Channel, ChannelReach};
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// Whether the human sees city `t` (its own, revealed, or the debug reveal).
fn seen(v: &Viewer, t: CityId) -> bool {
    let g = v.g;
    v.reveal || g.cities[t].owner == v.h || g.city_is_revealed(t, g.players[v.h].team, false)
}

/// Revolt pressure a city would settle at under `inflow` a turn (hundredths) against its
/// resistance and fading: `(inflow - resistance) / fade`.
fn settles(v: &Viewer, t: CityId, inflow: i32) -> i32 {
    let g = v.g;
    let fade = g.d.defines.culture_revolt_fade_percent.max(1);
    ((inflow - g.city_revolt_resistance(t)).max(0) * 100 / fade).max(0)
}

/// A channel's reach (`Game::culture_channel_reach`), the cities the human has seen: each with
/// its distance, the distance effect and what the channel does there a turn (`amount`: revolt
/// pressure in hundredths of culture, or a spread's chance in hundredths of a percent); for
/// Foment Revolt also the city's resistance a turn and where this channel alone would bring its
/// pressure (`settles`, hundredths).
pub fn reach_json(v: &Viewer, action: CulturalActionType, reach: &[ChannelReach]) -> (Json, usize) {
    let g = v.g;
    let revolt = g.d.cultural_actions[action].effect == CultureEffect::FomentRevolt;
    let mut unseen = 0;
    let mut out = Vec::new();
    for x in reach {
        if !seen(v, x.city) {
            unseen += 1;
            continue;
        }
        let mut j = crate::jobj! {
            "city" => x.city,
            "name" => city_name(g, x.city),
            "owner" => g.cities[x.city].owner,
            "distance" => x.distance,
            "effect" => x.effect,
            "amount" => x.amount,
        };
        if revolt {
            j.set("resist", g.city_revolt_resistance(x.city));
            j.set("settles", settles(v, x.city, x.amount));
            j.set("capital", g.city_is_capital(x.city));
        }
        out.push(j);
    }
    (Json::Arr(out), unseen)
}

/// What city `c` would do channelling `action` (with `religion`) into `target`: whether it may
/// (`ok`, else `why`), whether it is hostile, its reach and, for a spread, the chance a turn.
pub fn option_json(v: &Viewer, c: CityId, action: CulturalActionType, target: PlayerId, religion: Option<ReligionType>) -> Json {
    let g = v.g;
    let caster = g.cities[c].owner;
    let reach = g.culture_channel_reach(c, action, target, religion);
    let (reach_j, unseen) = reach_json(v, action, &reach);
    let mut j = crate::jobj! {
        "action" => Json::idx(action),
        "target" => target,
        "religion" => Json::opt_idx(religion),
        "hostile" => g.culture_is_hostile(action, caster, target, religion),
        "reach" => reach_j,
        "unseen" => unseen,
    };
    match g.culture_channel_check(c, action, target, religion) {
        Ok(()) => {
            j.set("ok", true);
        }
        Err(e) => {
            j.set("ok", false);
            j.set("why", e.describe());
        }
    }
    if g.d.cultural_actions[action].effect == CultureEffect::SpreadReligion {
        j.set("chance", g.culture_spread_chance(&reach));
    }
    j
}

/// One channel: which city channels what into whom, whether it is hostile, since when; for the
/// human's own channels (or the debug reveal) also what it does: the culture a turn, the reach,
/// a spread's chance.
pub fn channel_json(v: &Viewer, ch: &Channel) -> Json {
    let g = v.g;
    let c = &ch.channel;
    let mut j = crate::jobj! {
        "city" => ch.city,
        "city_name" => city_name(g, ch.city),
        "caster" => ch.caster,
        "action" => Json::idx(c.action),
        "religion" => Json::opt_idx(c.religion),
        "target" => c.target,
        "since_turn" => c.since_turn,
        "hostile" => g.culture_is_hostile(c.action, ch.caster, c.target, c.religion),
    };
    if ch.caster == v.h || v.reveal {
        let reach = g.culture_channel_reach(ch.city, c.action, c.target, c.religion);
        let (reach_j, unseen) = reach_json(v, c.action, &reach);
        j.set("amount", g.city_channel_amount(ch.city));
        j.set("reach", reach_j);
        j.set("unseen", unseen);
        if g.d.cultural_actions[c.action].effect == CultureEffect::SpreadReligion {
            j.set("chance", g.culture_spread_chance(&reach));
        }
    }
    j
}

/// The revolt pressure on city `c` as the human sees it: the total and each caster's (hundredths
/// of culture; casters the human has not met are summed as `unknown`), the city's resistance a
/// turn, its level (`culture::REVOLT_*`) and unhappiness from it.
pub fn pressure_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let team = g.players[v.h].team;
    let mut casters = Vec::new();
    let mut unknown = 0;
    for p in g.player_ids() {
        let amount = g.cities[c].culture_pressure[p];
        if amount <= 0 {
            continue;
        }
        if v.reveal || g.culture_channel_visible(team, p) {
            casters.push(crate::jobj! { "caster" => p, "pressure" => amount });
        } else {
            unknown += amount;
        }
    }
    crate::jobj! {
        "total" => g.city_revolt_pressure(c),
        "casters" => Json::Arr(casters),
        "unknown" => unknown,
        "resist" => g.city_revolt_resistance(c),
        "level" => g.cities[c].culture_revolt_level,
        "unhappiness" => g.city_culture_unhappiness(c),
    }
}

/// The revolt thresholds (hundredths of culture where they are pressure): for the UI's scales.
pub fn thresholds_json(v: &Viewer) -> Json {
    let defines = &v.g.d.defines;
    crate::jobj! {
        "unhappy_step" => 100 * defines.culture_revolt_unhappy_step,
        "unhappy_max" => defines.culture_revolt_unhappy_max,
        "disorder" => 100 * defines.culture_revolt_disorder,
        "flip" => 100 * defines.culture_revolt_flip,
        "fade_percent" => defines.culture_revolt_fade_percent,
        "channel_percent" => defines.culture_channel_percent,
        "full_range" => defines.culture_channel_full_range,
        "range" => defines.culture_channel_range,
    }
}

/// `culture`: the Cultural Pressure panel. `ours`: the human's channels; `incoming`: revolt
/// pressure on its cities and the channels aimed at it; `others`: the other channels it sees;
/// and what the other leaders remember of the human's actions.
pub fn culture(v: &Viewer) -> Json {
    let g = v.g;
    if !g.culture_actions_on() {
        return crate::jobj! { "on" => false };
    }
    let team = g.players[v.h].team;
    let (mut aimed, mut ours, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for ch in g.culture_channels() {
        if !v.reveal && !g.culture_channel_visible(team, ch.caster) {
            continue;
        }
        let j = channel_json(v, &ch);
        if ch.caster == v.h {
            ours.push(j);
        } else if g.players[ch.channel.target].team == team {
            aimed.push(j);
        } else {
            others.push(j);
        }
    }
    let pressure: Vec<Json> = g.players[v.h]
        .cities
        .iter()
        .filter(|&&c| g.city_revolt_pressure(c) > 0 || g.cities[c].culture_revolt_level > 0)
        .map(|&c| {
            let mut j = pressure_json(v, c);
            j.set("city", c);
            j.set("name", city_name(g, c));
            j
        })
        .collect();
    // What met leaders remember of our cultural actions (their attitude reads it).
    let memories: Vec<Json> = g
        .player_ids()
        .filter(|&o| o != v.h && g.players[o].alive && g.teams[team].has_met[g.players[o].team])
        .filter_map(|o| {
            let [hostility, friendship] = g.players[o].culture_memory[v.h];
            (hostility > 0 || friendship > 0).then(|| crate::jobj! { "player" => o, "hostility" => hostility, "friendship" => friendship })
        })
        .collect();
    crate::jobj! {
        "on" => true,
        "ours" => Json::Arr(ours),
        "aimed" => Json::Arr(aimed),
        "pressure" => Json::Arr(pressure),
        "others" => Json::Arr(others),
        "memories" => Json::Arr(memories),
        "thresholds" => thresholds_json(v),
    }
}
