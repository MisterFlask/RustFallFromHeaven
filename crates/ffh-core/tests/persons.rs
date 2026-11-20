//! Houses (docs/PERSONS.md, "The third build": Houses without Persons of Note, on "The second
//! build": Houses as vassal civilizations): the first grant founds a House (its leader drawn from
//! the free minor leaders, the fallback to the liege's alignment, none free), grants, requests and
//! loyalty on the attitude scale, every vassal a House, tribute, the levy, breaking away, freeing,
//! previews, saves, the Crown's limit and the world unit. Every situation is built by
//! hand.

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

/// The next game turn's Houses' turn of player 0.
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

fn leader(name: &str) -> LeaderHeadType {
    ffh_data::global().t::<LeaderHeadType>(name)
}

/// A minor leader whose first trait gives `ambition` (the first in leader id order).
fn leader_of(ambition: Ambition) -> LeaderHeadType {
    let d = ffh_data::global();
    d.leader_heads.keys().find(|&l| d.leader_heads[l].minor_leader_of.is_some() && d.leader_heads[l].preferred_breakaway && ambition_of_leader(d, l) == ambition).expect("a minor leader of that ambition")
}

/// House `h` makes no request for a long while (its rest pushed back).
fn quiet(g: &mut Game, h: HouseId) {
    g.houses[h].demand = None;
    g.houses[h].demand_rest_until = g.game_turn() + 1000;
}

/// A House of player 0 founded at city `seat` by the rules' founding (`player_found_house`), led by
/// `l`, without a request.
fn house_led_by(g: &mut Game, seat: CityId, l: LeaderHeadType) -> HouseId {
    let (h, _) = g.player_found_house(P0, l, &[seat]).expect("founded");
    quiet(g, h);
    h
}

/// A House of Land ambition (Maereth Ashledger: Organized) founded at city `seat`, without a
/// request.
fn land_house(g: &mut Game, seat: CityId) -> HouseId {
    house_led_by(g, seat, leader("LEADER_MAERETH_ASHLEDGER"))
}

/// A loyalty modifier of `amount` for 50 turns on House `h` (its other modifiers gone; its
/// tribute's standing part set to 0 by a tribute of 10%).
fn set_house_loyalty(g: &mut Game, h: HouseId, amount: i32) {
    let t = g.game_turn();
    g.houses[h].tribute = 10;
    g.houses[h].tribute_now = 10;
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount, until: Some(t + 50) }];
}

/// House `h`'s tribute set and in force at `pct`, by hand (no lag).
fn in_force(g: &mut Game, h: HouseId, pct: i32) {
    g.houses[h].tribute = pct;
    g.houses[h].tribute_now = pct;
}

// ---------------------------------------------------------------------------------------------
// The attitude scale, the ambition
// ---------------------------------------------------------------------------------------------

#[test]
fn ambition_follows_the_first_personal_trait_of_the_houses_leader() {
    let d = ffh_data::global();
    let maereth = leader("LEADER_MAERETH_ASHLEDGER");
    assert_eq!(d.leader_heads[maereth].traits.first(), Some(&d.t::<TraitType>("TRAIT_ORGANIZED")));
    assert_eq!(ambition_of_leader(d, maereth), Ambition::Land);
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    assert_eq!(g.house_leader(h), Some(maereth));
    assert_eq!(g.house_ambition(h), Ambition::Land);
    assert_eq!(g.house_view(h).unwrap().leader_name, leader_name(d, maereth));
}

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
    let h = land_house(&mut g, a);
    let hp = g.houses[h].player;
    for (amount, state) in [(3, LoyaltyState::Friendly), (1, LoyaltyState::Pleased), (0, LoyaltyState::Cautious), (-3, LoyaltyState::Annoyed), (-6, LoyaltyState::Furious)] {
        set_house_loyalty(&mut g, h, amount);
        assert_eq!(g.house_state(h), state);
        assert_eq!(g.player_ai_get_attitude(hp, P0, false), state.attitude(), "the House's attitude toward its liege is its state");
        assert_eq!(g.player_ai_get_attitude(hp, P0, true), state.attitude());
    }
}

/// A liege that refuses every ultimatum (the third build, rule 5).
struct Refuses;

impl ffh_core::controller::Controller for Refuses {
    fn consider_ultimatum(&self, _g: &Game, _p: PlayerId, _u: &Ultimatum) -> bool {
        false
    }
}

