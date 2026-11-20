//! Houses courted by culture (docs/PERSONS.md, "The fourth build", rules 2 to 5 and 7; the third
//! build's rule 10): favour (its gain, fade and settling), the requirement and its parts (L, A, W),
//! the world culture unit, the split toward the courter (the half nearest it, never the capital, a
//! House of one city whole, the new House the courter's with a drawn leader, no war, the memory,
//! the favour reset), the hold contest, no split without a free leader, the guard, the Crown's
//! grudge, gifts (an ordinary gift; the side deals gone); and a realm's shared knowledge, gold
//! tribute, a House trading no tech. Every situation is built by hand.

use std::sync::Arc;

use ffh_core::controller::Controller;
use ffh_core::culture::{ChannelOrder, ChannelTarget};
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
const COURTED: MemoryType = MemoryType::MEMORY_COURTED_HOUSE;
const SPLIT: MemoryType = MemoryType::MEMORY_SPLIT_HOUSE;
const CULTURE: usize = 2;

/// A duel (the Bannor against the Elohim, no barbarians; with `three`, the Ljosalfar as player 2)
/// on one flat block of grassland (x 2..=17, y 3..=8) revealed to all, the starting units gone, the
/// teams met, player 0 under Aristocracy (it may grant cities). Player 0 holds its capital at (3,
/// 4) and cities A (7, 4), B (11, 4) and C (7, 7); player 1 a city at (15, 7); player 2 a city at
/// (15, 4). Each civilization's commerce is 50 gold a turn (the world commerce unit 50) and 20
/// culture (the world culture unit 20).
struct World {
    g: Game,
    a: CityId,
    b: CityId,
    c: CityId,
}

fn world() -> World {
    world_of(false)
}

fn world_of(three: bool) -> World {
    let d = ffh_data::global();
    let n = if three { 3 } else { 2 };
    let mut setup = GameSetup::standard(d, n, "WORLDSIZE_DUEL", 5);
    setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    let civs = [("CIVILIZATION_BANNOR", "LEADER_SABATHIEL"), ("CIVILIZATION_ELOHIM", "LEADER_EINION"), ("CIVILIZATION_LJOSALFAR", "LEADER_ARENDEL")];
    for (slot, (civ, leader)) in civs.into_iter().take(n).enumerate() {
        setup.players[slot].civ = Some(d.t::<CivilizationType>(civ));
        setup.players[slot].leader = Some(d.t::<LeaderHeadType>(leader));
    }
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
    let players: Vec<PlayerId> = (0..n).map(|i| PlayerId(i as _)).collect();
    for &p in &players {
        for &o in &players {
            let (tp, to) = (g.players[p].team, g.players[o].team);
            if tp != to && !g.teams[tp].has_met[to] {
                g.team_meet(tp, to, false);
            }
        }
    }
    g.player_init_city(P0, 3, 4, true, true);
    let a = g.player_init_city(P0, 7, 4, true, true);
    let b = g.player_init_city(P0, 11, 4, true, true);
    let c = g.player_init_city(P0, 7, 7, true, true);
    g.player_init_city(P1, 15, 7, true, true);
    if three {
        g.player_init_city(P2, 15, 4, true, true);
    }
    let civic = g.d.t::<CivicType>("CIVIC_ARISTOCRACY");
    let option = g.d.civics[civic].civic_option_type.expect("a civic option");
    g.player_set_civics(P0, option, Some(civic));
    for &p in &players {
        commerce(&mut g, p, 50);
    }
    World { g, a, b, c }
}

/// Player `p`'s total commerce a turn set to `n` gold and 20 culture (the rules keep it x100).
fn commerce(g: &mut Game, p: PlayerId, n: i32) {
    g.players[p].commerce_rate = [n * 100, 0, 2000, 0];
}

/// Player `p`'s culture a turn set to `n` (the world culture unit reads it).
fn culture(g: &mut Game, p: PlayerId, n: i32) {
    g.players[p].commerce_rate[CULTURE] = n * 100;
}

/// House `h`'s loyalty set to `amount` (one modifier for 100 turns, tribute 10%: its part 0).
fn loyalty(g: &mut Game, h: HouseId, amount: i32) {
    let t = g.game_turn();
    let house = &mut g.houses[h];
    house.tribute = 10;
    house.tribute_now = 10;
    house.standing.mods = vec![LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount, until: Some(t + 100) }];
}

