//! The AI's use of card batch B9, map and movement rules (docs/CARDS.md;
//! `ffh_core::ffh::cards::map_rules`). Ad hoc, as CLAUDE.md asks of a rules change.
//!
//! - **What the movement cards change** (Mountaineering, Ferries, Forest Paths, Causeway, Canal
//!   Locks, the Wall of the Kings, the Harbor Chain) is in the rules' movement and the path
//!   finder, which the AI's moves and reach flood read: nothing to do here.
//! - **Buildings** ([`building_value`]): the Wall of the Kings and the Hall of Judgment for an empire
//!   with rivals near (more at war), the Harbor Chain in a coastal city near a rival, the Prize
//!   Courts by the warships we have, the Wheeled City in a city near a rival.
//! - **Improvements** ([`build_value`]): a Canal on an isthmus between two waters, a Causeway
//!   between two landmasses, a Wayside Shrine on an unworked road plot.
//! - **Units** ([`production`]): two Powder Charges while at war with a walled neighbour, a Work
//!   Boat for a Causeway between two landmasses, a Masker for an evil holder that prepares a war.
//! - **Each turn** ([`turn`]): a Powder Charge next to a walled enemy city our stack besieges
//!   blows its walls; a unit of a stack that besieges a city builds Siege Lines under it; a Masker
//!   in a stack next to the units of the civ we prepare war on raises false colours and the stack
//!   strikes; a city with the Wheeled City that is about to fall packs up, and its caravan walks to
//!   the best safe site within range and refounds it.

use ffh_core::defs::*;
use ffh_core::ffh::cards::map_rules::UNPACK_RANGE;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::Choice;
use crate::reach::Flood;
use crate::units::{alive, move_to, push};
use crate::view::View;

fn ty<T: InfoId>(g: &Game, name: &str) -> Option<T> {
    g.d.ty::<T>(name)
}

/// Our land combat units on `plot`.
fn fighters_on(g: &Game, p: PlayerId, plot: PlotIdx) -> Vec<UnitId> {
    g.map.plots[plot]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units[u].owner == p && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && g.unit_can_attack(u))
        .collect()
}

/// Whether a rival civilization's city is within `range` plots of `plot`.
fn rival_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    g.cities.iter().any(|(_, c)| {
        let ct = g.players[c.owner].team;
        ct != team && !g.player_is_barbarian(c.owner) && g.plot_distance(x, y, c.x, c.y) <= range
    })
}

fn at_war_with_civ(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    g.team_ids().into_iter().any(|t| t != team && g.teams[team].at_war[t] && !g.team_is_barbarian(t) && g.team_is_alive(t))
}

// -------------------------------------------------------------------------------------------
// Production.

/// What the batch's buildings are worth in city `c` (`production::generic_building_value`'s
/// scale: a hammer a turn is about 150).
pub fn building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let info = &g.d.buildings[b];
    let wheels = ty::<BuildingType>(g, "BUILDING_WHEELED_CITY") == Some(b);
    if !(info.border_stops_enemies || info.blocks_enemy_ships || info.conquest_keeps_all || info.naval_capture_percent > 0 || wheels) {
        return 0;
    }
    let p = g.cities[c].owner;
    let plot = g.city_plot(c);
    let war = at_war_with_civ(g, p) || crate::diplomacy::preparing_target(g, g.players[p].team).is_some();
    let n = g.players[p].cities.len() as i32;
    let mut v = 0;
    if info.border_stops_enemies {
        let border = g.players[p].cities.iter().filter(|&&o| rival_near(g, p, g.city_plot(o), 8)).count() as i32;
        v += 150 * border.min(6) + if war { 400 } else { 0 };
    }
    if info.blocks_enemy_ships && rival_near(g, p, plot, 10) {
        v += if threat > 0 || war { 250 } else { 80 };
    }
    if info.conquest_keeps_all {
        v += if war { 500 } else { 100 } + 20 * n.min(10);
    }
    if info.naval_capture_percent > 0 {
        let warships = g.players[p].units.iter().filter(|&&u| g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && g.unit_can_attack(u)).count() as i32;
        v += 40 * warships.min(12) * info.naval_capture_percent / 50;
    }
    if wheels && !g.city_is_capital(c) && g.cities[c].population >= 4 && rival_near(g, p, plot, 6) {
        v += if war { 300 } else { 100 };
    }
    v
}

