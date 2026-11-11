//! Outposts and their lanes (docs/NAVAL.md, sections 9 and 10), as the human sees them: an
//! outpost's part of the city panel (`city`), the cities' `outpost` field (the banner's mark and
//! the Cities list's row), the lanes the Sea layer draws (`lanes`: the view's `outpost_lanes`) and
//! the resource tooltip's `Not on your landmasses.`. Text is bare (CLAUDE.md, "Rules text and
//! tooltips").

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::naval::SeaLane;
use ffh_core::outposts::LaneState;
use ffh_data::*;

use crate::json::Json;
use crate::view::Viewer;

/// The state of outpost `c`'s lane as the human works it out: `("open" | "cut" | "none", the
/// lane)`.
fn state(v: &Viewer, c: CityId) -> (&'static str, Option<SeaLane>) {
    match v.g.outpost_lane_state(c, v.team) {
        LaneState::Open(l) => ("open", Some(l)),
        LaneState::Cut(l) => ("cut", Some(l)),
        LaneState::NoLane => ("none", None),
    }
}

/// A city's name as the human may name it ("a city" when not revealed).
fn name(v: &Viewer, c: CityId) -> String {
    if v.g.cities.contains(c) && (v.city_revealed(c) || v.g.cities[c].owner == v.h) {
        crate::view::city_name(v.g, c)
    } else {
        "a city".to_string()
    }
}

/// The lines of outpost `c`'s panel (docs/NAVAL.md, 10).
fn lines(v: &Viewer, c: CityId) -> Vec<String> {
    let g = v.g;
    let starve = g.d.defines.lane_starve_turns;
    let turns = g.cities[c].lane_cut_turns;
    let mut out = Vec::new();
    match state(v, c) {
        ("open", Some(l)) => out.push(format!("Lane: open, to {} ({} plots).", name(v, l.port), l.path.len())),
        (_, Some(l)) => {
            let sea = l.cut_in.map_or_else(String::new, |(s, _)| g.map.seas[s.idx()].name.clone());
            out.push(format!("Lane: cut in {sea}, turn {} of {starve}.", turns.max(1)));
        }
        _ => out.push(format!("No lane to a home port in range, turn {} of {starve}.", turns.max(1))),
    }
    if g.city_outpost_starved(c) {
        out.push("Starved: -1 population a turn, not a port.".to_string());
    }
    out
}

/// Outpost `c`'s part of the city panel, null for a city: `{lane, turns, starve, starved, lines,
/// raise: {cost, enabled, reason, label}}`.
pub fn city(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if !g.cities.contains(c) || !g.cities[c].outpost {
        return Json::Null;
    }
    let (lane, _) = state(v, c);
    let cost = g.outpost_raise_cost();
    let refusal = (g.cities[c].owner == v.h).then(|| g.player_raise_outpost_refusal(v.h, c)).flatten();
    crate::jobj! {
        "lane" => lane,
        "turns" => g.cities[c].lane_cut_turns,
        "starve" => g.d.defines.lane_starve_turns,
        "starved" => g.city_outpost_starved(c),
        "max_pop" => g.d.defines.outpost_max_pop,
        "lines" => lines(v, c),
        "raise" => crate::jobj! {
            "cost" => cost,
            "enabled" => g.cities[c].owner == v.h && refusal.is_none(),
            "reason" => refusal,
            "label" => format!("Raise to city ({cost} gold)"),
        },
    }
}

/// The `outpost` field of a revealed city in the view's `cities` (null for a city): the human's
/// own outposts add the lane's state and the panel's first line (the Cities list's row).
pub fn city_fields(v: &Viewer, c: CityId, j: &mut Json) {
    let g = v.g;
    if !g.cities[c].outpost {
        return;
    }
    if g.cities[c].owner == v.h || v.reveal {
        let (lane, _) = state(v, c);
        j.set("outpost", crate::jobj! { "lane" => lane, "starved" => g.city_outpost_starved(c), "text" => lines(v, c).join(" ") });
    } else {
        j.set("outpost", crate::jobj! {});
    }
}

