//! Warships (switch `WARSHIPS`): what an armed ship does with its turn when it carries nothing,
//! and how many a civilization trains, and why. Under the sea rules (docs/NAVAL.md): ships operate
//! within range of a port, warships wear out away from port, and a sea is held by Patrol (the
//! plunder mission): a sea is closed to a team when an enemy patrols it with at least twice the
//! team's patrol strength there, and a city is blockaded when every sea it touches is closed to
//! its owner. Civ4's blockade of the plots in range of a ship is gone, so every station here is the
//! Patrol of a named sea.
//!
//! Before any rule, a ship out of its team's range, or a warship carrying nothing whose wear is
//! above half its maximum, makes for port ([`to_port`], called by `units::play` for every ship of
//! ours). Then a warship's turn, the first rule that applies:
//!
//! 1. **Heal**: damaged by [`HEAL_PCT`] or more outside a city of ours, it sails to the nearest
//!    port it reaches; in port it stays while damaged above [`HEALED_PCT`].
//! 2. **Kill**: an enemy ship (a team at war with ours; the barbarians always) in our range that
//!    it can attack this turn at the rules' odds (`get_combat_odds`) of [`ODDS_BAR`], or
//!    [`ODDS_BAR_PRIZE`] against a prize: a transport seen carrying units, a ship on Patrol or
//!    standing on a sea resource of ours. Loaded transports first, then the other prizes, then the
//!    best odds. Every attack is in the prediction ledger (`sea_attack`).
//! 3. **Escort**: while an assault fleet loads in its staging port (`naval::fleet_waits` takes our
//!    idle warships there along when it sails), a share of the warships go there and wait:
//!    one per [`ESCORT_PER_TRANSPORTS`] loaded transports, at most [`ESCORT_SHARE_PCT`] of them.
//! 4. **Attack together**: an enemy ship within [`COAST_RADIUS`] of our waters that no ship of
//!    ours beats alone: the warships that reach it this turn attack it together, strongest first,
//!    when the duels in that order kill it at [`ODDS_BAR`] (the first at [`GANG_FIRST_BAR`] or
//!    more; each loser's expected damage carried to the next duel: `combat_sim::fight`).
//! 5. **Siege blockade**: the Patrol of a sea of an enemy coastal city one of our armies besieges
//!    ([`blockade`] with the siege only; docs/NAVAL.md, section 8: closing every sea of a besieged
//!    port stops its healing and repair).
//! 6. **Guard**: the Patrol of a sea one of our ports stands on where an enemy patrols or has
//!    ships we see, then a sea our outposts' lanes pass through ([`guard`]), when this ship with those already there or bound there is not
//!    outmatched two to one; a sea already held by us needs no more.
//! 7. **Gather**: with an enemy ship near our waters and no sea to guard, the warships within
//!    [`GATHER_TURNS`] wait in the port nearest it (ships in a city cannot be attacked by ships).
//! 8. **Blockade**: at war with a civilization, the Patrol of a sea of its coast within
//!    [`BLOCKADE_TURNS`] that denies the most ([`sea_value`]: its water plots there, worked and with
//!    resources, its ports on the sea, those the sea's closing would blockade), per turn of travel,
//!    twice for a sea this ship's Patrol would close, and not in a sea where we would be
//!    outmatched, already closed by our other ships, or at a plot an enemy ship that beats it at
//!    [`DANGER_ODDS`] can reach. An enemy sea improvement it stands on is pillaged first.
//! 9. **Guard post**: one warship for each coastal city of ours within [`POST_RIVAL_DIST`] of a
//!    rival civilization's coastal city or [`POST_BARB_DIST`] of a barbarian ship we see (the
//!    barbarians' first), the rest for the port with the most worked sea plots and sea resources;
//!    each patrols a sea of its port (in a port with no sea, it fortifies there).
//!
//! A ship on Patrol keeps its station without new orders ([`keeps_station`]) until it is worn, out
//! of range, damaged by [`HEAL_PCT`], outmatched two to one in its sea, an enemy ship comes within
//! its moves, `naval.rs` wants it as a transport, or every [`RELOOK_TURNS`] turns.
//!
//! An idle transport (`naval.rs` has no cargo, settler, army or assault for it: [`free_transport`])
//! plays every rule but the escort; otherwise `naval::transport` has it. A sea explorer with
//! nothing left to reveal plays as a warship.
//!
//! Barbarian ships and sea animals ([`barbarian`]): they attack at [`BARB_ODDS`], hunt a ship
//! they beat within [`BARB_HUNT_TURNS`] (as land animals hunt), pillage a sea improvement they stand
//! on, otherwise move to a plot they reach this turn chosen by a hash of unit and turn, as land
//! animals wander. They do not patrol (the rules let no barbarian ship patrol), and range and wear
//! do not apply to them. (The rules keep animals out of owned plots.)
//!
//! Production ([`production`], coastal cities): warships for a reason, in numbers relative to it:
//! escorts for an assault fleet; guards, the more of the enemy ships seen near our coast and two
//! for each sea of our ports an enemy holds or contests; blockaders at war with a civilization
//! whose ships we outclass, one for each sea of its coast that reaches into our range (at most
//! [`BLOCKADERS_MAX`]); all within [`HAMMER_SHARE_PCT`] of the military's hammers; one more for a
//! civilization whose capital is coastal and whose data has a ship of its own. No reason, no
//! warship.

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use ffh_core::army::{EscortTarget, Station};

use crate::production::Choice;
use crate::units::{alive, can_start, hold, move_to, push, skip};
use ffh_core::seas::SeaControl;
use crate::view::View;
use crate::Ai;

/// Damage (percent of the maximum hit points) at which a ship outside a port sails home to heal.
pub const HEAL_PCT: i32 = 50;
/// A ship in port stays there while its damage is above this (percent).
pub const HEALED_PCT: i32 = 10;
/// The rules' odds (per mille) a warship attacks at.
pub const ODDS_BAR: i32 = 650;
/// ... against a prize: a transport seen carrying units, a ship blockading us or on our sea resource.
pub const ODDS_BAR_PRIZE: i32 = 500;
/// The first attacker of a joint attack needs at least these odds.
pub const GANG_FIRST_BAR: i32 = 350;
/// At most this many warships attack one ship together.
pub const GANG_MAX: usize = 3;
/// An enemy ship within this many plots of a water plot of ours threatens our coast.
pub const COAST_RADIUS: i32 = 2;
/// Warships within this many turns of a threat to our coast gather in the port nearest it.
pub const GATHER_TURNS: i32 = 3;
/// One escort per this many loaded transports of an assault fleet.
pub const ESCORT_PER_TRANSPORTS: i32 = 2;
/// At most this share of the warships escort.
pub const ESCORT_SHARE_PCT: i32 = 50;
/// A sea to blockade at most this many turns away.
pub const BLOCKADE_TURNS: i32 = 8;
/// The least a sea to blockade must deny ([`sea_value`]).
pub const BLOCKADE_MIN: i32 = 4;
/// An enemy ship that would beat ours at these odds (per mille) and can reach it next turn makes a
/// plot unsafe.
pub const DANGER_ODDS: i32 = 500;
/// A coastal city of ours within this many plots of a rival civilization's coastal city is a post.
pub const POST_RIVAL_DIST: i32 = 10;
/// ... or within this many plots of a barbarian ship we see.
pub const POST_BARB_DIST: i32 = 6;
/// Barbarian ships and sea animals attack at these odds (land animals: 50%, `units::barbarian`).
pub const BARB_ODDS: i32 = 500;
/// ... and hunt ships they beat within this many turns (land animals: 4).
pub const BARB_HUNT_TURNS: i32 = 2;
/// Production: at most this many blockaders.
pub const BLOCKADERS_MAX: i32 = 3;
/// Production: warships' hammers at most this share of the military's.
pub const HAMMER_SHARE_PCT: i64 = 20;
/// Production: an enemy ship within this many plots of a coastal city of ours is "seen in our waters".
pub const SEEN_RADIUS: i32 = 3;
/// A ship on Patrol looks again at least every this many turns.
pub const RELOOK_TURNS: i32 = 8;

// ----------------------------------------------------------------------------
// Facts
// ----------------------------------------------------------------------------

fn damage_pct(g: &Game, u: UnitId) -> i32 {
    g.units[u].damage * 100 / g.unit_max_hit_points(u).max(1)
}

fn own_city(g: &Game, p: PlayerId, q: PlotIdx) -> bool {
    g.map.plots[q].plot_city.is_some_and(|c| g.cities[c].owner == p)
}

/// A unit that can attack (strength, not defensive only, the rules let it).
fn armed(g: &Game, u: UnitId) -> bool {
    crate::view::attack_power(g, u) > 0 && g.unit_can_attack(u)
}

