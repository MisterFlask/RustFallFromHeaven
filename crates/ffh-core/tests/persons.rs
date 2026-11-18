//! Persons of Note (docs/PERSONS.md, "The first build"): the cast, commanders, demands and
//! loyalty, Houses, grants, claims, heirs, the quota's accounting, previews, saves. Every
//! situation is built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::persons::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// A duel (the Bannor against the Elohim, no barbarians) on one flat block of grassland
/// (x 2..=17, y 3..=8) revealed to both, the starting units gone, the two teams met. Player 0
/// holds its capital at (3, 4) and cities A (7, 4), B (11, 4) and C (7, 7); player 1 a city at
/// (15, 7). Player 0 is the stronger.
struct World {
    g: Game,
    cap: CityId,
    a: CityId,
    b: CityId,
    c: CityId,
    enemy: CityId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    g.log.enabled = true;
    for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (np, nt) = (g.num_players(), g.num_teams());
    g.map = Map::new(d, &MapInitData { grid_w: 20, grid_h: 12, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (3..=8).contains(&y) && (2..=17).contains(&x);
        g.plot_set_plot_type(q, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
        g.plot_set_terrain_type(q, if land { grass } else { coast }, false);
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    for t in g.team_ids() {
        for q in g.map.plot_ids() {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.update_plot_groups();
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_meet(t0, t1, false);
    let cap = g.player_init_city(P0, 3, 4, true, true);
    let a = g.player_init_city(P0, 7, 4, true, true);
    let b = g.player_init_city(P0, 11, 4, true, true);
    let c = g.player_init_city(P0, 7, 7, true, true);
    let enemy = g.player_init_city(P1, 15, 7, true, true);
    for c in [cap, a, b, c, enemy] {
        for t in g.team_ids() {
            g.cities[c].revealed[t] = true;
        }
    }
    strength(&mut g);
    World { g, cap, a, b, c, enemy }
}

/// Player 0 far stronger than player 1.
fn strength(g: &mut Game) {
    g.players[P0].power = 1000;
    g.players[P1].power = 10;
}

/// Player 0 adopts government civic `name` (Aristocracy: Houses may be granted cities).
fn government(g: &mut Game, name: &str) {
    let c = g.d.t::<CivicType>(name);
    let option = g.d.civics[c].civic_option_type.expect("a civic option");
    g.player_set_civics(P0, option, Some(c));
}

/// The next game turn's persons turn of player 0.
fn turn(g: &mut Game) {
    let t = g.game_turn();
    g.set_game_turn(t + 1);
    strength(g);
    g.player_persons_turn(P0);
}

fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
    let t = g.d.t::<UnitType>(name);
    g.player_init_unit(p, t, x, y, None, None)
}

/// A warrior of player 0 with `PROMOTION_HERO`.
fn hero(g: &mut Game, x: i32, y: i32) -> UnitId {
    let u = unit(g, P0, "UNIT_WARRIOR", x, y);
    let h = g.d.t::<PromotionType>("PROMOTION_HERO");
    g.unit_set_has_promotion(u, h, true);
    u
}

/// A warrior of player 0 of level `level`.
fn veteran(g: &mut Game, x: i32, y: i32, level: i32) -> UnitId {
    let u = unit(g, P0, "UNIT_WARRIOR", x, y);
    g.units[u].level = level;
    u
}

/// The person of hero `u` once entered.
fn enter_hero(g: &mut Game, x: i32, y: i32) -> PersonId {
    let u = hero(g, x, y);
    turn(g);
    g.unit_person(u).expect("entered")
}

fn plot(g: &Game, c: CityId) -> PlotIdx {
    g.city_plot(c)
}

fn mods(g: &Game, h: HouseId) -> Vec<(String, i32)> {
    g.houses[h].standing.mods.iter().map(|m| (m.cause.label(), m.amount)).collect()
}

fn logged(g: &Game, s: &str) -> bool {
    g.log.entries.iter().any(|e| e.text.contains(s))
}

// ---------------------------------------------------------------------------------------------
// The cast
// ---------------------------------------------------------------------------------------------

#[test]
fn a_hero_enters_once_and_does_not_return() {
    let World { mut g, .. } = world();
    let u = hero(&mut g, 5, 6);
    let adept = unit(&mut g, P0, "UNIT_ADEPT", 9, 6);
    let hp = g.d.t::<PromotionType>("PROMOTION_HERO");
    g.unit_set_has_promotion(adept, hp, true);
    turn(&mut g);
    assert_eq!(g.players[P0].persons.len(), 2);
    let x = g.unit_person(u).expect("the hero is a person");
    let person = &g.persons[x];
    assert_eq!((person.origin, person.leader, person.ambition), (PersonOrigin::Hero, None, Ambition::War));
    assert_eq!(person.name, "Warrior");
    assert_eq!(g.persons[g.unit_person(adept).unwrap()].ambition, Ambition::Works, "an adept hero: Works");
    assert!(logged(&g, "Warrior enters the court: level"), "told");
    assert!(g.persons[x].deeds[0].text.starts_with("Entered the court: level"));
    turn(&mut g);
    assert_eq!(g.players[P0].persons.len(), 2, "entered once");
    // Disloyal without a post: leaves the cast, the unit stays a hero and never enters again.
    let t = g.game_turn();
    g.persons[x].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -6, until: Some(t + 50) });
    turn(&mut g);
    assert!(!g.persons.contains(x), "left");
    assert!(g.units.contains(u) && g.unit_is_hero(u));
    assert!(logged(&g, "leaves the court"));
    turn(&mut g);
    assert_eq!(g.unit_person(u), None, "not entered again");
}

#[test]
fn one_veteran_a_turn_and_the_bar_rises() {
    let World { mut g, .. } = world();
    let v1 = veteran(&mut g, 5, 6, 6);
    let v2 = veteran(&mut g, 9, 6, 6);
    let v3 = veteran(&mut g, 13, 5, 5);
    turn(&mut g);
    assert_eq!(g.players[P0].persons.len(), 1, "one a turn");
    let x1 = g.unit_person(v1).expect("the lower id of the highest level");
    assert_eq!(g.persons[x1].origin, PersonOrigin::Veteran);
    let l = g.persons[x1].leader.expect("a drawn leader");
    assert_eq!(g.d.leader_heads[l].minor_leader_of, g.player_get_civilization_type(P0));
    assert_eq!(g.units[v1].name, g.persons[x1].name, "the unit takes the name");
    assert_eq!(g.persons[x1].ambition, ambition_of_leader(g.d, l));
    turn(&mut g);
    assert!(g.unit_person(v2).is_some(), "level 6 meets the bar 5 + 1");
    turn(&mut g);
    assert!(g.unit_person(v3).is_none(), "level 5 is under the bar 5 + 2");
    g.units[v3].level = 7;
    turn(&mut g);
    assert!(g.unit_person(v3).is_some());
    let mut drawn = g.persons_drawn.clone();
    drawn.sort();
    drawn.dedup();
    assert_eq!(drawn.len(), 3, "three distinct leaders drawn");
    // A veteran whose unit dies leaves.
    let x2 = g.unit_person(v2).unwrap();
    g.unit_kill(v2, false, None);
    assert!(!g.persons.contains(x2));
    assert!(logged(&g, "is dead"));
}

#[test]
fn the_pool_runs_dry() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let pool = g.persons_pool(P0);
    assert_eq!(pool.len(), 5, "the Bannor's five preferred minor leaders");
    g.persons_drawn = pool;
    assert!(g.persons_pool(P0).is_empty());
    let civ = g.player_get_civilization_type(P0).unwrap();
    assert!(ffh_core::ffh::cards::sundering::preferred_faction_leaders(&g, civ, &[]).is_empty(), "the Sundering counts drawn leaders as in play");
    let v = veteran(&mut g, 5, 6, 9);
    turn(&mut g);
    assert_eq!(g.unit_person(v), None, "no person without a pool");
    let prophet = unit(&mut g, P0, "UNIT_PROPHET", 7, 4);
    assert_eq!(g.player_grant_city(P0, a, GrantTo::GreatPerson(prophet)), Err("No person of note is left to found a House."));
    assert!(g.units.contains(prophet));
}

#[test]
fn ambition_follows_the_first_personal_trait() {
    let d = ffh_data::global();
    let cases = [("TRAIT_AGGRESSIVE", Ambition::War), ("TRAIT_SPIRITUAL", Ambition::Faith), ("TRAIT_INDUSTRIOUS", Ambition::Works), ("TRAIT_ORGANIZED", Ambition::Land), ("TRAIT_CHARISMATIC", Ambition::Land)];
    let mut seen = 0;
    for (name, want) in cases {
        let t = d.t::<TraitType>(name);
        if let Some(l) = d.leader_heads.keys().find(|&l| d.leader_heads[l].traits.first() == Some(&t)) {
            assert_eq!(ambition_of_leader(d, l), want, "{name}");
            seen += 1;
        }
    }
    assert!(seen >= 4, "the cases are in the data");
    // Authored order: Maereth Ashledger's first trait is Organized.
    let maereth = d.t::<LeaderHeadType>("LEADER_MAERETH_ASHLEDGER");
    assert_eq!(d.leader_heads[maereth].traits.first(), Some(&d.t::<TraitType>("TRAIT_ORGANIZED")));
    assert_eq!(ambition_of_leader(d, maereth), Ambition::Land);
}

#[test]
fn a_great_person_founds_a_house_and_is_consumed() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let prophet = unit(&mut g, P0, "UNIT_PROPHET", 7, 4);
    let h = g.player_grant_city(P0, a, GrantTo::GreatPerson(prophet)).expect("granted");
    assert!(!g.units.contains(prophet), "the unit is consumed");
    let x = g.houses[h].head.expect("a head");
    assert_eq!((g.persons[x].origin, g.persons[x].unit), (PersonOrigin::GreatPerson, None));
    assert!(g.persons[x].leader.is_some());
    assert_eq!(g.persons[x].post, Post::Head(h));
    assert_eq!(g.cities[a].house, Some(h));
    assert_eq!(g.houses[h].seat, Some(a));
    assert!(logged(&g, "enters the court:") && logged(&g, ", House "));
    assert!(g.persons[x].deeds.iter().any(|d| d.text.starts_with("Founded House")));
}

