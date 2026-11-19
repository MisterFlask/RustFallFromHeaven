//! Persons of Note (docs/PERSONS.md, "The second build": Houses as vassal civilizations; "The first
//! build" for the cast, commanders and demands): the cast, commanders, demands and loyalty on the
//! attitude scale, founding and grants, every vassal a House, tribute, the levy, breaking away,
//! freeing, side deals, previews, saves. Every situation is built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::log::Audience;
use ffh_core::map::{Map, MapInitData};
use ffh_core::persons::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::TradeData;
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
    world_of("CIVILIZATION_BANNOR", "LEADER_SABATHIEL")
}

/// [`world`] with player 0 of civilization `civ`, led by `leader`.
fn world_of(civ: &str, leader: &str) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    setup.players[0].civ = Some(d.t::<CivilizationType>(civ));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>(leader));
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

/// Player `p` adopts government civic `name` (Aristocracy: Houses may be granted cities).
fn government_of(g: &mut Game, p: PlayerId, name: &str) {
    let c = g.d.t::<CivicType>(name);
    let option = g.d.civics[c].civic_option_type.expect("a civic option");
    g.player_set_civics(p, option, Some(c));
}

fn government(g: &mut Game, name: &str) {
    government_of(g, P0, name);
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

/// A veteran of player 0 once entered (a drawn leader), with their unit.
fn enter_veteran(g: &mut Game, x: i32, y: i32) -> (PersonId, UnitId) {
    let u = veteran(g, x, y, 30);
    turn(g);
    (g.unit_person(u).expect("entered"), u)
}

fn plot(g: &Game, c: CityId) -> PlotIdx {
    g.city_plot(c)
}

/// The city on plot `q` now (a city changing hands has a new id).
fn city_at(g: &Game, q: PlotIdx) -> CityId {
    g.map.plots[q].plot_city.expect("a city")
}

fn mods(g: &Game, h: HouseId) -> Vec<(String, i32)> {
    g.houses[h].standing.mods.iter().map(|m| (m.cause.label(), m.amount)).collect()
}

fn logged(g: &Game, s: &str) -> bool {
    g.log.entries.iter().any(|e| e.text.contains(s))
}

/// A head of Land ambition, a veteran, with House `h` founded at city `seat`.
fn land_head(g: &mut Game, seat: CityId) -> (PersonId, HouseId) {
    let (x, y) = (g.cities[seat].x, g.cities[seat].y + 1);
    let (p, _) = enter_veteran(g, x, y);
    g.persons[p].ambition = Ambition::Land;
    g.persons[p].demand = None;
    let h = g.player_grant_city(P0, seat, GrantTo::Person(p)).expect("granted");
    (p, h)
}

/// A loyalty modifier of `amount` for 50 turns on House `h` (its other modifiers gone; its
/// tribute's standing part set to 0 by a tribute of 10%).
fn set_house_loyalty(g: &mut Game, h: HouseId, amount: i32) {
    let t = g.game_turn();
    g.houses[h].tribute = 10;
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount, until: Some(t + 50) }];
}

fn set_person_loyalty(g: &mut Game, x: PersonId, amount: i32) {
    let t = g.game_turn();
    g.persons[x].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount, until: Some(t + 50) }];
}

fn gold(data: i32) -> TradeData {
    TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false }
}

fn item(t: TradeableItem) -> TradeData {
    TradeData { item_type: t, data: 0, offering: false, hidden: false }
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
    assert_eq!(g.persons[g.unit_person(adept).unwrap()].ambition, Ambition::Works, "an adept hero: Works");
    assert!(logged(&g, "Warrior enters the court: level"), "told");
    turn(&mut g);
    assert_eq!(g.players[P0].persons.len(), 2, "entered once");
    // Furious without a post: leaves the cast, the unit stays a hero and never enters again.
    let t = g.game_turn();
    g.persons[x].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -6, until: Some(t + 50) });
    turn(&mut g);
    assert!(!g.persons.contains(x), "left");
    assert!(g.units.contains(u) && g.unit_is_hero(u));
    assert!(logged(&g, "is Furious and leaves the court"));
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
    turn(&mut g);
    assert!(g.unit_person(v2).is_some(), "level 6 meets the bar 5 + 1");
    turn(&mut g);
    assert!(g.unit_person(v3).is_none(), "level 5 is under the bar 5 + 2");
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
    let maereth = d.t::<LeaderHeadType>("LEADER_MAERETH_ASHLEDGER");
    assert_eq!(d.leader_heads[maereth].traits.first(), Some(&d.t::<TraitType>("TRAIT_ORGANIZED")));
    assert_eq!(ambition_of_leader(d, maereth), Ambition::Land);
}

// ---------------------------------------------------------------------------------------------
// The attitude scale
// ---------------------------------------------------------------------------------------------

#[test]
fn the_attitude_scale() {
    assert_eq!(LoyaltyState::of(3), LoyaltyState::Friendly);
    assert_eq!(LoyaltyState::of(2), LoyaltyState::Pleased);
    assert_eq!(LoyaltyState::of(1), LoyaltyState::Pleased);
    assert_eq!(LoyaltyState::of(0), LoyaltyState::Cautious);
    assert_eq!(LoyaltyState::of(-2), LoyaltyState::Cautious);
    assert_eq!(LoyaltyState::of(-3), LoyaltyState::Annoyed);
    assert_eq!(LoyaltyState::of(-5), LoyaltyState::Annoyed);
    assert_eq!(LoyaltyState::of(-6), LoyaltyState::Furious);
    assert!(!LoyaltyState::Cautious.withholds() && LoyaltyState::Annoyed.withholds() && LoyaltyState::Furious.withholds());
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    let hp = g.houses[h].player;
    for (amount, state) in [(3, LoyaltyState::Friendly), (1, LoyaltyState::Pleased), (0, LoyaltyState::Cautious), (-3, LoyaltyState::Annoyed), (-6, LoyaltyState::Furious)] {
        set_house_loyalty(&mut g, h, amount);
        assert_eq!(g.house_state(h), state);
        assert_eq!(g.player_ai_get_attitude(hp, P0, false), state.attitude(), "the House's attitude toward its liege is its state");
        assert_eq!(g.player_ai_get_attitude(hp, P0, true), state.attitude());
    }
}

#[test]
fn loyalty_expires_and_a_furious_house_counts_down() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let a_plot = plot(&g, a);
    let (_, h) = land_head(&mut g, a);
    set_house_loyalty(&mut g, h, 0);
    let t = g.game_turn();
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 1) }, LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 3) }];
    assert_eq!(g.house_loyalty(h), -6);
    assert_eq!(g.house_state(h), LoyaltyState::Furious);
    turn(&mut g);
    assert_eq!(g.house_loyalty(h), -3, "the first ended on its turn");
    assert_eq!(g.houses[h].standing.countdown, None);
    let t = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 30) });
    turn(&mut g);
    assert_eq!(g.house_state(h), LoyaltyState::Furious);
    let end = g.houses[h].standing.countdown.expect("the countdown");
    assert_eq!(end, g.game_turn() + 10);
    let outcome = g.house_breakaway_outcome(h);
    assert!(outcome.contains(" will break away in 10 turns, taking ") && outcome.ends_with(": war."), "{outcome}");
    assert!(logged(&g, &format!("{} is Furious. {outcome}", g.house_label(h))));
    let city = city_at(&g, a_plot);
    assert_eq!(g.city_house_mark(city).map(|m| (m.state, m.countdown_turns)), Some((LoyaltyState::Furious, Some(10))));
    // Back above: the countdown stops.
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandMet, amount: 6, until: Some(end + 60) });
    turn(&mut g);
    assert_eq!(g.houses[h].standing.countdown, None);
}

