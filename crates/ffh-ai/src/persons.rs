//! Houses (docs/PERSONS.md; docs/AI.md, "Houses"): the AI of the second build (Houses as vassal
//! civilizations, rule 12) as the third build left it (rule 11: no persons; the first grant
//! founds a House) and the fourth changed it (rule 9: courting is a cultural channel,
//! `culture.rs`; no buyer). A rules change is not gated on the AI (CLAUDE.md): reasonable first
//! choices, not measured ones. AI players only: a human's grants and tribute are the human's.
//!
//! Once a turn ([`turn`], after the empire's civics and religion and the turn's diplomacy):
//! - **Tribute** ([`tribute_for`]): each House's rate set to the highest whose loyalty once in
//!   force is Pleased with a point to spare (2 or more, [`TRIBUTE_KEEP_LOYALTY`]); lowered as soon
//!   as the projection at the rate set falls below that. The rate in force lags the rate set (rule
//!   4: a step every 5 turns), so a cut no longer helps at once: the point to spare is what lets
//!   the House take a grievance while a cut arrives. So the AI's Houses rarely reach an ultimatum,
//!   and a loyal House is dearer to court (the requirement counts its loyalty). Never above a
//!   contract's bound (the third build, rule 6). Set only when it changes.
//! - **Grants** ([`choose_grant`]): one a turn while the Crown is over its limit (rule 14: 3 cities,
//!   the capital alone for the Kuriotates; settlements never count), the least populous first,
//!   never the capital nor the best production city ([`kept_city`]), to the House whose capital is
//!   nearest the city ([`nearest_house`]); with no House, the grant founds one (rule 3; none when
//!   no leader is free). A grant whose preview makes a House Furious is passed over for the next
//!   city.
//!
//! Elsewhere: the levy ([`levy_city`]: `armies::requisition_cities` adds the cities of a House
//! that stays Pleased or better after the levy's `Levied -1`); a direct-rule civic is never
//! adopted while a House stands ([`civic_barred`], `empire.rs`); an idle army goes to hold the
//! most valuable city it can hold of a House Annoyed or worse, one step from an ultimatum or under
//! a contract ([`house_to_hold`]), or of a House whose strongest rival favour is past half its
//! requirement ([`courted_to_hold`]: the fourth build, rule 9); [`disloyal_house_post`], `military.rs`, the march [`hold_post`]: a city
//! where the liege's army units outnumber the House's goes back to the liege when it breaks away,
//! and holding every city lets the AI refuse an ultimatum); its answer to an ultimatum
//! ([`consider_ultimatum`]: accept, unless every city of the House would be held or the demand is
//! war on a civilization of more power than its own: the third build, rule 11); a requested war
//! weighs on the war planning ([`war_demanded`]); a House's own turn skips what a vassal cannot do
//! ([`is_house`], `lib.rs`). A House accepts any gift and nothing else (`diplomacy::judge_offer`).
//! The AI gives no gifts to court, and never frees a House.

use std::cmp::Reverse;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{DemandKind, GrantTo, LoyaltyState, Ultimatum, UltimatumKind, LEVIED, PLEASED_AT, TRIBUTE_MAX, TRIBUTE_STEP};
use ffh_data::*;

use crate::view::View;

/// The loyalty the AI liege keeps a House at once its tribute is in force: Pleased with a point
/// to spare (the rate in force lags a cut, rule 4).
pub const TRIBUTE_KEEP_LOYALTY: i32 = PLEASED_AT + 1;

/// The Houses' turn of AI player `p`: their tribute, one grant.
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.players[p].human || !g.player_has_houses(p) {
        return;
    }
    tribute(g, p);
    if let Some((c, to)) = choose_grant(g, p) {
        let city = g.d.text.name(&g.cities[c].name).to_string();
        match g.player_grant_city(p, c, to) {
            Ok(h) => crate::trace!(g, p, "houses: {city} granted to {}", g.house_label(h)),
            Err(e) => crate::trace!(g, p, "houses: no grant of {city}: {e}"),
        }
    }
}