// ---------------------------------------------------------------------------------------------
// Commanders
// ---------------------------------------------------------------------------------------------

#[test]
fn commander_by_rule_by_choice_and_lost_on_leaving() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let v1 = veteran(&mut g, 5, 6, 7);
    let v2 = veteran(&mut g, 5, 6, 6);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 5, 6);
    let army = g.army_form(P0, &[v1, v2, w]).expect("formed");
    turn(&mut g);
    turn(&mut g);
    let (x1, x2) = (g.unit_person(v1).unwrap(), g.unit_person(v2).unwrap());
    assert_eq!(g.armies[army].commander, Some(x1), "the highest level");
    assert_eq!(g.persons[x1].post, Post::Commander(army));
    assert_eq!(g.army_persons_with(army), vec![x1, x2]);
    // By choice.
    g.player_set_commander(P0, army, x2).expect("named");
    assert_eq!(g.armies[army].commander, Some(x2));
    assert_eq!(g.persons[x1].post, Post::None);
    assert_eq!(g.player_set_commander(P0, army, PersonId(999)), Err("Not a person of yours."));
    // x2 leaves the group: the command goes back by the rule, without a grievance.
    let other = g.army_split(P0, &[v2]).expect("split");
    assert_eq!(g.armies[army].commander, Some(x1));
    assert_eq!(g.armies[other].commander, Some(x2), "commands its own one-unit army");
    assert!(g.persons[x2].standing.mods.is_empty(), "no grievance");
    // A head does not command.
    g.player_grant_city(P0, a, GrantTo::Person(x1)).expect("granted");
    assert_eq!(g.armies[army].commander, None, "the head's army has no other person");
    assert!(matches!(g.persons[x1].post, Post::Head(_)));
    assert_eq!(g.player_set_commander(P0, army, x1), Err("A House's head does not command."));
}

