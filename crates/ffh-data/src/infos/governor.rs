//! This game's own info classes for governors (not FFH2's; docs/GOVERNORS.md):
//! `Gameinfo/CIV4GovernorProfileInfos.xml`, `CIV4GovernorTraitInfos.xml`, `CIV4GovernorNameInfos.xml`.

use super::InfoBase;
use crate::define_idx;
use crate::enums::*;
use crate::idx::IdxVec;
use crate::registry::Rd;

define_idx!(/// A governor's priority profile (`GOVERNORPROFILE_*`).
GovernorProfileType);
define_idx!(/// A governor trait (`GOVERNORTRAIT_*`).
GovernorTraitType);
define_idx!(/// A list of governor names (`GOVERNORNAMES_*`).
GovernorNameType);

/// What a governor leans toward (this game's `GovernorProfileInfo`).
#[derive(Debug, Clone, Default)]
pub struct GovernorProfileInfo {
    pub base: InfoBase,
    /// The neutral profile of a city without a governor; never drawn for the bench.
    pub interim: bool,
    /// Weights over the city AI's flavors (0-10).
    pub flavor_value: IdxVec<FlavorType, i32>,
    /// The citizen emphases the governor sets in its city.
    pub emphasizes: Vec<EmphasizeType>,
    /// Added to the city's garrison quota.
    pub defenders: i32,
}

pub fn read_governor_profile(rd: &Rd) -> GovernorProfileInfo {
    GovernorProfileInfo {
        base: InfoBase::read(rd),
        interim: rd.bool("bInterim"),
        flavor_value: rd.pairs_int("Flavors", 0),
        emphasizes: rd.ref_list("Emphasizes"),
        defenders: rd.int("iDefenders"),
    }
}

/// A small modifier of the city a governor holds (this game's `GovernorTraitInfo`).
#[derive(Debug, Clone, Default)]
pub struct GovernorTraitInfo {
    pub base: InfoBase,
    pub building_production_percent: i32,
    pub unit_production_percent: i32,
    pub happy: i32,
    pub health: i32,
    /// Experience of the units trained in the city.
    pub experience: i32,
    pub yield_modifier: [i32; YieldType::NUM],
    pub commerce_modifier: [i32; CommerceType::NUM],
    /// Leaders with these flavors draw the trait more often.
    pub flavor_value: IdxVec<FlavorType, i32>,
}

pub fn read_governor_trait(rd: &Rd) -> GovernorTraitInfo {
    GovernorTraitInfo {
        base: InfoBase::read(rd),
        building_production_percent: rd.int("iBuildingProductionPercent"),
        unit_production_percent: rd.int("iUnitProductionPercent"),
        happy: rd.int("iHappy"),
        health: rd.int("iHealth"),
        experience: rd.int("iExperience"),
        yield_modifier: rd.yields("YieldModifiers"),
        commerce_modifier: rd.commerces("CommerceModifiers"),
        flavor_value: rd.pairs_int("Flavors", 0),
    }
}

/// Governor names of a civilization, or the generic list (no `Civilization`).
#[derive(Debug, Clone, Default)]
pub struct GovernorNameInfo {
    pub base: InfoBase,
    pub civilization: Option<CivilizationType>,
    pub names: Vec<String>,
}

pub fn read_governor_names(rd: &Rd) -> GovernorNameInfo {
    GovernorNameInfo { base: InfoBase::read(rd), civilization: rd.rf("Civilization"), names: rd.string_list("Names") }
}
