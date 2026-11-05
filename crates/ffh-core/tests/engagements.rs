//! Engagement combat scenarios (formation battles; docs/ENGAGEMENTS.md):
//! whole engagements through the rules' entries (`group_attack`, `group_attack_force`,
//! `unit_attack`) on small staged worlds: slots, lanes, each kind's reach in the log and a city's
//! capture; the plot that falls only when no defender is left; an ordered formation and the auto
//! formation; large units; siege units' caps on either side; defensive strikes from the reserve
//! and splash; fear; withdrawals on both sides; a marching recruit's; a held line; hidden
//! nationality in the report; the defenders left; an immortal reborn; a conversion once after two
//! kills; battle XP before the per-kill effects; captures and slaves outside the army; Hidden's
//! quirk with no free plot; Blitz; Taunt; the AI's preamble; a missile strike and its kill; the
//! capture of non-combatants; ships keeping the duel; saves.
//!
//! The outcomes are random: a test either asserts what holds in every battle, or runs a handful
//! of seeds and asserts what must happen in at least one (each says which).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ffh_core::army::{March, RecruitDest};
use ffh_core::controller::{AiHook, Controller};
use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::{BattleLog, BattleReport, LogEvent, LogFate, SatOutWhy};
use ffh_core::engagement::{EngageResult, Place, Side};
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

fn world_with(war: bool) -> World {
    world_sized(war, "WORLDSIZE_DUEL")
}

fn world_sized(war: bool, size: &str) -> World {
    world_of(war, size, 2)
}

/// [`world_sized`] with `players` players (0 and 1 at war when `war`).
fn world_of(war: bool, size: &str, players: usize) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, players, size, 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
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
    world_with(true)
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

    /// A land plot with no unit and no owner at least `dist` plots from `near`.
    fn far_plot(&self, near: PlotIdx, dist: i32) -> PlotIdx {
        let (nx, ny) = self.g.map.xy(near);
        self.g
            .map
            .plot_ids()
            .find(|&p| {
                let (x, y) = self.g.map.xy(p);
                !self.g.plot_is_water(p)
                    && !self.g.plot_is_peak(p)
                    && self.g.map.plots[p].owner.is_none()
                    && self.g.map.plots[p].units.is_empty()
                    && self.g.plot_distance(x, y, nx, ny) >= dist
            })
            .expect("a far plot")
    }
}

fn events(r: &BattleReport) -> &[LogEvent] {
    &r.log.as_ref().expect("a logged battle").events
}

/// Replays `log`'s battle on its grids and checks each action's reach against the field as it
/// stood: a strike (and a charge where flanking does not work) hits the front unit of one of the
/// target's lanes, a shot, spell or bombardment the backmost; no action comes from or falls on a
/// unit that is not on the field (fallen, escaped, sat out). Returns the actions checked.
fn check_reach(log: &BattleLog) -> usize {
    use ffh_data::ActionKind as K;
    let n = log.fighters.len();
    let mut row: Vec<Option<u8>> = log.fighters.iter().map(|f| f.place.map(|p| p.1)).collect();
    let mut on: Vec<bool> = log.fighters.iter().map(|f| f.place.is_some()).collect();
    for e in &log.events {
        if let LogEvent::SatOut { unit, .. } = e {
            on[*unit as usize] = false;
        }
    }
    let lanes = |i: usize| {
        let f = &log.fighters[i];
        let l = f.place.map_or(0, |p| p.0);
        l..=l + f.large as u8
    };
    let mut checked = 0;
    for e in &log.events {
        let LogEvent::Action(a) = e else { continue };
        assert!(on[a.actor as usize], "fighter {} acts off the field: {a:?}", a.actor);
        if let (Some(t), Some(first)) = (a.target, a.hits.first()) {
            let t = t as usize;
            assert_eq!(first.target as usize, t);
            assert!(on[t], "fighter {t} hit off the field: {a:?}");
            let side = log.fighters[t].side;
            let shares = |j: usize| j != t && on[j] && log.fighters[j].side == side && lanes(j).any(|l| lanes(t).contains(&l));
            let ahead = |j: usize| shares(j) && row[j] < row[t];
            let behind = |j: usize| shares(j) && row[j] > row[t];
            let field_charge = a.kind == K::Charge && log.ground.flanking();
            if a.kind == K::Strike || (a.kind == K::Charge && !field_charge) {
                assert!(!(0..n).any(ahead), "a strike past the front of its lane: {a:?}");
                checked += 1;
            }
            if matches!(a.kind, K::Shoot | K::Spell | K::Bombard) {
                assert!(!(0..n).any(behind), "a shot short of the back of its lane: {a:?}");
                checked += 1;
            }
        }
        for h in &a.hits {
            if h.killed || h.withdrew {
                on[h.target as usize] = false;
            }
        }
        if let Some(k) = &a.knocked {
            assert_eq!(row[k.unit as usize], Some(k.from));
            row[k.unit as usize] = Some(k.to);
        }
    }
    checked
}

