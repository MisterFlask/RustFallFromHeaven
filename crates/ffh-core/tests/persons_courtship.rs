//! Houses, the third build's part 2B (docs/PERSONS.md, "The third build", rules 7, 8 and 10):
//! courtship (gifts of gold and resources, the requirement, the 20 turns of gold, the opening and
//! the 10 turns before an offer, the liege's memory), the price on the world unit, a realm's shared
//! knowledge (both ways, not as first discoverer, a team joining a realm), gold tribute, and a House
//! trading no tech. Every situation is built by hand.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::map::{Map, MapInitData};
use ffh_core::persons::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::TradeData;
use ffh_data::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const COURTED: MemoryType = MemoryType::MEMORY_COURTED_HOUSE;

/// A duel (the Bannor against the Elohim, no barbarians) on one flat block of grassland (x 2..=17,
/// y 3..=8) revealed to both, the starting units gone, the two teams met, player 0 under
/// Aristocracy (it may grant cities). Player 0 holds its capital at (3, 4) and cities A (7, 4), B
/// (11, 4) and C (7, 7); player 1 a city at (15, 7). Each civilization's commerce is 50 a turn
/// (the world unit 50).
struct World {
    g: Game,
    a: CityId,
    b: CityId,
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
    g.player_init_city(P0, 3, 4, true, true);
    let a = g.player_init_city(P0, 7, 4, true, true);
    let b = g.player_init_city(P0, 11, 4, true, true);
    g.player_init_city(P0, 7, 7, true, true);
    g.player_init_city(P1, 15, 7, true, true);
    let c = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
    let option = g.d.civics[c].civic_option_type.expect("a civic option");
    g.player_set_civics(P0, option, Some(c));
    for p in [P0, P1] {
        commerce(&mut g, p, 50);
    }
    World { g, a, b }
}

/// Player `p`'s total commerce a turn set to `n` (all of it gold; the rules keep it x100).
fn commerce(g: &mut Game, p: PlayerId, n: i32) {
    g.players[p].commerce_rate = [n * 100, 0, 0, 0];
}

/// A House of player 0 founded at city `seat` (a leader drawn as a founding draws it), without a
/// request, Cautious toward its liege (a modifier of -1 for 100 turns at a tribute of 10%): it
/// takes offers.
fn house(g: &mut Game, seat: CityId) -> HouseId {
    let civ = g.player_get_civilization_type(P0).expect("a civilization");
    let leader = g.house_leader_draw(civ, P0).expect("a free leader");
    let (h, _) = g.player_found_house(P0, leader, &[seat]).expect("founded");
    let t = g.game_turn();
    let house = &mut g.houses[h];
    house.demand = None;
    house.demand_rest_until = t + 1000;
    house.tribute = 10;
    house.tribute_now = 10;
    house.standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: -1, until: Some(t + 100) }];
    assert_eq!(g.house_state(h), LoyaltyState::Cautious);
    // (The founding recounted the liege's commerce from its cities: the world unit back to 50.)
    for p in [P0, P1] {
        commerce(g, p, 50);
    }
    h
}

fn gold(data: i32) -> TradeData {
    TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false }
}

fn item(t: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type: t, data, offering: false, hidden: false }
}

/// Player 1 gives House `h` `n` gold (a gift: nothing asked back), through a deal.
fn gift(g: &mut Game, h: HouseId, n: i32) {
    let hp = g.houses[h].player;
    g.players[P1].gold += n;
    g.implement_deal(P1, hp, &[gold(n)], &[], false);
}

/// The next game turn of player 0's Houses (its courtships checked).
fn turn(g: &mut Game) {
    let t = g.game_turn();
    g.set_game_turn(t + 1);
    g.player_persons_turn(P0);
}

fn logged(g: &Game, s: &str) -> bool {
    g.log.entries.iter().any(|e| e.text.contains(s))
}

