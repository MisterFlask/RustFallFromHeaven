//! Info classes: CommerceInfo, GameSpeedInfo, WorldInfo, ClimateInfo, SeaLevelInfo, EraInfo,
//! HandicapInfo, GameOptionInfo, GoodyInfo, HurryInfo, UpkeepInfo, CultureLevelInfo,
//! VictoryInfo, ProcessInfo, EmphasizeInfo, DiplomacyInfo.
//! Port of the corresponding `Cv*Info::read(CvXMLLoadUtility*)` functions in CvInfos.cpp.
//! None of these classes has a `readPass2`/`readPass3`.
//!
//! Not ported (no gameplay or AI effect):
//! - members read() never sets: `CvCommerceInfo::m_iChar` (font glyph, set at runtime),
//!   `CvGameSpeedInfo::m_szGameSpeedName`, `CvHandicapInfo::m_szHandicapName`;
//! - audio/visual members: `CvGoodyInfo::m_szSound`, `CvVictoryInfo::m_szMovie`, and the
//!   `CvEraInfo` soundtrack/soundscape members (`iSoundtrackSpace`, `bFirstSoundtrackFirst`,
//!   `EraInfoSoundtracks`, `CitySoundscapes`, `AudioUnitVictoryScript`, `AudioUnitDefeatScript`).
//!
//! The XML also carries `iCulturePercent` in GameSpeed and Era infos; the FFH2 0.41 DLL never
//! reads it, so it is ignored here as well.

use crate::enums::*;
use crate::idx::IdxVec;
use crate::infos::InfoBase;
use crate::registry::Rd;

// ---------------------------------------------------------------------------------------------
// CommerceInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvCommerceInfo` (commerces.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CommerceInfo {
    pub base: InfoBase,
    pub initial_percent: i32,
    pub initial_happiness: i32,
    /// C++ `getAIWeightPercent()`
    pub ai_weight_percent: i32,
    /// C++ `isFlexiblePercent()`
    pub flexible_percent: bool,
}

/// C++ `CvCommerceInfo::read`.
pub fn read_commerce(rd: &Rd) -> CommerceInfo {
    CommerceInfo {
        base: InfoBase::read(rd),
        initial_percent: rd.int("initial_percent"),
        initial_happiness: rd.int("initial_happiness"),
        ai_weight_percent: rd.int("ai_weight_percent"),
        flexible_percent: rd.bool("flexible_percent"),
    }
}

// ---------------------------------------------------------------------------------------------
// GameSpeedInfo
// ---------------------------------------------------------------------------------------------

/// C++ `GameTurnInfo` (CvStructs.h): one calendar increment of a game speed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameTurnInfo {
    /// C++ `iMonthIncrement`
    pub month_increment: i32,
    /// C++ `iNumGameTurnsPerIncrement` (XML `iTurnsPerIncrement`)
    pub num_game_turns_per_increment: i32,
}

/// C++ `CvGameSpeedInfo` (game_speeds.jsonc)
#[derive(Debug, Clone, Default)]
pub struct GameSpeedInfo {
    pub base: InfoBase,
    pub growth_percent: i32,
    pub train_percent: i32,
    pub construct_percent: i32,
    pub create_percent: i32,
    pub research_percent: i32,
    pub build_percent: i32,
    pub improvement_percent: i32,
    pub great_people_percent: i32,
    pub anarchy_percent: i32,
    pub barb_percent: i32,
    pub feature_production_percent: i32,
    pub unit_discover_percent: i32,
    pub unit_hurry_percent: i32,
    pub unit_trade_percent: i32,
    pub unit_great_work_percent: i32,
    pub golden_age_percent: i32,
    pub hurry_percent: i32,
    pub hurry_conscript_anger_percent: i32,
    pub inflation_offset: i32,
    pub inflation_percent: i32,
    pub victory_delay_percent: i32,
    /// C++ `getGameTurnInfo(i)` for `i < getNumTurnIncrements()`, in XML order
    /// (`m_pGameTurnInfo` / `m_iNumTurnIncrements`).
    pub game_turn_info: Vec<GameTurnInfo>,
}

impl GameSpeedInfo {
    /// C++ `getNumTurnIncrements()`.
    pub fn num_turn_increments(&self) -> i32 {
        self.game_turn_info.len() as i32
    }
}

