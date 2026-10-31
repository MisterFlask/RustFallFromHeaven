//! Armies, garrisons, requisition, marching recruits and supply (docs/ARMIES.md).

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map with armies on, after `game_init`: player 0's first city `a` and a second city `b`
/// at least 3 plots away on the same landmass, the starting units gone.
fn game() -> (Game, CityId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    let p0 = PlayerId(0);
    let u = g.players[p0].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let area = g.map.plots[g.unit_plot(u)].area;
    for u in g.players[p0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let a = g.player_init_city(p0, x, y, true, true);
    let mut spot = None;
    'find: for r in 3..=6 {
        for dy in -r..=r {
            for dx in -r..=r {
                let Some(q) = g.map.plot_xy(x + dx, y + dy) else { continue };
                if !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].area == area && g.plot_distance(x, y, x + dx, y + dy) == r {
                    spot = Some(g.map.xy(q));
                    break 'find;
                }
            }
        }
    }
    let (bx, by) = spot.expect("room for a second city");
    let b = g.player_init_city(p0, bx, by, true, true);
    (g, a, b)
}

fn warrior(g: &Game) -> UnitType {
    g.d.t::<UnitType>("UNIT_WARRIOR")
}

fn unit_at(g: &mut Game, x: i32, y: i32) -> UnitId {
    let w = warrior(g);
    g.player_init_unit(PlayerId(0), w, x, y, None, None)
}

/// A land plot next to city `c`, outside it.
fn next_to(g: &Game, c: CityId) -> (i32, i32) {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
        if let Some(q) = g.map.plot_xy(x + dx, y + dy) {
            if !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none() {
                return g.map.xy(q);
            }
        }
    }
    panic!("no land next to the city");
}

/// An army of `n` warriors of player 0 next to city `c`.
fn army_near(g: &mut Game, c: CityId, n: usize) -> ArmyId {
    let (x, y) = next_to(g, c);
    let units: Vec<UnitId> = (0..n).map(|_| unit_at(g, x, y)).collect();
    g.army_form(PlayerId(0), &units).expect("formed")
}