#[test]
fn loyalty_expires_and_a_furious_house_issues_an_ultimatum() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    set_house_loyalty(&mut g, h, 0);
    let t = g.game_turn();
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 1) }, LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 3) }];
    assert_eq!(g.house_loyalty(h), -6);
    assert_eq!(g.house_state(h), LoyaltyState::Furious);
    let outcome = g.house_breakaway_outcome(h);
    assert!(outcome.contains(" issues an ultimatum at your next turn. Refused, it breaks away, taking ") && outcome.ends_with(": war."), "{outcome}");
    assert!(g.house_view(h).unwrap().outcome.is_some(), "Furious: the card states it");
    turn(&mut g);
    assert_eq!(g.house_loyalty(h), -3, "the first ended on its turn");
    assert!(g.houses[h].contract.is_none(), "Annoyed: no ultimatum");
    let t = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -3, until: Some(t + 30) });
    turn(&mut g);
    // Furious: an ultimatum, accepted by the liege's controller (the rules' default accepts).
    let label = g.house_label(h);
    assert!(logged(&g, &format!("{label} is Furious.")));
    assert!(logged(&g, &format!("{label} issues an ultimatum: ")));
    assert!(g.houses[h].contract.is_some(), "accepted: a contract");
    assert_eq!(g.houses[h].ultimatums, 1);
    assert_eq!(g.house_state(h), LoyaltyState::Annoyed, "at least Annoyed while it runs");
    assert!(g.house_view(h).unwrap().outcome.is_none());
}

// ---------------------------------------------------------------------------------------------
// The first grant founds a House (the third build, rule 3)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_first_grant_founds_a_house_a_vassal_civilization() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let civ = g.player_get_civilization_type(P0).unwrap();
    let pool = g.house_leader_pool(civ, P0);
    assert!(pool.own, "the Bannor's own minor leaders");
    assert_eq!(pool.leaders.len(), 10, "the Bannor's ten preferred minor leaders");
    // The grant list offers the founding alone, with the pool it draws from.
    let cands = g.grant_candidates(P0, a);
    assert_eq!(cands.len(), 1);
    assert_eq!((cands[0].to, cands[0].refusal), (GrantTo::NewHouse, None));
    assert!(cands[0].lines.iter().any(|l| l.text == "A new House: its leader is drawn from the Bannor's minor leaders"), "{:?}", cands[0].lines);
    assert_eq!(g.house_founding_line(P0).as_deref(), Some("A new House: its leader is drawn from the Bannor's minor leaders."));
    // City A: a garrison warrior, an army standing in it, a unit marching through it.
    let garrison = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let w1 = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let w2 = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    let army = g.army_form(P0, &[w1, w2]).unwrap();
    let marcher = unit(&mut g, P0, "UNIT_WARRIOR", 7, 4);
    g.units[marcher].march = Some(ffh_core::army::March { dest: ffh_core::army::RecruitDest::Army(army), stalled: 0, left_turn: g.game_turn(), from_city: a, diverted: false });
    let a_plot = plot(&g, a);
    let players_before = g.num_players();
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).expect("granted");
    // A new player, the House's, the liege's vassal, led by a drawn minor leader of the Bannor.
    assert_eq!(g.num_players(), players_before + 1);
    let hp = g.houses[h].player;
    assert_eq!(g.players[hp].house, Some(h));
    assert_eq!(g.player_house(hp), Some(h));
    let (t0, ht) = (g.players[P0].team, g.players[hp].team);
    assert!(g.teams[ht].vassal[t0], "the liege's vassal");
    let l = g.house_leader(h).expect("a leader");
    assert!(pool.leaders.contains(&l), "drawn from the pool");
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
    assert_eq!(g.houses[h].tribute, TRIBUTE_DEFAULT);
    let m = mods(&g, h);
    assert!(m.iter().any(|(l, n)| l.starts_with("Granted ") && *n == 1), "{m:?}");
    assert!(!m.iter().any(|(l, _)| l == "Sought protection"), "a founding has no starting modifier: {m:?}");
    assert!(g.houses[h].deeds.iter().any(|dd| dd.text.starts_with("Founded at ")), "its record");
    assert!(logged(&g, &format!("{} is founded at ", g.house_label(h))));
    assert!(!logged(&g, "is your House"), "a founding is told once");
    // The leader leads a living player now: out of the pool.
    assert!(!g.house_leader_pool(civ, P0).leaders.contains(&l));
    // A realm with a House grants to its Houses: no second founding.
    assert_eq!(g.player_check_found_house(P0), Err("A realm with a House grants to its Houses."));
    assert_eq!(g.player_grant_city(P0, b, GrantTo::NewHouse), Err("A realm with a House grants to its Houses."));
    let cands = g.grant_candidates(P0, b);
    assert_eq!(cands.iter().map(|k| k.to).collect::<Vec<_>>(), vec![GrantTo::House(h)]);
}

