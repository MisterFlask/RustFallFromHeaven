//! Crusade scenario tests (docs/CRUSADE.md, "Tests and acceptance criteria"): the rules of
//! crusades and allied loans on a constructed map, with scripted controllers. Every test is
//! deterministic (fixed seed, fixed choices); combat odds are made one-sided where a fight decides
//! the scenario.

use std::sync::{Arc, Mutex};

use ffh_core::controller::Controller;
use ffh_core::crusade::loan::{LoanAnswer, LoanMission, LoanProposal, LoanTerms};
use ffh_core::crusade::{CrusadeId, CrusadeKind, CrusadeRules, Dissolution, Objective, Pledge};
use ffh_core::crusade::lend::LendError;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::mapscript::FractalMapScript;
use ffh_core::setup::SetupOptions;
use ffh_data::*;

/// Scripted decisions: pledges, draft picks (recorded), loans.
#[derive(Default)]
struct Script {
    pledge: Option<Pledge>,
    /// Draft: pick these cities (by plot) in order when offered, then stop; `None` = default.
    picks: Option<Vec<(i32, i32)>>,
    /// (player, budget) of every draft question, in order.
    asked: Mutex<Vec<(PlayerId, i32)>>,
    accept_loans: bool,
}

impl Controller for Script {
    fn crusade_pledge(&self, _g: &Game, _p: PlayerId, _c: CrusadeId) -> Option<Pledge> {
        self.pledge
    }

    fn crusade_draft_pick(&self, g: &Game, p: PlayerId, _c: CrusadeId, budget: i32, offered: &[(CityId, i32)]) -> Option<CityId> {
        self.asked.lock().unwrap().push((p, budget));
        match &self.picks {
            None => offered.iter().filter(|&&(_, v)| v <= budget).max_by_key(|&&(c, v)| (v, std::cmp::Reverse(c))).map(|&(c, _)| c),
            Some(list) => list.iter().find_map(|&(x, y)| offered.iter().find(|&&(c, _)| (g.cities[c].x, g.cities[c].y) == (x, y)).map(|&(c, _)| c)),
        }
    }

    fn consider_loan(&self, _g: &Game, _p: PlayerId, _proposal: &LoanProposal) -> LoanAnswer {
        if self.accept_loans {
            LoanAnswer::Accept
        } else {
            LoanAnswer::Refuse
        }
    }
}

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const T: PlayerId = PlayerId(2);
const C: PlayerId = PlayerId(3);

/// Four civs (A, B and C lend, T is the target) on a flat 30x14 grassland, every team meeting
/// every other, cities: A at (3, 4), B at (3, 10), C at (12, 12), T at (24, 4) and (24, 10).
/// No barbarians.
fn game(script: Arc<Script>, crusades: bool) -> Game {
    let d = ffh_data::global();
    let mut o = SetupOptions::new(d, 4, "WORLDSIZE_DUEL", 17);
    o.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    o.options.push(GameOptionType::GAMEOPTION_NO_EVENTS);
    if crusades {
        o.crusade = CrusadeRules::on();
    }
    let mut g = Game::new_with_options(d, &o);
    g.map_script = Arc::new(FractalMapScript::default());
    for p in g.player_ids() {
        g.controllers[p] = script.clone();
    }
    g.autorun = true;
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (w, h) = (30, 14);
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: w, grid_h: h, ..Default::default() }, np, nt);
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for p in g.map.plot_ids() {
        g.plot_set_plot_type(p, PlotType::PLOT_LAND, false);
        g.plot_set_terrain_type(p, grass, false);
    }
    g.map_recalculate_areas();
    for t in g.team_ids() {
        for p in g.map.plot_ids() {
            g.plot_set_revealed(p, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    for a in [A, B, T, C] {
        for b in [A, B, T, C] {
            if a != b {
                let (ta, tb) = (g.players[a].team, g.players[b].team);
                g.team_meet(ta, tb, false);
            }
        }
    }
    g.player_found(A, 3, 4);
    g.player_found(B, 3, 10);
    g.player_found(T, 24, 4);
    g.player_found(T, 24, 10);
    g.player_found(C, 12, 12);
    assert_eq!(g.players[T].cities.len(), 2, "target founded two cities");
    // (size-1 cities are razed when captured)
    for c in g.cities.ids() {
        g.city_set_population(c, 3);
    }
    g
}

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, Some(UnitAIType::UNITAI_ATTACK), None)
}

