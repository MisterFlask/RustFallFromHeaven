//! `ffh-play stage persons|persons-direct` (docs/PERSONS.md, "UI", part K): the staged scenes load,
//! the rules' invariants hold on them (`ffh_harness::check`), and each shows what the stage
//! promises: the four Houses' states, the commanders, the empty seat and its candidates, the taken
//! city's claimants, the held city, the wonder in production and a great person that can found a
//! House; `persons-direct` can adopt God King at once.

mod common;

use std::sync::Arc;

use ffh_core::persons::{Claimant, DemandKind, GrantTo, LoyaltyState, Post};
use ffh_data::*;
use ffh_play::json::Json;

#[test]
fn the_staged_persons_scenes_hold_every_state() {
    let d = ffh_data::global();
    for direct in [false, true] {
        let dir = common::TempDir::new(if direct { "stage-persons-direct" } else { "stage-persons" });
        let st = ffh_play::stage::persons(&dir.0, direct).expect("staged");
        assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some(st.stage));
        let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
        let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
        let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
        let g = &s.g;
        common::check::check(g);
        let h = st.human;
        assert_eq!(s.human, h);
        let house = |role: &str| st.houses.iter().find(|x| x.role == role).unwrap().id;
        let person = |role: &str| st.persons.iter().find(|x| x.0 == role).unwrap().1;
        let army = |role: &str| st.armies.iter().find(|x| x.0 == role).unwrap().1;

        // The government, the capital and the three Crown cities.
        assert_eq!(g.players[h].civics[d.civics[d.t::<CivicType>("CIVIC_ARISTOCRACY")].civic_option_type.unwrap()], Some(d.t::<CivicType>("CIVIC_ARISTOCRACY")));
        assert!(g.city_is_capital(st.capital));
        assert_eq!(st.crown.len(), 3);
        for &c in &st.crown {
            assert_eq!(g.cities[c].owner, h);
            assert_eq!(g.cities[c].house, None);
        }

        // The Houses.
        assert_eq!(g.house_state(house("A")), LoyaltyState::Devoted);
        assert_eq!(g.houses[house("A")].cities.len(), 3);
        assert_eq!(g.house_state(house("B")), LoyaltyState::Restless);
        assert_eq!(g.houses[house("B")].cities.len(), 2);
        assert!(g.houses[house("B")].standing.mods.iter().any(|m| m.amount == ffh_core::persons::DEMAND_REFUSED.0), "B was refused a demand");
        assert_eq!(g.house_state(house("C")), LoyaltyState::Disloyal);
        assert_eq!(g.houses[house("C")].cities.len(), 2);
        assert_eq!(g.houses[house("C")].standing.countdown, Some(g.game_turn() + 8));
        assert_eq!(g.house_cities_held(house("C")).1.len(), 1, "one of House C's cities is held by an army");
        assert_eq!(g.houses[house("D")].head, None);
        assert!(g.house_view(house("D")).unwrap().needs_heir);

        // House A's wonder demand, in production in its seat.
        let a_head = person("head_a");
        let dm = g.persons[a_head].demand.expect("House A's head has a demand");
        assert!(matches!(dm.kind, DemandKind::Wonder { .. }));
        assert!(g.demand_is_paused(dm.kind));

        // The commanders, the persons without a post (the heir candidates), the hero.
        assert_eq!(g.persons[person("commander_loyal")].post, Post::Commander(army("loyal")));
        assert_eq!(g.person_state(person("commander_loyal")), LoyaltyState::Loyal);
        assert!(g.unit_is_hero(g.person_unit(person("commander_loyal")).unwrap()));
        assert_eq!(g.persons[person("commander_disloyal")].post, Post::Commander(army("disloyal")));
        assert_eq!(g.person_state(person("commander_disloyal")), LoyaltyState::Disloyal);
        assert!(g.persons[person("commander_disloyal")].standing.countdown.is_some());
        assert!(!g.persons[person("commander_disloyal")].standing.sworn.is_empty());
        assert_eq!(g.armies[army("no_commander")].commander, None);
        assert_eq!(g.army_units(army("holds_c")).len(), 2);
        assert_eq!(g.player_heir_candidates(h), vec![person("candidate_1"), person("candidate_2"), person("no_post")]);

        // The taken city: claimed by Houses A and B and by the commander who took it.
        let claims = g.city_claimants(st.taken);
        assert_eq!(claims.len(), 3, "{claims:?}");
        assert!(claims.contains(&Claimant::House(house("A"))) && claims.contains(&Claimant::House(house("B"))));
        assert!(claims.contains(&Claimant::Commander(person("commander_loyal"))));

        // The great person can found a House; the neighbour's near city is in Land range.
        assert!(g.unit_is_great_person(st.great_person));
        assert_eq!(g.player_check_grant(h, st.crown[1], GrantTo::GreatPerson(st.great_person)), Ok(()));
        assert_eq!(g.cities[st.neighbour_near].owner, st.neighbour);
        assert_eq!(st.neighbour_cities.len(), 2);
        let court = g.court_view(h);
        assert_eq!(court.grants, Ok(()));
        assert_eq!(court.houses.len(), 4);

        let god_king = d.t::<CivicType>("CIVIC_GOD_KING");
        assert_eq!(direct, g.player_can_do_civics(h, Some(god_king)));
        if direct {
            assert!(g.player_can_revolution(h, None));
        }
    }
}