/// 8 Axemen against an Axeman and 3 Archers in a city of 50% defence (no wall building, so no
/// Sentinels: docs/SIEGES.md; tests/sieges.rs has the walls): the attacker commits up to its slots
/// (the city's defence costs none) on its lanes, every strike and shot falls in its reach, and the
/// city falls only when none of its defenders is left: then the launcher's group takes it; else
/// nobody moves. Every battle: the slots, lanes and reach; some seed of eight: the city taken.
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
    let (mut taken, mut checked) = (0, 0);
    for seed in 0..8 {
        let mut f = w.fork(seed);
        let (attacked, reports) = f.attack(axemen[0]);
        assert!(attacked);
        assert_eq!(reports.len(), 1, "one order, one engagement");
        let r = &reports[0];
        let log = r.log.as_ref().unwrap();
        assert_eq!(log.slots[0].total, 10, "10 a side on open land, the city's defence costs none: {:?}", log.slots[0]);
        assert_eq!((r.sides[0].committed, r.sides[1].committed), (8, 4));
        assert_eq!((r.sides[0].slots, r.sides[1].slots), (10, 10), "the report keeps the slots");
        assert_eq!((r.sides[0].lanes, r.sides[1].lanes), (4, 4));
        assert_eq!(log.lanes, [4, 4]);
        assert!(log.ground.city && !log.ground.flanking());
        assert!(r.place.city.is_some());
        assert!(!log.ordered[0], "the rules' auto formation");
        checked += check_reach(log);
        let c_now = f.g.map.plots[to].plot_city.expect("the city stands");
        if r.outcome.result == EngageResult::Taken {
            assert_eq!(f.g.cities[c_now].owner, p0, "a taken city is captured");
            assert!(r.outcome.plot_taken && r.outcome.city_captured == Some(c_now));
            assert_eq!((r.sides[1].standing, r.outcome.defenders_left), (0, 0));
            assert!(f.g.unit_plot_opt(axemen[0]) == Some(to) || !f.alive(axemen[0]));
            taken += 1;
        } else {
            assert_eq!(f.g.cities[c_now].owner, p1);
            assert!(r.outcome.defenders_left > 0 && !r.outcome.plot_taken);
            assert!(axemen.iter().all(|&a| !f.alive(a) || f.g.unit_plot(a) == from), "seed {seed}: nobody moves");
        }
        // Every attacker that fought has made its attack.
        for &a in &axemen {
            if f.alive(a) {
                assert!(f.g.units[a].made_attack && f.g.units[a].attacks_made == 1);
            }
        }
    }
    assert!(taken > 0, "eight axemen never took the city");
    assert!(checked > 50, "{checked} actions checked");
}

