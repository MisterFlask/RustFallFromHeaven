# Cultural actions (`GAMEOPTION_CULTURE_ACTIONS`)

This game's own rules, not FFH2's: the first system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29). Cities spend part of their culture, from a cultural queue,
on actions that help, pressure or resist other cities. The spec was written for the Civ4 DLL;
this file says what was built here, how the spec's open points and edge cases were decided, and
what is left.

| Phase (spec) | What | Status |
|---|---|---|
| 1 Baseline | AI autoplay turn times before the sprint | done (below); the owner then ruled turn time out of the acceptance criteria (2026-10-30: "no perf optimization until it impedes testing") |
| 2 Culture core | queue, channelling, visibility, Festival, Trade Fair, Spread Religion, Incite Unrest, events, memories, AI, UI | done |
| 3 Culture escalation | Siphon Knowledge, Foment Revolt, Flip City, espionage overlap | done |
| 7 FfH layer | Incite Mutiny (needs armies) | later |

Code: rules `crates/ffh-core/src/culture.rs` (hooks in the city turn, happiness, trade routes,
`city_kill`, the player turn); data `data/xml/Gameinfo/CIV4CulturalActionInfos.xml`
(`ffh-data` `infos/culture.rs`), tuning `data/xml/RulesDefines.xml`; AI
`crates/ffh-ai/src/culture.rs`; UI `crates/ffh-play/src/view/culture.rs`, the city query's
`cultural`, `assets/app/culture.js`. Tests: `crates/ffh-core/tests/culture.rs`.

## The option

`GAMEOPTION_CULTURE_ACTIONS` is a visible game option with `bDefault` 1: the browser's new-game
screen starts with it ticked (`setup::screen_defaults`), and a player can untick it to play the
rules as they were. Everything else (`ffh autoplay`, `ffh stats`, the AI batches, `ffh-play
agent`, soaks) plays without it unless asked (`--option GAMEOPTION_CULTURE_ACTIONS`), so the AI
measurements of docs/AI.md keep their baselines. With the option off nothing of this runs: queues
stay empty, the save carries empty lists.

## Rules

**The cultural queue.** Each city has one (`City::culture_queue`, at most `CULTURE_QUEUE_MAX`
orders), next to its production queue. Its first order is the *channel*. An empty queue is the
default and needs no attention: the culture grows the city's borders as before. An order may
repeat (queued again at the end when it completes) and may be put first: the interrupted channel
waits with its progress and resumes when it is first again (it is not cancelled).

**Cost: a rate for a duration.** An action has a rate R (culture a turn) and a duration D (turns
at normal speed, scaled by the game speed's `iTrainPercent` like build times; the effects'
durations scale the same way, so an action costs and gives the same in proportion at every
speed). A city may queue an action only while its culture rate is at least R. Each turn of the
channel the city diverts R (`City::culture_diverted`) before its culture is added: the diverted
culture reaches neither the city's culture (its level, the culture victory) nor its plots. A
channel whose city makes less than R, or whose target stops qualifying for a while (out of range,
share or effectiveness too low, a partner no longer allied, a duplicate channel elsewhere),
*pauses*: no progress, nothing diverted. After `CULTURE_CHANNEL_STALL_TURNS` paused turns in a
row it is cancelled. A channel whose target cannot qualify again is cancelled at once: the target
city is gone (destroyed or captured: a captured city is a new city), the target is now of the
wrong kind (a foreign city that became ours), a religion condition failed (the target got the
religion another way). Cancelling refunds nothing; effects resolve only when a channel completes.

