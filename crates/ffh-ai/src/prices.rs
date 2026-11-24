//! What the AI asks for what it gives: the price, in a deal's trade points, of a city, a civic
//! change, a religion change, an embargo or peace with a third team, and the refusals where a
//! player would refuse outright (docs/AI.md, "Diplomacy"). `diplomacy::judge_offer` adds the
//! prices of what the AI gives to its side of a deal; `trade::item_value` shows them.
//!
//! The scale is the deal's (`trade::deal_val`): 1 research point of a technology is 1 point, 1
//! gold is `trade::gold_trade_value_percent` / 100 points (2 for most), `tribute::GIFT_TIER` is
//! 150. Every price is rounded down to a multiple of `diplomacy_value_remainder` (10). Maps are
//! valued in `trade::deal_val` itself ([`map_value`]), on either side of a deal.

use ffh_core::controller::OfferVerdict;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;

/// A city is priced at this many turns of its production, commerce and food surplus (plus a
/// settler and its world wonders).
pub const CITY_TURNS: i64 = 100;
/// A civilization with this many cities or fewer sells none (outside a war). The refusal's text
/// in [`city_refusal`] repeats the number: change both together.
pub const MIN_CITIES_TO_SELL: usize = 3;
/// A civilization sells one city to the same team in this many turns (a city it lost to that
/// team in a war counts too). The refusal's text in [`city_refusal`] repeats the number: change
/// both together.
pub const CITY_SALE_TURNS: i32 = 30;
/// The most a civic change can lose, in percent of `peace_treaty_length` turns of output: the
/// loss for going from the best civic of the option `p` can run to the worst.
pub const CIVIC_SWING_PCT: i64 = 25;
/// Religion: points per point of drop in `empire::religion_value` from the current religion to
/// the asked one (on top of the anarchy).
pub const RELIGION_LOSS_POINTS: i64 = 3;
/// The turns of trade routes an embargo is priced at: the embargo's memory fades at 1 in 30 a
/// turn (`memory_decay_rand` of `MEMORY_STOPPED_TRADING_RECENT`), after which trade resumes.
pub const EMBARGO_TURNS: i64 = 30;
/// Map value of a land plot revealed to the receiver (before [`MAP_DIVISOR`]).
pub const MAP_LAND_POINTS: i64 = 5;
/// Map value of a water plot revealed to the receiver (before [`MAP_DIVISOR`]).
pub const MAP_WATER_POINTS: i64 = 1;
/// The plots' sum is divided by this to give the map's points.
pub const MAP_DIVISOR: i64 = 10;

/// `v` rounded down to a multiple of `diplomacy_value_remainder`, as an i32.
pub(crate) fn round(g: &Game, v: i64) -> i32 {
    let r = (g.d.defines.diplomacy_value_remainder as i64).max(1);
    let v = v - v.rem_euclid(r);
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Is `kind` priced by [`give_price`]?
pub fn is_give_kind(kind: TradeableItem) -> bool {
    matches!(
        kind,
        TradeableItem::TRADE_CITIES | TradeableItem::TRADE_PEACE | TradeableItem::TRADE_EMBARGO | TradeableItem::TRADE_CIVIC | TradeableItem::TRADE_RELIGION | TradeableItem::TRADE_PERMANENT_ALLIANCE
    )
}

/// What giving `item` to `to` costs `p` in trade points. `None`: not a kind priced here.
/// `Some(Err(reason))`: not given at any price (the reason is bare rules text, speaking of the
/// AI as "they/their" and of `to` as "you", like the reasons in `judge_offer`).
pub fn give_price(g: &Game, p: PlayerId, to: PlayerId, item: &TradeData) -> Option<Result<i32, &'static str>> {
    match item.item_type {
        TradeableItem::TRADE_CITIES => {
            let c = CityId(item.data as u32);
            if !g.cities.contains(c) || g.cities[c].owner != p {
                return Some(Err("not their city"));
            }
            Some(city_give_price(g, p, &city_ctx(g, p, to), c))
        }
        TradeableItem::TRADE_PEACE => Some(peace_parts(g, p, TeamId(item.data as u8)).price(g)),
        TradeableItem::TRADE_EMBARGO => Some(embargo_parts(g, p, TeamId(item.data as u8)).price(g)),
        TradeableItem::TRADE_CIVIC => Some(civic_parts(g, p, CivicType::from_usize(item.data as usize)).price(g)),
        TradeableItem::TRADE_RELIGION => Some(religion_parts(g, p, ReligionType::from_usize(item.data as usize)).price(g)),
        TradeableItem::TRADE_PERMANENT_ALLIANCE => Some(Err("permanent alliances are not signed")),
        _ => None,
    }
}

/// How the price of `item` (given by `p` to `to`) is made up, as bare rules text with its
/// numbers. `None` for an item not priced here or refused.
pub fn give_note(g: &Game, p: PlayerId, to: PlayerId, item: &TradeData) -> Option<String> {
    give_price(g, p, to, item)?.ok()?;
    match item.item_type {
        TradeableItem::TRADE_CITIES => Some(city_parts(g, p, CityId(item.data as u32)).note()),
        TradeableItem::TRADE_PEACE => peace_parts(g, p, TeamId(item.data as u8)).price(g).ok().map(|v| {
            if v == 0 {
                "They would make peace with that civilization now.".to_string()
            } else {
                format!("Tribute they would ask of that civilization for peace: {v}.")
            }
        }),
        TradeableItem::TRADE_EMBARGO => Some(embargo_parts(g, p, TeamId(item.data as u8)).note()),
        TradeableItem::TRADE_CIVIC => Some(civic_parts(g, p, CivicType::from_usize(item.data as usize)).note(g, true)),
        TradeableItem::TRADE_RELIGION => Some(religion_parts(g, p, ReligionType::from_usize(item.data as usize)).note(g, false)),
        _ => None,
    }
}

/// What `p`'s cities make each turn: production plus commerce (yield rates after modifiers).
pub fn output_per_turn(g: &Game, p: PlayerId) -> i32 {
    g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) + g.city_yield_rate(c, YieldType::YIELD_COMMERCE)).sum()
}

/// The production `p` needs for each of its units that the rules disband (`abandon`) once `gone`
/// says its requirement is gone: the units a civic or religion change costs.
fn deserters(g: &Game, p: PlayerId, gone: impl Fn(&UnitInfo) -> bool) -> i64 {
    let mut sum = 0i64;
    for &u in &g.players[p].units {
        if !g.units.contains(u) {
            continue;
        }
        let ut = g.units[u].unit_type;
        let info = &g.d.units[ut];
        if info.abandon && gone(info) {
            sum += g.player_get_production_needed_unit(p, ut).max(0) as i64;
        }
    }
    sum
}

