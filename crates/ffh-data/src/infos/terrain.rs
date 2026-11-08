//! Info classes: YieldInfo, TerrainInfo, FeatureInfo, BonusClassInfo, BonusInfo,
//! ImprovementInfo (with its per-bonus ImprovementBonusInfo), RouteInfo.
//!
//! Port of `Cv*Info::read` (plus `readPass2`/`readPass3`) in CvInfos.cpp and of
//! `CvXMLLoadUtility::SetImprovementBonuses`. C++ defers some type lookups to
//! `readPass2`/`readPass3` because the referenced file is not loaded yet; our registry
//! knows every type up front, so they resolve directly here. Every lookup in this file
//! was checked against the C++ load order (`CvXMLLoadUtility::LoadBasicInfos` /
//! `LoadPreMenuGlobals`): the referenced class is always registered by the time C++
//! resolves it, so no value differs from C++.
//!
//! Not ported (graphics/audio/UI only): `ArtDefineTag`, `WorldSoundscapeAudioScript`,
//! `FootstepSounds`, the feature `EffectType`/`iEffectProbability` (a bird-scatter effect
//! rolled on the async RNG in `CvUnit::move`), the yield `ColorType`/`SymbolPaths`, and the
//! font glyph `m_iChar` of yields and bonuses (assigned at runtime, not read from XML).
//! `CvBonusInfo::m_piImprovementChange` is never read from XML (only by the savegame
//! stream code) and has no rules use, so C++ `getImprovementChange()` is always -1.

use crate::enums::*;
use crate::idx::IdxVec;
use crate::infos::InfoBase;
use crate::registry::{InfoId, Rd};

/// C++ `GC.getNUM_ROUTE_PREREQ_OR_BONUSES()` (global_defines.jsonc). Readers cannot see the
/// defines, so the value is mirrored here; the data tests check it against `defines`.
const NUM_ROUTE_PREREQ_OR_BONUSES: usize = 2;

// ---------------------------------------------------------------------------------------
// YieldInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvYieldInfo` (yields.jsonc)
#[derive(Debug, Clone, Default)]
pub struct YieldInfo {
    pub base: InfoBase,
    pub hills_change: i32,
    pub peak_change: i32,
    pub lake_change: i32,
    pub city_change: i32,
    pub population_change_offset: i32,
    pub population_change_divisor: i32,
    pub min_city: i32,
    pub trade_modifier: i32,
    pub golden_age_yield: i32,
    pub golden_age_yield_threshold: i32,
    pub ai_weight_percent: i32,
}

/// C++ `CvYieldInfo::read`.
pub fn read_yield(rd: &Rd) -> YieldInfo {
    YieldInfo {
        base: InfoBase::read(rd),
        hills_change: rd.int("hills_change"),
        peak_change: rd.int("peak_change"),
        lake_change: rd.int("lake_change"),
        city_change: rd.int("city_change"),
        population_change_offset: rd.int("population_change_offset"),
        population_change_divisor: rd.int("population_change_divisor"),
        min_city: rd.int("min_city"),
        trade_modifier: rd.int("trade_modifier"),
        golden_age_yield: rd.int("golden_age_yield"),
        golden_age_yield_threshold: rd.int("golden_age_yield_threshold"),
        ai_weight_percent: rd.int("ai_weight_percent"),
    }
}

// ---------------------------------------------------------------------------------------
// TerrainInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvTerrainInfo` (terrains.jsonc)
#[derive(Debug, Clone, Default)]
pub struct TerrainInfo {
    pub base: InfoBase,
    /// `getMovementCost()` (`iMovement`)
    pub movement_cost: i32,
    /// `getSeeFromLevel()` (`iSeeFrom`)
    pub see_from_level: i32,
    /// `getSeeThroughLevel()` (`iSeeThrough`)
    pub see_through_level: i32,
    pub build_modifier: i32,
    /// `getDefenseModifier()` (`iDefense`)
    pub defense_modifier: i32,
    pub water: bool,
    pub impassable: bool,
    pub found: bool,
    pub found_coast: bool,
    pub found_fresh_water: bool,
    // ---- FfH ----
    pub normalize: bool,
    /// `getCivilizationYieldType()`: the civilization that gets `civilization_yield_change`
    /// on this terrain (C++ `readPass3`).
    pub civilization_yield_type: Option<CivilizationType>,
    /// `getCivilizationYieldChange(i)`
    pub civilization_yield_change: [i32; YieldType::NUM],
    /// `getPlotCounterDown()`: `CvPlot::changePlotCounter` turns the terrain into
    /// `terrain_down` when the plot counter (0..=100) drops below this. C++ quirk: the
    /// constructor default is -1, but `read()` resets it to 0 when the tag is absent.
    pub plot_counter_down: i32,
    /// `getTerrainDown()` (C++ `readPass2`)
    pub terrain_down: Option<TerrainType>,
    /// `getPlotCounterUp()`: the terrain becomes `terrain_up` when the plot counter rises
    /// above this. Same -1 (constructor) vs 0 (absent tag) quirk as `plot_counter_down`.
    pub plot_counter_up: i32,
    /// `getTerrainUp()` (C++ `readPass2`)
    pub terrain_up: Option<TerrainType>,
    // ---- arrays ----
    /// `getYield(i)` (`Yields`; `yield` is a Rust keyword)
    pub yield_: [i32; YieldType::NUM],
    pub river_yield_change: [i32; YieldType::NUM],
    pub hills_yield_change: [i32; YieldType::NUM],
    // ---- This game's own: engagement combat (docs/ENGAGEMENTS.md) ----
    /// `iEngagementSlotsChange`: each side's engagement slots on a flat plot of this terrain
    /// without a feature that sets its own change (`GameData::engagement_slots_change`).
    pub engagement_slots_change: i32,
}

