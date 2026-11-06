//! The formation resolver's rules, each on a small battle: the prototype's behaviours (reach per
//! kind, anti-flank first, the charge, the ground, misses, crits, knock-backs, withdrawal, the
//! siege floor, splash, heals, large units, the order of a pass), its draw order, determinism and
//! termination, and the auto formation.

use super::*;
use ffh_data::ActionKind as K;

/// A = Dv: an even hit's expected damage is `40 x 100 / 2` = 2000 centi-HP (20 HP).
const EQ: Pair = Pair { a: 400, dv: 400, ma: 400, md: 400, spell_pct: 100, pct: 100 };
const EVEN: i32 = 2000;

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

fn screen(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = frontline(side, lane, r);
    f.speed = 3;
    f.anti_flank = true;
    f
}

fn flanker(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 100), lane, r);
    f.speed = 6;
    f.actions = [row(K::Charge, 2), row(K::Strike, 1), row(K::Strike, 1)];
    f.evasion = 10;
    f.flanker = true;
    f
}

fn assassin(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 60), lane, r);
    f.speed = 7;
    f.actions = [row(K::Stab, 1), row(K::Stab, 2), row(K::Stab, 1)];
    f.evasion = 15;
    f.crit = 25;
    f
}

fn archer(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 80), lane, r);
    f.actions = [row(K::Shoot, 1), row(K::Shoot, 2), row(K::Shoot, 2)];
    f
}

fn siege(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 80), lane, r);
    f.speed = 1;
    f.actions = [row(K::None, 0), row(K::Bombard, 1), row(K::Bombard, 2)];
    f.combat_limit = 75;
    f
}

fn caster(side: Side, lane: u8, r: u8, spell: Spell) -> Fighter {
    let mut f = at(Fighter::new(side, 70), lane, r);
    f.speed = 3;
    f.actions = [row(K::Strike, 1), row(K::Spell, 1), row(K::Spell, 2)];
    f.spell = spell;
    f
}

fn healer(side: Side, lane: u8, r: u8) -> Fighter {
    let mut f = at(Fighter::new(side, 70), lane, r);
    f.speed = 3;
    f.actions = [row(K::Strike, 1), row(K::Heal, 1), row(K::Heal, 2)];
    f
}

fn large(mut f: Fighter) -> Fighter {
    f.large = true;
    f.max_hp = f.max_hp * 150 / 100;
    f.hp = f.max_hp * 100;
    f
}

fn hurt(mut f: Fighter, hp: i32) -> Fighter {
    f.hp = hp;
    f
}

const OPEN: Ground = Ground { city: false, forest: false, hills: false };
const FOREST: Ground = Ground { city: false, forest: true, hills: false };
const CITY: Ground = Ground { city: true, forest: false, hills: false };
const HILLS: Ground = Ground { city: false, forest: false, hills: true };

fn battle(ground: Ground, fighters: Vec<Fighter>) -> Formation {
    let b = Formation::new(Params::default(), ground, [4, 4], fighters, |_, _| EQ);
    assert_eq!(b.validate(), Ok(()));
    b
}

/// One round only (a battle's first).
fn one_round(mut b: Formation) -> Formation {
    b.params.rounds = 1;
    b
}

/// An RNG that answers each draw by its label (`f(label, n)`, clamped to `0..n`) and records the
/// labels.
struct Rigged<F: FnMut(&'static str, i32) -> i32> {
    f: F,
    labels: Vec<&'static str>,
}

impl<F: FnMut(&'static str, i32) -> i32> BattleRng for Rigged<F> {
    fn rand(&mut self, n: i32, label: &'static str) -> i32 {
        assert!((1..=0xFFFF).contains(&n), "{label}: {n} does not fit CvRandom");
        self.labels.push(label);
        (self.f)(label, n).clamp(0, n - 1)
    }
}

fn rigged<F: FnMut(&'static str, i32) -> i32>(f: F) -> Rigged<F> {
    Rigged { f, labels: Vec::new() }
}

/// Every physical action lands, nothing crits, r is 100, every withdrawal fails, every coin is 0
/// (equal speeds keep fighter order), no chance first strike is rolled.
fn sure(label: &'static str, n: i32) -> i32 {
    match label {
        "Engagement hit" | "Engagement heal" => 20,
        "Engagement order" | "Engagement first strikes" => 0,
        _ => n - 1,
    }
}

/// The events of the battle, played with `rng`.
fn play<R: BattleRng>(b: &Formation, rng: &mut R) -> Outcome {
    let out = resolve(b, rng);
    check_outcome(b, &out);
    out
}

/// What every outcome satisfies.
fn check_outcome(b: &Formation, out: &Outcome) {
    let n = b.len();
    assert!(out.rounds as i32 <= b.params.rounds);
    let mut kills = 0;
    for (i, e) in out.fighters.iter().enumerate() {
        assert!(e.hp >= 0 && e.hp <= b.fighters[i].max_hp * 100, "fighter {i}: hp {}", e.hp);
        assert!((e.row as usize) < ROWS && e.row >= b.fighters[i].row);
        match e.fate {
            Fate::Standing => assert!(e.hp > 0),
            Fate::Fell { by, .. } => {
                assert_eq!(e.hp, 0);
                assert_ne!(b.fighters[by as usize].side, b.fighters[i].side);
                kills += 1;
            }
            Fate::Withdrew { .. } => assert!(e.hp > 0),
        }
        let taken: i32 = (0..n).map(|h| out.dealt(h, i)).sum();
        assert_eq!(taken, e.taken);
        // HP at the end: the start, less what it took, plus what it was healed.
        assert_eq!(e.hp, b.fighters[i].hp - e.taken + e.was_healed, "fighter {i}");
    }
    assert_eq!(out.kills.len(), kills);
    for side in [Side::Attacker, Side::Defender] {
        let standing = (0..n).any(|i| b.fighters[i].side == side && out.fighters[i].fate == Fate::Standing);
        assert_eq!(out.down[side.index()], !standing);
    }
}

#[test]
fn lanes_from_slots() {
    for (slots, lanes) in [(3, 2), (4, 2), (5, 3), (6, 3), (7, 3), (8, 4), (9, 4), (10, 4), (11, 5), (13, 5), (14, 6)] {
        assert_eq!(lanes_for(slots), lanes, "{slots}");
    }
}

#[test]
fn reach_per_kind() {
    use Side::*;
    // The defender's grid (4 lanes): lane 0 front and middle, lane 1 middle and rear, lane 2 rear
    // only, lane 3 empty.
    let b = battle(
        OPEN,
        vec![
            frontline(Attacker, 0, 0),
            frontline(Defender, 0, 0), // 1
            frontline(Defender, 0, 1), // 2
            frontline(Defender, 1, 1), // 3
            frontline(Defender, 1, 2), // 4
            frontline(Defender, 2, 2), // 5
        ],
    );
    let s = State::new(&b);
    assert_eq!(b.reach(&s, 0, K::Strike), [1, 3, 5], "the front unit of each lane");
    for k in [K::Shoot, K::Spell, K::Bombard] {
        assert_eq!(b.reach(&s, 0, k), [2, 4, 5], "{k:?}: the backmost of each lane");
    }
    assert_eq!(b.reach(&s, 0, K::Stab), [1, 2, 3, 4, 5], "anyone");
    assert_eq!(b.reach(&s, 0, K::Charge), [1, 2, 3, 4, 5], "anyone where flanking works");
    assert!(b.reach(&s, 0, K::Heal).is_empty() && b.reach(&s, 0, K::None).is_empty());
    for g in [FOREST, CITY] {
        let mut b = b.clone();
        b.ground = g;
        assert_eq!(b.reach(&s, 0, K::Charge), [1, 3, 5], "{g:?}: a charge is a strike");
    }
    // The front falls: the unit behind it is the lane's front now.
    let mut s2 = s.clone();
    s2.alive[1] = false;
    assert_eq!(b.reach(&s2, 0, K::Strike), [2, 3, 5]);
    // The defender reaches the attacker's lone unit with every kind.
    for k in [K::Strike, K::Shoot, K::Stab, K::Charge] {
        assert_eq!(b.reach(&s, 1, k), [0], "{k:?}");
    }
}

#[test]
fn a_large_unit_holds_two_lanes() {
    use Side::*;
    let giant = large(frontline(Defender, 1, 1));
    let b = battle(
        OPEN,
        vec![
            frontline(Attacker, 0, 0),
            frontline(Defender, 1, 0), // 1: in front of the giant in lane 1
            giant,                     // 2: lanes 1 and 2, middle
            frontline(Defender, 0, 1), // 3: beside it
            frontline(Defender, 3, 1), // 4: beside it
            frontline(Defender, 2, 2), // 5: behind it in lane 2
            frontline(Defender, 0, 2), // 6: diagonal: not a neighbour
        ],
    );
    let s = State::new(&b);
    // Lane 1's front is unit 1, lane 2's the giant: it is in the strike's reach through lane 2.
    assert_eq!(b.reach(&s, 0, K::Strike), [1, 2, 3, 4]);
    // The backmost of lanes 0..3: 6, the giant (lane 1), 5 (lane 2), 4.
    assert_eq!(b.reach(&s, 0, K::Shoot), [2, 4, 5, 6]);
    assert_eq!(b.neighbours(&s, 2), [3, 1, 5, 4], "beside, in front and behind, in (lane, row) order");
    assert_eq!(b.neighbours(&s, 3), [6, 2], "unit 3 has unit 6 behind it and the giant beside it");
    assert_eq!(b.fighters[2].slots(), 2);
    // Grid checks: a large unit needs its second lane; two units never share a cell.
    let mut bad = b.clone();
    bad.fighters[2].lane = 3;
    assert_eq!(bad.validate(), Err(FormationError::OffGrid(2)));
    let mut bad = b.clone();
    bad.fighters[5].row = 1;
    assert_eq!(bad.validate(), Err(FormationError::Overlap(2, 5)));
    let mut bad = b.clone();
    bad.fighters[0].row = 3;
    assert_eq!(bad.validate(), Err(FormationError::OffGrid(0)));
    // A large unit is knocked back only when both cells behind it are free.
    let mut s3 = s.clone();
    let mut run = Run::new(&b, true);
    run.s = s3.clone();
    assert_eq!(run.knock_back(2), None, "unit 5 stands behind it in lane 2");
    s3.alive[5] = false;
    run.s = s3;
    assert_eq!(run.knock_back(2), Some(Knock { unit: 2, from: 1, to: 2 }));
}