/// The pool's fallback (b): with the liege's civilization's minor leaders all leading living
/// players, the minor leaders of the other civilizations of the liege's alignment; with none of
/// those either, no House can be founded (`No leader is free to found a House.`).
#[test]
fn the_founding_draws_from_the_lieges_alignment_else_is_refused() {
    let World { mut g, cap, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let d = g.d;
    let civ = g.player_get_civilization_type(P0).unwrap();
    let bannor: Vec<LeaderHeadType> = g.house_leader_pool(civ, P0).leaders;
    assert_eq!(bannor.len(), 10);
    // Player 1 founds ten Houses, each led by one of the Bannor's minor leaders.
    for (i, &l) in bannor.iter().enumerate() {
        let c = g.player_init_city(P1, 13 + (i as i32 % 3) * 2, 3 + (i as i32 / 3) * 2, true, true);
        g.player_found_house(P1, l, &[c]).expect("player 1's House");
    }
    assert_eq!(g.players[P1].houses.len(), 10);
    let pool = g.house_leader_pool(civ, P0);
    assert!(!pool.own, "the fallback");
    let alignment = g.players[P0].alignment.expect("an alignment");
    assert!(!pool.leaders.is_empty());
    for &l in &pool.leaders {
        assert_eq!(d.leader_heads[l].alignment, Some(alignment), "the liege's alignment");
        assert!(d.leader_heads[l].minor_leader_of.is_some_and(|c| c != civ), "another civilization's minor leader");
    }
    let name = d.text.name(&d.alignments[alignment].base.description).to_string();
    assert_eq!(g.house_founding_line(P0), Some(format!("A new House: its leader is drawn from other civilizations' {name} minor leaders.")));
    let pool_b = pool.leaders.clone();
    // Founded from the fallback: a Bannor House led by another civilization's minor leader.
    let a_plot = plot(&g, a);
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).expect("founded from the fallback");
    assert!(pool_b.contains(&g.house_leader(h).unwrap()));
    assert_eq!(g.player_get_civilization_type(g.houses[h].player), Some(civ));
    // None free: refused, said in the grant list.
    g.player_free_house(P0, h).unwrap();
    assert!(g.players[P0].houses.is_empty());
    g.players[P0].alignment = None;
    assert!(g.house_leader_pool(civ, P0).leaders.is_empty());
    assert_eq!(g.house_founding_line(P0), None);
    let c2 = g.player_init_city(P0, 3, 7, true, true);
    assert_eq!(g.player_check_found_house(P0), Err("No leader is free to found a House."));
    assert_eq!(g.player_grant_city(P0, c2, GrantTo::NewHouse), Err("No leader is free to found a House."));
    let cands = g.grant_candidates(P0, c2);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].refusal, Some("No leader is free to found a House."));
    assert_eq!(g.court_view(P0).founding, Err("No leader is free to found a House."));
    assert!(g.player_check_grant_city(P0, cap).is_err(), "the capital never");
    let _ = a_plot;
}

/// The draw uses the game RNG once, at the grant (the preview is pure): two games alike draw
/// alike.
#[test]
fn the_draw_is_the_game_rngs() {
    let draw = || {
        let World { mut g, a, .. } = world();
        government(&mut g, "CIVIC_ARISTOCRACY");
        let before = g.soren_rand.seed();
        let _ = g.grant_candidates(P0, a);
        let _ = g.court_view(P0);
        assert_eq!(g.soren_rand.seed(), before, "the preview draws nothing");
        let h = g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
        g.house_leader(h)
    };
    assert_eq!(draw(), draw());
}

#[test]
fn a_grant_to_a_house_moves_the_city_to_its_player() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    let b_plot = plot(&g, b);
    let warrior = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    let warrior2 = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    let soldier = unit(&mut g, P0, "UNIT_WARRIOR", 11, 4);
    g.army_form(P0, &[soldier]).expect("an army");
    // The grant list's preview of the garrison is the rules' own: the two warriors go, the
    // army's stays the liege's.
    let (going, staying) = g.city_grant_garrison(b);
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
    assert_eq!(mods(&g, h).iter().filter(|(l, n)| l.starts_with("Granted ") && *n == 1).count(), 1);
    // No size part (the third build, rule 1): a House of many cities has none.
    assert!(!g.house_view(h).unwrap().parts.iter().any(|p| p.label.starts_with("Holds ")));
}

#[test]
fn grant_refusals() {
    let World { mut g, cap, a, enemy, .. } = world();
    assert_eq!(g.player_grant_city(P0, a, GrantTo::NewHouse), Err("Government: direct rule. No city can be granted."), "Despotism");
    assert!(g.court_view(P0).grantable.is_empty());
    government(&mut g, "CIVIC_ARISTOCRACY");
    assert_eq!(g.player_grant_city(P0, cap, GrantTo::NewHouse), Err("The capital is the Crown's."));
    assert_eq!(g.player_grant_city(P0, enemy, GrantTo::NewHouse), Err("Not a city of yours."));
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).expect("granted");
    assert_eq!(g.player_grant_city(P0, a, GrantTo::House(h)), Err("Not a city of yours."), "the city is the House's player's");
    let court = g.court_view(P0);
    assert_eq!(court.grants, Ok(()));
    assert_eq!(court.houses.len(), 1);
    assert!(!court.grantable.contains(&cap));
    // A House grants nothing: a vassal holds no vassals.
    let hp = g.houses[h].player;
    assert!(!g.player_has_houses(hp));
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
    let m = g.grant_maintenance(a);
    g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
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
    assert_eq!(g.house_leader(h), g.player_get_leader_type(P1), "its leader is its player's");
    assert!(logged(&g, "is your House: it capitulated."));
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
    // The vassalage ends: the record ends.
    g.team_free_vassal(t0, t1);
    assert!(!g.teams[t1].vassal[t0]);
    assert!(!g.houses.contains(h));
    assert_eq!(g.players[P1].house, None);
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

