//! The councils: votes and the secretary's resolutions (C++ `CvPlayerAI::AI_diploVote`,
//! `CvTeamAI::AI_chooseElection`; More Naval AI's versions and FfH 0.41's, ported in ffh-cppai,
//! were read).
//!
//! FfH's Overcouncil (good and neutral civs, the Overcouncil civic) and Undercouncil (evil and
//! neutral, the Undercouncil civic) meet every 10 and 15 turns once two civs run the civic. Their
//! members elect a head councilor (the secretary general), who proposes one resolution a session;
//! every member votes on it, weighted by population, and half the votes pass it. Before this module
//! every AI abstained and never proposed, so both councils were inert.
//!
//! A member votes by what the resolution does to it ([`value`]): a war declared on its enemy is
//! welcome and one on itself is defied, a forced peace is judged like the peace it would make, a
//! banned mana is a loss for those who hold it and a blow to the rivals who do, a forced civic is
//! weighed against our own choice, trade routes, free units, embargoes, pacts and open borders by
//! who they bind us to. Positive: yes, negative: no, nothing at stake: abstain. A resolution that
//! would cost us dearly (a war on us, one of our cities given away) is defied: the rules let a
//! member veto a resolution at the price of unrest and its loyalty. For the head councilor's seat
//! we vote for ourselves when we stand, else for the candidate we get on with best, never our
//! victory rival or the world's clear leader. The head councilor proposes the resolution that
//! gains it the most among those our model of the members' votes says will pass (or, for one in
//! force that hurts us, will fail and so be repealed).
//!
//! Deterministic: no rolls (the C++ AI's defiance and proposal dice are not reproduced), pure
//! functions of the game state; nothing is stored.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::*;
use ffh_data::*;

use crate::diplomacy::{relation, team_leader};

/// What a member reads once before judging resolutions.
pub(crate) struct Seen {
    p: PlayerId,
    team: TeamId,
    plan: crate::victory::Plan,
    /// The victory rival our plan says to stop.
    rival: Option<TeamId>,
    /// The world's clear leader, when it is not us.
    leader: Option<TeamId>,
    /// The civ we prepare a war against.
    preparing: Option<TeamId>,
}

pub(crate) fn seen(g: &Game, p: PlayerId) -> Seen {
    let team = g.players[p].team;
    let plan = crate::victory::plan(g, p);
    Seen {
        p,
        team,
        rival: crate::victory::rival_team(g, &plan),
        leader: crate::diplomacy::clear_leader(g).filter(|&t| t != team),
        preparing: crate::diplomacy::preparing_target(g, team),
        plan,
    }
}

impl Seen {
    /// Our victory rival or the world's clear leader.
    fn front_runner(&self, t: TeamId) -> bool {
        self.rival == Some(t) || self.leader == Some(t)
    }
}

/// Voting member teams of the source other than ours.
fn members(g: &Game, team: TeamId, vs: VoteSourceType) -> Vec<TeamId> {
    g.civ_team_ids().filter(|&t| t != team && g.team_is_alive(t) && g.team_is_voting_member(t, vs)).collect()
}

/// Relation of our player to team `t`'s leader (0 when it has none).
fn rel_team(g: &Game, s: &Seen, t: TeamId) -> i32 {
    team_leader(g, t).map_or(0, |q| relation(g, s.p, q))
}

/// Our side of a forced peace with `enemy`, a team we are at war with: wanted when the war goes
/// badly, refused when we are winning it (or it is a war our victory needs).
fn peace_with(g: &Game, s: &Seen, enemy: TeamId) -> i32 {
    if crate::diplomacy::want_peace(g, s.team, enemy, &s.plan) {
        return 100;
    }
    if crate::victory::wants_conquest(&s.plan) || s.rival == Some(enemy) || crate::diplomacy::press_conquest(g, s.team, enemy) {
        return -150;
    }
    let ours = crate::diplomacy::end_war_val_seen(g, s.p, s.team, enemy);
    let theirs = crate::diplomacy::end_war_val_seen(g, s.p, enemy, s.team);
    if theirs > ours {
        -60
    } else {
        30
    }
}

