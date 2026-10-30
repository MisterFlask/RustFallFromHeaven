# Governors (`GAMEOPTION_GOVERNORS`)

This game's own rules, not FFH2's: the third system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), "City automation and governors". Beyond a few directly ruled
cities, every city is run by a governor from a bench; for each governed city the player decides
only who governs it and whether it feeds an army or develops itself. This file says what was
built, how the spec's open points and edge cases were decided, and what is left.

| Phase (spec) | What | Status |
|---|---|---|
| 5 Governors | production audit; direct-rule slots, governed-city modes, army target sizes, governors, the bench, the Governors screen, AI modes | done |
| 6 Engagement combat | stance quotas, which "Feed army" will fill | after the owner's review of the promotion inventory (docs/COMBAT_INVENTORY.md) |

Code: rules `crates/ffh-core/src/governors.rs` (hooks: `player_init_city`, `city_kill`, the player
turn, `player_find_new_capital`, the city's yield, commerce, happiness, health, production and
experience modifiers, `city_can_train` / `construct` / `create`); data
`data/xml/Gameinfo/CIV4GovernorProfileInfos.xml`, `CIV4GovernorTraitInfos.xml`,
`CIV4GovernorNameInfos.xml` (`ffh-data` `infos/governor.rs`), tuning `data/xml/RulesDefines.xml`
(`GOVERNOR_*`); AI `crates/ffh-ai/src/governor.rs` (the governor's production choice, the AI's
slots and modes), `culture.rs` (`govern`: a human's governors' cultural actions); UI
`crates/ffh-play/src/view/governors.rs`, the city query's `governance`, `assets/app/governors.js`.
Tests: `crates/ffh-core/tests/governors.rs`, `crates/ffh-ai/src/governor.rs`; the harness checks
that a governed city always has production (`ffh_harness::check`).

## The option

`GAMEOPTION_GOVERNORS` is a visible game option with `bDefault` 1, like the cultural actions and the
armies: the browser's new-game screen starts with it ticked, and everything else (`ffh autoplay`,
`ffh stats`, the AI batches, soaks) plays without it unless asked (`--option GAMEOPTION_GOVERNORS`).
Off, nothing of this runs. It works without armies too (governed cities then only develop).

## Rules

**Direct rule.** A player rules up to `GOVERNOR_DIRECT_RULE_CITIES` (4) cities directly, the capital
always among them; they play as cities always have. Every other city is governed. A new city
(founded, captured, traded, flipped) takes a free direct slot if there is one, else it is
governed: gaining a city never takes direct rule away from another. The player may rule a governed
city directly in place of a directly ruled one (or in a free slot), or hand a directly ruled city
to a governor; a city whose rule changed by choice keeps it for
`GOVERNOR_DIRECT_RULE_SWAP_COOLDOWN` (10) turns. World wonders, palaces, rituals (projects) and
settlers are built only in directly ruled cities (`city_can_*`).

**Governed cities.** Their production is automated and cannot be set by hand (the commands that
push, pop or clear orders, set the automation or a recruitment destination are refused); the
governor chooses it (below). Each governed city runs in one of two modes: *develop* (new
governed cities) or *feed* one of the player's armies (docs/ARMIES.md): units for it, sent in
transit, while the army with its units on their way is short of its target size, then develop
until it needs units again. The army's target size (the Armies screen) replaces "Reinforce to N"
for governed cities. An army gone or disbanded puts its feeding cities back to develop, with a
message. The governor's profile sets the citizens' emphasis (its `Emphasizes`).

**What a governor builds** (ffh-ai `governor.rs`, for the human's cities and the AI's alike):
1. the garrison the city's danger asks for (the city AI's quota, the profile's `iDefenders`);
2. feeding its army: siege while it is less than a third of the army, else the best attacker
   against cities;
3. workers while the land needs improving, work boats for unimproved sea resources;
4. the best building by the city AI's value per turn of production, weighted by the profile:
   up to +50% for a building whose flavors are all ones the profile weighs 5;
5. a missionary, for a profile that weighs religion, while its religion misses our cities;
6. the process the profile prefers (research, gold or culture).

