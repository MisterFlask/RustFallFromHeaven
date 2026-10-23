//! Diplomacy: war plans, war declarations, peace, which deals to accept.
//!
//! - The rules keep the C++ AI counters (`team_ai_do_counter`: at war / at peace / war plan
//!   state); war success is counted by combat.
//! - War (C++ `AI_doWar`, simplified): when at peace, every few turns an aggressive enough
//!   leader picks a reachable, weaker, disliked neighbour and starts preparing
//!   (`WARPLAN_PREPARING_TOTAL`: production builds an army, stacks gather on the rally city
//!   nearest the target); the war is declared once the army is ready (or preparation drags on).
//!   The willingness is a deterministic function of the turn and the leader's `iMaxWarRand`,
//!   so the AI draws nothing from the game RNG.
//! - Peace: when the war goes badly (war success and power against us), or drags on without
//!   gains, a peace treaty is offered through a real deal the other side's controller judges.
//!
//! Relations are a small explicit score (alignment, religion, shared borders, war history, the
//! Trust spell) instead of the C++ attitude machinery.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;

use crate::view::View;

/// Power ratio (ours * 100 / their defensive power) at which the AI judges itself decisively
/// stronger than a rival -- strong enough to commit to total war (conquering the civ) rather than
/// a limited land grab. Reused as the "stronger than its neighbour" bar for guns-vs-butter siege
/// research (`research::siege_offense`), so both read the same notion of clear superiority.
pub(crate) const TOTAL_WAR_RATIO: i64 = 140;

/// The terms of [`relation`] (what each is, and its value), in its order; they sum to it. The
/// human's diplomacy screen shows them as the reasons of an attitude.
pub fn relation_parts(g: &Game, p: PlayerId, other: PlayerId) -> Vec<(&'static str, i32)> {
    let a = &g.players[p];
    let b = &g.players[other];
    let mut out = Vec::new();
    match (a.alignment, b.alignment) {
        (Some(x), Some(y)) if x != y => out.push(("different alignment", -(x.idx() as i32 - y.idx() as i32).abs() * 2)),
        (Some(_), Some(_)) => out.push(("same alignment", 1)),
        _ => {}
    }
    match (g.player_get_state_religion(p), g.player_get_state_religion(other)) {
        (Some(x), Some(y)) if x == y => out.push(("same state religion", 2)),
        (Some(_), Some(_)) => out.push(("different state religion", -1)),
        _ => {}
    }
    let (ta, tb) = (a.team, b.team);
    if g.teams[ta].ai_at_war_counter[tb] > 0 {
        out.push(("war between us", -1));
    }
    if g.team_is_open_borders(ta, tb) {
        out.push(("open borders", 1));
    }
    // The Trust spell (Spirit III, once a game: "Raises other leaders' attitudes towards you")
    // only sets the feat; its effect is the AI's attitude, as in FfH (C++
    // `CvPlayerAI::AI_getTrustAttitude`: +4, about one attitude step on that scale). Here 2 is a
    // step: Cautious to Pleased, Pleased to Friendly.
    if b.feat_accomplished[FeatType::FEAT_TRUST] {
        out.push(("the Trust spell", 2));
    }
    out
}

/// Relation of player `p` towards `other`: < 0 hostile, > 0 friendly.
pub fn relation(g: &Game, p: PlayerId, other: PlayerId) -> i32 {
    relation_parts(g, p, other).iter().map(|t| t.1).sum()
}

/// The terms of [`attitude`]'s score: [`relation_parts`], and the war itself while at war.
pub fn attitude_parts(g: &Game, p: PlayerId, towards: PlayerId) -> Vec<(&'static str, i32)> {
    let mut out = relation_parts(g, p, towards);
    if g.at_war(g.players[p].team, g.players[towards].team) {
        out.push(("at war", -2));
    }
    out
}

/// The attitude of a score (the sum of [`attitude_parts`]).
pub fn attitude_of_score(r: i32) -> AttitudeType {
    match r {
        i32::MIN..=-4 => AttitudeType::ATTITUDE_FURIOUS,
        -3..=-1 => AttitudeType::ATTITUDE_ANNOYED,
        0 => AttitudeType::ATTITUDE_CAUTIOUS,
        1..=2 => AttitudeType::ATTITUDE_PLEASED,
        _ => AttitudeType::ATTITUDE_FRIENDLY,
    }
}

/// Attitude from the relation score (C++ `AI_getAttitude` thresholds are not reproduced).
pub fn attitude(g: &Game, p: PlayerId, towards: PlayerId) -> AttitudeType {
    if p == towards {
        return AttitudeType::ATTITUDE_FRIENDLY;
    }
    attitude_of_score(attitude_parts(g, p, towards).iter().map(|t| t.1).sum())
}

pub(crate) fn is_team_leader(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    g.player_ids().into_iter().find(|&q| g.players[q].alive && g.players[q].team == team) == Some(p)
}

/// The alive (non-barbarian) leader player of a team.
pub(crate) fn team_leader(g: &Game, t: TeamId) -> Option<PlayerId> {
    g.player_ids().into_iter().find(|&q| g.players[q].alive && g.players[q].team == t)
}

/// Our attack units (army roles), alive or in training.
pub fn army_size(g: &Game, p: PlayerId) -> i32 {
    let pl = &g.players[p];
    let mut n = 0;
    for a in [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL, UnitAIType::UNITAI_COUNTER] {
        n += pl.ai_num_ai_units[a];
    }
    n
}

/// Army units within 3 plots of our city nearest to `target`'s cities (on a shared landmass):
/// the force staged for a war against it.
fn staged_army(g: &Game, p: PlayerId, target: TeamId) -> i32 {
    let mut best: Option<(i32, CityId)> = None;
    for &c in &g.players[p].cities {
        let area = g.map.plots[g.city_plot(c)].area;
        for q in g.player_ids() {
            if !g.players[q].alive || g.players[q].team != target {
                continue;
            }
            for &o in &g.players[q].cities {
                if g.map.plots[g.city_plot(o)].area != area {
                    continue;
                }
                let d = g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[o].x, g.cities[o].y);
                if best.map_or(true, |(b, bc)| (d, c) < (b, bc)) {
                    best = Some((d, c));
                }
            }
        }
    }
    let Some((_, rally)) = best else { return 0 };
    let (rx, ry) = (g.cities[rally].x, g.cities[rally].y);
    let army = [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL, UnitAIType::UNITAI_COUNTER];
    g.players[p]
        .units
        .iter()
        .filter(|&&u| g.units[u].ai_type.is_some_and(|a| army.contains(&a)) && g.plot_distance(g.units[u].x, g.units[u].y, rx, ry) <= 3)
        .count() as i32
}

/// Army we want before declaring a prepared war.
pub fn army_needed_for_war(g: &Game, p: PlayerId, target: TeamId) -> i32 {
    let their_cities = g.teams[target].num_cities;
    (4 + g.players[p].cities.len() as i32 / 2 + their_cities / 2).min(14)
}

/// The team we are preparing war against (C++ `WARPLAN_PREPARING_*`).
pub fn preparing_target(g: &Game, team: TeamId) -> Option<TeamId> {
    g.team_ids().into_iter().find(|&o| o != team && g.team_is_alive(o) && !g.teams[team].at_war[o] && g.team_ai_is_sneak_attack_preparing(team, o))
}

/// Distance (plots) from our nearest city to their nearest city, if both have cities in a
/// shared land area.
pub(crate) fn reach_distance(g: &Game, team: TeamId, other: TeamId) -> Option<i32> {
    let mut best: Option<i32> = None;
    for p in g.player_ids() {
        if !g.players[p].alive || g.players[p].team != team {
            continue;
        }
        for &c in &g.players[p].cities {
            let ac = g.map.plots[g.city_plot(c)].area;
            for q in g.player_ids() {
                if !g.players[q].alive || g.players[q].team != other {
                    continue;
                }
                for &o in &g.players[q].cities {
                    if g.map.plots[g.city_plot(o)].area != ac {
                        continue;
                    }
                    let d = g.plot_distance(g.cities[c].x, g.cities[c].y, g.cities[o].x, g.cities[o].y);
                    best = Some(best.map_or(d, |b: i32| b.min(d)));
                }
            }
        }
    }
    best
}

