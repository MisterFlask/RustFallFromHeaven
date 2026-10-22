//! The C++ AI members that ffh-core does not keep.
//!
//! ffh-core already holds every AI member the rules read or write (and the ported AI code uses
//! those fields directly): `Player::ai_civic_timer`, `ai_religion_timer`, `ai_num_train_ai_units`,
//! `ai_num_ai_units`, `ai_peacetime_trade_value`, `ai_peacetime_grant_value`, `ai_gold_traded_to`,
//! `ai_attitude_extra`, `ai_first_contact`, `ai_memory_count`, `ai_bonus_value` (cells),
//! `sum_sui_mode` (summon suicide mode); `Team::ai_worst_enemy`, the `ai_*_counter`s,
//! `ai_war_success`, `ai_enemy_peacetime_*`, `ai_war_plan`; `City::ai_assign_work_dirty`,
//! `ai_choose_production_dirty`, `ai_emphasize*`; `Unit::ai_birthmark`, `ai_type`,
//! `ai_groupflag`, `ai_origin_x/y`, `ai_suicide_summon`, `ai_allowed_*`;
//! `SelectionGroup::ai_mission_ai_*`, `ai_force_separate`, `ai_group_attack*`;
//! `Area::area_ai_type`; `Plot::found_value`.
//!
//! The rest lives here, inside the controller (not saved: a game with this AI cannot be
//! resumed). Entries are created on first use with the C++ `AI_reset` values.

use std::collections::BTreeMap;

use ffh_core::ids::*;
use ffh_core::rng::Rng;
use ffh_data::*;

use crate::defs::*;

/// C++ `CvPlayerAI` members not in `ffh_core::player::Player`.
#[derive(Clone, Debug)]
pub struct PlayerAi {
    /// `AI_init` has run (peace weight, espionage weight, timers).
    pub initialized: bool,
    pub peace_weight: i32,
    pub espionage_weight: i32,
    pub attack_odds_change: i32,
    pub extra_gold_target: i32,
    pub strategy_hash: i32,
    pub strategy_hash_cache_turn: i32,
    pub averages_cache_turn: i32,
    pub average_great_people_multiplier: i32,
    pub average_yield_multiplier: [i32; YieldType::NUM],
    pub average_commerce_multiplier: [i32; CommerceType::NUM],
    pub average_commerce_exchange: [i32; CommerceType::NUM],
    pub upgrade_units_cache_turn: i32,
    pub upgrade_units_cached_exp_threshold: i32,
    pub upgrade_units_cached_gold: i32,
    /// By player id (sized on first use to `MAX_PLAYERS`-like bounds: see [`PlayerAi::new`]).
    pub same_religion_counter: Vec<i32>,
    pub different_religion_counter: Vec<i32>,
    pub favorite_civic_counter: Vec<i32>,
    pub bonus_trade_counter: Vec<i32>,
    /// `m_iExpansionX/Y` (BETTER AI naval): -1 = none.
    pub expansion_x: i32,
    pub expansion_y: i32,
    /// By unit class / unit combat.
    pub unit_class_weights: Vec<i32>,
    pub unit_combat_weights: Vec<i32>,
    /// By player id.
    pub close_borders_attitude_cache: Vec<i32>,
    /// `m_aaiContactTimer[player][contact]`.
    pub contact_timer: Vec<Vec<i32>>,
    /// `m_aiAICitySites`: plot indices.
    pub ai_city_sites: Vec<PlotIdx>,
    pub was_financial_trouble: bool,
    pub turn_last_production_dirty: i32,
}

impl PlayerAi {
    pub fn new(d: &GameData, num_players: usize) -> PlayerAi {
        PlayerAi {
            initialized: false,
            peace_weight: 0,
            espionage_weight: 0,
            attack_odds_change: 0,
            extra_gold_target: 0,
            strategy_hash: 0,
            strategy_hash_cache_turn: -1,
            averages_cache_turn: -1,
            average_great_people_multiplier: 0,
            average_yield_multiplier: [0; YieldType::NUM],
            average_commerce_multiplier: [0; CommerceType::NUM],
            average_commerce_exchange: [0; CommerceType::NUM],
            upgrade_units_cache_turn: -1,
            upgrade_units_cached_exp_threshold: 0,
            upgrade_units_cached_gold: 0,
            same_religion_counter: vec![0; num_players],
            different_religion_counter: vec![0; num_players],
            favorite_civic_counter: vec![0; num_players],
            bonus_trade_counter: vec![0; num_players],
            expansion_x: -1,
            expansion_y: -1,
            unit_class_weights: vec![0; d.unit_classes.len()],
            unit_combat_weights: vec![0; d.unit_combats.len()],
            close_borders_attitude_cache: vec![0; num_players],
            contact_timer: vec![vec![0; ContactType::NUM]; num_players],
            ai_city_sites: Vec::new(),
            was_financial_trouble: false,
            turn_last_production_dirty: -1,
        }
    }
}

