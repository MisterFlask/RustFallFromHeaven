//! Cultural channels (docs/CULTURE.md): channelling half the culture, Foment Revolt's pressure
//! (unhappiness, disorder, a flip), Spread Religion's chance, the checks, memories, saves.

use ffh_core::culture::{CultureRefusal, REVOLT_REVOLT, REVOLT_UNREST};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;

fn action(g: &Game, name: &str) -> CulturalActionType {
    g.d.t::<CulturalActionType>(name)
}

/// A duel map with cultural channels on, after `game_init`: a city of player 0 (`a`), another of
/// player 0 (`b`) and a city of player 1 (`f`), all within 4 plots of `a`, the two teams met and
/// each city revealed to both.
fn game() -> (Game, CityId, CityId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_CULTURE_ACTIONS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let u = g.players[p0].units[0];
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
    let a = g.player_init_city(p0, x, y, true, true);
    let b = g.player_init_city(p0, first.0, first.1, true, true);
    let f = g.player_init_city(p1, second.0, second.1, true, true);
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
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

/// A second city of player 1 (not its capital), on land 3 or more plots from every city and
/// within 5 plots of city `near`.
fn second_city(g: &mut Game, near: CityId) -> CityId {
    let (x, y) = (g.cities[near].x, g.cities[near].y);
    let mut spot = None;
    'find: for dy in -5..=5 {
        for dx in -5..=5 {
            let Some(q) = g.map.plot_xy(x + dx, y + dy) else { continue };
            let (px, py) = g.map.xy(q);
            if g.plot_is_water(q) || g.plot_is_city(q, false, None) || g.plot_distance(x, y, px, py) > 5 {
                continue;
            }
            if g.cities.iter().all(|(_, c)| g.plot_distance(px, py, c.x, c.y) >= 3) {
                spot = Some((px, py));
                break 'find;
            }
        }
    }
    let (px, py) = spot.expect("room for a second city");
    let c = g.player_init_city(PlayerId(1), px, py, true, true);
    let (t0, t1) = (g.players[PlayerId(0)].team, g.players[PlayerId(1)].team);
    g.cities[c].revealed[t0] = true;
    g.cities[c].revealed[t1] = true;
    c
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

#[test]
fn a_channel_diverts_half_the_culture_from_the_city_and_its_plots_until_changed() {
    let (mut g, a, _, _) = game();
    let revolt = action(&g, "CULTURALACTION_FOMENT_REVOLT");
    set_rate(&mut g, a, 10);
    // Mid-level, so that neither game's city reaches another culture level (whose plot spread
    // differs) this turn.
    g.cities[a].culture[PlayerId(0)] = 4000;
    let mut with = Game::load(g.d, &g.save()).unwrap();
    with.city_set_culture_channel(a, Some((revolt, PlayerId(1), None))).unwrap();
    let (p0, plot) = (PlayerId(0), g.city_plot(a));
    let (city0, plot0) = (g.cities[a].culture[p0], g.map.plots[plot].culture[p0]);
    g.city_do_turn(a);
    set_rate(&mut with, a, 10);
    with.city_do_turn(a);
    let half = 10 * g.d.defines.culture_channel_percent / 100;
    assert_eq!(with.cities[a].culture_diverted, half);
    assert_eq!(g.cities[a].culture[p0] - city0, (with.cities[a].culture[p0] - city0) + 100 * half, "the city's culture misses the half");
    assert_eq!(g.map.plots[plot].culture[p0] - plot0, (with.map.plots[plot].culture[p0] - plot0) + half, "its plot's culture too");
    // It stands until changed.
    assert!(with.cities[a].culture_channel.is_some());
    with.city_set_culture_channel(a, None).unwrap();
    assert!(with.cities[a].culture_channel.is_none());
}

#[test]
fn revolt_pressure_brings_unrest_then_disorder_then_the_city() {
    let (mut g, a, _, f) = game();
    let revolt = action(&g, "CULTURALACTION_FOMENT_REVOLT");
    let town = second_city(&mut g, f);
    for c in [f, town] {
        set_rate(&mut g, c, 0);
        for u in g.map.plots[g.city_plot(c)].units.clone() {
            g.unit_kill(u, false, None);
        }
    }
    g.city_set_culture_channel(a, Some((revolt, PlayerId(1), None))).unwrap();
    let town_plot = g.city_plot(town);
    let mut saw = (false, false);
    for turn in 0..60 {
        let next = g.game_turn() + 1;
        g.set_game_turn(next);
        // A great cultural city: enough to win the town, a few plots off.
        set_rate(&mut g, a, 200);
        g.city_do_culture_channel(a);
        if !g.cities.contains(town) {
            break;
        }
        g.city_do_revolt_pressure(town);
        if !g.cities.contains(town) {
            break;
        }
        let level = g.cities[town].culture_revolt_level;
        if level >= REVOLT_UNREST {
            assert!(g.city_culture_unhappiness(town) > 0, "turn {turn}: unrest makes unhappiness");
            saw.0 = true;
        }
        if level >= REVOLT_REVOLT {
            assert!(g.city_is_occupation(town), "turn {turn}: a revolt is disorder");
            saw.1 = true;
        }
    }
    assert!(saw.0 && saw.1, "unrest and revolt on the way: {saw:?}");
    let joined = g.map.plots[town_plot].plot_city.is_some_and(|c| g.cities[c].owner == PlayerId(0));
    assert!(joined, "the town joined us");
    assert!(g.cities.contains(f) && g.cities[f].owner == PlayerId(1), "a capital never flips");
    assert!(g.players[PlayerId(1)].culture_memory[PlayerId(0)][0] > 0, "their leader remembers it");
}

#[test]
fn resistance_and_fading_hold_pressure_back() {
    let (mut g, a, _, f) = game();
    let revolt = action(&g, "CULTURALACTION_FOMENT_REVOLT");
    g.city_set_culture_channel(a, Some((revolt, PlayerId(1), None))).unwrap();
    // Their own culture outweighs ours: no pressure stays.
    set_rate(&mut g, a, 10);
    set_rate(&mut g, f, 40);
    for _ in 0..10 {
        g.city_do_culture_channel(a);
        g.city_do_revolt_pressure(f);
    }
    assert_eq!(g.city_revolt_pressure(f), 0);
    // Ours outweighs theirs; then the channel stops and the pressure fades.
    set_rate(&mut g, f, 0);
    for _ in 0..10 {
        set_rate(&mut g, a, 10);
        g.city_do_culture_channel(a);
        g.city_do_revolt_pressure(f);
    }
    let high = g.city_revolt_pressure(f);
    assert!(high > 0);
    g.city_set_culture_channel(a, None).unwrap();
    for _ in 0..200 {
        g.city_do_revolt_pressure(f);
    }
    assert_eq!(g.city_revolt_pressure(f), 0, "faded from {high}");
}

#[test]
fn spread_religion_has_a_chance_a_turn_and_ends_with_nothing_left() {
    let (mut g, a, _, f) = game();
    let spread = action(&g, "CULTURALACTION_SPREAD_RELIGION");
    let r = g.d.religions.keys().next().unwrap();
    g.city_set_has_religion(a, r, true, false, false);
    assert!(matches!(g.culture_channel_check(a, spread, PlayerId(1), None), Err(CultureRefusal::Religion(_))));
    g.city_set_culture_channel(a, Some((spread, PlayerId(1), Some(r)))).unwrap();
    set_rate(&mut g, a, 20);
    let reach = g.culture_channel_reach(a, spread, PlayerId(1), Some(r));
    let x = reach.iter().find(|x| x.city == f).expect("f in reach");
    let present = g.d.religions.keys().filter(|&o| g.cities[f].has_religion[o]).count() as i32;
    let amount = g.city_channel_amount(a);
    assert_eq!(x.amount, amount * g.d.defines.culture_spread_per_culture * x.effect / 100 / (1 + present));
    assert!(g.culture_spread_chance(&reach) > 0);
    let mut turns = 0;
    while !g.cities[f].has_religion[r] {
        turns += 1;
        assert!(turns < 400, "it never spread");
        set_rate(&mut g, a, 20);
        g.city_do_culture_channel(a);
    }
    assert!(g.log.entries.iter().any(|m| m.text.contains("spread to")), "told");
    // Nothing left to spread it to: the channel ends.
    set_rate(&mut g, a, 20);
    g.city_do_culture_channel(a);
    assert!(g.cities[a].culture_channel.is_none());
}

#[test]
fn channels_are_checked() {
    let (mut g, a, _, _) = game();
    let revolt = action(&g, "CULTURALACTION_FOMENT_REVOLT");
    assert_eq!(g.culture_channel_check(a, revolt, PlayerId(0), None), Err(CultureRefusal::OwnTeam));
    assert!(g.culture_channel_check(a, revolt, PlayerId(1), None).is_ok());
    g.setup.options[GameOptionType::GAMEOPTION_CULTURE_ACTIONS] = false;
    assert_eq!(g.culture_channel_check(a, revolt, PlayerId(1), None), Err(CultureRefusal::Off));
}

#[test]
fn channels_and_pressure_survive_a_save() {
    let (mut g, a, _, f) = game();
    let revolt = action(&g, "CULTURALACTION_FOMENT_REVOLT");
    g.city_set_culture_channel(a, Some((revolt, PlayerId(1), None))).unwrap();
    set_rate(&mut g, a, 30);
    set_rate(&mut g, f, 0);
    g.city_do_culture_channel(a);
    g.city_do_revolt_pressure(f);
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.cities[a].culture_channel, g.cities[a].culture_channel);
    assert_eq!(h.cities[f].culture_pressure, g.cities[f].culture_pressure);
    assert_eq!(h.save(), bytes);
}
