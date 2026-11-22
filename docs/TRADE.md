# Trade: routes, caravans, cargo

**Status: phase 1's rules built (2026-11-21: sections 3, 4.1-4.3, 6, 7; "As built", section 14).
The AI of phase 1 built (section 8, "As built"). Not built: cargo (section 5), escort (4.4), the client (section 9).** This game's own rules, replacing the trade routes
inherited from Civ4 (each city's slots filled by the rules with the best-paying partner cities:
`city_update_trade_routes`). This file is the spec for the implementation, as docs/SUPPLY.md is for
supply lines and docs/NAVAL.md for the sea. Marked **(default)**: a value or rule chosen for a
point the owner left open, open to change when it plays badly. Unmarked rules of sections 3 to 7
are the owner's decisions (2026-11-21).

## 1. Why

Trade today cannot be seen or touched: the player chooses nothing, nothing moves on the map, and
an enemy has nothing to strike. The owner's design, after Galactic Civilizations: a player
establishes a few routes by hand, each carried by caravans that travel the map and can be
attacked. A longer route pays more and is harder to guard. A route over water is one more thing
that holding a sea decides.

The decisions:

- Few, large routes, chosen by the player. Routes between a player's own cities exist and pay
  less than routes to another civilization.
- A route is established once and then runs by itself: no order is ever given to its caravans.
- Two caravans a route. Each pays while it lives and is raided separately.
- A raid weakens a route sharply and never destroys it. Nothing has to be re-established.
- Killing a caravan is an act of war, except by a unit of hidden nationality.
- A route to another civilization needs open borders with it and no agreement from it.
- The sender owns the route. A caravan may carry one of the sender's resources: the resource goes
  to the recipient, the higher pay to the sender, more when the recipient lacked the resource.
  The cargo is chosen when the route is established.
- No city along the way is paid. Resource deals in diplomacy stay as they are.

## 2. Words

- **Route**: a standing link from an **origin** city of its **owner** to a **destination** city,
  the owner's or another player's.
- **Domestic** route: the destination's owner is the route's owner. **Foreign**: any other.
- **Caravan**: one of the two units that carry a route. On a water plot it is drawn as a merchant
  ship; it is the same unit.
- **Cargo**: one copy of a resource of the owner, carried by one caravan of a foreign route.
- **Open** / **closed**: whether the route runs this turn (3.4).
- **Luxury**: a resource with happiness or health. **Strategic**: every other resource, mana
  included.

## 3. Routes

### 3.1 How many

