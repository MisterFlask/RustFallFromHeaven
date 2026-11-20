//! Cultural channels (docs/CULTURE.md; docs/PERSONS.md, "The fourth build", rules 1 and 6): the
//! civilization's channel diverting half its cities' culture (nothing from a city in disorder),
//! the inflow, the reach from the nearest cities, Spread Religion at the civilization's level, the
//! checks, the messages, saves. Court, Guard and the split are `persons_courtship.rs`'s.

use ffh_core::culture::{ChannelOrder, ChannelTarget};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;
const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn action(g: &Game, name: &str) -> CulturalActionType {
    g.d.t::<CulturalActionType>(name)
}

/// A duel map after `game_init`: a city of player 0 (`a`, its capital), another of player 0 (`b`)
/// and a city of player 1 (`f`), all within 4 plots of `a`, the two teams met and each city
/// revealed to both.
fn game() -> (Game, CityId, CityId, CityId) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    let u = g.players[P0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    // Land plots 3 and 4 plots from the first city, at least 3 apart, with no city near.
    let land = |g: &Game, px: i32, py: i32| g.map.plot_xy(px, py).filter(|&q| !g.plot_is_water(q) && !g.plot_is_city(q, false, None));
    let mut spots = Vec::new();
    for dy in -4..=4 {
        for dx in -4..=4 {
            let dist = g.plot_distance(x, y, x + dx, y + dy);
            if dist >= 3 && land(&g, x + dx, y + dy).is_some() {
                spots.push((x + dx, y + dy));
            }
        }
    }
    let first = spots[0];
    let second = *spots.iter().find(|s| g.plot_distance(s.0, s.1, first.0, first.1) >= 3).expect("two spots");
    let a = g.player_init_city(P0, x, y, true, true);
    let b = g.player_init_city(P0, first.0, first.1, true, true);
    let f = g.player_init_city(P1, second.0, second.1, true, true);
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_meet(t0, t1, false);
    for c in [a, b, f] {
        g.cities[c].revealed[t0] = true;
        g.cities[c].revealed[t1] = true;
    }
    (g, a, b, f)
}

/// Sets city `c`'s culture rate (per turn) for the next reads.
fn set_rate(g: &mut Game, c: CityId, rate: i32) {
    g.cities[c].commerce_rate[CULTURE.idx()] = 100 * rate;
}

/// Player 0's Spread Religion of the first religion into player 1, that religion in city `a`.
fn spread_order(g: &mut Game, a: CityId) -> (ChannelOrder, ReligionType) {
    let r = g.d.religions.keys().next().unwrap();
    g.city_set_has_religion(a, r, true, false, false);
    (ChannelOrder { action: action(g, "CULTURALACTION_SPREAD_RELIGION"), target: ChannelTarget::Player(P1), religion: Some(r) }, r)
}

#[test]
fn the_effect_falls_with_the_square_of_the_distance() {
    let (g, ..) = game();
    let (full, range) = (g.d.defines.culture_channel_full_range, g.d.defines.culture_channel_range);
    assert_eq!(g.culture_distance_effect(0), 100);
    assert_eq!(g.culture_distance_effect(full), 100);
    assert_eq!(g.culture_distance_effect(2 * full), 25, "twice as far: a quarter");
    assert_eq!(g.culture_distance_effect(range + 1), 0, "beyond the range: nothing");
}

/// The reach runs from the caster's nearest city to the target's nearest city.
#[test]
fn reach_is_from_the_nearest_cities() {
    let (g, a, b, f) = game();
    let (da, db) = (g.plot_distance(g.cities[a].x, g.cities[a].y, g.cities[f].x, g.cities[f].y), g.plot_distance(g.cities[b].x, g.cities[b].y, g.cities[f].x, g.cities[f].y));
    assert_eq!(g.culture_city_distance(P0, f), Some(da.min(db)));
    let reach = g.culture_reach_to(P0, &[f]).expect("a reach");
    assert_eq!(reach.distance, da.min(db));
    assert_eq!(reach.effect, g.culture_distance_effect(da.min(db)));
    // From player 1's one city to the nearer of player 0's.
    assert_eq!(g.culture_reach_to(P1, &[a, b]).map(|r| r.distance), Some(da.min(db)));
}

