//! Houses split; ultimatums and contracts replace the countdown (docs/PERSONS.md, "The third
//! build", rules 4 to 6): a split divides by distance and its cadet inherits nothing; no split
//! without a free leader; the turn's roll; each ultimatum kind chosen when it applies and skipped
//! when it cannot be met; an accepted ultimatum binds (each voiding act refused) and floors the
//! House at Annoyed; the contract's gold a turn; a new ultimatum after the contract. Every situation
//! is built by hand (no seed's story).

use std::sync::Arc;

use ffh_core::controller::Controller;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::persons::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::TradeData;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// A duel (the Bannor under Aristocracy against the Elohim, no barbarians) on one flat block of
/// grassland (x 2..=29, y 2..=11) revealed to both, the starting units gone, the two teams met.
/// Player 0 holds its capital at (3, 3), player 1 a city at (27, 10). Player 0 is the stronger.
fn world() -> (Game, CityId) {
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
    g.map = Map::new(d, &MapInitData { grid_w: 32, grid_h: 14, ..Default::default() }, np, nt);
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let land = (2..=11).contains(&y) && (2..=29).contains(&x);
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
    let cap = city(&mut g, P0, 3, 3);
    city(&mut g, P1, 27, 10);
    let aristocracy = d.t::<CivicType>("CIVIC_ARISTOCRACY");
    let option = d.civics[aristocracy].civic_option_type.expect("a civic option");
    g.player_set_civics(P0, option, Some(aristocracy));
    strength(&mut g);
    (g, cap)
}

/// Player 0 far stronger than player 1.
fn strength(g: &mut Game) {
    g.players[P0].power = 1000;
    g.players[P1].power = 10;
}

fn city(g: &mut Game, p: PlayerId, x: i32, y: i32) -> CityId {
    let c = g.player_init_city(p, x, y, true, true);
    for t in g.team_ids() {
        g.cities[c].revealed[t] = true;
    }
    c
}

fn plot_at(g: &Game, x: i32, y: i32) -> PlotIdx {
    g.map.plot_xy(x, y).expect("a plot")
}

fn owner_at(g: &Game, x: i32, y: i32) -> PlayerId {
    let c = g.map.plots[plot_at(g, x, y)].plot_city.expect("a city");
    g.cities[c].owner
}

/// A House of player 0 founded by the rules' founding with player 0's cities at `spots` (the first
/// its capital), its leader `leader` (else drawn), making no request.
fn house_at(g: &mut Game, spots: &[(i32, i32)], leader: Option<LeaderHeadType>) -> HouseId {
    let cities: Vec<CityId> = spots.iter().map(|&(x, y)| city(g, P0, x, y)).collect();
    let civ = g.player_get_civilization_type(P0).unwrap();
    let l = leader.unwrap_or_else(|| g.house_leader_draw(civ, P0).expect("a free leader"));
    let (h, _) = g.player_found_house(P0, l, &cities).expect("founded");
    g.houses[h].demand = None;
    g.houses[h].demand_rest_until = g.game_turn() + 1000;
    h
}

/// The next game turn's Houses' turn of player 0.
fn turn(g: &mut Game) {
    let t = g.game_turn();
    g.set_game_turn(t + 1);
    strength(g);
    g.player_persons_turn(P0);
}

/// House `h`'s loyalty as its parts make it set to `total` (one lasting grievance), at its tribute
/// in force.
fn set_raw(g: &mut Game, h: HouseId, total: i32) {
    g.houses[h].standing.mods.clear();
    let base = g.house_loyalty_raw(h);
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: total - base, until: None });
    assert_eq!(g.house_loyalty_raw(h), total);
}

fn in_force(g: &mut Game, h: HouseId, pct: i32) {
    g.houses[h].tribute = pct;
    g.houses[h].tribute_now = pct;
}

fn logged(g: &Game, s: &str) -> bool {
    g.log.entries.iter().any(|e| e.text.contains(s))
}

fn peace() -> TradeData {
    TradeData { item_type: TradeableItem::TRADE_PEACE_TREATY, data: 0, offering: false, hidden: false }
}

/// A civilization that accepts any offer (peace included).
struct Accepts;

