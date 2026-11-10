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
//!   `peace_`), and how many of them spent no movement that turn (`_still`);
//! - naval facts (the second half of this file): a census of each player's sea units by class,
//!   place and war state, what its transports carry, its warships near enemy coasts; every fight
//!   with a sea unit on either side (`sea_fight`); units loaded and unloaded, each transport trip
//!   (`trip`), each unload outside a port (`unload`: a landing when the owner has no city on that
//!   landmass); sea improvements pillaged or destroyed; the geography of the game (`geo`) and
//!   each player's coastal cities over time (`coast`).
//!
//! Only observes, like the rest of telemetry: reads state, draws no random number. The state it
//! keeps (sieges, losses, peaks) lives in the telemetry sink, never in `Game`.

use std::collections::{BTreeMap, BTreeSet};

use ffh_data::*;

use super::{enum_name, ev, Obj};
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
    /// Naval: the plot a cargo unit is moving to as it leaves its transport (set by
    /// `unit_set_xy` just before the unload, read by the unload).
    unload_to: Option<(UnitId, PlotIdx)>,
    /// Naval: unit -> the turn it last came ashore from a ship outside a port of its team.
    landed: BTreeMap<UnitId, i32>,
    /// Naval: transport -> its open trip (from the first unit loaded until it is empty again).
    trips: BTreeMap<UnitId, Trip>,
    /// Naval: (player, first-kind) already emitted (`naval_first`).
    firsts: BTreeSet<(PlayerId, usize)>,
    /// Naval: cargo units that died aboard (their final `kill` comes before they leave the ship).
    dead_cargo: BTreeSet<UnitId>,
    /// Naval: the `geo` event was emitted.
    geo_done: bool,
}

/// A transport's trip: from the first unit loaded until its cargo is empty again.
#[derive(Clone, Copy)]
struct Trip {
    p: PlayerId,
    start: i32,
    /// The first end of an owner's turn it was seen outside a port of its team.
    left_port: Option<i32>,
    military: bool,
    settler: bool,
    max_cargo: i64,
    /// Units put ashore by where (`UNLOAD_WHERE`), and units that died aboard.
    ashore: [i64; 4],
    died: i64,
    last: Option<PlotIdx>,
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
        let mut near_units: Vec<UnitId> = Vec::new();
        for dx in -CAPTOR_NEAR..=CAPTOR_NEAR {
            for dy in -CAPTOR_NEAR..=CAPTOR_NEAR {
                let Some(pl) = self.map.plot_xy(x + dx, y + dy) else { continue };
                for &u in &self.map.plots[pl].units {
                    if self.players[self.units[u].owner].team == new_team && self.tel_au_military(u) {
                        near_n += 1;
                        near_str += self.tel_au_strength(u);
                        near_units.push(u);
                    }
                }
            }
        }
        // Naval: the city is coastal; the captor holds another city on its landmass.
        let coastal = self.city_is_coastal(c, self.d.defines.min_water_size_for_ocean);
        let area = self.city_area(c);
        let captor_land_city = self.map.areas.contains(area) && self.map.areas[area].cities_per_player[captor] > 0;
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
            let by_sea = near_units.iter().filter(|u| t.autopsy.landed.get(u).is_some_and(|&lt| turn - lt <= SEA_ARRIVAL_TURNS)).count();
            o = o.b("coastal", coastal).i("captor_near_by_sea", by_sea as i64).b("captor_land_city", captor_land_city);
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
        self.tel_naval_census(p);
    }
}

// ----------------------------------------------------------------------------
// Naval facts (docs/AUTOPSY.md, "Naval")
// ----------------------------------------------------------------------------

/// Sea unit classes, from the rules' unit info: `transport` (room for land units), `warship`
/// (some strength, no room), `other`.
const SEA_CLS: [&str; 3] = ["transport", "warship", "other"];

