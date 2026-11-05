//! Info classes: EventInfo, EventTriggerInfo (the random event system).
//!
//! Port of `CvEventInfo::read` + `CvEventInfo::readPass2` and `CvEventTriggerInfo::read`
//! (CvInfos.cpp), plus the `BuildingYieldChange` / `BuildingCommerceChange` structs of
//! CvStructs.h.
//!
//! C++ reads events.jsonc in two passes because `AdditionalEvents`, `EventTimes` and
//! `ClearEvents` name other events (`readPass2`). Our registry knows every type up front, so
//! [`read_event`] resolves them directly. Every other reference names a class that C++ loads
//! before the events, so resolving it here gives the same result.
//!
//! Naming: scalars and dense arrays use the C++ getter name (`getAdditionalEventChance(i)` ->
//! `additional_event_chance`). Variable-length lists (`std::vector`) use the plural of their
//! count getter, so `getNumEvents()` is `events.len()` and `getEvent(i)` is `events[i]`.
//!
//! Not ported (UI only): `CvEventInfo::m_szOtherPlayerPopup`, `m_szQuestFailText` and
//! `m_szLocalInfoText` (popup/message text keys). Trigger texts and world-news keys are kept:
//! C++ picks one of them with `getSorenRandNum`, so their counts (and the trigger texts' eras)
//! advance the synchronized RNG.

use crate::enums::*;
use crate::idx::{opt_from_i32, IdxVec};
use crate::infos::InfoBase;
use crate::registry::{InfoId, Rd};

/// C++ `BuildingYieldChange` (CvStructs.h): one `<BuildingExtraYield>` entry of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildingYieldChange {
    /// C++ `eBuildingClass`
    pub building_class: BuildingClassType,
    /// C++ `eYield`
    pub yield_type: YieldType,
    /// C++ `iChange`
    pub change: i32,
}

/// C++ `BuildingCommerceChange` (CvStructs.h): one `<BuildingExtraCommerce>` entry of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildingCommerceChange {
    /// C++ `eBuildingClass`
    pub building_class: BuildingClassType,
    /// C++ `eCommerce`
    pub commerce_type: CommerceType,
    /// C++ `iChange`
    pub change: i32,
}

/// C++ `CvEventInfo` (events.jsonc): one choice of a triggered event and its effects.
#[derive(Debug, Clone, Default)]
pub struct EventInfo {
    pub base: InfoBase,

    pub quest: bool,
    pub global: bool,
    pub team: bool,
    /// C++ `isCityEffect()`, read from `<bPickCity>`.
    pub city_effect: bool,
    /// C++ `isOtherPlayerCityEffect()`, read from `<bPickOtherPlayerCity>`.
    pub other_player_city_effect: bool,
    pub gold_to_player: bool,
    pub golden_age: bool,
    pub declare_war: bool,
    pub disband_unit: bool,

    pub gold: i32,
    pub random_gold: i32,
    pub culture: i32,
    pub espionage_points: i32,
    /// C++ `getTech()`: tech that receives `tech_percent` of its cost.
    pub tech: Option<TechType>,
    pub tech_percent: i32,
    pub tech_cost_percent: i32,
    pub tech_min_turns_left: i32,
    pub prereq_tech: Option<TechType>,
    /// C++ `getUnitClass()`: class of the free units.
    pub unit_class: Option<UnitClassType>,
    /// C++ `getNumUnits()`, read from `<iNumFreeUnits>`.
    pub num_units: i32,
    pub building_class: Option<BuildingClassType>,
    pub building_change: i32,
    pub happy: i32,
    pub health: i32,
    pub hurry_anger: i32,
    pub happy_turns: i32,
    pub food: i32,
    pub food_percent: i32,
    /// C++ `getFeature()`, read from `<FeatureType>`.
    pub feature: Option<FeatureType>,
    pub feature_change: i32,
    /// C++ `getImprovement()`, read from `<ImprovementType>`.
    pub improvement: Option<ImprovementType>,
    pub improvement_change: i32,
    /// C++ `getBonus()`, read from `<BonusType>`.
    pub bonus: Option<BonusType>,
    pub bonus_change: i32,
    /// C++ `getRoute()`, read from `<RouteType>`.
    pub route: Option<RouteType>,
    pub route_change: i32,
    pub bonus_revealed: Option<BonusType>,
    pub bonus_gift: Option<BonusType>,
    pub unit_experience: i32,
    pub unit_immobile_turns: i32,
    pub convert_own_cities: i32,
    pub convert_other_cities: i32,
    /// C++ `getMaxNumReligions()`; -1 = no limit.
    // C++ quirk: the constructor sets -1, but read() resets it to 0 when the tag is absent
    // (GetChildXmlValByName's default). Every entry in the data has the tag.
    pub max_num_religions: i32,
    pub our_attitude_modifier: i32,
    pub attitude_modifier: i32,
    pub their_enemy_attitude_modifier: i32,
    pub population_change: i32,
    pub revolt_turns: i32,
    pub min_pillage: i32,
    pub max_pillage: i32,
    pub unit_promotion: Option<PromotionType>,
    pub free_unit_support: i32,
    /// C++ `getInflationModifier()`, read from `<iInflationMod>`.
    pub inflation_modifier: i32,
    /// C++ `getSpaceProductionModifier()`, read from `<iSpaceProductionMod>`.
    pub space_production_modifier: i32,
    /// C++ `getAIValue()`
    pub ai_value: i32,

