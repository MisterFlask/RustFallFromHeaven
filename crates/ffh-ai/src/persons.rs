//! Persons of Note (docs/PERSONS.md; docs/AI.md, "Persons of Note"): the AI of the second build
//! (Houses as vassal civilizations, rule 12). A rules change is not gated on the AI (CLAUDE.md):
//! reasonable first choices, not measured ones. AI players only: a human's grants, tribute and
//! offers are the human's.
//!
//! Once a turn ([`turn`], after the empire's civics and religion and the turn's diplomacy):
//! - **Tribute** ([`tribute_for`]): each House's rate set to the highest whose loyalty once in
//!   force is Pleased with a point to spare (2 or more, [`TRIBUTE_KEEP_LOYALTY`]); lowered as soon
//!   as the projection at the rate set falls below that. The rate in force lags the rate set (rule
//!   4: a step every 5 turns), so a cut no longer helps at once: the point to spare is what lets
//!   the House take a grievance while a cut arrives. So the AI's Houses are not for sale (a House
//!   Cautious or worse takes a price) and their countdowns are stopped where a cut arrives in
//!   time. Set only when it changes.
//! - **Grants** ([`choose_grant`]): one a turn while the Crown is over its limit (rule 14: 3 cities,
//!   the capital alone for the Kuriotates; settlements never count), the least populous first,
//!   never the capital nor the best production city ([`kept_city`]). The candidate by, in
//!   order: no holder made Furious; a standing demand met (an Annoyed or Furious grantee's first);
//!   the least populous city; a claimant, the one in the best state; a person without a post
//!   founding a House, then a great person founding one (only when no such person is at hand and
//!   no House has room, and not one the card logic keeps waiting: [`great_person_spare`]), then a
//!   House enlarged (only to its realm's size threshold:
//!   [`house_has_room`]), the one with the fewest cities; the fewest holders made Annoyed. A
//!   commander founds a House (and loses the command) only for their claim or demand; a hero never
//!   founds one (the rules refuse it). A grant that would make some holder Furious waits.
//! - **Offers** ([`choose_offer`]): at war with a civilization, an offer to one of its Houses that
//!   is Cautious or worse, at its price ([`Game::house_bribe_price`]), in gold: Become your vassal
//!   (the cities, their units and a House, the war already on) when the price is at most half the
//!   treasury, else Secede when it is at most a third; the cheapest House first, one offer a turn,
//!   the same House again only after [`OFFER_PERIOD`] turns. Through `player_offer_house`, the
//!   deal path the human's offers take.
//!
//! Elsewhere: the levy ([`levy_city`]: `armies::requisition_cities` adds the cities of a House
//! that stays Pleased or better after the levy's `Levied -1`); a direct-rule civic is never
//! adopted while a House stands ([`civic_barred`], `empire.rs`); an idle army goes to hold the
//! most valuable city it can hold of a House counting down that no tribute cut can stop in time
//! ([`house_beyond_tribute`]; [`disloyal_house_post`], `military.rs`: a city where the liege's
//! army units outnumber the House's goes back to the liege when it breaks away); a Furious commander's army goes to the
//! city of ours with the largest garrison ([`disloyal_commander_haven`]); a demanded war weighs on the war planning ([`war_demanded`]); a
//! House's own turn skips what a vassal cannot do ([`is_house`], `lib.rs`). A House answers offers
//! by the rules' price (`diplomacy::judge_offer`). The AI never frees a House.

use std::cmp::Reverse;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{BribeKind, DemandKind, GrantCandidate, GrantTo, LoyaltyCause, LoyaltyState, Post, FURIOUS_AT, LEVIED, OFFER_NOTED_TURNS, PLEASED_AT, TRIBUTE_MAX, TRIBUTE_STEP};
use ffh_core::structs::TradeData;
use ffh_data::*;

use crate::view::View;

/// The turns before the AI offers the same House again.
pub const OFFER_PERIOD: i32 = 10;

/// The loyalty the AI liege keeps a House at once its tribute is in force: Pleased with a point
/// to spare (the rate in force lags a cut, rule 4).
pub const TRIBUTE_KEEP_LOYALTY: i32 = PLEASED_AT + 1;

/// The persons' turn of AI player `p`: its Houses' tribute, one grant, one offer to an enemy's
/// House.
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.players[p].human || !g.player_has_persons(p) {
        return;
    }
    tribute(g, p);
    if let Some((c, to)) = choose_grant(g, p) {
        let city = g.d.text.name(&g.cities[c].name).to_string();
        match g.player_grant_city(p, c, to) {
            Ok(h) => crate::trace!(g, p, "persons: {city} granted to {}", g.house_label(h)),
            Err(e) => crate::trace!(g, p, "persons: no grant of {city}: {e}"),
        }
    }
    if let Some(offer) = choose_offer(g, p) {
        let label = g.player_house(offer.house_player).map(|h| g.house_label(h)).unwrap_or_default();
        let get = [TradeData { item_type: offer.kind.item(), data: 0, offering: false, hidden: false }];
        let accepted = g.player_offer_house(p, offer.house_player, &offer.give, &get);
        crate::trace!(g, p, "persons: {} offered to {label} for {} ({})", offer.kind.name(), ffh_core::persons::house_offer_value(&offer.give), if accepted { "accepted" } else { "refused" });
    }
}

/// Whether `p` is a House: a vassal civilization, which plays only its own land (its liege makes
/// its war and peace).
pub fn is_house(g: &Game, p: PlayerId) -> bool {
    g.player_house(p).is_some()
}

/// The best state first (Friendly 0 .. Furious 4).
fn state_rank(s: LoyaltyState) -> i32 {
    match s {
        LoyaltyState::Friendly => 0,
        LoyaltyState::Pleased => 1,
        LoyaltyState::Cautious => 2,
        LoyaltyState::Annoyed => 3,
        LoyaltyState::Furious => 4,
    }
}

