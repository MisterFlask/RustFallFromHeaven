//! Staged situations for testing the clients (`ffh-play stage
//! capture|attack|automoves|load|trade|diplomacy|automate|armies|governors|engagement|supply|election|settlement|corrupt|crusade|cards|sea|outpost|rituals|rituals-charged|tithe|ritual-civ:CIVILIZATION_X|ice DIR`,
//! and the transports' tests): a saved game at a human control point arranged with rules calls,
//! so that a client can be tried on a situation a fresh game reaches only by chance (a captured
//! city's keep/raze decision, an attack, the AutoMoves rule between commands, the transport
//! picker, a trade with a civilization met, a leader's first contact and war declaration). Each writes only `DIR/KIND.ffhsave` and its sidecar: the staged game is played with
//! no journal and no autosaves, so a `DIR` that is also a game's directory keeps its journal and
//! autosaves (P5 review). Nothing here is used in play.

use std::path::{Path, PathBuf};
use std::sync::Arc;

mod rituals;
pub use rituals::{ritual_civ, rituals, rituals_charged, tithe, StagedRitualCiv, StagedRituals, StagedTithe};

use ffh_core::ids::{PlayerId, UnitId, VoteId};
use ffh_core::path::{FinderKind, PathFinder};
use ffh_core::structs::MissionData;
use ffh_data::*;

use crate::command::{CmdOk, Command};
use crate::human::AdvisorAnswerer;
use crate::json::Json;
use crate::session::{Session, SessionOptions, Stop};
use crate::setup::NewGame;

/// A saved game at a human control point (turn 4 of a duel, seed 11) where the human's warrior
/// `unit` stands next to an undefended size-3 AI city at (`x`, `y`) of a team at war with it: a
/// `move` onto the city captures it and raises the keep/raze decision inside the command.
pub struct StagedCapture {
    /// `<dir>/capture.ffhsave` (with its sidecar).
    pub save: PathBuf,
    pub unit: UnitId,
    pub x: i32,
    pub y: i32,
    /// Game turn of the save.
    pub turn: i32,
}

/// A saved game at a human control point (turn 3 of a duel, seed 20) where one group (`ready`,
/// a new warrior in the capital) is ready and another (`mover`, a land unit) has a MOVE_TO order
/// two or more turns long queued with its moves left (an "auto unit"); every other group sleeps
/// and no barbarian or animal is left (a human group in danger drops its orders). Skipping
/// `ready` leaves nothing ready while `mover` has orders: the exe's next frame sends AutoMoves
/// (`updateTestEndTurn`), which moves `mover` before the end of the turn.
pub struct StagedAutoMoves {
    pub save: PathBuf,
    pub ready: UnitId,
    pub mover: UnitId,
    /// Where `mover` stands in the save.
    pub from: (i32, i32),
    pub turn: i32,
}

impl StagedCapture {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "capture", "save" => self.save.display().to_string(), "unit" => self.unit, "x" => self.x, "y" => self.y, "turn" => self.turn }
    }
}

impl StagedAutoMoves {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "automoves", "save" => self.save.display().to_string(), "ready" => self.ready, "mover" => self.mover,
            "from" => crate::jarr![self.from.0, self.from.1], "turn" => self.turn,
        }
    }
}

/// A duel with the human's settler settled, played (forced end turns) to `turn`.
fn duel_at(dir: &Path, seed: u32, turn: i32) -> Result<Session, String> {
    duel_as(dir, seed, turn, None)
}

/// [`duel_at`] with the human's civilization (`None`: random).
fn duel_as(dir: &Path, seed: u32, turn: i32, civ: Option<&str>) -> Result<Session, String> {
    duel_with(dir, seed, turn, civ, &[])
}

/// [`duel_as`] with game options on.
fn duel_with(dir: &Path, seed: u32, turn: i32, civ: Option<&str>, options: &[GameOptionType]) -> Result<Session, String> {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, seed);
    ng.opponents = 1;
    ng.world = d.t("WORLDSIZE_DUEL");
    ng.civ = civ.map(|c| d.t::<CivilizationType>(c));
    ng.options.extend_from_slice(options);
    // Only the staged save goes to `dir`: no journal, no autosave ring (which would overwrite and
    // prune the autosaves of a game kept there).
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    let advance = |s: &mut Session| match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    };
    advance(&mut s)?;
    let h = s.human;
    let ok = |s: &mut Session, c: Command| match s.execute(&c) {
        Ok(CmdOk::Applied { .. }) => Ok(()),
        other => Err(format!("{c:?} -> {other:?}")),
    };
    let settler = s.g.players[h].units.iter().copied().find(|&u| d.units[s.g.units[u].unit_type].found).ok_or("no settler")?;
    ok(&mut s, Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false })?;
    while s.g.game_turn() < turn {
        ok(&mut s, Command::EndTurn { force: true })?;
        advance(&mut s)?;
    }
    Ok(s)
}

/// Writes a [`StagedCapture`] into `dir` (a game directory).
pub fn capture(dir: &Path) -> Result<StagedCapture, String> {
    let d = ffh_data::global();
    let mut s = duel_at(dir, 11, 4)?;
    let h = s.human;
    let (c, owner) = s.g.cities.iter().find(|(_, c)| c.owner != h).map(|(id, c)| (id, c.owner)).ok_or("no AI city")?;
    let g = &mut s.g;
    g.city_set_population(c, 3);
    let city_plot = g.city_plot(c);
    for u in g.map.plots[city_plot].units.clone() {
        g.unit_kill(u, false, None);
    }
    // Undefended: its walls' Sentinels breached too (docs/SIEGES.md), so a move takes it.
    for k in 0..g.city_sentinel_count(c) {
        g.city_set_sentinel_damage(c, k, 100);
    }
    let (my, their) = (g.players[h].team, g.players[owner].team);
    g.team_meet(my, their, false);
    g.team_declare_war(my, their, false, None);
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let mut attacker = None;
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
        let Some(p) = g.plot_xy(cx, cy, dx, dy) else { continue };
        if g.plot_is_water(p) || g.plot_is_impassable(p) || !g.map.plots[p].units.is_empty() {
            continue;
        }
        let (px, py) = (g.map.plots[p].x, g.map.plots[p].y);
        let u = g.player_init_unit(h, warrior, px, py, None, None);
        if g.unit_can_move_or_attack_into(u, city_plot, false) {
            attacker = Some(u);
            break;
        }
        g.unit_kill(u, false, None);
    }
    let unit = attacker.ok_or("no plot to attack from")?;
    let paths = s.save("capture").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedCapture { save: paths.save, unit, x: cx, y: cy, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) where the human's warrior
/// `unit` stands next to a barbarian warrior at (`x`, `y`) in the open: hovering it shows the
/// attack odds, a `move` there attacks it.
pub struct StagedAttack {
    pub save: PathBuf,
    pub unit: UnitId,
    pub enemy: UnitId,
    pub x: i32,
    pub y: i32,
    pub turn: i32,
}

impl StagedAttack {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "attack", "save" => self.save.display().to_string(), "unit" => self.unit, "enemy" => self.enemy, "x" => self.x, "y" => self.y, "turn" => self.turn }
    }
}

