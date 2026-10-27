//! The caster corps (`CASTER_ROLES`; More Naval AI's adept roles, `CvUnitAI::AI_update` and
//! `CvCityAI::AI_chooseProduction`), mana-node builders for casters without a role
//! (`MANA_NODES`, `mana.rs`), and Govannon's Teach Spellcasting (`TEACH_SPELLS`).
//!
//! **Roles.** Each caster of ours (an adept or better, not a hero: `mana::is_caster`) has a role,
//! kept in its unit AI type, so it survives saves and needs no state of its own:
//! - `UNITAI_MANA_UPGRADE`: the node builder, one while our raw mana has nodes worth building
//!   (`mana::build_node`);
//! - `UNITAI_TERRAFORMER`: one, while a caster knows a terraforming spell (Spring, Scorch,
//!   Vitalize) and our worked land has plots it would improve (`magic::terraform_target`);
//! - `UNITAI_MAGE`: city mages, one for every four cities, each in a city of its own (the most
//!   threatened first, then the capital, then the largest), where its city spells, blessings on the
//!   garrison and damage on attackers are cast at the end of the turn;
//! - `UNITAI_WARWIZARD`: the rest, with the stacks of our operation (`magic::caster_target`).
//! A caster keeps a role while it has work; a vacant builder or terraformer role takes the first
//! caster that can do it. More Naval AI keeps a mage in every city; an expert's corps is smaller
//! and goes where the spells pay: the economy first (nodes, terraforming, city spells), the army
//! when there is a war to fight.
//!
//! **Training** ([`production`]): while our mana gives a fresh caster a spell worth having
//! (`magic::sphere_value`), the corps is a city mage for every four cities, and at war or
//! preparing one a war wizard and one more for every ten attackers, five at most; the Mage Guild
//! that trains adepts in a productive city when no city of ours can ([`caster_building`]). Three
//! earlier attempts at training casters for their fight spells lost wins (docs/AI_LOG.md,
//! "Spellcasting", "Casters through a Mage Guild"): they came late, cast buffs and paid only in
//! fights this AI rarely fought. This corps pays its way in the economy first.

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::units::{alive, move_to, skip};
use crate::view::View;

/// The building a city needs to train our civilization's casters (the Mage Guild for adepts),
/// when no city of ours has it or builds it: in a productive city of ours (half our best city's
/// production or more) that can build it.
pub fn caster_building(g: &Game, c: CityId) -> Option<BuildingType> {
    let d = g.d;
    let p = g.cities[c].owner;
    let civ = g.player_get_civilization_type(p)?;
    let pl = &g.players[p];
    let mut needed: Option<BuildingType> = None;
    for cl in d.unit_classes.keys() {
        let Some(t) = d.civ_unit(civ, cl) else { continue };
        let info = &d.units[t];
        if !crate::mana::can_build_nodes(d, t) || info.production_cost <= 0 || info.min_level > 0 || info.default_unit_ai_type != Some(UnitAIType::UNITAI_MAGE) {
            continue;
        }
        if !g.player_can_train(p, t, false, false, false) {
            continue;
        }
        let Some(b) = crate::production::unit_building(d, civ, info) else {
            // A caster no building gates (the Clan's and the Doviello's Shamans): nothing to build.
            return None;
        };
        needed = Some(b);
        break;
    }
    let b = needed?;
    let bc = d.buildings[b].building_class_type?;
    if pl.cities.iter().any(|&o| g.city_is_has_building_class(o, bc) || g.city_production_building(o) == Some(b)) {
        return None;
    }
    // A productive city: at least half our best city's production (the best one may be busy).
    let best = pl.cities.iter().map(|&o| g.city_yield_rate(o, YieldType::YIELD_PRODUCTION)).max()?;
    (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) * 2 >= best && g.city_can_construct(c, b, false, false, false)).then_some(b)
}

/// City mages wanted: one for every four cities.
fn mage_quota(g: &Game, p: PlayerId) -> i32 {
    g.players[p].cities.len() as i32 / 4
}