/// "1 turn", "2 turns".
fn turns(n: i64) -> String {
    format!("{n} turn{}", if n == 1 { "" } else { "s" })
}

// ----------------------------------------------------------------------------
// Cities
// ----------------------------------------------------------------------------

/// What `p` reads once before pricing its cities for `to` (the victory plan is costly: it
/// estimates every civ's races).
pub struct CityCtx {
    /// `p`'s and `to`'s teams at war: a city is tribute, priced only.
    pub at_war: bool,
    /// `to`'s team is `p`'s victory rival.
    pub rival: bool,
    /// `to`'s team is the world's clear leader (not `p`'s team).
    pub leader: bool,
    /// `p`'s cities.
    pub num_cities: usize,
    /// `to`'s team holds a city `p` owned before, acquired within [`CITY_SALE_TURNS`].
    pub sold_recently: bool,
    /// At peace: `p`'s cities it sells, the lesser half by price (by price, then id; the first
    /// `num_cities / 2`).
    pub lesser: Vec<CityId>,
}

/// [`CityCtx`] for `p` giving to `to`. The plan, the leader and the ranks are computed only
/// outside a war.
pub fn city_ctx(g: &Game, p: PlayerId, to: PlayerId) -> CityCtx {
    let team = g.players[p].team;
    let to_team = g.players[to].team;
    let at_war = g.at_war(team, to_team);
    let num_cities = g.players[p].cities.len();
    let (mut rival, mut leader, mut sold_recently, mut lesser) = (false, false, false, Vec::new());
    if !at_war {
        rival = crate::victory::rival_team(g, &crate::victory::plan(g, p)) == Some(to_team);
        leader = crate::diplomacy::clear_leader(g).filter(|&t| t != team) == Some(to_team);
        let since = g.game_turn() - CITY_SALE_TURNS;
        sold_recently = g.player_ids().into_iter().filter(|&q| g.players[q].alive && g.players[q].team == to_team).any(|q| {
            g.players[q].cities.iter().any(|&c| g.cities.contains(c) && g.cities[c].previous_owner == Some(p) && g.cities[c].game_turn_acquired > since)
        });
        let mut ranked: Vec<(i32, CityId)> = g.players[p].cities.iter().filter(|&&c| g.cities.contains(c)).map(|&c| (city_price(g, p, c), c)).collect();
        ranked.sort();
        lesser = ranked.into_iter().take(num_cities / 2).map(|(_, c)| c).collect();
    }
    CityCtx { at_war, rival, leader, num_cities, sold_recently, lesser }
}

/// Why `p` does not give city `c` under `ctx`, if it does not.
pub fn city_refusal(ctx: &CityCtx, c: CityId) -> Option<&'static str> {
    if ctx.at_war {
        None
    } else if ctx.rival {
        Some("city withheld from their victory rival")
    } else if ctx.leader {
        Some("city withheld from the leading civilization")
    } else if ctx.num_cities <= MIN_CITIES_TO_SELL {
        Some("no city sold with 3 cities or fewer")
    } else if ctx.sold_recently {
        Some("one city in 30 turns")
    } else if !ctx.lesser.contains(&c) {
        Some("only their lesser cities are sold")
    } else {
        None
    }
}

/// What a city's price is made of.
#[derive(Clone, Copy, Debug, Default)]
pub struct CityParts {
    pub pop: i32,
    pub production: i32,
    pub commerce: i32,
    pub food: i32,
    /// The production cost to the owner of the world wonders the city holds.
    pub wonders: i64,
    /// The production cost to the owner of its civilization's settler.
    pub settler: i64,
}

impl CityParts {
    /// What the city makes a turn, as priced: production, commerce, the food surplus.
    pub fn per_turn(&self) -> i64 {
        (self.production + self.commerce + self.food.max(0)) as i64
    }

    pub fn price(&self, g: &Game) -> i32 {
        round(g, self.settler + CITY_TURNS * self.per_turn() + self.wonders)
    }

    /// `Settler 220 + 100 turns x 41 a turn (production 14, commerce 25, food surplus 2) + world wonders 297.`
    pub fn note(&self) -> String {
        let mut s = format!("Settler {}", self.settler);
        if self.per_turn() > 0 {
            let parts: Vec<String> = [("production", self.production), ("commerce", self.commerce), ("food surplus", self.food.max(0))]
                .iter()
                .filter(|(_, v)| *v != 0)
                .map(|(k, v)| format!("{k} {v}"))
                .collect();
            s += &format!(" + {CITY_TURNS} turns x {} a turn ({})", self.per_turn(), parts.join(", "));
        }
        if self.wonders > 0 {
            s += &format!(" + world wonders {}", self.wonders);
        }
        s + "."
    }
}

pub fn city_parts(g: &Game, p: PlayerId, c: CityId) -> CityParts {
    let d = g.d;
    let settler = d
        .ty::<UnitClassType>("UNITCLASS_SETTLER")
        .zip(g.player_get_civilization_type(p))
        .and_then(|(class, civ)| d.civ_unit(civ, class))
        .map_or(0, |u| g.player_get_production_needed_unit(p, u).max(0) as i64);
    let mut wonders = 0i64;
    for b in d.buildings.keys() {
        if g.cities[c].num_real_building[b] > 0 && crate::production::is_world_wonder(d, b) {
            wonders += g.player_get_production_needed_building(p, b).max(0) as i64;
        }
    }
    CityParts {
        pop: g.cities[c].population,
        production: g.city_yield_rate(c, YieldType::YIELD_PRODUCTION),
        commerce: g.city_yield_rate(c, YieldType::YIELD_COMMERCE),
        food: g.city_food_difference(c, true),
        wonders,
        settler,
    }
}

/// The price of city `c` to `p` (no refusal).
pub fn city_price(g: &Game, p: PlayerId, c: CityId) -> i32 {
    city_parts(g, p, c).price(g)
}

/// City `c` given by `p` under `ctx`: its price, or why it is not given.
pub fn city_give_price(g: &Game, p: PlayerId, ctx: &CityCtx, c: CityId) -> Result<i32, &'static str> {
    match city_refusal(ctx, c) {
        Some(why) => Err(why),
        None => Ok(city_price(g, p, c)),
    }
}

