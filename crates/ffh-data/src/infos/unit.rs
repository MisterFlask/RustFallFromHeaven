//! Info classes: SpecialUnitInfo, UnitClassInfo, PromotionInfo, BuildInfo, UnitInfo, SpellInfo, AutomateInfo, CommandInfo, MissionInfo.
//! Port of the corresponding `Cv*Info::read` functions in CvInfos.cpp.
//!
//! - `readPass2`/`readPass3` references (promotion prereqs, `DefaultUnit`, `PrereqCiv`,
//!   `UpgradeCiv`, `UnitConvertFromCombat`, ...) are resolved directly in `read_*`: the registry
//!   knows every type before any table is read.
//! - `CvHotkeyInfo::read` (base of the Promotion/Spell/Build/Unit/Mission/Command/Automate infos)
//!   only reads hotkeys, modifier keys and `iOrderPriority` (action-button sort order). All of it
//!   is UI, so it is not ported; these infos read [`InfoBase`] only.
//! - Not ported (graphics/audio/UI only): sounds, art define tags, unit mesh groups (group size,
//!   wave sizes, max speed, pad time), formation types, animation paths, entity events, effects,
//!   waypoints, unit art styles, advisor types, popup images.
//! - Members C++ sets outside `read()` while building the UI action list
//!   (`SetGlobalActionInfo`: `m_iCommandType`, `m_iMissionType`, `m_iActionInfoIndex`) are
//!   not ported either; they are constants (`COMMAND_PROMOTION`, `COMMAND_CAST`,
//!   `COMMAND_UPGRADE`, `MISSION_BUILD`) only used by the interface and entity animations.

use crate::enums::*;
use crate::idx::IdxVec;
use crate::infos::{read_engage_rows, read_stat_changes, read_stats, CombatRoleType, InfoBase, RowActions, StatChange, StatSheet, StatType, ROWS};
use crate::registry::Rd;

// ---------------------------------------------------------------------------------------------
// SpecialUnitInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvSpecialUnitInfo` (special_units.jsonc)
#[derive(Debug, Clone, Default)]
pub struct SpecialUnitInfo {
    pub base: InfoBase,
    pub valid: bool,
    pub city_load: bool,
    /// C++ `isCarrierUnitAIType(i)`. Always all `false`: see the quirk note in [`read_special_unit`].
    pub carrier_unit_ai_type: IdxVec<UnitAIType, bool>,
    pub production_traits: IdxVec<TraitType, i32>,
}

/// C++ `CvSpecialUnitInfo::read`.
pub fn read_special_unit(rd: &Rd) -> SpecialUnitInfo {
    SpecialUnitInfo {
        base: InfoBase::read(rd),
        valid: rd.bool("valid"),
        city_load: rd.bool("city_load"),
        // C++ quirk: C++ reads <CarrierUnitAITypes>, but the XML and its schema use
        // <CarrierUnitAIs>, so the list is never loaded and `isCarrierUnitAIType` is always
        // false (stock BTS bug, inherited by FFH2; consulted by CvPlayerAI::AI_unitValue).
        carrier_unit_ai_type: rd.pairs_bool::<UnitAIType>("carrier_unit_ai_types"),
        production_traits: rd.pairs_int::<TraitType>("production_traits", 0),
    }
}

// ---------------------------------------------------------------------------------------------
// UnitClassInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvUnitClassInfo` (unit_classes.jsonc)
#[derive(Debug, Clone, Default)]
pub struct UnitClassInfo {
    pub base: InfoBase,
    pub max_global_instances: i32,
    pub max_team_instances: i32,
    pub max_player_instances: i32,
    pub instance_cost_modifier: i32,
    /// C++ `getDefaultUnitIndex()` (`<DefaultUnit>`, resolved by C++ `readPass3`).
    pub default_unit_index: Option<UnitType>,
}

/// C++ `CvUnitClassInfo::read` + `readPass3`.
pub fn read_unit_class(rd: &Rd) -> UnitClassInfo {
    UnitClassInfo {
        base: InfoBase::read(rd),
        // The C++ default for the three instance limits is 0 (not -1) when the tag is absent;
        // every FFH2 entry sets them explicitly.
        max_global_instances: rd.int("max_global_instances"),
        max_team_instances: rd.int("max_team_instances"),
        max_player_instances: rd.int("max_player_instances"),
        instance_cost_modifier: rd.int("instance_cost_modifier"),
        default_unit_index: rd.rf::<UnitType>("default_unit"),
    }
}

