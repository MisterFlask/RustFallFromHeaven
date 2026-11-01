//! Crusades and allied loans (docs/CRUSADE.md, "Host behaviour", "AI decisions"; only runs
//! when the ruleset turns crusades on).
//!
//! - **The Host** (a crusade's own AI player) runs the suggested state machine, its state kept in
//!   `Crusade::ai` so saves resume it: *Muster* (gather the delivered units at a rally point: the
//!   lender's or victim's city nearest the objective) until the mustered strength reaches
//!   `muster_percent` of the pledges or the pledge deadline passes; *March* toward the current
//!   objective; *Engage* (assault the objective city and enemy units next to the army when the
//!   odds are worth it, else besiege next to it); *Recover* (heal in the nearest lender or victim
//!   city, at most `recover_turns`) when the army's average health falls below
//!   `recover_health_percent`. It never sits idle except to muster or heal; one unit garrisons
//!   each city it holds.
//! - **A civ's one decision** is how much to lend ([`pledge`]): the value of lending a strength
//!   `S` is expected spoils + goal value - expected unit losses - homeland risk - the standing a
//!   shortfall still costs, kept as separate terms ([`LendValue`]) so they can be tuned and
//!   traced. It lends in increments while the next adds value, keeping a home-defence floor;
//!   pledging the best amount or refusing outright, whichever costs less, falls out of the same
//!   maximum (refusing is the value at 0). Pledged units are delivered on its turns
//!   ([`deliver`]): army units, strongest first, the last one the weakest that covers the rest;
//!   garrisons and a home-defence floor are kept.
//! - **Draft**: cities near our own are worth more; one on another landmass, or more than twice
//!   [`FAR_CITY`] plots from our cities, is more burden than prize and left for wealth.
//! - **Loans**: a lender accepts when payment plus goal value beat losses and risk, and counters
//!   with the gold it would need; a borrower at war with a stronger enemy asks friendly civs.

use ffh_core::crusade::loan::{LoanAnswer, LoanMission, LoanProposal, LoanTerms};
use ffh_core::crusade::{CrusadeId, CrusadeKind, HostAi, HostState, Objective, Pledge};
use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;

use crate::military::{is_army_unit, MAX_STACK};
use crate::units::{alive, hold, move_to, push, skip};
use crate::view::View;

/// A draft city farther than this from our cities is left for wealth.
pub const FAR_CITY: i32 = 10;
/// Expected share of lent strength lost in a crusade.
const LOSS_PERCENT: i64 = 50;
/// Share of our strength always kept home (more when the target borders us).
const HOME_FLOOR_PERCENT: i64 = 35;

// ----------------------------------------------------------------------------
// Valuation
// ----------------------------------------------------------------------------

/// Gold value of one strength point of `p`'s army (production cost per strength, x2 for gold).
fn gold_per_strength_x100(g: &Game, p: PlayerId) -> i64 {
    let (mut cost, mut strength) = (0i64, 0i64);
    for &u in &g.players[p].units {
        if g.units.contains(u) && g.crusade_unit_lendable(u) {
            cost += g.d.units[g.units[u].unit_type].production_cost.max(0) as i64;
            strength += g.crusade_unit_strength(u) as i64;
        }
    }
    if strength <= 0 {
        return 30; // (no army: 0.3 gold a strength point, 30 a strength-1 unit)
    }
    (cost * 2 * 100 / strength).max(10)
}

/// Military strength of team `t` (its alive players' lendable strength, which is every combat
/// land unit).
fn team_strength(g: &Game, t: TeamId) -> i64 {
    g.player_ids().filter(|&q| g.players[q].alive && g.players[q].team == t).map(|q| g.crusade_military_strength(q) as i64).sum()
}

/// Distance from `p`'s capital (or first city) to `t`'s nearest city (map max if none).
fn distance_to_team(g: &Game, p: PlayerId, t: TeamId) -> i32 {
    let Some(&home) = g.players[p].capital_city.iter().chain(g.players[p].cities.iter()).find(|&&c| g.cities.contains(c)) else {
        return g.map.max_plot_distance();
    };
    let (hx, hy) = (g.cities[home].x, g.cities[home].y);
    g.player_ids()
        .filter(|&q| g.players[q].alive && g.players[q].team == t)
        .flat_map(|q| g.players[q].cities.iter().map(|&c| g.plot_distance(hx, hy, g.cities[c].x, g.cities[c].y)))
        .min()
        .unwrap_or_else(|| g.map.max_plot_distance())
}

/// The separate terms of the value of lending a strength (gold equivalents).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LendValue {
    pub spoils: i64,
    pub goal: i64,
    pub losses: i64,
    pub risk: i64,
    /// Standing still lost to a shortfall against the obligation (all of it at 0).
    pub standing: i64,
}

impl LendValue {
    pub fn total(&self) -> i64 {
        self.spoils + self.goal - self.losses - self.risk - self.standing
    }
}

/// What `p` knows about crusade `idx` for valuing its share.
struct Outlook {
    /// Strength the other participants are expected to bring.
    others: i64,
    /// Estimated draft pool value (the target's cities the crusade can expect to take, plus
    /// plunder).
    pool: i64,
    /// Value of the target's defeat for `p`'s own goals.
    goal: i64,
    obligation: i64,
    /// Full standing lost by refusing.
    refusal: i64,
    gold_per_strength: i64,
    /// Homeland exposure per strength lent (gold per strength point x100).
    risk_per_strength: i64,
    our_strength: i64,
    at_war: bool,
    draft: bool,
}