/// The corps' size at most (city mages and war wizards; the node builder and the terraformer are
/// casters with work of their own).
const MAX_CORPS: i32 = 5;

fn count_role(g: &Game, p: PlayerId, role: UnitAIType, except: UnitId) -> i32 {
    g.players[p].units.iter().filter(|&&o| o != except && g.units[o].ai_type == Some(role) && crate::mana::is_caster(g, o)).count() as i32
}

/// Is there node work for a builder (`MANA_NODES` on, a node worth a trip)?
fn node_work(g: &Game, p: PlayerId) -> bool {
    crate::switches::MANA_NODES.on(g, p) && crate::mana::jobs(g, p).iter().any(|&(_, _, v)| v >= crate::mana::MIN_NODE_VALUE)
}

/// The role caster `u` takes this turn (see the module doc).
fn role_for(g: &Game, p: PlayerId, u: UnitId) -> UnitAIType {
    const NODES: UnitAIType = UnitAIType::UNITAI_MANA_UPGRADE;
    const TERRAFORM: UnitAIType = UnitAIType::UNITAI_TERRAFORMER;
    let cur = g.units[u].ai_type;
    let terraform = || !crate::magic::terraform_work(g, p, u).is_empty();
    if cur == Some(NODES) && node_work(g, p) {
        return NODES;
    }
    if cur == Some(TERRAFORM) && terraform() {
        return TERRAFORM;
    }
    if count_role(g, p, NODES, u) == 0 && node_work(g, p) {
        return NODES;
    }
    if count_role(g, p, TERRAFORM, u) == 0 && terraform() {
        return TERRAFORM;
    }
    if count_role(g, p, UnitAIType::UNITAI_MAGE, u) < mage_quota(g, p) {
        return UnitAIType::UNITAI_MAGE;
    }
    UnitAIType::UNITAI_WARWIZARD
}

/// Orders for caster `u` by its role (`CASTER_ROLES`), or, without roles, a mana node when it has
/// nothing better to do (`MANA_NODES`). False: the generic caster logic decides (`units::caster`).
pub fn play(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    if !crate::mana::is_caster(g, u) {
        return false;
    }
    if !crate::switches::CASTER_ROLES.on(g, p) {
        // A caster with no strike to join and no city spell to keep builds nodes.
        if !crate::switches::MANA_NODES.on(g, p) || crate::magic::keeps_building(g, u) {
            return false;
        }
        if crate::magic::enabled(g, p) {
            crate::campaign::plan(g, v);
            if crate::magic::strike_target(g, v, u).is_some() {
                return false;
            }
        }
        return crate::mana::build_node(g, v, grp, u);
    }
    let role = role_for(g, p, u);
    if g.units[u].ai_type != Some(role) {
        crate::trace!(g, p, "caster {} {:?}: role {:?} -> {}", g.d.units[g.units[u].unit_type].base.type_, u, g.units[u].ai_type.map(|a| UnitAIType::NAMES[a.idx()]), UnitAIType::NAMES[role.idx()]);
        g.tel_decision(p, "caster_role", UnitAIType::NAMES.get(role.idx()).copied().unwrap_or("?"), "role", 0, String::new);
        g.unit_ai_set_unit_ai_type(u, role);
    }
    match role {
        UnitAIType::UNITAI_MANA_UPGRADE => crate::mana::build_node(g, v, grp, u),
        UnitAIType::UNITAI_TERRAFORMER => {
            let Some(q) = crate::magic::terraform_target(g, v, u) else { return false };
            go(g, grp, u, q, "terraform");
            true
        }
        UnitAIType::UNITAI_MAGE => city_mage(g, v, grp, u),
        _ => false,
    }
}