impl Controller for Accepts {
    fn consider_offer(&self, _g: &Game, _p: PlayerId, _from: PlayerId, _they_give: &[TradeData], _we_give: &[TradeData]) -> bool {
        true
    }
}

fn ultimatum(g: &Game, h: HouseId, kind: UltimatumKind) -> Ultimatum {
    Ultimatum { house: h, kind, request: false, until: g.game_turn() + g.d.defines.person_ultimatum_turns }
}

fn refusal(g: &Game, h: HouseId) -> String {
    let until = g.houses[h].contract.expect("a contract").until;
    format!("Your contract with {} forbids it until turn {until}.", g.house_label(h))
}

// ---------------------------------------------------------------------------------------------
// Splits (rule 4)
// ---------------------------------------------------------------------------------------------

/// A House of six: its seat and the nearest half stay, the farther half goes to a cadet House of
/// its civilization, led by a minor leader drawn from the free ones, with the units in its cities;
/// the cadet inherits nothing and takes the realm's default tribute, in force.
#[test]
fn a_split_divides_by_distance_and_the_cadet_inherits_nothing() {
    let (mut g, _) = world();
    // Seat (6, 3); at 3 plots (9, 3) and (6, 6); (9, 6) at 4; (15, 3) at 9; (18, 3) at 12.
    let spots = [(6, 3), (9, 3), (6, 6), (9, 6), (15, 3), (18, 3)];
    let h = house_at(&mut g, &spots, None);
    let hp = g.houses[h].player;
    let w = g.d.t::<UnitType>("UNIT_WARRIOR");
    g.player_init_unit(hp, w, 18, 3, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    let stays = g.player_init_unit(hp, w, 6, 3, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
    // What the parent carries: a grievance, a refused request, ultimatums, a memory.
    let turn0 = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::Levied, amount: -1, until: Some(turn0 + 20) });
    g.houses[h].refused = Some(DemandKind::War(P1));
    g.houses[h].ultimatums = 2;
    g.players[hp].ai_memory_count[P1][MemoryType::MEMORY_TOOK_CITY] = 1;
    g.players[P0].house_tribute = 30;
    assert_eq!(g.house_split_cities(h), 6);
    assert_eq!(g.house_split_line(h), Some("Holds 6 cities: 5% a turn to split.".to_string()));
    let (keep, cadet) = g.house_split_division(h).expect("a division");
    let at = |g: &Game, cs: &[CityId]| -> Vec<(i32, i32)> {
        let mut v: Vec<(i32, i32)> = cs.iter().map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
        v.sort();
        v
    };
    assert_eq!(at(&g, &keep), vec![(6, 3), (6, 6), (9, 3)]);
    assert_eq!(at(&g, &cadet), vec![(9, 6), (15, 3), (18, 3)]);
    let parent = g.house_label(h);
    let nh = g.house_split(h).expect("split");
    let cp = g.houses[nh].player;
    for &(x, y) in &[(6, 3), (9, 3), (6, 6)] {
        assert_eq!(owner_at(&g, x, y), hp, "({x}, {y}) stays");
    }
    for &(x, y) in &[(9, 6), (15, 3), (18, 3)] {
        assert_eq!(owner_at(&g, x, y), cp, "({x}, {y}) goes to the cadet");
    }
    // The units in its cities go with them (converted: a unit of the cadet's, the parent's dies
    // with the delayed deaths); the others stay.
    assert!(g.map.plots[plot_at(&g, 18, 3)].units.iter().any(|&u| g.units[u].owner == cp), "the garrison went with the city");
    assert_eq!(g.units[stays].owner, hp);
    // The cadet: a House of the liege, of the parent's civilization, led by a free minor leader.
    let cadet = &g.houses[nh];
    assert_eq!((cadet.owner, cadet.origin), (P0, HouseOrigin::Split));
    assert!(g.teams[g.players[cp].team].vassal[g.players[P0].team]);
    assert_eq!(g.player_get_civilization_type(cp), g.player_get_civilization_type(hp));
    let l = g.house_leader(nh).expect("a leader");
    assert_ne!(Some(l), g.house_leader(h));
    assert_eq!(g.d.leader_heads[l].minor_leader_of, g.player_get_civilization_type(P0));
    assert!(g.house_label(nh).starts_with("House "));
    // It inherits nothing; its tribute is the realm's default, in force.
    assert!(cadet.standing.mods.is_empty(), "{:?}", cadet.standing.mods);
    assert_eq!((cadet.demand, cadet.contract, cadet.refused, cadet.ultimatums), (None, None, None, 0));
    assert_eq!((cadet.tribute, cadet.tribute_now), (30, 30));
    assert!(g.players[cp].ai_memory_count.iter().all(|row| row.iter().all(|&n| n == 0)), "no memory");
    // The parent keeps what it had, and remembers no wrong from the split.
    assert_eq!(g.houses[h].ultimatums, 2);
    assert_eq!(g.houses[h].refused, Some(DemandKind::War(P1)));
    assert!(g.houses[h].standing.mods.iter().any(|m| m.cause == LoyaltyCause::Levied));
    assert_eq!(g.players[hp].ai_memory_count[cp][MemoryType::MEMORY_TOOK_CITY], 0);
    assert_eq!(g.house_split_line(h), None, "3 cities");
    let child = g.house_label(nh);
    let name = leader_name(g.d, l);
    assert!(logged(&g, &format!("{parent} splits: {child}, led by {name}, takes ")), "told");
}