// ----------------------------------------------------------------------------
// Maps
// ----------------------------------------------------------------------------

/// The plots `from_team` has seen and `to_team` has not: (land, water).
pub fn map_plots(g: &Game, from_team: TeamId, to_team: TeamId) -> (i64, i64) {
    let (mut land, mut water) = (0, 0);
    for pl in g.map.plot_ids() {
        let plot = &g.map.plots[pl];
        if plot.revealed[from_team] && !plot.revealed[to_team] {
            if g.plot_is_water(pl) {
                water += 1;
            } else {
                land += 1;
            }
        }
    }
    (land, water)
}

/// What `from_team`'s map is worth to `to_team`: [`MAP_LAND_POINTS`] a land plot it has not
/// seen, [`MAP_WATER_POINTS`] a water plot, over [`MAP_DIVISOR`], rounded.
pub fn map_value(g: &Game, from_team: TeamId, to_team: TeamId) -> i32 {
    let (land, water) = map_plots(g, from_team, to_team);
    round(g, (land * MAP_LAND_POINTS + water * MAP_WATER_POINTS) / MAP_DIVISOR)
}

/// `Plots the receiving side has not seen: 412 land x 5 / 10 + 230 water x 1 / 10.`
pub fn map_note(g: &Game, from_team: TeamId, to_team: TeamId) -> String {
    let (land, water) = map_plots(g, from_team, to_team);
    format!("Plots the receiving side has not seen: {land} land x {MAP_LAND_POINTS} / {MAP_DIVISOR} + {water} water x {MAP_WATER_POINTS} / {MAP_DIVISOR}.")
}

// ----------------------------------------------------------------------------
// Peace with a third team
// ----------------------------------------------------------------------------

/// The two verdicts on a bare peace between `p` and team X.
#[derive(Clone, Debug)]
pub struct PeaceParts {
    /// `p`'s verdict on the bare peace with X's leader (`judge_offer`, nothing recorded).
    pub ours: OfferVerdict,
    /// X's leader's verdict (its controller); `None` when X has no alive leader.
    pub theirs: Option<OfferVerdict>,
}

pub fn peace_parts(g: &Game, p: PlayerId, x_team: TeamId) -> PeaceParts {
    let pt = TradeData { item_type: TradeableItem::TRADE_PEACE_TREATY, data: 0, offering: false, hidden: false };
    let Some(x) = crate::diplomacy::team_leader(g, x_team) else {
        return PeaceParts { ours: OfferVerdict { accept: false, margin: None, why: Some("no leader".into()) }, theirs: None };
    };
    let ours = crate::diplomacy::judge_offer(g, p, x, std::slice::from_ref(&pt), std::slice::from_ref(&pt), false);
    let theirs = g.controller(x).judge_offer(g, x, p, std::slice::from_ref(&pt), std::slice::from_ref(&pt));
    PeaceParts { ours, theirs: Some(theirs) }
}

impl PeaceParts {
    /// 0 when `p` takes the bare peace; what it lacks on value when value refused it; a refusal
    /// when a rule did (or a refusal with a margin of 0 or more, which no value can mend). Then X
    /// must take it too.
    pub fn price(&self, g: &Game) -> Result<i32, &'static str> {
        let price = if self.ours.accept {
            0
        } else {
            match self.ours.margin {
                Some(m) if m < 0 => round(g, -(m as i64)),
                _ => return Err("they will not end that war"),
            }
        };
        match &self.theirs {
            Some(v) if v.accept => Ok(price),
            _ => Err("the other side refuses peace"),
        }
    }
}

// ----------------------------------------------------------------------------
// Embargo
// ----------------------------------------------------------------------------

/// What an embargo on team X costs `p`.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmbargoParts {
    /// A defensive pact with X.
    pub pact: bool,
    /// `p`'s attitude towards X's leader is Friendly.
    pub friendly: bool,
    /// The deals the embargo kills (`player_stop_trading_with_team`'s test).
    pub deals: i32,
    /// Their net value to `p`: over each deal, max(0, what `p` loses - what it gets back).
    pub net: i64,
    /// An open-borders deal is among them: the trade routes between the two close.
    pub open_borders: bool,
    /// What the routes that close pay `p` a turn, in hundredths.
    pub routes_x100: i64,
}

pub fn embargo_parts(g: &Game, p: PlayerId, x_team: TeamId) -> EmbargoParts {
    let team = g.players[p].team;
    let pact = g.teams[team].defensive_pact[x_team];
    let friendly = crate::diplomacy::team_leader(g, x_team).is_some_and(|x| crate::diplomacy::attitude(g, p, x) == AttitudeType::ATTITUDE_FRIENDLY);
    let (mut deals, mut net, mut open_borders) = (0, 0i64, false);
    for d in g.deals.ids() {
        if !g.deal_is_cancelable(d, Some(p)) || g.deal_is_peace_deal(d) {
            continue;
        }
        let deal = &g.deals[d];
        let (o, p_gives, o_gives) = if deal.first_player == p {
            (deal.second_player, &deal.first_trades, &deal.second_trades)
        } else if deal.second_player == p {
            (deal.first_player, &deal.second_trades, &deal.first_trades)
        } else {
            continue;
        };
        if g.players[o].team != x_team {
            continue;
        }
        deals += 1;
        open_borders |= p_gives.iter().chain(o_gives.iter()).any(|t| t.item_type == TradeableItem::TRADE_OPEN_BORDERS);
        // A resource it imports is valued as the copy it loses (change -1).
        let loses = crate::trade::deal_val(g, p, o, o_gives, false, -1) as i64;
        let gets_back = crate::trade::deal_val(g, p, o, p_gives, false, 1) as i64;
        net += (loses - gets_back).max(0);
    }
    let mut routes_x100 = 0i64;
    if open_borders {
        let dest_team = |c: CityId| g.cities.contains(c).then(|| g.players[g.cities[c].owner].team);
        for r in g.player_routes(p) {
            if g.routes[r].is_open() && dest_team(g.routes[r].destination) == Some(x_team) {
                routes_x100 += crate::caravans::route_pays(g, r) as i64;
            }
        }
        for q in g.player_ids() {
            if g.players[q].team != x_team {
                continue;
            }
            for r in g.player_routes(q) {
                let route = &g.routes[r];
                if route.is_open() && g.cities.contains(route.destination) && g.cities[route.destination].owner == p {
                    routes_x100 += route.dest_pay_x100 as i64;
                }
            }
        }
    }
    EmbargoParts { pact, friendly, deals, net, open_borders, routes_x100 }
}