/// Is `u` a warship by its type: a sea unit with strength and no room for land units?
pub fn is_warship_type(info: &UnitInfo) -> bool {
    info.domain_type == Some(DomainType::DOMAIN_SEA) && info.combat > 0 && !crate::naval::is_transport_type(info)
}

/// A ship of ours this module would play: armed, at sea or in port, carrying nothing.
fn our_ship(g: &Game, u: UnitId) -> bool {
    g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && !g.unit_is_cargo(u) && armed(g, u) && g.unit_get_cargo_units(u).is_empty()
}

/// An enemy sea unit of team `team` sees on `q`: at war with it, not hidden from it.
fn enemy_ship(g: &Game, team: TeamId, e: UnitId) -> bool {
    let et = g.unit_get_team(e);
    et != team && g.at_war(team, et) && g.unit_get_domain_type(e) == DomainType::DOMAIN_SEA && !g.unit_is_cargo(e) && !g.unit_is_invisible(e, team, false)
}

/// Visible enemy ships on water plot `q`.
fn enemy_ships_on(g: &Game, team: TeamId, q: PlotIdx) -> bool {
    g.plot_is_water(q) && g.plot_is_visible(q, team) && g.map.plots[q].units.iter().any(|&e| enemy_ship(g, team, e))
}

/// What sinking the ships on `q` is worth beyond a ship: 2 a transport seen carrying units, 1 a
/// ship on Patrol (its strength counts in who holds the sea) or standing on a sea resource of our
/// team, else 0.
fn prize(g: &Game, team: TeamId, q: PlotIdx) -> i32 {
    let mut best = 0;
    for &e in &g.map.plots[q].units {
        if !enemy_ship(g, team, e) {
            continue;
        }
        if !g.unit_get_cargo_units(e).is_empty() {
            return 2;
        }
        if g.units[e].blockading {
            best = 1;
        }
    }
    if best == 0 && g.plot_get_team(q) == Some(team) && g.plot_get_bonus_type(q, Some(team)).is_some() {
        best = 1;
    }
    best
}

/// The odds bar against the ships on `q`.
fn bar(prize: i32) -> i32 {
    if prize > 0 { ODDS_BAR_PRIZE } else { ODDS_BAR }
}

/// Water plots with visible enemy ships that `u` can attack this turn (next to a plot it reaches
/// with moves left), with the defender the rules put up against it and its odds.
fn targets(g: &Game, v: &mut View, u: UnitId) -> Vec<(PlotIdx, UnitId, i32)> {
    let p = g.units[u].owner;
    if g.unit_moves_left(u) <= 0 || !armed(g, u) {
        return Vec::new();
    }
    let team = v.team;
    let reach = v.flood.run(g, u, 0, false);
    let mut out: Vec<(PlotIdx, UnitId, i32)> = Vec::new();
    for r in &reach {
        if r.turns != 0 {
            continue;
        }
        let (x, y) = g.map.xy(r.plot);
        for dir in 0..DirectionType::NUM {
            let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
            // Not out of our range: the winner may advance onto it.
            if out.iter().any(|t| t.0 == q) || !enemy_ships_on(g, team, q) || !g.plot_in_sea_range(team, q) || !g.unit_can_move_into(u, q, true, false, false) {
                continue;
            }
            let Some(def) = g.plot_get_best_defender(q, None, Some(p), Some(u), false, false, false) else { continue };
            out.push((q, def, g.get_combat_odds(u, def)));
        }
    }
    out
}

/// The water plot next to `q` that `u` reaches this turn with moves left (or its own plot).
fn approach(g: &Game, v: &mut View, u: UnitId, q: PlotIdx) -> Option<PlotIdx> {
    let here = g.unit_plot(u);
    let (qx, qy) = g.map.xy(q);
    let (hx, hy) = g.map.xy(here);
    if g.plot_distance(hx, hy, qx, qy) == 1 {
        return Some(here);
    }
    let reach = v.flood.run(g, u, 0, false);
    reach
        .iter()
        .filter(|r| r.turns == 0 && {
            let (x, y) = g.map.xy(r.plot);
            g.plot_distance(x, y, qx, qy) == 1
        })
        .min_by_key(|r| (r.turns, r.plot))
        .map(|r| r.plot)
}

/// Attack the ships on `q` with group `grp` (led by `u`): to the plot beside it, then the attack.
fn attack(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, q: PlotIdx, odds: i32, src: &'static str) -> bool {
    let p = v.p;
    let Some(from) = approach(g, v, u, q) else { return false };
    let pz = prize(g, v.team, q);
    if from != g.unit_plot(u) {
        move_to(g, grp, from, 0);
        if !alive(g, grp) || !g.units.contains(u) || g.unit_plot(u) != from || g.unit_moves_left(u) <= 0 {
            return true;
        }
    }
    crate::trace!(g, p, "warship {:?} at {:?} attacks {:?} at odds {} ({})", u, g.map.xy(g.unit_plot(u)), g.map.xy(q), odds, src);
    crate::predict::sea_attack(g, p, u, q, odds, pz, src);
    g.tel_decision(p, "warship", src, "attack", odds as i64, String::new);
    move_to(g, grp, q, MOVE_DIRECT_ATTACK);
    true
}

/// Can an enemy ship that would beat `u` at [`DANGER_ODDS`] reach plot `q` next turn?
fn endangered(g: &Game, team: TeamId, u: UnitId, q: PlotIdx) -> bool {
    let (x, y) = g.map.xy(q);
    let r = 5;
    for dx in -r..=r {
        for dy in -r..=r {
            let Some(w) = g.plot_xy(x, y, dx, dy) else { continue };
            if !g.plot_is_water(w) || !g.plot_is_visible(w, team) {
                continue;
            }
            for &e in &g.map.plots[w].units {
                if !enemy_ship(g, team, e) || !armed(g, e) {
                    continue;
                }
                let reach = g.d.units[g.units[e].unit_type].moves.max(1) + 1;
                if dx.abs().max(dy.abs()) > reach {
                    continue;
                }
                if g.unit_can_attack_unit(e, u) && g.get_combat_odds(e, u) >= DANGER_ODDS {
                    return true;
                }
            }
        }
    }
    false
}

/// Is plot `q` within `r` plots of a water plot of team `team`?
fn near_our_waters(g: &Game, team: TeamId, q: PlotIdx, r: i32) -> bool {
    let (x, y) = g.map.xy(q);
    for dx in -r..=r {
        for dy in -r..=r {
            if let Some(w) = g.plot_xy(x, y, dx, dy) {
                if g.plot_is_water(w) && g.plot_get_team(w) == Some(team) {
                    return true;
                }
            }
        }
    }
    false
}

/// Our coastal city nearest plot `q` (by plot distance; ties by id).
fn port_nearest(g: &Game, p: PlayerId, q: PlotIdx) -> Option<CityId> {
    let (x, y) = g.map.xy(q);
    g.players[p].cities.iter().copied().filter(|&c| crate::naval::is_coastal_city(g, c)).min_by_key(|&c| (g.plot_distance(x, y, g.cities[c].x, g.cities[c].y), c))
}

/// Sail to (or stay in) `q`, then end the turn (in a port: fortify or sleep; `awake`: skip, so the
/// ship stays ready for `naval::fleet_waits` and the next turn's decision).
fn go(g: &mut Game, grp: GroupId, q: PlotIdx, awake: bool, mission_ai: Option<MissionAIType>) {
    if !alive(g, grp) {
        return;
    }
    let Some(head) = g.group_get_head_unit(grp) else { return };
    if g.unit_plot(head) != q {
        let (x, y) = g.map.xy(q);
        g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, x, y, 0, false, false, mission_ai, Some(q), None);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        if awake || g.unit_plot(head) != q { skip(g, grp) } else { hold(g, grp) }
    }
}

// ----------------------------------------------------------------------------
// Who plays here
// ----------------------------------------------------------------------------

/// An idle transport: armed, alone, carrying nothing, and none of `naval.rs`'s jobs open (an
/// assault, a settler waiting for a ship, an army waiting for one, a pickup under way, an army of
/// ours beside it). `naval::transport` would hold it in a port.
pub(crate) fn free_transport(g: &Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if !crate::switches::WARSHIPS.on(g, p) || g.groups[grp].units.len() != 1 || !our_ship(g, u) {
        return false;
    }
    if v.amphibious.is_some() || g.groups[grp].ai_mission_ai_type == Some(MissionAIType::MISSIONAI_PICKUP) {
        return false;
    }
    if crate::naval::settler_waiting(g, p).is_some() || crate::naval::settler_waits_for_ship(g, p) || !crate::naval::armies_waiting(g, p).is_empty() {
        return false;
    }
    let here = g.unit_plot(u);
    let (x, y) = g.map.xy(here);
    !(0..DirectionType::NUM).filter_map(|d| g.plot_direction(x, y, DirectionType::from_usize(d))).any(|q| !g.plot_is_water(q) && g.map.plots[q].units.iter().any(|&o| g.units[o].owner == p && crate::military::is_army_unit(g, o)))
}

