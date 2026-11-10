//! Telemetry for the autopsy of AI games (docs/AUTOPSY.md): facts at the moments games go wrong.
//!
//! - `battle` (event): a snapshot of each engagement as it is launched (each side's committed /
//!   present military, summed strength, the plot's owner), for losses by place and odds;
//! - `city_fell` (event): a city taken by conquest, with the facts around it -- its defenders
//!   when the day's assault began, the captor's units near it, where its owner's army stood,
//!   powers, wars, economy -- and whether it was razed or kept;
//! - `civ_died` (event): a civilization eliminated, with the cities it lost and to whom and its
//!   peak city count;
//! - `army` (counts): a census of each player's land military at the end of each of its turns,
//!   by where it stood (`city`, `home`: own territory outside cities, `enemy`: territory of a team
//!   at war with it, `other`), by whether its team was at war with a civilization (`war_` /
//!   `peace_`), and how many of them spent no movement that turn (`_still`).
//!
//! Only observes, like the rest of telemetry: reads state, draws no random number. The state it
//! keeps (sieges, losses, peaks) lives in the telemetry sink, never in `Game`.

use std::collections::BTreeMap;

use ffh_data::*;

use super::{ev, Obj};
use crate::game::Game;
use crate::ids::*;

/// The defenders of a city plot when the day's first engagement against it was launched.
#[derive(Clone, Copy)]
struct Siege {
    turn: i32,
    /// Defending military on the plot (count, summed strength) before the first battle.
    def_n: i64,
    def_str: i64,
    /// Their summed strength at full hit points.
    def_full: i64,
    /// Engagements launched against it that turn, and the attackers committed (summed).
    battles: i64,
    att_n: i64,
    att_str: i64,
}

/// The autopsy's part of the telemetry sink.
#[derive(Default)]
pub(super) struct Autopsy {
    /// City plot -> its latest siege day.
    sieges: BTreeMap<PlotIdx, Siege>,
    /// `city_fell` records begun at `player_acquire_city` and finished after the raze decision.
    pending: Vec<(CityId, PlayerId, Obj)>,
    /// Player -> cities lost by conquest: (turn, captor).
    losses: BTreeMap<PlayerId, Vec<(i32, PlayerId)>>,
    /// Player -> (peak city count, first turn at that peak).
    peak: BTreeMap<PlayerId, (i32, i32)>,
}

/// Where the old owner's military stood when a city fell (exclusive, in this order).
const WHERE: [&str; 5] = ["in_city", "near", "other_cities", "home_field", "abroad"];

/// `army` count keys: [at war with a civilization][place][still].
const ARMY_KEYS: [[[&str; 2]; 4]; 2] = [
    [
        ["peace_city", "peace_city_still"],
        ["peace_home", "peace_home_still"],
        ["peace_enemy", "peace_enemy_still"],
        ["peace_other", "peace_other_still"],
    ],
    [["war_city", "war_city_still"], ["war_home", "war_home_still"], ["war_enemy", "war_enemy_still"], ["war_other", "war_other_still"]],
];

/// Radius of "near" for the old owner's army (plots).
const NEAR: i32 = 5;
/// Radius of the captor's units counted around a fallen city (plots).
const CAPTOR_NEAR: i32 = 2;

impl Game {
    /// A unit that counts as military for the autopsy: it can fight (the rules' `canFight`: some
    /// strength) and is a land unit on the map.
    fn tel_au_military(&self, u: UnitId) -> bool {
        self.unit_plot_opt(u).is_some() && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && self.unit_can_fight(u)
    }

    /// A unit's strength x100: the rules' contribution value of a unit
    /// ([`Game::crusade_unit_strength`]: the better of its attack and defense strengths, with its
    /// general combat bonus, scaled by its hit points).
    fn tel_au_strength(&self, u: UnitId) -> i64 {
        self.crusade_unit_strength(u) as i64
    }

    /// [`Game::tel_au_strength`] at full hit points.
    fn tel_au_full_strength(&self, u: UnitId) -> i64 {
        let base = self.unit_base_combat_str(u).max(self.unit_base_combat_str_defense(u)) as i64;
        base * (100 + self.units[u].extra_combat_percent as i64).max(10)
    }

