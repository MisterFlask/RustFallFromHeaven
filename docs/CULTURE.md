# Cultural channels (`GAMEOPTION_CULTURE_ACTIONS`)

This game's own rules, not FFH2's: the first system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), redesigned on the owner's direction of 2026-10-30. A city may
channel half its culture into one standing order aimed at a civilization, until its owner changes
it: fomenting revolt among its people, or spreading a religion among its cities.

| Phase (spec) | What | Status |
|---|---|---|
| 1 Baseline | AI autoplay turn times before the sprint | done (below); the owner then ruled turn time out of the acceptance criteria (2026-10-30: "no perf optimization until it impedes testing") |
| 2-3 Culture core and escalation | built as a cultural queue of timed actions (Festival, Trade Fair, Spread Religion, Incite Unrest, Siphon Knowledge, Foment Revolt, Flip City), then replaced (below) | replaced 2026-10-30 |
| Redesign (owner, 2026-10-30) | standing channels: Foment Revolt (unrest, disorder, the city) and Spread Religion, aimed at a civilization; effect by the square of the distance; no festivals, fairs, siphoning | this file |
| 7 FfH layer | Incite Mutiny (needs armies) | later |

Code: rules `crates/ffh-core/src/culture.rs` (hooks in the city turn, unhappiness, `city_kill`,
the player turn); data `data/xml/Gameinfo/CIV4CulturalActionInfos.xml` (`ffh-data`
`infos/culture.rs`), tuning `data/xml/RulesDefines.xml` (`CULTURE_*`); AI
`crates/ffh-ai/src/culture.rs`; UI `crates/ffh-play/src/view/culture.rs`, the city query's
`cultural`, `assets/app/culture.js`. Tests: `crates/ffh-core/tests/culture.rs`.

## The option

`GAMEOPTION_CULTURE_ACTIONS` is a visible game option with `bDefault` 1: the browser's new-game
screen starts with it ticked (`setup::screen_defaults`), and a player can untick it to play the
rules as they were. Everything else (`ffh autoplay`, `ffh stats`, the AI batches, `ffh-play
agent`, soaks) plays without it unless asked (`--option GAMEOPTION_CULTURE_ACTIONS`). With the
option off nothing of this runs.

## Rules

**The channel.** A city has at most one channel (`City::culture_channel`): an action, a target
civilization and, for Spread Religion, a religion of the city. Each turn, before its culture is
added, the city diverts `CULTURE_CHANNEL_PERCENT` (50%) of its culture rate, rounded down
(`City::culture_diverted`); that culture reaches neither the city's culture (its level, the
culture victory) nor its plots. The channel stands until its owner changes it, or until it has
nothing left to do: its target is gone, none of the target's cities is within reach (or, for a
spread, could take the religion), the city lost the religion. A turn in which the city makes no
culture (in disorder) leaves it standing. The owner is told why it ended. A channel is always the
owner's decision: a governor never sets, changes or ends one (the owner's decision, 2026-10-30;
docs/GOVERNORS.md).

**Reach and distance.** The effect on each of the target's cities falls with the square of the
distance from the channelling city: in full within `CULTURE_CHANNEL_FULL_RANGE` (5 plots), then
`100·5²/d²` percent (7 plots 51%, 10 plots 25%, 15 plots 11%), nothing beyond
`CULTURE_CHANNEL_RANGE` (20 plots). Distance is Civ4's plot distance.

**Targets.** Any civilization the caster's team has met, and the barbarians. Foment Revolt only
another team's; Spread Religion any, the caster's own included. No war is needed, and no channel
is an act of war.

