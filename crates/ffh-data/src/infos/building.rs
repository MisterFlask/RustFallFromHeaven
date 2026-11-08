//! Info classes: SpecialBuildingInfo, BuildingClassInfo, BuildingInfo.
//! Port of `CvSpecialBuildingInfo::read`, `CvBuildingClassInfo::read` (+ `readPass3`) and
//! `CvBuildingInfo::read` (+ `readPass3`) in CvInfos.cpp.
//!
//! Not ported (graphics/audio/UI only): the `CvHotkeyInfo` hotkey fields, `ArtDefineTag`,
//! `MovieDefineTag`, `ConstructSound`, `fVisibilityPriority` (city art layering),
//! `Advisor` (only feeds the human "recommended" production popup; AdvisorInfos are not
//! loaded) and `m_iMissionType` (set for the action bar after loading, not by `read`).

use crate::define_idx;
use crate::enums::*;
use crate::idx::IdxVec;
use crate::infos::InfoBase;
use crate::registry::{InfoId, Rd};

define_idx!(/// A building tier (`BUILDINGTIER_*`, building_tiers.jsonc): this game's building suitability
/// (docs/SUITABILITY.md). Not in FFH2.
BuildingTierType);

impl BuildingTierType {
    pub const LESSER: BuildingTierType = BuildingTierType(0);
    pub const STANDARD: BuildingTierType = BuildingTierType(1);
    pub const FINE: BuildingTierType = BuildingTierType(2);
    pub const GRAND: BuildingTierType = BuildingTierType(3);
    pub const LEGENDARY: BuildingTierType = BuildingTierType(4);
    /// The tiers' type names in id order (the data's order is checked against it).
    pub const NAMES: &'static [&'static str] =
        &["BUILDINGTIER_LESSER", "BUILDINGTIER_STANDARD", "BUILDINGTIER_FINE", "BUILDINGTIER_GRAND", "BUILDINGTIER_LEGENDARY"];
    pub const NUM: usize = 5;
}

/// A building tier (building_tiers.jsonc; docs/SUITABILITY.md). Not in FFH2.
#[derive(Debug, Clone, Default)]
pub struct BuildingTierInfo {
    pub base: InfoBase,
    /// The tier's weight in a city's roll for a building class (the chance is the weight over
    /// the sum of the weights).
    pub weight: i32,
    /// A wonder's cost in a city of this tier for its class, in percent.
    pub wonder_cost_percent: i32,
}

/// Reads a `building_tiers.jsonc` entry.
pub fn read_building_tier(rd: &Rd) -> BuildingTierInfo {
    BuildingTierInfo { base: InfoBase::read(rd), weight: rd.int("weight"), wonder_cost_percent: rd.int_or("wonder_cost_percent", 100) }
}

/// C++ `GC.getNUM_BUILDING_AND_TECH_PREREQS()` (global_defines.jsonc). `Rd` cannot see the
/// defines, so the value is mirrored here; the data tests check it against
/// `defines.num_building_and_tech_prereqs`.
const NUM_BUILDING_AND_TECH_PREREQS: usize = 3;
/// C++ `GC.getNUM_BUILDING_PREREQ_OR_BONUSES()`: 5 from global_defines_alt.jsonc (overrides the
/// BTS 4 before `cacheGlobals`). Checked against the defines by the data tests.
const NUM_BUILDING_PREREQ_OR_BONUSES: usize = 5;

/// C++ `CvSpecialBuildingInfo` (special_buildings.jsonc)
#[derive(Debug, Clone, Default)]
pub struct SpecialBuildingInfo {
    pub base: InfoBase,
    pub obsolete_tech: Option<TechType>,
    pub tech_prereq: Option<TechType>,
    pub tech_prereq_anyone: Option<TechType>,
    /// C++ `isValid()`.
    pub valid: bool,
    /// C++ `getProductionTraits(i)`: production modifier (%) for players with the trait.
    pub production_traits: IdxVec<TraitType, i32>,
}

/// C++ `CvSpecialBuildingInfo::read`.
pub fn read_special_building(rd: &Rd) -> SpecialBuildingInfo {
    SpecialBuildingInfo {
        base: InfoBase::read(rd),
        obsolete_tech: rd.rf("obsolete_tech"),
        tech_prereq: rd.rf("tech_prereq"),
        tech_prereq_anyone: rd.rf("tech_prereq_anyone"),
        valid: rd.bool("valid"),
        production_traits: rd.pairs_int("production_traits", 0),
    }
}

/// C++ `CvBuildingClassInfo` (building_classes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct BuildingClassInfo {
    pub base: InfoBase,
    /// `!= -1` makes the class a world wonder (C++ `isWorldWonderClass`).
    pub max_global_instances: i32,
    /// `!= -1` makes the class a team wonder (C++ `isTeamWonderClass`).
    pub max_team_instances: i32,
    /// `!= -1` makes the class a national wonder (C++ `isNationalWonderClass`).
    pub max_player_instances: i32,
    pub extra_player_instances: i32,
    /// C++ `getDefaultBuildingIndex()`: `<DefaultBuilding>`, resolved in `readPass3()`.
    /// Default building of the class for civs that do not override it
    /// (see `CivilizationInfo::civilization_buildings`).
    pub default_building_index: Option<BuildingType>,
    pub no_limit: bool,
    pub monument: bool,
    /// C++ `getVictoryThreshold(i)`.
    pub victory_threshold: IdxVec<VictoryType, i32>,
    /// Derived at load (not in the data files; docs/SUITABILITY.md): a city rolls a tier for this
    /// class. True for a class with tier variants, and for a wonder class that can be built and
    /// is no victory's (its tier is a cost).
    pub rolls_tier: bool,
}

