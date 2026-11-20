//! Houses through the human's session (docs/PERSONS.md, "The third build", on "The second build",
//! rule 13; "UI", parts A, B and J; the protocol in docs/HUMAN.md), on the staged scene (`ffh-play
//! stage persons`): the `court` payload (the realm: the Crown and its limit, the lines that need
//! attention, a card per House with its leader, request, tribute selector, levy and Free) and the
//! view's count of what needs attention; the tribute command and its preview; a grant whose preview
//! is what then happens, and the founding of a realm's first House; the levy list's statuses on the
//! Armies screen; a Crown city's Governance line; each refusal with the rules' reason; freeing; an
//! gift to a House; the civilization's cultural channel; the command set; and the purity of the queries.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_core::persons::{GrantTo, LoyaltyState, DEMAND_REFUSED, LEVIED, TRIBUTE_LAG_TURNS};
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

fn bool_of(j: &Json, key: &str) -> bool {
    j.get(key).and_then(Json::as_bool).unwrap_or_else(|| panic!("no bool `{key}` in {j}"))
}

fn house_id(st: &StagedPersons, role: &str) -> HouseId {
    st.houses.iter().find(|x| x.role == role).unwrap().id
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

/// `+3`, `-2`, `0`.
fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// `Takes levies. Each levy: -1 (20 turns).` and how many levies a House at `loyalty` takes before
/// it takes none, by the rules' numbers (`Annoyed after 2 levies.`).
fn levies_until(loyalty: i32) -> String {
    let k = (1..100).find(|k| LoyaltyState::of(loyalty + k * LEVIED.0).withholds()).expect("a levy moves loyalty");
    format!("{} after {k} {}.", LoyaltyState::of(loyalty + k * LEVIED.0).name(), if k == 1 { "levy" } else { "levies" })
}

/// The Court's card of House `h`.
fn house_card(court: &Json, h: HouseId) -> Json {
    arr(court, "houses").iter().find(|x| int(x, "id") == h.0 as i64).unwrap_or_else(|| panic!("no card of House {}", h.0)).clone()
}

/// The `court` payload: the header (the government, the Crown's limit, the default tribute), the
/// lines that need attention in their order with their buttons, the Crown, the Houses' cards (the
/// leader, the request), the keywords; the view's count matches.
#[test]
fn the_court_shows_the_realm() {
    let (_dir, st, ses) = staged("persons-court");
    let g = &ses.g;
    let h = ses.human;
    let r = ses.query(&Query::Court).unwrap();
    assert_eq!(strs(&r, "header"), ["Government: Aristocracy."]);
    let crown_line = r.get("crown_line").unwrap();
    let text = s(crown_line, "text");
    assert_eq!(text, "The Crown holds 4 cities, 1 over its limit of 3.", "the Crown's limit (rule 14): its cost is the attention line's");
    assert!(bool_of(crown_line, "over"));
    assert!(r.get("realm").is_none(), "no size threshold (the third build, rule 1)");
    assert!(r.get("founding").unwrap().is_null(), "a realm with Houses founds none");
    assert!(arr(&r, "empty").is_empty());
    assert_eq!(r.get("grants"), Some(&Json::Null));
    assert_eq!(int(&r, "tribute_default"), 20);
    let steps: Vec<i64> = arr(&r, "tribute_steps").iter().filter_map(Json::as_i64).collect();
    assert_eq!(steps, [0, 10, 20, 30, 40, 50]);

    // Needs attention, in the contract's order: the Furious Houses (their ultimatums due), the House
    // that takes no levy, the Crown's city over its limit, the grantable cities.
    let att = arr(&r, "attention");
    let kinds: Vec<&str> = att.iter().map(|a| s(a, "kind")).collect();
    let at = |k: &str| kinds.iter().position(|&x| x == k).unwrap_or_else(|| panic!("no `{k}` line: {kinds:?}"));
    assert_eq!(kinds.iter().filter(|&&k| k == "furious").count(), 2, "Houses furious and large: {kinds:?}");
    assert!(at("furious") < at("levy") && at("levy") < at("crown_over") && at("crown_over") < at("grantable"), "{kinds:?}");
    let f_label = &st.houses.iter().find(|x| x.role == "furious").unwrap().label;
    assert!(att.iter().any(|a| s(a, "text").starts_with(&format!("{f_label}: Furious. Issues an ultimatum at your next turn. Refused, it breaks away, taking ")) && s(a, "text").contains(": war.")), "{att:?}");
    let v_label = &st.houses.iter().find(|x| x.role == "annoyed").unwrap().label;
    assert_eq!(s(&att[at("levy")], "text"), format!("{v_label} is Annoyed. It takes no levy."));
    let over = &att[at("crown_over")];
    let over_city = g.player_crown_over(h)[0];
    let over_name = ffh_play::view::city_name(g, over_city);
    assert!(s(over, "text").starts_with(&format!("{over_name} is over the Crown's limit of 3: maintenance +")) && s(over, "text").contains(" gold a turn."), "the cost, signed as a change in what you pay: {over:?}");
    let button = over.get("button").unwrap();
    assert_eq!(s(button, "label"), format!("Grant {over_name}"));
    assert_eq!(int(button.get("act").unwrap(), "city"), over_city.0 as i64);
    assert_eq!(s(&att[at("grantable")], "text"), "3 cities of the Crown can be granted.");
    let view = ses.query(&Query::View).unwrap();
    assert!(int(&r, "badge") >= 1);
    assert_eq!(int(&view, "court_attention"), int(&r, "badge"), "the menu's count is the Court's");

    // The Crown: the capital with its reason, the three grantable cities, the one over its limit.
    let crown = arr(&r, "crown");
    assert_eq!(crown.len(), 4);
    let cap = crown.iter().find(|c| bool_of(c, "capital")).unwrap();
    assert_eq!(s(cap, "refusal"), "The capital is the Crown's.");
    assert_eq!(crown.iter().filter(|c| bool_of(c, "grantable")).count(), 3);
    let overs: Vec<&Json> = crown.iter().filter(|c| bool_of(c, "over")).collect();
    assert_eq!(overs.len(), 1);
    assert_eq!(s(overs[0], "note"), "Over the Crown's limit.");

    // The Houses' cards.
    assert_eq!(arr(&r, "houses").len(), 5);
    let a = house_card(&r, house_id(&st, "friendly"));
    assert_eq!(s(&a, "state"), "Friendly");
    assert!(s(&a, "since").starts_with("Founded on turn "), "{}", s(&a, "since"));
    let a_loyalty = g.house_loyalty(house_id(&st, "friendly"));
    assert_eq!(s(a.get("levy").unwrap(), "text"), format!("Takes levies. Each levy: -1 (20 turns). {}", levies_until(a_loyalty)));
    assert!(bool_of(a.get("levy").unwrap(), "takes"));
    assert_eq!(arr(&a, "cities").len(), 2);
    assert!(a.get("size").is_none(), "no size part (the third build, rule 1)");
    assert!(strs(&a, "parts").iter().any(|p| p == "Tribute 0%: +1"), "{:?}", strs(&a, "parts"));
    assert!(s(&a, "total").starts_with("Loyalty: +"));
    // Its leader: its player's (portrait, name, title), and the ambition its first trait gives.
    assert!(a.get("head").is_none(), "no head: the House carries what its head carried");
    let ah = house_id(&st, "friendly");
    let leader = g.house_leader(ah).expect("a leader");
    assert_eq!(int(&a, "leader"), leader.idx() as i64);
    assert_eq!(s(&a, "leader_name"), ffh_core::persons::leader_name(g.d, leader));
    assert_eq!(s(&a, "ambition"), format!("Ambition: {}.", g.house_ambition(ah).name()));
    assert!(strs(&a, "record").iter().any(|l| l.contains("Founded at ")), "{:?}", strs(&a, "record"));
    // Its tribute: the line, and a rate for each step with the rules' numbers, once in force (the
    // House at rest at 0%: each step 5 turns away).
    let t = a.get("tribute").unwrap();
    assert_eq!((int(t, "pct"), int(t, "now"), int(t, "turns")), (0, 0, 0));
    assert!(s(t, "line").starts_with("Tribute 0%: "), "{}", s(t, "line"));
    for rate in arr(t, "rates") {
        let pct = int(rate, "pct") as i32;
        let loyalty = g.house_loyalty_at_tribute(ah, pct);
        assert_eq!(int(rate, "loyalty"), loyalty as i64);
        assert_eq!(int(rate, "part"), (1 - pct / 10) as i64, "the tribute part at {pct}%");
        assert_eq!(s(rate, "part_text"), format!("Tribute: {}", signed(1 - pct / 10)));
        let turns = pct / 10 * TRIBUTE_LAG_TURNS;
        assert_eq!(int(rate, "turns"), turns as i64);
        assert_eq!(s(rate, "when"), if turns == 0 { "In force.".to_string() } else { format!("In force in {turns} turns.") });
        assert_eq!(s(rate, "state"), LoyaltyState::of(loyalty).name());
        assert_eq!(bool_of(rate, "current"), pct == 0);
        assert_eq!(bool_of(rate, "in_force"), pct == 0);
        let gold = g.house_tribute_at(ah, pct);
        assert_eq!(s(rate, "estimate"), format!("Pays {gold} gold."), "gold only (the third build, rule 10)");
    }
    let v = house_card(&r, house_id(&st, "annoyed"));
    assert_eq!(s(&v, "state"), "Annoyed");
    assert!(s(&v, "since").starts_with("Capitulated on turn "), "{}", s(&v, "since"));
    assert_eq!(s(v.get("levy").unwrap(), "text"), format!("{} is Annoyed. It takes no levy.", s(&v, "label")));
    assert!(strs(&v, "parts").iter().any(|p| p.starts_with("Capitulated: -4 (")), "{:?}", strs(&v, "parts"));
    // The contract (the third build, rule 6): its line, its floor among the parts, the rates its
    // bound refuses barred in the tribute list.
    let until = g.houses[house_id(&st, "annoyed")].contract.expect("a contract").until;
    assert_eq!(s(&v, "contract"), format!("Contract until turn {until}: no tribute above 10%."));
    assert!(strs(&v, "parts").iter().any(|p| p.starts_with(&format!("Contract until turn {until}: at least Annoyed"))), "{:?}", strs(&v, "parts"));
    let why = format!("Your contract with {} forbids it until turn {until}.", s(&v, "label"));
    for rate in arr(v.get("tribute").unwrap(), "rates") {
        let barred = rate.get("barred").and_then(Json::as_str);
        assert_eq!(barred, (int(rate, "pct") > 10).then_some(why.as_str()), "{rate}");
    }
    // The split line (the third build, rule 4): none below 6 cities.
    let l = house_card(&r, house_id(&st, "large"));
    assert!(l.get("split").unwrap().is_null(), "5 cities: no split line");
    assert_eq!(s(&l, "state"), "Furious");
    let c = house_card(&r, house_id(&st, "furious"));
    assert_eq!(s(&c, "state"), "Furious");
    assert!(c.get("countdown_turns").is_none(), "no countdown");
    assert!(c.get("contract").unwrap().is_null());
    assert!(s(&c, "outcome").contains(" issues an ultimatum at your next turn. Refused, it breaks away, taking ") && s(&c, "outcome").contains(": war."), "{c:?}");
    // The contest for each city where an army of yours stands: the held one, the seat lost.
    assert!(s(&c, "outcome").contains(" returns to you: your army's 2 units outnumber its "), "the held city: {c:?}");
    assert!(s(&c, "outcome").contains(" goes with it: its 2 units against your army's 1."), "the seat contested: {c:?}");
    assert!(s(&c, "held").contains(" returns to you: your army's 2 units outnumber its ") && s(&c, "held").contains(" goes with it: "), "{}", s(&c, "held"));
    let chips = arr(&c, "cities");
    assert!(chips.iter().any(|x| bool_of(x, "held") && s(x, "hold").contains(" returns to you: ")), "{chips:?}");
    assert!(chips.iter().any(|x| !bool_of(x, "held") && s(x, "hold").contains(" goes with it: ")), "{chips:?}");
    assert_eq!(c.get("free").and_then(|f| f.get("ok")).and_then(Json::as_bool), Some(true));
    assert_eq!(s(c.get("free").unwrap(), "confirm"), format!("Free {f_label}? It is no longer your vassal, at peace with you. This cannot be undone."));

    // Every request with a term is a request (`Request: ...`) and says what its lapse does: the
    // rules' refusal, and the state the House would be in (the rules' state of its loyalty less the
    // refusal) when that changes; between requests, `No request.`
    let mut requests = 0;
    for card in arr(&r, "houses") {
        assert!(card.get("persons").is_none() && card.get("head").is_none());
        let dm = card.get("request").unwrap();
        if dm.get("kind").unwrap().is_null() {
            assert!(s(dm, "text").starts_with("No request."), "{dm}");
            continue;
        }
        requests += 1;
        assert!(s(dm, "text").starts_with("Request: "), "{dm}");
        let x = HouseId(int(card, "id") as u32);
        let loyalty = g.house_loyalty(x);
        assert_eq!(s(dm, "lapse"), format!("At its term: {} ({} turns)", DEMAND_REFUSED.0, DEMAND_REFUSED.1), "{dm}");
        let after = LoyaltyState::of(g.house_loyalty_floored(x, g.house_loyalty_raw(x) + DEMAND_REFUSED.0));
        let want = (after != g.house_state(x)).then(|| after.name());
        assert_eq!(dm.get("lapse_state").and_then(Json::as_str), want, "{dm}");
        // Furious is an ultimatum (the third build, rule 5).
        assert_eq!(dm.get("lapse_then").and_then(Json::as_str), (want == Some("Furious")).then_some("an ultimatum"), "{dm}");
        let _ = loyalty;
    }
    assert!(requests > 0, "the stage has requests with a term");
    assert!(r.get("persons").is_none(), "no persons");

    // Part J: the keywords (the loyalty states are the attitude words: none of their own).
    let keys: Vec<&str> = arr(&r, "reminders").iter().map(|x| s(x, "key")).collect();
    assert_eq!(keys, ["KEYWORD_HOUSE", "KEYWORD_CROWN", "KEYWORD_REQUEST", "KEYWORD_ULTIMATUM", "KEYWORD_TRIBUTE", "KEYWORD_LEVY", "KEYWORD_CLAIMANT"]);
    for st in LoyaltyState::ALL {
        assert!(ffh_data::global().ty::<KeywordType>(&format!("KEYWORD_{}", st.name().to_uppercase())).is_none(), "{} is no keyword", st.name());
    }
}

/// The tribute command and its preview: the rate's row says when it would be in force and what
/// the House's tribute part, total and state are then; after `house_tribute` the card says the
/// rate set and the rate in force (`Tribute 30%, now 0%: reaches 30% in 15 turns.`), its parts
/// still the rate in force's, the row marked; once the rate is in force the card says what the
/// row said. Setting the rate in force again stops it. The realm's default is set by
/// `tribute_default`.
#[test]
fn the_tribute_list_previews_its_change() {
    let (_dir, st, mut ses) = staged("persons-tribute-preview");
    let h = ses.human;
    for (role, pct) in [("friendly", 30), ("pleased", 50), ("annoyed", 0)] {
        let x = house_id(&st, role);
        let card = house_card(&ses.query(&Query::Court).unwrap(), x);
        let t = card.get("tribute").unwrap();
        let now = int(t, "now") as i32;
        let before_parts = strs(&card, "parts");
        let before_total = s(&card, "total").to_string();
        let rate = arr(t, "rates").iter().find(|r| int(r, "pct") == pct as i64).unwrap().clone();
        assert_eq!(bool_of(&rate, "changes"), s(&rate, "state") != s(&card, "state"));
        let turns = (pct - now).abs() / 10 * TRIBUTE_LAG_TURNS;
        assert_eq!(int(&rate, "turns"), turns as i64, "{role}: from rest, a step every 5 turns");
        assert_eq!(s(&rate, "when"), format!("In force in {turns} turns."));
        ses.execute(&Command::SetHouseTribute { house: x, pct }).unwrap();
        assert_eq!((ses.g.houses[x].tribute, ses.g.houses[x].tribute_now), (pct, now));
        // Set: the line says both rates; the parts and total are still the rate in force's.
        let card = house_card(&ses.query(&Query::Court).unwrap(), x);
        let t = card.get("tribute").unwrap();
        assert_eq!((int(t, "pct"), int(t, "now"), int(t, "turns")), (pct as i64, now as i64, turns as i64));
        assert!(s(t, "line").starts_with(&format!("Tribute {pct}%, now {now}%: reaches {pct}% in {turns} turns. ")), "{role}: {}", s(t, "line"));
        assert_eq!(strs(&card, "parts"), before_parts, "{role}: the parts follow the rate in force");
        assert_eq!(s(&card, "total"), before_total);
        let rows = arr(t, "rates");
        assert!(rows.iter().all(|r| bool_of(r, "current") == (int(r, "pct") == pct as i64)), "{role}: the rate set marked");
        assert!(rows.iter().all(|r| bool_of(r, "in_force") == (int(r, "pct") == now as i64)), "{role}: the rate in force");
        let set_row = rows.iter().find(|r| int(r, "pct") == pct as i64).unwrap();
        assert_eq!(s(set_row, "when"), format!("In force in {turns} turns."));
        assert_eq!(int(set_row, "loyalty"), int(&rate, "loyalty"), "{role}: the loyalty once in force is unchanged by the setting");
        let in_force_row = rows.iter().find(|r| int(r, "pct") == now as i64).unwrap();
        assert_eq!(s(in_force_row, "when"), "In force.");
        // Once in force (by hand): the card says what the row said.
        ses.g.houses[x].tribute_now = pct;
        let card = house_card(&ses.query(&Query::Court).unwrap(), x);
        let parts = strs(&card, "parts");
        let part = parts.iter().find(|p| p.starts_with("Tribute ")).unwrap();
        assert_eq!(*part, format!("Tribute {pct}%: {}", signed(int(&rate, "part") as i32)), "{role}: the part the row said");
        assert_eq!(s(&card, "total"), format!("Loyalty: {}.", signed(int(&rate, "loyalty") as i32)), "{role}: the total the row said");
        assert_eq!(s(&card, "state"), s(&rate, "state"), "{role}: the state the row said");
        assert!(s(card.get("tribute").unwrap(), "line").starts_with(&format!("Tribute {pct}%: ")), "the line without the rate in force once equal");
    }
    ses.execute(&Command::SetTributeDefault { pct: 40 }).unwrap();
    assert_eq!(ses.g.players[h].house_tribute, 40);
    assert_eq!(int(&ses.query(&Query::Court).unwrap(), "tribute_default"), 40);
}

/// The amount of a preview line (`...: -2 (30 turns).`), 0 without one.
fn amount(line: &str) -> i32 {
    let Some(i) = line.rfind(": ") else { return 0 };
    let rest = &line[i + 2..];
    let n: String = rest.chars().take_while(|c| *c == '+' || *c == '-' || c.is_ascii_digit()).collect();
    n.trim_start_matches('+').parse().unwrap_or(0)
}

/// The grant list of the contested city: the claimants first, the Houses only (a realm with a
/// House founds none); each card says what the grant does (the city moves to the House's realm, the maintenance, the
/// Crown back within its limit, the tribute); granting it to the claimant House moves the city to
/// the House's player and does what its preview said, to the grantee and to each snubbed claimant.
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
    assert!(s(r.get("crown_line").unwrap(), "text").starts_with("The Crown holds 4 cities, 1 over its limit of 3"));
    assert_eq!(r.get("refusal"), Some(&Json::Null));
    let cands = arr(&r, "candidates");
    let claims: Vec<bool> = cands.iter().map(|x| bool_of(x, "claimant")).collect();
    let n = claims.iter().filter(|&&x| x).count();
    assert!(n >= 1, "House `pleased` claims it: {claims:?}");
    assert!(claims[..n].iter().all(|&x| x) && claims[n..].iter().all(|&x| !x), "claimants first: {claims:?}");
    assert!(cands.iter().all(|x| s(x, "kind") == "house"), "Houses only");
    assert_eq!(cands.len(), 5, "the realm's five Houses");

    let grantee = house_id(&st, "pleased");
    let cand = cands.iter().find(|x| x.get("to").and_then(|t| t.get("house")).and_then(Json::as_i64) == Some(grantee.0 as i64)).unwrap();
    let label = s(cand, "house").to_string();
    assert_eq!(s(cand, "confirm"), format!("Grant {name} to {label}? This cannot be undone."));
    let lines = strs(cand, "lines");
    assert_eq!(lines[0], format!("Moves to {label}'s realm: its production and commerce are the House's."));
    // Its garrison, as the rules move it (the army that took the city stays the liege's).
    let (going, staying) = ses.g.city_grant_garrison(c);
    let garrison = strs(cand, "garrison");
    if going.is_empty() {
        assert_eq!(garrison[0], "No garrison goes with it.");
    } else {
        assert!(garrison[0].starts_with(&format!("Its garrison goes to {label}: ")), "{garrison:?}");
    }
    assert_eq!(garrison.contains(&"Units of your armies in it stay yours.".to_string()), !staying.is_empty(), "{garrison:?}");
    assert_eq!(lines[1..=garrison.len()], garrison[..], "the card lists the garrison");
    // The city is the one over the Crown's limit: the saving names the limit's part.
    let m = lines.iter().find(|l| l.starts_with("Your maintenance: ")).expect("a maintenance line");
    assert!(m.starts_with("Your maintenance: -") && m.ends_with(" of it the Crown's limit)."), "{m}");
    assert!(lines.contains(&"The Crown after the grant: 3 cities, within its limit of 3.".to_string()), "{lines:?}");
    assert!(lines.contains(&"Tribute from it at 10%.".to_string()), "{lines:?}");
    // Each subject's loyalty and state before, and what the preview says changes (the parts' amounts:
    // the loyalty as its parts make it; a contract's floor holds the total, the state after says it).
    let g = &ses.g;
    let subjects: Vec<(String, i32, Option<String>, i32)> = arr(cand, "effects")
        .iter()
        .map(|e| {
            let subject = s(e, "subject").to_string();
            let delta: i32 = strs(e, "lines").iter().map(|l| amount(l)).sum();
            let after = e.get("state_after").and_then(Json::as_str).map(str::to_string);
            let before = g.players[h].houses.iter().find(|&&x| g.house_label(x) == subject).map(|&x| g.house_loyalty_raw(x)).expect("a known subject");
            (subject, before, after, delta)
        })
        .collect();
    // The grantee: +1, and +3 more when its request is this city (met by the grant).
    let requested = g.houses[grantee].demand.is_some_and(|dm| dm.kind == ffh_core::persons::DemandKind::GrantCity(plot));
    let gain = 1 + if requested { ffh_core::persons::DEMAND_MET.0 } else { 0 };
    assert!(subjects.iter().any(|x| x.0 == label && x.3 == gain), "the grantee: +{gain}: {subjects:?}");
    assert!(subjects.iter().filter(|x| x.0 != label).all(|x| x.3 < 0), "every snubbed claimant loses: {subjects:?}");
    // Grant.
    let out = ses.execute(&Command::GrantCity { city: c, to: GrantTo::House(grantee) }).unwrap();
    let CmdOk::Applied { log, .. } = out else { panic!("{out:?}") };
    assert!(log.iter().any(|l| l.text == format!("{name} granted to {label}.")), "the toast's message: {:?}", log.iter().map(|l| &l.text).collect::<Vec<_>>());
    let g = &ses.g;
    let now = g.map.plots[plot].plot_city.unwrap();
    assert_eq!(g.cities[now].owner, g.houses[grantee].player, "the city is the House's player's");
    assert!(going.iter().all(|&u| !g.units.contains(u) || g.units[u].owner != h || g.units[u].death_delay), "the garrison went");
    assert!(staying.iter().all(|&u| g.units.contains(u) && g.units[u].owner == h), "the army's units stayed");
    assert_eq!(g.city_house(now), Some(grantee));
    for (subject, before, after, delta) in &subjects {
        let &x = g.players[h].houses.iter().find(|&&x| g.house_label(x) == *subject).unwrap();
        let (now, state) = (g.house_loyalty_raw(x), g.house_state(x));
        assert_eq!(now - before, *delta, "{subject}: the preview's change");
        if let Some(a) = after {
            assert_eq!(state.name(), a, "{subject}: the preview's state");
        }
    }
    // The Court: the city is no longer the Crown's, the Crown is within its limit; the grant list
    // of a city not ours: NotOwner; nor does the city screen open on it.
    let r = ses.query(&Query::Court).unwrap();
    assert!(!arr(&r, "crown").iter().any(|x| int(x, "city") == now.0 as i64));
    assert_eq!(s(r.get("crown_line").unwrap(), "text"), "The Crown holds 3 cities; its limit is 3.");
    assert!(!arr(&r, "attention").iter().any(|a| s(a, "kind") == "crown_over"));
    assert_eq!(ses.query(&Query::Grant { city: now }), Err(QueryErr::NotOwner));
    assert_eq!(ses.query(&Query::City { city: now }), Err(QueryErr::NotOwner));
    common::check::check(&ses.g);
}

