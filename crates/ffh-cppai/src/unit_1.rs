//! Port of CvUnitAI.cpp:41-2953 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES (for the integrator):
//! - `unit_ai_improve_local_plot(.., ignore_city: CityId)` and
//!   `unit_ai_next_city_to_improve(.., city: CityId)` (unit_6.rs): C++ passes NULL
//!   (`AI_improveLocalPlot(3, NULL)`, and `pCity` may be NULL in `AI_workerMove`), and
//!   `AI_improveLocalPlot` tests `NULL == pIgnoreCity`. Both should take `Option<CityId>`
//!   (only this file calls them). Until then this file passes [`NULL_CITY`] (an id no city
//!   has) for NULL: drop the `.unwrap_or(NULL_CITY)` at the call sites once they take an Option.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// Stand-in for a NULL `CvCity*` where a callee takes a plain `CityId` (see SIGNATURE NOTES).
const NULL_CITY: CityId = CityId(u32::MAX);

/// C++ `getGroup()` of a unit (always set for a live unit).
fn grp(g: &Game, u: UnitId) -> GroupId {
    g.units[u].group.expect("CvUnitAI: unit without a selection group")
}

/// C++ `getGroup()->pushMission(eMission)` (all other arguments at their defaults).
fn push_mission(g: &mut Game, u: UnitId, mission: MissionType) {
    let group = grp(g, u);
    g.group_push_mission(group, mission, -1, -1, 0, false, false, None, None, None);
}

/// C++ `getGroup()->pushMission(MISSION_MOVE_TO, iX, iY, iFlags)`.
fn push_move_to(g: &mut Game, u: UnitId, plot: PlotIdx, flags: i32) {
    let group = grp(g, u);
    let (x, y) = g.map.xy(plot);
    g.group_push_mission(group, MissionType::MISSION_MOVE_TO, x, y, flags, false, false, None, None, None);
}

/// C++ `getPathLastNode()->m_iData1` (moves left at the end of the last generated path).
fn path_last_node_data1(g: &Game) -> i32 {
    g.path_finder.borrow().last_node().map_or(0, |n| n.data1)
}

/// C++ `getTransportUnit()` (NULL once the transport is gone).
fn transport_unit(g: &Game, u: UnitId) -> Option<UnitId> {
    g.units[u].transport_unit.filter(|&t| g.units.contains(t))
}

/// C++ `pPlot->getArea() == getArea()`.
fn same_area(g: &Game, u: UnitId, plot: PlotIdx) -> bool {
    g.map.plots[plot].area == g.map.plots[g.unit_plot(u)].area
}

impl Cai {
    /// C++ `void CvUnitAI::AI_init(UnitAITypes eUnitAI)` (CvUnitAI.cpp:41-52)
    pub fn unit_ai_init(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType) {
        // ffh-core's `unit_ai_init` is `AI_reset(eUnitAI)` for the members the rules keep, the
        // birthmark draw (`AI_setBirthmark(getSorenRandNum(10000, "AI Unit Birthmark"))`) and the
        // area/player AI-unit counts.
        g.unit_ai_init(u, unit_ai);

        // The rest of AI_reset: the members only the AI keeps.
        let mut s = self.s();
        let e = s.unit(u);
        e.permanent_summon = false;
        e.automated_abort_turn = -1;
    }

    /// C++ `void CvUnitAI::AI_reset(UnitAITypes eUnitAI = NO_UNITAI)` (CvUnitAI.cpp:60-87)
    pub fn unit_ai_reset(&self, g: &mut Game, u: UnitId, unit_ai: Option<UnitAIType>) {
        {
            let unit = &mut g.units[u];
            unit.ai_birthmark = 0;

            // BETTER AI (New Functions Definition) Sephi
            unit.ai_groupflag = GROUPFLAG_NONE;
            unit.ai_origin_x = -1;
            unit.ai_origin_y = -1;
            unit.ai_suicide_summon = false;

            unit.ai_allowed_perm_defense = true;
            unit.ai_allowed_patrol = true;
            unit.ai_allowed_explore = true;

            unit.ai_type = unit_ai;
        }

        let mut s = self.s();
        let e = s.unit(u);
        e.permanent_summon = false;
        e.automated_abort_turn = -1;
    }

