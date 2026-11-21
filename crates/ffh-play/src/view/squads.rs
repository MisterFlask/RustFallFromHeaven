//! Squads as the human sees them (docs/ENGAGEMENTS.md, "Squads"): each of its units' `squad`
//! (`{id, num, cell, auto}`), and the squads of a plot (the `group` query's, the Armies screen's
//! per army) for the squad board. Pure, like every view.

use ffh_core::army::{SquadBody, SquadLayout};
use ffh_core::game::Game;
use ffh_core::ids::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// Unit `u`'s squad, from what the last sync wrote (`Unit::squad`, `Unit::squad_cell`): `{id,
/// num, cell, auto}` (`num`: its number within its body on the unit's plot, 1-based, as the
/// layouts order them: by id), or null for a unit in no squad.
pub fn unit_squad(g: &Game, u: UnitId) -> Json {
    let unit = &g.units[u];
    let Some(s) = unit.squad.filter(|&s| g.squads.contains(s) && g.unit_in_squads(u)) else { return Json::Null };
    let sq = &g.squads[s];
    let mut before: Vec<SquadId> = Vec::new();
    if let Some(plot) = g.unit_plot_opt(u) {
        for &x in &g.map.plots[plot].units {
            if !g.units.contains(x) || g.units[x].owner != unit.owner || !g.unit_in_squads(x) {
                continue;
            }
            if let Some(t) = g.units[x].squad.filter(|&t| t < s && g.squads.contains(t) && g.squads[t].body == sq.body && !before.contains(&t)) {
                before.push(t);
            }
        }
    }
    crate::jobj! { "id" => s, "num" => before.len() + 1, "cell" => unit.squad_cell, "auto" => !sq.hand_set }
}

/// What a squad's body is called: the army's name, "<city> garrison", or "Loose".
pub fn body_label(g: &Game, body: SquadBody) -> String {
    match body {
        SquadBody::Army(a) => g.armies.get(a).map_or_else(String::new, |a| a.name.clone()),
        SquadBody::Garrison(c) => format!("{} garrison", city_name(g, c)),
        SquadBody::Loose => "Loose".to_string(),
    }
}

fn layout_json(g: &Game, l: &SquadLayout, num: u32) -> Json {
    let (kind, army, city) = match l.body {
        SquadBody::Army(a) => ("army", Some(a), None),
        SquadBody::Garrison(c) => ("garrison", None, Some(c)),
        SquadBody::Loose => ("loose", None, None),
    };
    let places: i32 = l.units().iter().map(|&(u, _)| 1 + g.unit_engage_large(u) as i32).sum();
    crate::jobj! {
        "id" => l.squad,
        "num" => num,
        "body" => body_label(g, l.body),
        "kind" => kind,
        "army" => army,
        "city" => city,
        "auto" => !l.hand_set,
        "places" => places,
        "max_places" => g.squad_places(),
        "cells" => Json::Arr(l.cells.iter().map(|&c| Json::from(c)).collect()),
    }
}

/// The human's squads on `plot` (all of its bodies there, or only `body`'s), in the rules' order:
/// `[{id, num, body (its label), kind ("army" | "garrison" | "loose"), army, city, auto, places,
/// max_places, cells (9 unit ids or null, row * 3 + lane, row 0 the front; a large unit in both
/// of its cells)}]`. Empty for a plot the human has no squads on (or another player's units).
pub fn plot_squads(v: &Viewer, plot: PlotIdx, body: Option<SquadBody>) -> Json {
    let g = v.g;
    let layouts = g.squad_layouts_at(v.h, plot);
    let nums = Game::squad_numbers(&layouts);
    Json::Arr(layouts.iter().zip(nums).filter(|(l, _)| body.is_none_or(|b| l.body == b)).map(|(l, n)| layout_json(g, l, n)).collect())
}

/// Every owner's squads on a plot the human sees (the plot query's `squads`, for the read-only
/// board of another player's stack): `[{player, squads}]`, the owners in the order of their first
/// unit on the plot, each `squads` as [`plot_squads`] lists them. A unit the human cannot see
/// leaves its cells empty (null) and its places uncounted; a squad of none the human sees is left
/// out, and so are units whose owner the human does not know (hidden nationality: `v.unit_owner`).
pub fn owner_squads(v: &Viewer, plot: PlotIdx) -> Json {
    let g = v.g;
    let mut owners: Vec<PlayerId> = Vec::new();
    for &u in &g.map.plots[plot].units {
        if g.units.contains(u) && v.unit_visible(u) && v.unit_owner(u) == g.units[u].owner && !owners.contains(&g.units[u].owner) {
            owners.push(g.units[u].owner);
        }
    }
    let seen = |u: UnitId| g.units.contains(u) && v.unit_visible(u) && v.unit_owner(u) == g.units[u].owner;
    let out = owners.into_iter().filter_map(|p| {
        let layouts = g.squad_layouts_at(p, plot);
        let nums = Game::squad_numbers(&layouts);
        let squads: Vec<Json> = layouts
            .iter()
            .zip(nums)
            .filter_map(|(l, n)| {
                let mut l = l.clone();
                for c in l.cells.iter_mut() {
                    if c.is_some_and(|u| !seen(u)) {
                        *c = None;
                    }
                }
                l.cells.iter().any(Option::is_some).then(|| layout_json(g, &l, n))
            })
            .collect();
        (!squads.is_empty()).then(|| crate::jobj! { "player" => p, "squads" => Json::Arr(squads) })
    });
    Json::Arr(out.collect())
}
