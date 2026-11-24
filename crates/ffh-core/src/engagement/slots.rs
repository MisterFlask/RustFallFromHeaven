//! Engagement slots: how many places each side of an engagement commits.
//!
//! Every battle is one squad against one squad (docs/ENGAGEMENTS.md, "Squads"): each side has
//! `ENGAGEMENT_SQUAD_PLACES` (5) places on a grid of 3 rows by 3 lanes, whatever the ground (the
//! owner's decision, 2026-11-21: frontage is gone; the ground's own rules tell the grounds apart,
//! `formation::Ground`). A defended city's standing Sentinels (docs/SIEGES.md) stand in the
//! defender's front row on top of its squad, costing no place ([`Slots::sentinels`]).
//!
//! [`Slots`] carries each side's count in the battle log's form ([`SideSlots`]).

use super::formation::Side;
use super::report::SideSlots;
use super::stats::HitCtx;
use crate::game::Game;

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
    /// The slots of a squad: `ENGAGEMENT_SQUAD_PLACES` a side, no Sentinels.
    pub fn engage_squad_slots(&self) -> Slots {
        let n = self.d.defines.engagement_squad_places;
        let side = SideSlots { base: n, changes: Vec::new(), total: n, river_only: 0 };
        Slots { sides: [side.clone(), side], sentinels: 0, field_sentinels: 0, field_grow: 0 }
    }

    /// Each side's slots for an attack on `ctx.to` (module comment): a squad's, and the defended
    /// city's standing Sentinels.
    pub fn engage_slots(&self, ctx: &HitCtx) -> Slots {
        let mut slots = self.engage_squad_slots();
        slots.sentinels = self.engage_city_sentinels(ctx.to).map_or(0, |(_, ks)| ks.len()) as u8;
        slots
    }
}
