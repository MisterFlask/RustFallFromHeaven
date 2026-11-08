//! Card batch B9, map and movement rules (docs/CARDS.md; `ffh::cards::map_rules`): each card's
//! rule on a situation built by hand on a duel map.

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// Players 0 and 1 on a duel map; `row`: seven flat unowned grassland plots in a line, west to
/// east, with land above and below, no unit, feature, improvement, resource or river crossing,
/// three plots or more from any city.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    row: [PlotIdx; 7],
}

fn world() -> World {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
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
                // The plots above and below are land too (a strait is made in the middle).
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
    World { g, p0: PlayerId(0), p1: PlayerId(1), row }
}

/// Every delayed death done (a sacrificed caster, a converted loser).
fn flush(g: &mut Game) {
    let units: Vec<UnitId> = g.units.ids();
    for u in units {
        if g.units.contains(u) {
            g.unit_do_delayed_death(u);
        }
    }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    fn card(&mut self, p: PlayerId, card: &str) {
        let t = self.g.d.t::<TechType>(card);
        let team = self.g.players[p].team;
        self.g.team_set_has_tech(team, t, true, Some(p), false, false);
    }

    fn war(&mut self) {
        let (t0, t1) = (self.g.players[self.p0].team, self.g.players[self.p1].team);
        self.g.team_declare_war(t0, t1, false, None);
    }

    fn city(&mut self, p: PlayerId, at: PlotIdx) -> CityId {
        let (x, y) = self.g.map.xy(at);
        self.g.player_init_city(p, x, y, true, true)
    }

    fn build(&mut self, c: CityId, name: &str) {
        let b = self.g.d.t::<BuildingType>(name);
        self.g.city_set_num_real_building(c, b, 1);
    }

    fn improve(&mut self, at: PlotIdx, name: &str) {
        let i = self.g.d.t::<ImprovementType>(name);
        self.g.plot_set_improvement_type(at, Some(i));
    }

    fn own(&mut self, at: PlotIdx, p: PlayerId) {
        self.g.plot_set_owner(at, Some(p), true, true);
    }

    /// The plot below `p` made a Coast plot.
    fn water_below(&mut self, p: PlotIdx) -> PlotIdx {
        let (x, y) = self.g.map.xy(p);
        let q = self.g.map.plot_xy(x, y + 1).expect("a plot below");
        self.g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, true);
        let coast = self.g.d.t::<TerrainType>("TERRAIN_COAST");
        self.g.plot_set_terrain_type(q, coast, true);
        q
    }

    /// The middle plot of the row made a Coast plot: a one-plot strait between land.
    fn strait(&mut self) -> PlotIdx {
        let w = self.row[2];
        self.g.plot_set_plot_type(w, PlotType::PLOT_OCEAN, true);
        let coast = self.g.d.t::<TerrainType>("TERRAIN_COAST");
        self.g.plot_set_terrain_type(w, coast, true);
        w
    }
}

#[test]
fn wall_of_the_kings_stops_enemy_land_units_at_its_borders() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let [a, b, c, ..] = w.row;
    let city = w.city(p1, w.row[4]);
    w.own(b, p1);
    w.own(c, p1);
    let u = w.unit(p0, "UNIT_HORSEMAN", a);
    let cost = |w: &World| w.g.plot_movement_cost(b, u, a);
    let open = cost(&w);
    assert!(open < w.g.unit_max_moves(u));
    w.build(city, "BUILDING_WALL_OF_THE_KINGS");
    assert_eq!(cost(&w), open, "at peace: no stop");
    w.war();
    assert!(w.g.unit_stopped_at_border(u, a, b));
    assert_eq!(cost(&w), w.g.unit_max_moves(u), "an enemy entering the borders ends its move");
    assert!(!w.g.unit_stopped_at_border(u, b, c), "within the borders it moves on");
    let own = w.unit(p1, "UNIT_HORSEMAN", a);
    assert!(!w.g.unit_stopped_at_border(own, a, b), "the owner's units pass");
}

#[test]
fn harbor_chain_keeps_enemy_ships_from_the_water_next_to_the_city() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let water = w.strait();
    let city = w.city(p1, w.row[3]);
    let ship = w.unit(p0, "UNIT_TRIREME", water);
    // Another water plot next to the ship's and the city.
    let next = w.water_below(water);
    w.build(city, "BUILDING_HARBOR_CHAIN");
    assert!(!w.g.unit_harbor_blocked(ship, next), "at peace the chain is open");
    w.war();
    assert!(w.g.unit_harbor_blocked(ship, next));
    assert!(!w.g.unit_can_move_into(ship, next, false, false, false), "an enemy ship cannot enter the water next to it");
}

