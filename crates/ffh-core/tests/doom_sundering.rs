//! The Armageddon card The Sundering (docs/CARDS.md, section 4): every civilization of more than 3
//! cities loses its far quarter to a new faction of its own civilization, led by another leader of
//! it. Each test builds its civilizations by hand (cities on the best free sites of a generated
//! map, no starting units) and fires the card.

use ffh_core::ffh::cards::sundering as sd;
use ffh_core::ffh::cards::world;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A game of `civs` (civilization, leader), no barbarians, no starting units; every civilization
/// has met the first.
fn game(civs: &[(&str, &str)]) -> Game {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, civs.len(), "WORLDSIZE_SMALL", 11);
    for (slot, (civ, leader)) in civs.iter().enumerate() {
        setup.players[slot].civ = Some(d.t::<CivilizationType>(civ));
        setup.players[slot].leader = Some(d.t::<LeaderHeadType>(leader));
    }
    let mut g = Game::new(d, setup);
    g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    g.game_init();
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let t0 = g.players[PlayerId(0)].team;
    for i in 1..civs.len() {
        let t = g.players[PlayerId(i as u8)].team;
        g.team_meet(t0, t, false);
    }
    g
}

/// `n` cities of `p`'s, each on the best free site of the land away from every city; the first
/// is its capital.
fn cities(g: &mut Game, p: PlayerId, n: usize) -> Vec<CityId> {
    let mut out = Vec::new();
    for _ in 0..n {
        let taken: Vec<PlotIdx> = g.cities.ids().into_iter().map(|c| g.city_plot(c)).collect();
        let land: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| !g.plot_is_water(q)).collect();
        let site = world::best_sites(g, &land, 1, None, &taken)[0];
        let (x, y) = g.map.xy(site);
        out.push(g.player_init_city(p, x, y, true, true));
    }
    assert!(g.player_get_capital_city(p) == Some(out[0]), "the first city is the capital");
    out
}

fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
    g.plot_distance(ax, ay, bx, by)
}

fn leader(g: &Game, name: &str) -> LeaderHeadType {
    g.d.t::<LeaderHeadType>(name)
}

#[test]
fn a_civilization_of_eight_cities_loses_its_two_farthest_to_a_faction_of_its_own() {
    let mut g = game(&[
        ("CIVILIZATION_BANNOR", "LEADER_SABATHIEL"),
        ("CIVILIZATION_LJOSALFAR", "LEADER_ARENDEL"),
        ("CIVILIZATION_SHEAIM", "LEADER_TEBRYN"),
    ]);
    let d = g.d;
    let (p0, p1, p2) = (PlayerId(0), PlayerId(1), PlayerId(2));
    let mine = cities(&mut g, p0, 8);
    let three = cities(&mut g, p1, 3);
    let sheaim = cities(&mut g, p2, 5);
    // A tech and a card the parent knows.
    let (tech, card) = (d.t::<TechType>("TECH_MYSTICISM"), d.t::<TechType>("TECH_CARD_ABJURER"));
    let t0 = g.players[p0].team;
    g.team_set_has_tech(t0, tech, true, Some(p0), false, false);
    g.team_set_has_tech(t0, card, true, Some(p0), false, false);

    assert_eq!(sd::sundering_plan(&g), vec![(p0, leader(&g, "LEADER_CAPRIA"))], "the Bannor only, with a Bannor leader not in play");
    assert!(sd::valid_sundering(&g));
    let seceding = sd::seceding_cities(&g, p0);
    assert_eq!(seceding.len(), 2, "a quarter of eight");
    let cap = g.city_plot(mine[0]);
    let plots: Vec<PlotIdx> = seceding.iter().map(|&c| g.city_plot(c)).collect();
    let kept: Vec<PlotIdx> = mine.iter().map(|&c| g.city_plot(c)).filter(|q| !plots.contains(q)).collect();
    assert!(plots.iter().all(|&s| kept.iter().all(|&k| dist(&g, s, cap) >= dist(&g, k, cap))), "the farthest from the capital");

    // In one of them: a building, a garrison and a hero.
    let c = seceding[1];
    let elder = d.t::<BuildingType>("BUILDING_ELDER_COUNCIL");
    g.city_set_num_real_building(c, elder, 1);
    let (x, y) = g.map.xy(plots[1]);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    g.player_init_unit(p0, warrior, x, y, None, None);
    g.player_init_unit(p0, warrior, x, y, None, None);
    let hero = g.player_init_unit(p0, warrior, x, y, None, None);
    g.unit_set_has_promotion(hero, d.t::<PromotionType>("PROMOTION_HERO"), true);

    let np = world::next_player(&g);
    sd::fire_sundering(&mut g, 0);

    assert!(g.players[np].alive);
    assert_eq!(world::next_player(&g), PlayerId(np.0 + 1), "one faction");
    assert_eq!(g.player_get_civilization_type(np), g.player_get_civilization_type(p0), "of its parent's civilization");
    let l = g.player_get_leader_type(np).unwrap();
    assert_eq!(l, leader(&g, "LEADER_CAPRIA"));
    assert!(l != leader(&g, "LEADER_SABATHIEL") && d.civilizations[d.t::<CivilizationType>("CIVILIZATION_BANNOR")].leaders[l]);
    assert!(g.player_get_civilization_description(np).starts_with("Bannor of "), "named for its capital");
    for &q in &plots {
        assert_eq!(g.map.plots[q].plot_city.map(|c| g.cities[c].owner), Some(np), "the far cities secede");
    }
    assert_eq!(g.players[p0].cities.len(), 6);
    assert_eq!(g.players[p1].cities.len(), 3, "three cities: untouched");
    assert_eq!(g.players[p2].cities.len(), 5, "the Sheaim: untouched");
    assert!(three.iter().chain(&sheaim).all(|&c| g.cities.contains(c)));
    assert!(g.player_get_capital_city(np).is_some(), "a capital");
    let nc = g.map.plots[plots[1]].plot_city.unwrap();
    // (At the city's tier: a building's tier variant is the city's.)
    let class = d.buildings[elder].building_class_type;
    assert!(d.buildings.keys().any(|b| d.buildings[b].building_class_type == class && g.cities[nc].num_real_building[b] > 0), "with its buildings");
    let units: Vec<UnitId> = g.map.plots[plots[1]].units.clone();
    assert_eq!(units.iter().filter(|&&u| g.units[u].owner == np && g.units[u].unit_type == warrior).count(), 2, "with its garrison");
    assert!(g.units.contains(hero) && g.units[hero].owner == p0, "the hero stays with the parent");

    // Independent, at peace, -4 both ways, its parent's techs (not its cards), met who its parent met.
    let (nt, t1, t2) = (g.players[np].team, g.players[p1].team, g.players[p2].team);
    assert_eq!(g.players[np].parent, Some(p0));
    assert!(!g.teams[nt].vassal[t0] && !g.teams[t0].vassal[nt], "no vassal");
    assert!(!g.teams[nt].at_war[t0] && !g.teams[t0].at_war[nt], "at peace with its parent");
    assert_eq!(g.players[np].ai_attitude_extra[p0], sd::GRUDGE);
    assert_eq!(g.players[p0].ai_attitude_extra[np], sd::GRUDGE);
    assert!(g.team_is_has_tech(nt, Some(tech)) && !g.team_is_has_tech(nt, Some(card)));
    assert!(g.teams[nt].has_met[t1] && g.teams[nt].has_met[t2] && g.teams[t1].has_met[nt]);

    // Save and load after the split.
    let bytes = g.save();
    let h = Game::load(d, &bytes).expect("reload");
    assert_eq!(h.num_players(), g.num_players());
    assert_eq!(h.player_get_leader_type(np), Some(l));
    assert_eq!(h.player_get_civilization_description(np), g.player_get_civilization_description(np));
    assert_eq!(h.players[np].cities, g.players[np].cities);
    assert_eq!(h.players[np].ai_attitude_extra[p0], sd::GRUDGE);
    assert_eq!(h.players[p0].ai_attitude_extra[np], sd::GRUDGE);
    assert_eq!(h.save(), bytes, "the same game");
}