fn outlook(g: &Game, p: PlayerId, idx: usize) -> Outlook {
    let c = &g.crusades[idx];
    let target = c.target;
    let team = g.players[p].team;
    let obligation = c.calls.iter().find(|x| x.player == p).map_or(0, |x| x.obligation) as i64;
    let others: i64 = c
        .calls
        .iter()
        .filter(|x| x.player != p)
        .map(|x| match x.answer {
            Some(Pledge::Strength(s)) => s as i64,
            Some(Pledge::Refuse) => 0,
            None => x.obligation as i64 / 2,
        })
        .sum::<i64>()
        + c.lenders.iter().filter(|l| l.player != p).map(|l| l.delivered as i64).sum::<i64>();
    let target_strength = team_strength(g, target).max(1);
    let cities: Vec<CityId> = g.player_ids().filter(|&q| g.players[q].alive && g.players[q].team == target).flat_map(|q| g.players[q].cities.clone()).collect();
    let city_values: i64 = cities.iter().map(|&x| g.crusade_city_value(x) as i64).sum();
    // The crusade takes a share of the target's cities that grows with the force against it.
    let force = others + obligation.max(1);
    let take_percent = (force * 50 / (target_strength * 3 / 2)).clamp(0, 50);
    let pool = city_values * take_percent / 100 + 100;
    let mut goal = 0i64;
    if g.at_war(team, target) {
        goal += 150; // we fight it anyway: a second army helps
    }
    let plan = crate::victory::plan(g, p);
    if plan.rival.is_some_and(|(r, _)| g.players[r].team == target) {
        goal += 300; // the rival for our victory
    }
    if let Some(leader) = g.team_get_leader_id(target) {
        if crate::diplomacy::relation(g, p, leader) < 0 {
            goal += 100;
        }
    }
    let refusal = match c.kind {
        CrusadeKind::Council(_) => 150 + 40 * g.players[p].cities.len() as i64,
        _ => 0,
    };
    let our_strength = g.crusade_military_strength(p) as i64;
    let gps = gold_per_strength_x100(g, p);
    // Exposure: a close, strong target can strike what we left behind.
    let dist = distance_to_team(g, p, target) as i64;
    let max = g.map.max_plot_distance().max(1) as i64;
    let near = (100 - dist * 100 / max).max(0);
    let ratio = (target_strength * 100 / our_strength.max(1)).min(300);
    let risk_per_strength = gps * near * ratio / 100 / 100 / if g.at_war(team, target) { 3 } else { 1 };
    Outlook {
        others,
        pool,
        goal,
        obligation,
        refusal,
        gold_per_strength: gps,
        risk_per_strength,
        our_strength,
        at_war: g.at_war(team, target),
        draft: !matches!(c.kind, CrusadeKind::Loan { .. }),
    }
}

fn lend_value(o: &Outlook, s: i64) -> LendValue {
    let spoils = if o.draft && s > 0 { o.pool * s / (o.others + s).max(1) } else { 0 };
    // The goal value is what our part adds to the target's defeat, up to the obligation's worth.
    let goal = if o.obligation > 0 { o.goal * s.min(o.obligation) / o.obligation } else if s > 0 { o.goal / 2 } else { 0 };
    let losses = s * LOSS_PERCENT / 100 * o.gold_per_strength / 100;
    let risk = s * o.risk_per_strength / 100 + if s > 0 && !o.at_war { o.risk_per_strength * o.our_strength / 1000 } else { 0 };
    let standing = if o.obligation > 0 { o.refusal * (o.obligation - s).max(0) / o.obligation } else if s == 0 { o.refusal } else { 0 };
    LendValue { spoils, goal, losses, risk, standing }
}

/// Strength `p` may lend at most (the rest is the home-defence floor and garrisons).
fn lendable_strength(g: &Game, p: PlayerId) -> i64 {
    spare_units(g, p).iter().map(|&u| g.crusade_unit_strength(u) as i64).sum()
}

/// `Controller::crusade_pledge`: the best amount to pledge, or a refusal.
pub fn pledge(g: &Game, p: PlayerId, id: CrusadeId) -> Option<Pledge> {
    let idx = g.crusade_index(id)?;
    if g.crusade_can_lend(id, p).is_err() {
        return Some(Pledge::Refuse);
    }
    let o = outlook(g, p, idx);
    let max = lendable_strength(g, p);
    let step = (o.obligation / 4).max(100).max(max / 8);
    let mut best = (lend_value(&o, 0).total(), 0i64);
    let mut s = 0;
    // Increments while the next one adds value (the value is not concave in general: keep the
    // best seen up to the floor).
    while s + step <= max {
        s += step;
        let v = lend_value(&o, s).total();
        if v > best.0 {
            best = (v, s);
        }
    }
    crate::trace!(g, p, "crusade {} pledge: obligation {} lendable {} best {} ({:?})", id.0, o.obligation, max, best.1, lend_value(&o, best.1));
    if best.1 == 0 {
        Some(Pledge::Refuse)
    } else {
        Some(Pledge::Strength(best.1 as i32))
    }
}

// ----------------------------------------------------------------------------
// Delivering units
// ----------------------------------------------------------------------------

