//! The civilization's cultural channel as the human sees it: the Culture screen (the `culture`
//! query) and the city screen's Culture line (docs/CULTURE.md, "UI"; docs/PERSONS.md, "The fourth
//! build", rule 10, and "The fourth build as built, part 2A").
//!
//! Every number is the rules' read API's (`culture_channel_preview`, `culture_channel_options`,
//! `house_courtships`, `house_guard_with`, `house_requirement`, `player_culture_inflow_now`,
//! `player_culture_channel_base_now`): the view adds names and the cities' places and sends the
//! parts as fields, so the client lays out the tables without a formula of its own. Amounts of
//! culture are in hundredths, chances in hundredths of a percent.
//!
//! Every channel is public to the civilizations that have met its caster
//! (`Game::culture_channel_visible`).

use ffh_core::culture::{Channel, ChannelEffect, ChannelOrder, ChannelPreview, ChannelReach, ChannelTarget, Reach};
use ffh_core::ids::*;
use ffh_core::persons::{CourtshipView, GuardView, Requirement, SplitView};
use ffh_data::*;

use crate::json::Json;
use crate::view::Viewer;

/// The keywords whose reminders the Culture screen shows.
const KEYWORDS: &[&str] = &["KEYWORD_CHANNEL", "KEYWORD_COURT", "KEYWORD_GUARD"];

/// The reminders of keywords `names` (those the data has).
fn reminders(d: &GameData, names: &[&str]) -> Json {
    crate::view::keywords::reminders(d, names.iter().filter_map(|n| d.ty::<KeywordType>(n)))
}

/// A text of the rules with its keys resolved.
fn text(v: &Viewer, s: &str) -> String {
    crate::names::resolve_keys(v.g.d, s).into_owned()
}

/// A calculation's lines (the rules' `*_calc` reads), names resolved: the tooltip of the number
/// or line beside it (docs/PLAY_DESIGN.md, "Calculated numbers").
pub fn calc_lines(v: &Viewer, lines: &[String]) -> Json {
    Json::Arr(lines.iter().map(|l| Json::from(text(v, l))).collect())
}

/// How courtship `cv`'s numbers are made, from the human's side (`Game::house_courtship_calc`):
/// `{requirement, inflow, reach, gain, guard, turn}`, each a list of lines (`inflow` and `guard`
/// may be empty), the whole tooltip in that order.
pub fn courtship_calc_json(v: &Viewer, cv: &CourtshipView) -> Json {
    let c = v.g.house_courtship_calc(cv, v.h);
    crate::jobj! {
        "requirement" => calc_lines(v, &c.requirement),
        "inflow" => calc_lines(v, &c.inflow),
        "reach" => calc_lines(v, &c.reach),
        "gain" => calc_lines(v, &c.gain),
        "guard" => calc_lines(v, &c.guard),
        "turn" => calc_lines(v, &c.turn),
    }
}

/// A civilization's short name: `Sheaim`.
fn civ(v: &Viewer, p: PlayerId) -> String {
    text(v, &v.g.player_get_civilization_short_description(p))
}

/// An action's effect as the client names it (as the static data's `effect`).
fn effect_name(e: CultureEffect) -> &'static str {
    match e {
        CultureEffect::Court => "court",
        CultureEffect::Guard => "guard",
        CultureEffect::SpreadReligion => "spread_religion",
    }
}

fn reach_json(r: Option<Reach>) -> Json {
    r.map_or(Json::Null, |r| crate::jobj! { "distance" => r.distance, "effect" => r.effect })
}

/// A city as the screens name it: `{id, name, owner, x, y, known, own}` (`known`: the human's team
/// has seen it, so a link can show it on the map; `own`: the human's, a link opens its screen).
fn city_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let city = &g.cities[c];
    crate::jobj! {
        "id" => c,
        "name" => crate::view::city_name(g, c),
        "owner" => city.owner,
        "x" => city.x,
        "y" => city.y,
        "known" => v.reveal || city.revealed[v.team],
        "own" => city.owner == v.h,
    }
}

/// A channel's target: a House `{house, player (the House's player: what the command takes),
/// label, liege, liege_name}`, a civilization `{player, name}`.
pub fn target_json(v: &Viewer, target: ChannelTarget) -> Json {
    let g = v.g;
    match target {
        ChannelTarget::House(h) if g.houses.contains(h) => crate::jobj! {
            "house" => h,
            "player" => g.houses[h].player,
            "label" => text(v, &g.house_label(h)),
            "liege" => g.houses[h].owner,
            "liege_name" => civ(v, g.houses[h].owner),
        },
        ChannelTarget::House(h) => crate::jobj! { "house" => h, "player" => Json::Null, "label" => "a House that is gone" },
        ChannelTarget::Player(t) => crate::jobj! { "player" => t, "name" => civ(v, t) },
    }
}

