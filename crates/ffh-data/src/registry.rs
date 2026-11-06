//! Global type-string registry (C++ `GC.getInfoTypeForString`) and the data reader [`Rd`].
//!
//! Loading is two-pass: pass 1 registers every entry's `type` in every data file (so forward
//! references resolve without Civ4's readPass2/readPass3 hacks), pass 2 runs the per-class `read`
//! functions through [`Rd`].

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::enums::*;
use crate::infos::building::BuildingTierType;
use crate::infos::{CulturalActionType, GovernorNameType, GovernorProfileType, GovernorTraitType};
use crate::idx::{Idx, IdxVec};
use crate::jsonc::Value;

/// Associates a typed index with the registry class it indexes.
pub trait InfoId: Idx {
    /// Registry class name (the XML entry element name, e.g. "UnitInfo",
    /// or a GlobalTypes list name, e.g. "FlavorTypes").
    const CLASS: &'static str;
}

macro_rules! info_ids {
    ($($t:ty => $c:literal),* $(,)?) => { $(impl InfoId for $t { const CLASS: &'static str = $c; })* };
}

info_ids! {
    AlignmentType => "AlignmentInfo",
    DamageType => "DamageInfo",
    InvisibleType => "InvisibleInfo",
    MemoryType => "MemoryInfo",
    UnitAIType => "UnitAIInfo",
    UnitCombatType => "UnitCombatInfo",
    AttitudeType => "AttitudeInfo",
    DenialType => "DenialInfo",
    DomainType => "DomainInfo",
    CalendarType => "CalendarInfo",
    SeasonType => "SeasonInfo",
    MonthType => "MonthInfo",
    BuildingClassType => "BuildingClassInfo",
    BuildingType => "BuildingInfo",
    SpecialBuildingType => "SpecialBuildingInfo",
    CivilizationType => "CivilizationInfo",
    LeaderHeadType => "LeaderHeadInfo",
    DiploCommentType => "DiplomacyInfo",
    TraitType => "TraitInfo",
    EventType => "EventInfo",
    EventTriggerType => "EventTriggerInfo",
    CivicType => "CivicInfo",
    CivicOptionType => "CivicOptionInfo",
    CommerceType => "CommerceInfo",
    CorporationType => "CorporationInfo",
    CultureLevelType => "CultureLevelInfo",
    EmphasizeType => "EmphasizeInfo",
    EraType => "EraInfo",
    EspionageMissionType => "EspionageMissionInfo",
    GameOptionType => "GameOptionInfo",
    GameSpeedType => "GameSpeedInfo",
    GoodyType => "GoodyInfo",
    HandicapType => "HandicapInfo",
    ProcessType => "ProcessInfo",
    ProjectType => "ProjectInfo",
    ReligionType => "ReligionInfo",
    SpecialistType => "SpecialistInfo",
    VictoryType => "VictoryInfo",
    VoteType => "VoteInfo",
    VoteSourceType => "VoteSourceInfo",
    WorldSizeType => "WorldInfo",
    HurryType => "HurryInfo",
    UpkeepType => "UpkeepInfo",
    ClimateType => "ClimateInfo",
    SeaLevelType => "SeaLevelInfo",
    RouteType => "RouteInfo",
    TechType => "TechInfo",
    BonusClassType => "BonusClassInfo",
    BonusType => "BonusInfo",
    FeatureType => "FeatureInfo",
    ImprovementType => "ImprovementInfo",
    TerrainType => "TerrainInfo",
    YieldType => "YieldInfo",
    AutomateType => "AutomateInfo",
    BuildType => "BuildInfo",
    CommandType => "CommandInfo",
    MissionType => "MissionInfo",
    PromotionType => "PromotionInfo",
    SpecialUnitType => "SpecialUnitInfo",
    SpellType => "SpellInfo",
    UnitClassType => "UnitClassInfo",
    UnitType => "UnitInfo",
    FlavorType => "FlavorTypes",
    ArtStyleType => "ArtStyleTypes",
    CitySizeType => "CitySizeTypes",
    ContactType => "ContactTypes",
    DiplomacyPowerType => "DiplomacyPowerTypes",
    DirectionType => "DirectionTypes",
    CulturalActionType => "CulturalActionInfo",
    GovernorProfileType => "GovernorProfileInfo",
    GovernorTraitType => "GovernorTraitInfo",
    GovernorNameType => "GovernorNameInfo",
    BuildingTierType => "BuildingTierInfo",
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegEntry {
    pub class: &'static str,
    pub index: u16,
}

#[derive(Default, Debug)]
pub struct Registry {
    map: HashMap<String, RegEntry>,
    /// per class: ordered type names
    names: HashMap<&'static str, Vec<String>>,
    /// duplicate registrations (name, first class, second class)
    pub duplicates: Vec<(String, &'static str, &'static str)>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the types of a class in order. Returns the count.
    pub fn register_class(&mut self, class: &'static str, types: Vec<String>) -> usize {
        for (i, t) in types.iter().enumerate() {
            if t.is_empty() {
                continue;
            }
            let e = RegEntry { class, index: i as u16 };
            if let Some(prev) = self.map.insert(t.clone(), e) {
                if prev.class != class || prev.index != e.index {
                    self.duplicates.push((t.clone(), prev.class, class));
                }
            }
        }
        let n = types.len();
        self.names.insert(class, types);
        n
    }

    /// C++ `GC.getInfoTypeForString`: index or -1. Class-agnostic.
    pub fn info_type_for_string(&self, s: &str) -> i32 {
        self.map.get(s).map(|e| e.index as i32).unwrap_or(-1)
    }

    pub fn entry(&self, s: &str) -> Option<RegEntry> {
        self.map.get(s).copied()
    }

    /// Typed lookup; `None` if missing or registered under another class.
    pub fn lookup<T: InfoId>(&self, s: &str) -> Option<T> {
        match self.map.get(s) {
            Some(e) if e.class == T::CLASS => Some(T::from_usize(e.index as usize)),
            _ => None,
        }
    }

    pub fn count<T: InfoId>(&self) -> usize {
        self.names.get(T::CLASS).map(|v| v.len()).unwrap_or(0)
    }

    pub fn count_class(&self, class: &str) -> usize {
        self.names.get(class).map(|v| v.len()).unwrap_or(0)
    }

    pub fn type_name<T: InfoId>(&self, t: T) -> &str {
        self.names.get(T::CLASS).and_then(|v| v.get(t.idx())).map(|s| s.as_str()).unwrap_or("?")
    }

    /// Every registered class, sorted.
    pub fn classes(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self.names.keys().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn names_of(&self, class: &str) -> &[String] {
        self.names.get(class).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Load context shared by all readers of one file.
pub struct Ctx<'r> {
    pub reg: &'r Registry,
    pub file: String,
    pub warnings: RefCell<Vec<String>>,
    /// The entry being read (its `type`), for messages.
    entry: RefCell<String>,
    /// The values a reader looked up by key (their addresses): the members nobody read are
    /// reported (`Rd::check_unread`).
    touched: RefCell<HashSet<usize>>,
}

impl<'r> Ctx<'r> {
    pub fn new(reg: &'r Registry, file: impl Into<String>) -> Self {
        Ctx {
            reg,
            file: file.into(),
            warnings: RefCell::new(Vec::new()),
            entry: RefCell::new(String::new()),
            touched: RefCell::new(HashSet::new()),
        }
    }

    pub fn warn(&self, msg: String) {
        let entry = self.entry.borrow();
        let at = if entry.is_empty() { String::new() } else { format!(" {entry}:") };
        self.warnings.borrow_mut().push(format!("{}:{at} {}", self.file, msg));
    }

    /// Names the entry read next, for messages.
    pub fn set_entry(&self, name: &str) {
        *self.entry.borrow_mut() = name.to_string();
    }

    fn touch(&self, v: &Value) {
        self.touched.borrow_mut().insert(v as *const Value as usize);
    }

    fn touched(&self, v: &Value) -> bool {
        self.touched.borrow().contains(&(v as *const Value as usize))
    }
}

/// Reader over one value of a data file (an entry, or a record or list inside one). Each method
/// reads the member `key` of an object as one kind of value and answers its default when the
/// key is absent; a value of the wrong kind warns and answers the default too.
#[derive(Clone, Copy)]
pub struct Rd<'a> {
    pub v: &'a Value,
    pub cx: &'a Ctx<'a>,
}

impl<'a> Rd<'a> {
    pub fn new(v: &'a Value, cx: &'a Ctx<'a>) -> Self {
        Rd { v, cx }
    }

    pub fn reg(&self) -> &'a Registry {
        self.cx.reg
    }

    /// The member `key`, marked as read.
    fn get(&self, key: &str) -> Option<&'a Value> {
        let v: &'a Value = self.v;
        let m = v.get(key)?;
        self.cx.touch(m);
        Some(m)
    }

    fn wrong(&self, key: &str, expected: &str, found: &Value) {
        self.cx.warn(format!("{key:?} should be {expected}, not {}", found.kind()));
    }

    /// The member `key` as a reader of its own (a record or a list).
    pub fn child(&self, key: &str) -> Option<Rd<'a>> {
        self.get(key).map(|v| Rd { v, cx: self.cx })
    }

    /// The items of a list (or the values of an object), each as a reader.
    pub fn children(&self) -> std::vec::IntoIter<Rd<'a>> {
        let v: &'a Value = self.v;
        let all: Vec<Rd<'a>> = match v {
            Value::Arr(items) => items.iter().map(|v| Rd { v, cx: self.cx }).collect(),
            Value::Obj(members) => members.iter().map(|(_, v)| Rd { v, cx: self.cx }).collect(),
            _ => Vec::new(),
        };
        all.into_iter()
    }

    /// This value as a string ("" for another kind).
    pub fn text(&self) -> &'a str {
        let v: &'a Value = self.v;
        v.as_str().unwrap_or("")
    }

    /// This value as text: a string as it is, a number or flag written out, "" for the rest.
    pub fn scalar_text(&self) -> std::borrow::Cow<'a, str> {
        let v: &'a Value = self.v;
        match v {
            Value::Str(s) => std::borrow::Cow::Borrowed(s.as_str()),
            Value::Int(i) => std::borrow::Cow::Owned(i.to_string()),
            Value::Float(f) => std::borrow::Cow::Owned(f.to_string()),
            Value::Bool(b) => std::borrow::Cow::Borrowed(if *b { "1" } else { "0" }),
            _ => std::borrow::Cow::Borrowed(""),
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.v.get(key).is_some()
    }

    // ---- single values ----

    pub fn int(&self, key: &str) -> i32 {
        self.int_or(key, 0)
    }

    pub fn int_or(&self, key: &str, def: i32) -> i32 {
        match self.get(key) {
            None => def,
            Some(Value::Int(i)) if i32::try_from(*i).is_ok() => *i as i32,
            Some(v) => {
                self.wrong(key, "an integer", v);
                def
            }
        }
    }

    pub fn bool(&self, key: &str) -> bool {
        self.bool_or(key, false)
    }

    pub fn bool_or(&self, key: &str, def: bool) -> bool {
        match self.get(key) {
            None => def,
            Some(Value::Bool(b)) => *b,
            Some(v) => {
                self.wrong(key, "true or false", v);
                def
            }
        }
    }

    pub fn float(&self, key: &str) -> f32 {
        self.float_or(key, 0.0)
    }

    pub fn float_or(&self, key: &str, def: f32) -> f32 {
        match self.get(key) {
            None => def,
            Some(v) => match v.as_f64() {
                Some(f) => f as f32,
                None => {
                    self.wrong(key, "a number", v);
                    def
                }
            },
        }
    }

    /// A string ("" if absent).
    pub fn string(&self, key: &str) -> String {
        self.string_or(key, "")
    }

    pub fn string_or(&self, key: &str, def: &str) -> String {
        match self.get(key) {
            None => def.to_string(),
            Some(Value::Str(s)) => s.clone(),
            Some(v) => {
                self.wrong(key, "a string", v);
                def.to_string()
            }
        }
    }

    // ---- type references ----

    /// Class-agnostic index or -1 (C++ `FindInInfoClass`). Warns on unknown non-empty names.
    pub fn find_in_info_class(&self, s: &str) -> i32 {
        let idx = self.cx.reg.info_type_for_string(s);
        if idx < 0 && !s.is_empty() && s != "NONE" {
            self.cx.warn(format!("unknown type {:?}", s));
        }
        idx
    }

    /// Resolves a type name to a typed index; warns if unknown or of another class.
    pub fn resolve<T: InfoId>(&self, s: &str) -> Option<T> {
        let s = s.trim();
        if s.is_empty() || s == "NONE" {
            return None;
        }
        match self.cx.reg.entry(s) {
            Some(e) if e.class == T::CLASS => Some(T::from_usize(e.index as usize)),
            Some(e) => {
                self.cx.warn(format!("type {:?} is a {} but a {} was expected", s, e.class, T::CLASS));
                None
            }
            None => {
                self.cx.warn(format!("unknown type {:?} (expected {})", s, T::CLASS));
                None
            }
        }
    }

    /// A type named by the string `key` (`None` when absent).
    pub fn rf<T: InfoId>(&self, key: &str) -> Option<T> {
        match self.get(key) {
            None | Some(Value::Null) => None,
            Some(Value::Str(s)) => self.resolve::<T>(s),
            Some(v) => {
                self.wrong(key, "a type name", v);
                None
            }
        }
    }

    // ---- lists ----

    /// The list `key` (empty when absent; a value that is no list warns).
    fn items(&self, key: &str) -> &'a [Value] {
        match self.get(key) {
            None => &[],
            Some(Value::Arr(items)) => items,
            Some(v) => {
                self.wrong(key, "a list", v);
                &[]
            }
        }
    }

    /// The object `key` (empty when absent; a value that is no object warns).
    fn members(&self, key: &str) -> &'a [(String, Value)] {
        match self.get(key) {
            None => &[],
            Some(Value::Obj(m)) => m,
            Some(v) => {
                self.wrong(key, "an object", v);
                &[]
            }
        }
    }

    /// A list of `n` integers by position (yields, commerces), zeros for the missing.
    pub fn positional_ints(&self, key: &str, n: usize) -> Vec<i32> {
        let mut v = vec![0; n];
        let items = self.items(key);
        if items.len() > n {
            self.cx.warn(format!("{key:?} has more than {n} values"));
        }
        for (slot, x) in v.iter_mut().zip(items) {
            match x {
                Value::Int(i) => *slot = *i as i32,
                other => self.wrong(key, "a list of integers", other),
            }
        }
        v
    }

    pub fn yields(&self, key: &str) -> [i32; YieldType::NUM] {
        let v = self.positional_ints(key, YieldType::NUM);
        [v[0], v[1], v[2]]
    }

    pub fn commerces(&self, key: &str) -> [i32; CommerceType::NUM] {
        let v = self.positional_ints(key, CommerceType::NUM);
        [v[0], v[1], v[2], v[3]]
    }

    /// A list of `n` flags by position, false for the missing.
    pub fn positional_bools(&self, key: &str, n: usize) -> Vec<bool> {
        let mut v = vec![false; n];
        for (slot, x) in v.iter_mut().zip(self.items(key)) {
            match x {
                Value::Bool(b) => *slot = *b,
                other => self.wrong(key, "a list of true/false", other),
            }
        }
        v
    }

    /// A list of type names.
    pub fn ref_list<T: InfoId>(&self, key: &str) -> Vec<T> {
        self.items(key).iter().filter_map(|x| self.item_ref::<T>(key, x)).collect()
    }

    /// A list of type names by position, `null` for none, `n` long.
    pub fn ref_list_fixed<T: InfoId>(&self, key: &str, n: usize) -> Vec<Option<T>> {
        let items = self.items(key);
        if items.len() > n {
            self.cx.warn(format!("{key:?} has more than {n} entries"));
        }
        let mut out = vec![None; n];
        for (slot, x) in out.iter_mut().zip(items) {
            *slot = self.item_ref::<T>(key, x);
        }
        out
    }

    /// A list of type names (the old comma-separated tags).
    pub fn ref_tokens<T: InfoId>(&self, key: &str) -> Vec<T> {
        self.ref_list(key)
    }

    /// A list of strings.
    pub fn string_list(&self, key: &str) -> Vec<String> {
        self.items(key)
            .iter()
            .filter_map(|x| match x {
                Value::Str(s) => Some(s.clone()),
                other => {
                    self.wrong(key, "a list of strings", other);
                    None
                }
            })
            .collect()
    }

    fn item_ref<T: InfoId>(&self, key: &str, x: &Value) -> Option<T> {
        match x {
            Value::Null => None,
            Value::Str(s) => self.resolve::<T>(s),
            other => {
                self.wrong(key, "a list of type names", other);
                None
            }
        }
    }

    // ---- per-type values: an object from type names to values ----

    fn per_type<K: InfoId, T: Clone>(&self, key: &str, def: T, val: impl Fn(&Value) -> Option<T>, expected: &str) -> IdxVec<K, T> {
        let mut v = IdxVec::from_elem(def, self.cx.reg.count::<K>());
        for (name, x) in self.members(key) {
            if let Some(k) = self.resolve::<K>(name) {
                match val(x) {
                    Some(t) => v[k] = t,
                    None => self.wrong(key, expected, x),
                }
            }
        }
        v
    }

    pub fn pairs_int<K: InfoId>(&self, key: &str, def: i32) -> IdxVec<K, i32> {
        self.per_type(key, def, |x| x.as_i64().map(|i| i as i32), "an object of integers")
    }

    pub fn pairs_bool<K: InfoId>(&self, key: &str) -> IdxVec<K, bool> {
        self.pairs_bool_or::<K>(key, false)
    }

    pub fn pairs_bool_or<K: InfoId>(&self, key: &str, def: bool) -> IdxVec<K, bool> {
        self.per_type(key, def, |x| x.as_bool(), "an object of true/false")
    }

    pub fn pairs_float<K: InfoId>(&self, key: &str, def: f32) -> IdxVec<K, f32> {
        self.per_type(key, def, |x| x.as_f64().map(|f| f as f32), "an object of numbers")
    }

    pub fn pairs_string<K: InfoId>(&self, key: &str) -> IdxVec<K, String> {
        self.per_type(key, String::new(), |x| x.as_str().map(str::to_string), "an object of strings")
    }

    /// An object from type names to type names.
    pub fn pairs_ref<K: InfoId, V: InfoId>(&self, key: &str) -> IdxVec<K, Option<V>> {
        let mut v = IdxVec::from_elem(None, self.cx.reg.count::<K>());
        for (name, x) in self.members(key) {
            if let Some(k) = self.resolve::<K>(name) {
                v[k] = self.item_ref::<V>(key, x);
            }
        }
        v
    }

    /// An object from type names to integers, as (type, value) in file order.
    pub fn pair_list_int<K: InfoId>(&self, key: &str) -> Vec<(K, i32)> {
        let mut out = Vec::new();
        for (name, x) in self.members(key) {
            if let Some(k) = self.resolve::<K>(name) {
                match x.as_i64() {
                    Some(i) => out.push((k, i as i32)),
                    None => self.wrong(key, "an object of integers", x),
                }
            }
        }
        out
    }

    // ---- unknown keys ----

    /// Warns about every key of this entry that no reader looked up (a misspelt or obsolete
    /// key), and so in each record a reader looked into.
    pub fn check_unread(&self) {
        self.unread(self.v, "");
    }

    fn unread(&self, v: &Value, at: &str) {
        match v {
            Value::Obj(members) => {
                for (k, m) in members {
                    if !self.cx.touched(m) {
                        self.cx.warn(format!("unknown key {at}{k:?}: nothing reads it"));
                    } else if self.looked_into(m) {
                        self.unread(m, &format!("{at}{k}."));
                    } else if let Value::Arr(items) = m {
                        for (i, item) in items.iter().enumerate() {
                            if self.looked_into(item) {
                                self.unread(item, &format!("{at}{k}[{i}]."));
                            }
                        }
                    }
                }
            }
            Value::Arr(items) => {
                for (i, item) in items.iter().enumerate() {
                    if self.looked_into(item) {
                        self.unread(item, &format!("{at}[{i}]."));
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether a reader looked up some key of this object.
    fn looked_into(&self, v: &Value) -> bool {
        matches!(v, Value::Obj(members) if members.iter().any(|(_, m)| self.cx.touched(m)))
    }
}
