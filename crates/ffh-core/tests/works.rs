//! City works (docs/WORKS.md; `ffh_core::works`): the cities' crews and the army units' field
//! works, on situations built by hand on a duel map.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// Players 0 (human) and 1 on a duel map; `row`: seven flat grassland plots in a line, west to
/// east, with land above and below, no unit, feature, improvement, resource or river crossing,
/// three plots or more from any city. Player 0 has a city on `row[1]` and owns `row[0..=2]`; the
/// plots further east and the plots around them are nobody's.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    row: [PlotIdx; 7],
    city: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        let (x, y) = g.map.xy(p);
        let near_city = (-2..=2).any(|dx| (-2..=2).any(|dy| g.plot_xy(x, y, dx, dy).is_some_and(|q| g.map.plots[q].plot_city.is_some())));
        !g.plot_is_water(p)
            && !g.plot_is_peak(p)
            && !g.plot_is_hills(p)
            && plot.units.is_empty()
            && plot.owner.is_none()
            && plot.feature_type.is_none()
            && plot.improvement_type.is_none()
            && plot.bonus_type.is_none()
            && !near_city
    };
    let row = g
        .map
        .plot_ids()
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let mut row = [p; 7];
            for (i, slot) in row.iter_mut().enumerate() {
                let q = g.map.plot_xy(x + i as i32, y)?;
                if !flat(&g, q) || (i > 0 && g.plot_is_river_crossing(q, Some(DirectionType::DIRECTION_WEST))) {
                    return None;
                }
                for dy in [-1, 1] {
                    if g.map.plot_xy(x + i as i32, y + dy).is_none_or(|r| g.plot_is_water(r)) {
                        return None;
                    }
                }
                *slot = q;
            }
            Some(row)
        })
        .expect("seven flat plots in a row");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for &p in &row {
        g.map.plots[p].terrain_type = Some(grass);
    }
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    g.players[p0].human = true;
    let (x, y) = g.map.xy(row[1]);
    let city = g.player_init_city(p0, x, y, true, true);
    for &q in &row[0..=2] {
        g.plot_set_owner(q, Some(p0), false, false);
    }
    for &q in &row[3..] {
        let (qx, qy) = g.map.xy(q);
        for dy in -1..=1 {
            if let Some(r) = g.map.plot_xy(qx, qy + dy) {
                g.plot_set_owner(r, None, false, false);
                g.plot_set_route_type(r, None, false);
            }
        }
    }
    World { g, p0, p1, row, city }
}

impl World {
    fn tech(&mut self, p: PlayerId, name: &str) {
        let t = self.g.d.t::<TechType>(name);
        let team = self.g.players[p].team;
        self.g.team_set_has_tech(team, t, true, Some(p), false, false);
    }

    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn war(&mut self) {
        let (t0, t1) = (self.g.players[self.p0].team, self.g.players[self.p1].team);
        self.g.team_declare_war(t0, t1, false, None);
    }
}

/// What player `p`'s work rate modifiers make of `rate` for its cities' crews (`units`: for its
/// units, which its civilization's modifier leaves out).
fn modified(g: &Game, p: PlayerId, rate: i32, units: bool) -> i32 {
    let civ = if units { 0 } else { g.player_get_civilization_type(p).map_or(0, |c| g.d.civilizations[c].work_rate_modifier) };
    rate * (100 + g.players[p].worker_speed_modifier + civ) / 100
}

fn build(name: &str) -> BuildType {
    ffh_data::global().t::<BuildType>(name)
}

#[test]
fn there_are_no_worker_units() {
    let d = ffh_data::global();
    for name in ["UNIT_WORKER", "UNIT_MUD_GOLEM", "UNIT_WORKBOAT", "UNIT_PIONEER"] {
        assert!(d.ty::<UnitType>(name).is_none(), "{name}");
    }
    // No unit type has a crews' build as its own, but for the Road (nobody's own either).
    for b in d.builds.keys().filter(|&b| d.builds[b].city) {
        assert!(d.units.iter().all(|u| !u.builds[b]), "{}", d.builds[b].base.type_);
    }
}

