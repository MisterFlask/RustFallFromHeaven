//! Squads through the human's command layer and views (docs/ENGAGEMENTS.md, "Squads"; the
//! protocol in docs/HUMAN.md): `squad_move`, `squad_auto` and `squad_sort` on a staged army, and
//! what the unit views, the `group` query and the Armies screen show of its squads after each.
//! Asserts only what holds for any arrangement the rules' auto formation makes.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan};
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

const H: PlayerId = PlayerId(0);

/// A duel map after `game_init`, player 0's starting units gone, and an army of player 0 of four
/// warriors and three archers on a land plot of the starting area away from any city.
fn game() -> (Game, ArmyId, Vec<UnitId>) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    let mut g = Game::new(d, setup);
    g.game_init();
    let u = g.players[H].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    for u in g.players[H].units.clone() {
        g.unit_kill(u, false, None);
    }
    let (ox, oy) = (0..=3)
        .flat_map(|r: i32| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (x + dx, y + dy))))
        .find(|&(ox, oy)| g.map.plot_xy(ox, oy).is_some_and(|q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none()))
        .expect("land near the start");
    let (w, a) = (d.t::<UnitType>("UNIT_WARRIOR"), d.t::<UnitType>("UNIT_ARCHER"));
    let units: Vec<UnitId> = [w, w, w, w, a, a, a].into_iter().map(|t| g.player_init_unit(H, t, ox, oy, None, None)).collect();
    let army = g.army_form(H, &units).expect("formed");
    g.player_army_sync(H);
    (g, army, units)
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

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

fn flag(j: &Json, key: &str) -> bool {
    match j.get(key) {
        Some(Json::Bool(b)) => *b,
        _ => panic!("`{key}` in {j}"),
    }
}

/// The `group` query's squads for `units`.
fn squads(g: &Game, units: &[UnitId]) -> Vec<Json> {
    let grp = view::units::group(&Viewer::new(g, H, false), units).unwrap();
    grp.get("squads").and_then(Json::as_arr).unwrap_or_else(|| panic!("squads in {grp}")).to_vec()
}

/// A squad's cells as unit ids.
fn cells(sq: &Json) -> Vec<Option<u32>> {
    let c = sq.get("cells").and_then(Json::as_arr).expect("cells");
    assert_eq!(c.len(), 9, "{sq}");
    c.iter().map(|x| x.as_i64().map(|v| v as u32)).collect()
}

/// The board as the views show it agrees with itself: every unit stands in exactly one listed
/// squad, in the cell its own view names, with that squad's id, number and Auto flag; numbers run
/// 1, 2, ... in order; each squad's places count its units.
fn check_consistent(g: &Game, units: &[UnitId]) -> Vec<Json> {
    let list = squads(g, units);
    let v = Viewer::new(g, H, false);
    for (i, sq) in list.iter().enumerate() {
        assert_eq!(int(sq, "num"), i as i64 + 1, "numbers in order: {sq}");
        assert_eq!(sq.get("kind").and_then(Json::as_str), Some("army"), "{sq}");
        let n = cells(sq).iter().flatten().count() as i64;
        assert_eq!(int(sq, "places"), n, "{sq}");
        assert!(n >= 1 && n <= int(sq, "max_places"), "{sq}");
    }
    for &u in units {
        let holding: Vec<(usize, usize)> = list.iter().enumerate().flat_map(|(i, sq)| cells(sq).into_iter().enumerate().filter(move |(_, c)| *c == Some(u.0)).map(move |(c, _)| (i, c))).collect();
        assert_eq!(holding.len(), 1, "unit {} stands once: {list:?}", u.0);
        let (i, c) = holding[0];
        let us = view::unit_json(&v, u);
        let s = us.get("squad").unwrap_or_else(|| panic!("squad in {us}"));
        assert_eq!(int(s, "id"), int(&list[i], "id"), "{us}");
        assert_eq!(int(s, "num"), int(&list[i], "num"), "{us}");
        assert_eq!(int(s, "cell"), c as i64, "{us}");
        assert_eq!(flag(s, "auto"), flag(&list[i], "auto"), "{us}");
    }
    list
}

/// The squad (index in `list`) that holds unit `u`, and its cell.
fn where_is(list: &[Json], u: UnitId) -> (usize, usize) {
    list.iter().enumerate().find_map(|(i, sq)| cells(sq).iter().position(|&c| c == Some(u.0)).map(|c| (i, c))).expect("in a squad")
}