#[test]
fn siege_lines_are_built_next_to_an_enemy_city_and_heal_as_a_city() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let [a, b, c, ..] = w.row;
    w.city(p1, c);
    w.own(b, p1);
    w.own(a, p1);
    let build = w.g.d.t::<BuildType>("BUILD_SIEGE_LINES");
    w.card(p0, "TECH_CARD_SIEGE_LINES");
    assert!(!w.g.plot_can_build(b, build, p0, false), "at peace: not in their land");
    w.war();
    assert!(w.g.plot_can_build(b, build, p0, false), "at war: next to their city, in their land");
    assert!(!w.g.plot_can_build(a, build, p0, false), "not two plots away");
    let worker = w.unit(p0, "UNIT_WORKER", b);
    assert!(w.g.unit_can_build(worker, b, build, false), "a worker can build it");
    let soldier = w.unit(p0, "UNIT_WARRIOR", b);
    let before = w.g.unit_heal_rate(soldier, b);
    w.improve(b, "IMPROVEMENT_SIEGE_LINES");
    let heal = w.g.unit_heal_rate(soldier, b);
    assert!(heal > before, "{heal} > {before}");
    assert!(heal >= w.g.d.defines.city_heal_rate);
}

#[test]
fn a_powder_charge_breaches_every_sentinel_of_the_city_next_to_it() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let [a, b, ..] = w.row;
    let city = w.city(p1, b);
    let walls = w.g.d.t::<BuildingType>("BUILDING_WALLS");
    let class = w.g.d.buildings[walls].building_class_type.unwrap();
    w.g.cities[city].building_tiers[class] = BuildingTierType::STANDARD;
    w.g.city_set_num_real_building(city, walls, 1);
    w.card(p0, "TECH_CARD_POWDER_CHARGE");
    let charge = w.unit(p0, "UNIT_POWDER_CHARGE", a);
    let spell = w.g.d.t::<SpellType>("SPELL_POWDER_CHARGE");
    assert!(!w.g.unit_can_cast(charge, spell, false), "at peace: no target");
    w.war();
    assert_eq!(w.g.city_sentinels_standing(city).len(), 2);
    assert!(w.g.unit_can_cast(charge, spell, false));
    w.g.unit_cast(charge, spell);
    assert!(w.g.city_sentinels_standing(city).is_empty(), "every Sentinel is breached");
    flush(&mut w.g);
    assert!(!w.g.units.contains(charge) || w.g.units[charge].death_delay, "the Charge is used up");
}

#[test]
fn prize_courts_capture_about_half_the_ships_defeated() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let water = w.strait();
    let home = w.city(p0, w.row[0]);
    w.war();
    assert_eq!(w.g.player_naval_capture_percent(p0), 0);
    w.build(home, "BUILDING_PRIZE_COURTS");
    assert_eq!(w.g.player_naval_capture_percent(p0), 50);
    // (The loser on the next water plot: enemies on one plot would push each other off.)
    let next = w.water_below(water);
    let winner = w.unit(p0, "UNIT_TRIREME", water);
    let trireme = w.g.d.t::<UnitType>("UNIT_TRIREME");
    let mut captured = 0;
    for _ in 0..40 {
        let loser = w.unit(p1, "UNIT_TRIREME", next);
        let before = w.g.players[p0].units.iter().filter(|&&u| w.g.units[u].unit_type == trireme).count();
        w.g.unit_combat_won(winner, loser, true);
        let after = w.g.players[p0].units.iter().filter(|&&u| w.g.units.contains(u) && w.g.units[u].unit_type == trireme).count();
        captured += (after > before) as i32;
        if w.g.units.contains(loser) && !w.g.units[loser].death_delay {
            w.g.unit_kill(loser, false, None);
        }
        flush(&mut w.g);
    }
    assert!((5..=35).contains(&captured), "{captured} of 40 captured at 50%");
}

