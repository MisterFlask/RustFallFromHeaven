//! Allied loans (docs/CRUSADE.md, "Allied loans"): a small, time-limited crusade made by
//! agreement. A lender lends units to an ally's war; they serve under a dedicated AI general (a
//! Host) that neither civ controls, until the lender's term runs out.
//!
//! - **The proposal** names the terms (strength, mission, duration) and, like any deal, may be
//!   paired with payment or other items either way (`lender_gives` / `borrower_gives`, ordinary
//!   trade items implemented as a deal when the loan is agreed). Either side proposes; the other
//!   side's controller answers (`consider_loan`): accept, refuse, or a counter-offer, which the
//!   proposer may accept by agreeing to it (`loan_agree`).
//! - **The lender chooses** which of its units make up the agreed strength (`choose_loan_units`).
//! - **One general per borrower and target**: loans from several allies against the same enemy
//!   serve under one general and fight together; each lender's units leave when its own term
//!   ends.
//! - Like a crusade: the units are locked for the term, lending declares war on the target, and
//!   at the end the survivors go straight home with their promotions (the lender's war, if it
//!   began with the loan, ends with a peace treaty).
//! - Unlike a crusade: no draft (the general's captures go to the borrower at once, its gold at
//!   the end). The price is whatever the two sides put in the deal, as in any trade: gold, gold
//!   per turn, technologies, resources, ... An ongoing payment (gold per turn, a resource) runs
//!   as long as the loan: its deal ends when the loan does ([`Lender::deals`]).
//! - **Ending early**: the borrower makes peace with the target, the target is eliminated, the
//!   lender and the borrower go to war with each other, or the ongoing payment is cancelled (its
//!   deal ended by either side, or by the rules when it can no longer be paid).

use super::lend::LendError;
use super::*;
use crate::structs::TradeData;

/// What the general does with the units.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoanMission {
    /// Attack one of the borrower's current enemies.
    Attack,
    /// Defend the borrower: recover and hold its cities.
    Defend,
}

/// The terms of a loan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoanTerms {
    pub lender: PlayerId,
    pub borrower: PlayerId,
    /// The borrower's enemy the general fights.
    pub target: TeamId,
    pub mission: LoanMission,
    /// Combat strength lent ([`Game::crusade_unit_strength`] units).
    pub strength: i32,
    /// Turns the units serve.
    pub duration: i32,
}

/// A loan offered on the diplomacy table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoanProposal {
    pub terms: LoanTerms,
    /// Who proposes (the lender offering, or the borrower asking).
    pub from: PlayerId,
    /// Items the lender gives the borrower with the loan.
    pub lender_gives: Vec<TradeData>,
    /// Items the borrower gives the lender (the payment).
    pub borrower_gives: Vec<TradeData>,
}

/// The other side's answer to a loan proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoanAnswer {
    Accept,
    /// Changed terms or payment the answering side would agree to.
    Counter(LoanProposal),
    Refuse,
}

/// Why a loan cannot be made.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoanError {
    /// Crusades (and loans) are off in this game.
    Off,
    /// Same team, dead, a Host, no contact, or the lender is at war with the borrower.
    NotAllowed,
    /// The borrower is not at war with the target.
    NotAtWar,
    /// The terms are out of range (strength, duration, target).
    BadTerms,
    /// The lender cannot lend against the target.
    CannotLend(LendError),
    /// The lender's lendable units fall short of the strength.
    NotEnoughUnits,
    /// A payment item cannot be traded.
    BadPayment,
    /// No Host slot is free for a new general.
    NoGeneral,
}

impl Game {
    /// Could the loan `t` be made now?
    pub fn loan_check(&self, t: &LoanTerms) -> Result<(), LoanError> {
        if !self.crusades_enabled() {
            return Err(LoanError::Off);
        }
        let (lt, bt) = (self.players[t.lender].team, self.players[t.borrower].team);
        if lt == bt || !self.players[t.lender].alive || !self.players[t.borrower].alive {
            return Err(LoanError::NotAllowed);
        }
        if self.player_is_crusade_host(t.lender) || self.player_is_crusade_host(t.borrower) || self.player_is_barbarian(t.lender) || self.player_is_barbarian(t.borrower) {
            return Err(LoanError::NotAllowed);
        }
        if !self.teams[lt].has_met[bt] || self.at_war(lt, bt) {
            return Err(LoanError::NotAllowed);
        }
        if t.strength <= 0 || t.duration <= 0 || t.duration > speed_turns(self, self.crusade_rules().loan_max_turns) {
            return Err(LoanError::BadTerms);
        }
        if t.target == lt || t.target == bt || self.team_is_barbarian(t.target) || !self.team_is_alive(t.target) || self.team_is_crusade_host(t.target) {
            return Err(LoanError::BadTerms);
        }
        if !self.at_war(bt, t.target) {
            return Err(LoanError::NotAtWar);
        }
        if self.teams[t.target].vassal[lt] || self.teams[lt].vassal[t.target] {
            return Err(LoanError::CannotLend(LendError::NotAllowed));
        }
        if !self.at_war(lt, t.target) && !self.team_can_declare_war(lt, t.target) {
            return Err(LoanError::CannotLend(LendError::CannotDeclareWar));
        }
        if self.crusade_military_strength(t.lender) < t.strength {
            return Err(LoanError::NotEnoughUnits);
        }
        if self.loan_general(t.borrower, t.target).is_none() && self.crusade_free_host_slot().is_none() {
            return Err(LoanError::NoGeneral);
        }
        Ok(())
    }