// ---------------------------------------------------------------------------------------------
// Demands and loyalty
// ---------------------------------------------------------------------------------------------

#[test]
fn a_post_demand_lapses_and_one_only_refused_stays_restless() {
    let World { mut g, .. } = world();
    let x = enter_hero(&mut g, 5, 6);
    let dm = g.persons[x].demand.expect("a demand at once");
    assert_eq!(dm.kind, DemandKind::Post);
    assert_eq!(g.demand_kind_text(x, dm.kind), "A post: a command or a House.");
    assert!(logged(&g, "demands: A post: a command or a House. 40 turns."));
    // To its term.
    let mut warned = false;
    while g.persons[x].demand.is_some() {
        turn(&mut g);
        warned |= logged(&g, "10 turns left");
    }
    assert!(warned, "10 turns left told");
    let t = g.game_turn();
    assert_eq!(t, dm.deadline);
    assert_eq!(g.persons[x].standing.mods, vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 60) }]);
    assert_eq!(g.persons[x].demand_rest_until, t + 20);
    assert!(g.persons[x].deeds.iter().any(|d| d.text == "Demand refused: a post."));
    turn(&mut g);
    assert_eq!(g.person_state(x), LoyaltyState::Restless);
    assert!(logged(&g, "Warrior is Restless."));
    // Never met: a refusal every 60 turns, each lasting 60: Restless, never Disloyal.
    for _ in 0..300 {
        turn(&mut g);
        assert!(g.persons.contains(x));
        assert!(g.person_loyalty(x) >= -3, "turn {}: {}", g.game_turn(), g.person_loyalty(x));
    }
}

