//! Info classes: SpecialistInfo, ReligionInfo, CorporationInfo, CivicOptionInfo, CivicInfo, VoteInfo, VoteSourceInfo, EspionageMissionInfo, ProjectInfo, TechInfo.
//! Port of the corresponding `Cv*Info::read` functions in CvInfos.cpp (plus
//! `readPass2`/`readPass3`: the registry knows every type up front, so forward
//! references are resolved directly in the single read).
//!
//! Defaults follow C++: `GetChildXmlValByName` sets 0/false/"" when a tag is
//! absent (none of these classes pass an explicit default), a type tag that is
//! absent or `NONE` resolves to -1 (`None`), and every array is allocated by
//! `read()` (`InitList`/`SetYields`/`SetCommerce`/`SetVariableListTagPair`), so
//! the getters' NULL-array fallbacks never apply.
//!
//! Not ported (UI/art/audio only): `CvHotkeyInfo` fields of Specialist/Religion/
//! Corporation (hotkeys, order priority), specialist `Texture` and `bVisible`
//! (read only by the city screen), tech `Quote`/`Sound`/`SoundMP` and
//! `iGridX`/`iGridY` (tech-tree layout), religion `TechButton`/
//! `GenericTechButton`/`MovieFile`/`MovieSound`/`Sound`, corporation
//! `MovieFile`/`MovieSound`/`Sound`, project `MovieDefineTag`/`CreateSound`,
//! vote source `PopupText`/`SecretaryGeneralText` (popup and screen text only).
//! Members that `read()` does not set (`m_iMissionType`, `m_iChar`,
//! `m_iHolyCityChar`, `m_iHeadquarterChar`: action/font bookkeeping) are omitted too.

use crate::enums::*;
use crate::idx::IdxVec;
use crate::infos::InfoBase;
use crate::registry::Rd;

// ---------------------------------------------------------------------------
// private XML helpers
// ---------------------------------------------------------------------------

/// C++ `SetCommerce` on a `bool*` (e.g. `CommerceFlexible`): positional flags,
/// all false when the tag is absent.
fn commerce_flags(rd: &Rd, tag: &str) -> [bool; CommerceType::NUM] {
    let v = rd.positional_bools(tag, CommerceType::NUM);
    std::array::from_fn(|i| v[i])
}

