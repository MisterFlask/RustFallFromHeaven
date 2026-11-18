//! Persons of Note through the human's session (docs/PERSONS.md, "UI", parts A, B and J; the
//! protocol in docs/HUMAN.md), on the staged scene (`ffh-play stage persons`): the `court` payload's
//! parts and the view's count of what needs attention, a grant whose preview is what then happens,
//! each refusal with the rules' reason, naming an heir, the command set, and the purity of the two
//! queries.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_core::persons::{GrantTo, LoyaltyState, Post};
use ffh_play::command::{CmdErr, Command, COMMAND_NAMES};
use ffh_play::json::Json;
use ffh_play::query::{Query, QueryErr};
use ffh_play::stage::StagedPersons;
use ffh_play::{Session, SessionOptions};

fn staged(name: &str) -> (common::TempDir, StagedPersons, Session) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::persons(&dir.0, false).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    let s = Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    (dir, st, s)
}

fn strs(j: &Json, key: &str) -> Vec<String> {
    j.get(key).and_then(Json::as_arr).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::as_arr).unwrap_or(&[])
}

fn s<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or("")
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or(-1)
}

fn house_id(st: &StagedPersons, role: &str) -> HouseId {
    st.houses.iter().find(|x| x.role == role).unwrap().id
}

fn person_id(st: &StagedPersons, role: &str) -> PersonId {
    st.persons.iter().find(|x| x.0 == role).unwrap().1
}

fn army_id(st: &StagedPersons, role: &str) -> ArmyId {
    st.armies.iter().find(|x| x.0 == role).unwrap().1
}

