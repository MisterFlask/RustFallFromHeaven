//! Persons of Note as the human sees them (docs/PERSONS.md, "The second build", rule 13; "UI":
//! parts A, B and J): the Court screen (`court`: the realm, the Crown and its limit, a card per
//! House with its tribute selector, levy and Free, the commanders and persons without a post), a
//! city's grant list (`grant {city}`), the count of what needs attention (the view's
//! `court_attention`, the menu's badge), a requisition candidate's House line (the levy, for the
//! Armies screen), a Crown city's Governance line (the city screen) and the check of the command
//! `set_commander`.
//!
//! Every line the page shows about the rules is made here, from the rules' own read API
//! (`ffh_core::persons`: `court_view`, `house_view`, `person_view`, `grant_candidates`, the
//! label functions); the page lays the lines out and decides nothing. Pure: reads `&Game` only.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{Claimant, CourtView, DemandKind, GrantCandidate, GrantTo, HouseOrigin, HouseView, LoyaltyPart, LoyaltyState, PersonOrigin, PersonView, PersonWhere, Post, PreviewLine, DEMAND_WARNING_TURNS};
use ffh_data::*;

use crate::json::Json;
use crate::names;
use crate::query::QueryErr;
use crate::view::{city_name, Viewer};

// ---------------------------------------------------------------------------------------------
// Small texts
// ---------------------------------------------------------------------------------------------

/// A core text with any `TXT_KEY_*` resolved (civilization names come as keys).
fn core(g: &Game, s: &str) -> String {
    names::resolve_keys(g.d, s).into_owned()
}

/// Hundredths of gold as the screens write them: `6.2`, `9` (rounded to a tenth).
fn gold(times100: i32) -> String {
    let tenths = (times100.abs() + 5) / 10;
    let sign = if times100 < 0 && tenths > 0 { "-" } else { "" };
    if tenths % 10 == 0 {
        format!("{sign}{}", tenths / 10)
    } else {
        format!("{sign}{}.{}", tenths / 10, tenths % 10)
    }
}

/// `+3`, `-2`, `0`.
fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// `1 turn`, `8 turns`.
fn turns(n: i32) -> String {
    format!("{n} {}", if n == 1 { "turn" } else { "turns" })
}

/// `Demand refused: -3 (41 turns)`; a standing part without turns: `State religion opposed: -2`.
fn part_text(p: &LoyaltyPart) -> String {
    match p.turns_left {
        Some(t) => format!("{}: {} ({})", p.label, signed(p.amount), turns(t)),
        None => format!("{}: {}", p.label, signed(p.amount)),
    }
}

/// The loyalty parts, one line each, and the total line.
fn parts_json(parts: &[LoyaltyPart], total: i32) -> (Json, String) {
    (Json::Arr(parts.iter().map(|p| Json::from(part_text(p))).collect()), format!("Loyalty: {}.", signed(total)))
}

fn state_name(s: LoyaltyState) -> &'static str {
    s.name()
}

/// The human's government civic, if any.
fn government(g: &Game, h: PlayerId) -> Option<CivicType> {
    let d = g.d;
    d.civic_options.keys().find(|&o| d.civic_options[o].base.type_ == "CIVICOPTION_GOVERNMENT").and_then(|o| g.players[h].civics[o])
}

fn city_ref(g: &Game, c: CityId) -> Json {
    crate::jobj! { "city" => c, "name" => city_name(g, c), "x" => g.cities[c].x, "y" => g.cities[c].y }
}

// ---------------------------------------------------------------------------------------------
// Persons and their demands
// ---------------------------------------------------------------------------------------------

/// The plot person `x`'s demands and buttons measure from: their unit's, their seat's, the capital.
fn person_plot(g: &Game, x: PersonId) -> Option<(i32, i32)> {
    if let Some(plot) = g.person_unit(x).and_then(|u| g.unit_plot_opt(u)) {
        return Some(g.map.xy(plot));
    }
    if let Some(c) = g.person_house(x).and_then(|h| g.house_seat(h)) {
        return Some((g.cities[c].x, g.cities[c].y));
    }
    let p = g.persons[x].owner;
    g.player_get_capital_city(p).map(|c| (g.cities[c].x, g.cities[c].y))
}

/// The Crown's grantable city nearest person `x` (ties: the lower id).
fn nearest_grantable(g: &Game, h: PlayerId, x: PersonId, grantable: &[CityId]) -> Option<CityId> {
    let (px, py) = person_plot(g, x).unwrap_or((0, 0));
    let _ = h;
    grantable.iter().copied().min_by_key(|&c| (g.plot_distance(px, py, g.cities[c].x, g.cities[c].y), c))
}

/// Who `x` is in the grant list's `prefer` (`{house}` for a head, else `{person}`).
fn prefer_of(g: &Game, x: PersonId) -> Json {
    match g.person_house(x) {
        Some(h) => crate::jobj! { "house" => h },
        None => crate::jobj! { "person" => x },
    }
}

