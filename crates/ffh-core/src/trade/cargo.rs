//! Trade (docs/TRADE.md, section 5): cargo. A caravan of a foreign route may carry one resource
//! of its owner's to the destination's owner.
//!
//! **Delivering** is a pure function of the state ([`Game::cargo_fresh`]): a slot with a cargo
//! delivers while its caravan lives, its route is open, has arrived and is foreign, and its owner
//! has a copy to send. The owner's copies to send are its tradeable copies with every copy its
//! caravans export now added back (so the count does not depend on its own result), shared out
//! among its delivering slots of that resource by priority: the older route (`Route::established`),
//! then the lower route id, then the lower slot. What a slot delivers is applied as a deal's
//! resource is: one export of the owner and one import of the recipient
//! (`player_change_bonus_export` / `_import`), kept in `CaravanSlot::delivering` (the recipient and
//! the resource, so it is undone exactly when the recipient changes).
//!
//! **The premium** of a delivering slot is `TRADE_CARGO_NEW_PERCENT` while the recipient has no
//! copy of the resource apart from caravans' deliveries and the slot is the first, by the same
//! priority, of the caravans delivering it that resource; else `TRADE_CARGO_PERCENT`. It is kept in
//! `CaravanSlot::premium` and multiplies the slot's half of the pay.

use std::collections::BTreeMap;

use crate::game::Game;

use super::*;

/// Why a slot delivers its cargo or not (docs/TRADE.md, 5 and 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoState {
    /// It carries no cargo.
    NoCargo,
    /// It delivers.
    Delivering,
    /// Its caravan is dead.
    CaravanLost,
    /// The route is closed.
    RouteClosed,
    /// The route's first caravan has not arrived.
    NotArrived,
    /// The route is domestic now (its destination became the owner's).
    Domestic,
    /// The owner has no copy to send.
    NoCopy,
}

impl CargoState {
    /// The bare reason.
    pub fn text(self) -> &'static str {
        match self {
            CargoState::NoCargo => "No cargo.",
            CargoState::Delivering => "Delivered.",
            CargoState::CaravanLost => "Caravan lost.",
            CargoState::RouteClosed => "Route closed.",
            CargoState::NotArrived => "Not arrived.",
            CargoState::Domestic => "Domestic route.",
            CargoState::NoCopy => "No copy to send.",
        }
    }
}

/// What kind of resource a cargo is (docs/TRADE.md, section 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoClass {
    /// Happiness or health above 0.
    Luxury,
    /// Mana.
    Mana,
    /// Every other resource.
    Strategic,
}

impl CargoClass {
    fn key(self) -> &'static str {
        match self {
            CargoClass::Luxury => "luxury",
            CargoClass::Mana => "mana",
            CargoClass::Strategic => "strategic",
        }
    }
}

/// A resource the owner could load on a route between two cities ([`Game::route_cargo_options`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoOption {
    pub bonus: BonusType,
    pub class: CargoClass,
    /// The owner's copies of it no other caravan carries.
    pub free_copies: i32,
    /// Loading it leaves the owner's cities without it.
    pub only_copy: bool,
    /// The premium it would earn now (percent).
    pub premium_percent: i32,
}

/// A route slot's cargo as the view reads it ([`Game::route_slot_cargo`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoView {
    pub cargo: Option<BonusType>,
    pub state: CargoState,
    /// Its premium now (percent; 0 while it does not deliver).
    pub premium_percent: i32,
}

/// The fresh cargo state: what each slot delivers (recipient, resource) and its premium.
pub(crate) type CargoMap = BTreeMap<(RouteId, usize), ((PlayerId, BonusType), i32)>;

impl Game {
    /// The class of resource `b`.
    pub fn bonus_cargo_class(&self, b: BonusType) -> CargoClass {
        let info = &self.d.bonuses[b];
        if crate::mana::is_mana(self.d, b) {
            CargoClass::Mana
        } else if info.happiness > 0 || info.health > 0 {
            CargoClass::Luxury
        } else {
            CargoClass::Strategic
        }
    }

