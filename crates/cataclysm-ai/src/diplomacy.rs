//! War and peace: when to offer or accept peace, when to declare war, Crusades.
//!
//! Peace needs both sides (`Command::MakePeace` offers, or signs when the other side's offer
//! stands), so each civilization only answers "do I want this war to stop" from its own side;
//! it offers whenever it does, and the war ends when both do.

use cataclysm_core::{Alignment, CaptainAura, Command, Event, FactionId, Game, PlayerId, PlotIdx};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::military::{can_take_city_of, free_power, held_lifeboat, lifeboat_assault_target, sentinel_power, THREAT_RANGE};
use crate::try_cmd;

/// No peace in a war's first turns: a war is not judged before it has been fought.
const MIN_WAR_TURNS: i32 = 8;
/// We are clearly weaker below this share (percent) of the enemy's squad power: we want peace.
const WEAKER_PCT: i32 = 80;
/// We are clearly stronger from this share: we keep fighting.
const STRONGER_PCT: i32 = 150;
/// A war this old that we are not clearly winning is going nowhere: we want peace.
const STALE_WAR_TURNS: i32 = 15;
/// Declaring war: our free squads need this share (percent) of what defends the target's nearest
/// city (the warlike civilizations: `WARLIKE_EDGE_PCT`), and the city is at most this far from
/// one of ours.
const DECLARE_EDGE_PCT: i32 = 150;
const WARLIKE_EDGE_PCT: i32 = 120;
const DECLARE_REACH: i32 = 10;
/// The first turn a civilization starts a war (warlike ones earlier).
const DECLARE_FROM_TURN: i32 = 30;
const WARLIKE_FROM_TURN: i32 = 20;
/// Supply cost of a step (the engine's: road or own land 1, elsewhere 3).
const SUPPLY_OWN: i32 = 1;
const SUPPLY_WILD: i32 = 3;

fn warlike(f: FactionId) -> bool {
    matches!(f, FactionId::Hippus | FactionId::Bannor | FactionId::Sheaim | FactionId::Infernals)
}

fn power(g: &Game, p: PlayerId) -> i32 {
    g.squads.values().filter(|s| s.owner == p).map(|s| s.power_rating()).sum()
}

/// Our squad power against `e`'s, in percent.
fn ratio(g: &Game, p: PlayerId, e: PlayerId) -> i32 {
    power(g, p) * 100 / power(g, e).max(1)
}

/// The civilizations `p` is at war with (the Wild Clans never count).
fn civ_wars(g: &Game, p: PlayerId) -> Vec<PlayerId> {
    g.players.iter().filter(|o| o.alive && !o.is_neutral && o.id != p && g.at_war(p, o.id)).map(|o| o.id).collect()
}

/// `e`'s squads near a city of ours outmatch everything of ours that could hold it.
fn overmatched_city(g: &Game, p: PlayerId, e: PlayerId) -> bool {
    g.cities.values().filter(|c| c.owner == p).any(|c| {
        let theirs: i32 = g.squads.values().filter(|s| s.owner == e && g.world.distance(s.plot, c.plot) <= THREAT_RANGE).map(|s| s.power_rating()).sum();
        let ours: i32 = g.squads.values().filter(|s| s.owner == p && g.world.distance(s.plot, c.plot) <= THREAT_RANGE).map(|s| s.power_rating()).sum::<i32>() + sentinel_power(g, c.id);
        theirs > 0 && theirs > ours
    })
}

/// The owner of the Lifeboat our squads are sent at, if any.
fn assault_target_owner(g: &Game, p: PlayerId) -> Option<PlayerId> {
    let plot = lifeboat_assault_target(g, p)?;
    let cid = g.world.plot(plot).city?;
    g.cities.get(&cid).map(|c| c.owner)
}

