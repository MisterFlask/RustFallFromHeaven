//! Houses (docs/HOUSES.md, rule 15): a human's pick is a blocking decision with one option per
//! candidate, and the rule-11 messages reach the client marked as House alerts.

use std::sync::{Arc, Mutex};

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_play::decision::{Decision, DecisionKind};
use ffh_play::human::{Answer, FnAnswerer};
use ffh_play::{Session, SessionOptions, Stop};

mod common;

/// Three cities of player `h` near each other (its capital the first), on free land.
fn three_cities(g: &mut Game, h: PlayerId) -> Vec<CityId> {
    let mut cities: Vec<CityId> = g.players[h].cities.clone();
    let anchor = cities.first().map(|&c| (g.cities[c].x, g.cities[c].y)).unwrap_or_else(|| {
        let u = g.players[h].units[0];
        (g.units[u].x, g.units[u].y)
    });
    while cities.len() < 3 {
        let (x, y) = g
            .map
            .plot_ids()
            .filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none())
            .map(|q| g.map.xy(q))
            .filter(|&(x, y)| g.plot_distance(x, y, anchor.0, anchor.1) <= 4)
            .find(|&(x, y)| g.cities.iter().all(|(_, c)| g.plot_distance(x, y, c.x, c.y) >= 3))
            .expect("a free plot near the first city");
        cities.push(g.player_init_city(h, x, y, true, true));
    }
    cities
}

#[test]
fn a_humans_pick_blocks_with_one_option_per_candidate() {
    let asked: Arc<Mutex<Vec<Decision>>> = Arc::new(Mutex::new(Vec::new()));
    let seen = asked.clone();
    let answerer = Arc::new(FnAnswerer(move |_g: &Game, d: &Decision| {
        if let DecisionKind::HouseLeader { .. } = d.kind {
            seen.lock().unwrap().push(d.clone());
            Answer::Option(1)
        } else {
            Answer::Advisor
        }
    }));
    let mut s = Session::new_game(&common::steady_duel(5), answerer, SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    s.g.log.enabled = true;
    three_cities(&mut s.g, h);
    s.g.player_houses_turn(h);
    let asked = asked.lock().unwrap();
    assert_eq!(asked.len(), 1, "one question for the first House");
    let d = &asked[0];
    assert_eq!(d.kind.name(), "house_leader");
    assert!(d.title.starts_with("House ") && d.title.ends_with(" forms"), "{}", d.title);
    assert_eq!(d.options.len(), 3);
    assert!(d.advisor.is_some());
    for o in &d.options {
        assert!(o.help.contains("Alignment: ") && o.help.contains("Opinion now: ") && o.help.contains("Happiness: "), "{}", o.help);
    }
    let h0 = s.g.players[h].houses[0];
    assert_eq!(s.g.d.type_name(s.g.houses[h0].leader), d.options[1].key, "the option answered leads");
    // The forming message is a House alert for the client.
    let entry = s.g.log.entries.iter().find(|e| e.text.contains(" forms: ")).expect("the message");
    let j = ffh_play::report::entry_fields(ffh_play::Json::obj(), entry);
    assert_eq!(j.get("mark").and_then(ffh_play::Json::as_str), Some("house"));
    assert_eq!(j.get("kind").and_then(ffh_play::Json::as_str), Some("house"));
}