// ---------------------------------------------------------------------------------------------
// PromotionInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvPromotionInfo` (promotions.jsonc)
///
/// Not ported: `Sound`, `LayerAnimationPath`, `UnitArtStyleType` (unit art style, C++ `readPass3`)
/// and FfH `iGroupSize` (number of figures drawn; only feeds `CvUnit::getSubUnitsAlive`, which is
/// battle-animation code).
#[derive(Debug, Clone, Default)]
pub struct PromotionInfo {
    pub base: InfoBase,
    /// C++ `getPrereqPromotion()` (`<PromotionPrereq>`, C++ `readPass2`).
    pub prereq_promotion: Option<PromotionType>,
    /// C++ `getPrereqOrPromotion1()` (`<PromotionPrereqOr1>`, C++ `readPass2`).
    pub prereq_or_promotion1: Option<PromotionType>,
    /// C++ `getPrereqOrPromotion2()` (`<PromotionPrereqOr2>`, C++ `readPass2`).
    pub prereq_or_promotion2: Option<PromotionType>,
    pub tech_prereq: Option<TechType>,
    pub state_religion_prereq: Option<ReligionType>,
    pub visibility_change: i32,
    pub moves_change: i32,
    pub move_discount_change: i32,
    pub air_range_change: i32,
    pub intercept_change: i32,
    pub evasion_change: i32,
    pub withdrawal_change: i32,
    pub cargo_change: i32,
    pub collateral_damage_change: i32,
    pub bombard_rate_change: i32,
    pub first_strikes_change: i32,
    pub chance_first_strikes_change: i32,
    pub enemy_heal_change: i32,
    pub neutral_heal_change: i32,
    pub friendly_heal_change: i32,
    pub same_tile_heal_change: i32,
    pub adjacent_tile_heal_change: i32,
    /// C++ `getCityAttackPercent()` (`<iCityAttack>`).
    pub city_attack_percent: i32,
    /// C++ `getCityDefensePercent()` (`<iCityDefense>`).
    pub city_defense_percent: i32,
    /// C++ `getHillsAttackPercent()` (`<iHillsAttack>`).
    pub hills_attack_percent: i32,
    /// C++ `getHillsDefensePercent()` (`<iHillsDefense>`).
    pub hills_defense_percent: i32,
    pub revolt_protection: i32,
    pub collateral_damage_protection: i32,
    pub pillage_change: i32,
    pub upgrade_discount: i32,
    pub experience_percent: i32,
    pub kamikaze_percent: i32,
    /// C++ `isLeader()`. A leader promotion also forces `base.graphical_only` (as in C++).
    pub leader: bool,
    pub blitz: bool,
    pub amphib: bool,
    pub river: bool,
    pub enemy_route: bool,
    pub always_heal: bool,
    pub hills_double_move: bool,
    pub immune_to_first_strikes: bool,
    /// C++ `getTerrainAttackPercent(i)` (`<TerrainAttacks>`).
    pub terrain_attack_percent: IdxVec<TerrainType, i32>,
    /// C++ `getTerrainDefensePercent(i)` (`<TerrainDefenses>`).
    pub terrain_defense_percent: IdxVec<TerrainType, i32>,
    /// C++ `getFeatureAttackPercent(i)` (`<FeatureAttacks>`).
    pub feature_attack_percent: IdxVec<FeatureType, i32>,
    /// C++ `getFeatureDefensePercent(i)` (`<FeatureDefenses>`).
    pub feature_defense_percent: IdxVec<FeatureType, i32>,
    /// C++ `getUnitCombatModifierPercent(i)` (`<UnitCombatMods>`).
    pub unit_combat_modifier_percent: IdxVec<UnitCombatType, i32>,
    /// C++ `getDomainModifierPercent(i)` (`<DomainMods>`).
    pub domain_modifier_percent: IdxVec<DomainType, i32>,
    /// C++ `getTerrainDoubleMove(i)` (`<TerrainDoubleMoves>`).
    pub terrain_double_move: IdxVec<TerrainType, bool>,
    /// C++ `getFeatureDoubleMove(i)` (`<FeatureDoubleMoves>`).
    pub feature_double_move: IdxVec<FeatureType, bool>,
    /// C++ `getUnitCombat(i)` (`<UnitCombats>`): unit combat types that may take the promotion.
    pub unit_combat: IdxVec<UnitCombatType, bool>,
    // ---- FfH ----
    /// C++ `isAIControl()`.
    pub ai_control: bool,
    pub boarding: bool,
    pub only_defensive: bool,
    pub dispellable: bool,
    pub double_fortify_bonus: bool,
    pub equipment: bool,
    pub fear: bool,
    pub flying: bool,
    pub held: bool,
    pub hidden_nationality: bool,
    pub ignore_building_defense: bool,
    pub immortal: bool,
    pub immune_to_capture: bool,
    pub immune_to_defensive_strike: bool,
    pub immune_to_fear: bool,
    pub immune_to_magic: bool,
    pub invisible: bool,
    pub mutation: bool,
    pub not_alive: bool,
    pub prereq_alive: bool,
    pub race: bool,
    pub removed_by_casting: bool,
    pub removed_by_combat: bool,
    pub removed_when_healed: bool,
    pub see_invisible: bool,
    pub target_weakest_unit: bool,
    pub target_weakest_unit_counter: bool,
    pub twincast: bool,
    pub validate: bool,
    pub water_walking: bool,
    pub ai_weight: i32,
    pub betrayal_chance: i32,
    pub better_defender_than_percent: i32,
    pub caster_resist_modify: i32,
    pub combat_capture_percent: i32,
    pub combat_heal_percent: i32,
    pub combat_limit: i32,
    /// What it changes of a unit's battle stats (`stat_changes`; docs/UNIT_STATS.md). Its attack and
    /// defence also change the unit's strength outside land battles: their percentages that
    /// strength's (C++ `iCombatPercent`, `iCombatPercentDefense`), their points, in tens, its flat
    /// strength (`iExtraCombatStr`, `iExtraCombatDefense`). Its magic is a battle's only.
    pub stat_changes: [StatChange; StatType::NUM],
    /// C++ `getSpellDamageModify()`: damage added to its unit's spells outside battles (FfH's
    /// `doDamage`); a battle's spells use magic (`stat_changes`).
    pub spell_damage_modify: i32,
    pub combat_percent_global_counter: i32,
    pub combat_percent_in_borders: i32,
    pub defensive_strike_chance: i32,
    pub defensive_strike_damage: i32,
    pub expire_chance: i32,
    /// C++ `getFreeXPPerTurn()`.
    pub free_xp_per_turn: i32,
    /// C++ `getFreeXPFromCombat()`.
    pub free_xp_from_combat: i32,
    pub gold_from_combat: i32,
    pub min_level: i32,
    pub modify_global_counter: i32,
    pub modify_global_counter_on_combat: i32,
    pub resist_magic: i32,
    /// C++ `getSpellCasterXP()`.
    pub spell_caster_xp: i32,
    pub work_rate_modify: i32,
    pub capture_unit_combat: Option<UnitCombatType>,
    /// C++ `getPromotionCombatApply()` (C++ `readPass2`).
    pub promotion_combat_apply: Option<PromotionType>,
    pub promotion_immune1: Option<PromotionType>,
    pub promotion_immune2: Option<PromotionType>,
    pub promotion_immune3: Option<PromotionType>,
    pub promotion_random_apply: Option<PromotionType>,
    pub promotion_summon_perk: Option<PromotionType>,
    pub bonus_prereq: Option<BonusType>,
    /// C++ `getPromotionPrereqOr3()` (`<PromotionPrereqOr3>`, C++ `readPass2`).
    pub promotion_prereq_or3: Option<PromotionType>,
    /// C++ `getPromotionPrereqOr4()` (`<PromotionPrereqOr4>`, C++ `readPass2`).
    pub promotion_prereq_or4: Option<PromotionType>,
    /// C++ `getPromotionPrereqAnd()` (`<PromotionPrereqAnd>`, C++ `readPass2`).
    pub promotion_prereq_and: Option<PromotionType>,
    pub promotion_next_level: Option<PromotionType>,
    /// Python callback run every turn for units with this promotion (`<PyPerTurn>`).
    pub py_per_turn: String,
    pub promotion_combat_type: Option<PromotionType>,
    pub promotion_combat_mod: i32,
    /// C++ `getBonusAffinity(i)` (`<BonusAffinities>`).
    pub bonus_affinity: IdxVec<BonusType, i32>,
    /// C++ `getDamageTypeCombat(i)` (`<DamageTypeCombats>`).
    pub damage_type_combat: IdxVec<DamageType, i32>,
    /// C++ `getDamageTypeResist(i)` (`<DamageTypeResists>`).
    pub damage_type_resist: IdxVec<DamageType, i32>,
}

impl PromotionInfo {
    /// C++ `getCombatPercent()`: its attack's percentage (`stat_changes`), which is also the
    /// unit's strength percentage outside land battles.
    pub fn combat_percent(&self) -> i32 {
        self.stat_changes[StatType::Attack.idx()].percent
    }

    /// Its defence's percentage (`iCombatPercentDefense`).
    pub fn combat_percent_defense(&self) -> i32 {
        self.stat_changes[StatType::Defense.idx()].percent
    }

    /// C++ `getExtraCombatStr()`: its attack's points in strength points (tens).
    pub fn extra_combat_str(&self) -> i32 {
        self.stat_changes[StatType::Attack.idx()].flat / 10
    }

    /// C++ `getExtraCombatDefense()`: its defence's points in strength points (tens).
    pub fn extra_combat_defense(&self) -> i32 {
        self.stat_changes[StatType::Defense.idx()].flat / 10
    }
}

