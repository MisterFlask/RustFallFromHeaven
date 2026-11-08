//! The AI of the batch "battle more" (docs/CARDS.md). Ad hoc, as CLAUDE.md asks of a card's
//! content. The generic AI trains the batch's fighting units by their stats (the Runeguard, the
//! Witch-Hunter, the Gryphon Riders and the Automatons as attackers; the Bastion and the Cairn Golem
//! as defenders too) and takes its promotions by their `ai_weight`. Here:
//!
//! - **Training** ([`turn`]): at war or planning one, one order a turn for the unit that answers
//!   the enemy: War Drummers against an enemy with Fear units, Witch-Hunters and Runeguards against
//!   an enemy with casters, Dawnbringers against demons, the undead or invisible units, the Bastion
//!   for the main army, two Gryphon Riders against an enemy with a back line (casters, archers,
//!   engines); and a Cairn Golem while a frontier hill waits for a Cairn.
//! - **Buildings** ([`building_value`], production's building value): Sanctified Steel in a city
//!   that trains, more when a civ we meet fields demons or undead; the Gilded Legion in a productive
//!   city once a civ we met has units of its own.
//! - **Promotions** ([`promotion_value`], `units::best_promotion`'s hook): the Shieldbearer for a
//!   unit of the front row; Fire Arrows for an archer at war with a walled enemy; the Harpoons for a
//!   ship near sea monsters or great warships.
//! - **Units** ([`turn`]): a Gilded Legion city chooses the civilization it met with the most units
//!   of its own we could train; a Cairn Golem walks to a free hill near a foreign city and raises its
//!   Cairn there; the Bastion joins our largest stack.

use std::sync::OnceLock;

use ffh_core::defs::MOVE_IGNORE_DANGER;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::production::Choice;
use crate::units::move_to;
use crate::view::View;

/// The batch's types, looked up once (`None` without the cards' data).
struct Types {
    drummer: Option<UnitType>,
    witch_hunter: Option<UnitType>,
    runeguard: Option<UnitType>,
    dawnbringer: Option<UnitType>,
    gryphon: Option<UnitType>,
    bastion: Option<UnitType>,
    cairn_golem: Option<UnitType>,
    cairn: Option<ImprovementType>,
    raise_cairn: Option<SpellType>,
    legion_spell: Option<SpellType>,
    demon: Option<PromotionType>,
    undead: Option<PromotionType>,
    adept: Option<UnitCombatType>,
    disciple: Option<UnitCombatType>,
}

fn types(d: &GameData) -> &'static Types {
    static T: OnceLock<Types> = OnceLock::new();
    T.get_or_init(|| Types {
        drummer: d.ty::<UnitType>("UNIT_WAR_DRUMMER"),
        witch_hunter: d.ty::<UnitType>("UNIT_WITCH_HUNTER"),
        runeguard: d.ty::<UnitType>("UNIT_RUNEGUARD"),
        dawnbringer: d.ty::<UnitType>("UNIT_DAWNBRINGER"),
        gryphon: d.ty::<UnitType>("UNIT_GRYPHON_RIDER"),
        bastion: d.ty::<UnitType>("UNIT_BASTION"),
        cairn_golem: d.ty::<UnitType>("UNIT_CAIRN_GOLEM"),
        cairn: d.ty::<ImprovementType>("IMPROVEMENT_CAIRN"),
        raise_cairn: d.ty::<SpellType>("SPELL_RAISE_THE_CAIRN"),
        legion_spell: d.ty::<SpellType>("SPELL_GILDED_LEGION"),
        demon: d.ty::<PromotionType>("PROMOTION_DEMON"),
        undead: d.ty::<PromotionType>("PROMOTION_UNDEAD"),
        adept: d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT"),
        disciple: d.ty::<UnitCombatType>("UNITCOMBAT_DISCIPLE"),
    })
}

/// A foreign city within this many plots makes a hill a frontier hill.
const CAIRN_FRONTIER: i32 = 5;

/// Once a turn (`cards::TURNS`): the Gilded Legion's choice, the Cairn Golems, the Bastion, then
/// one training order.
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian {
        return;
    }
    let p = v.p;
    legion_choices(g, p);
    cairn_golems(g, p);
    bastions(g, p);
    train(g, p);
}