/// C++ `CvTerrainInfo::read` + `readPass2` (`TerrainDown`/`TerrainUp`) + `readPass3`
/// (`CivilizationYieldType`).
pub fn read_terrain(rd: &Rd) -> TerrainInfo {
    TerrainInfo {
        base: InfoBase::read(rd),
        movement_cost: rd.int("movement"),
        see_from_level: rd.int("see_from"),
        see_through_level: rd.int("see_through"),
        build_modifier: rd.int("build_modifier"),
        defense_modifier: rd.int("defense"),
        water: rd.bool("water"),
        impassable: rd.bool("impassable"),
        found: rd.bool("found"),
        found_coast: rd.bool("found_coast"),
        found_fresh_water: rd.bool("found_fresh_water"),
        normalize: rd.bool("normalize"),
        civilization_yield_type: rd.rf("civilization_yield_type"),
        civilization_yield_change: rd.yields("civilization_yield_change"),
        plot_counter_down: rd.int("plot_counter_down"),
        terrain_down: rd.rf("terrain_down"),
        plot_counter_up: rd.int("plot_counter_up"),
        terrain_up: rd.rf("terrain_up"),
        yield_: rd.yields("yields"),
        river_yield_change: rd.yields("river_yield_change"),
        hills_yield_change: rd.yields("hills_yield_change"),
        engagement_slots_change: rd.int("engagement_slots_change"),
    }
}

// ---------------------------------------------------------------------------------------
// FeatureInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvFeatureInfo` (features.jsonc)
#[derive(Debug, Clone, Default)]
pub struct FeatureInfo {
    pub base: InfoBase,
    /// `getMovementCost()` (`iMovement`)
    pub movement_cost: i32,
    /// `getSeeThroughChange()` (`iSeeThrough`)
    pub see_through_change: i32,
    pub health_percent: i32,
    /// `getAppearanceProbability()` (`iAppearance`, per 10000 at map generation)
    pub appearance_probability: i32,
    /// `getDisappearanceProbability()` (`iDisappearance`)
    pub disappearance_probability: i32,
    /// `getGrowthProbability()` (`iGrowth`)
    pub growth_probability: i32,
    /// `getDefenseModifier()` (`iDefense`)
    pub defense_modifier: i32,
    pub advanced_start_remove_cost: i32,
    pub turn_damage: i32,
    pub no_coast: bool,
    pub no_river: bool,
    pub no_adjacent: bool,
    pub requires_flatlands: bool,
    pub requires_river: bool,
    pub adds_fresh_water: bool,
    pub impassable: bool,
    pub no_city: bool,
    pub no_improvement: bool,
    pub visible_always: bool,
    pub nuke_immune: bool,
    /// `getOnUnitChangeTo()`: the feature this one becomes when a unit moves in
    /// (`CvUnit::move`; FfH doors). C++ keeps the type string and resolves it on use, where
    /// a non-empty unknown name would clear the feature; here it resolves at load (and warns).
    pub on_unit_change_to: Option<FeatureType>,
    // ---- FfH ----
    pub flammable: bool,
    /// `getFeatureUpgrade()` (C++ `readPass2`)
    pub feature_upgrade: Option<FeatureType>,
    pub prereq_state_religion: Option<ReligionType>,
    /// `getPythonOnMove()`: Python called (spell module `onMoveFeature`) when a unit moves
    /// onto the plot (`CvUnit::setXY`).
    pub python_on_move: String,
    /// `getRequireResist()`: units resisting this damage type by less than
    /// `FEATURE_REQUIRE_RESIST_AMOUNT` may not enter (`CvUnit::canMoveInto`).
    pub require_resist: Option<DamageType>,
    // ---- arrays ----
    /// `getYieldChange(i)` (`YieldChanges`)
    pub yield_change: [i32; YieldType::NUM],
    pub river_yield_change: [i32; YieldType::NUM],
    pub hills_yield_change: [i32; YieldType::NUM],
    /// `isTerrain(i)` (`TerrainBooleans`): terrains the feature may be placed on.
    pub terrain: IdxVec<TerrainType, bool>,
    // ---- This game's own: engagement combat (docs/ENGAGEMENTS.md) ----
    /// `iEngagementSlotsChange`: each side's engagement slots on a plot with this feature,
    /// replacing the terrain's where non-zero (`GameData::engagement_slots_change`).
    pub engagement_slots_change: i32,
}