// ---------------------------------------------------------------------------------------------
// Courtship (rule 7)
// ---------------------------------------------------------------------------------------------

/// The requirement is 3 turns of the world unit; gold given counts at once, and the courtship
/// opens the turn its value reaches the requirement: the liege is told and remembers the giver;
/// the House is not angered. The House card's line and the courter's line say where it stands.
#[test]
fn a_gift_opens_a_courtship_at_the_requirement() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    assert_eq!(g.world_commerce_unit(), 50);
    assert_eq!(g.courtship_requirement(), 150);
    assert_eq!(g.courtship_resource_worth(), 50);
    let loyalty = g.house_loyalty(h);
    // A gift is a deal the House accepts with nothing asked back: gold from its treasury to the
    // House's.
    g.players[P1].gold = 10;
    assert!(g.player_can_trade_item(P1, hp, &gold(10), false));
    assert!(!g.player_can_trade_item(P1, hp, &gold(11), false), "no more than its treasury");
    let before = g.players[hp].gold;
    gift(&mut g, h, 140);
    assert_eq!(g.players[hp].gold, before + 140);
    assert_eq!(g.house_courtship_value(h, P1), 140);
    assert_eq!(g.house_courtship_opened(h, P1), None, "below the requirement");
    let label = g.house_label(h);
    assert_eq!(g.house_courtship_refusal(h, P1), Some(format!("Court {label} first: gifts worth 150 (you have given 140).")));
    let cv = g.house_courtships(h);
    assert_eq!(cv.len(), 1);
    assert_eq!(cv[0].line, "Courted by the Elohim (gifts: 140 gold). Not open: 140 of 150.");
    assert_eq!(g.house_courting_line(h, P1), Some(format!("Courting {label}: 140 of 150 given.")));
    assert_eq!(g.players[P0].ai_memory_count[P1][COURTED], 0);
    // The gift that reaches it opens it this turn.
    let t = g.game_turn();
    gift(&mut g, h, 10);
    assert_eq!(g.house_courtship_opened(h, P1), Some(t));
    let from = t + g.d.defines.person_courtship_turns;
    assert!(logged(&g, &format!("The Elohim court {label}: offers possible from turn {from}.")), "the liege is told");
    assert_eq!(g.players[P0].ai_memory_count[P1][COURTED], 1, "the liege remembers it");
    assert_eq!(g.players[hp].ai_memory_count[P1][COURTED], 0, "the House, given gifts, does not");
    assert_eq!(g.house_loyalty(h), loyalty, "the House is not angered");
    assert_eq!(g.house_courtships(h)[0].line, format!("Courted by the Elohim (gifts: 150 gold). Offers possible from turn {from}."));
    assert_eq!(g.house_courtship_refusal(h, P1), Some(format!("Offers possible from turn {from}.")));
    // Its liege never courts it: no gift from its own team.
    g.players[P0].gold = 10;
    assert!(!g.player_can_trade_item(P0, hp, &gold(10), false));
    assert!(g.house_courter_refusal(h, P0).is_some());
}

/// A resource given counts a third of the requirement while it is given, and stops counting when
/// its trade ends: the courtship it held open closes. (The trade is made by force: this map has no
/// trade network between the two.)
#[test]
fn a_resource_counts_a_third_while_given() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let wine = g.d.t::<BonusType>("BONUS_WINE");
    g.implement_deal(P1, hp, &[item(TradeableItem::TRADE_RESOURCES, wine.idx() as i32)], &[], true);
    assert_eq!(g.house_gift_resources(h, P1), vec![wine]);
    assert_eq!(g.house_courtship_value(h, P1), 50, "a third of 150");
    let label = g.house_label(h);
    assert_eq!(g.house_courting_line(h, P1), Some(format!("Courting {label}: 50 of 150 given (Wine counts 50).")));
    gift(&mut g, h, 100);
    assert!(g.house_courtship_opened(h, P1).is_some(), "100 gold and the Wine reach 150");
    assert_eq!(g.house_courtships(h)[0].line.split(". ").next(), Some("Courted by the Elohim (gifts: 100 gold, Wine)"));
    // The trade cancelled: the Wine no longer counts and the courtship closes.
    let deal = g.deals.iter().find(|(_, d)| d.first_player == P1 && d.second_player == hp).map(|(id, _)| id).expect("the resource's deal");
    g.deal_kill(deal, true);
    assert_eq!(g.house_courtship_value(h, P1), 100);
    assert_eq!(g.house_courtship_opened(h, P1), None);
    assert!(g.houses[h].courtships.is_empty(), "closed");
}