/// The players `p` faces: those it is at war with, else those it has met (not teammates, not the
/// barbarians).
fn foes(g: &Game, p: PlayerId) -> Vec<PlayerId> {
    let team = g.players[p].team;
    let others = |war: bool| -> Vec<PlayerId> {
        g.player_ids()
            .filter(|&o| o != p && g.players[o].alive && g.players[o].team != team && !g.player_is_barbarian(o))
            .filter(|&o| if war { g.at_war(team, g.players[o].team) } else { g.teams[team].has_met[g.players[o].team] })
            .collect()
    };
    let war = others(true);
    if war.is_empty() {
        others(false)
    } else {
        war
    }
}

/// How many land units of `p`'s foes satisfy `f`.
fn foe_units(g: &Game, p: PlayerId, f: impl Fn(UnitId) -> bool) -> usize {
    foes(g, p).iter().flat_map(|&o| g.players[o].units.iter().copied()).filter(|&u| g.units.contains(u) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && f(u)).count()
}

/// Whether unit `u` is a demon or undead.
fn unholy(g: &Game, u: UnitId) -> bool {
    let t = types(g.d);
    [t.demon, t.undead].into_iter().flatten().any(|pr| g.units[u].has_promotion[pr])
}

/// Whether unit `u` casts in battle (an adept or disciple, or any unit with magic).
fn caster(g: &Game, u: UnitId) -> bool {
    let t = types(g.d);
    let uc = g.unit_get_unit_combat_type(u);
    (uc.is_some() && (uc == t.adept || uc == t.disciple)) || g.unit_stat(u, StatType::Magic) > 0
}

/// Whether `p` is at war with a civilization or plans a war.
fn war_footing(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    crate::production::civ_enemies_at_war(g, p) || g.team_ids().any(|t| t != team && g.teams[team].ai_war_plan[t].is_some())
}

/// How many more units of type `t` `p` wants now: `want`, at most the class's national limit, less
/// those it has or makes.
fn wanted(g: &Game, p: PlayerId, t: UnitType, want: i32) -> i32 {
    let d = g.d;
    let Some(class) = d.units[t].unit_class_type else { return 0 };
    let limit = d.unit_classes[class].max_player_instances;
    let limit = if limit < 0 { want } else { limit };
    let have = g.player_get_unit_class_count_plus_making(p, class);
    (want.min(limit) - have).max(0)
}

/// One training order a turn (module comment), in `p`'s most productive city that can train the
/// unit and is not training a unit.
fn train(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let tt = types(d);
    let war = war_footing(g, p);
    let mut wish: Vec<(UnitType, i32)> = Vec::new();
    if war {
        let fear = foe_units(g, p, |u| g.unit_is_fear(u));
        let casters = foe_units(g, p, |u| caster(g, u));
        let unholy = foe_units(g, p, |u| unholy(g, u));
        let hidden = foe_units(g, p, |u| g.unit_get_invisible_type(u).is_some());
        if let (Some(t), true) = (tt.drummer, fear > 0) {
            wish.push((t, 2));
        }
        if casters >= 2 {
            wish.extend(tt.witch_hunter.map(|t| (t, 2)));
            wish.extend(tt.runeguard.map(|t| (t, 2)));
        }
        if let (Some(t), true) = (tt.dawnbringer, unholy >= 2 || hidden > 0) {
            wish.push((t, 2));
        }
        wish.extend(tt.bastion.map(|t| (t, 1)));
        // The Gryphon Riders reach the casters, archers and engines behind a line, in cities and
        // forests too (by its stats alone production prefers the Horse Archer).
        let backline = foe_units(g, p, |u| caster(g, u) || matches!(g.unit_engage_kind(u), ActionKind::Shoot | ActionKind::Bombard));
        if backline >= 3 {
            wish.extend(tt.gryphon.map(|t| (t, 2)));
        }
    }
    // (The sites are a scan of the map: only for a player that could train a golem now.)
    if let Some(t) = tt.cairn_golem {
        if g.player_can_train(p, t, false, false, false) && wanted(g, p, t, 1) > 0 && !cairn_sites(g, p).is_empty() {
            wish.push((t, 1));
        }
    }
    for (t, want) in wish {
        if !g.player_can_train(p, t, false, false, false) || wanted(g, p, t, want) == 0 {
            continue;
        }
        let ai = d.units[t].default_unit_ai_type.unwrap_or(UnitAIType::UNITAI_ATTACK);
        if let Some(c) = best_city(g, p, |c| g.city_can_train(c, t, false, false, false, false) && g.city_production_unit(c).is_none()) {
            crate::production::push(g, c, Choice::Train(t, ai));
            return;
        }
    }
}

