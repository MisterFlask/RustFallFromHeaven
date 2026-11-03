//! The odds the AI and the interface read (`Game::get_combat_odds`, a model of the duel) agree
//! with the duels the rules play (`unit_attack`). They are two separate pieces of code: a change
//! to the duel that reaches one and not the other would leave the AI misjudging every duel and the
//! player reading wrong odds, without a test failing anywhere else.
//!
//! The duel is the combat of ships and air units: land attacks are formation battles
//! (docs/ENGAGEMENTS.md), whose forecast is `engage_preview` (tests/engagement_preview.rs). So the
//! matchups here are naval: two ships on adjacent water plots.
//!
//! Each matchup is fought many times from the same position with different random seeds, and the
//! attacker's win rate must lie within a few standard deviations of the predicted odds.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// Trials per matchup.
const TRIALS: u32 = 400;

/// A generated duel at war at sea: `attacker` (player 0, with `damage`) on an empty water plot next
/// to another with `defender` (player 1). Two empty land plots side by side, away from the
/// players' units, are made ocean for it.
fn duel(attacker: &str, defender: &str, damage: i32) -> (Game, UnitId, UnitId, PlotIdx) {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let free = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && plot.units.is_empty() && plot.owner.is_none() && plot.plot_city.is_none()
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| free(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            free(&g, q).then_some((p, q))
        })
        .expect("two free land plots side by side");
    for p in [from, to] {
        g.plot_set_plot_type(p, PlotType::PLOT_OCEAN, true);
    }
    let (fx, fy) = g.map.xy(from);
    let (tx, ty) = g.map.xy(to);
    let a = g.player_init_unit(p0, d.t(attacker), fx, fy, None, None);
    let b = g.player_init_unit(p1, d.t(defender), tx, ty, None, None);
    g.units[a].damage = damage;
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    (g, a, b, to)
}

/// The attacker's win rate over [`TRIALS`] fights, and the predicted odds (both per mille).
fn measure(attacker: &str, defender: &str, damage: i32) -> (i32, i32) {
    let (g, a, b, to) = duel(attacker, defender, damage);
    let d = g.d;
    let odds = g.get_combat_odds(a, b);
    let save = g.save();
    let mut wins = 0;
    for trial in 0..TRIALS {
        let mut g = Game::load(d, &save).expect("load");
        g.soren_rand.reseed(1000 + trial);
        g.unit_attack(a, to);
        assert!(g.battle_reports.is_empty(), "a duel at sea, not an engagement");
        if !g.units.contains(b) && g.units.contains(a) {
            wins += 1;
        }
    }
    ((wins * 1000 / TRIALS) as i32, odds)
}

#[test]
fn predicted_odds_match_the_fights() {
    let mut failures = Vec::new();
    for (attacker, defender, damage) in [
        ("UNIT_TRIREME", "UNIT_TRIREME", 0),
        ("UNIT_TRIREME", "UNIT_TRIREME", 40),
        ("UNIT_PRIVATEER", "UNIT_TRIREME", 0),
        ("UNIT_TRIREME", "UNIT_PRIVATEER", 0),
        ("UNIT_FRIGATE", "UNIT_PRIVATEER", 30),
    ] {
        let (won, odds) = measure(attacker, defender, damage);
        // Four standard deviations of a binomial proportion, plus 2% for the model's rounding.
        let p = odds as f64 / 1000.0;
        let sigma = (p * (1.0 - p) / TRIALS as f64).sqrt();
        let tolerance = ((4.0 * sigma + 0.02) * 1000.0) as i32;
        eprintln!("{attacker} ({damage} damage) vs {defender}: won {won}, predicted {odds} (tolerance {tolerance})");
        if (won - odds).abs() > tolerance {
            failures.push(format!("{attacker} ({damage} damage) vs {defender}: won {won}/1000, predicted {odds}/1000"));
        }
    }
    assert!(failures.is_empty(), "combat odds disagree with combat: {failures:?}");
}
