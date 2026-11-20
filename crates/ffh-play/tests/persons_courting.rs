//! Courting on the Houses' screens (docs/PERSONS.md, "The fourth build", rule 10), on the staged
//! scenes (`ffh-play stage persons|persons-direct`) through the session's queries: a courted
//! House's card (its courtships in the rules' lines, one past two thirds marked, its guard, the
//! Guard button with the rules' preview, pressed through its command), the Court's line that needs
//! attention for it and the badge that counts it, the keywords where courting shows; a House of
//! another realm's card (`house {house}`: your courtship, your requirement against its leader's
//! attitude, the Court button and what it replaces, your channel once it courts); the buttons'
//! refusals in the rules' words; the trade table with a House (gifts only, your courtship or none,
//! the requirement at each better attitude); and `persons-direct`'s courtship, whose next liege turn
//! splits the House toward the human.

mod common;

use std::sync::Arc;

use ffh_core::culture::{ChannelOrder, ChannelTarget};
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::Command;
use ffh_play::json::Json;
use ffh_play::query::{Query, QueryErr};
use ffh_play::stage::StagedPersons;
use ffh_play::{Session, SessionOptions};

fn staged(name: &str, direct: bool) -> (common::TempDir, StagedPersons, Session) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::persons(&dir.0, direct).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    let s = Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    (dir, st, s)
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::as_arr).unwrap_or(&[])
}

fn s<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("no string `{key}` in {j}"))
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or(-1)
}

fn house_of(st: &StagedPersons, role: &str) -> HouseId {
    st.houses.iter().find(|x| x.role == role).unwrap().id
}

fn order(ses: &Session, effect: CultureEffect, h: HouseId) -> ChannelOrder {
    ChannelOrder { action: ses.g.culture_action(effect).expect("the action"), target: ChannelTarget::House(h), religion: None }
}

/// The reminders' keys of a payload.
fn keys(j: &Json) -> Vec<String> {
    arr(j, "reminders").iter().map(|r| s(r, "key").to_string()).collect()
}

/// The command a button carries, as the client sends it.
fn button_cmd(ses: &Session, b: &Json) -> Command {
    Command::from_json(ses.g.d, b.get("cmd").expect("a command")).expect("the button's command decodes")
}

/// `the Sheaim's favour is 130 of 188, a split in ~6 turns` as the rules' prediction reads.
fn prediction(cv: &ffh_core::persons::CourtshipView) -> String {
    match cv.turns {
        Some(1) => ", a split next turn".into(),
        Some(t) => format!(", a split in ~{t} turns"),
        None if cv.gain > 0 => format!(", settling near {}", cv.settles / 100),
        None => ", fading".into(),
    }
}

