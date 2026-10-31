//! This game's armies through the human's command layer and views (docs/ARMIES.md, the protocol in
//! docs/HUMAN.md): an army's requisition set and released by commands and shown on the Armies
//! screen, a requisition order flagged in the city queue that cannot be taken off by hand, and a
//! marching recruit that shows its march and refuses every order until it arrives.

mod common;

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan};
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

const H: PlayerId = PlayerId(0);

/// A duel map with armies on, after `game_init`: player 0's first city `a` and a second city `b`
/// at least 3 plots away on the same landmass, the starting units gone.
fn game() -> (Game, CityId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
    let mut g = Game::new(d, setup);
    g.game_init();
    let u = g.players[H].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let area = g.map.plots[g.unit_plot(u)].area;
    for u in g.players[H].units.clone() {
        g.unit_kill(u, false, None);
    }
    let a = g.player_init_city(H, x, y, true, true);
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
    let b = g.player_init_city(H, bx, by, true, true);
    (g, a, b)
}

fn warrior(g: &Game) -> UnitType {
    g.d.t::<UnitType>("UNIT_WARRIOR")
}

/// An army of `n` warriors next to city `c`.
fn army_near(g: &mut Game, c: CityId, n: usize) -> ArmyId {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let (ox, oy) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)]
        .into_iter()
        .map(|(dx, dy)| (x + dx, y + dy))
        .find(|&(ox, oy)| g.map.plot_xy(ox, oy).is_some_and(|q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none()))
        .expect("land next to the city");
    let w = warrior(g);
    let units: Vec<UnitId> = (0..n).map(|_| g.player_init_unit(H, w, ox, oy, None, None)).collect();
    g.army_form(H, &units).expect("formed")
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

/// The reason of a refusal.
fn refused(r: Result<(), CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(s)) | Err(CmdErr::Invalid(s)) => s,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

/// The Armies screen's entry of army `a`.
fn screen_army(g: &Game, a: ArmyId) -> Json {
    let s = view::armies::screen(&Viewer::new(g, H, false));
    arr(&s, "armies").into_iter().find(|j| int(j, "id") == a.0 as i64).expect("the army on the screen")
}