/// Every member declares war on team `t`.
fn force_war(g: &Game, s: &Seen, t: TeamId, vs: VoteSourceType) -> i32 {
    let team = s.team;
    if t == team || g.teams[team].vassal[t] || g.teams[t].vassal[team] {
        return -1000;
    }
    if g.at_war(team, t) {
        return 200; // allies against our enemy
    }
    if s.preparing == Some(t) {
        return 150;
    }
    // We declare too: a war we would choose, or a coalition that dwarfs the front-runner.
    if s.front_runner(t) {
        let coalition: i64 = members(g, t, vs).iter().map(|&m| g.team_get_power(m, false) as i64).sum();
        if coalition * 100 >= g.team_get_defensive_power(t).max(1) as i64 * 150 {
            return 100;
        }
    }
    let bar = crate::diplomacy::war_bar(g, s.p);
    if team_leader(g, t).is_some_and(|q| crate::diplomacy::territory_target(g, s.p, &bar, q, false).is_some()) {
        return 80;
    }
    if crate::diplomacy::reach_distance(g, team, t).is_some_and(|d| d <= 16) {
        -100
    } else {
        -40
    }
}

/// Every member not on team `t`'s side makes peace with it (or signs a peace treaty).
fn force_peace(g: &Game, s: &Seen, t: TeamId, vs: VoteSourceType) -> i32 {
    let team = s.team;
    if t == team {
        let mut v = 0;
        for m in members(g, team, vs) {
            if g.at_war(team, m) {
                v += peace_with(g, s, m);
            } else if s.preparing == Some(m) {
                v -= 100;
            } else if crate::pacts::threat(g, s.p).is_some_and(|(th, _)| th == m) {
                v += 30;
            }
        }
        return v;
    }
    if g.at_war(team, t) {
        return peace_with(g, s, t);
    }
    if s.preparing == Some(t) {
        return -100;
    }
    // It fights one of our enemies: keep it busy.
    let ours_too = g.civ_team_ids().any(|e| e != t && e != team && g.team_is_alive(e) && g.at_war(team, e) && g.at_war(t, e));
    if ours_too {
        return -50;
    }
    // Its own wars: stop a winner, spare a loser we like.
    let (mut success, mut against) = (0, 0);
    for e in g.civ_team_ids() {
        if e != t && g.team_is_alive(e) && g.at_war(t, e) {
            success += g.teams[t].ai_war_success[e];
            against += g.teams[e].ai_war_success[t];
        }
    }
    let winning = success > against;
    if s.front_runner(t) {
        return if winning { 60 } else { -20 };
    }
    let rel = rel_team(g, s, t);
    match (winning, rel) {
        (true, r) if r < 0 => 30,
        (false, r) if r > 0 => 20,
        _ => 0,
    }
}

/// Members stop trading with team `t`.
fn force_no_trade(g: &Game, s: &Seen, t: TeamId) -> i32 {
    let team = s.team;
    if t == team {
        return -100;
    }
    if g.at_war(team, t) || s.preparing == Some(t) {
        return 40;
    }
    let rel = rel_team(g, s, t);
    let mut v = if rel < 0 { 20 } else if rel >= 1 { -20 } else { 0 };
    if g.team_is_open_borders(team, t) {
        v -= 20; // trade routes lost
    }
    if s.front_runner(t) {
        v += 20;
    }
    v
}

/// City `data.city` of `data.player` goes to `data.other_player` (its culture's owner).
fn assign_city(g: &Game, s: &Seen, data: &VoteSelectionSubData) -> i32 {
    let (Some(owner), Some(to)) = (data.player, data.other_player) else { return 0 };
    let (ot, tt) = (g.players[owner].team, g.players[to].team);
    if tt == s.team {
        return 200;
    }
    if ot == s.team {
        return -300;
    }
    let mut v = (relation(g, s.p, to) - relation(g, s.p, owner)) * 10;
    if s.front_runner(tt) {
        v -= 30;
    }
    if s.front_runner(ot) {
        v += 30;
    }
    v
}

