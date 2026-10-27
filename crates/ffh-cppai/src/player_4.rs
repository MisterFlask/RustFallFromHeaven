//! Port of CvPlayerAI.cpp:9822-12384 (FFH2 0.41 C++ AI).
//!
//! SIGNATURE NOTES:
//! - `player_ai_wake_plot_target_mission_ais` takes `&mut Game` (was `&Game`): the C++ is
//!   `const` but wakes the groups it counts (`setActivityType(ACTIVITY_AWAKE)`). Its one caller
//!   (`CvUnitAI::AI_seaBombardRange`, unit_5.rs) is a `&mut Game` function, so `g` still passes.
//! - The `MissionAIType` parameters cannot express `NO_MISSIONAI`, which the C++ mission lists
//!   use as "any mission AI". `CvUnitAI.cpp:16912` calls
//!   `AI_plotTargetMissionAIs(pCity->plot(), NO_MISSIONAI, getGroup(), 2)`: call
//!   `player_ai_plot_target_mission_ais_any` (added here) for it.
//! - `player_ai_missionary_value` / `player_ai_executive_value` take an `AreaId`: the C++
//!   `pArea == NULL` (whole team) branch is unreachable (no C++ caller passes NULL).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

/// Runs `f` on the C++ AI state of player `p`, the per-player tables sized to this game first.
fn pai<R>(cai: &Cai, g: &Game, p: PlayerId, f: impl FnOnce(&mut PlayerAi) -> R) -> R {
    let n = g.players.len();
    let mut s = cai.s();
    s.ensure_players(n);
    f(s.player(p))
}

/// C++ `GC.getLeaderHeadInfo(getPersonalityType())`.
fn personality_info(g: &Game, p: PlayerId) -> &'static LeaderHeadInfo {
    let d: &'static GameData = g.d;
    // PORT ADAPT: C++ assumes a personality; fall back to the leader type, then leader 0.
    let l = g.players[p].personality_type.or_else(|| g.player_get_leader_type(p)).unwrap_or(LeaderHeadType(0));
    &d.leader_heads[l]
}

/// C++ `GC.getLeaderHeadInfo(getLeaderType())`.
fn leader_info(g: &Game, p: PlayerId) -> &'static LeaderHeadInfo {
    let d: &'static GameData = g.d;
    let l = g.player_get_leader_type(p).or(g.players[p].personality_type).unwrap_or(LeaderHeadType(0));
    &d.leader_heads[l]
}

/// `pPlot->getPlotCity()`.
fn plot_city(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].plot_city
}

/// C++ `CvPlayerAI::AI_plotTargetMissionAIs(pPlot, aeMissionAI, iMissionAICount,
/// iClosestTargetRange, pSkipSelectionGroup, iRange)` with `None` = `NO_MISSIONAI` (any).
fn plot_target_mission_ais(g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: &[Option<MissionAIType>], closest_target_range: &mut i32, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
    let mut count = 0;
    *closest_target_range = MAX_INT;

    let (px, py) = (g.map.plots[plot].x, g.map.plots[plot].y);

    for &grp in &g.players[p].groups {
        if Some(grp) != skip_selection_group {
            if let Some(mission_plot) = g.group_ai_get_mission_ai_plot(grp) {
                let group_mission_ai = g.groups[grp].ai_mission_ai_type;
                let (mx, my) = (g.map.plots[mission_plot].x, g.map.plots[mission_plot].y);
                let distance = g.step_distance(px, py, mx, my);

                if distance <= range {
                    for &m in mission_ai {
                        if m.is_none() || group_mission_ai == m {
                            count += g.groups[grp].units.len() as i32;

                            if distance < *closest_target_range {
                                *closest_target_range = distance;
                            }
                        }
                    }
                }
            }
        }
    }

    count
}

impl Cai {
    /// C++ `int CvPlayerAI::AI_neededWorkers(CvArea* pArea) const` (CvPlayerAI.cpp:9822-9856)
    pub fn player_ai_needed_workers(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        let mut count = g.player_count_unimproved_bonuses(p, area, None) * 2;

        for &c in &g.players[p].cities {
            if g.city_area(c) == area {
                count += self.city_ai_get_workers_needed(g, c) * 3;
            }
        }

        if count == 0 {
            return 0;
        }

        let cities_per_player = g.map.areas[area].cities_per_player[p];

        if g.player_get_best_route(p, None).is_some() {
            count += cities_per_player / 2;
        }

        count += 1;
        count /= 3;
        count = count.min(3 * cities_per_player);
        count = count.min((1 + g.players[p].total_population) / 2);

        1.max(count)
    }

    /// C++ `int CvPlayerAI::AI_neededMissionaries(CvArea* pArea, ReligionTypes eReligion) const` (CvPlayerAI.cpp:9859-9902)
    pub fn player_ai_needed_missionaries(&self, g: &Game, p: PlayerId, area: AreaId, religion: ReligionType) -> i32 {
        let culture_victory = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2);

        let holy = g.player_has_holy_city(p, religion);
        let state_religion = g.player_get_state_religion(p);
        let state = state_religion == Some(religion);
        let holy_state = match state_religion {
            Some(sr) => g.player_has_holy_city(p, sr),
            None => false,
        };

        let mut count = 0;

        //internal spread.
        if culture_victory || state || holy {
            count = count.max(g.map.areas[area].cities_per_player[p] - g.area_count_has_religion(area, religion, Some(p)));
            if count > 0 {
                if !culture_victory {
                    count = 1.max(count / if holy { 2 } else { 4 });
                }
                return count;
            }
        }