/// C++ `CvBuildingClassInfo::read` + `readPass3`.
pub fn read_building_class(rd: &Rd) -> BuildingClassInfo {
    BuildingClassInfo {
        base: InfoBase::read(rd),
        // C++ defaults are 0 (not -1) when the tags are absent; every FFH2 entry sets them.
        max_global_instances: rd.int("max_global_instances"),
        max_team_instances: rd.int("max_team_instances"),
        max_player_instances: rd.int("max_player_instances"),
        extra_player_instances: rd.int("extra_player_instances"),
        no_limit: rd.bool("no_limit"),
        monument: rd.bool("monument"),
        victory_threshold: rd.pairs_int("victory_thresholds", 0),
        // readPass3: m_iDefaultBuildingIndex = GC.getInfoTypeForString(<DefaultBuilding>)
        default_building_index: rd.rf("default_building"),
        rolls_tier: false,
    }
}

/// C++ `CvBuildingInfo` (buildings.jsonc, and the tier variants of building_variants.jsonc)
#[derive(Debug, Clone, Default)]
pub struct BuildingInfo {
    pub base: InfoBase,

    // ---- tiers (this game's building suitability, docs/SUITABILITY.md; not in FFH2) ----
    /// The tier of this building: `variant_tier` of a variant (building_variants.jsonc), Standard
    /// for a building that has variants (derived), `None` for one without tiers.
    pub tier: Option<BuildingTierType>,
    /// The Standard building this one is a tier variant of (`None` for any other building).
    pub variant_of: Option<BuildingType>,
    /// Derived: the buildings of this one's family by tier (`[tier]`), the same list on every
    /// member; empty for a building without tiers.
    pub tier_variants: Vec<BuildingType>,
    /// Experience for the units this building unlocks (whose `prereq_building` is of its family
    /// or whose `prereq_building_class` is its class) when they are trained in its city.
    pub unlocked_unit_experience: i32,
    /// Production modifier (%) of the units it unlocks, in its city.
    pub unlocked_unit_production_modifier: i32,
    /// A promotion for the units it unlocks trained in its city (where the promotion's unit
    /// combats allow it, as for `free_promotion`).
    pub unlocked_unit_promotion: Option<PromotionType>,
    /// A second promotion for every unit trained in its city, as `free_promotion`.
    pub extra_free_promotion: Option<PromotionType>,
    /// Yield change of the city for each of these bonuses it has (like `bonus_health_changes`).
    pub bonus_yield_change: IdxVec<BonusType, [i32; YieldType::NUM]>,
    /// Commerce per citizen of the city, in hundredths (50: +1 per 2 citizens).
    pub population_commerce: [i32; CommerceType::NUM],
    /// Percentage points added to `SENTINEL_REPAIR_PERCENT` for its city's Sentinels.
    pub sentinel_repair_change: i32,
    // ---- this game's own: the great-person cards' keys (docs/CARDS.md, section 2) ----
    /// What it does to its city's battles (`battle_cards::BuildingBattle`).
    pub battle: crate::infos::BuildingBattle,
    /// `exclusive_with`: a city that has a building of one of these classes cannot build it.
    pub exclusive_with: Vec<BuildingClassType>,
    /// `legion_cost_percent` (the Gilded Legion): its city may train the units a civilization
    /// chosen once it is built (`City::legion_civ`) has for each unit class (not world units), with
    /// their own prerequisites, at this percent of their cost.
    pub legion_cost_percent: i32,

    // ---- the Armageddon deck (docs/CARDS.md; not in FFH2) ----
    /// Commerce of its city per 10 points of the Armageddon counter (`global_counter_commerces`:
    /// the Orrery of Ceridwen's research).
    pub global_counter_commerce: [i32; CommerceType::NUM],
    /// How many unrevealed Armageddon cards ahead its owner sees (`doom_sight`: the Orrery 2).
    pub doom_sight: i32,

