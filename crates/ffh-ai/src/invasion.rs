//! The amphibious invasion's plan (switch `INVASION`; docs/AI.md, "Naval"): an invasion sails
//! only when it can take the city, lands together, and takes it -- or is not launched.
//!
//! - **The force** is what the rules' battle forecast says takes the city: the smallest prefix of
//!   our army, in the order it would be committed (the army on the staging landmass, strongest
//!   first; then units elsewhere standing in for attackers still to be trained), whose
//!   battles from the landing plot on their next turn ([`ForecastTurn::Next`]: the stack lands, then
//!   attacks) take the city with a chance of at least [`BAR`] against the garrison we see plus the
//!   enemy units that can join it in the turn ashore ([`enemy_support`]), and [`FORCE_MARGIN_PCT`]
//!   more for what the port cannot see (the garrison growing while the fleet sails). Before the
//!   war, the battles are forecast as fought once it is declared ([`WarTest::DeclareWar`]). Beyond
//!   the army we have, the force is scaled from the forecast's kills ([`EXTRAP_PCT`]).
//! - **The fleet** carries that force in one wave: transports with the room still missing, from
//!   the best transport the staging port can train. If the port cannot build them within
//!   [`FLEET_TURNS`], or their hammers pass [`FLEET_SHARE_PCT`] of the empire's production over that
//!   time, or the force passes [`MAX_FORCE`], the invasion is not affordable: nothing is built or
//!   gathered for it, and the war gate does not choose the target.
//!
//! Pure and deterministic (the forecast plays on a private RNG).

