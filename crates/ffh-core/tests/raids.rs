//! Raids (docs/RAIDS.md): the order's refusals and endings, the take and its modifier, the
//! guard's sallies under each posture (in the fields, without moving or paying), driven off,
//! stripped and recovered, raiders that neither besiege nor invest and are supplied, fields that
//! are not pillaged, saves; on situations built by hand on a flat map.

use ffh_core::army::{Station, SupplyState};
use ffh_core::engagement::{HitCtx, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::raid::{RaidState, Sally};
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel game (no barbarians) on one flat block of grassland (x 1..=38, y 2..=9) in an ocean,
/// revealed to both teams, the teams met and at war, the starting units gone. Player 1 has a city
/// on (10, 5) whose countryside is every plot within 2 of it (its owner's, worked by it or not);
/// player 0 has a city on (35, 5) owning only its plot. No plot has a route.
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
    g.player_init_city(p0, 35, 5, true, true);
    let city = g.player_init_city(p1, 10, 5, true, true);
    for q in g.map.plot_ids() {
        if g.map.plots[q].plot_city.is_none() {
            g.plot_set_owner(q, None, false, false);
            g.plot_set_route_type(q, None, false);
        }
    }
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if (x - 10).abs() <= 2 && (y - 5).abs() <= 2 && g.map.plots[q].plot_city.is_none() {
            g.plot_set_owner(q, Some(p1), false, false);
        }
    }
    for q in g.map.plot_ids() {
        g.plot_update_working_city(q);
    }
    g.team_declare_war(t0, t1, false, None);
    // Some production and commerce to take.
    g.city_change_base_yield_rate(city, YieldType::YIELD_PRODUCTION, 10);
    g.city_change_base_yield_rate(city, YieldType::YIELD_COMMERCE, 10);
    World { g, p0, p1, city }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    /// A one-unit army of `p`'s unit of `name` on (x, y).
    fn army(&mut self, p: PlayerId, name: &str, x: i32, y: i32) -> ArmyId {
        let u = self.unit(p, name, x, y);
        self.g.army_form(p, &[u]).expect("an army")
    }

    /// `n` of player 1's units of `name` in its city's garrison.
    fn guard(&mut self, name: &str, n: usize) -> Vec<UnitId> {
        let us: Vec<UnitId> = (0..n).map(|_| self.unit(self.p1, name, 10, 5)).collect();
        self.g.player_army_sync(self.p1);
        us
    }

    fn raid(&mut self, a: ArmyId) {
        let c = self.city;
        self.g.army_set_station(self.p0, a, Station::Raid(c)).expect("the raid order");
    }

    fn reports_since(&self, before: u32) -> Vec<ffh_core::engagement::report::BattleReport> {
        self.g.battle_reports.iter().filter(|r| r.id >= before).cloned().collect()
    }

    fn logged(&self, text: &str) -> bool {
        self.g.log.entries.iter().any(|m| m.text.contains(text))
    }

    fn rate(&self, y: YieldType) -> i32 {
        self.g.city_yield_rate(self.city, y)
    }
}

#[test]
fn the_defines_are_the_designs() {
    let w = world();
    let d = &w.g.d.defines;
    assert_eq!((d.raid_take_percent, d.raid_contested_percent, d.raid_strip_turns, d.raid_recover_turns, d.raid_sally_odds, d.raid_sally_forecast_runs), (50, 10, 5, 15, 600, 12));
}

#[test]
fn the_countryside_is_the_citys_land_but_its_plot() {
    let w = world();
    let land = w.g.city_countryside(w.city);
    assert!(land.contains(&w.at(12, 5)) && land.contains(&w.at(11, 5)));
    assert!(!land.contains(&w.at(10, 5)), "the city's own plot");
    assert!(!land.contains(&w.at(14, 5)), "beyond its land");
    assert!(land.iter().all(|&q| w.g.map.plots[q].working_city == Some(w.city)));
}

