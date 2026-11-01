//! The crusade turn, dissolution, the spoils draft and the return of the lent units
//! (docs/CRUSADE.md, "Spoils and the draft", "Dissolution and return").
//!
//! Each game turn (`CvGame::doTurn`, after the council votes) every crusade asks its unanswered
//! calls, settles them at the deadline, ends loans whose term ran out, and dissolves when its
//! objective is met, its term is over, its target is gone, or its Host has no units left.
//!
//! Dissolution, in order: (1) the Host's war ends; (2) the draft: the pool is every city the
//! Host still holds plus the gold it sacked; each lender's budget is its share of contribution
//! times the pool value (largest remainders, so the budgets sum to the pool); in descending share
//! (ties to the earliest first credit) each lender picks cities until it stops or its budget is
//! spent, and may overdraw by paying the difference from its treasury into the pool; (3) unspent
//! budgets are paid from the gold, pro rata when it is short (because cities went unclaimed);
//! unclaimed cities go to the victim, or free as independent (barbarian) cities; (4-5) every
//! surviving unit goes home to its lender's nearest city with what it earned, heroes too; (6) a
//! lender whose war began with its lending makes peace with the target (a standard peace treaty);
//! (7) the Host slot is freed. A loan general has no draft: its cities and gold go to the borrower.

use super::*;

/// Largest-remainder split of `total` in proportion to `weights` (sums to `total` when any
/// weight is positive; ties to the earlier entry).
pub fn split_by_weight(total: i64, weights: &[i64]) -> Vec<i64> {
    let sum: i64 = weights.iter().sum();
    if sum <= 0 || total <= 0 {
        return vec![0; weights.len()];
    }
    let mut shares: Vec<i64> = weights.iter().map(|&w| total * w / sum).collect();
    let mut rest = total - shares.iter().sum::<i64>();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(total * weights[i] % sum), i));
    for &i in &order {
        if rest <= 0 {
            break;
        }
        if weights[i] > 0 {
            shares[i] += 1;
            rest -= 1;
        }
    }
    shares
}

impl Game {
    /// `CvGame::doTurn` hook: every crusade's turn.
    pub fn crusade_do_turn(&mut self) {
        if self.crusades.is_empty() {
            return;
        }
        let ids: Vec<CrusadeId> = self.crusades.iter().map(|c| c.id).collect();
        for id in ids {
            let Some(idx) = self.crusade_index(id) else { continue };
            self.crusade_ask_pledges(idx);
            let turn = self.game_turn();
            if turn >= self.crusades[idx].deadline_turn {
                self.crusade_settle_calls(idx);
            }
            if let CrusadeKind::Loan { .. } = self.crusades[idx].kind {
                self.loan_do_turn(idx);
            }
            let Some(idx) = self.crusade_index(id) else { continue };
            if let Some(reason) = self.crusade_dissolution_due(idx) {
                self.crusade_dissolve(id, reason);
            }
        }
    }

    /// Should crusade `idx` dissolve now, and why?
    pub fn crusade_dissolution_due(&self, idx: usize) -> Option<Dissolution> {
        let c = &self.crusades[idx];
        let turn = self.game_turn();
        if let CrusadeKind::Loan { borrower } = c.kind {
            let bt = self.players[borrower].team;
            if !self.players[borrower].alive || !self.at_war(bt, c.target) || !c.lenders.iter().any(|l| l.term_end >= turn) {
                return Some(Dissolution::LoansEnded);
            }
        }
        let held_by_target = |plot: &PlotIdx| {
            self.map.plots[*plot].plot_city.is_some_and(|city| self.cities.contains(city) && self.crusade_is_enemy_team(idx, self.players[self.cities[city].owner].team))
        };
        let met = match &c.objective {
            Objective::Term => false,
            Objective::Capture(plots) => !plots.iter().any(held_by_target),
            Objective::Recover => turn > c.start_turn && !c.victim_plots.iter().any(held_by_target),
        };
        if met {
            return Some(Dissolution::Objective);
        }
        if !self.team_is_alive(c.target) {
            return Some(Dissolution::TargetGone);
        }
        if turn > c.end_turn {
            return Some(Dissolution::Term);
        }
        let units = self.players[c.host].units.iter().filter(|&&u| self.units.contains(u) && !self.units[u].death_delay).count();
        if units == 0 && (c.had_units || turn >= c.deadline_turn) {
            return Some(Dissolution::NoUnits);
        }
        None
    }