/// `p`'s living Houses, in id order.
fn houses_of(g: &Game, p: PlayerId) -> Vec<HouseId> {
    let mut out: Vec<HouseId> = g.players[p].houses.iter().copied().filter(|&h| g.houses.contains(h)).collect();
    out.sort();
    out
}

// -------------------------------------------------------------------------------------------
// Tribute
// -------------------------------------------------------------------------------------------

/// The tribute (percent) the AI liege sets for House `h`: the highest rate whose loyalty once in
/// force (`Game::house_loyalty_at_tribute`, the steady state) is [`TRIBUTE_KEEP_LOYALTY`] or more,
/// else 0. The rate in force follows it a step every 5 turns (rule 4): a higher one is collected
/// step by step, and a lower one is set as soon as the projection at the rate set falls below
/// the margin, so the cut starts moving at once.
pub fn tribute_for(g: &Game, h: HouseId) -> i32 {
    (0..=TRIBUTE_MAX / TRIBUTE_STEP).rev().map(|k| k * TRIBUTE_STEP).find(|&pct| g.house_loyalty_at_tribute(h, pct) >= TRIBUTE_KEEP_LOYALTY).unwrap_or(0)
}

/// `p` sets each House's tribute to [`tribute_for`], where it changes.
fn tribute(g: &mut Game, p: PlayerId) {
    for h in houses_of(g, p) {
        let pct = tribute_for(g, h);
        if g.houses[h].tribute != pct && g.player_set_house_tribute(p, h, pct).is_ok() {
            crate::trace!(g, p, "persons: {} set to {pct}%, {}% in force ({})", g.house_label(h), g.houses[h].tribute_now, g.house_state(h).name());
        }
    }
}

// -------------------------------------------------------------------------------------------
// The levy
// -------------------------------------------------------------------------------------------

/// Whether `p`'s armies levy from city `c` of one of its Houses: the House stays Pleased or better
/// after the levy's grievance (`Levied -1`; a House Cautious or worse takes a price to secede, and
/// the rules take no levy from one Annoyed or worse).
pub fn levy_city(g: &Game, p: PlayerId, c: CityId) -> bool {
    g.city_levied_by(c, p) && g.city_house(c).is_some_and(|h| g.house_loyalty(h) + LEVIED.0 >= PLEASED_AT)
}

/// The cities of `p`'s Houses its armies levy from now ([`levy_city`]), in the rules' order.
pub fn levy_cities(g: &Game, p: PlayerId) -> Vec<CityId> {
    g.player_levy_cities(p).into_iter().filter(|&c| levy_city(g, p, c)).collect()
}

// -------------------------------------------------------------------------------------------
// Grants
// -------------------------------------------------------------------------------------------

/// The city `p` keeps in the Crown besides its capital: its best production city (ties: the
/// older). The AI's empire orders (wonders, settlers, its army) go to its capital and most
/// productive cities (`governor::empire_cities`).
pub(crate) fn kept_city(g: &Game, p: PlayerId) -> Option<CityId> {
    let capital = g.players[p].capital_city;
    g.players[p].cities.iter().copied().filter(|&c| Some(c) != capital && g.cities.contains(c)).max_by_key(|&c| (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION), Reverse(c)))
}

/// The Crown cities `p` may grant this turn, the least populous first (ties: the older): none
/// while the Crown is within its limit (`player_crown_over`); else every Crown city but the
/// capital and [`kept_city`] (one grant a turn, so the Crown comes down to its limit and stops).
pub fn grantable_cities(g: &Game, p: PlayerId) -> Vec<CityId> {
    if g.player_crown_over(p).is_empty() {
        return Vec::new();
    }
    let kept = kept_city(g, p);
    let mut out: Vec<CityId> = g.player_crown_cities(p).into_iter().filter(|&c| Some(c) != kept && !g.city_is_capital(c)).collect();
    out.sort_by_key(|&c| (g.cities[c].population, c));
    out
}

/// Whether House `h` may take one more Crown city: it stays within its realm's size threshold
/// (rule 14: the threshold grows with the realm, so a large realm's Houses are larger).
pub fn house_has_room(g: &Game, h: HouseId) -> bool {
    g.house_great_cities(h) + 1 <= g.house_size_threshold(h)
}

/// The state of the grantee of `to` when the grant of city `c` meets their standing demand.
fn meets_demand(g: &Game, c: CityId, to: GrantTo) -> Option<LoyaltyState> {
    match to {
        GrantTo::House(h) => {
            let x = g.houses[h].head?;
            let dm = g.persons.get(x)?.demand?;
            (dm.kind == DemandKind::GrantCity(g.city_plot(c))).then(|| g.house_state(h))
        }
        GrantTo::Person(x) => {
            let dm = g.persons[x].demand?;
            matches!(dm.kind, DemandKind::Post | DemandKind::House).then(|| g.person_state(x))
        }
        GrantTo::GreatPerson(_) => None,
    }
}

/// Whether great person `u` may found a House: its card (docs/DECKS.md) is none, or its needs are
/// too far for the card logic to keep it waiting (`cards::great_person_card`).
fn great_person_spare(g: &Game, p: PlayerId, u: UnitId) -> bool {
    match g.units[u].card {
        None => true,
        Some(card) => crate::cards::missing_needs(g, p, card).len() > crate::cards::WAIT_MISSING,
    }
}

/// What [`choose_grant`] weighs beyond the candidate: the Crown's cities, whether a person
/// without a post can found a House, whether some House has room.
struct GrantContext {
    founder: bool,
    room: bool,
}

