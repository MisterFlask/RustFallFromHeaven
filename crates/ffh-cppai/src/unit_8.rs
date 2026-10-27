//! Port of CvUnitAI.cpp:19728-22704 (FFH2 0.41 C++ AI).
//!
//! Sephi's BETTER AI unit moves: barbarian smashers, feasting vampires, permanent city defense,
//! patrol and settler groups, conquest stacks, heroes (Govannon, Loki, Rantine), mages and
//! their spell casting, naval settling, the Inquisition and Lanun pirate coves.
//!
//! Notes:
//! - `getGroup()` is read afresh at every use (units leave and join groups in these loops).
//! - Loops over a plot's or a group's unit list iterate a copy of the list taken at the start;
//!   the C++ linked-list iteration takes the next node before acting on the current one, so the
//!   copy visits the same units (every condition is evaluated live).
//! - `getPathEndTurnPlot()` is read right after the `generatePath` it belongs to (the C++ reads
//!   it a few calls later, none of which generate a path).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// C++ `NO_SPELL` (an `int` spell from `GC.getDefineINT`).
const NO_SPELL: i32 = -1;

/// C++ `getGroup()` of a unit (NULL when it has none or the group is gone).
fn grp(g: &Game, u: UnitId) -> Option<GroupId> {
    g.units.get(u).and_then(|unit| unit.group).filter(|&x| g.groups.contains(x))
}

/// C++ `pGroup->getNumUnits()`.
fn group_num_units(g: &Game, x: GroupId) -> i32 {
    g.groups.get(x).map_or(0, |group| group.units.len() as i32)
}

/// C++ `getGroup()->getNumUnits()`.
fn num_units(g: &Game, u: UnitId) -> i32 {
    grp(g, u).map_or(0, |x| group_num_units(g, x))
}

/// C++ `getGroup()->getLengthMissionQueue()`.
fn queue_len(g: &Game, u: UnitId) -> usize {
    grp(g, u).map_or(0, |x| g.groups[x].mission_queue.len())
}

/// C++ `getGroup()->pushMission(eMission, iData1, iData2, iFlags, false, false, eMissionAI,
/// pMissionAIPlot, pMissionAIUnit)`.
#[allow(clippy::too_many_arguments)]
fn push(
    g: &mut Game,
    u: UnitId,
    mission: MissionType,
    data1: i32,
    data2: i32,
    flags: i32,
    mission_ai: Option<MissionAIType>,
    ai_plot: Option<PlotIdx>,
    ai_unit: Option<UnitId>,
) {
    if !g.units.contains(u) {
        return;
    }
    if let Some(x) = grp(g, u) {
        g.group_push_mission(x, mission, data1, data2, flags, false, false, mission_ai, ai_plot, ai_unit);
    }
}

/// C++ `getGroup()->pushMission(MISSION_SKIP)`.
fn push_skip(g: &mut Game, u: UnitId) {
    push(g, u, MissionType::MISSION_SKIP, -1, -1, 0, None, None, None);
}

/// C++ `getGroup()->pushMission(MISSION_MOVE_TO, pPlot->getX(), pPlot->getY(), iFlags, false,
/// false, eMissionAI, pMissionAIPlot)`.
fn push_move_to(g: &mut Game, u: UnitId, plot: PlotIdx, flags: i32, mission_ai: Option<MissionAIType>, ai_plot: Option<PlotIdx>) {
    let (x, y) = g.map.xy(plot);
    push(g, u, MissionType::MISSION_MOVE_TO, x, y, flags, mission_ai, ai_plot, None);
}

/// C++ `getGroup()->pushMission(MISSION_MOVE_TO_UNIT, pTarget->getOwner(), pTarget->getID(), 0,
/// false, false, eMissionAI, NULL, pTarget)`.
fn push_move_to_unit(g: &mut Game, u: UnitId, target: UnitId, mission_ai: Option<MissionAIType>) {
    let owner = g.units[target].owner;
    push(g, u, MissionType::MISSION_MOVE_TO_UNIT, owner.i(), target.i(), 0, mission_ai, None, Some(target));
}

/// C++ `pUnit->getArea()` for a unit that may be off the map (None).
fn unit_area_opt(g: &Game, u: UnitId) -> Option<AreaId> {
    g.unit_plot_opt(u).and_then(|p| g.map.plots[p].area)
}

/// C++ `pPlot->isCity()`.
fn is_city(g: &Game, plot: PlotIdx) -> bool {
    g.plot_is_city(plot, false, None)
}

/// C++ `pPlot->getPlotCity()`.
fn plot_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))
}

/// C++ `pPlot->getOwnerINLINE() == ePlayer`.
fn owned_by(g: &Game, plot: PlotIdx, p: PlayerId) -> bool {
    g.map.plots[plot].owner == Some(p)
}

/// C++ `generatePath(pPlot, 0, true, &iPathTurns)` for unit `u`.
fn path(g: &Game, u: UnitId, plot: PlotIdx, turns: &mut i32) -> bool {
    g.unit_generate_path(u, plot, 0, true, Some(turns))
}

/// C++ `GET_TEAM(eTeam).isAtWar(pPlot->getTeam())` (false for an unowned plot).
fn team_at_war_with_plot(g: &Game, t: TeamId, plot: PlotIdx) -> bool {
    g.plot_get_team(plot).map_or(false, |pt| g.teams[t].at_war[pt])
}

/// C++ `pArea->getNumAIUnits(ePlayer, eUnitAI)` with a possibly NULL area (0).
fn area_num_ai_units(g: &Game, area: Option<AreaId>, p: PlayerId, unit_ai: UnitAIType) -> i32 {
    match area {
        Some(a) if g.map.areas.contains(a) => g.map.areas[a].num_ai_units[p][unit_ai],
        _ => 0,
    }
}

/// C++ `(SpellTypes)GC.getDefineINT(..)` (None = NO_SPELL).
fn spell_define(v: i32) -> Option<SpellType> {
    opt_from_i32::<SpellType>(v)
}

/// The cargo counts both naval settle moves start with: settlers and workers carried, total
/// cargo capacity and free cargo space over the group.
fn naval_cargo_counts(g: &Game, u: UnitId) -> (i32, i32, i32, i32) {
    let mut settler_count = 0;
    let mut worker_count = 0;
    let mut total_cargo = 0;
    let mut total_cargo_space = 0;
    if let Some(my) = grp(g, u) {
        for &lu in &g.groups[my].units {
            settler_count += g.unit_get_unit_ai_cargo(lu, UnitAIType::UNITAI_SETTLE);
            worker_count += g.unit_get_unit_ai_cargo(lu, UnitAIType::UNITAI_WORKER);
            total_cargo += g.units[lu].cargo_capacity;
            total_cargo_space += 0.max(g.units[lu].cargo_capacity - g.units[lu].cargo);
        }
    }
    (settler_count, worker_count, total_cargo, total_cargo_space)
}

impl Cai {
    /// C++ `void CvUnitAI::AI_barbsmashermove()` (CvUnitAI.cpp:19728-20066)
    pub fn unit_ai_barbsmashermove(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        let _odds_threshold = 30;
        let min_stack = 1;
        let range = 20;
        // C++ `bFollow` is false: only its `generatePath` branches are ported.

        let ai = g.units[u].ai_type;
        if !(ai == Some(UnitAIType::UNITAI_HERO) || ai == Some(UnitAIType::UNITAI_WARWIZARD)) {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_BARBSMASHER);
        }

