//! The AI of the great-person cards that act in battles and at walls (docs/CARDS.md; the batch
//! "battle arms"). Ad hoc, as CLAUDE.md asks of a card's content: the generic AI already trains
//! the cards' units (the Siege Ballista as siege, with its `ai_weight`; the Siege Colossus and the
//! Herald of Agares as attackers) and takes their promotions (Wardance and Templar, by their
//! `ai_weight`). Here:
//!
//! - **Buildings** ([`building_value`], added to production's building value): the battle
//!   buildings in the cities an enemy can reach (a threat, or a foreign city within
//!   [`FRONTIER`] plots), more in the capital; the Sentinel buildings (Living Walls, the Arcane
//!   Fortress) only where there are walls; the Unconquered City only in a city never lost; an
//!   Elemental Forge in a city that trains (its production), less where the element is resisted
//!   by the units of the civs we are at war with (or, at peace, have met).
//! - **Trenches** ([`build_value`], the cities' crews): on a plot of ours where a military unit of ours
//!   holds outside a city, at war: the side that defends on a trench is the one it helps.
//! - **The Siege Colossus** ([`end_turn`]): after the units have moved, a unit with a bombard
//!   range that still has its attack and a move bombards an enemy city in reach whose Sentinels
//!   stand (it then counts as besieged until its next turn). Bombarding before the stacks move
//!   would hold its stack back.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// A city within this many plots of a foreign city may be attacked.
pub const FRONTIER: i32 = 6;

/// Whether city `c` lies within [`FRONTIER`] plots of a city of another civ (not a teammate,
/// not the barbarians).
fn frontier(g: &Game, c: CityId) -> bool {
    let (x, y, owner) = (g.cities[c].x, g.cities[c].y, g.cities[c].owner);
    let team = g.players[owner].team;
    g.player_ids().filter(|&o| o != owner && g.players[o].alive && g.players[o].team != team && !g.player_is_barbarian(o)).any(|o| {
        g.players[o].cities.iter().any(|&oc| g.cities.contains(oc) && g.plot_distance(x, y, g.cities[oc].x, g.cities[oc].y) <= FRONTIER)
    })
}

/// The share (percent) of the units of the civs `p` is at war with (else of those it has met)
/// that resist damage type `dt` by half or more.
fn resisted_pct(g: &Game, p: PlayerId, dt: DamageType) -> i32 {
    let team = g.players[p].team;
    let foes: Vec<PlayerId> = g.player_ids().filter(|&o| o != p && g.players[o].alive && g.at_war(team, g.players[o].team) && !g.player_is_barbarian(o)).collect();
    let foes = if foes.is_empty() {
        g.player_ids().filter(|&o| o != p && g.players[o].alive && g.players[o].team != team && g.teams[team].has_met[g.players[o].team] && !g.player_is_barbarian(o)).collect()
    } else {
        foes
    };
    let (mut all, mut resist) = (0, 0);
    for o in foes {
        for &u in &g.players[o].units {
            if !g.units.contains(u) || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || !g.unit_can_fight(u) {
                continue;
            }
            all += 1;
            if g.unit_get_damage_type_resist(u, dt) >= 50 {
                resist += 1;
            }
        }
    }
    if all == 0 {
        0
    } else {
        resist * 100 / all
    }
}

/// What building `b`'s card keys are worth in city `c` (module comment); 0 for a building
/// without them. `threat` is production's local threat.
pub fn building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let d = g.d;
    let info = &d.buildings[b];
    let bt = &info.battle;
    let mut v = 0;
    if bt.any() {
        let weight = if threat > 0 {
            4
        } else if frontier(g, c) {
            3
        } else {
            1
        };
        let walls = g.city_sentinel_count(c) > 0;
        if bt.pre_battle_damage.is_some() {
            v += 120 * weight;
        }
        if bt.hold_melee_rounds > 0 {
            v += 100 * weight;
        }
        if bt.last_stand_damage > 0 {
            v += 90 * weight;
        }
        if bt.never_lost_damage > 0 && g.city_never_lost(c) {
            v += 100 * weight;
        }
        if walls && bt.sentinel_unit.is_some() {
            v += 170 * weight;
        }
        if walls && bt.sentinel_ranged_resist > 0 {
            v += 150 * weight;
        }
        if g.city_is_capital(c) {
            v = v * 3 / 2;
        }
    }
    // An Elemental Forge: a free promotion whose attack deals an element.
    if let Some(dt) = info.free_promotion.and_then(|p| d.promotions[p].attack_damage_type) {
        let prod = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION).max(0);
        v += prod.min(15) * 10 * (100 - resisted_pct(g, g.cities[c].owner, dt)) / 100;
    }
    v
}

/// What a crew's `build` on `plot` is worth to `p` when its improvement narrows the attacker
/// (the Trench; module comment): `None` for any other build.
pub fn build_value(g: &Game, p: PlayerId, plot: PlotIdx, build: BuildType) -> Option<i32> {
    let d = g.d;
    let imp = d.builds[build].improvement?;
    if d.improvements[imp].engagement_attacker_slots_change >= 0 {
        return None;
    }
    let pl = &g.map.plots[plot];
    if pl.owner != Some(p) || pl.improvement_type.is_some() || pl.bonus_type.is_some() || pl.plot_city.is_some() {
        return Some(0);
    }
    let team = g.players[p].team;
    let at_war = g.player_ids().any(|o| o != p && g.players[o].alive && g.at_war(team, g.players[o].team));
    let held = pl.units.iter().any(|&u| g.units.contains(u) && g.units[u].owner == p && g.unit_can_fight(u) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND);
    Some(if at_war && held { 60 } else { 0 })
}

