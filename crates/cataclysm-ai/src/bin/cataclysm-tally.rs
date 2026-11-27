//! `cataclysm-tally`: plays AI-only Cataclysm games and reports how and when they end.
//!
//! The instrument for tuning the ruleset (docs/CATACLYSM.md, "Pacing targets"): the turn of the
//! Blowout, what raised Strain, when research ran out, which Lifeboats won, how wars went.
//!
//! ```text
//! cargo run --release -p cataclysm-ai --bin cataclysm-tally -- [--games N] [--opponents K]
//!     [--policy standard|quiet|pusher] [--seed S] [--rows] [--calibrate]
//! ```
//!
//! `--policy quiet`: nobody casts a ritual that raises Strain. `--policy pusher`: seat 0 spends
//! its Mana raising Strain, the others play the standard AI. The three together bracket what the
//! rules allow, so the rules are not tuned to one script. `--calibrate` also checks the attack
//! forecast against what attacks did, on copies of the game (the games themselves are unchanged).
//! Seat 0 takes each starting faction in turn; a game is its seed and nothing else.

use cataclysm_ai::{play_turn_as, Policy};
use cataclysm_core::{Command, Event, FactionId, Game, GameSetup, PlayerId, PlotIdx, SquadId, TECHS};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Standard,
    Quiet,
    Pusher,
}

struct Args {
    games: usize,
    opponents: usize,
    mode: Mode,
    seed: u64,
    rows: bool,
    calibrate: bool,
}

fn parse_args() -> Args {
    let mut a = Args { games: 120, opponents: GameSetup::default().ai_count, mode: Mode::Standard, seed: 1000, rows: false, calibrate: false };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = |name: &str| it.next().unwrap_or_else(|| panic!("{name} needs a value"));
        match k.as_str() {
            "--games" => a.games = val("--games").parse().expect("--games N"),
            "--opponents" => a.opponents = val("--opponents").parse().expect("--opponents K"),
            "--seed" => a.seed = val("--seed").parse().expect("--seed S"),
            "--policy" => {
                a.mode = match val("--policy").as_str() {
                    "standard" => Mode::Standard,
                    "quiet" => Mode::Quiet,
                    "pusher" => Mode::Pusher,
                    other => panic!("unknown policy {other}: standard, quiet or pusher"),
                }
            }
            "--rows" => a.rows = true,
            "--calibrate" => a.calibrate = true,
            other => panic!("unknown argument {other}"),
        }
    }
    a
}

/// Everything summed over the games.
#[derive(Default)]
struct Totals {
    games: usize,
    stalled: usize,
    invariant_errors: usize,
    outcomes: BTreeMap<String, usize>,
    winners_per_game: BTreeMap<usize, usize>,
    end_turn: Vec<i32>,
    strain_50: Vec<i32>,
    blowout: Vec<i32>,
    strain_without_blowout: Vec<i32>,
    capstone: Vec<i32>,
    exhausted: Vec<i32>,
    civs_alive_at_end: usize,
    idle_lore: i64,
    great_people: i64,
    civ_slots: usize,
    captured: i64,
    razed: i64,
    battles: i64,
    founded: i64,
    eliminated: i64,
    wars_declared: i64,
    peaces: i64,
    lifeboat_captures: i64,
    games_with_lifeboat_capture: usize,
    /// kind -> (registered, standing at the end, held by a winner at the end)
    lifeboats: BTreeMap<String, (usize, usize, usize)>,
    strain_sources: BTreeMap<String, i64>,
    /// faction -> (games present, games among the winners)
    factions: BTreeMap<&'static str, (usize, usize)>,
    keepers_games: usize,
    vigils: usize,
    seat0_out: Vec<i32>,
    /// Forecast decile -> (attacks, sum of the predicted permille, plots that fell)
    calibration: [(usize, i64, usize); 10],
}

struct GameRow {
    seed: u64,
    seat0: &'static str,
    end_turn: i32,
    outcome: String,
    winners: Vec<&'static str>,
    blowout: Option<i32>,
    strain: i32,
}

fn source_of(reason: &str) -> String {
    if reason == "caldera pressure" {
        "drift".into()
    } else if reason == "bloody battle" {
        "bloody battles".into()
    } else if reason.ends_with(" researched") {
        "techs".into()
    } else if reason.ends_with(" founded") {
        "creeds".into()
    } else if reason.ends_with(" razed") {
        "cities razed".into()
    } else if reason.ends_with("Benediction") {
        "Prophet bursts".into()
    } else {
        reason.to_string()
    }
}

