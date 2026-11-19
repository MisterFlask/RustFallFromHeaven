//! The memory of wrongs (the owner's decisions of 2026-11-18; docs/PORTING.md, "Changes from
//! FFH2"; docs/PERSONS.md, "The second build"): a city taken in battle and a House bought are
//! remembered, an act against a House by its liege too, a refused offer by no one; a declared war
//! is remembered once by each player declared on, a House pulled in included; a House takes gold
//! only. Every situation is built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::persons::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::TradeData;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

const TOOK: MemoryType = MemoryType::MEMORY_TOOK_CITY;
const BOUGHT: MemoryType = MemoryType::MEMORY_BOUGHT_HOUSE;
const WAR: MemoryType = MemoryType::MEMORY_DECLARED_WAR;

/// Three civilizations on one block of grassland, all met, no barbarians: the Bannor (player 0)
/// with its capital at (3, 4) and city `own` at (3, 7); the Elohim (player 1), the Bannor's House
/// (a vassal by a deal) with cities `a` (9, 4) and `b` (9, 7); the Ljosalfar (player 2) with a
/// city at (15, 7). The House is Cautious toward its liege: it takes offers.
struct World {
    g: Game,
    own: CityId,
    a: CityId,
    b: CityId,
    h: HouseId,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 3, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    for (slot, (civ, leader)) in [("CIVILIZATION_BANNOR", "LEADER_SABATHIEL"), ("CIVILIZATION_ELOHIM", "LEADER_EINION"), ("CIVILIZATION_LJOSALFAR", "LEADER_ARENDEL")].into_iter().enumerate() {
        setup.players[slot].civ = Some(d.t::<CivilizationType>(civ));
        setup.players[slot].leader = Some(d.t::<LeaderHeadType>(leader));
    }
    let mut g = Game::new(d, setup);
    g.game_init();
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
    let teams: Vec<TeamId> = [P0, P1, P2].iter().map(|&p| g.players[p].team).collect();
    for &t in &teams {
        for &o in &teams {
            if t != o && !g.teams[t].has_met[o] {
                g.team_meet(t, o, false);
            }
        }
    }
    g.player_init_city(P0, 3, 4, true, true);
    let own = g.player_init_city(P0, 3, 7, true, true);
    let a = g.player_init_city(P1, 9, 4, true, true);
    let b = g.player_init_city(P1, 9, 7, true, true);
    g.player_init_city(P2, 15, 7, true, true);
    g.team_assign_vassal(teams[0], teams[1], false);
    let h = g.player_house(P1).expect("the Elohim are a House of the Bannor");
    assert_eq!(g.houses[h].owner, P0);
    // Cautious: its tribute's standing part 0 (10%), one modifier of -1.
    let turn = g.game_turn();
    g.houses[h].tribute = 10;
    g.houses[h].tribute_now = 10;
    g.houses[h].standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -1, until: Some(turn + 50) }];
    assert_eq!(g.house_state(h), LoyaltyState::Cautious);
    World { g, own, a, b, h }
}

fn memory(g: &Game, p: PlayerId, of: PlayerId, m: MemoryType) -> i32 {
    g.players[p].ai_memory_count[of][m]
}

fn trade(item_type: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type, data, offering: false, hidden: false }
}

#[test]
fn a_city_taken_in_battle_is_remembered_by_a_house_and_its_liege() {
    let World { mut g, own, a, b, .. } = world();
    // The House's city: the House and its liege remember it.
    g.player_acquire_city(P2, a, true, false, true);
    assert_eq!(memory(&g, P1, P2, TOOK), 1, "the House");
    assert_eq!(memory(&g, P0, P2, TOOK), 1, "its liege");
    assert_eq!(memory(&g, P2, P1, TOOK) + memory(&g, P2, P0, TOOK), 0, "not the taker");
    // The liege's own city: the liege only.
    g.player_acquire_city(P2, own, true, false, true);
    assert_eq!(memory(&g, P0, P2, TOOK), 2);
    assert_eq!(memory(&g, P1, P2, TOOK), 1, "a House does not take up its liege's wrongs");
    // A city traded (or granted, or broken away with) is no conquest.
    g.player_acquire_city(P2, b, false, true, true);
    assert_eq!((memory(&g, P1, P2, TOOK), memory(&g, P0, P2, TOOK)), (1, 2));
}

