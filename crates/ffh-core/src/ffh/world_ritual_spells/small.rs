//! Four world rituals that replace FFH2's world spells (docs/CARDS.md, "World rituals";
//! docs/PORTING.md, "Changes from FFH2"):
//!
//! | Civilization | Ritual | Spell | Effect |
//! |---|---|---|---|
//! | Calabim | The Tithe | Name the Tithe (`world_action`, a player) | at completion: the named rival's cities above 2 lose 2 population; our vampires gain experience |
//! | Doviello | The Cull | The Cull (a plot) | the weaker half of our units there die; the survivors share their experience |
//! | Luchuirp | Upgrade | (none) | at completion: each of our golems gains a stack of Reinforced (+10% HP in battle) |
//! | Amurites | Wellspring | Wellspring (placed by the player: a plot and a mana) | a bare plot of ours gains a mana resource and its node |

use super::Hooks;
use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// This module's hooks.
pub const HOOKS: Hooks = Hooks { spell: find_spell, req: find_req, targets: find_targets, label: find_target_label, cannot_create, project_built, check };

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellNameTheTithe" => spell_name_the_tithe,
        "spellTheCull" => spell_the_cull,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqNameTheTithe" => req_name_the_tithe,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsNameTheTithe" => targets_name_the_tithe,
        "targetsTheCull" => targets_the_cull,
        "targetsWellspring" => targets_wellspring,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelWellspring" => label_wellspring,
        "labelNameTheTithe" => label_name_the_tithe,
        _ => return None,
    };
    Some(f)
}

/// The rituals of this module that a city cannot create: The Tithe without a living named rival
/// (outside a visibility test).
fn cannot_create(g: &Game, c: CityId, project: ProjectType, test_visible: bool) -> bool {
    if test_visible || tithe_project(g.d) != Some(project) {
        return false;
    }
    let p = g.cities[c].owner;
    !g.players[p].tithe_target.is_some_and(|r| g.players[r].alive)
}

/// At the completion of a ritual of this module.
fn project_built(g: &mut Game, c: CityId, project: ProjectType) {
    let d = g.d;
    if tithe_project(d) == Some(project) {
        tithe_completed(g, c);
    } else if d.ty::<ProjectType>("PROJECT_RITUAL_UPGRADE") == Some(project) {
        upgrade_completed(g, c);
    }
}

/// Once a turn for player `p` (`Some`), or for every player after a war, a peace or a death: a
/// Tithe whose rival is dead or no longer at war is cancelled.
fn check(g: &mut Game, p: Option<PlayerId>) {
    let players: Vec<PlayerId> = match p {
        Some(p) => vec![p],
        None => g.player_ids().filter(|&o| g.players[o].tithe_target.is_some()).collect(),
    };
    for p in players {
        tithe_check(g, p);
    }
}

// ---------------------------------------------------------------------------------------------
// Calabim: The Tithe. Name the Tithe (an action of the empire, any unit) names a rival the
// caster's team has met and is at war with or may declare war on, declaring the war; the ritual
// is built only while a living rival is named. Peace with the rival or its death cancel the
// ritual (its order leaves the queue, its production is lost: `city_check_rituals`) and
// the name. At completion each city of the rival above 2 population loses 2, and each of the
// owner's units with the Vampire promotion gains max(1, D / 2) experience (D the population
// lost).

/// The Calabim's ritual.
pub fn tithe_project(d: &GameData) -> Option<ProjectType> {
    d.ty::<ProjectType>("PROJECT_RITUAL_THE_TITHE")
}

/// May player `p` name `rival` with Name the Tithe: a living civilization of another team that
/// `p`'s team has met, at war with it or open to its declaration?
pub fn tithe_target_ok(g: &Game, p: PlayerId, rival: PlayerId) -> bool {
    if rival == p || !g.players[rival].alive || g.player_is_barbarian(rival) {
        return false;
    }
    let (t, o) = (g.players[p].team, g.players[rival].team);
    t != o && g.teams[t].has_met[o] && (g.teams[t].at_war[o] || g.team_can_declare_war(t, o))
}