// ---------------------------------------------------------------------------------------------
// Founding and grants
// ---------------------------------------------------------------------------------------------

#[test]
fn a_veteran_founds_a_house_a_vassal_civilization() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, u) = enter_veteran(&mut g, 5, 6);
    let leader = g.persons[x].leader.unwrap();
    // City A: a garrison warrior, an army standing in it, a unit marching through it.
    let garrison = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let w1 = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let w2 = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let army = g.army_form(P0, &[w1, w2]).unwrap();
    let marcher = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    g.units[marcher].march = Some(ffh_core::army::March { dest: ffh_core::army::RecruitDest::Army(army), stalled: 0, left_turn: g.game_turn(), from_city: a, diverted: false });
    let a_plot = plot(&g, a);
    let players_before = g.num_players();
    let h = g.player_grant_city(P0, a, GrantTo::Person(x)).expect("granted");
    // A new player, the House's, the liege's vassal.
    assert_eq!(g.num_players(), players_before + 1);
    let hp = g.houses[h].player;
    assert_eq!(g.players[hp].house, Some(h));
    assert_eq!(g.player_house(hp), Some(h));
    let (t0, ht) = (g.players[P0].team, g.players[hp].team);
    assert!(g.teams[ht].vassal[t0], "the liege's vassal");
    assert_eq!(g.player_get_leader_type(hp), Some(leader), "led by the person");
    assert_eq!(g.player_get_civilization_type(hp), g.player_get_civilization_type(P0));
    assert_eq!(g.player_get_civilization_description(hp), g.house_label(h));
    assert!(g.house_label(h).starts_with("House "));
    assert_eq!(g.houses[h].origin, HouseOrigin::Founded);
    let barbarian = g.barbarian_player();
    assert!(std::sync::Arc::ptr_eq(&g.controllers[hp], &g.controllers[barbarian]), "played by the game's AI");
    assert!(g.teams[ht].has_met[g.players[P1].team], "met everyone its liege has met");
    assert_eq!(g.player_color_player(hp), P0, "the liege's colour");
    // The city moved with its garrison; the army and the marcher stay the liege's.
    let now = city_at(&g, a_plot);
    assert_eq!(g.cities[now].owner, hp);
    assert_eq!(g.city_house(now), Some(h));
    assert!(!g.units.contains(garrison) || g.units[garrison].death_delay, "the garrison went (converted)");
    assert!(g.map.plots[a_plot].units.iter().any(|&o| g.units[o].owner == hp), "a garrison unit of the House");
    assert_eq!(g.units[w1].owner, P0);
    assert_eq!(g.units[w2].owner, P0);
    assert_eq!(g.units[marcher].owner, P0);
    // The person left the map and heads the House.
    assert!(!g.units.contains(u), "their unit is removed");
    assert!(g.persons.contains(x));
    assert_eq!(g.persons[x].unit, None);
    assert_eq!(g.persons[x].post, Post::Head(h));
    assert_eq!(g.houses[h].head, Some(x));
    assert_eq!(g.houses[h].tribute, TRIBUTE_DEFAULT);
    let m = mods(&g, h);
    assert!(m.iter().any(|(l, n)| l.starts_with("Granted ") && *n == 1), "{m:?}");
    assert!(!m.iter().any(|(l, _)| l == "Sought protection"), "a founding has no starting modifier: {m:?}");
    assert_eq!(g.persons[x].demand, None, "its demand for a post met");
    assert!(logged(&g, "leaves the map"));
    assert!(!logged(&g, "is your House"), "a founding is told once");
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
    assert_eq!(g.player_get_leader_type(g.houses[h].player), g.persons[x].leader);
}

#[test]
fn a_grant_to_a_house_moves_the_city_to_its_player() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    let b_plot = plot(&g, b);
    let warrior = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    let warrior2 = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    let soldier = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    g.army_form(P0, &[soldier]).expect("an army");
    // The grant list's preview of the garrison is the rules' own: the two warriors go, the
    // army's stays the liege's.
    let (going, staying) = g.city_grant_garrison(b, None);
    assert_eq!((going.clone(), staying.clone()), (vec![warrior, warrior2], vec![soldier]));
    g.player_grant_city(P0, b, GrantTo::House(h)).expect("granted");
    let hp = g.houses[h].player;
    let now = city_at(&g, b_plot);
    assert_eq!(g.cities[now].owner, hp);
    assert_eq!(g.house_cities(h).len(), 2);
    for u in going {
        assert!(!g.units.contains(u) || g.units[u].death_delay, "the garrison went (converted)");
    }
    assert_eq!(g.map.plots[b_plot].units.iter().filter(|&&u| g.units.contains(u) && g.units[u].owner == hp && !g.units[u].death_delay).count(), 2, "the House's now");
    assert_eq!(g.units[soldier].owner, P0, "the army's unit stays the liege's");
    assert_eq!(mods(&g, h).iter().filter(|(l, n)| l.starts_with("Granted ") && *n == 1).count(), 2);
}

#[test]
fn grant_refusals() {
    let World { mut g, cap, a, enemy, .. } = world();
    let (x, _) = enter_veteran(&mut g, 5, 6);
    assert_eq!(g.player_grant_city(P0, a, GrantTo::Person(x)), Err("Government: direct rule. No city can be granted."), "Despotism");
    assert!(g.court_view(P0).grantable.is_empty());
    government(&mut g, "CIVIC_ARISTOCRACY");
    assert_eq!(g.player_grant_city(P0, cap, GrantTo::Person(x)), Err("The capital is the Crown's."));
    assert_eq!(g.player_grant_city(P0, enemy, GrantTo::Person(x)), Err("Not a city of yours."));
    let hx = enter_hero(&mut g, 5, 7);
    assert_eq!(g.player_grant_city(P0, a, GrantTo::Person(hx)), Err("A hero cannot head a House."));
    let h = g.player_grant_city(P0, a, GrantTo::Person(x)).expect("granted");
    assert_eq!(g.player_grant_city(P0, a, GrantTo::House(h)), Err("Not a city of yours."), "the city is the House's player's");
    let court = g.court_view(P0);
    assert_eq!(court.grants, Ok(()));
    assert_eq!(court.houses.len(), 1);
    assert!(!court.grantable.contains(&cap));
    // A House grants nothing: it has no persons, and a vassal holds no vassals.
    let hp = g.houses[h].player;
    assert!(!g.player_has_persons(hp));
    assert!(g.player_check_grants(hp).is_err());
}

#[test]
fn the_grant_list_states_the_maintenance_honestly() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let m = g.grant_maintenance(b);
    let own = g.cities[b].maintenance;
    assert_eq!(m.now_times100, g.players[P0].cities.iter().map(|&c| g.cities[c].maintenance).sum::<i32>(), "the rules' maintenance now");
    assert!(m.after_times100 <= m.now_times100 - own + 1, "the city's own maintenance goes: {m:?} own {own}");
    // What the grant does is what the preview says.
    let (x, _) = enter_veteran(&mut g, 5, 6);
    let m = g.grant_maintenance(a);
    g.player_grant_city(P0, a, GrantTo::Person(x)).unwrap();
    let after: i32 = g.players[P0].cities.iter().map(|&c| g.cities[c].maintenance).sum();
    assert_eq!(after, m.after_times100, "{m:?}");
}

