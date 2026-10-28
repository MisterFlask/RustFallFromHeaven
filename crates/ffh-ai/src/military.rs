//! Field armies: stacks that hunt barbarians and animals near our lands, clear lairs (an AI
//! unit entering a lair destroys it, FfH `CvUnit::setXY`), and take barbarian and enemy cities
//! in coordinated operations (`campaign.rs`).
//!
//! A stack is a selection group led by an army unit; army units meeting on a plot merge (up
//! to [`MAX_STACK`]). Attacks go through the rules' group attack (`MISSION_MOVE_TO` with
//! `MOVE_DIRECT_ATTACK`: the best attacker fights, or a sacrifice when the odds are poor),
//! repeated while the odds of the best attacker against the best defender, given how many
//! attackers remain for how many defenders, make the assault worth it (C++
//! `AI_groupAttack`/`AI_attackCityMove` follow-through).

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::garrison;
use crate::units::{alive, hold, move_to, nearest_city, skip};
use crate::view::{attack_power, defense_power, View};

/// Largest stack we build.
pub const MAX_STACK: usize = 8;

/// A target for a field army (computed once per turn in the view).
#[derive(Copy, Clone, Debug)]
pub struct Target {
    pub plot: PlotIdx,
    pub value: i32,
    /// Enemy (non-animal or animal) units visible there.
    pub units: i32,
    pub city: bool,
    pub lair: bool,
}

pub fn is_army_unit(g: &Game, u: UnitId) -> bool {
    let unit = &g.units[u];
    let info = &g.d.units[unit.unit_type];
    info.combat > 0
        && !info.found
        && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
        && matches!(
            unit.ai_type,
            Some(UnitAIType::UNITAI_ATTACK) | Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL) | Some(UnitAIType::UNITAI_COUNTER) | Some(UnitAIType::UNITAI_RESERVE) | Some(UnitAIType::UNITAI_PILLAGE)
        )
}

/// Units of the group able to attack now (not the casters that march with it without fighting
/// for it, `magic::spared_caster`).
fn attackers(g: &Game, grp: GroupId) -> Vec<UnitId> {
    g.groups[grp].units.iter().copied().filter(|&u| crate::campaign::fights(g, u) && g.unit_moves_left(u) > 0).collect()
}

/// Fight estimate against a plot.
#[derive(Copy, Clone, Debug)]
pub struct Assessment {
    /// Best odds (per mille) of one of our attackers against their best defender.
    pub best: i32,
    /// Defenders there.
    pub n_def: i32,
    /// Our attackers able to fight now.
    pub n_att: i32,
    /// Expected "defender kills" of the whole stack, per mille: each attacker counts its odds
    /// plus a third of the rest (a lost fight still wounds the defender); collateral
    /// attackers count more against stacks.
    pub effective: i32,
}

/// Fight estimate against plot `q`.
pub(crate) fn assess(g: &Game, grp: GroupId, q: PlotIdx) -> Option<Assessment> {
    let mut atts = attackers(g, grp);
    if atts.is_empty() {
        return None;
    }
    atts.sort_by_key(|&u| (std::cmp::Reverse(attack_power(g, u)), u));
    let owner = g.groups[grp].owner;
    let mut best = 0;
    let mut n_def = 0;
    let mut effective = 0;
    let mut any = false;
    for &u in atts.iter().take(MAX_STACK) {
        if !g.unit_can_move_or_attack_into(u, q, false) {
            continue;
        }
        any = true;
        let Some(def) = g.plot_get_best_defender(q, None, Some(owner), Some(u), false, false, false) else {
            return Some(Assessment { best: 1000, n_def: 0, n_att: atts.len() as i32, effective: 1000 * atts.len() as i32 });
        };
        let odds = g.get_combat_odds(u, def);
        best = best.max(odds);
        n_def = n_def.max(g.plot_get_num_visible_enemy_defenders(q, u));
        effective += odds + (1000 - odds) / 3;
        if g.d.units[g.units[u].unit_type].collateral_damage > 0 {
            effective += 250;
        }
    }
    if !any {
        return None;
    }
    Some(Assessment { best, n_def: n_def.max(1), n_att: atts.len() as i32, effective })
}

/// Is an assault worth it?
pub fn attack_ok(a: &Assessment, city: bool) -> bool {
    if a.best >= 650 {
        return true;
    }
    let need = a.n_def * 1000 + if city { 300 } else { 0 };
    if a.best < 100 {
        return false;
    }
    a.effective >= need && (a.best >= 300 || a.n_att >= a.n_def + 2)
}

/// The group attack of `grp` on plot `q` played in expected values as the rules will fight it
/// (`campaign::simulate`, [`Order::Group`](crate::campaign::Order::Group)): the attackers
/// that can attack there, against the defenders we see (switch `FIELD_SIM`). Raiders on our land
/// or within three plots of one of our cities (any enemy but animals, which stay out of our
/// borders) count four times what they are worth: left alone they take the city. (Valued at face
/// value, the stacks let them be, and more of our cities fell to the barbarians: docs/AI.md,
/// round 7g.)
pub(crate) fn field_sim(g: &Game, grp: GroupId, q: PlotIdx) -> Option<crate::campaign::Sim> {
    let atts: Vec<UnitId> = attackers(g, grp).into_iter().filter(|&u| g.unit_can_move_or_attack_into(u, q, false)).collect();
    if atts.is_empty() {
        return None;
    }
    let p = g.groups[grp].owner;
    let team = g.players[p].team;
    let defs = crate::campaign::visible_defenders(g, team, q);
    let animals = !defs.is_empty() && defs.iter().all(|&d| g.d.units[g.units[d].unit_type].animal);
    let ours = g.map.plots[q].owner.is_some_and(|o| g.players[o].team == team);
    let (x, y) = g.map.xy(q);
    let near_city = ours || g.players[p].cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= 3);
    let kill_pct = if !animals && near_city { 400 } else { 100 };
    Some(crate::campaign::simulate(g, q, &defs, &atts, 0, crate::campaign::Order::Group, kill_pct, 200))
}

