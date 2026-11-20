//! Breaking away and the civil war of direct rule (docs/PERSONS.md, "The second build", rules 7
//! and 11).
//!
//! - **A House breaks away** ([`Game::house_break_away`]) when its liege refuses its ultimatum (the
//!   third build, rule 5: `ultimatums.rs`), when its liege comes under a direct-rule civic (every
//!   House at once: the civil war): first every city of the House in which the liege's army units outnumber the House's own land combat units goes
//!   back to the liege ([`Game::city_hold`]; as a trade: its buildings and garrison; `Censura
//!   returns to you: your army's 4 units outnumber its 2.`; a city contested and lost is told too:
//!   `Corona goes with House Arne: its 3 units against your army's 2.`), then the vassalage ends
//!   (the record ends), and the House's team declares war on the liege's team, [`GRUDGE`] both
//!   ways. Everyone who has met the liege is told. Houses breaking away the same turn are not
//!   merged.
//! - **Direct rule** ([`Game::player_persons_civil_war`]): a player with Houses that comes under
//!   a direct-rule civic, by any means, has every House break away at once; the counter on the
//!   map holds.

use ffh_data::*;

use super::*;

/// The attitude modifier between a civilization that broke away and its former liege, both ways
/// (FFH's scale, 4 a step; the Sundering's grudge).
pub const GRUDGE: i32 = -4;

/// Why a House breaks away (its telemetry and its message).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Why {
    /// Its ultimatum refused (the third build, rule 5).
    Ultimatum,
    /// A direct-rule civic adopted.
    CivilWar,
}

impl Game {
    /// Whether unit `u` of player `p` on a released city's plot goes with the city
    /// (`world::birth`'s release: the owner's land units that can fight, not heroes).
    pub(crate) fn unit_goes_with_city(&self, u: UnitId, p: PlayerId) -> bool {
        self.units.contains(u)
            && self.units[u].owner == p
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && self.d.units[self.units[u].unit_type].combat > 0
            && !self.unit_is_hero(u)
    }

    fn city_names(&self, cities: &[CityId]) -> String {
        and_list(&cities.iter().map(|&c| self.persons_city_name(c)).collect::<Vec<_>>())
    }

    // -----------------------------------------------------------------------------------------
    // A House breaking away
    // -----------------------------------------------------------------------------------------

    /// House `h` breaks away (`why`): its cities where its liege's army units outnumber its own go
    /// back to the liege, the vassalage ends, and it declares war on its liege. Returns its player,
    /// alive.
    pub(crate) fn house_break_away(&mut self, h: HouseId, why: Why) -> Option<PlayerId> {
        let p = self.houses[h].owner;
        let hp = self.houses[h].player;
        let label = self.house_label(h);
        let civ_name = self.player_get_civilization_short_description(p);
        let (going, held) = self.house_cities_held(h);
        let turn = self.game_turn();
        // The contests the liege lost, told before the cities change hands.
        let lost: Vec<((i32, i32), String)> = going.iter().filter_map(|&c| self.city_hold_line(c, &label).map(|l| ((self.cities[c].x, self.cities[c].y), l))).collect();
        // The counter on the map: the held cities go back to the liege, as a trade.
        for &c in &held {
            let plot = self.city_plot(c);
            let city = self.persons_city_name(c);
            let line = self.city_hold_line(c, &label).unwrap_or_else(|| format!("{city} returns to you."));
            self.persons_move_city(c, p);
            let at = self.map.xy(plot);
            self.log.msg_marked(turn, Some(p), at, LogMark::House, LogTag::None, || line);
        }
        for (at, line) in lost {
            self.log.msg_marked(turn, Some(p), at, LogMark::House, LogTag::None, || line);
        }
        // The vassalage ends: the record ends.
        let (ht, pt) = (self.players[hp].team, self.players[p].team);
        self.team_end_vassalage(ht, pt);
        if self.houses.contains(h) {
            self.house_record_end(h);
        }
        self.player_verify_alive(hp);
        let alive = self.players[hp].alive;
        if alive {
            self.players[hp].ai_attitude_extra[p] += GRUDGE;
            self.players[p].ai_attitude_extra[hp] += GRUDGE;
            self.team_declare_war(ht, pt, false, None);
        }
        let names = self.city_names(&going.iter().copied().filter(|&c| self.cities.contains(c)).collect::<Vec<_>>());
        let at = self.player_get_capital_city(hp).map(|c| (self.cities[c].x, self.cities[c].y));
        // The news names the cities a House takes after "taking".
        let text = match (why, alive) {
            (_, false) => format!("{label} ends: every city of it stands held by an army of the {civ_name}."),
            (Why::CivilWar, _) => format!("Civil war: {label} breaks away from the {civ_name}, taking {names}: war."),
            (Why::Ultimatum, _) => format!("{label} breaks away from the {civ_name}, taking {names}: war."),
        };
        let others = self.audience_met(pt);
        match at {
            Some(at) => self.log.msg_to_at(turn, others, at, LogMark::House, || text),
            None => self.log.msg_to(turn, others, || text),
        }
        let tel = match why {
            Why::CivilWar => "civil_war",
            Why::Ultimatum => "ultimatum",
        };
        self.tel_house(p, "seceded", if alive { tel } else { "held" });
        alive.then_some(hp)
    }

    // -----------------------------------------------------------------------------------------
    // The turn, the civil war
    // -----------------------------------------------------------------------------------------

    /// Player `p` adopting direct rule with Houses: every House breaks away at once
    /// (docs/PERSONS.md, rule 11). Called when its civics change into direct rule
    /// (`player_set_civics`); a vassal taken later under direct rule is a House and stays.
    pub fn player_persons_civil_war(&mut self, p: PlayerId) {
        if self.players[p].houses.is_empty() || !self.player_rules_directly(p) {
            return;
        }
        let mut houses = self.players[p].houses.clone();
        houses.sort();
        for h in houses {
            if self.houses.contains(h) {
                self.house_break_away(h, Why::CivilWar);
            }
            // (Whatever could not break away ends too.)
            if self.houses.contains(h) {
                self.house_record_end(h);
            }
        }
        self.players[p].houses.clear();
    }
}