    /// The general serving `borrower`'s war against `target`, if one is running.
    pub fn loan_general(&self, borrower: PlayerId, target: TeamId) -> Option<CrusadeId> {
        self.crusades
            .iter()
            .find(|c| !c.dissolving && c.target == target && c.kind == CrusadeKind::Loan { borrower })
            .map(|c| c.id)
    }

    /// Strongest-first pick of `p`'s lendable units reaching `strength` (the default
    /// `choose_loan_units`): the units nearest `near` first among equals.
    pub fn loan_default_units(&self, p: PlayerId, strength: i32) -> Vec<UnitId> {
        let mut units: Vec<(i32, UnitId)> = self.players[p]
            .units
            .iter()
            .copied()
            .filter(|&u| self.units.contains(u) && self.crusade_unit_lendable(u))
            .map(|u| (self.crusade_unit_strength(u), u))
            .collect();
        units.sort_by_key(|&(s, u)| (std::cmp::Reverse(s), u));
        let mut out = Vec::new();
        let mut sum = 0;
        for (s, u) in units {
            if sum >= strength {
                break;
            }
            out.push(u);
            sum += s;
        }
        out
    }

    /// Offer loan `proposal` to the other side, whose controller answers. On `Accept` the loan is
    /// made (`Ok((Accept, Some(general)))`); a counter-offer or refusal changes nothing.
    pub fn loan_propose(&mut self, proposal: &LoanProposal) -> Result<(LoanAnswer, Option<CrusadeId>), LoanError> {
        let t = &proposal.terms;
        self.loan_check(t)?;
        if proposal.from != t.lender && proposal.from != t.borrower {
            return Err(LoanError::NotAllowed);
        }
        let other = if proposal.from == t.lender { t.borrower } else { t.lender };
        let answer = self.controller(other).consider_loan(self, other, proposal);
        if answer == LoanAnswer::Accept {
            let id = self.loan_agree(proposal)?;
            return Ok((answer, Some(id)));
        }
        Ok((answer, None))
    }

    /// Make the loan both sides agreed to: payment, the lender's units (its controller's
    /// choice, at least the agreed strength), the general (joined if one serves this borrower
    /// against this target).
    pub fn loan_agree(&mut self, proposal: &LoanProposal) -> Result<CrusadeId, LoanError> {
        self.loan_agree_with(proposal, None)
    }

    /// [`Game::loan_agree`] with the lender's units named (a human lender picks them; at least
    /// the agreed strength, all lendable, else `NotEnoughUnits`). `None` asks the lender's
    /// controller.
    pub fn loan_agree_with(&mut self, proposal: &LoanProposal, named: Option<&[UnitId]>) -> Result<CrusadeId, LoanError> {
        let t = proposal.terms.clone();
        self.loan_check(&t)?;
        for item in &proposal.lender_gives {
            if !self.player_can_trade_item(t.lender, t.borrower, item, false) {
                return Err(LoanError::BadPayment);
            }
        }
        for item in &proposal.borrower_gives {
            if !self.player_can_trade_item(t.borrower, t.lender, item, false) {
                return Err(LoanError::BadPayment);
            }
        }
        let mut units: Vec<UnitId> = match named {
            Some(v) => v.to_vec(),
            None => self.controller(t.lender).choose_loan_units(self, t.lender, &t),
        };
        units.retain(|&u| self.units.contains(u) && self.units[u].owner == t.lender && self.crusade_unit_lendable(u));
        units.sort();
        units.dedup();
        let chosen: i32 = units.iter().map(|&u| self.crusade_unit_strength(u)).sum();
        if chosen < t.strength && named.is_none() {
            units = self.loan_default_units(t.lender, t.strength);
        }
        let chosen: i32 = units.iter().map(|&u| self.crusade_unit_strength(u)).sum();
        if units.is_empty() || chosen < t.strength {
            return Err(LoanError::NotEnoughUnits);
        }

        let turn = self.game_turn();
        let id = match self.loan_general(t.borrower, t.target) {
            Some(id) => id,
            None => self
                .crusade_call(CrusadeKind::Loan { borrower: t.borrower }, t.target, Some(t.borrower), Objective::Term, t.duration, 0, &[])
                .ok_or(LoanError::NoGeneral)?,
        };
        let idx = self.crusade_index(id).expect("general");
        let end = turn + t.duration;
        self.crusades[idx].end_turn = self.crusades[idx].end_turn.max(end);
        let war_from_lending = !self.at_war(self.players[t.lender].team, t.target);
        match self.crusades[idx].lenders.iter_mut().find(|l| l.player == t.lender) {
            Some(l) => l.term_end = l.term_end.max(end),
            None => self.crusades[idx].lenders.push(Lender {
                player: t.lender,
                delivered: 0,
                units: 0,
                credit: 0,
                first_credit_turn: -1,
                first_lend_turn: turn,
                war_from_lending,
                term_end: end,
                credits: Vec::new(),
                deals: Vec::new(),
            }),
        }
        if t.mission == LoanMission::Defend {
            self.crusades[idx].defend = true;
        }
        if let Err(e) = self.crusade_lend(id, t.lender, &units) {
            // (cannot happen after the checks above; undo the empty entry)
            if let Some(idx) = self.crusade_index(id) {
                self.crusades[idx].lenders.retain(|l| l.player != t.lender || l.units > 0);
            }
            return Err(LoanError::CannotLend(e));
        }
        if !proposal.lender_gives.is_empty() || !proposal.borrower_gives.is_empty() {
            // Deal ids are never reused: the deal still standing after the items are implemented
            // that was not there before carries the ongoing payment (one-off items leave none).
            let before = self.deals.ids();
            self.implement_deal(t.lender, t.borrower, &proposal.lender_gives, &proposal.borrower_gives, false);
            let new: Vec<DealId> = self.deals.ids().into_iter().filter(|d| !before.contains(d)).collect();
            if let Some(idx) = self.crusade_index(id) {
                if let Some(l) = self.crusades[idx].lenders.iter_mut().find(|l| l.player == t.lender) {
                    l.deals.extend(new);
                }
            }
        }
        self.log.msg(turn, Some(t.borrower), || format!("Loan: player {} lends strength {} for {} turns against team {}", t.lender, strength_text(chosen), t.duration, t.target));
        Ok(id)
    }

