//! Where each side's units stand (spec 1.1 and 1.8): the lanes of a side, the rules' auto
//! formation, and the check of a formation the attacker orders.
//!
//! **The grid.** Each side has `max(2, ceil((slots + 2) / 3))` lanes (its ordinary and river slots
//! together: [`Game::engage_lanes`]) by 3 rows (front 0, middle 1, rear 2). A [`Place`] puts a unit
//! in one cell; a large unit (`bLarge`) fills its lane and the next in that row and counts 2
//! slots.
//!
//! **The auto formation** ([`Game::engage_auto_formation`]; `formation::auto_arrange`, the
//! prototype's `autoArrange`): one unit of each kind first (the action of the row it prefers:
//! `formation::kind_pick_order`), in the order anti-flanking strikers, strikers, shooters,
//! chargers, casters, healers, bombards, stabbers, then the rest in that order; within a kind the
//! healthy units first (not below `ENGAGEMENT_AUTO_MIN_HP_PERCENT`), then the strongest (its A on
//! its side against nobody in particular, with its promotions; a large unit's counts double), then
//! the lower unit id; while the slots last (a large unit needs 2; units without `bRiver` at most
//! the ordinary slots, Q9). Then the large units are placed first, each unit in the first row of
//! its rank preference (`engage_row_order`) with room, the lanes nearest the centre first. A defended city's Sentinels (docs/SIEGES.md) hold their cells first: the front
//! row, the lanes nearest the centre ([`Game::engage_sentinel_cells`]), and the defender's units
//! take the cells left. Deterministic, no RNG, the same for humans and the AI. Heroes are not held
//! back. The attacker fights it when its order gives no formation; the defender always does
//! ([`Game::engage_defense_formation`]).
//!
//! **Tactics** (docs/TACTICS.md): each side's auto formation is arranged by the side's battle tactic
//! ([`Game::engage_side_tactic`]: the tactic of the army, or of the city's garrison, most of the
//! side's units belong to; Line, the auto formation above, for units of no army or garrison and
//! for the barbarians). A tactic changes the selection and the placement
//! (`formation::auto_arrange_around`'s knobs, `battle_tactics.jsonc`).
//!
//! **An ordered formation** ([`Game::engage_validate_formation`]): every unit in the attacker's
//! pool once, inside its side's lanes and rows, no two units in one cell (a large unit on two free
//! cells of one row), at most the attacker's slots, and the units without `bRiver` at most its
//! ordinary slots.

