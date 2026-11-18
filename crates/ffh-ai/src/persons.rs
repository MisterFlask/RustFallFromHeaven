//! Persons of Note (docs/PERSONS.md; docs/AI.md, "Persons of Note"): the AI of the second build
//! (Houses as vassal civilizations, rule 12). A rules change is not gated on the AI (CLAUDE.md):
//! reasonable first choices, not measured ones. AI players only: a human's grants, tribute and
//! offers are the human's.
//!
//! Once a turn ([`turn`], after the empire's civics and religion and the turn's diplomacy):
//! - **Tribute** ([`tribute_for`]): each House pays the realm's default (20%) while it is Pleased or
//!   Friendly at that rate, 10% while it would be Cautious, nothing while it would be Annoyed or
//!   worse. Judged at the default rate, so the choice does not feed on itself (a House that the
//!   lower rate lifts from Cautious to Pleased keeps the lower rate). Set only when it changes.
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
//! - **Offers** ([`choose_offer`]): at war with a civilization, Secede offered to one of its Houses
//!   that is Cautious or worse, at its price ([`Game::house_bribe_price`]), in gold when that is at
//!   most a third of the treasury, else in gold per turn when that is at most a third of the gold
//!   rate; the cheapest first, one offer a turn, the same House again only after
//!   [`OFFER_PERIOD`] turns. Through `player_offer_house`, the deal path the human's offers take.
//!
//! Elsewhere: the levy ([`levy_city`]: `armies::requisition_cities` adds the cities of a Pleased
//! or Friendly House); a direct-rule civic is never adopted while a House stands
//! ([`civic_barred`], `empire.rs`); an idle army stands in an unheld city of a Furious House
//! ([`disloyal_house_post`], `military.rs`: the city goes back to the liege when the House breaks
//! away); a Furious commander's army goes to the city of ours with the largest garrison
//! ([`disloyal_commander_haven`]); a demanded war weighs on the war planning ([`war_demanded`]); a
//! House's own turn skips what a vassal cannot do ([`is_house`], `lib.rs`). A House answers offers
//! by the rules' price (`diplomacy::judge_offer`). The AI never frees a House.

use std::cmp::Reverse;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{DemandKind, GrantCandidate, GrantTo, LoyaltyCause, LoyaltyState, Post, OFFER_NOTED_TURNS, TRIBUTE_DEFAULT};
use ffh_core::structs::TradeData;
use ffh_data::*;

use crate::view::View;


/// The tribute of a House that would be Cautious at the default rate (percent).
pub const TRIBUTE_CAUTIOUS: i32 = 10;