/// C++ `CvFeatureInfo::read` + `readPass2` (`FeatureUpgrade`).
pub fn read_feature(rd: &Rd) -> FeatureInfo {
    FeatureInfo {
        base: InfoBase::read(rd),
        movement_cost: rd.int("movement"),
        see_through_change: rd.int("see_through"),
        health_percent: rd.int("health_percent"),
        appearance_probability: rd.int("appearance"),
        disappearance_probability: rd.int("disappearance"),
        growth_probability: rd.int("growth"),
        defense_modifier: rd.int("defense"),
        advanced_start_remove_cost: rd.int("advanced_start_remove_cost"),
        turn_damage: rd.int("turn_damage"),
        no_coast: rd.bool("no_coast"),
        no_river: rd.bool("no_river"),
        no_adjacent: rd.bool("no_adjacent"),
        requires_flatlands: rd.bool("requires_flatlands"),
        requires_river: rd.bool("requires_river"),
        adds_fresh_water: rd.bool("adds_fresh_water"),
        impassable: rd.bool("impassable"),
        no_city: rd.bool("no_city"),
        no_improvement: rd.bool("no_improvement"),
        visible_always: rd.bool("visible_always"),
        nuke_immune: rd.bool("nuke_immune"),
        on_unit_change_to: rd.rf("on_unit_change_to"),
        flammable: rd.bool("flammable"),
        feature_upgrade: rd.rf("feature_upgrade"),
        prereq_state_religion: rd.rf("prereq_state_religion"),
        python_on_move: rd.string("python_on_move"),
        require_resist: rd.rf("require_resist"),
        yield_change: rd.yields("yield_changes"),
        river_yield_change: rd.yields("river_yield_change"),
        hills_yield_change: rd.yields("hills_yield_change"),
        terrain: rd.pairs_bool("terrain_booleans"),
        engagement_slots_change: rd.int("engagement_slots_change"),
    }
}

// ---------------------------------------------------------------------------------------
// BonusClassInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvBonusClassInfo` (bonus_classes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct BonusClassInfo {
    pub base: InfoBase,
    /// `getUniqueRange()` (`iUnique`): map generation keeps other bonus types of this class
    /// out of this range (`CvMapGenerator::canPlaceBonusAt`).
    pub unique_range: i32,
}

/// C++ `CvBonusClassInfo::read`.
pub fn read_bonus_class(rd: &Rd) -> BonusClassInfo {
    BonusClassInfo { base: InfoBase::read(rd), unique_range: rd.int("unique") }
}

// ---------------------------------------------------------------------------------------
// BonusInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvBonusInfo` (bonuses.jsonc)
#[derive(Debug, Clone, Default)]
pub struct BonusInfo {
    pub base: InfoBase,
    pub bonus_class_type: Option<BonusClassType>,
    pub tech_reveal: Option<TechType>,
    pub tech_city_trade: Option<TechType>,
    pub tech_obsolete: Option<TechType>,
    pub ai_trade_modifier: i32,
    pub ai_objective: i32,
    pub health: i32,
    pub happiness: i32,
    pub min_area_size: i32,
    pub min_latitude: i32,
    /// `getMaxLatitude()`: 90 when the tag is absent.
    pub max_latitude: i32,
    pub placement_order: i32,
    pub const_appearance: i32,
    /// `getRandAppearance1()` (`Rands/iRandApp1`); likewise 2..4.
    pub rand_appearance1: i32,
    pub rand_appearance2: i32,
    pub rand_appearance3: i32,
    pub rand_appearance4: i32,
    /// `getPercentPerPlayer()` (`iPlayer`)
    pub percent_per_player: i32,
    pub tiles_per: i32,
    pub min_land_percent: i32,
    /// `getUniqueRange()` (`iUnique`): map generation keeps this same bonus out of this range.
    pub unique_range: i32,
    pub group_range: i32,
    pub group_rand: i32,
    /// `isOneArea()` (`bArea`)
    pub one_area: bool,
    pub hills: bool,
    pub flatlands: bool,
    pub no_river_side: bool,
    pub normalize: bool,
    // ---- FfH ----
    pub modifier_per_bonus: bool,
    pub bad_attitude: i32,
    pub damage_type: Option<DamageType>,
    pub discover_rand_modifier: i32,
    /// `getFreePromotion()` (C++ `readPass3`)
    pub free_promotion: Option<PromotionType>,
    pub great_people_rate_modifier: i32,
    pub heal_change: i32,
    pub heal_change_enemy: i32,
    pub maintenance_modifier: i32,
    pub mutate_chance: i32,
    pub research_modifier: i32,
    // ---- arrays ----
    /// `getYieldChange(i)` (`YieldChanges`)
    pub yield_change: [i32; YieldType::NUM],
    /// `isTerrain(i)` (`TerrainBooleans`)
    pub terrain: IdxVec<TerrainType, bool>,
    /// `isFeature(i)` (`FeatureBooleans`)
    pub feature: IdxVec<FeatureType, bool>,
    /// `isFeatureTerrain(i)` (`FeatureTerrainBooleans`)
    pub feature_terrain: IdxVec<TerrainType, bool>,
}

