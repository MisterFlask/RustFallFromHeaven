//! Engagement combat scenarios (GAMEOPTION_ENGAGEMENTS, phase 6 step 6; docs/ENGAGEMENTS.md):
//! whole engagements through the rules' entries (`group_attack`, `unit_attack`) on small staged
//! worlds: slots, the shield and a city's capture; siege units' caps on either side; defensive
//! strikes and collateral reaching the reserve; fear and the rout; withdrawals on both sides; the
//! draw; hidden nationality in the report; an immortal reborn; a conversion once after two kills;
//! Blitz; a missile strike; the capture of non-combatants; ships keeping the duel; saves.
//!
//! The outcomes are random: a test either asserts what holds in every battle, or runs a handful
//! of seeds and asserts what must happen in at least one (each says which).

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::{BattleReport, LogEvent, SatOutWhy};
use ffh_core::engagement::{EngageResult, Fate, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war (both AI), `from` and `to` two unowned flat grassland plots
/// side by side with no feature, improvement or river crossing.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world_with(engagements: bool, war: bool) -> World {
    world_sized(engagements, war, "WORLDSIZE_DUEL")
}

fn world_sized(engagements: bool, war: bool, size: &str) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, size, 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = engagements;
    let mut g = Game::new(d, setup);
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p)
            && !g.plot_is_peak(p)
            && !g.plot_is_hills(p)
            && plot.units.is_empty()
            && plot.owner.is_none()
            && plot.feature_type.is_none()
            && plot.improvement_type.is_none()
            && g.plot_defense_modifier(p, None, false) == 0
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((p, q))
        })
        .expect("two flat plots side by side");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    g.map.plots[from].terrain_type = Some(grass);
    g.map.plots[to].terrain_type = Some(grass);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    if war {
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_declare_war(t0, t1, false, None);
    }
    World { g, p0, p1, from, to }
}

fn world() -> World {
    world_with(true, true)
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    fn units(&mut self, owner: PlayerId, name: &str, at: PlotIdx, n: usize) -> Vec<UnitId> {
        (0..n).map(|_| self.unit(owner, name, at)).collect()
    }

    fn promote(&mut self, u: UnitId, name: &str) {
        let p = self.g.d.t::<PromotionType>(name);
        self.g.unit_set_has_promotion(u, p, true);
    }

    fn city(&mut self, owner: PlayerId, at: PlotIdx, pct: i32) -> CityId {
        let (x, y) = self.g.map.xy(at);
        let c = self.g.player_init_city(owner, x, y, true, true);
        let total = self.g.city_total_defense(c, false);
        self.g.cities[c].building_defense += pct - total;
        c
    }

    /// The same world (a save and load), its sync RNG seeded with `seed`.
    fn fork(&self, seed: u32) -> World {
        let mut g = Game::load(self.g.d, &self.g.save()).expect("a save of this build");
        g.soren_rand = Rng::new(seed);
        World { g, ..*self }
    }

    /// `u`'s group attacks `to` (an AI move order's direct attack); the reports it added.
    fn attack(&mut self, u: UnitId) -> (bool, Vec<BattleReport>) {
        let before = self.g.battle_report_next;
        let grp = self.g.units[u].group.expect("a group");
        let (x, y) = self.g.map.xy(self.to);
        let (attacked, _) = self.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
        let reports = self.g.battle_reports.iter().filter(|r| r.id >= before).cloned().collect();
        (attacked, reports)
    }

    fn alive(&self, u: UnitId) -> bool {
        self.g.units.contains(u) && !self.g.unit_is_dead(u)
    }
}

fn events(r: &BattleReport) -> &[LogEvent] {
    &r.log.as_ref().expect("a logged battle").events
}

