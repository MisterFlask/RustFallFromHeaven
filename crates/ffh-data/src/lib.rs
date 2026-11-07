//! FFH2 game data: loads `data/json/*.jsonc` (JSON with comments) into typed info tables.
//!
//! The tables are a port of the DLL's `CvInfos.cpp`; the data began as FFH2 0.41's XML.
//! Field names mirror the C++ members (`m_iCombat` -> `combat`,
//! `m_piYieldChange` -> `yield_change`), so ported rules code maps
//! `kInfo.getCombat()` to `info.combat`.

pub mod defines;
pub mod defines_gen;
pub mod enums;
pub mod idx;
pub mod infos;
pub mod jsonc;
pub mod registry;
pub mod text;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use defines::{DefineMap, DefineVal};
pub use defines_gen::Defines;
pub use enums::*;
pub use idx::{opt_from_i32, opt_to_i32, Idx, IdxVec};
pub use infos::*;
pub use registry::{Ctx, InfoId, Rd, Registry};
pub use text::Text;
use jsonc::Value;

/// Declares every info table: field, id type, info type, reader fn. Its data file is
/// `json/<field>.jsonc`.
macro_rules! game_data {
    ($( $field:ident : $id:ty => $info:ty, $read:path; )*) => {
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
            /// Derived at load (not in the data files): the promotions with `promotion_combat_mod > 0`, in id
            /// order. `CvUnit::maxCombatStr` only reads those, so it scans this short list
            /// instead of every promotion.
            pub combat_mod_promotions: Vec<PromotionType>,
            /// Derived at load (docs/SUITABILITY.md): the buildings with an effect on the units
            /// they unlock (`unlocked_unit_*`), in id order.
            pub unlocking_buildings: Vec<BuildingType>,
            /// Derived at load: the buildings with `population_commerce`, in id order.
            pub population_commerce_buildings: Vec<BuildingType>,
            /// Derived at load: the buildings with `sentinel_repair_change`, in id order.
            pub sentinel_repair_buildings: Vec<BuildingType>,
            /// Derived at load: the buildings with a `bonus_yield_change`, in id order.
            pub bonus_yield_buildings: Vec<BuildingType>,
            /// Derived at load: for each tech, the improvements that raise the research rate on it
            /// and by how many percent (`ImprovementInfo::tech_research_bonus`), in improvement id
            /// order.
            pub tech_research_bonuses: IdxVec<TechType, Vec<(ImprovementType, i32)>>,
        }

        /// (data file, registry class) for every info table, in load order.
        pub const INFO_FILES: &[(&str, &str)] = &[ $( (concat!(stringify!($field), ".jsonc"), <$id as InfoId>::CLASS), )* ];

        impl GameData {
            fn load_tables(docs: &Docs, reg: Registry, define_map: DefineMap, text: Text) -> GameData {
                let mut warnings = Vec::new();
                $(
                    let $field: IdxVec<$id, $info> = {
                        let file = concat!(stringify!($field), ".jsonc");
                        let doc = docs.get(file);
                        let cx = Ctx::new(&reg, file);
                        let mut v = IdxVec::new();
                        for e in entries(doc) {
                            cx.set_entry(e.get("type").and_then(Value::as_str).unwrap_or(""));
                            let rd = Rd::new(e, &cx);
                            v.push($read(&rd));
                            rd.check_unread();
                        }
                        warnings.extend(cx.warnings.into_inner());
                        v
                    };
                )*
                let defines = Defines::from_map(&define_map);
                // (derived tables are filled by `GameData::derive`)
                GameData { reg, define_map, defines, text, barks: String::new(), warnings, $( $field, )* combat_mod_promotions: Vec::new(), unlocking_buildings: Vec::new(), population_commerce_buildings: Vec::new(), sentinel_repair_buildings: Vec::new(), bonus_yield_buildings: Vec::new(), tech_research_bonuses: IdxVec::new() }
            }

            /// Everything loaded, one `(name, Debug text)` line per table entry, in a fixed order
            /// (the registry and the defines sorted): two loads compare line by line, to show a
            /// change of the data's format or of the loader left the game's data as it was.
            pub fn dump(&self) -> Vec<(String, String)> {
                let mut out = Vec::new();
                for class in self.reg.classes() {
                    out.push((format!("reg.{class}"), format!("{:?}", self.reg.names_of(class))));
                }
                let mut defines: Vec<_> = self.define_map.map.iter().map(|(k, v)| (k.clone(), format!("{v:?}"))).collect();
                defines.sort();
                for (k, v) in defines {
                    out.push((format!("define.{k}"), v));
                }
                out.push(("defines".to_string(), format!("{:?}", self.defines)));
                $(
                    for (i, e) in self.$field.iter().enumerate() {
                        out.push((format!("{}[{}]", stringify!($field), i), format!("{e:?}")));
                    }
                )*
                out.push(("combat_mod_promotions".to_string(), format!("{:?}", self.combat_mod_promotions)));
                out.push(("unlocking_buildings".to_string(), format!("{:?}", self.unlocking_buildings)));
                out.push(("tech_research_bonuses".to_string(), format!("{:?}", self.tech_research_bonuses)));
                out.push(("warnings".to_string(), format!("{:?}", self.warnings)));
                out
            }
        }
    };
}