#[test]
fn flankers_take_anti_flank_units_first() {
    use Side::*;
    // A wounded archer is the most dangerous unit for its HP; the screen is anti-flank.
    let b = battle(
        OPEN,
        vec![
            flanker(Attacker, 0, 0),
            frontline(Attacker, 1, 0),
            screen(Defender, 0, 0),             // 2
            hurt(archer(Defender, 2, 1), 2000), // 3
            frontline(Defender, 3, 0),          // 4
        ],
    );
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Charge);
    assert_eq!(reach, [2, 3, 4]);
    assert_eq!(b.choose(&s, 0, K::Charge, 1, &reach), Some(2), "the screen first");
    // Without the screen, the charge goes for the highest threat for its HP left.
    let mut s2 = s.clone();
    s2.alive[2] = false;
    let reach = b.reach(&s2, 0, K::Charge);
    assert_eq!(b.choose(&s2, 0, K::Charge, 1, &reach), Some(3));
    // A frontline unit is not drawn to screens: among the fronts of each lane (the archer is lane
    // 2's) it takes the kill on offer.
    let reach = b.reach(&s, 1, K::Strike);
    assert_eq!(reach, [2, 3, 4]);
    assert_eq!(b.choose(&s, 1, K::Strike, 1, &reach), Some(3), "a kill on offer: the archer at 20 HP");
    // A screen out of a flanker's reach does not draw it: in a forest the flanker strikes the
    // fronts, and a screen behind a frontline unit is not among them.
    let b = battle(
        FOREST,
        vec![flanker(Attacker, 0, 0), frontline(Defender, 0, 0), screen(Defender, 0, 1), hurt(frontline(Defender, 1, 0), 6000)],
    );
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Charge);
    assert_eq!(reach, [1, 3]);
    assert_eq!(b.choose(&s, 0, K::Charge, 1, &reach), Some(3), "the normal choice: the most hurt for its HP");
}

#[test]
fn the_charge_and_its_first_round_bonus_only_where_flanking_works() {
    use Side::*;
    let fs = vec![flanker(Attacker, 0, 0), frontline(Defender, 0, 0)];
    let b = battle(OPEN, fs.clone());
    assert_eq!(b.expected(0, 1, K::Charge, 1, false), 2 * EVEN, "doubled in the first round");
    assert_eq!(b.expected(0, 1, K::Charge, 2, false), EVEN);
    assert_eq!(b.expected(0, 1, K::Strike, 1, false), EVEN);
    // Played: the first round's charges are charges and deal double (r 100).
    let out = play(&one_round(b.clone()), &mut rigged(sure));
    let charges: Vec<&Event> = out.events.iter().filter(|e| e.actor == 0).collect();
    assert_eq!(charges.len(), 2);
    for e in charges {
        assert_eq!((e.kind, e.hits[0].dmg), (K::Charge, 2 * EVEN));
    }
    // In a forest or a city it is a strike: front reach, no bonus.
    for g in [FOREST, CITY] {
        let b = battle(g, fs.clone());
        assert_eq!(b.expected(0, 1, K::Strike, 1, false), EVEN);
        let out = play(&one_round(b), &mut rigged(sure));
        let e = out.events.iter().find(|e| e.actor == 0).unwrap();
        assert_eq!((e.kind, e.hits[0].dmg), (K::Strike, EVEN), "{g:?}");
    }
    // The charge's target: the highest threat (A x actions from its row) for its HP left. An
    // archer in the middle row (2 actions) at full HP (8000) against a frontline unit in the front
    // row (2 actions) at 12000: 800/8000 > 800/12000.
    let b = battle(OPEN, vec![flanker(Attacker, 0, 0), frontline(Defender, 0, 0), archer(Defender, 1, 1), siege(Defender, 2, 0)]);
    let s = State::new(&b);
    assert_eq!(b.threat(&s, 1), 800);
    assert_eq!(b.threat(&s, 2), 800);
    assert_eq!(b.threat(&s, 3), 0, "a siege engine in the front row does nothing");
    let reach = b.reach(&s, 0, K::Charge);
    assert_eq!(b.choose(&s, 0, K::Charge, 1, &reach), Some(2));
    let mut s2 = s.clone();
    s2.row[2] = 0; // from the front row an archer shoots once: 400/8000 < 800/12000
    assert_eq!(b.choose(&s2, 0, K::Charge, 1, &reach), Some(1));
}

#[test]
fn the_ground() {
    use Side::*;
    let fs = vec![
        archer(Attacker, 0, 1),
        siege(Attacker, 1, 2),
        frontline(Attacker, 2, 0),
        caster(Attacker, 3, 2, Spell::known(7)),
        archer(Defender, 0, 1),
        siege(Defender, 1, 2),
        frontline(Defender, 2, 0),
    ];
    // (ground, attacker's shoot, bombard, strike, spell; defender's shoot, bombard, strike)
    for (g, want) in [
        (OPEN, [EVEN, EVEN, EVEN, EVEN, EVEN, EVEN, EVEN]),
        (CITY, [EVEN / 2, EVEN, EVEN, EVEN, EVEN, EVEN, EVEN]),
        (FOREST, [EVEN / 2, EVEN / 2, EVEN, EVEN, EVEN / 2, EVEN / 2, EVEN]),
        (HILLS, [1500, 1500, 1500, 1500, 2500, 2500, EVEN]),
        // several multiply: 2000 x 50% x 75% = 750; 2000 x 50% x 125% = 1250
        (Ground { forest: true, hills: true, city: false }, [750, 750, 1500, 1500, 1250, 1250, EVEN]),
        (Ground { city: true, hills: true, forest: false }, [750, 1500, 1500, 1500, 2500, 2500, EVEN]),
    ] {
        let b = battle(g, fs.clone());
        let got = [
            b.expected(0, 6, K::Shoot, 1, false),
            b.expected(1, 6, K::Bombard, 1, false),
            b.expected(2, 6, K::Strike, 1, false),
            b.expected(3, 6, K::Spell, 1, false),
            b.expected(4, 2, K::Shoot, 1, false),
            b.expected(5, 2, K::Bombard, 1, false),
            b.expected(6, 2, K::Strike, 1, false),
        ];
        assert_eq!(got, want, "{g:?}");
    }
    // A spell's power: a known spell 100%, the bolt 60%; a splash 50% of the hit.
    let mut b = battle(OPEN, fs.clone());
    assert_eq!(b.expected(3, 6, K::Spell, 1, true), EVEN / 2);
    b.fighters[3].spell = Spell::bolt(&b.params);
    assert_eq!(b.expected(3, 6, K::Spell, 1, false), EVEN * 60 / 100);
    // A and Dv: 600 against 200 is 40 x 100 x 600 / 800.
    let b = Formation::new(Params::default(), OPEN, [4, 4], fs, |h, _| if h == 2 { Pair::physical(600, 200) } else { EQ });
    assert_eq!(b.expected(2, 6, K::Strike, 1, false), 3000);
}

/// A missile strikes from outside the field at `missile_pct` (50) of a hit whatever the ground,
/// takes the kill its own hit offers, and looks for no nearby unit.
#[test]
fn a_missile_s_strike() {
    use Side::*;
    let fs = vec![
        frontline(Attacker, 0, 0),
        hurt(frontline(Defender, 0, 0), 1500), // a field strike's kill (2000), not a missile's
        hurt(frontline(Defender, 3, 0), 900),  // the missile's kill (1000), three lanes off
        frontline(Defender, 1, 0),
    ];
    for g in [OPEN, CITY, FOREST, HILLS] {
        let b = battle(g, fs.clone());
        assert_eq!((b.missile_expected(0, 1), b.missile_expected(0, 2)), (EVEN / 2, EVEN / 2), "{g:?}");
    }
    let b = battle(OPEN, fs);
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Strike);
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &reach), Some(1), "on the field: nearby first, its kill");
    assert_eq!(b.choose_with(&s, 0, K::Strike, 1, &reach, true), Some(2));
    let mut rng = rigged(sure);
    let hit = missile_hit(&b, 0, &mut rng).expect("a target");
    assert_eq!((hit.target, hit.expected, hit.dmg, hit.killed), (2, EVEN / 2, 900, true));
    assert_eq!(rng.labels, ["Engagement hit"], "r only");
}

#[test]
fn misses_and_crits() {
    use Side::*;
    let mut big_dodge = frontline(Defender, 3, 0);
    big_dodge.evasion = 90;
    let b = battle(
        OPEN,
        vec![frontline(Attacker, 0, 0), assassin(Attacker, 1, 1), frontline(Defender, 0, 0), flanker(Defender, 1, 0), assassin(Defender, 2, 0), big_dodge],
    );
    // (kind, target) -> miss chance: 100 less the hitter's accuracy (90) less the target's evasion
    // and the kind's (shoot 10, bombard 15), at most MAX_CHANCE.
    assert_eq!(b.miss_chance(K::Strike, 0, 2), 10);
    assert_eq!(b.miss_chance(K::Strike, 0, 3), 20);
    assert_eq!(b.miss_chance(K::Shoot, 0, 4), 35);
    assert_eq!(b.miss_chance(K::Bombard, 0, 4), 40);
    assert_eq!(b.miss_chance(K::Shoot, 0, 5), MAX_CHANCE);
    // A sharper eye hits more: accuracy 105 against evasion 15 hits a shot 80%.
    let mut b2 = b.clone();
    b2.fighters[0].accuracy = 105;
    assert_eq!(b2.miss_chance(K::Shoot, 0, 4), 20);
    assert_eq!((b.crit_chance(0), b.crit_chance(1)), (10, 25));
    // A miss: no crit roll, no r, no damage.
    let b = one_round(battle(OPEN, vec![frontline(Attacker, 0, 0), frontline(Defender, 0, 0)]));
    let mut rng = rigged(|label, n| if label == "Engagement miss" { 0 } else { sure(label, n) });
    let out = play(&b, &mut rng);
    assert!(out.events.iter().all(|e| e.hits.len() == 1 && e.hits[0].miss && e.hits[0].dmg == 0));
    assert!(!rng.labels.contains(&"Engagement crit") && !rng.labels.contains(&"Engagement hit"));
    // A crit: 150% of the hit.
    let mut rng = rigged(|label, n| if label == "Engagement crit" { 0 } else { sure(label, n) });
    let out = play(&b, &mut rng);
    let e = &out.events[0];
    assert!(e.hits[0].crit);
    assert_eq!(e.hits[0].dmg, EVEN * 3 / 2);
    // Spells never miss and never crit; bombardments miss but never crit.
    let b = one_round(battle(OPEN, vec![caster(Attacker, 0, 2, Spell::known(1)), siege(Attacker, 1, 2), frontline(Defender, 0, 0)]));
    let mut rng = rigged(sure);
    let out = play(&b, &mut rng);
    let kinds: Vec<K> = out.events.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&K::Spell) && kinds.contains(&K::Bombard));
    let misses = rng.labels.iter().filter(|&&l| l == "Engagement miss").count();
    let bombards = kinds.iter().filter(|&&k| k == K::Bombard).count();
    let strikes = kinds.iter().filter(|&&k| k == K::Strike).count();
    assert_eq!(misses, bombards + strikes, "only the physical actions roll a miss");
    assert_eq!(rng.labels.iter().filter(|&&l| l == "Engagement crit").count(), strikes, "only the strikes roll a crit");
}

