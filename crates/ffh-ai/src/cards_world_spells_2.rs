//! The AI's use of batch B7's cards (docs/CARDS.md; the rules are
//! `ffh_core::ffh::cards::world_spells_2`), once a turn from `cards::turn`, before the units move.
//! The moments are the ones an expert would pick; the choices are ad hoc (CLAUDE.md, "AI changes":
//! the cards are a rules change).
//!
//! - **Sing the Stones:** at once, when a City Ruins can be refounded.
//! - **Dragon's Oath:** at once (Sylvarith fights for us from that turn).
//! - **Call the Leviathan:** at once, in the coastal capital, else the largest coastal city.
//! - **Judgment of Junil:** a war we are losing ends (an enemy civilization of half again our
//!   power, its war success against us at least ours); else, while we fight or prepare a war, the
//!   strongest other neighbour we can reach (of at least four fifths of our power) is held to
//!   peace.
//! - **Earthshaper's Decree:** once it improves at least [`DECREE_MIN_PLOTS`] plots and three per
//!   city.
//! - **The Plagiarist's Lens:** at once, on the card of the highest band (then the most holders).
//! - **Rituals:** at war with a civilization, a city of ours not in peril that finishes the ritual
//!   within [`RITUAL_MAX_TURNS`] turns starts it, at the front of its queue: the Pillar of Dawn
//!   where an enemy army (3 units) stands within 5 plots (no Pillar of ours standing), the Weather
//!   Engine near a front (an enemy city, or an enemy army, within 10 plots), the Tidecall where an
//!   enemy's improved shore lies within 3 plots. One of each at a time.
//! - **Weathers:** a Storm on a city of ours that enemy shooters or siege engines threaten; a
//!   Blizzard on an enemy stack in or by our land; a Fog on an enemy city our army stands next to.
//! - **Tidecall:** the enemy's improved shore plot that yields most.
//!
//! The Second Sky is a building: [`building_value`] values its reveal.

use ffh_core::ffh::cards::world_spells_2 as ws;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::{AreaEffectKind, SpellTarget};
use ffh_data::*;

use crate::view::View;

/// Earthshaper's Decree: the fewest plots it improves for the AI to cast it.
pub const DECREE_MIN_PLOTS: usize = 8;
/// The most turns a city takes to finish a ritual the AI starts.
pub const RITUAL_MAX_TURNS: i32 = 8;

/// The batch's turn (`cards::TURNS`).
pub fn turn(g: &mut Game, v: &View) {
    let p = v.p;
    if v.barbarian || !g.players[p].alive {
        return;
    }
    sing_the_stones(g, p);
    dragons_oath(g, p);
    call_the_leviathan(g, p);
    judgment_of_junil(g, v);
    earthshapers_decree(g, p);
    plagiarists_lens(g, p);
    rituals(g, v);
    weathers(g, v);
    tidecalls(g, v);
}

/// `name`'s spell if `p` may cast it now: its card is known and, for a card's world spell, not
/// cast yet.
fn ready(g: &Game, p: PlayerId, name: &str) -> Option<SpellType> {
    let spell = g.d.ty::<SpellType>(name)?;
    let info = &g.d.spells[spell];
    let team = g.players[p].team;
    (g.team_is_has_tech(team, info.tech_prereq) && !(info.once && g.players[p].once_spells.contains(&spell))).then_some(spell)
}

/// The unit of `p` that casts `spell` from `units` (a city defender first, then one that loses no
/// promotion by casting, then the oldest), if one can now (`target`: at it).
fn caster(g: &Game, units: &[UnitId], spell: SpellType, target: Option<SpellTarget>) -> Option<UnitId> {
    let d = g.d;
    let loses = |u: UnitId| d.promotions.keys().filter(|&pr| g.units[u].has_promotion[pr] && d.promotions[pr].removed_by_casting).count();
    units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u) && g.unit_march(u).is_none())
        .filter(|&u| match target {
            Some(t) => g.unit_can_cast_at(u, spell, t),
            None => g.unit_can_cast(u, spell, false),
        })
        .min_by_key(|&u| (g.units[u].ai_type != Some(UnitAIType::UNITAI_CITY_DEFENSE), loses(u), u))
}