/// The demand line of person `x` with its one button (docs/PERSONS.md, "UI", the demand line), or
/// between demands `No demand. Next in 14 turns.`
fn demand_json(v: &Viewer, pv: &PersonView, grantable: &[CityId]) -> Json {
    let g = v.g;
    let h = v.h;
    let team = g.players[h].team;
    let Some(dm) = &pv.demand else {
        let text = if pv.demand_rest_turns > 0 { format!("No demand. Next in {}.", turns(pv.demand_rest_turns)) } else { "No demand.".to_string() };
        return crate::jobj! { "kind" => Json::Null, "text" => text, "turns_left" => Json::Null, "urgent" => false, "paused" => false, "note" => Json::Null, "button" => Json::Null };
    };
    let what = core(g, &dm.text);
    let urgent = !dm.paused && dm.turns_left <= DEMAND_WARNING_TURNS;
    let show = |plot: PlotIdx| {
        let (x, y) = g.map.xy(plot);
        // "On the map": it closes the Court and centres the map (the attention block's "Show"
        // opens a card within the Court).
        crate::jobj! { "label" => "On the map", "act" => crate::jobj! { "kind" => "show", "x" => x, "y" => y } }
    };
    // The owner of a foreign city named by plot, and whether a war is needed for it.
    let owner_note = |plot: PlotIdx| -> Option<String> {
        let c = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))?;
        let o = g.cities[c].owner;
        if o == h {
            return None;
        }
        let civ = core(g, &g.player_get_civilization_short_description(o));
        let ot = g.players[o].team;
        Some(if g.teams[team].at_war[ot] { format!("Owner: {civ}. At war.") } else { format!("Owner: {civ}. At peace.") })
    };
    let (kind, note, button): (&str, Option<String>, Json) = match dm.kind {
        DemandKind::Post | DemandKind::House => {
            let kind = if dm.kind == DemandKind::Post { "post" } else { "house" };
            match nearest_grantable(g, h, pv.id, grantable) {
                Some(c) => (kind, None, crate::jobj! { "label" => "Grant a city", "act" => crate::jobj! { "kind" => "grant", "city" => c, "prefer" => prefer_of(g, pv.id) } }),
                None => (kind, Some("No city of the Crown can be granted.".to_string()), Json::Null),
            }
        }
        DemandKind::GrantCity(plot) => {
            let c = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c));
            match c {
                Some(c) if g.cities[c].owner == h => match g.player_check_grant_city(h, c) {
                    Ok(()) => ("grant_city", None, crate::jobj! { "label" => format!("Grant {}", city_name(g, c)), "act" => crate::jobj! { "kind" => "grant", "city" => c, "prefer" => prefer_of(g, pv.id) } }),
                    Err(why) => ("grant_city", Some(why.to_string()), Json::Null),
                },
                _ => ("grant_city", owner_note(plot), show(plot)),
            }
        }
        DemandKind::TakeCity(plot) => ("take_city", owner_note(plot), show(plot)),
        DemandKind::War(q) => ("war", None, crate::jobj! { "label" => "Diplomacy", "act" => crate::jobj! { "kind" => "diplomacy", "player" => q } }),
        DemandKind::AdoptReligion => ("adopt_religion", None, crate::jobj! { "label" => "Religion", "act" => crate::jobj! { "kind" => "religion" } }),
        DemandKind::SpreadReligion(plot) => ("spread_religion", None, show(plot)),
        DemandKind::Wonder { city, building } => {
            let seat = g.map.plots[city].plot_city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == h);
            match seat {
                Some(c) if dm.paused => {
                    let i = g.cities[c].order_queue.iter().position(|o| o.order_type == OrderType::ORDER_CONSTRUCT && o.data1 == building.idx() as i32);
                    let t = i.map_or(0, |i| g.city_order_turns_left(c, i));
                    ("wonder", Some(format!("In production: {}. The term does not run.", turns(t))), Json::Null)
                }
                Some(c) => {
                    let data = building.idx() as i32;
                    let refusal = g.city_check_empire_order(h, c, OrderType::ORDER_CONSTRUCT, data).err();
                    let what = names::display(g.d, building).into_owned();
                    let line = format!("{what} in {}: {}, ahead of what the city builds.", city_name(g, c), turns(g.city_production_turns_left_building(c, building, 0)));
                    let act = crate::jobj! {
                        "kind" => "order", "city" => c, "name" => city_name(g, c), "order" => Json::idx(OrderType::ORDER_CONSTRUCT), "data" => data,
                        "what" => what, "line" => line,
                    };
                    let mut b = crate::jobj! { "label" => "Order", "act" => act };
                    if let Some(why) = refusal {
                        b.set("refusal", why);
                    }
                    ("wonder", None, b)
                }
                None => ("wonder", None, Json::Null),
            }
        }
    };
    crate::jobj! {
        "kind" => kind,
        "text" => format!("Demands: {what}"),
        "turns_left" => dm.turns_left,
        "turns" => if dm.paused { format!("{} (paused).", turns(dm.turns_left)) } else { format!("{}.", turns(dm.turns_left)) },
        "urgent" => urgent,
        "paused" => dm.paused,
        "note" => note,
        "button" => button,
    }
}

/// `Veteran: level 7 Axeman, 2nd Army`; `Hero: level 3 Ranger, in Torrin`; `Great person: no unit.`
fn where_text(g: &Game, pv: &PersonView) -> String {
    let origin = match pv.origin {
        PersonOrigin::Hero => "Hero",
        PersonOrigin::Veteran => "Veteran",
        PersonOrigin::GreatPerson => "Great person",
        PersonOrigin::Ruler => "Ruler",
    };
    // A House's head leads it, off the map.
    if let Post::Head(hh) = pv.post {
        if g.houses.contains(hh) {
            return format!("{origin}: leads {}.", g.house_label(hh));
        }
    }
    let Some(t) = pv.unit_type else { return format!("{origin}: no unit.") };
    // A hero's unit type is its name (`Donal Lugh`): not said twice.
    let type_name = names::display(g.d, t);
    let unit = if *type_name == *pv.name { format!("level {}", pv.level.unwrap_or(0)) } else { format!("level {} {type_name}", pv.level.unwrap_or(0)) };
    let at = match pv.location {
        PersonWhere::Army(a) => format!(", {}", g.armies[a].name),
        PersonWhere::City(c) => format!(", in {}", city_name(g, c)),
        PersonWhere::Plot(_) => ", in the field".to_string(),
        PersonWhere::Away => String::new(),
    };
    format!("{origin}: {unit}{at}.")
}

/// `Commander of the 2nd Army.`, `Head of House Arne.`, `No post.`
fn post_text(g: &Game, pv: &PersonView) -> String {
    match pv.post {
        Post::Commander(a) if g.armies.contains(a) => format!("Commander of the {}.", g.armies[a].name),
        Post::Head(hh) if g.houses.contains(hh) => format!("Head of {}.", g.house_label(hh)),
        _ => "No post.".to_string(),
    }
}

fn ambition_text(pv: &PersonView) -> String {
    let a = match pv.ambition {
        ffh_core::persons::Ambition::Land => "Land",
        ffh_core::persons::Ambition::War => "War",
        ffh_core::persons::Ambition::Faith => "Faith",
        ffh_core::persons::Ambition::Works => "Works",
    };
    format!("Ambition: {a}.")
}