/// The inflow is half of every city's culture, in hundredths (nothing lost to rounding), and a city
/// in disorder diverts nothing; the diverted half reaches neither the city's culture nor its plot.
#[test]
fn the_inflow_is_half_the_culture_and_nothing_from_a_city_in_disorder() {
    let (mut g, a, b, _) = game();
    let (o, _) = spread_order(&mut g, a);
    set_rate(&mut g, a, 7);
    set_rate(&mut g, b, 5);
    assert_eq!(g.city_culture_diverted_times100(a), 0, "no channel: nothing diverted");
    assert_eq!(g.player_culture_inflow_now(P0), 600, "what a channel would divert now");
    g.player_set_culture_channel(P0, Some(o)).expect("set");
    assert_eq!((g.city_culture_diverted_times100(a), g.city_culture_diverted_times100(b)), (350, 250), "half of 7 and of 5, in hundredths");
    // A city's turn: the half goes to the inflow, the rest to the city and its plot.
    g.cities[a].culture[P0] = 4000;
    let mut without = Game::load(g.d, &g.save()).unwrap();
    without.player_set_culture_channel(P0, None).unwrap();
    let plot = g.city_plot(a);
    let (city0, plot0) = (g.cities[a].culture[P0], g.map.plots[plot].culture[P0]);
    g.player_culture_channel_turn_start(P0);
    assert_eq!(g.players[P0].culture_inflow, 0, "counted from 0 at the turn's start");
    g.city_do_turn(a);
    set_rate(&mut without, a, 7);
    without.city_do_turn(a);
    assert_eq!(g.players[P0].culture_inflow, 350);
    assert_eq!(without.cities[a].culture[P0] - city0, (g.cities[a].culture[P0] - city0) + 350, "the city's culture misses the half");
    assert_eq!(without.map.plots[plot].culture[P0] - plot0, (g.map.plots[plot].culture[P0] - plot0) + 7 - (700 - 350) / 100, "its plot's culture too (whole culture)");
    // A city in disorder diverts nothing.
    g.city_set_occupation_timer(b, 2);
    assert!(g.city_is_disorder(b));
    assert_eq!(g.city_culture_diverted_times100(b), 0);
    set_rate(&mut g, b, 5);
    g.city_do_turn(b);
    assert_eq!(g.players[P0].culture_inflow, 350, "b in disorder added nothing");
}

/// Spread Religion is the civilization's: the chance sums over the target's cities in reach, each at
/// its distance from the caster's nearest city, from the turn's inflow; it rolls after the cities'
/// turns, and the channel ends with nothing left to do.
#[test]
fn spread_religion_at_the_civilization_level() {
    let (mut g, a, _, f) = game();
    let (o, r) = spread_order(&mut g, a);
    g.player_set_culture_channel(P0, Some(o)).expect("set");
    let inflow = 2000;
    let reach = g.culture_spread_reach(P0, P1, r, inflow);
    let x = reach.iter().find(|x| x.city == f).expect("f in reach");
    assert_eq!(x.distance, g.culture_city_distance(P0, f).unwrap());
    let present = g.d.religions.keys().filter(|&o| g.cities[f].has_religion[o]).count() as i32;
    assert_eq!(x.amount, inflow * g.d.defines.culture_spread_per_culture * x.effect / 10000 / (1 + present));
    assert!(g.culture_spread_chance(&reach) > 0);
    let mut turns = 0;
    while !g.cities[f].has_religion[r] {
        turns += 1;
        assert!(turns < 400, "it never spread");
        g.players[P0].culture_inflow = inflow;
        g.player_do_culture_channel(P0);
    }
    assert!(g.log.entries.iter().any(|m| m.text.contains("spread to")), "told");
    // Nothing left to spread it to: the channel ends at the next turn's start, its caster told why.
    g.player_culture_channel_turn_start(P0);
    assert!(g.players[P0].culture_channel.is_none());
    assert!(g.log.entries.iter().any(|m| m.text.starts_with("Channel ended: Spread Religion")), "the caster is told");
}

