//! Crusades for the human (docs/CRUSADE.md, "Human play"): the setup flag, a call waiting in
//! `pending` and the end-turn gate, `crusade_answer`, `crusade_lend` (with its war
//! confirmation), the `crusades` query, `propose_loan` both ways (and an AI's counter-offer
//! resent), and a refusal costing council standing at the deadline. Set up with rules APIs on a
//! tiny map; the assertions are about the protocol, not about how the AI plays.

use std::sync::Arc;

use ffh_core::crusade::{CrusadeId, CrusadeKind, Objective};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command, Confirm};
use ffh_play::decision::Decision;
use ffh_play::human::{Answer, FnAnswerer};
use ffh_play::query::Query;
use ffh_play::{NewGame, Session, SessionOptions, Stop};

const TARGET: PlayerId = PlayerId(1);
const ALLY: PlayerId = PlayerId(2);

/// A crusade game (2 AI opponents, tiny map) at the human's control point with a capital, every
/// team meeting every other; the answerer takes option 0.
fn game(seed: u32) -> Session {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, seed);
    ng.opponents = 2;
    ng.world = d.t("WORLDSIZE_TINY");
    ng.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    let answerer = Arc::new(FnAnswerer(|_: &Game, _: &Decision| Answer::Option(0)));
    let mut s = Session::new_game(&ng, answerer, SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).expect("a settler");
    applied(&mut s, Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false });
    let teams: Vec<TeamId> = [h, TARGET, ALLY].iter().map(|&p| s.g.players[p].team).collect();
    for &a in &teams {
        for &b in &teams {
            if a != b {
                s.g.team_meet(a, b, false);
            }
        }
    }
    s
}

fn applied(s: &mut Session, c: Command) -> Option<bool> {
    match s.execute(&c) {
        Ok(CmdOk::Applied { accepted, .. }) => accepted,
        other => panic!("{c:?} -> {other:?}"),
    }
}

/// `n` warriors of `p` in its first city (or where its first unit stands: the AIs found their
/// cities in their own first turn).
fn warriors(s: &mut Session, p: PlayerId, n: usize) -> Vec<UnitId> {
    let g = &mut s.g;
    let (x, y) = match g.players[p].cities.first() {
        Some(&c) => (g.cities[c].x, g.cities[c].y),
        None => {
            let u = g.players[p].units[0];
            (g.units[u].x, g.units[u].y)
        }
    };
    let t = g.d.t::<UnitType>("UNIT_WARRIOR");
    (0..n).map(|_| g.player_init_unit(p, t, x, y, Some(UnitAIType::UNITAI_ATTACK), None)).collect()
}

fn call(s: &mut Session, kind: CrusadeKind, deadline: i32) -> CrusadeId {
    let h = s.human;
    let target = s.g.players[TARGET].team;
    s.g.crusade_call(kind, target, None, Objective::Term, 30, deadline, &[h, ALLY]).expect("a Host slot")
}

