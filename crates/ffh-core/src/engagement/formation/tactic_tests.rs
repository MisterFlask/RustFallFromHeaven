//! The battle tactics' selection and placement on hand-built pools (docs/TACTICS.md, section 3),
//! and Line against the auto formation as it was before the tactics.

use super::*;
use ffh_data::{TacticKind as AK, TacticKnobs, TacticPlacement};

/// The auto formation as it was before the tactics (`auto_arrange_around` of 2026-11-08), kept as
/// the reference Line must reproduce.
fn reference(cands: &[Candidate], lanes: u8, slots: i32, ordinary: i32, blocked: &[(u8, u8)]) -> Vec<Placement> {
    let value = |c: &Candidate| c.value * if c.large { 2 } else { 1 };
    let mut order: Vec<usize> = (0..cands.len()).collect();
    order.sort_by(|&x, &y| {
        let (a, b) = (&cands[x], &cands[y]);
        a.pick_order.cmp(&b.pick_order).then(b.healthy.cmp(&a.healthy)).then(value(b).cmp(&value(a))).then(x.cmp(&y))
    });
    let mut first: Vec<usize> = Vec::new();
    let mut rest: Vec<usize> = Vec::new();
    for &i in &order {
        if first.iter().any(|&j| cands[j].pick_order == cands[i].pick_order) {
            rest.push(i);
        } else {
            first.push(i);
        }
    }
    let (mut used, mut non_river) = (0, 0);
    let mut chosen: Vec<usize> = Vec::new();
    for i in first.into_iter().chain(rest) {
        let size = 1 + cands[i].large as i32;
        let river = cands[i].river;
        if used + size <= slots && (river || non_river + size <= ordinary) {
            chosen.push(i);
            used += size;
            if !river {
                non_river += size;
            }
        }
    }
    let lanes_n = lanes as usize;
    let mut grid = vec![[false; ROWS]; lanes_n];
    for &(l, r) in blocked {
        if let Some(cell) = grid.get_mut(l as usize).and_then(|g| g.get_mut(r as usize)) {
            *cell = true;
        }
    }
    let centre = |l: usize, large: bool| (2 * l as i32 + large as i32 - (lanes as i32 - 1)).abs();
    let mut out: Vec<Placement> = Vec::new();
    let larges = chosen.iter().copied().filter(|&i| cands[i].large);
    let smalls = chosen.iter().copied().filter(|&i| !cands[i].large);
    let mut larges_left = chosen.iter().filter(|&&i| cands[i].large).count();
    for i in larges.chain(smalls).collect::<Vec<_>>() {
        let c = &cands[i];
        if c.large {
            larges_left -= 1;
        }
        let mut cell = None;
        for r in c.row_order {
            let r = (r as usize).min(ROWS - 1);
            let mut free: Vec<usize> =
                (0..lanes_n).filter(|&l| if c.large { l + 1 < lanes_n && !grid[l][r] && !grid[l + 1][r] } else { !grid[l][r] }).collect();
            let odd_left = |l: usize| c.large && larges_left > 0 && (0..l).rev().take_while(|&k| !grid[k][r]).count() % 2 == 1;
            free.sort_by_key(|&l| (odd_left(l), centre(l, c.large), l));
            if let Some(&l) = free.first() {
                cell = Some((l, r));
                break;
            }
        }
        if let Some((l, r)) = cell {
            grid[l][r] = true;
            if c.large {
                grid[l + 1][r] = true;
            }
            out.push(Placement { unit: i, lane: l as u8, row: r as u8 });
        }
    }
    out.sort_by_key(|p| p.unit);
    out
}

#[test]
fn line_is_the_auto_formation_as_it_was() {
    let mut rng = Rng::new(20261109);
    let mut r = |n: i32| rng.get_i(n, "test");
    let orders: [[u8; ROWS]; 6] = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    for _ in 0..5000 {
        let n = r(17) as usize;
        // A narrow range of values in some pools: ties are common.
        let range = if r(2) == 0 { 6 } else { 300 };
        let cands: Vec<Candidate> = (0..n)
            .map(|_| Candidate {
                pick_order: 1 + r(9),
                row_order: orders[r(6) as usize],
                large: r(100) < 15,
                value: 1 + r(range) as i64,
                healthy: r(100) < 80,
                river: r(100) < 80,
                veteran: r(100) < 30,
            })
            .collect();
        let lanes = 2 + r(5) as u8;
        let slots = 3 + r(12);
        let ordinary = slots - r(3);
        let blocked = if r(3) == 0 { sentinel_cells(lanes, r(4) as usize) } else { Vec::new() };
        assert_eq!(
            auto_arrange_around(&cands, lanes, slots, ordinary, &blocked, &TacticKnobs::line()),
            reference(&cands, lanes, slots, ordinary, &blocked),
            "{cands:?} lanes {lanes} slots {slots} ordinary {ordinary} blocked {blocked:?}"
        );
    }
}