fn call(g: &mut Game, called: &[PlayerId]) -> CrusadeId {
    let target = g.players[T].team;
    g.crusade_call(CrusadeKind::Direct, target, None, Objective::Term, 30, 5, called).expect("a Host slot is free")
}

fn host_of(g: &Game, id: CrusadeId) -> PlayerId {
    g.crusade(id).unwrap().host
}

fn lender_credit(g: &Game, id: CrusadeId, p: PlayerId) -> i32 {
    let idx = g.crusade_index(id).unwrap();
    g.crusade_lender(idx, p).map_or(0, |l| l.credit)
}

#[test]
fn lending_transfers_ownership_records_the_lender_removes_upkeep_and_declares_war() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A, B]);
    let host = host_of(&g, id);
    assert!(g.player_is_host_slot(host) && g.players[host].alive);
    let (ta, tt) = (g.players[A].team, g.players[T].team);
    assert!(!g.at_war(ta, tt));
    assert!(g.at_war(g.players[host].team, tt), "the Host is at war with the target");

    let units: Vec<UnitId> = (0..4).map(|_| unit(&mut g, A, "UNIT_WARRIOR", 3, 4)).collect();
    let strength: i32 = units.iter().map(|&u| g.crusade_unit_strength(u)).sum();
    let lender_units = g.players[A].units.len();
    let lent = g.crusade_lend(id, A, &units).expect("A can lend");
    assert_eq!(lent, strength);
    assert_eq!(g.players[A].units.len(), lender_units - 4, "the units left the lender");
    let served = g.crusade_lent_units(g.crusade_index(id).unwrap(), Some(A));
    assert_eq!(served.len(), 4, "the Host owns them");
    for &u in &served {
        let rec = g.units[u].lent.as_ref().unwrap();
        assert_eq!(rec.lender, A);
        assert_eq!(g.units[u].owner, host);
        assert!(!rec.credited);
    }
    assert_eq!(served.iter().map(|&u| g.units[u].lent.as_ref().unwrap().strength).sum::<i32>(), strength, "handover strength recorded");
    assert_eq!(g.player_calculate_inflated_costs(host), 0, "Host units cost no one anything");
    assert!(g.at_war(ta, tt), "lending declared war on the target");
    // Lent units cannot be recalled: there is no API for it, and the lender cannot make peace.
    let peace = ffh_core::utils::set_trade_item(TradeableItem::TRADE_PEACE_TREATY, 0);
    assert!(!g.player_can_trade_item(A, T, &peace, false), "no separate peace while units serve");
}

#[test]
fn lending_is_refused_when_the_lender_could_not_declare_war() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let (ta, tt) = (g.players[A].team, g.players[T].team);
    g.teams[ta].force_peace[tt] = true; // a peace treaty in force
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    assert_eq!(g.crusade_lend(id, A, &[u]), Err(LendError::CannotDeclareWar));
    assert_eq!(g.units[u].owner, A);
    assert!(!g.at_war(ta, tt));
}

#[test]
fn a_lent_unit_that_never_fights_earns_nothing_and_returns_at_dissolution() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    let name = g.units[u].name.clone();
    g.crusade_lend(id, A, &[u]).unwrap();
    assert_eq!(lender_credit(&g, id, A), 0);
    let host = host_of(&g, id);
    g.crusade_dissolve(id, Dissolution::Term);
    assert!(g.crusade(id).is_none());
    assert!(!g.players[host].alive, "the Host is deleted");
    let report = g.crusade_reports.last().unwrap();
    assert_eq!(report.lenders.iter().find(|r| r.player == A).unwrap().credit, 0);
    assert_eq!(g.players[A].units.len(), 1, "the unit came home");
    let back = g.players[A].units[0];
    assert_eq!(g.units[back].name, name);
    assert!(g.units[back].lent.is_none());
}