#[test]
fn a_call_waits_in_pending_and_the_human_answers_and_lends() {
    let mut s = game(3);
    let h = s.human;
    let mine = warriors(&mut s, h, 3);
    let id = call(&mut s, CrusadeKind::Direct, 5);
    // The call waits for the human (the AI ally answered at once).
    let p = s.pending();
    assert_eq!(p.crusade_calls.len(), 1);
    assert_eq!(p.crusade_calls[0].crusade, id.0);
    assert!(p.crusade_calls[0].lendable >= 3 * s.g.crusade_unit_strength(mine[0]));
    assert!(s.g.crusade(id).unwrap().calls.iter().any(|c| c.player == ALLY && c.answer.is_some()));
    // The end-turn gate counts it.
    match s.execute(&Command::EndTurn { force: false }) {
        Ok(CmdOk::NeedsConfirm(Confirm::EndTurn { crusade_calls, .. })) => assert_eq!(crusade_calls, 1),
        other => panic!("{other:?}"),
    }
    // Answer: pledge.
    let strength = 2 * s.g.crusade_unit_strength(mine[0]);
    applied(&mut s, Command::CrusadeAnswer { crusade: id.0, strength: Some(strength) });
    assert!(s.pending().crusade_calls.is_empty());
    assert!(matches!(s.execute(&Command::CrusadeAnswer { crusade: id.0, strength: None }), Err(CmdErr::Cannot(_))), "answered once");
    let lendable_before = s.query(&Query::Crusades).unwrap().req_arr("lendable").unwrap().len();
    // Lend: lending is war, so the first try asks.
    let target = s.g.players[TARGET].team;
    let lend = |declare_war| Command::CrusadeLend { crusade: id.0, units: mine[..2].to_vec(), declare_war };
    match s.execute(&lend(false)) {
        Ok(CmdOk::NeedsConfirm(Confirm::LendWar { team })) => assert_eq!(team, target),
        other => panic!("{other:?}"),
    }
    assert!(!s.g.at_war(s.g.players[h].team, target), "nothing changed before the confirmation");
    applied(&mut s, lend(true));
    assert!(s.g.at_war(s.g.players[h].team, target), "lending declared war");
    let host = s.g.crusade(id).unwrap().host;
    assert_eq!(s.g.crusade_lent_units(s.g.crusade_index(id).unwrap(), Some(h)).len(), 2);
    assert!(s.g.players[host].units.len() >= 2);
    // A unit that is not ours, or gone, is refused.
    assert!(matches!(s.execute(&Command::CrusadeLend { crusade: id.0, units: mine[..1].to_vec(), declare_war: true }), Err(CmdErr::Gone | CmdErr::NotOwner)));
    // The query shows the human's part.
    let q = s.query(&Query::Crusades).unwrap();
    let cr = &q.req_arr("crusades").unwrap()[0];
    assert_eq!(cr.req::<u32>("id"), Ok(id.0));
    assert_eq!(cr.req::<i32>("delivered"), Ok(strength));
    assert_eq!(cr.req::<usize>("serving"), Ok(2));
    assert_eq!(cr.req_value("call").unwrap().req::<i32>("answer"), Ok(strength));
    assert_eq!(q.req_arr("lendable").unwrap().len(), lendable_before - 2, "the lent units are no longer ours to lend");
    // The text view tells an agent too.
    let text = s.query(&Query::Text).unwrap();
    assert!(text.req_str("text").is_ok_and(|t| t.contains(&format!("Crusade {} against team", id.0))), "{text}");
}

#[test]
fn refusing_a_council_call_defies_the_council_at_the_deadline() {
    let mut s = game(4);
    let h = s.human;
    let vs = VoteSourceType(0);
    s.g.players[h].loyal_member[vs] = true;
    let id = call(&mut s, CrusadeKind::Council(vs), 1);
    applied(&mut s, Command::CrusadeAnswer { crusade: id.0, strength: None });
    let deadline = s.g.crusade(id).unwrap().deadline_turn;
    while s.g.game_turn() <= deadline {
        applied(&mut s, Command::EndTurn { force: true });
        assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    }
    assert!(!s.g.players[h].loyal_member[vs], "the refusal defied the council");
    assert!(s.g.players[h].cities.iter().any(|&c| s.g.cities[c].defy_resolution_anger_timer > 0));
}