/// C++ `CvPromotionInfo::read` + `readPass2` + `readPass3`.
pub fn read_promotion(rd: &Rd) -> PromotionInfo {
    let mut base = InfoBase::read(rd);
    let leader = rd.bool("leader");
    if leader {
        // C++: "don't show in Civilopedia list of promotions".
        base.graphical_only = true;
    }
    PromotionInfo {
        base,
        // readPass2 (C++ uses GC.getInfoTypeForString for these three).
        prereq_promotion: rd.rf::<PromotionType>("promotion_prereq"),
        prereq_or_promotion1: rd.rf::<PromotionType>("promotion_prereq_or1"),
        prereq_or_promotion2: rd.rf::<PromotionType>("promotion_prereq_or2"),
        tech_prereq: rd.rf::<TechType>("tech_prereq"),
        state_religion_prereq: rd.rf::<ReligionType>("state_religion_prereq"),
        leader,
        blitz: rd.bool("blitz"),
        amphib: rd.bool("amphib"),
        river: rd.bool("river"),
        enemy_route: rd.bool("enemy_route"),
        always_heal: rd.bool("always_heal"),
        hills_double_move: rd.bool("hills_double_move"),
        immune_to_first_strikes: rd.bool("immune_to_first_strikes"),
        visibility_change: rd.int("visibility_change"),
        moves_change: rd.int("moves_change"),
        move_discount_change: rd.int("move_discount_change"),
        air_range_change: rd.int("air_range_change"),
        intercept_change: rd.int("intercept_change"),
        evasion_change: rd.int("evasion_change"),
        withdrawal_change: rd.int("withdrawal_change"),
        cargo_change: rd.int("cargo_change"),
        collateral_damage_change: rd.int("collateral_damage_change"),
        bombard_rate_change: rd.int("bombard_rate_change"),
        first_strikes_change: rd.int("first_strikes_change"),
        chance_first_strikes_change: rd.int("chance_first_strikes_change"),
        enemy_heal_change: rd.int("enemy_heal_change"),
        neutral_heal_change: rd.int("neutral_heal_change"),
        friendly_heal_change: rd.int("friendly_heal_change"),
        same_tile_heal_change: rd.int("same_tile_heal_change"),
        adjacent_tile_heal_change: rd.int("adjacent_tile_heal_change"),
        city_attack_percent: rd.int("city_attack"),
        city_defense_percent: rd.int("city_defense"),
        hills_attack_percent: rd.int("hills_attack"),
        hills_defense_percent: rd.int("hills_defense"),
        revolt_protection: rd.int("revolt_protection"),
        collateral_damage_protection: rd.int("collateral_damage_protection"),
        pillage_change: rd.int("pillage_change"),
        upgrade_discount: rd.int("upgrade_discount"),
        experience_percent: rd.int("experience_percent"),
        kamikaze_percent: rd.int("kamikaze_percent"),
        terrain_attack_percent: rd.pairs_int::<TerrainType>("terrain_attacks", 0),
        terrain_defense_percent: rd.pairs_int::<TerrainType>("terrain_defenses", 0),
        feature_attack_percent: rd.pairs_int::<FeatureType>("feature_attacks", 0),
        feature_defense_percent: rd.pairs_int::<FeatureType>("feature_defenses", 0),
        unit_combat_modifier_percent: rd.pairs_int::<UnitCombatType>("unit_combat_mods", 0),
        domain_modifier_percent: rd.pairs_int::<DomainType>("domain_mods", 0),
        terrain_double_move: rd.pairs_bool::<TerrainType>("terrain_double_moves"),
        feature_double_move: rd.pairs_bool::<FeatureType>("feature_double_moves"),
        unit_combat: rd.pairs_bool::<UnitCombatType>("unit_combats"),
        // ---- FfH ----
        ai_control: rd.bool("ai_control"),
        boarding: rd.bool("boarding"),
        only_defensive: rd.bool("only_defensive"),
        dispellable: rd.bool("dispellable"),
        double_fortify_bonus: rd.bool("double_fortify_bonus"),
        equipment: rd.bool("equipment"),
        fear: rd.bool("fear"),
        flying: rd.bool("flying"),
        held: rd.bool("held"),
        hidden_nationality: rd.bool("hidden_nationality"),
        ignore_building_defense: rd.bool("ignore_building_defense"),
        immortal: rd.bool("immortal"),
        immune_to_capture: rd.bool("immune_to_capture"),
        immune_to_defensive_strike: rd.bool("immune_to_defensive_strike"),
        immune_to_fear: rd.bool("immune_to_fear"),
        immune_to_magic: rd.bool("immune_to_magic"),
        invisible: rd.bool("invisible"),
        mutation: rd.bool("mutation"),
        not_alive: rd.bool("not_alive"),
        prereq_alive: rd.bool("prereq_alive"),
        race: rd.bool("race"),
        removed_by_casting: rd.bool("removed_by_casting"),
        removed_by_combat: rd.bool("removed_by_combat"),
        removed_when_healed: rd.bool("removed_when_healed"),
        see_invisible: rd.bool("see_invisible"),
        target_weakest_unit: rd.bool("target_weakest_unit"),
        target_weakest_unit_counter: rd.bool("target_weakest_unit_counter"),
        twincast: rd.bool("twincast"),
        validate: rd.bool("validate"),
        water_walking: rd.bool("water_walking"),
        ai_weight: rd.int("ai_weight"),
        betrayal_chance: rd.int("betrayal_chance"),
        better_defender_than_percent: rd.int("better_defender_than_percent"),
        caster_resist_modify: rd.int("caster_resist_modify"),
        combat_capture_percent: rd.int("combat_capture_percent"),
        combat_heal_percent: rd.int("combat_heal_percent"),
        combat_limit: rd.int("combat_limit"),
        stat_changes: read_stat_changes(rd),
        spell_damage_modify: rd.int("spell_damage_modify"),
        combat_percent_global_counter: rd.int("combat_percent_global_counter"),
        combat_percent_in_borders: rd.int("combat_percent_in_borders"),
        defensive_strike_chance: rd.int("defensive_strike_chance"),
        defensive_strike_damage: rd.int("defensive_strike_damage"),
        expire_chance: rd.int("expire_chance"),
        free_xp_per_turn: rd.int("free_xp_per_turn"),
        free_xp_from_combat: rd.int("free_xp_from_combat"),
        gold_from_combat: rd.int("gold_from_combat"),
        min_level: rd.int("min_level"),
        modify_global_counter: rd.int("modify_global_counter"),
        modify_global_counter_on_combat: rd.int("modify_global_counter_on_combat"),
        resist_magic: rd.int("resist_magic"),
        spell_caster_xp: rd.int("spell_caster_xp"),
        work_rate_modify: rd.int("work_rate_modify"),
        capture_unit_combat: rd.rf::<UnitCombatType>("capture_unit_combat"),
        bonus_prereq: rd.rf::<BonusType>("bonus_prereq"),
        py_per_turn: rd.string("py_per_turn"),
        promotion_combat_mod: rd.int("promotion_combat_mod"),
        bonus_affinity: rd.pairs_int::<BonusType>("bonus_affinities", 0),
        damage_type_combat: rd.pairs_int::<DamageType>("damage_type_combats", 0),
        damage_type_resist: rd.pairs_int::<DamageType>("damage_type_resists", 0),
        // readPass2 (FfH)
        promotion_combat_apply: rd.rf::<PromotionType>("promotion_combat_apply"),
        promotion_immune1: rd.rf::<PromotionType>("promotion_immune1"),
        promotion_immune2: rd.rf::<PromotionType>("promotion_immune2"),
        promotion_immune3: rd.rf::<PromotionType>("promotion_immune3"),
        promotion_random_apply: rd.rf::<PromotionType>("promotion_random_apply"),
        promotion_summon_perk: rd.rf::<PromotionType>("promotion_summon_perk"),
        promotion_prereq_or3: rd.rf::<PromotionType>("promotion_prereq_or3"),
        promotion_prereq_or4: rd.rf::<PromotionType>("promotion_prereq_or4"),
        promotion_prereq_and: rd.rf::<PromotionType>("promotion_prereq_and"),
        promotion_next_level: rd.rf::<PromotionType>("promotion_next_level"),
        promotion_combat_type: rd.rf::<PromotionType>("promotion_combat_type"),
    }
}

// ---------------------------------------------------------------------------------------------
// BuildInfo
// ---------------------------------------------------------------------------------------------

/// A key bound to an action (mission, command, build, automation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hotkey {
    /// The key (`KB_F`, `KB_SPACE`, ...).
    pub key: String,
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
    /// Among the actions bound to one key, the highest priority's is taken.
    pub priority: i32,
}

/// An action's `hotkeys`: a list of `{key, alt, shift, ctrl, priority}` (the flags false and the
/// priority 0 when absent).
pub fn read_hotkeys(rd: &Rd) -> Vec<Hotkey> {
    let Some(list) = rd.child("hotkeys") else { return Vec::new() };
    list.children()
        .map(|b| Hotkey { key: b.string("key"), alt: b.bool("alt"), shift: b.bool("shift"), ctrl: b.bool("ctrl"), priority: b.int("priority") })
        .collect()
}

/// C++ `CvBuildInfo` (`builds.jsonc`)
///
/// Not ported: `EntityEvent` (worker animation) and `m_iMissionType` (always `MISSION_BUILD`, set
/// by the UI action-list setup).
#[derive(Debug, Clone, Default)]
pub struct BuildInfo {
    pub base: InfoBase,
    /// The keys bound to this action (C++ `CvHotkeyInfo`: the interface's; the client reads them).
    pub hotkeys: Vec<Hotkey>,
    /// C++ `getOrderPriority()`: the action button's place in the interface (higher first).
    pub order_priority: i32,
    pub time: i32,
    pub cost: i32,
    /// C++ `getTechPrereq()` (`<PrereqTech>`).
    pub tech_prereq: Option<TechType>,
    /// C++ `getImprovement()` (`<ImprovementType>`).
    pub improvement: Option<ImprovementType>,
    /// C++ `getRoute()` (`<RouteType>`).
    pub route: Option<RouteType>,
    pub kill: bool,
    /// C++ `getFeatureTech(i)`: tech needed to build on the feature (`<FeatureStructs>`).
    pub feature_tech: IdxVec<FeatureType, Option<TechType>>,
    /// C++ `getFeatureTime(i)`: extra build time on the feature.
    pub feature_time: IdxVec<FeatureType, i32>,
    /// C++ `getFeatureProduction(i)`: production gained when the feature is removed.
    pub feature_production: IdxVec<FeatureType, i32>,
    /// C++ `isFeatureRemove(i)`.
    pub feature_remove: IdxVec<FeatureType, bool>,
}

/// C++ `CvBuildInfo::read`.
pub fn read_build(rd: &Rd) -> BuildInfo {
    let mut info = BuildInfo {
        base: InfoBase::read(rd),
        hotkeys: read_hotkeys(rd),
        order_priority: rd.int("order_priority"),
        tech_prereq: rd.rf::<TechType>("prereq_tech"),
        time: rd.int("time"),
        cost: rd.int("cost"),
        kill: rd.bool("kill"),
        improvement: rd.rf::<ImprovementType>("improvement_type"),
        route: rd.rf::<RouteType>("route_type"),
        ..Default::default()
    };
    set_feature_struct(rd, &mut info);
    info
}

/// C++ `CvXMLLoadUtility::SetFeatureStruct`: every feature gets tech -1, time 0, production 0,
/// remove false; then each `<FeatureStruct>` overwrites its feature's entries. Absent child tags
/// take the `GetChildXmlValByName` defaults (`""` -> no tech, 0, false).
fn set_feature_struct(rd: &Rd, info: &mut BuildInfo) {
    let n = rd.reg().count::<FeatureType>();
    info.feature_tech = IdxVec::from_elem(None, n);
    info.feature_time = IdxVec::from_elem(0, n);
    info.feature_production = IdxVec::from_elem(0, n);
    info.feature_remove = IdxVec::from_elem(false, n);
    let Some(list) = rd.child("feature_structs") else { return };
    for s in list.children() {
        let Some(f) = s.rf::<FeatureType>("feature_type") else {
            // C++ shows "iFeatureIndex is -1" and then writes out of bounds.
            rd.cx.warn(format!("<FeatureStructs> entry without a valid FeatureType in {}", info.base.type_));
            continue;
        };
        info.feature_tech[f] = s.rf::<TechType>("prereq_tech");
        info.feature_time[f] = s.int("time");
        info.feature_production[f] = s.int("production");
        info.feature_remove[f] = s.bool("remove");
    }
}

