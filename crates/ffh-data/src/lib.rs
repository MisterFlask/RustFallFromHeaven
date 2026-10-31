//! FFH2 game data: loads the vendored Civ4/FFH2 XML into typed info tables.
//!
//! This is a port of the DLL's `CvInfos.cpp` + `CvXMLLoadUtility*.cpp`.
//! Field names mirror the C++ members (`m_iCombat` -> `combat`,
//! `m_piYieldChange` -> `yield_change`), so ported rules code maps
//! `kInfo.getCombat()` to `info.combat`.

pub mod defines;
pub mod defines_gen;
pub mod enums;
pub mod idx;
pub mod infos;
pub mod registry;
pub mod text;
pub mod xml;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use defines::{DefineMap, DefineVal};
pub use defines_gen::Defines;
pub use enums::*;
pub use idx::{opt_from_i32, opt_to_i32, Idx, IdxVec};
pub use infos::*;
pub use registry::{Ctx, InfoId, Rd, Registry};
pub use text::Text;
use xml::XmlDoc;

/// Declares every info table: field, id type, info type, xml file, reader fn.
macro_rules! game_data {
    ($( $field:ident : $id:ty => $info:ty, $file:literal, $read:path; )*) => {
        /// All static game data. Load once, then share as `&'static GameData`.
        pub struct GameData {
            pub reg: Registry,
            pub define_map: DefineMap,
            pub defines: Defines,
            pub text: Text,
            /// The lines each civilization's leaders have for each other civilization
            /// (`text/barks.md`, Markdown; empty when absent). Presentation only: ffh-play reads it.
            pub barks: String,
            /// Non-fatal data problems found while loading (unknown types etc.).
            pub warnings: Vec<String>,
            $( pub $field: IdxVec<$id, $info>, )*
            /// Derived at load (not XML): the promotions with `promotion_combat_mod > 0`, in id
            /// order. `CvUnit::maxCombatStr` only reads those, so it scans this short list
            /// instead of every promotion.
            pub combat_mod_promotions: Vec<PromotionType>,
            /// Derived at load (not XML): the engagement role of each unit type
            /// ([`GameData::type_combat_role`]). Every land and immobile type has one, a type
            /// without strength too (workers, settlers, great people): whether a unit fights is its
            /// own strength (C++ `CvUnit::canFight`), which promotions add to (a Worker that takes
            /// the Godslayer is a 3/3 unit, and Frontline). `None` for sea and air types only, which
            /// never take part in a land engagement. A unit's own role also depends on its
            /// promotions ([`Self::screen_role`]).
            pub unit_roles: IdxVec<UnitType, Option<CombatRoleType>>,
            /// Derived at load: the role a unit changes to by promotion, from the role with
            /// `FromRole` (from, promotion, to): a Frontline unit with Formation is a Screen.
            pub screen_role: Option<(CombatRoleType, PromotionType, CombatRoleType)>,
            /// Derived at load: the `bDefault` stance (new armies and garrisons, units in no army).
            pub default_stance: StanceType,
        }

        /// (xml file, registry class) for every info table, in load order.
        pub const INFO_FILES: &[(&str, &str)] = &[ $( ($file, <$id as InfoId>::CLASS), )* ];

        impl GameData {
            fn load_tables(docs: &Docs, reg: Registry, define_map: DefineMap, text: Text) -> GameData {
                let mut warnings = Vec::new();
                $(
                    let $field: IdxVec<$id, $info> = {
                        let doc = docs.get($file);
                        let cx = Ctx::new(&reg, $file);
                        let mut v = IdxVec::new();
                        for e in entries(doc) {
                            let rd = Rd::new(e, &cx);
                            v.push($read(&rd));
                        }
                        warnings.extend(cx.warnings.into_inner());
                        v
                    };
                )*
                let defines = Defines::from_map(&define_map);
                // (derived tables are filled by `GameData::derive`)
                GameData { reg, define_map, defines, text, barks: String::new(), warnings, $( $field, )* combat_mod_promotions: Vec::new(), unit_roles: IdxVec::new(), screen_role: None, default_stance: StanceType::default() }
            }
        }
    };
}