// ---------------------------------------------------------------------------------------------
// Every vassal is a House
// ---------------------------------------------------------------------------------------------

#[test]
fn a_capitulation_makes_a_house_and_ends_with_the_vassalage() {
    let World { mut g, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_assign_vassal(t0, t1, true);
    let h = g.player_house(P1).expect("a House");
    assert_eq!(g.houses[h].owner, P0);
    assert_eq!(g.houses[h].origin, HouseOrigin::Capitulated);
    assert_eq!(g.house_label(h), g.player_get_civilization_short_description(P1));
    assert!(mods(&g, h).contains(&("Capitulated".into(), -4)));
    let x = g.houses[h].head.expect("a head");
    assert_eq!(g.persons[x].origin, PersonOrigin::Ruler);
    assert_eq!(g.persons[x].owner, P0, "in the liege's cast");
    assert_eq!(g.persons[x].leader, g.player_get_leader_type(P1));
    assert_eq!(g.persons[x].unit, None);
    assert!(g.players[P0].persons.contains(&x));
    // -4 capitulated, -1 tribute 20%: Annoyed.
    assert_eq!(g.house_state(h), LoyaltyState::Annoyed);
    assert_eq!(g.player_ai_get_attitude(P1, P0, true), AttitudeType::ATTITUDE_ANNOYED);
    // Civ4's revolt rule frees no House, and no one cancels its vassal deal.
    assert!(!g.team_can_vassal_revolt(t1, t0));
    let deals: Vec<DealId> = g.deals.ids().into_iter().filter(|&d| g.deal_is_vassal_deal(d)).collect();
    assert!(!deals.is_empty(), "a deal records it");
    for d in deals {
        assert!(!g.deal_is_cancelable(d, Some(P1)) && !g.deal_is_cancelable(d, Some(P0)));
    }
    // The vassalage ends: the record ends, the head leaves the cast.
    g.team_free_vassal(t0, t1);
    assert!(!g.teams[t1].vassal[t0]);
    assert!(!g.houses.contains(h));
    assert_eq!(g.players[P1].house, None);
    assert!(!g.persons.contains(x));
    assert!(g.players[P0].houses.is_empty());
}

#[test]
fn a_vassal_deal_seeks_protection_even_under_direct_rule() {
    let World { mut g, .. } = world();
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    // Under Despotism (direct rule) a vassal is held, as a House: no city can be granted to it.
    assert_eq!(g.team_vassal_refusal(t0), None);
    g.team_set_vassal(t1, t0, true, false);
    let h = g.player_house(P1).expect("a House");
    assert_eq!(g.houses[h].origin, HouseOrigin::Vassal);
    assert!(mods(&g, h).contains(&("Sought protection".into(), 2)));
    assert_eq!(g.player_check_grants(P0), Err("Government: direct rule. No city can be granted."));
    // Its turn under direct rule keeps it: only adopting direct rule is civil war.
    g.player_governance_turn(P0);
    assert_eq!(g.player_house(P1), Some(h));
    assert!(g.teams[t1].vassal[t0]);
}

// ---------------------------------------------------------------------------------------------
// Tribute
// ---------------------------------------------------------------------------------------------

#[test]
fn tribute_takes_gold_and_adds_research() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    let hp = g.houses[h].player;
    // The standing part: 1 - pct/10.
    let base = g.house_loyalty(h);
    g.player_set_house_tribute(P0, h, 0).unwrap();
    assert_eq!(g.house_loyalty(h), base + 2, "20% is -1, 0% is +1");
    g.player_set_house_tribute(P0, h, 50).unwrap();
    assert_eq!(g.house_loyalty(h), base - 3 - 5, "50% is -4, and the raise from 0% is -5 for 30 turns");
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Tribute 50%" && p.amount == -4));
    assert_eq!(g.player_set_house_tribute(P0, h, 25), Err("Tribute: 0 to 50 percent, in steps of 10."));
    assert_eq!(g.player_set_house_tribute(P1, h, 20), Err("Not a House of yours."));
    g.player_set_house_tribute_default(P0, 30).unwrap();
    assert_eq!(g.players[P0].house_tribute, 30);
    // The transfer.
    let (gold_commerce, research_commerce) = g.house_commerce(h);
    g.players[hp].gold = 1000;
    let liege_gold = g.players[P0].gold;
    let tech = g.d.techs.keys().find(|&t| g.player_can_research(P0, t, false)).expect("a tech to research");
    g.player_push_research(P0, tech, true);
    let team = g.players[P0].team;
    let progress = g.team_get_research_progress(team, Some(tech));
    turn(&mut g);
    let want_gold = gold_commerce * 50 / 100;
    let want_research = research_commerce * 50 / 100;
    assert_eq!(g.players[hp].gold, 1000 - want_gold);
    assert_eq!(g.players[P0].gold, liege_gold + want_gold);
    assert_eq!(g.houses[h].last_gold, want_gold);
    assert_eq!(g.houses[h].last_research, want_research);
    if !g.team_is_has_tech(team, Some(tech)) {
        assert_eq!(g.team_get_research_progress(team, Some(tech)), progress + want_research, "added to the liege's research");
    }
    // No more than it holds.
    g.players[hp].gold = 0;
    turn(&mut g);
    assert_eq!(g.houses[h].last_gold, 0);
    // Lost while the liege researches nothing.
    g.player_clear_research_queue(P0);
    turn(&mut g);
    assert_eq!(g.houses[h].last_research, 0);
    assert_eq!(g.house_tribute_at(h, 50).1, 0);
}

// ---------------------------------------------------------------------------------------------
// The levy
// ---------------------------------------------------------------------------------------------

#[test]
fn the_levy_trains_the_lieges_unit_and_is_a_grievance() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let a_plot = plot(&g, a);
    let (_, h) = land_head(&mut g, a);
    set_house_loyalty(&mut g, h, 0);
    let hc = city_at(&g, a_plot);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 9, 5);
    let army = g.army_form(P0, &[w]).unwrap();
    g.army_add_requisition(P0, army, hc).expect("a House's city may be requisitioned");
    g.army_set_target_size(P0, army, 3).unwrap();
    g.player_do_requisition(P0);
    let orders = g.army_requisition_orders(army);
    assert_eq!(orders.iter().map(|&(c, _)| c).collect::<Vec<_>>(), vec![hc], "one order in the House's city");
    assert!(mods(&g, h).contains(&("Levied".into(), -1)), "a grievance");
    // The House's own requisition step keeps it.
    let hp = g.houses[h].player;
    g.player_do_requisition(hp);
    assert_eq!(g.army_requisition_orders(army).len(), 1);
    // Trained: the liege's unit, marching to the army.
    let units_before = g.players[P0].units.len();
    let (_, i) = g.army_requisition_orders(army)[0];
    assert_eq!(i, 0);
    g.city_pop_order(hc, 0, true, false);
    assert_eq!(g.players[P0].units.len(), units_before + 1, "the liege's unit");
    let new = *g.players[P0].units.last().unwrap();
    assert!(g.units[new].march.is_some() || g.units[new].army == Some(army), "it goes to the army");
    // Annoyed: passed over, with the reason.
    set_house_loyalty(&mut g, h, -3);
    assert_eq!(g.house_levy_refusal(h), Some(format!("{} is Annoyed. It takes no levy.", g.house_label(h))));
    assert_eq!(g.city_levy_refusal(hc), g.house_levy_refusal(h));
    g.player_do_requisition(P0);
    assert!(g.army_requisition_orders(army).is_empty(), "no order placed while Annoyed");
}