/// What a card unit this city should train now, if any.
pub fn production(g: &Game, c: CityId, threat: i32) -> Option<Choice> {
    let p = g.cities[c].owner;
    if threat > 0 || g.player_is_barbarian(p) || !crate::production::top_production_city(g, c, |_| true) {
        return None;
    }
    let team = g.players[p].team;
    let count = |u: UnitType| g.players[p].units.iter().filter(|&&o| g.units[o].unit_type == u).count();
    // Powder Charges while at war with a civilization that has walled cities.
    if let Some(charge) = ty::<UnitType>(g, "UNIT_POWDER_CHARGE").filter(|&u| g.city_can_train(c, u, false, false, false, false)) {
        let walled_enemy = || {
            g.cities.iter().any(|(oc, o)| g.teams[team].at_war[g.players[o.owner].team] && !g.player_is_barbarian(o.owner) && !g.cities[oc].sentinel_damage.is_empty())
        };
        if count(charge) < 2 && walled_enemy() {
            return Some(Choice::Train(charge, UnitAIType::UNITAI_ATTACK_CITY));
        }
    }
    // (A Causeway between two landmasses is the city's crews' to build: `build_value`.)
    // A Masker for a war we prepare.
    if let Some(masker) = ty::<UnitType>(g, "UNIT_MASKER").filter(|&u| g.city_can_train(c, u, false, false, false, false)) {
        if count(masker) < 1 && crate::diplomacy::preparing_target(g, team).is_some() {
            return Some(Choice::Train(masker, UnitAIType::UNITAI_UNKNOWN));
        }
    }
    None
}

/// What a card improvement is worth on `plot` to `p`'s crews (`workers::build_value`'s scale),
/// `None` for an improvement of no card of this batch.
pub fn build_value(g: &Game, p: PlayerId, plot: PlotIdx, imp: ImprovementType) -> Option<i32> {
    let info = &g.d.improvements[imp];
    if info.canal {
        // An isthmus: two different waters next to the plot, one of them with a city of ours.
        let (x, y) = g.map.xy(plot);
        let mut waters: Vec<AreaId> = Vec::new();
        for i in 0..DirectionType::NUM {
            let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
            if let Some(a) = g.map.plots[q].area.filter(|_| g.plot_is_water(q)) {
                if !waters.contains(&a) {
                    waters.push(a);
                }
            }
        }
        let ours = waters.iter().any(|&a| g.players[p].cities.iter().any(|&c| g.plot_is_adjacent_to_area(g.city_plot(c), a)));
        return Some(if waters.len() >= 2 && ours { 300 } else { 0 });
    }
    if info.land_bridge {
        // A link between two landmasses.
        let (x, y) = g.map.xy(plot);
        let mut lands: Vec<AreaId> = Vec::new();
        for i in 0..DirectionType::NUM {
            let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
            if let Some(a) = g.map.plots[q].area.filter(|_| !g.plot_is_water(q)) {
                if !lands.contains(&a) {
                    lands.push(a);
                }
            }
        }
        return Some(if lands.len() >= 2 { 250 } else { 0 });
    }
    if info.requires_route && info.city_heal_tech.is_some() {
        // A roadside shrine on a plot no city works, not near another one.
        let pl = &g.map.plots[plot];
        if pl.route_type.is_none() || pl.improvement_type.is_some() || g.plot_is_being_worked(plot) || pl.bonus_type.is_some() {
            return Some(0);
        }
        let (x, y) = g.map.xy(plot);
        for dx in -3..=3 {
            for dy in -3..=3 {
                if g.plot_xy(x, y, dx, dy).is_some_and(|q| g.map.plots[q].improvement_type == Some(imp)) {
                    return Some(0);
                }
            }
        }
        return Some(if rival_near(g, p, plot, 8) { 120 } else { 60 });
    }
    if info.enemy_city_adjacent {
        // Siege Lines are sent by `turn` to a stack that besieges a city.
        return Some(0);
    }
    None
}