game_data! {
    // ---- BasicInfos ----
    alignments: AlignmentType => AlignmentInfo, "Basicinfos/CIV4AlignmentInfos.xml", basic::read_alignment;
    attitudes: AttitudeType => AttitudeInfo, "Basicinfos/CIV4AttitudeInfos.xml", basic::read_attitude;
    calendars: CalendarType => CalendarInfo, "Basicinfos/CIV4CalendarInfos.xml", basic::read_calendar;
    damages: DamageType => DamageInfo, "Basicinfos/CIV4DamageInfos.xml", basic::read_damage;
    denials: DenialType => DenialInfo, "Basicinfos/CIV4DenialInfos.xml", basic::read_denial;
    domains: DomainType => DomainInfo, "Basicinfos/CIV4DomainInfos.xml", basic::read_domain;
    invisibles: InvisibleType => InvisibleInfo, "Basicinfos/CIV4InvisibleInfos.xml", basic::read_invisible;
    memories: MemoryType => MemoryInfo, "Basicinfos/CIV4MemoryInfos.xml", basic::read_memory;
    months: MonthType => MonthInfo, "Basicinfos/CIV4MonthInfos.xml", basic::read_month;
    seasons: SeasonType => SeasonInfo, "Basicinfos/CIV4SeasonInfos.xml", basic::read_season;
    unit_ais: UnitAIType => UnitAIInfo, "Basicinfos/CIV4UnitAIInfos.xml", basic::read_unit_ai;
    unit_combats: UnitCombatType => UnitCombatInfo, "Basicinfos/CIV4UnitCombatInfos.xml", basic::read_unit_combat;
    // ---- Terrain ----
    yields: YieldType => YieldInfo, "Terrain/CIV4YieldInfos.xml", terrain::read_yield;
    terrains: TerrainType => TerrainInfo, "Terrain/CIV4TerrainInfos.xml", terrain::read_terrain;
    features: FeatureType => FeatureInfo, "Terrain/CIV4FeatureInfos.xml", terrain::read_feature;
    bonus_classes: BonusClassType => BonusClassInfo, "Terrain/CIV4BonusClassInfos.xml", terrain::read_bonus_class;
    bonuses: BonusType => BonusInfo, "Terrain/CIV4BonusInfos.xml", terrain::read_bonus;
    improvements: ImprovementType => ImprovementInfo, "Terrain/CIV4ImprovementInfos.xml", terrain::read_improvement;
    routes: RouteType => RouteInfo, "Misc/CIV4RouteInfos.xml", terrain::read_route;
    // ---- GameInfo ----
    commerces: CommerceType => CommerceInfo, "Gameinfo/CIV4CommerceInfo.xml", game::read_commerce;
    game_speeds: GameSpeedType => GameSpeedInfo, "Gameinfo/CIV4GameSpeedInfo.xml", game::read_game_speed;
    worlds: WorldSizeType => WorldInfo, "Gameinfo/CIV4WorldInfo.xml", game::read_world;
    climates: ClimateType => ClimateInfo, "Gameinfo/CIV4ClimateInfo.xml", game::read_climate;
    sea_levels: SeaLevelType => SeaLevelInfo, "Gameinfo/CIV4SeaLevelInfo.xml", game::read_sea_level;
    eras: EraType => EraInfo, "Gameinfo/CIV4EraInfos.xml", game::read_era;
    handicaps: HandicapType => HandicapInfo, "Gameinfo/CIV4HandicapInfo.xml", game::read_handicap;
    game_options: GameOptionType => GameOptionInfo, "Gameinfo/CIV4GameOptionInfos.xml", game::read_game_option;
    goodies: GoodyType => GoodyInfo, "Gameinfo/CIV4GoodyInfo.xml", game::read_goody;
    hurries: HurryType => HurryInfo, "Gameinfo/CIV4HurryInfo.xml", game::read_hurry;
    upkeeps: UpkeepType => UpkeepInfo, "Gameinfo/CIV4UpkeepInfo.xml", game::read_upkeep;
    culture_levels: CultureLevelType => CultureLevelInfo, "Gameinfo/CIV4CultureLevelInfo.xml", game::read_culture_level;
    victories: VictoryType => VictoryInfo, "Gameinfo/CIV4VictoryInfo.xml", game::read_victory;
    processes: ProcessType => ProcessInfo, "Gameinfo/CIV4ProcessInfo.xml", game::read_process;
    emphasizes: EmphasizeType => EmphasizeInfo, "Gameinfo/CIV4EmphasizeInfo.xml", game::read_emphasize;
    specialists: SpecialistType => SpecialistInfo, "Gameinfo/CIV4SpecialistInfos.xml", civics::read_specialist;
    religions: ReligionType => ReligionInfo, "Gameinfo/CIV4ReligionInfo.xml", civics::read_religion;
    corporations: CorporationType => CorporationInfo, "Gameinfo/CIV4CorporationInfo.xml", civics::read_corporation;
    civic_options: CivicOptionType => CivicOptionInfo, "Gameinfo/CIV4CivicOptionInfos.xml", civics::read_civic_option;
    civics: CivicType => CivicInfo, "Gameinfo/CIV4CivicInfos.xml", civics::read_civic;
    votes: VoteType => VoteInfo, "Gameinfo/CIV4VoteInfo.xml", civics::read_vote;
    vote_sources: VoteSourceType => VoteSourceInfo, "Gameinfo/CIV4VoteSourceInfos.xml", civics::read_vote_source;
    espionage_missions: EspionageMissionType => EspionageMissionInfo, "Gameinfo/CIV4EspionageMissionInfo.xml", civics::read_espionage_mission;
    projects: ProjectType => ProjectInfo, "Gameinfo/CIV4ProjectInfo.xml", civics::read_project;
    // ---- Technologies ----
    techs: TechType => TechInfo, "Technologies/CIV4TechInfos.xml", civics::read_tech;
    // ---- Buildings ----
    special_buildings: SpecialBuildingType => SpecialBuildingInfo, "Buildings/CIV4SpecialBuildingInfos.xml", building::read_special_building;
    building_classes: BuildingClassType => BuildingClassInfo, "Buildings/CIV4BuildingClassInfos.xml", building::read_building_class;
    buildings: BuildingType => BuildingInfo, "Buildings/CIV4BuildingInfos.xml", building::read_building;
    // ---- Civilizations ----
    traits: TraitType => TraitInfo, "Civilizations/CIV4TraitInfos.xml", civ::read_trait;
    civilizations: CivilizationType => CivilizationInfo, "Civilizations/CIV4CivilizationInfos.xml", civ::read_civilization;
    leader_heads: LeaderHeadType => LeaderHeadInfo, "Civilizations/CIV4LeaderHeadInfos.xml", civ::read_leader_head;
    diplomacies: DiploCommentType => DiplomacyInfo, "Gameinfo/CIV4DiplomacyInfos.xml", game::read_diplomacy;
    // ---- Units ----
    special_units: SpecialUnitType => SpecialUnitInfo, "Units/CIV4SpecialUnitInfos.xml", unit::read_special_unit;
    unit_classes: UnitClassType => UnitClassInfo, "Units/CIV4UnitClassInfos.xml", unit::read_unit_class;
    promotions: PromotionType => PromotionInfo, "Units/CIV4PromotionInfos.xml", unit::read_promotion;
    builds: BuildType => BuildInfo, "Units/CIV4BuildInfos.xml", unit::read_build;
    units: UnitType => UnitInfo, "Units/CIV4UnitInfos.xml", unit::read_unit;
    spells: SpellType => SpellInfo, "Units/CIV4SpellInfos.xml", unit::read_spell;
    automates: AutomateType => AutomateInfo, "Units/CIV4AutomateInfos.xml", unit::read_automate;
    commands: CommandType => CommandInfo, "Units/CIV4CommandInfos.xml", unit::read_command;
    missions: MissionType => MissionInfo, "Units/CIV4MissionInfos.xml", unit::read_mission;
    // ---- Events ----
    events: EventType => EventInfo, "Events/CIV4EventInfos.xml", event::read_event;
    event_triggers: EventTriggerType => EventTriggerInfo, "Events/CIV4EventTriggerInfos.xml", event::read_event_trigger;
    // ---- This game's own rules (not FFH2's) ----
    cultural_actions: CulturalActionType => CulturalActionInfo, "Gameinfo/CIV4CulturalActionInfos.xml", culture::read_cultural_action;
    governor_profiles: GovernorProfileType => GovernorProfileInfo, "Gameinfo/CIV4GovernorProfileInfos.xml", governor::read_governor_profile;
    governor_traits: GovernorTraitType => GovernorTraitInfo, "Gameinfo/CIV4GovernorTraitInfos.xml", governor::read_governor_trait;
    governor_names: GovernorNameType => GovernorNameInfo, "Gameinfo/CIV4GovernorNameInfos.xml", governor::read_governor_names;
    combat_roles: CombatRoleType => CombatRoleInfo, "Units/CIV4CombatRoleInfos.xml", engagement::read_combat_role;
    stances: StanceType => StanceInfo, "Units/CIV4StanceInfos.xml", engagement::read_stance;
}

