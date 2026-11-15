//! Investment and garrison hunger (docs/SIEGES.md; the owner's siege design).
use ffh_data::*;
use crate::{army::SupplyState, game::Game, ids::*};

impl Game {
    fn unit_investment_soldier(&self, u: UnitId) -> bool {
        self.units.contains(u) && !self.unit_is_dead(u)
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && self.unit_can_fight(u) && !self.d.units[self.units[u].unit_type].animal
    }

    /// Soldiers physically defending the city, including visiting armies of its team.
    pub fn city_investment_garrison(&self, c: CityId) -> Vec<UnitId> {
        let team = self.city_team(c);
        self.map.plots[self.city_plot(c)].units.iter().copied()
            .filter(|&u| self.unit_investment_soldier(u) && !self.unit_is_cargo(u) && self.unit_get_team(u) == team).collect()
    }

    /// Visible land soldiers at war on adjacent land. Animals and bombard-only sieges do not invest.
    pub fn city_investment_besiegers(&self, c: CityId) -> Vec<UnitId> {
        let team = self.city_team(c);
        self.city_besiegers(c).into_iter().filter(|&u| self.unit_investment_soldier(u) && !self.unit_is_cargo(u)
            && self.teams[team].at_war[self.unit_get_team(u)]
            && self.plot_is_visible(self.unit_plot(u), team)).collect()
    }

    pub fn city_investment_required(&self, c: CityId) -> usize {
        self.city_investment_garrison(c).len().div_ceil(2).max(3)
    }

    pub fn city_is_invested(&self, c: CityId) -> bool {
        self.city_investment_besiegers(c).len() >= self.city_investment_required(c)
    }

    /// An open harbour relieves investment, both for the counter and as an army supply source.
    pub fn city_supply_cut_by_siege(&self, c: CityId) -> bool {
        self.city_is_invested(c) && self.city_harbour(c) != crate::seas::Harbour::Open
    }

    pub fn city_siege_state(&self, c: CityId) -> SupplyState {
        self.supply_state(self.cities[c].siege_turns)
    }

    /// Counter on a unit's city plot. Every unit there is affected; Sentinels are not units.
    pub fn plot_siege_turns(&self, plot: PlotIdx) -> i32 {
        self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c))
            .map_or(0, |c| self.cities[c].siege_turns)
    }

    /// The same investment test with a selection hypothetically standing at `to`. Pure.
    pub fn city_invested_after_move(&self, c: CityId, units: &[UnitId], to: PlotIdx) -> bool {
        let team = self.city_team(c);
        let city_plot = self.city_plot(c);
        let (cx, cy) = self.map.xy(city_plot);
        let (tx, ty) = self.map.xy(to);
        let adjacent = self.plot_distance(cx, cy, tx, ty) == 1 && !self.plot_is_water(to);
        let mut besiegers = self.city_investment_besiegers(c).into_iter().filter(|u| !units.contains(u)).count();
        let mut defenders = self.city_investment_garrison(c).into_iter().filter(|u| !units.contains(u)).count();
        for &u in units {
            if !self.unit_investment_soldier(u) { continue; }
            let ut = self.unit_get_team(u);
            if to == city_plot && ut == team { defenders += 1; }
            if adjacent && self.teams[team].at_war[ut] && !self.unit_is_invisible_in_plot(u, team, Some(to), false) {
                // The city sees its adjacent plots; a moving unit brings no hidden information.
                besiegers += 1;
            }
        }
        besiegers >= defenders.div_ceil(2).max(3)
    }

    /// The owner's turn starts: count investment, pause once when relieved, clear on the second.
    pub fn player_update_city_sieges(&mut self, p: PlayerId) {
        for c in self.players[p].cities.clone() {
            let before = self.cities[c].siege_turns;
            if self.city_supply_cut_by_siege(c) {
                self.cities[c].siege_turns = before.saturating_add(1);
                self.cities[c].siege_relief_turns = 0;
            } else if before > 0 {
                self.cities[c].siege_relief_turns += 1;
                if self.cities[c].siege_relief_turns >= 2 {
                    self.cities[c].siege_turns = 0;
                    self.cities[c].siege_relief_turns = 0;
                }
            } else {
                self.cities[c].siege_relief_turns = 0;
            }
            let after = self.cities[c].siege_turns;
            let hunger = self.d.defines.supply_hunger_turns;
            let text = if before == 0 && after > 0 {
                Some(format!("Hunger: turn 1 of {hunger}. No healing."))
            } else if after == hunger + 1 && after > before {
                Some(format!("Starvation: -{}% HP a turn (to {}%), {:+}% damage dealt.", self.d.defines.starvation_damage_percent, self.d.defines.starvation_floor_percent, self.d.defines.starvation_damage_dealt_percent))
            } else if before > 0 && after == 0 {
                Some("Siege hunger cleared after two relieved turns.".into())
            } else { None };
            if let Some(text) = text {
                let name = self.d.text.name(&self.cities[c].name).to_owned();
                let at = self.map.xy(self.city_plot(c));
                self.log.msg_at(self.game_turn(), Some(p), at, || format!("{name}: {text}"));
            }
        }
    }
}