/// Why `p` wants its war with `e` to stop, or `None` when it does not.
fn peace_reason(g: &Game, p: PlayerId, e: PlayerId, wars: &[PlayerId]) -> Option<&'static str> {
    let infernals = |x: PlayerId| g.players[x.idx()].faction == FactionId::Infernals;
    if infernals(p) || infernals(e) {
        return None;
    }
    if g.crusades.values().any(|c| c.active && c.caller == p && c.target == e) {
        return None;
    }
    if g.war_turns(p, e).unwrap_or(0) < MIN_WAR_TURNS {
        return None;
    }
    if g.apocalypse.carcosa_risen {
        if assault_target_owner(g, p) == Some(e) {
            return None;
        }
        if held_lifeboat(g, p).is_some() {
            return Some("we hold a Lifeboat");
        }
    }
    let r = ratio(g, p, e);
    // Winning: clearly stronger, or a city of theirs is ours to take this turn.
    if r >= STRONGER_PCT || can_take_city_of(g, p, e) {
        return None;
    }
    if r < WEAKER_PCT {
        return Some("weaker");
    }
    if g.war_turns(p, e).unwrap_or(0) >= STALE_WAR_TURNS {
        return Some("stale");
    }
    if overmatched_city(g, p, e) {
        return Some("a city overmatched");
    }
    // At war with several: keep only the war we have the best ratio in (King Cinder's Host's war,
    // which can never end, does not count).
    let ends: Vec<PlayerId> = wars.iter().copied().filter(|&o| !infernals(o)).collect();
    if ends.len() >= 2 {
        let best = ends.iter().copied().max_by_key(|&o| (ratio(g, p, o), Reverse(o))).unwrap();
        if best != e {
            return Some("another war is better");
        }
    }
    None
}

fn manage_peace(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let wars = civ_wars(g, p);
    for &e in &wars {
        // The reason is kept for reading traces; any reason is enough to offer.
        if g.check_peace(p, e).is_ok() && peace_reason(g, p, e, &wars).is_some() {
            try_cmd(g, p, Command::MakePeace { target: e }, events);
        }
    }
}

/// Cheapest supply cost from one of `p`'s cities to every plot (the engine's step costs, no
/// enemy in the way: we are at peace with the target when this is asked).
fn supply_costs(g: &Game, p: PlayerId) -> Vec<i32> {
    let mut dist = vec![i32::MAX; g.world.len()];
    let mut pq = BinaryHeap::new();
    for c in g.cities.values().filter(|c| c.owner == p) {
        dist[c.plot.idx()] = 0;
        pq.push(Reverse((0, c.plot)));
    }
    while let Some(Reverse((d, u))) = pq.pop() {
        if d > dist[u.idx()] {
            continue;
        }
        for nb in g.world.neighbors(u) {
            let pl = g.world.plot(nb);
            if !pl.kind.is_passable_land() {
                continue;
            }
            let nd = d + if pl.road || pl.owner == Some(p) { SUPPLY_OWN } else { SUPPLY_WILD };
            if nd < dist[nb.idx()] {
                dist[nb.idx()] = nd;
                pq.push(Reverse((nd, nb)));
            }
        }
    }
    dist
}

/// What `e` can defend its city on `plot` with: every squad of its within `DECLARE_REACH` of it
/// (in peace they stand anywhere: at lairs, on roads, at home) and the city's Sentinels.
fn defense_of(g: &Game, e: PlayerId, plot: PlotIdx) -> i32 {
    let cid = g.world.plot(plot).city.unwrap();
    g.squads.values().filter(|s| s.owner == e && g.world.distance(s.plot, plot) <= DECLARE_REACH).map(|s| s.power_rating()).sum::<i32>() + sentinel_power(g, cid)
}

