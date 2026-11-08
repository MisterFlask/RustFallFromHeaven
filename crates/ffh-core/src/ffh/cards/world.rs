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

// ---------------------------------------------------------------------------------------------
// Borders and the Armageddon counter.

/// Does `p` own a plot within `range` plots (plot distance) of `plot`? A card's "within N plots of
/// your borders".
pub fn within_range_of_borders(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> bool {
    let (x, y) = g.map.xy(plot);
    for dx in -range..=range {
        for dy in -range..=range {
            if g.plot_distance(x, y, x + dx, y + dy) > range {
                continue;
            }
            if g.plot_xy(x, y, dx, dy).is_some_and(|q| g.map.plots[q].owner == Some(p)) {
                return true;
            }
        }
    }
    false
}

/// The raw change (`change_global_counter`) that raises the Armageddon counter's percentage
/// (`get_global_counter`, the scale of the Armageddon deck's slots) by `points`, whatever the
/// counter's limit: a card's "+N to the Armageddon counter".
pub fn counter_change_for_points(g: &Game, points: i32) -> i32 {
    let limit = g.global_counter_limit.max(1) as i64;
    let target = (g.get_global_counter() + points) as i64;
    // The smallest raw value whose percentage reaches the target.
    let raw = (target * limit + 99) / 100;
    (raw - g.global_counter as i64).max(0) as i32
}