        // If player is in ConquestMode, seperate
        if g.players[owner].conquest_mode {
            let pl = g.unit_plot(u);
            for lu in g.map.plots[pl].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                if g.units[lu].ai_groupflag == GROUPFLAG_DEFENSE_NEW {
                    g.unit_join_group(lu, None, true);
                    g.units[lu].ai_groupflag = GROUPFLAG_CONQUEST;
                    push_skip(g, lu);
                }
            }
            return;
        }

        if num_units(g, u) == 1 {
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_BARBSMASHER, -1, -1, -1, false, false, false, 20, false) {
                if !g.units.contains(u) {
                    return;
                }
                // TEMPFIX to stop WoC
                if queue_len(g, u) == 0 {
                    // Make sure we push a Mission if joining a group failed
                    push_skip(g, u);
                }
            }
        }
        if !g.units.contains(u) {
            return;
        }

        if num_units(g, u) == 1 {
            let mut best_value = 1;
            let mut best_unit: Option<UnitId> = None;
            let my = grp(g, u);

            for lg in g.players[owner].groups.clone() {
                if !g.groups.contains(lg) {
                    continue;
                }
                let Some(head) = g.group_get_head_unit(lg) else { continue };
                if g.units[head].ai_groupflag != GROUPFLAG_DEFENSE_NEW || Some(lg) == my {
                    continue;
                }
                let Some(lp) = g.unit_plot_opt(head) else { continue };
                if !self.unit_ai_plot_valid(g, u, lp) {
                    continue;
                }
                if g.unit_is_enemy(u, g.plot_get_team(lp), None) {
                    continue;
                }
                if !self.unit_ai_allow_group(g, u, head, UnitAIType::UNITAI_UNKNOWN) {
                    continue;
                }
                if g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                    continue;
                }
                let mut turns = 0;
                if path(g, u, lp, &mut turns) {
                    let value = group_num_units(g, lg);
                    if value >= best_value {
                        best_value = value;
                        best_unit = Some(head);
                    }
                }
            }

            if let Some(best) = best_unit {
                if g.unit_plot_opt(best).map_or(false, |bp| g.unit_at_plot(u, bp)) {
                    let bg = grp(g, best);
                    g.unit_join_group(u, bg, true);
                    if queue_len(g, u) == 0 {
                        // Make sure we push a Mission if joining a group failed
                        push_skip(g, u);
                    }
                } else {
                    push_move_to_unit(g, u, best, Some(MissionAIType::MISSIONAI_GROUP));
                }
                return;
            }
        }

        // When not at war, maybe City needs defense?
        // Are we in A City that needs Defense?
        let pl = g.unit_plot(u);
        if is_city(g, pl) && g.team_get_at_war_count(team, true) == 0 {
            if owned_by(g, pl, owner) {
                if let Some(c) = plot_city(g, pl) {
                    if self.city_ai_needed_perm_defense(g, c, 0) > 0 {
                        // Split so we can join the Defense
                        let wait_after_split = num_units(g, u) == 1;

                        let members = grp(g, u).map_or(Vec::new(), |my| g.groups[my].units.clone());
                        for lu in members {
                            if !g.units.contains(lu) {
                                continue;
                            }
                            if self.unit_ai_is_unit_allowed_perm_defense(g, lu) {
                                let lg = grp(g, lu);
                                let not_head_or_alone = lg.map_or(false, |lg| {
                                    g.group_get_head_unit(lg) != Some(lu) || group_num_units(g, lg) == 1
                                });
                                if not_head_or_alone {
                                    if g.units[u].ai_type == Some(UnitAIType::UNITAI_BARBSMASHER) {
                                        g.unit_join_group(lu, None, true);
                                        g.units[lu].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                                        g.unit_ai_set_unit_ai_type(lu, UnitAIType::UNITAI_CITY_DEFENSE);
                                        push_skip(g, lu);
                                    }
                                }
                            }
                        }
                        if wait_after_split {
                            push_skip(g, u);
                            return;
                        }
                    }
                }
            }
        }

        if self.unit_ai_groupheal(g, u, 30, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_plot: Option<PlotIdx> = None;
        let search_range = 20;
        let mut best_value = -100;
        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                if !(self.unit_ai_plot_valid(g, u, lp) && owned_by(g, lp, owner)) {
                    continue;
                }
                if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                    continue;
                }
                let mut turns = 0;
                if g.unit_at_plot(u, lp) || !(path(g, u, lp, &mut turns) && turns <= range) {
                    continue;
                }
                let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                if g.plot_get_num_visible_enemy_defenders(lp, u) >= min_stack {
                    let Some(my) = grp(g, u) else { continue };
                    let our_strength = self.group_ai_group_power(g, my, lp, false);
                    let their_strength = self.player_ai_get_enemy_plot_strength(g, owner, lp, 0, true, false);
                    if (our_strength as f64) > (their_strength as f64) * 1.2 && their_strength > 0 {
                        let mut value = self.group_ai_attack_odds(g, my, lp, true);
                        value *= 100 / (1 + turns);
                        value *= 3 + g.plot_get_num_visible_enemy_defenders(lp, u);
                        if value > best_value {
                            best_value = value;
                            best_plot = end_turn_plot;
                        }
                    }
                }
            }
        }
        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, 0, None, None);
            return;
        }

        // look for barb cities
        let search_range = 5;
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;

        for c in g.players[owner].cities.clone() {
            if !g.cities.contains(c) {
                continue;
            }
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(cx, cy, dx, dy) else { continue };
                    if !self.unit_ai_plot_valid(g, u, lp) {
                        continue;
                    }
                    if !is_city(g, lp) {
                        continue;
                    }
                    if !team_at_war_with_plot(g, team, lp) {
                        continue;
                    }
                    // Avoid hopeless causes
                    // FIX: the C++ asks for the best defender of our own city here
                    // (`pLoopCity->plot()`, our units: always NULL against us), so the check never
                    // fired; the target city's defender is meant.
                    let target_owner = g.map.plots[lp].owner;
                    let best_defender = g.plot_get_best_defender(lp, target_owner, Some(owner), Some(u), false, false, false);
                    let hopeful = match best_defender {
                        None => true,
                        Some(d) => {
                            !g.unit_is_fear(d)
                                || g.unit_is_immune_to_fear(u)
                                || g.unit_base_combat_str(u) > g.unit_base_combat_str_defense(d)
                        }
                    };
                    if !hopeful {
                        continue;
                    }
                    let mut turns = 0;
                    if path(g, u, lp, &mut turns) {
                        let Some(my) = grp(g, u) else { continue };
                        let mut our_strength = self.group_ai_group_power(g, my, lp, false);
                        let their_strength = self.player_ai_get_enemy_plot_strength(g, owner, lp, 0, true, false);
                        if num_units(g, u) >= g.plot_get_num_visible_enemy_defenders(lp, u) * 3 {
                            our_strength = (our_strength as f64 * 1.7) as i32;
                        }

                        if (our_strength as f64) > (their_strength as f64) * 1.2 && their_strength > 0 {
                            let value = (our_strength * 100 / their_strength) / (1 + turns);
                            if value > best_value {
                                best_value = value;
                                best_plot = Some(lp);
                            }
                        }
                    }
                }
            }
        }
        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, MOVE_DIRECT_ATTACK, None, None);
            return;
        }

        // pick up reinforcements
        if self.unit_ai_group(g, u, UnitAIType::UNITAI_BARBSMASHER, -1, -1, -1, false, false, true, 3, true) {
            // TEMPFIX to stop WoC
            if g.units.contains(u) && queue_len(g, u) == 0 {
                // Make sure we push a Mission if joining a group failed
                push_skip(g, u);
            }
            return;
        }

        if self.unit_ai_groupheal(g, u, 1, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        if owned_by(g, g.unit_plot(u), owner) {
            if self.unit_ai_goody(g, u, 1) {
                return;
            }

            if self.rand(100, "CheckUnitforUpgrade") < 30 {
                if self.unit_ai_travel_to_upgrade_city(g, u) {
                    return;
                }
            }

            if self.unit_ai_patrol(g, u) {
                return;
            }
        }
        if !g.units.contains(u) {
            return;
        }

        if !is_city(g, g.unit_plot(u)) {
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::AI_feastingmove()` (CvUnitAI.cpp:20068-20109)
    pub fn unit_ai_feastingmove(&self, g: &mut Game, u: UnitId) {
        let feast = g.d.defines.spell_feast;
        if feast != NO_SPELL {
            let unit_type = g.units[u].unit_type.i();
            if !(unit_type == g.d.defines.unit_vampire || unit_type == g.d.defines.unit_vampire_lord) {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK);
                push_skip(g, u);
                return;
            } else if g.units[u].experience > 30 || !is_city(g, g.unit_plot(u)) {
                g.units[u].ai_groupflag = GROUPFLAG_CONQUEST;
                push_skip(g, u);
                return;
            }
            if let Some(c) = plot_city(g, g.unit_plot(u)) {
                if g.cities[c].population > 13 {
                    if let Some(spell) = spell_define(feast) {
                        if crate::ai_can_cast(g, u, spell, false) {
                            g.tel_cast_reason(u, "feastingmove", 0);
                            g.unit_cast(u, spell);
                        }
                    }
                    push_skip(g, u);
                    return;
                } else {
                    g.units[u].ai_groupflag = GROUPFLAG_CONQUEST;
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK_CITY);
                    push_skip(g, u);
                    return;
                }
            }
        }
        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::PermDefenseNewMove()` (CvUnitAI.cpp:20111-20381)
    pub fn unit_ai_perm_defense_new_move(&self, g: &mut Game, u: UnitId) {
        if g.units[u].ai_groupflag != GROUPFLAG_PERMDEFENSE_NEW {
            return;
        }
        if num_units(g, u) > 1 {
            push_skip(g, u);
            g.unit_join_group(u, None, true);
            push_skip(g, u);
            return;
        }

        let owner = g.units[u].owner;

        // Needed perm defense of city `c` for this unit's AI type (C++ `bvalid` / `iMod`).
        let needed_for_type = |cai: &Cai, g: &Game, c: CityId| -> Option<i32> {
            let ai = g.units[u].ai_type;
            let mut result = None;
            if ai == Some(UnitAIType::UNITAI_CITY_DEFENSE) && cai.city_ai_needed_perm_defense(g, c, 0) > 0 {
                result = Some(cai.city_ai_needed_perm_defense(g, c, 0));
            }
            if ai == Some(UnitAIType::UNITAI_CITY_COUNTER) && cai.city_ai_needed_perm_defense(g, c, 1) > 0 {
                result = Some(cai.city_ai_needed_perm_defense(g, c, 1));
            }
            if ai == Some(UnitAIType::UNITAI_MAGE) && cai.city_ai_needed_perm_defense(g, c, 2) > 0 {
                result = Some(cai.city_ai_needed_perm_defense(g, c, 2));
            }
            if ai == Some(UnitAIType::UNITAI_MEDIC) && cai.city_ai_needed_perm_defense(g, c, 3) > 0 {
                result = Some(cai.city_ai_needed_perm_defense(g, c, 3));
            }
            result
        };

        // Unit in a City that needs Defense?
        let pl = g.unit_plot(u);
        if is_city(g, pl) && owned_by(g, pl, owner) {
            if let Some(c) = plot_city(g, pl) {
                if needed_for_type(self, g, c).is_some() {
                    for lu in g.map.plots[pl].units.clone() {
                        if !g.units.contains(lu) {
                            continue;
                        }
                        if g.units[lu].ai_groupflag == GROUPFLAG_PERMDEFENSE {
                            g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE;
                            let lg = grp(g, lu);
                            g.unit_join_group(u, lg, true);
                            push_skip(g, u);
                            self.city_ai_calculate_needed_perm_defense(g, c); // recalculate City Defense
                            return;
                        }
                    }
                    g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE;
                    push_skip(g, u);
                    self.city_ai_calculate_needed_perm_defense(g, c); // recalculate City Defense
                    return;
                }
            }
        }

        // Check for Units specialized in Defense
        let (bd1, bd2, bd3) = match g.player_get_civilization_type(owner) {
            Some(civ) => {
                let ci = &g.d.civilizations[civ];
                (ci.best_defender1, ci.best_defender2, ci.best_defender3)
            }
            None => (None, None, None),
        };
        let class = Some(g.unit_get_unit_class_type(u));
        let reinforcement = class == bd1 || class == bd2 || class == bd3;

        let (ux, uy) = (g.units[u].x, g.units[u].y);

        if reinforcement {
            let mut replacement: Option<UnitId> = None;
            let search_range = 8;
            let mut best_value = 10;
            let my_ai = g.units[u].ai_type;
            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                    if !is_city(g, lp) || !owned_by(g, lp, owner) {
                        continue;
                    }
                    for &lu in &g.map.plots[lp].units {
                        if g.units[lu].ai_groupflag != GROUPFLAG_PERMDEFENSE || g.units[lu].ai_type != my_ai {
                            continue;
                        }
                        let lclass = Some(g.unit_get_unit_class_type(lu));
                        if class == bd1 {
                            if lclass != bd1 && lclass != bd2 && lclass != bd3 {
                                let mut turns = 0;
                                if path(g, u, lp, &mut turns) && turns < best_value {
                                    replacement = Some(lu);
                                    best_value = turns;
                                }
                            }
                        }

                        if class == bd2 {
                            if lclass != bd2 && lclass != bd3 {
                                let mut turns = 0;
                                if path(g, u, lp, &mut turns) && turns < best_value {
                                    replacement = Some(lu);
                                    best_value = turns;
                                }
                            }
                        }

                        if class == bd3 {
                            if lclass != bd3 {
                                let mut turns = 0;
                                if path(g, u, lp, &mut turns) && turns < best_value {
                                    replacement = Some(lu);
                                    best_value = turns;
                                }
                            }
                        }
                    }
                }
            }
            if let Some(rep) = replacement {
                let rp = g.unit_plot(rep);
                if g.unit_at_plot(u, rp) {
                    g.units[u].ai_groupflag = GROUPFLAG_PERMDEFENSE;
                    let rg = grp(g, rep);
                    g.unit_join_group(u, rg, true);
                    push_skip(g, u);
                    g.unit_join_group(rep, None, true);
                    g.units[rep].ai_groupflag = GROUPFLAG_NONE;
                    push_skip(g, rep);
                } else {
                    push_move_to(g, u, rp, MOVE_DIRECT_ATTACK, None, None);
                }
                return;
            }
            // Look for Cities around
        }

        // Look for Cities around that need DefHelp
        let search_range = 10;
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 0;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                if !is_city(g, lp) || !owned_by(g, lp, owner) {
                    continue;
                }
                let Some(c) = plot_city(g, lp) else { continue };
                if let Some(imod) = needed_for_type(self, g, c) {
                    let mut turns = 0;
                    if path(g, u, lp, &mut turns) {
                        // A found path has at least 1 turn (guarded against 0).
                        let value = (imod * 100) / turns.max(1);
                        if value > best_value {
                            best_plot = Some(lp);
                            best_value = value;
                        }
                    }
                }
            }
        }
        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, MOVE_DIRECT_ATTACK, None, None);
            return;
        }
        g.units[u].ai_groupflag = GROUPFLAG_NONE;
        if g.units[u].ai_type == Some(UnitAIType::UNITAI_MAGE) {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_WARWIZARD);
        }
        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::PermDefenseMove()` (CvUnitAI.cpp:20384-20395)
    pub fn unit_ai_perm_defense_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let pl = g.unit_plot(u);
        if !is_city(g, pl) || !owned_by(g, pl, owner) {
            g.units[u].ai_groupflag = GROUPFLAG_NONE;
            push_skip(g, u);
            return;
        }

        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::PatrolNewMove()` (CvUnitAI.cpp:20397-20565)
    pub fn unit_ai_patrol_new_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let search_range = 10;

        let mut best_group: Option<GroupId> = None;
        let mut best_city: Option<CityId> = None;
        let mut best_value = 12;
        // C++ quirk: `newGroup` is uninitialized; it is only read together with a non-NULL
        // best city / group, which set it.
        let mut new_group = false;

        let flag = if g.units[u].ai_type == Some(UnitAIType::UNITAI_WARWIZARD) { 1 } else { 0 };

        let (ux, uy) = (g.units[u].x, g.units[u].y);

        // Search for A City nearby to join its Patrol
        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                if !is_city(g, lp) {
                    continue;
                }
                let mut turns = 0;
                if !(owned_by(g, lp, owner) && path(g, u, lp, &mut turns)) {
                    continue;
                }
                // found a City, What's its status?
                let Some(target_city) = plot_city(g, lp) else { continue };

                if self.city_ai_get_city_patrol_origin(g, target_city).is_none() {
                    // is a City closeby with a Patrolgroup?
                    let search_range2 = 6;
                    let (tx, ty) = (g.cities[target_city].x, g.cities[target_city].y);
                    for dx2 in -search_range2..=search_range2 {
                        for dy2 in -search_range2..=search_range2 {
                            let Some(lp2) = g.plot_xy(tx, ty, dx2, dy2) else { continue };
                            if !is_city(g, lp2) || !owned_by(g, lp2, owner) {
                                continue;
                            }
                            let Some(c2) = plot_city(g, lp2) else { continue };
                            if let Some(origin2) = self.city_ai_get_city_patrol_origin(g, c2) {
                                if lp2 == origin2 {
                                    let mut turns2 = 0;
                                    if path(g, u, lp2, &mut turns2) {
                                        self.city_ai_set_city_patrol_origin(g, target_city, origin2);
                                    }
                                }
                            }
                        }
                    }
                    // Else make the City the center of a new Patrolgroup
                    if self.city_ai_get_city_patrol_origin(g, target_city).is_none() {
                        let cp = g.city_plot(target_city);
                        self.city_ai_set_city_patrol_origin(g, target_city, cp);
                    }
                }

                // What's the Cities Patrolgroup?
                let target_origin = self.city_ai_get_city_patrol_origin(g, target_city);
                let mut patrolgroup: Option<GroupId> = None;
                for &pu in &g.players[owner].units {
                    if g.units[pu].ai_groupflag == GROUPFLAG_PATROL {
                        if g.unit_ai_get_origin_plot(pu) == target_origin {
                            patrolgroup = grp(g, pu);
                            break;
                        }
                    }
                }

                if let Some(origin) = target_origin {
                    let mut value = 0;
                    if path(g, u, origin, &mut value) {
                        if value < best_value {
                            if patrolgroup.is_none() {
                                best_value = value;
                                new_group = true;
                                best_city = Some(target_city);
                            } else if self.city_ai_needed_patrol(g, target_city, flag) > 0 {
                                best_value = value;
                                new_group = false;
                                best_group = patrolgroup;
                            }
                        }
                    }
                }
            }
        }

        if new_group && best_city.is_some() {
            let bc = best_city.unwrap();
            g.units[u].ai_groupflag = GROUPFLAG_PATROL;
            if flag == 0 {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_COUNTER);
            }
            let origin = if g.cities.contains(bc) { self.city_ai_get_city_patrol_origin(g, bc) } else { None };
            g.unit_ai_set_origin_plot(u, origin);
            push_skip(g, u);
            return;
        } else if !new_group && best_group.is_some() {
            let bg = best_group.unwrap();
            let head = if g.groups.contains(bg) { g.group_get_head_unit(bg) } else { None };
            let at = g.groups.contains(bg) && g.group_plot(bg).map_or(false, |gp| g.unit_at_plot(u, gp));
            if at {
                g.units[u].ai_groupflag = GROUPFLAG_PATROL;
                if flag == 0 {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_COUNTER);
                }
                let head_origin = head.and_then(|h| g.unit_ai_get_origin_plot(h));
                g.unit_ai_set_origin_plot(u, head_origin);

                if let Some(ho) = head_origin {
                    if let Some(oc) = plot_city(g, ho) {
                        self.city_ai_calculate_needed_patrol(g, oc); // recalculate Patrol
                    }
                }

                if g.groups.contains(bg) {
                    g.unit_join_group(u, Some(bg), true);
                }
            } else if let Some(h) = head {
                push_move_to_unit(g, u, h, Some(MissionAIType::MISSIONAI_GROUP));
            }
            return;
        }

        if self.unit_ai_is_unit_allowed_defense_group(g, u) {
            g.units[u].ai_groupflag = GROUPFLAG_DEFENSE_NEW;
            push_skip(g, u);
            return;
        }

        g.units[u].ai_groupflag = GROUPFLAG_NONE;
        push_skip(g, u);
    }

    /// C++ `bool CvUnitAI::RejoinPatrolGroup()` (CvUnitAI.cpp:20568-20604)
    /// Returns true if a Mission was pushed.
    pub fn unit_ai_rejoin_patrol_group(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let mut best_value = -1;
        let mut best_unit: Option<UnitId> = None;
        let my_origin = g.unit_ai_get_origin_plot(u);

        for &pu in &g.players[owner].units {
            if g.units[pu].ai_groupflag == GROUPFLAG_PATROL {
                let pu_origin = g.unit_ai_get_origin_plot(pu);
                if pu_origin.is_some() && pu != u {
                    if pu_origin == my_origin {
                        let n = num_units(g, pu);
                        if n > best_value {
                            best_unit = Some(pu);
                            best_value = n;
                        }
                    }
                }
            }
        }
        if let Some(best) = best_unit {
            if g.unit_plot_opt(best).map_or(false, |bp| g.unit_at_plot(u, bp)) {
                let bg = grp(g, best);
                g.unit_join_group(u, bg, true);
            } else {
                push_move_to_unit(g, u, best, Some(MissionAIType::MISSIONAI_GROUP));
            }
            return true;
        }
        false
    }

    /// C++ `void CvUnitAI::PatrolMove()` (CvUnitAI.cpp:20606-20868)
    pub fn unit_ai_patrol_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        // C++ `bFollow` is false: only its `generatePath` branches are ported.
        let min_stack = 1;
        let mut best_value = -1;
        let mut best_plot: Option<PlotIdx> = None;

        let flag = if g.units[u].ai_type == Some(UnitAIType::UNITAI_WARWIZARD) { 1 } else { 0 };

        // every patrol should have a Citygroup it wants to defend
        if g.unit_ai_get_origin_plot(u).is_none() {
            push_skip(g, u);
            g.unit_join_group(u, None, true);
            g.units[u].ai_groupflag = GROUPFLAG_NONE;
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK);
            push_skip(g, u);
            return;
        }

        // Do we need to join Patrol group again?
        if num_units(g, u) == 1 {
            if self.unit_ai_rejoin_patrol_group(g, u) {
                return;
            }
        }

        // is there a City under Siege?
        if self.unit_ai_patrol_guardcity(g, u) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        // Look for Settlers to escort?
        // Cycle through Cities we protect
        for c in g.players[owner].cities.clone() {
            if !g.cities.contains(c) {
                continue;
            }
            let Some(city_origin) = self.city_ai_get_city_patrol_origin(g, c) else { continue };
            if Some(city_origin) != g.unit_ai_get_origin_plot(u) {
                continue;
            }
            let cp = g.city_plot(c);
            for lu in g.map.plots[cp].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                // There is a Settler! Either move to where he is or regroup
                if g.units[lu].ai_type != Some(UnitAIType::UNITAI_SETTLE) {
                    continue;
                }
                let can_join = grp(g, lu).map_or(false, |lg| self.unit_ai_can_join_group(g, u, lg));
                if !can_join {
                    continue;
                }
                if num_units(g, lu) < 4 {
                    if g.unit_plot_opt(lu).map_or(false, |lp| g.unit_at_plot(u, lp)) {
                        let mut enough = 0;
                        while enough < 4 {
                            // search for a unit without much experience and add it to the Settlergroup
                            let mut best_unit: Option<UnitId> = None;
                            let mut best_exp = 100;
                            let up = g.unit_plot(u);
                            for &lu1 in &g.map.plots[up].units {
                                if g.units[lu1].ai_groupflag == GROUPFLAG_PATROL
                                    && g.unit_ai_get_origin_plot(lu1) == g.unit_ai_get_origin_plot(u)
                                {
                                    if lu1 != u || num_units(g, u) == 1 {
                                        if num_units(g, lu) < 4 {
                                            if g.units[lu1].experience < best_exp {
                                                best_unit = Some(lu1);
                                                best_exp = g.units[lu1].experience;
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(b) = best_unit {
                                g.unit_join_group(b, None, true);
                                let settler_group = grp(g, lu);
                                g.unit_join_group(b, settler_group, true);
                                g.units[b].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                                g.unit_ai_set_origin_plot(b, None);
                                push_skip(g, b);
                                enough += 1;
                            } else {
                                // didn't find a unit to add to the settler, abort the loop
                                enough = 1000;
                            }
                        }
                        push_skip(g, u);
                        return;
                    } else {
                        push_move_to_unit(g, u, lu, Some(MissionAIType::MISSIONAI_GROUP));
                    }
                }
                return;
            }
        }

        // wait for reinforcements
        if num_units(g, u) == 1 {
            push_skip(g, u);
            return;
        }

        // Are we in A City that needs Defense?
        let pl = g.unit_plot(u);
        if is_city(g, pl) && owned_by(g, pl, owner) {
            if let Some(c) = plot_city(g, pl) {
                if self.city_ai_needed_perm_defense(g, c, 0) > 0 {
                    // Split so we can join the Defense
                    for lu in g.map.plots[pl].units.clone() {
                        if !g.units.contains(lu) {
                            continue;
                        }
                        if g.units[lu].owner == owner && g.units[lu].ai_groupflag == GROUPFLAG_PATROL {
                            if flag == 0 {
                                g.units[lu].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                                g.unit_ai_set_unit_ai_type(lu, UnitAIType::UNITAI_CITY_DEFENSE);
                            }
                            if flag == 1 {
                                g.units[lu].ai_groupflag = GROUPFLAG_NONE;
                            }
                            g.unit_ai_set_origin_plot(lu, None);
                            g.unit_join_group(lu, None, true);
                            push_skip(g, lu);
                        }
                    }
                    push_skip(g, u);
                    return;
                }
            }
        }

        // Is trouble Around?
        let my_origin = g.unit_ai_get_origin_plot(u);
        for c in g.players[owner].cities.clone() {
            let Some(city_origin) = self.city_ai_get_city_patrol_origin(g, c) else { continue };
            if Some(city_origin) != my_origin {
                continue;
            }
            let (sx, sy) = (g.cities[c].x, g.cities[c].y);
            let search_range = 4;
            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(sx, sy, dx, dy) else { continue };
                    if !(self.unit_ai_plot_valid(g, u, lp) && owned_by(g, lp, owner)) {
                        continue;
                    }
                    if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                        continue;
                    }
                    let mut turns = 0;
                    if g.unit_at_plot(u, lp) || !path(g, u, lp, &mut turns) {
                        continue;
                    }
                    let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                    if g.plot_get_num_visible_enemy_defenders(lp, u) >= min_stack {
                        let Some(my) = grp(g, u) else { continue };
                        let our_strength = self.group_ai_group_power(g, my, lp, false);
                        let their_strength = self.player_ai_get_enemy_plot_strength(g, owner, lp, 0, true, false);
                        if (our_strength as f64) > (their_strength as f64) * 1.2 {
                            let mut value = self.group_ai_attack_odds(g, my, lp, true);
                            if value > 70 {
                                value *= 100 / (1 + turns);
                                value *= 3 + g.plot_get_num_visible_enemy_defenders(lp, u);
                                if value > best_value {
                                    best_value = value;
                                    best_plot = end_turn_plot;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, 0, None, None);
            return;
        }

        if !is_city(g, g.unit_plot(u)) {
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::HNgroupMove()` (CvUnitAI.cpp:20870-20874)
    pub fn unit_ai_h_ngroup_move(&self, g: &mut Game, u: UnitId) {
        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::SettlergroupMove()` (CvUnitAI.cpp:20876-20940)
    pub fn unit_ai_settlergroup_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let pl = g.unit_plot(u);
        if !is_city(g, pl) || !owned_by(g, pl, owner) {
            g.unit_join_group(u, None, true);
            g.units[u].ai_groupflag = GROUPFLAG_NONE;
            push_skip(g, u);
            return;
        }

        let needs_defense = plot_city(g, pl).map_or(false, |c| self.city_ai_needed_perm_defense(g, c, 0) > 0);
        if needs_defense {
            // Are we in A City that needs Defense?
            // Split so we can join the Defense
            for lu in g.map.plots[pl].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                if g.units[lu].owner == owner && g.units[lu].ai_groupflag == GROUPFLAG_SETTLERGROUP {
                    g.units[lu].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                    g.unit_ai_set_unit_ai_type(lu, UnitAIType::UNITAI_CITY_DEFENSE);
                    g.unit_join_group(lu, None, true);
                    push_skip(g, lu);
                }
            }
        } else {
            // settler around that we can join?
            for lu in g.map.plots[pl].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                if g.units[lu].owner == owner && g.units[lu].ai_type == Some(UnitAIType::UNITAI_SETTLE) {
                    g.unit_join_group(u, None, true);
                    let sg = grp(g, lu);
                    g.unit_join_group(u, sg, true);
                    push_skip(g, lu);
                }
            }
        }
    }

    /// C++ `bool CvUnitAI::AI_patrol_guardcity()` (CvUnitAI.cpp:20943-20977)
    /// Returns true if the Patrol group decides to guard a city.
    pub fn unit_ai_patrol_guardcity(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let mut best_city: Option<PlotIdx> = None;
        let mut best_value = 0;
        let my_origin = g.unit_ai_get_origin_plot(u);
        for c in g.players[owner].cities.clone() {
            let Some(city_origin) = self.city_ai_get_city_patrol_origin(g, c) else { continue };
            if Some(city_origin) == my_origin {
                let danger = self.city_ai_city_danger_value(g, c);
                if danger > best_value {
                    best_city = Some(g.city_plot(c));
                    best_value = danger;
                }
            }
        }
        if let Some(bc) = best_city {
            if g.unit_at_plot(u, bc) {
                push_skip(g, u); // need to put in a check for an assault
            } else {
                push_move_to(g, u, bc, MOVE_IGNORE_DANGER, None, None);
            }
            return true;
        }
        false
    }

    /// C++ `void CvUnitAI::ConquestMove()` (CvUnitAI.cpp:20979-21420)
    pub fn unit_ai_conquest_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        // C++ `bFollow` is false: only its `generatePath` branches are ported.

        let ai = g.units[u].ai_type;
        if !(ai == Some(UnitAIType::UNITAI_HERO) || ai == Some(UnitAIType::UNITAI_WARWIZARD)) {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK_CITY);
        }

        if owned_by(g, g.unit_plot(u), owner) {
            let mut best_value = num_units(g, u);
            let mut best_unit: Option<UnitId> = None;
            let my = grp(g, u);

            if num_units(g, u) == 1 {
                for lg in g.players[owner].groups.clone() {
                    if !g.groups.contains(lg) {
                        continue;
                    }
                    let Some(head) = g.group_get_head_unit(lg) else { continue };
                    // TEMPFIX FOR HIDDEN NATIONALITY
                    if !(g.units[head].ai_groupflag == GROUPFLAG_CONQUEST && !g.unit_is_hidden_nationality(head)) {
                        continue;
                    }
                    if Some(lg) == my {
                        continue;
                    }
                    let Some(lp) = g.unit_plot_opt(head) else { continue };
                    if !self.unit_ai_plot_valid(g, u, lp) {
                        continue;
                    }
                    if g.unit_is_enemy(u, g.plot_get_team(lp), None) {
                        continue;
                    }
                    if !self.unit_ai_allow_group(g, u, head, UnitAIType::UNITAI_UNKNOWN) {
                        continue;
                    }
                    if g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                        continue;
                    }
                    let mut turns = 0;
                    if path(g, u, lp, &mut turns) {
                        let value = group_num_units(g, lg);
                        if value >= best_value {
                            best_value = value;
                            best_unit = Some(head);
                        }
                    }
                }
                if let Some(best) = best_unit {
                    if g.unit_plot_opt(best).map_or(false, |bp| g.unit_at_plot(u, bp)) {
                        let bg = grp(g, best);
                        g.unit_join_group(u, bg, true);
                        if queue_len(g, u) == 0 {
                            // Make sure we push a Mission if joining a group failed
                            push_skip(g, u);
                        }
                    } else {
                        push_move_to_unit(g, u, best, Some(MissionAIType::MISSIONAI_GROUP));
                    }
                    return;
                }
            } else {
                let mut splitup = false;
                for lg in g.players[owner].groups.clone() {
                    if !g.groups.contains(lg) {
                        continue;
                    }
                    let Some(head) = g.group_get_head_unit(lg) else { continue };
                    if g.units[head].ai_groupflag != GROUPFLAG_CONQUEST || Some(lg) == my {
                        continue;
                    }
                    let Some(pp) = g.unit_plot_opt(head) else { continue };
                    if !self.unit_ai_plot_valid(g, u, pp) {
                        continue;
                    }
                    if g.unit_is_enemy(u, g.plot_get_team(pp), None) {
                        continue;
                    }
                    if g.plot_is_visible_enemy_unit_for_unit(pp, u) {
                        continue;
                    }
                    if !self.unit_ai_allow_group(g, u, head, UnitAIType::UNITAI_UNKNOWN) {
                        continue;
                    }
                    let mut turns = 0;
                    if path(g, u, pp, &mut turns) {
                        if group_num_units(g, lg) >= num_units(g, u) {
                            splitup = true;
                        }
                    }
                }
                if splitup {
                    // FIX: the C++ calls `joinGroup(NULL)` on this unit for every member (it hops
                    // from new group to new group and only it leaves the stack); every member
                    // is meant to leave, to join the bigger conquest stack as a single unit.
                    let members = grp(g, u).map_or(Vec::new(), |my| g.groups[my].units.clone());
                    for lu in members {
                        if !g.units.contains(lu) {
                            continue;
                        }
                        g.unit_join_group(lu, None, true);
                        g.units[lu].ai_groupflag = GROUPFLAG_CONQUEST;
                        push_skip(g, lu);
                    }
                    return;
                }
                let n = num_units(g, u);
                if !(n > 15 || n > g.players[owner].cities.len() as i32 * 3) {
                    // check for enemies in own territory
                    let _odds_threshold = 70;
                    let min_stack = 1 + (n / 5);
                    let range = 20;
                    let mut best_plot: Option<PlotIdx> = None;
                    let search_range = 20;
                    let mut best_value = 0;
                    let (ux, uy) = (g.units[u].x, g.units[u].y);
                    for dx in -search_range..=search_range {
                        for dy in -search_range..=search_range {
                            let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                            if !(self.unit_ai_plot_valid(g, u, lp) && owned_by(g, lp, owner)) {
                                continue;
                            }
                            if !g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                                continue;
                            }
                            let mut turns = 0;
                            if g.unit_at_plot(u, lp) || !(path(g, u, lp, &mut turns) && turns <= range) {
                                continue;
                            }
                            let end_turn_plot = g.unit_get_path_end_turn_plot(u);
                            if g.plot_get_num_visible_enemy_defenders(lp, u) >= min_stack {
                                let Some(my) = grp(g, u) else { continue };
                                let our_strength = self.group_ai_group_power(g, my, lp, false);
                                let their_strength = self.player_ai_get_enemy_plot_strength(g, owner, lp, 0, true, false);
                                if (our_strength as f64) > (their_strength as f64) * 1.2 {
                                    let mut value = self.unit_ai_attack_odds(g, u, lp, true);
                                    if value > 30 {
                                        value *= 100 / (1 + turns);
                                        value *= 3 + g.plot_get_num_visible_enemy_defenders(lp, u);
                                        if value > best_value {
                                            best_value = value;
                                            best_plot = end_turn_plot;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(bp) = best_plot {
                        push_move_to(g, u, bp, 0, None, None);
                        return;
                    }
                    // Wait until we have more troops
                    if !is_city(g, g.unit_plot(u)) {
                        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                            return;
                        }
                    }
                    if !g.units.contains(u) {
                        return;
                    }
                }
            }
        }

        if self.unit_ai_groupheal(g, u, 70, MAX_INT) {
            return;
        }
        if self.unit_ai_groupheal(g, u, 50, 1) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }
        if is_city(g, g.unit_plot(u)) {
            if self.unit_ai_groupheal(g, u, 30, 1) {
                return;
            }
        }
        if !g.units.contains(u) {
            return;
        }

        // Are we in A City that needs Defense?
        let pl = g.unit_plot(u);
        if is_city(g, pl) && owned_by(g, pl, owner) {
            // && getGroup()->getNumUnits()>30)
            if let Some(c) = plot_city(g, pl) {
                if self.city_ai_needed_perm_defense(g, c, 0) > 0 {
                    // Split so we can join the Defense
                    let mut enough = 0;
                    while enough < 2 {
                        // search for a unit without much experience and add it to the CityDefense
                        let mut best_unit: Option<UnitId> = None;
                        let mut best_value = 100;
                        if let Some(my) = grp(g, u) {
                            for &lu in &g.groups[my].units {
                                if lu != u {
                                    let mut value = g.units[lu].experience;
                                    if g.unit_city_defense_modifier(lu) > 0 {
                                        value /= 2;
                                    }

                                    if value < best_value {
                                        best_unit = Some(lu);
                                        best_value = value;
                                    }
                                }
                            }
                        }
                        if let Some(b) = best_unit {
                            g.unit_join_group(b, None, true);
                            g.units[b].ai_groupflag = GROUPFLAG_PERMDEFENSE_NEW;
                            push_skip(g, b);
                            enough += 1;
                        } else {
                            // didn't find a unit , abort the loop
                            enough = 1000;
                        }
                    }
                }
            }
        }

        let area = g.unit_area(u);
        let area_ai = g.map.areas[area].area_ai_type[team];
        let _ignore_faster = self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_FASTMOVERS)
            && area_ai != Some(AreaAIType::AREAAI_ASSAULT);

        let land_war = !g.unit_is_barbarian(u)
            && (area_ai == Some(AreaAIType::AREAAI_OFFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_MASSING));

        let city = is_city(g, g.unit_plot(u));

        if !city {
            if self.unit_ai_bombard_city(g, u) {
                return;
            }

            let min_attack_size = 1.max(num_units(g, u) / 3);

            if self.unit_ai_any_attack(g, u, 2, 65, min_attack_size, false) {
                return;
            }

            if num_units(g, u) > 1 {
                if self.unit_ai_stack_attack_city(g, u, 1, 250, true) {
                    return;
                }
            }

            // stack attack
            if num_units(g, u) > 1 {
                if self.unit_ai_stack_attack_city(g, u, 1, 110, true) {
                    return;
                }
            }
        }
        if !g.units.contains(u) {
            return;
        }
        if self.unit_ai_groupheal(g, u, 50, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        let area = g.unit_area(u);
        let area_ai = g.map.areas[area].area_ai_type[team];
        let barbarian = g.barbarian_player();
        let mut hunt_barbs = false;
        if g.map.areas[area].cities_per_player[barbarian] > 0 {
            if area_ai != Some(AreaAIType::AREAAI_OFFENSIVE) && area_ai != Some(AreaAIType::AREAAI_DEFENSIVE) {
                hunt_barbs = true;
            }
        }
        let ready_to_attack =
            num_units(g, u) >= (if hunt_barbs { 3 } else { self.unit_ai_stack_of_doom_extra(g, u) });
        if owned_by(g, g.unit_plot(u), owner) {
            if !land_war {
                if self.unit_ai_load(
                    g,
                    u,
                    UnitAIType::UNITAI_ASSAULT_SEA,
                    MissionAIType::MISSIONAI_LOAD_ASSAULT,
                    None,
                    -1,
                    -1,
                    -1,
                    -1,
                    MOVE_SAFE_TERRITORY,
                    4,
                ) {
                    return;
                }
            }

            if !ready_to_attack {
                let target_count = self.player_ai_unit_target_mission_ais(g, owner, u, MissionAIType::MISSIONAI_GROUP, None);
                if (target_count * 5) > num_units(g, u) {
                    if self.unit_ai_move_to_staging_city(g, u) {
                        return;
                    }
                }
            }
        }
        if !g.units.contains(u) {
            return;
        }

        if g.unit_collateral_damage(u) > 0 {
            if self.unit_ai_any_attack(g, u, 1, 45, 3, false) {
                return;
            }
        }

        if self.unit_ai_any_attack(g, u, 1, 60, 0, false) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        let _at_war = g.unit_is_enemy(u, g.plot_get_team(g.unit_plot(u)), None);

        if self.unit_ai_groupheal(g, u, 30, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        if !g.unit_is_enemy(u, g.plot_get_team(g.unit_plot(u)), None) {
            if self.unit_ai_groupheal(g, u, 20, MAX_INT) {
                return;
            }
        }
        if !g.units.contains(u) {
            return;
        }

        // XXX more sophisticated logic for attacking is long overdue here
        if ready_to_attack {
            if hunt_barbs && self.unit_ai_target_barb_city(g, u) {
                return;
            } else if land_war {
                if !g.units.contains(u) {
                    return;
                }
                if self.unit_ai_target_city(g, u, 0) {
                    return;
                }
                if !g.units.contains(u) {
                    return;
                }
                if g.team_get_any_war_plan_count(team, true) > 0 {
                    let area = g.unit_area(u);
                    if let Some(target_city) = g.area_get_target_city(area, owner) {
                        let tp = g.city_plot(target_city);
                        let declare_war = g.team_get_at_war_count(team, true) == 0;
                        if self.unit_ai_solve_blockage_problem(g, u, tp, declare_war) {
                            return;
                        }
                    }
                }
            }
        }
        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_move_to_staging_city(g, u) {
            return;
        }

        if self.unit_ai_offensive_airlift(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::AI_heromove()` (CvUnitAI.cpp:21422-21474)
    pub fn unit_ai_heromove(&self, g: &mut Game, u: UnitId) {
        let unit_type = g.units[u].unit_type.i();
        if unit_type == g.d.defines.unit_govannon {
            if self.unit_ai_govannonmove(g, u) {
                return;
            }
            push_skip(g, u);
            return;
        }
        if unit_type == g.d.defines.unit_loki {
            if self.unit_ai_lokimove(g, u) {
                return;
            }
            push_skip(g, u);
            return;
        }
        if unit_type == g.d.defines.unit_rantine {
            if self.unit_ai_rantinemove(g, u) {
                return;
            }
            push_skip(g, u);
            return;
        }

        let owner = g.units[u].owner;
        if g.players[owner].conquest_mode {
            g.units[u].ai_groupflag = GROUPFLAG_CONQUEST;
            push_skip(g, u);
        } else {
            if g.player_count_group_flag_units(owner, GROUPFLAG_DEFENSE_NEW) > 4 {
                g.units[u].ai_groupflag = GROUPFLAG_DEFENSE_NEW;
                push_skip(g, u);
                return;
            }
            if self.unit_ai_guard_city(g, u, true, false, MAX_INT) {
                return;
            }
            push_skip(g, u);
        }
    }

    /// C++ `bool CvUnitAI::AI_Govannonmove()` (CvUnitAI.cpp:21476-21536)
    pub fn unit_ai_govannonmove(&self, g: &mut Game, u: UnitId) -> bool {
        let teach = g.d.defines.spell_teach_spellcasting;
        if teach != NO_SPELL {
            if let Some(spell) = spell_define(teach) {
                if crate::ai_can_cast(g, u, spell, false) {
                    g.tel_cast_reason(u, "govannonmove", 0);
                    g.unit_cast(u, spell);
                }
            }
            if !g.units.contains(u) {
                return true;
            }
            let owner = g.units[u].owner;
            let mut best_value = 0;
            let mut best_unit: Option<UnitId> = None;
            let my = grp(g, u);
            let up = g.unit_plot(u);
            for lg in g.players[owner].groups.clone() {
                if !g.groups.contains(lg) {
                    continue;
                }
                let Some(head) = g.group_get_head_unit(lg) else { continue };
                if Some(lg) == my {
                    continue;
                }
                let Some(pp) = g.unit_plot_opt(head) else { continue };
                if !self.unit_ai_plot_valid(g, u, pp) {
                    continue;
                }
                if !(owned_by(g, pp, owner) && pp != up) {
                    continue;
                }
                if g.plot_is_visible_enemy_unit_for_unit(pp, u) {
                    continue;
                }
                if self.rand(100, "GovannonScores") < 10 {
                    let mut turns = 0;
                    if path(g, u, pp, &mut turns) {
                        let value = group_num_units(g, lg) / (turns + 1);
                        if value >= best_value {
                            best_value = value;
                            best_unit = Some(head);
                        }
                    }
                }
            }
            if let Some(best) = best_unit {
                let bp = g.unit_plot(best);
                if g.unit_at_plot(u, bp) {
                    // Spell
                    return false;
                }
                let mut turns = 0;
                if path(g, u, bp, &mut turns) {
                    push_move_to_unit(g, u, best, None);
                    return true;
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_Lokimove()` (CvUnitAI.cpp:21538-21591)
    pub fn unit_ai_lokimove(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let search_range = 6;
        if is_city(g, g.unit_plot(u)) {
            let disrupt = g.d.defines.spell_disrupt;
            if disrupt != NO_SPELL {
                if let Some(spell) = spell_define(disrupt) {
                    if crate::ai_can_cast(g, u, spell, false) {
                        g.tel_cast_reason(u, "lokimove", 0);
                        g.unit_cast(u, spell);
                    }
                }
            }
        }
        if !g.units.contains(u) {
            return true;
        }
        for c in g.players[owner].cities.clone() {
            if !g.cities.contains(c) {
                continue;
            }
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(cx, cy, dx, dy) else { continue };
                    if !self.unit_ai_plot_valid(g, u, lp) {
                        continue;
                    }
                    if !(is_city(g, lp) && g.plot_get_team(lp) != Some(team)) {
                        continue;
                    }
                    if team_at_war_with_plot(g, team, lp) {
                        continue;
                    }
                    let Some(target) = plot_city(g, lp) else { continue };
                    let Some(plot_owner) = g.map.plots[lp].owner else { continue };
                    if g.city_culture(target, plot_owner) == 0 {
                        let mut turns = 0;
                        if path(g, u, lp, &mut turns) {
                            push_move_to(g, u, lp, MOVE_THROUGH_ENEMY, None, None);
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_Rantinemove()` (CvUnitAI.cpp:21593-21717)
    pub fn unit_ai_rantinemove(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        if num_units(g, u) < 4 {
            let pl = g.unit_plot(u);
            if !(is_city(g, pl) && owned_by(g, pl, owner)) {
                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    return true;
                }
            }
            if !g.units.contains(u) {
                return true;
            }
            let search_range = 5;
            let (ux, uy) = (g.units[u].x, g.units[u].y);

            for dx in -search_range..=search_range {
                for dy in -search_range..=search_range {
                    let Some(lp) = g.plot_xy(ux, uy, dx, dy) else { continue };
                    if !owned_by(g, lp, owner) {
                        continue;
                    }
                    let mut turns = 0;
                    if !path(g, u, lp, &mut turns) {
                        continue;
                    }
                    for lu in g.map.plots[lp].units.clone() {
                        if !g.units.contains(lu) {
                            continue;
                        }
                        let Some(lg) = grp(g, lu) else { continue };
                        let not_head_or_alone = g.group_get_head_unit(lg) != Some(lu) || group_num_units(g, lg) == 1;
                        if !not_head_or_alone {
                            continue;
                        }
                        if g.units[lu].ai_type == Some(UnitAIType::UNITAI_COUNTER)
                            && g.units[lu].ai_groupflag == GROUPFLAG_PATROL
                        {
                            if g.group_get_head_unit(lg) != Some(lu) || group_num_units(g, lg) == 1 {
                                if g.unit_at_plot(lu, g.unit_plot(u)) {
                                    g.unit_join_group(lu, None, true);
                                    g.units[lu].ai_groupflag = GROUPFLAG_NONE;
                                    g.unit_ai_set_unit_ai_type(lu, UnitAIType::UNITAI_ATTACK);
                                    g.unit_ai_set_origin_plot(lu, None);
                                    let my = grp(g, u);
                                    g.unit_join_group(lu, my, true);
                                    return false;
                                } else {
                                    push_move_to_unit(g, lu, u, Some(MissionAIType::MISSIONAI_GROUP));
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let convert = g.d.defines.spell_convert_city_rantine;
            if convert != NO_SPELL {
                let search_range = 8;
                if is_city(g, g.unit_plot(u)) {
                    if let Some(spell) = spell_define(convert) {
                        if crate::ai_can_cast(g, u, spell, false) {
                            g.tel_cast_reason(u, "rantinemove", 0);
                            g.unit_cast(u, spell);
                        }
                    }
                }
                if !g.units.contains(u) {
                    return true;
                }
                for c in g.players[owner].cities.clone() {
                    if !g.cities.contains(c) {
                        continue;
                    }
                    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
                    for dx in -search_range..=search_range {
                        for dy in -search_range..=search_range {
                            let Some(lp) = g.plot_xy(cx, cy, dx, dy) else { continue };
                            if !self.unit_ai_plot_valid(g, u, lp) {
                                continue;
                            }
                            if !(is_city(g, lp) && g.plot_get_team(lp) != Some(team)) {
                                continue;
                            }
                            if team_at_war_with_plot(g, team, lp) {
                                continue;
                            }
                            let Some(target) = plot_city(g, lp) else { continue };
                            if g.city_is_barbarian(target) {
                                let mut turns = 0;
                                if path(g, u, lp, &mut turns) {
                                    push_move_to(g, u, lp, MOVE_THROUGH_ENEMY, None, None);
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// C++ `bool CvUnitAI::AI_promoteMagicUnit()` (CvUnitAI.cpp:21719-21744)
    pub fn unit_ai_promote_magic_unit(&self, g: &mut Game, u: UnitId) -> bool {
        let Some(magic_talent) = opt_from_i32::<PromotionType>(g.d.defines.promotion_channeling1) else {
            return false;
        };

        if !g.units[u].has_promotion[magic_talent] {
            return false;
        }

        let best_promotion = self.py_ai_mage_promotion(g, u);
        if let Some(best) = best_promotion {
            g.unit_promote(u, Some(best), None);
            return true;
        }

        false
    }

    /// C++ `void CvUnitAI::AI_upgrademanaMove()` (CvUnitAI.cpp:21746-21765)
    pub fn unit_ai_upgrademana_move(&self, g: &mut Game, u: UnitId) {
        let result = self.py_ai_mage_upgrade_mana(g, u);

        if result == 1 {
            return;
        }
        if !g.units.contains(u) {
            return;
        }
        if result == 2 {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
        }
        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::AI_mageCast()` (CvUnitAI.cpp:21768-21797)
    /// Called every turn once in DoTurnUnitsPre().
    pub fn unit_ai_mage_cast(&self, g: &mut Game, u: UnitId) {
        let city = is_city(g, g.unit_plot(u));
        let mut best_spell: Option<SpellType> = None;
        let mut best_value = 0;
        for spell in g.d.spells.keys() {
            let info = &g.d.spells[spell];
            let mut value = info.ai_weight_war;
            if city {
                value += info.ai_weight_city;
            }
            if value > 0 {
                if crate::ai_can_cast(g, u, spell, false) {
                    if value > best_value {
                        best_value = value;
                        best_spell = Some(spell);
                    }
                }
            }
        }
        if let Some(spell) = best_spell {
            g.tel_cast_reason(u, "mage_cast", 0);
            g.unit_cast(u, spell);
        }
    }

    /// C++ `void CvUnitAI::AI_mageMove()` (CvUnitAI.cpp:21800-21803)
    pub fn unit_ai_mage_move(&self, g: &mut Game, u: UnitId) {
        push(g, u, MissionType::MISSION_FORTIFY, -1, -1, 0, None, None, None);
    }

    /// C++ `void CvUnitAI::AI_terraformerMove()` (CvUnitAI.cpp:21805-21818)
    pub fn unit_ai_terraformer_move(&self, g: &mut Game, u: UnitId) {
        let result = self.py_ai_mage_turn(g, u);
        if result != 1 {
            push_skip(g, u);
        }
    }

    /// C++ `bool CvUnitAI::isSummoner()` (CvUnitAI.cpp:21821-21834)
    /// Returns true if the Unit can Summon stuff.
    pub fn unit_ai_is_summoner(&self, g: &Game, u: UnitId) -> bool {
        for spell in g.d.spells.keys() {
            if g.d.spells[spell].create_unit_type.is_some() {
                if crate::ai_can_cast(g, u, spell, false) {
                    return true;
                }
            }
        }
        false
    }

    /// C++ `void CvUnitAI::AI_SummonCast()` (CvUnitAI.cpp:21837-21898)
    pub fn unit_ai_summon_cast(&self, g: &mut Game, u: UnitId) {
        if g.units[u].has_casted {
            return;
        }
        if !self.unit_ai_is_summoner(g, u) {
            return;
        }
        let mut best_value = 0;
        let mut best_spell: Option<SpellType> = None;
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        for spell in g.d.spells.keys() {
            let mut value = 0;
            let info = &g.d.spells[spell];
            if let Some(create_unit) = info.create_unit_type {
                if crate::ai_can_cast(g, u, spell, false) {
                    let move_range = g.d.units[create_unit].moves + g.unit_get_extra_spell_move(u);
                    let mut enemy = false;
                    'scan: for i in -move_range..=move_range {
                        for j in -move_range..=move_range {
                            if let Some(lp) = g.plot_xy(ux, uy, i, j) {
                                if g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                                    enemy = true;
                                    break 'scan;
                                }
                            }
                        }
                    }
                    if enemy {
                        let cu = &g.d.units[create_unit];
                        let mut temp_value = cu.combat;
                        for dt in g.d.damages.keys() {
                            temp_value += cu.damage_type_combat.get(dt).copied().unwrap_or(0);
                        }
                        temp_value *= 100;
                        temp_value *= info.create_unit_num;
                        value += temp_value;
                    }
                }
            }
            if value > best_value {
                best_value = value;
                best_spell = Some(spell);
            }
        }
        if let Some(spell) = best_spell {
            g.tel_cast_reason(u, "summon_cast", 0);
            g.unit_cast(u, spell);
        }
    }

    /// C++ `bool CvUnitAI::isDirectDamageCaster()` (CvUnitAI.cpp:21901-21918)
    pub fn unit_ai_is_direct_damage_caster(&self, g: &Game, u: UnitId) -> bool {
        if g.units[u].has_casted {
            return false;
        }
        for spell in g.d.spells.keys() {
            if g.d.spells[spell].damage > 0 {
                if crate::ai_can_cast(g, u, spell, false) {
                    return true;
                }
            }
        }
        false
    }

    /// C++ `void CvUnitAI::AI_DirectDamageCast(int Threshold)` (CvUnitAI.cpp:21922-22000)
    /// C++ quirk: `Threshold` is never read.
    pub fn unit_ai_direct_damage_cast(&self, g: &mut Game, u: UnitId, threshold: i32) {
        if g.units[u].has_casted {
            return;
        }
        if !self.unit_ai_is_direct_damage_caster(g, u) {
            return;
        }
        let team = g.unit_get_team(u);
        let up = g.unit_plot(u);
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let mut best_spell: Option<SpellType> = None;
        let mut best_value = 0;
        for spell in g.d.spells.keys() {
            let info = &g.d.spells[spell];
            let range = info.range;
            let mut value = 0;
            if info.damage != 0 {
                if crate::ai_can_cast(g, u, spell, false) {
                    let dmg = info.damage;
                    let dmg_limit = info.damage_limit;
                    for i in -range..=range {
                        for j in -range..=range {
                            let Some(lp) = g.plot_xy(ux, uy, i, j) else { continue };
                            if lp == up {
                                continue;
                            }
                            for &lu in &g.map.plots[lp].units {
                                if g.unit_is_immune_to_spell(lu, u, spell) {
                                    continue;
                                }
                                let lu_enemy = g.unit_is_enemy(lu, Some(team), None);
                                if lu_enemy {
                                    if g.units[lu].damage < dmg_limit {
                                        value += dmg * 10;
                                    }
                                }
                                let lu_team = g.unit_get_team(lu);
                                if lu_team == team {
                                    value -= dmg * 20;
                                }
                                if lu_team != team && !lu_enemy {
                                    value -= 1000;
                                }
                            }
                        }
                    }
                }
            }
            if value > best_value {
                best_value = value;
                best_spell = Some(spell);
            }
        }
        if let Some(spell) = best_spell {
            g.tel_cast_reason(u, "direct_damage_cast", 0);
            g.unit_cast(u, spell);
        }
    }

    /// C++ `bool CvUnitAI::isMovementCaster()` (CvUnitAI.cpp:22002-22018)
    pub fn unit_ai_is_movement_caster(&self, g: &Game, u: UnitId) -> bool {
        let ai = g.units[u].ai_type;
        if ai == Some(UnitAIType::UNITAI_MAGE) || ai == Some(UnitAIType::UNITAI_TERRAFORMER) {
            return false;
        }
        let haste = g.d.defines.spell_haste;
        if haste == NO_SPELL {
            return false;
        }
        match spell_define(haste) {
            Some(spell) => crate::ai_can_cast(g, u, spell, false),
            None => false,
        }
    }

    /// C++ `void CvUnitAI::AI_MovementCast()` (CvUnitAI.cpp:22020-22030)
    pub fn unit_ai_movement_cast(&self, g: &mut Game, u: UnitId) {
        let haste = g.d.defines.spell_haste;
        if haste != NO_SPELL {
            if let Some(spell) = spell_define(haste) {
                if crate::ai_can_cast(g, u, spell, false) {
                    g.tel_cast_reason(u, "movement_cast", 0);
                    g.unit_cast(u, spell);
                }
            }
        }
    }

    /// C++ `bool CvUnitAI::isBuffer()` (CvUnitAI.cpp:22032-22048)
    pub fn unit_ai_is_buffer(&self, g: &Game, u: UnitId) -> bool {
        for spell in g.d.spells.keys() {
            if let Some(promotion) = g.d.spells[spell].add_promotion_type1 {
                if g.d.promotions[promotion].ai_weight > 0 {
                    if crate::ai_can_cast(g, u, spell, false) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// C++ `void CvUnitAI::AI_BuffCast()` (CvUnitAI.cpp:22051-22092)
    /// This is run often, so lets keep things simple.
    pub fn unit_ai_buff_cast(&self, g: &mut Game, u: UnitId) {
        if g.units[u].has_casted {
            return;
        }
        if !self.unit_ai_is_buffer(g, u) {
            return;
        }
        let mut best_spell: Option<SpellType> = None;
        let mut best_value = 0;
        for spell in g.d.spells.keys() {
            let mut value = 0;
            let info = &g.d.spells[spell];
            if let Some(promotion1) = info.add_promotion_type1 {
                if crate::ai_can_cast(g, u, spell, false) {
                    value += g.d.promotions[promotion1].ai_weight;
                    if let Some(promotion2) = info.add_promotion_type2 {
                        value += g.d.promotions[promotion2].ai_weight;
                    }
                    if let Some(promotion3) = info.add_promotion_type3 {
                        value += g.d.promotions[promotion3].ai_weight;
                    }
                }
            }
            if value > best_value {
                best_value = value;
                best_spell = Some(spell);
            }
        }
        if let Some(spell) = best_spell {
            g.tel_cast_reason(u, "buff_cast", 0);
            g.unit_cast(u, spell);
        }
    }

    /// C++ `bool CvUnitAI::isSuicideSummon()` (CvUnitAI.cpp:22094-22097)
    pub fn unit_ai_is_suicide_summon(&self, g: &Game, u: UnitId) -> bool {
        g.units[u].ai_suicide_summon
    }

    /// C++ `void CvUnitAI::setSuicideSummon(bool newvalue)` (CvUnitAI.cpp:22099-22102)
    pub fn unit_ai_set_suicide_summon(&self, g: &mut Game, u: UnitId, newvalue: bool) {
        g.units[u].ai_suicide_summon = newvalue;
    }

    /// C++ `bool CvUnitAI::isPermanentSummon()` (CvUnitAI.cpp:22104-22107)
    pub fn unit_ai_is_permanent_summon(&self, g: &Game, u: UnitId) -> bool {
        let v = self.s().units.get(&u).map_or(false, |x| x.permanent_summon);
        v
    }

    /// C++ `void CvUnitAI::setPermanentSummon(bool newvalue)` (CvUnitAI.cpp:22109-22112)
    pub fn unit_ai_set_permanent_summon(&self, g: &mut Game, u: UnitId, newvalue: bool) {
        self.s().unit(u).permanent_summon = newvalue;
    }

    /// C++ `void CvUnitAI::changeAllowedPermDefense(bool bNewValue)` (CvUnitAI.cpp:22114-22117)
    pub fn unit_ai_change_allowed_perm_defense(&self, g: &mut Game, u: UnitId, new_value: bool) {
        g.units[u].ai_allowed_perm_defense = new_value;
    }

    /// C++ `void CvUnitAI::changeAllowedPatrol(bool bNewValue)` (CvUnitAI.cpp:22119-22122)
    pub fn unit_ai_change_allowed_patrol(&self, g: &mut Game, u: UnitId, new_value: bool) {
        g.units[u].ai_allowed_patrol = new_value;
    }

    /// C++ `void CvUnitAI::changeAllowedExplore(bool bNewValue)` (CvUnitAI.cpp:22124-22127)
    pub fn unit_ai_change_allowed_explore(&self, g: &mut Game, u: UnitId, new_value: bool) {
        g.units[u].ai_allowed_explore = new_value;
    }

    /// C++ `void CvUnitAI::AI_SettleNavalNewMove()` (CvUnitAI.cpp:22130-22166)
    /// BETTER AI (Naval AI) Sephi.
    pub fn unit_ai_settle_naval_new_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        for lu in g.players[owner].units.clone() {
            if !g.units.contains(lu) || lu == u {
                continue;
            }
            let flag = g.units[lu].ai_groupflag;
            if flag == GROUPFLAG_NAVAL_SETTLE || flag == GROUPFLAG_NAVAL_SETTLE_PICKUP {
                let Some(lp) = g.unit_plot_opt(lu) else { continue };
                if g.unit_at_plot(u, lp) {
                    let lg = grp(g, lu);
                    g.unit_join_group(u, lg, true);
                    g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
                    return;
                } else {
                    let mut turns = 0;
                    if path(g, u, lp, &mut turns) {
                        push_move_to_unit(g, u, lu, Some(MissionAIType::MISSIONAI_GROUP));
                        return;
                    }

                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_EXPLORE_SEA);
                    return;
                }
            }
        }
        g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
    }

    /// C++ `void CvUnitAI::AI_SettleNavalPickupMove()` (CvUnitAI.cpp:22168-22343)
    pub fn unit_ai_settle_naval_pickup_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let pl = g.unit_plot(u);
        let expansion_plot = self.player_ai_get_expansion_plot(g, owner);
        let Some(expansion_plot) = expansion_plot.filter(|_| is_city(g, pl)) else {
            g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
            push_skip(g, u);
            return;
        };

        // Count Settlers and Workers
        let (settler_count, _worker_count, _total_cargo, total_cargo_space) = naval_cargo_counts(g, u);

        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let my_area = g.unit_area(u);

        // Need to Pick up a Settler
        if settler_count == 0 {
            if total_cargo_space < 1 {
                if let Some(my) = grp(g, u) {
                    g.group_unload_all(my);
                }
            }
            if area_num_ai_units(g, g.map.plots[pl].area, owner, UnitAIType::UNITAI_SETTLE) == 0 {
                g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
                push_skip(g, u);
                return;
            }

            for lu in g.players[owner].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                if unit_area_opt(g, lu) != Some(my_area) || g.units[lu].ai_type != Some(UnitAIType::UNITAI_SETTLE) {
                    continue;
                }
                if Some(my_area) == g.map.plots[expansion_plot].area {
                    continue;
                }
                let mut turns = 0;
                if !g.unit_generate_path(lu, pl, 0, true, Some(&mut turns)) {
                    continue;
                }
                if g.unit_at_plot(lu, pl) {
                    let settler_members = grp(g, lu).map_or(Vec::new(), |lg| g.groups[lg].units.clone());
                    for lu1 in settler_members {
                        if !g.units.contains(lu1) {
                            continue;
                        }
                        g.unit_join_group(lu1, None, true);

                        // Load Units into Transporters
                        let mut unit_loaded = false;

                        let transports = grp(g, u).map_or(Vec::new(), |my| g.groups[my].units.clone());
                        for trans in transports {
                            if !g.units.contains(trans) {
                                continue;
                            }
                            // Settler(pLoopUnit) has Priority
                            if !g.unit_is_full(trans) {
                                if !g.unit_is_cargo(lu) {
                                    if let Some(lg) = grp(g, lu) {
                                        g.group_set_transport_unit(lg, Some(trans));
                                    }
                                }
                            }
                            if !g.unit_is_full(trans) {
                                if let Some(l1g) = grp(g, lu1) {
                                    g.group_set_transport_unit(l1g, Some(trans));
                                }
                                unit_loaded = true;
                            }

                            if unit_loaded {
                                break;
                            }
                        }
                    }
                    push_skip(g, u);
                    return;
                } else {
                    push(g, lu, MissionType::MISSION_MOVE_TO, ux, uy, 0, None, Some(pl), None);
                    return;
                }
            }
        }

        // Need to Pick up Guarding Units
        if total_cargo_space > 0 {
            let parea = g.map.plots[pl].area;
            if area_num_ai_units(g, parea, owner, UnitAIType::UNITAI_BARBSMASHER)
                + area_num_ai_units(g, parea, owner, UnitAIType::UNITAI_ATTACK_CITY)
                == 0
            {
                g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
                push_skip(g, u);
                return;
            }
            let _units_needed = total_cargo_space;

            for lu in g.players[owner].units.clone() {
                if !g.units.contains(lu) {
                    continue;
                }
                if unit_area_opt(g, lu) != Some(my_area) {
                    continue;
                }
                let lai = g.units[lu].ai_type;
                if !(lai == Some(UnitAIType::UNITAI_BARBSMASHER) || lai == Some(UnitAIType::UNITAI_ATTACK_CITY)) {
                    continue;
                }
                // for now only pick up newly build units
                if num_units(g, lu) != 1 {
                    continue;
                }
                let mut turns = 0;
                if !g.unit_generate_path(lu, pl, 0, true, Some(&mut turns)) {
                    continue;
                }
                if g.unit_at_plot(lu, pl) {
                    g.unit_join_group(lu, None, true);
                    let mut unit_loaded = false;

                    let transports = grp(g, u).map_or(Vec::new(), |my| g.groups[my].units.clone());
                    for trans in transports {
                        if !g.units.contains(trans) {
                            continue;
                        }
                        if !g.unit_is_full(trans) {
                            if let Some(lg) = grp(g, lu) {
                                g.group_set_transport_unit(lg, Some(trans));
                            }
                            unit_loaded = true;
                        }

                        if unit_loaded {
                            break;
                        }
                    }
                } else {
                    push(g, lu, MissionType::MISSION_MOVE_TO, ux, uy, 0, None, Some(pl), None);
                    return;
                }
            }
        }
        // can start the Settle Mission
        if settler_count > 0 && total_cargo_space == 0 {
            g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE;
            push_skip(g, u);
        }
    }

    /// C++ `void CvUnitAI::AI_InquisitionMove()` (CvUnitAI.cpp:22345-22413)
    pub fn unit_ai_inquisition_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let my_area = g.unit_area(u);
        let state_belief = g.player_get_state_religion(owner);
        let mut best_plot: Option<PlotIdx> = None;
        let mut best_value = 100;

        for i in 0..g.map.plots.len() {
            let lp = PlotIdx(i as u32);
            // (the cheap tests first: they have no side effects)
            if !is_city(g, lp) || g.map.plots[lp].area != Some(my_area) {
                continue;
            }
            if !self.unit_ai_plot_valid(g, u, lp) {
                continue;
            }
            if g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                continue;
            }
            let Some(city) = plot_city(g, lp) else { continue };
            let mut valid_target = false;

            for religion in g.d.religions.keys() {
                if state_belief != Some(religion)
                    && g.cities[city].has_religion[religion]
                    && !g.city_is_holy_city(city, religion)
                {
                    valid_target = true;
                }
            }

            let city_owner = g.cities[city].owner;
            if state_belief.is_none() {
                if owner != city_owner {
                    valid_target = false;
                }
            }

            if state_belief != g.player_get_state_religion(city_owner) {
                valid_target = false;
            }

            let mut value = 0;
            if valid_target && path(g, u, lp, &mut value) {
                if value < best_value {
                    best_plot = Some(lp);
                    best_value = value;
                }
            }
        }

        if let Some(bp) = best_plot {
            push_move_to(g, u, bp, 0, None, None);
            return;
        }

        push_skip(g, u);
    }

    /// C++ `void CvUnitAI::AI_SettleNavalMove()` (CvUnitAI.cpp:22417-22577)
    pub fn unit_ai_settle_naval_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;

        // Do we have a target?
        let Some(target_plot) = self.player_ai_get_expansion_plot(g, owner) else {
            // wait until we get a new Target
            push_skip(g, u);
            return;
        };

        // Count Settlers and Workers
        let (settler_count, worker_count, total_cargo, total_cargo_space) = naval_cargo_counts(g, u);

        let guarding_units = total_cargo - total_cargo_space - settler_count - worker_count;

        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let pl = g.unit_plot(u);

        // Transport needs Settler?
        if settler_count == 0 {
            if is_city(g, pl) {
                // if units are around to pick up
                if area_num_ai_units(g, g.map.plots[pl].area, owner, UnitAIType::UNITAI_SETTLE) > 0 {
                    g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE_PICKUP;
                    push_skip(g, u);
                    return;
                }
            }

            let mut best_city: Option<CityId> = None;
            let mut best_value = 0;

            for c in g.players[owner].cities.clone() {
                if !g.city_is_coastal(c, -1) {
                    continue;
                }
                let mut turns = 0;
                if path(g, u, g.city_plot(c), &mut turns) {
                    let mut value = area_num_ai_units(g, Some(g.city_area(c)), owner, UnitAIType::UNITAI_SETTLE);
                    value *= 1000;
                    let divisor = (g.cities[c].x - ux).abs() + (g.cities[c].y - uy).abs();
                    value /= 20 + divisor;

                    if value > best_value {
                        best_value = value;
                        best_city = Some(c);
                    }
                }
            }

            if let Some(bc) = best_city {
                let bp = g.city_plot(bc);
                push_move_to(g, u, bp, 0, None, Some(bp));
                return;
            }
            // no Reinforcements Available
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }

            push_skip(g, u);
            return;
        }

        // Transport is not Full?
        if total_cargo_space > 0 && 5 > guarding_units {
            if is_city(g, pl) {
                // if units are around to pick up
                let parea = g.map.plots[pl].area;
                let uarea = Some(g.unit_area(u));
                if area_num_ai_units(g, parea, owner, UnitAIType::UNITAI_ATTACK_CITY)
                    + area_num_ai_units(g, uarea, owner, UnitAIType::UNITAI_BARBSMASHER)
                    > 0
                {
                    g.units[u].ai_groupflag = GROUPFLAG_NAVAL_SETTLE_PICKUP;
                    push_skip(g, u);
                    return;
                }
            }
            // move to potential units to pick up

            let mut best_city: Option<CityId> = None;
            let mut best_value = 0;

            for c in g.players[owner].cities.clone() {
                if !g.city_is_coastal(c, -1) {
                    continue;
                }
                let mut turns = 0;
                if path(g, u, g.city_plot(c), &mut turns) {
                    let carea = Some(g.city_area(c));
                    let mut value = area_num_ai_units(g, carea, owner, UnitAIType::UNITAI_ATTACK_CITY);
                    value += area_num_ai_units(g, carea, owner, UnitAIType::UNITAI_BARBSMASHER);
                    value *= 1000;
                    let divisor = (g.cities[c].x - ux).abs() + (g.cities[c].y - uy).abs();
                    value /= 20 + divisor;

                    if value > best_value {
                        best_value = value;
                        best_city = Some(c);
                    }
                }
            }

            if let Some(bc) = best_city {
                let bp = g.city_plot(bc);
                push_move_to(g, u, bp, 0, None, Some(bp));
                return;
            }
            // no Reinforcements Available
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }

            push_skip(g, u);
            return;
        }

        // Move Closer to TargetPlot
        if g.unit_at_plot(u, target_plot) {
            if let Some(my) = grp(g, u) {
                g.group_unload_all(my);
            }
            push_skip(g, u);
        } else {
            push_move_to(g, u, target_plot, 0, Some(MissionAIType::MISSIONAI_FOUND), Some(target_plot));
        }
    }

    /// C++ `bool CvUnitAI::AI_buildPirateCove()` (CvUnitAI.cpp:22587-22704)
    /// Skyre Mod, BETTER AI (Lanun Pirate Coves) merged Sephi.
    pub fn unit_ai_build_pirate_cove(&self, g: &mut Game, u: UnitId) -> bool {
        // FIX: `PIRATE_COVE_SPELL` is not a define of FFH2 0.41, so the C++ reads 0
        // (SPELL_ADD_TO_CITY, never castable on a water plot): the unit walked to the cove site
        // and skipped there forever. The pirate cove spell is meant.
        let Some(cove_spell) = g.d.ty::<SpellType>("SPELL_PIRATE_COVE") else {
            return false;
        };

        let owner = g.units[u].owner;
        let my_area = g.unit_area(u);
        let up = g.unit_plot(u);

        let mut good_plots: Vec<PlotIdx> = Vec::new();
        let mut best_plot_value = 0;

        for i in 0..g.map.plots.len() {
            let lp = PlotIdx(i as u32);

            // (the cheap ownership test first: no side effects)
            if !owned_by(g, lp, owner) || g.map.plots[lp].working_city.filter(|&c| g.cities.contains(c)).is_none() {
                continue;
            }

            if !self.unit_ai_plot_valid(g, u, lp) {
                continue;
            }

            let lp_area = g.map.plots[lp].area;
            if lp_area != Some(my_area) && !lp_area.map_or(false, |a| g.plot_is_adjacent_to_area(up, a)) {
                continue;
            }

            if g.plot_is_visible_enemy_unit_for_unit(lp, u) {
                continue;
            }

            if !g.plot_is_pirate_cove_valid(lp, owner) {
                continue;
            }

            if !g.unit_at_plot(u, lp) && !g.unit_can_move_into(u, lp, false, false, false) {
                continue;
            }

            // C++ quirk: `iDistance` starts at 0 and only shrinks under `std::min`, so every
            // plot is worth 70 and the nearest by path wins.
            let mut distance = 0;
            let (lx, ly) = g.map.xy(lp);
            for ix in -5..=5 {
                for iy in -5..=5 {
                    if let Some(search_plot) = g.plot_xy(lx, ly, ix, iy) {
                        if g.plot_is_pirate_cove(search_plot) {
                            distance = distance.min(ix.abs().max(iy.abs()) * 10 + ix.abs().min(iy.abs()));
                        }
                    }
                }
            }

            let plot_value = if distance > 0 { 100 - distance } else { 70 };

            if plot_value > best_plot_value {
                good_plots.clear();
                good_plots.push(lp);

                best_plot_value = plot_value;
            } else if plot_value == best_plot_value {
                good_plots.push(lp);
            }
        }

        let mut best_plot: Option<PlotIdx> = None;

        if !good_plots.is_empty() {
            let mut shortest_distance = MAX_INT;

            for &gp in &good_plots {
                let mut turns = 0;
                path(g, u, gp, &mut turns);

                if turns < shortest_distance {
                    best_plot = Some(gp);
                    shortest_distance = turns;
                }
            }
        }

        if let Some(bp) = best_plot {
            if g.unit_plot(u) != bp {
                push_move_to(g, u, bp, 0, Some(MissionAIType::MISSIONAI_BUILD), Some(bp));
            } else {
                if crate::ai_can_cast(g, u, cove_spell, false) {
                    g.tel_cast_reason(u, "build_pirate_cove", 0);
                    g.unit_cast(u, cove_spell);
                }

                push_skip(g, u);
            }

            return true;
        }

        false
    }
}
