//! Supply lines (docs/SUPPLY.md): the line (sources, costs, what it cannot enter, the budget, the
//! tie rule) and Hunger and Starvation (the counter, no healing, the HP loss and
//! its floor, the damage dealt in battle, split, merge and form, save), on situations built by
//! hand on a flat map.

use ffh_core::army::{SupplySource, SupplyState};
use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::LogEvent;
use ffh_core::engagement::{PreviewInput, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians) whose map is one flat block of grassland (x 1..=38, y 2..=9) in an
/// ocean, revealed to both teams, the teams met, the starting units gone, player 0's turn active;
/// player 0 has a city on (5, 5) that owns only its own plot, and no plot has a route.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    city: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 40, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=9).contains(&y) && (1..=38).contains(&x);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.players[p0].turn_active = true;
    let city = g.player_init_city(p0, 5, 5, true, true);
    let mut w = World { g, p0, p1, city };
    w.clear();
    w
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    /// Every plot without a city is nobody's and has no route.
    fn clear(&mut self) {
        for q in self.g.map.plot_ids() {
            if self.g.map.plots[q].plot_city.is_none() {
                self.g.plot_set_owner(q, None, false, false);
                self.g.plot_set_route_type(q, None, false);
            }
        }
    }

    fn road(&mut self, xs: std::ops::RangeInclusive<i32>, y: i32) {
        let road = self.g.d.t::<RouteType>("ROUTE_ROAD");
        for x in xs {
            let q = self.at(x, y);
            self.g.plot_set_route_type(q, Some(road), false);
        }
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    /// An army of `n` of `p`'s warriors on (x, y).
    fn army(&mut self, p: PlayerId, x: i32, y: i32, n: usize) -> ArmyId {
        let units: Vec<UnitId> = (0..n).map(|_| self.unit(p, "UNIT_WARRIOR", x, y)).collect();
        self.g.army_form(p, &units).expect("an army")
    }

    fn war(&mut self) {
        let (t0, t1) = (self.g.players[self.p0].team, self.g.players[self.p1].team);
        self.g.team_declare_war(t0, t1, false, None);
    }

    fn cost(&self, x: i32, y: i32) -> Option<i32> {
        self.g.plot_supply_cost(self.p0, self.at(x, y))
    }

    fn logged(&self, text: &str) -> bool {
        self.g.log.entries.iter().any(|m| m.text.contains(text))
    }
}

#[test]
fn open_country_reaches_eight_plots() {
    let w = world();
    let d = &w.g.d.defines;
    assert_eq!((d.supply_line_budget, d.supply_cost_road, d.supply_cost_own, d.supply_cost_land, d.supply_hunger_turns), (24, 1, 1, 3, 2));
    assert_eq!((d.starvation_damage_percent, d.starvation_floor_percent, d.starvation_damage_dealt_percent), (10, 25, -25));
    // 7 plots of open country (3 each) and the city's plot (its owner's: 1).
    let line = w.g.plot_supply_line(w.p0, w.at(13, 5)).expect("supplied");
    assert_eq!(line.cost, 22);
    assert_eq!(line.path.len(), 9);
    assert_eq!((line.path[0], *line.path.last().unwrap()), (w.at(13, 5), w.at(5, 5)), "from the army to the source");
    assert_eq!(line.source, SupplySource::City(w.city));
    let sum: i32 = line.path[1..].iter().map(|&q| w.g.plot_supply_cost(w.p0, q).unwrap()).sum();
    assert_eq!(sum, line.cost, "the cost of the plots entered");
    // One plot further: 25, beyond the budget.
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(14, 5)), None, "cut");
}

#[test]
fn roads_and_own_territory_reach_three_times_as_far() {
    let mut w = world();
    // Anyone's road (on nobody's land): 22 road plots and the city.
    w.road(6..=27, 5);
    let line = w.g.plot_supply_line(w.p0, w.at(28, 5)).expect("supplied");
    assert_eq!(line.cost, 23);
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(30, 5)), None, "beyond the last road: 3 more each");
    // The owner's territory without a road costs as a road.
    w.clear();
    for x in 6..=20 {
        let q = w.at(x, 5);
        w.g.plot_set_owner(q, Some(w.p0), false, false);
    }
    assert_eq!(w.cost(10, 5), Some(1));
    assert_eq!(w.cost(10, 6), Some(3));
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(20, 5)).map(|l| l.cost), Some(15));
    // Another player's road costs as a road too (at war, its land may be entered).
    w.war();
    let q = w.at(25, 5);
    w.g.plot_set_owner(q, Some(w.p1), false, false);
    w.road(25..=25, 5);
    assert_eq!(w.cost(25, 5), Some(1));
}