/// Is the group attack [`field_sim`] played worth starting (or, `fought`, going on with)? As an
/// expert decides, on the simulation's numbers. Nothing to fight (a worker to capture, our own
/// units there): yes. The attacker the rules send has good odds (65%; 75% for a `hurt` stack):
/// yes -- a raider, an animal, a shot at a barbarian town's garrison. Wearing the defenders down
/// at worse odds: only when the stack clears the plot (every defender 80% sure to fall) and the
/// kills are worth more than the losses (`gain`). A city to `capture` (an opportune capture, a
/// landing): when it falls; once the fighting has started, also on good odds or a winning trade.
/// (Before: `attack_ok`, which counted a third of a kill for every lost fight whatever the odds,
/// the defender's strength or its first strikes -- it sent four Warriors at a Griffon at 1% -- and
/// took the odds against a defender at full health for each attacker. Measured first as a pure
/// trade of what kills and losses are worth, the stacks traded lone units at 35-50% odds and the
/// player fell behind in units by turn 100: docs/AI.md, round 7g.)
pub(crate) fn field_go(s: &crate::campaign::Sim, city: bool, capture: bool, fought: bool, hurt: bool) -> bool {
    if s.n_def == 0 {
        return true;
    }
    let falls = s.used.is_some();
    if city && capture && !fought {
        return falls;
    }
    let good = s.first >= if hurt { 750 } else { 650 };
    good || falls && s.gain > 0
}

/// (Trace) Where the simulation ([`field_go`]) and the old estimate ([`attack_ok`]; for a hunt
/// also 30% odds with two attackers) disagree on attacking `q`: the firings of switch
/// `FIELD_SIM`, to fork.
fn trace_field(g: &Game, grp: GroupId, q: PlotIdx, city: bool, hurt: bool, hunt: bool) {
    let p = g.groups[grp].owner;
    if !crate::trace_on(p) {
        return;
    }
    let s = field_sim(g, grp, q);
    let new = s.is_some_and(|s| field_go(&s, city, false, false, hurt));
    let old = assess(g, grp, q).is_some_and(|a| if hurt { a.best >= 750 } else { attack_ok(&a, city) || hunt && a.best >= 300 && a.n_att >= 2 }) && precious_holds(g, grp, q).is_none();
    if new != old {
        let theirs: Vec<String> = g.map.plots[q].units.iter().map(|&o| format!("{}({}%) p{}", g.d.units[g.units[o].unit_type].base.type_.trim_start_matches("UNIT_"), 100 - g.units[o].damage, g.units[o].owner.0)).collect();
        let team = g.players[p].team;
        crate::trace!(g, p, "  field sim: {} {:?} on {:?} {:?} (seen {}): simulation {} ({:?}), old estimate {}", if hunt { "hunt" } else { "attack" }, grp, g.map.xy(q), theirs, crate::view::sees_units(g, team, q), new, s, old);
    }
}

/// The attacker the rules would send from `grp` into `q` (`AI_getBestGroupAttacker`, which already
/// puts heroes, veterans and war wizards last) is a hero or a veteran below its odds (switch
/// `PRECIOUS`, `campaign::precious_bar`), or a caster marching with the stack below war wizard
/// odds (switch `CASTER_STACKS`, `magic::spared_caster`): `(unit, odds, bar)`. The others are worse still, so the stack does
/// not attack there -- and does not pick the target either: a Balseraph stack whose best attacker
/// was a level-5 Warrior at 35% stood next to the same raider for 12 turns (docs/AI.md, "Forks of
/// the hero gate").
fn precious_holds(g: &Game, grp: GroupId, q: PlotIdx) -> Option<(UnitId, i32, i32)> {
    let p = g.units[g.group_get_head_unit(grp)?].owner;
    let precious = crate::switches::PRECIOUS.on(g, p);
    if !precious && !crate::switches::CASTER_STACKS.on(g, p) {
        return None;
    }
    let b = g.group_ai_get_best_group_attacker(grp, q, false, false, false).0?;
    // A caster that marches with the stack (`magic::spared_caster`): the rules send it below war
    // wizard odds only when no other unit of the stack can attack; then the stack stops too.
    let caster_bar = if crate::magic::spared_caster(g, b) { crate::magic::SPARED_CASTER_ODDS } else { 0 };
    let bar = if precious { crate::campaign::precious_bar(g, b) } else { 0 }.max(caster_bar);
    if bar == 0 {
        return None;
    }
    let odds = g.plot_get_best_defender(q, None, Some(p), Some(b), false, false, false).map_or(1000, |d| g.get_combat_odds(b, d));
    (odds < bar).then_some((b, odds, bar))
}

