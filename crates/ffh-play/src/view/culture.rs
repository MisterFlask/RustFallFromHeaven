//! Cultural actions as the human sees them (docs/CULTURE.md): the `culture` query (the Cultural
//! Pressure panel) and the pieces the city screen and the map's city list use.
//!
//! Every channel is public to the civilizations that have met its caster
//! (`Game::culture_channel_visible`), so the human sees every channel of a civilization it has
//! met, aimed at a city it has seen; a channel aimed at a city it has not seen shows without the
//! target's name.

use ffh_core::culture::{Channel, CultureTarget};
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// One channel: who casts what at whom, how far along, whether it is hostile to the target's
/// owner, and its predicted effect where the human may know it (its own channels).
pub fn channel_json(v: &Viewer, ch: &Channel) -> Json {
    let g = v.g;
    let CultureTarget::City(t) = ch.order.target;
    let team = g.players[v.h].team;
    let target_seen = g.cities.contains(t) && (g.cities[t].owner == v.h || g.city_is_revealed(t, team, false) || v.reveal);
    let hostile = g.cities.contains(t) && g.culture_is_hostile(ch.order.action, ch.caster, t, ch.order.religion);
    let mut j = crate::jobj! {
        "city" => ch.city,
        "city_name" => city_name(g, ch.city),
        "caster" => ch.caster,
        "action" => Json::idx(ch.order.action),
        "religion" => Json::opt_idx(ch.order.religion),
        "target" => target_seen.then_some(t),
        "target_name" => if target_seen { Json::from(city_name(g, t)) } else { Json::Null },
        "target_owner" => if g.cities.contains(t) { Json::from(g.cities[t].owner) } else { Json::Null },
        "progress" => ch.order.progress,
        "duration" => ch.duration,
        "turns_left" => (ch.duration - ch.order.progress).max(0),
        "paused" => ch.order.stalled > 0,
        "hostile" => hostile,
    };
    if ch.caster == v.h || v.reveal {
        if let Ok(p) = g.culture_check(ch.city, ch.order.action, ch.order.target, ch.order.religion, ffh_core::culture::CultureCheck::Channel) {
            j.set("effect", p.effectiveness);
            j.set("magnitude", p.magnitude);
        }
    }
    j
}

/// The timed effects of cultural actions on city `c` (festivals, fairs, unrest), with their
/// casters: everyone sees them on a city whose banner it sees.
pub fn marks_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    Json::Arr(
        g.cities[c]
            .culture_marks
            .iter()
            .map(|k| {
                let kind = match k.effect {
                    CultureEffect::Festival => "festival",
                    CultureEffect::TradeFair => "trade_fair",
                    CultureEffect::InciteUnrest => "unrest",
                    // (only the three above leave marks)
                    CultureEffect::SpreadReligion | CultureEffect::SiphonKnowledge | CultureEffect::FomentRevolt | CultureEffect::FlipCity => "other",
                };
                crate::jobj! { "kind" => kind, "from" => k.from, "amount" => k.amount, "turns" => k.turns }
            })
            .collect(),
    )
}

/// The number of hostile channels the human sees aimed at city `c` (its banner's pressure mark).
pub fn pressure_count(v: &Viewer, c: CityId) -> usize {
    let g = v.g;
    let team = g.players[v.h].team;
    g.culture_hostile_channels_at(c).iter().filter(|ch| v.reveal || g.culture_channel_visible(team, ch.caster)).count()
}

/// `culture`: the Cultural Pressure panel. `incoming`: the channels aimed at the human's cities;
/// `ours`: those it runs; `others`: the other channels it sees (between other civilizations);
/// and what the other leaders remember of the human's actions.
pub fn culture(v: &Viewer) -> Json {
    let g = v.g;
    if !g.culture_actions_on() {
        return crate::jobj! { "on" => false };
    }
    let team = g.players[v.h].team;
    let (mut incoming, mut ours, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for ch in g.culture_channels() {
        if !v.reveal && !g.culture_channel_visible(team, ch.caster) {
            continue;
        }
        let CultureTarget::City(t) = ch.order.target;
        let at_us = g.cities.contains(t) && g.cities[t].owner == v.h;
        let j = channel_json(v, &ch);
        if ch.caster == v.h {
            ours.push(j);
        } else if at_us {
            incoming.push(j);
        } else {
            others.push(j);
        }
    }
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
        "incoming" => Json::Arr(incoming),
        "ours" => Json::Arr(ours),
        "others" => Json::Arr(others),
        "memories" => Json::Arr(memories),
    }
}
