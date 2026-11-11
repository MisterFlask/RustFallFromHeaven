//! Governors (this game's own rules, not FFH2's; docs/GOVERNORS.md).
//!
//! **Who runs the cities.** The government civic decides: under a civic with `direct_rule`
//! (Despotism, God King) every city of its player is ruled directly; under any other every city
//! is governed, the capital included ([`Game::player_rules_directly`]). A city gained takes its
//! owner's mode; a change of civics hands every city over, or takes every city back, at once
//! ([`Game::player_governance_sync`]). Barbarian cities are always ruled directly.
//!
//! **Roles.** A governed city has a role (`city_roles.jsonc`), which its governor runs it by: the
//! role's flavors weigh what it builds and its emphasis is the citizens'. The role is the city's
//! proposal ([`Game::city_role_proposal`]: the role its building tiers suit best, the owner's
//! empire priorities weighing more) unless the player chose another one for it, which needs a
//! suitability of `GOVERNOR_ROLE_MIN_SUITABILITY` ([`Game::player_set_city_role`]).
//!
//! **Governors** are characters: a name and at most one trait (`governor_traits.jsonc`: how it
//! judges, never a modifier of the city). A city gets one the first time it is governed; it stays
//! with the city, dormant while the city is ruled directly, and leaves the game with a city lost.
//!
//! **Production.** A governed city's production is the governor's (the controller's production
//! choice). The player places empire orders there ([`Game::city_push_empire_order`]: a unit, a
//! wonder, a palace, a project), which go behind an army's requisition orders (docs/ARMIES.md) and
//! ahead of the governor's choice, which keeps its progress and resumes after them. The governor
//! is not asked while a requisition or empire order leads the queue.

use std::collections::BTreeSet;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::structs::OrderKind;
use crate::utils::{is_national_wonder_class, is_team_wonder_class, is_world_wonder_class};

/// A governor (`Game::governors`; owner list `Player::governors`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Governor {
    pub id: GovernorId,
    pub owner: PlayerId,
    pub name: String,
    /// Its trait, if any (`GOVERNOR_TRAIT_CHANCE`%).
    pub trait_: Option<GovernorTraitType>,
    /// The turn it was given to its player.
    pub born_turn: i32,
    /// The city it governs (dormant while the city is ruled directly).
    pub city: CityId,
    /// The turn it took its city.
    pub since_turn: i32,
}

/// Something about a player's governed cities worth its attention ([`Game::player_governor_diagnoses`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovDiagnosis {
    /// No governed city has this role: one of the player's priorities, or any role once the
    /// player governs 6 cities or more.
    RoleUnfilled(CityRoleType),
    /// A city whose role the player chose suits it less than a Standard city would (`suitability`
    /// under 100).
    PoorFit { city: CityId, role: CityRoleType, suitability: i32 },
    /// A governed city whose first empire order has more than 30 turns left.
    SlowOrder { city: CityId, turns: i32 },
}

pub type GovResult = Result<(), &'static str>;

/// The number of governed cities from which every role is expected to be filled.
const ALL_ROLES_FROM_CITIES: usize = 6;
/// An empire order with more turns left than this is slow.
const SLOW_ORDER_TURNS: i32 = 30;

impl Game {
    /// Whether city `c` is governed (not directly ruled). Barbarian cities are never governed.
    pub fn city_is_governed(&self, c: CityId) -> bool {
        // This game's outposts (docs/NAVAL.md, 9.2) have no governor.
        !self.cities[c].direct_rule && !self.player_is_barbarian(self.cities[c].owner) && !self.cities[c].outpost
    }

    /// Whether player `p` rules its cities directly: one of its civics has `direct_rule`, or it is
    /// the barbarian player. Otherwise every city of it is governed.
    pub fn player_rules_directly(&self, p: PlayerId) -> bool {
        self.player_is_barbarian(p) || self.players[p].civics.iter().flatten().any(|&civic| self.d.civics[civic].direct_rule)
    }

    /// The role city `c` is governed by; `None` for a directly ruled city.
    pub fn city_role(&self, c: CityId) -> Option<CityRoleType> {
        if !self.city_is_governed(c) {
            return None;
        }
        self.cities[c].role
    }

