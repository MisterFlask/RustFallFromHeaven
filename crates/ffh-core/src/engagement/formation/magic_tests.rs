//! The battle cards' rules in the resolver (`magic.rs`), each on a small battle built by hand.

use super::*;
use ffh_data::{ActionKind as K, BattleCharm, Presence};

/// A = Dv: an even hit.
const EQ: Pair = Pair { a: 400, dv: 400, ma: 400, md: 400, spell_pct: 100, pct: 100 };

fn row(kind: K, count: i32) -> RowActions {
    RowActions { kind, count }
}

fn at(mut f: Fighter, lane: u8, r: u8) -> Fighter {
    f.lane = lane;
    f.row = r;
    f
}

fn frontline(side: Side, lane: u8, r: u8) -> Fighter {
    at(Fighter::new(side, 120), lane, r)
}

fn caster(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 70), lane, r);
    f.speed = 3;
    f.actions = [row(K::Strike, 1), row(K::Spell, 1), row(K::Spell, 2)];
    f.spell = Spell::battle(1, 100, Some(AttackPattern::Single));
    f
}

fn archer(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 80), lane, r);
    f.actions = [row(K::Shoot, 1), row(K::Shoot, 2), row(K::Shoot, 2)];
    f.accuracy = 100;
    f
}

fn battle(fighters: Vec<Fighter>) -> Formation {
    Formation::new(Params::default(), Ground::default(), [3, 3], fighters, |_, _| EQ)
}

fn run(f: &Formation, seed: u32) -> Outcome {
    resolve(f, &mut crate::rng::Rng::new(seed))
}

/// Grounding (Gravity Well): the enemy's charges are strikes for the battle, unless its side has a
/// hex ward on the field.
#[test]
fn grounding_turns_the_enemy_charges_into_strikes() {
    let mut horse = frontline(Side::Attacker, 1, 0);
    horse.actions = [row(K::Charge, 2), row(K::Strike, 1), row(K::Strike, 1)];
    horse.flanker = true;
    horse.speed = 9;
    let mut well = caster(Side::Defender, 1, 2);
    well.magic.presence = Some(Presence::Grounding);
    let kinds = |fighters: Vec<Fighter>| -> Vec<K> { run(&battle(fighters), 3).events.iter().filter(|e| e.actor == 0).map(|e| e.kind).collect() };
    let def = frontline(Side::Defender, 1, 0);
    let plain = kinds(vec![horse.clone(), def.clone(), caster(Side::Defender, 1, 2)]);
    assert!(plain.contains(&K::Charge), "{plain:?}");
    let grounded = kinds(vec![horse.clone(), def.clone(), well.clone()]);
    assert!(!grounded.is_empty() && grounded.iter().all(|&k| k == K::Strike), "{grounded:?}");
    let mut abjurer = caster(Side::Attacker, 0, 2);
    abjurer.magic.hex_ward = true;
    let warded = kinds(vec![horse, abjurer, def, well]);
    assert!(warded.contains(&K::Charge), "a hex ward keeps the charge: {warded:?}");
}

/// Madness (the Whisperer): in round 1 the enemy fighter with the highest A hits its own side; in
/// round 2 it hits its foes again.
#[test]
fn madness_turns_the_strongest_enemy_on_its_own_side() {
    let mut champ = frontline(Side::Attacker, 1, 0);
    champ.a = 900;
    champ.speed = 9;
    let ally = frontline(Side::Attacker, 0, 0);
    let mut whisperer = caster(Side::Defender, 1, 2);
    whisperer.magic.presence = Some(Presence::Madness);
    let def = frontline(Side::Defender, 1, 0);
    let out = run(&battle(vec![champ, ally, def, whisperer]), 5);
    let side_of = |t: FighterIdx| if t < 2 { Side::Attacker } else { Side::Defender };
    let hits: Vec<(u8, Side)> = out.events.iter().filter(|e| e.actor == 0).filter_map(|e| e.hits.first().map(|h| (e.round, side_of(h.target)))).collect();
    assert!(hits.iter().any(|&(r, s)| r == 1 && s == Side::Attacker), "round 1 on its own side: {hits:?}");
    assert!(hits.iter().filter(|&&(r, _)| r == 1).all(|&(_, s)| s == Side::Attacker), "{hits:?}");
    assert!(hits.iter().filter(|&&(r, _)| r >= 2).all(|&(_, s)| s == Side::Defender), "round 2 on its foes: {hits:?}");
}

