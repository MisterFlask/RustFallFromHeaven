//! The helpers the cards share (docs/CARDS.md, section 3), and their lookups.

// (Remove `unused_variables, unreachable_code` once every lookup has a card in it.)
#![allow(unused_imports, unused_variables, unreachable_code)]

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

/// Unit callbacks.
pub fn find_unit_fn(name: &str) -> Option<UnitFn> {
    let f: UnitFn = match name {
        _ => return None,
    };
    #[allow(unreachable_code)]
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Regions (docs/DECKS_ROUND8.md C.3, rule 9).

/// The regions of plots that pass `member`: each a connected group (the 8 neighbours) of unowned
/// land plots without a city, largest first (ties: the region with the lowest plot index first).
pub fn regions(g: &Game, member: impl Fn(&Game, PlotIdx) -> bool) -> Vec<Vec<PlotIdx>> {
    use crate::utils::{PLOT_DIRECTION_X, PLOT_DIRECTION_Y};
    let n = g.map.plots.len();
    let ok = |t: PlotIdx| {
        let plot = &g.map.plots[t];
        plot.owner.is_none() && plot.plot_city.is_none() && !g.plot_is_water(t) && member(g, t)
    };
    let mut seen = vec![false; n];
    let mut out: Vec<Vec<PlotIdx>> = Vec::new();
    for start in g.map.plot_ids() {
        if seen[start.idx()] || !ok(start) {
            continue;
        }
        seen[start.idx()] = true;
        let mut region = vec![start];
        let mut i = 0;
        while i < region.len() {
            let (x, y) = g.map.xy(region[i]);
            i += 1;
            for k in 0..PLOT_DIRECTION_X.len() {
                if let Some(t) = g.map.plot_xy(x + PLOT_DIRECTION_X[k], y + PLOT_DIRECTION_Y[k]) {
                    if !seen[t.idx()] && ok(t) {
                        seen[t.idx()] = true;
                        region.push(t);
                    }
                }
            }
        }
        region.sort();
        out.push(region);
    }
    out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    out
}

/// The largest region of plots that pass `member` (see [`regions`]).
pub fn largest_region(g: &Game, member: impl Fn(&Game, PlotIdx) -> bool) -> Option<Vec<PlotIdx>> {
    regions(g, member).into_iter().next()
}
