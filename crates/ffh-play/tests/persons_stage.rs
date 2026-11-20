//! `ffh-play stage persons|persons-direct` (docs/PERSONS.md, "UI", part K; "The third build" on
//! "The second build"): the staged scenes load, the rules' invariants hold on them
//! (`ffh_harness::check`), and each shows what the stage promises: the Houses as vassal
//! civilizations in each state (Friendly, Pleased or Cautious, the capitulated one Annoyed under a
//! contract, Furious with its ultimatum due and a city held by an army, a House of five Furious
//! too), two tributes, each House led by its player's leader, the taken city's claimant House, and
//! the neighbour's own House and the courtships; `persons-direct` can adopt God King at once.

mod common;

use std::sync::Arc;

use ffh_core::persons::{HouseOrigin, LoyaltyState};
use ffh_data::*;
use ffh_play::json::Json;

#[test]
fn the_staged_persons_scenes_hold_every_state() {
    let d = ffh_data::global();
    for direct in [false, true] {
        let dir = common::TempDir::new(if direct { "stage-persons-direct" } else { "stage-persons" });
        let st = ffh_play::stage::persons(&dir.0, direct).expect("staged");
        assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some(st.stage));
        assert!(st.to_json().get("persons").is_none(), "no persons");
        let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
        let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
        let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
        let g = &s.g;
        common::check::check(g);
        let h = st.human;
        assert_eq!(s.human, h);
        let house = |role: &str| st.houses.iter().find(|x| x.role == role).unwrap().id;
        let army = |role: &str| st.armies.iter().find(|x| x.0 == role).unwrap().1;

        // The government, the capital and the three Crown cities.
        assert_eq!(g.players[h].civics[d.civics[d.t::<CivicType>("CIVIC_ARISTOCRACY")].civic_option_type.unwrap()], Some(d.t::<CivicType>("CIVIC_ARISTOCRACY")));
        assert!(g.city_is_capital(st.capital));
        assert_eq!(st.crown.len(), 3);
        for &c in &st.crown {
            assert_eq!(g.cities[c].owner, h);
        }

        // The Houses: each a vassal civilization of the human, in its state, led by its player's
        // leader.
        for role in ["friendly", "pleased", "annoyed", "furious"] {
            let hh = house(role);
            assert_eq!(g.houses[hh].owner, h, "{role}");
            let hp = g.houses[hh].player;
            assert_eq!(g.players[hp].house, Some(hh));
            assert!(g.teams[g.players[hp].team].vassal[g.players[h].team], "{role} is a vassal");
            assert!(!g.players[hp].human);
            assert_eq!(g.house_leader(hh), g.player_get_leader_type(hp));
        }
        let civ = g.player_get_civilization_type(h);
        for role in ["friendly", "pleased", "furious"] {
            let l = g.house_leader(house(role)).unwrap();
            assert_eq!(d.leader_heads[l].minor_leader_of, civ, "{role}: a minor leader of the Bannor");
            assert_eq!(g.houses[house(role)].origin, HouseOrigin::Founded);
        }
        assert_eq!(g.house_state(house("friendly")), LoyaltyState::Friendly);
        assert_eq!(g.house_cities(house("friendly")).len(), 2);
        assert_eq!((g.houses[house("friendly")].tribute, g.houses[house("friendly")].tribute_now), (0, 0), "in force at once");
        assert!([LoyaltyState::Pleased, LoyaltyState::Cautious].contains(&g.house_state(house("pleased"))));
        assert_eq!((g.houses[house("pleased")].tribute, g.houses[house("pleased")].tribute_now), (10, 10));
        assert_eq!(g.house_state(house("annoyed")), LoyaltyState::Annoyed);
        assert_eq!(g.houses[house("annoyed")].origin, HouseOrigin::Capitulated);
        assert!(g.house_levy_refusal(house("annoyed")).is_some(), "an Annoyed House takes no levy");
        assert_eq!(g.house_state(house("furious")), LoyaltyState::Furious);
        assert!(g.houses[house("furious")].contract.is_none(), "its ultimatum due at the next turn");
        assert!(g.houses[house("annoyed")].contract.is_some_and(|c| c.kind == ffh_core::persons::UltimatumKind::Tribute(10)), "under a contract");
        assert_eq!(g.houses[house("annoyed")].ultimatums, 1);
        assert_eq!(g.house_state(house("large")), LoyaltyState::Furious);
        assert_eq!(g.house_great_cities(house("large")), 5, "one grant from the split's threshold");
        assert!(g.houses[house("large")].contract.is_none());
        assert_eq!(g.house_cities(house("furious")).len(), 2);
        assert_eq!(g.house_cities_held(house("furious")).1.len(), 1, "one of its cities is held by an army");
        assert_eq!((g.houses[house("furious")].tribute, g.houses[house("furious")].tribute_now), (20, 20));
        assert!(g.houses[house("furious")].deeds.iter().any(|dd| dd.text.starts_with("Request refused: ")), "its refused requests in its record");

        // The armies: the levy's army, the one holding the Furious House's city, the one in its
        // seat (outnumbered).
        assert_eq!(g.army_units(army("holds_furious")).len(), 2);
        assert_eq!(g.army_units(army("contests_furious")).len(), 1);
        assert!(!g.army_units(army("main")).is_empty());

        // The taken city: claimed by House `pleased`.
        let claims = g.city_claimants(st.taken);
        assert!(claims.contains(&house("pleased")), "{claims:?}");

        // The neighbour's House: Cautious or worse, the human's gift recorded on it (the fourth
        // build, rule 3).
        let n = house("neighbour");
        assert_eq!(g.houses[n].owner, st.neighbour);
        assert!(g.house_state(n) >= LoyaltyState::Cautious);
        assert!(g.gifts_value(g.houses[n].player, h) > 0, "the human's gift");
        // The neighbour courts House `pleased` by its Court channel, its favour past two thirds.
        let p = house("pleased");
        assert!(g.house_courtships(p).iter().any(|c| c.courter == st.neighbour && c.attention && c.line.starts_with("Courted by the ")));
        let court = g.court_view(h);
        assert_eq!(court.grants, Ok(()));
        assert_eq!(court.houses.len(), 5);
        assert!(court.founding.is_err(), "a realm with Houses founds none");

        let god_king = d.t::<CivicType>("CIVIC_GOD_KING");
        assert_eq!(direct, g.player_can_do_civics(h, Some(god_king)));
        if direct {
            assert!(g.player_can_revolution(h, None));
        }
    }
}