/// `p`'s units it can lend without baring its cities: lendable units except one defender per
/// city (two in the capital and in cities the target borders), city-defense units in cities,
/// casters and units in danger of being cut off.
pub fn spare_units(g: &Game, p: PlayerId) -> Vec<UnitId> {
    let mut keep_in_city: std::collections::BTreeMap<PlotIdx, i32> = std::collections::BTreeMap::new();
    for &c in &g.players[p].cities {
        keep_in_city.insert(g.city_plot(c), 1 + i32::from(g.city_is_capital(c)));
    }
    let mut candidates: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units.contains(u) && g.crusade_unit_lendable(u)).collect();
    // Garrisons keep their best defenders.
    candidates.sort_by_key(|&u| (std::cmp::Reverse(g.units[u].base_combat_defense), u));
    let mut out = Vec::new();
    let total: i64 = candidates.iter().map(|&u| g.crusade_unit_strength(u) as i64).sum();
    let mut kept: i64 = 0;
    for u in candidates {
        let plot = g.unit_plot(u);
        let ai = g.units[u].ai_type;
        if let Some(k) = keep_in_city.get_mut(&plot) {
            if *k > 0 {
                *k -= 1;
                kept += g.crusade_unit_strength(u) as i64;
                continue;
            }
        }
        if matches!(ai, Some(UnitAIType::UNITAI_CITY_DEFENSE) | Some(UnitAIType::UNITAI_CITY_SPECIAL) | Some(UnitAIType::UNITAI_MAGE) | Some(UnitAIType::UNITAI_WARWIZARD) | Some(UnitAIType::UNITAI_MEDIC) | Some(UnitAIType::UNITAI_TERRAFORMER) | Some(UnitAIType::UNITAI_MANA_UPGRADE) | Some(UnitAIType::UNITAI_WORKER) | Some(UnitAIType::UNITAI_SETTLE)) {
            kept += g.crusade_unit_strength(u) as i64;
            continue;
        }
        out.push(u);
    }
    // Home-defence floor: a share of the whole army stays.
    let floor = total * HOME_FLOOR_PERCENT / 100;
    out.sort_by_key(|&u| (!is_army_unit(g, u), std::cmp::Reverse(g.crusade_unit_strength(u)), u));
    let mut lendable: i64 = out.iter().map(|&u| g.crusade_unit_strength(u) as i64).sum();
    while kept < floor && !out.is_empty() {
        let u = out.pop().expect("non-empty");
        let s = g.crusade_unit_strength(u) as i64;
        kept += s;
        lendable -= s;
    }
    let _ = lendable;
    out
}

/// Deliver the pledged strength of every crusade `p` pledged to (its own turn, before its units
/// move): army units first, strongest first, nearest the target among equals.
pub fn deliver(g: &mut Game, p: PlayerId) {
    if g.crusades.is_empty() {
        return;
    }
    let ids: Vec<(CrusadeId, i32)> = g
        .crusades
        .iter()
        .filter(|c| !c.dissolving)
        .filter_map(|c| c.calls.iter().find(|x| x.player == p).and_then(|x| match x.answer {
            Some(Pledge::Strength(s)) => Some((c.id, s)),
            _ => None,
        }))
        .collect();
    for (id, pledged) in ids {
        let Some(idx) = g.crusade_index(id) else { continue };
        let owed = pledged - g.crusade_delivered(idx, p);
        if owed <= 0 || g.crusade_can_lend(id, p).is_err() {
            continue;
        }
        let units = pick_units(g, p, owed as i64);
        if units.is_empty() {
            continue;
        }
        let result = g.crusade_lend(id, p, &units);
        crate::trace!(g, p, "crusade {} delivers {} units: {:?}", id.0, units.len(), result);
    }
}

/// Spare units of `p` reaching `strength` with the least overshoot: while one spare unit covers
/// what is left, the weakest such unit; otherwise the strongest (army units first).
pub fn pick_units(g: &Game, p: PlayerId, strength: i64) -> Vec<UnitId> {
    let mut spare = spare_units(g, p);
    let mut out = Vec::new();
    let mut left = strength;
    while left > 0 && !spare.is_empty() {
        let s = |u: UnitId| g.crusade_unit_strength(u) as i64;
        let i = match spare.iter().enumerate().filter(|&(_, &u)| s(u) >= left).min_by_key(|&(_, &u)| (s(u), u)) {
            Some((i, _)) => i,
            None => 0, // (strongest first: `spare_units` sorts army units by strength)
        };
        let u = spare.remove(i);
        left -= s(u);
        out.push(u);
    }
    out
}

// ----------------------------------------------------------------------------
// Draft
// ----------------------------------------------------------------------------

/// `Controller::crusade_draft_pick`: the near city worth most within the budget (or, for a city
/// next to our lands, an affordable overdraw); `None` takes wealth.
pub fn draft_pick(g: &Game, p: PlayerId, budget: i32, offered: &[(CityId, i32)]) -> Option<CityId> {
    crate::trace!(g, p, "draft: budget {} offered {:?}", budget, offered.iter().map(|&(c, v)| (g.map.xy(g.city_plot(c)), v)).collect::<Vec<_>>());
    let ours: Vec<(i32, i32, Option<AreaId>)> = g.players[p].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y, g.map.plots[g.city_plot(c)].area)).collect();
    let mut best: Option<(i64, CityId)> = None;
    for &(c, v) in offered {
        let area = g.map.plots[g.city_plot(c)].area;
        let Some(dist) = ours.iter().filter(|o| o.2 == area).map(|&(x, y, _)| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y)).min() else { continue };
        if dist > 2 * FAR_CITY {
            continue;
        }
        let over = (v - budget).max(0);
        // Overdraw for a city within reach, paying at most half our treasury (the alternative is
        // a pro rata cut of gold that is short whenever a city goes unclaimed).
        if over > 0 && (dist > FAR_CITY || over > g.players[p].gold / 2) {
            continue;
        }
        // A near city is worth more than its draft price, a far one less (but the share's gold
        // is short when cities go unclaimed, so a city on our landmass still beats it).
        let worth = v as i64 * (3 * FAR_CITY as i64 - dist as i64) / (2 * FAR_CITY as i64) - over as i64;
        if worth <= 0 {
            continue;
        }
        if best.is_none_or(|(bw, bc)| (worth, std::cmp::Reverse(c)) > (bw, std::cmp::Reverse(bc))) {
            best = Some((worth, c));
        }
    }
    best.map(|(_, c)| c)
}