#[test]
fn a_first_kill_credits_the_handover_strength_once() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_CHAMPION", 10, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    let strength = g.units[hu].lent.as_ref().unwrap().strength;
    for (i, x) in [11, 12].into_iter().enumerate() {
        let e = unit(&mut g, T, "UNIT_WARRIOR", x, 4);
        g.units[e].damage = 95;
        let plot = g.map.plot_idx(x, 4).unwrap();
        g.units[hu].moves = 10 * g.d.defines.move_denominator;
        g.units[hu].made_attack = false;
        g.unit_attack(hu, plot);
        assert!(g.units.contains(hu), "the champion won fight {i}");
        assert!(!g.units.contains(e) || g.units[e].death_delay, "the warrior died");
        assert_eq!(lender_credit(&g, id, A), strength, "credited once, after fight {i}");
        assert!(g.units[hu].lent.as_ref().unwrap().credited);
    }
}

#[test]
fn a_unit_that_dies_without_killing_is_credited() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 10, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    let strength = g.units[hu].lent.as_ref().unwrap().strength;
    g.unit_kill(hu, false, Some(T));
    assert_eq!(lender_credit(&g, id, A), strength);
}

#[test]
fn bandwagon_late_big_lender_ranks_below_the_early_lender_that_fought() {
    let script = Arc::new(Script { picks: Some(vec![]), ..Default::default() });
    let mut g = game(script.clone(), true);
    let id = call(&mut g, &[A, B]);
    let host = host_of(&g, id);
    // A lends 10 units early; all fight (and fall).
    let a_units: Vec<UnitId> = (0..10).map(|_| unit(&mut g, A, "UNIT_WARRIOR", 3, 4)).collect();
    g.crusade_lend(id, A, &a_units).unwrap();
    for u in g.crusade_lent_units(g.crusade_index(id).unwrap(), Some(A)) {
        g.unit_kill(u, false, Some(T));
    }
    // B lends 20 units on the last turn; 2 get kills.
    let b_units: Vec<UnitId> = (0..20).map(|_| unit(&mut g, B, "UNIT_WARRIOR", 10, 10)).collect();
    g.crusade_lend(id, B, &b_units).unwrap();
    let b_served = g.crusade_lent_units(g.crusade_index(id).unwrap(), Some(B));
    for (k, &hu) in b_served.iter().take(2).enumerate() {
        let x = 11 + k as i32;
        let e = unit(&mut g, T, "UNIT_WARRIOR", x, 10);
        g.units[e].damage = 99;
        g.units[hu].x = 10;
        g.unit_attack(hu, g.map.plot_idx(x, 10).unwrap());
    }
    assert!(g.players[host].units.len() >= 18);
    let (ca, cb) = (lender_credit(&g, id, A), lender_credit(&g, id, B));
    assert!(ca > cb && cb > 0, "A {ca} > B {cb}");
    g.players[host].gold = 300; // sacked wealth to share
    g.crusade_dissolve(id, Dissolution::Term);
    let asked = script.asked.lock().unwrap().clone();
    assert_eq!(asked.first().map(|a| a.0), None, "no city to draft (only wealth)");
    let report = g.crusade_reports.last().unwrap();
    let ra = report.lenders.iter().find(|r| r.player == A).unwrap();
    let rb = report.lenders.iter().find(|r| r.player == B).unwrap();
    assert!(ra.spoils > rb.spoils, "A ranks first: {} > {}", ra.spoils, rb.spoils);
    assert_eq!(ra.spoils + rb.spoils, 300);
}

/// Hand both target cities to the Host (as if captured), for the draft tests.
fn host_takes_target_cities(g: &mut Game, host: PlayerId) {
    for c in g.players[T].cities.clone() {
        g.player_acquire_city(host, c, false, true, true);
    }
    assert_eq!(g.players[host].cities.len(), 2);
}

