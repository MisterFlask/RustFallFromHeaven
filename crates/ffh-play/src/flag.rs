//! Flags: a spectator's record of play that looks wrong (docs/HUMAN.md, "Watching the AI"). A
//! flag names the plot, the turn whose moves are on screen, the units and events there, the
//! build, the game's settings and the commands that replay that turn with the AI's trace on;
//! the save the replay starts from is copied out of the autosave ring (`flags/flag-N`), which
//! keeps only the last few turns. Each flag is a line of `<game dir>/flags.jsonl`.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use ffh_core::game::Game;
use ffh_core::ids::PlayerId;
use ffh_core::log::LogEntry;
use ffh_data::*;

use crate::json::Json;
use crate::setup::NewGame;

/// The flags file of a game directory.
pub const FLAGS_FILE: &str = "flags.jsonl";
/// The directory of the flags' pinned saves.
pub const FLAGS_DIR: &str = "flags";

/// What a flag is made from (`Session::flag`).
pub struct FlagInput<'a> {
    pub g: &'a Game,
    /// An observed (AI-only) game; else a human game (Tools -> Spectate, or one the human plays).
    pub observed: bool,
    /// The plot flagged.
    pub x: i32,
    pub y: i32,
    pub note: &'a str,
    /// The game's settings, when the session knows them (a new game, or a save whose sidecar
    /// records them).
    pub setup: Option<&'a NewGame>,
    /// The game directory (autosaves, journal); `None`: nothing is written.
    pub dir: Option<&'a Path>,
    pub journal: Option<PathBuf>,
    /// The latest messages (the events at the plot are quoted).
    pub messages: &'a [LogEntry],
}

/// The game turn whose moves are on screen: observed, at a turn boundary, the turn that just
/// ended (`None` at the start, before any turn); in a human game, at a control point of turn T,
/// turn T - 1 (every other player moved last in turn T - 1, and so did the human's slot).
pub fn moved_turn(g: &Game, observed: bool) -> Option<i32> {
    let start = g.start_turn;
    let t = if observed {
        g.at_turn_boundary().then(|| g.game_turn())
    } else {
        Some(g.game_turn() - 1)
    };
    t.filter(|&t| t >= start)
}

/// The autosave a replay of turn `moved` starts from, if it is still in the ring: the boundary
/// before it (`auto/T{moved-1}`), or `auto/start` for the game's first turn.
fn start_save(dir: &Path, g: &Game, moved: i32) -> Option<PathBuf> {
    let p = if moved <= g.start_turn { dir.join("auto").join("start.ffhsave") } else { dir.join("auto").join(format!("T{:04}.ffhsave", moved - 1)) };
    p.is_file().then_some(p)
}

/// The flags of a game directory, oldest first (lines that do not read are skipped).
pub fn list(dir: Option<&Path>) -> Vec<Json> {
    let Some(dir) = dir else { return Vec::new() };
    let Ok(text) = fs::read_to_string(dir.join(FLAGS_FILE)) else { return Vec::new() };
    text.lines().filter(|l| !l.trim().is_empty()).filter_map(|l| Json::parse(l).ok()).collect()
}

fn type_name<T: InfoId>(d: &GameData, t: Option<T>) -> Json {
    t.map_or(Json::Null, |t| Json::from(d.type_name(t)))
}

/// The plot's units (and those of its eight neighbours, `dx`/`dy` from it), with what a reader
/// of the bug needs: ids, owners, groups, the AI's role and the group's orders.
pub fn units_around(g: &Game, x: i32, y: i32) -> Vec<Json> {
    let d = g.d;
    let mut out = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let Some(p) = g.map.plot_xy(x + dx, y + dy) else { continue };
            for &u in &g.map.plots[p].units {
                let Some(unit) = g.units.get(u) else { continue };
                let grp = unit.group;
                let missions: Vec<Json> = grp
                    .and_then(|grp| g.groups.get(grp))
                    .map(|grp| grp.mission_queue.iter().map(|m| Json::from(d.type_name(m.mission_type))).collect())
                    .unwrap_or_default();
                out.push(crate::jobj! {
                    "id" => u.0,
                    "type" => d.type_name(unit.unit_type),
                    "owner" => unit.owner,
                    "dx" => dx,
                    "dy" => dy,
                    "group" => grp.map(|grp| grp.0),
                    "ai_type" => type_name(d, unit.ai_type),
                    "missions" => Json::Arr(missions),
                    "hp" => g.unit_curr_hit_points(u),
                    "level" => unit.level,
                    "moves" => unit.moves,
                });
            }
        }
    }
    out
}

