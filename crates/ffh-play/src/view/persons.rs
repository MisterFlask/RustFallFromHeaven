//! Houses as the human sees them (docs/PERSONS.md, "The third build", on "The second build", rule
//! 13; "UI": parts A, B and J): the Court screen (`court`: the realm, the Crown and its limit, a
//! card per House with its leader, request, tribute selector, levy and Free), a city's grant list
//! (`grant {city}`: the Houses, or with none the founding of one), the count of what needs
//! attention (the view's `court_attention`, the menu's badge), a requisition candidate's House line
//! (the levy, for the Armies screen) and a Crown city's Governance line (the city screen).
//!
//! Every line the page shows about the rules is made here, from the rules' own read API
//! (`ffh_core::persons`: `court_view`, `house_view`, `grant_candidates`, the label functions); the
//! page lays the lines out and decides nothing. Pure: reads `&Game` only.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{CourtView, DemandKind, GrantCandidate, GrantTo, HouseOrigin, HouseView, LoyaltyPart, LoyaltyState, PreviewLine, DEMAND_REFUSED, DEMAND_WARNING_TURNS};
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

/// A change in hundredths of gold as the screens write it, signed as a change in what the player
/// pays: `+7`, `-3.2` (+ costs more, - costs less).
fn gold_change(times100: i32) -> String {
    let g = gold(times100);
    if times100 > 0 && g != "0" {
        format!("+{g}")
    } else {
        g
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

/// `Request refused: -3 (41 turns)`; a standing part without turns: `State religion opposed: -2`;
/// a part whose label is a sentence (a contract's floor, the third build, rule 6): `Contract until
/// turn 160: at least Annoyed (+3).`, or with nothing to add `Contract until turn 160: at least
/// Annoyed.`
fn part_text(p: &LoyaltyPart) -> String {
    if p.label.contains(": ") {
        return if p.amount == 0 { format!("{}.", p.label) } else { format!("{} ({}).", p.label, signed(p.amount)) };
    }
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
// A House's request
// ---------------------------------------------------------------------------------------------

/// The request line of House `hv` with its one button (docs/PERSONS.md, "UI", the demand line,
/// now the request: "The third build", rule 2), or between requests `No request. Next in 14
/// turns.`
fn request_json(v: &Viewer, hv: &HouseView) -> Json {
    let g = v.g;
    let h = v.h;
    let team = g.players[h].team;
    let Some(dm) = &hv.demand else {
        let text = if hv.demand_rest_turns > 0 { format!("No request. Next in {}.", turns(hv.demand_rest_turns)) } else { "No request.".to_string() };
        return crate::jobj! { "kind" => Json::Null, "text" => text, "turns_left" => Json::Null, "urgent" => false, "note" => Json::Null, "button" => Json::Null };
    };
    let what = core(g, &dm.text);
    let urgent = dm.turns_left <= DEMAND_WARNING_TURNS;
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
        DemandKind::GrantCity(plot) => {
            let c = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c));
            match c {
                Some(c) if g.cities[c].owner == h => match g.player_check_grant_city(h, c) {
                    Ok(()) => ("grant_city", None, crate::jobj! { "label" => format!("Grant {}", city_name(g, c)), "act" => crate::jobj! { "kind" => "grant", "city" => c, "prefer" => crate::jobj! { "house" => hv.id } } }),
                    Err(why) => ("grant_city", Some(why.to_string()), Json::Null),
                },
                _ => ("grant_city", owner_note(plot), show(plot)),
            }
        }
        DemandKind::TakeCity(plot) => ("take_city", owner_note(plot), show(plot)),
        DemandKind::War(q) => ("war", None, crate::jobj! { "label" => "Diplomacy", "act" => crate::jobj! { "kind" => "diplomacy", "player" => q } }),
        DemandKind::AdoptReligion => ("adopt_religion", None, crate::jobj! { "label" => "Religion", "act" => crate::jobj! { "kind" => "religion" } }),
        DemandKind::SpreadReligion(plot) => ("spread_religion", None, show(plot)),
    };
    let (lapse, lapse_state) = request_lapse(g, hv);
    crate::jobj! {
        "kind" => kind,
        "text" => format!("Request: {what}"),
        "turns_left" => dm.turns_left,
        "turns" => format!("{}.", turns(dm.turns_left)),
        "lapse" => lapse,
        "lapse_state" => lapse_state,
        // (the third build, rule 5: Furious is an ultimatum; the client writes `, becomes ✖
        // Furious: an ultimatum.`)
        "lapse_then" => (lapse_state == Some("Furious")).then_some("an ultimatum"),
        "urgent" => urgent,
        "note" => note,
        "button" => button,
    }
}