// ---------------------------------------------------------------------------------------------
// UnitInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvUnitInfo` (units.jsonc)
///
/// Not ported (graphics/UI): `Advisor`, `bRenderBelowWater`, `bRenderAlways`, `UnitMeshGroups`
/// (`getGroupSize`, `getGroupDefinitions`, `getUnitGroupRequired`, wave sizes, `fMaxSpeed`,
/// `fPadTime`, art define tags), `FormationType`, `Image` (popup picture), the art-define button.
#[derive(Debug, Clone, Default)]
pub struct UnitInfo {
    pub base: InfoBase,
    /// C++ `getUnitClassType()` (`<Class>`).
    pub unit_class_type: Option<UnitClassType>,
    /// C++ `getSpecialUnitType()` (`<Special>`).
    pub special_unit_type: Option<SpecialUnitType>,
    /// C++ `getUnitCaptureClassType()` (`<Capture>`): unit class created when this unit is captured.
    pub unit_capture_class_type: Option<UnitClassType>,
    /// C++ `getUnitCombatType()` (`<Combat>`).
    pub unit_combat_type: Option<UnitCombatType>,
    /// C++ `getDomainType()` (`<Domain>`).
    pub domain_type: Option<DomainType>,
    /// C++ `getDefaultUnitAIType()` (`<DefaultUnitAI>`).
    pub default_unit_ai_type: Option<UnitAIType>,
    /// C++ `getInvisibleType()` (`<Invisible>`).
    pub invisible_type: Option<InvisibleType>,
    /// C++ `getSeeInvisibleType(i)` / `getNumSeeInvisibleTypes()` (comma-separated `<SeeInvisible>`,
    /// NONE/unknown tokens dropped).
    pub see_invisible_types: Vec<InvisibleType>,
    pub animal: bool,
    /// C++ `isFoodProduction()` (`<bFood>`).
    pub food_production: bool,
    pub no_bad_goodies: bool,
    pub only_defensive: bool,
    pub no_capture: bool,
    /// C++ `isQuickCombat()`; only decides whether combat is animated (`CvUnit::isCombatVisible`).
    pub quick_combat: bool,
    pub rival_territory: bool,
    pub military_happiness: bool,
    pub military_support: bool,
    pub military_production: bool,
    pub pillage: bool,
    pub spy: bool,
    pub sabotage: bool,
    pub destroy: bool,
    pub steal_plans: bool,
    pub investigate: bool,
    pub counter_spy: bool,
    pub found: bool,
    pub golden_age: bool,
    pub invisible: bool,
    pub first_strike_immune: bool,
    pub no_defensive_bonus: bool,
    pub ignore_building_defense: bool,
    pub can_move_impassable: bool,
    pub can_move_all_terrain: bool,
    pub flat_movement_cost: bool,
    pub ignore_terrain_cost: bool,
    pub nuke_immune: bool,
    pub prereq_bonuses: bool,
    /// C++ `isPrereqReligion()` (`<bPrereqReligion>`); named `is_` because `getPrereqReligion()`
    /// (the religion type) takes `prereq_religion`.
    pub is_prereq_religion: bool,
    /// C++ `isMechUnit()` (`<bMechanized>`). FfH: mechanized units are not alive.
    pub mech_unit: bool,
    pub suicide: bool,
    pub line_of_sight: bool,
    pub hidden_nationality: bool,
    pub always_hostile: bool,
    pub no_reveal_map: bool,
    /// C++ `getUpgradeUnitClass(i)` (`<UnitClassUpgrades>`).
    pub upgrade_unit_class: IdxVec<UnitClassType, bool>,
    /// C++ `getTargetUnitClass(i)` (`<UnitClassTargets>`).
    pub target_unit_class: IdxVec<UnitClassType, bool>,
    /// C++ `getTargetUnitCombat(i)` (`<UnitCombatTargets>`).
    pub target_unit_combat: IdxVec<UnitCombatType, bool>,
    /// C++ `getDefenderUnitClass(i)` (`<UnitClassDefenders>`).
    pub defender_unit_class: IdxVec<UnitClassType, bool>,
    /// C++ `getDefenderUnitCombat(i)` (`<UnitCombatDefenders>`).
    pub defender_unit_combat: IdxVec<UnitCombatType, bool>,
    /// C++ `getFlankingStrikeUnitClass(i)` (`<FlankingStrikes>`).
    pub flanking_strike_unit_class: IdxVec<UnitClassType, i32>,
    /// C++ `getUnitAIType(i)` (`<UnitAIs>`): UnitAIs the AI may assign to this unit.
    pub unit_ai_type: IdxVec<UnitAIType, bool>,
    /// C++ `getNotUnitAIType(i)` (`<NotUnitAIs>`).
    pub not_unit_ai_type: IdxVec<UnitAIType, bool>,
    pub builds: IdxVec<BuildType, bool>,
    pub religion_spreads: IdxVec<ReligionType, i32>,
    pub corporation_spreads: IdxVec<CorporationType, i32>,
    /// C++ `getTerrainPassableTech(i)` (`<TerrainPassableTechs>`).
    pub terrain_passable_tech: IdxVec<TerrainType, Option<TechType>>,
    /// C++ `getFeaturePassableTech(i)` (`<FeaturePassableTechs>`).
    pub feature_passable_tech: IdxVec<FeatureType, Option<TechType>>,
    pub great_peoples: IdxVec<SpecialistType, bool>,
    pub buildings: IdxVec<BuildingType, bool>,
    pub force_buildings: IdxVec<BuildingType, bool>,
    pub holy_city: Option<ReligionType>,
    pub religion_type: Option<ReligionType>,
    pub state_religion: Option<ReligionType>,
    /// C++ `getPrereqReligion()` (`<PrereqReligion>`).
    pub prereq_religion: Option<ReligionType>,
    pub prereq_corporation: Option<CorporationType>,
    pub prereq_building: Option<BuildingType>,
    /// C++ `getPrereqAndTech()` (`<PrereqTech>`).
    pub prereq_and_tech: Option<TechType>,
    /// C++ `getPrereqAndTechs(i)` (`<TechTypes>`). C++ stores a fixed array of
    /// `NUM_UNIT_AND_TECH_PREREQS` (GlobalDefines, 3) entries filled with -1; defines are not
    /// available while reading, so this keeps only the listed techs, in order (every C++ caller
    /// scans all slots and skips NO_TECH, so the two forms are equivalent).
    pub prereq_and_techs: Vec<TechType>,
    /// C++ `getPrereqAndBonus()` (`<BonusType>`).
    pub prereq_and_bonus: Option<BonusType>,
    /// C++ `getPrereqOrBonuses(i)` (`<PrereqBonuses>`). Same compaction as [`Self::prereq_and_techs`]
    /// (C++ array size: `NUM_UNIT_PREREQ_OR_BONUSES`, 4).
    pub prereq_or_bonuses: Vec<BonusType>,
    pub production_traits: IdxVec<TraitType, i32>,
    /// C++ `getFlavorValue(i)` (`<Flavors>`).
    pub flavor_value: IdxVec<FlavorType, i32>,
    pub ai_weight: i32,
    /// C++ `getProductionCost()` (`<iCost>`).
    pub production_cost: i32,
    pub hurry_cost_modifier: i32,
    pub advanced_start_cost: i32,
    pub advanced_start_cost_increase: i32,
    pub min_area_size: i32,
    pub moves: i32,
    pub air_range: i32,
    pub air_unit_cap: i32,
    pub drop_range: i32,
    pub nuke_range: i32,
    pub work_rate: i32,
    pub base_discover: i32,
    pub discover_multiplier: i32,
    pub base_hurry: i32,
    pub hurry_multiplier: i32,
    pub base_trade: i32,
    pub trade_multiplier: i32,
    pub great_work_culture: i32,
    pub espionage_points: i32,
    /// C++ `getTerrainImpassable(i)` (`<TerrainImpassables>`).
    pub terrain_impassable: IdxVec<TerrainType, bool>,
    /// C++ `getFeatureImpassable(i)` (`<FeatureImpassables>`).
    pub feature_impassable: IdxVec<FeatureType, bool>,
    pub combat: i32,
    pub combat_limit: i32,
    pub air_combat: i32,
    pub air_combat_limit: i32,
    /// C++ `getXPValueAttack()`.
    pub xp_value_attack: i32,
    /// C++ `getXPValueDefense()`.
    pub xp_value_defense: i32,
    pub first_strikes: i32,
    pub chance_first_strikes: i32,
    pub interception_probability: i32,
    pub evasion_probability: i32,
    /// C++ `getWithdrawalProbability()` (`<iWithdrawalProb>`).
    pub withdrawal_probability: i32,
    pub collateral_damage: i32,
    pub collateral_damage_limit: i32,
    pub collateral_damage_max_units: i32,
    /// C++ `getCityAttackModifier()` (`<iCityAttack>`).
    pub city_attack_modifier: i32,
    /// C++ `getCityDefenseModifier()` (`<iCityDefense>`).
    pub city_defense_modifier: i32,
    /// C++ `getAnimalCombatModifier()` (`<iAnimalCombat>`).
    pub animal_combat_modifier: i32,
    /// C++ `getHillsAttackModifier()` (`<iHillsAttack>`).
    pub hills_attack_modifier: i32,
    /// C++ `getHillsDefenseModifier()` (`<iHillsDefense>`).
    pub hills_defense_modifier: i32,
    /// C++ `getTerrainNative(i)` (`<TerrainNatives>`).
    pub terrain_native: IdxVec<TerrainType, bool>,
    /// C++ `getFeatureNative(i)` (`<FeatureNatives>`).
    pub feature_native: IdxVec<FeatureType, bool>,
    /// C++ `getTerrainAttackModifier(i)` (`<TerrainAttacks>`).
    pub terrain_attack_modifier: IdxVec<TerrainType, i32>,
    /// C++ `getTerrainDefenseModifier(i)` (`<TerrainDefenses>`).
    pub terrain_defense_modifier: IdxVec<TerrainType, i32>,
    /// C++ `getFeatureAttackModifier(i)` (`<FeatureAttacks>`).
    pub feature_attack_modifier: IdxVec<FeatureType, i32>,
    /// C++ `getFeatureDefenseModifier(i)` (`<FeatureDefenses>`).
    pub feature_defense_modifier: IdxVec<FeatureType, i32>,
    /// C++ `getUnitClassAttackModifier(i)` (`<UnitClassAttackMods>`).
    pub unit_class_attack_modifier: IdxVec<UnitClassType, i32>,
    /// C++ `getUnitClassDefenseModifier(i)` (`<UnitClassDefenseMods>`).
    pub unit_class_defense_modifier: IdxVec<UnitClassType, i32>,
    /// C++ `getUnitCombatModifier(i)` (`<UnitCombatMods>`).
    pub unit_combat_modifier: IdxVec<UnitCombatType, i32>,
    /// C++ `getUnitCombatCollateralImmune(i)` (`<UnitCombatCollateralImmunes>`); an int in C++.
    pub unit_combat_collateral_immune: IdxVec<UnitCombatType, i32>,
    /// C++ `getDomainModifier(i)` (`<DomainMods>`).
    pub domain_modifier: IdxVec<DomainType, i32>,
    /// C++ `getBonusProductionModifier(i)` (`<BonusProductionModifiers>`).
    pub bonus_production_modifier: IdxVec<BonusType, i32>,
    pub bomb_rate: i32,
    pub bombard_rate: i32,
    /// C++ `getSpecialCargo()` (`<SpecialCargo>`).
    pub special_cargo: Option<SpecialUnitType>,
    /// C++ `getDomainCargo()` (`<DomainCargo>`).
    pub domain_cargo: Option<DomainType>,
    /// C++ `getCargoSpace()` (`<iCargo>`).
    pub cargo_space: i32,
    /// C++ `getConscriptionValue()` (`<iConscription>`).
    pub conscription_value: i32,
    /// C++ `getCultureGarrisonValue()` (`<iCultureGarrison>`).
    pub culture_garrison_value: i32,
    pub extra_cost: i32,
    /// C++ `getAssetValue()` (`<iAsset>`).
    pub asset_value: i32,
    /// C++ `getPowerValue()` (`<iPower>`).
    pub power_value: i32,
    /// C++ `getUnitNames(i)` / `getNumUnitNames()` (`<UniqueNames>`, text keys).
    pub unit_names: Vec<String>,
    pub free_promotions: IdxVec<PromotionType, bool>,
    pub leader_promotion: Option<PromotionType>,
    pub leader_experience: i32,
    // ---- FfH ----
    pub abandon: bool,
    pub auto_raze: bool,
    pub disable_upgrade_to: bool,
    pub explode_in_combat: bool,
    /// C++ `isFreeXP()`.
    pub free_xp: bool,
    pub immortal: bool,
    pub immune_to_defensive_strike: bool,
    pub never_obsolete: bool,
    pub no_war_weariness: bool,
    pub combat_defense: i32,
    pub defensive_strike_chance: i32,
    pub defensive_strike_damage: i32,
    pub duration_from_combat: i32,
    pub enslavement_chance: i32,
    pub free_promotion_pick: i32,
    pub gold_from_combat: i32,
    pub min_level: i32,
    pub miscast_chance: i32,
    pub modify_global_counter: i32,
    /// C++ `getDiploVoteType()` (`<DiploVoteType>`): the vote source this unit carries.
    pub diplo_vote_type: Option<VoteSourceType>,
    pub equipment_promotion: Option<PromotionType>,
    pub prereq_alignment: Option<AlignmentType>,
    pub prereq_building_class: Option<BuildingClassType>,
    /// C++ `getPrereqCiv()` (`<PrereqCiv>`, resolved by C++ `readPass3`).
    pub prereq_civ: Option<CivilizationType>,
    pub prereq_civic: Option<CivicType>,
    pub prereq_global_counter: i32,
    pub promotion_from_combat: Option<PromotionType>,
    pub tier: i32,
    /// C++ `getUnitConvertFromCombat()` (C++ `readPass2`).
    pub unit_convert_from_combat: Option<UnitType>,
    pub unit_convert_from_combat_chance: i32,
    /// C++ `getUnitCreateFromCombat()` (C++ `readPass2`).
    pub unit_create_from_combat: Option<UnitType>,
    pub unit_create_from_combat_chance: i32,
    /// C++ `getUpgradeCiv()` (`<UpgradeCiv>`, resolved by C++ `readPass3`).
    pub upgrade_civ: Option<CivilizationType>,
    pub weapon_tier: i32,
    /// C++ `getWithdrawlProbDefensive()` (sic).
    pub withdrawl_prob_defensive: i32,
    /// Python callback after this unit loses a combat (`<PythonPostCombatLost>`).
    pub py_post_combat_lost: String,
    /// Python callback after this unit wins a combat (`<PythonPostCombatWon>`).
    pub py_post_combat_won: String,
    /// C++ `getDamageTypeCombat(i)` (`<DamageTypeCombats>`).
    pub damage_type_combat: IdxVec<DamageType, i32>,
    /// C++ `getBonusAffinity(i)` (`<BonusAffinities>`).
    pub bonus_affinity: IdxVec<BonusType, i32>,
    // ---- Sephi AI addon ----
    /// C++ `isAIblockExplore()`.
    pub ai_block_explore: bool,
    /// C++ `isAIblockPermDefense()`.
    pub ai_block_perm_defense: bool,
    /// C++ `isAIblockPatrol()`.
    pub ai_block_patrol: bool,
    // ---- This game's own: engagement combat (docs/ENGAGEMENTS.md) ----
    /// `CombatRole`: the unit's engagement role in place of the one its unit combat gives it
    /// (`GameData::type_combat_role`).
    pub combat_role: Option<CombatRoleType>,
    /// Its battle numbers (`stats`; docs/UNIT_STATS.md); `None` for a type that never fights a land
    /// battle (ships, air units).
    pub stats: Option<StatSheet>,
    /// `bLarge`: a large unit in a formation battle (giants, elephants, dragons, golems,
    /// treants, ...): it fills two cells side by side in one row and two slots, and has
    /// `ENGAGEMENT_LARGE_HP_PERCENT` of its role's HP.
    pub large: bool,
    /// `EngageRows` (three `Row`s, front, middle, rear, each an `Action` and its `iCount`): what
    /// it does from each row, in place of its role's rows (`GameData::engage_rows`); `None`: the
    /// role's.
    pub engage_rows: Option<[RowActions; ROWS]>,
}