/// City `c` finishes a warrior for `dest` (an order with that destination, completed at once);
/// returns the new unit.
fn train_for(g: &mut Game, c: CityId, dest: Option<RecruitDest>) -> UnitId {
    let p0 = PlayerId(0);
    let w = warrior(g);
    g.city_push_order(c, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[c].order_queue[0].dest = dest;
    g.cities[c].unit_production[w] = g.city_production_needed_unit(c, w);
    let before: Vec<UnitId> = g.players[p0].units.clone();
    g.city_pop_order(c, 0, true, false);
    *g.players[p0].units.iter().find(|u| !before.contains(u)).expect("a new unit")
}

/// A turn of player 0's units: their moves back, the marching recruits march (as the turn
/// begins), then the end-of-turn joining (as its turn processing does).
fn march_turn(g: &mut Game) {
    let p0 = PlayerId(0);
    g.player_do_turn_units(p0);
    g.player_march_join(p0);
    g.player_army_sync(p0);
}

#[test]
fn units_in_a_city_are_its_garrison_and_units_outside_are_armies() {
    let (mut g, a, _) = game();
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    let u = unit_at(&mut g, x, y);
    g.player_army_sync(p0);
    assert_eq!(g.units[u].army, None);
    assert_eq!(g.unit_garrison_city(u), Some(a));
    assert_eq!(g.city_garrison_units(a), vec![u]);
    // Outside the city: a one-unit army.
    let (ox, oy) = next_to(&g, a);
    let v = unit_at(&mut g, ox, oy);
    g.player_army_sync(p0);
    let army = g.units[v].army.expect("a one-unit army");
    assert_eq!(g.army_units(army), vec![v]);
    assert_eq!(g.armies[army].name, "1st Army");
}

#[test]
fn form_split_merge_and_garrison() {
    let (mut g, a, _) = game();
    let p0 = PlayerId(0);
    let (ox, oy) = next_to(&g, a);
    let units: Vec<UnitId> = (0..4).map(|_| unit_at(&mut g, ox, oy)).collect();
    let army = g.army_form(p0, &units).expect("formed");
    assert_eq!(g.army_units(army).len(), 4);
    let grp = g.armies[army].group.unwrap();
    assert!(units.iter().all(|&u| g.units[u].group == Some(grp)), "one group");
    // Split two off: a new army; the rest keeps the old one.
    let other = g.army_split(p0, &units[2..]).expect("split");
    assert_ne!(other, army);
    assert_eq!(g.army_units(army), units[..2].to_vec());
    assert_eq!(g.army_units(other), units[2..].to_vec());
    // Merge back.
    g.army_merge(p0, other, army).expect("merged");
    assert!(!g.armies.contains(other), "the merged army is gone");
    assert_eq!(g.army_units(army).len(), 4);
    // Into a city (the army moves as one group): an army stays an army; to the garrison on
    // command.
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    // (`setXY` with `bGroup`: each unit keeps its group)
    for &u in &units {
        g.unit_set_xy(u, x, y, true);
    }
    g.player_army_sync(p0);
    assert!(units.iter().all(|&u| g.units[u].army == Some(army)), "still the army in the city");
    g.army_to_garrison(p0, &units).expect("garrisoned");
    assert!(units.iter().all(|&u| g.unit_garrison_city(u) == Some(a)));
    assert!(!g.armies.contains(army), "an army without units is gone");
}

#[test]
fn a_recruit_is_a_real_unit_that_marches_to_its_army_and_joins_it() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 2);
    let units = g.army_units(army);
    let u = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    // On the map in the city that trained it, marching: in no army and no garrison, no orders.
    assert_eq!(g.unit_plot(u), g.city_plot(a));
    assert_eq!(g.unit_march(u).map(|m| m.dest), Some(RecruitDest::Army(army)));
    assert_eq!(g.units[u].army, None);
    assert_eq!(g.unit_garrison_city(u), None);
    assert!(g.city_garrison_units(a).is_empty());
    assert_eq!(g.army_marching(army), vec![u]);
    assert_eq!(g.unit_march_refusal(u).as_deref(), Some("marching to the 1st Army"));
    assert!(g.army_form(p0, &[u]).is_err(), "army commands refuse it");
    assert!(g.log.entries.iter().any(|m| m.text.contains("marches to the 1st Army")), "told");
    // The sync leaves it alone.
    g.player_army_sync(p0);
    assert_eq!(g.units[u].army, None);
    let eta = g.unit_march_eta(u).expect("a way there");
    assert!(eta >= 1, "some turns away: {eta}");
    let mut turns = 0;
    let mut last = g.unit_plot(u);
    while g.units[u].march.is_some() {
        turns += 1;
        assert!(turns <= 20, "arrived");
        march_turn(&mut g);
        // The rules moved it, and it holds for the rest of the turn.
        if g.units[u].march.is_some() {
            assert_ne!(g.unit_plot(u), last, "progress each turn");
            assert_eq!(g.groups[g.units[u].group.unwrap()].activity_type, ActivityType::ACTIVITY_HOLD);
        }
        last = g.unit_plot(u);
    }
    assert_eq!(g.units[u].army, Some(army), "joined the army");
    assert_eq!(g.army_units(army).len(), units.len() + 1);
    assert_eq!(g.unit_plot(u), g.army_plot(army).unwrap());
    assert_eq!(g.units[u].group, g.armies[army].group, "in its group");
    assert!(g.log.entries.iter().any(|m| m.text.contains("joined the 1st Army")), "told");
}

#[test]
fn production_for_the_own_garrison_stays_and_an_army_in_the_city_takes_the_unit_at_once() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    // Another city's garrison: on the map at once, marching there.
    let u = train_for(&mut g, a, Some(RecruitDest::Garrison(b)));
    assert_eq!(g.unit_march(u).map(|m| m.dest), Some(RecruitDest::Garrison(b)));
    // The city's own garrison (the default): as without armies.
    let v = train_for(&mut g, a, None);
    assert!(g.unit_march(v).is_none());
    assert_eq!(g.unit_garrison_city(v), Some(a));
    // An army standing in the city takes the unit at once.
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    let (ox, oy) = next_to(&g, a);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let army = g.army_form(p0, &units).unwrap();
    for &w in &units {
        g.unit_set_xy(w, x, y, true);
    }
    g.player_army_sync(p0);
    let r = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    assert!(g.unit_march(r).is_none());
    assert_eq!(g.units[r].army, Some(army));
}

