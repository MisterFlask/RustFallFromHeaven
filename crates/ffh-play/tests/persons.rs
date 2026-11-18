//! Persons of Note through the human's session (docs/PERSONS.md, "UI", parts A, B and J; "The
//! second build"; the protocol in docs/HUMAN.md), on the staged scene (`ffh-play stage persons`):
//! the `court` payload's parts and the view's count of what needs attention, a grant whose preview
//! is what then happens, each refusal with the rules' reason, the tribute and freeing commands, an
//! offer to a House, the command set, and the purity of the two queries.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_core::persons::{GrantTo, LoyaltyState};
use ffh_core::structs::TradeData;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command, COMMAND_NAMES};
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

fn cannot(r: Result<CmdOk, CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The `court` payload: the header, the lines that need attention with their buttons, the Crown,
/// the Houses, the persons without a House, the reminders; the view's count matches.
#[test]
fn the_court_shows_every_part() {
    let (_dir, st, ses) = staged("persons-court");
    let r = ses.query(&Query::Court).unwrap();
    let header = strs(&r, "header");
    assert_eq!(header[0], "Government: Aristocracy.");
    assert_eq!(header[1], "Tribute of a new House: 20%.");
    assert_eq!(header[2], "The Crown holds 4 cities; its limit is 5.", "the Crown's limit (rule 14)");
    assert!(arr(&r, "empty").is_empty());
    assert_eq!(r.get("grants"), Some(&Json::Null));
    assert_eq!(int(&r, "tribute_default"), 20);

    // Needs attention: the Furious House and commander, the Crown's grantable cities.
    let att = arr(&r, "attention");
    let kinds: Vec<&str> = att.iter().map(|a| s(a, "kind")).collect();
    let f_label = &st.houses.iter().find(|x| x.role == "furious").unwrap().label;
    assert!(att.iter().any(|a| s(a, "text").starts_with(&format!("{f_label}: Furious. Will break away with ")) && s(a, "text").contains(" in 8 turns: war.")), "{att:?}");
    assert_eq!(kinds.iter().filter(|&&k| k == "furious").count(), 2, "the House and the commander: {kinds:?}");
    assert!(att.iter().any(|a| s(a, "text") == "3 cities of the Crown can be granted."), "{att:?}");
    let view = ses.query(&Query::View).unwrap();
    assert!(int(&r, "badge") >= 2);
    assert_eq!(int(&view, "court_attention"), int(&r, "badge"), "the menu's count is the Court's");

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
    let a = by("friendly");
    assert_eq!(s(a, "state"), "Friendly");
    assert!(s(a, "tribute").starts_with("Tribute 0%"), "{}", s(a, "tribute"));
    assert_eq!(s(a, "levy"), "Takes levies.");
    assert!(s(a, "seat").starts_with("Capital: "));
    assert_eq!(arr(a, "cities").len(), 2);
    assert_eq!(arr(a, "tribute_at").len(), 6, "0 to 50 by 10");
    assert!(strs(a, "parts").iter().any(|p| p == "Tribute 0%: +1"), "{:?}", strs(a, "parts"));
    let head = a.get("head").unwrap();
    assert_eq!(int(head, "id"), person_id(&st, "head_friendly").0 as i64);
    assert!(!arr(a, "parts").is_empty() && s(a, "total").starts_with("Loyalty: +"));
    let b = by("pleased");
    assert!(s(b, "tribute").starts_with("Tribute 10%"));
    let v = by("annoyed");
    assert_eq!(s(v, "state"), "Annoyed");
    assert_eq!(s(v, "levy"), format!("{} is Annoyed. It takes no levy.", s(v, "label")));
    assert!(strs(v, "parts").iter().any(|p| p.starts_with("Capitulated: -4 (")), "{:?}", strs(v, "parts"));
    let c = by("furious");
    assert_eq!(s(c, "state"), "Furious");
    assert_eq!(int(c, "countdown_turns"), 8);
    assert!(s(c, "outcome").contains(" in 8 turns: war."), "{c:?}");
    assert!(s(c, "held").contains(": an army of ours stands in "), "{}", s(c, "held"));
    assert_eq!(c.get("free").and_then(|f| f.get("ok")).and_then(Json::as_bool), Some(true));

    // The persons without a House: the commanders and the person with no post.
    let persons = arr(&r, "persons");
    let p = |role: &str| persons.iter().find(|x| int(x, "id") == person_id(&st, role).0 as i64).unwrap_or_else(|| panic!("{role}"));
    let loyal = p("commander_loyal");
    assert_eq!(s(loyal, "post"), format!("Commander of the {}.", ses.g.armies[army_id(&st, "loyal")].name));
    assert!(s(loyal, "where").starts_with("Hero: level "), "{}", s(loyal, "where"));
    let fur = p("commander_furious");
    assert_eq!(s(fur, "state"), "Furious");
    let o = s(fur, "outcome");
    assert!([" will seize ", " will join ", " will resign "].iter().any(|w| o.contains(w)), "{o}");
    assert_eq!(s(p("no_post"), "post"), "No post.");

    // Part J: the keywords' reminders.
    let keys: Vec<&str> = arr(&r, "reminders").iter().map(|x| s(x, "key")).collect();
    for k in ["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_FRIENDLY", "KEYWORD_PLEASED", "KEYWORD_CAUTIOUS", "KEYWORD_ANNOYED", "KEYWORD_FURIOUS", "KEYWORD_SWORN", "KEYWORD_CLAIMANT"] {
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
/// the claimant House moves the city to the House's player and does what its preview said, to
/// the grantee and to each snubbed claimant.
#[test]
fn a_grant_does_what_its_preview_says() {
    let (_dir, st, mut ses) = staged("persons-grant");
    let h = ses.human;
    let c = st.taken;
    let plot = ses.g.city_plot(c);
    let r = ses.query(&Query::Grant { city: c }).unwrap();
    let name = ffh_play::view::city_name(&ses.g, c);
    assert_eq!(s(&r, "title"), format!("Grant {name}"));
    assert_eq!(s(&r, "note"), "A grant is for good.");
    assert!(s(&r, "baseline").starts_with("Held by the Crown: its maintenance "));
    assert_eq!(r.get("refusal"), Some(&Json::Null));
    let cands = arr(&r, "candidates");
    let claims: Vec<bool> = cands.iter().map(|x| x.get("claimant").and_then(Json::as_bool).unwrap()).collect();
    let n = claims.iter().filter(|&&x| x).count();
    assert!(n >= 2, "House `pleased` and the commander who took it: {claims:?}");
    assert!(claims[..n].iter().all(|&x| x) && claims[n..].iter().all(|&x| !x), "claimants first: {claims:?}");
    assert!(cands.iter().any(|x| s(x, "kind") == "great_person"), "the great person can found a House");

    let grantee = house_id(&st, "pleased");
    let cand = cands.iter().find(|x| x.get("to").and_then(|t| t.get("house")).and_then(Json::as_i64) == Some(grantee.0 as i64)).unwrap();
    let label = s(cand, "house").to_string();
    assert_eq!(s(cand, "confirm"), format!("Grant {name} to {label}? This cannot be undone."));
    let lines = strs(cand, "lines");
    assert!(lines.iter().any(|l| l.starts_with("Your maintenance: ")) && lines.iter().any(|l| l.contains(" pays a tribute of 10% ")), "{lines:?}");
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
    let CmdOk::Applied { log, .. } = out else { panic!("{out:?}") };
    assert!(log.iter().any(|l| l.text == format!("{name} granted to {label}.")), "the toast's message: {:?}", log.iter().map(|l| &l.text).collect::<Vec<_>>());
    let g = &ses.g;
    let now = g.map.plots[plot].plot_city.unwrap();
    assert_eq!(g.cities[now].owner, g.houses[grantee].player, "the city is the House's player's");
    assert_eq!(g.city_house(now), Some(grantee));
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
    // The Court: the city is no longer the Crown's; the grant list of a city not ours: NotOwner.
    let r = ses.query(&Query::Court).unwrap();
    assert!(!arr(&r, "crown").iter().any(|x| int(x, "city") == now.0 as i64));
    assert_eq!(ses.query(&Query::Grant { city: now }), Err(QueryErr::NotOwner));
}

/// Every command of the system refuses with the rules' own reason, and changes nothing.
#[test]
fn refusals_carry_the_rules_reasons() {
    let (_dir, st, mut ses) = staged("persons-refusals");
    let before = ses.g.save();
    let house_a = house_id(&st, "friendly");
    // Grants.
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.capital, to: GrantTo::House(house_a) })), "The capital is the Crown's.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::Person(person_id(&st, "head_friendly")) })), "A head is granted cities through their House.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::Person(person_id(&st, "no_post")) })), "A hero cannot head a House.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::GreatPerson(ses.g.players[ses.human].units[0]) })), "Not a great person of yours.");
    let r = ses.query(&Query::Grant { city: st.capital }).unwrap();
    assert_eq!(s(&r, "refusal"), "The capital is the Crown's.");
    // Commanders.
    assert_eq!(cannot(ses.execute(&Command::SetCommander { army: army_id(&st, "loyal"), person: person_id(&st, "no_post") })), "Not with this army.");
    assert_eq!(cannot(ses.execute(&Command::SetCommander { army: army_id(&st, "loyal"), person: person_id(&st, "head_friendly") })), "A House's head does not command.");
    // Tribute and freeing.
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: house_a, pct: 15 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: house_id(&st, "neighbour"), pct: 10 })), "Not a House of yours.");
    assert_eq!(cannot(ses.execute(&Command::SetTributeDefault { pct: 60 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::FreeHouse { house: house_id(&st, "neighbour") })), "Not a House of yours.");
    assert!(before == ses.g.save(), "a refusal changed the game");
}

