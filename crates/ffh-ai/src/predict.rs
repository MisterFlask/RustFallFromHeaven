//! What the AI's estimators predict, for the prediction ledger (`ffh_core::telemetry`,
//! docs/CALIBRATION.md): the horizons, and the recording shared by several call sites. Recording
//! only: nothing here changes a decision, and every call does nothing with telemetry off.

use ffh_core::engagement::WarTest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::telemetry::{Obj, Outcome, Prediction, HORIZON_THIS_TURN};

use crate::combat_sim::Sim;
use crate::forecast::{Cache, Est};
use crate::reach::Flood;

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

/// `field2`: the rules' forecast ([`crate::forecast`]) of the battles player `p`'s attack from
/// `from` on `q` fights this turn (the pool of `p`'s units on `from` that can attack it, as the
/// rules pick it, then its reserve: [`crate::forecast::attack_turn`]), beside `field`: the same
/// outcome and horizon, pct the chance the plot falls, `acted` the old estimate's decision; ctx
/// `one`: the chance it falls in the first battle alone.
pub(crate) fn field2(g: &Game, cache: Option<&Cache>, p: PlayerId, from: PlotIdx, q: PlotIdx, city: bool, acted: bool, src: &'static str) {
    if !g.telemetry.on() {
        return;
    }
    let pool = g.engage_attackers(p, from, q, WarTest::AtWar);
    if pool.is_empty() {
        return;
    }
    let attacks = [(pool.clone(), from)];
    let Some(e) = crate::forecast::attack_turn(g, cache, &attacks, q) else { return };
    let one = crate::forecast::attack(g, cache, &pool, from, q).map_or(-1, |o| o.p_take as i64);
    let team = g.players[p].team;
    g.tel_prediction(p, Prediction::new("field2", Outcome::StackClears { team, plot: q }, HORIZON_THIS_TURN, e.p_take / 10).score(e.p_take as i64).acted(acted), || {
        est_ctx(&e).i("one", one).s("src", src).b("city", city).end()
    });
}

/// The facts of an [`Est`] for a prediction's `ctx`.
fn est_ctx(e: &Est) -> Obj {
    Obj::new()
        .i("p_take", e.p_take as i64)
        .i("p_wipe", e.p_wipe as i64)
        .i("p_repulsed", e.p_repulsed as i64)
        .i("own_lost_x100", e.own_lost_x100 as i64)
        .i("enemy_lost_x100", e.enemy_lost_x100 as i64)
        .i("n_att", e.n_att as i64)
        .i("n_def", e.n_def as i64)
}

/// `assault2` and `strike2` (`kind`): the rules' forecast of our `units` next to `city` taking it
/// this turn: the battles they fight one after another, the plot with the most of them first,
/// each plot's pool then its reserve ([`crate::forecast::by_plot`], [`crate::forecast::assault`]),
/// beside `assault` and `strike`: the same outcome and horizon, the old decision's `acted`; ctx
/// `one`: the chance the first battle alone takes it.
pub(crate) fn city2(g: &Game, cache: Option<&Cache>, kind: &'static str, p: PlayerId, city: CityId, units: &[UnitId], horizon: i32, acted: bool) {
    if !g.telemetry.on() || units.is_empty() || !g.cities.contains(city) {
        return;
    }
    let tp = g.city_plot(city);
    let attacks = crate::forecast::by_plot(g, units, tp);
    if attacks.is_empty() {
        return;
    }
    let Some(e) = crate::forecast::assault(g, cache, &attacks, city) else { return };
    let one = crate::forecast::attack(g, cache, &attacks[0].0, attacks[0].1, tp).map_or(-1, |o| o.p_take as i64);
    let team = g.players[p].team;
    let n_plots = attacks.len() as i64;
    g.tel_prediction(p, Prediction::new(kind, Outcome::CityCapturedBy { city, team }, horizon, e.p_take / 10).score(e.p_take as i64).acted(acted), || {
        est_ctx(&e).i("one", one).i("n_units", units.len() as i64).i("n_plots", n_plots).end()
    });
}

/// `attack2`: unit `u`'s attack on `q` under engagements: the battle its plot's pool fights (the
/// rules' forecast), pct the chance the defender's line is wiped out (the plot taken or cleared) times the
/// chance `u` does not fall (the ledger's `AttackWon`: its side takes or clears the plot and it does not
/// fall; the two treated as independent), beside `attack`.
pub(crate) fn attack2(g: &Game, cache: Option<&Cache>, p: PlayerId, u: UnitId, q: PlotIdx, src: &'static str) {
    if !g.telemetry.on() || !g.units.contains(u) {
        return;
    }
    let from = g.unit_plot(u);
    let pool = g.engage_attackers(p, from, q, WarTest::AtWar);
    if !pool.contains(&u) {
        return;
    }
    let Some(f) = crate::forecast::attack_full(g, cache, &pool, from, q) else { return };
    let e = Est::of(&f);
    let fall = f.per.iter().find(|x| x.unit == u).map(|x| x.fall_pct);
    let pct = e.p_wipe * (100 - fall.unwrap_or(0)) / 1000;
    g.tel_prediction(p, Prediction::new("attack2", Outcome::AttackWon { attacker: u }, HORIZON_THIS_TURN, pct).score(e.p_take as i64).acted(true), || {
        est_ctx(&e).s("src", src).i("fall_pct", fall.map_or(-1, |x| x as i64)).b("committed", fall.is_some()).end()
    });
}

/// `holds2_now` and `holds2_reinforced`: the defensive forecast of city `c` ([`forecast::holds`])
/// beside `peril`: the city is lost within [`CITY_LOST_HORIZON`] turns, pct its chance to fall as
/// the garrison stands and with the reinforcements that can arrive first; de-duplicated by
/// answer. Nothing when no stack threatens it.
pub(crate) fn holds2(g: &Game, cache: Option<&Cache>, flood: &mut Flood, p: PlayerId, c: CityId, peril: bool, threat: i32, defense: i32) {
    if !g.telemetry.on() {
        return;
    }
    let h = crate::forecast::holds(g, cache, flood, c);
    if !h.threat {
        return;
    }
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let owners = format!("[{}]", h.owners.iter().map(|o| o.0.to_string()).collect::<Vec<_>>().join(","));
    let barbarian = h.owners.iter().any(|&o| g.player_is_barbarian(o));
    for (kind, permille) in [("holds2_now", h.p_fall_now), ("holds2_reinforced", h.p_fall_reinforced)] {
        let pr = Prediction::new(kind, Outcome::CityLost { city: c }, CITY_LOST_HORIZON, permille / 10).score(permille as i64).per_answer();
        g.tel_prediction(p, pr, || {
            Obj::new()
                .i("now", h.p_fall_now as i64)
                .i("reinforced", h.p_fall_reinforced as i64)
                .i("single_now", h.single_now as i64)
                .i("union_now", h.union_now as i64)
                .i("single_reinforced", h.single_reinforced as i64)
                .i("union_reinforced", h.union_reinforced as i64)
                .i("reinforcements", h.reinforcements.len() as i64)
                .i("turns", h.turns as i64)
                .i("stacks", h.stacks as i64)
                .i("enemy_units", h.enemy_units as i64)
                .i("garrison", crate::production::garrison(g, p, g.city_plot(c)).0 as i64)
                .b("peril", peril)
                .i("threat", threat as i64)
                .i("defense", defense as i64)
                .i("x", x as i64)
                .i("y", y as i64)
                .raw("owners", &owners)
                .b("barbarian", barbarian)
                .end()
        });
    }
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
