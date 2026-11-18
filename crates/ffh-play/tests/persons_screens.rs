//! Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts C to I; "The second
//! build"), on the staged scenes (`ffh-play stage persons|persons-direct`) through the session's
//! queries: the House marks of the cities on the map (the liege sees the attitude, the others the
//! shield only), the Crown cities in the Governors screen, the Armies screen's commanders and the
//! levy's House lines among the requisition's candidates, the unit panel's person and Found a
//! House, the civics screen's warning of a direct-rule civic, the war and peace previews, and the
//! log's tags for the alerts.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_core::log::{LogEntry, LogMark, LogTag};
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::stage::StagedPersons;

fn load(name: &str, direct: bool) -> (common::TempDir, StagedPersons, ffh_play::Session) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::persons(&dir.0, direct).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
    let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    (dir, st, s)
}

fn q(s: &ffh_play::Session, text: &str) -> Json {
    s.query_json(&Json::parse(text).unwrap()).unwrap_or_else(|e| panic!("{text}: {e:?}"))
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("no array `{key}` in {j}")).to_vec()
}

fn find(list: &[Json], key: &str, id: i64) -> Json {
    list.iter().find(|x| x.get(key).and_then(Json::as_i64) == Some(id)).unwrap_or_else(|| panic!("no {key} {id}")).clone()
}

fn s_of<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("no string `{key}` in {j}"))
}