#[test]
fn knock_back_only_with_free_cells() {
    use Side::*;
    let crit = |label: &'static str, n: i32| if label == "Engagement crit" { 0 } else { sure(label, n) };
    // A strike's crit on a front unit with a free cell behind it: it goes back a row, and acts
    // from there (one action from the middle row instead of two from the front).
    let mut atk = frontline(Attacker, 0, 0);
    atk.speed = 9;
    atk.actions[0].count = 1;
    let mut def = frontline(Defender, 0, 0);
    def.actions[0].count = 2;
    let b = one_round(battle(OPEN, vec![atk.clone(), def.clone()]));
    let out = play(&b, &mut rigged(crit));
    assert_eq!(out.events[0].knocked, Some(Knock { unit: 1, from: 0, to: 1 }));
    assert_eq!(out.fighters[1].row, 1);
    assert_eq!(out.events.iter().filter(|e| e.actor == 1).count(), 1, "it acts once, from the middle row");
    // A unit behind it: no knock-back.
    let b = one_round(battle(OPEN, vec![atk.clone(), def.clone(), frontline(Defender, 0, 1)]));
    let out = play(&b, &mut rigged(crit));
    assert_eq!(out.events[0].knocked, None);
    // The rear row: nowhere to go.
    let mut rear = def.clone();
    rear.row = 2;
    let b = one_round(battle(OPEN, vec![atk.clone(), rear]));
    assert_eq!(play(&b, &mut rigged(crit)).events[0].knocked, None);
    // Only melee crits knock back: a stab's crit does not.
    let mut stabber = assassin(Attacker, 0, 0);
    stabber.speed = 9;
    let b = one_round(battle(OPEN, vec![stabber, def.clone()]));
    let out = play(&b, &mut rigged(crit));
    assert!(out.events[0].hits[0].crit);
    assert_eq!(out.events[0].knocked, None);
    // A charge's crit does.
    let mut rider = flanker(Attacker, 0, 0);
    rider.speed = 9;
    let b = one_round(battle(OPEN, vec![rider, def]));
    assert_eq!(play(&b, &mut rigged(crit)).events[0].knocked, Some(Knock { unit: 1, from: 0, to: 1 }));
}

#[test]
fn withdrawal_replaces_the_killing_hit_and_not_in_a_city() {
    use Side::*;
    let mut atk = frontline(Attacker, 0, 0);
    atk.speed = 9;
    atk.actions[0].count = 1;
    let mut runner = hurt(frontline(Defender, 0, 0), 1500);
    runner.withdraw_pct = 40;
    let flee = |label: &'static str, n: i32| if label == "Engagement withdraw" { 0 } else { sure(label, n) };
    // The killing hit is replaced: the unit leaves alive with its HP.
    let b = one_round(battle(OPEN, vec![atk.clone(), runner.clone()]));
    let mut rng = rigged(flee);
    let out = play(&b, &mut rng);
    let h = &out.events[0].hits[0];
    assert!(h.withdrew && !h.killed && h.dmg == 0);
    assert_eq!(out.fighters[1].fate, Fate::Withdrew { round: 1 });
    assert_eq!(out.fighters[1].hp, 1500);
    assert!(out.down[1], "nobody is left on the defender's field");
    assert_eq!(out.events.len(), 1, "the battle ends at once");
    // The roll fails (39 < 40 succeeds, 40 does not): killed.
    let mut rng = rigged(|label, n| if label == "Engagement withdraw" { 40 } else { sure(label, n) });
    let out = play(&b, &mut rng);
    assert_eq!(out.fighters[1].fate, Fate::Fell { by: 0, round: 1 });
    assert_eq!(out.events[0].hits[0].dmg, 1500, "a killing hit counts the HP the target had");
    // A hit that does not kill rolls nothing.
    let b2 = one_round(battle(OPEN, vec![atk.clone(), hurt(runner.clone(), 9000)]));
    let mut rng = rigged(flee);
    play(&b2, &mut rng);
    assert!(!rng.labels.contains(&"Engagement withdraw"));
    // A city's defenders never withdraw; its attackers may.
    let b = one_round(battle(CITY, vec![atk.clone(), runner.clone()]));
    let mut rng = rigged(flee);
    let out = play(&b, &mut rng);
    assert_eq!(out.fighters[1].fate, Fate::Fell { by: 0, round: 1 });
    assert!(!rng.labels.contains(&"Engagement withdraw"));
    let mut atk_runner = hurt(frontline(Attacker, 0, 0), 1500);
    atk_runner.withdraw_pct = 40;
    let mut killer = frontline(Defender, 0, 0);
    killer.speed = 9;
    let b = one_round(battle(CITY, vec![atk_runner, killer]));
    assert_eq!(play(&b, &mut rigged(flee)).fighters[0].fate, Fate::Withdrew { round: 1 });
}

#[test]
fn siege_never_takes_a_target_below_its_limit() {
    use Side::*;
    // A combat limit of 75: never below 25% of max HP (3000 of a frontline unit's 12000). The
    // defenders barely scratch it (A 1 against Dv 400).
    let fs = vec![siege(Attacker, 0, 2), hurt(frontline(Defender, 0, 0), 4000), hurt(frontline(Defender, 1, 0), 3000)];
    let b = Formation::new(Params::default(), OPEN, [4, 4], fs, |h, _| if h == 0 { EQ } else { Pair::physical(1, 400) });
    let s = State::new(&b);
    assert_eq!(b.floor(0, 1), 3000);
    assert_eq!(b.reach(&s, 0, K::Bombard), [1], "a target at the limit is not in its reach");
    // Its hit stops at the limit: unit 1 goes from 4000 to 3000, not to 2000.
    let out = play(&one_round(b.clone()), &mut rigged(sure));
    let mine: Vec<&Event> = out.events.iter().filter(|e| e.actor == 0).collect();
    assert_eq!(mine.len(), 2, "two bombardments from the rear row");
    assert_eq!(mine[0].hits[0].dmg, 1000);
    assert_eq!(out.fighters[1].hp, 3000);
    assert_eq!(out.fighters[1].fate, Fate::Standing);
    assert_eq!(mine[1].note, Some(Note::NoTarget), "its second bombardment finds none");
    // It takes no kill: a target its hit would "kill" is at most taken to the limit, so the kill
    // rule (the highest A) does not choose; the hit that hurts most for the HP left does.
    let mut weak = hurt(frontline(Defender, 0, 0), 3500);
    weak.a = 100;
    let mut strong = hurt(frontline(Defender, 1, 0), 3800);
    strong.a = 900;
    let b = Formation::new(Params::default(), OPEN, [4, 4], vec![siege(Attacker, 0, 2), weak, strong], |_, _| EQ);
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Bombard);
    // capped: unit 1 500 of 3500, unit 2 800 of 3800: 2 is hurt more for its HP
    assert_eq!(b.choose(&s, 0, K::Bombard, 1, &reach), Some(2));
}

#[test]
fn splash() {
    use Side::*;
    // The target (the most hurt for its HP) in lane 1, front; around it 2 and 3 beside, 4 behind;
    // 5 diagonal (lane 2, middle): not a neighbour.
    let mut berserker = frontline(Attacker, 0, 0);
    berserker.speed = 9;
    berserker.actions[0].count = 1;
    berserker.collateral_limit = Some(100);
    let fs = vec![
        berserker,
        hurt(frontline(Defender, 1, 0), 11000), // 1
        frontline(Defender, 0, 0),              // 2
        frontline(Defender, 2, 0),              // 3
        frontline(Defender, 1, 1),              // 4
        frontline(Defender, 2, 1),              // 5
    ];
    let b = one_round(battle(OPEN, fs));
    let s = State::new(&b);
    assert_eq!(b.reach(&s, 0, K::Strike), [1, 2, 3]);
    assert_eq!(b.neighbours(&s, 1), [2, 4, 3]);
    // Collateral damage: each neighbour takes 50% of a hit; a crit on the target is not a crit on
    // the splash.
    let crit = |label: &'static str, n: i32| if label == "Engagement crit" { 0 } else { sure(label, n) };
    let mut rng = rigged(crit);
    let out = play(&b, &mut rng);
    let e = &out.events[0];
    assert_eq!(e.target, Some(1));
    let got: Vec<(FighterIdx, i32, bool, bool)> = e.hits.iter().map(|h| (h.target, h.dmg, h.crit, h.splash)).collect();
    assert_eq!(got, [(1, 3000, true, false), (2, 1000, false, true), (4, 1000, false, true), (3, 1000, false, true)]);
    assert_eq!(e.knocked, None, "unit 4 stands behind it");
    // Each splashed unit draws its own r, after the target's: miss, crit, then four r's.
    let first: Vec<&str> = rng.labels.iter().copied().skip_while(|&l| l == "Engagement order").take(6).collect();
    assert_eq!(first, ["Engagement miss", "Engagement crit", "Engagement hit", "Engagement hit", "Engagement hit", "Engagement hit"]);
    // The collateral limit: never below it (a limit of 50: 6000 of 12000).
    let mut b2 = b.clone();
    b2.fighters[0].collateral_limit = Some(50);
    b2.fighters[1].hp = 5000;
    b2.fighters[4].hp = 6500;
    let out = play(&b2, &mut rigged(sure));
    let e = &out.events[0];
    assert_eq!(e.target, Some(1));
    assert_eq!(e.hits[0].dmg, EVEN, "the target itself is hit in full");
    assert_eq!(e.hits.iter().find(|h| h.target == 4).map(|h| h.dmg), Some(500));
    let mut b3 = b2.clone();
    b3.fighters[4].hp = 6000;
    let out = play(&b3, &mut rigged(sure));
    assert!(out.events[0].hits.iter().all(|h| h.target != 4), "a unit at the limit is not splashed");
    // A known spell splashes (50%), and its hits can kill; the bolt hits one target (60%).
    let mut b4 = b.clone();
    b4.fighters[0] = caster(Attacker, 0, 2, Spell::known(3));
    b4.fighters[0].speed = 9;
    b4.fighters[0].actions[2].count = 1;
    b4.fighters[2].hp = 900;
    let s4 = State::new(&b4);
    // the backmost of each lane: 2 (lane 0), 4 (lane 1, middle), 5 (lane 2, middle)
    assert_eq!(b4.reach(&s4, 0, K::Spell), [2, 4, 5]);
    let out = play(&b4, &mut rigged(sure));
    let e = &out.events[0];
    assert_eq!((e.kind, e.spell, e.target), (K::Spell, Some(3), Some(2)), "the kill on offer");
    let got: Vec<(FighterIdx, i32, bool, bool)> = e.hits.iter().map(|h| (h.target, h.dmg, h.killed, h.splash)).collect();
    assert_eq!(got, [(2, 900, true, false), (1, 1000, false, true)]);
    let mut b5 = b4.clone();
    b5.fighters[0].spell = Spell::bolt(&b5.params);
    b5.fighters[2].hp = 12000;
    let out = play(&b5, &mut rigged(sure));
    let e = &out.events[0];
    assert_eq!((e.spell, e.target, e.hits.len(), e.hits[0].dmg), (None, Some(2), 1, EVEN * 60 / 100));
}