/// A candidate of kind `k` with its kind's usual rank order, of strength `value`.
fn c(k: AK, value: i64) -> Candidate {
    let row_order = match k {
        AK::AntiFlank | AK::Strike | AK::Charge => [0, 1, 2],
        AK::Shoot | AK::Heal | AK::Stab => [1, 2, 0],
        AK::Spell | AK::Bombard | AK::None => [2, 1, 0],
    };
    Candidate { pick_order: k.pick_order(), row_order, large: false, value, healthy: true, river: true, veteran: false }
}

fn charge() -> TacticKnobs {
    TacticKnobs {
        pick_order: vec![AK::Charge, AK::AntiFlank, AK::Strike, AK::Shoot, AK::Stab, AK::Spell, AK::Heal, AK::Bombard],
        one_of_each: false,
        place_first: vec![AK::Charge],
        ..TacticKnobs::line()
    }
}

fn volley() -> TacticKnobs {
    TacticKnobs {
        pick_order: vec![AK::Shoot, AK::Spell, AK::Bombard, AK::Stab, AK::Heal, AK::AntiFlank, AK::Strike, AK::Charge],
        one_of_each: false,
        screen_per_lane: 1,
        ..TacticKnobs::line()
    }
}

fn placement(p: TacticPlacement) -> TacticKnobs {
    TacticKnobs { placement: p, ..TacticKnobs::line() }
}

fn cells(p: &[Placement]) -> Vec<(usize, u8, u8)> {
    p.iter().map(|x| (x.unit, x.lane, x.row)).collect()
}

fn units(p: &[Placement]) -> Vec<usize> {
    p.iter().map(|x| x.unit).collect()
}

#[test]
fn charge_takes_chargers_first_into_the_front() {
    // 4 strikers, 4 chargers, 2 archers; 6 slots, 3 lanes.
    let mut pool: Vec<Candidate> = (0..4).map(|i| c(AK::Strike, 50 + i)).collect();
    pool.extend((0..4).map(|i| c(AK::Charge, 40 + i)));
    pool.extend((0..2).map(|i| c(AK::Shoot, 30 + i)));
    // Line: one of each first (the strongest striker 3, charger 7, archer 9), then the strikers.
    let line = auto_arrange_around(&pool, 3, 6, 6, &[], &TacticKnobs::line());
    assert_eq!(units(&line), [0, 1, 2, 3, 7, 9]);
    // Charge: the four chargers, then the strikers.
    let p = auto_arrange_around(&pool, 3, 6, 6, &[], &charge());
    assert_eq!(units(&p), [2, 3, 4, 5, 6, 7]);
    // The chargers (strongest first) take the front from the centre out; the fourth and the
    // strikers the middle.
    let cell = |u: usize| p.iter().find(|x| x.unit == u).map(|x| (x.lane, x.row)).unwrap();
    assert_eq!(cell(7), (1, 0));
    assert_eq!(cell(6), (0, 0));
    assert_eq!(cell(5), (2, 0));
    assert_eq!(cell(4).1, 1);
    assert_eq!(cell(3).1, 1);
    // With the chargers already in, place_first still puts them before the guards taken earlier:
    // a Charge tactic that kept Line's selection.
    let knobs = TacticKnobs { place_first: vec![AK::Charge], ..TacticKnobs::line() };
    let p = auto_arrange_around(&pool, 3, 6, 6, &[], &knobs);
    assert_eq!(p.iter().find(|x| x.unit == 7).map(|x| (x.lane, x.row)), Some((1, 0)), "the charger takes the centre of the front");
}