/// House `pleased` courted past two thirds by the neighbour: its card's line (marked), the Guard
/// button (the human's channel courts already: what it replaces; the rules' preview), the Court's
/// attention line and the badge, the Court's keywords; the button's command sets the guard, and
/// the card then says its guard and offers no button.
#[test]
fn a_courted_houses_card_its_attention_line_and_the_guard_button() {
    let (_dir, st, mut ses) = staged("persons-courting-card", false);
    let h = ses.human;
    let p = house_of(&st, "pleased");
    let label = ses.g.house_label(p);
    let r = ses.query(&Query::Court).unwrap();
    let card = arr(&r, "houses").iter().find(|x| int(x, "id") == p.0 as i64).unwrap().clone();
    let cvs = ses.g.house_courtships(p);
    assert_eq!(cvs.len(), 1);
    let cv = &cvs[0];
    let courtships = arr(&card, "courtships");
    assert_eq!(courtships.len(), 1);
    assert_eq!(int(&courtships[0], "courter"), st.neighbour.0 as i64);
    assert_eq!(s(&courtships[0], "line"), ffh_play::names::resolve_keys(ses.g.d, &cv.line));
    assert_eq!(courtships[0].get("attention").and_then(Json::as_bool), Some(true), "past two thirds");
    assert!(card.get("guard").unwrap().is_null());
    // The Guard button: the human's channel courts the neighbour's House, so the label says what
    // it replaces; its line is the rules' preview of the order.
    let guard = order(&ses, CultureEffect::Guard, p);
    let running = ses.g.players[h].culture_channel.as_ref().map(|ch| ch.order()).expect("the human's channel");
    let b = card.get("channel").unwrap();
    assert_eq!(s(b, "kind"), "guard");
    assert_eq!(s(b, "label"), format!("Guard (replaces {})", ses.g.culture_channel_name(&running)));
    assert_eq!(s(b, "line"), ses.g.culture_channel_preview(h, &guard).line);
    assert!(b.get("refusal").unwrap().is_null());
    // The Houses no one courts: no button.
    for x in arr(&r, "houses").iter().filter(|x| int(x, "id") != p.0 as i64) {
        assert!(x.get("channel").unwrap().is_null() && arr(x, "courtships").is_empty(), "{x}");
    }
    // The line that needs attention, its button the Culture screen; the badge counts it.
    let att: Vec<&Json> = arr(&r, "attention").iter().filter(|a| s(a, "kind") == "courted").collect();
    assert_eq!(att.len(), 1);
    let by = ses.g.player_get_civilization_short_description(st.neighbour);
    assert_eq!(s(att[0], "text"), format!("{label}: the {by}'s favour is {} of {}{}.", cv.favour / 100, cv.requirement.total, prediction(cv)));
    assert_eq!(int(att[0], "house"), p.0 as i64);
    assert_eq!(s(att[0].get("button").unwrap().get("act").unwrap(), "kind"), "culture");
    let court = ses.g.court_view(h);
    let furious = court.houses.iter().filter(|hv| hv.state == ffh_core::persons::LoyaltyState::Furious).count() as i64;
    let requests = court.houses.iter().filter(|hv| hv.demand.as_ref().is_some_and(|dm| dm.turns_left <= ffh_core::persons::DEMAND_WARNING_TURNS)).count() as i64;
    assert_eq!(int(&r, "badge"), furious + 1 + requests, "the courtship past two thirds counts");
    let view = ses.query(&Query::View).unwrap();
    assert_eq!(int(&view, "court_attention"), int(&r, "badge"));
    // The keywords where courting shows.
    let k = keys(&r);
    assert!(["KEYWORD_COURT", "KEYWORD_GUARD", "KEYWORD_CHANNEL"].iter().all(|x| k.contains(&x.to_string())), "{k:?}");

    // The button's command: the guard runs; the card says so, without a button.
    let cmd = button_cmd(&ses, b);
    ses.execute(&cmd).unwrap();
    assert_eq!(ses.g.players[h].culture_channel.as_ref().map(|ch| ch.order()), Some(guard));
    let r = ses.query(&Query::Court).unwrap();
    let card = arr(&r, "houses").iter().find(|x| int(x, "id") == p.0 as i64).unwrap().clone();
    assert_eq!(s(&card, "guard"), ses.g.house_guard_view(p).expect("guarded").line);
    assert!(card.get("channel").unwrap().is_null(), "guarded already");
}