/// Tribute is gold only (the third build, rule 10: a realm shares its knowledge, so a research part
/// would count a House's beakers twice).
#[test]
fn tribute_takes_gold_only() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    let hp = g.houses[h].player;
    // The standing part: 1 - pct/10, of the rate in force.
    let base = g.house_loyalty(h);
    assert_eq!(g.houses[h].tribute_now, 20);
    in_force(&mut g, h, 0);
    assert_eq!(g.house_loyalty(h), base + 2, "20% is -1, 0% is +1");
    in_force(&mut g, h, 50);
    assert_eq!(g.house_loyalty(h), base - 3, "50% is -4");
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Tribute 50%" && p.amount == -4));
    assert_eq!(g.player_set_house_tribute(P0, h, 25), Err("Tribute: 0 to 50 percent, in steps of 10.".to_string()));
    assert_eq!(g.player_set_house_tribute(P1, h, 20), Err("Not a House of yours.".to_string()));
    g.player_set_house_tribute_default(P0, 30).unwrap();
    assert_eq!(g.players[P0].house_tribute, 30);
    // The transfer: gold only; the liege's research is not touched.
    let gold_commerce = g.house_commerce(h);
    g.players[hp].gold = 1000;
    let liege_gold = g.players[P0].gold;
    let tech = g.d.techs.keys().find(|&t| g.player_can_research(P0, t, false)).expect("a tech to research");
    g.player_push_research(P0, tech, true);
    let team = g.players[P0].team;
    let progress = g.team_get_research_progress(team, Some(tech));
    let own = g.player_calculate_research_rate(P0, Some(tech));
    turn(&mut g);
    let want_gold = gold_commerce * 50 / 100;
    assert_eq!(g.players[hp].gold, 1000 - want_gold);
    assert_eq!(g.players[P0].gold, liege_gold + want_gold);
    assert_eq!(g.houses[h].last_gold, want_gold);
    assert_eq!(g.house_tribute_at(h, 50), want_gold);
    // (The Houses' turn alone adds no research: the liege's own is added in its own turn.)
    assert_eq!(g.team_get_research_progress(team, Some(tech)), progress, "no research from the tribute (the liege's own rate is {own})");
    // No more than it holds.
    g.players[hp].gold = 0;
    turn(&mut g);
    assert_eq!(g.houses[h].last_gold, 0);
    assert_eq!(g.house_tribute_at(h, 50), 0);
}

// ---------------------------------------------------------------------------------------------
// The levy
// ---------------------------------------------------------------------------------------------

#[test]
fn the_levy_trains_the_lieges_unit_and_is_a_grievance() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let a_plot = plot(&g, a);
    let h = land_house(&mut g, a);
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

/// Player 0's House `h` Furious, its ultimatum refused by its liege (the third build, rule 5).
fn ultimatum_refused(g: &mut Game, h: HouseId) {
    g.controllers[P0] = std::sync::Arc::new(Refuses);
    set_house_loyalty(g, h, -6);
    turn(g);
}

#[test]
fn a_furious_house_breaks_away_and_its_held_cities_return() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let (a_plot, b_plot) = (plot(&g, a), plot(&g, b));
    let h = land_house(&mut g, a);
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
    ultimatum_refused(&mut g, h);
    assert!(!g.houses.contains(h), "the record ended");
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
/// House's own land combat units; a tie goes to the House; a unit without strength does not count.
#[test]
fn a_city_is_held_only_when_the_lieges_army_outnumbers_the_house() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let b_plot = plot(&g, b);
    let h = land_house(&mut g, a);
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
}

// ---------------------------------------------------------------------------------------------
// The tribute's lag (rule 4)
// ---------------------------------------------------------------------------------------------

/// House `h`'s rate set and rate in force.
fn rates(g: &Game, h: HouseId) -> (i32, i32) {
    (g.houses[h].tribute, g.houses[h].tribute_now)
}

/// `turns` game turns of player 0's Houses' turns; after each, House `h`'s rate in force, and that
/// its payment was at the rate in force of that turn (its treasury and commerce as they are).
fn turns_paid(g: &mut Game, h: HouseId, turns: usize) -> Vec<i32> {
    let hp = g.houses[h].player;
    let mut out = Vec::new();
    for _ in 0..turns {
        g.players[hp].gold = 100_000;
        let want = g.house_tribute_at(h, g.houses[h].tribute_now);
        turn(g);
        assert_eq!(g.houses[h].last_gold, want, "paid at the rate in force");
        out.push(g.houses[h].tribute_now);
    }
    out
}

