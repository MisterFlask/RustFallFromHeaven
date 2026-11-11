//! Outposts (docs/NAVAL.md, sections 8 and 9; a rules change's AI, kept small: docs/AI.md).
//!
//! - **Where**: an outpost site ([`site`]) is a plot where the rules let us found one, unowned
//!   (or ours), whose water is in our range, holding within its 8 plots (or on itself) a revealed
//!   resource we have none of, any far-shore resource, or a mana node; the most such resources,
//!   then the nearest to our ports, then the lower plot index. Never one another colonist of ours
//!   is bound for (sites within 3 plots of it are skipped).
//! - **Production** ([`production`]): a coastal city that is safe trains a Colonist when a site
//!   exists, no colonist of ours is alive or in training, we hold fewer than [`MAX_OUTPOSTS`]
//!   outposts and at least [`MIN_CITIES`] cities.
//! - **The colonist** ([`colonist`]): on its site it founds the outpost; else it goes there in
//!   boats (`MOVE_BOATS`, over water in our range) when the way is open: the lane from the site to
//!   our ports, as we see it, is not cut and passes no sea where an enemy patrols that we see, and
//!   no enemy warship we see is within 3 plots; else it waits in port. (Not built: the transport
//!   for a colonist, as settlers go.)
//! - **Raising** ([`turn`]): an outpost at its full population whose lane is open is raised to a
//!   city while we are at peace with every civilization and hold three times the cost in gold;
//!   one a turn.
//! - Its garrison and its lane's patrols: the defense and patrol logic count outposts as any city
//!   (`naval::patrol_target`'s third need is our outposts' lanes).

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::outposts::FOUND_OUTPOST;
use ffh_data::*;

use crate::production::Choice;
use crate::units::{alive, can_start, move_to, push, skip};
use crate::view::View;

/// The most outposts the AI holds.
pub const MAX_OUTPOSTS: i32 = 3;
/// The cities (not outposts) we hold before we train a colonist: the home land first.
pub const MIN_CITIES: i32 = 3;

/// Whether unit type `t` is a colonist (founds outposts, not cities).
fn is_colonist_type(d: &GameData, t: UnitType) -> bool {
    d.units[t].found_outpost && !d.units[t].found
}

/// The colonist city `c` can train, if any.
fn colonist_type(g: &Game, c: CityId) -> Option<UnitType> {
    g.d.units.keys().find(|&t| is_colonist_type(g.d, t) && g.city_can_train(c, t, false, false, false, false))
}

/// Our colonists alive, and in training.
fn colonists(g: &Game, p: PlayerId) -> (Vec<UnitId>, i32) {
    let alive: Vec<UnitId> = g.players[p].units.iter().copied().filter(|&u| g.units.contains(u) && is_colonist_type(g.d, g.units[u].unit_type)).collect();
    let training = g.players[p]
        .cities
        .iter()
        .filter(|&&c| g.cities.contains(c))
        .flat_map(|&c| g.cities[c].order_queue.iter())
        .filter(|o| o.order_type == OrderType::ORDER_TRAIN && o.data1 >= 0 && is_colonist_type(g.d, UnitType::from_usize(o.data1 as usize)))
        .count() as i32;
    (alive, training)
}

/// What `q`'s outpost would bring us: per resource on it or its 8 plots (revealed to us), one for
/// a resource we have none of, a far-shore resource or a mana node.
fn site_value(g: &Game, p: PlayerId, q: PlotIdx) -> i32 {
    let team = g.players[p].team;
    let mana_class = (opt_from_i32::<BonusClassType>(g.d.defines.bonusclass_mana), opt_from_i32::<BonusClassType>(g.d.defines.bonusclass_mana_raw));
    let (x, y) = g.map.xy(q);
    let mut value = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let Some(r) = g.plot_xy(x, y, dx, dy) else { continue };
            if g.map.plots[r].owner.is_some_and(|o| o != p) {
                continue;
            }
            let Some(b) = g.plot_get_bonus_type(r, Some(team)).filter(|_| g.map.plots[r].revealed[team]) else { continue };
            let info = &g.d.bonuses[b];
            let mana = info.bonus_class_type.is_some() && (info.bonus_class_type == mana_class.0 || info.bonus_class_type == mana_class.1);
            if info.far_shore || mana || g.player_get_num_available_bonuses(p, b) == 0 {
                value += 1;
            }
        }
    }
    value
}

/// Whether some water plot next to `q` is in `team`'s range.
fn water_in_range(g: &Game, team: TeamId, q: PlotIdx) -> bool {
    let (x, y) = g.map.xy(q);
    (0..DirectionType::NUM).filter_map(|i| g.plot_direction(x, y, DirectionType::from_usize(i))).any(|w| g.plot_is_water(w) && g.plot_in_sea_range(team, w))
}

