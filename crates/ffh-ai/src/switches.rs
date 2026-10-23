//! Measurement switches for generic behaviours: one binary plays a behaviour for some
//! civilizations and not for others, so a change can be measured both for everyone (all on
//! against all off) and as one civilization with it against opponents without it (`civ_ab.py`
//! with `--base-env FFH_AI_X_OFF=ALL --new-env FFH_AI_X_ONLY={civ}`), which is what a land race
//! or any other contest between civilizations needs.
//!
//! `FFH_AI_<NAME>_OFF` lists civilizations (`CIVILIZATION_KHAZAD` or `KHAZAD`, comma-separated,
//! or `ALL`) that play without the behaviour; `FFH_AI_<NAME>_ONLY` lists the only ones that play
//! with it. A switch made with [`Switch::off_by_default`] is off unless `FFH_AI_<NAME>_ONLY` lists
//! the civilization (`ALL` for every one). Like `FFH_AI_CIV_OFF`, the table is read once and never
//! changes during a game, so play stays deterministic, and the run cache keys runs by every
//! `FFH_*` variable.

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::{GameData, Idx};

pub struct Switch {
    name: &'static str,
    /// On unless listed in `FFH_AI_<NAME>_OFF`; when false, off unless listed in
    /// `FFH_AI_<NAME>_ONLY` (a behaviour kept for measurement whose cost is not worked out).
    default_on: bool,
    table: OnceLock<Vec<bool>>,
}

/// Expert moments for the world spells FfH's conditions almost never allowed (`world_spells.rs`).
pub static WS_TIMING: Switch = Switch::new("WS_TIMING");
/// A prepared war is declared once the army stands near the target (`diplomacy::manage`).
pub static WAR_STAGING: Switch = Switch::new("WAR_STAGING");
/// City defenders weighed by the rival civilizations near each city, and sorties only with a
/// strength edge (`defense.rs`). Off by default (`FFH_AI_RIVAL_DEFENSE_ONLY=ALL` turns it on):
/// on fresh seeds it cost legacy -3.2% and research -7.2% at turn 300 (docs/AI.md, Known gaps).
pub static RIVAL_DEFENSE: Switch = Switch::off_by_default("RIVAL_DEFENSE");
/// Field stacks heal as More Naval AI's do: only when most of the stack is hurt and where healing
/// is good, the wounded splitting off in our cities (`military::heal`). Off by default
/// (`FFH_AI_HEAL_ONLY=ALL`): it cost legacy -1 to -2% on every fresh set (docs/AI.md, Known gaps).
pub static HEAL: Switch = Switch::off_by_default("HEAL");
/// A stack next to an enemy city at war takes it when it is empty or the stack clearly covers it,
/// even outside the operation (`military::opportune`).
pub static CAPTURE: Switch = Switch::new("CAPTURE");
/// A second siege operation for a committed force twice the first target's need
/// (`campaign::plan`).
pub static SECOND_OP: Switch = Switch::new("SECOND_OP");
/// Heroes and veterans attack only at good odds in a strike (`campaign::strike`). Off by default
/// (`FFH_AI_PRECIOUS_ONLY=ALL`): on fresh seeds it cost legacy -2.0% and research -5.1% at turn
/// 300 (docs/AI.md, Known gaps).
pub static PRECIOUS: Switch = Switch::off_by_default("PRECIOUS");
/// Assault transports wait in port for the landing force and sail together, escorted
/// (`naval::transport`).
pub static FLEET: Switch = Switch::new("FLEET");
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
/// Sanctuary and March of the Trees when the enemies' power at war is 1.5x ours and an invasion
/// is at the gates (`world_spells::losing_war`). Off by default (`FFH_AI_WS_LOSING_WAR_ONLY=ALL`
/// turns it on): it cost the Elohim in development (docs/AI_LOG.md, "Sanctuary and March of the
/// Trees at a losing war").
pub static WS_LOSING_WAR: Switch = Switch::off_by_default("WS_LOSING_WAR");
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
/// Built units are kept: More Naval AI's disband rule (`finance::disband_military`) instead of
/// scrapping surplus garrisons and field units for their upkeep, and no extra defender against
/// barbarians an interior city's garrison holds once units cost gold
/// (`production::garrison_holds`; docs/AI.md, "Finance").
pub static KEEP_UNITS: Switch = Switch::new("KEEP_UNITS");
/// Heroes only in the top third of our cities by production, world wonders outside it only when
/// quick (`production::top_production_city`).
pub static TOP_CITIES: Switch = Switch::new("TOP_CITIES");
/// A wonder slot in the top production cities at peace, and a production emphasis while a city
/// builds a world wonder (`production::wonder_slot`, `production::wonder_emphasis`).
pub static WONDER_SLOT: Switch = Switch::new("WONDER_SLOT");
/// Production re-decided when the empire is in financial trouble: a long build gives way to the
/// economy building the city now needs (`production::trouble_rebuild`).
pub static TROUBLE_REBUILD: Switch = Switch::new("TROUBLE_REBUILD");

fn civ_list(var: &str) -> Option<Vec<String>> {
    let v = std::env::var(var).ok()?;
    Some(v.split(',').map(|s| s.trim().trim_start_matches("CIVILIZATION_").to_ascii_uppercase()).filter(|s| !s.is_empty()).collect())
}

impl Switch {
    pub const fn new(name: &'static str) -> Self {
        Switch { name, default_on: true, table: OnceLock::new() }
    }

    /// A switch that is off unless `FFH_AI_<NAME>_ONLY` lists the civilization (or `ALL`).
    pub const fn off_by_default(name: &'static str) -> Self {
        Switch { name, default_on: false, table: OnceLock::new() }
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
                    !listed(&off) && only.as_deref().map_or(self.default_on, listed)
                })
                .collect()
        })
    }

    /// Does player `p` play with the behaviour? (Players without a civilization: yes.)
    pub fn on(&self, g: &Game, p: PlayerId) -> bool {
        g.player_get_civilization_type(p).map_or(self.default_on, |c| self.table(g.d)[c.idx()])
    }
}
