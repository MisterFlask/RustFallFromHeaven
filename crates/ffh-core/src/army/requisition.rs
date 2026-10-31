//! Requisition (this game's own rules, `GAMEOPTION_ARMIES`; docs/ARMIES.md, "Recruitment"): an
//! army takes soldiers from the cities its owner picks for it, interrupting their production,
//! while it is short of its target size.
//!
//! - **The cities.** `Army::requisition`, in the order the player added them: any of its cities,
//!   directly ruled or governed, the capital included ([`Game::army_add_requisition`],
//!   [`Game::army_remove_requisition`], [`Game::army_set_requisition`]). A city lost leaves every
//!   list (`city_army_lost`).
//! - **The shortfall** ([`Game::army_shortfall`]): the target size less the army's units, its
//!   recruits marching to it, its requisition orders in production and the other orders bound for
//!   it at the head of a city's queue (the first order that is not a requisition). No target size
//!   (0), no requisition.
//! - **Once a turn** ([`Game::player_do_requisition`], in the owner's turn processing before its
//!   cities produce, so a new requisition takes this turn's production): orders without a reason
//!   any more (the army gone or disbanding, the city no longer on its list) are withdrawn; for
//!   each army, while the shortfall is positive, a picked city without a requisition order for it
//!   gets one, one unit per city at a time, the city whose unit would join soonest first
//!   (production turns + march turns; ties: the order the cities were picked in); orders beyond
//!   the shortfall (the army filled up another way) are withdrawn, those that would join last
//!   first. What was put into a withdrawn order stays with the unit type, as for any order taken
//!   off a queue.
//! - **The order** is an `ORDER_TRAIN` at the front of the city's queue (`OrderData::dest` the army,
//!   `OrderData::requisition`): it interrupts what the city was building, which keeps its
//!   progress and resumes after it. Requisition orders stay at the front: an order pushed to the
//!   front goes behind them, and replacing the head order replaces the first one behind them
//!   (`city_push_order`). The player cannot take one off by hand
//!   ([`Game::city_check_remove_order`]): the city is released from the army's requisition.
//! - **The unit** is the controller's decision (`Controller::requisition_unit`); the rules' default
//!   ([`Game::requisition_unit_default`], the human's): siege while siege is less than a third of
//!   the army (its marching recruits and requisition orders counted), else the strongest unit
//!   against cities, else the strongest attacker; only army units the city can train (not a hero:
//!   no army takes one), and no world or national unit. A picked city that can train none is
//!   skipped.

use std::cmp::Reverse;

use ffh_data::*;

use super::march::with_article;
use super::{ArmyResult, RecruitDest};
use crate::game::Game;
use crate::ids::*;

/// The march turns an army out of reach by land counts for, in the requisition's order.
const NO_WAY: i32 = 1000;

impl Game {
    // ---------------------------------------------------------------------------------------
    // Commands (the human's Armies screen; the AI uses them too)
    // ---------------------------------------------------------------------------------------

    /// Whether `p` may change army `a`'s requisition (else why not).
    pub fn army_check_requisition(&self, p: PlayerId, a: ArmyId) -> ArmyResult {
        if !self.armies_on() {
            return Err("armies are off in this game");
        }
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        if self.armies[a].disband_to.is_some() {
            return Err("the army is disbanding");
        }
        Ok(())
    }