#[test]
fn the_hall_of_judgment_keeps_a_conquered_citys_buildings_and_population() {
    let d = ffh_data::global();
    let granary = d.t::<BuildingType>("BUILDING_GRANARY");
    // Conquests with and without the Hall: the Granary survives a capture with its chance only.
    let mut kept = [0, 0];
    let mut pop = [0, 0];
    for (k, judged) in [false, true].into_iter().enumerate() {
        let mut w = world();
        let (p0, p1) = (w.p0, w.p1);
        let home = w.city(p0, w.row[0]);
        if judged {
            w.build(home, "BUILDING_HALL_OF_JUDGMENT");
        }
        let target = w.city(p1, w.row[3]);
        w.g.city_set_population(target, 5);
        let class = d.buildings[granary].building_class_type.unwrap();
        w.g.cities[target].building_tiers[class] = BuildingTierType::STANDARD;
        w.g.city_set_num_real_building(target, granary, 1);
        w.war();
        w.g.player_acquire_city(p0, target, true, false, true);
        let c = w.g.map.plots[w.row[3]].plot_city.expect("the city stands");
        kept[k] = w.g.cities[c].num_real_building[granary];
        pop[k] = w.g.cities[c].population;
    }
    assert_eq!(pop, [4, 5], "a conquest costs a citizen, unless the conqueror holds the Hall");
    assert_eq!(kept[1], 1, "the Hall keeps the Granary");
}

#[test]
fn a_causeway_carries_land_units_and_stops_ships() {
    let mut w = world();
    let p0 = w.p0;
    let water = w.strait();
    let next = w.water_below(water);
    let warrior = w.unit(p0, "UNIT_WARRIOR", w.row[1]);
    assert!(!w.g.unit_can_move_into(warrior, water, false, false, false), "a land unit cannot walk on water");
    let boat = w.unit(p0, "UNIT_WORKBOAT", water);
    w.card(p0, "TECH_CARD_CAUSEWAY");
    let build = w.g.d.t::<BuildType>("BUILD_CAUSEWAY");
    assert!(w.g.unit_can_build(boat, water, build, false), "a Work Boat builds it");
    w.g.unit_kill(boat, false, None);
    let ship = w.unit(p0, "UNIT_TRIREME", next);
    assert!(w.g.unit_can_move_into(ship, water, false, false, false), "ships sail there before");
    w.improve(water, "IMPROVEMENT_CAUSEWAY");
    assert!(w.g.plot_is_land_bridge(water));
    assert!(w.g.unit_can_move_into(warrior, water, false, false, false), "land units walk onto it");
    assert!(w.g.plot_is_valid_domain_for_action(water, warrior));
    assert!(!w.g.unit_can_move_into(ship, water, false, false, false), "a dam: ships cannot enter it");
    let def = w.g.plot_defense_modifier(water, Some(w.g.players[p0].team), false);
    assert!(def >= 50, "its own defence: {def}");
}

#[test]
fn ships_enter_a_canal_unless_at_war_with_its_owner() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let water = w.strait();
    let canal_plot = w.row[3];
    w.city(p0, w.row[0]);
    for i in 3..6 {
        w.own(w.row[i], p0);
    }
    w.card(p0, "TECH_CARD_CANAL_LOCKS");
    let build = w.g.d.t::<BuildType>("BUILD_CANAL");
    assert!(w.g.plot_can_build(canal_plot, build, p0, false), "next to water");
    assert!(!w.g.plot_can_build(w.row[5], build, p0, false), "not away from water");
    w.improve(canal_plot, "IMPROVEMENT_CANAL");
    assert!(w.g.plot_can_build(w.row[4], build, p0, false), "next to another canal");
    let ship = w.unit(p0, "UNIT_TRIREME", water);
    assert!(w.g.unit_can_move_into(ship, canal_plot, false, false, false), "a ship enters the canal");
    assert!(w.g.plot_is_valid_domain_for_location(canal_plot, ship));
    w.war();
    let next = w.water_below(water);
    let enemy = w.unit(p1, "UNIT_TRIREME", next);
    assert!(!w.g.unit_can_move_into(enemy, canal_plot, false, false, false), "the locks close to an enemy");
    assert!(w.g.unit_can_move_into(ship, canal_plot, false, false, false), "and stay open to the owner");
}