game_data! {
    // ---- BasicInfos ----
    alignments: AlignmentType => AlignmentInfo, basic::read_alignment;
    attitudes: AttitudeType => AttitudeInfo, basic::read_attitude;
    calendars: CalendarType => CalendarInfo, basic::read_calendar;
    damages: DamageType => DamageInfo, basic::read_damage;
    denials: DenialType => DenialInfo, basic::read_denial;
    domains: DomainType => DomainInfo, basic::read_domain;
    invisibles: InvisibleType => InvisibleInfo, basic::read_invisible;
    memories: MemoryType => MemoryInfo, basic::read_memory;
    months: MonthType => MonthInfo, basic::read_month;
    seasons: SeasonType => SeasonInfo, basic::read_season;
    unit_ais: UnitAIType => UnitAIInfo, basic::read_unit_ai;
    unit_combats: UnitCombatType => UnitCombatInfo, basic::read_unit_combat;
    // ---- Terrain ----
    yields: YieldType => YieldInfo, terrain::read_yield;
    terrains: TerrainType => TerrainInfo, terrain::read_terrain;
    features: FeatureType => FeatureInfo, terrain::read_feature;
    bonus_classes: BonusClassType => BonusClassInfo, terrain::read_bonus_class;
    bonuses: BonusType => BonusInfo, terrain::read_bonus;
    improvements: ImprovementType => ImprovementInfo, terrain::read_improvement;
    routes: RouteType => RouteInfo, terrain::read_route;
    // ---- GameInfo ----
    commerces: CommerceType => CommerceInfo, game::read_commerce;
    game_speeds: GameSpeedType => GameSpeedInfo, game::read_game_speed;
    worlds: WorldSizeType => WorldInfo, game::read_world;
    climates: ClimateType => ClimateInfo, game::read_climate;
    sea_levels: SeaLevelType => SeaLevelInfo, game::read_sea_level;
    eras: EraType => EraInfo, game::read_era;
    handicaps: HandicapType => HandicapInfo, game::read_handicap;
    game_options: GameOptionType => GameOptionInfo, game::read_game_option;
    goodies: GoodyType => GoodyInfo, game::read_goody;
    hurries: HurryType => HurryInfo, game::read_hurry;
    upkeeps: UpkeepType => UpkeepInfo, game::read_upkeep;
    culture_levels: CultureLevelType => CultureLevelInfo, game::read_culture_level;
    victories: VictoryType => VictoryInfo, game::read_victory;
    processes: ProcessType => ProcessInfo, game::read_process;
    emphasizes: EmphasizeType => EmphasizeInfo, game::read_emphasize;
    specialists: SpecialistType => SpecialistInfo, civics::read_specialist;
    religions: ReligionType => ReligionInfo, civics::read_religion;
    corporations: CorporationType => CorporationInfo, civics::read_corporation;
    civic_options: CivicOptionType => CivicOptionInfo, civics::read_civic_option;
    civics: CivicType => CivicInfo, civics::read_civic;
    votes: VoteType => VoteInfo, civics::read_vote;
    vote_sources: VoteSourceType => VoteSourceInfo, civics::read_vote_source;
    espionage_missions: EspionageMissionType => EspionageMissionInfo, civics::read_espionage_mission;
    projects: ProjectType => ProjectInfo, civics::read_project;
    // ---- Technologies ----
    techs: TechType => TechInfo, civics::read_tech;
    // ---- Buildings ----
    special_buildings: SpecialBuildingType => SpecialBuildingInfo, building::read_special_building;
    building_classes: BuildingClassType => BuildingClassInfo, building::read_building_class;
    buildings: BuildingType => BuildingInfo, building::read_building;
    // ---- Civilizations ----
    traits: TraitType => TraitInfo, civ::read_trait;
    civilizations: CivilizationType => CivilizationInfo, civ::read_civilization;
    leader_heads: LeaderHeadType => LeaderHeadInfo, civ::read_leader_head;
    diplomacies: DiploCommentType => DiplomacyInfo, game::read_diplomacy;
    // ---- Units ----
    special_units: SpecialUnitType => SpecialUnitInfo, unit::read_special_unit;
    unit_classes: UnitClassType => UnitClassInfo, unit::read_unit_class;
    promotions: PromotionType => PromotionInfo, unit::read_promotion;
    builds: BuildType => BuildInfo, unit::read_build;
    units: UnitType => UnitInfo, unit::read_unit;
    spells: SpellType => SpellInfo, unit::read_spell;
    automates: AutomateType => AutomateInfo, unit::read_automate;
    commands: CommandType => CommandInfo, unit::read_command;
    missions: MissionType => MissionInfo, unit::read_mission;
    // ---- Events ----
    events: EventType => EventInfo, event::read_event;
    event_triggers: EventTriggerType => EventTriggerInfo, event::read_event_trigger;
    // ---- This game's own rules (not FFH2's) ----
    cultural_actions: CulturalActionType => CulturalActionInfo, culture::read_cultural_action;
    governor_profiles: GovernorProfileType => GovernorProfileInfo, governor::read_governor_profile;
    governor_traits: GovernorTraitType => GovernorTraitInfo, governor::read_governor_trait;
    governor_names: GovernorNameType => GovernorNameInfo, governor::read_governor_names;
    building_tiers: BuildingTierType => BuildingTierInfo, building::read_building_tier;
}