/// Barrage's collateral extra and Drill's collateral protection scale a collateral splash (not
/// the hit on the target, not a known spell's splash); full protection keeps a unit out of it.
#[test]
fn barrage_and_drill_scale_the_collateral_splash() {
    use Side::*;
    let mut berserker = frontline(Attacker, 0, 0);
    berserker.speed = 9;
    berserker.actions[0].count = 1;
    berserker.collateral_limit = Some(100);
    berserker.collateral_extra = 100; // Barrage III
    let mut drilled = frontline(Defender, 0, 0);
    drilled.collateral_protection = 40; // Drill III and IV
    let mut proof = frontline(Defender, 2, 0);
    proof.collateral_protection = 100;
    let fs = vec![
        berserker,
        hurt(frontline(Defender, 1, 0), 11000), // 1: the target (the most hurt)
        drilled,                                // 2: beside it
        proof,                                  // 3: beside it
        frontline(Defender, 1, 1),              // 4: behind it
    ];
    let b = one_round(battle(OPEN, fs));
    assert_eq!(b.expected(0, 1, K::Strike, 1, false), EVEN, "the target's hit is a hit");
    // A splash: 50%, x 200% (Barrage III); x 60% on the drilled unit; nothing through 100%.
    assert_eq!(b.expected(0, 4, K::Strike, 1, true), EVEN);
    assert_eq!(b.expected(0, 2, K::Strike, 1, true), EVEN * 60 / 100);
    assert_eq!(b.expected(0, 3, K::Strike, 1, true), 0);
    let mut rng = rigged(sure);
    let out = play(&b, &mut rng);
    let e = &out.events[0];
    assert_eq!(e.target, Some(1));
    let got: Vec<(FighterIdx, i32, bool)> = e.hits.iter().map(|h| (h.target, h.dmg, h.splash)).collect();
    assert_eq!(got, [(1, EVEN, false), (2, EVEN * 60 / 100, true), (4, EVEN, true)], "3 is not splashed");
    // ...and draws nothing for 3: miss, crit, the target's r, two splash r's, then the next
    // mover's miss.
    let first: Vec<&str> = rng.labels.iter().copied().skip_while(|&l| l == "Engagement order").take(6).collect();
    assert_eq!(first, ["Engagement miss", "Engagement crit", "Engagement hit", "Engagement hit", "Engagement hit", "Engagement miss"]);
    // Without Barrage the plain 50%.
    let mut plain = b.clone();
    plain.fighters[0].collateral_extra = 0;
    assert_eq!(plain.expected(0, 4, K::Strike, 1, true), EVEN / 2);
    assert_eq!(plain.expected(0, 2, K::Strike, 1, true), EVEN / 2 * 60 / 100);
    // A known spell splashes as the spell: neither Barrage nor Drill counts, and the protected
    // unit is splashed too.
    let mut mage = b.clone();
    mage.fighters[0] = caster(Attacker, 0, 2, Spell::known(3));
    mage.fighters[0].collateral_limit = Some(100);
    mage.fighters[0].collateral_extra = 100;
    assert!(!mage.collateral_splash(0, K::Spell));
    assert!(mage.collateral_splash(0, K::Strike), "its strikes splash as collateral");
    for t in [2, 3, 4] {
        assert_eq!(mage.expected(0, t, K::Spell, 1, true), EVEN / 2, "{t}");
    }
    // The magic bolt of a unit with collateral damage splashes as collateral damage.
    mage.fighters[0].spell = Spell::bolt(&mage.params);
    assert!(mage.collateral_splash(0, K::Spell));
}

#[test]
fn heals_the_most_hurt_ally() {
    use Side::*;
    let mut priest = healer(Attacker, 0, 1);
    priest.speed = 9;
    let b = one_round(battle(
        OPEN,
        vec![
            priest,
            hurt(frontline(Attacker, 1, 0), 11000), // 1000 missing
            hurt(frontline(Attacker, 2, 0), 9000),  // 3000 missing
            hurt(archer(Attacker, 3, 1), 5000),     // 3000 missing: a tie, the lower lane first
            Fighter::new(Defender, 120),
        ],
    ));
    let s = State::new(&b);
    assert_eq!(b.heal_target(&s, 0), Some(2));
    let out = play(&b, &mut rigged(sure));
    let e = &out.events[0];
    // base 40 x 50% x r 100: 2000 centi-HP.
    assert_eq!((e.kind, e.heal), (K::Heal, Some(Heal { target: 2, amount: 2000, r: 100 })));
    // Capped at what is missing.
    let mut b2 = b.clone();
    for i in 1..=3 {
        b2.fighters[i].hp = b2.fighters[i].max_hp * 100;
    }
    b2.fighters[1].hp = 11500;
    let out = play(&b2, &mut rigged(sure));
    assert_eq!(out.events[0].heal, Some(Heal { target: 1, amount: 500, r: 100 }));
    // No one hurt: the action is spent, and draws nothing.
    b2.fighters[1].hp = 12000;
    let mut rng = rigged(sure);
    let out = play(&b2, &mut rng);
    assert_eq!((out.events[0].note, out.events[0].heal), (Some(Note::NoOneHurt), None));
    assert!(!rng.labels.contains(&"Engagement heal"));
}

#[test]
fn the_order_of_a_pass() {
    use Side::*;
    // Faster first, always; equal speeds by the coin, both ways over seeds.
    let mut slow = frontline(Attacker, 0, 0);
    slow.speed = 3;
    let fast = flanker(Defender, 0, 0);
    let b = one_round(battle(OPEN, vec![slow.clone(), fast]));
    for seed in 0..20 {
        let out = play(&b, &mut Rng::new(seed));
        assert_eq!(out.events[0].actor, 1);
    }
    let even = one_round(battle(OPEN, vec![frontline(Attacker, 0, 0), frontline(Defender, 0, 0)]));
    let firsts: Vec<FighterIdx> = (0..40).map(|seed| play(&even, &mut Rng::new(seed)).events[0].actor).collect();
    assert!(firsts.contains(&0) && firsts.contains(&1), "{firsts:?}");
    // The coin decides: the lower coin first.
    let coins = |a: i32, d: i32| {
        let mut k = 0;
        move |label: &'static str, n: i32| {
            if label == "Engagement order" {
                k += 1;
                if k % 2 == 1 {
                    a
                } else {
                    d
                }
            } else {
                sure(label, n)
            }
        }
    };
    assert_eq!(play(&even, &mut rigged(coins(5, 9))).events[0].actor, 0);
    assert_eq!(play(&even, &mut rigged(coins(9, 5))).events[0].actor, 1);
    assert_eq!(play(&even, &mut rigged(coins(7, 7))).events[0].actor, 0, "equal coins: fighter order");
    // First strikes add speed, rolled chance first strikes too.
    let mut quick = frontline(Attacker, 0, 0);
    quick.first_strikes = 1;
    quick.chance_first_strikes = 2;
    let b = one_round(battle(OPEN, vec![quick, frontline(Defender, 0, 0)]));
    let out = play(&b, &mut rigged(|label, n| if label == "Engagement first strikes" { 1 } else { sure(label, n) }));
    assert_eq!((out.fighters[0].speed, out.fighters[0].rolled_first_strikes), (4 + 2 * 2, 1));
    assert_eq!(out.fighters[1].speed, 4);
    // Passes: pass 1 everyone with an action, pass 2 those with two, ...; a unit acts once a
    // pass. The frontline units have 2 actions from the front: passes 1 and 2.
    let out = play(&even, &mut rigged(sure));
    let passes: Vec<(u8, u8, FighterIdx)> = out.events.iter().map(|e| (e.round, e.pass, e.actor)).collect();
    assert_eq!(passes, [(1, 1, 0), (1, 1, 1), (1, 2, 0), (1, 2, 1)]);
    // A tier 4 unit's third action is a third pass.
    let mut elite = frontline(Attacker, 0, 0);
    elite.actions[0].count = 3;
    let b = one_round(battle(OPEN, vec![elite, frontline(Defender, 0, 0)]));
    let out = play(&b, &mut rigged(sure));
    assert_eq!(out.events.last().map(|e| (e.pass, e.actor)), Some((3, 0)));
}

#[test]
fn a_unit_killed_before_its_turn_does_not_act() {
    use Side::*;
    let mut a = frontline(Attacker, 0, 0);
    a.speed = 9;
    a.actions[0].count = 1;
    let b = one_round(battle(OPEN, vec![a, hurt(frontline(Defender, 0, 0), 1000), frontline(Defender, 1, 0)]));
    let out = play(&b, &mut rigged(sure));
    assert_eq!(out.fighters[1].fate, Fate::Fell { by: 0, round: 1 });
    assert!(out.events.iter().all(|e| e.actor != 1));
    assert_eq!(out.kills, [Kill { victim: 1, killer: 0, round: 1 }]);
    assert_eq!(out.dealt(0, 1), 1000);
}