    // ---- type references ----
    /// C++ `getBuildingClassType()`, XML `<BuildingClass>`.
    pub building_class_type: Option<BuildingClassType>,
    pub victory_prereq: Option<VictoryType>,
    pub free_start_era: Option<EraType>,
    pub max_start_era: Option<EraType>,
    pub obsolete_tech: Option<TechType>,
    /// C++ `getPrereqAndTech()`, XML `<PrereqTech>`.
    pub prereq_and_tech: Option<TechType>,
    pub no_bonus: Option<BonusType>,
    pub power_bonus: Option<BonusType>,
    pub free_bonus: Option<BonusType>,
    pub num_free_bonuses: i32,
    /// C++ `getFreeBuildingClass()`, XML `<FreeBuilding>`.
    pub free_building_class: Option<BuildingClassType>,
    pub free_promotion: Option<PromotionType>,
    pub civic_option: Option<CivicOptionType>,
    pub special_building_type: Option<SpecialBuildingType>,
    pub holy_city: Option<ReligionType>,
    pub religion_type: Option<ReligionType>,
    /// C++ `getStateReligion()` (the religion). The bool `isStateReligion()` is
    /// [`Self::is_state_religion`].
    pub state_religion: Option<ReligionType>,
    /// C++ `getPrereqReligion()` (the religion). The bool `isPrereqReligion()` is
    /// [`Self::is_prereq_religion`].
    pub prereq_religion: Option<ReligionType>,
    pub prereq_corporation: Option<CorporationType>,
    pub founds_corporation: Option<CorporationType>,
    /// C++ `getGlobalReligionCommerce()`: religion whose global spread yields commerce here.
    pub global_religion_commerce: Option<ReligionType>,
    /// C++ `getGlobalCorporationCommerce()`.
    pub global_corporation_commerce: Option<CorporationType>,
    /// C++ `getPrereqAndBonus()`, XML `<Bonus>`.
    pub prereq_and_bonus: Option<BonusType>,
    pub great_people_unit_class: Option<UnitClassType>,
    /// C++ `getVoteSourceType()`, XML `<DiploVoteType>`.
    pub vote_source_type: Option<VoteSourceType>,

    // ---- ints ----
    /// C++ `getAIWeight()`.
    pub ai_weight: i32,
    /// C++ `getProductionCost()`, XML `<iCost>` (-1 = cannot be built).
    pub production_cost: i32,
    pub hurry_cost_modifier: i32,
    pub hurry_anger_modifier: i32,
    pub advanced_start_cost: i32,
    pub advanced_start_cost_increase: i32,
    pub min_area_size: i32,
    /// C++ `getNumCitiesPrereq()`, XML `<iCitiesPrereq>`.
    pub num_cities_prereq: i32,
    /// C++ `getNumTeamsPrereq()`, XML `<iTeamsPrereq>`.
    pub num_teams_prereq: i32,
    /// C++ `getUnitLevelPrereq()`, XML `<iLevelPrereq>`.
    pub unit_level_prereq: i32,
    pub min_latitude: i32,
    /// Defaults to 90 when absent.
    pub max_latitude: i32,
    pub great_people_rate_modifier: i32,
    pub great_general_rate_modifier: i32,
    pub domestic_great_general_rate_modifier: i32,
    pub global_great_people_rate_modifier: i32,
    pub anarchy_modifier: i32,
    pub golden_age_modifier: i32,
    pub global_hurry_modifier: i32,
    /// C++ `getFreeExperience()`, XML `<iExperience>`.
    pub free_experience: i32,
    /// C++ `getGlobalFreeExperience()`, XML `<iGlobalExperience>`.
    pub global_free_experience: i32,
    pub food_kept: i32,
    pub airlift: i32,
    pub air_modifier: i32,
    pub air_unit_capacity: i32,
    pub nuke_modifier: i32,
    pub nuke_explosion_rand: i32,
    pub free_specialist: i32,
    pub area_free_specialist: i32,
    pub global_free_specialist: i32,
    pub happiness: i32,
    pub area_happiness: i32,
    pub global_happiness: i32,
    pub state_religion_happiness: i32,
    pub worker_speed_modifier: i32,
    pub military_production_modifier: i32,
    pub space_production_modifier: i32,
    pub global_space_production_modifier: i32,
    pub trade_routes: i32,
    pub coastal_trade_routes: i32,
    pub global_trade_routes: i32,
    pub trade_route_modifier: i32,
    pub foreign_trade_route_modifier: i32,
    /// C++ `getAssetValue()`, XML `<iAsset>`.
    pub asset_value: i32,
    /// C++ `getPowerValue()`, XML `<iPower>`.
    pub power_value: i32,
    pub great_people_rate_change: i32,
    /// C++ `getConquestProbability()`, XML `<iConquestProb>`.
    pub conquest_probability: i32,
    pub maintenance_modifier: i32,
    pub war_weariness_modifier: i32,
    pub global_war_weariness_modifier: i32,
    pub enemy_war_weariness_modifier: i32,
    pub heal_rate_change: i32,
    pub health: i32,
    pub area_health: i32,
    pub global_health: i32,
    pub global_population_change: i32,
    pub free_techs: i32,
    /// C++ `getDefenseModifier()`, XML `<iDefense>`.
    pub defense_modifier: i32,
    /// C++ `getBombardDefenseModifier()`, XML `<iBombardDefense>`.
    pub bombard_defense_modifier: i32,
    /// Not in C++: XML `<iSentinels>` (docs/SIEGES.md): the Sentinels a city with this building has
    /// (the most any of its buildings gives).
    pub sentinels: i32,
    /// C++ `getAllCityDefenseModifier()`, XML `<iAllCityDefense>`.
    pub all_city_defense_modifier: i32,
    /// This game's (docs/CARDS.md, Prize Courts): the owner's ships capture the enemy ships they
    /// defeat with this chance in percent (the Boarding promotion's capture; the most any of its
    /// buildings gives).
    pub naval_capture_percent: i32,
    /// C++ `getEspionageDefenseModifier()`, XML `<iEspionageDefense>`.
    pub espionage_defense_modifier: i32,

