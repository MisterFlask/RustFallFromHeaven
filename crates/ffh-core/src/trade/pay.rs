//! Trade (docs/TRADE.md): pay (6.1, 6.3), the refresh and its check.

use crate::game::Game;

use super::*;

impl Game {
    // ---------------------------------------------------------------------------------------
    // Pay (6.1)

    /// The effective length of a route of `length` plots: in full to `TRADE_LENGTH_FULL`, at half
    /// (rounded down) from there to `TRADE_LENGTH_CAP`, nothing beyond.
    pub fn route_effective_length(&self, length: i32) -> i32 {
        let d = &self.d.defines;
        let full = d.trade_length_full.max(0);
        let cap = d.trade_length_cap.max(full);
        let l = length.max(0);
        if l <= full {
            l
        } else {
            full + (l.min(cap) - full) / 2
        }
    }

    /// The percent terms of a route's pay (6.1): foreign, water on the path, the origin's
    /// modifiers, growth.
    pub fn route_pay_percent(&self, owner: PlayerId, origin: CityId, dest: CityId, water: bool, growth: i32) -> i32 {
        let d = &self.d.defines;
        let foreign = self.cities[dest].owner != owner;
        let mut pct = 0;
        if foreign {
            pct += d.trade_foreign_percent;
            pct += self.cities[origin].foreign_trade_route_modifier;
        }
        if water {
            pct += d.overseas_trade_modifier;
        }
        pct += self.cities[origin].trade_route_modifier;
        pct += growth;
        pct
    }

    /// A route's whole pay (6.1) before it is split between the caravans, in hundredths: the
    /// effective length x the two cities' population x `TRADE_PAY_PERCENT` / 100, then
    /// x (100 + the percent terms) / 100 (at least 0).
    pub fn route_pay_total_x100(&self, owner: PlayerId, origin: CityId, dest: CityId, length: i32, water: bool, growth: i32) -> i32 {
        if !self.cities.contains(origin) || !self.cities.contains(dest) {
            return 0;
        }
        let pop = self.cities[origin].population + self.cities[dest].population;
        let base = self.route_effective_length(length) as i64 * pop as i64 * self.d.defines.trade_pay_percent as i64 / 100;
        let pct = 100 + self.route_pay_percent(owner, origin, dest, water, growth) as i64;
        (base * pct / 100).clamp(0, i32::MAX as i64) as i32
    }

    /// Route `r`'s pay now (6.1 and 6.3), from its stored state: nothing while closed or before
    /// its first caravan arrived; each living caravan half (the remainder to the first), the
    /// destination `TRADE_DEST_PERCENT` of the living halves.
    pub fn route_pay(&self, r: RouteId) -> RoutePay {
        let route = &self.routes[r];
        let mut pay = RoutePay::default();
        if !route.is_open() || !route.arrived || !self.cities.contains(route.origin) || !self.cities.contains(route.destination) {
            return pay;
        }
        let water = route.path.iter().any(|&q| self.plot_is_water(q));
        let total = self.route_pay_total_x100(route.owner, route.origin, route.destination, route.length, water, route.growth);
        let halves = [total - total / 2, total / 2];
        let dest_pct = self.d.defines.trade_dest_percent;
        for i in 0..CARAVAN_SLOTS {
            if route.slots[i].unit.is_some() {
                // A delivered cargo's premium goes on its caravan's half (docs/TRADE.md, 5); the
                // destination's share is counted before it (6.3).
                let premium = if route.slots[i].delivering.is_some() { route.slots[i].premium } else { 0 };
                pay.caravan_x100[i] = (halves[i] as i64 * (100 + premium) as i64 / 100) as i32;
                pay.dest_x100 += halves[i] * dest_pct / 100;
            }
        }
        pay
    }

    /// City `c`'s trade, in hundredths, from the routes as they pay now.
    pub fn city_route_trade_x100(&self, c: CityId) -> i32 {
        let mut sum = 0;
        for (id, r) in self.routes.iter() {
            if r.origin != c && r.destination != c {
                continue;
            }
            let pay = self.route_pay(id);
            if r.origin == c {
                sum += pay.caravan_x100.iter().sum::<i32>();
            }
            if r.destination == c {
                sum += pay.dest_x100;
            }
        }
        sum
    }

