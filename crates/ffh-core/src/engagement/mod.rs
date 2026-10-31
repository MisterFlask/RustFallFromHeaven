//! Engagement combat (this game's own rules, not FFH2's; `GAMEOPTION_ENGAGEMENTS`, phase 6 of the
//! owner's spec "Engagement combat"; docs/COMBAT_INVENTORY.md for the owner's answers Q1-Q13 and
//! the ground rules G1-G5).
//!
//! When an army attacks a land plot, each side commits up to S units, which fight in rounds until
//! one side's committed force is destroyed or has withdrawn. So far this module holds the **pure
//! round model**, which knows nothing of `Game`:
//! - [`Battle`] ([`battle`]): the fighters of both sides in slot order (role flags, target-class
//!   order, max HP and HP, volleys, withdrawal chance, combat limit, morale), the pair table of A
//!   and Dv, and the battle-level effects phase 7's in-battle spells will return
//!   ([`BattleEffect`]).
//! - [`targets`] and the class table on [`Battle`]: the targeting rules (class order, the shield,
//!   first-strike immunity in volleys, combat-limit skips), read by both integrators.
//! - [`resolve_sampled`] ([`resolve`]): the battle as fought, drawing from the RNG passed in
//!   ([`BattleRng`], backed by the game's sync RNG), with the per-round event log the battle
//!   report and the battle UI replay ([`BattleEvent`]).
//! - [`resolve_expected`] ([`expect`]): the estimator's integrator, the same battle in moments with
//!   no RNG, and its calibrated win chance ([`expect::win_permille`]).
//!
//! The rules (set-up, morale, defensive strikes and collateral, the post-battle steps, the
//! estimator's inputs) map units onto these types; the calibration battery that makes the win
//! table is `calibrate` (tests).

pub mod battle;
pub mod expect;
mod phi_table;
pub mod resolve;

#[cfg(test)]
mod calibrate;

pub use battle::{damage_from_hp, hp_from_damage, Battle, BattleEffect, Fighter, FighterIdx, Pair, Params, Side, TargetClass};
pub use expect::{resolve_expected, resolve_expected_from, Expectation, FighterExpect, MomentStart};
pub use resolve::{
    resolve_sampled, resolve_sampled_quiet, targets, BattleEvent, BattleLog, BattleRng, Death, EngageResult, Fate, FighterEnd, Hit, Kill, NoTarget,
    Phase, TargetState, Targets,
};