/// `sea` count keys: [at war with a civilization][class][place: in a port of its team, at sea in
/// its team's waters, at sea elsewhere][still].
const SEA_KEYS: [[[[&str; 2]; 3]; 3]; 2] = [
    [
        [["peace_transport_port", "peace_transport_port_still"], ["peace_transport_own", "peace_transport_own_still"], ["peace_transport_sea", "peace_transport_sea_still"]],
        [["peace_warship_port", "peace_warship_port_still"], ["peace_warship_own", "peace_warship_own_still"], ["peace_warship_sea", "peace_warship_sea_still"]],
        [["peace_other_port", "peace_other_port_still"], ["peace_other_own", "peace_other_own_still"], ["peace_other_sea", "peace_other_sea_still"]],
    ],
    [
        [["war_transport_port", "war_transport_port_still"], ["war_transport_own", "war_transport_own_still"], ["war_transport_sea", "war_transport_sea_still"]],
        [["war_warship_port", "war_warship_port_still"], ["war_warship_own", "war_warship_own_still"], ["war_warship_sea", "war_warship_sea_still"]],
        [["war_other_port", "war_other_port_still"], ["war_other_own", "war_other_own_still"], ["war_other_sea", "war_other_sea_still"]],
    ],
];

/// `sea_cargo` keys: [war][what a transport carries: military, a settler, other units, nothing].
const CARGO_KEYS: [[&str; 4]; 2] = [
    ["peace_military", "peace_settler", "peace_other", "peace_empty"],
    ["war_military", "war_settler", "war_other", "war_empty"],
];

/// What a land unit put aboard or ashore is: a fighter, a settler, anything else.
const CARGO_KIND: [&str; 3] = ["military", "settler", "other"];

/// Where a cargo unit left its ship: a port (a city of its team), land of a landmass where its
/// owner has a city, land of a landmass where it has none (a landing), anywhere else (water: onto
/// another ship). Units that die aboard are counted `died_aboard`.
const UNLOAD_WHERE: [&str; 4] = ["port", "home", "landing", "other"];

/// `sea_unload` keys: [kind][where].
const UNLOAD_KEYS: [[&str; 4]; 3] = [
    ["military_port", "military_home", "military_landing", "military_other"],
    ["settler_port", "settler_home", "settler_landing", "settler_other"],
    ["other_port", "other_home", "other_landing", "other_other"],
];

/// `naval_first` kinds: the first end of a turn the player had a sea unit, one able to enter every
/// water terrain (no water terrain impassable to it), a warship-class unit, a transport able to
/// enter every water terrain.
const FIRST_KIND: [&str; 4] = ["ship", "ocean_ship", "warship", "ocean_transport"];

/// Turns after coming ashore during which a unit counts as having arrived by sea (`city_fell`'s
/// `captor_near_by_sea`).
const SEA_ARRIVAL_TURNS: i32 = 10;
/// Radius (plots) around an unload in which cities of others are listed (`unload`'s `cities`).
const UNLOAD_CITY_RADIUS: i32 = 2;
/// The `coast` sample is taken at the end of each civilization's turn every this many turns.
const COAST_SAMPLE_EVERY: i32 = 10;

/// A trip's outcome name.
fn trip_kind(tr: &Trip) -> &'static str {
    if tr.military {
        "military"
    } else if tr.settler {
        "settler"
    } else {
        "other"
    }
}

fn trip_event(turn: i32, ship: UnitId, tr: &Trip, outcome: &str, died: i64, end: (i32, i32)) -> String {
    ev(turn, "trip", tr.p)
        .i("ship", ship.0 as i64)
        .i("start", tr.start as i64)
        .opt("left_port", tr.left_port.map(|v| v as i64))
        .s("kind", trip_kind(tr))
        .i("max_cargo", tr.max_cargo)
        .s("outcome", outcome)
        .raw("ashore", &format!("[{},{},{},{}]", tr.ashore[0], tr.ashore[1], tr.ashore[2], tr.ashore[3]))
        .i("died", died)
        .i("x", end.0 as i64)
        .i("y", end.1 as i64)
        .end()
}

impl Game {
    /// A sea unit type's class index in [`SEA_CLS`].
    fn tel_nv_class(&self, ut: UnitType) -> usize {
        let info = &self.d.units[ut];
        if info.cargo_space > 0 && info.domain_cargo == Some(DomainType::DOMAIN_LAND) {
            0
        } else if info.combat > 0 {
            1
        } else {
            2
        }
    }

