//! The sea, phase 2, as the human sees it (docs/NAVAL.md, section 10; `view/seas.rs`): the map's
//! `sea_id` array and the view's `seas` (revealed only), the states and badges (only ships the
//! human sees count), the harbours, the path preview's interception lines, the Patrol action and
//! the unit panel, the sea's panel and its send, the Seas screen, the supply ships' lanes and the
//! transport's refusal; on situations built by hand (seas cut by hand).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::path::{FinderKind, PathFinder};
use ffh_core::setup::GameSetup;
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::view::{self, Viewer};

/// A duel game on a block of grassland (x 1..=38, y 2..=9) between two bands of coast (y 0..=1
/// and 10..=11), at war; player 0 (the human) has a port on (5, 2) and has revealed x < 30 only.
/// The south band is cut into three seas: Bay of Lorda (x 0..=12, the port's), The Grey Narrows
/// (x 13..=25) and Sirens' Water (x 26..=39); the north band's x >= 31 is Hidden Sea (never
/// revealed).
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    bay: SeaId,
    narrows: SeaId,
    sirens: SeaId,
    hidden: SeaId,
    port: CityId,
}

fn world() -> World {
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
            if g.map.xy(q).0 < 30 {
                g.plot_set_revealed(q, t, true, false, None, false);
            }
        }
    }
    let band = |g: &Game, x0: i32, x1: i32, y0: i32, y1: i32| -> Vec<PlotIdx> { g.map.plot_ids().filter(|&q| { let (x, y) = g.map.xy(q); (x0..=x1).contains(&x) && (y0..=y1).contains(&y) }).collect() };
    let (b, n, s, h) = (band(&g, 0, 12, 0, 1), band(&g, 13, 25, 0, 1), band(&g, 26, 39, 0, 1), band(&g, 31, 39, 10, 11));
    let bay = g.map_cut_sea("Bay of Lorda", &b);
    let narrows = g.map_cut_sea("The Grey Narrows", &n);
    let sirens = g.map_cut_sea("Sirens' Water", &s);
    let hidden = g.map_cut_sea("Hidden Sea", &h);
    g.update_plot_groups();
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_meet(t0, t1, false);
    g.team_declare_war(t0, t1, false, None);
    g.players[p0].turn_active = true;
    let port = g.player_init_city(p0, 5, 2, true, true);
    World { g, p0, p1, bay, narrows, sirens, hidden, port }
}

impl World {
    fn at(&self, x: i32, y: i32) -> PlotIdx {
        self.g.map.plot_xy(x, y).expect("on the map")
    }

    fn unit(&mut self, owner: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.player_init_unit(owner, ty, x, y, None, None)
    }

    /// A Trireme of `owner` on Patrol at (x, y).
    fn patrol(&mut self, owner: PlayerId, x: i32, y: i32) -> UnitId {
        let u = self.unit(owner, "UNIT_TRIREME", x, y);
        assert!(self.g.unit_plunder(u), "patrol at ({x}, {y})");
        u
    }

    fn v(&self) -> Viewer<'_> {
        Viewer::new(&self.g, self.p0, false)
    }

    fn them(&self) -> String {
        self.g.team_label(self.g.players[self.p1].team)
    }
}

fn s(j: &Json, k: &str) -> String {
    j.get(k).and_then(Json::as_str).unwrap_or_default().to_string()
}

fn strs(j: Option<&Json>) -> Vec<String> {
    j.and_then(Json::as_arr).unwrap_or(&[]).iter().filter_map(|l| l.as_str().map(str::to_string)).collect()
}

fn sea_entry(v: &Viewer, id: SeaId) -> Option<Json> {
    view::seas::seas_json(v).as_arr().unwrap().iter().find(|j| j.get("id").and_then(Json::as_i64) == Some(id.0 as i64)).cloned()
}