    // FfH additions
    pub crime: i32,
    pub global_counter: i32,
    pub prereq_alignment: Option<AlignmentType>,
    pub prereq_bonus: Option<BonusType>,
    pub prereq_civilization: Option<CivilizationType>,
    /// C++ `getPrereqCorporation()`; the FFH2 XML never sets `<PrereqCorporation>`.
    pub prereq_corporation: Option<CorporationType>,
    pub prereq_religion: Option<ReligionType>,
    pub prereq_state_religion: Option<ReligionType>,

    /// C++ `getTechFlavorValue(i)`, `<TechFlavors>`.
    pub tech_flavor_value: IdxVec<FlavorType, i32>,
    /// C++ `getPlotExtraYield(i)`, keyed `<PlotExtraYields>` pairs.
    pub plot_extra_yield: [i32; YieldType::NUM],
    /// C++ `getFreeSpecialistCount(i)`, `<FreeSpecialistCounts>`.
    pub free_specialist_count: IdxVec<SpecialistType, i32>,
    /// C++ `getAdditionalEventChance(i)`, `<AdditionalEvents>` (C++ readPass2).
    pub additional_event_chance: IdxVec<EventType, i32>,
    /// C++ `getAdditionalEventTime(i)`, `<EventTimes>` (C++ readPass2).
    pub additional_event_time: IdxVec<EventType, i32>,
    /// C++ `getClearEventChance(i)`, `<ClearEvents>` (C++ readPass2).
    pub clear_event_chance: IdxVec<EventType, i32>,
    /// C++ `getUnitCombatPromotion(i)`, `<UnitCombatPromotions>`.
    pub unit_combat_promotion: IdxVec<UnitCombatType, Option<PromotionType>>,
    /// C++ `getUnitClassPromotion(i)`, `<UnitClassPromotions>`.
    pub unit_class_promotion: IdxVec<UnitClassType, Option<PromotionType>>,

    /// C++ `m_aBuildingYieldChanges` (`<BuildingExtraYields>`), in document order;
    /// see [`EventInfo::building_yield_change`].
    pub building_yield_changes: Vec<BuildingYieldChange>,
    /// C++ `m_aBuildingCommerceChanges` (`<BuildingExtraCommerces>`), in document order;
    /// see [`EventInfo::building_commerce_change`].
    pub building_commerce_changes: Vec<BuildingCommerceChange>,
    /// C++ `m_aBuildingHappyChanges` (`<BuildingExtraHappies>`), in document order.
    pub building_happy_changes: Vec<(BuildingClassType, i32)>,
    /// C++ `m_aBuildingHealthChanges` (`<BuildingExtraHealths>`), in document order.
    pub building_health_changes: Vec<(BuildingClassType, i32)>,

    pub python_callback: String,
    pub python_expire_check: String,
    pub python_can_do: String,
    pub python_help: String,
    /// C++ `getUnitNameKey()`, `<UnitName>`: the event renames its unit (`CvUnit::applyEvent`).
    pub unit_name_key: String,
    /// C++ `getWorldNews(i)` / `getNumWorldNews()`; `CvPlayer::applyEvent` picks one with the game RNG.
    pub world_news: Vec<String>,
}

impl EventInfo {
    /// C++ `getBuildingYieldChange(iBuildingClass, iYield)`: the first matching entry, else 0.
    pub fn building_yield_change(&self, building_class: BuildingClassType, yield_type: YieldType) -> i32 {
        self.building_yield_changes
            .iter()
            .find(|c| c.building_class == building_class && c.yield_type == yield_type)
            .map_or(0, |c| c.change)
    }

