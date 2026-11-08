//! The economy and content batch of cards (docs/CARDS.md, section 5): the rules their data keys
//! name, and the callbacks of their spells. Each rule is called from where the rule it changes
//! lives, with a `// Card:` comment there.
//!
//! - Buildings that train units (`BuildingInfo::train_units`: the Beast Market, the Elemental
//!   Foundry): [`Game::city_train_unit_entry`], [`Game::player_unit_base_cost`].
//! - Civic unit costs (Magocracy) and no war weariness (Stoicism):
//!   [`Game::player_civic_unit_cost_modifier`], [`Game::player_no_war_weariness`].
//! - Golems of the Earthmother: [`Game::player_earthmother_golem`].
//! - Maponos's Cradle (`free_unit_turns`): [`Game::city_do_free_units`].
//! - Muster (`ProcessInfo::production_to_experience`): [`Game::city_do_muster`].
//! - The Spire of the Archmagi (`free_promotion_choices`): [`Game::city_free_promotion_choice`].
//! - Agares's Well (`free_bonus_choices`): [`Game::city_free_bonus_choice`].
//! - The Trading Post (`ImprovementInfo::claim_range`): [`Game::plot_claim_owner`],
//!   [`Game::plot_claim`].
//! - The Font of Rebirth (`rebirth_range`): [`Game::unit_rebirth_font`].
//! - Mana Grafting (`SPELL_MANA_GRAFT`) and the Paymaster's Buy Out (`SPELL_BUY_OUT`): the
//!   callbacks below.

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use crate::utils::is_world_unit_class;
use ffh_data::*;

/// Maponos's Cradle: a free unit that founds cities is made only while a city site within this
/// many plots of the city is free (docs/DECKS.md, "Maponos's Cradle").
pub const FREE_UNIT_SITE_RANGE: i32 = 10;

/// The Font of Rebirth: the share of its HP, in percent, a unit it saves keeps.
pub const REBIRTH_HP_PERCENT: i32 = 10;

/// The Paymaster: a unit bought out costs this many times its production cost in gold.
pub const BUY_OUT_COST_TIMES: i32 = 3;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellManaGraft" => spell_mana_graft,
        "spellBuyOut" => spell_buy_out,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqManaGraft" => req_mana_graft,
        "reqBuyOut" => req_buy_out,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsManaGraft" => targets_mana_graft,
        "targetsBuyOut" => targets_buy_out,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Mana Grafting.

/// The mana node a build makes for each typed mana resource: an improvement some build makes whose
/// `bonus_convert` is that resource (`BUILD_MANA_FIRE` makes `IMPROVEMENT_MANA_FIRE`, which makes
/// its raw mana `BONUS_MANA_FIRE`). In build order.
pub fn mana_nodes(d: &GameData) -> Vec<(BonusType, ImprovementType)> {
    let mut out: Vec<(BonusType, ImprovementType)> = Vec::new();
    for b in d.builds.iter() {
        let Some(imp) = b.improvement else { continue };
        let Some(bonus) = d.improvements[imp].bonus_convert else { continue };
        let mana = d.bonuses[bonus].bonus_class_type.map_or(-1, |k| k.i()) == d.defines.bonusclass_mana;
        if mana && !out.iter().any(|&(x, _)| x == bonus) {
            out.push((bonus, imp));
        }
    }
    out
}

impl Game {
    /// Mana Grafting: the mana types of the nodes `p` has in its territory (any improvement that
    /// makes a typed mana resource of the nodes: built nodes and the unique features'), in the
    /// order of [`mana_nodes`].
    pub fn player_node_types(&self, p: PlayerId) -> Vec<BonusType> {
        let d = self.d;
        let nodes = mana_nodes(d);
        let counts = &self.players[p].improvement_count;
        let mut out: Vec<BonusType> = Vec::new();
        for imp in d.improvements.keys() {
            let Some(bonus) = d.improvements[imp].bonus_convert else { continue };
            if counts[imp] > 0 && nodes.iter().any(|&(b, _)| b == bonus) && !out.contains(&bonus) {
                out.push(bonus);
            }
        }
        out.sort_by_key(|&b| nodes.iter().position(|&(x, _)| x == b));
        out
    }

