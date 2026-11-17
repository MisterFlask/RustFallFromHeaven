//! T8 for the queries `tests/fog.rs` does not cover (design §8): the city screen, the Armies
//! screen, the seas, the battle reports and logs, the crusades, the players, trade and the turn
//! report show only units, cities, plots, seas and players the human may know (§4.4). Each reply
//! is walked as a client receives it: every member that names a player, a unit, a city, a plot
//! or a sea is checked against the game's own state. The culture queries are left out (their
//! redesign is under way).

mod common;

use common::{bot_session, peaceful_game};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_play::query::{Query, QueryErr};
use ffh_play::{bot, Json, Session, Stop};

fn int(j: &Json, k: &str) -> i64 {
    j.get(k).and_then(Json::as_i64).unwrap_or(-1)
}

fn arr<'a>(j: &'a Json, k: &str) -> &'a [Json] {
    j.get(k).and_then(Json::as_arr).unwrap_or(&[])
}

/// A reply as a client receives it.
fn wire(s: &Session, q: &Query) -> Json {
    let r = s.query(q).unwrap_or_else(|e| panic!("{q:?}: {e}"));
    Json::parse(&r.to_string()).expect("valid JSON")
}

/// The human may see unit `u` (§4.4; as tests/fog.rs).
fn may_see_unit(g: &Game, h: PlayerId, u: UnitId) -> bool {
    let team = g.players[h].team;
    let unit = &g.units[u];
    unit.owner == h || (g.map.plot_xy(unit.x, unit.y).is_some_and(|p| g.plot_is_visible(p, team)) && !g.unit_is_invisible(u, team, true))
}

/// The human knows player `p`: itself, a player its team has met, or the barbarians (whom a
/// hidden-nationality unit shows).
fn knows_player(g: &Game, h: PlayerId, p: i64) -> bool {
    let Ok(p) = u8::try_from(p) else { return false };
    let p = PlayerId(p);
    if p.0 as usize >= g.num_players() {
        return false;
    }
    let team = g.players[h].team;
    p == h || p == g.barbarian_player() || g.players[p].team == team || g.teams[team].has_met[g.players[p].team]
}

/// Walks reply `j` of query `what` and checks every member that names a game object: `player`,
/// `owner`, `shown_player` and `with` a known player; `unit` / `units` (ids) a unit the human
/// may see, while it lives (a report names units that have died since), or one of `fought` (the
/// units of the human's battles, which it saw fight); `city` a city of the human's or one it has
/// revealed; an object's `x`, `y` a plot it has revealed. Returns how many members it checked.
fn check(g: &Game, h: PlayerId, fought: &[UnitId], what: &str, j: &Json) -> usize {
    let team = g.players[h].team;
    let mut n = 0;
    let unit_ok = |u: i64| {
        let u = UnitId(u as u32);
        !g.units.contains(u) || may_see_unit(g, h, u) || fought.contains(&u)
    };
    match j {
        Json::Obj(m) => {
            for (k, v) in m.iter() {
                // A battle log's events name their fighters by index into its `fighters`
                // (whose `unit` is checked), not by unit id.
                if k == "events" && what.starts_with("battle_log") {
                    continue;
                }
                match (k.as_str(), v) {
                    ("player" | "owner" | "shown_player" | "with", Json::Int(p)) if *p >= 0 => {
                        n += 1;
                        assert!(knows_player(g, h, *p), "{what}: `{k}` names player {p}, whom the human does not know: {j}");
                    }
                    ("unit", Json::Int(u)) => {
                        n += 1;
                        assert!(unit_ok(*u), "{what}: unit {u} is hidden from the human: {j}");
                    }
                    ("units", Json::Arr(us)) => {
                        for u in us.iter().filter_map(Json::as_i64) {
                            n += 1;
                            assert!(unit_ok(u), "{what}: unit {u} is hidden from the human: {j}");
                        }
                    }
                    ("city", Json::Int(c)) => {
                        let c = CityId(*c as u32);
                        if let Some(city) = g.cities.get(c) {
                            n += 1;
                            assert!(city.owner == h || city.revealed[team], "{what}: city {c:?} is unrevealed: {j}");
                        }
                    }
                    _ => {}
                }
                n += check(g, h, fought, &format!("{what}.{k}"), v);
            }
            if let (Some(Json::Int(x)), Some(Json::Int(y))) = (j.get("x"), j.get("y")) {
                if let Some(p) = g.map.plot_xy(*x as i32, *y as i32) {
                    n += 1;
                    assert!(g.map.plots[p].revealed[team], "{what}: plot ({x}, {y}) is unrevealed: {j}");
                }
            }
        }
        Json::Arr(a) => {
            for v in a {
                n += check(g, h, fought, what, v);
            }
        }
        _ => {}
    }
    n
}

