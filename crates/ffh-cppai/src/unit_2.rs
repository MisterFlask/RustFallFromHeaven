//! Port of CvUnitAI.cpp:2956-5692 (FFH2 0.41 C++ AI).
//!
//! The per-UNITAI move functions: each tries a list of AI behaviours in order and stops at the
//! first that acts (pushes a mission), else skips the turn.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// C++ `getGroup()` of a live unit.
fn group_of(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnit::getGroup: unit without a group")
}

/// C++ `getGroup()->getNumUnits()`.
fn group_num_units(g: &Game, u: UnitId) -> i32 {
    g.groups[group_of(g, u)].units.len() as i32
}

/// C++ `getGroup()->pushMission(eMission)` with the C++ defaults (`iData1 = -1, iData2 = -1,
/// iFlags = 0, bAppend = false, bManual = false, NO_MISSIONAI, NULL, NULL`).
fn push_mission(g: &mut Game, u: UnitId, mission: MissionType) {
    if !g.units.contains(u) {
        return;
    }
    let grp = group_of(g, u);
    g.group_push_mission(grp, mission, -1, -1, 0, false, false, None, None, None);
}

/// C++ `area()->getAreaAIType(getTeam())`.
fn area_ai_type(g: &Game, u: UnitId) -> Option<AreaAIType> {
    let a = g.unit_area(u);
    let t = g.unit_get_team(u);
    g.map.areas[a].area_ai_type[t]
}

/// C++ `plot()->getOwnerINLINE() == getOwnerINLINE()`.
fn plot_owned_by_us(g: &Game, u: UnitId) -> bool {
    g.map.plots[g.unit_plot(u)].owner == Some(g.units[u].owner)
}

/// C++ `plot()->getTeam()` (NO_TEAM = None).
fn plot_team(g: &Game, u: UnitId) -> Option<TeamId> {
    g.plot_get_team(g.unit_plot(u))
}

/// C++ `plot()->isCity()`.
fn plot_is_city(g: &Game, u: UnitId) -> bool {
    g.plot_is_city(g.unit_plot(u), false, None)
}

/// The tail shared word for word by the great person moves (prophet, artist, scientist,
/// engineer; e.g. CvUnitAI.cpp:4335-4392): golden age, discover or join, discover in danger or
/// when old, then retreat, safety, skip.
fn great_person_tail(cai: &Cai, g: &mut Game, u: UnitId) {
    let p = g.units[u].owner;
    let golden_age_value = cai.player_ai_calculate_golden_age_value(g, p) / g.player_units_required_for_golden_age(p);
    let discover_value = 1.max(g.unit_get_discover_research(u, None));

    if (golden_age_value * 100) / discover_value > 60 {
        if cai.unit_ai_golden_age(g, u) {
            return;
        }

        if discover_value > golden_age_value {
            if cai.unit_ai_discover(g, u, false, false) {
                return;
            }
            let class = g.unit_get_unit_class_type(u);
            if g.players[p].unit_class_count[class] > 1 {
                if cai.unit_ai_join(g, u, MAX_INT) {
                    return;
                }
            }
        }
    } else {
        if cai.unit_ai_discover(g, u, false, false) {
            return;
        }

        if cai.unit_ai_join(g, u, MAX_INT) {
            return;
        }
    }

    if cai.player_ai_get_plot_danger(g, p, g.unit_plot(u), 2, true) > 0 || g.units[u].game_turn_created < g.game_turn() - 25 {
        if cai.unit_ai_discover(g, u, false, false) {
            return;
        }
    }

    if cai.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
        return;
    }

    if cai.unit_ai_safety(g, u) {
        return;
    }

    push_mission(g, u, MissionType::MISSION_SKIP);
}

impl Cai {
    /// C++ `void CvUnitAI::AI_paratrooperMove()` (CvUnitAI.cpp:2956-3079)
    pub fn unit_ai_paratrooper_move(&self, g: &mut Game, u: UnitId) {
        {
            let hostile = g.plot_is_owned(g.unit_plot(u)) && g.unit_is_potential_enemy(u, plot_team(g, u), None);
            if !hostile {
                if self.unit_ai_guard_city(g, u, true, false, MAX_INT) {
                    return;
                }

                if plot_team(g, u) == Some(g.unit_get_team(u)) {
                    if plot_is_city(g, u) {
                        if self.unit_ai_heal(g, u, 30, 1) {
                            return;
                        }
                    }

                    let area_ai = area_ai_type(g, u);
                    let land_war = area_ai == Some(AreaAIType::AREAAI_OFFENSIVE)
                        || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE)
                        || area_ai == Some(AreaAIType::AREAAI_MASSING);
                    if !land_war {
                        if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, None, -1, -1, -1, 0, MOVE_SAFE_TERRITORY, 4) {
                            return;
                        }
                    }
                }

                if self.unit_ai_guard_city(g, u, false, true, 1) {
                    return;
                }
            }

            if self.unit_ai_city_attack(g, u, 1, 45, false) {
                return;
            }

            if self.unit_ai_any_attack(g, u, 1, 55, 0, false) {
                return;
            }

            if !hostile {
                let drop_range = g.unit_get_drop_range(u);
                if self.unit_ai_paradrop(g, u, drop_range) {
                    return;
                }

                if self.unit_ai_offensive_airlift(g, u) {
                    return;
                }

                if self.unit_ai_move_to_staging_city(g, u) {
                    return;
                }

                if self.unit_ai_guard_fort(g, u, true) {
                    return;
                }

                if self.unit_ai_guard_city_airlift(g, u) {
                    return;
                }
            }

