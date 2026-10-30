//! Cultural channels as the human sees them (docs/CULTURE.md): the `culture` query (the Cultural
//! Pressure panel) and the pieces the city screen and the map's city list use.
//!
//! Every channel is public to the civilizations that have met its caster
//! (`Game::culture_channel_visible`). A channel's reach lists only the cities the human has seen
//! (`unseen` counts the others).

use ffh_core::culture::{Channel, ChannelReach, REVOLT_REVOLT};
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// Whether the human sees city `t` (its own, revealed, or the debug reveal).
fn seen(v: &Viewer, t: CityId) -> bool {
    let g = v.g;
    v.reveal || g.cities[t].owner == v.h || g.city_is_revealed(t, g.players[v.h].team, false)
}

/// What city `t` resists of one caster's revolt pressure in its turn
/// (`Game::city_do_revolt_pressure`), in hundredths of culture, and what decides which part
/// applies. A city whose total pressure (every caster's) reached `CULTURE_REVOLT_DISORDER` after
/// its turn is in revolt (disorder) through its next turn and makes no culture, so it resists with
/// its defenders alone; it leaves the revolt in the turn the total falls below, and its culture
/// resists again from the turn after.
#[derive(Clone, Copy, Debug)]
struct Resistance {
    /// Out of revolt: `Game::city_revolt_resistance` now (its culture's part and its defenders').
    full: i32,
    /// In revolt: `CULTURE_REVOLT_GARRISON` per defender.
    revolt: i32,
    /// The other casters' pressure on the city now, taken as constant.
    rest: i32,
    /// In revolt (or otherwise in disorder) now. Its full resistance cannot be read then (its
    /// culture is 0), so it resists `revolt` both ways and `full` is not used.
    in_revolt: bool,
}

/// What city `t` resists a turn in revolt (hundredths): its defenders' part of
/// `Game::city_revolt_resistance`.
fn revolt_resistance(v: &Viewer, t: CityId) -> i32 {
    let g = v.g;
    100 * g.d.defines.culture_revolt_garrison * g.culture_garrison_count(t)
}

/// Whether city `t` is in revolt now, or otherwise in disorder: its culture is 0 then.
fn in_revolt(v: &Viewer, t: CityId) -> bool {
    let g = v.g;
    g.cities[t].culture_revolt_level >= REVOLT_REVOLT || g.city_is_disorder(t)
}

/// City `t`'s [`Resistance`] to `caster`'s pressure now.
fn resistance(v: &Viewer, t: CityId, caster: PlayerId) -> Resistance {
    let g = v.g;
    Resistance {
        full: g.city_revolt_resistance(t),
        revolt: revolt_resistance(v, t),
        rest: (g.city_revolt_pressure(t) - g.cities[t].culture_pressure[caster]).max(0),
        in_revolt: in_revolt(v, t),
    }
}

/// What the UI's predictions need to run the same turns (`culture.js`) for `caster`'s pressure
/// on city `t`: `resist` (now), `resist_revolt` (its defenders' part: all it resists in revolt),
/// `rest` (the other casters' pressure now), `in_revolt`.
fn set_resistance(j: &mut Json, v: &Viewer, t: CityId, caster: PlayerId) {
    let r = resistance(v, t, caster);
    j.set("resist", r.full);
    j.set("resist_revolt", r.revolt);
    j.set("rest", r.rest);
    j.set("in_revolt", r.in_revolt);
}

/// Where `caster`'s revolt pressure on city `t` settles under `inflow` a turn (hundredths), from
/// its pressure now ([`steady_pressure`]).
fn settles(v: &Viewer, t: CityId, caster: PlayerId, inflow: i32) -> i32 {
    let g = v.g;
    let defines = &g.d.defines;
    let now = g.cities[t].culture_pressure[caster];
    steady_pressure(now, inflow, &resistance(v, t, caster), defines.culture_revolt_fade_percent.max(0), 100 * defines.culture_revolt_disorder)
}

/// One turn of a caster's revolt pressure `p` (hundredths) in a city, as the rules run it: the
/// inflow in (in the channelling city's turn), then the city's turn takes out its resistance (its
/// defenders' part alone while `revolt`, or while `r.in_revolt`) and the fade (`fade` percent, at
/// least a culture). Returns the pressure after it and whether the city is in revolt in its next
/// turn: the total (with the others' `rest`) at `disorder` or more.
fn revolt_step(p: i32, inflow: i32, revolt: bool, r: &Resistance, fade: i32, disorder: i32) -> (i32, bool) {
    let q = p + inflow;
    let resist = if revolt || r.in_revolt { r.revolt } else { r.full };
    let next = if q > 0 { (q - resist - (q * fade / 100).max(100)).max(0) } else { 0 };
    (next, next + r.rest >= disorder)
}