/// C++ `CvUnitInfo::read` + `readPass2` + `readPass3`.
pub fn read_unit(rd: &Rd) -> UnitInfo {
    UnitInfo {
        base: InfoBase::read(rd),
        unit_class_type: rd.rf::<UnitClassType>("class"),
        special_unit_type: rd.rf::<SpecialUnitType>("special"),
        unit_capture_class_type: rd.rf::<UnitClassType>("capture"),
        unit_combat_type: rd.rf::<UnitCombatType>("unit_combat"),
        domain_type: rd.rf::<DomainType>("domain"),
        default_unit_ai_type: rd.rf::<UnitAIType>("default_unit_ai"),
        invisible_type: rd.rf::<InvisibleType>("invisible_type"),
        see_invisible_types: rd.ref_tokens::<InvisibleType>("see_invisible"),
        animal: rd.bool("animal"),
        food_production: rd.bool("food"),
        no_bad_goodies: rd.bool("no_bad_goodies"),
        only_defensive: rd.bool("only_defensive"),
        no_capture: rd.bool("no_capture"),
        quick_combat: rd.bool("quick_combat"),
        rival_territory: rd.bool("rival_territory"),
        military_happiness: rd.bool("military_happiness"),
        military_support: rd.bool("military_support"),
        military_production: rd.bool("military_production"),
        pillage: rd.bool("pillage"),
        spy: rd.bool("spy"),
        sabotage: rd.bool("sabotage"),
        destroy: rd.bool("destroy"),
        steal_plans: rd.bool("steal_plans"),
        investigate: rd.bool("investigate"),
        counter_spy: rd.bool("counter_spy"),
        found: rd.bool("found"),
        golden_age: rd.bool("golden_age"),
        invisible: rd.bool("invisible"),
        first_strike_immune: rd.bool("first_strike_immune"),
        no_defensive_bonus: rd.bool("no_defensive_bonus"),
        ignore_building_defense: rd.bool("ignore_building_defense"),
        can_move_impassable: rd.bool("can_move_impassable"),
        can_move_all_terrain: rd.bool("can_move_all_terrain"),
        flat_movement_cost: rd.bool("flat_movement_cost"),
        ignore_terrain_cost: rd.bool("ignore_terrain_cost"),
        nuke_immune: rd.bool("nuke_immune"),
        prereq_bonuses: rd.bool("prereq_bonuses"),
        is_prereq_religion: rd.bool("is_prereq_religion"),
        mech_unit: rd.bool("mechanized"),
        suicide: rd.bool("suicide"),
        line_of_sight: rd.bool("line_of_sight"),
        hidden_nationality: rd.bool("hidden_nationality"),
        always_hostile: rd.bool("always_hostile"),
        no_reveal_map: rd.bool("no_reveal_map"),
        upgrade_unit_class: rd.pairs_bool::<UnitClassType>("unit_class_upgrades"),
        target_unit_class: rd.pairs_bool::<UnitClassType>("unit_class_targets"),
        target_unit_combat: rd.pairs_bool::<UnitCombatType>("unit_combat_targets"),
        defender_unit_class: rd.pairs_bool::<UnitClassType>("unit_class_defenders"),
        defender_unit_combat: rd.pairs_bool::<UnitCombatType>("unit_combat_defenders"),
        flanking_strike_unit_class: rd.pairs_int::<UnitClassType>("flanking_strikes", 0),
        unit_ai_type: rd.pairs_bool::<UnitAIType>("unit_ais"),
        not_unit_ai_type: rd.pairs_bool::<UnitAIType>("not_unit_ais"),
        builds: rd.pairs_bool::<BuildType>("builds"),
        religion_spreads: rd.pairs_int::<ReligionType>("religion_spreads", 0),
        corporation_spreads: rd.pairs_int::<CorporationType>("corporation_spreads", 0),
        terrain_passable_tech: rd.pairs_ref::<TerrainType, TechType>("terrain_passable_techs"),
        feature_passable_tech: rd.pairs_ref::<FeatureType, TechType>("feature_passable_techs"),
        great_peoples: rd.pairs_bool::<SpecialistType>("great_peoples"),
        buildings: rd.pairs_bool::<BuildingType>("buildings"),
        force_buildings: rd.pairs_bool::<BuildingType>("force_buildings"),
        holy_city: rd.rf::<ReligionType>("holy_city"),
        religion_type: rd.rf::<ReligionType>("religion_type"),
        state_religion: rd.rf::<ReligionType>("state_religion"),
        prereq_religion: rd.rf::<ReligionType>("prereq_religion"),
        prereq_corporation: rd.rf::<CorporationType>("prereq_corporation"),
        prereq_building: rd.rf::<BuildingType>("prereq_building"),
        prereq_and_tech: rd.rf::<TechType>("prereq_tech"),
        prereq_and_techs: rd.ref_list::<TechType>("tech_types"),
        prereq_and_bonus: rd.rf::<BonusType>("bonus_type"),
        prereq_or_bonuses: rd.ref_list::<BonusType>("prereq_or_bonuses"),
        production_traits: rd.pairs_int::<TraitType>("production_traits", 0),
        flavor_value: rd.pairs_int::<FlavorType>("flavors", 0),
        ai_weight: rd.int("ai_weight"),
        production_cost: rd.int("cost"),
        hurry_cost_modifier: rd.int("hurry_cost_modifier"),
        advanced_start_cost: rd.int("advanced_start_cost"),
        advanced_start_cost_increase: rd.int("advanced_start_cost_increase"),
        min_area_size: rd.int("min_area_size"),
        moves: rd.int("moves"),
        air_range: rd.int("air_range"),
        air_unit_cap: rd.int("air_unit_cap"),
        drop_range: rd.int("drop_range"),
        nuke_range: rd.int("nuke_range"),
        work_rate: rd.int("work_rate"),
        base_discover: rd.int("base_discover"),
        discover_multiplier: rd.int("discover_multiplier"),
        base_hurry: rd.int("base_hurry"),
        hurry_multiplier: rd.int("hurry_multiplier"),
        base_trade: rd.int("base_trade"),
        trade_multiplier: rd.int("trade_multiplier"),
        great_work_culture: rd.int("great_work_culture"),
        espionage_points: rd.int("espionage_points"),
        terrain_impassable: rd.pairs_bool::<TerrainType>("terrain_impassables"),
        feature_impassable: rd.pairs_bool::<FeatureType>("feature_impassables"),
        combat: rd.int("combat"),
        combat_limit: rd.int("combat_limit"),
        air_combat: rd.int("air_combat"),
        air_combat_limit: rd.int("air_combat_limit"),
        xp_value_attack: rd.int("xp_value_attack"),
        xp_value_defense: rd.int("xp_value_defense"),
        first_strikes: rd.int("first_strikes"),
        chance_first_strikes: rd.int("chance_first_strikes"),
        interception_probability: rd.int("interception_probability"),
        evasion_probability: rd.int("evasion_probability"),
        withdrawal_probability: rd.int("withdrawal_prob"),
        collateral_damage: rd.int("collateral_damage"),
        collateral_damage_limit: rd.int("collateral_damage_limit"),
        collateral_damage_max_units: rd.int("collateral_damage_max_units"),
        city_attack_modifier: rd.int("city_attack"),
        city_defense_modifier: rd.int("city_defense"),
        animal_combat_modifier: rd.int("animal_combat"),
        hills_attack_modifier: rd.int("hills_attack"),
        hills_defense_modifier: rd.int("hills_defense"),
        terrain_native: rd.pairs_bool::<TerrainType>("terrain_natives"),
        feature_native: rd.pairs_bool::<FeatureType>("feature_natives"),
        terrain_attack_modifier: rd.pairs_int::<TerrainType>("terrain_attacks", 0),
        terrain_defense_modifier: rd.pairs_int::<TerrainType>("terrain_defenses", 0),
        feature_attack_modifier: rd.pairs_int::<FeatureType>("feature_attacks", 0),
        feature_defense_modifier: rd.pairs_int::<FeatureType>("feature_defenses", 0),
        unit_class_attack_modifier: rd.pairs_int::<UnitClassType>("unit_class_attack_mods", 0),
        unit_class_defense_modifier: rd.pairs_int::<UnitClassType>("unit_class_defense_mods", 0),
        unit_combat_modifier: rd.pairs_int::<UnitCombatType>("unit_combat_mods", 0),
        unit_combat_collateral_immune: rd.pairs_int::<UnitCombatType>("unit_combat_collateral_immunes", 0),
        domain_modifier: rd.pairs_int::<DomainType>("domain_mods", 0),
        bonus_production_modifier: rd.pairs_int::<BonusType>("bonus_production_modifiers", 0),
        bomb_rate: rd.int("bomb_rate"),
        bombard_rate: rd.int("bombard_rate"),
        // The C++ constructor sets these two to 0, but read() always overwrites them with
        // FindInInfoClass(..), i.e. -1 when the tag is absent or NONE.
        special_cargo: rd.rf::<SpecialUnitType>("special_cargo"),
        domain_cargo: rd.rf::<DomainType>("domain_cargo"),
        cargo_space: rd.int("cargo"),
        conscription_value: rd.int("conscription"),
        culture_garrison_value: rd.int("culture_garrison"),
        extra_cost: rd.int("extra_cost"),
        asset_value: rd.int("asset"),
        power_value: rd.int("power"),
        unit_names: rd.string_list("unique_names"),
        free_promotions: rd.pairs_bool::<PromotionType>("free_promotions"),
        leader_promotion: rd.rf::<PromotionType>("leader_promotion"),
        leader_experience: rd.int("leader_experience"),
        // ---- FfH ----
        abandon: rd.bool("abandon"),
        auto_raze: rd.bool("auto_raze"),
        disable_upgrade_to: rd.bool("disable_upgrade_to"),
        explode_in_combat: rd.bool("explode_in_combat"),
        free_xp: rd.bool("free_xp"),
        immortal: rd.bool("immortal"),
        immune_to_defensive_strike: rd.bool("immune_to_defensive_strike"),
        never_obsolete: rd.bool("never_obsolete"),
        no_war_weariness: rd.bool("no_war_weariness"),
        combat_defense: rd.int("combat_defense"),
        defensive_strike_chance: rd.int("defensive_strike_chance"),
        defensive_strike_damage: rd.int("defensive_strike_damage"),
        duration_from_combat: rd.int("duration_from_combat"),
        enslavement_chance: rd.int("enslavement_chance"),
        free_promotion_pick: rd.int("free_promotion_pick"),
        gold_from_combat: rd.int("gold_from_combat"),
        min_level: rd.int("min_level"),
        miscast_chance: rd.int("miscast_chance"),
        modify_global_counter: rd.int("modify_global_counter"),
        diplo_vote_type: rd.rf::<VoteSourceType>("diplo_vote_type"),
        equipment_promotion: rd.rf::<PromotionType>("equipment_promotion"),
        prereq_alignment: rd.rf::<AlignmentType>("prereq_alignment"),
        prereq_building_class: rd.rf::<BuildingClassType>("prereq_building_class"),
        prereq_civ: rd.rf::<CivilizationType>("prereq_civ"),
        prereq_civic: rd.rf::<CivicType>("prereq_civic"),
        promotion_from_combat: rd.rf::<PromotionType>("promotion_from_combat"),
        prereq_global_counter: rd.int("prereq_global_counter"),
        tier: rd.int("tier"),
        unit_convert_from_combat_chance: rd.int("unit_convert_from_combat_chance"),
        unit_create_from_combat_chance: rd.int("unit_create_from_combat_chance"),
        upgrade_civ: rd.rf::<CivilizationType>("upgrade_civ"),
        weapon_tier: rd.int("weapon_tier"),
        withdrawl_prob_defensive: rd.int("withdrawl_prob_defensive"),
        py_post_combat_lost: rd.string("python_post_combat_lost"),
        py_post_combat_won: rd.string("python_post_combat_won"),
        damage_type_combat: rd.pairs_int::<DamageType>("damage_type_combats", 0),
        bonus_affinity: rd.pairs_int::<BonusType>("bonus_affinities", 0),
        // ---- Sephi AI addon ----
        ai_block_explore: rd.bool("ai_block_explore"),
        ai_block_perm_defense: rd.bool("ai_block_perm_defense"),
        ai_block_patrol: rd.bool("ai_block_patrol"),
        // ---- This game's own: engagement combat ----
        combat_role: rd.rf::<CombatRoleType>("combat_role"),
        large: rd.bool("large"),
        stats: read_stats(rd, &rd.string("type")),
        engage_rows: read_engage_rows(rd, &rd.string("type")),
        // ---- readPass2 ----
        unit_convert_from_combat: rd.rf::<UnitType>("unit_convert_from_combat"),
        unit_create_from_combat: rd.rf::<UnitType>("unit_create_from_combat"),
    }
}