/// The tribute commands set the House's tribute (its loyalty part follows) and the default; Free
/// ends the vassalage at peace.
#[test]
fn tribute_and_freeing() {
    let (_dir, st, mut ses) = staged("persons-tribute");
    let h = ses.human;
    let a = house_id(&st, "friendly");
    let loyalty = ses.g.house_loyalty(a);
    ses.execute(&Command::SetHouseTribute { house: a, pct: 30 }).unwrap();
    assert_eq!(ses.g.houses[a].tribute, 30);
    assert_eq!(ses.g.house_loyalty(a), loyalty - 3, "0% is +1, 30% is -2");
    ses.execute(&Command::SetTributeDefault { pct: 40 }).unwrap();
    assert_eq!(ses.g.players[h].house_tribute, 40);
    let hp = ses.g.houses[a].player;
    ses.execute(&Command::FreeHouse { house: a }).unwrap();
    let g = &ses.g;
    assert!(!g.houses.contains(a));
    assert_eq!(g.players[hp].house, None);
    let (ht, pt) = (g.players[hp].team, g.players[h].team);
    assert!(!g.teams[ht].vassal[pt] && !g.at_war(ht, pt), "freed at peace");
    common::check::check(g);
}

/// An offer to the neighbour's House: below its price refused (and noted), at its price it
/// secedes, at war with its old liege.
#[test]
fn an_offer_to_a_house() {
    let (_dir, st, mut ses) = staged("persons-offer");
    let h = ses.human;
    let n = house_id(&st, "neighbour");
    let hp = ses.g.houses[n].player;
    let price = ses.g.house_bribe_price(n).expect("a price");
    ses.g.players[h].gold = price + 100;
    let gold = |data: i32| TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false };
    let secede = TradeData { item_type: TradeableItem::TRADE_SECEDE, data: 0, offering: false, hidden: false };
    if price > 1 {
        let out = ses.execute(&Command::ProposeDeal { to: hp, give: vec![gold(price - 1)], get: vec![secede.clone()] }).unwrap();
        assert!(matches!(out, CmdOk::Applied { accepted: Some(false), .. }), "{out:?}");
        assert!(ses.g.houses[n].standing.mods.iter().any(|m| m.cause.label().starts_with("Offered ")), "the offer is noted");
    }
    let price = ses.g.house_bribe_price(n).expect("a price");
    let out = ses.execute(&Command::ProposeDeal { to: hp, give: vec![gold(price.max(1))], get: vec![secede] }).unwrap();
    assert!(matches!(out, CmdOk::Applied { accepted: Some(true), .. }), "{out:?}");
    let g = &ses.g;
    assert!(!g.houses.contains(n), "the House broke away");
    let (ht, nt) = (g.players[hp].team, g.players[st.neighbour].team);
    assert!(g.at_war(ht, nt), "at war with its old liege");
    assert!(!g.teams[ht].vassal[nt]);
    common::check::check(g);
}

/// The commands are in the protocol, decode from their JSON and refuse an ambiguous grantee.
#[test]
fn the_command_set() {
    for n in ["grant_city", "house_tribute", "set_commander", "free_house", "tribute_default"] {
        assert!(COMMAND_NAMES.contains(&n), "{n}");
    }
    let d = ffh_data::global();
    let j = Json::parse(r#"{"c":"grant_city","city":3,"unit":7}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::GrantCity { city: CityId(3), to: GrantTo::GreatPerson(UnitId(7)) });
    let j = Json::parse(r#"{"c":"grant_city","city":3,"house":1,"person":2}"#).unwrap();
    assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))));
    let j = Json::parse(r#"{"c":"house_tribute","house":1,"pct":30}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::SetHouseTribute { house: HouseId(1), pct: 30 });
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
    let _ = LoyaltyState::Cautious;
}