- A player's routes are limited to `Player::trade_routes` (`INITIAL_TRADE_ROUTES`, now 2
  **(default)**, plus 1 for each tech and civic that has `trade_routes`) plus the game's
  `trade_routes` (the votes). `coastal_trade_routes` (a civic, the Lanun's building) add routes
  that only a coastal city can be the origin of.
- A city is the origin of at most 1 route plus its `extra_trade_routes` (the buildings that have
  `trade_routes`). It is the destination of any number.
- `MAX_TRADE_ROUTES` is removed. With today's data a civilization has 2 to about 6 routes.
- An outpost (docs/NAVAL.md, 9.2) is neither origin nor destination **(default)**.

### 3.2 Establishing

- The **Caravan** (`UNIT_CARAVAN`, cost 60, no tech **(default)**) is trained in any city. It is
  not an army unit, costs no upkeep and counts for no military total.
- Standing in a city of its owner that can be an origin, with a route free under 3.1, it takes
  the order **Establish route**: a destination and, for a foreign route, each caravan's cargo
  (section 5). The destinations offered are the cities that pass 3.3.
- The Caravan then travels to the destination by itself. The route exists from the order (its
  slot is used); it starts to pay when the Caravan arrives. On arrival the second caravan forms
  in the origin city, free, and both shuttle between the two cities from then on.
- A Caravan killed before it arrives re-forms as any caravan does (4.3): the route is not lost.

### 3.3 Which destinations

A city other than the origin, revealed to the owner, that is not an outpost, where:

- foreign: `player_can_have_trade_routes_with` holds both ways (open borders, a vassal or its
  master; not at war; neither side under a no-foreign-trade civic);
- a path exists (3.5);
- no route of the owner already links the two cities, in either direction.

### 3.4 Open and closed

A route is **closed** while its destination fails 3.3 (war, open borders ended, no path, the
civic), and **open** otherwise. It is checked at the start of its owner's turn and when war is
declared.

- Closed: it pays nothing and delivers no cargo. Its caravans are removed from the map and wait
  in the origin city. Its growth (6.2) is kept, except that war between the two owners sets it
  to 0.
- Open again: the caravans leave the origin on the owner's next turn. Nothing is rebuilt.
- **Seizure.** When war starts between the owners of a route's two ends, each caravan of it
  standing on a plot owned by the enemy is lost to that enemy as if raided (4.3).
- A route **ends** (its slot is free, its caravans are gone) only when its owner dissolves it,
  its origin leaves its owner's hands, or its destination is destroyed. A destination that
  changes owner keeps the route, open or closed by 3.3 with the new owner **(default)**.

### 3.5 The path

`Game::route_path(owner, origin, destination)` is a pure search of the route's own (as
`plot_supply_line` is supply's), not the units' path finder: the cheapest path from the origin's
plot to the destination's, 8 directions, by plot entered:

| Plot | Cost |
|---|---|
| land with a route (a road, anyone's) | `CARAVAN_COST_ROAD` (3) |
| other land | `CARAVAN_COST_LAND` (6) |
| water | `CARAVAN_COST_WATER` (4) |

- It may enter: passable land that is unowned, the owner's team's, or a team's whose borders the
  owner may enter; passable water in the owner's team's sea range (docs/NAVAL.md, 3.1) whose sea
  is not closed to the owner's team (4.3 there; water with no sea counts as open); the
  destination city and any city of a team whose land it may enter. Never a city of a team whose
  land it may not enter, and no diagonal step the rules refuse a ship (between two land plots).
- Ties: the lower cost, then the fewer plots, then the lower plot index at the first difference.
- It ignores units. It is traced at the start of the owner's turn and stored (`Route::path`); its
  number of plots, the origin's left out, is the route's **length** that turn.

## 4. Caravans

### 4.1 Movement

- A caravan moves by the route's rule, not by movement points: at the end of each turn of its
  owner it steps along the route's path toward the city it is bound for, spending
  `CARAVAN_MOVE` (12) a turn on the path's costs (3.5): 4 plots of road, 2 of open land, 3 of
  water **(default)**. A step that costs more than what is left of the turn's 12 waits for the
  next turn. Entering the city it is bound for ends its turn; the next turn it is bound for the
  other city.
- A caravan off the path (the path changed under it) goes to the nearest plot of the new path by
  the same search, then on.
- It does not step onto a plot it could enter only by attacking (a unit its owner is at war
  with): it waits.
- On water it is not in boats (docs/NAVAL.md, 3.2 and the attrition of 3.1 do not apply): it keeps
  its guard **(default)**. A patrol intercepts it as any mover (4.4, point 4 there).
- It takes no orders, cannot attack, pillage, fortify or be loaded on a ship, never waits for
  orders, and cannot be deleted (dissolving the route removes it).
- The second caravan forms in the origin when the first reaches the destination, so the two
  travel in opposite phase.

### 4.2 The guard

The Caravan has a stat sheet of its own (strength 3; it only defends). Its guard grows with what
its owner can field: at each start of its owner's turn it holds the promotions
`PROMOTION_CARAVAN_GUARD_2` to `_4` up to the highest tier (`UnitInfo::tier`) of an army unit its
route's origin city can train **(default)**; each raises its strength and its sheet's Defense
and HP. It gains no experience and takes no other promotion.

On land an attack on its plot is a formation battle (docs/ENGAGEMENTS.md) in which it stands as
a loose squad, beside whatever else defends the plot; on water it fights the duel, behind any
warship on its plot by the best-defender rule. It heals as a unit does.

### 4.3 Raids

- **Who may attack.** A caravan is a unit of its owner: attacking it needs war with that owner,
  and the attack declares it where the rules let an attack declare war. A unit of hidden
  nationality attacks it without war, as it attacks any unit. The barbarians and animals attack
  it as they attack any unit. A patrol intercepts it at sea (docs/NAVAL.md, 4.4, point 4).
- **Killed.** The killer's owner gains **plunder**: `TRADE_PLUNDER_TURNS` (10) times what that
  caravan paid its owner last turn, cargo included (new gold; the barbarians gain nothing). The
  route's growth is halved. The caravan is gone, and with it its half of the pay and its cargo's
  delivery.
- **Re-forming.** After `CARAVAN_REFORM_TURNS` (10) turns of its owner the caravan forms again in
  the origin city, free, with the cargo it had. With both caravans dead the route pays nothing
  until the first re-forms.
- A caravan is never captured.

### 4.4 Escort

The Escort standing order (docs/NAVAL.md, 7.2) takes a caravan as its target, for a fleet on the
water legs and for an army on land **(default; phase 3)**.

## 5. Cargo

Foreign routes only. Domestic routes carry none **(default)**.

- **Choosing.** At the Establish order each caravan is given no cargo or one resource of which the
  owner has a tradeable copy (`player_get_num_tradeable_bonuses`: its own, not imported) not
  already given to another caravan. The two caravans of a route carry different resources.
- **Fixed.** The cargo does not change afterwards, except by a **refit**: a second Caravan unit
  standing in the origin takes the order Refit route, sets both cargos again and is used up. The
  route keeps its growth.
- **Delivering.** A cargo is delivered on each turn its caravan is alive, its route is open and
  its owner has a copy to send (one not imported and not sent by another caravan). While
  delivered, the owner has one copy less (an export) and the recipient one more (an import to its
  capital's network), as a deal's resource. The copy the recipient gets cannot be traded on.
- **Not delivered** (the caravan dead, the route closed, the copy lost): the copy is back with
  its owner and the caravan carries nothing. It resumes by itself.
- **The premium.** A delivered cargo raises its caravan's pay by `TRADE_CARGO_PERCENT` (50). It
  raises it by `TRADE_CARGO_NEW_PERCENT` (150) instead while the recipient has no copy of that
  resource other than caravans' deliveries (its own and its deals' imports count as having it)
  and this cargo is, among the caravans delivering that resource to that recipient, the one of
  the oldest route (ties: the lower route id).
- The recipient pays nothing and decides nothing.

## 6. Pay

### 6.1 The route's pay

At the start of its owner's turn, for an open route, in hundredths:

`base = effective length x (origin population + destination population) x TRADE_PAY_PERCENT / 100`

The effective length counts the route's plots in full up to `TRADE_LENGTH_FULL`, at half (rounded
down) from there up to `TRADE_LENGTH_CAP`, and nothing beyond:
`min(L, FULL) + (min(L, CAP) - FULL) / 2` when L is above FULL (at most 16 with the defaults).
Then percent terms, summed:

| Term | Percent |
|---|---|
| foreign | `TRADE_FOREIGN_PERCENT` (+25) |
| the path has a water plot | `OVERSEAS_TRADE_MODIFIER` (+25) |
| the origin's `trade_route_modifier` (buildings) | as the data |
| foreign: the origin's `foreign_trade_route_modifier` | as the data |
| growth (6.2) | 0 to `TRADE_GROWTH_MAX` (+50) |

Each living caravan carries half of the result, with its cargo's premium (section 5) on its half.

### 6.2 Growth

`Route::growth`, in percent: +`TRADE_GROWTH_PERCENT` (2) each turn of its owner on which the route
is open with both caravans alive, to `TRADE_GROWTH_MAX` (50). Halved when a caravan is killed.
Kept while closed; 0 after a war between the two owners.

### 6.3 Who is paid

- The owner: the sum of its caravans' pay enters the origin city as trade yield, as a trade
  route's profit does today (`city_set_trade_yield`: commerce, through the player's
  `trade_yield_modifier`) **(default)**.
