//! Cultural actions (docs/CULTURE.md): the queue, channelling, the four core actions, the
//! public channels, saves.

use ffh_core::culture::{CultureOrder, CultureRefusal, CultureTarget};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const CULTURE: CommerceType = CommerceType::COMMERCE_CULTURE;

fn action(g: &Game, name: &str) -> CulturalActionType {
    g.d.t::<CulturalActionType>(name)
}

/// A duel map with cultural actions on, after `game_init`: a city of player 0 (`a`), another of
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

/// Gives player `p` culture on city `c`'s plot (the shares, its nationality, follow).
fn set_culture(g: &mut Game, c: CityId, p: PlayerId, culture: i32) {
    let plot = g.city_plot(c);
    g.map.plots[plot].culture[p] = culture;
}

#[test]
fn diverted_culture_reaches_neither_the_city_nor_its_plots() {
    let (mut g, a, _, _) = game();
    let festival = action(&g, "CULTURALACTION_FESTIVAL");
    set_rate(&mut g, a, 10);
    // Mid-level, so that neither game's city reaches another culture level (whose plot spread
    // differs) this turn.
    g.cities[a].culture[PlayerId(0)] = 4000;
    let mut with = Game::load(g.d, &g.save()).unwrap();
    with.city_push_culture_order(a, festival, CultureTarget::City(a), None, false, false).unwrap();
    let (p0, plot) = (PlayerId(0), g.city_plot(a));
    let (city0, plot0) = (g.cities[a].culture[p0], g.map.plots[plot].culture[p0]);
    g.city_do_turn(a);
    set_rate(&mut with, a, 10);
    with.city_do_turn(a);
    let rate = g.d.cultural_actions[festival].rate;
    assert_eq!(with.cities[a].culture_diverted, rate);
    assert_eq!(g.cities[a].culture[p0] - city0, (with.cities[a].culture[p0] - city0) + 100 * rate, "the city's culture misses the rate");
    assert_eq!(g.map.plots[plot].culture[p0] - plot0, (with.map.plots[plot].culture[p0] - plot0) + rate, "its plot's culture too");
}

#[test]
fn festival_completes_after_its_duration_and_wears_off() {
    let (mut g, a, b, _) = game();
    let festival = action(&g, "CULTURALACTION_FESTIVAL");
    set_rate(&mut g, a, 5);
    // A festival from one city on another of ours, in range.
    g.city_push_culture_order(a, festival, CultureTarget::City(b), None, false, false).unwrap();
    let happy = g.city_happy_level(b);
    let d = g.culture_action_duration(festival);
    for turn in 0..d {
        assert!(g.cities[b].culture_marks.is_empty(), "nothing before turn {turn}");
        set_rate(&mut g, a, 5);
        g.city_do_culture_channel(a);
    }
    assert!(g.cities[a].culture_queue.is_empty(), "done");
    assert_eq!(g.city_happy_level(b), happy + g.d.defines.culture_festival_happy);
    let lasts = g.culture_speed_turns(g.d.defines.culture_festival_turns);
    for _ in 0..lasts {
        assert_eq!(g.cities[b].culture_marks.len(), 1);
        g.city_do_culture_marks(b);
    }
    assert!(g.cities[b].culture_marks.is_empty(), "worn off after {lasts} turns");
}

#[test]
fn a_channel_pauses_without_its_rate_and_ends_after_the_stall() {
    let (mut g, a, _, _) = game();
    let festival = action(&g, "CULTURALACTION_FESTIVAL");
    set_rate(&mut g, a, 5);
    g.city_push_culture_order(a, festival, CultureTarget::City(a), None, false, false).unwrap();
    set_rate(&mut g, a, 5);
    g.city_do_culture_channel(a);
    assert_eq!(g.cities[a].culture_queue[0].progress, 1);
    let stall = g.d.defines.culture_channel_stall_turns;
    for i in 1..=stall {
        set_rate(&mut g, a, 1);
        g.city_do_culture_channel(a);
        assert_eq!(g.cities[a].culture_diverted, 0, "nothing diverted while paused");
        if i < stall {
            assert_eq!(g.cities[a].culture_queue[0].stalled, i);
            assert_eq!(g.cities[a].culture_queue[0].progress, 1, "no progress while paused");
        }
    }
    assert!(g.cities[a].culture_queue.is_empty(), "cancelled after {stall} paused turns");
}

