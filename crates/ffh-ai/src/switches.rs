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
/// Mana nodes as an economy: casters convert our raw mana into the node type worth most, a
/// caster is trained for it, research values the node techs (`mana.rs`).
pub static MANA_NODES: Switch = Switch::new("MANA_NODES");
/// A small caster corps by role -- terraformer, city mages, war wizards -- and its training
/// (`casters.rs`).
pub static CASTER_ROLES: Switch = Switch::new("CASTER_ROLES");
/// A temporary summon on its last turn attacks at any odds when it is not its stack's cover
/// (`casting::summon_target`).
pub static SUMMON_SPEND: Switch = Switch::new("SUMMON_SPEND");
/// Govannon walks to our largest stack and casts Teach Spellcasting (`casters::govannon`).
pub static TEACH_SPELLS: Switch = Switch::new("TEACH_SPELLS");
/// Sanctuary and March of the Trees when the enemies' power at war is 1.5x ours
/// (`world_spells::expert_moment`).
pub static WS_LOSING_WAR: Switch = Switch::new("WS_LOSING_WAR");

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