// ----------------------------------------------------------------------------
// Loans
// ----------------------------------------------------------------------------

fn gold_of(items: &[TradeData]) -> i64 {
    items.iter().filter(|t| t.item_type == TradeableItem::TRADE_GOLD).map(|t| t.data.max(0) as i64).sum::<i64>()
        + items.iter().filter(|t| t.item_type == TradeableItem::TRADE_GOLD_PER_TURN).map(|t| t.data.max(0) as i64 * 10).sum::<i64>()
}

/// What lending `t.strength` costs `lender` (losses and risk) less what helping the borrower and
/// fighting the target are worth to it, in gold.
fn loan_cost(g: &Game, lender: PlayerId, t: &LoanTerms) -> i64 {
    let gps = gold_per_strength_x100(g, lender);
    let s = t.strength as i64;
    let losses = s * LOSS_PERCENT / 100 * gps / 100 * t.duration.min(20) as i64 / 20;
    let team = g.players[lender].team;
    let dist = distance_to_team(g, lender, t.target) as i64;
    let max = g.map.max_plot_distance().max(1) as i64;
    let near = (100 - dist * 100 / max).max(0);
    let ours = g.crusade_military_strength(lender).max(1) as i64;
    let ratio = (team_strength(g, t.target) * 100 / ours).min(300);
    let risk = if g.at_war(team, t.target) { 0 } else { gps * near * ratio / 100 / 100 * ours / 1000 + s * gps * near / 100 / 200 };
    let mut help = 0;
    if crate::diplomacy::relation(g, lender, t.borrower) > 0 {
        help += 80;
    }
    if g.at_war(team, t.target) {
        help += s * gps / 100 / 4;
    }
    losses + risk - help
}

/// `Controller::consider_loan`.
pub fn consider_loan(g: &Game, p: PlayerId, prop: &LoanProposal) -> LoanAnswer {
    let t = &prop.terms;
    if g.loan_check(t).is_err() {
        return LoanAnswer::Refuse;
    }
    if p == t.lender {
        if lendable_strength(g, p) < t.strength as i64 {
            return LoanAnswer::Refuse;
        }
        let cost = loan_cost(g, p, t);
        let paid = gold_of(&prop.borrower_gives) - gold_of(&prop.lender_gives);
        crate::trace!(g, p, "loan to {} of strength {} for {}: cost {} paid {} (borrower gold {})", t.borrower.0, t.strength, t.duration, cost, paid, g.players[t.borrower].gold);
        if paid >= cost {
            return LoanAnswer::Accept;
        }
        let want = (cost - gold_of(&prop.lender_gives)).max(1);
        if want <= g.players[t.borrower].gold as i64 {
            let mut counter = prop.clone();
            counter.from = p;
            counter.borrower_gives.retain(|x| x.item_type != TradeableItem::TRADE_GOLD);
            counter.borrower_gives.push(TradeData { item_type: TradeableItem::TRADE_GOLD, data: want as i32, offering: false, hidden: false });
            return LoanAnswer::Counter(counter);
        }
        LoanAnswer::Refuse
    } else {
        // An offer to lend to us: worth it while we fight a stronger enemy, for what we can spare.
        let us = team_strength(g, g.players[p].team);
        let them = team_strength(g, t.target);
        let price = gold_of(&prop.borrower_gives) - gold_of(&prop.lender_gives);
        if them * 100 >= us * 110 && price <= (g.players[p].gold / 3) as i64 {
            LoanAnswer::Accept
        } else {
            LoanAnswer::Refuse
        }
    }
}

/// A borrower at war with a stronger civ asks one friendly civ a turn for a loan (every few
/// turns per enemy), paying what it can.
pub fn ask_for_loans(g: &mut Game, p: PlayerId) {
    let turn = g.game_turn();
    if (turn + p.0 as i32) % 5 != 0 || g.players[p].cities.is_empty() {
        return;
    }
    let team = g.players[p].team;
    let us = team_strength(g, team);
    let enemies: Vec<TeamId> = g.civ_team_ids().filter(|&t| t != team && g.team_is_alive(t) && g.at_war(team, t) && !g.team_is_crusade_host(t) && !g.team_is_barbarian(t)).collect();
    for enemy in enemies {
        let them = team_strength(g, enemy);
        crate::trace!(g, p, "loans: at war with team {} (strength {} vs ours {})", enemy.0, them, us);
        if them * 100 < us * 120 || g.loan_general(p, enemy).is_some() {
            continue;
        }
        let deficit = (them - us).max(100);
        let lenders: Vec<PlayerId> = g
            .civ_player_ids()
            .filter(|&l| l != p && g.players[l].alive && !g.player_is_crusade_host(l) && crate::diplomacy::relation(g, p, l) > 0 && g.player_can_contact(p, l))
            .collect();
        for l in lenders {
            let strength = (lendable_strength(g, l) / 2).min(deficit) as i32;
            if strength < 200 {
                continue;
            }
            let ours_in_danger = g.players[p].cities.iter().any(|&c| g.map.plots[g.city_plot(c)].units.iter().any(|&u| g.unit_get_team(u) == enemy));
            let mission = if ours_in_danger { LoanMission::Defend } else { LoanMission::Attack };
            // The borrower pays the upkeep each turn: borrow only what it can keep paying.
            let offer = (g.players[p].gold / 4).max(0);
            let per_unit = g.crusade_rules().loan_upkeep_gold.max(1);
            let unit_strength = (lendable_strength(g, l) / spare_units(g, l).len().max(1) as i64).max(1);
            let units = (strength as i64 / unit_strength).max(1);
            let duration = ((g.players[p].gold - offer) as i64 / (units * per_unit as i64)).min(10) as i32;
            if duration < 4 {
                continue;
            }
            let terms = LoanTerms { lender: l, borrower: p, target: enemy, mission, strength, duration };
            if let Err(e) = g.loan_check(&terms) {
                crate::trace!(g, p, "loans: {} cannot lend {}: {:?}", l.0, strength, e);
                continue;
            }
            let borrower_gives = if offer > 0 { vec![TradeData { item_type: TradeableItem::TRADE_GOLD, data: offer, offering: false, hidden: false }] } else { vec![] };
            let prop = LoanProposal { terms, from: p, lender_gives: vec![], borrower_gives };
            match g.loan_propose(&prop) {
                Ok((LoanAnswer::Accept, _)) => {
                    crate::trace!(g, p, "loan from {} against team {} agreed", l.0, enemy.0);
                    return;
                }
                Ok((LoanAnswer::Counter(c), _)) => {
                    if gold_of(&c.borrower_gives) <= (g.players[p].gold / 2) as i64 && g.loan_agree(&c).is_ok() {
                        crate::trace!(g, p, "loan from {} against team {} agreed on its counter", l.0, enemy.0);
                    }
                    return;
                }
                other => crate::trace!(g, p, "loans: {} answered {:?}", l.0, other),
            }
        }
    }
}

