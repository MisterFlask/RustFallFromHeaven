//! The squads of a battle (docs/ENGAGEMENTS.md, "Squads"; the squads themselves are
//! `army/squad.rs`): one battle is one squad of the attacker against one squad of the defender.
//!
//! **The attacking squad.** The attacker's squads on the plot it attacks from, each with those
//! of its units that can attack the plot (the others leave their cells empty). The order names
//! one; without one, the strongest (the sum of its fighting units' A against nobody in
//! particular, a large unit's counting double; ties: the first in order) fights.
//!
//! **The defending squad** (the owner's rule, 2026-11-21: first who has fought least, then the
//! best). The defenders' squads on the defended plot (each owner's), each with those of its units
//! that are candidates. Only the squads that fought the fewest battles this game turn
//! (`Game::squad_fought_this_turn`, attacking or defending; units in no squad count 0) may
//! fight, so no squad fights a second battle in a turn before every other has fought one. Of
//! those, with one, it fights; with several, each is played against the attacking squad
//! `ENGAGEMENT_DEFENSE_PICK_RUNS` times on a private RNG seeded from the battle (no draw of the
//! game's RNG), and the squad with the best result for the defender fights: the most attacker HP
//! lost less defender HP lost, each unit's loss in permille of its HP; ties: the first in order.
//! The other candidates are the defender's reserve.
//!
//! Units that stand in no squad (an attack from a ship, a defender aboard a transport) are formed
//! into squads of their own by the auto formation, after the others.
//!
//! **Sentinels** (docs/SIEGES.md) hold the front cells of the defender's grid; a unit of the
//! defending squad whose cell they hold stands in the nearest free cell (its row first, the lanes
//! nearest the centre first, then the rows behind, then the rows before).

use super::arrange::Place;
use super::engage::Setup;
use super::formation::{self, Side};
use super::slots::Slots;
use super::stats::HitCtx;
use crate::army::{SquadBody, SQUAD_LANES};
use crate::game::Game;
use crate::ids::*;
use crate::rng::Rng;
use ffh_data::ROWS;

/// A squad that can fight on one side of a battle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSquad {
    /// The squad (`None`: units that stand in no squad, formed for this battle).
    pub squad: Option<SquadId>,
    pub owner: PlayerId,
    pub body: SquadBody,
    /// Its number within its body on its plot (1-based; 0 for units in no squad).
    pub number: u32,
    /// Its units that fight, in their cells.
    pub places: Vec<Place>,
}

impl FieldSquad {
    pub fn units(&self) -> Vec<UnitId> {
        self.places.iter().map(|p| p.unit).collect()
    }
}

impl Game {
    /// The squads of `units` (all on `plot`), each owner's in the order of
    /// [`Game::squad_layouts_at`], keeping only the units in `units`; the units that stand in no
    /// squad formed into squads of their own by the auto formation of side `side`.
    pub fn engage_field_squads(&self, plot: PlotIdx, units: &[UnitId], side: Side, ctx: &HitCtx) -> Vec<FieldSquad> {
        let mut owners: Vec<PlayerId> = units.iter().map(|&u| self.units[u].owner).collect();
        owners.sort();
        owners.dedup();
        let mut out: Vec<FieldSquad> = Vec::new();
        let mut placed: Vec<UnitId> = Vec::new();
        for p in owners {
            let layouts = self.squad_layouts_at(p, plot);
            let numbers = Game::squad_numbers(&layouts);
            for (l, n) in layouts.iter().zip(numbers) {
                let places: Vec<Place> = l
                    .units()
                    .into_iter()
                    .filter(|(u, _)| units.contains(u))
                    .map(|(u, c)| Place { unit: u, lane: c % SQUAD_LANES, row: c / SQUAD_LANES })
                    .collect();
                if places.is_empty() {
                    continue;
                }
                placed.extend(places.iter().map(|p| p.unit));
                out.push(FieldSquad { squad: l.squad, owner: p, body: l.body, number: n, places });
            }
        }
        // Units in no squad: squads of the auto formation, one after the other.
        let mut rest: Vec<UnitId> = units.iter().copied().filter(|u| !placed.contains(u) && self.unit_engages_on_land(*u)).collect();
        let slots = self.engage_squad_slots();
        while !rest.is_empty() {
            let f = self.engage_auto_formation(&rest, side, &slots, ctx);
            if f.is_empty() {
                break;
            }
            rest.retain(|u| !f.iter().any(|p| p.unit == *u));
            let owner = self.units[f[0].unit].owner;
            out.push(FieldSquad { squad: None, owner, body: SquadBody::Loose, number: 0, places: f });
        }
        out
    }

    /// The attacker's squads in `pool` on `from` (module comment).
    pub fn engage_attacking_squads(&self, from: PlotIdx, pool: &[UnitId], ctx: &HitCtx) -> Vec<FieldSquad> {
        self.engage_field_squads(from, pool, Side::Attacker, ctx)
    }

    /// The strongest of `squads` on side `side` (module comment): its index.
    pub fn engage_strongest_squad(&self, squads: &[FieldSquad], side: Side, ctx: &HitCtx) -> Option<usize> {
        let power = |s: &FieldSquad| -> i64 { s.places.iter().map(|p| self.engage_a_alone(p.unit, side, ctx) as i64 * (1 + self.unit_engage_large(p.unit) as i64)).sum() };
        let mut best: Option<(usize, i64)> = None;
        for (i, s) in squads.iter().enumerate() {
            let v = power(s);
            if best.map_or(true, |(_, b)| v > b) {
                best = Some((i, v));
            }
        }
        best.map(|(i, _)| i)
    }