/// C++ `CvBonusInfo::read` + `readPass3` (`FreePromotion`).
pub fn read_bonus(rd: &Rd) -> BonusInfo {
    // Without a `<Rands>` block the members keep their constructor default (0).
    let rands = rd.child("rands");
    let rand = |tag: &str| rands.map_or(0, |r| r.int(tag));
    BonusInfo {
        base: InfoBase::read(rd),
        bonus_class_type: rd.rf("bonus_class_type"),
        tech_reveal: rd.rf("tech_reveal"),
        tech_city_trade: rd.rf("tech_city_trade"),
        tech_obsolete: rd.rf("tech_obsolete"),
        ai_trade_modifier: rd.int("ai_trade_modifier"),
        ai_objective: rd.int("ai_objective"),
        health: rd.int("health"),
        happiness: rd.int("happiness"),
        min_area_size: rd.int("min_area_size"),
        min_latitude: rd.int("min_latitude"),
        max_latitude: rd.int_or("max_latitude", 90),
        placement_order: rd.int("placement_order"),
        const_appearance: rd.int("const_appearance"),
        rand_appearance1: rand("rand_app1"),
        rand_appearance2: rand("rand_app2"),
        rand_appearance3: rand("rand_app3"),
        rand_appearance4: rand("rand_app4"),
        percent_per_player: rd.int("player"),
        tiles_per: rd.int("tiles_per"),
        min_land_percent: rd.int("min_land_percent"),
        unique_range: rd.int("unique"),
        group_range: rd.int("group_range"),
        group_rand: rd.int("group_rand"),
        one_area: rd.bool("area"),
        hills: rd.bool("hills"),
        flatlands: rd.bool("flatlands"),
        no_river_side: rd.bool("no_river_side"),
        normalize: rd.bool("normalize"),
        modifier_per_bonus: rd.bool("modifier_per_bonus"),
        bad_attitude: rd.int("bad_attitude"),
        damage_type: rd.rf("damage_type"),
        discover_rand_modifier: rd.int("discover_rand_modifier"),
        free_promotion: rd.rf("free_promotion"),
        great_people_rate_modifier: rd.int("great_people_rate_modifier"),
        heal_change: rd.int("heal_change"),
        heal_change_enemy: rd.int("heal_change_enemy"),
        maintenance_modifier: rd.int("maintenance_modifier"),
        mutate_chance: rd.int("mutate_chance"),
        research_modifier: rd.int("research_modifier"),
        yield_change: rd.yields("yield_changes"),
        terrain: rd.pairs_bool("terrain_booleans"),
        feature: rd.pairs_bool("feature_booleans"),
        feature_terrain: rd.pairs_bool("feature_terrain_booleans"),
    }
}

// ---------------------------------------------------------------------------------------
// ImprovementInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvImprovementBonusInfo`: what an improvement does with one bonus (a
/// `<BonusTypeStruct>`), filled by `CvXMLLoadUtility::SetImprovementBonuses`. Its
/// `CvInfoBase` part is never read, so there is no `base` field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImprovementBonusInfo {
    /// `getDiscoverRand()`: 1-in-N chance per turn that a worked plot with this improvement
    /// discovers the bonus (`CvPlot::doImprovement`, before the player's discover modifier
    /// and game-speed scaling); 0 = never.
    pub discover_rand: i32,
    pub bonus_makes_valid: bool,
    pub bonus_trade: bool,
    /// `getYieldChange(i)` (`YieldChanges`)
    pub yield_change: [i32; YieldType::NUM],
}

