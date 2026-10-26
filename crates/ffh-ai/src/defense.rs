//! The standing army against rival civilizations (switch `RIVAL_DEFENSE`): how many defenders
//! beyond the barbarian and capital quotas our cities hold, and where.
//!
//! **In peacetime: deterrence** ([`deterrence`]). An expert keeps enough army that no neighbour who
//! can reach him and would fight him sees an easy target, and puts it where that neighbour's army
//! would land. Every AI here starts a war of expansion through the same gate
//! (`diplomacy::territory_check`): its power, with the power of the civs already at war with the
//! target, at least its leader's need ratio (85-180% by temperament, more when broke) of the
//! target's defensive power, the target within 16 plots, the target's weakest city takeable by its
//! best attacker, and a cautious leader sparing a civ it likes. So for each neighbour that passes
//! the other tests, the defensive power that keeps its ratio under its bar with a 10% margin; the
//! largest shortfall against what we hold, in our best defender's power, becomes defenders in the
//! cities facing that neighbour, nearest first, at most three a city. No neighbour that would see
//! an easy target: no extra defender. (Before: one or two defenders wherever More Naval AI's
//! closeness x attitude x power index passed a threshold, whatever our army already deterred.)
//!
//! **At war** (or preparing one) the flat defender every city added before is a pool of one per
//! city, shared out by each city's share of the war threat -- More Naval AI's
//! `CvCityAI::AI_cityThreat` and floating defenders: each enemy's closeness to the city (its
//! cities on the landmass within 12 plots, `(20 + both populations) x (13 - distance) / 13`, the
//! nearest in full and the rest a quarter) times its hostility (at war 3, our war in preparation
//! 1.8) times its power over ours when it is the stronger (up to 3). The front holds two or three,
//! the interior none, at the old cost.
//!
//! Production (`production::decide`), the defenders' placement (`units::garrison_quota`: spare
//! defenders walk to the border) and the upkeep scrapping read the same number.
//!
//! **Sorties** (More Naval AI's `AI_leaveAttack(1, 70, 175)`): a defender beyond its city's quota
//! attacks out only while our strength in the city is at least 175% of the enemy strength within
//! two plots.
//!
//! Only what a human could see is read (cities, power, wars, attitudes, leaders' temperaments);
//! no state, no RNG.

use std::collections::BTreeMap;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::view::{attack_power, defense_power};

/// Plots within which a rival city counts in peacetime (More Naval AI's `DEFAULT_PLAYER_CLOSENESS`
/// is 7 path steps; plot distance runs a little shorter), and at war (armies march further than a
/// raid).
const CLOSE: i32 = 8;
const CLOSE_WAR: i32 = 12;
/// Defensive power wanted over what a rival's bar asks, percent.
const DETER_MARGIN: i64 = 110;
/// A rival's cities within this many plots of ours: its army can reach us (the war gate's 16).
const REACH: i32 = 16;
/// Most extra defenders a city holds for its rivals.
const MAX_EXTRA: i32 = 3;
/// Strength in the city against the enemy's within two plots, percent.
const SORTIE_EDGE: i64 = 175;

/// A rival civilization and its weight (hostility x power, x10000).
#[derive(Copy, Clone, Debug)]
pub struct Rival {
    pub player: PlayerId,
    pub weight: i64,
    /// At war with us, or the target of the war we prepare: weighs on the war pool.
    pub war: bool,
}

/// The rivals of `p` that weigh on its cities (met, alive, with cities, not our team, not the
/// barbarians), with their weights.
pub fn rivals(g: &Game, p: PlayerId) -> Vec<Rival> {
    let team = g.players[p].team;
    let ours = g.team_get_defensive_power(team).max(1) as i64;
    let preparing = crate::diplomacy::preparing_target(g, team);
    let mut out = Vec::new();
    let mut power: BTreeMap<TeamId, i64> = BTreeMap::new();
    for r in g.player_ids() {
        let pl = &g.players[r];
        let t = pl.team;
        if r == p || !pl.alive || pl.cities.is_empty() || t == team || g.player_is_barbarian(r) || !g.teams[team].has_met[t] {
            continue;
        }
        let at_war = g.at_war(team, t);
        let hostility: i64 = if at_war {
            300
        } else if preparing == Some(t) {
            180
        } else if g.teams[t].vassal[team] || g.teams[team].vassal[t] {
            30
        } else {
            match crate::diplomacy::attitude(g, r, p) {
                AttitudeType::ATTITUDE_FURIOUS => 180,
                AttitudeType::ATTITUDE_ANNOYED => 130,
                AttitudeType::ATTITUDE_CAUTIOUS => 100,
                AttitudeType::ATTITUDE_PLEASED => 50,
                _ => 20,
            }
        };
        // Their power over ours counts only when they are the stronger (as in C++).
        let theirs = *power.entry(t).or_insert_with(|| g.team_get_power(t, true) as i64);
        let factor = (theirs * 100 / ours).clamp(100, 300);
        out.push(Rival { player: r, weight: hostility * factor, war: at_war || preparing == Some(t) });
    }
    out
}