/// C++ `CvGameSpeedInfo::read`.
pub fn read_game_speed(rd: &Rd) -> GameSpeedInfo {
    GameSpeedInfo {
        base: InfoBase::read(rd),
        growth_percent: rd.int("growth_percent"),
        train_percent: rd.int("train_percent"),
        construct_percent: rd.int("construct_percent"),
        create_percent: rd.int("create_percent"),
        research_percent: rd.int("research_percent"),
        build_percent: rd.int("build_percent"),
        improvement_percent: rd.int("improvement_percent"),
        great_people_percent: rd.int("great_people_percent"),
        anarchy_percent: rd.int("anarchy_percent"),
        barb_percent: rd.int("barb_percent"),
        feature_production_percent: rd.int("feature_production_percent"),
        unit_discover_percent: rd.int("unit_discover_percent"),
        unit_hurry_percent: rd.int("unit_hurry_percent"),
        unit_trade_percent: rd.int("unit_trade_percent"),
        unit_great_work_percent: rd.int("unit_great_work_percent"),
        golden_age_percent: rd.int("golden_age_percent"),
        hurry_percent: rd.int("hurry_percent"),
        hurry_conscript_anger_percent: rd.int("hurry_conscript_anger_percent"),
        inflation_offset: rd.int("inflation_offset"),
        inflation_percent: rd.int("inflation_percent"),
        victory_delay_percent: rd.int("victory_delay_percent"),
        game_turn_info: read_game_turn_infos(rd),
    }
}