#[test]
fn a_post_demand_is_met_by_a_house_and_the_founder_brings_their_grievances() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let x = enter_hero(&mut g, 5, 6);
    let t = g.game_turn();
    g.persons[x].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 60) });
    let h = g.player_grant_city(P0, a, GrantTo::Person(x)).expect("granted");
    assert_eq!(g.persons[x].demand, None, "met at once");
    let m = mods(&g, h);
    assert!(m.contains(&("Demand refused".into(), -3)) && m.contains(&("Demand met".into(), 3)) && m.iter().any(|(l, n)| l.starts_with("Granted ") && *n == 1), "{m:?}");
    assert!(g.persons[x].standing.mods.is_empty(), "moved to the House");
    assert_eq!(g.house_loyalty(h), 1);
    assert!(g.house_label(h).starts_with("House "));
}

/// A head of Land ambition with House `h` at city `seat`.
fn land_head(g: &mut Game, seat: CityId) -> (PersonId, HouseId) {
    let (x, y) = (g.cities[seat].x, g.cities[seat].y + 1);
    let p = enter_hero(g, x, y);
    g.persons[p].ambition = Ambition::Land;
    g.persons[p].demand = None;
    let h = g.player_grant_city(P0, seat, GrantTo::Person(p)).expect("granted");
    (p, h)
}

#[test]
fn land_demands_the_nearest_crown_city_met_at_once_by_the_grant() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    turn(&mut g);
    let dm = g.persons[x].demand.expect("a Land demand");
    assert_eq!(dm.kind, DemandKind::GrantCity(plot(&g, c)), "C is nearest the seat (3 plots; B 4)");
    let name = g.d.text.name(&g.cities[c].name).to_string();
    assert_eq!(g.demand_kind_text(x, dm.kind), format!("{name} granted to {}.", g.house_label(h)));
    let preview = g.grant_preview(P0, c, GrantTo::House(h));
    assert!(preview.iter().any(|l| l.text == "Demand met: +3 (60 turns)"), "{preview:?}");
    g.player_grant_city(P0, c, GrantTo::House(h)).expect("granted");
    assert_eq!(g.persons[x].demand, None, "met at once");
    assert!(g.persons[x].deeds.iter().any(|d| d.text == format!("Demand met: {name} granted.")));
}

#[test]
fn land_demand_refused_when_the_city_goes_to_another_house_with_snubs_and_rivals() {
    let World { mut g, a, b, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x1, h1) = land_head(&mut g, a);
    let (x2, h2) = land_head(&mut g, b);
    for _ in 0..21 {
        turn(&mut g);
    }
    assert_eq!(g.persons[x1].demand.map(|d| d.kind), Some(DemandKind::GrantCity(plot(&g, c))));
    // A commander who took C claims it too.
    let v = veteran(&mut g, 9, 7, 9);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 9, 7);
    let army = g.army_form(P0, &[v, w]).unwrap();
    turn(&mut g);
    let xc = g.unit_person(v).expect("a veteran");
    assert_eq!(g.armies[army].commander, Some(xc));
    let t = g.game_turn();
    g.cities[c].taken_by = Some((xc, t));
    let claimants = g.city_claimants(c);
    assert!(claimants.len() == 3, "{claimants:?}");
    let cands = g.grant_candidates(P0, c);
    assert!(cands.iter().filter(|k| k.claimant).count() == 3, "{cands:?}");
    let to_h2 = cands.iter().find(|k| k.to == GrantTo::House(h2)).unwrap();
    assert!(to_h2.lines.iter().any(|l| l.subject == g.house_label(h1) && l.text.ends_with("-1 (30 turns)")), "{:?}", to_h2.lines);
    assert!(to_h2.lines.iter().any(|l| l.subject == g.house_label(h1) && l.text == "Demand refused: -3 (60 turns)"));
    assert!(to_h2.lines.iter().any(|l| l.text.ends_with("-2 (30 turns)")), "the commander's snub");
    let before = g.house_loyalty(h1);
    g.player_grant_city(P0, c, GrantTo::House(h2)).expect("granted");
    assert_eq!(g.persons[x1].demand, None, "refused at once");
    assert_eq!(g.house_loyalty(h1), before - 1 - 3);
    assert_eq!(g.persons[x1].rival, Some(x2));
    assert_eq!(g.persons[xc].rival, Some(x2));
    assert_eq!(g.person_loyalty(xc), -2);
    let label = format!("{} granted to {}", g.d.text.name(&g.cities[c].name), g.house_label(h2));
    assert!(mods(&g, h1).contains(&(label, -1)));
    assert!(g.persons[xc].deeds.iter().any(|d| d.text.starts_with("Passed over:")));
}

