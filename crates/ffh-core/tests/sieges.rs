//! Sieges and Sentinels (docs/SIEGES.md): walls bring Sentinels that grow with the city's defence;
//! a walled city with no soldiers fights with them and falls only when they are breached; they
//! intercept the shots and spells aimed past them and siege takes them first; a besieged city
//! neither heals its units nor repairs its Sentinels; a bombard hits Sentinels and soldiers (never
//! below the floor); a conquered city's Sentinels are breached; a kill's XP is shared equally.

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::formation::{Fighter, Formation, Ground, Pair, Params, Side, State};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war, `from` and `to` two unowned flat grassland plots side by
/// side; a city of player 1 on `to` (no walls yet).
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
    city: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && !g.plot_is_peak(p) && !g.plot_is_hills(p) && plot.units.is_empty() && plot.owner.is_none() && plot.feature_type.is_none() && plot.improvement_type.is_none()
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
    let (x, y) = g.map.xy(to);
    let city = g.player_init_city(p1, x, y, true, true);
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    World { g, p0, p1, from, to, city }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn walls(&mut self) {
        let walls = self.g.d.t::<BuildingType>("BUILDING_WALLS");
        self.g.city_set_num_real_building(self.city, walls, 1);
    }

    /// Attacks `to` with `u`'s group; the reports it made.
    fn attack(&mut self, u: UnitId) -> Vec<ffh_core::engagement::report::BattleReport> {
        let before = self.g.battle_report_next;
        let grp = self.g.units[u].group.expect("a group");
        let (x, y) = self.g.map.xy(self.to);
        self.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
        self.g.battle_reports.iter().filter(|r| r.id >= before).cloned().collect()
    }
}

#[test]
fn walls_bring_sentinels_that_grow_with_the_citys_defence() {
    let mut w = world();
    let c = w.city;
    assert_eq!(w.g.city_sentinel_count(c), 0, "no walls, no Sentinels");
    assert!(w.g.engage_city_sentinels(w.to).is_none());
    w.walls();
    assert_eq!(w.g.city_sentinel_count(c), 2);
    assert_eq!(w.g.city_sentinels_standing(c), vec![0, 1]);
    assert_eq!(w.g.cities[c].sentinel_damage, vec![0, 0], "new walls stand whole");
    let def = w.g.city_total_defense(c, false);
    assert!(def >= 25, "the walls' own defence: {def}");
    assert_eq!(w.g.city_sentinel_max_hp(c), 100 * (100 + def) / 100);
    assert_eq!(w.g.city_sentinel_dv_x100(c), 3 * (100 + def));
    assert_eq!(w.g.sentinel_a_x100(), 100);
    // More defence, stronger Sentinels.
    w.g.cities[c].building_defense += 50;
    assert_eq!(w.g.city_sentinel_max_hp(c), 100 * (150 + def) / 100);
    // A breached Sentinel does not fight.
    w.g.city_set_sentinel_damage(c, 1, 100);
    assert_eq!(w.g.city_sentinels_standing(c), vec![0]);
}

#[test]
fn a_walled_city_without_soldiers_fights_its_sentinels() {
    let mut w = world();
    let (p0, from, to, c) = (w.p0, w.from, w.to, w.city);
    // Walls built while no enemy is near stand whole (built under siege they would start breached).
    w.walls();
    assert_eq!(w.g.city_sentinels_standing(c), vec![0, 1]);
    let probe = w.unit(p0, "UNIT_AXEMAN", from);
    assert!(w.g.city_is_besieged(c), "the axeman next to it besieges it");
    assert!(!w.g.unit_can_move_into(probe, to, false, false, false), "its Sentinels stop a plain move");
    assert!(w.g.unit_can_move_into(probe, to, true, false, false), "and make it an attack");
    w.g.player_army_sync(p0);
    let reports = w.attack(probe);
    assert_eq!(reports.len(), 1, "one battle against the Sentinels");
    let r = &reports[0];
    assert_eq!((r.outcome.sentinels, r.sides[1].committed), (2, 0), "two Sentinels, no soldier");
    assert_eq!(w.g.cities[c].owner, w.p1, "one axeman (six strikes at most) does not breach two Sentinels in one battle");
    assert!(!r.outcome.plot_taken);
    assert!(w.g.cities[c].sentinel_damage.iter().any(|&d| d > 0), "the walls took damage: {:?}", w.g.cities[c].sentinel_damage);
    assert_eq!(r.outcome.defenders_left, 2 - r.outcome.sentinels_breached);
    // Breached: the city is open to a move again.
    w.g.city_set_sentinel_damage(c, 0, 100);
    w.g.city_set_sentinel_damage(c, 1, 100);
    let fresh = w.unit(p0, "UNIT_AXEMAN", from);
    assert!(w.g.unit_can_move_into(fresh, to, false, false, false), "breached walls do not defend");
}

