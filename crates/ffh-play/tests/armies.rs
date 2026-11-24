//! This game's armies through the human's command layer and views (docs/ARMIES.md, the protocol in
//! docs/HUMAN.md): a trained unit appears in its city's garrison as an ordinary unit that takes
//! orders, and the protocol has no recruitment logistics (requisition, target size, destinations).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan};
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

const H: PlayerId = PlayerId(0);

/// A duel map with armies on, after `game_init`: player 0's first city `a`, the starting units
/// gone.
fn game() -> (Game, CityId) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let u = g.players[H].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    for u in g.players[H].units.clone() {
        g.unit_kill(u, false, None);
    }
    let a = g.player_init_city(H, x, y, true, true);
    (g, a)
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

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

#[test]
fn a_trained_unit_joins_its_city_garrison_and_takes_orders() {
    let (mut g, a) = game();
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    g.city_push_order(a, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[a].unit_production[w] = g.city_production_needed_unit(a, w);
    let before = g.players[H].units.clone();
    g.city_pop_order(a, 0, true, false);
    let u = *g.players[H].units.iter().find(|u| !before.contains(u)).expect("a new unit");
    assert_eq!((g.units[u].x, g.units[u].y), (g.cities[a].x, g.cities[a].y), "it appears in its city");
    assert_eq!(g.unit_garrison_city(u), Some(a), "in the city's garrison");
    // The city screen lists it in the garrison; the queue's orders carry no destination.
    let c = view::city::city(&Viewer::new(&g, H, false), a).unwrap();
    assert!(arr(&c, "army_garrison").contains(&Json::from(u)), "{c}");
    assert!(c.get("recruit").is_none(), "{c}");
    // Its group offers orders, and it takes one.
    let grp = view::units::group(&Viewer::new(&g, H, false), &[u]).unwrap();
    assert!(!arr(&grp, "actions").is_empty(), "{grp}");
    assert!(grp.get("marching").is_none(), "{grp}");
    run(&mut g, &Command::Mission { units: vec![u], mission: MissionType::MISSION_SKIP, data1: -1, data2: -1, flags: 0, append: false, confirm_end_siege: false }).unwrap();
}

#[test]
fn the_protocol_has_no_recruitment_logistics() {
    let d = ffh_data::global();
    for line in [
        r#"{"c":"govern_feed","city":1,"army":2}"#,
        r#"{"c":"recruit_dest","city":1,"index":0,"dest":null}"#,
        r#"{"c":"army_requisition","army":1,"city":2,"on":true}"#,
        r#"{"c":"army_requisition_set","army":1,"cities":[2,3]}"#,
        r#"{"c":"army_target","army":1,"size":4}"#,
        r#"{"c":"empire_order","city":1,"order":"ORDER_TRAIN","data":"UNIT_WARRIOR","dest":{"army":2}}"#,
    ] {
        let j = Json::parse(line).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))), "{line}");
    }
}