/// Type lists of `global_types.jsonc` (key, registry class): types the data names that are
/// no table's entries.
pub const GLOBAL_TYPE_LISTS: &[(&str, &str)] = &[
    ("flavor_types", "FlavorTypes"),
    ("art_style_types", "ArtStyleTypes"),
    ("city_size_types", "CitySizeTypes"),
    ("contact_types", "ContactTypes"),
    ("diplomacy_power_types", "DiplomacyPowerTypes"),
    ("direction_types", "DirectionTypes"),
    // The automate entries have no `type`; C++ names them from this list.
    ("automate_types", "AutomateInfo"),
];

/// The defines files, in load order (a later file's value replaces an earlier one's).
pub const DEFINES_FILES: &[&str] = &["global_defines.jsonc", "global_defines_alt.jsonc", "python_callback_defines.jsonc", "rules_defines.jsonc"];

struct Docs {
    docs: Vec<(String, Value)>,
}

impl Docs {
    fn get(&self, file: &str) -> &Value {
        &self.docs.iter().find(|(f, _)| f == file).expect("document not loaded").1
    }
}

/// The entries of a table's file: the items of its top-level list.
fn entries(doc: &Value) -> &[Value] {
    doc.as_arr().unwrap_or(&[])
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

/// This game's building tiers (docs/SUITABILITY.md; not FFH2's): the tier variants of buildings.
pub const BUILDING_VARIANTS_FILE: &str = "building_variants.jsonc";

/// Appends the tier variants of `building_variants.jsonc` to the buildings table: each variant is
/// the entry of the building its `variant_of` names, with the variant's keys in place of that
/// entry's own (a key the variant leaves out keeps the Standard building's value). Appended
/// after FFH2's buildings, which keep their ids.
fn add_building_variants(docs: &mut [(String, Value)], json_dir: &Path) -> Result<(), LoadError> {
    let file = BUILDING_VARIANTS_FILE;
    let variants = read_doc(json_dir, file)?;
    let Some(list) = variants.as_arr() else {
        return Err(LoadError(format!("{file}: expected a list of entries")));
    };
    let Some((_, Value::Arr(entries))) = docs.iter_mut().find(|(f, _)| f == "buildings.jsonc") else {
        return Err(LoadError("buildings.jsonc: expected a list of entries".to_string()));
    };
    let mut added = Vec::new();
    for v in list {
        let (Value::Obj(members), Some(base_name)) = (v, v.get("variant_of").and_then(Value::as_str)) else {
            return Err(LoadError(format!("{file}: an entry without variant_of")));
        };
        let base = entries.iter().find(|e| e.get("type").and_then(Value::as_str) == Some(base_name));
        let Some(Value::Obj(base_members)) = base else {
            return Err(LoadError(format!("{file}: variant_of {base_name:?} is no building")));
        };
        let mut merged = base_members.clone();
        for (k, val) in members {
            match merged.iter_mut().find(|(mk, _)| mk == k) {
                Some(slot) => slot.1 = val.clone(),
                None => merged.push((k.clone(), val.clone())),
            }
        }
        added.push(Value::Obj(merged));
    }
    entries.extend(added);
    Ok(())
}

/// The great-person deck's cards (docs/DECKS.md, "The deck as built"): one file per card, each an
/// object whose members are info tables (a table's file name without `.jsonc`: `techs`, `units`,
/// `promotions`, `buildings`, `spells`, ...) holding the entries the card adds, plus `text`, an
/// object of the card's English text (TXT_KEY -> string). The entries are appended to their
/// tables after the tables' own (and the building variants), the files in name order, so the
/// game's own types keep their ids.
pub const CARDS_DIR: &str = "cards";

/// Appends every card file's entries to their tables (see [`CARDS_DIR`]); returns the cards' text.
fn add_card_files(docs: &mut [(String, Value)], json_dir: &Path) -> Result<Vec<(String, String)>, LoadError> {
    let dir = json_dir.join(CARDS_DIR);
    let mut names: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| n.ends_with(".jsonc"))
            .collect(),
        Err(_) => return Ok(Vec::new()),
    };
    names.sort();
    let mut text = Vec::new();
    for name in names {
        let rel = format!("{CARDS_DIR}/{name}");
        let doc = read_doc(json_dir, &rel)?;
        let Value::Obj(members) = doc else {
            return Err(LoadError(format!("{rel}: expected an object of tables")));
        };
        for (key, val) in members {
            if key == "text" {
                let Value::Obj(lines) = val else {
                    return Err(LoadError(format!("{rel}: text must be an object")));
                };
                for (k, v) in lines {
                    let Some(v) = v.as_str() else {
                        return Err(LoadError(format!("{rel}: text {k} is not a string")));
                    };
                    text.push((k, v.to_string()));
                }
                continue;
            }
            let file = format!("{key}.jsonc");
            let Some((_, Value::Arr(entries))) = docs.iter_mut().find(|(f, _)| *f == file) else {
                return Err(LoadError(format!("{rel}: {key} is no info table")));
            };
            let Value::Arr(added) = val else {
                return Err(LoadError(format!("{rel}: {key} must be a list of entries")));
            };
            entries.extend(added);
        }
    }
    Ok(text)
}