/// Attack plot `q` with the group until it falls or the odds turn bad. True if we fought.
pub fn assault(g: &mut Game, v: &View, grp: GroupId, q: PlotIdx) -> bool {
    let mut fought = false;
    let city = g.map.plots[q].plot_city.is_some();
    let by_sim = crate::switches::FIELD_SIM.on(g, v.p);
    // Casters of ours around the target cast first when the assault goes ahead (magic.rs).
    let goes = if by_sim { field_sim(g, grp, q).is_some_and(|s| field_go(&s, city, false, false, false)) } else { assess(g, grp, q).is_some_and(|a| a.n_def > 0 && attack_ok(&a, city)) };
    if goes {
        let atts = attackers(g, grp);
        crate::magic::before_attack(g, v, q, &atts);
    }
    for _ in 0..12 {
        if !alive(g, grp) {
            break;
        }
        let Some(head) = g.group_get_head_unit(grp) else { break };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) {
            break;
        }
        let Some(a) = assess(g, grp, q) else { break };
        let (n_def, n_att) = (a.n_def, a.n_att);
        crate::trace!(g, v.p, "  assault {:?} on {:?}: {:?} ok {}", grp, g.map.xy(q), a, attack_ok(&a, city));
        if n_def == 0 {
            break;
        }
        if by_sim {
            let s = field_sim(g, grp, q);
            crate::trace!(g, v.p, "  assault {:?} on {:?} simulated: {:?} go {}", grp, g.map.xy(q), s, s.is_some_and(|s| field_go(&s, city, false, fought, false)));
            if !s.is_some_and(|s| field_go(&s, city, false, fought, false)) {
                break;
            }
        } else if !attack_ok(&a, city) {
            break;
        }
        // The attacker the rules would send is a hero or a veteran below its odds: the stack stops
        // here.
        if let Some((b, odds, bar)) = precious_holds(g, grp, q) {
            crate::trace!(g, v.p, "stack {:?} holds: its next attacker {} has odds {} (wants {})", grp, g.d.units[g.units[b].unit_type].base.type_, odds, bar);
            g.tel_decision(v.p, "military", "precious_held", "assault", 1, String::new);
            break;
        }
        g.groups[grp].ai_group_attack = false;
        let before = g.groups[grp].units.len();
        move_to(g, grp, q, MOVE_DIRECT_ATTACK);
        fought = true;
        if !alive(g, grp) {
            break;
        }
        // Nothing happened (no attack possible): stop.
        if g.groups[grp].units.len() == before && attackers(g, grp).len() as i32 == n_att {
            break;
        }
    }
    if alive(g, grp) {
        g.groups[grp].ai_group_attack = false;
        // The defenders are gone: move in (capture a city, destroy a lair, take workers).
        if let Some(head) = g.group_get_head_unit(grp) {
            if g.unit_plot(head) != q && !g.plot_is_visible_enemy_unit_for_unit(q, head) && g.unit_moves_left(head) > 0 && g.group_can_move_into(grp, q, false) {
                move_to(g, grp, q, 0);
            }
        }
    }
    fought
}

/// Merge other army units on this plot into `grp` (up to [`MAX_STACK`]).
fn merge_here(g: &mut Game, v: &View, grp: GroupId, here: PlotIdx) {
    if g.groups[grp].units.len() >= MAX_STACK {
        return;
    }
    let others: Vec<UnitId> = g.map.plots[here].units.iter().copied().filter(|&o| g.units[o].owner == v.p && g.units[o].group != Some(grp) && is_army_unit(g, o)).collect();
    for o in others {
        if g.groups[grp].units.len() >= MAX_STACK {
            break;
        }
        // Only fresh units join (a group whose members have different moves left would keep
        // the turn from ending).
        if g.unit_plot(o) == here && g.unit_moves_left(o) == g.unit_max_moves(o) && g.unit_can_join_group(o, here, grp) {
            g.unit_join_group(o, Some(grp), true);
        }
    }
}