fn cannot(r: Result<ffh_play::command::CmdOk, CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The `court` payload: the header, the lines that need attention with their buttons, the Crown,
/// the four Houses, the persons without a House, the reminders; the view's count matches.
#[test]
fn the_court_shows_every_part() {
    let (_dir, st, ses) = staged("persons-court");
    let r = ses.query(&Query::Court).unwrap();
    let header = strs(&r, "header");
    assert_eq!(header[0], "Government: Aristocracy.");
    assert_eq!(header[1], "A House takes 1.5 orders a city every 30 turns.");
    assert!(arr(&r, "empty").is_empty());
    assert_eq!(r.get("grants"), Some(&Json::Null));

    // Needs attention: the heir, the Disloyal House and commander, the Crown's grantable cities.
    let att = arr(&r, "attention");
    let kinds: Vec<&str> = att.iter().map(|a| s(a, "kind")).collect();
    let d_label = &st.houses.iter().find(|x| x.role == "D").unwrap().label;
    let heir = att.iter().find(|a| s(a, "kind") == "heir").expect("an heir to name");
    assert_eq!(s(heir, "text"), format!("{d_label} has no head."));
    let b = heir.get("button").unwrap();
    assert_eq!(s(b, "label"), "Name an heir");
    assert_eq!(int(b.get("act").unwrap(), "house"), house_id(&st, "D").0 as i64);
    let c_label = &st.houses.iter().find(|x| x.role == "C").unwrap().label;
    assert!(att.iter().any(|a| s(a, "text").starts_with(&format!("{c_label}: Disloyal. Will secede with ")) && s(a, "text").ends_with(" in 8 turns.")), "{att:?}");
    assert_eq!(kinds.iter().filter(|&&k| k == "disloyal").count(), 2, "the House and the commander: {kinds:?}");
    assert!(att.iter().any(|a| s(a, "text") == "3 cities of the Crown can be granted."), "{att:?}");
    assert_eq!(int(&r, "badge"), 3);
    let view = ses.query(&Query::View).unwrap();
    assert_eq!(int(&view, "court_attention"), 3, "the menu's count is the Court's");

    // The Crown: the capital with its reason, the three grantable cities.
    let crown = arr(&r, "crown");
    assert_eq!(crown.len(), 4);
    let cap = crown.iter().find(|c| c.get("capital").and_then(Json::as_bool) == Some(true)).unwrap();
    assert_eq!(s(cap, "refusal"), "The capital is the Crown's.");
    assert_eq!(crown.iter().filter(|c| c.get("grantable").and_then(Json::as_bool) == Some(true)).count(), 3);

    // The Houses.
    let houses = arr(&r, "houses");
    assert_eq!(houses.len(), 4);
    let by = |role: &str| houses.iter().find(|x| int(x, "id") == house_id(&st, role).0 as i64).unwrap();
    let a = by("A");
    assert_eq!(s(a, "state"), "Devoted");
    assert_eq!(s(a, "orders"), "Orders: no limit.");
    assert!(s(a, "maintenance").starts_with("Maintenance saved: ") && s(a, "maintenance").ends_with(" gold a turn."));
    assert!(s(a, "seat").starts_with("Seat: "));
    assert_eq!(arr(a, "cities").len(), 3);
    let head = a.get("head").unwrap();
    assert_eq!(int(head, "id"), person_id(&st, "head_a").0 as i64);
    let demand = head.get("demand").unwrap();
    assert_eq!(s(demand, "kind"), "wonder");
    assert!(s(demand, "text").starts_with("Demands: ") && s(demand, "note").starts_with("In production: ") && s(demand, "note").ends_with(" The term does not run."), "{demand:?}");
    assert!(!arr(a, "parts").is_empty() && s(a, "total").starts_with("Loyalty: +"));
    let b = by("B");
    assert_eq!(s(b, "state"), "Restless");
    assert_eq!(s(b, "orders"), "Orders: none.");
    assert!(strs(b, "parts").iter().any(|p| p.starts_with("Demand refused: -3 (")), "{:?}", strs(b, "parts"));
    let c = by("C");
    assert_eq!(s(c, "state"), "Disloyal");
    assert_eq!(int(c, "countdown_turns"), 8);
    assert!(s(c, "outcome").ends_with(" in 8 turns."), "{c:?}");
    assert!(s(c, "held").starts_with("Held by armies: "));
    assert_eq!(s(c.get("release").unwrap(), "refusal"), "A Disloyal House cannot be released.");
    let dh = by("D");
    assert_eq!(dh.get("head"), Some(&Json::Null));
    assert_eq!(dh.get("needs_heir").and_then(Json::as_bool), Some(true));
    assert_eq!(arr(dh, "heir_candidates").len(), 3);
    assert_eq!(s(dh, "maintenance"), "Maintenance saved: none while the seat is empty.");

    // The persons without a House: the commanders and the persons with no post.
    let persons = arr(&r, "persons");
    let p = |role: &str| persons.iter().find(|x| int(x, "id") == person_id(&st, role).0 as i64).unwrap_or_else(|| panic!("{role}"));
    let loyal = p("commander_loyal");
    assert_eq!(s(loyal, "post"), format!("Commander of the {}.", ses.g.armies[army_id(&st, "loyal")].name));
    assert_eq!(s(loyal, "state"), "Loyal");
    assert!(s(loyal, "where").starts_with("Hero: level "), "{}", s(loyal, "where"));
    let dis = p("commander_disloyal");
    assert_eq!(s(dis, "state"), "Disloyal");
    assert!(s(dis, "outcome").contains(" sworn units in "), "{}", s(dis, "outcome"));
    assert_eq!(s(p("no_post"), "post"), "No post.");
    let dm = p("no_post").get("demand").unwrap();
    assert_eq!(s(dm, "kind"), "post");
    assert_eq!(s(dm.get("button").unwrap(), "label"), "Grant a city");

    // Part J: the eight keywords' reminders.
    let keys: Vec<&str> = arr(&r, "reminders").iter().map(|x| s(x, "key")).collect();
    for k in ["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_DEVOTED", "KEYWORD_LOYAL", "KEYWORD_RESTLESS", "KEYWORD_DISLOYAL", "KEYWORD_SWORN", "KEYWORD_CLAIMANT"] {
        assert!(keys.contains(&k), "{k}: {keys:?}");
    }
}

/// The amount of a preview line (`...: -2 (30 turns).`), 0 without one.
fn amount(line: &str) -> i32 {
    let Some(i) = line.rfind(": ") else { return 0 };
    let rest = &line[i + 2..];
    let n: String = rest.chars().take_while(|c| *c == '+' || *c == '-' || c.is_ascii_digit()).collect();
    n.trim_start_matches('+').parse().unwrap_or(0)
}

/// The grant list of the contested city: the claimants first, each card's preview; granting it to
/// the first claimant does what its preview said, to the grantee and to each snubbed claimant.
#[test]
fn a_grant_does_what_its_preview_says() {
    let (_dir, st, mut ses) = staged("persons-grant");
    let h = ses.human;
    let c = st.taken;
    let r = ses.query(&Query::Grant { city: c }).unwrap();
    let name = ffh_play::view::city_name(&ses.g, c);
    assert_eq!(s(&r, "title"), format!("Grant {name}"));
    assert_eq!(s(&r, "note"), "A grant is for good.");
    assert!(s(&r, "baseline").starts_with("Held by the Crown: city-count maintenance "));
    assert_eq!(r.get("refusal"), Some(&Json::Null));
    let cands = arr(&r, "candidates");
    let claims: Vec<bool> = cands.iter().map(|x| x.get("claimant").and_then(Json::as_bool).unwrap()).collect();
    assert_eq!(claims.iter().filter(|&&x| x).count(), 3, "Houses A and B and the commander who took it: {claims:?}");
    assert!(claims[..3].iter().all(|&x| x) && claims[3..].iter().all(|&x| !x), "claimants first: {claims:?}");
    assert!(cands.iter().any(|x| s(x, "kind") == "great_person"), "the great person can found a House");
    let keys: Vec<&str> = arr(&r, "reminders").iter().map(|x| s(x, "key")).collect();
    assert!(keys.contains(&"KEYWORD_CLAIMANT"), "{keys:?}");

    // The first House among the claimants.
    let cand = cands.iter().find(|x| x.get("claimant").and_then(Json::as_bool) == Some(true) && s(x, "kind") == "house").unwrap();
    let grantee = HouseId(int(cand.get("to").unwrap(), "house") as u32);
    let label = s(cand, "house").to_string();
    assert_eq!(s(cand, "confirm"), format!("Grant {name} to {label}? This cannot be undone."));
    let lines = strs(cand, "lines");
    assert!(lines.iter().any(|l| l.starts_with("Maintenance:")) && lines.iter().any(|l| l.starts_with(&format!("Trait in {name}: "))) && lines.iter().any(|l| l.starts_with(&format!("Orders in {name}: "))), "{lines:?}");
    // Each subject's loyalty and state before, and what the preview says changes.
    let g = &ses.g;
    let subjects: Vec<(String, i32, Option<String>, i32)> = arr(cand, "effects")
        .iter()
        .map(|e| {
            let subject = s(e, "subject").to_string();
            let delta: i32 = strs(e, "lines").iter().map(|l| amount(l)).sum();
            let after = e.get("state_after").and_then(Json::as_str).map(str::to_string);
            let before = g.players[h].houses.iter().find(|&&x| g.house_label(x) == subject).map(|&x| g.house_loyalty(x)).or_else(|| g.players[h].persons.iter().find(|&&x| g.person_house(x).is_none() && g.person_subject(x) == subject).map(|&x| g.person_loyalty(x))).expect("a known subject");
            (subject, before, after, delta)
        })
        .collect();
    assert!(subjects.iter().any(|x| x.0 == label && x.3 == 1), "the grantee: +1: {subjects:?}");
    assert!(subjects.iter().filter(|x| x.0 != label).all(|x| x.3 < 0), "every snubbed claimant loses: {subjects:?}");
    // Grant.
    let out = ses.execute(&Command::GrantCity { city: c, to: GrantTo::House(grantee) }).unwrap();
    let ffh_play::command::CmdOk::Applied { log, .. } = out else { panic!("{out:?}") };
    assert!(log.iter().any(|l| l.text == format!("{name} granted to {label}.")), "the toast's message: {:?}", log.iter().map(|l| &l.text).collect::<Vec<_>>());
    let g = &ses.g;
    assert_eq!(g.city_house(c), Some(grantee));
    for (subject, before, after, delta) in &subjects {
        let (now, state) = match g.players[h].houses.iter().find(|&&x| g.house_label(x) == *subject) {
            Some(&x) => (g.house_loyalty(x), g.house_state(x)),
            None => {
                let &x = g.players[h].persons.iter().find(|&&x| g.person_subject(x) == *subject).unwrap();
                (g.person_loyalty(x), g.person_state(x))
            }
        };
        assert_eq!(now - before, *delta, "{subject}: the preview's change");
        if let Some(a) = after {
            assert_eq!(state.name(), a, "{subject}: the preview's state");
        }
    }
    // The Court: the city is no longer the Crown's.
    let r = ses.query(&Query::Court).unwrap();
    assert!(!arr(&r, "crown").iter().any(|x| int(x, "city") == c.0 as i64));
    // The grant list of a House's city answers its reason.
    let r = ses.query(&Query::Grant { city: c }).unwrap();
    assert_eq!(s(&r, "refusal"), "The city is a House's. A grant is for good.");
    assert!(arr(&r, "candidates").is_empty());
}

/// Every command of the system refuses with the rules' own reason, and changes nothing.
#[test]
fn refusals_carry_the_rules_reasons() {
    let (_dir, st, mut ses) = staged("persons-refusals");
    let before = ses.g.save();
    let house_a = house_id(&st, "A");
    // Grants.
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.capital, to: GrantTo::House(house_a) })), "The capital is the Crown's.");
    let a_city = ses.g.houses[house_a].cities[0];
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: a_city, to: GrantTo::House(house_id(&st, "B")) })), "The city is a House's. A grant is for good.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::Person(person_id(&st, "head_a")) })), "A head is granted cities through their House.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::GreatPerson(ses.g.players[ses.human].units[0]) })), "Not a great person of yours.");
    assert_eq!(ses.execute(&Command::GrantCity { city: st.neighbour_near, to: GrantTo::House(house_a) }), Err(CmdErr::NotOwner));
    let r = ses.query(&Query::Grant { city: st.capital }).unwrap();
    assert_eq!(s(&r, "refusal"), "The capital is the Crown's.");
    assert_eq!(ses.query(&Query::Grant { city: st.neighbour_near }), Err(QueryErr::NotOwner));
    // Heirs.
    assert_eq!(cannot(ses.execute(&Command::NameHeir { house: house_a, person: person_id(&st, "candidate_1") })), "The seat is not empty.");
    assert_eq!(cannot(ses.execute(&Command::NameHeir { house: house_id(&st, "D"), person: person_id(&st, "commander_loyal") })), "The heir must hold no post.");
    // Commanders.
    assert_eq!(cannot(ses.execute(&Command::SetCommander { army: army_id(&st, "loyal"), person: person_id(&st, "no_post") })), "Not with this army.");
    assert_eq!(cannot(ses.execute(&Command::SetCommander { army: army_id(&st, "loyal"), person: person_id(&st, "head_a") })), "A House's head does not command.");
    // Release.
    assert_eq!(cannot(ses.execute(&Command::ReleaseHouse { house: house_id(&st, "C") })), "A Disloyal House cannot be released.");
    assert_eq!(cannot(ses.execute(&Command::ReleaseHouse { house: house_id(&st, "D") })), "The seat is empty.");
    assert!(before == ses.g.save(), "a refusal changed the game");
}