/// How many entries the card files of the default data directory add to `table` (a table's file
/// name without `.jsonc`): what tests that count a table's entries subtract, to count the
/// game's own (docs/CARDS.md).
pub fn card_entry_count(table: &str) -> usize {
    let dir = default_data_dir().join("json").join(CARDS_DIR);
    let Ok(rd) = std::fs::read_dir(&dir) else { return 0 };
    rd.filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".jsonc"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|text| jsonc::parse(&text).ok())
        .map(|v| v.get(table).and_then(Value::as_arr).map_or(0, |a| a.len()))
        .sum()
}

fn read_doc(dir: &Path, rel: &str) -> Result<Value, LoadError> {
    let p = dir.join(rel);
    let text = std::fs::read_to_string(&p).map_err(|e| LoadError(format!("{}: {}", p.display(), e)))?;
    jsonc::parse(&text).map_err(|e| LoadError(format!("{}: {}", p.display(), e)))
}

impl GameData {
    /// Loads everything from a data directory containing `json/` and `text/`.
    pub fn load(data_dir: &Path) -> Result<GameData, LoadError> {
        let json_dir = data_dir.join("json");
        let mut docs = Vec::new();
        for (file, _) in INFO_FILES {
            let doc = read_doc(&json_dir, file)?;
            if doc.as_arr().is_none() {
                return Err(LoadError(format!("{file}: expected a list of entries")));
            }
            docs.push((file.to_string(), doc));
        }
        add_building_variants(&mut docs, &json_dir)?;
        let card_text = add_card_files(&mut docs, &json_dir)?;
        let docs = Docs { docs };

        // Pass 1: register every type name.
        let mut reg = Registry::new();
        for (file, class) in INFO_FILES {
            let names = entries(docs.get(file)).iter().map(|e| e.get("type").and_then(Value::as_str).unwrap_or("").to_string()).collect();
            reg.register_class(class, names);
        }
        let gt = read_doc(&json_dir, "global_types.jsonc")?;
        for (list, class) in GLOBAL_TYPE_LISTS {
            if let Some(l) = gt.get(list).and_then(Value::as_arr) {
                let names = l.iter().map(|c| c.as_str().unwrap_or("").to_string()).collect();
                reg.register_class(class, names);
            }
        }

        // Defines (text defines resolve through the registry).
        let mut define_map = DefineMap::default();
        for f in DEFINES_FILES {
            define_map.read_file(&read_doc(&json_dir, f)?, &reg);
        }

        let mut text = match std::fs::read(data_dir.join("text").join("en.tsv")) {
            Ok(b) => Text::parse_tsv(&String::from_utf8_lossy(&b)),
            Err(_) => Text::default(),
        };
        for (k, v) in card_text {
            text.insert(&k, &v);
        }

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

    /// Fills the derived tables (not in the data files) from the loaded ones.
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
        self.tech_research_bonuses = IdxVec::from_elem(Vec::new(), self.techs.len());
        for i in self.improvements.keys() {
            let info = &self.improvements[i];
            if let Some(t) = info.tech_research_bonus.filter(|_| info.tech_research_percent > 0) {
                self.tech_research_bonuses[t].push((i, info.tech_research_percent));
            }
        }
        self.derive_building_tiers();
    }