fn go(g: &mut Game, grp: GroupId, u: UnitId, q: PlotIdx, why: &str) {
    let p = g.units[u].owner;
    crate::trace!(g, p, "caster {} at {:?} -> {:?} ({})", g.d.units[g.units[u].unit_type].base.type_, g.map.xy(g.unit_plot(u)), g.map.xy(q), why);
    if q != g.unit_plot(u) {
        move_to(g, grp, q, MOVE_SAFE_TERRITORY);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
}

/// A city mage guards a city of its own: where it stands if no other city mage of ours is there,
/// else the city without one that is most threatened, then the capital, then the largest, for the
/// turns it takes (within 10).
fn city_mage(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    let here = g.unit_plot(u);
    let other_mage = |q: PlotIdx| g.map.plots[q].units.iter().any(|&o| o != u && g.units[o].owner == p && g.units[o].ai_type == Some(UnitAIType::UNITAI_MAGE) && crate::mana::is_caster(g, o));
    if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) && !other_mage(here) && !v.is_claimed(here) {
        v.claimed.push(here);
        go(g, grp, u, here, "city mage");
        return true;
    }
    let reach = v.flood.run(g, u, 10, false);
    let mut best: Option<(i64, PlotIdx)> = None;
    for &c in &g.players[p].cities {
        let cp = g.city_plot(c);
        if other_mage(cp) || v.is_claimed(cp) {
            continue;
        }
        let Some(turns) = reach.iter().find(|r| r.plot == cp).map(|r| r.turns) else { continue };
        let threat = v.danger_at(cp).min(5000) as i64 / 100 + v.peril.iter().filter(|&&(q, _)| q == cp).count() as i64 * 20;
        let score = (10 + threat) * (g.cities[c].population as i64 + 5) * if g.city_is_capital(c) { 2 } else { 1 } * 10 / (turns as i64 + 2);
        if best.map_or(true, |(b, bq)| score > b || score == b && cp < bq) {
            best = Some((score, cp));
        }
    }
    let Some((_, q)) = best else { return false };
    v.claimed.push(q);
    go(g, grp, u, q, "city mage");
    true
}

/// A fresh caster is worth training for our mana: a sphere we have opens a spell worth at least
/// this (`magic::sphere_value`; Haste, a stack blessing that lasts one turn, is about 50).
const MIN_SPHERE_VALUE: i32 = 60;

/// The best first-level sphere a fresh caster of ours could take with the mana we have.
pub fn best_sphere(g: &Game, p: PlayerId) -> i32 {
    let d = g.d;
    d.promotions
        .keys()
        .filter(|&pr| d.promotions[pr].bonus_prereq.is_some_and(|b| crate::mana::is_mana(d, b) && g.player_has_bonus(p, b)))
        .map(|pr| crate::magic::sphere_value(g, p, pr))
        .max()
        .unwrap_or(0)
}

/// Production (`CASTER_ROLES`): a caster for the corps while it is short (see the module doc), in a
/// city that can train one, or the Mage Guild where none can. `at_war`: at war or preparing one;
/// `attackers`: our field army.
pub fn production(g: &Game, c: CityId, at_war: bool, attackers: i32) -> Option<crate::production::Choice> {
    use crate::production::Choice;
    let p = g.cities[c].owner;
    if !crate::switches::CASTER_ROLES.on(g, p) || g.players[p].cities.len() < 3 {
        return None;
    }
    let quota = mage_quota(g, p);
    let war = if at_war { 1 + attackers / 10 } else { 0 };
    let pl = &g.players[p];
    let training: i32 = [UnitAIType::UNITAI_MAGE, UnitAIType::UNITAI_WARWIZARD, UnitAIType::UNITAI_MANA_UPGRADE, UnitAIType::UNITAI_TERRAFORMER]
        .iter()
        .map(|&a| pl.ai_num_train_ai_units[a])
        .sum();
    let have = pl.units.iter().filter(|&&u| crate::mana::is_caster(g, u)).count() as i32 + training;
    if have >= (quota + war).min(MAX_CORPS) || best_sphere(g, p) < MIN_SPHERE_VALUE {
        return None;
    }
    if let Some(u) = crate::production::best_unit(g, c, crate::production::Role::Mage).filter(|&u| crate::mana::can_build_nodes(g.d, u)) {
        let mages = pl.units.iter().filter(|&&o| g.units[o].ai_type == Some(UnitAIType::UNITAI_MAGE) && crate::mana::is_caster(g, o)).count() as i32
            + pl.ai_num_train_ai_units[UnitAIType::UNITAI_MAGE];
        let role = if mages < quota { UnitAIType::UNITAI_MAGE } else { UnitAIType::UNITAI_WARWIZARD };
        crate::trace!(g, p, "{}: a caster for the corps ({}; {} of {} + {})", g.cities[c].name, UnitAIType::NAMES[role.idx()], have, quota, war);
        return Some(Choice::Train(u, role));
    }
    caster_building(g, c).map(Choice::Construct)
}

