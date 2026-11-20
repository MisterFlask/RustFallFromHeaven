# Cultural channels

This game's own rules, not FFH2's: the first system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), redesigned on the owner's direction of 2026-10-30, and again
on 2026-11-20, when courting a House became a cultural channel (docs/PERSONS.md, "The fourth
build"). **A civilization may channel half its culture into one standing order until it changes
it: courting a rival's House (Court), guarding one of its own Houses (Guard), or spreading a
religion among a civilization's cities (Spread Religion).**

| Phase (spec) | What | Status |
|---|---|---|
| 1 Baseline | AI autoplay turn times before the sprint | done (below); the owner then ruled turn time out of the acceptance criteria (2026-10-30: "no perf optimization until it impedes testing") |
| 2-3 Culture core and escalation | built as a cultural queue of timed actions (Festival, Trade Fair, Spread Religion, Incite Unrest, Siphon Knowledge, Foment Revolt, Flip City), then replaced (below) | replaced 2026-10-30 |
| Redesign (owner, 2026-10-30) | standing channels of a city: Foment Revolt (unrest, disorder, the city) and Spread Religion, aimed at a civilization; effect by the square of the distance; no festivals, fairs, siphoning | replaced 2026-11-20 |
| Redesign (owner, 2026-11-20) | one standing channel of the civilization: Court and Guard a House (docs/PERSONS.md, "The fourth build"), Spread Religion; Foment Revolt and per-city channels retired | part 1 built (the rules, the AI, the read API: "Rules as built" below); part 2 (the screens) being built |
| 7 FfH layer | Incite Mutiny (needs armies) | later |

Code: rules `crates/ffh-core/src/culture.rs` and `crates/ffh-core/src/persons/` (favour and the
split); data `data/json/cultural_actions.jsonc` (`ffh-data` `infos/culture.rs`), tuning
`data/json/rules_defines.jsonc` (`CULTURE_*`); AI `crates/ffh-ai/src/culture.rs`; UI
`crates/ffh-play/src/view/culture.rs`, `assets/app/culture.js`. Tests:
`crates/ffh-core/tests/culture.rs`.

Every game has cultural channels, the AI-only ones (`ffh autoplay`, `ffh stats`, the AI batches,
`ffh-play agent`, soaks) too. Until 2026-11-05 they were a game option
(`GAMEOPTION_CULTURE_ACTIONS`); the owner removed it, with those of the armies and the governors.

## Rules (2026-11-20)

The contract is docs/PERSONS.md, "The fourth build", rules 1 to 10; in short:

- **The channel** is the civilization's (one that can have Houses): at most one, a standing order;
  while it runs every city diverts `CULTURE_CHANNEL_PERCENT` (50%) of its culture, counted in
  hundredths, and the sum is the channel's inflow. The diverted culture reaches neither the cities'
  culture (levels, borders, the culture victory) nor their plots; a city in disorder diverts
  nothing. A governor never sets a channel.
- **Reach** falls with the distance from the caster's nearest city to the target's nearest city: in
  full within `CULTURE_CHANNEL_FULL_RANGE` (5 plots), then `100·5²/d²` percent (7 plots 51%, 10
  plots 25%, 15 plots 11%), nothing beyond `CULTURE_CHANNEL_RANGE` (20). Distance is Civ4's plot
  distance.
- **Court** a House of another team's realm: the House's favour toward the courter gains the inflow
  times the reach in the liege's turn, loses a guard's, fades `CULTURE_FAVOUR_FADE_PERCENT` (5%);
  at `(L + 6 − 2A, at least 1) × W` (L the House's loyalty to its liege, A its leader's attitude
  toward the courter from −2 to +2, W the world culture unit: the median civilization's culture a
  turn) the House splits toward the courter: the half of its cities nearest the courter (never its
  capital; a House of one city whole) becomes a House of the courter's realm, in peace, the hold
  contest keeping what the liege's armies outnumber. The liege remembers a running Court channel
  (`MEMORY_COURTED_HOUSE`, a point every `CULTURE_COURT_MEMORY_TURNS` turns) and a split
  (`MEMORY_SPLIT_HOUSE`).
- **Guard** one of your own Houses: each courter's favour at it loses the guard's inflow times its
  reach, each turn. Nothing else.
- **Spread Religion**: each turn one roll for the target civilization: the chance is the sum over its
  cities in reach (each at its distance from the caster's nearest city) that lack the religion and
  may take it (a foreign owner whose civics admit only its state religion takes only that one) of
  `CULTURE_SPREAD_PER_CULTURE` (1% per culture of inflow) times the reach divided by one plus the
  number of religions already in that city, at most `CULTURE_SPREAD_MAX` (25%). The religion is one
  that a city of the caster has. On a success one of those cities, picked by its share of the
  chance, gets the religion as from a missionary (FfH's religion hooks run). A spread into a
  civilization with a state religion leaves friendship (its own religion) or hostility (another),
  up to `CULTURE_MEMORY_MAX` (8) points, each fading a point every `CULTURE_MEMORY_DECAY_TURNS`
  (10) turns ("cultural friendship", "cultural hostility" on the diplomacy screen).
- **Visibility.** Every channel is public to the civilizations that have met its caster; its target
  (a House's liege, for Court) is told when it starts and when it stops.

**Retired 2026-11-20:** per-city channels and Foment Revolt (revolt pressure, unrest, revolt, the
flip, a city's resistance and garrison, the revolt levels and marks). They had taken 0 to 14 cities
a game in 12 AI games of 400 turns, nearly all after turn 300 and by one strong cultural
civilization a game; a city alone made too little culture to channel before turn 250.

### Rules as built (2026-11-20, the fourth build's part 1)

docs/PERSONS.md, "The fourth build as built, part 1", has every detail; in short:

- **The channel** is `Player::culture_channel` (`CultureChannel { action, target: ChannelTarget,
  religion, since_turn }`; `ChannelTarget::House` for Court and Guard, `ChannelTarget::Player` for
  Spread Religion), checked by `culture_channel_check` (the refusal in the screen's words) and set
  by `player_set_culture_channel`. Each city diverts `city_culture_diverted_times100` (half its
  culture rate in hundredths, 0 in disorder) in its turn; the turn's sum is
  `Player::culture_inflow`, counted from 0 at the start of the player's city turns, where a channel
  that can no longer go on ends (its caster told why). The previews read the inflow at the cities'
  culture now (`player_culture_inflow_now`).
- **Court, Guard, the requirement, the split** are `persons/favour.rs` (the liege's House turn);
  the grudge (`MEMORY_COURTED_HOUSE` every `CULTURE_COURT_MEMORY_TURNS` turns, up to 8) and Spread
  Religion's roll are the caster's turn, after its cities (`player_do_culture_channel`).
- **Spread Religion** measures each of the target's cities from the caster's nearest city
  (`culture_spread_reach`; a city of the caster itself is at 0, in full).
- **Defines**: `CULTURE_CHANNEL_PERCENT` (50), `CULTURE_CHANNEL_FULL_RANGE` (5),
  `CULTURE_CHANNEL_RANGE` (20), `CULTURE_FAVOUR_FADE_PERCENT` (5), `CULTURE_COURT_MEMORY_TURNS` (5),
  `CULTURE_SPREAD_PER_CULTURE` (100), `CULTURE_SPREAD_MAX` (2500), `CULTURE_MEMORY_DECAY_TURNS`
  (10), `CULTURE_MEMORY_MAX` (8); the `CULTURE_REVOLT_*` defines and
  `CULTURE_AI_MAX_HOSTILE_PER_TARGET` are gone.
- **Data**: `cultural_actions.jsonc` holds `CULTURALACTION_COURT_HOUSE` (effect `COURT`, hostile
  `ALWAYS`), `CULTURALACTION_GUARD_HOUSE` (`GUARD`, `NEVER`) and `CULTURALACTION_SPREAD_RELIGION`
  (`SPREAD_RELIGION`, `DIFFERENT_STATE_RELIGION`); `CULTURALACTION_FOMENT_REVOLT` is gone.
- **Read API**: `culture_channel_preview`, `culture_channel_options`, `culture_channels`,
  `culture_channels_between_others`, `culture_channel_visible`; the courtships and guards
  `house_courtships`, `house_guard_view`, `house_courting_line`.
- **Telemetry**: `culture_channel` / `culture_channel_end` / `culture_spread` by action,
  `house_split` `courted` / `courted_whole` / `courted_held` / `courted_no_leader`; the AI's
  decisions of kind `culture`.

## AI (summary; docs/AI.md "Cultural actions")

Once a turn (`ffh-ai/src/culture.rs`), after the diplomacy and the governors, the civilization's
one channel (the fourth build, rule 9; a rules change: reasonable, not measured):

- **Nothing** while it pursues a cultural victory: its cities keep their culture.
- **Guard** a House of its own whose strongest rival favour is past half its requirement, or would
  reach it within 20 turns at the inflows now (without our guard): the most pressed one. A guard
  of a House still pressed is kept.
- **Court** the House whose cities it would gain are worth most for the time: each city of the
  split (the half nearest us, the cities the hold contest keeps left out) at `150 + 50 a
  population + 20 a building + 150 a wonder + 200 a holy city`, over the turns to the requirement
  (the rules' prediction at our inflow now) plus five, at most 60 turns; scaled by the leader's
  `iCulturalAggression` (halfway from 100%, a conqueror's a quarter higher). Spared: a liege we are
  Pleased or Friendly with, vassals and masters, defensive pact partners, and at peace a liege whose
  defensive power is above 125% of ours (150% for a channel that stands).
- **Spread Religion** as the per-city channels did: the state religion (or one whose shrine we
  hold) into our own cities first, then to friends and neutrals; into a civilization with another
  state religion only where we dislike it and a shrine pays.
- **Against it** the culture spent (each city's half: all of a commerce while its borders still
  grow, half after). A channel that stands is kept while it is worth 70% of the best and of its
  cost; a new one must beat the standing one by a quarter, and its cost by a quarter. A channel
  stands its first 5 turns (a guard of a pressed House overrides it), and a guard is kept while a
  rival favour is past a quarter of the requirement or would reach it within 40 turns: without
  these the lieges and the courters chased each other every turn in the first games.
- **As liege**, a House whose strongest rival favour is past half its requirement is held (the
  hold-march, `courted_to_hold`). **As a House**, it accepts any gift. The AI gives no gifts to
  court.

Retired with Foment Revolt: the revolt simulation, the defenders against revolt pressure (the
governors' and the directly ruled cities'), the provocation of a city close to flipping.

## UI (ffh-play) -- the per-city build of 2026-10-30, retired by the fourth build

The fourth build's part 1 left the client a plain reading of the new channel (docs/PERSONS.md, "The
fourth build as built, part 1", the last paragraph); part 2 builds the screens of rule 10. What
follows is the retired per-city interface, kept for the record.

- **City screen, "Culture"** sits in the left column, under the city's numbers. It shows the culture rate and, under a channel, the half it takes and what is left to the city.
  - **The channel:** action, civilization (and religion), since when, the culture it takes a turn, "Stop", and what it does now.
    - Foment Revolt lists each of their cities in reach, strongest first, with:
      - its distance and the distance effect;
      - the caster's pressure there now;
      - what this channel adds a turn, and what the caster's other cities add, against the city's resistance;
      - a small bar with the pressure now, a marker where it settles, and the thresholds.

      It then says what that brings (no effect, unrest with its unhappiness, revolt, or the city joins you; a capital never) and when ("revolt in ~6 turns").
    - Spread Religion shows the chance a turn (with the 25% cap) and each city's part of it, divided by the religions already there. It also lists the cities that have the religion or cannot take it, and how many cities in reach you have not seen; those count in the chance.
  - **"Channel culture"** is open while the city channels nothing, and folded as "Change the channel" under one.
    - You pick the action, the religion (for a spread), and a civilization with a city in reach.
    - Each civilization has a one-line summary: how many cities and the most the channel would bring, or the chance a turn; "hostile"; or why it is not possible.
    - Below come the preview of the one picked and the button that sets it ("Channel …" / "Switch to …").
    - A city whose half rounds to nothing says so.
  - **"Revolt pressure on this city"** has a bar by civilization, in their colours, against the thresholds: each point of unrest, revolt, two thirds of the way, and the line where one civilization's own pressure takes the city.
    - Then come the level and its unhappiness, and each civilization's pressure with what its channels add a turn, where that settles and when the next level comes (or that it fades, and when it is gone).
    - Last come the city's resistance (half its culture, 2 per defender; in revolt only its defenders) and, close to the flip, what to do.
- **Cultural pressure (F8, the menu's Culture)** lists:
  - your cities under revolt pressure, most pressed first (a click opens the city);
  - the channels aimed at you, with the cities of yours they reach and what they do there;
  - your channels, with what they do and "Stop";
  - the channels you see between others (a click centres the map on the casting city);
  - what other leaders remember of your cultural actions: hostility and friendship, out of `CULTURE_MEMORY_MAX`.
- **Map:** a city's banner ends with a mark while you see it under revolt pressure, on the isometric and 2D maps:
  - a small ochre dot while the pressure has no effect yet;
  - an amber "!" for unrest;
  - a red "!" in revolt;
  - a red "!!" in a gold ring when a civilization's pressure is two thirds of the way to taking the city.
- **Messages** from the rules reach the turn report and the message log like any other, and the located ones have their ⌖. They report:
  - a spread, with its chance a turn;
  - a city's revolt level changing;
  - a hostile channel against you started or stopped;
  - a channel that ended, and why;
  - a city changing hands.
- **Predictions:** "settles near" is where a civilization's pressure ends up after the city's turn under a steady inflow, found by running the turns from the pressure now: about `(inflow − resistance) / 5% − inflow`. "When" counts the same turns.
  - The turns have two resistances. Out of revolt the city resists in full (half its culture and its defenders). From the turn after its total (this civilization's pressure and the others', taken as constant) reaches the revolt, only its defenders resist, since a city in disorder makes no culture; its culture resists again from the turn after the total falls below.
  - So a city whose full resistance would hold the pressure above the revolt loses its culture's part, and the pressure runs on, often to the flip: the prediction says "the city joins you" there, as the rules will.
  - A city in revolt now resists with its defenders both ways: its full resistance cannot be read while it makes no culture.
  - Unrest and the revolt count the city's total, the flip the civilization's own pressure. The chooser's one line names the reached city with the furthest stage (a capital never changes hands).
  - Pressure from civilizations you have not met shows only as "unknown".
- **Protocol:**
  - The command `culture_channel {city, action?, target?, religion?}`; no action means stop.
  - The `culture` query.
  - The city query's `cultural`: `rate`, `amount`, `diverted`, `channel`, `options`, `pressure`, `thresholds`.
  - The view's cities carry `channel` (your own cities) and `revolt` / `unrest` (cities you see under pressure).
  - docs/HUMAN.md has the fields.

## Phase 1 baseline (master `9be40af`, before the sprint)

`ffh bench`, 8 civs, standard map, 400 turns, CPU-exclusive on this container: seed 1 19.2 s
(48.1 ms/turn, slowest 197 ms), seed 2 24.1 s (60.3 ms/turn, slowest 235 ms); no crashes.