/// Lists declared in GlobalTypes.xml that are referenced by type name in XML.
const GLOBAL_TYPE_LISTS: &[(&str, &str)] = &[
    ("FlavorTypes", "FlavorTypes"),
    ("ArtStyleTypes", "ArtStyleTypes"),
    ("CitySizeTypes", "CitySizeTypes"),
    ("ContactTypes", "ContactTypes"),
    ("DiplomacyPowerTypes", "DiplomacyPowerTypes"),
    ("DirectionTypes", "DirectionTypes"),
    // CIV4AutomateInfos.xml entries have no <Type>; C++ names them from this list.
    ("AutomateTypes", "AutomateInfo"),
];

struct Docs {
    docs: Vec<(String, XmlDoc)>,
}

impl Docs {
    fn get(&self, file: &str) -> &XmlDoc {
        &self.docs.iter().find(|(f, _)| f == file).expect("document not loaded").1
    }
}

/// Entry elements of an info file: root -> container -> entries.
/// A container nested inside the container (FFH2's CIV4CommandInfos.xml has one)
/// is flattened in place, matching the C++ enum order.
fn entries(doc: &XmlDoc) -> Vec<xml::Elem<'_>> {
    fn collect<'a>(container: xml::Elem<'a>, out: &mut Vec<xml::Elem<'a>>) {
        for c in container.children() {
            if c.name() == container.name() {
                collect(c, out);
            } else {
                out.push(c);
            }
        }
    }
    let mut out = Vec::new();
    if let Some(container) = doc.root().first_child() {
        collect(container, &mut out);
    }
    out
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

