//! The Armageddon cards of docs/DECKS_ROUND8.md section B that survived review (C.5, C.7), and
//! the rules other cards add to the deck (the Orrery of Ceridwen's sight).
//!
//! Each card's text (`TXT_KEY_DOOM_<NAME>`, `TXT_KEY_DOOM_<NAME>_HELP`) is in its card file,
//! `data/json/cards/doom_<name>.jsonc`. Cards that last for the rest of the game are rules where
//! the rule lives, asking `Game::doom_fired`:
//! - The Lairs Deepen: `Game::plot_lair_spawn_unit` (plot/terrain.rs);
//! - The Long Night: `Game::plot_calculate_yield` (plot/geography.rs), `Game::unit_heal_rate`
//!   (unit/actions.rs).
//!
//! The Gathering's vassals are marked (`Team::gathered`) and do not count toward their master's
//! victory (`Game::team_victory_population`, `team_victory_land`, the conquest test).
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
pub const LAIRS_DEEPEN: &str = "DOOM_LAIRS_DEEPEN";
pub const SECOND_SUN: &str = "DOOM_SECOND_SUN";
pub const TWILIGHT_OF_THE_GODS: &str = "DOOM_TWILIGHT_OF_THE_GODS";
pub const SKY_FALLS: &str = "DOOM_SKY_FALLS";
pub const GATHERING: &str = "DOOM_GATHERING";
pub const LONG_NIGHT: &str = "DOOM_LONG_NIGHT";
pub const GODLESS_AGE: &str = "DOOM_GODLESS_AGE";
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
    DoomCard { key: LAIRS_DEEPEN, stages: &[60], fire: fire_nothing, valid: Some(lairs_spawn), on_reveal: None },
    DoomCard { key: SECOND_SUN, stages: &[60], fire: fire_second_sun, valid: Some(valid_second_sun), on_reveal: None },
    DoomCard { key: TWILIGHT_OF_THE_GODS, stages: &[80], fire: fire_twilight_of_the_gods, valid: Some(valid_twilight_of_the_gods), on_reveal: None },
    DoomCard { key: SKY_FALLS, stages: &[80], fire: fire_sky_falls, valid: None, on_reveal: None },
    DoomCard { key: GATHERING, stages: &[100], fire: fire_gathering, valid: Some(valid_gathering), on_reveal: None },
    DoomCard { key: LONG_NIGHT, stages: &[100], fire: fire_long_night, valid: None, on_reveal: None },
    DoomCard { key: GODLESS_AGE, stages: &[100], fire: fire_godless_age, valid: Some(valid_godless_age), on_reveal: None },
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

/// A card that only names its targets, or whose rule asks whether it has fired, does nothing
/// when it fires.
fn fire_nothing(_: &mut Game, _: usize) {}

fn targets(g: &Game, key: &str, stage: usize) -> Vec<PlotIdx> {
    g.doom_targets(key, stage).iter().filter(|&&i| i >= 0 && (i as usize) < g.map.plots.len()).map(|&i| PlotIdx(i as u32)).collect()
}

// ---------------------------------------------------------------------------------------------
// A01 The Wild Awakens (20) and A30 The Lairs Deepen (60): the lairs.

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
// A32 The Second Sun (60).

/// The terrain one step warmer: Snow to Tundra, Tundra to Plains (`None`: no change).
fn warmer(d: &GameData, t: TerrainType) -> Option<TerrainType> {
    if t == d.t::<TerrainType>("TERRAIN_SNOW") {
        Some(d.t::<TerrainType>("TERRAIN_TUNDRA"))
    } else if t == d.t::<TerrainType>("TERRAIN_TUNDRA") {
        Some(d.t::<TerrainType>("TERRAIN_PLAINS"))
    } else {
        None
    }
}

fn valid_second_sun(g: &Game) -> bool {
    g.map.plots.iter().any(|pl| pl.terrain_type.is_some_and(|t| warmer(g.d, t).is_some()))
}