/// A realm without a House (the human's four freed): the Court says its next grant founds one and
/// where the leader is drawn from; the grant list offers the founding alone, with the rules' line;
/// granting founds a House led by a drawn minor leader of the Bannor; then the founding is no longer
/// offered.
#[test]
fn the_first_grant_founds_a_house() {
    let (_dir, st, mut ses) = staged("persons-founding");
    let h = ses.human;
    for hs in st.houses.iter().filter(|x| x.role != "neighbour") {
        ses.execute(&Command::FreeHouse { house: hs.id }).unwrap();
    }
    assert!(ses.g.players[h].houses.is_empty());
    let r = ses.query(&Query::Court).unwrap();
    let founding = r.get("founding").unwrap();
    assert!(bool_of(founding, "ok"));
    assert_eq!(s(founding, "text"), "No House yet. A grant of a Crown city founds one. A new House: its leader is drawn from the Bannor's minor leaders.");
    assert!(arr(&r, "houses").is_empty() && arr(&r, "empty").is_empty(), "the Crown's card stays");
    let c = st.crown.iter().copied().find(|&c| ses.g.player_check_grant_city(h, c).is_ok()).unwrap();
    let r = ses.query(&Query::Grant { city: c }).unwrap();
    let cands = arr(&r, "candidates");
    assert_eq!(cands.len(), 1, "the founding alone");
    let card = &cands[0];
    assert_eq!(s(card, "kind"), "found");
    assert_eq!(card.get("to"), Some(&Json::parse(r#"{"found":true}"#).unwrap()));
    assert_eq!(s(card, "title"), "A new House");
    let house = s(card, "house").to_string();
    assert!(house.starts_with("House "), "{house}");
    let lines = strs(card, "lines");
    assert_eq!(lines[0], "A new House: its leader is drawn from the Bannor's minor leaders.");
    assert_eq!(lines[1], format!("Moves to {house}'s realm: its production and commerce are the House's."));
    assert!(lines.contains(&"Tribute from it at 20%.".to_string()), "{lines:?}");
    assert_eq!(s(card, "button"), format!("Found {house}"));
    let name = ffh_play::view::city_name(&ses.g, c);
    assert_eq!(s(card, "confirm"), format!("Grant {name} to found {house}? Its leader is drawn now. This cannot be undone."));
    assert!(card.get("refusal").unwrap().is_null());
    // The founder's own block keeps only the lines that move loyalty.
    let own = arr(card, "effects").iter().find(|e| bool_of(e, "self")).unwrap();
    assert!(strs(own, "lines").iter().all(|l| amount(l) != 0), "{own:?}");
    // Granted: a House led by a minor leader of the Bannor drawn now.
    let civ = ses.g.player_get_civilization_type(h).unwrap();
    let pool = ses.g.house_leader_pool(civ, h).leaders;
    let j = Json::parse(&format!(r#"{{"c":"grant_city","city":{},"found":true}}"#, c.0)).unwrap();
    let cmd = Command::from_json(ses.g.d, &j).unwrap();
    ses.execute(&cmd).unwrap();
    let g = &ses.g;
    assert_eq!(g.players[h].houses.len(), 1);
    let hh = g.players[h].houses[0];
    assert_eq!(g.house_label(hh), house);
    assert!(pool.contains(&g.house_leader(hh).unwrap()), "drawn from the pool");
    let r = ses.query(&Query::Court).unwrap();
    assert!(r.get("founding").unwrap().is_null());
    common::check::check(&ses.g);
}

/// The Armies screen's *Requisition from* list (the levy, rule 6): the Houses' cities after the
/// Crown's, each with its House and whether it takes levies, in the rules' words; and a Crown
/// city's Governance line (the city screen).
#[test]
fn the_levy_list_and_the_governance_line() {
    let (_dir, st, mut ses) = staged("persons-levy");
    let r = ses.query(&Query::Armies).unwrap();
    let army = arr(&r, "armies").iter().find(|x| int(x, "id") == army_id(&st, "main").0 as i64).unwrap().clone();
    let cands = arr(army.get("requisition").unwrap(), "candidates");
    let row = |c: CityId| cands.iter().find(|x| int(x, "city") == c.0 as i64).unwrap_or_else(|| panic!("city {} not listed", c.0)).clone();
    let a = st.houses.iter().find(|x| x.role == "friendly").unwrap();
    let x = row(a.cities[0]);
    let house = x.get("house").unwrap();
    assert_eq!(s(house, "label"), a.label);
    assert!(bool_of(house, "takes_levy"));
    assert_eq!(s(house, "text"), "Takes levies. Each levy: -1 (20 turns).");
    assert_eq!(s(house, "until"), levies_until(ses.g.house_loyalty(a.id)), "its state mark's tooltip");
    assert!(house.get("passed").unwrap().is_null());
    for role in ["annoyed", "furious"] {
        let v = st.houses.iter().find(|x| x.role == role).unwrap();
        let c = v.cities[0];
        let house = row(c).get("house").unwrap().clone();
        assert!(!bool_of(&house, "takes_levy"));
        assert_eq!(s(&house, "text"), "Takes no levy.");
        assert_eq!(s(&house, "state"), v.state.name());
        assert_eq!(s(&house, "reason"), format!("{} is {}. It takes no levy.", v.label, v.state.name()));
        assert_eq!(s(&house, "passed"), format!("{}: passed over ({} takes no levy).", ffh_play::view::city_name(&ses.g, c), v.label));
        assert!(house.get("until").unwrap().is_null());
    }
    assert!(row(st.capital).get("house").unwrap().is_null(), "a Crown city");
    // The Crown's cities come first.
    let first_house = cands.iter().position(|x| !x.get("house").unwrap().is_null()).unwrap();
    assert!(cands[first_house..].iter().all(|x| !x.get("house").unwrap().is_null()));

    // A House's city can be picked (the requisition passes it over while its House refuses).
    let loyal = army_id(&st, "main");
    let v = st.houses.iter().find(|x| x.role == "annoyed").unwrap();
    ses.execute(&Command::ArmyRequisition { army: loyal, city: v.cities[0], on: true }).expect("a House's city is picked");
    assert!(ses.g.armies[loyal].requisition.contains(&v.cities[0]));
    ses.execute(&Command::ArmyRequisition { army: loyal, city: v.cities[0], on: false }).expect("and released");
    assert!(matches!(ses.execute(&Command::ArmyRequisition { army: loyal, city: st.neighbour_near, on: true }), Err(CmdErr::NotOwner)), "a foreign city is not levied");

    // The Governance line: every city of ours is the Crown's.
    let over = ses.g.player_crown_over(ses.human)[0];
    for c in [st.capital, over] {
        let city = ses.query(&Query::City { city: c }).unwrap();
        let k = city.get("governance").unwrap();
        assert!(k.get("house").unwrap().is_null());
        let crown = k.get("crown").unwrap();
        if c == st.capital {
            assert!(!bool_of(crown, "grantable"));
            assert_eq!(s(crown, "refusal"), "The capital is the Crown's.");
            assert!(crown.get("over").unwrap().is_null());
        } else {
            assert!(bool_of(crown, "grantable"));
            assert_eq!(s(crown, "over"), "Over the Crown's limit of 3.");
        }
    }
}

/// Every command of the system refuses with the rules' own reason, and changes nothing.
#[test]
fn refusals_carry_the_rules_reasons() {
    let (_dir, st, mut ses) = staged("persons-refusals");
    let before = ses.g.save();
    let house_a = house_id(&st, "friendly");
    // Grants.
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.capital, to: GrantTo::House(house_a) })), "The capital is the Crown's.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::NewHouse })), "A realm with a House grants to its Houses.");
    assert_eq!(cannot(ses.execute(&Command::GrantCity { city: st.crown[1], to: GrantTo::House(house_id(&st, "neighbour")) })), "Not a House of yours.");
    let r = ses.query(&Query::Grant { city: st.capital }).unwrap();
    assert_eq!(s(&r, "refusal"), "The capital is the Crown's.");
    assert!(arr(&r, "candidates").is_empty());
    // Tribute and freeing.
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: house_a, pct: 15 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: house_a, pct: 60 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: house_id(&st, "neighbour"), pct: 10 })), "Not a House of yours.");
    assert_eq!(cannot(ses.execute(&Command::SetTributeDefault { pct: 60 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::SetTributeDefault { pct: -10 })), "Tribute: 0 to 50 percent, in steps of 10.");
    assert_eq!(cannot(ses.execute(&Command::FreeHouse { house: house_id(&st, "neighbour") })), "Not a House of yours.");
    // A contract's bound (the third build, rule 6): House `annoyed` takes no tribute above 10%.
    let v = house_id(&st, "annoyed");
    let until = ses.g.houses[v].contract.expect("a contract").until;
    assert_eq!(cannot(ses.execute(&Command::SetHouseTribute { house: v, pct: 30 })), format!("Your contract with {} forbids it until turn {until}.", ses.g.house_label(v)));
    assert!(before == ses.g.save(), "a refusal changed the game");
}