#[test]
fn requisition_is_set_and_released_by_commands_and_its_order_cannot_be_removed_by_hand() {
    let (mut g, a, b) = game();
    let army = army_near(&mut g, a, 2);
    run(&mut g, &Command::ArmyTarget { army, size: 4 }).unwrap();
    // Every city of the human's is a candidate, none picked yet; each with the unit it would
    // train for the army and its turns.
    let e = screen_army(&g, army);
    let cands = arr(e.get("requisition").unwrap(), "candidates");
    assert_eq!(cands.len(), 2, "{e}");
    assert!(cands.iter().all(|c| c.get("picked") == Some(&Json::Bool(false))), "{e}");
    assert!(cands.iter().all(|c| c.get("unit").is_some_and(|u| u.as_i64().is_some())), "a warrior at least: {e}");
    // Pick city b; picking it again, or a list naming a city twice, is refused.
    run(&mut g, &Command::ArmyRequisition { army, city: b, on: true }).unwrap();
    assert_eq!(g.armies[army].requisition, vec![b]);
    assert!(refused(run(&mut g, &Command::ArmyRequisition { army, city: b, on: true })).contains("already"));
    assert!(refused(run(&mut g, &Command::ArmyRequisitionSet { army, cities: vec![a, a] })).contains("twice"));
    // The view's army lists the picked city (ids); the screen marks it picked.
    let va = view::armies::armies(&Viewer::new(&g, H, false));
    let v0 = &va.as_arr().unwrap()[0];
    assert_eq!(arr(v0.get("requisition").unwrap(), "cities"), vec![Json::from(b)]);
    // The turn's requisition puts an order at the front of b's queue: flagged in the city screen,
    // naming the army, and not removable by hand, nor its destination changeable.
    g.city_push_order(b, OrderType::ORDER_TRAIN, warrior(&g).idx() as i32, -1, false, false, true, false);
    g.player_do_requisition(H);
    assert!(g.cities[b].order_queue[0].requisition, "a requisition order leads b's queue");
    assert!(g.cities[b].order_queue.len() >= 2);
    let c = view::city::city(&Viewer::new(&g, H, false), b).unwrap();
    let q0 = arr(&c, "queue")[0].clone();
    assert_eq!(q0.get("requisition"), Some(&Json::Bool(true)), "{q0}");
    assert_eq!(q0.get("dest_name").and_then(Json::as_str), Some("the 1st Army"), "{q0}");
    let by = arr(c.get("recruit").unwrap(), "requisitioned_by");
    assert_eq!(by.len(), 1, "{c}");
    let why = refused(run(&mut g, &Command::PopOrder { city: b, index: 0 }));
    assert!(why.contains("requisitioned by the 1st Army"), "{why}");
    assert!(refused(run(&mut g, &Command::RecruitDest { city: b, index: 0, dest: None })).contains("requisition"));
    // The order behind it can still go.
    run(&mut g, &Command::PopOrder { city: b, index: 1 }).unwrap();
    // The screen shows the order in progress, on the army and on its city.
    let e = screen_army(&g, army);
    let orders = arr(e.get("requisition").unwrap(), "orders");
    assert_eq!(orders.len(), 1, "{e}");
    assert_eq!(int(&orders[0], "city"), b.0 as i64);
    let cb = arr(e.get("requisition").unwrap(), "candidates").into_iter().find(|j| int(j, "city") == b.0 as i64).unwrap();
    assert_eq!(cb.get("picked"), Some(&Json::Bool(true)));
    assert!(cb.get("order").is_some_and(|o| !matches!(o, Json::Null)), "{cb}");
    // Released: the order is withdrawn at once; releasing again is refused.
    run(&mut g, &Command::ArmyRequisition { army, city: b, on: false }).unwrap();
    assert!(g.armies[army].requisition.is_empty());
    assert!(!g.cities[b].order_queue.iter().any(|o| o.requisition));
    assert!(refused(run(&mut g, &Command::ArmyRequisition { army, city: b, on: false })).contains("does not"));
    // The list form sets both cities, in order, and clears them.
    run(&mut g, &Command::ArmyRequisitionSet { army, cities: vec![b, a] }).unwrap();
    assert_eq!(g.armies[army].requisition, vec![b, a]);
    run(&mut g, &Command::ArmyRequisitionSet { army, cities: vec![] }).unwrap();
    assert!(g.armies[army].requisition.is_empty());
}