#[test]
fn sea_ids_and_names_are_sent_for_revealed_plots_only() {
    let w = world();
    let rows = view::map::rows(&w.v());
    let col = view::map::LAYERS.iter().position(|&l| l == "sea_id").expect("a sea_id layer");
    let at = |x: i32, y: i32| rows.rows[(y * 40 + x) as usize][col];
    assert_eq!(at(3, 0), w.bay.0 as i32);
    assert_eq!(at(20, 1), w.narrows.0 as i32);
    assert_eq!(at(27, 1), w.sirens.0 as i32);
    assert_eq!(at(33, 1), -1, "not revealed");
    assert_eq!(at(10, 5), -1, "land");
    assert_eq!(at(5, 11), -1, "water on no sea");
    let v = w.v();
    let seas = view::seas::seas_json(&v);
    let names: Vec<String> = seas.as_arr().unwrap().iter().map(|j| s(j, "name")).collect();
    assert_eq!(names, ["Bay of Lorda", "The Grey Narrows", "Sirens' Water"], "Hidden Sea has no revealed plot");
    // Sirens' Water: only its revealed plots count, and its name goes on one of them.
    let sirens = sea_entry(&v, w.sirens).unwrap();
    assert_eq!(sirens.get("plots").and_then(Json::as_i64), Some(8));
    assert!(sirens.get("x").and_then(Json::as_i64).unwrap() < 30);
    assert!(sea_entry(&v, w.hidden).is_none());
}

#[test]
fn badges_give_the_state_and_strengths_the_human_can_see() {
    let mut w = world();
    let v = w.v();
    assert_eq!(s(&sea_entry(&v, w.bay).unwrap(), "badge"), "OPEN");
    assert_eq!(s(&sea_entry(&v, w.sirens).unwrap(), "badge"), "OUT OF RANGE", "no plot of it in range");
    assert_eq!(s(&sea_entry(&v, w.sirens).unwrap(), "state"), "out");
    // Ours in the bay: held by us.
    w.patrol(w.p0, 8, 0);
    // Contested in the narrows: one of each, side by side (each sees the other).
    w.patrol(w.p0, 20, 1);
    w.patrol(w.p1, 21, 1);
    // An enemy patrol nobody of ours sees counts for nothing.
    w.patrol(w.p1, 28, 0);
    let v = w.v();
    let bay = w.g.sea_state_known(w.bay, w.g.players[w.p0].team);
    let e = sea_entry(&v, w.bay).unwrap();
    assert_eq!(s(&e, "state"), "held_you");
    assert_eq!(s(&e, "badge"), format!("HELD: YOU {} to 0", bay.ours));
    assert!(bay.ours > 0);
    let e = sea_entry(&v, w.narrows).unwrap();
    assert_eq!(s(&e, "state"), "contested");
    assert_eq!(s(&e, "badge"), format!("CONTESTED {0} to {0}", bay.ours));
    assert_eq!(s(&sea_entry(&v, w.sirens).unwrap(), "badge"), "OUT OF RANGE", "the unseen patrol is not counted");
    // Two enemies seen in the bay (next to the port) against our one: held by them.
    w.patrol(w.p1, 4, 1);
    w.patrol(w.p1, 4, 1);
    let v = w.v();
    let e = sea_entry(&v, w.bay).unwrap();
    assert_eq!(s(&e, "state"), "held");
    assert_eq!(s(&e, "badge"), format!("HELD: {} {} to {}", w.them().to_uppercase(), bay.ours, 2 * bay.ours));
    assert_eq!(s(&e, "control"), format!("Held by {}", w.them()));
}

#[test]
fn a_harbour_says_who_holds_its_water() {
    let mut w = world();
    let line = |w: &World| view::seas::harbour_json(&w.v(), w.port);
    assert_eq!(s(&line(&w), "text"), "Harbour: open.");
    assert_eq!(s(&line(&w), "state"), "open");
    let ours = w.patrol(w.p0, 8, 0);
    assert_eq!(s(&line(&w), "text"), "Harbour: open. Bay of Lorda is held by you.");
    w.g.unit_kill(ours, false, None);
    w.patrol(w.p1, 6, 1);
    let j = line(&w);
    assert_eq!(s(&j, "state"), "blockaded");
    assert_eq!(s(&j, "text"), format!("Harbour: blockaded. Bay of Lorda is held by {}. Sea plots not worked, no sea trade.", w.them()));
    // The view's cities carry it (the banner's mark), and the city screen.
    let cities = view::cities(&w.v());
    let c = cities.as_arr().unwrap().iter().find(|j| j.get("id").and_then(Json::as_i64) == Some(w.port.0 as i64)).unwrap();
    assert_eq!(c.get("harbour").map(|h| s(h, "state")), Some("blockaded".to_string()));
    // An inland city has none.
    let inland = w.g.player_init_city(w.p0, 20, 6, true, true);
    assert!(view::seas::harbour_json(&w.v(), inland).is_null());
}