#[test]
fn the_order_is_refused_and_taken() {
    let mut w = world();
    let c = w.city;
    let far = w.army(w.p0, "UNIT_WARRIOR", 20, 5);
    assert_eq!(w.g.army_raid_refusal(far, c), Some("The army is not on the city's countryside."));
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    // No war: refused.
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_make_peace(t0, t1, false);
    assert_eq!(w.g.army_raid_refusal(a, c), Some("Requires war with the city's owner."));
    assert!(w.g.army_set_station(w.p0, a, Station::Raid(c)).is_err());
    w.g.team_declare_war(t0, t1, false, None);
    // A stripped countryside: refused.
    w.g.cities[c].raid_stripped = 3;
    assert_eq!(w.g.army_raid_refusal(a, c), Some("The countryside is stripped."));
    w.g.cities[c].raid_stripped = 0;
    assert_eq!(w.g.army_raid_refusal(a, c), None);
    // Taken: its moves end, its units sleep, both sides are told.
    w.raid(a);
    assert_eq!(w.g.armies[a].station, Station::Raid(c));
    assert_eq!(w.g.army_raid_city(a), Some(c));
    assert_eq!(w.g.city_raiders(c), vec![a]);
    let units = w.g.army_units(a);
    assert!(units.iter().all(|&u| !w.g.unit_can_move(u)));
    let grp = w.g.armies[a].group.unwrap();
    assert_eq!(w.g.groups[grp].activity_type, ActivityType::ACTIVITY_SLEEP);
    assert!(w.logged("raids"));
    assert_eq!(w.g.army_raid_refusal(a, c), Some("The army raids this city already."));
    // A fleet's orders are not a land army's.
    assert!(w.g.army_set_station(w.p0, a, Station::Patrol(SeaId::from_usize(0))).is_err());
}

#[test]
fn a_move_order_or_the_peace_or_the_city_gone_ends_it() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    // A move order ends it at once.
    let grp = w.g.armies[a].group.unwrap();
    w.g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, 13, 5, 0, false, true, None, None, None);
    assert_eq!(w.g.armies[a].station, Station::None);
    assert!(w.logged("stops raiding"));
    // The peace: no longer valid at once, cleared at its owner's turn end.
    let b = w.army(w.p0, "UNIT_WARRIOR", 11, 6);
    w.raid(b);
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_make_peace(t0, t1, false);
    assert_eq!(w.g.army_raid_city(b), None);
    assert!(w.g.city_raiders(c).is_empty());
    w.g.player_fleet_stations(w.p0);
    assert_eq!(w.g.armies[b].station, Station::None);
    // The city gone: nothing panics, the order is cleared.
    w.g.team_declare_war(t0, t1, false, None);
    let e = w.army(w.p0, "UNIT_WARRIOR", 9, 4);
    w.raid(e);
    w.g.city_kill(c, true);
    assert!(!w.g.cities.contains(c));
    assert_eq!(w.g.army_raid_city(e), None);
    w.g.player_do_raids(w.p1);
    w.g.player_fleet_stations(w.p0);
    assert_eq!(w.g.armies[e].station, Station::None);
}