/// A sea explorer with nothing left to reveal that is no transport: plays as a warship.
pub(crate) fn explorer_done(g: &Game, v: &mut View, u: UnitId) -> bool {
    crate::switches::WARSHIPS.on(g, v.p) && !crate::naval::is_transport_type(&g.d.units[g.units[u].unit_type]) && our_ship(g, u) && crate::naval::sea_explore_target(g, v, u).is_none()
}

/// A ship of ours that could join a joint attack this turn: free to act and fit.
fn free_warship(g: &Game, v: &View, u: UnitId) -> bool {
    let Some(grp) = g.units[u].group else { return false };
    if !our_ship(g, u) || g.groups[grp].units.len() != 1 || !g.group_ready_to_move(grp, false) || damage_pct(g, u) >= HEAL_PCT || g.unit_moves_left(u) <= 0 {
        return false;
    }
    let info = &g.d.units[g.units[u].unit_type];
    if crate::naval::is_transport_type(info) {
        return free_transport(g, v, grp, u);
    }
    true
}

// ----------------------------------------------------------------------------
// A warship's turn
// ----------------------------------------------------------------------------

/// One decision for warship group `grp` led by `u` (a civilization's). `transport`: an idle
/// transport (every rule but the escort). True if it took an order; false leaves the ship to the
/// caller (`naval::transport` for a transport; else it holds).
pub fn play(ai: &Ai, g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, transport: bool) -> bool {
    let _ = ai;
    let p = v.p;
    let here = g.unit_plot(u);
    // 1. Heal.
    let dmg = damage_pct(g, u);
    if own_city(g, p, here) && dmg > HEALED_PCT {
        g.tel_decision(p, "warship", "heal_stay", "heal", dmg as i64, String::new);
        hold(g, grp);
        return true;
    }
    if dmg >= HEAL_PCT && !own_city(g, p, here) {
        let reach = v.flood.run(g, u, 20, false);
        if let Some(r) = reach.iter().filter(|r| own_city(g, p, r.plot)).min_by_key(|r| (r.turns, r.plot)).copied() {
            crate::trace!(g, p, "warship {:?} at {:?} ({}% damage) sails to port {:?}", u, g.map.xy(here), dmg, g.map.xy(r.plot));
            g.tel_decision(p, "warship", "heal_sail", "heal", dmg as i64, String::new);
            go(g, grp, r.plot, false, None);
            return true;
        }
    }
    // 2. Kill what it can.
    let ts = targets(g, v, u);
    let best = ts.iter().filter(|&&(q, _, odds)| odds >= bar(prize(g, v.team, q))).max_by_key(|&&(q, _, odds)| (prize(g, v.team, q), odds, std::cmp::Reverse(q))).copied();
    if let Some((q, _, odds)) = best {
        return attack(g, v, grp, u, q, odds, "kill");
    }
    // 3. Escort an assault fleet already at sea (the Escort standing order: docs/NAVAL.md, 7.2),
    // else the one loading in its staging port.
    if !transport && escort_at_sea(g, v, grp, u) {
        return true;
    }
    if !transport {
        if let Some(port) = escort_post(g, v, u) {
            crate::trace!(g, p, "warship {:?} escorts the assault fleet at {:?}", u, g.map.xy(port));
            g.tel_decision(p, "warship", "escort", "escort", 1, String::new);
            go(g, grp, port, true, Some(MissionAIType::MISSIONAI_GROUP));
            return true;
        }
    }
    // 4. Attack a threat to our coast together.
    let threats = coast_threats(g, v, u);
    if !threats.is_empty() && gang_attack(g, v, grp, u, &threats) {
        return true;
    }
    if !alive(g, grp) || !g.units.contains(u) || !g.group_ready_to_move(grp, false) {
        return true;
    }
    // The seas this ship reaches within BLOCKADE_TURNS: (sea, the plot of it to go to, turns).
    let seas = reachable_seas(g, v, u);
    // 5. The sea of an enemy port our army besieges.
    if !v.civ_enemies.is_empty() && blockade(g, v, grp, u, &seas, true) {
        return true;
    }
    // 6. A sea of our ports an enemy patrols or sails in.
    if guard(g, v, grp, u, &seas) {
        return true;
    }
    // 7. A threat near our coast and no sea to guard: gather in the port nearest it.
    if !threats.is_empty() && gather(g, v, grp, u, &threats) {
        return true;
    }
    // 8. Blockade.
    if !v.civ_enemies.is_empty() && blockade(g, v, grp, u, &seas, false) {
        return true;
    }
    // 9. Guard post.
    post(g, v, grp, u, &seas);
    true
}

/// The staging port of an assault fleet that wants this warship as an escort: while an invasion is
/// on, one escort per [`ESCORT_PER_TRANSPORTS`] of our assault transports (alive), this warship
/// among the nearest [`ESCORT_SHARE_PCT`] of our warships to the port. It goes before the fleet is
/// loaded: the fleet sails the turn it is ready, taking along the idle warships standing there.
fn escort_post(g: &Game, v: &View, u: UnitId) -> Option<PlotIdx> {
    let p = v.p;
    let amph = v.amphibious.as_ref()?;
    let port = amph.staging_plot;
    let loaded = g.players[p]
        .units
        .iter()
        .filter(|&&t| crate::naval::is_transport_type(&g.d.units[g.units[t].unit_type]) && g.units[t].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA))
        .count() as i32;
    if loaded == 0 {
        return None;
    }
    let want = (loaded + ESCORT_PER_TRANSPORTS - 1) / ESCORT_PER_TRANSPORTS;
    let (px, py) = g.map.xy(port);
    // Our warships (not transports), nearest the port first.
    let mut ships: Vec<(i32, UnitId)> = g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&o| our_ship(g, o) && !crate::naval::is_transport_type(&g.d.units[g.units[o].unit_type]) && damage_pct(g, o) < HEAL_PCT)
        .map(|o| {
            let (x, y) = g.map.xy(g.unit_plot(o));
            (g.plot_distance(x, y, px, py), o)
        })
        .collect();
    ships.sort();
    let share = ((ships.len() as i32) * ESCORT_SHARE_PCT / 100).max(1);
    let take = want.min(share) as usize;
    ships.iter().take(take).any(|&(_, o)| o == u).then_some(port)
}

/// The visible enemy ships near our waters (within [`COAST_RADIUS`] of a water plot of ours) within
/// this ship's gathering range (its moves x [`GATHER_TURNS`] + [`COAST_RADIUS`]): (plot distance,
/// plot), nearest first.
fn coast_threats(g: &Game, v: &View, u: UnitId) -> Vec<(i32, PlotIdx)> {
    let team = v.team;
    let (hx, hy) = g.map.xy(g.unit_plot(u));
    let moves = g.d.units[g.units[u].unit_type].moves.max(1);
    let r = moves * GATHER_TURNS + COAST_RADIUS;
    let mut threats: Vec<(i32, PlotIdx)> = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            let Some(q) = g.plot_xy(hx, hy, dx, dy) else { continue };
            if enemy_ships_on(g, team, q) && near_our_waters(g, team, q, COAST_RADIUS) {
                threats.push((dx.abs().max(dy.abs()), q));
            }
        }
    }
    threats.sort();
    threats
}

/// Rule 4. A threat near our coast (`threats`) that no ship of ours beats alone and this ship can
/// attack now: attack it together when the duels in order kill it.
fn gang_attack(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, threats: &[(i32, PlotIdx)]) -> bool {
    let p = v.p;
    let team = v.team;
    let mine = targets(g, v, u);
    for &(_, q) in threats {
        if !mine.iter().any(|t| t.0 == q) {
            continue;
        }
        if let Some(gang) = gang_for(g, v, q) {
            if gang.contains(&u) {
                crate::trace!(g, p, "warships {:?} attack {:?} together", gang, g.map.xy(q));
                g.tel_decision(p, "warship", "gang", "attack", gang.len() as i64, String::new);
                for o in gang {
                    if !g.units.contains(o) || !enemy_ships_on(g, team, q) {
                        break;
                    }
                    let Some(og) = g.units[o].group else { continue };
                    let Some(def) = g.plot_get_best_defender(q, None, Some(p), Some(o), false, false, false) else { break };
                    let odds = g.get_combat_odds(o, def);
                    attack(g, v, og, o, q, odds, "gang");
                }
                if alive(g, grp) && g.group_ready_to_move(grp, false) {
                    skip(g, grp);
                }
                return true;
            }
        }
    }
    false
}

