//! The persons turn (docs/PERSONS.md, "The first build").

use super::*;

impl Game {
    /// Player `p`'s persons and Houses in its turn processing (`player_do_turn`, after the army
    /// sync): references to what is gone dropped, entry, empty seats filled, commanders by the
    /// rule, modifiers expired, demands (met, impossible, lapsed, new), states and countdowns;
    /// then the countdowns that ran out (secession: Houses, then commanders), the Disloyal heads'
    /// units sent to their Houses' largest cities, and the Houses' cities refreshed (their trait
    /// and maintenance).
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
        self.player_persons_held_heads(p);
        self.player_persons_cities(p);
    }
}