/// What a request's term lapsing does (the rules' refusal, `DEMAND_REFUSED`): `At its term: -3
/// (60 turns)`, and the state it leaves the House in when that changes (the rules' state of its
/// loyalty less the refusal's amount, a contract's floor included; the client writes `, becomes ✖
/// Furious: an ultimatum.`), else null (`.`).
fn request_lapse(g: &Game, hv: &HouseView) -> (String, Option<&'static str>) {
    let (amount, t) = DEMAND_REFUSED;
    let after = LoyaltyState::of(g.house_loyalty_floored(hv.id, g.house_loyalty_raw(hv.id) + amount));
    (format!("At its term: {} ({})", signed(amount), turns(t)), (after != hv.state).then(|| state_name(after)))
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
/// (hundredths of gold a turn, inflation included): the rules' `player_crown_over_cost_times100`,
/// with a city that pays none now counted at what it pays once that ends; `without`: were that city
/// granted.
fn crown_over_lasting_times100(g: &Game, h: PlayerId, without: Option<CityId>) -> i32 {
    g.player_crown_over_cost_without_times100(h, without, true)
}

/// The Crown's limit as the rules state it (docs/PERSONS.md, rule 14): `The Crown holds 5
/// cities, 2 over its limit of 3.` or `The Crown holds 3 cities; its limit is 3.`, with
/// `Settlements are not counted.` for a civilization with settlements; `over`: beyond the limit
/// (the caution colour). What the cities over it cost is the line that needs attention's (with its
/// button). `None` where there is no limit (no city can be granted).
fn crown_line(g: &Game, h: PlayerId, cv: &CourtView) -> Option<Json> {
    let limit = cv.crown_limit?;
    let n = cv.crown_cities.len() as i32;
    let over = cv.crown_over.len();
    let mut text = if over > 0 {
        format!("The Crown holds {}, {over} over its limit of {limit}.", cities_text(n))
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
        HouseOrigin::Split => format!("Split off on turn {}.", hv.founded_turn),
    }
}

/// `12 gold, 8 research`.
fn gold_research(gold: i32, research: i32) -> String {
    format!("{gold} gold, {research} research")
}

/// A House's tribute (rule 4) and its selector: `pct` (the rate set), `now` (the rate in force),
/// `turns` (until the rate set is in force, 0 when it is); `line` (`Tribute 30%: 12 gold, 8
/// research last turn.`, `Tribute 30%. Not paid yet.`; while the rate in force moves: `Tribute
/// 30%, now 10%: reaches 30% in 10 turns. Paid last turn: 4 gold, 6 research.`); `rates`: each rate
/// it may be set to (0 to 50 by 10), from the rules' read (`TributeView::at`), once in force:
/// `{pct, current (the rate set), in_force (the rate in force), turns (until it would be in force
/// were it set now), when (`In force.`, `In force in 15 turns.`), estimate (what it pays at the
/// House's commerce now), part (its tribute part), part_text, loyalty (its total once in force),
/// loyalty_text, state, changes (its state differs from now)}`.
fn tribute_json(g: &Game, hv: &HouseView) -> Json {
    let t = &hv.tribute;
    let paid = gold_research(t.last_gold, t.last_research);
    let line = if t.pct == t.now {
        if t.last_turn < 0 {
            format!("Tribute {}%. Not paid yet.", t.pct)
        } else if t.last_turn >= g.game_turn() - 1 {
            format!("Tribute {}%: {paid} last turn.", t.pct)
        } else {
            format!("Tribute {}%: {paid} on turn {}.", t.pct, t.last_turn)
        }
    } else {
        let moving = format!("Tribute {}%, now {}%: reaches {}% in {}.", t.pct, t.now, t.pct, turns(t.turns));
        if t.last_turn < 0 {
            format!("{moving} Not paid yet.")
        } else if t.last_turn >= g.game_turn() - 1 {
            format!("{moving} Paid last turn: {paid}.")
        } else {
            format!("{moving} Paid on turn {}: {paid}.", t.last_turn)
        }
    };
    let rates: Vec<Json> = t
        .at
        .iter()
        .map(|r| {
            crate::jobj! {
                "pct" => r.pct,
                "current" => r.pct == t.pct,
                "in_force" => r.pct == t.now,
                "turns" => r.turns,
                "when" => if r.turns == 0 { "In force.".to_string() } else { format!("In force in {}.", turns(r.turns)) },
                "estimate" => format!("Pays {}.", gold_research(r.gold, r.research)),
                "part" => r.part,
                "part_text" => format!("Tribute: {}", signed(r.part)),
                "loyalty" => r.loyalty,
                "loyalty_text" => format!("Loyalty: {}", signed(r.loyalty)),
                "state" => state_name(r.state),
                "changes" => r.state != hv.state,
                // A rate a contract's bound refuses (the third build, rule 6), with the reason.
                "barred" => if r.barred { g.player_check_house_tribute(hv.owner, hv.id, r.pct).err() } else { None },
            }
        })
        .collect();
    crate::jobj! { "pct" => t.pct, "now" => t.now, "turns" => t.turns, "line" => line, "rates" => Json::Arr(rates) }
}

/// `Takes levies. Each levy: -1 (20 turns).` (rule 6, the rules' `LEVIED`).
fn levy_takes_text() -> String {
    let (amount, t) = ffh_core::persons::LEVIED;
    format!("Takes levies. Each levy: {} ({}).", signed(amount), turns(t))
}

/// How many levies a House at loyalty `loyalty` takes before it takes none, from the rules'
/// numbers (each levy's `LEVIED` amount, the rules' state of the loyalty after them):
/// `Annoyed after 2 levies.`; `None` when it takes none now (or a levy moves no loyalty).
fn levies_until_none(loyalty: i32) -> Option<String> {
    let (amount, _) = ffh_core::persons::LEVIED;
    if amount >= 0 || LoyaltyState::of(loyalty).withholds() {
        return None;
    }
    let mut k = 1;
    while !LoyaltyState::of(loyalty + k * amount).withholds() {
        k += 1;
    }
    let state = LoyaltyState::of(loyalty + k * amount);
    Some(format!("{} after {k} {}.", state.name(), if k == 1 { "levy" } else { "levies" }))
}

/// A House's levy status (rule 6): `{takes, text}`: `Takes levies. Each levy: -1 (20 turns).
/// Annoyed after 2 levies.` or the rules' reason it takes none (`House Arne is Annoyed. It takes
/// no levy.`).
fn levy_status(reason: Option<&str>, loyalty: i32) -> Json {
    let text = match reason {
        Some(why) => why.to_string(),
        None => match levies_until_none(loyalty) {
            Some(until) => format!("{} {until}", levy_takes_text()),
            None => levy_takes_text(),
        },
    };
    crate::jobj! { "takes" => reason.is_none(), "text" => text }
}

/// A House's card (rule 13; "The third build", rule 1: a House carries what its head carried):
/// its leader (portrait, name, title), its attitude and, Furious, the outcome of its ultimatum
/// (rule 5), its contract (rule 6: `Contract until turn 160: no tribute above 10%.`), its split
/// line (rule 4: `Holds 7 cities: 5% a turn to split.`), its request, its loyalty parts, its
/// tribute and the selector, its levy, its cities, its price to secede, Free; and opened, its
/// record, its leader's ambition and backstory.
fn house_json(v: &Viewer, hv: &HouseView) -> Json {
    let g = v.g;
    let (parts, total) = parts_json(&hv.parts, hv.loyalty);
    let (_, held) = g.house_cities_held(hv.id);
    // Each city with the contest for it when an army of yours stands in it (rule 7).
    let cities: Vec<Json> = hv
        .cities
        .iter()
        .filter(|&&c| g.cities.contains(c))
        .map(|&c| city_ref(g, c).with("held", held.contains(&c)).with("hold", g.city_hold_line(c, "it").map(|l| core(g, &l))).with("seat", hv.seat == Some(c)))
        .collect();
    let contests = g.house_hold_text(hv.id);
    // Its price when it would accept an offer (Cautious or worse) is in the card's body; a House
    // that refuses any says so under Record.
    let price = hv.bribe_price.map(|p| format!("Price to secede: {p} gold."));
    let price_refusal = hv.bribe_price.is_none().then_some("Refuses any offer to secede.");
    let record: Vec<Json> = hv.deeds.iter().rev().map(|dd| Json::from(format!("Turn {}: {}", dd.turn, core(g, &dd.text)))).collect();
    crate::jobj! {
        "id" => hv.id,
        "player" => hv.player,
        "label" => hv.label.as_str(),
        "civilization" => core(g, &hv.civilization_name),
        "leader" => Json::opt_idx(hv.leader),
        "leader_name" => hv.leader_name.as_str(),
        "title" => hv.leader_title.clone(),
        "color" => hv.color as i64,
        "state" => state_name(hv.state),
        "outcome" => hv.outcome.as_deref().map(|s| core(g, s)),
        "held" => (hv.outcome.is_some() && !contests.is_empty()).then(|| core(g, contests.trim_start())),
        // the third build, part 2A
        "contract" => hv.contract.as_ref().map(|c| core(g, &c.line)),
        "split" => hv.split_line.clone(),
        "since" => since_text(hv),
        "request" => request_json(v, hv),
        "cities" => Json::Arr(cities),
        "tribute" => tribute_json(g, hv),
        "levy" => levy_status(hv.levy.reason.as_deref(), hv.loyalty),
        "price" => price,
        "price_refusal" => price_refusal,
        "free" => crate::jobj! { "ok" => hv.free.is_ok(), "refusal" => hv.free.err(), "confirm" => format!("Free {}? It is no longer your vassal, at peace with you. This cannot be undone.", hv.label) },
        "parts" => parts,
        "total" => total,
        "ambition" => format!("Ambition: {}.", hv.ambition.name()),
        "record" => Json::Arr(record),
        "backstory" => hv.leader.map(|l| names::text(g.d, &g.d.leader_heads[l].base.civilopedia).to_string()).filter(|s| !s.is_empty() && !s.starts_with("TXT_KEY")),
    }
}

// ---------------------------------------------------------------------------------------------
// The Court
// ---------------------------------------------------------------------------------------------

/// The heads of the screen when there is nothing to list (principle 6): why, in one or two lines.
fn empty_lines(g: &Game, h: PlayerId, grants: Result<(), &'static str>, houses: usize) -> Vec<String> {
    let gov = government(g, h).map(|c| names::display(g.d, c).into_owned()).unwrap_or_else(|| "none".into());
    if !g.player_has_houses(h) {
        return vec!["This civilization has no Houses.".to_string()];
    }
    if grants.is_err() && houses == 0 {
        return vec![format!("Government: {gov}. Every city is the Crown's.")];
    }
    Vec::new()
}

/// The realm's founding line while it has no House (rule 3): `No House yet. A grant of a Crown city
/// founds one. A new House: its leader is drawn from the Bannor's minor leaders.`, or why none can
/// be founded (`No leader is free to found a House.`); `{text, ok}`. Null with a House, or where no
/// city can be granted.
fn founding_json(cv: &CourtView) -> Json {
    if !cv.houses.is_empty() || cv.grants.is_err() {
        return Json::Null;
    }
    match (&cv.founding, &cv.founding_line) {
        (Ok(()), Some(line)) => crate::jobj! { "ok" => true, "text" => format!("No House yet. A grant of a Crown city founds one. {line}") },
        (Err(why), _) => crate::jobj! { "ok" => false, "text" => format!("No House yet. {why}") },
        (Ok(()), None) => Json::Null,
    }
}

/// An outcome (`House Ashledger issues an ultimatum ...`) as a line after `House Ashledger:
/// Furious.`: without its subject, capitalized (`Issues an ultimatum ...`).
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
/// Furious House with its stated outcome, a House that takes no levy, a request with 10 turns or
/// fewer, the Crown's cities over its limit, the Crown's cities that can be granted; and the badge
/// count (principle 6: a Furious House, a request with 10 turns or fewer).
fn attention(v: &Viewer, cv: &CourtView) -> (Vec<Json>, usize) {
    let g = v.g;
    let (houses, grantable) = (&cv.houses, &cv.grantable);
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
    // A House that takes no levy (a Furious one is told above).
    for hv in houses {
        if hv.state != LoyaltyState::Furious {
            if let Some(why) = &hv.levy.reason {
                out.push(line(why.clone(), "Show", crate::jobj! { "kind" => "card", "house" => hv.id }, "levy"));
            }
        }
    }
    for hv in houses {
        if hv.demand.as_ref().is_some_and(|dm| dm.turns_left <= DEMAND_WARNING_TURNS) {
            badge += 1;
            let d = request_json(v, hv);
            let text = format!("{}: {} {}", hv.label, d.get("text").and_then(Json::as_str).unwrap_or(""), d.get("turns").and_then(Json::as_str).unwrap_or(""));
            let button = d.get("button").cloned().unwrap_or(Json::Null);
            // What the term's lapse does, as on the card (`At its term: -3 (60 turns)`, and the
            // state after when it changes).
            let mut j = crate::jobj! {
                "kind" => "request", "text" => text, "urgent" => true, "button" => button,
                "lapse" => d.get("lapse").cloned().unwrap_or(Json::Null), "lapse_state" => d.get("lapse_state").cloned().unwrap_or(Json::Null),
            };
            if let Some(n) = d.get("note").and_then(Json::as_str) {
                j.set("note", n);
            }
            out.push(j);
        }
    }
    // The Crown's cities over its limit that can be granted, first; then every grantable one.
    let over: Vec<CityId> = cv.crown_over.iter().copied().filter(|c| grantable.contains(c)).collect();
    if let (Some(&first), Some(limit)) = (over.first(), cv.crown_limit) {
        let names = and_list(&over.iter().map(|&c| city_name(g, c)).collect::<Vec<_>>());
        let verb = if over.len() == 1 { "is" } else { "are" };
        // At the cities' lasting rate: a city in disorder pays none now, said once, here.
        let lasting = crown_over_lasting_times100(g, v.h, None);
        let cost = if gold(lasting) != "0" { format!(": maintenance {} gold a turn", gold_change(lasting)) } else { String::new() };
        let mut text = format!("{names} {verb} over the Crown's limit of {limit}{cost}.");
        for &c in &cv.crown_over {
            if let Some(why) = pays_none_now(g, c) {
                text.push(' ');
                // One city: `It pays none ...`; several: the city's name.
                text.push_str(&if cv.crown_over.len() == 1 { why.replacen(&city_name(g, c), "It", 1) } else { why });
            }
        }
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
/// without Houses.
pub fn attention_count(v: &Viewer) -> usize {
    let g = v.g;
    if !g.player_has_houses(v.h) {
        return 0;
    }
    let court = g.court_view(v.h);
    let mut n = court.houses.iter().filter(|hv| hv.state == LoyaltyState::Furious).count();
    n += court.houses.iter().filter(|hv| hv.demand.as_ref().is_some_and(|dm| dm.turns_left <= DEMAND_WARNING_TURNS)).count();
    n
}

/// The keywords of this system (rule 13; part J). The loyalty states are the game's attitude
/// words, no keywords: a state mark's tooltip is the attitude's own help (attitudes.jsonc).
const KEYWORDS: &[&str] = &["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_REQUEST", "KEYWORD_ULTIMATUM", "KEYWORD_TRIBUTE", "KEYWORD_LEVY", "KEYWORD_CLAIMANT"];

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
    let has = g.player_has_houses(h);
    let header = vec![format!("Government: {}.", gov.map(|c| names::display(d, c).into_owned()).unwrap_or_else(|| "none".into()))];
    let (attention, badge) = if has { attention(v, &cv) } else { (Vec::new(), 0) };
    let houses: Vec<Json> = cv.houses.iter().map(|hv| house_json(v, hv)).collect();
    let steps: Vec<i32> = (0..=ffh_core::persons::TRIBUTE_MAX).step_by(ffh_core::persons::TRIBUTE_STEP as usize).collect();
    crate::jobj! {
        "government" => Json::opt_idx(gov),
        "header" => header,
        "crown_line" => crown_line(g, h, &cv),
        "founding" => founding_json(&cv),
        "tribute_default" => has.then_some(cv.tribute_default),
        "tribute_steps" => steps,
        "grants" => cv.grants.err(),
        "empty" => empty_lines(g, h, cv.grants, cv.houses.len()),
        "attention" => Json::Arr(attention),
        "badge" => badge,
        "crown" => Json::Arr(crown_cities_json(g, h, &cv)),
        "houses" => Json::Arr(houses),
        "crown_limit" => cv.crown_limit,
        "reminders" => crate::view::keywords::reminders(d, keywords(d, KEYWORDS)),
    }
}

// ---------------------------------------------------------------------------------------------
// The grant list
// ---------------------------------------------------------------------------------------------

/// `{house}` or `{found: true}`: the protocol's form of a grantee (`grant_city`'s members).
pub fn grant_to_json(to: GrantTo) -> Json {
    match to {
        GrantTo::House(x) => crate::jobj! { "house" => x },
        GrantTo::NewHouse => crate::jobj! { "found" => true },
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
/// city, its lines, the state after when it changes). `own`: the grantee's own subject (its
/// House's label): its block is marked `self`, and the client folds it into the candidate's card,
/// whose header names it; it keeps only the lines that move loyalty (the card's own lines say the
/// rest).
fn effects_json(g: &Game, c: CityId, lines: &[PreviewLine], own: &str) -> Vec<Json> {
    let h = g.cities[c].owner;
    // The Houses' states now and their claims, by label.
    let claims = g.city_claimants(c);
    let known: Vec<(String, LoyaltyState, Option<String>)> = g.players[h]
        .houses
        .iter()
        .copied()
        .filter(|&hh| g.houses.contains(hh))
        .map(|hh| (g.house_label(hh), g.house_state(hh), claims.contains(&hh).then(|| "claimant".to_string())))
        .collect();
    let mut out: Vec<(String, Vec<String>, Option<LoyaltyState>)> = Vec::new();
    for l in lines {
        let mine = l.subject == own;
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
                "self" => subject == own,
            }
        })
        .collect()
}

/// A grant's maintenance line: `Your maintenance: -3.2 gold a turn.` (the rules' maintenance of
/// the owner's cities now and with the city a House's, inflation and the Crown's limit included),
/// at the city's lasting rate: a city that pays none now (disorder) is counted at what it pays once
/// that ends (the list's head says it pays none now). When the grant takes the Crown back toward
/// its limit, the part of the saving that is the limit's (the rules' cost of the cities over it,
/// before and after): `Your maintenance: -11 gold a turn (7 of it the Crown's limit).`
fn maintenance_line(g: &Game, c: CityId) -> String {
    let h = g.cities[c].owner;
    let m = g.grant_maintenance(c);
    let change = if pays_none_now(g, c).is_some() { m.gold_change_times100 - lasting_maintenance_times100(g, c) } else { m.gold_change_times100 };
    if gold(change) == "0" {
        return "Your maintenance: no change.".to_string();
    }
    let crown = crown_over_lasting_times100(g, h, None) - crown_over_lasting_times100(g, h, Some(c));
    let part = if change < 0 && crown > 0 && gold(crown) != "0" { format!(" ({} of it the Crown's limit)", gold(crown)) } else { String::new() };
    format!("Your maintenance: {} gold a turn{part}.", gold_change(change))
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

/// `1 Warrior`, `2 Warriors`, `2 Axemen`, `2 Priests of the Veil`: a count of a unit type's name.
fn units_of(n: usize, name: &str) -> String {
    if n == 1 {
        return format!("1 {name}");
    }
    let (head, tail) = match name.find(" of ") {
        Some(i) => name.split_at(i),
        None => (name, ""),
    };
    let plural = if let Some(stem) = head.strip_suffix("man").filter(|stem| !stem.ends_with('a')) {
        format!("{stem}men")
    } else if head.ends_with('s') || head.ends_with('x') || head.ends_with("ch") || head.ends_with("sh") {
        format!("{head}es")
    } else if head.ends_with('y') && !head.ends_with("ay") && !head.ends_with("ey") && !head.ends_with("oy") {
        format!("{}ies", &head[..head.len() - 1])
    } else {
        format!("{head}s")
    };
    format!("{n} {plural}{tail}")
}

/// What a grant of city `c` to `house` does to the units in it (rule 3; the rules' own garrison,
/// `Game::city_grant_garrison`): `Its garrison goes to House Bertram: 2 Warriors, 1 Archer.` or `No
/// garrison goes with it.`, and `Units of your armies in it stay yours.` when the liege's army
/// units stand in it.
fn garrison_lines(g: &Game, c: CityId, house: &str) -> Vec<String> {
    let (going, staying) = g.city_grant_garrison(c);
    let mut types: Vec<(UnitType, usize)> = Vec::new();
    for &u in &going {
        let t = g.units[u].unit_type;
        match types.iter_mut().find(|(k, _)| *k == t) {
            Some((_, n)) => *n += 1,
            None => types.push((t, 1)),
        }
    }
    let mut out = Vec::new();
    if types.is_empty() {
        out.push("No garrison goes with it.".to_string());
    } else {
        let list: Vec<String> = types.iter().map(|&(t, n)| units_of(n, &names::display(g.d, t))).collect();
        out.push(format!("Its garrison goes to {house}: {}.", list.join(", ")));
    }
    if !staying.is_empty() {
        out.push("Units of your armies in it stay yours.".to_string());
    }
    out
}

/// One candidate of the grant list: a House, or with none the founding of one (rule 3); what the
/// grant does (`lines`: for a founding, where its leader is drawn from; the city moves to the
/// House's realm, its garrison, the maintenance, the Crown's limit, the tribute), and its loyalty
/// changes (`effects`); `garrison`: the garrison's lines again, for the confirmation.
fn candidate_json(v: &Viewer, c: CityId, cand: &GrantCandidate) -> Json {
    let g = v.g;
    let city = city_name(g, c);
    let mut lines: Vec<String> = Vec::new();
    let house_label = cand.label.clone();
    let (kind, title, state, color, leader) = match cand.to {
        GrantTo::House(hh) => {
            let hv = g.house_view(hh).expect("a standing House");
            ("house", hv.label.clone(), Some(hv.state), Some(hv.color as i64), hv.leader)
        }
        GrantTo::NewHouse => ("found", "A new House".to_string(), None, None, None),
    };
    if cand.to == GrantTo::NewHouse {
        if let Some(line) = g.house_founding_line(v.h) {
            lines.push(core(g, &line));
        }
    }
    // What the grant does: the city leaves the Crown for the House's player, with its garrison.
    lines.push(format!("Moves to {house_label}'s realm: its production and commerce are the House's."));
    let garrison = garrison_lines(g, c, &house_label);
    lines.extend(garrison.iter().cloned());
    // The third build, rule 4: a grant that brings the House to the split's threshold.
    if let GrantTo::House(hh) = cand.to {
        lines.extend(g.house_split_line_after_grant(hh, c));
    }
    lines.push(maintenance_line(g, c));
    if let Some(l) = crown_after_line(g, v.h, c) {
        lines.push(l);
    }
    let tribute = match cand.to {
        GrantTo::House(hh) => g.houses[hh].tribute_now,
        GrantTo::NewHouse => g.players[v.h].house_tribute,
    };
    lines.push(format!("Tribute from it at {tribute}%."));
    let (confirm, button) = match cand.to {
        GrantTo::House(_) => (format!("Grant {city} to {house_label}? This cannot be undone."), format!("Grant to {house_label}")),
        GrantTo::NewHouse => (format!("Grant {city} to found {house_label}? Its leader is drawn now. This cannot be undone."), format!("Found {house_label}")),
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
        "lines" => lines,
        "effects" => Json::Arr(effects_json(g, c, &cand.lines, &house_label)),
        "refusal" => cand.refusal,
        "button" => button,
        "confirm" => confirm,
        "garrison" => garrison,
    }
}

/// `grant {city}`: the grant list of one of the human's cities (rule 13; part B): its Houses, the
/// claimants first; with none, the founding of a House (rule 3). A city that cannot be granted has
/// its `refusal` and no candidates.
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
            // Claimants first, then the other Houses (a founding is alone).
            let rank = if cand.claimant { 0 } else { 1 };
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
        "none" => (refusal.is_none() && candidates.is_empty()).then_some("No House to grant it to."),
        "candidates" => Json::Arr(candidates.into_iter().map(|(_, j)| j).collect()),
        "reminders" => crate::view::keywords::reminders(d, keywords(d, &kws)),
    })
}