// -------------------------------------------------------------------------------------------
// The turn.

/// The batch's turn (`cards::TURNS`).
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian {
        return;
    }
    powder_charges(g, v);
    siege_lines(g, v);
    false_colours(g, v);
    wheeled_cities(g, v);
}

/// The single unit of `u`'s group, if it is alone in it (a fresh group to move it with).
fn own_group(g: &mut Game, u: UnitId) -> Option<GroupId> {
    let grp = g.units[u].group?;
    if g.groups[grp].units.len() > 1 {
        g.unit_join_group(u, None, true);
    }
    g.units[u].group
}

/// Joins `u` to the group on its plot with the most of our fighters (it marches with that
/// stack), if it is not in one already.
fn join_stack(g: &mut Game, u: UnitId) {
    let p = g.units[u].owner;
    let here = g.unit_plot(u);
    let mut best: Option<(usize, GroupId)> = None;
    for f in fighters_on(g, p, here) {
        let Some(grp) = g.units[f].group else { continue };
        let n = g.groups[grp].units.iter().filter(|&&o| g.unit_can_attack(o)).count();
        if best.map_or(true, |(m, b)| n > m || n == m && grp < b) {
            best = Some((n, grp));
        }
    }
    if let Some((_, grp)) = best {
        if g.units[u].group != Some(grp) {
            g.unit_join_group(u, Some(grp), true);
        }
    }
}

/// The plot of our largest land stack (at least `min` fighters), nearest to `from`.
fn nearest_stack(g: &Game, p: PlayerId, from: PlotIdx, min: usize, near: impl Fn(PlotIdx) -> bool) -> Option<PlotIdx> {
    let (fx, fy) = g.map.xy(from);
    let mut best: Option<(i32, i32, PlotIdx)> = None;
    let mut seen: Vec<PlotIdx> = Vec::new();
    for &u in &g.players[p].units {
        let Some(q) = g.unit_plot_opt(u) else { continue };
        if seen.contains(&q) || g.plot_is_water(q) {
            continue;
        }
        seen.push(q);
        let n = fighters_on(g, p, q).len();
        if n < min || !near(q) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        let key = (g.plot_distance(fx, fy, x, y), -(n as i32), q);
        if best.map_or(true, |b| (key.0, key.1) < (b.0, b.1)) {
            best = Some(key);
        }
    }
    best.map(|b| b.2)
}

/// A city of a civ at war with `team`, next to `plot`, with Sentinels standing.
fn walled_enemy_next_to(g: &Game, team: TeamId, plot: PlotIdx) -> bool {
    let (x, y) = g.map.xy(plot);
    (0..DirectionType::NUM).any(|i| {
        g.plot_direction(x, y, DirectionType::from_usize(i)).is_some_and(|q| {
            g.map.plots[q].plot_city.is_some_and(|c| g.cities.contains(c) && g.teams[team].at_war[g.city_team(c)] && !g.city_sentinels_standing(c).is_empty())
        })
    })
}

/// Powder Charges: blow the walls the stack beside them besieges; else join the nearest stack
/// that is near a walled enemy city.
fn powder_charges(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(charge) = ty::<UnitType>(g, "UNIT_POWDER_CHARGE") else { return };
    let Some(spell) = ty::<SpellType>(g, "SPELL_POWDER_CHARGE") else { return };
    let team = v.team;
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.units[u].unit_type != charge || g.unit_is_cargo(u) {
            continue;
        }
        let here = g.unit_plot(u);
        if g.unit_can_cast(u, spell, false) && fighters_on(g, p, here).len() >= 3 {
            crate::magic::cast(g, u, spell, "powder_charge", 0);
            continue;
        }
        if fighters_on(g, p, here).len() >= 3 {
            join_stack(g, u);
            continue;
        }
        if !g.unit_can_move(u) {
            continue;
        }
        let Some(target) = nearest_stack(g, p, here, 3, |q| {
            let (x, y) = g.map.xy(q);
            (-3..=3).any(|dx| (-3..=3).any(|dy| g.plot_xy(x, y, dx, dy).is_some_and(|r| walled_enemy_next_to(g, team, r))))
        }) else {
            continue;
        };
        if target != here {
            if let Some(grp) = own_group(g, u) {
                move_to(g, grp, target, MOVE_IGNORE_DANGER);
            }
        }
    }
}