/// The plot falls only when no defender is left on it (spec 1.7): in a forest (6 slots) eight
/// weak warriors put six in the line and keep two in reserve; the attackers wipe the line out,
/// and the plot holds (nobody moves, the survivors keep their damage). With six only, the same
/// battle takes the plot and the launcher's group moves in. Every battle.
#[test]
fn the_plot_falls_only_when_no_defender_is_left() {
    for (n, falls) in [(8, false), (6, true)] {
        let mut w = world();
        let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
        w.g.map.plots[to].feature_type = Some(w.g.d.t::<FeatureType>("FEATURE_FOREST"));
        w.g.players[p1].human = true;
        for d in w.units(p1, "UNIT_WARRIOR", to, n) {
            w.g.units[d].damage = 95;
        }
        let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
        for &a in &axemen {
            w.g.units[a].extra_combat_percent = 400;
        }
        for seed in 0..4 {
            let mut f = w.fork(seed);
            let (_, reports) = f.attack(axemen[0]);
            let r = &reports[0];
            assert_eq!((r.sides[0].slots, r.sides[1].slots, r.sides[1].committed), (6, 6, 6), "seed {seed}");
            assert_eq!(r.sides[1].fallen + r.sides[1].escaped, 6, "seed {seed}: the line is wiped out: {:?}", r.sides[1]);
            assert!(check_reach(r.log.as_ref().unwrap()) > 0);
            if falls {
                assert_eq!(r.outcome.result, EngageResult::Taken, "seed {seed}");
                assert!(r.outcome.plot_taken && r.outcome.defenders_left == 0);
                assert_eq!(f.g.unit_plot(axemen[0]), to, "seed {seed}: the launcher moved in");
            } else {
                assert_eq!(r.outcome.result, EngageResult::Cleared, "seed {seed}");
                assert!(!r.outcome.plot_taken && r.outcome.defenders_left == 2, "seed {seed}: {:?}", r.outcome);
                assert!(axemen.iter().all(|&a| f.g.unit_plot(a) == from), "seed {seed}: nobody moves");
                assert!(f.g.map.plots[to].units.iter().all(|&u| f.g.units[u].owner == p1));
            }
        }
    }
}

/// An ordered formation is the one fought: every unit stands where the order put it, the log says
/// the formation was ordered, a large unit fills two cells and counts two slots; an order that
/// breaks the grid is refused and changes nothing. Without one, the rules' auto formation. Every
/// battle.
#[test]
fn the_ordered_formation_is_the_one_fought() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    w.units(p1, "UNIT_WARRIOR", to, 2);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 3);
    let archers = w.units(p0, "UNIT_ARCHER", from, 2);
    let giant = w.unit(p0, "UNIT_HILL_GIANT", from);
    let all: Vec<UnitId> = axemen.iter().chain(&archers).copied().chain([giant]).collect();
    let army = w.g.army_form(p0, &all).expect("an army");
    let grp = w.g.armies[army].group.expect("the army's group");
    let (x, y) = w.g.map.xy(to);
    // The archers in front, the axemen behind, the giant in the rear across lanes 2 and 3.
    let order = vec![
        Place { unit: archers[0], lane: 0, row: 0 },
        Place { unit: archers[1], lane: 3, row: 0 },
        Place { unit: axemen[0], lane: 0, row: 1 },
        Place { unit: axemen[1], lane: 1, row: 1 },
        Place { unit: axemen[2], lane: 2, row: 1 },
        Place { unit: giant, lane: 2, row: 2 },
    ];
    let mut f = w.fork(3);
    f.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(order.clone())).expect("the attack");
    let r = f.g.battle_reports.last().expect("a report").clone();
    let log = r.log.as_ref().unwrap();
    assert!(log.ordered[0] && !log.ordered[1]);
    for p in &order {
        let lf = log.fighters.iter().find(|lf| lf.unit == p.unit).expect("in the log");
        assert_eq!(lf.place, Some((p.lane, p.row)), "{:?}", p.unit);
    }
    let g = log.fighters.iter().find(|lf| lf.unit == giant).unwrap();
    assert!(g.large && g.max_hp == 180, "a large unit: two cells and 150% of its role's HP ({})", g.max_hp);
    assert_eq!((r.sides[0].committed, r.sides[0].used), (6, 7));
    check_reach(log);

    // A formation off the grid, or with two units in a cell: refused, nothing changed.
    let mut f = w.fork(3);
    let before = f.g.save();
    let bad = vec![Place { unit: giant, lane: 3, row: 2 }];
    assert!(f.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(bad)).is_err());
    let bad = vec![Place { unit: axemen[0], lane: 1, row: 0 }, Place { unit: axemen[1], lane: 1, row: 0 }];
    assert!(f.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(bad)).is_err());
    assert!(f.g.save() == before, "a refused order changes nothing");

    // No order: the auto formation (the giant first, in the centre of the front row).
    let mut f = w.fork(3);
    f.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, None).expect("the attack");
    let r = f.g.battle_reports.last().unwrap().clone();
    let log = r.log.as_ref().unwrap();
    assert!(!log.ordered[0]);
    let ctx = ffh_core::engagement::HitCtx::new(from, to);
    let auto = w.g.engage_auto_formation(&all, Side::Attacker, &w.g.engage_slots(&ctx), &ctx);
    assert_eq!(auto.len(), all.len());
    for p in &auto {
        let lf = log.fighters.iter().find(|lf| lf.unit == p.unit).expect("in the log");
        assert_eq!(lf.place, Some((p.lane, p.row)));
    }
    assert_eq!(log.fighters.iter().find(|lf| lf.unit == giant).and_then(|lf| lf.place), Some((1, 0)));
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
        assert_ne!(r.outcome.result, EngageResult::Taken);
        assert_eq!(f.g.cities[f.g.map.plots[to].plot_city.unwrap()].owner, p1);
    }
    // Defending catapults have no limit: they fall to the axemen. Some seed of four.
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.city(p1, to, 0);
    w.units(p1, "UNIT_CATAPULT", to, 2);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 4);
    let mut lost = 0;
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(axemen[0]);
        lost += reports[0].sides.iter().map(|s| s.fallen).sum::<i32>();
    }
    assert!(lost > 0, "nobody ever fell");
}