// ---------------------------------------------------------------------------------------------
// Breaking away
// ---------------------------------------------------------------------------------------------

/// Player 0's House `h` to the end of its countdown.
fn countdown_ends(g: &mut Game, h: HouseId) {
    set_house_loyalty(g, h, -6);
    turn(g);
    assert!(g.houses[h].standing.countdown.is_some());
    for _ in 0..g.d.defines.person_disloyal_turns {
        if !g.houses.contains(h) {
            return;
        }
        set_house_loyalty(g, h, -6);
        turn(g);
    }
}

#[test]
fn a_furious_house_breaks_away_and_its_held_cities_return() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (a_plot, b_plot) = (plot(&g, a), plot(&g, b));
    let (x, h) = land_head(&mut g, a);
    g.player_grant_city(P0, b, GrantTo::House(h)).unwrap();
    // An army of the liege stands in B (a vassal's land is friendly to its master), where the
    // House has no unit; another in A, outnumbered by the House's two warriors.
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    g.army_form(P0, &[w]).unwrap();
    let hp = g.houses[h].player;
    unit(&mut g, hp, "UNIT_WARRIOR", 7, 4);
    unit(&mut g, hp, "UNIT_WARRIOR", 7, 4);
    let wa = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    g.army_form(P0, &[wa]).unwrap();
    let hb = city_at(&g, b_plot);
    let (going, held) = g.house_cities_held(h);
    assert_eq!((going, held), (vec![city_at(&g, a_plot)], vec![hb]));
    let (a_name, b_name) = (g.d.text.name(&g.cities[city_at(&g, a_plot)].name).to_string(), g.d.text.name(&g.cities[hb].name).to_string());
    let label = g.house_label(h);
    countdown_ends(&mut g, h);
    assert!(!g.houses.contains(h), "the record ended");
    assert!(!g.persons.contains(x), "the head left the cast: they lead the civilization");
    assert_eq!(g.players[hp].house, None);
    let (t0, ht) = (g.players[P0].team, g.players[hp].team);
    assert!(!g.teams[ht].vassal[t0]);
    assert!(g.at_war(t0, ht), "at war");
    assert_eq!(g.cities[city_at(&g, a_plot)].owner, hp, "A goes");
    assert_eq!(g.cities[city_at(&g, b_plot)].owner, P0, "B, held, went back to the liege");
    assert!(logged(&g, &format!("{label} breaks away from the Bannor, taking ")), "told");
    assert!(logged(&g, &format!("{b_name} returns to you: your army's 1 unit outnumbers its 0.")));
    assert!(logged(&g, &format!("{a_name} goes with {label}: its 2 units against your army's 1.")));
}

/// The contest for a House's city (rule 7): the liege's army units standing in it against the
/// House's own land combat units; a tie goes to the House; a unit without strength and the units
/// of an army whose commander is Furious do not count.
#[test]
fn a_city_is_held_only_when_the_lieges_army_outnumbers_the_house() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let b_plot = plot(&g, b);
    let (_, h) = land_head(&mut g, a);
    g.player_grant_city(P0, b, GrantTo::House(h)).unwrap();
    let hp = g.houses[h].player;
    let hb = city_at(&g, b_plot);
    let name = g.d.text.name(&g.cities[hb].name).to_string();
    // The House: two warriors (its settler, without strength, does not count).
    unit(&mut g, hp, "UNIT_WARRIOR", 11, 4);
    unit(&mut g, hp, "UNIT_WARRIOR", 11, 4);
    unit(&mut g, hp, "UNIT_SETTLER", 11, 4);
    assert_eq!(g.city_hold(hb), Some(CityHold { liege: 0, armies: 0, house: 2 }));
    assert_eq!(g.city_hold_line(hb, "it"), None, "no army of the liege: nothing to say");
    // The liege: an army of two: a tie.
    let l: Vec<UnitId> = (0..2).map(|_| unit(&mut g, P0, "UNIT_WARRIOR", 11, 4)).collect();
    let army = g.army_form(P0, &l).unwrap();
    assert_eq!(g.city_hold(hb), Some(CityHold { liege: 2, armies: 1, house: 2 }));
    assert!(!g.city_held_by_army(hb), "a tie goes to the House");
    assert!(g.house_cities_held(h).1.is_empty());
    assert_eq!(g.city_hold_line(hb, "it"), Some(format!("{name} goes with it: its 2 units against your army's 2.")));
    // A third unit in the army: held.
    let l3 = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    g.army_join(P0, &[l3], army).unwrap();
    assert!(g.city_held_by_army(hb));
    assert_eq!(g.house_cities_held(h).1, vec![hb]);
    let line = format!("{name} returns to you: your army's 3 units outnumber its 2.");
    assert_eq!(g.city_hold_line(hb, "it"), Some(line.clone()));
    assert!(g.house_breakaway_outcome(h).ends_with(&line), "{}", g.house_breakaway_outcome(h));
    // A second army of one: the armies' units together.
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    let second = g.army_form(P0, &[w]).unwrap();
    assert_ne!(second, army);
    assert_eq!(g.city_hold_line(hb, "it"), Some(format!("{name} returns to you: your armies' 4 units outnumber its 2.")));
    // A veteran joins and commands the first army: 5 against 2. Furious, that army counts for
    // nothing: the second's one unit against the House's two.
    let v = veteran(&mut g, 11, 4, 9);
    g.army_join(P0, &[v], army).unwrap();
    turn(&mut g);
    let x = g.unit_person(v).expect("a veteran");
    assert_eq!(g.armies[army].commander, Some(x));
    assert_eq!(g.city_hold(hb).map(|hold| (hold.liege, hold.held())), Some((5, true)));
    set_person_loyalty(&mut g, x, -6);
    assert_eq!(g.person_state(x), LoyaltyState::Furious);
    assert_eq!(g.city_hold(hb), Some(CityHold { liege: 1, armies: 1, house: 2 }));
    assert_eq!(g.city_hold_line(hb, "it"), Some(format!("{name} goes with it: its 2 units against your army's 1.")));
}

// ---------------------------------------------------------------------------------------------
// Tribute raises
// ---------------------------------------------------------------------------------------------

fn raises(g: &Game, h: HouseId) -> Vec<i32> {
    g.houses[h].standing.mods.iter().filter(|m| m.cause == LoyaltyCause::TributeRaised).map(|m| m.amount).collect()
}