#[test]
fn land_demands_a_foreign_city_when_the_crown_has_none_and_raze_refuses_it() {
    let World { mut g, cap, a, b, c, enemy } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, b);
    g.player_grant_city(P0, a, GrantTo::House(h)).unwrap();
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    let _ = cap;
    for _ in 0..21 {
        turn(&mut g);
    }
    let enemy_plot = plot(&g, enemy);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::GrantCity(enemy_plot)));
    let text = g.demand_kind_text(x, DemandKind::GrantCity(enemy_plot));
    assert!(text.contains(" (") && text.contains("taken and granted to House"), "{text}");
    // Gained (the Crown's), then razed by us: refused at once.
    g.player_acquire_city(P0, enemy, false, true, true);
    let taken = g.map.plots[enemy_plot].plot_city.expect("kept");
    assert_eq!(g.cities[taken].house, None, "a city gained is the Crown's");
    let preview = g.persons_preview(P0, &PersonsAct::Raze(taken));
    assert!(preview.iter().any(|l| l.text == "Demand refused: -3 (60 turns)"), "{preview:?}");
    g.player_raze(P0, taken);
    assert_eq!(g.persons[x].demand, None);
    assert!(mods(&g, h).contains(&("Demand refused".into(), -3)));
}

/// A head of War ambition with House `h` at city `seat`, without a demand.
fn war_head(g: &mut Game, seat: CityId) -> (PersonId, HouseId) {
    let (x, h) = land_head(g, seat);
    g.persons[x].ambition = Ambition::War;
    g.persons[x].demand = None;
    (x, h)
}

#[test]
fn war_demands_war_then_a_city_and_peace_refuses_it() {
    let World { mut g, b, enemy, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = war_head(&mut g, b);
    turn(&mut g);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::War(P1)));
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    let preview = g.persons_preview(P0, &PersonsAct::DeclareWar(t1));
    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].text, "Demand met: +3 (60 turns)");
    assert_eq!(preview[0].subject, g.house_label(h));
    g.team_declare_war(t0, t1, false, None);
    turn(&mut g);
    assert_eq!(g.persons[x].demand, None, "met");
    for _ in 0..20 {
        turn(&mut g);
    }
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::TakeCity(plot(&g, enemy))));
    let preview = g.persons_preview(P0, &PersonsAct::MakePeace(t1));
    assert_eq!(preview.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), vec!["Demand refused: -3 (60 turns)"]);
    let before = g.house_loyalty(h);
    g.team_make_peace(t0, t1, true);
    assert_eq!(g.persons[x].demand, None, "refused at once");
    assert_eq!(g.house_loyalty(h), before - 3);
}

#[test]
fn a_demand_made_impossible_otherwise_is_replaced_without_a_grievance() {
    let World { mut g, b, enemy, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = war_head(&mut g, b);
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_declare_war(t0, t1, false, None);
    turn(&mut g);
    let enemy_plot = plot(&g, enemy);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::TakeCity(enemy_plot)));
    let before = mods(&g, h);
    // The city goes, not by our act.
    g.city_kill(enemy, true);
    turn(&mut g);
    assert_eq!(mods(&g, h), before, "no grievance");
    assert_ne!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::TakeCity(enemy_plot)), "replaced");
}

#[test]
fn faith_demands_a_state_religion_then_the_religion_in_a_city() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, _) = war_head(&mut g, b);
    g.persons[x].ambition = Ambition::Faith;
    let order = g.d.t::<ReligionType>("RELIGION_THE_ORDER");
    g.city_set_has_religion(a, order, true, false, false);
    if !g.player_can_convert(P0, Some(order)) {
        // The civics in force admit no state religion: nothing to test here.
        return;
    }
    turn(&mut g);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::AdoptReligion));
    let preview = g.persons_preview(P0, &PersonsAct::Convert(Some(order)));
    assert!(preview.iter().any(|l| l.text == "Demand met: +3 (60 turns)"), "{preview:?}");
    g.player_convert(P0, Some(order));
    turn(&mut g);
    assert_eq!(g.persons[x].demand, None);
    for _ in 0..20 {
        turn(&mut g);
    }
    let Some(DemandKind::SpreadReligion(q)) = g.persons[x].demand.map(|d| d.kind) else { panic!("{:?}", g.persons[x].demand) };
    assert_eq!(q, plot(&g, b), "a city of their House first");
    g.city_set_has_religion(b, order, true, false, false);
    turn(&mut g);
    assert_eq!(g.persons[x].demand, None, "met");
}

