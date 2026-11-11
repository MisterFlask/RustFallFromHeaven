//! The sea, phase 1 (docs/NAVAL.md, sections 3 and 10), as the human sees it: the range per plot
//! for the Sea layer (the map's `sea` array), the team's range (`me.sea_range`), a port's mark on
//! the cities, the path preview's lines (in boats, out of range), the unit panel's lines (range,
//! wear, in boats) and an army's supply at sea. Text is bare (CLAUDE.md, "Rules text and
//! tooltips").

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::Viewer;

/// What the `sea` map array needs, read once per map: the team's range, or `None` for a team the
/// range rule does not apply to (no city: `Game::team_sea_exempt`).
#[derive(Clone, Copy, Debug)]
pub struct SeaCtx {
    pub range: Option<i32>,
}

impl SeaCtx {
    pub fn new(v: &Viewer) -> SeaCtx {
        SeaCtx { range: team_range(v.g, v.team) }
    }

    /// The `sea` value of revealed plot `p` (the caller clips unrevealed plots): -1 on land or
    /// without a range rule; on water, 0 out of range, else the steps from the nearest port (1 next
    /// to it, at most the range). Beyond the range only "out" is told, not how far.
    pub fn value(&self, v: &Viewer, p: PlotIdx) -> i32 {
        let Some(range) = self.range else { return -1 };
        if !v.g.plot_is_water(p) {
            return -1;
        }
        match v.g.plot_sea_range(v.team, p) {
            Some(d) if d <= range => d,
            _ => 0,
        }
    }
}

/// `team`'s range at sea, `None` when the rule does not apply to it (no city).
pub fn team_range(g: &Game, team: TeamId) -> Option<i32> {
    (!g.team_sea_exempt(team)).then(|| g.team_sea_range(team))
}

/// Whether city `c` is a port of the human's team (a city next to water of a team whose ports are
/// its ports), while the range rule applies to the human: a port's tooltip on the Sea layer names
/// the range it gives.
pub fn is_own_port(v: &Viewer, c: CityId) -> bool {
    let g = v.g;
    !g.team_sea_exempt(v.team) && g.city_is_port(c) && g.team_is_port_team(v.team, g.city_team(c))
}

