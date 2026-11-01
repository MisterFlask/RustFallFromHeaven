//! Calling a crusade, pledges, and the standing a call costs (docs/CRUSADE.md, "Calling a
//! crusade").
//!
//! A call names a target (and a victim, if any), an objective, a term, a pledge deadline and the
//! civs called. Each called civ owes an *obligation*: a share of its military strength that
//! falls with its distance to the target, so the call cannot demand the impossible. It answers
//! with a pledge or a refusal (the controller's `crusade_pledge`, asked every turn until it
//! answers). At the deadline what it delivered is settled against its obligation: nothing
//! (a refusal, no answer, or no units) costs its full standing with the caller; less than the
//! obligation costs the shortfall's share; the obligation met costs nothing. For a council the
//! standing is the existing defiance: a full refusal defies the resolution
//! (`setDefiedResolution`: defiance anger in the cities, loyal membership lost); a shortfall
//! gives its share of the defiance anger only.

use super::*;
use crate::structs::VoteSelectionSubData;

/// Did a called civ refuse (or ignore) the call? A pledge with nothing delivered against an
/// obligation counts as refusing; a civ with nothing to lend (obligation 0) that pledged did not.
pub fn call_refused(call: &Call, delivered: i32) -> bool {
    matches!(call.answer, None | Some(Pledge::Refuse)) || (delivered == 0 && call.obligation > 0)
}

impl Game {
    /// Start a crusade: a Host comes alive at war with `target`'s bloc, and `called` civs are
    /// asked to pledge. `term` and `deadline` are in turns at this game's speed. `None` when
    /// crusades are off, the target is dead or no Host slot is free.
    #[allow(clippy::too_many_arguments)]
    pub fn crusade_call(
        &mut self,
        kind: CrusadeKind,
        target: TeamId,
        victim: Option<PlayerId>,
        objective: Objective,
        term: i32,
        deadline: i32,
        called: &[PlayerId],
    ) -> Option<CrusadeId> {
        if !self.crusades_enabled() || !self.team_is_alive(target) || self.team_is_barbarian(target) || self.team_is_crusade_host(target) {
            return None;
        }
        let host = self.crusade_free_host_slot()?;
        let turn = self.game_turn();
        let id = CrusadeId(self.next_crusade_id);
        self.next_crusade_id += 1;

        let target_leader = self.team_get_leader_id(target)?;
        let name = match kind {
            CrusadeKind::Loan { borrower } => {
                format!("Host of {} against {}", self.player_get_civilization_description(borrower), self.player_get_civilization_description(target_leader))
            }
            _ => format!("Crusade against {}", self.player_get_civilization_description(target_leader)),
        };
        self.crusade_activate_host(host, target, name);

        let mut calls = Vec::new();
        for &p in called {
            if self.players[p].alive && self.players[p].team != target && !calls.iter().any(|c: &Call| c.player == p) {
                let obligation = self.crusade_obligation(p, target);
                calls.push(Call { player: p, obligation, answer: None, settled: false });
            }
        }
        let crusade = Crusade {
            id,
            kind,
            host,
            target,
            victim,
            objective,
            start_turn: turn,
            end_turn: turn + term.max(1),
            deadline_turn: turn + deadline.max(0),
            calls,
            lenders: Vec::new(),
            victim_plots: Vec::new(),
            had_units: false,
            dissolving: false,
            defend: false,
            engaged: 0,
            ai: HostAi { state: HostState::Muster, since: turn, rally: None, goal: None },
        };
        self.crusades.push(crusade);
        let idx = self.crusades.len() - 1;

        // The Host fights the target and its vassals.
        let host_team = self.players[host].team;
        for t in self.crusade_enemy_teams(idx) {
            self.crusade_set_host_war(host_team, t);
        }
        if let Some(v) = victim {
            let vt = self.players[v].team;
            self.crusade_grant_passage(idx, vt);
        }
        let log_kind = kind;
        self.log.msg(turn, None, || format!("Crusade {} ({:?}) called against team {}", id.0, log_kind, target));
        // Ask at once (an AI answers now; a human may answer until the deadline).
        self.crusade_ask_pledges(idx);
        Some(id)
    }

    /// Obligation of `p` called against `target`: `obligation_percent` of its military strength
    /// next to the target, falling linearly to `far_obligation_percent` of that at the map's
    /// farthest distance (capital to the target's nearest city).
    pub fn crusade_obligation(&self, p: PlayerId, target: TeamId) -> i32 {
        let rules = self.crusade_rules();
        let strength = self.crusade_military_strength(p) as i64;
        let near = strength * rules.obligation_percent as i64 / 100;
        let Some(capital) = self.players[p].capital_city.filter(|&c| self.cities.contains(c)) else { return near as i32 };
        let (cx, cy) = (self.cities[capital].x, self.cities[capital].y);
        let mut dist = i32::MAX;
        for q in self.player_ids() {
            if self.players[q].alive && self.players[q].team == target {
                for &c in &self.players[q].cities {
                    dist = dist.min(self.plot_distance(cx, cy, self.cities[c].x, self.cities[c].y));
                }
            }
        }
        if dist == i32::MAX {
            return near as i32;
        }
        let max = self.map.max_plot_distance().max(1) as i64;
        let dist = (dist as i64).min(max);
        let far = rules.far_obligation_percent as i64;
        let percent = 100 - (100 - far) * dist / max;
        (near * percent / 100) as i32
    }