/// Writes a [`StagedAttack`] into `dir` (a game directory).
pub fn attack(dir: &Path) -> Result<StagedAttack, String> {
    let d = ffh_data::global();
    let mut s = duel_at(dir, 11, 4)?;
    let h = s.human;
    let g = &mut s.g;
    let barbarian = g.barbarian_player();
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    // An empty land plot near the capital for our warrior, and one next to it for theirs.
    let free = |g: &ffh_core::game::Game, p: ffh_core::ids::PlotIdx| {
        !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.map.plots[p].units.is_empty() && g.map.plots[p].plot_city.is_none()
    };
    for r in 1..4 {
        for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
            let Some(p) = g.plot_xy(cx, cy, dx, dy) else { continue };
            if !free(g, p) {
                continue;
            }
            for (ex, ey) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let (px, py) = (g.map.plots[p].x, g.map.plots[p].y);
                let Some(q) = g.plot_xy(px, py, ex, ey) else { continue };
                if !free(g, q) || g.map.plots[q].owner.is_some_and(|o| o != h) {
                    continue;
                }
                let (qx, qy) = (g.map.plots[q].x, g.map.plots[q].y);
                let unit = g.player_init_unit(h, warrior, px, py, None, None);
                let enemy = g.player_init_unit(barbarian, warrior, qx, qy, None, None);
                if g.unit_can_move_or_attack_into(unit, q, false) {
                    let paths = s.save("attack").map_err(|e| format!("save: {e:?}"))?;
                    return Ok(StagedAttack { save: paths.save, unit, enemy, x: qx, y: qy, turn: s.g.game_turn() });
                }
                g.unit_kill(enemy, false, None);
                g.unit_kill(unit, false, None);
            }
        }
    }
    Err("no two free plots near the capital".into())
}

/// Writes a [`StagedAutoMoves`] into `dir` (a game directory).
pub fn auto_moves(dir: &Path) -> Result<StagedAutoMoves, String> {
    let d = ffh_data::global();
    let mut s = duel_at(dir, 20, 3)?;
    let h = s.human;
    let g = &mut s.g;
    let barbarian = g.barbarian_player();
    for u in g.players[barbarian].units.clone() {
        g.unit_kill(u, false, None);
    }
    // A land group of ours and a destination two or more turns away (found with a finder of
    // our own, as the interface does).
    let mut target = None;
    'search: for &grp in &g.players[h].groups {
        let head = g.groups[grp].units[0];
        if g.unit_get_domain_type(head) != DomainType::DOMAIN_LAND || !g.group_ready_to_move(grp, true) {
            continue;
        }
        let (x, y) = (g.units[head].x, g.units[head].y);
        let mut finder = PathFinder::new(FinderKind::Path);
        finder.set_group(Some(grp));
        for r in 3..8 {
            for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
                let Some(p) = g.plot_xy(x, y, dx, dy) else { continue };
                let (tx, ty) = (g.map.plots[p].x, g.map.plots[p].y);
                if finder.generate_path(g, x, y, tx, ty, false, 0) && finder.path_turns().is_some_and(|t| t >= 2) {
                    target = Some((grp, tx, ty));
                    break 'search;
                }
            }
        }
    }
    let (grp, tx, ty) = target.ok_or("no land group with a destination two turns away")?;
    let mover = g.groups[grp].units[0];
    for other in g.players[h].groups.clone() {
        if other != grp && g.group_ready_to_move(other, true) {
            g.group_set_activity_type(other, ActivityType::ACTIVITY_SLEEP);
        }
    }
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let ready = g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), cx, cy, None, None);
    let turn = g.game_turn();
    g.group_insert_at_end_mission_queue(grp, MissionData { mission_type: MissionType::MISSION_MOVE_TO, data1: tx, data2: ty, flags: 0, push_turn: turn }, false);
    let ready_group = g.units[ready].group.ok_or("the new warrior has no group")?;
    if crate::decision::ready_groups(g, h) != [ready_group] {
        return Err("more than the new warrior is ready".into());
    }
    if !g.player_has_auto_unit(h) {
        return Err("the mover has no orders to continue".into());
    }
    let from = (g.units[mover].x, g.units[mover].y);
    let paths = s.save("automoves").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedAutoMoves { save: paths.save, ready, mover, from, turn })
}

/// A saved game at a human control point (a duel at turn 1 whose capital is on the coast)
/// where a new warrior `unit` stands in the capital at (`x`, `y`) with two new triremes
/// `transports` it can load into: the `load` action asks which one (the LOADUNIT picker,
/// `confirm load_unit`), answered with `do_command COMMAND_LOAD_UNIT`, `data1` = the
/// transport's owner and `data2` = its id (CvDLLButtonPopup LOADUNIT `OnOkClicked`).
pub struct StagedLoad {
    pub save: PathBuf,
    pub unit: UnitId,
    pub transports: Vec<UnitId>,
    pub x: i32,
    pub y: i32,
    pub turn: i32,
}

