# The sea: range, boats, control, fleets, far shores

**Status: spec (2026-11-10). Built in four phases; each phase's status is in its heading. Phase 1
(section 3) built in the rules (`crates/ffh-core/src/naval.rs`, the boats rules in the unit code,
supply at sea in `crates/ffh-core/src/army/supply.rs`, tests in `crates/ffh-core/tests/naval.rs`)
with the AI minimum of section 8's first two bullets (`crates/ffh-ai/src/naval.rs`); the client
(section 10) not yet.** This
game's own rules, not FFH2's (the owner's direction, 2026-11-10: the naval game inherited from
Civ4 is irrelevant and there are no colonial adventures). This file is the spec for the
implementation, as docs/SUPPLY.md is for supply lines. Marked **(default)**: a value or rule chosen
for a point the owner left open, open to change when it plays badly.

Not built, on purpose: boarding and capture, and a formation battle at sea (the owner, 2026-11-10:
"ignore boarding for now"; without boarding and a roster with roles a sea formation battle is a
strength comparison). Naval combat stays the duel. Also not built: beachheads, native states,
named seas.

## 1. Why

In Civ4 a fleet can only sink other fleets, a land power can ignore the sea, ships can go
anywhere so there is nothing to hold, and nothing overseas is worth the trip. The rules below give
the sea three things:

- **a shape**: ships and boats operate within range of a port, so ports and island chains matter;
- **a say in the land war**: a blockade closes a port city to a siege's relief, an army fed from
  the sea is fed only while the water is held, ships fire into battles on the coast, and every
  army can cross water, slowly and helplessly, so a warship has prey and an invasion needs escort;
- **a pull**: landmasses nobody starts on hold goods no start landmass has, taken cheaply by
  outposts whose goods come home along a lane a raider can sit on.

## 2. Words

- **Warship**: a sea unit (`DOMAIN_SEA`) with a strength, not an animal (the Sea Serpent, the
  Giant Tortoise, the Kraken and the Leviathan are not warships).
- **Port** of a team: a city next to a water plot. A port serves the water areas it touches. A
  team's ports are the ports of its own players, of its vassals and its master, and of teams it
  has open borders with (not while at war with them). A starved outpost (section 9) is no port.
- **Sea zone** of a warship: the water plots within `SEA_ZONE_RANGE` (1) of it, in its own water
  area; `SEA_STATION_RANGE` (2) while its fleet holds a Patrol or Blockade station (section 7).
  A plot is in an **enemy zone** for a team when it is in the zone of a warship that is an enemy
  of that team (`unit_is_enemy`: at war, or always hostile / hidden nationality of another
  owner). The rules use the truth, as supply lines do; what a player is shown uses what the player
  sees.
- **Lane**: a fixed line over water from a plot to a home port (section 5). It does not bend
  around enemies: it is **cut** while any of its plots is in an enemy zone.

## 3. Phase 1: range, boats, wear -- status: built in the rules (the client: not yet)

### 3.1 Operating range

- **The range field.** For each team and water plot: the fewest steps over water (8 directions,
  through passable water plots of one water area) from a water plot next to one of the team's
  ports; a plot next to a port is at 1. A plot is **in range** when that number is at most the
  team's range: `SEA_RANGE` (8) plus the `sea_range_change` of each tech the team knows (Optics +4,
  Astronomy +4) **(default)**. A lake's plots count like any water.
- **No ocean ban.** The Galley and the Trireme lose `terrain_impassables: TERRAIN_OCEAN`. Range
  replaces it.
- **Out of range.** At the start of its owner's turn a sea unit or a land unit in boats (3.2)
  standing on a water plot out of range loses `SEA_ATTRITION_PERCENT` (15) of its full HP, and it
  does not heal that turn. This kills **(default)**. A unit carried by a ship takes no attrition
  of its own. Message on the first turn out of range and when a unit is lost.
- **Exempt**: the barbarians, animals, and a player whose team has no city (range is not traced
  at all). A hidden-nationality ship uses its owner's ports.
