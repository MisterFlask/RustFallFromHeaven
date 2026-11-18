//! The persons turn (docs/PERSONS.md, "The second build").

use super::*;

impl Game {
    /// Player `p`'s persons and Houses in its turn processing (`player_do_turn`, after the army
    /// sync): references to what is gone dropped, entry, the House records checked, commanders by
    /// the rule, modifiers expired, demands (met, impossible, lapsed, new), states and countdowns;
    /// then the countdowns that ran out (Houses break away, then commanders secede), and the
    /// Houses' tribute.
    pub fn player_persons_turn(&mut self, p: PlayerId) {
        if !self.player_has_persons(p) {
            return;
        }
        self.player_persons_drop_dead(p);
        self.player_persons_entry(p);
        self.player_persons_houses(p);
        self.player_persons_commanders(p);
        self.player_persons_expire(p);
        self.player_persons_demands(p);
        self.player_persons_states(p);
        self.player_persons_secessions(p);
        if !self.players[p].alive {
            return;
        }
        self.player_persons_tribute(p);
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