#[test]
fn overrun_takes_what_the_city_loses_and_the_modifier_goes_with_the_raiders() {
    let mut w = world();
    let c = w.city;
    let (prod, comm) = (w.rate(YieldType::YIELD_PRODUCTION), w.rate(YieldType::YIELD_COMMERCE));
    assert!(prod >= 10 && comm >= 10);
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    let forecast = w.g.army_raid_forecast(a, c);
    let gold = w.g.players[w.p0].gold;
    // No guard: no sally; overrun.
    w.g.player_do_raids(w.p1);
    assert_eq!(w.g.cities[c].raid_percent, 50);
    assert_eq!(w.g.city_raid_state(c), RaidState::Overrun);
    assert_eq!(w.g.cities[c].raid_turns, 1);
    let (prod2, comm2) = (w.rate(YieldType::YIELD_PRODUCTION), w.rate(YieldType::YIELD_COMMERCE));
    assert!(prod2 < prod && comm2 < comm, "{prod} {prod2} {comm} {comm2} {:?}", w.g.cities[c].base_yield_rate);
    let calc = w.g.city_raid_calc(c);
    assert_eq!((calc.production_without, calc.production_with), (prod, prod2));
    assert_eq!((calc.commerce_without, calc.commerce_with), (comm, comm2));
    assert_eq!(calc.total, (prod - prod2) + (comm - comm2));
    assert_eq!(calc, forecast, "the forecast is the take");
    assert_eq!(calc.shares.len(), 1);
    let share = &calc.shares[0];
    assert_eq!(share.gold, share.share + share.share * w.g.players[w.p0].pillaging_gold / 100);
    assert_eq!(w.g.players[w.p0].gold - gold, share.gold);
    assert_eq!(w.g.city_base_yield_rate_modifier(c, YieldType::YIELD_PRODUCTION, 50), w.g.city_base_yield_rate_modifier(c, YieldType::YIELD_PRODUCTION, 0) + 50);
    assert_eq!(w.g.city_base_yield_rate_modifier(c, YieldType::YIELD_FOOD, 0), 100, "food is untouched");
    assert!(w.logged("Raid on"));
    // The raiders leave: no take, the modifier goes, the counter falls.
    w.g.army_set_station(w.p0, a, Station::None).unwrap();
    w.g.player_do_raids(w.p1);
    assert_eq!(w.g.cities[c].raid_percent, 0);
    assert_eq!(w.g.city_raid_state(c), RaidState::None);
    assert_eq!(w.g.cities[c].raid_turns, 0);
    assert_eq!((w.rate(YieldType::YIELD_PRODUCTION), w.rate(YieldType::YIELD_COMMERCE)), (prod, comm));
}

#[test]
fn two_raiders_share_the_take() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    let b = w.army(w.p0, "UNIT_WARRIOR", 8, 5);
    w.raid(a);
    w.raid(b);
    let gold = w.g.players[w.p0].gold;
    w.g.player_do_raids(w.p1);
    let calc = w.g.city_raid_calc(c);
    assert_eq!(calc.shares.len(), 2);
    assert_eq!(calc.shares[0].share + calc.shares[1].share, calc.total);
    assert!(calc.shares[0].share >= calc.shares[1].share);
    assert_eq!(w.g.players[w.p0].gold - gold, calc.shares.iter().map(|s| s.gold).sum::<i32>());
}

#[test]
fn never_does_not_sally_always_does_and_the_guard_neither_moves_nor_pays() {
    let mut w = world();
    let c = w.city;
    let guard = w.guard("UNIT_AXEMAN", 2);
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    // Never: no battle, overrun.
    w.g.city_set_sally(w.p1, c, Sally::Never).unwrap();
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    assert!(w.reports_since(before).is_empty());
    assert_eq!(w.g.city_raid_state(c), RaidState::Overrun);
    // Always: a battle, fought from the city in the fields; the guard stays and pays nothing.
    w.g.city_set_sally(w.p1, c, Sally::Always).unwrap();
    assert!(w.g.city_set_sally(w.p0, c, Sally::Never).is_err(), "not its city");
    let moves: Vec<i32> = guard.iter().map(|&u| w.g.units[u].moves).collect();
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    let reports = w.reports_since(before);
    assert!(!reports.is_empty(), "the guard sallied");
    let r = &reports[0];
    assert_eq!((r.from, r.plot), (w.at(10, 5), w.at(12, 5)), "from the city, against a plot two away");
    assert!(!r.place.river_crossing);
    for (i, &u) in guard.iter().enumerate() {
        if w.g.units.contains(u) {
            assert_eq!(w.g.unit_plot(u), w.at(10, 5), "the guard does not move");
            assert_eq!(w.g.units[u].moves, moves[i], "it pays no moves");
            assert_eq!((w.g.units[u].attacks_made, w.g.units[u].made_attack), (0, false), "it pays no attack");
        }
    }
    // The raiders that lost were driven off (and pushed back); else the city is contested.
    match r.outcome.loser {
        Some(Side::Defender) => {
            if w.g.armies.contains(a) {
                assert_eq!(w.g.armies[a].station, Station::None, "driven off");
            }
            assert_eq!(w.g.cities[c].raid_percent, 0, "no take");
        }
        _ => {
            assert_eq!(w.g.armies[a].station, Station::Raid(c));
            assert_eq!(w.g.city_raid_state(c), RaidState::Contested);
            assert_eq!(w.g.cities[c].raid_percent, 10);
        }
    }
}

