//! Houses on the diplomacy screen (docs/PERSONS.md, "The second build", rules 7 and 13; "UI"; "The
//! fourth build"): the war a declaration brings in for certain by the vassal rules (the war
//! dialog's `war_against` and `war_join`, checked against the war the rules then make), what a
//! declaration (`war_memory`) costs with them, checked against their attitude parts after it (the
//! memory of wrongs, the owner's decision of 2026-11-18), and a House's state and the human's
//! courtship of it on its line in the list. On the staged scene (`ffh-play stage persons`), with the neighbour
//! given a second House by the rules' own calls (one of the human's freed, then the neighbour's
//! vassal by a deal).

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use ffh_core::ids::*;
use ffh_play::json::Json;
use ffh_play::stage::StagedPersons;

fn load(name: &str) -> (common::TempDir, StagedPersons, ffh_play::Session) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::persons(&dir.0, false).expect("staged");
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

fn strs(j: &Json, key: &str) -> BTreeSet<String> {
    arr(j, key).iter().map(|x| x.as_str().unwrap_or_else(|| panic!("{key}: {x}")).to_string()).collect()
}

/// The labels of the players of `teams` (a House by its label), as the screens name them.
fn labels(g: &ffh_core::game::Game, teams: &BTreeSet<TeamId>) -> BTreeSet<String> {
    g.player_ids().filter(|&p| g.players[p].alive && teams.contains(&g.players[p].team)).map(|p| g.player_house(p).map_or_else(|| g.player_house_name(p), |h| g.house_label(h))).collect()
}

/// The teams at war with team `t`.
fn at_war(g: &ffh_core::game::Game, t: TeamId) -> BTreeSet<TeamId> {
    g.team_ids().filter(|&o| g.teams[t].at_war[o]).collect()
}

/// The human's House `role` is freed (rule 8) and becomes the neighbour's vassal by a deal: the
/// neighbour then has two Houses. Returns the new House's team.
fn second_house(s: &mut ffh_play::Session, st: &StagedPersons, role: &str) -> TeamId {
    let x = st.houses.iter().find(|x| x.role == role).unwrap();
    s.g.player_free_house(st.human, x.id).expect("freed");
    let (nt, xt) = (s.g.players[st.neighbour].team, s.g.players[x.player].team);
    assert!(!s.g.teams[xt].at_war[nt]);
    s.g.team_assign_vassal(nt, xt, false);
    assert!(s.g.teams[xt].vassal[nt] && s.g.player_house(x.player).is_some_and(|h| s.g.houses[h].owner == st.neighbour), "the neighbour's House");
    xt
}