#[test]
fn the_draw_order() {
    use Side::*;
    // Per battle the chance first strikes (in fighter order), per pass the coins (in fighter
    // order), per action miss, crit, r, the withdrawal, then the splash's r's.
    let mut a = frontline(Attacker, 0, 0);
    a.chance_first_strikes = 1;
    a.collateral_limit = Some(100);
    a.speed = 9;
    a.actions[0].count = 1;
    let mut d0 = hurt(frontline(Defender, 0, 0), 500);
    d0.withdraw_pct = 50;
    let mut d1 = frontline(Defender, 1, 0);
    d1.chance_first_strikes = 1;
    let b = one_round(battle(OPEN, vec![a, d0, d1]));
    let mut rng = rigged(sure);
    play(&b, &mut rng);
    assert_eq!(
        &rng.labels[..9],
        [
            "Engagement first strikes",
            "Engagement first strikes",
            "Engagement order",
            "Engagement order",
            "Engagement order",
            "Engagement miss",
            "Engagement crit",
            "Engagement hit",
            "Engagement withdraw",
        ]
    );
    assert_eq!(rng.labels[9], "Engagement hit", "the splash on unit 2");
}

/// A random battle: up to 10 units a side of every role, some large, some hurt, on random
/// ground, with random A and Dv.
fn random_battle(seed: u32) -> Formation {
    let mut g = Rng::new(seed);
    let mut r = |n: i32| g.get_i(n, "test");
    let ground = Ground { city: r(3) == 0, forest: r(3) == 0, hills: r(3) == 0 };
    let lanes = [2 + r(4) as u8, 2 + r(4) as u8];
    let mut fighters = Vec::new();
    for side in [Side::Attacker, Side::Defender] {
        let lanes_n = lanes[side.index()];
        let mut taken = vec![[false; ROWS]; lanes_n as usize];
        for _ in 0..r(11) + 1 {
            let (lane, rw) = (r(lanes_n as i32) as u8, r(3) as u8);
            let mut f = match r(8) {
                0 => frontline(side, lane, rw),
                1 => screen(side, lane, rw),
                2 => flanker(side, lane, rw),
                3 => assassin(side, lane, rw),
                4 => archer(side, lane, rw),
                5 => siege(side, lane, rw),
                6 => caster(side, lane, rw, if r(2) == 0 { Spell::known(r(5) as u32) } else { Spell::bolt(&Params::default()) }),
                _ => healer(side, lane, rw),
            };
            if r(6) == 0 && lane + 1 < lanes_n {
                f = large(f);
            }
            let cells: Vec<u8> = f.lanes().collect();
            if cells.iter().any(|&l| taken[l as usize][rw as usize]) {
                continue;
            }
            for &l in &cells {
                taken[l as usize][rw as usize] = true;
            }
            if r(3) == 0 {
                f.hp = 1 + r(f.max_hp * 100);
            }
            f.withdraw_pct = [0, 0, 25, 80][r(4) as usize];
            f.first_strikes = r(2);
            f.chance_first_strikes = r(2);
            if r(5) == 0 {
                f.collateral_limit = Some(50 + r(51));
            }
            if r(8) == 0 {
                f.actions[f.actions.iter().position(|a| a.count == f.actions.iter().map(|a| a.count).max().unwrap()).unwrap()].count += 1;
            }
            f.a = 100 + r(1000);
            f.tag = fighters.len() as u32;
            fighters.push(f);
        }
    }
    let strength: Vec<(i32, i32)> = (0..fighters.len()).map(|_| (100 + r(900), 100 + r(900))).collect();
    let b = Formation::new(Params::default(), ground, lanes, fighters, |h, t| Pair::physical(strength[h].0, strength[t].1));
    assert_eq!(b.validate(), Ok(()));
    b
}

#[test]
fn every_battle_ends_and_the_outcome_holds_together() {
    let mut total_events = 0;
    let mut ended_early = 0;
    for seed in 0..600 {
        let b = random_battle(seed);
        let out = play(&b, &mut Rng::new(seed.wrapping_mul(2654435761)));
        // Bounded: per round at most one action per action of every unit (+1 each for a unit
        // knocked into a richer row is not possible: passes are counted at the round's start).
        let per_round: i32 = b.fighters.iter().map(|f| f.actions.iter().map(|a| a.count).max().unwrap()).sum();
        assert!(out.events.len() as i32 <= per_round * b.params.rounds, "seed {seed}");
        total_events += out.events.len();
        if out.down[0] || out.down[1] {
            ended_early += 1;
            // No event after the action that cleared a side.
            let last = out.events.last().unwrap();
            assert!(last.hits.iter().any(|h| h.killed || h.withdrew), "seed {seed}");
        }
        for e in &out.events {
            let actor = &b.fighters[e.actor as usize];
            for h in &e.hits {
                assert_ne!(b.fighters[h.target as usize].side, actor.side, "seed {seed}");
                assert!(!(h.miss && (h.crit || h.dmg != 0)), "seed {seed}");
                assert!(!(h.killed && h.withdrew), "seed {seed}");
                if h.withdrew {
                    assert!(!(b.ground.city && b.fighters[h.target as usize].side == Side::Defender), "seed {seed}");
                }
            }
            if let Some(hl) = e.heal {
                assert_eq!(b.fighters[hl.target as usize].side, actor.side);
            }
        }
    }
    assert!(total_events > 10_000, "{total_events}");
    assert!(ended_early > 100, "{ended_early}");
}

#[test]
fn determinism_and_the_quiet_resolver() {
    for seed in 0..200 {
        let b = random_battle(seed + 1000);
        let (mut r1, mut r2, mut r3) = (Rng::new(seed), Rng::new(seed), Rng::new(seed));
        let a = resolve(&b, &mut r1);
        let again = resolve(&b, &mut r2);
        assert_eq!(a, again, "seed {seed}");
        let quiet = resolve_quiet(&b, &mut r3);
        assert!(quiet.events.is_empty());
        assert_eq!((&quiet.fighters, &quiet.kills, quiet.rounds, quiet.down), (&a.fighters, &a.kills, a.rounds, a.down), "seed {seed}");
        assert_eq!(r1, r3, "the same draws");
        // Every draw fits the game's CvRandom (u16 ranges).
        let mut rec = rigged(|_, n| (seed as i32 * 7919) % n);
        resolve(&b, &mut rec);
    }
}

#[test]
fn a_side_with_nobody_fights_no_round() {
    let b = Formation::new(Params::default(), OPEN, [4, 4], vec![frontline(Side::Attacker, 0, 0)], |_, _| EQ);
    let mut rng = rigged(sure);
    let out = resolve(&b, &mut rng);
    assert_eq!((out.rounds, out.down, out.events.len()), (0, [false, true], 0));
    assert!(rng.labels.is_empty());
}

#[test]
fn the_kill_choice_and_its_ties() {
    use Side::*;
    // A kill on offer: the most dangerous (highest A) of the units the hit would kill, though
    // another is hurt more for its HP.
    let mut weak = hurt(frontline(Defender, 0, 0), 1000);
    weak.a = 300;
    let mut strong = hurt(frontline(Defender, 1, 0), 1900);
    strong.a = 900;
    let b = battle(OPEN, vec![frontline(Attacker, 0, 0), weak, strong, frontline(Defender, 2, 0)]);
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Strike);
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &reach), Some(2));
    // No kill: the most hurt for its HP left (2000 of 8000 beats 2000 of 12000).
    let b = battle(OPEN, vec![frontline(Attacker, 0, 0), frontline(Defender, 0, 0), hurt(frontline(Defender, 1, 0), 8000)]);
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Strike);
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &reach), Some(2));
    // Ties: the higher A, then the lane, then the row; never the list order (the hitter in lane
    // 1: both lanes nearby).
    let mut b = battle(OPEN, vec![frontline(Attacker, 1, 0), frontline(Defender, 2, 0), frontline(Defender, 1, 0)]);
    let s = State::new(&b);
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &[1, 2]), Some(2));
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &[2, 1]), Some(2));
    b.fighters[1].a = 500;
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &[2, 1]), Some(1));
}