/// An order: `{action, effect, religion, target, name}` (`name`: `Court House Aldric`).
fn order_json(v: &Viewer, o: &ChannelOrder) -> Json {
    let g = v.g;
    crate::jobj! {
        "action" => Json::idx(o.action),
        "effect" => effect_name(g.culture_effect(o.action)),
        "religion" => Json::opt_idx(o.religion),
        "target" => target_json(v, o.target),
        "name" => text(v, &g.culture_channel_name(o)),
    }
}

/// A requirement's parts (`Game::house_requirement`): `{total, loyalty, attitude, attitude_name,
/// unit, formula}`; `attitude_name` the attitude of House `h`'s leader toward `courter` that the
/// requirement reads.
fn requirement_json(v: &Viewer, h: HouseId, courter: PlayerId, r: &Requirement) -> Json {
    let g = v.g;
    let a = g.player_ai_get_attitude(g.houses[h].player, courter, true);
    crate::jobj! {
        "total" => r.total,
        "loyalty" => r.loyalty,
        "attitude" => r.attitude,
        "attitude_name" => crate::names::display(g.d, a).into_owned(),
        "unit" => r.unit,
        "formula" => r.formula(),
    }
}

fn split_json(v: &Viewer, sp: &SplitView) -> Json {
    crate::jobj! {
        "whole" => sp.whole,
        "going" => Json::Arr(sp.going.iter().map(|&c| city_json(v, c)).collect()),
        "held" => Json::Arr(sp.held.iter().map(|&c| city_json(v, c)).collect()),
    }
}

/// A courtship (`persons::CourtshipView`) with its names: `{house, label, liege, liege_name,
/// courter, courter_name, favour, inflow, gain, reach, guard, requirement, settles, turns, split,
/// attention, summary, line, calc}` (`calc`: [`courtship_calc_json`]).
fn courtship_json(v: &Viewer, cv: &CourtshipView) -> Json {
    let g = v.g;
    let h = cv.house;
    crate::jobj! {
        "house" => h,
        "label" => text(v, &g.house_label(h)),
        "liege" => g.houses[h].owner,
        "liege_name" => civ(v, g.houses[h].owner),
        "courter" => cv.courter,
        "courter_name" => civ(v, cv.courter),
        "favour" => cv.favour,
        "inflow" => cv.inflow,
        "gain" => cv.gain,
        "reach" => reach_json(cv.reach),
        "guard" => cv.guard,
        "requirement" => requirement_json(v, h, cv.courter, &cv.requirement),
        "settles" => cv.settles,
        "turns" => cv.turns,
        "split" => cv.split.as_ref().map_or(Json::Null, |sp| split_json(v, sp)),
        "attention" => cv.attention,
        "summary" => text(v, &cv.summary),
        "line" => text(v, &cv.line),
        "calc" => courtship_calc_json(v, cv),
    }
}

/// A guard (`persons::GuardView`): `{house, label, guard, reach, inflow, loss, courtships, line,
/// calc}` (`calc`: how the loss is made, `Game::house_guard_calc`),
/// each courtship `{courter, courter_name, favour, requirement, gain, settles_with,
/// settles_without, turns_with, turns_without}` (`requirement`: its total, in culture).
fn guard_json(v: &Viewer, gv: &GuardView) -> Json {
    let g = v.g;
    let h = gv.house;
    let courtships: Vec<Json> = gv
        .courtships
        .iter()
        .map(|c| {
            crate::jobj! {
                "courter" => c.courter,
                "courter_name" => civ(v, c.courter),
                "favour" => c.favour,
                "requirement" => g.house_requirement(h, c.courter).total,
                "gain" => c.gain,
                "settles_with" => c.settles_with,
                "settles_without" => c.settles_without,
                "turns_with" => c.turns_with,
                "turns_without" => c.turns_without,
            }
        })
        .collect();
    crate::jobj! {
        "house" => h,
        "label" => text(v, &g.house_label(h)),
        "guard" => gv.guard,
        "reach" => reach_json(gv.reach),
        "inflow" => gv.inflow,
        "loss" => gv.loss,
        "courtships" => Json::Arr(courtships),
        "line" => text(v, &gv.line),
        "calc" => calc_lines(v, &g.house_guard_calc(gv, v.h)),
    }
}