#[test]
fn a_civilization_whose_every_leader_is_in_play_gets_a_minor_leader() {
    let mut g = game(&[("CIVILIZATION_KURIOTATES", "LEADER_CARDITH"), ("CIVILIZATION_BANNOR", "LEADER_SABATHIEL")]);
    let d = g.d;
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    cities(&mut g, p0, 4);
    cities(&mut g, p1, 2);
    // The Kuriotates' only leader is in play: the leader invented for them.
    let kuriotates = d.t::<CivilizationType>("CIVILIZATION_KURIOTATES");
    let ysolde = leader(&g, "LEADER_YSOLDE");
    assert_eq!(d.leader_heads[ysolde].minor_leader_of, Some(kuriotates));
    assert!(!d.civilizations[kuriotates].leaders[ysolde], "no civilization lists it");
    assert_eq!(sd::sundering_plan(&g), vec![(p0, ysolde)]);
    // The Bannor's own leaders first; with all three taken, their FFH2 minor leader.
    let bannor = d.t::<CivilizationType>("CIVILIZATION_BANNOR");
    assert_eq!(sd::faction_leader(&g, bannor, &[]), Some(leader(&g, "LEADER_CAPRIA")));
    let taken = [leader(&g, "LEADER_CAPRIA"), leader(&g, "LEADER_DECIUS")];
    assert_eq!(sd::faction_leader(&g, bannor, &taken), Some(leader(&g, "LEADER_TETHIRA")));
    let all = [taken[0], taken[1], leader(&g, "LEADER_TETHIRA")];
    assert_eq!(sd::faction_leader(&g, bannor, &all), None, "no leader free");

    let np = world::next_player(&g);
    sd::fire_sundering(&mut g, 0);
    assert!(g.players[np].alive);
    assert_eq!(g.player_get_leader_type(np), Some(ysolde));
    assert_eq!(g.player_get_civilization_type(np), Some(kuriotates));
    assert_eq!(g.players[np].cities.len(), 1, "a quarter of four");
    assert_eq!(g.players[p0].cities.len(), 3);
    assert_eq!(g.players[p1].cities.len(), 2);
    // Its leader is in play now: no second faction would get it.
    assert_eq!(sd::faction_leader(&g, kuriotates, &[]), None);
}

#[test]
fn every_civilization_that_can_split_has_a_minor_leader() {
    let d = ffh_data::global();
    let never = ["CIVILIZATION_SHEAIM", "CIVILIZATION_INFERNAL", "CIVILIZATION_BARBARIAN", "CIVILIZATION_RANDOM"];
    // The base civilizations (not the cards'), the Mercurians included.
    let own = d.civilizations.len() - ffh_data::card_entry_count("civilizations");
    for civ in d.civilizations.keys().take(own) {
        let name = d.type_name(civ);
        if never.contains(&name) {
            continue;
        }
        assert!(d.leader_heads.keys().any(|l| d.leader_heads[l].minor_leader_of == Some(civ)), "{name} has a minor leader");
    }
    // No minor leader is a leader any civilization lists (no game starts with one).
    for l in d.leader_heads.keys().filter(|&l| d.leader_heads[l].minor_leader_of.is_some()) {
        assert!(d.civilizations.keys().all(|c| !d.civilizations[c].leaders[l]), "{}", d.type_name(l));
    }
}