fn credit_both(g: &mut Game, id: CrusadeId, a_n: usize, b_n: usize) {
    let a: Vec<UnitId> = (0..a_n).map(|_| unit(g, A, "UNIT_WARRIOR", 3, 4)).collect();
    let b: Vec<UnitId> = (0..b_n).map(|_| unit(g, B, "UNIT_WARRIOR", 3, 10)).collect();
    g.crusade_lend(id, A, &a).unwrap();
    g.crusade_lend(id, B, &b).unwrap();
    let idx = g.crusade_index(id).unwrap();
    for u in g.crusade_lent_units(idx, None) {
        g.unit_kill(u, false, Some(T));
    }
}

#[test]
fn budgets_sum_to_the_pool_and_the_top_contributor_drafts_first() {
    let script = Arc::new(Script::default());
    let mut g = game(script.clone(), true);
    let id = call(&mut g, &[A, B]);
    let host = host_of(&g, id);
    credit_both(&mut g, id, 2, 6); // B contributes three times A
    host_takes_target_cities(&mut g, host);
    g.players[host].gold = 101;
    let values: i32 = g.players[host].cities.iter().map(|&c| g.crusade_city_value(c)).sum();
    let pool = values + 101;
    let budgets = ffh_core::crusade::dissolve::split_by_weight(pool as i64, &[6, 2]);
    assert_eq!(budgets.iter().sum::<i64>(), pool as i64, "budgets sum to the pool value");
    g.crusade_dissolve(id, Dissolution::Term);
    let asked = script.asked.lock().unwrap().clone();
    assert_eq!(asked[0], (B, budgets[0] as i32), "the top contributor drafts first, with its whole share");
    let report = g.crusade_reports.last().unwrap();
    assert_eq!(report.pool, pool);
    let total: i32 = report.lenders.iter().map(|r| r.spoils).sum();
    assert_eq!(total, pool, "every gold of the pool is shared (no city unclaimed)");
}

#[test]
fn overdraw_debits_the_lender_and_credits_the_wealth_pool() {
    // A (small share) takes the far bigger city: it pays the difference into the pool, which B
    // then draws on as wealth.
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A, B]);
    let host = host_of(&g, id);
    credit_both(&mut g, id, 1, 1);
    host_takes_target_cities(&mut g, host);
    // Make (24, 4) much bigger than (24, 10).
    let big = g.players[host].cities.iter().copied().find(|&c| g.cities[c].y == 4).unwrap();
    g.city_set_population(big, 10);
    let small = g.players[host].cities.iter().copied().find(|&c| g.cities[c].y == 10).unwrap();
    let (vb, vs) = (g.crusade_city_value(big), g.crusade_city_value(small));
    let pool = vb + vs; // no sacked gold
    let budgets = ffh_core::crusade::dissolve::split_by_weight(pool as i64, &[1, 1]);
    let over = vb - budgets[0] as i32;
    assert!(over > 0, "the big city exceeds A's budget");
    let script = Arc::new(Script { picks: Some(vec![(24, 4)]), ..Default::default() });
    g.controllers[A] = script.clone();
    g.controllers[B] = Arc::new(Script { picks: Some(vec![]), ..Default::default() });
    g.players[A].gold = 5000;
    g.players[B].gold = 0;
    g.crusade_dissolve(id, Dissolution::Term);
    assert_eq!(g.players[A].gold, 5000 - over, "A paid the overdraw from its treasury");
    // B took no city: its budget is paid from the wealth pool, which holds only the overdraw
    // (the small city went unclaimed), so B gets the overdraw, pro rata of its unspent budget.
    assert_eq!(g.players[B].gold, over.min(budgets[1] as i32), "the overdraw went into the pool");
    assert!(g.map.plots[g.map.plot_idx(24, 4).unwrap()].plot_city.is_some_and(|c| g.cities[c].owner == A));
}