/// Enemy city our army marches on: this turn's war operation (`campaign.rs`), else the
/// closest known enemy city to any of our cities.
pub fn war_target(g: &Game, v: &View) -> Option<CityId> {
    // (The plan was made earlier this turn: the city may have fallen or been razed since.)
    if let Some(c) = v.campaign.war_target().filter(|&c| g.cities.contains(c) && g.cities[c].owner != v.p) {
        return Some(c);
    }
    let preparing = crate::diplomacy::preparing_target(g, v.team);
    if v.civ_enemies.is_empty() && preparing.is_none() {
        return None;
    }
    let ours: Vec<(i32, i32, Option<AreaId>)> = g.players[v.p].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y, g.map.plots[g.city_plot(c)].area)).collect();
    let mut best: Option<(CityId, i32)> = None;
    for pl in g.players.iter() {
        if !pl.alive || !(v.civ_enemies.contains(&pl.team) || v.civ_enemies.is_empty() && preparing == Some(pl.team)) {
            continue;
        }
        for &c in &pl.cities {
            let cp = g.city_plot(c);
            if !g.map.plots[cp].revealed[v.team] {
                continue;
            }
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            let area = g.map.plots[cp].area;
            let Some(dist) = ours.iter().filter(|o| o.2 == area).map(|&(x, y, _)| g.plot_distance(x, y, cx, cy)).min() else { continue };
            if best.map_or(true, |(bc, bd)| dist < bd || (dist == bd && c < bc)) {
                best = Some((c, dist));
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Stack strength in rough power units (sum of attack power of its fighters).
fn stack_power(g: &Game, grp: GroupId) -> i32 {
    g.groups[grp].units.iter().map(|&u| attack_power(g, u)).sum()
}

/// One decision for an army stack led by `u`.
/// Diagnostic (`FFH_AI_EXPOSURE=1`): report army stacks of `p` left on a plot that a *much
/// stronger* visible enemy unit can reach next turn -- the single strongest enemy attacker in
/// reach out-powers our best defender by half again (`maxatt > 1.5x bestdef`), the "obviously
/// about to be killed" case. `natt` is how many enemy attackers reach the plot and `wipe` whether
/// there are enough to kill every unit in the stack. Skips a garrison in one of our cities and a
/// stack next to its target about to strike this turn; `ctx=op` marks a stack marching/striking in
/// an operation, `ctx=loose` one not in any. Compares single units (not summed power, which
/// over-counts many weak attackers a strong defender beats one at a time). Pure read, printed only
/// when the env var is set, so it has no cost or effect otherwise. Called once per player's turn.
pub fn report_exposure(g: &Game, v: &View, p: PlayerId) {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var("FFH_AI_EXPOSURE").is_ok()) {
        return;
    }
    let mut seen = std::collections::BTreeSet::new();
    for &u in &g.players[p].units {
        if !g.units.contains(u) {
            continue;
        }
        let Some(grp) = g.units[u].group else { continue };
        if !seen.insert(grp) || !g.groups[grp].units.iter().any(|&o| is_army_unit(g, o)) {
            continue;
        }
        let head = g.group_get_head_unit(grp).unwrap_or(u);
        let here = g.unit_plot(head);
        if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) || crate::campaign::striking(g, v, grp) {
            continue;
        }
        let (hx, hy) = g.map.xy(here);
        let best_def = g.groups[grp].units.iter().map(|&o| defense_power(g, o)).max().unwrap_or(0).max(1);
        // Strongest single visible enemy attacker that can reach this plot next turn, and how many.
        let mut max_att = 0;
        let mut n_att = 0;
        for pl in g.players.iter() {
            if !pl.alive || !v.enemies.contains(&pl.team) {
                continue;
            }
            for &e in &pl.units {
                if !g.units.contains(e) {
                    continue;
                }
                let eu = &g.units[e];
                let att = attack_power(g, e);
                if att == 0 {
                    continue;
                }
                let Some(ep) = g.map.plot_xy(eu.x, eu.y) else { continue };
                if !g.plot_is_visible(ep, v.team) {
                    continue;
                }
                let range = (g.d.units[eu.unit_type].moves.max(1) + 1).min(4);
                if g.plot_distance(eu.x, eu.y, hx, hy) <= range {
                    max_att = max_att.max(att);
                    n_att += 1;
                }
            }
        }
        if max_att * 2 > best_def * 3 {
            let ctx = if crate::campaign::on_the_move(v, grp) { "op" } else { "loose" };
            eprintln!(
                "[EXPOSED t{} p{} grp{}] bestdef={} maxatt={} ratio={} natt={} units={} wipe={} ctx={}",
                g.game_turn(), p.0, grp.0, best_def, max_att, max_att * 100 / best_def, n_att, g.groups[grp].units.len(), n_att >= g.groups[grp].units.len(), ctx
            );
        }
    }
}

pub fn army(g: &mut Game, v: &mut View, mut grp: GroupId, mut u: UnitId) {
    let p = v.p;
    let here = g.unit_plot(u);
    if v.barbarian {
        crate::units::barbarian(g, v, grp, u);
        return;
    }

    // A group attack the rules queued (AI follow-through).
    if g.groups[grp].ai_group_attack {
        let (x, y) = (g.groups[grp].ai_group_attack_x, g.groups[grp].ai_group_attack_y);
        g.groups[grp].ai_group_attack = false;
        if let Some(q) = g.map.plot_xy(x, y) {
            if assault(g, v, grp, q) {
                finish(g, grp);
                return;
            }
        }
    }

    // An undefended city of ours (a fresh one, typically founded by the settler we escorted):
    // its first garrison.
    if let Some(c) = g.map.plots[here].plot_city.filter(|&c| g.cities[c].owner == p) {
        if garrison(g, p, here).0 == 0 && v.garrison.get(&c).copied().unwrap_or(0) == 0 {
            *v.garrison.entry(c).or_insert(0) += 1;
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_CITY_DEFENSE);
            if let Some(ng) = g.units[u].group {
                hold(g, ng);
            }
            return;
        }
    }

    merge_here(g, v, grp, here);
    crate::traits::before_army(g, v, grp); // trait-play
    if !alive(g, grp) {
        return;
    }

    // Heal before anything but a sure kill.
    let max_hp = g.d.defines.max_hit_points.max(1);
    let hurt = g.groups[grp].units.iter().any(|&o| g.units[o].damage * 100 / max_hp > 35);

    // Adjacent targets first (the operation's target city is left to its strike).
    crate::campaign::plan(g, v);
    // An enemy city next to us that is empty, or that this stack clearly takes: take it, plan or
    // not (C++ `AI_stackAttackCity(1, 160)`).
    if crate::switches::CAPTURE.on(g, p) && opportune(g, v, grp, u) {
        finish(g, grp);
        return;
    }
    let op_target = v.campaign.ops.first().map(|o| o.plot);
    if let Some(q) = best_adjacent(g, grp, here, hurt, op_target) {
        assault(g, v, grp, q);
        finish(g, grp);
        return;
    }
    if !alive(g, grp) {
        return;
    }
    // (Not a stack about to strike: the strike estimate counts its wounds, and the city's
    // defenders heal faster than we do.)
    if crate::switches::HEAL.on(g, p) {
        match heal(g, v, grp, u) {
            Healed::Done => return,
            Healed::Continue(ng, nu) => {
                grp = ng;
                u = nu;
            }
        }
    } else if hurt && !crate::campaign::striking(g, v, grp) && v.danger_at(here) * 2 < g.groups[grp].units.iter().map(|&o| defense_power(g, o)).sum::<i32>().max(1) * 3 {
        if g.group_can_start_mission(grp, MissionType::MISSION_HEAL, -1, -1, None, false) {
            crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
            return;
        }
    }

    // A city of ours in peril within a few turns: defend it (unless the stack is on its way to
    // a siege: campaign.rs).
    if !v.peril.is_empty() && !crate::campaign::on_the_move(v, grp) {
        let reach = v.flood.run(g, u, 5, false);
        if let Some(r) = v.peril.iter().filter_map(|&(q, _)| reach.iter().find(|r| r.plot == q).copied()).min_by_key(|r| (r.turns, r.plot)) {
            crate::trace!(g, p, "stack {:?} defends city at {:?}", grp, g.map.xy(r.plot));
            if r.plot == here {
                hold(g, grp);
            } else {
                move_to(g, grp, r.plot, 0);
                finish(g, grp);
            }
            return;
        }
    }

    // A settler waits for an escort.
    if !v.escort_requests.is_empty() && g.groups[grp].units.len() == 1 {
        let reach = v.flood.run(g, u, 6, false);
        if let Some((i, r)) = v.escort_requests.iter().enumerate().filter_map(|(i, &q)| reach.iter().find(|r| r.plot == q).map(|r| (i, *r))).min_by_key(|(_, r)| (r.turns, r.plot)) {
            v.escort_requests.remove(i);
            if r.plot != here {
                move_to(g, grp, r.plot, 0);
            }
            finish(g, grp);
            return;
        }
    }

    // An amphibious invasion (naval.rs): a committed stack boards the transport in port to
    // cross the water; a stack already landed on the enemy's landmass takes the overseas city
    // (which `campaign.rs` cannot, as it only targets landmasses where we hold a city).
    if crate::naval::amphibious(g, v, grp, u) {
        return;
    }

    // War (or its preparation): the operation against the chosen enemy city; stacks close to
    // a barbarian target act on its operation before hunting (campaign.rs).
    if crate::campaign::act(g, v, grp, u, true) {
        return;
    }

    // Hunt: the best target in reach that the stack can take.
    if hunt(g, v, grp, u) {
        return;
    }

    // A Raiders leader's spare stack raids the enemy's improvements (traits.rs).
    if crate::traits::raid(g, v, grp, u) { return; } // trait-play

    // Peacetime campaign against a barbarian city (campaign.rs).
    if crate::campaign::act(g, v, grp, u, false) {
        return;
    }

    // Gather: join the nearest other stack within a few turns, else hold in the nearest city.
    // (Posting lone units here to block barbarian spawns near our cities was tried and reverted:
    // the units held in cities fight for them. docs/AI_LOG.md, "Fog-busting posts".)
    if g.groups[grp].units.len() < MAX_STACK {
        let reach = v.flood.run(g, u, 3, false);
        let mut best: Option<(PlotIdx, i32)> = None;
        for r in &reach {
            if r.plot == here {
                continue;
            }
            let n = g.map.plots[r.plot].units.iter().filter(|&&o| g.units[o].owner == p && is_army_unit(g, o)).count() as i32;
            if n == 0 || n as usize >= MAX_STACK {
                continue;
            }
            // Smaller stacks walk to bigger ones (ties: lower plot index gathers).
            let mine = g.groups[grp].units.len() as i32;
            if n < mine || (n == mine && r.plot.idx() > here.idx()) {
                continue;
            }
            let score = n * 10 - r.turns * 5;
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((r.plot, score));
            }
        }
        if let Some((q, _)) = best {
            move_to(g, grp, q, 0);
            finish(g, grp);
            return;
        }
    }
    crate::trace!(g, p, "stack {:?} ({} units, power {}) idle at {:?}; {} targets", grp, g.groups[grp].units.len(), stack_power(g, grp), g.map.xy(here), v.targets.len());
    if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) {
        hold(g, grp);
    } else if let Some(c) = nearest_city(g, p, here) {
        move_to(g, grp, g.city_plot(c), 0);
        finish(g, grp);
    } else {
        hold(g, grp);
    }
}

