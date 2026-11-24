//! The production list's reasons (`buildable`'s `why`): every entry the city cannot start now
//! says why, in the words of the rules' refusal (ffh-play's `view::refusal`, from ffh-core's
//! `city::refusal`), never a placeholder; and a few refusals built by hand give their sentences.
//! Games that cannot end (`common::STEADY`); nothing here depends on how a seed's game goes.

mod common;

use std::sync::Arc;

use ffh_core::city::refusal::{ConstructRefusal, PlayerConstructRefusal};
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdOk, Command};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;
use ffh_play::{Json, Session, SessionOptions, Stop};

/// A steady duel as `civ` at the human's first control point, its capital founded.
fn duel_as(seed: u32, civ: &str) -> Session {
    let d = ffh_data::global();
    let mut ng = common::steady_duel(seed);
    ng.civ = Some(d.t::<CivilizationType>(civ));
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).unwrap();
    let r = s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false, confirm_end_siege: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    s
}

/// Every entry of every city of the human: a disabled one has a reason (a sentence, not a
/// placeholder), an enabled one none. Returns how many disabled entries it saw.
fn check_reasons(s: &Session) -> usize {
    let mut disabled = 0;
    for &c in &s.g.players[s.human].cities {
        let city = s.query(&Query::City { city: c }).unwrap();
        for b in city.get("buildable").and_then(Json::as_arr).unwrap_or(&[]) {
            let enabled = b.get("enabled").and_then(Json::as_bool).unwrap_or(false);
            let why = b.get("why");
            if enabled {
                assert!(why.is_none() || why == Some(&Json::Null), "an enabled entry has no reason: {b:?}");
                continue;
            }
            // Processes are always enabled; every other disabled entry says why.
            disabled += 1;
            let why = why.and_then(Json::as_str).unwrap_or_else(|| panic!("a disabled entry says why: {b:?}"));
            assert!(!why.trim().is_empty(), "{b:?}");
            assert!(why.ends_with('.'), "a sentence: {why:?}");
            let lower = why.to_lowercase();
            assert!(!lower.contains("possible now") && !lower.contains("available now"), "a placeholder: {why:?}");
        }
    }
    disabled
}

#[test]
fn every_disabled_entry_says_why() {
    let mut seen = 0;
    for (seed, civ) in [(3, "CIVILIZATION_BANNOR"), (11, "CIVILIZATION_ELOHIM"), (23, "CIVILIZATION_KURIOTATES")] {
        let mut s = duel_as(seed, civ);
        seen += check_reasons(&s);
        // Every tech: most of the list is visible, much of it refused (resources, wonders, levels).
        let (h, team) = (s.human, s.g.players[s.human].team);
        for t in s.g.d.techs.keys() {
            if !s.g.teams[team].has_tech[t] {
                s.g.team_set_has_tech(team, t, true, Some(h), false, false);
            }
        }
        seen += check_reasons(&s);
        for _ in 0..8 {
            let r = s.execute(&Command::EndTurn { force: true });
            assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
            assert_eq!(s.advance(&mut |_| {}), Stop::Human);
        }
        seen += check_reasons(&s);
    }
    assert!(seen > 0, "some entries are disabled in a new city");
}

/// A building's reason as the production list gives it, and its refusal.
fn construct(s: &Session, c: CityId, b: BuildingType) -> (Option<ConstructRefusal>, Option<String>) {
    let r = s.g.city_construct_refusal(c, b, false, false, false);
    (r, r.map(|r| ffh_play::view::refusal::construct_refusal_text(&s.g, c, b, r)))
}

#[test]
fn hand_built_refusals_give_their_sentences() {
    let d = ffh_data::global();
    let mut s = duel_as(5, "CIVILIZATION_BANNOR");
    let h = s.human;
    let capital = s.g.player_get_capital_city(h).unwrap();
    assert_eq!(s.g.players[h].num_cities(), 1);

    // Cities: the Forbidden Palace needs 8 (no tech).
    let palace = d.t::<BuildingType>("BUILDING_FORBIDDEN_PALACE");
    let need = d.buildings[palace].num_cities_prereq;
    let (r, text) = construct(&s, capital, palace);
    assert_eq!(r, Some(ConstructRefusal::Player(PlayerConstructRefusal::Cities { have: 1, need, max: -1 })));
    assert_eq!(text.as_deref(), Some(format!("Cities: 1 of {need}.").as_str()));

    // A tech: the Form of the Titan before Warfare.
    let titan = d.t::<BuildingType>("BUILDING_FORM_OF_THE_TITAN");
    let warfare = d.t::<TechType>("TECH_WARFARE");
    let team = s.g.players[h].team;
    assert!(!s.g.teams[team].has_tech[warfare]);
    let (_, text) = construct(&s, capital, titan);
    assert_eq!(text.as_deref(), Some(format!("Requires {}.", ffh_play::names::display(d, warfare)).as_str()));

    // With Warfare: a unit of level 6.
    s.g.teams[team].has_tech[warfare] = true;
    let level = s.g.players[h].highest_unit_level;
    let need = d.buildings[titan].unit_level_prereq;
    let (r, text) = construct(&s, capital, titan);
    assert_eq!(r, Some(ConstructRefusal::Player(PlayerConstructRefusal::UnitLevel { have: level, need })));
    assert_eq!(text.as_deref(), Some(format!("Requires a unit of level {need}: highest is {level}.").as_str()));

    // The production list carries the same sentence for the entry (listed: the level is not
    // part of the rules' visibility test).
    let city = s.query(&Query::City { city: capital }).unwrap();
    let entry = city
        .get("buildable")
        .and_then(Json::as_arr)
        .unwrap()
        .iter()
        .find(|b| b.get("order").and_then(Json::as_i64) == Some(OrderType::ORDER_CONSTRUCT.idx() as i64) && b.get("type").and_then(Json::as_i64) == Some(titan.idx() as i64))
        .cloned()
        .expect("the Form of the Titan is listed");
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(entry.get("why").and_then(Json::as_str), text.as_deref());
}