pub fn manage(g: &mut Game, v: &View) {
    let p = v.p;
    if v.barbarian || !is_team_leader(g, p) {
        return;
    }
    let team = v.team;
    let turn = g.game_turn();

    let plan = crate::victory::plan(g, p); // victory-pursuit
    // Peace when the war goes badly or drags on without gains -- or, when we are clearly winning and
    // the war has run its course (>10 turns), a peace that converts our edge into reparations (tech +
    // gold) instead of a valueless white peace (C++ `AI_doDiplo` peace build). A winner pursuing
    // conquest/domination against this enemy, or winning a total war (press_conquest), keeps fighting
    // to take cities rather than cashing out.
    for &enemy in &v.civ_enemies {
        let Some(other) = team_leader(g, enemy).filter(|&q| !g.players[q].human) else { continue };
        let our_want = end_war_val_seen(g, p, team, enemy); // iOurValue: how much WE want peace
        let their_want = end_war_val_seen(g, p, enemy, team); // iTheirValue: how much THEY want peace
        let victory_war = crate::victory::wants_conquest(&plan) || crate::victory::rival_team(g, &plan) == Some(enemy);
        // Don't sell a conquest war we are winning: press on to take cities instead (see press_conquest).
        let winner_initiated = their_want > our_want && g.teams[team].ai_at_war_counter[enemy] > 10 && !victory_war && !press_conquest(g, team, enemy);
        let wanted = want_peace(g, team, enemy, &plan);
        // A war where nothing happens ends (MNAI): neither side has an army at the other's cities.
        let quiet = !wanted && crate::switches::QUIET_PEACE.on(g, p) && quiet_war(g, team, enemy);
        if !wanted && !winner_initiated && !quiet {
            continue;
        }
        let item = TradeData { item_type: TradeableItem::TRADE_PEACE_TREATY, data: 0, offering: false, hidden: false };
        if !g.player_can_trade_item(p, other, &item, true) || !g.player_can_trade_item(other, p, &item, true) {
            continue;
        }
        let mut ours_list = vec![item.clone()];
        let mut theirs_list = vec![item];
        if their_want > our_want {
            // Winning: demand reparations from them (added to their side of the deal).
            let tribute = crate::trade::peace_tribute(g, p, other, their_want - our_want);
            // Fight on when the next city is close: a cash-out sells the war only when the tribute
            // pays for what the army at their gates is worth to us (the drop in our want for peace
            // that the forces at the front make; nothing when no army is near).
            if winner_initiated && !wanted && !quiet {
                let at_gates = end_war_val(g, team, enemy) - our_want;
                let worth = crate::trade::deal_val(g, p, other, &tribute, false, 1);
                if at_gates > 0 && worth < at_gates {
                    crate::trace!(g, p, "fights on against team {:?}: the next city is close (tribute {} < {})", enemy, worth, at_gates);
                    g.tel_decision(p, "peace", &format!("team {}", enemy.0), "fight_on", at_gates as i64, String::new);
                    continue;
                }
            }
            theirs_list.extend(tribute);
        } else if our_want > their_want {
            // Losing: sweeten the peace with tribute of our own so the winner will accept.
            ours_list.extend(crate::trade::peace_tribute(g, other, p, our_want - their_want));
        }
        let accept = g.controller(other).consider_offer(g, other, p, &ours_list, &theirs_list);
        if accept {
            crate::trace!(g, p, "peace with team {:?} after {} turns (we take {}, we give {})", enemy, g.teams[team].ai_at_war_counter[enemy], theirs_list.len() - 1, ours_list.len() - 1);
            g.tel_decision(p, "peace", &format!("team {}", enemy.0), if winner_initiated { "cash_out" } else if quiet { "quiet_war" } else { "want_peace" }, (their_want - our_want) as i64, String::new);
            g.implement_deal(p, other, &ours_list, &theirs_list, false);
        }
    }

    // Hopelessly behind, or crushed in a war that peace did not end: become a vassal.
    if crate::vassals::seek(g, p) {
        return;
    }

    // Trade with friendly neighbours every few turns (staggered): technologies swapped one for
    // one at similar cost, open borders.
    if (turn + p.0 as i32) % 5 == 0 {
        trade(g, p);
        // A weaker civ's check against a stronger, warlike neighbour (pacts need peace all round).
        if v.civ_enemies.is_empty() {
            crate::pacts::seek(g, p);
        }
    }

    // Gift a free tech to a markedly weaker, well-liked ally (C++ give-help). The ~150-turn contact
    // cadence keeps it rare, so run every turn rather than on the staggered trade schedule.
    for other in g.player_ids() {
        if other == p {
            continue;
        }
        let op = &g.players[other];
        if !op.alive || op.human || op.team == team || g.player_is_barbarian(other) || !is_team_leader(g, other) || !g.teams[team].has_met[op.team] {
            continue;
        }
        give_help(g, p, other, &plan);
    }

    // While at war, pay a third AI (tech + gold) to pile onto one of our enemies (C++ war bribe).
    if !v.civ_enemies.is_empty() {
        for bribee in g.player_ids() {
            if bribee == p {
                continue;
            }
            let bp = &g.players[bribee];
            if !bp.alive || bp.human || bp.team == team || g.player_is_barbarian(bribee) || !is_team_leader(g, bribee) || !g.teams[team].has_met[bp.team] {
                continue;
            }
            hire_war_ally(g, p, bribee, &plan);
        }
    }

    // One war at a time.
    if !v.civ_enemies.is_empty() {
        return;
    }

    // A prepared war: declare when the army is ready, or give up when the target is gone.
    if let Some(target) = preparing_target(g, team) {
        // Reachable by land (a shared landmass) or by sea (an overseas city we can ship an army to).
        let reachable = reach_distance(g, team, target).is_some()
            || team_leader(g, target).is_some_and(|q| crate::naval::amphibious_city(g, p, q).is_some());
        if !reachable || !g.team_can_declare_war(team, target) {
            g.team_ai_set_war_plan(team, target, None, true);
            return;
        }
        let ready = army_size(g, p) >= army_needed_for_war(g, p, target);
        let waited = g.teams[team].ai_war_plan_state_counter[target];
        // An expert declares with the army already at the border: the war's first turns are
        // spent fighting, not marching while the enemy mobilises (and the war's first ten turns,
        // before peace can be made, are not spent on the road).
        let staged = !crate::switches::WAR_STAGING.on(g, p) || staged_army(g, p, target) * 3 >= army_needed_for_war(g, p, target) * 2;
        if ready && staged && waited >= 5 || waited >= 40 {
            // The last chance to change our mind (MNAI `AI_doWar`): a target that no longer passes
            // the gate that chose it -- new pacts behind it, its cities no longer takeable, a
            // friendship since -- is dropped, the army kept for the next choice.
            if let Err(why) = if crate::switches::WAR_RECHECK.on(g, p) { still_a_target(g, p, target, &plan) } else { Ok(()) } {
                crate::trace!(g, p, "drops its war plan on team {:?}: it no longer passes the gate ({}, prepared {} turns)", target, why, waited);
                g.tel_decision(p, "war_drop", why, "recheck_dropped", waited as i64, String::new);
                g.team_ai_set_war_plan(team, target, None, true);
                return;
            }
            crate::trace!(g, p, "declares war on team {:?} (army {}, prepared {} turns)", target, army_size(g, p), waited);
            g.tel_war_reason(team, target, || format!("{} after {} turns: army {} of {}", if ready && staged && waited >= 5 { "army ready" } else { "waited long" }, waited, army_size(g, p), army_needed_for_war(g, p, target)));
            let plan = if g.teams[team].ai_war_plan[target] == Some(WarPlanType::WARPLAN_PREPARING_LIMITED) { WarPlanType::WARPLAN_LIMITED } else { WarPlanType::WARPLAN_TOTAL };
            g.team_declare_war(team, target, true, Some(plan));
        }
        return;
    }

    // Start preparing a war: rarely, against a weaker, disliked neighbour we can reach.
    if turn < 50 || (turn + p.0 as i32 * 3) % 5 != 0 || g.players[p].cities.len() < 2 {
        return;
    }
    // victory-pursuit: a war our victory plan calls for (a rival about to win, our domination
    // or conquest) needs no roll and no grudge.
    if let Some((target, why)) = victory_war(g, p, &plan) {
        crate::trace!(g, p, "prepares war on team {:?} for the victory plan ({})", target, why);
        g.tel_war_reason(team, target, || format!("victory plan ({why})"));
        g.tel_decision(p, "war_prep", &format!("team {}", target.0), "victory_plan", 0, String::new);
        g.team_ai_set_war_plan(team, target, Some(WarPlanType::WARPLAN_PREPARING_TOTAL), false);
        return;
    }
    // Greedy expansion by war: prepare war on the reachable rival whose territory we can take at
    // acceptable risk, for the best gain -- no dice roll and no grudge requirement. A civ uses the
    // military power it has to take land whenever the opportunity is good (domination and conquest
    // are this greedy rule run to its conclusion; the victory planner no longer drives wars).
    // Flavour biases the bar, not the drive: a war-eager leader (low C++ iMaxWarRand) strikes at
    // near parity and will hit anyone, a cautious one needs a clear edge and spares civs it likes.
    let broke = crate::production::financial_trouble(g, p);
    if broke && g.player_calculate_gold_rate(p) < 0 && crate::economy::spend_pct(g, p) == 0 {
        crate::trace!(g, p, "would expand by war but is bankrupt");
        return;
    }
    let bar = war_bar(g, p);
    let mut best: Option<(TeamId, Target)> = None;
    // Telemetry: every target that passed the gates, with its score.
    let mut scored: Vec<(TeamId, i64)> = Vec::new();
    for other in g.player_ids() {
        let Some(t) = territory_target(g, p, &bar, other, false) else { continue };
        let ot = g.players[other].team;
        scored.push((ot, t.score));
        if best.as_ref().map_or(true, |(_, b)| t.score > b.score) {
            best = Some((ot, t));
        }
    }
    if let Some((target, t)) = best {
        let plan = if t.ratio >= TOTAL_WAR_RATIO { WarPlanType::WARPLAN_PREPARING_TOTAL } else { WarPlanType::WARPLAN_PREPARING_LIMITED };
        let why = if t.allies > 0 { "dogpile" } else { "territory" };
        crate::trace!(g, p, "prepares war on team {:?} to take territory ({:?}, power ratio {}, its enemies' power {})", target, plan, t.ratio, t.allies);
        if g.telemetry.on() {
            let (ratio, allies, need_ratio) = (t.ratio, t.allies, bar.need_ratio);
            g.tel_war_reason(team, target, || {
                if allies > 0 {
                    format!("dogpile (power ratio {ratio} with its enemies' {allies}, needed {need_ratio})")
                } else {
                    format!("territory (power ratio {ratio}, needed {need_ratio})")
                }
            });
            let names: Vec<(String, i64)> = scored.iter().map(|&(t, s)| (format!("team {}", t.0), s)).collect();
            g.tel_decision(p, "war_prep", &format!("team {}", target.0), why, t.score, || ffh_core::telemetry::tel_alts(names.iter().map(|(n, s)| (n.as_str(), *s))));
        }
        g.team_ai_set_war_plan(team, target, Some(plan), false);
    }
}