/// Twelve archers on open land (10 defender slots): the two left in reserve still make defensive
/// strikes, and the catapult (collateral damage) splashes its hits on the target's neighbours.
/// Some seed of six for each.
#[test]
fn the_reserve_strikes_and_the_catapult_splashes() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    w.units(p1, "UNIT_ARCHER", to, 12);
    let cat = w.unit(p0, "UNIT_CATAPULT", from);
    // Tough enough to outlive the archers' first volleys (it acts last: speed 1).
    w.g.units[cat].extra_combat_percent_defense = 2000;
    w.units(p0, "UNIT_AXEMAN", from, 6);
    let (mut strikes, mut collateral) = (0, 0);
    for seed in 0..6 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(cat);
        let r = &reports[0];
        assert_eq!(r.sides[1].committed, 10);
        assert_eq!(r.outcome.defenders_left, 2 + r.sides[1].standing + r.sides[1].sat_out, "seed {seed}: the two in reserve and the line's survivors");
        let log = r.log.as_ref().unwrap();
        let cat_b = log.fighters.iter().position(|lf| lf.unit == cat).expect("the catapult fights") as u16;
        for ev in &log.events {
            match ev {
                LogEvent::DefStrike { striker, .. } if log.fighters[*striker as usize].place.is_none() => strikes += 1,
                LogEvent::Action(a) if a.actor == cat_b => collateral += a.hits.iter().filter(|h| h.splash && h.dmg > 0).count(),
                _ => {}
            }
        }
        check_reach(log);
    }
    assert!(strikes > 0, "no reserve archer ever struck");
    assert!(collateral > 0, "the catapult never splashed");
}

/// A Fear defender makes some attackers sit out (their attack spent, their cell empty: they never
/// act). Some seed.
#[test]
fn fear_sits_attackers_out() {
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
        let log = r.log.as_ref().unwrap();
        assert_eq!(log.fighters.iter().filter(|lf| lf.fate == Some(LogFate::SatOut)).count() as i32, afraid);
        check_reach(log);
        sat += afraid;
        for &u in &warriors {
            if f.alive(u) {
                assert_eq!(f.g.units[u].attacks_made, 1, "a unit that sat out spent its attack too");
            }
        }
    }
    assert!(sat > 0, "nobody was ever afraid");
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
                assert_eq!(r.sides[1].escaped, 0);
            } else {
                assert_eq!(r.sides[1].escaped, 1, "seed {seed}");
                assert!(f.alive(runner) && f.g.unit_plot(runner) != to);
            }
        }
    }
}

