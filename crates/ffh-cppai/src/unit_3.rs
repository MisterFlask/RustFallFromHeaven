//! Port of CvUnitAI.cpp:5695-8506 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `AI_attackAirMove` calls `AI_bestAreaUnitAIValue(UNITAI_CARRIER_SEA, NULL, &eBestCarrierUnit)`
//!   with a NULL area, which `player_ai_best_area_unit_ai_value` (area: `AreaId`) cannot take; the
//!   NULL path (`AI_bestCityUnitAIValue` with no city: `CvPlayer::canTrain` over the civ's units)
//!   is ported privately here (`best_unit_ai_unit_any_city`). If `player_ai_best_area_unit_ai_value`
//!   / `player_ai_best_city_unit_ai_value` / `player_ai_unit_value` get `Option` area/city
//!   parameters, that helper can become `player_ai_best_area_unit_ai_value(g, p, ai, None, ..)`.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// C++ `getGroup()` of a unit (always set for a unit on the map).
fn gr(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnit::getGroup: unit without a group")
}

/// C++ `AI_getUnitAIType()` (NO_UNITAI never happens for a unit that went through `AI_init`).
fn ai_of(g: &Game, u: UnitId) -> UnitAIType {
    g.units[u].ai_type.unwrap_or(UnitAIType::UNITAI_UNKNOWN)
}

/// C++ `getGroup()->getNumUnits()`.
fn num_units(g: &Game, grp: GroupId) -> i32 {
    g.groups[grp].units.len() as i32
}

/// C++ `getGroup()->pushMission(eMission)` with every default argument.
fn push(g: &mut Game, grp: GroupId, mission: MissionType) {
    g.group_push_mission(grp, mission, -1, -1, 0, false, false, None, None, None);
}

/// C++ `getGroup()->pushMission(eMission)` for unit `u`.
fn push_u(g: &mut Game, u: UnitId, mission: MissionType) {
    let grp = gr(g, u);
    push(g, grp, mission);
}

/// C++ `plot()->isCity()`.
fn plot_is_city(g: &Game, plot: PlotIdx) -> bool {
    g.plot_is_city(plot, false, None)
}

/// C++ `plot()->isCity() && plot()->getOwnerINLINE() == getOwnerINLINE()`.
fn in_own_city(g: &Game, u: UnitId) -> bool {
    let plot = g.unit_plot(u);
    plot_is_city(g, plot) && g.map.plots[plot].owner == Some(g.units[u].owner)
}

/// C++ `getGroup()->unloadAll(); getGroup()->pushMission(MISSION_SKIP);`.
fn unload_and_skip(g: &mut Game, u: UnitId) {
    let grp = gr(g, u);
    g.group_unload_all(grp);
    if g.units.contains(u) {
        push_u(g, u, MissionType::MISSION_SKIP);
    }
}

/// C++ `area()->getAreaAIType(getTeam())`.
fn area_ai_of(g: &Game, u: UnitId) -> Option<AreaAIType> {
    let area = g.unit_area(u);
    let team = g.unit_get_team(u);
    g.map.areas[area].area_ai_type[team]
}

impl Cai {
    /// `CvPlayerAI::AI_bestAreaUnitAIValue(eUnitAI, NULL, &eBest)` = `AI_bestCityUnitAIValue(eUnitAI,
    /// NULL, &eBest)`: the best unit for `unit_ai` the player can train anywhere (best unit type
    /// only; the value is unused by the caller).
    fn best_unit_ai_unit_any_city(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType, area: AreaId) -> Option<UnitType> {
        let civ = g.player_get_civilization_type(p)?;
        let mut best_value = 0;
        let mut best_unit = None;
        for class in g.d.unit_classes.keys() {
            let Some(loop_unit) = g.d.civilizations[civ].civilization_units(class, g.d.unit_classes[class].default_unit_index) else {
                continue;
            };
            if !g.players[p].human || g.d.units[loop_unit].default_unit_ai_type == Some(unit_ai) {
                if g.player_can_train(p, loop_unit, false, false, false) {
                    // PORT ADAPT: C++ passes a NULL area to AI_unitValue here; the only caller asks
                    // for UNITAI_CARRIER_SEA, whose value does not read the area.
                    let value = self.player_ai_unit_value(g, p, loop_unit, unit_ai, area);
                    if value > best_value {
                        best_value = value;
                        best_unit = Some(loop_unit);
                    }
                }
            }
        }
        best_unit
    }

    /// C++ `void CvUnitAI::AI_exploreSeaMove()` (CvUnitAI.cpp:5695-5835)
    pub fn unit_ai_explore_sea_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        let human = g.unit_is_human(u);
        let p = g.units[u].owner;

        if !human {
            if self.unit_ai_any_attack(g, u, 1, 60, 0, false) {
                return;
            }
        }

        if !human && !g.unit_is_barbarian(u) {
            //XXX move some of this into a function? maybe useful elsewhere
            //Obsolete?
            let unit_type = g.units[u].unit_type;
            let area = g.unit_area(u);
            let ai = ai_of(g, u);
            let value = self.player_ai_unit_value(g, p, unit_type, ai, area);
            let best_value = self.player_ai_best_area_unit_ai_value(g, p, ai, area, &mut None);

            if value < best_value {
                //Transform
                if self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_WORKER_SEA, area) > 0 {
                    self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_WORKER_SEA);
                    return;
                }