    /// Dissolve crusade `id` now (docs/CRUSADE.md, "Dissolution and return").
    pub fn crusade_dissolve(&mut self, id: CrusadeId, reason: Dissolution) {
        let Some(idx) = self.crusade_index(id) else { return };
        if self.crusades[idx].dissolving {
            return;
        }
        self.crusades[idx].dissolving = true;
        let turn = self.game_turn();
        if self.crusades[idx].calls.iter().any(|c| !c.settled) {
            self.crusade_settle_calls(idx);
        }
        let host = self.crusades[idx].host;
        let host_team = self.players[host].team;

        // 1. The Host stops fighting.
        for t in self.team_ids() {
            if t != host_team && self.teams[host_team].at_war[t] && !self.team_is_barbarian(t) {
                self.team_make_peace(host_team, t, true);
            }
        }

        // 2-3. Spoils.
        let mut report = CrusadeReport {
            id,
            kind: self.crusades[idx].kind,
            target: self.crusades[idx].target,
            start_turn: self.crusades[idx].start_turn,
            end_turn: turn,
            reason,
            pool: 0,
            wealth: 0,
            delivered: self.crusades[idx].lenders.iter().map(|l| l.delivered).sum(),
            engaged: self.crusades[idx].engaged,
            refusals: 0,
            lenders: Vec::new(),
        };
        for l in &self.crusades[idx].lenders {
            report.lenders.push(LenderReport {
                player: l.player,
                called: false,
                obligation: 0,
                refused: false,
                delivered: l.delivered,
                credit: l.credit,
                late_credit: 0,
                spoils: 0,
                cities: 0,
            });
        }
        for call in &self.crusades[idx].calls {
            let delivered = self.crusades[idx].lenders.iter().find(|l| l.player == call.player).map_or(0, |l| l.delivered);
            let refused = super::call::call_refused(call, delivered);
            if refused {
                report.refusals += 1;
            }
            match report.lenders.iter_mut().find(|r| r.player == call.player) {
                Some(r) => {
                    r.called = true;
                    r.obligation = call.obligation;
                    r.refused = refused;
                }
                None => report.lenders.push(LenderReport {
                    player: call.player,
                    called: true,
                    obligation: call.obligation,
                    refused,
                    delivered: 0,
                    credit: 0,
                    late_credit: 0,
                    spoils: 0,
                    cities: 0,
                }),
            }
        }
        // Final quarter of the crusade, for the bandwagon measure.
        let start = self.crusades[idx].start_turn;
        let late_from = start + (turn - start) * 3 / 4;
        for l in &self.crusades[idx].lenders {
            let late: i32 = l.credits.iter().filter(|&&(t, _)| t >= late_from && turn > start).map(|&(_, s)| s).sum();
            if let Some(r) = report.lenders.iter_mut().find(|r| r.player == l.player) {
                r.late_credit = late;
            }
        }

        match self.crusades[idx].kind {
            CrusadeKind::Loan { borrower } => self.loan_hand_over_spoils(idx, borrower, &mut report),
            _ => self.crusade_draft(idx, &mut report),
        }

        // 4-5. Survivors and heroes go home.
        let idx = self.crusade_index(id).expect("dissolving crusade");
        for u in self.players[host].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            self.crusade_send_home(u);
        }

        // 6. The lenders' crusade wars end.
        let lenders = self.crusades[idx].lenders.clone();
        for l in &lenders {
            if l.war_from_lending {
                self.crusade_end_lender_war(idx, l.player);
            }
        }