/// A grant that brings a House to the threshold says so; with no leader free (the liege's
/// civilization's minor leaders all leading its Houses, no alignment to fall back on), the House
/// does not split.
#[test]
fn no_split_without_a_free_leader() {
    let (mut g, _) = world();
    let civ = g.player_get_civilization_type(P0).unwrap();
    let pool = g.house_leader_pool(civ, P0).leaders;
    assert_eq!(pool.len(), 10);
    let big = house_at(&mut g, &[(6, 3), (9, 3), (12, 3), (15, 3), (18, 3)], Some(pool[0]));
    let sixth = city(&mut g, P0, 21, 3);
    let label = g.house_label(big);
    assert_eq!(g.house_split_line_after_grant(big, sixth), Some(format!("{label} will hold 6 cities: 5% a turn to split.")));
    g.player_grant_city(P0, sixth, GrantTo::House(big)).unwrap();
    for (i, &l) in pool[1..].iter().enumerate() {
        house_at(&mut g, &[(6 + 3 * i as i32, 9)], Some(l));
    }
    g.players[P0].alignment = None;
    assert!(g.house_leader_pool(civ, P0).leaders.is_empty());
    assert_eq!(g.house_split(big), None, "no leader is free");
    assert_eq!(g.house_split_cities(big), 6);
    assert_eq!(g.players[P0].houses.len(), 10);
}

/// The turn rolls for a House at the threshold (5% a turn), never for one below it.
#[test]
fn the_turns_roll_splits_a_house_at_the_threshold_only() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3), (9, 3), (12, 3), (15, 3), (18, 3)], None);
    for _ in 0..150 {
        turn(&mut g);
    }
    assert_eq!(g.players[P0].houses.len(), 1, "five cities: no roll");
    let sixth = city(&mut g, P0, 21, 3);
    g.player_grant_city(P0, sixth, GrantTo::House(h)).unwrap();
    let mut n = 0;
    while g.players[P0].houses.len() == 1 && n < 1000 {
        turn(&mut g);
        n += 1;
    }
    assert_eq!(g.players[P0].houses.len(), 2, "split within {n} turns");
    assert!(n > 0);
}

// ---------------------------------------------------------------------------------------------
// Ultimatums (rule 5): the first demand that applies, each only when it can be met now
// ---------------------------------------------------------------------------------------------