    /// The priority of slot `slot` of route `r` among caravans: the older route, the lower id,
    /// the lower slot.
    fn cargo_key(&self, r: RouteId, slot: usize) -> (i32, RouteId, usize) {
        (self.routes[r].established, r, slot)
    }

    /// The copies of `b` player `p`'s caravans export now (as applied).
    fn cargo_exports_applied(&self, p: PlayerId, b: BonusType) -> i32 {
        self.routes.iter().filter(|(_, r)| r.owner == p).map(|(_, r)| r.slots.iter().filter(|s| s.delivering.is_some_and(|(_, x)| x == b)).count() as i32).sum()
    }

    /// The copies of `b` player `p` receives from caravans now (as applied).
    fn cargo_imports_applied(&self, p: PlayerId, b: BonusType) -> i32 {
        self.routes.iter().map(|(_, r)| r.slots.iter().filter(|s| s.delivering == Some((p, b))).count() as i32).sum()
    }

    /// Player `p`'s own copies of `b` to send by caravan: its tradeable copies (its own, not
    /// imported) with what its caravans export now added back.
    pub fn player_cargo_copies(&self, p: PlayerId, b: BonusType) -> i32 {
        (self.player_get_num_tradeable_bonuses(p, b) + self.cargo_exports_applied(p, b)).max(0)
    }

    /// Whether player `p` has a copy of `b` apart from caravans' deliveries (its own or a deal's
    /// import; its own caravans' exports added back).
    pub fn player_has_bonus_apart_from_caravans(&self, p: PlayerId, b: BonusType) -> bool {
        self.player_get_num_available_bonuses(p, b) - self.cargo_imports_applied(p, b) + self.cargo_exports_applied(p, b) > 0
    }

    /// Why slot `slot` of route `r` could not deliver whatever its owner's copies (`None`: it
    /// could, given a copy).
    fn cargo_blocked(&self, r: RouteId, slot: usize) -> Option<CargoState> {
        let route = &self.routes[r];
        let s = &route.slots[slot];
        if s.cargo.is_none() {
            return Some(CargoState::NoCargo);
        }
        // A caravan that was killed is lost whatever its route's state. Otherwise the route's
        // state comes first: a closed route's caravans wait in the origin, and the second
        // caravan does not exist before the first arrival; neither is lost.
        if s.unit.is_none() && s.reform_turns > 0 {
            return Some(CargoState::CaravanLost);
        }
        if !route.is_open() {
            return Some(CargoState::RouteClosed);
        }
        if !route.arrived {
            return Some(CargoState::NotArrived);
        }
        if s.unit.is_none() {
            return Some(CargoState::CaravanLost);
        }
        if !self.cities.contains(route.destination) || !self.cities.contains(route.origin) || self.cities[route.destination].owner == route.owner {
            return Some(CargoState::Domestic);
        }
        None
    }

    /// The fresh cargo state (module comment): what each slot delivers now and its premium. It
    /// reads the applied state only to add it back, so it does not depend on its own result.
    pub(crate) fn cargo_fresh(&self) -> CargoMap {
        let d = &self.d.defines;
        let mut claims: Vec<((i32, RouteId, usize), PlayerId, PlayerId, BonusType)> = Vec::new();
        for (r, route) in self.routes.iter() {
            for slot in 0..CARAVAN_SLOTS {
                if self.cargo_blocked(r, slot).is_some() {
                    continue;
                }
                let b = route.slots[slot].cargo.expect("a cargo");
                claims.push((self.cargo_key(r, slot), route.owner, self.cities[route.destination].owner, b));
            }
        }
        claims.sort();
        let mut used: BTreeMap<(PlayerId, BonusType), i32> = BTreeMap::new();
        let mut delivering: Vec<((i32, RouteId, usize), PlayerId, BonusType)> = Vec::new();
        for &(key, owner, rec, b) in &claims {
            let n = used.entry((owner, b)).or_insert(0);
            if *n < self.player_cargo_copies(owner, b) {
                *n += 1;
                delivering.push((key, rec, b));
            }
        }
        let mut out = CargoMap::new();
        let mut first: BTreeMap<(PlayerId, BonusType), bool> = BTreeMap::new();
        for &((est, r, slot), rec, b) in &delivering {
            let _ = est;
            let is_first = !first.contains_key(&(rec, b));
            first.insert((rec, b), true);
            let premium = if is_first && !self.player_has_bonus_apart_from_caravans(rec, b) { d.trade_cargo_new_percent } else { d.trade_cargo_percent };
            out.insert((r, slot), ((rec, b), premium));
        }
        out
    }