**Tiers and reach.** Tier 1 actions need no culture level, tier 2 `CULTURE_TIER2_LEVEL`
(Influential), tier 3 `CULTURE_TIER3_LEVEL` (Legendary). The target must be within the tier's
range of the casting city (`CULTURE_RANGE_T1..3`: 6/9/12 plots, Civ4's plot distance), also for
our own cities (a city festivals a neighbour within reach). A foreign target also needs the
caster's share of the culture on the city's plot (its nationality, as the city screen shows it)
to be at least `CULTURE_MIN_SHARE_T1..3` (10/25/50%). Targets of the caster's own team need no
share.

**Effectiveness.** On a foreign city, an action's magnitude or chance is multiplied by
E = Sc / (Sc + K·So + M): the caster's and the owner's shares of the plot's culture, K =
`CULTURE_RESIST_K` (100 = 1.0), M the city's defences. Integer percent here. M counts only
against *hostile* actions: a festival in the city adds `CULTURE_FESTIVAL_RESIST` (0.5, once
however many festivals), and each land unit of the owner's team able to defend on the city plot
adds `CULTURE_GARRISON_RESIST` (0.05) against the actions marked `bGarrisonResists` (Incite
Unrest, Foment Revolt, Flip City). An action below `CULTURE_MIN_EFFECT`
(15%) cannot be queued (and pauses if it falls below while channelling). Actions on our own team's
cities use E = 100%; Festival is never scaled.

**Who may be targeted.** A foreign target's owner must have been met (or be the barbarians) and
the city revealed to the caster's team. Hostile actions need no war (only Incite Mutiny will).
The barbarians cast nothing. One civilization channels an identical order (action, target,
religion) from one city at a time; a queue cannot hold the same order twice.

**Visibility.** Every channel is public to the civilizations that have met its caster
(`Game::culture_channels`, `culture_channel_visible`): casting city, action, target, progress.
The target's owner is told when a hostile channel starts and when it completes or is given up;
the caster when its channel completes or is cancelled (game log messages, at the target city).

**Memory.** When an action on another team's city completes, the target's owner remembers it
(`Player::culture_memory[caster]`: hostility for a hostile action, friendship for a fair, a
festival for an ally, or a spread of its own state religion), by the action's `iMemoryWeight`, up
to `CULTURE_MEMORY_MAX`; every `CULTURE_MEMORY_DECAY_TURNS` turns each memory loses a point. The
AI's attitude reads them (docs/AI.md, "Cultural actions"), and the diplomacy screen lists them
among an attitude's reasons ("cultural hostility", "cultural friendship").

**Events.** `Game::culture_event` runs on every start, completion (with success and magnitude) and
cancellation: the messages, the memory, telemetry (`culture_start`, `culture_done`,
`culture_cancel` counts; `culture` events). It is the place a later system (a threat score,
warscore) subscribes. `iThreatWeight` is in the data for that system; nothing reads it yet.

### The actions

| Action | Tier | Target | Hostile | R / D | On completion |
|---|---|---|---|---|---|
| Festival | 1 | own city, or a city of a team we have a defensive pact with (a permanent alliance is one team) | no | 3 / 3 | for `CULTURE_FESTIVAL_TURNS` (5): +`CULTURE_FESTIVAL_HAPPY` (1) happiness and +0.5 to M; not scaled by E; festivals do not stack, a new one refreshes |
| Trade Fair | 1 | foreign city of a civilization we can trade with (no war; open borders: `canHaveTradeRoutesWith`) | no | 3 / 4 | for `CULTURE_TRADE_FAIR_TURNS` (10): trade routes between that city and the caster's cities yield +`CULTURE_TRADE_FAIR_PCT`×E (50%×E) for both sides (a trade-route modifier, so it also draws routes) |
| Spread Religion | 1 | any city lacking the religion; the religion must be in the casting city; a foreign owner whose civics admit only its state religion only takes that one | only when the target's owner has another state religion | 3 / 4 | a chance `CULTURE_SPREAD_BASE`×E (80%×E; E = 100% at home) to spread it, through the rules' religion spread (FfH's hooks fire as for a missionary) |
| Incite Unrest | 1 | foreign city | yes | 4 / 4 | +`CULTURE_UNREST_UNHAPPY`×E unhappiness (3×E, rounded, at least 1) for `CULTURE_UNREST_TURNS` (5); one mark per caster (a new one refreshes), casters add up |
| Siphon Knowledge | 1 | foreign city whose owner's team knows a tech the caster's lacks and the caster could research | yes (minor: memory 1) | 5 / 5 | research worth `CULTURE_SIPHON_PCT_OF_COST`×E (15%×E) of that tech's cost: the caster's current research if eligible, else the cheapest eligible (ties: the first); the target loses `CULTURE_SIPHON_TARGET_LOSS_PCT` (0%: diffusion) of it from its current research, never below nothing |
| Foment Revolt | 2 | foreign city | yes | 10 / 6 | a chance `CULTURE_REVOLT_BASE`×E (60%×E): disorder for `CULTURE_REVOLT_TURNS` (2; the occupation timer, as a culture revolt) and each defender of the city's team loses `CULTURE_REVOLT_DAMAGE` (30%) of its hit points, never its last; the revolt's turn is recorded on the city (`City::culture_revolt_turn`) |
| Flip City | 3 | foreign city, not a capital; not with `GAMEOPTION_NO_CITY_FLIPPING` | yes | 40 / 10 | needs, when queued and when the channel starts, a successful Foment Revolt there within `CULTURE_FLIP_REVOLT_WINDOW` (10) turns or the city in disorder; a chance `CULTURE_FLIP_BASE`×E (50%×E) that the city joins the caster as in a culture revolt that flips it (barbarian units on the plot move away, defenders lose half their hit points, the plot and city change hands; in a one-city challenge a human caster's flip destroys the city); a failed roll changes nothing else |

Timed effects live on the target city (`City::culture_marks`) and count down in its turn.

## Edge cases decided (beyond the spec's text)

- **Share is the plot's nationality.** The spec says "the caster civ's nationality percentage in
  the target city ... as the city screen shows it", which is the culture on the city's plot (C++
  `CvPlot::calculateCulturePercent`), not the city's own culture table (which holds only its
  owners' culture). Built that way.
- **M only against hostile actions.** The spec puts a festival into M without saying against
  what; a festival resisting an ally's trade fair would be absurd, so the defences count only
  against actions hostile to the city's owner.
- **Range for own cities, share only abroad.** The range keeps actions local everywhere; a share
  gate on one's own city would stop a conquered city from defending itself.
- **Effect durations scale with the game speed** like D (see Cost).
- **Pause or cancel.** Anything that can pass by itself pauses (and the stall timer ends it);
  what cannot comes back cancels at once (listed under Cost). A captured caster city loses its
  queue with a cancellation (the capturer's new city starts empty).
- **Put first** keeps the interrupted channel's progress (it waits; no cancellation, no new
  start event), so an AI or a player can answer a threat with a festival without wasting a channel.
- **Barbarian cities** may be targeted (they are foreign); barbarians cast nothing.
- **Espionage overlap.** FfH locks `GAMEOPTION_NO_ESPIONAGE` on (`Game::is_option`), so BtS's
  influence missions cannot run in this game: there is nothing to disable.
- **Flip City's prerequisite is checked when the channel starts, not every turn.** The spec asks
  for a revolt "within the last 10 turns" and a 10-turn channel: checked every turn, a flip queued
  a turn after the revolt could never finish. Here the order may only be queued, and only becomes
  the channel, while the prerequisite holds (a waiting flip pauses until it does, and the stall
  ends it); once started it runs to its roll. So a flip still cannot complete without its
  prerequisite having held.
- **Flip City and FFH2's flip options.** `GAMEOPTION_NO_CITY_FLIPPING` (no culture flips) also
  forbids Flip City. `GAMEOPTION_FLIPPING_AFTER_CONQUEST`'s rule (a conquered city does not flip
  back to its old owner) does not apply: retaking a city that culturally is still yours is what the
  action is for (the spec: "recently conquered cities rank first").
- **Siphon Knowledge's target loss** comes out of the target's current research (the spec's "taken
  from the target's research"), never below nothing; at the default 0% it is diffusion.

## Open points for the owner

- **Tiers 2 and 3 are rarely within an AI's reach.** Foment Revolt needs an Influential city
  making 10 culture a turn and a 25% share abroad; Flip City a Legendary city making 40 and a 50%
  share. In three 8-civ, 450-turn AI games (seeds 1-3, phase 3) Siphon Knowledge completed 65-242
  times a game, Foment Revolt 0-10 (3 revolts), Flip City never. Lower rates or culture levels for
  tiers 2-3 would bring the escalation into ordinary games; as specified they are late, rare
  weapons.

- **Trade Fair is weak at the spec's numbers.** It is multiplied by E (the spec notes no
  exception), and E abroad is usually 15-40%, so a fair lifts the linked routes by 8-20%: a 4-commerce
  route gains well under a commerce a turn for 12 culture. The AI holds one where routes link the
  two cities and the partner is Pleased or Friendly (the goodwill counts), about 15-80 a game in
  8-civ games. Exempting it from E, or raising `CULTURE_TRADE_FAIR_PCT`, would make it a real choice.
- **`CULTURE_MIN_EFFECT` overrides `CULTURE_MIN_SHARE_T1`.** With two cultures on a plot E ≈ Sc,
  so the 15% minimum effect is the binding gate for tier 1 and the 10% minimum share never decides.

## AI (summary; docs/AI.md "Cultural actions")

Phase 3 adds: siphoning knowledge from anyone but a Pleased or Friendly civilization; fomenting
revolt in a rival's city (its output for the disorder, the hurt garrison, a third of the city's
worth when one of our cities could then flip it); flipping a rival's city after a revolt (its worth
times the chance; at peace only when our power is at least 80% of their defence). Against a revolt
or a flip aimed at a city of ours, it also trains a defender first while the city has fewer than
three; and a civilization channelling Flip City against us provokes the war logic (no friend
spared, 85% of the usual power edge, a higher target score).


