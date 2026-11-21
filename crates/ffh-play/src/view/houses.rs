//! This game's Houses on the human's screens (docs/HOUSES.md, rule 15; docs/HUMAN.md, "Houses"):
//! the `houses` query (the Houses screen), a House as a card, the pick's offer on the decision,
//! the previews of an act (rule 12) for the questions where the act is done, a city's mark on the
//! map's plates and the Houses of another civilization for the diplomacy screen. Every line is the
//! rules' (`ffh_core::houses` read API); this module only lays them out as JSON.

use ffh_core::game::Game;
use ffh_core::houses::{HouseAct, HouseOffer, HousePreview, HouseState, HouseSummary, OpinionPart};
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::Viewer;

/// A rules line with any `TXT_KEY_*` token resolved.
fn line(d: &GameData, s: &str) -> Json {
    Json::from(crate::names::resolve_keys(d, s).into_owned())
}

fn lines(d: &GameData, list: &[String]) -> Json {
    Json::Arr(list.iter().map(|s| line(d, s)).collect())
}

/// The keywords of the Houses' screens: `House`, `Sympathy`, `Outsider` (those in the data).
pub fn keywords(d: &GameData) -> Vec<KeywordType> {
    ["KEYWORD_HOUSE", "KEYWORD_SYMPATHY", "KEYWORD_OUTSIDER"].iter().filter_map(|k| d.ty::<KeywordType>(k)).collect()
}

/// The keywords' reminders (`[{key, name, text, with}]`).
pub fn reminders(d: &GameData) -> Json {
    crate::view::keywords::reminders(d, keywords(d))
}

fn parts_json(d: &GameData, parts: &[OpinionPart]) -> Json {
    Json::Arr(parts.iter().map(|p| crate::jobj! { "key" => p.key, "label" => line(d, &p.label), "value" => p.value }).collect())
}

/// `{id, name, x, y}` of city `c`.
fn city_json(g: &Game, c: CityId) -> Json {
    let city = &g.cities[c];
    crate::jobj! { "id" => c, "name" => crate::view::city_name(g, c), "x" => city.x, "y" => city.y }
}

/// `Happiness: +1 in 3 cities.`
fn happiness_line(n: i32, cities: usize) -> String {
    format!("Happiness: {} in {cities} {}.", signed(n), if cities == 1 { "city" } else { "cities" })
}

fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// `Perk off: the House is Furious.` while the perk is off; null while it is on.
fn perk_off_line(on: bool) -> Option<&'static str> {
    (!on).then_some("Perk off: the House is Furious.")
}

/// House `s` as a card of its owner's Houses screen:
/// `{id, label, owner, leader, leader_name, epithet, origin, foreign, outsider, alignment, religion,
/// sympathy, lines, perk: {type, name, lines, on}, perk_off, state, opinion, opinion_line, parts,
/// calc, happiness, happiness_line, cities: [{id, name, x, y, seat}], formed_turn}`.
pub fn house_json(g: &Game, s: &HouseSummary) -> Json {
    let d = g.d;
    let cities: Vec<Json> = s
        .cities
        .iter()
        .enumerate()
        .filter(|(_, &c)| g.cities.contains(c))
        .map(|(i, &c)| city_json(g, c).with("seat", i == 0))
        .collect();
    crate::jobj! {
        "id" => s.id,
        "label" => s.label.as_str(),
        "owner" => s.owner,
        "leader" => Json::idx(s.leader),
        "leader_name" => s.leader_name.as_str(),
        "epithet" => s.epithet.clone(),
        "origin" => Json::idx(s.origin),
        "foreign" => s.foreign,
        "outsider" => s.outsider,
        "alignment" => Json::opt_idx(s.alignment),
        "religion" => Json::opt_idx(s.religion),
        "sympathy" => s.sympathy,
        "lines" => lines(d, &s.lines),
        "perk" => crate::jobj! { "type" => Json::idx(s.perk), "name" => s.perk_name.as_str(), "lines" => lines(d, &s.perk_lines), "on" => s.perk_on },
        "perk_off" => perk_off_line(s.perk_on),
        "state" => s.state.name(),
        "opinion" => s.opinion,
        "opinion_line" => format!("Opinion: {} ({})", s.state.name(), s.opinion),
        "parts" => parts_json(d, &s.parts),
        "calc" => lines(d, &s.calc),
        "happiness" => s.happiness,
        "happiness_line" => happiness_line(s.happiness, s.cities.len()),
        "cities" => Json::Arr(cities),
        "formed_turn" => s.formed_turn,
    }
}