    /// The building tiers' derived fields (docs/SUITABILITY.md): each family's buildings by tier,
    /// the Standard building's own tier, and the classes a city rolls a tier for.
    fn derive_building_tiers(&mut self) {
        let mut families: Vec<(BuildingType, Vec<Option<BuildingType>>)> = Vec::new();
        for b in self.buildings.keys() {
            let info = &self.buildings[b];
            let (Some(base), Some(tier)) = (info.variant_of, info.tier) else {
                if info.variant_of.is_some() != info.tier.is_some() {
                    self.warnings.push(format!("{}: variant_of and variant_tier go together", info.base.type_));
                }
                continue;
            };
            let i = match families.iter().position(|(f, _)| *f == base) {
                Some(i) => i,
                None => {
                    let mut tiers = vec![None; BuildingTierType::NUM];
                    tiers[BuildingTierType::STANDARD.idx()] = Some(base);
                    families.push((base, tiers));
                    families.len() - 1
                }
            };
            if families[i].1[tier.idx()].replace(b).is_some() {
                self.warnings.push(format!("{}: a second {:?} variant of {}", info.base.type_, tier, self.buildings[base].base.type_));
            }
        }
        for (base, tiers) in families {
            if tiers.iter().any(Option::is_none) {
                self.warnings.push(format!("{}: not every tier has a variant", self.buildings[base].base.type_));
                continue;
            }
            let list: Vec<BuildingType> = tiers.into_iter().flatten().collect();
            self.buildings[base].tier = Some(BuildingTierType::STANDARD);
            for &b in &list {
                if self.buildings[b].building_class_type != self.buildings[base].building_class_type {
                    self.warnings.push(format!("{}: a variant of another class", self.buildings[b].base.type_));
                }
                self.buildings[b].tier_variants = list.clone();
            }
        }
        for class in self.building_classes.keys() {
            let has_tiers = self.buildings.iter().any(|b| b.building_class_type == Some(class) && !b.tier_variants.is_empty());
            let info = &self.building_classes[class];
            let wonder = info.max_global_instances > 0 || info.max_team_instances > 0 || info.max_player_instances > 0;
            let buildable = info.default_building_index.is_some_and(|b| self.buildings[b].production_cost > 0 && self.buildings[b].victory_prereq.is_none());
            let victory = info.victory_threshold.iter().any(|&t| t > 0);
            self.building_classes[class].rolls_tier = has_tiers || (wonder && buildable && !victory);
        }
        let list = |f: &dyn Fn(&BuildingInfo) -> bool| self.buildings.keys().filter(|&b| f(&self.buildings[b])).collect::<Vec<_>>();
        self.unlocking_buildings = list(&|b| {
            b.unlocked_unit_experience != 0 || b.unlocked_unit_production_modifier != 0 || b.unlocked_unit_promotion.is_some()
        });
        self.population_commerce_buildings = list(&|b| b.population_commerce.iter().any(|&v| v != 0));
        self.sentinel_repair_buildings = list(&|b| b.sentinel_repair_change != 0);
        self.bonus_yield_buildings = list(&|b| b.bonus_yield_change.iter().any(|y| y.iter().any(|&v| v != 0)));
    }

