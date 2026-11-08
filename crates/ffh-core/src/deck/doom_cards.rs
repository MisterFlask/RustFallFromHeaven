//! The Armageddon cards of docs/DECKS_ROUND8.md section B that survived review (C.5, C.7), and
//! the rules other cards add to the deck (the Orrery of Ceridwen's sight).
//!
//! Each card's text (`TXT_KEY_DOOM_<NAME>`, `TXT_KEY_DOOM_<NAME>_HELP`) is in its card file,
//! `data/json/cards/doom_<name>.jsonc`. Cards that last for the rest of the game are rules where
//! the rule lives, asking `Game::doom_fired`:
//! - The Long Night: `Game::plot_calculate_yield` (plot/geography.rs), `Game::unit_heal_rate`
//!   (unit/actions.rs).
//!
//! The Sky Falls leaves Crater (`TERRAIN_CRATER`, in its card file): its rules are the terrain's
//! keys, asked by `Game::plot_is_unimprovable` and `Game::plot_is_unworkable` (plot/terrain.rs).
//!
//! The Godless Age, The Gathering, The Twilight of the Gods, The Lairs Deepen and The Second Sun
//! were cut (the owner's call, 2026-11-08).
//!
//! The cards that bring a civilization into the game (A04 The Scaled Kingdom, A06 The Brood, A22
//! The Infernal Gate Opens) are batch B8's: their `valid` and `fire` functions are in
//! `ffh/cards/new_civs_2.rs`.

use std::cmp::Reverse;

use crate::deck::doom::DoomCard;
use crate::ffh::cards::new_civs_2;
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

pub const WILD_AWAKENS: &str = "DOOM_WILD_AWAKENS";
pub const SEA_RECEDES: &str = "DOOM_SEA_RECEDES";
pub const VOLCANIC_SPRING: &str = "DOOM_VOLCANIC_SPRING";
pub const WEREWOLF_MOON: &str = "DOOM_WEREWOLF_MOON";
pub const BLADE_RETURNS: &str = "DOOM_BLADE_RETURNS";
pub const SKY_FALLS: &str = "DOOM_SKY_FALLS";
pub const LONG_NIGHT: &str = "DOOM_LONG_NIGHT";
pub const SCALED_KINGDOM: &str = "DOOM_SCALED_KINGDOM";
pub const BROOD: &str = "DOOM_BROOD";
pub const INFERNAL_GATE: &str = "DOOM_INFERNAL_GATE";

/// The new cards (FFH2's are in `doom::CARDS`).
pub const CARDS: &[DoomCard] = &[
    DoomCard { key: WILD_AWAKENS, stages: &[20], fire: fire_wild_awakens, valid: Some(lairs_spawn), on_reveal: None },
    DoomCard { key: SEA_RECEDES, stages: &[30], fire: fire_sea_recedes, valid: Some(valid_sea_recedes), on_reveal: Some(name_sea) },
    DoomCard { key: VOLCANIC_SPRING, stages: &[40], fire: fire_volcanic_spring, valid: Some(valid_volcanic_spring), on_reveal: Some(name_hills) },
    DoomCard { key: WEREWOLF_MOON, stages: &[40], fire: fire_werewolf_moon, valid: Some(barbarians_on), on_reveal: None },
    DoomCard { key: BLADE_RETURNS, stages: &[50], fire: fire_blade_returns, valid: Some(valid_blade_returns), on_reveal: None },
    DoomCard { key: SKY_FALLS, stages: &[80], fire: fire_sky_falls, valid: None, on_reveal: None },
    DoomCard { key: LONG_NIGHT, stages: &[100], fire: fire_long_night, valid: None, on_reveal: None },
    DoomCard { key: SCALED_KINGDOM, stages: &[30], fire: new_civs_2::fire_scaled_kingdom, valid: Some(new_civs_2::valid_scaled_kingdom), on_reveal: None },
    DoomCard { key: BROOD, stages: &[30, 50], fire: new_civs_2::fire_brood, valid: Some(new_civs_2::valid_brood), on_reveal: None },
    DoomCard { key: INFERNAL_GATE, stages: &[60], fire: new_civs_2::fire_infernal_gate, valid: Some(new_civs_2::valid_infernal_gate), on_reveal: None },
];

/// How many unrevealed Armageddon cards ahead `p` sees: the most any building of theirs shows
/// (`BuildingInfo::doom_sight`: the Orrery of Ceridwen, 2).
pub fn doom_sight(g: &Game, p: PlayerId) -> usize {
    let d = g.d;
    let mut sight = 0;
    for b in d.buildings.keys().filter(|&b| d.buildings[b].doom_sight > 0) {
        if g.players[p].cities.iter().any(|&c| g.cities.contains(c) && g.city_num_active_building(c, b) > 0) {
            sight = sight.max(d.buildings[b].doom_sight as usize);
        }
    }
    sight
}

// ---------------------------------------------------------------------------------------------
// Shared

/// The civilizations a card strikes, as FFH2's Armageddon events chose them: alive, not the
/// barbarians, minor civs or crusade Hosts.
fn civ_players(g: &Game) -> Vec<PlayerId> {
    g.player_ids()
        .filter(|&p| g.players[p].alive && !g.player_is_barbarian(p) && !g.player_is_minor_civ(p) && !g.player_is_crusade_host(p))
        .collect()
}

/// The world's cities, largest first (ties: the lower city id).
fn cities_by_size(g: &Game) -> Vec<CityId> {
    let mut cities: Vec<CityId> = g.cities.ids();
    cities.sort_by_key(|&c| (Reverse(g.cities[c].population), c));
    cities
}

/// `p`'s largest city (ties: the lower city id).
fn largest_city(g: &Game, p: PlayerId) -> Option<CityId> {
    g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c)).min_by_key(|&c| (Reverse(g.cities[c].population), c))
}

