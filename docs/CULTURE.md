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
| Redesign (owner, 2026-11-20) | one standing channel of the civilization: Court and Guard a House (docs/PERSONS.md, "The fourth build"), Spread Religion; Foment Revolt and per-city channels retired | part 1 built (the rules, the AI, the read API: "Rules as built" below); part 2A built (the Culture screen and the city screen's line: "UI" below); part 2B (the Houses' side: the House card, the trade table, the diplomacy list) being built |
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
  `house_courtships`, `house_guard_view`, `house_courting_line`. Part 2A added
  `culture_channel_refusal` (why a civilization may set no channel at all: the check's first
  part), `player_culture_channel_base_now` (the culture of the cities that would divert: the
  screen's `half of`) and `Requirement::formula` (`(3 + 6 − 2 × 1) × 24 = 168`, with the
  constants `REQUIREMENT_BASE` and `REQUIREMENT_ATTITUDE` the requirement reads).
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

## UI (ffh-play) -- the fourth build's part 2A (2026-11-20)

The civilization's channel has one screen, **Culture** (F8, the menu's Culture), and one line on
each city screen (docs/PERSONS.md, "The fourth build", rule 10, and "as built, part 2A";
docs/HUMAN.md has the screens and the protocol). Every number is the rules' read API's
(`culture_channel_preview`, `culture_channel_options`, `house_courtships`, `house_guard_with`,
`house_requirement`, `player_culture_inflow_now`, `player_culture_channel_base_now`): the view
(`crates/ffh-play/src/view/culture.rs`) sends the parts as fields, the client
(`assets/app/culture.js`) formats and lays them out, with no formula of its own. Culture shows
with one decimal, favour and requirements as whole numbers, as the rules' lines write them.

- **The head.** `Channel: Court House Aldric, since turn 140.` or `Channel: none. Your cities keep
  all their culture.`; then `Inflow: 6.4 culture a turn (half of 12.8).` or, without a channel, `A
  channel would take 6.4 culture a turn (half of 12.8).` (the "half of" is the culture of the
  cities that would divert: those not in disorder). A civilization that cannot channel (a House,
  one without a city) reads the rules' reason instead.
- **Your channel**, while one runs: its full preview and **Stop**.
  - Court: a favour bar (the favour against the requirement, a mark at two thirds); the
    requirement's parts as a table (`Loyalty of House Aldric to the Bannor: 3`, `Attitude of its
    leader toward you: Pleased (+1)`, `World culture unit: 24`, `Requirement: (3 + 6 − 2 × 1) × 24 =
    168`, the formula written by `Requirement::formula`); the reach (`7 plots: 51%`), the gain and
    the guard a turn, where the favour settles, the split (`In ~31 turns.`, `Never at these
    inflows.`); the cities that would go and those the hold contest keeps, as city links (`It
    joins whole.` for a House of one city).
  - Guard: what it takes from each courtship a turn (`Guard: −3.2 a turn to each courtship.`), its
    reach, and a row per courter: its favour of its requirement, where it settles and its split,
    with the guard and without it.
  - Spread Religion: the chance a turn and its cities in reach (city, distance, share; `has it`,
    `cannot take it`).
- **Choose a channel.** The three actions (Spread Religion with the religions your cities have);
  the targets of the picked one from `culture_channel_options`, each a button with its label and
  one line (a refused one disabled, with the rules' refusal; your channel's `Your channel.`); a
  picked target's full preview opens under it with **Set** (`Set (replaces Court House Aldric)`
  while a channel runs). An action without a target says why in one line (`No House of another
  realm that you have met.`, `You have no House.`, `None of your cities has a religion.`). Spread
  Religion's targets are every civilization you have met with a city, a House among them by its
  label (`House Aldric (Bannor)`), yours `Bannor (yours)`.
- **Aimed at you.** Each courtship of each of your Houses by its line, a line past two thirds
  marked `!` as needing attention; each such House a **Guard House Aldric** button (`... (replaces
  Court House Bertram)` while a channel runs; disabled with the reason when refused) with its
  preview from `house_guard_with(h, your inflow now)` (`Guard: −3.2 a turn to each courtship. The
  Sheaim: a split in ~6 turns without it, settling near 40 with it.`), or `Your channel guards
  it.`; then the Spread Religion channels aimed at your civilization, each with its chance.
- **Between others**: the channels you see between others, by name (`Sheaim: Court House Bertram
  (Kuriotates), since turn 130.`).
- **What others remember**: each met leader's cultural hostility, friendship and courted-House
  memory, each out of `CULTURE_MEMORY_MAX`.
- **Keywords**: Channel, Court and Guard as chips; a chip shows its reminder boxes. The screen's
  help (its (i)) says where things are and shows the same reminders; no text restates them.
- **The city screen's Culture section** is one line: `Culture: 12.8 a turn; 6.4 of it to Court
  House Aldric.`, `Culture: 12.8 a turn; nothing diverted in disorder.` or `Culture: 12.8 a turn;
  no channel.`, then a link that opens the Culture screen (the city screen is read again when it
  closes) and the keyword Channel.
- **The map** draws nothing of channels: the revolt marks went with Foment Revolt.
- **Protocol**: the command `culture_channel {action?, target?, religion?}` (no action stops); the
  `culture` query; the city query's `cultural` (`{rate, diverted, disorder, channel, reminders}`).
- **Tests**: `crates/ffh-play/tests/culture_screen.rs` (the view's numbers against the rules' on
  `ffh-play stage culture`), `assets/test/culture.test.mjs` (the formatting),
  `tools/web-culture-smoke.mjs` (the screen in a browser on that stage, with the review crops),
  `tools/web-smoke.mjs`'s `cultureUi` (a fresh game: the city's line and link, the three actions).
  `ffh-play stage culture` is the `persons` stage with a religion in every city of the human (set
  by hand), so Spread Religion has a refused target.

The per-city interface of 2026-10-30 (a city's channel and its chooser, the revolt pressure on a
city and its scale, the Cultural pressure panel, the map's revolt marks, the predictions of
revolt pressure) is retired with Foment Revolt; git history has it.

## Phase 1 baseline (master `9be40af`, before the sprint)

`ffh bench`, 8 civs, standard map, 400 turns, CPU-exclusive on this container: seed 1 19.2 s
(48.1 ms/turn, slowest 197 ms), seed 2 24.1 s (60.3 ms/turn, slowest 235 ms); no crashes.