#[test]
fn works_demands_a_wonder_whose_term_pauses_while_queued() {
    let World { mut g, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let t0 = g.players[P0].team;
    for tech in g.d.techs.keys() {
        g.teams[t0].has_tech[tech] = true;
    }
    let (x, _) = war_head(&mut g, b);
    g.persons[x].ambition = Ambition::Works;
    turn(&mut g);
    let Some(DemandKind::Wonder { city, building }) = g.persons[x].demand.map(|d| d.kind) else { panic!("{:?}", g.persons[x].demand) };
    assert_eq!(city, plot(&g, b), "in their seat");
    let deadline = g.persons[x].demand.unwrap().deadline;
    g.city_push_order(b, OrderType::ORDER_CONSTRUCT, building.idx() as i32, -1, false, false, false, false);
    assert!(g.demand_is_paused(DemandKind::Wonder { city, building }));
    turn(&mut g);
    assert_eq!(g.persons[x].demand.unwrap().deadline, deadline + 1, "the term does not run");
    g.cities[b].num_real_building[building] = 1;
    turn(&mut g);
    assert_eq!(g.persons[x].demand, None, "met");
}

#[test]
fn loyalty_sums_expires_and_has_four_states() {
    assert_eq!(LoyaltyState::of(3), LoyaltyState::Devoted);
    assert_eq!(LoyaltyState::of(2), LoyaltyState::Loyal);
    assert_eq!(LoyaltyState::of(-2), LoyaltyState::Loyal);
    assert_eq!(LoyaltyState::of(-3), LoyaltyState::Restless);
    assert_eq!(LoyaltyState::of(-5), LoyaltyState::Restless);
    assert_eq!(LoyaltyState::of(-6), LoyaltyState::Disloyal);
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    g.houses[h].standing.mods.clear();
    let t = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 1) });
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 3) });
    assert_eq!(g.house_loyalty(h), -6);
    assert_eq!(g.house_state(h), LoyaltyState::Disloyal);
    turn(&mut g);
    // The first ended on its turn: Restless, the countdown not started.
    assert_eq!(g.house_loyalty(h), -3);
    assert_eq!(g.house_state(h), LoyaltyState::Restless);
    assert_eq!(g.houses[h].standing.countdown, None);
    let t = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 30) });
    turn(&mut g);
    assert_eq!(g.house_state(h), LoyaltyState::Disloyal);
    let end = g.houses[h].standing.countdown.expect("the countdown");
    assert_eq!(end, g.game_turn() + 10);
    let city = g.d.text.name(&g.cities[a].name).to_string();
    let outcome = format!("{} will secede with {city} in 10 turns.", g.house_label(h));
    assert_eq!(g.house_secession_outcome(h), outcome);
    assert!(logged(&g, &format!("{} is Disloyal. {outcome}", g.house_label(h))));
    assert_eq!(g.city_house_mark(a).map(|m| (m.state, m.countdown_turns)), Some((LoyaltyState::Disloyal, Some(10))));
    g.set_game_turn(end);
    assert_eq!(g.persons_countdowns_due(P0), vec![CountdownDue::House(h)]);
    // Back above: the countdown stops.
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandMet, amount: 6, until: Some(end + 60) });
    turn(&mut g);
    assert_eq!(g.houses[h].standing.countdown, None);
    assert!(g.persons_countdowns_due(P0).is_empty());
    let _ = x;
}

#[test]
fn a_disloyal_commander_swears_its_army() {
    let World { mut g, .. } = world();
    let v = veteran(&mut g, 5, 6, 8);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 5, 6);
    let army = g.army_form(P0, &[v, w]).unwrap();
    turn(&mut g);
    let x = g.unit_person(v).unwrap();
    assert_eq!(g.persons[x].post, Post::Commander(army));
    let t = g.game_turn();
    g.persons[x].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -6, until: Some(t + 60) });
    turn(&mut g);
    assert!(g.persons.contains(x), "a commander does not leave");
    let mut sworn = g.persons[x].standing.sworn.clone();
    sworn.sort();
    assert_eq!(sworn, vec![v, w]);
    assert!(g.commander_secession_outcome(x).contains("will leave with 2 sworn units in 10 turns"));
}