#[test]
fn volley_screens_then_takes_the_support() {
    // 6 strikers, 6 archers, 2 mages: 8 slots, 4 lanes.
    let mut pool: Vec<Candidate> = (0..6).map(|i| c(AK::Strike, 50 + i)).collect();
    pool.extend((0..6).map(|i| c(AK::Shoot, 30 + i)));
    pool.extend((0..2).map(|i| c(AK::Spell, 20 + i)));
    let p = auto_arrange_around(&pool, 4, 8, 8, &[], &volley());
    // The screen: the 4 strongest strikers (2-5), then the archers (all 6 would go, 4 fit).
    assert_eq!(units(&p), [2, 3, 4, 5, 8, 9, 10, 11]);
    assert!(p.iter().filter(|x| x.unit < 6).all(|x| x.row == 0), "the screen stands in front");
    assert!(p.iter().filter(|x| x.unit >= 6).all(|x| x.row == 1), "the archers in their row");
    // 12 slots: the screen, the 6 archers, the 2 mages; then the remaining strikers.
    let p = auto_arrange_around(&pool, 5, 12, 12, &[], &volley());
    // (5 lanes: the 5 strongest strikers 1-5, the 6 archers, the stronger mage 13.)
    assert_eq!(units(&p), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13]);
    // A large guard screens two lanes; a blocked front cell (a Sentinel) needs no screen.
    let mut pool2 = pool.clone();
    pool2[5].large = true;
    let p = auto_arrange_around(&pool2, 4, 8, 8, &[(1, 0)], &volley());
    // 3 front lanes to screen: the giant (2 lanes) and striker 4; then 5 archers.
    let guards: Vec<usize> = units(&p).into_iter().filter(|&u| u < 6).collect();
    assert_eq!(guards, [4, 5]);
    assert_eq!(p.len(), 7, "the giant counts 2 slots: 1 + 2 + 5 archers = 8");
}

#[test]
fn square_puts_the_support_between_guards() {
    // 4 strikers, 1 anti-flanking striker, 4 archers, 1 mage: 10 slots, 4 lanes.
    let mut pool: Vec<Candidate> = (0..4).map(|i| c(AK::Strike, 50 + i)).collect(); // 0-3
    pool.push(c(AK::AntiFlank, 45)); // 4
    pool.extend((0..4).map(|i| c(AK::Shoot, 30 + i))); // 5-8
    pool.push(c(AK::Spell, 60)); // 9
    let p = auto_arrange_around(&pool, 4, 10, 10, &[], &placement(TacticPlacement::Square));
    let mut got = cells(&p);
    got.sort_by_key(|&(_, l, r)| (l, r));
    // Lanes from the centre out: 1, 2, 0, 3. Lane 1: the strongest striker (3) in front, the
    // strongest support (the mage 9) in the middle, the anti-flanking striker (4) in the rear.
    // Lane 2: striker 2, archer 8, striker 1. Lane 0: striker 0, archer 7, then no guard: archer 6
    // in the rear. Lane 3: no guard left: archer 5 in front.
    assert_eq!(got, [(0, 0, 0), (7, 0, 1), (6, 0, 2), (3, 1, 0), (9, 1, 1), (4, 1, 2), (2, 2, 0), (8, 2, 1), (1, 2, 2), (5, 3, 0)]);
    // Without support, the middle takes a guard.
    let guards: Vec<Candidate> = (0..4).map(|i| c(AK::Strike, 50 + i)).collect();
    let p = auto_arrange_around(&guards, 2, 4, 4, &[], &placement(TacticPlacement::Square));
    let mut got = cells(&p);
    got.sort_by_key(|&(_, l, r)| (l, r));
    assert_eq!(got, [(3, 0, 0), (2, 0, 1), (1, 0, 2), (0, 1, 0)]);
    // A large guard takes a front pair; the lanes it holds take a support unit in the middle.
    let mut pool3 = vec![c(AK::Strike, 50), c(AK::Shoot, 30), c(AK::Shoot, 20), c(AK::Strike, 40)];
    pool3[0].large = true;
    let p = auto_arrange_around(&pool3, 2, 6, 6, &[], &placement(TacticPlacement::Square));
    let mut got = cells(&p);
    got.sort_by_key(|&(_, l, r)| (l, r));
    assert_eq!(got, [(0, 0, 0), (1, 0, 1), (3, 0, 2), (2, 1, 1)]);
}