/// (a) Its refused request again, while `Request refused` runs and it can be met: the Crown city
/// granted; not a city of another, nor once the modifier ended.
#[test]
fn a_its_refused_request_again() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 10);
    let crown = city(&mut g, P0, 9, 6);
    let crown_plot = g.city_plot(crown);
    set_raw(&mut g, h, -7);
    let t = g.game_turn();
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: 0, until: Some(t + 60) });
    g.houses[h].refused = Some(DemandKind::GrantCity(crown_plot));
    let u = g.house_ultimatum(h).unwrap();
    assert_eq!((u.kind, u.request), (UltimatumKind::GrantCity(crown_plot), true));
    assert_eq!(u.until, t + 20);
    // A city of another civilization: it cannot be granted now.
    let enemy = g.map.plots[plot_at(&g, 27, 10)].plot_city.unwrap();
    g.houses[h].refused = Some(DemandKind::GrantCity(g.city_plot(enemy)));
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)));
    // War on that civilization (as (d)).
    g.houses[h].refused = Some(DemandKind::War(P1));
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::War(P1));
    // The modifier ended: no longer.
    g.houses[h].standing.mods.retain(|m| m.amount != 0);
    g.houses[h].standing.mods.iter_mut().for_each(|m| m.cause = LoyaltyCause::Levied);
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)));
    // Accepted: the city is granted, the request no longer asked.
    g.houses[h].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: 0, until: Some(t + 60) });
    g.houses[h].refused = Some(DemandKind::GrantCity(crown_plot));
    let u = g.house_ultimatum(h).unwrap();
    assert_eq!(g.house_ultimatum_accept(&u), UltimatumKind::GrantCity(crown_plot));
    assert_eq!(owner_at(&g, 9, 6), g.houses[h].player);
    assert_eq!(g.houses[h].refused, None);
}

/// (b) A lower tribute when the rate set is 20% or more: the highest whose loyalty once in force is
/// -4 or more; not below 20%.
#[test]
fn b_a_lower_tribute() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 30);
    // -6 at 30%: -5 at 20%, -4 at 10%.
    set_raw(&mut g, h, -6);
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Tribute(10));
    set_raw(&mut g, h, -9);
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Tribute(0), "no rate reaches -4: 0%");
    in_force(&mut g, h, 10);
    set_raw(&mut g, h, -6);
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)), "below 20%: no lower tribute");
}

/// (c) Peace with a civilization at war with the liege that took cities of the House's, when it
/// would accept peace now; (d) war on one the House remembers a wrong from, at peace, that the
/// liege can declare war on now.
#[test]
fn c_peace_and_d_war() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 10);
    set_raw(&mut g, h, -6);
    let hp = g.houses[h].player;
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    // (d): a war declared on it remembered.
    g.players[hp].ai_memory_count[P1][MemoryType::MEMORY_DECLARED_WAR] = 1;
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::War(P1));
    g.teams[t0].force_peace[t1] = true;
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)), "no war can be declared: skipped");
    g.teams[t0].force_peace[t1] = false;
    // (c): at war, a city taken remembered; the other side refuses peace (the default controller).
    g.team_declare_war(t0, t1, false, None);
    g.players[hp].ai_memory_count[P1][MemoryType::MEMORY_TOOK_CITY] = 1;
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)), "it would not accept peace: skipped");
    g.controllers[P1] = Arc::new(Accepts);
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Peace(P1));
    // Accepted: peace is made through the rules' peace path, and no war on it may be declared.
    let u = g.house_ultimatum(h).unwrap();
    assert_eq!(g.house_ultimatum_accept(&u), UltimatumKind::Peace(P1));
    assert!(!g.at_war(t0, t1));
    g.teams[t0].force_peace[t1] = false;
    assert!(!g.team_can_declare_war(t0, t1), "the contract forbids it");
    assert_eq!(g.team_contract_refusal(t0, ContractAct::DeclareWar(t1)), Some(refusal(&g, h)));
}

/// (e) A resource the liege has a spare source of and the House has none of; not without a spare
/// one.
#[test]
fn e_a_resource() {
    let (mut g, cap) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 10);
    set_raw(&mut g, h, -6);
    // A road from the capital to the House's seat: the trade networks join.
    let road = g.d.t::<RouteType>("ROUTE_ROAD");
    for x in 4..=5 {
        let q = plot_at(&g, x, 3);
        g.plot_set_route_type(q, Some(road), true);
    }
    g.update_plot_groups();
    let wine = g.d.t::<BonusType>("BONUS_WINE");
    g.city_change_free_bonus(cap, wine, 1);
    assert!(matches!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold(_)), "one source: none to spare");
    g.city_change_free_bonus(cap, wine, 1);
    assert_eq!(g.player_get_num_tradeable_bonuses(P0, wine), 2);
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Resource(wine));
    // Accepted: a trade from the liege to the House, which cannot be cancelled while it runs.
    let u = g.house_ultimatum(h).unwrap();
    g.house_ultimatum_accept(&u);
    let d = g.houses[h].contract.unwrap().deal.expect("the trade");
    assert!(g.deals.contains(d));
    assert_eq!(g.deal_contract_refusal(d), Some(refusal(&g, h)));
    let hp = g.houses[h].player;
    assert_eq!(g.player_get_num_available_bonuses(hp, wine), 1, "the House has it");
    for _ in 0..12 {
        turn(&mut g);
    }
    assert!(!g.deal_is_cancelable(d, Some(P0)), "past the treaty's turns, the contract holds it");
    for _ in 12..20 {
        turn(&mut g);
    }
    assert!(!g.deals.contains(d), "the contract's end ends the trade");
}

