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

/// `incoming3`, `incoming6`, `incoming10`: the incoming-attack estimate ([`crate::threat`]) of
/// each city of `p` with a hostile stack in reach (and, at war with a civilization, of every other
/// city, pct 0): the city is attacked within 3 / 6 / 10 turns. `lost2_as_is`, `lost2_reinforced`:
/// it is lost within [`CITY_LOST_HORIZON`] turns, pct the chance it is attacked within them times
/// the forecast of the expected force against the garrison as it stands / with the units of ours
/// that arrive before it. All by answer. With events written, one `decision` event `threat` per
/// call with every city's estimate and every stack (the misses' analysis reads it).
///
/// Recorded only with `FFH_AI_SHADOW_THREAT=1`: the estimator costs about 2.8 times the game's own
/// time as built (docs/CALIBRATION.md), which every telemetry run would otherwise pay.
pub(crate) fn incoming(g: &Game, cache: &Cache, flood: &mut Flood, p: PlayerId, civ_war: bool) {
    use crate::threat::{incoming, Incoming, Threats, HORIZONS};
    if !g.telemetry.on() || !shadow_threat() || g.players[p].cities.is_empty() {
        return;
    }
    let t = Threats::of(g, Some(cache), flood, p);
    let (mut floods, mut forecasts) = (t.floods as i64, t.forecasts as i64);
    let mut rows: Vec<(CityId, Incoming)> = Vec::new();
    let mut threatened = 0;
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) {
            continue;
        }
        let inc = incoming(g, Some(cache), flood, &t, c);
        floods += inc.floods as i64;
        forecasts += inc.forecasts as i64;
        let seen = inc.owners > 0;
        threatened += seen as i64;
        if seen || civ_war {
            let (x, y) = (g.cities[c].x, g.cities[c].y);
            let barbarian = t.about(c).any(|(f, _)| f.barbarian);
            let at_sea = t.stacks.iter().any(|s| s.at_sea && s.cands.iter().any(|x| x.city == c));
            let enemy_units: i64 = inc.force.iter().map(|x| x.0.len() as i64).sum();
            let best_take = t.about(c).map(|(f, i)| f.cities[i].p_take as i64).max().unwrap_or(-1);
            let garrison = crate::production::garrison(g, p, g.city_plot(c)).0 as i64;
            let ctx = |inc: &Incoming| {
                Obj::new()
                    .b("stack", seen)
                    .s("kind", inc.kind)
                    .i("owners", inc.owners as i64)
                    .i("stacks", inc.stacks as i64)
                    .i("takers", inc.takers as i64)
                    .i("turns", inc.turns_until.map_or(-1, |x| x as i64))
                    .i("p1", inc.p_attacked_within[0] as i64)
                    .i("p3", inc.p_attacked_within[1] as i64)
                    .i("p6", inc.p_attacked_within[2] as i64)
                    .i("p10", inc.p_attacked_within[3] as i64)
                    .i("as_is", inc.p_lost_as_is as i64)
                    .i("reinforced", inc.p_lost_reinforced as i64)
                    .i("reinforcements", inc.reinforcements.len() as i64)
                    .i("best_take", best_take)
                    .i("enemy_units", enemy_units)
                    .i("garrison", garrison)
                    .b("barbarian", barbarian)
                    .b("at_sea", at_sea)
                    .b("civ_war", civ_war)
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .end()
            };
            for (k, kind) in [(1, "incoming3"), (2, "incoming6"), (3, "incoming10")] {
                let pr = Prediction::new(kind, Outcome::CityAttackedBy { city: c, team: None }, HORIZONS[k], inc.p_attacked_within[k]).per_answer();
                g.tel_prediction(p, pr, || ctx(&inc));
            }
            let p10 = inc.p_attacked_within[3];
            for (kind, permille) in [("lost2_as_is", inc.p_lost_as_is), ("lost2_reinforced", inc.p_lost_reinforced)] {
                let pr = Prediction::new(kind, Outcome::CityLost { city: c }, CITY_LOST_HORIZON, p10 * permille / 1000).score(permille as i64).per_answer();
                g.tel_prediction(p, pr, || ctx(&inc));
            }
        }
        rows.push((c, inc));
    }
    // The trace of the call: every city's estimate (with the military of the civilizations we are
    // not at war with near it) and every stack.
    g.tel_decision(p, "threat", "turn", "", threatened, || threat_trace(g, p, &t, &rows, floods, forecasts));
    g.tel_decision(p, "threat", "floods", "", floods, String::new);
    g.tel_decision(p, "threat", "forecasts", "", forecasts, String::new);
}

