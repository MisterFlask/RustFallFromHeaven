//! The area effects and the tunnels as the human sees them (docs/CARDS.md, "Area effects"; the
//! Lanun's Raging Seas, the Khazad's Delve): the view's `area_effects` and `delves`, which the
//! renderers draw over the ground and the plot tooltip names (assets/app/marks.js).

use ffh_core::ids::*;
use ffh_core::structs::AreaEffectKind;

use crate::json::Json;
use crate::view::Viewer;

/// An effect's kind as the protocol names it, and as people do.
fn kind_names(kind: AreaEffectKind) -> (&'static str, &'static str) {
    match kind {
        AreaEffectKind::Blizzard => ("blizzard", "Blizzard"),
        AreaEffectKind::Storm => ("storm", "Storm"),
        AreaEffectKind::Fog => ("fog", "Fog"),
        AreaEffectKind::PillarOfDawn => ("pillar_of_dawn", "Pillar of Dawn"),
        AreaEffectKind::RagingSeas => ("raging_seas", "Raging Seas"),
    }
}

/// The plots within `radius` plots of `centre`.
fn area(v: &Viewer, centre: PlotIdx, radius: i32) -> Vec<PlotIdx> {
    let g = v.g;
    let (x, y) = g.map.xy(centre);
    let mut out = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if let Some(q) = g.plot_xy(x, y, dx, dy) {
                let (qx, qy) = g.map.xy(q);
                if g.plot_distance(x, y, qx, qy) <= radius {
                    out.push(q);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `area_effects`: the effects that hold this turn which the human sees a plot of (or made), each
/// `{kind, label, owner, x, y, radius, turns_left, plots: [[x, y], ...]}` with the plots of it the
/// human has revealed.
pub fn area_effects(v: &Viewer) -> Json {
    let g = v.g;
    let turn = g.game_turn();
    let mut out = Vec::new();
    for e in g.area_effects.iter().filter(|e| e.until > turn) {
        let plots = area(v, e.plot, e.radius);
        if e.owner != v.h && !plots.iter().any(|&q| v.plot_visible(q)) {
            continue;
        }
        let (kind, label) = kind_names(e.kind);
        let (x, y) = g.map.xy(e.plot);
        let shown: Vec<Json> = plots.into_iter().filter(|&q| v.plot_revealed(q)).map(|q| {
            let (qx, qy) = g.map.xy(q);
            Json::Arr(vec![Json::from(qx), Json::from(qy)])
        }).collect();
        out.push(crate::jobj! {
            "kind" => kind,
            "label" => label,
            "owner" => e.owner,
            "x" => x,
            "y" => y,
            "radius" => e.radius,
            "turns_left" => e.until - turn,
            "plots" => Json::Arr(shown),
        });
    }
    Json::Arr(out)
}

/// `delves`: each tunnel end the human has revealed, `{x, y, to: [x, y]}`.
pub fn delves(v: &Viewer) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for &(a, b) in &g.delves {
        for (here, there) in [(a, b), (b, a)] {
            if v.plot_revealed(here) {
                let (x, y) = g.map.xy(here);
                let (tx, ty) = g.map.xy(there);
                out.push(crate::jobj! { "x" => x, "y" => y, "to" => Json::Arr(vec![Json::from(tx), Json::from(ty)]) });
            }
        }
    }
    Json::Arr(out)
}