/// A House of another realm's card (`house {house}`): who it is, your courtship of it (your own
/// line), your requirement against its leader's attitude, and while your channel courts it that
/// channel instead of a button; stopped, the Court button with the rules' preview; with another
/// channel running, what it replaces. One of yours is the Court's card. A House that does not
/// stand is gone.
#[test]
fn a_foreign_houses_card_and_its_court_button() {
    let (_dir, st, mut ses) = staged("persons-courting-foreign", false);
    let h = ses.human;
    let n = house_of(&st, "neighbour");
    let label = ses.g.house_label(n);
    let r = ses.query(&Query::House { house: n }).unwrap();
    assert_eq!(r.get("own").and_then(Json::as_bool), Some(false));
    let x = r.get("house").unwrap();
    assert_eq!(s(x, "label"), label);
    assert_eq!(s(x, "vassal"), format!("Vassal of the {}.", ses.g.player_get_civilization_short_description(st.neighbour)));
    assert_eq!(s(x, "state"), ses.g.house_state(n).name());
    let mine = arr(x, "courtships").iter().find(|c| int(c, "courter") == h.0 as i64).expect("your courtship").clone();
    assert_eq!(mine.get("own").and_then(Json::as_bool), Some(true));
    assert_eq!(Some(s(&mine, "line").to_string()), ses.g.house_courting_line(n, h));
    assert_eq!(s(x, "requirement"), ffh_play::view::persons::requirement_line(&ses.g, n, h));
    let court = order(&ses, CultureEffect::Court, n);
    assert_eq!(s(x, "courting"), format!("Your channel: {}.", ses.g.culture_channel_name(&court)));
    assert!(x.get("channel").unwrap().is_null(), "it courts it already");
    let k = keys(&r);
    assert!(k.contains(&"KEYWORD_COURT".to_string()) && k.contains(&"KEYWORD_HOUSE".to_string()), "{k:?}");
    // Every city of it the human's map shows.
    assert!(!arr(x, "cities").is_empty());

    // The channel stopped: the Court button, the rules' preview.
    ses.execute(&Command::CultureChannel { action: None, target: None, religion: None }).unwrap();
    let r = ses.query(&Query::House { house: n }).unwrap();
    let b = r.get("house").unwrap().get("channel").unwrap().clone();
    assert_eq!((s(&b, "kind"), s(&b, "label")), ("court", "Court"));
    assert_eq!(s(&b, "line"), ses.g.culture_channel_preview(h, &court).line);
    assert!(b.get("refusal").unwrap().is_null());
    // Another channel running: what it replaces.
    let p = house_of(&st, "pleased");
    let guard = order(&ses, CultureEffect::Guard, p);
    ses.g.player_set_culture_channel(h, Some(guard)).unwrap();
    let r = ses.query(&Query::House { house: n }).unwrap();
    let b = r.get("house").unwrap().get("channel").unwrap().clone();
    assert_eq!(s(&b, "label"), format!("Court (replaces {})", ses.g.culture_channel_name(&guard)));
    ses.execute(&button_cmd(&ses, &b)).unwrap();
    assert_eq!(ses.g.players[h].culture_channel.as_ref().map(|ch| ch.order()), Some(court), "the button's command courts it");

    // One of yours: the Court's card. A House that does not stand: gone.
    let r = ses.query(&Query::House { house: p }).unwrap();
    assert_eq!(r.get("own").and_then(Json::as_bool), Some(true));
    assert_eq!(int(r.get("house").unwrap(), "id"), p.0 as i64);
    assert!(matches!(ses.query(&Query::House { house: HouseId(9999) }), Err(QueryErr::Gone)));
}

/// A button the rules refuse carries their reason: a Guard of a House not yours, a Court of one of
/// your realm.
#[test]
fn a_channel_button_carries_the_rules_refusal() {
    let (_dir, st, ses) = staged("persons-courting-refusal", false);
    let v = ffh_play::view::Viewer::new(&ses.g, ses.human, false);
    let n = house_of(&st, "neighbour");
    let p = house_of(&st, "pleased");
    for (o, why) in [
        (order(&ses, CultureEffect::Guard, n), format!("{} is not your House.", ses.g.house_label(n))),
        (order(&ses, CultureEffect::Court, p), format!("{} is of your realm.", ses.g.house_label(p))),
    ] {
        let b = ffh_play::view::persons::channel_button(&v, &o);
        assert_eq!(s(&b, "refusal"), why);
        assert_eq!(ses.g.culture_channel_check(ses.human, &o).err(), Some(why));
        assert!(b.get("line").unwrap().is_null(), "a refused order says nothing of its effect");
    }
}