fn read_doc(dir: &Path, rel: &str) -> Result<XmlDoc, LoadError> {
    let p = dir.join(rel);
    let bytes = std::fs::read(&p).map_err(|e| LoadError(format!("{}: {}", p.display(), e)))?;
    XmlDoc::parse_bytes(&bytes).map_err(|e| LoadError(format!("{}: {}", p.display(), e)))
}

impl GameData {
    /// Loads everything from a data directory containing `xml/` and `text/`.
    pub fn load(data_dir: &Path) -> Result<GameData, LoadError> {
        let xml_dir = data_dir.join("xml");
        let mut docs = Vec::new();
        for (file, _) in INFO_FILES {
            docs.push((file.to_string(), read_doc(&xml_dir, file)?));
        }
        let docs = Docs { docs };

        // Pass 1: register every type name.
        let mut reg = Registry::new();
        for (file, class) in INFO_FILES {
            let doc = docs.get(file);
            let mut names = Vec::new();
            for e in entries(doc) {
                if e.name() != *class {
                    return Err(LoadError(format!("{file}: entry <{}> but expected <{class}>", e.name())));
                }
                names.push(e.child_text("Type").unwrap_or("").to_string());
            }
            reg.register_class(class, names);
        }
        let gt = read_doc(&xml_dir, "GlobalTypes.xml")?;
        for (list, class) in GLOBAL_TYPE_LISTS {
            if let Some(l) = gt.root().child(list) {
                let names = l.children().map(|c| c.text().to_string()).collect();
                reg.register_class(class, names);
            }
        }

        // Defines (text defines resolve through the registry).
        let mut define_map = DefineMap::default();
        for f in ["GlobalDefines.xml", "GlobalDefinesAlt.xml", "PythonCallbackDefines.xml", "RulesDefines.xml"] {
            define_map.read_file(&read_doc(&xml_dir, f)?, &reg);
        }

        let text = match std::fs::read(data_dir.join("text").join("en.tsv")) {
            Ok(b) => Text::parse_tsv(&String::from_utf8_lossy(&b)),
            Err(_) => Text::default(),
        };

        // Pass 2: typed tables.
        let mut gd = GameData::load_tables(&docs, reg, define_map, text);
        gd.barks = std::fs::read_to_string(data_dir.join("text").join("barks.md")).unwrap_or_default();
        for (name, a, b) in &gd.reg.duplicates {
            gd.warnings.push(format!("type {name} registered as both {a} and {b}"));
        }
        gd.check_enum_orders();
        gd.derive();
        Ok(gd)
    }

