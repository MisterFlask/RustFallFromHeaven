//! The great-person cards that act in battles and at walls (docs/CARDS.md; the batch "battle
//! arms"), on `Game`: what the defended city's buildings, the units' types and their promotions
//! give an engagement's fighters ([`Game::engage_arms_setup`], called at the end of every set-up,
//! so the real fight and the preview see the same), the defended city's pre-battle damage
//! ([`Game::engage_arms_opening`]), the Sentinels a building changes, the attacker's slots on an
//! improvement, a unit's bombard range and the element of its attack. The resolver's side is
//! `formation::arms_cards`.
//!
//! | Card | Key | Here |
//! |---|---|---|
//! | Thunderhead Tower | `pre_battle_damage` | [`Game::engage_arms_opening`] |
//! | Arcane Fortress | `sentinel_ranged_resist` | the Sentinels' `ranged_resist`; [`Game::city_sentinel_ranged_resist`] for map bombards |
//! | Living Walls | `sentinel_unit` | [`Game::city_sentinel_stat`] |
//! | Moat | `hold_melee_rounds` | the attackers' `hold_melee_rounds` in the first battle at the city each turn |
//! | Trench | `engagement_attacker_slots_change` | [`Game::engage_arms_attacker_slots`] |
//! | Siege Colossus | `engage_first_round`, `bombard_range`, `bombard_besieges` | the fighter's `first_round`; [`Game::unit_bombard_plots`] |
//! | Last Stand Hymn | `last_stand_damage` | the defenders' `last_stand_pct` |
//! | Wardance | `first_melee_pattern` | the fighter's `first_melee_pattern` |
//! | Templar | `heal_behind`, `unit_classes` | the fighter's `heal_behind` |
//! | The Unconquered City | `never_lost_damage` | the defenders' pairs' percentage |
//! | Elemental Forge | `attack_damage_type`, `exclusive_with` | [`Game::engage_arms_attack`] |

use ffh_data::*;

use super::engage::Setup;
use super::formation::{FighterIdx, Side};
use super::report::{LogEvent, SlotChange, SlotSource};
use super::sink::Sink;
use crate::game::Game;
use crate::ids::*;