/// The civilization worth a war now, if any: one whose nearest city our free squads outmatch,
/// within reach and supply.
fn war_target(g: &Game, p: PlayerId) -> Option<PlayerId> {
    let me = &g.players[p.idx()];
    let wl = warlike(me.faction);
    if g.turn < if wl { WARLIKE_FROM_TURN } else { DECLARE_FROM_TURN } {
        return None;
    }
    let edge = if wl { WARLIKE_EDGE_PCT } else { DECLARE_EDGE_PCT };
    let free = free_power(g, p);
    let costs = supply_costs(g, p);
    let budget = g.supply_budget(p, CaptainAura::Vanguard);
    let ours: Vec<PlotIdx> = g.cities.values().filter(|c| c.owner == p).map(|c| c.plot).collect();
    let mut best: Option<(i32, PlayerId)> = None;
    for o in g.players.iter().filter(|o| o.alive && !o.is_neutral && o.id != p && !g.at_war(p, o.id)) {
        if g.check_war(p, o.id).is_err() {
            continue;
        }
        // No second front on a civilization already at war with another (King Cinder's Host
        // aside): piling on finished the weak in a few turns, before any peace could be made.
        if civ_wars(g, o.id).iter().any(|&x| g.players[x.idx()].faction != FactionId::Infernals) {
            continue;
        }
        // Good civilizations do not start wars on Good ones before the Blowout.
        if me.alignment == Alignment::Good && o.alignment == Alignment::Good && !g.apocalypse.carcosa_risen {
            continue;
        }
        // Their city nearest to ours.
        let Some((d, plot)) = g
            .cities
            .values()
            .filter(|c| c.owner == o.id)
            .filter_map(|c| ours.iter().map(|&m| g.world.distance(m, c.plot)).min().map(|d| (d, c.plot)))
            .min()
        else {
            continue;
        };
        if d > DECLARE_REACH {
            continue;
        }
        // A squad next to it must stay supplied.
        let supplied = g.world.neighbors(plot).into_iter().any(|nb| costs[nb.idx()] <= budget);
        let def = defense_of(g, o.id, plot);
        if !supplied || free * 100 < def * edge {
            continue;
        }
        let margin = free * 100 / def.max(1);
        if best.is_none_or(|(m, _)| margin > m) {
            best = Some((margin, o.id));
        }
    }
    best.map(|(_, t)| t)
}

fn declare(g: &mut Game, p: PlayerId, target: PlayerId, events: &mut Vec<Event>) {
    if g.check_war(p, target).is_err() {
        return;
    }
    if try_cmd(g, p, Command::DeclareWar { target }, events) && g.players[target.idx()].alignment != g.players[p.idx()].alignment {
        if g.check_call_crusade(p, target).is_ok() {
            try_cmd(g, p, Command::CallCrusade { target }, events);
        }
    }
}

fn manage_war(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    // After the Blowout, a civilization goes to war with the holder of the Lifeboat its squads are
    // sent at (`military::assign_roles`): a Lifeboat for one that holds none, Caldera Keep while a
    // Furnace civilization holds it.
    if g.apocalypse.carcosa_risen {
        if let Some(owner) = assault_target_owner(g, p) {
            if !g.players[owner.idx()].is_neutral && !g.at_war(p, owner) {
                declare(g, p, owner, events);
            }
        }
    }
    // Otherwise only from peace (the Host's wars, which it starts on entering, aside), with the edge.
    let infernals = |x: PlayerId| g.players[x.idx()].faction == FactionId::Infernals;
    let at_peace = !civ_wars(g, p).into_iter().any(|e| !infernals(e) && !infernals(p));
    if at_peace {
        if let Some(t) = war_target(g, p) {
            declare(g, p, t, events);
        }
    }
}

fn manage_crusade_lending(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    // Lend a squad to an active Crusade against a rival if we have >= 3 squads.
    if g.player_squads(p).len() < 3 {
        return;
    }
    let cr = g
        .crusades
        .values()
        .find(|c| c.active && c.target != p && g.players[c.target.idx()].alignment != g.players[p.idx()].alignment)
        .map(|c| c.id);
    if let Some(cr_id) = cr {
        if let Some(sid) = g.squads.values().find(|s| s.owner == p && s.lent_by.is_none() && g.check_lend(p, cr_id, s.id).is_ok()).map(|s| s.id) {
            try_cmd(g, p, Command::LendSquadToCrusade { crusade: cr_id, squad: sid }, events);
        }
    }
}

pub(crate) fn manage_diplomacy(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    manage_peace(g, p, events);
    manage_war(g, p, events);
    manage_crusade_lending(g, p, events);
}