    /// Fills the derived (non-XML) tables from the loaded ones.
    fn derive(&mut self) {
        self.combat_mod_promotions =
            self.promotions.keys().filter(|&p| self.promotions[p].promotion_combat_mod > 0).collect();
        // A leader without `iCulturalAggression`: 100, more for a war-eager leader (FfH's
        // iLimitedWarRand runs from 60, eager, to 300, peaceful) and for a culture lover.
        let culture_flavor = self.reg.lookup::<FlavorType>("FLAVOR_CULTURE");
        for info in self.leader_heads.iter_mut() {
            if info.cultural_aggression < 0 {
                let flavor = culture_flavor.map_or(0, |f| info.flavor_value.get(f).copied().unwrap_or(0));
                info.cultural_aggression = (100 + (150 - info.limited_war_rand) / 3 + 2 * flavor).clamp(40, 180);
            }
        }
        self.derive_engagements();
    }

    /// Engagement combat's derived tables (`unit_roles`, `screen_role`, `default_stance`), and the
    /// checks the resolver relies on (one default role and stance, positive multipliers and HP).
    fn derive_engagements(&mut self) {
        let mut warnings = Vec::new();
        let defaults: Vec<CombatRoleType> = self.combat_roles.keys().filter(|&r| self.combat_roles[r].default).collect();
        if defaults.len() != 1 {
            warnings.push(format!("{} combat roles have bDefault (one expected)", defaults.len()));
        }
        for r in self.combat_roles.keys() {
            let info = &self.combat_roles[r];
            if info.attack_percent <= 0 || info.defense_percent <= 0 || info.max_hp <= 0 || info.targets.is_empty() {
                warnings.push(format!("{}: iAttackPercent, iDefensePercent and iMaxHP must be above 0 and TargetOrder not empty", info.base.type_));
            }
            if info.from_role.is_some() != info.from_promotion.is_some() {
                warnings.push(format!("{}: FromRole and FromPromotion go together", info.base.type_));
            }
        }
        let promoted: Vec<CombatRoleType> = self.combat_roles.keys().filter(|&r| self.combat_roles[r].from_role.is_some()).collect();
        if promoted.len() > 1 {
            warnings.push(format!("{} combat roles have a FromRole (at most one expected)", promoted.len()));
        }
        self.screen_role = promoted.first().and_then(|&to| {
            let info = &self.combat_roles[to];
            Some((info.from_role?, info.from_promotion?, to))
        });
        let stances: Vec<StanceType> = self.stances.keys().filter(|&s| self.stances[s].default).collect();
        if stances.len() != 1 {
            warnings.push(format!("{} stances have bDefault (one expected)", stances.len()));
        }
        self.default_stance = stances.first().copied().unwrap_or_default();
        self.unit_roles =
            IdxVec::from_fn(self.units.len(), |u| if self.type_engages_on_land(u) { self.type_combat_role(u) } else { None });
        self.warnings.extend(warnings);
    }