#[test]
fn an_attack_on_a_port_says_its_harbour() {
    let mut w = world();
    let theirs = w.g.player_init_city(w.p1, 20, 2, true, true);
    let v = w.v();
    assert_eq!(view::seas::attack_harbour_lines(&v, theirs), ["Harbour open. The city heals and repairs at 50% during the siege.", "To blockade it: hold The Grey Narrows (1 needed, you have 0)."]);
    w.patrol(w.p1, 20, 1);
    // A ship of ours next to it sees it.
    w.unit(w.p0, "UNIT_TRIREME", 21, 0);
    let v = w.v();
    let l = view::seas::attack_harbour_lines(&v, theirs);
    assert_eq!(l[0], format!("Harbour open: {} holds The Grey Narrows. The city heals and repairs at 50% during the siege.", w.them()));
    assert!(l[1].starts_with("To blockade it: hold The Grey Narrows ("), "{l:?}");
    for _ in 0..3 {
        w.patrol(w.p0, 19, 0);
    }
    let v = w.v();
    assert_eq!(view::seas::attack_harbour_lines(&v, theirs), ["Harbour blockaded."]);
}

#[test]
fn the_path_preview_says_where_a_patrol_intercepts() {
    let mut w = world();
    let u = w.unit(w.p0, "UNIT_WARRIOR", 12, 2);
    w.g.unit_move(u, w.at(12, 1));
    assert!(w.g.unit_is_in_boats(u));
    w.patrol(w.p1, 13, 0);
    let mut finder = PathFinder::new(FinderKind::Path);
    let v = w.v();
    let p = view::units::path(&v, &mut finder, &[u], 13, 1, 0).expect("a path");
    let lines = strs(p.get("sea").and_then(|s| s.get("lines")));
    let want = format!("Enters The Grey Narrows (held by {}) at plot 1: will be intercepted by {}'s patrol.", w.them(), w.them());
    assert!(lines.contains(&want), "{lines:?}");
    assert_eq!(view::seas::interception_lines(&v, u, &[(12, 1, 0), (13, 1, 1)]), [want]);
    // Staying in the bay: nothing.
    assert!(view::seas::interception_lines(&v, u, &[(12, 1, 0), (11, 1, 1)]).is_empty());
}

#[test]
fn patrol_is_offered_with_its_rule_and_sea_patrol_is_not() {
    let mut w = world();
    let ship = w.unit(w.p0, "UNIT_TRIREME", 8, 0);
    let v = w.v();
    let actions = view::units::actions(&v, &[ship]);
    let mission = |m: MissionType| actions.iter().find(|a| a.get("mission").and_then(Json::as_i64) == Some(m.idx() as i64)).cloned();
    assert!(mission(MissionType::MISSION_SEAPATROL).is_none(), "Civ4's Sea Patrol is not offered");
    let patrol = mission(MissionType::MISSION_PLUNDER).expect("Patrol");
    let help = s(&patrol, "help");
    assert!(help.contains("A sea is held by a side with twice its enemies' strength there."), "{help}");
    let st = w.g.sea_state_if_patrol(w.bay, w.g.players[w.p0].team, &[ship]);
    assert!(help.ends_with(&format!("Bay of Lorda: {} to 0. Held by you.", st.ours)), "{help}");
    // On Patrol: the unit panel says where.
    assert!(w.g.unit_plunder(ship));
    let v = w.v();
    let sea = view::units::unit_details(&v, ship).get("sea").cloned().unwrap();
    assert!(strs(sea.get("lines")).contains(&"On Patrol: Bay of Lorda.".to_string()), "{sea}");
}