impl StagedLoad {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "load", "save" => self.save.display().to_string(), "unit" => self.unit, "transports" => self.transports.clone(),
            "x" => self.x, "y" => self.y, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedLoad`] into `dir` (a game directory): the first of a few fixed seeds whose
/// capital is on the coast.
pub fn load(dir: &Path) -> Result<StagedLoad, String> {
    let d = ffh_data::global();
    let trireme = d.t::<UnitType>("UNIT_TRIREME");
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    for seed in [4, 1, 2, 3, 5, 6, 7, 8, 9, 10] {
        let mut s = duel_at(dir, seed, 1)?;
        let h = s.human;
        let g = &mut s.g;
        let capital = *g.players[h].cities.first().ok_or("no city")?;
        if !g.city_is_coastal(capital, d.defines.min_water_size_for_ocean) {
            continue;
        }
        let (x, y) = (g.cities[capital].x, g.cities[capital].y);
        let transports = vec![g.player_init_unit(h, trireme, x, y, None, None), g.player_init_unit(h, trireme, x, y, None, None)];
        let unit = g.player_init_unit(h, warrior, x, y, None, None);
        let offered = crate::selection::load_targets(g, &[unit]);
        if offered != transports {
            return Err(format!("seed {seed}: the warrior can load into {offered:?}, not the two triremes {transports:?}"));
        }
        let paths = s.save("load").map_err(|e| format!("save: {e:?}"))?;
        return Ok(StagedLoad { save: paths.save, unit, transports, x, y, turn: s.g.game_turn() });
    }
    Err("no seed with a coastal capital".into())
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) where the human has met the
/// AI player `with` and is at peace with it, knows Currency (gold trading) and both have 100
/// gold: the diplomacy screen offers a trade whose tables hold gold both ways (a gift of gold is
/// accepted).
pub struct StagedTrade {
    pub save: PathBuf,
    pub with: PlayerId,
    pub turn: i32,
}

impl StagedTrade {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "trade", "save" => self.save.display().to_string(), "with" => self.with, "turn" => self.turn }
    }
}

/// Writes a [`StagedTrade`] into `dir` (a game directory).
pub fn trade(dir: &Path) -> Result<StagedTrade, String> {
    let mut s = duel_at(dir, 11, 4)?;
    let h = s.human;
    let with = s.g.civ_player_ids().find(|&p| p != h && s.g.players[p].alive).ok_or("no AI player")?;
    let g = &mut s.g;
    let (my, their) = (g.players[h].team, g.players[with].team);
    g.team_meet(my, their, false);
    // Gold trading (`CvTeam::isGoldTrading`: either side's tech with bGoldTrading, Currency).
    let currency = g.d.techs.keys().find(|&t| g.d.techs[t].gold_trading).ok_or("no gold-trading tech")?;
    g.team_set_has_tech(my, currency, true, Some(h), false, false);
    // Gold to trade (C++ `CvPlayer::setGold` is a plain setter).
    g.players[h].gold = 100;
    g.players[with].gold = 100;
    let paths = s.save("trade").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedTrade { save: paths.save, with, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) in which the AI player
/// `with` has just met the human and then declared war on it: its leader's two moments (first
/// contact, then the declaration, `AI_DIPLOCOMMENT_FIRST_CONTACT` and `_DECLARE_WAR`) are in the
/// sidecar, so a client that loads the save shows them; at war since this turn, it refuses to
/// talk.
pub struct StagedDiplomacy {
    pub save: PathBuf,
    pub with: PlayerId,
    pub turn: i32,
}

impl StagedDiplomacy {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "diplomacy", "save" => self.save.display().to_string(), "with" => self.with, "turn" => self.turn }
    }
}

/// Writes a [`StagedDiplomacy`] into `dir` (a game directory).
pub fn diplomacy(dir: &Path) -> Result<StagedDiplomacy, String> {
    let mut s = duel_at(dir, 11, 4)?;
    let h = s.human;
    let with = s.g.civ_player_ids().find(|&p| p != h && s.g.players[p].alive).ok_or("no AI player")?;
    let (my, their) = (s.g.players[h].team, s.g.players[with].team);
    if s.g.teams[my].has_met[their] {
        return Err("the human has met the AI already".into());
    }
    // As a unit of theirs coming into view would (`bNewDiplo`): the AI leader opens the screen.
    s.g.team_meet(their, my, true);
    s.g.team_declare_war(their, my, true, None);
    s.drain_log();
    if s.message_store().moments_total() != 2 {
        return Err(format!("{} leader moments, expected 2", s.message_store().moments_total()));
    }
    let paths = s.save("diplomacy").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedDiplomacy { save: paths.save, with, turn: s.g.game_turn() })
}

/// A saved game (docs/CRUSADE.md) at a human control point: a crusade against
/// AI player 1 (`target`) waits for the human's answer (`crusade`), the human has warriors to
/// lend in its capital, and AI player 2 (`ally`) is at war with the target (a loan can be
/// offered to it). Gold trading is known (loan payments).
pub struct StagedCrusade {
    pub save: PathBuf,
    pub crusade: u32,
    pub target: PlayerId,
    pub ally: PlayerId,
    pub turn: i32,
}

impl StagedCrusade {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "crusade", "save" => self.save.display().to_string(), "crusade" => self.crusade, "target" => self.target, "ally" => self.ally, "turn" => self.turn }
    }
}

/// Writes a [`StagedCrusade`] into `dir` (a game directory).
pub fn crusade(dir: &Path) -> Result<StagedCrusade, String> {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, 12);
    ng.opponents = 2;
    ng.world = d.t("WORLDSIZE_TINY");
    ng.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    let advance = |s: &mut Session| match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    };
    advance(&mut s)?;
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| d.units[s.g.units[u].unit_type].found).ok_or("no settler")?;
    match s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false }) {
        Ok(CmdOk::Applied { .. }) => {}
        other => return Err(format!("found -> {other:?}")),
    }
    for _ in 0..2 {
        match s.execute(&Command::EndTurn { force: true }) {
            Ok(CmdOk::Applied { .. }) => advance(&mut s)?,
            other => return Err(format!("end turn -> {other:?}")),
        }
    }
    let (target, ally) = (PlayerId(1), PlayerId(2));
    let g = &mut s.g;
    let teams: Vec<_> = [h, target, ally].iter().map(|&p| g.players[p].team).collect();
    for &a in &teams {
        for &b in &teams {
            if a != b {
                g.team_meet(a, b, false);
            }
        }
    }
    let currency = d.techs.keys().find(|&t| d.techs[t].gold_trading).ok_or("no gold-trading tech")?;
    for &t in &teams {
        g.team_set_has_tech(t, currency, true, None, false, false);
    }
    g.players[h].gold = 300;
    g.players[ally].gold = 300;
    g.team_declare_war(teams[2], teams[1], false, None);
    let capital = *g.players[h].cities.first().ok_or("no capital")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    for _ in 0..3 {
        g.player_init_unit(h, warrior, cx, cy, Some(UnitAIType::UNITAI_ATTACK), None);
    }
    let id = g.crusade_call(ffh_core::crusade::CrusadeKind::Council(VoteSourceType(0)), teams[1], None, ffh_core::crusade::Objective::Term, 30, 5, &[h, ally]).ok_or("no Host slot")?;
    let paths = s.save("crusade").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedCrusade { save: paths.save, crusade: id.0, target, ally, turn: s.g.game_turn() })
}

/// Free land plots around the human's capital (nearest first), for staged units.
fn free_plots_near_capital(g: &ffh_core::game::Game, h: PlayerId) -> Result<Vec<(i32, i32)>, String> {
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let mut out = Vec::new();
    for r in 1..4 {
        for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
            let Some(p) = g.plot_xy(cx, cy, dx, dy) else { continue };
            if !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.map.plots[p].units.is_empty() && g.map.plots[p].plot_city.is_none() {
                out.push((g.map.plots[p].x, g.map.plots[p].y));
            }
        }
    }
    Ok(out)
}

/// A saved game at a human control point (turn 4 of a duel, seed 11, the Bannor) where a new
/// scout `scout` of the human waits for orders on a free plot near the capital: the unit panel
/// offers its automate button (P7).
pub struct StagedAutomate {
    pub save: PathBuf,
    pub scout: UnitId,
    pub turn: i32,
}

impl StagedAutomate {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "automate", "save" => self.save.display().to_string(), "scout" => self.scout, "turn" => self.turn }
    }
}

/// Writes a [`StagedAutomate`] into `dir` (a game directory).
pub fn automate(dir: &Path) -> Result<StagedAutomate, String> {
    let d = ffh_data::global();
    let mut s = duel_as(dir, 11, 4, Some("CIVILIZATION_BANNOR"))?;
    let h = s.human;
    let g = &mut s.g;
    let plots = free_plots_near_capital(g, h)?;
    let &(sx, sy) = plots.first().ok_or("no free plot")?;
    let scout = g.player_init_unit(h, d.t::<UnitType>("UNIT_SCOUT"), sx, sy, None, None);
    let paths = s.save("automate").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedAutomate { save: paths.save, scout, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) with the decks in play
/// (docs/CARDS.md): the human knows the Starmetal Meteor and Legacy of Kings cards (spells
/// `meteor` and `legacy`, the top bar's card spells, cast at a plot and at a choice) and Mithril
/// Working (the meteor's Mithril shows); its Great Bard `bard`, in the capital, holds the Anointed
/// Champion card with its need (Arete) missing; the Armageddon deck is dealt with its slot 30
/// revealed.
pub struct StagedCards {
    pub save: PathBuf,
    pub bard: UnitId,
    pub meteor: SpellType,
    pub legacy: SpellType,
    pub turn: i32,
}

impl StagedCards {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "cards", "save" => self.save.display().to_string(), "bard" => self.bard,
            "meteor" => Json::idx(self.meteor), "legacy" => Json::idx(self.legacy), "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedCards`] into `dir` (a game directory).
pub fn cards(dir: &Path) -> Result<StagedCards, String> {
    let d = ffh_data::global();
    let mut s = duel_at(dir, 11, 4)?;
    let h = s.human;
    let g = &mut s.g;
    let team = g.players[h].team;
    for t in ["TECH_CARD_STARMETAL_METEOR", "TECH_CARD_LEGACY_OF_KINGS", "TECH_MITHRIL_WORKING"] {
        g.team_set_has_tech(team, d.t::<TechType>(t), true, Some(h), false, false);
    }
    // The bard alone on its plot (a click there selects it).
    let &(x, y) = free_plots_near_capital(g, h)?.first().ok_or("no free plot")?;
    let civ = g.player_get_civilization_type(h).ok_or("no civilization")?;
    let bard_type = d.civ_unit(civ, d.t::<UnitClassType>("UNITCLASS_ARTIST")).ok_or("no Great Bard")?;
    let bard = g.player_init_unit(h, bard_type, x, y, None, None);
    g.units[bard].card = Some(d.t::<TechType>("TECH_CARD_ANOINTED_CHAMPION"));
    if !g.doom.dealt {
        g.doom_deal();
    }
    let at30 = g.doom.slots.iter().position(|s| s.threshold == 30).ok_or("no slot 30")?;
    g.doom.slots[at30].revealed = true;
    let paths = s.save("cards").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedCards {
        save: paths.save,
        bard,
        meteor: d.t::<SpellType>("SPELL_STARMETAL_METEOR"),
        legacy: d.t::<SpellType>("SPELL_LEGACY_OF_KINGS"),
        turn: s.g.game_turn(),
    })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11, armies and governors on)