/// A House of player 0 founded at city `seat` and granted `more` (a leader drawn as a founding
/// draws it), without a request, Cautious toward its liege (loyalty -1 at a tribute of 10%).
fn house_with(g: &mut Game, seat: CityId, more: &[CityId]) -> HouseId {
    let civ = g.player_get_civilization_type(P0).expect("a civilization");
    let leader = g.house_leader_draw(civ, P0).expect("a free leader");
    let (h, _) = g.player_found_house(P0, leader, &[seat]).expect("founded");
    for &c in more {
        g.player_grant_city(P0, c, GrantTo::House(h)).expect("granted");
    }
    let t = g.game_turn();
    let house = &mut g.houses[h];
    house.demand = None;
    house.demand_rest_until = t + 1000;
    loyalty(g, h, -1);
    assert_eq!(g.house_state(h), LoyaltyState::Cautious);
    // (The founding and the grants recounted the liege's commerce from its cities.)
    for p in g.player_ids().filter(|&p| g.player_has_houses(p)).collect::<Vec<_>>() {
        commerce(g, p, 50);
    }
    h
}

fn house(g: &mut Game, seat: CityId) -> HouseId {
    house_with(g, seat, &[])
}

fn gold(data: i32) -> TradeData {
    TradeData { item_type: TradeableItem::TRADE_GOLD, data, offering: false, hidden: false }
}

fn item(t: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type: t, data, offering: false, hidden: false }
}

/// `courter`'s Court order at House `h`.
fn court(g: &Game, h: HouseId) -> ChannelOrder {
    ChannelOrder { action: g.d.t::<CulturalActionType>("CULTURALACTION_COURT_HOUSE"), target: ChannelTarget::House(h), religion: None }
}

/// `p`'s Guard order at House `h`.
fn guard(g: &Game, h: HouseId) -> ChannelOrder {
    ChannelOrder { action: g.d.t::<CulturalActionType>("CULTURALACTION_GUARD_HOUSE"), target: ChannelTarget::House(h), religion: None }
}

/// House `h`'s favour toward `courter` set to `amount` hundredths.
fn set_favour(g: &mut Game, h: HouseId, courter: PlayerId, amount: i32) {
    let list = &mut g.houses[h].favour;
    list.retain(|f| f.courter != courter);
    list.push(Favour { courter, amount, warned: false });
    list.sort_by_key(|f| f.courter);
}

/// The next game turn of player 0's Houses (their favour stepped).
fn turn(g: &mut Game) {
    let t = g.game_turn();
    g.set_game_turn(t + 1);
    g.player_persons_turn(P0);
}

fn logged(g: &Game, s: &str) -> bool {
    g.log.entries.iter().any(|e| e.text.contains(s))
}

/// The city on the plot of `c` now (a city changing hands has a new id).
fn at(g: &Game, plot: PlotIdx) -> CityId {
    g.map.plots[plot].plot_city.expect("a city")
}

/// A controller whose leader has a set attitude toward everyone, and accepts any gift.
struct Attitude(AttitudeType);
impl Controller for Attitude {
    fn attitude(&self, _g: &Game, _p: PlayerId, _towards: PlayerId) -> AttitudeType {
        self.0
    }
    fn consider_offer(&self, _g: &Game, _p: PlayerId, _from: PlayerId, _they_give: &[TradeData], we_give: &[TradeData]) -> bool {
        we_give.is_empty()
    }
}

/// A controller whose leader is Pleased with whoever gave it a gift that still counts, else
/// Cautious (the AI's `gifts` relation in short), and accepts any gift, valuing gold at its sum.
struct GiftsPlease;
impl Controller for GiftsPlease {
    fn attitude(&self, g: &Game, p: PlayerId, towards: PlayerId) -> AttitudeType {
        if g.gifts_value(p, towards) > 0 {
            AttitudeType::ATTITUDE_PLEASED
        } else {
            AttitudeType::ATTITUDE_CAUTIOUS
        }
    }
    fn consider_offer(&self, _g: &Game, _p: PlayerId, _from: PlayerId, _they_give: &[TradeData], we_give: &[TradeData]) -> bool {
        we_give.is_empty()
    }
    fn trade_value(&self, _g: &Game, _p: PlayerId, _other: PlayerId, item: &TradeData, _p_gets: bool) -> Option<i32> {
        Some(item.data)
    }
}

// ---------------------------------------------------------------------------------------------
// The world culture unit, the requirement (rule 3)
// ---------------------------------------------------------------------------------------------

