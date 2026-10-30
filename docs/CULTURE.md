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
spread, could take the religion), the city lost the religion. The owner is told why it ended.

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
  timer, as a culture revolt) for as long as the pressure stays there.
- **Flip:** a caster whose own pressure reaches `CULTURE_REVOLT_FLIP` (300) takes the city, as a
  culture revolt flips a city (barbarian units on the plot move away, defenders lose half their hit
  points, the plot and the city change hands; in a one-city challenge a human caster's flip
  destroys it). Never a capital; not with `GAMEOPTION_NO_CITY_FLIPPING`.
The city's owner and the leading caster are told when the city's state changes: unrest, revolt,
close to flipping (two thirds of the way), calm again, flipped (`City::culture_revolt_level`).
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

## Phase 1 baseline (master `9be40af`, before the sprint)

`ffh bench`, 8 civs, standard map, 400 turns, CPU-exclusive on this container: seed 1 19.2 s
(48.1 ms/turn, slowest 197 ms), seed 2 24.1 s (60.3 ms/turn, slowest 235 ms); no crashes.