/// Whether `p` is a House: a vassal civilization, which plays only its own land (its liege makes
/// its war and peace).
pub fn is_house(g: &Game, p: PlayerId) -> bool {
    g.player_house(p).is_some()
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
/// the margin, so the cut starts moving at once. Never a rate a contract's bound refuses (the third
/// build, rule 6: the rules' `player_check_house_tribute`).
pub fn tribute_for(g: &Game, h: HouseId) -> i32 {
    let p = g.houses[h].owner;
    (0..=TRIBUTE_MAX / TRIBUTE_STEP)
        .rev()
        .map(|k| k * TRIBUTE_STEP)
        .find(|&pct| g.house_loyalty_at_tribute(h, pct) >= TRIBUTE_KEEP_LOYALTY && g.player_check_house_tribute(p, h, pct).is_ok())
        .unwrap_or(0)
}

/// `p` sets each House's tribute to [`tribute_for`], where it changes.
fn tribute(g: &mut Game, p: PlayerId) {
    for h in houses_of(g, p) {
        let pct = tribute_for(g, h);
        if g.houses[h].tribute != pct && g.player_set_house_tribute(p, h, pct).is_ok() {
            crate::trace!(g, p, "houses: {} set to {pct}%, {}% in force ({})", g.house_label(h), g.houses[h].tribute_now, g.house_state(h).name());
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

/// The House among `houses` whose capital (its seat: `Game::house_seat`) is nearest city `c`
/// (ties: the lower id).
pub fn nearest_house(g: &Game, houses: &[HouseId], c: CityId) -> Option<HouseId> {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    houses.iter().copied().filter_map(|h| g.house_seat(h).map(|s| (g.plot_distance(x, y, g.cities[s].x, g.cities[s].y), h))).min().map(|(_, h)| h)
}

/// The grant `p` makes this turn, if any: see the module comment.
pub fn choose_grant(g: &Game, p: PlayerId) -> Option<(CityId, GrantTo)> {
    if g.player_check_grants(p).is_err() {
        return None;
    }
    let cities = grantable_cities(g, p);
    let houses = houses_of(g, p);
    if houses.is_empty() {
        // The first grant founds a House (rule 3), when a leader is free.
        return cities.first().filter(|_| g.player_check_found_house(p).is_ok()).map(|&c| (c, GrantTo::NewHouse));
    }
    for c in cities {
        let Some(h) = nearest_house(g, &houses, c) else { continue };
        let to = GrantTo::House(h);
        // A grant that makes a House Furious waits: the Crown keeping the city snubs nobody.
        if g.grant_preview(p, c, to).iter().any(|l| l.state_after == Some(LoyaltyState::Furious)) {
            continue;
        }
        return Some((c, to));
    }
    None
}

/// Whether a House of `p` requests war on `other`'s team (a small bonus in the war planning:
/// `diplomacy::manage`).
pub(crate) fn war_demanded(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let team = g.players[other].team;
    houses_of(g, p).into_iter().any(|h| matches!(g.houses[h].demand.map(|dm| dm.kind), Some(DemandKind::War(q)) if g.players[q].team == team))
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

// -------------------------------------------------------------------------------------------
// Ultimatums (the third build, rules 5 and 11)
// -------------------------------------------------------------------------------------------

/// `Controller::consider_ultimatum` for AI liege `p`: accept, unless every city of the House would
/// be held when it broke away (`Game::house_cities_held`: refusing then takes nothing) or the
/// demand is war on a civilization of more power than `p`'s team (`Game::team_get_power`, vassals
/// included).
pub fn consider_ultimatum(g: &Game, p: PlayerId, u: &Ultimatum) -> bool {
    if !g.houses.contains(u.house) {
        return true;
    }
    let (going, _) = g.house_cities_held(u.house);
    if going.is_empty() {
        crate::trace!(g, p, "houses: {}'s ultimatum refused: every city is held", g.house_label(u.house));
        return false;
    }
    if let UltimatumKind::War(q) = u.kind {
        let (pt, qt) = (g.players[p].team, g.players[q].team);
        if g.team_get_power(qt, true) > g.team_get_power(pt, true) {
            crate::trace!(g, p, "houses: {}'s ultimatum refused: war on a stronger civilization", g.house_label(u.house));
            return false;
        }
    }
    true
}

/// Whether the AI liege holds House `h`'s cities with its idle armies (the third build, rule 11:
/// the second build's march, its trigger moved): the House is Annoyed or worse as the rules count
/// it (a contract's floor included), one step from an ultimatum or under one. Holding every city of
/// it is what lets the liege refuse an ultimatum ([`consider_ultimatum`]).
pub fn house_to_hold(g: &Game, h: HouseId) -> bool {
    g.houses.contains(h) && g.house_state(h).withholds()
}

// ---- The fourth build, rule 9: a courted House held -------------------------------------------

/// Whether the liege of House `h` holds its cities against a rival's courtship (the fourth build,
/// rule 9): the strongest rival favour at it is past half its requirement (a split takes the
/// cities its liege's armies do not hold).
pub fn courted_to_hold(g: &Game, h: HouseId) -> bool {
    g.houses.contains(h) && g.house_courtships(h).iter().any(|cv| 2 * cv.favour as i64 > 100 * cv.requirement.total as i64)
}

/// Where an idle army stack (`grp`, led by `u`) of `v.p` goes to hold the cities of its Houses to
/// hold ([`house_to_hold`], [`courted_to_hold`]): see [`hold_post`].
pub(crate) fn disloyal_house_post(g: &Game, v: &mut View, grp: GroupId, u: UnitId) -> Option<PlotIdx> {
    let p = v.p;
    let mut cities: Vec<CityId> = Vec::new();
    for &h in &g.players[p].houses {
        if g.houses.contains(h) && (house_to_hold(g, h) || courted_to_hold(g, h)) {
            cities.extend(g.house_cities(h));
        }
    }
    hold_post(g, v, grp, u, cities)
}

/// The march to hold Houses' cities (`cities`, of `v.p`'s Houses): a House that breaks away takes
/// every city where our army units do not outnumber its own land combat units (`Game::city_hold`,
/// rule 7). The stack holds the most valuable of `cities` it can hold (the most populous first,
/// ties: the older): one not held now where its army units, with ours already there, would
/// outnumber the House's, within 8 turns; or it stays in the city it stands in when that city is
/// held only with it (its plot). Public: the Houses a rival courts are held the same way.
pub fn hold_post(g: &Game, v: &mut View, grp: GroupId, u: UnitId, mut cities: Vec<CityId>) -> Option<PlotIdx> {
    let p = v.p;
    if cities.is_empty() {
        return None;
    }
    // The stack's units that count in the contest (an army's land units).
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

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::persons::{LoyaltyCause, LoyaltyMod};

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    /// Ten city sites of the one-landmass map, three plots apart.
    const SPOTS: [(i32, i32); 10] = [(3, 4), (6, 4), (9, 4), (12, 4), (15, 4), (3, 7), (6, 7), (9, 7), (12, 7), (15, 7)];

    /// The one-landmass duel with player 0 under Aristocracy (cities may be granted; the Crown's
    /// limit is 3) holding cities at `spots`, the first its capital; the two players have met.
    fn world(spots: &[(i32, i32)]) -> (Game, Vec<CityId>) {
        // (The Bannor: a civilization with minor leaders, so Houses can be founded.)
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

    /// A House of player 0 founded with city `seat` (a leader drawn as a founding draws it),
    /// without a request for the test's turns.
    fn house(g: &mut Game, seat: CityId) -> HouseId {
        let civ = g.player_get_civilization_type(P0).expect("a civilization");
        let leader = g.house_leader_draw(civ, P0).expect("a free leader");
        let (h, _) = g.player_found_house(P0, leader, &[seat]).expect("founded");
        quiet(g, h);
        h
    }

    /// House `h` makes no request for the test's turns.
    fn quiet(g: &mut Game, h: HouseId) {
        g.houses[h].demand = None;
        g.houses[h].demand_rest_until = g.game_turn() + 1000;
    }

    /// House `h`'s loyalty set to `total` (one modifier) at its tribute now.
    fn set_loyalty(g: &mut Game, h: HouseId, total: i32) {
        g.houses[h].standing.mods.clear();
        let base = g.house_loyalty(h);
        let t = g.game_turn();
        g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: total - base, until: Some(t + 60) });
        assert_eq!(g.house_loyalty(h), total);
    }

    fn pops(g: &mut Game, cities: &[CityId], pops: &[i32]) {
        for (&c, &n) in cities.iter().zip(pops) {
            g.city_set_population(c, n);
        }
    }

    fn gold(data: i32) -> ffh_core::structs::TradeData {
        ffh_core::structs::TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false }
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

    /// The next game turn: player 0 sets its Houses' tribute, then its Houses' turn (they pay,
    /// their rates in force step).
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
        let h = house(&mut g, c[1]);
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
    /// needed (to 0% when no rate keeps it); the rate in force follows a step every 5 turns.
    #[test]
    fn the_tribute_set_is_lowered_when_the_projection_falls() {
        let (mut g, c) = world(&SPOTS[..2]);
        let h = house(&mut g, c[1]);
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
    }

    /// The tribute rule never sets a rate above a contract's bound (the third build, rule 6).
    #[test]
    fn the_tribute_set_keeps_to_a_contract() {
        let (mut g, c) = world(&SPOTS[..2]);
        let h = house(&mut g, c[1]);
        set_rate(&mut g, h, 10);
        set_loyalty(&mut g, h, 6);
        assert_eq!(tribute_for(&g, h), 50, "no contract: the highest rate with a point to spare");
        let until = g.game_turn() + 20;
        g.houses[h].contract = Some(ffh_core::persons::Contract { kind: UltimatumKind::Tribute(10), until, deal: None });
        assert_eq!(tribute_for(&g, h), 10, "the bound");
        super::tribute(&mut g, P0);
        assert_eq!(rates(&g, h), (10, 10));
    }

    /// The House's cities are held from Annoyed, one step from an ultimatum, and under a contract
    /// (its floor counts as Annoyed); not from Cautious.
    #[test]
    fn a_house_to_hold_is_annoyed_or_worse() {
        let (mut g, c) = world(&SPOTS[..2]);
        let h = house(&mut g, c[1]);
        set_rate(&mut g, h, 20);
        for (total, hold) in [(0, false), (-2, false), (-3, true), (-5, true)] {
            set_loyalty(&mut g, h, total);
            assert_eq!(house_to_hold(&g, h), hold, "{total}");
        }
        // Under a contract: at least Annoyed whatever its parts make it.
        set_loyalty(&mut g, h, -9);
        let until = g.game_turn() + 20;
        g.houses[h].contract = Some(ffh_core::persons::Contract { kind: UltimatumKind::Gold(3), until, deal: None });
        assert_eq!(g.house_state(h), LoyaltyState::Annoyed);
        assert!(house_to_hold(&g, h));
    }

    /// The AI accepts an ultimatum, but refuses it when every city of the House would be held (its
    /// break-away takes nothing) or when it demands war on a civilization of more power.
    #[test]
    fn the_ai_accepts_an_ultimatum_unless_it_holds_everything_or_faces_a_stronger_war() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let h = house(&mut g, c[1]);
        let until = g.game_turn() + 20;
        let gold = Ultimatum { house: h, kind: UltimatumKind::Gold(4), request: false, until };
        assert!(consider_ultimatum(&g, P0, &gold), "a House that takes its city: accept");
        // War on player 1: refused while player 1 has more power.
        let war = Ultimatum { house: h, kind: UltimatumKind::War(P1), request: false, until };
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        let (pt, qt) = (g.players[P0].team, g.players[P1].team);
        while g.team_get_power(qt, true) <= g.team_get_power(pt, true) {
            g.player_init_unit(P1, w, 15, 7, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        assert!(!consider_ultimatum(&g, P0, &war), "a stronger civilization");
        while g.team_get_power(pt, true) <= g.team_get_power(qt, true) {
            g.player_init_unit(P0, w, 3, 4, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        assert!(consider_ultimatum(&g, P0, &war), "a weaker civilization");
        // Every city held: an army of ours outnumbers the House's units in its only city.
        let hc = g.house_cities(h)[0];
        let (x, y) = (g.cities[hc].x, g.cities[hc].y);
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(P0, w, x, y, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        g.army_form(P0, &units).unwrap();
        g.player_army_sync(P0);
        assert!(g.house_cities_held(h).0.is_empty(), "held: {:?}", g.city_hold(hc));
        assert!(!consider_ultimatum(&g, P0, &gold), "every city held: refuse");
    }

    // ---- The levy ----

    /// The main army's requisition takes a House's city only while the House stays Pleased or
    /// better after the levy's `Levied -1`.
    #[test]
    fn the_levy_takes_only_a_house_that_stays_pleased() {
        let (mut g, c) = world(&SPOTS[..3]);
        let plot = g.city_plot(c[1]);
        let h = house(&mut g, c[1]);
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
        let (g, cities) = world(&SPOTS[..8]);
        let kept = kept_city(&g, P0).expect("a Crown city kept");
        assert_ne!(kept, cities[0]);
        let rate = |g: &Game, c: CityId| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
        assert!(cities[1..].iter().all(|&c| rate(&g, c) <= rate(&g, kept)));
        let (c, _) = choose_grant(&g, P0).expect("a grant");
        assert!(c != kept && c != cities[0]);
    }

    /// Over its limit of 3 the Crown grants its cities, the least populous first, never the capital
    /// nor the best production city; at its limit it grants nothing. Without a House, the grant
    /// founds one.
    #[test]
    fn the_crown_grants_what_is_over_its_limit_and_the_first_grant_founds() {
        let (g3, _) = world(&SPOTS[..3]);
        assert!(g3.player_crown_over(P0).is_empty(), "at its limit");
        assert!(grantable_cities(&g3, P0).is_empty());
        assert_eq!(choose_grant(&g3, P0), None);
        let (mut g, c) = world(&SPOTS[..6]);
        pops(&mut g, &c, &[6, 5, 4, 3, 2, 1]);
        assert_eq!(g.player_crown_over(P0).len(), 3, "over its limit");
        let kept = kept_city(&g, P0);
        let list = grantable_cities(&g, P0);
        assert!(!list.is_empty() && !list.contains(&c[0]) && kept.is_none_or(|k| !list.contains(&k)), "{list:?}");
        let pop: Vec<i32> = list.iter().map(|&k| g.cities[k].population).collect();
        assert!(pop.windows(2).all(|w| w[0] <= w[1]), "least populous first: {pop:?}");
        assert_eq!(choose_grant(&g, P0), Some((list[0], GrantTo::NewHouse)), "no House: the first grant founds one");
        super::turn(&mut g, P0);
        assert_eq!(g.players[P0].houses.len(), 1, "founded");
        let h = g.players[P0].houses[0];
        assert_eq!(g.house_cities(h).len(), 1);
    }

    /// With Houses, each grant goes to the House whose capital is nearest the city.
    #[test]
    fn a_grant_goes_to_the_nearest_house() {
        let (mut g, c) = world(&SPOTS);
        pops(&mut g, &c[..9], &[9, 1, 8, 7, 3, 8, 8, 8, 8]);
        // A House in the west (6, 4) and one in the east (15, 7).
        let west = house(&mut g, c[1]);
        let east = house(&mut g, c[9]);
        let list = grantable_cities(&g, P0);
        assert_eq!(list.first(), Some(&c[4]), "the least populous Crown city: (15, 4)");
        assert_eq!(nearest_house(&g, &[west, east], c[4]), Some(east));
        assert_eq!(choose_grant(&g, P0), Some((c[4], GrantTo::House(east))));
        // A House only: no founding while one stands.
        assert!(g.player_check_found_house(P0).is_err());
    }

    /// A grant whose preview makes a House Furious is passed over for the next city: the western
    /// House claims (6, 7), so a grant of it to the eastern House would snub it (-1) into Furious.
    #[test]
    fn a_snub_that_makes_a_house_furious_is_avoided() {
        let (mut g, c) = world(&SPOTS);
        let west = house(&mut g, c[1]);
        let east = house(&mut g, c[9]);
        set_loyalty(&mut g, west, -5);
        assert_eq!(g.house_state(west), LoyaltyState::Annoyed);
        // A grant of (6, 7) to the eastern House would snub the western one into Furious; the
        // grant chosen makes no House Furious.
        let preview = g.grant_preview(P0, c[6], GrantTo::House(east));
        assert!(preview.iter().any(|l| l.subject == g.house_label(west) && l.state_after == Some(LoyaltyState::Furious)), "{preview:?}");
        let (k, to) = choose_grant(&g, P0).expect("a grant");
        let lines = g.grant_preview(P0, k, to);
        assert!(lines.iter().all(|l| l.state_after != Some(LoyaltyState::Furious)), "{k:?} {to:?}: {lines:?}");
    }

    // ---- A courted House ----

    /// The liege holds a House whose strongest rival favour is past half its requirement (the
    /// fourth build, rule 9); not one courted less.
    #[test]
    fn a_courted_house_is_held_past_half_its_requirement() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let h = house(&mut g, c[1]);
        assert!(!courted_to_hold(&g, h), "no one courts it");
        let need = 100 * g.house_requirement(h, P1).total;
        g.houses[h].favour = vec![ffh_core::persons::Favour { courter: P1, amount: need / 2, warned: false }];
        assert!(!courted_to_hold(&g, h), "half: not yet");
        g.houses[h].favour[0].amount = need / 2 + 1;
        assert!(courted_to_hold(&g, h), "past half");
    }

    /// A gift to a House is an ordinary gift (the fourth build, rule 3): its leader's relation
    /// toward the giver gains the `gifts` term, and the attitude it raises lowers the requirement
    /// of the giver's courtship.
    #[test]
    fn a_gift_raises_the_house_leaders_attitude() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let h = house(&mut g, c[1]);
        let hp = g.houses[h].player;
        let before = g.house_requirement(h, P1);
        let relation = crate::diplomacy::relation(&g, hp, P1);
        g.players[P1].gold = 5000;
        for _ in 0..3 {
            assert!(g.player_offer_house(P1, hp, &[gold(1000)], &[]), "a gift");
        }
        let parts = crate::diplomacy::relation_parts(&g, hp, P1);
        assert!(parts.iter().any(|&(why, n)| why == "gifts" && n > 0), "{parts:?}");
        assert!(crate::diplomacy::relation(&g, hp, P1) > relation);
        let after = g.house_requirement(h, P1);
        assert!(after.attitude >= before.attitude, "{before:?} {after:?}");
        assert_eq!(after.total, (after.loyalty + 6 - 2 * after.attitude).max(1) * after.unit);
        if after.attitude > before.attitude {
            assert!(after.total < before.total, "a better attitude, a lower requirement");
        }
    }

    /// A House accepts any gift (and nothing else: it trades only gifts).
    #[test]
    fn an_ai_house_accepts_any_gift() {
        let (mut g, c) = world(&SPOTS[..2]);
        g.player_init_city(P1, 15, 7, true, true);
        let h = house(&mut g, c[1]);
        let hp = g.houses[h].player;
        g.players[P1].gold = 100;
        assert!(g.player_offer_house(P1, hp, &[gold(5)], &[]), "a gift");
        assert_eq!((g.players[P1].gold, g.players[hp].gold >= 5), (95, true));
        assert!(!g.player_offer_house(P1, hp, &[gold(5)], &[gold(1)]), "not a gift");
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
