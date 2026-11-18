//! Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts C to I): the JSON pieces
//! the city screen, the Governors and Armies screens, the cities list, the unit panel, the map's
//! plates and the screens where an act moves loyalty share. Read only, from the rules' read API
//! (`ffh_core::persons`); the Court screen and the grant list are `view::persons`.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{LoyaltyState, Post, PreviewLine};
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A state as the client's mark takes it (`Friendly`, `Pleased`, `Cautious`, `Annoyed`,
/// `Furious`).
pub fn state_name(s: LoyaltyState) -> &'static str {
    s.name()
}

/// One preview line as a sentence: `House Arne: Demand refused: -3 (60 turns). Becomes Restless.`
pub fn preview_sentence(l: &PreviewLine) -> String {
    let mut s = format!("{}: {}", l.subject, l.text);
    if !s.ends_with('.') {
        s.push('.');
    }
    if let Some(st) = l.state_after {
        s.push_str(&format!(" Becomes {}.", st.name()));
    }
    s
}

/// Preview lines (`Game::persons_preview`, `grant_preview`): `[{subject, text, state_after,
/// line}]`, `line` the whole sentence ([`preview_sentence`]); `state_after` null when the state
/// does not change.
pub fn preview_json(lines: &[PreviewLine]) -> Json {
    Json::Arr(
        lines
            .iter()
            .map(|l| {
                crate::jobj! {
                    "subject" => l.subject.as_str(),
                    "text" => l.text.as_str(),
                    "state_after" => l.state_after.map(|s| Json::from(s.name())),
                    "line" => preview_sentence(l),
                }
            })
            .collect(),
    )
}

/// A House in a list (part E) and on a plate (part G): `{id, label, colour, state, turns}`
/// (`colour`: `House::color`, the client's `houseColour` index; `turns`: the countdown's turns
/// left, null without one).
pub fn house_chip(g: &Game, h: HouseId) -> Json {
    let state = g.house_state(h);
    let turns = g.houses[h].standing.countdown.map(|end| (end - g.game_turn()).max(0));
    crate::jobj! {
        "id" => h,
        "label" => g.house_label(h),
        "colour" => g.houses[h].color as i32,
        "state" => state.name(),
        "turns" => turns,
    }
}

/// A city of the human's in the lists and in the view's city entry: its House's chip, or null for
/// the Crown's. `None` when the human has no Houses (no persons: the field is left out).
pub fn city_house_field(v: &Viewer, c: CityId) -> Option<Json> {
    let g = v.g;
    if g.cities[c].owner != v.h || !g.player_has_persons(v.h) {
        return None;
    }
    Some(g.city_house(c).map_or(Json::Null, |h| house_chip(g, h)))
}

/// The map's mark of a House's city (part G; docs/PERSONS.md, rule 13): its shield for everyone
/// (`{id, label, colour}`), and for its liege its attitude mark (`state`, `turns`; null for the
/// others), from `Game::city_house_mark` (cheap); `None` for a city of a civilization that is no
/// House.
pub fn city_map_mark(v: &Viewer, c: CityId) -> Option<Json> {
    let g = v.g;
    let m = g.city_house_mark(c)?;
    let liege = m.owner == v.h;
    Some(crate::jobj! {
        "id" => m.house,
        "label" => g.house_label(m.house),
        "colour" => m.color as i32,
        "state" => liege.then(|| m.state.name()),
        "turns" => if liege { m.countdown_turns } else { None },
    })
}

/// A person in brief: `{person, name, title, leader}` (`leader`: the portrait, null for a hero).
fn person_brief(g: &Game, x: PersonId) -> Json {
    let p = &g.persons[x];
    crate::jobj! {
        "person" => x,
        "name" => p.name.as_str(),
        "title" => p.leader.and_then(|l| ffh_core::persons::leader_title(g.d, l)),
        "leader" => p.leader.map(Json::idx),
    }
}

/// The city screen's Governance line (part C), in `governance.house` or `governance.crown`:
/// - a House's city (its liege's view): `house: {id, label, colour, state, turns, outcome, head:
///   {person, name, title, leader} | null, levy (`Takes levies.` or the rules' reason it takes
///   none), takes_levy, tribute (`Tribute 20%.`)}`;
/// - a Crown city: `crown: {grantable, refusal (why it cannot be granted, else null)}`.
///
/// Nothing for a human without Houses.
pub fn governance_fields(v: &Viewer, c: CityId, j: &mut Json) {
    let g = v.g;
    if !g.player_has_persons(v.h) {
        return;
    }
    match g.city_house(c) {
        Some(h) => {
            let mut house = house_chip(g, h);
            let head = g.houses[h].head.filter(|&x| g.persons.contains(x));
            house.set("head", head.map(|x| person_brief(g, x)));
            house.set("outcome", g.houses[h].standing.countdown.map(|_| g.house_breakaway_outcome(h)));
            let refusal = g.house_levy_refusal(h);
            house.set("takes_levy", refusal.is_none());
            house.set("levy", refusal.unwrap_or_else(|| "Takes levies.".to_string()));
            house.set("tribute", format!("Tribute {}%.", g.houses[h].tribute));
            j.set("house", house);
            j.set("crown", Json::Null);
        }
        None => {
            let why = g.player_check_grant_city(v.h, c).err();
            j.set("house", Json::Null);
            j.set("crown", crate::jobj! { "grantable" => why.is_none(), "refusal" => why });
        }
    }
}