use ffh_core::engagement::{ForecastTurn, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::forecast::Cache;
use crate::production::{best_unit, Role};
use crate::view::attack_power;

/// The chance (per mille) the landed force takes the city on the turn after it lands, at or above
/// which the invasion is prepared and the fleet sails.
pub const BAR: i32 = 700;
/// Enemy units within this many plots of the city (on its landmass, outside it), and close enough
/// for their moves in [`SUPPORT_TURNS`], count among its defenders ([`enemy_support`]).
pub const ENEMY_REACH: i32 = 6;
/// The turns of the enemy's moves that count: the turn our force spends ashore before it attacks.
pub const SUPPORT_TURNS: i32 = 1;
/// A fleet at sea does not land, and sails home, when the force aboard (with ours already next to
/// the city) takes it with a chance under this (per mille): the enemy massed at the city while it
/// sailed. Between this and [`BAR`] it lands, and the landed stack decides on the field rule.
pub const ABORT_BAR: i32 = 350;
/// The largest force an invasion plans (two full stacks).
pub const MAX_FORCE: i32 = 16;
/// The staging port must build the missing transports within this many turns ...
pub const FLEET_TURNS: i32 = 20;
/// ... and their hammers must stay within this percent of the empire's production over those
/// turns.
pub const FLEET_SHARE_PCT: i32 = 25;
/// The force is the smallest that reaches [`BAR`] and this percent more (at least one unit more,
/// within our candidates): a named safety factor for what the forecast cannot see at the port -- the
/// garrison growing and the enemy gathering while the fleet sails (sailing on the smallest force,
/// 2 of 3 fleets met a garrison under the bar on arrival).
pub const FORCE_MARGIN_PCT: i32 = 50;
/// A force beyond the army we have: our candidates scaled by the defenders over the kills the
/// forecast gives them, with this margin (percent).
pub const EXTRAP_PCT: i32 = 125;
/// At most this many overseas cities of one player are planned for, the best by value for cost
/// first (each plan plays a few forecasts).
pub const MAX_TRIED: usize = 3;

/// Does player `p` plan its invasions this way (switch `INVASION`; never the barbarians)?
pub fn on(g: &Game, p: PlayerId) -> bool {
    !g.player_is_barbarian(p) && crate::switches::INVASION.on(g, p)
}

/// The plan of an invasion of one city (module comment).
#[derive(Clone, Debug)]
pub struct Plan {
    /// The land plot next to the city the force lands on and attacks from.
    pub landing: PlotIdx,
    /// Units the force needs.
    pub force: i32,
    /// The forecast's chance (per mille) the force takes the city (all candidates' when
    /// `estimated`).
    pub p_take: i32,
    /// The force is beyond the army we have (scaled, not forecast).
    pub estimated: bool,
    /// Our army units on the staging landmass (or aboard a transport in the staging port): the
    /// first candidates.
    pub staging_units: i32,
    /// The force's units of ours on the staging side, in commitment order.
    pub committed: Vec<UnitId>,
    /// Room in the assault transports we have or train, transports still to build, the room of
    /// one, their hammers, and the turns the port takes for them.
    pub capacity: i32,
    pub ships_needed: i32,
    pub cargo_each: i32,
    pub hammers: i32,
    pub port_turns: i32,
    /// The empire's production per turn.
    pub empire_production: i32,
    pub affordable: bool,
    /// Why not affordable (`ok` when it is).
    pub why: &'static str,
}

/// The war the forecast assumes against the city's owner: the one we are in, or the one we would
/// declare.
pub fn war_test(g: &Game, p: PlayerId, owner: PlayerId) -> WarTest {
    let (team, theirs) = (g.players[p].team, g.players[owner].team);
    if g.at_war(team, theirs) {
        WarTest::AtWar
    } else {
        WarTest::DeclareWar(theirs)
    }
}

/// The plot our force lands on next to the city on `cp`, sailing from the port on `staging_plot`:
/// a land plot beside the city (not impassable, no city) with no unit of another team we see on it,
/// beside a revealed water plot of a sea the port is on (a transport unloads from there; not a lake
/// it cannot reach) and, when a transport of ours is in the port, one the most restricted of them
/// reaches within 25 turns (a Trireme that cannot cross the ocean between); the best ground for us,
/// then the lowest index.
pub fn landing_plot(g: &Game, p: PlayerId, cp: PlotIdx, staging_plot: PlotIdx) -> Option<PlotIdx> {
    let team = g.players[p].team;
    let (tx, ty) = g.map.xy(cp);
    let around = |q: PlotIdx| -> Vec<PlotIdx> {
        let (x, y) = g.map.xy(q);
        (0..DirectionType::NUM).filter_map(|d| g.plot_direction(x, y, DirectionType::from_usize(d))).collect()
    };
    let seas: Vec<Option<AreaId>> = around(staging_plot).into_iter().filter(|&w| g.plot_is_water(w)).map(|w| g.map.plots[w].area).collect();
    let water_bars = |t: UnitId| g.d.units[g.units[t].unit_type].terrain_impassable.iter_enumerated().filter(|&(tt, &imp)| imp && g.d.terrains[tt].water).count();
    let ship = g.map.plots[staging_plot].units.iter().copied().filter(|&t| g.units[t].owner == p && crate::naval::is_transport_type(&g.d.units[g.units[t].unit_type])).max_by_key(|&t| (water_bars(t), std::cmp::Reverse(t)));
    let reach: Option<Vec<PlotIdx>> = ship.map(|t| crate::reach::Flood::default().run(g, t, 25, false).into_iter().map(|r| r.plot).collect());
    let on_our_sea = |l: PlotIdx| {
        around(l).into_iter().any(|w| g.plot_is_water(w) && g.map.plots[w].revealed[team] && seas.contains(&g.map.plots[w].area) && reach.as_ref().map_or(true, |r| r.contains(&w)))
    };
    let mut best: Option<(i32, std::cmp::Reverse<PlotIdx>)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(l) = g.plot_direction(tx, ty, DirectionType::from_usize(dir)) else { continue };
        if g.plot_is_water(l) || g.plot_is_impassable(l) || g.map.plots[l].plot_city.is_some() || !on_our_sea(l) {
            continue;
        }
        if g.map.plots[l].units.iter().any(|&o| g.unit_get_team(o) != team && !g.unit_is_invisible(o, team, false)) {
            continue;
        }
        let key = (g.plot_defense_modifier(l, Some(team), true), std::cmp::Reverse(l));
        if best.map_or(true, |b| key > b) {
            best = Some(key);
        }
    }
    best.map(|(_, std::cmp::Reverse(l))| l)
}