    /// City `c`'s governor's trait (`None` without a trait, and for a directly ruled city).
    pub fn city_governor_trait(&self, c: CityId) -> Option<GovernorTraitType> {
        if !self.city_is_governed(c) {
            return None;
        }
        self.cities[c].governor.and_then(|g| self.governors.get(g)).and_then(|g| g.trait_)
    }

    /// Player `p`'s governed cities.
    pub fn player_governed_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.players[p].cities.iter().copied().filter(|&c| self.city_is_governed(c)).collect()
    }

    // ---------------------------------------------------------------------------------------
    // Roles
    // ---------------------------------------------------------------------------------------

    /// How well city `c`'s building tiers suit `role`, in percent: the mean, over the role's key
    /// building classes its civilization can build at all, of its tier's core effect for the
    /// class (`BuildingTierInfo::core_effect_percent`); 100 when it can build none of them.
    pub fn city_role_suitability(&self, c: CityId, role: CityRoleType) -> i32 {
        let d = self.d;
        let civ = self.city_civilization_type(c);
        let (mut sum, mut n) = (0, 0);
        for &class in &d.city_roles[role].key_buildings {
            if d.civ_building(civ, class).is_none() {
                continue;
            }
            sum += d.building_tiers[self.city_tier(c, class)].core_effect_percent;
            n += 1;
        }
        if n == 0 {
            100
        } else {
            sum / n
        }
    }

    /// The role city `c` would be given: the one with the highest suitability, weighed
    /// `100 + GOVERNOR_PRIORITY_WEIGHT` percent for one of its owner's priorities (ties: the
    /// first role).
    pub fn city_role_proposal(&self, c: CityId) -> CityRoleType {
        let p = self.cities[c].owner;
        let priorities = &self.players[p].governor_priorities;
        let weight = self.d.defines.governor_priority_weight;
        self.d
            .city_roles
            .keys()
            .max_by_key(|&r| {
                let bonus = if priorities.contains(&r) { weight } else { 0 };
                (self.city_role_suitability(c, r) * (100 + bonus), std::cmp::Reverse(r))
            })
            .expect("a city role")
    }

    /// Whether player `p` may choose `role` for city `c` (else why not): its governed city, with a
    /// suitability of `GOVERNOR_ROLE_MIN_SUITABILITY` for the role (the proposal is always allowed).
    pub fn player_check_city_role(&self, p: PlayerId, c: CityId, role: CityRoleType) -> GovResult {
        if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_is_governed(c) {
            return Err("Not a governed city of yours.");
        }
        if role.idx() >= self.d.city_roles.len() {
            return Err("No such role.");
        }
        if role != self.city_role_proposal(c) && self.city_role_suitability(c, role) < self.d.defines.governor_role_min_suitability {
            return Err("Role requires suitability 75%.");
        }
        Ok(())
    }

    /// Player `p` chooses `role` for city `c`: the role is set and kept (`role_overridden`) until
    /// the player chooses again; choosing the proposal lets the role follow the proposal again.
    pub fn player_set_city_role(&mut self, p: PlayerId, c: CityId, role: CityRoleType) -> GovResult {
        self.player_check_city_role(p, c, role)?;
        let proposal = self.city_role_proposal(c);
        self.cities[c].role_overridden = role != proposal;
        if self.cities[c].role != Some(role) {
            self.cities[c].role = Some(role);
            self.city_apply_role_emphasis(c);
            self.city_governor_changed(c);
        }
        let turn = self.game_turn();
        let city = self.d.text.name(&self.cities[c].name).to_string();
        let name = self.d.text.name(&self.d.city_roles[role].base.description).to_string();
        let suitability = self.city_role_suitability(c, role);
        self.log.msg_at(turn, Some(p), (self.cities[c].x, self.cities[c].y), || format!("{city}: role {name} (suitability {suitability}%)."));
        Ok(())
    }

    /// Player `p`'s empire priorities are `roles` (at most `GOVERNOR_MAX_PRIORITIES`, distinct);
    /// its governed cities whose role follows the proposal take their new proposal.
    pub fn player_set_governor_priorities(&mut self, p: PlayerId, roles: Vec<CityRoleType>) -> GovResult {
        if roles.len() > self.d.defines.governor_max_priorities.max(0) as usize {
            return Err("Too many priorities.");
        }
        if roles.iter().any(|r| r.idx() >= self.d.city_roles.len()) {
            return Err("No such role.");
        }
        if roles.iter().collect::<BTreeSet<_>>().len() != roles.len() {
            return Err("A role is listed twice.");
        }
        self.players[p].governor_priorities = roles;
        self.player_repropose_roles(p);
        Ok(())
    }

    /// Player `p`'s governed cities whose role follows the proposal take it again.
    pub fn player_repropose_roles(&mut self, p: PlayerId) {
        for c in self.players[p].cities.clone() {
            if self.city_is_governed(c) && !self.cities[c].role_overridden {
                self.city_repropose_role(c);
            }
        }
    }

    /// City `c` (governed, its role not chosen by the player) takes its proposal.
    fn city_repropose_role(&mut self, c: CityId) {
        let proposal = self.city_role_proposal(c);
        if self.cities[c].role != Some(proposal) {
            self.cities[c].role = Some(proposal);
            self.city_apply_role_emphasis(c);
            self.city_governor_changed(c);
        }
    }

    /// The citizens' emphasis of governed city `c`: its role's, and no other.
    fn city_apply_role_emphasis(&mut self, c: CityId) {
        let Some(role) = self.city_role(c) else { return };
        let wanted: Vec<EmphasizeType> = self.d.city_roles[role].emphasizes.clone();
        for e in self.d.emphasizes.keys() {
            let on = wanted.contains(&e);
            if self.cities[c].ai_emphasize[e] != on {
                self.city_ai_set_emphasize(c, e, on);
            }
        }
    }

    /// The citizens of city `c` are to be assigned again (its role changed).
    fn city_governor_changed(&mut self, c: CityId) {
        self.cities[c].ai_assign_work_dirty = true;
    }

    // ---------------------------------------------------------------------------------------
    // Diagnoses
    // ---------------------------------------------------------------------------------------

    /// What player `p`'s governed cities need its attention for: roles of its priorities no
    /// governed city has (every role with `ALL_ROLES_FROM_CITIES` governed cities or more), cities
    /// whose chosen role suits them under 100%, cities whose first empire order has more than 30
    /// turns left. Roles in their order, then cities by id.
    pub fn player_governor_diagnoses(&self, p: PlayerId) -> Vec<GovDiagnosis> {
        let d = self.d;
        let mut governed = self.player_governed_cities(p);
        governed.sort();
        let filled: BTreeSet<CityRoleType> = governed.iter().filter_map(|&c| self.city_role(c)).collect();
        let all = governed.len() >= ALL_ROLES_FROM_CITIES;
        let mut out = Vec::new();
        for r in d.city_roles.keys() {
            if !filled.contains(&r) && (all || self.players[p].governor_priorities.contains(&r)) {
                out.push(GovDiagnosis::RoleUnfilled(r));
            }
        }
        for &c in &governed {
            if !self.cities[c].role_overridden {
                continue;
            }
            if let Some(role) = self.city_role(c) {
                let suitability = self.city_role_suitability(c, role);
                if suitability < 100 {
                    out.push(GovDiagnosis::PoorFit { city: c, role, suitability });
                }
            }
        }
        for &c in &governed {
            let q = &self.cities[c].order_queue;
            if let Some(i) = q.iter().position(|o| o.empire) {
                let turns = self.city_order_turns_left(c, i);
                if turns > SLOW_ORDER_TURNS {
                    out.push(GovDiagnosis::SlowOrder { city: c, turns });
                }
            }
        }
        out
    }

    /// The turns left to finish the order at `index` of city `c`'s queue (0 for a process).
    pub fn city_order_turns_left(&self, c: CityId, index: usize) -> i32 {
        let Some(o) = self.cities[c].order_queue.get(index) else { return 0 };
        let num = index as i32;
        match o.order_type {
            OrderType::ORDER_TRAIN => self.city_production_turns_left_unit(c, UnitType::from_usize(o.data1 as usize), num),
            OrderType::ORDER_CONSTRUCT => self.city_production_turns_left_building(c, BuildingType::from_usize(o.data1 as usize), num),
            OrderType::ORDER_CREATE => self.city_production_turns_left_project(c, ProjectType::from_usize(o.data1 as usize), num),
            _ => 0,
        }
    }

    // ---------------------------------------------------------------------------------------
    // A city's arrival and departure, and the change of civics
    // ---------------------------------------------------------------------------------------

    /// City `c` is new to its owner (founded, captured, traded, flipped): it takes its owner's
    /// mode. A founded city (`assign`) is handed to its governor at once; a city that changed
    /// hands once its state is restored (`city_governance_acquired`), so that its role is
    /// proposed for the city as it is (its civilization).
    pub fn city_governance_init(&mut self, c: CityId, assign: bool) {
        let p = self.cities[c].owner;
        self.cities[c].governor = None;
        self.cities[c].role = None;
        self.cities[c].role_overridden = false;
        self.cities[c].direct_rule = self.player_rules_directly(p);
        if !self.city_is_governed(c) {
            return;
        }
        self.cities[c].production_automated = true;
        self.cities[c].recruit_default = None;
        if assign {
            self.city_become_governed(c, true);
        }
    }

    /// City `c` was just acquired (`player_acquire_city`, after its state was restored and the
    /// raze decision taken): a governed one goes to its governor. Outside its owner's turn (a
    /// culture flip, a city traded in another player's turn) a governed city chooses its
    /// production at once, so that it never reaches a turn's end without; in its owner's turn the
    /// city's turn chooses it at the turn's end, with the units that took it standing in it.
    pub fn city_governance_acquired(&mut self, c: CityId) {
        if !self.cities.contains(c) || !self.city_is_governed(c) {
            return;
        }
        self.city_become_governed(c, true);
        let p = self.cities[c].owner;
        if !self.players[p].turn_active && self.cities[c].order_queue.is_empty() {
            self.city_choose_production(c, None, None, None, false, false);
        }
    }

    /// City `c` leaves its owner (captured, traded, flipped, razed): its governor leaves the game.
    pub fn city_governance_lost(&mut self, c: CityId) {
        if let Some(g) = self.cities[c].governor.take() {
            if let Some(gov) = self.governors.get(g) {
                let owner = gov.owner;
                self.governors.remove(g);
                self.players[owner].governors.retain(|&x| x != g);
            }
        }
    }

    /// Player `p`'s cities take the mode its civics give (`player_rules_directly`): all handed
    /// over, or all taken back, at once, with one message. Called when its civics change and in
    /// its turn.
    pub fn player_governance_sync(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) {
            return;
        }
        let direct = self.player_rules_directly(p);
        let mut changed = 0;
        for c in self.players[p].cities.clone() {
            if !self.cities.contains(c) {
                continue;
            }
            let governed = self.city_is_governed(c);
            if direct && governed {
                self.city_become_direct(c);
                changed += 1;
            } else if !direct && !governed {
                self.city_become_governed(c, false);
                changed += 1;
                if self.cities.contains(c) && self.cities[c].order_queue.is_empty() && !self.cities[c].settlement && !self.cities[c].outpost {
                    self.city_choose_production(c, None, None, None, false, false);
                }
            }
        }
        if changed > 0 {
            let turn = self.game_turn();
            self.log.msg(turn, Some(p), || {
                if direct {
                    format!("Government: every city is ruled directly ({changed} taken back from governors).")
                } else {
                    format!("Government: every city is governed ({changed} handed to governors).")
                }
            });
        }
    }

    /// City `c` goes to its governor (a new one the first time; `announce`: its owner is told):
    /// production automated, its units for its own garrison, its role proposed (unless the player
    /// chose one) and its emphasis applied. Its queue keeps the requisition orders and the orders
    /// that may be empire orders (they become empire orders); the others are dropped, and the
    /// governor chooses.
    fn city_become_governed(&mut self, c: CityId, announce: bool) {
        let p = self.cities[c].owner;
        self.cities[c].direct_rule = false;
        self.cities[c].production_automated = true;
        self.cities[c].recruit_default = None;
        let new_governor = self.cities[c].governor.is_none();
        if new_governor {
            self.governor_generate(p, c);
        }
        if self.cities[c].role.is_none() || !self.cities[c].role_overridden {
            self.cities[c].role_overridden = false;
            self.cities[c].role = Some(self.city_role_proposal(c));
        }
        self.city_apply_role_emphasis(c);
        // The queue: requisition orders stay, empire orders qualify, the rest gives way.
        let mut i = 0;
        while self.cities.contains(c) && i < self.cities[c].order_queue.len() {
            let o = self.cities[c].order_queue[i].clone();
            if o.requisition {
                i += 1;
            } else if self.order_may_be_empire(o.order_type, o.data1) {
                let o = &mut self.cities[c].order_queue[i];
                o.empire = true;
                o.save = false;
                i += 1;
            } else {
                self.city_pop_order(c, i as i32, false, false);
            }
        }
        self.city_governor_changed(c);
        if new_governor && announce {
            let turn = self.game_turn();
            let city = self.d.text.name(&self.cities[c].name).to_string();
            let msg = self.governor_intro(c);
            self.log.msg_at(turn, Some(p), (self.cities[c].x, self.cities[c].y), || format!("{msg} governs {city}."));
        }
    }

    /// "Name" or "Name (Trait)" of city `c`'s governor, for messages.
    fn governor_intro(&self, c: CityId) -> String {
        let Some(gov) = self.cities[c].governor.and_then(|g| self.governors.get(g)) else { return "A governor".to_string() };
        match gov.trait_ {
            Some(t) => format!("{} ({})", gov.name, self.d.text.name(&self.d.governor_traits[t].base.description)),
            None => gov.name.clone(),
        }
    }

    /// City `c` becomes directly ruled: its governor stays, dormant; its production is the
    /// player's again (the queue kept, empire orders as ordinary orders) and its role's emphasis
    /// is dropped (the player sets the emphasis).
    fn city_become_direct(&mut self, c: CityId) {
        if let Some(role) = self.city_role(c) {
            for e in self.d.city_roles[role].emphasizes.clone() {
                if self.cities[c].ai_emphasize[e] {
                    self.city_ai_set_emphasize(c, e, false);
                }
            }
        }
        self.cities[c].direct_rule = true;
        self.cities[c].production_automated = false;
        for o in self.cities[c].order_queue.iter_mut() {
            o.empire = false;
        }
        self.city_governor_changed(c);
    }

    // ---------------------------------------------------------------------------------------
    // Empire orders
    // ---------------------------------------------------------------------------------------

    /// Whether an order may be an empire order: a unit; a world, team or national wonder or a
    /// palace; a project. Not any other building, nor a process.
    pub fn order_may_be_empire(&self, order_type: OrderType, data1: i32) -> bool {
        let d = self.d;
        match order_type {
            OrderType::ORDER_TRAIN => data1 >= 0 && (data1 as usize) < d.units.len(),
            OrderType::ORDER_CREATE => data1 >= 0 && (data1 as usize) < d.projects.len(),
            OrderType::ORDER_CONSTRUCT => {
                if data1 < 0 || data1 as usize >= d.buildings.len() {
                    return false;
                }
                let info = &d.buildings[BuildingType::from_usize(data1 as usize)];
                info.capital || info.building_class_type.is_some_and(|k| is_world_wonder_class(d, k) || is_team_wonder_class(d, k) || is_national_wonder_class(d, k))
            }
            _ => false,
        }
    }

    /// Whether player `p` may place an empire order (`order_type`, `data1`) in city `c` (else why
    /// not): its governed city, an order that may be an empire order, which the city can build.
    pub fn city_check_empire_order(&self, p: PlayerId, c: CityId, order_type: OrderType, data1: i32) -> GovResult {
        if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_is_governed(c) {
            return Err("Not a governed city of yours.");
        }
        if !self.order_may_be_empire(order_type, data1) {
            return Err("Empire orders: units, wonders, palaces and projects only.");
        }
        let can = match order_type {
            OrderType::ORDER_TRAIN => self.city_can_train(c, UnitType::from_usize(data1 as usize), false, false, false, false),
            OrderType::ORDER_CONSTRUCT => self.city_can_construct(c, BuildingType::from_usize(data1 as usize), false, false, false),
            OrderType::ORDER_CREATE => self.city_can_create(c, ProjectType::from_usize(data1 as usize), false, false),
            _ => false,
        };
        if !can {
            return Err("The city cannot build this.");
        }
        Ok(())
    }

    /// Player `p` places an empire order in governed city `c`: behind its requisition orders and
    /// earlier empire orders, ahead of the governor's choice (which keeps its progress). `data2` is
    /// the unit AI of a unit (-1: its default), `dest` where it goes (a unit only; `None`: the
    /// city's own garrison).
    pub fn city_push_empire_order(&mut self, p: PlayerId, c: CityId, order_type: OrderType, data1: i32, data2: i32, dest: Option<crate::army::RecruitDest>) -> GovResult {
        self.city_check_empire_order(p, c, order_type, data1)?;
        if let Some(dest) = dest {
            if order_type != OrderType::ORDER_TRAIN {
                return Err("Only a unit has a destination.");
            }
            if !self.recruit_dest_valid(p, dest) {
                return Err("Not a destination of yours.");
            }
        }
        let data2 = if order_type == OrderType::ORDER_TRAIN { data2 } else { -1 };
        let Some(index) = self.city_push_order_kind(c, order_type, data1, data2, false, false, false, false, OrderKind::Empire) else {
            return Err("The city cannot build this.");
        };
        if self.cities.contains(c) && dest.is_some() {
            self.cities[c].order_queue[index].dest = dest;
        }
        Ok(())
    }

    /// Whether player `p` may take the order at `index` of governed city `c`'s queue off (else why
    /// not): only its own empire orders (requisition orders: `city_check_remove_order`).
    pub fn city_check_remove_empire_order(&self, p: PlayerId, c: CityId, index: usize) -> GovResult {
        if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_is_governed(c) {
            return Err("Not a governed city of yours.");
        }
        match self.cities[c].order_queue.get(index) {
            Some(o) if o.empire => Ok(()),
            Some(o) if o.requisition => Err("A requisition order: release the city from the army's requisition."),
            Some(_) => Err("The governor's order: only empire orders can be removed."),
            None => Err("No such order."),
        }
    }

    /// Player `p` takes the empire order at `index` of governed city `c`'s queue off (what was put
    /// into it stays with the item); an emptied queue goes to the governor.
    pub fn city_remove_empire_order(&mut self, p: PlayerId, c: CityId, index: usize) -> GovResult {
        self.city_check_remove_empire_order(p, c, index)?;
        self.city_pop_order(c, index as i32, false, true);
        Ok(())
    }

    // ---------------------------------------------------------------------------------------
    // The turn
    // ---------------------------------------------------------------------------------------

    /// Player `p`'s governance in its turn processing: its cities in the mode its civics give,
    /// governed cities automated, the governors' own initiative (a human's governed cities).
    pub fn player_governance_turn(&mut self, p: PlayerId) {
        if self.player_is_barbarian(p) || !self.players[p].alive {
            return;
        }
        self.player_governance_sync(p);
        for c in self.players[p].cities.clone() {
            if self.city_is_governed(c) && !self.cities[c].production_automated {
                self.cities[c].production_automated = true;
            }
        }
        let controller = self.controller(p);
        controller.govern_cities(self, p);
    }

    // ---------------------------------------------------------------------------------------
    // Governors
    // ---------------------------------------------------------------------------------------

    /// A new governor of player `p` for city `c`: a name of its civilization's list, a trait with
    /// `GOVERNOR_TRAIT_CHANCE`% (drawn uniformly; the game's RNG).
    fn governor_generate(&mut self, p: PlayerId, c: CityId) -> GovernorId {
        let d = self.d;
        let n = d.governor_traits.len() as i32;
        let trait_ = if n > 0 && self.soren_rand_num(100, "Governor trait chance") < d.defines.governor_trait_chance {
            Some(GovernorTraitType::from_usize(self.soren_rand_num(n, "Governor trait") as usize))
        } else {
            None
        };
        let name = self.governor_new_name(p);
        let turn = self.game_turn();
        let g = self.governors.insert_with(|id| Governor { id, owner: p, name, trait_, born_turn: turn, city: c, since_turn: turn });
        self.players[p].governors.push(g);
        self.players[p].governor_count += 1;
        self.cities[c].governor = Some(g);
        g
    }

    /// A name for a new governor of `p`: the first of its civilization's list none of its governors
    /// has, else of the generic list, else "Governor N".
    fn governor_new_name(&self, p: PlayerId) -> String {
        let d = self.d;
        let civ = self.player_get_civilization_type(p);
        let taken: BTreeSet<&str> = self.players[p].governors.iter().filter_map(|&g| self.governors.get(g)).map(|g| g.name.as_str()).collect();
        let lists = [d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_some() && d.governor_names[k].civilization == civ), d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_none())];
        for list in lists.into_iter().flatten() {
            if let Some(n) = d.governor_names[list].names.iter().find(|n| !taken.contains(n.as_str())) {
                return n.clone();
            }
        }
        format!("Governor {}", self.players[p].governor_count + 1)
    }
}