/// A requisition candidate's House line (part D; the levy, docs/PERSONS.md, rule 6): `{id, label,
/// colour, state, takes_levy, text}`, `text` `House Arne: takes levies. Each is -1 (20 turns).` or
/// the rules' reason it takes none; null for a city of no House.
pub fn requisition_house(g: &Game, c: CityId) -> Json {
    let Some(h) = g.city_house(c) else { return Json::Null };
    let label = g.house_label(h);
    let refusal = g.house_levy_refusal(h);
    let (amount, turns) = ffh_core::persons::LEVIED;
    let mut j = house_chip(g, h);
    j.set("takes_levy", refusal.is_none());
    j.set("text", refusal.unwrap_or_else(|| format!("{label}: takes levies. Each order placed: Levied {amount} ({turns} turns).")));
    j
}

/// An army's commander (part D), in the army's `commander`: `{person, name, title, leader,
/// unit_type, level, state, turns, outcome, withholds_attack}` (`outcome`: the countdown's stated
/// outcome, with its turns; `withholds_attack`: the army takes no order to
/// attack: Annoyed or Furious), or null; and `persons_with` (`[{person, name, unit_type, level}]`: every person with
/// the army, the commander among them; `[Change]` when more than one).
pub fn army_fields(v: &Viewer, a: ArmyId, j: &mut Json) {
    let g = v.g;
    if !g.player_has_persons(v.h) {
        return;
    }
    let commander = g.armies[a].commander.filter(|&x| g.persons.contains(x)).and_then(|x| g.person_view(x));
    j.set(
        "commander",
        commander.map(|pv| {
            let mut c = person_brief(g, pv.id);
            c.set("unit_type", pv.unit_type.map(Json::idx));
            c.set("level", pv.level);
            c.set("state", pv.state.name());
            c.set("turns", pv.countdown_turns);
            c.set("outcome", pv.outcome);
            c.set("withholds_attack", pv.state.withholds());
            c
        }),
    );
    let with: Vec<Json> = g
        .army_persons_with(a)
        .into_iter()
        .filter_map(|x| g.person_view(x))
        .map(|pv| crate::jobj! { "person" => pv.id, "name" => pv.name, "unit_type" => pv.unit_type.map(Json::idx), "level" => pv.level })
        .collect();
    j.set("persons_with", Json::Arr(with));
}

/// The person a unit is (part F), in the unit's `person`: `{person, name, title, leader, post
/// (`commander of the 2nd Army`, `head of House Arne`, `no post`), state, turns}`; null for a unit
/// that is no person.
pub fn unit_person(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let Some(x) = g.unit_person(u) else { return Json::Null };
    let Some(pv) = g.person_view(x) else { return Json::Null };
    let post = match pv.post {
        Post::Commander(a) if g.armies.contains(a) => format!("commander of the {}", g.armies[a].name),
        Post::Head(h) if g.houses.contains(h) => format!("head of {}", g.house_label(h)),
        _ => "no post".to_string(),
    };
    let mut j = person_brief(g, x);
    j.set("post", post);
    j.set("state", pv.state.name());
    j.set("turns", pv.countdown_turns);
    j
}

/// A great person's `Found a House` (part F), in the unit's `found_house`: `{city, name}` (the
/// Crown's grantable city nearest the unit: the grant list opens there, this unit first) or
/// `{city: null, why}` (no city can be granted now, or no person of note is left); null for a unit
/// that is no great person, or a human without Houses.
pub fn unit_found_house(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let unit = &g.units[u];
    if unit.owner != v.h || !g.player_has_persons(v.h) || !g.unit_is_great_person(u) {
        return Json::Null;
    }
    if let Err(why) = g.player_check_grants(v.h) {
        return crate::jobj! { "city" => Json::Null, "why" => why };
    }
    let nearest = g.players[v.h]
        .cities
        .iter()
        .copied()
        .filter(|&c| g.cities.contains(c) && g.player_check_grant_city(v.h, c).is_ok())
        .min_by_key(|&c| (g.step_distance(unit.x, unit.y, g.cities[c].x, g.cities[c].y), c));
    let Some(c) = nearest else { return crate::jobj! { "city" => Json::Null, "why" => "No city of the Crown can be granted." } };
    match g.player_check_grant(v.h, c, ffh_core::persons::GrantTo::GreatPerson(u)) {
        Ok(()) => crate::jobj! { "city" => c, "name" => city_name(g, c) },
        Err(why) => crate::jobj! { "city" => Json::Null, "why" => why },
    }
}