/// A Spread Religion's effect: `{chance, religion, cities, calc}`, each city `{city, distance,
/// effect, amount, has, can}` (`amount`: its share of the chance; `has`: it has the religion;
/// `can`: it could take it); `calc`: how the chance is made at inflow `inflow`
/// (`Game::culture_spread_calc`; empty without a religion).
fn spread_json(v: &Viewer, caster: PlayerId, religion: Option<ReligionType>, inflow: i32, chance: i32, reach: &[ChannelReach]) -> Json {
    let g = v.g;
    let calc = religion.map(|r| g.culture_spread_calc(r, inflow, reach)).unwrap_or_default();
    let cities: Vec<Json> = reach
        .iter()
        .map(|x| {
            crate::jobj! {
                "city" => city_json(v, x.city),
                "distance" => x.distance,
                "effect" => x.effect,
                "amount" => x.amount,
                "has" => religion.is_some_and(|r| g.cities[x.city].has_religion[r]),
                "can" => religion.is_some_and(|r| g.culture_can_take_religion(x.city, r, caster)),
            }
        })
        .collect();
    crate::jobj! { "chance" => chance, "religion" => Json::opt_idx(religion), "cities" => Json::Arr(cities), "calc" => calc_lines(v, &calc) }
}

/// An order previewed (`Game::culture_channel_preview`): `{order, refusal, inflow, reach, line,
/// court, guard, spread}`, one of the last three set by the order's effect.
fn preview_json(v: &Viewer, caster: PlayerId, pv: &ChannelPreview) -> Json {
    let (mut court, mut guard, mut spread) = (Json::Null, Json::Null, Json::Null);
    match &pv.effect {
        ChannelEffect::Court(cv) => court = courtship_json(v, cv),
        ChannelEffect::Guard(gv) => guard = guard_json(v, gv),
        ChannelEffect::Spread { chance, cities } => spread = spread_json(v, caster, pv.order.religion, pv.inflow, *chance, cities),
        ChannelEffect::None => {}
    }
    crate::jobj! {
        "order" => order_json(v, &pv.order),
        "refusal" => pv.refusal.as_deref().map(|s| text(v, s)),
        "inflow" => pv.inflow,
        "reach" => reach_json(pv.reach),
        "line" => text(v, &pv.line),
        "court" => court,
        "guard" => guard,
        "spread" => spread,
    }
}

/// A public channel: `{caster, caster_name, action, effect, religion, target, since_turn, name,
/// hostile}`.
pub fn channel_json(v: &Viewer, ch: &Channel) -> Json {
    let g = v.g;
    let c = &ch.channel;
    let o = c.order();
    crate::jobj! {
        "caster" => ch.caster,
        "caster_name" => civ(v, ch.caster),
        "action" => Json::idx(c.action),
        "effect" => effect_name(g.culture_effect(c.action)),
        "religion" => Json::opt_idx(c.religion),
        "target" => target_json(v, c.target),
        "since_turn" => c.since_turn,
        "name" => text(v, &g.culture_channel_name(&o)),
        "hostile" => g.culture_is_hostile(ch.caster, &o),
    }
}

/// Why action `effect` has no target to offer the human, in one line.
fn no_targets(effect: CultureEffect) -> &'static str {
    match effect {
        CultureEffect::Court => "No House of another realm that you have met.",
        CultureEffect::Guard => "You have no House.",
        CultureEffect::SpreadReligion => "None of your cities has a religion.",
    }
}

