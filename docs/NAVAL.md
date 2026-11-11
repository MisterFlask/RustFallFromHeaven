# The sea: range, boats, control, fleets, far shores

**Status: spec (2026-11-10). Built in four phases; each phase's status is in its heading. Phase 1
(section 3) built in the rules (`crates/ffh-core/src/naval.rs`, the boats rules in the unit code,
supply at sea in `crates/ffh-core/src/army/supply.rs`, tests in `crates/ffh-core/tests/naval.rs`)
with the AI minimum of section 8's first two bullets (`crates/ffh-ai/src/naval.rs`); phase 2
(sections 4 and 5) built in the rules (`crates/ffh-core/src/seas.rs`, lanes in `naval.rs`, shore
fire in `crates/ffh-core/src/engagement/shore.rs`, tests in `crates/ffh-core/tests/seas.rs`) with
section 8's Patrol bullet and the transports' range test; the client (section 10) not yet.** This
game's own rules, not FFH2's (the owner's direction, 2026-11-10: the naval game inherited from
Civ4 is irrelevant and there are no colonial adventures). This file is the spec for the
implementation, as docs/SUPPLY.md is for supply lines. Marked **(default)**: a value or rule chosen
for a point the owner left open, open to change when it plays badly.

Not built, on purpose: boarding and capture, and a formation battle at sea (the owner, 2026-11-10:
"ignore boarding for now"; without boarding and a roster with roles a sea formation battle is a
strength comparison). Naval combat stays the duel. Also not built: beachheads and native states.

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
- **Sea**: a named stretch of water, cut out when the map is made (4.1). Every rule about who
  holds the water is counted per sea (4.3).
- **Lane**: a fixed line over water from a plot to a home port (section 5). It does not bend
  around enemies: it is **cut** while it passes through a sea closed to its owner.

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

## 4. Phase 2: named seas and who holds them -- status: built in the rules (the client: not yet; 4.4 point 5 is phase 4's)

The owner's decision (2026-11-10): named seas. They replace this spec's first draft of zones
around each warship and of Civ4's blockade by range: the player reads one state per sea.

### 4.1 Seas (map generation)

After the map's water is final and before the starting plots: every water area of at least
`SEA_MIN_PLOTS` (6) plots is divided into seas; smaller water has no sea (`Plot::sea` is none
and no rule of this section applies there).

- An area gets `max(1, plots / SEA_TARGET_PLOTS (40))` seas (rounded to the nearest). Their seeds:
  the first a water plot of the area drawn by the map RNG, each next the plot farthest (steps
  over water, 8 directions) from every seed so far, ties to the lower plot index. Each plot
  belongs to the seed nearest over water, ties to the lower sea index.
- Each sea has a name drawn without repeats by the map RNG from `data/json/sea_names.jsonc` (at
  least 80 invented names: "Bay of ...", "The ... Narrows", "... Sea", "The ... Deep"; an area
  that is one sea and touches no map edge uses the names marked `lake`); when the list is
  exhausted, "Sea 81", ...
- Saved: `Plot::sea: Option<SeaId>`, `Map::seas` (name, plots). A plot that becomes water later
  takes the sea of a neighbouring water plot (the lowest sea index) or none; one that becomes
  land has none.

### 4.2 The Patrol order

