//! Engagement rows (phase 6): what a unit does from each row is per unit type (`engage_rows` and
//! `engage_row_order` in units.jsonc), but whether a unit fights is the unit's own strength (C++
//! `CvUnit::canFight`, on `baseCombatStr` / `baseCombatStrDefense`), which promotions add to. So a
//! type without strength must have rows too: a Worker that takes the Godslayer is a 3/3 unit that
//! defends its plot, and an engagement commits it like any other defender.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

#[test]
fn a_worker_that_takes_the_godslayer_defends_and_has_rows() {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let p0 = PlayerId(0);
    let first = g.players[p0].units[0];
    let (x, y) = (g.units[first].x, g.units[first].y);
    let worker_type = d.t::<UnitType>("UNIT_WORKER");
    let info = &d.units[worker_type];
    assert_eq!((info.combat, info.combat_defense, info.damage_type_combat.iter().sum::<i32>()), (0, 0, 0));
    let worker = g.player_init_unit(p0, worker_type, x, y, None, None);
    assert!(!g.unit_can_fight(worker) && !g.unit_can_defend(worker, None));

    // The equipment on the worker's plot, taken by the worker's own ability.
    g.player_init_unit(p0, d.t::<UnitType>("EQUIPMENT_GODSLAYER"), x, y, None, None);
    let take = d.t::<SpellType>("SPELL_TAKE_GODSLAYER_UNIT");
    assert!(g.unit_can_cast(worker, take, false), "a worker can take the Godslayer");
    g.unit_cast(worker, take);
    assert!(g.units[worker].has_promotion[d.t::<PromotionType>("PROMOTION_GODSLAYER")]);
    assert_eq!((g.unit_base_combat_str(worker), g.unit_base_combat_str_defense(worker)), (3, 3));
    assert!(g.unit_can_fight(worker) && g.unit_can_defend(worker, None));

    // Its type has rows and a rank preference (it strikes, from the front first), so the
    // engagement code can rely on every land unit that can fight having them.
    assert!(g.unit_engages_on_land(worker));
    assert_eq!((g.unit_engage_kind(worker), g.unit_engage_row_order(worker)), (ActionKind::Strike, [0, 1, 2]));
}
