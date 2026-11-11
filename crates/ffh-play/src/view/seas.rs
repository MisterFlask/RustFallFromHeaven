//! The sea, phase 2 (docs/NAVAL.md, sections 4, 5 and 10), as the human sees it: named seas and
//! who holds them (the map's `sea_id` array, the view's `seas`, the `sea` query's panel and the
//! `seas` query's screen), the harbours (each city's `harbour`), the supply ships' lanes (the
//! view's `sea_lanes`), interception and the Patrol order (the path preview's and the unit
//! panel's lines). Everything is what the human can work out (the rules' *known* forms: only the
//! ships it sees count), apart from its own cities' harbours. Text is bare (CLAUDE.md, "Rules
//! text and tooltips").

use ffh_core::army::SupplyShipRefusal;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::path::PathFinder;
use ffh_core::seas::{Harbour, SeaControl, SeaParty, SeaState};
use ffh_data::*;

use crate::json::Json;
use crate::query::QueryErr;
use crate::view::sea::port_name;
use crate::view::Viewer;

/// The `sea_id` map array's value of revealed plot `p`: its sea's index on water, else -1.
pub fn sea_id_value(g: &Game, p: PlotIdx) -> i32 {
    if !g.plot_is_water(p) {
        return -1;
    }
    g.plot_sea(p).map_or(-1, |s| s.0 as i32)
}

/// A party's name as the human reads it: "you", a team's civilization, or "Unknown" (ships of
/// hidden nationality whose team the human cannot tell).
pub fn party_name(v: &Viewer, p: SeaParty) -> String {
    match p {
        Some(t) if t == v.team => "you".to_string(),
        Some(t) => v.g.team_label(t),
        None => "Unknown".to_string(),
    }
}

/// A team's name as the human reads it ("you" for its own).
fn team_name(v: &Viewer, t: TeamId) -> String {
    party_name(v, Some(t))
}

fn sea_name(g: &Game, s: SeaId) -> String {
    g.map.seas[s.idx()].name.clone()
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn plural(n: i32, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

/// "twice" for `SEA_HOLD_PERCENT` 200, else "N times" or "N% of".
fn hold_times(g: &Game) -> String {
    let p = g.d.defines.sea_hold_percent;
    match (p % 100, p / 100) {
        (0, 2) => "twice".to_string(),
        (0, n) => format!("{n} times"),
        _ => format!("{p}% of"),
    }
}

/// The strength holding against `x` takes: `SEA_HOLD_PERCENT` of it, rounded up, at least 1.
fn hold_need(g: &Game, x: i32) -> i64 {
    let hold = g.d.defines.sea_hold_percent.max(0) as i64;
    ((x as i64 * hold + 99) / 100).max(1)
}

/// The control of a state in words: "Held by you", "Held by Lanun", "Contested", "Open",
/// "Patrolled by Lanun".
pub fn control_text(v: &Viewer, st: &SeaState) -> String {
    match st.control {
        SeaControl::HeldByUs => "Held by you".to_string(),
        SeaControl::HeldBy(p) => format!("Held by {}", party_name(v, p)),
        SeaControl::Contested(_) => "Contested".to_string(),
        SeaControl::Open => "Open".to_string(),
        SeaControl::PatrolledBy(p) => format!("Patrolled by {}", party_name(v, p)),
    }
}

/// A state's result line: `38 to 17. Held by you.`
pub fn result_text(v: &Viewer, st: &SeaState) -> String {
    format!("{} to {}. {}.", st.ours, st.theirs, control_text(v, st))
}

/// What the human is shown of a sea ([`summary`]).
pub struct SeaSummary {
    /// `held_you`, `held`, `contested`, `open`, `patrolled` or `out`.
    pub state: &'static str,
    /// `HELD: YOU 24 to 0`, `CONTESTED 14 to 17`, `HELD: LANUN 0 to 31`, `OPEN`, `PATROLLED:
    /// LANUN`, `OUT OF RANGE`.
    pub badge: String,
    /// "Held by you", ..., "Out of range".
    pub control: String,
    pub st: SeaState,
    /// The party the control names (none for "you" and for Open).
    pub party: Option<SeaParty>,
}

/// Sea `s` as the human can work it out (only ships it sees count). Out of range: open, and no
/// plot of it the human has revealed is in its team's range.
pub fn summary(v: &Viewer, s: SeaId) -> SeaSummary {
    let g = v.g;
    let st = g.sea_state_known(s, v.team);
    let out_of_range = !g.team_sea_exempt(v.team) && !g.map.seas[s.idx()].plots.iter().any(|&q| v.plot_revealed(q) && g.plot_in_sea_range(v.team, q));
    let upper = |p: SeaParty| party_name(v, p).to_uppercase();
    let (state, badge, party) = match st.control {
        SeaControl::HeldByUs => ("held_you", format!("HELD: YOU {} to {}", st.ours, st.theirs), None),
        SeaControl::HeldBy(p) => ("held", format!("HELD: {} {} to {}", upper(p), st.ours, st.theirs), Some(p)),
        SeaControl::Contested(p) => ("contested", format!("CONTESTED {} to {}", st.ours, st.theirs), Some(p)),
        SeaControl::Open if out_of_range => ("out", "OUT OF RANGE".to_string(), None),
        SeaControl::Open => ("open", "OPEN".to_string(), None),
        SeaControl::PatrolledBy(p) => ("patrolled", format!("PATROLLED: {}", upper(p)), Some(p)),
    };
    let control = if state == "out" { "Out of range".to_string() } else { control_text(v, &st) };
    SeaSummary { state, badge, control, st, party }
}

/// The seas the human has revealed a plot of, in id order, each with its revealed plots.
pub fn seen_seas(v: &Viewer) -> Vec<(SeaId, Vec<PlotIdx>)> {
    let g = v.g;
    g.sea_ids()
        .filter_map(|s| {
            let plots: Vec<PlotIdx> = g.map.seas[s.idx()].plots.iter().copied().filter(|&q| v.plot_revealed(q)).collect();
            (!plots.is_empty()).then_some((s, plots))
        })
        .collect()
}

/// Where a sea's name goes on the map: of its `plots` (the revealed ones), the one farthest in
/// steps (8 directions) from every plot that is not one of them; ties: the one with the least
/// total distance to the other tied plots (the middle of the widest part).
pub fn label_plot(g: &Game, plots: &[PlotIdx]) -> PlotIdx {
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    let inside: BTreeSet<PlotIdx> = plots.iter().copied().collect();
    let mut dist: BTreeMap<PlotIdx, u32> = BTreeMap::new();
    let mut queue: VecDeque<PlotIdx> = VecDeque::new();
    for &q in plots {
        let (x, y) = g.map.xy(q);
        let edge = (-1..=1).any(|dy| (-1..=1).any(|dx| (dx != 0 || dy != 0) && g.map.plot_xy(x + dx, y + dy).is_none_or(|n| !inside.contains(&n))));
        if edge {
            dist.insert(q, 0);
            queue.push_back(q);
        }
    }
    while let Some(q) = queue.pop_front() {
        let d = dist[&q];
        let (x, y) = g.map.xy(q);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(n) = g.map.plot_xy(x + dx, y + dy).filter(|n| inside.contains(n) && !dist.contains_key(n)) {
                    dist.insert(n, d + 1);
                    queue.push_back(n);
                }
            }
        }
    }
    let depth = |q: &PlotIdx| dist.get(q).copied().unwrap_or(0);
    let best = plots.iter().map(depth).max().unwrap_or(0);
    let tied: Vec<PlotIdx> = plots.iter().copied().filter(|q| depth(q) == best).take(200).collect();
    let xy: Vec<(i32, i32)> = tied.iter().map(|&q| g.map.xy(q)).collect();
    let mut pick = (i64::MAX, tied[0]);
    for (i, &q) in tied.iter().enumerate() {
        let (x, y) = xy[i];
        let sum: i64 = xy.iter().map(|&(ox, oy)| g.plot_distance(x, y, ox, oy) as i64).sum();
        if sum < pick.0 {
            pick = (sum, q);
        }
    }
    pick.1
}