/// Where a caster's revolt pressure, now `now`, settles under `inflow` a turn against `r` with
/// `fade` percent fading, after the city's turn: the turns ([`revolt_step`]) run until neither
/// the pressure nor the revolt moves. A city whose full resistance would hold the total at
/// `disorder` or more falls into revolt, loses its culture's part, and the pressure runs on to
/// where its defenders alone hold it (often past the flip).
fn steady_pressure(now: i32, inflow: i32, r: &Resistance, fade: i32, disorder: i32) -> i32 {
    let mut p = now.max(0);
    let mut revolt = r.in_revolt;
    for _ in 0..2000 {
        let (next, next_revolt) = revolt_step(p, inflow, revolt, r, fade, disorder);
        if next == p && next_revolt == revolt {
            break;
        }
        p = next;
        revolt = next_revolt;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::{steady_pressure, Resistance};

    /// A city resisting `full` out of revolt and `revolt` in it, with `rest` from other casters.
    fn r(full: i32, revolt: i32, rest: i32, in_revolt: bool) -> Resistance {
        Resistance { full, revolt, rest, in_revolt }
    }

    #[test]
    fn steady_pressure_is_where_the_turns_stop_moving() {
        // +5.1 a turn against 2 resisted, 5% fade: about 19 x 5.1 - 20 x 2 = 56.9.
        let p = steady_pressure(0, 510, &r(200, 200, 0, false), 5, 10000);
        assert!((p - 5690).abs() <= 20, "{p}");
        // From above it comes down to the same point.
        let p = steady_pressure(9000, 510, &r(200, 200, 0, false), 5, 10000);
        assert!((p - 5690).abs() <= 20, "{p}");
        // The city resists it all, or the minimum fade (a culture) eats the rest.
        assert_eq!(steady_pressure(0, 200, &r(300, 300, 0, false), 5, 10000), 0);
        assert_eq!(steady_pressure(0, 250, &r(200, 200, 0, false), 5, 10000), 0);
        // No fade: it stops only at the minimum fade's balance.
        assert_eq!(steady_pressure(0, 400, &r(300, 300, 0, false), 0, 10000), 0);
    }

    #[test]
    fn a_revolt_takes_the_culture_out_of_the_resistance() {
        // +18 a turn against 10 (8 of its culture, 2 of a defender): its full resistance alone
        // would hold it near 19 x 18 - 20 x 10 = 142, past the revolt at 100; in revolt only the
        // defender resists, and it runs on to 19 x 18 - 20 x 2 = 302, past the flip at 300.
        let p = steady_pressure(0, 1800, &r(1000, 200, 0, false), 5, 10000);
        assert!((p - 30200).abs() <= 20, "{p}");
        // Held below the revolt, its culture keeps resisting: 19 x 12 - 20 x 10 = 28.
        let p = steady_pressure(0, 1200, &r(1000, 200, 0, false), 5, 10000);
        assert!((p - 2800).abs() <= 20, "{p}");
        // The others' 80 bring the total to the revolt: 19 x 12 - 20 x 2 = 188.
        let p = steady_pressure(0, 1200, &r(1000, 200, 8000, false), 5, 10000);
        assert!((p - 18800).abs() <= 20, "{p}");
        // In revolt now: its defenders alone resist both ways (its full resistance, unread while
        // it makes no culture, is not used), even once the total falls below the revolt.
        let p = steady_pressure(5000, 1200, &r(1000, 200, 0, true), 5, 10000);
        assert!((p - 18800).abs() <= 20, "{p}");
        assert_eq!(steady_pressure(30000, 0, &r(1000, 200, 0, true), 5, 10000), 0);
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
/// a percent). For Foment Revolt also the city's resistance a turn ([`set_resistance`]: now, in
/// revolt, the other casters' pressure, whether it is in revolt), where this channel alone would
/// bring the caster's pressure from its pressure now (`settles`, hundredths), the caster's
/// pressure there now (`now`), what the caster's other channelling cities add a turn (`others`),
/// and whether it is a capital (never flips). For Spread Religion whether it has the religion
/// (`has`) and how many religions it has (`religions`: each divides the chance).
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
            set_resistance(&mut j, v, x.city, caster);
            j.set("settles", settles(v, x.city, caster, x.amount));
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
/// channels add a turn (`inflow`), where it would settle under that (`settles`) and the other
/// casters' pressure (`rest`); the city's resistance a turn now (`resist`) and in revolt
/// (`resist_revolt`: its defenders' part), whether it is in revolt now (`in_revolt`: its full
/// resistance cannot be read then), its defenders (each resists), its level (`culture::REVOLT_*`),
/// the unhappiness from it, whether it is a capital (never flips).
pub fn pressure_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let team = g.players[v.h].team;
    let city_team = g.players[g.cities[c].owner].team;
    let total = g.city_revolt_pressure(c);
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
            casters.push(crate::jobj! { "caster" => p, "pressure" => amount, "inflow" => inflow, "settles" => settles(v, c, p, inflow), "rest" => (total - amount).max(0) });
        } else {
            unknown += amount;
        }
    }
    crate::jobj! {
        "total" => total,
        "casters" => Json::Arr(casters),
        "unknown" => unknown,
        "resist" => g.city_revolt_resistance(c),
        "resist_revolt" => revolt_resistance(v, c),
        "in_revolt" => in_revolt(v, c),
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