/// Nearby first (spec 1.5, the owner's rule): the enemies in reach within one lane of the hitter
/// come first, the rest only when none of them is in reach; a large unit's lanes are both its
/// lanes; a charge where flanking works goes anywhere; a flanker takes anti-flank units first,
/// then the rule; heals do not follow it; a splash's centre does.
#[test]
fn nearby_first() {
    use Side::*;
    // A strike from lane 0: the wounded unit in lane 3 would die to it, the units of lanes 0 and 1
    // would not. The nearby ones come first: the more hurt for its HP of lanes 0 and 1.
    let b = battle(
        OPEN,
        vec![
            frontline(Attacker, 0, 0),
            frontline(Defender, 0, 0),             // 1
            hurt(frontline(Defender, 1, 0), 9000), // 2
            hurt(frontline(Defender, 3, 0), 1000), // 3: a kill on offer, three lanes away
        ],
    );
    let s = State::new(&b);
    let reach = b.reach(&s, 0, K::Strike);
    assert_eq!(reach, [1, 2, 3]);
    assert_eq!((b.lane_distance(0, 1), b.lane_distance(0, 2), b.lane_distance(0, 3)), (0, 1, 3));
    assert_eq!(b.choose(&s, 0, K::Strike, 1, &reach), Some(2));
    // Shots and stabs too; with nobody nearby in reach, the rest: the kill in lane 3.
    for k in [K::Shoot, K::Stab] {
        let reach = b.reach(&s, 0, k);
        assert_eq!(b.choose(&s, 0, k, 1, &reach), Some(2), "{k:?}");
    }
    let mut s2 = s.clone();
    s2.alive[1] = false;
    s2.alive[2] = false;
    let reach = b.reach(&s2, 0, K::Strike);
    assert_eq!(reach, [3]);
    assert_eq!(b.choose(&s2, 0, K::Strike, 1, &reach), Some(3));
    // From lane 2 every unit but lane 0's is nearby: the kill on offer in lane 3.
    let mut b2 = b.clone();
    b2.fighters[0].lane = 2;
    let reach = b2.reach(&s, 0, K::Strike);
    assert_eq!(b2.choose(&s, 0, K::Strike, 1, &reach), Some(3));
    // A large hitter in lanes 1-2 reaches lanes 0-3 nearby; a large target in lanes 2-3 is one
    // lane from a hitter in lane 1.
    let mut b3 = battle(OPEN, vec![large(frontline(Attacker, 1, 0)), frontline(Defender, 0, 0), hurt(frontline(Defender, 3, 0), 1000)]);
    let s3 = State::new(&b3);
    assert_eq!((b3.lane_distance(0, 1), b3.lane_distance(0, 2)), (1, 1));
    let reach = b3.reach(&s3, 0, K::Strike);
    assert_eq!(b3.choose(&s3, 0, K::Strike, 1, &reach), Some(2), "both lanes nearby: the kill in lane 3");
    b3.fighters[0].large = false;
    b3.fighters[0].lane = 0;
    assert_eq!(b3.choose(&s3, 0, K::Strike, 1, &reach), Some(1), "from lane 0 alone, lane 3 is far");
    let b4 = battle(OPEN, vec![frontline(Attacker, 1, 0), large(frontline(Defender, 2, 0)), hurt(frontline(Defender, 0, 1), 100)]);
    assert_eq!(b4.lane_distance(0, 1), 1);
    // Sides of different lane counts: lanes are numbered the same on both. An attacker in lane 1
    // of 2 lanes and a defender's lanes 0..3: lanes 0, 1, 2 are nearby.
    let b5 = Formation::new(
        Params::default(),
        OPEN,
        [2, 4],
        vec![frontline(Attacker, 1, 0), hurt(frontline(Defender, 3, 0), 1000), frontline(Defender, 2, 0), frontline(Defender, 0, 0)],
        |_, _| EQ,
    );
    assert_eq!(b5.validate(), Ok(()));
    let s5 = State::new(&b5);
    let reach = b5.reach(&s5, 0, K::Strike);
    assert_eq!(reach, [1, 2, 3]);
    assert_eq!(b5.choose(&s5, 0, K::Strike, 1, &reach), Some(3), "lanes 0 and 2 are nearby, lane 3 (the kill) is not; the tie by lane");
    // A charge where flanking works keeps going for its target wherever it is: the highest threat
    // for its HP left, three lanes away. In a forest it is a strike, and nearby first holds.
    let fs = vec![flanker(Attacker, 0, 0), frontline(Defender, 0, 0), hurt(archer(Defender, 3, 1), 2000)];
    let b6 = battle(OPEN, fs.clone());
    let s6 = State::new(&b6);
    let reach = b6.reach(&s6, 0, K::Charge);
    assert_eq!(b6.choose(&s6, 0, K::Charge, 1, &reach), Some(2));
    let b7 = battle(FOREST, vec![flanker(Attacker, 0, 0), frontline(Defender, 0, 0), hurt(frontline(Defender, 3, 0), 1000)]);
    let s7 = State::new(&b7);
    let reach = b7.reach(&s7, 0, K::Charge);
    assert_eq!(reach, [1, 2]);
    assert_eq!(b7.choose(&s7, 0, K::Charge, 1, &reach), Some(1));
    // A flanker takes anti-flank units first, wherever, then nearby first among them; its middle
    // row's strike likewise.
    let b8 = battle(
        FOREST,
        vec![flanker(Attacker, 0, 0), hurt(frontline(Defender, 0, 0), 1000), hurt(screen(Defender, 3, 0), 6000), screen(Defender, 1, 0)],
    );
    let s8 = State::new(&b8);
    let reach = b8.reach(&s8, 0, K::Charge);
    assert_eq!(reach, [1, 2, 3]);
    assert_eq!(b8.choose(&s8, 0, K::Charge, 1, &reach), Some(3), "the screens first, and of them the nearby one");
    let mut s8b = s8.clone();
    s8b.alive[3] = false;
    let reach = b8.reach(&s8b, 0, K::Charge);
    assert_eq!(b8.choose(&s8b, 0, K::Charge, 1, &reach), Some(2), "a screen three lanes away before a kill in its own lane");
    // Heals do not follow it: the most hurt ally three lanes away.
    let b9 = battle(OPEN, vec![healer(Attacker, 0, 1), hurt(frontline(Attacker, 1, 0), 11000), hurt(frontline(Attacker, 3, 0), 6000), frontline(Defender, 0, 0)]);
    assert_eq!(b9.heal_target(&State::new(&b9), 0), Some(2));
    // A splash's centre: nearby first. A caster in lane 0 with a known spell: the backmost of
    // lanes 0 and 1 before the lone wounded unit of lane 3.
    let b10 = battle(
        OPEN,
        vec![caster(Attacker, 0, 2, Spell::known(1)), frontline(Defender, 0, 0), frontline(Defender, 1, 0), frontline(Defender, 1, 1), hurt(frontline(Defender, 3, 0), 1000)],
    );
    let s10 = State::new(&b10);
    let reach = b10.reach(&s10, 0, K::Spell);
    assert_eq!(reach, [1, 3, 4]);
    assert_eq!(b10.choose(&s10, 0, K::Spell, 1, &reach), Some(1));
}

/// The rank order of a unit placed by a home row, as the data's orders are (the rows behind before
/// those in front): home, +1, -1, +2, -2.
fn order_from(home: u8) -> [u8; ROWS] {
    let h = home as i32;
    let rows: Vec<u8> = [h, h + 1, h - 1, h + 2, h - 2].into_iter().filter(|r| (0..ROWS as i32).contains(r)).map(|r| r as u8).collect();
    [rows[0], rows[1], rows[2]]
}

fn cand(pick_order: i32, home_row: u8, value: i64) -> Candidate {
    Candidate { pick_order, row_order: order_from(home_row), large: false, value, healthy: true, river: true }
}

#[test]
fn the_auto_formation() {
    // Roles by pick order: 1 screen (front), 2 frontline (front), 3 archer (middle), 4 flanker
    // (front), 7 siege (rear).
    let cands = vec![
        cand(2, 0, 40),  // 0 frontline
        cand(2, 0, 60),  // 1 frontline, stronger
        cand(3, 1, 30),  // 2 archer
        cand(3, 1, 50),  // 3 archer, stronger
        cand(4, 0, 40),  // 4 flanker
        cand(7, 2, 40),  // 5 siege
        cand(2, 0, 50),  // 6 frontline
        Candidate { large: true, ..cand(2, 0, 45) }, // 7 a giant: counts double (90), 2 slots
    ];
    // 4 slots: one of each role first, in the role order, the strongest of each (the giant, archer
    // 3, flanker 4, siege 5: 5 slots, the siege no longer fits), then the rest in the role order.
    let p = auto_arrange(&cands, 2, 4, 4);
    let units: Vec<usize> = p.iter().map(|x| x.unit).collect();
    assert_eq!(units, [3, 4, 7]);
    // The giant first, in the front row (both lanes of 2); then the archer in its home row (the
    // middle; equal distances to the centre: the lower lane); the flanker's home row is full: the
    // nearest row with room, behind.
    let cell = |u: usize| p.iter().find(|x| x.unit == u).map(|x| (x.lane, x.row));
    assert_eq!(cell(7), Some((0, 0)));
    assert_eq!(cell(3), Some((0, 1)));
    assert_eq!(cell(4), Some((1, 1)));
    // 10 slots, 4 lanes: everyone (9 slots), placed the large first, then in the order taken
    // (3, 4, 5, 1, 6, 0, 2); the centre lanes first (1 and 2, then 0 and 3).
    let p = auto_arrange(&cands, 4, 10, 10);
    assert_eq!(p.len(), 8);
    let cell = |u: usize| p.iter().find(|x| x.unit == u).map(|x| (x.lane, x.row));
    assert_eq!(cell(7), Some((1, 0)), "the giant in the centre lanes 1-2");
    assert_eq!(cell(3), Some((1, 1)));
    assert_eq!(cell(4), Some((0, 0)));
    assert_eq!(cell(5), Some((1, 2)));
    assert_eq!(cell(1), Some((3, 0)));
    assert_eq!(cell(6), Some((2, 1)), "the front is full: the middle row, centre first");
    assert_eq!(cell(0), Some((0, 1)));
    assert_eq!(cell(2), Some((3, 1)));
    // Units placed never overlap, and stay inside the grid.
    let mut cells = std::collections::BTreeSet::new();
    for x in &p {
        for l in x.lane..=x.lane + cands[x.unit].large as u8 {
            assert!(l < 4 && x.row < 3);
            assert!(cells.insert((l, x.row)));
        }
    }
    // Healthy units first within a role: the hurt strong frontline waits for the healthy ones.
    let mut c2 = cands.clone();
    c2[1].healthy = false;
    let p = auto_arrange(&c2[..3], 4, 2, 2);
    assert_eq!(p.iter().map(|x| x.unit).collect::<Vec<_>>(), [0, 2]);
    // Large units are placed first: with 2 lanes a large flanker takes the front row, and the
    // frontline units go behind it.
    let c3 = vec![cand(2, 0, 50), cand(2, 0, 40), Candidate { large: true, ..cand(4, 0, 10) }];
    let p = auto_arrange(&c3, 2, 4, 4);
    assert_eq!(p.iter().map(|x| (x.unit, x.lane, x.row)).collect::<Vec<_>>(), [(0, 0, 1), (1, 1, 1), (2, 0, 0)]);
    // No room: left out (3 slots of large units on 2 lanes x 3 rows: three fit, a fourth does not).
    let c4 = vec![Candidate { large: true, ..cand(2, 0, 50) }; 4];
    assert_eq!(auto_arrange(&c4, 2, 8, 8).len(), 3);
    // The river rule: units that cannot cross freely fill at most the ordinary slots; the river
    // slots go to those that can, whatever their place in the order.
    let mut c5 = vec![cand(2, 0, 60), cand(2, 0, 50), cand(2, 0, 40), cand(3, 1, 10)];
    c5[0].river = false;
    c5[1].river = false;
    c5[2].river = false;
    let p = auto_arrange(&c5, 4, 4, 2);
    assert_eq!(p.iter().map(|x| x.unit).collect::<Vec<_>>(), [0, 1, 3], "two ordinary slots, the river slot to the archer");
    c5[0].large = true;
    let p = auto_arrange(&c5, 4, 4, 2);
    assert_eq!(p.iter().map(|x| x.unit).collect::<Vec<_>>(), [0, 3], "a large unit fills both ordinary slots");
}

/// A unit of the prototype's scenarios (`scenarios.js`): role, strength, defence, level, tier,
/// first strikes, withdrawal, combat limit, large, a known spell.
#[derive(Clone, Copy)]
struct Proto {
    role: usize,
    str_: i32,
    def: i32,
    level: i32,
    fs: i32,
    wd: i32,
    lim: i32,
    large: bool,
    spell: bool,
}