#[test]
fn the_screens_show_houses_commanders_and_persons() {
    let (_dir, st, s) = load("persons-screens", false);
    let g = &s.g;
    let house = |role: &str| st.houses.iter().find(|x| x.role == role).unwrap();
    let person = |role: &str| st.persons.iter().find(|x| x.0 == role).unwrap().1;
    let army = |role: &str| st.armies.iter().find(|x| x.0 == role).unwrap().1;

    // G. The map's marks: a House's city carries its shield, and its liege sees its attitude.
    let view = q(&s, r#"{"q":"view"}"#);
    let cities = arr(&view, "cities");
    let f = house("furious");
    let mark = find(&cities, "id", f.cities[0].0 as i64).get("house").unwrap().clone();
    assert_eq!(s_of(&mark, "state"), "Furious");
    assert_eq!(mark.get("turns").and_then(Json::as_i64), Some(8));
    assert_eq!(mark.get("colour").and_then(Json::as_i64), Some(g.houses[f.id].color as i64));
    assert!(find(&cities, "id", st.crown[0].0 as i64).get("house").unwrap().is_null(), "a Crown city");
    let n = house("neighbour");
    if let Some(x) = cities.iter().find(|x| n.cities.iter().any(|c| x.get("id").and_then(Json::as_i64) == Some(c.0 as i64))) {
        let m = x.get("house").expect("another's House city shows its shield");
        assert!(m.get("state").unwrap().is_null(), "only the liege sees the attitude: {m}");
    }
    // H. The cast in brief, for the alerts' portraits.
    let cast = arr(&view, "persons");
    assert!(find(&cast, "id", person("head_friendly").0 as i64).get("leader").is_some());

    // E. The Governors screen's rows: the human's cities are the Crown's.
    let gs = q(&s, r#"{"q":"governors"}"#);
    let rows = arr(&gs, "cities");
    assert!(find(&rows, "city", st.capital.0 as i64).get("house").unwrap().is_null());

    // D. The Armies screen: the commanders' lines and the persons with an army.
    let ar = q(&s, r#"{"q":"armies"}"#);
    let armies = arr(&ar, "armies");
    let loyal = find(&armies, "id", army("loyal").0 as i64);
    let cmd = loyal.get("commander").unwrap();
    assert_eq!(cmd.get("person").and_then(Json::as_i64), Some(person("commander_loyal").0 as i64));
    assert_eq!(cmd.get("withholds_attack").and_then(Json::as_bool), Some(false));
    assert!(!arr(&loyal, "persons_with").is_empty());
    let furious = find(&armies, "id", army("furious").0 as i64);
    let cmd = furious.get("commander").unwrap();
    assert_eq!(s_of(cmd, "state"), "Furious");
    assert_eq!(cmd.get("withholds_attack").and_then(Json::as_bool), Some(true));
    assert!(cmd.get("turns").and_then(Json::as_i64).is_some());
    let o = s_of(cmd, "outcome");
    assert!([" will seize ", " will join ", " will resign "].iter().any(|w| o.contains(w)), "{cmd}");
    // The levy: the Houses' cities among the requisition's candidates, with their House line.
    let cands = arr(loyal.get("requisition").unwrap(), "candidates");
    let a = house("friendly");
    let row = find(&cands, "city", a.cities[0].0 as i64);
    assert!(s_of(row.get("house").unwrap(), "text").starts_with(&format!("{}: takes levies.", a.label)), "{row}");
    let v = house("annoyed");
    let row = find(&cands, "city", v.cities[0].0 as i64);
    assert_eq!(s_of(row.get("house").unwrap(), "text"), format!("{} is Annoyed. It takes no levy.", v.label));
    assert!(find(&cands, "city", st.capital.0 as i64).get("house").unwrap().is_null());

    // F. The unit panel: a commander's unit and the great person's Found a House.
    let u = g.person_unit(person("commander_loyal")).unwrap();
    let grp = q(&s, &format!(r#"{{"q":"group","units":[{}]}}"#, u.0));
    let pu = arr(&grp, "units")[0].get("person").unwrap().clone();
    assert_eq!(s_of(&pu, "post"), format!("commander of the {}", g.armies[army("loyal")].name));
    let grp = q(&s, &format!(r#"{{"q":"group","units":[{}]}}"#, st.great_person.0));
    let fh = arr(&grp, "units")[0].get("found_house").unwrap().clone();
    let fc = CityId(fh.get("city").and_then(Json::as_i64).expect("a grantable city") as u32);
    assert!(g.player_check_grant_city(st.human, fc).is_ok());
    assert!(arr(&grp, "units")[0].get("person").unwrap().is_null());

    // I. The war dialog's preview, the peace preview (at peace: none).
    let pl = q(&s, r#"{"q":"players"}"#);
    let nb = find(&arr(&pl, "players"), "id", st.neighbour.0 as i64);
    assert!(nb.get("war_preview").and_then(Json::as_arr).is_some(), "{nb}");
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, st.neighbour.0));
    assert_eq!(tr.get("peace_preview").and_then(Json::as_arr).map(|a| a.len()), Some(0));
    // A House is a party on the diplomacy screen: it gives Secede and Become your vassal.
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, g.houses[n.id].player.0));
    let theirs = arr(&tr, "they_give");
    let names: Vec<&str> = theirs.iter().filter_map(|x| x.get("item").and_then(Json::as_i64)).map(|i| TradeableItem::NAMES[i as usize]).collect();
    assert!(names.contains(&"TRADE_SECEDE") && names.contains(&"TRADE_HOUSE_VASSAL"), "{names:?}");
    // A religion's preview list is there for every religion.
    let rl = q(&s, r#"{"q":"religions"}"#);
    for x in arr(&rl, "religions") {
        assert!(x.get("persons").and_then(Json::as_arr).is_some());
    }
}

#[test]
fn the_civics_screen_warns_of_a_direct_rule_civic() {
    let (_dir, st, s) = load("persons-screens-direct", true);
    let g = &s.g;
    let d = g.d;
    let god_king = d.t::<CivicType>("CIVIC_GOD_KING");
    let r = q(&s, &format!(r#"{{"q":"civics","with":[{}]}}"#, god_king.idx()));
    let p = r.get("proposed").unwrap();
    let warning = s_of(p, "persons_warning");
    let houses = g.players[st.human].houses.len();
    assert!(warning.starts_with(&format!("Adopting {}: {houses} Houses break away with ", ffh_play::names::display(d, god_king))), "{warning}");
    assert!(warning.contains(", each at war with us: "), "{warning}");
    assert!(warning.contains(" stays: an army of ours stands in it.") || warning.contains(" stay: an army of ours stands in each."), "{warning}");
    assert_eq!(s_of(p, "persons_button"), format!("Adopt {}: {houses} Houses break away", ffh_play::names::display(d, god_king)));
    let lines = arr(p, "persons");
    assert_eq!(lines.len(), houses, "one line a House: {p}");
    // Another government: no warning.
    let republic = d.t::<CivicType>("CIVIC_REPUBLIC");
    if g.player_can_do_civics(st.human, Some(republic)) {
        let r = q(&s, &format!(r#"{{"q":"civics","with":[{}]}}"#, republic.idx()));
        assert!(r.get("proposed").unwrap().get("persons_warning").is_none());
    }
}

#[test]
fn the_log_tags_persons_and_break_aways_for_the_alerts() {
    let mut e = LogEntry::new(3, Some(0), "Hrolf demands: A House. 40 turns.".into());
    e.mark = LogMark::Person;
    e.tag = LogTag::Person(PersonId(7));
    e.at = Some((4, 5));
    let j = ffh_play::report::entry_fields(Json::Obj(Default::default()), &e);
    assert_eq!(j.get("kind").and_then(Json::as_str), Some("person"));
    assert_eq!(j.get("person").and_then(Json::as_i64), Some(7));
    assert_eq!(j.get("mark").and_then(Json::as_str), Some("person"));
    for text in [
        "House Arne breaks away from the Bannor with Prespur and Elam: war.",
        "Hippus breaks away from the Bannor with Prespur for the Elohim: war.",
        "Civil war: House Arne breaks away from the Bannor with Prespur: war.",
        "Hrolf, 2nd Army goes over to House Arne with 4 units.",
        "Hrolf, 2nd Army seizes Vallus and secedes from the Bannor with 4 units.",
        "House Arne is freed by the Bannor.",
    ] {
        let e = LogEntry::new(3, None, text.into());
        let j = ffh_play::report::entry_fields(Json::Obj(Default::default()), &e);
        assert_eq!(j.get("kind").and_then(Json::as_str), Some("secession"), "{text}");
    }
    for text in ["Your Warrior has been killed.", "Hrolf goes over to the Elohim."] {
        let e = LogEntry::new(3, None, text.into());
        assert!(ffh_play::report::entry_fields(Json::Obj(Default::default()), &e).get("kind").is_none(), "{text}");
    }
}