// ----------------------------------------------------------------------------
// The Host
// ----------------------------------------------------------------------------

fn health_percent(g: &Game, u: UnitId) -> i32 {
    let max = g.d.defines.max_hit_points.max(1);
    (max - g.units[u].damage).max(0) * 100 / max
}

/// The Host's army: its combat land units on the map.
fn army(g: &Game, p: PlayerId) -> Vec<UnitId> {
    g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u) && !g.units[u].death_delay && g.unit_can_fight(u) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && !g.unit_is_cargo(u))
        .collect()
}

/// Is the city on `plot` held by an enemy of crusade `idx`?
fn enemy_city(g: &Game, idx: usize, plot: PlotIdx) -> bool {
    g.map.plots[plot].plot_city.is_some_and(|c| g.cities.contains(c) && g.crusade_is_enemy_team(idx, g.players[g.cities[c].owner].team))
}

/// The objective: the enemy city the Host goes for next, nearest `from` (the previous one while
/// it stands).
fn objective(g: &Game, idx: usize, from: PlotIdx) -> Option<PlotIdx> {
    let c = &g.crusades[idx];
    if let Some(goal) = c.ai.goal.filter(|&q| enemy_city(g, idx, q)) {
        return Some(goal);
    }
    let host_team = g.players[c.host].team;
    let area = g.map.plots[from].area;
    let (fx, fy) = g.map.xy(from);
    let listed: Vec<PlotIdx> = match &c.objective {
        Objective::Capture(plots) => plots.clone(),
        Objective::Recover => c.victim_plots.clone(),
        Objective::Term => Vec::new(),
    };
    let mut candidates: Vec<PlotIdx> = listed.into_iter().filter(|&q| enemy_city(g, idx, q)).collect();
    if candidates.is_empty() && (!c.defend || !c.victim_plots.is_empty()) {
        let defend_plots: Vec<PlotIdx> = c.victim_plots.iter().copied().filter(|&q| enemy_city(g, idx, q)).collect();
        candidates = if c.defend && !defend_plots.is_empty() {
            defend_plots
        } else {
            g.player_ids()
                .filter(|&q| g.players[q].alive && g.crusade_is_enemy_team(idx, g.players[q].team))
                .flat_map(|q| g.players[q].cities.iter().map(|&x| g.city_plot(x)))
                .filter(|&q| g.map.plots[q].revealed[host_team])
                .collect()
        };
    }
    // The nearest city the army can take (its attack power beats the defense we see by a
    // third), else the least defended one.
    let power: i64 = army(g, c.host).iter().map(|&u| crate::view::attack_power(g, u) as i64).sum();
    let host_units = army(g, c.host);
    let covers = |q: PlotIdx| -> bool {
        if !g.plot_is_visible(q, host_team) {
            return false;
        }
        let defs = crate::combat_sim::visible_defenders(g, host_team, q);
        crate::combat_sim::simulate(g, q, &defs, &host_units, 0, crate::combat_sim::Order::Strike, 100, 100).used.is_some()
    };
    let defense = |q: PlotIdx| -> i64 {
        crate::combat_sim::visible_defenders(g, host_team, q).iter().map(|&u| crate::view::defense_power(g, u) as i64).sum::<i64>()
            * (100 + g.map.plots[q].plot_city.map_or(0, |x| g.city_defense_modifier(x, false)) as i64)
            / 100
    };
    let pick = candidates
        .into_iter()
        .filter(|&q| g.map.plots[q].area == area)
        .min_by_key(|&q| {
            let (x, y) = g.map.xy(q);
            let dfn = defense(q);
            let feasible = covers(q) || !g.plot_is_visible(q, host_team) && power * 3 >= dfn * 4;
            (!feasible, if feasible { 0 } else { dfn }, g.plot_distance(fx, fy, x, y), q)
        });
    if let Some(q) = pick {
        crate::trace!(g, c.host, "host objective {:?}: power {} vs defense {}", g.map.xy(q), power, defense(q));
    }
    pick
}

/// Friendly ground for the Host: its lenders' and the victim's cities (and its own).
fn friendly_cities(g: &Game, idx: usize) -> Vec<CityId> {
    let c = &g.crusades[idx];
    let mut owners: Vec<PlayerId> = c.lenders.iter().map(|l| l.player).collect();
    owners.extend(c.victim);
    owners.push(c.host);
    owners.iter().filter(|&&q| g.players[q].alive).flat_map(|&q| g.players[q].cities.clone()).collect()
}

