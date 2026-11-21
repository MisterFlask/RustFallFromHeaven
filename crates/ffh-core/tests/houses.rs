//! Houses (docs/HOUSES.md): membership, the pick, opinion, happiness, perks, previews and saves.
//! Every situation is built by hand on a game whose turns are not played; nothing asserted
//! depends on the seed's story.

use std::sync::{Arc, Mutex};

use ffh_core::controller::Controller;
use ffh_core::game::Game;
use ffh_core::houses::*;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// Records every offer and picks `pick` (else the default pick).
#[derive(Default)]
struct Recorder {
    offers: Mutex<Vec<HouseOffer>>,
    pick: Option<usize>,
}

impl Controller for Recorder {
    fn choose_house_leader(&self, g: &Game, _p: PlayerId, offer: &HouseOffer) -> usize {
        self.offers.lock().unwrap().push(offer.clone());
        self.pick.unwrap_or_else(|| house_pick_default(g.d, offer))
    }
}

/// A small map after `game_init`: the Bannor (0, Good), the Elohim (1, Good) and the Sheaim (2,
/// Evil), every starting unit gone, no city yet; player 0's controller records its offers.
fn game() -> (Game, Arc<Recorder>) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 3, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[2].civ = Some(d.t::<CivilizationType>("CIVILIZATION_SHEAIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    setup.players[2].leader = Some(d.t::<LeaderHeadType>("LEADER_TEBRYN"));
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for p in [P0, P1, P2] {
        for u in g.players[p].units.clone() {
            g.unit_kill(u, false, None);
        }
    }
    g.players[P0].alignment = Some(AlignmentType::ALIGNMENT_GOOD);
    g.players[P1].alignment = Some(AlignmentType::ALIGNMENT_GOOD);
    g.players[P2].alignment = Some(AlignmentType::ALIGNMENT_EVIL);
    let rec = Arc::new(Recorder::default());
    g.controllers[P0] = rec.clone();
    (g, rec)
}

fn land(g: &Game, q: PlotIdx) -> bool {
    !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].plot_city.is_none()
}

/// A free land plot at distance `lo..=hi` from `(x, y)` and at least 3 from every city, that
/// `ok` accepts.
fn plot_at(g: &Game, (x, y): (i32, i32), lo: i32, hi: i32, ok: impl Fn(PlotIdx) -> bool) -> (i32, i32) {
    g.map
        .plot_ids()
        .filter(|&q| land(g, q) && ok(q))
        .map(|q| g.map.xy(q))
        .find(|&(a, b)| {
            let dist = g.plot_distance(x, y, a, b);
            dist >= lo && dist <= hi && g.cities.iter().all(|(_, c)| g.plot_distance(a, b, c.x, c.y) >= 3)
        })
        .expect("a free plot")
}

/// A first city of player `p`, at least 12 plots from every city.
fn first_city(g: &mut Game, p: PlayerId) -> CityId {
    let (x, y) = g
        .map
        .plot_ids()
        .filter(|&q| land(g, q))
        .map(|q| g.map.xy(q))
        .find(|&(a, b)| g.cities.iter().all(|(_, c)| g.plot_distance(a, b, c.x, c.y) >= 12))
        .expect("a free plot far from every city");
    g.player_init_city(p, x, y, true, true)
}

/// A city of `p` at distance `lo..=hi` from city `near`.
fn city_near(g: &mut Game, p: PlayerId, near: CityId, lo: i32, hi: i32) -> CityId {
    let at = (g.cities[near].x, g.cities[near].y);
    let (x, y) = plot_at(g, at, lo, hi, |_| true);
    g.player_init_city(p, x, y, true, true)
}

/// Player 0's capital with `n - 1` more cities within 4 plots of it, and the Houses pass.
fn home(g: &mut Game, n: usize) -> Vec<CityId> {
    let capital = first_city(g, P0);
    let mut cities = vec![capital];
    while cities.len() < n {
        cities.push(city_near(g, P0, capital, 3, 4));
    }
    g.player_houses_turn(P0);
    cities
}

fn messages(g: &Game, p: PlayerId) -> Vec<String> {
    g.log.entries.iter().filter(|e| e.player == Some(p.0)).map(|e| e.text.clone()).collect()
}

/// Every check of the stress harness holds.
fn check(g: &Game) {
    ffh_harness_check(g);
}