    /// Does the general `id` defend its borrower (a Defend loan serves under it)?
    pub fn loan_is_defense(&self, id: CrusadeId) -> bool {
        self.crusade(id).is_some_and(|c| c.defend)
    }

    /// A loan general's turn: lenders whose term ended (or whose loan ended early: war between
    /// them, the payment cancelled) take their units home.
    pub(crate) fn loan_do_turn(&mut self, idx: usize) {
        let CrusadeKind::Loan { borrower } = self.crusades[idx].kind else { return };
        let turn = self.game_turn();
        let bt = self.players[borrower].team;
        for i in 0..self.crusades[idx].lenders.len() {
            let l = self.crusades[idx].lenders[i].clone();
            if l.term_end < turn && self.crusade_lent_units(idx, Some(l.player)).is_empty() {
                continue; // already over
            }
            let lt = self.players[l.player].team;
            let unpaid = l.deals.iter().any(|&d| !self.deals.contains(d));
            if l.term_end < turn || !self.players[l.player].alive || self.at_war(lt, bt) || unpaid {
                if unpaid {
                    let id = self.crusades[idx].id;
                    self.log.msg(turn, Some(l.player), || format!("Loan to general {}: the payment was cancelled, the loan ends", id.0));
                }
                self.loan_end(idx, l.player);
            }
        }
    }

    /// The ongoing payment of `lender`'s loan under general `idx` ends with the loan.
    pub(crate) fn loan_end_payments(&mut self, idx: usize, lender: PlayerId) {
        let Some(l) = self.crusades[idx].lenders.iter_mut().find(|l| l.player == lender) else { return };
        for d in std::mem::take(&mut l.deals) {
            if self.deals.contains(d) {
                self.deal_kill(d, true);
            }
        }
    }

    /// Loan of `lender` under general `idx` ends: its units go home, its payment stops, its loan
    /// war ends.
    fn loan_end(&mut self, idx: usize, lender: PlayerId) {
        let turn = self.game_turn();
        for u in self.crusade_lent_units(idx, Some(lender)) {
            if self.units.contains(u) {
                self.crusade_send_home(u);
            }
        }
        self.loan_end_payments(idx, lender);
        let l = self.crusades[idx].lenders.iter_mut().find(|l| l.player == lender).expect("lender");
        l.term_end = l.term_end.min(turn - 1);
        let war_from_lending = l.war_from_lending;
        l.war_from_lending = false;
        if war_from_lending {
            self.crusade_end_lender_war(idx, lender);
        }
        let id = self.crusades[idx].id;
        self.log.msg(turn, Some(lender), || format!("Loan to general {} ended", id.0));
    }

    /// Dissolution of a loan general: its cities and gold go to the borrower.
    pub(crate) fn loan_hand_over_spoils(&mut self, idx: usize, borrower: PlayerId, report: &mut CrusadeReport) {
        let host = self.crusades[idx].host;
        let gold = self.players[host].gold.max(0);
        self.players[host].gold = 0;
        report.wealth = gold;
        let to = if self.players[borrower].alive { borrower } else { self.barbarian_player() };
        if self.players[borrower].alive {
            self.players[borrower].gold += gold;
        }
        for c in self.players[host].cities.clone() {
            if self.cities.contains(c) {
                report.pool += self.crusade_city_value(c);
                self.player_acquire_city(to, c, false, true, true);
            }
        }
    }
}