// ----------------------------------------------------------------------------
// Govannon
// ----------------------------------------------------------------------------

/// Teach Spellcasting is worth a walk from this much (`magic::teach_value`, `promotion_value`'s
/// scale: Combat I is about 40).
const MIN_TEACH: i32 = 40;

/// Govannon (`TEACH_SPELLS`; More Naval AI's `AI_Govannonmove`): Teach Spellcasting gives every
/// unit of ours on his plot his first-level spheres and Channeling I, so he casts it where it
/// teaches most -- here if that is worth it, else at the stack or city of ours worth most for the
/// turns it takes him (within 10, on our land, no enemy in sight there). FfH's own value for the
/// spell is 0 (all its effect is Python), so no AI ever cast it. False when there is nothing worth
/// teaching: he fights as the hero he is.
pub fn govannon(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    static TEACH: std::sync::OnceLock<Option<(SpellType, UnitClassType)>> = std::sync::OnceLock::new();
    let d = g.d;
    let p = v.p;
    let Some((teach, class)) = *TEACH.get_or_init(|| d.ty::<SpellType>("SPELL_TEACH_SPELLCASTING").and_then(|s| d.spells[s].unit_class_prereq.map(|cl| (s, cl)))) else {
        return false;
    };
    if class != g.unit_get_unit_class_type(u) || !crate::switches::TEACH_SPELLS.on(g, p) {
        return false;
    }
    let here = g.unit_plot(u);
    let value_here = crate::magic::teach_value(g, u, here);
    if value_here >= MIN_TEACH && g.unit_can_cast(u, teach, false) {
        crate::magic::cast(g, u, teach, "teach", value_here);
        if alive(g, grp) && g.group_ready_to_move(grp, false) {
            skip(g, grp);
        }
        return true;
    }
    let reach = v.flood.run(g, u, 10, false);
    let mut best: Option<(i32, PlotIdx, i32)> = None;
    for &o in &g.players[p].groups {
        let Some(h) = g.group_get_head_unit(o) else { continue };
        let q = g.unit_plot(h);
        if q == here || g.map.plots[q].owner != Some(p) || g.plot_is_visible_enemy_unit(q, p) {
            continue;
        }
        let Some(turns) = reach.iter().find(|r| r.plot == q).map(|r| r.turns) else { continue };
        let value = crate::magic::teach_value(g, u, q);
        if value < MIN_TEACH {
            continue;
        }
        let score = value * 10 / (turns + 2);
        if best.map_or(true, |(b, bq, _)| score > b || score == b && q < bq) {
            best = Some((score, q, value));
        }
    }
    let Some((_, q, value)) = best else { return false };
    // Alone: the stack he stands with does not walk with him.
    let grp = if g.groups[grp].units.len() > 1 {
        g.unit_join_group(u, None, true);
        match g.units[u].group {
            Some(n) => n,
            None => return true,
        }
    } else {
        grp
    };
    crate::trace!(g, p, "Govannon {:?} -> {:?} to teach (value {})", g.map.xy(here), g.map.xy(q), value);
    move_to(g, grp, q, MOVE_SAFE_TERRITORY);
    if g.units.contains(u) && g.unit_plot(u) == q && g.unit_can_cast(u, teach, false) {
        let value = crate::magic::teach_value(g, u, q);
        if value >= MIN_TEACH {
            crate::magic::cast(g, u, teach, "teach", value);
        }
    }
    if alive(g, grp) && g.group_ready_to_move(grp, false) {
        skip(g, grp);
    }
    true
}