/// A wounded attacker with a sure withdrawal against a far stronger defender leaves alive when a
/// hit would kill it, stays where it was, in its army, and has made its attack. Every battle.
#[test]
fn an_attacker_withdraws_and_stays_in_its_army() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let wall = w.unit(p1, "UNIT_AXEMAN", to);
    w.g.units[wall].extra_combat_percent_defense = 900;
    w.g.units[wall].extra_combat_percent = 900;
    let rider = w.unit(p0, "UNIT_HORSEMAN", from);
    w.g.unit_change_extra_withdrawal(rider, 100);
    w.g.units[rider].damage = 80;
    let army = w.g.units[rider].army;
    assert!(army.is_some());
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(rider);
        let r = &reports[0];
        assert_eq!(r.outcome.result, EngageResult::Repulsed, "seed {seed}");
        assert_eq!(r.sides[0].escaped, 1);
        assert!(f.alive(rider) && f.g.unit_plot(rider) == from);
        assert_eq!(f.g.units[rider].army, army);
        assert!(f.g.units[rider].made_attack);
    }
}

/// Two sides that can hardly hurt each other fight the three rounds and both lines hold: the plot
/// does not change hands and nobody moves. Every battle.
#[test]
fn a_held_line_keeps_the_plot() {
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
    assert_eq!((r.outcome.result, r.outcome.rounds), (EngageResult::Held, 3));
    assert!(!r.outcome.plot_taken && f.alive(a) && f.alive(d));
    assert_eq!(f.g.unit_plot(d), to);
}

/// A hidden-nationality force at peace attacks: the defender's report shows the barbarians as
/// the attacker, the attacker's its real owner.
#[test]
fn hidden_nationality_shows_the_barbarian() {
    let mut w = world_with(false);
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

/// Ships fight ships in the duel: no engagement, no report.
#[test]
fn ships_keep_the_duel() {
    let mut w = world_with(true);
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
    assert!(!f.alive(ship) || !f.alive(target) || f.g.units[ship].damage > 0 || f.g.units[target].damage > 0, "the duel was fought");
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
    let killed = log.fighters.iter().filter(|lf| lf.side == Side::Defender && matches!(lf.fate, Some(LogFate::Fell { .. }))).count() as i32;
    assert_eq!(killed, reports[0].sides[1].fallen);
    assert_eq!(killed, reports[0].sides[1].lost.iter().map(|(_, n)| n).sum::<i32>());
}

/// A marching recruit on the defended plot defends like any unit there; one that withdraws (outside
/// a city) flees and still marches, in no army. Every battle.
#[test]
fn a_marching_recruit_defends_and_keeps_its_march() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let home = w.far_plot(to, 6);
    let c = w.city(p1, home, 0);
    let recruit = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.unit_change_extra_withdrawal(recruit, 100);
    let dest = RecruitDest::Garrison(c);
    w.g.units[recruit].march = Some(March { dest, stalled: 0, left_turn: 0, from_city: c, diverted: false });
    w.g.player_army_sync(p1);
    let axemen = w.units(p0, "UNIT_AXEMAN", from, 6);
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(axemen[0]);
        let r = &reports[0];
        assert_eq!(r.sides[1].committed, 1, "seed {seed}: the recruit defends");
        assert_eq!(r.sides[1].escaped, 1, "seed {seed}");
        assert!(f.alive(recruit) && f.g.unit_plot(recruit) != to, "seed {seed}: it fled");
        assert_eq!(f.g.units[recruit].march.as_ref().map(|m| m.dest), Some(dest), "seed {seed}: it still marches");
        assert_eq!(f.g.units[recruit].army, None, "seed {seed}");
    }
}