fn plural(n: i32, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

/// The fields of one of the human's own units in the view: `boats` (in boats now) and `wear` (its
/// wear in percent of strength, 0 for none or not a warship).
pub fn unit_fields(g: &Game, u: UnitId, j: &mut Json) {
    j.set("boats", g.unit_is_in_boats(u));
    j.set("wear", g.unit_sea_wear_percent(u));
}

/// The unit panel's sea lines for one of the human's units (docs/NAVAL.md, section 10), or null:
/// `{boats, range, of, out, wear, turns, free_turns, lines}` (`range`: the plot's steps from the
/// nearest port, null on land or beyond reach; `of`: the team's range, null without a range rule;
/// `out`: out of range now; `turns`: its turns at sea for wear, warships only).
pub fn unit_panel(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let d = &g.d.defines;
    let Some(plot) = g.unit_plot_opt(u) else { return Json::Null };
    let sea_unit = g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && !g.unit_is_animal(u);
    let boats = g.unit_is_in_boats(u);
    if !sea_unit && !boats {
        return Json::Null;
    }
    let team = g.unit_get_team(u);
    let of = team_range(g, team);
    let mut lines: Vec<String> = Vec::new();
    if boats {
        lines.push(format!("In boats: strength {}, cannot attack, does not heal.", d.embarked_strength));
    }
    let range = g.plot_sea_range(team, plot);
    let out = g.unit_is_out_of_sea_range(u);
    if let Some(of) = of {
        if out {
            lines.push(format!("Out of range: -{}% HP a turn.", d.sea_attrition_percent));
        } else if g.unit_is_cargo(u) {
            // Carried: no range of its own.
        } else if !g.plot_is_water(plot) {
            lines.push(format!("Range: in port ({}).", plural(of, "plot")));
        } else if let Some(r) = range {
            lines.push(format!("Range: in range ({r} of {of})."));
        }
    }
    let warship = g.unit_is_warship(u) && !g.unit_is_barbarian(u);
    let wear = g.unit_sea_wear_percent(u);
    if warship {
        lines.push(if wear > 0 {
            format!("Wear: -{wear}% strength. Enter a port to clear.")
        } else {
            format!("Wear: none (turn {} of {}).", g.units[u].sea_turns, d.sea_wear_free_turns)
        });
    }
    crate::jobj! {
        "boats" => boats,
        "range" => range,
        "of" => of,
        "out" => out,
        "wear" => wear,
        "turns" => if warship { Json::from(g.units[u].sea_turns) } else { Json::Null },
        "free_turns" => d.sea_wear_free_turns,
        "lines" => lines,
    }
}

/// The path preview's sea part (docs/NAVAL.md, section 10) for the selection `units` along `nodes`
/// (`(x, y, turn)` from the start plot, index 0, to the target), or null when the path has none:
/// `boats_from` (the first plot index where a selected land unit would be in boats: a water plot
/// where no ship of its team has room for it), `boats_moves` (the water plots a turn: the fewest
/// of those units'), `boats_turns` (the turns from the first such plot's turn of arrival to the
/// last one's), `out_from` (the first water plot the human has revealed that is out of range, for a
/// selection the range rule would apply to there), `attrition` (`SEA_ATTRITION_PERCENT`),
/// `strength` (`EMBARKED_STRENGTH`) and `lines`, the preview's words.
pub fn path_json(v: &Viewer, units: &[UnitId], nodes: &[(i32, i32, i32)]) -> Json {
    let g = v.g;
    let d = &g.d.defines;
    let land: Vec<UnitId> = units.iter().copied().filter(|&u| g.unit_can_use_boats(u)).collect();
    let ships = units.iter().any(|&u| g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && !g.unit_is_animal(u) && !g.unit_is_barbarian(u));
    let ruled = !g.team_sea_exempt(v.team);
    let mut boats_from: Option<usize> = None;
    let mut boats_turns: Option<(i32, i32)> = None;
    let mut out_from: Option<usize> = None;
    for (i, &(x, y, turn)) in nodes.iter().enumerate() {
        let Some(p) = g.map.plot_xy(x, y) else { continue };
        if !g.plot_is_water(p) {
            continue;
        }
        let boats = land.iter().any(|&u| g.unit_needs_boats(u, p) && !(i > 0 && g.unit_can_load(u, p)) && (i > 0 || g.unit_is_in_boats(u)));
        if boats {
            boats_from.get_or_insert(i);
            boats_turns = Some(match boats_turns {
                None => (turn, turn),
                Some((a, _)) => (a, turn),
            });
        }
        if ruled && out_from.is_none() && (boats || ships) && v.plot_revealed(p) && !g.plot_in_sea_range(v.team, p) {
            out_from = Some(i);
        }
    }
    if boats_from.is_none() && out_from.is_none() {
        return Json::Null;
    }
    let moves = land.iter().map(|&u| g.unit_embarked_moves(u)).min().unwrap_or(1);
    let turns = boats_turns.map(|(a, b)| (b - a + 1).max(1));
    let at = |i: usize| if i == 0 { "now".to_string() } else { format!("from plot {i}") };
    let mut lines = Vec::new();
    if let (Some(i), Some(t)) = (boats_from, turns) {
        lines.push(format!("In boats {}: strength {}, {} a turn. {}.", at(i), d.embarked_strength, plural(moves, "plot"), plural(t, "turn")));
    }
    if let Some(i) = out_from {
        lines.push(format!("Out of range {}: -{}% HP a turn.", at(i), d.sea_attrition_percent));
    }
    crate::jobj! {
        "boats_from" => boats_from,
        "boats_moves" => boats_from.map(|_| moves),
        "boats_turns" => turns,
        "out_from" => out_from,
        "attrition" => d.sea_attrition_percent,
        "strength" => d.embarked_strength,
        "lines" => lines,
    }
}

/// The name of the port an army at sea is supplied from, as the human may name it: the city's
/// name when it has revealed it, else "a port".
pub fn port_name(v: &Viewer, c: CityId) -> String {
    if v.g.cities.contains(c) && (v.city_revealed(c) || v.g.cities[c].owner == v.h) {
        crate::view::city_name(v.g, c)
    } else {
        "a port".to_string()
    }
}