/// Casts `spell` (at `target`) with the best of `units` that can; true if cast.
fn cast(g: &mut Game, units: &[UnitId], spell: SpellType, target: Option<SpellTarget>, why: &str) -> bool {
    let Some(u) = caster(g, units, spell, target) else { return false };
    let p = g.units[u].owner;
    crate::trace!(g, p, "card spell {} at {:?} ({})", g.d.spells[spell].base.type_, target, why);
    g.tel_cast_reason(u, "card", 0);
    match target {
        Some(t) => g.unit_cast_at(u, spell, t),
        None => g.unit_cast(u, spell),
    }
    true
}

/// `p`'s units.
fn all_units(g: &Game, p: PlayerId) -> Vec<UnitId> {
    g.players[p].units.clone()
}

/// `p`'s units in city `c`.
fn units_in(g: &Game, p: PlayerId, c: CityId) -> Vec<UnitId> {
    g.map.plots[g.city_plot(c)].units.iter().copied().filter(|&u| g.units[u].owner == p).collect()
}

/// At war with a civilization (not only the barbarians).
fn civ_war(v: &View) -> bool {
    !v.civ_enemies.is_empty()
}

// ---------------------------------------------------------------------------------------------
// The world spells.

fn sing_the_stones(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_SING_THE_STONES") else { return };
    let ruins = ws::sing_the_stones_ruins(g, p);
    if !ruins.is_empty() {
        let units = all_units(g, p);
        cast(g, &units, spell, None, &format!("{} ruins", ruins.len()));
    }
}

fn dragons_oath(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_DRAGONS_OATH") else { return };
    if ws::dragons_oath_city(g, p).is_some() {
        let units = all_units(g, p);
        cast(g, &units, spell, None, "Sylvarith");
    }
}

/// The coastal city the beast comes to: the capital if coastal, else the most populous.
pub fn leviathan_city(g: &Game, p: PlayerId, caster: UnitId) -> Option<CityId> {
    let cities: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c) && ws::leviathan_city_ok(g, caster, c)).collect();
    cities.iter().copied().max_by_key(|&c| (g.city_is_capital(c), g.cities[c].population, std::cmp::Reverse(c)))
}

fn call_the_leviathan(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_CALL_THE_LEVIATHAN") else { return };
    let units = all_units(g, p);
    let Some(&any) = units.iter().find(|&&u| g.unit_can_cast_untargeted(u, spell, false)) else { return };
    if let Some(c) = leviathan_city(g, p, any) {
        cast(g, &units, spell, Some(SpellTarget::City(c)), "a sea beast");
    }
}

/// Judgment of Junil's target for `v.p`: the stronger enemy civilization of a war we are losing,
/// else a strong neighbour while we fight or prepare a war elsewhere.
pub fn junil_target(g: &Game, v: &View, caster: UnitId) -> Option<PlayerId> {
    let team = v.team;
    let ours = g.team_get_power(team, true).max(1) as i64;
    let candidates: Vec<PlayerId> = g.player_ids().filter(|&o| ws::junil_ok(g, caster, o)).collect();
    // A war we are losing: the enemy of half again our power that we are not beating (its war
    // success against us at least ours), the strongest first.
    let losing = candidates
        .iter()
        .copied()
        .filter(|&o| {
            let ot = g.players[o].team;
            g.at_war(team, ot) && g.teams[ot].ai_war_success[team] >= g.teams[team].ai_war_success[ot]
        })
        .map(|o| (g.team_get_power(g.players[o].team, true) as i64, o))
        .filter(|&(power, _)| power * 2 > ours * 3)
        .max_by_key(|&(power, o)| (power, std::cmp::Reverse(o)));
    if let Some((_, o)) = losing {
        return Some(o);
    }
    // A flank to hold while we fight or prepare a war.
    let busy = civ_war(v) || g.team_ids().any(|t| matches!(g.teams[team].ai_war_plan[t], Some(WarPlanType::WARPLAN_PREPARING_LIMITED) | Some(WarPlanType::WARPLAN_PREPARING_TOTAL)));
    if !busy {
        return None;
    }
    candidates
        .iter()
        .copied()
        .filter(|&o| {
            let ot = g.players[o].team;
            !g.at_war(team, ot) && g.teams[team].ai_war_plan[ot].is_none() && crate::diplomacy::reach_distance(g, team, ot).is_some()
        })
        .map(|o| (g.team_get_power(g.players[o].team, true) as i64, o))
        .filter(|&(power, _)| power * 5 >= ours * 4)
        .max_by_key(|&(power, o)| (power, std::cmp::Reverse(o)))
        .map(|(_, o)| o)
}