/// The view's `seas` (docs/NAVAL.md, 10: the Sea layer's names and badges): each sea the human has
/// revealed a plot of, `{id, name, x, y (where its name goes), plots (its revealed plots), state,
/// badge, control, ours, theirs, party (the team the control names; null for none, you or an
/// unknown party)}`.
pub fn seas_json(v: &Viewer) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for (s, plots) in seen_seas(v) {
        let sm = summary(v, s);
        let (x, y) = g.map.xy(label_plot(g, &plots));
        out.push(crate::jobj! {
            "id" => s.0 as i64,
            "name" => sea_name(g, s),
            "x" => x,
            "y" => y,
            "plots" => plots.len() as i64,
            "state" => sm.state,
            "badge" => sm.badge,
            "control" => sm.control,
            "ours" => sm.st.ours,
            "theirs" => sm.st.theirs,
            "party" => sm.party.flatten().filter(|&t| t != v.team).map(|t| t.0 as i64),
        });
    }
    Json::Arr(out)
}

// ---------------------------------------------------------------------------------------------
// Harbours

/// A city's harbour as the human may know it (docs/NAVAL.md, 4.4 point 1): the truth for its own
/// cities, [`Game::city_harbour_known`] for the others'.
pub fn harbour(v: &Viewer, c: CityId) -> Harbour {
    let g = v.g;
    if g.cities[c].owner == v.h || v.reveal {
        g.city_harbour(c)
    } else {
        g.city_harbour_known(c, v.team)
    }
}

/// The sea of city `c` its own team holds, as the human can work it out (the first by id).
fn held_by_city_team(v: &Viewer, c: CityId) -> Option<SeaId> {
    let g = v.g;
    let team = g.city_team(c);
    g.city_seas(c).into_iter().find(|&s| g.sea_state_known_if(s, team, v.team, &[]).control == SeaControl::HeldByUs)
}

