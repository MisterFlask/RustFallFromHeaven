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
/// resistance and fading, where its turn weighs it (`Game::city_do_revolt_pressure`: the inflow
/// in, then the resistance and the fade, at least a culture, out): about `(inflow - resistance)
/// / fade - inflow`, found by running the turns until it stops moving.
fn settles(v: &Viewer, t: CityId, inflow: i32) -> i32 {
    let g = v.g;
    steady_pressure(inflow, g.city_revolt_resistance(t), g.d.defines.culture_revolt_fade_percent.max(0))
}

/// Where a caster's revolt pressure settles under `inflow` a turn against `resist` (hundredths)
/// with `fade` percent fading (at least a culture), after the city's turn: the turns run until
/// the pressure stops moving.
fn steady_pressure(inflow: i32, resist: i32, fade: i32) -> i32 {
    if inflow <= resist {
        return 0;
    }
    let mut p = 0;
    for _ in 0..2000 {
        let q = p + inflow;
        let next = (q - resist - (q * fade / 100).max(100)).max(0);
        if next == p {
            break;
        }
        p = next;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::steady_pressure;

    #[test]
    fn steady_pressure_is_where_the_turns_stop_moving() {
        // +5.1 a turn against 2 resisted, 5% fade: about 19 x 5.1 - 20 x 2 = 56.9.
        let p = steady_pressure(510, 200, 5);
        assert!((p - 5690).abs() <= 20, "{p}");
        // The city resists it all, or the minimum fade (a culture) eats the rest.
        assert_eq!(steady_pressure(200, 300, 5), 0);
        assert_eq!(steady_pressure(250, 200, 5), 0);
        // No fade: it stops only at the minimum fade's balance.
        assert_eq!(steady_pressure(400, 300, 0), 0);
    }
}

/// The revolt pressure a turn (hundredths) that `caster`'s channels add to city `t`: its cities
/// channelling Foment Revolt into `t`'s owner, but city `except`.
fn revolt_inflow(v: &Viewer, caster: PlayerId, except: Option<CityId>, t: CityId) -> i32 {
    let g = v.g;
    let owner = g.cities[t].owner;
    let mut sum = 0;
    for &o in &g.players[caster].cities {
        if Some(o) == except || !g.cities.contains(o) {
            continue;
        }
        let Some(ch) = &g.cities[o].culture_channel else { continue };
        if ch.target != owner || g.d.cultural_actions[ch.action].effect != CultureEffect::FomentRevolt {
            continue;
        }
        sum += g.culture_channel_reach(o, ch.action, ch.target, ch.religion).iter().find(|x| x.city == t).map_or(0, |x| x.amount);
    }
    sum
}

/// City `c`'s reach with `action` (and `religion`; `Game::culture_channel_reach`), the cities the
/// human has seen: each with its distance, the distance effect and what the channel does there a
/// turn (`amount`: revolt pressure in hundredths of culture, or a spread's chance in hundredths of
/// a percent). For Foment Revolt also the city's resistance a turn, where this channel alone
/// would bring the caster's pressure (`settles`, hundredths), the caster's pressure there now
/// (`now`), what the caster's other channelling cities add a turn (`others`), and whether it is a
/// capital (never flips). For Spread Religion whether it has the religion (`has`) and how many
/// religions it has (`religions`: each divides the chance).
pub fn reach_json(v: &Viewer, c: CityId, action: CulturalActionType, religion: Option<ReligionType>, reach: &[ChannelReach]) -> (Json, usize) {
    let g = v.g;
    let caster = g.cities[c].owner;
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
            j.set("now", g.cities[x.city].culture_pressure[caster]);
            j.set("others", revolt_inflow(v, caster, Some(c), x.city));
            j.set("capital", g.city_is_capital(x.city));
        } else {
            j.set("has", religion.is_some_and(|r| g.cities[x.city].has_religion[r]));
            j.set("religions", g.d.religions.keys().filter(|&r| g.cities[x.city].has_religion[r]).count());
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
    let (reach_j, unseen) = reach_json(v, c, action, religion, &reach);
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
/// human's own channels, those aimed at its team (whose reach is then its own cities) and under
/// the debug reveal also what it does: the culture a turn, the reach, a spread's chance.
pub fn channel_json(v: &Viewer, ch: &Channel) -> Json {
    let g = v.g;
    let c = &ch.channel;
    let mut j = crate::jobj! {
        "city" => ch.city,
        "city_name" => city_name(g, ch.city),
        "x" => g.cities[ch.city].x,
        "y" => g.cities[ch.city].y,
        "caster" => ch.caster,
        "action" => Json::idx(c.action),
        "religion" => Json::opt_idx(c.religion),
        "target" => c.target,
        "since_turn" => c.since_turn,
        "hostile" => g.culture_is_hostile(c.action, ch.caster, c.target, c.religion),
    };
    if ch.caster == v.h || v.reveal || g.players[c.target].team == g.players[v.h].team {
        let reach = g.culture_channel_reach(ch.city, c.action, c.target, c.religion);
        let (reach_j, unseen) = reach_json(v, ch.city, c.action, c.religion, &reach);
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
/// of culture; casters the human has not met are summed as `unknown`) with what that caster's
/// channels add a turn (`inflow`) and where it would settle under that (`settles`); the city's
/// resistance a turn and its defenders (each resists), its level (`culture::REVOLT_*`), the
/// unhappiness from it, whether it is a capital (never flips).
pub fn pressure_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let team = g.players[v.h].team;
    let city_team = g.players[g.cities[c].owner].team;
    let mut casters = Vec::new();
    let mut unknown = 0;
    for p in g.player_ids() {
        let amount = g.cities[c].culture_pressure[p];
        let visible = v.reveal || g.culture_channel_visible(team, p);
        let inflow = if visible && g.players[p].team != city_team { revolt_inflow(v, p, None, c) } else { 0 };
        if amount <= 0 && inflow <= 0 {
            continue;
        }
        if visible {
            casters.push(crate::jobj! { "caster" => p, "pressure" => amount, "inflow" => inflow, "settles" => settles(v, c, inflow) });
        } else {
            unknown += amount;
        }
    }
    crate::jobj! {
        "total" => g.city_revolt_pressure(c),
        "casters" => Json::Arr(casters),
        "unknown" => unknown,
        "resist" => g.city_revolt_resistance(c),
        "defenders" => g.culture_garrison_count(c),
        "level" => g.cities[c].culture_revolt_level,
        "unhappiness" => g.city_culture_unhappiness(c),
        "capital" => g.city_is_capital(c),
    }
}

/// The revolt thresholds (hundredths of culture where they are pressure; `near_flip`: two thirds
/// of the way to the flip, where a city is told it is close), the resistance (`resist_percent` of
/// the city's culture rate, `garrison` hundredths per defender), the spread's numbers
/// (`spread_max` in hundredths of a percent), whether cities may flip at all: for the UI's scales
/// and hints.
pub fn thresholds_json(v: &Viewer) -> Json {
    let defines = &v.g.d.defines;
    let flip = 100 * defines.culture_revolt_flip;
    crate::jobj! {
        "unhappy_step" => 100 * defines.culture_revolt_unhappy_step,
        "unhappy_max" => defines.culture_revolt_unhappy_max,
        "disorder" => 100 * defines.culture_revolt_disorder,
        "near_flip" => (flip * 2 + 2) / 3,
        "flip" => flip,
        "fade_percent" => defines.culture_revolt_fade_percent,
        "resist_percent" => defines.culture_revolt_resist_percent,
        "garrison" => 100 * defines.culture_revolt_garrison,
        "channel_percent" => defines.culture_channel_percent,
        "full_range" => defines.culture_channel_full_range,
        "range" => defines.culture_channel_range,
        "spread_per_culture" => defines.culture_spread_per_culture,
        "spread_max" => defines.culture_spread_max,
        "flipping" => !v.g.is_option(GameOptionType::GAMEOPTION_NO_CITY_FLIPPING),
    }
}

/// `culture`: the Cultural Pressure panel. `ours`: the human's channels; `aimed`: the channels
/// aimed at its team; `pressure`: its cities under revolt pressure; `others`: the other channels
/// it sees; `memories`: what the other leaders remember of the human's cultural actions.
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
        "memory_max" => g.d.defines.culture_memory_max,
    }
}