/// Does a city of `p` have production in The Tithe?
pub fn tithe_in_progress(g: &Game, p: PlayerId) -> bool {
    let Some(pr) = tithe_project(g.d) else { return false };
    g.players[p].cities.iter().any(|&c| g.cities.contains(c) && g.city_world_ritual_in_progress(c) == Some(pr))
}

/// `reqNameTheTithe(caster)`: not while the ritual has production.
fn req_name_the_tithe(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    !tithe_in_progress(g, g.units[caster].owner)
}

/// `targetsNameTheTithe(caster)`: the rivals it may name, in player order.
fn targets_name_the_tithe(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let p = g.units[caster].owner;
    g.player_ids().filter(|&r| tithe_target_ok(g, p, r)).map(SpellTarget::Player).collect()
}

/// `labelNameTheTithe(caster, target)`: the rival's civilization and whether the caster's team is
/// at war with it (`Sheaim, at war`, `Sheaim, at peace`), after its leader's name.
fn label_name_the_tithe(g: &Game, caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    let SpellTarget::Player(rival) = target else { return String::new() };
    let (t, o) = (g.unit_get_team(caster), g.players[rival].team);
    let state = if g.teams[t].at_war[o] { "at war" } else { "at peace" };
    format!("{}, {state}", g.player_get_civilization_short_description(rival))
}

/// `spellNameTheTithe(caster)`: declares war on the rival if not at war, and names it.
fn spell_name_the_tithe(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Player(rival)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    if !tithe_target_ok(g, p, rival) || tithe_in_progress(g, p) {
        return;
    }
    let (t, o) = (g.players[p].team, g.players[rival].team);
    if !g.teams[t].at_war[o] {
        g.team_declare_war(t, o, true, None);
    }
    if !g.teams[t].at_war[o] {
        return;
    }
    g.players[p].tithe_target = Some(rival);
    let name = g.player_get_civilization_short_description(rival);
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Name the Tithe: {name}."));
}

/// Player `p`'s Tithe: a named rival that is dead or no longer at war with `p` is unnamed, and
/// the ritual leaves every queue of `p`'s (its production lost).
fn tithe_check(g: &mut Game, p: PlayerId) {
    let Some(rival) = g.players[p].tithe_target else { return };
    let (t, o) = (g.players[p].team, g.players[rival].team);
    if g.players[rival].alive && g.teams[t].at_war[o] {
        return;
    }
    g.players[p].tithe_target = None;
    if !g.players[p].alive {
        return;
    }
    let Some(pr) = tithe_project(g.d) else { return };
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) {
            continue;
        }
        while let Some(i) = g.cities[c].order_queue.iter().position(|q| q.order_type == OrderType::ORDER_CREATE && q.data1 == pr.i()) {
            g.city_pop_order(c, i as i32, false, true);
        }
    }
}

/// The Tithe completed in city `c`: the rival pays, the vampires feed, the name is spent.
fn tithe_completed(g: &mut Game, c: CityId) {
    let d = g.d;
    let p = g.cities[c].owner;
    let Some(rival) = g.players[p].tithe_target.take() else { return };
    let (t, o) = (g.players[p].team, g.players[rival].team);
    if !g.players[rival].alive || !g.teams[t].at_war[o] {
        return;
    }
    let mut lost = 0;
    for rc in g.players[rival].cities.clone() {
        if g.cities.contains(rc) && g.cities[rc].population > 2 {
            g.city_change_population(rc, -2);
            lost += 2;
        }
    }
    let gain = (lost / 2).max(1);
    let vampire = d.t::<PromotionType>("PROMOTION_VAMPIRE");
    for u in g.players[p].units.clone() {
        if g.units.contains(u) && g.units[u].has_promotion[vampire] {
            g.unit_change_experience(u, gain, -1, false, false, false);
        }
    }
    let name = g.player_get_civilization_short_description(rival);
    let turn = g.game_turn();
    for who in [p, rival] {
        g.log.msg(turn, Some(who), || format!("The Tithe: {name} loses {lost} population."));
    }
}

