//! The odds the AI and the interface read (`Game::get_combat_odds`, a model of the fight) agree
//! with the fights the rules play (`unit_attack`). They are two separate pieces of code: a change
//! to combat that reaches one and not the other would leave the AI misjudging every attack and the
//! player reading wrong odds, without a test failing anywhere else.
//!
//! Each matchup is fought many times from the same position with different random seeds, and the
//! attacker's win rate must lie within a few standard deviations of the predicted odds.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::DirectionType;

/// Trials per matchup.
const TRIALS: u32 = 400;

/// A generated duel at war, with `attacker` (player 0) on a flat land plot next to `defender`
/// (player 1).
fn duel(attacker: &str, defender: &str) -> (Game, UnitId, UnitId, PlotIdx) {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && !g.plot_is_peak(p) && !g.plot_is_hills(p) && plot.units.is_empty() && plot.owner.is_none() && plot.feature_type.is_none() && plot.improvement_type.is_none()
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((p, q))
        })
        .expect("two flat plots side by side");
    let (fx, fy) = g.map.xy(from);
    let (tx, ty) = g.map.xy(to);
    let a = g.player_init_unit(p0, d.t(attacker), fx, fy, None, None);
    let b = g.player_init_unit(p1, d.t(defender), tx, ty, None, None);
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    (g, a, b, to)
}

/// The attacker's win rate over [`TRIALS`] fights, and the predicted odds (both per mille).
fn measure(attacker: &str, defender: &str) -> (i32, i32) {
    let (g, a, b, to) = duel(attacker, defender);
    let d = g.d;
    let odds = g.get_combat_odds(a, b);
    let save = g.save();
    let mut wins = 0;
    for trial in 0..TRIALS {
        let mut g = Game::load(d, &save).expect("load");
        g.soren_rand.reseed(1000 + trial);
        g.unit_attack(a, to);
        if !g.units.contains(b) && g.units.contains(a) {
            wins += 1;
        }
    }
    ((wins * 1000 / TRIALS) as i32, odds)
}

#[test]
fn predicted_odds_match_the_fights() {
    let mut failures = Vec::new();
    for (attacker, defender) in [
        ("UNIT_WARRIOR", "UNIT_WARRIOR"),
        ("UNIT_AXEMAN", "UNIT_WARRIOR"),
        ("UNIT_WARRIOR", "UNIT_AXEMAN"),
        ("UNIT_HORSEMAN", "UNIT_ARCHER"),
        ("UNIT_AXEMAN", "UNIT_ARCHER"),
    ] {
        let (won, odds) = measure(attacker, defender);
        // Four standard deviations of a binomial proportion, plus 2% for the model's rounding.
        let p = odds as f64 / 1000.0;
        let sigma = (p * (1.0 - p) / TRIALS as f64).sqrt();
        let tolerance = ((4.0 * sigma + 0.02) * 1000.0) as i32;
        eprintln!("{attacker} vs {defender}: won {won}, predicted {odds} (tolerance {tolerance})");
        if (won - odds).abs() > tolerance {
            failures.push(format!("{attacker} vs {defender}: won {won}/1000, predicted {odds}/1000"));
        }
    }
    assert!(failures.is_empty(), "combat odds disagree with combat: {failures:?}");
}
