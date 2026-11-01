//! Adding a promotion or a building and removing it again leaves the game as it was.
//!
//! The rules keep many running totals up to date by adding and subtracting the effects of what a
//! unit or a city has (`unit_set_has_promotion` updates about 80 of them, `city_process_building`
//! as many). A total that is changed on the way in and not on the way out, or by a different
//! amount, drifts for the rest of the game; this is the bug a rules change most easily makes.
//! The check is the save: every saved byte must be back as it was, apart from history the rules
//! keep on purpose (how many of a building class were ever built, the Armageddon counter's
//! high-water mark) and effects that are permanent by design (below).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::Idx;

/// The save, with the history that adding something records on purpose put back as in `base`.
fn save_without_history(g: &mut Game, base: &Game) -> Vec<u8> {
    g.building_class_created_count = base.building_class_created_count.clone();
    g.max_global_counter = base.max_global_counter;
    for (c, city) in base.cities.iter() {
        // AI bookkeeping: citizens are reassigned at the next update.
        g.cities[c].ai_assign_work_dirty = city.ai_assign_work_dirty;
    }
    g.save()
}

/// A generated duel map after `game_init`, with player 0's first unit standing in a city it
/// founded there.
fn game_with_city() -> (Game, UnitId, CityId) {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 3));
    g.game_init();
    let p = PlayerId(0);
    let u = g.players[p].units[0];
    let (x, y) = (g.units[u].x, g.units[u].y);
    let c = g.player_init_city(p, x, y, true, true);
    (g, u, c)
}

#[test]
fn promotions_add_and_remove_cleanly() {
    let (mut g, u, _) = game_with_city();
    let d = g.d;
    let mut failed = Vec::new();
    for promotion in d.promotions.keys() {
        let info = &d.promotions[promotion];
        // Permanent by design: a unit taken over by the AI or hiding its nationality leaves its
        // group, a mutation changes the unit for good, and more sight reveals plots.
        let permanent = info.ai_control || info.hidden_nationality || info.visibility_change > 0 || promotion.idx() as i32 == d.defines.mutated_promotion;
        if g.units[u].has_promotion[promotion] || permanent {
            continue;
        }
        let base = Game::load(d, &g.save()).expect("reload");
        let before = g.save();
        g.unit_set_has_promotion(u, promotion, true);
        g.unit_set_has_promotion(u, promotion, false);
        if save_without_history(&mut g, &base) != before {
            failed.push(d.reg.type_name(promotion).to_string());
            g = Game::load(d, &before).expect("reload");
        }
    }
    assert!(failed.is_empty(), "{} promotions leave a trace after add + remove: {failed:?}", failed.len());
}

#[test]
fn buildings_add_and_remove_cleanly() {
    let (mut g, _, c) = game_with_city();
    let d = g.d;
    let mut failed = Vec::new();
    for building in d.buildings.keys() {
        let info = &d.buildings[building];
        // What a building does once, when it is first built (C++ `setNumRealBuildingTimed`):
        // a new capital, a golden age, population, a religion, free techs.
        let once = info.capital
            || info.golden_age
            || info.global_population_change != 0
            || info.religion_change.iter().any(|&r| r > 0)
            || info.free_techs > 0;
        if g.cities[c].num_real_building[building] != 0 || once {
            continue;
        }
        let base = Game::load(d, &g.save()).expect("reload");
        let before = g.save();
        g.city_set_num_real_building(c, building, 1);
        g.city_set_num_real_building(c, building, 0);
        if save_without_history(&mut g, &base) != before {
            failed.push(d.reg.type_name(building).to_string());
            g = Game::load(d, &before).expect("reload");
        }
    }
    assert!(failed.is_empty(), "{} buildings leave a trace after add + remove: {failed:?}", failed.len());
}