/// The power edge player `p`'s leader needs to start a war of expansion, and whether it spares
/// civs it likes. iMaxWarRand is the leader's propensity to start total war -- LOW = eager. It maps
/// to the power edge we require over the target's defence: eager leaders accept near parity (even a
/// slight underdog, acceptable risk for a warmonger), cautious ones want clear superiority. A broke
/// empire wants a bigger edge (a captured city has to pay for itself in plunder and land).
pub(crate) struct WarBar {
    pub need_ratio: i64,
    spare_friends: bool,
    ours: i64,
}

pub(crate) fn war_bar(g: &Game, p: PlayerId) -> WarBar {
    let leader = g.players[p].personality_type.map(|l| &g.d.leader_heads[l]);
    let broke = crate::production::financial_trouble(g, p);
    let war_rand = leader.map_or(150, |l| l.max_war_rand);
    WarBar {
        need_ratio: (80 + war_rand as i64 / 4).clamp(85, 180) + if broke { 30 } else { 0 },
        spare_friends: war_rand > 80,
        ours: g.team_get_power(g.players[p].team, true).max(1) as i64,
    }
}

/// A war of expansion against one civ, as [`territory_target`] scores it.
pub(crate) struct Target {
    pub score: i64,
    /// Our power (with `allies`) per 100 of their defensive power.
    pub ratio: i64,
    /// The power of their current enemies counted with ours (dogpile).
    pub allies: i64,
}

/// The power of `target`'s current enemies other than `team` (MNAI `AI_doWar`'s dogpile power):
/// a civ at war is a weaker target than its defensive power says, its army busy on another front.
/// An enemy that does not share a landmass with it counts half (its war is fought over water).
pub(crate) fn dogpile_power(g: &Game, team: TeamId, target: TeamId) -> i64 {
    let mut sum = 0;
    for t in g.team_ids() {
        if t == team || t == target || !g.team_is_alive(t) || g.team_is_barbarian(t) || !g.at_war(t, target) {
            continue;
        }
        let power = g.team_get_power(t, false) as i64;
        sum += if reach_distance(g, t, target).is_some() { power } else { power / 2 };
    }
    sum
}

/// The war-of-expansion gate against `other`'s team (see `manage`): met, can be declared on, at
/// peace for a while, not a friend a cautious leader spares, our power edge (with the power of its
/// current enemies under `DOGPILE`) over its defensive power at the leader's bar, and a city we can
/// reach and take. `recheck`: the same test just before the declaration, with 10% slack on the bar
/// (a prepared army is not given up for a small swing).
pub(crate) fn territory_target(g: &Game, p: PlayerId, bar: &WarBar, other: PlayerId, recheck: bool) -> Option<Target> {
    territory_check(g, p, bar, other, recheck).ok()
}

/// [`territory_target`], or the test the target fails.
fn territory_check(g: &Game, p: PlayerId, bar: &WarBar, other: PlayerId, recheck: bool) -> Result<Target, &'static str> {
    let team = g.players[p].team;
    let op = &g.players[other];
    if !op.alive || op.team == team || g.player_is_barbarian(other) || op.cities.is_empty() {
        return Err("gone");
    }
    let ot = op.team;
    if !g.teams[team].has_met[ot] || !g.team_can_declare_war(team, ot) {
        return Err("cannot_declare");
    }
    // Peace treaties hold for a while.
    if g.teams[team].ai_at_peace_counter[ot] < 10 && g.teams[team].ai_has_met_counter[ot] > 10 {
        return Err("treaty");
    }
    let rel = relation(g, p, other);
    // A cautious leader does not attack a civ it likes; a war-eager one takes any opening.
    if bar.spare_friends && rel > 1 {
        return Err("friend");
    }
    let theirs = g.team_get_defensive_power(ot).max(1) as i64;
    let allies = if crate::switches::DOGPILE.on(g, p) { dogpile_power(g, team, ot) } else { 0 };
    let ratio = (bar.ours + allies) * 100 / theirs;
    if ratio < if recheck { bar.need_ratio * 9 / 10 } else { bar.need_ratio } {
        return Err("power");
    }
    // Reachable and takeable: by land (a shared landmass within 16 plots whose weakest city
    // our best trainable attacker can take), or, failing that, by sea (an overseas city we
    // can ship an army to and take -- `amphibious_city` already checks its takeability). No
    // hardcoded unit or stat; reachability from the flood/distance helpers.
    // At the re-check the army is massed at the border: our best attacker need only match their
    // weakest city's best defender (the 120% fortification margin selects wars, see AI_LOG "Wars").
    let land = reach_distance(g, team, ot).filter(|&d| d <= 16 && can_take_cities_at(g, p, other, if recheck { 100 } else { 120 }));
    let dist = match land {
        Some(d) => d,
        None => match crate::naval::amphibious_city(g, p, other) {
            Some(a) => a.dist,
            None if reach_distance(g, team, ot).is_some_and(|d| d <= 16) => return Err("not_takeable"),
            None => return Err("unreachable"),
        },
    };
    // Flavor bias (secondary): prefer striking a rival of the opposed alignment -- the
    // good/evil conflict at the heart of FFH. Added only to the score, so it re-ranks targets
    // we can already reach and take (the reachability/takeability gate above has run); it never
    // makes us attack an unreachable civ, so it cannot trade away the greedy territory-grab.
    let flavor = {
        let good = g.d.ty::<AlignmentType>("ALIGNMENT_GOOD");
        let evil = g.d.ty::<AlignmentType>("ALIGNMENT_EVIL");
        let (us, them) = (g.players[p].alignment, op.alignment);
        if (us == good && them == evil) || (us == evil && them == good) { 30 } else { 0 }
    };
    // Greedy gain: a bigger edge, more and closer cities; minus how much we like them; a
    // flavor nudge toward alignment-opposites.
    let score = ratio - dist as i64 * 6 - rel as i64 * 15 + op.cities.len() as i64 * 10 + flavor;
    Ok(Target { score, ratio, allies })
}

/// Does team `target` still pass a gate that would choose it now (`WAR_RECHECK`): the victory
/// plan's (a rival about to win; our domination or conquest) or the war of expansion's, with slack (a first version kept takeability's 120% margin and dropped half the wars: docs/AI_LOG.md, "Round 7c").
/// `Err` names the test the war of expansion's gate fails (for traces and telemetry).
fn still_a_target(g: &Game, p: PlayerId, target: TeamId, plan: &crate::victory::Plan) -> Result<(), &'static str> {
    let bar = war_bar(g, p);
    let mut why = "gone";
    for q in g.player_ids().into_iter().filter(|&q| g.players[q].alive && g.players[q].team == target) {
        if (plan.rival.is_some_and(|(r, _)| r == q) && victory_target(g, p, q, 150).is_some()) || (crate::victory::wants_conquest(plan) && victory_target(g, p, q, 100).is_some()) {
            return Ok(());
        }
        match territory_check(g, p, &bar, q, true) {
            Ok(_) => return Ok(()),
            Err(e) => why = e,
        }
    }
    Err(why)
}

fn tech_item(t: TechType) -> TradeData {
    TradeData { item_type: TradeableItem::TRADE_TECHNOLOGIES, data: t.i(), offering: false, hidden: false }
}

/// C++ `AI_doDiplo` give-help (FfH "Better Diplomatics" block B, CvPlayerAI.cpp:13105): the team
/// leader `p` gifts a free tech to a markedly weaker civ it likes, to keep an ally viable. The
/// recipient's side of the deal is empty (a free gift), implemented directly -- the base game does
/// not consult the recipient. Deliberate deviations from the C++, for ffh-ai's conventions and the
/// project's "do not prop up a rival" rule: the gift tech is the CHEAPEST one we can give (determin-
/// istic -- ffh-ai draws no RNG -- and this avoids handing over an expensive key enabler) rather than
/// the C++ random pick; and we never gift a civ that is our victory rival. Giving does not raise
/// attitude: `relation()` does not track gifts (C++ `AI_getTradeAttitude` unported), as with
/// resource/open-borders trading. The per-turn contact-timer + 1/iContactRand roll is replaced by the
/// deterministic `offers::contact_due` cadence (period = contact_delay + contact_rand, ~150 turns).
fn give_help(g: &mut Game, p: PlayerId, other: PlayerId, plan: &crate::victory::Plan) {
    let (team, ot) = (g.players[p].team, g.players[other].team);
    // Markedly weaker (team assets < half ours), at peace, and not a civ contesting our victory.
    if g.team_get_assets(ot) >= g.team_get_assets(team) / 2 || g.at_war(team, ot) || crate::victory::rival_team(g, plan) == Some(ot) {
        return;
    }
    // Attitude + FfH alignment / favourite-religion sweetener vs the TARGET leader's NoGiveHelp bar.
    let i_mod = i32::from(g.players[other].alignment == g.players[p].alignment)
        + i32::from(g.players[other].favorite_religion == g.players[p].favorite_religion);
    let thr = g.players[other].personality_type.map_or(-1, |l| opt_to_i32(g.d.leader_heads[l].no_give_help_attitude_threshold));
    if attitude(g, p, other).i() + i_mod <= thr || !crate::offers::contact_due(g, p, other, "CONTACT_GIVE_HELP") {
        return;
    }
    // The cheapest tech they lack and we can give (deterministic; C++ picks one at random).
    let give = g
        .d
        .techs
        .keys()
        .filter(|&t| g.player_can_trade_item(p, other, &tech_item(t), true))
        .min_by_key(|&t| (g.team_get_research_cost(team, t), t));
    let Some(t) = give else { return };
    crate::trace!(g, p, "gifts {:?} to weaker ally {:?}", t, other);
    let ours = vec![tech_item(t)];
    g.implement_deal(p, other, &ours, &[], false);
}

