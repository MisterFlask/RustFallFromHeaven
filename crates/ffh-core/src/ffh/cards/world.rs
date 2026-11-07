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

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelInfo" => label_info,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Choice targets whose options are entries of an info table (docs/CARDS.md, section 3).

/// The kinds `labelInfo` names entries of (the info tables a choice spell may offer).
pub const LABEL_KINDS: [&str; 15] = [
    "TraitType",
    "BuildingType",
    "BuildingClassType",
    "UnitType",
    "UnitClassType",
    "PromotionType",
    "BonusType",
    "BonusClassType",
    "DamageType",
    "ReligionType",
    "CivicType",
    "TechType",
    "ImprovementType",
    "ProjectType",
    "SpecialistType",
];

/// The description key of entry `i` of the info table of kind `kind` (one of [`LABEL_KINDS`]).
pub fn info_description<'d>(d: &'d GameData, kind: &str, i: i32) -> Option<&'d str> {
    let i = usize::try_from(i).ok()?;
    fn key<'d, I: Idx, T>(v: &'d IdxVec<I, T>, i: usize, f: impl Fn(&'d T) -> &'d InfoBase) -> Option<&'d str> {
        (i < v.len()).then(|| f(&v[I::from_usize(i)]).description.as_str())
    }
    match kind {
        "TraitType" => key(&d.traits, i, |x| &x.base),
        "BuildingType" => key(&d.buildings, i, |x| &x.base),
        "BuildingClassType" => key(&d.building_classes, i, |x| &x.base),
        "UnitType" => key(&d.units, i, |x| &x.base),
        "UnitClassType" => key(&d.unit_classes, i, |x| &x.base),
        "PromotionType" => key(&d.promotions, i, |x| &x.base),
        "BonusType" => key(&d.bonuses, i, |x| &x.base),
        "BonusClassType" => key(&d.bonus_classes, i, |x| &x.base),
        "DamageType" => key(&d.damages, i, |x| &x.base),
        "ReligionType" => key(&d.religions, i, |x| &x.base),
        "CivicType" => key(&d.civics, i, |x| &x.base),
        "TechType" => key(&d.techs, i, |x| &x.base),
        "ImprovementType" => key(&d.improvements, i, |x| &x.base),
        "ProjectType" => key(&d.projects, i, |x| &x.base),
        "SpecialistType" => key(&d.specialists, i, |x| &x.base),
        _ => None,
    }
}

/// `labelInfo(pCaster, iChoice, 'TraitType')`: a choice spell whose options are entries of an info
/// table (its third argument, one of [`LABEL_KINDS`]) names option `choice` by the entry's name.
/// A UI also shows the entry's help (ffh-play reads the kind from the call).
fn label_info(g: &Game, _caster: UnitId, choice: i32, call: &PyCall) -> String {
    let d = g.d;
    match call.str_arg(2).and_then(|kind| info_description(d, kind, choice)) {
        Some(key) => d.text.name(key).to_string(),
        None => format!("Option {choice}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Distances and levels.

/// The plots within `range` plots (the 8 neighbours, the map wrapping) of a plot player `p` owns,
/// its own plots included, as a mask by plot index: "within N plots of your borders".
pub fn near_borders(g: &Game, p: PlayerId, range: i32) -> Vec<bool> {
    use crate::utils::{PLOT_DIRECTION_X, PLOT_DIRECTION_Y};
    let n = g.map.plots.len();
    let mut dist = vec![i32::MAX; n];
    let mut queue: Vec<PlotIdx> = Vec::new();
    for t in g.map.plot_ids() {
        if g.map.plots[t].owner == Some(p) {
            dist[t.idx()] = 0;
            queue.push(t);
        }
    }
    let mut i = 0;
    while i < queue.len() {
        let t = queue[i];
        i += 1;
        let next = dist[t.idx()] + 1;
        if next > range {
            continue;
        }
        let (x, y) = g.map.xy(t);
        for k in 0..PLOT_DIRECTION_X.len() {
            if let Some(q) = g.map.plot_xy(x + PLOT_DIRECTION_X[k], y + PLOT_DIRECTION_Y[k]) {
                if dist[q.idx()] > next {
                    dist[q.idx()] = next;
                    queue.push(q);
                }
            }
        }
    }
    dist.into_iter().map(|x| x <= range).collect()
}

/// Unit `u` gains `n` levels (docs/DECKS_ROUND8.md C.3, rule 10: a level is the XP for its next
/// level): its experience rises to what it needs for `n` more promotions than it can take now.
pub fn unit_gain_levels(g: &mut Game, u: UnitId, n: i32) {
    if n <= 0 {
        return;
    }
    let owner = g.units[u].owner;
    let level = g.units[u].level;
    let xp = g.units[u].experience;
    // The promotions it can take now: the levels whose threshold its experience meets.
    let mut ready = 0;
    while g.py_get_experience_needed(level + ready, owner) <= xp {
        ready += 1;
    }
    let target = g.py_get_experience_needed(level + ready + n - 1, owner);
    if target > xp {
        g.unit_set_experience(u, target, -1);
    }
    g.unit_test_promotion_ready(u);
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
