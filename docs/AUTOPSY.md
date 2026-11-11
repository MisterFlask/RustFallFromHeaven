# Autopsy: what goes wrong in AI games, counted

The AI work list starts from what goes wrong in played games, counted and attributed to a cause
(CLAUDE.md, "How AI changes are checked", step 1). The autopsy is the tool for that: the rules
record facts at the moments a game goes wrong (telemetry, docs/TELEMETRY.md), and
`crates/ffh-cli/scripts/autopsy.py` classifies them and counts them over many games.

The rules side records facts only, never a judgement. The classes are computed by the script
from those facts, with their cut-offs as named constants at the top of the script, printed in
the report's header. Nothing names a unit, building, technology or civilization, and no turn is
fixed: strengths are the rules' own, the game's thirds are thirds of its length.

## Running it

```
python3 crates/ffh-cli/scripts/autopsy.py --bin $W --seeds 1-24 --turns 300 --out DIR \
    [--civs 6] [--size WORLDSIZE_STANDARD] [--env FFH_X=1 ...] [--jobs N] [--by civ] [--json FILE]
python3 crates/ffh-cli/scripts/autopsy.py --load DIR [--by civ] [--json FILE]
```

`--bin` is a build output directory (`W=$(tools/agents/build-tree.sh --bins ffh)`) or an `ffh`
binary. Each seed plays `ffh stats` with `FFH_TELEMETRY_FILE`, writing `DIR/seed_N.json` (the stats,
with the telemetry aggregates) and `DIR/seed_N.jsonl` (the events); a seed whose two files exist is
not played again, so a directory grows by adding seeds. Each game holds one machine-wide CPU slot
(docs/AGENTS.md). These runs bypass the run cache, which cannot replay the events file. `--load`
reports from a directory without playing. `--by civ` adds every section per civilization (the
old owner for cities fallen); `--json` writes everything, each fallen city with its facts and class.

## What the rules record

In `crates/ffh-core/src/telemetry_autopsy.rs`, only when telemetry is on.

**`battle`** (event), as each engagement is launched (before summons, defensive strikes and the
battle): the attacker `p`, the defender `vs` (the city's owner, else the first defender's owner),
`x`, `y`, `city`, the plot's owner `owner` and the launch plot's `from_owner` (-1: unowned),
`att_n` / `att_str` (the committed force), `def_n` / `def_str` / `def_full` (the military of teams
at war with the attacker on the plot; `def_full` at full hit points). The `engagement` event that
follows (docs/TELEMETRY.md) has the result and the losses; the script pairs them by turn, player
and plot.

**`city_fell`** (event), a city taken by conquest (`player_acquire_city`), its player the old owner:
- `to` (the captor), `barb` (the captor is the barbarians), `host` / `owner_host` (the captor / the
  old owner is a crusade Host slot, docs/CRUSADE.md), `razed` (razed on capture, else kept);
- the city: `city` (id), `x`, `y`, `pop`, `age` (turns since founded), `held` (turns since its
  owner acquired it), `capital`, `defense` (its total defense percentage), `defense_damage`;
- the defenders: `siege_turn` (the turn of the latest day of engagements against the plot, null
  when none), `def_n`, `def_str`, `def_full` (the military on the plot when that day's first
  engagement was launched), `battles`, `att_n`, `att_str` (that day's engagements and the attackers
  committed in them, summed), `left_in_city` (the old owner's military still on the plot at the
  capture). The capture itself happens after its defenders died, so the defenders are those of the
  day's first engagement; a city taken with no engagement that turn had none at that moment. A
  city whose last defenders died in a duel (ships, air units) rather than an engagement shows the
  last engagement's day or none;
- `captor_near_n`, `captor_near_str`: the captor team's military within 2 plots (the city's plot
  included);