fn nearest_city_to(g: &Game, cities: &[CityId], to: PlotIdx, area: Option<AreaId>) -> Option<CityId> {
    let (x, y) = g.map.xy(to);
    cities.iter().copied().filter(|&c| g.map.plots[g.city_plot(c)].area == area).min_by_key(|&c| (g.plot_distance(x, y, g.cities[c].x, g.cities[c].y), c))
}

/// The plot next to `goal` the army stands on to strike: free of enemies, reachable, best
/// defense, nearest `from`.
fn staging(g: &Game, head: UnitId, goal: PlotIdx, from: PlotIdx) -> Option<PlotIdx> {
    let (gx, gy) = g.map.xy(goal);
    let (fx, fy) = g.map.xy(from);
    let team = g.unit_get_team(head);
    let mut best: Option<(i32, PlotIdx)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(gx, gy, DirectionType::from_usize(dir)) else { continue };
        if g.plot_is_impassable(q) || g.plot_is_water(q) || g.map.plots[q].area != g.map.plots[goal].area {
            continue;
        }
        if g.map.plots[q].units.iter().any(|&o| g.units.contains(o) && g.at_war(g.unit_get_team(o), team)) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        let score = g.plot_distance(fx, fy, x, y) * 10 - g.plot_defense_modifier(q, Some(team), false) / 10;
        if best.is_none_or(|(s, bq)| (score, q) < (s, bq)) {
            best = Some((score, q));
        }
    }
    best.map(|(_, q)| q)
}

/// The plot next to `goal` holding most of the Host's army (where the other stacks join it).
fn siege_camp(g: &Game, idx: usize, goal: PlotIdx, from: PlotIdx) -> Option<PlotIdx> {
    let host = g.crusades[idx].host;
    let (gx, gy) = g.map.xy(goal);
    let (fx, fy) = g.map.xy(from);
    let mut best: Option<(usize, i32, PlotIdx)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(gx, gy, DirectionType::from_usize(dir)) else { continue };
        let n = g.map.plots[q].units.iter().filter(|&&o| g.units.contains(o) && g.units[o].owner == host).count();
        if n == 0 {
            continue;
        }
        let (x, y) = g.map.xy(q);
        let d = g.plot_distance(fx, fy, x, y);
        if best.is_none_or(|(bn, bd, bq)| (n, -d, std::cmp::Reverse(q)) > (bn, -bd, std::cmp::Reverse(bq))) {
            best = Some((n, d, q));
        }
    }
    best.map(|(_, _, q)| q)
}

/// Merge the Host's fresh units on each plot into stacks of up to `MAX_STACK`.
fn merge_stacks(g: &mut Game, p: PlayerId) {
    let mut by_plot: std::collections::BTreeMap<PlotIdx, Vec<UnitId>> = std::collections::BTreeMap::new();
    for u in army(g, p) {
        by_plot.entry(g.unit_plot(u)).or_default().push(u);
    }
    for (plot, units) in by_plot {
        let mut lead: Option<GroupId> = None;
        for u in units {
            let Some(grp) = g.units[u].group else { continue };
            let fresh = g.unit_moves_left(u) == g.unit_max_moves(u);
            match lead {
                Some(l) if l != grp && fresh && g.groups[l].units.len() < MAX_STACK && g.unit_can_join_group(u, plot, l) => {
                    g.unit_join_group(u, Some(l), true);
                }
                Some(l) if g.groups[l].units.len() >= MAX_STACK => lead = Some(grp),
                None => lead = Some(grp),
                _ => {}
            }
        }
    }
}