/// A raise of the tribute by k steps is remembered (rule 4): `Tribute raised: -k (30 turns)` on
/// top of the standing part; a cut adds nothing and leaves the raises running; two raises stack;
/// the realm's default taken by a new House is no raise. The selector counts the raise.
#[test]
fn a_tribute_raise_is_remembered_and_a_cut_is_not() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    assert_eq!(g.houses[h].tribute, 20, "the realm's default");
    assert!(raises(&g, h).is_empty(), "the default is no raise");
    let base = g.house_loyalty(h);
    // The selector: 40% is two steps up (the standing part -3, raised -2); 0% a cut.
    let view = g.house_tribute_view(h);
    let rate = |pct: i32| view.at.iter().find(|r| r.pct == pct).copied().unwrap();
    assert_eq!((rate(40).part, rate(40).raised, rate(40).loyalty), (-3, -2, base - 4));
    assert_eq!((rate(20).part, rate(20).raised, rate(20).loyalty), (-1, 0, base));
    assert_eq!((rate(0).part, rate(0).raised, rate(0).loyalty), (1, 0, base + 2));
    assert_eq!(g.house_loyalty_at_tribute(h, 40), base - 4);
    // Raised two steps.
    g.player_set_house_tribute(P0, h, 40).unwrap();
    assert_eq!(g.house_loyalty(h), base - 4);
    assert_eq!(raises(&g, h), vec![-2]);
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Tribute raised" && p.amount == -2 && p.turns_left == Some(TRIBUTE_RAISED.1)));
    // Cut to 10%: the standing part rises at once (+2), the raise keeps running.
    g.player_set_house_tribute(P0, h, 10).unwrap();
    assert_eq!(g.house_loyalty(h), base + 1 - 2);
    assert_eq!(raises(&g, h), vec![-2], "a cut adds nothing");
    // A second raise stacks: 10% to 20%.
    g.player_set_house_tribute(P0, h, 20).unwrap();
    assert_eq!(raises(&g, h), vec![-2, -1]);
    assert_eq!(g.house_loyalty(h), base - 3);
    // The same rate again is no raise.
    g.player_set_house_tribute(P0, h, 20).unwrap();
    assert_eq!(raises(&g, h), vec![-2, -1]);
    // They run 30 turns.
    let t = g.game_turn();
    g.set_game_turn(t + TRIBUTE_RAISED.1);
    assert!(g.house_view(h).unwrap().parts.iter().all(|p| p.label != "Tribute raised"), "ended");
}

#[test]
fn direct_rule_is_civil_war() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (_, h1) = land_head(&mut g, a);
    let (_, h2) = land_head(&mut g, b);
    set_house_loyalty(&mut g, h2, 6);
    let (p1, p2) = (g.houses[h1].player, g.houses[h2].player);
    government(&mut g, "CIVIC_DESPOTISM");
    assert!(g.players[P0].houses.is_empty(), "adopting direct rule: every House breaks away");
    let t0 = g.players[P0].team;
    for hp in [p1, p2] {
        assert!(g.at_war(t0, g.players[hp].team), "every House breaks away, Friendly included");
    }
    assert!(logged(&g, "Civil war:"));
}

#[test]
fn freeing_a_house_ends_the_vassalage_at_peace() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    let hp = g.houses[h].player;
    assert_eq!(g.player_check_free_house(P1, h), Err("Not a House of yours."));
    g.player_free_house(P0, h).expect("freed");
    assert!(!g.houses.contains(h) && !g.persons.contains(x));
    let (t0, ht) = (g.players[P0].team, g.players[hp].team);
    assert!(!g.teams[ht].vassal[t0] && !g.at_war(t0, ht));
}

// ---------------------------------------------------------------------------------------------
// Side deals
// ---------------------------------------------------------------------------------------------

/// Player 1 under Aristocracy, with House `h` of player 0's at loyalty `amount`.
fn bribe_world(amount: i32) -> (Game, HouseId) {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    government_of(&mut g, P1, "CIVIC_ARISTOCRACY");
    let (_, h) = land_head(&mut g, a);
    set_house_loyalty(&mut g, h, amount);
    (g, h)
}

#[test]
fn a_houses_price_and_its_refusals() {
    let (mut g, h) = bribe_world(2);
    let hp = g.houses[h].player;
    assert_eq!(g.house_state(h), LoyaltyState::Pleased);
    assert_eq!(g.house_bribe_price(h), None);
    assert_eq!(g.house_bribe_refusal(h, P1, BribeKind::Secede), Some(format!("{} is Pleased with the Bannor.", g.house_label(h))));
    assert!(!g.player_can_trade_item(hp, P1, &item(TradeableItem::TRADE_SECEDE), false));
    assert_eq!(g.house_bribe_refusal(h, P0, BribeKind::Secede), Some(format!("{} pays its liege by tribute.", g.house_label(h))));
    // Cautious: (loyalty + 6) x 25 x its cities.
    set_house_loyalty(&mut g, h, -1);
    assert_eq!(g.house_state(h), LoyaltyState::Cautious);
    assert_eq!(g.house_bribe_price(h), Some(5 * 25));
    assert_eq!(g.house_bribe_refusal(h, P1, BribeKind::Secede), None);
    assert!(g.player_can_trade_item(hp, P1, &item(TradeableItem::TRADE_SECEDE), false));
    // Furious: one step at least (never free: the countdown is the liege's window).
    set_house_loyalty(&mut g, h, -7);
    assert_eq!(g.house_bribe_price(h), Some(25));
    // A House trades nothing else; only gold comes to it, and not from its liege.
    assert!(!g.player_can_trade_item(hp, P1, &item(TradeableItem::TRADE_MAPS), false));
    assert!(!g.player_can_trade_item(P1, hp, &item(TradeableItem::TRADE_OPEN_BORDERS), false));
    assert!(g.player_can_trade_item(P1, hp, &gold(0), false));
    assert!(!g.player_can_trade_item(P0, hp, &gold(0), false));
    assert!(!g.player_can_trade_item(P1, P0, &item(TradeableItem::TRADE_SECEDE), false), "only a House gives it");
    assert_eq!(house_offer_value(&[gold(100), TradeData { item_type: TradeableItem::TRADE_GOLD_PER_TURN, data: 5, offering: false, hidden: false }]), 100, "gold only");
}

/// A refused offer leaves no trace (the owner's decision, 2026-11-18): no grudge of the liege, no
/// `Offered` entry in the House's loyalty, no message.
#[test]
fn an_offer_refused_leaves_no_trace() {
    let (mut g, h) = bribe_world(-1);
    let hp = g.houses[h].player;
    g.players[P1].gold = 500;
    let price = g.house_bribe_price(h).unwrap();
    let extra = g.players[P0].ai_attitude_extra[P1];
    let before = mods(&g, h);
    let messages = g.log.entries.len();
    // Below the price (and the null controller of this world refuses any offer).
    assert!(!g.player_offer_house(P1, hp, &[gold(price - 1)], &[item(TradeableItem::TRADE_SECEDE)]));
    assert!(g.houses.contains(h));
    assert_eq!(mods(&g, h), before, "no Offered entry");
    assert_eq!(g.players[P0].ai_attitude_extra[P1], extra, "the liege bears no grudge");
    assert_eq!(g.log.entries.len(), messages, "no message");
    assert_eq!(g.players[P1].gold, 500);
}

