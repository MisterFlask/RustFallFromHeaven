//! Squads as the human sees them (docs/ENGAGEMENTS.md, "Squads"): each of its units' `squad`
//! (`{id, num, cell, auto}`), and the squads of a plot (the `group` query's, the Armies screen's
//! per army) for the squad board; and where every listed unit stands on the map (`stand`). Pure,
//! like every view.

use std::collections::{BTreeMap, BTreeSet};

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

/// Where a listed unit stands on its plot (the unit's `stand`): its squad's rank among the squads
/// on the plot with a listed unit, strongest first (0-based), its cell in that squad's layout (a
/// large unit's first) and whether it is large (two cells).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stand {
    pub rank: u32,
    pub cell: u8,
    pub large: bool,
}

impl Stand {
    pub fn json(&self) -> Json {
        let mut j = crate::jobj! { "rank" => self.rank, "cell" => self.cell };
        if self.large {
            j.set("large", true);
        }
        j
    }
}

/// Where each of `units` (the view's unit list) that stands in a squad stands: the layouts as the
/// rules have them now ([`Game::squad_layouts_at`], once per owner and plot), every owner's squads
/// on a plot ranked together by the summed attack rating of their listed units (a large unit
/// counts double; ties: owners by id, then the layouts' order). Units of a squad not in `units`
/// count for nothing; a squad with no listed unit takes no rank. Units in no squad are absent.
pub fn stands(v: &Viewer, units: &[UnitId]) -> BTreeMap<UnitId, Stand> {
    let g = v.g;
    let listed: BTreeSet<UnitId> = units.iter().copied().collect();
    // Each plot with a listed squad unit, and its owners there (ascending).
    let mut at: BTreeMap<PlotIdx, BTreeSet<PlayerId>> = BTreeMap::new();
    for &u in units {
        if g.units.contains(u) && g.unit_in_squads(u) {
            if let Some(plot) = g.unit_plot_opt(u) {
                at.entry(plot).or_default().insert(g.units[u].owner);
            }
        }
    }
    let mut out = BTreeMap::new();
    for (plot, owners) in at {
        // (strength, [(unit, cell, large)]) of each squad with a listed unit, in the rules' order.
        let mut ranked: Vec<(i64, Vec<(UnitId, u8, bool)>)> = Vec::new();
        for p in owners {
            for l in g.squad_layouts_at(p, plot) {
                let members: Vec<(UnitId, u8, bool)> = l.units().into_iter().filter(|(u, _)| listed.contains(u)).map(|(u, c)| (u, c, g.unit_engage_large(u))).collect();
                if members.is_empty() {
                    continue;
                }
                let strength = members.iter().map(|&(u, _, large)| g.unit_attack_rating(u) as i64 * (1 + large as i64)).sum();
                ranked.push((strength, members));
            }
        }
        // Stable: ties keep the rules' order.
        ranked.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
        for (rank, (_, members)) in ranked.into_iter().enumerate() {
            for (u, cell, large) in members {
                out.insert(u, Stand { rank: rank as u32, cell, large });
            }
        }
    }
    out
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