/// The chooser's entry for action `a`: `{action, effect, name, religions, options, none}`.
/// `religions`: Spread Religion's, those the human's cities have; `options`: the targets of
/// `Game::culture_channel_options` (for Spread Religion, those of each religion), each `{target,
/// religion, label, line, refusal, current, preview}` (`current`: the channel that stands;
/// `preview`: the order's full preview, null when refused); `none`: why there is no target.
fn action_json(v: &Viewer, a: CulturalActionType, refusal: Option<&str>) -> Json {
    let g = v.g;
    let h = v.h;
    let effect = g.culture_effect(a);
    let religions: Vec<ReligionType> = if effect == CultureEffect::SpreadReligion {
        g.d.religions.keys().filter(|&r| g.players[h].cities.iter().any(|&c| g.cities.contains(c) && g.cities[c].has_religion[r])).collect()
    } else {
        Vec::new()
    };
    let standing = g.players[h].culture_channel.as_ref().map(|ch| ch.order());
    let mut options = Vec::new();
    if refusal.is_none() {
        let picks: Vec<Option<ReligionType>> = if effect == CultureEffect::SpreadReligion { religions.iter().map(|&r| Some(r)).collect() } else { vec![None] };
        for religion in picks {
            for o in g.culture_channel_options(h, a, religion) {
                let order = ChannelOrder { action: a, target: o.target, religion };
                let preview = if o.refusal.is_none() { preview_json(v, h, &g.culture_channel_preview(h, &order)) } else { Json::Null };
                options.push(crate::jobj! {
                    "target" => target_json(v, o.target),
                    "religion" => Json::opt_idx(religion),
                    "label" => text(v, &o.label),
                    "line" => text(v, &o.line),
                    "refusal" => o.refusal.as_deref().map(|s| text(v, s)),
                    "current" => standing == Some(order),
                    "preview" => preview,
                });
            }
        }
    }
    let none = match refusal {
        Some(why) => Some(why.to_string()),
        None if options.is_empty() => Some(no_targets(effect).to_string()),
        None => None,
    };
    crate::jobj! {
        "action" => Json::idx(a),
        "effect" => effect_name(effect),
        "name" => crate::names::display(g.d, a).into_owned(),
        "religions" => Json::Arr(religions.into_iter().map(Json::idx).collect()),
        "options" => Json::Arr(options),
        "none" => none,
    }
}

/// The human's Houses that are courted, in id order: `{house, player, label, courtships, guarded,
/// guard, guard_order, guard_refusal}`. `courtships`: every courtship of the House
/// (`Game::house_courtships`); `guarded`: the human's channel guards it; `guard`: what a guard at
/// the human's inflow now would do (`Game::house_guard_with`), the Guard button's preview;
/// `guard_order`: that order (null without a Guard action); `guard_refusal`: why it cannot be set.
fn aimed_houses(v: &Viewer) -> Vec<Json> {
    let g = v.g;
    let h = v.h;
    let inflow = g.player_culture_inflow_now(h);
    let guard_action = g.culture_action(CultureEffect::Guard);
    let standing = g.players[h].culture_channel.as_ref().map(|ch| ch.order());
    let mut houses: Vec<HouseId> = g.houses.iter().filter(|(_, house)| house.owner == h).map(|(id, _)| id).collect();
    houses.sort();
    houses
        .into_iter()
        .filter_map(|hh| {
            let courtships = g.house_courtships(hh);
            if courtships.is_empty() {
                return None;
            }
            let order = guard_action.map(|action| ChannelOrder { action, target: ChannelTarget::House(hh), religion: None });
            let guarded = order.is_some() && standing == order;
            let refusal = match order {
                Some(o) if !guarded => g.culture_channel_check(h, &o).err().map(|s| text(v, &s)),
                _ => None,
            };
            Some(crate::jobj! {
                "house" => hh,
                "player" => g.houses[hh].player,
                "label" => text(v, &g.house_label(hh)),
                "courtships" => Json::Arr(courtships.iter().map(|cv| courtship_json(v, cv)).collect()),
                "guarded" => guarded,
                "guard" => guard_json(v, &g.house_guard_with(hh, inflow)),
                "guard_order" => order.map_or(Json::Null, |o| order_json(v, &o)),
                "guard_refusal" => refusal,
            })
        })
        .collect()
}