/// The ranking key of a grant candidate (larger is better), or `None` when the AI does not
/// consider it.
fn grant_key(g: &Game, p: PlayerId, c: CityId, k: &GrantCandidate, cx: &GrantContext) -> Option<(bool, i32, Reverse<i32>, bool, i32, i32, i32, i32)> {
    if k.refusal.is_some() {
        return None;
    }
    let met = meets_demand(g, c, k.to);
    let kind = match k.to {
        GrantTo::House(h) if !house_has_room(g, h) => return None,
        GrantTo::House(_) => 1,
        GrantTo::Person(x) => match g.persons[x].post {
            Post::None => 3,
            // A commander founding a House loses the command: only to meet their demand or
            // their claim.
            _ if met.is_some() || k.claimant => 0,
            _ => return None,
        },
        GrantTo::GreatPerson(u) => {
            if cx.founder || cx.room || !great_person_spare(g, p, u) {
                return None;
            }
            2
        }
    };
    let furious = k.lines.iter().any(|l| l.state_after == Some(LoyaltyState::Furious));
    let annoyed = k.lines.iter().filter(|l| l.state_after == Some(LoyaltyState::Annoyed)).count() as i32;
    let demand = met.map_or(0, |s| 1 + (state_rank(s) - 2).max(0));
    let grantee_state = match k.to {
        GrantTo::House(h) => g.house_state(h),
        GrantTo::Person(x) => g.person_state(x),
        GrantTo::GreatPerson(_) => LoyaltyState::Cautious,
    };
    let claimant_rank = if k.claimant { 4 - state_rank(grantee_state) } else { 0 };
    let fewest = match k.to {
        GrantTo::House(h) => -(g.house_cities(h).len() as i32),
        _ => 0,
    };
    Some((!furious, demand, Reverse(g.cities[c].population), k.claimant, claimant_rank, kind, fewest, -annoyed))
}

/// The grant `p` makes this turn, if any: see the module comment.
pub fn choose_grant(g: &Game, p: PlayerId) -> Option<(CityId, GrantTo)> {
    if g.player_check_grants(p).is_err() {
        return None;
    }
    let pl = &g.players[p];
    let has_great = pl.units.iter().any(|&u| g.units.contains(u) && g.unit_is_great_person(u));
    if pl.persons.is_empty() && pl.houses.is_empty() && !has_great {
        return None;
    }
    let cities = grantable_cities(g, p);
    if cities.is_empty() {
        return None;
    }
    let with_leader = |post: fn(&Post) -> bool| pl.persons.iter().any(|&x| g.persons.get(x).is_some_and(|person| post(&person.post) && person.leader.is_some()));
    let founder = with_leader(|post| *post == Post::None);
    let room = houses_of(g, p).into_iter().any(|h| house_has_room(g, h));
    // Nobody could take a city (the previews below are the cost of this turn).
    if !founder && !room && !has_great && !with_leader(|post| matches!(post, Post::Commander(_))) {
        return None;
    }
    let cx = GrantContext { founder, room };
    let mut best: Option<((bool, i32, Reverse<i32>, bool, i32, i32, i32, i32), CityId, GrantTo)> = None;
    for c in cities {
        for k in g.grant_candidates(p, c) {
            let Some(key) = grant_key(g, p, c, &k, &cx) else { continue };
            if best.as_ref().map_or(true, |(b, ..)| key > *b) {
                best = Some((key, c, k.to));
            }
        }
    }
    // A grant that makes a holder Furious waits: the Crown keeping the city snubs nobody.
    best.filter(|(key, ..)| key.0).map(|(_, c, to)| (c, to))
}

/// Whether a person of `p` demands war on `other`'s team (a small bonus in the war planning:
/// `diplomacy::manage`).
pub(crate) fn war_demanded(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let team = g.players[other].team;
    g.players[p].persons.iter().any(|&x| matches!(g.persons.get(x).and_then(|person| person.demand).map(|dm| dm.kind), Some(DemandKind::War(q)) if g.players[q].team == team))
}

// -------------------------------------------------------------------------------------------
// Offers to an enemy's Houses
// -------------------------------------------------------------------------------------------

/// Whether a civilization named `by` offered House `h` a deal in the last [`OFFER_PERIOD`] turns
/// (the House's loyalty list notes every offer for `OFFER_NOTED_TURNS`).
fn offered_lately(g: &Game, h: HouseId, by: &str) -> bool {
    let turn = g.game_turn();
    g.houses[h].standing.mods.iter().any(|m| matches!(&m.cause, LoyaltyCause::Offered { by: b, .. } if b == by) && m.until.is_some_and(|u| u - OFFER_NOTED_TURNS > turn - OFFER_PERIOD))
}

/// An offer the AI makes to an enemy's House ([`choose_offer`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The House's player.
    pub house_player: PlayerId,
    /// What it asks: Become your vassal or Secede.
    pub kind: BribeKind,
    /// What `p` gives: gold, the House's price.
    pub give: Vec<TradeData>,
}

/// What `p` offers House `h` (of a civilization it is at war with) at its price, if it can pay it
/// in gold (a House takes gold only): Become your vassal when the price is at most half its
/// treasury (the gold it keeps left out) and the House would give it, else Secede when it is at
/// most a third and the House would give it.
fn offer_for(g: &Game, p: PlayerId, h: HouseId) -> Option<(BribeKind, Vec<TradeData>)> {
    let price = g.house_bribe_price(h)?;
    let hp = g.houses[h].player;
    let item = |item_type: TradeableItem, data: i32| TradeData { item_type, data, offering: false, hidden: false };
    let treasury = g.players[p].gold - crate::civs::gold_floor(g, p);
    let gold = item(TradeableItem::TRADE_GOLD, price);
    let give = if price <= 0 { Vec::new() } else { vec![gold.clone()] };
    if price > 0 && !g.player_can_trade_item(p, hp, &gold, false) {
        return None;
    }
    let gives = |kind: BribeKind| g.house_bribe_refusal(h, p, kind).is_none() && g.player_can_trade_item(hp, p, &item(kind.item(), 0), false);
    if price <= treasury / 2 && gives(BribeKind::Vassal) {
        return Some((BribeKind::Vassal, give));
    }
    (price <= treasury / 3 && gives(BribeKind::Secede)).then_some((BribeKind::Secede, give))
}

