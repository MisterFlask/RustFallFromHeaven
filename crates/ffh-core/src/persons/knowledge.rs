//! A realm shares its knowledge (docs/PERSONS.md, "The third build", rule 10).
//!
//! A realm is a liege's team and the teams of its Houses ([`Game::team_realm`]). A tech one team of
//! a realm gains by any means (`team_set_has_tech` with `value: true`), every other team of the
//! realm gains too, in team id order, as not its first discoverer (no first-discovery effects:
//! religions, a free great person, free techs) ([`Game::team_share_tech`]). A team that becomes a
//! House of a realm (the vassal rules) and the realm share what they know at once, both ways
//! ([`Game::realm_share_knowledge`]). Not shared: a card of the deck (a tech of the data, one of a
//! kind: `TechInfo::card`) and a repeatable tech (it is never had, only counted). Techs, like war,
//! are the liege's to trade: a House trades none (`player_can_trade_item`, `bribes.rs`).

use ffh_data::*;

use super::*;

impl Game {
    /// The teams of team `t`'s realm, in team id order: its liege's team and the teams of the
    /// liege's Houses when a player of `t` is a House or has Houses; else none (no realm).
    pub fn team_realm(&self, t: TeamId) -> Vec<TeamId> {
        let liege = self.player_ids().find_map(|p| {
            if self.players[p].team != t || !self.players[p].alive {
                return None;
            }
            match self.player_liege(p) {
                Some(l) => Some(l),
                None => self.player_houses(p).first().map(|_| p),
            }
        });
        let Some(liege) = liege else { return Vec::new() };
        let mut out = vec![self.players[liege].team];
        for h in self.player_houses(liege) {
            let hp = self.houses[h].player;
            if self.players[hp].alive {
                out.push(self.players[hp].team);
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Whether `tech` is knowledge a realm shares: not a card of the deck, not repeatable.
    fn tech_is_shared(&self, tech: TechType) -> bool {
        let info = &self.d.techs[tech];
        info.card.is_none() && !info.repeat
    }

    /// Team `t` has just gained `tech` (`team_set_has_tech`): every other team of its realm that
    /// lacks it gains it, in team id order, as not its first discoverer. The teams reached gain it
    /// through `team_set_has_tech_one`, which shares nothing: no recursion.
    pub(crate) fn team_share_tech(&mut self, t: TeamId, tech: TechType, announce: bool) {
        if !self.tech_is_shared(tech) {
            return;
        }
        for o in self.team_realm(t) {
            if o != t && self.team_is_alive(o) && !self.team_is_has_tech(o, Some(tech)) {
                self.team_set_has_tech_one(o, tech, true, None, false, announce);
            }
        }
    }

    /// The realm of `liege` shares what it knows, both ways (a team has just joined it): each tech
    /// any team of the realm has, every team of the realm that lacks it gains, by tech then team id
    /// order, as not its first discoverer.
    pub(crate) fn realm_share_knowledge(&mut self, liege: PlayerId) {
        let teams = self.team_realm(self.players[liege].team);
        if teams.len() < 2 {
            return;
        }
        for tech in self.d.techs.keys() {
            if !self.tech_is_shared(tech) || !teams.iter().any(|&t| self.team_is_has_tech(t, Some(tech))) {
                continue;
            }
            for &t in &teams {
                if self.team_is_alive(t) && !self.team_is_has_tech(t, Some(tech)) {
                    self.team_set_has_tech_one(t, tech, true, None, false, false);
                }
            }
        }
    }
}