/// W is the median culture a turn of the civilizations that can have Houses (the lower middle of
/// an even count), at least 1; a House's culture does not count.
#[test]
fn the_world_culture_unit_is_the_median() {
    let World { mut g, a, .. } = world_of(true);
    let h = house(&mut g, a);
    for (p, n) in [(P0, 10), (P1, 30), (P2, 20)] {
        culture(&mut g, p, n);
    }
    let hp = g.houses[h].player;
    culture(&mut g, hp, 1000);
    assert_eq!(g.world_culture_unit(), 20, "the middle of 10, 20 and 30");
    culture(&mut g, P2, 0);
    assert_eq!(g.world_culture_unit(), 10, "the middle of 0, 10 and 30");
    let World { mut g, .. } = world();
    culture(&mut g, P0, 10);
    culture(&mut g, P1, 30);
    assert_eq!(g.world_culture_unit(), 10, "the lower middle of two");
    culture(&mut g, P0, 0);
    culture(&mut g, P1, 0);
    assert_eq!(g.world_culture_unit(), 1, "at least 1");
}

/// The requirement is `(L + 6 − 2A, at least 1) × W`: the House's loyalty to its liege, its
/// leader's attitude toward the courter (−2 to +2), the world culture unit.
#[test]
fn the_requirement_reads_loyalty_attitude_and_the_world_unit() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    assert_eq!(g.house_requirement(h, P1), Requirement { total: 100, loyalty: -1, attitude: 0, unit: 20 }, "Cautious, A 0, W 20: 5 W");
    loyalty(&mut g, h, 3);
    assert_eq!(g.house_requirement(h, P1).total, 9 * 20, "Friendly toward its liege: 9 W");
    culture(&mut g, P0, 30);
    culture(&mut g, P1, 40);
    assert_eq!(g.house_requirement(h, P1).total, 9 * 30, "W follows the world");
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_FRIENDLY));
    assert_eq!(g.house_requirement(h, P1), Requirement { total: 5 * 30, loyalty: 3, attitude: 2, unit: 30 }, "its leader Friendly with the courter: 4 less");
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_FURIOUS));
    assert_eq!(g.house_requirement(h, P1).total, 13 * 30, "Furious with the courter: 4 more");
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_FRIENDLY));
    loyalty(&mut g, h, -6);
    assert_eq!(g.house_requirement(h, P1).total, 30, "at least 1 × W");
}

/// A gift to a House is an ordinary gift: recorded on the House (`implement_proposal`), it raises
/// its leader's attitude toward the giver (here a controller that reads it as the AI's `gifts`
/// relation does), which lowers the requirement. Its liege gives it none.
#[test]
fn a_gift_lowers_the_requirement_through_the_attitude() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    g.controllers[hp] = Arc::new(GiftsPlease);
    let before = g.house_requirement(h, P1);
    g.players[P1].gold = 100;
    let treasury = g.players[hp].gold;
    assert!(g.player_offer_house(P1, hp, &[gold(100)], &[]), "a gift");
    assert_eq!(g.players[hp].gold, treasury + 100);
    assert!(g.gifts_value(hp, P1) > 0, "recorded as a gift");
    let after = g.house_requirement(h, P1);
    assert_eq!((before.attitude, after.attitude), (0, 1));
    assert_eq!(after.total, before.total - 2 * before.unit);
    // Nothing but a gift: a House gives nothing.
    g.players[P1].gold = 100;
    assert!(!g.player_offer_house(P1, hp, &[gold(10)], &[gold(1)]));
    assert_eq!(g.house_gift_refusal(h, P0), Some(format!("{} pays its liege by tribute.", g.house_label(h))));
}

/// The trade table's read of what a better attitude brings (`house_requirement_steps`): the
/// requirement at the attitude now and at each better one up to Friendly, from the same parts
/// (`Requirement::at`: `(L + 6 − 2A, at least 1) × W`).
#[test]
fn the_requirement_at_each_better_attitude() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let r = g.house_requirement(h, P1);
    assert_eq!(r.at(r.attitude), r.total);
    assert_eq!(
        g.house_requirement_steps(h, P1),
        vec![(AttitudeType::ATTITUDE_CAUTIOUS, 5 * 20), (AttitudeType::ATTITUDE_PLEASED, 3 * 20), (AttitudeType::ATTITUDE_FRIENDLY, 20)],
        "Cautious (L -1, A 0, W 20), then Pleased and Friendly"
    );
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_FURIOUS));
    let steps = g.house_requirement_steps(h, P1);
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0], (AttitudeType::ATTITUDE_FURIOUS, 9 * 20));
    assert_eq!(steps[0].1, g.house_requirement(h, P1).total, "the first is the requirement now");
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_FRIENDLY));
    assert_eq!(g.house_requirement_steps(h, P1), vec![(AttitudeType::ATTITUDE_FRIENDLY, 20)], "Friendly: no better step");
    // At least 1 × W at every step.
    loyalty(&mut g, h, -6);
    g.controllers[hp] = Arc::new(Attitude(AttitudeType::ATTITUDE_CAUTIOUS));
    assert!(g.house_requirement_steps(h, P1).iter().all(|&(_, t)| t == 20));
}

