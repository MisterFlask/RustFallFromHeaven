# Armies

This game's own rules, not FFH2's: the second system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), "Armies and recruitment". Land combat units serve in named
armies or in a city's garrison. A trained unit appears in the city that trained it, as in Civ4; a
city may have a **waypoint** (a plot, or one of its owner's armies), and the units it trains walk
there under an ordinary go-to order, which their owner can replace with any other. This file says
what was built, how the spec's open points and edge cases were decided, and what is left.

Recruitment logistics were removed on 2026-11-23 (the owner's decision): an army's requisition
from picked cities, the recruit destinations per order and per city, the marching recruits the
rules moved, the army target size as a player setting, and supply's gating of recruits. The
measurements showed no benefit for what they cost the player in control of units and queues
(docs/AI_LOG.md, "Requisition and marching recruits"); waypoints replace them.

| Phase (spec) | What | Status |
|---|---|---|
| 4 Armies and recruitment | armies, garrisons, supply, form / split / join / merge / garrison / rename / disband / upgrade all, AI, UI | done |
| 5 Governors | (the "Feed army" mode, replaced by requisition on 2026-10-31, itself removed on 2026-11-23) | done |
| Waypoints (2026-11-23) | rules, saves, telemetry, UI (city screen, Armies screen, unit panel) | done |
| 6 Engagement combat | formation battles (revision 4: no stances), commander | in progress: formation battles in the rules (docs/ENGAGEMENTS.md, 2026-10-31) |
| 7 FfH layer | Incite Mutiny (a cultural action against an army: there are no cultural actions since 2026-11-21), commander auras | later |

Code: rules `crates/ffh-core/src/army/` (`mod.rs`: armies, garrisons, the sync, the commands, the
march estimate `march_turns`; `waypoint.rs`; `fleet.rs`; `squad.rs`; `supply.rs`; hooks: the
production queue's train order in `city_pop_order` (`city/turn.rs`), the player's turn processing
in `player/turn.rs`, the orders that end a waypoint in `group/missions.rs`, `group/movement.rs`
and `unit/actions.rs`, `unit_convert` in `unit/lifecycle.rs`, `city_kill`); tuning
`data/json/rules_defines.jsonc` (the `SUPPLY_*` and `STARVATION_*` defines, docs/SUPPLY.md); AI
`crates/ffh-ai/src/armies.rs` (the main army's empire orders) and `military.rs`
(`armies::join_main_army`); UI `crates/ffh-play/src/view/armies.rs`, the unit panel's army actions
(`view/units.rs`), the `city_waypoint` command, `assets/app/armies.js`. Tests:
`crates/ffh-core/tests/armies.rs`, `crates/ffh-core/tests/waypoints.rs`,
`crates/ffh-ai/src/armies.rs`; the harness checks that a unit on its way to a waypoint is in no
army, that every waypoint names an army of its owner or a plot on the map, and that empire orders
lead their queue (`ffh_harness::check`).

**Fleets** (docs/NAVAL.md, section 7, 2026-11-11): the container has a domain (`Army::domain`,
`ArmyDomain::Land` or `Sea`). A sea army is a fleet ("1st Fleet", numbered apart from the armies)
of a player's warships and ships with cargo room; it shares the sync, the commands and Upgrade
all, can be a coastal city's waypoint, and has no supply line, no squads and no garrison; it holds
a standing order (`Army::station`: Patrol of a sea, Escort). A land army's standing orders: Raid
(docs/RAIDS.md) and the Escort of a caravan (docs/TRADE.md, 4.4). Everything below about armies is
about land armies unless it says otherwise; the code that only means land armies skips fleets
(`Game::army_is_fleet`). An army carried by ships stays one army aboard and lands as one group
(`Army::carried`).

Every game has armies, the AI-only ones (`ffh autoplay`, `ffh stats`, the AI batches, soaks) too.
Until 2026-11-05 they were a game option (`GAMEOPTION_ARMIES`, on by default only in the browser's
new-game screen); the owner removed it, with those of the cultural actions and the governors.

## Rules

**Squads** (2026-11-21): an army and a city's garrison are divided into squads, the units a battle
commits (docs/ENGAGEMENTS.md, "Squads"); the army commands keep whole squads together where they
can (a squad split off or merged whole keeps its cells).

**Army units.** Land units with a strength, except mages (`UNITCOMBAT_ADEPT`), recon units
(`UNITCOMBAT_RECON`), heroes (`PROMOTION_HERO`: a unit that has it, and so a unit type that gives
it for free), summons (a duration), animals (`UnitInfo::animal`: a civilization's Wolf Pack or
Bear, since 2026-11-10, docs/SUPPLY.md "Second pass" B; it fights beside an army as heroes and
mages do) and the barbarians' units. Ships, air units, settlers and missionaries are not army
units either. The others move on their own, or along with an army's group (a hero travelling with
an army is not in it: the commander slot is phase 6).

**Armies.** An army is a persistent, named container (`Army`, `Unit::army`): "1st Army", "2nd
Army", ... (the lowest free number), renamed at will. Its units are kept in one selection group, so
it moves as one at the speed of its slowest member, as a Civ4 group does; several armies may share
a plot. An army unit on its own outside a city (after a split, a move out of a city) is a one-unit
army. An army has no commander. `Army::target_size` is the AI's bookkeeping (the size its war
needs); the rules do not read it and the player does not set it.

**Garrisons.** An army unit in one of its owner's cities, in no army and not on its way to a
waypoint, is in that city's garrison. The garrison and every army on the plot defend the city
together (Civ4 already picks the defender among every unit on the plot, a unit on its way
included).

**The army sync.** Units change groups for many reasons (the human's Group and Split buttons, a
unit's move, the AI's grouping), so after every human command, after each army command and in each
player's turn processing (Civ4's doTurn) the armies are settled with the groups: each group of army
units has one army; the oldest group holding units of an army keeps that army, so a part that split
off becomes a new army; a group whose units had none gets a new one (a garrison group stays the
garrison); an army without units is gone. Units on their way to a waypoint are left out: they are
in no army until they arrive. Once the armies are settled, the units on their way that stand on
their target's plot arrive (below), and the armies are settled again.