    /// C++ `getBuildingCommerceChange(iBuildingClass, iCommerce)`: the first matching entry, else 0.
    pub fn building_commerce_change(&self, building_class: BuildingClassType, commerce_type: CommerceType) -> i32 {
        self.building_commerce_changes
            .iter()
            .find(|c| c.building_class == building_class && c.commerce_type == commerce_type)
            .map_or(0, |c| c.change)
    }

    /// C++ `getBuildingHappyChange(iBuildingClass)`: the first matching entry, else 0.
    pub fn building_happy_change(&self, building_class: BuildingClassType) -> i32 {
        first_change(&self.building_happy_changes, building_class)
    }

    /// C++ `getBuildingHealthChange(iBuildingClass)`: the first matching entry, else 0.
    pub fn building_health_change(&self, building_class: BuildingClassType) -> i32 {
        first_change(&self.building_health_changes, building_class)
    }
}

fn first_change(changes: &[(BuildingClassType, i32)], building_class: BuildingClassType) -> i32 {
    changes.iter().find(|(bc, _)| *bc == building_class).map_or(0, |&(_, change)| change)
}

/// C++ `CvEventInfo::read` + `CvEventInfo::readPass2`. The struct literal is in C++ read order.
pub fn read_event(rd: &Rd) -> EventInfo {
    EventInfo {
        base: InfoBase::read(rd),
        quest: rd.bool("quest"),
        global: rd.bool("global"),
        team: rd.bool("team"),
        city_effect: rd.bool("pick_city"),
        other_player_city_effect: rd.bool("pick_other_player_city"),
        gold_to_player: rd.bool("gold_to_player"),
        golden_age: rd.bool("golden_age"),
        declare_war: rd.bool("declare_war"),
        gold: rd.int("gold"),
        random_gold: rd.int("random_gold"),
        culture: rd.int("culture"),
        espionage_points: rd.int("espionage_points"),
        tech: rd.rf("tech"),
        tech_percent: rd.int("tech_percent"),
        tech_cost_percent: rd.int("tech_cost_percent"),
        tech_min_turns_left: rd.int("tech_min_turns_left"),
        prereq_tech: rd.rf("prereq_tech"),
        unit_class: rd.rf("unit_class"),
        num_units: rd.int("num_free_units"),
        disband_unit: rd.bool("disband_unit"),
        unit_experience: rd.int("unit_experience"),
        unit_immobile_turns: rd.int("unit_immobile_turns"),
        building_class: rd.rf("building_class"),
        building_change: rd.int("building_change"),

        happy: rd.int("happy"),
        health: rd.int("health"),
        hurry_anger: rd.int("hurry_anger"),
        happy_turns: rd.int("happy_turns"),
        food: rd.int("food"),
        food_percent: rd.int("food_percent"),

        feature: rd.rf("feature_type"),
        feature_change: rd.int("feature_change"),
        improvement: rd.rf("improvement_type"),
        improvement_change: rd.int("improvement_change"),
        bonus: rd.rf("bonus_type"),
        bonus_change: rd.int("bonus_change"),
        route: rd.rf("route_type"),
        route_change: rd.int("route_change"),
        bonus_revealed: rd.rf("bonus_revealed"),
        bonus_gift: rd.rf("bonus_gift"),

        tech_flavor_value: rd.pairs_int::<FlavorType>("tech_flavors", 0),
        plot_extra_yield: {
            // SetVariableListTagPair keyed by YieldInfo types (not positional like SetYields).
            let v = rd.pairs_int::<YieldType>("plot_extra_yields", 0);
            std::array::from_fn(|i| v.raw.get(i).copied().unwrap_or(0))
        },
        free_specialist_count: rd.pairs_int::<SpecialistType>("free_specialist_counts", 0),

        convert_own_cities: rd.int("convert_own_cities"),
        convert_other_cities: rd.int("convert_other_cities"),
        max_num_religions: rd.int("max_num_religions"),
        our_attitude_modifier: rd.int("our_attitude_modifier"),
        attitude_modifier: rd.int("attitude_modifier"),
        their_enemy_attitude_modifier: rd.int("their_enemy_attitude_modifier"),
        population_change: rd.int("population_change"),
        revolt_turns: rd.int("revolt_turns"),
        min_pillage: rd.int("min_pillage"),
        max_pillage: rd.int("max_pillage"),
        unit_promotion: rd.rf("unit_promotion"),
        free_unit_support: rd.int("free_unit_support"),
        inflation_modifier: rd.int("inflation_mod"),
        space_production_modifier: rd.int("space_production_mod"),
        ai_value: rd.int("ai_value"),

        crime: rd.int("crime"),
        global_counter: rd.int("global_counter"),
        prereq_alignment: rd.rf("prereq_alignment"),
        prereq_bonus: rd.rf("prereq_bonus"),
        prereq_civilization: rd.rf("prereq_civilization"),
        prereq_corporation: rd.rf("prereq_corporation"),
        prereq_religion: rd.rf("prereq_religion"),
        prereq_state_religion: rd.rf("prereq_state_religion"),

        // C++ reads a "NONE"-filled string list, then FindInInfoClass on every entry.
        unit_combat_promotion: rd.pairs_ref::<UnitCombatType, PromotionType>("unit_combat_promotions"),
        unit_class_promotion: rd.pairs_ref::<UnitClassType, PromotionType>("unit_class_promotions"),

        unit_name_key: rd.string("unit_name"),
        python_callback: rd.string("python_callback"),
        python_expire_check: rd.string("python_expire_check"),
        python_can_do: rd.string("python_can_do"),
        python_help: rd.string("python_help"),

        world_news: rd.string_list("world_news_texts"),

        building_yield_changes: read_building_yield_changes(rd),
        building_commerce_changes: read_building_commerce_changes(rd),
        building_happy_changes: read_building_changes(rd, "building_extra_happies"),
        building_health_changes: read_building_changes(rd, "building_extra_healths"),

        // readPass2
        additional_event_chance: rd.pairs_int::<EventType>("additional_events", 0),
        additional_event_time: rd.pairs_int::<EventType>("event_times", 0),
        clear_event_chance: rd.pairs_int::<EventType>("clear_events", 0),
    }
}