#[test]
fn unspent_budgets_are_paid_from_wealth_pro_rata_when_short() {
    let mut g = game(Arc::new(Script { picks: Some(vec![]), ..Default::default() }), true);
    let id = call(&mut g, &[A, B]);
    let host = host_of(&g, id);
    credit_both(&mut g, id, 1, 3);
    host_takes_target_cities(&mut g, host);
    g.players[host].gold = 400;
    let (ga, gb) = (g.players[A].gold, g.players[B].gold);
    g.crusade_dissolve(id, Dissolution::Term);
    // Nobody drafted a city: budgets sum to the pool (cities + 400) but only 400 is wealth.
    let (pa, pb) = (g.players[A].gold - ga, g.players[B].gold - gb);
    assert_eq!(pa + pb, 400, "the whole wealth is paid out");
    assert_eq!(pb, 300, "pro rata of the unspent budgets (3:1)");
    // Unclaimed cities with no victim go free as independent cities.
    let barb = g.barbarian_player();
    assert_eq!(g.players[barb].cities.len(), 2);
}

#[test]
fn a_victims_recaptured_city_returns_at_once_and_never_enters_the_pool() {
    let mut g = game(Arc::new(Script { picks: Some(vec![]), ..Default::default() }), true);
    // B is the victim: T took B's city.
    let bc = g.players[B].cities[0];
    g.player_acquire_city(T, bc, true, false, true);
    let target = g.players[T].team;
    let id = g.crusade_call(CrusadeKind::Direct, target, Some(B), Objective::Recover, 30, 5, &[A]).unwrap();
    let host = host_of(&g, id);
    // (the Recover list is filled as the target takes victim cities; the city was taken before
    // the call, so its original owner identifies it)
    let u = unit(&mut g, A, "UNIT_WARRIOR", 4, 10);
    g.crusade_lend(id, A, &[u]).unwrap();
    let hu = g.players[host].units[0];
    for d in g.map.plots[g.map.plot_idx(3, 10).unwrap()].units.clone() {
        g.unit_kill(d, false, None);
    }
    g.unit_set_xy(hu, 3, 10, false);
    let city = g.map.plots[g.map.plot_idx(3, 10).unwrap()].plot_city.unwrap();
    assert_eq!(g.cities[city].owner, B, "returned to the victim at once");
    assert!(g.players[host].cities.is_empty());
    g.crusade_dissolve(id, Dissolution::Objective);
    assert_eq!(g.crusade_reports.last().unwrap().pool, 0, "never entered the pool");
}

#[test]
fn dissolution_fires_on_objective_term_and_last_unit() {
    let target_city = |g: &Game| g.players[T].cities[0];
    // Objective: the listed city is taken.
    let mut g = game(Arc::new(Script::default()), true);
    let tc = target_city(&g);
    let plot = g.city_plot(tc);
    let target = g.players[T].team;
    let id = g.crusade_call(CrusadeKind::Direct, target, None, Objective::Capture(vec![plot]), 30, 5, &[A]).unwrap();
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let host = host_of(&g, id);
    assert_eq!(g.crusade_dissolution_due(g.crusade_index(id).unwrap()), None);
    g.player_acquire_city(host, tc, true, false, true);
    assert_eq!(g.crusade_dissolution_due(g.crusade_index(id).unwrap()), Some(Dissolution::Objective));
    g.crusade_do_turn();
    assert!(g.crusade(id).is_none());
    assert_eq!(g.crusade_reports.last().unwrap().reason, Dissolution::Objective);

    // Term.
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let end = g.crusade(id).unwrap().end_turn;
    g.set_game_turn(end + 1);
    g.crusade_do_turn();
    assert_eq!(g.crusade_reports.last().unwrap().reason, Dissolution::Term);

    // Last unit lost.
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    g.unit_kill(hu, false, Some(T));
    g.crusade_do_turn();
    assert_eq!(g.crusade_reports.last().unwrap().reason, Dissolution::NoUnits);
    assert!(!g.players[host].alive);
}