    /// Mana Grafting: the types plot `plot`'s node can be grafted into for player `p`: none unless
    /// it holds a built node of `p`'s that was never grafted; then every other type of `p`'s nodes.
    pub fn plot_graft_targets(&self, plot: PlotIdx, p: PlayerId) -> Vec<BonusType> {
        let d = self.d;
        let pl = &self.map.plots[plot];
        if pl.grafted || pl.owner != Some(p) {
            return Vec::new();
        }
        let Some(imp) = pl.improvement_type else { return Vec::new() };
        let nodes = mana_nodes(d);
        let Some(&(current, _)) = nodes.iter().find(|&&(_, i)| i == imp) else { return Vec::new() };
        if pl.bonus_type != Some(current) {
            return Vec::new();
        }
        self.player_node_types(p).into_iter().filter(|&b| b != current).collect()
    }

    /// Mana Grafting: plot `plot`'s node becomes a node of `bonus` (its improvement and resource),
    /// for good.
    pub fn plot_graft(&mut self, plot: PlotIdx, bonus: BonusType) {
        let Some(&(_, imp)) = mana_nodes(self.d).iter().find(|&&(b, _)| b == bonus) else { return };
        self.plot_set_improvement_type(plot, Some(imp));
        // (The node's `bonus_convert` made the resource `bonus`.)
        self.map.plots[plot].grafted = true;
    }
}

/// `reqManaGraft(caster)`: the caster stands on a node of its owner's that can be grafted.
fn req_mana_graft(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    !g.plot_graft_targets(g.unit_plot(caster), g.units[caster].owner).is_empty()
}

/// `targetsManaGraft(caster)`: one choice per mana type the node can become (the resource's id).
fn targets_mana_graft(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    g.plot_graft_targets(g.unit_plot(caster), g.units[caster].owner).into_iter().map(|b| SpellTarget::Choice(b.i())).collect()
}

/// `spellManaGraft(caster)`: the node becomes the chosen type.
fn spell_mana_graft(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Choice(i)) = g.spell_target else { return };
    let plot = g.unit_plot(caster);
    let Some(&bonus) = g.plot_graft_targets(plot, g.units[caster].owner).iter().find(|b| b.i() == i) else { return };
    g.plot_graft(plot, bonus);
}

// ---------------------------------------------------------------------------------------------
// The Paymaster's Buy Out.

impl Game {
    /// The Paymaster: the gold unit `t` costs to buy out ([`BUY_OUT_COST_TIMES`] times its
    /// production cost at the game's speed); `None` for a unit that cannot be bought (a hero, a
    /// world unit, a unit without a production cost).
    pub fn unit_buy_out_price(&self, t: UnitId) -> Option<i32> {
        let d = self.d;
        let unit_type = self.units[t].unit_type;
        let info = &d.units[unit_type];
        if self.unit_is_hero(t) || info.unit_class_type.is_some_and(|k| is_world_unit_class(d, k)) {
            return None;
        }
        let cost = if info.production_cost > 0 { info.production_cost } else { d.train_unit_buildings.iter().find_map(|&b| d.train_unit_entry(b, unit_type)).map_or(0, |e| e.cost) };
        if cost <= 0 {
            return None;
        }
        let cost = cost * d.game_speeds[self.game_speed_type()].train_percent / 100;
        Some(BUY_OUT_COST_TIMES * cost.max(1))
    }

