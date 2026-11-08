//! Batch B4's cards (docs/CARDS.md, section 5): when the AI casts the world spells that give birth
//! to a civilization (the Patrian Awakening, the Dawn Knights, the Fey Court, the Condottieri),
//! Mirror Colony, the Free City Charter and the Mirror Gate. A new civilization is played by the
//! normal AI (its controller is its founder's, as the Infernals'); its own units and rules need
//! nothing here.
//!
//! - **The births and Mirror Colony** are cast as soon as they can be: each gives a vassal or two
//!   cities on land that is free now, and the land and the player slots go to whoever is first.
//! - **The Free City Charter** releases the city an expert would: a city (not the capital) that an
//!   enemy army is about to take, the enemy's land units within 2 plots at least
//!   [`FREE_CITY_ODDS`] times its defenders. The Free City is at peace with that enemy, and keeps
//!   the garrison behind a Wall of Stone; the empire must keep 3 cities.
//! - **The Mirror Gate**: a gate city with fewer than [`GATE_GARRISON`] defenders gets one from a
//!   gate city that keeps more.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::view::View;

/// The world spells cast as soon as they can be.
const AT_ONCE: [&str; 5] = ["SPELL_PATRIAN_AWAKENING", "SPELL_DAWN_KNIGHTS", "SPELL_FEY_COURT", "SPELL_CONDOTTIERI", "SPELL_MIRROR_COLONY"];

/// The enemy units near a city, per defender in it, at which the AI releases it as a Free City.
pub const FREE_CITY_ODDS: i32 = 2;

/// The defenders a city with a Mirror Gate should hold.
pub const GATE_GARRISON: usize = 2;

/// The batch's turn (`cards::TURNS`).
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian {
        return;
    }
    let p = v.p;
    for name in AT_ONCE {
        if let Some(spell) = g.d.ty::<SpellType>(name) {
            if let Some(u) = caster(g, p, spell) {
                if g.unit_can_cast(u, spell, false) {
                    crate::magic::cast(g, u, spell, "card", 0);
                }
            }
        }
    }
    free_city(g, p);
    mirror_gate(g, p);
}

/// A unit of `p`'s that may cast its world spell `spell` (once a game, any unit): the first on the
/// map that has not cast this turn, while `p`'s team knows the spell's card and has not cast it.
fn caster(g: &Game, p: PlayerId, spell: SpellType) -> Option<UnitId> {
    let info = &g.d.spells[spell];
    let team = g.players[p].team;
    if info.tech_prereq.is_some_and(|t| !g.team_is_has_tech(team, Some(t))) || g.players[p].once_spells.contains(&spell) {
        return None;
    }
    g.players[p].units.iter().copied().find(|&u| !g.units[u].has_casted && g.unit_plot_opt(u).is_some())
}

/// Our land combat units on `plot`.
fn defenders(g: &Game, p: PlayerId, plot: PlotIdx) -> Vec<UnitId> {
    g.map.plots[plot]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units[u].owner == p && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && g.d.units[g.units[u].unit_type].combat > 0)
        .collect()
}

/// The land combat units of teams at war with `p`'s within 2 plots of `plot`.
fn enemies_near(g: &Game, p: PlayerId, plot: PlotIdx) -> i32 {
    let team = g.players[p].team;
    let (x, y) = g.map.xy(plot);
    let mut n = 0;
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            for &u in &g.map.plots[q].units {
                let o = g.units[u].owner;
                if g.at_war(team, g.players[o].team) && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && g.d.units[g.units[u].unit_type].combat > 0 {
                    n += 1;
                }
            }
        }
    }
    n
}

/// The Free City Charter: release the city that is about to fall (the module comment).
fn free_city(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_FREE_CITY_CHARTER") else { return };
    let Some(u) = caster(g, p, spell) else { return };
    if g.players[p].cities.len() < 3 {
        return;
    }
    let mut best: Option<(i32, CityId)> = None;
    for target in g.unit_spell_targets(u, spell) {
        let SpellTarget::City(c) = target else { continue };
        let plot = g.city_plot(c);
        let enemies = enemies_near(g, p, plot);
        let held = defenders(g, p, plot).len() as i32;
        if enemies >= 3 && enemies >= FREE_CITY_ODDS * held.max(1) && best.map_or(true, |(e, bc)| (enemies, std::cmp::Reverse(c)) > (e, std::cmp::Reverse(bc))) {
            best = Some((enemies, c));
        }
    }
    let Some((enemies, c)) = best else { return };
    let target = SpellTarget::City(c);
    if g.unit_can_cast_at(u, spell, target) {
        crate::trace!(g, p, "releases {} as a Free City ({} enemy units near)", g.cities[c].name, enemies);
        g.tel_cast_reason(u, "card", enemies as i64);
        g.unit_cast_at(u, spell, target);
    }
}

/// The Mirror Gate: a gate city short of defenders gets one from a gate city with more.
fn mirror_gate(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let (Some(spell), Some(gate)) = (d.ty::<SpellType>("SPELL_MIRROR_GATE"), d.ty::<BuildingType>("BUILDING_MIRROR_GATE")) else { return };
    if g.players[p].once_spells.is_empty() {
        return;
    }
    let gates: Vec<CityId> = g.players[p].cities.iter().copied().filter(|&c| g.city_num_building(c, gate) > 0).collect();
    if gates.len() < 2 {
        return;
    }
    for &short in &gates {
        if defenders(g, p, g.city_plot(short)).len() >= GATE_GARRISON {
            continue;
        }
        for &from in &gates {
            let held = defenders(g, p, g.city_plot(from));
            if from == short || held.len() <= GATE_GARRISON {
                continue;
            }
            let target = SpellTarget::City(short);
            let mover = held.iter().copied().filter(|&u| g.unit_can_cast_at(u, spell, target)).max_by_key(|&u| (d.units[g.units[u].unit_type].combat, std::cmp::Reverse(u)));
            if let Some(u) = mover {
                crate::trace!(g, p, "sends a defender through the Mirror Gate to {}", g.cities[short].name);
                g.tel_cast_reason(u, "card", 0);
                g.unit_cast_at(u, spell, target);
                break;
            }
        }
    }
}