// ---------------------------------------------------------------------------------------------
// SpellInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvSpellInfo` (spells.jsonc)
///
/// Not ported: `Effect` (spell graphics) and `Sound`.
#[derive(Debug, Clone, Default)]
pub struct SpellInfo {
    pub base: InfoBase,
    pub promotion_prereq1: Option<PromotionType>,
    pub promotion_prereq2: Option<PromotionType>,
    pub unit_prereq: Option<UnitType>,
    pub unit_combat_prereq: Option<UnitCombatType>,
    pub unit_class_prereq: Option<UnitClassType>,
    pub unit_in_stack_prereq: Option<UnitType>,
    pub building_prereq: Option<BuildingType>,
    pub building_class_owned_prereq: Option<BuildingClassType>,
    pub civilization_prereq: Option<CivilizationType>,
    pub corporation_prereq: Option<CorporationType>,
    pub feature_or_prereq1: Option<FeatureType>,
    pub feature_or_prereq2: Option<FeatureType>,
    pub improvement_prereq: Option<ImprovementType>,
    pub promotion_in_stack_prereq: Option<PromotionType>,
    pub religion_prereq: Option<ReligionType>,
    pub state_religion_prereq: Option<ReligionType>,
    pub tech_prereq: Option<TechType>,
    /// C++ `isAllowAI()`.
    pub allow_ai: bool,
    /// C++ `isAllowAuto()` (Sephi addon).
    pub allow_auto: bool,
    pub allow_automate_terrain: bool,
    pub adjacent_to_water_only: bool,
    pub caster_must_be_alive: bool,
    pub caster_no_duration: bool,
    pub causes_war: bool,
    pub global: bool,
    pub in_borders_only: bool,
    pub in_city_only: bool,
    pub prereq_slave_trade: bool,
    pub resistable: bool,
    /// C++ `getAIWeight()`.
    pub ai_weight: i32,
    /// C++ `getAIWeightCity()`.
    pub ai_weight_city: i32,
    /// C++ `getAIWeightWar()`.
    pub ai_weight_war: i32,
    pub caster_min_level: i32,
    pub display_when_disabled: bool,
    pub has_casted: bool,
    pub ignore_has_casted: bool,
    pub range: i32,
    pub resist_modify: i32,
    pub damage: i32,
    pub damage_limit: i32,
    pub damage_type: Option<DamageType>,
    pub add_promotion_type1: Option<PromotionType>,
    pub add_promotion_type2: Option<PromotionType>,
    pub add_promotion_type3: Option<PromotionType>,
    pub remove_promotion_type1: Option<PromotionType>,
    pub remove_promotion_type2: Option<PromotionType>,
    pub remove_promotion_type3: Option<PromotionType>,
    pub buff_caster_only: bool,
    pub convert_unit_type: Option<UnitType>,
    pub create_building_type: Option<BuildingType>,
    pub create_feature_type: Option<FeatureType>,
    pub create_improvement_type: Option<ImprovementType>,
    pub spread_religion: Option<ReligionType>,
    pub create_unit_type: Option<UnitType>,
    pub create_unit_num: i32,
    pub copy_casters_promotions: bool,
    pub permanent_unit_create: bool,
    pub create_unit_promotion: Option<PromotionType>,
    pub immune_team: bool,
    pub immune_neutral: bool,
    pub immune_enemy: bool,
    pub immune_flying: bool,
    pub immune_not_alive: bool,
    pub ability: bool,
    pub dispel: bool,
    pub push: bool,
    pub remove_has_casted: bool,
    pub sacrifice_caster: bool,
    pub change_population: i32,
    pub cost: i32,
    pub delay: i32,
    pub immobile_turns: i32,
    pub miscast_chance: i32,
    /// Python callback run on a miscast (`<PyMiscast>`).
    pub py_miscast: String,
    /// Python callback that performs the spell (`<PyResult>`).
    pub py_result: String,
    /// Python callback that decides whether the spell can be cast (`<PyRequirement>`).
    pub py_requirement: String,
}

