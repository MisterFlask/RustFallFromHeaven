//! Sieges and Sentinels (docs/SIEGES.md; the owner's design, 2026-11-04).
//!
//! **Sentinels.** A city with a wall building (`iSentinels` on Palisade, Walls, Wall of Stone) has
//! that many Sentinels (the most any of its buildings gives): wall sections that fight in the
//! defender's front row of every battle for the city. They are not units: the city keeps each
//! one's damage (`City::sentinel_damage`, percent of its max HP, 100 breached) and the engagement
//! adds them as fighters ([`super::engage`]). Their values are `UNIT_SENTINEL`'s (`iHP`,
//! `iAttack`, `iDefense`, its rows and speed), their max HP and Dv grown by the city's total
//! defence: `x (100 + defence) / 100`. An occupied city's Sentinels do not fight.
//!
//! **Siege.** A city is besieged while a plot next to it holds a land unit at war with its owner
//! that can fight and that its team sees ([`Game::city_is_besieged`]): a pure test of the map,
//! nothing saved. A besieged city's Sentinels do not repair and its units get no heal from the
//! city (`unit_heal_rate`).
//!
//! **Repair.** At the city's turn each Sentinel repairs `SENTINEL_REPAIR_PERCENT` of its max HP
//! unless the city is besieged. Sentinels a city gains start whole, or breached if it is besieged
//! then; a conquered city's are breached.
//!
//! **Bombard** ([`Game::city_bombard_damage`]): a bombard hits every Sentinel (its rate, less the
//! walls' bombard defence, in percent of max HP) and every land unit at war with the bomber on the
//! city's plot (half that, less its collateral protection, never below
//! `SIEGE_BOMBARD_FLOOR_PERCENT` of its HP).

use ffh_data::*;

use super::formation::Side;
use super::report::{Immunities, LogFighter, Stat, Term, TermSource};
use crate::game::Game;
use crate::ids::*;

impl Game {
    /// `UNIT_SENTINEL`, whose values the Sentinels fight with.
    pub fn sentinel_unit_type(&self) -> UnitType {
        self.d.t::<UnitType>("UNIT_SENTINEL")
    }

    /// City `c`'s Sentinels: the most any of its buildings gives (`iSentinels`).
    pub fn city_sentinel_count(&self, c: CityId) -> usize {
        let d = self.d;
        d.buildings.keys().filter(|&b| d.buildings[b].sentinels > 0 && self.city_num_building(c, b) > 0).map(|b| d.buildings[b].sentinels as usize).max().unwrap_or(0)
    }

    /// Card: the Free City Charter (docs/CARDS.md). The percent more defence city `c` gives the
    /// units fighting in it: the most any of its buildings gives (`defender_defense`).
    pub fn city_defender_defense(&self, c: CityId) -> i32 {
        static WITH: std::sync::OnceLock<Vec<BuildingType>> = std::sync::OnceLock::new();
        let d = self.d;
        let with = WITH.get_or_init(|| d.buildings.keys().filter(|&b| d.buildings[b].defender_defense != 0).collect());
        with.iter().filter(|&&b| self.city_num_building(c, b) > 0).map(|&b| d.buildings[b].defender_defense).max().unwrap_or(0)
    }

    /// Sentinel `k`'s damage in percent of its max HP (100: breached).
    pub fn city_sentinel_damage(&self, c: CityId, k: usize) -> i32 {
        self.cities[c].sentinel_damage.get(k).copied().unwrap_or(0).clamp(0, 100)
    }

    /// The city's total defence as the Sentinels grow with it (never below 0).
    fn city_sentinel_defense(&self, c: CityId) -> i32 {
        self.city_total_defense(c, false).max(0)
    }

    /// `UNIT_SENTINEL`'s stat `s` (its sheet's).
    pub fn sentinel_stat(&self, s: StatType) -> i32 {
        self.d.unit_stat(self.sentinel_unit_type(), s)
    }

    /// A Sentinel's max HP in a battle: `UNIT_SENTINEL`'s HP x (100 + the city's defence) / 100.
    pub fn city_sentinel_max_hp(&self, c: CityId) -> i32 {
        (self.sentinel_stat(StatType::Hp).max(1) * (100 + self.city_sentinel_defense(c)) / 100).max(1)
    }

    /// A Sentinel's Dv, x100: `UNIT_SENTINEL`'s defence, x10, x (100 + the city's defence) / 100.
    pub fn city_sentinel_dv_x100(&self, c: CityId) -> i32 {
        (self.sentinel_stat(StatType::Defense) * 10 * (100 + self.city_sentinel_defense(c)) / 100).max(1)
    }

    /// A Sentinel's magic defence, x100: `UNIT_SENTINEL`'s, x10, grown by the city's defence too.
    pub fn city_sentinel_md_x100(&self, c: CityId) -> i32 {
        (self.sentinel_stat(StatType::MagicDefense) * 10 * (100 + self.city_sentinel_defense(c)) / 100).max(1)
    }