/// C++ `AI_doDiplo` war-bribe (CvPlayerAI.cpp:13986): while at war, pay a third AI (`bribee`, at
/// peace with us) tech + gold to declare war on one of our enemies. Deterministic (ffh-ai draws no
/// RNG): the target is the enemy the bribee is most hostile to (`AI_getAttitudeWeight`), the payment
/// is our costliest giveable tech(s) plus a gold top-up; the C++ random target/tech picks and the
/// 1/`DeclareWarTradeRand` roll are replaced by value-ranked picks and a deterministic staggered
/// cadence. The bribee's `consider_offer` is the final judge (its `AI_declareWarTrade` gates + value
/// test), so an underpriced or refused offer does nothing. We never pay our victory rival (project
/// "do not prop up a rival" rule). Argument order for the price follows the C++ offer path
/// (this = briber, eWarTeam = target, eTeam = bribee); the bribee re-derives the same value in its
/// `consider_offer`.
fn hire_war_ally(g: &mut Game, p: PlayerId, bribee: PlayerId, plan: &crate::victory::Plan) {
    let (team, bribee_team) = (g.players[p].team, g.players[bribee].team);
    if g.at_war(team, bribee_team) || crate::victory::rival_team(g, plan) == Some(bribee_team) {
        return;
    }
    // Deterministic cadence (replaces the getDeclareWarTradeRand roll); staggered by the pair.
    let period = g.players[p].personality_type.map_or(1, |l| g.d.leader_heads[l].declare_war_trade_rand).max(1);
    if (g.game_turn() + 7 * p.0 as i32 + 3 * bribee.0 as i32).rem_euclid(period) != 0 {
        return;
    }
    // Target: an enemy of ours the bribee is not at war with, not already dogpiled, that the bribee
    // can be bribed to attack. Pick the one the bribee is most hostile to (C++ picks by weight x rand).
    let cap = 2.max(g.team_ids().filter(|&t| g.team_is_alive(t) && !g.team_is_barbarian(t)).count() as i32 / 2);
    let mut best: Option<(TeamId, i64)> = None;
    for target in g.team_ids() {
        if !g.team_is_alive(target) || !g.at_war(team, target) || g.at_war(bribee_team, target) || g.team_get_at_war_count(target, true) >= cap {
            continue;
        }
        let item = TradeData { item_type: TradeableItem::TRADE_WAR, data: target.0 as i32, offering: false, hidden: false };
        if !g.player_can_trade_item(bribee, p, &item, true) {
            continue;
        }
        let w = 101 + attitude_weight(team_attitude(g, target, team));
        if best.map_or(true, |(_, bw)| w > bw) {
            best = Some((target, w));
        }
    }
    let Some((target, _)) = best else { return };
    let our_val = declare_war_trade_val(g, team, target, bribee_team) as i64;
    // Payment: the costliest tech(s) the bribee would value that we can give, then a gold top-up.
    let pct = crate::trade::gold_trade_value_percent(g, p).max(1) as i64;
    let mut ours: Vec<TradeData> = Vec::new();
    let mut given: Vec<TechType> = Vec::new();
    let mut their_val: i64 = 0;
    for _ in 0..2 {
        if their_val >= our_val {
            break;
        }
        let give = g
            .d
            .techs
            .keys()
            .filter(|&t| !given.contains(&t) && g.player_can_trade_item(p, bribee, &tech_item(t), true))
            .max_by_key(|&t| (g.team_get_research_cost(bribee_team, t), std::cmp::Reverse(t)));
        let Some(t) = give else { break };
        their_val += g.team_get_research_cost(bribee_team, t) as i64;
        given.push(t);
        ours.push(tech_item(t));
    }
    if their_val < our_val {
        let gold = (((our_val - their_val) * 100 / pct) as i32).min(crate::trade::max_gold_trade(g, p, bribee));
        if gold > 0 {
            their_val += gold as i64 * pct / 100;
            ours.push(crate::trade::gold_item(gold));
        }
    }
    // Offer only if the payment is worth more than three quarters of our war-valuation (C++ 14177).
    if their_val <= our_val * 3 / 4 || ours.is_empty() {
        return;
    }
    let theirs = vec![TradeData { item_type: TradeableItem::TRADE_WAR, data: target.0 as i32, offering: false, hidden: false }];
    crate::trace!(g, p, "bribes {:?} to war team {:?} (pay {} items, price {})", bribee, target, ours.len(), our_val);
    if g.controller(bribee).consider_offer(g, bribee, p, &ours, &theirs) {
        g.implement_deal(p, bribee, &ours, &theirs, false);
    }
}

/// The team holding a clear lead in the world: a share of the world (score) at least 1.5 times the
/// runner-up's, as the victory planner reads a lead (`victory::active`). What a human sees on the
/// scoreboard.
pub(crate) fn clear_leader(g: &Game) -> Option<TeamId> {
    let scores = g.world_scores();
    let barb = g.barbarian_team();
    let mut teams: Vec<(TeamId, i64)> = Vec::new();
    for q in g.civ_player_ids() {
        let pl = &g.players[q];
        if !pl.alive || pl.team == barb {
            continue;
        }
        match teams.iter_mut().find(|(t, _)| *t == pl.team) {
            Some(e) => e.1 += scores[q.idx()] as i64,
            None => teams.push((pl.team, scores[q.idx()] as i64)),
        }
    }
    teams.sort_by_key(|&(t, s)| (std::cmp::Reverse(s), t));
    match teams.as_slice() {
        [(t, a), (_, b), ..] if a * 2 >= b * 3 => Some(*t),
        _ => None,
    }
}

/// What player `p` reads before giving away technologies (`TECH_DISCIPLINE`), once per round.
pub(crate) struct TechCtx {
    on: bool,
    /// Our victory rival: it gets no technology from us.
    rival: Option<TeamId>,
    /// The world's clear leader (not us): it gets a swap only when we come out ahead.
    leader: Option<TeamId>,
    /// We prepare or fight a war with a civ: no military technology leaves us.
    war: bool,
}

pub(crate) fn tech_ctx(g: &Game, p: PlayerId) -> TechCtx {
    if !crate::switches::TECH_DISCIPLINE.on(g, p) {
        return TechCtx { on: false, rival: None, leader: None, war: false };
    }
    let team = g.players[p].team;
    let plan = crate::victory::plan(g, p);
    let rival = crate::victory::rival_team(g, &plan);
    let leader = clear_leader(g).filter(|&t| t != team);
    let war = preparing_target(g, team).is_some() || g.team_ids().into_iter().any(|t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t));
    TechCtx { on: true, rival, leader, war }
}

impl TechCtx {
    /// Do we trade technologies with `to` at all (not with our victory rival)?
    pub(crate) fn partner_ok(&self, g: &Game, to: PlayerId) -> bool {
        !self.on || self.rival != Some(g.players[to].team)
    }

    /// The least we take in a swap with `to`, in percent of what we give (by research cost):
    /// four fifths, as `consider_offer` always did, but five fourths from the world's clear leader
    /// -- we trade with the runaway only when we gain more than it does.
    pub(crate) fn min_return_pct(&self, g: &Game, to: PlayerId) -> i64 {
        if self.on && self.leader == Some(g.players[to].team) {
            125
        } else {
            80
        }
    }
}

/// Does `tech` enable a unit of `to`'s civilization able to fight at 60% of the best land unit's
/// strength in the world (a weapon worth keeping from someone we may fight)?
fn is_military_tech(g: &Game, to: PlayerId, tech: TechType) -> bool {
    let d = g.d;
    let Some(civ) = g.player_get_civilization_type(to) else { return false };
    let best = g.best_land_unit_combat().max(1);
    d.unit_classes.keys().filter_map(|uc| d.civ_unit(civ, uc)).any(|u| {
        let info = &d.units[u];
        (info.prereq_and_tech == Some(tech) || info.prereq_and_techs.contains(&tech))
            && info.combat > 0
            && !info.found
            && info.combat * 100 >= best * 60
            && matches!(info.domain_type, Some(DomainType::DOMAIN_LAND) | Some(DomainType::DOMAIN_SEA))
    })
}

/// Is a city of team `team` building a world wonder, a world unit or a world project that needs
/// `tech` (MNAI `AI_techTrade`'s DENIAL_MYSTERY: the race we are running)?
fn racing_with(g: &Game, team: TeamId, tech: TechType) -> bool {
    let d = g.d;
    let needs = |and: Option<TechType>, ands: &[TechType]| and == Some(tech) || ands.contains(&tech);
    g.player_ids().into_iter().filter(|&q| g.players[q].alive && g.players[q].team == team).any(|q| {
        g.players[q].cities.iter().any(|&c| {
            if let Some(b) = g.city_production_building(c) {
                let info = &d.buildings[b];
                let ands: Vec<TechType> = info.prereq_and_techs.iter().flatten().copied().collect();
                if info.building_class_type.is_some_and(|bc| ffh_core::utils::is_world_wonder_class(d, bc)) && needs(info.prereq_and_tech, &ands) {
                    return true;
                }
            }
            if let Some(u) = g.city_production_unit(c) {
                let info = &d.units[u];
                if info.unit_class_type.is_some_and(|uc| ffh_core::utils::is_world_unit_class(d, uc)) && needs(info.prereq_and_tech, &info.prereq_and_techs) {
                    return true;
                }
            }
            g.city_production_project(c).is_some_and(|pj| d.projects[pj].tech_prereq == Some(tech) && ffh_core::utils::is_world_project(d, pj))
        })
    })
}