#[test]
fn a_declared_war_is_remembered_once_by_each_player_declared_on() {
    let World { mut g, .. } = world();
    let (t0, t1, t2) = (g.players[P0].team, g.players[P1].team, g.players[P2].team);
    g.team_declare_war(t2, t0, false, None);
    assert!(g.teams[t2].at_war[t1], "the House is pulled in by the vassal rules");
    assert_eq!(memory(&g, P0, P2, WAR), 1, "the liege, once: not again for its House");
    assert_eq!(memory(&g, P1, P2, WAR), 1, "the House");
    assert_eq!(memory(&g, P2, P0, WAR) + memory(&g, P2, P1, WAR), 0, "not the declarer");
}

#[test]
fn a_bought_house_is_remembered_by_its_liege_only() {
    let World { mut g, h, .. } = world();
    let hp = g.houses[h].player;
    // Courted first (docs/PERSONS.md, "The third build", rule 7): a gift that opens the courtship,
    // then its 10 turns; the liege remembers the courtship too.
    let need = g.courtship_requirement();
    g.players[P2].gold = need;
    g.implement_deal(P2, hp, &[trade(TradeableItem::TRADE_GOLD, need)], &[], false);
    assert_eq!(memory(&g, P0, P2, MemoryType::MEMORY_COURTED_HOUSE), 1, "the liege remembers the courtship");
    assert_eq!(memory(&g, hp, P2, MemoryType::MEMORY_COURTED_HOUSE), 0, "the House, given gifts, does not");
    let t = g.game_turn();
    g.set_game_turn(t + g.d.defines.person_courtship_turns);
    let price = g.house_bribe_price(h).expect("Cautious: it takes offers");
    g.players[P2].gold = price;
    g.implement_deal(P2, hp, &[trade(TradeableItem::TRADE_GOLD, price)], &[trade(TradeableItem::TRADE_SECEDE, 0)], false);
    assert!(!g.houses.contains(h), "it seceded");
    assert_eq!(memory(&g, P0, P2, BOUGHT), 1, "the liege remembers the buyer");
    assert_eq!(memory(&g, hp, P2, BOUGHT), 0, "the House was paid");
    assert_eq!(g.players[P0].ai_attitude_extra[P2], 0, "no attitude modifier");
}

/// A refused offer leaves no trace (the owner's decision, 2026-11-18): nothing remembered.
#[test]
fn a_refused_offer_is_remembered_by_no_one() {
    let World { mut g, h, .. } = world();
    let hp = g.houses[h].player;
    let price = g.house_bribe_price(h).unwrap();
    g.players[P2].gold = price;
    // Below its price (and this world's null controller refuses any offer).
    assert!(!g.player_offer_house(P2, hp, &[trade(TradeableItem::TRADE_GOLD, price - 1)], &[trade(TradeableItem::TRADE_SECEDE, 0)]));
    assert!(g.houses.contains(h));
    for p in [P0, P1] {
        assert_eq!(memory(&g, p, P2, BOUGHT), 0);
    }
}

/// A House takes gold, never gold per turn (the owner's decision, 2026-11-18): it could be paid
/// for a turn and ended by a war on the seceded House.
#[test]
fn a_house_takes_no_gold_per_turn() {
    let World { mut g, h, .. } = world();
    let hp = g.houses[h].player;
    g.players[P2].gold = 1000;
    let gpt = trade(TradeableItem::TRADE_GOLD_PER_TURN, 5);
    assert!(!g.player_can_trade_item(P2, hp, &gpt, false), "not tradeable with a House");
    assert!(g.player_can_trade_item(P2, hp, &trade(TradeableItem::TRADE_GOLD, 100), false));
    assert_eq!(house_offer_value(&[trade(TradeableItem::TRADE_GOLD, 100), gpt.clone()]), 100, "worth its gold only");
    // Offered gold per turn alone at any rate: refused, nothing moves.
    let get = [trade(TradeableItem::TRADE_SECEDE, 0)];
    assert!(!g.player_offer_house(P2, hp, &[trade(TradeableItem::TRADE_GOLD_PER_TURN, 1000)], &get));
    assert!(g.houses.contains(h));
    assert_eq!(memory(&g, P0, P2, BOUGHT), 0);
}
