//! Houses in the existing screens (docs/PERSONS.md, "UI", parts C to I): the JSON pieces the city
//! screen, the Governors and Armies screens, the cities list, the map's plates and the screens
//! where an act moves a House's loyalty share. Read only, from the rules' read API
//! (`ffh_core::persons`); the Court screen and the grant list are `view::persons`.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{LoyaltyState, PreviewLine};
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A state as the client's mark takes it (`Friendly`, `Pleased`, `Cautious`, `Annoyed`,
/// `Furious`).
pub fn state_name(s: LoyaltyState) -> &'static str {
    s.name()
}

/// One preview line as a sentence: `House Arne: Request refused: -3 (60 turns). Becomes Annoyed.`
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
/// the Crown's. `None` when the human can have no Houses (the field is left out).
pub fn city_house_field(v: &Viewer, c: CityId) -> Option<Json> {
    let g = v.g;
    if g.cities[c].owner != v.h || !g.player_has_houses(v.h) {
        return None;
    }
    Some(g.city_house(c).map_or(Json::Null, |h| house_chip(g, h)))
}

/// The map's mark of a House's city (part G; docs/PERSONS.md, rules 1 and 13): its shield for
/// everyone (`{id, label, colour, liege}`: `liege` is the player whose colour the House is drawn
/// in, `Game::player_color_player`), and for its liege its attitude mark (`state`, `turns`; null
/// for the others), from `Game::city_house_mark` (cheap); `None` for a city of a civilization that
/// is no House.
pub fn city_map_mark(v: &Viewer, c: CityId) -> Option<Json> {
    let g = v.g;
    let m = g.city_house_mark(c)?;
    let liege = m.owner == v.h;
    Some(crate::jobj! {
        "id" => m.house,
        "label" => g.house_label(m.house),
        "colour" => m.color as i32,
        "liege" => m.owner,
        "state" => liege.then(|| m.state.name()),
        "turns" => if liege { m.countdown_turns } else { None },
    })
}

/// `+2`, `-1`, `0`.
fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// A loyalty part as a line: `Tribute 20%: -1`, `Levied: -1 (12 turns)`.
fn part_line(p: &ffh_core::persons::LoyaltyPart) -> String {
    match p.turns_left {
        Some(t) => format!("{}: {} ({t} {})", p.label, signed(p.amount), if t == 1 { "turn" } else { "turns" }),
        None => format!("{}: {}", p.label, signed(p.amount)),
    }
}

/// A House among the players (the view's `players[].house`; docs/PERSONS.md, rules 1 and 13: the
/// diplomacy screen lists a House under its liege): `{id, label, liege, liege_name, colour}` and,
/// for its liege, its attitude toward it as the rules give it: `state`, `turns` (the Furious
/// countdown's turns left, else null), `outcome` (the countdown's stated outcome, else null),
/// `parts` (`["Tribute 20%: -1", ..]`) and `loyalty` (`Loyalty: -2.`); null for a player that is
/// no House. `liege_name` is the liege's civilization (`Bannor`), as the rules' lines name it.
pub fn player_house_brief(v: &Viewer, p: PlayerId) -> Json {
    let g = v.g;
    let Some(h) = g.player_house(p) else { return Json::Null };
    let house = &g.houses[h];
    let liege = house.owner;
    let mut j = crate::jobj! {
        "id" => h,
        "label" => g.house_label(h),
        "liege" => liege,
        "liege_name" => g.player_get_civilization_short_description(liege),
        "colour" => house.color as i32,
    };
    if liege == v.h {
        if let Some(hv) = g.house_view(h) {
            j.set("state", hv.state.name());
            j.set("turns", hv.countdown_turns);
            j.set("outcome", hv.outcome);
            j.set("parts", Json::Arr(hv.parts.iter().map(|x| Json::from(part_line(x))).collect()));
            j.set("loyalty", format!("Loyalty: {}.", signed(hv.loyalty)));
        }
    }
    j
}