// The prototype's roles in its ROLES order: frontline, screen, flanker, assassin, archer, siege,
// arcane, divine; (A %, Dv %, HP, speed, rows, dodge, crit bonus, anti-flank, auto-pick, home row).
type ProtoRole = (i32, i32, i32, i32, [(K, i32); 3], i32, i32, bool, i32, u8);
const PROTO_ROLES: [ProtoRole; 8] = [
    (100, 100, 120, 4, [(K::Strike, 2), (K::Strike, 1), (K::Strike, 1)], 0, 0, false, 2, 0),
    (100, 110, 120, 3, [(K::Strike, 2), (K::Strike, 1), (K::Strike, 1)], 0, 0, true, 1, 0),
    (110, 80, 100, 6, [(K::Charge, 2), (K::Strike, 1), (K::Strike, 1)], 10, 0, false, 4, 0),
    (120, 60, 60, 7, [(K::Stab, 1), (K::Stab, 2), (K::Stab, 1)], 15, 15, false, 8, 1),
    (100, 60, 80, 4, [(K::Shoot, 1), (K::Shoot, 2), (K::Shoot, 2)], 0, 0, false, 3, 1),
    (100, 60, 80, 1, [(K::None, 0), (K::Bombard, 1), (K::Bombard, 2)], 0, 0, false, 7, 2),
    (100, 50, 70, 3, [(K::Strike, 1), (K::Spell, 1), (K::Spell, 2)], 0, 0, false, 5, 2),
    (100, 50, 70, 3, [(K::Strike, 1), (K::Heal, 1), (K::Heal, 2)], 0, 0, false, 6, 1),
];

fn proto(name: &str, level: i32) -> Proto {
    let u = |role, str_, def| Proto { role, str_, def, level, fs: 0, wd: 0, lim: 100, large: false, spell: false };
    match name {
        "champion" => u(0, 6, 6),
        "axeman" => u(0, 4, 4),
        "swordsman" => u(1, 4, 4),
        "hunter" => u(0, 4, 4),
        "ranger" => u(0, 7, 7),
        "hillgiant" => Proto { large: true, ..u(0, 7, 7) },
        "archer" => Proto { fs: 1, ..u(4, 3, 5) },
        "longbow" => Proto { fs: 1, ..u(4, 5, 6) },
        "horseman" => Proto { fs: 1, wd: 25, ..u(2, 4, 4) },
        "horsearcher" => Proto { fs: 1, wd: 35, ..u(2, 6, 6) },
        "elephant" => Proto { wd: 25, large: true, ..u(2, 7, 7) },
        "assassin" => Proto { fs: 1, ..u(3, 5, 3) },
        "adept" => u(6, 3, 3),
        "mage" => Proto { spell: true, ..u(6, 4, 4) },
        "priest" => u(7, 4, 4),
        "disciple" => u(7, 3, 3),
        "catapult" => Proto { wd: 80, lim: 75, ..u(5, 4, 3) },
        _ => panic!("{name}"),
    }
}

/// The prototype's scenario `key` as a formation: its armies auto-arranged on its ground.
fn proto_scenario(key: &str) -> (Formation, Vec<String>) {
    let army = |list: &[(&str, i32)]| list.iter().map(|&(n, l)| proto(n, l)).collect::<Vec<Proto>>();
    let (ground, slots, att, def) = match key {
        "open" => (
            OPEN,
            [10, 10],
            army(&[("champion", 4), ("axeman", 1), ("axeman", 1), ("swordsman", 2), ("longbow", 1), ("longbow", 1), ("elephant", 1), ("horseman", 1), ("mage", 1), ("priest", 1), ("catapult", 1), ("assassin", 1)]),
            army(&[("ranger", 3), ("champion", 1), ("hillgiant", 1), ("hunter", 2), ("swordsman", 2), ("longbow", 1), ("longbow", 1), ("archer", 1), ("horsearcher", 1), ("adept", 1), ("disciple", 1)]),
        ),
        "forest" => (
            FOREST,
            [6, 6],
            army(&[("champion", 4), ("axeman", 1), ("axeman", 1), ("swordsman", 2), ("longbow", 1), ("longbow", 1), ("horseman", 1), ("mage", 1), ("priest", 1), ("assassin", 1)]),
            army(&[("ranger", 3), ("axeman", 1), ("hunter", 1), ("archer", 1), ("archer", 1), ("horsearcher", 1), ("disciple", 1)]),
        ),
        _ => (
            CITY,
            [8, 10],
            army(&[("champion", 4), ("axeman", 1), ("axeman", 1), ("swordsman", 2), ("longbow", 1), ("longbow", 1), ("mage", 1), ("catapult", 1), ("catapult", 1), ("priest", 1), ("assassin", 1)]),
            army(&[("ranger", 3), ("axeman", 1), ("axeman", 1), ("archer", 1), ("archer", 1), ("archer", 1), ("adept", 1), ("disciple", 1)]),
        ),
    };
    let lanes = [lanes_for(slots[0]), lanes_for(slots[1])];
    let mut fighters = Vec::new();
    let mut units: Vec<Proto> = Vec::new();
    let mut shown = Vec::new();
    for (side, list) in [(Side::Attacker, &att), (Side::Defender, &def)] {
        let cands: Vec<Candidate> = list
            .iter()
            .map(|u| {
                let r = &PROTO_ROLES[u.role];
                Candidate { pick_order: r.8, row_order: order_from(r.9), large: u.large, value: (u.str_ * (100 + 10 * (u.level - 1))) as i64, healthy: true, river: true }
            })
            .collect();
        let tag = if side == Side::Attacker { "att" } else { "def" };
        for p in auto_arrange(&cands, lanes[side.index()], slots[side.index()], slots[side.index()]) {
            let u = list[p.unit];
            let r = &PROTO_ROLES[u.role];
            shown.push(format!("{tag}:{}@{},{}", p.unit, p.lane, p.row));
            let max_hp = r.2 * if u.large { 150 } else { 100 } / 100;
            let mut f = at(Fighter::new(side, max_hp), p.lane, p.row);
            f.large = u.large;
            f.speed = r.3;
            f.first_strikes = u.fs;
            f.actions = r.4.map(|(kind, count)| row(kind, count));
            f.a = u.str_ * r.0 * (100 + 10 * (u.level - 1)) / 100;
            f.evasion = r.5;
            f.crit = 10 + r.6;
            f.anti_flank = r.7;
            f.flanker = r.4.iter().any(|x| x.0 == K::Charge);
            f.withdraw_pct = u.wd;
            f.combat_limit = u.lim;
            // the prototype's bombardment always splashes, on the engine's own floor
            f.collateral_limit = (u.role == 5).then_some(u.lim);
            f.spell = if u.spell { Spell::known(1) } else { Spell::bolt(&Params::default()) };
            fighters.push(f);
            units.push(u);
        }
    }
    let dv = |u: &Proto| u.def * PROTO_ROLES[u.role].1 * (100 + 10 * (u.level - 1)) / 100;
    let b = Formation::new(Params::default(), ground, lanes, fighters.clone(), |h, t| Pair::physical(fighters[h].a, dv(&units[t])));
    assert_eq!(b.validate(), Ok(()));
    (b, shown)
}

/// The prototype's three scenarios against its own numbers (`run()` over 4000 battles, its defaults:
/// base 40, three rounds, dice, knock-backs). The auto formation places every unit where its
/// `autoArrange` does. The losses and withdrawals per side are close: the resolver is the
/// prototype made integer (centi-HP instead of rounding to whole HP, the coin's range, A as a
/// charge's first tie-break), with two rules of the spec's: a splash's centre is chosen by the
/// unit's normal rule (the prototype took the centre with the most damage in all) and a siege
/// engine takes no kill (its hit cannot kill). With the prototype's two rules instead, every number
/// came within 2% (before nearby first, 2026-10-31: open 3.63 / 5.22, forest 2.38 / 3.47, city
/// 4.26 / 4.64 dead; withdrawn within 0.01). The numbers below are the prototype's with nearby
/// first (engine.js `nearFirst`, 2026-10-31); against them, with the spec's two rules, open is
/// within 2% (2.93 / 5.60 dead), the forest battle loses 12% fewer attackers (2.52 / 2.88) and the
/// city 7% fewer attackers and 3% more defenders (3.64 / 5.53; focused spells and stones kill
/// sooner, and a withdrawal more often comes too late).
#[test]
fn the_prototypes_scenarios() {
    // (scenario, the prototype's placements, its dead attackers, dead defenders, withdrawn
    // attackers, withdrawn defenders per battle)
    let want = [
        ("open", "att:6@1,0 att:3@0,0 att:0@3,0 att:4@1,1 att:8@1,2 att:9@2,1 att:10@2,2 att:11@0,1 att:1@3,1 def:2@1,0 def:4@0,0 def:5@1,1 def:8@3,0 def:9@1,2 def:10@2,1 def:0@0,1 def:1@3,1 def:3@2,2", 2.882, 5.622, 0.014, 0.355),
        ("forest", "att:3@1,0 att:0@0,0 att:4@1,1 att:6@2,0 att:7@1,2 att:8@0,1 def:0@1,0 def:3@1,1 def:5@0,0 def:6@0,1 def:1@2,0 def:2@2,1", 2.856, 2.744, 0.228, 0.321),
        ("city", "att:3@1,0 att:0@2,0 att:4@1,1 att:6@1,2 att:9@2,1 att:7@2,2 att:10@0,1 att:1@0,0 def:0@1,0 def:3@1,1 def:6@1,2 def:7@2,1 def:1@2,0 def:2@0,0 def:4@0,1 def:5@3,1", 3.912, 5.375, 0.612, 0.0),
    ];
    for (key, placed, dead_a, dead_d, wd_a, wd_d) in want {
        let (b, shown) = proto_scenario(key);
        let mut theirs: Vec<&str> = placed.split(' ').collect();
        theirs.sort_unstable();
        let mut mine: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
        mine.sort_unstable();
        assert_eq!(mine, theirs, "{key}: the auto formation");
        let n = 4000;
        let (mut dead, mut gone) = ([0; 2], [0; 2]);
        for seed in 1..=n {
            let out = resolve_quiet(&b, &mut Rng::new(seed * 7919 + 13));
            for (f, e) in b.fighters.iter().zip(&out.fighters) {
                match e.fate {
                    Fate::Fell { .. } => dead[f.side.index()] += 1,
                    Fate::Withdrew { .. } => gone[f.side.index()] += 1,
                    Fate::Standing => {}
                }
            }
        }
        let per = |x: i32| x as f64 / n as f64;
        let got = [per(dead[0]), per(dead[1]), per(gone[0]), per(gone[1])];
        for (g, w, what) in [(got[0], dead_a, "dead attackers"), (got[1], dead_d, "dead defenders")] {
            assert!((g - w).abs() <= 0.15 * w + 0.15, "{key}: {what} {g:.3}, the prototype's {w}");
        }
        for (g, w, what) in [(got[2], wd_a, "withdrawn attackers"), (got[3], wd_d, "withdrawn defenders")] {
            assert!((g - w).abs() <= 0.4 * w + 0.1, "{key}: {what} {g:.3}, the prototype's {w}");
        }
    }
}