/// One player as a flag names it.
fn player_json(g: &Game, p: PlayerId) -> Json {
    let d = g.d;
    crate::jobj! {
        "id" => p,
        "name" => crate::view::player_name(g, p),
        "civ" => type_name(d, g.player_get_civilization_type(p)),
        "leader" => type_name(d, g.player_get_leader_type(p)),
        "alive" => g.players[p].alive,
    }
}

/// The `ffh autoplay` command that plays the same game as observed setup `ng` (`ffh autoplay`
/// takes every setting of an observed game; docs/AI.md "Tools").
pub fn ffh_args(d: &GameData, ng: &NewGame) -> String {
    let mut s = format!("--seed {} --civs {} --size {}", ng.seed, ng.opponents + 1, d.type_name(ng.world));
    let defaults = NewGame::new(d, ng.seed);
    let mut opt = |flag: &str, v: &str, default: &str| {
        if v != default {
            let _ = write!(s, " {flag} {v}");
        }
    };
    opt("--speed", d.type_name(ng.speed), d.type_name(defaults.speed));
    opt("--handicap", d.type_name(ng.handicap), d.type_name(defaults.handicap));
    opt("--era", d.type_name(ng.era), d.type_name(defaults.era));
    opt("--climate", d.type_name(ng.climate), d.type_name(defaults.climate));
    opt("--sea-level", d.type_name(ng.sea_level), d.type_name(defaults.sea_level));
    if ng.max_turns != 0 {
        let _ = write!(s, " --max-turns {}", ng.max_turns);
    }
    for &o in &ng.options {
        let _ = write!(s, " --option {}", d.type_name(o));
    }
    for &v in &ng.disabled_victories {
        let _ = write!(s, " --no-victory {}", d.type_name(v));
    }
    if let Ok((civ, leader)) = ng.resolve_civ(d) {
        if let Some(c) = civ {
            let _ = write!(s, " --civ {}", d.type_name(c));
        }
        // A civilization alone gets the leader the seed picks, in both programs.
        if let (Some(l), Some(_)) = (leader, ng.leader) {
            let _ = write!(s, " --leader {}", d.type_name(l));
        }
    }
    s
}

