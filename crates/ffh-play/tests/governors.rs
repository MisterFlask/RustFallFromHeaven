//! This game's governors through the human's command layer and views (docs/GOVERNORS.md, the
//! protocol in docs/HUMAN.md), on hand-built situations: the Governors screen of cities run by
//! hand, of a mixed empire and under a governed government, cities handed over and taken back by
//! command, roles and priorities set by commands, empire orders placed
//! and taken off (by their own commands and through the city screen's `push_order` /
//! `pop_order`), the two steps of an empire order (`empire_items`, `empire_cities`) and the civics
//! preview of who will run the cities.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::command::{self, CmdErr, Command, Ctx, Plan};
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

const H: PlayerId = PlayerId(0);

/// A duel map after `game_init`: player 0's first city `a` and a second city `b` 4 to 6 plots
/// away on the same landmass, the starting units gone, under Despotism (both run by hand).
fn game() -> (Game, CityId, CityId) {
    let d = ffh_data::global();
    let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
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
    'find: for r in 4..=6 {
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

/// The human adopts government civic `name`.
fn government(g: &mut Game, name: &str) {
    let c = g.d.t::<CivicType>(name);
    let option = g.d.civics[c].civic_option_type.expect("a civic option");
    g.player_set_civics(H, option, Some(c));
}

fn warrior(g: &Game) -> UnitType {
    g.d.t::<UnitType>("UNIT_WARRIOR")
}

/// A unit that serves in armies which both cities can train now (the warrior of most
/// civilizations).
fn soldier(g: &Game, a: CityId, b: CityId) -> UnitType {
    g.d.units
        .keys()
        .find(|&u| g.unit_type_is_army_unit(u) && [a, b].iter().all(|&c| g.city_offers_unit(c, u) && g.city_can_train(c, u, false, false, false, false)))
        .expect("a unit both cities train")
}

/// Validates `cmd` for the human and applies it (the rules' own refusal, if any, as `Cannot`).
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

fn flag(j: &Json, key: &str) -> bool {
    j.get(key).and_then(Json::as_bool).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

fn screen(g: &Game) -> Json {
    view::governors::screen(&Viewer::new(g, H, false))
}

/// The Governors screen's entry of city `c`.
fn screen_city(g: &Game, c: CityId) -> Json {
    arr(&screen(g), "cities").into_iter().find(|j| int(j, "city") == c.0 as i64).expect("the city on the screen")
}

#[test]
fn cities_run_by_hand_show_the_mode_and_the_civics_preview_says_who_will_run_the_cities() {
    let (mut g, a, b) = game();
    let s = screen(&g);
    assert!(!flag(&s, "civics_govern"), "{s}");
    assert_eq!(int(&s, "governed"), 0, "{s}");
    assert_eq!(int(&s, "government"), g.d.t::<CivicType>("CIVIC_DESPOTISM").idx() as i64);
    let cities = arr(&s, "cities");
    assert_eq!(cities.len(), 2);
    assert_eq!(int(&cities[0], "city"), a.0 as i64, "the capital first");
    assert!(cities.iter().all(|c| !flag(c, "governed") && c.get("card").is_none()), "{s}");
    let gv = view::governors::city(&Viewer::new(&g, H, false), b);
    assert!(!flag(&gv, "governed"), "{gv}");
    // The civics preview: Aristocracy hands both cities to governors; God King changes nothing.
    let aristocracy = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
    let p = view::player::civics(&Viewer::new(&g, H, false), &[aristocracy]);
    let proposed = p.get("proposed").cloned().unwrap();
    assert_eq!(proposed.get("rule_change").and_then(Json::as_str), Some("governed"), "{p}");
    assert_eq!(int(&proposed, "cities"), 2, "{p}");
    let god_king = g.d.t::<CivicType>("CIVIC_GOD_KING");
    let p = view::player::civics(&Viewer::new(&g, H, false), &[god_king]);
    assert_eq!(p.get("proposed").and_then(|x| x.get("rule_change")), Some(&Json::Null), "{p}");
    // Under Aristocracy, Republic's preview: the cities stay governed.
    government(&mut g, "CIVIC_ARISTOCRACY");
    let republic = g.d.t::<CivicType>("CIVIC_REPUBLIC");
    let p = view::player::civics(&Viewer::new(&g, H, false), &[republic]);
    assert_eq!(p.get("proposed").and_then(|x| x.get("rule_change")).and_then(Json::as_str), Some("stay"), "{p}");
    government(&mut g, "CIVIC_DESPOTISM");
    for c in [a, b] {
        run(&mut g, &Command::GovernCity { city: c, governed: false }).unwrap();
    }
    // Empire orders need a governed city.
    let w = warrior(&g).idx() as i32;
    assert!(refused(run(&mut g, &Command::EmpireOrder { city: a, order: OrderType::ORDER_TRAIN, data: w })).contains("governed"));
    let items = view::governors::empire_items(&Viewer::new(&g, H, false)).unwrap();
    assert_eq!(int(&items, "governed"), 0, "{items}");
}

#[test]
fn a_city_is_handed_over_and_taken_back_by_command_and_the_screens_show_a_mixed_empire() {
    let (mut g, a, b) = game();
    government(&mut g, "CIVIC_REPUBLIC");
    // Run by hand under Republic; the city screen offers the hand-over.
    let gv = view::governors::city(&Viewer::new(&g, H, false), b);
    assert!(!flag(&gv, "governed") && !flag(&gv, "civics_govern") && gv.get("toggle_why") == Some(&Json::Null), "{gv}");
    run(&mut g, &Command::GovernCity { city: b, governed: true }).unwrap();
    assert!(g.city_is_governed(b) && !g.city_is_governed(a));
    // Twice: refused, with the reason.
    assert!(refused(run(&mut g, &Command::GovernCity { city: b, governed: true })).contains("run by a governor"));
    // The Governors screen of a mixed empire: one governed with its card, one run by hand.
    let s = screen(&g);
    assert_eq!(int(&s, "governed"), 1, "{s}");
    let (ea, eb) = (screen_city(&g, a), screen_city(&g, b));
    assert!(!flag(&ea, "governed") && ea.get("card").is_none() && ea.get("toggle_why") == Some(&Json::Null), "{ea}");
    assert!(flag(&eb, "governed") && eb.get("card").is_some() && eb.get("toggle_why") == Some(&Json::Null), "{eb}");
    // Empire orders: from the governed city only.
    let items = view::governors::empire_items(&Viewer::new(&g, H, false)).unwrap();
    assert_eq!(int(&items, "governed"), 1, "{items}");
    // Taken back.
    run(&mut g, &Command::GovernCity { city: b, governed: false }).unwrap();
    assert!(!g.city_is_governed(b));
    // Under City States every city is governed, and taking one back is refused with the civic's reason.
    government(&mut g, "CIVIC_CITY_STATES");
    assert!(g.city_is_governed(a) && g.city_is_governed(b));
    let why = refused(run(&mut g, &Command::GovernCity { city: a, governed: false }));
    assert_eq!(why, "City States: every city is run by a governor.");
    let gv = view::governors::city(&Viewer::new(&g, H, false), a);
    assert!(flag(&gv, "civics_govern") && gv.get("toggle_why").and_then(Json::as_str) == Some(why.as_str()), "{gv}");
    // Malformed: another's or no city.
    assert!(matches!(run(&mut g, &Command::GovernCity { city: CityId(9999), governed: true }), Err(CmdErr::Gone)));
}

#[test]
fn a_governed_government_gives_every_city_a_role_card_and_roles_and_priorities_are_set_by_commands() {
    let (mut g, a, b) = game();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let s = screen(&g);
    assert!(flag(&s, "civics_govern"), "{s}");
    let n_roles = g.d.city_roles.len();
    for c in [a, b] {
        let e = screen_city(&g, c);
        assert!(flag(&e, "governed"), "{e}");
        let card = e.get("card").expect("a role card");
        assert_eq!(arr(card, "roles").len(), n_roles, "{card}");
        assert_eq!(arr(card, "key_buildings").len(), 4, "{card}");
        assert!(card.get("plan").and_then(Json::as_arr).is_some(), "{card}");
    }
    // The city screen's governance names the role.
    let gv = view::governors::city(&Viewer::new(&g, H, false), b);
    assert!(flag(&gv, "governed") && gv.get("role").and_then(Json::as_i64).is_some(), "{gv}");

    // Roles: every role Standard, then one Lesser: below 75% it cannot be chosen (the card says why).
    for k in g.d.building_classes.keys() {
        g.cities[b].building_tiers[k] = BuildingTierType::STANDARD;
    }
    let low = g
        .d
        .city_roles
        .keys()
        .find(|&r| {
            let civ = g.city_civilization_type(b);
            g.d.city_roles[r].key_buildings.iter().any(|&k| g.d.civ_building(civ, k).is_some())
        })
        .expect("a role the city's civilization builds for");
    for k in g.d.city_roles[low].key_buildings.clone() {
        g.cities[b].building_tiers[k] = BuildingTierType::LESSER;
    }
    assert!(g.city_role_suitability(b, low) < 75);
    if g.city_role_proposal(b) != low {
        let why = refused(run(&mut g, &Command::GovernRole { city: b, role: low }));
        assert!(why.contains("75%"), "{why}");
        let card = screen_city(&g, b).get("card").cloned().unwrap();
        let entry = arr(&card, "roles").into_iter().find(|x| int(x, "role") == low.idx() as i64).unwrap();
        assert!(!flag(&entry, "allowed") && entry.get("why").and_then(Json::as_str).is_some_and(|s| s.contains("75%")), "{entry}");
    }
    // Another role at 100%: chosen, overruled; the proposal again: proposed.
    let proposal = g.city_role_proposal(b);
    let other = g.d.city_roles.keys().find(|&r| r != proposal && g.city_role_suitability(b, r) >= 75).expect("another role");
    run(&mut g, &Command::GovernRole { city: b, role: other }).unwrap();
    let e = screen_city(&g, b);
    assert_eq!((int(&e, "role"), flag(&e, "role_overridden")), (other.idx() as i64, true), "{e}");
    run(&mut g, &Command::GovernRole { city: b, role: proposal }).unwrap();
    assert!(!flag(&screen_city(&g, b), "role_overridden"));
    assert!(matches!(run(&mut g, &Command::GovernRole { city: b, role: CityRoleType::from_usize(n_roles) }), Err(CmdErr::Invalid(_))));

    // Priorities: two at most, distinct.
    let r = |i: usize| CityRoleType::from_usize(i);
    run(&mut g, &Command::GovernPriorities { roles: vec![r(1), r(4)] }).unwrap();
    assert_eq!(g.players[H].governor_priorities, vec![r(1), r(4)]);
    assert_eq!(arr(&screen(&g), "priorities"), vec![Json::from(1), Json::from(4)]);
    assert!(refused(run(&mut g, &Command::GovernPriorities { roles: vec![r(0), r(1), r(2)] })).contains("At most"));
    assert!(refused(run(&mut g, &Command::GovernPriorities { roles: vec![r(2), r(2)] })).contains("twice"));
    run(&mut g, &Command::GovernPriorities { roles: vec![] }).unwrap();
    assert!(g.players[H].governor_priorities.is_empty());
}

#[test]
fn empire_orders_are_placed_and_taken_off_and_the_city_screen_offers_only_them() {
    let (mut g, a, b) = game();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let d = g.d;
    let w = soldier(&g, a, b);
    // The governor's own order in b (the controller of a bare game chooses nothing).
    if g.cities[b].order_queue.is_empty() {
        g.city_push_order(b, OrderType::ORDER_TRAIN, w.idx() as i32, -1, false, false, false, false);
    }
    let own = g.cities[b].order_queue.len();
    // The city screen lists only what may be an empire order, and no process.
    let cs = view::city::city(&Viewer::new(&g, H, false), b).unwrap();
    for x in arr(&cs, "buildable") {
        let order = OrderType::from_usize(int(&x, "order") as usize);
        assert!(order != OrderType::ORDER_MAINTAIN, "{x}");
        assert!(g.order_may_be_empire(order, int(&x, "type") as i32), "{x}");
    }
    // An ordinary building and a process are refused, with the rules' reason.
    let plain = d.buildings.keys().find(|&x| !g.order_may_be_empire(OrderType::ORDER_CONSTRUCT, x.idx() as i32) && g.city_can_construct(b, x, false, false, false));
    if let Some(x) = plain {
        let why = refused(run(&mut g, &Command::PushOrder { city: b, order: OrderType::ORDER_CONSTRUCT, data: x.idx() as i32, alt: false, shift: false, ctrl: false }));
        assert!(why.contains("Empire orders"), "{why}");
    }
    assert!(refused(run(&mut g, &Command::PushOrder { city: b, order: OrderType::ORDER_MAINTAIN, data: 0, alt: false, shift: false, ctrl: false })).contains("governor"));
    // A unit through the city screen's push_order: an empire order ahead of the governor's.
    run(&mut g, &Command::PushOrder { city: b, order: OrderType::ORDER_TRAIN, data: w.idx() as i32, alt: false, shift: true, ctrl: false }).unwrap();
    assert_eq!(g.cities[b].order_queue.len(), own + 1);
    assert!(g.cities[b].order_queue[0].empire);
    let q = arr(&view::city::city(&Viewer::new(&g, H, false), b).unwrap(), "queue");
    assert!(flag(&q[0], "empire") && !flag(&q[1], "empire"), "{q:?}");
    // The governor's order cannot be taken off; the empire order can (pop_order and its own command).
    let why = refused(run(&mut g, &Command::PopOrder { city: b, index: 1 }));
    assert!(why.contains("governor"), "{why}");
    run(&mut g, &Command::PopOrder { city: b, index: 0 }).unwrap();
    assert_eq!(g.cities[b].order_queue.len(), own);
    assert!(!g.cities[b].order_queue.iter().any(|o| o.empire));
    run(&mut g, &Command::EmpireOrder { city: b, order: OrderType::ORDER_TRAIN, data: w.idx() as i32 }).unwrap();
    let card = screen_city(&g, b).get("card").cloned().unwrap();
    let orders = arr(&card, "orders");
    assert_eq!(orders.len(), 1, "{card}");
    assert!(flag(&orders[0], "empire") && int(&orders[0], "turns") > 0, "{card}");
    run(&mut g, &Command::EmpireOrderRemove { city: b, index: 0 }).unwrap();
    assert!(refused(run(&mut g, &Command::EmpireOrderRemove { city: b, index: 0 })).contains("governor"));
    // In the other city too; taken back (Despotism first: leaving Aristocracy changes no city) its
    // empire order is an ordinary order.
    run(&mut g, &Command::EmpireOrder { city: a, order: OrderType::ORDER_TRAIN, data: w.idx() as i32 }).unwrap();
    government(&mut g, "CIVIC_DESPOTISM");
    assert!(g.cities[a].order_queue.iter().any(|o| o.empire));
    run(&mut g, &Command::GovernCity { city: a, governed: false }).unwrap();
    assert!(!g.cities[a].order_queue.iter().any(|o| o.empire));
    assert!(refused(run(&mut g, &Command::EmpireOrder { city: a, order: OrderType::ORDER_TRAIN, data: w.idx() as i32 })).contains("governed"));
}

#[test]
fn an_empire_order_lists_the_capable_cities_and_suggests_the_fewest_turns() {
    let (mut g, a, b) = game();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let w = soldier(&g, a, b);
    let v = Viewer::new(&g, H, false);
    // Step 1: the soldier from both cities; every entry an empire order.
    let items = view::governors::empire_items(&v).unwrap();
    assert_eq!(int(&items, "governed"), 2);
    let list = arr(&items, "items");
    let wi = list.iter().find(|x| int(x, "type") == w.idx() as i64 && int(x, "order") == OrderType::ORDER_TRAIN.idx() as i64).expect("the soldier");
    assert_eq!(int(wi, "cities"), 2, "{wi}");
    assert!(flag(wi, "army_unit"), "{wi}");
    for x in list {
        assert!(g.order_may_be_empire(OrderType::from_usize(int(&x, "order") as usize), int(&x, "type") as i32), "{x}");
    }
    // Step 2: both cities, one suggestion: the fewest turns.
    let cities = view::governors::empire_cities(&v, OrderType::ORDER_TRAIN, w.idx() as i32).unwrap();
    let rows = arr(&cities, "cities");
    assert_eq!(rows.len(), 2, "{cities}");
    assert!(rows.iter().all(|x| int(x, "turns") >= int(x, "own_turns")), "{cities}");
    let suggested: Vec<&Json> = rows.iter().filter(|x| flag(x, "suggested")).collect();
    assert_eq!(suggested.len(), 1, "{cities}");
    let best = rows.iter().map(|x| int(x, "turns")).min().unwrap();
    assert_eq!(int(suggested[0], "turns"), best, "{cities}");
    // Malformed: a process, data out of range.
    assert!(view::governors::empire_cities(&v, OrderType::ORDER_MAINTAIN, 0).is_err());
    assert!(view::governors::empire_cities(&v, OrderType::ORDER_TRAIN, 100_000).is_err());
}