/// Whether group `grp` has a Siege Lines job.
pub fn escorted_siege_work(g: &Game, grp: GroupId) -> bool {
    g.groups[grp].mission_queue.iter().any(|m| {
        m.mission_type == MissionType::MISSION_BUILD
            && m.data1 >= 0
            && (m.data1 as usize) < g.d.builds.len()
            && g.d.builds[BuildType::from_usize(m.data1 as usize)].improvement.is_some_and(|i| g.d.improvements[i].enemy_city_adjacent)
    })
}

/// Siege Lines: one unit at a time of our largest stack next to an enemy city builds them under it
/// (army units build the field works, docs/WORKS.md).
fn siege_lines(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(build) = g.d.builds.keys().find(|&b| g.d.builds[b].improvement.is_some_and(|i| g.d.improvements[i].enemy_city_adjacent)) else { return };
    if !g.team_is_has_tech(v.team, g.d.builds[build].tech_prereq) {
        return;
    }
    // One job at a time.
    if g.players[p].groups.iter().any(|&grp| alive(g, grp) && escorted_siege_work(g, grp)) {
        return;
    }
    let imp = g.d.builds[build].improvement;
    let mut best: Option<(usize, PlotIdx)> = None;
    let mut seen = Vec::new();
    for &u in &g.players[p].units {
        let Some(q) = g.unit_plot_opt(u) else { continue };
        if seen.contains(&q) {
            continue;
        }
        seen.push(q);
        let n = fighters_on(g, p, q).len();
        if n >= 4 && g.map.plots[q].improvement_type != imp && g.plot_can_build(q, build, p, false) && best.map_or(true, |(m, _)| n > m) {
            best = Some((n, q));
        }
    }
    let Some((_, site)) = best else { return };
    // The stack's weakest unit that can build them digs; the others stay free to fight.
    let digger = fighters_on(g, p, site).into_iter().filter(|&u| g.unit_can_build(u, site, build, false) && !g.unit_is_cargo(u)).min_by_key(|&u| (g.unit_base_combat_str(u), u));
    let Some(digger) = digger else { return };
    let Some(grp) = own_group(g, digger) else { return };
    g.group_clear_mission_queue(grp);
    push(g, grp, MissionType::MISSION_BUILD, build.i(), -1, 0, false);
    g.groups[grp].force_update = false;
}