/// The turns before the AI offers the same House again.
pub const OFFER_PERIOD: i32 = 10;

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
    if let Some((hp, give)) = choose_offer(g, p) {
        let label = g.player_house(hp).map(|h| g.house_label(h)).unwrap_or_default();
        let get = [TradeData { item_type: TradeableItem::TRADE_SECEDE, data: 0, offering: false, hidden: false }];
        let accepted = g.player_offer_house(p, hp, &give, &get);
        crate::trace!(g, p, "persons: Secede offered to {label} for {} ({})", ffh_core::persons::house_offer_value(&give), if accepted { "accepted" } else { "refused" });
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

/// The tribute (percent) the AI liege takes from House `h`: the default while the House would be
/// Pleased or Friendly paying it, [`TRIBUTE_CAUTIOUS`] while it would be Cautious, nothing while
/// it would be Annoyed or worse.
pub fn tribute_for(g: &Game, h: HouseId) -> i32 {
    match LoyaltyState::of(g.house_loyalty_at_tribute(h, TRIBUTE_DEFAULT)) {
        LoyaltyState::Friendly | LoyaltyState::Pleased => TRIBUTE_DEFAULT,
        LoyaltyState::Cautious => TRIBUTE_CAUTIOUS,
        LoyaltyState::Annoyed | LoyaltyState::Furious => 0,
    }
}

/// `p` sets each House's tribute to [`tribute_for`], where it changes.
fn tribute(g: &mut Game, p: PlayerId) {
    for h in houses_of(g, p) {
        let pct = tribute_for(g, h);
        if g.houses[h].tribute != pct && g.player_set_house_tribute(p, h, pct).is_ok() {
            crate::trace!(g, p, "persons: {} pays {pct}% ({})", g.house_label(h), g.house_state(h).name());
        }
    }
}

// -------------------------------------------------------------------------------------------
// The levy
// -------------------------------------------------------------------------------------------

/// Whether `p`'s armies levy from city `c` of one of its Houses: the House is Pleased or Friendly
/// (a levy is a grievance; the rules take none from a House Annoyed or worse).
pub fn levy_city(g: &Game, p: PlayerId, c: CityId) -> bool {
    g.city_levied_by(c, p) && g.city_house(c).is_some_and(|h| g.house_state(h) <= LoyaltyState::Pleased)
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

/// What `p` offers House `h` (of a civilization it is at war with) for Secede at its price, if it
/// can pay it: gold up to a third of its treasury (the gold it keeps left out), else gold per turn
/// up to a third of its gold rate, as the rules let it trade them.
fn offer_for(g: &Game, p: PlayerId, h: HouseId) -> Option<Vec<TradeData>> {
    let price = g.house_bribe_price(h)?;
    let hp = g.houses[h].player;
    if price <= 0 {
        return Some(Vec::new());
    }
    let item = |item_type: TradeableItem, data: i32| TradeData { item_type, data, offering: false, hidden: false };
    let treasury = g.players[p].gold - crate::civs::gold_floor(g, p);
    let gold = item(TradeableItem::TRADE_GOLD, price);
    if price <= treasury / 3 && g.player_can_trade_item(p, hp, &gold, false) {
        return Some(vec![gold]);
    }
    let turns = ffh_core::persons::BRIBE_GOLD_PER_TURN_TURNS;
    let per_turn = (price + turns - 1) / turns;
    let gpt = item(TradeableItem::TRADE_GOLD_PER_TURN, per_turn);
    (per_turn <= g.player_calculate_gold_rate(p) / 3 && g.player_can_trade_item(p, hp, &gpt, false)).then(|| vec![gpt])
}

/// The offer `p` makes this turn, if any: (the House's player, what `p` gives). See the module
/// comment.
pub fn choose_offer(g: &Game, p: PlayerId) -> Option<(PlayerId, Vec<TradeData>)> {
    if g.players[p].human || !g.player_has_persons(p) {
        return None;
    }
    let team = g.players[p].team;
    let by = g.player_get_civilization_short_description(p);
    let secede = TradeData { item_type: TradeableItem::TRADE_SECEDE, data: 0, offering: false, hidden: false };
    let mut best: Option<((i32, HouseId), PlayerId, Vec<TradeData>)> = None;
    for q in g.player_ids() {
        let pl = &g.players[q];
        if !pl.alive || pl.houses.is_empty() || !g.at_war(team, pl.team) {
            continue;
        }
        for h in houses_of(g, q) {
            let hp = g.houses[h].player;
            if g.house_bribe_refusal(h, p, ffh_core::persons::BribeKind::Secede).is_some() || !g.player_can_trade_item(hp, p, &secede, false) || offered_lately(g, h, &by) {
                continue;
            }
            let Some(price) = g.house_bribe_price(h) else { continue };
            let Some(give) = offer_for(g, p, h) else { continue };
            let key = (price, h);
            if best.as_ref().map_or(true, |(k, ..)| key < *k) {
                best = Some((key, hp, give));
            }
        }
    }
    best.map(|(_, hp, give)| (hp, give))
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

/// The armies of `p` standing on `plot`, but `except`.
fn armies_at(g: &Game, p: PlayerId, plot: PlotIdx, except: Option<ArmyId>) -> usize {
    g.players[p].armies.iter().filter(|&&a| Some(a) != except && g.armies.contains(a) && g.army_plot(a) == Some(plot)).count()
}

/// Where an idle army stack (`grp`, led by `u`) of `v.p` stands against a break-away: a city of a
/// Furious House where no army of ours stands (a city where one stands goes back to us), the
/// largest first, within 8 turns; or the city it holds alone already (its plot).
pub(crate) fn disloyal_house_post(g: &Game, v: &mut View, grp: GroupId, u: UnitId) -> Option<PlotIdx> {
    let p = v.p;
    let mut cities: Vec<CityId> = Vec::new();
    for &h in &g.players[p].houses {
        if g.houses.contains(h) && g.house_state(h) == LoyaltyState::Furious {
            cities.extend(g.house_cities(h));
        }
    }
    if cities.is_empty() {
        return None;
    }
    cities.sort_by_key(|&c| (std::cmp::Reverse(g.cities[c].population), c));
    let here = g.unit_plot(u);
    let mine = g.units[u].army.or_else(|| g.players[p].armies.iter().copied().find(|&a| g.armies.get(a).is_some_and(|army| army.group == Some(grp))));
    if cities.iter().any(|&c| g.city_plot(c) == here) && armies_at(g, p, here, mine) == 0 {
        return Some(here);
    }
    let open: Vec<PlotIdx> = cities.iter().map(|&c| g.city_plot(c)).filter(|&q| armies_at(g, p, q, None) == 0).collect();
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
    use ffh_core::persons::{Ambition, Demand, LoyaltyMod, BribeKind};

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

    /// The tribute follows the House's state at the default rate: 20% Pleased or Friendly, 10%
    /// Cautious, nothing Annoyed or Furious; set again, it does not move.
    #[test]
    fn tribute_by_the_state_at_the_default_rate() {
        let (mut g, c) = world(&SPOTS[..2]);
        let (_, h) = house(&mut g, c[1]);
        for (at_default, pct) in [(3, 20), (1, 20), (0, 10), (-2, 10), (-3, 0), (-5, 0), (-6, 0)] {
            g.houses[h].tribute = TRIBUTE_DEFAULT;
            set_loyalty(&mut g, h, at_default);
            assert_eq!(tribute_for(&g, h), pct, "loyalty {at_default} at 20%");
            super::tribute(&mut g, P0);
            assert_eq!(g.houses[h].tribute, pct, "loyalty {at_default} at 20%");
            super::tribute(&mut g, P0);
            assert_eq!(g.houses[h].tribute, pct, "stable: loyalty {at_default} at 20%");
        }
    }

    // ---- The levy ----

    /// The main army's requisition takes a House's city only while the House is Pleased or
    /// Friendly.
    #[test]
    fn the_levy_takes_only_a_pleased_or_friendly_house() {
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
        for (total, state, levied) in [(3, LoyaltyState::Friendly, true), (1, LoyaltyState::Pleased, true), (0, LoyaltyState::Cautious, false), (-3, LoyaltyState::Annoyed, false)] {
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

    /// At war, the buyer courts the cheapest House it can pay with a third of its treasury, not one
    /// it offered lately nor a Pleased one; at peace, none.
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
        g.players[P1].gold = 600;
        assert_eq!(choose_offer(&g, P1), None, "at peace");
        let (t0, t1) = (g.players[P0].team, g.players[P1].team);
        g.team_declare_war(t1, t0, false, None);
        assert_eq!(choose_offer(&g, P1), Some((hp2, vec![gold(75)])), "the cheapest");
        // Offered lately: the next one.
        g.house_note_offer(P1, hp2, &[gold(10)]);
        assert_eq!(choose_offer(&g, P1), Some((hp1, vec![gold(150)])));
        // Its turn: offered and accepted; the House breaks away at peace with the buyer.
        super::turn(&mut g, P1);
        assert!(!g.houses.contains(h1), "seceded");
        let nt = g.players[hp1].team;
        assert!(g.at_war(t0, nt) && !g.at_war(t1, nt));
        assert_eq!(g.players[P1].gold, 450);
        // Ten turns later the second House again, while a third of the treasury pays it.
        let t = g.game_turn();
        g.set_game_turn(t + OFFER_PERIOD);
        assert_eq!(choose_offer(&g, P1), Some((hp2, vec![gold(75)])));
        g.players[P1].gold = 200;
        if g.player_calculate_gold_rate(P1) / 3 < 8 {
            assert_eq!(choose_offer(&g, P1), None, "a third of 200 does not pay 75");
        }
        assert!(g.house_bribe_refusal(h2, P1, BribeKind::Secede).is_none());
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