/// After the units have moved (module comment): a unit of `p` with a bombard range bombards an
/// enemy city in reach whose Sentinels stand.
pub fn end_turn(g: &mut Game, p: PlayerId) {
    let d = g.d;
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || d.units[g.units[u].unit_type].bombard_range <= 1 {
            continue;
        }
        if !g.unit_can_move(u) || !g.unit_has_attack_left(u) {
            continue;
        }
        let plot = g.unit_plot(u);
        let Some(c) = g.unit_bombard_target(u, plot) else { continue };
        if g.city_sentinels_standing(c).is_empty() || !g.at_war(g.players[p].team, g.city_team(c)) {
            continue;
        }
        g.unit_bombard(u);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Players 0 and 1 met and at war on one landmass, both knowing some of the batch's cards: a
    /// city of player 0 on the front (4 plots from player 1's), another far behind.
    fn front() -> (Game, CityId, CityId, CityId) {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let near = g.player_init_city(p0, 8, 5, true, true);
        let far = g.player_init_city(p0, 16, 5, true, true);
        let theirs = g.player_init_city(p1, 4, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        for card in ["TECH_CARD_THUNDERHEAD_TOWER", "TECH_CARD_SIEGE_COLOSSUS", "TECH_CARD_TRENCH", "TECH_CARD_UNCONQUERED_CITY", "TECH_CARD_LIVING_WALLS"] {
            let tech = g.d.t::<TechType>(card);
            for t in [t0, t1] {
                g.team_set_has_tech(t, tech, true, None, false, false);
            }
        }
        (g, near, far, theirs)
    }

    /// The battle buildings are worth more in a city on the front; the Unconquered City is worth
    /// nothing once the city was lost; the Living Walls nothing without walls.
    #[test]
    fn battle_buildings_go_to_the_front() {
        let (mut g, near, far, _) = front();
        let tower = g.d.t::<BuildingType>("BUILDING_THUNDERHEAD_TOWER");
        assert!(building_value(&g, near, tower, 0) > building_value(&g, far, tower, 0));
        assert!(building_value(&g, far, tower, 0) > 0);
        let chronicle = g.d.t::<BuildingType>("BUILDING_UNCONQUERED_CITY");
        assert!(building_value(&g, near, chronicle, 0) > 0);
        g.cities[near].never_lost = false;
        assert_eq!(building_value(&g, near, chronicle, 0), 0);
        let living = g.d.t::<BuildingType>("BUILDING_LIVING_WALLS");
        assert_eq!(building_value(&g, near, living, 0), 0);
    }

    /// Crews dig a trench only under a unit of ours that holds a plot of ours, at war.
    #[test]
    fn a_trench_goes_where_our_units_hold() {
        let (mut g, near, _, _) = front();
        let p0 = PlayerId(0);
        let build = g.d.t::<BuildType>("BUILD_TRENCH");
        let plot = g.map.plot_xy(g.cities[near].x, g.cities[near].y + 1).expect("a plot");
        if g.map.plots[plot].owner != Some(p0) {
            g.plot_set_owner(plot, Some(p0), false, false);
        }
        assert_eq!(build_value(&g, p0, plot, build), Some(0), "nobody holds it");
        let (x, y) = g.map.xy(plot);
        let axeman = g.d.t::<UnitType>("UNIT_AXEMAN");
        g.player_init_unit(p0, axeman, x, y, None, None);
        assert_eq!(build_value(&g, p0, plot, build), Some(60));
        let farm = g.d.t::<BuildType>("BUILD_FARM");
        assert_eq!(build_value(&g, p0, plot, farm), None, "not a trench");
    }

    /// The Siege Colossus bombards an enemy city two plots away once the units have moved.
    #[test]
    fn the_colossus_bombards_from_range_at_the_end_of_the_turn() {
        let (mut g, _, _, theirs) = front();
        let p0 = PlayerId(0);
        let walls = g.d.t::<BuildingType>("BUILDING_WALLS");
        let class = g.d.buildings[walls].building_class_type.unwrap();
        g.cities[theirs].building_tiers[class] = BuildingTierType::STANDARD;
        g.city_set_num_real_building(theirs, walls, 1);
        g.city_sync_sentinels(theirs);
        let colossus = g.d.t::<UnitType>("UNIT_SIEGE_COLOSSUS");
        let (x, y) = (g.cities[theirs].x + 2, g.cities[theirs].y);
        g.player_init_unit(p0, colossus, x, y, None, None);
        assert!(g.cities[theirs].sentinel_damage.iter().all(|&d| d == 0));
        end_turn(&mut g, p0);
        assert!(g.cities[theirs].sentinel_damage.iter().all(|&d| d > 0), "{:?}", g.cities[theirs].sentinel_damage);
        assert!(g.city_is_besieged(theirs));
    }
}
