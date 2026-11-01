//! The FFH2 0.41 C++ AI (CvUnitAI, CvPlayerAI, CvCityAI, CvTeamAI, CvSelectionGroupAI,
//! CvGameAI, the AI parts of CvUnit/CvPlot/CvPlayer and the Python AI hooks of
//! CvGameUtils.py/CustomFunctions.py) ported to Rust as a [`Controller`], to compare with the
//! new AI (ffh-ai) head to head.
//!
//! Layout: every C++ AI method is a method of [`Cai`] named `<class>_ai_<snake name>` taking
//! the game and the object id first (`CvUnitAI::AI_attackMove()` ->
//! `self.unit_ai_attack_move(g, u)`); the files `unit_N.rs`, `player_N.rs`, `city_N.rs`,
//! `team_N.rs` are consecutive line ranges of the C++ files (each file's header names its
//! range), `py_N.rs` the Python. C++ members the rules do not keep live in [`state::State`]
//! inside the controller (`self.s()`).
//!
//! Differences from the DLL that are not the AI's: the AI draws its random numbers from its own
//! stream ([`Cai::rand`]) instead of the game's (so the rules' stream is the same whichever AI
//! plays), and its state is not saved.

#![allow(clippy::all)]

use std::sync::{Arc, Mutex, MutexGuard};

use ffh_core::controller::{AiHook, Controller, ControllerRef, ConqueredCityChoice};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::*;
use crate::paths::CppPaths;
use ffh_data::*;

pub mod defs;
pub mod paths;
pub mod state;

pub mod city_1;
pub mod city_2;
pub mod city_3;
pub mod city_4;
pub mod player_1;
pub mod player_2;
pub mod player_3;
pub mod player_4;
pub mod player_5;
pub mod player_6;
pub mod py_1;
pub mod py_2;
pub mod team_1;
pub mod team_2;
pub mod unit_1;
pub mod unit_2;
pub mod unit_3;
pub mod unit_4;
pub mod unit_5;
pub mod unit_6;
pub mod unit_7;
pub mod unit_8;

/// Debug tracing: `FFH_CPPAI_TRACE=all` or `=<player id>` prints C++ AI decisions to stderr
/// (read once; it only prints).
pub fn trace_on(p: PlayerId) -> bool {
    static TRACE: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
    let t = TRACE.get_or_init(|| match std::env::var("FFH_CPPAI_TRACE") {
        Ok(v) if v == "all" => Some(-1),
        Ok(v) => v.parse().ok(),
        Err(_) => None,
    });
    matches!(*t, Some(-1)) || *t == Some(p.0 as i32)
}

/// Everything the ported files need, imported with `use crate::prelude::*`.
pub mod prelude {
    pub use crate::defs::*;
    pub use crate::paths::CppPaths;
    pub use crate::state::*;
    pub use crate::{trace_on, Cai};
    pub use ffh_core::controller::{AiHook, Controller};
    pub use ffh_core::defs::*;
    pub use ffh_core::game::Game;
    pub use ffh_core::ids::*;
    pub use ffh_core::structs::*;
    pub use ffh_core::utils::*;
    pub use ffh_data::*;
}

/// The C++ AI as a controller (one per game: it keeps the AI state of every player it plays).
pub struct Cai {
    st: Mutex<state::State>,
}

impl Cai {
    pub fn new(seed: u32) -> Cai {
        Cai { st: Mutex::new(state::State::new(ffh_data::global(), seed ^ 0x5eed_c0de)) }
    }