/// The offer `p` makes this turn, if any. See the module comment.
pub fn choose_offer(g: &Game, p: PlayerId) -> Option<Offer> {
    if g.players[p].human || !g.player_has_persons(p) {
        return None;
    }
    let team = g.players[p].team;
    let by = g.player_get_civilization_short_description(p);
    let mut best: Option<((i32, HouseId), Offer)> = None;
    for q in g.player_ids() {
        let pl = &g.players[q];
        if !pl.alive || pl.houses.is_empty() || !g.at_war(team, pl.team) {
            continue;
        }
        for h in houses_of(g, q) {
            if offered_lately(g, h, &by) {
                continue;
            }
            let Some(price) = g.house_bribe_price(h) else { continue };
            let Some((kind, give)) = offer_for(g, p, h) else { continue };
            let key = (price, h);
            if best.as_ref().map_or(true, |(k, _)| key < *k) {
                best = Some((key, Offer { house_player: g.houses[h].player, kind, give }));
            }
        }
    }
    best.map(|(_, offer)| offer)
}

// -------------------------------------------------------------------------------------------
// Civics
// -------------------------------------------------------------------------------------------

/// Whether civic `c` is barred to `p`: a direct-rule civic while a House stands and `p` does not
/// rule directly already (adopting one is the civil war: every House breaks away, rule 11; a
/// vassal taken under direct rule is a House that stays).
pub(crate) fn civic_barred(g: &Game, p: PlayerId, c: CivicType) -> bool {
    g.d.civics[c].direct_rule && !g.player_rules_directly(p) && g.players[p].houses.iter().any(|&h| g.houses.contains(h))
}

// -------------------------------------------------------------------------------------------
// Armies
// -------------------------------------------------------------------------------------------

/// Whether House `h` breaks away whatever its tribute does: Furious, and still Furious when its
/// countdown ends (its full length when it has not started) at the rate in force then were its
/// rate set to 0% now (rule 4: the rate in force lags a cut, a step every 5 turns).
pub fn house_beyond_tribute(g: &Game, h: HouseId) -> bool {
    if g.house_state(h) != LoyaltyState::Furious {
        return false;
    }
    let turns = g.houses[h].standing.countdown.map_or(g.d.defines.person_disloyal_turns, |end| (end - g.game_turn()).max(0));
    let rate = g.house_tribute_in_force_after(h, 0, turns);
    g.house_loyalty_at_tribute(h, rate) <= FURIOUS_AT
}

/// Where an idle army stack (`grp`, led by `u`) of `v.p` goes against a break-away that the
/// tribute cannot stop in time ([`house_beyond_tribute`]): such a House breaks away at its
/// countdown's end, and takes every city where our army units do not outnumber its own land
/// combat units (`Game::city_hold`, rule 7). The stack holds the House's most valuable city it can
/// hold (the most populous first, ties: the older): one not held now where its army units, with
/// ours already there, would outnumber the House's, within 8 turns; or it stays in the city it
/// stands in when that city is held only with it (its plot).
pub(crate) fn disloyal_house_post(g: &Game, v: &mut View, grp: GroupId, u: UnitId) -> Option<PlotIdx> {
    let p = v.p;
    let mut cities: Vec<CityId> = Vec::new();
    for &h in &g.players[p].houses {
        if g.houses.contains(h) && house_beyond_tribute(g, h) {
            cities.extend(g.house_cities(h));
        }
    }
    if cities.is_empty() {
        return None;
    }
    // The stack's units that count in the contest (an army's land units, its commander not
    // Furious).
    let mine = g.groups[grp].units.iter().filter(|&&o| g.unit_holds_for_liege(o, p)).count() as i32;
    if mine == 0 {
        return None;
    }
    cities.sort_by_key(|&c| (std::cmp::Reverse(g.cities[c].population), c));
    let here = g.unit_plot(u);
    if let Some(hold) = cities.iter().find(|&&c| g.city_plot(c) == here).and_then(|&c| g.city_hold(c)) {
        if hold.held() && hold.liege - mine <= hold.house {
            return Some(here);
        }
    }
    let open: Vec<PlotIdx> = cities
        .iter()
        .filter(|&&c| g.city_plot(c) != here && g.city_hold(c).is_some_and(|hold| !hold.held() && hold.liege + mine > hold.house))
        .map(|&c| g.city_plot(c))
        .collect();
    if open.is_empty() {
        return None;
    }
    let reach = v.flood.run(g, u, 8, false);
    open.into_iter().find(|&q| reach.iter().any(|r| r.plot == q))
}

