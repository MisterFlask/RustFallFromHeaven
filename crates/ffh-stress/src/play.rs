//! Plays one game (the minimal construction of `ffh-cli`'s `run::new_game`/`autoplay`, with
//! custom settings) and catches a panic.

use std::sync::Arc;
use std::time::Instant;

use ffh_core::controller::ControllerRef;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::mapscript::FractalMapScript;
use ffh_data::*;

use ffh_harness::panic::{self as harness_panic, set_turn};

use crate::settings::Settings;

pub struct Outcome {
    pub turn: i32,
    pub turns_played: i32,
    /// `ON`, `OVER <victory> <winner civs>`, ...
    pub end: String,
    pub panic: Option<(String, String, Vec<String>)>,
    pub secs: f64,
    pub checksum: i32,
}

impl Outcome {
    pub fn to_lines(&self) -> String {
        let mut s = format!("TURN\t{}\nEND\t{} (played {} turns, {:.1} s, checksum {:08x})\n", self.turn, self.end, self.turns_played, self.secs, self.checksum as u32);
        if let Some((msg, loc, bt)) = &self.panic {
            s += &format!("PANIC\t{} at {}\n", msg.replace('\n', " / "), loc);
            for f in bt {
                s += &format!("BT\t{f}\n");
            }
        }
        s
    }
}

/// Name of a team for reports: its members' civilizations.
fn team_name(g: &Game, t: TeamId) -> String {
    let mut names = Vec::new();
    for p in g.player_ids() {
        if g.players[p].team == t {
            let civ = g.player_get_civilization_type(p).map_or("?".to_string(), |c| g.d.civilizations[c].base.type_.clone());
            names.push(format!("{}:{}", p.0, civ.trim_start_matches("CIVILIZATION_")));
        }
    }
    names.join("+")
}

fn end_state(g: &Game) -> String {
    let alive = g.player_ids().filter(|&p| g.players[p].alive && p != g.barbarian_player()).count();
    let eliminated = g.player_ids().filter(|&p| g.players[p].ever_alive && !g.players[p].alive).count();
    let state = match g.game_state {
        GameStateType::GAMESTATE_ON => "ON".to_string(),
        GameStateType::GAMESTATE_OVER => "OVER".to_string(),
        _ => "EXTENDED".to_string(),
    };
    let mut s = state;
    if g.game_state != GameStateType::GAMESTATE_ON {
        let v = g.victory.map_or("NO_VICTORY".to_string(), |v| g.d.victories[v].base.type_.clone());
        let w = g.winner.map_or("NO_TEAM".to_string(), |t| team_name(g, t));
        s += &format!(" {} winner={}", v.trim_start_matches("VICTORY_"), w);
    }
    s + &format!(" alive={alive} eliminated={eliminated} cities={} units={}", g.cities.len(), g.units.len())
}

/// Builds the game for `s`, as `ffh-cli`'s `run::new_game` does (AI on every slot, Fractal
/// map script, autorun) but from `SetupOptions`.
pub fn new_game(s: &Settings) -> Result<Game, String> {
    let d = ffh_data::global();
    let o = s.setup_options(d)?;
    let mut g = Game::new_with_options(d, &o);
    g.map_script = Arc::new(FractalMapScript::default());
    let ai: ControllerRef = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    // FFH_STRESS_LOG=1: print the game log (messages, events) as it grows.
    g.log.enabled = std::env::var_os("FFH_STRESS_LOG").is_some();
    g.game_init();
    flush_log(&mut g);
    Ok(g)
}

/// Prints and clears the game log entries (only filled with FFH_STRESS_LOG set).
fn flush_log(g: &mut Game) {
    for e in g.log.entries.drain(..) {
        println!("LOG\t{}\t{}\t{}", e.turn, e.player.map_or("-".to_string(), |p| p.to_string()), e.text);
    }
}

/// `--grant-victory VICTORY_X@T`: (victory, turn).
fn parse_grant(g: &Game, s: &Settings) -> Option<(VictoryType, i32)> {
    let spec = s.grant_victory.as_ref()?;
    let (v, t) = spec.split_once('@').expect("--grant-victory VICTORY_X@TURN");
    Some((g.d.t::<VictoryType>(v), t.parse().expect("--grant-victory turn")))
}