#[test]
fn an_enemy_unit_on_the_road_forces_a_detour() {
    let mut w = world();
    w.war();
    w.road(6..=27, 5);
    w.unit(w.p1, "UNIT_WARRIOR", 12, 5);
    assert_eq!(w.cost(12, 5), None, "a unit at war with the owner stands there");
    // Around it, over a plot of open country: 2 more.
    let line = w.g.plot_supply_line(w.p0, w.at(20, 5)).expect("supplied");
    assert_eq!(line.cost, 17);
    assert!(!line.path.contains(&w.at(12, 5)));
    let (north, south) = (w.at(12, 4), w.at(12, 6));
    assert!(line.path.contains(&north.min(south)), "a tie: the lower plot index");
    // From the end of the road the detour does not fit any more.
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(28, 5)), None);
}

#[test]
fn what_a_line_cannot_enter() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    // Water.
    assert_eq!(w.cost(10, 1), None);
    // Another team's territory at peace, without open borders; open once at war.
    let q = w.at(20, 5);
    w.g.plot_set_owner(q, Some(p1), false, false);
    assert_eq!(w.cost(20, 5), None);
    w.war();
    assert_eq!(w.cost(20, 5), Some(3));
    // A city of a team at war.
    w.g.player_init_city(p1, 30, 5, true, true);
    w.clear();
    assert_eq!(w.cost(30, 5), None);
    // A plot next to a fort an enemy unit holds (and the fort itself); a fort nobody holds
    // denies nothing.
    let fort = w.g.d.t::<ImprovementType>("IMPROVEMENT_FORT");
    let f = w.at(8, 7);
    w.g.plot_set_improvement_type(f, Some(fort));
    assert_eq!(w.cost(8, 6), Some(3));
    w.unit(p1, "UNIT_WARRIOR", 8, 7);
    for (x, y) in [(7, 6), (8, 6), (9, 6), (7, 8), (9, 7), (8, 7)] {
        assert_eq!(w.cost(x, y), None, "({x}, {y})");
    }
    assert_eq!(w.cost(8, 5), Some(3), "two plots away");
    // What the fort upgrades to counts as a fort.
    let castle = w.g.d.improvements[fort].improvement_upgrade.expect("a fort upgrades");
    w.g.plot_set_improvement_type(f, Some(castle));
    assert_eq!(w.cost(7, 6), None);
    // The army's own plot is always allowed: an army next to the manned fort traces from it.
    let a = w.army(p0, 7, 6, 1);
    let line = w.g.army_supply_line(a).expect("supplied");
    assert_eq!((line.path[0], line.cost), (w.at(7, 6), 4));
}

#[test]
fn in_its_city_an_army_is_supplied_and_an_occupied_city_is_no_source() {
    let mut w = world();
    let p0 = w.p0;
    w.g.cities[w.city].occupation_timer = 3;
    let here = w.g.plot_supply_line(p0, w.at(5, 5)).expect("in its city");
    assert_eq!((here.cost, here.path.len(), here.source), (0, 1, SupplySource::City(w.city)));
    assert_eq!(w.g.plot_supply_line(p0, w.at(6, 5)), None, "the only city is in occupation");
    w.g.cities[w.city].occupation_timer = 0;
    assert_eq!(w.g.plot_supply_line(p0, w.at(6, 5)).map(|l| l.cost), Some(1));
}

#[test]
fn a_ship_in_a_home_water_area_is_a_source() {
    let mut w = world();
    let p0 = w.p0;
    let d = w.g.d;
    let transport = d
        .units
        .keys()
        .find(|&t| {
            let u = &d.units[t];
            u.domain_type == Some(DomainType::DOMAIN_SEA) && u.cargo_space > 0 && u.domain_cargo == Some(DomainType::DOMAIN_LAND) && u.special_cargo.is_none()
        })
        .expect("a ship for land units");
    // No coastal city: a ship is no source.
    let ship = w.g.player_init_unit(p0, transport, 30, 1, None, None);
    assert_eq!(w.g.plot_supply_line(p0, w.at(30, 3)), None);
    // A city on that coast beyond the ship's range: no lane (docs/NAVAL.md, 4.6), no source.
    let far = w.g.player_init_city(p0, 20, 2, true, true);
    w.clear();
    assert_eq!(w.g.plot_supply_line(p0, w.at(30, 3)), None, "no lane in range");
    w.g.city_kill(far, false);
    // With a city on the coast in range: the line ends on a land plot next to the ship, the
    // lowest plot index of the three at the same cost and length.
    w.g.player_init_city(p0, 25, 2, true, true);
    w.clear();
    let line = w.g.plot_supply_line(p0, w.at(30, 3)).expect("supplied by the ship");
    assert_eq!(line.source, SupplySource::Ship(ship));
    assert_eq!(line.cost, 3);
    let ends = [w.at(29, 2), w.at(30, 2), w.at(31, 2)];
    assert_eq!(line.path, vec![w.at(30, 3), *ends.iter().min().unwrap()]);
    // An army aboard it is supplied by it.
    let aboard = w.g.plot_supply_line(p0, w.at(30, 1)).expect("aboard");
    assert_eq!((aboard.cost, aboard.source), (0, SupplySource::Ship(ship)));
}

