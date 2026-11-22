//! Trade as the human sees and commands it (docs/TRADE.md, section 9; docs/HUMAN.md, "Trade"):
//! `establish_route` and `dissolve_route` with the rules' refusals, the Establish action, the
//! `route_offers` query, the view's `trade` (the routes, the others' routes into the human's
//! cities, the limits, the Trade layer's lines), the cities' routes, the caravans' fields and
//! the attack preview's plunder, the help lines; on the staged saves (`stage trade-ready |
//! trade-routes | trade-closed | trade-raid`) and on what the test changes by hand. Nothing asserts how a
//! game goes; no leak of unrevealed plots, cities or pay.

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;
use ffh_play::view::{self, Viewer};
use ffh_play::{Json, Session, SessionOptions};

/// The staged save `kind`, loaded, with the stage's reply.
fn staged(kind: &str) -> (Session, Json, std::path::PathBuf) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("ffh-trade-{kind}-{}-{n}", std::process::id()));
    let fixture = ffh_play::stage::trade_scene(&root, kind).unwrap();
    let bytes = std::fs::read(fixture.req::<String>("save").unwrap()).unwrap();
    let s = Session::load(&bytes, None, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    (s, fixture, root)
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

fn trade_view(s: &Session) -> Json {
    view::trade::trade_json(&Viewer::new(&s.g, s.human, false))
}

fn route_row(s: &Session, r: RouteId) -> Json {
    arr(&trade_view(s), "routes").into_iter().find(|j| j.get("id") == Some(&Json::from(r))).unwrap_or_else(|| panic!("route {} in the view", r.0))
}


/// Sends a command the client would send (its JSON), as the client does.
fn send(s: &mut Session, cmd: &Json) -> Result<CmdOk, CmdErr> {
    let c = Command::from_json(s.g.d, cmd).unwrap();
    s.execute(&c)
}

fn refused(r: Result<CmdOk, CmdErr>) -> String {
    match r {
        Err(CmdErr::Cannot(why)) => why,
        other => panic!("refused with a reason, got {other:?}"),
    }
}

fn offers(s: &Session, u: UnitId) -> Json {
    s.query(&Query::RouteOffers { unit: u }).unwrap()
}

fn xy_json(x: i32, y: i32) -> Json {
    Json::Arr(vec![Json::from(x), Json::from(y)])
}

#[test]
fn establish_from_the_unit_panel_through_the_dialog() {
    let (mut s, st, root) = staged("trade-ready");
    let u: UnitId = st.req("caravan").unwrap();
    let (capital, port, far): (CityId, CityId, CityId) = (st.req("capital").unwrap(), st.req("port").unwrap(), st.req("far").unwrap());
    let v = Viewer::new(&s.g, s.human, false);
    // The unit panel's action: enabled, no command of its own (the dialog sends it).
    let actions = arr(&view::units::group(&v, &[u]).unwrap(), "actions");
    let a = actions.iter().find(|a| a.get("kind").and_then(Json::as_str) == Some("trade")).expect("the Establish action");
    assert_eq!(text(a, "label"), "Establish route");
    assert_eq!(a.get("enabled"), Some(&Json::from(true)));
    assert_eq!(a.get("cmd"), Some(&Json::Null));
    assert!(a.get("why").is_none());
    // The dialog: the limits line and one row per destination, by pay.
    let o = offers(&s, u);
    assert_eq!(text(&o, "limits"), "Routes: 1 of 2. This city: 0 of 1.");
    assert_eq!(o.get("origin").and_then(|j| j.get("id")), Some(&Json::from(capital)));
    let rows = arr(&o, "offers");
    assert_eq!(rows.len(), 2, "{o}");
    let pays: Vec<i64> = rows.iter().map(|r| r.get("pay_x100").and_then(Json::as_i64).unwrap()).collect();
    assert!(pays.windows(2).all(|w| w[0] >= w[1]), "by pay: {pays:?}");
    let foreign = rows.iter().find(|r| r.get("city") == Some(&Json::from(far))).unwrap();
    let rival: PlayerId = st.req("rival").unwrap();
    let civ = s.g.player_get_civilization_short_description(rival);
    let far_name = view::city_name(&s.g, far);
    let t = text(foreign, "text");
    assert!(t.starts_with(&format!("{far_name} ({civ}). ")), "{t}");
    assert!(t.contains(" plots. ") && t.contains(" a turn."), "{t}");
    assert!(t.ends_with(" Passes Strait of Lorda: open."), "{t}");
    let seas = arr(foreign, "seas");
    assert_eq!(text(&seas[0], "name"), "Strait of Lorda");
    assert_eq!(text(&seas[0], "state"), "open");
    let home = rows.iter().find(|r| r.get("city") == Some(&Json::from(port))).unwrap();
    assert_eq!(text(home, "text"), format!("{}. 6 plots. {} a turn.", view::city_name(&s.g, port), text(home, "pay")));
    assert_eq!(home.get("civ"), Some(&Json::Null));
    assert!(!arr(home, "segments").is_empty());
    // Choosing a row sends its command.
    let cmd = home.get("cmd").unwrap().clone();
    assert!(matches!(send(&mut s, &cmd).unwrap(), CmdOk::Applied { .. }));
    let r = s.g.units[u].route.expect("in a route");
    assert_eq!((s.g.routes[r].origin, s.g.routes[r].destination), (capital, port));
    assert!(s.messages().iter().any(|m| m.text.starts_with("Route established: ")), "a message");
    // Now in a route: no Establish action, no orders, the panel says why.
    let g = view::units::group(&Viewer::new(&s.g, s.human, false), &[u]).unwrap();
    assert!(arr(&g, "actions").is_empty());
    assert_eq!(text(&g, "in_route"), "A caravan in a route takes no orders.");
    let panel = view::units::unit_details(&Viewer::new(&s.g, s.human, false), u);
    let lines: Vec<String> = arr(panel.get("trade").unwrap(), "lines").iter().map(|l| l.as_str().unwrap().to_string()).collect();
    assert_eq!(lines[0], format!("Route: {} to {}.", view::city_name(&s.g, capital), view::city_name(&s.g, port)));
    assert!(lines[1].starts_with("In ") && lines[1].contains(", bound for "), "{lines:?}");
    assert_eq!(lines[2], "Cargo: none.");
    assert_eq!(lines[3], "Plunder: 0 gold.");
    assert_eq!(trade_view(&s).get("limits").and_then(|l| l.get("text")).and_then(Json::as_str), Some("Routes: 2 of 2."));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn establish_and_dissolve_are_refused_with_the_rules_reasons() {
    let (mut s, st, root) = staged("trade-ready");
    let u: UnitId = st.req("caravan").unwrap();
    let (capital, far): (CityId, CityId) = (st.req("capital").unwrap(), st.req("far").unwrap());
    let est = |u: UnitId, c: CityId| Command::EstablishRoute { unit: u, city: c, cargo: [None, None] };
    assert_eq!(refused(s.execute(&est(u, capital))), "A route links two cities.");
    // A cargo: refused until cargo is built; the member's shape is read.
    // Cargo (docs/TRADE.md 5): the same resource twice; a cargo on a domestic route.
    let fur = s.g.d.t::<BonusType>("BONUS_FUR");
    assert_eq!(refused(s.execute(&Command::EstablishRoute { unit: u, city: far, cargo: [Some(fur), Some(fur)] })), "The two caravans carry different resources.");
    let port: CityId = st.req("port").unwrap();
    assert_eq!(refused(s.execute(&Command::EstablishRoute { unit: u, city: port, cargo: [Some(fur), None] })), "A domestic route carries no cargo.");
    let j = Json::parse(&format!(r#"{{"c":"establish_route","unit":{},"city":{}}}"#, u.0, far.0)).unwrap();
    assert_eq!(Command::from_json(s.g.d, &j).unwrap(), est(u, far));
    let bad = Json::parse(&format!(r#"{{"c":"establish_route","unit":{},"city":{},"cargo":[null]}}"#, u.0, far.0)).unwrap();
    assert!(Command::from_json(s.g.d, &bad).is_err());
    // A unit that is not a Caravan; another player's; a city that is gone.
    let (cx, cy) = (s.g.cities[capital].x, s.g.cities[capital].y);
    let warrior = s.g.player_init_unit(s.human, s.g.d.t("UNIT_WARRIOR"), cx, cy, None, None);
    assert_eq!(refused(s.execute(&est(warrior, far))), "Only a Caravan establishes a route.");
    let theirs: UnitId = st.req("theirs_unit").unwrap();
    assert!(matches!(s.execute(&est(theirs, far)), Err(CmdErr::NotOwner)));
    assert!(matches!(s.execute(&est(u, CityId(999))), Err(CmdErr::Gone)));
    // Out of a city: the action shown disabled with the reason; the dialog says it too.
    let q = s.g.map.plot_xy(cx - 2, cy).unwrap();
    let (x, y) = s.g.map.xy(q);
    let ty = s.g.caravan_unit_type().unwrap();
    let out = s.g.player_init_unit(s.human, ty, x, y, None, None);
    let v = Viewer::new(&s.g, s.human, false);
    let a = arr(&view::units::group(&v, &[out]).unwrap(), "actions").into_iter().find(|a| a.get("kind").and_then(Json::as_str) == Some("trade")).unwrap();
    assert_eq!(a.get("enabled"), Some(&Json::from(false)));
    assert_eq!(text(&a, "why"), "A route starts in one of your cities.");
    let o = offers(&s, out);
    assert_eq!(o.get("enabled"), Some(&Json::from(false)));
    assert_eq!(text(&o, "why"), "A route starts in one of your cities.");
    assert!(arr(&o, "offers").is_empty());
    assert!(matches!(s.query(&Query::RouteOffers { unit: warrior }), Err(ffh_play::query::QueryErr::Invalid(_))));
    // Dissolve: the human's own route only.
    let foreign: RouteId = st.req("foreign").unwrap();
    let theirs_route: RouteId = st.req("theirs").unwrap();
    assert!(matches!(s.execute(&Command::DissolveRoute { route: theirs_route }), Err(CmdErr::NotOwner)));
    assert!(matches!(s.execute(&Command::DissolveRoute { route: RouteId(77) }), Err(CmdErr::Gone)));
    let j = Json::parse(&format!(r#"{{"c":"dissolve_route","route":{}}}"#, foreign.0)).unwrap();
    assert!(matches!(send(&mut s, &j).unwrap(), CmdOk::Applied { .. }));
    assert!(!s.g.routes.contains(foreign));
    assert!(arr(&trade_view(&s), "routes").is_empty());
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn the_trade_screen_rows_the_cities_and_the_caravans() {
    let (s, st, root) = staged("trade-routes");
    let (capital, port, far): (CityId, CityId, CityId) = (st.req("capital").unwrap(), st.req("port").unwrap(), st.req("far").unwrap());
    let (foreign, domestic, theirs): (RouteId, RouteId, RouteId) = (st.req("foreign").unwrap(), st.req("domestic").unwrap(), st.req("theirs").unwrap());
    let rival: PlayerId = st.req("rival").unwrap();
    let civ = s.g.player_get_civilization_short_description(rival);
    let (cn, pn, fname) = (view::city_name(&s.g, capital), view::city_name(&s.g, port), view::city_name(&s.g, far));
    let t = trade_view(&s);
    assert_eq!(text(t.get("limits").unwrap(), "text"), "Routes: 2 of 2.");
    // The foreign route: open, its pay and terms, one caravan on the road, one lost.
    let f = route_row(&s, foreign);
    assert_eq!(text(&f, "title"), format!("{pn} to {fname} ({civ})."));
    assert_eq!(text(&f, "state"), "Open.");
    let pay = s.g.routes[foreign].owner_pay_x100();
    assert_eq!(text(&f, "pay"), format!("Pay: {} a turn.", view::trade::pay_text(pay)));
    let terms: Vec<&str> = f.get("terms").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
    assert_eq!(terms[0], format!("Length: {} plots.", s.g.routes[foreign].length));
    assert!(terms.contains(&"Foreign: +25%.") && terms.contains(&"Over water: +25%."), "{terms:?}");
    let growth = s.g.routes[foreign].growth;
    assert_eq!(text(&f, "growth_text"), format!("Growth: +{growth}% of 50%."));
    let car = arr(&f, "caravans");
    assert_eq!(text(&car[0], "state"), "road");
    let t0 = text(&car[0], "text");
    assert!(t0.starts_with("On the road: plot ") || t0.starts_with("In "), "{}", car[0]);
    assert_eq!(text(&car[1], "state"), "lost");
    assert_eq!(text(&car[1], "text"), format!("Lost on turn {} to the barbarians. Re-forms in 10 turns.", s.g.game_turn()));
    assert_eq!(car[1].get("cargo"), Some(&Json::idx(s.g.d.t::<BonusType>("BONUS_SILK"))));
    // The domestic route: both caravans on the road, the plunder of each.
    let d = route_row(&s, domestic);
    assert_eq!(text(&d, "title"), format!("{cn} to {pn}."));
    for c in arr(&d, "caravans") {
        assert_eq!(text(&c, "state"), "road");
        assert!(text(&c, "text").starts_with("On the road: plot "), "{c}");
        let slot = c.get("slot").and_then(Json::as_i64).unwrap() as usize;
        let want = ffh_core::trade::round_x100(10 * s.g.routes[domestic].slots[slot].pay_x100);
        assert_eq!(c.get("plunder"), Some(&Json::from(want)));
    }
    // The others' route into the capital: what it brings there.
    let into = arr(&t, "into");
    assert_eq!(into.len(), 1);
    assert_eq!(into[0].get("id"), Some(&Json::from(theirs)));
    assert_eq!(text(&into[0], "text"), format!("From {fname} ({civ}) to {cn}."));
    assert_eq!(text(&into[0], "pay"), format!("{} a turn to {cn}.", view::trade::pay_text(s.g.routes[theirs].dest_pay_x100)));
    // The map's lines: every open route, the rival's too.
    let lines: Vec<Json> = arr(&t, "lines");
    let ids: Vec<&Json> = lines.iter().map(|l| l.get("route").unwrap()).collect();
    for r in [foreign, domestic, theirs] {
        assert!(ids.contains(&&Json::from(r)), "{ids:?}");
    }
    // The city panel: the routes from and into each city with the pay to it.
    let v = Viewer::new(&s.g, s.human, false);
    let cap = view::city::city(&v, capital).unwrap();
    let cr = cap.get("routes").unwrap();
    assert_eq!(text(&arr(cr, "from")[0], "text"), format!("To {pn}: {} a turn.", view::trade::pay_text(s.g.routes[domestic].owner_pay_x100())));
    assert_eq!(text(&arr(cr, "into")[0], "text"), format!("From {fname} ({civ}): {} a turn.", view::trade::pay_text(s.g.routes[theirs].dest_pay_x100)));
    assert_eq!(text(cr, "limit"), "This city: 1 of 1.");
    assert!(cap.get("trade_routes").is_none());
    // A caravan the human sees: its route and plunder, in the view's units.
    let tu: UnitId = st.req("theirs_unit").unwrap();
    let c = view::unit_json(&v, tu);
    let cj = c.get("caravan").unwrap();
    assert_eq!(text(cj, "text"), format!("Caravan: {fname} to {cn}."));
    let slot = s.g.routes[theirs].slots.iter().position(|x| x.unit == Some(tu)).unwrap();
    let gold = ffh_core::trade::round_x100(10 * s.g.routes[theirs].slots[slot].pay_x100);
    assert_eq!(text(cj, "plunder_text"), format!("Plunder: {gold} gold."));
    // A unit in no route has none.
    let w: UnitId = st.req("warrior").unwrap();
    assert!(view::unit_json(&v, w).get("caravan").is_none());
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_closed_route_names_its_reason_and_draws_no_line() {
    let (s, st, root) = staged("trade-closed");
    let foreign: RouteId = st.req("foreign").unwrap();
    let rival: PlayerId = st.req("rival").unwrap();
    let port: CityId = st.req("port").unwrap();
    let civ = s.g.player_get_civilization_short_description(rival);
    let f = route_row(&s, foreign);
    assert_eq!(f.get("open"), Some(&Json::from(false)));
    assert_eq!(text(&f, "state"), format!("Closed: no open borders with {civ}."));
    assert_eq!(text(&f, "closed_key"), "no_open_borders");
    assert!(f.get("terms").and_then(Json::as_arr).unwrap().is_empty());
    let car = arr(&f, "caravans");
    assert_eq!(text(&car[0], "text"), format!("Waits in {}: the route is closed.", view::city_name(&s.g, port)));
    assert_eq!(text(&car[1], "state"), "lost");
    let t = trade_view(&s);
    assert!(arr(&t, "lines").iter().all(|l| l.get("route") != Some(&Json::from(foreign))));
    let v = Viewer::new(&s.g, s.human, false);
    let pr = view::city::city(&v, port).unwrap();
    assert!(text(&arr(pr.get("routes").unwrap(), "from")[0], "text").ends_with(&format!("Closed: no open borders with {civ}.")));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn the_attack_preview_names_the_plunder() {
    let (s, st, root) = staged("trade-raid");
    let w: UnitId = st.req("warrior").unwrap();
    let target: UnitId = st.req("target").unwrap();
    let home: RouteId = st.req("home").unwrap();
    let (tx, ty): (i32, i32) = (st.req("tx").unwrap(), st.req("ty").unwrap());
    let v = Viewer::new(&s.g, s.human, false);
    let c = view::units::combat(&v, &[w], tx, ty).unwrap();
    assert_eq!(c.get("valid"), Some(&Json::from(true)), "{c}");
    // Every caravan of the route on that plot (the two may meet there).
    let here: Vec<usize> = (0..2).filter(|&i| s.g.routes[home].slots[i].unit.is_some_and(|x| s.g.unit_plot(x) == s.g.unit_plot(target))).collect();
    let gold: i32 = here.iter().map(|&i| view::trade::plunder(&s.g, home, i)).sum();
    assert!(gold > 0);
    let want = if here.len() == 1 { format!("Plunder: {gold} gold.") } else { format!("Plunder: {gold} gold ({} caravans).", here.len()) };
    let lines: Vec<&str> = c.get("plunder").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
    assert_eq!(lines, [want]);
    // At war the foreign routes are closed.
    let foreign: RouteId = st.req("foreign").unwrap();
    let civ = s.g.player_get_civilization_short_description(st.req("rival").unwrap());
    assert_eq!(text(&route_row(&s, foreign), "state"), format!("Closed: at war with {civ}."));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn nothing_leaks_unrevealed_plots_cities_or_pay() {
    let (mut s, st, root) = staged("trade-raid");
    let home: RouteId = st.req("home").unwrap();
    let far: CityId = st.req("far").unwrap();
    let team = s.g.players[s.human].team;
    // Another's domestic route: its line, no row, no pay figure.
    let t = trade_view(&s);
    assert!(arr(&t, "routes").iter().chain(arr(&t, "into").iter()).all(|r| r.get("id") != Some(&Json::from(home))));
    let line = arr(&t, "lines").into_iter().find(|l| l.get("route") == Some(&Json::from(home))).expect("its line");
    assert!(line.get("pay_x100").is_none() && line.get("pay").is_none());
    // The plots of its path the human has not revealed are cut out of its line.
    let path = s.g.routes[home].path.clone();
    let hidden = path[path.len() / 2];
    s.g.plot_set_revealed(hidden, team, false, false, None, false);
    let (hx, hy) = s.g.map.xy(hidden);
    let t = trade_view(&s);
    if let Some(line) = arr(&t, "lines").into_iter().find(|l| l.get("route") == Some(&Json::from(home))) {
        for seg in arr(&line, "segments") {
            assert!(seg.as_arr().unwrap().iter().all(|p| p != &xy_json(hx, hy)), "{seg}");
        }
    }
    // Its own routes too: a path is clipped to what the human has revealed.
    let foreign: RouteId = st.req("foreign").unwrap();
    let domestic: RouteId = st.req("domestic").unwrap();
    let dpath = s.g.routes[domestic].path.clone();
    s.g.plot_set_revealed(dpath[2], team, false, false, None, false);
    let (ax, ay) = s.g.map.xy(dpath[2]);
    let d = route_row(&s, domestic);
    assert_eq!(arr(&d, "segments").len(), 2, "{d}");
    assert!(arr(&d, "segments").iter().all(|seg| seg.as_arr().unwrap().iter().all(|p| p != &xy_json(ax, ay))));
    // A destination the human has not revealed: "a city", no coordinates.
    let fq = s.g.city_plot(far);
    s.g.plot_set_revealed(fq, team, false, false, None, false);
    let v = Viewer::new(&s.g, s.human, false);
    assert!(!v.city_revealed(far), "the city's plot unrevealed hides it");
    let f = route_row(&s, foreign);
    assert_eq!(text(f.get("destination").unwrap(), "name"), "a city");
    assert_eq!(f.get("destination").unwrap().get("x"), Some(&Json::Null));
    assert!(text(&f, "title").contains("a city"), "{f}");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn help_lines_say_what_the_keys_do_now() {
    let d = ffh_data::global();
    let strip = |t: &str| ffh_play::names::resolve_keys(d, t).replace("[KW=KEYWORD_ROUTE]", "").replace("[\\KW]", "");
    let civic = d.t::<ffh_data::CivicType>("CIVIC_FOREIGN_TRADE");
    let h = strip(&ffh_play::view::help::effects_of(d, "CivicType", civic.idx()).unwrap());
    assert!(h.contains("+1 route.") && h.contains("+1 route from a coastal city."), "{h}");
    let b = d.buildings.keys().find(|&b| d.buildings[b].trade_route_modifier == 50).unwrap();
    let h = strip(&ffh_play::view::help::effects_of(d, "BuildingType", b.idx()).unwrap());
    assert!(h.contains("+50% pay of the route from this city."), "{h}");
    let b = d.buildings.keys().find(|&b| d.buildings[b].trade_routes == 1).unwrap();
    let h = strip(&ffh_play::view::help::effects_of(d, "BuildingType", b.idx()).unwrap());
    assert!(h.contains("This city can be the origin of 1 more route."), "{h}");
    assert!(!h.to_lowercase().contains("trade route"), "{h}");
    let caravan = d.t::<ffh_data::UnitType>("UNIT_CARAVAN");
    let h = ffh_play::view::help::effects_of(d, "UnitType", caravan.idx()).unwrap();
    assert!(h.contains("[KW=KEYWORD_ROUTE]route[\\KW]"), "{h}");
    // It names the Route keyword: its reminder comes with it.
    let k = ffh_play::view::keywords::in_text(d, &h);
    assert_eq!(k.len(), 1);
    assert_eq!(ffh_play::view::keywords::name(d, k[0]), "Route");
}

fn find<'a>(list: &'a [Json], key: &str, val: &Json) -> &'a Json {
    list.iter().find(|j| j.get(key) == Some(val)).unwrap_or_else(|| panic!("{key} = {val} in {list:?}"))
}

#[test]
fn cargo_in_the_establish_dialog_and_its_order() {
    let (mut s, st, root) = staged("trade-ready");
    let u: UnitId = st.req("caravan").unwrap();
    let (port, far): (CityId, CityId) = (st.req("port").unwrap(), st.req("far").unwrap());
    let d = s.g.d;
    let (fur, gems, dye) = (d.t::<BonusType>("BONUS_FUR"), d.t::<BonusType>("BONUS_GEMS"), d.t::<BonusType>("BONUS_DYE"));
    let civ = s.g.player_get_civilization_short_description(st.req("rival").unwrap());
    let o = offers(&s, u);
    let rows = arr(&o, "offers");
    let foreign = find(&rows, "city", &Json::from(far));
    let home = find(&rows, "city", &Json::from(port));
    assert_eq!(home.get("cargo"), Some(&Json::Null), "a domestic route carries none");
    let cargo = foreign.get("cargo").unwrap();
    let opts = arr(cargo, "options");
    // A resource the recipient lacks (+150%), one it has (+50%), an only copy with its warning.
    let f = find(&opts, "bonus", &Json::idx(fur));
    assert_eq!(text(f, "text"), format!("Fur (luxury): {civ} has none. +150% on this caravan."));
    assert_eq!(f.get("warn"), Some(&Json::Null));
    let g = find(&opts, "bonus", &Json::idx(gems));
    assert_eq!(text(g, "text"), format!("Gems (luxury): {civ} has it. +50% on this caravan."));
    let dy = find(&opts, "bonus", &Json::idx(dye));
    assert_eq!(dy.get("only_copy"), Some(&Json::from(true)));
    assert_eq!(text(dy, "warn"), "Your only copy: your cities lose Dye.");
    // The preselection: a spare luxury the recipient lacks, never an only copy, never mana.
    assert_eq!(arr(cargo, "preselect"), vec![Json::idx(fur), Json::Null]);
    let halves: Vec<i64> = arr(cargo, "halves").iter().map(|j| j.as_i64().unwrap()).collect();
    assert_eq!(halves.iter().sum::<i64>(), foreign.get("pay_x100").and_then(Json::as_i64).unwrap());
    // Sent with its cargos: the route carries them.
    let mut cmd = foreign.get("cmd").unwrap().clone();
    cmd.set("cargo", Json::Arr(vec![Json::idx(fur), Json::idx(gems)]));
    assert!(matches!(send(&mut s, &cmd).unwrap(), CmdOk::Applied { .. }));
    let r = s.g.units[u].route.unwrap();
    assert_eq!(s.g.routes[r].slots[0].cargo, Some(fur));
    assert_eq!(s.g.routes[r].slots[1].cargo, Some(gems));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn cargo_lines_escorts_and_what_others_bring() {
    let (s, st, root) = staged("trade-routes");
    let (foreign, domestic): (RouteId, RouteId) = (st.req("foreign").unwrap(), st.req("domestic").unwrap());
    let escort: ArmyId = st.req("escort").unwrap();
    let f = route_row(&s, foreign);
    let car = arr(&f, "caravans");
    assert_eq!(text(&car[0], "cargo_text"), "Cargo: Gems. Delivered. Premium +150%.");
    assert_eq!(text(&car[0], "cargo_state"), "delivering");
    assert_eq!(text(&car[1], "cargo_text"), "Cargo: Silk. Caravan lost.");
    let d = route_row(&s, domestic);
    let dc = arr(&d, "caravans");
    assert_eq!(text(&dc[0], "cargo_text"), "Cargo: none.");
    let esc = arr(&dc[0], "escorts");
    assert_eq!(esc[0].get("army"), Some(&Json::from(escort)));
    assert!(arr(&dc[1], "escorts").is_empty());
    let into = arr(&trade_view(&s), "into");
    assert_eq!(text(&into[0], "brings"), "Brings: Silk.");
    // Another player's caravan the human sees: its cargo; one a viewer does not see: nothing.
    let tu: UnitId = st.req("theirs_unit").unwrap();
    let v = Viewer::new(&s.g, s.human, false);
    assert_eq!(text(view::unit_json(&v, tu).get("caravan").unwrap(), "cargo_text"), "Cargo: Silk.");
    let barbs = s.g.barbarian_player();
    let blind = Viewer::new(&s.g, barbs, false);
    if !blind.unit_visible(tu) {
        assert_eq!(view::trade::unit_caravan(&blind, tu), Json::Null);
    }
    // The owner's caravan panel names its cargo and its escort.
    let mine = s.g.routes[domestic].slots[0].unit.unwrap();
    let lines: Vec<String> = arr(view::units::unit_details(&v, mine).get("trade").unwrap(), "lines").iter().map(|l| l.as_str().unwrap().to_string()).collect();
    assert!(lines.contains(&"Cargo: none.".to_string()), "{lines:?}");
    assert!(lines.contains(&format!("Escort: {}.", s.g.armies[escort].name)), "{lines:?}");
    // A caravan on water is drawn as a ship; on land as itself.
    let galley = s.g.d.t::<UnitType>("UNIT_GALLEY");
    for (_, route) in s.g.routes.iter() {
        for c in route.caravans() {
            let water = s.g.plot_is_water(s.g.unit_plot(c));
            let art = view::unit_json(&v, c).get("art").cloned();
            assert_eq!(art, water.then(|| Json::idx(galley)), "caravan {}", c.0);
        }
    }
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn refit_from_the_unit_panel() {
    let (mut s, st, root) = staged("trade-routes");
    let u: UnitId = st.req("refit_caravan").unwrap();
    let foreign: RouteId = st.req("foreign").unwrap();
    let theirs: RouteId = st.req("theirs").unwrap();
    let d = s.g.d;
    let (fur, gems) = (d.t::<BonusType>("BONUS_FUR"), d.t::<BonusType>("BONUS_GEMS"));
    let v = Viewer::new(&s.g, s.human, false);
    let acts = arr(&view::units::group(&v, &[u]).unwrap(), "actions");
    let est = acts.iter().find(|a| a.get("trade").and_then(Json::as_str) == Some("establish")).unwrap();
    assert_eq!(est.get("enabled"), Some(&Json::from(false)), "the port is full");
    let refit = acts.iter().find(|a| a.get("trade").and_then(Json::as_str) == Some("refit")).expect("Refit route");
    assert_eq!(text(refit, "label"), "Refit route");
    assert_eq!(refit.get("enabled"), Some(&Json::from(true)));
    assert_eq!(refit.get("route"), Some(&Json::from(foreign)));
    let o = offers(&s, u);
    let rf = &arr(&o, "refits")[0];
    assert_eq!(text(rf, "line"), "The route keeps its growth. This Caravan is used up.");
    // It starts from the route's cargos.
    assert_eq!(arr(rf.get("cargo").unwrap(), "preselect"), vec![Json::idx(gems), Json::idx(d.t::<BonusType>("BONUS_SILK"))]);
    // Refused: another's route, the same resource twice; then done.
    assert!(matches!(s.execute(&Command::RefitRoute { unit: u, route: theirs, cargo: [None, None] }), Err(CmdErr::NotOwner)));
    assert_eq!(refused(s.execute(&Command::RefitRoute { unit: u, route: foreign, cargo: [Some(fur), Some(fur)] })), "The two caravans carry different resources.");
    let growth = s.g.routes[foreign].growth;
    let mut cmd = rf.get("cmd").unwrap().clone();
    cmd.set("cargo", Json::Arr(vec![Json::idx(fur), Json::Null]));
    assert!(matches!(send(&mut s, &cmd).unwrap(), CmdOk::Applied { .. }));
    assert_eq!((s.g.routes[foreign].slots[0].cargo, s.g.routes[foreign].slots[1].cargo), (Some(fur), None));
    assert_eq!(s.g.routes[foreign].growth, growth);
    assert!(!s.g.units.contains(u), "used up");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn escort_of_a_caravan_from_the_unit_panel() {
    let (mut s, st, root) = staged("trade-routes");
    let escort: ArmyId = st.req("escort").unwrap();
    let units: Vec<UnitId> = st.req("escort_units").unwrap();
    let (domestic, theirs): (RouteId, RouteId) = (st.req("domestic").unwrap(), st.req("theirs").unwrap());
    let (cn, pn) = (view::city_name(&s.g, st.req("capital").unwrap()), view::city_name(&s.g, st.req("port").unwrap()));
    let station = |s: &Session| -> Json {
        let all = view::armies::armies(&Viewer::new(&s.g, s.human, false));
        find(all.as_arr().unwrap(), "id", &Json::from(escort)).get("station").unwrap().clone()
    };
    assert_eq!(text(&station(&s), "text"), format!("Escorting Caravan: {cn} to {pn} (1)."));
    let acts = arr(&view::units::group(&Viewer::new(&s.g, s.human, false), &units).unwrap(), "actions");
    let army = |k: &str| acts.iter().find(|a| a.get("army").and_then(Json::as_str) == Some(k)).cloned();
    assert_eq!(text(&army("stop_escort").unwrap(), "label"), "Stop escorting");
    let other = army("escort_caravan").expect("Escort the other caravan");
    assert_eq!(text(&other, "label"), format!("Escort Caravan: {cn} to {pn} (2)"));
    assert_eq!(text(&other, "help"), "A caravan standing with its escort moves only as far as the escort can.");
    // Taken by its command (a land army: no "only a fleet" refusal), refused for another's route.
    assert!(matches!(send(&mut s, other.get("cmd").unwrap()).unwrap(), CmdOk::Applied { .. }));
    assert_eq!(s.g.armies[escort].station, ffh_core::army::Station::Escort(ffh_core::army::EscortTarget::Caravan(domestic, 1)));
    let bad = Command::ArmyStation { army: escort, station: ffh_core::army::Station::Escort(ffh_core::army::EscortTarget::Caravan(theirs, 0)) };
    assert_eq!(refused(s.execute(&bad)), "no such caravan to escort");
    // Waiting: the caravan lost.
    let c = s.g.routes[domestic].slots[1].unit.unwrap();
    s.g.unit_kill(c, false, None);
    assert_eq!(text(&station(&s), "wait"), "Waits: caravan lost.");
    let row = route_row(&s, domestic);
    assert_eq!(arr(&arr(&row, "caravans")[1], "escorts")[0].get("army"), Some(&Json::from(escort)));
    std::fs::remove_dir_all(root).ok();
}