#[test]
fn vanguard_shields_the_veterans() {
    // 4 strikers (one strong veteran, one weak veteran), 2 archers: 6 slots, 3 lanes.
    let mut pool: Vec<Candidate> = vec![c(AK::Strike, 50), c(AK::Strike, 40), c(AK::Strike, 90), c(AK::Strike, 60), c(AK::Shoot, 30), c(AK::Shoot, 20)];
    pool[2].veteran = true;
    pool[3].veteran = true;
    let p = auto_arrange_around(&pool, 3, 6, 6, &[], &placement(TacticPlacement::Vanguard));
    let cell = |u: usize| p.iter().find(|x| x.unit == u).map(|x| (x.lane, x.row)).unwrap();
    // The centre lane: the weakest striker that is no veteran (1) before the most valuable veteran
    // (2); lane 0: striker 0 before veteran 3. The archers in their row, where there is room.
    assert_eq!(cell(1), (1, 0));
    assert_eq!(cell(2), (1, 1));
    assert_eq!(cell(0), (0, 0));
    assert_eq!(cell(3), (0, 1));
    assert_eq!(cell(4), (2, 1));
    assert_eq!(cell(5), (1, 2), "the middle is full: the next row of its order");
    // Line puts the veterans in front.
    let line = auto_arrange_around(&pool, 3, 6, 6, &[], &TacticKnobs::line());
    assert!(line.iter().filter(|x| x.unit == 2 || x.unit == 3).all(|x| x.row == 0));
    // A veteran with nobody to stand behind is placed as in Line.
    let mut pool2 = vec![c(AK::Strike, 50), c(AK::Strike, 90), c(AK::Strike, 60)];
    pool2[1].veteran = true;
    pool2[2].veteran = true;
    let p = auto_arrange_around(&pool2, 3, 6, 6, &[], &placement(TacticPlacement::Vanguard));
    let cell = |u: usize| p.iter().find(|x| x.unit == u).map(|x| (x.lane, x.row)).unwrap();
    assert_eq!(cell(0), (1, 0));
    assert_eq!(cell(1), (1, 1));
    assert_eq!(cell(2).1, 0, "no guard to stand behind: the front, as in Line");
}

#[test]
fn every_tactic_places_each_unit_once_inside_the_grid() {
    let mut rng = Rng::new(7);
    let mut r = |n: i32| rng.get_i(n, "test");
    let tactics = [TacticKnobs::line(), charge(), volley(), placement(TacticPlacement::Square), placement(TacticPlacement::Vanguard)];
    for _ in 0..2000 {
        let n = r(15) as usize;
        let cands: Vec<Candidate> = (0..n)
            .map(|_| {
                let k = AK::ALL[r(9) as usize];
                let value = 1 + r(100) as i64;
                let (large, healthy, river, veteran) = (r(100) < 15, r(100) < 80, r(100) < 80, r(100) < 30);
                Candidate { large, healthy, river, veteran, ..c(k, value) }
            })
            .collect();
        let lanes = 2 + r(5) as u8;
        let slots = 3 + r(12);
        let ordinary = slots - r(3);
        let blocked = if r(3) == 0 { sentinel_cells(lanes, r(4) as usize) } else { Vec::new() };
        for k in &tactics {
            let p = auto_arrange_around(&cands, lanes, slots, ordinary, &blocked, k);
            let mut taken: Vec<(u8, u8)> = blocked.clone();
            let (mut used, mut non_river) = (0, 0);
            for x in &p {
                assert!(x.row < 3);
                let size = 1 + cands[x.unit].large as i32;
                used += size;
                if !cands[x.unit].river {
                    non_river += size;
                }
                for l in x.lane..x.lane + size as u8 {
                    assert!(l < lanes);
                    assert!(!taken.contains(&(l, x.row)), "{k:?}: two units in one cell");
                    taken.push((l, x.row));
                }
            }
            assert!(used <= slots && non_river <= ordinary);
            assert!(p.windows(2).all(|w| w[0].unit < w[1].unit));
            // Small units always find a cell: a tactic leaves out only what the slots leave out.
            if cands.iter().all(|c| !c.large) {
                let river = cands.iter().filter(|c| c.river).count();
                let fit = (slots as usize).min(river + (cands.len() - river).min(ordinary as usize)).min(3 * lanes as usize - blocked.len());
                assert_eq!(p.len(), fit, "{k:?}: {} placed, {fit} fit", p.len());
            }
        }
    }
}