impl EmbargoParts {
    /// What the routes cost over [`EMBARGO_TURNS`] (points).
    pub fn routes(&self) -> i64 {
        self.routes_x100 * EMBARGO_TURNS / 100
    }

    pub fn price(&self, g: &Game) -> Result<i32, &'static str> {
        if self.pact {
            return Err("defensive pact with the target");
        }
        if self.friendly {
            return Err("attitude towards the target Friendly");
        }
        Ok(round(g, self.net + self.routes()))
    }

    /// `Deals cancelled: net value 60. Trade routes closed: 4 a turn x 30 turns.`
    pub fn note(&self) -> String {
        let mut s = String::new();
        if self.net > 0 || self.routes() == 0 {
            s += &format!("Deals cancelled: net value {}.", self.net);
        }
        if self.routes() > 0 {
            if !s.is_empty() {
                s.push(' ');
            }
            s += &format!("Trade routes closed: {} a turn x {EMBARGO_TURNS} turns.", self.routes_x100 / 100);
        }
        s
    }
}

// ----------------------------------------------------------------------------
// Civics and religion
// ----------------------------------------------------------------------------

/// What adopting a civic, or converting to a religion, costs `p`.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChangeParts {
    /// The value (`empire::civic_value` / `religion_value`) of the current civic or religion
    /// (0 when none).
    pub value_cur: i32,
    /// The value of the asked one.
    pub value_new: i32,
    /// Turns of anarchy of the change.
    pub anarchy: i32,
    /// The production cost of the units the change disbands.
    pub deserters: i64,
    /// The points of the drop in value.
    pub loss: i64,
    /// `output_per_turn` of `p`.
    pub output: i32,
    /// A civic refused: in financial trouble, and the asked civic costs more than the current.
    pub costlier_in_trouble: bool,
}

impl ChangeParts {
    /// Turns lost: the anarchy, twice when the change is a loss (`p` goes back afterwards).
    pub fn turns(&self) -> i64 {
        self.anarchy as i64 * if self.loss > 0 { 2 } else { 1 }
    }

    pub fn total(&self, g: &Game) -> i32 {
        round(g, self.turns() * self.output as i64 + self.loss + self.deserters)
    }

    pub fn price(&self, g: &Game) -> Result<i32, &'static str> {
        if self.costlier_in_trouble {
            return Err("in financial trouble: no costlier civic");
        }
        Ok(self.total(g))
    }

    /// `Anarchy 1 turn x output 416 a turn, there and back. Loss over 10 turns: 645. Units that
    /// leave: 120.` (`civic`: the loss is over `peace_treaty_length` turns; a religion's is not.)
    pub fn note(&self, g: &Game, civic: bool) -> String {
        let mut parts = Vec::new();
        if self.anarchy > 0 || (self.loss == 0 && self.deserters == 0) {
            let back = if self.loss > 0 { ", there and back" } else { "" };
            parts.push(format!("Anarchy {} x output {} a turn{back}.", turns(self.anarchy as i64), self.output));
        }
        if self.loss > 0 {
            parts.push(if civic { format!("Loss over {}: {}.", turns(g.d.defines.peace_treaty_length as i64), self.loss) } else { format!("Loss: {}.", self.loss) });
        }
        if self.deserters > 0 {
            parts.push(format!("Units that leave: {}.", self.deserters));
        }
        parts.join(" ")
    }
}

