//! Measurement switches for generic behaviours: one binary plays a behaviour for some
//! civilizations and not for others, so a change can be measured both for everyone (all on
//! against all off) and as one civilization with it against opponents without it (`civ_ab.py`
//! with `--base-env FFH_AI_X_OFF=ALL --new-env FFH_AI_X_ONLY={civ}`), which is what a land race
//! or any other contest between civilizations needs.
//!
//! `FFH_AI_<NAME>_OFF` lists civilizations (`CIVILIZATION_KHAZAD` or `KHAZAD`, comma-separated,
//! any case, or `ALL`; `1` also means every one) that play without the behaviour;
//! `FFH_AI_<NAME>_ONLY` lists the only ones that play with it (unset or empty: no such limit). A
//! switch made with [`Switch::off_by_default`] is off unless `FFH_AI_<NAME>_ONLY` lists the
//! civilization (`ALL` for every one). The table is read once and never changes during a game, so
//! play stays deterministic, and the run cache keys runs by every `FFH_*` variable.
//!
//! Every measurement switch of the AI is read here ([`env_list`]): the generic behaviours below,
//! and the switches of whole modules -- [`MAGIC`] (`FFH_AI_MAGIC_OFF`, `magic::enabled`), [`CIV`]
//! (`FFH_AI_CIV_OFF`, `civs::enabled`), [`VASSAL`] (`FFH_AI_VASSAL_OFF`, `vassals.rs`) and the
//! trait switches (`FFH_AI_TRAIT_OFF` lists traits, [`TRAIT_CIVS`] civilizations; `traits.rs`).

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
/// Vassal states (`vassals.rs`): off, a player neither seeks nor agrees to vassalage, and as the
/// master takes a vassal only when it costs nothing. (`FFH_AI_VASSAL_OFF=1`, as before the switch
/// was per civilization, is every civilization.)
pub static VASSAL: Switch = Switch::new("VASSAL");
/// The civilizations whose players play their leaders' traits (`traits.rs`):
/// `FFH_AI_TRAIT_CIVS` lists them (unset: every one); the traits themselves are turned off by
/// `FFH_AI_TRAIT_OFF` ([`env_list`], trait names).
pub static TRAIT_CIVS: Switch = Switch::with_vars("TRAIT_CIVS", None, Some("FFH_AI_TRAIT_CIVS"));

// Generic behaviours.

/// Expert moments for the world spells FfH's conditions almost never allowed (`world_spells.rs`).
pub static WS_TIMING: Switch = Switch::new("WS_TIMING");
/// A prepared war is declared once the army stands near the target (`diplomacy::manage`).
pub static WAR_STAGING: Switch = Switch::new("WAR_STAGING");
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
/// For the conquerors (`campaign::conqueror`): the strike phase at 60% sure instead of 70%
/// (`combat_sim::covers`). On since round 7h.
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
/// Cultural actions (`culture.rs`, docs/CULTURE.md): off, a civilization channels none.
pub static CULTURE: Switch = Switch::new("CULTURE");
/// Armies' marching reinforcements (`armies.rs`, docs/ARMIES.md): off, every unit stays in its
/// city's garrison when trained.
pub static ARMIES: Switch = Switch::new("ARMIES");
/// The main army's requisition (`armies::requisition_cities`, docs/ARMIES.md): off, the AI's
/// armies requisition from no city (the directly ruled cities' attackers still go to the main
/// army).
pub static REQUISITION: Switch = Switch::new("REQUISITION");
/// Governed cities (`governor.rs`, docs/GOVERNORS.md): off, the AI keeps its first cities direct
/// and every governed city develops.
pub static GOVERNORS: Switch = Switch::new("GOVERNORS");
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
/// A city under threat switches its production to a defender only when `production::decide`
/// would put a fighter first (`production::emergency`); off, it always switches (and re-picks the
/// same building when its garrison is full).
pub static EMERGENCY_DECIDE: Switch = Switch::new("EMERGENCY_DECIDE");
/// AI-to-AI deals (peace, gifts, bribes, trades, pacts) and offers to humans need both players to
/// be able to contact the other (`diplomacy::can_talk`); off, the AI-to-AI ones did not ask.
pub static CONTACT: Switch = Switch::new("CONTACT");
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
/// A magic leader (Arcane or Summoner) keeps a caster for every four field units, on top of its
/// army (`traits::mage_army`).
pub static MAGE_ARMY: Switch = Switch::new("MAGE_ARMY");
/// A city spell's building is valued against the enemies in sight of the city (`magic.rs`): a
/// Wall of Stone in a besieged city.
pub static CITY_SPELL_THREAT: Switch = Switch::new("CITY_SPELL_THREAT");
/// The war of expansion's takeability test counts our casters' damage on a city's defenders
/// (`diplomacy::can_take_cities_at`).
pub static MAGIC_WAR: Switch = Switch::new("MAGIC_WAR");
/// A caster in a marching stack of our operations hastes it when that raises the stack's pace
/// (`magic::haste_march`).
pub static HASTE_MARCH: Switch = Switch::new("HASTE_MARCH");

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