/// C++ `CvCivicInfo::read`, `ImprovementYieldChanges` block: `Init2DIntList` (all
/// zero), then each child's `ImprovementType` selects a row that is (re)read from
/// its `ImprovementYields` with `SetYields` (zeros if that tag is missing).
/// Children whose type is absent/`NONE` are skipped, as `FindInInfoClass` gives -1.
fn improvement_yield_changes(rd: &Rd, tag: &str) -> IdxVec<ImprovementType, [i32; YieldType::NUM]> {
    let n = rd.reg().count::<ImprovementType>();
    let mut out = IdxVec::from_elem([0; YieldType::NUM], n);
    if let Some(list) = rd.child(tag) {
        for entry in list.children() {
            if let Some(imp) = entry.rf::<ImprovementType>("improvement_type") {
                out[imp] = entry.yields("improvement_yields");
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SpecialistInfo
// ---------------------------------------------------------------------------

/// C++ `CvSpecialistInfo` (specialists.jsonc)
#[derive(Debug, Clone, Default)]
pub struct SpecialistInfo {
    pub base: InfoBase,
    /// `getGreatPeopleUnitClass()` (XML `GreatPeopleUnitClass`)
    pub great_people_unit_class: Option<UnitClassType>,
    pub great_people_rate_change: i32,
    pub experience: i32,
    /// `getYieldChange(i)` (XML `Yields`)
    pub yield_change: [i32; YieldType::NUM],
    /// `getCommerceChange(i)` (XML `Commerces`)
    pub commerce_change: [i32; CommerceType::NUM],
    /// `getFlavorValue(i)` (XML `Flavors`)
    pub flavor_value: IdxVec<FlavorType, i32>,
}

/// C++ `CvSpecialistInfo::read`.
pub fn read_specialist(rd: &Rd) -> SpecialistInfo {
    SpecialistInfo {
        base: InfoBase::read(rd),
        great_people_unit_class: rd.rf("great_people_unit_class"),
        great_people_rate_change: rd.int("great_people_rate_change"),
        yield_change: rd.yields("yields"),
        commerce_change: rd.commerces("commerces"),
        experience: rd.int("experience"),
        flavor_value: rd.pairs_int("flavors", 0),
    }
}

// ---------------------------------------------------------------------------
// ReligionInfo
// ---------------------------------------------------------------------------

/// C++ `CvReligionInfo` (religions.jsonc)
#[derive(Debug, Clone, Default)]
pub struct ReligionInfo {
    pub base: InfoBase,
    pub tech_prereq: Option<TechType>,
    pub free_unit_class: Option<UnitClassType>,
    /// `getNumFreeUnits()` (XML `iFreeUnits`)
    pub num_free_units: i32,
    pub spread_factor: i32,
    /// `getGlobalReligionCommerce(i)` (XML `GlobalReligionCommerces`)
    pub global_religion_commerce: [i32; CommerceType::NUM],
    /// `getHolyCityCommerce(i)` (XML `HolyCityCommerces`)
    pub holy_city_commerce: [i32; CommerceType::NUM],
    /// `getStateReligionCommerce(i)` (XML `StateReligionCommerces`)
    pub state_religion_commerce: [i32; CommerceType::NUM],
    /// `getAdjectiveKey()`: text key (XML `Adjective`), used in game messages.
    pub adjective_key: String,
    // FfH
    pub hidden: bool,
    pub sneak_attack: bool,
    pub update_sight: bool,
    pub global_counter_found: i32,
    pub global_counter_spread: i32,
    pub alignment: Option<AlignmentType>,
    pub alignment_best: Option<AlignmentType>,
    pub alignment_worst: Option<AlignmentType>,
    // Better AI (Sephi), part of the FFH2 0.41 DLL
    /// `getReligionHero1()`
    pub religion_hero1: Option<UnitClassType>,
    /// `getReligionHero2()`
    pub religion_hero2: Option<UnitClassType>,
    /// `getReligionTech1()`
    pub religion_tech1: Option<TechType>,
    /// `getReligionTech2()`
    pub religion_tech2: Option<TechType>,
}

/// C++ `CvReligionInfo::read`.
pub fn read_religion(rd: &Rd) -> ReligionInfo {
    ReligionInfo {
        base: InfoBase::read(rd),
        tech_prereq: rd.rf("tech_prereq"),
        free_unit_class: rd.rf("free_unit_class"),
        num_free_units: rd.int("free_units"),
        spread_factor: rd.int("spread_factor"),
        global_religion_commerce: rd.commerces("global_religion_commerces"),
        holy_city_commerce: rd.commerces("holy_city_commerces"),
        state_religion_commerce: rd.commerces("state_religion_commerces"),
        adjective_key: rd.string("adjective"),
        hidden: rd.bool("hidden"),
        sneak_attack: rd.bool("sneak_attack"),
        update_sight: rd.bool("update_sight"),
        global_counter_found: rd.int("global_counter_found"),
        global_counter_spread: rd.int("global_counter_spread"),
        alignment: rd.rf("alignment"),
        alignment_best: rd.rf("alignment_best"),
        alignment_worst: rd.rf("alignment_worst"),
        religion_hero1: rd.rf("religion_hero1"),
        religion_hero2: rd.rf("religion_hero2"),
        religion_tech1: rd.rf("religion_tech1"),
        religion_tech2: rd.rf("religion_tech2"),
    }
}

// ---------------------------------------------------------------------------
// CorporationInfo (FFH2 ships an empty corporations.jsonc)
// ---------------------------------------------------------------------------

/// C++ `CvCorporationInfo` (corporations.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CorporationInfo {
    pub base: InfoBase,
    pub tech_prereq: Option<TechType>,
    pub free_unit_class: Option<UnitClassType>,
    pub spread_factor: i32,
    pub spread_cost: i32,
    pub maintenance: i32,
    pub bonus_produced: Option<BonusType>,
    /// `getPrereqBonus(i)` (XML `PrereqBonuses`). C++ keeps a fixed array of
    /// `NUM_CORPORATION_PREREQ_BONUSES` entries filled with -1; this holds only the
    /// listed bonuses, in XML order, without the -1 (`NONE`) slots.
    // C++ quirk: without a `PrereqBonuses` tag the C++ array stays NULL and
    // getPrereqBonus() would crash; here the list is simply empty.
    pub prereq_bonus: Vec<BonusType>,
    /// `getHeadquarterCommerce(i)` (XML `HeadquarterCommerces`)
    pub headquarter_commerce: [i32; CommerceType::NUM],
    /// `getCommerceProduced(i)` (XML `CommercesProduced`)
    pub commerce_produced: [i32; CommerceType::NUM],
    /// `getYieldProduced(i)` (XML `YieldsProduced`)
    pub yield_produced: [i32; YieldType::NUM],
}

/// C++ `CvCorporationInfo::read`.
pub fn read_corporation(rd: &Rd) -> CorporationInfo {
    CorporationInfo {
        base: InfoBase::read(rd),
        tech_prereq: rd.rf("tech_prereq"),
        free_unit_class: rd.rf("free_unit_class"),
        spread_factor: rd.int("spread_factor"),
        spread_cost: rd.int("spread_cost"),
        maintenance: rd.int("maintenance"),
        headquarter_commerce: rd.commerces("headquarter_commerces"),
        commerce_produced: rd.commerces("commerces_produced"),
        yield_produced: rd.yields("yields_produced"),
        prereq_bonus: rd.ref_list("prereq_bonuses"),
        bonus_produced: rd.rf("bonus_produced"),
    }
}

// ---------------------------------------------------------------------------
// CivicOptionInfo
// ---------------------------------------------------------------------------

/// C++ `CvCivicOptionInfo` (civic_options.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CivicOptionInfo {
    pub base: InfoBase,
    /// `getTraitNoUpkeep(i)` (XML `TraitNoUpkeeps`)
    pub trait_no_upkeep: IdxVec<TraitType, bool>,
}

/// C++ `CvCivicOptionInfo::read`.
pub fn read_civic_option(rd: &Rd) -> CivicOptionInfo {
    CivicOptionInfo { base: InfoBase::read(rd), trait_no_upkeep: rd.pairs_bool("trait_no_upkeeps") }
}

// ---------------------------------------------------------------------------
// CivicInfo
// ---------------------------------------------------------------------------

/// C++ `CvCivicInfo` (civics.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CivicInfo {
    pub base: InfoBase,
    pub civic_option_type: Option<CivicOptionType>,
    pub anarchy_length: i32,
    /// `getUpkeep()`: an `UpkeepTypes` value (XML `Upkeep`).
    pub upkeep: Option<UpkeepType>,
    pub ai_weight: i32,
    pub great_people_rate_modifier: i32,
    pub great_general_rate_modifier: i32,
    pub domestic_great_general_rate_modifier: i32,
    pub state_religion_great_people_rate_modifier: i32,
    pub distance_maintenance_modifier: i32,
    pub num_cities_maintenance_modifier: i32,
    pub corporation_maintenance_modifier: i32,
    pub extra_health: i32,
    pub free_experience: i32,
    pub worker_speed_modifier: i32,
    pub improvement_upgrade_rate_modifier: i32,
    pub military_production_modifier: i32,
    pub base_free_units: i32,
    pub base_free_military_units: i32,
    pub free_units_population_percent: i32,
    pub free_military_units_population_percent: i32,
    pub gold_per_unit: i32,
    pub gold_per_military_unit: i32,
    pub happy_per_military_unit: i32,
    pub largest_city_happiness: i32,
    pub war_weariness_modifier: i32,
    pub free_specialist: i32,
    pub trade_routes: i32,
    pub tech_prereq: Option<TechType>,
    pub civic_percent_anger: i32,
    pub max_conscript: i32,
    pub state_religion_happiness: i32,
    pub non_state_religion_happiness: i32,
    pub state_religion_unit_production_modifier: i32,
    pub state_religion_building_production_modifier: i32,
    pub state_religion_free_experience: i32,
    pub exp_in_border_modifier: i32,
    pub military_food_production: bool,
    pub no_unhealthy_population: bool,
    pub building_only_healthy: bool,
    pub no_foreign_trade: bool,
    pub no_corporations: bool,
    pub no_foreign_corporations: bool,
    pub state_religion: bool,
    pub no_non_state_religion_spread: bool,
    /// `getWeLoveTheKing()`: text key of the celebration message ("" if none).
    pub we_love_the_king: String,
    // FfH
    pub hidden: bool,
    pub no_diplomacy_with_enemies: bool,
    pub prereq_war: bool,
    pub attitude_share_mod: i32,
    pub coastal_trade_routes: i32,
    pub enslavement_chance: i32,
    pub food_consumption_per_population: i32,
    pub block_alignment: Option<AlignmentType>,
    pub prereq_alignment: Option<AlignmentType>,
    pub prereq_civilization: Option<CivilizationType>,
    pub prereq_religion: Option<ReligionType>,
    // arrays
    /// `getYieldModifier(i)` (XML `YieldModifiers`)
    pub yield_modifier: [i32; YieldType::NUM],
    /// `getCapitalYieldModifier(i)` (XML `CapitalYieldModifiers`)
    pub capital_yield_modifier: [i32; YieldType::NUM],
    /// `getTradeYieldModifier(i)` (XML `TradeYieldModifiers`)
    pub trade_yield_modifier: [i32; YieldType::NUM],
    /// `getCommerceModifier(i)` (XML `CommerceModifiers`)
    pub commerce_modifier: [i32; CommerceType::NUM],
    /// `getCapitalCommerceModifier(i)` (XML `CapitalCommerceModifiers`)
    pub capital_commerce_modifier: [i32; CommerceType::NUM],
    /// `getSpecialistExtraCommerce(i)` (XML `SpecialistExtraCommerces`)
    pub specialist_extra_commerce: [i32; CommerceType::NUM],
    /// `getBuildingHappinessChanges(i)`
    pub building_happiness_changes: IdxVec<BuildingClassType, i32>,
    /// `getBuildingHealthChanges(i)`
    pub building_health_changes: IdxVec<BuildingClassType, i32>,
    /// `getFeatureHappinessChanges(i)`
    pub feature_happiness_changes: IdxVec<FeatureType, i32>,
    /// `isHurry(i)` (XML `Hurrys`)
    pub hurry: IdxVec<HurryType, bool>,
    /// `isSpecialBuildingNotRequired(i)` (XML `SpecialBuildingNotRequireds`)
    pub special_building_not_required: IdxVec<SpecialBuildingType, bool>,
    /// `isSpecialistValid(i)` (XML `SpecialistValids`)
    pub specialist_valid: IdxVec<SpecialistType, bool>,
    /// `getImprovementYieldChanges(i, j)`: per improvement, per yield.
    pub improvement_yield_changes: IdxVec<ImprovementType, [i32; YieldType::NUM]>,
    // This game's (the cards, docs/CARDS.md)
    /// Magocracy: the percent every unit's production cost changes by (`unit_cost_modifier`),
    /// unless its unit combat is in `unit_combat_cost_modifiers`.
    pub unit_cost_modifier: i32,
    /// Magocracy: the percent the units of a unit combat cost more or less, in place of
    /// `unit_cost_modifier` (`unit_combat_cost_modifiers`, in file order).
    pub unit_combat_cost_modifiers: Vec<(UnitCombatType, i32)>,
    /// Stoicism: war weariness causes no unhappiness in its player's cities (`no_war_weariness`).
    pub no_war_weariness: bool,
    /// Gold Standard: its player buys the production of units at this many gold per hammer left,
    /// whatever its Labor civic (`unit_rush_gold`, as a trait's; 0: no).
    pub unit_rush_gold: i32,
    /// This game's governors (docs/GOVERNORS.md): under this civic every city of its player is
    /// ruled directly (else every city is governed).
    pub direct_rule: bool,
}

impl CivicInfo {
    /// The percent this civic changes the production cost of a unit of `combat` by.
    pub fn unit_cost_modifier_for(&self, combat: Option<UnitCombatType>) -> i32 {
        combat.and_then(|c| self.unit_combat_cost_modifiers.iter().find(|&&(k, _)| k == c)).map_or(self.unit_cost_modifier, |&(_, m)| m)
    }
}

/// C++ `CvCivicInfo::read` + `readPass3` (`PrereqCivilization`).
pub fn read_civic(rd: &Rd) -> CivicInfo {
    CivicInfo {
        base: InfoBase::read(rd),
        civic_option_type: rd.rf("civic_option_type"),
        tech_prereq: rd.rf("tech_prereq"),
        anarchy_length: rd.int("anarchy_length"),
        upkeep: rd.rf("upkeep"),
        ai_weight: rd.int("ai_weight"),
        great_people_rate_modifier: rd.int("great_people_rate_modifier"),
        great_general_rate_modifier: rd.int("great_general_rate_modifier"),
        domestic_great_general_rate_modifier: rd.int("domestic_great_general_rate_modifier"),
        state_religion_great_people_rate_modifier: rd.int("state_religion_great_people_rate_modifier"),
        distance_maintenance_modifier: rd.int("distance_maintenance_modifier"),
        num_cities_maintenance_modifier: rd.int("num_cities_maintenance_modifier"),
        corporation_maintenance_modifier: rd.int("corporation_maintenance_modifier"),
        extra_health: rd.int("extra_health"),
        free_experience: rd.int("free_experience"),
        worker_speed_modifier: rd.int("worker_speed_modifier"),
        improvement_upgrade_rate_modifier: rd.int("improvement_upgrade_rate_modifier"),
        military_production_modifier: rd.int("military_production_modifier"),
        base_free_units: rd.int("base_free_units"),
        base_free_military_units: rd.int("base_free_military_units"),
        free_units_population_percent: rd.int("free_units_population_percent"),
        free_military_units_population_percent: rd.int("free_military_units_population_percent"),
        gold_per_unit: rd.int("gold_per_unit"),
        gold_per_military_unit: rd.int("gold_per_military_unit"),
        happy_per_military_unit: rd.int("happy_per_military_unit"),
        military_food_production: rd.bool("military_food_production"),
        max_conscript: rd.int("max_conscript"),
        no_unhealthy_population: rd.bool("no_unhealthy_population"),
        building_only_healthy: rd.bool("building_only_healthy"),
        largest_city_happiness: rd.int("largest_city_happiness"),
        war_weariness_modifier: rd.int("war_weariness_modifier"),
        free_specialist: rd.int("free_specialist"),
        trade_routes: rd.int("trade_routes"),
        no_foreign_trade: rd.bool("no_foreign_trade"),
        no_corporations: rd.bool("no_corporations"),
        no_foreign_corporations: rd.bool("no_foreign_corporations"),
        civic_percent_anger: rd.int("civic_percent_anger"),
        state_religion: rd.bool("state_religion"),
        no_non_state_religion_spread: rd.bool("no_non_state_religion_spread"),
        state_religion_happiness: rd.int("state_religion_happiness"),
        non_state_religion_happiness: rd.int("non_state_religion_happiness"),
        state_religion_unit_production_modifier: rd.int("state_religion_unit_production_modifier"),
        state_religion_building_production_modifier: rd.int("state_religion_building_production_modifier"),
        state_religion_free_experience: rd.int("state_religion_free_experience"),
        exp_in_border_modifier: rd.int("exp_in_border_modifier"),
        yield_modifier: rd.yields("yield_modifiers"),
        capital_yield_modifier: rd.yields("capital_yield_modifiers"),
        trade_yield_modifier: rd.yields("trade_yield_modifiers"),
        commerce_modifier: rd.commerces("commerce_modifiers"),
        capital_commerce_modifier: rd.commerces("capital_commerce_modifiers"),
        specialist_extra_commerce: rd.commerces("specialist_extra_commerces"),
        hurry: rd.pairs_bool("hurrys"),
        special_building_not_required: rd.pairs_bool("special_building_not_requireds"),
        specialist_valid: rd.pairs_bool("specialist_valids"),
        building_happiness_changes: rd.pairs_int("building_happiness_changes", 0),
        building_health_changes: rd.pairs_int("building_health_changes", 0),
        feature_happiness_changes: rd.pairs_int("feature_happiness_changes", 0),
        improvement_yield_changes: improvement_yield_changes(rd, "improvement_yield_changes"),
        we_love_the_king: rd.string("we_love_the_king"),
        hidden: rd.bool("hidden"),
        no_diplomacy_with_enemies: rd.bool("no_diplomacy_with_enemies"),
        prereq_war: rd.bool("prereq_war"),
        attitude_share_mod: rd.int("attitude_share_mod"),
        coastal_trade_routes: rd.int("coastal_trade_routes"),
        enslavement_chance: rd.int("enslavement_chance"),
        food_consumption_per_population: rd.int("food_consumption_per_population"),
        block_alignment: rd.rf("block_alignment"),
        prereq_alignment: rd.rf("prereq_alignment"),
        // readPass3 in C++ (civilizations load after civics)
        prereq_civilization: rd.rf("prereq_civilization"),
        prereq_religion: rd.rf("prereq_religion"),
        unit_cost_modifier: rd.int("unit_cost_modifier"),
        unit_combat_cost_modifiers: rd.pair_list_int("unit_combat_cost_modifiers"),
        no_war_weariness: rd.bool("no_war_weariness"),
        unit_rush_gold: rd.int("unit_rush_gold"),
        direct_rule: rd.bool("direct_rule"),
    }
}

// ---------------------------------------------------------------------------
// VoteInfo
// ---------------------------------------------------------------------------

/// C++ `CvVoteInfo` (votes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct VoteInfo {
    pub base: InfoBase,
    pub population_threshold: i32,
    pub state_religion_vote_percent: i32,
    pub trade_routes: i32,
    pub min_voters: i32,
    pub secretary_general: bool,
    pub victory: bool,
    pub free_trade: bool,
    pub no_nukes: bool,
    pub city_voting: bool,
    pub civ_voting: bool,
    pub defensive_pact: bool,
    pub open_borders: bool,
    pub force_peace: bool,
    pub force_no_trade: bool,
    pub force_war: bool,
    pub assign_city: bool,
    /// `isForceCivic(i)` (XML `ForceCivics`)
    pub force_civic: IdxVec<CivicType, bool>,
    /// `isVoteSourceType(i)`: vote sources that can hold this vote (XML `DiploVotes`).
    pub vote_source_type: IdxVec<VoteSourceType, bool>,
    // FfH
    pub gambling_ring: bool,
    pub no_outside_tech_trades: bool,
    pub slave_trade: bool,
    pub smuggling_ring: bool,
    pub cost: i32,
    pub free_units: i32,
    pub free_unit_class: Option<UnitClassType>,
    /// `getNoBonus()`: bonus banned by this resolution.
    pub no_bonus: Option<BonusType>,
    /// `getPyResult()`: Python callback run when the vote passes ("" if none).
    pub py_result: String,
}

