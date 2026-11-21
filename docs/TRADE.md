# Trade: routes, caravans, cargo

**Status: spec (2026-11-21). Nothing built.** This game's own rules, replacing the trade routes
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

`base = length x (origin population + destination population) x TRADE_PAY_PERCENT / 100`

with the length counted to at most `TRADE_LENGTH_CAP` plots. Then percent terms, summed:

| Term | Percent |
|---|---|
| foreign | `TRADE_FOREIGN_PERCENT` (+100) |
| the path has a water plot | `OVERSEAS_TRADE_MODIFIER` (+25) |
| the origin's `trade_route_modifier` (buildings) | as the data |
| foreign: the origin's `foreign_trade_route_modifier` | as the data |
| growth (6.2) | 0 to `TRADE_GROWTH_MAX` (+50) |

Each living caravan carries half of the result, with its cargo's premium (section 5) on its half.

`TRADE_PAY_PERCENT` and `TRADE_LENGTH_CAP` are set from the first measure of section 10, so that
a civilization's routes together pay about what its trade routes pay today at the same turn.

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

Before the numbers are set (CLAUDE.md: a rules change is not gated on the AI, but this one moves
the economy):

- what share of each civilization's commerce its trade routes supply today, at turns 50, 100,
  200 (sets `TRADE_PAY_PERCENT`);
- path lengths between the city pairs a route could link, own and foreign (sets
  `TRADE_LENGTH_CAP`);
- tradeable spare copies a civilization, by turn, luxury and strategic (if nearly every caravan
  can be loaded, the cargo is no choice: lower the premium or limit cargo);
- team pairs with open borders, by turn (how early a foreign route can exist).

With the first build: routes a civilization by turn; caravans killed, by the killer's kind
(civilization at war, hidden nationality, barbarians, animals) and where (own land, foreign,
unowned, water); route-turns closed, by reason; the share of commerce from routes against today's.
If animals and barbarians kill most early caravans, the guard is too weak.

## 11. Defines (`rules_defines.jsonc`)

`INITIAL_TRADE_ROUTES` 2, `CARAVAN_MOVE` 12, `CARAVAN_COST_ROAD` 3, `CARAVAN_COST_LAND` 6,
`CARAVAN_COST_WATER` 4, `CARAVAN_REFORM_TURNS` 10,
`TRADE_PLUNDER_TURNS` 10, `TRADE_PAY_PERCENT` and `TRADE_LENGTH_CAP` (from section 10),
`TRADE_FOREIGN_PERCENT` 100, `TRADE_DEST_PERCENT` 50, `TRADE_GROWTH_PERCENT` 2,
`TRADE_GROWTH_MAX` 50, `TRADE_CARGO_PERCENT` 50, `TRADE_CARGO_NEW_PERCENT` 150. Data:
`UNIT_CARAVAN`, `PROMOTION_CARAVAN_GUARD_2` to `_4`. Removed: `MAX_TRADE_ROUTES`, `THEIR_POPULATION_TRADE_PERCENT`,
`CAPITAL_TRADE_MODIFIER`, `FOREIGN_TRADE_MODIFIER`, `OUR_POPULATION_TRADE_MODIFIER` and its offset,
`TRADE_PROFIT_PERCENT`, `IGNORE_PLOT_GROUP_FOR_TRADE_ROUTES`.

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