fn judgment_of_junil(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(spell) = ready(g, p, "SPELL_JUDGMENT_OF_JUNIL") else { return };
    let units = all_units(g, p);
    let Some(&any) = units.iter().find(|&&u| g.unit_can_cast_untargeted(u, spell, false)) else { return };
    if let Some(o) = junil_target(g, v, any) {
        let why = if g.at_war(v.team, g.players[o].team) { "end a losing war" } else { "hold a flank" };
        cast(g, &units, spell, Some(SpellTarget::Player(o)), why);
    }
}

fn earthshapers_decree(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_EARTHSHAPERS_DECREE") else { return };
    let n = ws::decree_plots(g, p).len();
    if n >= DECREE_MIN_PLOTS && n >= 3 * g.players[p].cities.len() {
        let units = all_units(g, p);
        cast(g, &units, spell, None, &format!("{n} plots"));
    }
}

/// The card the Lens copies for `p`: the highest band, then the most holders, then the lowest id.
pub fn lens_choice(g: &Game, p: PlayerId) -> Option<TechType> {
    let d = g.d;
    let holders = |t: TechType| g.team_ids().filter(|&o| g.team_is_alive(o) && g.team_is_has_tech(o, Some(t))).count();
    ws::lens_cards(g, p).into_iter().max_by_key(|&t| (d.techs[t].era, holders(t), std::cmp::Reverse(t)))
}

fn plagiarists_lens(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_PLAGIARISTS_LENS") else { return };
    if let Some(card) = lens_choice(g, p) {
        let units = all_units(g, p);
        cast(g, &units, spell, Some(SpellTarget::Choice(card.idx() as i32)), "copy a card");
    }
}

// ---------------------------------------------------------------------------------------------
// The rituals.

/// Does `p` know the card of ritual `name`?
fn ritual(g: &Game, p: PlayerId, name: &str) -> Option<ProjectType> {
    let pr = g.d.ty::<ProjectType>(name)?;
    g.team_is_has_tech(g.players[p].team, g.d.projects[pr].tech_prereq).then_some(pr)
}

/// Is ritual `pr` in a queue of `p`'s, or charged in one of its cities?
fn ritual_pending(g: &Game, p: PlayerId, pr: ProjectType) -> bool {
    g.players[p].cities.iter().any(|&c| {
        let city = &g.cities[c];
        city.ritual_charges.contains(&pr) || city.order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == pr.i())
    })
}

/// Starts ritual `pr` at the front of city `c`'s queue if it may build it within
/// `RITUAL_MAX_TURNS` turns; true if started.
fn start_ritual(g: &mut Game, c: CityId, pr: ProjectType, why: &str) -> bool {
    if !g.city_can_create(c, pr, false, false) || g.city_production_turns_left_project(c, pr, 0) > RITUAL_MAX_TURNS {
        return false;
    }
    // (An army's requisition stays at the head of the queue.)
    if g.cities[c].order_queue.first().is_some_and(|o| o.requisition) {
        return false;
    }
    let p = g.cities[c].owner;
    crate::trace!(g, p, "card ritual {} in {} ({})", g.d.projects[pr].base.type_, g.cities[c].name, why);
    g.city_push_order(c, OrderType::ORDER_CREATE, pr.i(), -1, false, false, false, false);
    true
}