/// The side deals are gone: no Secede, no Become your vassal; a House gives nothing and takes only
/// gold (no gold per turn) and resources, not from its liege.
#[test]
fn a_house_trades_only_gifts() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    assert!(!TradeableItem::NAMES.iter().any(|n| *n == "TRADE_SECEDE" || *n == "TRADE_HOUSE_VASSAL"), "the side deals are gone");
    g.players[hp].gold = 100;
    g.players[P1].gold = 100;
    assert!(!g.player_can_trade_item(hp, P1, &gold(10), false), "it gives nothing");
    assert!(!g.player_can_trade_item(hp, P1, &item(TradeableItem::TRADE_MAPS, 0), false));
    assert!(g.player_can_trade_item(P1, hp, &gold(10), false));
    assert!(!g.player_can_trade_item(P1, hp, &gold(101), false), "no more than its treasury");
    assert!(!g.player_can_trade_item(P1, hp, &item(TradeableItem::TRADE_GOLD_PER_TURN, 5), false), "no gold per turn");
    assert!(!g.player_can_trade_item(P1, hp, &item(TradeableItem::TRADE_OPEN_BORDERS, 0), false));
    g.players[P0].gold = 100;
    assert!(!g.player_can_trade_item(P0, hp, &gold(10), false), "not from its liege");
}

/// At war with the House's liege, a gift to the House makes no peace (its war is its liege's).
#[test]
fn a_gift_at_war_makes_no_peace() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let (t0, t1) = (g.players[P0].team, g.players[P1].team);
    g.team_declare_war(t1, t0, false, None);
    let ht = g.players[hp].team;
    assert!(g.at_war(t1, ht));
    g.players[P1].gold = 150;
    g.implement_deal(P1, hp, &[gold(150)], &[], false);
    assert!(g.at_war(t1, ht) && g.at_war(t1, t0), "still at war");
    assert_eq!(g.players[hp].gold, 150);
}

// ---------------------------------------------------------------------------------------------
// Favour (rule 2)
// ---------------------------------------------------------------------------------------------

/// Favour gains the courter's inflow times its reach in the liege's turn, then fades 5% (at least a
/// hundredth), and settles near nineteen times the inflow; a channel stopped, it fades away. The
/// liege and the courter are told once it passes two thirds of the requirement.
#[test]
fn favour_gains_fades_and_settles() {
    let World { mut g, a, b, .. } = world();
    let h = house_with(&mut g, a, &[b]);
    g.player_set_culture_channel(P1, Some(court(&g, h))).expect("P1 courts the House");
    assert_eq!(g.culture_house_reach(P1, h).map(|r| (r.distance, r.effect)), Some((5, 100)), "B 5 plots from P1's city: in full");
    g.players[P1].culture_inflow = 1000;
    turn(&mut g);
    assert_eq!(g.house_favour(h, P1), 950, "1000 less 5%");
    turn(&mut g);
    assert_eq!(g.house_favour(h, P1), 1950 - 97);
    let settles = g.favour_settles(1000, 0);
    assert!((18_900..=19_000).contains(&settles), "settles near 19 × the inflow: {settles}");
    // The courtship's line.
    let cv = &g.house_courtships(h)[0];
    assert_eq!((cv.courter, cv.requirement.total), (P1, 100));
    assert_eq!(cv.gain, g.player_culture_inflow_now(P1), "the gain at the inflow now (in full: 5 plots)");
    assert!(cv.line.starts_with("Courted by the Elohim: favour 18 of 100"), "{}", cv.line);
    // Two thirds: told once.
    let label = g.house_label(h);
    set_favour(&mut g, h, P1, 7000);
    turn(&mut g);
    assert!(logged(&g, &format!("{label}: the Elohim's favour is ")), "the liege is told");
    assert!(g.houses[h].favour[0].warned);
    // The channel stops: the favour fades to nothing, and goes.
    g.player_set_culture_channel(P1, None).unwrap();
    let f = g.house_favour(h, P1);
    turn(&mut g);
    assert_eq!(g.house_favour(h, P1), f - f * 5 / 100, "it keeps its favour while it fades");
    for _ in 0..400 {
        turn(&mut g);
    }
    assert!(g.houses[h].favour.is_empty(), "faded away");
}