/// C++ `CvImprovementInfo` (improvements.jsonc)
#[derive(Debug, Clone, Default)]
pub struct ImprovementInfo {
    pub base: InfoBase,
    pub advanced_start_cost: i32,
    pub advanced_start_cost_increase: i32,
    pub tiles_per_goody: i32,
    /// `getGoodyUniqueRange()` (`iGoodyRange`)
    pub goody_unique_range: i32,
    /// `getFeatureGrowthProbability()` (`iFeatureGrowth`)
    pub feature_growth_probability: i32,
    pub upgrade_time: i32,
    pub air_bomb_defense: i32,
    pub defense_modifier: i32,
    pub happiness: i32,
    pub pillage_gold: i32,
    /// `getImprovementPillage()`: what is left when this improvement is pillaged
    /// (C++ `readPass2`).
    pub improvement_pillage: Option<ImprovementType>,
    /// `getImprovementUpgrade()` (C++ `readPass2`)
    pub improvement_upgrade: Option<ImprovementType>,
    /// `isActsAsCity()`. C++ quirk: the constructor default is `true`, but `read()` resets
    /// it to `false` when the tag is absent (every FFH2 entry sets it explicitly).
    pub acts_as_city: bool,
    pub hills_makes_valid: bool,
    pub fresh_water_makes_valid: bool,
    pub river_side_makes_valid: bool,
    pub no_fresh_water: bool,
    pub requires_flatlands: bool,
    pub requires_river_side: bool,
    pub requires_irrigation: bool,
    pub carries_irrigation: bool,
    pub requires_feature: bool,
    pub water: bool,
    pub goody: bool,
    pub permanent: bool,
    pub outside_borders: bool,
    // ---- FfH ----
    pub requires_peak: bool,
    pub unique: bool,
    /// `getAppearanceProbability()`: per-10000 chance per eligible plot at map generation.
    pub appearance_probability: i32,
    pub heal_rate_change: i32,
    pub range: i32,
    pub range_defense_modifier: i32,
    pub bonus_convert: Option<BonusType>,
    pub feature_upgrade: Option<FeatureType>,
    /// `getPrereqCivilization()` (C++ `readPass3`)
    pub prereq_civilization: Option<CivilizationType>,
    /// Not in FFH2: the tech whose research rate rises by `tech_research_percent` percent for a
    /// player whose team has this improvement inside a member's borders (`tech_research_bonus`;
    /// the unique features). `GameData::tech_research_bonuses` lists them by tech.
    pub tech_research_bonus: Option<TechType>,
    pub tech_research_percent: i32,
    /// `getPythonAtRange()`: Python called (spell module `atRange`, if the plot's Python is
    /// active) when a unit moves onto an ADJACENT plot (`CvUnit::setXY`).
    pub python_at_range: String,
    /// `getPythonOnMove()`: Python called (spell module `onMove`, if the plot's Python is
    /// active) when a unit moves onto the plot (`CvUnit::setXY`).
    pub python_on_move: String,
    /// `getSpawnUnitType()` (C++ `readPass3`)
    pub spawn_unit_type: Option<UnitType>,
    /// Not in FFH2: what a lair spawns instead of `spawn_unit_type` once the Armageddon card
    /// The Lairs Deepen has fired (docs/CARDS.md; `deep_spawn_unit_type`).
    pub deep_spawn_unit_type: Option<UnitType>,
    pub visibility_change: i32,
    // ---- this game's: the cards' improvements (docs/CARDS.md) ----
    /// Causeway: land units move onto and over this water plot as over land, and ships cannot
    /// enter it.
    pub land_bridge: bool,
    /// Canal Locks: ships can enter this land plot unless it is in the land of a civ at war with
    /// them; it is built next to water or to another plot with this improvement.
    pub canal: bool,
    /// Siege Lines, Wayside Shrines: units of a team that knows this tech heal on the plot as in
    /// a city of their own.
    pub city_heal_tech: Option<TechType>,
    /// Siege Lines: built only next to a city of a civ the builder is at war with, and then also
    /// in that civ's land.
    pub enemy_city_adjacent: bool,
    /// Wayside Shrines: built only on a plot with a route.
    pub requires_route: bool,
    // ---- arrays ----
    /// `getPrereqNatureYield(i)` (`PrereqNatureYields`)
    pub prereq_nature_yield: [i32; YieldType::NUM],
    /// `getYieldChange(i)` (`YieldChanges`)
    pub yield_change: [i32; YieldType::NUM],
    pub river_side_yield_change: [i32; YieldType::NUM],
    pub hills_yield_change: [i32; YieldType::NUM],
    /// `getIrrigatedYieldChange(i)` (C++ member `m_piIrrigatedChange`)
    pub irrigated_yield_change: [i32; YieldType::NUM],
    /// `getTerrainMakesValid(i)` (`TerrainMakesValids`)
    pub terrain_makes_valid: IdxVec<TerrainType, bool>,
    /// `getFeatureMakesValid(i)` (`FeatureMakesValids`)
    pub feature_makes_valid: IdxVec<FeatureType, bool>,
    /// `getTechYieldChanges(tech, yield)` (`TechYieldChanges`)
    pub tech_yield_changes: IdxVec<TechType, [i32; YieldType::NUM]>,
    /// `getRouteYieldChanges(route, yield)` (`RouteYieldChanges`)
    pub route_yield_changes: IdxVec<RouteType, [i32; YieldType::NUM]>,
    /// C++ `m_paImprovementBonus` (`BonusTypeStructs`), one entry per bonus:
    /// `getImprovementBonusYield(b, y)` is `improvement_bonus[b].yield_change[y]`;
    /// `isImprovementBonusMakesValid(b)`, `isImprovementBonusTrade(b)` and
    /// `getImprovementBonusDiscoverRand(b)` are its other fields.
    pub improvement_bonus: IdxVec<BonusType, ImprovementBonusInfo>,
    /// This game's own (a great-person card's key, docs/CARDS.md section 2):
    /// `engagement_attacker_slots_change`, the change to the attacker's slots in battles on its
    /// plot (before the minimum).
    pub engagement_attacker_slots_change: i32,
    // This game's (the Trading Post card, docs/CARDS.md)
    /// The plots within this distance of the improvement (its own included) are its builder's
    /// territory, whatever their culture (`claim_range`; 0: no claim). An improvement with a
    /// claim is built only outside every civilization's borders.
    pub claim_range: i32,
    /// The improvement is built only this many plots or more from every city
    /// (`min_city_distance`; 0: anywhere).
    pub min_city_distance: i32,
    /// This game's own (the Cairn Golem's Cairn, docs/CARDS.md section 2): `battle_sentinels`, the
    /// Sentinels the defender fights with in battles on its plot (`UNIT_SENTINEL`'s values grown by
    /// `engagement_defense`, whole at the start of each battle).
    pub battle_sentinels: i32,
    /// `engagement_defense`: percent more defence (Dv) for the defender's units in battles on its
    /// plot (and for its Sentinels' HP and Dv).
    pub engagement_defense: i32,
}