/// (f) Gold a turn, always: per House city the world unit's share, half again for each earlier
/// ultimatum, at most half the liege's total commerce, at least 1.
#[test]
fn f_gold_a_turn() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3), (9, 3)], None);
    in_force(&mut g, h, 10);
    set_raw(&mut g, h, -6);
    let unit = g.world_commerce_unit();
    let base = ((unit + ULTIMATUM_GOLD_ROUND) / ULTIMATUM_GOLD_DIVISOR).max(1);
    let cap = g.player_total_commerce_rate(P0) / 2;
    assert_eq!(g.house_ultimatum(h).unwrap().kind, UltimatumKind::Gold((base * 2).min(cap).max(1)));
    g.houses[h].ultimatums = 2;
    assert_eq!(g.ultimatum_gold(h), (base * 2 * 2).min(cap).max(1), "half again for each earlier one");
    // At turn 150's world unit (47): 2 a city; at turn 300's (153): 6.
    assert_eq!((47 + ULTIMATUM_GOLD_ROUND) / ULTIMATUM_GOLD_DIVISOR, 2);
    assert_eq!((153 + ULTIMATUM_GOLD_ROUND) / ULTIMATUM_GOLD_DIVISOR, 6);
}

// ---------------------------------------------------------------------------------------------
// Contracts (rule 6)
// ---------------------------------------------------------------------------------------------

/// An accepted tribute ultimatum binds: no rate set above it (the tribute list bars those rates),
/// and the House is at least Annoyed for every rule while it runs (its parts say why).
#[test]
fn a_contract_binds_the_tribute_and_floors_at_annoyed() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 30);
    set_raw(&mut g, h, -9);
    assert_eq!(g.house_state(h), LoyaltyState::Furious);
    let u = ultimatum(&g, h, UltimatumKind::Tribute(10));
    g.house_ultimatum_accept(&u);
    assert_eq!(g.houses[h].tribute, 10, "carried out");
    assert_eq!(g.player_check_house_tribute(P0, h, 20), Err(refusal(&g, h)));
    assert_eq!(g.player_set_house_tribute(P0, h, 50), Err(refusal(&g, h)));
    assert_eq!(g.player_check_house_tribute(P0, h, 0), Ok(()));
    let barred: Vec<(i32, bool)> = g.house_tribute_view(h).at.iter().map(|r| (r.pct, r.barred)).collect();
    assert_eq!(barred, vec![(0, false), (10, false), (20, true), (30, true), (40, true), (50, true)]);
    // The floor: -9 as its parts make it, Annoyed for the rules.
    assert_eq!(g.house_loyalty_raw(h), -9);
    assert_eq!((g.house_loyalty(h), g.house_state(h)), (-5, LoyaltyState::Annoyed));
    assert!(g.house_levy_refusal(h).is_some(), "Annoyed: no levy");
    let parts = g.house_parts(h);
    let line = format!("Contract until turn {}: at least Annoyed", u.until);
    assert!(parts.iter().any(|p| p.label == line && p.amount == 4), "{parts:?}");
    assert_eq!(parts.iter().map(|p| p.amount).sum::<i32>(), -5, "the parts add up to the loyalty");
    assert_eq!(g.house_contract_view(h).unwrap().line, format!("Contract until turn {}: no tribute above 10%.", u.until));
    // No ultimatum while it runs.
    turn(&mut g);
    assert_eq!(g.houses[h].contract.map(|c| c.until), Some(u.until));
}