/// Why player `p` would not give `tech` to `to` (`TECH_DISCIPLINE`; MNAI `CvTeamAI::AI_techTrade`),
/// or `None`: `to` is our victory rival; a military technology while we prepare or fight a war
/// (any recipient could pass it on); and, to anyone we are not friendly with, a monopoly worth
/// keeping -- the technology of a world wonder, world unit or project we are building, or a
/// military technology a land neighbour we may fight would gain while most civs we know lack it. (A first version, keeping every valuable technology and refusing the leader outright, cost the civ using it: docs/AI_LOG.md, "Round 7c".)
pub(crate) fn tech_denial(g: &Game, p: PlayerId, to: PlayerId, tech: TechType, ctx: &TechCtx) -> Option<&'static str> {
    if !ctx.on {
        return None;
    }
    let (team, ot) = (g.players[p].team, g.players[to].team);
    if ctx.rival == Some(ot) {
        return Some("rival");
    }
    if ctx.war && is_military_tech(g, to, tech) {
        return Some("military");
    }
    if attitude(g, p, to).i() < AttitudeType::ATTITUDE_FRIENDLY.i() {
        if racing_with(g, team, tech) {
            return Some("wonder");
        }
        if reach_distance(g, team, ot).is_some_and(|d| d <= 16) && is_military_tech(g, to, tech) {
            let (mut known, mut possible) = (0i64, 0i64);
            for t in g.team_ids() {
                if t == team || t == ot || !g.team_is_alive(t) || g.team_is_barbarian(t) || !g.teams[team].has_met[t] {
                    continue;
                }
                possible += 1;
                known += i64::from(g.team_is_has_tech(t, Some(tech)));
            }
            // With no third civ met there is no edge to keep over the others.
            if possible > 0 && known * 2 < possible {
                return Some("monopoly");
            }
        }
    }
    None
}

/// Tech swaps and open borders with the AI leaders of teams we know and like well enough.
fn trade(g: &mut Game, p: PlayerId) {
    let team = g.players[p].team;
    let tech_rules = tech_ctx(g, p);
    for other in g.player_ids() {
        let op = &g.players[other];
        if other == p || !op.alive || op.human || op.team == team || g.player_is_barbarian(other) || !is_team_leader(g, other) {
            continue;
        }
        let ot = op.team;
        if !g.teams[team].has_met[ot] || g.at_war(team, ot) || relation(g, p, other) < 0 {
            continue;
        }
        // Open borders (trade routes, passage).
        if !g.team_is_open_borders(team, ot) && relation(g, p, other) >= 1 {
            let item = TradeData { item_type: TradeableItem::TRADE_OPEN_BORDERS, data: 0, offering: false, hidden: false };
            if g.player_can_trade_item(p, other, &item, true) && g.player_can_trade_item(other, p, &item, true) {
                let ours = vec![item.clone()];
                let theirs = vec![item];
                if g.controller(other).consider_offer(g, other, p, &ours, &theirs) {
                    crate::trace!(g, p, "open borders with {:?}", other);
                    g.implement_deal(p, other, &ours, &theirs, false);
                }
            }
        }
        // A resource of theirs we want, paid for with a surplus of ours or with gold.
        resource_trade(g, p, other);
        // No technology swaps with our victory rival or a clear world leader (TECH_DISCIPLINE).
        if !tech_rules.partner_ok(g, other) {
            g.tel_decision(p, "tech_trade", "partner_blocked", "rival", 0, String::new);
            continue;
        }
        // What they would hand over (the trade screen shows a refusal): their own discipline.
        let their_rules = tech_ctx(g, other);
        if !their_rules.partner_ok(g, p) {
            continue;
        }
        // The tech of theirs we value most, for the tech of ours closest in cost.
        let cost = |t: TechType| g.team_get_research_cost(team, t).max(1);
        let mut want: Option<(TechType, i32)> = None;
        for t in g.d.techs.keys() {
            if g.team_is_has_tech(team, Some(t)) || !g.player_can_research(p, t, false) || !g.player_can_trade_item(other, p, &tech_item(t), true) {
                continue;
            }
            if tech_denial(g, other, p, t, &their_rules).is_some() {
                continue;
            }
            let c = cost(t);
            if want.map_or(true, |(_, b)| c > b) {
                want = Some((t, c));
            }
        }
        let Some((get, get_cost)) = want else { continue };
        let mut give: Option<(TechType, i32)> = None;
        // The swap we would have offered without the discipline (telemetry: what it withheld).
        let mut free: Option<(TechType, i32)> = None;
        for t in g.d.techs.keys() {
            if !g.team_is_has_tech(team, Some(t)) || g.team_is_has_tech(ot, Some(t)) || !g.player_can_trade_item(p, other, &tech_item(t), true) {
                continue;
            }
            let diff = (cost(t) - get_cost).abs();
            if free.map_or(true, |(_, b)| diff < b) {
                free = Some((t, diff));
            }
            if tech_denial(g, p, other, t, &tech_rules).is_some() {
                continue;
            }
            if give.map_or(true, |(_, b)| diff < b) {
                give = Some((t, diff));
            }
        }
        if let Some((t, _)) = free.filter(|&(t, _)| give.map(|x| x.0) != Some(t)) {
            if let Some(why) = tech_denial(g, p, other, t, &tech_rules) {
                g.tel_decision(p, "tech_trade", "withheld", why, 0, String::new);
            }
        }
        let Some((give, _)) = give else { continue };
        // With the world's clear leader only a swap that favours us (TECH_DISCIPLINE).
        if (cost(give) as i64) * tech_rules.min_return_pct(g, other) > get_cost as i64 * 100 {
            g.tel_decision(p, "tech_trade", "withheld", "leader", 0, String::new);
            continue;
        }
        let ours = vec![tech_item(give)];
        let theirs = vec![tech_item(get)];
        if g.controller(other).consider_offer(g, other, p, &ours, &theirs) {
            crate::trace!(g, p, "trades {} for {} with {:?}", g.d.techs[give].base.type_, g.d.techs[get].base.type_, other);
            g.implement_deal(p, other, &ours, &theirs, false);
        }
    }
}

/// Offer `other` a resource trade ([`trade::buy_resource_offer`][crate::trade::buy_resource_offer]
/// builds the deal); the AI leader `other` judges it through its own `consider_offer`.
fn resource_trade(g: &mut Game, p: PlayerId, other: PlayerId) {
    let Some((ours, theirs)) = crate::trade::buy_resource_offer(g, p, other) else { return };
    if g.controller(other).consider_offer(g, other, p, &ours, &theirs) {
        let got = BonusType::from_usize(theirs[0].data as usize);
        crate::trace!(g, p, "buys {} from {:?}", g.d.bonuses[got].base.type_, other);
        g.implement_deal(p, other, &ours, &theirs, false);
    }
}

/// Our best trainable land attacker's effective city-attack strength (x100), and whether we
/// can train a siege engine. `None` when we have no capital to train from. No hardcoded unit:
/// every land attacker the capital can train is weighed (combat + damage types, city-attack
/// bonus), siege engines noted for the bombard discount.
pub(crate) fn best_trainable_attack(g: &Game, p: PlayerId) -> Option<(i32, bool)> {
    let d = g.d;
    let c = g.players[p].capital_city.filter(|&c| g.cities.contains(c))?;
    let mut best_attack = 0;
    let mut siege = false;
    for u in d.units.keys() {
        let info = &d.units[u];
        if info.domain_type != Some(DomainType::DOMAIN_LAND) || info.combat <= 0 || info.only_defensive {
            continue;
        }
        if !g.city_offers_unit(c, u) || !g.city_can_train(c, u, false, false, false, false) {
            continue;
        }
        best_attack = best_attack.max((info.combat + info.damage_type_combat.iter().sum::<i32>()) * (100 + info.city_attack_modifier.max(0)));
        siege |= info.bombard_rate > 0;
    }
    Some((best_attack, siege))
}

/// The strength (x100) of city `c`'s best defender as we see it (fortified city bonuses), or
/// `None` when the city plot is not revealed to our team.
fn city_best_defense(g: &Game, p: PlayerId, c: CityId) -> Option<i32> {
    let team = g.players[p].team;
    let cp = g.city_plot(c);
    if !g.map.plots[cp].revealed[team] {
        return None;
    }
    let other = g.cities[c].owner;
    let mut best = 0;
    for &e in &g.map.plots[cp].units {
        if g.units[e].owner != other {
            continue;
        }
        let info = &g.d.units[g.units[e].unit_type];
        if info.combat <= 0 {
            continue;
        }
        let def = if info.combat_defense > 0 { info.combat_defense } else { info.combat } + info.damage_type_combat.iter().sum::<i32>();
        best = best.max(def * (100 + info.city_defense_modifier.max(0)));
    }
    Some(best)
}

/// Can our best trainable attacker realistically take city `c`: its best defender (fortified in
/// the city, with city bonuses), siege engines lowering the bar. False when we cannot train an
/// attacker or the city plot is not revealed.
pub(crate) fn can_take_city(g: &Game, p: PlayerId, c: CityId) -> bool {
    let Some((best_attack, siege)) = best_trainable_attack(g, p) else { return false };
    let Some(best_defense) = city_best_defense(g, p, c) else { return false };
    // Fortified in a city: roughly +50%; siege engines take most of that away.
    let needed = best_defense * if siege { 85 } else { 120 } / 100;
    best_attack >= needed
}

/// Can our units realistically take `other`'s cities: our best trainable attacker against
/// their weakest revealed city's best defender (fortified, with city bonuses), or siege
/// engines to wear the walls down.
fn can_take_cities(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    can_take_cities_at(g, p, other, 120)
}

/// [`can_take_cities`] with `margin`% instead of the fortification's 120% (without siege engines).
fn can_take_cities_at(g: &Game, p: PlayerId, other: PlayerId, margin: i32) -> bool {
    let Some((best_attack, siege)) = best_trainable_attack(g, p) else { return false };
    // Their weakest city (the one we would attack first): its best defender.
    let mut weakest: Option<i32> = None;
    for &oc in &g.players[other].cities {
        if let Some(best) = city_best_defense(g, p, oc) {
            weakest = Some(weakest.map_or(best, |w: i32| w.min(best)));
        }
    }
    let Some(best_defense) = weakest else { return false };
    // Fortified in a city: roughly +50%; siege engines take most of that away.
    let needed = best_defense * if siege { 85 } else { margin } / 100;
    best_attack >= needed
}