    // ---- bools ----
    pub team_share: bool,
    pub water: bool,
    pub river: bool,
    pub power: bool,
    pub dirty_power: bool,
    pub area_clean_power: bool,
    /// C++ `isAreaBorderObstacle()`, XML `<bBorderObstacle>`.
    pub area_border_obstacle: bool,
    /// This game's (docs/CARDS.md, Wall of the Kings): land units at war with the owner (and
    /// barbarians) that enter the owner's borders end their move there.
    pub border_stops_enemies: bool,
    /// This game's (docs/CARDS.md, Harbor Chain): ships at war with the owner (and barbarian
    /// ships) cannot enter the water plots next to the city.
    pub blocks_enemy_ships: bool,
    /// This game's (docs/CARDS.md, Hall of Judgment): the cities the owner conquers keep every
    /// building that can be captured, and lose no population.
    pub conquest_keeps_all: bool,
    pub force_team_vote_eligible: bool,
    pub capital: bool,
    pub government_center: bool,
    pub golden_age: bool,
    pub map_centering: bool,
    pub no_unhappiness: bool,
    pub no_unhealthy_population: bool,
    pub building_only_healthy: bool,
    pub never_capture: bool,
    pub nuke_immune: bool,
    /// C++ `isPrereqReligion()` (named apart from the religion [`Self::prereq_religion`]).
    pub is_prereq_religion: bool,
    pub center_in_city: bool,
    /// C++ `isStateReligion()` (named apart from the religion [`Self::state_religion`]).
    pub is_state_religion: bool,
    pub allows_nukes: bool,

    // ---- FfH additions ----
    pub apply_free_promotion_on_move: bool,
    pub equipment: bool,
    pub hide_units: bool,
    pub no_civic_anger: bool,
    pub requires_caster: bool,
    pub see_invisible: bool,
    pub unhappy_production: bool,
    pub free_promotion_pick: i32,
    pub free_bonus2: Option<BonusType>,
    pub free_bonus3: Option<BonusType>,
    pub global_resist_enemy_modify: i32,
    pub global_resist_modify: i32,
    pub modify_global_counter: i32,
    pub plot_radius: i32,
    /// C++ `getPrereqCiv()`: always `None`.
    ///
    /// C++ quirk: `read()` only stashes `<PrereqCiv>` for `readPass3()`, and the loop that
    /// would call `CvBuildingInfo::readPass3()` in `LoadPreMenuGlobals` is commented out
    /// (`/** ... **/`), so `m_iPrereqCiv` keeps its constructor value `NO_CIVILIZATION` and
    /// the `CvCity::canConstruct` check never fires. No gameplay effect in FFH2 0.41: every
    /// building with a `<PrereqCiv>` sits in a `BUILDINGCLASS_CIV_BUILDINGn` class whose
    /// default building is `NONE`, so only civs mapping the class to it can build it.
    /// The XML value is kept in [`Self::prereq_civ_xml`].
    pub prereq_civ: Option<CivilizationType>,
    /// `<PrereqCiv>` as written in the XML (the string C++ keeps in
    /// `m_aszExtraXMLforPass3`), resolved. Never applied by the C++ -- rules code must use
    /// [`Self::prereq_civ`]; this is for inspection and documented deviations only.
    pub prereq_civ_xml: Option<CivilizationType>,
    pub prereq_trait: Option<TraitType>,
    pub remove_promotion: Option<PromotionType>,
    pub resist_magic: i32,
    /// C++ `getSpecialistCommerceChange(i, j)`: per-specialist commerce change.
    pub specialist_commerce_change: IdxVec<SpecialistType, [i32; CommerceType::NUM]>,
    /// C++ `isVictoryBuilding()` (Better AI), XML `<bVictoryBuilding>`.
    pub victory_building: bool,