/// The visible enemy units (of civilizations at war with us) within `range` plots of `plot`.
fn enemies_near(g: &Game, v: &View, plot: PlotIdx, range: i32) -> Vec<UnitId> {
    let (x, y) = g.map.xy(plot);
    let mut out = Vec::new();
    for dy in -range..=range {
        for dx in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            if !g.plot_is_visible(q, v.team) {
                continue;
            }
            for &u in &g.map.plots[q].units {
                let t = g.unit_get_team(u);
                if v.civ_enemies.contains(&t) && !g.unit_is_invisible(u, v.team, false) && g.unit_can_fight(u) {
                    out.push(u);
                }
            }
        }
    }
    out
}

/// The enemy cities (of civilizations at war with us) we see within `range` plots of `plot`.
fn enemy_cities_near(g: &Game, v: &View, plot: PlotIdx, range: i32) -> Vec<CityId> {
    let (x, y) = g.map.xy(plot);
    let mut out = Vec::new();
    for pl in g.players.iter() {
        if !pl.alive || !v.civ_enemies.contains(&pl.team) {
            continue;
        }
        for &c in &pl.cities {
            if g.cities.contains(c) && g.plot_is_visible(g.city_plot(c), v.team) && g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= range {
                out.push(c);
            }
        }
    }
    out
}

