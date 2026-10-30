//! The city screen's revolt predictions (`view::culture`, docs/CULTURE.md "Predictions") against
//! the rules' own turns: where a Foment Revolt channel brings a city ("settles"), including the
//! city's culture leaving its resistance once it is in revolt.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::view::culture::option_json;
use ffh_play::view::Viewer;

const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;

/// A duel map with cultural channels on: player 0's capital `a` and a town `t` of player 1 (not
/// its capital) within 5 plots of it (the distance effect in full), met and revealed, the town
/// without defenders and with a culture building, so that its culture resists until it is in
/// revolt.
fn game() -> (Game, CityId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_CULTURE_ACTIONS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let u = g.players[p0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let free = |g: &Game, px: i32, py: i32| g.map.plot_xy(px, py).filter(|&q| !g.plot_is_water(q) && !g.plot_is_city(q, false, None));
    let mut spots = Vec::new();
    for dy in -5..=5 {
        for dx in -5..=5 {
            let dist = g.plot_distance(x, y, x + dx, y + dy);
            if (3..=5).contains(&dist) && free(&g, x + dx, y + dy).is_some() {
                spots.push((x + dx, y + dy));
            }
        }
    }
    let capital = spots[0];
    let town = *spots.iter().find(|s| g.plot_distance(s.0, s.1, capital.0, capital.1) >= 3).expect("two spots");
    let a = g.player_init_city(p0, x, y, true, true);
    let f = g.player_init_city(p1, capital.0, capital.1, true, true);
    let t = g.player_init_city(p1, town.0, town.1, true, true);
    assert!(g.city_is_capital(f) && !g.city_is_capital(t));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    for c in [a, f, t] {
        g.cities[c].revealed[t0] = true;
        g.cities[c].revealed[t1] = true;
    }
    for u in g.map.plots[g.city_plot(t)].units.clone() {
        g.unit_kill(u, false, None);
    }
    let b = d.buildings.keys().find(|&b| d.buildings[b].commerce_change[CULTURE.idx()] >= 2 && d.buildings[b].prereq_religion.is_none() && !d.buildings[b].capital).expect("a culture building");
    g.city_set_num_real_building(t, b, 1);
    (g, a, t)
}

/// The prediction for a new channel of `rate / 2` culture from `a` at the town (hundredths), and
/// the town's pressure from player 0 after `turns` of the rules (`None`: it joined player 0).
fn predicted_and_played(rate: i32, turns: i32) -> (i64, Option<i32>, i32) {
    let (mut g, a, t) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let revolt = g.d.t::<CulturalActionType>("CULTURALACTION_FOMENT_REVOLT");
    g.cities[a].commerce_rate[CULTURE.idx()] = 100 * rate;
    let resist = g.city_revolt_resistance(t);
    assert!(resist > 0, "the town's culture resists");
    let option = option_json(&Viewer::new(&g, p0, false), a, revolt, p1, None);
    let reach = option.req_arr("reach").unwrap();
    let entry = reach.iter().find(|x| x.req::<i64>("city").unwrap() == t.0 as i64).expect("the town in reach");
    assert_eq!(entry.req::<i64>("effect").unwrap(), 100);
    let settles = entry.req::<i64>("settles").unwrap();
    g.city_set_culture_channel(a, Some((revolt, p1, None))).unwrap();
    let plot = g.city_plot(t);
    for _ in 0..turns {
        let next = g.game_turn() + 1;
        g.set_game_turn(next);
        g.cities[a].commerce_rate[CULTURE.idx()] = 100 * rate;
        g.city_do_culture_channel(a);
        g.city_do_turn(t);
        if !g.cities.contains(t) || g.cities[t].owner != p1 {
            let owner = g.map.plots[plot].plot_city.map(|c| g.cities[c].owner);
            assert_eq!(owner, Some(p0), "the town joined the caster");
            return (settles, None, resist);
        }
    }
    (settles, Some(g.cities[t].culture_pressure[p0]), resist)
}

/// With the town's culture resisting, the channel would stop short of the flip; in revolt it
/// makes no culture and the pressure runs on to the flip. The prediction says so, and the rules
/// take the town.
#[test]
fn a_channel_that_wins_the_town_once_its_culture_stops_resisting() {
    let (settles, end, resist) = predicted_and_played(32, 200);
    let flip = 100 * ffh_data::global().defines.culture_revolt_flip as i64;
    let inflow = 1600i64;
    // Where it would settle with the culture resisting all along (the rules' step, by hand).
    let mut p = 0i64;
    for _ in 0..2000 {
        let q = p + inflow;
        p = (q - resist as i64 - (q * 5 / 100).max(100)).max(0);
    }
    assert!(p < flip, "with its culture resisting it would stop at {p}, short of {flip}");
    assert!(settles >= flip, "the prediction: it joins us ({settles})");
    assert_eq!(end, None, "the rules: the town joined us");
}

/// A channel that holds the town in revolt without taking it: the prediction is where the rules
/// bring the pressure.
#[test]
fn a_channel_that_holds_the_town_in_revolt_settles_where_predicted() {
    let (settles, end, _) = predicted_and_played(24, 300);
    let d = ffh_data::global();
    let (disorder, flip) = (100 * d.defines.culture_revolt_disorder as i64, 100 * d.defines.culture_revolt_flip as i64);
    assert!(settles >= disorder && settles < flip, "in revolt, not taken: {settles}");
    let end = end.expect("not taken") as i64;
    assert!((end - settles).abs() <= settles / 50, "settled at {end}, predicted {settles}");
}