    /// The Paymaster: the units `caster` can buy out now: land units of civilizations its team is
    /// at war with, next to it, that its team sees, at a price its owner can pay.
    pub fn unit_buy_out_targets(&self, caster: UnitId) -> Vec<UnitId> {
        let owner = self.units[caster].owner;
        let team = self.players[owner].team;
        let gold = self.players[owner].gold;
        let (x, y) = (self.units[caster].x, self.units[caster].y);
        let mut out = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(q) = self.plot_xy(x, y, dx, dy) else { continue };
                for &t in &self.map.plots[q].units {
                    let tt = self.unit_get_team(t);
                    if !self.teams[team].at_war[tt] || self.unit_is_dead(t) || self.units[t].death_delay {
                        continue;
                    }
                    if self.unit_get_domain_type(t) != DomainType::DOMAIN_LAND || self.unit_is_invisible(t, team, false) {
                        continue;
                    }
                    if self.unit_buy_out_price(t).is_some_and(|price| price <= gold) {
                        out.push(t);
                    }
                }
            }
        }
        out
    }

    /// The Paymaster: `caster`'s owner pays for unit `t`, which joins it on `caster`'s plot (its
    /// promotions and experience kept), with no moves left this turn.
    pub fn unit_buy_out(&mut self, caster: UnitId, t: UnitId) {
        let Some(price) = self.unit_buy_out_price(t) else { return };
        let owner = self.units[caster].owner;
        if self.players[owner].gold < price {
            return;
        }
        self.players[owner].gold -= price;
        let (x, y) = (self.units[caster].x, self.units[caster].y);
        let unit_type = self.units[t].unit_type;
        let nu = self.player_init_unit(owner, unit_type, x, y, None, Some(DirectionType::DIRECTION_SOUTH));
        self.unit_convert(nu, t);
        if self.units.contains(nu) {
            self.unit_finish_moves(nu);
        }
        let turn = self.game_turn();
        let name = self.unit_get_name(nu);
        self.log.msg(turn, Some(owner), || format!("{name} has been bought for {price} gold"));
    }
}

/// `reqBuyOut(caster)`: an enemy unit next to the Paymaster can be bought.
fn req_buy_out(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    !g.unit_buy_out_targets(caster).is_empty()
}

/// `targetsBuyOut(caster)`: the units it can buy out.
fn targets_buy_out(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    g.unit_buy_out_targets(caster).into_iter().map(SpellTarget::Unit).collect()
}

/// `spellBuyOut(caster)`: the chosen unit joins the caster's owner.
fn spell_buy_out(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Unit(t)) = g.spell_target else { return };
    if g.units.contains(t) && g.unit_buy_out_targets(caster).contains(&t) {
        g.unit_buy_out(caster, t);
    }
}

// ---------------------------------------------------------------------------------------------
// Unit costs and limits.

impl Game {
    /// Card: the Beast Market, the Elemental Foundry (`train_units`): the entry of a building of
    /// city `c` that lets it train `unit`.
    pub fn city_train_unit_entry(&self, c: CityId, unit: UnitType) -> Option<TrainUnit> {
        let d = self.d;
        d.train_unit_buildings.iter().filter(|&&b| self.city_num_active_building(c, b) > 0).find_map(|&b| d.train_unit_entry(b, unit).copied())
    }

    /// The production cost of `unit` for `p` before the game's modifiers: the unit's own cost;
    /// for a unit only a building trains (`production_cost` -1), that building's price
    /// (`train_units`); the Mithril Golem at half for Golems of the Earthmother.
    pub fn player_unit_base_cost(&self, p: PlayerId, unit: UnitType) -> i32 {
        let d = self.d;
        let info = &d.units[unit];
        if info.production_cost == -1 {
            // Card: the Beast Market, the Elemental Foundry (docs/CARDS.md).
            if let Some(e) = d.train_unit_buildings.iter().find_map(|&b| d.train_unit_entry(b, unit)) {
                return e.cost;
            }
            return -1;
        }
        // Card: Golems of the Earthmother (docs/CARDS.md).
        if self.player_earthmother_golem(p, unit) {
            return info.production_cost / 2;
        }
        info.production_cost
    }