/// C++ `CvImprovementInfo::read`, plus `readPass2` (`ImprovementPillage`,
/// `ImprovementUpgrade`) and `readPass3` (`PrereqCivilization`, `SpawnUnitType`).
pub fn read_improvement(rd: &Rd) -> ImprovementInfo {
    ImprovementInfo {
        base: InfoBase::read(rd),
        advanced_start_cost: rd.int("advanced_start_cost"),
        advanced_start_cost_increase: rd.int("advanced_start_cost_increase"),
        tiles_per_goody: rd.int("tiles_per_goody"),
        goody_unique_range: rd.int("goody_range"),
        feature_growth_probability: rd.int("feature_growth"),
        upgrade_time: rd.int("upgrade_time"),
        air_bomb_defense: rd.int("air_bomb_defense"),
        defense_modifier: rd.int("defense_modifier"),
        happiness: rd.int("happiness"),
        pillage_gold: rd.int("pillage_gold"),
        improvement_pillage: rd.rf("improvement_pillage"),
        improvement_upgrade: rd.rf("improvement_upgrade"),
        acts_as_city: rd.bool("acts_as_city"),
        hills_makes_valid: rd.bool("hills_makes_valid"),
        fresh_water_makes_valid: rd.bool("fresh_water_makes_valid"),
        river_side_makes_valid: rd.bool("river_side_makes_valid"),
        no_fresh_water: rd.bool("no_fresh_water"),
        requires_flatlands: rd.bool("requires_flatlands"),
        requires_river_side: rd.bool("requires_river_side"),
        requires_irrigation: rd.bool("requires_irrigation"),
        carries_irrigation: rd.bool("carries_irrigation"),
        requires_feature: rd.bool("requires_feature"),
        water: rd.bool("water"),
        goody: rd.bool("goody"),
        permanent: rd.bool("permanent"),
        outside_borders: rd.bool("outside_borders"),
        requires_peak: rd.bool("requires_peak"),
        unique: rd.bool("unique"),
        appearance_probability: rd.int("appearance_probability"),
        heal_rate_change: rd.int("heal_rate_change"),
        range: rd.int("range"),
        range_defense_modifier: rd.int("range_defense_modifier"),
        bonus_convert: rd.rf("bonus_convert"),
        feature_upgrade: rd.rf("feature_upgrade"),
        prereq_civilization: rd.rf("prereq_civilization"),
        tech_research_bonus: rd.rf("tech_research_bonus"),
        tech_research_percent: rd.int("tech_research_percent"),
        python_at_range: rd.string("python_at_range"),
        python_on_move: rd.string("python_on_move"),
        spawn_unit_type: rd.rf("spawn_unit_type"),
        deep_spawn_unit_type: rd.rf("deep_spawn_unit_type"),
        visibility_change: rd.int("visibility_change"),
        land_bridge: rd.bool("land_bridge"),
        canal: rd.bool("canal"),
        city_heal_tech: rd.rf("city_heal_tech"),
        enemy_city_adjacent: rd.bool("enemy_city_adjacent"),
        requires_route: rd.bool("requires_route"),
        prereq_nature_yield: rd.yields("prereq_nature_yields"),
        yield_change: rd.yields("yield_changes"),
        river_side_yield_change: rd.yields("river_side_yield_change"),
        hills_yield_change: rd.yields("hills_yield_change"),
        irrigated_yield_change: rd.yields("irrigated_yield_change"),
        terrain_makes_valid: rd.pairs_bool("terrain_makes_valids"),
        feature_makes_valid: rd.pairs_bool("feature_makes_valids"),
        tech_yield_changes: yields_by_type(rd, "tech_yield_changes", "prereq_tech", "tech_yields"),
        route_yield_changes: yields_by_type(rd, "route_yield_changes", "route_type", "route_yields"),
        improvement_bonus: improvement_bonuses(rd),
        engagement_attacker_slots_change: rd.int("engagement_attacker_slots_change"),
        claim_range: rd.int("claim_range"),
        min_city_distance: rd.int("min_city_distance"),
        battle_sentinels: rd.int("battle_sentinels"),
        engagement_defense: rd.int("engagement_defense"),
    }
}