/// Sea `id` has a plot the human has revealed.
fn sea_known(g: &Game, h: PlayerId, id: i64) -> bool {
    let team = g.players[h].team;
    g.map.seas.get(id as usize).is_some_and(|s| s.plots.iter().any(|&p| g.map.plots[p].revealed[team]))
}

/// The queries of this file at one control point, checked; returns the members checked.
fn check_queries(s: &Session) -> usize {
    let g = &s.g;
    let h = s.human;
    let mut n = 0;
    // Players: met, other than us.
    let players = wire(s, &Query::Players);
    for p in arr(&players, "players") {
        assert!(int(p, "id") != h.0 as i64 && knows_player(g, h, int(p, "id")), "players: unmet player {}", int(p, "id"));
    }
    n += check(g, h, &[], "players", &players);
    // City screens: ours, and no other.
    for (c, city) in g.cities.iter() {
        if city.owner == h {
            n += check(g, h, &[], "city", &wire(s, &Query::City { city: c }));
        } else {
            assert_eq!(s.query(&Query::City { city: c }).map(|_| ()), Err(QueryErr::NotOwner), "city screen of {c:?}");
        }
    }
    let armies = wire(s, &Query::Armies);
    n += check(g, h, &[], "armies", &armies);
    for sea in arr(&armies, "seas") {
        assert!(sea_known(g, h, int(sea, "id")), "armies: unknown sea {}", int(sea, "id"));
    }
    // Seas: the ones the human has revealed a plot of; another sea's panel is `gone`.
    let seas = wire(s, &Query::Seas);
    n += check(g, h, &[], "seas", &seas);
    let listed: Vec<i64> = arr(&seas, "seas").iter().map(|r| int(r, "id")).collect();
    for i in 0..g.map.seas.len() as i64 {
        assert_eq!(listed.contains(&i), sea_known(g, h, i), "seas: sea {i}");
        let q = Query::Sea { id: SeaId(i as u32), units: Vec::new() };
        if sea_known(g, h, i) {
            n += check(g, h, &[], "sea", &wire(s, &q));
        } else {
            assert_eq!(s.query(&q).map(|_| ()), Err(QueryErr::Gone), "sea {i} is unknown");
        }
    }
    // Battle reports and their logs: their units are the ones that fought the human's battles.
    let fought: Vec<UnitId> = g.battle_reports.iter().filter(|r| r.involves(h)).flat_map(|r| r.log.iter().flat_map(|l| l.fighters.iter().map(|f| f.unit))).collect();
    let reports = wire(s, &Query::BattleReports);
    n += check(g, h, &fought, "battle_reports", &reports);
    for r in arr(&reports, "reports") {
        if let Ok(log) = s.query(&Query::BattleLog { report: int(r, "id") as u32 }) {
            n += check(g, h, &fought, "battle_log", &Json::parse(&log.to_string()).unwrap());
        }
    }
    // Crusades: the wars among teams the human has met.
    let crusades = wire(s, &Query::Crusades);
    n += check(g, h, &[], "crusades", &crusades);
    for w in arr(&crusades, "wars") {
        for t in w.as_arr().unwrap().iter().filter_map(Json::as_i64) {
            let t = TeamId(t as u8);
            assert!(t == g.players[h].team || g.teams[g.players[h].team].has_met[t], "crusades: a war of unmet team {t:?}");
        }
    }
    // Trade: with a known civilization, else refused.
    for p in g.civ_player_ids().filter(|&p| p != h) {
        match s.query(&Query::Trade { with: p }) {
            Ok(t) => {
                assert!(knows_player(g, h, p.0 as i64), "trade with unmet {p:?}");
                n += check(g, h, &[], "trade", &Json::parse(&t.to_string()).unwrap());
            }
            Err(e) => assert_eq!(e.code(), "invalid", "trade with {p:?}: {e}"),
        }
    }
    n += check(g, h, &[], "report", &wire(s, &Query::Report));
    n
}