/// C++ `CvVoteInfo::read`.
pub fn read_vote(rd: &Rd) -> VoteInfo {
    VoteInfo {
        base: InfoBase::read(rd),
        population_threshold: rd.int("population_threshold"),
        state_religion_vote_percent: rd.int("state_religion_vote_percent"),
        trade_routes: rd.int("trade_routes"),
        min_voters: rd.int("min_voters"),
        secretary_general: rd.bool("secretary_general"),
        victory: rd.bool("victory"),
        free_trade: rd.bool("free_trade"),
        no_nukes: rd.bool("no_nukes"),
        city_voting: rd.bool("city_voting"),
        civ_voting: rd.bool("civ_voting"),
        defensive_pact: rd.bool("defensive_pact"),
        open_borders: rd.bool("open_borders"),
        force_peace: rd.bool("force_peace"),
        force_no_trade: rd.bool("force_no_trade"),
        force_war: rd.bool("force_war"),
        assign_city: rd.bool("assign_city"),
        force_civic: rd.pairs_bool("force_civics"),
        vote_source_type: rd.pairs_bool("diplo_votes"),
        gambling_ring: rd.bool("gambling_ring"),
        no_outside_tech_trades: rd.bool("no_outside_tech_trades"),
        slave_trade: rd.bool("slave_trade"),
        smuggling_ring: rd.bool("smuggling_ring"),
        cost: rd.int("cost"),
        free_units: rd.int("free_units"),
        free_unit_class: rd.rf("free_unit_class"),
        no_bonus: rd.rf("no_bonus"),
        py_result: rd.string("py_result"),
    }
}