/// with this game's armies in play (docs/ARMIES.md): the human's `army` of two warriors next to
/// its capital `capital` (not its 1st Army when a starting unit outside the capital formed one),
/// with a target size of 4 and requisitioning from the capital, whose
/// requisition order leads its queue (ahead of a warrior it was building); a second city `city`
/// that no army requisitions from; and `recruit`, a warrior `city` trained for the army, marching
/// to it (the rules move it: it takes no orders until it arrives).
pub struct StagedArmies {
    pub save: PathBuf,
    pub army: ffh_core::ids::ArmyId,
    pub capital: ffh_core::ids::CityId,
    pub city: ffh_core::ids::CityId,
    pub recruit: UnitId,
    pub turn: i32,
}

impl StagedArmies {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "armies", "save" => self.save.display().to_string(), "army" => self.army, "capital" => self.capital,
            "city" => self.city, "recruit" => self.recruit, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedArmies`] into `dir` (a game directory).
pub fn armies(dir: &Path) -> Result<StagedArmies, String> {
    use ffh_core::army::RecruitDest;
    let d = ffh_data::global();
    let mut s = duel_with(dir, 11, 4, None, &[])?;
    let h = s.human;
    let g = &mut s.g;
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let area = g.map.plots[g.city_plot(capital)].area;
    // The second city 5 to 7 plots away on the capital's land: a recruit needs some turns to march.
    let mut site = None;
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let dist = g.plot_distance(x, y, cx, cy);
        if (5..=7).contains(&dist) && g.map.plots[q].area == area && g.player_can_found(h, x, y, false) && g.map.plots[q].units.is_empty() && site.map_or(true, |(b, _, _)| dist < b) {
            site = Some((dist, x, y));
        }
    }
    let (_, x, y) = site.ok_or("no city site 5 to 7 plots from the capital")?;
    let city = g.player_init_city(h, x, y, true, true);
    let w = d.t::<UnitType>("UNIT_WARRIOR");
    let &(ax, ay) = free_plots_near_capital(g, h)?.first().ok_or("no free plot near the capital")?;
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, w, ax, ay, None, None)).collect();
    let army = g.army_form(h, &units)?;
    g.army_set_target_size(h, army, 4)?;
    // The second city finishes a warrior for the army: it appears there and marches.
    g.city_push_order(city, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[city].order_queue[0].dest = Some(RecruitDest::Army(army));
    g.cities[city].unit_production[w] = g.city_production_needed_unit(city, w);
    let before = g.players[h].units.clone();
    g.city_pop_order(city, 0, true, false);
    let recruit = g.players[h].units.iter().copied().find(|u| !before.contains(u)).ok_or("no recruit")?;
    if g.unit_march(recruit).is_none() {
        return Err("the recruit does not march".into());
    }
    // The capital builds a warrior; the army's requisition puts its order ahead of it.
    g.city_push_order(capital, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, true, false);
    g.army_add_requisition(h, army, capital)?;
    g.player_do_requisition(h);
    if !g.cities[capital].order_queue.first().is_some_and(|o| o.requisition) {
        return Err("no requisition order in the capital".into());
    }
    g.player_army_sync(h);
    let paths = s.save("armies").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedArmies { save: paths.save, army, capital, city, recruit, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) under a governed government
/// (docs/GOVERNORS.md): the human has adopted Aristocracy, so its `capital` and a second city
/// `city` (5 to 7 plots away) are governed, each with a governor and a role; its `army` of two
/// warriors stands next to the capital, with no requisition.
pub struct StagedGovernors {
    pub save: PathBuf,
    pub army: ffh_core::ids::ArmyId,
    pub capital: ffh_core::ids::CityId,
    pub city: ffh_core::ids::CityId,
    pub turn: i32,
}

impl StagedGovernors {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "governors", "save" => self.save.display().to_string(), "army" => self.army, "capital" => self.capital,
            "city" => self.city, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedGovernors`] into `dir` (a game directory).
pub fn governors(dir: &Path) -> Result<StagedGovernors, String> {
    let d = ffh_data::global();
    let mut s = duel_with(dir, 11, 4, None, &[])?;
    let h = s.human;
    let g = &mut s.g;
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let area = g.map.plots[g.city_plot(capital)].area;
    let mut site = None;
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let dist = g.plot_distance(x, y, cx, cy);
        if (5..=7).contains(&dist) && g.map.plots[q].area == area && g.player_can_found(h, x, y, false) && g.map.plots[q].units.is_empty() && site.map_or(true, |(b, _, _)| dist < b) {
            site = Some((dist, x, y));
        }
    }
    let (_, x, y) = site.ok_or("no city site 5 to 7 plots from the capital")?;
    let city = g.player_init_city(h, x, y, true, true);
    let w = d.t::<UnitType>("UNIT_WARRIOR");
    let &(ax, ay) = free_plots_near_capital(g, h)?.first().ok_or("no free plot near the capital")?;
    let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, w, ax, ay, None, None)).collect();
    let army = g.army_form(h, &units)?;
    let aristocracy = d.t::<CivicType>("CIVIC_ARISTOCRACY");
    let option = d.civics[aristocracy].civic_option_type.ok_or("no civic option")?;
    g.player_set_civics(h, option, Some(aristocracy));
    if !g.city_is_governed(capital) || !g.city_is_governed(city) {
        return Err("the cities are not governed".into());
    }
    g.player_army_sync(h);
    let paths = s.save("governors").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedGovernors { save: paths.save, army, capital, city, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 4 of a duel, seed 11, armies and engagements on)
/// with an engagement to fight (docs/ENGAGEMENTS.md): the human's `army` (`units`: two Axemen, an
/// Archer, a Horseman and a Warrior) stands next to `city` at (`x`, `y`), a size-3 AI city of a
/// team at war with the human, held by `defenders` (a Warrior, an Archer and a Warrior); the
/// human's `capital`. Moving the army onto the city opens the battle plan in the browser client;
/// `engage {army, to}` fights it.
pub struct StagedEngagement {
    pub save: PathBuf,
    pub army: ffh_core::ids::ArmyId,
    pub units: Vec<UnitId>,
    pub city: ffh_core::ids::CityId,
    pub x: i32,
    pub y: i32,
    pub defenders: Vec<UnitId>,
    pub capital: ffh_core::ids::CityId,
    pub turn: i32,
}