#[test]
fn ties_the_fewer_plots_then_the_lower_index() {
    let mut w = world();
    let p0 = w.p0;
    // Two plots from the city: three paths of one plot of open country and the city (4).
    let line = w.g.plot_supply_line(p0, w.at(7, 5)).expect("supplied");
    assert_eq!(line.cost, 4);
    let middle = [w.at(6, 4), w.at(6, 5), w.at(6, 6)];
    assert_eq!(line.path, vec![w.at(7, 5), *middle.iter().min().unwrap(), w.at(5, 5)]);
    // A road around costs as much over more plots: the fewer plots win.
    let road = w.g.d.t::<RouteType>("ROUTE_ROAD");
    for (x, y) in [(7, 4), (6, 3), (5, 4)] {
        let q = w.at(x, y);
        w.g.plot_set_route_type(q, Some(road), false);
    }
    let line = w.g.plot_supply_line(p0, w.at(7, 5)).expect("supplied");
    assert_eq!((line.cost, line.path.len()), (4, 3));
}

#[test]
fn the_counter_hunger_starvation_and_supplied_again() {
    let mut w = world();
    let p0 = w.p0;
    let a = w.army(p0, 20, 5, 2);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Supplied, "a new army");
    assert!(w.g.army_supply_line(a).is_none(), "but its line is cut");
    w.g.player_update_supply(p0);
    assert_eq!(w.g.armies[a].unsupplied_turns, 1);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Hunger { turn: 1, of: 2 });
    assert_eq!(w.g.army_supply_state(a).turns_to_starvation(), Some(2));
    assert!(!w.g.army_in_supply(a));
    assert!(w.logged("The 1st Army has no supply line"));
    w.g.player_update_supply(p0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Hunger { turn: 2, of: 2 });
    w.g.player_update_supply(p0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Starvation);
    assert!(w.logged("The 1st Army is starving"));
    // Another player's turn changes nothing.
    w.g.player_update_supply(w.p1);
    assert_eq!(w.g.armies[a].unsupplied_turns, 3);
    // A road home restores it at once at the next turn start; nothing lingers.
    w.road(6..=19, 5);
    w.g.player_update_supply(p0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Supplied);
    assert!(w.g.army_in_supply(a));
    assert!(w.logged("The 1st Army is supplied again"));
}

#[test]
fn within_its_borders_an_army_needs_no_line() {
    let mut w = world();
    let p0 = w.p0;
    let a = w.army(p0, 20, 5, 2);
    let here = w.at(20, 5);
    assert!(!w.g.army_supply_home(a));
    w.g.player_update_supply(p0);
    assert_eq!(w.g.armies[a].unsupplied_turns, 1, "outside its borders, with no line: Hunger");
    // The plot becomes its owner's (a pocket far from any city): no line, yet supplied.
    w.g.plot_set_owner(here, Some(p0), false, false);
    assert!(w.g.plot_supply_home(p0, here) && w.g.army_supply_home(a));
    assert!(w.g.army_supply_line(a).is_none(), "the line itself is still cut");
    assert!(w.g.plot_is_supplied(p0, here));
    w.g.player_update_supply(p0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Supplied);
    assert!(w.logged("The 1st Army is supplied again"));
    // Encircled by enemy units on its own land: still supplied.
    w.war();
    for (x, y) in [(19, 4), (20, 4), (21, 4), (19, 5), (21, 5), (19, 6), (20, 6), (21, 6)] {
        w.unit(w.p1, "UNIT_WARRIOR", x, y);
    }
    w.g.player_update_supply(p0);
    assert_eq!(w.g.armies[a].unsupplied_turns, 0);
    // Another player's territory is not home.
    w.g.plot_set_owner(here, Some(w.p1), false, false);
    assert!(!w.g.plot_supply_home(p0, here));
    w.g.player_update_supply(p0);
    assert_eq!(w.g.armies[a].unsupplied_turns, 1);
}

#[test]
fn a_player_without_a_city_has_no_supply_rule() {
    let mut w = world();
    let p1 = w.p1;
    assert!(w.g.players[p1].cities.is_empty());
    // The starting units of a player who has not founded its capital yet, far from any city.
    let a = w.army(p1, 30, 5, 2);
    assert!(w.g.army_supply_line(a).is_none(), "no line: there is no source");
    w.g.player_update_supply(p1);
    w.g.player_update_supply(p1);
    w.g.player_update_supply(p1);
    assert_eq!(w.g.armies[a].unsupplied_turns, 0, "no Hunger before the first city");
    assert_eq!(w.g.army_supply_state(a), SupplyState::Supplied);
    assert!(!w.logged("has no supply line"));
    // Its first city brings the rule: the army out of reach of it is cut from then on.
    w.g.player_init_city(p1, 36, 8, true, true);
    w.g.player_update_supply(p1);
    assert_eq!(w.g.armies[a].unsupplied_turns, 0, "within reach of the new city");
    let b = w.army(p1, 15, 5, 2);
    w.g.player_update_supply(p1);
    assert_eq!(w.g.armies[b].unsupplied_turns, 1, "out of reach of it");
}

