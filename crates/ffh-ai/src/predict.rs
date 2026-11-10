//! What the AI's estimators predict, for the prediction ledger (`ffh_core::telemetry`,
//! docs/CALIBRATION.md): the horizons, and the recording shared by several call sites. Recording
//! only: nothing here changes a decision, and every call does nothing with telemetry off.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::telemetry::{Obj, Outcome, Prediction, HORIZON_THIS_TURN};

use crate::combat_sim::Sim;

/// `assault`: the adjacent ring takes the city within this many turns.
pub const ASSAULT_HORIZON: i32 = 3;
/// `garrison_holds`, `peril`: the city is lost within this many turns.
pub const CITY_LOST_HORIZON: i32 = 10;
/// `sortie`: the city a defender sorties from is lost within this many turns.
pub const SORTIE_HORIZON: i32 = 3;
/// `war_takes_city`: a war takes a city of the target within this many turns of its declaration.
pub const WAR_HORIZON: i32 = 40;
/// `fight_on`: a war fought on because our army is at the enemy's next city takes a city of
/// theirs within this many turns.
pub const FIGHT_ON_HORIZON: i32 = 10;

/// `field`: the group attack [`crate::military::field_sim`] played on plot `q` clears it this turn
/// (pct: the simulation's chance every defender falls; score: its `gain`), acted on or not.
pub(crate) fn field(g: &Game, p: PlayerId, q: PlotIdx, s: &Sim, city: bool, acted: bool, src: &'static str) {
    if !g.telemetry.on() || s.n_def == 0 {
        return;
    }
    let team = g.players[p].team;
    let barb = g.map.plots[q].units.iter().any(|&u| g.player_is_barbarian(g.units[u].owner));
    g.tel_prediction(p, Prediction::new("field", Outcome::StackClears { team, plot: q }, HORIZON_THIS_TURN, s.p_fall / 10).score(s.gain).acted(acted), || {
        Obj::new()
            .s("src", src)
            .b("city", city)
            .b("barbarian", barb)
            .b("covers", s.used.is_some())
            .i("first", s.first as i64)
            .i("best", s.best as i64)
            .i("n_def", s.n_def as i64)
            .i("n_att", s.n_att as i64)
            .end()
    });
}

/// `attack`: unit `u` wins the attack it is about to make on plot `q` (pct: the odds, per
/// mille, the AI weighed it by).
pub(crate) fn attack(g: &Game, p: PlayerId, u: UnitId, q: PlotIdx, odds: i32, src: &'static str) {
    if !g.telemetry.on() {
        return;
    }
    let city = g.map.plots[q].plot_city.is_some();
    let barb = g.map.plots[q].units.iter().any(|&e| g.player_is_barbarian(g.units[e].owner));
    g.tel_prediction(p, Prediction::new("attack", Outcome::AttackWon { attacker: u }, HORIZON_THIS_TURN, odds / 10).score(odds as i64).acted(true), || {
        Obj::new().s("src", src).b("city", city).b("barbarian", barb).s("unit", g.d.units[g.units[u].unit_type].base.type_.as_str()).end()
    });
}