/// Open borders between all members: trade routes with friends, passage for those we distrust.
fn open_borders(g: &Game, s: &Seen, vs: VoteSourceType) -> i32 {
    let mut v = 0;
    for m in members(g, s.team, vs) {
        if g.at_war(s.team, m) || s.preparing == Some(m) {
            return -100;
        }
        if !g.team_is_open_borders(s.team, m) {
            v += if rel_team(g, s, m) < 0 { -20 } else { 10 };
        }
    }
    v
}

/// Defensive pacts between all members: a shield when a neighbour threatens us, a chain that drags
/// us into others' wars (and ties our hands) otherwise.
fn defensive_pact(g: &Game, s: &Seen, vs: VoteSourceType) -> i32 {
    let bar = crate::diplomacy::war_bar(g, s.p);
    for m in members(g, s.team, vs) {
        if g.at_war(s.team, m) || s.preparing == Some(m) {
            return -200;
        }
        if team_leader(g, m).is_some_and(|q| crate::diplomacy::territory_target(g, s.p, &bar, q, false).is_some()) {
            return -60; // a civ we could take
        }
    }
    if crate::pacts::threat(g, s.p).is_some() {
        40
    } else {
        -20
    }
}

/// What resolution `data` of source `vs` is worth to the member `s` (positive: wanted).
pub(crate) fn value(g: &Game, s: &Seen, data: &VoteSelectionSubData, vs: VoteSourceType) -> i32 {
    let d = g.d;
    let info = &d.votes[data.vote];
    let p = s.p;
    let team = s.team;
    let target = data.player.map(|q| g.players[q].team);
    if info.force_war {
        return target.map_or(0, |t| force_war(g, s, t, vs));
    }
    if info.force_peace {
        return target.map_or(0, |t| force_peace(g, s, t, vs));
    }
    if info.force_no_trade {
        return target.map_or(0, |t| force_no_trade(g, s, t));
    }
    if info.assign_city {
        return assign_city(g, s, data);
    }
    if info.open_borders {
        return open_borders(g, s, vs);
    }
    if info.defensive_pact {
        return defensive_pact(g, s, vs);
    }
    // The rest add up (a resolution may carry several effects).
    let mut v = 0;
    for c in d.civics.keys().filter(|&c| info.force_civic[c]) {
        if g.player_is_civic(p, c) {
            v += 20;
            continue;
        }
        let Some(opt) = d.civics[c].civic_option_type else { continue };
        let best = d
            .civics
            .keys()
            .filter(|&o| d.civics[o].civic_option_type == Some(opt) && (g.player_can_do_civics(p, Some(o)) || g.players[p].civics[opt] == Some(o)))
            .map(|o| crate::empire::civic_value(g, p, o))
            .max()
            .unwrap_or(0);
        v += if crate::empire::civic_value(g, p, c) * 120 / 100 >= best { 10 } else { -40 };
    }
    if info.trade_routes > 0 {
        // MNAI: a large empire gains little from the routes everyone gets.
        let fair = 2 * g.num_cities / (g.count_civ_players_alive() + 1);
        v += if g.players[p].cities.len() as i32 > fair { -10 } else { 20 * info.trade_routes };
    }
    if info.free_trade {
        let closed = g.civ_team_ids().any(|t| t != team && g.team_is_alive(t) && g.teams[team].has_met[t] && !g.team_is_open_borders(team, t) && g.teams[t].num_cities > 0);
        v += if closed { 20 } else { 0 };
    }
    if let Some(b) = info.no_bonus {
        if g.player_get_num_available_bonuses(p, b) > 0 {
            v -= 60;
        } else {
            let foe_has = g.civ_player_ids().any(|q| {
                let qt = g.players[q].team;
                g.players[q].alive && qt != team && (g.at_war(team, qt) || s.preparing == Some(qt) || s.front_runner(qt)) && g.player_get_num_available_bonuses(q, b) > 0
            });
            v += if foe_has { 30 } else { 5 };
        }
    }
    if info.no_outside_tech_trades {
        // Technology swaps with the civs outside the council end.
        let outside = g
            .civ_team_ids()
            .filter(|&t| t != team && g.team_is_alive(t) && g.teams[team].has_met[t] && !g.at_war(team, t) && !g.team_is_voting_member(t, vs) && g.teams[t].num_cities > 0)
            .count() as i32;
        v -= 5 * outside;
    }
    if info.free_units > 0 && g.players[p].gold > info.cost + 10 + crate::civs::gold_floor(g, p) {
        // A great person (no hammer cost) is worth a technology; another unit, its upkeep less.
        let great = info.free_unit_class.and_then(|uc| d.unit_classes[uc].default_unit_index).is_some_and(|u| d.units[u].production_cost < 0);
        v += info.free_units * if great { 40 } else { 10 };
    }
    if d.ty::<VoteType>("VOTE_FUND_DISSIDENTS") == Some(data.vote) {
        // Unrest in the Overcouncil members' cities.
        if let Some(over) = d.ty::<VoteSourceType>("DIPLOVOTE_OVERCOUNCIL") {
            for q in g.civ_player_ids() {
                let qt = g.players[q].team;
                if !g.players[q].alive || !g.player_is_full_member(q, over) {
                    continue;
                }
                if qt == team {
                    v -= 50;
                } else if g.at_war(team, qt) || s.preparing == Some(qt) || s.front_runner(qt) {
                    v += 20;
                } else {
                    let r = relation(g, p, q);
                    v += if r >= 2 { -15 } else if r < 0 { 5 } else { 0 };
                }
            }
        }
    }
    if info.gambling_ring {
        v += 10; // Gambling Houses at a quarter of the cost
    }
    if info.smuggling_ring {
        v += 5; // the Smugglers' Port
    }
    v - info.crime
}