/// The civics screen's preview of a proposed set (part I), set on `proposed`: `persons` (the
/// rules' lines) and, when the set brings direct rule while Houses stand, `persons_warning`
/// (`Adopting God King: 3 Houses break away, taking 9 cities, each at war with you.`),
/// `persons_houses` (a line per House, from the cities its break-away takes and the contest for
/// each city where an army of yours stands, `Game::house_cities_held` and `Game::city_hold_line`,
/// as the rules' preview reads them: `House Aldric: Ventry. Corona returns to you: your army's 4
/// units outnumber its 2.`, `Malakim (a vassal): Astrum and Golden Leane. Astrum goes with it: its
/// 3 units against your army's 2.`) and `persons_button` (`Adopt God King: 3 Houses break away`).
pub fn civics_preview(v: &Viewer, set: &[Option<CivicType>], proposed: &mut Json) {
    let g = v.g;
    let d = g.d;
    if !g.player_has_houses(v.h) {
        return;
    }
    let adopted: Vec<CivicType> = set.iter().enumerate().filter_map(|(o, &c)| c.filter(|_| g.players[v.h].civics[CivicOptionType::from_usize(o)] != c)).collect();
    let lines = g.persons_preview(v.h, &ffh_core::persons::PersonsAct::Civics(&adopted));
    proposed.set("persons", preview_json(&lines));
    let direct = adopted.iter().copied().find(|&c| d.civics[c].direct_rule);
    let houses: Vec<HouseId> = g.players[v.h].houses.iter().copied().filter(|&h| g.houses.contains(h)).collect();
    if let (Some(civic), false, false) = (direct, houses.is_empty(), g.player_rules_directly(v.h)) {
        let names = |cs: &[CityId]| and_list(&cs.iter().map(|&c| city_name(g, c)).collect::<Vec<_>>());
        let mut going_n = 0;
        let mut lines = Vec::new();
        for &h in &houses {
            let (going, _) = g.house_cities_held(h);
            going_n += going.len();
            // A vassal civilization's name is not a House's: it is told as one.
            let who = if g.houses[h].origin == ffh_core::persons::HouseOrigin::Founded { g.house_label(h) } else { format!("{} (a vassal)", g.house_label(h)) };
            // The contest for each city where an army of yours stands (` Corona returns to you:
            // your army's 4 units outnumber its 2.`).
            let contests = g.house_hold_text(h);
            lines.push(if going.is_empty() { format!("{who} ends.{contests}") } else { format!("{who}: {}.{contests}", names(&going)) });
        }
        let n = houses.len();
        let houses_n = if n == 1 { "1 House".to_string() } else { format!("{n} Houses") };
        let civic_name = crate::names::display(d, civic);
        let text = if going_n == 0 {
            format!("Adopting {civic_name}: {houses_n} end{}: your armies hold every city.", if n == 1 { "s" } else { "" })
        } else {
            format!(
                "Adopting {civic_name}: {houses_n} break{} away, taking {going_n} cit{}, each at war with you.",
                if n == 1 { "s" } else { "" },
                if going_n == 1 { "y" } else { "ies" },
            )
        };
        proposed.set("persons_warning", text);
        proposed.set("persons_houses", lines);
        proposed.set("persons_button", format!("Adopt {civic_name}: {houses_n} break{} away", if n == 1 { "s" } else { "" }));
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
/// without Houses.
pub fn act_preview(v: &Viewer, act: &ffh_core::persons::PersonsAct) -> Json {
    if !v.g.player_has_houses(v.h) || v.g.players[v.h].houses.is_empty() {
        return Json::Arr(Vec::new());
    }
    preview_json(&v.g.persons_preview(v.h, act))
}

/// The human's Houses in brief, for the alerts' portraits and where a tap on one goes (part H):
/// `[{id, label, leader, leader_name}]` (`leader`: the portrait).
pub fn houses_brief(v: &Viewer) -> Json {
    let g = v.g;
    Json::Arr(
        g.players[v.h]
            .houses
            .iter()
            .filter(|&&h| g.houses.contains(h))
            .map(|&h| {
                let leader = g.house_leader(h);
                crate::jobj! {
                    "id" => h,
                    "label" => g.house_label(h),
                    "leader" => leader.map(Json::idx),
                    "leader_name" => leader.map(|l| ffh_core::persons::leader_name(g.d, l)),
                }
            })
            .collect(),
    )
}
