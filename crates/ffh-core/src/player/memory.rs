//! A player's memory of what others did to it (`Player::ai_memory_count`, C++ `CvPlayerAI`'s
//! `m_aaiMemoryCount`): counts by player and `MemoryType`, recorded by the rules where the act
//! happens, weighed by the AI (ffh-ai `diplomacy::relation_parts`) and fading with the leader's
//! decay rates (`LeaderHeadInfo::memory_decay_rand`).
//!
//! This game's rules (docs/PORTING.md, "Changes from FFH2"; the owner's decisions of 2026-11-18):
//! - The decay is a rule: [`Game::player_do_memory_decay`] runs in every player's turn (C++ ran
//!   it in `CvPlayerAI::AI_doCounter`, the AI's half of the turn).
//! - [`Game::player_add_memory`] records a wrong of this game's own (`MEMORY_TOOK_CITY`,
//!   `MEMORY_SPLIT_HOUSE`, `MEMORY_COURTED_HOUSE`). An act against a House angers the House and its liege
//!   (docs/PERSONS.md, "The second build"): the liege remembers it too. The ported increments
//!   of FFH2's memories stay where they were (a declared war is remembered by every player of the
//!   side declared on, a House pulled in included, by `team_declare_war` itself).

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

impl Game {
    /// Player `p` remembers `mem` of player `of` (+1 to `ai_memory_count[of][mem]`). Nothing when
    /// either is the barbarians, `p` is `of`, or they are of the same team. When `p` is a House,
    /// its liege remembers it as well (an act against a House angers the House and its liege),
    /// unless the liege is `of` or of its team.
    pub fn player_add_memory(&mut self, p: PlayerId, of: PlayerId, mem: MemoryType) {
        if !self.memory_counts(p, of) {
            return;
        }
        self.players[p].ai_memory_count[of][mem] += 1;
        if let Some(h) = self.player_house(p) {
            let liege = self.houses[h].owner;
            if liege != p && self.memory_counts(liege, of) {
                self.players[liege].ai_memory_count[of][mem] += 1;
            }
        }
    }

    /// Whether `p` can remember an act of `of` ([`Game::player_add_memory`]).
    fn memory_counts(&self, p: PlayerId, of: PlayerId) -> bool {
        p != of && !self.player_is_barbarian(p) && !self.player_is_barbarian(of) && self.players[p].team != self.players[of].team
    }

    /// The memory part of C++ `CvPlayerAI::AI_doCounter`, a rule here: for each other living
    /// player and each memory `p` holds of it, a chance of 1 in its leader's decay rate
    /// (`memory_decay_rand`, none when 0) that it fades by one. Draws the game RNG.
    pub fn player_do_memory_decay(&mut self, p: PlayerId) {
        let Some(leader) = self.players[p].personality_type else { return };
        let d = self.d;
        let decay = &d.leader_heads[leader].memory_decay_rand;
        for other in self.player_ids() {
            if !self.players[other].alive {
                continue;
            }
            for mem in (0..MemoryType::NUM).map(MemoryType::from_usize) {
                if self.players[p].ai_memory_count[other][mem] > 0 && decay[mem] > 0 && self.soren_rand_num(decay[mem], "Memory Decay") == 0 {
                    self.players[p].ai_memory_count[other][mem] -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 3, "WORLDSIZE_SMALL", 11));
        g.game_init();
        g
    }

    #[test]
    fn a_memory_fades_with_a_decay_and_never_without_one() {
        let mut g = game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let leader = g.players[p0].personality_type.expect("a leader");
        let rates = &g.d.leader_heads[leader].memory_decay_rand;
        assert_eq!(rates[MemoryType::MEMORY_DECLARED_WAR], 50);
        // A memory without a decay: MEMORY_DECLARED_WAR_ON_FRIEND (FFH2 gives none).
        assert_eq!(rates[MemoryType::MEMORY_DECLARED_WAR_ON_FRIEND], 0);
        g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_DECLARED_WAR] = 5;
        g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_DECLARED_WAR_ON_FRIEND] = 5;
        for _ in 0..1000 {
            g.player_do_memory_decay(p0);
        }
        // 1000 draws of 1 in 50: about 20 decays expected, 5 needed.
        assert_eq!(g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_DECLARED_WAR], 0, "faded");
        assert_eq!(g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_DECLARED_WAR_ON_FRIEND], 5, "no decay, no fading");
    }

    #[test]
    fn the_decay_falls_one_at_a_time_and_draws_only_for_held_memories() {
        let mut g = game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        // Nothing held (the start holds the barbarians' declarations: cleared): no draw.
        for row in g.players[p0].ai_memory_count.iter_mut() {
            for m in row.iter_mut() {
                *m = 0;
            }
        }
        let before = g.soren_rand.clone();
        g.player_do_memory_decay(p0);
        assert_eq!(g.soren_rand, before, "no memory, no draw");
        g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_TOOK_CITY] = 3;
        let mut last = 3;
        for _ in 0..2000 {
            g.player_do_memory_decay(p0);
            let now = g.players[p0].ai_memory_count[p1][MemoryType::MEMORY_TOOK_CITY];
            assert!(now == last || now == last - 1, "{last} -> {now}");
            last = now;
        }
        assert_eq!(last, 0);
    }

    #[test]
    fn a_memory_is_not_recorded_against_oneself_a_teammate_or_the_barbarians() {
        let mut g = game();
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let barbarian = g.barbarian_player();
        let m = MemoryType::MEMORY_TOOK_CITY;
        g.player_add_memory(p0, p0, m);
        g.player_add_memory(p0, barbarian, m);
        g.player_add_memory(barbarian, p0, m);
        assert_eq!(g.players[p0].ai_memory_count[p0][m], 0);
        assert_eq!(g.players[p0].ai_memory_count[barbarian][m], 0);
        assert_eq!(g.players[barbarian].ai_memory_count[p0][m], 0);
        g.player_add_memory(p0, p1, m);
        assert_eq!(g.players[p0].ai_memory_count[p1][m], 1);
        assert_eq!(g.players[p1].ai_memory_count[p0][m], 0, "one way");
    }
}
