//! The callbacks of the batch "battle more" (docs/CARDS.md) and the rules the Gilded Legion adds to
//! a city's training:
//!
//! - **The Gilded Legion** (`legion_cost_percent`): once built, its city chooses one civilization
//!   it has met (`SPELL_GILDED_LEGION`, cast by a unit in the city at a player of that
//!   civilization; `City::legion_civ`). The city then trains that civilization's unit for each
//!   class (not a world unit) as that civilization's city would, with the unit's own prerequisites,
//!   at the building's percent of its cost ([`Game::city_legion_unit_civ`], read by
//!   `city_can_train` and `city_production_needed_unit`).
//! - **The Cairn Golem** (`SPELL_RAISE_THE_CAIRN`): on a hill of its owner or nobody's, the golem
//!   becomes a Cairn (the spell's `create_improvement_type`; the Cairn's battle keys are in
//!   `engagement/more_cards.rs`).

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use crate::utils::is_world_unit_class;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellGildedLegion" => spell_gilded_legion,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqGildedLegion" => req_gilded_legion,
        "reqRaiseCairn" => req_raise_cairn,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsGildedLegion" => targets_gilded_legion,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelGildedLegion" => label_gilded_legion,
        _ => return None,
    };
    Some(f)
}

impl Game {
    /// The percent of their cost at which city `c` trains a chosen civilization's units
    /// (`legion_cost_percent`: the most of its buildings'; 0 without one).
    pub fn city_legion_cost_percent(&self, c: CityId) -> i32 {
        let d = self.d;
        d.buildings.keys().filter(|&b| d.buildings[b].legion_cost_percent > 0 && self.city_num_building(c, b) > 0).map(|b| d.buildings[b].legion_cost_percent).max().unwrap_or(0)
    }

    /// The civilization whose units city `c` trains with its Gilded Legion: chosen and the
    /// building there.
    pub fn city_legion_civ(&self, c: CityId) -> Option<CivilizationType> {
        self.cities[c].legion_civ.filter(|_| self.city_legion_cost_percent(c) > 0)
    }

    /// The civilization through which city `c` would train `unit` (module comment): its legion's,
    /// when `unit` is that civilization's unit for its class, not a world unit, and not already the
    /// city's or its owner's unit for the class.
    pub fn city_legion_unit_civ(&self, c: CityId, unit: UnitType) -> Option<CivilizationType> {
        let civ = self.city_legion_civ(c)?;
        let d = self.d;
        let Some(class) = d.units[unit].unit_class_type else { return None };
        if is_world_unit_class(d, class) || d.civ_unit(civ, class) != Some(unit) {
            return None;
        }
        let own = d.civ_unit(self.city_civilization_type(c), class) == Some(unit) || self.player_get_civilization_type(self.cities[c].owner).and_then(|o| d.civ_unit(o, class)) == Some(unit);
        (!own).then_some(civ)
    }

    /// The city of unit `u`'s owner it stands in whose Gilded Legion has not chosen yet.
    pub fn unit_legion_city_to_choose(&self, u: UnitId) -> Option<CityId> {
        let plot = self.unit_plot_opt(u)?;
        let c = self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c))?;
        (self.cities[c].owner == self.units[u].owner && self.cities[c].legion_civ.is_none() && self.city_legion_cost_percent(c) > 0).then_some(c)
    }

    /// The players whose civilization a Gilded Legion of `p`'s may choose: alive, met by `p`'s
    /// team, not barbarian, of a civilization other than `p`'s; in player order.
    pub fn legion_civ_choices(&self, p: PlayerId) -> Vec<PlayerId> {
        let team = self.players[p].team;
        let own = self.player_get_civilization_type(p);
        self.player_ids()
            .filter(|&o| o != p && self.players[o].alive && !self.player_is_barbarian(o) && self.teams[team].has_met[self.players[o].team])
            .filter(|&o| self.player_get_civilization_type(o).is_some() && self.player_get_civilization_type(o) != own)
            .collect()
    }
}

/// `reqGildedLegion(caster)`: the caster stands in a city of its owner whose Gilded Legion has not
/// chosen, and a civilization is there to choose.
fn req_gilded_legion(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.unit_legion_city_to_choose(caster).is_some() && !g.legion_civ_choices(g.units[caster].owner).is_empty()
}

/// `targetsGildedLegion(caster)`: the players of the civilizations it may choose.
fn targets_gilded_legion(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    if g.unit_legion_city_to_choose(caster).is_none() {
        return Vec::new();
    }
    g.legion_civ_choices(g.units[caster].owner).into_iter().map(SpellTarget::Player).collect()
}

/// `labelGildedLegion(caster, target)`: the target player's civilization.
fn label_gilded_legion(g: &Game, _caster: UnitId, target: SpellTarget, _call: &PyCall) -> String {
    let SpellTarget::Player(o) = target else { return String::new() };
    g.player_get_civilization_type(o).map(|c| g.d.text.name(&g.d.civilizations[c].base.description).to_string()).unwrap_or_default()
}

/// `spellGildedLegion(caster)`: the city chooses the target player's civilization.
fn spell_gilded_legion(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Player(o)) = g.spell_target else { return };
    let Some(c) = g.unit_legion_city_to_choose(caster) else { return };
    if !g.legion_civ_choices(g.units[caster].owner).contains(&o) {
        return;
    }
    let civ = g.player_get_civilization_type(o);
    g.cities[c].legion_civ = civ;
    let turn = g.game_turn();
    let owner = g.units[caster].owner;
    if let Some(civ) = civ {
        let name = g.d.text.name(&g.d.civilizations[civ].base.description).to_string();
        let city = g.cities[c].name.clone();
        g.log.msg(turn, Some(owner), || format!("{city} trains the units of the {name}"));
    }
}

/// `reqRaiseCairn(caster)`: a plot of its owner or of nobody.
fn req_raise_cairn(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    let plot = g.unit_plot(caster);
    g.map.plots[plot].owner.map_or(true, |o| o == g.units[caster].owner)
}
