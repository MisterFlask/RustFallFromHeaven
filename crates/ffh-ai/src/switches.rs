//! Measurement switches: one binary plays a behaviour for some civilizations and not for others,
//! so a change can be measured both for everyone (all on against all off) and as one civilization
//! with it against opponents without it (`civ_ab.py` with `--base-env FFH_AI_X_OFF=ALL --new-env
//! FFH_AI_X_ONLY={civ}`), which is what a land race or any other contest between civilizations
//! needs. A behaviour under measurement gets a [`Switch`]; once it is settled, the switch goes and
//! the behaviour stays (or goes) unconditionally.
//!
//! `FFH_AI_<NAME>_OFF` lists civilizations (`CIVILIZATION_KHAZAD` or `KHAZAD`, comma-separated,
//! any case, or `ALL`; `1` also means every one) that play without the behaviour;
//! `FFH_AI_<NAME>_ONLY` lists the only ones that play with it (unset or empty: no such limit). A
//! switch made with [`Switch::off_by_default`] is off unless `FFH_AI_<NAME>_ONLY` lists the
//! civilization (`ALL` for every one; the probes, `probes.rs`). The table is read once and never
//! changes during a game, so play stays deterministic, and the run cache keys runs by every
//! `FFH_*` variable.
//!
//! The switches of whole modules are read here ([`env_list`]): [`MAGIC`] (`FFH_AI_MAGIC_OFF`,
//! `magic::enabled`), [`CIV`] (`FFH_AI_CIV_OFF`, `civs::enabled`) and the trait switches
//! (`FFH_AI_TRAIT_OFF` lists traits, [`TRAIT_CIVS`] civilizations; `traits.rs`).

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::{CivilizationType, GameData, Idx};

pub struct Switch {
    name: &'static str,
    /// On unless listed in `FFH_AI_<NAME>_OFF`; when false, off unless listed in
    /// `FFH_AI_<NAME>_ONLY` (a behaviour kept for measurement whose cost is not worked out).
    default_on: bool,
    /// The variables listing the civilizations without and with the behaviour, when they are not
    /// `FFH_AI_<NAME>_OFF` and `FFH_AI_<NAME>_ONLY` (`None`: no such variable).
    vars: Option<(Option<&'static str>, Option<&'static str>)>,
    table: OnceLock<Vec<bool>>,
}

// Switches of whole modules (see the module doc).

/// This AI's spellcasting (`magic::enabled`); off, a civilization casts by FfH's terms
/// (`casting.rs`). The barbarians always do.
pub static MAGIC: Switch = Switch::new("MAGIC");
/// Civilization-specific play (`civs::enabled`): off, a civilization plays as a generic one.
pub static CIV: Switch = Switch::new("CIV");
/// The civilizations whose players play their leaders' traits (`traits.rs`):
/// `FFH_AI_TRAIT_CIVS` lists them (unset: every one); the traits themselves are turned off by
/// `FFH_AI_TRAIT_OFF` ([`env_list`], trait names).
pub static TRAIT_CIVS: Switch = Switch::with_vars("TRAIT_CIVS", None, Some("FFH_AI_TRAIT_CIVS"));

// Behaviours under measurement (removed from here once kept or dropped).

/// Attacks decided on the rules' battle forecast (`forecast.rs`): the field rule
/// (`military::field_decide`: assaults, adjacent targets, hunts, captures, landings,
/// `units::attack_adjacent`) and the strike (`campaign::strike`, the strike phase); off, on
/// `combat_sim`'s duels as in round 7g. Barbarians always play the old way.
pub static FORECAST: Switch = Switch::new("FORECAST");

/// The names environment variable `var` lists, comma-separated: trimmed, upper-cased, `prefix`
/// (`CIVILIZATION_`, `TRAIT_`) taken off. `None` when it is unset or lists nothing.
pub fn env_list(var: &str, prefix: &str) -> Option<Vec<String>> {
    let v = std::env::var(var).ok()?;
    let names: Vec<String> = v.split(',').map(|s| s.trim().to_ascii_uppercase().trim_start_matches(prefix).to_string()).filter(|s| !s.is_empty()).collect();
    (!names.is_empty()).then_some(names)
}

/// Does `list` ([`env_list`]) name `name` (without its prefix), or everything (`ALL`, `1`)?
pub fn listed(list: &[String], name: &str) -> bool {
    list.iter().any(|o| o == "ALL" || o == "1" || o == name)
}

impl Switch {
    pub const fn new(name: &'static str) -> Self {
        Switch { name, default_on: true, vars: None, table: OnceLock::new() }
    }

    /// A switch that is off unless `FFH_AI_<NAME>_ONLY` lists the civilization (or `ALL`).
    pub const fn off_by_default(name: &'static str) -> Self {
        Switch { name, default_on: false, vars: None, table: OnceLock::new() }
    }

    /// A switch (on by default) read from the variables `off` and `only` instead of
    /// `FFH_AI_<NAME>_OFF` and `FFH_AI_<NAME>_ONLY`.
    pub const fn with_vars(name: &'static str, off: Option<&'static str>, only: Option<&'static str>) -> Self {
        Switch { name, default_on: true, vars: Some((off, only)), table: OnceLock::new() }
    }

    fn table(&self, d: &GameData) -> &[bool] {
        self.table.get_or_init(|| {
            let read = |var: Option<String>| var.and_then(|v| env_list(&v, "CIVILIZATION_"));
            let (off, only) = match self.vars {
                Some((off, only)) => (read(off.map(String::from)), read(only.map(String::from))),
                None => (read(Some(format!("FFH_AI_{}_OFF", self.name))), read(Some(format!("FFH_AI_{}_ONLY", self.name)))),
            };
            let off = off.unwrap_or_default();
            d.civilizations
                .keys()
                .map(|c| {
                    let name = d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_");
                    !listed(&off, name) && only.as_deref().map_or(self.default_on, |l| listed(l, name))
                })
                .collect()
        })
    }

    /// Does civilization `c` play with the behaviour?
    pub fn civ_on(&self, d: &GameData, c: CivilizationType) -> bool {
        self.table(d)[c.idx()]
    }

    /// Does player `p` play with the behaviour? (Players without a civilization: yes.)
    pub fn on(&self, g: &Game, p: PlayerId) -> bool {
        g.player_get_civilization_type(p).map_or(self.default_on, |c| self.civ_on(g.d, c))
    }
}