    /// Card: the Elemental Foundry (`train_units`, `national`): `p` has as many units of
    /// `unit`'s class as the building's limit for it, so the next costs
    /// `NATIONAL_UNIT_OVER_LIMIT_COST_PERCENT`.
    pub fn player_is_over_train_unit_limit(&self, p: PlayerId, unit: UnitType) -> bool {
        let d = self.d;
        let Some(class) = d.units[unit].unit_class_type else { return false };
        if d.units[unit].production_cost != -1 {
            return false;
        }
        let national = d.train_unit_buildings.iter().find_map(|&b| d.train_unit_entry(b, unit)).map_or(0, |e| e.national);
        national > 0 && self.players[p].unit_class_count[class] >= national
    }

    /// Card: Magocracy (`unit_cost_modifier`, `unit_combat_cost_modifiers`): the percent `p`'s
    /// civics change the production cost of `unit` by.
    pub fn player_civic_unit_cost_modifier(&self, p: PlayerId, unit: UnitType) -> i32 {
        let d = self.d;
        let combat = d.units[unit].unit_combat_type;
        self.players[p].civics.iter().flatten().map(|&c| d.civics[c].unit_cost_modifier_for(combat)).sum()
    }

    /// Card: Stoicism (`no_war_weariness`): one of `p`'s civics ends the unhappiness of war
    /// weariness.
    pub fn player_no_war_weariness(&self, p: PlayerId) -> bool {
        self.players[p].civics.iter().flatten().any(|&c| self.d.civics[c].no_war_weariness)
    }

    /// Card: Golems of the Earthmother: `p`'s team holds the card and `unit` is the Mithril Golem
    /// (no Armageddon counter needed; half the cost).
    pub fn player_earthmother_golem(&self, p: PlayerId, unit: UnitType) -> bool {
        let d = self.d;
        // (Only units with a counter requirement are looked up by name.)
        if d.units[unit].prereq_global_counter <= 0 {
            return false;
        }
        let (Some(card), Some(golem)) = (d.ty::<TechType>("TECH_CARD_GOLEMS_OF_THE_EARTHMOTHER"), d.ty::<UnitType>("UNIT_MITHRIL_GOLEM")) else { return false };
        unit == golem && self.team_is_has_tech(self.players[p].team, Some(card))
    }
}

// ---------------------------------------------------------------------------------------------
// City rules: Maponos's Cradle, Muster, the Spire of the Archmagi, Agares's Well.

impl Game {
    /// Card: Maponos's Cradle (`free_unit_turns`, `free_unit_classes`): every `free_unit_turns`
    /// turns after the building was built, city `c` makes a free unit of the first of its classes
    /// it may (a unit that founds cities only while a site within [`FREE_UNIT_SITE_RANGE`] plots is
    /// free). The city loses no population.
    pub fn city_do_free_units(&mut self, c: CityId) {
        let d = self.d;
        for b in d.buildings.keys() {
            let info = &d.buildings[b];
            if info.free_unit_turns <= 0 || self.city_num_active_building(c, b) == 0 {
                continue;
            }
            let built = self.cities[c].building_original_time[b];
            if built == crate::defs::MIN_INT {
                continue;
            }
            let age = self.game_turn() - built;
            if age <= 0 || age % info.free_unit_turns != 0 {
                continue;
            }
            let owner = self.cities[c].owner;
            let civ = self.city_civilization_type(c);
            let unit = info.free_unit_classes.iter().filter_map(|&k| d.civ_unit(civ, k)).find(|&u| !d.units[u].found || self.city_free_site_near(c, FREE_UNIT_SITE_RANGE));
            if let Some(unit) = unit {
                let (x, y) = (self.cities[c].x, self.cities[c].y);
                let nu = self.player_init_unit(owner, unit, x, y, None, Some(DirectionType::DIRECTION_SOUTH));
                let turn = self.game_turn();
                let name = self.unit_get_name(nu);
                let city = self.cities[c].name.clone();
                self.log.msg(turn, Some(owner), || format!("{} has made a free {name}", d.text.name(&city)));
            }
        }
    }

