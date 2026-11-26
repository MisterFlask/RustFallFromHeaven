//! Literal Supply Lines traced over the map (roads = 1, own land = 1, open ground = 3).
//!
//! Enemy squads and manned Forts block supply lines. A squad outside its own borders without a
//! supply line within budget suffers 2 turns of Hunger (-15% damage) then Starvation (-35% damage
//! and -10% HP/turn).

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use crate::ids::{PlayerId, PlotIdx};
use crate::world::{Improvement, World};

pub const BASE_SUPPLY_BUDGET: i32 = 18;
pub const SUPPLY_COST_ROAD: i32 = 1;
pub const SUPPLY_COST_OWN: i32 = 1;
pub const SUPPLY_COST_WILD: i32 = 3;

#[derive(Clone, Debug)]
pub struct SupplyLine {
    pub path: Vec<PlotIdx>,
    pub cost: i32,
    pub source: PlotIdx,
}

/// Traces the cheapest supply line from `start` back to any unbesieged own city within `budget`.
/// Returns `None` if cut.
pub fn trace_supply_line(
    world: &World,
    owner: PlayerId,
    start: PlotIdx,
    budget: i32,
    forest_supply: bool,
    own_sources: &BTreeSet<PlotIdx>,
    enemy_plots: &BTreeSet<PlotIdx>,
    enemy_fort_zoc: &BTreeSet<PlotIdx>,
) -> Option<SupplyLine> {
    if own_sources.contains(&start) {
        return Some(SupplyLine { path: vec![start], cost: 0, source: start });
    }
    let n = world.len();
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None::<PlotIdx>; n];
    let mut pq: BinaryHeap<Reverse<(i32, PlotIdx)>> = BinaryHeap::new();

    dist[start.idx()] = 0;
    pq.push(Reverse((0, start)));

    while let Some(Reverse((d, u))) = pq.pop() {
        if d > dist[u.idx()] || d > budget {
            continue;
        }
        if own_sources.contains(&u) {
            let mut path = vec![u];
            let mut cur = u;
            while let Some(p) = prev[cur.idx()] {
                path.push(p);
                cur = p;
            }
            path.reverse();
            return Some(SupplyLine { path, cost: d, source: u });
        }
        for nb in world.neighbors(u) {
            let pl = world.plot(nb);
            if !pl.kind.is_passable_land() {
                continue;
            }
            if nb != start && (enemy_plots.contains(&nb) || enemy_fort_zoc.contains(&nb)) {
                continue;
            }
            let step = if pl.road {
                SUPPLY_COST_ROAD
            } else if pl.owner == Some(owner) || (forest_supply && pl.feature.is_woods()) {
                SUPPLY_COST_OWN
            } else {
                SUPPLY_COST_WILD
            };
            let nd = d + step;
            if nd <= budget && nd < dist[nb.idx()] {
                dist[nb.idx()] = nd;
                prev[nb.idx()] = Some(u);
                pq.push(Reverse((nd, nb)));
            }
        }
    }
    None
}

/// Computes plots blocked by enemy squads and the 1-tile Zone of Control around manned enemy Forts.
pub fn enemy_supply_blocks(world: &World, enemy_squad_plots: &[PlotIdx]) -> (BTreeSet<PlotIdx>, BTreeSet<PlotIdx>) {
    let mut enemy_plots = BTreeSet::new();
    let mut fort_zoc = BTreeSet::new();
    for &p in enemy_squad_plots {
        enemy_plots.insert(p);
        if world.plot(p).improvement == Improvement::Fort {
            for nb in world.neighbors(p) {
                fort_zoc.insert(nb);
            }
        }
    }
    (enemy_plots, fort_zoc)
}

/// Starvation combat damage percentage modifier (`100` = normal, `85` = Hungry, `65` = Starving).
pub fn supply_combat_pct(unsupplied_turns: i32) -> i32 {
    if unsupplied_turns <= 0 {
        100
    } else if unsupplied_turns <= 2 {
        85
    } else {
        65
    }
}