/// Rule 7. Gather in the port nearest the closest threat, when it is within [`GATHER_TURNS`].
fn gather(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, threats: &[(i32, PlotIdx)]) -> bool {
    let p = v.p;
    let Some(&(_, q)) = threats.first() else { return false };
    let Some(c) = port_nearest(g, p, q) else { return false };
    let cp = g.city_plot(c);
    let reach = v.flood.run_nearest(g, u, GATHER_TURNS, false, |rr| rr.plot == cp);
    if !reach.iter().any(|rr| rr.plot == cp) {
        return false;
    }
    crate::trace!(g, p, "warship {:?} gathers at {:?} against the ship at {:?}", u, g.map.xy(cp), g.map.xy(q));
    g.tel_decision(p, "warship", "gather", "guard", 1, String::new);
    go(g, grp, cp, true, Some(MissionAIType::MISSIONAI_GUARD_CITY));
    true
}

/// The warships of ours that attack the ships on `q` together this turn, in order (strongest
/// first): the free warships that can attack it now, while the chance the top defender dies stays
/// under [`ODDS_BAR`]; `None` when the first is under [`GANG_FIRST_BAR`] or all of them do not reach
/// the bar. (One warship that reaches the bar alone is rule 2's.)
fn gang_for(g: &Game, v: &mut View, q: PlotIdx) -> Option<Vec<UnitId>> {
    let p = v.p;
    let (qx, qy) = g.map.xy(q);
    let mut cands: Vec<(i32, UnitId, UnitId)> = Vec::new();
    let ids: Vec<UnitId> = g.players[p].units.clone();
    for o in ids {
        if !free_warship(g, v, o) {
            continue;
        }
        let (x, y) = g.map.xy(g.unit_plot(o));
        if g.plot_distance(x, y, qx, qy) > g.d.units[g.units[o].unit_type].moves.max(1) + 1 {
            continue;
        }
        if let Some(&(_, def, odds)) = targets(g, v, o).iter().find(|t| t.0 == q) {
            cands.push((odds, o, def));
        }
    }
    cands.sort_by_key(|&(odds, o, _)| (std::cmp::Reverse(odds), o));
    let &(first, _, def) = cands.first()?;
    if first < GANG_FIRST_BAR || cands.len() < 2 {
        return None;
    }
    let bar = bar(prize(g, v.team, q));
    let mut hp = g.unit_curr_hit_points(def);
    let mut alive_pm: i64 = 1000; // the chance (per mille) the defender still stands
    let mut gang = Vec::new();
    for &(_, o, _) in cands.iter().take(GANG_MAX) {
        let (odds, dealt) = crate::combat_sim::fight(g, o, def, q, hp.max(1), 0);
        alive_pm = alive_pm * (1000 - odds as i64) / 1000;
        hp -= dealt;
        gang.push(o);
        if 1000 - alive_pm >= bar as i64 {
            return Some(gang);
        }
    }
    None
}

// ----------------------------------------------------------------------------
// Seas and Patrol (docs/NAVAL.md, sections 4 and 8)
// ----------------------------------------------------------------------------

/// Whether a warship's wear is above half its maximum (it makes for port, [`to_port`]).
fn worn(g: &Game, u: UnitId) -> bool {
    g.unit_sea_wear_percent(u) > g.d.defines.sea_wear_max_percent / 2
}

/// Whether `team` would be outmatched two to one in sea `s` (by what it sees) with `extra` of its
/// ships on Patrol there as well.
fn outmatched_if(g: &Game, team: TeamId, s: SeaId, extra: &[UnitId]) -> bool {
    let st = g.sea_state_known_if(s, team, team, extra);
    st.theirs > 0 && st.theirs >= 2 * st.ours.max(1) && !matches!(st.control, SeaControl::PatrolledBy(_))
}

/// The warships of player `p`'s groups other than `except` that are bound for a Patrol in sea `s`
/// (the mission AI `MISSIONAI_BLOCKADE` with its plot in `s`) and not on Patrol yet.
fn bound_for(g: &Game, p: PlayerId, s: SeaId, except: GroupId) -> Vec<UnitId> {
    let mut out = Vec::new();
    for &o in &g.players[p].groups {
        if o == except || !g.groups.contains(o) || g.groups[o].ai_mission_ai_type != Some(MissionAIType::MISSIONAI_BLOCKADE) {
            continue;
        }
        if g.group_ai_get_mission_ai_plot(o).and_then(|m| g.map.plots[m].sea) != Some(s) {
            continue;
        }
        out.extend(g.groups[o].units.iter().copied().filter(|&w| g.unit_is_warship(w) && !g.units[w].blockading));
    }
    out
}

/// The seas ship `u` reaches within [`BLOCKADE_TURNS`] (in range, by the reach flood): each with the
/// plot of it to go to (the nearest outside a city, by turns then plot; its own plot when it stands
/// in the sea) and the turns to get there.
fn reachable_seas(g: &Game, v: &mut View, u: UnitId) -> Vec<(SeaId, PlotIdx, i32)> {
    let team = v.team;
    let here = g.unit_plot(u);
    let mut out: Vec<(SeaId, PlotIdx, i32)> = Vec::new();
    if let Some(s) = g.map.plots[here].sea {
        out.push((s, here, 0));
    }
    let reach = v.flood.run(g, u, BLOCKADE_TURNS, false);
    let mut sorted: Vec<(i32, PlotIdx)> = reach.iter().map(|r| (r.turns, r.plot)).collect();
    sorted.sort();
    for (turns, q) in sorted {
        let Some(s) = g.map.plots[q].sea else { continue };
        if g.map.plots[q].plot_city.is_some() || !g.plot_in_sea_range(team, q) || out.iter().any(|e| e.0 == s) {
            continue;
        }
        out.push((s, q, turns));
    }
    out
}

/// Take the Patrol of sea `s` (docs/NAVAL.md, 4.2): at once when the ship stands in it outside a
/// city, else sail to `to` (in it) and patrol on arrival. `why` names the rule for the trace and
/// the telemetry.
fn go_patrol(g: &mut Game, v: &View, grp: GroupId, u: UnitId, s: SeaId, to: PlotIdx, why: &'static str) {
    go_patrol_orders(g, v, grp, u, s, to, why);
    // The fleet holds the Patrol of the sea as its standing order (docs/NAVAL.md, 7.2): the rules
    // keep it there over the turns (a ship in port forms a fleet of its own for it). Set after the
    // orders above, which end any standing order it had.
    if alive(g, grp) {
        if let Some(a) = fleet_for(g, v.p, grp) {
            let _ = g.army_set_station(v.p, a, Station::Patrol(s));
        }
    }
}

/// The fleet of our ship group `grp` (docs/NAVAL.md, 7.1): the fleet it is, or for ships of ours in
/// no fleet (in port: every unit of the group a fleet unit) a fleet formed of them. `None`: none.
fn fleet_for(g: &mut Game, p: PlayerId, grp: GroupId) -> Option<ArmyId> {
    let head = g.group_get_head_unit(grp)?;
    if let Some(a) = g.units[head].army.filter(|&a| g.army_is_fleet(a) && g.armies[a].group == Some(grp)) {
        return Some(a);
    }
    let units = g.groups[grp].units.clone();
    if units.iter().any(|&o| g.units[o].army.is_some() || !g.unit_is_fleet_unit(o)) {
        return None;
    }
    let a = g.army_form(p, &units).ok()?;
    (g.armies.contains(a) && g.armies[a].group == Some(grp)).then_some(a)
}

/// A ship group of ours decides again (`units::play`, before its role's decision): its fleet's
/// standing order ends, and the decision sets a new one if it is a Patrol or an Escort (a ship held
/// in port, to heal or to wait for cargo, must not be sailed out by an old order at its next turn).
pub(crate) fn clear_station(g: &mut Game, p: PlayerId, grp: GroupId) {
    let Some(head) = g.group_get_head_unit(grp) else { return };
    if let Some(a) = g.units[head].army.filter(|&a| g.army_is_fleet(a) && g.armies[a].group == Some(grp) && g.armies[a].station != Station::None) {
        let _ = g.army_set_station(p, a, Station::None);
    }
}

/// Whether ship `t` of ours carries land fighters (an invasion's cargo).
fn carries_army(g: &Game, t: UnitId) -> bool {
    g.unit_get_cargo_units(t).iter().any(|&c| g.unit_get_domain_type(c) == DomainType::DOMAIN_LAND && g.unit_can_fight(c) && !g.d.units[g.units[c].unit_type].found)
}