/// The Host's whole turn.
pub fn host_turn(g: &mut Game, p: PlayerId) {
    let Some(idx) = g.crusade_of_host(p) else { return };
    let id = g.crusades[idx].id;
    let rules = g.crusade_rules().clone();
    let turn = g.game_turn();
    let units = army(g, p);
    if units.is_empty() {
        crate::units::finalize(g, p);
        return;
    }
    // The state machine moves once a turn, on the first call (the rules call again while units
    // still move).
    let first_call = !units.iter().any(|&u| g.unit_has_moved(u));
    merge_stacks(g, p);
    let v = View::new(g, p);

    // Garrisons: one unit holds each city the Host holds (its best defender there).
    let mut garrison: Vec<UnitId> = Vec::new();
    for &c in &g.players[p].cities.clone() {
        let cp = g.city_plot(c);
        if let Some(&u) = units.iter().filter(|&&u| g.unit_plot(u) == cp).max_by_key(|&&u| (g.units[u].base_combat_defense, u)) {
            garrison.push(u);
        }
    }
    let field: Vec<UnitId> = units.iter().copied().filter(|u| !garrison.contains(u)).collect();
    let lead = field.iter().copied().max_by_key(|&u| (g.crusade_unit_strength(u), std::cmp::Reverse(u))).unwrap_or(units[0]);
    let here = g.unit_plot(lead);
    let goal = objective(g, idx, here);
    let avg_health = if field.is_empty() { 100 } else { field.iter().map(|&u| health_percent(g, u)).sum::<i32>() / field.len() as i32 };

    // The rally point: the friendly city nearest the objective.
    let friends = friendly_cities(g, idx);
    let mut ai: HostAi = g.crusades[idx].ai.clone();
    if ai.rally.is_none() {
        ai.rally = goal.and_then(|q| nearest_city_to(g, &friends, q, g.map.plots[q].area)).map(|c| g.city_plot(c)).or(Some(here));
    }
    let rally = ai.rally.expect("rally set");
    let pledged: i64 = g.crusades[idx].calls.iter().map(|x| match x.answer {
        Some(Pledge::Strength(s)) => s as i64,
        _ => 0,
    }).sum::<i64>().max(g.crusades[idx].lenders.iter().map(|l| l.delivered as i64).sum());
    let (rx, ry) = g.map.xy(rally);
    let mustered: i64 = field.iter().filter(|&&u| {
        let (x, y) = (g.units[u].x, g.units[u].y);
        g.plot_distance(x, y, rx, ry) <= 2
    }).map(|&u| g.crusade_unit_strength(u) as i64).sum();
    let near_goal = goal.is_some_and(|q| {
        let (qx, qy) = g.map.xy(q);
        field.iter().any(|&u| g.plot_distance(g.units[u].x, g.units[u].y, qx, qy) <= 2)
    });
    let old = ai.state;
    ai.state = if !first_call { ai.state } else { match ai.state {
        HostState::Muster => {
            if mustered * 100 >= rules.muster_percent as i64 * pledged.max(1) || turn >= g.crusades[idx].deadline_turn {
                HostState::March
            } else {
                HostState::Muster
            }
        }
        HostState::March | HostState::Engage if avg_health < rules.recover_health_percent => HostState::Recover,
        HostState::March => if near_goal { HostState::Engage } else { HostState::March },
        HostState::Engage => if goal.is_none() || !near_goal { HostState::March } else { HostState::Engage },
        HostState::Recover => {
            if avg_health >= 90 || turn - ai.since >= rules.recover_turns {
                HostState::March
            } else {
                HostState::Recover
            }
        }
    } };
    if ai.state != old {
        ai.since = turn;
    }
    ai.goal = goal;
    crate::trace!(g, p, "host of crusade {}: {:?} -> {:?}, goal {:?}, mustered {} of {}, health {}", id.0, old, ai.state, goal.map(|q| g.map.xy(q)), mustered, pledged, avg_health);
    if let Some(i) = g.crusade_index(id) {
        g.crusades[i].ai = ai.clone();
    }

    for &u in &garrison {
        if g.units.contains(u) {
            if let Some(grp) = g.units[u].group {
                if g.groups[grp].units.len() > 1 {
                    g.unit_join_group(u, None, true);
                }
            }
            if let Some(grp) = g.units[u].group {
                if g.group_ready_to_move(grp, false) {
                    hold(g, grp);
                }
            }
        }
    }

    let groups: Vec<GroupId> = g.players[p].groups.clone();
    if first_call {
        // Field stacks decide afresh every turn (a fortified or healing stack is not "ready").
        for &grp in &groups {
            let head = g.group_get_head_unit(grp);
            if alive(g, grp) && head.is_some_and(|h| field.contains(&h)) {
                g.group_clear_mission_queue(grp);
                g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
            }
        }
    }
    // Engage: every stack next to the objective strikes together, as a civ's siege operation
    // does (`campaign::strike`: the expected outcome with the whole force next to the city, the
    // attackers in order, the victors moving in).
    if let (HostState::Engage | HostState::March, Some(goal)) = (ai.state, goal) {
        let (gx, gy) = g.map.xy(goal);
        let next: Vec<GroupId> = groups
            .iter()
            .copied()
            .filter(|&grp| {
                alive(g, grp)
                    && g.group_ready_to_move(grp, false)
                    && g.group_get_head_unit(grp).is_some_and(|h| field.contains(&h) && {
                        let (x, y) = g.map.xy(g.unit_plot(h));
                        g.plot_distance(x, y, gx, gy) == 1
                    })
            })
            .collect();
        if let (false, Some(city)) = (next.is_empty(), g.map.plots[goal].plot_city) {
            let op = crate::campaign::Op {
                city,
                plot: goal,
                civ: true,
                phase: crate::campaign::Phase::Strike,
                groups: next.clone(),
                rally: Some(rally),
                assembly: None,
                staging: None,
                struck: false,
                at_war: true,
                need: 0,
                have: next.iter().map(|&grp| g.groups[grp].units.len() as i32).sum(),
                probe: false,
            };
            let mut v2 = View::new(g, p);
            crate::campaign::strike(g, &mut v2, &op);
        }
    }
    for grp in groups {
        if !alive(g, grp) || !g.group_ready_to_move(grp, false) {
            continue;
        }
        let Some(head) = g.group_get_head_unit(grp) else { continue };
        if garrison.contains(&head) || !field.contains(&head) {
            if g.group_ready_to_move(grp, false) {
                hold(g, grp);
            }
            continue;
        }
        act(g, &v, idx, grp, head, ai.state, goal, rally, &friends);
        if alive(g, grp) && g.group_ready_to_move(grp, true) {
            skip(g, grp);
        }
    }
    crate::units::finalize(g, p);
}