#[test]
fn crews_improve_their_citys_land() {
    let mut w = world();
    for t in ["TECH_AGRICULTURE", "TECH_EDUCATION", "TECH_EXPLORATION"] {
        w.tech(w.p0, t);
    }
    let (city, q) = (w.city, w.row[2]);
    let g = &mut w.g;
    assert_eq!(g.plot_crew_city(q, w.p0), Some(city));
    assert_eq!(g.city_crew_rate(city), modified(g, w.p0, g.d.defines.city_work_rate, false), "a human's city");
    let b = g.d.builds.keys().find(|&b| g.d.builds[b].improvement.is_some() && g.city_crews_can_build(city, q, b)).expect("an improvement the crews can build");
    assert!(g.city_choose_crew_build_default(city).is_some(), "the crews find work by themselves");
    assert!(g.city_set_crew_build(city, Some((q, b))));
    let turns = g.city_crew_turns_left(city, q, b).expect("a work rate");
    assert!(turns > 1);
    for turn in 0..turns {
        assert_eq!(g.cities[city].crew_build, Some((q, b)), "turn {turn}: they keep their build");
        assert!(g.map.plots[q].improvement_type.is_none());
        g.city_do_crews(city);
    }
    assert_eq!(g.map.plots[q].improvement_type, g.d.builds[b].improvement);
    assert_eq!(g.cities[city].crew_build, None, "done: they choose again next turn");
}

#[test]
fn crews_keep_to_their_land_and_their_builds() {
    let mut w = world();
    for t in ["TECH_AGRICULTURE", "TECH_EXPLORATION", "TECH_CONSTRUCTION"] {
        w.tech(w.p0, t);
    }
    let (city, row, p1) = (w.city, w.row, w.p1);
    let g = &mut w.g;
    let (road, fort, farm) = (build("BUILD_ROAD"), build("BUILD_FORT"), build("BUILD_FARM"));
    // The field works are the army units'.
    assert!(g.d.builds[fort].field && !g.d.builds[fort].city);
    assert!(!g.city_crews_can_build(city, row[2], fort));
    assert!(!g.city_set_crew_build(city, Some((row[2], fort))) && g.cities[city].crew_build.is_none());
    // Beyond the borders: a road, from our land or from a road, plot by plot.
    assert!(g.city_crews_can_build(city, row[2], road));
    assert!(g.city_crews_can_build(city, row[3], road), "next to our land");
    assert!(!g.city_crews_can_build(city, row[3], farm), "only a road beyond the borders");
    assert!(!g.city_crews_can_build(city, row[5], road), "not ahead of the road");
    let route = g.d.builds[road].route;
    g.plot_set_route_type(row[3], route, false);
    g.plot_set_route_type(row[4], route, false);
    assert!(g.city_crews_can_build(city, row[5], road), "from the road's end");
    // Never in another civilization's land.
    g.plot_set_owner(row[5], Some(p1), false, false);
    assert!(!g.city_crews_can_build(city, row[5], road));
}

#[test]
fn an_enemy_unit_keeps_the_crews_off() {
    let mut w = world();
    w.tech(w.p0, "TECH_EXPLORATION");
    let (city, row, p1) = (w.city, w.row, w.p1);
    let road = build("BUILD_ROAD");
    let foe = w.unit(p1, "UNIT_WARRIOR", row[4]);
    assert!(w.g.city_crews_can_build(city, row[3], road), "at peace");
    w.war();
    assert!(w.g.units.contains(foe));
    assert!(!w.g.city_crews_can_build(city, row[3], road), "beyond the borders: off a plot next to an enemy");
    assert!(w.g.city_crews_can_build(city, row[2], road), "inside them: only off its plot");
    w.unit(p1, "UNIT_WARRIOR", row[2]);
    assert!(!w.g.city_crews_can_build(city, row[2], road));
}

