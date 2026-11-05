//! Where each side's units stand (spec 1.1 and 1.8): the lanes of a side, the rules' auto
//! formation, and the check of a formation the attacker orders.
//!
//! **The grid.** Each side has `max(2, ceil((slots + 2) / 3))` lanes (its ordinary and river slots
//! together: [`Game::engage_lanes`]) by 3 rows (front 0, middle 1, rear 2). A [`Place`] puts a unit
//! in one cell; a large unit (`bLarge`) fills its lane and the next in that row and counts 2
//! slots.
//!
//! **The auto formation** ([`Game::engage_auto_formation`]; `formation::auto_arrange`, the
//! prototype's `autoArrange`): one unit of each role first, in the roles' `iAutoPick` order
//! (Screen, Frontline, Archer, Flanker, Arcane, Divine, Siege, Assassin), then the rest in that
//! order; within a role the healthy units first (not below `ENGAGEMENT_AUTO_MIN_HP_PERCENT`),
//! then the strongest (its A on its side against nobody in particular, with its promotions; a
//! large unit's counts double), then the lower unit id; while the slots last (a large unit needs
//! 2; units without `bRiver` at most the ordinary slots, Q9). Then the large units are placed
//! first, each unit in its role's home row, the lanes nearest the centre first, else the nearest
//! row with room. A defended city's Sentinels (docs/SIEGES.md) hold their cells first: the front
//! row, the lanes nearest the centre ([`Game::engage_sentinel_cells`]), and the defender's units
//! take the cells left. Deterministic, no RNG, the same for humans and the AI. Heroes are not held
//! back. The attacker fights it when its order gives no formation; the defender always does
//! ([`Game::engage_defense_formation`]: the hook for standing formations, a later step).
//!
//! **An ordered formation** ([`Game::engage_validate_formation`]): every unit in the attacker's
//! pool once, inside its side's lanes and rows, no two units in one cell (a large unit on two free
//! cells of one row), at most the attacker's slots, and the units without `bRiver` at most its
//! ordinary slots.

use super::formation::{auto_arrange_around, lanes_for, sentinel_cells, Candidate, Side};
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use ffh_data::ROWS;

/// A unit's cell in a formation: its lane (a large unit's first) and row (0 front, 1 middle, 2
/// rear).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Place {
    pub unit: UnitId,
    pub lane: u8,
    pub row: u8,
}

impl Game {
    /// Each side's lanes (attacker, defender) for its slots: `max(2, ceil((slots + 2) / 3))`, the
    /// defender's cells counting its Sentinels.
    pub fn engage_lanes(&self, slots: &Slots) -> [u8; 2] {
        [lanes_for(slots.max(Side::Attacker)), lanes_for(slots.max(Side::Defender) + slots.sentinels as i32)]
    }

    /// The cells of the defender's Sentinels (docs/SIEGES.md), in the order the city's standing
    /// Sentinels take them.
    pub fn engage_sentinel_cells(&self, slots: &Slots) -> Vec<(u8, u8)> {
        sentinel_cells(self.engage_lanes(slots)[Side::Defender.index()], slots.sentinels as usize)
    }

    /// Whether unit `u` is large (`bLarge`: two cells, two slots, more HP).
    pub fn unit_engage_large(&self, u: UnitId) -> bool {
        self.d.units[self.units[u].unit_type].large
    }

    /// The slots unit `u` fills (2 for a large unit).
    pub fn unit_engage_size(&self, u: UnitId) -> i32 {
        1 + self.unit_engage_large(u) as i32
    }

    /// Unit `u`'s strength for the auto formation (module comment): its A on side `side` of the
    /// attack `ctx` against nobody in particular, x100.
    pub fn engage_strength(&self, u: UnitId, side: Side, ctx: &HitCtx) -> i32 {
        self.engage_a_alone(u, side, ctx)
    }