// ---------------------------------------------------------------------------------------------
// Doviello: The Cull. At a plot holding 2 or more land units of the caster's owner with
// strength, the weaker half (by base strength, then experience, then id; rounded down) dies, and
// the survivors share the dead's experience plus 5 a death, the remainder to the strongest first.

/// Experience a unit killed by The Cull adds to the survivors' share, beyond its own.
pub const CULL_XP_PER_KILL: i32 = 5;

/// The units of player `p` on `plot` that The Cull counts, weakest first: alive (no death under
/// way), land, with base strength above 0, not carried.
pub fn cull_units(g: &Game, p: PlayerId, plot: PlotIdx) -> Vec<UnitId> {
    let mut out: Vec<UnitId> = g.map.plots[plot]
        .units
        .iter()
        .copied()
        .filter(|&u| {
            g.units.contains(u)
                && g.units[u].owner == p
                && !g.unit_is_dead(u)
                && !g.units[u].death_delay
                && !g.units[u].battle_summon
                && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
                && g.unit_base_combat_str(u) > 0
                && !g.unit_is_cargo(u)
        })
        .collect();
    out.sort_by_key(|&u| (g.unit_base_combat_str(u), g.units[u].experience, u));
    out
}

/// May player `p` cast The Cull at `plot` (2 or more of its units there count)?
pub fn cull_ok(g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
    cull_units(g, p, plot).len() >= 2
}

/// `targetsTheCull(caster)`: the plots of the owner's units where it may be cast, in plot order.
fn targets_the_cull(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let p = g.units[caster].owner;
    let mut plots: Vec<PlotIdx> = g.players[p].units.iter().filter(|&&u| g.units.contains(u)).filter_map(|&u| g.unit_plot_opt(u)).collect();
    plots.sort();
    plots.dedup();
    plots.into_iter().filter(|&q| cull_ok(g, p, q)).map(SpellTarget::Plot).collect()
}

/// `spellTheCull(caster)`.
fn spell_the_cull(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(plot)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    let units = cull_units(g, p, plot);
    if units.len() < 2 {
        return;
    }
    let (killed, survivors) = units.split_at(units.len() / 2);
    let pool: i32 = killed.iter().map(|&u| g.units[u].experience).sum::<i32>() + CULL_XP_PER_KILL * killed.len() as i32;
    // (A delayed death: the caster may be among them, and its cast goes on.)
    for &u in killed {
        if g.units.contains(u) {
            g.unit_kill(u, true, None);
        }
    }
    let n = survivors.len() as i32;
    let (share, rest) = (pool / n, pool % n);
    // The strongest first: the end of the weakest-first order.
    for (i, &u) in survivors.iter().rev().enumerate() {
        let gain = share + i32::from((i as i32) < rest);
        if gain > 0 && g.units.contains(u) {
            g.unit_change_experience(u, gain, -1, false, false, false);
        }
    }
    let k = killed.len();
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("The Cull: {k} units killed, {pool} experience to {n} units."));
}

// ---------------------------------------------------------------------------------------------
// Luchuirp: Upgrade. At completion each golem of the owner (the Golem promotion) gains a stack
// of Reinforced (`Unit::reinforced`): +10% maximum HP in a battle (`unit_engage_max_hp`).

/// Upgrade completed in city `c`.
fn upgrade_completed(g: &mut Game, c: CityId) {
    let p = g.cities[c].owner;
    let golem = g.d.t::<PromotionType>("PROMOTION_GOLEM");
    let mut n = 0;
    for u in g.players[p].units.clone() {
        if g.units.contains(u) && g.units[u].has_promotion[golem] {
            g.units[u].reinforced += 1;
            n += 1;
        }
    }
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Upgrade: {n} golems reinforced."));
}

/// At a conversion (`unit_convert`, `from` into `u`): Reinforced passes to `u` only if it is a
/// golem.
pub fn convert_reinforced(g: &mut Game, u: UnitId, from: UnitId) {
    let golem = g.d.t::<PromotionType>("PROMOTION_GOLEM");
    g.units[u].reinforced = if g.units[u].has_promotion[golem] { g.units[from].reinforced } else { 0 };
}