    /// A plot within `range` of city `c`, on its landmass, where its owner could found a city.
    pub fn city_free_site_near(&self, c: CityId, range: i32) -> bool {
        let owner = self.cities[c].owner;
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        let area = self.map.plots[self.city_plot(c)].area;
        for dx in -range..=range {
            for dy in -range..=range {
                if let Some(q) = self.plot_xy(x, y, dx, dy) {
                    let plot = &self.map.plots[q];
                    if plot.area != area || plot.found_disabled {
                        continue;
                    }
                    let (qx, qy) = self.map.xy(q);
                    if self.player_can_found(owner, qx, qy, false) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Card: Muster (`ProcessInfo::production_to_experience`): city `c`'s production while it
    /// maintains the process: each `production_to_experience` hammers give one experience to the
    /// owner's unit in the city with the least (ties: the first in the plot's list). With no
    /// unit to drill, the hammers are lost.
    pub fn city_do_muster(&mut self, c: CityId) {
        let Some(process) = self.city_production_process(c) else { return };
        let per = self.d.processes[process].production_to_experience;
        if per <= 0 || self.city_is_disorder(c) {
            return;
        }
        let hammers = self.city_yield_rate(c, YieldType::YIELD_PRODUCTION).max(0);
        self.cities[c].muster_production += hammers;
        let owner = self.cities[c].owner;
        let plot = self.city_plot(c);
        while self.cities[c].muster_production >= per {
            let units: Vec<UnitId> = self.map.plots[plot].units.iter().copied().filter(|&u| self.units[u].owner == owner && !self.unit_is_dead(u) && self.unit_can_acquire_promotion_any(u)).collect();
            let Some(&u) = units.iter().min_by_key(|&&u| self.units[u].experience) else {
                self.cities[c].muster_production = 0;
                return;
            };
            self.cities[c].muster_production -= per;
            self.unit_change_experience(u, 1, -1, false, false, false);
            self.unit_test_promotion_ready(u);
        }
    }

    /// Card: the Spire of the Archmagi (`free_promotion_choices`): unit `u`, just trained in city
    /// `c`, takes one promotion of each such building of the city's that it can acquire, its
    /// owner's choice ([`crate::controller::Controller::choose_free_promotion`]).
    pub fn city_free_promotion_choice(&mut self, c: CityId, u: UnitId) {
        let d = self.d;
        for b in d.buildings.keys() {
            if d.buildings[b].free_promotion_choices.is_empty() || self.city_num_active_building(c, b) == 0 || !self.units.contains(u) {
                continue;
            }
            let options: Vec<PromotionType> = d.buildings[b].free_promotion_choices.iter().copied().filter(|&pr| self.unit_can_acquire_promotion(u, pr)).collect();
            let Some(&first) = options.first() else { continue };
            let owner = self.units[u].owner;
            let pick = self.controller(owner).choose_free_promotion(self, owner, u, &options).filter(|pr| options.contains(pr)).unwrap_or(first);
            self.unit_set_has_promotion(u, pick, true);
        }
    }

    /// Card: Agares's Well (`free_bonus_choices`): the resource building `b` gives city `c`. When
    /// none was chosen yet and `ask` is set (the building is being added), its owner chooses
    /// ([`crate::controller::Controller::choose_free_bonus`]) and the choice is kept
    /// (`City::free_bonus_choices`).
    pub fn city_free_bonus_choice(&mut self, c: CityId, b: BuildingType, ask: bool) -> Option<BonusType> {
        let options = &self.d.buildings[b].free_bonus_choices;
        if let Some(&(_, bonus)) = self.cities[c].free_bonus_choices.iter().find(|&&(x, _)| x == b) {
            return Some(bonus);
        }
        let &first = options.first()?;
        if !ask {
            return None;
        }
        let owner = self.cities[c].owner;
        let pick = self.controller(owner).choose_free_bonus(self, owner, c, b, options).filter(|x| options.contains(x)).unwrap_or(first);
        self.cities[c].free_bonus_choices.push((b, pick));
        Some(pick)
    }
}

// ---------------------------------------------------------------------------------------------
// The Trading Post.

impl Game {
    /// Card: the Trading Post (`claim_range`): the player whose claim covers land plot `p`: the
    /// owner of the nearest claiming improvement within its range (ties: the lowest plot index),
    /// if that owner is alive. A claiming improvement's own plot keeps its owner.
    pub fn plot_claim_owner(&self, p: PlotIdx) -> Option<PlayerId> {
        let d = self.d;
        let claims = &d.claim_improvements;
        if claims.is_empty() || self.plot_is_water(p) {
            return None;
        }
        // Cheap test first (this runs for every plot whose culture changes): the plot's area has
        // a claiming improvement.
        let area = self.map.plots[p].area?;
        if !claims.iter().any(|&i| self.map.areas[area].num_improvements[i] > 0) {
            return None;
        }
        let range = claims.iter().map(|&i| d.improvements[i].claim_range).max().unwrap_or(0);
        let (x, y) = self.map.xy(p);
        let mut best: Option<(i32, PlotIdx, PlayerId)> = None;
        for dx in -range..=range {
            for dy in -range..=range {
                let Some(q) = self.plot_xy(x, y, dx, dy) else { continue };
                let Some(imp) = self.map.plots[q].improvement_type else { continue };
                let dist = dx.abs().max(dy.abs());
                if d.improvements[imp].claim_range < dist {
                    continue;
                }
                let Some(owner) = self.map.plots[q].owner.filter(|&o| self.players[o].alive) else { continue };
                if best.map_or(true, |(bd, bq, _)| (dist, q) < (bd, bq)) {
                    best = Some((dist, q, owner));
                }
            }
        }
        best.map(|(_, _, o)| o)
    }

    /// Card: the Trading Post: player `owner` claims the plots around claiming improvement plot
    /// `p` (just built by its unit): `p` becomes its, then the plots in range follow the claim.
    pub fn plot_claim(&mut self, p: PlotIdx, owner: PlayerId) {
        let Some(imp) = self.map.plots[p].improvement_type else { return };
        if self.d.improvements[imp].claim_range <= 0 {
            return;
        }
        self.plot_set_owner(p, Some(owner), true, true);
        self.plot_update_claims_around(p, imp);
    }

    /// Card: the Trading Post: the plots within the range of claiming improvement `imp` at `p`
    /// (built or removed there) take their owner again.
    pub fn plot_update_claims_around(&mut self, p: PlotIdx, imp: ImprovementType) {
        let range = self.d.improvements[imp].claim_range;
        let (x, y) = self.map.xy(p);
        for dx in -range..=range {
            for dy in -range..=range {
                if let Some(q) = self.plot_xy(x, y, dx, dy) {
                    self.plot_update_culture(q, true, true);
                }
            }
        }
    }

    /// Card: the Trading Post (`claim_range`, `min_city_distance`): may improvement `imp` be built
    /// on plot `p`? A claim only on unowned land; `min_city_distance` plots or more from every
    /// city.
    pub fn plot_card_improvement_site_ok(&self, p: PlotIdx, imp: ImprovementType) -> bool {
        let info = &self.d.improvements[imp];
        if info.claim_range > 0 && self.map.plots[p].owner.is_some() {
            return false;
        }
        let r = info.min_city_distance - 1;
        if r >= 0 {
            let (x, y) = self.map.xy(p);
            for dx in -r..=r {
                for dy in -r..=r {
                    if self.plot_xy(x, y, dx, dy).is_some_and(|q| self.map.plots[q].plot_city.is_some()) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------------------------
// The Font of Rebirth.

impl Game {
    /// Card: the Font of Rebirth (`rebirth_range`, `rebirth_promotion`): the city of unit `u`'s
    /// owner whose Font saves `u` from dying in a battle on plot `battle` (within the Font's range,
    /// the nearest; ties: the lowest city id), and the promotion that marks it; `None` for a unit
    /// a Font already saved, an immortal one, or none in range.
    pub fn unit_rebirth_font(&self, u: UnitId, battle: PlotIdx) -> Option<(CityId, PromotionType)> {
        let d = self.d;
        if self.unit_is_immortal(u) || self.unit_get_domain_type(u) != DomainType::DOMAIN_LAND {
            return None;
        }
        let owner = self.units[u].owner;
        let (bx, by) = self.map.xy(battle);
        let mut best: Option<(i32, CityId, PromotionType)> = None;
        for &c in &self.players[owner].cities {
            for &b in &d.rebirth_buildings {
                let info = &d.buildings[b];
                let Some(mark) = info.rebirth_promotion else { continue };
                if self.units[u].has_promotion[mark] || self.city_num_active_building(c, b) == 0 {
                    continue;
                }
                let dist = self.step_distance(bx, by, self.cities[c].x, self.cities[c].y);
                if dist <= info.rebirth_range && best.map_or(true, |(bd, bc, _)| (dist, c) < (bd, bc)) {
                    best = Some((dist, c, mark));
                }
            }
        }
        best.map(|(_, c, mark)| (c, mark))
    }

    /// Card: the Font of Rebirth: unit `u`, saved from its death by the Font of city `c`, is
    /// carried there and marked (its HP were set by the battle).
    pub fn unit_carry_to_font(&mut self, u: UnitId, c: CityId, mark: PromotionType) {
        let (x, y) = (self.cities[c].x, self.cities[c].y);
        if self.unit_plot(u) != self.city_plot(c) {
            self.unit_join_group(u, None, true);
            self.unit_set_xy(u, x, y, false);
        }
        if self.units.contains(u) {
            self.unit_set_has_promotion(u, mark, true);
            let owner = self.units[u].owner;
            let turn = self.game_turn();
            let name = self.unit_get_name(u);
            let city = self.cities[c].name.clone();
            let d = self.d;
            self.log.msg(turn, Some(owner), || format!("{name} has been carried to the Font of Rebirth in {}", d.text.name(&city)));
        }
    }
}

/// Card: the Font of Rebirth: the victims of battle `r` on plot `to` (fighters `units`, the
/// Sentinels after them) that a Font saves: (fighter index, the Font's city, its mark), in kill
/// order.
pub(crate) fn font_saves(g: &Game, units: &[UnitId], to: PlotIdx, r: &crate::engagement::engage::Fought) -> Vec<(usize, CityId, PromotionType)> {
    if g.d.rebirth_buildings.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for k in &r.kills {
        let Some(&u) = units.get(k.victim) else { continue }; // a Sentinel
        if !g.units.contains(u) || out.iter().any(|&(i, _, _)| i == k.victim) {
            continue;
        }
        if let Some((c, mark)) = g.unit_rebirth_font(u, to) {
            out.push((k.victim, c, mark));
        }
    }
    out
}

/// Card: the Font of Rebirth: battle `r` with the `saved` victims standing at
/// [`REBIRTH_HP_PERCENT`] of their HP, their deaths taken back.
pub(crate) fn spare_saved(r: &crate::engagement::engage::Fought, saved: &[(usize, CityId, PromotionType)]) -> crate::engagement::engage::Fought {
    let mut f = r.clone();
    for &(i, _, _) in saved {
        f.fate[i] = crate::engagement::report::LogFate::Standing;
        f.hp[i] = (f.max_hp[i] * REBIRTH_HP_PERCENT / 100).max(1);
        f.kills.retain(|k| k.victim != i);
    }
    f
}