fn rituals(g: &mut Game, v: &View) {
    let p = v.p;
    if !civ_war(v) {
        return;
    }
    // A city in peril builds its defence, not a ritual.
    let cities: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| !v.peril.iter().any(|&(q, _)| q == g.city_plot(c))).collect();
    // The Pillar of Dawn: where an enemy army (3 units or more) stands within its reach, while no
    // Pillar of ours stands; the city with the most production first.
    if let Some(pr) = ritual(g, p, "PROJECT_PILLAR_OF_DAWN") {
        let standing = g.area_effects.iter().any(|e| e.owner == p && e.kind == AreaEffectKind::PillarOfDawn && e.until > g.game_turn());
        if !standing && !ritual_pending(g, p, pr) {
            let mut fronts: Vec<CityId> = cities.iter().copied().filter(|&c| enemies_near(g, v, g.city_plot(c), ws::PILLAR_RADIUS).len() >= 3).collect();
            fronts.sort_by_key(|&c| (std::cmp::Reverse(g.city_current_production_difference(c, false, false)), c));
            for c in fronts {
                if start_ritual(g, c, pr, "an enemy army near") {
                    break;
                }
            }
        }
    }
    // The Weather Engine: near a front.
    if let Some(pr) = ritual(g, p, "PROJECT_WEATHER_ENGINE") {
        if !ritual_pending(g, p, pr) {
            let mut fronts: Vec<CityId> = cities
                .iter()
                .copied()
                .filter(|&c| {
                    let plot = g.city_plot(c);
                    !enemy_cities_near(g, v, plot, ws::WEATHER_RANGE).is_empty() || enemies_near(g, v, plot, ws::WEATHER_RANGE).len() >= 3
                })
                .collect();
            fronts.sort_by_key(|&c| (std::cmp::Reverse(g.city_current_production_difference(c, false, false)), c));
            for c in fronts {
                if start_ritual(g, c, pr, "a front") {
                    break;
                }
            }
        }
    }
    // The Tidecall: where an enemy's improved shore lies within reach.
    if let Some(pr) = ritual(g, p, "PROJECT_TIDECALL_OF_DANALIN") {
        if !ritual_pending(g, p, pr) {
            for c in cities {
                if !tidecall_plots(g, v, c).is_empty() && start_ritual(g, c, pr, "an enemy shore") {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Spending the charges.

/// Does unit `u` shoot or bombard in a battle (a row of its type does)?
fn ranged(g: &Game, u: UnitId) -> bool {
    g.d.units[g.units[u].unit_type].engage_rows.as_ref().is_some_and(|rows| rows.iter().any(|r| matches!(r.kind, ActionKind::Shoot | ActionKind::Bombard)))
}

/// The weather `v.p` casts from city `c` (holding a Weather Engine charge), if one serves now: the
/// spell's name and its centre.
pub fn weather_choice(g: &Game, v: &View, c: CityId) -> Option<(&'static str, PlotIdx)> {
    let p = v.p;
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let in_range = |t: PlotIdx| {
        let (x, y) = g.map.xy(t);
        g.plot_distance(cx, cy, x, y) <= ws::WEATHER_RANGE
    };
    // A Storm over a city of ours that enemy shooters or siege engines threaten.
    for &oc in &g.players[p].cities {
        let plot = g.city_plot(oc);
        if !in_range(plot) || g.plot_has_weather(plot, AreaEffectKind::Storm) {
            continue;
        }
        let near = enemies_near(g, v, plot, 3);
        if near.len() >= 3 && near.iter().filter(|&&u| ranged(g, u)).count() >= 2 {
            return Some(("SPELL_WEATHER_STORM", plot));
        }
    }
    // A Fog on an enemy city our army stands next to.
    for e in enemy_cities_near(g, v, g.city_plot(c), ws::WEATHER_RANGE) {
        let ep = g.city_plot(e);
        if g.plot_has_weather(ep, AreaEffectKind::Fog) {
            continue;
        }
        let (ex, ey) = (g.cities[e].x, g.cities[e].y);
        let mut ours = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(q) = g.plot_xy(ex, ey, dx, dy) {
                    ours += g.map.plots[q].units.iter().filter(|&&u| g.units[u].owner == p && crate::military::is_army_unit(g, u)).count();
                }
            }
        }
        if ours >= 3 {
            return Some(("SPELL_WEATHER_FOG", ep));
        }
    }
    // A Blizzard on the largest enemy stack in or next to our land.
    let mut best: Option<(usize, PlotIdx)> = None;
    for u in enemies_near(g, v, g.city_plot(c), ws::WEATHER_RANGE) {
        let q = g.unit_plot(u);
        let ours_near = ffh_core::ffh::cards::world::within_range_of_borders(g, p, q, 1);
        if !ours_near || g.plot_has_weather(q, AreaEffectKind::Blizzard) {
            continue;
        }
        let n = g.map.plots[q].units.iter().filter(|&&o| v.civ_enemies.contains(&g.unit_get_team(o))).count();
        if n >= 3 && best.map_or(true, |(b, bq)| n > b || n == b && q < bq) {
            best = Some((n, q));
        }
    }
    best.map(|(_, q)| ("SPELL_WEATHER_BLIZZARD", q))
}

fn weathers(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(pr) = g.d.ty::<ProjectType>("PROJECT_WEATHER_ENGINE") else { return };
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) || !g.cities[c].ritual_charges.contains(&pr) {
            continue;
        }
        let Some((name, q)) = weather_choice(g, v, c) else { continue };
        let Some(spell) = g.d.ty::<SpellType>(name) else { continue };
        let units = units_in(g, p, c);
        cast(g, &units, spell, Some(SpellTarget::Plot(q)), name);
    }
}

/// The plots city `c` of `v.p`'s Tidecall would drown with profit: enemy-owned (at war) improved
/// shore plots it may drown (any unit of ours in the city would do), best first (the most yield).
fn tidecall_plots(g: &Game, v: &View, c: CityId) -> Vec<PlotIdx> {
    let p = v.p;
    let (cx, cy) = (g.cities[c].x, g.cities[c].y);
    let mut out: Vec<(i32, PlotIdx)> = Vec::new();
    for dy in -ws::TIDECALL_RANGE..=ws::TIDECALL_RANGE {
        for dx in -ws::TIDECALL_RANGE..=ws::TIDECALL_RANGE {
            let Some(t) = g.plot_xy(cx, cy, dx, dy) else { continue };
            let plot = &g.map.plots[t];
            let (x, y) = g.map.xy(t);
            let enemy = plot.owner.is_some_and(|o| o != p && v.civ_enemies.contains(&g.players[o].team));
            if !enemy || plot.improvement_type.is_none() || plot.plot_city.is_some() || g.plot_is_water(t) || g.plot_is_hills(t) || g.plot_is_peak(t) {
                continue;
            }
            if g.plot_distance(cx, cy, x, y) > ws::TIDECALL_RANGE || !g.plot_is_adjacent_to_water(t) || !g.plot_is_visible(t, v.team) {
                continue;
            }
            let yields: i32 = (0..YieldType::NUM).map(|i| g.plot_calculate_yield(t, YieldType::from_usize(i))).sum();
            out.push((yields, t));
        }
    }
    out.sort_by_key(|&(y, t)| (std::cmp::Reverse(y), t));
    out.dedup_by_key(|&mut (_, t)| t);
    out.into_iter().map(|(_, t)| t).collect()
}