#[test]
fn a_state_religion_of_the_opposite_alignment_is_a_standing_grievance() {
    let World { mut g, .. } = world();
    let v = veteran(&mut g, 5, 6, 9);
    turn(&mut g);
    let x = g.unit_person(v).unwrap();
    let alignment = g.person_alignment(x);
    let good = g.d.t::<AlignmentType>("ALIGNMENT_GOOD");
    let evil = g.d.t::<AlignmentType>("ALIGNMENT_EVIL");
    let opposed = if alignment == Some(good) { evil } else if alignment == Some(evil) { good } else { return };
    let r = g.d.religions.keys().find(|&r| g.d.religions[r].alignment == Some(opposed)).expect("a religion");
    let preview = g.persons_preview(P0, &PersonsAct::Convert(Some(r)));
    assert!(preview.iter().any(|l| l.text == "State religion opposed: -2"), "{preview:?}");
}

// ---------------------------------------------------------------------------------------------
// Grants, Houses, heirs
// ---------------------------------------------------------------------------------------------

#[test]
fn grant_refusals() {
    let World { mut g, cap, a, enemy, .. } = world();
    let x = enter_hero(&mut g, 5, 6);
    assert_eq!(g.player_grant_city(P0, a, GrantTo::Person(x)), Err("Government: direct rule. No city can be granted."), "Despotism");
    assert!(g.court_view(P0).grantable.is_empty());
    government(&mut g, "CIVIC_ARISTOCRACY");
    assert_eq!(g.player_grant_city(P0, cap, GrantTo::Person(x)), Err("The capital is the Crown's."));
    assert_eq!(g.player_grant_city(P0, enemy, GrantTo::Person(x)), Err("Not a city of yours."));
    let h = g.player_grant_city(P0, a, GrantTo::Person(x)).expect("granted");
    assert_eq!(g.player_grant_city(P0, a, GrantTo::House(h)), Err("The city is a House's. A grant is for good."));
    assert!(g.player_grant_city(P1, enemy, GrantTo::House(h)).is_err());
    let court = g.court_view(P0);
    assert_eq!(court.grants, Ok(()));
    assert_eq!(court.houses.len(), 1);
    assert!(!court.grantable.contains(&cap) && !court.grantable.contains(&a));
}

#[test]
fn a_city_lost_leaves_its_house_and_a_house_without_cities_ends() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    assert_eq!(g.houses[h].seat, Some(a));
    let (a_plot, c_plot) = (plot(&g, a), plot(&g, c));
    g.player_acquire_city(P1, a, false, true, true);
    let lost = g.map.plots[a_plot].plot_city.unwrap();
    assert_eq!(g.cities[lost].house, None);
    assert_eq!(g.houses[h].cities, vec![c]);
    assert_eq!(g.houses[h].seat, Some(c), "a new seat");
    g.player_acquire_city(P1, c, false, true, true);
    assert!(!g.houses.contains(h), "ended");
    assert!(g.players[P0].houses.is_empty());
    assert_eq!(g.persons[x].post, Post::None);
    assert!(logged(&g, "has ended"));
    assert!(g.city_house_mark(g.map.plots[c_plot].plot_city.unwrap()).is_none());
}

#[test]
fn heirs_one_several_and_none_then_one() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    // One candidate: succeeds at once.
    let (x1, h) = land_head(&mut g, a);
    let x2 = enter_hero(&mut g, 13, 7);
    let u1 = g.persons[x1].unit.unwrap();
    g.unit_kill(u1, false, None);
    assert_eq!(g.houses[h].head, Some(x2));
    assert_eq!(g.persons[x2].post, Post::Head(h));
    assert!(logged(&g, "succeeds as head of"));
    // Several: the owner names one.
    let x3 = enter_hero(&mut g, 13, 6);
    let x4 = enter_hero(&mut g, 13, 5);
    let u2 = g.persons[x2].unit.unwrap();
    g.unit_kill(u2, false, None);
    assert_eq!(g.houses[h].head, None);
    let view = g.house_view(h).unwrap();
    assert!(view.needs_heir);
    assert_eq!(view.heir_candidates, vec![x3, x4]);
    assert_eq!(view.state, LoyaltyState::Loyal, "an empty seat is Loyal");
    assert_eq!(g.house_orders_left(h), None, "an empty seat: no limit");
    assert_eq!(g.player_name_heir(P0, h, x4), Ok(()));
    assert_eq!(g.houses[h].head, Some(x4));
    assert_eq!(g.player_name_heir(P0, h, x3), Err("The seat is not empty."));
    // None, then one enters and takes the seat.
    let h2 = g.player_grant_city(P0, b, GrantTo::Person(x3)).unwrap();
    let u4 = g.persons[x4].unit.unwrap();
    g.unit_kill(u4, false, None);
    assert_eq!(g.houses[h].head, None);
    assert!(g.player_heir_candidates(P0).is_empty());
    let u5 = hero(&mut g, 13, 4);
    turn(&mut g);
    let x5 = g.unit_person(u5).unwrap();
    assert_eq!(g.houses[h].head, Some(x5));
    assert_eq!(g.houses[h2].head, Some(x3));
}