/// A person's card: who they are (portrait: `leader`, else their unit's `unit_type`), where, their
/// post, state, demand, and opened: loyalty parts, ambition, trait, rival, record.
fn person_json(v: &Viewer, pv: &PersonView, grantable: &[CityId]) -> Json {
    let g = v.g;
    let unit_at = pv.unit.and_then(|u| g.unit_plot_opt(u)).map(|q| g.map.xy(q));
    let (parts, total) = parts_json(&pv.parts, pv.loyalty);
    let record: Vec<Json> = pv.deeds.iter().rev().map(|dd| Json::from(format!("Turn {}: {}", dd.turn, core(g, &dd.text)))).collect();
    crate::jobj! {
        "id" => pv.id,
        "name" => pv.name.as_str(),
        "title" => pv.title.clone(),
        "leader" => Json::opt_idx(pv.leader),
        "origin" => match pv.origin { PersonOrigin::Hero => "hero", PersonOrigin::Veteran => "veteran", PersonOrigin::GreatPerson => "great_person", PersonOrigin::Ruler => "ruler" },
        "unit" => pv.unit,
        "unit_type" => Json::opt_idx(pv.unit_type),
        "x" => unit_at.map(|p| p.0),
        "y" => unit_at.map(|p| p.1),
        "where" => where_text(g, pv),
        "post" => post_text(g, pv),
        "army" => match pv.post { Post::Commander(a) if g.armies.contains(a) => Some(a), _ => None },
        "house" => match pv.post { Post::Head(hh) if g.houses.contains(hh) => Some(hh), _ => None },
        "state" => state_name(pv.state),
        "countdown_turns" => pv.countdown_turns,
        "outcome" => pv.outcome.as_deref().map(|s| core(g, s)),
        "demand" => demand_json(v, pv, grantable),
        "ambition" => ambition_text(pv),
        "rival" => pv.rival_name.as_ref().map(|r| format!("Rival: {r}.")),
        "parts" => parts,
        "total" => total,
        "record" => Json::Arr(record),
        "backstory" => pv.leader.map(|l| names::text(g.d, &g.d.leader_heads[l].base.civilopedia).to_string()).filter(|s| !s.is_empty() && !s.starts_with("TXT_KEY")),
    }
}

// ---------------------------------------------------------------------------------------------
// The Crown
// ---------------------------------------------------------------------------------------------

/// Whether player `h` counts settlements apart (a city cap: the Kuriotates).
fn has_settlements(g: &Game, h: PlayerId) -> bool {
    g.players[h].max_cities != -1
}

/// `1 city`, `4 cities`.
fn cities_text(n: i32) -> String {
    if n == 1 {
        "1 city".to_string()
    } else {
        format!("{n} cities")
    }
}

/// Why city `c` pays no maintenance now (`Myrean pays none while disorder lasts.`), if it pays
/// none for a while: in disorder, or during We Love the King Day.
fn pays_none_now(g: &Game, c: CityId) -> Option<String> {
    let city = &g.cities[c];
    if g.city_is_disorder(c) {
        Some(format!("{} pays none while disorder lasts.", city_name(g, c)))
    } else if city.we_love_the_king_day {
        Some(format!("{} pays none while We Love the King Day lasts.", city_name(g, c)))
    } else {
        None
    }
}

/// City `c`'s own maintenance at its lasting rate (hundredths of gold a turn, inflation included):
/// what it pays once a disorder or a We Love the King Day ends (the rules' base maintenance, the
/// Crown's limit included, times its modifier).
fn lasting_maintenance_times100(g: &Game, c: CityId) -> i32 {
    let city = &g.cities[c];
    if city.population <= 0 || city.outpost {
        return 0;
    }
    let own = g.city_calculate_base_maintenance_times100(c) * (city.maintenance_modifier + 100).max(0) / 100;
    own * (g.player_calculate_inflation_rate(city.owner) + 100).max(0) / 100
}

/// What the Crown's cities over its limit cost beyond the normal rate, at their lasting rate
/// (hundredths of gold a turn): the rules' `player_crown_over_cost_times100`, with a city that
/// pays none now counted at what it pays once that ends (its city-count maintenance, the limit's
/// factor included, less the normal rate).
fn crown_over_lasting_times100(g: &Game, h: PlayerId, over: &[CityId]) -> i32 {
    let f = ffh_core::persons::CROWN_OVER_FACTOR;
    let mut extra = 0;
    for &c in over {
        let city = &g.cities[c];
        if city.population <= 0 {
            continue;
        }
        let num = g.city_calculate_num_cities_maintenance_times100(c);
        extra += num * (f - 1) / f * (city.maintenance_modifier + 100).max(0) / 100;
    }
    extra * (g.player_calculate_inflation_rate(h) + 100).max(0) / 100
}

/// The Crown's limit as the rules state it (docs/PERSONS.md, rule 14): `The Crown holds 5
/// cities, 2 over its limit of 3: +8.4 gold a turn.` or `The Crown holds 3 cities; its limit is
/// 3.`, with `Settlements are not counted.` for a civilization with settlements; `over`: beyond the
/// limit (the caution colour). The cost is at the cities' lasting rate, a city that pays none now
/// said apart (`Myrean pays none while disorder lasts.`). `None` where there is no limit (no city
/// can be granted).
fn crown_line(g: &Game, h: PlayerId, cv: &CourtView) -> Option<Json> {
    let limit = cv.crown_limit?;
    let n = cv.crown_cities.len() as i32;
    let over = cv.crown_over.len();
    let mut text = if over > 0 {
        let cost = crown_over_lasting_times100(g, h, &cv.crown_over).max(cv.crown_over_cost_times100);
        let mut t = format!("The Crown holds {}, {over} over its limit of {limit}: +{} gold a turn.", cities_text(n), gold(cost));
        for &c in &cv.crown_over {
            if let Some(why) = pays_none_now(g, c) {
                t.push(' ');
                t.push_str(&why);
            }
        }
        t
    } else {
        format!("The Crown holds {}; its limit is {limit}.", cities_text(n))
    };
    if has_settlements(g, h) {
        text.push_str(" Settlements are not counted.");
    }
    Some(crate::jobj! { "text" => text, "over" => over > 0 })
}