fn tidecalls(g: &mut Game, v: &View) {
    let p = v.p;
    let Some(pr) = g.d.ty::<ProjectType>("PROJECT_TIDECALL_OF_DANALIN") else { return };
    let Some(spell) = g.d.ty::<SpellType>("SPELL_TIDECALL_OF_DANALIN") else { return };
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) || !g.cities[c].ritual_charges.contains(&pr) {
            continue;
        }
        let units = units_in(g, p, c);
        for t in tidecall_plots(g, v, c) {
            if cast(g, &units, spell, Some(SpellTarget::Plot(t)), "an enemy shore") {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The Second Sky.

/// The value of building `b` in city `c` for its sites revealed (`reveals_sites`, the Second Sky):
/// worth most early, while the map is unknown and the mana, lairs and huts it shows are still
/// there to take; nothing once its owner has it.
pub fn building_value(g: &Game, c: CityId, b: BuildingType, _threat: i32) -> i32 {
    let d = g.d;
    if !d.buildings[b].reveals_sites {
        return 0;
    }
    let team = g.city_team(c);
    let hidden = g.map.plot_ids().filter(|&t| !g.map.plots[t].revealed[team] && ws::is_site(g, t)).count() as i32;
    let early = (200 - g.game_turn()).max(0);
    hidden.min(20) * 20 + early * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game;

    const US: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    fn learn(g: &mut Game, p: PlayerId, card: &str) {
        let team = g.players[p].team;
        g.team_set_has_tech(team, g.d.t::<TechType>(card), true, Some(p), false, false);
    }

    fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let t = g.d.t::<UnitType>(name);
        g.player_init_unit(p, t, x, y, None, None)
    }

    fn war(g: &mut Game) {
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
    }

    fn play(g: &mut Game) {
        let v = View::new(g, US);
        turn(g, &v);
    }

    #[test]
    fn dragons_oath_and_the_lens_are_cast_at_once() {
        let mut g = one_landmass_game(51);
        learn(&mut g, US, "TECH_CARD_DRAGONS_OATH");
        learn(&mut g, US, "TECH_CARD_PLAGIARISTS_LENS");
        g.player_found(US, 4, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        let card = g.d.t::<TechType>("TECH_CARD_ASTRAL_LEAP");
        g.team_set_has_tech(t1, card, true, Some(THEM), false, false);
        play(&mut g);
        let sylvarith = g.d.t::<UnitType>("UNIT_SYLVARITH");
        assert!(g.players[US].units.iter().any(|&u| g.units[u].unit_type == sylvarith));
        assert!(g.team_is_has_tech(t0, Some(card)), "the Lens copied the card");
    }

    #[test]
    fn judgment_of_junil_ends_a_losing_war() {
        let mut g = one_landmass_game(52);
        learn(&mut g, US, "TECH_CARD_JUDGMENT_OF_JUNIL");
        g.player_found(US, 4, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.player_found(THEM, 15, 5);
        for _ in 0..6 {
            unit(&mut g, THEM, "UNIT_AXEMAN", 15, 5);
        }
        war(&mut g);
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.players[THEM].power = 10 * g.players[US].power.max(10);
        play(&mut g);
        assert!(!g.at_war(t0, t1), "the war ends");
        assert!(g.team_has_truce(t0, t1));
    }

    #[test]
    fn the_weather_engine_is_built_at_the_front_and_its_fog_falls_on_the_enemy_city() {
        let mut g = one_landmass_game(53);
        learn(&mut g, US, "TECH_CARD_WEATHER_ENGINE");
        g.player_found(US, 4, 5);
        let c = g.players[US].cities[0];
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.player_found(THEM, 12, 5);
        war(&mut g);
        // Our city sees the enemy city (a scout next to it).
        unit(&mut g, US, "UNIT_SCOUT", 11, 5);
        g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] = 200;
        play(&mut g);
        let pr = g.d.t::<ProjectType>("PROJECT_WEATHER_ENGINE");
        assert!(g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == pr.i()), "the ritual is started");
        // Completed: the army next to the enemy city gets a Fog.
        g.cities[c].order_queue.clear();
        ws::project_built(&mut g, c, pr);
        for _ in 0..3 {
            let u = unit(&mut g, US, "UNIT_AXEMAN", 11, 6);
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK);
        }
        play(&mut g);
        let their = g.players[THEM].cities[0];
        assert!(g.plot_has_weather(g.city_plot(their), AreaEffectKind::Fog));
        assert!(g.cities[c].ritual_charges.is_empty());
    }

    #[test]
    fn the_tidecall_drowns_an_enemy_shore() {
        let mut g = one_landmass_game(55);
        learn(&mut g, US, "TECH_CARD_TIDECALL_OF_DANALIN");
        g.player_found(US, 4, 3);
        let c = g.players[US].cities[0];
        unit(&mut g, US, "UNIT_WARRIOR", 4, 3);
        g.player_found(THEM, 12, 5);
        war(&mut g);
        // An enemy cottage on the shore, 2 plots from our city.
        let shore = g.map.plot_xy(6, 3).unwrap();
        g.plot_set_owner(shore, Some(THEM), false, false);
        g.plot_set_improvement_type(shore, Some(g.d.t::<ImprovementType>("IMPROVEMENT_COTTAGE")));
        unit(&mut g, US, "UNIT_SCOUT", 5, 3);
        g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] = 200;
        play(&mut g);
        let pr = g.d.t::<ProjectType>("PROJECT_TIDECALL_OF_DANALIN");
        assert!(g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == pr.i()), "the ritual is started");
        g.cities[c].order_queue.clear();
        ws::project_built(&mut g, c, pr);
        play(&mut g);
        assert!(g.plot_is_water(shore), "the shore is drowned");
    }

    #[test]
    fn the_pillar_of_dawn_is_raised_where_an_enemy_army_stands() {
        let mut g = one_landmass_game(56);
        learn(&mut g, US, "TECH_CARD_PILLAR_OF_DAWN");
        g.player_found(US, 4, 5);
        let c = g.players[US].cities[0];
        for _ in 0..4 {
            let u = unit(&mut g, US, "UNIT_AXEMAN", 4, 5);
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_CITY_DEFENSE);
        }
        g.player_found(THEM, 15, 5);
        war(&mut g);
        for _ in 0..3 {
            unit(&mut g, THEM, "UNIT_WARRIOR", 9, 5);
        }
        unit(&mut g, US, "UNIT_SCOUT", 8, 5);
        g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()] = 200;
        play(&mut g);
        let pr = g.d.t::<ProjectType>("PROJECT_PILLAR_OF_DAWN");
        assert!(g.cities[c].order_queue.iter().any(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == pr.i()), "the Pillar is started");
    }

    #[test]
    fn the_second_sky_is_worth_building_early() {
        let mut g = one_landmass_game(54);
        g.player_found(US, 4, 5);
        let c = g.players[US].cities[0];
        let sky = g.d.t::<BuildingType>("BUILDING_SECOND_SKY");
        assert!(building_value(&g, c, sky, 0) > 0);
        assert_eq!(building_value(&g, c, g.d.t::<BuildingType>("BUILDING_GRANARY"), 0), 0);
    }
}