/// A guard counts against every courter of the House: each one's favour loses the guard's inflow
/// times its reach, before it fades.
#[test]
fn guard_subtracts_from_every_courter() {
    let World { mut g, a, b, .. } = world_of(true);
    let h = house_with(&mut g, a, &[b]);
    for p in [P1, P2] {
        g.player_set_culture_channel(p, Some(court(&g, h))).expect("courts");
        g.players[p].culture_inflow = 1000;
    }
    g.player_set_culture_channel(P0, Some(guard(&g, h))).expect("the liege guards it");
    let reach = g.culture_house_reach(P0, h).expect("a reach").effect;
    g.players[P0].culture_inflow = 400;
    let loss = 400 * reach / 100;
    turn(&mut g);
    for p in [P1, P2] {
        let f = 1000 - loss;
        assert_eq!(g.house_favour(h, p), f - (f * 5 / 100).max(1), "player {p:?}");
    }
    let gv = g.house_guard_view(h).expect("a guard");
    assert_eq!(gv.courtships.len(), 2);
    assert!(gv.line.starts_with("Guarded: "), "{}", gv.line);
    // A guard that outweighs a courtship leaves nothing of it.
    g.players[P0].culture_inflow = 100_000;
    turn(&mut g);
    assert!(g.houses[h].favour.iter().all(|f| f.amount == 0) || g.houses[h].favour.is_empty());
}

/// The tooltips' calculations (docs/PLAY_DESIGN.md, "Calculated numbers") show the numbers the
/// rules use: the courtship's requirement, gain, guard and next turn, the guard's product, the
/// inflow by city, the reach's falloff and the requirement at each better attitude.
#[test]
fn the_calculations_show_the_rules_numbers() {
    let World { mut g, a, b, .. } = world_of(true);
    let h = house_with(&mut g, a, &[b]);
    for (p, rate) in [(P0, 900), (P1, 1100)] {
        for c in g.players[p].cities.clone() {
            g.cities[c].commerce_rate[CULTURE] = rate;
        }
    }
    g.player_set_culture_channel(P1, Some(court(&g, h))).expect("courts");
    g.player_set_culture_channel(P0, Some(guard(&g, h))).expect("the liege guards it");
    set_favour(&mut g, h, P1, 530);
    let cv = g.house_courtship_view(h, P1);
    assert!(cv.inflow > 0 && cv.gain > 0 && cv.guard > 0, "{cv:?}");
    let reach = cv.reach.expect("a reach");
    let calc = g.house_courtship_calc(&cv, P1);
    let t = ffh_core::culture::format_tenths;
    assert_eq!(calc.requirement[0], format!("Requirement: {}.", cv.requirement.formula()));
    assert!(calc.requirement[1].contains("attitude toward you"), "{:?}", calc.requirement);
    assert_eq!(calc.inflow, vec![g.culture_inflow_line(P1, P1, cv.inflow)]);
    assert_eq!(calc.reach, vec![format!("Reach: {}", g.culture_reach_calc(&reach))]);
    assert_eq!(calc.gain, vec![format!("Gain: {} \u{d7} {}% = {} a turn.", t(cv.inflow), reach.effect, t(cv.gain))]);
    assert!(calc.guard.last().unwrap().contains(&format!("= {} a turn", t(cv.guard))), "{:?}", calc.guard);
    assert!(calc.guard[0].starts_with("Inflow: the "), "{:?}", calc.guard);
    let next = g.favour_step(cv.favour, cv.gain, cv.guard);
    assert!(calc.turn[0].starts_with("Next liege turn: 5.3 + ") && calc.turn[0].ends_with(&format!(": {}.", t(next))), "{:?}", calc.turn);
    assert_eq!(calc.lines().len(), calc.requirement.len() + calc.inflow.len() + calc.reach.len() + calc.gain.len() + calc.guard.len() + calc.turn.len());
    // The liege's own view of the guard says "your".
    let gv = g.house_guard_view(h).expect("a guard");
    assert!(g.house_guard_calc(&gv, P0)[0].starts_with("Inflow: your cities' culture "), "{:?}", g.house_guard_calc(&gv, P0));
    // The inflow by city ends with the inflow now.
    let inflow = g.player_culture_inflow_calc(P1);
    assert_eq!(inflow.last().unwrap(), &format!("Inflow: {} a turn.", ffh_core::culture::format_hundredths(g.player_culture_inflow_now(P1))));
    assert_eq!(inflow.len(), g.players[P1].cities.len() + 1);
    // The reach's falloff.
    use ffh_core::culture::Reach;
    assert_eq!(g.culture_reach_calc(&Reach { distance: 8, effect: g.culture_distance_effect(8) }), "8 plots: (5 \u{f7} 8)\u{b2} = 39%.");
    assert_eq!(g.culture_reach_calc(&Reach { distance: 3, effect: 100 }), "3 plots: within 5, 100%.");
    assert_eq!(g.culture_reach_calc(&Reach { distance: 23, effect: 0 }), "23 plots: beyond 20, 0%.");
    // The requirement at the attitude now and each better one.
    let steps = g.house_requirement_steps(h, P1);
    let lines = g.house_requirement_steps_calc(h, P1);
    assert_eq!(lines.len(), steps.len() + 1);
    for ((_, total), line) in steps.iter().zip(&lines) {
        assert!(line.ends_with(&format!(" = {total}.")), "{line}");
    }
    // Without a channel at the House, its gain is 0 and says why.
    g.player_set_culture_channel(P1, None).expect("stopped");
    let idle = g.house_courtship_view(h, P1);
    assert_eq!(g.house_courtship_calc(&idle, P1).gain, vec![format!("Gain: 0 (no Court channel of yours at {}).", g.house_label(h))]);
}