/// The war our victory plan calls for (victory-pursuit): against the rival about to win, else,
/// when our goal is domination or conquest, against the weakest civ we can reach -- any civ we
/// can reach over land within 20 plots whose weakest city we can take, whose defensive power
/// is below ours (the rival's: below 1.5x ours). Returns the team and why.
/// The victory plan's war gate against `other` (see [`victory_war`]): its defensive power at most
/// `ratio_pct`% of our power, reachable over land within 20 plots, a city we can take. The score.
fn victory_target(g: &Game, p: PlayerId, other: PlayerId, ratio_pct: i64) -> Option<i64> {
    let team = g.players[p].team;
    let ours = g.team_get_power(team, true).max(1) as i64;
    let ot = g.players[other].team;
    if !g.players[other].alive || ot == team || g.player_is_barbarian(other) || g.players[other].cities.is_empty() {
        return None;
    }
    if !g.teams[team].has_met[ot] || !g.team_can_declare_war(team, ot) {
        return None;
    }
    if g.teams[team].ai_at_peace_counter[ot] < 10 && g.teams[team].ai_has_met_counter[ot] > 10 {
        return None;
    }
    let theirs = g.team_get_defensive_power(ot).max(1) as i64;
    if theirs * 100 > ours * ratio_pct {
        return None;
    }
    let dist = reach_distance(g, team, ot)?;
    if dist > 20 || !can_take_cities(g, p, other) {
        return None;
    }
    Some(ours * 100 / theirs - dist as i64 * 6)
}

fn victory_war(g: &Game, p: PlayerId, plan: &crate::victory::Plan) -> Option<(TeamId, &'static str)> {
    let broke = crate::production::financial_trouble(g, p);
    if broke && g.player_calculate_gold_rate(p) < 0 {
        return None;
    }
    let ok = |other: PlayerId, ratio_pct: i64| victory_target(g, p, other, ratio_pct);
    if let Some((q, _)) = plan.rival {
        if ok(q, 150).is_some() {
            return Some((g.players[q].team, "a rival close to victory"));
        }
    }
    if !crate::victory::wants_conquest(plan) {
        return None;
    }
    let mut best: Option<(TeamId, i64)> = None;
    for other in g.player_ids() {
        if let Some(score) = ok(other, 100) {
            let ot = g.players[other].team;
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((ot, score));
            }
        }
    }
    best.map(|(t, _)| (t, "domination or conquest"))
}

/// Power ratio (ours * 100 / their defensive power) at or above which a winner should press a war to
/// taking cities (conquest) rather than cash out for a token tribute -- an *overwhelming* edge (2x),
/// well above the `TOTAL_WAR_RATIO` = 140 bar for merely committing to a war. Below it (a 1-2x edge)
/// the enemy's cities are not reliably stormable, so a tribute peace is a sensible outcome and this
/// does not fire; at or above it, conquest is achievable and selling it for a tech forfeits the
/// primary goal. Tunable by A/B (the siege/launch bar is deliberately lower).
pub(crate) const PRESS_CONQUEST_RATIO: i64 = 200;

/// Are we overwhelmingly winning this war and should press it to taking cities rather than cashing
/// out for tribute? True when our power is at least `PRESS_CONQUEST_RATIO`% of the enemy's defensive
/// power -- a civ this far ahead can take cities, and conquest (the primary goal) must not be sold
/// for a secondary tech. Without this a strong civ tribute-farms a weak victim for the whole game
/// instead of finishing it; the plain victory-plan guard misses it because it keys on the civ's
/// overall victory goal, not on this war's power edge. The war plan is *not* used: an AI's active war
/// plan is overwritten to LIMITED/DOGPILE on declaration (never TOTAL), and the cash-out fires at the
/// first legal turn before war success accrues, so only the immediate power ratio is reliable. Both
/// the offer path (`manage`) and the acceptance path (`consider_offer`) consult this so a winner
/// neither offers nor accepts a peace that abandons a conquest it is winning.
pub(crate) fn press_conquest(g: &Game, team: TeamId, enemy: TeamId) -> bool {
    let ours = g.team_get_power(team, true).max(1) as i64;
    let theirs = g.team_get_defensive_power(enemy).max(1) as i64;
    ours * 100 >= theirs * PRESS_CONQUEST_RATIO
}

/// Team-level attitude as the leaders' attitude (ffh-ai has no team-averaged `AI_getAttitude`).
fn team_attitude(g: &Game, t: TeamId, towards: TeamId) -> AttitudeType {
    match (team_leader(g, t), team_leader(g, towards)) {
        (Some(a), Some(b)) => attitude(g, a, b),
        _ => AttitudeType::ATTITUDE_CAUTIOUS,
    }
}

/// C++ `AI_getAttitudeWeight` (CvTeamAI.cpp:3814): a signed weight used to bias the choice of which
/// enemy to bribe a third party against.
fn attitude_weight(a: AttitudeType) -> i64 {
    match a {
        AttitudeType::ATTITUDE_FURIOUS => -100,
        AttitudeType::ATTITUDE_ANNOYED => -40,
        AttitudeType::ATTITUDE_PLEASED => 50,
        AttitudeType::ATTITUDE_FRIENDLY => 100,
        _ => -5, // CAUTIOUS
    }
}

/// C++ `CvTeamAI::AI_declareWarTradeVal` (CvTeamAI.cpp:1932): the price `this_team` puts on declaring
/// war on `war_team` as a favour to `other_team`. Integer math byte-faithful (each `/` truncates).
/// Team attitude is the leaders' `attitude` (no team-averaged attitude in ffh-ai); `AI_isAllyLandTarget`
/// is unported and assumed false (the common case -> the x2 branch, a conservative upward bias on the
/// price); nukes are 0 in FfH 0.41. Widened to i64 to avoid the C++ `int` overflow on large powers.
pub(crate) fn declare_war_trade_val(g: &Game, this_team: TeamId, war_team: TeamId, other_team: TeamId) -> i32 {
    let mut v: i64 = 0;
    v += g.teams[war_team].num_cities as i64 * 10;
    v += g.team_get_total_population(war_team, true) as i64 * 2;
    let i_modifier: i64 = match team_attitude(g, other_team, war_team) {
        AttitudeType::ATTITUDE_ANNOYED => 25,
        AttitudeType::ATTITUDE_CAUTIOUS => 50,
        AttitudeType::ATTITUDE_PLEASED => 150,
        AttitudeType::ATTITUDE_FRIENDLY => 400,
        _ => 0, // FURIOUS
    };
    v *= (i_modifier + 100).max(0);
    v /= 100;
    let their_power = g.team_get_power(other_team, true) as i64;
    let war_power = g.team_get_power(war_team, true) as i64;
    v *= 50 + (100 * war_power) / (their_power + war_power + 1);
    v /= 100;
    v *= 2; // AI_isAllyLandTarget unported -> assume not an ally land target
    if !g.at_war(this_team, war_team) {
        v *= 3;
    } else {
        v *= 150;
        let ws = g.teams[this_team].ai_war_success[war_team] as i64;
        let pop = g.team_get_total_population(this_team, false) as i64;
        let inner = (100 * ws / (8 + pop)).min(100);
        v /= (100 + (50 * inner) / 100).max(1);
    }
    v += g.teams[other_team].num_cities as i64 * 20;
    v += g.team_get_total_population(other_team, true) as i64 * 15;
    if g.at_war(this_team, war_team) {
        let at_war_count = g.team_get_at_war_count(this_team, true) as i64;
        let mult: i64 = match team_attitude(g, other_team, this_team) {
            AttitudeType::ATTITUDE_PLEASED => (100 - at_war_count * 10).max(75),
            AttitudeType::ATTITUDE_FRIENDLY => (100 - at_war_count * 20).max(50),
            _ => 100, // FURIOUS / ANNOYED / CAUTIOUS
        };
        v *= mult;
        v /= 100;
    }
    v += g.team_get_num_nuke_units(war_team) as i64 * 250;
    v += g.team_get_num_nuke_units(other_team) as i64 * 150;
    if g.team_get_at_war_count(war_team, false) == 0 {
        v *= 2;
        for i in g.team_ids() {
            if g.team_is_alive(i) && i != this_team && i != war_team && i != other_team && g.teams[war_team].defensive_pact[i] {
                v += g.teams[i].num_cities as i64 * 30;
                v += g.team_get_total_population(i, true) as i64 * 20;
            }
        }
    }
    v *= 60 + (140 * g.game_turn() as i64) / (g.estimate_end_turn as i64).max(1);
    v /= 100;
    let remainder = g.d.defines.diplomacy_value_remainder.max(1) as i64;
    v -= v.rem_euclid(remainder);
    if g.team_is_human(this_team) {
        v = v.max(remainder);
    }
    v as i32
}