#[test]
fn sentinels_take_the_shots_down_their_lanes_and_siege_takes_them_first() {
    let p = Params::default();
    let shoot = |count| RowActions { kind: ActionKind::Shoot, count };
    let bombard = |count| RowActions { kind: ActionKind::Bombard, count };
    let none = RowActions { kind: ActionKind::None, count: 0 };
    let mut archer = Fighter::new(Side::Attacker, 80);
    archer.row = 1;
    archer.actions = [shoot(1), shoot(2), shoot(2)];
    let mut catapult = Fighter::new(Side::Attacker, 80);
    (catapult.lane, catapult.row) = (1, 2);
    catapult.actions = [none, bombard(1), bombard(2)];
    let at = |lane, row, fixed| {
        let mut f = Fighter::new(Side::Defender, 120);
        (f.lane, f.row, f.fixed) = (lane, row, fixed);
        f
    };
    // Lane 0: a Sentinel in front of an archer at the back; lane 1: a soldier in front, one behind.
    let fighters = vec![archer, catapult, at(0, 0, true), at(0, 2, false), at(1, 0, false), at(1, 1, false)];
    let f = Formation::new(p, Ground::default(), [2, 2], fighters, |_, _| Pair { a: 400, dv: 400 });
    let s = State::new(&f);
    assert_eq!(f.reach(&s, 0, ActionKind::Shoot), vec![2, 5], "lane 0's shot hits the Sentinel; lane 1's the back");
    assert_eq!(f.reach(&s, 0, ActionKind::Spell), vec![2, 5], "spells too");
    assert_eq!(f.reach(&s, 1, ActionKind::Bombard), vec![2], "siege takes the Sentinel first");
    // A Sentinel is never healed in the battle.
    let mut s2 = State::new(&f);
    s2.hp[2] = 100;
    assert_eq!(f.heal_target(&s2, 3), None, "the hurt Sentinel is not healed");
}

#[test]
fn a_besieged_city_neither_heals_its_units_nor_repairs_its_sentinels() {
    let mut w = world();
    let (p0, p1, from, to, c) = (w.p0, w.p1, w.from, w.to, w.city);
    w.walls();
    let defender = w.unit(p1, "UNIT_AXEMAN", to);
    w.g.units[defender].damage = 50;
    w.g.city_set_sentinel_damage(c, 0, 50);
    assert!(!w.g.city_is_besieged(c));
    let open = w.g.unit_heal_rate(defender, to);
    w.g.city_repair_sentinels(c);
    assert_eq!(w.g.cities[c].sentinel_damage[0], 40, "repairs 10% a turn");

    let besieger = w.unit(p0, "UNIT_WARRIOR", from);
    assert!(w.g.city_is_besieged(c));
    assert_eq!(w.g.city_besiegers(c), vec![besieger]);
    let besieged = w.g.unit_heal_rate(defender, to);
    assert!(besieged <= open - w.g.d.defines.city_heal_rate, "no heal from the city: {besieged} against {open}");
    w.g.city_repair_sentinels(c);
    assert_eq!(w.g.cities[c].sentinel_damage[0], 40, "no repair under siege");
}