/// The Abjurer: the enemy's hexes have no effect on its side, and the enemy side's charms end at
/// the start of round 2.
#[test]
fn an_abjurer_wards_its_side_and_ends_the_enemy_charms() {
    let mut hexer = caster(Side::Attacker, 0, 2);
    hexer.speed = 9;
    hexer.charms = vec![(7, BattleCharm { enemy: true, whole_side: true, taken: 50, ..BattleCharm::default() }), (8, BattleCharm { whole_side: true, damage: 40, ..BattleCharm::default() })];
    let mut abjurer = caster(Side::Defender, 1, 2);
    abjurer.magic.hex_ward = true;
    abjurer.magic.end_charms = true;
    let f = battle(vec![hexer, frontline(Side::Attacker, 1, 0), abjurer, frontline(Side::Defender, 1, 0)]);
    let mut run = Run::new(&f, true);
    // The hex lands on no one; the charm lands on the hexer's side.
    run.charm(0, 7, f.fighters[0].charms[0].1, Event { round: 1, pass: 1, actor: 0, kind: K::Spell, spell: None, target: None, hits: vec![], heal: None, knocked: None, note: None, entered: vec![] });
    assert_eq!(run.s.taken_mod[2..], [0, 0], "warded");
    run.charm(0, 8, f.fighters[0].charms[1].1, Event { round: 1, pass: 2, actor: 0, kind: K::Spell, spell: None, target: None, hits: vec![], heal: None, knocked: None, note: None, entered: vec![] });
    assert_eq!(run.s.dmg_mod[..2], [40, 40]);
    run.magic_round(2);
    assert_eq!(run.s.dmg_mod[..2], [0, 0], "ended at round 2");
}

/// The Lullaby: each enemy fighter in the rear row skips its next action.
#[test]
fn the_lullaby_makes_the_rear_row_skip() {
    let mut lutist = caster(Side::Attacker, 0, 2);
    lutist.speed = 20;
    lutist.charms = vec![(9, BattleCharm { enemy: true, rear: true, skip: 1, ..BattleCharm::default() })];
    let f = battle(vec![lutist, frontline(Side::Attacker, 1, 0), frontline(Side::Defender, 1, 0), archer(Side::Defender, 1, 2)]);
    let out = run(&f, 2);
    let skipped: Vec<&Event> = out.events.iter().filter(|e| e.note == Some(Note::Skipped)).collect();
    assert_eq!(skipped.len(), 1, "{:?}", out.events);
    assert_eq!((skipped[0].actor, skipped[0].round), (3, 1), "the archer, in round 1");
}

/// A Spellsword casts its spell in place of its first strike each round, then strikes.
#[test]
fn a_spellsword_casts_with_its_first_strike_each_round() {
    let mut sword = frontline(Side::Attacker, 1, 0);
    sword.speed = 9;
    sword.magic.strike_spell = true;
    sword.spell = Spell::battle(3, 120, Some(AttackPattern::Single));
    let f = battle(vec![sword, frontline(Side::Defender, 1, 0), archer(Side::Defender, 1, 2)]);
    let out = run(&f, 4);
    for r in 1..=out.rounds {
        let kinds: Vec<K> = out.events.iter().filter(|e| e.actor == 0 && e.round == r).map(|e| e.kind).collect();
        if kinds.is_empty() {
            continue;
        }
        assert_eq!(kinds.first(), Some(&K::Spell), "round {r}: {kinds:?}");
        assert!(kinds[1..].iter().all(|&k| k == K::Strike), "round {r}: {kinds:?}");
    }
    let spell = out.events.iter().find(|e| e.actor == 0 && e.kind == K::Spell).unwrap();
    assert_eq!((spell.spell, spell.target), (Some(3), Some(2)), "its spell, at the backmost");
}