/// The `TechYieldChanges`/`RouteYieldChanges` loops of `CvImprovementInfo::read`:
/// `Init2DIntList` (all zero), then every child of `<tag>` whose `<key_tag>` names a type
/// gets that type's yields from its `<yields_tag>` (`SetYields`; zeros if absent). Children
/// with a missing or unknown key are skipped; a repeated key overwrites.
fn yields_by_type<K: InfoId>(
    rd: &Rd,
    tag: &str,
    key_tag: &str,
    yields_tag: &str,
) -> IdxVec<K, [i32; YieldType::NUM]> {
    let mut v = IdxVec::from_elem([0; YieldType::NUM], rd.reg().count::<K>());
    if let Some(list) = rd.child(tag) {
        for entry in list.children() {
            if let Some(k) = entry.rf::<K>(key_tag) {
                v[k] = entry.yields(yields_tag);
            }
        }
    }
    v
}

/// C++ `CvXMLLoadUtility::SetImprovementBonuses` (`bonus_type_structs`): every bonus gets a
/// zeroed entry (`InitImprovementBonusList`), then each item of the list that names a
/// `bonus_type` sets that bonus's entry (a repeated bonus overwrites the earlier one).
fn improvement_bonuses(rd: &Rd) -> IdxVec<BonusType, ImprovementBonusInfo> {
    let mut v = IdxVec::from_elem(ImprovementBonusInfo::default(), rd.reg().count::<BonusType>());
    let Some(list) = rd.child("bonus_type_structs") else {
        return v;
    };
    for s in list.children() {
        let Some(bonus) = s.rf::<BonusType>("bonus_type") else { continue };
        v[bonus] = ImprovementBonusInfo {
            bonus_makes_valid: s.bool("bonus_makes_valid"),
            bonus_trade: s.bool("bonus_trade"),
            discover_rand: s.int("discover_rand"),
            yield_change: s.yields("yield_changes"),
        };
    }
    v
}

// ---------------------------------------------------------------------------------------
// RouteInfo
// ---------------------------------------------------------------------------------------

/// C++ `CvRouteInfo` (routes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct RouteInfo {
    pub base: InfoBase,
    pub advanced_start_cost: i32,
    pub advanced_start_cost_increase: i32,
    pub value: i32,
    /// `getMovementCost()` (`iMovement`)
    pub movement_cost: i32,
    /// `getFlatMovementCost()` (`iFlatMovement`)
    pub flat_movement_cost: i32,
    /// `getPrereqBonus()` (`BonusType`)
    pub prereq_bonus: Option<BonusType>,
    /// `getYieldChange(i)` (`Yields`)
    pub yield_change: [i32; YieldType::NUM],
    /// `getTechMovementChange(i)` (`TechMovementChanges`)
    pub tech_movement_change: IdxVec<TechType, i32>,
    /// `getPrereqOrBonus(i)` (`PrereqOrBonuses`): `GC.getNUM_ROUTE_PREREQ_OR_BONUSES()`
    /// slots filled in order, `None` = empty (C++ -1).
    pub prereq_or_bonus: Vec<Option<BonusType>>,
}