/// 8 Axemen against an Axeman and 3 Archers in a walled city: the attacker commits up to its
/// slots (the walls cost it two), the archers (back position) are reached only once the axeman
/// in front has fallen, and a won battle takes the city with the launcher's group. Every battle:
/// the slots and the shield; some seed of eight: the city taken.
#[test]
fn axemen_storm_a_walled_city_of_archers() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let c = w.city(p1, to, 50);
    w.g.city_set_population(c, 6);
    w.g.players[p1].human = true; // the battle log is written for a human's battle
    w.unit(p1, "UNIT_AXEMAN", to);
    w.units(p1, "UNIT_ARCHER", to, 3);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 8);
    let mut taken = 0;
    for seed in 0..8 {
        let mut f = w.fork(seed);
        let (attacked, reports) = f.attack(axemen[0]);
        assert!(attacked);
        assert_eq!(reports.len(), 1, "one order, one engagement");
        let r = &reports[0];
        let log = r.log.as_ref().unwrap();
        assert_eq!(log.slots[0].total, 8, "10 a side on open land, two lost to the walls: {:?}", log.slots[0]);
        assert_eq!((r.sides[0].committed, r.sides[1].committed), (8, 4));
        assert!(r.place.city.is_some());
        // The front (the axeman) shields the archers from the attacking axemen.
        let front = log.fighters.iter().position(|lf| lf.side == Side::Defender && lf.slot.is_some() && lf.pos == ffh_core::engagement::report::Position::Front).unwrap();
        let mut front_alive = true;
        for ev in events(r) {
            match ev {
                LogEvent::Hit { h, t, .. } if (*h as usize) < 8 => {
                    let target = &log.fighters[*t as usize];
                    assert!(!front_alive || target.pos == ffh_core::engagement::report::Position::Front, "an archer hit behind a living front, seed {seed}");
                }
                LogEvent::RoundEnd { deaths } => {
                    if deaths.iter().any(|dth| dth.b as usize == front) {
                        front_alive = false;
                    }
                }
                _ => {}
            }
        }
        if r.outcome.result == EngageResult::AttackerWon {
            let c = f.g.map.plots[to].plot_city.expect("the city stands");
            assert_eq!(f.g.cities[c].owner, p0, "a won battle takes the city");
            assert!(r.outcome.plot_taken && r.outcome.city_captured == Some(c));
            assert!(f.g.unit_plot_opt(axemen[0]) == Some(to) || !f.alive(axemen[0]));
            taken += 1;
        } else {
            assert_eq!(f.g.cities[f.g.map.plots[to].plot_city.unwrap()].owner, p1);
        }
        // Every attacker that fought has made its attack.
        for &a in &axemen {
            if f.alive(a) {
                assert!(f.g.units[a].made_attack && f.g.units[a].attacks_made == 1);
            }
        }
    }
    assert!(taken > 0, "eight axemen never took the city");
}

/// Attacking catapults never take a defender past their combat limit, and leave once every
/// target is at it: a city held by one warrior is not emptied. Defending catapults have no limit
/// and fight to the end. Every battle.
#[test]
fn siege_units_cap_on_attack_and_fight_to_the_end_on_defense() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.city(p1, to, 0);
    let warrior = w.unit(p1, "UNIT_WARRIOR", to);
    let cats = w.units(p0, "UNIT_CATAPULT", from, 3);
    for seed in 0..6 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(cats[0]);
        let r = &reports[0];
        assert!(f.alive(warrior), "seed {seed}");
        assert!(f.g.units[warrior].damage <= 75, "seed {seed}: {}", f.g.units[warrior].damage);
        assert!(!r.outcome.plot_taken);
        assert_ne!(r.outcome.result, EngageResult::AttackerWon);
        assert_eq!(f.g.cities[f.g.map.plots[to].plot_city.unwrap()].owner, p1);
    }
    // Defending catapults: no draw, a decided battle.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.city(p1, to, 0);
    w.units(p1, "UNIT_CATAPULT", to, 2);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 4);
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(axemen[0]);
        let r = &reports[0];
        assert_ne!(r.outcome.result, EngageResult::Draw, "seed {seed}");
        let lost: i32 = r.sides.iter().map(|s| s.lost.iter().map(|(_, n)| n).sum::<i32>()).sum();
        assert!(lost > 0, "seed {seed}: someone falls");
    }
}

/// Twelve archers on open land (10 defender slots): the two left in reserve still make defensive
/// strikes, and a catapult's collateral reaches them. Some seed of six for each.
#[test]
fn the_reserve_strikes_and_takes_collateral() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    w.units(p1, "UNIT_ARCHER", to, 12);
    let cat = w.unit(p0, "UNIT_CATAPULT", from);
    w.units(p0, "UNIT_AXEMAN", from, 6);
    let (mut strikes, mut collateral) = (0, 0);
    for seed in 0..6 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(cat);
        let r = &reports[0];
        assert_eq!(r.sides[1].committed, 10);
        let log = r.log.as_ref().unwrap();
        for ev in &log.events {
            match ev {
                LogEvent::DefStrike { striker, .. } if log.fighters[*striker as usize].slot.is_none() => strikes += 1,
                LogEvent::Collateral { target, .. } if log.fighters[*target as usize].slot.is_none() => collateral += 1,
                _ => {}
            }
        }
    }
    assert!(strikes > 0, "no reserve archer ever struck");
    assert!(collateral > 0, "collateral never reached the reserve");
}