// ---------------------------------------------------------------------------
// VoteSourceInfo
// ---------------------------------------------------------------------------

/// C++ `CvVoteSourceInfo` (vote_sources.jsonc)
#[derive(Debug, Clone, Default)]
pub struct VoteSourceInfo {
    pub base: InfoBase,
    pub vote_interval: i32,
    pub free_specialist: Option<SpecialistType>,
    pub civic: Option<CivicType>,
    /// `getReligionYield(i)` (XML `ReligionYields`)
    pub religion_yield: [i32; YieldType::NUM],
    /// `getReligionCommerce(i)` (XML `ReligionCommerces`)
    pub religion_commerce: [i32; CommerceType::NUM],
}

/// C++ `CvVoteSourceInfo::read` + `readPass3` (`Civic`).
pub fn read_vote_source(rd: &Rd) -> VoteSourceInfo {
    // C++ quirk: ReligionYields is read with SetCommerce, which allocates
    // NUM_COMMERCE_TYPES (4) slots; getReligionYield() only ever reads the first
    // NUM_YIELD_TYPES (3), so a 4th XML value is accepted silently and ignored.
    let ry = rd.positional_ints("religion_yields", CommerceType::NUM);
    VoteSourceInfo {
        base: InfoBase::read(rd),
        vote_interval: rd.int("vote_interval"),
        free_specialist: rd.rf("free_specialist"),
        // readPass3 in C++ (civics load after vote sources)
        civic: rd.rf("civic"),
        religion_yield: std::array::from_fn(|i| ry[i]),
        religion_commerce: rd.commerces("religion_commerces"),
    }
}