/// The `houses` query: the human's Houses screen. `{houses: [house_json], unhoused: [{id, name, x,
/// y, reason}], reminders}`; empty lists for a civilization without Houses.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let houses: Vec<Json> = g.player_house_summaries(v.h).iter().map(|s| house_json(g, s)).collect();
    let unhoused: Vec<Json> = g.player_unhoused_cities(v.h).iter().map(|u| city_json(g, u.city).with("reason", line(d, &u.reason))).collect();
    crate::jobj! {
        "has_houses" => g.player_has_houses(v.h),
        "houses" => Json::Arr(houses),
        "unhoused" => Json::Arr(unhoused),
        "reminders" => reminders(d),
    }
}

/// A House of another civilization as the diplomacy screen lists it for whoever has met its
/// owner: `{id, label, leader, leader_name, epithet, alignment, religion, sympathy, outsider,
/// lines, perk, perk_on, state, cities}` (its state toward its own ruler; `cities`: how many).
fn foreign_house_json(g: &Game, s: &HouseSummary) -> Json {
    let d = g.d;
    crate::jobj! {
        "id" => s.id,
        "label" => s.label.as_str(),
        "leader" => Json::idx(s.leader),
        "leader_name" => s.leader_name.as_str(),
        "epithet" => s.epithet.clone(),
        "alignment" => Json::opt_idx(s.alignment),
        "religion" => Json::opt_idx(s.religion),
        "sympathy" => s.sympathy,
        "outsider" => s.outsider,
        "lines" => lines(d, &s.lines),
        "perk" => s.perk_name.as_str(),
        "perk_on" => s.perk_on,
        "state" => s.state.name(),
        "cities" => s.cities.len(),
    }
}

/// Player `p`'s Houses for the diplomacy screen (the `players` query's `houses`), as the viewer
/// may know them: listed when the viewer has met `p` (the query lists only players met).
pub fn player_houses(v: &Viewer, p: PlayerId) -> Json {
    let g = v.g;
    if p != v.h && !v.player_met(p) {
        return Json::Arr(Vec::new());
    }
    Json::Arr(g.player_house_summaries(p).iter().map(|s| foreign_house_json(g, s)).collect())
}

/// The previews of act `act` of the human (rule 12): `[{house, label, before, after,
/// happiness_before, happiness_after, cities, lines}]`, one per House whose state would change
/// (empty when none).
pub fn preview(v: &Viewer, act: HouseAct) -> Json {
    let g = v.g;
    if !g.player_has_houses(v.h) {
        return Json::Arr(Vec::new());
    }
    Json::Arr(g.house_preview(v.h, act).iter().map(|x| preview_json(g, x)).collect())
}

fn preview_json(g: &Game, x: &HousePreview) -> Json {
    crate::jobj! {
        "house" => x.house,
        "label" => g.house_label(x.house),
        "before" => x.before.name(),
        "after" => x.after.name(),
        "happiness_before" => x.happiness_before,
        "happiness_after" => x.happiness_after,
        "cities" => x.cities,
        "lines" => lines(g.d, &x.lines),
    }
}

/// City `c`'s House as its plate marks it for its owner (`{id, label, state}`), null for a city
/// in no House or of another player.
pub fn city_mark(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let city = &g.cities[c];
    if city.owner != v.h {
        return Json::Null;
    }
    match city.house.filter(|&h| g.houses.contains(h)) {
        Some(h) => crate::jobj! { "id" => h, "label" => g.house_label(h), "state" => g.house_state(h).name() },
        None => Json::Null,
    }
}