/// The threat to our city `c` from the rivals at war with us (`war`) or the others (see the module
/// doc).
pub fn city_threat(g: &Game, c: CityId, rivals: &[Rival], war: bool) -> i64 {
    let city = &g.cities[c];
    let area = g.map.plots[g.city_plot(c)].area;
    let close = if war { CLOSE_WAR } else { CLOSE };
    let mut threat = 0i64;
    for r in rivals.iter().filter(|r| r.war == war) {
        let mut best = 0i64;
        let mut sum = 0i64;
        for &o in &g.players[r.player].cities {
            let other = &g.cities[o];
            let d = g.plot_distance(city.x, city.y, other.x, other.y);
            if d > close || g.map.plots[g.city_plot(o)].area != area {
                continue;
            }
            let v = (20 + other.population as i64 + city.population as i64) * (close + 1 - d) as i64 / (close + 1) as i64;
            best = best.max(v);
            sum += v;
        }
        let closeness = best + sum / 4;
        threat += closeness * r.weight / 10000;
    }
    threat
}

/// A neighbour that would see us as an easy target, and how much defensive power we lack to
/// deter it.
#[derive(Copy, Clone, Debug)]
pub struct Deterrence {
    pub rival: PlayerId,
    pub shortfall: i64,
}

/// The peacetime standing army an expert keeps: enough that no neighbour who can reach us and
/// would fight us sees an easy target. Every AI here starts a war of expansion through the same
/// gate (`diplomacy::territory_check`): its power, with the power of the civs already at war with
/// the target, at least its leader's need ratio (85-180% by temperament, more when broke) of the
/// target's defensive power; the target's cities within 16 plots; the target's weakest city
/// takeable by its best attacker; and a leader who spares friends does not attack a civ it likes.
/// For each neighbour that passes the other tests, the defensive power that keeps its ratio under
/// its bar with a margin ([`DETER_MARGIN`]); the largest shortfall against what we hold is the one
/// to close. None when no neighbour sees an easy target (or at war with it: the war pool decides).
pub fn deterrence(g: &Game, p: PlayerId) -> Option<Deterrence> {
    let team = g.players[p].team;
    let ours = g.team_get_defensive_power(team).max(1) as i64;
    let mut best: Option<Deterrence> = None;
    for r in g.player_ids() {
        let pl = &g.players[r];
        let rt = pl.team;
        if r == p || !pl.alive || pl.cities.is_empty() || rt == team || g.player_is_barbarian(r) || !g.teams[team].has_met[rt] {
            continue;
        }
        // One player per team (its leader), vassals and masters, enemies already at war: not here.
        if crate::diplomacy::team_leader(g, rt) != Some(r) || g.team_is_a_vassal(rt) || g.teams[team].vassal[rt] || g.at_war(team, rt) {
            continue;
        }
        if crate::diplomacy::reach_distance(g, rt, team).map_or(true, |d| d > REACH) {
            continue;
        }
        let bar = crate::diplomacy::war_bar(g, r);
        if bar.spare_friends && crate::diplomacy::relation(g, r, p) > 1 {
            continue;
        }
        if !crate::diplomacy::can_take_cities_at(g, r, p, 120) {
            continue;
        }
        let theirs = g.team_get_power(rt, true) as i64 + crate::diplomacy::dogpile_power(g, rt, team);
        let wanted = theirs * DETER_MARGIN / bar.need_ratio.max(1);
        let shortfall = wanted - ours;
        if shortfall > 0 && best.map_or(true, |b| shortfall > b.shortfall) {
            best = Some(Deterrence { rival: r, shortfall });
        }
    }
    best
}

/// The defenders that close the shortfall of [`deterrence`]: its power in our best defender,
/// placed in the cities facing that neighbour (on its landmass, its cities within [`REACH`]),
/// nearest first, one a city per round, at most [`MAX_EXTRA`] a city.
fn deterrence_defenders(g: &Game, p: PlayerId) -> BTreeMap<CityId, i32> {
    let mut out = BTreeMap::new();
    let Some(det) = deterrence(g, p) else { return out };
    let cities = &g.players[p].cities;
    let Some(capital) = g.players[p].capital_city.filter(|&c| g.cities.contains(c)).or_else(|| cities.first().copied()) else { return out };
    let power = crate::production::best_unit(g, capital, crate::production::Role::Defense).map_or(1, |u| g.d.units[u].power_value.max(1)) as i64;
    let mut n = (det.shortfall + power - 1) / power;
    // The cities facing the neighbour, nearest first.
    let mut front: Vec<(i32, CityId)> = Vec::new();
    for &c in cities {
        let area = g.map.plots[g.city_plot(c)].area;
        let near = g.players[det.rival]
            .cities
            .iter()
            .filter(|&&o| g.map.plots[g.city_plot(o)].area == area)
            .map(|&o| g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[o].x, g.cities[o].y))
            .min();
        if let Some(d) = near.filter(|&d| d <= REACH) {
            front.push((d, c));
        }
    }
    front.sort();
    'rounds: for _ in 0..MAX_EXTRA {
        for &(_, c) in &front {
            if n <= 0 {
                break 'rounds;
            }
            *out.entry(c).or_insert(0) += 1;
            n -= 1;
        }
    }
    crate::trace!(g, p, "deterrence: {} would see an easy target (short {} power) -> {} defenders on {} front cities", g.player_get_civilization_type(det.rival).map_or("?", |c| g.d.civilizations[c].base.type_.as_str()), det.shortfall, out.values().sum::<i32>(), front.len());
    out
}