/// The trade table with a House of another realm: it takes gifts and gives nothing (one bare
/// line), your courtship's line or `You do not court House X.`, and the requirement against its
/// leader's attitude toward you at the attitude now and each better one up to Friendly (the rules'
/// `house_requirement_steps`); the Court keyword.
#[test]
fn the_trade_table_with_a_house() {
    let (_dir, st, mut ses) = staged("persons-courting-trade", false);
    let h = ses.human;
    let n = house_of(&st, "neighbour");
    let hp = ses.g.houses[n].player;
    let label = ses.g.house_label(n);
    let tr = ses.query(&Query::Trade { with: hp }).unwrap();
    let x = tr.get("house").unwrap();
    assert_eq!(s(x, "takes"), format!("{label} takes gifts of gold and resources and gives nothing."));
    assert!(arr(&tr, "they_give").is_empty());
    assert_eq!(x.get("courts").and_then(Json::as_bool), Some(true));
    assert_eq!(Some(s(x, "courting").to_string()), ses.g.house_courting_line(n, h));
    let steps = ses.g.house_requirement_steps(n, h);
    let word = |a: AttitudeType| ffh_play::names::display(ses.g.d, a).into_owned();
    let mut want = format!("Its leader's attitude toward you: {}. Requirement: {}", word(steps[0].0), steps[0].1);
    for &(a, t) in &steps[1..] {
        want.push_str(&format!("; at {} {t}", word(a)));
    }
    want.push('.');
    assert_eq!(s(x, "requirement"), want);
    assert_eq!(steps.last().unwrap().0, AttitudeType::ATTITUDE_FRIENDLY, "up to Friendly");
    assert_eq!(steps[0].1, ses.g.house_requirement(n, h).total, "the first is the requirement now");
    assert!(keys(x).contains(&"KEYWORD_COURT".to_string()));
    // Not courting it (no channel, no favour left): said so.
    ses.execute(&Command::CultureChannel { action: None, target: None, religion: None }).unwrap();
    ses.g.houses[n].favour.retain(|f| f.courter != h);
    let tr = ses.query(&Query::Trade { with: hp }).unwrap();
    let x = tr.get("house").unwrap();
    assert_eq!(x.get("courts").and_then(Json::as_bool), Some(false));
    assert_eq!(s(x, "courting"), format!("You do not court {label}."));
    // The human's own House: no such lines.
    let own = ses.g.houses[house_of(&st, "friendly")].player;
    let tr = ses.query(&Query::Trade { with: own }).unwrap();
    let x = tr.get("house").unwrap();
    assert!(x.get("takes").unwrap().is_null() && x.get("courting").unwrap().is_null() && x.get("requirement").unwrap().is_null());
}

/// `persons-direct`: the human's courtship of the neighbour's House is one liege turn from a split
/// (the rules' prediction says so; the half that would go is not held; a leader is free), and the
/// House's liege turn, with the inflow the human's cities divert, splits it toward the human: a new
/// House of the human's realm with those cities, the old House keeping its seat; the human is told
/// with the House's mark, tagged with the new House.
#[test]
fn persons_direct_splits_at_the_next_liege_turn() {
    let (_dir, st, mut ses) = staged("persons-courting-direct", true);
    let h = ses.human;
    let n = house_of(&st, "neighbour");
    let label = ses.g.house_label(n);
    let cv = ses.g.house_courtship_view(n, h);
    assert_eq!(cv.turns, Some(1), "{}", cv.line);
    assert!(cv.favour < 100 * cv.requirement.total, "short of the requirement");
    let split = cv.split.clone().expect("a split");
    assert!(!split.whole && split.held.is_empty() && split.going.len() >= 1, "{split:?}");
    assert!(ses.g.house_cities(n).len() >= 3, "two cities or more besides its seat");
    let going: Vec<PlotIdx> = split.going.iter().map(|&c| ses.g.city_plot(c)).collect();
    let seat = ses.g.house_seat(n).map(|c| ses.g.city_plot(c)).expect("a seat");
    // The liege's turn, with what the human's cities divert stored as their turn stores it.
    let g = &mut ses.g;
    g.players[h].culture_inflow = g.player_culture_inflow_now(h);
    g.player_persons_turn(st.neighbour);
    let at = |g: &ffh_core::game::Game, q: PlotIdx| g.map.plots[q].plot_city.expect("a city");
    let np = g.cities[at(g, going[0])].owner;
    let nh = g.player_house(np).expect("a new House");
    assert_eq!(g.houses[nh].owner, h, "of the human's realm");
    assert!(going.iter().all(|&q| g.cities[at(g, q)].owner == np));
    assert_eq!(g.cities[at(g, seat)].owner, g.houses[n].player, "the old House keeps its seat");
    let e = g.log.entries.iter().rev().find(|e| e.text.starts_with(&format!("{label} splits toward you: "))).expect("the human is told");
    assert_eq!(e.player, Some(h.0));
    assert_eq!(e.mark, ffh_core::log::LogMark::House);
    assert_eq!(e.tag, ffh_core::log::LogTag::House(nh));
}