impl StagedEngagement {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "engagement", "save" => self.save.display().to_string(), "army" => self.army, "units" => self.units.clone(),
            "city" => self.city, "x" => self.x, "y" => self.y, "defenders" => self.defenders.clone(), "capital" => self.capital, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedEngagement`] into `dir` (a game directory).
pub fn engagement(dir: &Path) -> Result<StagedEngagement, String> {
    use ffh_core::engagement::WarTest;
    let d = ffh_data::global();
    let mut s = duel_with(dir, 11, 4, None, &[])?;
    let h = s.human;
    let g = &mut s.g;
    let capital = *g.players[h].cities.first().ok_or("no city of the human's")?;
    let (city, owner) = g.cities.iter().find(|(_, c)| c.owner != h).map(|(id, c)| (id, c.owner)).ok_or("no AI city")?;
    g.city_set_population(city, 3);
    let city_plot = g.city_plot(city);
    for u in g.map.plots[city_plot].units.clone() {
        g.unit_kill(u, false, None);
    }
    let (my, their) = (g.players[h].team, g.players[owner].team);
    g.team_meet(my, their, false);
    g.team_declare_war(my, their, false, None);
    let (cx, cy) = (g.cities[city].x, g.cities[city].y);
    let ty = |n: &str| d.t::<UnitType>(n);
    let defenders: Vec<UnitId> = ["UNIT_WARRIOR", "UNIT_ARCHER", "UNIT_WARRIOR"].iter().map(|n| g.player_init_unit(owner, ty(n), cx, cy, None, None)).collect();
    let kinds = ["UNIT_AXEMAN", "UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_WARRIOR"];
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
        let Some(p) = g.plot_xy(cx, cy, dx, dy) else { continue };
        if g.plot_is_water(p) || g.plot_is_impassable(p) || !g.map.plots[p].units.is_empty() || g.map.plots[p].plot_city.is_some() {
            continue;
        }
        let (px, py) = (g.map.plots[p].x, g.map.plots[p].y);
        let units: Vec<UnitId> = kinds.iter().map(|n| g.player_init_unit(h, ty(n), px, py, None, None)).collect();
        if g.engage_attackers(h, p, city_plot, WarTest::AtWar).len() == units.len() && !g.engage_defenders(city_plot, h, &units, WarTest::AtWar).is_empty() {
            let army = g.army_form(h, &units)?;
            g.player_army_sync(h);
            let paths = s.save("engagement").map_err(|e| format!("save: {e:?}"))?;
            return Ok(StagedEngagement { save: paths.save, army, units, city, x: cx, y: cy, defenders, capital, turn: s.g.game_turn() });
        }
        for u in units {
            g.unit_kill(u, false, None);
        }
    }
    Err("no plot to attack the city from".into())
}

/// A saved game at a human control point (turn 4 of a duel, seed 11) with supply lines in play
/// (docs/SUPPLY.md): the human's army `supplied` of two warriors at (`x`, `y`), 3 to 5 plots from
/// its `capital` on its land, whose supply line runs 3 plots or more to a city; and its army
/// `hungry` of two warriors at (`hx`, `hy`), the nearest land plot with no supply line (on the
/// capital's land when it has one, else elsewhere), in Hunger
/// (turn 1 of `SUPPLY_HUNGER_TURNS`: its counter set as a turn start with the line cut would).
pub struct StagedSupply {
    pub save: PathBuf,
    pub supplied: ffh_core::ids::ArmyId,
    pub hungry: ffh_core::ids::ArmyId,
    pub capital: ffh_core::ids::CityId,
    pub x: i32,
    pub y: i32,
    pub hx: i32,
    pub hy: i32,
    pub turn: i32,
}

impl StagedSupply {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "supply", "save" => self.save.display().to_string(), "supplied" => self.supplied, "hungry" => self.hungry,
            "capital" => self.capital, "x" => self.x, "y" => self.y, "hx" => self.hx, "hy" => self.hy, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedSupply`] into `dir` (a game directory).
pub fn supply(dir: &Path) -> Result<StagedSupply, String> {
    let d = ffh_data::global();
    let mut s = duel_with(dir, 11, 4, None, &[])?;
    let h = s.human;
    let g = &mut s.g;
    let capital = *g.players[h].cities.first().ok_or("no city of the human's")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let area = g.map.plots[g.city_plot(capital)].area;
    let free = |g: &ffh_core::game::Game, q: ffh_core::ids::PlotIdx| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].plot_city.is_none();
    // (off the capital's land, distance, plot index): the nearest on the capital's land first.
    let mut supplied = None;
    let mut hungry = None;
    for q in g.map.plot_ids() {
        if !free(g, q) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        let key = (g.map.plots[q].area != area, g.plot_distance(x, y, cx, cy));
        match g.plot_supply_line(h, q) {
            Some(l) if !key.0 && (3..=5).contains(&key.1) && l.path.len() >= 3 => {
                if supplied.is_none_or(|(b, _)| key < b) {
                    supplied = Some((key, q));
                }
            }
            None => {
                if hungry.is_none_or(|(b, _)| key < b) {
                    hungry = Some((key, q));
                }
            }
            _ => {}
        }
    }
    let (_, sp) = supplied.ok_or("no supplied plot 3 to 5 plots from the capital")?;
    let (_, hp) = hungry.ok_or("no land plot without a supply line")?;
    let w = d.t::<UnitType>("UNIT_WARRIOR");
    let form = |g: &mut ffh_core::game::Game, q: ffh_core::ids::PlotIdx| {
        let (x, y) = g.map.xy(q);
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, w, x, y, None, None)).collect();
        g.army_form(h, &units)
    };
    let a = form(g, sp)?;
    let b = form(g, hp)?;
    g.armies[b].unsupplied_turns = 1;
    g.player_army_sync(h);
    if g.army_supply_line(a).is_none() || g.army_supply_line(b).is_some() {
        return Err("the staged armies' lines are not as staged".into());
    }
    let ((x, y), (hx, hy)) = (g.map.xy(sp), g.map.xy(hp));
    let paths = s.save("supply").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedSupply { save: paths.save, supplied: a, hungry: b, capital, x, y, hx, hy, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 3 of a duel, seed 15) where both players run a
/// council's civic, the human's team is its secretary general, and the secretary's election
/// (BUTTONPOPUP_CHOOSEELECTION) waits for the human: `pending.elections` holds `selection` (P7).
pub struct StagedElection {
    pub save: PathBuf,
    pub selection: VoteId,
    pub turn: i32,
}

impl StagedElection {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "election", "save" => self.save.display().to_string(), "selection" => self.selection, "turn" => self.turn }
    }
}

/// Writes a [`StagedElection`] into `dir` (a game directory).
pub fn election(dir: &Path) -> Result<StagedElection, String> {
    let d = ffh_data::global();
    let mut s = duel_at(dir, 15, 3)?;
    let h = s.human;
    let g = &mut s.g;
    let team = g.players[h].team;
    let mut council = None;
    for name in ["DIPLOVOTE_OVERCOUNCIL", "DIPLOVOTE_UNDERCOUNCIL"] {
        let vs = d.t::<VoteSourceType>(name);
        let Some(civic) = d.vote_sources[vs].civic else { continue };
        if let Some(t) = d.civics[civic].tech_prereq {
            g.team_set_has_tech(team, t, true, Some(h), false, false);
        }
        if g.player_can_do_civics(h, Some(civic)) {
            council = Some((vs, civic));
            break;
        }
    }
    let (vs, civic) = council.ok_or("no council the human can join")?;
    let option = d.civics[civic].civic_option_type.ok_or("a civic without an option")?;
    for p in g.civ_player_ids().collect::<Vec<_>>() {
        if g.players[p].alive {
            g.player_set_civics(p, option, Some(civic));
        }
    }
    let sg = d.votes.keys().find(|&v| d.votes[v].secretary_general && d.votes[v].vote_source_type[vs]).ok_or("no secretary general vote")?;
    g.vote_outcome[sg] = ffh_core::structs::PlayerVote(team.0 as i32);
    g.vote_timer[vs] = 0;
    g.secretary_general_timer[vs] = 5;
    let before: Vec<VoteId> = g.vote_selections.iter().map(|v| v.id).collect();
    g.do_vote_selection();
    let selection = g.vote_selections.iter().map(|v| v.id).find(|id| !before.contains(id)).ok_or("no election to hold")?;
    let paths = s.save("election").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedElection { save: paths.save, selection, turn: s.g.game_turn() })
}

/// A saved game at a human control point (turn 2 of a duel, seed 12, the Kuriotates) where a
/// new settler `settler` of the human stands on a city site at (`x`, `y`): founding there asks
/// city or settlement (BUTTONPOPUP_CONFIRMSETTLEMENT, P7).
pub struct StagedSettlement {
    pub save: PathBuf,
    pub settler: UnitId,
    pub x: i32,
    pub y: i32,
    pub turn: i32,
}