#[test]
fn a_hungry_army_does_not_heal() {
    let mut w = world();
    let p0 = w.p0;
    let cut = w.army(p0, 20, 5, 2);
    let fed = w.army(p0, 7, 5, 2);
    let units: Vec<UnitId> = w.g.army_units(cut).into_iter().chain(w.g.army_units(fed)).collect();
    for &u in &units {
        w.g.units[u].damage = 40;
    }
    // The start of its owner's turn: the lines are traced before the units heal.
    w.g.player_do_turn_units(p0);
    assert_eq!(w.g.armies[cut].unsupplied_turns, 1);
    assert!(w.g.army_units(cut).iter().all(|&u| w.g.units[u].damage == 40), "no healing");
    assert!(w.g.army_units(fed).iter().all(|&u| w.g.units[u].damage < 40), "the supplied army heals");
}

#[test]
fn starvation_takes_hp_to_a_floor_and_never_kills() {
    let mut w = world();
    let p0 = w.p0;
    let a = w.army(p0, 20, 5, 3);
    let units = w.g.army_units(a);
    w.g.units[units[1]].damage = 70;
    w.g.units[units[2]].damage = 80;
    w.g.armies[a].unsupplied_turns = 2;
    w.g.player_update_supply(p0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Starvation);
    let damage = |w: &World| units.iter().map(|&u| w.g.units[u].damage).collect::<Vec<_>>();
    // 10% of full HP; down to 25% of it; a unit already below loses nothing.
    assert_eq!(damage(&w), [10, 75, 80]);
    for _ in 0..12 {
        w.g.player_update_supply(p0);
    }
    assert_eq!(damage(&w), [75, 75, 80]);
    assert!(units.iter().all(|&u| w.g.units.contains(u)), "never kills");
}

#[test]
fn a_starving_army_deals_less_damage() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    w.war();
    let a = w.army(p0, 20, 5, 3);
    let defenders: Vec<UnitId> = (0..2).map(|_| w.unit(p1, "UNIT_WARRIOR", 21, 5)).collect();
    w.g.player_army_sync(p1);
    let inp = PreviewInput { player: p0, from: w.at(20, 5), to: w.at(21, 5), squad: None, war: WarTest::AtWar };
    let fed = w.g.engage_preview(&inp).expect("a preview");
    assert!(fed.starving.is_empty());
    w.g.armies[a].unsupplied_turns = 3;
    let starving = w.g.engage_preview(&inp).expect("a preview");
    let mut listed: Vec<UnitId> = starving.starving.iter().map(|x| x.0).collect();
    listed.sort();
    let mut mine: Vec<UnitId> = starving.formation.iter().map(|p| p.unit).collect();
    mine.sort();
    assert_eq!(listed, mine, "the starving army's units on the field");
    assert!(starving.starving.iter().all(|x| x.1 == -25));
    assert!(starving.forecast.lost_x100[1] <= fed.forecast.lost_x100[1], "the defenders lose no more");
    // The battle: its log has the line.
    w.g.players[p1].human = true;
    let grp = w.g.armies[a].group.expect("a group");
    let before = w.g.battle_report_next;
    w.g.group_attack_force(grp, 21, 5, MOVE_DIRECT_ATTACK, None).expect("the attack");
    let r = w.g.battle_reports.iter().find(|r| r.id >= before).expect("a battle");
    let log = r.log.as_ref().expect("a log");
    let line = log.events.iter().find_map(|e| match e {
        LogEvent::Starvation { units, dealt_pct } => Some((units.clone(), *dealt_pct)),
        _ => None,
    });
    let (units, pct) = line.expect("a Starvation line");
    assert_eq!(pct, -25);
    assert!(units.iter().all(|&i| defenders.iter().all(|&d| log.fighters[i as usize].unit != d)), "the attackers only");
    assert_eq!(units.len(), mine.len());
}

#[test]
fn split_merge_and_form_from_a_garrison() {
    let mut w = world();
    let p0 = w.p0;
    let a = w.army(p0, 6, 5, 4);
    w.g.armies[a].unsupplied_turns = 2;
    let units = w.g.army_units(a);
    // A split takes its army's counter.
    let b = w.g.army_split(p0, &units[..2]).unwrap();
    assert_eq!((w.g.armies[a].unsupplied_turns, w.g.armies[b].unsupplied_turns), (2, 2));
    // A merge takes the higher counter.
    let c = w.army(p0, 6, 5, 1);
    w.g.armies[b].unsupplied_turns = 3;
    w.g.army_merge(p0, b, c).unwrap();
    assert_eq!(w.g.armies[c].unsupplied_turns, 3);
    w.g.army_merge(p0, c, a).unwrap();
    assert_eq!(w.g.armies[a].unsupplied_turns, 3);
    // A unit that leaves for a garrison is out of it at once; an army formed from the garrison
    // starts supplied.
    let g0 = w.unit(p0, "UNIT_WARRIOR", 5, 5);
    w.g.player_army_sync(p0);
    assert_eq!(w.g.unit_garrison_city(g0), Some(w.city));
    assert!(!w.g.unit_is_unsupplied(g0));
    let formed = w.g.army_form(p0, &[g0]).unwrap();
    assert_eq!(w.g.armies[formed].unsupplied_turns, 0);
    let one = w.g.army_units(a)[0];
    assert!(w.g.unit_is_unsupplied(one));
    w.g.unit_set_xy(one, 5, 5, false);
    w.g.army_to_garrison(p0, &[one]).unwrap();
    assert!(!w.g.unit_is_unsupplied(one), "in the garrison");
}