use super::formation::{auto_arrange_around, kind_pick_order, lanes_for, sentinel_cells, Candidate, Side};
use ffh_data::BattleTacticType;
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
    /// defender's cells counting its Sentinels (card: the field's too, `more_cards.rs`).
    pub fn engage_lanes(&self, slots: &Slots) -> [u8; 2] {
        [lanes_for(slots.max(Side::Attacker)), lanes_for(slots.max(Side::Defender) + slots.sentinels as i32 + slots.field_sentinels as i32)]
    }

    /// The cells of the defender's Sentinels (docs/SIEGES.md), in the order the city's standing
    /// Sentinels take them, then (card: `more_cards.rs`) the field's.
    pub fn engage_sentinel_cells(&self, slots: &Slots) -> Vec<(u8, u8)> {
        sentinel_cells(self.engage_lanes(slots)[Side::Defender.index()], slots.sentinels as usize + slots.field_sentinels as usize)
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

    /// The battle tactic Line (`TACTIC_LINE`, docs/TACTICS.md): every army and garrison starts with
    /// it.
    pub fn tactic_line(&self) -> BattleTacticType {
        self.d.t::<BattleTacticType>("TACTIC_LINE")
    }

    /// The tactic a side made of `units` fights in on `plot` (docs/TACTICS.md, section 4): the tactic
    /// of the body most of them belong to, counting by army, the garrison units of `plot`'s city
    /// counting for the city; the largest count wins, an army before the garrison on a tie, the
    /// lower army id on a tie between armies. Units in no army and no garrison count for nobody.
    /// Line when none of them belongs to an army or the garrison, and for the barbarians.
    pub fn engage_side_tactic(&self, units: &[UnitId], plot: PlotIdx) -> BattleTacticType {
        let city = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c));
        let mut armies: Vec<(ArmyId, i32)> = Vec::new();
        let mut garrison = 0;
        for &u in units {
            if !self.units.contains(u) || self.player_is_barbarian(self.units[u].owner) {
                continue;
            }
            if let Some(a) = self.units[u].army.filter(|&a| self.armies.contains(a)) {
                match armies.iter_mut().find(|x| x.0 == a) {
                    Some(x) => x.1 += 1,
                    None => armies.push((a, 1)),
                }
            } else if city.is_some() && self.unit_garrison_city(u) == city {
                garrison += 1;
            }
        }
        // The largest army count, the lower id on a tie.
        armies.sort_by_key(|&(a, n)| (std::cmp::Reverse(n), a));
        match (armies.first(), city) {
            (Some(&(a, n)), _) if n >= garrison => self.armies[a].tactic,
            (_, Some(c)) if garrison > 0 => self.cities[c].garrison_tactic,
            _ => self.tactic_line(),
        }
    }

    /// The plot side `side` of `ctx` fights from: the defended plot, or where the attack comes
    /// from (else where the first of `pool` stands).
    fn engage_side_plot(&self, pool: &[UnitId], side: Side, ctx: &HitCtx) -> Option<PlotIdx> {
        match side {
            Side::Defender => Some(ctx.to),
            Side::Attacker => ctx.from.or_else(|| pool.first().and_then(|&u| self.unit_plot_opt(u))),
        }
    }

    /// The tactic side `side` of `ctx` made of `pool` fights in ([`Game::engage_side_tactic`] on its
    /// plot).
    pub fn engage_pool_tactic(&self, pool: &[UnitId], side: Side, ctx: &HitCtx) -> BattleTacticType {
        match self.engage_side_plot(pool, side, ctx) {
            Some(plot) => self.engage_side_tactic(pool, plot),
            None => self.tactic_line(),
        }
    }

    /// The rules' auto formation of side `side` from `pool` (module comment), in the pool's order,
    /// arranged by the side's tactic ([`Game::engage_pool_tactic`]). Units that never fight land
    /// battles (sea and air units) are left out.
    pub fn engage_auto_formation(&self, pool: &[UnitId], side: Side, slots: &Slots, ctx: &HitCtx) -> Vec<Place> {
        let tactic = self.engage_pool_tactic(pool, side, ctx);
        self.engage_tactic_formation(pool, side, slots, ctx, tactic)
    }

    /// [`Game::engage_auto_formation`] in tactic `tactic`.
    pub fn engage_tactic_formation(&self, pool: &[UnitId], side: Side, slots: &Slots, ctx: &HitCtx, tactic: BattleTacticType) -> Vec<Place> {
        let d = self.d;
        let min_hp = d.defines.engagement_auto_min_hp_percent;
        let veteran_level = d.defines.tactic_veteran_level;
        let mut units: Vec<UnitId> = Vec::with_capacity(pool.len());
        let mut cands: Vec<Candidate> = Vec::with_capacity(pool.len());
        for &u in pool {
            if !self.unit_engages_on_land(u) {
                continue;
            }
            let order = self.unit_engage_row_order(u);
            cands.push(Candidate {
                pick_order: kind_pick_order(self.unit_engage_kind(u), self.unit_engage_anti_flank(u)),
                row_order: order.map(|r| r.min(ROWS - 1) as u8),
                large: self.unit_engage_large(u),
                value: self.engage_strength(u, side, ctx) as i64,
                healthy: 100 - self.units[u].damage >= min_hp,
                river: side == Side::Defender || self.unit_is_river(u),
                veteran: self.unit_is_hero(u) || self.units[u].level >= veteran_level,
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
        let placed = auto_arrange_around(&sorted, lanes, slots.max(side), slots.ordinary(side), &blocked, &d.battle_tactics[tactic].knobs);
        let mut out: Vec<Place> = placed.into_iter().map(|p| Place { unit: units[order[p.unit]], lane: p.lane, row: p.row }).collect();
        // Card: a familiar stands in its caster's lane (battle_magic.rs).
        self.engage_familiar_places(&mut out, lanes, &blocked);
        let pos = |u: UnitId| pool.iter().position(|&x| x == u);
        out.sort_by_key(|p| pos(p.unit));
        out
    }

    /// The defender's formation on `ctx.to` from its `candidates` (spec 1.8): the rules' auto
    /// formation in the defenders' tactic (docs/TACTICS.md).
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