/// The `GameTurnInfos` block of `CvGameSpeedInfo::read`: one increment per child element, whose
/// `iMonthIncrement` / `iTurnsPerIncrement` are looked up by name (default 0). No block -> none.
fn read_game_turn_infos(rd: &Rd) -> Vec<GameTurnInfo> {
    match rd.child("game_turn_infos") {
        Some(list) => list
            .children()
            .map(|t| GameTurnInfo {
                month_increment: t.int("month_increment"),
                num_game_turns_per_increment: t.int("turns_per_increment"),
            })
            .collect(),
        None => Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// WorldInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvWorldInfo` (worlds.jsonc), indexed by `WorldSizeType`.
#[derive(Debug, Clone, Default)]
pub struct WorldInfo {
    pub base: InfoBase,
    pub default_players: i32,
    pub unit_name_modifier: i32,
    pub target_num_cities: i32,
    pub num_free_building_bonuses: i32,
    pub building_class_prereq_modifier: i32,
    pub max_conscript_modifier: i32,
    pub war_weariness_modifier: i32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub terrain_grain_change: i32,
    pub feature_grain_change: i32,
    pub research_percent: i32,
    pub trade_profit_percent: i32,
    pub distance_maintenance_percent: i32,
    pub num_cities_maintenance_percent: i32,
    pub colony_maintenance_percent: i32,
    pub corporation_maintenance_percent: i32,
    pub num_cities_anarchy_percent: i32,
    pub advanced_start_points_mod: i32,
    /// FfH: C++ `getMaxCitiesMod()`
    pub max_cities_mod: i32,
    /// FfH: C++ `getUniqueFeatureChance()`
    pub unique_feature_chance: i32,
}

/// C++ `CvWorldInfo::read`.
pub fn read_world(rd: &Rd) -> WorldInfo {
    WorldInfo {
        base: InfoBase::read(rd),
        default_players: rd.int("default_players"),
        unit_name_modifier: rd.int("unit_name_modifier"),
        target_num_cities: rd.int("target_num_cities"),
        num_free_building_bonuses: rd.int("num_free_building_bonuses"),
        building_class_prereq_modifier: rd.int("building_class_prereq_modifier"),
        max_conscript_modifier: rd.int("max_conscript_modifier"),
        war_weariness_modifier: rd.int("war_weariness_modifier"),
        grid_width: rd.int("grid_width"),
        grid_height: rd.int("grid_height"),
        terrain_grain_change: rd.int("terrain_grain_change"),
        feature_grain_change: rd.int("feature_grain_change"),
        research_percent: rd.int("research_percent"),
        trade_profit_percent: rd.int("trade_profit_percent"),
        distance_maintenance_percent: rd.int("distance_maintenance_percent"),
        num_cities_maintenance_percent: rd.int("num_cities_maintenance_percent"),
        colony_maintenance_percent: rd.int("colony_maintenance_percent"),
        corporation_maintenance_percent: rd.int("corporation_maintenance_percent"),
        num_cities_anarchy_percent: rd.int("num_cities_anarchy_percent"),
        advanced_start_points_mod: rd.int("advanced_start_points_mod"),
        max_cities_mod: rd.int("max_cities_mod"),
        unique_feature_chance: rd.int("unique_feature_chance"),
    }
}

// ---------------------------------------------------------------------------------------------
// ClimateInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvClimateInfo` (climates.jsonc). The latitude changes are C++ `float`s
/// (used by map generation), so they stay `f32`.
#[derive(Debug, Clone, Default)]
pub struct ClimateInfo {
    pub base: InfoBase,
    pub desert_percent_change: i32,
    pub jungle_latitude: i32,
    pub hill_range: i32,
    pub peak_percent: i32,
    pub snow_latitude_change: f32,
    pub tundra_latitude_change: f32,
    pub grass_latitude_change: f32,
    pub desert_bottom_latitude_change: f32,
    pub desert_top_latitude_change: f32,
    pub ice_latitude: f32,
    pub rand_ice_latitude: f32,
}

/// C++ `CvClimateInfo::read`.
pub fn read_climate(rd: &Rd) -> ClimateInfo {
    ClimateInfo {
        base: InfoBase::read(rd),
        desert_percent_change: rd.int("desert_percent_change"),
        jungle_latitude: rd.int("jungle_latitude"),
        hill_range: rd.int("hill_range"),
        peak_percent: rd.int("peak_percent"),
        snow_latitude_change: rd.float("snow_latitude_change"),
        tundra_latitude_change: rd.float("tundra_latitude_change"),
        grass_latitude_change: rd.float("grass_latitude_change"),
        desert_bottom_latitude_change: rd.float("desert_bottom_latitude_change"),
        desert_top_latitude_change: rd.float("desert_top_latitude_change"),
        ice_latitude: rd.float("ice_latitude"),
        rand_ice_latitude: rd.float("rand_ice_latitude"),
    }
}

// ---------------------------------------------------------------------------------------------
// SeaLevelInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvSeaLevelInfo` (sea_levels.jsonc)
#[derive(Debug, Clone, Default)]
pub struct SeaLevelInfo {
    pub base: InfoBase,
    pub sea_level_change: i32,
}

/// C++ `CvSeaLevelInfo::read`.
pub fn read_sea_level(rd: &Rd) -> SeaLevelInfo {
    SeaLevelInfo { base: InfoBase::read(rd), sea_level_change: rd.int("sea_level_change") }
}

// ---------------------------------------------------------------------------------------------
// EraInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvEraInfo` (eras.jsonc). Soundtrack/soundscape/audio members not ported.
#[derive(Debug, Clone, Default)]
pub struct EraInfo {
    pub base: InfoBase,
    pub starting_unit_multiplier: i32,
    pub starting_defense_units: i32,
    pub starting_explore_units: i32,
    pub advanced_start_points: i32,
    pub starting_gold: i32,
    pub free_population: i32,
    pub start_percent: i32,
    pub growth_percent: i32,
    pub train_percent: i32,
    pub construct_percent: i32,
    pub create_percent: i32,
    pub research_percent: i32,
    pub build_percent: i32,
    pub improvement_percent: i32,
    pub great_people_percent: i32,
    pub anarchy_percent: i32,
    pub event_chance_per_turn: i32,
    /// C++ `isNoGoodies()`
    pub no_goodies: bool,
    /// C++ `isNoAnimals()`
    pub no_animals: bool,
    /// C++ `isNoBarbUnits()`
    pub no_barb_units: bool,
    /// C++ `isNoBarbCities()`
    pub no_barb_cities: bool,
}

/// C++ `CvEraInfo::read` (minus the audio tags).
pub fn read_era(rd: &Rd) -> EraInfo {
    EraInfo {
        base: InfoBase::read(rd),
        no_goodies: rd.bool("no_goodies"),
        no_animals: rd.bool("no_animals"),
        no_barb_units: rd.bool("no_barb_units"),
        no_barb_cities: rd.bool("no_barb_cities"),
        starting_unit_multiplier: rd.int("starting_unit_multiplier"),
        starting_defense_units: rd.int("starting_defense_units"),
        starting_explore_units: rd.int("starting_explore_units"),
        advanced_start_points: rd.int("advanced_start_points"),
        starting_gold: rd.int("starting_gold"),
        free_population: rd.int("free_population"),
        start_percent: rd.int("start_percent"),
        growth_percent: rd.int("growth_percent"),
        train_percent: rd.int("train_percent"),
        construct_percent: rd.int("construct_percent"),
        create_percent: rd.int("create_percent"),
        research_percent: rd.int("research_percent"),
        build_percent: rd.int("build_percent"),
        improvement_percent: rd.int("improvement_percent"),
        great_people_percent: rd.int("great_people_percent"),
        anarchy_percent: rd.int("anarchy_percent"),
        event_chance_per_turn: rd.int("event_chance_per_turn"),
    }
}

// ---------------------------------------------------------------------------------------------
// HandicapInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvHandicapInfo` (handicaps.jsonc). Several XML tags differ from the
/// getter names; the tag is noted where it does.
#[derive(Debug, Clone, Default)]
pub struct HandicapInfo {
    pub base: InfoBase,
    pub free_wins_vs_barbs: i32,
    pub animal_attack_prob: i32,
    /// C++ `getStartingLocationPercent()` (XML `iStartingLocPercent`)
    pub starting_location_percent: i32,
    pub advanced_start_points_mod: i32,
    /// C++ `getStartingGold()` (XML `iGold`)
    pub starting_gold: i32,
    pub free_units: i32,
    pub unit_cost_percent: i32,
    pub research_percent: i32,
    pub distance_maintenance_percent: i32,
    pub num_cities_maintenance_percent: i32,
    pub max_num_cities_maintenance: i32,
    pub colony_maintenance_percent: i32,
    pub max_colony_maintenance: i32,
    pub corporation_maintenance_percent: i32,
    pub civic_upkeep_percent: i32,
    pub inflation_percent: i32,
    pub health_bonus: i32,
    pub happy_bonus: i32,
    pub attitude_change: i32,
    pub no_tech_trade_modifier: i32,
    pub tech_trade_known_modifier: i32,
    pub unowned_tiles_per_game_animal: i32,
    pub unowned_tiles_per_barbarian_unit: i32,
    pub unowned_water_tiles_per_barbarian_unit: i32,
    pub unowned_tiles_per_barbarian_city: i32,
    pub barbarian_creation_turns_elapsed: i32,
    pub barbarian_city_creation_turns_elapsed: i32,
    pub barbarian_city_creation_prob: i32,
    /// C++ `getAnimalCombatModifier()` (XML `iAnimalBonus`)
    pub animal_combat_modifier: i32,
    /// C++ `getBarbarianCombatModifier()` (XML `iBarbarianBonus`)
    pub barbarian_combat_modifier: i32,
    /// C++ `getAIAnimalCombatModifier()` (XML `iAIAnimalBonus`)
    pub ai_animal_combat_modifier: i32,
    /// C++ `getAIBarbarianCombatModifier()` (XML `iAIBarbarianBonus`)
    pub ai_barbarian_combat_modifier: i32,
    pub starting_defense_units: i32,
    pub starting_explore_units: i32,
    /// C++ `getAIStartingUnitMultiplier()`
    pub ai_starting_unit_multiplier: i32,
    /// C++ `getAIStartingDefenseUnits()`
    pub ai_starting_defense_units: i32,
    /// C++ `getAIStartingExploreUnits()`
    pub ai_starting_explore_units: i32,
    /// C++ `getBarbarianInitialDefenders()` (XML `iBarbarianDefenders`)
    pub barbarian_initial_defenders: i32,
    /// C++ `getAIDeclareWarProb()`
    pub ai_declare_war_prob: i32,
    /// C++ `getAIWorkRateModifier()`
    pub ai_work_rate_modifier: i32,
    /// C++ `getAIGrowthPercent()`
    pub ai_growth_percent: i32,
    /// C++ `getAITrainPercent()`
    pub ai_train_percent: i32,
    /// C++ `getAIWorldTrainPercent()`
    pub ai_world_train_percent: i32,
    /// C++ `getAIConstructPercent()`
    pub ai_construct_percent: i32,
    /// C++ `getAIWorldConstructPercent()`
    pub ai_world_construct_percent: i32,
    /// C++ `getAICreatePercent()`
    pub ai_create_percent: i32,
    /// C++ `getAIWorldCreatePercent()`
    pub ai_world_create_percent: i32,
    /// C++ `getAICivicUpkeepPercent()`
    pub ai_civic_upkeep_percent: i32,
    /// C++ `getAIUnitCostPercent()`
    pub ai_unit_cost_percent: i32,
    /// C++ `getAIUnitSupplyPercent()`
    pub ai_unit_supply_percent: i32,
    /// C++ `getAIUnitUpgradePercent()`
    pub ai_unit_upgrade_percent: i32,
    /// C++ `getAIInflationPercent()`
    pub ai_inflation_percent: i32,
    /// C++ `getAIWarWearinessPercent()`
    pub ai_war_weariness_percent: i32,
    /// C++ `getAIPerEraModifier()`
    pub ai_per_era_modifier: i32,
    /// C++ `getAIAdvancedStartPercent()`
    pub ai_advanced_start_percent: i32,
    /// FfH: C++ `getAIFreeXP()`
    pub ai_free_xp: i32,
    /// FfH: C++ `getLairSpawnRate()`
    pub lair_spawn_rate: i32,
    /// C++ `getGoodies(i)` for `i < getNumGoodies()`, in XML order. Duplicates are kept: they
    /// weight the random pick in `CvPlayer::doGoody`.
    pub goodies: Vec<GoodyType>,
    /// C++ `isFreeTechs(i)`
    pub free_techs: IdxVec<TechType, bool>,
    /// C++ `isAIFreeTechs(i)`
    pub ai_free_techs: IdxVec<TechType, bool>,
}

impl HandicapInfo {
    /// C++ `getNumGoodies()`.
    pub fn num_goodies(&self) -> i32 {
        self.goodies.len() as i32
    }
}

/// C++ `CvHandicapInfo::read`.
pub fn read_handicap(rd: &Rd) -> HandicapInfo {
    HandicapInfo {
        base: InfoBase::read(rd),
        free_wins_vs_barbs: rd.int("free_wins_vs_barbs"),
        animal_attack_prob: rd.int("animal_attack_prob"),
        starting_location_percent: rd.int("starting_loc_percent"),
        advanced_start_points_mod: rd.int("advanced_start_points_mod"),
        starting_gold: rd.int("gold"),
        free_units: rd.int("free_units"),
        unit_cost_percent: rd.int("unit_cost_percent"),
        research_percent: rd.int("research_percent"),
        distance_maintenance_percent: rd.int("distance_maintenance_percent"),
        num_cities_maintenance_percent: rd.int("num_cities_maintenance_percent"),
        max_num_cities_maintenance: rd.int("max_num_cities_maintenance"),
        colony_maintenance_percent: rd.int("colony_maintenance_percent"),
        max_colony_maintenance: rd.int("max_colony_maintenance"),
        corporation_maintenance_percent: rd.int("corporation_maintenance_percent"),
        civic_upkeep_percent: rd.int("civic_upkeep_percent"),
        inflation_percent: rd.int("inflation_percent"),
        health_bonus: rd.int("health_bonus"),
        happy_bonus: rd.int("happy_bonus"),
        attitude_change: rd.int("attitude_change"),
        no_tech_trade_modifier: rd.int("no_tech_trade_modifier"),
        tech_trade_known_modifier: rd.int("tech_trade_known_modifier"),
        unowned_tiles_per_game_animal: rd.int("unowned_tiles_per_game_animal"),
        unowned_tiles_per_barbarian_unit: rd.int("unowned_tiles_per_barbarian_unit"),
        unowned_water_tiles_per_barbarian_unit: rd.int("unowned_water_tiles_per_barbarian_unit"),
        unowned_tiles_per_barbarian_city: rd.int("unowned_tiles_per_barbarian_city"),
        barbarian_creation_turns_elapsed: rd.int("barbarian_creation_turns_elapsed"),
        barbarian_city_creation_turns_elapsed: rd.int("barbarian_city_creation_turns_elapsed"),
        barbarian_city_creation_prob: rd.int("barbarian_city_creation_prob"),
        animal_combat_modifier: rd.int("animal_bonus"),
        barbarian_combat_modifier: rd.int("barbarian_bonus"),
        ai_animal_combat_modifier: rd.int("ai_animal_bonus"),
        ai_barbarian_combat_modifier: rd.int("ai_barbarian_bonus"),
        starting_defense_units: rd.int("starting_defense_units"),
        starting_explore_units: rd.int("starting_explore_units"),
        ai_starting_unit_multiplier: rd.int("ai_starting_unit_multiplier"),
        ai_starting_defense_units: rd.int("ai_starting_defense_units"),
        ai_starting_explore_units: rd.int("ai_starting_explore_units"),
        barbarian_initial_defenders: rd.int("barbarian_defenders"),
        ai_declare_war_prob: rd.int("ai_declare_war_prob"),
        ai_work_rate_modifier: rd.int("ai_work_rate_modifier"),
        ai_growth_percent: rd.int("ai_growth_percent"),
        ai_train_percent: rd.int("ai_train_percent"),
        ai_world_train_percent: rd.int("ai_world_train_percent"),
        ai_construct_percent: rd.int("ai_construct_percent"),
        ai_world_construct_percent: rd.int("ai_world_construct_percent"),
        ai_create_percent: rd.int("ai_create_percent"),
        ai_world_create_percent: rd.int("ai_world_create_percent"),
        ai_civic_upkeep_percent: rd.int("ai_civic_upkeep_percent"),
        ai_unit_cost_percent: rd.int("ai_unit_cost_percent"),
        ai_unit_supply_percent: rd.int("ai_unit_supply_percent"),
        ai_unit_upgrade_percent: rd.int("ai_unit_upgrade_percent"),
        ai_inflation_percent: rd.int("ai_inflation_percent"),
        ai_war_weariness_percent: rd.int("ai_war_weariness_percent"),
        ai_per_era_modifier: rd.int("ai_per_era_modifier"),
        ai_advanced_start_percent: rd.int("ai_advanced_start_percent"),
        // C++: SetStringList over <Goodies>, then FindInInfoClass on every name. All names are
        // known GoodyInfo types (an unknown one is reported as a load warning and skipped).
        goodies: rd.ref_list::<GoodyType>("goodies"),
        free_techs: rd.pairs_bool::<TechType>("free_techs"),
        ai_free_techs: rd.pairs_bool::<TechType>("ai_free_techs"),
        ai_free_xp: rd.int("ai_free_xp"),
        lair_spawn_rate: rd.int("lair_spawn_rate"),
    }
}

// ---------------------------------------------------------------------------------------------
// GameOptionInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvGameOptionInfo` (game_options.jsonc). The table order must match the
/// hard-coded `GameOptionType` consts (checked at load).
#[derive(Debug, Clone, Default)]
pub struct GameOptionInfo {
    pub base: InfoBase,
    /// C++ `getDefault()`
    pub default: bool,
    /// C++ `getVisible()`
    pub visible: bool,
}

/// C++ `CvGameOptionInfo::read`.
pub fn read_game_option(rd: &Rd) -> GameOptionInfo {
    GameOptionInfo {
        base: InfoBase::read(rd),
        default: rd.bool("default"),
        // C++ quirk: the constructor sets m_bVisible = true, but read() calls
        // GetChildXmlValByName(&m_bVisible, "bVisible") whose default (false) overwrites it,
        // so an option without <bVisible> ends up hidden. Kept as is.
        visible: rd.bool("visible"),
    }
}

// ---------------------------------------------------------------------------------------------
// GoodyInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvGoodyInfo` (goodies.jsonc). `m_szSound` not ported.
#[derive(Debug, Clone, Default)]
pub struct GoodyInfo {
    pub base: InfoBase,
    pub gold: i32,
    pub gold_rand1: i32,
    pub gold_rand2: i32,
    pub map_offset: i32,
    pub map_range: i32,
    pub map_prob: i32,
    pub experience: i32,
    pub healing: i32,
    pub damage_prereq: i32,
    pub barbarian_unit_prob: i32,
    pub min_barbarians: i32,
    /// C++ `getUnitClassType()` (XML `UnitClass`): unit class granted.
    pub unit_class_type: Option<UnitClassType>,
    /// C++ `getBarbarianUnitClass()` (XML `BarbarianClass`): hostile unit class spawned.
    pub barbarian_unit_class: Option<UnitClassType>,
    /// C++ `isTech()`
    pub tech: bool,
    /// C++ `isBad()`
    pub bad: bool,
}

/// C++ `CvGoodyInfo::read` (minus `Sound`).
pub fn read_goody(rd: &Rd) -> GoodyInfo {
    GoodyInfo {
        base: InfoBase::read(rd),
        gold: rd.int("gold"),
        gold_rand1: rd.int("gold_rand1"),
        gold_rand2: rd.int("gold_rand2"),
        map_offset: rd.int("map_offset"),
        map_range: rd.int("map_range"),
        map_prob: rd.int("map_prob"),
        experience: rd.int("experience"),
        healing: rd.int("healing"),
        damage_prereq: rd.int("damage_prereq"),
        tech: rd.bool("tech"),
        bad: rd.bool("bad"),
        unit_class_type: rd.rf::<UnitClassType>("unit_class"),
        barbarian_unit_class: rd.rf::<UnitClassType>("barbarian_class"),
        barbarian_unit_prob: rd.int("barbarian_unit_prob"),
        min_barbarians: rd.int("min_barbarians"),
    }
}

// ---------------------------------------------------------------------------------------------
// HurryInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvHurryInfo` (hurries.jsonc)
#[derive(Debug, Clone, Default)]
pub struct HurryInfo {
    pub base: InfoBase,
    pub gold_per_production: i32,
    pub production_per_population: i32,
    /// C++ `isAnger()`
    pub anger: bool,
}

/// C++ `CvHurryInfo::read`.
pub fn read_hurry(rd: &Rd) -> HurryInfo {
    HurryInfo {
        base: InfoBase::read(rd),
        gold_per_production: rd.int("gold_per_production"),
        production_per_population: rd.int("production_per_population"),
        anger: rd.bool("anger"),
    }
}

// ---------------------------------------------------------------------------------------------
// UpkeepInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvUpkeepInfo` (upkeeps.jsonc)
#[derive(Debug, Clone, Default)]
pub struct UpkeepInfo {
    pub base: InfoBase,
    pub population_percent: i32,
    pub city_percent: i32,
}

/// C++ `CvUpkeepInfo::read`.
pub fn read_upkeep(rd: &Rd) -> UpkeepInfo {
    UpkeepInfo {
        base: InfoBase::read(rd),
        population_percent: rd.int("population_percent"),
        city_percent: rd.int("city_percent"),
    }
}

// ---------------------------------------------------------------------------------------------
// CultureLevelInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvCultureLevelInfo` (culture_levels.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CultureLevelInfo {
    pub base: InfoBase,
    pub city_defense_modifier: i32,
    /// C++ `getSpeedThreshold(eGameSpeed)`: culture needed for this level (default 0).
    pub speed_threshold: IdxVec<GameSpeedType, i32>,
}

/// C++ `CvCultureLevelInfo::read`.
pub fn read_culture_level(rd: &Rd) -> CultureLevelInfo {
    CultureLevelInfo {
        base: InfoBase::read(rd),
        city_defense_modifier: rd.int("city_defense_modifier"),
        speed_threshold: rd.pairs_int::<GameSpeedType>("speed_thresholds", 0),
    }
}

// ---------------------------------------------------------------------------------------------
// VictoryInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvVictoryInfo` (victories.jsonc). `m_szMovie` not ported.
#[derive(Debug, Clone, Default)]
pub struct VictoryInfo {
    pub base: InfoBase,
    pub population_percent_lead: i32,
    pub land_percent: i32,
    pub min_land_percent: i32,
    pub religion_percent: i32,
    /// C++ `getCityCulture()`: culture level each culture city must reach.
    pub city_culture: Option<CultureLevelType>,
    pub num_culture_cities: i32,
    pub total_culture_ratio: i32,
    pub victory_delay_turns: i32,
    /// C++ `isTargetScore()`
    pub target_score: bool,
    /// C++ `isEndScore()`
    pub end_score: bool,
    /// C++ `isConquest()`
    pub conquest: bool,
    /// C++ `isDiploVote()`
    pub diplo_vote: bool,
    /// C++ `isPermanent()`
    pub permanent: bool,
}

/// C++ `CvVictoryInfo::read` (minus `VictoryMovie`).
pub fn read_victory(rd: &Rd) -> VictoryInfo {
    VictoryInfo {
        base: InfoBase::read(rd),
        target_score: rd.bool("target_score"),
        end_score: rd.bool("end_score"),
        conquest: rd.bool("conquest"),
        diplo_vote: rd.bool("diplo_vote"),
        permanent: rd.bool("permanent"),
        population_percent_lead: rd.int("population_percent_lead"),
        land_percent: rd.int("land_percent"),
        min_land_percent: rd.int("min_land_percent"),
        religion_percent: rd.int("religion_percent"),
        city_culture: rd.rf::<CultureLevelType>("city_culture"),
        num_culture_cities: rd.int("num_culture_cities"),
        total_culture_ratio: rd.int("total_culture_ratio"),
        victory_delay_turns: rd.int("victory_delay_turns"),
    }
}

// ---------------------------------------------------------------------------------------------
// ProcessInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvProcessInfo` (processes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct ProcessInfo {
    pub base: InfoBase,
    pub tech_prereq: Option<TechType>,
    /// C++ `getProductionToCommerceModifier(eCommerce)`
    pub production_to_commerce_modifier: [i32; CommerceType::NUM],
    /// This game's (the Muster card, docs/CARDS.md): the hammers per point of experience the
    /// process gives the units in its city (`production_to_experience`; 0: none).
    pub production_to_experience: i32,
}