#[test]
fn the_counter_survives_a_save() {
    let mut w = world();
    let a = w.army(w.p0, 20, 5, 2);
    w.g.armies[a].unsupplied_turns = 3;
    let bytes = w.g.save();
    let h = Game::load(w.g.d, &bytes).unwrap();
    assert_eq!(h.armies[a].unsupplied_turns, 3);
    assert_eq!(h.save(), bytes);
}

// ---- The known line (docs/SUPPLY.md, "Second pass", A) ----

#[test]
fn an_unseen_blocker_bends_the_true_line_not_the_known_one() {
    let mut w = world();
    let (p0, t0) = (w.p0, w.g.players[w.p0].team);
    w.war();
    w.road(6..=27, 5);
    let enemy = w.unit(w.p1, "UNIT_WARRIOR", 12, 5);
    let (blocked, from) = (w.at(12, 5), w.at(20, 5));
    assert!(!w.g.plot_is_visible(blocked, t0), "player 0 does not see (12, 5)");
    let truth = w.g.plot_supply_line(p0, from).expect("supplied");
    assert_eq!(truth.cost, 17, "the true line goes around the enemy");
    let known = w.g.plot_supply_line_known(p0, from, t0).expect("supplied as known");
    assert_eq!(known.cost, 15, "the known line keeps to the road");
    assert!(known.path.contains(&blocked));
    // From the end of the road: cut in truth, supplied as far as player 0 knows.
    assert_eq!(w.g.plot_supply_line(p0, w.at(28, 5)), None);
    assert!(w.g.plot_supply_line_known(p0, w.at(28, 5), t0).is_some());
    // Seen now: it bends both.
    w.g.map.plots[blocked].visibility_count[t0] += 1;
    assert!(w.g.plot_is_visible(blocked, t0) && !w.g.unit_is_invisible(enemy, t0, true));
    assert_eq!(w.g.plot_supply_line_known(p0, from, t0), Some(truth));
    assert_eq!(w.g.plot_supply_line_known(p0, w.at(28, 5), t0), None);
}

#[test]
fn a_fort_denies_its_neighbours_only_when_its_unit_is_seen() {
    let mut w = world();
    let (p0, t0) = (w.p0, w.g.players[w.p0].team);
    w.war();
    w.road(6..=27, 5);
    let fort = w.g.d.t::<ImprovementType>("IMPROVEMENT_FORT");
    let f = w.at(12, 6);
    w.g.plot_set_improvement_type(f, Some(fort));
    w.unit(w.p1, "UNIT_WARRIOR", 12, 6);
    let from = w.at(20, 5);
    // The road beside the fort is denied in truth (around it to the north: 21), not to a viewer
    // that does not see the fort's unit (straight down the road: 15).
    assert_eq!(w.g.plot_supply_line(p0, from).map(|l| l.cost), Some(21));
    assert_eq!(w.g.plot_supply_line_known(p0, from, t0).map(|l| l.cost), Some(15));
    // Seen: the same as the truth.
    w.g.map.plots[f].visibility_count[t0] += 1;
    assert_eq!(w.g.plot_supply_line_known(p0, from, t0), w.g.plot_supply_line(p0, from));
}

#[test]
fn the_known_line_enters_only_revealed_plots() {
    let mut w = world();
    let (p0, t0) = (w.p0, w.g.players[w.p0].team);
    w.road(6..=27, 5);
    let from = w.at(20, 5);
    let truth = w.g.plot_supply_line(p0, from).expect("supplied");
    assert_eq!(truth.cost, 15);
    // A column of unrevealed plots across the land at x = 12: the known line is cut, the true one
    // is not.
    for y in 2..=9 {
        let q = w.at(12, y);
        w.g.plot_set_revealed(q, t0, false, false, None, false);
    }
    assert_eq!(w.g.plot_supply_line_known(p0, from, t0), None);
    assert_eq!(w.g.plot_supply_line(p0, from), Some(truth));
    // One revealed plot off the road: the known line goes through it.
    let q = w.at(12, 6);
    w.g.plot_set_revealed(q, t0, true, false, None, false);
    let known = w.g.plot_supply_line_known(p0, from, t0).expect("through (12, 6)");
    assert!(known.path.contains(&q));
    assert_eq!(known.cost, 17);
}