/// While a Court channel runs at one of its Houses, the liege remembers its caster: a point of
/// `MEMORY_COURTED_HOUSE` every 5 turns, while the count is below 8.
#[test]
fn the_grudge_accrues_every_five_turns_up_to_eight() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    g.player_set_culture_channel(P1, Some(court(&g, h))).expect("courts");
    let since = g.game_turn();
    for t in 1..=60 {
        g.set_game_turn(since + t);
        g.player_do_culture_channel(P1);
        assert_eq!(g.players[P0].ai_memory_count[P1][COURTED], (t / 5).min(8), "turn {t}");
    }
    let hp = g.houses[h].player;
    assert_eq!(g.players[hp].ai_memory_count[P1][COURTED], 0, "the House does not");
}

// ---------------------------------------------------------------------------------------------
// The split (rule 4)
// ---------------------------------------------------------------------------------------------

/// At the requirement the House splits toward the courter: the half of its cities nearest it
/// (rounded down, never the capital) go to a new House of the courter's realm, of the House's
/// civilization, its leader drawn; in peace; the liege remembers it; the courter's favour is 0.
#[test]
fn a_split_at_the_requirement_takes_the_half_nearest_the_courter() {
    let World { mut g, a, b, c } = world();
    let d = g.player_init_city(P0, 12, 7, true, true);
    let (pa, pb, pc, pd) = (g.city_plot(a), g.city_plot(b), g.city_plot(c), g.city_plot(d));
    let h = house_with(&mut g, a, &[b, c, d]);
    let hp = g.houses[h].player;
    assert_eq!(g.house_seat(h).map(|s| g.city_plot(s)), Some(pa));
    // D is 3 plots from P1's city, B 5, C 8; the seat stays.
    let split = g.house_split_toward(h, P1).expect("a split");
    assert!(!split.whole);
    assert_eq!(split.going.iter().map(|&x| g.city_plot(x)).collect::<Vec<_>>(), vec![pd, pb]);
    assert!(split.held.is_empty());
    let need = 100 * g.house_requirement(h, P1).total;
    set_favour(&mut g, h, P1, 2 * need);
    let label = g.house_label(h);
    turn(&mut g);
    // The old House keeps its seat and C.
    assert!(g.houses.contains(h));
    assert_eq!(g.house_cities(h).iter().map(|&x| g.city_plot(x)).collect::<Vec<_>>().len(), 2);
    assert_eq!(g.cities[at(&g, pa)].owner, hp);
    assert_eq!(g.cities[at(&g, pc)].owner, hp);
    // The new House: the courter's, of the Bannor, its leader a Bannor minor leader.
    let np = g.cities[at(&g, pd)].owner;
    assert_eq!(g.cities[at(&g, pb)].owner, np);
    let nh = g.player_house(np).expect("a House");
    assert_eq!((g.houses[nh].owner, g.houses[nh].origin), (P1, HouseOrigin::Courted));
    assert_eq!(g.player_get_civilization_type(np), g.player_get_civilization_type(hp));
    let leader = g.house_leader(nh).expect("a leader");
    assert!(g.d.leader_heads[leader].minor_leader_of.is_some());
    assert!(g.house_label(nh).starts_with("House "));
    // In peace.
    let (t0, t1, ht, nt) = (g.players[P0].team, g.players[P1].team, g.players[hp].team, g.players[np].team);
    assert!(!g.at_war(t0, t1) && !g.at_war(t0, nt) && !g.at_war(ht, nt) && !g.at_war(t1, nt));
    assert!(g.teams[nt].vassal[t1]);
    // Remembered; the favour spent; told.
    assert_eq!(g.players[P0].ai_memory_count[P1][SPLIT], 1);
    assert_eq!(g.house_favour(h, P1), 0);
    assert!(logged(&g, &format!("The Elohim's courtship splits {label}: ")));
    assert!(logged(&g, &format!("{label} splits toward you: ")));
}

