//! Persons of Note (docs/PERSONS.md; docs/AI.md, "Persons of Note"): the AI's minimum. A rules
//! change is not gated on the AI (CLAUDE.md): this is a reasonable first version, not a measured
//! one. AI players only: a human's grants and demands are the human's.
//!
//! The second build (Houses as vassal civilizations, 2026-11-18) changed the rules under this
//! module; it was kept compiling with the first build's choices mapped onto the new scale (Furious
//! for Disloyal, Annoyed for Restless). The AI of the second build (tribute by attitude, levies,
//! grants beyond the six largest cities, bribes) is still to be written (docs/PERSONS.md, rule 12).
//!
//! Once a turn ([`turn`], after the empire's civics and religion):
//! - **Grants** ([`choose_grant`]): one Crown city a turn (never the capital, nor the best
//!   production city: [`kept_city`]), to the candidate by, in order: no holder made Furious; a
//!   standing demand met (an Annoyed or Furious grantee's first); a claimant, the one in the best
//!   state; a person without a post founding a House, then a great person founding the first
//!   House, then the House with the fewest cities; the fewest holders made Annoyed. A grant that
//!   would make some holder Furious waits. No House is enlarged past the Crown: a House whose
//!   cities would outnumber the Crown's after the grant gets no more.
//!
//! Elsewhere: a direct-rule civic is never adopted while a House stands ([`civic_barred`],
//! `empire.rs`); an idle army stands in an unheld city of a Furious House
//! ([`disloyal_house_post`], `military.rs`: the city goes back to the liege when the House breaks
//! away); a Furious commander's army goes to the city of ours with the largest garrison
//! ([`disloyal_commander_haven`]); a demanded war weighs on the war planning ([`war_demanded`]).
//! The AI never frees a House.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::{DemandKind, GrantCandidate, GrantTo, LoyaltyState, Post};
use ffh_data::*;

use crate::view::View;

/// The persons' turn of AI player `p`: one grant.
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.players[p].human || !g.player_has_persons(p) {
        return;
    }
    if let Some((c, to)) = choose_grant(g, p) {
        let city = g.d.text.name(&g.cities[c].name).to_string();
        match g.player_grant_city(p, c, to) {
            Ok(h) => crate::trace!(g, p, "persons: {city} granted to {}", g.house_label(h)),
            Err(e) => crate::trace!(g, p, "persons: no grant of {city}: {e}"),
        }
    }
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

// -------------------------------------------------------------------------------------------
// Grants
// -------------------------------------------------------------------------------------------

/// The city `p` keeps in the Crown besides its capital: its best production city (ties: the
/// older). The AI's empire orders (wonders, settlers, its army) go to its capital and most
/// productive cities (`governor::empire_cities`).
pub(crate) fn kept_city(g: &Game, p: PlayerId) -> Option<CityId> {
    let capital = g.players[p].capital_city;
    g.players[p].cities.iter().copied().filter(|&c| Some(c) != capital && g.cities.contains(c)).max_by_key(|&c| (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(c)))
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

/// The cities of `p` the Crown holds (the capital among them): all of its own.
fn crown_cities(g: &Game, p: PlayerId) -> usize {
    g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).count()
}