Once a turn (`ffh-ai/src/culture.rs`): festivals against every hostile channel aimed at our or a
partner's city (the city itself puts one first, else an idle neighbour in reach), then each idle
city queues its best action worth its culture: a trade fair with a Pleased or Friendly
civilization, spreading the state religion at home and (once its shrine pays) abroad, inciting
unrest in a rival's cities (at war, or Annoyed or Furious; at most
`CULTURE_AI_MAX_HOSTILE_PER_TARGET` channels against one civilization), weighed by the leader's
`iCulturalAggression` (derived from war eagerness and culture flavor when the XML has none).
Culture counts in full while the city's borders still grow to its working radius, half after; the
cities carrying a cultural victory the AI pursues keep their culture. Switch `FFH_AI_CULTURE_OFF`.

In three 8-civ, standard-map, 450-turn games (seeds 1-3, all AI) every action was used: per game
86-170 festivals, 151-225 incited unrests, 26-59 religion spreads and 15-68 trade fairs completed.

## UI (ffh-play)

- **City screen**, section "Culture": the city's culture rate and what the channel took last
  turn; the queue with each order's progress, why it pauses, a repeat toggle and ✕ (cancel); the
  hostile channels aimed at the city; its festivals, fairs and unrest; "Start a cultural action":
  for each action its tier, rate, duration and reach, the targets in reach with the predicted
  effectiveness and effect (hostile ones marked), and the ones not possible with the reason. Click
  queues, Shift puts first, Alt repeats.
- **Cultural pressure (F8, the menu's Culture button):** every channel aimed at our cities, every
  channel we run, the others we see, and what other leaders remember of our actions.
- **Map:** a red "!" on the banner of a city with hostile channels aimed at it (isometric map; "!"
  after the name on the 2D map).
- Protocol: commands `culture_push {city, action, target, religion?, repeat?, first?}`,
  `culture_pop {city, index}`, `culture_repeat {city, index, repeat}`; the `culture` query; the
  `city` query's `cultural`; the view's cities carry `channel` (own), `pressure`, `festival`,
  `unrest`. Help: the actions' XML help through the `data` tables (`CulturalActionType`).

## Phase 1 baseline (master `9be40af`, before the sprint)

`ffh bench`, 8 civs, standard map, 400 turns, CPU-exclusive on this container: seed 1 19.2 s
(48.1 ms/turn, slowest 197 ms), seed 2 24.1 s (60.3 ms/turn, slowest 235 ms); no crashes.