// ---------------------------------------------------------------------------
// EspionageMissionInfo
// ---------------------------------------------------------------------------

/// C++ `CvEspionageMissionInfo` (espionage_missions.jsonc)
///
/// The C++ constructor leaves every member uninitialized; `read()` assigns all of
/// them (0/false when absent, -1 for `TechPrereq`).
#[derive(Debug, Clone, Default)]
pub struct EspionageMissionInfo {
    pub base: InfoBase,
    pub cost: i32,
    /// `isPassive()` (XML `bIsPassive`)
    pub passive: bool,
    /// `isTwoPhases()` (XML `bIsTwoPhases`)
    pub two_phases: bool,
    pub targets_city: bool,
    pub select_plot: bool,
    pub tech_prereq: Option<TechType>,
    pub visibility_level: i32,
    pub investigate_city: bool,
    pub see_demographics: bool,
    pub no_active_missions: bool,
    pub see_research: bool,
    pub destroy_improvement: bool,
    pub destroy_building_cost_factor: i32,
    pub destroy_unit_cost_factor: i32,
    pub destroy_project_cost_factor: i32,
    pub destroy_production_cost_factor: i32,
    pub buy_unit_cost_factor: i32,
    pub buy_city_cost_factor: i32,
    pub steal_treasury_types: i32,
    pub city_insert_culture_amount_factor: i32,
    pub city_insert_culture_cost_factor: i32,
    pub city_poison_water_counter: i32,
    pub city_unhappiness_counter: i32,
    pub city_revolt_counter: i32,
    pub buy_tech_cost_factor: i32,
    pub switch_civic_cost_factor: i32,
    pub switch_religion_cost_factor: i32,
    pub player_anarchy_counter: i32,
    pub counterespionage_num_turns: i32,
    pub counterespionage_mod: i32,
    pub difficulty_mod: i32,
}

/// C++ `CvEspionageMissionInfo::read`.
pub fn read_espionage_mission(rd: &Rd) -> EspionageMissionInfo {
    EspionageMissionInfo {
        base: InfoBase::read(rd),
        cost: rd.int("cost"),
        passive: rd.bool("is_passive"),
        two_phases: rd.bool("is_two_phases"),
        targets_city: rd.bool("targets_city"),
        select_plot: rd.bool("select_plot"),
        tech_prereq: rd.rf("tech_prereq"),
        visibility_level: rd.int("visibility_level"),
        investigate_city: rd.bool("investigate_city"),
        see_demographics: rd.bool("see_demographics"),
        no_active_missions: rd.bool("no_active_missions"),
        see_research: rd.bool("see_research"),
        destroy_improvement: rd.bool("destroy_improvement"),
        destroy_building_cost_factor: rd.int("destroy_building_cost_factor"),
        destroy_unit_cost_factor: rd.int("destroy_unit_cost_factor"),
        destroy_project_cost_factor: rd.int("destroy_project_cost_factor"),
        destroy_production_cost_factor: rd.int("destroy_production_cost_factor"),
        buy_unit_cost_factor: rd.int("buy_unit_cost_factor"),
        buy_city_cost_factor: rd.int("buy_city_cost_factor"),
        steal_treasury_types: rd.int("steal_treasury_types"),
        city_insert_culture_amount_factor: rd.int("city_insert_culture_amount_factor"),
        city_insert_culture_cost_factor: rd.int("city_insert_culture_cost_factor"),
        city_poison_water_counter: rd.int("city_poison_water_counter"),
        city_unhappiness_counter: rd.int("city_unhappiness_counter"),
        city_revolt_counter: rd.int("city_revolt_counter"),
        buy_tech_cost_factor: rd.int("buy_tech_cost_factor"),
        switch_civic_cost_factor: rd.int("switch_civic_cost_factor"),
        switch_religion_cost_factor: rd.int("switch_religion_cost_factor"),
        player_anarchy_counter: rd.int("player_anarchy_counter"),
        counterespionage_num_turns: rd.int("counterespionage_num_turns"),
        counterespionage_mod: rd.int("counterespionage_mod"),
        difficulty_mod: rd.int("difficulty_mod"),
    }
}

// ---------------------------------------------------------------------------
// ProjectInfo
// ---------------------------------------------------------------------------

