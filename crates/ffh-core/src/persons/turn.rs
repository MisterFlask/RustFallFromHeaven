//! The Houses' turn (docs/PERSONS.md, "The second build" and "The third build").

use super::*;

impl Game {
    /// Player `p`'s Houses in its turn processing (`player_do_turn`, after the army sync): the
    /// House records checked, modifiers expired, contracts whose turns ran out ended, requests (met,
    /// impossible, lapsed, new), states and the ultimatums of the Furious (answered at once: a
    /// refusal breaks the House away); then the Houses' tribute and their splits; last, their
    /// favour toward the civilizations that court them, and a split toward a courter at the
    /// requirement (the fourth build, rules 2 to 5: `favour.rs`).
    pub fn player_persons_turn(&mut self, p: PlayerId) {
        if !self.player_has_houses(p) {
            return;
        }
        self.player_persons_houses(p);
        self.player_persons_expire(p);
        self.player_persons_contracts(p);
        self.player_persons_demands(p);
        self.player_persons_states(p);
        if !self.players[p].alive {
            return;
        }
        self.player_persons_tribute(p);
        self.player_persons_splits(p);
        // The fourth build, rules 2 to 5 (favour.rs).
        self.player_persons_favour(p);
    }

    /// Player `p`'s House records in its turn processing: a record whose player is dead or no
    /// longer its vassal ends (the vassal rules end them as it happens; this keeps the list true
    /// whatever ended it).
    pub(crate) fn player_persons_houses(&mut self, p: PlayerId) {
        let pt = self.players[p].team;
        for h in self.players[p].houses.clone() {
            let Some(house) = self.houses.get(h) else {
                self.players[p].houses.retain(|&o| o != h);
                continue;
            };
            let hp = house.player;
            if !self.players[hp].alive || !self.teams[self.players[hp].team].vassal[pt] {
                self.house_record_end(h);
            }
        }
    }
}