impl StagedSettlement {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "settlement", "save" => self.save.display().to_string(), "settler" => self.settler, "x" => self.x, "y" => self.y, "turn" => self.turn }
    }
}

/// Writes a [`StagedSettlement`] into `dir` (a game directory).
pub fn settlement(dir: &Path) -> Result<StagedSettlement, String> {
    let d = ffh_data::global();
    let mut s = duel_as(dir, 12, 2, Some("CIVILIZATION_KURIOTATES"))?;
    let h = s.human;
    let g = &mut s.g;
    let capital = *g.players[h].cities.first().ok_or("no city")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let mut site = None;
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let dist = g.plot_distance(x, y, cx, cy);
        if (4..=6).contains(&dist) && g.player_can_found(h, x, y, false) && g.map.plots[q].units.is_empty() && site.map_or(true, |(b, _, _)| dist < b) {
            site = Some((dist, x, y));
        }
    }
    let (_, x, y) = site.ok_or("no city site near the capital")?;
    let settler = g.player_init_unit(h, d.t::<UnitType>("UNIT_SETTLER"), x, y, None, None);
    let paths = s.save("settlement").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedSettlement { save: paths.save, settler, x, y, turn: s.g.game_turn() })
}

/// A saved game at a human control point in a real mid-game world, for the 3D renderer's vertical
/// slice (docs/R3D.md): seed 11 (or the next seed, up to 30, while an AI change leaves no evil
/// civilization holding a capital at turn 200), a standard map at low sea level, the human playing the Ljosalfar
/// against six AI civilizations, 200 turns of AI autoplay; then the heartland of the evil
/// civilization with the most cities is corrupted by FfH2's own hell rule -- every land plot it
/// owns within two plots of its capital takes the plot counter an Infernal owner gives its plots
/// each turn (`plot_change_plot_counter(p, 100)`, CvGameUtils.doHellTerrain's Infernal branch):
/// the terrains turn to their hell forms and the resources to their hell kinds, as in play.
pub struct StagedCorrupt {
    pub save: PathBuf,
    /// The corrupted civilization and its capital.
    pub player: PlayerId,
    pub x: i32,
    pub y: i32,
    /// Plots whose counter was raised.
    pub plots: usize,
    pub turn: i32,
    /// The game's seed (11 unless that seed had no evil civilization with a capital).
    pub seed: u32,
}

impl StagedCorrupt {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "stage" => "corrupt", "save" => self.save.display().to_string(), "player" => self.player.0 as i64, "x" => self.x, "y" => self.y, "plots" => self.plots as i64, "turn" => self.turn, "seed" => self.seed as i64 }
    }
}

pub fn corrupt(dir: &Path) -> Result<StagedCorrupt, String> {
    // The AI decides which civilizations live to turn 200: when an AI change leaves seed 11 with no
    // evil civilization holding a capital, the next seed that has one is staged instead.
    let mut last = String::new();
    for seed in 11..=30 {
        match corrupt_seed(dir, seed) {
            Ok(s) => return Ok(s),
            Err(e) if e == NO_EVIL => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(format!("{last} (seeds 11-30)"))
}

const NO_EVIL: &str = "no evil civilization with a capital";

fn corrupt_seed(dir: &Path, seed: u32) -> Result<StagedCorrupt, String> {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, seed);
    ng.opponents = 6;
    ng.world = d.t("WORLDSIZE_STANDARD");
    ng.sea_level = d.t("SEALEVEL_LOW");
    ng.civ = Some(d.t::<CivilizationType>("CIVILIZATION_LJOSALFAR"));
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    let advance = |s: &mut Session| match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    };
    advance(&mut s)?;
    match s.execute(&Command::AutoPlay { turns: 200 }) {
        Ok(CmdOk::Applied { .. }) => {}
        other => return Err(format!("autoplay -> {other:?}")),
    }
    advance(&mut s)?;
    let evil = d.t::<AlignmentType>("ALIGNMENT_EVIL");
    let g = &mut s.g;
    let player = g
        .player_ids()
        .filter(|&p| g.players[p].alive && g.players[p].alignment == Some(evil) && g.player_get_capital_city(p).is_some())
        .max_by_key(|&p| (g.players[p].cities.len(), std::cmp::Reverse(p.0)))
        .ok_or(NO_EVIL.to_string())?;
    let capital = g.player_get_capital_city(player).ok_or("no capital")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let mut plots = 0;
    for p in g.map.plot_ids() {
        let (x, y) = g.map.xy(p);
        if g.plot_distance(x, y, cx, cy) <= 2 && g.map.plots[p].owner == Some(player) && !g.plot_is_water(p) {
            g.plot_change_plot_counter(p, 100);
            plots += 1;
        }
    }
    let turn = s.g.game_turn();
    let paths = s.save("corrupt").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedCorrupt { save: paths.save, player, x: cx, y: cy, plots, turn, seed })
}

/// A saved game at a human control point (turn 1 of a duel; the first of a few fixed seeds whose
/// capital is on the coast, with two seas near it that it does not touch) for the sea
/// (docs/NAVAL.md, section 10), the plots within 16 of the capital revealed. Phase 1: a Trireme
/// next to the capital (`near`, in range), a Trireme just out of range and worn (`far`, 14 turns at
/// sea), a Warrior in boats near the coast (`boats`) and a Warrior on land next to the water
/// (`walker`). Phase 2 (named seas), at war with the other civilization: every sea the capital
/// touches held by its patrols (two Triremes on Patrol next to the capital in each: `held`, the
/// capital blockaded), the nearest other sea held by a Trireme of the human's on Patrol (`yours`,
/// `patrol`), the next contested by a Trireme of each side on Patrol side by side (`contested`,
/// `rival`, `enemy`), and a Galleon of the human's in `yours` whose lane to the capital is cut in
/// a held sea (`transport`).
pub struct StagedSea {
    pub save: PathBuf,
    pub capital: ffh_core::ids::CityId,
    pub near: UnitId,
    pub far: UnitId,
    pub boats: UnitId,
    pub walker: UnitId,
    pub held: Vec<ffh_core::ids::SeaId>,
    pub yours: ffh_core::ids::SeaId,
    pub contested: ffh_core::ids::SeaId,
    pub patrol: UnitId,
    pub rival: UnitId,
    pub enemy: UnitId,
    pub transport: UnitId,
    pub turn: i32,
}