/// One stack's move in state `state`.
#[allow(clippy::too_many_arguments)]
fn act(g: &mut Game, v: &View, idx: usize, grp: GroupId, head: UnitId, state: HostState, goal: Option<PlotIdx>, rally: PlotIdx, friends: &[CityId]) {
    let here = g.unit_plot(head);
    // Enemy units next to the stack: strike when the odds are worth it (never while healing).
    if state != HostState::Recover && state != HostState::Muster || v.danger_at(here) > 0 {
        if let Some(q) = best_adjacent(g, idx, grp, here, goal) {
            if host_assault(g, v, grp, q) {
                return;
            }
        }
    }
    if !alive(g, grp) {
        return;
    }
    match state {
        HostState::Muster => {
            if here == rally {
                hold(g, grp);
            } else {
                move_to(g, grp, rally, 0);
            }
        }
        HostState::Recover => {
            let area = g.map.plots[here].area;
            match nearest_city_to(g, friends, here, area) {
                Some(c) if g.city_plot(c) != here => move_to(g, grp, g.city_plot(c), 0),
                _ => {
                    if g.group_can_start_mission(grp, MissionType::MISSION_HEAL, -1, -1, None, false) {
                        push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
                    } else {
                        hold(g, grp);
                    }
                }
            }
        }
        HostState::March | HostState::Engage => {
            let Some(goal) = goal else {
                // Nothing to take on this landmass: guard the rally point.
                if here == rally { hold(g, grp) } else { move_to(g, grp, rally, 0) }
                return;
            };
            let ((hx, hy), (qx, qy)) = (g.map.xy(here), g.map.xy(goal));
            // An undefended objective: walk in (the city is captured).
            if !g.plot_is_visible_enemy_unit_for_unit(goal, head) && g.group_can_move_into(grp, goal, false) {
                move_to(g, grp, goal, 0);
                return;
            }
            if g.plot_distance(hx, hy, qx, qy) == 1 {
                if host_assault(g, v, grp, goal) {
                    return;
                }
                // Besiege: bombard if we can; the other stacks join this one; else pillage
                // around the city, else hold next to it.
                if g.group_can_start_mission(grp, MissionType::MISSION_BOMBARD, -1, -1, None, false) {
                    push(g, grp, MissionType::MISSION_BOMBARD, -1, -1, 0, false);
                } else if !pillage_near(g, idx, grp, head) {
                    hold(g, grp);
                }
                return;
            }
            // Stacks converge on the largest stack next to the goal.
            if let Some(q) = siege_camp(g, idx, goal, here) {
                if q != here {
                    move_to(g, grp, q, 0);
                    return;
                }
            }
            match staging(g, head, goal, here) {
                Some(q) if q != here => move_to(g, grp, q, 0),
                _ => hold(g, grp),
            }
        }
    }
}

/// The Host exists to fight: it takes a fight a field army would wait for (the best attacker
/// wins more often than not, or the stack wears the defenders down).
fn host_attack_ok(a: &crate::military::Assessment) -> bool {
    a.n_def > 0 && (a.best >= 550 || (a.best >= 250 && a.effective >= a.n_def * 900))
}

/// Host assault on `q` while [`host_attack_ok`] holds (the military assault's loop with the
/// Host's odds); true if it fought.
fn host_assault(g: &mut Game, v: &View, grp: GroupId, q: PlotIdx) -> bool {
    let mut fought = false;
    for _ in 0..12 {
        if !alive(g, grp) {
            break;
        }
        let Some(head) = g.group_get_head_unit(grp) else { break };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) {
            break;
        }
        let Some(a) = crate::military::assess(g, grp, q) else { break };
        if !host_attack_ok(&a) {
            break;
        }
        let before = g.groups[grp].units.len();
        g.groups[grp].ai_group_attack = false;
        move_to(g, grp, q, MOVE_DIRECT_ATTACK);
        fought = true;
        if !alive(g, grp) || g.groups[grp].units.len() == before && crate::military::assess(g, grp, q).is_some_and(|b| b.n_att == a.n_att) {
            break;
        }
    }
    // Defenders gone: move in (a city is captured).
    if fought && alive(g, grp) {
        if let Some(head) = g.group_get_head_unit(grp) {
            if g.unit_plot(head) != q && !g.plot_is_visible_enemy_unit_for_unit(q, head) && g.unit_moves_left(head) > 0 && g.group_can_move_into(grp, q, false) {
                move_to(g, grp, q, 0);
            }
        }
    }
    let _ = v;
    fought
}

/// Pillage an enemy improvement here or next to the stack (sacked gold joins the spoils); true
/// if it did or moved to do it.
fn pillage_near(g: &mut Game, idx: usize, grp: GroupId, head: UnitId) -> bool {
    let here = g.unit_plot(head);
    if g.map.plots[here].owner.is_some_and(|o| g.crusade_is_enemy_team(idx, g.players[o].team)) && g.group_can_start_mission(grp, MissionType::MISSION_PILLAGE, -1, -1, None, false) {
        push(g, grp, MissionType::MISSION_PILLAGE, -1, -1, 0, false);
        return true;
    }
    let (x, y) = g.map.xy(here);
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
        let pl = &g.map.plots[q];
        if pl.improvement_type.is_none() || pl.plot_city.is_some() || !pl.owner.is_some_and(|o| g.crusade_is_enemy_team(idx, g.players[o].team)) {
            continue;
        }
        if g.plot_is_visible_enemy_unit_for_unit(q, head) || !g.group_can_move_into(grp, q, false) {
            continue;
        }
        move_to(g, grp, q, 0);
        return true;
    }
    false
}

/// Best adjacent plot holding enemy units of the crusade worth attacking now (the objective
/// city included).
fn best_adjacent(g: &Game, idx: usize, grp: GroupId, here: PlotIdx, goal: Option<PlotIdx>) -> Option<PlotIdx> {
    let head = g.group_get_head_unit(grp)?;
    let (x, y) = g.map.xy(here);
    let mut best: Option<(i32, PlotIdx)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) {
            continue;
        }
        let ours = g.map.plots[q].units.iter().any(|&o| g.units.contains(o) && g.crusade_is_enemy_team(idx, g.unit_get_team(o)));
        if !ours && Some(q) != goal {
            continue;
        }
        let Some(a) = crate::military::assess(g, grp, q) else { continue };
        if !host_attack_ok(&a) {
            continue;
        }
        let score = a.best + if Some(q) == goal { 500 } else { 0 };
        if best.is_none_or(|(s, bq)| (score, std::cmp::Reverse(q)) > (s, std::cmp::Reverse(bq))) {
            best = Some((score, q));
        }
    }
    best.map(|(_, q)| q)
}