/// The enemy units that can join the defence of the city on `cp` in the turn our force spends
/// ashore, as `p` sees them (none for a barbarian city: the barbarians' units roam and raid, they
/// do not gather to defend it): land units of its owner's team able to defend, not animals (they do
/// not enter owned land), on its landmass, outside it (those are its garrison), not hidden from us,
/// within [`ENEMY_REACH`] plots and within their moves times [`SUPPORT_TURNS`] (in plots, roads
/// ignored).
pub fn enemy_support(g: &Game, p: PlayerId, cp: PlotIdx) -> Vec<UnitId> {
    let team = g.players[p].team;
    let Some(c) = g.map.plots[cp].plot_city else { return Vec::new() };
    // (The barbarians do not gather to defend a city: their units roam and raid.)
    if g.player_is_barbarian(g.cities[c].owner) {
        return Vec::new();
    }
    let theirs = g.players[g.cities[c].owner].team;
    let area = g.map.plots[cp].area;
    let (cx, cy) = g.map.xy(cp);
    let denom = g.d.defines.move_denominator.max(1);
    let mut out = Vec::new();
    for dx in -ENEMY_REACH..=ENEMY_REACH {
        for dy in -ENEMY_REACH..=ENEMY_REACH {
            let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
            if q == cp || g.plot_is_water(q) || g.map.plots[q].area != area {
                continue;
            }
            let (qx, qy) = g.map.xy(q);
            let d = g.step_distance(qx, qy, cx, cy);
            for &u in &g.map.plots[q].units {
                if g.unit_get_team(u) != theirs || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || g.unit_is_invisible(u, team, false) || !g.unit_can_defend(u, None) {
                    continue;
                }
                let info = &g.d.units[g.units[u].unit_type];
                if info.animal || info.combat <= 0 && info.combat_defense <= 0 {
                    continue;
                }
                if d > (g.unit_max_moves(u) / denom).max(1) * SUPPORT_TURNS {
                    continue;
                }
                out.push(u);
            }
        }
    }
    out.sort();
    out
}

/// Our army candidates for an invasion launched from `staging_plot`, in commitment order: the
/// army units aboard a transport in the staging port; then the army units of our stacks on the
/// staging landmass, strongest first (the smallest force), the nearer the port first among equals;
/// then our army units elsewhere, strongest first (stand-ins for the attackers the port will
/// train). At most [`MAX_FORCE`] + 1. Returns the list and how many of it are on the staging side.
pub fn candidates(g: &Game, p: PlayerId, staging_plot: PlotIdx) -> (Vec<UnitId>, usize) {
    let area = g.map.plots[staging_plot].area;
    let (sx, sy) = g.map.xy(staging_plot);
    let power = |u: UnitId| std::cmp::Reverse(attack_power(g, u));
    // Aboard a transport in the staging port.
    let mut aboard: Vec<UnitId> = g.map.plots[staging_plot]
        .units
        .iter()
        .copied()
        .filter(|&t| g.units[t].owner == p && crate::naval::is_transport_type(&g.d.units[g.units[t].unit_type]))
        .flat_map(|t| g.unit_get_cargo_units(t))
        .filter(|&u| crate::military::is_army_unit(g, u))
        .collect();
    aboard.sort_by_key(|&u| (power(u), u));
    let mut ashore: Vec<(std::cmp::Reverse<i32>, i32, UnitId)> = Vec::new();
    for &gr in &g.players[p].groups {
        let Some(head) = g.group_get_head_unit(gr) else { continue };
        if g.unit_is_cargo(head) || !crate::military::is_army_unit(g, head) {
            continue;
        }
        let hp = g.unit_plot(head);
        if g.map.plots[hp].area != area {
            continue;
        }
        let (hx, hy) = g.map.xy(hp);
        let d = g.plot_distance(hx, hy, sx, sy);
        for &u in &g.groups[gr].units {
            if crate::military::is_army_unit(g, u) && !aboard.contains(&u) {
                ashore.push((power(u), d, u));
            }
        }
    }
    ashore.sort();
    let mut out = aboard;
    out.extend(ashore.into_iter().map(|(_, _, u)| u));
    let cap = MAX_FORCE as usize + 1;
    out.truncate(cap);
    let staging = out.len();
    if out.len() < cap {
        let mut others: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units.contains(u) && crate::military::is_army_unit(g, u) && !out.contains(&u)).collect();
        others.sort_by_key(|&u| (power(u), u));
        others.truncate(cap - out.len());
        out.extend(others);
    }
    (out, staging)
}