/// The rate in force moves one step toward the rate set every 5 turns, up and down; the payment
/// and the loyalty part follow it; the selector answers for each rate once in force and says how
/// long until then.
#[test]
fn the_rate_in_force_moves_a_step_every_5_turns_both_ways() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    assert_eq!(rates(&g, h), (20, 20));
    let base = g.house_loyalty(h);
    // Set 40%: nothing moves yet.
    g.player_set_house_tribute(P0, h, 40).unwrap();
    assert_eq!(rates(&g, h), (40, 20));
    assert_eq!(g.house_loyalty(h), base, "the loyalty part is the rate in force's");
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Tribute 20%" && p.amount == -1));
    assert_eq!(g.house_tribute_turns(h, 40), 10);
    // The selector: each rate once in force, and when (the count runs from the setting).
    let view = g.house_tribute_view(h);
    assert_eq!((view.pct, view.now, view.turns), (40, 20, 10));
    let rate = |pct: i32| view.at.iter().find(|r| r.pct == pct).copied().unwrap();
    assert_eq!((rate(40).part, rate(40).turns, rate(40).loyalty), (-3, 10, base - 2));
    assert_eq!((rate(30).turns, rate(20).turns, rate(10).turns, rate(0).turns), (5, 0, 5, 10));
    assert_eq!((rate(0).part, rate(0).loyalty, rate(0).state), (1, base + 2, LoyaltyState::of(base + 2)));
    assert_eq!(g.house_loyalty_at_tribute(h, 50), base - 3);
    // Up: a step each 5 turns, in force from the turn after the processing that takes it.
    assert_eq!(turns_paid(&mut g, h, 10), [20, 20, 20, 30, 30, 30, 30, 30, 40, 40]);
    assert_eq!(rates(&g, h), (40, 40));
    assert_eq!(g.house_loyalty(h), base - 2);
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Tribute 40%" && p.amount == -3));
    assert_eq!(g.house_tribute_turns(h, 40), 0);
    // Down, from rest: four steps, 20 turns.
    g.player_set_house_tribute(P0, h, 0).unwrap();
    assert_eq!(g.house_tribute_turns(h, 0), 20);
    assert_eq!(g.house_tribute_in_force_after(h, 0, 4), 40);
    assert_eq!(g.house_tribute_in_force_after(h, 0, 5), 30);
    assert_eq!(g.house_tribute_in_force_after(h, 0, 19), 10);
    assert_eq!(g.house_tribute_in_force_after(h, 0, 20), 0);
    let seen = turns_paid(&mut g, h, 20);
    assert_eq!(seen, [40, 40, 40, 30, 30, 30, 30, 30, 20, 20, 20, 20, 20, 10, 10, 10, 10, 10, 0, 0]);
    assert_eq!(g.house_loyalty(h), base + 2);
    assert_eq!(g.house_tribute_view(h).turns, 0);
}

/// Setting the rate back before it is reached reverses the movement, keeping the count (a change
/// of mind neither costs nor gains time); setting the rate in force stops it.
#[test]
fn setting_the_rate_back_reverses_the_movement() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    g.player_set_house_tribute(P0, h, 40).unwrap();
    assert_eq!(turns_paid(&mut g, h, 4), [20, 20, 20, 30]);
    // In force at 30% from the next turn: 40% the step after, 6 turns from this turn's end.
    assert_eq!(g.house_tribute_turns(h, 40), 6);
    // Back to 0%: the count runs on (three steps, the first 6 turns away).
    g.player_set_house_tribute(P0, h, 0).unwrap();
    assert_eq!(g.house_tribute_turns(h, 0), 16);
    assert_eq!(turns_paid(&mut g, h, 5), [30, 30, 30, 30, 20]);
    assert_eq!(rates(&g, h), (0, 20));
    // The rate in force set: it stays.
    g.player_set_house_tribute(P0, h, 20).unwrap();
    assert_eq!(g.house_tribute_turns(h, 20), 0);
    assert_eq!(turns_paid(&mut g, h, 6), [20; 6]);
    // From rest again, the count starts at the setting.
    g.player_set_house_tribute(P0, h, 10).unwrap();
    assert_eq!(g.house_tribute_turns(h, 10), 5);
    assert_eq!(turns_paid(&mut g, h, 5), [20, 20, 20, 10, 10]);
}