#[test]
fn the_quota_is_counted() {
    let World { mut g, a, c, cap, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    g.houses[h].standing.mods.clear();
    assert_eq!(g.player_house_orders(P0), 150);
    assert_eq!(g.house_orders_total(h), 3, "2 cities x 150 / 100");
    assert_eq!(g.house_orders_left(h), Some(3));
    for _ in 0..3 {
        g.house_record_order(h);
    }
    assert_eq!(g.house_orders_left(h), Some(0));
    let label = g.house_label(h);
    assert_eq!(g.city_house_takes_order(a), Err(format!("{label} takes no more orders for 30 turns.")));
    assert_eq!(g.city_house_takes_order(cap), Ok(()), "the Crown's city");
    for _ in 0..30 {
        turn(&mut g);
    }
    g.houses[h].standing.mods.retain(|m| m.cause != LoyaltyCause::DemandRefused && m.cause != LoyaltyCause::DemandMet);
    assert_eq!(g.house_orders_left(h), Some(3), "the window passed");
    assert!(g.houses[h].orders.is_empty(), "pruned");
    let t = g.game_turn();
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 9) }];
    assert_eq!(g.house_orders_left(h), Some(0));
    assert_eq!(g.city_house_takes_order(c), Err(format!("{label} is Restless. It takes no orders.")));
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandMet, amount: 3, until: Some(t + 9) }];
    assert_eq!(g.house_orders_left(h), None, "Devoted: no limit");
    government(&mut g, "CIVIC_CITY_STATES");
    assert_eq!(g.player_house_orders(P0), 50);
    assert_eq!(g.house_orders_total(h), 1, "at least one");
}

#[test]
fn civics_preview_states_the_secession_and_the_cities_held() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    // An army stands in C.
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", cx, cy);
    g.army_form(P0, &[w]).unwrap();
    assert_eq!(g.house_cities_held(h), (vec![a], vec![c]));
    let despotism = g.d.t::<CivicType>("CIVIC_DESPOTISM");
    let lines = g.persons_preview(P0, &PersonsAct::Civics(&[despotism]));
    let (an, cn) = (g.d.text.name(&g.cities[a].name).to_string(), g.d.text.name(&g.cities[c].name).to_string());
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].subject, g.house_label(h));
    assert_eq!(lines[0].text, format!("Secedes with {an}. Held by armies: {cn}."));
    let republic = g.d.t::<CivicType>("CIVIC_REPUBLIC");
    assert!(g.persons_preview(P0, &PersonsAct::Civics(&[republic])).is_empty());
}

#[test]
fn read_api_and_save_round_trip() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    let v = veteran(&mut g, 9, 7, 9);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 9, 7);
    let army = g.army_form(P0, &[v, w]).unwrap();
    for _ in 0..22 {
        turn(&mut g);
    }
    let xv = g.unit_person(v).unwrap();
    g.cities[c].taken_by = Some((xv, g.game_turn()));
    g.house_record_order(h);
    assert!(g.persons[x].demand.is_some(), "a demand standing");
    let view = g.person_view(xv).unwrap();
    assert_eq!(view.post, Post::Commander(army));
    assert_eq!(view.location, PersonWhere::Army(army));
    assert_eq!(view.level, Some(9));
    assert!(view.title.is_some(), "a minor leader's title");
    assert!(view.parts.iter().all(|p| p.turns_left.is_some()));
    let hv = g.house_view(h).unwrap();
    assert_eq!(hv.cities, vec![a]);
    assert_eq!(hv.orders_left, g.house_orders_left(h));
    assert_eq!(g.city_house_mark(a).map(|m| m.house), Some(h));
    assert_eq!(g.city_house_mark(c), None);
    // Save and load.
    let bytes = g.save();
    let g2 = Game::load(g.d, &bytes).expect("loads");
    assert_eq!(g2.persons.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), g.persons.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    assert_eq!(g2.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), g.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    assert_eq!(g2.persons_drawn, g.persons_drawn);
    assert_eq!(g2.players[P0].persons, g.players[P0].persons);
    assert_eq!(g2.players[P0].houses, g.players[P0].houses);
    assert_eq!(g2.players[P0].house_names, g.players[P0].house_names);
    assert_eq!(g2.cities[a].house, Some(h));
    assert_eq!(g2.cities[c].taken_by, g.cities[c].taken_by);
    assert_eq!(g2.armies[army].commander, Some(xv));
    assert_eq!(g2.person_view(x), g.person_view(x));
}