/// The ranking key of a grant candidate (larger is better), or `None` when the AI does not
/// consider it. `first_house`: `p` has no House and no person without a post (a great person may
/// then found one).
fn grant_key(g: &Game, p: PlayerId, c: CityId, k: &GrantCandidate, first_house: bool) -> Option<(bool, i32, bool, i32, i32, i32, i32)> {
    if k.refusal.is_some() {
        return None;
    }
    let met = meets_demand(g, c, k.to);
    let kind = match k.to {
        // No House outgrows the Crown: one whose cities would outnumber the Crown's after the
        // grant is not enlarged, a demand or not (a break-away takes its cities).
        GrantTo::House(h) if g.house_cities(h).len() + 1 > crown_cities(g, p).saturating_sub(1) => return None,
        GrantTo::House(_) => 1,
        GrantTo::Person(x) => match g.persons[x].post {
            Post::None => 3,
            // A commander founding a House loses the command: only to meet their demand or
            // their claim.
            _ if met.is_some() || k.claimant => 0,
            _ => return None,
        },
        GrantTo::GreatPerson(u) => {
            if !first_house || !great_person_spare(g, p, u) {
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
    Some((!furious, demand, k.claimant, claimant_rank, kind, fewest, -annoyed))
}

/// The grant `p` makes this turn, if any: see the module comment.
pub(crate) fn choose_grant(g: &Game, p: PlayerId) -> Option<(CityId, GrantTo)> {
    if g.player_check_grants(p).is_err() {
        return None;
    }
    let pl = &g.players[p];
    let has_great = pl.units.iter().any(|&u| g.units.contains(u) && g.unit_is_great_person(u));
    if pl.persons.is_empty() && pl.houses.is_empty() && !has_great {
        return None;
    }
    let kept = kept_city(g, p);
    let first_house = pl.houses.is_empty() && !pl.persons.iter().any(|&x| g.persons.get(x).is_some_and(|person| person.post == Post::None && person.leader.is_some()));
    let mut cities: Vec<CityId> = pl.cities.iter().copied().filter(|&c| g.cities.contains(c) && Some(c) != kept && !g.city_is_capital(c)).collect();
    cities.sort();
    let mut best: Option<((bool, i32, bool, i32, i32, i32, i32), CityId, GrantTo)> = None;
    for c in cities {
        for k in g.grant_candidates(p, c) {
            let Some(key) = grant_key(g, p, c, &k, first_house) else { continue };
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
// Civics
// -------------------------------------------------------------------------------------------

/// Whether civic `c` is barred to `p`: a direct-rule civic while a House stands (the rules would
/// have every House break away).
pub(crate) fn civic_barred(g: &Game, p: PlayerId, c: CivicType) -> bool {
    g.d.civics[c].direct_rule && !g.players[p].houses.is_empty()
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
    use ffh_core::persons::{Ambition, Demand, LoyaltyCause, LoyaltyMod};

    const P0: PlayerId = PlayerId(0);

    /// The one-landmass duel with player 0 under Aristocracy (cities may be granted) holding cities
    /// at `spots`, the first its capital.
    fn world(spots: &[(i32, i32)]) -> (Game, Vec<CityId>) {
        // (The Bannor: a civilization with minor leaders, so veterans enter the cast.)
        let mut g = crate::testkit::one_landmass_game_with(7, &["CIVILIZATION_BANNOR"]);
        let cities: Vec<CityId> = spots.iter().map(|&(x, y)| g.player_init_city(P0, x, y, true, true)).collect();
        let civic = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
        let option = g.d.civics[civic].civic_option_type.expect("a civic option");
        g.player_set_civics(P0, option, Some(civic));
        (g, cities)
    }

    /// A veteran of player 0 entered into the cast (the next game turn's persons turn), of Land
    /// ambition, with no demand.
    fn person(g: &mut Game, x: i32, y: i32) -> PersonId {
        let t = g.d.t::<UnitType>("UNIT_WARRIOR");
        let u = g.player_init_unit(P0, t, x, y, None, None);
        g.units[u].level = 30;
        let turn = g.game_turn();
        g.set_game_turn(turn + 1);
        g.player_persons_turn(P0);
        let x = g.unit_person(u).expect("entered");
        g.persons[x].demand = None;
        g.persons[x].ambition = Ambition::Land;
        x
    }

    /// A House founded with city `seat` by a new person, its head without a demand.
    fn house(g: &mut Game, seat: CityId) -> (PersonId, HouseId) {
        let (x, y) = (g.cities[seat].x, g.cities[seat].y);
        let p = person(g, x, y);
        let h = g.player_grant_city(P0, seat, GrantTo::Person(p)).expect("granted");
        g.persons[p].demand = None;
        (p, h)
    }

    fn grievance(g: &mut Game, h: HouseId, amount: i32) {
        let t = g.game_turn();
        g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount, until: Some(t + 60) });
    }

    fn demand(g: &mut Game, x: PersonId, kind: DemandKind) {
        let t = g.game_turn();
        g.persons[x].demand = Some(Demand { kind, issued: t, deadline: t + 40 });
    }

    /// The best production city stays in the Crown; never the capital.
    #[test]
    fn the_best_production_city_is_kept() {
        let (mut g, cities) = world(&[(3, 4), (7, 4), (11, 4), (7, 7)]);
        let kept = kept_city(&g, P0).expect("a Crown city kept");
        assert_ne!(kept, cities[0]);
        let rate = |g: &Game, c: CityId| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
        assert!(cities[1..].iter().all(|&c| rate(&g, c) <= rate(&g, kept)));
        let _y = person(&mut g, 13, 6);
        let (c, _) = choose_grant(&g, P0).expect("a grant");
        assert!(c != kept && c != cities[0]);
    }

    /// A grant that meets a standing demand comes first: the House whose head demands city C gets
    /// it, ahead of founding a House for a person without a post.
    #[test]
    fn a_grant_meets_a_demand_first() {
        let (mut g, cities) = world(&[(3, 4), (7, 4), (11, 4), (7, 7), (15, 4), (3, 8)]);
        let (a, c) = (cities[1], cities[3]);
        let (x, h) = house(&mut g, a);
        let _y = person(&mut g, 13, 6);
        assert_ne!(kept_city(&g, P0), Some(c));
        let plot = g.city_plot(c);
        demand(&mut g, x, DemandKind::GrantCity(plot));
        assert_eq!(choose_grant(&g, P0), Some((c, GrantTo::House(h))));
        super::turn(&mut g, P0);
        assert_eq!(g.city_house(g.map.plots[plot].plot_city.unwrap()), Some(h));
        assert_eq!(g.persons[x].demand, None, "met");
    }

    /// A grant whose preview turns a holder Furious is not made when another is not: the person
    /// without a post demands one, but every Crown city is claimed by a House that the snub (-1)
    /// would make Furious, so the city goes to the House.
    #[test]
    fn a_snub_that_makes_a_holder_furious_is_avoided() {
        let (mut g, cities) = world(&[(3, 4), (8, 5), (5, 7), (11, 4), (14, 8)]);
        let (_, h) = house(&mut g, cities[1]);
        let loyalty = g.house_loyalty(h);
        grievance(&mut g, h, -5 - loyalty);
        assert_eq!(g.house_state(h), LoyaltyState::Annoyed);
        let y = person(&mut g, 13, 6);
        demand(&mut g, y, DemandKind::Post);
        for &c in &cities[2..4] {
            let k = g.grant_candidates(P0, c).into_iter().find(|k| k.to == GrantTo::Person(y)).unwrap();
            assert!(k.lines.iter().any(|l| l.state_after == Some(LoyaltyState::Furious)), "{:?}", k.lines);
        }
        let (c, to) = choose_grant(&g, P0).expect("a grant");
        assert!(to == GrantTo::House(h) || !cities[2..4].contains(&c), "{c:?} {to:?}");
    }

    /// A direct-rule civic is barred while a House stands, and the AI's civics choice adopts none.
    #[test]
    fn no_direct_rule_civic_with_a_house() {
        let (mut g, cities) = world(&[(3, 4), (7, 4)]);
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