/// The best outpost site for us (module comment); `except`: a colonist of ours whose own
/// destination this is (another colonist's surroundings are skipped).
pub fn site(g: &Game, p: PlayerId, except: Option<UnitId>) -> Option<PlotIdx> {
    let team = g.players[p].team;
    let capital = g.player_get_capital_city(p)?;
    let home = g.map.plots[g.city_plot(capital)].area;
    let ports: Vec<(i32, i32)> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c) && g.city_is_port(c)).map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
    if ports.is_empty() {
        return None;
    }
    let others: Vec<PlotIdx> = colonists(g, p).0.into_iter().filter(|&u| Some(u) != except).map(|u| g.unit_plot(u)).collect();
    let min_water = g.d.defines.min_water_size_for_ocean;
    let mut best: Option<(i32, i32, PlotIdx)> = None;
    for q in g.map.plot_ids() {
        let plot = &g.map.plots[q];
        if !plot.revealed[team] || g.plot_is_water(q) || plot.area == home || plot.owner.is_some_and(|o| o != p) || !g.plot_is_coastal_land(q, min_water) {
            continue;
        }
        let (x, y) = g.map.xy(q);
        if others.iter().any(|&o| {
            let (ox, oy) = g.map.xy(o);
            g.plot_distance(x, y, ox, oy) <= 3
        }) {
            continue;
        }
        let value = site_value(g, p, q);
        if value <= 0 || !water_in_range(g, team, q) || !g.player_can_found_outpost(p, x, y, false) {
            continue;
        }
        let dist = ports.iter().map(|&(px, py)| g.plot_distance(x, y, px, py)).min().unwrap_or(i32::MAX);
        let key = (value, -dist, q);
        if best.is_none_or(|b| (key.0, key.1) > (b.0, b.1) || ((key.0, key.1) == (b.0, b.1) && q < b.2)) {
            best = Some(key);
        }
    }
    best.map(|b| b.2)
}

/// What coastal city `c` trains for an outpost (module comment), if anything.
pub fn production(g: &Game, c: CityId) -> Option<Choice> {
    let p = g.cities[c].owner;
    if g.player_is_barbarian(p) || !g.city_is_port(c) || g.player_get_num_outposts(p) >= MAX_OUTPOSTS || g.player_num_real_cities(p) < MIN_CITIES {
        return None;
    }
    let (alive, training) = colonists(g, p);
    if !alive.is_empty() || training > 0 {
        return None;
    }
    let t = colonist_type(g, c)?;
    let at = site(g, p, None);
    crate::trace!(g, p, "outposts: {} asks for a site: {:?}", g.cities[c].name, at.map(|q| g.map.xy(q)));
    at?;
    Some(Choice::Train(t, UnitAIType::UNITAI_UNKNOWN))
}

/// Whether the way in boats to `site` is open to `team` as it sees it (module comment).
fn boats_way_open(g: &Game, team: TeamId, here: PlotIdx, site: PlotIdx) -> bool {
    let Some(lane) = g.sea_lane_known(team, site, team) else { return false };
    if lane.is_cut() {
        return false;
    }
    let mut seas: Vec<SeaId> = lane.path.iter().filter_map(|&w| g.map.plots[w].sea).collect();
    seas.sort();
    seas.dedup();
    if seas.iter().any(|&s| g.sea_state_known(s, team).theirs > 0) {
        return false;
    }
    let (x, y) = g.map.xy(here);
    for dx in -3..=3 {
        for dy in -3..=3 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            if g.plot_is_visible(q, team) && g.map.plots[q].units.iter().any(|&o| g.unit_is_warship(o) && g.unit_is_enemy(o, Some(team), Some(q)) && !g.unit_is_invisible(o, team, false)) {
                return false;
            }
        }
    }
    true
}

/// A colonist of ours (module comment). False if `u` is no colonist; true: its turn is decided.
pub fn colonist(g: &mut Game, v: &View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || !g.units.contains(u) || !is_colonist_type(g.d, g.units[u].unit_type) {
        return false;
    }
    let p = v.p;
    let team = v.team;
    let here = g.unit_plot(u);
    if !g.unit_is_in_boats(u) && g.unit_can_found_outpost(u, here, false) && site_value(g, p, here) > 0 && can_start(g, grp, MissionType::MISSION_FOUND, FOUND_OUTPOST, -1) {
        crate::trace!(g, p, "colonist {:?} founds an outpost at {:?}", u, g.map.xy(here));
        g.tel_decision(p, "outpost", "found", "site", 0, String::new);
        push(g, grp, MissionType::MISSION_FOUND, FOUND_OUTPOST, -1, 0, false);
        return true;
    }
    let Some(target) = site(g, p, Some(u)) else {
        skip(g, grp);
        return true;
    };
    if g.unit_is_in_boats(u) || boats_way_open(g, team, here, target) {
        if g.group_find_path(grp, here, target, MOVE_BOATS | MOVE_IGNORE_DANGER).is_some() {
            crate::trace!(g, p, "colonist {:?} sails in boats to {:?}", u, g.map.xy(target));
            move_to(g, grp, target, MOVE_BOATS | MOVE_IGNORE_DANGER);
            if alive(g, grp) && g.units.contains(u) && g.group_ready_to_move(grp, true) {
                skip(g, grp);
            }
            return true;
        }
    }
    skip(g, grp);
    true
}

/// Once a turn (module comment): an outpost raised to a city when rich and at peace.
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.player_is_barbarian(p) || g.player_get_num_outposts(p) == 0 {
        return;
    }
    let team = g.players[p].team;
    let at_war = g.civ_player_ids().any(|q| g.players[q].alive && g.at_war(team, g.players[q].team));
    if at_war || g.players[p].gold < 3 * g.outpost_raise_cost() {
        return;
    }
    let mut ready: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c) && g.city_outpost_full(c) && !g.cities[c].lane.is_empty()).collect();
    ready.sort();
    if let Some(&c) = ready.first() {
        crate::trace!(g, p, "raises outpost {}", g.cities[c].name);
        g.tel_decision(p, "outpost", "raise", "rich", 0, String::new);
        g.player_raise_outpost(p, c);
    }
}
