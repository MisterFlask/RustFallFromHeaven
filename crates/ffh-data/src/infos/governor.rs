//! This game's own info classes for governors (not FFH2's; docs/GOVERNORS.md):
//! `city_roles.jsonc`, `governor_traits.jsonc`, `governor_names.jsonc`.

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::idx::IdxVec;
use crate::registry::Rd;

define_idx!(/// A governed city's role (`ROLE_*`).
CityRoleType);
define_idx!(/// A governor trait (`GOVERNORTRAIT_*`).
GovernorTraitType);
define_idx!(/// A list of governor names (`GOVERNORNAMES_*`).
GovernorNameType);

/// What a governed city is run for (this game's `CityRoleInfo`).
#[derive(Debug, Clone, Default)]
pub struct CityRoleInfo {
    pub base: InfoBase,
    /// Weights over the city AI's flavors (0-10).
    pub flavor_value: IdxVec<FlavorType, i32>,
    /// The citizen emphases the role sets in its city.
    pub emphasizes: Vec<EmphasizeType>,
    /// Added to the city's garrison quota.
    pub defenders: i32,
    /// The generic building classes that carry the role: a city's suitability for it is its
    /// tiers' core effect for them (`Game::city_role_suitability`).
    pub key_buildings: Vec<BuildingClassType>,
}

pub fn read_city_role(rd: &Rd) -> CityRoleInfo {
    CityRoleInfo {
        base: InfoBase::read(rd),
        flavor_value: rd.pairs_int("flavors", 0),
        emphasizes: rd.ref_list("emphasizes"),
        defenders: rd.int("defenders"),
        key_buildings: rd.ref_list("key_buildings"),
    }
}

/// How a governor judges (this game's `GovernorTraitInfo`): never a modifier of its city.
#[derive(Debug, Clone, Default)]
pub struct GovernorTraitInfo {
    pub base: InfoBase,
    /// Added to its city's garrison quota.
    pub defenders: i32,
    /// It prefers the costliest building it may build.
    pub ambitious: bool,
}

pub fn read_governor_trait(rd: &Rd) -> GovernorTraitInfo {
    GovernorTraitInfo { base: InfoBase::read(rd), defenders: rd.int("defenders"), ambitious: rd.bool("ambitious") }
}

/// Governor names of a civilization, or the generic list (no `Civilization`).
#[derive(Debug, Clone, Default)]
pub struct GovernorNameInfo {
    pub base: InfoBase,
    pub civilization: Option<CivilizationType>,
    pub names: Vec<String>,
}

pub fn read_governor_names(rd: &Rd) -> GovernorNameInfo {
    GovernorNameInfo { base: InfoBase::read(rd), civilization: rd.rf("civilization"), names: rd.string_list("names") }
}