// ---------------------------------------------------------------------------------------------
// The other screens: the levy (the Armies screen) and a Crown city's Governance section
// ---------------------------------------------------------------------------------------------

/// A requisition candidate's House (the levy, rule 6; the Armies screen's *Requisition from*):
/// `{id, label, colour, state, takes_levy, text, reason, until, passed}`. The row shows the
/// House's shield, name and state mark, then `text`: `Takes levies. Each levy: -1 (20 turns).` or
/// `Takes no levy.` (the mark beside it says why); the mark's tooltip carries `until` (`Annoyed
/// after 2 levies.`, from the rules' numbers) or `reason`, the rules' reason it takes none (`House
/// Arne is Annoyed. It takes no levy.`). `passed`: what the army's requisition head says of a
/// picked city it passes over (`Ventry: passed over (House Arne takes no levy).`). Null for a city
/// of no House.
pub fn levy_house(g: &Game, c: CityId) -> Json {
    let Some(h) = g.city_house(c) else { return Json::Null };
    let refusal = g.house_levy_refusal(h);
    let label = g.house_label(h);
    let text = if refusal.is_some() { "Takes no levy.".to_string() } else { levy_takes_text() };
    let until = if refusal.is_none() { levies_until_none(g.house_loyalty(h)) } else { None };
    crate::jobj! {
        "id" => h,
        "label" => label.as_str(),
        "colour" => g.houses[h].color as i32,
        "state" => g.house_state(h).name(),
        "takes_levy" => refusal.is_none(),
        "text" => text,
        "reason" => refusal.as_deref(),
        "until" => until,
        "passed" => refusal.is_some().then(|| format!("{}: passed over ({label} takes no levy).", city_name(g, c))),
    }
}

/// The city screen's Governance line for one of the human's cities (rule 13; part C): every city
/// of the human's is the Crown's (a House's city is its own player's), so `crown: {grantable,
/// refusal (why it cannot be granted, else null), over (`Over the Crown's limit of 3.` or
/// null)}` and `house: null`. Nothing for a human without Houses.
pub fn governance_fields(v: &Viewer, c: CityId, j: &mut Json) {
    let g = v.g;
    if !g.player_has_houses(v.h) || g.cities[c].owner != v.h {
        return;
    }
    let why = g.player_check_grant_city(v.h, c).err();
    let over = g.player_crown_limit(v.h).filter(|_| g.player_crown_over(v.h).contains(&c)).map(|limit| format!("Over the Crown's limit of {limit}."));
    j.set("house", Json::Null);
    j.set("crown", crate::jobj! { "grantable" => why.is_none(), "refusal" => why, "over" => over });
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