#[test]
fn survivors_and_heroes_return_home_with_their_promotions() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 20, 4);
    let promo = g.d.t::<PromotionType>("PROMOTION_COMBAT1");
    g.crusade_lend(id, A, &[u]).unwrap();
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    g.unit_set_has_promotion(hu, promo, true); // earned inside the Host
    g.units[hu].experience = 7;
    // A hero (immortal: the usual escape rule): dies mid-crusade and escapes home.
    let hero = g.d.t::<UnitType>("UNIT_CHAMPION");
    let h = g.player_init_unit(A, hero, 3, 4, Some(UnitAIType::UNITAI_ATTACK), None);
    let immortal = g.d.t::<PromotionType>("PROMOTION_IMMORTAL");
    g.unit_set_has_promotion(h, immortal, true);
    g.crusade_lend(id, A, &[h]).unwrap();
    let hh = *g.players[host].units.iter().find(|&&x| g.units[x].unit_type == hero).unwrap();
    g.unit_kill(hh, false, Some(T));
    let home_hero = *g.players[A].units.iter().find(|&&x| g.units[x].unit_type == hero).expect("the hero escaped to its lender");
    assert!(g.units[home_hero].lent.is_none());
    let capital = g.players[A].capital_city.unwrap();
    assert_eq!((g.units[home_hero].x, g.units[home_hero].y), (g.cities[capital].x, g.cities[capital].y), "reborn in the lender's capital");

    g.crusade_dissolve(id, Dissolution::Term);
    let back = *g.players[A].units.iter().find(|&&x| g.units[x].unit_type == g.d.t::<UnitType>("UNIT_WARRIOR")).unwrap();
    assert!(g.units[back].has_promotion[promo], "kept the promotion earned in the Host");
    assert_eq!(g.units[back].experience, 7);
    assert_eq!((g.units[back].x, g.units[back].y), (3, 4), "placed in the lender's nearest city");
}

#[test]
fn refusing_or_ignoring_a_council_call_costs_standing() {
    let mut g = game(Arc::new(Script { pledge: Some(Pledge::Refuse), ..Default::default() }), true);
    let vs = VoteSourceType(0);
    for p in [A, B] {
        g.players[p].loyal_member[vs] = true;
    }
    g.controllers[B] = Arc::new(Script::default()); // B never answers
    let target = g.players[T].team;
    let id = g.crusade_call(CrusadeKind::Council(vs), target, None, Objective::Term, 30, 3, &[A, B]).unwrap();
    let anger_before: i32 = g.players[A].cities.iter().map(|&c| g.cities[c].defy_resolution_anger_timer).sum();
    let deadline = g.crusade(id).unwrap().deadline_turn;
    g.set_game_turn(deadline);
    g.crusade_do_turn();
    for p in [A, B] {
        assert!(!g.players[p].loyal_member[vs], "player {p:?} lost its loyal standing");
        let anger: i32 = g.players[p].cities.iter().map(|&c| g.cities[c].defy_resolution_anger_timer).sum();
        assert!(anger > anger_before, "player {p:?} has defiance anger");
    }
}

#[test]
fn faithful_mode_is_unchanged_with_the_flag_off() {
    let d = ffh_data::global();
    let o = SetupOptions::new(d, 4, "WORLDSIZE_DUEL", 17);
    let s = ffh_core::setup::GameSetup::from_options(d, &o);
    assert_eq!(s.players.len(), 4 + ffh_core::setup::GameSetup::SPARE_SLOTS + 1, "no Host slots");
    assert!(s.players.iter().all(|p| !p.host));
    let mut g = game(Arc::new(Script::default()), false);
    let target = g.players[T].team;
    assert!(!g.crusades_enabled());
    assert_eq!(g.crusade_call(CrusadeKind::Direct, target, None, Objective::Term, 30, 5, &[A]), None, "no crusade without the flag");
    let terms = LoanTerms { lender: A, borrower: B, target, mission: LoanMission::Attack, strength: 100, duration: 5 };
    assert_eq!(g.loan_check(&terms), Err(ffh_core::crusade::loan::LoanError::Off));
}

// ---- allied loans ----