/// False Colours: a Masker in a stack next to units of the civ we prepare war on strikes first;
/// else it joins our largest stack.
fn false_colours(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(masker) = ty::<UnitType>(g, "UNIT_MASKER") else { return };
    let Some(spell) = ty::<SpellType>(g, "SPELL_FALSE_COLOURS") else { return };
    let target_team = crate::diplomacy::preparing_target(g, v.team);
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.units[u].unit_type != masker || g.unit_is_cargo(u) {
            continue;
        }
        let here = g.unit_plot(u);
        let ours = fighters_on(g, p, here);
        if let Some(t) = target_team {
            // The weakest plot of theirs next to us that our stack outnumbers twice.
            let (x, y) = g.map.xy(here);
            let mut best: Option<(usize, PlotIdx)> = None;
            for i in 0..DirectionType::NUM {
                let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
                if g.plot_is_water(q) {
                    continue;
                }
                let theirs = g.map.plots[q].units.iter().filter(|&&d| g.unit_get_team(d) == t && g.unit_can_defend(d, Some(q)) && !g.unit_is_invisible(d, v.team, false)).count();
                if theirs > 0 && ours.len() >= 3 && ours.len() >= 2 * theirs && best.map_or(true, |(m, _)| theirs < m) {
                    best = Some((theirs, q));
                }
            }
            if let Some((_, q)) = best {
                if g.unit_can_cast(u, spell, false) {
                    crate::magic::cast(g, u, spell, "false_colours", 0);
                }
                if let Some(&a) = ours.iter().find(|&&a| g.units.contains(a) && g.unit_has_false_colours(a)) {
                    if let Some(grp) = g.units[a].group {
                        let (qx, qy) = g.map.xy(q);
                        crate::trace!(g, p, "False Colours: strikes team {:?} at ({qx},{qy})", t);
                        let _ = g.group_attack_force(grp, qx, qy, MOVE_DIRECT_ATTACK, None);
                    }
                }
                continue;
            }
        }
        // Keep with the army: our largest stack.
        if !g.units.contains(u) {
            continue;
        }
        if ours.len() >= 3 {
            join_stack(g, u);
            continue;
        }
        if !g.unit_can_move(u) {
            continue;
        }
        if let Some(target) = nearest_stack(g, p, here, 3, |_| true) {
            if target != here {
                if let Some(grp) = own_group(g, u) {
                    move_to(g, grp, target, 0);
                }
            }
        }
    }
}

/// Wheeled City: a city with it that is about to fall packs up; a caravan walks to the best safe
/// site within range of its origin and refounds the city there.
fn wheeled_cities(g: &mut Game, v: &View) {
    let p = v.p;
    let pack = ty::<SpellType>(g, "SPELL_PACK_UP");
    let unpack = ty::<SpellType>(g, "SPELL_UNPACK");
    let (Some(pack), Some(unpack)) = (pack, unpack) else { return };
    if let Some(wheels) = ty::<BuildingType>(g, "BUILDING_WHEELED_CITY") {
        for c in g.players[p].cities.clone() {
            if !g.cities.contains(c) || g.city_num_building(c, wheels) == 0 || !g.city_can_pack_up(c) || !g.city_is_besieged(c) {
                continue;
            }
            let plot = g.city_plot(c);
            let (_, defense) = crate::production::garrison(g, p, plot);
            let threat = crate::production::local_threat(g, p, plot, 2);
            if g.cities[c].population < 3 || threat <= 3 * defense.max(1) {
                continue;
            }
            if let Some(&caster) = g.map.plots[plot].units.iter().find(|&&u| g.units[u].owner == p && g.unit_can_cast(u, pack, false)) {
                crate::magic::cast(g, caster, pack, "wheeled_city", threat);
            }
        }
    }
    let mut flood = Flood::default();
    for u in g.players[p].units.clone() {
        if !g.units.contains(u) || g.units[u].packed_city.is_none() || g.unit_is_cargo(u) {
            continue;
        }
        let here = g.unit_plot(u);
        if v.danger_at(here) == 0 && g.unit_can_cast(u, unpack, false) && site_value(g, p, here) > 0 {
            crate::magic::cast(g, u, unpack, "wheeled_city", 0);
            continue;
        }
        if !g.unit_can_move(u) {
            continue;
        }
        let origin = g.units[u].packed_city.as_ref().map(|pc| pc.origin).unwrap_or(here);
        let (ox, oy) = g.map.xy(origin);
        let reach = flood.run(g, u, 8, false);
        let best = reach
            .iter()
            .filter(|r| {
                let (x, y) = g.map.xy(r.plot);
                g.plot_distance(x, y, ox, oy) <= UNPACK_RANGE && v.danger_at(r.plot) == 0 && g.player_can_found(p, x, y, false)
            })
            .map(|r| (site_value(g, p, r.plot) - 200 * r.turns, r.plot))
            .filter(|&(val, _)| val > 0)
            .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        if let Some((_, site)) = best {
            if site != here {
                if let Some(grp) = own_group(g, u) {
                    move_to(g, grp, site, 0);
                }
            }
        }
    }
}

fn site_value(g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
    let (x, y) = g.map.xy(plot);
    g.player_ai_found_value(p, x, y, -1, false, &[])
}