#[test]
fn the_human_borrows_and_lends_through_propose_loan() {
    let mut s = game(5);
    let h = s.human;
    let (my, target, ally) = (s.g.players[h].team, s.g.players[TARGET].team, s.g.players[ALLY].team);
    s.g.team_declare_war(my, target, false, None);
    s.g.players[h].gold = 2000;
    // (gold changes hands in deals once a team has the tech for it)
    let d = s.g.d;
    let gold_tech = d.techs.keys().find(|&t| d.techs[t].gold_trading).expect("a gold-trading tech");
    for t in [my, ally] {
        s.g.team_set_has_tech(t, gold_tech, true, None, false, false);
    }
    let theirs = warriors(&mut s, ALLY, 6);
    let one = s.g.crusade_unit_strength(theirs[0]);

    // Borrow, paying gold: accepted, or countered (then the counter is resent and accepted).
    let gold = |n: i32| vec![TradeData { item_type: TradeableItem::TRADE_GOLD, data: n, offering: false, hidden: false }];
    let borrow = |give: Vec<TradeData>| Command::ProposeLoan { to: ALLY, lend: false, target, defend: false, strength: one, duration: 6, give, get: vec![], units: vec![] };
    let accepted = match s.execute(&borrow(gold(600))) {
        Ok(CmdOk::Applied { accepted, .. }) => accepted,
        Ok(CmdOk::NeedsConfirm(c @ Confirm::LoanCounter { .. })) => {
            let cmd = Command::from_json(s.g.d, c.to_json().req_value("cmd").unwrap()).unwrap();
            applied(&mut s, cmd)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(accepted, Some(true), "the ally agreed");
    let general = s.g.loan_general(h, target).expect("a general serves the human's war");
    let host = s.g.crusade(general).unwrap().host;
    assert!(!s.g.players[host].units.is_empty(), "the ally's units serve");
    assert!(s.g.at_war(ally, target), "lending put the ally at war");
    assert!(s.g.players[h].gold < 2000, "the human paid");

    // A loan that cannot be made is refused with the reason.
    let bad = Command::ProposeLoan { to: ALLY, lend: false, target: ally, defend: false, strength: one, duration: 6, give: vec![], get: vec![], units: vec![] };
    assert!(matches!(s.execute(&bad), Err(CmdErr::Cannot(_))));

    // Lend: the human names its units; the borrower (the ally, now at war with the target, which
    // we make stronger) takes them for nothing.
    warriors(&mut s, TARGET, 12);
    let mine = warriors(&mut s, h, 2);
    let lend = Command::ProposeLoan { to: ALLY, lend: true, target, defend: true, strength: one, duration: 4, give: vec![], get: vec![], units: vec![mine[0]] };
    match s.execute(&lend) {
        Ok(CmdOk::Applied { accepted: Some(true), .. }) => {
            let general = s.g.loan_general(ALLY, target).expect("the ally's general");
            let idx = s.g.crusade_index(general).unwrap();
            assert_eq!(s.g.crusade_lent_units(idx, Some(h)).len(), 1, "exactly the named unit was lent");
            assert!(s.g.units.get(mine[1]).is_some_and(|u| u.owner == h), "the other stayed home");
        }
        // The AI judges its own need; refusing is a valid answer too, and changes nothing.
        Ok(CmdOk::Applied { accepted: Some(false), .. }) => assert!(s.g.loan_general(ALLY, target).is_none()),
        other => panic!("{other:?}"),
    }
}

/// Troops in the trade screen (docs/CRUSADE.md, "Allied loans"): the `trade` query offers a
/// loan each way against the borrower's enemies, with the human's lendable units; the selection
/// has no lend order (lending is done from a screen: a crusade's lend dialog, or a loan in a
/// deal); a loan proposed with the units picked lends exactly those, priced by the rest of the
/// deal.
#[test]
fn troops_are_an_item_of_the_trade_screen() {
    let mut s = game(6);
    let h = s.human;
    let mine = warriors(&mut s, h, 2);
    let (target, ally) = (s.g.players[TARGET].team, s.g.players[ALLY].team);
    s.g.team_declare_war(ally, target, false, None);
    let t = s.query(&Query::Trade { with: ALLY }).unwrap();
    let troops = t.req_value("troops").unwrap();
    let lend = troops.req_value("lend").unwrap();
    assert_eq!(lend.req::<Vec<TeamId>>("targets"), Ok(vec![target]), "{troops}");
    let units: Vec<UnitId> = lend.req_arr("units").unwrap().iter().map(|u| u.req::<UnitId>("unit").unwrap()).collect();
    assert!(mine.iter().all(|u| units.contains(u)), "{troops}");
    let borrow = troops.req_value("borrow").unwrap();
    assert!(borrow.req::<Vec<TeamId>>("targets").unwrap().is_empty(), "the human is at war with no one: {troops}");
    assert!(borrow.req_str("why_not").is_ok(), "{troops}");
    // No lend order on the selection.
    let r = s.query(&Query::Group { units: mine.clone() }).unwrap();
    assert!(r.req_arr("actions").unwrap().iter().all(|a| a.req_str("kind") != Ok("crusade")));
    // The loan as the trade screen sends it: our units picked, a gift with it.
    let one = s.g.crusade_unit_strength(mine[0]);
    let cmd = Command::ProposeLoan { to: ALLY, lend: true, target, defend: false, strength: 2 * one, duration: 5, give: vec![], get: vec![], units: mine.clone() };
    let accepted = match s.execute(&cmd) {
        Ok(CmdOk::Applied { accepted, .. }) => accepted,
        Ok(CmdOk::NeedsConfirm(c @ Confirm::LoanCounter { .. })) => {
            let cmd = Command::from_json(s.g.d, c.to_json().req_value("cmd").unwrap()).unwrap();
            applied(&mut s, cmd)
        }
        other => panic!("{other:?}"),
    };
    if accepted == Some(true) {
        let id = s.g.loan_general(ALLY, target).expect("the general");
        let host = s.g.crusade(id).unwrap().host;
        let lent: Vec<UnitId> = s.g.players[host].units.clone();
        assert_eq!(lent.len(), 2, "exactly the picked units serve");
        assert!(lent.iter().all(|&u| s.g.units[u].lent.as_ref().is_some_and(|r| r.lender == h)));
    }
}
