//! The C++ selection-group path API on top of ffh-core's [`Game::group_find_path`].
//!
//! The C++ AI reads "the last path generated" back from the global path finder
//! (`getPathEndTurnPlot`, `getPathFirstPlot`) and resets it (`resetPath`); ffh-core returns the
//! path as a value instead and keeps no last path. This keeps the C++ calling convention for the
//! ported AI: the last path found through these methods is remembered here (per thread; the
//! simulation is single-threaded). `bReuse` is ignored: every search is fresh.

use std::cell::Cell;

use ffh_core::group::GroupPath;

use crate::prelude::*;

thread_local! {
    static LAST_PATH: Cell<Option<GroupPath>> = const { Cell::new(None) };
}

/// C++ `CvSelectionGroup` / `CvUnit` path calls with the last-path convention.
pub trait CppPaths {
    /// C++ `CvSelectionGroup::generatePath(pFromPlot, pToPlot, iFlags, bReuse, piPathTurns)`.
    fn group_generate_path(&self, g: GroupId, from: PlotIdx, to: PlotIdx, flags: i32, reuse: bool) -> Option<i32>;
    /// C++ `CvUnit::generatePath(pToPlot, iFlags, bReuse, piPathTurns)`.
    fn unit_generate_path(&self, u: UnitId, to: PlotIdx, flags: i32, reuse: bool, path_turns: Option<&mut i32>) -> bool;
    /// C++ `CvSelectionGroup::getPathFirstPlot`.
    fn group_get_path_first_plot(&self, g: GroupId) -> Option<PlotIdx>;
    /// C++ `CvSelectionGroup::getPathEndTurnPlot`.
    fn group_get_path_end_turn_plot(&self, g: GroupId) -> Option<PlotIdx>;
    /// C++ `CvUnit::getPathEndTurnPlot`.
    fn unit_get_path_end_turn_plot(&self, u: UnitId) -> Option<PlotIdx>;
    /// C++ `CvSelectionGroup::resetPath`.
    fn group_reset_path(&self, g: GroupId);
}

impl CppPaths for Game {
    fn group_generate_path(&self, g: GroupId, from: PlotIdx, to: PlotIdx, flags: i32, _reuse: bool) -> Option<i32> {
        let path = self.group_find_path(g, from, to, flags);
        LAST_PATH.with(|last| last.set(path));
        path.map(|p| p.turns)
    }

    fn unit_generate_path(&self, u: UnitId, to: PlotIdx, flags: i32, _reuse: bool, path_turns: Option<&mut i32>) -> bool {
        let path = self.unit_find_path(u, to, flags);
        LAST_PATH.with(|last| last.set(path));
        if let Some(path_turns) = path_turns {
            *path_turns = path.map_or(MAX_INT, |p| p.turns);
        }
        path.is_some()
    }

    fn group_get_path_first_plot(&self, _g: GroupId) -> Option<PlotIdx> {
        LAST_PATH.with(|last| last.get()).and_then(|p| p.first_plot)
    }

    fn group_get_path_end_turn_plot(&self, _g: GroupId) -> Option<PlotIdx> {
        LAST_PATH.with(|last| last.get()).and_then(|p| p.end_turn_plot)
    }

    fn unit_get_path_end_turn_plot(&self, _u: UnitId) -> Option<PlotIdx> {
        LAST_PATH.with(|last| last.get()).and_then(|p| p.end_turn_plot)
    }

    fn group_reset_path(&self, _g: GroupId) {
        LAST_PATH.with(|last| last.set(None));
    }
}
