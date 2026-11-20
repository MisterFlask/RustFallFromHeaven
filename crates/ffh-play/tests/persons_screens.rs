//! Houses in the existing screens (docs/PERSONS.md, "UI", parts C to I; "The second build" and
//! "The third build"), on the staged scenes (`ffh-play stage persons|persons-direct`) through the
//! session's queries: the House marks of the cities on the map (the liege sees the attitude, the
//! others the shield only), the Crown cities in the Governors screen, the Armies screen (no
//! commanders) and the levy's House lines among the requisition's candidates, the unit panel (no
//! person, no Found a House), the civics screen's warning of a direct-rule civic, the war and peace
//! previews, and the log's tags for the alerts.

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
fn the_screens_show_houses() {
    let (_dir, st, s) = load("persons-screens", false);
    let g = &s.g;
    let house = |role: &str| st.houses.iter().find(|x| x.role == role).unwrap();
    let army = |role: &str| st.armies.iter().find(|x| x.0 == role).unwrap().1;

    // G. The map's marks: a House's city carries its shield, and its liege sees its attitude.
    let view = q(&s, r#"{"q":"view"}"#);
    let cities = arr(&view, "cities");
    let f = house("furious");
    let mark = find(&cities, "id", f.cities[0].0 as i64).get("house").unwrap().clone();
    assert_eq!(s_of(&mark, "state"), "Furious");
    assert!(mark.get("turns").is_none(), "no countdown");
    assert_eq!(mark.get("colour").and_then(Json::as_i64), Some(g.houses[f.id].color as i64));
    assert!(find(&cities, "id", st.crown[0].0 as i64).get("house").unwrap().is_null(), "a Crown city");
    let n = house("neighbour");
    if let Some(x) = cities.iter().find(|x| n.cities.iter().any(|c| x.get("id").and_then(Json::as_i64) == Some(c.0 as i64))) {
        let m = x.get("house").expect("another's House city shows its shield");
        assert!(m.get("state").unwrap().is_null(), "only the liege sees the attitude: {m}");
    }
    // H. The Houses in brief, for the alerts' portraits.
    assert!(view.get("persons").is_none(), "no cast");
    let brief = arr(&view, "houses");
    let a = house("friendly");
    let row = find(&brief, "id", a.id.0 as i64);
    assert_eq!(row.get("leader").and_then(Json::as_i64), g.house_leader(a.id).map(|l| l.idx() as i64));
    assert_eq!(s_of(&row, "label"), a.label);

    // E. The Governors screen's rows: the human's cities are the Crown's.
    let gs = q(&s, r#"{"q":"governors"}"#);
    let rows = arr(&gs, "cities");
    assert!(find(&rows, "city", st.capital.0 as i64).get("house").unwrap().is_null());

    // D. The Armies screen: no commander, no person with an army.
    let ar = q(&s, r#"{"q":"armies"}"#);
    let armies = arr(&ar, "armies");
    let main = find(&armies, "id", army("main").0 as i64);
    assert!(main.get("commander").is_none() && main.get("persons_with").is_none(), "{main}");
    // The levy: the Houses' cities among the requisition's candidates, with their House line.
    let cands = arr(main.get("requisition").unwrap(), "candidates");
    let row = find(&cands, "city", a.cities[0].0 as i64);
    assert_eq!(s_of(row.get("house").unwrap(), "text"), "Takes levies. Each levy: -1 (20 turns).", "{row}");
    let v = house("annoyed");
    let row = find(&cands, "city", v.cities[0].0 as i64);
    assert_eq!(s_of(row.get("house").unwrap(), "text"), "Takes no levy.");
    assert_eq!(s_of(row.get("house").unwrap(), "reason"), format!("{} is Annoyed. It takes no levy.", v.label));
    assert!(find(&cands, "city", st.capital.0 as i64).get("house").unwrap().is_null());

    // F. The unit panel: no person line, no Found a House.
    let u = g.army_units(army("main"))[0];
    let grp = q(&s, &format!(r#"{{"q":"group","units":[{}]}}"#, u.0));
    let unit = &arr(&grp, "units")[0];
    assert!(unit.get("person").is_none() && unit.get("found_house").is_none(), "{unit}");

    // I. The war dialog's preview, the peace preview (at peace: none).
    let pl = q(&s, r#"{"q":"players"}"#);
    let nb = find(&arr(&pl, "players"), "id", st.neighbour.0 as i64);
    assert!(nb.get("war_preview").and_then(Json::as_arr).is_some(), "{nb}");
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, st.neighbour.0));
    assert_eq!(tr.get("peace_preview").and_then(Json::as_arr).map(|a| a.len()), Some(0));
    // A House is a party on the diplomacy screen: it gives nothing (it takes gifts).
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, g.houses[n.id].player.0));
    let theirs = arr(&tr, "they_give");
    let names: Vec<&str> = theirs.iter().filter_map(|x| x.get("item").and_then(Json::as_i64)).map(|i| TradeableItem::NAMES[i as usize]).collect();
    assert!(names.is_empty(), "{names:?}");
    // A religion's preview list is there for every religion.
    let rl = q(&s, r#"{"q":"religions"}"#);
    for x in arr(&rl, "religions") {
        assert!(x.get("persons").and_then(Json::as_arr).is_some());
    }
}

/// The map's marks and the colours (docs/PERSONS.md, "The second build", rules 1 and 13): a
/// House's city carries its shield and its liege (whose colour it is drawn in) for every viewer,
/// the attitude mark and countdown for its liege only; the players list says whose colour each
/// player is drawn in, and a House's record (its attitude and parts for its liege only).
#[test]
fn a_house_is_drawn_in_its_lieges_colour_with_its_shield() {
    let (_dir, st, s) = load("persons-colours", false);
    let g = &s.g;
    let house = |role: &str| st.houses.iter().find(|x| x.role == role).unwrap();
    let (f, n) = (house("furious"), house("neighbour"));
    // The human's view: its own House's city, with the attitude; the neighbour's House's city,
    // its shield and liege only.
    let cities = ffh_play::view::cities(&ffh_play::view::Viewer::new(g, st.human, false));
    let cities = cities.as_arr().unwrap();
    let fm = find(cities, "id", f.cities[0].0 as i64).get("house").unwrap().clone();
    assert_eq!(fm.get("liege").and_then(Json::as_i64), Some(st.human.0 as i64));
    assert_eq!(s_of(&fm, "label"), f.label);
    assert_eq!(s_of(&fm, "state"), "Furious");
    assert!(fm.get("turns").is_none(), "no countdown");
    let nc = n.cities[0];
    assert!(g.cities[nc].owner == n.player);
    // (Seen with debug reveal: the scene's human need not have revealed it.)
    let revealed = ffh_play::view::cities(&ffh_play::view::Viewer::new(g, st.human, true));
    let nm = find(revealed.as_arr().unwrap(), "id", nc.0 as i64).get("house").expect("the neighbour's House's city shows its shield").clone();
    assert_eq!(nm.get("liege").and_then(Json::as_i64), Some(st.neighbour.0 as i64));
    assert_eq!(nm.get("colour").and_then(Json::as_i64), Some(g.houses[n.id].color as i64));
    assert!(nm.get("state").unwrap().is_null() && nm.get("turns").is_none(), "{nm}");
    // The neighbour's view: its own House's city with the attitude, the human's Houses' (where it
    // knows them) without.
    let theirs = ffh_play::view::cities(&ffh_play::view::Viewer::new(g, st.neighbour, true));
    let theirs = theirs.as_arr().unwrap();
    let nm = find(theirs, "id", nc.0 as i64).get("house").unwrap().clone();
    assert_eq!(s_of(&nm, "state"), g.house_state(n.id).name());
    for x in theirs {
        if let Some(m) = x.get("house").filter(|m| !m.is_null()) {
            if m.get("liege").and_then(Json::as_i64) == Some(st.human.0 as i64) {
                assert!(m.get("state").unwrap().is_null(), "only the liege sees the attitude: {m}");
            }
        }
    }
    // The players list: whose colour each is drawn in, and the Houses' records.
    let pl = q(&s, r#"{"q":"players"}"#);
    let players = arr(&pl, "players");
    let row = find(&players, "id", f.player.0 as i64);
    assert_eq!(row.get("color_player").and_then(Json::as_i64), Some(st.human.0 as i64));
    let hb = row.get("house").unwrap();
    assert_eq!(s_of(hb, "label"), f.label);
    assert_eq!(s_of(hb, "state"), "Furious");
    assert!(hb.get("turns").is_none(), "no countdown");
    assert!(s_of(hb, "outcome").contains(" issues an ultimatum at your next turn. Refused, it breaks away"), "{hb}");
    assert!(s_of(hb, "loyalty").starts_with("Loyalty: "));
    assert!(arr(hb, "parts").iter().any(|x| x.as_str().is_some_and(|t| t.starts_with("Tribute "))), "{hb}");
    // Its vassal deal cannot be cancelled (the rules): no count runs for it.
    for d in arr(&row, "deals") {
        assert_eq!(d.get("cancelable").and_then(Json::as_bool), Some(false), "{d}");
        assert_eq!(d.get("cancel_in").and_then(Json::as_i64), Some(0), "{d}");
    }
    let row = find(&players, "id", n.player.0 as i64);
    assert_eq!(row.get("color_player").and_then(Json::as_i64), Some(st.neighbour.0 as i64));
    let hb = row.get("house").unwrap();
    assert_eq!(hb.get("liege").and_then(Json::as_i64), Some(st.neighbour.0 as i64));
    assert_eq!(s_of(hb, "liege_name"), g.player_get_civilization_short_description(st.neighbour));
    assert!(hb.get("state").is_none() && hb.get("parts").is_none(), "another's House: no attitude toward its liege shown: {hb}");
    let row = find(&players, "id", st.neighbour.0 as i64);
    assert_eq!(row.get("color_player").and_then(Json::as_i64), Some(st.neighbour.0 as i64));
    assert!(row.get("house").unwrap().is_null());
    // The alerts' Houses in brief: each of the human's, with its leader.
    let view = q(&s, r#"{"q":"view"}"#);
    let brief = arr(&view, "houses");
    assert_eq!(brief.len(), g.players[st.human].houses.len());
    assert_eq!(s_of(&find(&brief, "id", f.id.0 as i64), "label"), f.label);
}

/// A House on the diplomacy screen (docs/PERSONS.md, "The second build", rule 13; "The fourth
/// build", rule 3): it takes gifts, gold and resources, and gives nothing; the rest is its liege's
/// (`Trades through the X.`); a gift is accepted and anything asked back refused, in the rules'
/// words; the human's own House trades nothing (tribute).
#[test]
fn a_houses_trade_table_and_its_answers() {
    let (_dir, st, s) = load("persons-trade", false);
    let n = st.houses.iter().find(|x| x.role == "neighbour").unwrap();
    let hp = n.player;
    let liege = s.g.player_get_civilization_short_description(st.neighbour);
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, hp.0));
    assert_eq!(tr.get("contact").and_then(Json::as_bool), Some(true));
    let house = tr.get("house").unwrap();
    assert_eq!(house.get("own").and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(house, "trades_through"), format!("Trades through the {liege}."));
    assert!(tr.get("troops").unwrap().is_null());
    let names = |side: &str| -> Vec<&'static str> { arr(&tr, side).iter().filter_map(|x| x.get("item").and_then(Json::as_i64)).map(|i| TradeableItem::NAMES[i as usize]).collect() };
    assert!(names("they_give").is_empty(), "it gives nothing");
    assert!(names("we_give").iter().all(|x| ["TRADE_GOLD", "TRADE_RESOURCES"].contains(x)), "{:?}", names("we_give"));
    // It takes gifts and gives nothing; the human's courtship of it: the rules' line (the stage:
    // the human courts it); its requirement against its leader's attitude (the fourth build, rule
    // 10; `tests/persons_courting.rs` reads them closer).
    assert_eq!(s_of(house, "takes"), format!("{} takes gifts of gold and resources and gives nothing.", n.label));
    assert_eq!(house.get("courting").and_then(Json::as_str).map(str::to_string), s.g.house_courting_line(n.id, st.human));
    assert!(s_of(house, "requirement").starts_with("Its leader's attitude toward you: "), "{house}");
    // A gift is accepted; anything asked back is refused.
    let gift = q(&s, &format!(r#"{{"q":"trade_answer","with":{},"give":[{{"item":"TRADE_GOLD","data":1}}],"get":[]}}"#, hp.0));
    assert_eq!(gift.get("accept").and_then(Json::as_bool), Some(true), "{gift}");
    let ask = q(&s, &format!(r#"{{"q":"trade_answer","with":{},"give":[{{"item":"TRADE_GOLD","data":1}}],"get":[{{"item":"TRADE_GOLD","data":1}}]}}"#, hp.0));
    assert_eq!(ask.get("accept").and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(&ask, "why"), format!("{} takes gifts only", n.label));
    // The human's own House: no gift, it pays tribute.
    let own = st.houses.iter().find(|x| x.role == "friendly").unwrap();
    let tr = q(&s, &format!(r#"{{"q":"trade","with":{}}}"#, own.player.0));
    let house = tr.get("house").unwrap();
    assert_eq!(house.get("own").and_then(Json::as_bool), Some(true));
    assert_eq!(s_of(house, "line"), format!("{} pays its liege by tribute.", own.label));
    assert!(arr(&tr, "we_give").is_empty() && arr(&tr, "they_give").is_empty());
}

/// The previews of the acts (docs/PERSONS.md, "UI", part I) come from the rules' `persons_preview`
/// as lines worded with the attitude scale.
#[test]
fn the_previews_and_the_unit_panel() {
    let (_dir, st, s) = load("persons-previews", false);
    let g = &s.g;
    let v = ffh_play::view::Viewer::new(g, st.human, false);
    let states = ["Friendly", "Pleased", "Cautious", "Annoyed", "Furious"];
    let war = ffh_play::view::house_marks::act_preview(&v, &ffh_core::persons::PersonsAct::DeclareWar(g.players[st.neighbour].team));
    let raze = ffh_play::view::house_marks::act_preview(&v, &ffh_core::persons::PersonsAct::Raze(st.crown[1]));
    for list in [&war, &raze] {
        for l in list.as_arr().unwrap() {
            let line = s_of(l, "line");
            if let Some(after) = l.get("state_after").and_then(Json::as_str) {
                assert!(states.contains(&after) && line.ends_with(&format!(" Becomes {after}.")), "{line}");
            }
        }
    }
    assert_eq!(war.as_arr().unwrap().len(), g.persons_preview(st.human, &ffh_core::persons::PersonsAct::DeclareWar(g.players[st.neighbour].team)).len());
    // Every subject is a House of the human's.
    let labels: Vec<String> = g.players[st.human].houses.iter().map(|&h| g.house_label(h)).collect();
    for list in [&war, &raze] {
        for l in list.as_arr().unwrap() {
            assert!(labels.iter().any(|x| x == s_of(l, "subject")), "{l}");
        }
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
    assert!(warning.starts_with(&format!("Adopting {}: {houses} Houses break away, taking ", ffh_play::names::display(d, god_king))), "{warning}");
    assert!(warning.ends_with(" cities, each at war with you."), "{warning}");
    // A line per House: the cities it takes, and the contest for each city where an army of
    // yours stands (rule 7: held when your army's units outnumber the House's).
    let ph = arr(p, "persons_houses");
    let per_house: Vec<&str> = ph.iter().filter_map(Json::as_str).collect();
    assert_eq!(per_house.len(), houses, "{p}");
    for &h in &g.players[st.human].houses {
        let label = g.house_label(h);
        let line = per_house.iter().find(|l| l.starts_with(&format!("{label}: ")) || l.starts_with(&format!("{label} (a vassal): ")) || l.starts_with(&format!("{label} ends."))).unwrap_or_else(|| panic!("{label}: {per_house:?}"));
        let (going, held) = g.house_cities_held(h);
        for c in going {
            assert!(line.contains(&ffh_play::view::city_name(g, c)), "{line}");
        }
        for c in &held {
            assert!(line.contains(&format!("{} returns to you: your army's ", ffh_play::view::city_name(g, *c))), "{line}");
        }
    }
    assert!(per_house.iter().any(|l| l.contains(" returns to you: your army's 2 units outnumber its ")), "the held city: {per_house:?}");
    assert!(per_house.iter().any(|l| l.contains(" goes with it: its 2 units against your army's 1.")), "the seat contested: {per_house:?}");
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
fn the_log_tags_houses_and_break_aways_for_the_alerts() {
    let mut e = LogEntry::new(3, Some(0), "House Arne requests: Torrin granted to House Arne. 40 turns.".into());
    e.mark = LogMark::House;
    e.tag = LogTag::House(HouseId(7));
    e.at = Some((4, 5));
    let j = ffh_play::report::entry_fields(Json::Obj(Default::default()), &e);
    assert_eq!(j.get("kind").and_then(Json::as_str), Some("house"));
    assert_eq!(j.get("house").and_then(Json::as_i64), Some(7));
    assert_eq!(j.get("mark").and_then(Json::as_str), Some("house"));
    // Read back (a reloaded log keeps its mark and tag).
    let mut back = LogEntry::new(3, Some(0), e.text.clone());
    ffh_play::report::entry_fields_from(&j, &mut back);
    assert_eq!((back.mark, back.tag), (LogMark::House, LogTag::House(HouseId(7))));
    for text in [
        "House Arne breaks away from the Bannor, taking Prespur and Elam: war.",
        "Hippus breaks away from the Bannor, taking Prespur. Bought by the Elohim: war.",
        "Hippus breaks away from the Bannor, taking Prespur. Bought by you: war.",
        "Hippus breaks away from the Bannor, taking Prespur, as a vassal of the Elohim: war.",
        "Hippus breaks away from the Bannor, taking Prespur, as your vassal: war.",
        "Civil war: House Arne breaks away from the Bannor, taking Prespur: war.",
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