/// C++ `CvSpellInfo::read`.
///
/// C++ only assigns a type reference when the tag text is non-empty
/// (`if (szTextVal != "") m_iX = FindInInfoClass(szTextVal)`); otherwise the constructor's
/// NO_X stays. `rf` yields the same `None` for absent, empty and NONE tags.
pub fn read_spell(rd: &Rd) -> SpellInfo {
    SpellInfo {
        base: InfoBase::read(rd),
        promotion_prereq1: rd.rf::<PromotionType>("promotion_prereq1"),
        promotion_prereq2: rd.rf::<PromotionType>("promotion_prereq2"),
        unit_prereq: rd.rf::<UnitType>("unit_prereq"),
        unit_combat_prereq: rd.rf::<UnitCombatType>("unit_combat_prereq"),
        unit_class_prereq: rd.rf::<UnitClassType>("unit_class_prereq"),
        unit_in_stack_prereq: rd.rf::<UnitType>("unit_in_stack_prereq"),
        building_prereq: rd.rf::<BuildingType>("building_prereq"),
        building_class_owned_prereq: rd.rf::<BuildingClassType>("building_class_owned_prereq"),
        civilization_prereq: rd.rf::<CivilizationType>("civilization_prereq"),
        corporation_prereq: rd.rf::<CorporationType>("corporation_prereq"),
        feature_or_prereq1: rd.rf::<FeatureType>("feature_or_prereq1"),
        feature_or_prereq2: rd.rf::<FeatureType>("feature_or_prereq2"),
        improvement_prereq: rd.rf::<ImprovementType>("improvement_prereq"),
        promotion_in_stack_prereq: rd.rf::<PromotionType>("promotion_in_stack_prereq"),
        religion_prereq: rd.rf::<ReligionType>("religion_prereq"),
        state_religion_prereq: rd.rf::<ReligionType>("state_religion_prereq"),
        tech_prereq: rd.rf::<TechType>("tech_prereq"),
        allow_ai: rd.bool("allow_ai"),
        allow_auto: rd.bool("allow_auto"),
        allow_automate_terrain: rd.bool("allow_automate_terrain"),
        adjacent_to_water_only: rd.bool("adjacent_to_water_only"),
        caster_must_be_alive: rd.bool("caster_must_be_alive"),
        caster_no_duration: rd.bool("caster_no_duration"),
        causes_war: rd.bool("causes_war"),
        global: rd.bool("global"),
        in_borders_only: rd.bool("in_borders_only"),
        in_city_only: rd.bool("in_city_only"),
        prereq_slave_trade: rd.bool("prereq_slave_trade"),
        resistable: rd.bool("resistable"),
        ai_weight: rd.int("ai_weight"),
        ai_weight_city: rd.int("ai_weight_city"),
        ai_weight_war: rd.int("ai_weight_war"),
        caster_min_level: rd.int("caster_min_level"),
        display_when_disabled: rd.bool("display_when_disabled"),
        has_casted: rd.bool("has_casted"),
        ignore_has_casted: rd.bool("ignore_has_casted"),
        range: rd.int("range"),
        resist_modify: rd.int("resist_modify"),
        damage: rd.int("damage"),
        damage_limit: rd.int("damage_limit"),
        damage_type: rd.rf::<DamageType>("damage_type"),
        add_promotion_type1: rd.rf::<PromotionType>("add_promotion_type1"),
        add_promotion_type2: rd.rf::<PromotionType>("add_promotion_type2"),
        add_promotion_type3: rd.rf::<PromotionType>("add_promotion_type3"),
        remove_promotion_type1: rd.rf::<PromotionType>("remove_promotion_type1"),
        remove_promotion_type2: rd.rf::<PromotionType>("remove_promotion_type2"),
        remove_promotion_type3: rd.rf::<PromotionType>("remove_promotion_type3"),
        buff_caster_only: rd.bool("buff_caster_only"),
        convert_unit_type: rd.rf::<UnitType>("convert_unit_type"),
        create_building_type: rd.rf::<BuildingType>("create_building_type"),
        create_feature_type: rd.rf::<FeatureType>("create_feature_type"),
        create_improvement_type: rd.rf::<ImprovementType>("create_improvement_type"),
        spread_religion: rd.rf::<ReligionType>("spread_religion"),
        create_unit_type: rd.rf::<UnitType>("create_unit_type"),
        create_unit_num: rd.int("create_unit_num"),
        copy_casters_promotions: rd.bool("copy_casters_promotions"),
        permanent_unit_create: rd.bool("permanent_unit_create"),
        create_unit_promotion: rd.rf::<PromotionType>("create_unit_promotion"),
        immune_team: rd.bool("immune_team"),
        immune_neutral: rd.bool("immune_neutral"),
        immune_enemy: rd.bool("immune_enemy"),
        immune_flying: rd.bool("immune_flying"),
        immune_not_alive: rd.bool("immune_not_alive"),
        ability: rd.bool("ability"),
        dispel: rd.bool("dispel"),
        push: rd.bool("push"),
        remove_has_casted: rd.bool("remove_has_casted"),
        sacrifice_caster: rd.bool("sacrifice_caster"),
        change_population: rd.int("change_population"),
        cost: rd.int("cost"),
        delay: rd.int("delay"),
        immobile_turns: rd.int("immobile_turns"),
        miscast_chance: rd.int("miscast_chance"),
        py_miscast: rd.string("py_miscast"),
        py_result: rd.string("py_result"),
        py_requirement: rd.string("py_requirement"),
    }
}