#[test]
fn an_unrevealed_city_is_no_source_and_the_own_team_knows_the_truth() {
    let mut w = world();
    let (p0, t0, t1) = (w.p0, w.g.players[w.p0].team, w.g.players[w.p1].team);
    let from = w.at(10, 5);
    let truth = w.g.plot_supply_line(p0, from).expect("supplied");
    // Its own team: every plot revealed, no enemy anywhere: the same line.
    assert_eq!(w.g.plot_supply_line_known(p0, from, t0), Some(truth.clone()));
    // Another team that has revealed player 0's city works out the same line; one that has not
    // finds no source.
    w.g.cities[w.city].revealed[t1] = true;
    assert_eq!(w.g.plot_supply_line_known(p0, from, t1), Some(truth.clone()));
    w.g.cities[w.city].revealed[t1] = false;
    assert_eq!(w.g.plot_supply_line_known(p0, from, t1), None);
    assert_eq!(w.g.plot_supply_line_known(p0, w.at(5, 5), t1), None, "not even standing in it");
    assert_eq!(w.g.plot_supply_line(p0, from), Some(truth));
}

#[test]
fn investment_requires_three_and_half_the_physical_garrison_without_animals() {
    let mut w = world();
    w.war();
    let a = w.army(w.p0, 5, 5, 7);
    assert_eq!(w.g.city_investment_garrison(w.city).len(), 7, "stationed armies count as defenders");
    let enemies: Vec<_> = (0..3).map(|_| w.unit(w.p1, "UNIT_WARRIOR", 6, 5)).collect();
    for _ in 0..3 { w.unit(w.p1, "UNIT_WOLF", 5, 4); }
    assert!(w.g.city_is_besieged(w.city), "the original one-enemy repair/healing test remains");
    assert_eq!(w.g.city_investment_besiegers(w.city).len(), 3);
    assert_eq!(w.g.city_investment_required(w.city), 4, "half of seven rounds up");
    assert!(!w.g.city_is_invested(w.city));
    let fourth = w.unit(w.p1, "UNIT_WARRIOR", 6, 5);
    assert!(w.g.city_is_invested(w.city));
    assert!(!w.g.city_invested_after_move(w.city, &[fourth], w.at(8, 5)));
    assert!(w.g.city_invested_after_move(w.city, &[enemies[0]], w.at(6, 6)));
    assert_eq!(w.g.army_units(a).len(), 7);
}

#[test]
fn siege_hunger_blocks_every_heal_then_starves_once_with_army_hunger_and_never_kills() {
    let mut w = world();
    w.war();
    let a = w.army(w.p0, 5, 5, 2);
    let warrior = w.g.army_units(a)[0];
    let worker = w.unit(w.p0, "UNIT_SETTLER", 5, 5);
    let hero = w.unit(w.p0, "UNIT_ADEPT", 5, 5);
    for _ in 0..3 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    w.g.unit_set_damage(warrior, 20, None);
    for turn in 1..=2 {
        w.g.player_update_supply(w.p0);
        assert_eq!(w.g.cities[w.city].siege_turns, turn);
        assert!(w.g.unit_is_unsupplied(worker), "civilian units are hungry too");
        assert_eq!(w.g.unit_heal_rate(warrior, w.at(5, 5)), 0);
        w.g.unit_set_damage(warrior, 0, None);
        w.g.unit_change_damage(warrior, -10, None);
        assert_eq!(w.g.units[warrior].damage, 20, "spells and other direct heals cannot bypass Hunger");
        assert_eq!(w.g.units[worker].damage, 0, "Hunger itself does no damage");
    }
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.city_siege_state(w.city), SupplyState::Starvation);
    // An invested city is not home territory: the army needs a line, and has none.
    assert!(w.g.armies[a].unsupplied_turns > w.g.d.defines.supply_hunger_turns);
    assert_eq!(w.g.units[warrior].damage, 30, "army and city starvation cause only one HP loss");
    for u in [worker, hero] {
        assert!(w.g.unit_is_starving(u));
        assert_eq!(w.g.units[u].damage, w.g.unit_max_hit_points(u) / 10);
    }
    for _ in 0..20 { w.g.player_update_supply(w.p0); }
    let max = w.g.unit_max_hit_points(warrior);
    assert_eq!(w.g.units[warrior].damage, max - (max * 25 + 99) / 100);
    w.g.unit_set_damage(warrior, max - 1, None);
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.units[warrior].damage, max - 1, "a unit already below the floor stays there");
}