/// Illusions: cast with the caster's first spell action into their free cells, they do not act,
/// a spell's hit kills one, and a side of illusions alone is down.
#[test]
fn illusions_enter_with_the_first_spell_and_die_to_a_spell() {
    let mut host = caster(Side::Attacker, 0, 2);
    host.speed = 20;
    host.magic.illusions = Some(IllusionSpell { id: 11, count: 2, hp: 60 });
    let mut fighters = vec![host, frontline(Side::Attacker, 1, 0), frontline(Side::Defender, 1, 0), caster(Side::Defender, 1, 2)];
    let casters = add_illusions(&mut fighters, [3, 3]);
    assert_eq!(casters, [0, 0]);
    assert_eq!(fighters.len(), 6);
    let cells: Vec<(u8, u8)> = fighters[4..].iter().map(|x| (x.lane, x.row)).collect();
    assert_eq!(cells, [(0, 0), (2, 0)], "the front row's free cells, the centre first");
    let f = battle(fighters);
    let out = run(&f, 8);
    let cast = out.events.iter().find(|e| !e.entered.is_empty()).expect("the illusions enter");
    assert_eq!((cast.actor, cast.spell, cast.round), (0, Some(11), 1));
    assert_eq!(cast.entered.iter().map(|x| (x.unit, x.lane, x.row, x.hp)).collect::<Vec<_>>(), [(4, 0, 0, 6000), (5, 2, 0, 6000)]);
    assert!(out.events.iter().all(|e| e.actor < 4), "illusions do not act");
    for e in out.events.iter().filter(|e| e.kind == K::Spell && e.actor == 3) {
        for h in e.hits.iter().filter(|h| h.target >= 4) {
            assert!(h.killed && h.dmg == 6000, "a spell's hit takes all its HP: {h:?}");
        }
    }
    // A side of illusions alone is down.
    let mut s = State::new(&f);
    s.mx.pending = vec![false; 6];
    for i in [0, 1] {
        s.alive[i] = false;
    }
    assert!(f.side_down(&s, Side::Attacker));
}

/// A familiar takes the first shot aimed at its caster, and only the first.
#[test]
fn a_familiar_takes_the_first_shot_at_its_caster() {
    let mut a = archer(Side::Attacker, 1, 2);
    a.speed = 9;
    let mut mage = caster(Side::Defender, 1, 2);
    mage.max_hp = 500;
    mage.hp = 50_000;
    // In front of its caster: no shot can reach it but the one it takes.
    let mut fam = at(Fighter::new(Side::Defender, 40), 1, 1);
    fam.actions = [RowActions::default(); ROWS];
    fam.magic.guards = Some(1);
    let f = battle(vec![a, mage, fam]);
    let out = run(&f, 6);
    let shots: Vec<FighterIdx> = out.events.iter().filter(|e| e.actor == 0 && e.kind == K::Shoot).filter_map(|e| e.target).collect();
    assert!(shots.len() >= 2, "{shots:?}");
    assert_eq!(shots[0], 2, "the familiar takes the first: {shots:?}");
    assert!(shots[1..].iter().all(|&t| t == 1), "then the caster: {shots:?}");
}