    /// C++ `bool CvUnitAI::AI_update()` (CvUnitAI.cpp:90-732)
    ///
    /// Returns true when the caller should abort the loop and wait until the next slice.
    pub fn unit_ai_update(&self, g: &mut Game, u: UnitId) -> bool {
        let owner = g.units[u].owner;

        // allow python to handle it
        //FfH: Modified by Kael 10/02/2008 (only barbarians go through AI_unitUpdate)
        if g.unit_is_barbarian(u) {
            if self.py_ai_unit_update(g, u) {
                return false;
            }
            if !g.units.contains(u) {
                return false;
            }
        }

        // BETTER AI Sephi (Time for the Mages to Caste Haste, etc.)
        if !g.players[owner].human {
            for loop_unit in g.groups[grp(g, u)].units.clone() {
                if !g.units.contains(loop_unit) {
                    continue;
                }
                if self.unit_ai_is_movement_caster(g, loop_unit) {
                    self.unit_ai_movement_cast(g, loop_unit);
                }
            }
            if !g.units.contains(u) {
                return false;
            }
        }

        //FfH: Added by Kael 12/22/2007
        let plot = g.unit_plot(u);
        let (px, py) = g.map.xy(plot);
        let mut valid = false;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(loop_plot) = g.plot_xy(px, py, dx, dy) {
                    if g.unit_can_move_or_attack_into(u, loop_plot, false) {
                        valid = true;
                    }
                }
            }
        }
        if !valid {
            push_mission(g, u, MissionType::MISSION_SKIP);
            return false;
        }
        //FfH: End Add

        if g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND {
            if g.plot_is_water(g.unit_plot(u)) && !g.unit_can_move_all_terrain(u) {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return false;
            } else if let Some(transport) = transport_unit(g, u) {
                if let Some(tg) = g.units[transport].group.filter(|&tg| g.groups.contains(tg)) {
                    if g.group_has_moved(tg) || !g.groups[tg].mission_queue.is_empty() {
                        push_mission(g, u, MissionType::MISSION_SKIP);
                        return false;
                    }
                }
            }
        }

        if self.unit_ai_after_attack(g, u) {
            return false;
        }
        if !g.units.contains(u) {
            return false;
        }

        //FfH: Added by Kael 10/26/2008
        if !g.unit_is_barbarian(u) {
            if g.units[u].level < 2 {
                let mut does_build = false;
                let civ = g.unit_get_civilization_type(u);
                let unit_type = g.units[u].unit_type;
                for bc in g.d.building_classes.keys() {
                    if let Some(building) = g.d.civ_building(civ, bc) {
                        let info = &g.d.units[unit_type];
                        if info.force_buildings[building] || info.buildings[building] {
                            let here = g.unit_plot(u);
                            if g.unit_can_construct(u, here, Some(building), false) {
                                g.unit_construct(u, Some(building));
                                return false;
                            }
                            does_build = true;
                        }
                    }
                }
                if does_build {
                    if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
                        return false;
                    }
                    if !g.units.contains(u) {
                        return false;
                    }
                }
            }
        }

        // BUGFIX (also AI Units can become Enraged) Sephi
        if g.unit_is_ai_control(u) {
            if g.groups[grp(g, u)].units.len() > 1 {
                g.unit_join_group(u, None, true);
                return true;
            }
            //remove AI control from Defensive only Units
            else if g.unit_is_only_defensive(u) {
                g.unit_change_ai_control(u, -1);
                push_mission(g, u, MissionType::MISSION_SKIP);
                return false;
            } else if g.unit_is_ai_control(u) {
                if self.unit_ai_any_attack(g, u, 3, 0, 0, false) {
                    return false;
                }
                if !g.units.contains(u) {
                    return false;
                }
                if self.unit_ai_any_attack(g, u, 10, 0, 0, false) {
                    return false;
                }
                if !g.units.contains(u) {
                    return false;
                }

                self.unit_ai_barb_attack_move(g, u);
                return false;
            }
        }
        //FfH: End Add

        let group = grp(g, u);
        if g.group_is_automated(group) {
            match g.groups[group].automate_type {
                Some(AutomateType::AUTOMATE_BUILD) => {
                    if g.units[u].ai_type == Some(UnitAIType::UNITAI_WORKER) {
                        self.unit_ai_worker_move(g, u);
                    } else if g.units[u].ai_type == Some(UnitAIType::UNITAI_WORKER_SEA) {
                        self.unit_ai_worker_sea_move(g, u);
                    }
                }

                Some(AutomateType::AUTOMATE_NETWORK) => {
                    self.unit_ai_network_automated(g, u);
                    // XXX else wake up???
                }

                Some(AutomateType::AUTOMATE_CITY) => {
                    self.unit_ai_city_automated(g, u);
                    // XXX else wake up???
                }

                Some(AutomateType::AUTOMATE_EXPLORE) => {
                    match g.unit_get_domain_type(u) {
                        DomainType::DOMAIN_SEA => self.unit_ai_explore_sea_move(g, u),

                        DomainType::DOMAIN_AIR => {
                            // if we are cargo (on a carrier), hold if the carrier is not done moving yet
                            if let Some(transport) = transport_unit(g, u) {
                                if g.unit_is_automated(transport)
                                    && g.unit_can_move(transport)
                                    && g.units[transport]
                                        .group
                                        .map_or(true, |tg| g.groups[tg].activity_type != ActivityType::ACTIVITY_HOLD)
                                {
                                    push_mission(g, u, MissionType::MISSION_SKIP);
                                }
                            }
                        }

                        DomainType::DOMAIN_LAND => self.unit_ai_explore_move(g, u),

                        _ => {}
                    }

                    if !g.units.contains(u) {
                        return false;
                    }

                    // if we have air cargo (we are a carrier), and we done moving, explore with the aircraft as well
                    if g.unit_has_cargo(u)
                        && g.unit_domain_cargo(u) == Some(DomainType::DOMAIN_AIR)
                        && (!g.unit_can_move(u) || g.groups[grp(g, u)].activity_type == ActivityType::ACTIVITY_HOLD)
                    {
                        let cargo_units = g.unit_get_cargo_units(u);
                        for cargo in cargo_units {
                            if !g.units.contains(u) || !g.unit_is_automated(u) {
                                break;
                            }
                            if !g.units.contains(cargo) {
                                continue;
                            }
                            if g.unit_get_domain_type(cargo) == DomainType::DOMAIN_AIR {
                                if g.unit_can_move(cargo) {
                                    let cg = grp(g, cargo);
                                    g.group_set_automate_type(cg, Some(AutomateType::AUTOMATE_EXPLORE));
                                    g.group_set_activity_type(cg, ActivityType::ACTIVITY_AWAKE);
                                }
                            }
                        }
                    }
                }

                Some(AutomateType::AUTOMATE_RELIGION) => {
                    if g.units[u].ai_type == Some(UnitAIType::UNITAI_MISSIONARY) {
                        self.unit_ai_missionary_move(g, u);
                    }
                }

                // ADDON (automatic Terraforming) Sephi
                Some(AutomateType::AUTOMATE_TERRAFORMING) => {
                    self.unit_ai_set_unit_ai_type(g, u, UnitAIType::UNITAI_TERRAFORMER);
                    self.unit_ai_terraformer_move(g, u);
                }

                _ => {}
            }

            // if no longer automated, then we want to bail
            if !g.units.contains(u) {
                return false;
            }
            return !g.group_is_automated(grp(g, u));
        }

        // BETTER AI (UnitAI::AI_update) Sephi
        if !g.unit_is_barbarian(u) {
            match g.units[u].ai_groupflag {
                GROUPFLAG_CONQUEST => {
                    self.unit_ai_conquest_move(g, u);
                    return false;
                }
                GROUPFLAG_PERMDEFENSE => {
                    self.unit_ai_perm_defense_move(g, u);
                    return false;
                }
                GROUPFLAG_PERMDEFENSE_NEW => {
                    self.unit_ai_perm_defense_new_move(g, u);
                    return false;
                }
                GROUPFLAG_PATROL => {
                    self.unit_ai_patrol_move(g, u);
                    return false;
                }
                GROUPFLAG_PATROL_NEW => {
                    self.unit_ai_patrol_new_move(g, u);
                    return false;
                }
                GROUPFLAG_EXPLORE => {
                    self.unit_ai_explore_move(g, u);
                    return false;
                }
                GROUPFLAG_DEFENSE_NEW => {
                    self.unit_ai_barbsmashermove(g, u);
                    return false;
                }
                GROUPFLAG_HERO => {
                    self.unit_ai_heromove(g, u);
                    return false;
                }
                GROUPFLAG_SETTLERGROUP => {
                    self.unit_ai_settlergroup_move(g, u);
                    return false;
                }
                GROUPFLAG_HNGROUP => {
                    self.unit_ai_h_ngroup_move(g, u);
                    return false;
                }
                GROUPFLAG_FEASTING => {
                    self.unit_ai_feastingmove(g, u);
                    return false;
                }
                GROUPFLAG_NAVAL_SETTLE_NEW => {
                    self.unit_ai_settle_naval_new_move(g, u);
                    return false;
                }
                GROUPFLAG_NAVAL_SETTLE => {
                    self.unit_ai_settle_naval_move(g, u);
                    return false;
                }
                GROUPFLAG_NAVAL_SETTLE_PICKUP => {
                    self.unit_ai_settle_naval_pickup_move(g, u);
                    return false;
                }
                GROUPFLAG_ANIMAL_CONSTRUCTBUILDING => {
                    if self.unit_ai_construct(g, u, 10000, MAX_INT, 15) {
                        return false;
                    }
                    if !g.units.contains(u) {
                        return false;
                    }
                }
                _ => {}
            }

            if g.units[u].ai_suicide_summon {
                self.unit_ai_summon_attack_move(g, u);
                if g.units.contains(u) {
                    g.units[u].ai_suicide_summon = false;
                }
                return false;
            }
            match g.units[u].ai_type {
                Some(UnitAIType::UNITAI_UNKNOWN)
                | Some(UnitAIType::UNITAI_ANIMAL)
                | Some(UnitAIType::UNITAI_ATTACK)
                | Some(UnitAIType::UNITAI_WARWIZARD)
                | Some(UnitAIType::UNITAI_BARBSMASHER)
                | Some(UnitAIType::UNITAI_ATTACK_CITY)
                | Some(UnitAIType::UNITAI_COLLATERAL)
                | Some(UnitAIType::UNITAI_PILLAGE)
                | Some(UnitAIType::UNITAI_RESERVE)
                | Some(UnitAIType::UNITAI_COUNTER)
                | Some(UnitAIType::UNITAI_CITY_DEFENSE)
                | Some(UnitAIType::UNITAI_CITY_COUNTER) => {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return false;
                }
                _ => {}
            }
        }

        let Some(unit_ai) = g.units[u].ai_type else {
            return false;
        };
        match unit_ai {
            UnitAIType::UNITAI_UNKNOWN => push_mission(g, u, MissionType::MISSION_SKIP),

            UnitAIType::UNITAI_ANIMAL => self.unit_ai_animal_move(g, u),

            UnitAIType::UNITAI_SETTLE => self.unit_ai_settle_move(g, u),

            UnitAIType::UNITAI_WORKER => self.unit_ai_worker_move(g, u),

            UnitAIType::UNITAI_ATTACK => {
                //Added by Kael 09/19/2007
                if g.units[u].duration > 0 {
                    self.unit_ai_summon_attack_move(g, u);
                } else if g.unit_is_barbarian(u) {
                    self.unit_ai_barb_attack_move(g, u);
                } else {
                    self.unit_ai_attack_move(g, u);
                }
            }

            // BETTER AI (New UNITAI) Sephi
            UnitAIType::UNITAI_BARBSMASHER => self.unit_ai_barbsmashermove(g, u),
            UnitAIType::UNITAI_HERO => {
                if g.unit_is_barbarian(u) {
                    self.unit_ai_barb_attack_move(g, u);
                } else {
                    self.unit_ai_heromove(g, u);
                }
            }
            UnitAIType::UNITAI_FEASTING => self.unit_ai_feastingmove(g, u),
            UnitAIType::UNITAI_MANA_UPGRADE => self.unit_ai_upgrademana_move(g, u),
            UnitAIType::UNITAI_MAGE => self.unit_ai_mage_move(g, u),
            UnitAIType::UNITAI_TERRAFORMER => self.unit_ai_terraformer_move(g, u),
            UnitAIType::UNITAI_ATTACK_CITY => {
                if g.unit_is_barbarian(u) {
                    self.unit_ai_barb_attack_move(g, u);
                } else {
                    self.unit_ai_attack_city_move(g, u);
                }
            }

            UnitAIType::UNITAI_COLLATERAL => self.unit_ai_collateral_move(g, u),

            UnitAIType::UNITAI_PILLAGE => self.unit_ai_pillage_move(g, u),

            UnitAIType::UNITAI_RESERVE => self.unit_ai_reserve_move(g, u),

            UnitAIType::UNITAI_MEDIC | UnitAIType::UNITAI_COUNTER => self.unit_ai_counter_move(g, u),

            UnitAIType::UNITAI_PARADROP => self.unit_ai_paratrooper_move(g, u),

            UnitAIType::UNITAI_CITY_DEFENSE => self.unit_ai_city_defense_move(g, u),

            UnitAIType::UNITAI_CITY_COUNTER | UnitAIType::UNITAI_CITY_SPECIAL => self.unit_ai_city_defense_extra_move(g, u),

            UnitAIType::UNITAI_EXPLORE => self.unit_ai_explore_move(g, u),

            UnitAIType::UNITAI_MISSIONARY => self.unit_ai_missionary_move(g, u),

            UnitAIType::UNITAI_PROPHET => self.unit_ai_prophet_move(g, u),

            UnitAIType::UNITAI_ARTIST => self.unit_ai_artist_move(g, u),

            UnitAIType::UNITAI_SCIENTIST => self.unit_ai_scientist_move(g, u),

            UnitAIType::UNITAI_GENERAL => self.unit_ai_general_move(g, u),

            UnitAIType::UNITAI_MERCHANT => self.unit_ai_merchant_move(g, u),

            UnitAIType::UNITAI_ENGINEER => self.unit_ai_engineer_move(g, u),

            UnitAIType::UNITAI_SPY => self.unit_ai_spy_move(g, u),

            UnitAIType::UNITAI_ICBM => self.unit_ai_icbm_move(g, u),

            UnitAIType::UNITAI_WORKER_SEA => self.unit_ai_worker_sea_move(g, u),

            UnitAIType::UNITAI_ATTACK_SEA => {
                if g.unit_is_barbarian(u) {
                    self.unit_ai_barb_attack_sea_move(g, u);
                } else {
                    self.unit_ai_attack_sea_move(g, u);
                }
            }

            UnitAIType::UNITAI_RESERVE_SEA => self.unit_ai_reserve_sea_move(g, u),

            UnitAIType::UNITAI_ESCORT_SEA => self.unit_ai_escort_sea_move(g, u),

            UnitAIType::UNITAI_EXPLORE_SEA => self.unit_ai_explore_sea_move(g, u),

            UnitAIType::UNITAI_ASSAULT_SEA => self.unit_ai_assault_sea_move(g, u),

            UnitAIType::UNITAI_SETTLER_SEA => self.unit_ai_settler_sea_move(g, u),

            UnitAIType::UNITAI_MISSIONARY_SEA => self.unit_ai_missionary_sea_move(g, u),

            UnitAIType::UNITAI_SPY_SEA => self.unit_ai_spy_sea_move(g, u),

            UnitAIType::UNITAI_CARRIER_SEA => self.unit_ai_carrier_sea_move(g, u),

            UnitAIType::UNITAI_MISSILE_CARRIER_SEA => self.unit_ai_missile_carrier_sea_move(g, u),

            UnitAIType::UNITAI_PIRATE_SEA => self.unit_ai_pirate_sea_move(g, u),

            UnitAIType::UNITAI_ATTACK_AIR => self.unit_ai_attack_air_move(g, u),

            UnitAIType::UNITAI_DEFENSE_AIR => self.unit_ai_defense_air_move(g, u),

            UnitAIType::UNITAI_CARRIER_AIR => self.unit_ai_carrier_air_move(g, u),

            UnitAIType::UNITAI_MISSILE_AIR => self.unit_ai_missile_air_move(g, u),

            UnitAIType::UNITAI_ATTACK_CITY_LEMMING => self.unit_ai_attack_city_lemming_move(g, u),

            // C++: default: FAssert(false) (UNITAI_WARWIZARD of barbarians lands here).
            _ => {}
        }

        false
    }

    /// C++ `bool CvUnitAI::AI_follow()` (CvUnitAI.cpp:736-774)
    ///
    /// Returns true if took an action or should wait to move later.
    pub fn unit_ai_follow(&self, g: &mut Game, u: UnitId) -> bool {
        if self.unit_ai_follow_bombard(g, u) {
            return true;
        }
        if !g.units.contains(u) {
            return true;
        }

        if self.unit_ai_city_attack(g, u, 1, 65, true) {
            return true;
        }
        if !g.units.contains(u) {
            return true;
        }

        let plot = g.unit_plot(u);
        if g.unit_is_enemy(u, g.plot_get_team(plot), None) {
            if g.unit_can_pillage(u, plot) {
                push_mission(g, u, MissionType::MISSION_PILLAGE);
                return true;
            }
        }

        if self.unit_ai_any_attack(g, u, 1, 70, 2, true) {
            return true;
        }
        if !g.units.contains(u) {
            return true;
        }

        if g.unit_is_found(u) {
            let owner = g.units[u].owner;
            let area = g.unit_area(u);
            if g.map.areas[area].best_found_value[owner] > 0 {
                if self.unit_ai_found_range(g, u, FOUND_RANGE, true) {
                    return true;
                }
            }
        }

        false
    }

    /// C++ `void CvUnitAI::AI_upgrade()` (CvUnitAI.cpp:779-825)
    pub fn unit_ai_upgrade(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;
        let Some(unit_ai) = g.units[u].ai_type else {
            return;
        };
        let area = g.unit_area(u);

        let current_value = self.player_ai_unit_value(g, owner, g.units[u].unit_type, unit_ai, area);

        for pass in 0..2 {
            let mut best_value = 0;
            let mut best_unit: Option<UnitType> = None;

            for i in g.d.units.keys() {
                if pass > 0 || g.d.units[i].unit_ai_type[unit_ai] {
                    let new_value = self.player_ai_unit_value(g, owner, i, unit_ai, area);
                    if (pass == 0 || new_value > 0) && new_value > current_value {
                        if g.unit_can_upgrade(u, Some(i), false) {
                            let value = 1 + self.rand(10000, "AI Upgrade");

                            if value > best_value {
                                best_value = value;
                                best_unit = Some(i);
                            }
                        }
                    }
                }
            }

            if best_unit.is_some() {
                g.unit_upgrade(u, best_unit);
                if g.units.contains(u) {
                    g.unit_do_delayed_death(u);
                }
                return;
            }
        }
    }

    /// C++ `void CvUnitAI::AI_promote()` (CvUnitAI.cpp:828-895)
    pub fn unit_ai_promote(&self, g: &mut Game, u: UnitId) {
        // BETTER AI (Better Promotion selection) Sephi
        //some units should wait to promote
        if !g.unit_is_barbarian(u) {
            //only temporary
            let ai = g.units[u].ai_type;
            if ai == Some(UnitAIType::UNITAI_MAGE) || ai == Some(UnitAIType::UNITAI_WARWIZARD) {
                let best_promotion = self.py_ai_mage_promotion(g, u);
                if best_promotion.is_some() {
                    g.unit_promote(u, best_promotion, None);
                    return;
                }
            }
        }

        if self.unit_ai_promote_magic_unit(g, u) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        let mut best_value = 0;
        let mut best_promotion: Option<PromotionType> = None;

        for i in g.d.promotions.keys() {
            if g.unit_can_promote(u, Some(i), None) {
                let value = self.unit_ai_promotion_value(g, u, i);

                if value > best_value {
                    best_value = value;
                    best_promotion = Some(i);
                }
            }
        }

        if best_promotion.is_some() {
            g.unit_promote(u, best_promotion, None);
            if g.units.contains(u) {
                self.unit_ai_promote(g, u);
            }
        }
    }

    /// C++ `int CvUnitAI::AI_groupFirstVal()` (CvUnitAI.cpp:898-1116)
    pub fn unit_ai_group_first_val(&self, g: &Game, u: UnitId) -> i32 {
        // BETTER AI (improved logic which unit becomes head of a group) Sephi
        if g.units[u].duration > 0 {
            return 1;
        }

        if g.units[u].ai_groupflag == GROUPFLAG_CONQUEST {
            return 25;
        }

        let Some(unit_ai) = g.units[u].ai_type else {
            return 0;
        };
        match unit_ai {
            UnitAIType::UNITAI_UNKNOWN | UnitAIType::UNITAI_ANIMAL => 0,

            UnitAIType::UNITAI_SETTLE => 21,

            UnitAIType::UNITAI_WORKER => 20,

            UnitAIType::UNITAI_ATTACK => {
                if g.unit_collateral_damage(u) > 0 {
                    17
                } else if g.unit_withdrawal_probability(u) > 0 {
                    15
                } else {
                    13
                }
            }

            UnitAIType::UNITAI_ATTACK_CITY => {
                if g.unit_bombard_rate(u) > 0 {
                    19
                } else if g.unit_collateral_damage(u) > 0 {
                    18
                } else if g.unit_withdrawal_probability(u) > 0 {
                    16
                } else {
                    14
                }
            }

            UnitAIType::UNITAI_COLLATERAL => 7,

            UnitAIType::UNITAI_PILLAGE => 12,

            UnitAIType::UNITAI_RESERVE => 6,

            UnitAIType::UNITAI_COUNTER => 5,

            UnitAIType::UNITAI_CITY_DEFENSE => 3,

            UnitAIType::UNITAI_CITY_COUNTER => 2,

            UnitAIType::UNITAI_CITY_SPECIAL
            | UnitAIType::UNITAI_MAGE
            | UnitAIType::UNITAI_TERRAFORMER
            | UnitAIType::UNITAI_MANA_UPGRADE
            | UnitAIType::UNITAI_WARWIZARD => 3,

            UnitAIType::UNITAI_BARBSMASHER | UnitAIType::UNITAI_FEASTING | UnitAIType::UNITAI_MEDIC => 3,
            UnitAIType::UNITAI_HERO => 100, //Heroes don't like to get pushed around

            UnitAIType::UNITAI_PARADROP => 4,

            UnitAIType::UNITAI_EXPLORE => 8,

            UnitAIType::UNITAI_MISSIONARY => 10,

            UnitAIType::UNITAI_PROPHET
            | UnitAIType::UNITAI_ARTIST
            | UnitAIType::UNITAI_SCIENTIST
            | UnitAIType::UNITAI_GENERAL
            | UnitAIType::UNITAI_MERCHANT
            | UnitAIType::UNITAI_ENGINEER => 11,

            UnitAIType::UNITAI_SPY => 9,

            UnitAIType::UNITAI_ICBM => 0,

            UnitAIType::UNITAI_WORKER_SEA => 8,

            UnitAIType::UNITAI_ATTACK_SEA => 3,

            UnitAIType::UNITAI_RESERVE_SEA => 2,

            UnitAIType::UNITAI_ESCORT_SEA => 1,

            UnitAIType::UNITAI_EXPLORE_SEA => 5,

            UnitAIType::UNITAI_ASSAULT_SEA => 11,

            UnitAIType::UNITAI_SETTLER_SEA => 9,

            UnitAIType::UNITAI_MISSIONARY_SEA => 9,

            UnitAIType::UNITAI_SPY_SEA => 10,

            UnitAIType::UNITAI_CARRIER_SEA => 7,

            UnitAIType::UNITAI_MISSILE_CARRIER_SEA => 6,

            UnitAIType::UNITAI_PIRATE_SEA => 4,

            UnitAIType::UNITAI_ATTACK_AIR
            | UnitAIType::UNITAI_DEFENSE_AIR
            | UnitAIType::UNITAI_CARRIER_AIR
            | UnitAIType::UNITAI_MISSILE_AIR => 0,

            UnitAIType::UNITAI_ATTACK_CITY_LEMMING => 1,

            _ => 0,
        }
    }

    /// C++ `int CvUnitAI::AI_groupSecondVal()` (CvUnitAI.cpp:1119-1122)
    pub fn unit_ai_group_second_val(&self, g: &Game, u: UnitId) -> i32 {
        if g.unit_get_domain_type(u) == DomainType::DOMAIN_AIR {
            g.unit_air_base_combat_str(u)
        } else {
            g.unit_base_combat_str(u)
        }
    }

    /// C++ `int CvUnitAI::AI_attackOdds(const CvPlot* pPlot, bool bPotentialEnemy) const` (CvUnitAI.cpp:1127-1227)
    ///
    /// Attack odds out of 100 (the higher, the better), withdrawal odds included.
    pub fn unit_ai_attack_odds(&self, g: &Game, u: UnitId, plot: PlotIdx, potential_enemy: bool) -> i32 {
        let owner = g.units[u].owner;
        let Some(defender) = g.plot_get_best_defender(plot, None, Some(owner), Some(u), !potential_enemy, potential_enemy, false) else {
            return 100;
        };

        let air = g.unit_get_domain_type(u) == DomainType::DOMAIN_AIR;
        let mut our_strength = if air { g.unit_air_curr_combat_str(u, None) } else { g.unit_curr_combat_str(u, None, None, None) };
        let our_firepower = if air { our_strength } else { g.unit_curr_firepower(u, None, None) };

        if our_strength == 0 {
            return 1;
        }

        let mut their_strength = g.unit_curr_combat_str(defender, Some(plot), Some(u), None);
        let their_firepower = g.unit_curr_firepower(defender, Some(plot), Some(u));

        let base_odds = (100 * our_strength) / (our_strength + their_strength).max(1);
        if base_odds == 0 {
            return 1;
        }

        let strength_factor = (our_firepower + their_firepower + 1) / 2;

        let combat_damage = g.d.defines.combat_damage;
        let damage_to_us = 1.max((combat_damage * (their_firepower + strength_factor)) / (our_firepower + strength_factor).max(1));
        let damage_to_them = 1.max((combat_damage * (our_firepower + strength_factor)) / (their_firepower + strength_factor).max(1));

        let hit_limit_them = g.unit_max_hit_points(defender) - g.units[u].combat_limit;

        let mut needed_rounds_us = (0.max(g.unit_curr_hit_points(defender) - hit_limit_them) + damage_to_them - 1) / damage_to_them;
        let mut needed_rounds_them = (0.max(g.unit_curr_hit_points(u)) + damage_to_us - 1) / damage_to_us;

        if !air {
            // BETTER_BTS_AI_MOD 10/30/09 Mongoose & jdog5000 (From Mongoose SDK)
            if !g.unit_immune_to_first_strikes(defender) {
                needed_rounds_us -= ((base_odds * g.unit_first_strikes(u)) + ((base_odds * g.unit_chance_first_strikes(u)) / 2)) / 100;
            }
            if !g.unit_immune_to_first_strikes(u) {
                needed_rounds_them -= (((100 - base_odds) * g.unit_first_strikes(defender))
                    + (((100 - base_odds) * g.unit_chance_first_strikes(defender)) / 2))
                    / 100;
            }
            needed_rounds_us = 1.max(needed_rounds_us);
            needed_rounds_them = 1.max(needed_rounds_them);
        }

        let rounds_diff = needed_rounds_us - needed_rounds_them;
        if rounds_diff > 0 {
            their_strength = their_strength.wrapping_mul(1 + rounds_diff);
        } else {
            our_strength = our_strength.wrapping_mul(1 - rounds_diff);
        }

        let mut odds = (our_strength.wrapping_mul(100)) / (our_strength.wrapping_add(their_strength)).max(1);
        odds += ((100 - odds) * g.unit_withdrawal_probability(u)) / 100;
        // BETTER AI (attack odds) Sephi: orig `iOdds += AI_getAttackOddsChange()` removed.

        1.max(odds.min(99))
    }

    /// C++ `bool CvUnitAI::AI_bestCityBuild(CvCity* pCity, CvPlot** ppBestPlot = NULL, BuildTypes* peBestBuild = NULL, CvPlot* pIgnorePlot = NULL, CvUnit* pUnit = NULL)` (CvUnitAI.cpp:1231-1373)
    ///
    /// Returns true if the unit found a build for this city.
    pub fn unit_ai_best_city_build(&self, g: &mut Game, u: UnitId, city: CityId, best_plot: &mut Option<PlotIdx>, best_build: &mut Option<BuildType>, ignore_plot: Option<PlotIdx>, unit: Option<UnitId>) -> bool {
        let owner = g.units[u].owner;
        let group = grp(g, u);
        let ruins = opt_from_i32::<ImprovementType>(g.d.defines.ruins_improvement);
        let safe_automation = g.players[owner].options[PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION];
        let (cx, cy) = (g.cities[city].x, g.cities[city].y);

        let mut best_value = 0;
        let mut e_best_build: Option<BuildType> = None;
        let mut p_best_plot: Option<PlotIdx> = None;

        for pass in 0..2 {
            for i in 0..NUM_CITY_PLOTS {
                let Some(loop_plot) = g.plot_city(cx, cy, i) else {
                    continue;
                };
                if !self.unit_ai_plot_valid(g, u, loop_plot) {
                    continue;
                }
                if Some(loop_plot) == ignore_plot {
                    continue;
                }
                let improvement = g.map.plots[loop_plot].improvement_type;
                if !(improvement.is_none() || !(safe_automation && !(improvement == ruins))) {
                    continue;
                }
                let mut value = self.city_ai_get_best_build_value(g, city, i as i32);

                if value > best_value {
                    let Some(build) = self.city_ai_get_best_build(g, city, i as i32) else {
                        continue;
                    };
                    if pass == 0 {
                        best_value = value;
                        p_best_plot = Some(loop_plot);
                        e_best_build = Some(build);
                    } else if g.unit_can_build(u, loop_plot, build, false) {
                        if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                                // XXX take advantage of range (warning... this could lead to some units doing nothing...)
                                let mut max_workers = 1;
                                let last_data1 = path_last_node_data1(g);
                                if last_data1 == 0 {
                                    path_turns += 1;
                                } else if path_turns <= 1 {
                                    max_workers = self.unit_ai_calculate_plot_workers_needed(g, u, loop_plot, build);
                                }
                                if let Some(unit) = unit {
                                    if g.plot_is_city(g.unit_plot(unit), false, None) && path_turns == 1 && last_data1 > 0 {
                                        max_workers += 10;
                                    }
                                }
                                if self.player_ai_plot_target_mission_ais(g, owner, loop_plot, MissionAIType::MISSIONAI_BUILD, Some(group), 0) < max_workers {
                                    //XXX this could be improved greatly by
                                    //looking at the real build time and other factors
                                    //when deciding whether to stack.
                                    value /= path_turns.max(1);

                                    best_value = value;
                                    p_best_plot = Some(loop_plot);
                                    e_best_build = Some(build);
                                }
                            }
                        }
                    }
                }
            }

            if pass == 0 {
                if let (Some(build), Some(plot)) = (e_best_build, p_best_plot) {
                    let mut path_turns = 0;
                    if g.unit_generate_path(u, plot, 0, true, Some(&mut path_turns))
                        && g.unit_can_build(u, plot, build, false)
                        && !g.plot_is_visible_enemy_unit_for_unit(plot, u)
                    {
                        let mut max_workers = 1;
                        if let Some(unit) = unit {
                            if g.plot_is_city(g.unit_plot(unit), false, None) {
                                max_workers += 10;
                            }
                        }
                        if path_last_node_data1(g) == 0 {
                            path_turns += 1;
                        } else if path_turns <= 1 {
                            max_workers = self.unit_ai_calculate_plot_workers_needed(g, u, plot, build);
                        }
                        let worker_count = self.player_ai_plot_target_mission_ais(g, owner, plot, MissionAIType::MISSIONAI_BUILD, Some(group), 0);
                        if worker_count < max_workers {
                            //Good to go.
                            break;
                        }
                    }
                    e_best_build = None;
                    best_value = 0;
                }
            }
        }

        if let Some(build) = e_best_build {
            *best_plot = p_best_plot;
            *best_build = Some(build);
        }

        e_best_build.is_some()
    }

    /// C++ `bool CvUnitAI::AI_isCityAIType() const` (CvUnitAI.cpp:1376-1382)
    pub fn unit_ai_is_city_ai_type(&self, g: &Game, u: UnitId) -> bool {
        g.unit_ai_is_city_ai_type(u)
    }

    /// C++ `int CvUnitAI::AI_getBirthmark() const` (CvUnitAI.cpp:1385-1388)
    pub fn unit_ai_get_birthmark(&self, g: &Game, u: UnitId) -> i32 {
        g.units[u].ai_birthmark
    }

    /// C++ `void CvUnitAI::AI_setBirthmark(int iNewValue)` (CvUnitAI.cpp:1391-1422)
    pub fn unit_ai_set_birthmark(&self, g: &mut Game, u: UnitId, new_value: i32) {
        g.units[u].ai_birthmark = new_value;
        if g.units[u].ai_type == Some(UnitAIType::UNITAI_EXPLORE_SEA) {
            if g.circumnavigation_available() {
                let owner = g.units[u].owner;
                let mut birthmark = g.units[u].ai_birthmark;
                birthmark -= birthmark % 4;
                let mut explorer_count = g.players[owner].ai_num_ai_units[UnitAIType::UNITAI_EXPLORE_SEA];
                explorer_count += owner.i() % 4;
                if g.map.wrap_x {
                    if (explorer_count % 2) == 1 {
                        birthmark += 1;
                    }
                }
                if g.map.wrap_y {
                    if !g.map.wrap_x {
                        explorer_count *= 2;
                    }

                    if ((explorer_count >> 1) % 2) == 1 {
                        birthmark += 2;
                    }
                }
                g.units[u].ai_birthmark = birthmark;
            }
        }
    }

    /// C++ `UnitAITypes CvUnitAI::AI_getUnitAIType() const` (CvUnitAI.cpp:1425-1428)
    pub fn unit_ai_get_unit_ai_type(&self, g: &Game, u: UnitId) -> Option<UnitAIType> {
        g.units[u].ai_type
    }

    /// C++ `void CvUnitAI::AI_setUnitAIType(UnitAITypes eNewValue)` (CvUnitAI.cpp:1432-1448)
    pub fn unit_ai_set_unit_ai_type(&self, g: &mut Game, u: UnitId, new_value: UnitAIType) {
        // ffh-core ports it (area and player AI-unit counts, `joinGroup(NULL)` on change).
        g.unit_ai_set_unit_ai_type(u, new_value);
    }

    /// C++ `int CvUnitAI::AI_sacrificeValue(const CvPlot* pPlot) const` (CvUnitAI.cpp:1450-1495)
    pub fn unit_ai_sacrifice_value(&self, g: &Game, u: UnitId, plot: PlotIdx) -> i32 {
        let info = &g.d.units[g.units[u].unit_type];
        let mut collateral_damage_value = 0;
        {
            let possible_targets = (g.plot_get_num_visible_enemy_defenders(plot, u) - 1).min(g.unit_collateral_damage_max_units(u));

            if possible_targets > 0 {
                collateral_damage_value = g.unit_collateral_damage(u);
                collateral_damage_value += 0.max(collateral_damage_value - 100);
                collateral_damage_value *= possible_targets;
                collateral_damage_value /= 5;
            }
        }

        let mut value: i32;
        if g.unit_get_domain_type(u) == DomainType::DOMAIN_AIR {
            value = 128 * (100 + g.unit_curr_interception_probability(u));
            if info.nuke_range != -1 {
                value += 25000;
            }
            value /= 1.max(1 + info.production_cost);
            value = value.wrapping_mul(g.unit_max_hit_points(u) - g.units[u].damage);
            value /= 100;
        } else {
            value = 128i32.wrapping_mul(g.unit_curr_effective_str(u, Some(plot), Some(u), None));
            value = value.wrapping_mul(100 + collateral_damage_value);
            value /= (100 + g.unit_city_defense_modifier(u)).max(1);
            value = value.wrapping_mul(100 + g.unit_withdrawal_probability(u));
            value /= 1.max(1 + info.production_cost);
            value /= (10 + g.units[u].experience).max(1);

            if info.combat_limit < 100 {
                value = value.wrapping_mul(150);
                value /= 100;
            }
        }

        value
    }

    /// C++ `void CvUnitAI::AI_animalMove()` (CvUnitAI.cpp:1499-1536)
    pub fn unit_ai_animal_move(&self, g: &mut Game, u: UnitId) {
        //FfH: Added by Kael 10/26/2008 So that animals can built their pens...
        if !g.unit_is_barbarian(u) {
            if g.units[u].level < 2 {
                if self.unit_ai_construct(g, u, MAX_INT, MAX_INT, 15) {
                    return;
                }
                if !g.units.contains(u) {
                    return;
                }
            }
        }
        //FfH: End Add

        if self.rand(100, "Animal Attack") < g.d.handicaps[g.handicap].animal_attack_prob {
            if self.unit_ai_any_attack(g, u, 1, 0, 0, false) {
                return;
            }
            if !g.units.contains(u) {
                return;
            }
        }

        if self.unit_ai_heal(g, u, 0, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_patrol(g, u) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// The best plot within 6 plots to found a city on, reachable in under 3 turns (the two
    /// identical search loops of `AI_settleMove`).
    fn settle_search_best_plot(&self, g: &mut Game, u: UnitId) -> Option<PlotIdx> {
        let owner = g.units[u].owner;
        let search_range = 6;
        let mut best_value = 0;
        let mut best_plot: Option<PlotIdx> = None;
        let (ux, uy) = (g.units[u].x, g.units[u].y);

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                    continue;
                };
                if self.unit_ai_plot_valid(g, u, loop_plot) && g.unit_can_found(u, loop_plot, false) {
                    if !g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                        let mut path_turns = 0;
                        if g.unit_generate_path(u, loop_plot, 0, true, Some(&mut path_turns)) {
                            if path_turns < 3 {
                                let value = g.plot_get_found_value(loop_plot, owner);
                                if value > best_value {
                                    best_value = value;
                                    best_plot = Some(loop_plot);
                                }
                            }
                        }
                    }
                }
            }
        }

        best_plot
    }

    /// C++ `void CvUnitAI::AI_settleMove()` (CvUnitAI.cpp:1539-1909)
    pub fn unit_ai_settle_move(&self, g: &mut Game, u: UnitId) {
        let owner = g.units[u].owner;

        // BETTER AI (UNITAI_SETTLE move) Sephi
        //build a city after leaving a ship
        if g.players[owner].cities.len() > 1 {
            let here = g.unit_plot(u);
            if self.player_ai_get_expansion_plot(g, owner) == Some(here) {
                if g.unit_can_found(u, here, false) {
                    push_mission(g, u, MissionType::MISSION_FOUND);
                    return;
                }
            }
        }

        //reset values after first city is build
        if g.players[owner].cities.len() == 1 && g.groups[grp(g, u)].units.len() == 1 {
            self.player_ai_update_found_values(g, owner, false);
        }

        if g.players[owner].cities.is_empty() {
            if g.game_turn() == 0 {
                self.player_ai_update_found_values(g, owner, false);

                if let Some(best_plot) = self.settle_search_best_plot(g, u) {
                    if g.unit_at_plot(u, best_plot) {
                        push_mission(g, u, MissionType::MISSION_FOUND);
                        return;
                    } else {
                        push_move_to(g, u, best_plot, 0);
                        // Teleport other units to this plot
                        if g.game_turn() < 4 {
                            let (bx, by) = g.map.xy(best_plot);
                            for loop_unit in g.players[owner].units.clone() {
                                if !g.units.contains(loop_unit) {
                                    continue;
                                }
                                if loop_unit != u {
                                    g.unit_set_xy(loop_unit, bx, by, false);
                                }
                            }
                        }
                        return;
                    }
                }
            }

            let here = g.unit_plot(u);
            if g.unit_can_found(u, here, false) {
                push_mission(g, u, MissionType::MISSION_FOUND);
                return;
            }

            //haven't found a City after 3 turns?
            if g.game_turn() > 3 {
                if let Some(best_plot) = self.settle_search_best_plot(g, u) {
                    if g.unit_at_plot(u, best_plot) {
                        push_mission(g, u, MissionType::MISSION_FOUND);
                        return;
                    } else {
                        push_move_to(g, u, best_plot, 0);
                        return;
                    }
                }
            }
        }

        let mut needed_patrol = 4;
        let team = g.unit_get_team(u);
        if g.teams[team].barbarian_ally {
            needed_patrol = 2;
        }

        let here = g.unit_plot(u);
        if g.plot_is_city(here, false, None) && (g.groups[grp(g, u)].units.len() as i32) < needed_patrol {
            if g.map.plots[here].owner == Some(owner) {
                for loop_unit in g.map.plots[here].units.clone() {
                    if !g.units.contains(loop_unit) {
                        continue;
                    }
                    if g.units[loop_unit].ai_groupflag == GROUPFLAG_SETTLERGROUP {
                        let target = g.units[loop_unit].group;
                        g.unit_join_group(u, target, true);
                        push_mission(g, u, MissionType::MISSION_SKIP);
                        return;
                    }
                }

                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        if (g.groups[grp(g, u)].units.len() as i32) < needed_patrol {
            if g.plot_is_city(here, false, None) {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }
        // END BETTER AI

        let mut area_best_found_value = 0;
        let mut other_best_found_value = 0;

        let num_sites = self.player_ai_get_num_city_sites(g, owner);
        for i in 0..num_sites {
            let Some(city_site_plot) = self.player_ai_get_city_site(g, owner, i) else {
                continue;
            };
            // UNOFFICIAL_PATCH 01/10/09 jdog5000: only count city sites we can get to
            if (same_area(g, u, city_site_plot) || g.unit_can_move_all_terrain(u)) && g.unit_generate_path(u, city_site_plot, 0, true, None) {
                if g.unit_plot(u) == city_site_plot {
                    if g.unit_can_found(u, city_site_plot, false) {
                        push_mission(g, u, MissionType::MISSION_FOUND);
                        return;
                    }
                }
                area_best_found_value = area_best_found_value.max(g.plot_get_found_value(city_site_plot, owner));
            } else {
                other_best_found_value = other_best_found_value.max(g.plot_get_found_value(city_site_plot, owner));
            }
        }
        // BETTER AI Sephi: the scrap / AI_load(UNITAI_SETTLER_SEA) blocks are commented out.

        let here = g.unit_plot(u);
        if (area_best_found_value > 0) && g.plot_is_best_adjacent_found(here, owner) {
            if g.unit_can_found(u, here, false) {
                push_mission(g, u, MissionType::MISSION_FOUND);
                return;
            }
        }

        if !g.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE)
            && !g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI)
            && !g.group_can_defend(grp(g, u))
        {
            if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
                return;
            }
            if !g.units.contains(u) {
                return;
            }
        }

        let here = g.unit_plot(u);
        if g.plot_is_city(here, false, None) && (g.map.plots[here].owner == Some(owner)) {
            if (self.player_ai_get_plot_danger(g, owner, here, -1, true) > 0) && (g.max_city_elimination() > 0) {
                if g.groups[grp(g, u)].units.len() < 3 {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return;
                }
            }
        }

        if area_best_found_value > 0 {
            if self.unit_ai_found(g, u) {
                return;
            }
            if !g.units.contains(u) {
                return;
            }
        }
        // BETTER AI Sephi: AI_load(UNITAI_SETTLER_SEA, ..) commented out.

        if self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        if self.unit_ai_safety(g, u) {
            return;
        }
        if !g.units.contains(u) {
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_workerMove()` (CvUnitAI.cpp:1912-2269)
    pub fn unit_ai_worker_move(&self, g: &mut Game, u: UnitId) {
        macro_rules! act {
            ($e:expr) => {
                if $e {
                    return;
                }
                if !g.units.contains(u) {
                    return;
                }
            };
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);

        let can_route = g.unit_can_build_route(u);
        let mut next_city = false;

        // XXX could be trouble...
        if g.map.plots[g.unit_plot(u)].owner != Some(owner) {
            act!(self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT));
        }

        if !g.unit_is_human(u) {
            if g.map.plots[g.unit_plot(u)].owner == Some(owner) {
                act!(self.unit_ai_load(
                    g,
                    u,
                    UnitAIType::UNITAI_SETTLER_SEA,
                    MissionAIType::MISSIONAI_LOAD_SETTLER,
                    Some(UnitAIType::UNITAI_SETTLE),
                    2,
                    -1,
                    -1,
                    0,
                    MOVE_SAFE_TERRITORY,
                    MAX_INT
                ));
            }
        }

        // Skyre Mod / BETTER AI (Workers retreat at Danger) merged Sephi
        if self.player_ai_is_plot_threatened(g, owner, g.unit_plot(u), 3, true) {
            act!(self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT));
        }

        if can_route {
            let here = g.unit_plot(u);
            if g.map.plots[here].owner == Some(owner) {
                // XXX team???
                if let Some(non_obsolete_bonus) = g.plot_get_non_obsolete_bonus_type(here, team) {
                    if !g.plot_is_connected_to_capital(here, None) {
                        if let Some(improvement) = g.map.plots[here].improvement_type {
                            if g.d.improvements[improvement].improvement_bonus[non_obsolete_bonus].bonus_trade {
                                act!(self.unit_ai_connect_plot(g, u, here, 0));
                            }
                        }
                    }
                }
            }
        }

        let mut best_bonus_plot: Option<PlotIdx> = None;
        let mut best_bonus_build: Option<BuildType> = None;
        let mut best_bonus_value = 0;

        act!(self.unit_ai_improve_bonus(g, u, 25, &mut best_bonus_plot, &mut best_bonus_build, &mut best_bonus_value));

        if can_route && !g.unit_is_barbarian(u) {
            act!(self.unit_ai_connect_city(g, u));
        }

        let mut city: Option<CityId> = None;

        let here = g.unit_plot(u);
        if g.map.plots[here].owner == Some(owner) {
            city = g.map.plots[here].plot_city.filter(|&c| g.cities.contains(c));
            if city.is_none() {
                city = g.map.plots[here].working_city.filter(|&c| g.cities.contains(c));
            }
        }

        if let Some(c) = city {
            let needed = self.city_ai_get_workers_needed(g, c);
            if (needed > 0)
                && (g.plot_is_city(g.unit_plot(u), false, None) || (needed < ((1 + self.city_ai_get_workers_have(g, c) * 2) / 3)))
            {
                act!(self.unit_ai_improve_city(g, u, c));
            }
        }

        act!(self.unit_ai_improve_local_plot(g, u, 2, city.unwrap_or(NULL_CITY)));

        //FfH: Modified by Kael 11/11/2007 (fort building commented out)

        if can_route && g.unit_is_barbarian(u) {
            act!(self.unit_ai_connect_city(g, u));
        }

        let needs_next = match city.filter(|&c| g.cities.contains(c)) {
            None => true,
            Some(c) => {
                let needed = self.city_ai_get_workers_needed(g, c);
                needed == 0 || (self.city_ai_get_workers_have(g, c) > (needed + 1))
            }
        };
        if needs_next {
            if let (Some(bp), Some(bb)) = (best_bonus_plot, best_bonus_build) {
                if best_bonus_value >= 15 {
                    act!(self.unit_ai_improve_plot(g, u, bp, bb));
                }
            }

            act!(self.unit_ai_next_city_to_improve(g, u, city.unwrap_or(NULL_CITY)));

            next_city = true;
        }

        if let (Some(bp), Some(bb)) = (best_bonus_plot, best_bonus_build) {
            act!(self.unit_ai_improve_plot(g, u, bp, bb));
        }

        if let Some(c) = city.filter(|&c| g.cities.contains(c)) {
            act!(self.unit_ai_improve_city(g, u, c));
        }

        if !next_city {
            act!(self.unit_ai_next_city_to_improve(g, u, city.unwrap_or(NULL_CITY)));
        }

        if can_route {
            act!(self.unit_ai_route_territory(g, u, true));

            act!(self.unit_ai_connect_bonus(g, u, false));

            act!(self.unit_ai_route_city(g, u));
        }

        act!(self.unit_ai_irrigate_territory(g, u));

        //FfH: Modified by Kael 11/11/2007 (fort building commented out)

        if can_route {
            act!(self.unit_ai_route_territory(g, u, false));
        }

        let human = g.unit_is_human(u);
        if !human || (g.unit_is_automated(u) && g.team_get_at_war_count(team, true) == 0) {
            if !human || (g.units[u].game_turn_created < g.game_turn()) {
                act!(self.unit_ai_next_city_to_improve_airlift(g, u));
            }
            // TEMPFIX Sephi: AI_load(UNITAI_SETTLER_SEA, ..) commented out.
        }

        act!(self.unit_ai_improve_local_plot(g, u, 3, NULL_CITY));

        // BETTER AI (stop AI from deleting workers) Sephi: the scrap block is commented out.

        act!(self.unit_ai_retreat_to_city(g, u, false, true, MAX_INT));

        act!(self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT));

        act!(self.unit_ai_safety(g, u));

        push_mission(g, u, MissionType::MISSION_SKIP);
    }

    /// C++ `void CvUnitAI::AI_barbAttackMove()` (CvUnitAI.cpp:2272-2670)
    pub fn unit_ai_barb_attack_move(&self, g: &mut Game, u: UnitId) {
        macro_rules! act {
            ($e:expr) => {
                if $e {
                    return;
                }
                if !g.units.contains(u) {
                    return;
                }
            };
        }

        let owner = g.units[u].owner;

        // BETTER AI (faster barbs) Sephi 0.41k
        let here = g.unit_plot(u);
        if g.plot_is_city(here, false, None) {
            if g.map.plots[here].units.len() == 1 {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        //Orthus (and other Heroes) will wait until they have enough XP
        if g.units[u].ai_type == Some(UnitAIType::UNITAI_HERO) {
            if g.units[u].level < 7 {
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }

        //chance every turn for a Lair unit to break free
        if g.unit_ai_get_origin_plot(u).is_some() {
            if self.rand(400, "break free") == 0 {
                g.unit_ai_set_origin_plot(u, None);
            }
        }

        if g.units[u].ai_type == Some(UnitAIType::UNITAI_ATTACK_CITY) {
            //wait for Reinforcements?
            if g.groups[grp(g, u)].units.len() < 4 && g.plot_is_city(g.unit_plot(u), false, None) {
                act!(self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, -1, -1, 1, false, false, true, 1, true));
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
            //Looks for targets (preference of Cities)
            else {
                act!(self.unit_ai_heal(g, u, 0, MAX_INT));

                act!(self.unit_ai_any_attack(g, u, 1, 20, 0, false));

                act!(self.unit_ai_city_attack(g, u, 3, 20, false));

                act!(self.unit_ai_city_attack(g, u, 3, 0, false));

                act!(self.unit_ai_city_attack(g, u, 10, 40, false));

                act!(self.unit_ai_city_attack(g, u, 10, 20, false));

                act!(self.unit_ai_city_attack(g, u, 10, 0, false));

                act!(self.unit_ai_city_attack(g, u, 20, 40, false));

                act!(self.unit_ai_city_attack(g, u, 20, 20, false));

                act!(self.unit_ai_city_attack(g, u, 20, 0, false));
                push_mission(g, u, MissionType::MISSION_SKIP);
                return;
            }
        }
        let ai = g.units[u].ai_type;
        if ai == Some(UnitAIType::UNITAI_ATTACK) || ai == Some(UnitAIType::UNITAI_HERO) {
            let here = g.unit_plot(u);
            // Barbs pillage and heal less in AI land.
            let in_ai_land = g.map.plots[here].owner.map_or(false, |o| !g.players[o].human);

            if g.unit_can_pillage(u, here) {
                let pillage_chance = if in_ai_land { 10 } else { 50 };

                if self.rand(100, "AI Pillage") < pillage_chance {
                    let group = grp(g, u);
                    g.group_push_mission(
                        group,
                        MissionType::MISSION_PILLAGE,
                        -1,
                        -1,
                        0,
                        false,
                        false,
                        Some(MissionAIType::MISSIONAI_PILLAGE),
                        Some(here),
                        None,
                    );
                    return;
                }
            }

            if g.units[u].damage > 10 {
                //Barbs heal less in AI Land
                let heal_chance = if in_ai_land { 20 } else { 80 };
                if self.rand(100, "Healing") < heal_chance {
                    push_mission(g, u, MissionType::MISSION_SKIP);
                    return;
                }
            }

            //Look for a nice target close by
            let range = 5;
            let mut best_plot: Option<PlotIdx> = None;
            let mut best_value = 10000;

            let pillage_chance = if in_ai_land { 8 } else { 33 };
            let time_to_pillage = self.rand(100, "AI Pillage") < pillage_chance;

            let (ux, uy) = (g.units[u].x, g.units[u].y);
            for dx in -range..=range {
                for dy in -range..=range {
                    let Some(loop_plot) = g.plot_xy(ux, uy, dx, dy) else {
                        continue;
                    };
                    if loop_plot == here || !same_area(g, u, loop_plot) {
                        continue;
                    }
                    if !time_to_pillage {
                        //look for things to smash
                        if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u)
                            && self.player_ai_get_enemy_plot_strength(g, owner, loop_plot, 0, true, false) > 0
                        {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, loop_plot, 0, false, Some(&mut path_turns)) {
                                // C++ computes a victim value (enemy strength + 1000, / path
                                // turns, halved when owned) that it never uses: the closest wins.
                                if path_turns < best_value {
                                    best_plot = Some(loop_plot);
                                    best_value = path_turns;
                                }
                            }
                        }
                    } else {
                        //look for things to pillage
                        if g.unit_can_pillage(u, loop_plot) {
                            let mut path_turns = 0;
                            if g.unit_generate_path(u, loop_plot, 0, false, Some(&mut path_turns)) {
                                if path_turns < best_value {
                                    best_plot = Some(loop_plot);
                                    best_value = path_turns;
                                }
                            }
                        }
                    }
                }
            }

            if let Some(best_plot) = best_plot {
                if time_to_pillage {
                    // FIX: C++ pushes MISSION_PILLAGE on the unit's own plot (never the best
                    // plot, which is another plot), a mission that cannot start when the own
                    // plot has nothing to pillage, so the barbarian idles. Move to the plot to
                    // pillage instead (as AI_pillageRange does).
                    let (bx, by) = g.map.xy(best_plot);
                    let group = grp(g, u);
                    g.group_push_mission(
                        group,
                        MissionType::MISSION_MOVE_TO,
                        bx,
                        by,
                        0,
                        false,
                        false,
                        Some(MissionAIType::MISSIONAI_PILLAGE),
                        Some(best_plot),
                        None,
                    );
                    return;
                } else {
                    push_move_to(g, u, best_plot, MOVE_DIRECT_ATTACK);
                    return;
                }
            }

            act!(self.unit_ai_any_attack(g, u, 10, 0, 0, false));

            push_mission(g, u, MissionType::MISSION_SKIP);
            return;
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
        // C++ quirk: the rest of the function (the FfH barbarian logic: guard city, goody,
        // pillage range, city attack, target city, load, patrol, retreat, safety) follows an
        // unconditional `return;` and never runs.
    }

    /// C++ `void CvUnitAI::AI_attackMove()` (CvUnitAI.cpp:2673-2953)
    pub fn unit_ai_attack_move(&self, g: &mut Game, u: UnitId) {
        macro_rules! act {
            ($e:expr) => {
                if $e {
                    return;
                }
                if !g.units.contains(u) {
                    return;
                }
            };
        }

        let owner = g.units[u].owner;
        let team = g.unit_get_team(u);
        let danger = self.player_ai_get_plot_danger(g, owner, g.unit_plot(u), 3, true) > 0;

        // CvUnitAI::AI_attackMove() 1
        {
            act!(self.unit_ai_guard_city(g, u, true, false, MAX_INT));

            act!(self.unit_ai_heal(g, u, 30, 1));

            act!(self.unit_ai_guard_city_airlift(g, u));

            act!(self.unit_ai_guard_city(g, u, false, true, 1));

            //join any city attacks in progress
            if g.map.plots[g.unit_plot(u)].owner != Some(owner) {
                act!(self.unit_ai_group_merge_range(g, u, UnitAIType::UNITAI_ATTACK_CITY, 1, true, true, false));
            }

            let area_ai_type = g.map.areas[g.unit_area(u)].area_ai_type[team];
            let here = g.unit_plot(u);
            if g.plot_is_city(here, false, None) {
                if g.map.plots[here].owner == Some(owner) {
                    if (area_ai_type == Some(AreaAIType::AREAAI_ASSAULT)) || (area_ai_type == Some(AreaAIType::AREAAI_ASSAULT_ASSIST)) {
                        act!(self.unit_ai_offensive_airlift(g, u));
                    }
                }
            }

            if danger {
                act!(self.unit_ai_city_attack(g, u, 1, 55, false));

                act!(self.unit_ai_any_attack(g, u, 1, 65, 0, false));

                if g.unit_collateral_damage(u) > 0 {
                    act!(self.unit_ai_any_attack(g, u, 1, 45, 3, false));
                }
            }

            if !g.unit_no_defensive_bonus(u) {
                act!(self.unit_ai_guard_city(g, u, false, false, MAX_INT));
            }

            if !danger {
                if g.map.plots[g.unit_plot(u)].owner == Some(owner) {
                    act!(self.unit_ai_load(
                        g,
                        u,
                        UnitAIType::UNITAI_SETTLER_SEA,
                        MissionAIType::MISSIONAI_LOAD_SETTLER,
                        Some(UnitAIType::UNITAI_SETTLE),
                        3,
                        -1,
                        -1,
                        -1,
                        MOVE_SAFE_TERRITORY,
                        3
                    ));

                    act!(self.unit_ai_load(
                        g,
                        u,
                        UnitAIType::UNITAI_ASSAULT_SEA,
                        MissionAIType::MISSIONAI_LOAD_ASSAULT,
                        None,
                        -1,
                        2,
                        -1,
                        -1,
                        MOVE_SAFE_TERRITORY,
                        4
                    ));

                    if self.player_ai_unit_target_mission_ais(g, owner, u, MissionAIType::MISSIONAI_GROUP, None) > 0 {
                        push_mission(g, u, MissionType::MISSION_SKIP);
                        return;
                    }
                }
            }

            if danger {
                act!(self.unit_ai_pillage_range(g, u, 1, 20));

                act!(self.unit_ai_city_attack(g, u, 1, 35, false));

                act!(self.unit_ai_any_attack(g, u, 1, 45, 0, false));

                act!(self.unit_ai_pillage_range(g, u, 3, 20));

                act!(self.unit_ai_choke(g, u, 1));
            }

            act!(self.unit_ai_goody(g, u, 3));

            act!(self.unit_ai_any_attack(g, u, 1, 70, 0, false));
        }

        // CvUnitAI::AI_attackMove() 2
        {
            if danger {
                act!(self.unit_ai_city_attack(g, u, 4, 30, false));

                act!(self.unit_ai_any_attack(g, u, 2, 40, 0, false));
            }

            let here = g.unit_plot(u);
            if !g.unit_is_enemy(u, g.plot_get_team(here), None) {
                act!(self.unit_ai_heal(g, u, 0, MAX_INT));
            }

            if (g.players[owner].ai_num_ai_units[UnitAIType::UNITAI_CITY_DEFENSE] > 0) || (g.team_get_at_war_count(team, true) > 0) {
                act!(self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK_CITY, 1, 1, -1, true, true, true, 5, false));

                act!(self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK, 1, 1, -1, true, true, false, 4, false));

                // C++ quirk: `getMoves()` is the movement already used this turn, not maxMoves().
                if (g.units[u].moves > 1) && self.player_ai_is_do_strategy(g, owner, AI_STRATEGY_FASTMOVERS) {
                    act!(self.unit_ai_group(g, u, UnitAIType::UNITAI_ATTACK, 4, 1, -1, true, false, false, 3, false));
                }
            }

            let area = g.unit_area(u);
            if g.map.areas[area].area_ai_type[team] != Some(AreaAIType::AREAAI_OFFENSIVE) {
                let barbarian = g.barbarian_player();
                if g.map.areas[area].cities_per_player[barbarian] > 0 {
                    act!(self.unit_ai_target_barb_city(g, u));
                }
            } else {
                if g.groups[grp(g, u)].units.len() > 1 {
                    act!(self.unit_ai_target_city(g, u, 0));
                }
            }

            act!(self.unit_ai_guard_city(g, u, false, true, 3));

            if (g.players[owner].cities.len() > 1) && (g.groups[grp(g, u)].units.len() == 1) {
                let area = g.unit_area(u);
                if g.map.areas[area].area_ai_type[team] != Some(AreaAIType::AREAAI_DEFENSIVE) {
                    if g.area_get_num_unrevealed_tiles(area, team) > 0 {
                        let group = grp(g, u);
                        if self.player_ai_area_mission_ais(g, owner, area, MissionAIType::MISSIONAI_EXPLORE, Some(group))
                            < (self.player_ai_needed_explorers(g, owner, area) + 1)
                        {
                            act!(self.unit_ai_explore_range(g, u, 3));

                            act!(self.unit_ai_explore(g, u));
                        }
                    }
                }
            }

            act!(self.unit_ai_protect(g, u, 35));

            act!(self.unit_ai_offensive_airlift(g, u));

            act!(self.unit_ai_defend(g, u));

            act!(self.unit_ai_travel_to_upgrade_city(g, u));

            if self.rand(200, "CheckUnitforUpgrade") < 2 {
                act!(self.unit_ai_travel_to_upgrade_city(g, u));
            }

            act!(self.unit_ai_patrol(g, u));

            act!(self.unit_ai_retreat_to_city(g, u, false, false, MAX_INT));

            act!(self.unit_ai_safety(g, u));
        }

        push_mission(g, u, MissionType::MISSION_SKIP);
    }
}