#[test]
fn raiders_that_lose_are_pushed_back_and_driven_off() {
    let mut w = world();
    let c = w.city;
    w.guard("UNIT_AXEMAN", 4);
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    let raider = w.g.army_units(a)[0];
    // A raider at 5% HP against four whole axemen.
    w.g.units[raider].damage = 95;
    w.g.city_set_sally(w.p1, c, Sally::Always).unwrap();
    let gold = w.g.players[w.p0].gold;
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    let reports = w.reports_since(before);
    assert_eq!(reports.len(), 1, "one squad of raiders: one battle");
    assert_eq!(reports[0].outcome.loser, Some(Side::Defender));
    assert_eq!(w.g.players[w.p0].gold, gold, "no take");
    assert_eq!(w.g.cities[c].raid_percent, 0);
    // (Its units may have fallen, and the army with them.)
    if w.g.units.contains(raider) && !w.g.unit_is_dead(raider) {
        assert_ne!(w.g.unit_plot(raider), w.at(12, 5), "pushed back");
    }
    if w.g.armies.contains(a) {
        assert_eq!(w.g.armies[a].station, Station::None);
        assert!(w.logged("driven off"));
    }
}

#[test]
fn contested_when_the_raiders_hold_a_sally() {
    let mut w = world();
    let c = w.city;
    let guard = w.guard("UNIT_WARRIOR", 1);
    w.g.units[guard[0]].damage = 90;
    let a = w.army(w.p0, "UNIT_AXEMAN", 12, 5);
    w.raid(a);
    w.g.city_set_sally(w.p1, c, Sally::Always).unwrap();
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    let reports = w.reports_since(before);
    assert_eq!(reports.len(), 1);
    assert_ne!(reports[0].outcome.loser, Some(Side::Defender));
    assert_eq!(w.g.armies[a].station, Station::Raid(c));
    assert_eq!(w.g.city_raid_state(c), RaidState::Contested);
    assert_eq!(w.g.cities[c].raid_percent, w.g.d.defines.raid_contested_percent);
    assert_eq!(w.g.cities[c].raid_turns, 0, "a contested turn leaves the counter");
}

#[test]
fn favoured_sallies_on_the_forecast_which_draws_no_random_number() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    let to = w.at(12, 5);
    assert_eq!(w.g.raid_sally_forecast(c, to), None, "no guard");
    let guard = w.guard("UNIT_AXEMAN", 3);
    assert_eq!(w.g.cities[c].sally, Sally::Favoured);
    let rng = w.g.soren_rand.clone();
    let won = w.g.raid_sally_forecast(c, to).expect("a squad that can fight");
    assert_eq!(w.g.soren_rand, rng, "the forecast draws nothing");
    assert_eq!(w.g.raid_sally_forecast(c, to), Some(won), "the same forecast twice");
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    assert_eq!(!w.reports_since(before).is_empty(), won >= w.g.d.defines.raid_sally_odds);
    // A weak guard stays in.
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_AXEMAN", 12, 5);
    w.raid(a);
    let weak = w.guard("UNIT_WARRIOR", 1);
    w.g.units[weak[0]].damage = 90;
    let won = w.g.raid_sally_forecast(c, to).expect("a squad that can fight");
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    assert_eq!(!w.reports_since(before).is_empty(), won >= w.g.d.defines.raid_sally_odds);
    let _ = guard;
}

#[test]
fn five_overrun_turns_strip_the_countryside_for_fifteen() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    for turn in 1..=4 {
        w.g.player_do_raids(w.p1);
        assert_eq!(w.g.cities[c].raid_turns, turn);
        assert_eq!(w.g.armies[a].station, Station::Raid(c));
    }
    let gold = w.g.players[w.p0].gold;
    w.g.player_do_raids(w.p1);
    assert!(w.g.players[w.p0].gold > gold, "the fifth turn takes");
    assert_eq!(w.g.cities[c].raid_stripped, 15);
    assert_eq!(w.g.cities[c].raid_turns, 0);
    assert_eq!(w.g.city_raid_state(c), RaidState::Stripped(15));
    assert_eq!(w.g.armies[a].station, Station::None, "every order on it ends");
    assert!(w.logged("countryside stripped. No raid for 15 turns."));
    assert_eq!(w.g.army_raid_refusal(a, c), Some("The countryside is stripped."));
    for left in (0..15).rev() {
        w.g.player_do_raids(w.p1);
        assert_eq!(w.g.cities[c].raid_stripped, left);
        assert_eq!(w.g.cities[c].raid_percent, 0, "nothing is taken");
    }
    assert_eq!(w.g.city_raid_state(c), RaidState::None);
    assert_eq!(w.g.army_raid_refusal(a, c), None, "recovered");
}