    /// A Sentinel's A, x100: `UNIT_SENTINEL`'s attack, x10.
    pub fn sentinel_a_x100(&self) -> i32 {
        (self.sentinel_stat(StatType::Attack) * 10).max(1)
    }

    /// The Sentinels of city `c` that can fight: not breached, the city not occupied.
    pub fn city_sentinels_standing(&self, c: CityId) -> Vec<usize> {
        if self.city_is_occupation(c) {
            return Vec::new();
        }
        (0..self.city_sentinel_count(c)).filter(|&k| self.city_sentinel_damage(c, k) < 100).collect()
    }

    /// The city on plot `to` and its Sentinels that fight for it (none: `None`).
    pub fn engage_city_sentinels(&self, to: PlotIdx) -> Option<(CityId, Vec<usize>)> {
        let c = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c))?;
        let ks = self.city_sentinels_standing(c);
        (!ks.is_empty()).then_some((c, ks))
    }

    /// A Sentinel of city `c` in cell `place` with `hp` (centi-HP) as the battle log and the plan
    /// show it: `UNIT_SENTINEL`, the city's owner, its values in a battle (docs/SIEGES.md).
    pub fn city_sentinel_log_fighter(&self, c: CityId, place: (u8, u8), hp: i32) -> LogFighter {
        let d = self.d;
        let ty = self.sentinel_unit_type();
        let owner = self.cities[c].owner;
        let defense = if self.city_is_occupation(c) { 0 } else { self.city_sentinel_defense(c) };
        let a = self.sentinel_a_x100();
        let base_dv = self.sentinel_stat(StatType::Defense) * 10;
        let base_md = self.sentinel_stat(StatType::MagicDefense) * 10;
        let mag = self.sentinel_stat(StatType::Magic) * 10;
        LogFighter {
            unit: LogFighter::SENTINEL_UNIT,
            sentinel: true,
            unit_type: ty,
            name: String::new(),
            owner,
            shown_owner: owner,
            side: Side::Defender,
            place: Some(place),
            large: false,
            level: 0,
            xp: 0,
            hero: false,
            max_hp: self.city_sentinel_max_hp(c),
            hp_start: hp,
            hp_battle: hp,
            speed: self.sentinel_stat(StatType::Speed),
            first_strikes: 0,
            chance_first_strikes: 0,
            rolled_first_strikes: 0,
            actions: d.engage_actions(ty, false),
            spell: None,
            accuracy: self.sentinel_stat(StatType::Accuracy),
            evasion: self.sentinel_stat(StatType::Evasion),
            crit: self.sentinel_stat(StatType::Crit),
            anti_flank: false,
            flanker: false,
            withdraw_pct: 0,
            combat_limit: 100,
            collateral_limit: None,
            def_strike: None,
            immune: Immunities::default().with(Immunities::DEF_STRIKE, true).with(Immunities::FEAR, true),
            fear: false,
            a: Stat { base_x100: a, terms: Vec::new(), total_x100: a },
            dv: Stat { base_x100: base_dv, terms: vec![Term { src: TermSource::CityDefense, pct: defense }], total_x100: self.city_sentinel_dv_x100(c) },
            mag: Stat { base_x100: mag, terms: Vec::new(), total_x100: mag },
            mdef: Stat { base_x100: base_md, terms: vec![Term { src: TermSource::CityDefense, pct: defense }], total_x100: self.city_sentinel_md_x100(c) },
            fate: None,
            dealt: 0,
            taken: 0,
            healed: 0,
        }
    }

    /// Whether plot `to` holds a city of a team `team` is at war with whose Sentinels stand: an
    /// attack there meets them even with no unit there.
    pub fn plot_has_enemy_sentinels(&self, to: PlotIdx, team: TeamId) -> bool {
        let Some(c) = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c)) else { return false };
        let ct = self.city_team(c);
        ct != team && self.teams[team].at_war[ct] && !self.city_sentinels_standing(c).is_empty()
    }

    /// The units besieging city `c` (module comment): on a plot next to it, on land, at war with
    /// its owner, able to fight, seen by its team. In plot order.
    pub fn city_besiegers(&self, c: CityId) -> Vec<UnitId> {
        let ct = self.city_team(c);
        let plot = self.city_plot(c);
        let (x, y) = self.map.xy(plot);
        let mut out = Vec::new();
        for i in 0..DirectionType::NUM {
            let Some(lp) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
            if self.plot_is_water(lp) {
                continue;
            }
            for &u in &self.map.plots[lp].units {
                if self.unit_is_besieger(u, ct, lp) {
                    out.push(u);
                }
            }
        }
        out
    }

    fn unit_is_besieger(&self, u: UnitId, ct: TeamId, at: PlotIdx) -> bool {
        self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && !self.unit_is_dead(u)
            && self.unit_base_combat_str(u) > 0
            && self.unit_is_enemy(u, Some(ct), Some(at))
            && !self.unit_is_invisible(u, ct, false)
    }

    /// Whether city `c` is besieged (module comment).
    pub fn city_is_besieged(&self, c: CityId) -> bool {
        let ct = self.city_team(c);
        let plot = self.city_plot(c);
        let (x, y) = self.map.xy(plot);
        (0..DirectionType::NUM).any(|i| {
            self.plot_direction(x, y, DirectionType::from_usize(i))
                .is_some_and(|lp| !self.plot_is_water(lp) && self.map.plots[lp].units.iter().any(|&u| self.unit_is_besieger(u, ct, lp)))
        })
    }

    /// Sets Sentinel `k`'s damage (percent of its max HP, clamped to 0..=100).
    pub fn city_set_sentinel_damage(&mut self, c: CityId, k: usize, damage: i32) {
        let v = &mut self.cities[c].sentinel_damage;
        if v.len() <= k {
            v.resize(k + 1, 0);
        }
        v[k] = damage.clamp(0, 100);
    }

    /// Brings the city's damage list to its Sentinel count: Sentinels it gains start whole, or
    /// breached if the city is besieged; those it loses are dropped.
    pub fn city_sync_sentinels(&mut self, c: CityId) {
        let n = self.city_sentinel_count(c);
        let have = self.cities[c].sentinel_damage.len();
        if n > have {
            let start = if self.city_is_besieged(c) { 100 } else { 0 };
            self.cities[c].sentinel_damage.resize(n, start);
        } else if n < have {
            self.cities[c].sentinel_damage.truncate(n);
        }
    }

    /// The city's turn (module comment): the Sentinels repair unless it is besieged.
    pub fn city_repair_sentinels(&mut self, c: CityId) {
        self.city_sync_sentinels(c);
        if self.cities[c].sentinel_damage.is_empty() || self.city_is_besieged(c) {
            return;
        }
        // This game's building suitability (docs/SUITABILITY.md): a Legendary Palisade repairs faster.
        let pct = (self.d.defines.sentinel_repair_percent + self.city_sentinel_repair_change(c)).max(0);
        for dmg in self.cities[c].sentinel_damage.iter_mut() {
            *dmg = (*dmg - pct).max(0);
        }
    }

    /// A conquered city's Sentinels are breached.
    pub fn city_breach_sentinels(&mut self, c: CityId) {
        self.city_sync_sentinels(c);
        for dmg in self.cities[c].sentinel_damage.iter_mut() {
            *dmg = 100;
        }
    }

    /// The soldiers a bombard by `u` hits in city `c` (module comment): land units on the city's
    /// plot at war with it, in plot order.
    pub fn city_bombard_soldiers(&self, c: CityId, u: UnitId) -> Vec<UnitId> {
        let plot = self.city_plot(c);
        let team = self.unit_get_team(u);
        self.map.plots[plot]
            .units
            .iter()
            .copied()
            .filter(|&d| self.unit_get_domain_type(d) == DomainType::DOMAIN_LAND && !self.unit_is_dead(d) && self.unit_is_enemy(d, Some(team), Some(plot)))
            .collect()
    }

    /// The damage a soldier takes from a bombard of `pct` percent (module comment): half of it,
    /// less its collateral protection, never below the floor; `0` when it is at the floor.
    pub fn bombard_soldier_damage(&self, d: UnitId, pct: i32) -> i32 {
        let floor_damage = 100 - self.d.defines.siege_bombard_floor_percent.clamp(0, 100);
        let protection = self.units[d].collateral_damage_protection.clamp(0, 100);
        let dmg = pct / 2 * (100 - protection) / 100;
        let now = self.units[d].damage;
        (now + dmg).min(floor_damage.max(now)) - now
    }

    /// Whether a bombard on city `c` by `u` would hurt anything: a Sentinel standing, or a
    /// soldier above the floor.
    pub fn city_bombard_hurts(&self, c: CityId, u: UnitId) -> bool {
        !self.city_sentinels_standing(c).is_empty() || self.city_bombard_soldiers(c, u).iter().any(|&d| self.bombard_soldier_damage(d, 100) > 0)
    }

    /// A bombard of `pct` percent on city `c` by `u` (module comment): every Sentinel standing
    /// takes `pct` of its max HP, every soldier its share. Returns the Sentinels hit and the
    /// soldiers hurt.
    pub fn city_bombard_damage(&mut self, c: CityId, u: UnitId, pct: i32) -> (i32, i32) {
        self.city_sync_sentinels(c);
        let mut walls = 0;
        for k in self.city_sentinels_standing(c) {
            let now = self.city_sentinel_damage(c, k);
            self.city_set_sentinel_damage(c, k, now + pct.max(0));
            walls += 1;
        }
        let mut soldiers = 0;
        for d in self.city_bombard_soldiers(c, u) {
            let dmg = self.bombard_soldier_damage(d, pct.max(0));
            if dmg > 0 {
                let owner = self.units[u].owner;
                self.unit_change_damage(d, dmg, Some(owner));
                soldiers += 1;
            }
        }
        (walls, soldiers)
    }
}