#[test]
fn the_sea_panel_gives_sides_effects_and_the_send() {
    let mut w = world();
    let ours = w.patrol(w.p0, 20, 1);
    w.patrol(w.p1, 21, 1);
    let idle = w.unit(w.p0, "UNIT_TRIREME", 8, 0);
    let mut finder = PathFinder::new(FinderKind::Path);
    let v = w.v();
    let p = view::seas::sea_panel(&v, &mut finder, w.narrows, &[idle]).expect("a panel");
    assert_eq!(s(&p, "name"), "The Grey Narrows");
    assert_eq!(s(&p, "plots_text"), "26 plots");
    assert_eq!(s(&p, "control"), "Contested");
    let x = w.g.sea_state_known(w.narrows, w.g.players[w.p0].team).theirs;
    assert_eq!(s(&p, "hold"), format!("Held at twice the enemy's strength on patrol. You need {}.", 2 * x));
    let sides = p.get("sides").and_then(Json::as_arr).unwrap();
    assert_eq!(sides.len(), 2);
    assert_eq!(s(&sides[0], "label"), "You");
    assert_eq!(s(&sides[1], "label"), format!("{} (at war)", w.them()));
    assert!(s(&sides[1], "note").ends_with("Ships you do not see are not counted."));
    // (A Trireme carries a unit, but our patrol is out of range: it has no lane.)
    assert_eq!(strs(p.get("now")), ["Lanes through it: none.", "Your ports on it: none.", "Interception: both sides."]);
    let they = p.get("if_they").unwrap();
    assert_eq!(s(they, "title"), format!("If {} holds it", w.them()));
    assert_eq!(s(p.get("if_you").unwrap(), "title"), "If you hold it");
    // The send: a go-to to the narrows, then Patrol, and the state it gives.
    let send = p.get("send").unwrap();
    assert_eq!(send.get("here").and_then(Json::as_bool), Some(false));
    assert!(s(send, "label").starts_with("Send the Trireme to patrol here ("), "{send}");
    let at = (send.get("x").and_then(Json::as_i64).unwrap(), send.get("y").and_then(Json::as_i64).unwrap());
    assert!((13..=25).contains(&at.0) && at.1 <= 1, "a plot of the narrows: {at:?}");
    let after = w.g.sea_state_if_patrol(w.narrows, w.g.players[w.p0].team, &[idle]);
    assert_eq!(strs(send.get("result"))[0], format!("The Grey Narrows: {} to {}. Held by you.", after.ours, after.theirs));
    // The ship already there on Patrol: nothing to send.
    let p = view::seas::sea_panel(&v, &mut finder, w.narrows, &[ours]).unwrap();
    assert_eq!(p.get("send").and_then(|s| s.get("enabled")).and_then(Json::as_bool), Some(false));
    // A land unit: no send.
    let walker = w.unit(w.p0, "UNIT_WARRIOR", 10, 4);
    let v = w.v();
    assert!(view::seas::sea_panel(&v, &mut finder, w.narrows, &[walker]).unwrap().get("send").is_some_and(Json::is_null));
    // A sea with no revealed plot is not answered.
    assert!(view::seas::sea_panel(&v, &mut finder, w.hidden, &[]).is_err());
}

#[test]
fn the_seas_screen_lists_ports_and_lanes() {
    let mut w = world();
    w.patrol(w.p1, 6, 1);
    w.patrol(w.p1, 6, 1);
    // A supply ship in the narrows: its lane to the port runs into the bay, held by them.
    let galleon = w.unit(w.p0, "UNIT_GALLEON", 13, 1);
    let v = w.v();
    let rows = view::seas::seas_screen(&v);
    let rows = rows.get("seas").and_then(Json::as_arr).unwrap();
    let row = |id: SeaId| rows.iter().find(|j| j.get("id").and_then(Json::as_i64) == Some(id.0 as i64)).unwrap();
    let port = view::city_name(&w.g, w.port);
    assert_eq!(s(row(w.bay), "ports"), format!("{port}: blockaded"));
    assert_eq!(s(row(w.bay), "lanes"), "1 cut");
    assert_eq!(s(row(w.narrows), "lanes"), "1 cut");
    assert_eq!(s(row(w.narrows), "ports"), "none");
    assert!(s(row(w.bay), "enemy").ends_with(&w.them()), "{}", row(w.bay));
    // The lane on the Sea layer: cut where it enters the bay.
    let lanes = view::seas::lanes_json(&v);
    let lane = lanes.as_arr().unwrap().iter().find(|j| j.get("ship").and_then(Json::as_i64) == Some(galleon.0 as i64)).expect("its lane").clone();
    let cut = lane.get("cut").expect("cut");
    assert_eq!(s(cut, "sea"), "Bay of Lorda");
    let at = cut.get("at").and_then(Json::as_i64).unwrap() as usize;
    let path = lane.get("path").and_then(Json::as_arr).unwrap();
    assert!(path[at].as_arr().unwrap()[0].as_i64().unwrap() <= 12, "the break is the bay's first plot: {lane}");
    assert!(at > 0 && path[at - 1].as_arr().unwrap()[0].as_i64().unwrap() >= 13);
    // Next to it on land, an army would read why the transport is no source.
    assert_eq!(view::seas::transport_refusal(&v, w.at(13, 2)).as_deref(), Some("the transport's lane is cut in Bay of Lorda."));
    // In the bay itself: the sea is closed.
    w.g.unit_kill(galleon, false, None);
    w.unit(w.p0, "UNIT_GALLEON", 10, 1);
    let v = w.v();
    assert_eq!(view::seas::transport_refusal(&v, w.at(10, 2)), Some(format!("the transport is in Bay of Lorda, held by {}.", w.them())));
}