    /// The rules' auto formation of side `side` from `pool` (module comment), in the pool's order.
    /// Units without a role are left out.
    pub fn engage_auto_formation(&self, pool: &[UnitId], side: Side, slots: &Slots, ctx: &HitCtx) -> Vec<Place> {
        let d = self.d;
        let min_hp = d.defines.engagement_auto_min_hp_percent;
        let mut units: Vec<UnitId> = Vec::with_capacity(pool.len());
        let mut cands: Vec<Candidate> = Vec::with_capacity(pool.len());
        for &u in pool {
            let Some(role) = self.unit_combat_role(u) else { continue };
            let info = &d.combat_roles[role];
            cands.push(Candidate {
                pick_order: info.auto_pick,
                home_row: info.home_row.min(ROWS - 1) as u8,
                large: self.unit_engage_large(u),
                value: self.engage_strength(u, side, ctx) as i64,
                healthy: 100 - self.units[u].damage >= min_hp,
                river: side == Side::Defender || self.unit_is_river(u),
            });
            units.push(u);
        }
        // The candidates in unit id order: `auto_arrange` breaks its last ties by candidate
        // index, so equal candidates go by the lower unit id, whatever the pool's order.
        let mut order: Vec<usize> = (0..units.len()).collect();
        order.sort_by_key(|&i| units[i]);
        let sorted: Vec<Candidate> = order.iter().map(|&i| cands[i]).collect();
        let lanes = self.engage_lanes(slots)[side.index()];
        let blocked = if side == Side::Defender { self.engage_sentinel_cells(slots) } else { Vec::new() };
        let placed = auto_arrange_around(&sorted, lanes, slots.max(side), slots.ordinary(side), &blocked);
        let mut out: Vec<Place> = placed.into_iter().map(|p| Place { unit: units[order[p.unit]], lane: p.lane, row: p.row }).collect();
        let pos = |u: UnitId| pool.iter().position(|&x| x == u);
        out.sort_by_key(|p| pos(p.unit));
        out
    }

    /// The defender's formation on `ctx.to` from its `candidates` (spec 1.8): the rules' auto
    /// formation. The hook for standing formations per army (a later step).
    pub fn engage_defense_formation(&self, candidates: &[UnitId], slots: &Slots, ctx: &HitCtx) -> Vec<Place> {
        self.engage_auto_formation(candidates, Side::Defender, slots, ctx)
    }

    /// Checks a formation the attacker chose (the `engage` command's, an AI's; module comment)
    /// against its `pool` ([`Game::engage_attackers`]) and its slots. The error is the reason the
    /// UI shows.
    pub fn engage_validate_formation(&self, pool: &[UnitId], formation: &[Place], slots: &Slots) -> Result<(), &'static str> {
        if formation.is_empty() {
            return Err("no units to attack with");
        }
        let lanes = self.engage_lanes(slots)[Side::Attacker.index()];
        let mut grid = vec![[false; ROWS]; lanes as usize];
        let (mut used, mut non_river) = (0, 0);
        for (i, p) in formation.iter().enumerate() {
            if formation[..i].iter().any(|q| q.unit == p.unit) {
                return Err("a unit is in the formation twice");
            }
            if !pool.contains(&p.unit) {
                return Err("a unit in the formation cannot attack this plot");
            }
            let size = self.unit_engage_size(p.unit);
            let last = p.lane as i32 + size - 1;
            if p.row as usize >= ROWS || last >= lanes as i32 {
                return Err("a unit stands outside the battlefield");
            }
            for l in p.lane as usize..=last as usize {
                if grid[l][p.row as usize] {
                    return Err("two units stand in one cell");
                }
                grid[l][p.row as usize] = true;
            }
            used += size;
            if !self.unit_is_river(p.unit) {
                non_river += size;
            }
        }
        if used > slots.max(Side::Attacker) {
            return Err("more units than the battle has slots for");
        }
        if non_river > slots.att() {
            return Err("the river slots are only for units that cross rivers freely");
        }
        Ok(())
    }
}
