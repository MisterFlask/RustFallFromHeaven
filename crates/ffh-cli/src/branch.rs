//! Branching a game at a decision (`ffh save` / `ffh resume`): the first step in judging an AI
//! behaviour (CLAUDE.md, "AI changes"; docs/AI.md, "Tools"). A game is played to a turn and saved;
//! the save is resumed twice, the behaviour on and off (its switch), and the local facts it should
//! change are printed turn by turn for both. Up to the fork the two branches are the same game --
//! same map, same opponents, same random-number state -- so the difference is that one decision's
//! (and what follows from it).
//!
//!   ffh save FILE --to-turn T [run options]
//!       play the game the run options describe (as `ffh stats` does) to the start of turn T and
//!       write it to FILE;
//!   ffh resume FILE --turns N [--watch SPEC]...
//!       load FILE, give every player the AI (fresh, as after any load), play N turns and print
//!       one line per watch and turn. SPEC is `city:X,Y` (the city there: owner, size, defenders
//!       and their health, defense damage), `player:P` (cities, units, army, power, research,
//!       gold, wars), `area:X,Y,R` (every player's units within R plots: count, health, types) or
//!       `unit:ID` (alive?, where, health, experience).
//!
//! `tools/ai-branch.sh` runs the pair and shows them side by side.

use std::sync::Arc;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::mapscript::FractalMapScript;
use ffh_data::*;

use crate::run;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(2)
}

/// `ffh save FILE --to-turn T [run options]`.
pub fn cmd_save(args: &[String], parse: impl Fn(&[String]) -> run::RunConfig) {
    let file = args.first().unwrap_or_else(|| fail("usage: ffh save FILE --to-turn T [run options]"));
    let mut rest: Vec<String> = Vec::new();
    let mut to_turn = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--to-turn" {
            to_turn = args.get(i + 1).and_then(|v| v.parse::<i32>().ok());
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }
    let to_turn = to_turn.unwrap_or_else(|| fail("ffh save: --to-turn T is required"));
    let mut cfg = parse(&rest);
    let (mut g, _) = run::new_game(&cfg);
    cfg.turns = to_turn - g.game_turn();
    cfg.until_supremacy = false;
    run::play(&mut g, &cfg, |_, _| {});
    // The safe point: no player's turn active (where `play` stops anyway).
    while g.game_state == GameStateType::GAMESTATE_ON && !g.at_turn_boundary() {
        g.update();
    }
    if g.game_turn() != to_turn {
        fail(format!("ffh save: the game ended at turn {} (before {to_turn})", g.game_turn()));
    }
    std::fs::write(file, g.save()).unwrap_or_else(|e| fail(format!("ffh save {file}: {e}")));
    println!("saved turn {} to {file} (checksum {:08x})", g.game_turn(), g.calculate_sync_checksum() as u32);
}

/// What `--watch` prints.
enum Watch {
    City(i32, i32),
    Player(PlayerId),
    Area(i32, i32, i32),
    Unit(UnitId),
}

fn parse_watch(spec: &str) -> Watch {
    let (kind, rest) = spec.split_once(':').unwrap_or_else(|| fail(format!("--watch {spec}: KIND:ARGS")));
    let n: Vec<i32> = rest.split(',').map(|v| v.trim().parse().unwrap_or_else(|_| fail(format!("--watch {spec}: numbers")))).collect();
    match (kind, n.as_slice()) {
        ("city", [x, y]) => Watch::City(*x, *y),
        ("player", [p]) => Watch::Player(PlayerId(*p as u8)),
        ("area", [x, y, r]) => Watch::Area(*x, *y, *r),
        ("unit", [u]) => Watch::Unit(UnitId(*u as u32)),
        _ => fail(format!("--watch {spec}: city:X,Y | player:P | area:X,Y,R | unit:ID")),
    }
}

fn civ(g: &Game, p: PlayerId) -> String {
    g.player_get_civilization_type(p).map_or_else(|| format!("p{}", p.0), |c| format!("p{} {}", p.0, g.d.civilizations[c].base.type_.trim_start_matches("CIVILIZATION_")))
}

fn hp(g: &Game, u: UnitId) -> i32 {
    let max = g.d.defines.max_hit_points.max(1);
    100 - g.units[u].damage * 100 / max
}

fn unit_name(g: &Game, u: UnitId) -> &str {
    g.d.units[g.units[u].unit_type].base.type_.trim_start_matches("UNIT_")
}