#[test]
fn army_units_build_the_field_works() {
    let mut w = world();
    w.tech(w.p0, "TECH_EXPLORATION");
    let (row, p0) = (w.row, w.p0);
    let road = build("BUILD_ROAD");
    let soldier = w.unit(p0, "UNIT_WARRIOR", row[4]);
    let scout = w.unit(p0, "UNIT_SCOUT", row[4]);
    let adept = w.unit(p0, "UNIT_ADEPT", row[4]);
    let g = &mut w.g;
    assert!(g.unit_can_build(soldier, row[4], road, false) && g.unit_can_build_route(soldier));
    assert!(!g.unit_can_build(scout, row[4], road, false), "a recon unit is no army unit");
    assert!(!g.unit_can_build(adept, row[4], road, false), "a mage builds its nodes only");
    assert!(!g.unit_can_build(soldier, row[4], build("BUILD_FARM"), false), "a farm is the crews'");
    // (Its race may add to its rate, as it did to a worker's: `work_rate_modify`.)
    let rate = modified(g, p0, g.d.defines.field_work_rate + g.units[soldier].work_rate_modify, true);
    assert_eq!(g.unit_work_rate(soldier, true), rate);
    let time = g.plot_get_build_time(row[4], road);
    for _ in 0..(time + rate - 1) / rate {
        assert!(g.map.plots[row[4]].route_type.is_none());
        g.units[soldier].moves = 0;
        assert!(g.unit_can_build(soldier, row[4], road, false));
        g.unit_build(soldier, road);
    }
    assert_eq!(g.map.plots[row[4]].route_type, g.d.builds[road].route);
}

#[test]
fn a_slave_and_the_luchuirp_add_to_the_work_rate() {
    let mut w = world();
    let (city, row, p0) = (w.city, w.row, w.p0);
    let base = w.g.city_crew_rate(city);
    let slave = w.unit(p0, "UNIT_SLAVE", row[2]);
    assert_eq!(w.g.city_crew_rate(city), base, "outside the city it adds nothing");
    let (x, y) = w.g.map.xy(row[1]);
    w.g.unit_set_xy(slave, x, y, false);
    let helps = w.g.d.units[w.g.units[slave].unit_type].crew_work;
    assert_eq!(helps, 25, "half a crew, as a Slave was half a Worker");
    assert_eq!(w.g.city_crew_rate(city), modified(&w.g, p0, w.g.d.defines.city_work_rate + helps, false));
    assert!(w.g.city_crew_rate(city) > base);
    assert!(w.g.d.units[w.g.units[slave].unit_type].builds.iter().all(|&b| !b), "a Slave builds nothing itself");
    let d = w.g.d;
    assert_eq!(d.civilizations[d.t::<CivilizationType>("CIVILIZATION_LUCHUIRP")].work_rate_modifier, 50);
    // Disorder stops the crews.
    w.g.cities[city].occupation_timer = 2;
    assert_eq!(w.g.city_crew_rate(city), 0);
}

#[test]
fn the_pirate_cove_is_the_lanuns_crews() {
    let mut w = world();
    w.tech(w.p0, "TECH_FISHING");
    let g = &w.g;
    let d = g.d;
    let cove = build("BUILD_PIRATE_COVE");
    let lanun = d.t::<CivilizationType>("CIVILIZATION_LANUN");
    assert!(d.builds[cove].city && d.builds[cove].prereq_civ == Some(lanun));
    assert!(d.ty::<SpellType>("SPELL_PIRATE_COVE").is_none());
    if g.player_get_civilization_type(w.p0) != Some(lanun) {
        assert!(g.map.plot_ids().all(|q| !g.player_can_build(w.p0, q, cove, false, false)), "nobody else builds one");
    }
}

#[test]
fn a_cards_build_takes_its_models_builders() {
    let d = ffh_data::global();
    let by = |f: &dyn Fn(&ImprovementInfo) -> bool| d.builds.iter().find(|b| b.improvement.is_some_and(|i| f(&d.improvements[i]))).expect("the card's build");
    let siege_lines = by(&|i| i.enemy_city_adjacent);
    assert!(siege_lines.city && siege_lines.field, "the Road's: crews and army units");
    let post = by(&|i| i.claim_range > 0);
    assert!(post.field && !post.city, "the Fort's: army units");
    let causeway = by(&|i| i.land_bridge);
    assert!(causeway.city && !causeway.field, "the Fishing Boats': crews");
}