/// C++ `CvProjectInfo` (projects.jsonc)
#[derive(Debug, Clone, Default)]
pub struct ProjectInfo {
    pub base: InfoBase,
    pub victory_prereq: Option<VictoryType>,
    pub tech_prereq: Option<TechType>,
    /// `getAnyoneProjectPrereq()` (XML `AnyonePrereqProject`)
    pub anyone_project_prereq: Option<ProjectType>,
    /// -1 = unlimited. C++ passes no default, so an absent tag gives 0; the XML
    /// always writes -1 explicitly.
    pub max_global_instances: i32,
    /// Same convention as `max_global_instances`.
    pub max_team_instances: i32,
    /// `getProductionCost()` (XML `iCost`)
    pub production_cost: i32,
    pub nuke_interception: i32,
    pub tech_share: i32,
    pub everyone_special_unit: Option<SpecialUnitType>,
    pub everyone_special_building: Option<SpecialBuildingType>,
    pub victory_delay_percent: i32,
    pub success_rate: i32,
    pub spaceship: bool,
    pub allows_nukes: bool,
    // FfH
    pub ai_weight: i32,
    pub modify_global_counter: i32,
    pub prereq_civilization: Option<CivilizationType>,
    pub prereq_global_counter: i32,
    // arrays
    /// `getBonusProductionModifier(i)` (XML `BonusProductionModifiers`)
    pub bonus_production_modifier: IdxVec<BonusType, i32>,
    /// `getVictoryThreshold(i)` (XML `VictoryThresholds`)
    pub victory_threshold: IdxVec<VictoryType, i32>,
    /// `getVictoryMinThreshold(i)`: the value that getter returns, i.e. the XML
    /// `VictoryMinThresholds` entry, or `victory_threshold[i]` where that entry is 0.
    pub victory_min_threshold: IdxVec<VictoryType, i32>,
    /// `getProjectsNeeded(i)`: count of each project required (XML `PrereqProjects`).
    pub projects_needed: IdxVec<ProjectType, i32>,
    /// This game's (`repeat_cost_percent`, docs/CARDS.md): a repeatable ritual whose every
    /// completion by a team makes its next cost this percent of the last (the Pillar of Dawn:
    /// 200); 0 for none.
    pub repeat_cost_percent: i32,
}

/// C++ `CvProjectInfo::read` + `readPass2` (`PrereqProjects`, `AnyonePrereqProject`)
/// + `readPass3` (`PrereqCivilization`).
pub fn read_project(rd: &Rd) -> ProjectInfo {
    let victory_threshold = rd.pairs_int::<VictoryType>("victory_thresholds", 0);
    let min_raw = rd.pairs_int::<VictoryType>("victory_min_thresholds", 0);
    // getVictoryMinThreshold() falls back to getVictoryThreshold() for 0 entries.
    let victory_min_threshold =
        IdxVec::from_fn(min_raw.len(), |v| if min_raw[v] != 0 { min_raw[v] } else { victory_threshold[v] });
    ProjectInfo {
        base: InfoBase::read(rd),
        victory_prereq: rd.rf("victory_prereq"),
        tech_prereq: rd.rf("tech_prereq"),
        max_global_instances: rd.int("max_global_instances"),
        max_team_instances: rd.int("max_team_instances"),
        production_cost: rd.int("cost"),
        nuke_interception: rd.int("nuke_interception"),
        tech_share: rd.int("tech_share"),
        everyone_special_unit: rd.rf("everyone_special_unit"),
        everyone_special_building: rd.rf("everyone_special_building"),
        spaceship: rd.bool("spaceship"),
        allows_nukes: rd.bool("allows_nukes"),
        bonus_production_modifier: rd.pairs_int("bonus_production_modifiers", 0),
        victory_threshold,
        victory_min_threshold,
        victory_delay_percent: rd.int("victory_delay_percent"),
        success_rate: rd.int("success_rate"),
        ai_weight: rd.int("ai_weight"),
        modify_global_counter: rd.int("modify_global_counter"),
        prereq_civilization: rd.rf("prereq_civilization"),
        prereq_global_counter: rd.int("prereq_global_counter"),
        // readPass2 in C++ (references other projects)
        projects_needed: rd.pairs_int("prereq_projects", 0),
        anyone_project_prereq: rd.rf("anyone_prereq_project"),
        repeat_cost_percent: rd.int("repeat_cost_percent"),
    }
}

// ---------------------------------------------------------------------------
// TechInfo
// ---------------------------------------------------------------------------