fn show(g: &Game, w: &Watch) -> String {
    match *w {
        Watch::City(x, y) => {
            let Some(c) = g.map.plot_xy(x, y).and_then(|q| g.map.plots[q].plot_city) else { return format!("city {x},{y}: none") };
            let city = &g.cities[c];
            let plot = g.city_plot(c);
            let def: Vec<UnitId> = g.map.plots[plot].units.iter().copied().filter(|&u| g.units[u].owner == city.owner && g.d.units[g.units[u].unit_type].combat > 0).collect();
            let health = if def.is_empty() { 0 } else { def.iter().map(|&u| hp(g, u)).sum::<i32>() / def.len() as i32 };
            format!("city {x},{y}: {} size {} defenders {} (health {}%) defense damage {}", civ(g, city.owner), city.population, def.len(), health, city.defense_damage)
        }
        Watch::Player(p) => {
            if p.idx() >= g.players.len() {
                return format!("player {}: none", p.0);
            }
            let pl = &g.players[p];
            if !pl.alive {
                return format!("{}: dead", civ(g, p));
            }
            let army = pl.units.iter().filter(|&&u| g.d.units[g.units[u].unit_type].combat > 0).count();
            let team = pl.team;
            let wars: Vec<String> = g.team_ids().into_iter().filter(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)).map(|t| format!("t{}", t.0)).collect();
            format!(
                "{}: cities {} units {} army {} power {} research {} gold {} wars [{}]",
                civ(g, p),
                pl.cities.len(),
                pl.units.len(),
                army,
                pl.power,
                g.player_calculate_research_rate(p, None),
                pl.gold,
                wars.join(" ")
            )
        }
        Watch::Area(x, y, r) => {
            let mut by: std::collections::BTreeMap<PlayerId, Vec<UnitId>> = std::collections::BTreeMap::new();
            for dx in -r..=r {
                for dy in -r..=r {
                    let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                    for &u in &g.map.plots[q].units {
                        by.entry(g.units[u].owner).or_default().push(u);
                    }
                }
            }
            let parts: Vec<String> = by
                .iter()
                .map(|(&p, us)| {
                    let health = us.iter().map(|&u| hp(g, u)).sum::<i32>() / us.len().max(1) as i32;
                    let mut kinds: std::collections::BTreeMap<&str, i32> = std::collections::BTreeMap::new();
                    for &u in us {
                        *kinds.entry(unit_name(g, u)).or_insert(0) += 1;
                    }
                    let kinds: Vec<String> = kinds.iter().map(|(k, n)| if *n > 1 { format!("{k} x{n}") } else { k.to_string() }).collect();
                    format!("{} {}u {}% [{}]", civ(g, p), us.len(), health, kinds.join(", "))
                })
                .collect();
            format!("area {x},{y},{r}: {}", if parts.is_empty() { "empty".to_string() } else { parts.join("; ") })
        }
        Watch::Unit(u) => {
            if !g.units.contains(u) {
                return format!("unit {}: gone", u.0);
            }
            let unit = &g.units[u];
            format!("unit {} {} of {}: at {},{} health {}% xp {} level {}", u.0, unit_name(g, u), civ(g, unit.owner), unit.x, unit.y, hp(g, u), unit.experience, unit.level)
        }
    }
}

/// `ffh resume FILE --turns N [--watch SPEC]...`.
pub fn cmd_resume(args: &[String]) {
    let file = args.first().unwrap_or_else(|| fail("usage: ffh resume FILE --turns N [--watch SPEC]..."));
    let mut turns = 10;
    let mut watches = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let val = args.get(i + 1).cloned().unwrap_or_else(|| fail(format!("{}: a value", args[i])));
        match args[i].as_str() {
            "--turns" => turns = val.parse().unwrap_or_else(|_| fail("--turns N")),
            "--watch" => watches.push(parse_watch(&val)),
            o => fail(format!("ffh resume: unknown option {o}")),
        }
        i += 2;
    }
    let d = ffh_data::global();
    let bytes = std::fs::read(file).unwrap_or_else(|e| fail(format!("ffh resume {file}: {e}")));
    let mut g = Game::load(d, &bytes).unwrap_or_else(|e| fail(format!("ffh resume {file}: {e}")));
    g.map_script = Arc::new(FractalMapScript::default());
    let ai = ffh_ai::ai_controller();
    for p in g.player_ids() {
        g.controllers[p] = ai.clone();
    }
    g.autorun = true;
    if std::env::var("FFH_LOG").is_ok() {
        g.log.enabled = true;
    }
    let print = |g: &Game| {
        for w in &watches {
            println!("t{} {}", g.game_turn(), show(g, w));
        }
    };
    print(&g);
    let cfg = run::RunConfig { turns: 1, ..run::RunConfig::default() };
    for _ in 0..turns {
        if g.game_state != GameStateType::GAMESTATE_ON {
            break;
        }
        run::play(&mut g, &cfg, |_, _| {});
        print(&g);
    }
    println!("t{} checksum {:08x}", g.game_turn(), g.calculate_sync_checksum() as u32);
}