/// The war dialog names, for certain, the target's Houses at war with the human and the human's
/// Houses that join, as the rules then make the war.
#[test]
fn the_war_dialog_names_the_houses_that_join() {
    let (_dir, st, mut s) = load("persons-diplo-war");
    let xt = second_house(&mut s, &st, "friendly");
    let g = &s.g;
    let (ht, nt) = (g.players[st.human].team, g.players[st.neighbour].team);
    let nh = st.houses.iter().find(|x| x.role == "neighbour").unwrap();
    let theirs: BTreeSet<TeamId> = [g.players[nh.player].team, xt].into();
    let ours: BTreeSet<TeamId> = g.team_ids().filter(|&o| g.teams[o].vassal[ht]).collect();
    assert!(ours.len() >= 2, "the human keeps Houses: {ours:?}");
    let pl = q(&s, r#"{"q":"players"}"#);
    let row = find(&arr(&pl, "players"), "id", st.neighbour.0 as i64);
    assert_eq!(row.get("can_declare_war").and_then(Json::as_bool), Some(true));
    assert_eq!(strs(&row, "war_against"), labels(g, &theirs), "{row}");
    assert_eq!(strs(&row, "war_join"), labels(g, &ours), "{row}");
    // A war on a House is a war on its liege and the liege's other Houses (the vassal rules).
    let hrow = find(&arr(&pl, "players"), "id", nh.player.0 as i64);
    assert_eq!(hrow.get("can_declare_war").and_then(Json::as_bool), Some(true));
    let mut want = labels(g, &[xt].into());
    want.insert(format!("the {}", g.player_get_civilization_short_description(st.neighbour)));
    assert_eq!(strs(&hrow, "war_against"), want, "{hrow}");
    assert_eq!(strs(&hrow, "war_join"), labels(g, &ours), "{hrow}");
    // What it costs: the neighbour remembers it, and so does each House of its at war with the
    // human (the war on a House names its liege).
    let line = row.get("war_memory").and_then(Json::as_str).expect("an AI neighbour remembers it").to_string();
    let n: i32 = line.strip_prefix("They remember it: attitude toward you ").and_then(|r| r.split_once(" (fades). So do ")).unwrap_or_else(|| panic!("{line}")).0.parse().unwrap();
    assert!(n < 0, "{line}");
    let war_term = |g: &ffh_core::game::Game| ffh_ai::diplomacy::attitude_parts(g, st.neighbour, st.human).iter().filter(|x| x.0 == "declared war on us").map(|x| x.1).sum::<i32>();
    let term_before = war_term(g);
    for name in labels(g, &theirs) {
        assert!(line.contains(&name), "{line} names {name}");
    }
    let hline = hrow.get("war_memory").and_then(Json::as_str).unwrap().to_string();
    assert!(hline.contains(&format!("the {}", g.player_get_civilization_short_description(st.neighbour))), "{hline}");
    // The human's own Houses: no war to declare, nobody listed.
    let own = st.houses.iter().find(|x| x.role == "pleased").unwrap();
    let orow = find(&arr(&pl, "players"), "id", own.player.0 as i64);
    assert_eq!(orow.get("can_declare_war").and_then(Json::as_bool), Some(false));
    assert!(arr(&orow, "war_against").is_empty() && arr(&orow, "war_join").is_empty());

    // The rules' war: the same parties (defensive pacts aside, which the dialog says may join).
    let pacts: BTreeSet<TeamId> = g.team_ids().filter(|&o| g.teams[o].defensive_pact[nt]).collect();
    let (before_h, before_n) = (at_war(g, ht), at_war(g, nt));
    s.g.team_declare_war(ht, nt, false, None);
    let g = &s.g;
    let against: BTreeSet<TeamId> = at_war(g, ht).difference(&before_h).copied().filter(|&o| o != nt && !pacts.contains(&o)).collect();
    let joined: BTreeSet<TeamId> = at_war(g, nt).difference(&before_n).copied().filter(|&o| o != ht).collect();
    assert_eq!(against, theirs);
    assert_eq!(joined, ours);
    // The line's number is what the neighbour's attitude part moved.
    assert_eq!(war_term(g) - term_before, n, "{line}");
}

/// The diplomacy list's line of a House not the human's: its state toward its liege and the human's
/// courtship of it (`courting`: the rules' `house_courting_line`; the fourth build), and every other
/// courtship of it, in the open (`courted`: the human's own is its `courting` line).
#[test]
fn a_rival_houses_line_states_its_state_and_courtship() {
    let (_dir, st, s) = load("persons-diplo-line");
    let nh = st.houses.iter().find(|x| x.role == "neighbour").unwrap();
    let r = find(&arr(&q(&s, r#"{"q":"players"}"#), "players"), "id", nh.player.0 as i64);
    let o = r.get("house_offer").expect("a rival's House states its line");
    assert_eq!(o.get("state").and_then(Json::as_str), Some(s.g.house_state(nh.id).name()));
    assert_eq!(o.get("courting").and_then(Json::as_str).map(str::to_string), s.g.house_courting_line(nh.id, st.human));
    let courted: Vec<&Json> = r.get("house").unwrap().get("courted").and_then(Json::as_arr).expect("courted").iter().collect();
    assert_eq!(courted.len(), s.g.house_courtships(nh.id).iter().filter(|c| c.courter != st.human).count(), "{courted:?}");
    for c in &courted {
        assert!(c.get("line").and_then(Json::as_str).is_some_and(|l| l.starts_with("Courted by the ")), "{c:?}");
        let turn = c.get("calc").and_then(|k| k.get("turn")).and_then(Json::as_arr).expect("its calculation");
        assert!(turn.first().and_then(Json::as_str).is_some_and(|l| l.starts_with("Next liege turn: ")), "{c:?}");
    }
    assert!(o.get("courting").and_then(Json::as_str).is_some(), "the human courts it (the stage)");
    // The human's own Houses carry no line (their state is the record's, for the liege).
    let own = st.houses.iter().find(|x| x.role == "furious").unwrap();
    let r = find(&arr(&q(&s, r#"{"q":"players"}"#), "players"), "id", own.player.0 as i64);
    assert!(r.get("house_offer").is_none(), "{r}");
    assert_eq!(r.get("house").unwrap().get("state").and_then(Json::as_str), Some("Furious"));
}