/// A city's `harbour` in the view (null for a city on no sea): `{state: "open" | "blockaded",
/// besieged (blockaded and besieged: the banner mark's second state; only while the human sees
/// the city), text}`: `Harbour: open. Bay of Lorda is held by you.`, `Harbour: open.`, `Harbour:
/// blockaded. Sirens' Water is held by Lanun. Sea plots not worked, no sea trade.`
pub fn harbour_json(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let team = g.city_team(c);
    match harbour(v, c) {
        Harbour::None => Json::Null,
        Harbour::Open => {
            let text = match held_by_city_team(v, c) {
                Some(s) => format!("Harbour: open. {} is held by {}.", sea_name(g, s), team_name(v, team)),
                None => "Harbour: open.".to_string(),
            };
            crate::jobj! { "state" => "open", "besieged" => false, "text" => text }
        }
        Harbour::Blockaded(held) => {
            let parts: Vec<String> = held.iter().map(|&(s, t)| format!("{} is held by {}.", sea_name(g, s), team_name(v, t))).collect();
            let besieged = v.city_visible(c) && g.city_is_besieged(c);
            let text = format!("Harbour: blockaded. {} Sea plots not worked, no sea trade.", parts.join(" "));
            crate::jobj! { "state" => "blockaded", "besieged" => besieged, "text" => text }
        }
    }
}