    /// Whether units of a type can take part in a land engagement once they have strength: a land
    /// domain (`DOMAIN_LAND`, or `DOMAIN_IMMOBILE`: Guardian Vines). Ships never defend a land plot
    /// and air units never engage. The type's own strength is not asked: whether a unit fights is
    /// decided per unit (C++ `CvUnit::canFight`, on `baseCombatStr` / `baseCombatStrDefense`,
    /// which promotions raise: equipment, Great Commander, Heroic Strength).
    pub fn type_engages_on_land(&self, u: UnitType) -> bool {
        matches!(self.units[u].domain_type, Some(DomainType::DOMAIN_LAND) | Some(DomainType::DOMAIN_IMMOBILE))
    }

    /// The engagement role a unit type's data gives it, its strength and domain aside: its
    /// `CombatRole`, else the first role whose `DefaultUnitCombats` holds its unit combat and
    /// whose `DefaultFreePromotion`, if any, it has free (roles with a promotion are tried first,
    /// then XML order: a recon unit with Marksman free is an Assassin), else the `bDefault` role.
    /// [`Self::unit_roles`] holds it for every land and immobile type.
    pub fn type_combat_role(&self, u: UnitType) -> Option<CombatRoleType> {
        let info = &self.units[u];
        if info.combat_role.is_some() {
            return info.combat_role;
        }
        if let Some(uc) = info.unit_combat_type {
            let claims = |r: CombatRoleType| {
                let role = &self.combat_roles[r];
                role.default_unit_combats.contains(&uc) && role.default_free_promotion.map_or(true, |p| info.free_promotions[p])
            };
            let with_promotion = self.combat_roles.keys().filter(|&r| self.combat_roles[r].default_free_promotion.is_some());
            let without = self.combat_roles.keys().filter(|&r| self.combat_roles[r].default_free_promotion.is_none());
            if let Some(r) = with_promotion.chain(without).find(|&r| claims(r)) {
                return Some(r);
            }
        }
        self.combat_roles.keys().find(|&r| self.combat_roles[r].default)
    }

    /// The change to each side's engagement slots on a plot (added to `ENGAGEMENT_SLOTS_BASE`):
    /// `ENGAGEMENT_SLOTS_HILLS` on hills, plus the feature's `iEngagementSlotsChange` where it is
    /// non-zero, else, on a flat plot, the terrain's. So the terrain's "open land" change counts only
    /// on a flat plot without a forest or jungle, and a feature replaces it.
    pub fn engagement_slots_change(&self, terrain: TerrainType, feature: Option<FeatureType>, hills: bool) -> i32 {
        let feature_change = feature.map_or(0, |f| self.features[f].engagement_slots_change);
        let cover = if feature_change != 0 {
            feature_change
        } else if hills {
            0
        } else {
            self.terrains[terrain].engagement_slots_change
        };
        (if hills { self.defines.engagement_slots_hills } else { 0 }) + cover
    }