#[test]
fn secede_breaks_away_for_the_buyer() {
    let (mut g, h) = bribe_world(-1);
    let hp = g.houses[h].player;
    g.players[P1].gold = 500;
    let price = g.house_bribe_price(h).unwrap();
    let extra = g.players[P0].ai_attitude_extra[P1];
    g.implement_deal(P1, hp, &[gold(price)], &[item(TradeableItem::TRADE_SECEDE)], false);
    assert!(!g.houses.contains(h), "broke away");
    let (t0, t1, ht) = (g.players[P0].team, g.players[P1].team, g.players[hp].team);
    assert!(g.at_war(t0, ht), "at war with its old liege");
    assert!(!g.at_war(t1, ht));
    assert_eq!(g.players[P1].gold, 500 - price);
    // An accepted offer is noted: the liege is told, an AI liege remembers.
    assert!(logged(&g, "The Elohim offered "));
    assert_eq!(g.players[P0].ai_memory_count[P1][MemoryType::MEMORY_BOUGHT_HOUSE], 1, "the liege remembers");
    assert_eq!(g.players[P0].ai_attitude_extra[P1], extra, "as a memory, not a modifier");
    // The news names the cities after "taking"; the buyer reads "you".
    let told = |p: PlayerId, s: &str| g.log.entries.iter().any(|e| e.text.contains(s) && matches!(&e.audience, Audience::Players(l) if l.contains(&p)));
    assert!(told(P0, ", taking ") && told(P0, ". Bought by the Elohim: war."), "the liege's news");
    assert!(told(P1, ". Bought by you: war.") && !told(P1, "Bought by the Elohim"), "the buyer's news");
}

#[test]
fn become_your_vassal_makes_it_the_buyers_house_and_the_buyer_shares_its_war() {
    let (mut g, h) = bribe_world(-1);
    let hp = g.houses[h].player;
    g.players[P1].gold = 500;
    let price = g.house_bribe_price(h).unwrap();
    g.implement_deal(P1, hp, &[gold(price)], &[item(TradeableItem::TRADE_HOUSE_VASSAL)], false);
    assert!(!g.houses.contains(h));
    let nh = g.player_house(hp).expect("the buyer's House now");
    assert_eq!(g.houses[nh].owner, P1);
    let (t0, t1, ht) = (g.players[P0].team, g.players[P1].team, g.players[hp].team);
    assert!(g.teams[ht].vassal[t1]);
    assert!(g.at_war(t0, ht) && g.at_war(t0, t1), "the buyer shares its war with the old liege");
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
    g.player_set_commander(P0, army, x2).expect("named");
    assert_eq!(g.armies[army].commander, Some(x2));
    let other = g.army_split(P0, &[v2]).expect("split");
    assert_eq!(g.armies[army].commander, Some(x1));
    assert_eq!(g.armies[other].commander, Some(x2));
    // Founding a House ends the command and takes the person off the map.
    g.player_grant_city(P0, a, GrantTo::Person(x1)).expect("granted");
    assert_eq!(g.armies[army].commander, None);
    assert!(matches!(g.persons[x1].post, Post::Head(_)));
    assert_eq!(g.player_set_commander(P0, army, x1), Err("A House's head does not command."));
}

#[test]
fn an_annoyed_commanders_army_takes_no_order_to_attack() {
    let World { mut g, .. } = world();
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_declare_war(t0, t1, false, None);
    let v = veteran(&mut g, 12, 5, 9);
    let w = unit(&mut g, P0, "UNIT_WARRIOR", 12, 5);
    let army = g.army_form(P0, &[v, w]).unwrap();
    turn(&mut g);
    let x = g.unit_person(v).expect("a veteran");
    unit(&mut g, P1, "UNIT_WARRIOR", 13, 5);
    let target = g.map.plot_xy(13, 5).unwrap();
    assert!(g.unit_can_move_into(w, target, true, false, false), "Cautious: it attacks");
    set_person_loyalty(&mut g, x, -3);
    assert_eq!(g.person_state(x), LoyaltyState::Annoyed);
    assert!(!g.unit_can_move_into(w, target, true, false, false), "Annoyed: no attack");
    let name = g.persons[x].name.clone();
    assert_eq!(g.unit_attack_refusal(w), Some(format!("Commander {name} is Annoyed. The {} takes no order to attack.", g.armies[army].name)));
}

#[test]
fn a_furious_commander_seizes_a_city() {
    let World { mut g, b, cap, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let v = veteran(&mut g, 12, 5, 9);
    let w1 = unit(&mut g, P0, "UNIT_WARRIOR", 12, 5);
    let w2 = unit(&mut g, P0, "UNIT_WARRIOR", 12, 5);
    g.army_form(P0, &[v, w1, w2]).unwrap();
    turn(&mut g);
    let x = g.unit_person(v).expect("a veteran");
    set_person_loyalty(&mut g, x, -6);
    turn(&mut g);
    assert_eq!(g.persons[x].standing.sworn.len(), 3);
    let (going, _) = g.commander_sworn_going(x);
    assert_eq!(g.commander_seize_target(x, &going), Some(b), "the nearest, not the capital");
    let b_plot = plot(&g, b);
    let players_before = g.num_players();
    for _ in 0..g.d.defines.person_disloyal_turns {
        if !g.persons.contains(x) {
            break;
        }
        set_person_loyalty(&mut g, x, -6);
        turn(&mut g);
    }
    assert!(!g.persons.contains(x));
    assert_eq!(g.num_players(), players_before + 1);
    let np = PlayerId(players_before as u8);
    assert_eq!(g.cities[city_at(&g, b_plot)].owner, np, "B seized");
    assert_eq!(g.cities[cap].owner, P0);
    assert!(g.at_war(g.players[np].team, g.players[P0].team));
}

// ---------------------------------------------------------------------------------------------
// Demands
// ---------------------------------------------------------------------------------------------

#[test]
fn a_post_demand_lapses_and_one_only_refused_stays_annoyed() {
    let World { mut g, .. } = world();
    let x = enter_hero(&mut g, 5, 6);
    let dm = g.persons[x].demand.expect("a demand at once");
    assert_eq!(dm.kind, DemandKind::Post);
    while g.persons[x].demand.is_some() {
        turn(&mut g);
    }
    let t = g.game_turn();
    assert_eq!(g.persons[x].standing.mods, vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 60) }]);
    turn(&mut g);
    assert_eq!(g.person_state(x), LoyaltyState::Annoyed);
    assert!(logged(&g, "Warrior is Annoyed."));
    for _ in 0..200 {
        turn(&mut g);
        assert!(g.persons.contains(x));
        assert!(g.person_loyalty(x) >= -3);
    }
}

#[test]
fn land_demands_the_nearest_crown_city_met_at_once_by_the_grant() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    turn(&mut g);
    let dm = g.persons[x].demand.expect("a Land demand");
    assert_eq!(dm.kind, DemandKind::GrantCity(plot(&g, c)), "C is nearest the seat");
    let preview = g.grant_preview(P0, c, GrantTo::House(h));
    assert!(preview.iter().any(|l| l.text == "Demand met: +3 (60 turns)"), "{preview:?}");
    g.player_grant_city(P0, c, GrantTo::House(h)).expect("granted");
    assert_eq!(g.persons[x].demand, None, "met at once");
}