    /// The AI state. Hold the guard for one expression only (`self.s().player(p).x = v;`,
    /// `let v = self.s().city(c).y;`): never across a call into the rules or another `Cai`
    /// method (the rules call back into the controller, and a second lock panics).
    pub fn s(&self) -> MutexGuard<'_, state::State> {
        self.st.try_lock().expect("C++ AI state locked re-entrantly (a guard was held across a call)")
    }

    /// C++ `GC.getGameINLINE().getSorenRandNum(num, log)` made by AI code: `0..num` (0 when
    /// `num <= 0`... C++ asserts num > 0 and returns 0 for 0).
    pub fn rand(&self, num: i32, log: &str) -> i32 {
        if num <= 0 {
            return 0;
        }
        self.s().rng.get_i(num, log)
    }

    /// C++ `pPlot->getFoundValue(ePlayer)` from `&Game`: the cached value, or (when the cache
    /// holds -1) the value `getFoundValue` would compute and store (`AI_foundValue`), without
    /// storing it.
    pub fn plot_found_value(&self, g: &Game, plot: PlotIdx, p: PlayerId) -> i32 {
        let v = g.map.plots[plot].found_value[p];
        if v != -1 {
            return v;
        }
        let (x, y) = g.map.xy(plot);
        g.player_ai_found_value(p, x, y, -1, true, &[]) as i16 as i32
    }

    /// `AI_init` of player `p` on first use (C++ runs it at game start): peace and espionage
    /// weights, religion and civic timers.
    fn ensure_player_init(&self, g: &mut Game, p: PlayerId) {
        let n = g.players.len();
        self.s().ensure_players(n);
        if self.s().player(p).initialized {
            return;
        }
        self.s().player(p).initialized = true;
        if let Some(leader) = g.players[p].personality_type {
            let li = &g.d.leader_heads[leader];
            let peace = li.base_peace_weight + self.rand(li.peace_weight_rand, "AI Peace Weight");
            self.player_ai_set_peace_weight(g, p, peace);
            self.player_ai_set_espionage_weight(g, p, li.espionage_weight);
            self.player_ai_set_religion_timer(g, p, 1);
            let civic = if g.players[p].max_anarchy_turns == 0 { 1 } else { 2 };
            self.player_ai_set_civic_timer(g, p, civic);
        }
    }

    /// `CvGame::updateMoves`, BETTER AI end-of-turn spellcasting (non-human players).
    fn end_turn_spells(&self, g: &mut Game, p: PlayerId) {
        g.players[p].sum_sui_mode = true;
        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            if let Some(spell) = self.unit_choose_spell(g, u) {
                g.tel_cast_reason(u, "end_turn_spells", 0);
                g.unit_cast(u, spell);
            }
            // break if casting a spell killed the unit
            if !g.units.contains(u) || g.units[u].death_delay {
                continue;
            }
            if self.unit_ai_is_buffer(g, u) {
                self.unit_ai_buff_cast(g, u);
            }
            if !g.units.contains(u) {
                continue;
            }
            if self.unit_ai_is_summoner(g, u) {
                self.unit_ai_summon_cast(g, u);
            }
        }
        g.players[p].sum_sui_mode = false;
    }

    /// `CvSelectionGroup::groupAttack`, BETTER AI spells before attacking an adjacent plot.
    fn group_attack_spells(&self, g: &mut Game, grp: GroupId) {
        for u in g.groups[grp].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            if !g.units[u].has_casted && self.unit_ai_is_direct_damage_caster(g, u) {
                self.unit_ai_direct_damage_cast(g, u, 5);
            }
            if g.units.contains(u) && !g.units[u].has_casted && self.unit_ai_is_summoner(g, u) {
                self.unit_ai_summon_cast(g, u);
            }
            if g.units.contains(u) && !g.units[u].has_casted && self.unit_ai_is_direct_damage_caster(g, u) {
                self.unit_ai_direct_damage_cast(g, u, 1);
            }
            if g.units.contains(u) && !g.units[u].has_casted && self.unit_ai_is_buffer(g, u) {
                self.unit_ai_buff_cast(g, u);
            }
            if !g.groups.contains(grp) {
                return;
            }
        }
    }
}

/// C++ `CvUnit::canCast`'s `bAllowAI` refusal for AI players, which the port's rules no longer
/// apply (AI policy: docs/PORTING.md, "Changes from FFH2"); the C++ AI keeps it.
pub(crate) fn ai_can_cast(g: &Game, u: UnitId, spell: SpellType, test_visible: bool) -> bool {
    g.d.spells[spell].allow_ai && g.unit_can_cast(u, spell, test_visible)
}

impl Controller for Cai {
    fn name(&self) -> &str {
        "cppai"
    }

