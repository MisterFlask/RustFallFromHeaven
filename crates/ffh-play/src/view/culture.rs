//! The civilization's cultural channel as the human sees it (docs/CULTURE.md; docs/PERSONS.md,
//! "The fourth build"): the `culture` query and the pieces the city screen uses.
//!
//! The fourth build's part 1 left this as a plain reading of the rules' read API (the channel,
//! its preview line, the channels aimed at the human and between others, the memories); the
//! Culture screen of rule 10 is part 2's.
//!
//! Every channel is public to the civilizations that have met its caster
//! (`Game::culture_channel_visible`).

use ffh_core::culture::{Channel, ChannelTarget};
use ffh_core::ids::*;

use crate::json::Json;
use crate::view::Viewer;

/// A channel's target: `{house, house_player, label}` for a House, `{player}` for a civilization.
pub fn target_json(v: &Viewer, target: ChannelTarget) -> Json {
    let g = v.g;
    match target {
        ChannelTarget::House(h) if g.houses.contains(h) => crate::jobj! {
            "house" => h,
            "house_player" => g.houses[h].player,
            "liege" => g.houses[h].owner,
            "label" => crate::names::resolve_keys(g.d, &g.house_label(h)).into_owned(),
        },
        ChannelTarget::House(h) => crate::jobj! { "house" => h },
        ChannelTarget::Player(t) => crate::jobj! { "player" => t },
    }
}

/// One channel: who channels what into whom, since when, its name and whether it is hostile; for
/// the human's own channel and those aimed at its team (or under the debug reveal) also its
/// inflow now and its preview's line.
pub fn channel_json(v: &Viewer, ch: &Channel) -> Json {
    let g = v.g;
    let c = &ch.channel;
    let o = c.order();
    let mut j = crate::jobj! {
        "caster" => ch.caster,
        "action" => Json::idx(c.action),
        "religion" => Json::opt_idx(c.religion),
        "target" => target_json(v, c.target),
        "since_turn" => c.since_turn,
        "name" => g.culture_channel_name(&o),
        "hostile" => g.culture_is_hostile(ch.caster, &o),
    };
    let aimed = g.culture_target_player(c.target).is_some_and(|t| g.players[t].team == v.team);
    if ch.caster == v.h || v.reveal || aimed {
        let preview = g.culture_channel_preview(ch.caster, &o);
        j.set("inflow", preview.inflow);
        j.set("inflow_last", g.players[ch.caster].culture_inflow);
        j.set("line", preview.line);
    }
    j
}

/// `culture`: the channels the human sees. `ours`: its own channel (null: none); `aimed`: those
/// aimed at its team (its Houses, its civilization); `others`: those between others it sees;
/// `inflow`: what its cities would divert now (hundredths a turn); `memories`: what the other
/// leaders remember of its cultural actions (cultural hostility and friendship, and the courted
/// Houses' memory, `MEMORY_COURTED_HOUSE`).
pub fn culture(v: &Viewer) -> Json {
    let g = v.g;
    let team = v.team;
    let mut aimed = Vec::new();
    for ch in g.culture_channels() {
        if ch.caster == v.h || (!v.reveal && !g.culture_channel_visible(team, ch.caster)) {
            continue;
        }
        if g.culture_target_player(ch.channel.target).is_some_and(|t| g.players[t].team == team) {
            aimed.push(channel_json(v, &ch));
        }
    }
    let others: Vec<Json> = g.culture_channels_between_others(team).iter().filter(|ch| ch.caster != v.h).map(|ch| channel_json(v, ch)).collect();
    let ours = g.players[v.h].culture_channel.clone().map_or(Json::Null, |channel| channel_json(v, &Channel { caster: v.h, channel }));
    let courted = ffh_data::MemoryType::MEMORY_COURTED_HOUSE;
    let memories: Vec<Json> = g
        .player_ids()
        .filter(|&o| o != v.h && g.players[o].alive && g.teams[team].has_met[g.players[o].team])
        .filter_map(|o| {
            let [hostility, friendship] = g.players[o].culture_memory[v.h];
            let court = g.players[o].ai_memory_count[v.h][courted];
            (hostility > 0 || friendship > 0 || court > 0).then(|| crate::jobj! { "player" => o, "hostility" => hostility, "friendship" => friendship, "courted" => court })
        })
        .collect();
    crate::jobj! {
        "ours" => ours,
        "aimed" => Json::Arr(aimed),
        "others" => Json::Arr(others),
        "inflow" => g.player_culture_inflow_now(v.h),
        "memories" => Json::Arr(memories),
        "memory_max" => g.d.defines.culture_memory_max,
    }
}

/// City `c`'s part in its owner's channel: the culture it diverts a turn now (hundredths) under
/// the civilization's channel (0 without one).
pub fn city_diverted(v: &Viewer, c: CityId) -> i32 {
    v.g.city_culture_diverted_times100(c)
}
