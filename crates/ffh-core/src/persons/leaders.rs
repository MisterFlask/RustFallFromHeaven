//! A new House's leader (docs/PERSONS.md, "The third build", rule 3; part 2's splits draw the
//! same way, rule 4): drawn with the game RNG from (a) the minor leaders of the civilization that
//! lead no living player, else (b) the minor leaders of other civilizations of the liege's
//! alignment that lead no living player. None free: no House can be founded.

use ffh_data::*;

use super::*;

/// A minor leader's title: the first paragraph of their civilopedia text when it is a short
/// title (`The Auditor of Oaths`); `None` when the text starts with prose.
pub fn leader_title(d: &GameData, l: LeaderHeadType) -> Option<String> {
    let text = d.text.get(&d.leader_heads[l].base.civilopedia);
    let (first, rest) = text.split_once("[NEWLINE][NEWLINE]")?;
    let first = first.trim();
    if first.is_empty() || rest.is_empty() || first.len() > 60 || first.contains('.') {
        return None;
    }
    Some(first.to_string())
}

/// A leader's name.
pub fn leader_name(d: &GameData, l: LeaderHeadType) -> String {
    d.text.name(&d.leader_heads[l].base.description).to_string()
}

/// The leaders a new House's leader is drawn from ([`Game::house_leader_pool`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaderPool {
    /// The free leaders, in leader id order (empty: no House can be founded).
    pub leaders: Vec<LeaderHeadType>,
    /// They are the civilization's own minor leaders (a); else the other civilizations' of the
    /// liege's alignment (b).
    pub own: bool,
}

impl Game {
    /// Whether leader `l` leads a living player.
    fn leader_leads_living(&self, l: LeaderHeadType) -> bool {
        self.player_ids().any(|p| self.players[p].alive && self.player_get_leader_type(p) == Some(l))
    }

    /// The pool a new House of civilization `civ` under `liege` draws its leader from (pure, in
    /// leader id order): (a) `civ`'s minor leaders (`minor_leader_of`, `preferred_breakaway`) that
    /// lead no living player; else (b) the minor leaders of the other civilizations whose
    /// alignment is the liege's, leading no living player.
    pub fn house_leader_pool(&self, civ: CivilizationType, liege: PlayerId) -> LeaderPool {
        let d = self.d;
        let minor = |l: LeaderHeadType| d.leader_heads[l].preferred_breakaway && d.leader_heads[l].minor_leader_of.is_some() && !self.leader_leads_living(l);
        let own: Vec<LeaderHeadType> = d.leader_heads.keys().filter(|&l| minor(l) && d.leader_heads[l].minor_leader_of == Some(civ)).collect();
        if !own.is_empty() {
            return LeaderPool { leaders: own, own: true };
        }
        let alignment = self.players[liege].alignment;
        let others: Vec<LeaderHeadType> = match alignment {
            Some(a) => d.leader_heads.keys().filter(|&l| minor(l) && d.leader_heads[l].minor_leader_of != Some(civ) && d.leader_heads[l].alignment == Some(a)).collect(),
            None => Vec::new(),
        };
        LeaderPool { leaders: others, own: false }
    }

    /// Draws a new House's leader from [`Game::house_leader_pool`] with the game RNG (a pool of
    /// one needs no draw); `None` when no leader is free.
    pub fn house_leader_draw(&mut self, civ: CivilizationType, liege: PlayerId) -> Option<LeaderHeadType> {
        let pool = self.house_leader_pool(civ, liege).leaders;
        match pool.len() {
            0 => None,
            1 => Some(pool[0]),
            n => Some(pool[self.soren_rand_num(n as i32, "Houses: leader") as usize]),
        }
    }

    /// Where a founding's leader would come from, as the grant list says it (pure): `A new House:
    /// its leader is drawn from the Bannor's minor leaders.`, or from the fallback, `A new House:
    /// its leader is drawn from other civilizations' Good minor leaders.`; `None` when no leader is
    /// free.
    pub fn house_founding_line(&self, liege: PlayerId) -> Option<String> {
        let civ = self.player_get_civilization_type(liege)?;
        let pool = self.house_leader_pool(civ, liege);
        if pool.leaders.is_empty() {
            return None;
        }
        Some(if pool.own {
            format!("A new House: its leader is drawn from the {}'s minor leaders.", self.player_get_civilization_short_description(liege))
        } else {
            let alignment = self.players[liege].alignment.map(|a| self.d.text.name(&self.d.alignments[a].base.description).to_string()).unwrap_or_default();
            format!("A new House: its leader is drawn from other civilizations' {alignment} minor leaders.")
        })
    }
}