    fn play_turn(&self, g: &mut Game, p: PlayerId) {
        self.ensure_player_init(g, p);
        if trace_on(p) {
            let sites: Vec<(i32, i32, i32)> = self.s().player(p).ai_city_sites.clone().into_iter().map(|pl| {
                let (x, y) = g.map.xy(pl);
                (x, y, g.map.plots[pl].found_value[p])
            }).collect();
            let settlers: Vec<String> = g.players[p].units.iter().filter(|&&u| std::env::var("FFH_CPPAI_TRACE_UNITS").is_ok() || g.units[u].ai_type == Some(UnitAIType::UNITAI_SETTLE)).map(|&u| {
                let gr = g.units[u].group;
                format!("{} ai{:?} u{}@({},{}) miss={:?} flag={} grp_n={} act={:?} moves={}", g.d.units[g.units[u].unit_type].base.type_, g.units[u].ai_type.map(|a| a.0), u.0, g.units[u].x, g.units[u].y, gr.map(|gr| g.groups[gr].ai_mission_ai_type), g.units[u].ai_groupflag, gr.map_or(0, |gr| g.groups[gr].units.len()), gr.map(|gr| g.groups[gr].activity_type), g.units[u].moves)
            }).collect();
            eprintln!("[cppai t{} p{}] groups {:?} cycle {:?}", g.game_turn(), p.0, g.players[p].groups, g.players[p].group_cycle);
            if let Some(&u0) = g.players[p].units.first() {
                let here = g.unit_plot(u0);
                let (x, y) = g.map.xy(here);
                let mut res = Vec::new();
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (2, 2), (-2, 2)] {
                    if let Some(t) = g.map.plot_xy(x + dx, y + dy) {
                        let mut turns = 0;
                        let ok = g.unit_generate_path(u0, t, 0, false, Some(&mut turns));
                        res.push((dx, dy, ok, turns, self.unit_ai_plot_valid(g, u0, t), g.map.plots[t].area.map(|a| a.0), g.plot_is_water(t), g.map.plots[t].plot_type));
                    }
                }
                eprintln!("[cppai] u{} at ({},{}) area {:?} type {} moves {} maxmoves {}", u0.0, x, y, g.map.plots[here].area, g.d.units[g.units[u0].unit_type].base.type_, g.units[u0].moves, g.unit_max_moves(u0));
                eprintln!("[cppai t{} p{}] path probe u{} {:?}", g.game_turn(), p.0, u0.0, res);
            }
            for c in g.players[p].cities.clone() {
                let (bb, need, have) = {
                    let mut st = self.s();
                    let ca = st.city(c);
                    (ca.best_build.iter().filter(|b| b.is_some()).count(), ca.workers_needed, ca.workers_have)
                };
                eprintln!("[cppai t{} p{}] city {} best_builds {} workers needed {} have {} danger {} threatened {}", g.game_turn(), p.0, c.0, bb, need, have,
                    self.player_ai_get_plot_danger(g, p, g.city_plot(c), -1, true), self.player_ai_is_plot_threatened(g, p, g.city_plot(c), 3, true));
            }
            eprintln!("[cppai t{} p{}] play_turn: cities {} sites {:?} settlers {:?} gold {} research {:?}", g.game_turn(), p.0,
                g.players[p].cities.len(), sites, settlers, g.players[p].gold, g.player_get_current_research(p));
        }
        self.player_ai_unit_update(g, p);
        if g.telemetry.on() {
            // The strategy flags this turn's decisions used (the cached hash: computing it here
            // could cache a value the AI would not have used).
            let (hash, turn) = {
                let mut s = self.s();
                let pa = s.player(p);
                (pa.strategy_hash, pa.strategy_hash_cache_turn)
            };
            if turn == g.game_turn() {
                g.tel_intent(p, "strategy", || strategy_names(hash));
            }
        }
    }

    fn choose_production(&self, g: &mut Game, c: CityId) {
        self.city_ai_choose_production(g, c);
        if g.telemetry.on() && g.cities.contains(c) {
            if let Some(o) = g.cities[c].order_queue.first() {
                let d = g.d;
                let (what, why): (&str, &'static str) = match o.order_type {
                    OrderType::ORDER_TRAIN => (
                        d.units[UnitType(o.data1 as _)].base.type_.as_str(),
                        UnitAIType::NAMES.get(o.data2 as usize).copied().unwrap_or("UNITAI_NONE"),
                    ),
                    OrderType::ORDER_CONSTRUCT => (d.buildings[BuildingType(o.data1 as _)].base.type_.as_str(), "building"),
                    OrderType::ORDER_CREATE => (d.projects[ProjectType(o.data1 as _)].base.type_.as_str(), "project"),
                    _ => (d.processes[ProcessType(o.data1 as _)].base.type_.as_str(), "process"),
                };
                g.tel_decision(g.cities[c].owner, "production", what, why, 0, String::new);
            }
        }
        if g.cities.contains(c) && trace_on(g.cities[c].owner) {
            let order = g.cities[c].order_queue.first().map(|o| match o.order_type {
                OrderType::ORDER_TRAIN => format!("train {} ai {}", g.d.units[UnitType(o.data1 as _)].base.type_, o.data2),
                OrderType::ORDER_CONSTRUCT => format!("build {}", g.d.buildings[BuildingType(o.data1 as _)].base.type_),
                OrderType::ORDER_CREATE => format!("create {}", o.data1),
                _ => format!("maintain {}", o.data1),
            });
            eprintln!("[cppai t{} p{}] city {} pop {} chooses {:?}", g.game_turn(), g.cities[c].owner.0, c.0, g.cities[c].population, order);
        }
    }

    fn choose_research(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        let t = self.player_ai_choose_research(g, p);
        if let Some(t) = t {
            g.tel_decision(p, "research", &g.d.techs[t].base.type_, "", 0, String::new);
        }
        t
    }

    fn ai_choose_research(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        self.choose_research(g, p)
    }

    fn choose_free_tech(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        self.player_ai_choose_free_tech(g, p)
    }

    fn raze_conquered_city(&self, g: &Game, p: PlayerId, c: CityId) -> bool {
        self.player_ai_conquer_city(g, p, c)
    }

    fn conquered_city_choice(&self, g: &Game, p: PlayerId, c: CityId, can_raze: bool, _gift_to: Option<PlayerId>) -> ConqueredCityChoice {
        if can_raze && self.raze_conquered_city(g, p, c) {
            ConqueredCityChoice::Raze
        } else {
            ConqueredCityChoice::Keep
        }
    }

    fn choose_event(&self, g: &Game, p: PlayerId, t: &EventTriggeredData, options: &[EventType]) -> Option<EventType> {
        match self.player_ai_choose_event(g, p, t) {
            Some(e) if options.contains(&e) => Some(e),
            _ => options.first().copied(),
        }
    }

    fn choose_election(&self, g: &Game, p: PlayerId, vs: &VoteSelectionData) -> Option<usize> {
        let t = g.players[p].team;
        let i = self.team_ai_choose_election(g, t, vs);
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    }

    fn diplo_vote(&self, g: &Game, p: PlayerId, vote: &VoteSelectionSubData, source: VoteSourceType) -> PlayerVote {
        self.player_ai_diplo_vote(g, p, vote, source, false)
    }

    fn keep_captured_unit(&self, g: &Game, p: PlayerId, unit: UnitType, plot: PlotIdx) -> bool {
        self.player_ai_capture_unit(g, p, unit, plot)
    }

    fn accept_unit(&self, g: &Game, p: PlayerId, unit: UnitId) -> bool {
        self.player_ai_accept_unit(g, p, unit)
    }

    fn consider_offer(&self, g: &Game, p: PlayerId, from: PlayerId, they_give: &[TradeData], we_give: &[TradeData]) -> bool {
        self.player_ai_consider_offer(g, p, from, they_give, we_give, 1)
    }

    fn choose_religion(&self, g: &Game, p: PlayerId) -> Option<ReligionType> {
        self.player_ai_choose_religion(g, p)
    }

    fn attitude(&self, g: &Game, p: PlayerId, towards: PlayerId) -> AttitudeType {
        self.player_ai_get_attitude(g, p, towards, false)
    }

    fn hook(&self, g: &mut Game, h: AiHook) {
        match h {
            AiHook::PlayerTurnSephi(p) => {
                self.ensure_player_init(g, p);
                for c in g.players[p].cities.clone() {
                    if g.cities.contains(c) {
                        self.city_ai_calculate_needed_perm_defense(g, c);
                        self.city_ai_calculate_needed_patrol(g, c);
                        self.city_ai_calculate_needed_city_defense_production(g, c);
                        self.city_ai_calculate_needed_city_patrol_production(g, c);
                    }
                }
                g.player_ai_do_favorite_religion(p);
                g.player_ai_do_tower_mastery(p);
            }
            AiHook::PlayerTurnPre(p) => {
                self.ensure_player_init(g, p);
                self.player_ai_do_turn_pre(g, p);
            }
            AiHook::PlayerTurnPost(p) => self.player_ai_do_turn_post(g, p),
            AiHook::PlayerTurnUnitsPre(p) => self.player_ai_do_turn_units_pre(g, p),
            AiHook::PlayerTurnUnitsPost(p) => self.player_ai_do_turn_units_post(g, p),
            AiHook::CityTurn(c) => {
                if trace_on(g.cities[c].owner) {
                    eprintln!("[cppai t{}] city turn {}", g.game_turn(), c.0);
                }
                self.city_ai_do_turn(g, c)
            }
            AiHook::TeamTurnPost(t) => self.team_ai_do_turn_post(g, t),
            AiHook::TeamAreaStrategies(t) => self.team_ai_update_area_stragies(g, t, true),
            AiHook::EndTurnSpells(p) => self.end_turn_spells(g, p),
            AiHook::GroupAttackSpells(grp) => self.group_attack_spells(g, grp),
            AiHook::UnitUpdate(u) => {
                if g.units.contains(u) {
                    self.unit_ai_update(g, u);
                }
            }
            AiHook::NowHasTech(p, tech) => self.player_ai_now_has_tech(g, p, tech),
            AiHook::DoCommerce(p) => self.player_ai_do_commerce(g, p),
            AiHook::UpdateFoundValues(p) => self.player_ai_update_found_values(g, p, false),
            AiHook::WarScript(p) => self.py_war_script(g, p),
            AiHook::UnitBuilt(u) => {
                if g.units.contains(u) {
                    self.py_unit_built_unit_ai(g, u);
                }
            }
            AiHook::ChooseGroupflag(u) => {
                if g.units.contains(u) {
                    self.unit_ai_choose_groupflag(g, u);
                }
            }
        }
    }

    fn assign_working_plots(&self, g: &mut Game, c: CityId) -> bool {
        self.city_ai_assign_working_plots(g, c);
        true
    }
}