/// Naming an heir from the Court: the seat is taken, the line that needed it goes, the count falls.
#[test]
fn naming_an_heir() {
    let (_dir, st, mut ses) = staged("persons-heir");
    let d = house_id(&st, "D");
    let x = person_id(&st, "candidate_2");
    ses.execute(&Command::NameHeir { house: d, person: x }).unwrap();
    assert_eq!(ses.g.houses[d].head, Some(x));
    assert_eq!(ses.g.persons[x].post, Post::Head(d));
    let r = ses.query(&Query::Court).unwrap();
    assert!(!arr(&r, "attention").iter().any(|a| s(a, "kind") == "heir"));
    assert_eq!(int(&r, "badge"), 2);
    let house = arr(&r, "houses").iter().find(|h| int(h, "id") == d.0 as i64).unwrap();
    assert_eq!(int(house.get("head").unwrap(), "id"), x.0 as i64);
    assert_eq!(ses.g.house_state(d), LoyaltyState::of(ses.g.house_loyalty(d)));
}

/// The four commands are in the protocol, decode from their JSON and refuse an ambiguous grantee.
#[test]
fn the_command_set() {
    for n in ["grant_city", "name_heir", "set_commander", "release_house"] {
        assert!(COMMAND_NAMES.contains(&n), "{n}");
    }
    let d = ffh_data::global();
    let j = Json::parse(r#"{"c":"grant_city","city":3,"unit":7}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::GrantCity { city: CityId(3), to: GrantTo::GreatPerson(UnitId(7)) });
    let j = Json::parse(r#"{"c":"grant_city","city":3,"house":1,"person":2}"#).unwrap();
    assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))));
    let j = Json::parse(r#"{"c":"grant_city","city":3}"#).unwrap();
    assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))));
}

/// `court` and every city's `grant` leave the game as it was (PLAY_DESIGN 2.5).
#[test]
fn the_queries_are_pure() {
    let (_dir, _st, ses) = staged("persons-pure");
    let before = ses.g.save();
    ses.query(&Query::Court).unwrap();
    for c in ses.g.players[ses.human].cities.clone() {
        ses.query(&Query::Grant { city: c }).unwrap();
    }
    ses.query(&Query::View).unwrap();
    assert!(before == ses.g.save(), "a query changed the game");
}