    /// The unit type can enter every water terrain (no water terrain is impassable to it).
    fn tel_nv_ocean(&self, ut: UnitType) -> bool {
        !self.d.units[ut].terrain_impassable.iter_enumerated().any(|(t, &imp)| imp && self.d.terrains[t].water)
    }

    /// A land unit as cargo: index in [`CARGO_KIND`].
    fn tel_nv_kind(&self, u: UnitId) -> usize {
        if self.d.units[self.units[u].unit_type].found {
            1
        } else if self.unit_can_fight(u) {
            0
        } else {
            2
        }
    }

    fn tel_nv_is_civ(&self, p: PlayerId) -> bool {
        !self.player_is_barbarian(p) && !self.player_is_host_slot(p)
    }

    /// The plot holds a city of team `team`.
    fn tel_nv_port(&self, plot: PlotIdx, team: TeamId) -> bool {
        self.map.plots[plot].plot_city.is_some_and(|c| self.cities.contains(c) && self.players[self.cities[c].owner].team == team)
    }

    /// Where a cargo unit of `p` left its ship onto `plot`: index in [`UNLOAD_WHERE`].
    fn tel_nv_where(&self, p: PlayerId, plot: PlotIdx) -> usize {
        if self.tel_nv_port(plot, self.players[p].team) {
            0
        } else if self.plot_is_water(plot) {
            3
        } else if self.map.plots[plot].area.is_some_and(|a| self.map.areas.contains(a) && self.map.areas[a].cities_per_player[p] > 0) {
            1
        } else {
            2
        }
    }

    /// Teams `a` and `b` both hold a city on some landmass.
    pub(super) fn tel_nv_share_land(&self, a: TeamId, b: TeamId) -> bool {
        let members = |t: TeamId| -> Vec<PlayerId> { self.player_ids().into_iter().filter(|&p| self.players[p].team == t).collect() };
        let (ma, mb) = (members(a), members(b));
        self.map.areas.iter().any(|(_, ar)| !ar.water && ma.iter().any(|&p| ar.cities_per_player[p] > 0) && mb.iter().any(|&p| ar.cities_per_player[p] > 0))
    }

    /// `unit_set_xy` is about to take cargo unit `u` off its transport onto `to`.
    pub(crate) fn tel_naval_unload_to(&self, u: UnitId, to: PlotIdx) {
        self.telemetry.with(|t| t.autopsy.unload_to = Some((u, to)));
    }