/// A Fear defender makes some attackers sit out (their attack spent, their slot empty); a Fear
/// attacker that wins outside a city routs the units left on the plot. Some seed for each.
#[test]
fn fear_sits_attackers_out_and_routs_the_rest() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    let scary = w.unit(p1, "UNIT_AXEMAN", to);
    w.g.units[scary].fear = 1;
    let warriors = w.units(p0, "UNIT_WARRIOR", from, 4);
    let mut sat = 0;
    for seed in 0..6 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(warriors[0]);
        let r = &reports[0];
        let afraid = events(r).iter().filter(|e| matches!(e, LogEvent::SatOut { why: SatOutWhy::Afraid, .. })).count() as i32;
        assert_eq!(afraid, r.sides[0].sat_out);
        sat += afraid;
        for &u in &warriors {
            if f.alive(u) {
                assert_eq!(f.g.units[u].attacks_made, 1, "a unit that sat out spent its attack too");
            }
        }
    }
    assert!(sat > 0, "nobody was ever afraid");

    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[w.g.map.plots[to].units[0]].damage = 90;
    w.unit(p1, "UNIT_WORKER", to);
    let champion = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[champion].fear = 1;
    w.units(p0, "UNIT_AXEMAN", from, 2);
    let mut routed = 0;
    for seed in 0..8 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(champion);
        let r = &reports[0];
        if r.outcome.result == EngageResult::AttackerWon {
            routed += events(r).iter().filter(|e| matches!(e, LogEvent::Rout { .. })).count();
        }
    }
    assert!(routed > 0, "the worker was never routed");
}

/// A defender with a sure defensive withdrawal leaves the battle alive and flees outside a city;
/// in a city it has no withdrawal and stays. Every battle.
#[test]
fn defensive_withdrawal_only_outside_cities() {
    for in_city in [false, true] {
        let mut w = world();
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        if in_city {
            w.city(p1, to, 0);
        }
        let runner = w.unit(p1, "UNIT_WARRIOR", to);
        w.g.unit_change_extra_withdrawal(runner, 100);
        let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
        for seed in 0..4 {
            let mut f = w.fork(seed);
            let (_, reports) = f.attack(axemen[0]);
            let r = &reports[0];
            if in_city {
                assert_eq!(r.sides[1].withdrew, 0);
            } else {
                assert_eq!((r.sides[1].withdrew, r.sides[1].fled), (1, 1), "seed {seed}");
                assert!(f.alive(runner) && f.g.unit_plot(runner) != to);
            }
        }
    }
}

/// An attacker with a sure withdrawal against a far stronger defender leaves alive, stays where it
/// was, in its army, and has made its attack. Every battle.
#[test]
fn an_attacker_withdraws_and_stays_in_its_army() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let wall = w.unit(p1, "UNIT_AXEMAN", to);
    w.g.units[wall].extra_combat_percent_defense = 900;
    w.g.units[wall].extra_combat_percent = 900;
    let rider = w.unit(p0, "UNIT_HORSEMAN", from);
    w.g.unit_change_extra_withdrawal(rider, 100);
    let army = w.g.units[rider].army;
    assert!(army.is_some());
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(rider);
        let r = &reports[0];
        assert_eq!(r.outcome.result, EngageResult::DefenderHeld, "seed {seed}");
        assert_eq!(r.sides[0].withdrew, 1);
        assert!(f.alive(rider) && f.g.unit_plot(rider) == from);
        assert_eq!(f.g.units[rider].army, army);
        assert!(f.g.units[rider].made_attack);
    }
}

/// Two sides that can hardly hurt each other fight the 40 rounds to a draw: both disengage and
/// the plot does not change hands. Every battle.
#[test]
fn a_draw_keeps_the_plot() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.unit(p1, "UNIT_WARRIOR", to);
    let a = w.unit(p0, "UNIT_WARRIOR", from);
    for u in [a, d] {
        w.g.units[u].extra_combat_percent_defense = 100_000;
    }
    let mut f = w.fork(1);
    let (_, reports) = f.attack(a);
    let r = &reports[0];
    assert_eq!((r.outcome.result, r.outcome.rounds), (EngageResult::Draw, 40));
    assert!(!r.outcome.plot_taken && f.alive(a) && f.alive(d));
    assert_eq!(f.g.unit_plot(d), to);
}