#[test]
fn raiders_neither_besiege_nor_invest_and_are_supplied() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 11, 5);
    for _ in 0..3 {
        let u = w.unit(w.p0, "UNIT_WARRIOR", 11, 5);
        let grp = w.g.armies[a].group.unwrap();
        w.g.unit_join_group(u, Some(grp), false);
    }
    w.g.player_army_sync(w.p0);
    assert!(w.g.city_is_besieged(c));
    assert_eq!(w.g.city_besiegers(c).len(), 4);
    assert!(w.g.city_is_invested(c));
    w.raid(a);
    assert!(!w.g.city_is_besieged(c), "raiders are dispersed");
    assert!(w.g.city_besiegers(c).is_empty());
    assert!(!w.g.city_is_invested(c));
    // Far from its own city, the line is cut: raiders are supplied all the same.
    assert!(w.g.army_supply_line(a).is_none());
    for _ in 0..4 {
        w.g.player_update_supply(w.p0);
    }
    assert_eq!(w.g.armies[a].unsupplied_turns, 0);
    assert_eq!(w.g.army_supply_state(a), SupplyState::Supplied);
    assert!(w.g.army_in_supply(a));
    // Without the order it goes hungry.
    w.g.army_set_station(w.p0, a, Station::None).unwrap();
    w.g.player_update_supply(w.p0);
    assert_eq!(w.g.armies[a].unsupplied_turns, 1);
    assert!(w.g.city_is_besieged(c));
}

#[test]
fn fields_are_raided_not_pillaged() {
    let mut w = world();
    let d = w.g.d;
    let (farm, fort, cottage, town) = (d.t::<ImprovementType>("IMPROVEMENT_FARM"), d.t::<ImprovementType>("IMPROVEMENT_FORT"), d.t::<ImprovementType>("IMPROVEMENT_COTTAGE"), d.t::<ImprovementType>("IMPROVEMENT_TOWN"));
    assert!(w.g.improvement_is_fields(farm) && w.g.improvement_is_fields(cottage) && w.g.improvement_is_fields(town));
    assert!(!w.g.improvement_is_fields(fort));
    let (field, far) = (w.at(12, 5), w.at(20, 5));
    w.g.plot_set_improvement_type(field, Some(farm));
    w.g.plot_set_improvement_type(far, Some(farm));
    w.g.plot_set_owner(far, Some(w.p1), false, false);
    w.g.plot_update_working_city(far);
    assert!(w.g.plot_is_fields(field));
    assert!(!w.g.plot_is_fields(far), "no city works it");
    let u = w.unit(w.p0, "UNIT_AXEMAN", 12, 5);
    assert!(!w.g.unit_can_pillage(u, field));
    assert_eq!(w.g.unit_pillage_refusal(u, field), Some("Fields are raided, not pillaged."));
    let v = w.unit(w.p0, "UNIT_AXEMAN", 20, 5);
    assert!(w.g.unit_can_pillage(v, far), "a farm no city works");
    assert_eq!(w.g.unit_pillage_refusal(v, far), None);
    // A road on the fields: the road is pillaged, the farm stays.
    let road = d.t::<RouteType>("ROUTE_ROAD");
    w.g.plot_set_route_type(field, Some(road), false);
    assert!(w.g.unit_can_pillage(u, field));
    assert!(w.g.unit_pillage(u));
    assert_eq!(w.g.map.plots[field].route_type, None);
    assert_eq!(w.g.map.plots[field].improvement_type, Some(farm));
    // A fort on the countryside is pillaged.
    let fort_plot = w.at(11, 6);
    w.g.plot_set_improvement_type(fort_plot, Some(fort));
    let f = w.unit(w.p0, "UNIT_AXEMAN", 11, 6);
    assert!(w.g.unit_can_pillage(f, fort_plot));
}