/// A new House starts with the realm's default in force, whatever made it (a founding, a
/// capitulation); setting the default changes no House that stands.
#[test]
fn a_new_house_starts_with_its_rate_in_force() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
    assert_eq!(rates(&g, h), (TRIBUTE_DEFAULT, TRIBUTE_DEFAULT));
    assert_eq!(g.house_tribute_view(h).turns, 0);
    g.player_set_house_tribute_default(P0, 40).unwrap();
    assert_eq!(rates(&g, h), (TRIBUTE_DEFAULT, TRIBUTE_DEFAULT), "the default changes no House");
    let h2 = land_house(&mut g, b);
    assert_eq!(rates(&g, h2), (40, 40));
    assert!(g.house_view(h2).unwrap().parts.iter().any(|p| p.label == "Tribute 40%" && p.amount == -3));
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_assign_vassal(t0, t1, true);
    let h3 = g.player_house(P1).expect("a capitulated House");
    assert_eq!(rates(&g, h3), (40, 40));
}

#[test]
fn direct_rule_is_civil_war() {
    let World { mut g, a, b, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h1 = g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
    let h2 = land_house(&mut g, b);
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
    let h = land_house(&mut g, a);
    let hp = g.houses[h].player;
    assert_eq!(g.player_check_free_house(P1, h), Err("Not a House of yours."));
    g.player_free_house(P0, h).expect("freed");
    assert!(!g.houses.contains(h));
    let (t0, ht) = (g.players[P0].team, g.players[hp].team);
    assert!(!g.teams[ht].vassal[t0] && !g.at_war(t0, ht));
    // A realm without a House again: its next grant founds one.
    assert_eq!(g.player_check_found_house(P0), Ok(()));
    // The freed House a vassal again by a deal, a turn later: a vassal (it sought protection), no
    // founding, though it is named `House <the liege's last name>`.
    let t = g.game_turn();
    g.set_game_turn(t + 1);
    g.team_set_vassal(ht, t0, true, false);
    let h2 = g.player_house(hp).expect("a House again");
    assert_eq!(g.houses[h2].origin, HouseOrigin::Vassal);
    assert!(mods(&g, h2).contains(&("Sought protection".into(), 2)));
}

// ---------------------------------------------------------------------------------------------
// Requests (the third build, rule 2: the second build's demands)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_request_lapses_refused_and_is_told_as_a_request() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    g.houses[h].demand_rest_until = g.game_turn() + 1;
    turn(&mut g);
    let dm = g.houses[h].demand.expect("a request");
    assert_eq!(dm.deadline, g.game_turn() + g.d.defines.person_demand_turns);
    let label = g.house_label(h);
    assert!(logged(&g, &format!("{label} requests: ")), "told as a request");
    assert!(g.house_view(h).unwrap().demand.is_some());
    let before = g.house_loyalty(h);
    g.houses[h].demand.as_mut().unwrap().deadline = g.game_turn() + 1;
    turn(&mut g);
    assert_eq!(g.houses[h].demand, None, "lapsed");
    assert_eq!(g.house_loyalty(h), before - 3);
    assert!(mods(&g, h).contains(&("Request refused".into(), -3)));
    assert!(g.house_view(h).unwrap().parts.iter().any(|p| p.label == "Request refused" && p.amount == -3 && p.turns_left == Some(60)));
    assert!(logged(&g, &format!("{label}: request refused (")));
    assert!(g.houses[h].deeds.iter().any(|dd| dd.text.starts_with("Request refused: ")), "in its record");
    assert_eq!(g.house_view(h).unwrap().demand_rest_turns, g.d.defines.person_demand_rest);
}

#[test]
fn land_requests_the_nearest_crown_city_met_at_once_by_the_grant() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = land_house(&mut g, a);
    g.houses[h].demand_rest_until = 0;
    turn(&mut g);
    let dm = g.houses[h].demand.expect("a Land request");
    assert_eq!(dm.kind, DemandKind::GrantCity(plot(&g, c)), "C is nearest the seat");
    assert_eq!(g.demand_kind_text(h, dm.kind), format!("{} granted to {}.", g.d.text.name(&g.cities[c].name), g.house_label(h)));
    let preview = g.grant_preview(P0, c, GrantTo::House(h));
    assert!(preview.iter().any(|l| l.text == "Request met: +3 (60 turns)"), "{preview:?}");
    g.player_grant_city(P0, c, GrantTo::House(h)).expect("granted");
    assert_eq!(g.houses[h].demand, None, "met at once");
    assert!(mods(&g, h).contains(&("Request met".into(), 3)));
}