#[test]
fn siege_counter_pauses_then_resets_after_two_consecutive_relieved_turns_and_is_saved() {
    let mut w = world();
    w.war();
    w.unit(w.p0, "UNIT_WARRIOR", 5, 5);
    let enemies: Vec<_> = (0..3).map(|_| w.unit(w.p1, "UNIT_WARRIOR", 6, 5)).collect();
    for _ in 0..3 { w.g.player_update_supply(w.p0); }
    w.g.unit_kill(enemies[0], false, None);
    w.g.player_update_supply(w.p0);
    assert_eq!((w.g.cities[w.city].siege_turns, w.g.cities[w.city].siege_relief_turns), (3, 1));
    let saved = w.g.save();
    w.g = Game::load(w.g.d, &saved).unwrap();
    assert_eq!(w.g.save(), saved);
    w.unit(w.p1, "UNIT_WARRIOR", 6, 5);
    w.g.player_update_supply(w.p0);
    assert_eq!((w.g.cities[w.city].siege_turns, w.g.cities[w.city].siege_relief_turns), (4, 0), "renewed investment cancels the relief streak");
    w.g.unit_kill(enemies[1], false, None);
    w.g.player_update_supply(w.p0);
    w.g.player_update_supply(w.p0);
    assert_eq!((w.g.cities[w.city].siege_turns, w.g.cities[w.city].siege_relief_turns), (0, 0));
}

#[test]
fn invested_cities_are_no_supply_source_but_another_city_can_still_supply_the_army() {
    let mut w = world();
    w.war();
    w.unit(w.p1, "UNIT_WARRIOR", 6, 5);
    let at = w.at(5, 5);
    assert_eq!(w.g.plot_supply_line(w.p0, at).unwrap().source, SupplySource::City(w.city));
    for _ in 0..2 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    assert_eq!(w.g.plot_supply_line(w.p0, at), None, "an army standing in the invested city has no exemption");
    let relief = w.g.player_init_city(w.p0, 10, 5, true, true);
    assert_eq!(w.g.plot_supply_line(w.p0, at).unwrap().source, SupplySource::City(relief));
}

#[test]
fn an_open_harbour_prevents_siege_hunger_and_keeps_the_city_a_supply_source() {
    let mut w = world();
    let coast = w.g.d.t("TERRAIN_COAST");
    for y in 2..=4 {
        let water = w.at(5, y);
        w.g.plot_set_plot_type(water, PlotType::PLOT_OCEAN, false);
        w.g.plot_set_terrain_type(water, coast, false);
    }
    w.g.map_recalculate_areas();
    w.g.map_generate_seas();
    w.war();
    for _ in 0..3 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    assert!(w.g.city_is_invested(w.city));
    assert_eq!(w.g.city_harbour(w.city), ffh_core::seas::Harbour::Open);
    for _ in 0..4 { w.g.player_update_supply(w.p0); }
    assert_eq!(w.g.cities[w.city].siege_turns, 0);
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(5, 5)).unwrap().source, SupplySource::City(w.city));
    let boats = w.at(5, 2);
    assert_eq!(w.g.plot_supply_line(w.p0, boats).unwrap().source, SupplySource::Sea(w.city));
    let ship = w.unit(w.p1, "UNIT_FRIGATE", 5, 4);
    assert!(w.g.unit_plunder(ship));
    assert!(matches!(w.g.city_harbour(w.city), ffh_core::seas::Harbour::Blockaded(_)));
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.cities[w.city].siege_turns, 1, "blockade removes the harbour's relief");
    assert_eq!(w.g.plot_supply_line(w.p0, w.at(5, 5)), None);
    assert_eq!(w.g.plot_supply_line(w.p0, boats), None, "boats cannot use the invested port's automatic supply");
    let other = w.g.player_init_city(w.p0, 9, 2, true, true);
    let team = w.g.players[w.p0].team;
    assert_eq!(w.g.plot_sea_port(team, boats), Some(w.city), "the operating-range field is unchanged");
    assert_eq!(w.g.plot_supply_line(w.p0, boats).unwrap().source, SupplySource::Sea(other), "another eligible port within range can feed the army");
}

#[test]
fn a_starving_garrison_carries_its_battle_penalty_in_a_sally_and_sentinels_are_exempt() {
    let mut w = world();
    let wall = w.g.d.t("BUILDING_PALISADE");
    w.g.city_set_num_real_building(w.city, wall, 1);
    w.g.city_sync_sentinels(w.city);
    let sentinels = w.g.cities[w.city].sentinel_damage.clone();
    assert!(!sentinels.is_empty());
    let defenders: Vec<_> = (0..2).map(|_| w.unit(w.p0, "UNIT_WARRIOR", 5, 5)).collect();
    w.war();
    for _ in 0..3 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    for _ in 0..3 { w.g.player_update_supply(w.p0); }
    assert_eq!(w.g.cities[w.city].sentinel_damage, sentinels, "Starvation does not damage Sentinels");
    let inp = PreviewInput { player: w.p0, from: w.at(5, 5), to: w.at(6, 5), squad: None, war: WarTest::AtWar };
    let preview = w.g.engage_preview(&inp).expect("sally forecast");
    assert!(!preview.starving.is_empty());
    assert!(preview.starving.iter().all(|&(u, pct)| defenders.contains(&u) && pct == -25));
    let inp = PreviewInput { player: w.p1, from: w.at(6, 5), to: w.at(5, 5), squad: None, war: WarTest::AtWar };
    let preview = w.g.engage_preview(&inp).expect("assault forecast");
    assert!(preview.starving.iter().all(|&(u, pct)| defenders.contains(&u) && pct == -25), "only physical garrison units receive the penalty");
}