/// C++ `CvRouteInfo::read`.
pub fn read_route(rd: &Rd) -> RouteInfo {
    RouteInfo {
        base: InfoBase::read(rd),
        advanced_start_cost: rd.int("advanced_start_cost"),
        advanced_start_cost_increase: rd.int("advanced_start_cost_increase"),
        value: rd.int("value"),
        movement_cost: rd.int("movement"),
        flat_movement_cost: rd.int("flat_movement"),
        prereq_bonus: rd.rf("bonus_type"),
        yield_change: rd.yields("yields"),
        tech_movement_change: rd.pairs_int("tech_movement_changes", 0),
        prereq_or_bonus: rd.ref_list_fixed("prereq_or_bonuses", NUM_ROUTE_PREREQ_OR_BONUSES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Ctx, Registry};

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn registry() -> Registry {
        let mut reg = Registry::new();
        reg.register_class("TechInfo", names(&["TECH_X", "TECH_Y", "TECH_Z"]));
        reg.register_class("RouteInfo", names(&["ROUTE_ROAD", "ROUTE_RAILROAD"]));
        reg.register_class("BonusInfo", names(&["BONUS_A", "BONUS_B", "BONUS_C"]));
        reg
    }

    /// Runs `f` on the entry object `json` and returns its result and the warnings.
    fn read<T>(json: &str, f: impl Fn(&Rd) -> T) -> (T, Vec<String>) {
        let reg = registry();
        let value = crate::jsonc::parse(json).expect("test json parses");
        let cx = Ctx::new(&reg, "test.jsonc");
        let out = f(&Rd::new(&value, &cx));
        (out, cx.warnings.into_inner())
    }

    #[test]
    fn tech_and_route_yield_changes() {
        let json = r#"{
            "tech_yield_changes": [
                {"prereq_tech": "TECH_Y", "tech_yields": [1, 2]},
                {"prereq_tech": "TECH_Z"},
                {"tech_yields": [9]}
            ],
            "route_yield_changes": [
                {"route_type": "ROUTE_RAILROAD", "route_yields": [0, 1, 0]}
            ]
        }"#;
        let ((tech, route), warnings) = read(json, |rd| {
            let tech: IdxVec<TechType, _> = yields_by_type(rd, "tech_yield_changes", "prereq_tech", "tech_yields");
            let route: IdxVec<RouteType, _> = yields_by_type(rd, "route_yield_changes", "route_type", "route_yields");
            (tech, route)
        });
        // Short yield lists are zero-padded; a key without yields is all zero; an entry
        // without a key is ignored.
        assert_eq!(tech.raw, vec![[0, 0, 0], [1, 2, 0], [0, 0, 0]]);
        assert_eq!(route.raw, vec![[0, 0, 0], [0, 1, 0]]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn improvement_bonuses_fill_the_named_bonus() {
        let json = r#"{
            "bonus_type_structs": [
                {
                    "bonus_type": "BONUS_B",
                    "bonus_makes_valid": true,
                    "bonus_trade": false,
                    "discover_rand": 100,
                    "yield_changes": [0, 1]
                },
                {"bonus_type": "BONUS_C", "bonus_trade": true}
            ]
        }"#;
        let (v, warnings) = read(json, improvement_bonuses);
        assert_eq!(v.len(), 3);
        assert_eq!(v.raw[0], ImprovementBonusInfo::default());
        assert_eq!(
            v.raw[1],
            ImprovementBonusInfo { discover_rand: 100, bonus_makes_valid: true, bonus_trade: false, yield_change: [0, 1, 0] }
        );
        // Absent keys leave the defaults.
        assert_eq!(v.raw[2], ImprovementBonusInfo { bonus_trade: true, ..Default::default() });
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn faithful_defaults_for_absent_tags() {
        let (b, _) = read(r#"{"type": "BONUS_A"}"#, read_bonus);
        assert_eq!(b.max_latitude, 90);
        assert_eq!((b.rand_appearance1, b.tech_reveal, b.bonus_class_type), (0, None, None));

        let (t, _) = read(r#"{"type": "T"}"#, read_terrain);
        assert_eq!((t.plot_counter_down, t.plot_counter_up), (0, 0));
        assert_eq!((t.terrain_down, t.civilization_yield_type), (None, None));

        let (i, _) = read(r#"{"type": "I"}"#, read_improvement);
        assert!(!i.acts_as_city);
        assert_eq!(i.improvement_bonus.len(), 3);
        assert_eq!(i.tech_yield_changes.len(), 3);
        assert_eq!(i.route_yield_changes.len(), 2);

        let (r, _) = read(r#"{"type": "R", "prereq_or_bonuses": ["BONUS_C"]}"#, read_route);
        assert_eq!(r.prereq_or_bonus, vec![Some(BonusType(2)), None]);
        assert_eq!(r.tech_movement_change.raw, vec![0, 0, 0]);
    }
}
