//! Fleets as the human sees and commands them (docs/NAVAL.md, section 7 and 10; the protocol in
//! docs/HUMAN.md): the Armies screen's Fleets section (a fleet with its standing order, wear and
//! the seas it may patrol), `army_station` setting and ending a standing order, the unit panel's
//! fleet actions, and the refusals (no tactic, no standing order for an army); on a situation
//! built by hand.

use ffh_core::army::Station;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan};
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

const H: PlayerId = PlayerId(0);

/// A duel game on a block of grassland (x 1..=38, y 2..=9) inside a ring of coast, revealed; the
/// human (player 0) has a port on (5, 2) and knows Sailing; seas cut by hand: West (y 0..=1,
/// x 0..=12) and East (y 0..=1, x 13..=39).
fn world() -> (Game, SeaId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    setup.players[0].human = true;
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let band = |g: &Game, xs: std::ops::RangeInclusive<i32>| {
        let mut v = Vec::new();
        for y in 0..=1 {
            for x in xs.clone() {
                v.push(g.map.plot_xy(x, y).unwrap());
            }
        }
        v
    };
    let west = g.map_cut_sea("West", &band(&g, 0..=12));
    g.map_cut_sea("East", &band(&g, 13..=39));
    let (t0, t1) = (g.players[H].team, g.players[PlayerId(1)].team);
    g.team_meet(t0, t1, false);
    let sailing = d.t::<TechType>("TECH_SAILING");
    g.team_set_has_tech(t0, sailing, true, Some(H), false, false);
    g.players[H].turn_active = true;
    let port = g.player_init_city(H, 5, 2, true, true);
    (g, west, port)
}

/// Validates `cmd` for the human and applies it.
fn run(g: &mut Game, cmd: &Command) -> Result<(), CmdErr> {
    match command::validate(g, H, cmd, &Ctx { events: &[] })? {
        Plan::Apply(a) => {
            command::apply(g, H, &a);
            Ok(())
        }
        other => panic!("{cmd:?}: {other:?}"),
    }
}

fn refused(r: Result<(), CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(s)) | Err(CmdErr::Invalid(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

#[test]
fn the_fleets_section_and_the_standing_order() {
    let (mut g, west, _port) = world();
    let tri = g.d.t::<UnitType>("UNIT_TRIREME");
    let ship = g.player_init_unit(H, tri, 9, 1, None, None);
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let soldier = g.player_init_unit(H, warrior, 9, 4, None, None);
    g.player_army_sync(H);
    let f = g.units[ship].army.expect("a fleet");
    let army = g.units[soldier].army.expect("an army");
    // The screen: the army under Armies, the fleet under Fleets, with its order and wear.
    let s = view::armies::screen(&Viewer::new(&g, H, false));
    assert_eq!(arr(&s, "armies").len(), 1, "{s}");
    let fleets = arr(&s, "fleets");
    assert_eq!(fleets.len(), 1, "{s}");
    let e = &fleets[0];
    assert_eq!(text(e, "name"), "1st Fleet");
    assert_eq!(text(e, "domain"), "sea");
    assert_eq!(e.get("supply"), Some(&Json::Null), "no supply line");
    assert_eq!(text(e.get("station").unwrap(), "text"), "Standing order: none.");
    assert_eq!(text(e, "wear_text"), "Wear: none.");
    assert!(arr(&s, "seas").iter().any(|j| text(j, "name") == "West"), "{s}");
    assert_eq!(arr(&s, "escort_targets").len(), 2);
    // The view's own list holds both (the unit panel's badge names the fleet).
    let va = view::armies::armies(&Viewer::new(&g, H, false));
    assert_eq!(va.as_arr().unwrap().len(), 2);
    // Patrol of West by command: the order stands and the screen says so.
    run(&mut g, &Command::ArmyStation { army: f, station: Station::Patrol(west) }).unwrap();
    assert_eq!(g.armies[f].station, Station::Patrol(west));
    let s = view::armies::screen(&Viewer::new(&g, H, false));
    let e = arr(&s, "fleets")[0].clone();
    assert_eq!(text(e.get("station").unwrap(), "text"), "Standing order: Patrol of West.");
    // A move order ends it.
    run(&mut g, &Command::Move { units: vec![ship], x: 8, y: 0, append: false, declare_war: None }).unwrap();
    assert_eq!(g.armies[f].station, Station::None);
    // Refusals: an army takes no standing order, a fleet no tactic.
    assert!(refused(run(&mut g, &Command::ArmyStation { army, station: Station::Patrol(west) })).contains("only a fleet"));
    let tactic = g.armies[f].tactic;
    assert!(refused(run(&mut g, &Command::ArmyTactic { army: f, tactic })).contains("no battle tactic"));
}

#[test]
fn the_unit_panel_offers_fleet_actions_for_ships() {
    let (mut g, _west, _port) = world();
    let tri = g.d.t::<UnitType>("UNIT_TRIREME");
    let a = g.player_init_unit(H, tri, 9, 1, None, None);
    let b = g.player_init_unit(H, tri, 9, 1, None, None);
    g.player_army_sync(H);
    g.army_form(H, &[a, b]).unwrap();
    let j = view::units::group(&Viewer::new(&g, H, false), &[a]).unwrap();
    let labels: Vec<String> = arr(&j, "actions").iter().filter(|x| x.get("kind").and_then(Json::as_str) == Some("army")).filter_map(|x| x.get("label").and_then(Json::as_str).map(str::to_string)).collect();
    assert!(labels.iter().any(|l| l == "Split into a new fleet"), "{labels:?}");
    let both = view::units::group(&Viewer::new(&g, H, false), &[a, b]).unwrap();
    let labels: Vec<String> = arr(&both, "actions").iter().filter(|x| x.get("kind").and_then(Json::as_str) == Some("army")).filter_map(|x| x.get("label").and_then(Json::as_str).map(str::to_string)).collect();
    assert!(labels.iter().any(|l| l.starts_with("Disband: sail to")), "{labels:?}");
    assert!(!labels.iter().any(|l| l.contains("garrison")), "{labels:?}");
    // Split by command: a second fleet (the lowest free number: the first fleets were merged
    // into the one formed above).
    run(&mut g, &Command::ArmySplit { units: vec![a] }).unwrap();
    assert_ne!(g.units[a].army, g.units[b].army);
    assert_eq!(g.armies[g.units[a].army.unwrap()].name, "1st Fleet");
}