/// The report counts the defender's units still on the plot (its reserve and its line's
/// survivors), also for a hidden-nationality attack at peace. Every battle.
#[test]
fn the_report_counts_the_reserve_left() {
    let mut w = world_with(false);
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    for d in w.units(p1, "UNIT_WARRIOR", to, 11) {
        w.g.units[d].damage = 60;
    }
    let a = w.units(p0, "UNIT_AXEMAN", from, 4);
    for &u in &a {
        w.promote(u, "PROMOTION_HIDDEN_NATIONALITY");
    }
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (attacked, reports) = f.attack(a[0]);
        assert!(attacked);
        let r = &reports[0];
        assert_eq!((r.sides[1].slots, r.sides[1].committed), (10, 10));
        assert_eq!(r.outcome.defenders_left, 1 + r.sides[1].standing + r.sides[1].sat_out, "seed {seed}: {:?}", r.outcome);
    }
}

/// Battle XP is granted before the per-kill effects (as the duel grants it before `combatWon`):
/// the log's XP of the killer is the battle's alone, its kill's heal and free XP follow, and the
/// unit ends with both XP. Every battle with a kill.
#[test]
fn battle_xp_comes_before_the_per_kill_effects() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    w.g.players[p1].human = true;
    let v = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[v].damage = 90;
    let killer = w.unit(p0, "UNIT_AXEMAN", from);
    w.g.units[killer].damage = 50;
    w.g.units[killer].combat_heal_percent = 30;
    w.g.players[p0].free_xp_from_combat = 7;
    let mut kills = 0;
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let before = f.g.units[killer].experience;
        let (_, reports) = f.attack(killer);
        let r = &reports[0];
        if r.sides[1].lost.is_empty() {
            continue;
        }
        kills += 1;
        let evs = events(r);
        let xp_at = evs.iter().position(|e| matches!(e, LogEvent::Xp { unit: 0, .. })).expect("the killer's battle XP");
        let heal_at = evs.iter().position(|e| matches!(e, LogEvent::Heal { unit: 0, .. })).expect("the heal on a kill");
        assert!(xp_at < heal_at, "seed {seed}: battle XP first");
        let LogEvent::Xp { amount, .. } = evs[xp_at] else { unreachable!() };
        assert!(amount >= f.g.d.defines.min_experience_per_combat, "seed {seed}: {amount}");
        assert_eq!(f.g.units[killer].experience - before, amount + 7, "seed {seed}: the battle's XP, then the kill's free XP");
    }
    assert!(kills > 0, "never a kill");
}

/// Captured and enslaved units appear on the capturer's plot outside the engagement and its army:
/// in a group of their own and in no army of the capturer's. Every battle with the kill.
#[test]
fn captures_and_slaves_are_outside_the_army() {
    // A captured animal (Subdue Animal captures every animal it kills).
    let mut w = world();
    let (p0, from, to) = (w.p0, w.from, w.to);
    let barbarian = w.g.barbarian_player();
    let wolf = w.unit(barbarian, "UNIT_WOLF", to);
    w.g.units[wolf].damage = 95;
    let tamer = w.unit(p0, "UNIT_AXEMAN", from);
    w.promote(tamer, "PROMOTION_SUBDUE_ANIMAL");
    let army = w.g.units[tamer].army;
    assert!(army.is_some());
    let wolf_type = w.g.d.t::<UnitType>("UNIT_WOLF");
    let mut captured = 0;
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(tamer);
        let r = &reports[0];
        if r.sides[1].lost.is_empty() {
            continue;
        }
        assert!(f.alive(tamer), "seed {seed}");
        let ours: Vec<UnitId> = f.g.players[p0].units.iter().copied().filter(|&u| f.g.units[u].unit_type == wolf_type).collect();
        assert_eq!(ours.len(), 1, "seed {seed}: the wolf is captured");
        let cu = ours[0];
        assert_eq!(f.g.unit_plot(cu), from, "seed {seed}: on the capturer's plot (the tamer moved in)");
        assert_eq!(f.g.unit_plot(tamer), to, "seed {seed}");
        assert_ne!(f.g.units[cu].group, f.g.units[tamer].group, "seed {seed}");
        assert_eq!(f.g.units[tamer].army, army, "seed {seed}");
        assert!(f.g.units[cu].army.is_none() || f.g.units[cu].army != army, "seed {seed}: not in the tamer's army");
        assert_eq!(r.sides[0].committed, 1, "seed {seed}: not in the engagement");
        captured += 1;
    }
    assert!(captured > 0, "the tamer never took the wolf");

    // A slave (the player's enslavement chance).
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let v = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[v].damage = 95;
    let master = w.unit(p0, "UNIT_AXEMAN", from);
    w.units(p0, "UNIT_AXEMAN", from, 2);
    w.g.players[p0].enslavement_chance = 100;
    let slave = opt_from_i32::<UnitType>(w.g.d.defines.slave_unit).expect("a slave unit");
    let mut enslaved = 0;
    for seed in 0..4 {
        let mut f = w.fork(seed);
        let (_, reports) = f.attack(master);
        if reports[0].sides[1].lost.is_empty() {
            continue;
        }
        let slaves: Vec<UnitId> = f.g.players[p0].units.iter().copied().filter(|&u| f.g.units[u].unit_type == slave).collect();
        assert_eq!(slaves.len(), 1, "seed {seed}: one slave");
        let su = slaves[0];
        assert_eq!(f.g.unit_plot(su), from, "seed {seed}: on the killer's plot");
        for a in f.g.players[p0].units.clone() {
            if a != su && f.alive(a) {
                assert_ne!(f.g.units[su].group, f.g.units[a].group, "seed {seed}");
                assert!(f.g.units[su].army.is_none() || f.g.units[su].army != f.g.units[a].army, "seed {seed}");
            }
        }
        enslaved += 1;
    }
    assert!(enslaved > 0, "the warrior never fell");
}