/// One attack the active player could make now, tried on a copy: (predicted permille, plot fell).
fn sample_attack(g: &Game, p: PlayerId) -> Option<(i32, bool)> {
    let (sid, to): (SquadId, PlotIdx) = g.squads.values().filter(|s| s.owner == p && s.moves_left > 0).find_map(|s| {
        g.world
            .neighbors(s.plot)
            .into_iter()
            .find(|&nb| {
                g.squads.values().any(|o| o.plot == nb && g.at_war(p, o.owner))
                    || g.world.plot(nb).city.and_then(|c| g.cities.get(&c)).is_some_and(|c| g.at_war(p, c.owner))
            })
            .map(|nb| (s.id, nb))
    })?;
    let (_, forecast) = g.preview_attack(p, sid, to).ok()?;
    let mut copy = g.clone();
    let events = copy.apply(p, Command::AttackPlot { squad: sid, to }).ok()?;
    let fell = events.iter().find_map(|e| match e {
        Event::BattleFought { plot_taken, .. } => Some(*plot_taken),
        _ => None,
    })?;
    Some((forecast.plot_falls_permille, fell))
}

fn run_game(seed: u64, faction: FactionId, args: &Args, calibrate: bool, t: &mut Totals) -> GameRow {
    let mut g = Game::new(GameSetup { seed, human_faction: faction, ai_count: args.opponents, ..GameSetup::default() });
    let present: Vec<&'static str> = g.players.iter().filter(|p| !p.is_neutral).map(|p| p.faction.info().name).collect();
    let (mut strain_50, mut blowout, mut seat0_out) = (None, None, None);
    let mut capstone: Vec<Option<i32>> = Vec::new();
    let mut exhausted: Vec<Option<i32>> = Vec::new();
    let mut lifeboat_captures = 0i64;
    let mut wars: BTreeSet<(PlayerId, PlayerId)> = g.wars.clone();
    let mut steps = 0;
    let mut stalled = false;
    while !g.is_over() {
        let p = g.active_player;
        if !g.players[p.idx()].alive || steps > 40_000 {
            stalled = true;
            break;
        }
        if calibrate {
            if let Some((permille, fell)) = sample_attack(&g, p) {
                let bin = &mut t.calibration[(permille / 100).clamp(0, 9) as usize];
                bin.0 += 1;
                bin.1 += permille as i64;
                bin.2 += fell as usize;
            }
        }
        let policy = match args.mode {
            Mode::Standard => Policy::Standard,
            Mode::Quiet => Policy::Quiet,
            Mode::Pusher if p.idx() == 0 => Policy::Pusher,
            Mode::Pusher => Policy::Standard,
        };
        for e in play_turn_as(&mut g, p, policy) {
            match e {
                Event::CityCaptured { city, .. } => {
                    t.captured += 1;
                    if g.apocalypse.carcosa_risen && g.apocalypse.is_lifeboat(city).is_some() {
                        lifeboat_captures += 1;
                    }
                }
                Event::CityRazed { .. } => t.razed += 1,
                Event::BattleFought { .. } => t.battles += 1,
                Event::CityFounded { .. } => t.founded += 1,
                _ => {}
            }
        }
        // What moved Strain (the game keeps only the last few lines, so take them every step).
        for line in std::mem::take(&mut g.apocalypse.recent_changes) {
            let delta: i64 = line.split(' ').next().and_then(|d| d.parse().ok()).unwrap_or(0);
            let reason = line.split_once('(').map_or("?", |(_, r)| r.trim_end_matches(')'));
            *t.strain_sources.entry(source_of(reason)).or_default() += delta;
        }
        t.wars_declared += g.wars.difference(&wars).count() as i64;
        t.peaces += wars.difference(&g.wars).count() as i64;
        wars = g.wars.clone();
        if seat0_out.is_none() && !g.players[0].alive {
            seat0_out = Some(g.turn);
        }
        if strain_50.is_none() && g.apocalypse.counter >= 50 {
            strain_50 = Some(g.turn);
        }
        if blowout.is_none() && g.apocalypse.carcosa_risen {
            blowout = Some(g.turn);
        }
        capstone.resize(g.players.len(), None);
        exhausted.resize(g.players.len(), None);
        for i in 0..g.players.len() {
            let pl = &g.players[i];
            if pl.is_neutral || !pl.alive {
                continue;
            }
            if capstone[i].is_none() && pl.primary_doctrine.is_some() && TECHS.iter().any(|x| x.tier == 4 && x.doctrine == pl.primary_doctrine && pl.has_tech(x.id)) {
                capstone[i] = Some(g.turn);
            }
            if exhausted[i].is_none() && pl.current_research.is_none() && pl.primary_doctrine.is_some() && TECHS.iter().all(|x| g.can_research(PlayerId(i as u16), x.id).is_err()) {
                exhausted[i] = Some(g.turn);
            }
        }
        steps += 1;
    }

    t.games += 1;
    t.stalled += stalled as usize;
    t.invariant_errors += g.check_invariants().is_err() as usize;
    let outcome = g.apocalypse.outcome.clone();
    let kind = outcome.as_ref().map_or_else(|| "none (stalled)".to_string(), |o| format!("{:?}", o.kind));
    let winner_ids: Vec<PlayerId> = outcome.as_ref().map(|o| o.winners.clone()).unwrap_or_default();
    let winners: Vec<&'static str> = winner_ids.iter().map(|w| g.players[w.idx()].faction.info().name).collect();
    *t.outcomes.entry(kind.clone()).or_default() += 1;
    *t.winners_per_game.entry(winners.len()).or_default() += 1;
    t.end_turn.push(g.turn);
    t.strain_50.extend(strain_50);
    match blowout {
        Some(turn) => t.blowout.push(turn),
        None => t.strain_without_blowout.push(g.apocalypse.counter),
    }
    t.seat0_out.extend(seat0_out);
    for (i, pl) in g.players.iter().enumerate().filter(|(_, pl)| !pl.is_neutral) {
        t.civ_slots += 1;
        t.great_people += pl.great_people_born as i64;
        if !pl.alive {
            t.eliminated += 1;
            continue;
        }
        t.civs_alive_at_end += 1;
        t.idle_lore += pl.lore_progress as i64;
        t.capstone.extend(capstone.get(i).copied().flatten());
        t.exhausted.extend(exhausted.get(i).copied().flatten());
    }
    t.lifeboat_captures += lifeboat_captures;
    t.games_with_lifeboat_capture += (lifeboat_captures > 0) as usize;
    for l in &g.apocalypse.lifeboats {
        let e = t.lifeboats.entry(l.kind.name().to_string()).or_default();
        e.0 += 1;
        if !l.razed {
            e.1 += 1;
            if g.cities.get(&l.city_id).is_some_and(|c| winner_ids.contains(&c.owner)) {
                e.2 += 1;
            }
        }
    }
    for f in &present {
        t.factions.entry(f).or_default().0 += 1;
    }
    for f in &winners {
        t.factions.entry(f).or_default().1 += 1;
    }
    if present.contains(&FactionId::Elohim.info().name) {
        t.keepers_games += 1;
        t.vigils += (kind == "ElohimVigil") as usize;
    }
    GameRow { seed, seat0: faction.info().name, end_turn: g.turn, outcome: kind, winners, blowout, strain: g.apocalypse.counter }
}