/// The Crown's card's cities: the Crown's in its order (the capital first), then the player's
/// settlements and outposts: `{city, name, x, y, capital, grantable, refusal, over, note}`
/// (`note`: `Over the Crown's limit.`, `Settlement: not counted.`).
fn crown_cities_json(g: &Game, h: PlayerId, cv: &CourtView) -> Vec<Json> {
    let mut list: Vec<CityId> = cv.crown_cities.iter().copied().filter(|&c| g.cities.contains(c)).collect();
    let rest: Vec<CityId> = g.players[h].cities.iter().copied().filter(|&c| g.cities.contains(c) && !list.contains(&c)).collect();
    list.extend(rest);
    list.iter()
        .map(|&c| {
            let why = g.player_check_grant_city(h, c).err();
            let over = cv.crown_over.contains(&c);
            let note = if over {
                Some("Over the Crown's limit.")
            } else if g.cities[c].settlement && cv.crown_limit.is_some() {
                Some("Settlement: not counted.")
            } else {
                None
            };
            city_ref(g, c).with("capital", g.city_is_capital(c)).with("grantable", why.is_none()).with("refusal", why).with("over", over).with("note", note)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Houses
// ---------------------------------------------------------------------------------------------

/// `Founded on turn 3.`, `Capitulated on turn 120.`, `Vassal since turn 40.`
fn since_text(hv: &HouseView) -> String {
    match hv.origin {
        HouseOrigin::Founded => format!("Founded on turn {}.", hv.founded_turn),
        HouseOrigin::Capitulated => format!("Capitulated on turn {}.", hv.founded_turn),
        HouseOrigin::Vassal => format!("Vassal since turn {}.", hv.founded_turn),
    }
}

/// A House's size against its realm's threshold (rule 14), from the rules' read: `Size: 2 of 3
/// cities without a grievance.`, or beyond it `Size: 5 cities, more than 3: -2.`; `over`.
fn size_json(hv: &HouseView) -> Json {
    let (n, t) = (hv.great_cities, hv.size_threshold);
    if n > t {
        crate::jobj! { "text" => format!("Size: {}, more than {t}: {}.", cities_text(n), t - n), "over" => true }
    } else {
        crate::jobj! { "text" => format!("Size: {n} of {} without a grievance.", cities_text(t)), "over" => false }
    }
}

/// `12 gold, 8 research`.
fn gold_research(gold: i32, research: i32) -> String {
    format!("{gold} gold, {research} research")
}

/// A House's tribute (rule 4) and its selector: `pct`; `line` (`Tribute 30%: 12 gold, 8 research
/// last turn.`, or `Tribute 30%. Not paid yet.`); `rates`: each rate it may be set to (0 to 50 by
/// 10), from the rules' read (`TributeView::at`): `{pct, current, estimate (what it would pay this
/// turn), part (its tribute part at that rate), part_text, loyalty (its total), loyalty_text,
/// state, changes (its state differs from now)}`.
fn tribute_json(g: &Game, hv: &HouseView) -> Json {
    let t = &hv.tribute;
    let line = if t.last_turn < 0 {
        format!("Tribute {}%. Not paid yet.", t.pct)
    } else if t.last_turn >= g.game_turn() - 1 {
        format!("Tribute {}%: {} last turn.", t.pct, gold_research(t.last_gold, t.last_research))
    } else {
        format!("Tribute {}%: {} on turn {}.", t.pct, gold_research(t.last_gold, t.last_research), t.last_turn)
    };
    // The tribute part now (the parts' `Tribute N%`), and at each rate: the part changes by what
    // the total does.
    let part_now = hv.parts.iter().find(|p| p.label.starts_with("Tribute ") && p.turns_left.is_none()).map_or(0, |p| p.amount);
    let rates: Vec<Json> = t
        .at
        .iter()
        .map(|&(pct, gold, research, loyalty, state)| {
            let part = part_now + loyalty - hv.loyalty;
            crate::jobj! {
                "pct" => pct,
                "current" => pct == t.pct,
                "estimate" => format!("Pays {}.", gold_research(gold, research)),
                "part" => part,
                "part_text" => format!("Tribute: {}", signed(part)),
                "loyalty" => loyalty,
                "loyalty_text" => format!("Loyalty: {}", signed(loyalty)),
                "state" => state_name(state),
                "changes" => state != hv.state,
            }
        })
        .collect();
    crate::jobj! { "pct" => t.pct, "line" => line, "rates" => Json::Arr(rates) }
}

/// A House's levy status (rule 6): `{takes, text}`: `Takes levies. Each levy: -1 (20 turns).` or
/// the rules' reason it takes none (`House Arne is Annoyed. It takes no levy.`).
fn levy_status(reason: Option<&str>) -> Json {
    let (amount, t) = ffh_core::persons::LEVIED;
    let text = match reason {
        Some(why) => why.to_string(),
        None => format!("Takes levies. Each levy: {} ({}).", signed(amount), turns(t)),
    };
    crate::jobj! { "takes" => reason.is_none(), "text" => text }
}

/// A House's card (rule 13): who leads it (the head's card: portrait, name, title, demand,
/// record), its attitude, countdown and outcome, its loyalty parts, its tribute and the selector,
/// its levy, its cities and its size, its price to secede, and Free.
fn house_json(v: &Viewer, hv: &HouseView, persons: &[PersonView], grantable: &[CityId]) -> Json {
    let g = v.g;
    let head = hv.head.and_then(|x| persons.iter().find(|p| p.id == x)).map(|pv| person_json(v, pv, grantable));
    let (parts, total) = parts_json(&hv.parts, hv.loyalty);
    let (_, held) = g.house_cities_held(hv.id);
    let cities: Vec<Json> = hv.cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| city_ref(g, c).with("held", held.contains(&c)).with("seat", hv.seat == Some(c))).collect();
    let price = match hv.bribe_price {
        Some(p) => format!("Price to secede: {p} gold."),
        None => "Refuses any offer to secede.".to_string(),
    };
    crate::jobj! {
        "id" => hv.id,
        "player" => hv.player,
        "label" => hv.label.as_str(),
        "civilization" => core(g, &hv.civilization_name),
        "leader" => Json::opt_idx(hv.leader),
        "color" => hv.color as i64,
        "state" => state_name(hv.state),
        "countdown_turns" => hv.countdown_turns,
        "outcome" => hv.outcome.as_deref().map(|s| core(g, s)),
        "held" => (hv.countdown_turns.is_some() && !held.is_empty()).then(|| crate::view::house_marks::stays_line(g, &held)),
        "head" => head,
        "since" => since_text(hv),
        "cities" => Json::Arr(cities),
        "size" => size_json(hv),
        "tribute" => tribute_json(g, hv),
        "levy" => levy_status(hv.levy.reason.as_deref()),
        "price" => price,
        "free" => crate::jobj! { "ok" => hv.free.is_ok(), "refusal" => hv.free.err(), "confirm" => format!("Free {}? It is no longer our vassal, at peace with us. This cannot be undone.", hv.label) },
        "parts" => parts,
        "total" => total,
    }
}

// ---------------------------------------------------------------------------------------------
// The Court
// ---------------------------------------------------------------------------------------------

/// The heads of the screen when there is nothing to list (principle 6): why, in one or two lines.
fn empty_lines(g: &Game, h: PlayerId, grants: Result<(), &'static str>, persons: usize, houses: usize) -> Vec<String> {
    let gov = government(g, h).map(|c| names::display(g.d, c).into_owned()).unwrap_or_else(|| "none".into());
    if !g.player_has_persons(h) {
        return vec!["This civilization has no persons of note and no Houses.".to_string()];
    }
    if grants.is_err() && houses == 0 {
        let mut out = vec![format!("Government: {gov}. Every city is the Crown's.")];
        if persons == 0 {
            out.push("No person of note yet.".to_string());
        }
        return out;
    }
    if persons == 0 && houses == 0 {
        return vec!["No person of note yet. Heroes, veterans and great people enter the court.".to_string()];
    }
    Vec::new()
}

/// An outcome (`House Ashledger will break away ...`) as a line after `House Ashledger:
/// Furious.`: without its subject, capitalized (`Will break away ...`).
fn outcome_rest(subject: &str, outcome: &str) -> String {
    match outcome.strip_prefix(&format!("{subject} ")) {
        Some(r) => {
            let mut c = r.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
        None => outcome.to_string(),
    }
}

/// `A`, `A and B`, `A, B and C`.
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The lines that need attention (rule 13; part A.2), each with its button, in this order: a
/// Furious House or commander with its stated outcome, a House that takes no levy, a demand with
/// 10 turns or fewer, a person without a post, the Crown's cities over its limit, the Crown's
/// cities that can be granted; and the badge count (principle 6: a Furious holder, a demand with
/// 10 turns or fewer).
fn attention(v: &Viewer, cv: &CourtView) -> (Vec<Json>, usize) {
    let g = v.g;
    let (persons, houses, grantable) = (&cv.persons, &cv.houses, &cv.grantable);
    let mut out = Vec::new();
    let mut badge = 0;
    let line = |text: String, label: &str, act: Json, kind: &str| crate::jobj! { "kind" => kind, "text" => text, "button" => crate::jobj! { "label" => label, "act" => act } };
    for hv in houses {
        if hv.state == LoyaltyState::Furious {
            badge += 1;
            let outcome = hv.outcome.as_deref().map(|s| core(g, s)).unwrap_or_default();
            let text = format!("{}: Furious. {}", hv.label, outcome_rest(&hv.label, &outcome));
            out.push(line(text, "Show", crate::jobj! { "kind" => "card", "house" => hv.id }, "furious"));
        }
    }
    for pv in persons.iter().filter(|p| !matches!(p.post, Post::Head(_))) {
        if pv.state == LoyaltyState::Furious {
            badge += 1;
            let subject = g.person_subject(pv.id);
            let outcome = pv.outcome.as_deref().map(|s| core(g, s)).unwrap_or_default();
            out.push(line(format!("{subject}: Furious. {}", outcome_rest(&pv.name, &outcome)), "Show", crate::jobj! { "kind" => "card", "person" => pv.id }, "furious"));
        }
    }
    // A House that takes no levy (a Furious one is told above).
    for hv in houses {
        if hv.state != LoyaltyState::Furious {
            if let Some(why) = &hv.levy.reason {
                out.push(line(why.clone(), "Show", crate::jobj! { "kind" => "card", "house" => hv.id }, "levy"));
            }
        }
    }
    for pv in persons {
        if let Some(dm) = &pv.demand {
            if !dm.paused && dm.turns_left <= DEMAND_WARNING_TURNS {
                badge += 1;
                let d = demand_json(v, pv, grantable);
                let subject = g.person_subject(pv.id);
                let text = format!("{subject}: {} {}", d.get("text").and_then(Json::as_str).unwrap_or(""), d.get("turns").and_then(Json::as_str).unwrap_or(""));
                let button = d.get("button").cloned().unwrap_or(Json::Null);
                let mut j = crate::jobj! { "kind" => "demand", "text" => text, "urgent" => true, "button" => button };
                if let Some(n) = d.get("note").and_then(Json::as_str) {
                    j.set("note", n);
                }
                out.push(j);
            }
        }
    }
    for pv in persons {
        if pv.post == Post::None && pv.leader.is_some() {
            if let Some(c) = nearest_grantable(g, v.h, pv.id, grantable) {
                out.push(line(format!("{} has no post.", pv.name), "Grant a city", crate::jobj! { "kind" => "grant", "city" => c, "prefer" => crate::jobj! { "person" => pv.id } }, "no_post"));
            }
        }
    }
    // The Crown's cities over its limit that can be granted, first; then every grantable one.
    let over: Vec<CityId> = cv.crown_over.iter().copied().filter(|c| grantable.contains(c)).collect();
    if let (Some(&first), Some(limit)) = (over.first(), cv.crown_limit) {
        let names = and_list(&over.iter().map(|&c| city_name(g, c)).collect::<Vec<_>>());
        let verb = if over.len() == 1 { "is" } else { "are" };
        let lasting = crown_over_lasting_times100(g, v.h, &cv.crown_over).max(cv.crown_over_cost_times100);
        let cost = if lasting > 0 { format!(": +{} gold a turn", gold(lasting)) } else { String::new() };
        let text = format!("{names} {verb} over the Crown's limit of {limit}{cost}.");
        out.push(line(text, &format!("Grant {}", city_name(g, first)), crate::jobj! { "kind" => "grant", "city" => first }, "crown_over"));
    }
    if !grantable.is_empty() {
        let n = grantable.len();
        let text = if n == 1 { "1 city of the Crown can be granted.".to_string() } else { format!("{n} cities of the Crown can be granted.") };
        out.push(line(text, "Show", crate::jobj! { "kind" => "crown" }, "grantable"));
    }
    (out, badge)
}

/// The count on the Court button (principle 6): the view's `court_attention`. 0 for a player
/// without persons.
pub fn attention_count(v: &Viewer) -> usize {
    let g = v.g;
    if !g.player_has_persons(v.h) {
        return 0;
    }
    let court = g.court_view(v.h);
    let mut n = court.houses.iter().filter(|hv| hv.state == LoyaltyState::Furious).count();
    n += court.persons.iter().filter(|p| !matches!(p.post, Post::Head(_)) && p.state == LoyaltyState::Furious).count();
    n += court.persons.iter().filter(|p| p.demand.as_ref().is_some_and(|dm| !dm.paused && dm.turns_left <= DEMAND_WARNING_TURNS)).count();
    n
}

/// The keywords of this system (rule 13; part J). The loyalty states are the game's attitude
/// words, no keywords: a state mark's tooltip is the attitude's own help (attitudes.jsonc).
const KEYWORDS: &[&str] = &["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_TRIBUTE", "KEYWORD_LEVY", "KEYWORD_SWORN", "KEYWORD_CLAIMANT"];

fn keywords(d: &GameData, names: &[&str]) -> Vec<KeywordType> {
    names.iter().filter_map(|n| d.ty::<KeywordType>(n)).collect()
}

/// `court`: the Court screen (docs/PERSONS.md, rule 13; "UI", part A): the realm.
pub fn court(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let cv = g.court_view(h);
    let gov = government(g, h);
    let has = g.player_has_persons(h);
    let header = vec![format!("Government: {}.", gov.map(|c| names::display(d, c).into_owned()).unwrap_or_else(|| "none".into()))];
    // A House's size (rule 14): the realm's threshold.
    let realm = (!cv.houses.is_empty()).then(|| format!("Realm: {}. A House holds {} without a grievance.", cities_text(cv.realm_cities), cv.house_size_threshold));
    let (attention, badge) = if has { attention(v, &cv) } else { (Vec::new(), 0) };
    let houses: Vec<Json> = cv.houses.iter().map(|hv| house_json(v, hv, &cv.persons, &cv.grantable)).collect();
    let persons: Vec<Json> = cv.persons.iter().filter(|p| !matches!(p.post, Post::Head(_))).map(|pv| person_json(v, pv, &cv.grantable)).collect();
    let steps: Vec<i32> = (0..=ffh_core::persons::TRIBUTE_MAX).step_by(ffh_core::persons::TRIBUTE_STEP as usize).collect();
    crate::jobj! {
        "government" => Json::opt_idx(gov),
        "header" => header,
        "crown_line" => crown_line(g, h, &cv),
        "realm" => realm,
        "tribute_default" => has.then_some(cv.tribute_default),
        "tribute_steps" => steps,
        "grants" => cv.grants.err(),
        "empty" => empty_lines(g, h, cv.grants, cv.persons.len(), cv.houses.len()),
        "attention" => Json::Arr(attention),
        "badge" => badge,
        "crown" => Json::Arr(crown_cities_json(g, h, &cv)),
        "houses" => Json::Arr(houses),
        "persons" => Json::Arr(persons),
        "pool_left" => cv.pool_left,
        "crown_limit" => cv.crown_limit,
        "reminders" => crate::view::keywords::reminders(d, keywords(d, KEYWORDS)),
    }
}

// ---------------------------------------------------------------------------------------------
// The grant list
// ---------------------------------------------------------------------------------------------

/// `{house}`, `{person}` or `{unit}`: the protocol's form of a grantee (`grant_city`'s members).
pub fn grant_to_json(to: GrantTo) -> Json {
    match to {
        GrantTo::House(x) => crate::jobj! { "house" => x },
        GrantTo::Person(x) => crate::jobj! { "person" => x },
        GrantTo::GreatPerson(u) => crate::jobj! { "unit" => u },
    }
}

/// Whether a preview line moves loyalty (`Granted Torrin: +1 (30 turns)`, `Tribute 20%: -1`):
/// it ends in an amount, with or without its turns.
fn moves_loyalty(text: &str) -> bool {
    let Some(i) = text.rfind(": ") else { return false };
    let rest = text[i + 2..].trim_end_matches('.');
    let n = rest.split(' ').next().unwrap_or("");
    n.trim_start_matches(['+', '-']).parse::<i32>().is_ok()
}

/// The preview's lines grouped by subject (each subject's state now, its tag when it claims the
/// city, its lines, the state after when it changes).
/// `own`: the grantee's own subject (its House's label, or the founder's subject): its block is
/// marked `self`, and the client folds it into the candidate's card, whose header names it; it
/// keeps only the lines that move loyalty (the card's own lines say the rest).
fn effects_json(g: &Game, c: CityId, lines: &[PreviewLine], own: &[String]) -> Vec<Json> {
    let h = g.cities[c].owner;
    let city = city_name(g, c);
    // The subjects' states now and their claims, by the subject's label.
    let mut known: Vec<(String, LoyaltyState, Option<String>)> = Vec::new();
    let claims = g.city_claimants(c);
    for &hh in &g.players[h].houses {
        if g.houses.contains(hh) {
            let tag = claims.contains(&Claimant::House(hh)).then(|| "claimant".to_string());
            known.push((g.house_label(hh), g.house_state(hh), tag));
        }
    }
    for &x in &g.players[h].persons {
        if g.persons.contains(x) && g.person_house(x).is_none() {
            let tag = claims.contains(&Claimant::Commander(x)).then(|| format!("took {city}"));
            known.push((g.person_subject(x), g.person_state(x), tag));
        }
    }
    let mut out: Vec<(String, Vec<String>, Option<LoyaltyState>)> = Vec::new();
    for l in lines {
        let mine = own.iter().any(|o| *o == l.subject);
        let text = format!("{}.", core(g, &l.text));
        let entry = match out.iter_mut().position(|(s, _, _)| *s == l.subject) {
            Some(i) => &mut out[i],
            None => {
                out.push((l.subject.clone(), Vec::new(), None));
                out.last_mut().expect("pushed")
            }
        };
        if l.state_after.is_some() {
            entry.2 = l.state_after;
        }
        if !mine || moves_loyalty(&l.text) {
            entry.1.push(text);
        }
    }
    out.into_iter()
        .map(|(subject, lines, after)| {
            let k = known.iter().find(|(s, _, _)| *s == subject);
            crate::jobj! {
                "subject" => subject.as_str(),
                "state" => k.map(|k| state_name(k.1)),
                "tag" => k.and_then(|k| k.2.clone()),
                "lines" => lines,
                "becomes" => after.map(|s| format!("Becomes {}.", s.name())),
                "state_after" => after.map(state_name),
                "self" => own.iter().any(|o| *o == subject),
            }
        })
        .collect()
}

/// A grant's maintenance line: `Your maintenance: -3.2 gold a turn.` (the rules' maintenance of
/// the owner's cities now and with the city a House's, inflation and the Crown's limit included),
/// at the city's lasting rate: a city that pays none now (disorder) is counted at what it pays once
/// that ends (the list's head says it pays none now).
fn maintenance_line(g: &Game, c: CityId) -> String {
    let m = g.grant_maintenance(c);
    let change = if pays_none_now(g, c).is_some() { m.gold_change_times100 - lasting_maintenance_times100(g, c) } else { m.gold_change_times100 };
    if change == 0 {
        "Your maintenance: no change.".to_string()
    } else {
        format!("Your maintenance: {} gold a turn.", gold(change))
    }
}

/// The Crown after granting city `c` (rule 14), while it is over its limit: `The Crown after the
/// grant: 3 cities, within its limit of 3.` or `... 4 cities, 1 over its limit of 3.`; `None` when
/// the Crown is within its limit now, or the city does not count.
fn crown_after_line(g: &Game, h: PlayerId, c: CityId) -> Option<String> {
    let limit = g.player_crown_limit(h)?;
    let crown = g.player_crown_cities(h);
    if !crown.contains(&c) || g.player_crown_over(h).is_empty() {
        return None;
    }
    let n = crown.len() as i32 - 1;
    Some(if n > limit {
        format!("The Crown after the grant: {}, {} over its limit of {limit}.", cities_text(n), n - limit)
    } else {
        format!("The Crown after the grant: {}, within its limit of {limit}.", cities_text(n))
    })
}

/// One candidate of the grant list: who, what the grant does (`lines`: the city moves to the
/// House's realm, the maintenance, the Crown's limit, the tribute, the founder leaving the map),
/// and its loyalty changes (`effects`).
fn candidate_json(v: &Viewer, c: CityId, cand: &GrantCandidate) -> Json {
    let g = v.g;
    let d = g.d;
    let city = city_name(g, c);
    let mut lines: Vec<String> = Vec::new();
    let (kind, title, house_label, state, color, leader, unit_type, person) = match cand.to {
        GrantTo::House(hh) => {
            let hv = g.house_view(hh).expect("a standing House");
            ("house", hv.label.clone(), hv.label.clone(), Some(hv.state), Some(hv.color as i64), hv.leader, None, hv.head)
        }
        GrantTo::Person(x) => {
            let label = format!("House {}", g.house_new_name(v.h));
            ("person", format!("Found a House: {}", g.persons[x].name), label, Some(g.person_state(x)), None, g.persons[x].leader, g.person_unit(x).map(|u| g.units[u].unit_type), Some(x))
        }
        GrantTo::GreatPerson(u) => {
            let label = format!("House {}", g.house_new_name(v.h));
            let unit = names::display(d, g.units[u].unit_type).into_owned();
            let at = g.unit_plot_opt(u).and_then(|q| g.map.plots[q].plot_city).filter(|&k| g.cities.contains(k)).map(|k| format!(" in {}", city_name(g, k))).unwrap_or_default();
            ("great_person", format!("Found a House: {unit}{at}"), label, None, None, None, Some(g.units[u].unit_type), None)
        }
    };
    // What the grant does: the city leaves the Crown for the House's player.
    lines.push(format!("Moves to {house_label}'s realm: its production and commerce are the House's."));
    lines.push(maintenance_line(g, c));
    if let Some(l) = crown_after_line(g, v.h, c) {
        lines.push(l);
    }
    let tribute = match cand.to {
        GrantTo::House(hh) => g.houses[hh].tribute,
        _ => g.players[v.h].house_tribute,
    };
    lines.push(format!("Tribute from it at {tribute}%."));
    match cand.to {
        GrantTo::Person(x) => {
            let name = &g.persons[x].name;
            if let Some(a) = g.person_army(x) {
                lines.push(format!("Command of the {} ends.", g.armies[a].name));
            }
            lines.push(format!("{name} leaves the map and leads {house_label}."));
        }
        GrantTo::GreatPerson(u) => {
            let unit = names::display(d, g.units[u].unit_type).into_owned();
            lines.push(format!("The {unit} is consumed. A person of note is drawn and leads {house_label}."));
        }
        GrantTo::House(_) => {}
    }
    let own_subjects: Vec<String> = match cand.to {
        GrantTo::House(_) => vec![house_label.clone()],
        GrantTo::Person(x) => vec![g.person_subject(x), house_label.clone()],
        GrantTo::GreatPerson(_) => vec![house_label.clone()],
    };
    let confirm = format!("Grant {city} to {house_label}? This cannot be undone.");
    let button = match cand.to {
        GrantTo::House(_) => format!("Grant to {house_label}"),
        _ => format!("Found {house_label}"),
    };
    crate::jobj! {
        "to" => grant_to_json(cand.to),
        "kind" => kind,
        "title" => title,
        "house" => house_label.as_str(),
        "claimant" => cand.claimant,
        "state" => state.map(state_name),
        "color" => color,
        "leader" => Json::opt_idx(leader),
        "unit_type" => Json::opt_idx(unit_type),
        "person" => person,
        "lines" => lines,
        "effects" => Json::Arr(effects_json(g, c, &cand.lines, &own_subjects)),
        "refusal" => cand.refusal,
        "button" => button,
        "confirm" => confirm,
    }
}

/// `grant {city}`: the grant list of one of the human's cities (rule 13; part B). A city that
/// cannot be granted has its `refusal` and no candidates. Heroes are no candidates (a hero cannot
/// head a House).
pub fn grant(v: &Viewer, c: CityId) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    match g.cities.get(c) {
        None => return Err(QueryErr::Gone),
        Some(city) if city.owner != v.h => return Err(QueryErr::NotOwner),
        Some(_) => {}
    }
    let name = city_name(g, c);
    let refusal = g.player_check_grant_city(v.h, c).err();
    // What the Crown pays for it (the city's own maintenance at its lasting rate; a grant moves it
    // to the House), and whether it pays none now.
    let mut baseline = format!("Held by the Crown: its maintenance {} gold a turn.", gold(lasting_maintenance_times100(g, c)));
    if let Some(why) = pays_none_now(g, c) {
        baseline.push(' ');
        baseline.push_str(&why);
    }
    let mut candidates: Vec<(u8, Json)> = Vec::new();
    if refusal.is_none() {
        for cand in g.grant_candidates(v.h, c) {
            if matches!(cand.to, GrantTo::Person(x) if g.persons[x].leader.is_none()) {
                continue;
            }
            // Claimants first, then the other Houses, then persons without a House, then great
            // people (the core lists Houses, persons, great people).
            let rank = match (cand.claimant, cand.to) {
                (true, _) => 0,
                (false, GrantTo::House(_)) => 1,
                (false, GrantTo::Person(_)) => 2,
                (false, GrantTo::GreatPerson(_)) => 3,
            };
            candidates.push((rank, candidate_json(v, c, &cand)));
        }
    }
    candidates.sort_by_key(|(r, _)| *r);
    let mut kws = vec!["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_TRIBUTE"];
    if candidates.iter().any(|(_, j)| j.get("claimant").and_then(Json::as_bool) == Some(true)) {
        kws.push("KEYWORD_CLAIMANT");
    }
    let cv = g.court_view(v.h);
    Ok(crate::jobj! {
        "city" => c,
        "name" => name.as_str(),
        "x" => g.cities[c].x,
        "y" => g.cities[c].y,
        "title" => format!("Grant {name}"),
        "note" => "A grant is for good.",
        "baseline" => baseline,
        "crown_line" => crown_line(g, v.h, &cv),
        "refusal" => refusal,
        "none" => (refusal.is_none() && candidates.is_empty()).then_some("No House, person or great person to grant it to."),
        "candidates" => Json::Arr(candidates.into_iter().map(|(_, j)| j).collect()),
        "reminders" => crate::view::keywords::reminders(d, keywords(d, &kws)),
    })
}

// ---------------------------------------------------------------------------------------------
// The other screens: the levy (the Armies screen) and a Crown city's Governance section
// ---------------------------------------------------------------------------------------------

/// A requisition candidate's House (the levy, rule 6; the Armies screen's *Requisition from*):
/// `{id, label, colour, state, turns, takes_levy, text}`, `text` `Takes levies. Each levy: -1 (20
/// turns).` or the rules' reason it takes none (`House Arne is Annoyed. It takes no levy.`);
/// null for a city of no House.
pub fn levy_house(g: &Game, c: CityId) -> Json {
    let Some(h) = g.city_house(c) else { return Json::Null };
    let refusal = g.house_levy_refusal(h);
    let status = levy_status(refusal.as_deref());
    crate::jobj! {
        "id" => h,
        "label" => g.house_label(h),
        "colour" => g.houses[h].color as i32,
        "state" => g.house_state(h).name(),
        "turns" => g.houses[h].standing.countdown.map(|end| (end - g.game_turn()).max(0)),
        "takes_levy" => refusal.is_none(),
        "text" => status.get("text").cloned().unwrap_or(Json::Null),
    }
}

/// The city screen's Governance line for one of the human's cities (rule 13; part C): every city
/// of the human's is the Crown's (a House's city is its own player's), so `crown: {grantable,
/// refusal (why it cannot be granted, else null), over (`Over the Crown's limit of 3.` or
/// null)}` and `house: null`. Nothing for a human without Houses.
pub fn governance_fields(v: &Viewer, c: CityId, j: &mut Json) {
    let g = v.g;
    if !g.player_has_persons(v.h) || g.cities[c].owner != v.h {
        return;
    }
    let why = g.player_check_grant_city(v.h, c).err();
    let over = g.player_crown_limit(v.h).filter(|_| g.player_crown_over(v.h).contains(&c)).map(|limit| format!("Over the Crown's limit of {limit}."));
    j.set("house", Json::Null);
    j.set("crown", crate::jobj! { "grantable" => why.is_none(), "refusal" => why, "over" => over });
}

// ---------------------------------------------------------------------------------------------
// The commands' checks (the rules' own reasons; the rules have no separate check for these two,
// so their apply's checks are read here with the same words)
// ---------------------------------------------------------------------------------------------

/// `player_set_commander`'s checks, without applying it.
pub fn check_set_commander(g: &Game, p: PlayerId, a: ArmyId, x: PersonId) -> Result<(), &'static str> {
    if !g.armies.contains(a) || g.armies[a].owner != p {
        return Err("Not an army of yours.");
    }
    if g.armies[a].domain != ffh_core::army::ArmyDomain::Land {
        return Err("A fleet has no commander.");
    }
    if !g.persons.contains(x) || g.persons[x].owner != p {
        return Err("Not a person of yours.");
    }
    if g.person_house(x).is_some() {
        return Err("A House's head does not command.");
    }
    if !g.army_persons_with(a).contains(&x) {
        return Err("Not with this army.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_the_screens_write_them() {
        assert_eq!(gold(620), "6.2");
        assert_eq!(gold(900), "9");
        assert_eq!(gold(314), "3.1");
        assert_eq!(gold(-314), "-3.1");
        assert_eq!(gold(0), "0");
        assert_eq!(signed(3), "+3");
        assert_eq!(signed(-3), "-3");
        assert_eq!(turns(1), "1 turn");
    }
}