/// C++ `CvCityAI` members not in `ffh_core::city::City`.
#[derive(Clone, Debug)]
pub struct CityAi {
    /// `m_routeToCity`.
    pub route_to_city: Option<CityId>,
    pub force_emphasize_culture: bool,
    /// `m_aiBestBuildValue[NUM_CITY_PLOTS]`, `m_aeBestBuild[NUM_CITY_PLOTS]`.
    pub best_build_value: [i32; NUM_CITY_PLOTS],
    pub best_build: [Option<BuildType>; NUM_CITY_PLOTS],
    pub special_yield_multiplier: [i32; YieldType::NUM],
    pub cache_player_closeness_turn: i32,
    pub cache_player_closeness_distance: i32,
    /// By player id.
    pub player_closeness: Vec<i32>,
    pub needed_floating_defenders: i32,
    pub needed_floating_defenders_cache_turn: i32,
    pub workers_needed: i32,
    pub workers_have: i32,
    pub permdefender_needed: [i32; NUM_PERMDEFENDER_TYPES],
    pub patrol_needed: [i32; NUM_PATROL_TYPES],
    pub city_defense_production_needed: [i32; NUM_CITYDEFENSE_TIERS],
    pub patrol_production_needed: [i32; NUM_PATROL_TIERS],
    /// `m_iAI_CityPatrolgroupOrigin_X/Y`, `m_iAI_CityDefenseProduction_X/Y`,
    /// `m_iAI_CityPatrolProduction_X/Y` (-1 = none; C++ `AI_reset` leaves the last pair
    /// unset, here -1 too).
    pub city_patrolgroup_origin_x: i32,
    pub city_patrolgroup_origin_y: i32,
    pub city_defense_production_x: i32,
    pub city_defense_production_y: i32,
    pub city_patrol_production_x: i32,
    pub city_patrol_production_y: i32,
}

impl CityAi {
    pub fn new(num_players: usize) -> CityAi {
        CityAi {
            route_to_city: None,
            force_emphasize_culture: false,
            // C++ quirk: AI_reset fills m_aiBestBuildValue with NO_BUILD (-1).
            best_build_value: [-1; NUM_CITY_PLOTS],
            best_build: [None; NUM_CITY_PLOTS],
            special_yield_multiplier: [0; YieldType::NUM],
            cache_player_closeness_turn: -1,
            cache_player_closeness_distance: -1,
            player_closeness: vec![0; num_players],
            needed_floating_defenders: -1,
            needed_floating_defenders_cache_turn: -1,
            workers_needed: 0,
            workers_have: 0,
            permdefender_needed: [0; NUM_PERMDEFENDER_TYPES],
            patrol_needed: [0; NUM_PATROL_TYPES],
            city_defense_production_needed: [0; NUM_CITYDEFENSE_TIERS],
            patrol_production_needed: [0; NUM_PATROL_TIERS],
            city_patrolgroup_origin_x: -1,
            city_patrolgroup_origin_y: -1,
            city_defense_production_x: -1,
            city_defense_production_y: -1,
            city_patrol_production_x: -1,
            city_patrol_production_y: -1,
        }
    }
}

/// C++ `CvUnitAI` members not in `ffh_core::unit::Unit`.
#[derive(Clone, Debug, Default)]
pub struct UnitAi {
    pub automated_abort_turn: i32,
    /// `m_bPermanentSummon`.
    pub permanent_summon: bool,
}

/// All C++ AI state of one game that ffh-core does not keep.
pub struct State {
    /// The AI's random numbers (C++ `getSorenRandNum` calls made by AI code): a stream of its
    /// own so the rules' stream is the same whichever AI plays (seeded from the game seed).
    pub rng: Rng,
    pub num_players: usize,
    pub players: Vec<PlayerAi>,
    pub cities: BTreeMap<CityId, CityAi>,
    pub units: BTreeMap<UnitId, UnitAi>,
    d: &'static GameData,
}

impl State {
    pub fn new(d: &'static GameData, seed: u32) -> State {
        State { rng: Rng::new(seed), num_players: 0, players: Vec::new(), cities: BTreeMap::new(), units: BTreeMap::new(), d }
    }

    /// Sizes the per-player tables (called with the game's player count before any access).
    pub fn ensure_players(&mut self, n: usize) {
        if self.num_players < n {
            self.num_players = n;
            for p in &mut self.players {
                p.same_religion_counter.resize(n, 0);
                p.different_religion_counter.resize(n, 0);
                p.favorite_civic_counter.resize(n, 0);
                p.bonus_trade_counter.resize(n, 0);
                p.close_borders_attitude_cache.resize(n, 0);
                p.contact_timer.resize(n, vec![0; ContactType::NUM]);
            }
        }
        while self.players.len() < n {
            self.players.push(PlayerAi::new(self.d, n));
        }
    }

    pub fn player(&mut self, p: PlayerId) -> &mut PlayerAi {
        let i = p.idx();
        if i >= self.players.len() {
            self.ensure_players(i + 1);
        }
        &mut self.players[i]
    }

    pub fn city(&mut self, c: CityId) -> &mut CityAi {
        let n = self.num_players;
        self.cities.entry(c).or_insert_with(|| CityAi::new(n))
    }

    pub fn unit(&mut self, u: UnitId) -> &mut UnitAi {
        self.units.entry(u).or_default()
    }
}