/// Gives the capital of the first alive civ every building whose class counts towards `v`.
fn grant_victory_buildings(g: &mut Game, v: VictoryType) {
    let d = g.d;
    let barbarian = g.barbarian_player();
    let Some(p) = g.player_ids().find(|&p| p != barbarian && g.players[p].alive && g.player_get_capital_city(p).is_some()) else {
        return;
    };
    let capital = g.player_get_capital_city(p).unwrap();
    for class in d.building_classes.keys() {
        if d.building_classes[class].victory_threshold[v] > 0 {
            if let Some(b) = d.building_classes[class].default_building_index {
                g.city_set_num_real_building(capital, b, 1);
            }
        }
    }
    println!("GRANT\t{}\t{}", g.game_turn(), team_name(g, g.players[p].team));
}

/// Plays `s.turns` turns (or until the game ends), as `ffh-cli`'s `run::autoplay`.
fn play(s: &Settings, g: &mut Game) {
    let end_turn = g.game_turn() + s.turns;
    let mut turn = g.game_turn();
    set_turn(turn);
    let mut updates_this_turn = 0u64;
    // After a victory under autorun the game goes on (`GAMESTATE_EXTENDED`, as in C++): play
    // up to `EXTENDED_TURNS` more turns of it.
    const EXTENDED_TURNS: i32 = 50;
    let mut stop_turn = end_turn;
    let mut ended = false;
    let mut grant = parse_grant(g, s);
    let check_every: i32 = std::env::var("FFH_STRESS_CHECK").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    while g.game_state != GameStateType::GAMESTATE_OVER && g.game_turn() < stop_turn {
        if g.game_state != GameStateType::GAMESTATE_ON && !ended {
            ended = true;
            check_end(g);
            println!("VICTORY\t{}\t{}", g.game_turn(), end_state(g));
            stop_turn = end_turn.min(g.game_turn() + EXTENDED_TURNS);
            continue;
        }
        if let Some((v, t)) = grant {
            if g.game_turn() >= t {
                grant_victory_buildings(g, v);
                grant = None;
            }
        }
        g.update();
        updates_this_turn += 1;
        if g.game_turn() != turn {
            turn = g.game_turn();
            set_turn(turn);
            println!("PROGRESS\t{turn}");
            flush_log(g);
            if check_every > 0 && turn % check_every == 0 {
                ffh_harness::check::check(g);
            }
            updates_this_turn = 0;
        } else if updates_this_turn > 100_000 {
            panic!("turn {turn} did not finish after {updates_this_turn} updates (a player never ends its turn)");
        }
    }
    // A game that ended straight in GAMESTATE_OVER (never extended).
    if !ended {
        check_end(g);
    }
}

/// Consistency of a finished game: a game that is over has a victory and a winner (C++
/// `setWinner` + `setGameState`), except at max turns when no civ team has the best score alone
/// (C++ `testVictory`: no time victory, then `GAMESTATE_EXTENDED`/`OVER` with no winner).
fn check_end(g: &Game) {
    if g.game_state == GameStateType::GAMESTATE_ON {
        return;
    }
    if g.victory.is_none() && g.max_turns() > 0 && g.elapsed_game_turns >= g.max_turns() {
        let barbarian = g.players[g.barbarian_player()].team;
        let mut scores: Vec<i32> = g.team_ids().filter(|&t| t != barbarian && g.team_is_alive(t)).map(|t| g.team_score[t]).collect();
        scores.sort_unstable_by(|a, b| b.cmp(a));
        assert!(scores.is_empty() || (scores.len() >= 2 && scores[0] == scores[1]), "max turns reached, best team score {scores:?} alone, but no time victory");
        println!("NOTE\tmax turns reached with no civ team alive or a tie for the best team score {scores:?}: no winner (as in C++)");
        return;
    }
    assert!(g.victory.is_some() && g.winner.is_some(), "game over without a victory/winner: {}", end_state(g));
}

pub fn run_caught(s: &Settings) -> Outcome {
    let t = Instant::now();
    let mut start_turn = 0;
    let r = harness_panic::catch("ffh_stress::play::run_caught", || -> Result<(String, i32), String> {
        let mut g = new_game(s)?;
        start_turn = g.game_turn();
        set_turn(start_turn);
        play(s, &mut g);
        Ok((end_state(&g), g.calculate_sync_checksum()))
    });
    let turn = harness_panic::turn();
    let (end, checksum, panic) = match r {
        Ok(Ok((end, cs))) => (end, cs, None),
        Ok(Err(e)) => ("SETTINGS-ERROR".to_string(), 0, Some((e, "settings".to_string(), Vec::new()))),
        Err(c) => ("PANIC".to_string(), 0, Some((c.message, c.location, c.frames))),
    };
    Outcome { turn, turns_played: (turn - start_turn).max(0), end, panic, secs: t.elapsed().as_secs_f64(), checksum }
}
