//! Balseraphs.
//!
//! - **Loki** (hero, enters any territory) can cast **Disrupt** in a city of another team: 0-2 of
//!   the owner's culture there is lost, and a city left with none becomes ours, without war (its
//!   owner's attitude drops by 4). The generic AI never took him into a foreign city.
//!   [`Balseraphs::unit`] sends him, at peace, to the nearest reachable city of a civ we are not at
//!   war with whose owner culture is low and grows by nothing (Disrupt drains one a turn on
//!   average, so a city producing culture never falls), and Disrupt is cast there. Otherwise he
//!   moves as a caster (`units::role_of`: he defends only): as an army unit he walked alone, and
//!   30 of 42 Lokis were killed.
//! - **Arena Battle** (the Arena, their Training Yard; FfH kept it from its AI, `bAllowAI` 0,
//!   `unflagged.rs`): a melee or recon unit or a slave in the city fights in the arena -- the city
//!   gets 3 turns of +1 happiness (`TEMP_HAPPY`); half the time the unit dies, else it gains 1-6
//!   experience (a slave becomes a Warrior). [`arena`] stages one in a city with an unhappy
//!   citizen and no games running, with a slave if one is there, else the cheapest melee or recon
//!   unit the garrison can spare (tier 1: arena fodder an expert would gamble).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;
use crate::units::{alive, hold, move_to};
use crate::view::View;
use crate::Ai;

pub struct Balseraphs;

impl CivPlay for Balseraphs {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        arena(g, v);
    }

    fn unit(&self, _ai: &Ai, g: &mut Game, v: &mut View, grp: GroupId, head: UnitId) -> bool {
        if g.d.units[g.units[head].unit_type].base.type_ != "UNIT_LOKI" {
            return false;
        }
        loki(g, v, grp, head)
    }
}

/// A rival civ's city Loki can take with Disrupt: not ours, not at war with us, its owner's
/// culture there below what 20 turns of Disrupt drain (one a turn on average) and not growing.
fn disrupt_target(g: &Game, team: TeamId, c: CityId) -> bool {
    let owner = g.cities[c].owner;
    let t = g.players[owner].team;
    t != team
        && !g.player_is_barbarian(owner)
        && !g.at_war(team, t)
        && g.city_culture(c, owner) < 20
        && g.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE) == 0
}

/// Loki: Disrupt in a target city; else to the nearest target at peace (see the module doc).
fn loki(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    let Some(spell) = g.d.ty::<SpellType>("SPELL_DISRUPT") else { return false };
    let here = g.unit_plot(u);
    if let Some(c) = g.map.plots[here].plot_city.filter(|&c| disrupt_target(g, v.team, c)) {
        if g.unit_can_cast(u, spell, false) {
            crate::trace!(g, p, "loki: disrupt {} (culture {})", g.cities[c].name, g.city_culture(c, g.cities[c].owner));
            g.tel_cast_reason(u, "loki_disrupt", 0);
            g.unit_cast(u, spell);
            // A city taken is a new city (the old id is gone).
            crate::trace!(g, p, "loki: {}", if g.cities.contains(c) { "not yet" } else { "city taken" });
        }
        if alive(g, grp) && g.group_ready_to_move(grp, true) {
            hold(g, grp);
        }
        return true;
    }
    if !v.civ_enemies.is_empty() || g.units[u].damage > 20 {
        return false;
    }
    let reach = v.flood.run(g, u, 12, false);
    let target = reach.iter().find(|r| g.map.plots[r.plot].plot_city.is_some_and(|c| g.map.plots[r.plot].revealed[v.team] && disrupt_target(g, v.team, c)));
    let Some(r) = target.copied() else { return false };
    crate::trace!(g, p, "loki at {:?}: to {:?} ({} turns)", g.map.xy(here), g.map.xy(r.plot), r.turns);
    move_to(g, grp, r.plot, 0);
    if alive(g, grp) && g.units.contains(u) && g.unit_plot(u) == r.plot && g.unit_can_cast(u, spell, false) {
        g.tel_cast_reason(u, "loki_disrupt", 0);
        g.unit_cast(u, spell);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        hold(g, grp);
    }
    true
}

/// Arena Battle in each Arena city with an unhappy citizen and no games running (see the module
/// doc): a slave first, else a tier-1 melee or recon unit the garrison can spare.
fn arena(g: &mut Game, v: &View) {
    let d = g.d;
    let p = v.p;
    let (Some(spell), Some(arena)) = (d.ty::<SpellType>("SPELL_ARENA_BATTLE"), d.ty::<BuildingType>("BUILDING_ARENA")) else { return };
    let slave = d.ty::<UnitClassType>("UNITCLASS_SLAVE");
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) || g.city_num_building(c, arena) == 0 || g.cities[c].happiness_timer > 0 {
            continue;
        }
        if g.city_happy_level(c) - g.city_unhappy_level(c, 0) >= 0 {
            continue;
        }
        let plot = g.city_plot(c);
        let ours: Vec<UnitId> = g.map.plots[plot].units.iter().copied().filter(|&u| g.units[u].owner == p && !g.units[u].has_casted).collect();
        let defenders = ours.iter().filter(|&&u| g.unit_can_defend(u, Some(plot))).count() as i32;
        let spare = defenders > crate::units::garrison_quota(g, v, c);
        let pick = ours
            .iter()
            .copied()
            .filter(|&u| g.unit_can_cast(u, spell, false))
            .filter(|&u| {
                let info = &d.units[g.units[u].unit_type];
                info.unit_class_type == slave || (info.tier <= 1 && (spare || !g.unit_can_defend(u, Some(plot))))
            })
            .min_by_key(|&u| (d.units[g.units[u].unit_type].unit_class_type != slave, d.units[g.units[u].unit_type].production_cost, u));
        let Some(u) = pick else { continue };
        crate::trace!(g, p, "{}: arena battle with {} (unhappy city)", g.cities[c].name, d.units[g.units[u].unit_type].base.type_);
        g.unit_cast(u, spell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game_with;

    /// An Arena city with an unhappy citizen sends its spare Warrior into the arena (the city
    /// gets its games: the happiness timer); a content city does not.
    #[test]
    fn an_unhappy_arena_city_holds_games() {
        let d = ffh_data::global();
        let mut g = one_landmass_game_with(9, &["CIVILIZATION_BALSERAPHS", "CIVILIZATION_MALAKIM"]);
        let us = PlayerId(0);
        g.player_found(us, 4, 5);
        let c = g.players[us].cities[0];
        g.city_set_num_real_building(c, d.t::<BuildingType>("BUILDING_ARENA"), 1);
        let warrior = d.t::<UnitType>("UNIT_WARRIOR");
        for _ in 0..4 {
            g.player_init_unit(us, warrior, 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let v = View::new(&g, us);
        arena(&mut g, &v);
        assert_eq!(g.cities[c].happiness_timer, 0, "a content city holds no games");
        g.city_set_population(c, 14);
        assert!(g.city_happy_level(c) < g.city_unhappy_level(c, 0), "the city is unhappy");
        let v = View::new(&g, us);
        arena(&mut g, &v);
        assert!(g.cities[c].happiness_timer > 0, "the arena's games");
    }
}
