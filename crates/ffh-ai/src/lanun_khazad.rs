//! The AI's use of the Lanun's and the Khazad's world spells (the rules:
//! `ffh_core::ffh::world_ritual_spells::lanun_khazad`). A rules change, so the choices are ad hoc
//! (CLAUDE.md, "AI changes"):
//!
//! - **Raging Seas:** with its cast pending and at war with a civilization (`world_spells::ready`), at the
//!   water plot whose storm would cover the most enemy strength (units of teams at war with us on
//!   plots we see that the storm strikes; ties: nearest an enemy coastal city); with no enemy unit
//!   in reach, next to the enemy coastal city nearest a city of ours.
//! - **Steer the Storm:** each turn, to the reachable centre covering the most enemy strength, when
//!   it covers more than where the storm stands.
//! - **The Delve:** with its cast pending, by a unit of ours already on a valid hill (a hill city's
//!   garrison), to the valid hill nearest our city farthest from that unit; of the units that can,
//!   the one whose tunnel is longest.
//! - **Enter the Delve** (every AI, the barbarians included): a land unit on a tunnel's end at the
//!   start of its player's turn, with all its moves, whose group's move order leads at least
//!   [`DELVE_SHORTCUT`] plots closer from the other end, travels.

use ffh_core::ffh::world_ritual_spells::lanun_khazad as lk;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::view::View;

/// Enter the Delve: how many plots closer to its destination the other end must be.
pub const DELVE_SHORTCUT: i32 = 4;

/// Once a turn, after the empire decisions: the storm and the tunnels.
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian || !g.players[v.p].alive {
        return;
    }
    raging_seas(g, v.p);
    steer(g, v.p);
    the_delve(g, v.p);
}

fn distance(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let (ax, ay) = g.map.xy(a);
    let (bx, by) = g.map.xy(b);
    g.plot_distance(ax, ay, bx, by)
}

/// The first unit of `p` (by id) that can cast `spell` now, without asking for its targets.
fn caster(g: &Game, p: PlayerId, spell: SpellType) -> Option<UnitId> {
    let mut units = g.players[p].units.clone();
    units.sort();
    units.into_iter().find(|&u| g.units.contains(u) && g.unit_march(u).is_none() && g.unit_can_cast_untargeted(u, spell, false))
}

/// Per plot: the strength of the units of teams at war with `p`'s that a storm would strike there,
/// on the plots `p`'s team sees (`None`: no enemy in sight anywhere).
fn enemy_strength(g: &Game, p: PlayerId) -> Option<Vec<i32>> {
    let team = g.players[p].team;
    let mut out = vec![0; g.map.plots.len()];
    let mut any = false;
    for t in g.map.plot_ids() {
        if !g.plot_is_visible(t, team) || !g.plot_storm_strikes(t) {
            continue;
        }
        for &u in &g.map.plots[t].units {
            let other = g.unit_get_team(u);
            if other != team && g.at_war(team, other) && !g.unit_is_invisible(u, team, false) {
                out[t.idx()] += g.unit_base_combat_str(u).max(1);
                any = true;
            }
        }
    }
    any.then_some(out)
}

/// The enemy strength a storm centred on `centre` covers.
fn covered(g: &Game, strength: &[i32], centre: PlotIdx) -> i32 {
    lk::plots_within(g, centre, lk::STORM_RADIUS).into_iter().map(|t| strength[t.idx()]).sum()
}

/// The coastal cities of the civilizations at war with `p`'s team that it has revealed.
fn enemy_coastal_cities(g: &Game, p: PlayerId) -> Vec<CityId> {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    let mut out: Vec<CityId> = g
        .cities
        .iter()
        .map(|(c, _)| c)
        .filter(|&c| {
            let ct = g.city_team(c);
            ct != team && ct != barb && g.at_war(team, ct) && g.city_is_coastal(c, 1) && g.map.plots[g.city_plot(c)].revealed[team]
        })
        .collect();
    out.sort();
    out
}

