//! Measurement switches for generic behaviours: one binary plays a behaviour for some
//! civilizations and not for others, so a change can be measured both for everyone (all on
//! against all off) and as one civilization with it against opponents without it (`civ_ab.py`
//! with `--base-env FFH_AI_X_OFF=ALL --new-env FFH_AI_X_ONLY={civ}`), which is what a land race
//! or any other contest between civilizations needs.
//!
//! `FFH_AI_<NAME>_OFF` lists civilizations (`CIVILIZATION_KHAZAD` or `KHAZAD`, comma-separated,
//! or `ALL`) that play without the behaviour; `FFH_AI_<NAME>_ONLY` lists the only ones that play
//! with it. Like `FFH_AI_CIV_OFF`, the table is read once and never changes during a game, so play
//! stays deterministic, and the run cache keys runs by every `FFH_*` variable.

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::{GameData, Idx};

pub struct Switch {
    name: &'static str,
    table: OnceLock<Vec<bool>>,
}

/// Expert moments for the world spells FfH's conditions almost never allowed (`world_spells.rs`).
pub static WS_TIMING: Switch = Switch::new("WS_TIMING");
/// A prepared war is declared once the army stands near the target (`diplomacy::manage`).
pub static WAR_STAGING: Switch = Switch::new("WAR_STAGING");
/// Peace weighs the forces at the front: our army near their next city against theirs near ours
/// (`diplomacy::forces_pct`).
pub static PEACE_FORCES: Switch = Switch::new("PEACE_FORCES");
/// A war where nothing happens ends (`diplomacy::quiet_war`).
pub static QUIET_PEACE: Switch = Switch::new("QUIET_PEACE");
/// At peace, the power of a target's current enemies counts toward our edge (`diplomacy::dogpile_power`).
pub static DOGPILE: Switch = Switch::new("DOGPILE");
/// Tech-trade discipline (`diplomacy::tech_denial`): no swaps with the victory rival or a clear
/// world leader, no military techs while we prepare or fight a war, keep monopolies.
pub static TECH_DISCIPLINE: Switch = Switch::new("TECH_DISCIPLINE");
/// A prepared war is re-checked against the selection gate before it is declared (`diplomacy::manage`).
pub static WAR_RECHECK: Switch = Switch::new("WAR_RECHECK");
/// Council votes and the secretary's resolutions (`council.rs`); off = abstain, propose nothing.
pub static COUNCIL: Switch = Switch::new("COUNCIL");
/// Missionaries spread the state religion abroad once its shrine stands (`units::missionary`).
pub static MISSIONARY_ABROAD: Switch = Switch::new("MISSIONARY_ABROAD");

fn civ_list(var: &str) -> Option<Vec<String>> {
    let v = std::env::var(var).ok()?;
    Some(v.split(',').map(|s| s.trim().trim_start_matches("CIVILIZATION_").to_ascii_uppercase()).filter(|s| !s.is_empty()).collect())
}

impl Switch {
    pub const fn new(name: &'static str) -> Self {
        Switch { name, table: OnceLock::new() }
    }

    fn table(&self, d: &GameData) -> &[bool] {
        self.table.get_or_init(|| {
            let off = civ_list(&format!("FFH_AI_{}_OFF", self.name)).unwrap_or_default();
            let only = civ_list(&format!("FFH_AI_{}_ONLY", self.name));
            d.civilizations
                .keys()
                .map(|c| {
                    let name = d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_");
                    let listed = |l: &[String]| l.iter().any(|o| o == "ALL" || o == name);
                    !listed(&off) && only.as_deref().map_or(true, listed)
                })
                .collect()
        })
    }

    /// Does player `p` play with the behaviour? (Players without a civilization: yes.)
    pub fn on(&self, g: &Game, p: PlayerId) -> bool {
        g.player_get_civilization_type(p).map_or(true, |c| self.table(g.d)[c.idx()])
    }
}