/// The human's Houses in brief for the alerts (the view's `houses`): `[{id, label, leader,
/// leader_name, state}]`.
pub fn brief(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    Json::Arr(
        g.player_houses(v.h)
            .iter()
            .filter(|&&h| g.houses.contains(h))
            .map(|&h| {
                let house = &g.houses[h];
                crate::jobj! {
                    "id" => h,
                    "label" => g.house_label(h),
                    "leader" => Json::idx(house.leader),
                    "leader_name" => ffh_core::houses::leader_name(d, house.leader),
                    "state" => g.house_state(h).name(),
                }
            })
            .collect(),
    )
}

/// The pick's offer as the decision carries it (docs/HOUSES.md, rule 5; the `house_leader`
/// decision's `offer`): `{name, label, origin, foreign, tier, head, cities: [{id, name, x, y}],
/// culture: [{player, name, pct, yours}], cards: [{index, leader, leader_type, name, epithet,
/// outsider, alignment, religion, sympathy, lines, perk: {type, name, lines}, opinion, state,
/// opinion_line, parts, calc, happiness, happiness_line, advisor}], advisor, reminders}`.
/// `lines`: the card's alignment, religion, sympathy (or the outsider's line) and perk lines.
pub fn offer_json(g: &Game, offer: &HouseOffer, advisor: Option<usize>) -> Json {
    let d = g.d;
    let n = offer.cities.len();
    let cards: Vec<Json> = offer
        .cards
        .iter()
        .enumerate()
        .map(|(i, card)| {
            let c = &card.candidate;
            let total = card.opinion.total();
            let parts = g.house_opinion_part_list(offer.owner, &c.leaning(), &card.opinion);
            let opinion_line = card.lines.iter().find(|l| l.starts_with("Opinion now: ")).cloned().unwrap_or_else(|| format!("Opinion now: {} ({total})", card.state.name()));
            crate::jobj! {
                "index" => i,
                "leader" => Json::idx(c.leader),
                "leader_type" => d.type_name(c.leader),
                "name" => card.name.as_str(),
                "epithet" => card.epithet.clone(),
                "outsider" => c.outsider,
                "alignment" => Json::opt_idx(c.alignment),
                "religion" => Json::opt_idx(c.religion),
                "sympathy" => c.sympathy,
                "lines" => lines(d, &g.house_leaning_lines(&c.leaning(), c.perk)),
                "perk" => crate::jobj! { "type" => Json::idx(c.perk), "name" => ffh_core::houses::perk_name(d, c.perk), "lines" => lines(d, &card.perk_lines) },
                "opinion" => total,
                "state" => card.state.name(),
                "opinion_line" => line(d, &opinion_line),
                "parts" => parts_json(d, &parts),
                "calc" => lines(d, &g.house_card_calc(offer.owner, card, n)),
                "happiness" => card.happiness,
                "happiness_line" => happiness_line(card.happiness, n),
                "perk_off" => perk_off_line(card.state != HouseState::Furious),
                "advisor" => advisor == Some(i),
            }
        })
        .collect();
    let culture: Vec<Json> = offer
        .culture
        .iter()
        .map(|&(q, pct)| crate::jobj! { "player" => q, "name" => g.house_civ_name(q), "pct" => pct, "yours" => q == offer.owner })
        .collect();
    crate::jobj! {
        "name" => offer.name.as_str(),
        "label" => format!("House {}", offer.name),
        "origin" => Json::idx(offer.origin),
        "foreign" => offer.foreign,
        "tier" => offer.tier,
        "head" => lines(d, &offer.head(g)),
        "cities" => Json::Arr(offer.cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| city_json(g, c)).collect()),
        "culture" => Json::Arr(culture),
        "cards" => Json::Arr(cards),
        "advisor" => advisor,
        "reminders" => reminders(d),
    }
}