/// Over a bot game: the queries keep the fog at control points of a long, peaceful game (no
/// game can end in it, [`peaceful_game`]); only invariants and coverage are asserted.
#[test]
fn the_newer_queries_keep_the_fog() {
    let ng = peaceful_game(22);
    let (mut s, mut b) = bot_session(&ng, None);
    let until = s.g.game_turn() + 100;
    let mut checked = 0;
    let mut points = 0;
    let stop = bot::run(&mut s, &mut b, until, |s, _| {
        if s.g.game_turn() % 5 == 0 {
            checked += check_queries(s);
            points += 1;
        }
    });
    assert_eq!(stop, Stop::Human);
    assert!(points >= 15 && checked > 100, "{points} control points, {checked} members checked");
}

/// A war between two teams the human has not met is not in the crusades' wars; under debug
/// reveal it is (built by hand: the war is declared directly).
#[test]
fn an_unmet_war_stays_out_of_the_crusades() {
    let ng = peaceful_game(5);
    let (mut s, mut b) = bot_session(&ng, None);
    let until = s.g.game_turn() + 3;
    assert_eq!(bot::run(&mut s, &mut b, until, |_, _| {}), Stop::Human);
    let h = s.human;
    let my = s.g.players[h].team;
    // Two AI teams, neither met by the human (met with each other for the declaration).
    let unmet: Vec<TeamId> = s.g.civ_team_ids().filter(|&t| t != my && s.g.team_is_alive(t) && !s.g.teams[my].has_met[t]).collect();
    assert!(unmet.len() >= 2, "the test needs two teams the human has not met: {unmet:?}");
    let (a, b2) = (unmet[0], unmet[1]);
    common::allow_war(&mut s.g);
    s.g.team_meet(a, b2, false);
    s.g.team_declare_war(a, b2, false, None);
    assert!(s.g.at_war(a, b2));
    let has = |s: &Session| {
        let c = wire(s, &Query::Crusades);
        arr(&c, "wars").iter().any(|w| {
            let w: Vec<i64> = w.as_arr().unwrap().iter().filter_map(Json::as_i64).collect();
            w.contains(&(a.0 as i64)) && w.contains(&(b2.0 as i64))
        })
    };
    assert!(!has(&s), "a war between unmet teams is shown");
    check_queries(&s);
    s.set_reveal(true);
    assert!(has(&s), "debug reveal shows the war");
    s.set_reveal(false);
}

/// The staged engagement (`ffh-play stage engagement`): after the battle, its report and log
/// name only players, units and plots the human may know.
#[test]
fn a_battle_report_keeps_the_fog() {
    let dir = common::TempDir::new("fog-queries-engagement");
    let st = ffh_play::stage::engagement(&dir.0).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = ffh_play::SessionOptions { journal: false, ..Default::default() };
    let mut s = Session::load(&bytes, sidecar.as_ref(), std::sync::Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    let j = Json::parse(&format!(r#"{{"c":"engage","army":{},"to":{{"x":{},"y":{}}}}}"#, st.army.0, st.x, st.y)).unwrap();
    let c = ffh_play::command::Command::from_json(s.g.d, &j).unwrap();
    s.execute(&c).expect("the attack");
    assert_eq!(s.g.battle_reports.len(), 1);
    let reports = wire(&s, &Query::BattleReports);
    assert_eq!(arr(&reports, "reports").len(), 1);
    assert!(check_queries(&s) > 0);
}