/// C++ `CvEventTriggerInfo` (event_triggers.jsonc): when an event may fire, and
/// which [`EventInfo`] choices it offers.
#[derive(Debug, Clone, Default)]
pub struct EventTriggerInfo {
    pub base: InfoBase,

    pub percent_games_active: i32,
    /// C++ `getProbability()`, read from `<iWeight>` (-1 for triggers fired only by Python
    /// or as follow-ups).
    pub probability: i32,
    pub num_units: i32,
    pub num_buildings: i32,
    pub num_units_global: i32,
    pub num_buildings_global: i32,
    pub num_plots_required: i32,
    /// C++ `getPlotType()`, a raw `PlotTypes` value in `<iPlotType>` (-1 = `NO_PLOT`).
    // C++ quirk: an absent tag reads as 0 = PLOT_PEAK, not NO_PLOT. Every entry has the tag.
    pub plot_type: Option<PlotType>,
    pub num_religions: i32,
    pub num_corporations: i32,
    pub other_player_share_borders: i32,
    pub other_player_has_tech: Option<TechType>,
    pub city_food_weight: i32,
    pub civic: Option<CivicType>,
    pub min_population: i32,
    pub max_population: i32,
    pub min_map_landmass: i32,
    pub min_our_landmass: i32,
    pub max_our_landmass: i32,
    pub min_difficulty: Option<HandicapType>,
    pub angry: i32,
    pub unhealthy: i32,
    pub unit_damaged_weight: i32,
    pub unit_distance_weight: i32,
    pub unit_experience_weight: i32,
    pub min_treasury: i32,

    /// C++ `getUnitRequired(i)` / `getNumUnitsRequired()`: unit *classes*.
    pub units_required: Vec<UnitClassType>,
    /// C++ `getBuildingRequired(i)` / `getNumBuildingsRequired()`: building *classes*.
    pub buildings_required: Vec<BuildingClassType>,
    /// C++ `getPrereqOrTechs(i)` / `getNumPrereqOrTechs()`, `<OrPreReqs>`.
    pub prereq_or_techs: Vec<TechType>,
    /// C++ `getPrereqAndTechs(i)` / `getNumPrereqAndTechs()`, `<AndPreReqs>`.
    pub prereq_and_techs: Vec<TechType>,
    /// C++ `getObsoleteTech(i)` / `getNumObsoleteTechs()`.
    pub obsolete_techs: Vec<TechType>,
    /// C++ `getEvent(i)` / `getNumEvents()`: the choices offered.
    pub events: Vec<EventType>,
    /// C++ `getPrereqEvent(i)` / `getNumPrereqEvents()`.
    pub prereq_events: Vec<EventType>,
    /// C++ `getFeatureRequired(i)` / `getNumFeaturesRequired()`. `None` (XML `NONE`) is a real
    /// entry: `CvPlot::canTrigger` compares entries with the plot's feature, so it matches a
    /// plot without one.
    pub features_required: Vec<Option<FeatureType>>,
    /// C++ `getTerrainRequired(i)` / `getNumTerrainsRequired()`.
    pub terrains_required: Vec<TerrainType>,
    /// C++ `getImprovementRequired(i)` / `getNumImprovementsRequired()`; `None` matches a plot
    /// without an improvement (see `features_required`).
    pub improvements_required: Vec<Option<ImprovementType>>,
    /// C++ `getBonusRequired(i)` / `getNumBonusesRequired()`; `None` matches a plot without a
    /// (visible) bonus.
    pub bonuses_required: Vec<Option<BonusType>>,
    /// C++ `getRouteRequired(i)` / `getNumRoutesRequired()`; `None` matches a plot without a route.
    pub routes_required: Vec<Option<RouteType>>,
    /// C++ `getReligionRequired(i)` / `getNumReligionsRequired()`.
    pub religions_required: Vec<ReligionType>,
    /// C++ `getCorporationRequired(i)` / `getNumCorporationsRequired()`.
    pub corporations_required: Vec<CorporationType>,