#[test]
fn squad_commands_change_the_board_the_views_show() {
    let (mut g, army, units) = game();
    let list = check_consistent(&g, &units);
    assert!(list.iter().all(|sq| flag(sq, "auto")), "new squads are Auto: {list:?}");
    // The Armies screen lists the army's squads, as the group query does.
    let screen = view::armies::screen(&Viewer::new(&g, H, false));
    let e = screen.get("armies").and_then(Json::as_arr).unwrap().iter().find(|j| int(j, "id") == army.0 as i64).unwrap().clone();
    assert_eq!(e.get("squads").and_then(Json::as_arr).map(|a| a.len()), Some(list.len()), "{e}");

    // A unit to a new squad of its own: both squads hand-set.
    let u = units[0];
    let (from, _) = where_is(&list, u);
    let from_id = SquadId(int(&list[from], "id") as u32);
    run(&mut g, &Command::SquadMove { unit: u, squad: None, cell: None }).unwrap();
    let list = check_consistent(&g, &units);
    let (i, _) = where_is(&list, u);
    assert_eq!(cells(&list[i]).iter().flatten().count(), 1, "a squad of one: {list:?}");
    assert!(!flag(&list[i], "auto"), "{list:?}");
    let old = list.iter().find(|sq| int(sq, "id") == from_id.0 as i64).expect("the old squad has units left");
    assert!(!flag(old, "auto"), "{old}");
    // Alone in its squad, it cannot form another one.
    assert!(refused(run(&mut g, &Command::SquadMove { unit: u, squad: None, cell: None })).contains("already a squad of its own"));

    // Back into the old squad, in a free cell named: it stands there.
    let free = cells(old).iter().position(|c| c.is_none()).expect("a free cell");
    run(&mut g, &Command::SquadMove { unit: u, squad: Some(from_id), cell: Some(free as u8) }).unwrap();
    let list = check_consistent(&g, &units);
    let (i, c) = where_is(&list, u);
    assert_eq!((int(&list[i], "id"), c), (from_id.0 as i64, free), "{list:?}");
    // Onto a unit of the same size: the two swap.
    let other = cells(&list[i]).iter().enumerate().find_map(|(c, x)| x.filter(|&x| x != u.0).map(|x| (c, UnitId(x)))).expect("another unit");
    run(&mut g, &Command::SquadMove { unit: u, squad: Some(from_id), cell: Some(other.0 as u8) }).unwrap();
    let list = check_consistent(&g, &units);
    assert_eq!(where_is(&list, u).1, other.0, "{list:?}");
    assert_eq!(where_is(&list, other.1).1, free, "{list:?}");
    // Refusals carry the rules' words and change nothing.
    let before = list.clone();
    assert!(refused(run(&mut g, &Command::SquadMove { unit: u, squad: Some(from_id), cell: Some(9) })).contains("outside the squad"));
    assert!(refused(run(&mut g, &Command::SquadMove { unit: u, squad: Some(SquadId(9999)), cell: None })).contains("no such squad"));
    assert!(matches!(run(&mut g, &Command::SquadAuto { squad: SquadId(9999) }), Err(CmdErr::Cannot(_))));
    assert!(matches!(run(&mut g, &Command::SquadSort { squads: vec![] }), Err(CmdErr::Invalid(_))));
    assert_eq!(squads(&g, &units), before);

    // Auto: the squad is the auto formation's again.
    run(&mut g, &Command::SquadAuto { squad: from_id }).unwrap();
    let list = check_consistent(&g, &units);
    assert!(flag(list.iter().find(|sq| int(sq, "id") == from_id.0 as i64).unwrap(), "auto"), "{list:?}");

    // Auto-sort of every squad: as few squads as hold the units, all Auto.
    let ids: Vec<SquadId> = list.iter().map(|sq| SquadId(int(sq, "id") as u32)).collect();
    run(&mut g, &Command::SquadSort { squads: ids }).unwrap();
    let list = check_consistent(&g, &units);
    let cap = g.d.defines.engagement_squad_places as usize;
    assert_eq!(list.len(), units.len().div_ceil(cap), "{list:?}");
    assert!(list.iter().all(|sq| flag(sq, "auto")), "{list:?}");
}