#[test]
fn requisition_puts_front_orders_and_stops_at_the_target_size() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 2);
    // City a was building something: the requisition interrupts it, which keeps its progress.
    let building = g.d.buildings.keys().find(|&k| g.city_can_construct(a, k, false, false, false)).expect("a building");
    g.city_push_order(a, OrderType::ORDER_CONSTRUCT, building.idx() as i32, -1, false, false, true, false);
    g.cities[a].building_production[building] = 7;
    // No target size: nothing requisitioned.
    g.army_add_requisition(p0, army, a).unwrap();
    g.army_add_requisition(p0, army, b).unwrap();
    assert!(g.army_add_requisition(p0, army, b).is_err(), "once");
    assert_eq!(g.armies[army].requisition, vec![a, b]);
    g.player_do_requisition(p0);
    assert!(g.army_requisition_orders(army).is_empty(), "no target size, no requisition");
    // Four wanted, two there: one order in each city, at the front.
    g.army_set_target_size(p0, army, 4).unwrap();
    assert_eq!(g.army_shortfall(army), 2);
    g.player_do_requisition(p0);
    for c in [a, b] {
        let o = &g.cities[c].order_queue[0];
        assert!(o.requisition, "a requisition at the front of {c:?}");
        assert_eq!(o.dest, Some(RecruitDest::Army(army)));
        assert!(g.unit_type_is_army_unit(UnitType::from_usize(o.data1 as usize)));
    }
    assert_eq!(g.cities[a].order_queue[1].data1, building.idx() as i32, "the building waits behind it");
    assert_eq!(g.cities[a].building_production[building], 7, "with its progress");
    assert_eq!(g.army_shortfall(army), 0);
    assert!(g.log.entries.iter().any(|m| m.text.contains("requisitions")), "told");
    // Once a turn, one per city: nothing more.
    g.player_do_requisition(p0);
    assert_eq!(g.army_requisition_orders(army).len(), 2);
    // Not by hand: removing it is refused, replacing the head replaces what is behind it, and an
    // order pushed to the front goes behind it.
    assert!(g.city_check_remove_order(a, 0).unwrap_err().contains("requisitioned by the 1st Army"));
    assert!(g.city_check_remove_order(a, 1).is_ok());
    let w = warrior(&g);
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, true, false, false);
    assert!(g.cities[a].order_queue[0].requisition, "still first");
    assert_eq!(g.cities[a].order_queue[1].data1, w.idx() as i32, "the building was replaced");
    g.city_push_order(a, OrderType::ORDER_CONSTRUCT, building.idx() as i32, -1, false, false, false, false);
    assert!(g.cities[a].order_queue[0].requisition);
    assert_eq!(g.cities[a].order_queue[1].data1, building.idx() as i32, "pushed to the front: behind it");
    // The army needs fewer: the order that would join last is withdrawn.
    g.army_set_target_size(p0, army, 3).unwrap();
    g.player_do_requisition(p0);
    assert_eq!(g.army_requisition_orders(army).len(), 1);
    // Released: its order goes at once.
    let (c, _) = g.army_requisition_orders(army)[0];
    g.army_remove_requisition(p0, army, c).unwrap();
    assert!(g.army_requisition_orders(army).is_empty());
    assert!(g.cities[c].order_queue.iter().all(|o| !o.requisition));
}

#[test]
fn a_requisition_unit_marches_and_counts_toward_the_target() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 1);
    g.army_set_target_size(p0, army, 2).unwrap();
    g.army_set_requisition(p0, army, &[a]).unwrap();
    g.player_do_requisition(p0);
    assert_eq!(g.army_requisition_orders(army), vec![(a, 0)]);
    // Finished: it marches, and the army is not short any more.
    let unit = UnitType::from_usize(g.cities[a].order_queue[0].data1 as usize);
    g.cities[a].unit_production[unit] = g.city_production_needed_unit(a, unit);
    g.city_pop_order(a, 0, true, false);
    assert_eq!(g.army_marching(army).len(), 1);
    assert_eq!(g.army_shortfall(army), 0);
    g.player_do_requisition(p0);
    assert!(g.army_requisition_orders(army).is_empty(), "nothing more");
}

#[test]
fn supply_gates_the_march_and_a_long_stall_goes_to_a_garrison() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    // An army out of supply: on our landmass, far from both cities.
    let area = g.map.plots[g.city_plot(a)].area;
    let far = g
        .map
        .plot_ids()
        .filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].area == area && !g.plot_in_supply(p0, q))
        .min_by_key(|&q| {
            let (qx, qy) = g.map.xy(q);
            (g.plot_distance(qx, qy, g.cities[a].x, g.cities[a].y), q)
        });
    let (fx, fy) = g.map.xy(far.expect("land out of supply on our landmass"));
    let army_units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, fx, fy)).collect();
    let army = g.army_form(p0, &army_units).unwrap();
    assert!(!g.army_in_supply(army));
    let u = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    let at = g.unit_plot(u);
    let stall = g.d.defines.army_transit_stall_turns;
    for turn in 1..stall {
        march_turn(&mut g);
        assert_eq!(g.unit_plot(u), at, "it waits where it is");
        assert_eq!(g.unit_march(u).map(|m| m.stalled), Some(turn));
    }
    assert!(g.log.entries.iter().any(|m| m.text.contains("waits: the army is out of supply")), "told");
    // The last turn of the stall: to the nearest garrison it can reach (the city it stands in).
    march_turn(&mut g);
    assert!(g.unit_march(u).is_none(), "arrived");
    assert_eq!(g.unit_garrison_city(u), Some(a));
    assert!(g.log.entries.iter().any(|m| m.text.contains("instead: the army is out of supply")), "told");
    let _ = b;
}