/// What [`heal`] did: the stack's turn is spent, or a stack (perhaps the healthy part of it,
/// with a new head) goes on with its turn.
enum Healed {
    Done,
    Continue(GroupId, UnitId),
}

/// A unit counts as hurt from this much damage (percent): in a field stack (most of it hurt sends
/// the stack to heal) and in one of our cities (it splits off and heals there). One threshold for
/// both, so a stack leaves a city only in a state the field rule accepts: with More Naval AI's 30%
/// in cities (`AI_heal(30, 1)`), units healed to 21-30% walked out and the next turn's field rule
/// sent them back, turn after turn (a fork of seed 26006, turn 239: docs/AI.md, "Forks of the
/// healing rule").
const HURT_PCT: i32 = 20;
/// A lone unit stops to heal from this much damage (the old rule for every stack).
const ALONE_PCT: i32 = 35;

/// Healing as More Naval AI's stacks do (`CvUnitAI::AI_heal`), fitted to our small stacks: in one
/// of our cities the wounded split off and heal while the rest go on; in the field a stack
/// committed to a siege stops only when more than half of it is hurt, and only where healing is
/// better than 10% a turn (our land, a city: not enemy or neutral land, where it heals at 5-10%
/// beside defenders that heal faster), else it walks into a city of ours within two turns or onto
/// our land within one, else it carries on; a loose stack (a few units hunting raiders) heals where
/// it stands for one badly hurt unit, as before. A stack about to strike never stops. (Before: one unit over 35% damage
/// halted the whole stack wherever it stood, committed stacks next to their target included. More
/// Naval AI's rule for every stack -- stop only when most of it is hurt -- lost units and cities
/// here: our loose stacks are two to four units, and a badly hurt one is a large part of them.)
fn heal(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> Healed {
    let p = v.p;
    if crate::campaign::striking(g, v, grp) {
        return Healed::Continue(grp, u);
    }
    let here = g.unit_plot(u);
    let max_hp = g.d.defines.max_hit_points.max(1);
    let pct = |g: &Game, o: UnitId| g.units[o].damage * 100 / max_hp;
    let units = g.groups[grp].units.clone();
    let safe = |g: &Game, v: &View| v.danger_at(here) * 2 < g.groups[grp].units.iter().map(|&o| defense_power(g, o)).sum::<i32>().max(1) * 3;
    let can_heal = |g: &Game, grp: GroupId| g.group_can_start_mission(grp, MissionType::MISSION_HEAL, -1, -1, None, false);
    let our_city = g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p);
    if our_city {
        // The wounded (that can still heal this turn: they have not moved) split off and heal.
        let wounded: Vec<UnitId> = units.iter().copied().filter(|&o| pct(g, o) >= HURT_PCT && !g.unit_has_moved(o)).collect();
        if wounded.is_empty() {
            return Healed::Continue(grp, u);
        }
        if wounded.len() == units.len() {
            if can_heal(g, grp) {
                crate::trace!(g, p, "stack {:?} ({} units) heals in {:?}", grp, units.len(), g.map.xy(here));
                g.tel_decision(p, "military", "heal_city", "heal", units.len() as i64, String::new);
                crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
                return Healed::Done;
            }
            return Healed::Continue(grp, u);
        }
        // Move one part into a new group without taking the head out of the old one (a head
        // leaving its group marks the group for separation, and it would take no more units).
        let head_wounded = wounded.contains(&u);
        let leave: Vec<UnitId> = if head_wounded { units.iter().copied().filter(|o| !wounded.contains(o)).collect() } else { wounded.clone() };
        let first = leave[0];
        g.unit_join_group(first, None, true);
        let Some(ng) = g.units[first].group else { return Healed::Continue(grp, u) };
        for &o in &leave[1..] {
            g.unit_join_group(o, Some(ng), true);
        }
        let (heal_grp, go_grp, go_head) = if head_wounded { (grp, ng, first) } else { (ng, grp, u) };
        crate::trace!(g, p, "stack {:?}: {} wounded split off to heal in {:?}", grp, wounded.len(), g.map.xy(here));
        g.tel_decision(p, "military", "heal_split", "heal", wounded.len() as i64, String::new);
        if alive(g, heal_grp) && can_heal(g, heal_grp) {
            crate::units::push(g, heal_grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
        }
        if alive(g, go_grp) && g.units.contains(go_head) && g.units[go_head].group == Some(go_grp) && g.group_ready_to_move(go_grp, false) {
            return Healed::Continue(go_grp, go_head);
        }
        return Healed::Done;
    }
    // In the field.
    let n = units.len();
    let badly = units.iter().any(|&o| pct(g, o) > ALONE_PCT);
    let most = units.iter().filter(|&&o| pct(g, o) >= HURT_PCT).count() * 2 > n;
    // A loose stack (a few units hunting raiders near home) heals where it stands as soon as one
    // unit is badly hurt, as before this rule: making it walk home, or wait for most of it to be
    // hurt, let raiders run and lost cities to the barbarians (+29-43% on fresh seeds).
    let marching = crate::campaign::on_the_move(v, grp);
    if !marching {
        if badly && safe(g, v) && can_heal(g, grp) {
            crate::trace!(g, p, "stack {:?} ({} units) heals at {:?}", grp, n, g.map.xy(here));
            g.tel_decision(p, "military", "heal_stack", "heal", n as i64, String::new);
            crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
            return Healed::Done;
        }
        return Healed::Continue(grp, u);
    }
    // A stack committed to a siege past gathering stops only when most of it is hurt (C++).
    if !most {
        if badly && safe(g, v) {
            crate::trace!(g, p, "stack {:?} ({} units) marches on with its wounded at {:?}", grp, n, g.map.xy(here));
            g.tel_decision(p, "military", "heal_skipped", "few_hurt", 1, String::new);
        }
        return Healed::Continue(grp, u);
    }
    if g.unit_heal_rate(u, here) > 10 && safe(g, v) && can_heal(g, grp) {
        crate::trace!(g, p, "stack {:?} ({} units) heals at {:?}", grp, n, g.map.xy(here));
        g.tel_decision(p, "military", "heal_stack", "heal", n as i64, String::new);
        crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
        return Healed::Done;
    }
    // Poor healing here: into a city of ours within two turns, else onto our land within one.
    let ours = |g: &Game, q: PlotIdx| g.map.plots[q].plot_city.is_some_and(|c| g.cities[c].owner == p);
    let reach = v.flood.run(g, u, 2, false);
    let city = reach.iter().filter(|r| ours(g, r.plot)).min_by_key(|r| (r.turns, r.plot)).copied();
    let land = || reach.iter().filter(|r| r.turns <= 1 && g.map.plots[r.plot].owner == Some(p) && v.danger_at(r.plot) == 0).min_by_key(|r| (r.turns, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(v.team), true)), r.plot)).copied();
    if let Some(r) = city.or_else(land) {
        crate::trace!(g, p, "stack {:?} ({} units) goes to heal at {:?}", grp, n, g.map.xy(r.plot));
        g.tel_decision(p, "military", "heal_retreat", "heal", n as i64, String::new);
        move_to(g, grp, r.plot, 0);
        finish(g, grp);
        return Healed::Done;
    }
    g.tel_decision(p, "military", "heal_skipped", "poor_healing", 1, String::new);
    Healed::Continue(grp, u)
}