    /// C++ `getText(i)` / `getNumTexts()`: popup text keys. `CvPlayer::initTriggeredData` picks
    /// one of those valid for the current era with the game RNG.
    pub texts: Vec<String>,
    /// C++ `getTextEra(i)`, parallel to `texts`; `None` = any era.
    pub text_eras: Vec<Option<EraType>>,
    /// C++ `getWorldNews(i)` / `getNumWorldNews()`. Empty `<Text></Text>` entries are kept (as C++
    /// does): they still count for the RNG pick in `CvPlayer::initTriggeredData`.
    pub world_news: Vec<String>,

    pub single_player: bool,
    pub team: bool,
    pub recurring: bool,
    pub global: bool,
    pub pick_player: bool,
    pub other_player_war: bool,
    pub other_player_has_religion: bool,
    pub other_player_has_other_religion: bool,
    /// C++ `isOtherPlayerAI()`
    pub other_player_ai: bool,
    pub pick_city: bool,
    pub pick_other_player_city: bool,
    // C++ quirk: the constructor sets m_bShowPlot to true, but read() resets it to false when
    // `<bShowPlot>` is absent (GetChildXmlValByName's default). Every entry has the tag.
    pub show_plot: bool,
    pub units_on_plot: bool,
    pub own_plot: bool,
    pub pick_religion: bool,
    pub state_religion: bool,
    pub holy_city: bool,
    pub pick_corporation: bool,
    pub headquarters: bool,
    pub probability_unit_multiply: bool,
    pub probability_building_multiply: bool,
    /// C++ `isPrereqEventCity()`, read from `<bPrereqEventPlot>`.
    pub prereq_event_city: bool,

    pub python_callback: String,
    pub python_can_do: String,
    pub python_can_do_city: String,
    pub python_can_do_unit: String,

    // FfH additions
    pub prereq_crime: i32,
    pub prereq_global_counter: i32,
    pub prereq_alignment: Option<AlignmentType>,
    pub prereq_civilization_pleased: Option<CivilizationType>,
    pub prereq_leader: Option<LeaderHeadType>,
    pub prereq_trait: Option<TraitType>,
}