**Governors.** A governor (`Governor`) has a name, a profile, 0 to `GOVERNOR_MAX_TRAITS` (2)
traits, an origin and a tenure (recorded, no effect yet). Profiles: Steward (growth, production),
Scholar (science), Merchant (gold), Zealot (religion, culture), Marshal (military; one more
defender), Builder (production), and the neutral Interim profile of a city without a governor.
Traits (12): +10% production toward buildings or units, +2 experience for units trained, +1
happiness, +1 health, ±10% food, production, gold, research or culture, -1 happiness with +10%
production; no two of one kind. Names come from the civilization's list, else a generic one.

**The bench.** A player's governors without a city. While the bench is shorter than the vacant
posts plus `GOVERNOR_BENCH_SURPLUS` (2), each turn adds a candidate with
`GOVERNOR_BENCH_GEN_CHANCE` (15)% (the game's RNG), its profile and traits drawn toward the
leader's flavors. A newly governed city takes the candidate that suits it best (its profile's
flavors against what the city needs: growth while small or starving, production while it makes
little, military near a rival, science and gold with much commerce, culture on contested ground,
religion with a religion); an interim city takes the next candidate that appears. Replacing a
governor costs the city `GOVERNOR_SWAP_PENALTY` (25)% of its production and commerce for
`GOVERNOR_SWAP_TURNS` (5) turns, and the replaced governor returns to the bench; moving a governor
from one city to another costs both (the one it leaves is interim). A city lost (captured, flipped,
traded, razed) takes its governor out of the game.

**Cultural actions.** A governor queues friendly and defensive cultural actions on its own
(festivals against hostile channels, trade fairs, religion: the AI's cultural play, limited to
the governed cities), and hostile ones only where the player allowed them (the city's "Allow hostile
cultural actions", off by default). The player can still queue any action by hand.

## Edge cases decided (beyond the spec's text)

- **Barbarian cities are never governed** (and settlements, the Kuriotates' cities that build
  nothing, are governed but have nothing to build).
- **The capital moves.** A new capital (the old one lost) is ruled directly at once; if that makes
  one direct city too many, the city made direct most recently goes to a governor, with a message.
  A palace cannot be built in a governed city, so the capital never moves into one by choice.
- **Filling a vacant post costs nothing**: the penalty is for replacing a governor, not for giving
  an interim city one.
- **An interim city takes the next candidate** without the player's click (the spec has newly
  governed cities auto-assign; an interim city is one that could not, and leaving it interim
  until the player notices would only cost the city).
- **Directly ruled cities keep their emphasis**; a governed city's emphasis is its profile's
  (the player sets it again after taking a city back).
- **Feeding without a target size** (0) feeds without end: the player's choice.

## Open points for the owner

- **Workers trained by a human's governed cities** are the human's to move (or automate), like any
  other worker; the spec does not say whether governors should automate them.
- **Stance quotas.** "Feed army" fills by a simple role mix (a third siege) until phase 6 brings
  stances.
- **Conquest.** With governors on, the AI captures a fifth fewer cities (below): governed cities
  develop instead of building surplus attackers. Its wars are fed by its feeding cities.

## The production audit

The spec asks to audit the production AI before governors: forced automation is tolerable only if a
neutral governor is competent. The AI's own production choice (`production.rs`) was the starting
point; the governor's (`governor.rs`) keeps its garrison, worker and building logic and drops what
direct rule is for (settlers, wonders, the AI's campaign armies outside feeding). Measured as the
spec's acceptance asks ("large AI empires keep pace with the baseline in autoplay"): 8 civs,
standard map, 350 turns, seeds 1-4, armies on in both, governors on against off (the AI governing
every city beyond its first four):

| | governors off | governors on |
|---|---|---|
| population (all civs, end) | 1815 | 1888 (+4%) |
| techs | 1446 | 1515 (+5%) |
| production | 1733 | 2048 (+18%) |
| research | 4667 | 5545 (+19%) |
| cities | 196 | 198 |
| cities captured | 85 | 68 |
| civilizations eliminated | 7 | 4 |
| the largest empire's techs (per seed) | 39, 54, 48, 42 | 46, 58, 44, 51 |

The governed empires grow and research faster and fight a little less. Checked-build stress games
(400 turns, culture, armies and governors on) pass the harness's checks, a governed city never
without production among them.
