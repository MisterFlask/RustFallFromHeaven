//! The public siege counter, its effects, and the consequence of moving the investing force.
use ffh_core::{army::SupplyState, game::Game, ids::*};
use crate::{json::Json, view::{city_name, Viewer}};

pub fn city(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    if !v.city_visible(c) && g.cities[c].owner != v.h && !v.reveal { return Json::Null; }
    let turns = g.cities[c].siege_turns;
    let invested = g.city_is_invested(c);
    if turns == 0 && !invested { return Json::Null; }
    let counting = g.city_supply_cut_by_siege(c);
    let (state, label, mut text) = match g.city_siege_state(c) {
        SupplyState::Supplied if counting => ("invested", "Invested".into(), "Invested: Hunger from the city's next turn. No army supply source.".into()),
        SupplyState::Supplied => ("relieved", "Open harbour".into(), "Open harbour: investment causes no Hunger and the city remains an army supply source.".into()),
        SupplyState::Hunger { turn, of } => ("hunger", format!("Hunger {turn}/{of}"), format!("Hunger: turn {turn} of {of}. No healing of any kind.")),
        SupplyState::Starvation => ("starvation", format!("Starvation {turns}"), format!("Starvation: -{}% HP a turn (to {}%), {:+}% damage dealt. No healing of any kind.", g.d.defines.starvation_damage_percent, g.d.defines.starvation_floor_percent, g.d.defines.starvation_damage_dealt_percent)),
    };
    if turns > 0 {
        if counting { text.push_str(" No army supply source while invested."); }
        else { text.push_str(&format!(" Investment relieved: {} of 2 turns toward recovery.", g.cities[c].siege_relief_turns)); }
        text.push_str(" Two consecutive relieved turns clear the siege counter. Sentinels are exempt from Hunger and Starvation.");
    }
    let own = g.cities[c].owner == v.h || v.reveal;
    crate::jobj! {
        "state" => state, "label" => label, "text" => text,
        "turn" => turns, "hunger_turns" => g.d.defines.supply_hunger_turns,
        "relief_turns" => g.cities[c].siege_relief_turns,
        "invested" => invested, "counting" => counting,
        "besiegers" => own.then(|| g.city_investment_besiegers(c).len()),
        "required" => own.then(|| g.city_investment_required(c)),
        "garrison" => own.then(|| g.city_investment_garrison(c).len()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndSiege {
    pub city: CityId,
    pub name: String,
    pub turn: i32,
    pub starving: bool,
    pub text: String,
}
impl EndSiege {
    pub fn to_json(&self) -> Json { crate::jobj! {
        "city" => self.city, "name" => self.name.as_str(), "turn" => self.turn,
        "starving" => self.starving, "text" => self.text.as_str(),
    } }
}

pub fn ended_by_move(g: &Game, h: PlayerId, units: &[UnitId], to: PlotIdx) -> Vec<EndSiege> {
    let v = Viewer::new(g, h, false);
    g.cities.iter().filter_map(|(c, city)| {
        let affected = g.city_plot(c) == to || units.iter().any(|&u| g.unit_plot_opt(u).is_some_and(|p| {
            let (x, y) = g.map.xy(p);
            g.plot_distance(x, y, city.x, city.y) <= 1
        }));
        if !affected { return None; }
        if !v.city_visible(c) || !g.city_supply_cut_by_siege(c) || g.city_invested_after_move(c, units, to) { return None; }
        let name = city_name(g, c);
        let turn = city.siege_turns;
        let starving = g.city_siege_state(c) == SupplyState::Starvation;
        let state = if starving { "Starvation" } else if turn > 0 { "Hunger" } else { "before Hunger" };
        Some(EndSiege { city: c, name: name.clone(), turn, starving,
            text: format!("Ends the siege of {name} (turn {turn}, {state}).") })
    }).collect()
}