- The destination city: `TRADE_DEST_PERCENT` (50) of the route's pay before cargo premiums, by
  the living caravans, the same way **(default)**. On a domestic route both cities are the
  owner's.
- A city's trade is the sum of these, in hundredths, rounded to the nearest whole.

## 7. What goes

- `city_update_trade_routes`, `City::trade_route`, the profit rules (`city_base_trade_profit`,
  `city_total_trade_modifier`, the population, capital and peace-turn terms) and their defines.
- Kept and re-read: the data keys `trade_routes`, `coastal_trade_routes`, `trade_route_modifier`,
  `foreign_trade_route_modifier`, `trade_yield_modifier` (3.1, 6.1); `City::trade_yield`.
- A blockade's "no sea trade" is 3.5: the path does not cross a sea closed to the owner.
- `crates/ffh-cppai` gets the minimal mechanical fix (CLAUDE.md).
- docs/PORTING.md, "Changes from FFH2", lists the change. Save format: `Game::routes` (a slab,
  `RouteId`), `Unit::route`.

## 8. What the AI does

A rules change is not gated on the AI (CLAUDE.md): it does something reasonable with each rule.

- **Establishing.** With a route free, a city that can be an origin trains a Caravan when the
  best destination's pay repays its cost within 30 turns. The destination: the highest pay among
  those whose path, as the AI knows the map, enters no land of a team it is at war with.