/// The trace's radius for the military of civilizations we are not at war with near a city.
const NEUTRAL_RADIUS: i32 = 4;

/// The JSON of [`incoming`]'s trace: `cities` as `[id, x, y, p1, p3, p6, p10, as_is, reinforced,
/// turns, [[owner, units]..]]` (the last: land military of civilizations not at war with us within
/// [`NEUTRAL_RADIUS`]), `stacks` as `{o, n, x, y, sea, c: [[city, turns]..]}`, `forces` as `{o,
/// barb, t (the target city, -1: none), c: [[city, turns, when it takes it (-1: never), p_take]..]}`,
/// `far` as `[[owner, units]..]` (hostile units that reach none of our cities).
fn threat_trace(g: &Game, p: PlayerId, t: &crate::threat::Threats, rows: &[(CityId, crate::threat::Incoming)], floods: i64, forecasts: i64) -> String {
    let team = g.players[p].team;
    let cities: Vec<String> = rows
        .iter()
        .map(|(c, inc)| {
            let (x, y) = (g.cities[*c].x, g.cities[*c].y);
            let mut nbr: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
            for dx in -NEUTRAL_RADIUS..=NEUTRAL_RADIUS {
                for dy in -NEUTRAL_RADIUS..=NEUTRAL_RADIUS {
                    let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                    for &u in &g.map.plots[q].units {
                        let o = g.units[u].owner;
                        let ot = g.players[o].team;
                        if ot == team || g.at_war(team, ot) || g.player_is_barbarian(o) || g.unit_is_invisible(u, team, false) {
                            continue;
                        }
                        if g.d.units[g.units[u].unit_type].animal || !g.unit_can_attack(u) || g.unit_get_domain_type(u) != ffh_data::DomainType::DOMAIN_LAND {
                            continue;
                        }
                        *nbr.entry(o.0 as i64).or_insert(0) += 1;
                    }
                }
            }
            let nbr: Vec<String> = nbr.iter().map(|(o, n)| format!("[{o},{n}]")).collect();
            let a = &inc.p_attacked_within;
            format!("[{},{},{},{},{},{},{},{},{},{},[{}]]", c.0, x, y, a[0], a[1], a[2], a[3], inc.p_lost_as_is, inc.p_lost_reinforced, inc.turns_until.unwrap_or(-1), nbr.join(","))
        })
        .collect();
    let stacks: Vec<String> = t
        .stacks
        .iter()
        .map(|s| {
            let (x, y) = g.map.xy(s.at);
            let cands: Vec<String> = s.cands.iter().map(|c| format!("[{},{}]", c.city.0, c.turns)).collect();
            Obj::new()
                .i("o", s.owner.0 as i64)
                .i("n", s.units.len() as i64)
                .i("x", x as i64)
                .i("y", y as i64)
                .b("sea", s.at_sea)
                .raw("c", &format!("[{}]", cands.join(",")))
                .end()
        })
        .collect();
    let forces: Vec<String> = t
        .forces
        .iter()
        .map(|f| {
            let cs: Vec<String> = f.cities.iter().enumerate().map(|(i, c)| format!("[{},{},{},{}]", c.city.0, c.turns, f.when(i).unwrap_or(-1), c.p_take)).collect();
            Obj::new()
                .i("o", f.owner.0 as i64)
                .b("barb", f.barbarian)
                .i("t", f.target.map_or(-1, |i| f.cities[i].city.0 as i64))
                .raw("c", &format!("[{}]", cs.join(",")))
                .end()
        })
        .collect();
    Obj::new()
        .i("floods", floods)
        .i("forecasts", forecasts)
        .raw("cities", &format!("[{}]", cities.join(",")))
        .raw("stacks", &format!("[{}]", stacks.join(",")))
        .raw("forces", &format!("[{}]", forces.join(",")))
        .raw("far", &format!("[{}]", t.far.iter().map(|(o, n)| format!("[{},{n}]", o.0)).collect::<Vec<_>>().join(",")))
        .end()
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

/// `FFH_AI_SHADOW_THREAT=1`: record the incoming-attack estimator's shadow predictions (read once;
/// recording changes no decision, so play is the same either way).
fn shadow_threat() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FFH_AI_SHADOW_THREAT").map_or(false, |v| !v.is_empty() && v != "0"))
}