/// A hidden-nationality force at peace attacks: the defender's report shows the barbarians as
/// the attacker, the attacker's its real owner.
#[test]
fn hidden_nationality_shows_the_barbarian() {
    let mut w = world_with(true, false);
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[d].damage = 80;
    let a = w.unit(p0, "UNIT_AXEMAN", from);
    w.promote(a, "PROMOTION_HIDDEN_NATIONALITY");
    let barbarian = w.g.barbarian_player();
    let mut f = w.fork(1);
    let (attacked, reports) = f.attack(a);
    assert!(attacked);
    let r = &reports[0];
    let side = &r.sides[0];
    assert_eq!((side.player, side.shown_player), (p0, barbarian));
    assert_eq!(r.owner_for(p1, Side::Attacker, side.player, side.shown_player), barbarian);
    assert_eq!(r.owner_for(p0, Side::Attacker, side.player, side.shown_player), p0);
    assert!(!f.g.teams[f.g.players[p0].team].at_war[f.g.players[p1].team], "no war was declared");
}

/// An immortal defender killed in the battle is reborn in its capital.
#[test]
fn an_immortal_is_reborn() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let capital = w.g.players[p1].cities.first().copied().unwrap_or_else(|| {
        let (tx, ty) = w.g.map.xy(to);
        let at = w
            .g
            .map
            .plot_ids()
            .find(|&p| {
                let (x, y) = w.g.map.xy(p);
                !w.g.plot_is_water(p) && !w.g.plot_is_peak(p) && w.g.map.plots[p].owner.is_none() && w.g.plot_distance(x, y, tx, ty) > 6
            })
            .unwrap();
        w.city(p1, at, 0)
    });
    let hero = w.unit(p1, "UNIT_WARRIOR", to);
    w.promote(hero, "PROMOTION_IMMORTAL");
    w.g.units[hero].damage = 95;
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 4);
    let mut reborn = 0;
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(axemen[0]);
        if reports[0].sides[1].lost.is_empty() {
            continue;
        }
        assert!(f.alive(hero), "seed {seed}: the immortal is gone");
        assert_eq!(f.g.unit_plot(hero), f.g.city_plot(capital));
        reborn += 1;
    }
    assert!(reborn > 0);
}

/// A Ravenous Werewolf (it always converts after a kill) that kills two units in one battle
/// converts once, into one Werewolf that keeps the XP of both kills.
#[test]
fn a_werewolf_converts_once_after_its_last_kill() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    for _ in 0..2 {
        let v = w.unit(p1, "UNIT_WARRIOR", to);
        w.g.units[v].damage = 97;
    }
    let wolf = w.unit(p0, "UNIT_RAVENOUS_WEREWOLF", from);
    let werewolf = w.g.d.t::<UnitType>("UNIT_WEREWOLF");
    let mut both = 0;
    for seed in 0..12 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(wolf);
        let r = &reports[0];
        let kills: i32 = r.sides[1].lost.iter().map(|(_, n)| n).sum();
        let new: Vec<UnitId> = f.g.players[p0].units.iter().copied().filter(|&u| f.g.units[u].unit_type == werewolf && !f.g.unit_is_dead(u)).collect();
        if kills == 0 {
            assert!(new.is_empty());
            continue;
        }
        assert_eq!(new.len(), 1, "seed {seed}: converted once");
        if kills == 2 {
            assert!(f.g.units[new[0]].experience >= 2 * f.g.d.defines.min_experience_per_combat, "the XP of both kills");
            both += 1;
        }
    }
    assert!(both > 0, "never two kills");
}

/// Blitz: a second engagement in the turn, not a third.
#[test]
fn blitz_attacks_twice() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.unit(p1, "UNIT_WARRIOR", to);
    let a = w.unit(p0, "UNIT_HORSEMAN", from);
    w.promote(a, "PROMOTION_BLITZ");
    for u in [a, d] {
        w.g.units[u].extra_combat_percent_defense = 100_000;
    }
    let mut f = w.fork(1);
    assert_eq!(f.attack(a).1.len(), 1);
    f.g.units[a].moves = 0;
    assert_eq!(f.attack(a).1.len(), 1, "Blitz: one more");
    f.g.units[a].moves = 0;
    let (attacked, reports) = f.attack(a);
    assert!(!attacked && reports.is_empty(), "no third attack");
}

