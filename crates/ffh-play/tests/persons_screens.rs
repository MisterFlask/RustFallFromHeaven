//! Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts C to I), on the staged
//! scenes (`ffh-play stage persons|persons-direct`) through the session's queries: the city
//! screen's holder line, the Governors screen's and the view's House of each own city (and none
//! for another's), the Armies screen's commanders and the requisition's House line, the unit
//! panel's person and Found a House, the civics screen's warning of a direct-rule civic, the war
//! and peace previews, and the log's tags for the alerts.

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

    // C. The city screen's Governance: a House's city, then a Crown city and the capital.
    let a = house("A");
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, a.seat.unwrap().0));
    let gov = city.get("governance").unwrap();
    let hs = gov.get("house").expect("governance.house");
    assert_eq!(s_of(hs, "label"), a.label);
    assert_eq!(s_of(hs, "state"), "Devoted");
    assert_eq!(hs.get("colour").and_then(Json::as_i64), Some(g.houses[a.id].color as i64));
    assert_eq!(hs.get("head").and_then(|x| x.get("person")).and_then(Json::as_i64), Some(person("head_a").0 as i64));
    assert_eq!(s_of(hs, "orders_text"), "Orders: no limit.");
    assert!(hs.get("order_refusal").unwrap().is_null());
    assert!(hs.get("trait").is_some());
    let b = house("B");
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, b.seat.unwrap().0));
    let hs = city.get("governance").unwrap().get("house").unwrap().clone();
    assert_eq!(s_of(&hs, "state"), "Restless");
    assert_eq!(s_of(&hs, "order_refusal"), format!("{} is Restless. It takes no orders.", b.label));
    assert_eq!(s_of(&hs, "orders_text"), format!("{} is Restless. It takes no orders.", b.label));
    let c = house("C");
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, c.seat.unwrap().0));
    let hs = city.get("governance").unwrap().get("house").unwrap().clone();
    assert_eq!(s_of(&hs, "state"), "Disloyal");
    assert_eq!(hs.get("turns").and_then(Json::as_i64), Some(8));
    assert!(s_of(&hs, "outcome").contains("in 8 turns"), "{hs}");
    // The House's city lists its empire orders disabled, with the reason.
    for b in arr(&city, "buildable") {
        assert_eq!(b.get("enabled").and_then(Json::as_bool), Some(false), "{b}");
    }
    let crown = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, st.crown[1].0));
    let gov = crown.get("governance").unwrap();
    assert!(gov.get("house").unwrap().is_null());
    assert_eq!(gov.get("crown").and_then(|x| x.get("grantable")).and_then(Json::as_bool), Some(true));
    let capital = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, st.capital.0));
    let cr = capital.get("governance").unwrap().get("crown").unwrap().clone();
    assert_eq!(cr.get("grantable").and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(&cr, "refusal"), "The capital is the Crown's.");

    // C. The empire order's city step: House B's cities are listed refused, with the reason.
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR").idx();
    let ec = q(&s, &format!(r#"{{"q":"empire_cities","order":0,"data":{warrior}}}"#));
    let refused = arr(&ec, "refused");
    for &rc in &b.cities {
        let row = find(&refused, "city", rc.0 as i64);
        assert_eq!(s_of(&row, "why"), format!("{} is Restless. It takes no orders.", b.label));
        assert!(arr(&ec, "cities").iter().all(|x| x.get("city").and_then(Json::as_i64) != Some(rc.0 as i64)));
    }

    // E. The Governors screen's rows and the view's cities: own cities carry their House (null:
    // the Crown's); another's carry none.
    let gs = q(&s, r#"{"q":"governors"}"#);
    let rows = arr(&gs, "cities");
    assert_eq!(s_of(find(&rows, "city", a.cities[1].0 as i64).get("house").unwrap(), "label"), a.label);
    assert!(find(&rows, "city", st.capital.0 as i64).get("house").unwrap().is_null());
    let view = q(&s, r#"{"q":"view"}"#);
    let cities = arr(&view, "cities");
    let mark = find(&cities, "id", c.cities[0].0 as i64).get("house").unwrap().clone();
    assert_eq!(s_of(&mark, "state"), "Disloyal");
    assert_eq!(mark.get("turns").and_then(Json::as_i64), Some(8));
    assert_eq!(mark.get("colour").and_then(Json::as_i64), Some(g.houses[c.id].color as i64));
    assert!(find(&cities, "id", st.crown[0].0 as i64).get("house").unwrap().is_null());
    for &nc in &st.neighbour_cities {
        if let Some(x) = cities.iter().find(|x| x.get("id").and_then(Json::as_i64) == Some(nc.0 as i64)) {
            assert!(x.get("house").is_none(), "another's city has no house: {x}");
        }
    }
    // H. The cast in brief, for the alerts' portraits.
    let cast = arr(&view, "persons");
    assert!(find(&cast, "id", person("head_a").0 as i64).get("leader").is_some());

    // D. The Armies screen: the commanders' lines and the persons with an army.
    let ar = q(&s, r#"{"q":"armies"}"#);
    let armies = arr(&ar, "armies");
    let loyal = find(&armies, "id", army("loyal").0 as i64);
    let cmd = loyal.get("commander").unwrap();
    assert_eq!(cmd.get("person").and_then(Json::as_i64), Some(person("commander_loyal").0 as i64));
    assert_eq!(s_of(cmd, "state"), "Loyal");
    assert_eq!(cmd.get("withholds_attack").and_then(Json::as_bool), Some(false));
    assert!(!arr(&loyal, "persons_with").is_empty());
    let disloyal = find(&armies, "id", army("disloyal").0 as i64);
    let cmd = disloyal.get("commander").unwrap();
    assert_eq!(s_of(cmd, "state"), "Disloyal");
    assert_eq!(cmd.get("withholds_attack").and_then(Json::as_bool), Some(true));
    assert!(cmd.get("turns").and_then(Json::as_i64).is_some());
    assert!(s_of(cmd, "outcome").contains("sworn units"), "{cmd}");
    assert!(find(&armies, "id", army("no_commander").0 as i64).get("commander").unwrap().is_null());
    // The requisition's House line.
    let cands = arr(loyal.get("requisition").unwrap(), "candidates");
    let row = find(&cands, "city", b.cities[0].0 as i64);
    assert_eq!(s_of(row.get("house").unwrap(), "text"), format!("{} is Restless. It takes no orders.", b.label));
    assert_eq!(s_of(find(&cands, "city", a.cities[0].0 as i64).get("house").unwrap(), "text"), format!("{}: no limit.", a.label));
    assert!(find(&cands, "city", st.capital.0 as i64).get("house").unwrap().is_null());
    // The commander's change goes by the command `set_commander` (client agent A's), whose
    // payload the screen sends: {c, army, person}.

    // F. The unit panel: a commander's unit, a head's unit, and the great person's Found a House.
    let u = g.person_unit(person("commander_loyal")).unwrap();
    let grp = q(&s, &format!(r#"{{"q":"group","units":[{}]}}"#, u.0));
    let pu = arr(&grp, "units")[0].get("person").unwrap().clone();
    assert_eq!(s_of(&pu, "post"), format!("commander of the {}", g.armies[army("loyal")].name));
    assert_eq!(s_of(&pu, "state"), "Loyal");
    assert!(grp.get("held").unwrap().is_null());
    let hu = g.person_unit(person("head_a")).unwrap();
    let grp = q(&s, &format!(r#"{{"q":"group","units":[{}]}}"#, hu.0));
    assert_eq!(s_of(arr(&grp, "units")[0].get("person").unwrap(), "post"), format!("head of {}", a.label));
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
    let houses = st.houses.len();
    assert!(warning.starts_with(&format!("Adopting {}: {houses} Houses secede with ", ffh_play::names::display(d, god_king))), "{warning}");
    assert!(warning.contains("Held by armies: "), "{warning}");
    assert_eq!(s_of(p, "persons_button"), "Adopt and face the secession");
    let lines = arr(p, "persons");
    assert_eq!(lines.len(), houses, "one line a House: {p}");
    for l in &lines {
        assert!(s_of(l, "line").starts_with("House "), "{l}");
    }
    // Another government: no warning.
    let republic = d.t::<CivicType>("CIVIC_REPUBLIC");
    if g.player_can_do_civics(st.human, Some(republic)) {
        let r = q(&s, &format!(r#"{{"q":"civics","with":[{}]}}"#, republic.idx()));
        assert!(r.get("proposed").unwrap().get("persons_warning").is_none());
    }
}

#[test]
fn the_log_tags_persons_and_secessions_for_the_alerts() {
    let mut e = LogEntry::new(3, Some(0), "Hrolf demands: A House. 40 turns.".into());
    e.mark = LogMark::Person;
    e.tag = LogTag::Person(PersonId(7));
    e.at = Some((4, 5));
    let j = ffh_play::report::entry_fields(Json::Obj(Default::default()), &e);
    assert_eq!(j.get("kind").and_then(Json::as_str), Some("person"));
    assert_eq!(j.get("person").and_then(Json::as_i64), Some(7));
    assert_eq!(j.get("mark").and_then(Json::as_str), Some("person"));
    for text in [
        "House Arne secedes from the Bannor with Prespur and Elam.",
        "House Arne and House Bel secede from the Bannor with Prespur.",
        "Civil war: House Arne secedes from the Bannor with Prespur.",
        "Hrolf, 2nd Army goes over to the Elohim of House Arne with 4 units.",
        "Hrolf, 2nd Army seizes Vallus and secedes from the Bannor with 4 units.",
        "House Arne is released from the Bannor as a vassal with Prespur.",
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