/// C++ `CvTechInfo` (techs.jsonc)
#[derive(Debug, Clone, Default)]
pub struct TechInfo {
    pub base: InfoBase,
    /// `getAdvisorType()` as the raw `<Advisor>` type name (e.g. `ADVISOR_SCIENCE`).
    /// AdvisorInfos (BTS Interface/CIV4AdvisorInfos.xml) are not vendored or
    /// registered, so it is not resolved to an index; C++ reads it only in
    /// `CvPlayerAI::AI_bestTech`'s `eIgnoreAdvisor` filter, which only the
    /// tech-choice popup sets.
    pub advisor_type: String,
    pub ai_weight: i32,
    pub ai_trade_modifier: i32,
    /// `getResearchCost()` (XML `iCost`)
    pub research_cost: i32,
    pub advanced_start_cost: i32,
    pub advanced_start_cost_increase: i32,
    pub trade_routes: i32,
    pub feature_production_modifier: i32,
    pub worker_speed_modifier: i32,
    pub first_free_unit_class: Option<UnitClassType>,
    pub health: i32,
    pub happiness: i32,
    pub first_free_techs: i32,
    /// `getAssetValue()` (XML `iAsset`)
    pub asset_value: i32,
    /// `getPowerValue()` (XML `iPower`)
    pub power_value: i32,
    pub repeat: bool,
    pub trade: bool,
    pub disable: bool,
    pub goody_tech: bool,
    pub extra_water_see_from: bool,
    pub map_centering: bool,
    pub map_visible: bool,
    pub map_trading: bool,
    pub tech_trading: bool,
    pub gold_trading: bool,
    pub open_borders_trading: bool,
    pub defensive_pact_trading: bool,
    pub permanent_alliance_trading: bool,
    /// `isVassalStateTrading()` (XML `bVassalTrading`)
    pub vassal_state_trading: bool,
    pub bridge_building: bool,
    pub irrigation: bool,
    pub ignore_irrigation: bool,
    pub water_work: bool,
    pub river_trade: bool,
    // FfH
    pub water: bool,
    pub preferred_alignment: Option<AlignmentType>,
    pub prereq_religion: Option<ReligionType>,
    // arrays
    /// `getDomainExtraMoves(i)`
    pub domain_extra_moves: IdxVec<DomainType, i32>,
    /// `getFlavorValue(i)` (XML `Flavors`)
    pub flavor_value: IdxVec<FlavorType, i32>,
    /// `getPrereqOrTechs(i)` (XML `OrPreReqs`): any one of these is required.
    /// C++ keeps a fixed array of `NUM_OR_TECH_PREREQS` entries filled with -1
    /// (NULL when the tag is absent); this holds only the listed techs, in XML
    /// order, without the -1 (`NONE`) slots.
    pub prereq_or_techs: Vec<TechType>,
    /// `getPrereqAndTechs(i)` (XML `AndPreReqs`): all of these are required.
    /// Same representation as `prereq_or_techs` (C++ array size `NUM_AND_TECH_PREREQS`).
    pub prereq_and_techs: Vec<TechType>,
    /// `isCommerceFlexible(i)` (XML `CommerceFlexible`)
    pub commerce_flexible: [bool; CommerceType::NUM],
    /// `isTerrainTrade(i)` (XML `TerrainTrades`)
    pub terrain_trade: IdxVec<TerrainType, bool>,
    /// This game's (docs/CARDS.md, Mountaineering): the team's land units can enter peaks (3
    /// moves); on a peak they get +50% defence and sight 3.
    pub peak_movement: bool,
    /// This game's (docs/CARDS.md, Ferries): the team's land units can enter a one-plot strait
    /// (a water plot with land on two opposite sides) as if it were land.
    pub strait_crossing: bool,
    /// This game's (docs/CARDS.md, Forest Paths): the features (forest and jungle) the team's
    /// units move through in its own borders as through open ground; units at war with it that
    /// start their turn in them there have one move fewer.
    pub forest_paths: Vec<FeatureType>,
    /// This game's: the tech is a great person's card (docs/DECKS.md, "The deck as built"),
    /// `None` for an ordinary tech. A card tech is never researched or traded (`disable` and
    /// `!trade` are forced): a great person holding it discovers it.
    pub card: Option<CardInfo>,
}

/// A great-person card (docs/DECKS.md): the deck it is drawn from, and who may draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct CardInfo {
    /// The great person's class that draws it (`card_suit`: Sage `UNITCLASS_SCIENTIST`, Prophet
    /// `UNITCLASS_PROPHET`, Engineer `UNITCLASS_ENGINEER`, Merchant `UNITCLASS_MERCHANT`, Bard
    /// `UNITCLASS_ARTIST`).
    pub suit: UnitClassType,
    /// Drawn only by civilizations of this alignment (`card_alignment`).
    pub alignment: Option<AlignmentType>,
    /// Never drawn by civilizations of this alignment (`card_block_alignment`: a [good or
    /// neutral] card blocks evil).
    pub block_alignment: Option<AlignmentType>,
    /// The card's faith (`card_religion`): drawn whatever the drawer's religion; discovering it
    /// also spreads this religion to three of the discoverer's cities.
    pub religion: Option<ReligionType>,
    /// The card's band (`band`, 1 to 3): the deck deals the lowest band that still holds cards
    /// of the suit first (docs/CARDS.md).
    pub band: i32,
}

/// C++ `CvTechInfo::read` + `readPass2` (`OrPreReqs`, `AndPreReqs`) + `readPass3`
/// (`PrereqReligion`).
pub fn read_tech(rd: &Rd) -> TechInfo {
    let mut tech = TechInfo {
        base: InfoBase::read(rd),
        advisor_type: rd.string("advisor"),
        ai_weight: rd.int("ai_weight"),
        ai_trade_modifier: rd.int("ai_trade_modifier"),
        research_cost: rd.int("cost"),
        advanced_start_cost: rd.int("advanced_start_cost"),
        advanced_start_cost_increase: rd.int("advanced_start_cost_increase"),
        first_free_unit_class: rd.rf("first_free_unit_class"),
        feature_production_modifier: rd.int("feature_production_modifier"),
        worker_speed_modifier: rd.int("worker_speed_modifier"),
        trade_routes: rd.int("trade_routes"),
        health: rd.int("health"),
        happiness: rd.int("happiness"),
        first_free_techs: rd.int("first_free_techs"),
        asset_value: rd.int("asset"),
        power_value: rd.int("power"),
        repeat: rd.bool("repeat"),
        trade: rd.bool("trade"),
        disable: rd.bool("disable"),
        goody_tech: rd.bool("goody_tech"),
        extra_water_see_from: rd.bool("extra_water_see_from"),
        map_centering: rd.bool("map_centering"),
        map_visible: rd.bool("map_visible"),
        map_trading: rd.bool("map_trading"),
        tech_trading: rd.bool("tech_trading"),
        gold_trading: rd.bool("gold_trading"),
        open_borders_trading: rd.bool("open_borders_trading"),
        defensive_pact_trading: rd.bool("defensive_pact_trading"),
        permanent_alliance_trading: rd.bool("permanent_alliance_trading"),
        vassal_state_trading: rd.bool("vassal_trading"),
        bridge_building: rd.bool("bridge_building"),
        irrigation: rd.bool("irrigation"),
        ignore_irrigation: rd.bool("ignore_irrigation"),
        water_work: rd.bool("water_work"),
        river_trade: rd.bool("river_trade"),
        commerce_flexible: commerce_flags(rd, "commerce_flexible"),
        domain_extra_moves: rd.pairs_int("domain_extra_moves", 0),
        terrain_trade: rd.pairs_bool("terrain_trades"),
        flavor_value: rd.pairs_int("flavors", 0),
        water: rd.bool("water"),
        preferred_alignment: rd.rf("preferred_alignment"),
        // readPass3 in C++ (religions load after techs)
        prereq_religion: rd.rf("prereq_religion"),
        // readPass2 in C++ (references other techs)
        prereq_or_techs: rd.ref_list("or_pre_reqs"),
        prereq_and_techs: rd.ref_list("and_pre_reqs"),
        peak_movement: rd.bool("peak_movement"),
        strait_crossing: rd.bool("strait_crossing"),
        forest_paths: rd.ref_list("forest_paths"),
        card: None,
    };
    if let Some(suit) = rd.rf::<UnitClassType>("card_suit") {
        let band = rd.int("band");
        if !(1..=3).contains(&band) {
            rd.cx.warn(format!("{}: card band {} is not 1, 2 or 3", tech.base.type_, band));
        }
        tech.card = Some(CardInfo { suit, alignment: rd.rf("card_alignment"), block_alignment: rd.rf("card_block_alignment"), religion: rd.rf("card_religion"), band: band.clamp(1, 3) });
        tech.disable = true;
        tech.trade = false;
    }
    tech
}

