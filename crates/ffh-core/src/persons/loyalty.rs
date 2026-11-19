//! A House's loyalty: the modifiers, the sum, the five states (the attitude scale) and the Furious
//! House's ultimatum (docs/PERSONS.md, "The second build", rule 5; "The third build", rules 5 and 6:
//! the ultimatum replaced the countdown, and a contract running floors the loyalty at Annoyed).
//!
//! The sum is the modifiers still running plus the standing parts: `State religion opposed: -2`
//! (the state religion's alignment the opposite of the House's leader's) and `Tribute N%: 1 -
//! N/10`. The tribute part is the rate in force's (`House::tribute_now`), which moves slowly toward
//! the rate the liege set (rule 4, `vassals.rs`). (The second build's size part, `Holds N cities,
//! more than T`, went with the third build, rule 1.)

use ffh_data::*;

use super::*;

/// The standing loyalty part of a tribute of `pct` percent: `1 - pct / 10` (0%: +1, 20%: -1,
/// 50%: -4).
pub fn tribute_part(pct: i32) -> i32 {
    1 - pct / 10
}

impl Game {
    /// The standing part `State religion opposed` for House `h` under state religion `religion`:
    /// [`RELIGION_OPPOSED`] when the religion's alignment is the opposite of its leader's.
    pub(crate) fn house_religion_part_under(&self, h: HouseId, religion: Option<ReligionType>) -> i32 {
        let Some(r) = religion else { return 0 };
        if self.alignments_opposed(self.d.religions[r].alignment, self.house_alignment(h)) {
            RELIGION_OPPOSED
        } else {
            0
        }
    }

    /// House `h`'s standing religion part under its liege's state religion.
    pub(crate) fn house_religion_part(&self, h: HouseId) -> i32 {
        let religion = self.player_get_state_religion(self.houses[h].owner);
        self.house_religion_part_under(h, religion)
    }

    /// The sum of House `h`'s running modifiers (without the standing parts).
    pub(crate) fn house_mods_sum(&self, h: HouseId) -> i32 {
        let turn = self.game_turn();
        self.houses[h].standing.mods.iter().filter(|m| m.until.map_or(true, |u| u > turn)).map(|m| m.amount).sum()
    }

    /// House `h`'s loyalty as its parts make it: its running modifiers plus the standing parts (its
    /// state religion part and its tribute part), without a contract's floor.
    pub fn house_loyalty_raw(&self, h: HouseId) -> i32 {
        self.house_mods_sum(h) + self.house_religion_part(h) + tribute_part(self.houses[h].tribute_now)
    }

    /// Loyalty `total` of House `h` as every rule counts it: at least [`CONTRACT_FLOOR`] (Annoyed)
    /// while its contract runs (the third build, rule 6).
    pub fn house_loyalty_floored(&self, h: HouseId, total: i32) -> i32 {
        if self.house_contract_runs(h) {
            total.max(CONTRACT_FLOOR)
        } else {
            total
        }
    }

    /// House `h`'s loyalty: its running modifiers plus the standing parts (its state religion part
    /// and its tribute part), at least Annoyed while a contract runs.
    pub fn house_loyalty(&self, h: HouseId) -> i32 {
        self.house_loyalty_floored(h, self.house_loyalty_raw(h))
    }

    /// House `h`'s state: its attitude toward its liege.
    pub fn house_state(&self, h: HouseId) -> LoyaltyState {
        LoyaltyState::of(self.house_loyalty(h))
    }

    /// House `h`'s loyalty once a tribute of `pct` percent is in force (the steady state: the
    /// tribute selector's preview and the AI's): its standing part at `pct` in place of the rate
    /// in force's, the rest as it is now. How long until then: [`Game::house_tribute_turns`].
    pub fn house_loyalty_at_tribute(&self, h: HouseId, pct: i32) -> i32 {
        let now = self.houses[h].tribute_now;
        self.house_loyalty_floored(h, self.house_loyalty_raw(h) - tribute_part(now) + tribute_part(pct))
    }

    /// Adds a modifier `(amount, turns)` with `cause` to House `h`.
    pub(crate) fn house_add_mod(&mut self, h: HouseId, cause: LoyaltyCause, (amount, turns): (i32, i32)) {
        let until = Some(self.game_turn() + turns);
        self.houses[h].standing.mods.push(LoyaltyMod { cause, amount, until });
    }

    /// Player `p`'s Houses that stand, in its list order.
    pub(crate) fn player_houses(&self, p: PlayerId) -> Vec<HouseId> {
        self.players[p].houses.iter().copied().filter(|&h| self.houses.contains(h)).collect()
    }

    /// Modifiers whose turns ran out end (in player `p`'s turn processing).
    pub(crate) fn player_persons_expire(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for h in self.player_houses(p) {
            self.houses[h].standing.mods.retain(|m| m.until.map_or(true, |u| u > turn));
        }
    }

    /// The states of player `p`'s Houses: a change is told; a House Furious with no contract
    /// running issues an ultimatum, which its liege answers at once (the third build, rule 5:
    /// `ultimatums.rs`; a refusal is its break-away).
    pub(crate) fn player_persons_states(&mut self, p: PlayerId) {
        for h in self.player_houses(p) {
            if !self.houses.contains(h) || !self.players[p].alive {
                continue;
            }
            let state = self.house_state(h);
            let old = self.houses[h].standing.state;
            if state != old {
                self.houses[h].standing.state = state;
                self.tel_house(p, "state", state.name());
                let label = self.house_label(h);
                let text = format!("{label} is {}.", state.name());
                self.house_msg(h, || text);
            }
            if state == LoyaltyState::Furious && !self.house_contract_runs(h) {
                self.house_issue_ultimatum(h);
            }
        }
    }
}