    // ---- arrays ----
    /// C++ `getPrereqAndTechs(i)`, XML `<TechTypes>`: `NUM_BUILDING_AND_TECH_PREREQS` (3)
    /// slots, `None`-filled.
    pub prereq_and_techs: Vec<Option<TechType>>,
    /// C++ `getPrereqOrBonuses(i)`, XML `<PrereqBonuses>`: `NUM_BUILDING_PREREQ_OR_BONUSES`
    /// (5) slots, `None`-filled.
    pub prereq_or_bonuses: Vec<Option<BonusType>>,
    /// C++ `getProductionTraits(i)`: production modifier (%) for players with the trait.
    pub production_traits: IdxVec<TraitType, i32>,
    /// C++ `getHappinessTraits(i)`: extra happiness for players with the trait.
    pub happiness_traits: IdxVec<TraitType, i32>,
    pub sea_plot_yield_change: [i32; YieldType::NUM],
    pub river_plot_yield_change: [i32; YieldType::NUM],
    pub global_sea_plot_yield_change: [i32; YieldType::NUM],
    pub yield_change: [i32; YieldType::NUM],
    pub yield_modifier: [i32; YieldType::NUM],
    pub power_yield_modifier: [i32; YieldType::NUM],
    pub area_yield_modifier: [i32; YieldType::NUM],
    pub global_yield_modifier: [i32; YieldType::NUM],
    pub commerce_change: [i32; CommerceType::NUM],
    pub obsolete_safe_commerce_change: [i32; CommerceType::NUM],
    pub commerce_change_double_time: [i32; CommerceType::NUM],
    pub commerce_modifier: [i32; CommerceType::NUM],
    pub global_commerce_modifier: [i32; CommerceType::NUM],
    pub specialist_extra_commerce: [i32; CommerceType::NUM],
    pub state_religion_commerce: [i32; CommerceType::NUM],
    pub commerce_happiness: [i32; CommerceType::NUM],
    pub religion_change: IdxVec<ReligionType, i32>,
    pub specialist_count: IdxVec<SpecialistType, i32>,
    pub free_specialist_count: IdxVec<SpecialistType, i32>,
    pub bonus_health_changes: IdxVec<BonusType, i32>,
    pub bonus_happiness_changes: IdxVec<BonusType, i32>,
    pub bonus_production_modifier: IdxVec<BonusType, i32>,
    pub unit_combat_free_experience: IdxVec<UnitCombatType, i32>,
    pub domain_free_experience: IdxVec<DomainType, i32>,
    pub domain_production_modifier: IdxVec<DomainType, i32>,
    pub building_happiness_changes: IdxVec<BuildingClassType, i32>,
    /// C++ `getPrereqNumOfBuildingClass(i)`, XML `<PrereqBuildingClasses>`.
    pub prereq_num_of_building_class: IdxVec<BuildingClassType, i32>,
    pub flavor_value: IdxVec<FlavorType, i32>,
    pub improvement_free_specialist: IdxVec<ImprovementType, i32>,
    /// C++ `isCommerceFlexible(i)`, XML `<CommerceFlexibles>`.
    pub commerce_flexible: [bool; CommerceType::NUM],
    /// C++ `isCommerceChangeOriginalOwner(i)`, XML `<CommerceChangeOriginalOwners>`.
    pub commerce_change_original_owner: [bool; CommerceType::NUM],
    /// C++ `isBuildingClassNeededInCity(i)`, XML `<BuildingClassNeededs>`.
    pub building_class_needed_in_city: IdxVec<BuildingClassType, bool>,
    /// C++ `getSpecialistYieldChange(i, j)`.
    pub specialist_yield_change: IdxVec<SpecialistType, [i32; YieldType::NUM]>,
    /// C++ `getBonusYieldModifier(i, j)`.
    pub bonus_yield_modifier: IdxVec<BonusType, [i32; YieldType::NUM]>,
}