- The field is derived state: it is computed from the map, never saved, and may be cached as long
  as the cache cannot change a game's course (a cache rebuilt when ports, open borders, vassalage,
  war or techs change; or recomputed at need).

### 3.2 Boats: land units cross water

Any land unit that can move may step from land onto a water plot and stand there **in boats**. No
transport is built, no tech is needed **(default)**. Settlers and heroes too **(default)**.

- A land unit on a water plot that is not cargo of a ship is in boats. A unit that steps onto a
  water plot where a ship of its owner has room for it is loaded as before; otherwise it is in
  boats.
- **Moves.** Entering a water plot, from land or water, ends the unit's moves for the turn, and a
  unit in boats enters at most `EMBARKED_MOVES` (1) water plot a turn, plus its civilization's
  `embarked_moves_change` (the Lanun +1: two water plots a turn) **(default)**. Stepping from
  water onto land ends its moves.
- **Strength.** A unit in boats defends with strength `EMBARKED_STRENGTH` (1), with no modifier of
  any kind (no promotions, terrain, fortification, first strikes or withdrawal). A fight on a
  water plot is the duel, as naval combat is.
- **It cannot attack.** A unit in boats cannot enter a plot that holds an enemy unit or an enemy
  city, on water or on land, and a land unit on land cannot attack onto a water plot. It lands on
  an empty plot (or a friendly one) and fights from there.
- **It does nothing else.** In boats a unit does not heal, fortify, cast, build, pillage, found
  or use any other action; it can move, wait or be deleted. Its sight is 1 plot.
- **Where.** Any passable water plot. Out of range it takes attrition (3.1).
- **Who defends.** Civ4's best-defender rule already puts a warship before a unit of strength 1,
  so a ship on the same plot is the escort.
- **Barbarians and animals** never use boats.

### 3.3 Supply at sea

An army whose plot is water (in boats): supplied while its plot is in range (its line is that one
plot; the source is the nearest port by the range field, ties to the lower city id); out of range
it is cut (Hunger, then Starvation, on top of the attrition). An army carried by a source ship is
supplied as before. Landed, it needs a line like any army: a city, or a source ship (section 6).

### 3.4 Wear

A warship of a player (not the barbarians) counts the starts of its owner's turns it has spent
outside a city: `Unit::sea_turns`. A turn start in a city of a team whose ports are its team's
ports (section 2) sets it to 0. Above `SEA_WEAR_FREE_TURNS` (10) the ship's strength is lowered by
`SEA_WEAR_PERCENT` (5) per turn above, to at most `SEA_WEAR_MAX_PERCENT` (50), as one more modifier
in attack and defence. Message when wear starts.

### 3.5 As built (2026-11-10)

Where the code is:

- `crates/ffh-core/src/naval.rs`: the range field and its cache (`Game::plot_sea_range`,
  `plot_sea_port`, `plot_in_sea_range`, `team_sea_range`, `team_sea_exempt`, `team_is_port_team`,
  `city_is_port`), boats (`unit_can_use_boats`, `unit_needs_boats`, `unit_in_boats_at`,
  `unit_is_in_boats`, `unit_embarked_moves`, `unit_embarked_step_cost`), attrition and wear
  (`unit_sea_range_applies`, `unit_is_out_of_sea_range`, `unit_sea_no_heal_at`, `unit_is_warship`,
  `unit_sea_wear_percent`, `plot_is_home_port`, `player_update_sea`, called next to
  `player_update_supply` at the owner's turn start, before healing).
- The boats rules where the unit code decides: `unit_can_move_into` (entering water, no attack,
  no enemy plot or city), `plot_movement_cost` (the embarked step), `unit_can_defend` and the
  duel's strength terms (`duel_combat_terms`: strength 1, no modifier; the wear term
  `TermSource::SeaWear`), first strikes, defensive withdrawal and defensive strikes (none in
  boats), `unit_heal_rate` and `unit_do_turn` (no healing, no fortify turns), `unit_can_fortify`,
  `unit_can_cast`, `unit_can_found`, `unit_can_bombard`, `unit_can_range_strike`,
  `group_can_start_mission` and `unit_can_do_command` (the actions left), `unit_visibility_range`
  (sight 1), the captures in `unit_update_combat` and `unit_set_xy` (none in boats),
  `plot_verify_unit_valid_plot` (a unit in boats is not bumped off the water).