fn loan_game() -> (Game, Arc<Script>) {
    let script = Arc::new(Script { accept_loans: true, ..Default::default() });
    let mut g = game(script.clone(), true);
    let (tb, tt) = (g.players[B].team, g.players[T].team);
    g.team_declare_war(tb, tt, false, None); // B is at war with T and borrows
    g.players[B].gold = 1000;
    (g, script)
}

fn propose(g: &mut Game, lender: PlayerId, strength: i32, duration: i32) -> CrusadeId {
    let target = g.players[T].team;
    let terms = LoanTerms { lender, borrower: B, target, mission: LoanMission::Attack, strength, duration };
    let p = LoanProposal { terms, from: B, lender_gives: vec![], borrower_gives: vec![] };
    let (answer, id) = g.loan_propose(&p).expect("valid loan");
    assert_eq!(answer, LoanAnswer::Accept);
    id.expect("loan made")
}

#[test]
fn an_accepted_loan_moves_units_to_a_general_at_war_with_the_target_and_the_lender_to_war() {
    let (mut g, _) = loan_game();
    for _ in 0..3 {
        unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    }
    let one = g.crusade_unit_strength(g.players[A].units[0]);
    let id = propose(&mut g, A, 2 * one, 5);
    let host = host_of(&g, id);
    let (ta, tt) = (g.players[A].team, g.players[T].team);
    assert!(g.at_war(g.players[host].team, tt), "the general fights the target");
    assert!(g.at_war(ta, tt), "the lender is at war with the target");
    assert_eq!(g.players[host].units.len(), 2, "the agreed strength was lent");
    assert_eq!(g.players[A].units.len(), 1);
}

#[test]
fn loan_survivors_return_home_with_promotions_when_the_term_ends() {
    let (mut g, _) = loan_game();
    unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    let s = g.crusade_unit_strength(g.players[A].units[0]);
    let id = propose(&mut g, A, s, 4);
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    let promo = g.d.t::<PromotionType>("PROMOTION_COMBAT1");
    g.unit_set_has_promotion(hu, promo, true);
    let gold = g.players[B].gold;
    let end = g.crusade(id).unwrap().lenders[0].term_end;
    for turn in g.game_turn() + 1..=end + 1 {
        g.set_game_turn(turn);
        g.crusade_do_turn();
    }
    assert!(g.players[B].gold < gold, "the borrower paid the upkeep");
    assert!(g.crusade(id).is_none(), "the general dissolved when its only loan ended");
    let back = g.players[A].units[0];
    assert!(g.units[back].has_promotion[promo]);
    let (ta, tt) = (g.players[A].team, g.players[T].team);
    assert!(!g.at_war(ta, tt), "the lender's loan war ended");
}

#[test]
fn two_allies_lending_against_the_same_target_share_one_general() {
    let (mut g, _) = loan_game();
    unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    unit(&mut g, C, "UNIT_WARRIOR", 12, 12);
    let s = g.crusade_unit_strength(g.players[A].units[0]);
    let id1 = propose(&mut g, A, s, 5);
    let id2 = propose(&mut g, C, s, 8);
    assert_eq!(id1, id2, "one general per borrower and target");
    let host = host_of(&g, id1);
    assert_eq!(g.players[host].units.len(), 2, "both allies' units serve under it");
    assert_eq!(g.crusades.len(), 1);
    // Each lender's units leave when its own term ends.
    let end_a = g.crusade(id1).unwrap().lenders.iter().find(|l| l.player == A).unwrap().term_end;
    g.set_game_turn(end_a + 1);
    g.crusade_do_turn();
    assert!(g.crusade(id1).is_some(), "C's loan still runs");
    assert_eq!(g.players[A].units.len(), 1, "A's unit went home");
    assert_eq!(g.players[host].units.len(), 1);
}