    /// The cargo refresh: the fresh state applied (exports and imports changed by the difference,
    /// as a deal's resource), the premiums stored; messages on a change. Part of
    /// [`Game::trade_refresh_pay`].
    pub(crate) fn cargo_refresh(&mut self) {
        let fresh = self.cargo_fresh();
        let turn = self.game_turn();
        for r in self.routes.ids() {
            for slot in 0..CARAVAN_SLOTS {
                let old = self.routes[r].slots[slot].delivering;
                let old_premium = self.routes[r].slots[slot].premium;
                let (new, premium) = match fresh.get(&(r, slot)) {
                    Some(&(v, pr)) => (Some(v), pr),
                    None => (None, 0),
                };
                if old != new {
                    let owner = self.routes[r].owner;
                    if let Some((rec, b)) = old {
                        self.player_change_bonus_export(owner, b, -1);
                        self.player_change_bonus_import(rec, b, -1);
                    }
                    self.routes[r].slots[slot].delivering = new;
                    if let Some((rec, b)) = new {
                        self.player_change_bonus_export(owner, b, 1);
                        self.player_change_bonus_import(rec, b, 1);
                    }
                    let at = self.route_origin_xy(r);
                    match (old, new) {
                        (Some((_, b)), None) => {
                            let why = self.cargo_blocked(r, slot).unwrap_or(CargoState::NoCopy).text();
                            let name = self.bonus_name(b);
                            self.log.msg_at(turn, Some(owner), at, || format!("{name}: delivery stopped. {why}"));
                        }
                        (_, Some((rec, b))) => {
                            let (name, civ) = (self.bonus_name(b), self.player_get_civilization_short_description(rec));
                            self.log.msg_at(turn, Some(owner), at, || format!("{name}: delivered to {civ}. Premium +{premium}%."));
                        }
                        _ => {}
                    }
                } else if let Some((rec, b)) = new.filter(|_| old_premium != premium) {
                    let owner = self.routes[r].owner;
                    let at = self.route_origin_xy(r);
                    let (name, civ) = (self.bonus_name(b), self.player_get_civilization_short_description(rec));
                    let has = self.player_has_bonus_apart_from_caravans(rec, b);
                    self.log.msg_at(turn, Some(owner), at, || if has && premium < old_premium { format!("{civ} now has {name}. Premium +{premium}%.") } else { format!("{name} to {civ}: premium +{premium}%.") });
                }
                self.routes[r].slots[slot].premium = premium;
            }
        }
    }

    /// Undoes what route `r`'s slots deliver (before it ends).
    pub(crate) fn cargo_undo_route(&mut self, r: RouteId) {
        let owner = self.routes[r].owner;
        for slot in 0..CARAVAN_SLOTS {
            if let Some((rec, b)) = self.routes[r].slots[slot].delivering.take() {
                self.player_change_bonus_export(owner, b, -1);
                self.player_change_bonus_import(rec, b, -1);
            }
            self.routes[r].slots[slot].premium = 0;
        }
    }