/// Free ends the vassalage at peace; the House is gone from the Court and a second Free is refused.
#[test]
fn freeing_a_house() {
    let (_dir, st, mut ses) = staged("persons-free");
    let h = ses.human;
    let a = house_id(&st, "friendly");
    let hp = ses.g.houses[a].player;
    ses.execute(&Command::FreeHouse { house: a }).unwrap();
    let g = &ses.g;
    assert!(!g.houses.contains(a));
    assert_eq!(g.players[hp].house, None);
    let (ht, pt) = (g.players[hp].team, g.players[h].team);
    assert!(!g.teams[ht].vassal[pt] && !g.at_war(ht, pt), "freed at peace");
    let r = ses.query(&Query::Court).unwrap();
    assert!(!arr(&r, "houses").iter().any(|x| int(x, "id") == a.0 as i64));
    assert_eq!(cannot(ses.execute(&Command::FreeHouse { house: a })), "Not a House of yours.");
    common::check::check(&ses.g);
}

/// A gift to the neighbour's House through the session: accepted (a House takes any gift) and
/// recorded as a gift on it (the fourth build, rule 3); anything asked back is refused.
#[test]
fn a_gift_to_a_house() {
    let (_dir, st, mut ses) = staged("persons-gift");
    let h = ses.human;
    let n = house_id(&st, "neighbour");
    let hp = ses.g.houses[n].player;
    let gold = |data: i32| TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false };
    ses.g.players[h].gold += 100;
    let before = ses.g.gifts_value(hp, h);
    let out = ses.execute(&Command::ProposeDeal { to: hp, give: vec![gold(100)], get: vec![] }).unwrap();
    assert!(matches!(out, CmdOk::Applied { accepted: Some(true), .. }), "a gift: {out:?}");
    assert!(ses.g.gifts_value(hp, h) > before, "recorded as a gift");
    common::check::check(&ses.g);
}