/// Extra defenders our city `c` wants for the rivals near it (0 with the switch off).
pub fn rival_defenders(g: &Game, c: CityId) -> i32 {
    let p = g.cities[c].owner;
    all_rival_defenders(g, p).get(&c).copied().unwrap_or(0)
}

/// The extra defenders of every city of `p` (cities that want none are absent). At war (or while
/// preparing one) a pool of one defender per city -- what the flat war defender every city held
/// added up to -- is shared out by each city's share of the war threat (largest remainders; at
/// most [`MAX_EXTRA`] a city), so the front holds two or three and the interior none. The
/// peacetime defenders deter the neighbour that would see us as an easy target ([`deterrence`]).
pub fn all_rival_defenders(g: &Game, p: PlayerId) -> BTreeMap<CityId, i32> {
    let mut out = BTreeMap::new();
    if !crate::switches::RIVAL_DEFENSE.on(g, p) || g.player_is_barbarian(p) {
        return out;
    }
    let rivals = rivals(g, p);
    if rivals.is_empty() {
        return out;
    }
    let cities = g.players[p].cities.clone();
    let war: Vec<i64> = cities.iter().map(|&c| if rivals.iter().any(|r| r.war) { city_threat(g, c, &rivals, true) } else { 0 }).collect();
    let total: i64 = war.iter().sum();
    let mut share = vec![0i32; cities.len()];
    if total > 0 {
        let pool = cities.len() as i64;
        let mut rest: Vec<(i64, CityId, usize)> = Vec::new();
        let mut left = pool;
        for (i, &c) in cities.iter().enumerate() {
            let exact = pool * war[i] * 1000 / total;
            share[i] = ((exact / 1000) as i32).min(MAX_EXTRA);
            left -= share[i] as i64;
            if share[i] < MAX_EXTRA {
                rest.push((exact % 1000, c, i));
            }
        }
        rest.sort_by_key(|&(r, c, _)| (std::cmp::Reverse(r), c));
        for &(r, _, i) in rest.iter() {
            if left <= 0 || r == 0 {
                break;
            }
            share[i] += 1;
            left -= 1;
        }
    }
    // Peacetime: what deters the neighbour that would see us as an easy target (deterrence).
    let deter = deterrence_defenders(g, p);
    for (i, &c) in cities.iter().enumerate() {
        let n = (share[i] + deter.get(&c).copied().unwrap_or(0)).min(MAX_EXTRA);
        if n > 0 {
            out.insert(c, n);
        }
        if war[i] > 0 {
            crate::trace!(g, p, "{}: rival war threat {} -> {} more defenders", g.cities[c].name, war[i], n);
        }
    }
    out
}

/// May defender `u` of our city on `plot` attack out of it? Our units' defense power there must
/// be at least [`SORTIE_EDGE`] percent of the attack power of the enemy units within two plots,
/// fog included ([`crate::view::known_threat`]). Always true with the switch off.
pub fn sortie_ok(g: &Game, p: PlayerId, plot: PlotIdx, u: UnitId) -> bool {
    if !crate::switches::RIVAL_DEFENSE.on(g, p) {
        return true;
    }
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    // Nothing next to the city to attack: nothing to hold back.
    if !(0..DirectionType::NUM).filter_map(|dir| g.plot_direction(x, y, DirectionType::from_usize(dir))).any(|q| g.plot_is_visible_enemy_unit_for_unit(q, u)) {
        return true;
    }
    // (As in C++, the sortie unit counts: the strength in the city as it stands. The defenders
    // the city's quota wants stay whatever happens.)
    let ours: i64 = g.map.plots[plot].units.iter().filter(|&&o| g.units[o].owner == p).map(|&o| defense_power(g, o) as i64).sum();
    let mut theirs = 0i64;
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            for &e in &g.map.plots[q].units {
                let et = g.unit_get_team(e);
                if et != team && g.at_war(team, et) && crate::view::known_threat(g, p, e) {
                    theirs += attack_power(g, e) as i64;
                }
            }
        }
    }
    let ok = ours * 100 >= theirs * SORTIE_EDGE;
    if !ok {
        crate::trace!(g, p, "sortie held at {:?}: ours {} against {}", g.map.xy(plot), ours, theirs);
        g.tel_decision(p, "military", "sortie_held", "rival_defense", 1, String::new);
    }
    ok
}