/// `p`'s city with the most production among those `ok` accepts.
fn best_city(g: &Game, p: PlayerId, ok: impl Fn(CityId) -> bool) -> Option<CityId> {
    g.players[p].cities.iter().copied().filter(|&c| ok(c)).max_by_key(|&c| (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(c)))
}

/// What building `b`'s keys of the batch are worth in city `c` (module comment); 0 for another.
pub fn building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let d = g.d;
    let info = &d.buildings[b];
    let p = g.cities[c].owner;
    let prod = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION).max(0);
    let mut v = 0;
    // Sanctified Steel: its two promotions for the units the city trains.
    let slaying = [info.free_promotion, info.extra_free_promotion].into_iter().flatten().filter(|&pr| d.promotions[pr].promotion_combat_type.is_some_and(|k| Some(k) == types(d).demon || Some(k) == types(d).undead)).count() as i32;
    if slaying > 0 {
        let foes = foe_units(g, p, |u| unholy(g, u)) as i32;
        let weight = if foes >= 3 { 10 } else if foes > 0 { 6 } else { 2 };
        v += slaying * prod.min(15) * weight / 2 + if threat > 0 { 20 } else { 0 };
    }
    // The Gilded Legion: a civ we met with units of its own we could train.
    if info.legion_cost_percent > 0 {
        let best = g.legion_civ_choices(p).iter().map(|&o| legion_units(g, p, o)).max().unwrap_or(0);
        if best > 0 {
            v += 40 + prod.min(25) * 6 + best * 15;
        }
    }
    v
}

/// The units of player `o`'s civilization `p` could train through a Gilded Legion now (its units
/// for a class that are not `p`'s, not world units, whose techs `p` knows).
fn legion_units(g: &Game, p: PlayerId, o: PlayerId) -> i32 {
    let d = g.d;
    let (Some(civ), Some(own)) = (g.player_get_civilization_type(o), g.player_get_civilization_type(p)) else { return 0 };
    d.unit_classes
        .keys()
        .filter(|&k| !ffh_core::utils::is_world_unit_class(d, k))
        .filter_map(|k| d.civ_unit(civ, k).filter(|&u| d.civ_unit(own, k) != Some(u)))
        .filter(|&u| d.units[u].combat > 0 && g.player_can_train(p, u, false, false, false))
        .count() as i32
}

/// Each city of `p` whose Gilded Legion has not chosen: the civilization with the most units of its
/// own `p` could train (ties: the lower player), cast by the first unit there that can.
fn legion_choices(g: &mut Game, p: PlayerId) {
    let Some(spell) = types(g.d).legion_spell else { return };
    for c in g.players[p].cities.clone() {
        if g.cities[c].legion_civ.is_some() || g.city_legion_cost_percent(c) == 0 {
            continue;
        }
        let Some(best) = g.legion_civ_choices(p).into_iter().max_by_key(|&o| (legion_units(g, p, o), std::cmp::Reverse(o))) else { continue };
        let plot = g.city_plot(c);
        let target = SpellTarget::Player(best);
        let caster = g.map.plots[plot].units.iter().copied().find(|&u| g.units[u].owner == p && g.unit_can_cast_at(u, spell, target));
        if let Some(u) = caster {
            g.tel_cast_reason(u, "gilded_legion", 0);
            g.unit_cast_at(u, spell, target);
        }
    }
}

/// The free hills where `p` would raise a Cairn: a hill of its own or nobody's that can take one,
/// within [`CAIRN_FRONTIER`] plots of a foreign city (not a teammate's, not the barbarians'), two
/// plots or more from another Cairn; in plot order.
fn cairn_sites(g: &Game, p: PlayerId) -> Vec<PlotIdx> {
    let d = g.d;
    let Some(cairn) = types(d).cairn else { return Vec::new() };
    let team = g.players[p].team;
    let foreign: Vec<(i32, i32)> = g
        .player_ids()
        .filter(|&o| o != p && g.players[o].alive && g.players[o].team != team && !g.player_is_barbarian(o))
        .flat_map(|o| g.players[o].cities.iter().copied())
        .filter(|&c| g.cities.contains(c) && g.map.plots[g.city_plot(c)].revealed[team])
        .map(|c| (g.cities[c].x, g.cities[c].y))
        .collect();
    if foreign.is_empty() {
        return Vec::new();
    }
    g.map
        .plot_ids()
        .filter(|&q| {
            let pl = &g.map.plots[q];
            if !g.plot_is_hills(q) || !pl.revealed[team] || pl.plot_city.is_some() || pl.improvement_type.is_some() || pl.owner.is_some_and(|o| o != p) {
                return false;
            }
            let (x, y) = g.map.xy(q);
            foreign.iter().any(|&(fx, fy)| g.plot_distance(x, y, fx, fy) <= CAIRN_FRONTIER)
                && !(-1..=1).any(|dx| (-1..=1).any(|dy| g.plot_xy(x, y, dx, dy).is_some_and(|r| g.map.plots[r].improvement_type == Some(cairn))))
                && g.plot_can_have_improvement(q, cairn, None, false)
        })
        .collect()
}