/// C++ `CvEventTriggerInfo::read`. The struct literal is in C++ read order.
pub fn read_event_trigger(rd: &Rd) -> EventTriggerInfo {
    let (texts, text_eras) = read_trigger_texts(rd);
    EventTriggerInfo {
        base: InfoBase::read(rd),
        percent_games_active: rd.int("percent_games_active"),
        probability: rd.int("weight"),

        num_units: rd.int("num_units"),
        num_buildings: rd.int("num_buildings"),
        num_units_global: rd.int("num_units_global"),
        num_buildings_global: rd.int("num_buildings_global"),

        num_plots_required: rd.int("num_plots_required"),
        plot_type: read_plot_type(rd),

        num_religions: rd.int("num_religions"),
        num_corporations: rd.int("num_corporations"),

        other_player_share_borders: rd.int("other_player_share_borders"),

        min_population: rd.int("min_population"),
        max_population: rd.int("max_population"),

        min_map_landmass: rd.int("min_map_landmass"),
        min_our_landmass: rd.int("min_our_landmass"),
        max_our_landmass: rd.int("max_our_landmass"),
        min_difficulty: rd.rf("min_difficulty"),
        angry: rd.int("angry"),
        unhealthy: rd.int("unhealthy"),
        unit_damaged_weight: rd.int("unit_damaged_weight"),
        unit_distance_weight: rd.int("unit_distance_weight"),
        unit_experience_weight: rd.int("unit_experience_weight"),
        min_treasury: rd.int("min_treasury"),

        units_required: ref_list_req(rd, "units_required"),
        buildings_required: ref_list_req(rd, "buildings_required"),
        prereq_or_techs: ref_list_req(rd, "or_pre_reqs"),
        prereq_and_techs: ref_list_req(rd, "and_pre_reqs"),

        other_player_has_tech: rd.rf("other_player_has_tech"),
        civic: rd.rf("civic"),

        obsolete_techs: ref_list_req(rd, "obsolete_techs"),
        events: ref_list_req(rd, "events"),
        prereq_events: ref_list_req(rd, "prereq_events"),
        features_required: ref_list_opt(rd, "features_required"),
        terrains_required: ref_list_req(rd, "terrains_required"),
        improvements_required: ref_list_opt(rd, "improvements_required"),
        bonuses_required: ref_list_opt(rd, "bonuses_required"),
        routes_required: ref_list_opt(rd, "routes_required"),
        religions_required: ref_list_req(rd, "religions_required"),
        corporations_required: ref_list_req(rd, "corporations_required"),

        texts,
        text_eras,
        world_news: rd.string_list("world_news_texts"),

        single_player: rd.bool("single_player"),
        team: rd.bool("team"),
        recurring: rd.bool("recurring"),
        global: rd.bool("global"),
        pick_player: rd.bool("pick_player"),
        other_player_war: rd.bool("other_player_war"),
        other_player_has_religion: rd.bool("other_player_has_religion"),
        other_player_has_other_religion: rd.bool("other_player_has_other_religion"),
        other_player_ai: rd.bool("other_player_ai"),
        pick_city: rd.bool("pick_city"),
        pick_other_player_city: rd.bool("pick_other_player_city"),
        show_plot: rd.bool("show_plot"),
        city_food_weight: rd.int("city_food_weight"),
        units_on_plot: rd.bool("units_on_plot"),
        own_plot: rd.bool("own_plot"),
        pick_religion: rd.bool("pick_religion"),
        state_religion: rd.bool("state_religion"),
        holy_city: rd.bool("holy_city"),
        pick_corporation: rd.bool("pick_corporation"),
        headquarters: rd.bool("headquarters"),
        probability_unit_multiply: rd.bool("probability_unit_multiply"),
        probability_building_multiply: rd.bool("probability_building_multiply"),
        prereq_event_city: rd.bool("prereq_event_plot"),

        python_callback: rd.string("python_callback"),
        python_can_do: rd.string("python_can_do"),
        python_can_do_city: rd.string("python_can_do_city"),
        python_can_do_unit: rd.string("python_can_do_unit"),

        prereq_crime: rd.int("prereq_crime"),
        prereq_global_counter: rd.int("prereq_global_counter"),
        prereq_alignment: rd.rf("prereq_alignment"),
        prereq_civilization_pleased: rd.rf("prereq_civilization_pleased"),
        prereq_leader: rd.rf("prereq_leader"),
        prereq_trait: rd.rf("prereq_trait"),
    }
}

// ---- readers for the C++ hand-written list loops (not covered by `Rd`) ----

/// Context for load warnings: the entry's `<Type>`.
fn entry_type(rd: &Rd) -> String {
    rd.string("type")
}

/// The C++ list loop (`SetToChildByTagName(tag)`, then `GetChildXmlVal`/`GetNextXmlVal` over the
/// children, pushing `FindInInfoClass(szTextVal)` for each), keeping `NONE` entries as `None`
/// exactly like the C++ -1 entries.
fn ref_list_opt<T: InfoId>(rd: &Rd, tag: &str) -> Vec<Option<T>> {
    match rd.child(tag) {
        Some(list) => list.children().map(|c| rd.resolve::<T>(c.text())).collect(),
        None => Vec::new(),
    }
}

/// The same C++ list loop for lists whose entries must name a type. C++ would keep a `NONE`
/// entry as -1 (still counted by `getNumX()`); it cannot be represented here, so it is
/// dropped with a load warning.
fn ref_list_req<T: InfoId>(rd: &Rd, tag: &str) -> Vec<T> {
    match rd.child(tag) {
        Some(list) => list.children().filter_map(|c| resolve_req::<T>(rd, c.text(), tag)).collect(),
        None => Vec::new(),
    }
}