/// Rule 3 at sea: a loaded assault transport group of ours outside a city, with no warship in it
/// and no fleet escorting it, that this warship's group reaches within two turns gets it as its
/// escort: the Escort standing order (docs/NAVAL.md, 7.2; the rules move the fleet onto the
/// transports' plot after each of their steps it can follow that turn). True if it took the order.
fn escort_at_sea(g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if v.amphibious.is_none() || g.groups[grp].units.iter().any(|&o| g.unit_has_cargo(o)) {
        return false;
    }
    let here = g.unit_plot(u);
    let escorted: Vec<EscortTarget> = g.players[p].armies.iter().filter_map(|&a| match g.armies.get(a).map(|ar| ar.station) {
        Some(Station::Escort(t)) => Some(t),
        _ => None,
    }).collect();
    let mut best: Option<(i32, GroupId, EscortTarget)> = None;
    for tg in g.players[p].groups.clone() {
        if tg == grp || !alive(g, tg) || g.group_get_domain_type(tg) != Some(DomainType::DOMAIN_SEA) {
            continue;
        }
        let units = &g.groups[tg].units;
        let loaded = units.iter().any(|&t| g.units[t].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA) && carries_army(g, t));
        if !loaded || units.iter().any(|&o| armed(g, o) && !crate::naval::is_transport_type(&g.d.units[g.units[o].unit_type])) {
            continue;
        }
        let Some(at) = g.group_plot(tg) else { continue };
        if g.map.plots[at].plot_city.is_some() {
            continue;
        }
        let head = units[0];
        let target = match g.units[head].army.filter(|&a| g.army_is_fleet(a) && g.armies[a].group == Some(tg)) {
            Some(a) => EscortTarget::Army(a),
            None => EscortTarget::Group(tg),
        };
        if escorted.contains(&target) {
            continue;
        }
        let Some(path) = g.group_find_path(grp, here, at, 0) else { continue };
        if path.turns <= 2 && best.as_ref().map_or(true, |&(t, gr, _)| (path.turns, tg) < (t, gr)) {
            best = Some((path.turns, tg, target));
        }
    }
    let Some((_, _, target)) = best else { return false };
    let Some(a) = fleet_for(g, p, grp) else { return false };
    if g.army_set_station(p, a, Station::Escort(target)).is_err() {
        return false;
    }
    crate::trace!(g, p, "warship {:?} escorts {} at sea", u, g.escort_target_name(target));
    g.tel_decision(p, "warship", "escort", "escort_sea", 1, String::new);
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
    true
}

/// An escort of ours whose work is done (docs/NAVAL.md, 7.2): its fleet's target gone, carrying no
/// army any more, or the fleet worn, out of range or damaged by [`HEAL_PCT`]: the order ends and
/// the group, asleep under it, wakes to decide again (`units::play`). False otherwise (it keeps
/// escorting) or when the group escorts nothing.
pub(crate) fn escort_ends(g: &mut Game, v: &View, grp: GroupId) -> bool {
    if v.barbarian {
        return false;
    }
    let Some(head) = g.group_get_head_unit(grp) else { return false };
    let Some(a) = g.units[head].army.filter(|&a| g.army_is_fleet(a) && g.armies[a].group == Some(grp)) else { return false };
    let target = match g.armies[a].station {
        Station::Escort(t) => Some(t),
        _ => None,
    };
    let asleep = g.groups[grp].activity_type == ActivityType::ACTIVITY_SLEEP;
    let busy = match target {
        Some(EscortTarget::Army(b)) => g.armies.get(b).and_then(|ar| ar.group).is_some_and(|gr| g.groups.contains(gr) && g.groups[gr].units.iter().any(|&o| carries_army(g, o))),
        Some(EscortTarget::Group(gr)) => g.groups.contains(gr) && g.groups[gr].units.iter().any(|&o| carries_army(g, o)),
        // The rules ended it (its target gone): an asleep former escort wakes.
        None => return false,
    };
    let tired = g.groups[grp].units.iter().any(|&o| worn(g, o) || g.unit_is_out_of_sea_range(o) || damage_pct(g, o) >= HEAL_PCT);
    if busy && !tired {
        return false;
    }
    crate::trace!(g, v.p, "escort of {:?} ends ({})", head, if tired { "worn, out of range or hurt" } else { "nothing aboard" });
    let _ = g.army_set_station(v.p, a, Station::None);
    if asleep {
        g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
    }
    true
}

/// The orders of [`go_patrol`].
fn go_patrol_orders(g: &mut Game, v: &View, grp: GroupId, u: UnitId, s: SeaId, to: PlotIdx, why: &'static str) {
    let p = v.p;
    let here = g.unit_plot(u);
    let mai = Some(MissionAIType::MISSIONAI_BLOCKADE);
    if g.map.plots[here].sea == Some(s) && g.map.plots[here].plot_city.is_none() && can_start(g, grp, MissionType::MISSION_PLUNDER, -1, -1) {
        crate::trace!(g, p, "warship {:?} patrols {} at {:?} ({})", u, g.map.seas[s.idx()].name, g.map.xy(here), why);
        g.tel_decision(p, "warship", why, "patrol", 1, String::new);
        g.group_push_mission(grp, MissionType::MISSION_PLUNDER, -1, -1, 0, false, false, mai, Some(here), None);
        if alive(g, grp) && g.group_ready_to_move(grp, false) {
            skip(g, grp);
        }
        return;
    }
    crate::trace!(g, p, "warship {:?} at {:?} sails to patrol {} at {:?} ({})", u, g.map.xy(here), g.map.seas[s.idx()].name, g.map.xy(to), why);
    g.tel_decision(p, "warship", why, "patrol_sail", 1, String::new);
    let (x, y) = g.map.xy(to);
    g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, x, y, 0, false, false, mai, Some(to), None);
    if !alive(g, grp) || !g.units.contains(u) {
        return;
    }
    let at = g.unit_plot(u);
    if g.group_ready_to_move(grp, false) && g.map.plots[at].sea == Some(s) && g.map.plots[at].plot_city.is_none() && can_start(g, grp, MissionType::MISSION_PLUNDER, -1, -1) {
        g.group_push_mission(grp, MissionType::MISSION_PLUNDER, -1, -1, 0, false, false, mai, Some(at), None);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
}

/// What the closing of sea `s` denies the civilizations we are at war with, by what we know: each
/// of their water plots in it 1, 2 more worked, 2 more with a resource; each of their ports on it
/// 4, 10 more when the closing would blockade it (its other seas are closed to it), 20 more when an
/// army of ours besieges it. With the second value: whether a port on it is besieged by us.
fn sea_value(g: &Game, v: &View, s: SeaId, ports: &[(CityId, bool)]) -> (i32, bool) {
    let mut score = 0;
    for &w in &g.map.seas[s.idx()].plots {
        if !g.map.plots[w].revealed[v.team] {
            continue;
        }
        let Some(t) = g.plot_get_team(w) else { continue };
        if !v.civ_enemies.contains(&t) {
            continue;
        }
        score += 1 + 2 * i32::from(g.plot_is_being_worked(w)) + 2 * i32::from(g.plot_get_bonus_type(w, Some(v.team)).is_some());
    }
    let mut siege = false;
    for &(c, besieged) in ports {
        if !g.city_seas(c).contains(&s) {
            continue;
        }
        score += 4;
        if g.city_harbour_if_held(c, s) {
            score += 10;
        }
        if besieged {
            score += 20;
            siege = true;
        }
    }
    (score, siege)
}

/// The coastal cities of the civilizations at war with us that we have revealed, each with whether
/// an army of ours besieges it.
fn enemy_ports(g: &Game, v: &View) -> Vec<(CityId, bool)> {
    let mut out = Vec::new();
    for q in g.player_ids() {
        if !g.players[q].alive || g.player_is_barbarian(q) || !v.civ_enemies.contains(&g.players[q].team) {
            continue;
        }
        for &c in &g.players[q].cities {
            if !g.cities.contains(c) || !g.city_is_revealed(c, v.team, false) || !crate::naval::is_coastal_city(g, c) {
                continue;
            }
            out.push((c, g.city_besiegers(c).iter().any(|&b| g.units[b].owner == v.p)));
        }
    }
    out
}