A warship of a player standing on a water plot of a sea, outside a city, can take the **Patrol**
order (Civ4's plunder mission and blockading state, reused and renamed; its range rule,
`SHIP_BLOCKADE_RANGE` and `unit_update_plunder`'s plots in range, is removed). Taking it ends the
ship's moves; it ends when the ship moves, is given another order or leaves the sea. The
barbarians' ships do not patrol.

### 4.3 Who holds a sea

- A team's **strength** in a sea: the sum, over its warships on Patrol there, of each ship's
  strength as the duel counts it in defence without terrain (its damage and wear included),
  rounded down. Ships passing through count for nothing.
- A sea is **closed to** team A while some team E at war with A has strength above 0 there and at
  least `SEA_HOLD_PERCENT` (200) percent of A's strength there. E then **holds** it against A.
  Ships of hidden nationality on Patrol count as at war with every other team.
- What a player is shown (section 10), for its own team: *Held by you* (it holds the sea against
  every enemy that patrols there, and has strength), *Held by X* (closed to it), *Contested* (it
  and an enemy both have strength, neither holds), *Open* (no enemy patrols and it does not), and
  *Patrolled by X* for teams it is not at war with.
- Control is a pure function of the map; whatever is stored for speed (the plots' blockade
  counts, the trade network) is refreshed when a ship starts or ends Patrol, is damaged, dies or
  changes owner, and when war or peace is made.

### 4.4 What a closed sea does

In a sea closed to team A:

1. **Blockade.** A's players cannot work its water plots and no trade of A's passes over them
   (the existing blockade of plots, now set over the sea). A coastal city of A is **blockaded**
   when every sea it touches is closed to A (`Game::city_harbour(c)`: Open / Blockaded, with the
   holders / None for a city on no sea).
2. **Lanes** of A through it are cut (section 5).
3. **Supply ships** of A standing in it are no source (4.6).

And in any sea, held or not:

4. **Interception.** When a group moves onto a plot of a sea where an enemy of it has ships on
   Patrol, that enemy intercepts: its strongest ship on Patrol there that has not intercepted
   this turn (`Unit::intercepted`, cleared at its owner's turn start) and whose odds against the
   plot's best defender are at least `INTERCEPT_MIN_ODDS` (50%) attacks that plot once, as a
   normal attack from where it stands, without moving. Then the mover goes on if it lives. One
   interception per ship per turn. A sea is an abstraction: the patrol finds what enters it.
5. **Raiding.** At the start of A's turn, for each lane of an outpost of A (9.3) that passes
   through a sea where an enemy patrols without holding it, the enemy with the most strength
   there (ties: the lower team) gains `LANE_RAID_GOLD_CONTESTED` (4) gold; the lane stays
   open **(default)**.

### 4.5 A port under siege

A besieged city (docs/SIEGES.md) whose harbour is open is relieved by sea: its units heal at
`PORT_SIEGE_HEAL_PERCENT` (50) of what they would heal unbesieged, and its Sentinels repair at that
percent of their repair **(default)**. Besieged and blockaded, or on no sea: as docs/SIEGES.md
(no healing from the city, no repair).

### 4.6 Supply ships need open water

A ship is a supply source (docs/SUPPLY.md, section 2) only while the sea it stands in is not
closed to its owner's team and it has an open lane (section 5) to a port of its team. The known
line (`plot_supply_line_known`) uses the control the viewer can work out: only ships it sees
count for strength.

### 4.7 Ships fire into battles on the coast

When an engagement (docs/ENGAGEMENTS.md) is fought on a land plot, each side's warships with a
bombard rate (`bombard_rate > 0`) that stand on a water plot next to the battle's plot, belong to
a player of that side and have not fired this turn (`Unit::shore_fired`, cleared at their owner's
turn start) fire once before round 1: at most `SHORE_FIRE_SHIPS` (3) a side, the strongest first.
Each shot is one SHOOT hit by the resolver's own damage rule, the ship's attack being its strength
on the units' stat scale, at a target chosen as an archer's first shot is. Ships take no damage and
are not fighters. The preview, the forecast and the battle report show the shots.

### 4.8 As built (2026-11-11)

Where the code is:

- `crates/ffh-core/src/seas.rs`: `Sea` (`name`, `plots`, `closed_to`), `Map::seas`, `Plot::sea`,
  `SeaId`; generation (`Game::map_generate_seas`, called first in `set_initial_items`, so every
  game's map path runs it: `game_init` and `regenerate_map`) and `Game::map_cut_sea` (seas cut by
  hand, for tests and tools); the rule for plots that change type (`plot_update_sea_on_type_change`,
  called by `plot_set_plot_type`, the one funnel of every change: terrain spells, the Sea Recedes
  card, lakes); strength and control (`unit_sea_strength_x100`, `sea_patrollers`, `sea_strength`,
  `sea_state`, `sea_state_known`, `sea_state_known_if`, `sea_state_if_patrol`, `sea_closed_to`,
  `sea_is_closed_to`, `plot_sea_closed`, `sea_closed_known`; `SeaControl`, `SeaState`); the stored
  blockade (`sea_refresh`, `seas_refresh_all`, `plot_sea_refresh`, `sea_control_stale`, the
  messages); harbours (`city_seas`, `city_harbour` -> `Harbour`, `city_harbour_if_held`,
  `sea_ports_blockaded_if_held`, `sea_ports`); interception (`sea_interceptors`,
  `sea_interceptors_for`, `sea_intercept_step`, called at the end of `group_move`); the turn start
  (`player_update_seas`).
- Patrol: `unit_can_plunder`, `unit_update_plunder` (now: refresh the unit's sea) in
  `unit/actions.rs`; the refresh hooks in `unit_set_damage`, `unit_set_has_promotion`, war and peace
  (`player_update_plunder`), hidden nationality, the crusade Host's war, and each game turn
  (`do_turn`). The duel without moving: `Game::unit_intercept_attack` (`unit/combat.rs`, an
  `intercept` flag of `updateCombat`).
- Lanes: `SeaLane`, `Game::sea_lane`, `Game::sea_lane_known` in `naval.rs`.
- A port under siege: `unit_heal_rate` (`unit/actions.rs`), `city_repair_sentinels`
  (`engagement/siege.rs`).
- Supply ships: `supply_ship_open`, `Game::supply_ship_refused` -> `SupplyShipRefusal`
  (`army/supply.rs`).
- Shore fire: `engagement/shore.rs` (`ShoreShip`, `ShoreShot`, `engage_shore_ships`,
  `engage_shore_fire`), `formation::shore_shot` (the resolver's side, pure), `Setup::shore`,
  `EngagePreview::shore`, `ReportSide::shore`, `LogEvent::ShoreFire`.
- The AI (section 8): `crates/ffh-ai/src/naval.rs` (`patrol`, `patrol_target`, `patrol_wake`,
  `patrol_production`, `way_in_range`).
- The stress harness checks `sea_control_stale` (`ffh_harness::check`). Save format 33: `Plot::sea`,
  `Map::seas`, `Unit::intercepted`, `Unit::shore_fired`, `ReportSide::shore`,
  `LogEvent::ShoreFire`. Data: `sea_names.jsonc` (`SeaNameInfo`: `names`, `lake`), the defines of
  section 11's phase 2.

Choices made where this spec was silent (each open to change):

- **Seas.** The seeds' distances and the plots' assignment step over every water plot of the area
  (impassable ones too: each plot has a sea). An area "touches a map edge" on a plot in the first or
  last column (unless the map wraps in x) or row (unless it wraps in y). A lake area takes a lake
  name while some are left, then a sea name, then "Sea N" (N: the sea's index + 1). The seas are
  cut after the lakes and the map script's own changes, before the starting plots; the normalizer's
  later lakes (inland, one plot) get no sea. A map made by hand (a test world) has no sea until
  `map_generate_seas` or `map_cut_sea` makes one.
- **Strength** is summed in hundredths and rounded down once (a ship at half strength counts),
  `unit_curr_combat_str` with no plot and no opponent: the duel's strength with its damage and
  wear and no terrain. Every comparison (above 0, twice) is on the rounded numbers.
- **Hidden nationality.** The truth names the hidden ship's team (it is part of that team's
  strength, hostile to the teams it is not at war with only through its hidden ships). What a
  viewer works out puts another team's hidden ships into one unknown party (`SeaParty` `None`),
  hostile; a lane cut by it names the barbarian team (as such a ship is shown).
- **Held by you** needs strength above 0 and at least twice each enemy's; with no enemy patrolling
  it is held by you as soon as you patrol. *Patrolled by X* only when neither you nor an enemy
  patrols (the strongest such team; ties: the lower).
- **The stored blockade.** `Plot::blockaded_count` (Civ4's field) is now 0 or 1 for a team: 1 when
  the plot's sea is closed to it. A sea's refresh (its control, its plots, the trade network once)
  runs when a ship starts or ends Patrol (moving, a new activity and death all end it through
  `unit_set_blockading`), when a patrolling ship's damage or promotions change, at war and peace,
  when its nationality changes, at each player's turn start for the seas its ships patrol (wear),
  and for every sea at each game turn (strengths that move with the game's counters). The harness
  checks that it equals a fresh computation.
- **Patrol** may be taken in one's own borders (Civ4 refused it there and ended a blockade when the
  plot changed owner; both are gone). It ends when the ship moves or takes another activity (Civ4's
  rule). Civ4's Sea Patrol order (`MISSION_SEAPATROL`, a sentry for ships) is unchanged and still
  offered; it does nothing for control.
- **Collected blockade gold** (Civ4's `collectBlockadeGold`: gold from the trade routes of cities
  near a blockading ship) is gone with the range rule; the raiding gold of 4.4 point 5 is phase 4's
  (a marked hook in `unit_collect_blockade_gold`).
- **Messages** of a change go to the players of each team a sea closed or opened to, when the team
  has a port on it: `Lanun holds Bay of Ostrel (31 to 0). Vallus is blockaded.` /
  `Bay of Ostrel is no longer closed to you (7 to 0).`
- **The harbour.** A city's seas are the seas of the water plots next to it. Port siege healing is
  the city's own part (`CITY_HEAL_RATE` and the city's heal) at `PORT_SIEGE_HEAL_PERCENT`; the
  unit's own extra heal and the medics count in full, as under any siege (50% of the whole would
  make an open harbour worse than a blockade for a unit with its own heal).
- **Interception.** Each enemy team with ships on Patrol in the sea intercepts once per step, in
  team order, with its strongest ship (sea strength; ties: the lower unit id) that has not
  intercepted this turn and whose odds (`get_combat_odds`, the rules' own) against the plot's best
  defender against it are at least `INTERCEPT_MIN_ODDS`; from anywhere in the sea. "An enemy of
  it": the ship is an enemy of the mover's team, or the mover's head unit is an enemy of the ship's
  (a mover of hidden nationality). The duel is `updateCombat`'s without the advance, the attack's
  moves, a change of groups or the clearing of either side's orders: the ship stays on Patrol, and
  the mover's mission goes on if it lives. It stops once no unit of the moving group is on the
  plot. Barbarian movers are intercepted too (every team is at war with them); barbarian ships do
  not patrol. Every step a group completes counts, the advance after a won attack included.
- **Supply ships.** The lane's home ports are the range field's ports (the team's, its vassals' and
  master's, open borders'). A crusade Host with no city has no range field, so no ship of it is a
  source (before, its lenders' coastal cities counted). An army aboard a ship that is no source is
  still supplied at sea while in range (3.3 reads range only). The known line uses
  `sea_closed_known` and `sea_lane_known`.
- **Lanes** run down the range field: from each start plot the path steps to the lowest-index
  neighbour one step nearer a port, which is the lowest path among the shortest; from land, the
  shortest of its water plots' paths, ties by the lower path.
- **Shore fire.** "A player of that side": a team of the side (the attacker's units' teams; the
  defender's units' and the defended city's). A ship's A is its sea strength x100 (its strength on
  the units' x10 stat scale, in A's hundredths); the target's Dv is its Dv against a hitter with no
  unit terms (`engage_dv_alone`; a Sentinel's, the set-up's). The choice is SHOOT's reach and the
  normal choice without nearby first (the ship has no lane). A shot never takes a unit below 1 HP
  (as every other pre-battle damage), so it kills nobody. The attacker's ships fire first, then the
  defender's, after the Thunderhead Tower's strike and before the battle; each shot draws r from the
  battle's RNG (the game's in the fight, the forecast's in the preview). A ship that fired is marked
  even if its shot found no target. None for a missile's strike or a battle on water.
- **The AI** (section 8): a warship (Role `Naval`, a sea explorer with nothing left to reveal, an
  idle transport that can fight) with no cargo and not worn patrols, by the order of need of
  section 8 (the besieged city's sea where it would hold it, then a sea of our ports where an enemy
  patrols or a warship of an enemy is seen, then the sea of our most populous port), going to the
  nearest plot of the sea outside a city it reaches; a group on Patrol is woken when worn, out of
  range, outmatched two to one by what it sees, when a transport has something to carry, and every 8
  turns. A coastal city builds a warship (`UNITAI_ATTACK_SEA`) while we have fewer warships than two
  per sea of ours an enemy holds or contests plus one per sea we share with a known enemy port (at
  most three). An invasion goes only at a city with a water plot whose lane (as we see it) is not cut
  (`way_in_range`), which fixes phase 1's targets beyond range. Not built: the war-target and
  settler crossings in boats.

## 5. Lanes -- status: built (`Game::sea_lane`; used by 4.6 with any port of the team; 9.3 is phase 4's)

`Game::sea_lane(team, from: PlotIdx) -> Option<SeaLane>`: the shortest path over water (8
directions, passable water, one water area) from `from` (a water plot, or the water plots next to
a land plot) to a water plot next to a home port, where every plot is in the team's range. Ties:
fewer plots, then the lower plot index at the first difference (as supply lines). `SeaLane` has
the path, the port, and `cut_in: Option<(SeaId, TeamId)>`: the first sea along the path, counted
from `from`, that is closed to the team, and who holds it. A lane with `cut_in` is cut; no path
is no lane. Which ports are home ports is said where a lane is used: 4.6 (any port of the team),
9.3 (the owner's cities that are not outposts).

## 6. Summary of what a navy now does to the land war

| | Without ships | With the sea held |
|---|---|---|
| Take a walled port | the city heals and repairs at half rate through the siege | blockaded: no healing, no repair |
| Feed an army overseas | a city must be taken | a transport in a sea not closed to you, with an open lane, is a source |
| Cross water | boats: 1 plot a turn, strength 1 | escorted, or carried at the ships' speed |
| Fight on the coast | | up to 3 ships fire before round 1 |
| Enemy crossing | lands where it likes | a patrol in the sea intercepts it |
| Enemy's coast | | its sea plots unworked, its sea trade cut |

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

### 7.2 Standing orders

A fleet holds one standing order, `Army::station`:

- **None**: under way, or waiting.
- **Patrol** of a named sea: the fleet sails to the nearest plot of that sea it can reach (in
  range) and its warships take the Patrol order there (4.2); it stays until ordered otherwise.
- **Escort** of one of its owner's armies or ships: whenever that group ends a move on a water
  plot the fleet moves to the same plot if it can reach it this turn.

A move order ends the standing order.

## 8. What the AI does

A rules change is not gated on the AI (CLAUDE.md): it does something reasonable with each rule.

- Ships path only through water in range, and go to port when wear is above half its maximum
  (built, phase 1).
- A land army does not use boats on its own initiative, except: to come home when stranded
  overseas (built, phase 1); to reach a war target it has no land path to, when the way is in
  range and crosses no sea closed to it or patrolled by an enemy it sees; a settler or colonist
  bound for a site across water, under the same test.
- Patrol: warships not carrying an invasion patrol, in this order of need: the sea of a coastal
  city one of its armies besieges, when its strength there would hold it; a sea one of its own
  ports stands on where an enemy patrols or has ships; a sea its outposts' lanes pass through;
  else the sea of its most populous port. It leaves a sea where it is outmatched two to one.
- Transports are sent only at targets whose way is in range.
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
  resources reach nobody, it has no trade routes). If cut, each player of the team that holds the
  sea it is cut in who has a ship on Patrol there shares `LANE_RAID_GOLD` (8) gold (the remainder
  to the lower player id).
- Above `LANE_STARVE_TURNS` (5) it is **starved**: it loses 1 population a turn, to 1 at least,
  and it is no port.

Messages: cut (with the sea and its holder), starved, open again; to the raider, the gold.

## 10. What the player sees

Text is bare (CLAUDE.md, "Rules text and tooltips"). Every effect is shown where it bites, naming
the sea and who holds it. The mock-up the owner saw (2026-11-10) is the reference for the Sea
layer, the sea's panel and the Seas screen.

**The Sea layer** (a map layer, on by itself while a ship, a fleet or a unit in boats is
selected; a toggle with the other map layers):

- water out of the player's range is darkened and the range's edge drawn; hovering a port or an
  outpost shows the range it gives;
- each sea the player has revealed a plot of: its border as a dotted line, its name on the
  water, and a badge with its state and the two strengths (`HELD: YOU 24 to 0`,
  `CONTESTED 14 to 17`, `HELD: LANUN 0 to 31`, `OPEN`, `OUT OF RANGE`); its water tinted by state
  (the states differ by pattern and lightness, not hue alone). Enemy strength counts only ships
  the player sees;
- lanes (phase 4; a supply ship's, phase 2), drawn as supply lines are, flowing to the home port,
  in their owner's colour; a cut lane breaks, with a mark, where it enters the closed sea, with
  the turns left before the outpost starves; lanes of others the player has revealed are drawn too;
- a wear pip on a ship that has wear.

**Clicking a sea's name or badge** opens its panel: plots; control and what holding takes
(`Held at twice the enemy's strength on patrol. You need 34.`); each side's strength with its
ships on Patrol (`Ships you do not see are not counted.`); `Effect now` (lanes, ports,
interception); `If X holds it` (each lane of yours that would be cut, each port of yours that
would be blockaded, supply ships); `If you hold it`; and, with a fleet or ship able to go there,
`Send ... to patrol here (3 turns)` with the result it would give (`38 to 17. Held by you.`).

**Always on**: a harbour mark on a coastal city's banner when it is blockaded (and a second state
when besieged and blockaded).

**The path preview** (it already shows turns, odds and supply) adds, for the path under the
pointer:

- `In boats from plot 3: strength 1, 1 plot a turn. 9 turns.`
- `Out of range from plot 6: -15% HP a turn.`
- `Enters The Grey Narrows (contested) at plot 4: will be intercepted by Lanun's patrol.`
- for a Patrol order: the sea's strengths and state after it, and the sea the ship leaves
  (`Bay of Lorda: 0 to 0. Open.`);
- for an attack on a coastal city: `Harbour open: Lanun holds Sirens' Water. The city heals and
  repairs at 50% during the siege.` / `Harbour blockaded.`

**Panels**:

- unit panel, a ship: `Range: in range (5 of 12).` / `Out of range: -15% HP a turn.`;
  `Wear: none (turn 4 of 10).` / `Wear: -15% strength. Enter a port to clear.`; `On Patrol: The
  Grey Narrows.`; a Patrol action; a unit in boats: `In boats: strength 1, cannot attack, does not
  heal.`;
- city panel: `Harbour: open. Bay of Lorda is held by you.` / `Harbour: blockaded. Sirens' Water
  is held by Lanun. Sea plots not worked, no sea trade.`; an outpost: `Lane: open, to Ithralia
  (14 plots).` / `Lane: cut in Sirens' Water, turn 2 of 5.` / `Starved: -1 population a turn, not
  a port.`;
- the army's supply line: `Supplied: at sea, in range of Ithralia.` / `No supply line: the
  transport is in Sirens' Water, held by Lanun.`;
- the battle preview and report: each side's ships that fire, with their shots;
- a **Seas** screen: one row per sea the player has seen: name, control, your strength, the
  enemy's, your ports on it, your lanes through it; a click shows the sea on the map;
- the Armies screen gains a **Fleets** section (phase 3): ships, standing order (a selector:
  none, Patrol with a sea, Escort with its target), wear;
- the Cities list shows outposts as rows marked `outpost`, with the lane's state, and a `Raise to
  city (150 gold)` action in the outpost's panel (phase 4);
- a resource's plot tooltip, when no land area the player has a city on holds that resource:
  `Not on your landmasses.` (phase 4).

**Messages**, on the turn of the change, with the plot: out of range; a unit lost at sea; wear
starts; a sea of yours (one a port or lane of yours is on) changes state (`Lanun holds Sirens'
Water (31 to 0). Vallus is blockaded.`); intercepted (both sides); an outpost's lane cut,
starved, open again; gold from a lane.

## 11. Defines

`rules_defines.jsonc`: `SEA_RANGE` 8, `SEA_ATTRITION_PERCENT` 15, `EMBARKED_MOVES` 1,
`EMBARKED_STRENGTH` 1, `SEA_WEAR_FREE_TURNS` 10, `SEA_WEAR_PERCENT` 5, `SEA_WEAR_MAX_PERCENT` 50,
`SEA_MIN_PLOTS` 6, `SEA_TARGET_PLOTS` 40, `SEA_HOLD_PERCENT` 200, `PORT_SIEGE_HEAL_PERCENT` 50,
`SHORE_FIRE_SHIPS` 3, `INTERCEPT_MIN_ODDS` 50, `LANE_RAID_GOLD_CONTESTED` 4, `FAR_SHORE_MIN_PLOTS`
8, `FAR_SHORE_MANA_PLOTS` 40, `OUTPOST_MAX_POP` 2, `OUTPOST_RAISE_COST` 150, `LANE_RAID_GOLD` 8,
`LANE_STARVE_TURNS` 5. Data: a tech's `sea_range_change`, a civilization's
`embarked_moves_change`, a bonus's `far_shore`, `sea_names.jsonc`.

## 12. Measures

(Each phase adds what its games showed.)