#[test]
fn channels_are_checked() {
    let (mut g, a, _, _) = game();
    let court = action(&g, "CULTURALACTION_COURT_HOUSE");
    let spread = action(&g, "CULTURALACTION_SPREAD_RELIGION");
    let r = g.d.religions.keys().next().unwrap();
    let order = |action, target, religion| ChannelOrder { action, target, religion };
    assert_eq!(g.culture_channel_check(P0, &order(court, ChannelTarget::Player(P1), None)), Err("Court and Guard take a House.".to_string()));
    assert_eq!(g.culture_channel_check(P0, &order(spread, ChannelTarget::Player(P1), None)), Err("Choose a religion.".to_string()));
    assert_eq!(g.culture_channel_check(P0, &order(spread, ChannelTarget::Player(P1), Some(r))), Err("None of your cities has that religion.".to_string()));
    g.city_set_has_religion(a, r, true, false, false);
    // Not for a turn without culture: the channel may stand.
    set_rate(&mut g, a, 0);
    assert!(g.culture_channel_check(P0, &order(spread, ChannelTarget::Player(P1), Some(r))).is_ok());
    // Refused when every city in reach has it.
    for c in g.players[P1].cities.clone() {
        g.city_set_has_religion(c, r, true, false, false);
    }
    assert!(g.culture_channel_check(P0, &order(spread, ChannelTarget::Player(P1), Some(r))).is_err_and(|e| e.starts_with("None of their cities in reach can take it")));
    // The barbarians channel nothing.
    let barbarian = g.barbarian_player();
    assert!(g.culture_channel_check(barbarian, &order(spread, ChannelTarget::Player(P1), Some(r))).is_err());
}

/// The target is told when a channel aimed at it starts and when it stops; a channel into the
/// caster's own cities tells no one.
#[test]
fn the_target_is_told_of_a_start_and_a_stop() {
    let (mut g, a, _, _) = game();
    let (o, _) = spread_order(&mut g, a);
    let to_p1 = |g: &Game| g.log.entries.iter().filter(|m| m.player == Some(P1.0)).count();
    let before = to_p1(&g);
    g.player_set_culture_channel(P0, Some(o)).expect("set");
    assert_eq!(to_p1(&g), before + 1, "told of the start");
    assert!(g.players[P0].culture_channel.as_ref().is_some_and(|ch| ch.since_turn == g.game_turn()));
    g.player_set_culture_channel(P0, Some(o)).expect("the same order");
    assert_eq!(to_p1(&g), before + 1, "the same channel set again changes nothing");
    g.player_set_culture_channel(P0, None).expect("stopped");
    assert_eq!(to_p1(&g), before + 2, "told of the stop");
    assert!(g.players[P0].culture_channel.is_none());
    // Its own cities: no one is told.
    let home = ChannelOrder { target: ChannelTarget::Player(P0), ..o };
    if g.culture_channel_check(P0, &home).is_ok() {
        g.player_set_culture_channel(P0, Some(home)).unwrap();
        assert_eq!(to_p1(&g), before + 2);
    }
    // Public to whoever met the caster.
    let t1 = g.players[P1].team;
    assert!(g.culture_channel_visible(t1, P0));
}

#[test]
fn the_channel_survives_a_save() {
    let (mut g, a, _, _) = game();
    let (o, _) = spread_order(&mut g, a);
    g.player_set_culture_channel(P0, Some(o)).expect("set");
    g.players[P0].culture_inflow = 1234;
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.players[P0].culture_channel, g.players[P0].culture_channel);
    assert_eq!(h.players[P0].culture_inflow, 1234);
    assert_eq!(h.save(), bytes);
}