#[test]
fn the_destination_gone_sends_the_recruit_to_the_nearest_garrison() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 1);
    let u = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    march_turn(&mut g);
    assert!(g.unit_march(u).is_some_and(|m| m.dest == RecruitDest::Army(army)));
    // The army is destroyed on the way: at once to the nearest garrison.
    for v in g.army_units(army) {
        g.unit_kill(v, false, None);
    }
    g.player_army_sync(p0);
    assert!(!g.armies.contains(army));
    let m = g.unit_march(u).cloned();
    if let Some(m) = m {
        assert!(matches!(m.dest, RecruitDest::Garrison(_)) && m.diverted, "{m:?}");
        let mut turns = 0;
        while g.units[u].march.is_some() {
            turns += 1;
            assert!(turns < 20, "arrived");
            march_turn(&mut g);
        }
    }
    assert!(g.unit_garrison_city(u).is_some(), "in a garrison");
    assert!(g.log.entries.iter().any(|m| m.text.contains("instead: its army is gone")), "told");
    // A garrison lost: the recruit goes to another (here the city it stands in).
    let v = train_for(&mut g, a, Some(RecruitDest::Garrison(b)));
    assert!(g.unit_march(v).is_some());
    g.city_kill(b, true);
    assert!(g.unit_march(v).is_none());
    assert_eq!(g.unit_garrison_city(v), Some(a));
}

#[test]
fn a_disbanded_army_sends_its_recruits_to_its_city_and_a_merged_one_to_the_other_army() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (ox, oy) = next_to(&g, b);
    let first: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let second: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let one = g.army_form(p0, &first).unwrap();
    let two = g.army_form(p0, &second).unwrap();
    let u = train_for(&mut g, a, Some(RecruitDest::Army(two)));
    g.army_set_target_size(p0, two, 5).unwrap();
    g.army_add_requisition(p0, two, a).unwrap();
    g.army_add_requisition(p0, one, b).unwrap();
    let w = warrior(&g);
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].order_queue[0].dest = Some(RecruitDest::Army(two));
    // Civ4's Group button: the second army's units join the first's group.
    let grp = g.armies[one].group.unwrap();
    for &v in &second {
        g.unit_join_group(v, Some(grp), false);
    }
    g.player_army_sync(p0);
    assert!(!g.armies.contains(two));
    assert_eq!(g.army_units(one).len(), 4);
    assert_eq!(g.unit_march(u).map(|m| m.dest), Some(RecruitDest::Army(one)), "the recruit follows");
    assert_eq!(g.cities[a].order_queue[0].dest, Some(RecruitDest::Army(one)), "so does the order");
    assert_eq!(g.armies[one].requisition, vec![b, a], "and the requisition");
    // Disbanded: the recruit goes to the city the army marches to; the requisition ends.
    g.army_disband(p0, one).unwrap();
    let to = g.armies[one].disband_to.expect("marching home");
    assert_eq!(g.unit_march(u).map(|m| m.dest), Some(RecruitDest::Garrison(to)));
    assert!(g.armies[one].requisition.is_empty());
    // The army destroyed: the order falls back to the city's default.
    for v in g.army_units(one) {
        g.unit_kill(v, false, None);
    }
    g.player_army_sync(p0);
    assert_eq!(g.cities[a].order_queue[0].dest, None);
}

#[test]
fn a_recruit_can_be_killed_on_the_way() {
    let (mut g, a, b) = game();
    let army = army_near(&mut g, b, 1);
    let u = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    march_turn(&mut g);
    g.unit_kill(u, false, Some(PlayerId(1)));
    assert!(!g.units.contains(u));
    assert!(g.log.entries.iter().any(|m| m.text.contains("was killed on its way to the 1st Army")), "told");
    assert!(g.army_marching(army).is_empty());
}