#[test]
fn the_sally_is_fought_on_open_ground_with_no_terrain_terms() {
    let mut w = world();
    let d = w.g.d;
    let guard = w.guard("UNIT_AXEMAN", 1)[0];
    let raider = w.unit(w.p0, "UNIT_AXEMAN", 12, 5);
    let (city, to) = (w.at(10, 5), w.at(12, 5));
    let flat = w.g.engage_hit(guard, raider, Side::Attacker, &HitCtx::sally(city, to));
    let flat_back = w.g.engage_hit(raider, guard, Side::Defender, &HitCtx::sally(city, to));
    // The raiders on a forested hill, fortified.
    w.g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
    w.g.plot_set_feature_type(to, Some(d.t::<FeatureType>("FEATURE_FOREST")));
    w.g.units[raider].fortify_turns = 5;
    let ctx = HitCtx::sally(city, to);
    let ground = w.g.engage_ground(&ctx);
    assert_eq!(ground, ffh_core::engagement::Ground { open: true, ..Default::default() });
    assert_eq!(w.g.engage_hit(guard, raider, Side::Attacker, &ctx), flat);
    assert_eq!(w.g.engage_hit(raider, guard, Side::Defender, &ctx), flat_back);
    // An attack there counts them.
    let attack = HitCtx::new(w.at(11, 5), to);
    assert!(w.g.engage_ground(&attack).hills && w.g.engage_ground(&attack).forest);
}

#[test]
fn a_save_keeps_the_raid() {
    let mut w = world();
    let c = w.city;
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    w.g.city_set_sally(w.p1, c, Sally::Always).unwrap();
    w.g.player_do_raids(w.p1);
    w.g.cities[c].raid_stripped = 0;
    let h = Game::load(w.g.d, &w.g.save()).expect("a save of this build");
    assert_eq!(h.armies[a].station, Station::Raid(c));
    assert_eq!(h.cities[c].sally, Sally::Always);
    assert_eq!((h.cities[c].raid_percent, h.cities[c].raid_turns, h.cities[c].raid_stripped), (w.g.cities[c].raid_percent, w.g.cities[c].raid_turns, 0));
    assert!(h.save() == w.g.save());
}

#[test]
fn every_order_on_a_plot_ends_when_its_raiders_lose() {
    let mut w = world();
    let c = w.city;
    w.guard("UNIT_AXEMAN", 4);
    let a = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    let b = w.army(w.p0, "UNIT_WARRIOR", 12, 5);
    w.raid(a);
    w.raid(b);
    assert_eq!(w.g.city_raiders(c), vec![a.min(b), a.max(b)]);
    let units: Vec<UnitId> = w.g.army_units(a).into_iter().chain(w.g.army_units(b)).collect();
    w.g.city_set_sally(w.p1, c, Sally::Always).unwrap();
    let before = w.g.battle_report_next;
    w.g.player_do_raids(w.p1);
    let reports = w.reports_since(before);
    assert!(!reports.is_empty());
    assert_eq!(reports[0].outcome.loser, Some(Side::Defender), "a warrior against four axemen");
    assert_eq!(reports.len(), 1, "no more sallies against them this turn");
    assert!(w.g.city_raiders(c).is_empty());
    for ar in [a, b] {
        if w.g.armies.contains(ar) {
            assert_eq!(w.g.armies[ar].station, Station::None);
        }
    }
    let survivors: Vec<UnitId> = units.into_iter().filter(|&u| w.g.units.contains(u) && !w.g.unit_is_dead(u)).collect();
    assert!(!survivors.is_empty(), "the squad that did not fight is pushed back, not killed");
    for u in survivors {
        assert_ne!(w.g.unit_plot(u), w.at(12, 5), "pushed back");
    }
    assert!(w.logged("driven off"));
    assert_eq!(w.g.cities[c].raid_percent, 0);
}