/// `culture`: the Culture screen (docs/HUMAN.md, "Culture"; docs/PERSONS.md, "The fourth build",
/// rule 10).
/// - `refusal`: why the human may set no channel at all (null: it may);
/// - `percent`, `inflow`, `base`: the share a channel takes, what the human's cities would divert
///   now and the culture of the cities that would divert (the head's `half of`); `inflow_calc`:
///   the inflow by city (`Game::player_culture_inflow_calc`, the head's tooltip);
/// - `channel`, `preview`: the human's channel (null: none) and its full preview;
/// - `actions`: the chooser (`action_json`);
/// - `aimed`: the human's courted Houses (`aimed_houses`); `aimed_spread`: the Spread Religion
///   channels aimed at its civilization, each with its preview's `line` and `chance`;
/// - `others`: the channels it sees between others;
/// - `memories`: what met leaders remember of its channels (`{player, civ, leader, hostility,
///   friendship, courted}`, each at most `memory_max`);
/// - `reminders`: the keywords Channel, Court and Guard.
pub fn culture(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let team = v.team;
    let refusal = g.culture_channel_refusal(h).map(|s| text(v, &s));
    let ours = g.players[h].culture_channel.clone();
    let channel = ours.clone().map_or(Json::Null, |channel| channel_json(v, &Channel { caster: h, channel }));
    let preview = ours.as_ref().map_or(Json::Null, |ch| preview_json(v, h, &g.culture_channel_preview(h, &ch.order())));
    let actions: Vec<Json> = d.cultural_actions.keys().map(|a| action_json(v, a, refusal.as_deref())).collect();
    let mut aimed_spread = Vec::new();
    for ch in g.culture_channels() {
        if ch.caster == h || (!v.reveal && !g.culture_channel_visible(team, ch.caster)) {
            continue;
        }
        let ChannelTarget::Player(t) = ch.channel.target else { continue };
        if g.players[t].team != team || g.players[ch.caster].team == team {
            continue;
        }
        let pv = g.culture_channel_preview(ch.caster, &ch.channel.order());
        let (chance, calc) = match (&pv.effect, ch.channel.religion) {
            (ChannelEffect::Spread { chance, cities }, Some(r)) => (*chance, g.culture_spread_calc(r, pv.inflow, cities)),
            _ => (0, Vec::new()),
        };
        let mut j = channel_json(v, &ch);
        j.set("line", text(v, &pv.line));
        j.set("chance", chance);
        j.set("calc", calc_lines(v, &calc));
        aimed_spread.push(j);
    }
    let others: Vec<Json> = g.culture_channels_between_others(team).iter().filter(|ch| ch.caster != h).map(|ch| channel_json(v, ch)).collect();
    let courted = MemoryType::MEMORY_COURTED_HOUSE;
    let memories: Vec<Json> = g
        .player_ids()
        .filter(|&o| o != h && g.players[o].alive && g.teams[team].has_met[g.players[o].team])
        .filter_map(|o| {
            let [hostility, friendship] = g.players[o].culture_memory[h];
            let court = g.players[o].ai_memory_count[h][courted];
            (hostility > 0 || friendship > 0 || court > 0).then(|| {
                crate::jobj! {
                    "player" => o,
                    "civ" => civ(v, o),
                    "leader" => crate::view::player_name(g, o),
                    "hostility" => hostility,
                    "friendship" => friendship,
                    "courted" => court,
                }
            })
        })
        .collect();
    crate::jobj! {
        "refusal" => refusal,
        "percent" => d.defines.culture_channel_percent,
        "inflow" => g.player_culture_inflow_now(h),
        "base" => g.player_culture_channel_base_now(h),
        "inflow_calc" => calc_lines(v, &g.player_culture_inflow_calc(h)),
        "channel" => channel,
        "preview" => preview,
        "actions" => Json::Arr(actions),
        "aimed" => Json::Arr(aimed_houses(v)),
        "aimed_spread" => Json::Arr(aimed_spread),
        "others" => Json::Arr(others),
        "memories" => Json::Arr(memories),
        "memory_max" => d.defines.culture_memory_max,
        "reminders" => reminders(d, KEYWORDS),
    }
}

/// The city query's `cultural` (city `c` of the human): `{rate, diverted, calc, disorder, channel,
/// reminders}`. `rate`: its culture a turn (hundredths); `diverted`: what it diverts a turn now
/// to its owner's channel (hundredths; 0 without one or in disorder); `calc`: how (`12.8 × 50% =
/// 6.4`, `Game::city_culture_diverted_calc`; null without a channel); `channel`: that channel
/// (`{name, action, effect, since_turn}`; null: none); `reminders`: the keyword Channel.
pub fn city_cultural(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let owner = g.cities[c].owner;
    let channel = g.players[owner]
        .culture_channel
        .as_ref()
        .filter(|_| owner == v.h || v.reveal || g.culture_channel_visible(v.team, owner))
        .map_or(Json::Null, |ch| {
            crate::jobj! {
                "name" => text(v, &g.culture_channel_name(&ch.order())),
                "action" => Json::idx(ch.action),
                "effect" => effect_name(g.culture_effect(ch.action)),
                "since_turn" => ch.since_turn,
            }
        });
    crate::jobj! {
        "rate" => g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE),
        "diverted" => g.city_culture_diverted_times100(c),
        "calc" => (owner == v.h && g.players[owner].culture_channel.is_some()).then(|| text(v, &g.city_culture_diverted_calc(c))),
        "disorder" => g.city_is_disorder(c),
        "channel" => channel,
        "reminders" => reminders(g.d, &["KEYWORD_CHANNEL"]),
    }
}