// ---------------------------------------------------------------------------------------------
// AutomateInfo / CommandInfo / MissionInfo
// ---------------------------------------------------------------------------------------------

/// C++ `CvAutomateInfo` (automates.jsonc)
///
/// The XML entries have no `<Type>` (so `base.type_` is empty, as in C++); their names come from
/// global_types.jsonc `AutomateTypes`.
#[derive(Debug, Clone, Default)]
pub struct AutomateInfo {
    pub base: InfoBase,
    /// The keys bound to this action (C++ `CvHotkeyInfo`: the interface's; the client reads them).
    pub hotkeys: Vec<Hotkey>,
    /// C++ `getOrderPriority()`: the action button's place in the interface (higher first).
    pub order_priority: i32,
    /// C++ `getCommand()`: the command that starts this automation (COMMAND_AUTOMATE).
    pub command: Option<CommandType>,
    /// C++ `getAutomate()`.
    pub automate: Option<AutomateType>,
    /// C++ `getConfirmCommand()` (interface only).
    pub confirm_command: bool,
    /// C++ `getVisible()` (interface only).
    pub visible: bool,
}

/// C++ `CvAutomateInfo::read`.
pub fn read_automate(rd: &Rd) -> AutomateInfo {
    AutomateInfo {
        base: InfoBase::read(rd),
        hotkeys: read_hotkeys(rd),
        order_priority: rd.int("order_priority"),
        command: rd.rf::<CommandType>("command"),
        // C++ GC.getTypesEnum: AutomateTypes from global_types.jsonc.
        automate: rd.rf::<AutomateType>("automate"),
        confirm_command: rd.bool("confirm_command"),
        visible: rd.bool("visible"),
    }
}

/// C++ `CvCommandInfo` (commands.jsonc)
#[derive(Debug, Clone, Default)]
pub struct CommandInfo {
    pub base: InfoBase,
    /// The keys bound to this action (C++ `CvHotkeyInfo`: the interface's; the client reads them).
    pub hotkeys: Vec<Hotkey>,
    /// C++ `getOrderPriority()`: the action button's place in the interface (higher first).
    pub order_priority: i32,
    /// C++ `getAutomate()` (only assigned when `<Automate>` is present; NO_AUTOMATE otherwise).
    pub automate: Option<AutomateType>,
    /// C++ `getConfirmCommand()` (interface only).
    pub confirm_command: bool,
    /// C++ `getVisible()` (interface only).
    pub visible: bool,
    /// C++ `getAll()`: with ALT held the command applies to every unit of the type
    /// (`CvMessageData` DO_COMMAND).
    pub all: bool,
}

/// C++ `CvCommandInfo::read`.
pub fn read_command(rd: &Rd) -> CommandInfo {
    CommandInfo {
        base: InfoBase::read(rd),
        hotkeys: read_hotkeys(rd),
        order_priority: rd.int("order_priority"),
        automate: rd.rf::<AutomateType>("automate"),
        confirm_command: rd.bool("confirm_command"),
        visible: rd.bool("visible"),
        all: rd.bool("all"),
    }
}

/// C++ `CvMissionInfo` (missions.jsonc)
///
/// Not ported: `Waypoint` (effect name), `bSound` (only plays the action sound) and
/// `EntityEventType` (animation).
#[derive(Debug, Clone, Default)]
pub struct MissionInfo {
    pub base: InfoBase,
    /// The keys bound to this action (C++ `CvHotkeyInfo`: the interface's; the client reads them).
    pub hotkeys: Vec<Hotkey>,
    /// C++ `getOrderPriority()`: the action button's place in the interface (higher first).
    pub order_priority: i32,
    /// C++ `getTime()`: mission/combat timer length (`CvSelectionGroup`/`CvUnit` timers).
    pub time: i32,
    /// C++ `isTarget()`.
    pub target: bool,
    /// C++ `isBuild()`.
    pub build: bool,
    /// C++ `getVisible()` (interface only).
    pub visible: bool,
}

/// C++ `CvMissionInfo::read`.
pub fn read_mission(rd: &Rd) -> MissionInfo {
    MissionInfo {
        base: InfoBase::read(rd),
        hotkeys: read_hotkeys(rd),
        order_priority: rd.int("order_priority"),
        time: rd.int("time"),
        target: rd.bool("target"),
        build: rd.bool("build"),
        visible: rd.bool("visible"),
    }
}