fn raging_seas(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_RAGING_SEAS") else { return };
    if !crate::world_spells::ready(g, p, spell) {
        return;
    }
    let Some(u) = caster(g, p, spell) else { return };
    let targets: Vec<PlotIdx> = g.unit_spell_targets(u, spell).into_iter().filter_map(|t| if let SpellTarget::Plot(q) = t { Some(q) } else { None }).collect();
    if targets.is_empty() {
        return;
    }
    let enemy_cities = enemy_coastal_cities(g, p);
    let to_city = |q: PlotIdx| enemy_cities.iter().map(|&c| distance(g, q, g.city_plot(c))).min().unwrap_or(i32::MAX);
    let mut choice = None;
    if let Some(strength) = enemy_strength(g, p) {
        // The most enemy strength, then nearest an enemy coastal city, then plot order.
        choice = targets.iter().copied().map(|q| (covered(g, &strength, q), q)).filter(|&(s, _)| s > 0).min_by_key(|&(s, q)| (std::cmp::Reverse(s), to_city(q), q)).map(|(_, q)| q);
    }
    if choice.is_none() {
        // Next to the enemy coastal city nearest a city of ours.
        let ours: Vec<PlotIdx> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| g.city_plot(c)).collect();
        let near_us = |c: CityId| ours.iter().map(|&o| distance(g, o, g.city_plot(c))).min().unwrap_or(i32::MAX);
        if let Some(c) = enemy_cities.iter().copied().min_by_key(|&c| (near_us(c), c)) {
            let at = g.city_plot(c);
            choice = targets.iter().copied().filter(|&q| distance(g, q, at) == 1).min();
        }
    }
    let Some(q) = choice else { return };
    let (x, y) = g.map.xy(q);
    crate::trace!(g, p, "world spell SPELL_RAGING_SEAS at ({}, {})", x, y);
    g.tel_cast_reason(u, "world_spell", 0);
    g.unit_cast_at(u, spell, SpellTarget::Plot(q));
}

fn steer(g: &mut Game, p: PlayerId) {
    let Some(i) = g.player_storm(p) else { return };
    let Some(spell) = g.d.ty::<SpellType>("SPELL_STEER_THE_STORM") else { return };
    let Some(strength) = enemy_strength(g, p) else { return };
    let Some(u) = caster(g, p, spell) else { return };
    let here = covered(g, &strength, g.area_effects[i].plot);
    let best = g
        .unit_spell_targets(u, spell)
        .into_iter()
        .filter_map(|t| if let SpellTarget::Plot(q) = t { Some(q) } else { None })
        .map(|q| (covered(g, &strength, q), q))
        .min_by_key(|&(s, q)| (std::cmp::Reverse(s), q));
    let Some((s, q)) = best else { return };
    if s <= here {
        return;
    }
    let (x, y) = g.map.xy(q);
    crate::trace!(g, p, "steer the storm to ({}, {}): {} -> {}", x, y, here, s);
    g.tel_cast_reason(u, "world_action", 0);
    g.unit_cast_at(u, spell, SpellTarget::Plot(q));
}

fn the_delve(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_THE_DELVE") else { return };
    if !crate::world_spells::ready(g, p, spell) {
        return;
    }
    let cities: Vec<PlotIdx> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| g.city_plot(c)).collect();
    let mut units = g.players[p].units.clone();
    units.sort();
    // (length, unit, target): the longest tunnel.
    let mut best: Option<(i32, UnitId, PlotIdx)> = None;
    for u in units {
        if !g.units.contains(u) || g.unit_march(u).is_some() || !g.unit_can_cast_untargeted(u, spell, false) {
            continue;
        }
        let at = g.unit_plot(u);
        let Some(far) = cities.iter().copied().max_by_key(|&c| (distance(g, at, c), std::cmp::Reverse(c))) else { continue };
        let target = g
            .unit_spell_targets(u, spell)
            .into_iter()
            .filter_map(|t| if let SpellTarget::Plot(q) = t { Some(q) } else { None })
            .min_by_key(|&q| (distance(g, q, far), q));
        let Some(q) = target else { continue };
        let len = distance(g, at, q);
        if best.is_none_or(|(l, _, _)| len > l) {
            best = Some((len, u, q));
        }
    }
    let Some((_, u, q)) = best else { return };
    let (x, y) = g.map.xy(q);
    crate::trace!(g, p, "world spell SPELL_THE_DELVE to ({}, {})", x, y);
    g.tel_cast_reason(u, "world_spell", 0);
    g.unit_cast_at(u, spell, SpellTarget::Plot(q));
}

