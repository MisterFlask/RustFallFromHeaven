//! National units (this game, not FFH2; docs/PORTING.md, "Changes from FFH2"): a national class's
//! limit is how many its owner trains at the normal cost; past it, each one costs
//! `NATIONAL_UNIT_OVER_LIMIT_COST_PERCENT`. The casters' classes (`hard_limit`) keep FFH2's limit.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A small map after `game_init`, the starting units gone, player 0 human with one city and every
/// tech.
fn game() -> (Game, PlayerId, CityId) {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7);
    setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    setup.players[1].civ = Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM"));
    setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_SABATHIEL"));
    setup.players[1].leader = Some(d.t::<LeaderHeadType>("LEADER_EINION"));
    let mut g = Game::new(d, setup);
    g.game_init();
    let p0 = PlayerId(0);
    g.players[p0].human = true;
    for u in g.players[p0].units.clone() {
        g.unit_kill(u, false, None);
    }
    let q = g.map.plot_ids().find(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].owner.is_none()).expect("land");
    let (x, y) = g.map.xy(q);
    let c = g.player_init_city(p0, x, y, true, true);
    let team = g.players[p0].team;
    for t in d.techs.keys() {
        g.team_set_has_tech(team, t, true, Some(p0), false, false);
    }
    (g, p0, c)
}

fn add(g: &mut Game, p: PlayerId, c: CityId, unit: UnitType, n: usize) -> Vec<UnitId> {
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    (0..n).map(|_| g.player_init_unit(p, unit, x, y, None, None)).collect()
}

#[test]
fn a_national_unit_past_its_limit_costs_the_over_limit_percent() {
    let (mut g, p, c) = game();
    let d = g.d;
    let knight = d.t::<UnitType>("UNIT_KNIGHT");
    let class = d.units[knight].unit_class_type.unwrap();
    let limit = d.unit_classes[class].max_player_instances;
    assert_eq!(limit, 4, "FFH2's national limit for the Knight");
    assert!(!d.unit_classes[class].hard_limit);
    let normal = g.player_get_production_needed_unit(p, knight);
    let percent = d.defines.national_unit_over_limit_cost_percent;
    assert_eq!(percent, 500);

    let mut knights = add(&mut g, p, c, knight, 3);
    assert_eq!(g.player_get_production_needed_unit(p, knight), normal, "3 of 4: the normal cost");
    knights.extend(add(&mut g, p, c, knight, 1));
    assert_eq!(g.player_get_production_needed_unit(p, knight), normal * percent / 100, "4 of 4: the next one costs 500%");
    // Not a ban: the class never maxes out, so a 5th and a 6th can be trained (at 500% each).
    assert!(!g.player_is_unit_class_maxed_out(p, class, 0));
    assert!(g.player_can_train(p, knight, false, false, false));
    knights.extend(add(&mut g, p, c, knight, 2));
    assert_eq!(g.player_get_production_needed_unit(p, knight), normal * percent / 100, "6: still 500%, the price does not climb");

    // The units alive count: with three gone, the next one is at the normal cost again.
    for &u in &knights[..3] {
        g.unit_kill(u, false, None);
    }
    assert_eq!(g.player_get_production_needed_unit(p, knight), normal);
}

#[test]
fn the_casters_keep_the_national_limit() {
    let (mut g, p, c) = game();
    let d = g.d;
    let archmage = d.t::<UnitType>("UNIT_ARCHMAGE");
    let class = d.units[archmage].unit_class_type.unwrap();
    assert!(d.unit_classes[class].hard_limit);
    let normal = g.player_get_production_needed_unit(p, archmage);
    add(&mut g, p, c, archmage, 4);
    assert!(g.player_is_unit_class_maxed_out(p, class, 0), "4 Archmages: no 5th");
    assert!(!g.player_can_train(p, archmage, false, false, false));
    assert!(!g.player_is_over_unit_class_limit(p, class));
    assert_eq!(g.player_get_production_needed_unit(p, archmage), normal, "a hard limit is not a price");
}

#[test]
fn every_hard_limited_class_is_a_casters_national_class() {
    let d = ffh_data::global();
    let channeling = d.t::<PromotionType>("PROMOTION_CHANNELING1");
    let channeling2 = d.t::<PromotionType>("PROMOTION_CHANNELING2");
    let mut hard = 0;
    for k in d.unit_classes.keys() {
        if !d.unit_classes[k].hard_limit {
            continue;
        }
        hard += 1;
        assert!(d.unit_classes[k].max_player_instances > 0, "{}: hard_limit on a class without a national limit", d.unit_classes[k].base.type_);
        for u in d.units.keys().filter(|&u| d.units[u].unit_class_type == Some(k) && d.units[u].production_cost > 0) {
            let info = &d.units[u];
            assert!(info.free_promotions[channeling] || info.free_promotions[channeling2], "{}: a hard-limited class's unit casts", info.base.type_);
        }
    }
    assert_eq!(hard, 9, "the Archmage, the Druid and the seven High Priests");
}

#[test]
fn upgrading_into_a_class_past_its_limit_pays_the_over_limit_cost() {
    let (mut g, p, c) = game();
    let d = g.d;
    let knight = d.t::<UnitType>("UNIT_KNIGHT");
    let horseman = d.t::<UnitType>("UNIT_HORSEMAN");
    let u = add(&mut g, p, c, horseman, 1)[0];
    add(&mut g, p, c, knight, 3);
    let under = g.unit_upgrade_price(u, knight);
    let normal = g.player_get_production_needed_unit(p, knight);
    add(&mut g, p, c, knight, 1);
    let over = g.unit_upgrade_price(u, knight);
    let extra = normal * d.defines.national_unit_over_limit_cost_percent / 100 - normal;
    assert_eq!(over - under, extra * d.defines.unit_upgrade_cost_per_production, "the upgrade pays for the extra production");
}