/// A House of one city goes whole: it becomes the courter's House (its record ends with its liege
/// and begins with the courter, origin `Courted`), in peace.
#[test]
fn a_house_of_one_city_goes_whole() {
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let hp = g.houses[h].player;
    let label = g.house_label(h);
    let split = g.house_split_toward(h, P1).expect("a split");
    assert!(split.whole && split.going.len() == 1);
    let need = 100 * g.house_requirement(h, P1).total;
    set_favour(&mut g, h, P1, 2 * need);
    turn(&mut g);
    assert!(!g.houses.contains(h), "its record with the old liege ended");
    let nh = g.player_house(hp).expect("a House of the courter");
    assert_eq!((g.houses[nh].owner, g.houses[nh].origin), (P1, HouseOrigin::Courted));
    assert_eq!(g.house_label(nh), label, "it keeps its name");
    let (t0, t1, ht) = (g.players[P0].team, g.players[P1].team, g.players[hp].team);
    assert!(g.teams[ht].vassal[t1] && !g.teams[ht].vassal[t0]);
    assert!(!g.at_war(t0, ht) && !g.at_war(t0, t1));
    assert_eq!(g.players[P0].ai_memory_count[P1][SPLIT], 1);
    assert!(g.houses[nh].standing.mods.is_empty(), "no modifier");
}

/// The hold contest keeps a city of the half where the liege's army units outnumber the House's;
/// when it keeps every one, nothing goes and the favour is spent all the same.
#[test]
fn the_hold_contest_keeps_a_held_city() {
    let World { mut g, a, b, c } = world();
    let d = g.player_init_city(P0, 12, 7, true, true);
    let (pb, pd) = (g.city_plot(b), g.city_plot(d));
    let h = house_with(&mut g, a, &[b, c, d]);
    let hp = g.houses[h].player;
    let warrior = g.d.t::<UnitType>("UNIT_WARRIOR");
    let (dx, dy) = g.map.xy(pd);
    let u = g.player_init_unit(P0, warrior, dx, dy, None, None);
    g.army_form(P0, &[u]).expect("an army");
    assert!(g.city_held_by_army(at(&g, pd)));
    let split = g.house_split_toward(h, P1).unwrap();
    assert_eq!((split.going.iter().map(|&x| g.city_plot(x)).collect::<Vec<_>>(), split.held.iter().map(|&x| g.city_plot(x)).collect::<Vec<_>>()), (vec![pb], vec![pd]));
    let need = 100 * g.house_requirement(h, P1).total;
    set_favour(&mut g, h, P1, 2 * need);
    turn(&mut g);
    assert_eq!(g.cities[at(&g, pd)].owner, hp, "D held: it stays with the House");
    assert_ne!(g.cities[at(&g, pb)].owner, hp, "B went");
    // Every city of the half held: nothing goes, the favour is spent.
    let World { mut g, a, b, .. } = world();
    let (bx, by) = (g.cities[b].x, g.cities[b].y);
    let pb = g.city_plot(b);
    let h = house_with(&mut g, a, &[b]);
    let hp = g.houses[h].player;
    let u = g.player_init_unit(P0, warrior, bx, by, None, None);
    g.army_form(P0, &[u]).expect("an army");
    let need = 100 * g.house_requirement(h, P1).total;
    set_favour(&mut g, h, P1, 2 * need);
    turn(&mut g);
    assert_eq!(g.cities[at(&g, pb)].owner, hp, "held");
    assert_eq!(g.house_favour(h, P1), 0, "spent all the same");
    assert_eq!(g.players[P0].ai_memory_count[P1][SPLIT], 0, "nothing split");
}

