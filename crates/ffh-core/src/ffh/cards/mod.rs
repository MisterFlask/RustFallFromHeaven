//! The rules callbacks of the great-person and Armageddon cards (docs/CARDS.md): the `PyResult`,
//! `PyRequirement`, `py_targets` and unit callbacks the cards' data names. One module per batch
//! of cards, each registering its functions by name in its `find_*` lookups (as `spells_a.rs`
//! does); `deck.rs` is the deck's own (discovering a card), `world.rs` the helpers the cards
//! share. A batch adds its module to [`MODULES`].

use crate::ffh::dispatch::*;

pub mod battle_magic;
pub mod deck;
pub mod world;

/// The lookups of one module: `find_spell`, `find_req`, `find_targets`, `find_unit_fn`.
pub struct Lookups {
    pub spell: fn(&str) -> Option<SpellFn>,
    pub req: fn(&str) -> Option<ReqFn>,
    pub targets: fn(&str) -> Option<TargetsFn>,
    pub unit_fn: fn(&str) -> Option<UnitFn>,
}

fn none<T>(_: &str) -> Option<T> {
    None
}

/// Every card module's lookups, searched in order.
const MODULES: &[Lookups] = &[
    Lookups { spell: deck::find_spell, req: deck::find_req, targets: none, unit_fn: none },
    Lookups { spell: world::find_spell, req: world::find_req, targets: world::find_targets, unit_fn: world::find_unit_fn },
    Lookups { spell: battle_magic::find_spell, req: battle_magic::find_req, targets: none, unit_fn: battle_magic::find_unit_fn },
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

/// Unit callbacks (promotion `PyPerTurn`, spell `PyMiscast`).
pub fn find_unit_fn(name: &str) -> Option<UnitFn> {
    MODULES.iter().find_map(|m| (m.unit_fn)(name))
}