/// At the start of player `p`'s turn (before its units play): its land units on a tunnel's end
/// whose group's move order is [`DELVE_SHORTCUT`] plots closer from the other end take the tunnel.
pub fn delve_travel(g: &mut Game, p: PlayerId) {
    if g.delves.is_empty() {
        return;
    }
    let Some(spell) = g.d.ty::<SpellType>("SPELL_ENTER_THE_DELVE") else { return };
    let mut units = g.players[p].units.clone();
    units.sort();
    for u in units {
        if !g.units.contains(u) {
            continue;
        }
        let Some(at) = g.unit_plot_opt(u) else { continue };
        let Some(other) = g.plot_delve_end(at) else { continue };
        let Some(grp) = g.units[u].group.filter(|&grp| g.groups.contains(grp)) else { continue };
        let dest = g.groups[grp].mission_queue.first().filter(|m| matches!(m.mission_type, MissionType::MISSION_MOVE_TO | MissionType::MISSION_ROUTE_TO)).and_then(|m| g.map.plot_xy(m.data1, m.data2));
        let Some(dest) = dest else { continue };
        if distance(g, at, dest) - distance(g, other, dest) < DELVE_SHORTCUT || !g.unit_can_cast(u, spell, false) {
            continue;
        }
        crate::trace!(g, p, "enter the delve {} -> {:?}", g.d.units[g.units[u].unit_type].base.type_, g.map.xy(other));
        g.tel_cast_reason(u, "delve", 0);
        g.unit_cast(u, spell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game_with;

    const US: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let t = g.d.t::<UnitType>(name);
        g.player_init_unit(p, t, x, y, None, None)
    }

    fn plot(g: &Game, x: i32, y: i32) -> PlotIdx {
        g.map.plot_xy(x, y).expect("on the map")
    }

    fn play(g: &mut Game) {
        let v = View::new(g, US);
        turn(g, &v);
    }

    #[test]
    fn the_lanun_raise_the_storm_over_the_enemy_and_steer_it_after_them() {
        let mut g = one_landmass_game_with(61, &["CIVILIZATION_LANUN", "CIVILIZATION_BANNOR"]);
        let team = g.players[US].team;
        g.team_set_has_tech(team, g.d.t::<TechType>("TECH_OPTICS"), true, Some(US), false, false);
        g.player_found(US, 4, 4);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 4);
        g.player_found(THEM, 15, 3);
        g.player_grant_world_ritual_pending(US);
        // At peace: no storm.
        play(&mut g);
        assert!(g.player_storm(US).is_none());
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        for _ in 0..3 {
            unit(&mut g, THEM, "UNIT_WARRIOR", 10, 3);
        }
        // Our eyes on the shore.
        unit(&mut g, US, "UNIT_SCOUT", 10, 4);
        play(&mut g);
        let i = g.player_storm(US).expect("a storm");
        let centre = g.area_effects[i].plot;
        let (x, y) = g.map.xy(centre);
        assert!(g.plot_distance(x, y, 10, 3) <= lk::STORM_RADIUS, "over the enemy: ({x}, {y})");
        // The enemy walks along the shore: the storm follows next turn.
        for u in g.players[THEM].units.clone() {
            g.unit_set_xy(u, 8, 3, false);
        }
        unit(&mut g, US, "UNIT_SCOUT", 8, 4);
        g.set_game_turn(g.game_turn() + 1);
        play(&mut g);
        let centre = g.area_effects[g.player_storm(US).unwrap()].plot;
        let (nx, ny) = g.map.xy(centre);
        assert!(g.plot_distance(nx, ny, x, y) <= lk::STEER_RANGE);
        assert!(g.plot_distance(x, y, 8, 3) > lk::STORM_RADIUS && g.plot_distance(nx, ny, 8, 3) <= lk::STORM_RADIUS, "({x}, {y}) -> ({nx}, {ny})");
    }

    #[test]
    fn the_khazad_dig_from_a_hill_garrison_and_units_take_the_tunnel() {
        let mut g = one_landmass_game_with(62, &["CIVILIZATION_KHAZAD", "CIVILIZATION_BANNOR"]);
        let team = g.players[US].team;
        g.team_set_has_tech(team, g.d.t::<TechType>("TECH_MINING"), true, Some(US), false, false);
        for (x, y) in [(4, 5), (12, 5), (13, 6)] {
            let t = plot(&g, x, y);
            g.plot_set_plot_type(t, PlotType::PLOT_HILLS, false);
        }
        g.player_found(US, 4, 5);
        g.player_found(US, 13, 5);
        for (x, y) in [(4, 5), (12, 5), (13, 6)] {
            let t = plot(&g, x, y);
            g.plot_set_owner(t, Some(US), false, false);
        }
        let guard = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.player_grant_world_ritual_pending(US);
        play(&mut g);
        assert_eq!(g.delves.len(), 1, "a tunnel");
        let (a, b) = g.delves[0];
        assert_eq!(a, plot(&g, 4, 5));
        assert!(b == plot(&g, 12, 5) || b == plot(&g, 13, 6), "next to the far city");
        // A unit on the near end ordered far east takes it.
        let walker = unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        let grp = g.units[walker].group.unwrap();
        g.groups[grp].mission_queue.push(ffh_core::structs::MissionData { mission_type: MissionType::MISSION_MOVE_TO, data1: 16, data2: 5, flags: 0, push_turn: 0 });
        delve_travel(&mut g, US);
        assert_eq!(g.unit_plot(walker), b);
        assert_eq!(g.unit_plot(guard), a, "no order: it stays");
    }
}
