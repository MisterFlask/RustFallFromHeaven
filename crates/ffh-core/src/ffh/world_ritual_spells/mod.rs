//! The civilizations' world spells that replace FFH2's (docs/CARDS.md, "World rituals";
//! docs/PORTING.md, "Changes from FFH2"): one module per file, each registering its callbacks by
//! name in its `find_*` lookups (as `ffh/cards` does), listed once in [`MODULES`].

use crate::ffh::cards::Lookups;
use crate::ffh::dispatch::*;

pub mod illians;

/// Every module's lookups, searched in order, with its `py_target_label` lookup.
const MODULES: &[(Lookups, fn(&str) -> Option<TargetLabelFn>)] = &[
    (Lookups { spell: illians::find_spell, req: illians::find_req, targets: illians::find_targets, unit_fn: none }, illians::find_target_label),
];

fn none<T>(_: &str) -> Option<T> {
    None
}

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    MODULES.iter().find_map(|(m, _)| (m.spell)(name))
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    MODULES.iter().find_map(|(m, _)| (m.req)(name))
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    MODULES.iter().find_map(|(m, _)| (m.targets)(name))
}

/// Unit callbacks (promotion `PyPerTurn`, spell `PyMiscast`).
pub fn find_unit_fn(name: &str) -> Option<UnitFn> {
    MODULES.iter().find_map(|(m, _)| (m.unit_fn)(name))
}

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    MODULES.iter().find_map(|(_, l)| l(name))
}