/// C++ `CvBuildingInfo::read` (+ the never-called `readPass3`, see `prereq_civ`).
pub fn read_building(rd: &Rd) -> BuildingInfo {
    BuildingInfo {
        base: InfoBase::read(rd),

        tier: rd.rf("variant_tier"),
        variant_of: rd.rf("variant_of"),
        tier_variants: Vec::new(),
        unlocked_unit_experience: rd.int("unlocked_unit_experience"),
        unlocked_unit_production_modifier: rd.int("unlocked_unit_production_modifier"),
        unlocked_unit_promotion: rd.rf("unlocked_unit_promotion"),
        extra_free_promotion: rd.rf("extra_free_promotion"),
        bonus_yield_change: keyed_arrays(rd, "bonus_yield_changes", "bonus_yield_change", "bonus_type", "yield_changes"),
        population_commerce: rd.commerces("population_commerces"),
        sentinel_repair_change: rd.int("sentinel_repair_change"),
        battle: crate::infos::read_building_battle(rd, &rd.string("type")),
        exclusive_with: rd.ref_list::<BuildingClassType>("exclusive_with"),
        legion_cost_percent: rd.int("legion_cost_percent"),

        global_counter_commerce: rd.commerces("global_counter_commerces"),
        doom_sight: rd.int("doom_sight"),

        building_class_type: rd.rf("building_class"),
        special_building_type: rd.rf("special_building_type"),
        holy_city: rd.rf("holy_city"),
        religion_type: rd.rf("religion_type"),
        state_religion: rd.rf("state_religion"),
        prereq_religion: rd.rf("prereq_religion"),
        prereq_corporation: rd.rf("prereq_corporation"),
        founds_corporation: rd.rf("founds_corporation"),
        global_religion_commerce: rd.rf("global_religion_commerce"),
        global_corporation_commerce: rd.rf("global_corporation_commerce"),
        victory_prereq: rd.rf("victory_prereq"),
        free_start_era: rd.rf("free_start_era"),
        max_start_era: rd.rf("max_start_era"),
        obsolete_tech: rd.rf("obsolete_tech"),
        prereq_and_tech: rd.rf("prereq_tech"),
        prereq_and_techs: rd.ref_list_fixed("tech_types", NUM_BUILDING_AND_TECH_PREREQS),
        prereq_and_bonus: rd.rf("bonus"),
        prereq_or_bonuses: rd.ref_list_fixed("prereq_bonuses", NUM_BUILDING_PREREQ_OR_BONUSES),
        production_traits: rd.pairs_int("production_traits", 0),
        happiness_traits: rd.pairs_int("happiness_traits", 0),
        no_bonus: rd.rf("no_bonus"),
        power_bonus: rd.rf("power_bonus"),
        free_bonus: rd.rf("free_bonus"),
        num_free_bonuses: rd.int("num_free_bonuses"),
        free_building_class: rd.rf("free_building"),
        free_promotion: rd.rf("free_promotion"),
        civic_option: rd.rf("civic_option"),
        great_people_unit_class: rd.rf("great_people_unit_class"),
        vote_source_type: rd.rf("diplo_vote_type"),

        great_people_rate_change: rd.int("great_people_rate_change"),
        team_share: rd.bool("team_share"),
        water: rd.bool("water"),
        river: rd.bool("river"),
        power: rd.bool("power"),
        dirty_power: rd.bool("dirty_power"),
        area_clean_power: rd.bool("area_clean_power"),
        area_border_obstacle: rd.bool("border_obstacle"),
        border_stops_enemies: rd.bool("border_stops_enemies"),
        blocks_enemy_ships: rd.bool("blocks_enemy_ships"),
        conquest_keeps_all: rd.bool("conquest_keeps_all"),
        force_team_vote_eligible: rd.bool("force_team_vote_eligible"),
        capital: rd.bool("capital"),
        government_center: rd.bool("government_center"),
        golden_age: rd.bool("golden_age"),
        allows_nukes: rd.bool("allows_nukes"),
        map_centering: rd.bool("map_centering"),
        no_unhappiness: rd.bool("no_unhappiness"),
        no_unhealthy_population: rd.bool("no_unhealthy_population"),
        building_only_healthy: rd.bool("building_only_healthy"),
        never_capture: rd.bool("never_capture"),
        nuke_immune: rd.bool("nuke_immune"),
        is_prereq_religion: rd.bool("is_prereq_religion"),
        center_in_city: rd.bool("center_in_city"),
        is_state_religion: rd.bool("is_state_religion"),
        ai_weight: rd.int("ai_weight"),
        production_cost: rd.int("cost"),
        hurry_cost_modifier: rd.int("hurry_cost_modifier"),
        hurry_anger_modifier: rd.int("hurry_anger_modifier"),
        advanced_start_cost: rd.int("advanced_start_cost"),
        advanced_start_cost_increase: rd.int("advanced_start_cost_increase"),
        min_area_size: rd.int("min_area_size"),
        conquest_probability: rd.int("conquest_prob"),
        num_cities_prereq: rd.int("cities_prereq"),
        num_teams_prereq: rd.int("teams_prereq"),
        unit_level_prereq: rd.int("level_prereq"),
        min_latitude: rd.int("min_latitude"),
        max_latitude: rd.int_or("max_latitude", 90),
        great_people_rate_modifier: rd.int("great_people_rate_modifier"),
        great_general_rate_modifier: rd.int("great_general_rate_modifier"),
        domestic_great_general_rate_modifier: rd.int("domestic_great_general_rate_modifier"),
        global_great_people_rate_modifier: rd.int("global_great_people_rate_modifier"),
        anarchy_modifier: rd.int("anarchy_modifier"),
        golden_age_modifier: rd.int("golden_age_modifier"),
        global_hurry_modifier: rd.int("global_hurry_modifier"),
        free_experience: rd.int("experience"),
        global_free_experience: rd.int("global_experience"),
        food_kept: rd.int("food_kept"),
        airlift: rd.int("airlift"),
        air_modifier: rd.int("air_modifier"),
        air_unit_capacity: rd.int("air_unit_capacity"),
        nuke_modifier: rd.int("nuke_modifier"),
        nuke_explosion_rand: rd.int("nuke_explosion_rand"),
        free_specialist: rd.int("free_specialist"),
        area_free_specialist: rd.int("area_free_specialist"),
        global_free_specialist: rd.int("global_free_specialist"),
        maintenance_modifier: rd.int("maintenance_modifier"),
        war_weariness_modifier: rd.int("war_weariness_modifier"),
        global_war_weariness_modifier: rd.int("global_war_weariness_modifier"),
        enemy_war_weariness_modifier: rd.int("enemy_war_weariness_modifier"),
        heal_rate_change: rd.int("heal_rate_change"),
        health: rd.int("health"),
        area_health: rd.int("area_health"),
        global_health: rd.int("global_health"),
        happiness: rd.int("happiness"),
        area_happiness: rd.int("area_happiness"),
        global_happiness: rd.int("global_happiness"),
        state_religion_happiness: rd.int("state_religion_happiness"),
        worker_speed_modifier: rd.int("worker_speed_modifier"),
        military_production_modifier: rd.int("military_production_modifier"),
        space_production_modifier: rd.int("space_production_modifier"),
        global_space_production_modifier: rd.int("global_space_production_modifier"),
        trade_routes: rd.int("trade_routes"),
        coastal_trade_routes: rd.int("coastal_trade_routes"),
        global_trade_routes: rd.int("global_trade_routes"),
        trade_route_modifier: rd.int("trade_route_modifier"),
        foreign_trade_route_modifier: rd.int("foreign_trade_route_modifier"),
        global_population_change: rd.int("global_population_change"),
        free_techs: rd.int("free_techs"),
        defense_modifier: rd.int("defense"),
        bombard_defense_modifier: rd.int("bombard_defense"),
        sentinels: rd.int("sentinels"),
        all_city_defense_modifier: rd.int("all_city_defense"),
        naval_capture_percent: rd.int("naval_capture_percent"),
        espionage_defense_modifier: rd.int("espionage_defense"),
        asset_value: rd.int("asset"),
        power_value: rd.int("power_value"),

        apply_free_promotion_on_move: rd.bool("apply_free_promotion_on_move"),
        equipment: rd.bool("equipment"),
        hide_units: rd.bool("hide_units"),
        no_civic_anger: rd.bool("no_civic_anger"),
        requires_caster: rd.bool("requires_caster"),
        see_invisible: rd.bool("see_invisible"),
        unhappy_production: rd.bool("unhappy_production"),
        free_promotion_pick: rd.int("free_promotion_pick"),
        free_bonus2: rd.rf("free_bonus2"),
        free_bonus3: rd.rf("free_bonus3"),
        global_resist_enemy_modify: rd.int("global_resist_enemy_modify"),
        global_resist_modify: rd.int("global_resist_modify"),
        modify_global_counter: rd.int("modify_global_counter"),
        plot_radius: rd.int("plot_radius"),
        // C++ quirk: readPass3 never runs for buildings (see the field docs).
        prereq_civ: None,
        prereq_civ_xml: rd.rf("prereq_civ"),
        prereq_trait: rd.rf("prereq_trait"),
        remove_promotion: rd.rf("remove_promotion"),
        resist_magic: rd.int("resist_magic"),
        victory_building: rd.bool("victory_building"),

        sea_plot_yield_change: rd.yields("sea_plot_yield_changes"),
        river_plot_yield_change: rd.yields("river_plot_yield_changes"),
        global_sea_plot_yield_change: rd.yields("global_sea_plot_yield_changes"),
        yield_change: rd.yields("yield_changes"),
        yield_modifier: rd.yields("yield_modifiers"),
        power_yield_modifier: rd.yields("power_yield_modifiers"),
        area_yield_modifier: rd.yields("area_yield_modifiers"),
        global_yield_modifier: rd.yields("global_yield_modifiers"),
        commerce_change: rd.commerces("commerce_changes"),
        obsolete_safe_commerce_change: rd.commerces("obsolete_safe_commerce_changes"),
        commerce_change_double_time: rd.commerces("commerce_change_double_times"),
        commerce_modifier: rd.commerces("commerce_modifiers"),
        global_commerce_modifier: rd.commerces("global_commerce_modifiers"),
        specialist_extra_commerce: rd.commerces("specialist_extra_commerces"),
        state_religion_commerce: rd.commerces("state_religion_commerces"),
        commerce_happiness: rd.commerces("commerce_happinesses"),
        religion_change: rd.pairs_int("religion_changes", 0),
        specialist_count: rd.pairs_int("specialist_counts", 0),
        free_specialist_count: rd.pairs_int("free_specialist_counts", 0),
        commerce_flexible: commerce_bools(rd, "commerce_flexibles"),
        commerce_change_original_owner: commerce_bools(rd, "commerce_change_original_owners"),
        bonus_health_changes: rd.pairs_int("bonus_health_changes", 0),
        bonus_happiness_changes: rd.pairs_int("bonus_happiness_changes", 0),
        bonus_production_modifier: rd.pairs_int("bonus_production_modifiers", 0),
        unit_combat_free_experience: rd.pairs_int("unit_combat_free_experiences", 0),
        domain_free_experience: rd.pairs_int("domain_free_experiences", 0),
        domain_production_modifier: rd.pairs_int("domain_production_modifiers", 0),
        prereq_num_of_building_class: rd.pairs_int("prereq_building_classes", 0),
        building_class_needed_in_city: rd.pairs_bool("building_class_neededs"),
        specialist_yield_change: keyed_arrays(rd, "specialist_yield_changes", "specialist_yield_change", "specialist_type", "yield_changes"),
        bonus_yield_modifier: keyed_arrays(rd, "bonus_yield_modifiers", "bonus_yield_modifier", "bonus_type", "yield_modifiers"),
        flavor_value: rd.pairs_int("flavors", 0),
        improvement_free_specialist: rd.pairs_int("improvement_free_specialists", 0),
        building_happiness_changes: rd.pairs_int("building_happiness_changes", 0),
        specialist_commerce_change: keyed_arrays(rd, "specialist_commerce_changes", "specialist_commerce_change", "specialist_type", "commerce_changes"),
    }
}