/// `FindInInfoClass` for a value that must name a type: `NONE`/empty warns (C++ would keep -1),
/// an unknown name warns through [`Rd::resolve`].
fn resolve_req<T: InfoId>(rd: &Rd, s: &str, tag: &str) -> Option<T> {
    let s = s.trim();
    if s.is_empty() || s == "NONE" {
        rd.cx.warn(format!("{}: <{}> entry {:?} (C++ keeps it as -1)", entry_type(rd), tag, s));
        return None;
    }
    rd.resolve::<T>(s)
}

/// Entries of a C++ multi-value list loop: the child elements of each child of `<tag>`.
/// C++ reads them positionally (`GetChildXmlVal`, then `GetNextXmlVal`, ignoring tag names) and
/// skips an entry without children (`GetChildXmlVal` fails).
fn tuple_entries<'a>(rd: &Rd<'a>, tag: &str) -> Vec<Vec<Rd<'a>>> {
    match rd.child(tag) {
        Some(list) => list
            .children()
            .map(|entry| entry.children().collect::<Vec<_>>())
            .filter(|values| !values.is_empty())
            .collect(),
        None => Vec::new(),
    }
}

/// Text of the `i`-th value of an entry; "" when missing (C++ `GetNextXmlVal` clears the string).
fn text_at<'a>(values: &[Rd<'a>], i: usize) -> std::borrow::Cow<'a, str> {
    values.get(i).map_or(std::borrow::Cow::Borrowed(""), |v| v.scalar_text())
}

/// Int value of the `i`-th value of an entry; 0 when missing (C++ `GetNextXmlVal(int*)` default).
/// Parses like `Rd::int_or` (an int, else a float truncated toward zero).
fn int_at(rd: &Rd, values: &[Rd], i: usize, tag: &str) -> i32 {
    let s = text_at(values, i);
    if s.is_empty() {
        return 0;
    }
    s.parse::<i32>().ok().or_else(|| s.parse::<f64>().ok().map(|f| f as i32)).unwrap_or_else(|| {
        rd.cx.warn(format!("{}: bad int in <{}> entry: {:?}", entry_type(rd), tag, s));
        0
    })
}

/// C++ `BuildingExtraYields` loop: (building class, yield, change) per entry.
fn read_building_yield_changes(rd: &Rd) -> Vec<BuildingYieldChange> {
    const TAG: &str = "building_extra_yields";
    let mut out = Vec::new();
    for values in tuple_entries(rd, TAG) {
        let building_class = resolve_req::<BuildingClassType>(rd, &text_at(&values, 0), TAG);
        let yield_type = resolve_req::<YieldType>(rd, &text_at(&values, 1), TAG);
        let change = int_at(rd, &values, 2, TAG);
        if let (Some(building_class), Some(yield_type)) = (building_class, yield_type) {
            out.push(BuildingYieldChange { building_class, yield_type, change });
        }
    }
    out
}

/// C++ `BuildingExtraCommerces` loop: (building class, commerce, change) per entry.
fn read_building_commerce_changes(rd: &Rd) -> Vec<BuildingCommerceChange> {
    const TAG: &str = "building_extra_commerces";
    let mut out = Vec::new();
    for values in tuple_entries(rd, TAG) {
        let building_class = resolve_req::<BuildingClassType>(rd, &text_at(&values, 0), TAG);
        let commerce_type = resolve_req::<CommerceType>(rd, &text_at(&values, 1), TAG);
        let change = int_at(rd, &values, 2, TAG);
        if let (Some(building_class), Some(commerce_type)) = (building_class, commerce_type) {
            out.push(BuildingCommerceChange { building_class, commerce_type, change });
        }
    }
    out
}

/// C++ `BuildingExtraHappies` / `BuildingExtraHealths` loops: (building class, change) per entry.
fn read_building_changes(rd: &Rd, tag: &str) -> Vec<(BuildingClassType, i32)> {
    let mut out = Vec::new();
    for values in tuple_entries(rd, tag) {
        let building_class = resolve_req::<BuildingClassType>(rd, &text_at(&values, 0), tag);
        let change = int_at(rd, &values, 1, tag);
        if let Some(building_class) = building_class {
            out.push((building_class, change));
        }
    }
    out
}

/// C++ `TriggerTexts` loop: per `<TriggerText>`, the text key (`GetChildXmlVal`) and the era
/// (`GetNextXmlVal` + `FindInInfoClass`; missing or `NONE` -> -1 = any era).
fn read_trigger_texts(rd: &Rd) -> (Vec<String>, Vec<Option<EraType>>) {
    let mut texts = Vec::new();
    let mut eras = Vec::new();
    for values in tuple_entries(rd, "trigger_texts") {
        texts.push(text_at(&values, 0).to_string());
        eras.push(rd.resolve::<EraType>(&text_at(&values, 1)));
    }
    (texts, eras)
}