/// Gold counts for 20 turns: a gift of turn t is gone at turn t + 20, and the courtship it held
/// open closes in the liege's turn; the requirement follows the world unit.
#[test]
fn gold_older_than_twenty_turns_drops_out() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let t = g.game_turn();
    gift(&mut g, h, 150);
    assert_eq!(g.house_courtship_opened(h, P1), Some(t));
    for _ in 0..19 {
        turn(&mut g);
    }
    assert_eq!(g.game_turn(), t + 19);
    assert_eq!(g.house_courtship_opened(h, P1), Some(t), "still counted at turn t + 19");
    turn(&mut g);
    assert_eq!(g.house_gift_gold(h, P1), 0);
    assert!(g.houses[h].gifts.is_empty(), "pruned");
    assert!(g.houses[h].courtships.is_empty(), "closed");
    // A richer world raises the requirement: a courtship open at 150 closes when the unit is 60.
    gift(&mut g, h, 150);
    assert!(g.house_courtship_opened(h, P1).is_some());
    for p in [P0, P1] {
        commerce(&mut g, p, 60);
    }
    assert_eq!(g.courtship_requirement(), 180);
    assert_eq!(g.house_courtship_opened(h, P1), None, "below the requirement now");
    turn(&mut g);
    assert!(g.houses[h].courtships.is_empty());
}

/// An offer (Cautious, its price covered) is refused before the courtship has been open 10 turns
/// and accepted after.
#[test]
fn an_offer_waits_for_ten_turns_of_courtship() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let label = g.house_label(h);
    let secede = item(TradeableItem::TRADE_SECEDE, 0);
    assert_eq!(g.house_bribe_terms_refusal(h, P1, BribeKind::Secede), None, "Cautious: its terms are met");
    assert_eq!(g.house_bribe_refusal(h, P1, BribeKind::Secede), Some(format!("Court {label} first: gifts worth 150 (you have given 0).")));
    assert!(!g.player_can_trade_item(hp, P1, &secede, false));
    let t = g.game_turn();
    gift(&mut g, h, 150);
    let from = t + 10;
    for k in 0..10 {
        assert_eq!(g.house_bribe_refusal(h, P1, BribeKind::Secede), Some(format!("Offers possible from turn {from}.")), "turn t + {k}");
        let price = g.house_bribe_price(h).unwrap();
        g.players[P1].gold = price;
        g.implement_deal(P1, hp, &[gold(price)], &[secede.clone()], false);
        assert!(g.houses.contains(h), "refused at turn t + {k}");
        turn(&mut g);
    }
    assert_eq!(g.game_turn(), from);
    assert_eq!(g.house_bribe_refusal(h, P1, BribeKind::Secede), None);
    let price = g.house_bribe_price(h).unwrap();
    g.players[P1].gold = price;
    g.implement_deal(P1, hp, &[gold(price)], &[secede], false);
    assert!(!g.houses.contains(h), "accepted: it broke away");
}

/// At war with the House's liege, a gift to the House makes no peace (its war is its liege's), and
/// it counts.
#[test]
fn a_gift_at_war_makes_no_peace() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_declare_war(t1, t0, false, None);
    let ht = g.players[hp].team;
    assert!(g.at_war(t1, ht));
    gift(&mut g, h, 150);
    assert!(g.at_war(t1, ht) && g.at_war(t1, t0), "still at war");
    assert!(g.house_courtship_opened(h, P1).is_some());
}