/// A resolution worth this little to us (a war on us, our city given away) is defied.
const DEFY: i32 = -300;

/// Our vote for the head councilor: ourselves when we stand, our master, else the candidate we
/// get on with best that is neither our victory rival nor the world's clear leader (and not at war
/// with us), when we do not dislike it; else abstain.
fn team_vote(g: &Game, p: PlayerId, vs: VoteSourceType) -> PlayerVote {
    let team = g.players[p].team;
    if g.is_team_vote_eligible(team, vs) {
        return PlayerVote::for_team(team);
    }
    let s = seen(g, p);
    let mut best: Option<(i32, TeamId)> = None;
    for t in g.civ_team_ids() {
        if t == team || !g.team_is_alive(t) || !g.is_team_vote_eligible(t, vs) {
            continue;
        }
        if g.teams[team].vassal[t] {
            return PlayerVote::for_team(t);
        }
        if g.at_war(team, t) || s.front_runner(t) {
            continue;
        }
        let r = rel_team(g, &s, t);
        if r >= 0 && best.map_or(true, |(b, bt)| (r, std::cmp::Reverse(t)) > (b, std::cmp::Reverse(bt))) {
            best = Some((r, t));
        }
    }
    best.map_or(PlayerVote::PLAYER_VOTE_ABSTAIN, |(_, t)| PlayerVote::for_team(t))
}

fn vote_of(g: &Game, s: &Seen, data: &VoteSelectionSubData, vs: VoteSourceType) -> PlayerVote {
    let v = value(g, s, data, vs);
    if v <= DEFY && g.player_can_defy_resolution(s.p, vs, data) {
        PlayerVote::PLAYER_VOTE_NEVER
    } else if v > 0 {
        PlayerVote::PLAYER_VOTE_YES
    } else if v < 0 {
        PlayerVote::PLAYER_VOTE_NO
    } else {
        PlayerVote::PLAYER_VOTE_ABSTAIN
    }
}