/// `p` adopting `civic` (in its option; a civic without an option changes nothing but costs
/// its anarchy).
pub fn civic_parts(g: &Game, p: PlayerId, civic: CivicType) -> ChangeParts {
    let d = g.d;
    let output = output_per_turn(g, p);
    let value_new = crate::empire::civic_value(g, p, civic);
    let Some(option) = d.civics[civic].civic_option_type else {
        return ChangeParts { value_new, output, ..Default::default() };
    };
    let cur = g.players[p].civics[option];
    let mut new = g.players[p].civics.clone();
    new[option] = Some(civic);
    let value_cur = cur.map_or(0, |c| crate::empire::civic_value(g, p, c));
    // The spread of the option's values (the civics `p` can run there, the current and the
    // asked one included): the loss is the drop's share of it.
    let (mut lo, mut hi) = (value_new.min(value_cur), value_new.max(value_cur));
    for c in d.civics.keys() {
        if d.civics[c].civic_option_type == Some(option) && g.player_can_do_civics(p, Some(c)) {
            let v = crate::empire::civic_value(g, p, c);
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    let gap = (hi - lo) as i64;
    let drop = cur.map_or(0, |_| (value_cur - value_new).max(0) as i64);
    let loss = if gap > 0 { drop * CIVIC_SWING_PCT * output as i64 * d.defines.peace_treaty_length as i64 / (100 * gap) } else { 0 };
    let costlier_in_trouble = crate::production::financial_trouble(g, p)
        && cur.is_some_and(|c| crate::finance::civic_cost_x100(g, p, civic) > crate::finance::civic_cost_x100(g, p, c));
    ChangeParts {
        value_cur,
        value_new,
        anarchy: g.player_get_civic_anarchy_length(p, &new),
        deserters: match cur {
            Some(c) if c != civic => deserters(g, p, |u| u.prereq_civic == Some(c)),
            _ => 0,
        },
        loss,
        output,
        costlier_in_trouble,
    }
}

/// `p` converting to `religion`.
pub fn religion_parts(g: &Game, p: PlayerId, religion: ReligionType) -> ChangeParts {
    let cur = g.player_get_state_religion(p);
    let value_cur = cur.map_or(0, |r| crate::empire::religion_value(g, p, r));
    let value_new = crate::empire::religion_value(g, p, religion);
    ChangeParts {
        value_cur,
        value_new,
        anarchy: g.player_get_religion_anarchy_length(p),
        deserters: match cur {
            Some(r) if r != religion => deserters(g, p, |u| u.state_religion == Some(r)),
            _ => 0,
        },
        loss: cur.map_or(0, |_| (value_cur - value_new).max(0) as i64 * RELIGION_LOSS_POINTS),
        output: output_per_turn(g, p),
        costlier_in_trouble: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diplomacy::judge_offer;
    use crate::testkit::{one_landmass_game, one_landmass_game_of};

    fn td(kind: TradeableItem, data: i32) -> TradeData {
        TradeData { item_type: kind, data, offering: false, hidden: false }
    }

    fn gold(n: i32) -> Vec<TradeData> {
        vec![td(TradeableItem::TRADE_GOLD, n)]
    }

    /// Every pair of `g`'s civ players met, at peace, trading gold and open borders, with gold
    /// and the same alignment (attitude Cautious or better).
    fn open_relations(g: &mut Game) {
        let ps: Vec<PlayerId> = g.civ_player_ids().collect();
        for &a in &ps {
            for &b in &ps {
                if a != b {
                    let (ta, tb) = (g.players[a].team, g.players[b].team);
                    g.team_meet(ta, tb, false);
                }
            }
            let t = g.players[a].team;
            g.teams[t].gold_trading_count += 1;
            g.teams[t].open_borders_trading_count += 1;
            g.teams[t].map_trading_count += 1;
            g.players[a].gold = 100_000;
            g.players[a].alignment = g.players[ps[0]].alignment;
        }
    }

    /// Us (player 0) with cities at `ours`, them (player 1) with one city at (15, 5).
    fn game(seed: u32, ours: &[(i32, i32)]) -> (Game, PlayerId, PlayerId) {
        let mut g = one_landmass_game(seed);
        let (us, them) = (PlayerId(0), PlayerId(1));
        for &(x, y) in ours {
            g.player_init_city(us, x, y, true, true);
        }
        g.player_init_city(them, 15, 5, true, true);
        open_relations(&mut g);
        (g, us, them)
    }

    const FOUR: [(i32, i32); 4] = [(3, 4), (3, 7), (6, 5), (9, 7)];

    fn city_item(c: CityId) -> TradeData {
        td(TradeableItem::TRADE_CITIES, c.0 as i32)
    }

    /// A city costs its civilization's settler, 100 turns of its production, commerce and food
    /// surplus, and the production cost of its world wonders.
    #[test]
    fn a_city_costs_a_settler_100_turns_of_its_yields_and_its_wonders() {
        let (mut g, us, _) = game(51, &[(3, 4)]);
        let d = g.d;
        let c = g.players[us].cities[0];
        let civ = g.player_get_civilization_type(us).unwrap();
        let settler = g.player_get_production_needed_unit(us, d.civ_unit(civ, d.t("UNITCLASS_SETTLER")).unwrap()) as i64;
        let per_turn = |g: &Game| (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION) + g.city_yield_rate(c, YieldType::YIELD_COMMERCE) + g.city_food_difference(c, true).max(0)) as i64;
        assert!(settler > 0 && per_turn(&g) > 0);
        assert_eq!(city_price(&g, us, c), round(&g, settler + 100 * per_turn(&g)));
        let wonder = d.buildings.keys().find(|&b| crate::production::is_world_wonder(d, b) && d.buildings[b].production_cost > 0).unwrap();
        g.city_set_num_real_building(c, wonder, 1);
        let cost = g.player_get_production_needed_building(us, wonder) as i64;
        assert_eq!(city_parts(&g, us, c).wonders, cost);
        assert_eq!(city_price(&g, us, c), round(&g, settler + 100 * per_turn(&g) + cost));
        let note = city_parts(&g, us, c).note();
        assert!(note.starts_with(&format!("Settler {settler} + 100 turns x {} a turn (", per_turn(&g))) && note.ends_with(&format!(" + world wonders {cost}.")), "{note}");
    }

    /// At peace a civilization with 3 cities or fewer sells none, then only the lesser half by
    /// price, and one city in 30 turns to the same team (a city it lost to that team counts); at
    /// war every city is priced.
    #[test]
    fn cities_are_sold_from_four_the_lesser_half_one_in_30_turns() {
        let (mut g, us, them) = game(53, &FOUR[..3]);
        for &c in &g.players[us].cities {
            assert_eq!(give_price(&g, us, them, &city_item(c)), Some(Err("no city sold with 3 cities or fewer")));
        }
        g.player_init_city(us, FOUR[3].0, FOUR[3].1, true, true);
        let mut ranked: Vec<(i32, CityId)> = g.players[us].cities.iter().map(|&c| (city_price(&g, us, c), c)).collect();
        ranked.sort();
        for (i, &(price, c)) in ranked.iter().enumerate() {
            let want = if i < 2 { Ok(price) } else { Err("only their lesser cities are sold") };
            assert_eq!(give_price(&g, us, them, &city_item(c)), Some(want), "rank {i}");
        }
        // A city of theirs that was ours, taken this turn.
        let theirs = g.players[them].cities[0];
        g.cities[theirs].previous_owner = Some(us);
        g.cities[theirs].game_turn_acquired = g.game_turn();
        assert_eq!(give_price(&g, us, them, &city_item(ranked[0].1)), Some(Err("one city in 30 turns")));
        g.cities[theirs].game_turn_acquired = g.game_turn() - CITY_SALE_TURNS;
        assert_eq!(give_price(&g, us, them, &city_item(ranked[0].1)), Some(Ok(ranked[0].0)));
        // At war: tribute, every city priced.
        let (t0, t1) = (g.players[us].team, g.players[them].team);
        g.team_declare_war(t1, t0, true, Some(WarPlanType::WARPLAN_TOTAL));
        for &(price, c) in &ranked {
            assert_eq!(give_price(&g, us, them, &city_item(c)), Some(Ok(price)));
        }
    }

    /// The civic loss is 0 for a civic valued at least as high as the current one, and never
    /// above `CIVIC_SWING_PCT` of `peace_treaty_length` turns of output; in financial trouble a
    /// costlier civic is refused.
    #[test]
    fn a_civic_loss_is_a_share_of_ten_turns_of_output() {
        let (mut g, us, them) = game(57, &FOUR[..2]);
        let d = g.d;
        let team = g.players[us].team;
        for t in d.techs.keys() {
            g.team_set_has_tech(team, t, true, Some(us), false, false);
        }
        let output = output_per_turn(&g, us) as i64;
        let most = CIVIC_SWING_PCT * output * d.defines.peace_treaty_length as i64 / 100;
        assert!(output > 0);
        let mut lost = 0;
        for c in d.civics.keys().filter(|&c| d.civics[c].civic_option_type.is_some() && g.player_can_do_civics(us, Some(c))) {
            let x = civic_parts(&g, us, c);
            if x.value_new >= x.value_cur {
                assert_eq!(x.loss, 0, "{c:?}");
            } else {
                lost += 1;
            }
            assert!(x.loss <= most, "{c:?}: {} > {most}", x.loss);
            assert_eq!(give_price(&g, us, them, &td(TradeableItem::TRADE_CIVIC, c.i())), Some(Ok(x.total(&g))));
        }
        assert!(lost > 0, "some civic is a loss");
        // Financial trouble: research pushed under 40%.
        let cost = |c: CivicType| crate::finance::civic_cost_x100(&g, us, c);
        let costlier = d.civics.keys().find(|&c| {
            d.civics[c].civic_option_type.is_some_and(|o| g.players[us].civics[o].is_some_and(|cur| cost(c) > cost(cur))) && g.player_can_do_civics(us, Some(c))
        });
        let costlier = costlier.expect("a civic costlier than the current one");
        g.players[us].commerce_percent[CommerceType::COMMERCE_RESEARCH.idx()] = 20;
        g.players[us].commerce_percent[CommerceType::COMMERCE_CULTURE.idx()] = 0;
        assert!(crate::production::financial_trouble(&g, us));
        assert_eq!(give_price(&g, us, them, &td(TradeableItem::TRADE_CIVIC, costlier.i())), Some(Err("in financial trouble: no costlier civic")));
    }

    /// Two religions in our cities, the first ours: the second costs its anarchy (twice when it
    /// is a loss) and 3 points per point of value lost.
    fn two_religions(g: &mut Game, us: PlayerId) -> (ReligionType, ReligionType) {
        let d = g.d;
        let cities = g.players[us].cities.clone();
        let ok: Vec<ReligionType> = d
            .religions
            .keys()
            .filter(|&r| g.players[us].personality_type.is_some_and(|l| d.leader_heads[l].religion_weight_modifier[r] >= 0))
            .collect();
        let (r1, r2) = (ok[0], ok[1]);
        for &c in &cities {
            g.city_set_has_religion(c, r1, true, false, false);
        }
        g.city_set_has_religion(cities[0], r2, true, false, false);
        g.player_convert(us, Some(r1));
        end_anarchy(g, us);
        assert_eq!(g.player_get_state_religion(us), Some(r1));
        (r1, r2)
    }

    fn end_anarchy(g: &mut Game, p: PlayerId) {
        let n = g.players[p].anarchy_turns;
        g.player_change_anarchy_turns(p, -n);
    }

    #[test]
    fn a_religion_costs_its_anarchy_and_the_value_lost() {
        let (mut g, us, them) = game(59, &FOUR[..2]);
        let (r1, r2) = two_religions(&mut g, us);
        let x = religion_parts(&g, us, r2);
        assert_eq!(x.value_cur, crate::empire::religion_value(&g, us, r1));
        assert_eq!(x.loss, (x.value_cur - x.value_new).max(0) as i64 * RELIGION_LOSS_POINTS);
        assert!(x.loss > 0, "{x:?}");
        let want = round(&g, x.anarchy as i64 * 2 * x.output as i64 + x.loss);
        assert_eq!(give_price(&g, us, them, &td(TradeableItem::TRADE_RELIGION, r2.i())), Some(Ok(want)));
    }

    /// Three civs: us (0), the asker (1), the target (2). Cities, all met.
    fn three() -> (Game, PlayerId, PlayerId, PlayerId) {
        let mut g = one_landmass_game_of(61, 3);
        let (us, to, x) = (PlayerId(0), PlayerId(1), PlayerId(2));
        for &(cx, cy) in &FOUR[..2] {
            g.player_init_city(us, cx, cy, true, true);
        }
        g.player_init_city(to, 12, 4, true, true);
        g.player_init_city(x, 15, 7, true, true);
        open_relations(&mut g);
        (g, us, to, x)
    }

    /// A bonus with happiness or health, revealed and tradeable from the start.
    fn plain_bonus(g: &Game) -> BonusType {
        let d = g.d;
        d.bonuses
            .keys()
            .find(|&b| {
                let i = &d.bonuses[b];
                i.tech_reveal.is_none() && i.tech_city_trade.is_none() && (i.happiness > 0 || i.health > 0)
            })
            .expect("a plain bonus")
    }

    /// An embargo kills the cancelable deals with the target: priced at what they were worth
    /// (a resource it imports at the copy it loses), refused under a defensive pact. Carried
    /// out, the deals are gone and neither side makes a new deal while the memory lasts.
    #[test]
    fn an_embargo_is_priced_at_the_deals_it_cancels_and_holds_off_new_deals() {
        let (mut g, us, to, x) = three();
        let (tu, tx) = (g.players[us].team, g.players[x].team);
        let ob = vec![td(TradeableItem::TRADE_OPEN_BORDERS, 0)];
        g.implement_deal(us, x, &ob, &ob, false);
        let b = plain_bonus(&g);
        let their_city = g.players[x].cities[0];
        g.plot_set_bonus_type(g.city_plot(their_city), Some(b));
        g.update_plot_groups();
        assert!(g.player_get_num_available_bonuses(x, b) > 0, "their city has the resource");
        // Forced: a resource trade needs the trade networks connected (roads), not built here.
        g.implement_deal(x, us, &[td(TradeableItem::TRADE_RESOURCES, b.i())], &[], true);
        assert!(g.team_is_open_borders(tu, tx));
        let embargo = td(TradeableItem::TRADE_EMBARGO, tx.0 as i32);
        // Not cancelable yet: nothing to price.
        assert_eq!(embargo_parts(&g, us, tx).deals, 0);
        g.setup.game_turn += g.d.defines.peace_treaty_length;
        let parts = embargo_parts(&g, us, tx);
        assert_eq!((parts.deals, parts.open_borders), (2, true), "{parts:?}");
        assert!(parts.net > 0, "the resource lost: {parts:?}");
        assert_eq!(give_price(&g, us, to, &embargo), Some(Ok(round(&g, parts.net + parts.routes()))));
        assert!(give_note(&g, us, to, &embargo).is_some_and(|n| n.starts_with(&format!("Deals cancelled: net value {}.", parts.net))));
        // A defensive pact with the target.
        g.team_set_defensive_pact(tu, tx, true);
        assert_eq!(give_price(&g, us, to, &embargo), Some(Err("defensive pact with the target")));
        g.team_set_defensive_pact(tu, tx, false);
        // Carried out.
        g.implement_deal(to, us, &[], std::slice::from_ref(&embargo), false);
        assert!(!g.team_is_open_borders(tu, tx));
        assert!(!g.deals.ids().into_iter().any(|d| {
            let (a, b) = (g.deals[d].first_player, g.deals[d].second_player);
            (a == us && b == x) || (a == x && b == us)
        }));
        let v = judge_offer(&g, x, us, &gold(100), &[td(TradeableItem::TRADE_MAPS, 0)], false);
        assert_eq!(v.why.as_deref(), Some("you stopped trading with them"), "{v:?}");
        let v = judge_offer(&g, us, x, &gold(100), &[td(TradeableItem::TRADE_MAPS, 0)], false);
        assert_eq!(v.why.as_deref(), Some("they stopped trading with you"), "{v:?}");
        // Gifts still pass.
        assert!(judge_offer(&g, x, us, &gold(100), &[], false).accept);
    }

    /// Peace with a third party: refused in a war under 10 turns old; otherwise what the AI's
    /// own judgement of the bare peace says, the target's too. Carried out, the war ends.
    #[test]
    fn peace_with_a_third_party_follows_both_judgements() {
        let (mut g, us, to, x) = three();
        let (tu, tx) = (g.players[us].team, g.players[x].team);
        g.team_declare_war(tu, tx, true, Some(WarPlanType::WARPLAN_LIMITED));
        let peace = td(TradeableItem::TRADE_PEACE, tx.0 as i32);
        assert_eq!(give_price(&g, us, to, &peace), Some(Err("they will not end that war")));
        g.teams[tu].ai_at_war_counter[tx] = 30;
        g.teams[tx].ai_at_war_counter[tu] = 30;
        // Even forces, no war success: both take the bare peace.
        let parts = peace_parts(&g, us, tx);
        assert!(parts.ours.accept && parts.theirs.as_ref().is_some_and(|v| v.accept), "{parts:?}");
        assert_eq!(give_price(&g, us, to, &peace), Some(Ok(0)));
        assert_eq!(give_note(&g, us, to, &peace).as_deref(), Some("They would make peace with that civilization now."));
        // Winning (more than 1.5 times their power, short of the conquest ratio, and war
        // success): the bare peace is refused on value; the price is what it lacks.
        g.players[us].power = 170;
        g.players[x].power = 100;
        g.teams[tu].ai_war_success[tx] = 50;
        let won = peace_parts(&g, us, tx);
        let m = won.ours.margin.unwrap_or(0);
        assert!(!won.ours.accept && m < 0 && won.theirs.as_ref().is_some_and(|v| v.accept), "{won:?}");
        assert_eq!(give_price(&g, us, to, &peace), Some(Ok(round(&g, -(m as i64)))));
        assert!(give_note(&g, us, to, &peace).is_some_and(|n| n.starts_with("Tribute they would ask of that civilization for peace: ")));
        // The target refuses (it is winning that war the same way): refused.
        g.players[us].power = 100;
        g.players[x].power = 170;
        g.teams[tu].ai_war_success[tx] = 0;
        g.teams[tx].ai_war_success[tu] = 50;
        let lost = peace_parts(&g, us, tx);
        assert!(lost.theirs.as_ref().is_some_and(|v| !v.accept), "{lost:?}");
        assert_eq!(give_price(&g, us, to, &peace), Some(Err("the other side refuses peace")));
        g.players[x].power = 100;
        g.teams[tx].ai_war_success[tu] = 0;
        // A demand for it costs nothing: granted, and nothing is recorded.
        let v = judge_offer(&g, us, to, &[], std::slice::from_ref(&peace), false);
        assert_eq!((v.accept, v.why.as_deref()), (true, Some("no cost to them")), "{v:?}");
        g.implement_proposal(to, us, &[], std::slice::from_ref(&peace));
        assert!(!g.at_war(tu, tx));
        assert!(g.players[us].tributes.is_empty() && g.demand_pause(us, to) == 0);
    }

    /// A map is worth 5 a land plot and 1 a water plot the receiver has not seen, over 10;
    /// carried out, the plots are revealed.
    #[test]
    fn a_map_is_worth_the_plots_not_seen() {
        let (mut g, us, them) = game(63, &FOUR[..1]);
        let tu = g.players[us].team;
        let plots: Vec<PlotIdx> = g.map.plot_ids().filter(|&q| g.map.xy(q).0 >= 12).collect();
        for &q in &plots {
            g.map.plots[q].revealed[tu] = false;
        }
        let land = plots.iter().filter(|&&q| !g.plot_is_water(q)).count() as i64;
        let water = plots.len() as i64 - land;
        let maps = td(TradeableItem::TRADE_MAPS, 0);
        let want = round(&g, (land * 5 + water) / 10);
        assert!(want > 0);
        assert_eq!(crate::trade::deal_val(&g, us, them, std::slice::from_ref(&maps), false, 1), want);
        assert_eq!(map_note(&g, g.players[them].team, tu), format!("Plots the receiving side has not seen: {land} land x 5 / 10 + {water} water x 1 / 10."));
        g.implement_deal(them, us, std::slice::from_ref(&maps), &[], false);
        assert!(plots.iter().all(|&q| g.map.plots[q].revealed[tu]));
    }

    /// A city is sold for enough gold and not for less, not on demand; carried out, it changes
    /// hands. A permanent alliance is refused.
    #[test]
    fn a_city_is_sold_for_its_price() {
        let (mut g, us, them) = game(65, &FOUR);
        let ctx = city_ctx(&g, us, them);
        let c = ctx.lesser[0];
        let price = city_price(&g, us, c) as i64;
        let pct_us = crate::trade::gold_trade_value_percent(&g, us) as i64;
        let enough = ((price * 100 + pct_us - 1) / pct_us) as i32;
        let v = judge_offer(&g, us, them, &gold(enough), &[city_item(c)], false);
        assert!(v.accept, "{v:?}");
        let v = judge_offer(&g, us, them, &gold(enough - 20), &[city_item(c)], false);
        assert!(!v.accept && v.margin.is_some_and(|m| m < 0), "{v:?}");
        let v = judge_offer(&g, us, them, &[], &[city_item(c)], false);
        assert_eq!(v.why.as_deref(), Some("cities are not given on demand"));
        // Two cities in one deal.
        let v = judge_offer(&g, us, them, &gold(enough * 10), &[city_item(c), city_item(ctx.lesser[1])], false);
        assert_eq!(v.why.as_deref(), Some("one city a deal"), "{v:?}");
        // A lease in the payment: refused, whatever it is worth.
        let mut pay = gold(enough);
        pay.push(td(TradeableItem::TRADE_GOLD_PER_TURN, 50));
        let v = judge_offer(&g, us, them, &pay, &[city_item(c)], false);
        assert_eq!(v.why.as_deref(), Some("no lease in payment for a city, civic, religion, embargo or peace"), "{v:?}");
        let v = judge_offer(&g, us, them, &gold(enough), &[td(TradeableItem::TRADE_PERMANENT_ALLIANCE, 0)], false);
        assert_eq!(v.why.as_deref(), Some("permanent alliances are not signed"));
        let at = g.city_plot(c);
        g.implement_deal(them, us, &gold(enough), &[city_item(c)], false);
        assert!(g.players[them].cities.iter().any(|&k| g.city_plot(k) == at), "the city is theirs");
    }

    /// At war, a city in a peace deal: ceded by a civilization losing the war, not by one
    /// winning it.
    #[test]
    fn a_losing_civilization_cedes_a_city_for_peace() {
        let (mut g, us, them) = game(67, &FOUR);
        let (tu, tt) = (g.players[us].team, g.players[them].team);
        g.team_declare_war(tt, tu, true, Some(WarPlanType::WARPLAN_TOTAL));
        g.teams[tu].ai_at_war_counter[tt] = 30;
        g.teams[tt].ai_at_war_counter[tu] = 30;
        let c = g.players[us].cities[3];
        let peace = td(TradeableItem::TRADE_PEACE_TREATY, 0);
        let ours = vec![peace.clone(), city_item(c)];
        // Losing: they hit us hard and are much stronger.
        g.players[us].power = 20;
        g.players[them].power = 400;
        g.teams[tt].ai_war_success[tu] = 100;
        let v = judge_offer(&g, us, them, std::slice::from_ref(&peace), &ours, false);
        assert!(v.accept, "{v:?}");
        // Winning: no city for peace.
        g.players[us].power = 400;
        g.players[them].power = 20;
        g.teams[tt].ai_war_success[tu] = 0;
        g.teams[tu].ai_war_success[tt] = 100;
        let v = judge_offer(&g, us, them, std::slice::from_ref(&peace), &ours, false);
        assert!(!v.accept, "{v:?}");
        // Nor at the rule's own peace: winning a war of conquest 80 turns old, the bare peace is
        // taken by rule, a city with it is still weighed.
        g.teams[tu].ai_at_war_counter[tt] = 80;
        g.teams[tt].ai_at_war_counter[tu] = 80;
        assert!(crate::diplomacy::press_conquest(&g, tu, tt));
        let v = judge_offer(&g, us, them, std::slice::from_ref(&peace), std::slice::from_ref(&peace), false);
        assert_eq!((v.accept, v.why.as_deref()), (true, Some("war longer than 80 turns")), "{v:?}");
        let v = judge_offer(&g, us, them, std::slice::from_ref(&peace), &ours, false);
        assert_eq!((v.accept, v.why.as_deref()), (false, Some("a city is ceded only for what peace itself is worth to them")), "{v:?}");
        // An even war: their tribute does not buy a city in the treaty, however large; two cities
        // are one too many.
        g.players[us].power = 100;
        g.players[them].power = 100;
        g.teams[tu].ai_war_success[tt] = 0;
        g.teams[tu].ai_at_war_counter[tt] = 30;
        g.teams[tt].ai_at_war_counter[tu] = 30;
        let mut rich = gold(90_000);
        rich.push(peace.clone());
        let v = judge_offer(&g, us, them, &rich, &ours, false);
        assert_eq!((v.accept, v.why.as_deref()), (false, Some("a city is ceded only for what peace itself is worth to them")), "{v:?}");
        let two = vec![peace.clone(), city_item(c), city_item(g.players[us].cities[2])];
        let v = judge_offer(&g, us, them, &rich, &two, false);
        assert_eq!(v.why.as_deref(), Some("one city a deal"), "{v:?}");
        // Carried out (as losing).
        g.players[us].power = 20;
        g.players[them].power = 400;
        g.teams[tt].ai_war_success[tu] = 100;
        let at = g.city_plot(c);
        g.implement_deal(them, us, std::slice::from_ref(&peace), &ours, false);
        assert!(g.players[them].cities.iter().any(|&k| g.city_plot(k) == at), "the city is theirs");
        assert!(!g.at_war(tu, tt));
    }

    /// A civic or a religion adopted in a deal is kept for `peace_treaty_length` turns: the AI's
    /// own civics and religion step leaves it alone and counts the timer down.
    #[test]
    fn a_civic_and_a_religion_adopted_in_a_deal_are_kept_for_a_while() {
        let (mut g, us, them) = game(69, &FOUR[..2]);
        let d = g.d;
        let team = g.players[us].team;
        for t in d.techs.keys() {
            g.team_set_has_tech(team, t, true, Some(us), false, false);
        }
        let length = d.defines.peace_treaty_length;
        let their_team = g.players[them].team;
        for t in d.techs.keys() {
            g.team_set_has_tech(their_team, t, true, Some(them), false, false);
        }
        // A civic it does not run, which they run (one that keeps a state religion possible).
        let c = d
            .civics
            .keys()
            .find(|&c| {
                d.civics[c].civic_option_type.is_some_and(|o| g.players[us].civics[o].is_some_and(|cur| cur != c && (d.civics[c].state_religion || !d.civics[cur].state_religion)))
                    && g.player_can_do_civics(us, Some(c))
            })
            .unwrap();
        let option = d.civics[c].civic_option_type.unwrap();
        g.player_set_civics(them, option, Some(c));
        g.implement_deal(them, us, &[], &[td(TradeableItem::TRADE_CIVIC, c.i())], false);
        assert_eq!(g.players[us].civics[option], Some(c));
        assert_eq!(g.players[us].ai_civic_timer, length);
        end_anarchy(&mut g, us);
        // A religion.
        let (_, r2) = two_religions(&mut g, us);
        let their_city = g.players[them].cities[0];
        g.city_set_has_religion(their_city, r2, true, false, false);
        g.player_convert(them, Some(r2));
        g.players[us].conversion_timer = 0;
        assert!(g.player_can_trade_item(us, them, &td(TradeableItem::TRADE_RELIGION, r2.i()), false));
        g.implement_deal(them, us, &[], &[td(TradeableItem::TRADE_RELIGION, r2.i())], false);
        end_anarchy(&mut g, us);
        assert_eq!(g.player_get_state_religion(us), Some(r2));
        assert_eq!(g.players[us].ai_religion_timer, length);
        for n in 1..=length {
            crate::empire::manage(&mut g, us);
            assert_eq!(g.players[us].civics[option], Some(c));
            assert_eq!(g.player_get_state_religion(us), Some(r2));
            assert_eq!((g.players[us].ai_civic_timer, g.players[us].ai_religion_timer), (length - n, length - n));
        }
    }
}