/// Make sure the group's turn ends.
fn finish(g: &mut Game, grp: GroupId) {
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
}

/// Opportunistic captures (switch `CAPTURE`): an enemy city at war (a civ's or the barbarians')
/// that the stack can walk into this turn because it is empty, or one next to it whose defenders
/// it takes with a reserve to spare (the siege operations' estimate, `campaign::covers`) -- even
/// when no operation targets it (the targets of our operations are left to their strikes). An
/// expert takes the city the enemy has left open, wherever his army stands. Before, a stack next
/// to an empty enemy city walked on: the plan named another city, and the hunt only went for
/// visible units. True if the stack acted.
fn opportune(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    let team = v.team;
    let here = g.unit_plot(u);
    let (x, y) = g.map.xy(here);
    if g.unit_moves_left(u) <= 0 {
        return false;
    }
    // Enemy cities around (two plots: as far as a unit with two moves gets this turn).
    let mut cities: Vec<(i32, CityId, PlotIdx)> = Vec::new();
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let Some(c) = g.map.plots[q].plot_city else { continue };
            let owner_team = g.players[g.cities[c].owner].team;
            if owner_team == team || !g.at_war(team, owner_team) || !crate::view::sees_units(g, team, q) || v.campaign.ops.iter().any(|o| o.plot == q) {
                continue;
            }
            let (qx, qy) = g.map.xy(q);
            cities.push((g.plot_distance(x, y, qx, qy), c, q));
        }
    }
    if cities.is_empty() {
        return false;
    }
    cities.sort();
    let mut reach: Option<Vec<crate::reach::Reached>> = None;
    for (d, c, q) in cities {
        let defs = crate::campaign::visible_defenders(g, team, q);
        let name = g.cities[c].name.clone();
        if defs.is_empty() {
            // Walk in, if we get there this turn.
            let arrives = d == 1 && g.group_can_move_into(grp, q, false) || {
                let r = reach.get_or_insert_with(|| v.flood.run(g, u, 1, false));
                r.iter().any(|r| r.plot == q && r.turns == 0)
            };
            if !arrives {
                continue;
            }
            crate::trace!(g, p, "stack {:?} ({} units) walks into the empty city {}", grp, g.groups[grp].units.len(), name);
            g.tel_decision(p, "military", "capture_empty", "capture", 1, String::new);
            move_to(g, grp, q, 0);
            return true;
        }
        if d != 1 {
            continue;
        }
        // Clearly: the siege estimate takes the city with a reserve to spare, and the odds are
        // ones the assault itself goes on with (the estimate counts the wounds lost fights leave,
        // which a stack fighting at 10% odds should not rely on).
        let units = g.groups[grp].units.clone();
        let takes = if crate::switches::FIELD_SIM.on(g, p) {
            field_sim(g, grp, q).is_some_and(|s| field_go(&s, true, true, false, false))
        } else {
            assess(g, grp, q).is_some_and(|a| attack_ok(&a, true)) && crate::campaign::covers(g, c, q, &defs, &units, true, false)
        };
        if !takes {
            continue;
        }
        crate::trace!(g, p, "stack {:?} ({} units) takes its chance on {} ({} defenders)", grp, units.len(), name, defs.len());
        g.tel_decision(p, "military", "capture_attack", "capture", defs.len() as i64, String::new);
        assault(g, v, grp, q);
        if !g.cities.contains(c) || g.cities[c].owner == p {
            g.tel_decision(p, "military", "capture_taken", "capture", 1, String::new);
        }
        return true;
    }
    false
}