/// The distinct plots within `r` of (`x`, `y`) (the map wraps), in plot order.
fn plots_within(g: &Game, x: i32, y: i32, r: i32) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            if let Some(p) = g.map.plot_xy(x + dx, y + dy) {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// An empty land plot next to `center` (else 2 away) to place units on: passable, no city,
/// no unit. One drawn among the nearest ring's (`None`: none within 2).
fn spawn_plot_near(g: &mut Game, center: PlotIdx) -> Option<PlotIdx> {
    let (x, y) = g.map.xy(center);
    for r in 1..=2 {
        let ring: Vec<PlotIdx> = plots_within(g, x, y, r)
            .into_iter()
            .filter(|&p| {
                let (px, py) = g.map.xy(p);
                g.plot_distance(x, y, px, py) == r
                    && !g.plot_is_water(p)
                    && !g.plot_is_impassable(p)
                    && g.map.plots[p].plot_city.is_none()
                    && g.map.plots[p].units.is_empty()
            })
            .collect();
        if !ring.is_empty() {
            let k = g.soren_rand_num(ring.len() as i32, "Armageddon spawn plot") as usize;
            return Some(ring[k]);
        }
    }
    None
}

/// Damage equal to `percent`% of each unit's hit points to every unit on `p` (a unit already
/// hurt that much dies).
fn damage_units(g: &mut Game, p: PlotIdx, percent: i32) {
    for u in g.map.plots[p].units.clone() {
        if g.units.contains(u) && !g.units[u].death_delay {
            let damage = g.unit_max_hit_points(u) * percent / 100;
            g.unit_change_damage(u, damage, None);
        }
    }
}

/// `c` loses `n` population, keeping at least 1.
fn lose_population(g: &mut Game, c: CityId, n: i32) {
    let loss = n.min(g.cities[c].population - 1);
    if loss > 0 {
        g.city_change_population(c, -loss);
    }
}

fn barbarians_on(g: &Game) -> bool {
    !g.is_option(GameOptionType::GAMEOPTION_NO_BARBARIANS)
}

fn targets(g: &Game, key: &str, stage: usize) -> Vec<PlotIdx> {
    g.doom_targets(key, stage).iter().filter(|&&i| i >= 0 && (i as usize) < g.map.plots.len()).map(|&i| PlotIdx(i as u32)).collect()
}

// ---------------------------------------------------------------------------------------------
// A01 The Wild Awakens (20): the lairs.

/// Barbarians are on and some lair is on the map.
fn lairs_spawn(g: &Game) -> bool {
    barbarians_on(g) && g.map.plot_ids().any(|p| g.plot_lair_spawn_unit(p).is_some())
}

/// Every lair spawns three of its units at once (where no other civilization's unit stands on
/// it, as for the lairs' own spawns).
fn fire_wild_awakens(g: &mut Game, _: usize) {
    if !barbarians_on(g) {
        return;
    }
    let barbarian = g.barbarian_player();
    for p in g.map.plot_ids() {
        let Some(unit) = g.plot_lair_spawn_unit(p) else { continue };
        if g.plot_is_visible_other_unit(p, barbarian) {
            continue;
        }
        for _ in 0..3 {
            g.plot_spawn_lair_unit(p, unit);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A02 The Sea Recedes (30).

/// The fewest plots the named sea must give up for the card to be played.
const SEA_MIN_PLOTS: usize = 10;

/// A plot of the sea that recedes: unimproved Coast outside every city's radius.
fn receding(g: &Game, p: PlotIdx, coast: TerrainType) -> bool {
    let pl = &g.map.plots[p];
    g.plot_is_water(p) && pl.terrain_type == Some(coast) && pl.improvement_type.is_none() && pl.plot_city.is_none() && !g.plot_is_city_radius(p)
}

/// How far the named sea reaches from its centre, in plots: a map's ocean is often one water
/// area, and the card names a stretch of it, not all of it.
const SEA_RADIUS: i32 = 7;

/// The sea: the receding plots within [`SEA_RADIUS`] of the receding plot that has the most of
/// them in its own water area (ties: the lower plot index), in plot order; `None` when it has
/// fewer than `SEA_MIN_PLOTS`.
fn sea(g: &Game) -> Option<Vec<PlotIdx>> {
    let coast = g.d.t::<TerrainType>("TERRAIN_COAST");
    let all: Vec<PlotIdx> = g.map.plot_ids().filter(|&p| receding(g, p, coast)).collect();
    let mut is = vec![false; g.map.plots.len()];
    for &p in &all {
        is[p.idx()] = true;
    }
    let mut best: Option<Vec<PlotIdx>> = None;
    for &c in &all {
        let (x, y) = g.map.xy(c);
        let area = g.plot_area(c);
        let around: Vec<PlotIdx> = plots_within(g, x, y, SEA_RADIUS).into_iter().filter(|&q| is[q.idx()] && g.plot_area(q) == area).collect();
        if best.as_ref().map_or(true, |b| around.len() > b.len()) {
            best = Some(around);
        }
    }
    best.filter(|b| b.len() >= SEA_MIN_PLOTS)
}

fn valid_sea_recedes(g: &Game) -> bool {
    sea(g).is_some()
}

/// Names the sea: its receding plots (indices).
fn name_sea(g: &mut Game, _: usize) -> Vec<i32> {
    sea(g).unwrap_or_default().iter().map(|p| p.0 as i32).collect()
}

/// The named sea's plots that still recede (unimproved Coast outside every city's radius)
/// become grassland, for good. Ships on them move first to the nearest water that stays (their
/// cargo with them); then each plot turns to land, which clears its sea resource and water
/// feature (`plot_erase`).
fn fire_sea_recedes(g: &mut Game, stage: usize) {
    let d = g.d;
    let coast = d.t::<TerrainType>("TERRAIN_COAST");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    let plots: Vec<PlotIdx> = targets(g, SEA_RECEDES, stage).into_iter().filter(|&p| receding(g, p, coast)).collect();
    let mut recedes = vec![false; g.map.plots.len()];
    for &p in &plots {
        recedes[p.idx()] = true;
    }
    for &p in &plots {
        for u in g.map.plots[p].units.clone() {
            if g.units.contains(u) && !g.units[u].death_delay && !g.unit_is_cargo(u) {
                move_to_water(g, u, &recedes);
            }
        }
    }
    for p in plots {
        g.plot_set_plot_type(p, PlotType::PLOT_LAND, true);
        g.plot_set_terrain_type(p, grass, true);
    }
}

/// Moves `u` to the nearest water plot it may enter that is not marked in `avoid` (ties: the
/// lower plot index), its cargo with it; it dies if there is none (as a unit left with no valid
/// plot does).
fn move_to_water(g: &mut Game, u: UnitId, avoid: &[bool]) {
    let (x, y) = (g.units[u].x, g.units[u].y);
    let mut best: Option<(i32, PlotIdx)> = None;
    for q in g.map.plot_ids() {
        if !g.plot_is_water(q) || avoid[q.idx()] || !g.plot_is_valid_domain_for_location(q, u) || !g.unit_can_move_into(u, q, false, false, false) {
            continue;
        }
        let (qx, qy) = g.map.xy(q);
        let dist = g.plot_distance(x, y, qx, qy);
        if best.map_or(true, |(b, _)| dist < b) {
            best = Some((dist, q));
        }
    }
    match best {
        Some((_, q)) => {
            let (qx, qy) = g.map.xy(q);
            g.unit_set_xy(u, qx, qy, false);
        }
        None => g.unit_kill(u, false, None),
    }
}

// ---------------------------------------------------------------------------------------------
// A09 Volcanic Spring (40).

/// How many volcanoes erupt.
const VOLCANOES: usize = 6;

/// The hills within 3 plots of `c` a volcano may take: no city, no permanent improvement, not
/// in `named`.
fn hills_near(g: &Game, c: CityId, named: &[PlotIdx]) -> Vec<PlotIdx> {
    let d = g.d;
    plots_within(g, g.cities[c].x, g.cities[c].y, 3)
        .into_iter()
        .filter(|&p| {
            let pl = &g.map.plots[p];
            g.plot_is_hills(p)
                && pl.plot_city.is_none()
                && pl.improvement_type.map_or(true, |i| !d.improvements[i].permanent)
                && !named.contains(&p)
        })
        .collect()
}

fn valid_volcanic_spring(g: &Game) -> bool {
    g.cities.ids().into_iter().any(|c| !hills_near(g, c, &[]).is_empty())
}

/// Names the hills: one drawn within 3 plots of each of the six largest cities that has one (a
/// city without a hill near it gives its place to the next largest).
fn name_hills(g: &mut Game, _: usize) -> Vec<i32> {
    let mut named: Vec<PlotIdx> = Vec::new();
    for c in cities_by_size(g) {
        if named.len() >= VOLCANOES {
            break;
        }
        let hills = hills_near(g, c, &named);
        if !hills.is_empty() {
            let k = g.soren_rand_num(hills.len() as i32, "Volcanic Spring") as usize;
            named.push(hills[k]);
        }
    }
    named.iter().map(|p| p.0 as i32).collect()
}

/// Each named hill erupts.
fn fire_volcanic_spring(g: &mut Game, stage: usize) {
    for p in targets(g, VOLCANIC_SPRING, stage) {
        erupt(g, p);
    }
}

/// A volcano erupts on `p`: the plot becomes flat land with a Volcano (unless a city stands on
/// it now, or it is no longer a land plot below the peaks). Within 1 plot, units take damage
/// equal to 50% of their hit points, improvements (not permanent ones) are destroyed and cities
/// lose 2 population; the plots around the volcano give +1 food from then on (the game's plot
/// extra yields). Units left on the volcano move off it.
fn erupt(g: &mut Game, p: PlotIdx) {
    let d = g.d;
    let volcano = d.t::<FeatureType>("FEATURE_VOLCANO");
    let (x, y) = g.map.xy(p);
    let cone = g.map.plots[p].plot_city.is_none() && !g.plot_is_water(p) && !g.plot_is_peak(p);
    if cone {
        g.plot_set_feature_type(p, None);
        g.plot_set_plot_type(p, PlotType::PLOT_LAND, true);
        g.plot_set_feature_type(p, Some(volcano));
    }
    for q in plots_within(g, x, y, 1) {
        damage_units(g, q, 50);
        if g.map.plots[q].improvement_type.is_some_and(|i| !d.improvements[i].permanent) {
            g.plot_set_improvement_type(q, None);
        }
        if let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities.contains(c)) {
            lose_population(g, c, 2);
        }
        if q != p {
            let (qx, qy) = g.map.xy(q);
            g.set_plot_extra_yield(qx, qy, YieldType::YIELD_FOOD, 1);
        }
    }
    if cone {
        for u in g.map.plots[p].units.clone() {
            if g.units.contains(u) && !g.units[u].death_delay && !g.unit_is_cargo(u) {
                g.unit_jump_to_nearest_valid_plot(u);
            }
        }
    }
    let turn = g.game_turn();
    g.log.msg(turn, None, || format!("Volcanic Spring: a volcano erupts at ({x}, {y})."));
}

// ---------------------------------------------------------------------------------------------
// A11 Werewolf Moon (40).

/// Three barbarian Ravenous Werewolves appear next to each civilization's largest city, and
/// every barbarian Wolf (and Wolf Pack) becomes a Greater Werewolf.
fn fire_werewolf_moon(g: &mut Game, _: usize) {
    let d = g.d;
    let barbarian = g.barbarian_player();
    let ravenous = d.t::<UnitType>("UNIT_RAVENOUS_WEREWOLF");
    let greater = d.t::<UnitType>("UNIT_GREATER_WEREWOLF");
    for p in civ_players(g) {
        let Some(c) = largest_city(g, p) else { continue };
        let center = g.city_plot(c);
        let Some(q) = spawn_plot_near(g, center) else { continue };
        let (x, y) = g.map.xy(q);
        for _ in 0..3 {
            g.player_init_unit(barbarian, ravenous, x, y, Some(UnitAIType::UNITAI_ATTACK), None);
        }
    }
    let wolves = [d.t::<UnitType>("UNIT_WOLF"), d.t::<UnitType>("UNIT_WOLF_PACK")];
    for u in g.players[barbarian].units.clone() {
        if !g.units.contains(u) || g.units[u].death_delay || !wolves.contains(&g.units[u].unit_type) {
            continue;
        }
        let (x, y) = (g.units[u].x, g.units[u].y);
        let w = g.player_init_unit(barbarian, greater, x, y, None, None);
        g.unit_convert(w, u);
    }
}

// ---------------------------------------------------------------------------------------------
// A33 The Blade Returns (50).

/// The Godslayer is in the world: lying somewhere, or held by a unit.
fn godslayer_in_world(g: &Game) -> bool {
    let d = g.d;
    let blade = d.t::<UnitType>("EQUIPMENT_GODSLAYER");
    let held = d.t::<PromotionType>("PROMOTION_GODSLAYER");
    g.units.iter().any(|(_, u)| u.unit_type == blade || u.has_promotion[held])
}

/// The unowned land plot farthest from every city (the nearest city's distance; ties: the lower
/// plot index): passable, no unit.
fn farthest_plot(g: &Game) -> Option<PlotIdx> {
    let cities: Vec<(i32, i32)> = g.cities.iter().map(|(_, c)| (c.x, c.y)).collect();
    let mut best: Option<(i32, PlotIdx)> = None;
    for p in g.map.plot_ids() {
        let pl = &g.map.plots[p];
        if g.plot_is_water(p) || g.plot_is_impassable(p) || pl.owner.is_some() || pl.plot_city.is_some() || !pl.units.is_empty() {
            continue;
        }
        let (x, y) = g.map.xy(p);
        let dist = cities.iter().map(|&(cx, cy)| g.plot_distance(x, y, cx, cy)).min().unwrap_or(i32::MAX);
        if best.map_or(true, |(b, _)| dist > b) {
            best = Some((dist, p));
        }
    }
    best.map(|(_, p)| p)
}

fn valid_blade_returns(g: &Game) -> bool {
    barbarians_on(g) && !godslayer_in_world(g) && farthest_plot(g).is_some()
}

/// The Godslayer, if it is not in the world, appears (the barbarians') at the unowned land plot
/// farthest from every city, guarded by four barbarian Greater Werewolves.
fn fire_blade_returns(g: &mut Game, _: usize) {
    if godslayer_in_world(g) {
        return;
    }
    let Some(p) = farthest_plot(g) else { return };
    let d = g.d;
    let barbarian = g.barbarian_player();
    let (x, y) = g.map.xy(p);
    g.player_init_unit(barbarian, d.t::<UnitType>("EQUIPMENT_GODSLAYER"), x, y, None, Some(DirectionType::DIRECTION_SOUTH));
    let greater = d.t::<UnitType>("UNIT_GREATER_WEREWOLF");
    for _ in 0..4 {
        g.player_init_unit(barbarian, greater, x, y, Some(UnitAIType::UNITAI_ATTACK), None);
    }
    let turn = g.game_turn();
    g.log.msg(turn, None, || format!("The Blade Returns: the Godslayer lies at ({x}, {y})."));
}

// ---------------------------------------------------------------------------------------------
// A40 The Sky Falls (80).

/// The terrain the meteors leave.
pub const CRATER: &str = "TERRAIN_CRATER";

/// Meteors strike the ten largest cities, largest first: units there take damage equal to 50% of
/// their hit points, each city loses 2 population, and half of its [`crater_sites`] (rounded
/// down) become Crater, drawn one at a time with the game's random numbers. Then every city stops
/// working the plots that are Crater now (the citizens are placed again on its turn).
fn fire_sky_falls(g: &mut Game, _: usize) {
    let crater = g.d.t::<TerrainType>(CRATER);
    for c in cities_by_size(g).into_iter().take(10) {
        if !g.cities.contains(c) {
            continue;
        }
        let p = g.city_plot(c);
        damage_units(g, p, 50);
        lose_population(g, c, 2);
        let mut sites = crater_sites(g, c, crater);
        for _ in 0..sites.len() / 2 {
            let k = g.soren_rand_num(sites.len() as i32, "The Sky Falls") as usize;
            make_crater(g, sites.remove(k), crater);
        }
    }
    for c in g.cities.ids() {
        g.city_verify_working_plots(c);
    }
}

/// The land plots of `c`'s work radius a meteor may turn to Crater, in the radius's order: not
/// the city's own plot, a peak, water, another city's plot or Crater already.
pub(crate) fn crater_sites(g: &Game, c: CityId, crater: TerrainType) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for i in 0..g.city_num_city_plots(c) as usize {
        let Some(q) = g.city_index_plot(c, i) else { continue };
        let pl = &g.map.plots[q];
        if i == crate::defs::CITY_HOME_PLOT
            || g.plot_is_water(q)
            || g.plot_is_peak(q)
            || pl.plot_city.is_some()
            || pl.terrain_type == Some(crater)
            || out.contains(&q)
        {
            continue;
        }
        out.push(q);
    }
    out
}

/// `p` becomes Crater: a hills plot of terrain Crater, without improvement, feature or resource
/// (its route stays); a temporary terrain on it ends. Crater can carry no improvement and no city
/// works it (`Game::plot_is_unimprovable`, `Game::plot_is_unworkable`).
fn make_crater(g: &mut Game, p: PlotIdx, crater: TerrainType) {
    g.plot_set_improvement_type(p, None);
    g.plot_set_feature_type(p, None);
    g.plot_set_bonus_type(p, None);
    g.map.plots[p].temp_terrain_timer = 0;
    g.map.plots[p].real_terrain_type = None;
    g.plot_set_plot_type(p, PlotType::PLOT_HILLS, true);
    g.plot_set_terrain_type(p, crater, true);
}

// ---------------------------------------------------------------------------------------------
// A49 The Long Night (100).

/// From now on, -1 food on every plot and no healing outside cities (the rules ask
/// `doom_fired(LONG_NIGHT)`): every plot's yield is computed again.
fn fire_long_night(g: &mut Game, _: usize) {
    for p in g.map.plot_ids() {
        g.plot_update_yield(p);
    }
}

/// For tests: a sea dug by hand (the test maps are nearly all land): the first 8 x 8 block of
/// land without a unit or a city, with land all around it (a sea of its own), becomes water,
/// its outer ring Coast and the rest Ocean. Returns its plots.
#[cfg(test)]
pub(crate) fn dig_test_sea(g: &mut Game) -> Vec<PlotIdx> {
    const SIDE: i32 = 8;
    let (w, h) = (g.map.grid_width, g.map.grid_height);
    let square = |g: &Game, ox: i32, oy: i32, side: i32| -> Vec<PlotIdx> {
        (0..side).flat_map(|dy| (0..side).map(move |dx| (ox + dx, oy + dy))).filter_map(|(x, y)| g.map.plot_xy(x, y)).collect()
    };
    for oy in 2..h - SIDE - 2 {
        for ox in 2..w - SIDE - 2 {
            let block = square(g, ox, oy, SIDE);
            let around = square(g, ox - 1, oy - 1, SIDE + 2);
            let clear = block.len() == (SIDE * SIDE) as usize
                && around.iter().all(|&p| !g.plot_is_water(p))
                && block.iter().all(|&p| g.map.plots[p].units.is_empty() && g.map.plots[p].plot_city.is_none());
            if clear {
                for &p in &block {
                    g.plot_set_plot_type(p, PlotType::PLOT_OCEAN, true);
                }
                return block;
            }
        }
    }
    panic!("no room for a sea");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::doom::DoomSlot;
    use crate::setup::GameSetup;

    fn game(civs: usize) -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, civs, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    /// Puts `key` (its first stage) in a slot as played, with `targets`: what the rules and its
    /// fire read.
    fn play(g: &mut Game, key: &str, targets: Vec<i32>) {
        let c = crate::deck::doom::find(key).expect("a card");
        g.doom.slots.push(DoomSlot { threshold: c.stages[0], card: Some((key.to_string(), 0)), revealed: true, fired: true, targets });
    }

    /// The empty passable land plots, in plot order.
    fn open_land(g: &Game) -> Vec<PlotIdx> {
        g.map
            .plot_ids()
            .filter(|&p| !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.map.plots[p].plot_city.is_none() && g.map.plots[p].units.is_empty())
            .collect()
    }

    fn civ(g: &Game, i: usize) -> PlayerId {
        civ_players(g)[i]
    }

    fn city_at(g: &mut Game, p: PlayerId, at: PlotIdx, pop: i32) -> CityId {
        let (x, y) = g.map.xy(at);
        let c = g.player_init_city(p, x, y, true, true);
        g.city_set_population(c, pop);
        c
    }

    fn unit_at(g: &mut Game, p: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = g.map.xy(at);
        let unit = g.d.t::<UnitType>(name);
        g.player_init_unit(p, unit, x, y, None, None)
    }

    fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
        let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
        g.plot_distance(ax, ay, bx, by)
    }

    /// The units of type `name` of `owner` on `p`.
    fn count(g: &Game, p: PlotIdx, owner: PlayerId, name: &str) -> usize {
        let unit = g.d.t::<UnitType>(name);
        g.map.plots[p].units.iter().filter(|&&u| g.units[u].unit_type == unit && g.units[u].owner == owner && !g.units[u].death_delay).count()
    }

    #[test]
    fn the_wild_awakens_spawns_three_at_every_lair_but_an_occupied_one() {
        let mut g = game(2);
        let d = g.d;
        let barrow = d.t::<ImprovementType>("IMPROVEMENT_BARROW");
        let land = open_land(&g);
        let (free, held) = (land[0], land[land.len() - 1]);
        // A civilization's unit stands on the second lair (placed first: a unit entering a lair
        // destroys it).
        let p0 = civ(&g, 0);
        unit_at(&mut g, p0, "UNIT_WARRIOR", held);
        g.plot_set_improvement_type(free, Some(barrow));
        g.plot_set_improvement_type(held, Some(barrow));
        assert!(lairs_spawn(&g));
        let barbarian = g.barbarian_player();
        fire_wild_awakens(&mut g, 0);
        assert_eq!(count(&g, free, barbarian, "UNIT_SKELETON"), 3);
        assert_eq!(count(&g, held, barbarian, "UNIT_SKELETON"), 0);
        // Not played without barbarians.
        g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
        assert!(!lairs_spawn(&g));
    }

    #[test]
    fn the_sea_recedes_turns_the_named_coast_to_grassland() {
        let mut g = game(2);
        let d = g.d;
        let coast = d.t::<TerrainType>("TERRAIN_COAST");
        let grass = d.t::<TerrainType>("TERRAIN_GRASS");
        let sea = dig_test_sea(&mut g);
        assert!(valid_sea_recedes(&g));
        let named = name_sea(&mut g, 0);
        let plots: Vec<PlotIdx> = named.iter().map(|&i| PlotIdx(i as u32)).collect();
        assert!(plots.len() >= SEA_MIN_PLOTS);
        let area = g.plot_area(plots[0]);
        assert!(plots.iter().all(|&p| sea.contains(&p) && receding(&g, p, coast) && g.plot_area(p) == area), "the dug sea's unimproved Coast");
        assert_eq!(plots.len(), sea.iter().filter(|&&p| g.map.plots[p].terrain_type == Some(coast)).count(), "all of it");

        // After the reveal: a city founded next to the first plot keeps the coast in its radius;
        // a fishing improvement keeps another at sea; a ship and a resource on two others.
        let near = plots[0];
        let (nx, ny) = g.map.xy(near);
        let site = plots_within(&g, nx, ny, 1).into_iter().find(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q)).expect("Coast is next to land");
        let p0 = civ(&g, 0);
        let city = city_at(&mut g, p0, site, 1);
        let far: Vec<PlotIdx> = plots.iter().copied().filter(|&p| dist(&g, p, site) > 3).collect();
        assert!(far.len() >= 3, "a sea of 10 plots or more reaches beyond the city");
        let (kept, sailed, rich) = (far[0], far[1], far[2]);
        g.plot_set_improvement_type(kept, Some(d.t::<ImprovementType>("IMPROVEMENT_FISHING_BOATS")));
        g.plot_set_bonus_type(rich, Some(d.t::<BonusType>("BONUS_FISH")));
        let ship = unit_at(&mut g, p0, "UNIT_CARAVEL", sailed);

        play(&mut g, SEA_RECEDES, named.clone());
        fire_sea_recedes(&mut g, 0);
        assert!(g.plot_is_water(near), "in a city's radius at the time it plays");
        assert!(g.plot_is_water(kept), "improved");
        for p in [sailed, rich] {
            assert!(!g.plot_is_water(p));
            assert_eq!(g.map.plots[p].terrain_type, Some(grass));
        }
        assert_eq!(g.map.plots[rich].bonus_type, None, "the sea resource is gone");
        assert!(g.units.contains(ship) && g.plot_is_water(g.unit_plot(ship)), "the ship moved to water");
        assert!(g.cities.contains(city));
    }

    #[test]
    fn volcanic_spring_erupts_on_the_named_hill() {
        let mut g = game(2);
        let d = g.d;
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let site = land[land.len() / 2];
        let city = city_at(&mut g, p0, site, 6);
        let (cx, cy) = g.map.xy(site);
        // One hill near the only city: the others within 3 are made flat.
        for q in plots_within(&g, cx, cy, 3) {
            if g.plot_is_hills(q) {
                g.plot_set_plot_type(q, PlotType::PLOT_LAND, true);
            }
        }
        let ring: Vec<PlotIdx> = plots_within(&g, cx, cy, 1).into_iter().filter(|&q| q != site && !g.plot_is_water(q) && !g.plot_is_peak(q)).collect();
        let hill = ring[0];
        g.plot_set_plot_type(hill, PlotType::PLOT_HILLS, true);
        g.plot_set_feature_type(hill, None);
        assert!(valid_volcanic_spring(&g));
        let named = name_hills(&mut g, 0);
        assert_eq!(named, vec![hill.0 as i32]);

        // Around it: a unit on the hill, one next to it, a farm.
        let (hx, hy) = g.map.xy(hill);
        let around: Vec<PlotIdx> = plots_within(&g, hx, hy, 1).into_iter().filter(|&q| q != hill && q != site && !g.plot_is_water(q) && !g.plot_is_impassable(q)).collect();
        let on_hill = unit_at(&mut g, p0, "UNIT_WARRIOR", hill);
        let beside = unit_at(&mut g, p0, "UNIT_WARRIOR", around[0]);
        let farm_plot = *around.last().unwrap();
        g.plot_set_improvement_type(farm_plot, Some(d.t::<ImprovementType>("IMPROVEMENT_FARM")));

        play(&mut g, VOLCANIC_SPRING, named);
        fire_volcanic_spring(&mut g, 0);
        assert!(!g.plot_is_hills(hill) && !g.plot_is_water(hill));
        assert_eq!(g.map.plots[hill].feature_type, Some(d.t::<FeatureType>("FEATURE_VOLCANO")));
        let half = g.unit_max_hit_points(beside) / 2;
        assert_eq!(g.units[beside].damage, half);
        assert_eq!(g.units[on_hill].damage, half);
        assert_ne!(g.unit_plot(on_hill), hill, "the survivor left the volcano");
        assert_eq!(g.map.plots[farm_plot].improvement_type, None);
        assert_eq!(g.cities[city].population, 4, "a city within 1 loses 2 population");
        let (fx, fy) = g.map.xy(around[0]);
        assert_eq!(g.plot_extra_yield(fx, fy, YieldType::YIELD_FOOD), 1, "+1 food next to the volcano");
        assert_eq!(g.plot_extra_yield(hx, hy, YieldType::YIELD_FOOD), 0);
    }

    #[test]
    fn werewolf_moon_raises_werewolves_by_the_largest_city_and_turns_the_wolves() {
        let mut g = game(2);
        let p0 = civ(&g, 0);
        let barbarian = g.barbarian_player();
        let land = open_land(&g);
        let site = land[land.len() / 2];
        let small = *land.iter().find(|&&p| dist(&g, p, site) > 6).unwrap();
        let big = city_at(&mut g, p0, site, 5);
        city_at(&mut g, p0, small, 2);
        let den = *open_land(&g).iter().find(|&&p| dist(&g, p, site) > 3 && dist(&g, p, small) > 3).unwrap();
        let wolf = unit_at(&mut g, barbarian, "UNIT_WOLF", den);
        assert!(barbarians_on(&g));
        fire_werewolf_moon(&mut g, 0);
        let (bx, by) = g.map.xy(g.city_plot(big));
        let near: usize = plots_within(&g, bx, by, 2).iter().map(|&q| count(&g, q, barbarian, "UNIT_RAVENOUS_WEREWOLF")).sum();
        assert_eq!(near, 3, "three by the largest city");
        assert!(!g.units.contains(wolf) || g.units[wolf].death_delay, "the wolf is gone");
        assert_eq!(count(&g, den, barbarian, "UNIT_GREATER_WEREWOLF"), 1, "a Greater Werewolf in its place");
        let _ = big;
    }

    #[test]
    fn the_blade_returns_far_from_every_city_and_only_once() {
        let mut g = game(2);
        let d = g.d;
        let land = open_land(&g);
        let (a, b) = (land[0], land[land.len() / 2]);
        let (p0, p1) = (civ(&g, 0), civ(&g, 1));
        city_at(&mut g, p0, a, 3);
        city_at(&mut g, p1, b, 3);
        assert!(valid_blade_returns(&g));
        // Before it: the candidate plots, and how far each is from the nearest city.
        let cities: Vec<PlotIdx> = g.cities.ids().into_iter().map(|c| g.city_plot(c)).collect();
        let reach = |g: &Game, q: PlotIdx| cities.iter().map(|&c| dist(g, q, c)).min().unwrap();
        let open: Vec<PlotIdx> = g
            .map
            .plot_ids()
            .filter(|&q| !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].owner.is_none() && g.map.plots[q].plot_city.is_none() && g.map.plots[q].units.is_empty())
            .collect();
        fire_blade_returns(&mut g, 0);
        let blade = d.t::<UnitType>("EQUIPMENT_GODSLAYER");
        let blades: Vec<UnitId> = g.units.iter().filter(|(_, u)| u.unit_type == blade).map(|(id, _)| id).collect();
        assert_eq!(blades.len(), 1);
        let p = g.unit_plot(blades[0]);
        let barbarian = g.barbarian_player();
        assert_eq!(g.units[blades[0]].owner, barbarian);
        assert!(open.contains(&p));
        assert!(open.iter().all(|&q| reach(&g, q) <= reach(&g, p)), "the farthest from every city");
        assert_eq!(count(&g, p, barbarian, "UNIT_GREATER_WEREWOLF"), 4);
        // In the world now: it does not come again.
        assert!(!valid_blade_returns(&g));
        fire_blade_returns(&mut g, 0);
        assert_eq!(g.units.iter().filter(|(_, u)| u.unit_type == blade).count(), 1);
    }

    #[test]
    fn the_sky_falls_on_the_ten_largest_cities() {
        let mut g = game(2);
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let step = land.len() / 12;
        let cities: Vec<CityId> = (0..11).map(|i| city_at(&mut g, p0, land[i * step], i as i32 + 2)).collect();
        let pops: Vec<i32> = cities.iter().map(|&c| g.cities[c].population).collect();
        let largest = g.city_plot(cities[10]);
        let fresh = unit_at(&mut g, p0, "UNIT_WARRIOR", largest);
        let hurt = unit_at(&mut g, p0, "UNIT_WARRIOR", largest);
        g.unit_change_damage(hurt, 60, None);
        fire_sky_falls(&mut g, 0);
        assert_eq!(g.cities[cities[0]].population, pops[0], "the smallest of eleven is spared");
        for i in 1..11 {
            assert_eq!(g.cities[cities[i]].population, pops[i] - 2);
        }
        assert_eq!(g.units[fresh].damage, g.unit_max_hit_points(fresh) / 2);
        assert!(!g.units.contains(hurt) || g.units[hurt].death_delay, "a unit already half hurt dies");
    }

    /// The plots of `c`'s work radius (distinct).
    fn radius(g: &Game, c: CityId) -> Vec<PlotIdx> {
        let mut out: Vec<PlotIdx> = (0..g.city_num_city_plots(c) as usize).filter_map(|i| g.city_index_plot(c, i)).collect();
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn the_sky_falls_turns_half_of_a_struck_citys_land_to_crater() {
        let mut g = game(2);
        let d = g.d;
        let crater = d.t::<TerrainType>(CRATER);
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let site = land[land.len() / 2];
        let city = city_at(&mut g, p0, site, 8);
        // In the radius: a peak, and on every other plot but the city's an improvement, a forest,
        // a resource and a road (the land the meteors may take).
        let around: Vec<PlotIdx> = radius(&g, city).into_iter().filter(|&q| q != site && !g.plot_is_water(q)).collect();
        let peak = around[0];
        g.plot_set_plot_type(peak, PlotType::PLOT_PEAK, true);
        let (farm, forest, wheat, road) = (
            d.t::<ImprovementType>("IMPROVEMENT_FARM"),
            d.t::<FeatureType>("FEATURE_FOREST"),
            d.t::<BonusType>("BONUS_WHEAT"),
            d.t::<RouteType>("ROUTE_ROAD"),
        );
        for &q in &around[1..] {
            g.plot_set_feature_type(q, Some(forest));
            g.plot_set_bonus_type(q, Some(wheat));
            g.plot_set_improvement_type(q, Some(farm));
            g.plot_set_route_type(q, Some(road), true);
        }
        let sites = crater_sites(&g, city, crater);
        assert!(!sites.contains(&site) && !sites.contains(&peak));
        assert_eq!(sites.len(), around.len() - 1, "every land plot of the radius but the city's and the peak");

        fire_sky_falls(&mut g, 0);
        let craters: Vec<PlotIdx> = radius(&g, city).into_iter().filter(|&q| g.map.plots[q].terrain_type == Some(crater)).collect();
        assert_eq!(craters.len(), sites.len() / 2, "half, rounded down");
        for &q in &craters {
            let pl = &g.map.plots[q];
            assert!(sites.contains(&q) && g.plot_is_hills(q));
            assert_eq!((pl.improvement_type, pl.feature_type, pl.bonus_type), (None, None, None));
            assert_eq!(pl.route_type, Some(road), "the road stays");
            assert_eq!(pl.yields, [0, 0, 0], "Crater yields nothing");
            assert!(!g.cities[city].working_plot[g.city_plot_index(city, q).unwrap()], "no citizen on it");
        }
        assert!(g.plot_is_peak(peak) && g.map.plots[site].terrain_type != Some(crater));
        // The rest of the land keeps what it had.
        for &q in sites.iter().filter(|q| !craters.contains(q)) {
            assert_eq!(g.map.plots[q].improvement_type, Some(farm));
        }
        // The citizens the craters took are placed again, never on Crater.
        g.city_assign_working_plots(city);
        assert!(craters.iter().all(|&q| !g.cities[city].working_plot[g.city_plot_index(city, q).unwrap()]));
        // A city struck again: half of what is left (the craters are not counted).
        assert_eq!(crater_sites(&g, city, crater).len(), sites.len() - craters.len());
    }

    #[test]
    fn crater_cannot_be_improved_or_worked_but_takes_a_road() {
        let mut g = game(2);
        let d = g.d;
        let crater = d.t::<TerrainType>(CRATER);
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let site = land[land.len() / 2];
        let city = city_at(&mut g, p0, site, 4);
        let (sx, sy) = g.map.xy(site);
        let next: Vec<PlotIdx> = plots_within(&g, sx, sy, 1).into_iter().filter(|&q| q != site && !g.plot_is_water(q) && !g.plot_is_peak(q)).collect();
        let (hill, pit) = (next[0], next[1]);
        for q in [hill, pit] {
            g.plot_set_feature_type(q, None);
            g.plot_set_plot_type(q, PlotType::PLOT_HILLS, true);
            g.plot_set_terrain_type(q, d.t::<TerrainType>("TERRAIN_GRASS"), true);
        }
        let mine = d.t::<ImprovementType>("IMPROVEMENT_MINE");
        let team = Some(g.players[p0].team);
        assert!(g.plot_can_have_improvement(hill, mine, team, false) && g.plot_can_have_improvement(pit, mine, team, false), "hills take a mine");
        assert!(g.city_can_work(city, hill) && g.city_can_work(city, pit));

        make_crater(&mut g, pit, crater);
        assert!(g.plot_is_unimprovable(pit) && g.plot_is_unworkable(pit));
        assert!(d.improvements.keys().all(|i| !g.plot_can_have_improvement(pit, i, team, false)), "no improvement on Crater");
        assert!(g.plot_can_have_improvement(hill, mine, team, false));
        assert!(!g.city_can_work(city, pit), "no city works Crater");
        assert!(g.city_can_work(city, hill));
        assert!(g.plot_can_build(pit, d.t::<BuildType>("BUILD_ROAD"), p0, false), "a road can be built");
        assert!(!g.plot_can_build(pit, d.t::<BuildType>("BUILD_MINE"), p0, false));
        // Under a temporary terrain it stays Crater for these rules.
        g.map.plots[pit].real_terrain_type = Some(crater);
        g.map.plots[pit].terrain_type = Some(d.t::<TerrainType>("TERRAIN_SNOW"));
        assert!(g.plot_is_unimprovable(pit) && g.plot_is_unworkable(pit));
    }

    #[test]
    fn crater_moves_defends_and_fights_as_hills() {
        let mut g = game(2);
        let d = g.d;
        let crater = d.t::<TerrainType>(CRATER);
        let grass = d.t::<TerrainType>("TERRAIN_GRASS");
        let p0 = civ(&g, 0);
        // A plot with two land neighbours, none of them by a river.
        let dry = |g: &Game, q: PlotIdx| !g.plot_is_water(q) && !g.plot_is_peak(q) && !g.plot_is_river_side(q) && g.map.plots[q].plot_city.is_none();
        let (from, hill, pit) = open_land(&g)
            .into_iter()
            .filter(|&o| dry(&g, o))
            .find_map(|o| {
                let (x, y) = g.map.xy(o);
                let n: Vec<PlotIdx> = plots_within(&g, x, y, 1).into_iter().filter(|&q| q != o && dry(&g, q)).collect();
                (n.len() >= 2).then(|| (o, n[0], n[1]))
            })
            .expect("dry land");
        for q in [hill, pit] {
            g.plot_set_improvement_type(q, None);
            g.plot_set_route_type(q, None, true);
            g.plot_set_feature_type(q, None);
            g.plot_set_plot_type(q, PlotType::PLOT_HILLS, true);
            g.plot_set_terrain_type(q, grass, true);
        }
        make_crater(&mut g, pit, crater);
        // A Horseman (3 moves): a hill's cost is not cut to the unit's moves.
        let rider = unit_at(&mut g, p0, "UNIT_HORSEMAN", from);
        assert_eq!(g.plot_movement_cost(pit, rider, from), g.plot_movement_cost(hill, rider, from));
        assert!(g.plot_movement_cost(pit, rider, from) > d.defines.move_denominator, "hills cost more than open ground");
        assert_eq!(g.plot_defense_modifier(pit, None, false), g.plot_defense_modifier(hill, None, false));
        assert_eq!(g.engage_ground(pit), g.engage_ground(hill));
        assert!(g.engage_ground(pit).hills);
        let team = g.players[p0].team;
        assert_eq!(g.plot_see_from_level(pit, team), g.plot_see_from_level(hill, team));
    }

    #[test]
    fn the_long_night_takes_a_food_everywhere_and_healing_outside_cities() {
        let mut g = game(2);
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let field = *land.iter().find(|&&p| g.map.plots[p].yields[YieldType::YIELD_FOOD.idx()] >= 2).expect("a plot with food");
        let site = *land.iter().find(|&&p| dist(&g, p, field) > 3).unwrap();
        let before = g.map.plots[field].yields[YieldType::YIELD_FOOD.idx()];
        city_at(&mut g, p0, site, 3);
        let outside = unit_at(&mut g, p0, "UNIT_WARRIOR", field);
        let inside = unit_at(&mut g, p0, "UNIT_WARRIOR", site);
        g.unit_change_damage(outside, 30, None);
        g.unit_change_damage(inside, 30, None);
        assert!(g.unit_heal_rate(outside, field) > 0);
        play(&mut g, LONG_NIGHT, vec![]);
        fire_long_night(&mut g, 0);
        assert_eq!(g.map.plots[field].yields[YieldType::YIELD_FOOD.idx()], before - 1);
        assert_eq!(g.unit_heal_rate(outside, field), 0, "no healing outside cities");
        assert!(g.unit_heal_rate(inside, site) > 0, "a city still heals");
    }

    /// Every unit stands where it says, and every plot has an area.
    fn consistent(g: &Game) -> Result<(), String> {
        for (u, unit) in g.units.iter() {
            if unit.death_delay || unit.x < 0 {
                continue;
            }
            let p = g.map.plot_xy(unit.x, unit.y).ok_or(format!("{u:?} off the map"))?;
            if !g.map.plots[p].units.contains(&u) {
                return Err(format!("{u:?} is not on its plot"));
            }
        }
        match g.map.plot_ids().find(|&p| g.plot_area(p).is_none()) {
            Some(p) => Err(format!("{p:?} has no area")),
            None => Ok(()),
        }
    }

    /// A world as games make it (the Fractal script: oceans, hills, coasts), with cities founded
    /// by hand, a lair, wolves and ships: every new card is dealt to
    /// its slots alone (a chain's every stage) and the counter raised to the last. Each is revealed
    /// and played (or dealt again at its reveal when it cannot be), and the world stays consistent.
    #[test]
    fn every_new_card_plays_on_a_generated_world() {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 4, "WORLDSIZE_SMALL", 7));
        g.map_script = std::sync::Arc::new(crate::mapscript::FractalMapScript::default());
        g.game_init();
        assert!(g.map.plot_ids().filter(|&p| g.plot_is_water(p)).count() > 200, "a world with seas");
        let civs = civ_players(&g);
        let barbarian = g.barbarian_player();
        // Cities: one on each starting plot, then coastal land plots in turn, spaced out.
        let mut sites: Vec<PlotIdx> = civs.iter().filter_map(|&p| g.player_get_starting_plot(p)).collect();
        for p in g.map.plot_ids() {
            if sites.len() >= 16 {
                break;
            }
            if !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.plot_is_coastal_land(p, -1) && sites.iter().all(|&s| dist(&g, s, p) >= 4) {
                sites.push(p);
            }
        }
        for (i, &s) in sites.iter().enumerate() {
            if g.map.plots[s].plot_city.is_none() {
                city_at(&mut g, civs[i % civs.len()], s, 3 + (i as i32 % 6));
            }
        }
        let cities = g.cities.ids();
        assert!(cities.len() >= 8);
        let land = open_land(&g);
        g.plot_set_improvement_type(land[land.len() / 3], Some(d.t::<ImprovementType>("IMPROVEMENT_BARROW")));
        unit_at(&mut g, barbarian, "UNIT_WOLF", land[land.len() / 2]);
        let coast = d.t::<TerrainType>("TERRAIN_COAST");
        let shore: Vec<PlotIdx> = g.map.plot_ids().filter(|&p| g.map.plots[p].terrain_type == Some(coast) && g.map.plots[p].units.is_empty()).collect();
        for k in 0..4 {
            unit_at(&mut g, civs[k % civs.len()], "UNIT_CARAVEL", shore[k * shore.len() / 4]);
        }
        consistent(&g).unwrap();

        for c in CARDS {
            g.doom.dealt = true;
            g.doom.slots = crate::deck::doom::SLOTS.iter().map(|&threshold| DoomSlot { threshold, ..Default::default() }).collect();
            // Every stage of the card in its slot, the counter raised to the last. (Its limit is
            // read each time: a card that brings a civilization raises it, as every new player.)
            for (k, &t) in c.stages.iter().enumerate() {
                let slot = g.doom.slots.iter_mut().find(|s| s.threshold == t).unwrap();
                slot.card = Some((c.key.to_string(), k as u8));
            }
            let last = *c.stages.last().unwrap();
            g.global_counter = (g.global_counter_limit * last + 99) / 100;
            g.doom_do_turn();
            for &t in c.stages {
                let slot = g.doom.slots.iter().find(|s| s.threshold == t).unwrap();
                assert!(slot.revealed && slot.fired, "{} at {t}", c.key);
            }
            consistent(&g).unwrap_or_else(|e| panic!("{}: {e}", c.key));
        }
    }

    #[test]
    fn the_orrery_shows_two_cards_and_researches_with_the_counter() {
        let mut g = game(2);
        let d = g.d;
        let (p0, p1) = (civ(&g, 0), civ(&g, 1));
        let orrery = d.t::<BuildingType>("BUILDING_ORRERY_OF_CERIDWEN");
        let site = open_land(&g)[0];
        let city = city_at(&mut g, p0, site, 4);
        g.doom_deal();
        assert_eq!(g.player_doom_sight(p0), 0);
        g.city_set_num_real_building(city, orrery, 1);
        assert_eq!(g.player_doom_sight(p0), 2);
        let ahead: Vec<usize> = (0..g.doom.slots.len()).filter(|&i| !g.doom.slots[i].revealed && g.doom.slots[i].card.is_some()).take(2).collect();
        assert_eq!(ahead.len(), 2);
        for i in 0..g.doom.slots.len() {
            assert_eq!(g.doom_slot_known(p0, i), ahead.contains(&i), "slot {i}");
            assert!(!g.doom_slot_known(p1, i));
        }
        // +2 research per 10 points of the counter, kept current as the counter moves.
        let research = CommerceType::COMMERCE_RESEARCH.idx();
        g.global_counter = 0;
        g.update_global_counter_commerce();
        let base = g.cities[city].building_commerce[research];
        g.change_global_counter(g.global_counter_limit * 35 / 100);
        assert_eq!(g.cities[city].building_commerce[research], base + 2 * (g.get_global_counter() / 10));
        assert!(g.get_global_counter() >= 30);
    }
}