/// C++ `CvTeamAI::AI_endWarVal` (CvTeamAI.cpp:992): value to `team` of ending its war with `enemy`.
/// Large when we are weak / losing, cut to a fifth once we dominate (>120% power) -- this gap is
/// what funds reparations at peace. Integer math throughout (each `/` truncates). The
/// warmonger-continue sub-factors needing unported strategy/financial state (DAGGER, financial
/// trouble, the maxWar/limitedWar/dogpileWar rands) are omitted; the land-ratio cut and the
/// war-plan multiplier -- both with ported inputs -- are kept. Cities-as-tribute is not valued here
/// (`AI_cityTradeVal` unported). Widened to i64 like the rest of this module's power math to avoid
/// the C++ `int` overflow on large late-game powers.
pub(crate) fn end_war_val(g: &Game, team: TeamId, enemy: TeamId) -> i32 {
    let mut v: i64 = 100;
    v += g.teams[team].num_cities as i64 * 3;
    v += g.teams[enemy].num_cities as i64 * 3;
    v += g.team_get_total_population(team, true) as i64;
    v += g.team_get_total_population(enemy, true) as i64;
    v += g.teams[enemy].ai_war_success[team] as i64 * 20; // how hard THEY have hit US
    let our_power = g.team_get_power(team, true).max(1) as i64;
    let their_power = g.team_get_defensive_power(enemy).max(1) as i64;
    v *= their_power + 10;
    v /= (our_power + 10).max(1); // FfH (Kael 04/23/2009): divide by OUR power only
    if our_power * 100 > their_power * 120 {
        v /= 5; // FfH: if we are >120% power, peace is worth a fifth
    }
    let our_plan = g.teams[team].ai_war_plan[enemy];
    let their_plan = g.teams[enemy].ai_war_plan[team];
    // Warmonger-continue block: only when we are not human and clearly stronger. C++ also folds in
    // DAGGER / financial-trouble / warRand sub-factors (unported strategy + financial state) --
    // omitted here; only the land-ratio cut has all inputs ported.
    if !g.team_is_human(team) && our_power > 120 * their_power / 100 {
        let land_ratio = g.team_get_total_land(team, true) as i64 * 100 / (g.team_get_total_land(enemy, true).max(1) as i64);
        if land_ratio > 120 {
            v = v * (9 * 100) / (10 * land_ratio);
        }
    }
    // War-plan multiplier (either side's plan toward the other).
    let total = (!g.team_is_human(team) && our_plan == Some(WarPlanType::WARPLAN_TOTAL))
        || (!g.team_is_human(enemy) && their_plan == Some(WarPlanType::WARPLAN_TOTAL));
    let dogpile = (!g.team_is_human(team) && our_plan == Some(WarPlanType::WARPLAN_DOGPILE) && g.team_get_at_war_count(enemy, true) > 1)
        || (!g.team_is_human(enemy) && their_plan == Some(WarPlanType::WARPLAN_DOGPILE) && g.team_get_at_war_count(team, true) > 1);
    if total {
        v *= 2;
    } else if dogpile {
        v = v * 3 / 2;
    }
    // (AGGRESSIVE_AI endangered-cities block omitted: the option is off in our games.)
    let remainder = g.d.defines.diplomacy_value_remainder.max(1) as i64;
    v -= v.rem_euclid(remainder);
    if g.team_is_human(team) {
        v = v.max(remainder);
    }
    v as i32
}

/// Can unit `u` take part in a fight for a city: a land unit able to attack, not aboard a ship and
/// not standing in a city of its own team (a garrison is not an army in the field).
fn front_unit(g: &Game, u: UnitId) -> bool {
    if crate::view::attack_power(g, u) <= 0 || g.unit_get_domain_type(u) != DomainType::DOMAIN_LAND || g.unit_is_cargo(u) {
        return false;
    }
    let plot = g.unit_plot(u);
    !(g.map.plots[plot].plot_city.is_some() && g.plot_get_team(plot) == Some(g.unit_get_team(u)))
}

/// The forces at the front of the war between `team` and `enemy` (MNAI `CvTeamAI::AI_endWarVal`'s
/// attackers en route against the enemy's in our land): our units able to attack within 2 plots of
/// their city where we have the most of them (their next city), and their units able to attack
/// within 2 plots of one of our cities that we can see. What a human sees at the front.
pub(crate) fn front_forces(g: &Game, team: TeamId, enemy: TeamId) -> (i32, i32) {
    let cities_of = |t: TeamId| -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for q in g.player_ids() {
            if g.players[q].alive && g.players[q].team == t {
                v.extend(g.players[q].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y)));
            }
        }
        v
    };
    let (theirs_c, ours_c) = (cities_of(enemy), cities_of(team));
    let mut per_city = vec![0; theirs_c.len()];
    let mut theirs = 0;
    for q in g.player_ids() {
        let pl = &g.players[q];
        if !pl.alive || (pl.team != team && pl.team != enemy) {
            continue;
        }
        for &u in &pl.units {
            if !front_unit(g, u) {
                continue;
            }
            let (x, y) = (g.units[u].x, g.units[u].y);
            if pl.team == team {
                for (i, &(cx, cy)) in theirs_c.iter().enumerate() {
                    if g.plot_distance(x, y, cx, cy) <= 2 {
                        per_city[i] += 1;
                    }
                }
            } else if g.plot_is_visible(g.unit_plot(u), team) && ours_c.iter().any(|&(cx, cy)| g.plot_distance(x, y, cx, cy) <= 2) {
                theirs += 1;
            }
        }
    }
    (per_city.into_iter().max().unwrap_or(0), theirs)
}

/// How much of its value peace keeps for `team` given the forces at the front (percent, 100 down
/// to 33): MNAI's `x 200 / range(iAttackerRatio, 200, 600)`, with our units near their next city
/// against theirs near ours (at least 2). An army at their gates, unopposed, makes peace worth a
/// third: the next city is close.
pub(crate) fn forces_pct(g: &Game, team: TeamId, enemy: TeamId) -> i64 {
    let (ours, theirs) = front_forces(g, team, enemy);
    let ratio = ours as i64 * 100 / theirs.max(2) as i64;
    200 * 100 / ratio.clamp(200, 600)
}

/// [`end_war_val`] as player `p` judges it: with `PEACE_FORCES`, scaled by the forces at the front
/// ([`forces_pct`]) -- sell the war when the next city is out of reach, fight on when it is close.
pub(crate) fn end_war_val_seen(g: &Game, p: PlayerId, team: TeamId, enemy: TeamId) -> i32 {
    let v = end_war_val(g, team, enemy);
    if !crate::switches::PEACE_FORCES.on(g, p) {
        return v;
    }
    let mut v = v as i64 * forces_pct(g, team, enemy) / 100;
    let remainder = g.d.defines.diplomacy_value_remainder.max(1) as i64;
    v -= v.rem_euclid(remainder);
    if g.team_is_human(team) {
        v = v.max(remainder);
    }
    v as i32
}

/// A war where nothing happens (MNAI `CvTeamAI::AI_doWar`, "making peace due to time and no
/// fighting"): past 15 turns, fewer than two attacks' worth of war success on both sides, no army
/// of ours near their cities nor of theirs (that we see) near ours, and no stack of ours marching
/// on them.
pub(crate) fn quiet_war(g: &Game, team: TeamId, enemy: TeamId) -> bool {
    if g.teams[team].ai_at_war_counter[enemy] < 15 {
        return false;
    }
    let success = g.teams[team].ai_war_success[enemy] + g.teams[enemy].ai_war_success[team];
    if success >= 2 * g.d.defines.war_success_attacking.max(1) {
        return false;
    }
    if front_forces(g, team, enemy) != (0, 0) {
        return false;
    }
    !g.player_ids().into_iter().any(|q| {
        g.players[q].alive
            && g.players[q].team == team
            && g.players[q].groups.iter().any(|&grp| crate::campaign::tagged(g, grp).is_some_and(|pl| g.plot_get_team(pl) == Some(enemy)))
    })
}

/// Should `team` seek peace with `enemy`?
pub(crate) fn want_peace(g: &Game, team: TeamId, enemy: TeamId, plan: &crate::victory::Plan) -> bool {
    let counter = g.teams[team].ai_at_war_counter[enemy];
    if counter < 10 {
        return false;
    }
    let ours = g.team_get_power(team, true).max(1) as i64;
    let theirs = g.team_get_power(enemy, true).max(1) as i64;
    let success = g.teams[team].ai_war_success[enemy] as i64;
    let their_success = g.teams[enemy].ai_war_success[team] as i64;
    // Losing: they hit harder and are not weaker.
    if their_success > success + 10 && ours * 10 < theirs * 12 {
        return true;
    }
    // victory-pursuit: a war for our domination or conquest, or against a rival about to win,
    // goes on while it is not lost.
    if crate::victory::wants_conquest(plan) || crate::victory::rival_team(g, plan) == Some(enemy) {
        return false;
    }
    // Clearly winning: keep going (a long war still ends eventually).
    if ours > theirs * 2 && counter < 60 {
        return false;
    }
    // Stalemate.
    counter >= 40 && success < their_success * 2 + 20 || counter >= 80
}

