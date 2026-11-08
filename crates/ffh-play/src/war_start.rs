//! The war start, a debug start of the new-game screen ("Start at war", session op `war_start`):
//! the game opens with you in charge of a civilization that an enemy stack is about to attack.
//!
//! An AI-only game of the new-game settings is played until two civilizations at war fight a
//! battle with [`WarStartOptions::min_stack`] or more fighting units on each side (the attacking
//! stack and the attacked one, as the fight starts: `Game::combat_notes`); then you take over the
//! attacked civilization at the last turn boundary at which it still moves before the attack.
//! Turns are sequential (players move in slot order), so for a battle in game turn T, with
//! attacker A and defender D, D's last move before it is its move of turn T when A moves after D,
//! else its move of turn T-1: the start is the turn boundary before that turn.
//!
//! From the take-over on your slot plays differently from the AI's (nothing moves until you move
//! it), so each candidate is checked: the start is played on with you idle (ending turns, the
//! advisor answering), and kept only if a battle of that size against you comes within
//! [`WarStartOptions::confirm_turns`] of your turns; otherwise the AI-only game goes on to the next
//! candidate. The search is deterministic (one setup gives one start). A chosen civilization (the
//! setup's `civ` / `leader`) plays slot 0 and only battles against it count; with "Random" any
//! attacked civilization does.

use std::collections::VecDeque;
use std::sync::Arc;

use ffh_core::game::Game;
use ffh_core::ids::PlayerId;
use ffh_core::log::CombatNote;
use ffh_data::*;

use crate::command::{CmdOk, Command};
use crate::human::AdvisorAnswerer;
use crate::json::Json;
use crate::session::{Session, SessionOptions, Stop};
use crate::setup::NewGame;

/// How the search runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarStartOptions {
    /// Fighting units on each side of the battle, at least ("more than 2 on each side").
    pub min_stack: i32,
    /// Game turns the AI-only game may play before the search gives up.
    pub max_turns: i32,
    /// Your turns after the start in which the battle must come in the check (you idle).
    pub confirm_turns: i32,
}

impl Default for WarStartOptions {
    fn default() -> WarStartOptions {
        WarStartOptions { min_stack: 3, max_turns: 500, confirm_turns: 2 }
    }
}

/// A battle: the first fight of an attack between two civilizations at war.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battle {
    pub turn: i32,
    pub attacker: PlayerId,
    pub defender: PlayerId,
    /// The attacked plot.
    pub x: i32,
    pub y: i32,
    pub attacker_stack: i32,
    pub defender_stack: i32,
}

impl Battle {
    /// `n` when it is a battle of `min_stack` a side between two civilizations at war (not a
    /// barbarian, an animal or a unit of hidden nationality).
    fn of(g: &Game, n: &CombatNote, min_stack: i32) -> Option<Battle> {
        let civ = |p: PlayerId| g.is_civ_player_slot(p);
        let (ta, td) = (g.players[n.attacker_owner].team, g.players[n.defender_owner].team);
        let war = ta != td && g.teams[ta].at_war[td];
        (civ(n.attacker_owner) && civ(n.defender_owner) && war && n.attacker_stack >= min_stack && n.defender_stack >= min_stack).then(|| Battle {
            turn: n.turn,
            attacker: n.attacker_owner,
            defender: n.defender_owner,
            x: g.map.plots[n.plot].x,
            y: g.map.plots[n.plot].y,
            attacker_stack: n.attacker_stack,
            defender_stack: n.defender_stack,
        })
    }

    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "turn" => self.turn,
            "attacker" => self.attacker,
            "defender" => self.defender,
            "x" => self.x,
            "y" => self.y,
            "attacker_stack" => self.attacker_stack,
            "defender_stack" => self.defender_stack,
        }
    }
}

/// A war start found.
pub struct WarStart {
    /// The game at the start's turn boundary with `player` human (a save `Session::load` takes:
    /// the AI-only game's, taken over, the slot given the new game's difficulty).
    pub save: Vec<u8>,
    pub player: PlayerId,
    /// The game turn of the save (the next turn played is `turn + 1`).
    pub turn: i32,
    /// The battle of the AI-only game that chose the start.
    pub battle: Battle,
    /// The battle of the check (you idle) that kept it.
    pub confirmed: Battle,
    /// Earlier candidates the check rejected.
    pub rejected: u32,
    /// The attacker's and your civilization and leader names.
    pub names: [String; 2],
}

impl WarStart {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "player" => self.player,
            "turn" => self.turn,
            "battle" => self.battle.to_json(),
            "confirmed" => self.confirmed.to_json(),
            "rejected" => self.rejected,
            "attacker_name" => self.names[0].as_str(),
            "player_name" => self.names[1].as_str(),
        }
    }
}

/// The AI-only game of `ng` (not yet `game_init`ed): an observed game's set-up
/// (`setup::build_observed_game`: every slot the AI's, a chosen civilization and leader in slot 0),
/// every slot AI-controlled and the game going on whoever is alive (as ffh-cli's autoplay).
fn ai_game(d: &'static GameData, ng: &NewGame) -> Result<Game, String> {
    let mut g = crate::setup::build_observed_game(d, ng)?;
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    Ok(g)
}