    /// The building `u` needs (`prereq_building`, `prereq_building_class`) is `b`, at any tier
    /// (docs/SUITABILITY.md: `b` unlocks `u`).
    pub fn building_unlocks_unit(&self, b: BuildingType, u: UnitType) -> bool {
        let info = &self.units[u];
        info.prereq_building.is_some_and(|p| self.same_building(p, b))
            || (info.prereq_building_class.is_some() && info.prereq_building_class == self.buildings[b].building_class_type)
    }

    /// This game's building tiers (docs/SUITABILITY.md): the building of `b`'s family at `tier`
    /// (`b` itself for a building without tiers).
    pub fn building_variant(&self, b: BuildingType, tier: BuildingTierType) -> BuildingType {
        self.buildings[b].tier_variants.get(tier.idx()).copied().unwrap_or(b)
    }

    /// The Standard building of `b`'s family (`b` for a Standard building or one without tiers).
    pub fn building_family(&self, b: BuildingType) -> BuildingType {
        self.buildings[b].variant_of.unwrap_or(b)
    }

    /// The buildings of `b`'s family: its tier variants, or `b` alone for a building without
    /// tiers.
    pub fn building_family_members(&self, b: BuildingType) -> Vec<BuildingType> {
        let v = &self.buildings[b].tier_variants;
        if v.is_empty() {
            vec![b]
        } else {
            v.clone()
        }
    }

    /// `a` and `b` are the same building, at any tier.
    pub fn same_building(&self, a: BuildingType, b: BuildingType) -> bool {
        self.building_family(a) == self.building_family(b)
    }

