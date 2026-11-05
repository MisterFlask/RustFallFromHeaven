//! T1 (scripted bot game) and T2 (determinism), design §8.

mod common;

use common::{bot_session, check, checkpoint, new_game, peaceful_game};
use ffh_play::bot;
use ffh_play::{NewGame, Stop};

/// What one bot game produced.
struct Played {
    stop: Stop,
    checkpoints: Vec<(i32, i32)>,
    save: Vec<u8>,
}

fn play(seed: u32, turns: i32, check_every: i32) -> Played {
    play_game(new_game(seed), seed, turns, check_every)
}

/// Plays `ng` with a fresh bot for `turns` game turns, checking ffh-stress's invariants every
/// `check_every` turns (0 = never). Returns the stop, the per-control-point checksums and the
/// final save (for the determinism test).
fn play_game(ng: NewGame, seed: u32, turns: i32, check_every: i32) -> Played {
    let (mut s, mut b) = bot_session(&ng, None);
    let until = s.g.game_turn() + turns;
    let mut checkpoints = Vec::new();
    let mut last_checked = -1;
    let stop = bot::run(&mut s, &mut b, until, |s, _| {
        assert!(s.g.num_game_turn_active <= 1, "turn {}: {} players active", s.g.game_turn(), s.g.num_game_turn_active);
        checkpoints.push(checkpoint(s));
        let turn = s.g.game_turn();
        if check_every > 0 && turn % check_every == 0 && turn != last_checked {
            last_checked = turn;
            s.read(check::check);
        }
    });
    assert!(!matches!(stop, Stop::Stuck(_)), "seed {seed}: {stop:?}");
    s.read(check::check);
    let st = s.stats();
    let techs = s.read(|g| g.teams[g.players[s.human].team].has_tech.iter().filter(|&&t| t).count());
    eprintln!("seed {seed}: {stop:?} at turn {}, techs {techs}, cities {}, alive {}, winner {:?} victory {:?}, bot {:?}, session {st:?}", s.g.game_turn(), s.g.players[s.human].cities.len(), s.g.players[s.human].alive, s.g.winner, s.g.victory, b.stats);
    let (save, _) = s.save_bytes().expect("save at the end");
    Played { stop, checkpoints, save }
}

/// T1: three 150-turn games on a small map against 3 AIs, the human slot played by the bot only
/// through the protocol's queries and commands. The games cannot end (`peaceful_game`:
/// `common::STEADY`), so each runs its 150 turns for any seed. Asserted: no panic, no stuck turn,
/// a single active player at every control point, ffh-stress's invariants every 5 turns. How the
/// bot's game goes (cities founded, techs, popups) is not asserted: that is the seed's story.
#[test]
fn t1_scripted_bot_games() {
    for seed in [1, 2, 3] {
        let played = play_game(peaceful_game(seed), seed, 150, 5);
        assert_eq!(played.stop, Stop::Human, "seed {seed}: a game that cannot end runs its turns");
    }
}

/// T2: the same seed twice gives the same checksum at every control point and byte-identical
/// final saves.
#[test]
fn t2_same_seed_same_game() {
    let a = play(50, 60, 0);
    let b = play(50, 60, 0);
    assert_eq!(a.stop, b.stop);
    assert_eq!(a.checkpoints.len(), b.checkpoints.len());
    for (x, y) in a.checkpoints.iter().zip(&b.checkpoints) {
        assert_eq!(x, y, "the games diverged");
    }
    assert!(a.save == b.save, "final saves differ");
}