fn vote_name(g: &Game, vote: VoteType) -> &str {
    g.d.votes[vote].base.type_.trim_start_matches("VOTE_")
}

/// `Controller::diplo_vote` (C++ `CvPlayerAI::AI_diploVote`).
pub fn diplo_vote(g: &Game, p: PlayerId, data: &VoteSelectionSubData, vs: VoteSourceType) -> PlayerVote {
    if !crate::switches::COUNCIL.on(g, p) {
        return PlayerVote::PLAYER_VOTE_ABSTAIN;
    }
    if g.is_team_vote(data.vote) {
        let v = team_vote(g, p, vs);
        g.tel_decision(p, "vote", "secretary", if v.team() == Some(g.players[p].team) { "self" } else if v.team().is_some() { "other" } else { "abstain" }, 0, String::new);
        return v;
    }
    let s = seen(g, p);
    let v = vote_of(g, &s, data, vs);
    if g.telemetry.on() {
        let how = match v {
            PlayerVote::PLAYER_VOTE_YES => "yes",
            PlayerVote::PLAYER_VOTE_NO => "no",
            PlayerVote::PLAYER_VOTE_NEVER => "defy",
            _ => "abstain",
        };
        g.tel_decision(p, "vote", &format!("{}:{}", vote_name(g, data.vote), how), how, value(g, &s, data, vs) as i64, String::new);
    }
    crate::trace!(g, p, "votes {:?} on {}", v, data.text);
    v
}

/// Would resolution `data` pass, by our model of the members' votes: our AI's members vote by
/// [`value`] (a defiance vetoes it), humans and other controllers abstain.
fn would_pass(g: &Game, seen_by: &[(PlayerId, Seen)], data: &VoteSelectionSubData, vs: VoteSourceType) -> bool {
    let mut yes = 0;
    for (q, s) in seen_by.iter() {
        match vote_of(g, s, data, vs) {
            PlayerVote::PLAYER_VOTE_YES => yes += g.player_get_votes(*q, Some(data.vote), vs),
            PlayerVote::PLAYER_VOTE_NEVER => return false,
            _ => {}
        }
    }
    yes >= g.vote_required(data.vote, vs)
}

/// `Controller::choose_election` (C++ `CvTeamAI::AI_chooseElection`): the head councilor `p`
/// proposes the resolution that gains it most -- one it wants that is not in force and will pass,
/// or one in force it wants gone that will fail (and so be repealed); nothing when none does.
pub fn choose_election(g: &Game, p: PlayerId, sel: &VoteSelectionData) -> Option<usize> {
    if !crate::switches::COUNCIL.on(g, p) {
        return None;
    }
    let vs = sel.vote_source;
    let ours = seen(g, p);
    let voters: Vec<(PlayerId, Seen)> = g
        .civ_player_ids()
        .filter(|&q| g.players[q].alive && g.player_is_voting_member(q, vs) && !g.players[q].human && g.controller(q).name() == "ffh-ai" && crate::switches::COUNCIL.on(g, q))
        .map(|q| (q, seen(g, q)))
        .collect();
    let mut best: Option<(i32, usize)> = None;
    for (i, data) in sel.vote_options.iter().enumerate() {
        if g.is_team_vote(data.vote) {
            continue;
        }
        let v = value(g, &ours, data, vs);
        let in_force = g.is_vote_passed(data.vote);
        let gain = if in_force { -v } else { v };
        if gain <= 0 || best.is_some_and(|(b, _)| gain <= b) {
            continue;
        }
        if would_pass(g, &voters, data, vs) == in_force {
            continue; // proposing it changes nothing
        }
        best = Some((gain, i));
    }
    let (gain, i) = best?;
    let data = &sel.vote_options[i];
    crate::trace!(g, p, "proposes {} (gain {})", data.text, gain);
    g.tel_decision(p, "election", vote_name(g, data.vote), if g.is_vote_passed(data.vote) { "repeal" } else { "pass" }, gain as i64, String::new);
    Some(i)
}