#[test]
fn supply_follows_the_network() {
    let (g, a, _) = game();
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    let here = g.city_plot(a);
    assert!(g.plot_in_supply(p0, here));
    // Far away (beyond the range of anything connected): out of supply.
    let far = g.map.plot_ids().filter(|&q| !g.plot_is_water(q)).max_by_key(|&q| {
        let (qx, qy) = g.map.xy(q);
        g.plot_distance(x, y, qx, qy)
    });
    if let Some(q) = far {
        let (qx, qy) = g.map.xy(q);
        if g.plot_distance(x, y, qx, qy) > 12 {
            assert!(!g.plot_in_supply(p0, q));
        }
    }
}

#[test]
fn armies_requisition_and_marching_recruits_survive_a_save() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 2);
    g.army_rename(p0, army, "The Iron Host").unwrap();
    g.army_set_target_size(p0, army, 6).unwrap();
    g.army_set_requisition(p0, army, &[b, a]).unwrap();
    let u = train_for(&mut g, a, Some(RecruitDest::Army(army)));
    march_turn(&mut g);
    g.player_do_requisition(p0);
    g.cities[a].recruit_default = Some(RecruitDest::Army(army));
    assert!(!g.army_requisition_orders(army).is_empty());
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.armies[army], g.armies[army]);
    assert_eq!(h.armies[army].requisition, vec![b, a]);
    assert_eq!(h.units[u].march, g.units[u].march);
    assert!(h.units[u].march.is_some());
    assert_eq!(h.army_requisition_orders(army), g.army_requisition_orders(army));
    assert_eq!(h.cities[a].recruit_default, Some(RecruitDest::Army(army)));
    assert_eq!(h.save(), bytes);
}

#[test]
fn the_march_is_deterministic() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 1);
    for c in [a, b] {
        train_for(&mut g, c, Some(RecruitDest::Army(army)));
    }
    let u = train_for(&mut g, b, Some(RecruitDest::Garrison(a)));
    // The same game from a save, played on in the same way: the same game.
    let mut h = Game::load(g.d, &g.save()).unwrap();
    for _ in 0..6 {
        march_turn(&mut g);
        march_turn(&mut h);
        assert_eq!(g.save(), h.save());
    }
    assert!(g.unit_march(u).is_none(), "the garrison recruit arrived");
    let _ = p0;
}

#[test]
fn disbanding_marches_the_units_home() {
    let (mut g, a, _) = game();
    let p0 = PlayerId(0);
    let (ox, oy) = next_to(&g, a);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let army = g.army_form(p0, &units).expect("formed");
    g.army_disband(p0, army).expect("disbanded");
    assert_eq!(g.armies[army].disband_to, Some(a));
    // They arrive (moved here directly, as a group) and join the garrison.
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    // (`setXY` with `bGroup`: each unit keeps its group)
    for &u in &units {
        g.unit_set_xy(u, x, y, true);
    }
    g.player_army_sync(p0);
    assert!(units.iter().all(|&u| g.unit_garrison_city(u) == Some(a)));
    assert!(!g.armies.contains(army));
}

#[test]
fn off_without_the_option() {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let p0 = PlayerId(0);
    g.player_army_sync(p0);
    assert!(g.armies.iter().next().is_none());
    assert!(g.players[p0].units.iter().all(|&u| g.units[u].army.is_none()));
}

#[test]
fn garrison_units_join_an_army_and_a_whole_army_merges_with_its_recruits() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    let g1 = unit_at(&mut g, x, y);
    g.player_army_sync(p0);
    assert_eq!(g.unit_garrison_city(g1), Some(a));
    // An army in the city: the garrison unit joins it.
    let (ox, oy) = next_to(&g, a);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let army = g.army_form(p0, &units).unwrap();
    for &u in &units {
        g.unit_set_xy(u, x, y, true);
    }
    g.army_join(p0, &[g1], army).expect("joined");
    assert_eq!(g.units[g1].army, Some(army));
    assert_eq!(g.army_units(army).len(), 3);
    // A second army here, with a recruit on its way, merges into the first: the recruit follows.
    let more: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, x, y)).collect();
    let other = g.army_form(p0, &more).unwrap();
    let r = train_for(&mut g, b, Some(RecruitDest::Army(other)));
    g.army_merge(p0, other, army).expect("merged");
    assert!(!g.armies.contains(other));
    assert_eq!(g.army_units(army).len(), 5);
    assert_eq!(g.unit_march(r).map(|m| m.dest), Some(RecruitDest::Army(army)));
    // Not on the army's plot: refused, nothing changes.
    let away = unit_at(&mut g, ox, oy);
    assert!(g.army_join(p0, &[away], army).is_err());
    assert_eq!(g.army_units(army).len(), 5);
}