- Supply at sea: `SupplySource::Sea(city)`, the army's own plot as the line.
- The AI: `Game::path_ai_sea_blocked` (path.rs; the AI-side restriction of the path finder, with
  the flag `MOVE_BOATS`), `ffh-ai` `naval::sea_blocked`, `ship_to_port`, `in_boats`,
  `walk_home_in_boats` (called from `supply::stranded_overseas`).
- Save format 32: `Unit::sea_turns`, `Unit::sea_out_of_range` (the first-turn message).

Choices made where this spec was silent (each open to change):

- **Passable water** for the field: a water plot that is not impassable (terrain or feature). The
  field steps diagonally between two land plots (ships cannot; the field ignores it).
- **The cache.** The field is cached per team, checked on every read against the teams whose
  cities were its ports and the players' teams, and dropped when a city is added or deleted or a
  plot's type, terrain or feature changes. A read equals a fresh computation.
- **"A team with no city"**: no player of the team has a city (a vassal's or an ally's do not
  count).
- **The embarked step** costs the unit's full moves split `EMBARKED_MOVES` + the civilization's
  change ways, rounded up: one plot a turn ends its moves; the Lanun's first water plot from land
  costs half of them, not all (so they do cross two a turn). Landing ends the moves (Civ4's rule
  for a landing from a ship, which this keeps).
- **Loading**: stepping onto water where a ship of its *team* with room stands loads it (Civ4's
  loading), at one move, not the whole turn. A unit carried by a ship cannot unload onto open
  water (as in Civ4); it steps off onto a neighbouring water plot instead, into boats.
- **Sight 1** applies to every land unit standing on water that neither flies nor walks on water:
  in boats, carried, or on a Causeway (a unit's sight may not change while it stands still, or the
  plots' sight counts would not be undone; gaining or losing flight or water walking re-applies
  it).
- **What a unit in boats may do**: the missions move, move to a unit, skip, sleep and sentry; the
  commands wake, cancel, stop automation, delete, load, promote. No first strikes, no withdrawal,
  no defensive strike.
- **Capture**: a unit in boats is never captured; a non-combatant in boats (a settler) dies when an
  enemy takes its plot.
- **Wear**: a warship is a sea unit that can fight (`unit_can_fight`), not an animal; its count
  goes up at each of its owner's turn starts outside a port city of its team (a city of a team
  whose ports are its team's) and goes to 0 at one inside. Wear is a term of the duel's strength;
  engagements never include ships.