    fn tel_au_plot_owner(&self, plot: PlotIdx) -> i64 {
        self.map.plots[plot].owner.map_or(-1, |o| o.0 as i64)
    }

    /// An engagement is launched by `p` from `from` at `to` with `force` (before summons and
    /// strikes). Emits `battle`; for a city plot, keeps the day's first defenders.
    pub(crate) fn tel_autopsy_battle(&self, p: PlayerId, from: PlotIdx, to: PlotIdx, force: &[UnitId]) {
        if !self.telemetry.on() {
            return;
        }
        let turn = self.game_turn();
        let team = self.players[p].team;
        let (mut def_n, mut def_str, mut def_full, mut vs) = (0i64, 0i64, 0i64, None);
        for &u in &self.map.plots[to].units {
            let o = self.units[u].owner;
            if self.at_war(team, self.players[o].team) && self.tel_au_military(u) {
                def_n += 1;
                def_str += self.tel_au_strength(u);
                def_full += self.tel_au_full_strength(u);
                vs.get_or_insert(o);
            }
        }
        let city = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c));
        let vs = city.map(|c| self.cities[c].owner).or(vs);
        let att_n = force.len() as i64;
        let att_str: i64 = force.iter().filter(|&&u| self.units.contains(u)).map(|&u| self.tel_au_strength(u)).sum();
        let (x, y) = self.map.xy(to);
        let (owner, from_owner) = (self.tel_au_plot_owner(to), self.tel_au_plot_owner(from));
        self.telemetry.with(|t| {
            if city.is_some() {
                let s = t.autopsy.sieges.entry(to).or_insert(Siege { turn: -1, def_n: 0, def_str: 0, def_full: 0, battles: 0, att_n: 0, att_str: 0 });
                if s.turn != turn {
                    *s = Siege { turn, def_n, def_str, def_full, battles: 0, att_n: 0, att_str: 0 };
                }
                s.battles += 1;
                s.att_n += att_n;
                s.att_str += att_str;
            }
            t.event(|| {
                ev(turn, "battle", p)
                    .opt("vs", vs.map(|v| v.0 as i64))
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .b("city", city.is_some())
                    .i("owner", owner)
                    .i("from_owner", from_owner)
                    .i("att_n", att_n)
                    .i("att_str", att_str)
                    .i("def_n", def_n)
                    .i("def_str", def_str)
                    .i("def_full", def_full)
                    .end()
            });
        });
    }

    /// `player_acquire_city` by conquest, before anything changes: the facts of `city_fell`
    /// (finished by [`Game::tel_autopsy_city_fell_end`] once the raze decision is made).
    pub(crate) fn tel_autopsy_city_fell(&self, captor: PlayerId, c: CityId) {
        if !self.telemetry.on() {
            return;
        }
        let turn = self.game_turn();
        let old = self.cities[c].owner;
        let plot = self.city_plot(c);
        let (x, y) = self.map.xy(plot);
        let (old_team, new_team) = (self.players[old].team, self.players[captor].team);
        let city = &self.cities[c];

        // Where the old owner's military stood.
        let mut by_where = [[0i64; 2]; 5];
        for &u in &self.players[old].units {
            if !self.tel_au_military(u) {
                continue;
            }
            let up = self.unit_plot_opt(u).expect("military units are on the map");
            let (ux, uy) = self.map.xy(up);
            let w = if up == plot {
                0
            } else if self.plot_distance(x, y, ux, uy) <= NEAR {
                1
            } else if self.plot_is_city(up, false, None) {
                2
            } else if self.map.plots[up].owner == Some(old) {
                3
            } else {
                4
            };
            by_where[w][0] += 1;
            by_where[w][1] += self.tel_au_strength(u);
        }
        // The captor team's military within CAPTOR_NEAR plots (the city's plot included).
        let (mut near_n, mut near_str) = (0i64, 0i64);
        for dx in -CAPTOR_NEAR..=CAPTOR_NEAR {
            for dy in -CAPTOR_NEAR..=CAPTOR_NEAR {
                let Some(pl) = self.map.plot_xy(x + dx, y + dy) else { continue };
                for &u in &self.map.plots[pl].units {
                    if self.players[self.units[u].owner].team == new_team && self.tel_au_military(u) {
                        near_n += 1;
                        near_str += self.tel_au_strength(u);
                    }
                }
            }
        }
        let military = |p: PlayerId| -> (i64, i64) {
            let mut n = 0;
            let mut s = 0;
            for &u in &self.players[p].units {
                if self.tel_au_military(u) {
                    n += 1;
                    s += self.tel_au_strength(u);
                }
            }
            (n, s)
        };
        let captor_mil = military(captor);
        let commerce: Vec<String> = self.players[old].commerce_percent.iter().map(|v| v.to_string()).collect();
        let research = self.player_get_commerce_rate(old, CommerceType::COMMERCE_RESEARCH);
        let gold_rate = self.player_get_commerce_rate(old, CommerceType::COMMERCE_GOLD);
        let producing = self.tel_production_name(c);
        let capital = self.city_is_capital(c);
        let defense = self.city_total_defense(c, false);
        let at_war = self.team_get_at_war_count(old_team, true);
        let barb = self.player_is_barbarian(captor);
        let (host, owner_host) = (self.player_is_host_slot(captor), self.player_is_host_slot(old));
        let (pop, founded, acquired, damage, id) = (city.population, city.game_turn_founded, city.game_turn_acquired, city.defense_damage, c.0);
        let (old_power, new_power) = (self.players[old].power, self.players[captor].power);
        let (old_cities, new_cities) = (self.players[old].num_cities(), self.players[captor].num_cities());
        let gold = self.players[old].gold;
        let mut left = 0i64;
        for &u in &self.map.plots[plot].units {
            if self.units[u].owner == old && self.tel_au_military(u) {
                left += 1;
            }
        }
        self.telemetry.with(|t| {
            let mut o = ev(turn, "city_fell", old)
                .i("to", captor.0 as i64)
                .b("barb", barb)
                .b("host", host)
                .b("owner_host", owner_host)
                .i("city", id as i64)
                .i("x", x as i64)
                .i("y", y as i64)
                .i("pop", pop as i64)
                .i("age", (turn - founded) as i64)
                .i("held", (turn - acquired) as i64)
                .b("capital", capital)
                .i("defense", defense as i64)
                .i("defense_damage", damage as i64)
                .i("left_in_city", left);
            match t.autopsy.sieges.get(&plot).copied() {
                Some(s) => {
                    o = o
                        .i("siege_turn", s.turn as i64)
                        .i("def_n", s.def_n)
                        .i("def_str", s.def_str)
                        .i("def_full", s.def_full)
                        .i("battles", s.battles)
                        .i("att_n", s.att_n)
                        .i("att_str", s.att_str);
                }
                None => {
                    o = o.raw("siege_turn", "null").i("def_n", 0).i("def_str", 0).i("def_full", 0).i("battles", 0).i("att_n", 0).i("att_str", 0);
                }
            }
            o = o.i("captor_near_n", near_n).i("captor_near_str", near_str);
            for (w, name) in WHERE.iter().enumerate() {
                o = o.raw(name, &format!("[{},{}]", by_where[w][0], by_where[w][1]));
            }
            o = o
                .raw("captor_mil", &format!("[{},{}]", captor_mil.0, captor_mil.1))
                .i("power", old_power as i64)
                .i("captor_power", new_power as i64)
                .i("cities", old_cities as i64)
                .i("captor_cities", new_cities as i64)
                .i("wars", at_war as i64);
            match t.war(old_team, new_team) {
                Some((w, side)) => {
                    let start = w.start;
                    o = o.i("war_turns", (turn - start) as i64).b("declared_by_owner", side == 0 && !barb);
                }
                None => o = o.raw("war_turns", "null").raw("declared_by_owner", "null"),
            }
            o = o
                .i("gold", gold as i64)
                .raw("commerce_pct", &format!("[{}]", commerce.join(",")))
                .i("research", research as i64)
                .i("gold_rate", gold_rate as i64)
                .s("producing", producing);
            t.autopsy.losses.entry(old).or_default().push((turn, captor));
            let pk = t.autopsy.peak.entry(old).or_insert((0, turn));
            if old_cities > pk.0 {
                *pk = (old_cities, turn);
            }
            t.autopsy.sieges.remove(&plot);
            t.autopsy.pending.push((c, captor, o));
        });
    }

    /// After the raze decision of a conquest: `new_city` still stands (kept) or not (razed).
    pub(crate) fn tel_autopsy_city_fell_end(&self, new_city: CityId) {
        if !self.telemetry.on() {
            return;
        }
        let kept = self.cities.contains(new_city);
        self.telemetry.with(|t| {
            if let Some((_, _, o)) = t.autopsy.pending.pop() {
                let line = o.b("razed", !kept).end();
                t.event(|| line);
            }
        });
    }

    /// A civilization is eliminated (`player_set_alive(p, false)`).
    pub(crate) fn tel_autopsy_civ_died(&self, p: PlayerId) {
        if !self.telemetry.on() || self.player_is_barbarian(p) || self.player_is_host_slot(p) {
            return;
        }
        let turn = self.game_turn();
        let units = self.players[p].units.len() as i64;
        self.telemetry.with(|t| {
            let losses = t.autopsy.losses.get(&p).cloned().unwrap_or_default();
            let (peak, peak_turn) = t.autopsy.peak.get(&p).copied().unwrap_or((0, 0));
            let mut by: BTreeMap<PlayerId, i64> = BTreeMap::new();
            for &(_, k) in &losses {
                *by.entry(k).or_insert(0) += 1;
            }
            let killer = losses.last().map(|&(_, k)| k.0 as i64);
            let by_s: Vec<String> = by.iter().map(|(k, n)| format!("\"{}\":{}", k.0, n)).collect();
            let chain: Vec<String> = losses.iter().map(|(tn, k)| format!("[{},{}]", tn, k.0)).collect();
            t.event(|| {
                ev(turn, "civ_died", p)
                    .opt("killer", killer)
                    .i("lost", losses.len() as i64)
                    .raw("lost_to", &format!("{{{}}}", by_s.join(",")))
                    .raw("chain", &format!("[{}]", chain.join(",")))
                    .i("peak_cities", peak as i64)
                    .i("peak_turn", peak_turn as i64)
                    .i("units", units)
                    .end()
            });
        });
    }

    /// The end of `p`'s turn: the census of its land military (`army` counts; each key's count
    /// is the player-turns in that war state, its sum the unit-turns), and its peak city count.
    pub(crate) fn tel_autopsy_census(&self, p: PlayerId) {
        if !self.telemetry.on() {
            return;
        }
        let team = self.players[p].team;
        let war = (self.team_get_at_war_count(team, true) > 0) as usize;
        // [place][still]
        let mut n = [[0i64; 2]; 4];
        for &u in &self.players[p].units {
            if !self.tel_au_military(u) {
                continue;
            }
            let plot = self.unit_plot_opt(u).expect("military units are on the map");
            let place = if self.plot_is_city(plot, false, None) {
                0
            } else {
                match self.plot_get_team(plot) {
                    Some(ot) if ot == team => 1,
                    Some(ot) if self.at_war(team, ot) => 2,
                    _ => 3,
                }
            };
            n[place][0] += 1;
            if self.units[u].moves == 0 {
                n[place][1] += 1;
            }
        }
        let cities = self.players[p].num_cities();
        let turn = self.game_turn();
        self.telemetry.with(|t| {
            for place in 0..4 {
                for still in 0..2 {
                    t.count(p, "army", ARMY_KEYS[war][place][still], n[place][still]);
                }
            }
            let pk = t.autopsy.peak.entry(p).or_insert((0, turn));
            if cities > pk.0 {
                *pk = (cities, turn);
            }
        });
    }
}