/// The path preview's lines for an attack on a coastal city (docs/NAVAL.md, 10): `Harbour open:
/// Lanun holds Sirens' Water. The city heals and repairs at 50% during the siege.` / `Harbour
/// open. The city heals ...` / `Harbour blockaded.`; for an enemy's open harbour, what blockading
/// it takes (`To blockade it: hold Sirens' Water (62 needed, you have 0).`). Empty for a city on
/// no sea.
pub fn attack_harbour_lines(v: &Viewer, c: CityId) -> Vec<String> {
    let g = v.g;
    let team = g.city_team(c);
    let pct = g.d.defines.port_siege_heal_percent;
    match harbour(v, c) {
        Harbour::None => Vec::new(),
        Harbour::Blockaded(_) => vec!["Harbour blockaded.".to_string()],
        Harbour::Open => {
            let head = match held_by_city_team(v, c) {
                Some(s) => format!("Harbour open: {} holds {}.", capital(&team_name(v, team)), sea_name(g, s)),
                None => "Harbour open.".to_string(),
            };
            let mut lines = vec![format!("{head} The city heals and repairs at {pct}% during the siege.")];
            let open: Vec<SeaId> = g.city_seas(c).into_iter().filter(|&s| g.sea_closed_known(s, team, v.team).is_none()).collect();
            if team != v.team && g.at_war(v.team, team) {
                if let [s] = open[..] {
                    let theirs = g.sea_state_known_if(s, team, v.team, &[]).ours;
                    let ours = g.sea_state_known(s, v.team).ours;
                    lines.push(format!("To blockade it: hold {} ({} needed, you have {ours}).", sea_name(g, s), hold_need(g, theirs)));
                } else if !open.is_empty() {
                    let names: Vec<String> = open.iter().map(|&s| sea_name(g, s)).collect();
                    lines.push(format!("To blockade it: hold {}.", names.join(" and ")));
                }
            }
            lines
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Interception and Patrol

/// The interception lines of a path (docs/NAVAL.md, 10): for each node after the start that
/// enters a sea other than the one before, where an enemy the human sees patrols: `Enters The Grey
/// Narrows (contested) at plot 4: will be intercepted by Lanun's patrol.` (or that the patrol
/// there would not attack, its odds being too low). `head`: the moving group's head; `nodes`:
/// `(x, y, turn)` from the start (index 0).
pub fn interception_lines(v: &Viewer, head: UnitId, nodes: &[(i32, i32, i32)]) -> Vec<String> {
    let g = v.g;
    let team = g.unit_get_team(head);
    let mut out = Vec::new();
    let mut prev: Option<SeaId> = None;
    for (i, &(x, y, _)) in nodes.iter().enumerate() {
        let Some(p) = g.map.plot_xy(x, y) else { continue };
        let s = if g.plot_is_water(p) { g.plot_sea(p) } else { None };
        let entered = i > 0 && s.is_some() && s != prev;
        prev = s;
        let (true, Some(s)) = (entered, s) else { continue };
        if !v.plot_revealed(p) {
            continue;
        }
        let enemies: Vec<UnitId> = g
            .sea_patrollers(s)
            .into_iter()
            .filter(|&u| v.unit_visible(u) && g.unit_get_team(u) != team && (g.unit_is_enemy(u, Some(team), Some(p)) || g.unit_is_enemy(head, Some(g.unit_get_team(u)), Some(p))))
            .collect();
        if enemies.is_empty() {
            continue;
        }
        let sm = summary(v, s);
        let state = match sm.st.control {
            SeaControl::HeldBy(q) => format!("held by {}", party_name(v, q)),
            _ => sm.control.to_lowercase(),
        };
        // The plot is empty before the move: the mover defends (its head).
        let by: Vec<UnitId> = g.sea_interceptors_for(head, p, Some(head)).into_iter().filter(|u| enemies.contains(u)).collect();
        let mut who: Vec<String> = Vec::new();
        for &u in &by {
            let n = party_name(v, if g.unit_is_hidden_nationality(u) { None } else { Some(g.unit_get_team(u)) });
            if !who.contains(&n) {
                who.push(n);
            }
        }
        let what = if who.is_empty() {
            format!("the patrol there does not attack (odds below {}%).", g.d.defines.intercept_min_odds)
        } else {
            format!("will be intercepted by {}.", who.iter().map(|n| format!("{n}'s patrol")).collect::<Vec<_>>().join(" and "))
        };
        out.push(format!("Enters {} ({state}) at plot {i}: {what}", sea_name(g, s)));
    }
    out
}

/// `On Patrol: The Grey Narrows.` for a ship on Patrol, else None.
pub fn patrol_line(g: &Game, u: UnitId) -> Option<String> {
    if !g.units[u].blockading {
        return None;
    }
    let s = g.unit_plot_opt(u).and_then(|q| g.plot_sea(q))?;
    Some(format!("On Patrol: {}.", sea_name(g, s)))
}

/// The Patrol order's rule, bare (the action's tooltip; docs/NAVAL.md, 4.2 to 4.4), lines joined
/// by `[NEWLINE]`.
pub fn patrol_rule(g: &Game) -> String {
    let d = &g.d.defines;
    [
        "A warship on Patrol counts its strength in the sea it stands in. It ends when the ship moves.".to_string(),
        format!("A sea is held by a side with {} its enemies' strength there.", hold_times(g)),
        "Held by an enemy: your lanes through it are cut, your ports on it are blockaded, your supply ships in it are no source.".to_string(),
        format!("A warship on Patrol intercepts enemies that enter its sea, once a turn, at odds of {}% or more.", d.intercept_min_odds),
    ]
    .join("[NEWLINE]")
}

/// What a Patrol order of `ships` on `plot` gives (docs/NAVAL.md, 10): the plot's sea's strengths
/// and state after it, and each sea one of them is on Patrol in now without them (`Bay of Lorda: 0
/// to 0. Open.`). Empty off the seas or without a warship.
pub fn patrol_preview(v: &Viewer, ships: &[UnitId], plot: PlotIdx) -> Vec<String> {
    let g = v.g;
    let Some(s) = g.plot_sea(plot) else { return Vec::new() };
    let ships: Vec<UnitId> = ships.iter().copied().filter(|&u| g.unit_is_warship(u) && !g.unit_is_barbarian(u)).collect();
    if ships.is_empty() {
        return Vec::new();
    }
    let after = g.sea_state_if_patrol(s, v.team, &ships);
    let mut out = vec![format!("{}: {}", sea_name(g, s), result_text(v, &after))];
    let mut left: Vec<SeaId> = Vec::new();
    for &u in &ships {
        let on = if g.units[u].blockading { g.unit_plot_opt(u).and_then(|q| g.plot_sea(q)) } else { None };
        if let Some(o) = on.filter(|&o| o != s && !left.contains(&o)) {
            left.push(o);
        }
    }
    for o in left {
        let st = g.sea_state_known_without(o, v.team, &ships);
        out.push(format!("{}: {}", sea_name(g, o), result_text(v, &st)));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Supply ships and their lanes

/// The supply ships of the human's team at sea (sea units with room for land units), in id order.
fn supply_ships(v: &Viewer) -> Vec<UnitId> {
    let g = v.g;
    let mut out: Vec<UnitId> = g
        .player_ids()
        .filter(|&q| g.players[q].team == v.team)
        .flat_map(|q| g.players[q].units.clone())
        .filter(|&u| g.units.contains(u) && g.units[u].cargo_capacity > 0 && g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && g.unit_domain_cargo(u) == Some(DomainType::DOMAIN_LAND))
        .filter(|&u| g.unit_plot_opt(u).is_some_and(|q| g.plot_is_water(q)))
        .collect();
    out.sort();
    out
}

/// The view's `sea_lanes` (docs/NAVAL.md, 4.6 and 10): the lane of each supply ship of the
/// human's team at sea, as the human can work it out ([`Game::sea_lane_known`]): `{ship, owner,
/// path: [[x, y], ...] (from the ship to the water by the port), port, cut: null | {at (the
/// path's index where it enters the closed sea), sea, holder}}`.
pub fn lanes_json(v: &Viewer) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for u in supply_ships(v) {
        let Some(lane) = g.sea_lane_known(v.team, g.unit_plot(u), v.team) else { continue };
        let path: Vec<Json> = lane
            .path
            .iter()
            .map(|&q| {
                let (x, y) = g.map.xy(q);
                crate::jarr![x, y]
            })
            .collect();
        let cut = lane.cut_in.map(|(s, t)| {
            let i = lane.path.iter().position(|&q| g.plot_sea(q) == Some(s)).unwrap_or(0);
            crate::jobj! { "at" => i as i64, "sea" => sea_name(g, s), "holder" => team_name(v, t) }
        });
        out.push(crate::jobj! { "ship" => u, "owner" => g.units[u].owner, "path" => Json::Arr(path), "port" => port_name(v, lane.port), "cut" => cut });
    }
    Json::Arr(out)
}

/// The words of a supply ship's refusal: `the transport is in Sirens' Water, held by Lanun.` /
/// `the transport's lane is cut in Sirens' Water.` / `the transport has no lane to a port in
/// range.`
pub fn refusal_text(v: &Viewer, r: &SupplyShipRefusal) -> String {
    let g = v.g;
    match *r {
        SupplyShipRefusal::SeaClosed(s, t) => format!("the transport is in {}, held by {}.", sea_name(g, s), team_name(v, t)),
        SupplyShipRefusal::LaneCut(s, _) => format!("the transport's lane is cut in {}.", sea_name(g, s)),
        SupplyShipRefusal::NoLane => "the transport has no lane to a port in range.".to_string(),
    }
}

/// Why no supply ship of the human's team serves `plot` (docs/NAVAL.md, 4.6): its supply ships
/// on `plot` (water) or next to it (land), as the human can work it out: the first one's refusal,
/// or None when there is none or one of them is a source.
pub fn transport_refusal(v: &Viewer, plot: PlotIdx) -> Option<String> {
    let g = v.g;
    let (x, y) = g.map.xy(plot);
    let water = g.plot_is_water(plot);
    let mut first = None;
    for u in supply_ships(v) {
        let (ux, uy) = (g.units[u].x, g.units[u].y);
        let near = if water { (ux, uy) == (x, y) } else { g.plot_distance(ux, uy, x, y) == 1 };
        if !near {
            continue;
        }
        match g.supply_ship_refused_known(v.team, g.unit_plot(u), v.team) {
            None => return None,
            Some(r) => {
                first.get_or_insert(r);
            }
        }
    }
    first.map(|r| refusal_text(v, &r))
}

// ---------------------------------------------------------------------------------------------
// The sea's panel and the Seas screen

/// The ships on Patrol in sea `s` the human sees, by party: the human's team first (always, even
/// with none), then the others by team, an unknown party last; each `(party, strength x100,
/// ships)`.
fn sea_sides(v: &Viewer, s: SeaId) -> Vec<(SeaParty, i32, Vec<UnitId>)> {
    let g = v.g;
    let mut out: Vec<(SeaParty, i32, Vec<UnitId>)> = vec![(Some(v.team), 0, Vec::new())];
    for u in g.sea_patrollers(s) {
        if !v.unit_visible(u) {
            continue;
        }
        let t = g.unit_get_team(u);
        let party = if g.unit_is_hidden_nationality(u) && t != v.team { None } else { Some(t) };
        let x = g.unit_sea_strength_x100(u);
        match out.iter_mut().find(|e| e.0 == party) {
            Some(e) => {
                e.1 += x;
                e.2.push(u);
            }
            None => out.push((party, x, vec![u])),
        }
    }
    let key = |p: &SeaParty| match p {
        Some(t) if *t == v.team => 0,
        Some(t) => 1 + t.0 as i64,
        None => i64::MAX,
    };
    out.sort_by_key(|e| key(&e.0));
    out
}

/// Of `lanes` (through sea `s`), those cut in another sea, counted by that sea (in sea order).
fn cut_elsewhere(s: SeaId, lanes: &[(UnitId, ffh_core::naval::SeaLane)]) -> Vec<(SeaId, usize)> {
    let mut out: Vec<(SeaId, usize)> = Vec::new();
    for (_, l) in lanes {
        if let Some((o, _)) = l.cut_in.filter(|&(o, _)| o != s) {
            match out.iter_mut().find(|e| e.0 == o) {
                Some(e) => e.1 += 1,
                None => out.push((o, 1)),
            }
        }
    }
    out.sort();
    out
}

fn x100(x: i32) -> String {
    if x % 100 == 0 {
        format!("{}", x / 100)
    } else {
        format!("{}.{}", x / 100, (x % 100) / 10)
    }
}

fn unit_name(g: &Game, u: UnitId) -> String {
    let d = g.d;
    d.text.name(&d.units[g.units[u].unit_type].base.description).to_string()
}

/// `sea {id, units}`: the sea's panel (docs/NAVAL.md, 10), for a sea the human has revealed a plot
/// of: `{id, name, x, y, plots, plots_text, state, badge, control, ours, theirs, hold, sides:
/// [{label, own, strength, ships: [text], note, party}], now: [text], if_they: null | {title,
/// lines}, if_you: null | {title, lines}, send}`. `units`: the selection; `send` (null without a
/// warship of the human's among them): `{label, here (Patrol where it stands), turns, x, y,
/// result: [text], units, enabled, why}`: a go-to to the nearest plot of the sea it reaches, then
/// Patrol (the client queues both).
pub fn sea_panel(v: &Viewer, finder: &mut PathFinder, s: SeaId, units: &[UnitId]) -> Result<Json, QueryErr> {
    let g = v.g;
    if s.idx() >= g.map.seas.len() {
        return Err(QueryErr::Gone);
    }
    let plots: Vec<PlotIdx> = g.map.seas[s.idx()].plots.iter().copied().filter(|&q| v.plot_revealed(q)).collect();
    if plots.is_empty() {
        return Err(QueryErr::Gone);
    }
    let all = plots.len() == g.map.seas[s.idx()].plots.len();
    let sm = summary(v, s);
    let st = sm.st;
    let sides = sea_sides(v, s);
    let enemy_parties: Vec<SeaParty> = sides.iter().filter(|e| e.0 != Some(v.team) && e.0.is_none_or(|t| g.at_war(t, v.team))).map(|e| e.0).collect();
    let strongest_enemy = sides.iter().filter(|e| enemy_parties.contains(&e.0)).max_by_key(|e| e.1).map(|e| e.0);
    // What holding takes.
    let mut hold = format!("Held at {} the enemy's strength on patrol.", hold_times(g));
    match st.control {
        SeaControl::HeldByUs if st.theirs > 0 => {
            let who = strongest_enemy.map_or("An enemy".to_string(), |p| capital(&party_name(v, p)));
            hold.push_str(&format!(" {who} needs {}.", hold_need(g, st.ours)));
        }
        SeaControl::HeldByUs => hold.push_str(" No enemy patrols it."),
        SeaControl::HeldBy(_) | SeaControl::Contested(_) => hold.push_str(&format!(" You need {}.", hold_need(g, st.theirs))),
        _ => hold.push_str(" Any warship of yours on Patrol holds it while no enemy patrols."),
    }
    let sides_json: Vec<Json> = sides
        .iter()
        .map(|(p, x, ships)| {
            let own = *p == Some(v.team);
            let label = if own {
                "You".to_string()
            } else {
                let rel = match p {
                    Some(t) if g.at_war(*t, v.team) => " (at war)",
                    Some(_) => " (not at war)",
                    None => " (hidden nationality)",
                };
                format!("{}{rel}", capital(&party_name(v, *p)))
            };
            let list: Vec<Json> = ships.iter().map(|&u| Json::from(format!("{} {}", unit_name(g, u), x100(g.unit_sea_strength_x100(u))))).collect();
            let n = ships.len() as i32;
            let note = if own {
                if n == 0 { "No ship on Patrol.".to_string() } else { format!("{} on Patrol.", plural(n, "ship")) }
            } else {
                format!("{} seen on Patrol. Ships you do not see are not counted.", plural(n, "ship"))
            };
            crate::jobj! { "label" => label, "own" => own, "strength" => *x / 100, "ships" => Json::Arr(list), "note" => note, "party" => p.filter(|&t| t != v.team).map(|t| t.0 as i64) }
        })
        .collect();
    // Effect now.
    let lanes = g.sea_lanes_through(s, v.team, v.team);
    let ports = g.sea_ports(s, v.team);
    let ships_in = supply_ships(v).into_iter().filter(|&u| g.plot_sea(g.unit_plot(u)) == Some(s)).count();
    let mut now = Vec::new();
    // This sea's own effect: a lane is cut here only while this sea is closed to the human; a
    // lane through it cut in another sea is said apart, naming that sea.
    now.push(match (lanes.is_empty(), st.closed()) {
        (true, _) => "Lanes through it: none.".to_string(),
        (false, true) => "Lanes through it: cut here.".to_string(),
        (false, false) => "Lanes through it: open.".to_string(),
    });
    if !st.closed() {
        for (other, n) in cut_elsewhere(s, &lanes) {
            now.push(format!("{n} lane{} through it {} cut in {}.", if n == 1 { "" } else { "s" }, if n == 1 { "is" } else { "are" }, sea_name(g, other)));
        }
    }
    if ports.is_empty() {
        now.push("Your ports on it: none.".to_string());
    } else {
        let blockaded: Vec<String> = ports.iter().filter(|&&c| matches!(g.city_harbour(c), Harbour::Blockaded(_))).map(|&c| crate::view::city_name(g, c)).collect();
        now.push(if blockaded.is_empty() { "Your ports on it: not blockaded.".to_string() } else { format!("Your ports on it: blockaded: {}.", blockaded.join(", ")) });
    }
    let ours_patrol = !sides[0].2.is_empty();
    now.push(match (ours_patrol, !enemy_parties.is_empty()) {
        (true, true) => "Interception: both sides.".to_string(),
        (true, false) => "Interception: by your patrol.".to_string(),
        (false, true) => format!("Interception: by {}.", enemy_parties.iter().map(|&p| party_name(v, p)).collect::<Vec<_>>().join(", ")),
        (false, false) => "Interception: none.".to_string(),
    });
    if st.closed() {
        now.push(if ships_in > 0 { "Your supply ships in it: not a source.".to_string() } else { "Your supply ships in it: none.".to_string() });
    }
    // If an enemy holds it (while it is not closed to the human).
    let if_they = if st.closed() {
        Json::Null
    } else {
        let holder = match st.control {
            SeaControl::Contested(p) => Some(p),
            _ => strongest_enemy,
        };
        let title = holder.map_or("If an enemy holds it".to_string(), |p| format!("If {} holds it", party_name(v, p)));
        let mut lines: Vec<String> = Vec::new();
        for (u, lane) in lanes.iter().filter(|(_, l)| !l.is_cut()) {
            let here = lane.path.iter().filter(|&&q| g.plot_sea(q) == Some(s)).count();
            lines.push(format!("Lane cut: {} to {} ({here} of its {} plots are here).", unit_name(g, *u), port_name(v, lane.port), lane.path.len()));
        }
        let shut = g.sea_ports_blockaded_if_held(s, v.team);
        lines.push(match (shut.is_empty(), ports.is_empty()) {
            (_, true) => "Your ports on it: none.".to_string(),
            (true, false) => "Your ports on it: not blockaded (they touch another open sea).".to_string(),
            (false, false) => format!("Blockaded: {}.", shut.iter().map(|&c| crate::view::city_name(g, c)).collect::<Vec<_>>().join(", ")),
        });
        lines.push(if ships_in > 0 { "Your supply ships in it: not a source.".to_string() } else { "Your supply ships in it: none.".to_string() });
        crate::jobj! { "title" => title, "lines" => lines }
    };
    // If the human holds it.
    let if_you = if st.control == SeaControl::HeldByUs {
        Json::Null
    } else {
        let barb = g.barbarian_team();
        let enemies: Vec<TeamId> = g.team_ids().filter(|&t| t != v.team && t != barb && v.team_met(t) && g.at_war(t, v.team)).collect();
        let who = if enemies.is_empty() { "Enemy".to_string() } else { enemies.iter().map(|&t| team_name(v, t)).collect::<Vec<_>>().join(", ") };
        let mut lines = vec![format!("{who} ships and boats entering it are intercepted.")];
        for &t in &enemies {
            let theirs: Vec<CityId> = g.cities.iter().filter(|(c, city)| g.players[city.owner].team == t && v.city_revealed(*c) && g.city_seas(*c).contains(&s)).map(|(c, _)| c).collect();
            let name = team_name(v, t);
            if theirs.is_empty() {
                lines.push(format!("{name} ports on it: none."));
                continue;
            }
            let shut: Vec<String> = theirs.iter().filter(|&&c| g.city_seas(c).iter().all(|&o| o == s || g.sea_closed_known(o, t, v.team).is_some())).map(|&c| crate::view::city_name(g, c)).collect();
            lines.push(if shut.is_empty() { format!("{name} ports on it: not blockaded (they touch another open sea).") } else { format!("{name} ports blockaded: {}.", shut.join(", ")) });
        }
        crate::jobj! { "title" => "If you hold it", "lines" => lines }
    };
    let send = send_json(v, finder, s, &plots, units);
    let (x, y) = g.map.xy(label_plot(g, &plots));
    Ok(crate::jobj! {
        "id" => s.0 as i64,
        "name" => sea_name(g, s),
        "x" => x,
        "y" => y,
        "plots" => plots.len() as i64,
        "plots_text" => format!("{} plots{}", plots.len(), if all { "" } else { " seen" }),
        "state" => sm.state,
        "badge" => sm.badge,
        "control" => sm.control,
        "ours" => st.ours,
        "theirs" => st.theirs,
        "hold" => hold,
        "sides" => Json::Arr(sides_json),
        "now" => now,
        "if_they" => if_they,
        "if_you" => if_you,
        "send" => send,
    })
}

/// The panel's `send` for the selection `units` ([`sea_panel`]).
fn send_json(v: &Viewer, finder: &mut PathFinder, s: SeaId, plots: &[PlotIdx], units: &[UnitId]) -> Json {
    let g = v.g;
    let ships: Vec<UnitId> = units.iter().copied().filter(|&u| g.units.get(u).is_some_and(|x| x.owner == v.h) && g.unit_is_warship(u) && !g.unit_is_barbarian(u)).collect();
    if ships.is_empty() {
        return Json::Null;
    }
    let who = if ships.len() == 1 { format!("the {}", unit_name(g, ships[0])) } else { format!("{} ships", ships.len()) };
    let from = g.unit_plot(ships[0]);
    let (fx, fy) = g.map.xy(from);
    let ids = Json::Arr(ships.iter().map(|&u| Json::from(u)).collect());
    let here = g.plot_sea(from) == Some(s) && ships.iter().all(|&u| g.unit_plot(u) == from);
    if here && ships.iter().all(|&u| g.units[u].blockading) {
        return crate::jobj! {
            "label" => format!("{} is on Patrol here", capital(&who)), "here" => true, "turns" => 0, "x" => fx, "y" => fy,
            "result" => Json::Arr(Vec::new()), "units" => ids, "enabled" => false, "why" => "On Patrol here.",
        };
    }
    if here && ships.iter().all(|&u| g.unit_can_plunder(u, from, false)) {
        let can = crate::selection::can_start_mission(g, &ships, MissionType::MISSION_PLUNDER, -1, -1, from, false);
        return crate::jobj! {
            "label" => format!("Patrol here with {who}"), "here" => true, "turns" => 0, "x" => fx, "y" => fy,
            "result" => patrol_preview(v, &ships, from), "units" => ids, "enabled" => can,
            "why" => if can { Json::Null } else { Json::from("Patrol is not possible now.") },
        };
    }
    let Some(grp) = crate::selection::mirrors_group(g, &ships).or(g.units[ships[0]].group) else { return Json::Null };
    // The plots of the sea it could patrol, nearest first; the fewest turns among the first ones
    // it reaches.
    let mut cands: Vec<(i32, PlotIdx)> = plots
        .iter()
        .copied()
        .filter(|&q| g.map.plots[q].plot_city.is_none() && !g.plot_is_impassable(q))
        .map(|q| {
            let (x, y) = g.map.xy(q);
            (g.plot_distance(fx, fy, x, y), q)
        })
        .collect();
    cands.sort();
    finder.set_group(Some(grp));
    let mut best: Option<(i32, i32, PlotIdx)> = None;
    for (k, (dist, q)) in cands.into_iter().enumerate() {
        if k >= 12 || best.is_some_and(|b| dist > b.1 + 2) {
            break;
        }
        let (x, y) = g.map.xy(q);
        if !finder.generate_path(g, fx, fy, x, y, false, 0) {
            continue;
        }
        let Some(turns) = finder.path_turns() else { continue };
        if best.is_none_or(|b| turns < b.0) {
            best = Some((turns, dist, q));
        }
    }
    let Some((turns, _, q)) = best else {
        return crate::jobj! {
            "label" => format!("Send {who} to patrol here"), "here" => false, "turns" => Json::Null, "x" => Json::Null, "y" => Json::Null,
            "result" => patrol_preview(v, &ships, plots[0]), "units" => ids, "enabled" => false, "why" => "No path to this sea.",
        };
    };
    let (x, y) = g.map.xy(q);
    crate::jobj! {
        "label" => format!("Send {who} to patrol here ({})", plural(turns.max(1), "turn")),
        "here" => false,
        "turns" => turns,
        "x" => x,
        "y" => y,
        "result" => patrol_preview(v, &ships, q),
        "units" => ids,
        "enabled" => true,
        "why" => Json::Null,
    }
}

/// `seas`: the Seas screen (docs/NAVAL.md, 10): `{seas: [row]}`, one row per sea the human has
/// revealed a plot of, `{id, name, x, y, state, badge, control, ours, theirs, enemy, ports, lanes,
/// lanes_cut}` (`enemy`: the strength of the party the control names, with its name; "0"; or "not
/// seen" while no plot of the sea is visible and nothing is known there; `ports`: the human's
/// ports on it, a blockaded one marked; `lanes`: its supply ships' lanes through it, open and
/// cut).
pub fn seas_screen(v: &Viewer) -> Json {
    let g = v.g;
    let mut rows = Vec::new();
    for (s, plots) in seen_seas(v) {
        let sm = summary(v, s);
        let seen = plots.iter().any(|&q| v.plot_visible(q));
        let enemy = match sm.party {
            Some(p) if sm.st.theirs > 0 && !matches!(sm.st.control, SeaControl::PatrolledBy(_)) => format!("{} {}", sm.st.theirs, capital(&party_name(v, p))),
            _ if !seen => "not seen".to_string(),
            _ => "0".to_string(),
        };
        let ports: Vec<String> = g
            .sea_ports(s, v.team)
            .into_iter()
            .filter(|&c| g.cities[c].owner == v.h)
            .map(|c| {
                let name = crate::view::city_name(g, c);
                if matches!(g.city_harbour(c), Harbour::Blockaded(_)) {
                    format!("{name}: blockaded")
                } else {
                    name
                }
            })
            .collect();
        let lanes = g.sea_lanes_through(s, v.team, v.team);
        // Cut here only while this sea is closed to the human; lanes cut in another sea apart.
        let closed = sm.st.closed();
        let elsewhere: usize = if closed { 0 } else { cut_elsewhere(s, &lanes).iter().map(|e| e.1).sum() };
        let cut = if closed { lanes.len() } else { 0 };
        let open = lanes.len() - cut - elsewhere;
        let mut parts = Vec::new();
        if open > 0 {
            parts.push(format!("{open} open"));
        }
        if cut > 0 {
            parts.push(format!("{cut} cut"));
        }
        if elsewhere > 0 {
            parts.push(format!("{elsewhere} cut elsewhere"));
        }
        let lanes_text = if parts.is_empty() { "none".to_string() } else { parts.join(", ") };
        let (x, y) = g.map.xy(label_plot(g, &plots));
        rows.push(crate::jobj! {
            "id" => s.0 as i64,
            "name" => sea_name(g, s),
            "x" => x,
            "y" => y,
            "state" => sm.state,
            "badge" => sm.badge,
            "control" => sm.control,
            "ours" => sm.st.ours,
            "theirs" => sm.st.theirs,
            "enemy" => enemy,
            "ports" => if ports.is_empty() { "none".to_string() } else { ports.join(", ") },
            "lanes" => lanes_text,
            "lanes_cut" => cut as i64,
        });
    }
    crate::jobj! { "seas" => Json::Arr(rows) }
}