/// Each Cairn Golem of `p` (module comment): it raises its Cairn where it stands on a site, else
/// walks to the nearest site.
fn cairn_golems(g: &mut Game, p: PlayerId) {
    let tt = types(g.d);
    let (Some(golem), Some(spell)) = (tt.cairn_golem, tt.raise_cairn) else { return };
    let mine: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units[u].unit_type == golem && !g.unit_is_cargo(u)).collect();
    if mine.is_empty() {
        return;
    }
    let sites = cairn_sites(g, p);
    for u in mine {
        if !g.units.contains(u) {
            continue;
        }
        let here = g.unit_plot(u);
        if sites.contains(&here) && g.unit_can_cast(u, spell, false) {
            crate::magic::cast(g, u, spell, "raise_cairn", 0);
            continue;
        }
        if !g.unit_can_move(u) {
            continue;
        }
        let (x, y) = g.map.xy(here);
        let Some(&site) = sites.iter().min_by_key(|&&q| {
            let (qx, qy) = g.map.xy(q);
            (g.plot_distance(x, y, qx, qy), q)
        }) else {
            continue;
        };
        if let Some(grp) = own_group(g, u) {
            move_to(g, grp, site, MOVE_IGNORE_DANGER);
        }
    }
}

/// The single unit of `u`'s group, if it is alone in it (a fresh group to move it with).
fn own_group(g: &mut Game, u: UnitId) -> Option<GroupId> {
    let grp = g.units[u].group?;
    if g.groups[grp].units.len() > 1 {
        g.unit_join_group(u, None, true);
    }
    g.units[u].group
}

/// `p`'s land fighters on `plot` (units that can attack).
fn fighters_on(g: &Game, p: PlayerId, plot: PlotIdx) -> Vec<UnitId> {
    g.map.plots[plot].units.iter().copied().filter(|&u| g.units[u].owner == p && g.unit_can_attack(u) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND).collect()
}

/// The Bastion of `p` (module comment): on a plot with three or more of our fighters it joins their
/// group; elsewhere it walks to the plot of our largest stack outside a city (at least three
/// fighters), nearest first.
fn bastions(g: &mut Game, p: PlayerId) {
    let Some(bastion) = types(g.d).bastion else { return };
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.units[u].unit_type != bastion || g.unit_is_cargo(u) {
            continue;
        }
        let here = g.unit_plot(u);
        let local = fighters_on(g, p, here);
        if local.len() >= 3 {
            if let Some(grp) = local.iter().filter_map(|&f| g.units[f].group).max_by_key(|&grp| (g.groups[grp].units.len(), std::cmp::Reverse(grp))) {
                if g.units[u].group != Some(grp) {
                    g.unit_join_group(u, Some(grp), true);
                }
            }
            continue;
        }
        if !g.unit_can_move(u) {
            continue;
        }
        let (x, y) = g.map.xy(here);
        let mut best: Option<(usize, i32, PlotIdx)> = None;
        let mut seen: Vec<PlotIdx> = Vec::new();
        for &o in &g.players[p].units {
            let Some(q) = g.unit_plot_opt(o) else { continue };
            if seen.contains(&q) || g.plot_is_water(q) || g.map.plots[q].plot_city.is_some() {
                continue;
            }
            seen.push(q);
            let n = fighters_on(g, p, q).len();
            if n < 3 {
                continue;
            }
            let (qx, qy) = g.map.xy(q);
            let dist = g.plot_distance(x, y, qx, qy);
            if best.map_or(true, |(bn, bd, _)| n > bn || n == bn && dist < bd) {
                best = Some((n, dist, q));
            }
        }
        if let Some((_, _, target)) = best {
            if let Some(grp) = own_group(g, u) {
                move_to(g, grp, target, MOVE_IGNORE_DANGER);
            }
        }
    }
}