    /// Engagement combat's checks the resolver relies on: every type that fights land battles has
    /// a stat sheet (docs/UNIT_STATS.md), rows and a rank preference (docs/ENGAGEMENTS.md).
    fn derive_engagements(&mut self) {
        let mut warnings = Vec::new();
        // A Wonder's outcomes are cast through it only (docs/ENGAGEMENTS.md, "Battle spells").
        let outcomes: Vec<SpellType> = self.spells.iter().flat_map(|s| s.battle_wonder.iter().map(|&(o, _)| o)).collect();
        for o in outcomes {
            self.spells[o].wonder_outcome = true;
            let info = &self.spells[o];
            if !info.battle_wonder.is_empty() || !info.battle_only {
                warnings.push(format!("{}: a battle_wonder outcome must be battle_only and no wonder itself", info.base.type_));
            }
        }
        for (u, info) in self.units.iter_enumerated() {
            if !self.type_engages_on_land(u) {
                continue;
            }
            if info.stats.is_none() {
                warnings.push(format!("{}: a land unit without stats", info.base.type_));
            }
            if info.engage_rows.is_none() {
                warnings.push(format!("{}: a land unit without engage_rows", info.base.type_));
            }
            if info.engage_row_order.is_none() {
                warnings.push(format!("{}: a land unit without engage_row_order", info.base.type_));
            }
        }
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

    /// The damage spell a unit casts in a battle (docs/ENGAGEMENTS.md, "Battle spells"): the
    /// `attack` of [`Self::engage_battle_spells`]; its prerequisites on the caster are
    /// `PromotionPrereq1` and `PromotionPrereq2` among its promotions, `UnitPrereq`,
    /// `UnitClassPrereq` and `UnitCombatPrereq` its type's, `ReligionPrereq` its `religion`.
    /// `None`: it knows none (a caster casts the magic bolt).
    pub fn engage_damage_spell(&self, unit_type: UnitType, has_promotion: impl Fn(PromotionType) -> bool, religion: Option<ReligionType>) -> Option<SpellType> {
        self.engage_battle_spells(unit_type, has_promotion, religion).attack
    }

    /// A damage spell's output in a battle, to choose a caster's strongest: its power times `100 +`
    /// what its pattern adds when every extra target is there (a splash and a VOLLEY, PIERCE or
    /// CLEAVE two, a SWEEP three, a TRAMPLE one, each at its percentage).
    pub fn battle_spell_output(&self, x: &SpellInfo) -> i64 {
        let d = &self.defines;
        let extra = match x.battle_pattern {
            None => 2 * d.engagement_splash_percent,
            Some(AttackPattern::Single) => 0,
            Some(AttackPattern::Volley) => 2 * d.engagement_volley_percent,
            Some(AttackPattern::Pierce) => 2 * d.engagement_pierce_percent,
            Some(AttackPattern::Sweep) => 3 * d.engagement_sweep_percent,
            Some(AttackPattern::Cleave) => 2 * d.engagement_cleave_percent,
            Some(AttackPattern::Trample) => d.engagement_trample_percent,
        };
        x.battle_power_or_default() as i64 * (100 + extra.max(0)) as i64
    }

    /// A caster's battle spells (docs/ENGAGEMENTS.md, "Battle spells"), of the spells whose
    /// promotion, unit, unit class, unit combat and religion prerequisites it meets (not a Wonder's
    /// outcomes): its damage spell (`iDamage > 0`, `battle_only`, a `battle_power` or a
    /// `battle_wonder`, and no heal or charm), a Wonder first, then the strongest by
    /// [`Self::battle_spell_output`], then `iDamage`, then the lower id; its heal (`battle_heal`),
    /// the strongest; its charms and hexes (`battle_charm`), all but those of a lower
    /// [`Self::spell_tier`] than its damage spell, the highest tier first, then by id.
    pub fn engage_battle_spells(&self, unit_type: UnitType, has_promotion: impl Fn(PromotionType) -> bool, religion: Option<ReligionType>) -> BattleSpells<SpellType> {
        let info = &self.units[unit_type];
        let mut out = BattleSpells::default();
        for s in self.spells.keys() {
            let spell = &self.spells[s];
            let meets = spell.promotion_prereq1.map_or(true, &has_promotion)
                && spell.promotion_prereq2.map_or(true, &has_promotion)
                && spell.unit_prereq.map_or(true, |t| t == unit_type)
                && spell.unit_class_prereq.map_or(true, |c| info.unit_class_type == Some(c))
                && spell.unit_combat_prereq.map_or(true, |c| info.unit_combat_type == Some(c))
                && spell.religion_prereq.map_or(true, |r| religion == Some(r));
            if !meets || spell.wonder_outcome {
                continue;
            }
            if spell.battle_charm.is_some() {
                out.charms.push(s);
            } else if spell.battle_heal > 0 {
                if out.heal.map_or(true, |b| spell.battle_heal > self.spells[b].battle_heal) {
                    out.heal = Some(s);
                }
            } else if spell.damage > 0 || spell.battle_only || spell.battle_power > 0 || !spell.battle_wonder.is_empty() {
                let key = |x: &SpellInfo| (!x.battle_wonder.is_empty(), self.battle_spell_output(x), x.damage);
                if out.attack.map_or(true, |b| key(spell) > key(&self.spells[b])) {
                    out.attack = Some(s);
                }
            }
        }
        // No charm of a lower sphere tier than its damage spell (an archmage with Firestorm does not
        // spend an action on Hold the Line); the strongest sphere's charms first, then the lower id.
        let floor = out.attack.map_or(0, |a| self.spell_tier(a));
        out.charms.retain(|&s| self.spell_tier(s) >= floor);
        out.charms.sort_by_key(|&s| (std::cmp::Reverse(self.spell_tier(s)), s));
        out
    }

    /// A spell's tier: the deepest of its promotion prerequisites in their lines (Fire III is 3:
    /// Fire II, then Fire I, below it), 0 without one.
    pub fn spell_tier(&self, s: SpellType) -> i32 {
        let info = &self.spells[s];
        [info.promotion_prereq1, info.promotion_prereq2].into_iter().flatten().map(|p| self.promotion_line_tier(p)).max().unwrap_or(0)
    }

    /// A promotion's place in its line: 1, plus 1 for each promotion below it (`prereq_promotion`).
    pub fn promotion_line_tier(&self, p: PromotionType) -> i32 {
        let mut tier = 1;
        let mut at = p;
        while let Some(below) = self.promotions[at].prereq_promotion {
            if tier >= 10 {
                break;
            }
            tier += 1;
            at = below;
        }
        tier
    }

    // A unit type's values in a formation battle: its stat sheet, its rows and its rank preference
    // (`stats`, `engage_rows`, `engage_row_order` in units.jsonc); nothing (0, no action) for a
    // type without them. The rules ask through `Game::unit_engage_*`.

    /// A unit type's stat `s` (its sheet's; 0 for a type without one).
    pub fn unit_stat(&self, t: UnitType, s: StatType) -> i32 {
        self.units[t].stats.map_or(0, |sheet| sheet[s.idx()])
    }

    /// What a unit type does from each row, front first, before the extra actions: its
    /// `engage_rows` (no action from any row for a type without them).
    pub fn engage_rows(&self, t: UnitType) -> [RowActions; ROWS] {
        self.units[t].engage_rows.unwrap_or_default()
    }

    /// A unit type's attack pattern from each row (`engage_rows`' `pattern`; docs/UNIT_ROLES.md).
    pub fn engage_patterns(&self, t: UnitType) -> [AttackPattern; ROWS] {
        self.units[t].engage_patterns
    }

    /// A unit type's rank preference: the rows in the order the auto formation tries them
    /// (`engage_row_order`; front, middle, rear for a type without one).
    pub fn engage_row_order(&self, t: UnitType) -> [usize; ROWS] {
        self.units[t].engage_row_order.unwrap_or([0, 1, 2])
    }

    /// A unit type's kind: what it does from the row it prefers ([`rows_kind`]), which the auto
    /// formation sorts by.
    pub fn engage_kind(&self, t: UnitType) -> ActionKind {
        rows_kind(&self.engage_rows(t), &self.engage_row_order(t))
    }

    /// A unit type's best row: the one of its rows ([`Self::engage_rows`]) with the most actions
    /// (the first such), where a tier 4 unit or a hero gets its extra actions.
    pub fn engage_best_row(&self, t: UnitType) -> usize {
        rows_best_row(&self.engage_rows(t))
    }

    /// The extra actions in its best row of a unit of type `t`: `ENGAGEMENT_EXTRA_ACTIONS` for a
    /// type of tier `ENGAGEMENT_EXTRA_ACTION_TIER` or above and for a hero (`hero`: the unit has
    /// the Hero promotion), else 0.
    pub fn engage_extra_actions(&self, t: UnitType, hero: bool) -> i32 {
        if hero || self.units[t].tier >= self.defines.engagement_extra_action_tier {
            self.defines.engagement_extra_actions
        } else {
            0
        }
    }

    /// What a unit of type `t` does from each row in a battle: its rows ([`Self::engage_rows`])
    /// with its extra actions ([`Self::engage_extra_actions`]) in its best row.
    pub fn engage_actions(&self, t: UnitType, hero: bool) -> [RowActions; ROWS] {
        rows_with_extra_actions(self.engage_rows(t), self.engage_extra_actions(t, hero))
    }

    /// A flanker: one of a unit type's rows ([`Self::engage_rows`]) charges, so anti-flank units
    /// in its reach are taken first.
    pub fn engage_flanks(&self, t: UnitType) -> bool {
        rows_charge(&self.engage_rows(t))
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
        check(w, &self.reg, "BuildingTierInfo", BuildingTierType::NAMES);
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
