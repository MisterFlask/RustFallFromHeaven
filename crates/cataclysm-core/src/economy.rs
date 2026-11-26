//! Physical Caravans, Trade Routes, and Transparent Economy Breakdowns (`calc` tooltips).

use crate::ids::{CaravanId, CityId, PlayerId, PlotIdx};
use crate::world::{Resource, World};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct CaravanRoute {
    pub id: CaravanId,
    pub owner: PlayerId,
    pub origin: CityId,
    pub destination: CityId,
    pub resource: Resource,
    pub path: Vec<PlotIdx>,
    /// Index along `path` (moves back and forth each turn).
    pub pos_idx: usize,
    pub outbound: bool,
    /// Turns until a raided caravan re-forms (0 = active on map).
    pub reform_turns: i32,
    /// Growth bonus from undisturbed turns (0..=50%).
    pub growth_pct: i32,
}

impl CaravanRoute {
    pub fn current_plot(&self) -> Option<PlotIdx> {
        if self.reform_turns > 0 {
            return None;
        }
        self.path.get(self.pos_idx).copied()
    }

    pub fn gold_per_turn(&self, foreign: bool, origin_size: i32, dest_size: i32, extra_pct: i32) -> i32 {
        if self.reform_turns > 0 || self.path.is_empty() {
            return 0;
        }
        let len = (self.path.len() as i32).clamp(2, 20);
        let base = 2 + len / 3 + (origin_size + dest_size) / 3;
        let mut pct = 100 + self.growth_pct + extra_pct;
        if foreign {
            pct += 35;
        }
        if self.resource != Resource::None {
            pct += 40;
        }
        (base * pct / 100).max(2)
    }

    pub fn step(&mut self) {
        if self.reform_turns > 0 {
            self.reform_turns -= 1;
            return;
        }
        if self.path.len() <= 1 {
            return;
        }
        for _ in 0..3 {
            if self.outbound {
                if self.pos_idx + 1 < self.path.len() {
                    self.pos_idx += 1;
                } else {
                    self.outbound = false;
                    break;
                }
            } else if self.pos_idx > 0 {
                self.pos_idx -= 1;
            } else {
                self.outbound = true;
                break;
            }
        }
        self.growth_pct = (self.growth_pct + 2).min(50);
    }
}

/// Shortest passable path (land or coastal sea crossing) between two cities for a Caravan.
pub fn find_caravan_path(world: &World, from: PlotIdx, to: PlotIdx) -> Option<Vec<PlotIdx>> {
    if from == to {
        return None;
    }
    let n = world.len();
    let mut prev = vec![None::<PlotIdx>; n];
    let mut seen = vec![false; n];
    let mut q = VecDeque::new();
    seen[from.idx()] = true;
    q.push_back(from);
    while let Some(u) = q.pop_front() {
        if u == to {
            let mut path = vec![to];
            let mut cur = to;
            while let Some(p) = prev[cur.idx()] {
                path.push(p);
                cur = p;
            }
            path.reverse();
            return Some(path);
        }
        for nb in world.neighbors(u) {
            if seen[nb.idx()] {
                continue;
            }
            let pl = world.plot(nb);
            if !pl.kind.is_passable_land() && pl.kind != crate::world::PlotKind::Coast {
                continue;
            }
            seen[nb.idx()] = true;
            prev[nb.idx()] = Some(u);
            q.push_back(nb);
        }
    }
    None
}

/// Transparent breakdown for UI tooltips (matching the repo's "every calculated number shows its formula" rule).
#[derive(Clone, Debug, Default)]
pub struct RateBreakdown {
    pub total: i32,
    pub lines: Vec<String>,
}

impl RateBreakdown {
    pub fn add(&mut self, amount: i32, label: impl Into<String>) {
        if amount != 0 {
            self.total += amount;
            self.lines.push(format!("{}: {:+}", label.into(), amount));
        }
    }
}