- **Cargo.** Luxuries it has a spare copy of, those the recipient lacks first. A strategic
  resource only to its vassal or master, or to a team at war with a team it is at war with. It
  never loads its only copy and never refits.
- **Raiding.** A unit already hunting (an army's hunt, a warship's attack) takes an enemy caravan
  in reach as a target at good odds. Its hidden-nationality units hunt the caravans of every
  team that is not its vassal, master or friend. The barbarians do nothing new.
- **Guarding.** One more patrol need for its warships: a sea its routes pass through
  (`naval::patrol_target`).
- **Open borders.** The resources it receives from the other's caravans count in what open
  borders are worth to it.

### As built (2026-11-21, the AI of phase 1: `crates/ffh-ai/src/caravans.rs`; docs/AI.md, "Caravans")

- **Establishing**: a Caravan is trained in the city that is the origin of the best offer among the
  player's cities that can be an origin now, once the player holds 2 cities, while a route is free
  that no route-less Caravan claims, one in training at a time, when the offer repays the Caravan's
  hammers within 30 turns (a hammer for a commerce). An offer's worth is its owner's pay plus the
  destination's share when the destination is ours. The Caravan walks (over land, its own landmass)
  to that origin and establishes there. Not as section 8 says: the destinations are those the rules'
  path allows (`route_offers`; the path is the rules', not "as the AI knows the map"), and an offer
  whose path crosses a sea where an enemy patrols, as the AI sees it, is skipped.
- **Replacing** (not in section 8): with no route free, an offer worth twice our least-paying route
  (a closed route counts 0) gets a Caravan; standing in its origin it dissolves that route and
  establishes the new one in the same decision.
- **Cargo**: not built (section 5).
- **Raiding**: as section 8 (an enemy caravan is a hunt and warship target, its plunder in its
  value; hidden-nationality units attack the caravans of every team that is not theirs, a vassal, a
  master or one they are Pleased with), with "friend" read as Pleased or better.
- **Guarding**: the seas of our open routes' paths are a guard need of `warships::guard` (there is no
  `naval::patrol_target`).
- **Open borders**: not the cargo's resources (no cargo yet) but the route itself: a better route to
  one of their cities than our best domestic one (or our least-paying route, with none free) is one
  attitude step toward open borders, estimated by plot distance.

## 9. What the player sees

Text is bare (CLAUDE.md, "Rules text and tooltips"). Screenshots of each go to the owner when
built (CLAUDE.md, "Show UI changes in a screenshot").

- **The Establish dialog**: one row per destination with its pay, length and what the path
  crosses (`Ithralia to Vallus (Lanun). 17 plots. 14.2 a turn. Passes Bay of Lorda: contested.`);
  per caravan a cargo selector with the result (`Gems: Lanun has none. +150%.` /
  `Silk: Lanun has it. +50%.` / `Your only copy: your cities lose Gems.`); the routes left
  (`Routes: 2 of 3. This city: 1 of 1.`).