/// The forecast's chance (per mille) that `units` landed on `landing` take the city on `cp` on
/// their next turn against its garrison and `extras`, under `war` ([`crate::forecast::RUNS`]
/// battles); with the whole estimate.
pub fn take(g: &Game, cache: Option<&Cache>, units: &[UnitId], landing: PlotIdx, cp: PlotIdx, extras: &[UnitId], war: WarTest) -> Option<crate::forecast::Est> {
    if units.is_empty() {
        return None;
    }
    let attacks = [(units.to_vec(), landing)];
    crate::forecast::forecast_turn_war(g, cache, &attacks, cp, extras, crate::forecast::RUNS, ForecastTurn::Next, war).map(|f| crate::forecast::Est::of(&f))
}

/// Room in our assault transports (alive, and in training at the room of `cargo_each`).
fn assault_capacity(g: &Game, p: PlayerId, cargo_each: i32) -> i32 {
    let alive: i32 = g.players[p]
        .units
        .iter()
        .filter(|&&u| g.units.contains(u) && crate::naval::is_transport_type(&g.d.units[g.units[u].unit_type]) && g.units[u].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA))
        .map(|&u| g.d.units[g.units[u].unit_type].cargo_space)
        .sum();
    alive + g.players[p].ai_num_train_ai_units[UnitAIType::UNITAI_ASSAULT_SEA] * cargo_each
}