#[test]
fn a_grant_snubs_the_claimants() {
    let World { mut g, a, b, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h1 = land_house(&mut g, a);
    let h2 = house_led_by(&mut g, b, leader("LEADER_SERA_VELL"));
    let claimants = g.city_claimants(c);
    assert!(claimants.contains(&h1) && claimants.contains(&h2), "{claimants:?}");
    let cands = g.grant_candidates(P0, c);
    let to_h2 = cands.iter().find(|k| k.to == GrantTo::House(h2)).unwrap();
    assert!(to_h2.lines.iter().any(|l| l.subject == g.house_label(h1) && l.text.ends_with("-1 (30 turns)")), "{:?}", to_h2.lines);
    let before = g.house_loyalty(h1);
    g.player_grant_city(P0, c, GrantTo::House(h2)).expect("granted");
    assert_eq!(g.house_loyalty(h1), before - 1);
    assert!(g.houses[h1].deeds.iter().any(|dd| dd.text.starts_with("Passed over: ")));
}

#[test]
fn war_requests_war_then_a_city_and_peace_refuses_it() {
    let World { mut g, b, enemy, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = house_led_by(&mut g, b, leader_of(Ambition::War));
    assert_eq!(g.house_ambition(h), Ambition::War);
    g.houses[h].demand_rest_until = 0;
    turn(&mut g);
    assert_eq!(g.houses[h].demand.map(|d| d.kind), Some(DemandKind::War(P1)));
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    let preview = g.persons_preview(P0, &PersonsAct::DeclareWar(t1));
    assert!(preview.iter().any(|l| l.subject == g.house_label(h) && l.text == "Request met: +3 (60 turns)"), "{preview:?}");
    g.team_declare_war(t0, t1, false, None);
    turn(&mut g);
    assert_eq!(g.houses[h].demand, None, "met");
    for _ in 0..20 {
        turn(&mut g);
    }
    assert_eq!(g.houses[h].demand.map(|d| d.kind), Some(DemandKind::TakeCity(plot(&g, enemy))));
    let before = g.house_loyalty(h);
    g.team_make_peace(t0, t1, true);
    assert_eq!(g.houses[h].demand, None, "refused at once");
    assert_eq!(g.house_loyalty(h), before - 3);
}

#[test]
fn a_works_house_requests_land() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = house_led_by(&mut g, a, leader("LEADER_TOVIN_CINDERWRIGHT"));
    assert_eq!(g.house_ambition(h), Ambition::Works);
    g.houses[h].demand_rest_until = 0;
    turn(&mut g);
    assert_eq!(g.houses[h].demand.map(|d| d.kind), Some(DemandKind::GrantCity(plot(&g, c))), "the Land list");
}

// ---------------------------------------------------------------------------------------------
// The read API and saves
// ---------------------------------------------------------------------------------------------

#[test]
fn read_api_and_save_round_trip() {
    let World { mut g, a, c, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
    g.houses[h].demand_rest_until = 0;
    turn(&mut g);
    assert!(g.houses[h].demand.is_some());
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_assign_vassal(t0, t1, true);
    let hv = g.house_view(h).unwrap();
    assert_eq!(hv.player, g.houses[h].player);
    assert_eq!(hv.leader, g.house_leader(h));
    assert_eq!(hv.leader_name, leader_name(g.d, hv.leader.unwrap()));
    assert_eq!(hv.cities, g.house_cities(h));
    assert_eq!(hv.tribute.at.len(), 6);
    assert!(hv.levy.takes);
    assert_eq!(hv.free, Ok(()));
    assert!(hv.demand.is_some() && !hv.deeds.is_empty());
    let court = g.court_view(P0);
    assert_eq!(court.houses.len(), 2);
    assert_eq!(court.tribute_default, TRIBUTE_DEFAULT);
    assert!(court.founding.is_err(), "a realm with Houses founds none");
    assert_eq!(g.city_house_mark(hv.cities[0]).map(|m| (m.house, m.owner)), Some((h, P0)));
    assert_eq!(g.city_house_mark(c), None);
    // Save and load.
    let bytes = g.save();
    let g2 = Game::load(g.d, &bytes).expect("loads");
    assert_eq!(g2.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), g.houses.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>());
    assert_eq!(g2.players[P0].houses, g.players[P0].houses);
    assert_eq!(g2.players[P1].house, g.players[P1].house);
    assert_eq!(g2.players[hv.player].house, Some(h));
    assert_eq!(g2.players[P0].house_tribute, TRIBUTE_DEFAULT);
    assert_eq!(g2.num_players(), g.num_players());
    assert_eq!(g2.house_view(h), g.house_view(h));
}

// ---------------------------------------------------------------------------------------------
// The Crown's limit (rule 14 (a) to (d))
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
    g.player_grant_city(P0, b, GrantTo::NewHouse).expect("granted");
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

/// The Kuriotates (Sprawling: a city cap, here 2 full cities on a Duel map; rule 14 as revised
/// 2026-11-20): beyond the cap a city is a settlement, a city of radius 1 under every other rule
/// (it pays maintenance and counts in the Crown, after the full cities); the Crown's limit is the
/// usual 3; a House with a city cap holds only settlements: a full city cannot be granted to one,
/// and a city it founds is a settlement.
#[test]
fn the_kuriotates_houses_hold_only_settlements() {
    let World { mut g, cap, a, b, c, .. } = world_of("CIVILIZATION_KURIOTATES", "LEADER_CARDITH");
    let cap_of_full = g.players[P0].max_cities;
    assert_eq!(cap_of_full, 2, "Sprawling's 3, the Duel map's -1");
    assert!(!g.cities[cap].settlement && !g.cities[a].settlement);
    assert_eq!((g.cities[cap].plot_radius, g.cities[a].plot_radius), (3, 3));
    assert!(g.cities[b].settlement && g.cities[c].settlement, "beyond the cap: settlements");
    assert_eq!((g.cities[b].plot_radius, g.cities[c].plot_radius), (1, 1));
    assert_eq!(g.city_num_city_plots(b), 9);
    assert_eq!(g.player_full_cities(P0, None), 2);
    // A settlement pays maintenance as any city.
    assert!(g.city_calculate_num_cities_maintenance_times100(b) > 0);
    // The Crown's limit is the usual 3; settlements count, after the full cities.
    government(&mut g, "CIVIC_ARISTOCRACY");
    assert_eq!(g.player_crown_limit(P0), Some(3));
    assert_eq!(g.player_crown_cities(P0), vec![cap, a, b, c]);
    assert_eq!(g.player_crown_over(P0), vec![c]);
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0));
    // A full city cannot go to a capped House, new or standing.
    assert_eq!(g.player_check_grant(P0, a, GrantTo::NewHouse), Err(FULL_CITY_TO_CAPPED_HOUSE));
    assert_eq!(g.player_check_grant_city(P0, a), Err(FULL_CITY_TO_CAPPED_HOUSE), "no House could take it");
    // A settlement founds a House and stays a settlement.
    let c_plot = plot(&g, c);
    assert_eq!(g.player_check_grant(P0, c, GrantTo::NewHouse), Ok(()));
    let h = g.player_grant_city(P0, c, GrantTo::NewHouse).expect("granted");
    let hp = g.houses[h].player;
    let c2 = city_at(&g, c_plot);
    assert_eq!(g.cities[c2].owner, hp);
    assert!(g.cities[c2].settlement, "a settlement stays one");
    assert_eq!(g.cities[c2].plot_radius, 1);
    assert_eq!(g.players[hp].max_cities, cap_of_full, "the House is Kuriotate: the same cap");
    assert!(g.player_crown_over(P0).is_empty());
    assert_eq!(g.players[P0].total_maintenance, cached_maintenance(&g, P0));
    assert_eq!(g.player_check_grant(P0, a, GrantTo::House(h)), Err(FULL_CITY_TO_CAPPED_HOUSE));
    assert_eq!(g.player_check_grant(P0, b, GrantTo::House(h)), Ok(()));
    // Its request for land names only a city it can be granted: no full city.
    if let Some(DemandKind::GrantCity(q)) = g.house_next_demand(h) {
        assert_ne!(q, plot(&g, a), "a full city is never asked for");
    }
    // A city the House founds is a settlement, though it holds no full city.
    let new = g.player_init_city(hp, 11, 7, true, true);
    assert!(g.cities[new].settlement && g.cities[new].plot_radius == 1, "a capped House founds settlements");
    // The liege at its cap founds a settlement too.
    let own = g.player_init_city(P0, 15, 4, true, true);
    assert!(g.cities[own].settlement);
}