/// Rules 5 and 8. The Patrol of the sea of an enemy coast in reach that denies the most
/// ([`sea_value`] per turn of travel, twice where this ship's Patrol would close the sea to an enemy
/// with a port on it); `siege`: only seas of a port our army besieges. Not a sea where we would be
/// outmatched, nor one already closed to every enemy with a port on it by our other ships (unless
/// this ship patrols there), nor a plot an enemy ship that beats it can reach. An enemy sea
/// improvement under it is pillaged first.
fn blockade(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, seas: &[(SeaId, PlotIdx, i32)], siege: bool) -> bool {
    let p = v.p;
    let team = v.team;
    // An enemy sea improvement under us: pillage it.
    if !siege && pillage_here(g, v, grp, u) {
        return true;
    }
    let ports = enemy_ports(g, v);
    if ports.is_empty() || (siege && !ports.iter().any(|e| e.1)) {
        return false;
    }
    let here = g.unit_plot(u);
    let patrolling = g.units[u].blockading;
    let mut best: Option<(i64, SeaId, PlotIdx, i32)> = None;
    for &(s, to, turns) in seas {
        let (value, besieged) = sea_value(g, v, s, &ports);
        if value < BLOCKADE_MIN || (siege && !besieged) {
            continue;
        }
        let mine_here = patrolling && g.map.plots[here].sea == Some(s);
        let others = bound_for(g, p, s, grp);
        let mut with = others.clone();
        with.push(u);
        if outmatched_if(g, team, s, &with) {
            continue;
        }
        // The enemy teams with a port on this sea.
        let mut teams: Vec<TeamId> = ports.iter().filter(|e| g.city_seas(e.0).contains(&s)).map(|e| g.city_team(e.0)).collect();
        teams.sort();
        teams.dedup();
        let closed_without = !teams.is_empty() && teams.iter().all(|&t| g.sea_state_known_if(s, t, team, &others).closed());
        if closed_without && !mine_here {
            continue;
        }
        let closes = teams.iter().any(|&t| g.sea_state_known_if(s, t, team, &with).closed());
        if !mine_here && endangered(g, team, u, to) {
            continue;
        }
        let bonus = if mine_here { 6 } else { 5 };
        let key = value as i64 * 100 * bonus * if closes { 2 } else { 1 } / (turns as i64 + 2);
        if best.map_or(true, |(k, bs, _, _)| key > k || key == k && s < bs) {
            best = Some((key, s, to, turns));
        }
    }
    let Some((_, s, to, _)) = best else { return false };
    go_patrol(g, v, grp, u, s, to, if siege { "siege_patrol" } else { "blockade" });
    true
}

/// Rule 6. The Patrol of a sea one of our ports stands on (ports by population, the largest
/// first) where an enemy patrols or a warship of an enemy is seen, that this ship reaches, when it
/// with our ships there or bound there is not outmatched two to one and our ships there or bound
/// there do not hold it already.
fn guard(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, seas: &[(SeaId, PlotIdx, i32)]) -> bool {
    let p = v.p;
    let team = v.team;
    let mut ports: Vec<(i32, CityId)> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| (-g.cities[c].population, c)).collect();
    ports.sort();
    let here = g.unit_plot(u);
    let patrolling = g.units[u].blockading;
    // The seas of our ports, then those our outposts' lanes pass through (docs/NAVAL.md, 8).
    let mut order: Vec<SeaId> = Vec::new();
    for &(_, c) in &ports {
        for s in g.city_seas(c) {
            if !order.contains(&s) {
                order.push(s);
            }
        }
    }
    for &(_, c) in &ports {
        if let Some(lane) = g.cities[c].outpost.then(|| g.outpost_lane(c)).flatten() {
            for s in lane.path.iter().filter_map(|&w| g.map.plots[w].sea) {
                if !order.contains(&s) {
                    order.push(s);
                }
            }
        }
    }
    {
        for s in order {
            let Some(&(_, to, _)) = seas.iter().find(|e| e.0 == s) else { continue };
            let st = g.sea_state_known(s, team);
            let enemy_patrols = st.theirs > 0 && matches!(st.control, SeaControl::HeldBy(_) | SeaControl::Contested(_));
            let enemy_ships = g.map.seas[s.idx()].plots.iter().any(|&w| {
                g.plot_is_visible(w, team) && g.map.plots[w].units.iter().any(|&o| g.unit_is_warship(o) && g.unit_is_enemy(o, Some(team), Some(w)) && !g.unit_is_invisible(o, team, false))
            });
            if !enemy_patrols && !enemy_ships {
                continue;
            }
            let mine_here = patrolling && g.map.plots[here].sea == Some(s);
            let others = bound_for(g, p, s, grp);
            if !mine_here && g.sea_state_known_if(s, team, team, &others).control == SeaControl::HeldByUs {
                continue;
            }
            let mut with = others;
            with.push(u);
            if outmatched_if(g, team, s, &with) {
                continue;
            }
            go_patrol(g, v, grp, u, s, to, "guard");
            return true;
        }
    }
    false
}

/// Pillage the enemy improvement on the plot we stand on (the rules decide: `canPillage`).
fn pillage_here(g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    let here = g.unit_plot(u);
    if !g.plot_is_water(here) || g.map.plots[here].improvement_type.is_none() {
        return false;
    }
    let owner_team = g.plot_get_team(here);
    if owner_team.is_some_and(|t| !v.enemies.contains(&t)) || owner_team.is_none() || !g.unit_can_pillage(u, here) {
        return false;
    }
    if !can_start(g, grp, MissionType::MISSION_PILLAGE, -1, -1) {
        return false;
    }
    crate::trace!(g, v.p, "ship {:?} pillages at {:?}", u, g.map.xy(here));
    g.tel_decision(v.p, "warship", "pillage", "pillage", 1, String::new);
    push(g, grp, MissionType::MISSION_PILLAGE, -1, -1, 0, false);
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
    true
}

/// Rule 9: the guard post of this warship (one per post; the warship of the lowest id standing
/// there holds it), else the main port; it patrols a sea of that port it reaches (the one where we
/// are weakest), else is fortified in the port.
fn post(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, seas: &[(SeaId, PlotIdx, i32)]) {
    let p = v.p;
    let team = v.team;
    let coastal: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| crate::naval::is_coastal_city(g, c)).collect();
    if coastal.is_empty() {
        hold(g, grp);
        return;
    }
    // Posts: (barbarian ship seen near first, then the nearest rival coast), city.
    let mut posts: Vec<(i32, i32, CityId)> = Vec::new();
    for &c in &coastal {
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        let mut barb = false;
        for dx in -POST_BARB_DIST..=POST_BARB_DIST {
            for dy in -POST_BARB_DIST..=POST_BARB_DIST {
                let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
                if g.plot_is_water(q) && g.plot_is_visible(q, team) && g.map.plots[q].units.iter().any(|&e| g.player_is_barbarian(g.units[e].owner) && g.unit_get_domain_type(e) == DomainType::DOMAIN_SEA && armed(g, e) && !g.unit_is_invisible(e, team, false)) {
                    barb = true;
                }
            }
        }
        let mut rival = i32::MAX;
        for o in g.player_ids() {
            if g.players[o].team == team || !g.players[o].alive || g.player_is_barbarian(o) {
                continue;
            }
            for &oc in &g.players[o].cities {
                let op = g.city_plot(oc);
                if g.map.plots[op].revealed[team] && crate::naval::is_coastal_city(g, oc) {
                    rival = rival.min(g.plot_distance(cx, cy, g.cities[oc].x, g.cities[oc].y));
                }
            }
        }
        if barb || rival <= POST_RIVAL_DIST {
            posts.push((i32::from(!barb), rival, c));
        }
    }
    posts.sort();
    // Held: another warship of ours stands in the port or patrols (or is bound for) its seas; this
    // warship keeps the post it holds. In the port, the lower id holds it.
    let here = g.unit_plot(u);
    let here_sea = g.map.plots[here].sea;
    let mine_at = |g: &Game, c: CityId| g.city_plot(c) == here || here_sea.is_some_and(|s| g.city_seas(c).contains(&s));
    let held = |g: &Game, c: CityId| {
        let cp = g.city_plot(c);
        let in_port = g.map.plots[cp].units.iter().any(|&o| o != u && (cp != here || o < u) && g.units[o].owner == p && our_ship(g, o) && !crate::naval::is_transport_type(&g.d.units[g.units[o].unit_type]));
        in_port || g.city_seas(c).iter().any(|&s| g.sea_patrollers(s).iter().any(|&o| o != u && g.units[o].owner == p) || !bound_for(g, p, s, grp).is_empty())
    };
    let free = |g: &Game, c: CityId| !held(g, c) && !v.is_claimed(g.city_plot(c));
    let mine = posts.iter().map(|&(_, _, c)| c).find(|&c| mine_at(g, c) && free(g, c)).or_else(|| posts.iter().map(|&(_, _, c)| c).find(|&c| free(g, c)));
    let target = mine.or_else(|| {
        // The main port: the most worked sea plots and sea resources within the city's radius.
        coastal.iter().copied().max_by_key(|&c| {
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            let mut s = 0;
            for dx in -2i32..=2 {
                for dy in -2i32..=2 {
                    if dx.abs() + dy.abs() == 4 {
                        continue;
                    }
                    let Some(w) = g.plot_xy(cx, cy, dx, dy) else { continue };
                    if g.plot_is_water(w) && g.map.plots[w].owner == Some(p) {
                        s += i32::from(g.plot_is_being_worked(w)) + 2 * i32::from(g.plot_get_bonus_type(w, Some(team)).is_some());
                    }
                }
            }
            (s, std::cmp::Reverse(c))
        })
    });
    let Some(c) = target else {
        hold(g, grp);
        return;
    };
    let cp = g.city_plot(c);
    if mine.is_some() {
        v.claimed.push(cp);
    }
    // A sea of the port this ship reaches, where we are weakest (ties: the lower id); the one it
    // patrols already when it is one of them.
    let sea = here_sea.filter(|&s| g.units[u].blockading && g.city_seas(c).contains(&s)).and_then(|s| seas.iter().find(|e| e.0 == s)).or_else(|| {
        g.city_seas(c).into_iter().filter_map(|s| seas.iter().find(|e| e.0 == s)).min_by_key(|e| (g.sea_state_known(e.0, team).ours, e.0))
    });
    if let Some(&(s, to, _)) = sea {
        go_patrol(g, v, grp, u, s, to, if mine.is_some() { "post" } else { "port" });
        return;
    }
    crate::trace!(g, p, "warship {:?} at {:?} {} {:?}", u, g.map.xy(g.unit_plot(u)), if mine.is_some() { "posts at" } else { "waits in port" }, g.map.xy(cp));
    g.tel_decision(p, "warship", if mine.is_some() { "post" } else { "port" }, "peace", 1, String::new);
    go(g, grp, cp, false, Some(MissionAIType::MISSIONAI_GUARD_CITY));
}

