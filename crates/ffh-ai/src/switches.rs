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
/// City defenders weighed by the rival civilizations: in peacetime what deters a neighbour that
/// would see us as an easy target, at war a pool shared by each city's war threat, and sorties
/// only with a strength edge (`defense.rs`).
pub static RIVAL_DEFENSE: Switch = Switch::new("RIVAL_DEFENSE");
/// Field stacks heal as More Naval AI's do: only when most of the stack is hurt and where healing
/// is good, the wounded splitting off in our cities (`military::heal`). On since round 7f, once a
/// city/field threshold mismatch was fixed (docs/AI.md).
pub static HEAL: Switch = Switch::new("HEAL");
/// A stack next to an enemy city at war takes it when it is empty or the stack clearly covers it,
/// even outside the operation (`military::opportune`).
pub static CAPTURE: Switch = Switch::new("CAPTURE");
/// A second siege operation for a committed force twice the first target's need
/// (`campaign::plan`).
pub static SECOND_OP: Switch = Switch::new("SECOND_OP");
/// Heroes and veterans below good odds attack last in a strike and do not lead an assault
/// (`campaign::strike`, `military::assault`). On since round 7f, once they counted in the force and
/// finished strikes instead of sitting them out (docs/AI.md).
pub static PRECIOUS: Switch = Switch::new("PRECIOUS");
/// Assault transports wait in port for the landing force and sail together, escorted
/// (`naval::transport`).
pub static FLEET: Switch = Switch::new("FLEET");
/// Mana nodes as an economy: casters convert our raw mana into the node type worth most, a
/// caster is trained for it, research values the node techs (`mana.rs`).
pub static MANA_NODES: Switch = Switch::new("MANA_NODES");
/// A small caster corps by role -- terraformer, city mages, war wizards -- and its training
/// (`casters.rs`).
pub static CASTER_ROLES: Switch = Switch::new("CASTER_ROLES");
/// The damage a lost fight is expected to deal counts first strikes, in the strike's simulation
/// and its sacrifices; a sacrifice (no attacker at 68%+) is the unit whose expected damage, not
/// strength, is cheapest for what it costs, and none is sent that would leave the defender almost
/// unhurt (`campaign::fight`, `campaign::strike`): Bloodpets and Moroi went at Acheron's city at
/// odds 0, turn after turn, and left him at full health (a fork of seed 26002, turn 274: docs/AI.md,
/// round 7f).
pub static STRIKE_DAMAGE: Switch = Switch::new("STRIKE_DAMAGE");
/// The strike's simulation plays the strike as it will be fought: the attackers in the order
/// the strike picks them, each against the defender the rules put up against it; a siege engine's
/// win no longer counts as a kill (`campaign::simulate_in_order`). On since round 7g.
pub static SIM_FAITHFUL: Switch = Switch::new("SIM_FAITHFUL");
/// Every attack outside a siege operation's strike -- a stack's assault, an adjacent target, a
/// hunt, an opportune capture, a landing -- is decided on the simulation of the group attack:
/// good odds for the attacker the rules send, or a grind that clears the plot at a favourable
/// trade; a city to capture when it falls (`military::field_sim`, `military::field_go`). On since
/// round 7g.
pub static FIELD_SIM: Switch = Switch::new("FIELD_SIM");
/// A siege strike starts at the odds that put the stacks in the strike phase (the adjacent ring
/// takes the city, every defender 70% sure to fall) instead of waiting next to the city for a
/// reserve (90%) (`campaign::strike`). On since round 7h.
pub static STRIKE_AT_PHASE: Switch = Switch::new("STRIKE_AT_PHASE");
/// For the conquerors (`campaign::conqueror`): `STRIKE_AT_PHASE`, and the strike phase at 60%
/// sure instead of 70% (`campaign::covers`). On since round 7h.
pub static BOLD_STRIKE: Switch = Switch::new("BOLD_STRIKE");
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
/// Threats to us seen through fog of war, never through stealth (`view::known_threat`: the danger
/// map, a city's local threat, sorties, peril, the front; docs/AI.md, "Fog of war").
pub static FOG_SIGHT: Switch = Switch::new("FOG_SIGHT");
/// Attacks planned through fog of war, never through stealth (`view::sees_units`: the war gate,
/// siege and amphibious garrisons, hunting targets, captures; docs/AI.md, "Fog of war"). Off:
/// only what is visible, the fog estimated (the war gate: the best defender the owner can train).
pub static FOG_ATTACK: Switch = Switch::new("FOG_ATTACK");
/// Production re-decided when the empire is in financial trouble: a long build gives way to the
/// economy building the city now needs (`production::trouble_rebuild`).
pub static TROUBLE_REBUILD: Switch = Switch::new("TROUBLE_REBUILD");
/// Building effects the rules implement in code valued by `code_effects` (the Planar Gate's
/// creatures, the Cave of Ancestors' experience...); the Warrens, valued before, stay on.
pub static CODE_EFFECTS: Switch = Switch::new("CODE_EFFECTS");
/// A mana node type is also worth the buildings that need the mana (`mana::node_value`).
pub static MANA_BUILDINGS: Switch = Switch::new("MANA_BUILDINGS");
/// A unit we disband in a Demon's Altar city is sacrificed for research first (`unflagged.rs`).
pub static SACRIFICE: Switch = Switch::new("SACRIFICE");
/// The caster school: an Adept works at home until it is a Mage, casters cross dangerous ground
/// only where they are sheltered, the Mage's tech and upgrade come sooner (`magic.rs`, "The
/// caster school").
pub static CASTER_SCHOOL: Switch = Switch::new("CASTER_SCHOOL");
/// Casters march in the group of the stack they go to war with, and attack only at FfH's war
/// wizard odds (95%): not in a strike, not in a stack's assault, not counted in its force
/// (`magic::spared_caster`, `magic::join_stack`). Needs `CASTER_SCHOOL`.
pub static CASTER_STACKS: Switch = Switch::new("CASTER_STACKS");
/// Every unit takes its promotions and upgrades at the start of its player's turn (C++
/// `CvPlayerAI::AI_doTurnUnitsPost` promotes every unit), not only a group's head before it acts
/// and units that do not move: a unit inside a marching stack waited for its promotions until a
/// strike took it out of the stack (`units::play`).
pub static PROMOTE_ALL: Switch = Switch::new("PROMOTE_ALL");
/// A Summoner without a mana whose Mage-level spell summons a creature goes for one first: its
/// node, research toward it, trade, the sphere's first promotion (`traits::summon_mana`).
pub static SUMMON_MANA: Switch = Switch::new("SUMMON_MANA");
/// A magic leader (Arcane or Summoner) researches toward its Adepts early and toward Sorcery after
/// Writing (`tech_plan`, `traits::arcane_path`).
pub static ARCANE_PATH: Switch = Switch::new("ARCANE_PATH");
/// Casters cast their damage and temporary summons at the start of our turn, before our units
/// decide their attacks (`casting::barrage`); a summon's estimated fight counts the defender's
/// resistances to its damage types (`magic::summon_fight`).
pub static BARRAGE: Switch = Switch::new("BARRAGE");
/// An operation's force estimates count the casts of the casters marching with its stacks: the
/// defenders they soften before the strike (`magic::planned_soften`, `campaign::covers_with`).
pub static CASTER_FORCE: Switch = Switch::new("CASTER_FORCE");
/// A magic leader (Arcane or Summoner) keeps a caster for every three field units, and its war
/// wizards count toward its army target (`traits::mage_army`).
pub static MAGE_ARMY: Switch = Switch::new("MAGE_ARMY");
/// A city mage in a quiet city goes to a threatened city of ours without a mage within six turns
/// (`casters::city_mage`).
pub static MAGE_FRONT: Switch = Switch::new("MAGE_FRONT");

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
