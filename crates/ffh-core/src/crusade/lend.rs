//! Lending units to a Host, contribution credit, and the rules hooks that keep the books
//! (docs/CRUSADE.md, "Pledging and lending", "Contribution").
//!
//! - **Lending** hands units over where they stand: each becomes the Host's (`initUnit` +
//!   `convert`, keeping its name, experience, level and promotions) and remembers its lender and
//!   its strength at handover ([`LentRecord`] on the unit). A civ's first transfer declares war on
//!   the target through the normal declaration path; a civ that could not legally declare that
//!   war cannot lend. Lent units cannot be recalled.
//! - **Credit**: a lent unit adds its handover strength to its lender's contribution on its first
//!   kill of an enemy of the crusade or its death, whichever comes first, once.
//! - **Captured cities**: a defended victim's own cities return to it as soon as the Host takes
//!   them; a loan general's captures go to the borrower.
//! - **Heroes**: a lent immortal unit that dies escapes home to its lender (its usual rebirth, in
//!   the lender's capital), not back to the Host.

use super::*;

/// Why a loan of units was refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LendError {
    NoSuchCrusade,
    Dissolving,
    /// The lender is dead, is a Host, or is on the target's team (or is its vassal).
    NotAllowed,
    /// Not at war with the target and could not legally declare that war.
    CannotDeclareWar,
    /// A loan general takes units only from its lenders, within their agreed terms.
    NotALoanLender,
    /// The unit is not the lender's or cannot be lent.
    NotLendable(UnitId),
    NoUnits,
}

impl Game {
    /// Can `p` lend to crusade `id` at all?
    pub fn crusade_can_lend(&self, id: CrusadeId, p: PlayerId) -> Result<(), LendError> {
        let idx = self.crusade_index(id).ok_or(LendError::NoSuchCrusade)?;
        let c = &self.crusades[idx];
        if c.dissolving {
            return Err(LendError::Dissolving);
        }
        let team = self.players[p].team;
        if !self.players[p].alive || self.player_is_crusade_host(p) || self.player_is_barbarian(p) || self.crusade_is_enemy_team(idx, team) {
            return Err(LendError::NotAllowed);
        }
        if self.teams[c.target].vassal[team] || self.teams[team].vassal[c.target] {
            return Err(LendError::NotAllowed);
        }
        if let CrusadeKind::Loan { .. } = c.kind {
            // A loan general serves only the lenders of agreed loans (`loan_agree`).
            if !c.lenders.iter().any(|l| l.player == p && l.term_end >= self.game_turn()) {
                return Err(LendError::NotALoanLender);
            }
        }
        if !self.at_war(team, c.target) && !self.team_can_declare_war(team, c.target) {
            return Err(LendError::CannotDeclareWar);
        }
        Ok(())
    }

    /// Lend `units` (all `p`'s) to crusade `id`. All or nothing; returns the strength handed over.
    pub fn crusade_lend(&mut self, id: CrusadeId, p: PlayerId, units: &[UnitId]) -> Result<i32, LendError> {
        self.crusade_can_lend(id, p)?;
        if units.is_empty() {
            return Err(LendError::NoUnits);
        }
        for &u in units {
            if !self.units.contains(u) || self.units[u].owner != p || !self.crusade_unit_lendable(u) {
                return Err(LendError::NotLendable(u));
            }
        }
        let idx = self.crusade_index(id).expect("checked");
        let turn = self.game_turn();
        let team = self.players[p].team;
        let target = self.crusades[idx].target;

        // Lending is war: the first transfer declares it through the normal path.
        let war_from_lending = !self.at_war(team, target);
        if war_from_lending {
            self.team_declare_war(team, target, false, Some(WarPlanType::WARPLAN_TOTAL));
            if !self.at_war(team, target) {
                return Err(LendError::CannotDeclareWar);
            }
        }
        // (the declaration may have changed what the crusade's enemies are)
        let idx = self.crusade_index(id).expect("crusade still running");
        self.crusade_grant_passage(idx, team);

        if !self.crusades[idx].lenders.iter().any(|l| l.player == p) {
            self.crusades[idx].lenders.push(Lender {
                player: p,
                delivered: 0,
                units: 0,
                credit: 0,
                first_credit_turn: -1,
                first_lend_turn: turn,
                war_from_lending,
                term_end: -1,
                credits: Vec::new(),
            });
        }
        let mut total = 0;
        for &u in units {
            if !self.units.contains(u) {
                continue;
            }
            let strength = self.crusade_unit_strength(u);
            self.crusade_transfer_to_host(idx, p, u, strength);
            total += strength;
        }
        let lender = self.crusades[idx].lenders.iter_mut().find(|l| l.player == p).expect("lender entry");
        lender.delivered += total;
        lender.units += units.len() as i32;
        self.crusades[idx].had_units = true;
        let n = units.len();
        self.log.msg(turn, Some(p), || format!("Crusade {}: lent {} units (strength {})", id.0, n, total));
        Ok(total)
    }

