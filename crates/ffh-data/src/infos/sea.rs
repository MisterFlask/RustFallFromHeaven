//! This game's own info class for the sea (not FFH2's; docs/NAVAL.md, 4.1): `sea_names.jsonc`.

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;

define_idx!(/// A list of sea names (`SEANAMES_*`).
SeaNameType);

/// Names the map's seas draw from (docs/NAVAL.md, 4.1); `lake`: for a water area that is one sea
/// and touches no map edge.
#[derive(Debug, Clone, Default)]
pub struct SeaNameInfo {
    pub base: InfoBase,
    pub lake: bool,
    pub names: Vec<String>,
}

pub fn read_sea_names(rd: &Rd) -> SeaNameInfo {
    SeaNameInfo { base: InfoBase::read(rd), lake: rd.bool("lake"), names: rd.string_list("names") }
}