// ---------------------------------------------------------------------------------------------
// The price (rule 8)
// ---------------------------------------------------------------------------------------------

/// A House's price is `(loyalty + 6) x P x its cities`, P 54% of the world unit (at least 1).
#[test]
fn the_price_follows_the_world_unit() {
    let World { mut g, a, b, .. } = world();
    let h = house(&mut g, a);
    g.player_grant_city(P0, b, GrantTo::House(h)).expect("granted");
    let h_loyalty = g.house_loyalty(h);
    let cities = g.house_cities(h).len() as i32;
    assert_eq!(cities, 2);
    for (unit, step) in [(50, 27), (47, 25), (14, 7), (153, 82), (1, 1)] {
        for p in [P0, P1] {
            commerce(&mut g, p, unit);
        }
        assert_eq!(g.world_commerce_unit(), unit);
        assert_eq!(g.house_price_step(), step, "P at a world unit of {unit}");
        assert_eq!(g.house_bribe_price(h), Some((h_loyalty + 6).max(1) * step * cities));
    }
}

// ---------------------------------------------------------------------------------------------
// Shared knowledge, gold tribute, techs (rule 10)
// ---------------------------------------------------------------------------------------------

/// A tech a House gains reaches its liege and the liege's other Houses, and one the liege gains
/// reaches its Houses; the others gain it as not its first discoverer (only the discoverer's
/// capital gets the tech's free great person). A card of the deck is not shared.
#[test]
fn a_realm_shares_its_techs_not_as_first_discoverer() {
    let World { mut g, a, b, .. } = world();
    let h1 = house(&mut g, a);
    let h2 = house(&mut g, b);
    let (t0, t1) = (g.players[P0].team, g.players[g.houses[h1].player].team);
    let t2 = g.players[g.houses[h2].player].team;
    let enemy = g.players[P1].team;
    let mut realm = vec![t0, t1, t2];
    realm.sort();
    assert_eq!(g.team_realm(t1), realm);
    assert_eq!(g.team_realm(t0), realm);
    assert!(g.team_realm(enemy).is_empty(), "no realm without Houses");
    // House 1 discovers Drama first: its capital gets the Artist; its liege and House 2 learn it.
    let drama = g.d.t::<TechType>("TECH_DRAMA");
    let artist = g.d.t::<UnitClassType>("UNITCLASS_ARTIST");
    let artists = |g: &Game| g.units.iter().filter(|(_, u)| g.d.units[u.unit_type].unit_class_type == Some(artist)).map(|(_, u)| u.owner).collect::<Vec<_>>();
    assert!(artists(&g).is_empty());
    let hp1 = g.houses[h1].player;
    g.team_set_has_tech(t1, drama, true, Some(hp1), true, true);
    for t in [t0, t1, t2] {
        assert!(g.team_is_has_tech(t, Some(drama)), "{t:?}");
    }
    assert!(!g.team_is_has_tech(enemy, Some(drama)), "not beyond the realm");
    assert_eq!(artists(&g), vec![hp1], "the first discoverer's great person only");
    // The liege gains one: its Houses gain it.
    let tech = g.d.techs.keys().find(|&t| g.d.techs[t].card.is_none() && !g.d.techs[t].repeat && !g.d.techs[t].disable && !g.team_is_has_tech(t0, Some(t))).expect("a tech");
    g.team_set_has_tech(t0, tech, true, Some(P0), false, false);
    for t in [t1, t2] {
        assert!(g.team_is_has_tech(t, Some(tech)), "{t:?}");
    }
    // A card is one of a kind: not shared.
    let card = g.d.techs.keys().find(|&t| g.d.techs[t].card.is_some()).expect("a card");
    g.team_set_has_tech(t1, card, true, Some(hp1), false, false);
    assert!(g.team_is_has_tech(t1, Some(card)));
    assert!(!g.team_is_has_tech(t0, Some(card)) && !g.team_is_has_tech(t2, Some(card)));
}