**Commands** (the unit panel's Army section; each one click):

- *Form a new army* of the selected units (from armies or a garrison).
- *Split into a new army*: some units of one army.
- *Join the N-th Army*: the selected units join an army on their plot (garrison units included);
  *Merge into* when the whole army is selected. The cities and units whose waypoint is the merged
  army name the army it joined; the fleets escorting it escort that army.
- *Join the garrison* of the city they stand in.
- *Disband*: the units walk to the nearest city (a go-to order) and join its garrison when they
  arrive; the army is no waypoint from then on. In a city it is *Join the garrison*.
- *Upgrade all*: each unit takes its best upgrade (the strongest, then the dearest), the most
  experienced first, while the gold lasts.
- *Rename* (the Armies screen).

**Recruitment.** A trained unit is created in the city that trained it as any unit (build effects,
experience, promotions, upkeep, unit class limits); an army unit is then in the city's garrison,
unless the city's waypoint sends it on. An army standing in the city does not take it, unless the
city's waypoint is that army.

**Waypoints** (`army/waypoint.rs`). A city may have a waypoint (`City::waypoint`): a plot, or an
army or fleet of its owner.

- *Setting it*: the city's owner sets or clears it at any time, for the units the city trains from
  then on. A plot must be on the map and revealed to its owner's team; an army must be its own and
  not disbanding; a fleet is a waypoint only for a coastal city. Nothing checks that the plot can
  be reached or suits the city's units: a unit that cannot follow it stays (below).
- *Who follows*: to an army or fleet, a unit that can serve in it (an army unit for an army; a
  warship or a ship with cargo room for a fleet); to a plot, a unit with a strength whose domain
  matches the plot (land units to land, ships to water; a hero or a mage follows a plot, never an
  army). Settlers, workers, missionaries, caravans and air units follow none. In a governed city
  only the units of empire orders follow (docs/GOVERNORS.md); the governor's own units stay in the
  garrison.
- *Setting out*: a unit that follows the waypoint, trained on its target's plot already, arrives
  at once (an army standing in the city that is the city's waypoint takes it). Otherwise, if its
  group has a path to the target's plot (the rules' path, as a go-to finds it), it gets an
  ordinary go-to order there (`MISSION_MOVE_TO`) and its waypoint (`Unit::waypoint`) is set. With
  no path it stays in the city, an ordinary unit (in the garrison), and its owner is told
  ("Axeman: no way to plot 30, 8.").
- *On the way*: an ordinary unit in every respect. Its go-to moves it as any go-to does; it attacks
  and is attacked, defends a plot it stands on, and can be promoted, upgraded and cast with. It is
  in no army and no garrison while on its way (the army sync leaves it out; its squad body is
  loose), so it has no supply line (docs/SUPPLY.md).
- *Any order from its owner ends the waypoint* for that unit (no message, the order is its
  owner's): a mission (move, fortify, sleep, sentry, ...), joining a group that holds other units
  (for every unit of that group), wake, cancel, automate, load or unload, a direct attack (an
  engagement its owner launches), an army command on it (form, join, to the garrison). From then
  on it is an ordinary unit of an army or a garrison. A promotion, an upgrade or a spell is no
  order: an upgrade or a conversion that keeps its owner keeps its waypoint and go-to. The rules'
  own continuation of the go-to, and a transport's landing, keep it.
- *Once a turn*, in its owner's turn processing after the army sync: a unit on its target's plot
  arrives; toward an army whose plot is not its go-to's, it gets a new go-to to where the army
  stands (a group still busy with a shown move waits a turn); with no path there, or toward a plot
  whose go-to the rules dropped, the waypoint ends: its missions are cleared and its owner is told
  ("Warrior: no way to the 2nd Army. It waits for orders.").
- *Arrival* (once a turn, and in every army sync): its waypoint is cleared. To an army or fleet it
  joins that army (the explicit join: the army keeps its name, group, orders and supply counter;
  "Warrior joined the 1st Army."). To a plot outside its owner's cities it joins the oldest army of
  its owner and its domain standing there (by the turn it was formed), else it stays alone (a
  one-unit army by the sync). On a plot of one of its owner's cities it is in the garrison.
- *Target gone*: an army destroyed or disbanded clears the waypoints naming it (a message for each
  city: "Ithralia: waypoint cleared. The 2nd Army is gone.") and ends the ways of the units going
  to it (their missions cleared, "Warrior: the 2nd Army is gone. It waits for orders."); an army
  merged into another hands it its waypoints, the cities' and the units'. A city that leaves its
  owner (captured, traded, flipped, razed) has no waypoint; its units already on their way keep
  theirs.
- Telemetry: `waypoint` (`sent`, `arrived`, `ended`), docs/TELEMETRY.md. A unit killed on the way
  was counted `sent` and is counted neither `arrived` nor `ended`, so `sent` can exceed the other
  two.

**Supply** (docs/SUPPLY.md, which replaces the rule of `ARMY_SUPPLY_RANGE` plots of the trade
network). Every army needs a supply line, the cheapest path over land back to a city or a ship of
its owner within a budget; cut, it is hungry for two turns (no healing), then starving (HP lost
each turn, less damage dealt).

## Edge cases decided (beyond the spec's text)

- **The garrison does not move, but its units do.** The spec's garrison "cannot move"; here a
  garrison unit the human moves out of its city leaves the garrison and forms (or with others,
  joins) an army as it leaves. Forbidding the move would only add a click (Form army, then move):
  the move is the transfer.
- **An army entering a city stays an army.** It does not dissolve into the garrison (armies pass
  through cities); *Join the garrison* is one click.
- **Armies are their groups.** Civ4's own Group, Split and Ctrl-click selections still work and
  keep the armies consistent through the sync: grouping two armies merges them (the older army
  keeps its name and takes the other's waypoints), splitting a group splits its army. So no unit is
  ever in two places, and nothing a Civ4 player is used to stops working.
- **A unit on its way is in no army and no garrison.** It walks alone under its go-to, so it is not
  a one-unit army with a supply line of its own, and it does not swell the garrison of a city it
  passes through.
- **Arrival is the explicit join.** The unit joins its army as *Join* does, never by the group
  merge (which would let the older group take the army over): the army keeps its name, its group
  and its orders, and its supply counter (the joining unit, from no army, brings none).
- **An army in the producing city takes the unit only when it is the city's waypoint.** Otherwise a
  trained army unit is in the garrison, as without armies: the player chooses where a city's units
  go, and an army passing through does not take them.
- **Governed cities send only their empire orders' units.** The governor's units are for its city;
  the player's empire orders are the player's choice, and they follow the waypoint.
- **No path at training is no waypoint.** The unit stays in its city's garrison with a message,
  rather than holding a go-to it cannot follow. A way lost later (the army went where it cannot be
  reached, the go-to dropped) ends the waypoint with its missions cleared, so the unit waits for
  orders and is never left on a stale go-to.
- **An army gone or merged.** Destroyed or disbanded, it clears the cities' waypoints naming it and
  the units' ways (they wait for orders where they are); merged, the cities and units name the
  army it joined.

## Open points for the owner

- **Supply** is decided in docs/SUPPLY.md.
- **A unit on its way walks alone.** Units bound for one army each follow their own go-to; going
  together (grouping them, which ends their waypoints) is the player's to do. A waypoint that
  gathers units before they set out would be a next step if lone units are picked off too often.

## AI (summary; docs/AI.md "Armies")

The AI's groups become armies through the sync, so its movement logic drives each army unchanged.
At war or preparing one, its main field army is the largest army of two or more units in supply;
it stays the main one while it is at least two thirds the size of the largest. Its target size
(`Army::target_size`, the AI's bookkeeping) is the army the war needs
(`diplomacy::army_needed_for_war`). While the main army is short of that, counting the field
units outside it on its landmass and the field orders in its cities' queues, the AI places empire
unit orders in its governed cities, one per city: the soonest to deliver (training plus the march
by land, within 20 turns), not a city that needs its production for its own defence, nor one with
a ritual under way or an empire order already queued. The unit is siege up to a third of the army,
then the best city attacker (`armies::reinforcement_unit`). New attackers are gathered by the
campaign's own movement; an idle field stack on the main army's landmass walks to it
(`armies::join_main_army`, `military::army`'s last resort). The AI sets no waypoints. A city
defender that walked to another of its cities joins that city's garrison.

A sanity run (2026-11-23; `ffh-stress`, 4 seeds, 150 turns; not compared against the base, not a
verdict): no panic or check failure; main armies of 4.8 to 6.7 units on average at war (max 8, the
stack limit), 7 to 65 such empire orders a game, 1 to 2 civilization cities conquered a game by
turn 150. The measurements of the removed requisition are in docs/AI_LOG.md.

## UI (ffh-play)

- **Armies screen** (F5, the menu's Armies): each army has its name (editable), units, place (a
  click selects it on the map), supply, Upgrade all and Disband. Its card says which cities have it
  as their waypoint ("Waypoint of") and which units are on their way to it ("On the way").
- **City screen.** A waypoint line with a selector: none, one of the player's armies (fleets for a
  coastal city), or a plot picked on the map (a pick-a-plot mode). It sends `city_waypoint`
  (`city`, `to`: null, `{"army": id}` or `{"plot": [x, y]}`), docs/HUMAN.md. Under the queue: the
  city's garrison.
- **Unit panel.** Each unit's row shows its army (amber; red when out of supply) or "garrison". A
  unit on its way says so: "Waypoint: the 2nd Army (3 turns)". With a whole army selected, a line
  gives its size and its supply, with a link to the Armies screen.
- **Messages**: a unit with no way to its city's waypoint, a unit joining its army on arrival, a
  waypoint ended (no way there, its army gone) and a city's waypoint cleared.