    /// The trade yield city `c` should have (6.3): its routes' hundredths rounded to the nearest
    /// whole, through `city_calculate_trade_yield`.
    pub fn city_route_trade_yield(&self, c: CityId, y: YieldType) -> i32 {
        self.city_calculate_trade_yield(c, y, round_x100(self.city_route_trade_x100(c)))
    }

    /// The pay refresh (6.1, 6.3): every route's stored pay and every city's trade yield set
    /// from the routes' state now. Cheap; called whenever something it reads changes.
    pub fn trade_refresh_pay(&mut self) {
        // Cargo first (section 5): the pay reads what is delivered and its premium.
        self.cargo_refresh();
        for r in self.routes.ids() {
            let pay = self.route_pay(r);
            let route = &mut self.routes[r];
            for i in 0..CARAVAN_SLOTS {
                route.slots[i].pay_x100 = pay.caravan_x100[i];
            }
            route.dest_pay_x100 = pay.dest_x100;
        }
        for c in self.cities.ids() {
            for i in 0..YieldType::NUM {
                let y = YieldType::from_usize(i);
                let v = self.city_route_trade_yield(c, y);
                if self.cities[c].trade_yield[i] != v {
                    self.city_set_trade_yield(c, y, v);
                }
            }
        }
    }

    /// The disagreements between the stored trade state and a fresh computation (the harness's
    /// check): each city's trade yield, each route's stored pay, and routes and caravan units
    /// referring to each other. Empty when they agree.
    pub fn trade_stale(&self) -> Vec<String> {
        let mut out = self.cargo_stale();
        for (id, r) in self.routes.iter() {
            if !self.cities.contains(r.origin) || self.cities[r.origin].owner != r.owner {
                out.push(format!("route {id:?}: its origin is not its owner's city"));
            }
            if !self.cities.contains(r.destination) {
                out.push(format!("route {id:?}: its destination is gone"));
            }
            let pay = self.route_pay(id);
            for i in 0..CARAVAN_SLOTS {
                if r.slots[i].pay_x100 != pay.caravan_x100[i] {
                    out.push(format!("route {id:?}: caravan {i} pay {} != {}", r.slots[i].pay_x100, pay.caravan_x100[i]));
                }
                if let Some(u) = r.slots[i].unit {
                    if !self.units.contains(u) || self.units[u].route != Some(id) || self.units[u].owner != r.owner {
                        out.push(format!("route {id:?}: caravan {i} is not a living unit of the route"));
                    }
                }
            }
            if r.dest_pay_x100 != pay.dest_x100 {
                out.push(format!("route {id:?}: destination pay {} != {}", r.dest_pay_x100, pay.dest_x100));
            }
            if !r.is_open() && r.slots.iter().any(|s| s.unit.is_some()) {
                out.push(format!("route {id:?}: closed with a caravan on the map"));
            }
        }
        for (u, unit) in self.units.iter() {
            if let Some(r) = unit.route {
                if !self.routes.contains(r) || !self.routes[r].slots.iter().any(|s| s.unit == Some(u)) {
                    out.push(format!("unit {u:?}: in route {r:?}, which does not hold it"));
                }
            }
        }
        for c in self.cities.ids() {
            for i in 0..YieldType::NUM {
                let v = self.city_route_trade_yield(c, YieldType::from_usize(i));
                if self.cities[c].trade_yield[i] != v {
                    out.push(format!("city {}: trade yield {i} {} != {v}", self.cities[c].name, self.cities[c].trade_yield[i]));
                }
            }
        }
        out
    }

    // ---------------------------------------------------------------------------------------
    // The Great Merchant's trade mission

    /// The trade a Great Merchant's mission in city `c` counts (`unit_get_trade_gold`, which
    /// multiplies it by the unit's `trade_multiplier`): what one caravan of a route of `c`'s owner
    /// from `c` to `capital` would pay a turn at growth 0, the plot distance as its length, times
    /// `TRADE_MISSION_PERCENT` / 100, rounded to the nearest whole (docs/TRADE.md, "As built").
    pub fn trade_mission_profit(&self, c: CityId, capital: CityId) -> i32 {
        let owner = self.cities[c].owner;
        let (a, b) = (&self.cities[c], &self.cities[capital]);
        let length = self.plot_distance(a.x, a.y, b.x, b.y);
        let total = self.route_pay_total_x100(owner, c, capital, length, false, 0);
        let half = (total - total / 2) as i64;
        round_x100((half * self.d.defines.trade_mission_percent as i64 / 100) as i32)
    }
}