- the old owner's military, each `[count, strength]`, by where it stood, the first that applies:
  `in_city`, `near` (within 5 plots), `other_cities`, `home_field` (its own territory outside
  cities), `abroad`; and `captor_mil` (the captor's whole military);
- `power` / `captor_power` (the rules' player power), `cities` / `captor_cities` (before the
  capture), `wars` (civilizations the old owner's team is at war with), `war_turns` (turns since
  the war between the two teams began, null when telemetry saw no declaration),
  `declared_by_owner`;
- the old owner's economy: `gold`, `commerce_pct` (the commerce sliders), `research` and
  `gold_rate` (its commerce rates), and `producing` (what the city was building).

**`civ_died`** (event), a civilization eliminated (`player_set_alive(p, false)`; not the
barbarians or crusade Hosts): `killer` (the last captor of one of its cities, null when it lost
none by conquest), `lost` and `lost_to` (cities lost by conquest, by captor), `chain` (`[turn,
captor]` for each), `peak_cities` and `peak_turn` (sampled at the end of its turns and at each
loss), `units` (left when it died).

**`army`** (counts, per player): a census of the land military at the end of each of the
player's turns. The key is the war state, `war_` (its team at war with a civilization) or `peace_`,
then the place: `city`, `home` (its territory outside cities), `enemy` (territory of a team it is
at war with), `other`; `_still` counts those that spent no movement that turn. The sum is
unit-turns; the count is the player-turns in that war state.

Military: a land unit on the map that can fight (the rules' `canFight`: some strength).
Strength: the rules' contribution value of a unit (`crusade_unit_strength`, docs/CRUSADE.md): the
better of its attack and defense strengths x100, with its general combat bonus, scaled by its
hit points. The report shows it /100.

Combat odds: duels (ships, air) have the rules' odds in the existing `attack_odds` counts.
Engagements have no single odds; the report buckets attacks by the attacker's share of the
summed strength at launch instead, which is not a probability.

## The classes of a fallen city

The first that applies:

1. `empty`: no military defender when the day's first engagement was launched, or no engagement
   that day.
2. `outclassed`: the old owner's power below `OUTCLASSED_POWER_RATIO` (0.60) of the captor's
   (not used for barbarian captors).
3. `army_away`: the old owner's military beyond 5 plots (in its other cities, in the field at
   home, abroad) had more than `ARMY_AWAY_RATIO` (1.0) times the strength of the captor's units
   within 2 plots: the force existed but was not there. Sub-split by where most of it stood.
4. `thin`: defended, not outclassed, the army not away: too little military anywhere. The report
   gives the owner's military strength relative to the captor's.
5. `other`: facts missing (a file from before a field existed).

The report also gives, per class, the five most typical examples (closest to the class's median
of its defining ratio) with seed, turn, city and plot, to fork with `tools/ai-branch.sh`.

## The rest of the report

- Cities fallen per game and in total, to the barbarians and to civilizations, by third of the
  game. Barbarian cities and crusade Host cities taken are counted apart, not classified.
- Eliminations: when, the peak, and the chain of cities lost with each one's captor and class.
- Army census: unit-turns by place and war state; `still`; `idle@war`: the share of army-turns at
  war spent outside any city and outside enemy territory.
- Losses in engagements: committed and lost attacking and defending, by whose territory the battle
  was on, and attacks by strength share with their losses and how often the plot was taken.

## Naval

The sea game's facts, so the naval work starts from counts (CLAUDE.md, step 1). Recorded in the
second half of `telemetry_autopsy.rs`, reported by `autopsy.py`'s `== Naval` section (`--sections
naval` alone; `--json` has it under `naval`). Nothing names a ship: a sea unit's class is what the
rules say of its type -- `transport` (room for land units), `warship` (some strength, no room),
`other` -- whether it can enter every water terrain (no water terrain impassable to it), and the
unit AI type the AI gave it. Most of the game's ships carry land units, so most are `transport`;
the unit AI type tells what the AI uses them for.

**What the rules record**, only when telemetry is on:

- `sea` (counts, at the end of each player's turn, as the army census): sea units by
  `{war|peace}_{class}_{port|own|sea}` (in a city of its team, in its team's waters, elsewhere),
  `_still` for those that spent no movement; `sea_cargo` (`{war|peace}_{military|settler|other|empty}`:
  what each transport carries); `sea_role` / `sea_role_moved` (by unit AI type; the sum of
  `sea_role` is the unit-turns in port); `sea_reach` (`coast` / `ocean`); at war, `sea_raid`
  (`near_enemy_city`: armed ships next to a city of a team at war with it; `on_enemy_resource`: on a
  water resource of such a team; `blockading`: on the rules' blockade, MISSION_PLUNDER).
- `naval_first` (event): the first end of a turn a player had a sea unit (`ship`), a `warship`, an
  `ocean_ship`, an `ocean_transport`.
- `sea_fight` (event): every duel with a sea unit on either side or on a water plot (duels are
  `tel_combat`; land attacks are engagements and never involve ships): classes (`land` for a land
  unit), unit AI types, the rules' odds as the fight began, `res`, `undefended`, `drowned_att` /
  `drowned_def` (cargo that went down with a sunk ship), `barb_att` / `barb_def`, `animal_def`,
  `water`, `waters` (whose territory: `att`, `def`, `other`, `none`).
- `sea_load` / `sea_unload` (counts): units put aboard by kind (`military`, `settler`, `other`), and
  taken off by kind and where (`port`: a city of its team; `home`: land of a landmass where the owner
  has a city; `landing`: one where it has none; `other`: water), `died_aboard`.
- `unload` (event): each unload onto land outside a port, with the cities of others within 2 plots
  (`[x, y, owner, distance, at war]`).
- `trip` (event): a transport's trip, from the first unit put aboard until it is empty again:
  `start`, `left_port` (the first end of a turn it was seen outside a port with cargo), `kind`,
  `max_cargo`, `outcome` (`landing`, `home`, `port`, `other`; `lost_at_sea` when cargo died aboard;
  `ship_lost` when the ship died with the trip open), `ashore` by where, `died`. `sea_trip.start`
  counts the trips begun (those still open at the end have no `trip` event).
- `sea_pillage` (event, count): a water improvement pillaged (by whom, whose, on a resource);
  `sea_imp_lost` (count, for the owner): an owned water plot's improvement removed, keyed by what the
  rules were doing (the sum: on a resource).
- `ship_built` / `ship_built_reach` (counts by class / reach; the sum is the hammers) and `hammers`
  (`ship`, `unit`, `building`: the production cost of what was finished).
- `geo` (event, once): plots, water plots, land areas, landmasses holding a start, and each
  civilization's start landmass with its size and how many other civilizations started on it.
- `coast` (event, every 10 turns per civilization): cities, coastal cities, cities off its start
  landmass, landmasses it holds cities on.
- Added to existing events: `city_fell` has `coastal`, `captor_near_by_sea` (the captor's units
  within 2 plots that came ashore from a ship outside a port within the last 10 turns) and
  `captor_land_city` (the captor had another city on that landmass); `found` has `area`, `overseas`
  (founded on a landmass where the founder had no city while it had cities elsewhere) and
  `coastal`; `war` has `share_land` (both teams held a city on one landmass).

**The report**: per game (water share, landmasses with a start, isolated civilizations, ships built,
naval fights, military landings, assaults and captures that followed, cities fallen and by sea,
overseas cities founded and kept, wars between civilizations sharing no landmass and how many saw a
fight), then by group -- `isolated` (no other civilization started on its landmass), `shared`, `late`
(entered without a start: a civilization an event brings) -- the final standing, coastal cities at the
thirds of the game, ships built and their share of hammers, the first-ship turns, the sea census,
cargo, trips and their outcomes, landings, cities lost and taken by sea, blockade-like turns; then
the fights themselves, the barbarians' sea units, the civilizations' sea units by unit AI type, and
the wars by shared land. A military landing is the units put ashore on a landmass without a city of
their owner, grouped by player, turn and plot; it is followed by an assault when its player launches
an engagement on a city of a civilization (or the barbarians) it was at war with within
`LANDING_CITY_DIST` (1) plots within `LANDING_ASSAULT_TURNS` (3) turns, by a capture when it takes
that city within `LANDING_CAPTURE_TURNS` (10).

**Limits**: a landing is counted where the cargo steps off; units that walked ashore and on are not
followed further than the `captor_near_by_sea` window. `left_port` is sampled at the end of the
owner's turn, so a transport that loads, sails and lands within one turn shows 0 turns in port. The
hammers are the cost when the item is finished (`hammers.unit` is computed after the new unit counts
toward its class's instance cost). Ships lost in a city that falls are not a `sea_fight`.

### First baseline (2026-11-10; standard AI, 6 civs, standard map)

Seeds 1-16 at 300 turns, the same 16 seeds with `--sea-level SEALEVEL_HIGH`
(`--arg='--sea-level SEALEVEL_HIGH'`), and seeds 1-8 to the end (`--turns 700 --arg=--until-supremacy`;
seed 2 panics at turn 468 in the rules, `city_production_experience` on a city gone, with or without
telemetry, so 7 games). The map script is always Fractal (continent grain 2, 78% water
+ the sea level's change): there is no archipelago or islands script and no option that makes one,
and high sea level only moves water from 80% to 85.5% (landmasses with a start: 2.6 -> 2.9 a game).

The counts that matter:

- **No naval combat.** 0 duels at sea in 16 standard games, 3 at high sea level (land units
  killing ships in port); no transport ever sunk, no unit drowned. The AI's warships (`Role::Naval`
  in `units.rs`: a sea unit with no cargo room and not a sea explorer) `hold` (fortify or sleep) every
  turn; transports and sea explorers never attack. Barbarian ships and sea animals also `hold`: 25,000
  barbarian sea unit-turns, 100% still, in each run. To the end of the game: 6 duels in 7 games (5
  land units attacking ships in port, one transport attacking a transport).
- **Ships are rare and late.** ~1 ship per civilization a game (45 of 130 civs had none at turn 300),
  0.8% of unit and building hammers; the first ship's median turn is 226, the first ship able to
  enter every water terrain 259 (37% of civs).
- **Invasions are a trickle.** Every military trip in the standard run carried one unit (31 of 31;
  41 of 49 at high sea level). 17 military landings in 16 games (25 at high sea level), 14 of them one
  unit; 2 were followed by an assault on the city beside them and 1 by its capture (3 and 3 at high
  sea level). Cities taken with units that came by sea: 1 of 241 (1 of 213). To the end of the game
  (7 games): 45 military landings (126 units), 6 assaults, 4 captures, 34 of the landings by
  civilizations alone on their landmass; 19 of 104 military trips ended with the army unloaded back in port.
- **No blockade, no raiding.** 0 armed-ship-turns blockading, 0 sea improvements pillaged or lost.
  The rules have a blockade (MISSION_PLUNDER: a ship able to pillage blockades the water plots within
  `SHIP_BLOCKADE_RANGE` (3) of its water area for the teams at war with it -- they cannot work them
  and their trade network does not pass them); no AI code issues the mission. (Since replaced by
  the Patrol of named seas: docs/NAVAL.md, 4.2.)
- **Wars across water are phony.** 6 civilization wars between teams sharing no landmass (7 at high
  sea level); 2 had any fight or landing. To the end of the game: 28, of which 9 had a fight or a
  landing.
- **Isolation pays.** Civilizations alone on their start landmass (14 of 130) ended with more
  cities (9.6 vs 8.0), techs (40.3 vs 37.0) and legacy score (1262 vs 997), all alive, at war 98
  player-turns against 4,483 for the others. To the end of the game (9 of 70): techs 60.9 vs 41.8,
  power 1574 vs 777, legacy score 2195 vs 1187, all alive.