#[cfg(test)]
mod tests {
    //! Synthetic-JSON checks of read paths the shipped data does not exercise.

    use super::*;
    use crate::idx::Idx;
    use crate::registry::{Ctx, Registry};

    fn registry() -> Registry {
        let mut reg = Registry::new();
        let mut add = |class: &'static str, names: &[&str]| {
            reg.register_class(class, names.iter().map(|s| s.to_string()).collect());
        };
        add("TechInfo", &["TECH_A", "TECH_B", "TECH_C"]);
        add("VictoryInfo", &["VICTORY_X", "VICTORY_Y", "VICTORY_Z"]);
        add("ImprovementInfo", &["IMPROVEMENT_FARM", "IMPROVEMENT_MINE", "IMPROVEMENT_QUARRY"]);
        add("FlavorTypes", &["FLAVOR_MILITARY", "FLAVOR_SCIENCE"]);
        reg
    }

    /// Runs `read` on one entry object and returns the info plus load warnings.
    fn read_with<T>(json: &str, read: fn(&Rd) -> T) -> (T, Vec<String>) {
        let reg = registry();
        let value = crate::jsonc::parse(json).expect("test json parses");
        let cx = Ctx::new(&reg, "test.jsonc");
        let info = read(&Rd::new(&value, &cx));
        (info, cx.warnings.into_inner())
    }

    #[test]
    fn absent_tags_take_cpp_defaults() {
        let (t, w) = read_with(r#"{"type": "TECH_C"}"#, read_tech);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(t.base.type_, "TECH_C");
        assert_eq!((t.research_cost, t.ai_weight, t.prereq_religion), (0, 0, None));
        assert!(t.prereq_or_techs.is_empty() && t.prereq_and_techs.is_empty());
        assert_eq!(t.commerce_flexible, [false; CommerceType::NUM]);
        assert_eq!(t.flavor_value.raw, vec![0, 0]);
        assert_eq!(t.advisor_type, "");

        let (p, w) = read_with(r#"{"type": "PROJECT_P"}"#, read_project);
        assert!(w.is_empty(), "{w:?}");
        // No explicit GetChildXmlValByName default: absent means 0, not -1 (unlimited).
        assert_eq!((p.max_global_instances, p.max_team_instances), (0, 0));
        assert_eq!(p.anyone_project_prereq, None);
    }

    #[test]
    fn tech_prereq_lists_keep_order_and_skip_none() {
        let json = r#"{
            "type": "TECH_C",
            "or_pre_reqs": ["TECH_B", "NONE", "TECH_A"],
            "and_pre_reqs": ["TECH_A"],
            "commerce_flexible": [false, true],
            "flavors": {"FLAVOR_SCIENCE": 7}
        }"#;
        let (t, w) = read_with(json, read_tech);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(t.prereq_or_techs, vec![TechType(1), TechType(0)]);
        assert_eq!(t.prereq_and_techs, vec![TechType(0)]);
        assert_eq!(t.commerce_flexible, [false, true, false, false]);
        assert_eq!(t.flavor_value.raw, vec![0, 7]);
    }

    #[test]
    fn victory_min_threshold_falls_back_to_threshold() {
        let json = r#"{
            "type": "PROJECT_P",
            "victory_thresholds": {"VICTORY_X": 10, "VICTORY_Y": 20},
            "victory_min_thresholds": {"VICTORY_Y": 5}
        }"#;
        let (p, w) = read_with(json, read_project);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(p.victory_threshold.raw, vec![10, 20, 0]);
        // getVictoryMinThreshold(i): the min entry if non-zero, else getVictoryThreshold(i)
        assert_eq!(p.victory_min_threshold.raw, vec![10, 5, 0]);
    }

    #[test]
    fn vote_source_religion_yields_use_commerce_reader() {
        // Four values: SetCommerce accepts them silently; only three yields are kept.
        let json = r#"{
            "type": "DIPLOVOTE_T",
            "religion_yields": [1, 2, 3, 4],
            "religion_commerces": [5]
        }"#;
        let (v, w) = read_with(json, read_vote_source);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(v.religion_yield, [1, 2, 3]);
        assert_eq!(v.religion_commerce, [5, 0, 0, 0]);
        assert_eq!((v.civic, v.free_specialist), (None, None));
    }

    #[test]
    fn civic_improvement_yield_changes() {
        let json = r#"{
            "type": "CIVIC_T",
            "improvement_yield_changes": [
                {"improvement_type": "IMPROVEMENT_MINE", "improvement_yields": [0, 2]},
                {"improvement_type": "NONE", "improvement_yields": [9]},
                {"improvement_type": "IMPROVEMENT_QUARRY", "improvement_yields": [4]},
                {"improvement_type": "IMPROVEMENT_QUARRY"}
            ]
        }"#;
        let (c, w) = read_with(json, read_civic);
        assert!(w.is_empty(), "{w:?}");
        let row = |i: usize| c.improvement_yield_changes[ImprovementType::from_usize(i)];
        assert_eq!(c.improvement_yield_changes.len(), 3);
        assert_eq!(row(0), [0, 0, 0]);
        assert_eq!(row(1), [0, 2, 0]);
        // A repeated row without "improvement_yields" is re-initialized to zeros (C++ InitList).
        assert_eq!(row(2), [0, 0, 0]);
    }
}