// ---------------------------------------------------------------------------------------------
// Amurites: Wellspring. Placed by the player, no unit casting it (`place_on_charge`,
// [`Game::player_place`]): a land plot of the player with no resource, city or improvement, not a
// peak, gains a typed mana resource and that mana's node (the improvement whose `bonus_convert`
// is the resource: `cards::economy::mana_nodes`). The target is a choice: the plot and the mana,
// `wellspring_choice`.

/// The typed mana resources and their nodes, in the data's build order.
pub fn wellspring_manas(d: &GameData) -> Vec<(BonusType, ImprovementType)> {
    crate::ffh::cards::economy::mana_nodes(d)
}

/// May player `p` raise a Wellspring on `plot`?
pub fn wellspring_plot_ok(g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
    let pl = &g.map.plots[plot];
    pl.owner == Some(p) && !g.plot_is_water(plot) && !g.plot_is_peak(plot) && pl.bonus_type.is_none() && pl.plot_city.is_none() && pl.improvement_type.is_none()
}

/// The choice of a Wellspring on `plot` of mana `bonus`.
pub fn wellspring_choice(d: &GameData, plot: PlotIdx, bonus: BonusType) -> i32 {
    plot.idx() as i32 * d.bonuses.len() as i32 + bonus.idx() as i32
}

/// The plot and the mana of a Wellspring choice.
pub fn wellspring_decode(g: &Game, choice: i32) -> Option<(PlotIdx, BonusType)> {
    let n = g.d.bonuses.len() as i32;
    if choice < 0 || (choice / n) as usize >= g.map.plots.len() {
        return None;
    }
    Some((PlotIdx::from_usize((choice / n) as usize), BonusType::from_usize((choice % n) as usize)))
}

/// The Wellspring choices of player `p`: by plot, then by mana (its charge aside).
pub fn wellspring_targets(g: &Game, p: PlayerId) -> Vec<SpellTarget> {
    let d = g.d;
    let manas = wellspring_manas(d);
    let mut out = Vec::new();
    for q in g.map.plot_ids() {
        if wellspring_plot_ok(g, p, q) {
            out.extend(manas.iter().map(|&(b, _)| SpellTarget::Choice(wellspring_choice(d, q, b))));
        }
    }
    out
}

/// `targetsWellspring(caster)`: [`wellspring_targets`] of the caster's owner.
fn targets_wellspring(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    wellspring_targets(g, g.units[caster].owner)
}

/// A Wellspring choice for people: its mana, `Fire Mana` (the plot is picked on the map first:
/// [`Game::spell_choice_plot`]).
pub fn wellspring_label(g: &Game, choice: i32) -> String {
    let d = g.d;
    let Some((_, bonus)) = wellspring_decode(g, choice) else { return String::new() };
    d.text.name(&d.bonuses[bonus].base.description).to_string()
}

fn label_wellspring(g: &Game, _caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    match target {
        SpellTarget::Choice(choice) => wellspring_label(g, choice),
        _ => String::new(),
    }
}

/// Player `p` places a Wellspring (`choice`): the plot gains the mana and its node. Returns
/// whether it did (a valid plot of `p` and a typed mana); the charge is the caller's
/// ([`Game::player_place`]).
pub fn wellspring_place(g: &mut Game, p: PlayerId, choice: i32) -> bool {
    let Some((plot, bonus)) = wellspring_decode(g, choice) else { return false };
    let Some(&(_, node)) = wellspring_manas(g.d).iter().find(|&&(b, _)| b == bonus) else { return false };
    if !wellspring_plot_ok(g, p, plot) {
        return false;
    }
    let (x, y) = g.map.xy(plot);
    let label = format!("{} ({x}, {y})", wellspring_label(g, choice));
    g.plot_set_bonus_type(plot, Some(bonus));
    g.plot_set_improvement_type(plot, Some(node));
    let turn = g.game_turn();
    g.log.msg_at(turn, Some(p), (x, y), || format!("Wellspring: {label}."));
    true
}

#[cfg(test)]
mod tests;