/// A C++ AI controller for one game.
pub fn cppai_controller(seed: u32) -> ControllerRef {
    Arc::new(Cai::new(seed))
}

/// The names of the `AI_STRATEGY_*` flags set in `hash` (telemetry).
fn strategy_names(hash: i32) -> String {
    use crate::defs::*;
    let flags = [
        (AI_STRATEGY_DAGGER, "DAGGER"),
        (AI_STRATEGY_SLEDGEHAMMER, "SLEDGEHAMMER"),
        (AI_STRATEGY_CASTLE, "CASTLE"),
        (AI_STRATEGY_FASTMOVERS, "FASTMOVERS"),
        (AI_STRATEGY_SLOWMOVERS, "SLOWMOVERS"),
        (AI_STRATEGY_CULTURE1, "CULTURE1"),
        (AI_STRATEGY_CULTURE2, "CULTURE2"),
        (AI_STRATEGY_CULTURE3, "CULTURE3"),
        (AI_STRATEGY_CULTURE4, "CULTURE4"),
        (AI_STRATEGY_MISSIONARY, "MISSIONARY"),
        (AI_STRATEGY_CRUSH, "CRUSH"),
        (AI_STRATEGY_PRODUCTION, "PRODUCTION"),
        (AI_STRATEGY_PEACE, "PEACE"),
        (AI_STRATEGY_GET_BETTER_UNITS, "GET_BETTER_UNITS"),
        (AI_STRATEGY_LAND_BLITZ, "LAND_BLITZ"),
        (AI_STRATEGY_AIR_BLITZ, "AIR_BLITZ"),
        (AI_STRATEGY_LAST_STAND, "LAST_STAND"),
        (AI_STRATEGY_FINAL_WAR, "FINAL_WAR"),
        (AI_STRATEGY_OWABWNW, "OWABWNW"),
        (AI_STRATEGY_BIG_ESPIONAGE, "BIG_ESPIONAGE"),
    ];
    let names: Vec<&str> = flags.iter().filter(|&&(f, _)| hash & f != 0).map(|&(_, n)| n).collect();
    if names.is_empty() {
        "DEFAULT".to_string()
    } else {
        names.join("+")
    }
}