/// Every plot's terrain one step from what it was before the card: Snow becomes Tundra, Tundra
/// becomes Plains (a temporary terrain's real one too). A feature the new terrain cannot have
/// goes (a Blizzard on new Plains; a feature bound to no terrain stays).
fn fire_second_sun(g: &mut Game, _: usize) {
    let d = g.d;
    for p in g.map.plot_ids() {
        if let Some(real) = g.map.plots[p].real_terrain_type.and_then(|t| warmer(d, t)) {
            g.map.plots[p].real_terrain_type = Some(real);
        }
        let Some(new) = g.map.plots[p].terrain_type.and_then(|t| warmer(d, t)) else { continue };
        g.plot_set_terrain_type(p, new, true);
        let bound = |f: FeatureType| d.features[f].terrain.iter().any(|&b| b);
        if g.map.plots[p].feature_type.is_some_and(|f| bound(f) && !d.features[f].terrain[new]) {
            g.plot_set_feature_type(p, None);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A39 The Twilight of the Gods (80) and A50 The Godless Age (100): religions.

/// The shrines of `religion`: its buildings that need its holy city and give its global
/// commerce.
fn shrines(d: &GameData, religion: ReligionType) -> Vec<BuildingType> {
    d.buildings.keys().filter(|&b| d.buildings[b].holy_city.is_some() && d.buildings[b].global_religion_commerce == Some(religion)).collect()
}

/// The (city, shrine) pairs of the holy cities that have their religion's shrine.
fn holy_shrines(g: &Game) -> Vec<(CityId, BuildingType)> {
    let d = g.d;
    let mut out = Vec::new();
    for r in d.religions.keys() {
        let Some(c) = g.get_holy_city(r) else { continue };
        for b in shrines(d, r) {
            if g.cities[c].num_real_building[b] > 0 {
                out.push((c, b));
            }
        }
    }
    out
}

fn valid_twilight_of_the_gods(g: &Game) -> bool {
    !holy_shrines(g).is_empty()
}

/// Every religion's holy city loses its shrine.
fn fire_twilight_of_the_gods(g: &mut Game, _: usize) {
    for (c, b) in holy_shrines(g) {
        g.city_set_num_real_building(c, b, 0);
    }
}

fn valid_godless_age(g: &Game) -> bool {
    g.cities.iter().any(|(_, c)| c.has_religion.iter().any(|&h| h)) || g.player_ids().any(|p| g.players[p].last_state_religion.is_some())
}

/// The shrines and holy cities are gone, every religion leaves every city, and every state
/// religion ends.
fn fire_godless_age(g: &mut Game, _: usize) {
    let d = g.d;
    for (c, b) in holy_shrines(g) {
        g.city_set_num_real_building(c, b, 0);
    }
    for r in d.religions.keys() {
        if g.get_holy_city(r).is_some() {
            g.set_holy_city(r, None, false);
        }
    }
    for c in g.cities.ids() {
        for r in d.religions.keys() {
            if g.cities.contains(c) && g.cities[c].has_religion[r] {
                g.city_set_has_religion(c, r, false, false, false);
            }
        }
    }
    for p in g.player_ids() {
        if g.players[p].alive {
            g.player_set_last_state_religion(p, None);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A40 The Sky Falls (80).

/// Meteors strike the ten largest cities: units there take damage equal to 50% of their hit
/// points, and each city loses 2 population.
fn fire_sky_falls(g: &mut Game, _: usize) {
    for c in cities_by_size(g).into_iter().take(10) {
        if !g.cities.contains(c) {
            continue;
        }
        let p = g.city_plot(c);
        damage_units(g, p, 50);
        lose_population(g, c, 2);
    }
}

// ---------------------------------------------------------------------------------------------
// A48 The Gathering (100).

/// How far apart two capitals may be.
const GATHERING_RANGE: i32 = 15;

/// The teams the Gathering acts on: alive, with a civilization player (not the barbarians, a
/// minor civ or a crusade Host) that has a capital.
fn gathering_teams(g: &Game) -> Vec<TeamId> {
    let civs = civ_players(g);
    g.team_ids().filter(|&t| civs.iter().any(|&p| g.players[p].team == t && g.player_get_capital_city(p).is_some())).collect()
}

/// Some capital of `a` is within `GATHERING_RANGE` plots of some capital of `b`.
fn capitals_near(g: &Game, a: TeamId, b: TeamId) -> bool {
    let capitals = |t: TeamId| -> Vec<(i32, i32)> {
        g.player_ids()
            .filter(|&p| g.players[p].alive && g.players[p].team == t)
            .filter_map(|p| g.player_get_capital_city(p))
            .map(|c| (g.cities[c].x, g.cities[c].y))
            .collect()
    };
    let cb = capitals(b);
    capitals(a).iter().any(|&(ax, ay)| cb.iter().any(|&(bx, by)| g.plot_distance(ax, ay, bx, by) <= GATHERING_RANGE))
}

/// The strongest team within range of `t` that may take a vassal (not itself a vassal), from
/// `teams` (ties: the lower team id).
fn strongest_near(g: &Game, t: TeamId, teams: &[TeamId], power: &dyn Fn(TeamId) -> i32) -> Option<TeamId> {
    teams.iter().copied().filter(|&m| m != t && !g.team_is_a_vassal(m) && capitals_near(g, t, m)).max_by_key(|&m| (power(m), Reverse(m)))
}

fn valid_gathering(g: &Game) -> bool {
    if g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) {
        return false;
    }
    let teams = gathering_teams(g);
    let power = |t: TeamId| g.team_get_power(t, false);
    teams.iter().any(|&t| !g.team_is_a_vassal(t) && strongest_near(g, t, &teams, &power).is_some_and(|m| power(t) * 2 < power(m)))
}

/// Every civilization with less than half the power of the strongest civilization within 15
/// plots, capital to capital, becomes its vassal (it capitulates). From the strongest down, on
/// the powers before the card: a team that became a vassal takes none. These vassals do not
/// count toward their master's victory (`Team::gathered`). Under No Vassal States the card is
/// never played (`valid_gathering`: dealt again at the reveal).
fn fire_gathering(g: &mut Game, _: usize) {
    if g.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) {
        return;
    }
    let teams = gathering_teams(g);
    let before: Vec<(TeamId, i32)> = teams.iter().map(|&t| (t, g.team_get_power(t, false))).collect();
    let power = |t: TeamId| before.iter().find(|&&(x, _)| x == t).map_or(0, |&(_, v)| v);
    let mut order = teams.clone();
    order.sort_by_key(|&t| (Reverse(power(t)), t));
    for t in order {
        if g.team_is_a_vassal(t) || !g.team_is_alive(t) {
            continue;
        }
        let Some(m) = strongest_near(g, t, &teams, &power) else { continue };
        if power(t) * 2 < power(m) {
            if !g.teams[t].has_met[m] {
                g.team_meet(t, m, false);
            }
            g.team_assign_vassal(m, t, true);
            g.teams[t].gathered = true;
            let turn = g.game_turn();
            g.log.msg(turn, None, || format!("The Gathering: team {t} becomes the vassal of team {m}."));
        }
    }
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
    fn the_lairs_deepen_changes_what_every_lair_spawns() {
        let mut g = game(2);
        let d = g.d;
        for info in d.improvements.iter() {
            if info.spawn_unit_type.is_some() {
                assert!(info.deep_spawn_unit_type.is_some(), "{}: a lair deepens", info.base.type_);
            }
        }
        let barrow = d.t::<ImprovementType>("IMPROVEMENT_BARROW");
        let p = open_land(&g)[0];
        g.plot_set_improvement_type(p, Some(barrow));
        assert_eq!(g.plot_lair_spawn_unit(p), Some(d.t::<UnitType>("UNIT_SKELETON")));
        play(&mut g, LAIRS_DEEPEN, vec![]);
        assert_eq!(g.plot_lair_spawn_unit(p), Some(d.t::<UnitType>("UNIT_WRAITH")));
        // What spawns from then on is the deeper kind.
        let barbarian = g.barbarian_player();
        fire_wild_awakens(&mut g, 0);
        assert_eq!(count(&g, p, barbarian, "UNIT_WRAITH"), 3);
        assert_eq!(count(&g, p, barbarian, "UNIT_SKELETON"), 0);
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
    fn the_second_sun_warms_ice_and_tundra_one_step() {
        let mut g = game(2);
        let d = g.d;
        let t = |n: &str| d.t::<TerrainType>(n);
        let blizzard = d.t::<FeatureType>("FEATURE_BLIZZARD");
        let land = open_land(&g);
        let (ice, ice_storm, tundra_storm, plains) = (land[0], land[1], land[2], land[3]);
        for (p, name) in [(ice, "TERRAIN_SNOW"), (ice_storm, "TERRAIN_SNOW"), (tundra_storm, "TERRAIN_TUNDRA"), (plains, "TERRAIN_PLAINS")] {
            g.plot_set_feature_type(p, None);
            g.plot_set_terrain_type(p, t(name), true);
        }
        g.plot_set_feature_type(ice_storm, Some(blizzard));
        g.plot_set_feature_type(tundra_storm, Some(blizzard));
        assert!(valid_second_sun(&g));
        fire_second_sun(&mut g, 0);
        assert_eq!(g.map.plots[ice].terrain_type, Some(t("TERRAIN_TUNDRA")), "one step, not two");
        assert_eq!(g.map.plots[ice_storm].terrain_type, Some(t("TERRAIN_TUNDRA")));
        assert_eq!(g.map.plots[ice_storm].feature_type, Some(blizzard), "a Blizzard can stay on Tundra");
        assert_eq!(g.map.plots[tundra_storm].terrain_type, Some(t("TERRAIN_PLAINS")));
        assert_eq!(g.map.plots[tundra_storm].feature_type, None, "a Blizzard cannot be on Plains");
        assert_eq!(g.map.plots[plains].terrain_type, Some(t("TERRAIN_PLAINS")), "no Plains to Desert");
        assert!(!g.map.plots.iter().any(|pl| pl.terrain_type == Some(t("TERRAIN_SNOW"))));
    }

    /// A holy city of the Order with its shrine, and a second city of the Order and the Runes.
    fn religious_world() -> (Game, CityId, CityId, ReligionType, ReligionType, BuildingType) {
        let mut g = game(2);
        let d = g.d;
        let p0 = civ(&g, 0);
        let land = open_land(&g);
        let holy = city_at(&mut g, p0, land[0], 4);
        let other = city_at(&mut g, p0, land[land.len() / 2], 4);
        let order = d.t::<ReligionType>("RELIGION_THE_ORDER");
        let runes = d.t::<ReligionType>("RELIGION_RUNES_OF_KILMORPH");
        g.set_holy_city(order, Some(holy), false);
        g.city_set_has_religion(other, order, true, false, false);
        g.city_set_has_religion(other, runes, true, false, false);
        let shrine = d.t::<BuildingType>("BUILDING_CODE_OF_JUNIL");
        g.city_set_num_real_building(holy, shrine, 1);
        g.player_set_last_state_religion(p0, Some(order));
        (g, holy, other, order, runes, shrine)
    }

    #[test]
    fn the_twilight_of_the_gods_takes_the_holy_cities_shrines() {
        let (mut g, holy, _, order, _, shrine) = religious_world();
        assert!(valid_twilight_of_the_gods(&g));
        fire_twilight_of_the_gods(&mut g, 0);
        assert_eq!(g.cities[holy].num_real_building[shrine], 0);
        assert_eq!(g.get_holy_city(order), Some(holy), "it stays the holy city");
        assert!(g.cities[holy].has_religion[order]);
        assert!(!valid_twilight_of_the_gods(&g));
    }

    #[test]
    fn the_godless_age_ends_every_religion() {
        let (mut g, holy, other, order, runes, shrine) = religious_world();
        let p0 = civ(&g, 0);
        assert!(valid_godless_age(&g));
        fire_godless_age(&mut g, 0);
        assert_eq!(g.cities[holy].num_real_building[shrine], 0);
        assert_eq!(g.get_holy_city(order), None);
        for c in [holy, other] {
            assert!(!g.cities[c].has_religion[order] && !g.cities[c].has_religion[runes]);
        }
        assert_eq!(g.players[p0].last_state_religion, None);
        assert!(!valid_godless_age(&g));
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

    #[test]
    fn the_gathering_makes_the_weak_neighbour_a_vassal_that_wins_nothing_for_its_master() {
        let mut g = game(3);
        let (a, b, c) = (civ(&g, 0), civ(&g, 1), civ(&g, 2));
        let land = open_land(&g);
        let pa = land[land.len() / 2];
        let pb = *land.iter().find(|&&p| (3..=15).contains(&dist(&g, p, pa))).unwrap();
        let pc = *land.iter().find(|&&p| dist(&g, p, pa) > 15 && dist(&g, p, pb) > 15).unwrap();
        city_at(&mut g, a, pa, 6);
        city_at(&mut g, b, pb, 4);
        city_at(&mut g, c, pc, 2);
        g.players[a].power = 100;
        g.players[b].power = 30;
        g.players[c].power = 5;
        let (ta, tb, tc) = (g.players[a].team, g.players[b].team, g.players[c].team);
        assert!(valid_gathering(&g));
        fire_gathering(&mut g, 0);
        assert!(g.teams[tb].vassal[ta] && g.teams[tb].gathered && g.teams[tb].capitulated);
        assert!(!g.team_is_a_vassal(ta) && !g.team_is_a_vassal(tc), "too far, or the strongest");
        // The vassal counts for its master's score totals, not for a victory.
        assert_eq!(g.team_victory_population(ta), g.teams[ta].total_population);
        assert_eq!(g.team_victory_land(ta), g.teams[ta].total_land);
        assert_eq!(g.team_get_total_population(ta, true), g.teams[ta].total_population + g.teams[tb].total_population / 2);
        // Freed, it counts as any vassal would if it were made one again.
        g.team_set_vassal(tb, ta, false, false);
        assert!(!g.teams[tb].gathered);
        // Under No Vassal States the card is not played.
        g.setup.options[GameOptionType::GAMEOPTION_NO_VASSAL_STATES] = true;
        assert!(!valid_gathering(&g));
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
    /// by hand, a holy city with its shrine, a lair, wolves and ships: every new card is dealt to
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
        let order = d.t::<ReligionType>("RELIGION_THE_ORDER");
        g.set_holy_city(order, Some(cities[0]), false);
        g.city_set_num_real_building(cities[0], d.t::<BuildingType>("BUILDING_CODE_OF_JUNIL"), 1);
        let believer = g.cities[cities[0]].owner;
        g.player_set_last_state_religion(believer, Some(order));
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