/// `unit_attack` (Taunt, events) is a one-unit engagement whatever the unit's moves and attacks
/// this turn: three units, three engagements.
#[test]
fn unit_attack_is_a_forced_engagement() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let d = w.unit(p1, "UNIT_AXEMAN", to);
    w.g.units[d].extra_combat_percent_defense = 100_000;
    let taunted = w.units(p0, "UNIT_WARRIOR", from, 3);
    for &u in &taunted {
        w.g.units[u].made_attack = true;
        w.g.units[u].attacks_made = 1;
        w.g.units[u].extra_combat_percent_defense = 100_000;
    }
    let mut f = w.fork(1);
    for &u in &taunted {
        f.g.unit_attack(u, to);
    }
    assert_eq!(f.g.battle_reports.len(), 3);
    assert!(f.g.battle_reports.iter().all(|r| r.sides[0].committed == 1 && r.sides[1].committed == 1));
}

/// A Fireball strikes one defender and dies; no engagement, no report.
#[test]
fn a_missile_strikes_and_dies() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let ds = w.units(p1, "UNIT_WARRIOR", to, 2);
    let fireball = w.unit(p0, "UNIT_FIREBALL", from);
    let mut f = w.fork(1);
    let (attacked, reports) = f.attack(fireball);
    assert!(attacked && reports.is_empty());
    assert!(!f.g.units.contains(fireball) || f.g.units[fireball].death_delay, "the missile dies");
    assert!(ds.iter().any(|&u| !f.alive(u) || f.g.units[u].damage > 0), "it hit someone");
}

/// No defender, only a worker: today's attack captures it.
#[test]
fn non_combatants_are_captured() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.unit(p1, "UNIT_WORKER", to);
    let a = w.unit(p0, "UNIT_AXEMAN", from);
    let mut f = w.fork(1);
    let (attacked, reports) = f.attack(a);
    assert!(attacked && reports.is_empty());
    assert_eq!(f.g.unit_plot(a), to);
    assert!(f.g.map.plots[to].units.iter().all(|&u| f.g.units[u].owner == p0), "the worker is ours");
}

/// Ships fight ships with the option on exactly as with it off.
#[test]
fn ships_keep_the_duel() {
    let mut outcomes = Vec::new();
    for engagements in [false, true] {
        let mut w = world_with(engagements, true);
        let (p0, p1) = (w.p0, w.p1);
        // Two empty land plots side by side, away from `from` and `to`, made sea.
        let (a, b) = w
            .g
            .map
            .plot_ids()
            .filter(|&p| p != w.from && p != w.to && !w.g.plot_is_water(p) && w.g.map.plots[p].units.is_empty() && w.g.map.plots[p].owner.is_none())
            .find_map(|p| {
                let (x, y) = w.g.map.xy(p);
                let q = w.g.map.plot_xy(x, y + 1)?;
                (q != w.from && q != w.to && !w.g.plot_is_water(q) && w.g.map.plots[q].units.is_empty() && w.g.map.plots[q].owner.is_none()).then_some((p, q))
            })
            .expect("two land plots");
        for p in [a, b] {
            w.g.plot_set_plot_type(p, PlotType::PLOT_OCEAN, true);
        }
        assert!(w.g.plot_is_water(a) && w.g.plot_is_water(b));
        w.to = b;
        let ship = w.unit(p0, "UNIT_TRIREME", a);
        let target = w.unit(p1, "UNIT_TRIREME", b);
        let mut f = w.fork(7);
        let (attacked, reports) = f.attack(ship);
        assert!(attacked && reports.is_empty(), "a duel, no engagement");
        let state = |u: UnitId| (f.alive(u), if f.g.units.contains(u) { f.g.units[u].damage } else { -1 });
        outcomes.push((state(ship), state(target), f.g.soren_rand.clone()));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

/// A real engagement's report, with its log, survives a save and load.
#[test]
fn reports_survive_a_save() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    w.units(p1, "UNIT_ARCHER", to, 2);
    let a = w.units(p0, "UNIT_AXEMAN", from, 3);
    let mut f = w.fork(2);
    let (_, reports) = f.attack(a[0]);
    assert!(reports[0].log.is_some());
    let h = Game::load(f.g.d, &f.g.save()).unwrap();
    assert_eq!(h.battle_reports, f.g.battle_reports);
    assert_eq!(h.battle_report_next, f.g.battle_report_next);
    // The fates of the log's committed fighters match the report.
    let log = reports[0].log.as_ref().unwrap();
    let killed = log.fighters.iter().filter(|lf| lf.side == Side::Defender && matches!(lf.fate, Some(Fate::Killed { .. }))).count() as i32;
    assert_eq!(killed, reports[0].sides[1].lost.iter().map(|(_, n)| n).sum::<i32>());
}