#[test]
fn a_grant_snubs_the_claimants_and_names_a_rival() {
    let World { mut g, a, b, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x1, h1) = land_head(&mut g, a);
    let (x2, h2) = land_head(&mut g, b);
    // (No demand stands: the snub alone.)
    for x in [x1, x2] {
        g.persons[x].demand = None;
        g.persons[x].demand_rest_until = 1000;
    }
    let claimants = g.city_claimants(c);
    assert!(claimants.contains(&Claimant::House(h1)) && claimants.contains(&Claimant::House(h2)), "{claimants:?}");
    let cands = g.grant_candidates(P0, c);
    let to_h2 = cands.iter().find(|k| k.to == GrantTo::House(h2)).unwrap();
    assert!(to_h2.lines.iter().any(|l| l.subject == g.house_label(h1) && l.text.ends_with("-1 (30 turns)")), "{:?}", to_h2.lines);
    let before = g.house_loyalty(h1);
    g.player_grant_city(P0, c, GrantTo::House(h2)).expect("granted");
    assert_eq!(g.house_loyalty(h1), before - 1);
    assert_eq!(g.persons[x1].rival, Some(x2));
}

#[test]
fn war_demands_war_then_a_city_and_peace_refuses_it() {
    let World { mut g, b, enemy, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, b);
    g.persons[x].ambition = Ambition::War;
    g.persons[x].demand = None;
    turn(&mut g);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::War(P1)));
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_declare_war(t0, t1, false, None);
    turn(&mut g);
    assert_eq!(g.persons[x].demand, None, "met");
    for _ in 0..20 {
        turn(&mut g);
    }
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::TakeCity(plot(&g, enemy))));
    let before = g.house_loyalty(h);
    g.team_make_peace(t0, t1, true);
    assert_eq!(g.persons[x].demand, None, "refused at once");
    assert_eq!(g.house_loyalty(h), before - 3);
}

#[test]
fn a_works_head_demands_land() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, _) = land_head(&mut g, a);
    g.persons[x].ambition = Ambition::Works;
    turn(&mut g);
    assert_eq!(g.persons[x].demand.map(|d| d.kind), Some(DemandKind::GrantCity(plot(&g, c))), "no wonder: the Land list");
}

// ---------------------------------------------------------------------------------------------
// The read API and saves
// ---------------------------------------------------------------------------------------------

#[test]
fn read_api_and_save_round_trip() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (x, h) = land_head(&mut g, a);
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_assign_vassal(t0, t1, true);
    let hv = g.house_view(h).unwrap();
    assert_eq!(hv.player, g.houses[h].player);
    assert_eq!(hv.head, Some(x));
    assert_eq!(hv.cities, g.house_cities(h));
    assert_eq!(hv.tribute.at.len(), 6);
    assert!(hv.levy.takes);
    assert_eq!(hv.free, Ok(()));
    let court = g.court_view(P0);
    assert_eq!(court.houses.len(), 2);
    assert_eq!(court.tribute_default, TRIBUTE_DEFAULT);
    assert_eq!(g.city_house_mark(hv.cities[0]).map(|m| (m.house, m.owner)), Some((h, P0)));
    assert_eq!(g.city_house_mark(c), None);
    // Save and load.
    let bytes = g.save();
    let g2 = Game::load(g.d, &bytes).expect("loads");
    assert_eq!(g2.persons.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), g.persons.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    assert_eq!(g2.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), g.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    assert_eq!(g2.players[P0].houses, g.players[P0].houses);
    assert_eq!(g2.players[P1].house, g.players[P1].house);
    assert_eq!(g2.players[hv.player].house, Some(h));
    assert_eq!(g2.players[P0].house_tribute, TRIBUTE_DEFAULT);
    assert_eq!(g2.num_players(), g.num_players());
    assert_eq!(g2.house_view(h), g.house_view(h));
    assert_eq!(g2.person_view(x), g.person_view(x));
}

// ---------------------------------------------------------------------------------------------
// The Crown's limit and a House's size (rule 14)
// ---------------------------------------------------------------------------------------------

/// The sum of player `p`'s cities' cached maintenance (hundredths).
fn cached_maintenance(g: &Game, p: PlayerId) -> i32 {
    g.players[p].cities.iter().map(|&c| g.cities[c].maintenance).sum()
}

#[test]
fn the_crown_beyond_its_limit_pays_triple() {
    let World { mut g, cap, a, b, c, .. } = world();
    // Despotism (direct rule): no limit.
    assert_eq!(g.player_crown_limit(P0), None);
    // Every governing civic: PERSON_CROWN_CITIES (3).
    for civic in ["CIVIC_CITY_STATES", "CIVIC_REPUBLIC", "CIVIC_ARISTOCRACY"] {
        government(&mut g, civic);
        assert_eq!(g.player_crown_limit(P0), Some(g.d.defines.person_crown_cities));
    }
    assert_eq!(g.player_crown_limit(P0), Some(3));
    assert_eq!(g.player_crown_cities(P0), vec![cap, a, b, c], "the capital first, then in order");
    assert_eq!(g.player_crown_over(P0), vec![c]);
    // Same size, same count: the city beyond pays triple the city-count maintenance.
    let (nb, nc) = (g.city_calculate_num_cities_maintenance_times100(b), g.city_calculate_num_cities_maintenance_times100(c));
    assert!(nb > 0);
    assert_eq!(nc, 3 * nb);
    assert!(g.player_crown_over_cost_times100(P0) > 0);
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0), "the finance total follows");
    let court = g.court_view(P0);
    assert_eq!((court.crown_limit, court.crown_over.clone()), (Some(3), vec![c]));
    // A grant brings the Crown under its limit: the preview counts the saving, the grant does it.
    let m = g.grant_maintenance(b);
    assert!(m.now_times100 - m.after_times100 > g.cities[b].maintenance, "more than the city's own: {m:?}");
    assert_eq!(m.now_times100, cached_maintenance(&g, P0));
    let (x, _) = enter_veteran(&mut g, 5, 6);
    g.player_grant_city(P0, b, GrantTo::Person(x)).expect("granted");
    assert_eq!(g.player_crown_cities(P0), vec![cap, a, c]);
    assert!(g.player_crown_over(P0).is_empty());
    assert_eq!(cached_maintenance(&g, P0), m.after_times100, "what the preview said");
    assert_eq!(g.city_calculate_num_cities_maintenance_times100(c), g.city_calculate_num_cities_maintenance_times100(a), "c at the normal rate again");
    // A change of civics refreshes it: under direct rule no limit (the House breaks away first).
    let f = g.player_init_city(P0, 15, 4, true, true);
    assert_eq!(g.player_crown_over(P0), vec![f]);
    government(&mut g, "CIVIC_DESPOTISM");
    assert_eq!(g.player_crown_limit(P0), None);
    assert!(g.player_crown_over(P0).is_empty());
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0));
}