impl StagedSea {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "sea", "save" => self.save.display().to_string(), "capital" => self.capital,
            "near" => self.near, "far" => self.far, "boats" => self.boats, "walker" => self.walker,
            "held" => Json::Arr(self.held.iter().map(|s| Json::from(s.0 as i64)).collect()),
            "yours" => self.yours.0 as i64, "contested" => self.contested.0 as i64,
            "patrol" => self.patrol, "rival" => self.rival, "enemy" => self.enemy, "transport" => self.transport,
            "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedSea`] into `dir` (a game directory).
pub fn sea(dir: &Path) -> Result<StagedSea, String> {
    use ffh_core::ids::PlotIdx;
    let d = ffh_data::global();
    let trireme = d.t::<UnitType>("UNIT_TRIREME");
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    for seed in [4, 1, 2, 3, 5, 6, 7, 8, 9, 10] {
        let mut s = duel_at(dir, seed, 1)?;
        let h = s.human;
        let g = &mut s.g;
        let capital = *g.players[h].cities.first().ok_or("no city")?;
        if !g.city_is_coastal(capital, d.defines.min_water_size_for_ocean) {
            continue;
        }
        let team = g.players[h].team;
        let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            if g.plot_distance(x, y, cx, cy) <= 16 {
                g.plot_set_revealed(q, team, true, false, None, false);
            }
        }
        let range = g.team_sea_range(team);
        // The water next to the capital; the water just out of range nearest the capital; water
        // at 2 from a port; land next to the water 2 or 3 plots from the capital.
        let (mut near, mut far, mut boats, mut walker): (Option<(i32, PlotIdx)>, Option<(i32, PlotIdx)>, Option<(i32, PlotIdx)>, Option<(i32, PlotIdx)>) = (None, None, None, None);
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            let dist = g.plot_distance(x, y, cx, cy);
            if !g.map.plots[q].units.is_empty() || g.map.plots[q].plot_city.is_some() || g.plot_is_impassable(q) {
                continue;
            }
            let key = (dist, q);
            let better = |b: Option<(i32, PlotIdx)>| b.is_none_or(|b| key < b);
            if g.plot_is_water(q) {
                match g.plot_sea_range(team, q) {
                    Some(1) if better(near) => near = Some(key),
                    Some(r) if r == range + 2 && better(far) => far = Some(key),
                    Some(2) if better(boats) => boats = Some(key),
                    _ => {}
                }
            } else if (2..=3).contains(&dist) && g.plot_is_adjacent_to_water(q) && better(walker) {
                walker = Some(key);
            }
        }
        let (Some((_, nq)), Some((_, fq)), Some((_, bq)), Some((_, wq))) = (near, far, boats, walker) else { continue };
        let ((nx, ny), (fx, fy), (bx, by), (wx, wy)) = (g.map.xy(nq), g.map.xy(fq), g.map.xy(bq), g.map.xy(wq));
        let near = g.player_init_unit(h, trireme, nx, ny, None, None);
        let far = g.player_init_unit(h, trireme, fx, fy, None, None);
        g.units[far].sea_turns = 14;
        let boats = g.player_init_unit(h, warrior, bx, by, None, None);
        if !g.unit_is_in_boats(boats) {
            return Err(format!("seed {seed}: the warrior at ({bx}, {by}) is not in boats"));
        }
        let walker = g.player_init_unit(h, warrior, wx, wy, None, None);
        let Some(p2) = sea_phase2(g, h, capital) else { continue };
        let paths = s.save("sea").map_err(|e| format!("save: {e:?}"))?;
        let (held, yours, contested, patrol, rival, enemy, transport) = p2;
        return Ok(StagedSea { save: paths.save, capital, near, far, boats, walker, held, yours, contested, patrol, rival, enemy, transport, turn: s.g.game_turn() });
    }
    Err("no seed with a coastal capital, water out of range and two other seas near it".into())
}

/// The named seas of [`StagedSea`] (phase 2), or None when the map does not allow them: `(held,
/// yours, contested, patrol, rival, enemy, transport)`.
#[allow(clippy::type_complexity)]
fn sea_phase2(g: &mut ffh_core::game::Game, h: PlayerId, capital: ffh_core::ids::CityId) -> Option<(Vec<ffh_core::ids::SeaId>, ffh_core::ids::SeaId, ffh_core::ids::SeaId, UnitId, UnitId, UnitId, UnitId)> {
    use ffh_core::ids::{PlotIdx, SeaId};
    let d = ffh_data::global();
    let trireme = d.t::<UnitType>("UNIT_TRIREME");
    let galleon = d.t::<UnitType>("UNIT_GALLEON");
    let team = g.players[h].team;
    let them = g.player_ids().find(|&p| p != h && g.players[p].alive && !g.player_is_barbarian(p))?;
    let their_team = g.players[them].team;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let held = g.city_seas(capital);
    if held.is_empty() {
        return None;
    }
    let free = |g: &ffh_core::game::Game, q: PlotIdx| g.plot_is_water(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].plot_city.is_none() && !g.plot_is_impassable(q) && g.map.plots[q].revealed[team];
    // Each sea of the capital: a free plot of it next to the capital.
    let mut held_at = Vec::new();
    for &s in &held {
        let q = g.map.seas[s.idx()].plots.iter().copied().find(|&q| {
            let (x, y) = g.map.xy(q);
            g.plot_distance(x, y, cx, cy) == 1 && free(g, q)
        })?;
        held_at.push(q);
    }
    // The other seas, nearest first (by their nearest revealed plot), with a free plot that has a
    // free neighbour of the same sea not next to an enemy patrol.
    let mut others: Vec<(i32, SeaId, PlotIdx, PlotIdx)> = Vec::new();
    for s in g.sea_ids() {
        if held.contains(&s) {
            continue;
        }
        let mut best: Option<(i32, PlotIdx, PlotIdx)> = None;
        for &q in &g.map.seas[s.idx()].plots {
            let (x, y) = g.map.xy(q);
            let dist = g.plot_distance(x, y, cx, cy);
            if dist > 12 || !free(g, q) || held_at.iter().any(|&e| { let (ex, ey) = g.map.xy(e); g.plot_distance(x, y, ex, ey) <= 2 }) {
                continue;
            }
            let pair = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))).filter(|&(dx, dy)| dx != 0 || dy != 0).filter_map(|(dx, dy)| g.map.plot_xy(x + dx, y + dy)).find(|&n| g.plot_sea(n) == Some(s) && free(g, n));
            if let Some(n) = pair {
                if best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, q, n));
                }
            }
        }
        if let Some((dist, q, n)) = best {
            others.push((dist, s, q, n));
        }
    }
    others.sort();
    let [(_, yours, yq, ytwin), (_, contested, cq, cn), ..] = others[..] else { return None };
    g.team_declare_war(team, their_team, false, None);
    let place = |g: &mut ffh_core::game::Game, p: PlayerId, ty: UnitType, q: PlotIdx, patrol: bool| {
        let (x, y) = g.map.xy(q);
        let u = g.player_init_unit(p, ty, x, y, None, None);
        if patrol {
            g.unit_plunder(u);
        }
        u
    };
    for &q in &held_at {
        place(g, them, trireme, q, true);
        place(g, them, trireme, q, true);
    }
    let patrol = place(g, h, trireme, yq, true);
    let transport = place(g, h, galleon, ytwin, false);
    let rival = place(g, h, trireme, cq, true);
    let enemy = place(g, them, trireme, cn, true);
    g.seas_refresh_all();
    // What the stage promises.
    let ok = matches!(g.city_harbour(capital), ffh_core::seas::Harbour::Blockaded(_))
        && g.sea_lane(team, g.unit_plot(transport)).is_some_and(|l| l.is_cut())
        && g.sea_state_known(contested, team).control != ffh_core::seas::SeaControl::HeldByUs;
    ok.then_some((held, yours, contested, patrol, rival, enemy, transport))
}

/// Two saved games at a human control point for outposts (docs/NAVAL.md, sections 9 and 10): an
/// outpost of the human's on the coast of another landmass within 14 plots of its coastal capital
/// (founded by the rules, the plots within 16 of the capital revealed), its lane open (`save`); the
/// same with the other player at war and a Frigate of its on Patrol in a sea the lane crosses, the
/// lane cut and counted for 2 turns (`save_cut`).
pub struct StagedOutpost {
    pub save: PathBuf,
    pub save_cut: PathBuf,
    pub outpost: ffh_core::ids::CityId,
    pub turn: i32,
}