    /// The disagreements of the stored cargo state with a fresh computation (for `trade_stale`).
    pub(crate) fn cargo_stale(&self) -> Vec<String> {
        let fresh = self.cargo_fresh();
        let mut out = Vec::new();
        for (r, route) in self.routes.iter() {
            for slot in 0..CARAVAN_SLOTS {
                let (new, premium) = match fresh.get(&(r, slot)) {
                    Some(&(v, pr)) => (Some(v), pr),
                    None => (None, 0),
                };
                let s = &route.slots[slot];
                if s.delivering != new || s.premium != premium {
                    out.push(format!("route {r:?}: slot {slot} delivers {:?} at {}% != {new:?} at {premium}%", s.delivering, s.premium));
                }
            }
        }
        out
    }

    fn bonus_name(&self, b: BonusType) -> String {
        self.d.text.name(&self.d.bonuses[b].base.description).to_string()
    }

    /// Telemetry of player `p`'s cargo at its turn start: cargo-turns delivered by class, premium
    /// turns by level (`new` / `cargo`).
    pub(crate) fn cargo_telemetry(&self, p: PlayerId) {
        if !self.telemetry.on() {
            return;
        }
        let d = &self.d.defines;
        for (_, r) in self.routes.iter().filter(|(_, r)| r.owner == p) {
            for s in &r.slots {
                if let Some((_, b)) = s.delivering {
                    self.tel_trade(p, "trade_cargo_turn", self.bonus_cargo_class(b).key(), 1);
                    let level = if s.premium == d.trade_cargo_new_percent { "new" } else { "cargo" };
                    self.tel_trade(p, "trade_premium_turn", level, s.premium as i64);
                }
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Choosing (5, "Choosing")

    /// The copies of `b` player `p`'s caravans carry (their cargo, delivering or not), route
    /// `except`'s left out.
    fn cargo_carried(&self, p: PlayerId, b: BonusType, except: Option<RouteId>) -> i32 {
        self.routes.iter().filter(|(id, r)| r.owner == p && Some(*id) != except).map(|(_, r)| r.slots.iter().filter(|s| s.cargo == Some(b)).count() as i32).sum()
    }

    /// The premium a cargo of `b` from `owner` to `rec` would earn now on a route with priority
    /// key `key` (`TRADE_CARGO_NEW_PERCENT` when the recipient has no copy apart from caravans and
    /// no older caravan delivers it one, else `TRADE_CARGO_PERCENT`).
    fn cargo_premium_if(&self, rec: PlayerId, b: BonusType, key: (i32, RouteId, usize), except: Option<RouteId>) -> i32 {
        let d = &self.d.defines;
        if self.player_has_bonus_apart_from_caravans(rec, b) {
            return d.trade_cargo_percent;
        }
        let older = self.routes.iter().any(|(id, r)| Some(id) != except && r.slots.iter().enumerate().any(|(i, s)| s.delivering == Some((rec, b)) && self.cargo_key(id, i) < key));
        if older {
            d.trade_cargo_percent
        } else {
            d.trade_cargo_new_percent
        }
    }

    /// The resources `owner` could load on a route from `origin` to `dest` (docs/TRADE.md, 5:
    /// foreign routes only), in type order; `route` names the route being refitted (its own
    /// caravans' copies count as free). Pure.
    pub fn route_cargo_options(&self, owner: PlayerId, origin: CityId, dest: CityId, route: Option<RouteId>) -> Vec<CargoOption> {
        if !self.cities.contains(origin) || !self.cities.contains(dest) || self.cities[dest].owner == owner {
            return Vec::new();
        }
        let rec = self.cities[dest].owner;
        let key = match route.filter(|&r| self.routes.contains(r)) {
            Some(r) => self.cargo_key(r, 0),
            None => (self.game_turn(), self.routes.next_id(), 0),
        };
        let mut out = Vec::new();
        for b in self.d.bonuses.keys() {
            let free = self.player_cargo_copies(owner, b) - self.cargo_carried(owner, b, route);
            if free <= 0 {
                continue;
            }
            let own_export = route.filter(|&r| self.routes.contains(r)).map_or(0, |r| self.routes[r].slots.iter().filter(|s| s.delivering.is_some_and(|(_, x)| x == b)).count() as i32);
            let left = self.player_get_num_available_bonuses(owner, b) + own_export - 1;
            out.push(CargoOption { bonus: b, class: self.bonus_cargo_class(b), free_copies: free, only_copy: left <= 0, premium_percent: self.cargo_premium_if(rec, b, key, route) });
        }
        out
    }

    /// Why `cargo` cannot be loaded on a route of `owner` from `origin` to `dest` (`route`: the
    /// one being refitted), `None` when it can.
    pub fn route_cargo_refusal(&self, owner: PlayerId, _origin: CityId, dest: CityId, route: Option<RouteId>, cargo: [Option<BonusType>; CARAVAN_SLOTS]) -> Option<RouteRefusal> {
        if cargo.iter().all(|c| c.is_none()) {
            return None;
        }
        if !self.cities.contains(dest) || self.cities[dest].owner == owner {
            return Some(RouteRefusal::CargoDomestic);
        }
        if cargo[0].is_some() && cargo[0] == cargo[1] {
            return Some(RouteRefusal::CargoSame);
        }
        for b in cargo.iter().flatten().copied() {
            if self.player_cargo_copies(owner, b) - self.cargo_carried(owner, b, route) <= 0 {
                return Some(RouteRefusal::CargoNoCopy(b));
            }
        }
        None
    }

    /// Slot `slot` of route `r`'s cargo as the view reads it.
    pub fn route_slot_cargo(&self, r: RouteId, slot: usize) -> CargoView {
        let s = &self.routes[r].slots[slot];
        let state = if s.delivering.is_some() { CargoState::Delivering } else { self.cargo_blocked(r, slot).unwrap_or(CargoState::NoCopy) };
        CargoView { cargo: s.cargo, state, premium_percent: s.premium }
    }

    // ---------------------------------------------------------------------------------------
    // Refit (5, "Fixed")

    /// Why Caravan `u` cannot refit route `r` with `cargo`, `None` when it can: a Caravan in no
    /// route, standing in the origin of `r`, a route of its owner; the cargo refusals of 5 (the
    /// route's own caravans' copies count as free).
    pub fn unit_refit_refusal(&self, u: UnitId, r: RouteId, cargo: [Option<BonusType>; CARAVAN_SLOTS]) -> Option<RouteRefusal> {
        if !self.unit_is_caravan(u) {
            return Some(RouteRefusal::NotACaravan);
        }
        if self.units[u].route.is_some() {
            return Some(RouteRefusal::InRoute);
        }
        if !self.routes.contains(r) {
            return Some(RouteRefusal::RouteGone);
        }
        let route = &self.routes[r];
        let owner = self.units[u].owner;
        let here = self.unit_plot_opt(u);
        if route.owner != owner || !self.cities.contains(route.origin) || here != Some(self.city_plot(route.origin)) {
            return Some(RouteRefusal::NotInOrigin);
        }
        self.route_cargo_refusal(owner, route.origin, route.destination, Some(r), cargo)
    }

    /// Caravan `u` refits route `r` (docs/TRADE.md, 5): both caravans' cargos are set again, the
    /// unit is used up, the route keeps its growth.
    pub fn unit_refit_route(&mut self, u: UnitId, r: RouteId, cargo: [Option<BonusType>; CARAVAN_SLOTS]) -> Result<(), RouteRefusal> {
        if let Some(why) = self.unit_refit_refusal(u, r, cargo) {
            return Err(why);
        }
        for slot in 0..CARAVAN_SLOTS {
            self.routes[r].slots[slot].cargo = cargo[slot];
        }
        self.tel_ctx_push("trade");
        self.unit_kill(u, false, None);
        self.tel_ctx_pop();
        let owner = self.routes[r].owner;
        let (on, dn) = (self.city_name_or(self.routes[r].origin), self.city_name_or(self.routes[r].destination));
        let (turn, at) = (self.game_turn(), self.route_origin_xy(r));
        self.log.msg_at(turn, Some(owner), at, || format!("Route {on} to {dn} refitted."));
        self.trade_refresh_pay();
        Ok(())
    }
}