    /// The cells of `squad`'s units with `blocked` cells (the Sentinels') taken (module comment):
    /// a unit whose cell is held stands in the nearest free one, or does not fight when the grid
    /// is full.
    pub fn engage_squad_places(&self, squad: &FieldSquad, blocked: &[(u8, u8)]) -> Vec<Place> {
        let lanes = SQUAD_LANES;
        let mut taken = vec![[false; ROWS]; lanes as usize];
        for &(l, r) in blocked {
            if (l as usize) < taken.len() && (r as usize) < ROWS {
                taken[l as usize][r as usize] = true;
            }
        }
        let size = |u: UnitId| 1 + self.unit_engage_large(u) as u8;
        let free = |taken: &Vec<[bool; ROWS]>, lane: u8, row: u8, n: u8| lane + n <= lanes && (0..n).all(|k| !taken[(lane + k) as usize][row as usize]);
        let mut out: Vec<Place> = Vec::new();
        let mut moved: Vec<Place> = Vec::new();
        for &p in &squad.places {
            if free(&taken, p.lane, p.row, size(p.unit)) {
                for k in 0..size(p.unit) {
                    taken[(p.lane + k) as usize][p.row as usize] = true;
                }
                out.push(p);
            } else {
                moved.push(p);
            }
        }
        for p in moved {
            let n = size(p.unit);
            let rows: Vec<u8> = std::iter::once(p.row).chain(p.row + 1..ROWS as u8).chain((0..p.row).rev()).collect();
            let order: &[u8] = if n > 1 { &[0, 1] } else { &[1, 0, 2] };
            let cell = rows.iter().find_map(|&r| order.iter().find(|&&l| free(&taken, l, r, n)).map(|&l| (l, r)));
            if let Some((l, r)) = cell {
                for k in 0..n {
                    taken[(l + k) as usize][r as usize] = true;
                }
                out.push(Place { unit: p.unit, lane: l, row: r });
            }
        }
        out
    }

    /// The defenders' squad that fights the attacking formation `att` (module comment): its
    /// index in `squads`.
    pub(crate) fn engage_pick_defending_squad(&self, ctx: &HitCtx, slots: &Slots, walls: &Option<(CityId, Vec<usize>)>, att: &[Place], candidates: &[UnitId], squads: &[FieldSquad]) -> usize {
        let fought = |s: &FieldSquad| s.squad.map_or(0, |id| self.squad_fought_this_turn(id));
        let fewest = squads.iter().map(fought).min().unwrap_or(0);
        let eligible: Vec<usize> = (0..squads.len()).filter(|&i| fought(&squads[i]) == fewest).collect();
        if eligible.len() <= 1 || att.is_empty() {
            return eligible.first().copied().unwrap_or(0);
        }
        let runs = self.d.defines.engagement_defense_pick_runs.max(1);
        let blocked = self.engage_sentinel_cells(slots);
        let mut words: Vec<u32> = vec![self.game_turn() as u32, ctx.to.0, ctx.from.map_or(u32::MAX, |p| p.0)];
        words.extend(att.iter().map(|p| p.unit.0));
        let mut best: Option<(usize, i64)> = None;
        for &i in &eligible {
            let def = self.engage_squad_places(&squads[i], &blocked);
            if def.is_empty() {
                continue;
            }
            let Ok(setup) = self.engage_setup_placed(*ctx, slots.clone(), walls.clone(), att.to_vec(), def, candidates.to_vec(), &[]) else { continue };
            let mut rng = Rng::new(seed_of(&words));
            let mut score: i64 = 0;
            for _ in 0..runs {
                score += self.engage_pick_score(&setup, &mut rng);
            }
            if best.map_or(true, |(_, b)| score > b) {
                best = Some((i, score));
            }
        }
        best.map_or(eligible[0], |(i, _)| i)
    }

    /// One quiet battle of `setup` for the defender's choice: the attacker's HP lost less the
    /// defender's, each unit's loss in permille of its HP.
    fn engage_pick_score(&self, setup: &Setup, rng: &mut Rng) -> i64 {
        let n = setup.units.len();
        let hp: Vec<i32> = (0..setup.fighters.len())
            .map(|i| if i < n { super::engage::hp_from_damage(self.units[setup.units[i]].damage, setup.fighters[i].max_hp) } else { setup.fighters[i].hp })
            .collect();
        let sat_out = setup.initial_sat_out();
        let (f, map) = setup.battle(&sat_out, &hp);
        let out = formation::resolve_quiet(&f, rng);
        let mut score = 0i64;
        for (k, end) in out.fighters.iter().enumerate() {
            let i = map.get(k).copied().unwrap_or(usize::MAX);
            if i >= n {
                continue;
            }
            let start = hp[i].max(1) as i64;
            let lost = (start - end.hp.max(0) as i64).max(0) * 1000 / start;
            score += if setup.sides[i] == Side::Attacker { lost } else { -lost };
        }
        score
    }
}

/// FNV-1a over the words (the defender's choice's seed).
fn seed_of(words: &[u32]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}