/// Best adjacent enemy plot worth attacking now.
fn best_adjacent(g: &Game, grp: GroupId, here: PlotIdx, hurt: bool, skip: Option<PlotIdx>) -> Option<PlotIdx> {
    let head = g.group_get_head_unit(grp)?;
    let (x, y) = g.map.xy(here);
    let mut best: Option<(PlotIdx, i64)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) || skip == Some(q) {
            continue;
        }
        // Do not attack into civ cities unless that is the plan (campaign.rs does).
        let city = g.map.plots[q].plot_city;
        if city.is_some_and(|c| !g.player_is_barbarian(g.cities[c].owner)) {
            continue;
        }
        trace_field(g, grp, q, city.is_some(), hurt, false);
        let (ok, score) = if crate::switches::FIELD_SIM.on(g, g.groups[grp].owner) {
            let Some(s) = field_sim(g, grp, q) else { continue };
            (field_go(&s, city.is_some(), false, false, hurt), if city.is_some() { 1_000_000 + s.best as i64 } else { s.gain })
        } else {
            let Some(a) = assess(g, grp, q) else { continue };
            (if hurt { a.best >= 750 } else { attack_ok(&a, city.is_some()) }, (a.best + if city.is_some() { 300 } else { 0 }) as i64)
        };
        if !ok || precious_holds(g, grp, q).is_some() {
            continue;
        }
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((q, score));
        }
    }
    best.map(|(q, _)| q)
}