impl StagedOutpost {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "outpost", "save" => self.save.display().to_string(), "save_cut" => self.save_cut.display().to_string(),
            "outpost" => self.outpost, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedOutpost`] into `dir` (a game directory).
pub fn outpost(dir: &Path) -> Result<StagedOutpost, String> {
    let d = ffh_data::global();
    let frigate = d.t::<UnitType>("UNIT_FRIGATE");
    for seed in [4, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        let mut s = duel_at(dir, seed, 1)?;
        let h = s.human;
        let g = &mut s.g;
        let capital = *g.players[h].cities.first().ok_or("no city")?;
        if !g.city_is_coastal(capital, d.defines.min_water_size_for_ocean) {
            continue;
        }
        let team = g.players[h].team;
        let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            if g.plot_distance(x, y, cx, cy) <= 16 {
                g.plot_set_revealed(q, team, true, false, None, false);
            }
        }
        let mut sites: Vec<(i32, ffh_core::ids::PlotIdx)> = g
            .map
            .plot_ids()
            .filter(|&q| {
                let (x, y) = g.map.xy(q);
                g.plot_distance(x, y, cx, cy) <= 14 && g.player_can_found_outpost(h, x, y, false)
            })
            .map(|q| {
                let (x, y) = g.map.xy(q);
                (g.plot_distance(x, y, cx, cy), q)
            })
            .collect();
        sites.sort();
        let Some(&(_, site)) = sites.first() else { continue };
        let (sx, sy) = g.map.xy(site);
        let Some(c) = g.player_found_outpost(h, sx, sy) else { continue };
        let Some(lane) = g.outpost_lane(c).filter(|l| !l.is_cut()) else { continue };
        let paths = s.save("outpost").map_err(|e| format!("save: {e:?}"))?;
        // The cut: war, and a Frigate of the other player on Patrol on the lane's middle plot.
        let g = &mut s.g;
        let other = g.player_ids().find(|&q| q != h && !g.player_is_barbarian(q)).ok_or("no rival")?;
        let ot = g.players[other].team;
        g.team_meet(team, ot, false);
        g.team_declare_war(ot, team, false, None);
        let mid = lane.path[lane.path.len() / 2];
        if g.map.plots[mid].sea.is_none() {
            continue;
        }
        let (mx, my) = g.map.xy(mid);
        let f = g.player_init_unit(other, frigate, mx, my, None, None);
        if !g.unit_can_plunder(f, mid, false) || !g.unit_plunder(f) {
            continue;
        }
        g.player_update_lanes(h);
        g.player_update_lanes(h);
        if !g.outpost_lane(c).is_some_and(|l| l.is_cut()) {
            continue;
        }
        let cut = s.save("outpost_cut").map_err(|e| format!("save: {e:?}"))?;
        return Ok(StagedOutpost { save: paths.save, save_cut: cut.save, outpost: c, turn: s.g.game_turn() });
    }
    Err("no seed with an outpost site in reach of a coastal capital".into())
}

/// A saved game (a duel, the human the Illians) at a human control point with Sea Ice (the
/// Illians' Frozen Passage, `ffh_core::ffh::world_ritual_spells::illians`): a patch cast near the
/// coastal capital where it touches the land on two sides, a warrior of the human standing on it
/// (`walker`), a trireme of the human on the water next to it (`ship`, Icebreak offered), and a
/// charge of Frozen Passage held (the top bar's world action).
pub struct StagedIce {
    pub save: PathBuf,
    pub capital: ffh_core::ids::CityId,
    /// The plot the patch was cast at.
    pub target: (i32, i32),
    pub walker: UnitId,
    pub ship: UnitId,
    pub turn: i32,
}

impl StagedIce {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "ice", "save" => self.save.display().to_string(), "capital" => self.capital,
            "target" => crate::jarr![self.target.0, self.target.1], "walker" => self.walker, "ship" => self.ship, "turn" => self.turn,
        }
    }
}

/// Writes a [`StagedIce`] into `dir` (a game directory).
pub fn ice(dir: &Path) -> Result<StagedIce, String> {
    use ffh_core::ffh::world_ritual_spells::illians as fp;
    use ffh_core::ids::PlotIdx;
    use ffh_core::structs::SpellTarget;
    let d = ffh_data::global();
    let trireme = d.t::<UnitType>("UNIT_TRIREME");
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let spell = d.t::<SpellType>("SPELL_FROZEN_PASSAGE");
    for seed in [4, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
        let mut s = duel_as(dir, seed, 1, Some("CIVILIZATION_ILLIANS"))?;
        let h = s.human;
        let g = &mut s.g;
        let capital = *g.players[h].cities.first().ok_or("no city")?;
        if !g.city_is_coastal(capital, d.defines.min_water_size_for_ocean) {
            continue;
        }
        let team = g.players[h].team;
        let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            if g.plot_distance(x, y, cx, cy) <= 12 {
                g.plot_set_revealed(q, team, true, false, None, false);
            }
        }
        if g.player_grant_world_ritual_charge(h).is_none() {
            return Err("the Illians have no world ritual".into());
        }
        let caster = g.player_init_unit(h, warrior, cx, cy, None, None);
        // The target: a patch that touches land on two sides (its shore plots at least 4 apart),
        // with open water next to it for the ship; the nearest to the capital (by plot distance,
        // then plot), 3 plots away or more.
        let mut best: Option<(i32, PlotIdx)> = None;
        for t in g.unit_spell_targets(caster, spell) {
            let SpellTarget::Plot(t) = t else { continue };
            let (tx, ty) = g.map.xy(t);
            let dist = g.plot_distance(tx, ty, cx, cy);
            // Away from a map edge that does not wrap (the camera stops there).
            let edge = |v: i32, n: i32, wraps: bool| wraps || (v >= 7 && v < n - 7);
            if dist < 3 || !edge(tx, g.map.grid_width, g.map.wrap_x) || !edge(ty, g.map.grid_height, g.map.wrap_y) {
                continue;
            }
            let patch = fp::frozen_passage_plots(g, t);
            let shore: Vec<(i32, i32)> =
                patch.iter().flat_map(|&q| fp::plots_within(g, q, 1)).filter(|&q| !g.plot_is_water(q) && !patch.contains(&q)).map(|q| g.map.xy(q)).collect();
            let spread = shore.iter().flat_map(|&(ax, ay)| shore.iter().map(move |&(bx, by)| (ax, ay, bx, by))).map(|(ax, ay, bx, by)| g.plot_distance(ax, ay, bx, by)).max().unwrap_or(0);
            let open = patch.iter().flat_map(|&q| fp::plots_within(g, q, 1)).any(|q| g.plot_is_water(q) && !patch.contains(&q));
            if spread >= 4 && open && best.is_none_or(|b| (dist, t) < b) {
                best = Some((dist, t));
            }
        }
        let Some((_, target)) = best else { continue };
        g.unit_cast_at(caster, spell, SpellTarget::Plot(target));
        if !g.map.plots[target].sea_ice {
            return Err(format!("seed {seed}: Frozen Passage did not freeze its target"));
        }
        g.unit_kill(caster, false, None);
        g.player_grant_world_ritual_charge(h);
        // The walker on the target; the ship on the water next to the patch, by a free ice plot.
        let (tx, ty) = g.map.xy(target);
        let walker = g.player_init_unit(h, warrior, tx, ty, None, None);
        let ship_at = g.map.plot_ids().filter(|&q| g.plot_is_water(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].revealed[team]).find(|&q| {
            fp::plots_within(g, q, 1).into_iter().any(|i| g.map.plots[i].sea_ice && g.map.plots[i].units.is_empty()) && fp::plots_within(g, q, 3).contains(&target)
        });
        let Some(ship_at) = ship_at else { continue };
        let (sx, sy) = g.map.xy(ship_at);
        let ship = g.player_init_unit(h, trireme, sx, sy, None, None);
        let paths = s.save("ice").map_err(|e| format!("save: {e:?}"))?;
        return Ok(StagedIce { save: paths.save, capital, target: (tx, ty), walker, ship, turn: s.g.game_turn() });
    }
    Err("no seed with a coastal capital and a strait or bay to freeze".into())
}
