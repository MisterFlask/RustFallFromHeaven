//! The civilizations' world ritual spells and ritual effects that replace FFH2's world spells
//! (docs/CARDS.md, "World rituals"): the callbacks their data names (`py_result`,
//! `py_requirement`, `py_targets`, `py_target_label`) and the rules hooks of their rituals. One
//! module per batch; each lists its hooks in [`MODULES`].

use crate::ffh::dispatch::*;
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

pub mod lanun_khazad;
pub mod small;

/// The hooks of one module (`none` where it has none).
pub struct Hooks {
    pub spell: fn(&str) -> Option<SpellFn>,
    pub req: fn(&str) -> Option<ReqFn>,
    pub targets: fn(&str) -> Option<TargetsFn>,
    pub label: fn(&str) -> Option<TargetLabelFn>,
    /// `cannotCreate` for a world ritual (outside a visibility test only when it says so).
    pub cannot_create: fn(&Game, CityId, ProjectType, bool) -> bool,
    /// At the completion of a world ritual (after the framework's count and charge).
    pub project_built: fn(&mut Game, CityId, ProjectType),
    /// At the start of a player's turn (`player_do_turn`), and after a war or peace or a player's
    /// death (`ev_change_war`, `ev_set_player_alive`: `None`).
    pub check: fn(&mut Game, Option<PlayerId>),
}

/// Every module's hooks, in order.
const MODULES: &[Hooks] = &[
    Hooks { spell: lanun_khazad::find_spell, req: lanun_khazad::find_req, targets: lanun_khazad::find_targets, label: lanun_khazad::find_target_label, cannot_create: |_, _, _, _| false, project_built: |_, _, _| {}, check: |_, _| {} },
    small::HOOKS,
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

/// Can city `c` not create world ritual `project` (any module refuses)?
pub fn cannot_create(g: &Game, c: CityId, project: ProjectType, test_visible: bool) -> bool {
    MODULES.iter().any(|m| (m.cannot_create)(g, c, project, test_visible))
}

/// At the completion of world ritual `project` in city `c`.
pub fn project_built(g: &mut Game, c: CityId, project: ProjectType) {
    for m in MODULES {
        (m.project_built)(g, c, project);
    }
}

/// At the start of player `p`'s turn (`Some`), or after a war, a peace or a death (`None`).
pub fn check(g: &mut Game, p: Option<PlayerId>) {
    for m in MODULES {
        (m.check)(g, p);
    }
}