    /// Verifies that data-defined tables whose order is hard-coded in C++ enums match.
    fn check_enum_orders(&mut self) {
        fn check(w: &mut Vec<String>, reg: &Registry, class: &str, names: &[&str]) {
            let got = reg.names_of(class);
            for (i, n) in names.iter().enumerate() {
                if got.get(i).map(|s| s.as_str()) != Some(*n) {
                    w.push(format!("{class}[{i}] is {:?} but C++ enum expects {n}", got.get(i)));
                }
            }
        }
        let w = &mut self.warnings;
        check(w, &self.reg, "DomainInfo", DomainType::NAMES);
        check(w, &self.reg, "UnitAIInfo", UnitAIType::NAMES);
        check(w, &self.reg, "YieldInfo", YieldType::NAMES);
        check(w, &self.reg, "CommerceInfo", CommerceType::NAMES);
        check(w, &self.reg, "MissionInfo", MissionType::NAMES);
        check(w, &self.reg, "CommandInfo", CommandType::NAMES);
        check(w, &self.reg, "AutomateInfo", AutomateType::NAMES);
        check(w, &self.reg, "GameOptionInfo", GameOptionType::NAMES);
        check(w, &self.reg, "MemoryInfo", MemoryType::NAMES);
        check(w, &self.reg, "AttitudeInfo", AttitudeType::NAMES);
        check(w, &self.reg, "DenialInfo", DenialType::NAMES);
        check(w, &self.reg, "DirectionTypes", DirectionType::NAMES);
        check(w, &self.reg, "AlignmentInfo", AlignmentType::NAMES);
        check(w, &self.reg, "DamageInfo", DamageType::NAMES);
        check(w, &self.reg, "CalendarInfo", CalendarType::NAMES);
        check(w, &self.reg, "WorldInfo", WorldSizeType::NAMES);
        check(w, &self.reg, "ContactTypes", ContactType::NAMES);
        check(w, &self.reg, "DiplomacyPowerTypes", DiplomacyPowerType::NAMES);
        check(w, &self.reg, "CitySizeTypes", CitySizeType::NAMES);
    }

    /// Typed lookup of a type name (C++ `GC.getInfoTypeForString` + cast).
    pub fn ty<T: InfoId>(&self, name: &str) -> Option<T> {
        self.reg.lookup::<T>(name)
    }

    /// Like [`Self::ty`] but panics if the name is unknown: for names hard-coded in ported rules.
    pub fn t<T: InfoId>(&self, name: &str) -> T {
        self.reg.lookup::<T>(name).unwrap_or_else(|| panic!("unknown {} type {name}", T::CLASS))
    }

    /// C++ `GC.getCivilizationInfo(civ).getCivilizationUnits(class)`: the civ's unit for a
    /// unit class (`None` = this civ cannot have it).
    pub fn civ_unit(&self, civ: CivilizationType, class: UnitClassType) -> Option<UnitType> {
        self.civilizations[civ].civilization_units(class, self.unit_classes[class].default_unit_index)
    }

    /// C++ `GC.getCivilizationInfo(civ).getCivilizationBuildings(class)`.
    pub fn civ_building(&self, civ: CivilizationType, class: BuildingClassType) -> Option<BuildingType> {
        self.civilizations[civ].civilization_buildings(class, &self.building_classes)
    }

    /// Type string of an index (C++ `getType()`).
    pub fn type_name<T: InfoId>(&self, t: T) -> &str {
        self.reg.type_name(t)
    }
}

/// Default data directory: `<repo>/data`.
pub fn default_data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FFH_DATA_DIR") {
        return PathBuf::from(d);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

static GLOBAL: OnceLock<GameData> = OnceLock::new();

/// Loads the default data once per process and returns a `'static` reference.
pub fn global() -> &'static GameData {
    GLOBAL.get_or_init(|| GameData::load(&default_data_dir()).expect("failed to load game data"))
}