/// `units::best_promotion`'s hook (module comment): `v` plus what promotion `promo`'s battle kit
/// and naval keys are worth to unit `u`.
pub fn promotion_value(g: &Game, u: UnitId, promo: PromotionType, v: i32) -> i32 {
    let d = g.d;
    let info = &d.promotions[promo];
    let p = g.units[u].owner;
    let mut value = v;
    let kit = info.kit;
    if kit.shield > 0 {
        // A unit of the front row shields the lane behind it.
        let front = g.unit_engages_on_land(u) && g.unit_engage_row_order(u)[0] == 0;
        value += if front { 60 } else { -1000 };
    }
    if kit.shots_at_walls {
        let walled = foes(g, p).iter().flat_map(|&o| g.players[o].cities.iter().copied()).any(|c| g.cities.contains(c) && g.city_sentinel_count(c) > 0);
        value += if walled && crate::production::civ_enemies_at_war(g, p) { 50 } else { 10 };
    }
    if info.naval_damage_vs.iter().any(|&x| x != 0) {
        value += 15;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Players 0 and 1 met and at war on one landmass, both knowing the batch's cards: a city of
    /// player 0, one of player 1 four plots away.
    fn front() -> (Game, CityId, CityId) {
        let mut g = crate::testkit::one_landmass_game(4);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 8, 5, true, true);
        let theirs = g.player_init_city(p1, 12, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        for card in ["TECH_CARD_SANCTIFIED_STEEL", "TECH_CARD_CAIRN_GOLEM", "TECH_CARD_GILDED_LEGION", "TECH_CARD_SHIELDBEARER", "TECH_CARD_FIRE_ARROWS"] {
            let tech = g.d.t::<TechType>(card);
            for t in [t0, t1] {
                g.team_set_has_tech(t, tech, true, None, false, false);
            }
        }
        (g, ours, theirs)
    }

    /// A Cairn Golem on a free hill near a foreign city raises its Cairn there.
    #[test]
    fn a_cairn_golem_raises_its_cairn_on_a_frontier_hill() {
        let (mut g, _, theirs) = front();
        let p0 = PlayerId(0);
        let (x, y) = (g.cities[theirs].x - 2, g.cities[theirs].y + 2);
        let hill = g.map.plot_xy(x, y).expect("a plot");
        g.map.plots[hill].plot_type = PlotType::PLOT_HILLS;
        g.map.plots[hill].feature_type = None;
        g.map.plots[hill].improvement_type = None;
        if g.map.plots[hill].owner.is_some_and(|o| o != p0) {
            g.plot_set_owner(hill, None, false, false);
        }
        g.map.plots[hill].revealed[g.players[p0].team] = true;
        assert!(cairn_sites(&g, p0).contains(&hill), "a frontier hill is a site");
        let golem = g.d.t::<UnitType>("UNIT_CAIRN_GOLEM");
        let u = g.player_init_unit(p0, golem, x, y, None, None);
        cairn_golems(&mut g, p0);
        assert!(!g.units.contains(u) || g.units[u].death_delay || g.unit_is_dead(u), "the golem is used up");
        assert_eq!(g.map.plots[hill].improvement_type, g.d.ty::<ImprovementType>("IMPROVEMENT_CAIRN"));
    }

    /// Sanctified Steel is worth more to a city that trains when the enemy fields the undead; the
    /// Shieldbearer only to a unit of the front row.
    #[test]
    fn sanctified_steel_and_the_shieldbearer_go_where_they_serve() {
        let (mut g, ours, theirs) = front();
        let steel = g.d.t::<BuildingType>("BUILDING_SANCTIFIED_STEEL");
        let before = building_value(&g, ours, steel, 0);
        assert!(before > 0);
        let skeleton = g.d.t::<UnitType>("UNIT_SKELETON");
        for _ in 0..3 {
            g.player_init_unit(PlayerId(1), skeleton, g.cities[theirs].x, g.cities[theirs].y, None, None);
        }
        assert!(building_value(&g, ours, steel, 0) > before, "undead to fight");
        let shield = g.d.t::<PromotionType>("PROMOTION_SHIELDBEARER");
        let axeman = g.d.t::<UnitType>("UNIT_AXEMAN");
        let archer = g.d.t::<UnitType>("UNIT_ARCHER");
        let (x, y) = (g.cities[ours].x, g.cities[ours].y);
        let a = g.player_init_unit(PlayerId(0), axeman, x, y, None, None);
        let b = g.player_init_unit(PlayerId(0), archer, x, y, None, None);
        assert!(promotion_value(&g, a, shield, 0) > 0);
        assert!(promotion_value(&g, b, shield, 0) < 0);
    }
}