- **A Trade screen**: one row per route: the cities, open or closed with the reason
  (`Closed: no open borders with Lanun.`), length, pay with its terms, growth
  (`Growth: +24% of 50%.`), each caravan (`On the road: plot 7 of 17.` / `Lost on turn 112 to
  the barbarians. Re-forms in 6 turns.`) and its cargo (`Gems: delivered. Premium +150%.` /
  `Gems: Lanun now has its own. +50%.`); Dissolve. A second list: routes of others into the
  player's cities and what each brings.
- **On the map** (a Trade layer, on by itself with a caravan selected): each route of the player
  as a line along its path in its colour; the routes of others over the plots the player has
  revealed. A caravan's tooltip: `Caravan: Ithralia to Vallus. Cargo: Gems.
  Plunder: 84 gold.`
- **The attack preview** on a caravan: the plunder, and `Attacking declares war on Lanun.`
- **The city panel**: the route the city is the origin of and each route that ends in it, with
  its pay to this city.
- **Messages**, on the turn of the change, with the plot: a caravan lost (to whom, the growth
  left), re-formed, seized; a route closed or open again; a premium gained or ended; to the
  raider, the plunder.
- The help text of every tech, civic and building with one of the data keys of section 7 says
  what the key now does (`+1 route.` / `This city can be the origin of 1 more route.` /
  `+50% pay of the route from this city.`).

## 10. Measures

**Measured before the numbers were set** (2026-11-21, origin/master b325e973; 12 AI games, seeds 1-12, 6 civilizations, standard map, 200 turns; a throwaway `ffh trade-measure`, not kept). Civilizations here stay small: a median of 3 cities at turn 200 (secession splits them: 6 become about 13 a game).

| | turn 100 | turn 150 | turn 200 |
|---|---|---|---|
| commerce a civilization, median | 18 | 23 | 32 |
| old trade routes' commerce, median (share of commerce) | 3 (14%) | 3 (11%) | 3 (9%) |
| old pay per filled slot | 1.00 | 1.02 | 1.05 |
| route limit of 3.1, median (above 2: samples) | 2 (0 of 76) | 2 (2 of 114) | 2 (39 of 160) |
| plot distance of own city pairs, median / p90 | 5 / 10 | 6 / 12 | 7 / 15 |
| plot distance of foreign pairs that may trade, median / p90 | 31 / 48 | 25 / 40 | 26 / 44 |
| civilizations with any foreign city they may trade with | 5 of 76 | 50 of 114 | 65 of 160 |
| team pairs with open borders | 3 of 204 | 34 of 499 | 51 of 994 |
| luxuries with a spare copy, mean a civilization | 0.14 | 0.39 | 0.49 |
| strategic and mana resources with a spare copy, mean | 0.02 | 0.03 | 0.05 |

What the numbers set:
- An old route paid 1 commerce, and a civilization's routes 3 together. A route that pays 1 is not worth a Caravan, so the new routes pay more than the old: `TRADE_PAY_PERCENT` 500 gives a route between two of a civilization's own cities 7 plots apart with 8 population between them 2.8 commerce a turn to the origin and 1.4 to the destination. Two such routes are about 8 commerce where the old routes gave 3.
- Foreign cities are four times as far as a civilization's own. With pay in proportion to length and the foreign term at +100 a foreign route paid 10 to 15 times a domestic one. So length counts in full to 8 plots and at half to 24 (`TRADE_LENGTH_FULL`, `TRADE_LENGTH_CAP`), and the foreign term is +25: a foreign route 30 plots long pays about 3 times the domestic route above before growth and cargo.
- Open borders are rare and late among the AIs (the first pair at a median of turn 119), so most AI routes are domestic until the AI is given a reason to open its borders (section 8).
- Spare copies are rare: a cargo is usually a civilization's only copy. The premium is not lowered.

**Measured with the AI of phase 1** (2026-11-21, branch `trade-ai`; the same 12 games: seeds
1-12, 6 civilizations, standard map, 200 turns; `ffh stats --at 50,100,150,200 --every 1` with
`FFH_TELEMETRY=1`, the trade fields of the snapshots). Civilizations alive at the checkpoint:

| | turn 50 | turn 100 | turn 150 | turn 200 |
|---|---|---|---|---|
| civilizations | 72 | 77 | 118 | 174 |
| routes a civilization, mean domestic / foreign | 0 / 0 | 0.82 / 0.06 | 0.82 / 0.30 | 0.97 / 0.56 |
| civilizations with a route | 0 | 47 | 79 | 130 |
| routes open / all | - | 68 / 68 | 122 / 132 | 232 / 266 |
| commerce a civilization, median (before: 18 / 23 / 32 at 100 / 150 / 200) | 12 | 19 | 27.5 | 37.5 |
| routes' trade a civilization, median (before: the old routes' 3 / 3 / 3) | 0 | 2 | 5 | 9.5 |
| routes' trade, share of all commerce | 0% | 16% | 25% | 35% |
| team pairs with open borders (before: 3 of 204, 34 of 499, 51 of 994) | 2 of 180 | 7 of 210 | 37 of 545 | 59 of 1196 |

The first route of a game: turns 58 to 86, median 72. Routes established 357 (262 domestic, 95
foreign), 61 of them replacing a route. Caravans lost 151 in 27,000 caravan-turns in a route
(5.6 per 1000): to the barbarians and animals 69, to a civilization at war 51, to a killer not at
war (hidden nationality) 24, no killer 7; on the owner's land 69, foreign land 43, unowned 27,
water 12. Plunder 3,486 gold in 66 kills. Route-turns: open 14,578; closed for no open borders 629,
war 318, no path 209, the destination not revealed 161 (a captured destination is a new city the
owner has not seen yet).

With the first build: routes a civilization by turn; caravans killed, by the killer's kind
(civilization at war, hidden nationality, barbarians, animals) and where (own land, foreign,
unowned, water); route-turns closed, by reason; the share of commerce from routes against today's.
If animals and barbarians kill most early caravans, the guard is too weak.

## 11. Defines (`rules_defines.jsonc`)

`INITIAL_TRADE_ROUTES` 2, `CARAVAN_MOVE` 12, `CARAVAN_COST_ROAD` 3, `CARAVAN_COST_LAND` 6,
`CARAVAN_COST_WATER` 4, `CARAVAN_REFORM_TURNS` 10,
`TRADE_PLUNDER_TURNS` 10, `TRADE_PAY_PERCENT` 500, `TRADE_LENGTH_FULL` 8, `TRADE_LENGTH_CAP` 24,
`TRADE_FOREIGN_PERCENT` 25, `TRADE_DEST_PERCENT` 50, `TRADE_GROWTH_PERCENT` 2,
`TRADE_GROWTH_MAX` 50, `TRADE_CARGO_PERCENT` 50, `TRADE_CARGO_NEW_PERCENT` 150. Data:
`UNIT_CARAVAN`, `PROMOTION_CARAVAN_GUARD_2` to `_4`. Removed: `MAX_TRADE_ROUTES`, `THEIR_POPULATION_TRADE_PERCENT`,
`CAPITAL_TRADE_MODIFIER`, `FOREIGN_TRADE_MODIFIER`, `OUR_POPULATION_TRADE_MODIFIER` and its offset,
`TRADE_PROFIT_PERCENT`, `IGNORE_PLOT_GROUP_FOR_TRADE_ROUTES`, and
`FOREIGN_TRADE_FULL_CREDIT_PEACE_TURNS` (the peace-turn term's; its only reader went with it).

## 12. Phases

1. **Routes and caravans**: sections 3, 4.1 to 4.3, 6 and 7, the AI's establishing and raiding,
   the dialog, the Trade screen and the lines on the map. The old trade routes go in this phase.
2. **Cargo**: section 5, its AI and its texts.
3. **Escort and guarding**: 4.4, the AI's patrol need.

## 13. Open

- **Animals.** An early caravan on unowned land meets them with the first guard. Section 10 counts
  it; the answers are a stronger first guard or animals that leave caravans alone.
- **Strategic resources as quantities** (docs/DECKS.md, "Stockpiles"): caravans would carry the
  amounts. A later system; resources stay on or off here.

## 14. As built (2026-11-21, phase 1's rules)

Where the code is:

- `crates/ffh-core/src/trade/`: `mod.rs` (the types: `Route`, `CaravanSlot`, `Leg`,
  `RouteRefusal` with `text` and `key`, `RouteOffer`, `RoutePay`; what a caravan is:
  `caravan_unit_type`, `unit_is_caravan`, `unit_in_route`, `group_has_route_caravan`; the limits:
  `player_route_limit`, `player_coastal_route_extra`, `player_routes_used`, `player_routes`,
  `city_origin_limit`, `city_origin_used`, `city_origin_refusal`, `city_is_route_coastal`,
  `city_routes_from`, `city_routes_into`; establishing: `unit_can_establish_route`,
  `unit_establish_origin_refusal`, `unit_establish_refusal`, `unit_establish_route`; the guard:
  `route_guard_tier`), `path.rs` (`plot_route_cost`, `route_path`, `route_path_seas`,
  `route_destination_refusal`, `route_offers`: one Dijkstra with the tie rule of 3.5, as
  `plot_supply_line`), `pay.rs` (`route_effective_length`, `route_pay_percent`,
  `route_pay_total_x100`, `route_pay`, `city_route_trade_x100`, `city_route_trade_yield`,
  `trade_refresh_pay`, `trade_stale`, `trade_mission_profit`), `caravans.rs` (`route_caravan_lost`,
  `route_dissolve`, `routes_on_war`, `player_update_routes`, `player_move_caravans`, and the hooks
  `unit_route_killed`, `routes_on_city_gone`, `routes_city_changed_hands`).
- The turn: `player_update_routes` at the owner's turn start, after `player_update_lanes`
  (`player_do_turn_units`); `player_move_caravans` at its turn end, after `player_fleet_stations`
  (`player_do_turn`, before its gold and its cities' turn: a first arrival pays that same turn).
  War: `routes_on_war` in `team_declare_war` right after `at_war` is set (before units are bumped),
  and in the crusade Host's war. Cities: `city_kill` (through `city_kill_as`, whose
  `changing_hands` `player_acquire_city` sets) and `player_acquire_city` (the routes into a captured
  or traded city go to its new id). Kills: `unit_kill`, before the delayed death.
- The pay refresh runs from `city_set_population` (when a route touches the city), the origin's two
  modifier changers, `player_change_trade_yield_modifier`, and every change of a route's state. The
  harness (`ffh_harness::check`) checks `trade_stale`: every city's trade yield and every route's
  stored pay equal a fresh computation, and routes and caravan units refer to each other.
- The exemptions of a caravan in a route (`Unit::route`): `unit_in_boats_at` (never in boats: the
  duel's strength, attrition, the no-heal, the boats' limits), `unit_can_defend` (it defends on
  water), `plot_verify_unit_valid_plot` (never bumped), `unit_is_immune_to_capture`,
  `unit_can_load_unit`, `group_can_start_mission`, `unit_can_do_command` (delete included),
  `group_ready_to_move` (never waiting for orders: the AI and the client skip it),
  `unit_can_join_group_opt` (it travels alone). Any Caravan: free of unit upkeep
  (`player_calculate_unit_cost_breakdown`, counted as the summons are), not an army unit
  (`unit_type_is_army_unit` names it), not brought back by Resurrection (`unit_fallen_record`).
- Data: `UNIT_CARAVAN` / `UNITCLASS_CARAVAN` (`units.jsonc`, `unit_classes.jsonc`; the barbarians
  cannot train it; `no_experience`; the Settler's art by `manifest.txt`), `PROMOTION_CARAVAN_GUARD_2`
  to `_4` (`promotions.jsonc`), the defines of section 11 in `rules_defines.jsonc`;
  `INITIAL_TRADE_ROUTES` changed in place in `global_defines.jsonc`. Save format 63: `Game::routes`,
  `Unit::route`; no `City::trade_route`, `trade_cities`.
- Telemetry counts (`tel_trade`): `trade_route` (domestic / foreign), `trade_caravan_lost_by`
  (war / hidden / barbarians / other), `trade_caravan_lost_at` (own / foreign / unowned / water /
  other), `trade_route_turn` (open, or the closing reason's key), `trade_plunder` (gold).
- Tests: `crates/ffh-core/tests/trade.rs`.

Choices made where this spec was silent (each open to change):

- **Entering land** (3.5): "a team whose borders the owner may enter" is its own, a friend's (a
  vassal or master), one with open borders, one it is at war with, or the barbarians. A city on the
  path other than the destination is passed only when its team is its own, a friend's or open
  borders (never at war, never the barbarians'): entering it would be an attack. The destination
  city is entered even where its land could not be. A city plot costs as land, a road where it has
  one.
- **The first caravan** is the unit that took the order; it is bound for the destination. A
  caravan formed meanwhile (the second, on arrival) moves from the next turn end. A caravan that
  re-forms, or forms when its route opens again, is bound for the destination. Entering the city it
  is bound for ends its move for the turn; it is bound for the other city at once.
- **Off the path** (4.1): the rejoin is the route search to the nearest plot of the stored path,
  then the path toward the city it is bound for.
- **Waiting**: before each step the plot's cost is taken again (a sea closed meanwhile stops it)
  and the step waits on any unit at war with the owner's team there (`unit_is_enemy`, seen or not:
  animals, the barbarians and hidden nationality included).
- **The move** is `group_move` with the caravan as its forced unit (the rules' step: sight,
  entering a city, the sea patrols' interception, escorts); its movement points are restored after
  each step and its group is set asleep.
- **Closed** (3.4): the caravans are removed without a killer (through `unit_kill`, under the
  telemetry cause `trade`); the slots keep their re-form timers,
  which count down whether the route is open or not; the caravans form when it is open again. At a
  war declaration every route of both sides' players is checked (a path through a third party's
  land can change too).
- **War between the ends**: growth 0, at the declaration and at each turn start while the reason is
  war. Seizure takes each caravan on a plot owned by a player of the enemy team, credited to that
  plot's owner (plunder by 4.3).
- **A captured destination** keeps its route (moved to the new city's id); its caravans standing in
  the city at that moment go back to wait in the origin. A destination razed or destroyed ends it;
  an origin captured, traded or destroyed ends it.
- **Pay** (6.1): the route's whole pay in hundredths, then split: the first caravan gets the
  remainder of an odd total. The destination's share is `TRADE_DEST_PERCENT` of each living
  caravan's half. A city adds every route it is the origin or destination of and rounds once.
  "The path has a water plot" reads the stored path. Nothing is paid before the first arrival or
  while closed. A route's stored pay (`CaravanSlot::pay_x100`, `Route::dest_pay_x100`) is the pay
  as of the last refresh, which equals the pay now.
- **Growth** rises at the owner's turn start after the caravans that may form there have formed.
- **Plunder** is the caravan's stored pay times `TRADE_PLUNDER_TURNS`, rounded to the nearest gold.
  The barbarians and a death with no killer gain none. Killer kinds: no killer is `other`; the
  barbarian player is `barbarians` (animals included: the rules receive only the killing player,
  not the unit); a killer at war with the owner is `war`; any other is `hidden`.
- **The guard** (4.2): the highest `tier` of a unit type that is an army unit the origin can train
  now (`city_can_train`), set when a caravan forms and at each turn start. Each guard promotion is
  DEF +10, MDEF +10 and HP +33% (percents sum: x1.33, x1.66, x1.99).
- **Limits** (3.1): routes from inland cities are limited to the base limit; all routes to the base
  limit plus `coastal_trade_routes`. Coastal is Civ4's (`city_is_coastal`, the ocean's minimum
  size). The building key `global_trade_routes` raises `Player::trade_routes` as before.
- **Refusals**: a foreign destination's refusal names the war, else a no-foreign-trade civic (when
  open borders or free trade hold), else no open borders. A cargo given is refused (`CargoNotBuilt`)
  until section 5 is built; the slots keep a `cargo` field for it.
- **The Great Merchant's trade mission** (`unit_get_trade_gold`) read the old profit of a route
  from the city to the capital. It now reads one caravan's pay of a route from the city (its
  owner's) to the capital at growth 0, the plot distance as its length, rounded
  (`trade_mission_profit`), times the unit's `trade_multiplier`.
- **Disorder and plunder**: Civ4's trade routes stopped in a city in disorder or plundered; routes do
  not (`City::plundered` stays, unread by trade).