    /// Unit `u` of `lender` becomes the Host's, where it stands.
    fn crusade_transfer_to_host(&mut self, idx: usize, lender: PlayerId, u: UnitId, strength: i32) -> UnitId {
        let host = self.crusades[idx].host;
        let id = self.crusades[idx].id;
        let new = self.crusade_change_owner(u, host, None);
        self.units[new].lent = Some(LentRecord { crusade: id, lender, strength, turn: self.game_turn(), credited: false, engaged: false });
        new
    }

    /// Unit `u` changes owner to `to` where it stands, or at `at` (`initUnit` + `convert`, the
    /// experience kept exactly); the old unit is gone at once. Returns the new unit.
    pub(crate) fn crusade_change_owner(&mut self, u: UnitId, to: PlayerId, at: Option<(i32, i32)>) -> UnitId {
        let (unit_type, ai) = (self.units[u].unit_type, self.units[u].ai_type);
        let (x, y) = at.unwrap_or((self.units[u].x, self.units[u].y));
        let experience = self.units[u].experience;
        // The record moves explicitly (see `crusade_on_convert`); the old unit must not be
        // credited for "dying" here.
        self.units[u].lent = None;
        let new = self.player_init_unit(to, unit_type, x, y, ai, None);
        self.unit_convert(new, u);
        self.unit_set_experience(new, experience, -1);
        if self.units.contains(u) {
            self.unit_kill(u, false, None);
        }
        new
    }

    /// `CvUnit::convert` hook: a Host unit converted into another of the Host's (an upgrade, an
    /// FfH transformation) keeps its record.
    pub(crate) fn crusade_on_convert(&mut self, new: UnitId, from: UnitId) {
        if self.units[from].lent.is_some() && self.units[new].owner == self.units[from].owner {
            self.units[new].lent = self.units[from].lent.take();
        }
    }

    /// Credit lent unit `u`'s handover strength to its lender (once).
    fn crusade_credit(&mut self, u: UnitId) {
        let Some(rec) = self.units[u].lent.clone() else { return };
        if rec.credited {
            return;
        }
        let turn = self.game_turn();
        if let Some(r) = self.units[u].lent.as_mut() {
            r.credited = true;
        }
        self.crusade_mark_engaged(u);
        let Some(idx) = self.crusade_index(rec.crusade) else { return };
        if let Some(l) = self.crusades[idx].lenders.iter_mut().find(|l| l.player == rec.lender) {
            l.credit += rec.strength;
            if l.first_credit_turn < 0 {
                l.first_credit_turn = turn;
            }
            l.credits.push((rec.turn, rec.strength));
        }
    }