#[test]
fn a_marching_recruit_shows_its_march_and_refuses_every_order() {
    let (mut g, a, b) = game();
    let army = army_near(&mut g, a, 2);
    // City b finishes a warrior for the army: it appears in b and marches.
    let w = warrior(&g);
    g.city_push_order(b, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    g.cities[b].order_queue[0].dest = Some(RecruitDest::Army(army));
    g.cities[b].unit_production[w] = g.city_production_needed_unit(b, w);
    let before = g.players[H].units.clone();
    g.city_pop_order(b, 0, true, false);
    let u = *g.players[H].units.iter().find(|u| !before.contains(u)).expect("a new unit");
    assert!(g.unit_march(u).is_some(), "the recruit marches");
    // Its view: the march with where it goes.
    let v = Viewer::new(&g, H, false);
    let uj = view::unit_json(&v, u);
    let m = uj.get("march").expect("a march");
    assert_eq!(m.get("to").and_then(Json::as_str), Some("the 1st Army"), "{uj}");
    assert_eq!(m.get("dest"), Some(&Json::parse(&format!("{{\"army\":{}}}", army.0)).unwrap()));
    // The army counts it (unit ids); the screen lists it in full.
    let va = view::armies::armies(&v);
    assert_eq!(arr(&va.as_arr().unwrap()[0], "marching"), vec![Json::from(u)]);
    let e = screen_army(&g, army);
    let r = arr(&e, "recruits");
    assert_eq!((r.len(), int(&r[0], "id")), (1, u.0 as i64), "{e}");
    // The group query offers no orders and says why.
    let grp = view::units::group(&Viewer::new(&g, H, false), &[u]).unwrap();
    assert!(arr(&grp, "actions").is_empty(), "{grp}");
    assert!(grp.get("marching").and_then(Json::as_str).is_some_and(|s| s.contains("the 1st Army")), "{grp}");
    // Every order is refused, naming its destination: a move, a mission, a command, grouping,
    // automation, the army commands.
    let (x, y) = (g.units[u].x, g.units[u].y);
    let cmds = [
        Command::Move { units: vec![u], x: x + 1, y, append: false, declare_war: None },
        Command::Mission { units: vec![u], mission: MissionType::MISSION_SKIP, data1: -1, data2: -1, flags: 0, append: false },
        Command::DoCommand { units: vec![u], command: CommandType::COMMAND_WAKE, data1: -1, data2: -1, alt: false, confirmed: false },
        Command::JoinGroup { unit: u, head: None },
        Command::AutoMission { unit: u },
        Command::ArmyForm { units: vec![u] },
        Command::ArmyGarrison { units: vec![u] },
    ];
    for c in &cmds {
        let why = refused(run(&mut g, c));
        assert!(why.contains("marching to the 1st Army"), "{c:?}: {why}");
    }
    // Another unit cannot take it into its group either.
    let other = g.player_init_unit(H, w, x, y, None, None);
    assert!(refused(run(&mut g, &Command::JoinGroup { unit: other, head: Some(u) })).contains("marching"));
    // It never waits for orders: not ready, not in the cycle.
    let grp = g.units[u].group.unwrap();
    assert!(!ffh_play::decision::ready_groups(&g, H).contains(&grp));
    assert!(!ffh_play::decision::is_selectable_group(&g, grp));
    // Arrived (its march over), it takes orders again and its view has no march.
    g.units[u].march = None;
    assert!(run(&mut g, &Command::Mission { units: vec![u], mission: MissionType::MISSION_SKIP, data1: -1, data2: -1, flags: 0, append: false }).is_ok());
    assert_eq!(view::unit_json(&Viewer::new(&g, H, false), u).get("march"), Some(&Json::Null));
}

#[test]
fn the_protocol_has_no_feed_mode_nor_reinforce_to() {
    let d = ffh_data::global();
    for line in [r#"{"c":"govern_feed","city":1,"army":2}"#, r#"{"c":"recruit_dest","city":1,"index":0,"dest":null,"reinforce_to":3}"#] {
        let j = Json::parse(line).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(_))), "{line}");
    }
    let j = Json::parse(r#"{"c":"army_requisition","army":1,"city":2,"on":true}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::ArmyRequisition { army: ArmyId(1), city: CityId(2), on: true });
    let j = Json::parse(r#"{"c":"army_requisition_set","army":1,"cities":[2,3]}"#).unwrap();
    assert_eq!(Command::from_json(d, &j).unwrap(), Command::ArmyRequisitionSet { army: ArmyId(1), cities: vec![CityId(2), CityId(3)] });
}

/// `ffh-play stage armies` (the browser smoke test's situation): the saved game loads with the
/// recruit marching, the capital's requisition order leading its queue and only the capital
/// picked.
#[test]
fn the_staged_armies_save_has_a_marching_recruit_and_a_requisition_order() {
    use std::sync::Arc;
    let dir = common::TempDir::new("stage-armies");
    let st = ffh_play::stage::armies(&dir.0).expect("staged");
    assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some("armies"));
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
    let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    let g = &s.g;
    assert!(g.unit_march(st.recruit).is_some(), "the recruit marches");
    assert!(g.cities[st.capital].order_queue[0].requisition, "the capital's requisition order leads");
    assert_eq!(g.armies[st.army].requisition, vec![st.capital]);
    assert_ne!(st.city, st.capital);
}