    /// Whether army `a` of `p` may requisition from city `c` (else why not).
    pub fn army_check_requisition_city(&self, p: PlayerId, a: ArmyId, c: CityId) -> ArmyResult {
        self.army_check_requisition(p, a)?;
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("not a city of yours");
        }
        Ok(())
    }

    /// Army `a` requisitions from city `c` too (last in its list).
    pub fn army_add_requisition(&mut self, p: PlayerId, a: ArmyId, c: CityId) -> ArmyResult {
        self.army_check_requisition_city(p, a, c)?;
        if self.armies[a].requisition.contains(&c) {
            return Err("the army already requisitions from this city");
        }
        self.armies[a].requisition.push(c);
        Ok(())
    }

    /// Army `a` no longer requisitions from city `c`: its requisition order there is withdrawn
    /// (what was put into it stays with the unit type).
    pub fn army_remove_requisition(&mut self, p: PlayerId, a: ArmyId, c: CityId) -> ArmyResult {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("no such army");
        }
        if !self.armies[a].requisition.contains(&c) {
            return Err("the army does not requisition from this city");
        }
        self.armies[a].requisition.retain(|&x| x != c);
        if self.cities.contains(c) {
            self.city_withdraw_requisitions(c, a, true);
        }
        Ok(())
    }

    /// Army `a` requisitions from `cities` (in that order; a city twice counts once): the orders
    /// of the cities it no longer requisitions from are withdrawn.
    pub fn army_set_requisition(&mut self, p: PlayerId, a: ArmyId, cities: &[CityId]) -> ArmyResult {
        self.army_check_requisition(p, a)?;
        let mut list: Vec<CityId> = Vec::new();
        for &c in cities {
            self.army_check_requisition_city(p, a, c)?;
            if !list.contains(&c) {
                list.push(c);
            }
        }
        let old = std::mem::replace(&mut self.armies[a].requisition, list);
        for c in old {
            if !self.armies[a].requisition.contains(&c) && self.cities.contains(c) {
                self.city_withdraw_requisitions(c, a, true);
            }
        }
        Ok(())
    }

    /// Whether the order at `index` of city `c`'s queue may be taken off by hand (else why not): a
    /// requisition order may not (the city is released from the army's requisition instead).
    pub fn city_check_remove_order(&self, c: CityId, index: usize) -> Result<(), String> {
        match self.cities.get(c).and_then(|city| city.order_queue.get(index)) {
            Some(o) if o.requisition => {
                let by = o.dest.map_or_else(|| "an army".to_string(), |d| self.recruit_dest_name(d));
                Err(format!("requisitioned by {by}: release the city from the army's requisition on the Armies screen"))
            }
            _ => Ok(()),
        }
    }

    // ---------------------------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------------------------

    /// Army `a`'s requisition orders in production: (city, index in its queue), in the order of
    /// its owner's cities.
    pub fn army_requisition_orders(&self, a: ArmyId) -> Vec<(CityId, usize)> {
        let Some(army) = self.armies.get(a) else { return Vec::new() };
        let mut out = Vec::new();
        for &c in &self.players[army.owner].cities {
            for (i, o) in self.cities[c].order_queue.iter().enumerate() {
                if o.requisition && o.dest == Some(RecruitDest::Army(a)) {
                    out.push((c, i));
                }
            }
        }
        out
    }

    /// Whether city `c` has a requisition order for army `a`.
    fn city_has_requisition(&self, c: CityId, a: ArmyId) -> bool {
        self.cities[c].order_queue.iter().any(|o| o.requisition && o.dest == Some(RecruitDest::Army(a)))
    }

    /// The orders bound for army `a` at the head of its owner's cities' queues that are not
    /// requisitions (the first order behind any requisition orders: what the city builds next).
    fn army_head_orders(&self, a: ArmyId) -> i32 {
        let owner = self.armies[a].owner;
        let mut n = 0;
        for &c in &self.players[owner].cities {
            let city = &self.cities[c];
            let Some(o) = city.order_queue.iter().find(|o| !o.requisition) else { continue };
            if o.order_type != OrderType::ORDER_TRAIN || o.data1 < 0 || o.data1 as usize >= self.d.units.len() {
                continue;
            }
            let unit = UnitType::from_usize(o.data1 as usize);
            if self.city_recruit_dest(c, o.dest, unit) == Some(RecruitDest::Army(a)) {
                n += 1;
            }
        }
        n
    }

    /// How many units army `a` lacks of its target size: the target less its units, its recruits
    /// marching to it, its requisition orders in production and the other orders bound for it at
    /// the head of a city's queue. Negative: more are coming than it needs.
    pub fn army_shortfall(&self, a: ArmyId) -> i32 {
        let army = &self.armies[a];
        let have = self.army_units(a).len() + self.army_marching(a).len() + self.army_requisition_orders(a).len();
        army.target_size - have as i32 - self.army_head_orders(a)
    }

    /// The turns until a unit of `unit` that city `c` trains (its order at `index` of the queue)
    /// would join army `a`: its production turns and its march (estimated).
    fn requisition_eta(&self, c: CityId, unit: UnitType, index: usize, a: ArmyId) -> i32 {
        let owner = self.cities[c].owner;
        let production = self.city_production_turns_left_unit(c, unit, index as i32).min(NO_WAY);
        let march = self.army_plot(a).and_then(|at| self.march_turns(owner, unit, self.city_plot(c), at)).unwrap_or(NO_WAY);
        production + march
    }

    /// What city `c` would train for army `a`'s requisition: the unit and its UnitAI (the
    /// controller's choice), its production turns there and its march to the army (estimated;
    /// `None`: no way by land). `None` when the city can train no unit for it.
    pub fn army_requisition_preview(&self, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType, i32, Option<i32>)> {
        let owner = self.cities[c].owner;
        let (unit, ai) = self.controller(owner).requisition_unit(self, c, a)?;
        let production = self.city_production_turns_left_unit(c, unit, 0);
        let march = self.army_plot(a).and_then(|at| self.march_turns(owner, unit, self.city_plot(c), at));
        Some((unit, ai, production, march))
    }

    /// Whether units of `unit` are siege (`UNITCOMBAT_SIEGE`).
    pub fn unit_type_is_siege(&self, unit: UnitType) -> bool {
        let siege = self.d.ty::<UnitCombatType>("UNITCOMBAT_SIEGE");
        siege.is_some() && self.d.units[unit].unit_combat_type == siege
    }

    /// Whether the rules' default requisition may pick units of `unit_type`: army units whose
    /// class has no instance limit. A world or national unit (a hero, a Paladin, a Knight: a few
    /// a civilization or one in the world) is its owner's to place, not the requisition's.
    pub fn unit_type_is_requisition_default(&self, unit_type: UnitType) -> bool {
        self.unit_type_is_army_unit(unit_type) && !self.d.units[unit_type].unit_class_type.is_some_and(|k| crate::utils::is_limited_unit_class(self.d, k))
    }

    /// The rules' choice of the unit city `c` trains for army `a`'s requisition (the default of
    /// `Controller::requisition_unit`, the human's): siege while siege is less than a third of the
    /// army (its recruits marching to it and its requisition orders counted), else the strongest
    /// unit against cities (a city attacker), else the strongest attacker; only army units the
    /// city can train, and no world or national unit
    /// ([`Game::unit_type_is_requisition_default`]). Strength against cities: strength times its
    /// city attack bonus; ties: the cheaper, then the first.
    pub fn requisition_unit_default(&self, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
        let d = self.d;
        let mut total = 0;
        let mut siege = 0;
        for u in self.army_units(a).into_iter().chain(self.army_marching(a)) {
            total += 1;
            siege += i32::from(self.unit_type_is_siege(self.units[u].unit_type));
        }
        for (oc, i) in self.army_requisition_orders(a) {
            let o = &self.cities[oc].order_queue[i];
            total += 1;
            siege += i32::from(o.data1 >= 0 && (o.data1 as usize) < d.units.len() && self.unit_type_is_siege(UnitType::from_usize(o.data1 as usize)));
        }
        let candidates: Vec<UnitType> = d.units.keys().filter(|&t| self.unit_type_is_requisition_default(t) && self.city_can_train(c, t, false, false, false, false)).collect();
        let best = |pred: &dyn Fn(UnitType) -> bool| -> Option<UnitType> {
            candidates
                .iter()
                .copied()
                .filter(|&t| pred(t))
                .max_by_key(|&t| (d.units[t].combat * (100 + d.units[t].city_attack_modifier), Reverse(d.units[t].production_cost), Reverse(t)))
        };
        if siege * 3 < total {
            if let Some(t) = best(&|t| self.unit_type_is_siege(t)) {
                let ai = if d.units[t].unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY] { UnitAIType::UNITAI_ATTACK_CITY } else { d.units[t].default_unit_ai_type.unwrap_or(UnitAIType::UNITAI_ATTACK_CITY) };
                return Some((t, ai));
            }
        }
        best(&|t| !self.unit_type_is_siege(t) && d.units[t].unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY])
            .or_else(|| best(&|t| d.units[t].unit_ai_type[UnitAIType::UNITAI_ATTACK]))
            .map(|t| (t, UnitAIType::UNITAI_ATTACK))
    }

    // ---------------------------------------------------------------------------------------
    // The turn
    // ---------------------------------------------------------------------------------------

    /// Player `p`'s requisition, in its turn processing before its cities produce: orders without
    /// a reason withdrawn, then for each army (in formation order) orders placed while it is short
    /// of its target size, or withdrawn beyond it (see the module comment).
    pub fn player_do_requisition(&mut self, p: PlayerId) {
        if !self.armies_on() || self.player_is_barbarian(p) {
            return;
        }
        // Orders without a reason any more.
        for c in self.players[p].cities.clone() {
            let mut i = 0;
            while i < self.cities[c].order_queue.len() {
                let o = &self.cities[c].order_queue[i];
                if o.requisition {
                    let keep = match o.dest {
                        Some(RecruitDest::Army(a)) => self.armies.get(a).is_some_and(|ar| ar.owner == p && ar.disband_to.is_none() && ar.requisition.contains(&c)),
                        _ => false,
                    };
                    if !keep {
                        self.city_withdraw_order(c, i, true);
                        continue;
                    }
                }
                i += 1;
            }
        }
        let controller = self.controller(p);
        for a in self.players[p].armies.clone() {
            // (An army without cities to requisition from has no orders left after the above.)
            if !self.armies.contains(a) || self.armies[a].disband_to.is_some() || self.armies[a].requisition.is_empty() {
                continue;
            }
            let mut short = self.army_shortfall(a);
            if short < 0 {
                // The army filled up another way: the orders that would join last go first.
                let mut orders: Vec<(i32, usize, CityId, usize)> = self
                    .army_requisition_orders(a)
                    .into_iter()
                    .map(|(c, i)| {
                        let unit = UnitType::from_usize(self.cities[c].order_queue[i].data1.max(0) as usize);
                        let pos = self.armies[a].requisition.iter().position(|&x| x == c).unwrap_or(usize::MAX);
                        (self.requisition_eta(c, unit, i, a), pos, c, i)
                    })
                    .collect();
                orders.sort_by_key(|&(eta, pos, c, i)| Reverse((eta, pos, c, i)));
                // (The last index of a city first, so that the others stay valid: a merged army may
                // have two orders in one city.)
                let mut gone: Vec<(CityId, usize)> = orders.iter().take((-short) as usize).map(|&(_, _, c, i)| (c, i)).collect();
                gone.sort_by_key(|&(c, i)| (c, Reverse(i)));
                for (c, i) in gone {
                    self.city_withdraw_order(c, i, true);
                }
                continue;
            }
            while short > 0 {
                let mut best: Option<((i32, usize), CityId, UnitType, UnitAIType)> = None;
                for (pos, c) in self.armies[a].requisition.clone().into_iter().enumerate() {
                    if !self.cities.contains(c) || self.cities[c].owner != p || self.city_has_requisition(c, a) {
                        continue;
                    }
                    let Some((unit, ai)) = controller.requisition_unit(self, c, a) else { continue };
                    if !self.unit_type_is_army_unit(unit) || !self.city_can_train(c, unit, false, false, false, false) {
                        continue;
                    }
                    let key = (self.requisition_eta(c, unit, 0, a), pos);
                    if best.as_ref().map_or(true, |(k, ..)| key < *k) {
                        best = Some((key, c, unit, ai));
                    }
                }
                let Some((_, c, unit, ai)) = best else { break };
                if !self.city_place_requisition(c, a, unit, ai) {
                    break;
                }
                short -= 1;
            }
        }
    }

    /// City `c` trains a unit of `unit` for army `a` first: a requisition order at the front of its
    /// queue (behind any other requisition orders). Returns whether it was placed.
    fn city_place_requisition(&mut self, c: CityId, a: ArmyId, unit: UnitType, ai: UnitAIType) -> bool {
        let lead = self.cities[c].order_queue.iter().take_while(|o| o.requisition).count();
        let len = self.cities[c].order_queue.len();
        self.city_push_order(c, OrderType::ORDER_TRAIN, unit.i(), ai.i(), false, false, false, false);
        if !self.cities.contains(c) || self.cities[c].order_queue.len() <= len {
            return false;
        }
        let o = &mut self.cities[c].order_queue[lead];
        o.dest = Some(RecruitDest::Army(a));
        o.requisition = true;
        let p = self.cities[c].owner;
        self.tel_requisition(p, "placed");
        let turn = self.game_turn();
        let city = self.d.text.name(&self.cities[c].name).to_string();
        let unit_name = with_article(self.d.text.name(&self.d.units[unit].base.description));
        let mut army = self.recruit_dest_name(RecruitDest::Army(a));
        if let Some(first) = army.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        self.log.msg_at(turn, Some(p), (self.cities[c].x, self.cities[c].y), || format!("{army} requisitions {unit_name} from {city}: it trains it first."));
        true
    }

    /// City `c`'s requisition orders for army `a` are withdrawn (with `choose`, an emptied queue
    /// chooses its production at once).
    pub(crate) fn city_withdraw_requisitions(&mut self, c: CityId, a: ArmyId, choose: bool) {
        let mut i = 0;
        while i < self.cities[c].order_queue.len() {
            let o = &self.cities[c].order_queue[i];
            if o.requisition && o.dest == Some(RecruitDest::Army(a)) {
                self.city_withdraw_order(c, i, choose);
                continue;
            }
            i += 1;
        }
    }

    /// The requisition order at `index` of city `c`'s queue is withdrawn: taken off the queue
    /// (what was put into it stays with the unit type), its owner told.
    fn city_withdraw_order(&mut self, c: CityId, index: usize, choose: bool) {
        let o = self.cities[c].order_queue[index].clone();
        let p = self.cities[c].owner;
        let city = self.d.text.name(&self.cities[c].name).to_string();
        let at = (self.cities[c].x, self.cities[c].y);
        self.city_pop_order(c, index as i32, false, choose);
        self.tel_requisition(p, "withdrawn");
        let turn = self.game_turn();
        let unit_name = match o.data1 {
            k if k >= 0 && (k as usize) < self.d.units.len() => with_article(self.d.text.name(&self.d.units[UnitType::from_usize(k as usize)].base.description)),
            _ => "a unit".to_string(),
        };
        let army = o.dest.map_or_else(|| "an army".to_string(), |d| self.recruit_dest_name(d));
        self.log.msg_at(turn, Some(p), at, || format!("{city} no longer trains {unit_name} for {army}: its requisition there is withdrawn."));
    }
}