/// The courtship's news reaches the log as a House's (`LogMark::House`, the client's alerts): the
/// notice at two thirds to the liege (tagged with its House) and to the courter; the split to the
/// liege (tagged), to the courter (tagged with the House it gained) and to the others; a House that
/// goes whole, to its old liege too.
#[test]
fn the_courtship_news_is_marked_as_a_houses() {
    use ffh_core::log::{LogMark, LogTag};
    let World { mut g, a, b, c } = world_of(true);
    let h = house_with(&mut g, a, &[b, c]);
    let label = g.house_label(h);
    let marked = |g: &Game, p: PlayerId, s: &str| -> Option<LogTag> {
        let e = g.log.entries.iter().rev().find(|e| e.text.contains(s) && e.player == Some(p.0))?;
        assert_eq!(e.mark, LogMark::House, "{}", e.text);
        assert!(e.at.is_some(), "{}", e.text);
        Some(e.tag)
    };
    let need = 100 * g.house_requirement(h, P1).total;
    g.player_set_culture_channel(P1, Some(court(&g, h))).expect("P1 courts the House");
    g.players[P1].culture_inflow = need / 10;
    set_favour(&mut g, h, P1, need * 2 / 3 + 1);
    turn(&mut g);
    assert!(g.houses[h].favour[0].warned && g.house_favour(h, P1) < need, "past two thirds, short of the requirement");
    assert_eq!(marked(&g, P0, &format!("{label}: the Elohim's favour is ")), Some(LogTag::House(h)), "the liege's notice");
    assert_eq!(marked(&g, P1, &format!("{label}: your favour is ")), Some(LogTag::None), "the courter's: not its House");
    set_favour(&mut g, h, P1, 2 * need);
    turn(&mut g);
    assert_eq!(marked(&g, P0, &format!("The Elohim's courtship splits {label}: ")), Some(LogTag::House(h)));
    let tag = marked(&g, P1, &format!("{label} splits toward you: ")).expect("the courter is told");
    let LogTag::House(nh) = tag else { panic!("tagged with the new House: {tag:?}") };
    assert_eq!(g.houses[nh].owner, P1);
    let news = g.log.entries.iter().find(|e| e.text.starts_with(&format!("The Elohim's courtship splits {label} of the "))).expect("the others' news");
    assert_eq!(news.mark, LogMark::House);
    // A House of one city goes whole: its old liege's news is marked too.
    let World { mut g, a, .. } = world();
    let h = house(&mut g, a);
    let label = g.house_label(h);
    let need = 100 * g.house_requirement(h, P1).total;
    set_favour(&mut g, h, P1, 2 * need);
    turn(&mut g);
    assert_eq!(marked(&g, P0, &format!("The Elohim's courtship takes {label}: ")), Some(LogTag::None));
    assert!(marked(&g, P1, &format!("{label} joins you with ")).is_some_and(|t| matches!(t, LogTag::House(_))));
}

/// No leader free: no split this turn (each party told once), the favour kept.
#[test]
fn no_free_leader_no_split() {
    let World { mut g, a, b, .. } = world();
    let h = house_with(&mut g, a, &[b]);
    let hp = g.houses[h].player;
    // A civilization without minor leaders, and a courter of no alignment (no fallback).
    g.setup.players[hp.idx()].civ = Some(g.d.t::<CivilizationType>("CIVILIZATION_BARBARIAN"));
    g.players[P1].alignment = None;
    let civ = g.player_get_civilization_type(hp).unwrap();
    assert!(g.house_leader_pool(civ, P1).leaders.is_empty());
    // The favour crosses the requirement this turn (its Court channel's gain).
    let need = 100 * g.house_requirement(h, P1).total;
    g.player_set_culture_channel(P1, Some(court(&g, h))).expect("courts");
    assert_eq!(g.culture_house_reach(P1, h).map(|r| r.effect), Some(100));
    g.players[P1].culture_inflow = need / 4;
    set_favour(&mut g, h, P1, need - 1);
    let label = g.house_label(h);
    let told = |g: &Game| g.log.entries.iter().filter(|e| e.text == format!("{label} would split toward the Elohim: no leader is free.")).count();
    turn(&mut g);
    assert_eq!(g.house_cities(h).len(), 2, "no split");
    assert!(g.house_favour(h, P1) >= need, "the favour kept");
    assert_eq!(told(&g), 1);
    let both: Vec<_> = g.log.entries.iter().filter(|e| e.text.ends_with(": no leader is free.")).collect();
    assert_eq!(both.len(), 2, "the liege and the courter");
    assert!(both.iter().all(|e| e.mark == ffh_core::log::LogMark::House), "marked as a House's news");
    turn(&mut g);
    assert_eq!(told(&g), 1, "told once");
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
