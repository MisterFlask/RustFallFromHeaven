//! Engagement combat (this game's own rules, not FFH2's: its land combat, not an option since
//! 2026-11-03; phase 6 of the owner's spec "Engagement combat"; docs/COMBAT_INVENTORY.md for the
//! owner's answers Q1-Q13 and the ground rules G1-G5).
//!
//! **Formation battles** (revision 4, the owner's Ogre Battle-style engagements; docs/ENGAGEMENTS.md):
//! when an army attacks a land plot, each side's units stand on a grid of three rows by its lanes
//! and act by row, role and speed for three rounds; the plot falls only when no defender is left
//! on it. Their pure resolver is [`formation`] ([`formation::resolve`]: the fighters on their
//! grids, the ground, the rounds and passes, the events and the end state; the rules' auto
//! formation, [`formation::auto_arrange`]), which knows nothing of `Game`.
//!
//! On `Game`: the dispatch rule (which attacks are engagements, [`state`]), the
//! battle reports with the battle log the client replays ([`report`]), the units' roles, HP and A
//! and Dv per hit ([`stats`]), each side's slots ([`slots`]), who can fight ([`pick`]), where each
//! side's units stand (the auto formation and an ordered formation's check: [`arrange`]), the
//! engagement itself (`engage`, `apply`, `sink`), and the player's attack preview, sampled on a
//! private RNG ([`preview`]). Sieges and the walls' Sentinels, which fight in the front row of a
//! city's battles, are [`siege`] (docs/SIEGES.md).

mod apply;
mod arms;
pub mod arrange;
mod battle_magic;
pub(crate) mod engage;
pub mod formation;
pub mod pick;
pub mod preview;
pub mod report;
pub mod siege;
mod sink;
pub mod slots;
pub mod state;
pub mod stats;

pub use arrange::Place;
pub use engage::{damage_from_hp, hp_from_damage, BattleEffect, EngageOutcome};
pub use formation::{BattleRng, FighterIdx, Ground, GroundRule, Pair, Side};
pub use pick::WarTest;
pub use preview::{EngagePreview, Forecast, PreviewFighter, PreviewInput, PreviewSentinel};
pub use report::EngageResult;
pub use slots::Slots;
pub use stats::HitCtx;