/// The plan of an invasion of `city` from `staging` by player `p` (module comment). `None` when
/// we do not know the garrison ([`crate::view::sees_units`]: with `FOG_ATTACK`, any revealed
/// plot; the sea explorer reveals the coasts), have no landing plot or no army to forecast with,
/// or the forecast refuses the battle.
pub fn plan(g: &Game, cache: Option<&Cache>, p: PlayerId, city: CityId, staging: CityId) -> Option<Plan> {
    let team = g.players[p].team;
    let cp = g.city_plot(city);
    if !crate::view::sees_units(g, team, cp) {
        return None;
    }
    let staging_plot = g.city_plot(staging);
    let landing = landing_plot(g, p, cp, staging_plot)?;
    let (cands, staging_n) = candidates(g, p, staging_plot);
    if cands.is_empty() {
        return None;
    }
    let war = war_test(g, p, g.cities[city].owner);
    let extras = enemy_support(g, p, cp);
    let n = cands.len();
    // Before any forecast: with no room in our assault transports, a port that cannot build one
    // transport within FLEET_TURNS cannot ship any force (the plan's answer, without its cost).
    let ship = best_unit(g, staging, Role::Transport);
    let cargo_each = ship.map_or(0, |u| g.d.units[u].cargo_space.max(1));
    if assault_capacity(g, p, cargo_each) == 0 {
        let one = ship.map(|u| {
            let cost = g.city_production_needed_unit(staging, u);
            let rate = g.city_production_difference(staging, cost, 0, g.city_production_modifier_unit(staging, u), false, false);
            if rate <= 0 {
                i32::MAX
            } else {
                (cost + rate - 1) / rate
            }
        });
        if one.map_or(true, |t| t > FLEET_TURNS) {
            let why = if ship.is_some() { "port_turns" } else { "no_ship" };
            let empire_production: i32 = g.players[p].cities.iter().map(|&c| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION)).sum();
            let (force, staging_units) = (1, staging_n as i32);
            return Some(Plan { landing, force, p_take: 0, estimated: true, staging_units, committed: Vec::new(), capacity: 0, ships_needed: 1, cargo_each, hammers: 0, port_turns: one.unwrap_or(0), empire_production, affordable: false, why });
        }
    }
    let all = take(g, cache, &cands, landing, cp, &extras, war)?;
    crate::trace!(g, p, "invasion forecast of {} candidates ({} staging) from {:?} on {:?} ({:?}, {} support): {:?}", n, staging_n, g.map.xy(landing), g.map.xy(cp), war, extras.len(), all);
    let (force, p_take, estimated) = if all.p_take >= BAR {
        // The smallest prefix that takes it (the forecast is close to monotone in the force).
        let (mut lo, mut hi, mut p_hi) = (1usize, n, all.p_take);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let pm = take(g, cache, &cands[..mid], landing, cp, &extras, war).map_or(0, |e| e.p_take);
            if pm >= BAR {
                hi = mid;
                p_hi = pm;
            } else {
                lo = mid + 1;
            }
        }
        // The margin, as far as our candidates go (the forecast of the larger force is not needed:
        // it only adds units to a force that already reaches the bar).
        let with_margin = (hi as i32 * (100 + FORCE_MARGIN_PCT) + 99) / 100;
        let with_margin = with_margin.max(hi as i32 + 1).min(n as i32);
        (with_margin.max(hi as i32), p_hi, false)
    } else {
        // Beyond what we have: scale our candidates by the defenders over what they kill.
        let defs = (all.n_def.max(1)) as i64;
        let kills = (all.enemy_lost_x100 as i64).max(25);
        let scaled = (n as i64 * defs * 100 * EXTRAP_PCT as i64 + kills * 100 - 1) / (kills * 100);
        (scaled.clamp(n as i64 + 1, 1000) as i32, all.p_take, true)
    };
    let committed: Vec<UnitId> = cands[..staging_n.min(force as usize)].to_vec();
    // The fleet: the room still missing, in the best transport the port trains now.
    let capacity = assault_capacity(g, p, cargo_each);
    let missing = (force - capacity).max(0);
    let empire_production: i32 = g.players[p].cities.iter().map(|&c| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION)).sum();
    let mut plan = Plan {
        landing,
        force,
        p_take,
        estimated,
        staging_units: staging_n as i32,
        committed,
        capacity,
        ships_needed: 0,
        cargo_each,
        hammers: 0,
        port_turns: 0,
        empire_production,
        affordable: false,
        why: "ok",
    };
    if force > MAX_FORCE {
        plan.why = "force";
        return Some(plan);
    }
    if missing > 0 {
        let Some(u) = ship else {
            plan.why = "no_ship";
            return Some(plan);
        };
        plan.ships_needed = (missing + cargo_each - 1) / cargo_each;
        let cost = g.city_production_needed_unit(staging, u);
        plan.hammers = plan.ships_needed * cost;
        let rate = g.city_production_difference(staging, cost, 0, g.city_production_modifier_unit(staging, u), false, false);
        if rate <= 0 {
            plan.why = "port_turns";
            plan.port_turns = i32::MAX;
            return Some(plan);
        }
        plan.port_turns = (plan.hammers + rate - 1) / rate;
        if plan.port_turns > FLEET_TURNS {
            plan.why = "port_turns";
            return Some(plan);
        }
        if plan.hammers as i64 * 100 > empire_production as i64 * FLEET_TURNS as i64 * FLEET_SHARE_PCT as i64 {
            plan.why = "hammers";
            return Some(plan);
        }
    }
    plan.affordable = true;
    Some(plan)
}

