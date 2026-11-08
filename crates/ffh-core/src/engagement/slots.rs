//! Engagement slots (plan 5.1): how many units each side of an engagement commits.
//!
//! ```text
//! S_plot    = SLOTS_BASE + the defended plot's change (GameData::engagement_slots_change)
//! n         = the defended city's Sentinels that fight (docs/SIEGES.md; 0 without walls)
//! def       = max(SLOTS_MIN, S_plot - n)   the defender's soldiers; its n Sentinels stand beside
//! att_all   = max(SLOTS_MIN, S_plot - n)
//! river     = across a river: min(SLOTS_RIVER, att_all - SLOTS_MIN), else 0
//! att       = att_all - river      the attacker's ordinary slots
//! att_river = river                slots only units that cross rivers freely (bRiver) fill (Q9)
//! ```
//!
//! Open flat land gives 10 a side, forest or hills 6, forested hills 4. A city's defence costs no
//! slots (it grows its Sentinels instead); its Sentinels cost both sides one slot each, and the
//! defender's Sentinels fill its cells ([`Slots::sentinels`]).
//!
//! [`Slots`] carries each side's count with its reasons in the battle log's form
//! ([`SideSlots`]): the base, then the changes in the order applied (the plot's, the Sentinels',
//! the minimum, the river), so `base + the deltas = total`.

use super::formation::Side;
use super::report::{SideSlots, SlotChange, SlotSource};
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;

/// Each side's slots in an engagement (plan 5.1): `[attacker, defender]` with their reasons
/// (the battle log's [`SideSlots`]). The defender has no river slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slots {
    pub sides: [SideSlots; 2],
    /// The defender's Sentinels: cells of its grid beside its soldiers' slots (docs/SIEGES.md).
    pub sentinels: u8,
    /// Card: the defender's Sentinels from the field (a Bastion's, a Cairn's: docs/CARDS.md;
    /// `more_cards.rs`), cells of its grid after the city's, at no slot; and the percent their
    /// values grow by (the plot's improvement's defence). Set by the set-up, from the defenders.
    pub field_sentinels: u8,
    pub field_grow: i32,
}

impl Slots {
    /// The attacker's ordinary slots (any unit fills them).
    #[inline]
    pub fn att(&self) -> i32 {
        self.sides[0].total
    }

    /// The attacker's river slots: only units that cross rivers freely (`bRiver`) fill them (Q9).
    #[inline]
    pub fn att_river(&self) -> i32 {
        self.sides[0].river_only
    }

    /// The defender's slots.
    #[inline]
    pub fn def(&self) -> i32 {
        self.sides[1].total
    }

    /// The most units side `side` commits: its ordinary and river slots.
    #[inline]
    pub fn max(&self, side: Side) -> i32 {
        let s = &self.sides[side.index()];
        s.total + s.river_only
    }

    /// The side's ordinary slots: at most this many of its units without `bRiver`.
    #[inline]
    pub fn ordinary(&self, side: Side) -> i32 {
        self.sides[side.index()].total
    }
}

impl Game {
    /// Each side's slots for an attack from `ctx.from` on `ctx.to` (module comment). With no
    /// approach plot there is no river crossing.
    pub fn engage_slots(&self, ctx: &HitCtx) -> Slots {
        let d = self.d;
        let defs = &d.defines;
        let base = defs.engagement_slots_base;
        let min = defs.engagement_slots_min;
        let to = ctx.to;

        let plot_changes = self.plot_engage_slot_changes(to);
        let s_plot = base + plot_changes.iter().map(|c| c.delta).sum::<i32>();

        // The city's Sentinels: one slot of each side each.
        let n = self.engage_city_sentinels(to).map_or(0, |(_, ks)| ks.len()) as i32;
        let mut plot_changes = plot_changes;
        if n > 0 {
            plot_changes.push(SlotChange { src: SlotSource::Sentinels(n), delta: -n });
        }
        let s_side = s_plot - n;

        // The defender.
        let mut def_changes = plot_changes.clone();
        let def = min.max(s_side);
        if def != s_side {
            def_changes.push(SlotChange { src: SlotSource::Minimum, delta: def - s_side });
        }

        // The attacker: the minimum, then the river.
        let mut att_changes = plot_changes;
        let mut att_all = s_side;
        // Card: Trench (docs/CARDS.md): the defended plot's improvement narrows the attacker.
        if let Some(c) = self.engage_arms_attacker_slots(to) {
            att_all += c.delta;
            att_changes.push(c);
        }
        if att_all < min {
            att_changes.push(SlotChange { src: SlotSource::Minimum, delta: min - att_all });
            att_all = min;
        }
        let crossing = ctx.from.is_some_and(|from| self.plot_is_river_crossing(from, self.direction_xy_plots(from, to)));
        let river = if crossing { defs.engagement_slots_river.min(att_all - min).max(0) } else { 0 };
        if crossing {
            att_changes.push(SlotChange { src: SlotSource::River, delta: -river });
        }

        Slots {
            sides: [
                SideSlots { base, changes: att_changes, total: att_all - river, river_only: river },
                SideSlots { base, changes: def_changes, total: def, river_only: 0 },
            ],
            sentinels: n as u8,
            field_sentinels: 0,
            field_grow: 0,
        }
    }

    /// The defended plot's changes to the slots of each side (plan 2.4,
    /// `GameData::engagement_slots_change`): hills, then the feature's change where it is
    /// non-zero, else, on a flat plot, the terrain's. Zero changes are left out.
    fn plot_engage_slot_changes(&self, p: PlotIdx) -> Vec<SlotChange> {
        let d = self.d;
        let plot = &self.map.plots[p];
        let hills = self.plot_is_hills(p);
        let mut changes = Vec::new();
        if hills && d.defines.engagement_slots_hills != 0 {
            changes.push(SlotChange { src: SlotSource::Hills, delta: d.defines.engagement_slots_hills });
        }
        let feature = plot.feature_type.filter(|&f| d.features[f].engagement_slots_change != 0);
        if let Some(f) = feature {
            changes.push(SlotChange { src: SlotSource::Feature(f), delta: d.features[f].engagement_slots_change });
        } else if !hills {
            if let Some(t) = plot.terrain_type.filter(|&t| d.terrains[t].engagement_slots_change != 0) {
                changes.push(SlotChange { src: SlotSource::Terrain(t), delta: d.terrains[t].engagement_slots_change });
            }
        }
        debug_assert!(plot.terrain_type.map_or(true, |t| changes.iter().map(|c| c.delta).sum::<i32>() == d.engagement_slots_change(t, plot.feature_type, hills)));
        changes
    }
}