/// Where the army of a Furious commander (`grp`, led by `u`) goes: the city of ours with the
/// largest garrison of our other land army units (its sworn units standing there with more of
/// those stay at the secession), within 10 turns (ties: the nearer, then the older city).
pub(crate) fn disloyal_commander_haven(g: &Game, v: &mut View, grp: GroupId, u: UnitId) -> Option<PlotIdx> {
    let x = g.unit_army_commander(u)?;
    if g.person_state(x) != LoyaltyState::Furious {
        return None;
    }
    let p = v.p;
    let reach = v.flood.run(g, u, 10, false);
    let mut best: Option<((usize, std::cmp::Reverse<i32>, std::cmp::Reverse<CityId>), PlotIdx)> = None;
    for &c in &g.players[p].cities {
        if !g.cities.contains(c) {
            continue;
        }
        let q = g.city_plot(c);
        let Some(r) = reach.iter().find(|r| r.plot == q) else { continue };
        let garrison = g.map.plots[q].units.iter().filter(|&&o| g.units[o].owner == p && g.units[o].group != Some(grp) && g.unit_is_army_unit(o)).count();
        let key = (garrison, std::cmp::Reverse(r.turns), std::cmp::Reverse(c));
        if best.as_ref().map_or(true, |(b, _)| key > *b) {
            best = Some((key, q));
        }
    }
    best.map(|(_, q)| q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::persons::{Ambition, Demand, LoyaltyMod};

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    /// Ten city sites of the one-landmass map, three plots apart.
    const SPOTS: [(i32, i32); 10] = [(3, 4), (6, 4), (9, 4), (12, 4), (15, 4), (3, 7), (6, 7), (9, 7), (12, 7), (15, 7)];

    /// The one-landmass duel with player 0 under Aristocracy (cities may be granted; the Crown's
    /// limit is 3) holding cities at `spots`, the first its capital; the two players have met.
    fn world(spots: &[(i32, i32)]) -> (Game, Vec<CityId>) {
        // (The Bannor: a civilization with minor leaders, so veterans enter the cast.)
        let mut g = crate::testkit::one_landmass_game_with(7, &["CIVILIZATION_BANNOR"]);
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_meet(t0, t1, false);
        let cities: Vec<CityId> = spots.iter().map(|&(x, y)| g.player_init_city(P0, x, y, true, true)).collect();
        let civic = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
        let option = g.d.civics[civic].civic_option_type.expect("a civic option");
        g.player_set_civics(P0, option, Some(civic));
        assert_eq!(g.player_crown_limit(P0), Some(3));
        (g, cities)
    }

    /// A veteran of player 0 entered into the cast (the next game turn's persons turn), of Land
    /// ambition, with no demand.
    fn person(g: &mut Game, x: i32, y: i32) -> PersonId {
        let t = g.d.t::<UnitType>("UNIT_WARRIOR");
        let u = g.player_init_unit(P0, t, x, y, None, None);
        g.units[u].level = 30;
        enter(g, u)
    }

    /// Unit `u` of player 0 entered into the cast (the next game turn's persons turn), without a
    /// demand.
    fn enter(g: &mut Game, u: UnitId) -> PersonId {
        let turn = g.game_turn();
        g.set_game_turn(turn + 1);
        g.player_persons_turn(P0);
        let x = g.unit_person(u).expect("entered");
        g.persons[x].demand = None;
        g.persons[x].ambition = Ambition::Land;
        x
    }

    /// A hero of player 0 entered into the cast.
    fn hero(g: &mut Game, x: i32, y: i32) -> PersonId {
        let t = g.d.t::<UnitType>("UNIT_WARRIOR");
        let u = g.player_init_unit(P0, t, x, y, None, None);
        let promotion = g.d.t::<PromotionType>("PROMOTION_HERO");
        g.unit_set_has_promotion(u, promotion, true);
        enter(g, u)
    }

    /// A House founded with city `seat` by a new person, its head without a demand.
    fn house(g: &mut Game, seat: CityId) -> (PersonId, HouseId) {
        let (x, y) = (g.cities[seat].x, g.cities[seat].y);
        let p = person(g, x, y);
        let h = g.player_grant_city(P0, seat, GrantTo::Person(p)).expect("granted");
        g.persons[p].demand = None;
        (p, h)
    }

    /// House `h`'s loyalty set to `total` (one modifier) at its tribute now.
    fn set_loyalty(g: &mut Game, h: HouseId, total: i32) {
        g.houses[h].standing.mods.clear();
        let base = g.house_loyalty(h);
        let t = g.game_turn();
        g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: total - base, until: Some(t + 60) });
        assert_eq!(g.house_loyalty(h), total);
    }

    fn demand(g: &mut Game, x: PersonId, kind: DemandKind) {
        let t = g.game_turn();
        g.persons[x].demand = Some(Demand { kind, issued: t, deadline: t + 40 });
    }

    fn pops(g: &mut Game, cities: &[CityId], pops: &[i32]) {
        for (&c, &n) in cities.iter().zip(pops) {
            g.city_set_population(c, n);
        }
    }

    fn gold(data: i32) -> TradeData {
        TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false }
    }

    fn secede() -> TradeData {
        TradeData { item_type: TradeableItem::TRADE_SECEDE, data: 0, offering: false, hidden: false }
    }

    /// The city on the plot city `c` stood on (a city changing hands has a new id).
    fn now(g: &Game, plot: PlotIdx) -> CityId {
        g.map.plots[plot].plot_city.expect("a city")
    }

    // ---- Tribute ----

    /// House `h`'s rate set and rate in force.
    fn rates(g: &Game, h: HouseId) -> (i32, i32) {
        (g.houses[h].tribute, g.houses[h].tribute_now)
    }

    /// House `h`'s rate set and in force, at rest at `pct`.
    fn set_rate(g: &mut Game, h: HouseId, pct: i32) {
        g.houses[h].tribute = pct;
        g.houses[h].tribute_now = pct;
    }

    /// Person `x` makes no demand for the test's turns.
    fn quiet(g: &mut Game, x: PersonId) {
        g.persons[x].demand = None;
        g.persons[x].demand_rest_until = g.game_turn() + 1000;
    }

    /// The next game turn: player 0 sets its Houses' tribute, then its persons turn (its Houses
    /// pay, their rates in force step).
    fn next_turn(g: &mut Game) {
        let t = g.game_turn();
        g.set_game_turn(t + 1);
        super::tribute(g, P0);
        g.player_persons_turn(P0);
    }

    /// The rate set is the highest whose loyalty once in force keeps the House Pleased with a point
    /// to spare (2 or more); the rate in force climbs to it a step every 5 turns, and the House
    /// ends Pleased with the point to spare.
    #[test]
    fn the_tribute_set_keeps_a_point_to_spare() {
        let (mut g, c) = world(&SPOTS[..2]);
        let (x, h) = house(&mut g, c[1]);
        quiet(&mut g, x);
        set_rate(&mut g, h, 10);
        set_loyalty(&mut g, h, 5);
        // Once in force: 40% is 5 - 3 = 2 (a point to spare), 50% is 1 (none).
        assert_eq!((g.house_loyalty_at_tribute(h, 40), g.house_loyalty_at_tribute(h, 50)), (2, 1));
        assert_eq!(tribute_for(&g, h), 40);
        super::tribute(&mut g, P0);
        assert_eq!(rates(&g, h), (40, 10), "set at once, in force step by step");
        assert_eq!(g.house_loyalty(h), 5, "the loyalty follows the rate in force");
        assert_eq!(g.house_tribute_turns(h, 40), 15);
        let mut seen = Vec::new();
        for _ in 0..15 {
            next_turn(&mut g);
            assert_eq!(g.houses[h].tribute, 40, "the rate set stays");
            seen.push(g.houses[h].tribute_now);
        }
        assert_eq!(seen, [10, 10, 10, 20, 20, 20, 20, 20, 30, 30, 30, 30, 30, 40, 40], "a step every 5 turns");
        assert_eq!((g.house_loyalty(h), g.house_state(h)), (2, LoyaltyState::Pleased));
    }

    /// The rate set is lowered as soon as the projection at it falls below the margin, as far as
    /// needed (to 0% when no rate keeps it); the rate in force follows a step every 5 turns. A
    /// Furious House's countdown stops once a step lifts it above Furious, when that comes in time.
    #[test]
    fn the_tribute_set_is_lowered_when_the_projection_falls() {
        let (mut g, c) = world(&SPOTS[..2]);
        let (x, h) = house(&mut g, c[1]);
        quiet(&mut g, x);
        set_rate(&mut g, h, 40);
        set_loyalty(&mut g, h, 2);
        super::tribute(&mut g, P0);
        assert_eq!(rates(&g, h), (40, 40), "kept: 2 once in force");
        // A grievance of 3: 40% projects -1, 10% projects 2.
        set_loyalty(&mut g, h, -1);
        assert_eq!(tribute_for(&g, h), 10);
        super::tribute(&mut g, P0);
        assert_eq!(rates(&g, h), (10, 40), "lowered at once; the rate in force lags");
        assert_eq!(g.house_tribute_turns(h, 10), 15);
        // No rate keeps the margin (0% projects 1): 0%.
        set_loyalty(&mut g, h, -3);
        assert_eq!(tribute_for(&g, h), 0);
        super::tribute(&mut g, P0);
        assert_eq!(rates(&g, h), (0, 40));

        // A Furious House at 20% (-6): its countdown starts, the rate set goes to 0%; the first
        // step (10%, -5) comes in 5 turns, within the countdown's 10, and stops it.
        set_rate(&mut g, h, 20);
        set_loyalty(&mut g, h, -6);
        assert!(!house_beyond_tribute(&g, h), "a cut arrives in time");
        next_turn(&mut g);
        assert!(g.houses[h].standing.countdown.is_some(), "Furious: the countdown runs");
        assert_eq!(rates(&g, h), (0, 20));
        for _ in 0..4 {
            next_turn(&mut g);
        }
        assert_eq!((rates(&g, h), g.house_loyalty(h)), ((0, 10), -5));
        assert!(g.houses[h].standing.countdown.is_some(), "told at the next states");
        next_turn(&mut g);
        assert!(g.houses[h].standing.countdown.is_none(), "the step stopped the countdown");
    }

    /// A House breaks away whatever the tribute does when it is Furious and still Furious at the
    /// rate in force when its countdown ends, its rate set to 0% now.
    #[test]
    fn a_house_beyond_the_tribute_is_one_no_cut_reaches_in_time() {
        let (mut g, c) = world(&SPOTS[..2]);
        let (x, h) = house(&mut g, c[1]);
        quiet(&mut g, x);
        // At 20% and -8: -6 at 0%, Furious even then.
        set_rate(&mut g, h, 20);
        set_loyalty(&mut g, h, -8);
        assert!(house_beyond_tribute(&g, h));
        // At 20% and -7, 10 turns left: 0% in force by then (-5).
        set_loyalty(&mut g, h, -7);
        let t = g.game_turn();
        g.houses[h].standing.countdown = Some(t + 10);
        assert!(!house_beyond_tribute(&g, h));
        // 3 turns left: no step before it ends (the first is 5 turns away).
        g.houses[h].standing.countdown = Some(t + 3);
        assert!(house_beyond_tribute(&g, h));
        // Not Furious: no.
        set_loyalty(&mut g, h, -5);
        assert!(!house_beyond_tribute(&g, h));
    }

    // ---- The levy ----

    /// The main army's requisition takes a House's city only while the House stays Pleased or
    /// better after the levy's `Levied -1`.
    #[test]
    fn the_levy_takes_only_a_house_that_stays_pleased() {
        let (mut g, c) = world(&SPOTS[..3]);
        let plot = g.city_plot(c[1]);
        let (_, h) = house(&mut g, c[1]);
        let hc = now(&g, plot);
        let hp = g.houses[h].player;
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let (x, y) = (g.cities[hc].x, g.cities[hc].y);
        g.player_init_unit(hp, w, x, y, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        g.city_clear_order_queue(hc);
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(P0, w, 8, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(P0, &army).unwrap();
        g.army_set_target_size(P0, a, 4).unwrap();
        let (u, _) = crate::armies::requisition_unit(&g, hc, a).expect("a unit to train");
        g.cities[hc].unit_production[u] = g.city_production_needed_unit(hc, u) - 1;
        for (total, state, levied) in [(3, LoyaltyState::Friendly, true), (2, LoyaltyState::Pleased, true), (1, LoyaltyState::Pleased, false), (0, LoyaltyState::Cautious, false), (-3, LoyaltyState::Annoyed, false)] {
            set_loyalty(&mut g, h, total);
            assert_eq!(g.house_state(h), state);
            assert_eq!(levy_city(&g, P0, hc), levied, "{state:?}");
            assert_eq!(levy_cities(&g, P0).contains(&hc), levied, "{state:?}");
            assert_eq!(crate::armies::requisition_cities(&g, P0, a).contains(&hc), levied, "{state:?}");
        }
    }

    // ---- Grants ----

    /// The best production city stays in the Crown; never the capital.
    #[test]
    fn the_best_production_city_is_kept() {
        let (mut g, cities) = world(&SPOTS[..8]);
        let kept = kept_city(&g, P0).expect("a Crown city kept");
        assert_ne!(kept, cities[0]);
        let rate = |g: &Game, c: CityId| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
        assert!(cities[1..].iter().all(|&c| rate(&g, c) <= rate(&g, kept)));
        let _y = person(&mut g, 13, 6);
        let (c, _) = choose_grant(&g, P0).expect("a grant");
        assert!(c != kept && c != cities[0]);
    }

    /// Over its limit of 3 the Crown grants its cities, the least populous first, never the capital
    /// nor the best production city; at its limit it grants nothing.
    #[test]
    fn the_crown_grants_what_is_over_its_limit() {
        let (g3, _) = world(&SPOTS[..3]);
        assert!(g3.player_crown_over(P0).is_empty(), "at its limit");
        assert!(grantable_cities(&g3, P0).is_empty());
        let (mut g, c) = world(&SPOTS[..6]);
        pops(&mut g, &c, &[6, 5, 4, 3, 2, 1]);
        assert_eq!(g.player_crown_over(P0).len(), 3, "over its limit");
        let kept = kept_city(&g, P0);
        let list = grantable_cities(&g, P0);
        assert!(!list.is_empty() && !list.contains(&c[0]) && kept.is_none_or(|k| !list.contains(&k)), "{list:?}");
        let pop: Vec<i32> = list.iter().map(|&k| g.cities[k].population).collect();
        assert!(pop.windows(2).all(|w| w[0] <= w[1]), "least populous first: {pop:?}");
        let y = person(&mut g, 13, 6);
        assert_eq!(choose_grant(&g, P0), Some((list[0], GrantTo::Person(y))));
        super::turn(&mut g, P0);
        assert!(g.players[P0].houses.len() == 1 && !g.persons[y].unit.is_some_and(|u| g.units.contains(u)), "founded: the veteran left the map");
    }

    /// A hero never founds a House: with only a hero, nothing is granted; a veteran founds.
    #[test]
    fn a_hero_never_founds_a_house() {
        let (mut g, c) = world(&SPOTS[..8]);
        let x = hero(&mut g, 13, 6);
        assert!(g.persons[x].leader.is_none());
        assert_eq!(choose_grant(&g, P0), None);
        let y = person(&mut g, 13, 5);
        assert!(matches!(choose_grant(&g, P0), Some((k, GrantTo::Person(z))) if z == y && k != c[0]));
    }

    /// A House at its size threshold (3 in a realm under 16 cities) is not enlarged; a new House is
    /// founded instead when a person can found one.
    #[test]
    fn a_full_house_is_not_enlarged_a_new_one_is_founded() {
        let (mut g, c) = world(&SPOTS);
        let (_, h) = house(&mut g, c[9]);
        for &k in &c[7..9] {
            g.player_grant_city(P0, k, GrantTo::House(h)).expect("granted");
        }
        assert_eq!(g.house_great_cities(h), 3);
        assert_eq!(g.house_size_threshold(h), 3);
        let crown = g.player_crown_cities(P0).len();
        assert_eq!(crown, 7);
        assert!(!house_has_room(&g, h));
        assert!(!grantable_cities(&g, P0).is_empty());
        assert_eq!(choose_grant(&g, P0), None, "the only House is full");
        let y = person(&mut g, 13, 6);
        assert!(matches!(choose_grant(&g, P0), Some((_, GrantTo::Person(z))) if z == y));
    }

    /// A grant that meets a standing demand comes first: the House whose head demands city C gets
    /// it, ahead of a less populous city and of founding a House for a person without a post.
    #[test]
    fn a_grant_meets_a_demand_first() {
        let (mut g, c) = world(&SPOTS);
        let (x, h) = house(&mut g, c[9]);
        pops(&mut g, &c[..9], &[5, 5, 5, 5, 5, 5, 3, 2, 1]);
        let _y = person(&mut g, 13, 6);
        assert_eq!(&grantable_cities(&g, P0)[..3], &[c[8], c[7], c[6]]);
        let plot = g.city_plot(c[6]);
        demand(&mut g, x, DemandKind::GrantCity(plot));
        assert_eq!(choose_grant(&g, P0), Some((c[6], GrantTo::House(h))));
        super::turn(&mut g, P0);
        assert_eq!(g.city_house(now(&g, plot)), Some(h));
        assert_eq!(g.persons[x].demand, None, "met");
    }

    /// A grant whose preview turns a holder Furious is not made when another is not: the person
    /// without a post demands one, the House claims the two Crown cities near its seat and the snub
    /// (-1) of the person founding there would make it Furious, so that grant is not the one made.
    #[test]
    fn a_snub_that_makes_a_holder_furious_is_avoided() {
        let (mut g, c) = world(&SPOTS);
        let (_, h) = house(&mut g, c[9]);
        set_loyalty(&mut g, h, -5);
        assert_eq!(g.house_state(h), LoyaltyState::Annoyed);
        let y = person(&mut g, 13, 6);
        demand(&mut g, y, DemandKind::Post);
        assert!(grantable_cities(&g, P0).iter().any(|k| c[7..9].contains(k)));
        for &k in &c[7..9] {
            let cand = g.grant_candidates(P0, k).into_iter().find(|cand| cand.to == GrantTo::Person(y)).unwrap();
            assert!(cand.lines.iter().any(|l| l.state_after == Some(LoyaltyState::Furious)), "{:?}", cand.lines);
        }
        // The grant chosen makes no holder Furious (with every Crown city over the limit to choose
        // from, it may well meet the Annoyed House's own demand first).
        let (k, to) = choose_grant(&g, P0).expect("a grant");
        let cand = g.grant_candidates(P0, k).into_iter().find(|cand| cand.to == to).expect("a candidate");
        assert!(cand.lines.iter().all(|l| l.state_after != Some(LoyaltyState::Furious)), "{k:?} {to:?}: {:?}", cand.lines);
        assert!(!(to == GrantTo::Person(y) && c[7..9].contains(&k)), "not y on a claimed city");
    }

    // ---- Offers ----

    /// An AI House accepts an offer to secede that covers its price and refuses one below it.
    #[test]
    fn an_ai_house_takes_its_price_and_refuses_less() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let (_, h) = house(&mut g, c[1]);
        let hp = g.houses[h].player;
        g.players[P1].gold = 1000;
        set_loyalty(&mut g, h, 2);
        assert_eq!(g.house_bribe_price(h), None, "Pleased: no price");
        assert!(!g.player_offer_house(P1, hp, &[gold(900)], &[secede()]));
        set_loyalty(&mut g, h, -1);
        let price = g.house_bribe_price(h).expect("Cautious: a price");
        assert_eq!(price, 5 * 25);
        assert!(!g.player_offer_house(P1, hp, &[gold(price - 1)], &[secede()]), "below the price");
        assert!(g.houses.contains(h));
        assert!(g.player_offer_house(P1, hp, &[gold(price)], &[secede()]), "the price");
        assert!(!g.houses.contains(h), "broke away");
        assert_eq!(g.players[P1].gold, 1000 - price);
        assert!(g.at_war(g.players[P0].team, g.players[hp].team));
    }

    fn offer(house_player: PlayerId, kind: BribeKind, price: i32) -> Option<Offer> {
        Some(Offer { house_player, kind, give: vec![gold(price)] })
    }

    /// At war, the buyer courts the cheapest House it can pay, not one it offered lately nor a
    /// Pleased one; at peace, none. Able to pay with half its treasury, it asks Become your vassal:
    /// the House breaks away as its House, at war with the old liege.
    #[test]
    fn the_buyer_courts_the_cheapest_house_it_can_pay() {
        let (mut g, c) = world(&SPOTS[..4]);
        g.player_init_city(P1, 15, 7, true, true);
        let (_, h1) = house(&mut g, c[1]);
        let (_, h2) = house(&mut g, c[2]);
        let (_, h3) = house(&mut g, c[3]);
        set_loyalty(&mut g, h1, 0);
        set_loyalty(&mut g, h2, -3);
        set_loyalty(&mut g, h3, 2);
        assert_eq!((g.house_bribe_price(h1), g.house_bribe_price(h2), g.house_bribe_price(h3)), (Some(150), Some(75), None));
        let (hp1, hp2) = (g.houses[h1].player, g.houses[h2].player);
        let floor = crate::civs::gold_floor(&g, P1);
        g.players[P1].gold = floor + 600;
        assert_eq!(choose_offer(&g, P1), None, "at peace");
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_declare_war(t1, t0, false, None);
        assert_eq!(choose_offer(&g, P1), offer(hp2, BribeKind::Vassal, 75), "the cheapest");
        // Offered lately: the next one.
        g.house_note_offer(P1, hp2, &[gold(10)]);
        assert_eq!(choose_offer(&g, P1), offer(hp1, BribeKind::Vassal, 150));
        // Its turn: offered and accepted; the House breaks away as the buyer's House.
        super::turn(&mut g, P1);
        assert!(!g.houses.contains(h1), "its record with the old liege ended");
        let nt = g.players[hp1].team;
        assert!(g.teams[nt].vassal[t1], "the buyer's vassal");
        assert_eq!(g.player_house(hp1).map(|h| g.houses[h].owner), Some(P1), "a House of the buyer's");
        assert!(g.at_war(t0, nt) && !g.at_war(t1, nt));
        assert_eq!(g.players[P1].gold, floor + 450);
        // Ten turns later the second House again.
        let t = g.game_turn();
        g.set_game_turn(t + OFFER_PERIOD);
        assert_eq!(choose_offer(&g, P1), offer(hp2, BribeKind::Vassal, 75));
    }

    /// The buyer asks Become your vassal when the price is at most half its treasury, else Secede
    /// when it is at most a third; when it cannot take a vassal, Secede at a third.
    #[test]
    fn the_buyer_asks_become_your_vassal_with_half_its_treasury() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let (_, h) = house(&mut g, c[1]);
        set_loyalty(&mut g, h, -3);
        let hp = g.houses[h].player;
        assert_eq!(g.house_bribe_price(h), Some(75));
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_declare_war(t1, t0, false, None);
        let floor = crate::civs::gold_floor(&g, P1);
        g.players[P1].gold = floor + 150;
        assert_eq!(choose_offer(&g, P1), offer(hp, BribeKind::Vassal, 75), "half of 150 pays 75");
        g.players[P1].gold = floor + 149;
        assert_eq!(choose_offer(&g, P1), None, "half of 149 does not, nor a third");
        // A buyer that cannot take a vassal (no vassal states): Secede, with a third.
        g.setup.options[GameOptionType::GAMEOPTION_NO_VASSAL_STATES] = true;
        assert!(g.house_bribe_refusal(h, P1, BribeKind::Vassal).is_some());
        g.players[P1].gold = floor + 225;
        assert_eq!(choose_offer(&g, P1), offer(hp, BribeKind::Secede, 75), "a third of 225 pays 75");
        g.players[P1].gold = floor + 224;
        assert_eq!(choose_offer(&g, P1), None);
    }

    // ---- Civics ----

    /// A direct-rule civic is barred while a House stands, and the AI's civics choice adopts none.
    #[test]
    fn no_direct_rule_civic_with_a_house() {
        let (mut g, cities) = world(&SPOTS[..2]);
        let despotism = g.d.t::<CivicType>("CIVIC_DESPOTISM");
        assert!(g.d.civics[despotism].direct_rule);
        assert!(!civic_barred(&g, P0, despotism), "no House yet");
        house(&mut g, cities[1]);
        assert!(civic_barred(&g, P0, despotism));
        assert!(!civic_barred(&g, P0, g.d.t::<CivicType>("CIVIC_ARISTOCRACY")));
        for turn in 0..20 {
            g.set_game_turn(turn);
            crate::empire::manage(&mut g, P0);
            assert!(!g.player_rules_directly(P0), "turn {turn}: a direct-rule civic adopted with a House");
        }
    }
}