/// A Hidden attacker loses Hidden after the battle (removed by combat); with a visible enemy on its
/// plot and every plot around it taken, today's quirk kills it there and then. The post-battle
/// steps after that go on without the unit. Every battle (it always survives the fight).
#[test]
fn a_hidden_attacker_with_no_free_plot_dies_after_the_battle() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    // Eleven weak warriors: ten fight, one stays in reserve and keeps `to` taken.
    for d in w.units(p1, "UNIT_WARRIOR", to, 11) {
        w.g.units[d].damage = 90;
    }
    let hidden = w.unit(p0, "UNIT_AXEMAN", from);
    w.promote(hidden, "PROMOTION_HIDDEN");
    w.g.units[hidden].extra_combat_percent = 3000;
    w.g.units[hidden].extra_combat_percent_defense = 3000;
    assert!(w.g.unit_is_invisible(hidden, w.g.players[p1].team, true));
    // An enemy on the attacker's own plot (it does not see the Hidden unit) and on every other
    // land plot around it.
    let (fx, fy) = w.g.map.xy(from);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(q) = w.g.map.plot_xy(fx + dx, fy + dy) {
                if q != to && !w.g.plot_is_water(q) && !w.g.plot_is_peak(q) {
                    w.unit(p1, "UNIT_WARRIOR", q);
                }
            }
        }
    }
    assert!(w.g.map.plots[from].units.iter().any(|&u| w.g.units[u].owner == p1), "an enemy shares the attacker's plot");
    for seed in 0..4 {
        let mut f = w.fork(seed);
        f.g.telemetry = ffh_core::telemetry::Telemetry::new(None);
        let (attacked, reports) = f.attack(hidden);
        assert!(attacked);
        let r = &reports[0];
        assert_eq!((r.sides[0].committed, r.sides[0].lost.len()), (1, 0), "seed {seed}: it fought and survived");
        assert_ne!(r.outcome.result, EngageResult::Taken, "seed {seed}: the reserve holds the plot");
        assert!(!r.outcome.plot_taken && r.outcome.defenders_left >= 1, "seed {seed}: {:?}", r.outcome);
        assert!(!f.g.units.contains(hidden), "seed {seed}: no free plot, the quirk killed it");
        assert!(f.g.players[p0].units.iter().all(|&u| u != hidden));
        assert!(f.g.players[p0].armies.iter().all(|&a| f.g.armies[a].group.map_or(true, |g| f.g.groups.contains(g))), "seed {seed}: armies settled");
    }
}