#[test]
fn the_borrower_making_peace_ends_the_loan_early() {
    let (mut g, _) = loan_game();
    unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    let s = g.crusade_unit_strength(g.players[A].units[0]);
    let id = propose(&mut g, A, s, 20);
    let (tb, tt) = (g.players[B].team, g.players[T].team);
    g.team_make_peace(tb, tt, true);
    g.crusade_do_turn();
    assert!(g.crusade(id).is_none());
    assert_eq!(g.crusade_reports.last().unwrap().reason, Dissolution::LoansEnded);
    assert_eq!(g.players[A].units.len(), 1, "the units went home");
}

#[test]
fn cities_the_general_captures_go_to_the_borrower() {
    let (mut g, _) = loan_game();
    unit(&mut g, A, "UNIT_WARRIOR", 20, 4);
    let s = g.crusade_unit_strength(g.players[A].units[0]);
    let id = propose(&mut g, A, s, 20);
    let host = host_of(&g, id);
    let hu = g.players[host].units[0];
    let plot = g.map.plot_idx(24, 4).unwrap();
    for d in g.map.plots[plot].units.clone() {
        g.unit_kill(d, false, None);
    }
    g.unit_set_xy(hu, 24, 4, false);
    let city = g.map.plots[plot].plot_city.unwrap();
    assert_eq!(g.cities[city].owner, B);
}

#[test]
fn a_saved_game_keeps_the_crusade() {
    let mut g = game(Arc::new(Script::default()), true);
    let id = call(&mut g, &[A, B]);
    let u = unit(&mut g, A, "UNIT_WARRIOR", 3, 4);
    g.crusade_lend(id, A, &[u]).unwrap();
    let bytes = g.save();
    let h = Game::load(g.d, &bytes).expect("loads");
    assert_eq!(h.crusades, g.crusades);
    assert_eq!(h.save(), bytes);
    let host = host_of(&h, id);
    assert!(h.units[h.players[host].units[0]].lent.is_some());
}

#[test]
fn a_passed_council_war_resolution_calls_members_to_dedicate_instead_of_declaring() {
    let mut g = game(Arc::new(Script::default()), true);
    let d = g.d;
    let over = d.t::<VoteSourceType>("DIPLOVOTE_OVERCOUNCIL");
    let civic = d.vote_sources[over].civic.expect("the Overcouncil has a civic");
    let option = d.civics[civic].civic_option_type.unwrap();
    for p in [A, B, T] {
        g.player_set_civics(p, option, Some(civic));
        assert!(g.player_is_voting_member(p, over));
    }
    let war = d.t::<VoteType>("VOTE_FORCE_WAR");
    let data = ffh_core::structs::VoteTriggeredData {
        id: VoteId(1),
        vote_source: over,
        vote_option: ffh_core::structs::VoteSelectionSubData { vote: war, player: Some(T), city: None, other_player: None, text: String::new() },
    };
    g.process_vote(&data, 1);
    assert_eq!(g.crusades.len(), 1, "the resolution called a crusade");
    let c = &g.crusades[0];
    assert_eq!(c.kind, CrusadeKind::Council(over));
    assert_eq!(c.target, g.players[T].team);
    let called: Vec<PlayerId> = c.calls.iter().map(|c| c.player).collect();
    assert_eq!(called, vec![A, B], "the members other than the target are called");
    assert!(c.calls.iter().all(|c| c.obligation > 0 || g.crusade_military_strength(c.player) == 0));
    let tt = g.players[T].team;
    assert!(!g.at_war(g.players[A].team, tt) && !g.at_war(g.players[B].team, tt), "nobody declared: they dedicate forces or defy");
}

#[test]
fn a_crusade_called_between_turns_does_not_start_the_hosts_turn_out_of_order() {
    let mut g = game(Arc::new(Script::default()), true);
    // (as at the start of a game turn, when the council votes are counted)
    for p in g.player_ids() {
        if g.players[p].turn_active {
            g.player_set_turn_active(p, false, false);
        }
    }
    assert_eq!(g.num_game_turn_active, 0);
    let id = call(&mut g, &[A]);
    let host = host_of(&g, id);
    assert!(!g.players[host].turn_active, "the Host waits for its slot");
    assert_eq!(g.num_game_turn_active, 0);
}