/// The civics screen's preview of a proposed set (part I), set on `proposed`: `persons` (the
/// rules' lines) and, when the set brings direct rule while Houses stand, `persons_warning`
/// (`Adopting God King: 3 Houses break away with 9 cities, each at war with us: Prespur, Elam and
/// Torrin. Vallus stays: an army of ours stands in it.`) and `persons_button` (`Adopt God King: 3
/// Houses break away`).
pub fn civics_preview(v: &Viewer, set: &[Option<CivicType>], proposed: &mut Json) {
    let g = v.g;
    let d = g.d;
    if !g.player_has_persons(v.h) {
        return;
    }
    let adopted: Vec<CivicType> = set.iter().enumerate().filter_map(|(o, &c)| c.filter(|_| g.players[v.h].civics[CivicOptionType::from_usize(o)] != c)).collect();
    let lines = g.persons_preview(v.h, &ffh_core::persons::PersonsAct::Civics(&adopted));
    proposed.set("persons", preview_json(&lines));
    let direct = adopted.iter().copied().find(|&c| d.civics[c].direct_rule);
    let houses: Vec<HouseId> = g.players[v.h].houses.iter().copied().filter(|&h| g.houses.contains(h)).collect();
    if let (Some(civic), false, false) = (direct, houses.is_empty(), g.player_rules_directly(v.h)) {
        let mut going = Vec::new();
        let mut held = Vec::new();
        for &h in &houses {
            let (gw, hd) = g.house_cities_held(h);
            going.extend(gw);
            held.extend(hd);
        }
        let names = |cs: &[CityId]| and_list(&cs.iter().map(|&c| city_name(g, c)).collect::<Vec<_>>());
        let n = houses.len();
        let houses_n = if n == 1 { "1 House".to_string() } else { format!("{n} Houses") };
        let civic_name = crate::names::display(d, civic);
        let mut text = if going.is_empty() {
            format!("Adopting {civic_name}: {houses_n} end{}: an army of ours stands in every city.", if n == 1 { "s" } else { "" })
        } else {
            format!(
                "Adopting {civic_name}: {houses_n} break{} away with {} cit{}, each at war with us: {}.",
                if n == 1 { "s" } else { "" },
                going.len(),
                if going.len() == 1 { "y" } else { "ies" },
                names(&going),
            )
        };
        if !going.is_empty() && !held.is_empty() {
            text.push(' ');
            text.push_str(&stays_line(g, &held));
        }
        proposed.set("persons_warning", text);
        proposed.set("persons_button", format!("Adopt {civic_name}: {houses_n} break{} away", if n == 1 { "s" } else { "" }));
    }
}

/// The cities a break-away leaves to the liege, because an army of the liege stands in them:
/// `Censura stays: an army of ours stands in it.`, `Censura and Torrin stay: an army of ours
/// stands in each.`
pub(crate) fn stays_line(g: &Game, held: &[CityId]) -> String {
    let names = and_list(&held.iter().map(|&c| city_name(g, c)).collect::<Vec<_>>());
    if held.len() == 1 {
        format!("{names} stays: an army of ours stands in it.")
    } else {
        format!("{names} stay: an army of ours stands in each.")
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

/// The preview lines of act `act` of the human (part I), as [`preview_json`]; an empty array
/// without Houses or persons.
pub fn act_preview(v: &Viewer, act: &ffh_core::persons::PersonsAct) -> Json {
    if !v.g.player_has_persons(v.h) || v.g.players[v.h].persons.is_empty() {
        return Json::Arr(Vec::new());
    }
    preview_json(&v.g.persons_preview(v.h, act))
}

/// The human's cast in brief, for the alerts' portraits (part H): `[{id, name, leader}]`.
pub fn cast_brief(v: &Viewer) -> Json {
    let g = v.g;
    Json::Arr(
        g.players[v.h]
            .persons
            .iter()
            .filter(|&&x| g.persons.contains(x))
            .map(|&x| crate::jobj! { "id" => x, "name" => g.persons[x].name.as_str(), "leader" => g.persons[x].leader.map(Json::idx) })
            .collect(),
    )
}