/// The Kuriotates (Sprawling: a city cap, here 2 great cities on a Duel map, and
/// `crown_cities_change` -2): the Crown's limit is the capital alone; settlements never count
/// toward it and are granted like any city, staying settlements; the cap counts the realm, the
/// Houses' great cities with the liege's.
#[test]
fn the_kuriotates_crown_is_the_capital_and_the_realm_counts_its_houses() {
    let World { mut g, cap, a, b, c, .. } = world_of("CIVILIZATION_KURIOTATES", "LEADER_CARDITH");
    let cap_of_great = g.players[P0].max_cities;
    assert_eq!(cap_of_great, 2, "Sprawling's 3, the Duel map's -1");
    assert!(!g.cities[cap].settlement && !g.cities[a].settlement);
    assert!(g.cities[b].settlement && g.cities[c].settlement, "beyond the cap: settlements");
    assert_eq!(g.player_realm_great_cities(P0), 2);
    // (c) The Crown's limit: 3 - 2 = 1, the capital alone.
    government(&mut g, "CIVIC_ARISTOCRACY");
    assert_eq!(g.player_crown_limit(P0), Some(1));
    // (b) Settlements never count toward it, nor are over it.
    assert_eq!(g.player_crown_cities(P0), vec![cap, a]);
    assert_eq!(g.player_crown_over(P0), vec![a]);
    assert!(g.player_crown_over_cost_times100(P0) > 0);
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0));
    // (b) A settlement is granted like any city: it founds a House and stays a settlement.
    let b_plot = plot(&g, b);
    let (x, _) = enter_veteran(&mut g, 11, 5);
    assert_eq!(g.player_check_grant(P0, b, GrantTo::Person(x)), Ok(()));
    let h = g.player_grant_city(P0, b, GrantTo::Person(x)).expect("granted");
    let hp = g.houses[h].player;
    let b2 = city_at(&g, b_plot);
    assert_eq!(g.cities[b2].owner, hp);
    assert!(g.cities[b2].settlement, "a settlement stays one");
    assert_eq!(g.cities[b2].plot_radius, 2);
    assert_eq!(g.players[hp].max_cities, cap_of_great, "the House is Kuriotate: the same cap");
    assert_eq!(g.player_crown_cities(P0), vec![cap, a], "the Crown's count unchanged");
    // (d) The realm's great cities: the liege's and its Houses' together, from either side.
    assert_eq!(g.player_realm_great_cities(P0), 2);
    assert_eq!(g.player_realm_great_cities(hp), 2);
    // A great city granted stays great; the realm's count is unchanged.
    let a_plot = plot(&g, a);
    g.player_grant_city(P0, a, GrantTo::House(h)).expect("granted");
    let a2 = city_at(&g, a_plot);
    assert_eq!(g.cities[a2].owner, hp);
    assert!(!g.cities[a2].settlement, "a great city stays great");
    assert_eq!(g.cities[a2].plot_radius, 3);
    assert_eq!(g.player_realm_great_cities(P0), 2);
    assert!(g.player_crown_over(P0).is_empty(), "the capital alone");
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0));
    // (d) A city the House founds beyond the realm's cap is a settlement, as one its liege founds.
    let new = g.player_init_city(hp, 11, 7, true, true);
    assert!(g.cities[new].settlement, "the realm is at its cap");
    let own = g.player_init_city(P0, 15, 4, true, true);
    assert!(g.cities[own].settlement);
    assert_eq!(g.player_realm_great_cities(P0), 2);
}

#[test]
fn a_house_of_more_than_three_cities_grieves() {
    let World { mut g, a, b, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let d = g.player_init_city(P0, 3, 7, true, true);
    let (_, h) = land_head(&mut g, a);
    g.player_grant_city(P0, b, GrantTo::House(h)).unwrap();
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    assert_eq!(g.house_cities(h).len(), 3);
    assert_eq!(g.house_size_part(h), 0);
    assert!(!g.house_view(h).unwrap().parts.iter().any(|p| p.label.starts_with("Holds ")));
    // The fourth city: the preview says it, the grant does it.
    let preview = g.grant_preview(P0, d, GrantTo::House(h));
    assert!(preview.iter().any(|l| l.text == "Holds 4 cities, more than 3: -1"), "{preview:?}");
    let before = g.house_loyalty(h);
    g.player_grant_city(P0, d, GrantTo::House(h)).unwrap();
    assert_eq!(g.house_size_part(h), -1);
    assert_eq!(g.house_loyalty(h), before + GRANTED.0 - 1);
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Holds 4 cities, more than 3" && p.amount == -1 && p.turns_left.is_none()));
}

/// The size grievance's threshold scales with the realm (the liege's great cities and its
/// Houses'): `max(3, 2 + realm / 8)`, 3 at 9 cities, 4 at 16, 5 at 24. A House's settlements do
/// not count; a grant leaves the threshold as it is, a loss moves it.
#[test]
fn a_house_grieves_beyond_a_threshold_that_scales_with_the_realm() {
    let World { mut g, cap, a, b, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let mut free: Vec<(i32, i32)> = (3..=8).flat_map(|y| (2..=17).map(move |x| (x, y))).filter(|&(x, y)| g.map.plot_xy(x, y).is_some_and(|q| g.map.plots[q].plot_city.is_none())).collect();
    free.reverse();
    let (_, h) = land_head(&mut g, a);
    g.player_grant_city(P0, b, GrantTo::House(h)).unwrap();
    g.player_grant_city(P0, c, GrantTo::House(h)).unwrap();
    let hp = g.houses[h].player;
    let mut found = |g: &mut Game, p: PlayerId, n: usize| -> Vec<CityId> {
        (0..n)
            .map(|_| loop {
                let (x, y) = free.pop().expect("a free plot");
                if g.map.plot_xy(x, y).is_some_and(|q| g.map.plots[q].plot_city.is_none()) {
                    break g.player_init_city(p, x, y, true, true);
                }
            })
            .collect()
    };
    // The House holds 5, the Crown 4: 9 in the realm, from either side.
    found(&mut g, hp, 2);
    found(&mut g, P0, 3);
    assert_eq!((g.house_great_cities(h), g.player_realm_great_cities(P0), g.player_realm_great_cities(hp)), (5, 9, 9));
    assert_eq!(g.player_house_size_threshold(P0), 3);
    assert_eq!(g.house_size_threshold(h), 3);
    assert_eq!(g.house_size_part(h), -2);
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Holds 5 cities, more than 3" && p.amount == -2));
    // A settlement of the House is not counted, in its size or in the realm's.
    let s = g.players[hp].cities[1];
    g.cities[s].settlement = true;
    assert_eq!((g.house_great_cities(h), g.player_realm_great_cities(P0), g.house_size_part(h)), (4, 8, -1));
    g.cities[s].settlement = false;
    // 16 in the realm: 4.
    found(&mut g, P0, 7);
    assert_eq!(g.player_realm_great_cities(P0), 16);
    assert_eq!(g.house_size_threshold(h), 4);
    assert_eq!(g.house_size_part(h), -1);
    // A grant moves a city within the realm: the threshold stays, the preview counts the city.
    let crown: Vec<CityId> = g.players[P0].cities.iter().copied().filter(|&k| k != cap).collect();
    let k = crown[crown.len() - 1];
    let preview = g.grant_preview(P0, k, GrantTo::House(h));
    assert!(preview.iter().any(|l| l.text == "Holds 6 cities, more than 4: -1 (-2 in all)"), "{preview:?}");
    g.player_grant_city(P0, k, GrantTo::House(h)).unwrap();
    assert_eq!((g.player_realm_great_cities(P0), g.house_size_threshold(h), g.house_size_part(h)), (16, 4, -2));
    // 24 in the realm: 5.
    found(&mut g, P0, 8);
    assert_eq!(g.player_realm_great_cities(P0), 24);
    assert_eq!(g.house_size_threshold(h), 5);
    assert_eq!(g.house_size_part(h), -1);
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Holds 6 cities, more than 5" && p.amount == -1));
    // A loss moves it: 23, back to 4.
    let lost = *g.players[P0].cities.last().unwrap();
    g.city_kill(lost, true);
    assert_eq!(g.player_realm_great_cities(P0), 23);
    assert_eq!((g.house_size_threshold(h), g.house_size_part(h)), (4, -2));
}
