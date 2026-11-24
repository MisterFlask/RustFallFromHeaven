//! Armies and garrisons (docs/ARMIES.md).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map with armies on, after `game_init`: player 0's first city `a` and a second city `b`
/// at least 3 plots away on the same landmass, the starting units gone.
fn game() -> (Game, CityId, CityId) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
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

/// City `c` finishes a warrior (an order completed at once); returns the new unit.
fn train(g: &mut Game, c: CityId) -> UnitId {
    let p0 = PlayerId(0);
    let w = warrior(g);
    g.city_push_order(c, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[c].unit_production[w] = g.city_production_needed_unit(c, w);
    let before: Vec<UnitId> = g.players[p0].units.clone();
    g.city_pop_order(c, 0, true, false);
    *g.players[p0].units.iter().find(|u| !before.contains(u)).expect("a new unit")
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

/// A trained unit appears in the city that trained it, in its garrison: an army standing in the
/// city does not take it.
#[test]
fn a_trained_unit_stays_in_its_city() {
    let (mut g, a, _) = game();
    let p0 = PlayerId(0);
    let (x, y) = (g.cities[a].x, g.cities[a].y);
    let (ox, oy) = next_to(&g, a);
    let units: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, ox, oy)).collect();
    let army = g.army_form(p0, &units).unwrap();
    for &w in &units {
        g.unit_set_xy(w, x, y, true);
    }
    g.player_army_sync(p0);
    let r = train(&mut g, a);
    g.player_army_sync(p0);
    assert_eq!(g.unit_plot(r), g.city_plot(a));
    assert_eq!(g.units[r].army, None);
    assert_eq!(g.unit_garrison_city(r), Some(a));
    assert_eq!(g.army_units(army).len(), 2);
    // An ordinary unit: army commands take it.
    g.army_join(p0, &[r], army).expect("joined");
    assert_eq!(g.army_units(army).len(), 3);
}

/// A hero type is no army unit type (no army keeps a hero).
#[test]
fn heroes_are_no_army_units() {
    let g = game().0;
    let d = g.d;
    let hero = d.t::<PromotionType>("PROMOTION_HERO");
    let sphener = d.t::<UnitType>("UNIT_SPHENER");
    assert!(d.units[sphener].free_promotions[hero]);
    for t in d.units.keys().filter(|&t| d.units[t].free_promotions[hero]) {
        assert!(!g.unit_type_is_army_unit(t), "{}", d.units[t].base.type_);
    }
}

#[test]
fn armies_survive_a_save() {
    let (mut g, _, b) = game();
    let p0 = PlayerId(0);
    let army = army_near(&mut g, b, 2);
    g.army_rename(p0, army, "The Iron Host").unwrap();
    g.armies[army].target_size = 6;
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).unwrap();
    assert_eq!(h.armies[army], g.armies[army]);
    assert_eq!(h.armies[army].target_size, 6);
    assert_eq!(h.army_units(army), g.army_units(army));
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
fn garrison_units_join_an_army_and_a_whole_army_merges() {
    let (mut g, a, _) = game();
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
    // A second army here merges into the first.
    let more: Vec<UnitId> = (0..2).map(|_| unit_at(&mut g, x, y)).collect();
    let other = g.army_form(p0, &more).unwrap();
    g.army_merge(p0, other, army).expect("merged");
    assert!(!g.armies.contains(other));
    assert_eq!(g.army_units(army).len(), 5);
    // Not on the army's plot: refused, nothing changes.
    let away = unit_at(&mut g, ox, oy);
    assert!(g.army_join(p0, &[away], army).is_err());
    assert_eq!(g.army_units(army).len(), 5);
}

/// An animal type is no army unit type (docs/SUPPLY.md, "Second pass" B): a civilization's Wolf
/// Pack outside a city is in no army, alone or grouped with a warrior that is.
#[test]
fn a_wolf_pack_is_in_no_army() {
    let (mut g, a, _) = game();
    let p0 = PlayerId(0);
    let wolves = g.d.t::<UnitType>("UNIT_WOLF_PACK");
    assert!(g.d.units[wolves].animal && !g.unit_type_is_army_unit(wolves));
    let (x, y) = next_to(&g, a);
    let pack = g.player_init_unit(p0, wolves, x, y, None, None);
    g.player_army_sync(p0);
    assert_eq!(g.units[pack].army, None, "alone");
    assert!(!g.unit_is_army_unit(pack));
    let w = unit_at(&mut g, x, y);
    let grp = g.units[w].group;
    g.unit_join_group(pack, grp, true);
    g.player_army_sync(p0);
    assert!(g.units[w].army.is_some(), "the warrior is an army");
    assert_eq!(g.units[pack].army, None, "beside the warrior");
}