/// A ship on Patrol that keeps its station this turn without new orders (`units::play` leaves the
/// group on its Patrol): not worn, in range, healthy, not outmatched two to one in its sea, not a
/// transport `naval.rs` now wants, no enemy ship within its moves + 1, and not on a turn when it
/// looks again (every [`RELOOK_TURNS`]).
pub(crate) fn keeps_station(g: &Game, v: &mut View, grp: GroupId) -> bool {
    if v.barbarian || !crate::switches::WARSHIPS.on(g, v.p) || g.groups[grp].activity_type != ActivityType::ACTIVITY_PLUNDER {
        return false;
    }
    let Some(u) = g.group_get_head_unit(grp) else { return false };
    let Some(here) = g.unit_plot_opt(u) else { return false };
    let Some(s) = g.map.plots[here].sea else { return false };
    if !g.units[u].blockading || g.groups[grp].units.iter().any(|&o| worn(g, o) || g.unit_is_out_of_sea_range(o) || damage_pct(g, o) >= HEAL_PCT) {
        crate::trace!(g, v.p, "patrol of {:?} ends: worn, out of range or hurt", u);
        return false;
    }
    if outmatched_if(g, v.team, s, &[]) {
        crate::trace!(g, v.p, "patrol of {:?} in {} ends: outmatched", u, g.map.seas[s.idx()].name);
        return false;
    }
    // A transport `naval.rs` now wants leaves.
    if crate::naval::is_transport_type(&g.d.units[g.units[u].unit_type]) && !free_transport(g, v, grp, u) {
        return false;
    }
    // An enemy ship close enough to attack: decide again.
    let (x, y) = g.map.xy(here);
    let r = g.d.units[g.units[u].unit_type].moves.max(1) + 1;
    for dx in -r..=r {
        for dy in -r..=r {
            if g.plot_xy(x, y, dx, dy).is_some_and(|q| enemy_ships_on(g, v.team, q)) {
                crate::trace!(g, v.p, "patrol of {:?} at {:?}: an enemy ship in reach", u, g.map.xy(here));
                return false;
            }
        }
    }
    (g.game_turn() + grp.0 as i32) % RELOOK_TURNS != 0
}

/// A ship's group that must make for port first (docs/NAVAL.md, section 8): one out of its team's
/// range, or a warship whose wear is above half its maximum that carries nothing. It sails to the
/// port nearest it by the range field (a city of a team whose ports are ours: its wear clears
/// there), else to one of our coastal cities (`naval::port`). True if its turn is over. Every ship
/// of ours, whatever its role (`units::play`).
pub(crate) fn to_port(g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || g.unit_get_domain_type(u) != DomainType::DOMAIN_SEA {
        return false;
    }
    let here = g.unit_plot(u);
    if !g.plot_is_water(here) {
        return false;
    }
    let team = g.unit_get_team(u);
    let out = g.groups[grp].units.iter().any(|&o| g.unit_is_out_of_sea_range(o));
    let worn = g.groups[grp].units.iter().any(|&o| worn(g, o)) && !g.groups[grp].units.iter().any(|&o| g.unit_has_cargo(o));
    if !out && !worn {
        return false;
    }
    crate::trace!(g, v.p, "ship group {:?} at {:?} to port ({})", grp, g.map.xy(here), if out { "out of range" } else { "wear" });
    g.tel_decision(v.p, "naval", "to_port", if out { "out_of_range" } else { "wear" }, 1, String::new);
    if let Some(c) = g.plot_sea_port(team, here).filter(|&c| g.cities.contains(c)) {
        let cp = g.city_plot(c);
        if g.group_find_path(grp, here, cp, 0).is_some() {
            move_to(g, grp, cp, 0);
            if alive(g, grp) && g.group_ready_to_move(grp, false) {
                skip(g, grp);
            }
            return true;
        }
    }
    crate::naval::port(g, v.p, grp, here);
    true
}

// ----------------------------------------------------------------------------
// Barbarian ships and sea animals
// ----------------------------------------------------------------------------

/// One decision for a barbarian ship or sea animal. False when the switch is off (it holds).
pub fn barbarian(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if !crate::switches::WARSHIPS.on(g, p) || !armed(g, u) || g.unit_is_cargo(u) {
        return false;
    }
    // Attack at the odds land animals attack at.
    let ts = targets(g, v, u);
    if let Some(&(q, _, odds)) = ts.iter().filter(|t| t.2 >= BARB_ODDS).max_by_key(|t| (t.2, std::cmp::Reverse(t.0))) {
        return attack(g, v, grp, u, q, odds, "barbarian");
    }
    if !alive(g, grp) || !g.units.contains(u) {
        return true;
    }
    // A sea improvement under it.
    if pillage_here(g, v, grp, u) {
        return true;
    }
    // Hunt a ship it beats within reach.
    let here = g.unit_plot(u);
    let reach = v.flood.run(g, u, BARB_HUNT_TURNS, true);
    let prey = reach
        .iter()
        .filter(|r| r.turns > 0 && enemy_ships_on(g, v.team, r.plot))
        .filter(|r| g.plot_get_best_defender(r.plot, None, Some(p), Some(u), false, false, false).is_some_and(|d| g.get_combat_odds(u, d) >= BARB_ODDS))
        .min_by_key(|r| (r.turns, r.plot))
        .copied();
    if let Some(r) = prey {
        if let Some(step) = reach.iter().filter(|s| {
            let (x, y) = g.map.xy(s.plot);
            let (rx, ry) = g.map.xy(r.plot);
            s.plot != r.plot && !enemy_ships_on(g, v.team, s.plot) && g.plot_distance(x, y, rx, ry) == 1
        }).min_by_key(|s| (s.turns, s.plot)).copied() {
            g.tel_decision(p, "warship", "barb_hunt", "barbarian", 1, String::new);
            go(g, grp, step.plot, true, None);
            return true;
        }
    }
    // Wander: a plot reached this turn, picked by a hash of unit and turn (no RNG draw; the land
    // animals' wander, `units::barbarian`).
    // (Within its moves in plots: a ship of one move reaches its neighbours only "next turn" in
    // the flood's count, which rounds a step that spends its last move up.)
    let (hx, hy) = g.map.xy(here);
    let moves = g.d.units[g.units[u].unit_type].moves.max(1);
    let near: Vec<PlotIdx> = reach
        .iter()
        .filter(|r| {
            let (x, y) = g.map.xy(r.plot);
            r.turns <= 1 && r.plot != here && g.plot_distance(hx, hy, x, y) <= moves && g.plot_is_water(r.plot) && !enemy_ships_on(g, v.team, r.plot)
        })
        .map(|r| r.plot)
        .collect();
    if !near.is_empty() {
        let k = ((u.0 as u64).wrapping_mul(2654435761).wrapping_add((g.game_turn() as u64).wrapping_mul(40503)) % near.len() as u64) as usize;
        crate::trace!(g, p, "sea barbarian {:?} at {:?} roams to {:?}", u, g.map.xy(here), g.map.xy(near[k]));
        go(g, grp, near[k], true, None);
        return true;
    }
    false
}