        // 7. The Host is deleted.
        self.crusade_free_host(host);
        let idx = self.crusade_index(id).expect("dissolving crusade");
        self.crusades.remove(idx);
        self.log.msg(turn, None, || format!("Crusade {} dissolved ({:?})", id.0, reason));
        self.crusade_reports.push(report);
    }

    /// A lender's war with the target, begun by its lending, ends with a standard peace treaty
    /// (unless its units still serve another Host against the target).
    pub(crate) fn crusade_end_lender_war(&mut self, idx: usize, p: PlayerId) {
        if !self.players[p].alive {
            return;
        }
        let team = self.players[p].team;
        let target = self.crusades[idx].target;
        if !self.at_war(team, target) || !self.team_is_alive(target) || self.team_is_a_vassal(team) {
            return;
        }
        let others_serving = (0..self.crusades.len()).any(|i| i != idx && !self.crusades[i].dissolving && self.crusades[i].target == target && !self.crusade_lent_units(i, Some(p)).is_empty());
        if others_serving {
            return;
        }
        let Some(other) = self.team_get_leader_id(target) else { return };
        self.player_force_peace(p, other);
    }

    /// Host unit `u` goes home: to its lender's nearest city with its promotions, or dies (no
    /// lender, or a lender without cities; units the Host gained otherwise).
    pub(crate) fn crusade_send_home(&mut self, u: UnitId) {
        let lender = self.units[u].lent.as_ref().map(|r| r.lender);
        let Some(lender) = lender.filter(|&l| self.players[l].alive && !self.players[l].cities.is_empty()) else {
            self.units[u].lent = None;
            self.unit_kill(u, false, None);
            return;
        };
        let (x, y) = (self.units[u].x, self.units[u].y);
        let mut best: Option<(i32, CityId)> = None;
        for &c in &self.players[lender].cities {
            let dist = self.plot_distance(x, y, self.cities[c].x, self.cities[c].y);
            if best.is_none_or(|(bd, bc)| (dist, c) < (bd, bc)) {
                best = Some((dist, c));
            }
        }
        let (_, city) = best.expect("lender has cities");
        let (cx, cy) = (self.cities[city].x, self.cities[city].y);
        self.crusade_change_owner(u, lender, Some((cx, cy)));
    }

    /// The draft of crusade `idx` (steps 2 and 3).
    fn crusade_draft(&mut self, idx: usize, report: &mut CrusadeReport) {
        let host = self.crusades[idx].host;
        let id = self.crusades[idx].id;
        let turn = self.game_turn();
        let cities: Vec<CityId> = self.players[host].cities.clone();
        let values: Vec<(CityId, i32)> = cities.iter().map(|&c| (c, self.crusade_city_value(c))).collect();
        let mut wealth = self.players[host].gold.max(0) as i64;
        self.players[host].gold = 0;
        let pool: i64 = values.iter().map(|&(_, v)| v as i64).sum::<i64>() + wealth;
        report.pool = pool as i32;
        report.wealth = wealth as i32;

        // Draft order: descending credit, ties to the earliest first credit (then slot order).
        let mut order: Vec<Lender> = self.crusades[idx].lenders.iter().filter(|l| l.credit > 0 && self.players[l.player].alive).cloned().collect();
        order.sort_by_key(|l| (std::cmp::Reverse(l.credit), l.first_credit_turn, l.player));
        let weights: Vec<i64> = order.iter().map(|l| l.credit as i64).collect();
        let budgets0 = split_by_weight(pool, &weights);
        let mut budgets = budgets0.clone();
        let mut available: Vec<(CityId, i32)> = values.clone();

        for (i, l) in order.iter().enumerate() {
            let p = l.player;
            loop {
                available.retain(|&(c, _)| self.cities.contains(c) && self.cities[c].owner == host);
                if available.is_empty() {
                    break;
                }
                let budget = budgets[i] as i32;
                let pick = self.controller(p).crusade_draft_pick(self, p, id, budget, &available);
                let Some(c) = pick else { break };
                let Some(&(_, value)) = available.iter().find(|&&(ac, _)| ac == c) else { break };
                let over = (value as i64 - budgets[i]).max(0);
                if over > 0 && (self.players[p].gold as i64) < over {
                    break; // cannot pay the overdraw: stop picking
                }
                if over > 0 {
                    self.players[p].gold -= over as i32;
                    wealth += over;
                    budgets[i] = 0;
                } else {
                    budgets[i] -= value as i64;
                }
                self.player_acquire_city(p, c, false, true, true);
                if let Some(r) = report.lenders.iter_mut().find(|r| r.player == p) {
                    r.cities += 1;
                    r.spoils += value - over as i32;
                }
                self.log.msg(turn, Some(p), || format!("Crusade {}: drafted a city worth {}", id.0, value));
            }
        }

        // Unspent budgets from the wealth, pro rata when short.
        let unspent: i64 = budgets.iter().sum();
        let pay: Vec<i64> = if unspent <= wealth { budgets.clone() } else { split_by_weight(wealth, &budgets) };
        for (i, l) in order.iter().enumerate() {
            if pay[i] > 0 {
                self.players[l.player].gold += pay[i] as i32;
                if let Some(r) = report.lenders.iter_mut().find(|r| r.player == l.player) {
                    r.spoils += pay[i] as i32;
                }
            }
        }
        // (budgets sum to the pool, so wealth left over is only possible without lenders)

        // Unclaimed cities: to the victim, else independent.
        let victim = self.crusades[idx].victim.filter(|&v| self.players[v].alive);
        for c in self.players[host].cities.clone() {
            if !self.cities.contains(c) {
                continue;
            }
            let to = victim.unwrap_or_else(|| self.barbarian_player());
            self.player_acquire_city(to, c, false, true, true);
        }
    }
}