#[test]
fn queueing_needs_the_rate_the_range_and_a_presence_abroad() {
    let (mut g, a, _, f) = game();
    let unrest = action(&g, "CULTURALACTION_INCITE_UNREST");
    let need = g.d.cultural_actions[unrest].rate;
    set_rate(&mut g, a, need - 1);
    assert_eq!(g.culture_check(a, unrest, CultureTarget::City(f), None, true), Err(CultureRefusal::Rate { have: need - 1, need }));
    set_rate(&mut g, a, need);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    set_culture(&mut g, f, p1, 95);
    set_culture(&mut g, f, p0, 5);
    assert!(matches!(g.culture_check(a, unrest, CultureTarget::City(f), None, true), Err(CultureRefusal::Share { have: 5, .. })));
    set_culture(&mut g, f, p0, 12);
    // 12 / (12 + 95) = 11% < the minimum effect
    assert!(matches!(g.culture_check(a, unrest, CultureTarget::City(f), None, true), Err(CultureRefusal::Effect { .. })));
    set_culture(&mut g, f, p0, 40);
    set_culture(&mut g, f, p1, 60);
    let p = g.culture_check(a, unrest, CultureTarget::City(f), None, true).expect("possible");
    assert!(p.hostile);
    assert_eq!(p.effectiveness, 40);
    // Not on our own city.
    assert!(matches!(g.culture_check(a, unrest, CultureTarget::City(a), None, true), Err(CultureRefusal::Target(_))));
}

#[test]
fn effectiveness_follows_the_shares_and_the_defences() {
    let (mut g, a, _, f) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let unrest = action(&g, "CULTURALACTION_INCITE_UNREST");
    set_culture(&mut g, f, p0, 30);
    set_culture(&mut g, f, p1, 70);
    // No garrison yet: E = 30 / (30 + 70) (only hostile actions meet the defences).
    let garrison = g.culture_garrison_count(f);
    let base = g.culture_effectiveness(unrest, p0, f, true);
    let resist = g.d.defines.culture_garrison_resist * garrison;
    assert_eq!(base, 100 * 30 * 100 / (30 * 100 + 70 * g.d.defines.culture_resist_k + resist * 100));
    // A festival in the city: M + 0.5.
    g.city_add_culture_mark(f, ffh_core::culture::CultureMark { effect: CultureEffect::Festival, from: p1, amount: 1, turns: 5 });
    let defended = g.culture_effectiveness(unrest, p0, f, true);
    let m = resist + g.d.defines.culture_festival_resist;
    assert_eq!(defended, 100 * 30 * 100 / (30 * 100 + 70 * g.d.defines.culture_resist_k + m * 100));
    assert!(defended < base);
    // A friendly action ignores the defences.
    let fair = action(&g, "CULTURALACTION_TRADE_FAIR");
    assert_eq!(g.culture_effectiveness(fair, p0, f, false), 30);
    let _ = a;
}

#[test]
fn incite_unrest_adds_unhappiness_scaled_by_effectiveness() {
    let (mut g, a, _, f) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let unrest = action(&g, "CULTURALACTION_INCITE_UNREST");
    set_culture(&mut g, f, p0, 50);
    set_culture(&mut g, f, p1, 50);
    set_rate(&mut g, a, 10);
    g.city_push_culture_order(a, unrest, CultureTarget::City(f), None, false, false).unwrap();
    let start_log = g.log.entries.len();
    let before = g.city_culture_unhappiness(f);
    for _ in 0..g.culture_action_duration(unrest) {
        set_rate(&mut g, a, 10);
        g.city_do_culture_channel(a);
    }
    let e = g.culture_effectiveness(unrest, p0, f, true);
    let expect = ((g.d.defines.culture_unrest_unhappy * e + 50) / 100).max(1);
    assert_eq!(g.city_culture_unhappiness(f) - before, expect);
    // The target's owner was told, and remembers it.
    assert!(g.log.entries[start_log..].iter().any(|m| m.player == Some(1)), "the target's owner hears of it");
    assert_eq!(g.players[p1].culture_memory[p0][0], g.d.cultural_actions[unrest].memory_weight);
}

#[test]
fn the_start_of_a_hostile_channel_is_announced_to_the_target() {
    let (mut g, a, _, f) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let unrest = action(&g, "CULTURALACTION_INCITE_UNREST");
    set_culture(&mut g, f, p0, 50);
    set_culture(&mut g, f, p1, 50);
    set_rate(&mut g, a, 10);
    let n = g.log.entries.len();
    g.city_push_culture_order(a, unrest, CultureTarget::City(f), None, false, false).unwrap();
    assert!(g.log.entries[n..].iter().any(|m| m.player == Some(1) && m.text.contains("has begun")));
    // Public: met civilizations see the channel; it is listed against the city.
    let t1 = g.players[p1].team;
    let chans = g.culture_hostile_channels_at(f);
    assert_eq!(chans.len(), 1);
    assert!(g.culture_channel_visible(t1, chans[0].caster));
}