/// Is a city of ours on the landmass of `plot` in peril this turn (`View::peril`)? The force is
/// needed at home.
pub fn home_in_peril(g: &Game, v: &crate::view::View, plot: PlotIdx) -> bool {
    let area = g.map.plots[plot].area;
    v.peril.iter().any(|&(q, _)| g.map.plots[q].area == area)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Player 0's port on (2, 5) of the west landmass with `n` Axemen in it; player 1's coastal
    /// city on (14, 3) of the east landmass with one Warrior; at war.
    fn invasion_game(n: usize) -> (Game, CityId, CityId) {
        let mut g = crate::testkit::two_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let port = g.player_init_city(p0, 2, 5, true, true);
        let theirs = g.player_init_city(p1, 14, 3, true, true);
        assert!(crate::naval::is_coastal_city(&g, port));
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        g.player_init_unit(p1, w, 14, 3, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        let axe = g.d.t::<UnitType>("UNIT_AXEMAN");
        for _ in 0..n {
            g.player_init_unit(p0, axe, 2, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        }
        (g, port, theirs)
    }

    /// The force is a prefix of our army that the forecast says takes the city from a landing
    /// plot next to it; with room for it already in port, the invasion costs nothing more and is
    /// affordable.
    #[test]
    fn the_force_is_what_the_forecast_says_takes_the_city() {
        let (mut g, port, theirs) = invasion_game(4);
        let p0 = PlayerId(0);
        let galleon = g.d.t::<UnitType>("UNIT_GALLEON");
        assert!(g.d.units[galleon].cargo_space >= 2);
        for _ in 0..2 {
            g.player_init_unit(p0, galleon, 1, 5, Some(UnitAIType::UNITAI_ASSAULT_SEA), None);
        }
        let plan = plan(&g, None, p0, theirs, port).expect("a plan");
        let (lx, ly) = g.map.xy(plan.landing);
        assert_eq!(g.step_distance(lx, ly, 14, 3), 1, "the landing plot is next to the city");
        assert!(!g.plot_is_water(plan.landing));
        assert!(!plan.estimated && (1..=4).contains(&plan.force), "{plan:?}");
        assert!(plan.p_take >= BAR, "{plan:?}");
        assert_eq!(plan.committed.len(), plan.force as usize);
        assert_eq!((plan.ships_needed, plan.affordable, plan.why), (0, true, "ok"), "{plan:?}");
        // The smallest prefix that reaches the bar, with the margin.
        let (cands, _) = candidates(&g, p0, g.city_plot(port));
        let cp = g.city_plot(theirs);
        let extras = enemy_support(&g, p0, cp);
        let war = war_test(&g, p0, PlayerId(1));
        let k = (1..=cands.len()).find(|&k| take(&g, None, &cands[..k], plan.landing, cp, &extras, war).map_or(0, |e| e.p_take) >= BAR).expect("a prefix takes it") as i32;
        let margin = ((k * (100 + FORCE_MARGIN_PCT) + 99) / 100).max(k + 1).min(cands.len() as i32);
        assert_eq!(plan.force, margin, "{plan:?}");
    }

    /// No room in port and no transport the port can train: the invasion is not affordable, and
    /// the city is no amphibious target (nothing is built or gathered for it).
    #[test]
    fn an_invasion_without_ships_is_not_prepared() {
        let (g, port, theirs) = invasion_game(4);
        let p0 = PlayerId(0);
        assert!(best_unit(&g, port, Role::Transport).is_none(), "no transport tech");
        let plan = plan(&g, None, p0, theirs, port).expect("a plan");
        assert_eq!((plan.affordable, plan.why), (false, "no_ship"), "{plan:?}");
        assert!(crate::naval::amphibious_city(&g, p0, PlayerId(1)).is_none());
    }
}