/// (10th percentile, median, 90th percentile) of `v`, or dashes when it is empty.
fn spread(v: &[i32]) -> String {
    if v.is_empty() {
        return "-/-/-".into();
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    let at = |pct: usize| s[(s.len() - 1) * pct / 100];
    format!("{}/{}/{}", at(10), at(50), at(90))
}

fn pct(part: usize, whole: usize) -> usize {
    part * 100 / whole.max(1)
}

fn main() {
    let args = parse_args();
    let mut t = Totals::default();
    let mut rows = Vec::new();
    for n in 0..args.games {
        let faction = FactionId::STARTING[n % FactionId::STARTING.len()];
        // The forecast check copies the game for every sample: a quarter of the games is plenty.
        let calibrate = args.calibrate && n % 4 == 0;
        rows.push(run_game(args.seed + n as u64, faction, &args, calibrate, &mut t));
    }
    if args.rows {
        for r in &rows {
            println!(
                "seed {} seat0 {:<20} end T{:<3} Blowout {:<4} Strain {:<3} {} [{}]",
                r.seed,
                r.seat0,
                r.end_turn,
                r.blowout.map_or("-".into(), |b| format!("T{b}")),
                r.strain,
                r.outcome,
                r.winners.join(", ")
            );
        }
        println!();
    }

    let policy = match args.mode {
        Mode::Standard => "standard",
        Mode::Quiet => "quiet",
        Mode::Pusher => "pusher",
    };
    let g = t.games.max(1) as f64;
    let with_blowout = t.blowout.len();
    println!("== {} games, {} civilizations, policy {} ==", t.games, args.opponents + 1, policy);
    if t.stalled + t.invariant_errors > 0 {
        println!("!! stalled games: {}, games ending with a broken invariant: {}", t.stalled, t.invariant_errors);
    }
    for (kind, n) in &t.outcomes {
        println!("outcome {kind:<20} {n}");
    }
    println!("winners per game (winners: games): {:?}", t.winners_per_game);
    println!("p10/median/p90 of turns: Strain 50 {}, Blowout {}, game end {}", spread(&t.strain_50), spread(&t.blowout), spread(&t.end_turn));
    println!("games without a Blowout: {} (Strain at the end {})", t.strain_without_blowout.len(), spread(&t.strain_without_blowout));
    println!("Strain by source, mean per game until the Blowout:");
    let mut sources: Vec<(&String, &i64)> = t.strain_sources.iter().collect();
    sources.sort_by_key(|(_, d)| -**d);
    for (name, total) in sources {
        println!("  {:<28} {:+.1}", name, *total as f64 / g);
    }
    let early = t.exhausted.iter().filter(|&&turn| turn < 100).count();
    println!(
        "research (civs alive at the end: {}): primary capstone turn {} ({} reached it); out of research turn {} ({}% of civs before turn 100); idle Lore at the end {} per civ",
        t.civs_alive_at_end,
        spread(&t.capstone),
        t.capstone.len(),
        spread(&t.exhausted),
        pct(early, t.civs_alive_at_end),
        t.idle_lore / t.civs_alive_at_end.max(1) as i64
    );
    println!("Great People per civilization: {:.2}", t.great_people as f64 / t.civ_slots.max(1) as f64);
    println!(
        "per game: wars declared {:.1}, peace made {:.1}, battles {:.1}, cities captured {:.1}, razed {:.1}, founded {:.1}, civilizations eliminated {:.1}",
        t.wars_declared as f64 / g,
        t.peaces as f64 / g,
        t.battles as f64 / g,
        t.captured as f64 / g,
        t.razed as f64 / g,
        t.founded as f64 / g,
        t.eliminated as f64 / g
    );
    println!("seat 0 eliminated in {} games (turn {}), before turn 60 in {}", t.seat0_out.len(), spread(&t.seat0_out), t.seat0_out.iter().filter(|&&x| x < 60).count());
    println!(
        "after the Blowout ({} games): a Lifeboat was captured in {} of them ({:.1} captures per game)",
        with_blowout,
        t.games_with_lifeboat_capture,
        t.lifeboat_captures as f64 / with_blowout.max(1) as f64
    );
    println!("Lifeboats: registered / standing at the end / held by a winner");
    let winning: usize = t.lifeboats.values().map(|x| x.2).sum();
    for (kind, (reg, standing, won)) in &t.lifeboats {
        println!("  {kind:<18} {reg:>4} {standing:>4} {won:>4}");
    }
    let keep_wins = t.lifeboats.get("Caldera Keep").map_or(0, |x| x.2);
    println!("Keepers' turn-limit win: {} of {} games with the Keepers", t.vigils, t.keepers_games);
    println!("faction: games present, games among the winners");
    for (name, (present, won)) in &t.factions {
        println!("  {:<22} {:>4} {:>4}  {}%", name, present, won, pct(*won, *present));
    }
    if args.calibrate {
        println!("attack forecast (plot falls): predicted against observed, by decile of the prediction");
        for (i, (n, sum, fell)) in t.calibration.iter().enumerate() {
            if *n > 0 {
                println!("  {:>3}-{:<4}% attacks {:>5}  predicted {:>3}%  observed {:>3}%", i * 10, i * 10 + 10, n, sum / *n as i64 / 10, pct(*fell, *n));
            }
        }
    }
    println!(
        "TALLY policy={} civs={} games={} | Blowout p10/med/p90 {} (none in {}) | end {} | capstone {} | out of research before T100 {}% | vigil {}/{} | Lifeboat captured in {}% of Blowout games, Keep {}% of winning Lifeboats | peace/game {:.1} | eliminated/game {:.1} | Great People/civ {:.2}",
        policy,
        args.opponents + 1,
        t.games,
        spread(&t.blowout),
        t.strain_without_blowout.len(),
        spread(&t.end_turn),
        spread(&t.capstone),
        pct(early, t.civs_alive_at_end),
        t.vigils,
        t.keepers_games,
        pct(t.games_with_lifeboat_capture, with_blowout),
        pct(keep_wins, winning),
        t.peaces as f64 / g,
        t.eliminated as f64 / g,
        t.great_people as f64 / t.civ_slots.max(1) as f64
    );
}