/// C++ `SetCommerce(&m_pbX)` on a bool list (or `InitList(.., NUM_COMMERCE_TYPES)` when absent).
fn commerce_bools(rd: &Rd, tag: &str) -> [bool; CommerceType::NUM] {
    let v = rd.positional_bools(tag, CommerceType::NUM);
    [v[0], v[1], v[2], v[3]]
}

/// The items of the list `list` (`entry` names what each item is, for the reader's sake).
fn list_entries<'a>(rd: &Rd<'a>, list: &str, _entry: &str) -> Vec<Rd<'a>> {
    match rd.child(list) {
        Some(l) => l.children().collect(),
        None => Vec::new(),
    }
}

/// 2D array keyed by an info type with `N` positional values per key: C++
/// `Init2DIntList(&m_ppai, GC.getNumXInfos(), N)` (all zeros) followed by the
/// `SpecialistYieldChanges` / `BonusYieldModifiers` / `SpecialistCommerceChanges` loops of
/// `CvBuildingInfo::read`: per entry, `<key_tag>` is looked up and, if valid, its row is
/// replaced by the `SetYields`/`SetCommerce` list `<vals_tag>` (zeros when that is absent).
fn keyed_arrays<K: InfoId, const N: usize>(
    rd: &Rd,
    list: &str,
    entry: &str,
    key_tag: &str,
    vals_tag: &str,
) -> IdxVec<K, [i32; N]> {
    let mut out = IdxVec::from_elem([0; N], rd.reg().count::<K>());
    for e in list_entries(rd, list, entry) {
        // C++ skips invalid keys for yields; for SpecialistCommerceChanges an invalid key
        // would index the array with -1 (undefined behaviour) -- skipped here as well.
        if let Some(k) = e.rf::<K>(key_tag) {
            let v = e.positional_ints(vals_tag, N);
            out[k].copy_from_slice(&v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Ctx, Registry};

    fn registry() -> Registry {
        let mut reg = Registry::new();
        let classes: [(&'static str, &[&str]); 5] = [
            ("BuildingClassInfo", &["BUILDINGCLASS_A"]),
            ("BuildingInfo", &["BUILDING_A"]),
            ("SpecialistInfo", &["SPECIALIST_A", "SPECIALIST_B"]),
            ("CivilizationInfo", &["CIVILIZATION_T"]),
            ("TechInfo", &["TECH_A"]),
        ];
        for (class, names) in classes {
            reg.register_class(class, names.iter().map(|s| s.to_string()).collect());
        }
        reg
    }

    /// Runs a reader on `json` against [`registry`]; returns the result and the warnings.
    fn read<T>(json: &str, f: fn(&Rd) -> T) -> (T, Vec<String>) {
        let reg = registry();
        let value = crate::jsonc::parse(json).expect("test json");
        let cx = Ctx::new(&reg, "test.jsonc");
        let v = f(&Rd::new(&value, &cx));
        (v, cx.warnings.into_inner())
    }

    #[test]
    fn building_defaults_and_prereq_civ_quirk() {
        let json = r#"{"type": "BUILDING_A", "prereq_civ": "CIVILIZATION_T", "tech_types": ["TECH_A"]}"#;
        let (b, w) = read(json, read_building);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(b.prereq_civ, None);
        assert_eq!(b.prereq_civ_xml, Some(CivilizationType(0)));
        assert_eq!(b.max_latitude, 90);
        assert_eq!(b.prereq_and_techs, vec![Some(TechType(0)), None, None]);
        assert_eq!(b.prereq_or_bonuses, vec![None; 5]);
        assert_eq!(b.specialist_yield_change.raw, vec![[0; 3]; 2]);
        assert_eq!(b.yield_change, [0; 3]);
        assert_eq!(b.commerce_flexible, [false; 4]);
        assert!(!b.victory_building);
    }

    #[test]
    fn keyed_2d_lists_follow_the_cxx_loop() {
        let json = r#"{
            "specialist_yield_changes": [
                {"specialist_type": "SPECIALIST_A", "yield_changes": [9]},
                {"specialist_type": "SPECIALIST_B", "yield_changes": [1, 2]},
                {"specialist_type": "SPECIALIST_NOPE", "yield_changes": [5]},
                {"specialist_type": "SPECIALIST_A"}
            ],
            "specialist_commerce_changes": [
                {"specialist_type": "SPECIALIST_A", "commerce_changes": [0, 3]}
            ],
            "commerce_flexibles": [false, true]
        }"#;
        let (b, w) = read(json, read_building);
        // Unknown keys are skipped (with a warning); a later entry replaces the row, and one
        // without "yield_changes" replaces it with zeros.
        assert_eq!(b.specialist_yield_change.raw, vec![[0, 0, 0], [1, 2, 0]]);
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(b.specialist_commerce_change.raw, vec![[0, 3, 0, 0], [0; 4]]);
        assert_eq!(b.commerce_flexible, [false, true, false, false]);
    }

    #[test]
    fn building_class_and_special_building() {
        let json = r#"{"default_building": "BUILDING_A", "max_player_instances": -1}"#;
        let (c, w) = read(json, read_building_class);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.default_building_index, Some(BuildingType(0)));
        // Absent instance limits read as 0 (C++ GetChildXmlValByName default), not -1.
        assert_eq!((c.max_global_instances, c.max_team_instances, c.max_player_instances), (0, 0, -1));
        let (c, _) = read(r#"{"default_building": "NONE"}"#, read_building_class);
        assert_eq!(c.default_building_index, None);
        let (s, w) = read(r#"{"tech_prereq": "TECH_A"}"#, read_special_building);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(s.tech_prereq, Some(TechType(0)));
        assert!(!s.valid);
    }
}