            if g.unit_collateral_damage(u) > 0 {
                if self.unit_ai_any_attack(g, u, 1, 45, 3, false) {
                    return;
                }
            }

            if self.unit_ai_pillage_range(g, u, 1, 15) {
                return;
            }

            if hostile {
                if self.unit_ai_choke(g, u, 1) {
                    return;
                }
            }

            if self.unit_ai_heal(g, u, 0, MAX_INT) {
                return;
            }

            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }

            if self.unit_ai_protect(g, u, 35) {
                return;
            }

            if self.unit_ai_safety(g, u) {
                return;
            }
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_attackCityMove()` (CvUnitAI.cpp:3082-3342)
    pub fn unit_ai_attack_city_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        let mut ignore_faster = false;
        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FASTMOVERS) {
            if area_ai_type(g, u) != Some(AreaAIType::AREAAI_ASSAULT) {
                ignore_faster = true;
            }
        }

        // force heal if we in our own city and damaged
        // can we remove this or call AI_heal here?
        if group_num_units(g, u) == 1 && g.units[u].damage > 0 && plot_owned_by_us(g, u) && plot_is_city(g, u) {
            push_mission(g, u, MissionType::MISSION_HEAL);
            return;
        }

        let area_ai = area_ai_type(g, u);
        let land_war = !g.unit_is_barbarian(u)
            && (area_ai == Some(AreaAIType::AREAAI_OFFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_MASSING));

        if plot_is_city(g, u) {
            let city = g.map.plots[g.unit_plot(u)].plot_city.expect("isCity without a city");
            if g.game_turn() - g.cities[city].game_turn_acquired <= 1 {
                let old_group = group_of(g, u);

                self.group_ai_seperate_non_ai(g, old_group, UnitAIType::UNITAI_ATTACK_CITY);

                if Some(old_group) != g.units[u].group {
                    return;
                }
            }

            if plot_owned_by_us(g, u) {
                if area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_ASSIST) {
                    if self.unit_ai_offensive_airlift(g, u) {
                        return;
                    }
                }
            }
        }

        if self.unit_ai_guard_city(g, u, false, false, 1) {
            return;
        }

        if self.unit_ai_group_merge_range(g, u, UnitAIType::UNITAI_ATTACK_CITY, 0, true, true, ignore_faster) {
            return;
        }

        let city = plot_is_city(g, u);

        if !city {
            if self.unit_ai_bombard_city(g, u) {
                return;
            }

            //stack attack
            if group_num_units(g, u) > 1 {
                if self.unit_ai_stack_attack_city(g, u, 1, 250, true) {
                    return;
                }
            }

            //stack attack
            if group_num_units(g, u) > 1 {
                if self.unit_ai_stack_attack_city(g, u, 1, 110, true) {
                    return;
                }
            }
        }

        if self.unit_ai_group_merge_range(g, u, UnitAIType::UNITAI_ATTACK_CITY, 2, true, true, ignore_faster) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        let mut hunt_barbs = false;
        let a = g.unit_area(u);
        if g.map.areas[a].cities_per_player[g.barbarian_player()] > 0 {
            let aai = area_ai_type(g, u);
            if aai != Some(AreaAIType::AREAAI_OFFENSIVE) && aai != Some(AreaAIType::AREAAI_DEFENSIVE) {
                hunt_barbs = true;
            }
        }
        let needed = if hunt_barbs { 3 } else { self.unit_ai_stack_of_doom_extra(g, u) };
        let ready_to_attack = group_num_units(g, u) >= needed;
        if plot_owned_by_us(g, u) {
            if !land_war {
                if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, None, -1, -1, -1, -1, MOVE_SAFE_TERRITORY, 4) {
                    return;
                }
            }

            if !ready_to_attack {
                let target_count = self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_GROUP, None);
                if target_count * 5 > group_num_units(g, u) {
                    if self.unit_ai_move_to_staging_city(g, u) {
                        return;
                    }
                }
            }
        }

        if g.unit_collateral_damage(u) > 0 {
            if self.unit_ai_any_attack(g, u, 1, 45, 3, false) {
                return;
            }
        }

        if self.unit_ai_any_attack(g, u, 1, 60, 0, false) {
            return;
        }

        let at_war = g.unit_is_enemy(u, plot_team(g, u), None);

        if at_war && group_num_units(g, u) <= 2 {
            if self.unit_ai_pillage_range(g, u, 3, 11) {
                return;
            }

            if self.unit_ai_pillage_range(g, u, 1, 0) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 50, 3) {
            return;
        }

        if !g.unit_is_enemy(u, plot_team(g, u), None) {
            if self.unit_ai_heal(g, u, 0, MAX_INT) {
                return;
            }

            if group_num_units(g, u) == 1 && Some(g.unit_get_team(u)) != plot_team(g, u) {
                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    return;
                }
            }
        }

        if !ready_to_attack && !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        //XXX more sophisticated logic for attacking is long overdue here
        if ready_to_attack {
            if hunt_barbs && self.unit_ai_target_barb_city(g, u) {
                return;
            } else if land_war {
                if self.unit_ai_target_city(g, u, 0) {
                    return;
                }
                let t = g.unit_get_team(u);
                if g.team_get_any_war_plan_count(t, true) > 0 {
                    let a = g.unit_area(u);
                    if let Some(target_city) = g.area_get_target_city(a, p) {
                        let dest = g.city_plot(target_city);
                        let declare_war = g.team_get_at_war_count(t, true) == 0;
                        if self.unit_ai_solve_blockage_problem(g, u, dest, declare_war) {
                            return;
                        }
                    }
                }
            }
        } else {
            if g.unit_bombard_rate(u) > 0 && g.unit_no_defensive_bonus(u) {
                if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, -1, -1, -1, ignore_faster, true, true, /*iMaxPath*/ 10, /*bAllowRegrouping*/ true) {
                    return;
                }
            } else {
                let max_group = self.unit_ai_stack_of_doom_extra(g, u) * 2;
                if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, max_group, -1, -1, ignore_faster, true, true, /*iMaxPath*/ 10, /*bAllowRegrouping*/ false) {
                    return;
                }
            }
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

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_attackCityLemmingMove()` (CvUnitAI.cpp:3345-3378)
    pub fn unit_ai_attack_city_lemming_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_city_attack(g, u, 1, 80, false) {
            return;
        }

        if self.unit_ai_bombard_city(g, u) {
            return;
        }

        if self.unit_ai_city_attack(g, u, 1, 40, false) {
            return;
        }

        if self.unit_ai_target_city(g, u, MOVE_THROUGH_ENEMY) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 70, 0, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 0, 0, false) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_collateralMove()` (CvUnitAI.cpp:3381-3480)
    pub fn unit_ai_collateral_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_leave_attack(g, u, 1, 20, 100) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        if self.unit_ai_city_attack(g, u, 1, 35, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 45, 3, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 55, 2, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 35, 3, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 30, 4, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 20, 5, false) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        if self.unit_ai_any_attack(g, u, 2, 55, 3, false) {
            return;
        }

        if self.unit_ai_city_attack(g, u, 2, 50, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 2, 60, 0, false) {
            return;
        }

        if self.unit_ai_protect(g, u, 50) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 3) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_pillageMove()` (CvUnitAI.cpp:3483-3610)
    pub fn unit_ai_pillage_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        //join any city attacks in progress
        if !plot_owned_by_us(g, u) {
            if self.unit_ai_group_merge_range(g, u, UnitAIType::UNITAI_ATTACK_CITY, 1, true, true, false) {
                return;
            }
        }

        if self.unit_ai_city_attack(g, u, 1, 55, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 65, 0, false) {
            return;
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        if self.unit_ai_pillage_range(g, u, 3, 11) {
            return;
        }

        if self.unit_ai_choke(g, u, 1) {
            return;
        }

        if self.unit_ai_pillage_range(g, u, 1, 0) {
            return;
        }

        if plot_owned_by_us(g, u) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, Some(UnitAIType::UNITAI_ATTACK), -1, -1, -1, -1, MOVE_SAFE_TERRITORY, 4) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 50, 3) {
            return;
        }

        if !g.unit_is_enemy(u, plot_team(g, u), None) {
            if self.unit_ai_heal(g, u, 0, MAX_INT) {
                return;
            }
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_PILLAGE, /*iMaxGroup*/ 1, /*iMaxOwnUnitAI*/ 1, -1, /*bIgnoreFaster*/ true, false, false, /*iMaxPath*/ 3, false) {
            return;
        }

        if area_ai_type(g, u) == Some(AreaAIType::AREAAI_OFFENSIVE) || g.unit_is_enemy(u, plot_team(g, u), None) {
            if self.unit_ai_pillage(g, u, 20) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 3) {
            return;
        }

        if self.unit_ai_offensive_airlift(g, u) {
            return;
        }

        if self.unit_ai_travel_to_upgrade_city(g, u) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_reserveMove()` (CvUnitAI.cpp:3612-3751)
    pub fn unit_ai_reserve_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;
        let danger = self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 3, true) > 0;

        if danger && self.unit_ai_leave_attack(g, u, 2, 55, 130) {
            return;
        }

        if plot_owned_by_us(g, u) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_SETTLE), -1, -1, 1, -1, MOVE_SAFE_TERRITORY, MAX_INT) {
                return;
            }
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_WORKER), -1, -1, 1, -1, MOVE_SAFE_TERRITORY, MAX_INT) {
                return;
            }
        }

        if !danger {
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_SETTLE, 2, -1, -1, false, false, false, 3, true) {
                return;
            }
        }

        if self.unit_ai_guard_city(g, u, true, false, MAX_INT) {
            return;
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_fort(g, u, false) {
                return;
            }
        }

        if self.unit_ai_guard_city_airlift(g, u) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if self.unit_ai_guard_city_site(g, u) {
            return;
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_fort(g, u, true) {
                return;
            }

            if self.unit_ai_guard_bonus(g, u, 15) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        if danger {
            if self.unit_ai_city_attack(g, u, 1, 55, false) {
                return;
            }

            if self.unit_ai_any_attack(g, u, 1, 60, 0, false) {
                return;
            }
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        if danger {
            if self.unit_ai_city_attack(g, u, 3, 45, false) {
                return;
            }

            if self.unit_ai_any_attack(g, u, 3, 50, 0, false) {
                return;
            }
        }

        if self.unit_ai_protect(g, u, 45) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 3) {
            return;
        }

        if self.unit_ai_defend(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_counterMove()` (CvUnitAI.cpp:3754-3869)
    pub fn unit_ai_counter_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if g.units[u].same_tile_heal > 0 {
            if !g.unit_can_attack(u) {
                if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_ATTACK_CITY, -1, -1, true) {
                    return;
                }
            }
        }

        let area_ai = area_ai_type(g, u);
        if plot_is_city(g, u) {
            if plot_owned_by_us(g, u) {
                if area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_ASSIST) {
                    if self.unit_ai_offensive_airlift(g, u) {
                        return;
                    }
                }
            }
        }

        let danger = self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 3, true) > 0;

        if plot_owned_by_us(g, u) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, Some(UnitAIType::UNITAI_ATTACK_CITY), -1, -1, -1, -1, MOVE_SAFE_TERRITORY, 4) {
                return;
            }

            if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, Some(UnitAIType::UNITAI_ATTACK), -1, -1, -1, -1, MOVE_SAFE_TERRITORY, 4) {
                return;
            }
        }

        if !g.unit_no_defensive_bonus(u) {
            if self.unit_ai_guard_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        if danger {
            if self.unit_ai_city_attack(g, u, 1, 30, false) {
                return;
            }

            if self.unit_ai_any_attack(g, u, 1, 40, 0, false) {
                return;
            }
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, /*iMaxGroup*/ -1, 2, -1, false, /*bIgnoreOwnUnitType*/ true, /*bStackOfDoom*/ true, /*iMaxPath*/ 6, false) {
            return;
        }

        let fast_movers = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_FASTMOVERS);

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK, /*iMaxGroup*/ 2, -1, -1, fast_movers, /*bIgnoreOwnUnitType*/ true, /*bStackOfDoom*/ true, /*iMaxPath*/ 5, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, /*iMaxGroup*/ -1, 2, -1, false, /*bIgnoreOwnUnitType*/ true, /*bStackOfDoom*/ true, /*iMaxPath*/ 6, false) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 3) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
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

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_cityDefenseMove()` (CvUnitAI.cpp:3872-4023)
    pub fn unit_ai_city_defense_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;
        let danger = self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 3, true) > 0;

        // BETTER AI (first city) Sephi
        if g.players[p].num_cities() == 0 && g.game_turn() < 10 {
            push_mission(g, u, MissionType::MISSION_SKIP);
            return;
        }

        if danger {
            if self.unit_ai_leave_attack(g, u, 1, 70, 175) {
                return;
            }

            if self.unit_ai_choke_defend(g, u) {
                return;
            }
        }

        if self.unit_ai_guard_city_best_defender(g, u) {
            return;
        }

        if !danger {
            if plot_owned_by_us(g, u) {
                if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_SETTLE), -1, -1, 1, -1, MOVE_SAFE_TERRITORY, 1) {
                    return;
                }
            }
        }

        if self.unit_ai_guard_city_min_defender(g, u, true) {
            return;
        }

        if self.unit_ai_guard_city(g, u, true, false, MAX_INT) {
            return;
        }

        if !danger {
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_SETTLE, /*iMaxGroup*/ 1, -1, -1, false, false, false, /*iMaxPath*/ 2, /*bAllowRegrouping*/ true) {
                return;
            }

            if self.unit_ai_group(g, u, UnitAIType::UNITAI_SETTLE, /*iMaxGroup*/ 2, -1, -1, false, false, false, /*iMaxPath*/ 2, /*bAllowRegrouping*/ true) {
                return;
            }

            if plot_owned_by_us(g, u) {
                if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_SETTLE), -1, -1, 1, -1, MOVE_SAFE_TERRITORY, MAX_INT) {
                    return;
                }
            }
        }

        let area_ai = area_ai_type(g, u);
        if area_ai == Some(AreaAIType::AREAAI_ASSAULT) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_MASSING) || area_ai == Some(AreaAIType::AREAAI_ASSAULT_ASSIST) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, Some(UnitAIType::UNITAI_ATTACK_CITY), -1, -1, -1, 0, MOVE_SAFE_TERRITORY, MAX_INT) {
                return;
            }
        }

        if g.units[u].ai_birthmark % 4 == 0 {
            if self.unit_ai_guard_fort(g, u, true) {
                return;
            }
        }

        if self.unit_ai_guard_city_airlift(g, u) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if plot_owned_by_us(g, u) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_SETTLE), 3, -1, -1, -1, MOVE_SAFE_TERRITORY, MAX_INT) {
                // will enter here if in danger
                return;
            }
        }

        if self.unit_ai_guard_city(g, u, false, true, MAX_INT) {
            return;
        }
        if !g.unit_is_barbarian(u) && (area_ai_type(g, u) == Some(AreaAIType::AREAAI_OFFENSIVE) || area_ai_type(g, u) == Some(AreaAIType::AREAAI_MASSING)) {
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, -1, 2, 4, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
                return;
            }
        }

        if area_ai_type(g, u) == Some(AreaAIType::AREAAI_ASSAULT) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_ASSAULT_SEA, MissionAIType::MISSIONAI_LOAD_ASSAULT, Some(UnitAIType::UNITAI_ATTACK_CITY), 1, 2, -1, 1, MOVE_SAFE_TERRITORY, MAX_INT) {
                // does this ever occur? the previous settler check is less strict, this one should never be true (I think)
                return;
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_cityDefenseExtraMove()` (CvUnitAI.cpp:4026-4108)
    pub fn unit_ai_city_defense_extra_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if self.unit_ai_leave_attack(g, u, 2, 55, 150) {
            return;
        }

        if self.unit_ai_choke_defend(g, u) {
            return;
        }

        if self.unit_ai_guard_city_best_defender(g, u) {
            return;
        }

        if self.unit_ai_guard_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_SETTLE, /*iMaxGroup*/ 1, -1, -1, false, false, false, /*iMaxPath*/ 2, /*bAllowRegrouping*/ true) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_SETTLE, /*iMaxGroup*/ 2, -1, -1, false, false, false, /*iMaxPath*/ 2, /*bAllowRegrouping*/ true) {
            return;
        }

        let city = g.map.plots[g.unit_plot(u)].plot_city;

        if let Some(city) = city {
            if g.cities[city].owner == p {
                // XXX check for other team?
                let unit_ai = opt_to_i32(g.units[u].ai_type);
                let count = g.plot_plot_count(g.unit_plot(u), Some(Game::puf_can_defend_group_head), -1, -1, Some(p), None, Some(Game::puf_is_unit_ai_type), unit_ai, -1);
                if count == 1 {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return;
                }
            }
        }

        if self.unit_ai_guard_city_airlift(g, u) {
            return;
        }

        if self.unit_ai_guard_city(g, u, false, true, 1) {
            return;
        }

        if plot_owned_by_us(g, u) {
            if self.unit_ai_load(g, u, UnitAIType::UNITAI_SETTLER_SEA, MissionAIType::MISSIONAI_LOAD_SETTLER, Some(UnitAIType::UNITAI_SETTLE), 3, -1, -1, -1, MOVE_SAFE_TERRITORY, 3) {
                return;
            }
        }

        if self.unit_ai_guard_city(g, u, false, true, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_exploreMove()` (CvUnitAI.cpp:4111-4239)
    pub fn unit_ai_explore_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        // BETTER AI (first city) Sephi
        if g.players[p].num_cities() == 0 && g.game_turn() < 10 {
            if !g.players[p].human {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        if !g.unit_is_human(u) && g.unit_can_attack(u) {
            if self.unit_ai_city_attack(g, u, 1, 60, false) {
                return;
            }

            if self.unit_ai_any_attack(g, u, 1, 70, 0, false) {
                return;
            }
        }

        if g.units[u].damage > 0 {
            let no_turn_damage = match g.map.plots[g.unit_plot(u)].feature_type {
                None => true,
                Some(f) => g.d.features[f].turn_damage == 0,
            };
            if no_turn_damage {
                push_mission(g, u, MissionType::MISSION_HEAL);
                return;
            }
        }

        if !g.unit_is_human(u) {
            if self.unit_ai_pillage_range(g, u, 1, 0) {
                return;
            }

            if self.unit_ai_city_attack(g, u, 3, 80, false) {
                return;
            }
        }

        if self.unit_ai_goody(g, u, 4) {
            return;
        }

        if self.unit_ai_explore_range(g, u, 3) {
            return;
        }

        if !g.unit_is_human(u) {
            if self.unit_ai_pillage_range(g, u, 3, 0) {
                return;
            }
        }

        if self.unit_ai_explore(g, u) {
            return;
        }

        if !g.unit_is_human(u) {
            if self.unit_ai_pillage(g, u, 0) {
                return;
            }
        }

        if !g.unit_is_human(u) {
            if self.unit_ai_travel_to_upgrade_city(g, u) {
                return;
            }
        }

        // FfH (Kael 08/21/2008) commented out the scrapping of surplus explorers here.

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_missionaryMove()` (CvUnitAI.cpp:4242-4296)
    pub fn unit_ai_missionary_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_spread_religion(g, u) {
            return;
        }

        if self.unit_ai_spread_corporation(g, u) {
            return;
        }

        let t = g.unit_get_team(u);
        if !g.unit_is_human(u) || (g.unit_is_automated(u) && g.team_get_at_war_count(t, true) == 0) {
            if !g.unit_is_human(u) || g.units[u].game_turn_created < g.game_turn() {
                if self.unit_ai_spread_religion_airlift(g, u) {
                    return;
                }
                if self.unit_ai_spread_corporation_airlift(g, u) {
                    return;
                }
            }

            if !g.unit_is_human(u) {
                if self.unit_ai_load(g, u, UnitAIType::UNITAI_MISSIONARY_SEA, MissionAIType::MISSIONAI_LOAD_SPECIAL, None, -1, -1, -1, 0, MOVE_SAFE_TERRITORY, MAX_INT) {
                    return;
                }

                if self.unit_ai_load(g, u, UnitAIType::UNITAI_MISSIONARY_SEA, MissionAIType::MISSIONAI_LOAD_SPECIAL, None, -1, -1, -1, 0, MOVE_NO_ENEMY_TERRITORY, MAX_INT) {
                    return;
                }
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_prophetMove()` (CvUnitAI.cpp:4299-4393)
    pub fn unit_ai_prophet_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        // BETTER AI (first city) Sephi 0.41k
        let altar_victory = match g.player_get_leader_type(p) {
            Some(leader) => g.d.leader_heads[leader].altar_victory,
            None => false,
        };
        if altar_victory {
            if self.unit_ai_construct(g, u, 10000, 10000, 15) {
                return;
            }
        }

        if self.unit_ai_construct(g, u, 1, MAX_INT, 15) {
            return;
        }

        if self.unit_ai_discover(g, u, true, true) {
            return;
        }

        if self.unit_ai_construct(g, u, 3, MAX_INT, 15) {
            return;
        }

        great_person_tail(self, g, u);
    }

    /// C++ `void CvUnitAI::AI_artistMove()` (CvUnitAI.cpp:4396-4494)
    pub fn unit_ai_artist_move(&self, g: &mut Game, u: UnitId) {
        // Skyre Mod / BETTER AI (use Artists to end Disorder) merged Sephi
        let city = g.map.plots[g.unit_plot(u)].plot_city;
        if let Some(city) = city {
            if g.city_is_disorder(city) {
                push_mission(g, u, MissionType::MISSION_GREAT_WORK);
                return;
            }
        }

        if self.unit_ai_artist_culture_victory_move(g, u) {
            return;
        }

        if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
            return;
        }

        if self.unit_ai_discover(g, u, true, true) {
            return;
        }

        if self.unit_ai_great_work(g, u) {
            return;
        }

        great_person_tail(self, g, u);
    }

    /// C++ `void CvUnitAI::AI_scientistMove()` (CvUnitAI.cpp:4497-4585)
    pub fn unit_ai_scientist_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if self.unit_ai_discover(g, u, true, true) {
            return;
        }

        if self.unit_ai_construct(g, u, MAX_INT, 1, 15) {
            return;
        }
        if g.players[p].current_era.i() < 3 {
            if self.unit_ai_join(g, u, 2) {
                return;
            }
        }

        if g.players[p].current_era.i() <= g.d.eras.len() as i32 / 2 {
            if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
                return;
            }
        }

        great_person_tail(self, g, u);
    }

    /// C++ `void CvUnitAI::AI_generalMove()` (CvUnitAI.cpp:4588-4668)
    pub fn unit_ai_general_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;
        let mut unit_ai_types: Vec<i32> = Vec::new();
        let danger = self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 2, true);

        let offense_war = area_ai_type(g, u) == Some(AreaAIType::AREAAI_OFFENSIVE);

        if danger > 0 {
            unit_ai_types.clear();
            unit_ai_types.push(UnitAIType::UNITAI_ATTACK.i());
            unit_ai_types.push(UnitAIType::UNITAI_COUNTER.i());
            if self.unit_ai_lead(g, u, &mut unit_ai_types) {
                return;
            }
        }

        if self.unit_ai_construct(g, u, 1, MAX_INT, 15) {
            return;
        }
        if self.unit_ai_join(g, u, 1) {
            return;
        }

        if offense_war && self.rand(2, "AI General Lead") == 0 {
            unit_ai_types.clear();
            unit_ai_types.push(UnitAIType::UNITAI_ATTACK_CITY.i());
            if self.unit_ai_lead(g, u, &mut unit_ai_types) {
                return;
            }
        }

        if self.unit_ai_join(g, u, 2) {
            return;
        }

        if self.unit_ai_construct(g, u, 2, MAX_INT, 15) {
            return;
        }
        if self.unit_ai_join(g, u, 4) {
            return;
        }

        if self.rand(3, "AI General Construct") == 0 {
            if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
                return;
            }
        }

        if self.unit_ai_join(g, u, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_merchantMove()` (CvUnitAI.cpp:4671-4754)
    pub fn unit_ai_merchant_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
            return;
        }

        if self.unit_ai_discover(g, u, true, true) {
            return;
        }

        let golden_age_value = self.player_ai_calculate_golden_age_value(g, p) / g.player_units_required_for_golden_age(p);
        let discover_value = 1.max(g.unit_get_discover_research(u, None));

        if self.unit_ai_trade(g, u, golden_age_value * 2) {
            return;
        }

        if (golden_age_value * 100) / discover_value > 60 {
            if self.unit_ai_golden_age(g, u) {
                return;
            }

            if self.unit_ai_trade(g, u, golden_age_value) {
                return;
            }

            if discover_value > golden_age_value {
                if self.unit_ai_discover(g, u, false, false) {
                    return;
                }
                let class = g.unit_get_unit_class_type(u);
                if g.players[p].unit_class_count[class] > 1 {
                    if self.unit_ai_join(g, u, MAX_INT) {
                        return;
                    }
                }
            }
        } else {
            if self.unit_ai_discover(g, u, false, false) {
                return;
            }

            if self.unit_ai_join(g, u, MAX_INT) {
                return;
            }
        }

        if self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 2, true) > 0 || g.units[u].game_turn_created < g.game_turn() - 25 {
            if self.unit_ai_discover(g, u, false, false) {
                return;
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_engineerMove()` (CvUnitAI.cpp:4757-4840)
    pub fn unit_ai_engineer_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
            return;
        }

        if self.unit_ai_switch_hurry(g, u) {
            return;
        }

        if self.unit_ai_hurry(g, u) {
            return;
        }

        if self.unit_ai_discover(g, u, true, true) {
            return;
        }

        great_person_tail(self, g, u);
    }

    /// C++ `void CvUnitAI::AI_spyMove()` (CvUnitAI.cpp:4843-4996)
    pub fn unit_ai_spy_move(&self, g: &mut Game, u: UnitId) {
        let t = g.unit_get_team(u);
        let mut espionage_chance = 0;
        if g.plot_is_owned(g.unit_plot(u)) && plot_team(g, u) != Some(t) {
            let plot_t = plot_team(g, u).expect("owned plot without a team");
            espionage_chance = match self.team_ai_get_attitude(g, t, plot_t, true) {
                AttitudeType::ATTITUDE_FURIOUS => 100,
                AttitudeType::ATTITUDE_ANNOYED => 50,
                AttitudeType::ATTITUDE_CAUTIOUS => 0,
                AttitudeType::ATTITUDE_PLEASED => 0,
                AttitudeType::ATTITUDE_FRIENDLY => 0,
                _ => 0,
            };

            let war_plan = g.teams[t].ai_war_plan[plot_t];
            if let Some(war_plan) = war_plan {
                if war_plan == WarPlanType::WARPLAN_LIMITED {
                    espionage_chance += 50;
                } else {
                    espionage_chance += 10;
                }
            }

            if plot_is_city(g, u) && plot_team(g, u) != Some(t) {
                if g.units[u].fortify_turns >= g.d.defines.max_fortify_turns {
                    if self.unit_ai_espionage_spy(g, u) {
                        return;
                    }
                }
                if self.rand(100, "AI Spy Skip Turn") > 5 {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return;
                }
            }

            if self.rand(100, "AI Spy Espionage") < espionage_chance {
                if self.unit_ai_espionage_spy(g, u) {
                    return;
                }
            }
        }

        if plot_team(g, u) == Some(t) {
            if g.team_get_any_war_plan_count(t, true) == 0 {
                if self.unit_ai_guard_spy(g, u, 0) {
                    return;
                }
            }

            if self.rand(100, "AI Spy pillage improvement") < 25 {
                if self.unit_ai_bonus_offense_spy(g, u, 3) {
                    return;
                }
            } else {
                if self.unit_ai_city_offense_spy(g, u, 10) {
                    return;
                }
            }
        }

        if espionage_chance > 0 && (plot_is_city(g, u) || g.plot_get_non_obsolete_bonus_type(g.unit_plot(u), t).is_some()) {
            if self.rand(7, "AI Spy Skip Turn") > 0 {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        if self.rand(4, "AI Spy Choose Movement") != 0 {
            if self.unit_ai_recon_spy(g, u, 3) {
                return;
            }
        } else {
            if self.unit_ai_city_offense_spy(g, u, 10) {
                return;
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_ICBMMove()` (CvUnitAI.cpp:4999-5062)
    pub fn unit_ai_icbm_move(&self, g: &mut Game, u: UnitId) {
        let air_range = g.unit_air_range(u);
        if air_range > 0 {
            if self.unit_ai_nuke_range(g, u, air_range) {
                return;
            }
        } else if self.unit_ai_nuke(g, u) {
            return;
        }

        if g.unit_is_cargo(u) {
            push_mission(g, u, MissionType::MISSION_SKIP);
            return;
        }

        if g.unit_air_range(u) > 0 {
            if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_MISSILE_CARRIER_SEA, 2, true) {
                return;
            }

            if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_MISSILE_CARRIER_SEA, 1, false) {
                return;
            }

            if g.units[u].ai_birthmark % 3 == 0 {
                if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_ATTACK_SEA, 0, false) {
                    return;
                }
            }

            if self.unit_ai_air_offensive_city(g, u) {
                return;
            }
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_workerSeaMove()` (CvUnitAI.cpp:5065-5183)
    pub fn unit_ai_worker_sea_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if !g.group_can_defend(group_of(g, u)) {
            if self.player_ai_get_plot_danger(g, p, g.unit_plot(u), -1, true) > 0 {
                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    return;
                }
            }
        }

        // Skyre Mod / BETTER AI (Lanun Pirate Coves) merged Sephi
        if g.player_is_pirate(p) {
            if self.unit_ai_build_pirate_cove(g, u) {
                return;
            }
        }

        if self.unit_ai_improve_bonus(g, u, 20, &mut None, &mut None, &mut 0) {
            return;
        }

        if self.unit_ai_improve_bonus(g, u, 10, &mut None, &mut None, &mut 0) {
            return;
        }

        if self.unit_ai_improve_bonus(g, u, 0, &mut None, &mut None, &mut 0) {
            return;
        }

        if g.unit_is_human(u) {
            let pl = g.unit_plot(u);
            if g.plot_get_bonus_type(pl, None).is_some() {
                if g.map.plots[pl].owner == Some(p) || !g.plot_is_owned(pl) {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return;
                }
            }

            for i in 0..DirectionType::NUM {
                let (x, y) = (g.units[u].x, g.units[u].y);
                if let Some(loop_plot) = g.plot_direction(x, y, DirectionType::from_usize(i)) {
                    if g.plot_get_bonus_type(loop_plot, None).is_some() {
                        if g.plot_is_valid_domain_for_location(loop_plot, u) {
                            push_mission(g, u, MissionType::MISSION_SKIP);
                            return;
                        }
                    }
                }
            }
        }

        if !g.unit_is_human(u) && g.units[u].ai_type == Some(UnitAIType::UNITAI_WORKER_SEA) {
            let city = g.map.plots[g.unit_plot(u)].plot_city;

            if let Some(city) = city {
                if g.cities[city].owner == p {
                    if self.city_ai_needed_sea_workers(g, city) == 0 {
                        if g.elapsed_game_turns > 10 {
                            if g.player_calculate_unit_cost(p) > 0 {
                                g.unit_scrap(u);
                                return;
                            }
                        }
                    } else {
                        //Probably icelocked since it can't perform actions.
                        g.unit_scrap(u);
                        return;
                    }
                }
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_barbAttackSeaMove()` (CvUnitAI.cpp:5186-5230)
    pub fn unit_ai_barb_attack_sea_move(&self, g: &mut Game, u: UnitId) {
        if self.rand(2, "AI Barb") == 0 {
            if self.unit_ai_pillage_range(g, u, 1, 0) {
                return;
            }
        }

        if self.unit_ai_any_attack(g, u, 2, 25, 0, false) {
            return;
        }

        if self.unit_ai_pillage_range(g, u, 4, 0) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_pirateSeaMove()` (CvUnitAI.cpp:5232-5363)
    pub fn unit_ai_pirate_sea_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        // BETTER AI (Naval AI) Sephi
        if g.player_is_pirate(p) {
            if g.units[u].cargo > 0 {
                //time to plunder
                if self.unit_ai_assault_sea_transport(g, u, false) {
                    return;
                }
            }
        }

        if self.unit_ai_pillage_range(g, u, 3, 0) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 3, 30, 0, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 20, 0, 0, false) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);

        // C++ quirk: the rest of the BTS function (heal in city, protect, group with pirates,
        // explore, pillage, AI_pirateBlockade, retreat, safety) follows an unconditional
        // `return` after Sephi's block and never runs; not ported.
    }

    /// C++ `void CvUnitAI::AI_attackSeaMove()` (CvUnitAI.cpp:5366-5477)
    pub fn unit_ai_attack_sea_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 35, 0, false) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 2, 40, 0, false) {
            return;
        }

        if self.unit_ai_sea_bombard_range(g, u, 6) {
            return;
        }

        if self.unit_ai_heal(g, u, 50, 3) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_CARRIER_SEA, /*iMaxGroup*/ 4, 1, -1, true, false, false, /*iMaxPath*/ 5, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_SEA, /*iMaxGroup*/ 1, -1, -1, true, false, false, /*iMaxPath*/ 3, false) {
            return;
        }

        if !g.plot_is_owned(g.unit_plot(u)) || !g.unit_is_enemy(u, plot_team(g, u), None) {
            if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_ASSAULT_SEA, 4, 34, true) {
                return;
            }

            if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_CARRIER_SEA, 4, 51, true) {
                return;
            }
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, 4, -1, false, false, false, MAX_INT, false) {
                return;
            }
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_CARRIER_SEA, -1, 1, -1, false, false, false, MAX_INT, false) {
            return;
        }

        if g.plot_is_owned(g.unit_plot(u)) && g.unit_is_enemy(u, plot_team(g, u), None) {
            if self.unit_ai_blockade(g, u) {
                return;
            }
        }

        if self.unit_ai_pillage_range(g, u, 4, 0) {
            return;
        }

        if self.unit_ai_protect(g, u, 35) {
            return;
        }

        if self.unit_ai_travel_to_upgrade_city(g, u) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_reserveSeaMove()` (CvUnitAI.cpp:5480-5577)
    pub fn unit_ai_reserve_sea_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_guard_bonus(g, u, 30) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 55, 0, false) {
            return;
        }

        if self.unit_ai_sea_bombard_range(g, u, 6) {
            return;
        }

        if self.unit_ai_protect(g, u, 40) {
            return;
        }

        if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_SETTLER_SEA, 1, -1, true) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_RESERVE_SEA, 1, -1, -1, false, false, false, MAX_INT, false) {
            return;
        }

        if g.unit_bombard_rate(u) > 0 {
            if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_ASSAULT_SEA, 2, 30, true) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 50, 3) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 3, 45, 0, false) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if !g.unit_is_never_invisible(u) {
            if self.unit_ai_any_attack(g, u, 5, 35, 0, false) {
                return;
            }
        }

        if self.unit_ai_travel_to_upgrade_city(g, u) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_escortSeaMove()` (CvUnitAI.cpp:5580-5692)
    pub fn unit_ai_escort_sea_move(&self, g: &mut Game, u: UnitId) {
        // (The C++ comments out a conversion of escorts with cargo to UNITAI_ASSAULT_SEA.)

        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_heal(g, u, 30, 1) {
            return;
        }

        if self.unit_ai_any_attack(g, u, 1, 55, 0, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_CARRIER_SEA, -1, /*iMaxOwnUnitAI*/ 0, -1, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, /*iMaxOwnUnitAI*/ 0, -1, /*bIgnoreFaster*/ true, false, false, /*iMaxPath*/ 3, false) {
            return;
        }

        if self.unit_ai_heal(g, u, 50, 3) {
            return;
        }

        if self.unit_ai_pillage_range(g, u, 2, 0) {
            return;
        }

        // C++ quirk: `AI_group(UNITAI_MISSILE_CARRIER_SEA, 1, 1, true)` passes `true` as
        // iMinUnitAI (1), not as bIgnoreFaster.
        if self.unit_ai_group(g, u, UnitAIType::UNITAI_MISSILE_CARRIER_SEA, 1, 1, 1, false, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, 1, /*iMaxOwnUnitAI*/ 0, /*iMinUnitAI*/ -1, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, /*iMaxOwnUnitAI*/ 2, /*iMinUnitAI*/ -1, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_CARRIER_SEA, -1, /*iMaxOwnUnitAI*/ 2, /*iMinUnitAI*/ -1, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, /*iMaxOwnUnitAI*/ 4, /*iMinUnitAI*/ -1, /*bIgnoreFaster*/ true, false, false, MAX_INT, false) {
            return;
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }

        if self.unit_ai_travel_to_upgrade_city(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }
}