/// The civilization's channel through the session (`culture_channel {action?, target?,
/// religion?}`, the fourth build, rule 1): Court names a House by its player, a player who is no
/// House is refused; a refused order carries the rules' reason; no action stops it.
#[test]
fn the_culture_channel_command() {
    let (_dir, st, mut ses) = staged("persons-channel");
    let h = ses.human;
    let d = ses.g.d;
    let court = d.t::<CulturalActionType>("CULTURALACTION_COURT_HOUSE");
    let n = house_id(&st, "neighbour");
    let hp = ses.g.houses[n].player;
    let j = Json::parse(&format!(r#"{{"c":"culture_channel","action":"CULTURALACTION_COURT_HOUSE","target":{}}}"#, hp.0)).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::CultureChannel { action: Some(court), target: Some(hp), religion: None });
    let civ = Command::CultureChannel { action: Some(court), target: Some(st.neighbour), religion: None };
    assert!(matches!(ses.execute(&civ), Err(CmdErr::Invalid(_))), "Court takes a House");
    let order = ffh_core::culture::ChannelOrder { action: court, target: ffh_core::culture::ChannelTarget::House(n), religion: None };
    let cmd = Command::CultureChannel { action: Some(court), target: Some(hp), religion: None };
    match ses.g.culture_channel_check(h, &order) {
        Ok(()) => {
            ses.execute(&cmd).unwrap();
            assert_eq!(ses.g.players[h].culture_channel.as_ref().map(|ch| ch.order()), Some(order));
        }
        Err(why) => match ses.execute(&cmd) {
            Err(CmdErr::Cannot(m)) => assert!(m.contains(&why), "{m}"),
            other => panic!("refused with the rules' reason: {other:?}"),
        },
    }
    ses.execute(&Command::CultureChannel { action: None, target: None, religion: None }).unwrap();
    assert!(ses.g.players[h].culture_channel.is_none(), "stopped");
}

/// The commands are in the protocol, decode from their JSON and refuse an ambiguous grantee.
#[test]
fn the_command_set() {
    for n in ["grant_city", "house_tribute", "free_house", "tribute_default"] {
        assert!(COMMAND_NAMES.contains(&n), "{n}");
    }
    assert!(!COMMAND_NAMES.contains(&"set_commander"), "no commanders");
    let d = ffh_data::global();
    let j = Json::parse(r#"{"c":"grant_city","city":3,"found":true}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::GrantCity { city: CityId(3), to: GrantTo::NewHouse });
    let j = Json::parse(r#"{"c":"grant_city","city":3,"house":1}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::GrantCity { city: CityId(3), to: GrantTo::House(HouseId(1)) });
    let j = Json::parse(r#"{"c":"grant_city","city":3,"house":1,"found":true}"#).unwrap();
    assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))));
    let j = Json::parse(r#"{"c":"grant_city","city":3,"person":2}"#).unwrap();
    assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))), "no person grantee");
    let j = Json::parse(r#"{"c":"house_tribute","house":1,"pct":30}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::SetHouseTribute { house: HouseId(1), pct: 30 });
    let j = Json::parse(r#"{"c":"tribute_default","pct":40}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::SetTributeDefault { pct: 40 });
    let j = Json::parse(r#"{"c":"free_house","house":2}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::FreeHouse { house: HouseId(2) });
}

/// `court`, every city's `grant`, the Armies screen and the city screens leave the game as it was
/// (PLAY_DESIGN 2.5).
#[test]
fn the_queries_are_pure() {
    let (_dir, _st, ses) = staged("persons-pure");
    let before = ses.g.save();
    ses.query(&Query::Court).unwrap();
    for c in ses.g.players[ses.human].cities.clone() {
        ses.query(&Query::Grant { city: c }).unwrap();
        ses.query(&Query::City { city: c }).unwrap();
    }
    ses.query(&Query::Armies).unwrap();
    ses.query(&Query::View).unwrap();
    assert!(before == ses.g.save(), "a query changed the game");
}