    /// Ask every called civ that has not answered (the controller's `crusade_pledge`).
    pub(crate) fn crusade_ask_pledges(&mut self, idx: usize) {
        let id = self.crusades[idx].id;
        for i in 0..self.crusades[idx].calls.len() {
            let call = &self.crusades[idx].calls[i];
            if call.answer.is_some() || call.settled || !self.players[call.player].alive {
                continue;
            }
            let p = call.player;
            if let Some(answer) = self.controller(p).crusade_pledge(self, p, id) {
                self.crusade_answer(id, p, answer);
            }
        }
    }

    /// `p` answers crusade `id`'s call (a human through the session; an AI from
    /// `crusade_pledge`). False if `p` was not called or has answered already.
    pub fn crusade_answer(&mut self, id: CrusadeId, p: PlayerId, answer: Pledge) -> bool {
        let Some(idx) = self.crusade_index(id) else { return false };
        let turn = self.game_turn();
        let Some(call) = self.crusades[idx].calls.iter_mut().find(|c| c.player == p) else { return false };
        if call.answer.is_some() || call.settled {
            return false;
        }
        let answer = match answer {
            Pledge::Strength(s) if s <= 0 => Pledge::Refuse,
            a => a,
        };
        call.answer = Some(answer);
        self.log.msg(turn, Some(p), || match answer {
            Pledge::Strength(s) => format!("Crusade {}: pledged strength {}", id.0, strength_text(s)),
            Pledge::Refuse => format!("Crusade {}: refused the call", id.0),
        });
        true
    }

    /// Strength `p` has delivered to crusade `idx`.
    pub fn crusade_delivered(&self, idx: usize, p: PlayerId) -> i32 {
        self.crusade_lender(idx, p).map_or(0, |l| l.delivered)
    }

    /// The deadline: charge each called civ's standing for its shortfall.
    pub(crate) fn crusade_settle_calls(&mut self, idx: usize) {
        for i in 0..self.crusades[idx].calls.len() {
            if self.crusades[idx].calls[i].settled {
                continue;
            }
            self.crusades[idx].calls[i].settled = true;
            let call = self.crusades[idx].calls[i].clone();
            if !self.players[call.player].alive {
                continue;
            }
            let delivered = self.crusade_delivered(idx, call.player);
            if call_refused(&call, delivered) {
                self.crusade_charge_standing(idx, call.player, 1000);
            } else if delivered < call.obligation {
                let shortfall = ((call.obligation - delivered) as i64 * 1000 / call.obligation.max(1) as i64) as i32;
                self.crusade_charge_standing(idx, call.player, shortfall);
            } else {
                self.crusade_honor_call(idx, call.player);
            }
        }
    }

    /// Lost standing with the caller, `permille` of a full refusal.
    pub(crate) fn crusade_charge_standing(&mut self, idx: usize, p: PlayerId, permille: i32) {
        let CrusadeKind::Council(vote_source) = self.crusades[idx].kind else { return };
        let turn = self.game_turn();
        let id = self.crusades[idx].id;
        if permille >= 1000 {
            let data = VoteSelectionSubData { vote: VoteType(0), player: None, city: None, other_player: None, text: String::new() };
            self.player_set_defied_resolution(p, vote_source, &data);
            self.log.msg(turn, Some(p), || format!("Crusade {}: refused the call (defied the council)", id.0));
        } else if permille > 0 {
            // The shortfall's share of the defiance anger; membership is kept.
            let religion = self.vote_source_religions[vote_source];
            for c in self.players[p].cities.clone() {
                if religion.is_none_or(|r| self.cities[c].has_religion[r]) {
                    let length = self.city_flat_defy_resolution_anger_length(c) * permille / 1000;
                    if length > 0 {
                        self.city_change_defy_resolution_anger_timer(c, length);
                    }
                }
            }
            self.log.msg(turn, Some(p), || format!("Crusade {}: fell short of its obligation by {}%", id.0, permille / 10));
        }
    }

    /// The obligation met: loyal standing with the caller.
    fn crusade_honor_call(&mut self, idx: usize, p: PlayerId) {
        let CrusadeKind::Council(vote_source) = self.crusades[idx].kind else { return };
        let data = VoteSelectionSubData { vote: VoteType(0), player: None, city: None, other_player: None, text: String::new() };
        if self.player_is_voting_member(p, vote_source) {
            self.player_set_endorsed_resolution(p, vote_source, &data);
        }
    }
}

impl Game {
    /// A passed council war resolution against `target` (docs/CRUSADE.md, "Call sources"): the
    /// voting members that could fight it (at war with the target already, or able to declare)
    /// are called to a Host that fights until the term ends.
    pub fn crusade_call_council(&mut self, vote_source: VoteSourceType, target: TeamId) -> Option<CrusadeId> {
        let called: Vec<PlayerId> = self
            .civ_player_ids()
            .filter(|&p| {
                let team = self.players[p].team;
                self.players[p].alive
                    && team != target
                    && self.player_is_voting_member(p, vote_source)
                    && !self.player_is_crusade_host(p)
                    && !self.teams[target].vassal[team]
                    && (self.at_war(team, target) || self.team_can_declare_war(team, target))
            })
            .collect();
        let rules = self.crusade_rules();
        let (term, deadline) = (speed_turns(self, rules.term), speed_turns(self, rules.pledge_deadline));
        self.crusade_call(CrusadeKind::Council(vote_source), target, None, Objective::Term, term, deadline, &called)
    }
}