- **Supply at sea, the known line** (`plot_supply_line_known`): at sea it reads the range field as
  it is (the ports are cities of the army's own side).
- **The AI** (section 8): the rules' path finder keeps an AI-controlled ship in range from leaving
  its range (one already out of range may cross any water to come back) and an AI land group off
  water it would be at sea on, unless the search carries `MOVE_BOATS` (then only water in range);
  `ffh-ai`'s reach flood does the same. A ship out of range, or a warship carrying nothing whose
  wear is above half its maximum, makes for the nearest port by the field. A stranded army walks
  home in boats when a path in boats over water in range reaches a city of ours and no enemy
  warship it sees is within 3 plots; a group of ours in boats goes on to the nearest city of ours
  it reaches in boats. An AI army also steps aboard a transport of ours at sea beside it (the rules
  now allow it). The war-target and settler crossings are not built.

## 4. Phase 2: who holds the water -- status: not built

### 4.1 The harbour of a city

A coastal city (`city_is_coastal` with the ocean's minimum water size) has a harbour. It is
**blockaded** while a warship at war with its owner holds the Blockade order (Civ4's plunder
mission, which the rules already have: the water plots it covers cannot be worked and carry no
trade) within `SHIP_BLOCKADE_RANGE` of the city, in a water area the city touches. Otherwise it is
**open**. `Game::city_harbour(c)` answers Open / Blockaded (with the blockading ships) / None; a
pure test of the map, nothing saved.

### 4.2 A port under siege

A besieged city (docs/SIEGES.md) whose harbour is open is relieved by sea: its units heal at
`PORT_SIEGE_HEAL_PERCENT` (50) of what they would heal unbesieged, and its Sentinels repair at that
percent of their repair **(default)**. Besieged and blockaded, or not coastal: as docs/SIEGES.md
(no healing from the city, no repair).

### 4.3 Supply ships need a lane

A ship is a supply source (docs/SUPPLY.md, section 2) only while it has an open lane (section 5)
to a port of its owner's team. The ship's own plot is part of the lane, so an enemy warship next
to the ship cuts it. The known-line search (`plot_supply_line_known`) counts only enemy warships
the viewer sees.

### 4.4 Ships fire into battles on the coast

When an engagement (docs/ENGAGEMENTS.md) is fought on a land plot, each side's warships with a
bombard rate (`bombard_rate > 0`) that stand on a water plot next to the battle's plot, belong to
a player of that side and have not fired this turn (`Unit::shore_fired`, cleared at their owner's
turn start) fire once before round 1: at most `SHORE_FIRE_SHIPS` (3) a side, the strongest first.
Each shot is one SHOOT hit by the resolver's own damage rule, the ship's attack being its strength
on the units' stat scale, at a target chosen as an archer's first shot is. Ships take no damage and
are not fighters. The preview, the forecast and the battle report show the shots.

## 5. Lanes

`Game::sea_lane(team, from: PlotIdx) -> Option<SeaLane>`: the shortest path over water (8
directions, passable water, one water area) from `from` (a water plot, or the water plots next to
a land plot) to a water plot next to a home port, where every plot is in the team's range. Ties:
fewer plots, then the lower plot index at the first difference (as supply lines). `SeaLane` has
the path, the port, and `cut_by: Option<UnitId>`: the enemy warship (lowest unit id) whose zone
holds the first plot of the path, counted from `from`, that is in an enemy zone. A lane with
`cut_by` is cut; no path is no lane. Which ports are home ports is said where a lane is used:
4.3 (any port of the team), 9.3 (the owner's cities that are not outposts).

## 6. Summary of what a navy now does to the land war

| | Without ships | With the water held |
|---|---|---|
| Take a walled port | the city heals and repairs at half rate through the siege | blockade: no healing, no repair |
| Feed an army overseas | a city must be taken | a transport with an open lane is a source |
| Cross water | boats: 1 plot a turn, strength 1 | escorted, or carried at the ships' speed |
| Fight on the coast | | up to 3 ships fire before round 1 |
| Enemy crossing | lands where it likes | a patrol intercepts it (7.3) |

## 7. Phase 3: fleets -- status: not built

### 7.1 A fleet is an army of ships

Warships and ships with cargo room serve in named **fleets**: the armies' container
(docs/ARMIES.md) with a domain. `Army::domain` is land or sea; a sea army is shown and named as a
fleet ("1st Fleet", numbered apart from the armies). It has what an army has that makes sense at
sea: one group that moves as one, the sync with the groups, form / split / join / merge / rename /
disband, a target size, requisition from the cities its owner picks (coastal ones; the ship is
built there and sails to the fleet as a recruit marches), upgrade all. It has no supply line
(range and wear instead), no tactic, no garrison (a ship in no fleet is simply a ship), and it
fights the duel. A lone warship outside a city is a one-ship fleet, as a lone army unit is a
one-unit army. Barbarian ships are in no fleet.

### 7.2 Stations

A fleet holds one standing order, `Army::station`:

- **None**: under way, or waiting.
- **Patrol**: stays on its plot; its ships' zones have `SEA_STATION_RANGE`; it intercepts (7.3).
- **Blockade**: Patrol, and its ships hold the Blockade order (the plunder mission), so the cities
  in `SHIP_BLOCKADE_RANGE` are blockaded (4.1).
- **Escort** of one of its owner's armies or ships: whenever that group ends a move on a water
  plot the fleet moves to the same plot if it can reach it this turn; it has no zone bonus.

A station ends when the fleet is given a move order. Taking a station ends the fleet's moves for
the turn.

### 7.3 Interception

When a group moves onto a water plot in the zone of a fleet on Patrol or Blockade that is its
enemy, the fleet intercepts: its strongest ship that has not intercepted this turn
(`Unit::intercepted`, cleared at its owner's turn start) and whose odds against the plot's best
defender are at least `INTERCEPT_MIN_ODDS` (50%) attacks that plot once, as a normal attack that
does not move the ship. Then the mover goes on if it lives. One interception per ship per turn; a
group is intercepted at most once per plot it enters, by each enemy fleet.

## 8. What the AI does

A rules change is not gated on the AI (CLAUDE.md): it does something reasonable with each rule.

- Ships path only through water in range, and go to port when wear is above half its maximum.
- A land army does not use boats on its own initiative, except: to come home when stranded
  overseas (in place of asking for a ship, when the way is in range and no enemy warship it sees
  is within 3 plots of it); to reach a war target it has no land path to, under the same test; a
  settler or colonist bound for a site across water.
- Warships not otherwise used form a fleet that patrols off the port of the AI's nearest to an
  enemy that has ships; when an army of its besieges a coastal city, a fleet takes the Blockade
  station off that city if it is in range.
- A Colonist (section 9) is built for an overseas site in range that has a resource, when no
  settler is bound there.

## 9. Phase 4: far shores, outposts, lanes -- status: not built

### 9.1 Far shores (map generation)

After the starting plots are set: a **far shore** is a land area with no starting plot and at
least `FAR_SHORE_MIN_PLOTS` (8) plots. If the map has none, nothing changes. Otherwise, through
the map RNG:

- each bonus flagged `far_shore` in `bonuses.jsonc` (Gems, Silk, Incense **(default)**) that
  stands on an area with a starting plot is moved to a plot on a far shore that can hold it; one
  that fits nowhere stays;
- each far shore gets one more raw mana node (`BONUS_MANA`) per `FAR_SHORE_MANA_PLOTS` (40) of its
  plots, at least one, on plots that can hold it.

### 9.2 Outposts

An **outpost** is a city with `City::outpost` set.

- **Founding.** The Colonist (`UNIT_COLONIST`: half the Settler's cost, the Settler's art, no
  population cost) founds outposts; the Settler founds a city or, by a second action, an outpost.
  The site: a plot where a city can be founded, next to water of the ocean's minimum size, on a
  land area other than the one its owner's capital stands on.
- **What it is.** Population at most `OUTPOST_MAX_POP` (2). Its borders and the plots it works are
  the 8 plots around it and do not grow. It builds nothing (no queue, no hurry, no buildings of
  its own), has no governor, sends no recruits, pays no maintenance and does not count as a city
  for any maintenance, civic or victory count. Its crews improve its land (docs/WORKS.md). It is a
  port. Its resources reach its owner while its lane is open (9.3).
- **Raising it.** For `OUTPOST_RAISE_COST` (150, scaled by game speed) gold its owner makes it a
  city: `outpost` is cleared and every rule of cities applies from then.
- A captured outpost stays an outpost.

### 9.3 An outpost's lane

Each outpost has a lane (section 5) from its plot to the nearest home port: a city of its owner
that is not an outpost. At the start of its owner's turn:

- **Open**: `City::lane_cut_turns` = 0.
- **Cut** or **no lane**: `lane_cut_turns` + 1; the outpost is off the trade network (its
  resources reach nobody, it has no trade routes). If cut, the owner of the cutting ship gains
  `LANE_RAID_GOLD` (8) gold.
- Above `LANE_STARVE_TURNS` (5) it is **starved**: it loses 1 population a turn, to 1 at least,
  and it is no port.

Messages: cut (with the plot of the cutting ship), starved, open again; to the raider, the gold.

## 10. What the player sees

Text is bare (CLAUDE.md, "Rules text and tooltips"). Every effect is shown where it bites, naming
the ship or fleet that causes it.

**The Sea layer** (a map layer, on by itself while a ship, a fleet or a unit in boats is
selected; a toggle with the other map layers):

- water out of the player's range is tinted and the range's edge drawn; hovering a port or an
  outpost shows the range it gives;
- lanes (phase 4; a supply ship's, phase 2), drawn as supply lines are, flowing to the home port,
  in their owner's colour; a cut lane breaks at the cutting ship with the turns left before the
  outpost starves; lanes of others the player has revealed are drawn too;
- the zone of each fleet on station the player sees, as a ring;
- a wear pip on a ship that has wear.

**Always on**: a harbour mark on a coastal city's banner when it is blockaded (and a second state
when besieged and blockaded).

**The path preview** (it already shows turns, odds and supply) adds, for the path under the
pointer:

- `In boats from plot 3: strength 1, 1 plot a turn. 9 turns.`
- `Out of range from plot 6: -15% HP a turn.`
- `In the zone of the 2nd Fleet (Lanun): will be intercepted.` (phase 3)
- for an attack on a coastal city: `Harbour open: the city heals and repairs at 50% during the
  siege.` / `Harbour blockaded.`
- for a ship: `Cuts the lane of Vallus (Bannor).` (phase 4)

**Panels**:

- unit panel, a ship: `Range: in range (5 of 12).` / `Out of range: -15% HP a turn.`;
  `Wear: none (turn 4 of 10).` / `Wear: -15% strength. Enter a port to clear.`; a unit in boats:
  `In boats: strength 1, cannot attack, does not heal.`;
- city panel: `Harbour: open.` / `Harbour: blockaded by 2 ships (Lanun). Sea plots not worked,
  no sea trade.`; an outpost: `Lane: open, to Ithralia (14 plots).` / `Lane: cut by a Privateer at
  (41, 17), turn 2 of 5.` / `Starved: -1 population a turn, not a port.`;
- the army's supply line: `Supplied: at sea, in range of Ithralia.` / `No supply line: the
  transport's lane is cut by a Frigate (Sheaim).`;
- the battle preview and report: each side's ships that fire, with their shots;
- the Armies screen gains a **Fleets** section (phase 3): ships, station (a selector: none,
  Patrol, Blockade, Escort with its target), wear, what it blockades;
- the Cities list shows outposts as rows marked `outpost`, with the lane's state, and a `Raise to
  city (150 gold)` action in the outpost's panel (phase 4);
- a resource's plot tooltip, when no land area the player has a city on holds that resource:
  `Not on your landmasses.` (phase 4).

**Messages**, on the turn of the change, with the plot: out of range; a unit lost at sea; wear
starts; a city blockaded / the blockade ends; a supply ship's lane cut; intercepted (both sides);
an outpost's lane cut, starved, open again; gold from a lane.

## 11. Defines

`rules_defines.jsonc`: `SEA_RANGE` 8, `SEA_ATTRITION_PERCENT` 15, `EMBARKED_MOVES` 1,
`EMBARKED_STRENGTH` 1, `SEA_WEAR_FREE_TURNS` 10, `SEA_WEAR_PERCENT` 5, `SEA_WEAR_MAX_PERCENT` 50,
`SEA_ZONE_RANGE` 1, `SEA_STATION_RANGE` 2, `PORT_SIEGE_HEAL_PERCENT` 50, `SHORE_FIRE_SHIPS` 3,
`INTERCEPT_MIN_ODDS` 50, `FAR_SHORE_MIN_PLOTS` 8, `FAR_SHORE_MANA_PLOTS` 40, `OUTPOST_MAX_POP` 2,
`OUTPOST_RAISE_COST` 150, `LANE_RAID_GOLD` 8, `LANE_STARVE_TURNS` 5. Data keys: a tech's
`sea_range_change`, a civilization's `embarked_moves_change`, a bonus's `far_shore`.

## 12. Measures

(Each phase adds what its games showed.)