// ----------------------------------------------------------------------------
// Production
// ----------------------------------------------------------------------------

/// The warship coastal city `c` trains, if a reason wants one (module doc): the best it can train
/// by strength (power squared per turn, as `production::best_unit` weighs attackers).
// warships (warships.rs): production hook, called from `production::decide`.
pub fn production(g: &Game, c: CityId) -> Option<Choice> {
    let p = g.cities[c].owner;
    if g.player_is_barbarian(p) || !crate::switches::WARSHIPS.on(g, p) || !crate::naval::is_coastal_city(g, c) {
        return None;
    }
    let want = wanted(g, p);
    if want <= 0 {
        return None;
    }
    let have = warships_had(g, p);
    if have >= want {
        return None;
    }
    // Within a share of the military's hammers.
    let (ships, all) = military_hammers(g, p);
    if all > 0 && (ships * 100) / all >= HAMMER_SHARE_PCT {
        return None;
    }
    let u = best_warship(g, c)?;
    let ai = if g.d.units[u].unit_ai_type[UnitAIType::UNITAI_ATTACK_SEA] { UnitAIType::UNITAI_ATTACK_SEA } else { UnitAIType::UNITAI_ESCORT_SEA };
    crate::trace!(g, p, "{} trains a warship ({} of {} wanted)", g.cities[c].name, have, want);
    Some(Choice::Train(u, ai))
}

/// Warships of ours and in training: sea units with strength trained or used as warships (no room,
/// or the warship AI types).
fn warships_had(g: &Game, p: PlayerId) -> i32 {
    let pl = &g.players[p];
    let alive = pl
        .units
        .iter()
        .filter(|&&u| {
            let info = &g.d.units[g.units[u].unit_type];
            info.domain_type == Some(DomainType::DOMAIN_SEA)
                && info.combat > 0
                && (is_warship_type(info) || matches!(g.units[u].ai_type, Some(UnitAIType::UNITAI_ATTACK_SEA) | Some(UnitAIType::UNITAI_ESCORT_SEA)))
        })
        .count() as i32;
    alive + pl.ai_num_train_ai_units[UnitAIType::UNITAI_ATTACK_SEA] + pl.ai_num_train_ai_units[UnitAIType::UNITAI_ESCORT_SEA]
}

/// (Hammers in our armed sea units, hammers in all our units with strength.)
fn military_hammers(g: &Game, p: PlayerId) -> (i64, i64) {
    let mut ships = 0i64;
    let mut all = 0i64;
    for &u in &g.players[p].units {
        let info = &g.d.units[g.units[u].unit_type];
        if info.combat <= 0 || info.production_cost <= 0 {
            continue;
        }
        all += info.production_cost as i64;
        if info.domain_type == Some(DomainType::DOMAIN_SEA) {
            ships += info.production_cost as i64;
        }
    }
    (ships, all)
}

/// The warships player `p` wants (module doc).
fn wanted(g: &Game, p: PlayerId) -> i32 {
    let team = g.players[p].team;
    let ports: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| crate::naval::is_coastal_city(g, c)).collect();
    if ports.is_empty() {
        return 0;
    }
    let mut want = 0;
    // Escorts: one per ESCORT_PER_TRANSPORTS assault transports.
    if crate::naval::amphibious_target(g, p).is_some() {
        let assault = g.players[p].units.iter().filter(|&&u| g.units[u].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA)).count() as i32
            + g.players[p].ai_num_train_ai_units[UnitAIType::UNITAI_ASSAULT_SEA];
        want += (assault + ESCORT_PER_TRANSPORTS - 1) / ESCORT_PER_TRANSPORTS;
    }
    // Guards: the enemy armed ships seen near our ports.
    let mut seen: Vec<UnitId> = Vec::new();
    let mut best_enemy = 0;
    for &c in &ports {
        let (cx, cy) = (g.cities[c].x, g.cities[c].y);
        for dx in -SEEN_RADIUS..=SEEN_RADIUS {
            for dy in -SEEN_RADIUS..=SEEN_RADIUS {
                let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
                if !g.plot_is_water(q) || !g.plot_is_visible(q, team) {
                    continue;
                }
                for &e in &g.map.plots[q].units {
                    if enemy_ship(g, team, e) && armed(g, e) && !g.unit_is_animal(e) && !seen.contains(&e) {
                        seen.push(e);
                    }
                }
            }
        }
    }
    let ours_best = ports.iter().filter_map(|&c| best_warship(g, c)).map(|u| g.d.units[u].combat).max().unwrap_or(0);
    // Guards only against ships ours can fight (an animal or monster far beyond our ships is
    // waited out in port: rule 7).
    let beatable = seen.iter().filter(|&&e| g.d.units[g.units[e].unit_type].combat * 2 <= ours_best * 3).count() as i32;
    // ... and two for each sea of our ports an enemy holds or contests, by what we see.
    let mut contested: Vec<SeaId> = Vec::new();
    for &c in &ports {
        for s in g.city_seas(c) {
            if !contested.contains(&s) && matches!(g.sea_state_known(s, team).control, SeaControl::HeldBy(_) | SeaControl::Contested(_)) {
                contested.push(s);
            }
        }
    }
    want += beatable.max(2 * contested.len() as i32);
    // Blockaders: at war with a civilization whose ships we have seen are not stronger than ours,
    // one for each sea of its revealed coast that reaches into our range.
    let mut targets: Vec<SeaId> = Vec::new();
    for o in g.player_ids() {
        let ot = g.players[o].team;
        if ot == team || !g.players[o].alive || g.player_is_barbarian(o) || !g.at_war(team, ot) {
            continue;
        }
        for &u in &g.players[o].units {
            let info = &g.d.units[g.units[u].unit_type];
            if info.domain_type == Some(DomainType::DOMAIN_SEA) && g.plot_is_visible(g.unit_plot(u), team) {
                best_enemy = best_enemy.max(info.combat);
            }
        }
        for &oc in &g.players[o].cities {
            if !g.city_is_revealed(oc, team, false) || !crate::naval::is_coastal_city(g, oc) {
                continue;
            }
            for s in g.city_seas(oc) {
                if !targets.contains(&s) && g.map.seas[s.idx()].plots.iter().any(|&w| g.plot_in_sea_range(team, w)) {
                    targets.push(s);
                }
            }
        }
    }
    if !targets.is_empty() && ours_best > 0 && best_enemy <= ours_best {
        want += (targets.len() as i32).min(BLOCKADERS_MAX);
    }
    // A seafaring civilization: one more, for any reason above.
    if want > 0 && seafaring(g, p) {
        want += 1;
    }
    want
}

/// A civilization whose capital is coastal and whose data gives it a sea unit of its own (a unit
/// class whose civilization unit differs from the class's default).
fn seafaring(g: &Game, p: PlayerId) -> bool {
    let Some(cap) = g.players[p].capital_city.filter(|&c| g.cities.contains(c)) else { return false };
    if !crate::naval::is_coastal_city(g, cap) {
        return false;
    }
    let Some(civ) = g.player_get_civilization_type(p) else { return false };
    g.d.unit_classes.iter_enumerated().any(|(class, ci)| {
        let own = g.d.civ_unit(civ, class);
        own.is_some() && own != ci.default_unit_index && own.is_some_and(|u| g.d.units[u].domain_type == Some(DomainType::DOMAIN_SEA) && g.d.units[u].combat > 0)
    })
}

/// The best warship city `c` can train: a sea unit with strength, no hero, the attack or escort
/// sea AI; by strength squared per turn of training.
fn best_warship(g: &Game, c: CityId) -> Option<UnitType> {
    let d = g.d;
    let mut best: Option<(UnitType, i64)> = None;
    for u in d.units.keys() {
        let info = &d.units[u];
        if info.domain_type != Some(DomainType::DOMAIN_SEA) || info.combat <= 0 || crate::production::is_hero(d, info) {
            continue;
        }
        if !(info.unit_ai_type[UnitAIType::UNITAI_ATTACK_SEA] || info.unit_ai_type[UnitAIType::UNITAI_ESCORT_SEA]) {
            continue;
        }
        if !g.city_offers_unit(c, u) || !g.city_can_train(c, u, false, false, false, false) {
            continue;
        }
        let power = crate::view::type_attack_power(d, u) as i64;
        let turns = g.city_production_turns_left_unit(c, u, 0).max(1) as i64;
        let score = power * power / 100 * 1000 / (turns + 4);
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((u, score));
        }
    }
    best.map(|(u, _)| u)
}