        //external spread.
        if (holy && state) || (holy && !holy_state && state_religion.is_some()) {
            count += (g.map.areas[area].num_cities * 2) - (g.area_count_has_religion(area, religion, None) * 3);
            count /= 8;

            count = 0.max(count);

            if self.player_ai_is_primary_area(g, p, area) {
                count += 1;
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_neededExecutives(CvArea* pArea, CorporationTypes eCorporation) const` (CvPlayerAI.cpp:9905-9923)
    pub fn player_ai_needed_executives(&self, g: &Game, p: PlayerId, area: AreaId, corporation: CorporationType) -> i32 {
        if !g.player_has_headquarters(p, corporation) {
            return 0;
        }

        let a = &g.map.areas[area];
        let mut count = (a.cities_per_player[p] - g.area_count_has_corporation(area, corporation, Some(p))) * 2;
        count += a.num_cities - g.area_count_has_corporation(area, corporation, None);

        count /= 3;

        if self.player_ai_is_primary_area(g, p, area) {
            count += 1;
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_adjacentPotentialAttackers(CvPlot* pPlot, bool bTestCanMove = false) const` (CvPlayerAI.cpp:9926-9973)
    pub fn player_ai_adjacent_potential_attackers(&self, g: &Game, p: PlayerId, plot: PlotIdx, test_can_move: bool) -> i32 {
        let mut count = 0;

        let (x, y) = (g.map.plots[plot].x, g.map.plots[plot].y);
        let plot_area = g.map.plots[plot].area;
        let domain = if g.plot_is_water(plot) { DomainType::DOMAIN_SEA } else { DomainType::DOMAIN_LAND };

        for i in 0..DirectionType::NUM {
            if let Some(loop_plot) = g.plot_direction(x, y, DirectionType::from_usize(i)) {
                if g.map.plots[loop_plot].area == plot_area {
                    for &u in &g.map.plots[loop_plot].units {
                        if g.units[u].owner == p {
                            if g.unit_get_domain_type(u) == domain {
                                if g.unit_can_attack(u) {
                                    if !test_can_move || g.unit_can_move(u) {
                                        if !g.unit_ai_is_city_ai_type(u) {
                                            count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_totalMissionAIs(MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:9976-9998)
    pub fn player_ai_total_mission_ais(&self, g: &Game, p: PlayerId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        let mut count = 0;

        for &grp in &g.players[p].groups {
            if Some(grp) != skip_selection_group {
                if g.groups[grp].ai_mission_ai_type == Some(mission_ai) {
                    count += g.groups[grp].units.len() as i32;
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_missionaryValue(CvArea* pArea, ReligionTypes eReligion, PlayerTypes* peBestPlayer = NULL) const` (CvPlayerAI.cpp:10000-10136)
    pub fn player_ai_missionary_value(&self, g: &Game, p: PlayerId, area: AreaId, religion: ReligionType, best_player: &mut Option<PlayerId>) -> i32 {
        let team = g.players[p].team;

        let mut spread_internal_value = 100;
        let mut spread_external_value = 0;
        // C++ quirk: the three nested checks all test AI_STRATEGY_CULTURE1.
        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1) {
            spread_internal_value += 500;
            if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1) {
                spread_internal_value += 1500;
                if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1) {
                    spread_internal_value += 3000;
                }
            }
        }
        let state_religion = g.player_get_state_religion(p) == Some(religion);
        if state_religion {
            spread_internal_value += 1000;
        } else {
            spread_internal_value += (500 * g.players[p].has_religion_count[religion]) / 1.max(g.players[p].num_cities());
        }

        if g.team_has_holy_city(team, religion) {
            spread_internal_value += if state_religion { 1000 } else { 200 };
            spread_external_value += if state_religion { 1000 } else { 0 };
            if g.team_has_shrine(team, Some(religion)) {
                spread_internal_value += if state_religion { 1000 } else { 500 };
                spread_external_value += if state_religion { 600 } else { 100 };
                if let Some(holy_city) = g.get_holy_city(religion) {
                    let gold_multiplier = g.city_total_commerce_rate_modifier(holy_city, CommerceType::COMMERCE_GOLD);
                    spread_internal_value += 10 * 0.max(gold_multiplier - 100);
                    spread_external_value += 5 * 0.max(gold_multiplier - 150);
                }
            }
        }

        // pArea is never NULL here (see SIGNATURE NOTES).
        let our_cities_have = g.area_count_has_religion(area, religion, Some(p));
        let our_cities_count = g.map.areas[area].cities_per_player[p];

        if our_cities_have < our_cities_count {
            spread_internal_value *= 50 + 50 * (our_cities_count - our_cities_have);
            spread_internal_value /= 100 * our_cities_count;
        } else {
            spread_internal_value = 0;
        }

        if spread_external_value > 0 {
            let missionary_strategy = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_MISSIONARY);
            let mut best: Option<PlayerId> = None;
            let mut best_value = 0;
            for other in g.player_ids() {
                if other != p {
                    let kp = &g.players[other];
                    if kp.alive && kp.team != team && kp.num_cities() > 0 {
                        if g.team_is_open_borders(kp.team, team) {
                            let mut cities_count = 0;
                            let mut cities_have = 0;
                            let mut multiplier = if missionary_strategy { 60 } else { 25 };
                            let other_state_religion = g.player_get_state_religion(other);
                            if !g.player_is_no_non_state_religion_spread(other) || other_state_religion == Some(religion) {
                                let player_spread_percent = (100 * kp.has_religion_count[religion]) / kp.num_cities();
                                cities_count += g.map.areas[area].cities_per_player[other];
                                cities_have += cities_count.min((cities_count * player_spread_percent) / 75);
                            }

                            if other_state_religion.is_none() {
                                if kp.state_religion_count > 0 {
                                    let total_religions = g.player_count_total_has_religion(other);
                                    multiplier += 100 * 0.max(kp.num_cities() - total_religions);
                                    multiplier += if total_religions == 0 { 100 } else { 0 };
                                }
                            }

                            let mut value = (multiplier * spread_external_value * (cities_count - cities_have)) / 1.max(cities_count);
                            value /= 100;
                            if value > best_value {
                                best_value = value;
                                best = Some(other);
                            }
                        }
                    }
                }
            }

            if best_value > spread_internal_value {
                *best_player = best;
                return best_value;
            }
        }

        *best_player = Some(p);
        spread_internal_value
    }

    /// C++ `int CvPlayerAI::AI_executiveValue(CvArea* pArea, CorporationTypes eCorporation, PlayerTypes* peBestPlayer = NULL) const` (CvPlayerAI.cpp:10138-10285)
    pub fn player_ai_executive_value(&self, g: &Game, p: PlayerId, area: AreaId, corporation: CorporationType, best_player: &mut Option<PlayerId>) -> i32 {
        let team = g.players[p].team;

        let mut spread_internal_value = 100;
        let mut spread_external_value = 0;

        if g.team_has_headquarters(team, corporation) {
            if let Some(hq) = g.get_headquarters(corporation) {
                let gold_multiplier = g.city_total_commerce_rate_modifier(hq, CommerceType::COMMERCE_GOLD);
                spread_internal_value += 10 * 0.max(gold_multiplier - 100);
                spread_external_value += 15 * 0.max(gold_multiplier - 150);
            }
        }

        // pArea is never NULL here (see SIGNATURE NOTES).
        let mut our_cities_have = g.area_count_has_corporation(area, corporation, Some(p));
        let our_cities_count = g.map.areas[area].cities_per_player[p];

        for corp in g.d.corporations.keys() {
            if g.is_competing_corporation(corporation, corp) {
                // C++ quirk: counts eCorporation again, not the competing iCorp.
                our_cities_have += g.area_count_has_corporation(area, corporation, Some(p));
            }
        }

        if our_cities_have >= our_cities_count {
            spread_internal_value = 0;
            // C++ quirk: `if (iSpreadExternalValue = 0)` assigns 0 and never returns.
            spread_external_value = 0;
        }

        let mut bonus_value = 0;
        if let Some(capital) = g.player_get_capital_city(p) {
            bonus_value = self.player_ai_corporation_value(g, p, corporation, Some(capital));
            bonus_value /= 100;
        }

        for other in g.player_ids() {
            let kp = &g.players[other];
            if kp.alive && kp.num_cities() > 0 {
                if kp.team == team || g.teams[kp.team].vassal[team] {
                    if g.player_get_has_corporation_count(other, corporation) == 0 {
                        bonus_value += 1000;
                    }
                }
            }
        }

        if bonus_value == 0 {
            return 0;
        }

        spread_internal_value += bonus_value;

        if spread_external_value > 0 {
            let mut best: Option<PlayerId> = None;
            let mut best_value = 0;
            for other in g.player_ids() {
                if other != p {
                    let kp = &g.players[other];
                    if kp.alive && kp.team != team && kp.num_cities() > 0 {
                        if g.team_is_open_borders(kp.team, team) {
                            if !g.player_is_no_corporations(other) && !g.player_is_no_foreign_corporations(other) {
                                let mut cities_count = 0;
                                let mut cities_have = 0;
                                let multiplier = self.player_ai_get_attitude_weight(g, p, other);
                                let player_spread_percent = (100 * g.player_get_has_corporation_count(other, corporation)) / kp.num_cities();
                                cities_count += g.map.areas[area].cities_per_player[other];
                                cities_have += cities_count.min((cities_count * player_spread_percent) / 50);

                                if cities_have < cities_count {
                                    let mut value = multiplier * spread_external_value;
                                    value += ((multiplier - 55) * bonus_value) / 4;
                                    value /= 100;
                                    if value > best_value {
                                        best_value = value;
                                        best = Some(other);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if best_value > spread_internal_value {
                *best_player = best;
                return best_value;
            }
        }

        *best_player = Some(p);
        spread_internal_value
    }

    /// C++ `int CvPlayerAI::AI_corporationValue(CorporationTypes eCorporation, CvCity* pCity = NULL) const` (CvPlayerAI.cpp:10288-10335)
    pub fn player_ai_corporation_value(&self, g: &Game, p: PlayerId, corporation: CorporationType, city: Option<CityId>) -> i32 {
        let city = city.or_else(|| g.player_get_capital_city(p));
        let Some(city) = city else {
            return 0;
        };
        let corp = &g.d.corporations[corporation];
        let mut bonus_value = 0;

        for bonus in g.d.bonuses.keys() {
            let bonus_count = g.city_num_bonuses(city, bonus);
            if bonus_count > 0 {
                for &prereq in &corp.prereq_bonus {
                    if bonus == prereq {
                        bonus_value += 100 * corp.yield_produced[YieldType::YIELD_FOOD.idx()] * bonus_count;
                        bonus_value += 100 * corp.yield_produced[YieldType::YIELD_PRODUCTION.idx()] * bonus_count;
                        bonus_value += 60 * corp.yield_produced[YieldType::YIELD_COMMERCE.idx()] * bonus_count;

                        bonus_value += 60 * corp.commerce_produced[CommerceType::COMMERCE_GOLD.idx()] * bonus_count;
                        bonus_value += 60 * corp.commerce_produced[CommerceType::COMMERCE_RESEARCH.idx()] * bonus_count;
                        bonus_value += 25 * corp.commerce_produced[CommerceType::COMMERCE_CULTURE.idx()] * bonus_count;
                        bonus_value += 40 * corp.commerce_produced[CommerceType::COMMERCE_ESPIONAGE.idx()] * bonus_count;

                        if let Some(produced) = corp.bonus_produced {
                            let bonuses = g.player_get_num_available_bonuses(p, produced);
                            bonus_value += (self.player_ai_base_bonus_val(g, p, produced) * 1000) / (1 + 3 * bonuses * bonuses);
                        }
                    }
                }
            }
        }
        bonus_value *= 3;

        bonus_value
    }

    /// C++ `int CvPlayerAI::AI_areaMissionAIs(CvArea* pArea, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10337-10368)
    pub fn player_ai_area_mission_ais(&self, g: &Game, p: PlayerId, area: AreaId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        let mut count = 0;

        for &grp in &g.players[p].groups {
            if Some(grp) != skip_selection_group {
                if g.groups[grp].ai_mission_ai_type == Some(mission_ai) {
                    if let Some(mission_plot) = g.group_ai_get_mission_ai_plot(grp) {
                        if g.map.plots[mission_plot].area == Some(area) {
                            count += g.groups[grp].units.len() as i32;
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10371-10375)
    pub fn player_ai_plot_target_mission_ais(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        let mut closest_target_range = 0;
        plot_target_mission_ais(g, p, plot, &[Some(mission_ai)], &mut closest_target_range, skip_selection_group, range)
    }

    /// `AI_plotTargetMissionAIs(pPlot, NO_MISSIONAI, pSkipSelectionGroup, iRange)`: units of
    /// every group whose mission AI plot is within `range` of `plot`, whatever its mission AI
    /// (the `MissionAIType` parameters cannot hold `NO_MISSIONAI`; for CvUnitAI.cpp:16912).
    pub fn player_ai_plot_target_mission_ais_any(&self, g: &Game, p: PlayerId, plot: PlotIdx, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        let mut closest_target_range = 0;
        plot_target_mission_ais(g, p, plot, &[None], &mut closest_target_range, skip_selection_group, range)
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, int& iClosestTargetRange, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10377-10380)
    pub fn player_ai_plot_target_mission_ais_2(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, closest_target_range: &mut i32, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        plot_target_mission_ais(g, p, plot, &[Some(mission_ai)], closest_target_range, skip_selection_group, range)
    }

    /// C++ `int CvPlayerAI::AI_plotTargetMissionAIs(CvPlot* pPlot, MissionAITypes* aeMissionAI, int iMissionAICount, int& iClosestTargetRange, CvSelectionGroup* pSkipSelectionGroup = NULL, int iRange = 0) const` (CvPlayerAI.cpp:10382-10421)
    pub fn player_ai_plot_target_mission_ais_3(&self, g: &Game, p: PlayerId, plot: PlotIdx, mission_ai: &[MissionAIType], closest_target_range: &mut i32, skip_selection_group: Option<GroupId>, range: i32) -> i32 {
        let missions: Vec<Option<MissionAIType>> = mission_ai.iter().map(|&m| Some(m)).collect();
        plot_target_mission_ais(g, p, plot, &missions, closest_target_range, skip_selection_group, range)
    }

    /// C++ `int CvPlayerAI::AI_unitTargetMissionAIs(CvUnit* pUnit, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10424-10427)
    pub fn player_ai_unit_target_mission_ais(&self, g: &Game, p: PlayerId, unit: UnitId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        self.player_ai_unit_target_mission_ais_2(g, p, unit, &[mission_ai], skip_selection_group)
    }

    /// C++ `int CvPlayerAI::AI_unitTargetMissionAIs(CvUnit* pUnit, MissionAITypes* aeMissionAI, int iMissionAICount, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10429-10458)
    pub fn player_ai_unit_target_mission_ais_2(&self, g: &Game, p: PlayerId, unit: UnitId, mission_ai: &[MissionAIType], skip_selection_group: Option<GroupId>) -> i32 {
        let mut count = 0;

        for &grp in &g.players[p].groups {
            if Some(grp) != skip_selection_group {
                if g.group_ai_get_mission_ai_unit(grp) == Some(unit) {
                    let group_mission_ai = g.groups[grp].ai_mission_ai_type;
                    for &m in mission_ai {
                        if group_mission_ai == Some(m) {
                            count += g.groups[grp].units.len() as i32;
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_enemyTargetMissionAIs(MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10460-10463)
    pub fn player_ai_enemy_target_mission_ais(&self, g: &Game, p: PlayerId, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        self.player_ai_enemy_target_mission_ais_2(g, p, &[mission_ai], skip_selection_group)
    }

    /// C++ `int CvPlayerAI::AI_enemyTargetMissionAIs(MissionAITypes* aeMissionAI, int iMissionAICount, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10465-10497)
    pub fn player_ai_enemy_target_mission_ais_2(&self, g: &Game, p: PlayerId, mission_ai: &[MissionAIType], skip_selection_group: Option<GroupId>) -> i32 {
        let mut count = 0;
        let team = g.players[p].team;

        for &grp in &g.players[p].groups {
            if Some(grp) != skip_selection_group {
                if let Some(mission_plot) = g.group_ai_get_mission_ai_plot(grp) {
                    if g.plot_is_owned(mission_plot) {
                        let group_mission_ai = g.groups[grp].ai_mission_ai_type;
                        for &m in mission_ai {
                            if group_mission_ai == Some(m) {
                                if let Some(plot_team) = g.plot_get_team(mission_plot) {
                                    if g.team_ai_is_chosen_war(team, plot_team) {
                                        count += g.groups[grp].units.len() as i32;
                                        count += g.group_get_cargo(grp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_wakePlotTargetMissionAIs(CvPlot* pPlot, MissionAITypes eMissionAI, CvSelectionGroup* pSkipSelectionGroup = NULL) const` (CvPlayerAI.cpp:10499-10526)
    ///
    /// Takes `&mut Game` (the C++ is `const` but wakes the groups): see SIGNATURE NOTES.
    pub fn player_ai_wake_plot_target_mission_ais(&self, g: &mut Game, p: PlayerId, plot: PlotIdx, mission_ai: MissionAIType, skip_selection_group: Option<GroupId>) -> i32 {
        let mut count = 0;

        for grp in g.players[p].groups.clone() {
            if !g.groups.contains(grp) {
                continue;
            }
            if Some(grp) != skip_selection_group {
                let group_mission_ai = g.groups[grp].ai_mission_ai_type;
                if group_mission_ai == Some(mission_ai) {
                    let mission_plot = g.group_ai_get_mission_ai_plot(grp);
                    if mission_plot == Some(plot) {
                        count += g.groups[grp].units.len() as i32;
                        g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
                    }
                }
            }
        }

        count
    }

    /// C++ `CivicTypes CvPlayerAI::AI_bestCivic(CivicOptionTypes eCivicOption) const` (CvPlayerAI.cpp:10528-10570)
    pub fn player_ai_best_civic(&self, g: &Game, p: PlayerId, civic_option: CivicOptionType) -> Option<CivicType> {
        let mut best_value = MIN_INT;
        let mut best_civic = None;

        for civic in g.d.civics.keys() {
            if g.d.civics[civic].civic_option_type == Some(civic_option) {
                if g.player_can_do_civics(p, Some(civic)) {
                    let mut value = self.player_ai_civic_value(g, p, civic);

                    if g.player_is_civic(p, civic) {
                        if g.players[p].max_anarchy_turns > 0 {
                            value *= 6;
                            value /= 5;
                        } else {
                            value *= 16;
                            value /= 15;
                        }
                    }

                    if value > best_value {
                        best_value = value;
                        best_civic = Some(civic);
                    }
                }
            }
        }

        best_civic
    }

    /// C++ `int CvPlayerAI::AI_civicValue(CivicTypes eCivic) const` (CvPlayerAI.cpp:10573-10896)
    pub fn player_ai_civic_value(&self, g: &Game, p: PlayerId, civic: CivicType) -> i32 {
        let d = g.d;
        let culture_victory3 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3);
        let culture_victory2 = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2);

        let k = &d.civics[civic];
        let pl = &g.players[p];
        let team = pl.team;
        let num_cities = pl.num_cities();
        let total_population = pl.total_population;
        let num_military_units = pl.num_military_units;
        let is_civic = g.player_is_civic(p, civic);
        let avoid_science = self.player_ai_avoid_science(g, p);

        let war_plan = g.team_get_any_war_plan_count(team, true) > 0;

        let connected_foreign_cities = g.player_count_potential_foreign_trade_cities_connected(p);
        let total_religion_count = g.player_count_total_has_religion(p);
        let mut best_religion = self.player_ai_best_religion(g, p);
        if best_religion.is_none() {
            best_religion = g.player_get_state_religion(p);
        }
        let highest_religion_count = match best_religion {
            None => 0,
            Some(r) => pl.has_religion_count[r],
        };
        let leader = personality_info(g, p);
        let warmonger_percent = 25000 / 100.max(100 + leader.max_war_rand);

        let mut value = num_cities * 6;

        value += k.ai_weight * num_cities;

        value += g.player_get_civic_percent_anger(p, civic, false) / 10;

        value += -(k.anarchy_length * num_cities);

        value += -g.player_get_single_civic_upkeep(p, Some(civic), true);

        value += (k.great_people_rate_modifier * num_cities) / 10;
        value += (k.great_general_rate_modifier * num_military_units) / 50;
        value += (k.domestic_great_general_rate_modifier * num_military_units) / 100;
        value += -((k.distance_maintenance_modifier * 0.max(num_cities - 3)) / 8);
        value += -((k.num_cities_maintenance_modifier * 0.max(num_cities - 3)) / 8);
        value += (k.free_experience * num_cities * (if war_plan { 8 } else { 5 }) * warmonger_percent) / 100;
        value += (k.worker_speed_modifier * self.player_ai_get_num_ai_units(g, p, UnitAIType::UNITAI_WORKER)) / 15;
        value += (k.improvement_upgrade_rate_modifier * num_cities) / 50;
        value += (k.military_production_modifier * num_cities * warmonger_percent) / (if war_plan { 300 } else { 500 });
        value += k.base_free_units / 2;
        value += k.base_free_military_units / 3;
        value += (k.free_units_population_percent * total_population) / 200;
        value += (k.free_military_units_population_percent * total_population) / 300;
        value += -(k.gold_per_unit * pl.num_units());
        value += -(k.gold_per_military_unit * num_military_units * warmonger_percent) / 200;

        //FfH: Added by Kael 01/31/2009
        if k.military_food_production {
            if total_population < 30 {
                value -= (30 - total_population) * 5;
            }
        }
        //FfH: End Add

        value += g.get_world_size_max_conscript(civic) * (if war_plan { 20 + num_cities } else { (8 + num_cities) / 2 });
        value += if k.no_unhealthy_population { total_population / 3 } else { 0 };
        if war_plan {
            value += (k.exp_in_border_modifier * num_military_units) / 200;
        }
        value += if k.building_only_healthy { num_cities * 3 } else { 0 };
        value += -((k.war_weariness_modifier * num_cities) / (if war_plan { 10 } else { 50 }));
        value += k.free_specialist * num_cities * 12;
        value += (k.trade_routes * 0.max(connected_foreign_cities - num_cities * 3) * 6) + (num_cities * 2);
        value += -(if k.no_foreign_trade { connected_foreign_cities * 3 } else { 0 });
        if k.no_corporations {
            value -= g.player_count_headquarters(p) * (40 + 3 * num_cities);
        }
        if k.no_foreign_corporations {
            for corp in d.corporations.keys() {
                if !g.team_has_headquarters(team, corp) {
                    value += g.player_count_corporations(p, corp) * 3;
                }
            }
        }
        if k.corporation_maintenance_modifier != 0 {
            let mut corp_count = 0;
            let mut hq_count = 0;
            for corp in d.corporations.keys() {
                if g.team_has_headquarters(team, corp) {
                    hq_count += 1;
                }
                corp_count += g.player_count_corporations(p, corp);
            }
            value += (-k.corporation_maintenance_modifier * (hq_count * (25 + num_cities * 2) + corp_count * 7)) / 25;
        }

        if k.civic_percent_anger != 0 {
            let num_other_cities = g.num_cities - num_cities;
            value += (30 * num_cities * g.player_get_civic_percent_anger(p, civic, true)) / k.civic_percent_anger;

            let growth_percent = d.game_speeds[g.game_speed_type()].growth_percent;
            let mut target_game_turn = 2 * num_cities * growth_percent;
            target_game_turn /= 1.max(g.count_civ_players_ever_alive());
            target_game_turn += growth_percent * 30;

            target_game_turn /= 100;
            target_game_turn = 10.max(target_game_turn);

            let elapsed_turns = g.elapsed_game_turns;

            if elapsed_turns > target_game_turn {
                value += (target_game_turn.min(elapsed_turns - target_game_turn) * (num_other_cities * k.civic_percent_anger)) / (15 * target_game_turn);
            }
        }

        if k.extra_health != 0 {
            value += (num_cities * 6 * self.player_ai_get_health_weight(g, p, if is_civic { -k.extra_health } else { k.extra_health }, 1)) / 100;
        }

        let temp_value = k.happy_per_military_unit * 3;
        if temp_value != 0 {
            value += (num_cities * 9 * self.player_ai_get_happiness_weight(g, p, if is_civic { -temp_value } else { temp_value }, 1)) / 100;
        }

        let temp_value = k.largest_city_happiness;
        if temp_value != 0 {
            let target_num_cities = d.worlds[g.setup.world_size].target_num_cities;
            value += (12 * num_cities.min(target_num_cities) * self.player_ai_get_happiness_weight(g, p, if is_civic { -temp_value } else { temp_value }, 1)) / 100;
        }

        if k.war_weariness_modifier != 0 {
            let anger_percent = g.player_get_war_weariness_percent_anger(p);
            let population = 3 + (total_population / 1.max(num_cities));

            let temp_value = (-k.war_weariness_modifier * anger_percent * population) / (d.defines.percent_anger_divisor * 100);
            if temp_value != 0 {
                value += (11 * num_cities * self.player_ai_get_happiness_weight(g, p, if is_civic { -temp_value } else { temp_value }, 1)) / 100;
            }
        }

        value += k.non_state_religion_happiness * (total_religion_count - highest_religion_count) * 5;

        if k.state_religion {
            if highest_religion_count > 0 {
                value += highest_religion_count;

                value += if k.no_non_state_religion_spread { (num_cities - highest_religion_count) * 2 } else { 0 };
                value += k.state_religion_happiness * highest_religion_count * 4;
                value += (k.state_religion_great_people_rate_modifier * highest_religion_count) / 20;
                value += k.state_religion_great_people_rate_modifier / 4;
                value += (k.state_religion_unit_production_modifier * highest_religion_count) / 4;
                value += (k.state_religion_building_production_modifier * highest_religion_count) / 3;
                value += k.state_religion_free_experience * highest_religion_count * (if war_plan { 6 } else { 2 });
            }
        }

        let capital = g.player_get_capital_city(p);
        for i in 0..YieldType::NUM {
            let yield_type = YieldType::from_usize(i);
            let mut temp_value = 0;

            temp_value += (k.yield_modifier[i] * num_cities) / 2;
            temp_value += (k.capital_yield_modifier[i] * 3) / 4;
            if let Some(cap) = capital {
                temp_value += (k.capital_yield_modifier[i] * g.cities[cap].base_yield_rate[i]) / 80;
            }
            temp_value += (k.trade_yield_modifier[i] * num_cities) / 11;

            let average_yield_multiplier = self.player_ai_average_yield_multiplier(g, p, yield_type);
            for imp in d.improvements.keys() {
                temp_value += (average_yield_multiplier * (k.improvement_yield_changes[imp][i] * (pl.improvement_count[imp] + num_cities * 2))) / 100;
            }

            if yield_type == YieldType::YIELD_FOOD {
                temp_value *= 3;
            } else if yield_type == YieldType::YIELD_PRODUCTION {
                temp_value *= if avoid_science { 6 } else { 2 };
            } else if yield_type == YieldType::YIELD_COMMERCE {
                temp_value *= if avoid_science { 1 } else { 2 };
            }

            value += temp_value;
        }

        for i in 0..CommerceType::NUM {
            let commerce = CommerceType::from_usize(i);
            let mut temp_value = 0;

            temp_value += (k.commerce_modifier[i] * num_cities) / 3;
            temp_value += k.capital_commerce_modifier[i] / 2;
            if commerce == CommerceType::COMMERCE_ESPIONAGE {
                temp_value *= self.player_ai_get_espionage_weight(g, p);
                temp_value /= 500;
            }
            temp_value += (k.specialist_extra_commerce[i] * total_population) / 15;

            temp_value *= self.player_ai_commerce_weight(g, p, commerce, None);

            if commerce == CommerceType::COMMERCE_CULTURE && culture_victory2 {
                temp_value *= 2;
                if culture_victory3 {
                    temp_value *= 2;
                }
            }
            temp_value /= 100;

            value += temp_value;
        }

        for bc in d.building_classes.keys() {
            if k.building_happiness_changes[bc] != 0 {
                value += k.building_happiness_changes[bc] * pl.building_class_count[bc] * 3;
            }
        }

        for f in d.features.keys() {
            let happiness = k.feature_happiness_changes[f];

            if happiness != 0 {
                value += happiness * g.player_count_city_features(p, f) * 5;
            }
        }

        for h in d.hurries.keys() {
            if k.hurry[h] {
                let hi = &d.hurries[h];
                let mut temp_value = 0;

                if hi.gold_per_production > 0 {
                    temp_value += ((if avoid_science { 50 } else { 25 }) * num_cities) / hi.gold_per_production;
                }
                temp_value += (hi.production_per_population * num_cities * (if war_plan { 2 } else { 1 })) / 5;
                value += temp_value;
            }
        }

        for sb in d.special_buildings.keys() {
            if k.special_building_not_required[sb] {
                value += (num_cities / 2) + 1; // XXX
            }
        }

        for sp in d.specialists.keys() {
            let mut temp_value = 0;
            if k.specialist_valid[sp] {
                temp_value += (num_cities * (if culture_victory3 { 10 } else { 1 })) + 6;
            }
            value += temp_value / 2;
        }

        if leader.favorite_civic == Some(civic) {
            if !k.state_religion || highest_religion_count > 0 {
                value *= 5;
                value /= 4;
                value += 6 * num_cities;
                value += 20;
            }
        }

        if culture_victory2 && k.no_non_state_religion_spread {
            value /= 10;
        }

        value
    }

    /// C++ `ReligionTypes CvPlayerAI::AI_bestReligion() const` (CvPlayerAI.cpp:10899-10958)
    pub fn player_ai_best_religion(&self, g: &Game, p: PlayerId) -> Option<ReligionType> {
        let mut best_value = 0;
        let mut best_religion: Option<ReligionType> = None;

        let favorite = leader_info(g, p).favorite_religion;
        let state_religion = g.player_get_state_religion(p);

        for r in g.d.religions.keys() {
            if g.player_can_do_religion(p, r) {
                let mut value = self.player_ai_religion_value(g, p, r);

                if state_religion == Some(r) {
                    value *= 4;
                    value /= 3;
                }

                if favorite == Some(r) {
                    value *= 5;
                    value /= 4;
                }

                if value > best_value {
                    best_value = value;
                    best_religion = Some(r);
                }
            }
        }

        let Some(best) = best_religion else {
            return None;
        };
        if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_MISSIONARY) {
            return best_religion;
        }

        let best_count = g.players[p].has_religion_count[best];
        let spread_percent = (best_count * 100) / 1.max(g.players[p].num_cities());
        let purity_percent = (best_count * 100) / 1.max(g.player_count_total_has_religion(p));
        if purity_percent < 49 {
            let is_favorite = favorite == Some(best);
            if spread_percent > (if is_favorite { 65 } else { 75 }) {
                if purity_percent > (if is_favorite { 25 } else { 32 }) {
                    return best_religion;
                }
            }
            return None;
        }

        best_religion
    }

    /// C++ `int CvPlayerAI::AI_religionValue(ReligionTypes eReligion) const` (CvPlayerAI.cpp:10961-11023)
    pub fn player_ai_religion_value(&self, g: &Game, p: PlayerId, religion: ReligionType) -> i32 {
        if g.players[p].has_religion_count[religion] == 0 {
            return 0;
        }

        let mut value = g.count_religion_levels(religion);

        for &c in &g.players[p].cities {
            if g.cities[c].has_religion[religion] {
                value += g.cities[c].population;
            }
        }

        if let Some(holy_city) = g.get_holy_city(religion) {
            let our_holy_city = g.cities[holy_city].owner == p;
            let our_team_holy_city = g.city_team(holy_city) == g.players[p].team;

            if our_holy_city || our_team_holy_city {
                let mut commerce_count = 0;

                for b in g.d.buildings.keys() {
                    let n = g.city_num_active_building(holy_city, b);
                    if n > 0 {
                        for j in 0..CommerceType::NUM {
                            if g.d.buildings[b].global_religion_commerce == Some(religion) {
                                commerce_count += g.d.religions[religion].global_religion_commerce[j] * n;
                            }
                        }
                    }
                }

                if our_holy_city {
                    value *= 3 + commerce_count;
                    value /= 2;
                } else if our_team_holy_city {
                    value *= 4 + commerce_count;
                    value /= 3;
                }
            }
        }

        //FfH: Modified by Kael 09/29/2007
        value += personality_info(g, p).religion_weight_modifier[religion];
        value.max(0)
        //FfH: End Modify
    }

    /// C++ `EspionageMissionTypes CvPlayerAI::AI_bestPlotEspionage(CvPlot* pSpyPlot, PlayerTypes& eTargetPlayer, CvPlot*& pPlot, int& iData) const` (CvPlayerAI.cpp:11025-11202)
    pub fn player_ai_best_plot_espionage(&self, g: &Game, p: PlayerId, spy_plot: PlotIdx, target_player: &mut Option<PlayerId>, plot: &mut Option<PlotIdx>, data: &mut i32) -> Option<EspionageMissionType> {
        //ooookay what missions are possible
        let d = g.d;

        *plot = None;
        *data = -1;

        let mut best_mission: Option<EspionageMissionType> = None;
        let mut best_value = 0;

        let team = g.players[p].team;
        let attitude_threshold = if g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) { 50 } else { 1 };

        if g.plot_is_owned(spy_plot) {
            let (Some(spy_owner), Some(spy_team)) = (g.map.plots[spy_plot].owner, g.plot_get_team(spy_plot)) else {
                return None;
            };
            if spy_team != team {
                let big_espionage = self.player_ai_is_do_strategy(g, p, AI_STRATEGY_BIG_ESPIONAGE);
                if !big_espionage && (g.teams[team].ai_war_plan[spy_team].is_some() || self.player_ai_get_attitude_weight(g, p, spy_owner) < attitude_threshold) {
                    //Destroy Improvement.
                    if g.map.plots[spy_plot].improvement_type.is_some() {
                        for mission in d.espionage_missions.keys() {
                            if d.espionage_missions[mission].destroy_improvement {
                                let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, -1);

                                if value > best_value {
                                    best_value = value;
                                    best_mission = Some(mission);
                                    *target_player = Some(spy_owner);
                                    *plot = Some(spy_plot);
                                    *data = -1;
                                }
                            }
                        }
                    }
                }

                if let Some(city) = plot_city(g, spy_plot) {
                    //Something malicious
                    if self.player_ai_get_attitude_weight(g, p, spy_owner) < attitude_threshold {
                        //Destroy Building.
                        if !big_espionage {
                            for mission in d.espionage_missions.keys() {
                                if d.espionage_missions[mission].destroy_building_cost_factor > 0 {
                                    for building in d.buildings.keys() {
                                        if g.city_num_building(city, building) > 0 {
                                            let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, building.i());

                                            if value > best_value {
                                                best_value = value;
                                                best_mission = Some(mission);
                                                *target_player = Some(spy_owner);
                                                *plot = Some(spy_plot);
                                                *data = building.i();
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        //Destroy Project
                        for mission in d.espionage_missions.keys() {
                            if d.espionage_missions[mission].destroy_project_cost_factor > 0 {
                                for project in d.projects.keys() {
                                    let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, project.i());

                                    if value > best_value {
                                        best_value = value;
                                        best_mission = Some(mission);
                                        *target_player = Some(spy_owner);
                                        *plot = Some(spy_plot);
                                        *data = project.i();
                                    }
                                }
                            }
                        }

                        //General dataless city mission.
                        if !big_espionage {
                            for mission in d.espionage_missions.keys() {
                                let mi = &d.espionage_missions[mission];
                                if mi.city_poison_water_counter > 0 || mi.destroy_production_cost_factor > 0 || mi.steal_treasury_types > 0 {
                                    let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, -1);

                                    if value > best_value {
                                        best_value = value;
                                        best_mission = Some(mission);
                                        *target_player = Some(spy_owner);
                                        *plot = Some(spy_plot);
                                        *data = -1;
                                    }
                                }
                            }
                        }

                        //Disruption suitable for war.
                        if g.teams[team].at_war[spy_team] {
                            for mission in d.espionage_missions.keys() {
                                let mi = &d.espionage_missions[mission];
                                if mi.city_revolt_counter > 0 || mi.player_anarchy_counter > 0 {
                                    let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, -1);

                                    if value > best_value {
                                        best_value = value;
                                        best_mission = Some(mission);
                                        *target_player = Some(spy_owner);
                                        *plot = Some(spy_plot);
                                        *data = -1;
                                    }
                                }
                            }
                        }
                    }

                    //Steal Technology
                    for mission in d.espionage_missions.keys() {
                        if d.espionage_missions[mission].buy_tech_cost_factor > 0 {
                            for tech in d.techs.keys() {
                                let value = self.player_ai_espionage_val(g, p, spy_owner, mission, spy_plot, tech.i());

                                if value > best_value {
                                    best_value = value;
                                    best_mission = Some(mission);
                                    *target_player = Some(spy_owner);
                                    *plot = Some(spy_plot);
                                    *data = tech.i();
                                }
                            }
                        }
                    }
                }
            }
        }

        best_mission
    }

    /// C++ `int CvPlayerAI::AI_espionageVal(PlayerTypes eTargetPlayer, EspionageMissionTypes eMission, CvPlot* pPlot, int iData) const` (CvPlayerAI.cpp:11204-11474)
    pub fn player_ai_espionage_val(&self, g: &Game, p: PlayerId, target_player: PlayerId, mission: EspionageMissionType, plot: PlotIdx, data: i32) -> i32 {
        let d = g.d;
        let target_team = g.players[target_player].team;
        let team = g.players[p].team;
        let mi = &d.espionage_missions[mission];

        let cost = g.player_get_espionage_mission_cost(p, mission, Some(target_player), Some(plot), data, None);

        if !g.player_can_do_espionage_mission(p, mission, Some(target_player), Some(plot), data, None) {
            return 0;
        }

        let mut value = 0;
        if mi.destroy_improvement {
            if g.map.plots[plot].owner == Some(target_player) {
                if let Some(improvement) = g.map.plots[plot].improvement_type {
                    if let Some(bonus) = g.plot_get_non_obsolete_bonus_type(plot, target_team) {
                        // GET_PLAYER(eTargetPlayer).AI_bonusVal: a valuation of the target, our port.
                        value += self.player_ai_bonus_val(g, target_player, bonus, -1);
                    } else {
                        let mut temp_value = 0;
                        if g.map.plots[plot].working_city.is_some() {
                            let owner = g.map.plots[plot].owner;
                            temp_value += g.plot_calculate_improvement_yield_change(plot, improvement, YieldType::YIELD_FOOD, owner, false, false) * 2;
                            temp_value += g.plot_calculate_improvement_yield_change(plot, improvement, YieldType::YIELD_PRODUCTION, owner, false, false) * 1;
                            temp_value += g.plot_calculate_improvement_yield_change(plot, improvement, YieldType::YIELD_COMMERCE, owner, false, false) * 4;
                            temp_value += d.improvements[improvement].upgrade_time / 2;
                            value += temp_value;
                        }
                    }
                }
            }
        }

        if mi.destroy_building_cost_factor > 0 {
            // PORT ADAPT: C++ casts iData blindly; here a non-building iData scores nothing.
            if let Some(building) = opt_from_i32::<BuildingType>(data).filter(|b| b.idx() < d.buildings.len()) {
                if g.player_can_spy_destroy_building(p, target_player, building) {
                    if let Some(city) = plot_city(g, plot) {
                        if g.cities[city].num_real_building[building] > 0 {
                            let kb = &d.buildings[building];
                            let world_wonder = kb.building_class_type.map_or(false, |bc| is_world_wonder_class(d, bc));
                            if kb.production_cost > 1 && !world_wonder {
                                value += self.city_ai_building_value(g, city, building, 0);
                            }
                        }
                    }
                }
            }
        }

        if mi.destroy_project_cost_factor > 0 {
            if let Some(project) = opt_from_i32::<ProjectType>(data).filter(|pr| pr.idx() < d.projects.len()) {
                if g.player_can_spy_destroy_project(p, target_player, project) {
                    let kp = &d.projects[project];

                    value += g.player_get_production_needed_project(p, project) * (if kp.max_team_instances == 1 { 3 } else { 2 });
                }
            }
        }

        if mi.destroy_production_cost_factor > 0 {
            if let Some(city) = plot_city(g, plot) {
                let temp_value = g.city_production(city);
                if temp_value > 0 {
                    if let Some(project) = g.city_production_project(city) {
                        let kp = &d.projects[project];
                        value += temp_value * (if kp.max_team_instances == 1 { 4 } else { 2 });
                    } else if let Some(building) = g.city_production_building(city) {
                        let kb = &d.buildings[building];
                        if kb.building_class_type.map_or(false, |bc| is_world_wonder_class(d, bc)) {
                            value += 3 * temp_value;
                        }
                        value += temp_value;
                    } else {
                        value += temp_value;
                    }
                }
            }
        }

        if mi.destroy_unit_cost_factor > 0 || mi.buy_unit_cost_factor > 0 {
            // PORT ADAPT: C++ `GET_PLAYER(eTargetPlayer).getUnit(iData)` (a per-player unit id);
            // here iData is read as a global UnitId owned by the target.
            let unit = if data >= 0 {
                let u = UnitId(data as u32);
                if g.units.contains(u) && g.units[u].owner == target_player {
                    Some(u)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(u) = unit {
                let unit_type = g.units[u].unit_type;
                let default_ai = d.units[unit_type].default_unit_ai_type.unwrap_or(UnitAIType::UNITAI_UNKNOWN);
                let unit_area = g.unit_area(u);

                // GET_PLAYER(eTargetPlayer).AI_unitValue: a valuation of the target, our port.
                value += self.player_ai_unit_value(g, target_player, unit_type, default_ai, unit_area);

                if mi.buy_unit_cost_factor > 0 {
                    if !g.player_can_train(p, unit_type, false, false, false) || g.player_get_production_needed_unit(p, unit_type) > cost {
                        value += self.player_ai_unit_value(g, p, unit_type, default_ai, unit_area);
                    }
                }
            }
        }

        if mi.steal_treasury_types > 0 {
            let gold_stolen = data;
            return 2 * (2 * gold_stolen - cost);
        }

        if mi.counterespionage_num_turns > 0 {
            //iValue += 100 * GET_TEAM(getTeam()).AI_getAttitudeVal(GET_PLAYER(eTargetPlayer).getTeam());
        }

        if mi.buy_city_cost_factor > 0 {
            if let Some(city) = plot_city(g, plot) {
                value += self.player_ai_city_trade_val(g, p, city);
            }
        }

        if mi.city_insert_culture_amount_factor > 0 {
            if let Some(city) = plot_city(g, plot) {
                if g.cities[city].owner != p {
                    let mut culture_amount = mi.city_insert_culture_amount_factor * g.map.plots[plot].culture[p];
                    culture_amount /= 100;
                    if g.city_calculate_culture_percent(city, p) > 40 {
                        value += culture_amount * 3;
                    }
                }
            }
        }

        if mi.city_poison_water_counter > 0 {
            if let Some(city) = plot_city(g, plot) {
                let city_health = g.city_good_health(city) - g.city_bad_health(city, false, 0);
                let base_unhealth = mi.city_poison_water_counter;

                let mut average_unhealth = 0.max(base_unhealth - city_health);
                average_unhealth += 0.max((base_unhealth / 2) - city_health);

                average_unhealth /= 2;

                value += 8 * average_unhealth * base_unhealth;
            }
        }

        if mi.city_unhappiness_counter > 0 {
            if let Some(city) = plot_city(g, plot) {
                let city_unhappy = g.city_unhappy_level(city, 0);
                if city_unhappy > 0 {
                    let base_anger = mi.city_unhappiness_counter;

                    value += 7 * base_anger * base_anger;
                }
            }
        }

        if mi.city_revolt_counter > 0 {
            if let Some(city) = plot_city(g, plot) {
                if g.at_war(team, g.city_team(city)) {
                    value -= 10;
                    value += 5 * self.player_ai_adjacent_potential_attackers(g, p, plot, true);
                }
            }
        }

        if mi.buy_tech_cost_factor > 0 {
            if let Some(tech) = opt_from_i32::<TechType>(data).filter(|t| t.idx() < d.techs.len()) {
                if cost < g.team_get_research_left(team, tech) * 4 / 3 {
                    value += self.team_ai_tech_trade_val(g, team, tech, target_team);
                }
            }
        }

        if mi.switch_civic_cost_factor > 0 {
            if let Some(civic) = opt_from_i32::<CivicType>(data).filter(|c| c.idx() < d.civics.len()) {
                value += self.player_ai_civic_trade_val(g, p, civic, target_player);
            }
        }

        if mi.switch_religion_cost_factor > 0 {
            if let Some(religion) = opt_from_i32::<ReligionType>(data).filter(|r| r.idx() < d.religions.len()) {
                value += self.player_ai_religion_trade_val(g, p, religion, target_player);
            }
        }

        if mi.player_anarchy_counter > 0 {
            // AI doesn't use Player Anarchy
        }

        value
    }

    /// C++ `int CvPlayerAI::AI_getPeaceWeight() const` (CvPlayerAI.cpp:11476-11479)
    pub fn player_ai_get_peace_weight(&self, g: &Game, p: PlayerId) -> i32 {
        pai(self, g, p, |s| s.peace_weight)
    }

    /// C++ `void CvPlayerAI::AI_setPeaceWeight(int iNewValue)` (CvPlayerAI.cpp:11482-11485)
    pub fn player_ai_set_peace_weight(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        pai(self, g, p, |s| s.peace_weight = new_value);
    }

    /// C++ `int CvPlayerAI::AI_getEspionageWeight() const` (CvPlayerAI.cpp:11487-11494)
    pub fn player_ai_get_espionage_weight(&self, g: &Game, p: PlayerId) -> i32 {
        if g.is_option(GameOptionType::GAMEOPTION_NO_ESPIONAGE) {
            return 0;
        }
        pai(self, g, p, |s| s.espionage_weight)
    }

    /// C++ `void CvPlayerAI::AI_setEspionageWeight(int iNewValue)` (CvPlayerAI.cpp:11496-11499)
    pub fn player_ai_set_espionage_weight(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        pai(self, g, p, |s| s.espionage_weight = new_value);
    }

    /// C++ `int CvPlayerAI::AI_getAttackOddsChange() const` (CvPlayerAI.cpp:11502-11505)
    pub fn player_ai_get_attack_odds_change(&self, g: &Game, p: PlayerId) -> i32 {
        pai(self, g, p, |s| s.attack_odds_change)
    }

    /// C++ `void CvPlayerAI::AI_setAttackOddsChange(int iNewValue)` (CvPlayerAI.cpp:11508-11511)
    pub fn player_ai_set_attack_odds_change(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        pai(self, g, p, |s| s.attack_odds_change = new_value);
    }

    /// C++ `int CvPlayerAI::AI_getCivicTimer() const` (CvPlayerAI.cpp:11514-11517)
    pub fn player_ai_get_civic_timer(&self, g: &Game, p: PlayerId) -> i32 {
        g.players[p].ai_civic_timer
    }

    /// C++ `void CvPlayerAI::AI_setCivicTimer(int iNewValue)` (CvPlayerAI.cpp:11520-11524)
    pub fn player_ai_set_civic_timer(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        g.players[p].ai_civic_timer = new_value;
        debug_assert!(g.players[p].ai_civic_timer >= 0);
    }

    /// C++ `void CvPlayerAI::AI_changeCivicTimer(int iChange)` (CvPlayerAI.cpp:11527-11530)
    pub fn player_ai_change_civic_timer(&self, g: &mut Game, p: PlayerId, change: i32) {
        let v = self.player_ai_get_civic_timer(g, p) + change;
        self.player_ai_set_civic_timer(g, p, v);
    }

    /// C++ `int CvPlayerAI::AI_getReligionTimer() const` (CvPlayerAI.cpp:11533-11536)
    pub fn player_ai_get_religion_timer(&self, g: &Game, p: PlayerId) -> i32 {
        g.players[p].ai_religion_timer
    }

    /// C++ `void CvPlayerAI::AI_setReligionTimer(int iNewValue)` (CvPlayerAI.cpp:11539-11543)
    pub fn player_ai_set_religion_timer(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        g.players[p].ai_religion_timer = new_value;
        debug_assert!(g.players[p].ai_religion_timer >= 0);
    }

    /// C++ `void CvPlayerAI::AI_changeReligionTimer(int iChange)` (CvPlayerAI.cpp:11546-11549)
    pub fn player_ai_change_religion_timer(&self, g: &mut Game, p: PlayerId, change: i32) {
        let v = self.player_ai_get_religion_timer(g, p) + change;
        self.player_ai_set_religion_timer(g, p, v);
    }

    /// C++ `int CvPlayerAI::AI_getExtraGoldTarget() const` (CvPlayerAI.cpp:11551-11554)
    pub fn player_ai_get_extra_gold_target(&self, g: &Game, p: PlayerId) -> i32 {
        pai(self, g, p, |s| s.extra_gold_target)
    }

    /// C++ `void CvPlayerAI::AI_setExtraGoldTarget(int iNewValue)` (CvPlayerAI.cpp:11556-11559)
    pub fn player_ai_set_extra_gold_target(&self, g: &mut Game, p: PlayerId, new_value: i32) {
        pai(self, g, p, |s| s.extra_gold_target = new_value);
    }

    /// C++ `int CvPlayerAI::AI_getNumTrainAIUnits(UnitAITypes eIndex) const` (CvPlayerAI.cpp:11561-11566)
    pub fn player_ai_get_num_train_ai_units(&self, g: &Game, p: PlayerId, index: UnitAIType) -> i32 {
        g.players[p].ai_num_train_ai_units[index]
    }

    /// C++ `void CvPlayerAI::AI_changeNumTrainAIUnits(UnitAITypes eIndex, int iChange)` (CvPlayerAI.cpp:11569-11575)
    pub fn player_ai_change_num_train_ai_units(&self, g: &mut Game, p: PlayerId, index: UnitAIType, change: i32) {
        g.players[p].ai_num_train_ai_units[index] += change;
        debug_assert!(g.players[p].ai_num_train_ai_units[index] >= 0);
    }

    /// C++ `int CvPlayerAI::AI_getNumAIUnits(UnitAITypes eIndex) const` (CvPlayerAI.cpp:11578-11583)
    pub fn player_ai_get_num_ai_units(&self, g: &Game, p: PlayerId, index: UnitAIType) -> i32 {
        g.players[p].ai_num_ai_units[index]
    }

    /// C++ `void CvPlayerAI::AI_changeNumAIUnits(UnitAITypes eIndex, int iChange)` (CvPlayerAI.cpp:11586-11592)
    pub fn player_ai_change_num_ai_units(&self, g: &mut Game, p: PlayerId, index: UnitAIType, change: i32) {
        g.players[p].ai_num_ai_units[index] += change;
        debug_assert!(g.players[p].ai_num_ai_units[index] >= 0);
    }

    /// C++ `int CvPlayerAI::AI_getSameReligionCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11595-11600)
    pub fn player_ai_get_same_religion_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        pai(self, g, p, |s| s.same_religion_counter[index.idx()])
    }

    /// C++ `void CvPlayerAI::AI_changeSameReligionCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11603-11609)
    pub fn player_ai_change_same_religion_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        pai(self, g, p, |s| s.same_religion_counter[index.idx()] += change);
    }

    /// C++ `int CvPlayerAI::AI_getDifferentReligionCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11612-11617)
    pub fn player_ai_get_different_religion_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        pai(self, g, p, |s| s.different_religion_counter[index.idx()])
    }

    /// C++ `void CvPlayerAI::AI_changeDifferentReligionCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11620-11626)
    pub fn player_ai_change_different_religion_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        pai(self, g, p, |s| s.different_religion_counter[index.idx()] += change);
    }

    /// C++ `int CvPlayerAI::AI_getFavoriteCivicCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11629-11634)
    pub fn player_ai_get_favorite_civic_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        pai(self, g, p, |s| s.favorite_civic_counter[index.idx()])
    }

    /// C++ `void CvPlayerAI::AI_changeFavoriteCivicCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11637-11643)
    pub fn player_ai_change_favorite_civic_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        pai(self, g, p, |s| s.favorite_civic_counter[index.idx()] += change);
    }

    /// C++ `int CvPlayerAI::AI_getBonusTradeCounter(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11646-11651)
    pub fn player_ai_get_bonus_trade_counter(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        pai(self, g, p, |s| s.bonus_trade_counter[index.idx()])
    }

    /// C++ `void CvPlayerAI::AI_changeBonusTradeCounter(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11654-11660)
    pub fn player_ai_change_bonus_trade_counter(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        pai(self, g, p, |s| s.bonus_trade_counter[index.idx()] += change);
    }

    /// C++ `int CvPlayerAI::AI_getPeacetimeTradeValue(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11663-11668)
    pub fn player_ai_get_peacetime_trade_value(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        g.players[p].ai_peacetime_trade_value[index]
    }

    /// C++ `void CvPlayerAI::AI_changePeacetimeTradeValue(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11671-11720)
    pub fn player_ai_change_peacetime_trade_value(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        // ffh-core keeps this member and its side effect on the enemies' grudges (BETTER AI).
        g.player_ai_change_peacetime_trade_value(p, index, change);
    }

    /// C++ `int CvPlayerAI::AI_getPeacetimeGrantValue(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11723-11728)
    pub fn player_ai_get_peacetime_grant_value(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        g.players[p].ai_peacetime_grant_value[index]
    }

    /// C++ `void CvPlayerAI::AI_changePeacetimeGrantValue(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11731-11780)
    pub fn player_ai_change_peacetime_grant_value(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        g.player_ai_change_peacetime_grant_value(p, index, change);
    }

    /// C++ `int CvPlayerAI::AI_getGoldTradedTo(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11783-11788)
    pub fn player_ai_get_gold_traded_to(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        g.players[p].ai_gold_traded_to[index]
    }

    /// C++ `void CvPlayerAI::AI_changeGoldTradedTo(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11791-11797)
    pub fn player_ai_change_gold_traded_to(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        g.players[p].ai_gold_traded_to[index] += change;
        debug_assert!(g.players[p].ai_gold_traded_to[index] >= 0);
    }

    /// C++ `int CvPlayerAI::AI_getAttitudeExtra(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11800-11805)
    pub fn player_ai_get_attitude_extra(&self, g: &Game, p: PlayerId, index: PlayerId) -> i32 {
        g.players[p].ai_attitude_extra[index]
    }

    /// C++ `void CvPlayerAI::AI_setAttitudeExtra(PlayerTypes eIndex, int iNewValue)` (CvPlayerAI.cpp:11808-11813)
    pub fn player_ai_set_attitude_extra(&self, g: &mut Game, p: PlayerId, index: PlayerId, new_value: i32) {
        g.players[p].ai_attitude_extra[index] = new_value;
    }

    /// C++ `void CvPlayerAI::AI_changeAttitudeExtra(PlayerTypes eIndex, int iChange)` (CvPlayerAI.cpp:11816-11819)
    pub fn player_ai_change_attitude_extra(&self, g: &mut Game, p: PlayerId, index: PlayerId, change: i32) {
        let v = self.player_ai_get_attitude_extra(g, p, index) + change;
        self.player_ai_set_attitude_extra(g, p, index, v);
    }

    /// C++ `bool CvPlayerAI::AI_isFirstContact(PlayerTypes eIndex) const` (CvPlayerAI.cpp:11822-11827)
    pub fn player_ai_is_first_contact(&self, g: &Game, p: PlayerId, index: PlayerId) -> bool {
        g.players[p].ai_first_contact[index]
    }

    /// C++ `void CvPlayerAI::AI_setFirstContact(PlayerTypes eIndex, bool bNewValue)` (CvPlayerAI.cpp:11830-11835)
    pub fn player_ai_set_first_contact(&self, g: &mut Game, p: PlayerId, index: PlayerId, new_value: bool) {
        g.players[p].ai_first_contact[index] = new_value;
    }

    /// C++ `int CvPlayerAI::AI_getContactTimer(PlayerTypes eIndex1, ContactTypes eIndex2) const` (CvPlayerAI.cpp:11838-11845)
    pub fn player_ai_get_contact_timer(&self, g: &Game, p: PlayerId, index1: PlayerId, index2: ContactType) -> i32 {
        pai(self, g, p, |s| s.contact_timer[index1.idx()][index2.idx()])
    }

    /// C++ `void CvPlayerAI::AI_changeContactTimer(PlayerTypes eIndex1, ContactTypes eIndex2, int iChange)` (CvPlayerAI.cpp:11848-11856)
    pub fn player_ai_change_contact_timer(&self, g: &mut Game, p: PlayerId, index1: PlayerId, index2: ContactType, change: i32) {
        pai(self, g, p, |s| s.contact_timer[index1.idx()][index2.idx()] += change);
    }

    /// C++ `int CvPlayerAI::AI_getMemoryCount(PlayerTypes eIndex1, MemoryTypes eIndex2) const` (CvPlayerAI.cpp:11859-11866)
    pub fn player_ai_get_memory_count(&self, g: &Game, p: PlayerId, index1: PlayerId, index2: MemoryType) -> i32 {
        g.players[p].ai_memory_count[index1][index2]
    }

    /// C++ `void CvPlayerAI::AI_changeMemoryCount(PlayerTypes eIndex1, MemoryTypes eIndex2, int iChange)` (CvPlayerAI.cpp:11869-11877)
    pub fn player_ai_change_memory_count(&self, g: &mut Game, p: PlayerId, index1: PlayerId, index2: MemoryType, change: i32) {
        g.players[p].ai_memory_count[index1][index2] += change;
        debug_assert!(g.players[p].ai_memory_count[index1][index2] >= 0);
    }

    /// C++ `int CvPlayerAI::AI_calculateGoldenAgeValue() const` (CvPlayerAI.cpp:11879-11898)
    pub fn player_ai_calculate_golden_age_value(&self, g: &Game, p: PlayerId) -> i32 {
        let mut value = 0;
        for i in 0..YieldType::NUM {
            let y = YieldType::from_usize(i);
            let yi = &g.d.yields[y];
            let mut temp_value = yi.golden_age_yield * self.player_ai_yield_weight(g, p, y);
            temp_value /= 1.max(1 + yi.golden_age_yield_threshold);
            value += temp_value;
        }

        value *= g.players[p].total_population;
        value *= g.golden_age_length();
        value /= 100;

        value
    }

    /// C++ `void CvPlayerAI::AI_doCounter()` (CvPlayerAI.cpp:11902-12006)
    pub fn player_ai_do_counter(&self, g: &mut Game, p: PlayerId) {
        let team = g.players[p].team;
        let leader = personality_info(g, p);
        let state_religion = g.player_get_state_religion(p);

        for other in g.player_ids() {
            if g.players[other].alive {
                let other_team = g.players[other].team;
                if other_team != team {
                    if g.teams[team].has_met[other_team] {
                        let other_state_religion = g.player_get_state_religion(other);

                        if state_religion.is_some() && state_religion == other_state_religion {
                            self.player_ai_change_same_religion_counter(g, p, other, 1);
                        } else if self.player_ai_get_same_religion_counter(g, p, other) > 0 {
                            self.player_ai_change_same_religion_counter(g, p, other, -1);
                        }

                        if state_religion.is_some() && other_state_religion.is_some() && state_religion != other_state_religion {
                            self.player_ai_change_different_religion_counter(g, p, other, 1);
                        } else if self.player_ai_get_different_religion_counter(g, p, other) > 0 {
                            self.player_ai_change_different_religion_counter(g, p, other, -1);
                        }

                        if let Some(favorite_civic) = leader.favorite_civic {
                            if g.player_is_civic(p, favorite_civic) && g.player_is_civic(other, favorite_civic) {
                                self.player_ai_change_favorite_civic_counter(g, p, other, 1);
                            } else if self.player_ai_get_favorite_civic_counter(g, p, other) > 0 {
                                self.player_ai_change_favorite_civic_counter(g, p, other, -1);
                            }
                        }

                        let bonus_imports = g.player_get_num_trade_bonus_imports(p, other);

                        if bonus_imports > 0 {
                            self.player_ai_change_bonus_trade_counter(g, p, other, bonus_imports);
                        } else {
                            let dec = self.player_ai_get_bonus_trade_counter(g, p, other).min((g.players[other].num_cities() / 4) + 1);
                            self.player_ai_change_bonus_trade_counter(g, p, other, -dec);
                        }
                    }
                }
            }
        }

        for other in g.player_ids() {
            if g.players[other].alive {
                for j in 0..ContactType::NUM {
                    let contact = ContactType::from_usize(j);
                    if self.player_ai_get_contact_timer(g, p, other, contact) > 0 {
                        self.player_ai_change_contact_timer(g, p, other, contact, -1);
                    }
                }
            }
        }

        for other in g.player_ids() {
            if g.players[other].alive {
                for j in 0..MemoryType::NUM {
                    let memory = MemoryType::from_usize(j);
                    if self.player_ai_get_memory_count(g, p, other, memory) > 0 {
                        let decay_rand = leader.memory_decay_rand[memory];
                        if decay_rand > 0 {
                            if self.rand(decay_rand, "Memory Decay") == 0 {
                                self.player_ai_change_memory_count(g, p, other, memory, -1);
                            }
                        }
                    }
                }
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_doMilitary()` (CvPlayerAI.cpp:12009-12027)
    pub fn player_ai_do_military(&self, g: &mut Game, p: PlayerId) {
        let team = g.players[p].team;
        if g.team_get_any_war_plan_count(team, true) == 0 {
            while self.player_ai_is_financial_trouble(g, p) && g.player_calculate_unit_cost(p) > 0 {
                if !self.player_ai_disband_unit(g, p, 1, false) {
                    break;
                }
            }
        }

        let leader = personality_info(g, p);
        // operands of `+`: left to right, #1 first.
        let r1 = self.rand(leader.attack_odds_change_rand, "AI Attack Odds Change #1");
        let r2 = self.rand(leader.attack_odds_change_rand, "AI Attack Odds Change #2");
        self.player_ai_set_attack_odds_change(g, p, leader.base_attack_odds_change + r1 + r2);
    }

    /// C++ `void CvPlayerAI::AI_doResearch()` (CvPlayerAI.cpp:12030-12039)
    pub fn player_ai_do_research(&self, g: &mut Game, p: PlayerId) {
        if g.player_get_current_research(p).is_none() {
            // AI_chooseResearch(): returns at once while research is disabled; otherwise it
            // clears the queue and pushes its choice (`player_ai_choose_research` returns it).
            if g.players[p].disable_research <= 0 {
                g.player_clear_research_queue(p);
                if let Some(tech) = self.player_ai_choose_research(g, p) {
                    g.tel_decision(p, "research", &g.d.techs[tech].base.type_, "", 0, String::new);
                    g.player_push_research(p, tech, false);
                }
            }
            self.player_ai_force_update_strategies(g, p); //to account for current research.
        }
    }

    /// C++ `void CvPlayerAI::AI_doCommerce()` (CvPlayerAI.cpp:12042-12294)
    pub fn player_ai_do_commerce(&self, g: &mut Game, p: PlayerId) {
        // BETTER_BTS_AI_MOD 07/20/09 jdog5000: Barbarian AI, efficiency
        if g.player_is_barbarian(p) {
            return;
        }

        let d = g.d;
        let team = g.players[p].team;
        let inc = d.defines.commerce_percent_change_increments;
        let research = CommerceType::COMMERCE_RESEARCH;
        let culture = CommerceType::COMMERCE_CULTURE;
        let espionage = CommerceType::COMMERCE_ESPIONAGE;
        let gold = CommerceType::COMMERCE_GOLD;
        let percent = |g: &Game, c: CommerceType| g.players[p].commerce_percent[c.idx()];
        // PORT ADAPT: the C++ loops below stop only on their own conditions; here they also stop
        // when a change no longer moves the rate (where the C++ would spin forever).
        let change = |g: &mut Game, c: CommerceType, delta: i32| -> bool {
            let before = g.players[p].commerce_percent[c.idx()];
            g.player_change_commerce_percent(p, c, delta);
            g.players[p].commerce_percent[c.idx()] != before
        };

        let mut gold_target = self.player_ai_gold_target(g, p);
        let mut target_turns = 4 * d.game_speeds[g.game_speed_type()].research_percent;
        target_turns /= 100;
        target_turns = 3.max(target_turns);

        if g.player_is_commerce_flexible(p, research) && !self.player_ai_avoid_science(g, p) {
            // set research rate to 100%
            g.player_set_commerce_percent(p, research, 100);

            // if the gold rate is under 0 at 90% research
            let gold_rate = g.player_calculate_gold_rate(p);
            if gold_rate < 0 {
                if let Some(current_research) = g.player_get_current_research(p) {
                    let research_turns_left = g.player_get_research_turns_left(p, current_research, true);

                    // if we can finish the current research without running out of gold, let us spend 2/3rds of our gold
                    if g.players[p].gold >= research_turns_left * gold_rate {
                        gold_target /= 3;
                    }
                }
            }
        }

        let mut reset = false;

        if g.player_is_commerce_flexible(p, culture) {
            if percent(g, culture) > 0 {
                g.player_set_commerce_percent(p, culture, 0);

                reset = true;
            }
        }

        if g.player_is_commerce_flexible(p, espionage) {
            if percent(g, espionage) > 0 {
                g.player_set_commerce_percent(p, espionage, 0);

                for t in g.civ_team_ids() {
                    g.player_set_espionage_spending_weight_against_team(p, t, 0);
                }

                reset = true;
            }
        }

        if reset {
            self.player_ai_assign_working_plots(g, p);
        }

        let first_tech = match g.player_get_current_research(p) {
            Some(tech) => self.player_ai_is_first_tech(g, p, tech),
            // C++ quirk: AI_isFirstTech(NO_TECH) matches religions without a tech prereq, then
            // reads the tech tables at -1; only the religion part is kept (PORT ADAPT).
            None => d.religions.keys().any(|r| d.religions[r].tech_prereq.is_none() && !g.religion_slot_taken[r]),
        };
        if g.player_is_commerce_flexible(p, culture) {
            if g.players[p].num_cities() > 0 {
                let mut ideal_percent = 0;

                for &c in &g.players[p].cities {
                    let happiness_per = g.cities[c].commerce_happiness_per[culture.idx()];
                    if happiness_per > 0 {
                        ideal_percent += (g.city_angry_population(c, 0) * 100) / happiness_per;
                    }
                }

                ideal_percent /= g.players[p].num_cities();

                ideal_percent -= ideal_percent % inc;

                ideal_percent = ideal_percent.min(20);

                if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE4) {
                    ideal_percent = 100;
                }

                g.player_set_commerce_percent(p, culture, ideal_percent);
            }
        }

        if g.player_is_commerce_flexible(p, research) {
            if (g.player_is_no_research_available(p) || self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE4)) && !first_tech {
                g.player_set_commerce_percent(p, research, 0);
            } else {
                while g.player_calculate_gold_rate(p) > 0 {
                    if !change(g, research, inc) {
                        break;
                    }

                    if percent(g, research) == 100 {
                        break;
                    }
                }

                if g.players[p].gold + target_turns * g.player_calculate_gold_rate(p) < gold_target {
                    while g.players[p].gold + target_turns * g.player_calculate_gold_rate(p) <= gold_target {
                        if !change(g, research, -inc) {
                            break;
                        }

                        if percent(g, research) == 0 {
                            break;
                        }
                    }
                } else if self.player_ai_avoid_science(g, p) {
                    change(g, research, -inc);
                }

                if g.team_get_chosen_war_count(team, true) > 0 || g.team_get_war_plan_count(team, WarPlanType::WARPLAN_ATTACKED_RECENT, true) > 0 {
                    change(g, research, -inc);
                }

                if percent(g, research) == 0 && g.player_calculate_gold_rate(p) > 0 {
                    g.player_set_commerce_percent(p, research, inc);
                }
            }
        }

        if g.player_is_commerce_flexible(p, espionage) && !first_tech {
            let mut espionage_target_rate = 0;

            for t in g.civ_team_ids() {
                if g.team_is_alive(t) && t != team && !g.teams[t].vassal[team] && !g.teams[team].vassal[t] {
                    let mut target = (g.teams[t].espionage_points_against_team[team] - g.teams[team].espionage_points_against_team[t]) / 8;

                    target -= self.team_ai_get_attitude_val(g, team, t, true);

                    if target > 0 {
                        espionage_target_rate += target;
                        g.player_change_espionage_spending_weight_against_team(p, t, target);
                    }
                }
            }

            //if economy is weak, neglect espionage spending.
            //instead invest hammers into espionage via spies/builds
            if self.player_ai_is_financial_trouble(g, p) || self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3) {
                //can still get trickle espionage income
                espionage_target_rate = 0;
            } else {
                espionage_target_rate *= 110 - percent(g, gold) * 2;
                espionage_target_rate /= 110;

                espionage_target_rate *= leader_info(g, p).espionage_weight;
                espionage_target_rate /= 100;

                let initial_research_percent = percent(g, research);

                while g.player_get_commerce_rate(p, espionage) < espionage_target_rate && percent(g, espionage) < 20 {
                    let moved_research = change(g, research, -inc);
                    let moved_espionage = change(g, espionage, inc);
                    if !moved_research && !moved_espionage {
                        break;
                    }

                    if g.players[p].gold + target_turns * g.player_calculate_gold_rate(p) < gold_target {
                        break;
                    }

                    if !self.player_ai_avoid_science(g, p) && !g.player_is_no_research_available(p) {
                        if percent(g, research) * 2 <= (percent(g, espionage) + inc) * 3 {
                            break;
                        }
                    }
                }
            }
        }

        if !first_tech && g.players[p].gold < gold_target && percent(g, research) > 40 {
            let mut hurry_gold = false;
            for h in d.hurries.keys() {
                if d.hurries[h].gold_per_production > 0 && g.player_can_hurry(p, h) {
                    hurry_gold = true;
                    break;
                }
            }
            if hurry_gold {
                if percent(g, espionage) > 0 {
                    change(g, espionage, -inc);
                } else {
                    change(g, research, -inc);
                }
                //changeCommercePercent(COMMERCE_GOLD, GC.getDefineINT("COMMERCE_PERCENT_CHANGE_INCREMENTS"));
            }
        }

        // this is called on doTurn, so make sure our gold is high enough keep us above zero gold.
        g.player_verify_gold_commerce_percent(p);
    }

    /// C++ `void CvPlayerAI::AI_doCivics()` (CvPlayerAI.cpp:12297-12337)
    pub fn player_ai_do_civics(&self, g: &mut Game, p: PlayerId) {
        if self.player_ai_get_civic_timer(g, p) > 0 {
            self.player_ai_change_civic_timer(g, p, -1);
            return;
        }

        if !g.player_can_revolution(p, None) {
            return;
        }

        let mut best_civics: IdxVec<CivicOptionType, Option<CivicType>> = IdxVec::from_elem(None, g.d.civic_options.len());

        for opt in g.d.civic_options.keys() {
            best_civics[opt] = self.player_ai_best_civic(g, p, opt);

            if best_civics[opt].is_none() {
                best_civics[opt] = g.players[p].civics[opt];
            }
        }

        // XXX AI skips revolution???
        if g.player_can_revolution(p, Some(&best_civics)) {
            g.player_revolution(p, &best_civics, false);
            let timer = if g.players[p].max_anarchy_turns == 0 { g.d.defines.min_revolution_turns * 2 } else { CIVIC_CHANGE_DELAY };
            self.player_ai_set_civic_timer(g, p, timer);
        }
    }

    /// C++ `void CvPlayerAI::AI_doReligion()` (CvPlayerAI.cpp:12340-12384)
    pub fn player_ai_do_religion(&self, g: &mut Game, p: PlayerId) {
        // BETTER_BTS_AI_MOD 07/20/09 jdog5000: Barbarian AI, efficiency
        if g.player_is_barbarian(p) {
            return;
        }

        if self.player_ai_get_religion_timer(g, p) > 0 {
            self.player_ai_change_religion_timer(g, p, -1);
            return;
        }

        if !g.player_can_change_religion(p) {
            return;
        }

        let mut best_religion = self.player_ai_best_religion(g, p);

        if best_religion.is_none() {
            best_religion = g.player_get_state_religion(p);
        }

        if g.player_can_convert(p, best_religion) {
            g.player_convert(p, best_religion);
            let timer = if g.players[p].max_anarchy_turns == 0 { g.d.defines.min_conversion_turns * 2 } else { RELIGION_CHANGE_DELAY };
            self.player_ai_set_religion_timer(g, p, timer);
        }
    }
}