#[test]
fn trade_fair_raises_the_routes_between_the_two() {
    let (mut g, a, _, f) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let fair = action(&g, "CULTURALACTION_TRADE_FAIR");
    set_culture(&mut g, f, p0, 40);
    set_culture(&mut g, f, p1, 60);
    set_rate(&mut g, a, 10);
    match g.culture_check(a, fair, CultureTarget::City(f), None, true) {
        Err(CultureRefusal::NoTrade) => {
            // No trade with them (no open borders): the fair needs trade routes.
            return;
        }
        r => {
            r.expect("a fair");
        }
    }
    g.city_push_culture_order(a, fair, CultureTarget::City(f), None, false, false).unwrap();
    let base = g.city_total_trade_modifier(a, Some(f));
    for _ in 0..g.culture_action_duration(fair) {
        set_rate(&mut g, a, 10);
        g.city_do_culture_channel(a);
    }
    let pct = g.cities[f].culture_marks[0].amount;
    assert_eq!(g.city_total_trade_modifier(a, Some(f)), base + pct);
    assert_eq!(g.city_trade_fair_modifier(f, a), pct, "both sides");
}

#[test]
fn spread_religion_goes_through_the_religion_rules() {
    let (mut g, a, b, _) = game();
    let spread = action(&g, "CULTURALACTION_SPREAD_RELIGION");
    let r = g.d.religions.keys().next().unwrap();
    // Not without the religion in the casting city.
    set_rate(&mut g, a, 10);
    assert!(matches!(g.culture_check(a, spread, CultureTarget::City(b), Some(r), true), Err(CultureRefusal::Religion(_))));
    g.city_set_has_religion(a, r, true, false, false);
    g.city_push_culture_order(a, spread, CultureTarget::City(b), Some(r), false, false).unwrap();
    for _ in 0..g.culture_action_duration(spread) {
        set_rate(&mut g, a, 10);
        g.city_do_culture_channel(a);
    }
    // Our own city: E = 100%, so the chance is CULTURE_SPREAD_BASE; either it spread, or the
    // message says it failed.
    assert!(g.cities[a].culture_queue.is_empty());
    assert!(g.cities[b].has_religion[r] || g.log.entries.iter().any(|m| m.text.contains("failed to spread")));
}

#[test]
fn a_lost_target_ends_the_channels_aimed_at_it() {
    let (mut g, a, _, f) = game();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let unrest = action(&g, "CULTURALACTION_INCITE_UNREST");
    set_culture(&mut g, f, p0, 50);
    set_culture(&mut g, f, p1, 50);
    set_rate(&mut g, a, 10);
    g.city_push_culture_order(a, unrest, CultureTarget::City(f), None, false, false).unwrap();
    g.city_kill(f, true);
    assert!(g.cities[a].culture_queue.is_empty());
}

#[test]
fn queues_and_marks_survive_a_save() {
    let (mut g, a, b, _) = game();
    let festival = action(&g, "CULTURALACTION_FESTIVAL");
    set_rate(&mut g, a, 10);
    g.city_push_culture_order(a, festival, CultureTarget::City(b), None, true, false).unwrap();
    g.city_add_culture_mark(b, ffh_core::culture::CultureMark { effect: CultureEffect::InciteUnrest, from: PlayerId(1), amount: 2, turns: 3 });
    g.players[PlayerId(0)].culture_memory[PlayerId(1)] = [3, 1];
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    let want = CultureOrder { action: festival, target: CultureTarget::City(b), religion: None, repeat: true, progress: 0, stalled: 0, started_turn: g.game_turn() };
    assert_eq!(h.cities[a].culture_queue, vec![want]);
    assert_eq!(h.cities[b].culture_marks, g.cities[b].culture_marks);
    assert_eq!(h.players[PlayerId(0)].culture_memory[PlayerId(1)], [3, 1]);
    assert_eq!(h.save(), bytes);
}

#[test]
fn off_without_the_option() {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let p0 = PlayerId(0);
    let u = g.players[p0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let a = g.player_init_city(p0, x, y, true, true);
    let festival = action(&g, "CULTURALACTION_FESTIVAL");
    set_rate(&mut g, a, 10);
    assert_eq!(g.culture_check(a, festival, CultureTarget::City(a), None, true), Err(CultureRefusal::Off));
}