/// Taunt makes each adjacent enemy attack the caster, one engagement each; once one has killed
/// the caster the spell ends: one report. Every cast.
#[test]
fn taunt_stops_when_the_caster_dies() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let caster = w.unit(p0, "UNIT_HARLEQUIN", from);
    assert!(w.g.d.unit_roles[w.g.units[caster].unit_type].is_some(), "the Harlequin defends in an engagement");
    w.g.units[caster].damage = 99;
    w.g.unit_change_extra_withdrawal(caster, -1000);
    let taunted = w.units(p1, "UNIT_AXEMAN", to, 3);
    for &u in &taunted {
        w.g.units[u].resist = -1000;
        w.g.units[u].extra_combat_percent = 900;
    }
    let taunt = w.g.d.t::<SpellType>("SPELL_TAUNT");
    for seed in 0..4 {
        let mut f = w.fork(seed);
        f.g.unit_cast(caster, taunt);
        assert!(!f.alive(caster), "seed {seed}: the caster fell");
        assert_eq!(f.g.battle_reports.len(), 1, "seed {seed}: one engagement, then the spell ends");
        assert_eq!(f.g.battle_reports[0].sides[0].committed, 1);
    }
}

/// Records the AI's pre-attack casting hook.
struct AttackSpells(AtomicUsize);

impl Controller for AttackSpells {
    fn hook(&self, _g: &mut Game, h: AiHook) {
        if matches!(h, AiHook::GroupAttackSpells(_)) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// `group_attack_force` with a formation runs today's AI preamble before the engagement: the
/// pre-attack casting hook, and the war declaration on the third party whose land the enemy
/// stands in (an AI with a war plan against it; the enemy is at war with it too, so the
/// declaration does not push it off the plot).
#[test]
fn a_forced_ai_attack_runs_the_preamble() {
    let mut w = world_of(true, "WORLDSIZE_TINY", 3);
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let p2 = PlayerId(2);
    let (t0, t1, t2) = (w.g.players[p0].team, w.g.players[p1].team, w.g.players[p2].team);
    w.g.team_meet(t0, t2, false);
    w.g.team_meet(t1, t2, false);
    w.g.team_declare_war(t1, t2, false, None);
    w.g.plot_set_owner(to, Some(p2), true, true);
    w.g.teams[t0].ai_war_plan[t2] = Some(WarPlanType::WARPLAN_LIMITED);
    assert!(!w.g.teams[t0].at_war[t2] && w.g.team_can_declare_war(t0, t2));
    let d = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[d].damage = 50;
    let a = w.unit(p0, "UNIT_AXEMAN", from);
    w.units(p0, "UNIT_AXEMAN", from, 2);
    let hook = Arc::new(AttackSpells(AtomicUsize::new(0)));
    w.g.controllers[p0] = hook.clone();
    let grp = w.g.units[a].group.unwrap();
    let (x, y) = w.g.map.xy(to);
    let r = w.g.group_attack_force(grp, x, y, MOVE_DIRECT_ATTACK, Some(vec![Place { unit: a, lane: 1, row: 0 }]));
    assert_eq!(hook.0.load(Ordering::SeqCst), 1, "the pre-attack casting hook ran once");
    assert!(w.g.teams[t0].at_war[t2], "the AI declared war on the plot's owner");
    assert_eq!(r, Ok((true, false)));
    assert_eq!(w.g.battle_reports.len(), 1);
    assert_eq!(w.g.battle_reports[0].sides[0].committed, 1, "the formation given");
}

/// A Fireball's kill counts as an engagement's kill does in telemetry (`kill_by`).
#[test]
fn a_missile_kill_counts_in_telemetry() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let v = w.unit(p1, "UNIT_WARRIOR", to);
    w.g.units[v].damage = 99;
    let fireball = w.unit(p0, "UNIT_FIREBALL", from);
    let mut f = w.fork(1);
    f.g.telemetry = ffh_core::telemetry::Telemetry::new(None);
    let (attacked, _) = f.attack(fireball);
    assert!(attacked && !f.alive(v), "the fireball killed the warrior");
    let json = f.g.telemetry_json().expect("telemetry on");
    assert!(json.contains("\"kill_by\":{\"UNIT_FIREBALL\":[1,"), "{json}");
    assert!(json.contains("\"engage_missile\":{\"kill\":[1,"), "{json}");
}