/// C++ `CvProcessInfo::read`.
pub fn read_process(rd: &Rd) -> ProcessInfo {
    ProcessInfo {
        base: InfoBase::read(rd),
        tech_prereq: rd.rf::<TechType>("tech_prereq"),
        // SetCommerce if the tag exists, else InitList(.., 0): zeros either way when absent.
        production_to_commerce_modifier: rd.commerces("production_to_commerce_modifiers"),
        production_to_experience: rd.int("production_to_experience"),
    }
}

// ---------------------------------------------------------------------------------------------
// EmphasizeInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvEmphasizeInfo` (emphasizes.jsonc): city governor emphasis.
#[derive(Debug, Clone, Default)]
pub struct EmphasizeInfo {
    pub base: InfoBase,
    /// C++ `isAvoidGrowth()`
    pub avoid_growth: bool,
    /// C++ `isGreatPeople()`
    pub great_people: bool,
    /// GrowthControl addon: C++ `isAvoidAngryCitizens()`
    pub avoid_angry_citizens: bool,
    /// GrowthControl addon: C++ `isAvoidUnhealthyCitizens()`
    pub avoid_unhealthy_citizens: bool,
    /// C++ `getYieldChange(eYield)` (member `m_piYieldModifiers`, XML `YieldModifiers`)
    pub yield_change: [i32; YieldType::NUM],
    /// C++ `getCommerceChange(eCommerce)` (member `m_piCommerceModifiers`, XML
    /// `CommerceModifiers`; the XML lists 3 values, espionage stays 0)
    pub commerce_change: [i32; CommerceType::NUM],
}