    /// Unit `u` left `old` (unloaded) and/or boarded `new` (`unit_set_transport_unit`, after the
    /// change). Counts `sea_load` / `sea_unload`, keeps the transports' trips (`trip` events),
    /// and emits `unload` for each unload onto land outside a port.
    pub(crate) fn tel_naval_cargo(&self, u: UnitId, old: Option<UnitId>, new: Option<UnitId>) {
        if !self.telemetry.on() || self.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return;
        }
        let p = self.units[u].owner;
        let kind = self.tel_nv_kind(u);
        let turn = self.game_turn();
        let sea = |t: UnitId| self.units.contains(t) && self.unit_get_domain_type(t) == DomainType::DOMAIN_SEA;
        if let Some(ot) = old.filter(|&t| sea(t)) {
            let cargo_left = self.units[ot].cargo as i64;
            let here = self.unit_plot_opt(u);
            self.telemetry.with(|t| {
                let to = match t.autopsy.unload_to.take() {
                    Some((v, to)) if v == u => Some(to),
                    _ => here,
                };
                let dying = t.autopsy.dead_cargo.remove(&u) || t.kill_cause.contains_key(&u);
                let w = if dying { None } else { to.map(|q| self.tel_nv_where(p, q)) };
                match w {
                    Some(w) => {
                        t.count(p, "sea_unload", UNLOAD_KEYS[kind][w], 0);
                        if w == 1 || w == 2 {
                            t.autopsy.landed.insert(u, turn);
                        }
                    }
                    None => t.count(p, "sea_unload", "died_aboard", 0),
                }
                if let (Some(w @ (1 | 2)), Some(q)) = (w, to) {
                    let (x, y) = self.map.xy(q);
                    let mut near = Vec::new();
                    for dx in -UNLOAD_CITY_RADIUS..=UNLOAD_CITY_RADIUS {
                        for dy in -UNLOAD_CITY_RADIUS..=UNLOAD_CITY_RADIUS {
                            let Some(cp) = self.map.plot_xy(x + dx, y + dy) else { continue };
                            let Some(c) = self.map.plots[cp].plot_city.filter(|&c| self.cities.contains(c)) else { continue };
                            let o = self.cities[c].owner;
                            if o == p {
                                continue;
                            }
                            let (cx, cy) = self.map.xy(cp);
                            let war = self.at_war(self.players[p].team, self.players[o].team);
                            near.push(format!("[{},{},{},{},{}]", cx, cy, o.0, self.plot_distance(x, y, cx, cy), war as i32));
                        }
                    }
                    t.event(|| {
                        ev(turn, "unload", p)
                            .i("x", x as i64)
                            .i("y", y as i64)
                            .s("kind", CARGO_KIND[kind])
                            .s("where", UNLOAD_WHERE[w])
                            .i("ship", ot.0 as i64)
                            .raw("cities", &format!("[{}]", near.join(",")))
                            .end()
                    });
                }
                if let Some(tr) = t.autopsy.trips.get_mut(&ot) {
                    match w {
                        Some(w) => tr.ashore[w] += 1,
                        None => tr.died += 1,
                    }
                    if to.is_some() {
                        tr.last = to;
                    }
                    if cargo_left == 0 {
                        let tr = t.autopsy.trips.remove(&ot).expect("the trip");
                        let outcome = if tr.died > 0 {
                            "lost_at_sea"
                        } else if tr.ashore[2] > 0 {
                            "landing"
                        } else if tr.ashore[1] > 0 {
                            "home"
                        } else if tr.ashore[0] > 0 {
                            "port"
                        } else {
                            "other"
                        };
                        let end = tr.last.map_or((-1, -1), |q| self.map.xy(q));
                        t.event(|| trip_event(turn, ot, &tr, outcome, tr.died, end));
                    }
                }
            });
        }
        if let Some(nt) = new.filter(|&t| sea(t)) {
            let cargo = self.units[nt].cargo as i64;
            let tp = self.units[nt].owner;
            self.telemetry.with(|t| {
                t.count(p, "sea_load", CARGO_KIND[kind], 0);
                if !t.autopsy.trips.contains_key(&nt) {
                    t.count(tp, "sea_trip", "start", 0);
                }
                let tr = t.autopsy.trips.entry(nt).or_insert(Trip {
                    p: tp,
                    start: turn,
                    left_port: None,
                    military: false,
                    settler: false,
                    max_cargo: 0,
                    ashore: [0; 4],
                    died: 0,
                    last: None,
                });
                tr.military |= kind == 0;
                tr.settler |= kind == 1;
                tr.max_cargo = tr.max_cargo.max(cargo);
            });
        }
    }

    /// A unit leaves the game: forget it (a transport's trip still open ends `ship_lost`, its
    /// cargo aboard counted with the units that died).
    pub(crate) fn tel_naval_unit_died(&self, u: UnitId) {
        let turn = self.game_turn();
        let cargo = self.units[u].cargo as i64;
        let aboard = self.units[u].transport_unit.is_some();
        self.telemetry.with(|t| {
            t.autopsy.landed.remove(&u);
            if aboard {
                t.autopsy.dead_cargo.insert(u);
            }
            if let Some(tr) = t.autopsy.trips.remove(&u) {
                t.event(|| trip_event(turn, u, &tr, "ship_lost", tr.died + cargo, (-1, -1)));
            }
        });
    }

    /// A duel (`tel_combat`) with a sea unit on either side or on a water plot: `sea_fight`.
    pub(crate) fn tel_naval_combat(&self, att: UnitId, def: UnitId, odds: i32, undefended: bool, plot: PlotIdx) {
        let (da, dd) = (self.unit_get_domain_type(att), self.unit_get_domain_type(def));
        if da != DomainType::DOMAIN_SEA && dd != DomainType::DOMAIN_SEA && !self.plot_is_water(plot) {
            return;
        }
        let turn = self.game_turn();
        let (pa, pd) = (self.units[att].owner, self.units[def].owner);
        let cls = |u: UnitId, d: DomainType| -> &'static str {
            if d == DomainType::DOMAIN_SEA {
                SEA_CLS[self.tel_nv_class(self.units[u].unit_type)]
            } else if d == DomainType::DOMAIN_LAND {
                "land"
            } else {
                "air"
            }
        };
        let (ca, cd) = (cls(att, da), cls(def, dd));
        let (att_dead, def_dead) = (self.unit_is_dead(att), self.unit_is_dead(def));
        let res = if att_dead {
            "loss"
        } else if def_dead {
            "win"
        } else {
            "withdraw"
        };
        // Cargo that goes down with a sunk ship.
        let drowned = |u: UnitId, dead: bool| -> i64 { if dead { self.unit_get_cargo_units(u).len() as i64 } else { 0 } };
        let (drown_a, drown_d) = (drowned(att, att_dead), drowned(def, def_dead));
        let water = self.plot_is_water(plot);
        let waters = match self.plot_get_team(plot) {
            None => "none",
            Some(t) if t == self.players[pa].team => "att",
            Some(t) if t == self.players[pd].team => "def",
            Some(_) => "other",
        };
        let ai = |u: UnitId| enum_name(UnitAIType::NAMES, self.units[u].ai_type.map_or(0, |a| a.idx()));
        let (aa, ad) = (ai(att), ai(def));
        let (barb_a, barb_d) = (self.player_is_barbarian(pa), self.player_is_barbarian(pd));
        let animal_d = self.d.units[self.units[def].unit_type].animal;
        let (x, y) = self.map.xy(plot);
        self.telemetry.with(|t| {
            t.event(|| {
                ev(turn, "sea_fight", pa)
                    .i("vs", pd.0 as i64)
                    .s("att", ca)
                    .s("def", cd)
                    .s("att_ai", aa)
                    .s("def_ai", ad)
                    .i("odds", odds as i64)
                    .b("undefended", undefended)
                    .s("res", res)
                    .i("drowned_att", drown_a)
                    .i("drowned_def", drown_d)
                    .b("barb_att", barb_a)
                    .b("barb_def", barb_d)
                    .b("animal_def", animal_d)
                    .b("water", water)
                    .s("waters", waters)
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .end()
            });
        });
    }

    /// `unit_pillage` took the improvement of water plot `plot` (owned by `owner`).
    pub(crate) fn tel_naval_pillage(&self, u: UnitId, plot: PlotIdx, owner: Option<PlayerId>) {
        let turn = self.game_turn();
        let p = self.units[u].owner;
        let (x, y) = self.map.xy(plot);
        let bonus = self.map.plots[plot].bonus_type.is_some();
        let barb = self.player_is_barbarian(p);
        let cls = if self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA { SEA_CLS[self.tel_nv_class(self.units[u].unit_type)] } else { "land" };
        self.telemetry.with(|t| {
            t.count(p, "sea_pillage", if barb { "by_barbarian" } else { "by_civ" }, 0);
            t.event(|| {
                ev(turn, "sea_pillage", p)
                    .opt("vs", owner.map(|o| o.0 as i64))
                    .b("barb", barb)
                    .b("bonus", bonus)
                    .s("unit", cls)
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .end()
            });
        });
    }

    /// `plot_set_improvement_type` removes the improvement of an owned water plot: counted for
    /// the owner (`sea_imp_lost`, keyed by what the rules were doing; the sum: on a resource).
    pub(crate) fn tel_naval_imp_lost(&self, plot: PlotIdx, owner: PlayerId) {
        let bonus = self.map.plots[plot].bonus_type.is_some() as i64;
        self.telemetry.with(|t| {
            let cause = t.ctx.last().copied().unwrap_or("other");
            t.count(owner, "sea_imp_lost", cause, bonus);
        });
    }

    /// A unit built: ships by class (`ship_built`, `ship_built_reach`; the sum their cost) and
    /// the hammers of land units and ships (`hammers`).
    pub(crate) fn tel_naval_built_unit(&self, u: UnitId) {
        let p = self.units[u].owner;
        let ut = self.units[u].unit_type;
        let cost = self.player_get_production_needed_unit(p, ut) as i64;
        let sea = self.unit_get_domain_type(u) == DomainType::DOMAIN_SEA;
        let cls = SEA_CLS[self.tel_nv_class(ut)];
        let ocean = self.tel_nv_ocean(ut);
        self.telemetry.with(|t| {
            if sea {
                t.count(p, "ship_built", cls, cost);
                t.count(p, "ship_built_reach", if ocean { "ocean" } else { "coast" }, cost);
                t.count(p, "hammers", "ship", cost);
            } else {
                t.count(p, "hammers", "unit", cost);
            }
        });
    }

    /// A building built: its hammers (`hammers`).
    pub(crate) fn tel_naval_built_building(&self, c: CityId, b: BuildingType) {
        let p = self.cities[c].owner;
        let cost = self.player_get_production_needed_building(p, b) as i64;
        self.telemetry.with(|t| t.count(p, "hammers", "building", cost));
    }

    /// The geography of the game, once (at the first census): `geo`.
    fn tel_naval_geo(&self) {
        let mut water = 0i64;
        let mut total = 0i64;
        for pl in self.map.plot_ids() {
            total += 1;
            water += self.plot_is_water(pl) as i64;
        }
        let land_areas = self.map.areas.iter().filter(|(_, a)| !a.water).count();
        let mut starts: Vec<(PlayerId, AreaId)> = Vec::new();
        for p in self.player_ids() {
            if !self.players[p].alive || !self.tel_nv_is_civ(p) {
                continue;
            }
            if let Some(a) = self.player_get_starting_plot(p).and_then(|q| self.map.plots[q].area) {
                starts.push((p, a));
            }
        }
        let mut start_areas: Vec<AreaId> = starts.iter().map(|&(_, a)| a).collect();
        start_areas.sort();
        start_areas.dedup();
        let civs: Vec<String> = starts
            .iter()
            .map(|&(p, a)| {
                let others = starts.iter().filter(|&&(q, b)| q != p && b == a).count();
                let tiles = if self.map.areas.contains(a) { self.map.areas[a].num_tiles } else { 0 };
                format!("[{},{},{},{}]", p.0, a.0, tiles, others)
            })
            .collect();
        let turn = self.game_turn();
        let p0 = starts.first().map_or(PlayerId(0), |&(p, _)| p);
        self.telemetry.with(|t| {
            t.event(|| {
                ev(turn, "geo", p0)
                    .i("plots", total)
                    .i("water", water)
                    .i("land_areas", land_areas as i64)
                    .i("start_areas", start_areas.len() as i64)
                    .raw("civs", &format!("[{}]", civs.join(",")))
                    .end()
            });
        });
    }

    /// The end of `p`'s turn: the sea census (`sea`, `sea_cargo`, `sea_role`, `sea_role_moved`,
    /// `sea_reach`, `sea_raid`), the trips' departures, `naval_first`, and every
    /// [`COAST_SAMPLE_EVERY`] turns the `coast` sample.
    fn tel_naval_census(&self, p: PlayerId) {
        let turn = self.game_turn();
        if !self.telemetry.with(|t| std::mem::replace(&mut t.autopsy.geo_done, true)).unwrap_or(true) {
            self.tel_naval_geo();
        }
        let team = self.players[p].team;
        let war = (self.team_get_at_war_count(team, true) > 0) as usize;
        // [class][place][all, still]
        let mut n = [[[0i64; 2]; 3]; 3];
        let mut cargo = [0i64; 4];
        let mut roles: Vec<(&'static str, bool, bool)> = Vec::new();
        let mut reach = [0i64; 2];
        let mut raid = [0i64; 3];
        let mut out_of_port: Vec<UnitId> = Vec::new();
        let mut first = [false; 4];
        for &u in &self.players[p].units {
            if !self.units.contains(u) || self.unit_get_domain_type(u) != DomainType::DOMAIN_SEA {
                continue;
            }
            let Some(plot) = self.unit_plot_opt(u) else { continue };
            let ut = self.units[u].unit_type;
            let cls = self.tel_nv_class(ut);
            let place = if self.tel_nv_port(plot, team) {
                0
            } else if self.plot_get_team(plot) == Some(team) {
                1
            } else {
                2
            };
            let still = self.units[u].moves == 0;
            n[cls][place][0] += 1;
            n[cls][place][1] += still as i64;
            let ocean = self.tel_nv_ocean(ut);
            reach[ocean as usize] += 1;
            first[0] = true;
            first[1] |= ocean;
            first[2] |= cls == 1;
            first[3] |= cls == 0 && ocean;
            roles.push((enum_name(UnitAIType::NAMES, self.units[u].ai_type.map_or(0, |a| a.idx())), place == 0, !still));
            if cls == 0 {
                let cu = self.unit_get_cargo_units(u);
                let k = if cu.is_empty() {
                    3
                } else if cu.iter().any(|&c| self.tel_nv_kind(c) == 0) {
                    0
                } else if cu.iter().any(|&c| self.tel_nv_kind(c) == 1) {
                    1
                } else {
                    2
                };
                cargo[k] += 1;
                if place != 0 && !cu.is_empty() {
                    out_of_port.push(u);
                }
            }
            if war == 1 && self.d.units[ut].combat > 0 {
                let (x, y) = self.map.xy(plot);
                let mut near = false;
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if let Some(c) = self.map.plot_xy(x + dx, y + dy).and_then(|q| self.map.plots[q].plot_city).filter(|&c| self.cities.contains(c)) {
                            near |= self.at_war(team, self.players[self.cities[c].owner].team);
                        }
                    }
                }
                let on_res = self.plot_is_water(plot) && self.map.plots[plot].bonus_type.is_some() && self.plot_get_team(plot).is_some_and(|ot| self.at_war(team, ot));
                raid[0] += near as i64;
                raid[1] += on_res as i64;
                raid[2] += self.units[u].blockading as i64;
            }
        }
        let coast = if self.tel_nv_is_civ(p) && turn % COAST_SAMPLE_EVERY == 0 {
            let start_area = self.player_get_starting_plot(p).and_then(|q| self.map.plots[q].area);
            let (mut all, mut coastal, mut off_start) = (0i64, 0i64, 0i64);
            let mut areas: Vec<AreaId> = Vec::new();
            for &c in &self.players[p].cities {
                all += 1;
                coastal += self.city_is_coastal(c, self.d.defines.min_water_size_for_ocean) as i64;
                let a = self.city_area(c);
                off_start += (Some(a) != start_area) as i64;
                areas.push(a);
            }
            areas.sort();
            areas.dedup();
            Some((all, coastal, off_start, areas.len() as i64))
        } else {
            None
        };
        self.telemetry.with(|t| {
            for cls in 0..3 {
                for place in 0..3 {
                    for still in 0..2 {
                        if n[cls][place][still] > 0 {
                            t.count(p, "sea", SEA_KEYS[war][cls][place][still], n[cls][place][still]);
                        }
                    }
                }
            }
            for k in 0..4 {
                if cargo[k] > 0 {
                    t.count(p, "sea_cargo", CARGO_KEYS[war][k], cargo[k]);
                }
            }
            for &(role, in_port, moved) in &roles {
                t.count(p, "sea_role", role, in_port as i64);
                if moved {
                    t.count(p, "sea_role_moved", role, 0);
                }
            }
            for (i, key) in ["coast", "ocean"].iter().enumerate() {
                if reach[i] > 0 {
                    t.count(p, "sea_reach", *key, reach[i]);
                }
            }
            if war == 1 {
                for (i, key) in ["near_enemy_city", "on_enemy_resource", "blockading"].iter().enumerate() {
                    t.count(p, "sea_raid", *key, raid[i]);
                }
            }
            for u in out_of_port {
                if let Some(tr) = t.autopsy.trips.get_mut(&u) {
                    tr.left_port.get_or_insert(turn);
                }
            }
            for (k, &has) in first.iter().enumerate() {
                if has && t.autopsy.firsts.insert((p, k)) {
                    t.event(|| ev(turn, "naval_first", p).s("kind", FIRST_KIND[k]).end());
                }
            }
            if let Some((all, coastal, off_start, areas)) = coast {
                t.event(|| ev(turn, "coast", p).i("cities", all).i("coastal", coastal).i("off_start", off_start).i("landmasses", areas).end());
            }
        });
    }
}