#[test]
fn mountaineering_opens_peaks_at_three_moves_with_defence_and_sight() {
    let mut w = world();
    let p0 = w.p0;
    let peak = w.row[2];
    w.g.plot_set_plot_type(peak, PlotType::PLOT_PEAK, true);
    let u = w.unit(p0, "UNIT_HORSEMAN", w.row[1]);
    assert!(!w.g.unit_can_move_into(u, peak, false, false, false), "peaks are impassable");
    w.card(p0, "TECH_CARD_MOUNTAINEERING");
    assert!(w.g.unit_can_move_into(u, peak, false, false, false), "the holder's land units climb");
    let moves = w.g.unit_base_moves(u).min(3) * w.g.d.defines.move_denominator;
    assert_eq!(w.g.plot_movement_cost(peak, u, w.row[1]), moves, "3 moves (at most its moves)");
    let worker = w.unit(p0, "UNIT_WORKER", w.row[1]);
    assert!(w.g.unit_can_move_into(worker, peak, false, false, false), "workers too");
    let (x, y) = w.g.map.xy(peak);
    w.g.unit_set_xy(u, x, y, false);
    assert_eq!(w.g.unit_visibility_range(u), 3, "sight 3 on a peak");
    assert!(w.g.unit_on_climbed_peak(u, peak));
    // +50% defence there, in a battle.
    let ctx = ffh_core::engagement::stats::HitCtx::new(w.row[1], peak);
    let dv = w.g.engage_stat(u, None, ffh_core::engagement::Side::Defender, false, &ctx);
    let peak_terrain = w.g.d.t::<TerrainType>("TERRAIN_PEAK");
    assert!(dv.terms.iter().any(|t| t.src == ffh_core::engagement::report::TermSource::Terrain(peak_terrain) && t.pct == 50), "{:?}", dv.terms);
}

#[test]
fn ferries_let_land_units_cross_a_one_plot_strait() {
    let mut w = world();
    let p0 = w.p0;
    let water = w.strait();
    assert!(w.g.plot_is_strait(water));
    assert!(!w.g.plot_is_strait(w.row[1]), "land is no strait");
    let u = w.unit(p0, "UNIT_WARRIOR", w.row[1]);
    assert!(!w.g.unit_can_move_into(u, water, false, false, false));
    w.card(p0, "TECH_CARD_FERRIES");
    assert!(w.g.unit_can_move_into(u, water, false, false, false), "it walks onto the strait");
    assert_eq!(w.g.plot_movement_cost(water, u, w.row[1]), w.g.d.defines.move_denominator, "as on flat land");
    let (x, y) = w.g.map.xy(water);
    w.g.unit_set_xy(u, x, y, false);
    assert!(w.g.unit_can_move_into(u, w.row[3], false, false, false), "and off it to the far shore");
}

#[test]
fn forest_paths_open_the_holders_forests_and_slow_its_enemies() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let [a, b, c, ..] = w.row;
    let forest = w.g.d.t::<FeatureType>("FEATURE_FOREST");
    w.g.plot_set_feature_type(b, Some(forest));
    w.city(p0, w.row[4]);
    w.own(b, p0);
    w.own(c, p0);
    let ours = w.unit(p0, "UNIT_HORSEMAN", a);
    let theirs = w.unit(p1, "UNIT_HORSEMAN", b);
    let slow = w.g.plot_movement_cost(b, ours, a);
    w.card(p0, "TECH_CARD_FOREST_PATHS");
    let fast = w.g.plot_movement_cost(b, ours, a);
    assert!(fast < slow, "open ground for the holder: {fast} < {slow}");
    let max = w.g.unit_max_moves(theirs);
    assert_eq!(w.g.unit_turn_moves_at(theirs, b), max, "at peace: no penalty");
    w.war();
    assert_eq!(w.g.unit_turn_moves_at(theirs, b), max - w.g.d.defines.move_denominator, "an enemy in the forest: one move less");
    w.g.unit_do_turn(theirs);
    assert_eq!(w.g.unit_moves_left(theirs), max - w.g.d.defines.move_denominator, "at the start of its turn");
}

#[test]
fn wayside_shrines_stand_on_roads_and_heal_as_a_city() {
    let mut w = world();
    let p0 = w.p0;
    let [a, b, ..] = w.row;
    w.city(p0, w.row[4]);
    w.own(b, p0);
    w.card(p0, "TECH_CARD_WAYSIDE_SHRINES");
    let build = w.g.d.t::<BuildType>("BUILD_WAYSIDE_SHRINE");
    assert!(!w.g.plot_can_build(b, build, p0, false), "no road");
    let road = w.g.d.t::<RouteType>("ROUTE_ROAD");
    w.g.plot_set_route_type(b, Some(road), true);
    assert!(w.g.plot_can_build(b, build, p0, false), "on a road");
    let u = w.unit(p0, "UNIT_WARRIOR", b);
    let _ = a;
    let before = w.g.unit_heal_rate(u, b);
    w.improve(b, "IMPROVEMENT_WAYSIDE_SHRINE");
    assert!(w.g.unit_heal_rate(u, b) > before);
}