/// A team that joins a realm (here a civilization that becomes a vassal by a deal: a House) and
/// the realm share what they know at once, both ways.
#[test]
fn a_team_joining_a_realm_shares_both_ways() {
    let World { mut g, a, .. } = world();
    let h1 = house(&mut g, a);
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    let th = g.players[g.houses[h1].player].team;
    let fresh = |g: &Game, t: TeamId| g.d.techs.keys().filter(|&x| g.d.techs[x].card.is_none() && !g.d.techs[x].repeat && !g.d.techs[x].disable && !g.team_is_has_tech(t, Some(x))).collect::<Vec<_>>();
    let theirs = fresh(&g, t0).into_iter().find(|&x| !g.team_is_has_tech(t1, Some(x)) && !g.team_is_has_tech(th, Some(x))).expect("a tech");
    g.team_set_has_tech(t1, theirs, true, Some(P1), false, false);
    let ours = fresh(&g, t1).into_iter().find(|&x| x != theirs && !g.team_is_has_tech(t0, Some(x))).expect("a tech");
    g.team_set_has_tech(t0, ours, true, Some(P0), false, false);
    assert!(g.team_is_has_tech(th, Some(ours)), "the House has it already (shared)");
    assert!(!g.team_is_has_tech(t0, Some(theirs)) && !g.team_is_has_tech(t1, Some(ours)));
    g.team_set_vassal(t1, t0, true, false);
    assert!(g.player_house(P1).is_some(), "a House now");
    for t in [t0, t1, th] {
        assert!(g.team_is_has_tech(t, Some(theirs)) && g.team_is_has_tech(t, Some(ours)), "{t:?}");
    }
}

/// Tribute is gold only: the House's gold commerce at the rate in force, from its treasury; no
/// research reaches its liege by it.
#[test]
fn tribute_pays_gold_only() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    g.houses[h].tribute = 50;
    g.houses[h].tribute_now = 50;
    g.players[hp].gold = 1000;
    let want = g.house_commerce(h) * 50 / 100;
    assert_eq!(g.house_tribute_at(h, 50), want);
    let liege = g.players[P0].gold;
    turn(&mut g);
    assert_eq!((g.houses[h].last_gold, g.players[P0].gold, g.players[hp].gold), (want, liege + want, 1000 - want));
    assert!(g.house_tribute_view(h).at.iter().all(|r| r.gold == g.house_tribute_at(h, r.pct)));
}

/// Techs are the liege's alone to trade: a House gives and takes none.
#[test]
fn a_house_trades_no_tech() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let (t1, th) = (g.players[P1].team, g.players[hp].team);
    for t in [t1, th] {
        g.teams[t].tech_trading_count = 1;
    }
    let enemy_tech = g.d.techs.keys().find(|&x| g.d.techs[x].trade && !g.team_is_has_tech(th, Some(x)) && g.player_can_research(hp, x, true)).expect("a tech");
    g.team_set_has_tech(t1, enemy_tech, true, Some(P1), false, false);
    let house_tech = g.d.techs.keys().find(|&x| g.d.techs[x].trade && g.team_is_has_tech(th, Some(x)) && !g.team_is_has_tech(t1, Some(x))).expect("a tech the House knows");
    assert!(!g.player_can_trade_item(P1, hp, &item(TradeableItem::TRADE_TECHNOLOGIES, enemy_tech.idx() as i32), false), "it takes none");
    assert!(!g.player_can_trade_item(hp, P1, &item(TradeableItem::TRADE_TECHNOLOGIES, house_tech.idx() as i32), false), "it gives none");
}