    /// Engagement measure: a lent unit fought (or died) within `engage_turns` of delivery.
    fn crusade_mark_engaged(&mut self, u: UnitId) {
        let Some(rec) = self.units[u].lent.clone() else { return };
        if rec.engaged {
            return;
        }
        if let Some(r) = self.units[u].lent.as_mut() {
            r.engaged = true;
        }
        let window = speed_turns(self, self.crusade_rules().engage_turns);
        if self.game_turn() - rec.turn <= window {
            if let Some(idx) = self.crusade_index(rec.crusade) {
                self.crusades[idx].engaged += rec.strength;
            }
        }
    }

    /// Combat hook (`CvUnit::updateCombat`, at the end of a fight): lent units fought; a lent
    /// winner that killed an enemy of its crusade is credited.
    pub(crate) fn crusade_on_combat(&mut self, attacker: UnitId, defender: UnitId, attacker_dead: bool, defender_dead: bool) {
        if self.crusades.is_empty() {
            return;
        }
        for (u, other, other_dead) in [(attacker, defender, defender_dead), (defender, attacker, attacker_dead)] {
            if !self.units.contains(u) {
                continue;
            }
            let Some(rec) = self.units[u].lent.clone() else { continue };
            self.crusade_mark_engaged(u);
            if other_dead && self.units.contains(other) {
                let other_team = self.unit_get_team(other);
                if let Some(idx) = self.crusade_index(rec.crusade) {
                    if self.crusade_is_enemy_team(idx, other_team) {
                        self.crusade_credit(u);
                    }
                }
            }
        }
    }

    /// `CvUnit::kill` hook: a lent unit's death is credited.
    pub(crate) fn crusade_on_unit_killed(&mut self, u: UnitId) {
        if self.units[u].lent.as_ref().is_some_and(|r| !r.credited) {
            self.crusade_credit(u);
        }
    }

    /// `CvUnit::kill` hook for an immortal unit: a lent hero escapes home to its lender (its
    /// usual rebirth, in the lender's capital). True if it did (the Host's unit is gone).
    pub(crate) fn crusade_hero_escape(&mut self, u: UnitId) -> bool {
        let Some(rec) = self.units[u].lent.clone() else { return false };
        let lender = rec.lender;
        if !self.players[lender].alive || self.player_get_capital_city(lender).is_none() {
            return false;
        }
        self.crusade_credit(u);
        self.units[u].death_delay = false;
        let home = self.crusade_change_owner(u, lender, None);
        self.unit_do_immortal_rebirth(home);
        let turn = self.game_turn();
        self.log.msg(turn, Some(lender), || format!("Crusade {}: a lent hero escaped home", rec.crusade.0));
        true
    }

    /// `CvPlayer::acquireCity` hook (after the acquisition): the victim's cities return to it
    /// when the Host retakes them, a loan general's captures go to its borrower, and the victim's
    /// losses to the target are remembered (Recover).
    pub(crate) fn crusade_after_city_acquired(&mut self, old_owner: PlayerId, new_owner: PlayerId, plot: PlotIdx, conquest: bool) {
        if self.crusades.is_empty() {
            return;
        }
        let new_team = self.players[new_owner].team;
        for idx in 0..self.crusades.len() {
            let Some(victim) = self.crusades[idx].victim else { continue };
            if old_owner == victim && self.crusade_is_enemy_team(idx, new_team) && !self.crusades[idx].victim_plots.contains(&plot) {
                self.crusades[idx].victim_plots.push(plot);
            }
        }
        let Some(idx) = self.crusade_of_host(new_owner) else { return };
        if self.crusades[idx].dissolving || !conquest {
            return;
        }
        let Some(c) = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == new_owner) else { return };
        let Some(victim) = self.crusades[idx].victim.filter(|&v| self.players[v].alive) else { return };
        let loan = matches!(self.crusades[idx].kind, CrusadeKind::Loan { .. });
        if loan || self.crusade_is_victim_city(idx, c) {
            self.player_acquire_city(victim, c, false, true, true);
            let turn = self.game_turn();
            let id = self.crusades[idx].id;
            self.log.msg(turn, Some(victim), || format!("Crusade {}: a city taken by the Host was handed over", id.0));
        }
    }
}