/// The pair's type percentage (damage against a unit combat, resistance) and the hitter's round
/// percentage multiply each hit (docs/UNIT_ROLES.md, L1, L2, L12).
#[test]
fn type_and_round_percentages() {
    use Side::*;
    let mut axe = frontline(Attacker, 0, 0);
    axe.speed = 9;
    axe.actions[0].count = 1;
    axe.round_pct = [150, 100, 100];
    let fs = vec![axe, frontline(Defender, 0, 0)];
    let b = Formation::new(Params::default(), OPEN, [4, 4], fs, |h, _| if h == 0 { Pair { pct: 200, ..EQ } } else { EQ });
    assert_eq!(b.expected(0, 1, K::Strike, 1, false), EVEN * 2 * 3 / 2);
    assert_eq!(b.expected(0, 1, K::Strike, 2, false), EVEN * 2);
    assert_eq!(b.expected(0, 1, K::Strike, 3, false), EVEN * 2, "round 3 and later: the last value");
    assert_eq!(b.expected(1, 0, K::Strike, 1, false), EVEN);
    let out = play(&one_round(b), &mut rigged(sure));
    assert_eq!(out.events[0].hits[0].dmg, EVEN * 3);
}

/// Each attack pattern's extra targets and their hits at the pattern's percentage (no crit, the
/// target's roll; docs/UNIT_ROLES.md, "Attack patterns").
#[test]
fn attack_patterns() {
    use Side::*;
    // The defender's grid: lanes 0-3 front (1-4), lane 1 middle (5) and rear (6), lane 2 rear (7).
    let def = || {
        vec![
            frontline(Defender, 0, 0), // 1
            frontline(Defender, 1, 0), // 2
            frontline(Defender, 2, 0), // 3
            frontline(Defender, 3, 0), // 4
            frontline(Defender, 1, 1), // 5
            frontline(Defender, 1, 2), // 6
            frontline(Defender, 2, 2), // 7
        ]
    };
    let mut hitter = frontline(Attacker, 1, 0);
    hitter.speed = 9;
    hitter.actions[0].count = 1;
    let b = battle(OPEN, std::iter::once(hitter.clone()).chain(def()).collect());
    let s = State::new(&b);
    let t = |p| b.pattern_targets(&s, 2, K::Strike, p);
    assert_eq!(t(AttackPattern::Single), Vec::<usize>::new());
    assert_eq!(t(AttackPattern::Cleave), [1, 3], "the front units beside lane 1");
    assert_eq!(t(AttackPattern::Pierce), [5, 6], "everyone behind it in its lane");
    assert_eq!(t(AttackPattern::Sweep), [1, 3, 4], "its row");
    assert_eq!(t(AttackPattern::Trample), [5], "the nearest behind it");
    // A volley of shots on lane 1's backmost (6): the backmost of lanes 0 and 2.
    assert_eq!(b.pattern_targets(&s, 6, K::Shoot, AttackPattern::Volley), [1, 7]);
    // A cleaving strike: the target in full, its neighbours at the cleave's 60%.
    let mut cleaver = hitter.clone();
    cleaver.patterns = [AttackPattern::Cleave; ROWS];
    // Unit 2 a little hurt: the choice (the most hurt for its HP).
    let hurt2 = || def().into_iter().enumerate().map(|(i, f)| if i == 1 { hurt(f, 11000) } else { f }).collect::<Vec<_>>();
    let b2 = one_round(battle(OPEN, std::iter::once(cleaver).chain(hurt2()).collect()));
    let out = play(&b2, &mut rigged(sure));
    let e = &out.events[0];
    let got: Vec<(FighterIdx, i32, bool)> = e.hits.iter().map(|h| (h.target, h.dmg, h.splash)).collect();
    assert_eq!(got, [(2, EVEN, false), (1, EVEN * 60 / 100, true), (3, EVEN * 60 / 100, true)]);
    // A pattern takes the place of a collateral splash.
    let mut both = hitter.clone();
    both.patterns = [AttackPattern::Trample; ROWS];
    both.collateral_limit = Some(100);
    let b3 = one_round(battle(OPEN, std::iter::once(both).chain(hurt2()).collect()));
    let out = play(&b3, &mut rigged(sure));
    assert_eq!(out.events[0].hits.iter().map(|h| h.target).collect::<Vec<_>>(), [2, 5]);
}

/// Armor takes HP off each physical hit (at least 1 centi-HP gets through), not off a spell's;
/// piercing ignores it. Pursuit denies the withdrawal.
#[test]
fn armor_piercing_and_pursuit() {
    use Side::*;
    let mut axe = frontline(Attacker, 0, 0);
    axe.speed = 9;
    axe.actions[0].count = 1;
    let mut target = frontline(Defender, 0, 0);
    target.armor = 500;
    let b = one_round(battle(OPEN, vec![axe.clone(), target.clone()]));
    let out = play(&b, &mut rigged(sure));
    assert_eq!(out.events[0].hits[0].dmg, EVEN - 500);
    let mut b2 = b.clone();
    b2.fighters[1].armor = 100_000;
    let out = play(&b2, &mut rigged(sure));
    assert_eq!(out.events[0].hits[0].dmg, 1, "at least 1 gets through");
    let mut b3 = b.clone();
    b3.fighters[0].pierce = true;
    let out = play(&b3, &mut rigged(sure));
    assert_eq!(out.events[0].hits[0].dmg, EVEN);
    let mut mage = caster(Attacker, 0, 2, Spell::known(1));
    mage.speed = 9;
    mage.actions[2].count = 1;
    let b4 = one_round(battle(OPEN, vec![mage, target]));
    let out = play(&b4, &mut rigged(sure));
    assert_eq!(out.events[0].hits[0].dmg, EVEN, "a spell is not physical");
    // Pursuit: a killing hit on a unit that would withdraw (every withdrawal roll succeeds).
    let mut runner = hurt(frontline(Defender, 0, 0), 1000);
    runner.withdraw_pct = 100;
    let flee = |label: &'static str, n: i32| if label == "Engagement withdraw" { 0 } else { sure(label, n) };
    let b5 = one_round(battle(OPEN, vec![axe.clone(), runner]));
    let out = play(&b5, &mut rigged(flee));
    assert!(out.events[0].hits[0].withdrew);
    let mut b6 = b5.clone();
    b6.fighters[0].pursuit = true;
    let mut rng = rigged(flee);
    let out = play(&b6, &mut rng);
    assert!(out.events[0].hits[0].killed, "no escape from a pursuer");
    assert!(!rng.labels.contains(&"Engagement withdraw"));
}

/// A unit with a target preference goes for its prey wherever it stands in its reach (no nearby
/// first), and only then for the usual choice.
#[test]
fn target_preference() {
    use Side::*;
    let mut stabber = assassin(Attacker, 0, 1);
    stabber.speed = 9;
    stabber.actions[1].count = 1;
    let mut rider = frontline(Defender, 3, 0);
    rider.class_bit = 1 << 2;
    let fs = vec![stabber.clone(), hurt(frontline(Defender, 0, 0), 3000), rider.clone()];
    let b = one_round(battle(OPEN, fs.clone()));
    let s = State::new(&b);
    let list = b.reach(&s, 0, K::Stab);
    assert_eq!(b.choose(&s, 0, K::Stab, 1, &list), Some(1), "nearby first, the kill");
    let mut b2 = b.clone();
    b2.fighters[0].prefer = 1 << 2;
    assert_eq!(b2.choose(&s, 0, K::Stab, 1, &list), Some(2), "the prey, three lanes away");
}

/// A unit's own heal percentage, and a sweeping heal: every other hurt ally of the target's row
/// for the sweep's percentage of the heal, one event each.
#[test]
fn heal_power_and_sweep() {
    use Side::*;
    let mut priest = healer(Attacker, 0, 1);
    priest.speed = 9;
    priest.heal_pct = Some(100);
    let fs = vec![
        priest.clone(),
        hurt(frontline(Attacker, 1, 0), 6000),
        hurt(frontline(Attacker, 2, 0), 10000),
        hurt(frontline(Attacker, 3, 1), 6000),
        Fighter::new(Defender, 120),
    ];
    let b = one_round(battle(OPEN, fs));
    let out = play(&b, &mut rigged(sure));
    assert_eq!(out.events[0].heal, Some(Heal { target: 1, amount: 4000, r: 100 }), "40 x 100%");
    assert!(out.events[1].kind != K::Heal || out.events[1].actor != 0, "one heal");
    let mut b2 = b.clone();
    b2.fighters[0].patterns = [AttackPattern::Sweep; ROWS];
    let out = play(&b2, &mut rigged(sure));
    let heals: Vec<Heal> = out.events.iter().filter(|e| e.actor == 0 && e.round == 1 && e.pass == 1).filter_map(|e| e.heal).collect();
    // The target (lane 1, front) in full; the other hurt unit of the front row (lane 2) at 35%;
    // lane 3's unit stands in the middle row.
    assert_eq!(heals, [Heal { target: 1, amount: 4000, r: 100 }, Heal { target: 2, amount: 1400, r: 100 }]);
}

/// Two large units that prefer the front fit side by side in a row of 4 lanes (lanes 0-1 and
/// 2-3); a lone one takes the centre (lanes 1-2).
#[test]
fn large_units_side_by_side() {
    let big = |value| Candidate { large: true, ..cand(4, 0, value) };
    let p = auto_arrange(&[big(50), big(40)], 4, 10, 10);
    let cells: Vec<(usize, u8, u8)> = p.iter().map(|x| (x.unit, x.lane, x.row)).collect();
    assert_eq!(cells, [(0, 0, 0), (1, 2, 0)]);
    let p = auto_arrange(&[big(50)], 4, 10, 10);
    assert_eq!((p[0].lane, p[0].row), (1, 0));
    // Four fill two rows; none is left out.
    assert_eq!(auto_arrange(&[big(50), big(40), big(30), big(20)], 4, 10, 10).len(), 4);
}