#[test]
fn concealed_soldiers_do_not_invest_and_an_open_harbour_relief_still_takes_two_turns() {
    let mut w = world();
    w.war();
    let enemies: Vec<_> = (0..3).map(|_| w.unit(w.p1, "UNIT_WARRIOR", 6, 5)).collect();
    let team = w.g.players[w.p0].team;
    w.g.units[enemies[2]].invisible_type = Some(w.g.d.t("INVISIBLE_LAND"));
    assert!(w.g.unit_is_invisible(enemies[2], team, false));
    assert!(!w.g.city_is_invested(w.city));
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.cities[w.city].siege_turns, 0);
    w.g.units[enemies[2]].invisible_type = None;
    for _ in 0..3 { w.g.player_update_supply(w.p0); }
    assert_eq!(w.g.cities[w.city].siege_turns, 3);
    let coast = w.g.d.t("TERRAIN_COAST");
    for y in 2..=4 {
        let q = w.at(5, y);
        w.g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, false);
        w.g.plot_set_terrain_type(q, coast, false);
    }
    w.g.map_recalculate_areas();
    w.g.map_generate_seas();
    assert_eq!(w.g.city_harbour(w.city), ffh_core::seas::Harbour::Open);
    assert!(w.g.city_is_invested(w.city));
    w.g.player_update_supply(w.p0);
    assert_eq!((w.g.cities[w.city].siege_turns, w.g.cities[w.city].siege_relief_turns), (3, 1));
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.cities[w.city].siege_turns, 0);
}

#[test]
fn bombard_and_starvation_floors_stack_without_healing_the_garrison() {
    let mut w = world();
    w.war();
    let defender = w.unit(w.p0, "UNIT_WARRIOR", 5, 5);
    let bomber = w.unit(w.p1, "UNIT_CATAPULT", 6, 5);
    for _ in 0..2 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    w.g.city_bombard_damage(w.city, bomber, 100);
    assert_eq!(w.g.units[defender].damage, 50);
    for _ in 0..3 { w.g.player_update_supply(w.p0); }
    assert_eq!(w.g.units[defender].damage, 60);
    w.g.city_bombard_damage(w.city, bomber, 100);
    assert_eq!(w.g.units[defender].damage, 60);
    for _ in 0..3 { w.g.player_update_supply(w.p0); }
    assert_eq!(w.g.units[defender].damage, 75);
}

#[test]
fn an_army_held_in_an_invested_city_leaves_only_to_attack() {
    let mut w = world();
    w.war();
    let a = w.army(w.p0, 5, 5, 2);
    let grp = w.g.units[w.g.army_units(a)[0]].group.expect("a group");
    for _ in 0..3 { w.unit(w.p1, "UNIT_WARRIOR", 6, 5); }
    assert!(w.g.city_is_invested(w.city));
    assert_eq!(w.g.group_siege_hold(grp), Some(w.city));
    assert!(!w.g.plot_supply_home(w.p0, w.at(5, 5)), "an invested city is not home territory");
    let city = w.at(5, 5);
    assert!(w.g.group_find_path(grp, city, w.at(4, 5), 0).is_none(), "no step out of the city");
    assert!(w.g.group_find_path(grp, city, w.at(3, 5), 0).is_none());
    assert!(w.g.group_find_path(grp, city, w.at(6, 5), 0).is_some(), "an attack on the besiegers: a sally");
    // A unit in no army (a garrison) is not held.
    let lone = w.unit(w.p0, "UNIT_WARRIOR", 5, 5);
    assert_eq!(w.g.unit_siege_hold(lone), None);
    // A move order out of the city: it stays.
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 4, 5, 0, false, true, None, None, None);
    assert_eq!(w.g.army_plot(a), Some(city), "held: no move out");
    // A move order onto the besiegers: a battle (a sally).
    w.g.players[w.p1].human = true;
    let before = w.g.battle_report_next;
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 6, 5, MOVE_DIRECT_ATTACK, false, true, None, None, None);
    assert!(w.g.battle_reports.iter().any(|r| r.id >= before), "the sally is fought");
}

#[test]
fn the_hold_ends_when_the_siege_lifts() {
    let mut w = world();
    w.war();
    let a = w.army(w.p0, 5, 5, 2);
    let grp = w.g.units[w.g.army_units(a)[0]].group.expect("a group");
    let besiegers: Vec<UnitId> = (0..3).map(|_| w.unit(w.p1, "UNIT_WARRIOR", 6, 5)).collect();
    assert_eq!(w.g.group_siege_hold(grp), Some(w.city));
    w.g.unit_kill(besiegers[0], false, None);
    assert!(!w.g.city_is_invested(w.city), "two besiegers do not invest");
    assert_eq!(w.g.group_siege_hold(grp), None);
    assert!(w.g.plot_supply_home(w.p0, w.at(5, 5)));
    assert!(w.g.group_find_path(grp, w.at(5, 5), w.at(4, 5), 0).is_some());
}