**Foment Revolt.** Each turn of the channelling city, every target city in reach gains revolt
pressure from the caster (`City::culture_pressure[caster]`, in hundredths of culture) equal to the
diverted culture times the distance effect. In its own turn a city resists: each caster's pressure
loses the city's resistance (`CULTURE_REVOLT_RESIST_PERCENT`, 50%, of the city's own culture rate,
plus `CULTURE_REVOLT_GARRISON`, 2, per land unit of its team able to defend it) and fades by
`CULTURE_REVOLT_FADE_PERCENT` (5%, at least a culture a turn). Under a steady inflow the pressure
settles near `(inflow - resistance) / 5%`, twenty times the net culture poured in a turn. What the
pressure does (all casters' together, except the flip):
- **Unrest:** a point of unhappiness per `CULTURE_REVOLT_UNHAPPY_STEP` (25), at most
  `CULTURE_REVOLT_UNHAPPY_MAX` (4).
- **Revolt:** at `CULTURE_REVOLT_DISORDER` (100) the city is in revolt: disorder (the occupation
  timer, as a culture revolt) for as long as the pressure stays there. The disorder holds through
  the city's whole turn (the pressure step runs before the occupation timer counts down), so in
  revolt the city makes no culture and resists with its garrison alone; it ends the turn the
  pressure falls below the threshold. A city that would settle just above the threshold with its
  culture resisting therefore runs on, usually to the flip.
- **Flip:** a caster whose own pressure reaches `CULTURE_REVOLT_FLIP` (300) takes the city, as a
  culture revolt flips a city (barbarian units on the plot move away, defenders lose half their hit
  points, the plot and the city change hands; in a one-city challenge a human caster's flip
  destroys it). Never a capital; not with `GAMEOPTION_NO_CITY_FLIPPING`.
The city's owner and the leading caster are told when the city's state changes: unrest, revolt,
close to flipping (two thirds of the way; never for a city that cannot flip), calm again, flipped
(`City::culture_revolt_level`).
Pressure from a caster that dies or joins the city's team goes.

**Spread Religion.** Each turn of the channelling city, one roll for the target civilization: the
chance is the sum over its cities in reach that lack the religion and may take it (a foreign owner
whose civics admit only its state religion takes only that one) of `CULTURE_SPREAD_PER_CULTURE`
(1% per culture diverted) times the distance effect divided by one plus the number of religions
already in that city, at most `CULTURE_SPREAD_MAX` (25%). On a success one of those cities, picked
by its share of the chance, gets the religion as from a missionary (FfH's religion hooks run).
The caster and the target are told where it spread and the chance a turn; the city screen shows
the chance and each city's part of it.

**Memory.** The target's leader remembers a hostile channel: a point of cultural hostility every
`CULTURE_REVOLT_MEMORY_TURNS` (5) turns it runs; a spread into a civilization with a state
religion leaves friendship (its own religion) or hostility (another). Up to `CULTURE_MEMORY_MAX`
(8) points; every `CULTURE_MEMORY_DECAY_TURNS` (10) turns each memory loses a point. The AI's
attitude reads them, and the diplomacy screen lists them ("cultural hostility", "cultural
friendship").

**Visibility.** Every channel is public to the civilizations that have met its caster
(`Game::culture_channels`, `culture_channel_visible`). The target is told when a hostile channel
starts and when it stops.

## Edge cases decided

- **Half, rounded down.** A city making 1 culture channels nothing: the channel stands but does
  nothing that turn.
- **Pressure is per caster**, so two casters each fight the city's full resistance (allies do not
  pool against it), while unrest and revolt count every caster's pressure together; the flip goes to
  the one whose own pressure reached the threshold.
- **The flip happens in the target city's turn**, as a culture revolt would; a flipped city starts
  with no pressure (it is a new city).
- **Barbarian cities** may be targeted (and flipped); barbarians channel nothing.
- **Espionage overlap.** FfH locks `GAMEOPTION_NO_ESPIONAGE` on (`Game::is_option`), so BtS's
  influence missions cannot run in this game: there is nothing to disable.
- **Saves before format 8** (the cultural queue) do not load.

## Open points for the owner

- **The numbers are first guesses** (the `CULTURE_*` defines): a city channelling 10 culture 7
  plots from a rival town without culture or garrison settles its pressure near 100 (revolt); a
  flip needs about 15 net culture a turn on the city, a great cultural city near the border or
  several channels.

## AI (summary; docs/AI.md "Cultural actions")

Once a turn (`ffh-ai/src/culture.rs`), after the diplomacy and the governors:

- **Defence.** Each defender resists 2 a turn, so the garrison answers revolt pressure.
  - A city needs enough defenders that the pressure from the channels aimed at us, as it would settle, stays short of disorder (at most six).
  - Past disorder the city makes no culture, only its garrison resists, and the pressure runs on to the flip.
  - A directly ruled city trains a defender first; in a governed city, the governor chooses again.
  - A rival whose pressure is two thirds of the way to taking one of our cities (not a capital) provokes the war logic: no friend is spared, 85% of the usual power edge is enough, and the target gets +40 to its score.
- **Foment Revolt** comes from cities whose culture is worth spending.
  - Cities carrying a cultural victory spend none. A point of culture counts in full while the city's borders still grow, half after.
  - For each target city, the pressure is simulated over 40 turns as the rules run it. Resistance counts the city's culture and only its stationed defenders (an army passing through does not stay); once the city is in revolt, only the garrison.
  - A city taken (never a capital) is worth 150 + 50 per population + 20 per building + 150 per wonder + 200 per holy city, spread over the turns it takes plus five. Disorder is worth the city's production and commerce a turn; unrest, half a commerce a point. Only what our channels change counts.
  - The rival's losses count in full at war, while preparing one, against the barbarians, or against a victory rival. At peace they count 60% (Annoyed or Furious) or 40% (Cautious), and a city taken counts 75%.
  - Against that stands the culture spent and, at peace, a memory cost per channel a turn, by what the target thinks of us: 2.5 while it is Cautious or better, 1 when Annoyed, 0.5 when Furious or while we prepare war.
  - Spared: Pleased or Friendly civilizations, vassals and masters, defensive pact partners, and, at peace, any civilization above 125% of our power (150% for a channel that stands).
  - The leader's `iCulturalAggression` scales the value halfway from 100%; a conqueror's goes a quarter higher.
  - Our cities aimed at one civilization are weighed as a group of at most `CULTURE_AI_MAX_HOSTILE_PER_TARGET`, twice that at war.
- **Spread Religion.** The state religion, or one whose shrine we hold, goes first into our own cities, then to friends and neutrals.
  - A civilization without a state religion is drawn toward ours, and a co-religionist remembers the spread as friendship.
  - It goes into a civilization with another state religion only where we dislike it and a shrine pays.
- **Stability.**
  - A channel that stands is kept while it is worth 70% of its cost.
  - A new one must be worth a quarter more than its cost.
  - A religion channel moves only for one better by half its cost.
- **Governed cities** channel as the civilization decides, like directly ruled ones. A human's governors set no channel; they only train defenders against revolt pressure.
- **Switch `FFH_AI_CULTURE_OFF`:** nothing is channelled.

Measured on 8 civs, a standard map and 350 turns, with `GAMEOPTION_CULTURE_ACTIONS`, `GAMEOPTION_ARMIES` and `GAMEOPTION_GOVERNORS`, seeds 1–4:

| Seed | Revolt channels | Religion channels | Spreads | Cities in revolt | Cities taken | Defenders trained first |
|---|---|---|---|---|---|---|
| 1 | 11 | 17 | 12 | 1 | 0 | 0 |
| 2 | 31 | 14 | 8 | 6 | 3 | 11 |
| 3 | 28 | 19 | 16 | 7 | 4 | 11 |
| 4 | 7 | 27 | 20 | 2 | 0 | 2 |

- Nearly all spreads went into the casters' own cities, and all but one city taken fell after turn 300.
- Checked stress games (400 turns, seeds 1–3) had no invariant failure.
- Revolt channels begin around turns 200–290, once cities make 20 culture and more. A city in revolt makes no culture, which speeds its fall and weakens its own channels.

## UI (ffh-play)

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
    - Last come the city's resistance (half its culture, 2 per defender) and, close to the flip, what to do.
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
- **Predictions:** "settles near" is where a civilization's pressure ends up after the city's turn under a steady inflow, found by running the turns: about `(inflow − resistance) / 5% − inflow`. "When" counts the same turns from the pressure now. Pressure from civilizations you have not met shows only as "unknown".
- **Protocol:**
  - The command `culture_channel {city, action?, target?, religion?}`; no action means stop.
  - The `culture` query.
  - The city query's `cultural`: `rate`, `amount`, `diverted`, `channel`, `options`, `pressure`, `thresholds`.
  - The view's cities carry `channel` (your own cities) and `revolt` / `unrest` (cities you see under pressure).
  - docs/HUMAN.md has the fields.

## Phase 1 baseline (master `9be40af`, before the sprint)

`ffh bench`, 8 civs, standard map, 400 turns, CPU-exclusive on this container: seed 1 19.2 s
(48.1 ms/turn, slowest 197 ms), seed 2 24.1 s (60.3 ms/turn, slowest 235 ms); no crashes.