/// Builds flag `n` from `input`: the record (with `text`, a summary to paste into a bug report).
/// With a game directory it pins the replay's start save and appends the record to
/// [`FLAGS_FILE`]. Returns the record.
pub fn record(input: &FlagInput) -> Result<Json, String> {
    let g = input.g;
    let d = g.d;
    let (x, y) = (input.x, input.y);
    let plot = g.map.plot_xy(x, y).ok_or_else(|| format!("({x}, {y}) is not on the map"))?;
    let n = list(input.dir).len() + 1;
    let moved = moved_turn(g, input.observed);
    let units = units_around(g, x, y);
    let pl = &g.map.plots[plot];
    let city = pl.plot_city.and_then(|c| g.cities.get(c).map(|city| (c, city)));
    // The players involved: the units' owners on the plot, its city's, the plot's.
    let mut owners: Vec<PlayerId> = Vec::new();
    for u in &units {
        if u.get("dx").and_then(Json::as_i64) == Some(0) && u.get("dy").and_then(Json::as_i64) == Some(0) {
            if let Some(o) = u.get("owner").and_then(Json::as_i64) {
                owners.push(PlayerId(o as u8));
            }
        }
    }
    owners.extend(city.map(|(_, c)| c.owner));
    owners.extend(pl.owner);
    owners.sort();
    owners.dedup();
    // The trace to turn on: the one civilization involved, else every player.
    let trace = match owners.as_slice() {
        [p] => p.0.to_string(),
        _ => "all".to_string(),
    };
    let near = |e: &LogEntry| e.at.is_some_and(|(ex, ey)| g.plot_distance(ex, ey, x, y) <= 1);
    let events: Vec<&LogEntry> = input.messages.iter().filter(|e| moved.is_some_and(|m| e.turn >= m) && near(e)).collect();

    // Pin the start save (the autosave ring keeps only the last turns).
    let mut pinned = None;
    if let (Some(dir), Some(m)) = (input.dir, moved) {
        if let Some(src) = start_save(dir, g, m) {
            let to = dir.join(FLAGS_DIR);
            fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
            let save = to.join(format!("flag-{n}.ffhsave"));
            fs::copy(&src, &save).map_err(|e| format!("{}: {e}", save.display()))?;
            let side = src.with_extension("ffhsess");
            if side.is_file() {
                let _ = fs::copy(&side, save.with_extension("ffhsess"));
            }
            pinned = Some(save);
        }
    }

    let setup_names = input.setup.map(|ng| ng.to_json_names(d));
    let mut repro = Vec::new();
    if let Some(m) = moved {
        let to_turn = format!("--to-turn {m} --plot {x},{y}");
        if input.observed {
            if let Some(ng) = input.setup {
                repro.push(format!("FFH_AI_TRACE={trace} ./target/release/ffh-play observe --setup '{}' {to_turn}", ng.to_json_names(d)));
            }
            if let Some(p) = &pinned {
                repro.push(format!("FFH_AI_TRACE={trace} ./target/release/ffh-play observe --load {} {to_turn}", p.display()));
            }
            if let Some(ng) = input.setup {
                // `--turns K` plays until turn K begins: turn `m` and the start of `m + 1`.
                repro.push(format!("FFH_AI_TRACE={trace} ./target/release/ffh autoplay {} --turns {}", ffh_args(d, ng), m + 1 - g.start_turn));
            }
        } else if let Some(j) = &input.journal {
            match &pinned {
                Some(p) => repro.push(format!("FFH_AI_TRACE={trace} ./target/release/ffh-play replay {} --from {} --to-turn {m}", j.display(), p.display())),
                None => repro.push(format!("FFH_AI_TRACE={trace} ./target/release/ffh-play replay {} --to-turn {m}", j.display())),
            }
        }
    }

    let unit_line = |u: &Json| -> String {
        let s = |k: &str| u.get(k).and_then(Json::as_str).unwrap_or("-").to_string();
        let i = |k: &str| u.get(k).and_then(Json::as_i64).unwrap_or(-1);
        let owner = PlayerId(i("owner") as u8);
        let missions: Vec<String> = u.get("missions").and_then(Json::as_arr).map(|a| a.iter().filter_map(Json::as_str).map(str::to_string).collect()).unwrap_or_default();
        format!(
            "{} #{} ({} p{}, group {}, {}{}, {} hp)",
            s("type"),
            i("id"),
            crate::view::player_name(g, owner),
            owner.0,
            i("group"),
            s("ai_type"),
            if missions.is_empty() { String::new() } else { format!(", orders {}", missions.join(" > ")) },
            i("hp")
        )
    };
    let here: Vec<String> = units.iter().filter(|u| u.get("dx").and_then(Json::as_i64) == Some(0) && u.get("dy").and_then(Json::as_i64) == Some(0)).map(unit_line).collect();
    let nearby: Vec<String> = units.iter().filter(|u| !(u.get("dx").and_then(Json::as_i64) == Some(0) && u.get("dy").and_then(Json::as_i64) == Some(0))).map(unit_line).collect();
    let checksum = g.calculate_sync_checksum();

    let mut text = String::new();
    let _ = writeln!(text, "FFH-RS flag {n}{}", if input.note.is_empty() { String::new() } else { format!(": {}", input.note) });
    match moved {
        Some(m) if input.observed => {
            let _ = writeln!(text, "Seen at the end of turn {m} (observed AI-only game); plot ({x}, {y}).");
        }
        Some(m) => {
            let _ = writeln!(text, "Seen at turn {} (human game; the other players' moves on screen are turn {m}'s); plot ({x}, {y}).", g.game_turn());
        }
        None => {
            let _ = writeln!(text, "Seen at the start of the game (turn {}), before any move; plot ({x}, {y}).", g.game_turn());
        }
    }
    let terrain = format!(
        "{}{}{}",
        pl.terrain_type.map_or("?", |t| d.type_name(t)),
        if g.plot_is_hills(plot) { ", hills" } else { "" },
        pl.feature_type.map_or(String::new(), |f| format!(", {}", d.type_name(f)))
    );
    let _ = writeln!(text, "Plot: {terrain}; owner {}.", pl.owner.map_or("none".to_string(), |o| format!("{} (p{})", crate::view::player_name(g, o), o.0)));
    if let Some((c, city)) = city {
        let _ = writeln!(text, "City: {} #{} ({} p{}, size {}).", crate::view::city_name(g, c), c.0, crate::view::player_name(g, city.owner), city.owner.0, city.population);
    }
    let _ = writeln!(text, "Units here: {}", if here.is_empty() { "none".to_string() } else { here.join("; ") });
    if !nearby.is_empty() {
        let _ = writeln!(text, "Units next to it: {}", nearby.join("; "));
    }
    for e in &events {
        let who = e.player.map_or("world".to_string(), |p| format!("p{p}"));
        let _ = writeln!(text, "Event (turn {}, {who}): {}", e.turn, e.text);
    }
    let _ = writeln!(text, "Build: {}", crate::BUILD_COMMIT);
    if let Some(s) = &setup_names {
        let _ = writeln!(text, "Setup: {s}");
    }
    let _ = writeln!(text, "Checksum now (turn {}, slice {}): {:08x}", g.game_turn(), g.turn_slice, checksum as u32);
    if let Some(p) = &pinned {
        let _ = writeln!(text, "Start save (pinned): {}", p.display());
    }
    if let Some(j) = &input.journal {
        let _ = writeln!(text, "Journal: {}", j.display());
    }
    if let Some(m) = moved {
        let _ = writeln!(text, "Replay turn {m} with the AI's trace (lines tagged [t{m} p{trace}]):");
        for r in &repro {
            let _ = writeln!(text, "  {r}");
        }
    }

    let rec = crate::jobj! {
        "flag" => n,
        "note" => input.note,
        "build" => crate::BUILD_COMMIT,
        "observed" => input.observed,
        "turn" => g.game_turn(),
        "year" => g.game_turn_year(),
        "moved_turn" => moved,
        "plot" => crate::jarr![x, y],
        "plot_owner" => pl.owner,
        "city" => city.map(|(c, city)| crate::jobj! { "id" => c.0, "name" => crate::view::city_name(g, c), "owner" => city.owner, "size" => city.population }),
        "units" => Json::Arr(units),
        "players" => Json::Arr(owners.iter().map(|&p| player_json(g, p)).collect()),
        "events" => Json::Arr(events.iter().map(|e| crate::save::log_entry_json(e)).collect()),
        "setup" => setup_names,
        "checksum" => format!("{:08x}", checksum as u32),
        "start_save" => pinned.as_ref().map(|p| p.display().to_string()),
        "journal" => input.journal.as_ref().map(|p| p.display().to_string()),
        "trace" => trace.as_str(),
        "repro" => Json::Arr(repro.iter().map(|r| Json::from(r.as_str())).collect()),
        "text" => text.as_str(),
    };
    if let Some(dir) = input.dir {
        let path = dir.join(FLAGS_FILE);
        let mut f = fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        writeln!(f, "{rec}").map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(rec)
}