/// The membership part of the harness's checks, restated (ffh-core's tests cannot use the
/// harness crate).
fn ffh_harness_check(g: &Game) {
    for (h, house) in g.houses.iter() {
        assert!(!house.cities.is_empty() && house.cities.len() <= g.d.defines.house_max_cities as usize, "{h:?}");
        for &c in &house.cities {
            assert_eq!(g.cities[c].house, Some(h));
            assert_eq!(g.cities[c].owner, house.owner);
            assert_eq!(g.city_origin(c), Some(house.origin));
        }
        assert!(g.players[house.owner].houses.contains(&h));
    }
    for (c, city) in g.cities.iter() {
        if let Some(h) = city.house {
            assert!(g.houses[h].cities.contains(&c));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------------------------

#[test]
fn no_house_at_two_cities_the_first_at_three_with_the_capital_as_seat() {
    let (mut g, rec) = game();
    let cities = home(&mut g, 2);
    assert!(g.houses.is_empty());
    let unhoused = g.player_unhoused_cities(P0);
    assert_eq!(unhoused.len(), 2);
    assert_eq!(unhoused[0].reason, "No House yet: the first forms at 3 cities (2 of 3).");
    let third = city_near(&mut g, P0, cities[0], 3, 4);
    assert!(g.houses.is_empty(), "a city gained only joins");
    g.player_houses_turn(P0);
    assert_eq!(g.players[P0].houses.len(), 1);
    let h = g.players[P0].houses[0];
    let house = &g.houses[h];
    assert_eq!(house.cities[0], cities[0], "the capital is the seat");
    assert_eq!(house.cities.len(), 3);
    assert!(house.cities.contains(&third));
    assert!(!house.outsider);
    assert_eq!(rec.offers.lock().unwrap().len(), 1);
    let offer = rec.offers.lock().unwrap()[0].clone();
    assert!(!offer.foreign);
    assert_eq!(offer.cards.len(), 3);
    let leader = leader_name(g.d, house.leader);
    let names: Vec<String> = house.cities.iter().map(|&c| g.d.text.name(&g.cities[c].name).to_string()).collect();
    let text = format!("{} forms: {leader} leads {}, {} and {}.", g.house_label(h), names[0], names[1], names[2]);
    assert!(messages(&g, P0).contains(&text), "{:?}", messages(&g, P0));
    assert!(g.player_unhoused_cities(P0).is_empty());
    check(&g);
}

#[test]
fn the_fifth_city_in_range_of_a_full_house_founds_a_second() {
    let (mut g, _) = game();
    let cities = home(&mut g, 4);
    assert_eq!(g.players[P0].houses.len(), 1);
    let first = g.players[P0].houses[0];
    assert_eq!(g.houses[first].cities.len(), 4);
    let fifth = city_near(&mut g, P0, cities[0], 3, 5);
    assert_eq!(g.cities[fifth].house, None);
    g.player_houses_turn(P0);
    assert_eq!(g.players[P0].houses.len(), 2);
    let second = g.players[P0].houses[1];
    assert_eq!(g.houses[second].cities, vec![fifth]);
    assert_ne!(g.houses[first].leader, g.houses[second].leader);
    assert_ne!(g.houses[first].name, g.houses[second].name);
    check(&g);
}

#[test]
fn a_city_out_of_range_founds_a_house_at_once() {
    let (mut g, _) = game();
    let cities = home(&mut g, 3);
    let first = g.players[P0].houses[0];
    let range = g.d.defines.house_join_range;
    let far = city_near(&mut g, P0, cities[0], range + 1, range + 6);
    g.player_houses_turn(P0);
    assert_eq!(g.players[P0].houses.len(), 2);
    assert_eq!(g.houses[g.players[P0].houses[1]].cities, vec![far]);
    assert_eq!(g.houses[first].cities.len(), 3);
    check(&g);
}

#[test]
fn a_city_in_range_joins_at_once() {
    let (mut g, _) = game();
    let cities = home(&mut g, 3);
    let h = g.players[P0].houses[0];
    let near = city_near(&mut g, P0, cities[0], 3, 5);
    assert_eq!(g.cities[near].house, Some(h));
    let name = g.d.text.name(&g.cities[near].name).to_string();
    assert!(messages(&g, P0).contains(&format!("{name} joins {}.", g.house_label(h))));
    check(&g);
}

#[test]
fn a_captured_foreign_city_founds_a_foreign_house_with_the_outsider_and_a_second_joins_it() {
    let (mut g, rec) = game();
    let capital = first_city(&mut g, P0);
    let theirs = first_city(&mut g, P1);
    let second = city_near(&mut g, P1, theirs, 3, 4);
    // Big enough not to be razed when taken.
    g.city_set_population(theirs, 4);
    g.city_set_population(second, 4);
    let plot = g.city_plot(theirs);
    g.player_acquire_city(P0, theirs, true, false, true);
    let taken = g.map.plots[plot].plot_city.expect("kept");
    assert_eq!(g.cities[taken].owner, P0);
    assert_eq!(g.city_origin(taken), g.player_get_civilization_type(P1));
    assert!(!g.city_is_home(taken));
    g.player_houses_turn(P0);
    let h = g.cities[taken].house.expect("a foreign House at once");
    assert!(g.house_is_foreign(h));
    assert_eq!(g.houses[h].origin, g.player_get_civilization_type(P1).unwrap());
    let offer = rec.offers.lock().unwrap().last().cloned().unwrap();
    assert!(offer.foreign);
    assert_eq!(offer.cards.len(), 4, "three and the outsider");
    let outsider = offer.cards.last().unwrap();
    assert!(outsider.candidate.outsider);
    assert_eq!(outsider.candidate.sympathy, None);
    assert_eq!(g.d.leader_heads[outsider.candidate.leader].minor_leader_of, g.player_get_civilization_type(P0));
    assert!(offer.cards[..3].iter().all(|c| !c.candidate.outsider));
    // The capital alone waits for the first home House.
    assert_eq!(g.cities[capital].house, None);
    // A second city of that origin in range joins it at once.
    let plot2 = g.city_plot(second);
    g.player_acquire_city(P0, second, true, false, true);
    let taken2 = g.map.plots[plot2].plot_city.unwrap();
    assert_eq!(g.cities[taken2].house, Some(h));
    check(&g);
}

#[test]
fn a_recaptured_home_city_joins_a_home_house() {
    let (mut g, _) = game();
    let cities = home(&mut g, 3);
    let h = g.players[P0].houses[0];
    let lost = cities[2];
    g.city_set_population(lost, 4);
    let plot = g.city_plot(lost);
    g.player_acquire_city(P1, lost, true, false, true);
    assert_eq!(g.houses[h].cities.len(), 2);
    let theirs = g.map.plots[plot].plot_city.unwrap();
    assert_eq!(g.cities[theirs].owner, P1);
    g.player_acquire_city(P0, theirs, true, false, true);
    let back = g.map.plots[plot].plot_city.unwrap();
    assert!(g.city_is_home(back));
    assert_eq!(g.cities[back].house, Some(h));
    check(&g);
}

#[test]
fn losing_the_seat_moves_the_seat_and_losing_the_last_city_ends_the_house() {
    let (mut g, _) = game();
    let cities = home(&mut g, 3);
    let h = g.players[P0].houses[0];
    let next = g.houses[h].cities[1];
    g.player_acquire_city(P1, cities[0], true, false, true);
    assert_eq!(g.house_seat(h), Some(next));
    // A House of one city: the far one.
    let range = g.d.defines.house_join_range;
    let far = city_near(&mut g, P0, next, range + 1, range + 6);
    g.player_houses_turn(P0);
    let single = g.cities[far].house.unwrap();
    let leader = g.houses[single].leader;
    assert!(!g.house_leader_is_free(leader));
    let label = g.house_label(single);
    g.city_kill(far, true);
    assert!(!g.houses.contains(single));
    assert!(!g.players[P0].houses.contains(&single));
    assert!(g.house_leader_is_free(leader));
    assert!(messages(&g, P0).contains(&format!("{label} ends: it holds no city.")));
    check(&g);
}

// ---------------------------------------------------------------------------------------------
// Opinion and happiness
// ---------------------------------------------------------------------------------------------

/// A home House of player 0 of three cities with every part of its opinion neutralised: Good
/// like its ruler (+1), no religion, no sympathy; player 1 and 2 hold a city each.
fn plain_house() -> (Game, HouseId) {
    let (mut g, _) = game();
    home(&mut g, 3);
    first_city(&mut g, P1);
    first_city(&mut g, P2);
    let h = g.players[P0].houses[0];
    let house = &mut g.houses[h];
    house.alignment = Some(AlignmentType::ALIGNMENT_GOOD);
    house.religion = None;
    house.sympathy = None;
    house.outsider = false;
    (g, h)
}

fn war(g: &mut Game, a: PlayerId, b: PlayerId, on: bool) {
    let (ta, tb) = (g.players[a].team, g.players[b].team);
    g.teams[ta].at_war[tb] = on;
    g.teams[tb].at_war[ta] = on;
}

fn state_religion(g: &mut Game, p: PlayerId, r: Option<ReligionType>) {
    g.players[p].last_state_religion = r;
    g.players[p].state_religion_count = r.is_some() as i32;
}

#[test]
fn each_opinion_part_alone() {
    let (mut g, h) = plain_house();
    let x = g.d.defines.clone();
    assert_eq!(g.house_opinion_split(h), OpinionParts { alignment: x.house_opinion_alignment_same, ..Default::default() });
    // (a) a step, two steps.
    g.houses[h].alignment = Some(AlignmentType::ALIGNMENT_NEUTRAL);
    assert_eq!(g.house_opinion_split(h).alignment, x.house_opinion_alignment_step);
    g.houses[h].alignment = Some(AlignmentType::ALIGNMENT_EVIL);
    assert_eq!(g.house_opinion_split(h).alignment, 2 * x.house_opinion_alignment_step);
    g.houses[h].alignment = Some(AlignmentType::ALIGNMENT_GOOD);
    // (b) the same religion, another, none of the owner's.
    let r0 = ReligionType::from_usize(0);
    let r1 = ReligionType::from_usize(1);
    g.houses[h].religion = Some(r0);
    assert_eq!(g.house_opinion_split(h).religion, 0, "no state religion");
    state_religion(&mut g, P0, Some(r0));
    assert_eq!(g.house_opinion_split(h).religion, x.house_opinion_religion_same);
    state_religion(&mut g, P0, Some(r1));
    assert_eq!(g.house_opinion_split(h).religion, x.house_opinion_religion_other);
    g.houses[h].religion = None;
    state_religion(&mut g, P0, None);
    // (c) the sympathy: war, a pact, a vassal, open borders, nothing.
    g.houses[h].sympathy = Some(P1);
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    assert_eq!(g.house_opinion_split(h).sympathy, 0);
    g.teams[t0].open_borders[t1] = true;
    g.teams[t1].open_borders[t0] = true;
    assert_eq!(g.house_opinion_split(h).sympathy, x.house_opinion_sympathy_open_borders);
    g.teams[t0].defensive_pact[t1] = true;
    g.teams[t1].defensive_pact[t0] = true;
    assert_eq!(g.house_opinion_split(h).sympathy, x.house_opinion_sympathy_pact);
    g.teams[t0].defensive_pact[t1] = false;
    g.teams[t1].defensive_pact[t0] = false;
    g.teams[t1].vassal[t0] = true;
    assert_eq!(g.house_opinion_split(h).sympathy, x.house_opinion_sympathy_pact);
    g.teams[t1].vassal[t0] = false;
    g.teams[t0].open_borders[t1] = false;
    g.teams[t1].open_borders[t0] = false;
    war(&mut g, P0, P1, true);
    let parts = g.house_opinion_split(h);
    assert_eq!(parts.sympathy, x.house_opinion_sympathy_war);
    assert_eq!(parts.wars, 0, "the favoured enemy is left out of (d)");
    war(&mut g, P0, P1, false);
    g.houses[h].sympathy = None;
    // (d) a war on a civilization of the House's alignment, and on one two steps from it.
    war(&mut g, P0, P1, true);
    assert_eq!(g.house_opinion_split(h).wars, x.house_opinion_war_same_alignment);
    war(&mut g, P0, P1, false);
    war(&mut g, P0, P2, true);
    assert_eq!(g.house_opinion_split(h).wars, x.house_opinion_war_opposed_alignment);
    war(&mut g, P0, P1, true);
    assert_eq!(g.house_opinion_split(h).wars, x.house_opinion_war_same_alignment + x.house_opinion_war_opposed_alignment);
    assert_eq!(g.house_opinion(h), g.house_opinion_split(h).total());
    // The labelled parts sum to the opinion.
    let parts = g.house_opinion_parts(h);
    assert_eq!(parts.iter().map(|p| p.value).sum::<i32>(), g.house_opinion(h));
    assert!(g.house_opinion_calc(h).iter().any(|l| l.starts_with("Opinion: ")));
}

#[test]
fn the_state_thresholds_at_their_edges() {
    let d = ffh_data::global();
    let cases = [(3, HouseState::Friendly), (2, HouseState::Pleased), (1, HouseState::Pleased), (0, HouseState::Cautious), (-2, HouseState::Cautious), (-3, HouseState::Annoyed), (-5, HouseState::Annoyed), (-6, HouseState::Furious)];
    for (n, s) in cases {
        assert_eq!(HouseState::of(d, n), s, "{n}");
    }
    let happy: Vec<i32> = HouseState::ALL.iter().map(|s| s.happiness(d)).collect();
    assert_eq!(happy, vec![2, 1, 0, -2, -4]);
}

/// The House's opinion set to `state` through its alignment and its sympathy at war.
fn set_state(g: &mut Game, h: HouseId, state: HouseState) {
    g.houses[h].sympathy = None;
    war(g, P0, P1, false);
    let (a, sympathy_war) = match state {
        HouseState::Pleased => (AlignmentType::ALIGNMENT_GOOD, false),   // +1
        HouseState::Cautious => (AlignmentType::ALIGNMENT_NEUTRAL, false), // -2
        HouseState::Annoyed => (AlignmentType::ALIGNMENT_EVIL, false),  // -4
        HouseState::Furious => (AlignmentType::ALIGNMENT_EVIL, true),   // -4 - 3
        HouseState::Friendly => unreachable!(),
    };
    g.houses[h].alignment = Some(a);
    if sympathy_war {
        g.houses[h].sympathy = Some(P1);
        war(g, P0, P1, true);
    }
    assert_eq!(g.house_state(h), state);
}

#[test]
fn happiness_by_state_in_the_city() {
    let (mut g, h) = plain_house();
    let c = g.houses[h].cities[0];
    for state in [HouseState::Pleased, HouseState::Cautious, HouseState::Annoyed, HouseState::Furious] {
        set_state(&mut g, h, state);
        let with = (g.city_happy_level(c), g.city_unhappy_level(c, 0));
        g.cities[c].house = None;
        let without = (g.city_happy_level(c), g.city_unhappy_level(c, 0));
        g.cities[c].house = Some(h);
        let n = state.happiness(g.d);
        assert_eq!(g.city_house_happiness(c), n);
        assert_eq!(with.0 - without.0, n.max(0), "{state:?}");
        if !g.city_is_no_unhappiness(c) {
            assert_eq!(with.1 - without.1, (-n).max(0), "{state:?}");
        }
        let parts = g.city_house_happiness_parts(c);
        if n != 0 {
            assert_eq!(parts, vec![(format!("{} is {}", g.house_label(h), state.name()), n)]);
        } else {
            assert!(parts.is_empty());
        }
    }
}

#[test]
fn the_outsider_has_minus_three_and_no_parts_c_or_d() {
    let (mut g, h) = plain_house();
    g.houses[h].outsider = true;
    g.houses[h].sympathy = Some(P1);
    war(&mut g, P0, P1, true);
    war(&mut g, P0, P2, true);
    let parts = g.house_opinion_split(h);
    assert_eq!((parts.sympathy, parts.wars), (0, 0));
    assert_eq!(parts.alignment, g.d.defines.house_opinion_alignment_same);
    let c = g.houses[h].cities[0];
    let (state, outsider) = g.city_house_happiness_terms(c);
    assert_eq!(outsider, g.d.defines.house_outsider_happy);
    assert_eq!(state, HouseState::Pleased.happiness(g.d));
    assert!(g.city_house_happiness_parts(c).contains(&("Ruled by an outsider".to_string(), -3)));
    assert_eq!(g.house_opinion_parts(h).len(), 2);
}

#[test]
fn the_perk_is_off_while_furious() {
    let (mut g, h) = plain_house();
    let c = g.houses[h].cities[0];
    set_state(&mut g, h, HouseState::Annoyed);
    assert_eq!(g.city_house_perk(c), Some(g.houses[h].perk));
    set_state(&mut g, h, HouseState::Furious);
    assert_eq!(g.city_house_perk(c), None);
    assert!(!g.house_perk_on(h));
    // A city in no House has no perk.
    let other = first_city(&mut g, P1);
    assert_eq!(g.city_house_perk(other), None);
    // The change is told in the owner's turn.
    g.player_houses_turn(P0);
    assert!(messages(&g, P0).contains(&format!("{} is now Furious: its perk is off.", g.house_label(h))));
    set_state(&mut g, h, HouseState::Annoyed);
    g.player_houses_turn(P0);
    assert!(messages(&g, P0).contains(&format!("{} is now Annoyed.", g.house_label(h))));
}

// ---------------------------------------------------------------------------------------------
// The pick and its rolls
// ---------------------------------------------------------------------------------------------

#[test]
fn the_tier_by_turn_at_normal_speed() {
    let (mut g, _) = game();
    assert_eq!(g.d.game_speeds[g.game_speed_type()].growth_percent, 100, "Normal speed");
    for (turn, tier) in [(0, 1), (99, 1), (100, 2), (199, 2), (200, 3), (300, 3)] {
        g.set_game_turn(turn);
        assert_eq!(g.house_tier_now(), tier, "turn {turn}");
    }
}

#[test]
fn candidates_have_different_families_and_admitted_religions() {
    let (mut g, _) = game();
    let c = first_city(&mut g, P0);
    let civ = g.player_get_civilization_type(P0).unwrap();
    for tier in 1..=3 {
        for _ in 0..20 {
            let cands = g.house_roll_candidates(P0, civ, &[c], tier);
            assert_eq!(cands.len(), 3);
            let mut families: Vec<&str> = cands.iter().map(|k| g.d.traits[k.perk].house_perk_family.as_str()).collect();
            assert!(cands.iter().all(|k| g.d.traits[k.perk].house_perk_tier == tier));
            families.sort();
            families.dedup();
            assert_eq!(families.len(), 3);
            let mut leaders: Vec<LeaderHeadType> = cands.iter().map(|k| k.leader).collect();
            leaders.sort();
            leaders.dedup();
            assert_eq!(leaders.len(), 3);
            for k in &cands {
                assert_eq!(g.d.leader_heads[k.leader].minor_leader_of, Some(civ));
                assert!(k.religion.map_or(true, |r| religion_admits(g.d, r, k.alignment)), "{:?}", k.religion);
                assert_ne!(k.sympathy, Some(P0));
            }
        }
    }
}

#[test]
fn religion_admits_only_within_the_alignment_range() {
    let d = ffh_data::global();
    for r in d.religions.keys() {
        let info = &d.religions[r];
        for a in [AlignmentType::ALIGNMENT_GOOD, AlignmentType::ALIGNMENT_NEUTRAL, AlignmentType::ALIGNMENT_EVIL] {
            let inside = info.alignment_best.map_or(true, |b| a >= b) && info.alignment_worst.map_or(true, |w| a <= w);
            assert_eq!(religion_admits(d, r, Some(a)), inside, "{}", d.type_name(r));
        }
    }
    // Some religion is closed to some alignment (the range means something).
    assert!(d.religions.keys().any(|r| !religion_admits(d, r, Some(AlignmentType::ALIGNMENT_GOOD)) || !religion_admits(d, r, Some(AlignmentType::ALIGNMENT_EVIL))));
}

#[test]
fn the_coastal_family_only_with_a_coastal_city() {
    let (mut g, _) = game();
    let w = g.d.defines.min_water_size_for_ocean;
    // The map is land: its top row becomes a sea.
    let coast_terrain = g.d.t::<TerrainType>("TERRAIN_COAST");
    for q in g.map.plot_ids() {
        if g.map.xy(q).1 == 0 {
            g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, false);
            g.plot_set_terrain_type(q, coast_terrain, false);
        }
    }
    g.map_recalculate_areas();
    let at = g.map.plot_ids().find(|&q| land(&g, q) && !g.plot_is_coastal_land(q, w)).map(|q| g.map.xy(q)).unwrap();
    let inland = g.player_init_city(P0, at.0, at.1, true, true);
    let coast = g.map.plot_ids().find(|&q| land(&g, q) && g.plot_is_coastal_land(q, w) && g.cities.iter().all(|(_, c)| g.plot_distance(g.map.xy(q).0, g.map.xy(q).1, c.x, c.y) >= 3)).map(|q| g.map.xy(q)).unwrap();
    let coastal = g.player_init_city(P0, coast.0, coast.1, true, true);
    assert!(!g.city_is_coastal(inland, w) && g.city_is_coastal(coastal, w));
    for tier in 1..=3 {
        let without = g.house_perk_options(tier, &[inland]);
        assert!(without.iter().all(|&t| !g.d.traits[t].house_requires_coastal));
        assert_eq!(without.len(), 13);
        let with = g.house_perk_options(tier, &[inland, coastal]);
        assert_eq!(with.len(), 14);
        assert!(with.iter().any(|&t| g.d.traits[t].house_requires_coastal));
    }
}

#[test]
fn the_sympathy_weights() {
    let (mut g, _) = game();
    let c = first_city(&mut g, P0);
    first_city(&mut g, P1);
    let x = g.d.defines.clone();
    let plots: Vec<PlotIdx> = (0..g.city_num_city_plots(c) as usize).filter_map(|i| g.city_index_plot(c, i)).collect();
    let set = |g: &mut Game, own: i32, rival: i32| {
        for &q in &plots {
            for p in g.player_ids() {
                g.map.plots[q].culture[p] = 0;
            }
            g.map.plots[q].culture[P0] = own;
            g.map.plots[q].culture[P1] = rival;
        }
    };
    // Only a rival's culture there.
    set(&mut g, 0, 100);
    assert_eq!(g.house_sympathy_weights(P0, &[P1], &[c]), vec![x.house_sympathy_none_weight as i64, (x.house_sympathy_weight + x.house_sympathy_culture_weight) as i64]);
    // Only the owner's.
    set(&mut g, 100, 0);
    assert_eq!(g.house_sympathy_weights(P0, &[P1], &[c]), vec![(x.house_sympathy_none_weight + x.house_sympathy_culture_weight) as i64, x.house_sympathy_weight as i64]);
    // None.
    set(&mut g, 0, 0);
    assert_eq!(g.house_sympathy_weights(P0, &[P1], &[c]), vec![x.house_sympathy_none_weight as i64, x.house_sympathy_weight as i64]);
    // Half and half.
    set(&mut g, 50, 50);
    assert_eq!(g.house_sympathy_weights(P0, &[P1], &[c]), vec![(x.house_sympathy_none_weight + 50) as i64, (x.house_sympathy_weight + 50) as i64]);
    // The candidates: met, another team, not two steps from the leader.
    assert!(g.house_sympathy_candidates(P0, Some(AlignmentType::ALIGNMENT_GOOD)).is_empty(), "not met yet");
    let (t0, t1, t2) = (g.players[P0].team, g.players[P1].team, g.players[P2].team);
    for t in [t1, t2] {
        g.teams[t0].has_met[t] = true;
        g.teams[t].has_met[t0] = true;
    }
    assert_eq!(g.house_sympathy_candidates(P0, Some(AlignmentType::ALIGNMENT_GOOD)), vec![P1], "the Sheaim are two steps away");
    assert_eq!(g.house_sympathy_candidates(P0, Some(AlignmentType::ALIGNMENT_NEUTRAL)), vec![P1, P2]);
    assert_eq!(g.house_sympathy_candidates(P0, Some(AlignmentType::ALIGNMENT_EVIL)), vec![P2]);
}

fn card(state: HouseState, happiness: i32) -> HouseCard {
    let d = ffh_data::global();
    HouseCard {
        candidate: HouseCandidate {
            leader: LeaderHeadType::from_usize(0),
            alignment: None,
            religion: None,
            sympathy: None,
            perk: d.house_perks(1)[0],
            outsider: false,
        },
        name: String::new(),
        epithet: None,
        opinion: OpinionParts::default(),
        state,
        happiness,
        lines: Vec::new(),
        perk_lines: Vec::new(),
    }
}

#[test]
fn the_default_pick_scores_perk_and_happiness() {
    let d = ffh_data::global();
    assert_eq!(house_pick_score(d, &card(HouseState::Pleased, 1)), 90);
    assert_eq!(house_pick_score(d, &card(HouseState::Cautious, 0)), 60);
    assert_eq!(house_pick_score(d, &card(HouseState::Furious, -4)), -120);
    assert_eq!(house_pick_score(d, &card(HouseState::Pleased, -2)), 0, "an outsider, Pleased");
    let offer = |cards: Vec<HouseCard>| HouseOffer { owner: P0, name: String::new(), origin: CivilizationType::from_usize(0), foreign: false, cities: Vec::new(), tier: 1, culture: Vec::new(), cards };
    assert_eq!(house_pick_default(d, &offer(vec![card(HouseState::Cautious, 0), card(HouseState::Pleased, 1), card(HouseState::Furious, -4)])), 1);
    assert_eq!(house_pick_default(d, &offer(vec![card(HouseState::Cautious, 0), card(HouseState::Cautious, 0)])), 0, "a tie to the first");
    assert_eq!(house_pick_default(d, &offer(vec![card(HouseState::Furious, -4), card(HouseState::Annoyed, -2)])), 1);
}

#[test]
fn a_card_shows_its_lines() {
    let (mut g, rec) = game();
    home(&mut g, 3);
    let offer = rec.offers.lock().unwrap()[0].clone();
    for card in &offer.cards {
        assert!(card.lines[0].starts_with("Alignment: "));
        assert!(card.lines[1].starts_with("Religion: "));
        assert!(card.lines[2].starts_with("Sympathy: "));
        assert!(card.lines[3].starts_with("Perk: "));
        assert_eq!(card.lines[4], format!("Opinion now: {} ({})", card.state.name(), card.opinion.total()));
        assert_eq!(card.lines.last().unwrap(), &format!("Happiness: {} in 3 cities.", if card.happiness > 0 { format!("+{}", card.happiness) } else { card.happiness.to_string() }));
        assert!(!card.perk_lines.is_empty());
    }
    assert!(offer.head(&g)[0].starts_with(&format!("House {} forms with ", offer.name)));
}

// ---------------------------------------------------------------------------------------------
// Previews
// ---------------------------------------------------------------------------------------------

#[test]
fn previews_of_a_war_and_a_religion_change() {
    let (mut g, h) = plain_house();
    // Pleased (+1 alignment), favouring the Elohim: a war on them makes it +1 - 3 = -2.
    g.houses[h].sympathy = Some(P1);
    assert_eq!(g.house_state(h), HouseState::Pleased);
    let t1 = g.players[P1].team;
    let before = g.house_opinion(h);
    let preview = g.house_preview(P0, HouseAct::DeclareWar(t1));
    assert_eq!(preview.len(), 1);
    let p = &preview[0];
    assert_eq!((p.house, p.before, p.after), (h, HouseState::Pleased, HouseState::Cautious));
    assert_eq!((p.opinion_before, p.opinion_after), (before, before - 3));
    assert_eq!(p.happiness_after - p.happiness_before, -1);
    assert_eq!(p.lines, vec![format!("{}: Pleased to Cautious. Happiness -1 in 3 cities.", g.house_label(h))]);
    assert_eq!(g.house_opinion(h), before, "a preview changes nothing");
    // Peace with a civilization not at war: nothing changes.
    assert!(g.house_preview(P0, HouseAct::MakePeace(t1)).is_empty());
    // A religion: the House's own adopted, then dropped.
    let r = (0..g.d.religions.len()).map(ReligionType::from_usize).find(|&r| religion_admits(g.d, r, Some(AlignmentType::ALIGNMENT_GOOD))).unwrap();
    g.houses[h].religion = Some(r);
    g.houses[h].sympathy = None;
    assert_eq!(g.house_state(h), HouseState::Pleased);
    // Its religion adopted (it admits the owner's Good: the alignment stays): +1 + 2.
    let preview = g.house_preview(P0, HouseAct::Convert(Some(r)));
    assert_eq!(preview.len(), 1);
    assert_eq!((preview[0].before, preview[0].after), (HouseState::Pleased, HouseState::Friendly));
    assert_eq!(preview[0].lines, vec![format!("{}: Pleased to Friendly. Happiness +1 in 3 cities.", g.house_label(h))]);
    // Another religion adopted: +1 - 1 = 0.
    let other = (0..g.d.religions.len()).map(ReligionType::from_usize).find(|&o| o != r && religion_admits(g.d, o, Some(AlignmentType::ALIGNMENT_GOOD)));
    if let Some(o) = other {
        let preview = g.house_preview(P0, HouseAct::Convert(Some(o)));
        assert_eq!((preview[0].before, preview[0].after), (HouseState::Pleased, HouseState::Cautious));
    }
    // A Furious House's perk.
    set_state(&mut g, h, HouseState::Annoyed);
    g.houses[h].religion = None;
    g.houses[h].sympathy = Some(P1);
    war(&mut g, P0, P1, false);
    let preview = g.house_preview(P0, HouseAct::DeclareWar(t1));
    assert_eq!(preview[0].after, HouseState::Furious);
    assert_eq!(preview[0].lines.last().unwrap(), "Its perk goes off.");
}

// ---------------------------------------------------------------------------------------------
// Saves
// ---------------------------------------------------------------------------------------------

#[test]
fn save_and_load_round_trip_a_game_with_houses() {
    let (mut g, _) = game();
    let cities = home(&mut g, 4);
    let range = g.d.defines.house_join_range;
    city_near(&mut g, P0, cities[0], range + 1, range + 6);
    g.player_houses_turn(P0);
    assert_eq!(g.houses.len(), 2);
    let bytes = g.save();
    let back = Game::load(g.d, &bytes).expect("load");
    assert_eq!(back.save(), bytes);
    assert_eq!(back.calculate_sync_checksum(), g.calculate_sync_checksum());
    assert_eq!(back.players[P0].houses, g.players[P0].houses);
    for (h, house) in g.houses.iter() {
        assert_eq!(&back.houses[h], house);
    }
}

