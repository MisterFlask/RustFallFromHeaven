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
            /// Derived at load (not in the data files): the engagement role of each unit type
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
                GameData { reg, define_map, defines, text, barks: String::new(), warnings, $( $field, )* combat_mod_promotions: Vec::new(), unit_roles: IdxVec::new(), screen_role: None }
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
                out.push(("unit_roles".to_string(), format!("{:?}", self.unit_roles)));
                out.push(("screen_role".to_string(), format!("{:?}", self.screen_role)));
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
    combat_roles: CombatRoleType => CombatRoleInfo, engagement::read_combat_role;
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
    }

    /// Engagement combat's derived tables (`unit_roles`, `screen_role`), and the checks the
    /// resolver relies on (one default role, positive multipliers and HP,
    /// rows that act exactly when they have an action, one auto-pick place per role).
    fn derive_engagements(&mut self) {
        let mut warnings = Vec::new();
        let defaults: Vec<CombatRoleType> = self.combat_roles.keys().filter(|&r| self.combat_roles[r].default).collect();
        if defaults.len() != 1 {
            warnings.push(format!("{} combat roles have bDefault (one expected)", defaults.len()));
        }
        for r in self.combat_roles.keys() {
            let info = &self.combat_roles[r];
            // A row acts iff it has an action kind; a role acts from some row (a unit type's own
            // EngageRows are checked so as they are read).
            if let Some(problem) = rows_problem(&info.rows) {
                warnings.push(format!("{}: {problem} (iActions)", info.base.type_));
            }
            if info.from_role.is_some() != info.from_promotion.is_some() {
                warnings.push(format!("{}: FromRole and FromPromotion go together", info.base.type_));
            }
        }
        let promoted: Vec<CombatRoleType> = self.combat_roles.keys().filter(|&r| self.combat_roles[r].from_role.is_some()).collect();
        let mut picks: Vec<i32> = self.combat_roles.iter().map(|info| info.auto_pick).collect();
        picks.sort_unstable();
        picks.dedup();
        if picks.len() != self.combat_roles.len() {
            warnings.push("two combat roles have the same iAutoPick".to_string());
        }
        if promoted.len() > 1 {
            warnings.push(format!("{} combat roles have a FromRole (at most one expected)", promoted.len()));
        }
        self.screen_role = promoted.first().and_then(|&to| {
            let info = &self.combat_roles[to];
            Some((info.from_role?, info.from_promotion?, to))
        });
        self.unit_roles =
            IdxVec::from_fn(self.units.len(), |u| if self.type_engages_on_land(u) { self.type_combat_role(u) } else { None });
        // Every type that fights land battles has a stat sheet (docs/UNIT_STATS.md).
        for (u, info) in self.units.iter_enumerated() {
            if self.unit_roles[u].is_some() && info.stats.is_none() {
                warnings.push(format!("{}: a land unit without stats", info.base.type_));
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

    /// The damage spell a unit casts in a formation battle (docs/ENGAGEMENTS.md, "Action kinds
    /// and reach": SPELL): of the spells with `iDamage > 0` whose prerequisites on the caster it
    /// meets (`PromotionPrereq1` and `PromotionPrereq2` among its promotions, `UnitPrereq`,
    /// `UnitClassPrereq` and `UnitCombatPrereq` its type's), the strongest: the highest `iDamage`,
    /// then the lower id. `None`: it casts the magic bolt.
    pub fn engage_damage_spell(&self, unit_type: UnitType, has_promotion: impl Fn(PromotionType) -> bool) -> Option<SpellType> {
        let info = &self.units[unit_type];
        let mut best: Option<SpellType> = None;
        for s in self.spells.keys() {
            let spell = &self.spells[s];
            let meets = spell.damage > 0
                && spell.promotion_prereq1.map_or(true, &has_promotion)
                && spell.promotion_prereq2.map_or(true, &has_promotion)
                && spell.unit_prereq.map_or(true, |t| t == unit_type)
                && spell.unit_class_prereq.map_or(true, |c| info.unit_class_type == Some(c))
                && spell.unit_combat_prereq.map_or(true, |c| info.unit_combat_type == Some(c));
            if meets && best.map_or(true, |b| spell.damage > self.spells[b].damage) {
                best = Some(s);
            }
        }
        best
    }

    // A unit's values in a formation battle: each is its type's own (`iEngageSpeed`,
    // `iEngageDodge`, `iEngageCritBonus`, `EngageRows` in units.jsonc), else its role's
    // (`role`: the unit's own, which promotions can change: `Game::unit_combat_role`), else
    // nothing (0, no action) for a unit with neither. The rules ask through `Game::unit_engage_*`.

    /// A unit type's stat `s` (its sheet's; 0 for a type without one).
    pub fn unit_stat(&self, t: UnitType, s: StatType) -> i32 {
        self.units[t].stats.map_or(0, |sheet| sheet[s.idx()])
    }

    /// What a unit type does from each row, front first, before the extra actions: its
    /// `EngageRows`, else the role's rows.
    pub fn engage_rows(&self, t: UnitType, role: Option<CombatRoleType>) -> [RowActions; ROWS] {
        self.units[t].engage_rows.or_else(|| role.map(|r| self.combat_roles[r].rows)).unwrap_or_default()
    }

    /// A unit type's best row: the one of its rows ([`Self::engage_rows`]) with the most actions
    /// (the first such), where a tier 4 unit or a hero gets its extra actions.
    pub fn engage_best_row(&self, t: UnitType, role: Option<CombatRoleType>) -> usize {
        rows_best_row(&self.engage_rows(t, role))
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
    pub fn engage_actions(&self, t: UnitType, role: Option<CombatRoleType>, hero: bool) -> [RowActions; ROWS] {
        rows_with_extra_actions(self.engage_rows(t, role), self.engage_extra_actions(t, hero))
    }

    /// A flanker: one of a unit type's rows ([`Self::engage_rows`]) charges, so anti-flank units
    /// in its reach are taken first.
    pub fn engage_flanks(&self, t: UnitType, role: Option<CombatRoleType>) -> bool {
        rows_charge(&self.engage_rows(t, role))
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
