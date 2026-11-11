//! Ships fire into battles on the coast (this game's own rules; docs/NAVAL.md, 4.7).
//!
//! When an engagement is fought on a land plot, each side's warships with a bombard rate that
//! stand on a water plot next to the battle's plot, belong to a player of a team of that side and
//! have not fired this turn (`Unit::shore_fired`) fire once before round 1: at most
//! `SHORE_FIRE_SHIPS` a side, the strongest first ([`Game::engage_shore_ships`], kept in the
//! set-up, so the real fight, the preview and the forecast see the same). Each shot is
//! `formation::shore_shot`: one SHOOT hit by the resolver's own damage rule, the ship's A its
//! strength on the units' stat scale ([`Game::unit_sea_strength_x100`]), the target chosen as an
//! arrow from outside the field is; it never takes a unit below 1 HP. The attacker's ships fire
//! first, then the defender's, after the defended city's own pre-battle strike; ships take no
//! damage and are not fighters. Each shot draws r ("Engagement hit") from the battle's RNG: the
//! game's in the real fight, the forecast's own in the preview.

use ffh_data::*;

use super::engage::Setup;
use super::formation::{self, BattleRng, Fighter, FighterIdx, Formation, Pair, Side};
use super::report::LogEvent;
use super::sink::Sink;
use crate::game::Game;
use crate::ids::*;

/// A ship that fires into a battle (module comment): its side and its A (x100).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShoreShip {
    pub unit: UnitId,
    pub side: Side,
    pub a: i32,
}

/// One shot as fired: the ship, its side, the target (set-up index) and the damage (centi-HP).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShoreShot {
    pub ship: UnitId,
    pub unit_type: UnitType,
    pub side: Side,
    pub target: usize,
    pub dmg: i32,
}

impl Game {
    /// The ships that fire into the battle on `to` (module comment), the attacker's then the
    /// defender's, each side's strongest first (ties: the lower unit id). `att_teams` and
    /// `def_teams`: the teams of each side. None for a battle on water.
    pub fn engage_shore_ships(&self, to: PlotIdx, att_teams: &[TeamId], def_teams: &[TeamId]) -> Vec<ShoreShip> {
        if self.plot_is_water(to) {
            return Vec::new();
        }
        let max = self.d.defines.shore_fire_ships.max(0) as usize;
        let (x, y) = self.map.xy(to);
        let mut out = Vec::new();
        for (side, teams) in [(Side::Attacker, att_teams), (Side::Defender, def_teams)] {
            let mut ships: Vec<(i32, UnitId)> = Vec::new();
            for i in 0..DirectionType::NUM {
                let Some(w) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                if !self.plot_is_water(w) {
                    continue;
                }
                for &u in &self.map.plots[w].units {
                    if !self.units.contains(u) || self.units[u].shore_fired || self.unit_is_cargo(u) || !teams.contains(&self.unit_get_team(u)) {
                        continue;
                    }
                    if !self.unit_is_warship(u) || self.d.units[self.units[u].unit_type].bombard_rate <= 0 {
                        continue;
                    }
                    ships.push((self.unit_sea_strength_x100(u), u));
                }
            }
            ships.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            out.extend(ships.into_iter().take(max).filter(|&(a, _)| a > 0).map(|(a, unit)| ShoreShip { unit, side, a }));
        }
        out
    }

    /// The teams of each side of a set-up: the attacker's units' and the defender's units' (and
    /// the defended city's owner's).
    pub(crate) fn engage_side_teams(&self, to: PlotIdx, units: &[UnitId], n_att: usize) -> (Vec<TeamId>, Vec<TeamId>) {
        let mut att: Vec<TeamId> = units[..n_att].iter().map(|&u| self.unit_get_team(u)).collect();
        let mut def: Vec<TeamId> = units[n_att..].iter().map(|&u| self.unit_get_team(u)).collect();
        if let Some(c) = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c)) {
            def.push(self.city_team(c));
        }
        att.sort();
        att.dedup();
        def.sort();
        def.dedup();
        def.retain(|t| !att.contains(t));
        (att, def)
    }

    /// The shore fire of `setup` (module comment) on the fighters that did not sit out, lowering
    /// `hp` (centi-HP, by set-up index); each shot logged to `sink`. Draws r per shot that finds a
    /// target. The shots in order.
    pub(crate) fn engage_shore_fire(&self, setup: &Setup, sat_out: &[bool], hp: &mut [i32], rng: &mut impl BattleRng, mut sink: Option<&mut Sink>) -> Vec<ShoreShot> {
        let mut shots = Vec::new();
        let n_units = setup.units.len();
        for ship in &setup.shore {
            let foe = ship.side.other();
            // The targets: the foe's committed units on the field, then (against the defender) its
            // Sentinels standing.
            let targets: Vec<usize> = (0..setup.fighters.len())
                .filter(|&i| setup.sides[i] == foe)
                .filter(|&i| {
                    if i < n_units {
                        let on = match setup.fighters[i].spawner {
                            Some(_) => false,
                            None => !sat_out[i],
                        };
                        on && hp[i] > 0
                    } else {
                        hp[i] > 0
                    }
                })
                .collect();
            if targets.is_empty() {
                continue;
            }
            let mut gunner = Fighter::new(ship.side, 1);
            gunner.a = ship.a;
            let mut fighters = vec![gunner];
            for &i in &targets {
                let mut f = setup.fighters[i].clone();
                f.hp = hp[i];
                f.spawner = None;
                fighters.push(f);
            }
            let dvs: Vec<i32> = targets
                .iter()
                .map(|&i| if i < n_units { self.engage_dv_alone(setup.units[i], foe, &setup.ctx) } else { setup.pair(0, i).dv })
                .collect();
            let f = Formation::new(setup.params, setup.ground, setup.lanes, fighters, |h, t| if h == 0 && t > 0 { Pair::physical(ship.a, dvs[t - 1]) } else { Pair::default() });
            let Some(hit) = formation::shore_shot(&f, 0, rng) else { continue };
            let i = targets[hit.target as usize - 1];
            hp[i] -= hit.dmg;
            shots.push(ShoreShot { ship: ship.unit, unit_type: self.units[ship.unit].unit_type, side: ship.side, target: i, dmg: hit.dmg });
            if let Some(s) = sink.as_deref_mut() {
                let u = &self.units[ship.unit];
                s.event(LogEvent::ShoreFire { ship: u.unit_type, owner: u.owner, side: ship.side, hits: vec![(i as FighterIdx, hit.dmg)] });
            }
        }
        shots
    }
}