impl Game {
    /// The city on plot `to`, if any.
    fn engage_arms_city(&self, to: PlotIdx) -> Option<CityId> {
        self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c))
    }

    /// The most any building of city `c` gives of a battle key (`f`), 0 for none.
    pub fn city_battle_max(&self, c: CityId, f: impl Fn(&BuildingBattle) -> i32) -> i32 {
        let d = self.d;
        d.buildings.keys().filter(|&b| d.buildings[b].battle.any() && self.city_num_building(c, b) > 0).map(|b| f(&d.buildings[b].battle)).max().unwrap_or(0).max(0)
    }

    /// Whether city `c` has never changed hands: never lost, and its original owner owns it.
    pub fn city_never_lost(&self, c: CityId) -> bool {
        let city = &self.cities[c];
        city.never_lost && city.original_owner == city.owner
    }

    /// Whether no battle has been fought at city `c` this game turn (the battle reports of the
    /// current turn are always kept).
    pub fn city_first_battle_this_turn(&self, c: CityId) -> bool {
        let (plot, turn) = (self.city_plot(c), self.game_turn());
        !self.battle_reports.iter().rev().take_while(|r| r.turn >= turn).any(|r| r.turn == turn && r.plot == plot)
    }

    /// The unit type whose attack and HP city `c`'s Sentinels fight with (`sentinel_unit`, the
    /// Living Walls), if a building of it names one.
    pub fn city_sentinel_override(&self, c: CityId) -> Option<UnitType> {
        let d = self.d;
        d.buildings.keys().filter(|&b| d.buildings[b].battle.sentinel_unit.is_some() && self.city_num_building(c, b) > 0).find_map(|b| d.buildings[b].battle.sentinel_unit)
    }

    /// Stat `s` of city `c`'s Sentinels before the city's defence grows them: `UNIT_SENTINEL`'s,
    /// or for HP and attack its override's ([`Game::city_sentinel_override`]).
    pub fn city_sentinel_stat(&self, c: CityId, s: StatType) -> i32 {
        match (s, self.city_sentinel_override(c)) {
            (StatType::Hp | StatType::Attack, Some(u)) => self.d.unit_stat(u, s),
            _ => self.sentinel_stat(s),
        }
    }

    /// City `c`'s Sentinels' A, x100: their attack ([`Game::city_sentinel_stat`]), x10.
    pub fn city_sentinel_a_x100(&self, c: CityId) -> i32 {
        (self.city_sentinel_stat(c, StatType::Attack) * 10).max(1)
    }

    /// Percent less damage city `c`'s Sentinels take from bombards, BOMBARD and SHOOT
    /// (`sentinel_ranged_resist`, the Arcane Fortress), at most 100.
    pub fn city_sentinel_ranged_resist(&self, c: CityId) -> i32 {
        self.city_battle_max(c, |b| b.sentinel_ranged_resist).min(100)
    }

    /// The battle keys of the cards on a set-up (module comment): each committed unit's type's
    /// first-round action and its promotions' first melee pattern and heal behind; the defended
    /// city's hold on the attacker's melee (the first battle there this turn), its last stand,
    /// its Sentinels' resistance to shots and bombards, and its defenders' damage while it has
    /// never changed hands.
    pub(crate) fn engage_arms_setup(&self, setup: &mut Setup) {
        let d = self.d;
        for i in 0..setup.units.len() {
            let u = setup.units[i];
            let unit = &self.units[u];
            let a = &mut setup.fighters[i].arms;
            a.first_round = d.units[unit.unit_type].engage_first_round;
            for p in d.promotions.keys().filter(|&p| unit.has_promotion[p]) {
                let info = &d.promotions[p];
                a.first_melee_pattern = a.first_melee_pattern.or(info.first_melee_pattern);
                a.heal_behind = a.heal_behind.max(info.heal_behind.max(0) * 100);
            }
        }
        let Some(c) = self.engage_arms_city(setup.ctx.to) else { return };
        let hold = if self.city_first_battle_this_turn(c) { self.city_battle_max(c, |b| b.hold_melee_rounds) } else { 0 };
        let last = self.city_battle_max(c, |b| b.last_stand_damage);
        let resist = self.city_sentinel_ranged_resist(c);
        let never = if self.city_never_lost(c) { self.city_battle_max(c, |b| b.never_lost_damage) } else { 0 };
        let n = setup.fighters.len();
        for i in 0..n {
            let sentinel = setup.is_sentinel(i);
            let a = &mut setup.fighters[i].arms;
            match setup.sides[i] {
                Side::Attacker => a.hold_melee_rounds = hold,
                Side::Defender if sentinel => a.ranged_resist = resist,
                Side::Defender => a.last_stand_pct = last,
            }
        }
        if never > 0 {
            for h in (0..n).filter(|&h| setup.sides[h] == Side::Defender) {
                for t in (0..n).filter(|&t| setup.sides[t] == Side::Attacker) {
                    let p = &mut setup.pairs[h * n + t];
                    p.pct = (p.pct.max(0) as i64 * (100 + never) as i64 / 100) as i32;
                }
            }
        }
    }

    /// The defended city's pre-battle damage (`pre_battle_damage`, the Thunderhead Tower) on a
    /// set-up's battle, after the opening: each building's row of the attacker's committed units
    /// that did not sit out loses its percent of max HP, less its resistance to the damage type,
    /// never below 1 HP. Lowers `hp` (centi-HP, by set-up index) and logs one event a building.
    /// Draws nothing.
    pub(crate) fn engage_arms_opening(&self, setup: &Setup, sat_out: &[bool], hp: &mut [i32], mut sink: Option<&mut Sink>) {
        let d = self.d;
        let Some(c) = self.engage_arms_city(setup.ctx.to) else { return };
        for b in d.buildings.keys() {
            let Some(pd) = d.buildings[b].battle.pre_battle_damage else { continue };
            if self.city_num_building(c, b) == 0 {
                continue;
            }
            let mut hits: Vec<(FighterIdx, i32)> = Vec::new();
            for i in setup.of(Side::Attacker) {
                if sat_out.get(i).copied().unwrap_or(false) || setup.places[i].1 as usize != pd.row {
                    continue;
                }
                let resist = pd.damage_type.map_or(0, |dt| self.unit_get_damage_type_resist(setup.units[i], dt));
                let pct = pd.percent.max(0) * (100 - resist).max(0) / 100;
                let dmg = (pct * setup.fighters[i].max_hp).min(hp[i] - 100).max(0);
                if dmg > 0 {
                    hp[i] -= dmg;
                    hits.push((i as FighterIdx, dmg));
                }
            }
            if let (Some(s), false) = (sink.as_deref_mut(), hits.is_empty()) {
                s.event(LogEvent::BuildingStrike { building: b, hits });
            }
        }
    }

    /// The attacker's slot change on plot `to` from its improvement
    /// (`engagement_attacker_slots_change`, the Trench), if any.
    pub fn engage_arms_attacker_slots(&self, to: PlotIdx) -> Option<SlotChange> {
        let i = self.map.plots[to].improvement_type?;
        let delta = self.d.improvements[i].engagement_attacker_slots_change;
        (delta != 0).then_some(SlotChange { src: SlotSource::Improvement(i), delta })
    }

    /// The damage type unit `u`'s physical attack deals (`attack_damage_type`, the Elemental
    /// Forge's promotions; card: the Lantern of Lugus's unit type), if a promotion of it or its type
    /// gives one (the promotion's first).
    pub fn unit_attack_damage_type(&self, u: UnitId) -> Option<DamageType> {
        let d = self.d;
        let unit = &self.units[u];
        d.promotions.keys().filter(|&p| unit.has_promotion[p]).find_map(|p| d.promotions[p].attack_damage_type).or(d.units[unit.unit_type].attack_damage_type)
    }

    /// Unit `u`'s physical attack base `own` (x100) against `opponent`: cut by the opponent's
    /// resistance to the damage type its attack deals (none against nobody in particular, or a
    /// Sentinel).
    pub fn engage_arms_attack(&self, u: UnitId, opponent: Option<UnitId>, own: i32) -> i32 {
        match (opponent, self.unit_attack_damage_type(u)) {
            (Some(o), Some(dt)) => (own as i64 * (100 - self.unit_get_damage_type_resist(o, dt)).max(0) as i64 / 100) as i32,
            _ => own,
        }
    }

    /// How far unit `u` bombards: its type's `bombard_range`, at least 1.
    pub fn unit_bombard_range(&self, u: UnitId) -> i32 {
        self.d.units[self.units[u].unit_type].bombard_range.max(1)
    }

    /// The plots unit `u` on `(x, y)` could bombard a city on: the eight next to it (in direction
    /// order), then for a unit with a bombard range (the Siege Colossus) the plots further out to
    /// that range that its team has revealed, row by row.
    pub fn unit_bombard_plots(&self, u: UnitId, x: i32, y: i32) -> Vec<PlotIdx> {
        let mut out: Vec<PlotIdx> = (0..DirectionType::NUM).filter_map(|i| self.plot_direction(x, y, DirectionType::from_usize(i))).collect();
        let range = self.unit_bombard_range(u);
        if range > 1 {
            let team = self.unit_get_team(u);
            for dy in -range..=range {
                for dx in -range..=range {
                    if dx.abs().max(dy.abs()) < 2 {
                        continue;
                    }
                    if let Some(p) = self.map.plot_xy(x + dx, y + dy) {
                        if !out.contains(&p) && self.map.plots[p].revealed[team] {
                            out.push(p);
                        }
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
#[path = "arms_tests.rs"]
mod tests;