/// The plots of `path` the human has revealed, split into runs where it has not.
fn revealed_runs(v: &Viewer, path: &[PlotIdx]) -> Vec<Vec<PlotIdx>> {
    let mut out: Vec<Vec<PlotIdx>> = Vec::new();
    let mut run: Vec<PlotIdx> = Vec::new();
    for &q in path {
        if v.plot_revealed(q) {
            run.push(q);
        } else if !run.is_empty() {
            out.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

fn xy(g: &Game, q: PlotIdx) -> Json {
    let (x, y) = g.map.xy(q);
    crate::jarr![x, y]
}

/// The outposts' lanes the Sea layer draws (docs/NAVAL.md, 10): `[{owner, city, mine, state,
/// segments: [[[x, y], ...]], cut_segments, cut: {x, y, sea, turns} | null}]`. Each segment runs
/// from the home port's end toward the outpost (the flow goes to the port). A cut lane breaks where
/// it enters the closed sea that cuts it: `segments` is the outpost's side up to there,
/// `cut_segments` the rest to the port (from the break), drawn dotted; `cut` the mark there, with
/// the turns left before the outpost starves (yours only). Your lanes in
/// full; others' (of outposts you have revealed) over the plots you have revealed only, as you can
/// work them out.
pub fn lanes(v: &Viewer) -> Json {
    let g = v.g;
    let starve = g.d.defines.lane_starve_turns;
    let mut out = Vec::new();
    for (c, city) in g.cities.iter() {
        if !city.outpost {
            continue;
        }
        let mine = city.owner == v.h || v.reveal;
        if !mine && !v.city_revealed(c) {
            continue;
        }
        let Some(lane) = g.outpost_lane_known(c, v.team) else { continue };
        // The parts: before the closed sea and after it (from the outpost).
        // The open part (from the outpost to where it enters the closed sea) and the cut part
        // (from there to the port, drawn dotted as a supply ship's cut lane is).
        let (parts, cut_part, cut) = match lane.cut_in {
            None => (vec![lane.path.clone()], Vec::new(), Json::Null),
            Some((s, _)) => {
                let start = lane.path.iter().position(|&q| g.map.plots[q].sea == Some(s)).unwrap_or(0);
                let at = lane.path[start];
                let (x, y) = g.map.xy(at);
                let left = (starve - city.lane_cut_turns).max(0);
                let mark = if mine || v.plot_revealed(at) { crate::jobj! { "x" => x, "y" => y, "sea" => g.map.seas[s.idx()].name.as_str(), "turns" => if mine { Json::from(left) } else { Json::Null } } } else { Json::Null };
                (vec![lane.path[..=start].to_vec()], lane.path[start..].to_vec(), mark)
            }
        };
        // Each cut run from the closed sea's entry toward the port (its first point the break).
        let mut cut_segments: Vec<Json> = Vec::new();
        for run in if mine { vec![cut_part] } else { revealed_runs(v, &cut_part) } {
            if run.len() >= 2 {
                cut_segments.push(Json::Arr(run.iter().map(|&q| xy(g, q)).collect()));
            }
        }
        let mut segments: Vec<Json> = Vec::new();
        for part in parts {
            let runs = if mine { vec![part] } else { revealed_runs(v, &part) };
            for run in runs {
                if run.len() < 2 {
                    continue;
                }
                segments.push(Json::Arr(run.iter().rev().map(|&q| xy(g, q)).collect()));
            }
        }
        out.push(crate::jobj! {
            "owner" => city.owner,
            "city" => c,
            "mine" => city.owner == v.h,
            "state" => if lane.is_cut() { "cut" } else { "open" },
            "segments" => Json::Arr(segments),
            "cut_segments" => Json::Arr(cut_segments),
            "cut" => cut,
        });
    }
    Json::Arr(out)
}

/// The resource tooltip's line (docs/NAVAL.md, 10): `Not on your landmasses.` when no land area
/// the human has a city on holds a plot it has revealed with that resource; null otherwise (or
/// with no city).
pub fn bonus_line(v: &Viewer, b: BonusType) -> Option<&'static str> {
    let g = v.g;
    if g.players[v.h].cities.is_empty() || g.player_bonus_on_own_landmass(v.h, b) {
        return None;
    }
    Some("Not on your landmasses.")
}