                if self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_PIRATE_SEA, area) > 0 {
                    self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_PIRATE_SEA);
                    return;
                }

                if self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_MISSIONARY_SEA, area) > 0 {
                    self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_MISSIONARY_SEA);
                    return;
                }

                if self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_RESERVE_SEA, area) > 0 {
                    self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_RESERVE_SEA);
                    return;
                }
                // C++ quirk: no return after scrap() (the unit dies with delay and goes on moving).
                g.unit_scrap(u);
                if !g.units.contains(u) {
                    return;
                }
            }
        }

        if g.units[u].damage > 0 {
            let plot = g.unit_plot(u);
            let feature = g.map.plots[plot].feature_type;
            if feature.is_none() || g.d.features[feature.unwrap()].turn_damage == 0 {
                push_u(g, u, MissionType::MISSION_HEAL);
                return;
            }
        }

        if !human {
            if self.unit_ai_pillage_range(g, u, 1, 0) {
                return;
            }
        }

        if self.unit_ai_explore_range(g, u, 4) {
            return;
        }

        if !human {
            if self.unit_ai_pillage_range(g, u, 4, 0) {
                return;
            }
        }

        if self.unit_ai_explore(g, u) {
            return;
        }

        if !human {
            if self.unit_ai_pillage(g, u, 0) {
                return;
            }
        }

        if !human {
            if self.unit_ai_travel_to_upgrade_city(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if !human && g.units[u].ai_type == Some(UnitAIType::UNITAI_EXPLORE_SEA) {
            let plot = g.unit_plot(u);
            if let Some(water_area) = g.plot_water_area(plot) {
                if self.player_ai_total_water_area_unit_ais(g, p, water_area, UnitAIType::UNITAI_EXPLORE_SEA)
                    > self.player_ai_needed_explorers(g, p, water_area)
                {
                    if g.player_calculate_unit_cost(p) > 0 {
                        g.unit_scrap(u);
                        return;
                    }
                }
            }
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

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_assaultSeaMove()` (CvUnitAI.cpp:5838-6100)
    pub fn unit_ai_assault_sea_move(&self, g: &mut Game, u: UnitId) {
        let empty = !g.group_has_cargo(gr(g, u));
        if empty {
            if self.unit_ai_any_attack(g, u, 1, 65, 0, false) {
                return;
            }
            if self.unit_ai_any_attack(g, u, 1, 40, 0, false) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let p = g.units[u].owner;
        let team = g.unit_get_team(u);
        let mut attack = false;
        let is_city = plot_is_city(g, g.unit_plot(u));
        let target_stack_size = 4.max(1 + self.unit_ai_stack_of_doom_extra(g, u));
        let cargo = g.group_get_cargo(gr(g, u));
        let full = self.group_ai_is_full(g, gr(g, u));

        let area_ai = area_ai_of(g, u);
        let land_war = !g.unit_is_barbarian(u)
            && (area_ai == Some(AreaAIType::AREAAI_OFFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE)
                || area_ai == Some(AreaAIType::AREAAI_MASSING));

        if is_city {
            if area_ai == Some(AreaAIType::AREAAI_ASSAULT) {
                if cargo >= target_stack_size {
                    attack = true;
                }
            }
            if !attack {
                if area_ai == Some(AreaAIType::AREAAI_ASSAULT) && cargo > 0 {
                    let attackers = self.player_ai_enemy_target_mission_ais(g, p, MissionAIType::MISSIONAI_ASSAULT, Some(gr(g, u)));
                    if attackers >= target_stack_size {
                        if full {
                            //Join the attack
                            attack = true;
                        }
                    }
                } else if self.player_ai_get_plot_danger(g, p, g.unit_plot(u), 2, true) != 0 {
                    if g.group_has_cargo(gr(g, u)) {
                        unload_and_skip(g, u);
                        return;
                    }
                } else if land_war {
                    if full && area_ai != Some(AreaAIType::AREAAI_DEFENSIVE) {
                        if self.unit_ai_assault_sea_transport(g, u, false) {
                            return;
                        }
                    } else if cargo > 0 {
                        unload_and_skip(g, u);
                        return;
                    }
                } else if self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_LOAD_ASSAULT, None) > 0 {
                    if !full {
                        push_u(g, u, MissionType::MISSION_SKIP);
                        return;
                    }
                } else {
                    if !land_war && cargo > 0 {
                        if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, -1, -1, false, false, false, MAX_INT, false) {
                            return;
                        }
                    }
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let plot = g.unit_plot(u);
        if g.plot_get_team(plot) == Some(team) {
            if cargo == 0 && num_units(g, gr(g, u)) > 1 {
                self.group_ai_make_force_separate(g, gr(g, u));
            }
        }

        if !is_city {
            let plot = g.unit_plot(u);
            if g.plot_is_owned(plot) && g.unit_is_enemy(u, g.plot_get_team(plot), None) {
                if cargo == 0 {
                    // if we just made a dropoff, bombard the city if we can
                    let grp = gr(g, u);
                    if g.group_count_num_unit_ai_type(grp, Some(UnitAIType::UNITAI_ATTACK_SEA))
                        + g.group_count_num_unit_ai_type(grp, Some(UnitAIType::UNITAI_RESERVE_SEA))
                        > 0
                    {
                        let mut mission_pushed = false;

                        if self.unit_ai_sea_bombard_range(g, u, 1) {
                            mission_pushed = true;
                        }
                        if !g.units.contains(u) {
                            return;
                        }

                        let old_group = g.units[u].group;

                        //Release any Warships to finish the job.
                        self.group_ai_seperate_ai(g, gr(g, u), UnitAIType::UNITAI_ATTACK_SEA);
                        self.group_ai_seperate_ai(g, gr(g, u), UnitAIType::UNITAI_RESERVE_SEA);

                        // UNOFFICIAL_PATCH: checking unit AI instead of unit type
                        if old_group == g.units[u].group && g.units[u].ai_type == Some(UnitAIType::UNITAI_ASSAULT_SEA) {
                            // Need to be sure all units can move
                            if g.group_can_all_move(gr(g, u)) {
                                if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
                                    mission_pushed = true;
                                }
                            }
                        }

                        if mission_pushed {
                            return;
                        }
                    }
                }
            }

            if cargo > target_stack_size || full {
                attack = true;
            }
        }

        if g.unit_is_barbarian(u) {
            if g.group_is_full(gr(g, u)) {
                if self.unit_ai_assault_sea_transport(g, u, false) {
                    return;
                }
            } else {
                if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_ATTACK_CITY) {
                    return;
                }

                if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_ATTACK) {
                    return;
                }
            }
        } else {
            let attack_barbarian = g.team_get_at_war_count(team, true) == 0;

            if attack {
                if self.unit_ai_assault_sea_transport(g, u, attack_barbarian) {
                    return;
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if full {
            if self.unit_ai_group(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, -1, -1, true, false, false, MAX_INT, false) {
                return;
            }
        } else {
            if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_ATTACK_CITY) {
                return;
            }

            if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_ATTACK) {
                return;
            }

            if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_COUNTER) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        // if we are in a city, and at/preparing land war, and we have cargo, unload, even if full, since AI_assaultSeaTransport already had its chance
        if is_city && land_war && g.group_has_cargo(gr(g, u)) {
            unload_and_skip(g, u);
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_missionarySeaMove()` (CvUnitAI.cpp:6102-6167)
    pub fn unit_ai_missionary_sea_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_MISSIONARY) > 0 {
            if self.unit_ai_special_sea_transport_missionary(g, u) {
                return;
            }
        } else if !g.group_has_cargo(gr(g, u)) {
            if self.unit_ai_pillage_range(g, u, 4, 0) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        // BETTER_BTS_AI_MOD merged Sephi 01/14/09 jdog5000: Naval AI
        if !g.group_is_full(gr(g, u)) {
            if self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_LOAD_SPECIAL, None) > 0 {
                // Wait for units on the way
                push_u(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_MISSIONARY) {
            return;
        }
        // BETTER_BTS_AI_MOD END

        if self.unit_ai_explore(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_settlerSeaMove()` (CvUnitAI.cpp:6169-6378)
    pub fn unit_ai_settler_sea_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;
        let empty = !g.group_has_cargo(gr(g, u));

        if empty {
            if self.unit_ai_any_attack(g, u, 1, 65, 0, false) {
                return;
            }
            if self.unit_ai_any_attack(g, u, 1, 40, 0, false) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let settler_count = g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_SETTLE);
        let worker_count = g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_WORKER);

        // BETTER_BTS_AI_MOD 0.41k 12/07/08 jdog5000: Naval AI merged Sephi
        if g.unit_has_cargo(u) && settler_count == 0 && worker_count == 0 {
            // Dump troop load at first oppurtunity after pick up
            if in_own_city(g, u) {
                unload_and_skip(g, u);
                return;
            } else {
                if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
                    return;
                }

                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    return;
                }
            }
        }
        // BETTER_BTS_AI_MOD END

        if !g.units.contains(u) {
            return;
        }

        // BETTER_BTS_AI_MOD 0.41k 06/02/09 jdog5000: Settler AI merged Sephi
        // Don't send transport with settler and no defense
        if settler_count > 0 && settler_count + worker_count == g.units[u].cargo_capacity {
            // No defenders for settler
            if in_own_city(g, u) {
                unload_and_skip(g, u);
                return;
            }
        }

        // Don't send transport with too many workers
        if settler_count == 0 && worker_count > 0 {
            // No defenders for settler
            if in_own_city(g, u) {
                unload_and_skip(g, u);
                return;
            }
        }

        if settler_count > 0
            && (g.unit_is_full(u)
                || (g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_CITY_DEFENSE) > 0
                    && self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_LOAD_SETTLER, None) == 0))
        // BETTER_BTS_AI_MOD END
        {
            if self.unit_ai_settler_sea_transport(g, u) {
                return;
            }
        } else if Some(g.unit_get_team(u)) != g.plot_get_team(g.unit_plot(u)) && empty {
            if self.unit_ai_pillage_range(g, u, 3, 0) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        // (0.41k: the AREAAI_ASSAULT switch to UNITAI_ASSAULT_SEA is commented out in the C++.)

        if worker_count > 0 && self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_LOAD_SETTLER, None) == 0 {
            if g.unit_is_full(u) || settler_count == 0 {
                if self.unit_ai_settler_sea_ferry(g, u) {
                    return;
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        // BETTER_BTS_AI_MOD merged Sephi 0.41k 01/14/09 jdog5000: Settler AI
        // (AI_pickup(UNITAI_SETTLE) here is commented out in the C++.)
        if !g.group_is_full(gr(g, u)) {
            if self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_LOAD_SETTLER, None) > 0 {
                // Wait for units on the way
                push_u(g, u, MissionType::MISSION_SKIP);
                return;
            }

            if settler_count > 0 {
                if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_COUNTER) {
                    return;
                }
            } else {
                if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_SETTLE) {
                    return;
                }
            }
        }
        // BETTER_BTS_AI_MOD END

        if !g.units.contains(u) {
            return;
        }

        if g.game_turn() - g.units[u].game_turn_created < 8 {
            let plot = g.unit_plot(u);
            let area = g.unit_area(u);
            if g.map.plots[plot].plot_city.is_none() || self.player_ai_total_area_unit_ais(g, p, area, UnitAIType::UNITAI_SETTLE) == 0 {
                if self.unit_ai_explore(g, u) {
                    return;
                }
            }
        }

        // (0.41k: AI_pickup(UNITAI_WORKER) is commented out in the C++.)
        if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_spySeaMove()` (CvUnitAI.cpp:6381-6435)
    pub fn unit_ai_spy_sea_move(&self, g: &mut Game, u: UnitId) {
        if g.unit_get_unit_ai_cargo(u, UnitAIType::UNITAI_SPY) > 0 {
            if self.unit_ai_special_sea_transport_spy(g, u) {
                return;
            }
            if !g.units.contains(u) {
                return;
            }

            let plot = g.unit_plot(u);
            if let Some(city) = g.map.plots[plot].plot_city {
                if g.cities[city].owner == g.units[u].owner {
                    let city_plot = g.city_plot(city);
                    let grp = gr(g, u);
                    g.group_push_mission(
                        grp,
                        MissionType::MISSION_SKIP,
                        -1,
                        -1,
                        0,
                        false,
                        false,
                        Some(MissionAIType::MISSIONAI_ATTACK_SPY),
                        Some(city_plot),
                        None,
                    );
                    return;
                }
            }
        } else if !g.group_has_cargo(gr(g, u)) {
            if self.unit_ai_pillage_range(g, u, 5, 0) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_pickup(g, u, UnitAIType::UNITAI_SPY) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_carrierSeaMove()` (CvUnitAI.cpp:6438-6524)
    pub fn unit_ai_carrier_sea_move(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_heal(g, u, 50, MAX_INT) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        let plot = g.unit_plot(u);
        if !g.unit_is_enemy(u, g.plot_get_team(plot), None) {
            if self.player_ai_unit_target_mission_ais(g, p, u, MissionAIType::MISSIONAI_GROUP, None) > 0 {
                push_u(g, u, MissionType::MISSION_SKIP);
                return;
            }
        } else {
            if self.unit_ai_sea_bombard_range(g, u, 1) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_group(g, u, UnitAIType::UNITAI_CARRIER_SEA, -1, 1, -1, false, false, false, MAX_INT, false) {
            return;
        }

        let grp = gr(g, u);
        if g.group_count_num_unit_ai_type(grp, Some(UnitAIType::UNITAI_ATTACK_SEA))
            + g.group_count_num_unit_ai_type(grp, Some(UnitAIType::UNITAI_ESCORT_SEA))
            == 0
        {
            if in_own_city(g, u) {
                push_u(g, u, MissionType::MISSION_SKIP);
                return;
            }
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if g.units[u].cargo > 0 {
            if self.unit_ai_carrier_sea_transport(g, u) {
                return;
            }

            if self.unit_ai_blockade(g, u) {
                return;
            }

            if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_ASSAULT_SEA, -1, -1, true) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_travel_to_upgrade_city(g, u) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, true, false, MAX_INT) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_missileCarrierSeaMove()` (CvUnitAI.cpp:6527-6581)
    pub fn unit_ai_missile_carrier_sea_move(&self, g: &mut Game, u: UnitId) {
        let is_stealth = g.unit_get_invisible_type(u).is_some();

        if self.unit_ai_sea_retreat_from_city_danger(g, u) {
            return;
        }

        let team = g.unit_get_team(u);
        let plot = g.unit_plot(u);
        if plot_is_city(g, plot) && g.plot_get_team(plot) == Some(team) {
            if self.unit_ai_heal(g, u, 0, MAX_INT) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let plot = g.unit_plot(u);
        let grp = gr(g, u);
        if (g.plot_get_team(plot) != Some(team) && g.group_has_cargo(grp)) || self.group_ai_is_full(g, grp) {
            if is_stealth {
                if self.unit_ai_carrier_sea_transport(g, u) {
                    return;
                }
            } else {
                if self.unit_ai_shadow(g, u, UnitAIType::UNITAI_ASSAULT_SEA, 1, 50, true) {
                    return;
                }

                if self.unit_ai_carrier_sea_transport(g, u) {
                    return;
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        // (AI_pickup(UNITAI_ICBM) / AI_pickup(UNITAI_MISSILE_AIR) are commented out in the C++.)
        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_attackAirMove()` (CvUnitAI.cpp:6584-6686)
    pub fn unit_ai_attack_air_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_air_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_air_attack_damaged_skip(g, u) {
            return;
        }

        if g.units[u].damage > 0 {
            if self.unit_ai_air_bomb_plots(g, u) {
                return;
            }
            if self.unit_ai_air_strike(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let p = g.units[u].owner;
        let unit_type = g.units[u].unit_type;
        let area = g.unit_area(u);
        let attack_value = self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_ATTACK_AIR, area);
        let carrier_value = self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_CARRIER_AIR, area);
        if carrier_value > 0 {
            let carriers = self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_CARRIER_SEA);
            if carriers > 0 {
                // C++ `kPlayer.AI_bestAreaUnitAIValue(UNITAI_CARRIER_SEA, NULL, &eBestCarrierUnit)`.
                let best_carrier_unit = self.best_unit_ai_unit_any_city(g, p, UnitAIType::UNITAI_CARRIER_SEA, area);
                if let Some(best_carrier_unit) = best_carrier_unit {
                    let carrier_air_needed = carriers * g.d.units[best_carrier_unit].cargo_space;
                    if self.player_ai_total_unit_ais(g, p, UnitAIType::UNITAI_CARRIER_AIR) < carrier_air_needed {
                        self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_CARRIER_AIR);
                        push_u(g, u, MissionType::MISSION_SKIP);
                        return;
                    }
                }
            }
        }

        let defense_value = self.player_ai_unit_value(g, p, unit_type, UnitAIType::UNITAI_DEFENSE_AIR, area);
        if defense_value > attack_value {
            if self.player_ai_best_area_unit_ai_value(g, p, UnitAIType::UNITAI_ATTACK_AIR, area, &mut None) > attack_value {
                self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_DEFENSE_AIR);
                push_u(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        if self.unit_ai_air_bomb_defenses(g, u) {
            return;
        }

        if self.rand(4, "AI Air Attack Move") == 0 {
            if self.unit_ai_air_bomb_plots(g, u) {
                return;
            }
        }

        if self.unit_ai_air_strike(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_can_air_attack(u) {
            if self.unit_ai_air_offensive_city(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_can_recon(u, Some(g.unit_plot(u))) {
            if self.unit_ai_explore_air(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_can_air_defend(u, None) {
            push_u(g, u, MissionType::MISSION_AIRPATROL);
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_defenseAirMove()` (CvUnitAI.cpp:6689-6765)
    pub fn unit_ai_defense_air_move(&self, g: &mut Game, u: UnitId) {
        if self.unit_ai_air_retreat_from_city_danger(g, u) {
            return;
        }

        if self.unit_ai_air_attack_damaged_skip(g, u) {
            return;
        }

        if self.rand(2, "AI Air Defense Move") == 0 {
            let plot = g.unit_plot(u);
            let city = g.map.plots[plot].plot_city;

            if city.is_some() && self.city_ai_is_danger(g, city.unwrap()) {
                if self.unit_ai_air_strike(g, u) {
                    return;
                }
            } else {
                if self.unit_ai_air_bomb_defenses(g, u) {
                    return;
                }

                if self.unit_ai_air_strike(g, u) {
                    return;
                }

                if !g.units.contains(u) {
                    return;
                }

                if self.unit_ai_get_birthmark(g, u) % 2 == 0 {
                    if self.unit_ai_air_bomb_plots(g, u) {
                        return;
                    }
                }
            }

            if !g.units.contains(u) {
                return;
            }

            if self.unit_ai_travel_to_upgrade_city(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        let no_war = g.team_get_at_war_count(g.unit_get_team(u), false) == 0;

        if g.unit_can_recon(u, Some(g.unit_plot(u))) {
            if self.rand(if no_war { 2 } else { 4 }, "AI defensive air recon") == 0 {
                if self.unit_ai_explore_air(g, u) {
                    return;
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_air_defensive_city(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_can_air_defend(u, None) {
            push_u(g, u, MissionType::MISSION_AIRPATROL);
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_carrierAirMove()` (CvUnitAI.cpp:6768-6848)
    pub fn unit_ai_carrier_air_move(&self, g: &mut Game, u: UnitId) {
        // XXX maybe protect land troops?

        if g.units[u].damage > 0 {
            push_u(g, u, MissionType::MISSION_SKIP);
            return;
        }

        if g.unit_is_cargo(u) {
            let rand = self.rand(3, "AI Air Carrier Move");

            if rand == 2 && g.unit_can_air_defend(u, None) {
                push_u(g, u, MissionType::MISSION_AIRPATROL);
                return;
            } else if self.unit_ai_air_bomb_defenses(g, u) {
                return;
            } else if rand == 1 {
                if self.unit_ai_air_bomb_plots(g, u) {
                    return;
                }

                if self.unit_ai_air_strike(g, u) {
                    return;
                }
            } else {
                if self.unit_ai_air_strike(g, u) {
                    return;
                }

                if self.unit_ai_air_bomb_plots(g, u) {
                    return;
                }
            }

            if !g.units.contains(u) {
                return;
            }

            if self.unit_ai_travel_to_upgrade_city(g, u) {
                return;
            }

            if !g.units.contains(u) {
                return;
            }

            if g.unit_can_air_defend(u, None) {
                push_u(g, u, MissionType::MISSION_AIRPATROL);
                return;
            }
            push_u(g, u, MissionType::MISSION_SKIP);
            return;
        }

        if self.unit_ai_air_carrier(g, u) {
            return;
        }

        if self.unit_ai_air_defensive_city(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_can_air_defend(u, None) {
            push_u(g, u, MissionType::MISSION_AIRPATROL);
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_missileAirMove()` (CvUnitAI.cpp:6851-6950)
    pub fn unit_ai_missile_air_move(&self, g: &mut Game, u: UnitId) {
        let plot = g.unit_plot(u);
        if let Some(city) = g.map.plots[plot].plot_city {
            if self.city_ai_is_danger(g, city) {
                if !self.city_ai_is_defended(g, city, 0) {
                    if self.unit_ai_air_offensive_city(g, u) {
                        return;
                    }
                }
            }
        }

        if !g.units.contains(u) {
            return;
        }

        if g.unit_is_cargo(u) {
            let rand = self.rand(3, "AI Air Missile plot bombing");
            if rand != 0 {
                if self.unit_ai_air_bomb_plots(g, u) {
                    return;
                }
            }

            let rand = self.rand(3, "AI Air Missile Carrier Move");
            if rand == 0 {
                if self.unit_ai_air_bomb_defenses(g, u) {
                    return;
                }

                if self.unit_ai_air_strike(g, u) {
                    return;
                }
            } else {
                if self.unit_ai_air_strike(g, u) {
                    return;
                }

                if self.unit_ai_air_bomb_defenses(g, u) {
                    return;
                }
            }

            if self.unit_ai_air_bomb_plots(g, u) {
                return;
            }

            if !g.units.contains(u) {
                return;
            }

            push_u(g, u, MissionType::MISSION_SKIP);
            return;
        }

        if self.unit_ai_air_strike(g, u) {
            return;
        }

        if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_MISSILE_CARRIER_SEA, -1, false) {
            return;
        }

        if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_RESERVE_SEA, 1, false) {
            return;
        }

        if self.unit_ai_missile_load(g, u, UnitAIType::UNITAI_ATTACK_SEA, 1, false) {
            return;
        }

        if self.unit_ai_air_bomb_defenses(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        if !g.unit_is_cargo(u) {
            if self.unit_ai_air_offensive_city(g, u) {
                return;
            }
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_networkAutomated()` (CvUnitAI.cpp:6953-7025)
    pub fn unit_ai_network_automated(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if !g.group_can_defend(gr(g, u)) {
            if self.player_ai_get_plot_danger(g, p, g.unit_plot(u), -1, true) > 0 {
                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    // XXX maybe not do this??? could be working productively somewhere else...
                    return;
                }
            }
        }

        if self.unit_ai_improve_bonus(g, u, 20, &mut None, &mut None, &mut 0) {
            return;
        }

        if self.unit_ai_improve_bonus(g, u, 10, &mut None, &mut None, &mut 0) {
            return;
        }

        if self.unit_ai_connect_bonus(g, u, true) {
            return;
        }

        if self.unit_ai_connect_city(g, u) {
            return;
        }

        if self.unit_ai_improve_bonus(g, u, 0, &mut None, &mut None, &mut 0) {
            return;
        }

        if self.unit_ai_route_territory(g, u, true) {
            return;
        }

        if self.unit_ai_connect_bonus(g, u, false) {
            return;
        }

        if self.unit_ai_route_city(g, u) {
            return;
        }

        if self.unit_ai_route_territory(g, u, false) {
            return;
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_cityAutomated()` (CvUnitAI.cpp:7028-7075)
    pub fn unit_ai_city_automated(&self, g: &mut Game, u: UnitId) {
        let p = g.units[u].owner;

        if !g.group_can_defend(gr(g, u)) {
            if self.player_ai_get_plot_danger(g, p, g.unit_plot(u), -1, true) > 0 {
                if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                    // XXX maybe not do this??? could be working productively somewhere else...
                    return;
                }
            }
        }

        let mut city = None;

        let plot = g.unit_plot(u);
        if g.map.plots[plot].owner == Some(p) {
            city = g.map.plots[plot].working_city;
        }

        if city.is_none() {
            // XXX do team???
            city = g.map_find_city(g.units[u].x, g.units[u].y, Some(p), None, true, false, None, None, None);
        }

        if let Some(city) = city {
            if self.unit_ai_improve_city(g, u, city) {
                return;
            }
        }

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }

        if !g.units.contains(u) {
            return;
        }

        push_u(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `int CvUnitAI::AI_promotionValue(PromotionTypes ePromotion)` (CvUnitAI.cpp:7079-7839)
    // XXX make sure we include any new UnitAITypes...
    pub fn unit_ai_promotion_value(&self, g: &Game, u: UnitId, promotion: PromotionType) -> i32 {
        use UnitAIType as A;

        let pi = &g.d.promotions[promotion];
        let unit = &g.units[u];
        let unit_info = &g.d.units[unit.unit_type];
        let ai = unit.ai_type;
        let is = |t: UnitAIType| ai == Some(t);
        let plot = g.unit_plot(u);

        let mut value = 0;
        let mut temp;
        let mut extra;

        if pi.leader {
            // Don't consume the leader as a regular promotion
            return 0;
        }

        if pi.blitz {
            if (is(A::UNITAI_RESERVE) && g.unit_base_moves(u) > 1) || is(A::UNITAI_PARADROP) {
                value += 10;
            } else {
                //FfH: Modified by Kael 06/28/2008
                value += 3 * g.unit_base_moves(u);
            }
        }

        if pi.amphib {
            if is(A::UNITAI_ATTACK) || is(A::UNITAI_ATTACK_CITY) {
                value += 5;
            } else {
                value += 1;
            }
        }

        if pi.river {
            if is(A::UNITAI_ATTACK) || is(A::UNITAI_ATTACK_CITY) {
                value += 5;
            } else {
                value += 1;
            }
        }

        if pi.enemy_route {
            if is(A::UNITAI_PILLAGE) {
                value += 40;
            } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_ATTACK_CITY) {
                value += 20;
            } else if is(A::UNITAI_PARADROP) {
                value += 10;
            } else {
                value += 4;
            }
        }

        if pi.always_heal {
            if is(A::UNITAI_ATTACK)
                || is(A::UNITAI_ATTACK_CITY)
                || is(A::UNITAI_PILLAGE)
                || is(A::UNITAI_COUNTER)
                || is(A::UNITAI_ATTACK_SEA)
                || is(A::UNITAI_PIRATE_SEA)
                || is(A::UNITAI_ESCORT_SEA)
                || is(A::UNITAI_PARADROP)
            {
                value += 10;
            } else {
                value += 8;
            }
        }

        if pi.hills_double_move {
            if is(A::UNITAI_EXPLORE) {
                value += 20;
            } else {
                value += 10;
            }
        }

        if pi.immune_to_first_strikes && !g.unit_immune_to_first_strikes(u) {
            if is(A::UNITAI_ATTACK_CITY) {
                value += 12;
            } else if is(A::UNITAI_ATTACK) {
                value += 8;
            } else {
                value += 4;
            }
        }

        temp = pi.visibility_change;
        if is(A::UNITAI_EXPLORE_SEA) || is(A::UNITAI_EXPLORE) {
            value += temp * 40;
        } else if is(A::UNITAI_PIRATE_SEA) {
            value += temp * 20;
        }

        temp = pi.moves_change;
        if is(A::UNITAI_ATTACK_SEA)
            || is(A::UNITAI_PIRATE_SEA)
            || is(A::UNITAI_RESERVE_SEA)
            || is(A::UNITAI_ESCORT_SEA)
            || is(A::UNITAI_EXPLORE_SEA)
            || is(A::UNITAI_ASSAULT_SEA)
            || is(A::UNITAI_SETTLER_SEA)
            || is(A::UNITAI_PILLAGE)
            || is(A::UNITAI_ATTACK)
            || is(A::UNITAI_PARADROP)
        {
            value += temp * 20;
        } else {
            value += temp * 4;
        }

        temp = pi.move_discount_change;
        if is(A::UNITAI_PILLAGE) {
            value += temp * 10;
        } else {
            value += temp * 2;
        }

        temp = pi.air_range_change;
        if is(A::UNITAI_ATTACK_AIR) || is(A::UNITAI_CARRIER_AIR) {
            value += temp * 20;
        } else if is(A::UNITAI_DEFENSE_AIR) {
            value += temp * 10;
        }

        temp = pi.intercept_change;
        if is(A::UNITAI_DEFENSE_AIR) {
            value += temp * 3;
        } else if is(A::UNITAI_CITY_SPECIAL) || is(A::UNITAI_CARRIER_AIR) {
            value += temp * 2;
        } else {
            value += temp / 10;
        }

        temp = pi.evasion_change;
        if is(A::UNITAI_ATTACK_AIR) || is(A::UNITAI_CARRIER_AIR) {
            value += temp * 3;
        } else {
            value += temp / 10;
        }

        temp = pi.first_strikes_change * 2;
        temp += pi.chance_first_strikes_change;
        if is(A::UNITAI_RESERVE)
            || is(A::UNITAI_COUNTER)
            || is(A::UNITAI_CITY_DEFENSE)
            || is(A::UNITAI_CITY_COUNTER)
            || is(A::UNITAI_CITY_SPECIAL)
            || is(A::UNITAI_ATTACK)
        {
            temp *= 8;
            extra = unit.extra_chance_first_strikes + unit.extra_first_strikes * 2;
            temp *= 100 + extra * 15;
            temp /= 100;
            value += temp;
        } else {
            value += temp * 5;
        }

        temp = pi.withdrawal_change;
        if temp != 0 {
            extra = unit_info.withdrawal_probability + unit.extra_withdrawal * 4;
            temp *= 100 + extra;
            temp /= 100;
            if is(A::UNITAI_ATTACK_CITY) {
                value += (temp * 4) / 3;
            } else if is(A::UNITAI_COLLATERAL) || is(A::UNITAI_RESERVE) || is(A::UNITAI_RESERVE_SEA) || unit.leader_unit_type.is_some() {
                value += temp * 1;
            } else {
                value += temp / 4;
            }
        }

        temp = pi.collateral_damage_change;
        if temp != 0 {
            extra = unit.extra_collateral_damage; //collateral has no strong synergy (not like retreat)
            temp *= 100 + extra;
            temp /= 100;

            if is(A::UNITAI_COLLATERAL) {
                value += temp * 1;
            } else if is(A::UNITAI_ATTACK_CITY) {
                value += (temp * 2) / 3;
            } else {
                value += temp / 8;
            }
        }

        temp = pi.bombard_rate_change;
        if is(A::UNITAI_ATTACK_CITY) {
            value += temp * 2;
        } else {
            value += temp / 8;
        }

        temp = pi.enemy_heal_change;
        if is(A::UNITAI_ATTACK) || is(A::UNITAI_ATTACK_SEA) || is(A::UNITAI_PARADROP) || is(A::UNITAI_PIRATE_SEA) {
            value += temp / 4;
        } else {
            value += temp / 8;
        }

        temp = pi.neutral_heal_change;
        value += temp / 8;

        temp = pi.friendly_heal_change;
        if is(A::UNITAI_CITY_DEFENSE) || is(A::UNITAI_CITY_COUNTER) || is(A::UNITAI_CITY_SPECIAL) {
            value += temp / 4;
        } else {
            value += temp / 8;
        }

        if unit.damage > 0 {
            let group_units = unit.group.map_or(0, |grp| num_units(g, grp));

            temp = pi.same_tile_heal_change + unit.same_tile_heal;
            extra = unit.same_tile_heal;

            temp *= 100 + extra * 5;
            temp /= 100;

            if temp > 0 {
                if g.unit_heal_rate(u, plot) < temp {
                    value += temp * if group_units > 4 { 4 } else { 2 };
                } else {
                    value += temp / 8;
                }
            }

            temp = pi.adjacent_tile_heal_change;
            extra = unit.adjacent_tile_heal;
            temp *= 100 + extra * 5;
            temp /= 100;
            if unit.same_tile_heal >= temp {
                value += temp * if group_units > 9 { 4 } else { 2 };
            } else {
                value += temp / 4;
            }
        }

        //FfH: Modified by Kael 11/14/2009 0.41k (the Warlord super-medic bonus is commented out)

        temp = pi.combat_percent;
        if is(A::UNITAI_ATTACK)
            || is(A::UNITAI_COUNTER)
            || is(A::UNITAI_CITY_COUNTER)
            || is(A::UNITAI_ATTACK_SEA)
            || is(A::UNITAI_RESERVE_SEA)
            || is(A::UNITAI_PARADROP)
            || is(A::UNITAI_PIRATE_SEA)
            || is(A::UNITAI_ESCORT_SEA)
            || is(A::UNITAI_CARRIER_SEA)
            || is(A::UNITAI_ATTACK_AIR)
            || is(A::UNITAI_CARRIER_AIR)
        {
            value += temp * 2;
        } else {
            value += temp * 1;
        }

        temp = pi.city_attack_percent;
        if temp != 0 {
            if unit_info.unit_ai_type[A::UNITAI_ATTACK]
                || unit_info.unit_ai_type[A::UNITAI_ATTACK_CITY]
                || unit_info.unit_ai_type[A::UNITAI_ATTACK_CITY_LEMMING]
            {
                extra = unit_info.city_attack_modifier + unit.extra_city_attack_percent * 2;
                temp *= 100 + extra;
                temp /= 100;
                if is(A::UNITAI_ATTACK_CITY) {
                    value += temp * 1;
                } else {
                    value -= temp / 4;
                }
            }
        }

        temp = pi.city_defense_percent;
        if temp != 0 {
            if is(A::UNITAI_CITY_DEFENSE) || is(A::UNITAI_CITY_SPECIAL) {
                extra = unit_info.city_defense_modifier + unit.extra_city_defense_percent * 2;
                value += (temp * (100 + extra)) / 100;
            } else {
                value += temp / 4;
            }
        }

        temp = pi.hills_attack_percent;
        if temp != 0 {
            extra = unit.extra_hills_attack_percent;
            temp *= 100 + extra * 2;
            temp /= 100;
            if is(A::UNITAI_ATTACK) || is(A::UNITAI_COUNTER) {
                value += temp / 4;
            } else {
                value += temp / 16;
            }
        }

        temp = pi.hills_defense_percent;
        if temp != 0 {
            extra = unit_info.hills_defense_modifier + unit.extra_hills_defense_percent * 2;
            temp *= 100 + extra;
            temp /= 100;
            if is(A::UNITAI_CITY_DEFENSE) {
                if plot_is_city(g, plot) && g.plot_is_hills(plot) {
                    value += (temp * 4) / 3;
                }
            } else if is(A::UNITAI_COUNTER) {
                if g.plot_is_hills(plot) {
                    value += temp / 4;
                } else {
                    value += 1;
                }
            } else {
                value += temp / 16;
            }
        }

        temp = pi.revolt_protection;
        if is(A::UNITAI_CITY_DEFENSE) || is(A::UNITAI_CITY_COUNTER) || is(A::UNITAI_CITY_SPECIAL) {
            if temp > 0 {
                let owner = g.plot_calculate_cultural_owner(plot);
                if let Some(owner) = owner {
                    if g.players[owner].team != g.players[unit.owner].team {
                        value += temp / 2;
                    }
                }
            }
        }

        temp = pi.collateral_damage_protection;
        if is(A::UNITAI_CITY_DEFENSE) || is(A::UNITAI_CITY_COUNTER) || is(A::UNITAI_CITY_SPECIAL) {
            value += temp / 3;
        } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_COUNTER) {
            value += temp / 4;
        } else {
            value += temp / 8;
        }

        temp = pi.pillage_change;
        if is(A::UNITAI_PILLAGE) || is(A::UNITAI_ATTACK_SEA) || is(A::UNITAI_PIRATE_SEA) {
            value += temp / 4;
        } else {
            value += temp / 16;
        }

        temp = pi.upgrade_discount;
        value += temp / 16;

        temp = pi.experience_percent;
        if is(A::UNITAI_ATTACK)
            || is(A::UNITAI_ATTACK_SEA)
            || is(A::UNITAI_PIRATE_SEA)
            || is(A::UNITAI_RESERVE_SEA)
            || is(A::UNITAI_ESCORT_SEA)
            || is(A::UNITAI_CARRIER_SEA)
            || is(A::UNITAI_MISSILE_CARRIER_SEA)
        {
            value += temp * 1;
        } else {
            value += temp / 2;
        }

        temp = pi.kamikaze_percent;
        if is(A::UNITAI_ATTACK_CITY) {
            value += temp / 16;
        } else {
            value += temp / 64;
        }

        let plot_terrain = g.map.plots[plot].terrain_type;
        for terrain in g.d.terrains.keys() {
            temp = pi.terrain_attack_percent[terrain];
            if temp != 0 {
                extra = unit.extra_terrain_attack_percent[terrain];
                temp *= 100 + extra * 2;
                temp /= 100;
                if is(A::UNITAI_ATTACK) || is(A::UNITAI_COUNTER) {
                    value += temp / 4;
                } else {
                    value += temp / 16;
                }
            }

            temp = pi.terrain_defense_percent[terrain];
            if temp != 0 {
                extra = unit.extra_terrain_defense_percent[terrain];
                temp *= 100 + extra;
                temp /= 100;
                if is(A::UNITAI_COUNTER) {
                    if plot_terrain == Some(terrain) {
                        value += temp / 4;
                    } else {
                        value += 1;
                    }
                } else {
                    value += temp / 16;
                }
            }

            if pi.terrain_double_move[terrain] {
                if is(A::UNITAI_EXPLORE) {
                    value += 20;
                } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_PILLAGE) {
                    value += 10;
                } else {
                    value += 1;
                }
            }
        }

        let plot_feature = g.map.plots[plot].feature_type;
        let no_defensive_bonus = g.unit_no_defensive_bonus(u);
        for feature in g.d.features.keys() {
            temp = pi.feature_attack_percent[feature];
            if temp != 0 {
                extra = unit.extra_feature_attack_percent[feature];
                temp *= 100 + extra * 2;
                temp /= 100;
                if is(A::UNITAI_ATTACK) || is(A::UNITAI_COUNTER) {
                    value += temp / 4;
                } else {
                    value += temp / 16;
                }
            }

            temp = pi.feature_defense_percent[feature];
            if temp != 0 {
                extra = unit.extra_feature_defense_percent[feature];
                temp *= 100 + extra * 2;
                temp /= 100;

                if !no_defensive_bonus {
                    if is(A::UNITAI_COUNTER) {
                        if plot_feature == Some(feature) {
                            value += temp / 4;
                        } else {
                            value += 1;
                        }
                    } else {
                        value += temp / 16;
                    }
                }
            }

            if pi.feature_double_move[feature] {
                if is(A::UNITAI_EXPLORE) {
                    value += 20;
                } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_PILLAGE) {
                    value += 10;
                } else {
                    value += 1;
                }
            }
        }

        let own_combat = g.unit_get_unit_combat_type(u);
        let mut other_combat = 0;
        let mut same_combat = 0;

        for combat in g.d.unit_combats.keys() {
            if Some(combat) == own_combat {
                same_combat += g.unit_unit_combat_modifier(u, combat);
            } else {
                other_combat += g.unit_unit_combat_modifier(u, combat);
            }
        }

        for combat in g.d.unit_combats.keys() {
            temp = pi.unit_combat_modifier_percent[combat];
            let mut combat_weight;
            //Fighting their own kind
            if Some(combat) == own_combat {
                if same_combat >= other_combat {
                    combat_weight = 70; //"axeman takes formation"
                } else {
                    combat_weight = 30;
                }
            } else {
                //fighting other kinds
                if g.unit_unit_combat_modifier(u, combat) > 10 {
                    combat_weight = 70; //"spearman takes formation"
                } else {
                    combat_weight = 30;
                }
            }

            combat_weight *= self.player_ai_get_unit_combat_weight(g, unit.owner, combat);
            combat_weight /= 100;

            if is(A::UNITAI_COUNTER) || is(A::UNITAI_CITY_COUNTER) {
                value += (temp * combat_weight) / 50;
            } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_RESERVE) {
                value += (temp * combat_weight) / 100;
            } else {
                value += (temp * combat_weight) / 200;
            }
        }

        for i in 0..DomainType::NUM {
            //WTF? why float and cast to int?
            temp = pi.domain_modifier_percent.get(DomainType::from_usize(i)).copied().unwrap_or(0);
            if is(A::UNITAI_COUNTER) {
                value += temp * 1;
            } else if is(A::UNITAI_ATTACK) || is(A::UNITAI_RESERVE) {
                value += temp / 2;
            } else {
                value += temp / 8;
            }
        }

        //FfH: Added by Kael 07/30/2007
        temp = pi.defensive_strike_chance + pi.defensive_strike_damage;
        temp /= 2;
        if is(A::UNITAI_CITY_DEFENSE) || is(A::UNITAI_CITY_COUNTER) || is(A::UNITAI_COUNTER) {
            temp *= 2;
        }
        value += temp;
        value += pi.ai_weight;
        //FfH: End Add

        if value > 0 {
            value += self.rand(15, "AI Promote");
        }

        value
    }

    /// C++ `bool CvUnitAI::AI_shadow(UnitAITypes eUnitAI, int iMax = -1, int iMaxRatio = -1, bool bWithCargoOnly = true)` (CvUnitAI.cpp:7843-7920)
    // Returns true if a mission was pushed...
    pub fn unit_ai_shadow(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max: i32, max_ratio: i32, with_cargo_only: bool) -> bool {
        let p = g.units[u].owner;
        let grp = gr(g, u);
        let our_base_moves = g.group_base_moves(grp);

        let mut best_value = 0;
        let mut best_unit: Option<UnitId> = None;

        for loop_unit in g.players[p].units.clone() {
            if loop_unit == u {
                continue;
            }
            let Some(loop_plot) = g.unit_plot_opt(loop_unit) else {
                continue;
            };
            if !self.unit_ai_plot_valid(g, u, loop_plot) {
                continue;
            }
            if !g.unit_is_group_head(loop_unit) {
                continue;
            }
            if g.unit_is_cargo(loop_unit) {
                continue;
            }
            if g.units[loop_unit].ai_type != Some(unit_ai) {
                continue;
            }
            let Some(loop_group) = g.units[loop_unit].group else {
                continue;
            };
            if g.group_base_moves(loop_group) > our_base_moves {
                continue;
            }
            if with_cargo_only && !g.group_has_cargo(loop_group) {
                continue;
            }
            let shadower_count = self.player_ai_unit_target_mission_ais(g, p, loop_unit, MissionAIType::MISSIONAI_SHADOW, Some(grp));
            if ((-1 == max) || (shadower_count < max))
                && ((-1 == max_ratio)
                    || (shadower_count == 0)
                    || (((100 * shadower_count) / 1.max(g.group_count_num_unit_ai_type(loop_group, Some(unit_ai)))) <= max_ratio))
            {
                if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                    let mut path_turns = 0;
                    if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                        //if (iPathTurns <= iMaxPath) XXX
                        let mut value = 1 + g.group_get_cargo(loop_group);
                        value *= 1000;
                        value /= 1i32.wrapping_add(path_turns);

                        if value > best_value {
                            best_value = value;
                            best_unit = Some(loop_unit);
                        }
                    }
                }
            }
        }

        if let Some(best_unit) = best_unit {
            let best_plot = g.unit_plot(best_unit);
            if g.unit_at_plot(u, best_plot) {
                g.group_push_mission(
                    grp,
                    MissionType::MISSION_SKIP,
                    -1,
                    -1,
                    0,
                    false,
                    false,
                    Some(MissionAIType::MISSIONAI_SHADOW),
                    None,
                    Some(best_unit),
                );
                return true;
            } else {
                let best_owner = g.units[best_unit].owner;
                g.group_push_mission(
                    grp,
                    MissionType::MISSION_MOVE_TO_UNIT,
                    best_owner.i(),
                    best_unit.i(),
                    0,
                    false,
                    false,
                    Some(MissionAIType::MISSIONAI_SHADOW),
                    None,
                    Some(best_unit),
                );
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_group(UnitAITypes eUnitAI, int iMaxGroup = -1, int iMaxOwnUnitAI = -1, int iMinUnitAI = -1, bool bIgnoreFaster = false, bool bIgnoreOwnUnitType = false, bool bStackOfDoom = false, int iMaxPath = MAX_INT, bool bAllowRegrouping = false)` (CvUnitAI.cpp:7924-8059)
    // Returns true if a group was joined or a mission was pushed...
    pub fn unit_ai_group(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max_group: i32, max_own_unit_ai: i32, min_unit_ai: i32, ignore_faster: bool, ignore_own_unit_type: bool, stack_of_doom: bool, max_path: i32, allow_regrouping: bool) -> bool {
        // if we are on a transport, then do not regroup
        if g.unit_is_cargo(u) {
            return false;
        }

        let grp = gr(g, u);
        if !allow_regrouping {
            if num_units(g, grp) > 1 {
                return false;
            }
        }

        let p = g.units[u].owner;
        if g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && !g.unit_can_move_all_terrain(u) {
            let area = g.unit_area(u);
            if g.map.areas[area].num_ai_units[p][unit_ai] == 0 {
                return false;
            }
        }

        if !self.unit_ai_can_group_with_ai_type(g, u, unit_ai) {
            return false;
        }

        let our_plot = g.unit_plot(u);
        let (our_x, our_y) = g.map.xy(our_plot);
        let our_ai = g.units[u].ai_type;
        let our_unit_type = g.units[u].unit_type;
        let our_base_moves = g.unit_base_moves(u);

        let mut best_value = MAX_INT;
        let mut best_unit: Option<UnitId> = None;

        for loop_unit in g.players[p].units.clone() {
            let Some(loop_group) = g.units[loop_unit].group else {
                continue;
            };
            let Some(loop_plot) = g.unit_plot_opt(loop_unit) else {
                continue;
            };
            if !self.unit_ai_plot_valid(g, u, loop_plot) {
                continue;
            }
            if !(max_path > 0 || loop_plot == our_plot) {
                continue;
            }
            if g.unit_is_enemy(u, g.plot_get_team(loop_plot), None) {
                continue;
            }
            if !self.unit_ai_allow_group(g, u, loop_unit, unit_ai) {
                continue;
            }
            let sod_extra = if stack_of_doom { self.unit_ai_stack_of_doom_extra(g, u) } else { 0 };
            if !((max_group == -1)
                || ((num_units(g, loop_group) + self.player_ai_unit_target_mission_ais(g, p, loop_unit, MissionAIType::MISSIONAI_GROUP, Some(grp)))
                    <= (max_group + sod_extra)))
            {
                continue;
            }
            if !((max_own_unit_ai == -1) || (g.group_count_num_unit_ai_type(loop_group, our_ai) <= (max_own_unit_ai + sod_extra))) {
                continue;
            }
            if !((min_unit_ai == -1) || (g.group_count_num_unit_ai_type(loop_group, Some(unit_ai)) >= min_unit_ai)) {
                continue;
            }
            if ignore_faster && g.group_base_moves(loop_group) > our_base_moves {
                continue;
            }
            if ignore_own_unit_type && g.units[loop_unit].unit_type == our_unit_type {
                continue;
            }
            if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                continue;
            }

            // SPEED TWEAK Sephi: no path search beyond the approximate maximum distance.
            let (loop_x, loop_y) = g.map.xy(loop_plot);
            let x_dist = loop_x - our_x;
            let y_dist = loop_y - our_y;
            // C++ quirk: iMaxPath*iMaxPath*4 overflows for the default MAX_INT (to 4), so the
            // default search only reaches adjacent plots.
            if (x_dist * x_dist) + (y_dist * y_dist) < max_path.wrapping_mul(max_path).wrapping_mul(4) {
                let mut path_turns = 0;
                if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                    if path_turns <= max_path {
                        let mut value = 1000i32.wrapping_mul(path_turns.wrapping_add(1));
                        value = value.wrapping_mul(4 + g.group_get_cargo(loop_group));
                        value /= num_units(g, loop_group).max(1);
                        if value < best_value {
                            best_value = value;
                            best_unit = Some(loop_unit);
                        }
                    }
                }
            }
        }

        if let Some(best_unit) = best_unit {
            let best_plot = g.unit_plot(best_unit);
            if g.unit_at_plot(u, best_plot) {
                let best_group = g.units[best_unit].group;
                g.unit_join_group(u, best_group, true);
                return true;
            } else {
                let best_owner = g.units[best_unit].owner;
                g.group_push_mission(
                    grp,
                    MissionType::MISSION_MOVE_TO_UNIT,
                    best_owner.i(),
                    best_unit.i(),
                    0,
                    false,
                    false,
                    Some(MissionAIType::MISSIONAI_GROUP),
                    None,
                    Some(best_unit),
                );
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_groupMergeRange(UnitAITypes eUnitAI, int iMaxRange, bool bBiggerOnly = true, bool bAllowRegrouping = false, bool bIgnoreFaster = false)` (CvUnitAI.cpp:8061-8187)
    pub fn unit_ai_group_merge_range(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, max_range: i32, bigger_only: bool, allow_regrouping: bool, ignore_faster: bool) -> bool {
        // if we are on a transport, then do not regroup
        if g.unit_is_cargo(u) {
            return false;
        }

        // cached values
        let plot = g.unit_plot(u);
        let group = gr(g, u);

        if !allow_regrouping {
            if num_units(g, group) > 1 {
                return false;
            }
        }

        let p = g.units[u].owner;
        if g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && !g.unit_can_move_all_terrain(u) {
            let area = g.unit_area(u);
            if g.map.areas[area].num_ai_units[p][unit_ai] == 0 {
                return false;
            }
        }

        if !self.unit_ai_can_group_with_ai_type(g, u, unit_ai) {
            return false;
        }

        let (x, y) = g.map.xy(plot);
        let plot_area = g.map.plots[plot].area;
        let our_base_moves = g.unit_base_moves(u);
        let our_num_units = num_units(g, group);

        // best match
        let mut best_unit: Option<UnitId> = None;
        let mut best_value = MAX_INT;
        // iterate over plots at each range
        for dx in -max_range..=max_range {
            for dy in -max_range..=max_range {
                let Some(loop_plot) = g.plot_xy(x, y, dx, dy) else {
                    continue;
                };
                if g.map.plots[loop_plot].area != plot_area {
                    continue;
                }

                for loop_unit in g.map.plots[loop_plot].units.clone() {
                    let Some(loop_group) = g.units[loop_unit].group else {
                        continue;
                    };

                    if self.unit_ai_allow_group(g, u, loop_unit, unit_ai) {
                        if !ignore_faster || g.group_base_moves(loop_group) <= our_base_moves {
                            if !bigger_only || num_units(g, loop_group) >= our_num_units {
                                // SPEED TWEAK Sephi: no path search beyond the approximate maximum distance.
                                let (loop_x, loop_y) = g.map.xy(loop_plot);
                                let x_dist = loop_x - x;
                                let y_dist = loop_y - y;
                                if (x_dist * x_dist) + (y_dist * y_dist) < (max_range + 2) * (max_range + 2) * 4 {
                                    let mut path_turns = 0;
                                    if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                        if path_turns <= max_range + 2 {
                                            let mut value = 1000 * (path_turns + 1);
                                            value /= num_units(g, loop_group).max(1);
                                            if value < best_value {
                                                best_value = value;
                                                best_unit = Some(loop_unit);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(best_unit) = best_unit {
            let best_plot = g.unit_plot(best_unit);
            if g.unit_at_plot(u, best_plot) {
                let best_group = gr(g, best_unit);
                g.group_merge_into_group(group, best_group);
                return true;
            } else {
                let best_owner = g.units[best_unit].owner;
                g.group_push_mission(
                    group,
                    MissionType::MISSION_MOVE_TO_UNIT,
                    best_owner.i(),
                    best_unit.i(),
                    0,
                    false,
                    false,
                    Some(MissionAIType::MISSIONAI_GROUP),
                    None,
                    Some(best_unit),
                );
                return true;
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_load(UnitAITypes eUnitAI, MissionAITypes eMissionAI, UnitAITypes eTransportedUnitAI = NO_UNITAI, int iMinCargo = -1, int iMinCargoSpace = -1, int iMaxCargoSpace = -1, int iMaxCargoOurUnitAI = -1, int iFlags = 0, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:8190-8364)
    // Returns true if we loaded onto a transport or a mission was pushed...
    pub fn unit_ai_load(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType, mission_ai: MissionAIType, transported_unit_ai: Option<UnitAIType>, min_cargo: i32, min_cargo_space: i32, max_cargo_space: i32, max_cargo_our_unit_ai: i32, flags: i32, max_path: i32) -> bool {
        // XXX what to do about groups??? (the C++ group size check is commented out)

        if g.units[u].cargo > 0 {
            return false;
        }

        if g.unit_is_cargo(u) {
            push_u(g, u, MissionType::MISSION_SKIP);
            return true;
        }

        let p = g.units[u].owner;
        let team = g.unit_get_team(u);
        let domain = g.unit_get_domain_type(u);
        if domain == DomainType::DOMAIN_LAND && !g.unit_can_move_all_terrain(u) {
            let area = g.unit_area(u);
            if g.map.areas[area].num_ai_units[p][unit_ai] == 0 {
                return false;
            }
        }

        // do not load transports if we are already in a land war
        let area_ai = area_ai_of(g, u);
        let land_war = area_ai == Some(AreaAIType::AREAAI_OFFENSIVE)
            || area_ai == Some(AreaAIType::AREAAI_DEFENSIVE)
            || area_ai == Some(AreaAIType::AREAAI_MASSING);
        if !g.unit_is_barbarian(u) && land_war && mission_ai != MissionAIType::MISSIONAI_LOAD_SETTLER {
            return false;
        }

        let mut best_value = MAX_INT;
        let mut best_unit: Option<UnitId> = None;

        let load_mission_ai = [
            MissionAIType::MISSIONAI_LOAD_ASSAULT,
            MissionAIType::MISSIONAI_LOAD_SETTLER,
            MissionAIType::MISSIONAI_LOAD_SPECIAL,
            MissionAIType::MISSIONAI_ATTACK_SPY,
        ];

        let grp = gr(g, u);
        let current_group_size = num_units(g, grp);
        let special_unit_type = g.unit_get_special_unit_type(u);
        let our_ai = ai_of(g, u);
        let our_plot = g.unit_plot(u);
        let (our_x, our_y) = g.map.xy(our_plot);
        let head_unit_ai = g.group_get_head_unit_ai(grp);

        for loop_unit in g.players[p].units.clone() {
            if loop_unit == u {
                continue;
            }
            let Some(loop_plot) = g.unit_plot_opt(loop_unit) else {
                continue;
            };
            if !self.unit_ai_plot_valid(g, u, loop_plot) {
                continue;
            }
            if !g.unit_can_load_unit(u, loop_unit, loop_plot) {
                continue;
            }
            // special case ASSAULT_SEA UnitAI, so that, if a unit is marked escort, but can load units, it will load them
            // transport units might have been built as escort, this most commonly happens with galleons
            let loop_unit_ai = g.units[loop_unit].ai_type;
            if loop_unit_ai != Some(unit_ai) {
                continue;
            }
            let mut cargo_space_available = g.unit_cargo_space_available(loop_unit, special_unit_type, Some(domain));
            cargo_space_available -= self.player_ai_unit_target_mission_ais_2(g, p, loop_unit, &load_mission_ai, Some(grp));
            if cargo_space_available <= 0 {
                continue;
            }
            if !(transported_unit_ai.is_none() || g.unit_get_unit_ai_cargo(loop_unit, transported_unit_ai.unwrap()) > 0) {
                continue;
            }
            if !((min_cargo == -1) || (g.units[loop_unit].cargo >= min_cargo)) {
                continue;
            }
            if !((min_cargo_space == -1) || (g.unit_cargo_space_available(loop_unit, None, None) >= min_cargo_space)) {
                continue;
            }
            if !((max_cargo_space == -1) || (g.unit_cargo_space_available(loop_unit, None, None) <= max_cargo_space)) {
                continue;
            }
            if !((max_cargo_our_unit_ai == -1) || (g.unit_get_unit_ai_cargo(loop_unit, our_ai) <= max_cargo_our_unit_ai)) {
                continue;
            }
            if !(head_unit_ai != Some(UnitAIType::UNITAI_CITY_DEFENSE)
                || !plot_is_city(g, our_plot)
                || (g.plot_get_team(our_plot) != Some(team)))
            {
                continue;
            }
            if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                continue;
            }
            let loop_group = gr(g, loop_unit);
            let unit_target_plot = g.group_ai_get_mission_ai_plot(loop_group);
            let target_ok = match unit_target_plot {
                None => true,
                Some(t) => {
                    g.plot_get_team(t) == Some(team)
                        || (!g.plot_is_owned(t) || !g.unit_is_potential_enemy(u, g.plot_get_team(t), Some(t)))
                }
            };
            if !target_ok {
                continue;
            }

            // SPEED TWEAK Sephi: no path search beyond the approximate maximum distance.
            let (loop_x, loop_y) = g.map.xy(loop_plot);
            let x_dist = loop_x - our_x;
            let y_dist = loop_y - our_y;
            // C++ quirk: iMaxPath*iMaxPath*4 overflows for the default MAX_INT (to 4), so the
            // default search only reaches adjacent plots.
            if (x_dist * x_dist) + (y_dist * y_dist) < max_path.wrapping_mul(max_path).wrapping_mul(4) {
                let mut path_turns = 0;
                if g.unit_generate_path(u, loop_plot, flags, true, Some(&mut path_turns)) {
                    if path_turns <= max_path {
                        // prefer a transport that can hold as much of our group as possible
                        let value = (0.max(current_group_size - cargo_space_available) * 5).wrapping_add(path_turns);
                        if value < best_value {
                            best_value = value;
                            best_unit = Some(loop_unit);
                        }
                    }
                }
            }
        }

        if let Some(best_unit) = best_unit {
            let best_plot = g.unit_plot(best_unit);
            if g.unit_at_plot(u, best_plot) {
                // XXX is this dangerous (not pushing a mission...) XXX air units?
                g.group_set_transport_unit(grp, Some(best_unit));
                return true;
            } else {
                let cargo_space_available = g.unit_cargo_space_available(best_unit, special_unit_type, Some(domain));

                // split our group to fit on the transport
                let (split_group, other_group) = g.group_split_group(grp, cargo_space_available, Some(u));

                if let Some(split_group) = split_group {
                    let old_plot = g.group_plot(split_group);
                    let best_owner = g.units[best_unit].owner;
                    g.group_push_mission(
                        split_group,
                        MissionType::MISSION_MOVE_TO_UNIT,
                        best_owner.i(),
                        best_unit.i(),
                        flags,
                        false,
                        false,
                        Some(mission_ai),
                        None,
                        Some(best_unit),
                    );
                    // C++ `pSplitGroup->plot() != pOldPlot` (the group is gone if its units died).
                    let new_plot = if g.groups.contains(split_group) { g.group_plot(split_group) } else { None };
                    let moved = new_plot != old_plot;
                    if !moved && g.units.contains(u) {
                        if let Some(other_group) = other_group.filter(|&o| g.groups.contains(o)) {
                            g.unit_join_group(u, Some(other_group), true);
                        }
                    }
                    return moved;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardCityBestDefender()` (CvUnitAI.cpp:8368-8389)
    // Returns true if a mission was pushed...
    pub fn unit_ai_guard_city_best_defender(&self, g: &mut Game, u: UnitId) -> bool {
        let plot = g.unit_plot(u);
        let owner = g.units[u].owner;

        if let Some(city) = g.map.plots[plot].plot_city {
            if g.cities[city].owner == owner {
                if g.plot_get_best_defender(plot, Some(owner), None, None, false, false, false) == Some(u) {
                    let grp = gr(g, u);
                    g.group_push_mission(
                        grp,
                        MissionType::MISSION_SKIP,
                        -1,
                        -1,
                        0,
                        false,
                        false,
                        Some(MissionAIType::MISSIONAI_GUARD_CITY),
                        None,
                        None,
                    );
                    return true;
                }
            }
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_guardCityMinDefender(bool bSearch = true)` (CvUnitAI.cpp:8391-8506)
    pub fn unit_ai_guard_city_min_defender(&self, g: &mut Game, u: UnitId, search: bool) -> bool {
        let owner = g.units[u].owner;
        let plot = g.unit_plot(u);
        let city_defense = UnitAIType::UNITAI_CITY_DEFENSE.i();

        if let Some(plot_city) = g.map.plots[plot].plot_city {
            if g.cities[plot_city].owner == owner {
                let city_plot = g.city_plot(plot_city);
                let city_defender_count =
                    g.plot_plot_count(city_plot, Some(Game::puf_is_unit_ai_type), city_defense, -1, Some(owner), None, None, -1, -1);
                if (city_defender_count - 1) < self.city_ai_min_defenders(g, plot_city) {
                    if (city_defender_count <= 2) || (self.rand(5, "AI shuffle defender") != 0) {
                        let grp = gr(g, u);
                        g.group_push_mission(
                            grp,
                            MissionType::MISSION_SKIP,
                            -1,
                            -1,
                            0,
                            false,
                            false,
                            Some(MissionAIType::MISSIONAI_GUARD_CITY),
                            None,
                            None,
                        );
                        return true;
                    }
                }
            }
        }

        if search {
            let mut best_value = 0;
            let mut best_plot: Option<PlotIdx> = None;
            let mut best_guard_plot: Option<PlotIdx> = None;

            let grp = gr(g, u);
            let (our_x, our_y) = g.map.xy(plot);
            let current_turn = g.game_turn();
            for loop_city in g.players[owner].cities.clone() {
                let loop_city_plot = g.city_plot(loop_city);
                if !self.unit_ai_plot_valid(g, u, loop_city_plot) {
                    continue;
                }
                let mut defenders_have =
                    g.plot_plot_count(loop_city_plot, Some(Game::puf_is_unit_ai_type), city_defense, -1, Some(owner), None, None, -1, -1);
                let defenders_need = self.city_ai_min_defenders(g, loop_city);
                if defenders_have < defenders_need {
                    if !g.plot_is_visible_enemy_unit_for_unit(loop_city_plot, u) {
                        defenders_have +=
                            self.player_ai_plot_target_mission_ais(g, owner, loop_city_plot, MissionAIType::MISSIONAI_GUARD_CITY, Some(grp), 0);
                        if defenders_have < defenders_need + 1 {
                            // SPEED TWEAK Sephi: no path search beyond the approximate maximum distance.
                            let (city_x, city_y) = g.map.xy(loop_city_plot);
                            let x_dist = city_x - our_x;
                            let y_dist = city_y - our_y;
                            if (x_dist * x_dist) + (y_dist * y_dist) < 200 {
                                let mut path_turns = 0;
                                if !g.unit_at_plot(u, loop_city_plot) && g.unit_generate_path(u, loop_city_plot, 0, true, Some(&mut path_turns)) {
                                    if path_turns <= 10 {
                                        let mut value = (defenders_need - defenders_have) * 20;
                                        value += 2 * 15.min(current_turn - g.cities[loop_city].game_turn_acquired);
                                        if g.city_is_occupation(loop_city) {
                                            value += 5;
                                        }
                                        value -= path_turns;
                                        if value > best_value {
                                            best_value = value;
                                            best_plot = g.unit_get_path_end_turn_plot(u);
                                            best_guard_plot = Some(loop_city_plot);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(best_plot) = best_plot {
                let best_guard_plot = best_guard_plot.expect("AI_guardCityMinDefender: best plot without a guard plot");
                if g.unit_at_plot(u, best_guard_plot) {
                    g.group_push_mission(
                        grp,
                        MissionType::MISSION_SKIP,
                        -1,
                        -1,
                        0,
                        false,
                        false,
                        Some(MissionAIType::MISSIONAI_GUARD_CITY),
                        None,
                        None,
                    );
                    return true;
                }
                let (best_x, best_y) = g.map.xy(best_plot);
                g.group_push_mission(
                    grp,
                    MissionType::MISSION_MOVE_TO,
                    best_x,
                    best_y,
                    0,
                    false,
                    false,
                    Some(MissionAIType::MISSIONAI_GUARD_CITY),
                    Some(best_guard_plot),
                    None,
                );
                return true;
            }
        }

        false
    }
}
