//! The civilizations' world spells as the world rituals rework them (docs/CARDS.md, "World
//! rituals"): the `PyResult`, `PyRequirement`, `py_targets` and `py_target_label` callbacks their
//! data names. One module per batch, each registering its functions by name in its `find_*`
//! lookups; a batch adds its module to [`MODULES`].

use crate::ffh::dispatch::*;

pub mod lanun_khazad;

/// The lookups of one module: `find_spell`, `find_req`, `find_targets`, `find_target_label`.
pub struct Lookups {
    pub spell: fn(&str) -> Option<SpellFn>,
    pub req: fn(&str) -> Option<ReqFn>,
    pub targets: fn(&str) -> Option<TargetsFn>,
    pub label: fn(&str) -> Option<TargetLabelFn>,
}

/// Every module's lookups, searched in order.
const MODULES: &[Lookups] = &[
    Lookups { spell: lanun_khazad::find_spell, req: lanun_khazad::find_req, targets: lanun_khazad::find_targets, label: lanun_khazad::find_target_label },
];

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    MODULES.iter().find_map(|m| (m.spell)(name))
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    MODULES.iter().find_map(|m| (m.req)(name))
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    MODULES.iter().find_map(|m| (m.targets)(name))
}

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    MODULES.iter().find_map(|m| (m.label)(name))
}