/// War, religion and gold contracts: no peace with the civilization war was declared on (the
/// trade item refused), no change of state religion, the gold a turn paid as the liege's expense.
#[test]
fn a_contract_binds_peace_religion_and_pays_gold() {
    let (mut g, cap) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    let hp = g.houses[h].player;
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    // War.
    let u = ultimatum(&g, h, UltimatumKind::War(P1));
    g.house_ultimatum_accept(&u);
    assert!(g.at_war(t0, t1), "war declared");
    assert!(!g.player_can_trade_item(P0, P1, &peace(), false));
    assert!(!g.player_can_trade_item(P1, P0, &peace(), false));
    assert_eq!(g.player_contract_refusal(P0, ContractAct::MakePeace(t1)), Some(refusal(&g, h)));
    let preview = g.persons_preview(P0, &PersonsAct::MakePeace(t1));
    assert!(preview.iter().any(|l| l.text.starts_with("Contract until turn ")), "{preview:?}");
    g.houses[h].contract = None;
    assert!(g.player_can_trade_item(P0, P1, &peace(), false), "without the contract");
    // A state religion.
    let (r1, r2) = (g.d.t::<ReligionType>("RELIGION_THE_ORDER"), g.d.t::<ReligionType>("RELIGION_RUNES_OF_KILMORPH"));
    g.city_set_has_religion(cap, r1, true, false, false);
    g.city_set_has_religion(cap, r2, true, false, false);
    let u = ultimatum(&g, h, UltimatumKind::AdoptReligion(r1));
    assert!(g.player_can_convert(P0, Some(r1)), "The Order can be adopted");
    g.house_ultimatum_accept(&u);
    assert_eq!(g.player_get_state_religion(P0), Some(r1));
    g.players[P0].conversion_timer = 0;
    g.players[P0].anarchy_turns = 0;
    assert!(!g.player_can_convert(P0, Some(r2)), "the contract forbids a change");
    assert!(!g.player_can_convert(P0, None));
    assert_eq!(g.player_contract_refusal(P0, ContractAct::ChangeReligion), Some(refusal(&g, h)));
    g.houses[h].contract = None;
    assert!(g.player_can_convert(P0, None), "without the contract");
    // Gold a turn: the liege's expense, the House's income, while it runs.
    let before = g.player_calculate_base_net_gold(P0);
    let u = ultimatum(&g, h, UltimatumKind::Gold(5));
    g.house_ultimatum_accept(&u);
    assert_eq!(g.players[P0].gold_per_turn_by_player[hp], -5);
    assert_eq!(g.players[hp].gold_per_turn_by_player[P0], 5);
    assert_eq!(g.player_calculate_base_net_gold(P0), before - 5);
    g.players[P0].gold = 100;
    let rate = g.player_calculate_gold_rate(P0);
    g.player_do_gold(P0);
    assert_eq!(g.players[P0].gold, 100 + rate, "paid each turn with the rest of the budget");
    for _ in 0..20 {
        turn(&mut g);
    }
    assert_eq!(g.houses[h].contract, None, "ended");
    assert_eq!((g.players[P0].gold_per_turn_by_player[hp], g.players[hp].gold_per_turn_by_player[P0]), (0, 0));
    assert!(logged(&g, &format!("{}: the contract ends.", g.house_label(h))));
}

/// The contract ends after its turns; a House still Furious then issues a new ultimatum (its gold
/// higher by half for the earlier one).
#[test]
fn a_new_ultimatum_after_the_contract() {
    let (mut g, _) = world();
    let h = house_at(&mut g, &[(6, 3)], None);
    in_force(&mut g, h, 10);
    set_raw(&mut g, h, -9);
    turn(&mut g);
    let first = g.houses[h].contract.expect("accepted (the rules' default controller)");
    assert_eq!(g.houses[h].ultimatums, 1);
    assert_eq!(first.until, g.game_turn() + 20);
    for _ in 0..19 {
        turn(&mut g);
        assert_eq!(g.houses[h].contract, Some(first), "it runs");
    }
    turn(&mut g);
    assert_eq!(g.houses[h].ultimatums, 2, "Furious again: a new ultimatum");
    let second = g.houses[h].contract.expect("a new contract");
    assert_eq!(second.until, g.game_turn() + 20);
}
