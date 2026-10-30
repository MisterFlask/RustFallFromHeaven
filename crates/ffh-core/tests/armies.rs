//! Armies, garrisons, recruitment in transit and supply (docs/ARMIES.md).

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
fn a_unit_trained_for_an_army_travels_and_joins_it() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (bx, by) = next_to(&g, b);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, bx, by)).collect();
    let army = g.army_form(p0, &units).expect("formed");
    let cost_before = g.player_calculate_unit_cost(p0);
    // City a trains a warrior for the army.
    assert!(g.city_send_in_transit(a, warrior(&g), None, RecruitDest::Army(army)));
    assert_eq!(g.players[p0].transit.len(), 1);
    assert_eq!(g.army_transit_count(army), 1);
    assert!(g.player_calculate_unit_cost(p0) >= cost_before, "units in transit pay upkeep");
    let eta = g.players[p0].transit[0].eta;
    assert!(eta >= 1, "a journey of some turns: {eta}");
    let mut turns = 0;
    while !g.players[p0].transit.is_empty() {
        turns += 1;
        assert!(turns <= eta + 1, "arrived in time");
        g.player_do_transit(p0);
    }
    assert_eq!(g.army_units(army).len(), 3, "joined the army");
    let new = *g.army_units(army).iter().find(|u| !units.contains(u)).unwrap();
    assert_eq!(g.unit_plot(new), g.army_plot(army).unwrap());
}

#[test]
fn transit_never_loses_a_unit() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (bx, by) = next_to(&g, b);
    let u = unit_at(&mut g, bx, by);
    g.player_army_sync(p0);
    let army = g.units[u].army.unwrap();
    assert!(g.city_send_in_transit(a, warrior(&g), None, RecruitDest::Army(army)));
    // The army is destroyed on the way: the unit goes to a garrison instead.
    g.unit_kill(u, false, None);
    g.player_army_sync(p0);
    assert!(!g.armies.contains(army));
    let mut turns = 0;
    while !g.players[p0].transit.is_empty() {
        turns += 1;
        assert!(turns < 30, "never arrived");
        g.player_do_transit(p0);
    }
    let w = warrior(&g);
    let arrived: Vec<UnitId> = g.players[p0].units.iter().copied().filter(|&x| g.units[x].unit_type == w).collect();
    assert_eq!(arrived.len(), 1);
    assert!(g.unit_garrison_city(arrived[0]).is_some(), "in a garrison");
    assert!(g.log.entries.iter().any(|m| m.text.contains("instead")), "told");
}

#[test]
fn production_sends_the_unit_to_its_destination() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let w = warrior(&g);
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].order_queue[0].dest = Some(RecruitDest::Garrison(b));
    let needed = g.city_production_needed_unit(a, w);
    g.cities[a].unit_production[w] = needed;
    let before = g.players[p0].units.len();
    g.city_pop_order(a, 0, true, false);
    assert_eq!(g.players[p0].units.len(), before, "not on the map yet");
    assert_eq!(g.players[p0].transit.len(), 1);
    // The city's own garrison (the default): at once, as without armies.
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].unit_production[w] = needed;
    g.city_pop_order(a, 0, true, false);
    assert_eq!(g.players[p0].units.len(), before + 1);
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
fn armies_and_transit_survive_a_save() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (bx, by) = next_to(&g, b);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, bx, by)).collect();
    let army = g.army_form(p0, &units).expect("formed");
    g.army_rename(p0, army, "The Iron Host").unwrap();
    g.army_set_target_size(p0, army, 6).unwrap();
    assert!(g.city_send_in_transit(a, warrior(&g), None, RecruitDest::Army(army)));
    g.cities[a].recruit_default = Some(RecruitDest::Army(army));
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.armies[army].name, "The Iron Host");
    assert_eq!(h.armies[army].target_size, 6);
    assert_eq!(h.players[p0].transit, g.players[p0].transit);
    assert_eq!(h.cities[a].recruit_default, Some(RecruitDest::Army(army)));
    assert_eq!(h.save(), bytes);
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
fn garrison_units_join_an_army_and_a_whole_army_merges_with_its_reinforcements() {
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
    // A second army here, with a reinforcement on its way, merges into the first: the
    // reinforcement follows.
    let more: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, x, y)).collect();
    let other = g.army_form(p0, &more).unwrap();
    assert!(g.city_send_in_transit(b, warrior(&g), None, RecruitDest::Army(other)));
    g.army_merge(p0, other, army).expect("merged");
    assert!(!g.armies.contains(other));
    assert_eq!(g.army_units(army).len(), 5);
    assert_eq!(g.players[p0].transit[0].dest, RecruitDest::Army(army));
    // Not on the army's plot: refused, nothing changes.
    let away = unit_at(&mut g, ox, oy);
    assert!(g.army_join(p0, &[away], army).is_err());
    assert_eq!(g.army_units(army).len(), 5);
}

#[test]
fn reinforce_to_n_repeats_the_order_until_the_army_counts_n() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (bx, by) = next_to(&g, b);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, bx, by)).collect();
    let army = g.army_form(p0, &units).unwrap();
    let w = warrior(&g);
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].order_queue[0].dest = Some(RecruitDest::Army(army));
    g.cities[a].order_queue[0].reinforce_to = 4;
    let needed = g.city_production_needed_unit(a, w);
    let mut sent = 0;
    for _ in 0..5 {
        if g.cities[a].order_queue.is_empty() {
            break;
        }
        g.cities[a].unit_production[w] = needed;
        g.city_pop_order(a, 0, true, false);
        sent += 1;
    }
    // Two in the army and two on their way: four, so the order stopped after two.
    assert_eq!(sent, 2);
    assert_eq!(g.army_transit_count(army), 2);
    assert!(g.cities[a].order_queue.is_empty());
}

#[test]
fn grouping_two_armies_keeps_the_reinforcements_and_orders_of_both() {
    let (mut g, a, b) = game();
    let p0 = PlayerId(0);
    let (ox, oy) = next_to(&g, b);
    let first: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let second: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let one = g.army_form(p0, &first).unwrap();
    let two = g.army_form(p0, &second).unwrap();
    assert!(g.city_send_in_transit(a, warrior(&g), None, RecruitDest::Army(two)));
    let w = warrior(&g);
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].order_queue[0].dest = Some(RecruitDest::Army(two));
    // Civ4's Group button: the second army's units join the first's group.
    let grp = g.armies[one].group.unwrap();
    for &u in &second {
        g.unit_join_group(u, Some(grp), false);
    }
    g.player_army_sync(p0);
    assert!(!g.armies.contains(two));
    assert_eq!(g.army_units(one).len(), 4);
    assert_eq!(g.players[p0].transit[0].dest, RecruitDest::Army(one), "the reinforcement follows");
    assert_eq!(g.cities[a].order_queue[0].dest, Some(RecruitDest::Army(one)), "so does the order");
    // The army destroyed: the order falls back to the city's default.
    for u in g.army_units(one) {
        g.unit_kill(u, false, None);
    }
    g.player_army_sync(p0);
    assert_eq!(g.cities[a].order_queue[0].dest, None);
}