/// C++ `CvEmphasizeInfo::read`.
pub fn read_emphasize(rd: &Rd) -> EmphasizeInfo {
    EmphasizeInfo {
        base: InfoBase::read(rd),
        avoid_growth: rd.bool("avoid_growth"),
        great_people: rd.bool("great_people"),
        // SetYields / SetCommerce if the tag exists, else InitList(.., 0).
        yield_change: rd.yields("yield_modifiers"),
        commerce_change: rd.commerces("commerce_modifiers"),
        avoid_angry_citizens: rd.bool("avoid_angry_citizens"),
        avoid_unhealthy_citizens: rd.bool("avoid_unhealthy_citizens"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Ctx, Registry};

    /// Runs `read` on the entry object `json` against a registry holding only `classes`;
    /// returns the info and the load warnings.
    fn read_with<T>(classes: &[(&'static str, &[&str])], json: &str, read: fn(&Rd) -> T) -> (T, Vec<String>) {
        let mut reg = Registry::new();
        for (class, names) in classes {
            reg.register_class(class, names.iter().map(|s| s.to_string()).collect());
        }
        let value = crate::jsonc::parse(json).expect("test json");
        let cx = Ctx::new(&reg, "test.jsonc");
        let info = read(&Rd::new(&value, &cx));
        (info, cx.warnings.into_inner())
    }

    const SPEEDS: (&str, &[&str]) = ("GameSpeedInfo", &["GAMESPEED_A", "GAMESPEED_B"]);
    const TECHS: (&str, &[&str]) = ("TechInfo", &["TECH_A", "TECH_B", "TECH_C"]);
    const GOODIES: (&str, &[&str]) = ("GoodyInfo", &["GOODY_A", "GOODY_B"]);

    #[test]
    fn game_option_visible_defaults_to_false() {
        // C++ quirk: the constructor says visible, read() resets it to false when "visible" is absent.
        let (o, w) = read_with(&[], r#"{"type": "GAMEOPTION_X"}"#, read_game_option);
        assert!(!o.visible && !o.default);
        assert!(w.is_empty());
        let (o, _) = read_with(&[], r#"{"default": true, "visible": true}"#, read_game_option);
        assert!(o.visible && o.default);
    }

    #[test]
    fn absent_tags_take_cpp_defaults() {
        let (h, w) = read_with(&[TECHS, GOODIES], r#"{"type": "HANDICAP_X"}"#, read_handicap);
        assert!(w.is_empty());
        assert_eq!(h.num_goodies(), 0);
        assert_eq!(h.free_techs.raw, vec![false; 3]);
        assert_eq!(h.ai_free_techs.raw, vec![false; 3]);

        let (c, _) = read_with(&[SPEEDS], "{}", read_culture_level);
        assert_eq!(c.speed_threshold.raw, vec![0, 0]);

        let (s, _) = read_with(&[], "{}", read_game_speed);
        assert_eq!(s.num_turn_increments(), 0);

        let (p, _) = read_with(&[TECHS], "{}", read_process);
        assert_eq!((p.tech_prereq, p.production_to_commerce_modifier), (None, [0; 4]));

        let (e, _) = read_with(&[], "{}", read_emphasize);
        assert_eq!((e.yield_change, e.commerce_change), ([0; 3], [0; 4]));

        let (g, w) = read_with(&[], r#"{"unit_class": "NONE"}"#, read_goody);
        assert_eq!((g.unit_class_type, g.barbarian_unit_class), (None, None));
        assert!(w.is_empty());
    }

    #[test]
    fn lists_keep_order_duplicates_and_partial_entries() {
        let json = r#"{
            "goodies": ["GOODY_B", "GOODY_A", "GOODY_B"],
            "free_techs": {"TECH_C": true}
        }"#;
        let (h, w) = read_with(&[TECHS, GOODIES], json, read_handicap);
        assert!(w.is_empty());
        assert_eq!(h.goodies, vec![GoodyType(1), GoodyType(0), GoodyType(1)]);
        assert_eq!(h.free_techs.raw, vec![false, false, true]);

        // Each game turn info's values are looked up by key, default 0.
        let json = r#"{"game_turn_infos": [
            {"turns_per_increment": 75, "month_increment": 6},
            {"turns_per_increment": 10}
        ]}"#;
        let (s, _) = read_with(&[], json, read_game_speed);
        assert_eq!(
            s.game_turn_info,
            vec![
                GameTurnInfo { month_increment: 6, num_game_turns_per_increment: 75 },
                GameTurnInfo { month_increment: 0, num_game_turns_per_increment: 10 },
            ]
        );

        // Speeds missing from "speed_thresholds" keep 0.
        let json = r#"{"speed_thresholds": {"GAMESPEED_B": 40}}"#;
        let (c, _) = read_with(&[SPEEDS], json, read_culture_level);
        assert_eq!(c.speed_threshold.raw, vec![0, 40]);
    }
}

// ---------------------------------------------------------------------------------------------
// DiplomacyInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvDiplomacyResponse`: one set of lines and the leaders, civilizations, attitudes and
/// relative powers it is for (an empty filter matches any; the exe picks among the matching
/// responses, see ffh-play's `view/speech.rs`).
#[derive(Debug, Clone, Default)]
pub struct DiplomacyResponse {
    pub civilization_types: IdxVec<CivilizationType, bool>,
    pub leader_head_types: IdxVec<LeaderHeadType, bool>,
    pub attitude_types: IdxVec<AttitudeType, bool>,
    pub diplomacy_power_types: IdxVec<DiplomacyPowerType, bool>,
    /// Text keys of the lines (`AI_DIPLO_GREETINGS_ATT_FR_LEADER_ARENDEL`, ...).
    pub diplomacy_text: Vec<String>,
}

/// C++ `CvDiplomacyResponse::read`.
fn read_diplomacy_response(rd: &Rd) -> DiplomacyResponse {
    DiplomacyResponse {
        civilization_types: rd.pairs_bool::<CivilizationType>("civilizations"),
        leader_head_types: rd.pairs_bool::<LeaderHeadType>("leaders"),
        attitude_types: rd.pairs_bool::<AttitudeType>("attitudes"),
        diplomacy_power_types: rd.pairs_bool::<DiplomacyPowerType>("diplomacy_powers"),
        diplomacy_text: rd.string_list("diplomacy_text"),
    }
}

/// C++ `CvDiplomacyInfo` (diplomacies.jsonc): what a leader says in one diplomacy
/// moment (`AI_DIPLOCOMMENT_*`, the AI's lines) or what the human may answer
/// (`USER_DIPLOCOMMENT_*`). Presentation only: the rules never read it.
#[derive(Debug, Clone, Default)]
pub struct DiplomacyInfo {
    pub base: InfoBase,
    pub responses: Vec<DiplomacyResponse>,
}

/// C++ `CvDiplomacyInfo::read`.
pub fn read_diplomacy(rd: &Rd) -> DiplomacyInfo {
    let responses = rd.child("responses").map(|r| r.children().map(|x| read_diplomacy_response(&x)).collect()).unwrap_or_default();
    DiplomacyInfo { base: InfoBase::read(rd), responses }
}