/// Finds the war start of `ng` (see the module docs). `progress` is called at every turn
/// boundary of the AI-only game with its turn.
pub fn find(ng: &NewGame, o: &WarStartOptions, progress: &mut dyn FnMut(i32)) -> Result<WarStart, String> {
    let d = ffh_data::global();
    let mut g = ai_game(d, ng)?;
    let chosen = ng.resolve_civ(d)?.0;
    g.game_init();
    g.combat_notes = Some(Vec::new());
    let end = g.game_turn() + o.max_turns;
    // The last two turn boundaries: (game turn, save).
    let mut boundaries: VecDeque<(i32, Vec<u8>)> = VecDeque::new();
    // Starts already checked (boundary turn, defender).
    let mut tried: Vec<(i32, PlayerId)> = Vec::new();
    let mut rejected = 0;
    let mut slices = 0u64;
    loop {
        if g.game_state != GameStateType::GAMESTATE_ON {
            return Err(format!("the AI-only game ended at turn {} before a battle of {} units a side", g.game_turn(), o.min_stack));
        }
        if g.at_turn_boundary() {
            let t = g.game_turn();
            if t >= end {
                return Err(format!("no battle of {} units a side between civilizations at war in {} turns{}", o.min_stack, o.max_turns, if chosen.is_some() { " against the chosen civilization (try Random)" } else { "" }));
            }
            progress(t);
            boundaries.push_back((t, g.save()));
            if boundaries.len() > 2 {
                boundaries.pop_front();
            }
        }
        g.update();
        slices += 1;
        if slices > 50_000_000 {
            return Err("the AI-only game did not progress".into());
        }
        let notes = g.combat_notes.as_mut().map(std::mem::take).unwrap_or_default();
        for n in &notes {
            let Some(battle) = Battle::of(&g, n, o.min_stack) else { continue };
            if chosen.is_some_and(|c| g.player_get_civilization_type(battle.defender) != Some(c)) {
                continue;
            }
            let start = if battle.attacker.0 > battle.defender.0 { battle.turn - 1 } else { battle.turn - 2 };
            if tried.contains(&(start, battle.defender)) {
                continue;
            }
            let Some((_, save)) = boundaries.iter().find(|(t, _)| *t == start) else { continue };
            tried.push((start, battle.defender));
            match take_over(save, battle.defender, ng.handicap).and_then(|bytes| confirm(&bytes, battle.defender, o).map(|c| (bytes, c))) {
                Ok((save, (confirmed, names))) => return Ok(WarStart { save, player: battle.defender, turn: start, battle, confirmed, rejected, names }),
                Err(_) => rejected += 1,
            }
        }
    }
}

/// The AI-only boundary `save` with slot `p` taken over (`Session::take_over`), its difficulty
/// the new game's (as `soak::spawn_civ` does: the human's handicap, where every AI slot has the
/// setup's default), saved again.
fn take_over(save: &[u8], p: PlayerId, handicap: HandicapType) -> Result<Vec<u8>, String> {
    let d = ffh_data::global();
    let mut g = Game::load(d, save).map_err(|e| e.to_string())?;
    g.setup.players[p.idx()].handicap = Some(handicap);
    let s = Session::take_over(&g.save(), p, Arc::new(AdvisorAnswerer), quiet()).map_err(|e| e.to_string())?;
    Ok(s.g.save())
}

/// Session options that write nothing (no saves, no journal).
fn quiet() -> SessionOptions {
    SessionOptions { saves_dir: None, autosave_keep: 0, journal: false, ..Default::default() }
}

/// Plays the start `save` (`p` human) on with `p` idle: the first battle of the search's size
/// against `p` after its first control point, within `confirm_turns` of its turns; with the
/// attacker's and `p`'s names.
fn confirm(save: &[u8], p: PlayerId, o: &WarStartOptions) -> Result<(Battle, [String; 2]), String> {
    let mut s = Session::load(save, None, Arc::new(AdvisorAnswerer), quiet()).map_err(|e| e.to_string())?;
    let advance = |s: &mut Session| match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the check stopped: {other:?}")),
    };
    advance(&mut s)?;
    let first = s.g.game_turn();
    // Fights before your first control point are not ones you could prepare for.
    s.g.combat_notes = Some(Vec::new());
    let team = s.g.players[p].team;
    loop {
        match s.execute(&Command::EndTurn { force: true }) {
            Ok(CmdOk::Applied { .. }) => {}
            other => return Err(format!("end turn: {other:?}")),
        }
        advance(&mut s)?;
        let notes = s.g.combat_notes.as_mut().map(std::mem::take).unwrap_or_default();
        let g = &s.g;
        if let Some(b) = notes.iter().filter_map(|n| Battle::of(g, n, o.min_stack)).find(|b| g.players[b.defender].team == team) {
            let name = |q: PlayerId| format!("{} of the {}", crate::view::player_name(g, q), g.player_get_civilization_short_description(q));
            return Ok((b.clone(), [name(b.attacker), name(p)]));
        }
        if s.g.game_turn() >= first + o.confirm_turns {
            return Err(format!("no battle against player {} by turn {}", p.0, s.g.game_turn()));
        }
    }
}

// No automated test: whether and when an AI-only game reaches a big battle is the seed's story
// (CLAUDE.md, Workflow). Try it with `ffh-play agent --no-game` and the session op `war_start`.