// ---------------------------------------------------------------------------------------------
// The world unit (the third build, rule 9)
// ---------------------------------------------------------------------------------------------

/// The world unit is the mean total commerce a turn, every commerce type, of the civilizations
/// that can have Houses: not a House (nor the barbarians); at least 1.
#[test]
fn the_world_unit_is_the_mean_commerce_of_the_civilizations() {
    let World { mut g, a, .. } = world();
    government(&mut g, "CIVIC_ARISTOCRACY");
    let h = g.player_grant_city(P0, a, GrantTo::NewHouse).unwrap();
    let hp = g.houses[h].player;
    let barbarian = g.barbarian_player();
    let set = |g: &mut Game, p: PlayerId, rates: [i32; CommerceType::NUM]| {
        g.players[p].commerce_rate = rates;
        g.players[p].disable_research = 0;
    };
    // (x100, as the rules keep it) gold 10, research 20, culture 5, espionage 3 (read as culture).
    set(&mut g, P0, [1000, 2000, 500, 300]);
    assert_eq!(g.player_total_commerce_rate(P0), 38);
    set(&mut g, P1, [1500, 0, 0, 0]);
    // The House and the barbarians, however rich, are not counted.
    set(&mut g, hp, [90_000, 90_000, 0, 0]);
    set(&mut g, barbarian, [90_000, 0, 0, 0]);
    assert_eq!(g.world_commerce_unit(), (38 + 15) / 2);
    // At least 1.
    set(&mut g, P0, [0; CommerceType::NUM]);
    set(&mut g, P1, [0; CommerceType::NUM]);
    assert_eq!(g.world_commerce_unit(), 1);
}