/// Peacetime (and barbarian) targets: move toward the best one we can take.
fn hunt(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if v.targets.is_empty() {
        return false;
    }
    let here = g.unit_plot(u);
    let power = stack_power(g, grp);
    let n_att = attackers(g, grp).len() as i32;
    if n_att == 0 {
        return false;
    }
    let reach = v.flood.run(g, u, 6, true);
    let mut cands: Vec<(i32, usize, i32)> = Vec::new(); // (score, target index, turns)
    for (i, t) in v.targets.iter().enumerate() {
        let Some(r) = reach.iter().find(|r| r.plot == t.plot) else { continue };
        if v.target_claims.iter().any(|&(q, _)| q == t.plot) && !t.city {
            continue;
        }
        // The operation's target is taken by the operation.
        if t.city && v.campaign.ops.iter().any(|o| o.plot == t.plot) {
            continue;
        }
        // Rough feasibility before exact odds: their strongest defender against our stack.
        let their: i32 = g.map.plots[t.plot].units.iter().filter(|&&e| g.units[e].owner != v.p).map(|&e| defense_power(g, e)).max().unwrap_or(0);
        if their > 0 && their * t.units.max(1) > power * 2 {
            continue;
        }
        // (An empty city needs no army: whoever gets there takes it, if it is still empty; not
        // worth a long march.)
        if t.city && n_att < 3 && t.units > 0 || t.city && t.units == 0 && r.turns > 2 {
            continue;
        }
        let score = t.value * 10 / (r.turns + 2);
        cands.push((score, i, r.turns));
    }
    cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, i, _) in cands.iter().take(4) {
        let t = v.targets[i];
        // Exact check where it matters: the best odds against what stands there.
        if t.units > 0 {
            trace_field(g, grp, t.plot, t.city, false, true);
            if crate::switches::FIELD_SIM.on(g, v.p) {
                if !field_sim(g, grp, t.plot).is_some_and(|s| field_go(&s, t.city, false, false, false)) {
                    continue;
                }
            } else {
                match assess(g, grp, t.plot) {
                    Some(a) if attack_ok(&a, t.city) || a.best >= 300 && a.n_att >= 2 => {}
                    _ => continue,
                }
            }
            if precious_holds(g, grp, t.plot).is_some() {
                continue;
            }
        }
        v.target_claims.push((t.plot, grp));
        crate::trace!(g, v.p, "stack {:?} ({} units) hunts {:?} (value {}, units {}, lair {}, city {})", grp, g.groups[grp].units.len(), g.map.xy(t.plot), t.value, t.units, t.lair, t.city);
        let (tx, ty) = g.map.xy(t.plot);
        let (x, y) = g.map.xy(here);
        if g.plot_distance(x, y, tx, ty) <= 1 && t.units > 0 {
            assault(g, v, grp, t.plot);
        } else if t.units == 0 {
            if t.city {
                g.tel_decision(v.p, "military", "capture_empty_hunt", "capture", 1, String::new);
            }
            move_to(g, grp, t.plot, 0);
        } else {
            // Next to it, on the best defensive ground we reach first.
            let reach = v.flood.run(g, u, 6, false);
            let staging = reach
                .iter()
                .filter(|r| {
                    let (rx, ry) = g.map.xy(r.plot);
                    g.plot_distance(rx, ry, tx, ty) == 1 && g.map.plots[r.plot].plot_city.map_or(true, |c| g.cities[c].owner == v.p)
                })
                .min_by_key(|r| (r.turns, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(v.team), true)), r.plot))
                .copied();
            match staging {
                Some(r) if r.plot != here => move_to(g, grp, r.plot, 0),
                _ => {}
            }
        }
        finish(g, grp);
        return true;
    }
    false
}

/// Targets for field armies near our lands: enemy units outside civ cities, lairs, barbarian
/// cities.
pub fn find_targets(g: &Game, v: &View) -> Vec<Target> {
    let p = v.p;
    let pl = &g.players[p];
    let ours: Vec<(i32, i32)> = pl.cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
    if ours.is_empty() {
        return Vec::new();
    }
    let near = |x: i32, y: i32, r: i32| ours.iter().map(|&(cx, cy)| g.plot_distance(x, y, cx, cy)).min().unwrap_or(i32::MAX) <= r;
    let mut out: Vec<Target> = Vec::new();
    // Enemy units (visible).
    for opl in g.players.iter() {
        if !opl.alive || !v.enemies.contains(&opl.team) {
            continue;
        }
        for &e in &opl.units {
            let unit = &g.units[e];
            let Some(q) = g.map.plot_xy(unit.x, unit.y) else { continue };
            if !crate::view::sees_units(g, v.team, q) || g.unit_is_invisible(e, v.team, false) || g.plot_is_water(q) {
                continue;
            }
            if out.iter().any(|t| t.plot == q) {
                continue;
            }
            let city = g.map.plots[q].plot_city;
            if city.is_some_and(|c| !g.player_is_barbarian(g.cities[c].owner)) {
                continue; // civ cities: war logic
            }
            let in_ours = g.map.plots[q].owner == Some(p);
            if !in_ours && !near(unit.x, unit.y, if city.is_some() { 10 } else { 6 }) {
                continue;
            }
            let n = g.map.plots[q].units.iter().filter(|&&o| g.units[o].owner == unit.owner).count() as i32;
            let lair = g.map.plots[q].improvement_type.is_some_and(|i| g.d.improvements[i].spawn_unit_type.is_some());
            let mut value = 150 + 30 * n;
            if in_ours {
                value += 250;
            }
            if lair {
                value += 250;
            }
            if city.is_some() {
                value += 700;
            }
            out.push(Target { plot: q, value, units: n, city: city.is_some(), lair });
        }
    }
    // Empty enemy cities near our lands (switch `CAPTURE`): a stack walks in (`hunt`).
    if crate::switches::CAPTURE.on(g, p) {
        for opl in g.players.iter() {
            if !opl.alive || !v.enemies.contains(&opl.team) {
                continue;
            }
            for &c in &opl.cities {
                let q = g.city_plot(c);
                let (x, y) = (g.cities[c].x, g.cities[c].y);
                if !crate::view::sees_units(g, v.team, q) || out.iter().any(|t| t.plot == q) || !near(x, y, 10) {
                    continue;
                }
                if !crate::campaign::visible_defenders(g, v.team, q).is_empty() {
                    continue;
                }
                out.push(Target { plot: q, value: 900 + 100 * g.cities[c].population, units: 0, city: true, lair: false });
            }
        }
    }
    // Empty lairs near our lands.
    let barb_team = g.barbarian_team();
    if v.enemies.contains(&barb_team) {
        for &(cx, cy) in &ours {
            for dx in -6..=6 {
                for dy in -6..=6 {
                    let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
                    let plot = &g.map.plots[q];
                    if !plot.revealed[v.team] || plot.owner.is_some_and(|o| o != p) {
                        continue;
                    }
                    let Some(imp) = plot.improvement_type else { continue };
                    if g.d.improvements[imp].spawn_unit_type.is_none() || out.iter().any(|t| t.plot == q) {
                        continue;
                    }
                    out.push(Target { plot: q, value: 300, units: 0, city: false, lair: true });
                }
            }
        }
    }
    out
}
