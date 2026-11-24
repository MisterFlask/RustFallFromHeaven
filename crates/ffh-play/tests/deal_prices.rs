//! The AI's prices for what it gives (ffh-ai `prices`), through the session: the trade query's
//! items for its cities carry a value or a refusal, a priced one its make-up; a city bought with
//! gold through `propose_deal` changes hands and the game plays on.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;
use ffh_play::command::{CmdOk, Command};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;
use ffh_play::{Json, Session, SessionOptions, Stop};

fn applied(s: &mut Session, c: Command) -> Option<bool> {
    match s.execute(&c) {
        Ok(CmdOk::Applied { accepted, .. }) => accepted,
        other => panic!("{c:?} -> {other:?}"),
    }
}

fn next_turn(s: &mut Session) {
    while let Some(city) = s.pending().specialties.first().map(|p| p.city) {
        applied(s, Command::ChooseSpecialty { city, option: 0 });
    }
    applied(s, Command::EndTurn { force: true });
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
}

fn item(item_type: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type, data, offering: false, hidden: false }
}

#[test]
fn a_city_bought_from_the_ai_changes_hands() {
    let ng = common::steady_duel(17);
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).unwrap();
    applied(&mut s, Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false, confirm_end_siege: false });
    let ai = s.g.civ_player_ids().find(|&p| p != h).unwrap();
    let (my, their) = (s.g.players[h].team, s.g.players[ai].team);
    s.g.team_meet(my, their, false);
    s.g.teams[my].gold_trading_count += 1;
    s.g.players[h].gold = 100_000;
    s.g.players[h].feat_accomplished[FeatType::FEAT_TRUST] = true;
    s.g.players[h].alignment = s.g.players[ai].alignment;
    // The AI's four cities, built by hand on free land 3 plots or more from any city.
    let plots: Vec<PlotIdx> = s.g.map.plot_ids().collect();
    for q in plots {
        if s.g.players[ai].cities.len() >= 4 {
            break;
        }
        let (x, y) = s.g.map.xy(q);
        let clear = s.g.cities.iter().all(|(_, c)| s.g.plot_distance(c.x, c.y, x, y) >= 3);
        if clear && s.g.player_can_found(ai, x, y, false) {
            s.g.player_init_city(ai, x, y, true, true);
        }
    }
    assert_eq!(s.g.players[ai].cities.len(), 4, "four cities");
    // The trade query: each city of theirs is priced or refused; a priced one says how.
    let t = s.query(&Query::Trade { with: ai }).unwrap();
    let cities: Vec<Json> = t.get("they_give").unwrap().as_arr().unwrap().iter().filter(|i| i.get("item") == Some(&Json::idx(TradeableItem::TRADE_CITIES))).cloned().collect();
    assert!(!cities.is_empty());
    let priced = |i: &Json| i.get("value").is_some_and(|v| !v.is_null());
    for i in &cities {
        assert!(priced(i) || i.get("refusal").is_some_and(|r| r.as_str().is_some()), "{i}");
        if priced(i) {
            assert!(i.get("value_note").and_then(|n| n.as_str()).is_some_and(|n| n.starts_with("Settler ")), "{i}");
        }
    }
    let city = cities.iter().find(|i| priced(i)).expect("a city for sale");
    let c = CityId(city.get("data").unwrap().as_i64().unwrap() as u32);
    let price = -city.get("value").unwrap().as_i64().unwrap() as i32;
    assert!(price > 0);
    let at = s.g.city_plot(c);
    // Gold is worth at least its face value to the AI: the price in gold is enough.
    let deal = Command::ProposeDeal { to: ai, give: vec![item(TradeableItem::TRADE_GOLD, price)], get: vec![item(TradeableItem::TRADE_CITIES, c.0 as i32)] };
    assert_eq!(applied(&mut s, deal), Some(true));
    let ours = |s: &Session| s.g.players[h].cities.iter().any(|&k| s.g.city_plot(k) == at);
    assert!(ours(&s), "the city is the human's");
    for _ in 0..15 {
        next_turn(&mut s);
    }
    assert!(ours(&s), "and stays so");
}