#[test]
fn a_wheeled_city_packs_up_and_refounds_with_its_people_and_buildings() {
    let mut w = world();
    let p0 = w.p0;
    let capital = w.city(p0, w.row[0]);
    let city = w.city(p0, w.row[4]);
    assert!(!w.g.city_can_pack_up(capital), "not the capital");
    w.g.city_set_population(city, 6);
    w.build(city, "BUILDING_GRANARY");
    w.build(city, "BUILDING_WHEELED_CITY");
    let caravan = w.g.d.t::<UnitType>("UNIT_CITY_CARAVAN");
    let u = w.g.city_pack_up(city, caravan).expect("it packs up");
    assert!(w.g.map.plots[w.row[4]].plot_city.is_none(), "the city is gone");
    let pc = w.g.units[u].packed_city.clone().expect("the caravan carries it");
    assert_eq!(pc.population, 6);
    // The caravan's city is saved with it.
    let loaded = Game::load(w.g.d, &w.g.save()).expect("the save loads");
    assert_eq!(loaded.units[u].packed_city.as_ref(), Some(&pc));
    // The caravan walks on and refounds it.
    let (x, y) = w.g.map.xy(w.row[3]);
    w.g.unit_set_xy(u, x, y, false);
    assert!(w.g.unit_can_unpack(u));
    let c = w.g.unit_unpack(u).expect("it refounds");
    assert_eq!(w.g.cities[c].population, 6);
    let granary = w.g.d.t::<BuildingType>("BUILDING_GRANARY");
    assert_eq!(w.g.city_num_family(c, granary), 1, "its buildings come with it (at its tier)");
    assert_eq!(w.g.cities[c].building_tiers, pc.tiers, "and its tiers");
}

#[test]
fn false_colours_attack_a_civ_at_peace_and_declare_the_war_at_the_battle() {
    let mut w = world();
    let (p0, p1) = (w.p0, w.p1);
    let [a, b, ..] = w.row;
    let (t0, t1) = (w.g.players[p0].team, w.g.players[p1].team);
    w.g.team_meet(t0, t1, false);
    let defender = w.unit(p1, "UNIT_WARRIOR", b);
    w.g.unit_set_fortify_turns(defender, 3);
    let attackers: Vec<UnitId> = (0..3).map(|_| w.unit(p0, "UNIT_AXEMAN", a)).collect();
    w.card(p0, "TECH_CARD_FALSE_COLOURS");
    let masker = w.unit(p0, "UNIT_MASKER", a);
    assert!(!w.g.unit_can_move_into(attackers[0], b, true, false, false), "no attack at peace");
    let spell = w.g.d.t::<SpellType>("SPELL_FALSE_COLOURS");
    assert!(w.g.unit_can_cast(masker, spell, false));
    w.g.unit_cast(masker, spell);
    assert!(attackers.iter().all(|&u| w.g.unit_has_false_colours(u)), "the stack is under false colours");
    assert_eq!(w.g.unit_false_colours_team(attackers[0], b), Some(t1));
    assert!(w.g.unit_can_move_into(attackers[0], b, true, false, false), "it may attack them");
    assert_eq!(w.g.unit_get_declare_war_move(attackers[0], b), None, "without a declaration");
    let grp = w.g.units[attackers[0]].group.unwrap();
    for &u in &attackers[1..] {
        w.g.unit_join_group(u, Some(grp), true);
    }
    let before = w.g.battle_report_next;
    let (x, y) = w.g.map.xy(b);
    w.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
    assert!(w.g.teams[t0].at_war[t1], "the war is declared at the battle");
    assert!(w.g.battle_report_next > before, "a battle was fought");
    if w.g.units.contains(defender) {
        assert_eq!(w.g.units[defender].fortify_turns, 0, "the defenders lost their fortification");
    }
    assert!(attackers.iter().filter(|&&u| w.g.units.contains(u)).all(|&u| !w.g.unit_has_false_colours(u)), "one battle: the colours are spent");
}