#[test]
fn a_bombard_hits_the_sentinels_and_the_soldiers_above_the_floor() {
    let mut w = world();
    let (p0, p1, from, to, c) = (w.p0, w.p1, w.from, w.to, w.city);
    w.walls();
    let soldier = w.unit(p1, "UNIT_AXEMAN", to);
    let catapult = w.unit(p0, "UNIT_CATAPULT", from);
    assert!(w.g.unit_can_bombard(catapult, from));
    assert!(w.g.unit_bombard(catapult));
    // Catapult 15, less the Walls' bombard defence 25%: 11% to each Sentinel, half to the soldier.
    assert_eq!(w.g.cities[c].sentinel_damage, vec![11, 11]);
    assert_eq!(w.g.units[soldier].damage, 5);
    // Near the floor (50%): a bombard takes it to the floor and no further.
    w.g.units[soldier].damage = 48;
    let second = w.unit(p0, "UNIT_CATAPULT", from);
    assert!(w.g.unit_bombard(second));
    assert_eq!(w.g.units[soldier].damage, 50);
    assert_eq!(w.g.cities[c].sentinel_damage, vec![22, 22]);
    // Walls breached and the soldier at the floor: nothing left to bombard.
    w.g.city_set_sentinel_damage(c, 0, 100);
    w.g.city_set_sentinel_damage(c, 1, 100);
    let third = w.unit(p0, "UNIT_CATAPULT", from);
    assert!(!w.g.unit_can_bombard(third, from));
}

#[test]
fn a_conquered_citys_sentinels_are_breached() {
    // Walls survive a conquest at their iConquestProb (25%): over a few seeds some do, and those
    // stand breached for the new owner.
    let mut kept = 0;
    for seed in 0..24u32 {
        let mut w = world();
        let (p0, to) = (w.p0, w.to);
        w.walls();
        w.g.city_set_population(w.city, 5);
        w.g.soren_rand = Rng::new(seed);
        w.g.player_acquire_city(p0, w.city, true, false, true);
        let c = w.g.map.plots[to].plot_city.expect("the city");
        assert_eq!(w.g.cities[c].owner, p0);
        if w.g.city_sentinel_count(c) == 0 {
            continue; // the walls fell with the city
        }
        kept += 1;
        assert_eq!(w.g.cities[c].sentinel_damage, vec![100, 100], "breached");
    }
    assert!(kept > 0, "some seed keeps the walls");
}

#[test]
fn a_kills_xp_is_shared_equally() {
    // Four Axemen against one badly hurt Warrior outside any city: whoever kills it, the kill's XP
    // is split equally among the four (hundredths kept in `xp_carry`, the remainder to the killer).
    let mut kills = 0;
    for seed in 0..8u32 {
        let mut w = world();
        let (p0, p1, from) = (w.p0, w.p1, w.from);
        // Fight on open ground: a plot next to `from` other than the city.
        let (x, y) = w.g.map.xy(from);
        let open = (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
            .filter_map(|(dx, dy)| w.g.map.plot_xy(x + dx, y + dy))
            .find(|&p| p != from && p != w.to && !w.g.plot_is_water(p) && !w.g.plot_is_peak(p) && w.g.map.plots[p].units.is_empty() && w.g.map.plots[p].plot_city.is_none());
        let Some(open) = open else { continue };
        w.to = open;
        let axes: Vec<UnitId> = (0..4).map(|_| w.unit(p0, "UNIT_AXEMAN", from)).collect();
        let victim = w.unit(p1, "UNIT_WARRIOR", open);
        w.g.units[victim].damage = 95;
        w.g.player_army_sync(p0);
        w.g.soren_rand = Rng::new(seed);
        let before: Vec<i32> = axes.iter().map(|&u| w.g.units[u].experience * 100 + w.g.units[u].xp_carry).collect();
        w.attack(axes[0]);
        if w.g.units.contains(victim) && !w.g.unit_is_dead(victim) {
            continue;
        }
        let gained: Vec<i32> = axes.iter().zip(&before).filter(|(&u, _)| w.g.units.contains(u)).map(|(&u, b)| w.g.units[u].experience * 100 + w.g.units[u].xp_carry - b).collect();
        if gained.len() < 4 {
            continue;
        }
        kills += 1;
        let total: i32 = gained.iter().sum();
        let (min, max) = (w.g.d.defines.min_experience_per_combat, w.g.d.defines.max_experience_per_combat);
        assert!(total % 100 == 0 && total / 100 >= min && total / 100 <= max.max(min), "a kill's XP whole: {gained:?}");
        let low = *gained.iter().min().unwrap();
        assert!(gained.iter().all(|&g| g - low < 4), "equal shares but the remainder: {gained:?}");
    }
    assert!(kills > 0, "some seed kills the Warrior");
}