/// C++ `AI_considerOffer` equivalent: accept peace when the war is not going our way or has
/// lasted long, open borders with friends, and gifts.
pub fn consider_offer(g: &Game, p: PlayerId, from: PlayerId, they_give: &[TradeData], we_give: &[TradeData]) -> bool {
    // Vassal states come with strings attached (vassals.rs): asked to become `from`'s vassal, or
    // offered `from` as ours.
    if let Some(yes) = crate::vassals::agree(g, p, from, we_give) {
        return yes;
    }
    if let Some(yes) = crate::vassals::accept_master(g, p, from, they_give, we_give) {
        return yes;
    }
    if we_give.is_empty() {
        return true;
    }
    let team = g.players[p].team;
    let their_team = g.players[from].team;
    let all = |items: &[TradeData], kind: TradeableItem| items.iter().all(|t| t.item_type == kind);
    // Peace, possibly with reparations: each list carries a peace treaty, and either may also carry
    // tribute (tech / gold). Value both sides as the at-war `AI_endWarVal` baseline plus the tribute
    // (C++ `AI_dealVal`/`AI_considerOffer`, CvPlayerAI.cpp:6815, 7057) and accept when what we get is
    // worth at least what we give.
    let has_peace = |items: &[TradeData]| items.iter().any(|t| t.item_type == TradeableItem::TRADE_PEACE_TREATY);
    if has_peace(they_give) && has_peace(we_give) {
        let counter = g.teams[team].ai_at_war_counter[their_team];
        if counter < 10 {
            return false;
        }
        // Never hand over a resource our own judgement blocks (ported `AI_bonusTrade`).
        for item in we_give {
            if item.item_type == TradeableItem::TRADE_RESOURCES
                && crate::trade::bonus_trade_denial(g, p, BonusType::from_usize(item.data as usize), from).is_some()
            {
                return false;
            }
        }
        let ours = g.team_get_power(team, true).max(1) as i64;
        let theirs = g.team_get_power(their_team, true).max(1) as i64;
        let success = g.teams[team].ai_war_success[their_team] as i64;
        let their_success = g.teams[their_team].ai_war_success[team] as i64;
        // victory-pursuit / press-conquest OVERRIDE: a war for our domination or conquest, against a
        // rival about to win, or a total war we are winning (press_conquest), is not sold for tribute
        // -- it ends only when it is lost (or after 80 turns). Without the press_conquest arm a winning
        // conqueror would simply re-buy peace from the loser's sweetened side of the deal.
        let plan = crate::victory::plan(g, p);
        if crate::victory::wants_conquest(&plan) || crate::victory::rival_team(g, &plan) == Some(their_team) || press_conquest(g, team, their_team) {
            let losing = their_success > success + 10 && ours * 10 < theirs * 12;
            return losing || counter >= 80;
        }
        // Reparations: value each side as the at-war `AI_endWarVal` baseline plus its tribute (C++
        // `AI_dealVal`/`AI_considerOffer`, CvPlayerAI.cpp:6815, 7057). A deal that pays us at least
        // what we give away -- a winner's reparations, or a loser sweetening the peace -- is taken.
        let their_v = end_war_val_seen(g, p, team, their_team) + crate::trade::deal_val(g, p, from, they_give, false, 1);
        let our_v = end_war_val_seen(g, p, their_team, team) + crate::trade::deal_val(g, from, p, we_give, false, 1);
        if their_v >= our_v {
            return true;
        }
        // The deal leaves us behind on value. Give away tribute of our own only when it pays
        // (the test above) -- never fall through to accept a peace in which we also hand something
        // over. A BARE peace (we give nothing but the treaty) still ends on plain willingness: we
        // take it unless we are clearly winning, and always once the war has dragged to exhaustion
        // (keeps the pre-reparations war-ending behaviour, so even/stalemated wars still resolve).
        if we_give.iter().any(|t| t.item_type != TradeableItem::TRADE_PEACE_TREATY) {
            return false;
        }
        let winning = ours > theirs * 3 / 2 && success > their_success + 20;
        // Nor while our army stands at their next city, three times what they have near ours
        // (PEACE_FORCES): the city is close.
        let losing = their_success > success + 10 && ours * 10 < theirs * 12;
        let at_gates = crate::switches::PEACE_FORCES.on(g, p) && !losing && forces_pct(g, team, their_team) <= 66;
        return !winning && !at_gates || counter >= 60;
    }
    // War bribe: `from` pays us (tech / gold) to declare war on a third team. Judge as the bribee.
    // The AI-judgement denial gates `ai_declare_war_trade` leaves to the AI (C++ `AI_declareWarTrade`,
    // CvTeamAI.cpp:2096-2155) run here; vassal/DP/can-declare are already enforced at construction by
    // `player_can_trade_item`. Then value both sides (`deal_val` scores the TRADE_WAR item via
    // `declare_war_trade_val`) and accept when the payment is worth at least our price (C++ 7057).
    if let Some(war_item) = we_give.iter().find(|t| t.item_type == TradeableItem::TRADE_WAR) {
        let target = TeamId(war_item.data as u8);
        // One war at a time (DENIAL_TOO_MANY_WARS).
        if g.team_get_at_war_count(team, true) > 0 || preparing_target(g, team).is_some() {
            return false;
        }
        // Too strong even with the briber's help, when the briber is already at war with the target
        // (DENIAL_NO_GAIN; AI_isAllyLandTarget unported -> no /2).
        let briber_help = if g.at_war(their_team, target) { g.team_get_power(their_team, true) as i64 } else { 0 };
        if g.team_get_defensive_power(target) as i64 > g.team_get_power(team, true) as i64 + briber_help {
            return false;
        }
        // Won't declare war for a civ we dislike, nor against one we like too much.
        if let Some(l) = g.players[p].personality_type.map(|l| &g.d.leader_heads[l]) {
            if attitude(g, p, from).i() <= opt_to_i32(l.declare_war_refuse_attitude_threshold) {
                return false;
            }
            if team_attitude(g, team, target).i() > opt_to_i32(l.declare_war_them_refuse_attitude_threshold) {
                return false;
            }
        }
        // (Target nukes: FfH 0.41 has none, and the briber is at war with the target anyway.)
        let theirs = crate::trade::deal_val(g, p, from, they_give, false, 1);
        let ours = crate::trade::deal_val(g, from, p, we_give, false, 1);
        return theirs >= ours;
    }
    // Tech-trade discipline (TECH_DISCIPLINE): no technology to our victory rival, no military
    // technology while we prepare or fight a war, no monopoly worth keeping to a non-friend.
    let gives_tech = we_give.iter().any(|t| t.item_type == TradeableItem::TRADE_TECHNOLOGIES);
    let rules = if gives_tech { tech_ctx(g, p) } else { TechCtx { on: false, rival: None, leader: None, war: false } };
    if gives_tech {
        let denied = we_give
            .iter()
            .filter(|t| t.item_type == TradeableItem::TRADE_TECHNOLOGIES)
            .find_map(|t| tech_denial(g, p, from, TechType::from_usize(t.data as usize), &rules));
        if let Some(why) = denied {
            g.tel_decision(p, "tech_trade", "refused", why, 0, String::new);
            return false;
        }
    }
    if all(they_give, TradeableItem::TRADE_DEFENSIVE_PACT) && all(we_give, TradeableItem::TRADE_DEFENSIVE_PACT) {
        return crate::pacts::accept(g, p, from);
    }
    if all(they_give, TradeableItem::TRADE_OPEN_BORDERS) && all(we_give, TradeableItem::TRADE_OPEN_BORDERS) {
        return !g.at_war(team, their_team) && relation(g, p, from) >= 1;
    }
    // Technology for technology: worth at least four fifths of what we give, with a civ we do
    // not dislike, and never to the one we are preparing a war against.
    if all(they_give, TradeableItem::TRADE_TECHNOLOGIES) && all(we_give, TradeableItem::TRADE_TECHNOLOGIES) {
        if g.at_war(team, their_team) || relation(g, p, from) < 0 || preparing_target(g, team) == Some(their_team) {
            return false;
        }
        // (From the world's clear leader we want more than we give: `TechCtx::min_return_pct`.)
        let value = |items: &[TradeData]| items.iter().map(|t| g.team_get_research_cost(team, TechType::from_usize(t.data as usize)) as i64).sum::<i64>();
        return value(they_give) * 100 >= value(we_give) * rules.min_return_pct(g, from);
    }
    // Resources and gold (and any mixed deal): value both sides in gold-equivalent (C++
    // `AI_dealVal`) and accept when what we receive is worth at least what we give away. A friendly
    // act, so only at peace with a civ we do not dislike and are not about to attack; we refuse
    // gold-for-gold, and never hand over a resource our own judgement blocks (`trade::bonus_trade_denial`,
    // the ported `AI_bonusTrade`: our worst enemy, a resource they already have, or a strategic /
    // happiness / health resource to a civ we do not like enough).
    let tradeable = |items: &[TradeData]| {
        items
            .iter()
            .any(|t| matches!(t.item_type, TradeableItem::TRADE_RESOURCES | TradeableItem::TRADE_GOLD | TradeableItem::TRADE_GOLD_PER_TURN))
    };
    if tradeable(they_give) || tradeable(we_give) {
        if crate::trade::is_gold_deal(they_give) && crate::trade::is_gold_deal(we_give) {
            return false;
        }
        if g.at_war(team, their_team) || relation(g, p, from) < 0 || preparing_target(g, team) == Some(their_team) {
            return false;
        }
        for item in we_give {
            if item.item_type == TradeableItem::TRADE_RESOURCES
                && crate::trade::bonus_trade_denial(g, p, BonusType::from_usize(item.data as usize), from).is_some()
            {
                return false;
            }
        }
        let theirs = crate::trade::deal_val(g, p, from, they_give, false, 1);
        let ours = crate::trade::deal_val(g, from, p, we_give, false, 1);
        return theirs >= ours;
    }
    false
}

/// C++ `AI_conquerCity`: raze small, distant conquests we could not hold.
pub fn raze_conquered_city(g: &Game, p: PlayerId, c: CityId) -> bool {
    let city = &g.cities[c];
    if city.population > 2 || g.player_is_barbarian(p) {
        return false;
    }
    let (x, y) = (city.x, city.y);
    let nearest = g.players[p].cities.iter().filter(|&&o| o != c).map(|&o| g.plot_distance(x, y, g.cities[o].x, g.cities[o].y)).min().unwrap_or(i32::MAX);
    let holy = g.d.religions.keys().any(|r| g.get_holy_city(r) == Some(c));
    nearest > 10 && !holy && city.num_world_wonders == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::tests::one_landmass_game;

    #[test]
    fn the_trust_spell_raises_attitudes() {
        let mut g = one_landmass_game(19);
        let (us, them) = (PlayerId(0), PlayerId(1));
        let (before, theirs) = (relation(&g, us, them), relation(&g, them, us));
        g.players[them].feat_accomplished[FeatType::FEAT_TRUST] = true;
        assert_eq!(relation(&g, us, them), before + 2, "Trust adds one attitude step towards its caster");
        assert!(attitude_parts(&g, us, them).contains(&("the Trust spell", 2)), "and the reason shows");
        assert_eq!(relation(&g, them, us), theirs, "the caster's own attitude is unchanged");
    }
}