/// `<iPlotType>`: a raw C++ `PlotTypes` value (-1 = `NO_PLOT`).
fn read_plot_type(rd: &Rd) -> Option<PlotType> {
    let v = rd.int("plot_type");
    if v < -1 || v >= PlotType::NUM as i32 {
        rd.cx.warn(format!("{}: iPlotType {} is not a PlotTypes value", entry_type(rd), v));
    }
    opt_from_i32(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Ctx, Registry};

    fn registry() -> Registry {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut reg = Registry::new();
        reg.register_class("EventInfo", names(&["EVENT_A", "EVENT_B"]));
        reg.register_class("ImprovementInfo", names(&["IMPROVEMENT_FARM"]));
        reg.register_class("BuildingClassInfo", names(&["BUILDINGCLASS_X"]));
        reg.register_class("YieldInfo", names(YieldType::NAMES));
        reg.register_class("EraInfo", names(&["ERA_ANCIENT"]));
        reg
    }

    /// Edge cases of the C++ list loops that the shipped data never hits.
    #[test]
    fn trigger_reader_follows_cpp_edge_cases() {
        let reg = registry();
        let cx = Ctx::new(&reg, "test.jsonc");
        let value = crate::jsonc::parse(
            r#"{
                "type": "EVENTTRIGGER_T",
                "improvements_required": ["NONE", "IMPROVEMENT_FARM"],
                "events": ["EVENT_B", "NONE"],
                "trigger_texts": [["TXT_A", "ERA_ANCIENT"], [], ["TXT_B"]],
                "world_news_texts": [""]
            }"#,
        )
        .unwrap();
        let t = read_event_trigger(&Rd::new(&value, &cx));
        // NONE is a real entry in plot requirement lists (matches "no improvement").
        assert_eq!(t.improvements_required, vec![None, Some(ImprovementType(0))]);
        // NONE in a list of required types is dropped with a warning (C++ would keep -1).
        assert_eq!(t.events, vec![EventType(1)]);
        let warnings = cx.warnings.borrow();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("EVENTTRIGGER_T") && warnings[0].contains("events"));
        // An empty trigger text entry is skipped; a missing era is -1 (any era).
        assert_eq!(t.texts, vec!["TXT_A", "TXT_B"]);
        assert_eq!(t.text_eras, vec![Some(EraType(0)), None]);
        // An empty world-news text still counts.
        assert_eq!(t.world_news, vec![String::new()]);
        // Absent keys: GetChildXmlValByName's own defaults win over the C++ constructor's.
        assert!(!t.show_plot);
        assert_eq!(t.plot_type, Some(PlotType::PLOT_PEAK));
        assert_eq!(t.min_difficulty, None);
    }

    #[test]
    fn event_reader_follows_cpp_edge_cases() {
        let reg = registry();
        let cx = Ctx::new(&reg, "test.jsonc");
        let value = crate::jsonc::parse(
            r#"{
                "type": "EVENT_A",
                "building_extra_yields": [["BUILDINGCLASS_X", "YIELD_COMMERCE", 3], []],
                "building_extra_happies": [["BUILDINGCLASS_X"], ["BUILDINGCLASS_X", 5]],
                "plot_extra_yields": {"YIELD_COMMERCE": 2},
                "additional_events": {"EVENT_B": 30}
            }"#,
        )
        .unwrap();
        let e = read_event(&Rd::new(&value, &cx));
        assert!(cx.warnings.borrow().is_empty(), "{:?}", cx.warnings.borrow());
        let x = BuildingClassType(0);
        // Entries are positional; an empty entry is skipped.
        assert_eq!(
            e.building_yield_changes,
            vec![BuildingYieldChange { building_class: x, yield_type: YieldType::YIELD_COMMERCE, change: 3 }]
        );
        assert_eq!(e.building_yield_change(x, YieldType::YIELD_COMMERCE), 3);
        // A missing value reads as 0, and the lookup returns the first match (as C++ does).
        assert_eq!(e.building_happy_changes, vec![(x, 0), (x, 5)]);
        assert_eq!(e.building_happy_change(x), 0);
        assert_eq!(e.plot_extra_yield, [0, 0, 2]);
        assert_eq!(e.additional_event_chance.raw, vec![0, 30]);
        // C++ constructor sets -1, but read() resets an absent key to 0.
        assert_eq!(e.max_num_religions, 0);
    }
}