/// The Halo: its heal brings back the first fighter of its side to fall, at 25% HP, once; the kill
/// is undone.
#[test]
fn the_halo_brings_back_the_first_to_fall() {
    let mut killer = frontline(Side::Attacker, 1, 0);
    killer.speed = 9;
    let mut victim = frontline(Side::Defender, 1, 0);
    victim.hp = 100;
    let mut halo = at(Fighter::new(Side::Defender, 70), 1, 2);
    halo.actions = [row(K::Strike, 1), row(K::Heal, 1), row(K::Heal, 2)];
    halo.magic.revive = 25;
    let f = battle(vec![killer, victim, halo, frontline(Side::Defender, 0, 0)]);
    let out = run(&f, 1);
    let back = out.events.iter().find(|e| !e.entered.is_empty()).expect("a revive");
    assert_eq!((back.actor, back.kind), (2, K::Heal));
    assert_eq!(back.entered[0].unit, 1);
    assert_eq!(back.entered[0].hp, 120 * 25);
    assert_eq!(out.events.iter().filter(|e| !e.entered.is_empty()).count(), 1, "once");
    assert!(out.kills.iter().filter(|k| k.victim == 1).count() <= 1, "its first kill undone");
}

/// A Spellbreaker's hit ends the target's charms and its dispellable promotions' A for the
/// battle.
#[test]
fn a_spellbreakers_hit_dispels() {
    let mut breaker = frontline(Side::Attacker, 1, 0);
    breaker.magic.dispel = true;
    let mut buffed = frontline(Side::Defender, 1, 0);
    buffed.magic.bare = Some(Bare { a_pct: 50, dv_pct: 100, md_pct: 100, speed: -2 });
    let f = battle(vec![breaker, buffed]);
    let mut run = Run::new(&f, false);
    run.charm(1, 5, BattleCharm { damage: 30, ..BattleCharm::default() }, Event { round: 1, pass: 1, actor: 1, kind: K::Spell, spell: None, target: None, hits: vec![], heal: None, knocked: None, note: None, entered: vec![] });
    assert_eq!(run.s.dmg_mod[1], 30);
    let before = run.magic_hit_pct(1, 0, K::Strike);
    let speed = run.s.speed[1];
    run.round = 1;
    run.hit(0, 1, K::Strike, false, Extra::Target, 0, &mut crate::rng::Rng::new(1));
    assert!(run.s.mx.dispelled[1]);
    assert_eq!(run.s.dmg_mod[1], 0, "its charm ended");
    assert_eq!(run.s.speed[1], speed - 2);
    assert_eq!(before, 100);
    // Its A halved: 200 / (200 + 400) of 400 / 800, in percent.
    assert_eq!(run.magic_hit_pct(1, 0, K::Strike), 100i64 * 200 * 800 / (400 * 600));
}

/// Contingency's recall: a killing hit takes the fighter out of the battle, even a defender in a
/// city, with no roll.
#[test]
fn a_recall_leaves_in_place_of_the_kill() {
    let mut a = frontline(Side::Attacker, 1, 0);
    a.speed = 9;
    let mut mage = caster(Side::Defender, 1, 0);
    mage.hp = 100;
    mage.magic.recall = true;
    let ground = Ground { city: true, ..Ground::default() };
    let f = Formation::new(Params::default(), ground, [3, 3], vec![a, mage, frontline(Side::Defender, 0, 0)], |_, _| EQ);
    let out = run(&f, 9);
    assert!(matches!(out.fighters[1].fate, Fate::Withdrew { round: 1 }), "{:?}", out.fighters[1]);
    assert!(out.kills.iter().all(|k| k.victim != 1));
}

/// An opening cast (Contingency's strike): the fighter casts its damage spell before round 1.
#[test]
fn an_opening_cast_comes_before_the_first_round() {
    let mut mage = caster(Side::Defender, 1, 2);
    mage.magic.opening_cast = true;
    let f = battle(vec![frontline(Side::Attacker, 1, 0), frontline(Side::Defender, 1, 0), mage]);
    let out = run(&f, 3);
    let first = &out.events[0];
    assert_eq!((first.round, first.pass, first.actor, first.kind, first.spell), (0, 0, 2, K::Spell, Some(1)));
    assert_eq!(out.events.iter().filter(|e| e.round == 0).count(), 1);
}
