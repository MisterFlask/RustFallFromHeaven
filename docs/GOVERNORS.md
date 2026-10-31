# Governors (`GAMEOPTION_GOVERNORS`)

This game's own rules, not FFH2's: the third system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), "City automation and governors". Beyond a few directly ruled
cities, every city is run by a governor from a bench; for each governed city the player decides
only who governs it. Armies requisition soldiers from governed cities as from any city (the
owner's redesign of 2026-10-31: "governors do their thing and instead of them selecting armies to
feed, armies (as directed by the player) requisition soldiers from cities, which interrupts normal
production"; docs/ARMIES.md). This file says what was built, how the spec's open points and edge
cases were decided, and what is left.

| Phase (spec) | What | Status |
|---|---|---|
| 5 Governors | production audit; direct-rule slots, army target sizes, governors, the bench, the Governors screen | done |
| Requisition (2026-10-31) | the feed mode gone; armies requisition from any city (docs/ARMIES.md) | rules done; AI and UI next |
| 6 Engagement combat | stance quotas, which requisition will fill | after the owner's review of the promotion inventory (docs/COMBAT_INVENTORY.md) |

Code: rules `crates/ffh-core/src/governors.rs` (hooks: `player_init_city`, `city_kill`, the player
turn, `player_find_new_capital`, the city's yield, commerce, happiness, health, production and
experience modifiers, `city_can_train` / `construct` / `create`); data
`data/xml/Gameinfo/CIV4GovernorProfileInfos.xml`, `CIV4GovernorTraitInfos.xml`,
`CIV4GovernorNameInfos.xml` (`ffh-data` `infos/governor.rs`), tuning `data/xml/RulesDefines.xml`
(`GOVERNOR_*`); AI `crates/ffh-ai/src/governor.rs` (the governor's production choice, the AI's
slots, the unit a requisition takes), `culture.rs` (`govern`: a human's governors' defenders against revolt pressure); UI
`crates/ffh-play/src/view/governors.rs`, the city query's `governance`, `assets/app/governors.js`.
Tests: `crates/ffh-core/tests/governors.rs`, `crates/ffh-ai/src/governor.rs`; the harness checks
that a governed city always has production (`ffh_harness::check`).

## The option

`GAMEOPTION_GOVERNORS` is a visible game option with `bDefault` 1, like the cultural actions and the
armies: the browser's new-game screen starts with it ticked, and everything else (`ffh autoplay`,
`ffh stats`, the AI batches, soaks) plays without it unless asked (`--option GAMEOPTION_GOVERNORS`).
Off, nothing of this runs. It works without armies too.

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
governor chooses it (below): every governed city develops. An army's requisition
(docs/ARMIES.md) interrupts it as it interrupts a directly ruled city's production: the player
picks the governed city on the Armies screen, and while the army is short of its target size the
city trains a unit for it first, the requisition order at the front of its queue ahead of the
governor's choice (which keeps its progress and resumes after it); the unit marches to the army. A
governed city's own units go to its own garrison: the default destination it had under direct
rule is dropped when a governor takes over (it cannot be set while governed). The governor's profile sets the citizens' emphasis (its
`Emphasizes`), which the player cannot change while the city is governed; a city back under direct
rule drops it.

**What a governor builds** (ffh-ai `governor.rs`, for the human's cities and the AI's alike):
1. the garrison the city's danger asks for (the city AI's quota, the profile's `iDefenders`);
   Against a Foment Revolt or Flip City aimed at the city, defenders up to three.
2. workers while the land needs improving, work boats for unimproved sea resources;
3. the best building by the city AI's value per turn of production, weighted by the profile:
   up to +50% for a building whose flavors are all ones the profile weighs 5;
4. a missionary, for a profile that weighs religion, while its religion misses our cities;
5. the process the profile prefers (research, gold or culture).

An army's requisition goes ahead of all of it (the rules place it at the front of the queue).

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
`GOVERNOR_SWAP_TURNS` (5) turns (the turns' output: the countdown runs after the city's production
and commerce), and the replaced governor returns to the bench; moving a governor from one city to
another costs both (the one it leaves is interim). A city lost (captured, flipped,
traded, razed) takes its governor out of the game.

**Cultural channels are the player's** (the owner's decision, 2026-10-30): a governor never sets,
changes or ends a city's channel (docs/CULTURE.md); the player channels a governed city's culture
as a directly ruled one's. The governor's part is its production: it trains defenders against the
revolt pressure aimed at its city.

## Edge cases decided (beyond the spec's text)

- **Barbarian cities are never governed** (and settlements, the Kuriotates' cities that build
  nothing, are governed but have nothing to build).
- **A city that changes hands takes its governor when its owner's turn ends.** Captured, traded,
  flipped or given: it is governed (if no direct slot is free) and interim until its new owner's
  turn processing gives it the candidate that suits it best. So the candidate is matched against
  the city as it comes (its size, buildings and religions), not the blank city the transfer creates
  first, and a city razed, given away or disbanded at once costs no governor from the bench. Its
  production is chosen then too, with the units that took it inside; a city that changes hands
  outside its owner's turn (a culture flip, a trade in another player's turn) chooses at once, so
  no governed city reaches a turn's end without production. The message promises a governor only
  when the bench has one for every vacant post.
- **The capital moves.** A new capital (the old one lost) is ruled directly at once; if that makes
  one direct city too many, the city made direct most recently goes to a governor, with a message.
  A palace cannot be built in a governed city, so the capital never moves into one by choice.
- **Filling a vacant post from the bench costs nothing**: the penalty is for replacing a governor,
  not for giving an interim city one (a governor moved in from another city costs both, the spec's
  rule).
- **An interim city takes the next candidate** without the player's click (the spec has newly
  governed cities auto-assign; an interim city is one that could not, and leaving it interim
  until the player notices would only cost the city).
- **Directly ruled cities keep their emphasis**; a governed city's emphasis is its profile's
  (the player sets it again after taking a city back).
- **The feed mode is gone** (2026-10-31): a governor no longer picks an army to feed; armies
  requisition from the cities the player picks, governed or not, and only while they have a
  target size (docs/ARMIES.md). Saves of format 8 and earlier do not load (format 9).

## Open points for the owner

- **Workers trained by a human's governed cities** are the human's to move (or automate), like any
  other worker; the spec does not say whether governors should automate them.
- **Stance quotas.** A requisition fills by a simple role mix (a third siege) until phase 6 brings
  stances.
- **Conquest.** With governors on, a quarter fewer cities change hands (below), mostly because
  governed cities are better built; the warlike civilizations declare about as many wars. Whether
  their conquest drive should requisition harder with governors is for the tuning phase.

## The production audit

The spec asks to audit the production AI before governors: forced automation is tolerable only if a
neutral governor is competent. The AI's own production choice (`production.rs`) was the starting
point; the governor's (`governor.rs`) keeps its garrison, worker and building logic and drops what
direct rule is for (settlers, wonders, the AI's campaign armies). (The audit below was measured with
the feed mode, before requisition replaced it.) Measured as the
spec's acceptance asks ("large AI empires keep pace with the baseline in autoplay"): 8 civs,
standard map, 350 turns, seeds 1-24, armies on in both, governors on against off (the AI governing
every city beyond its first four), totals over the 24 games with the paired per-seed difference:

| | governors off | governors on | change (t) |
|---|---|---|---|
| population (all civs, end) | 10517 | 11386 | +8.3% (2.1) |
| techs | 7824 | 8112 | +3.7% (1.1) |
| production | 10633 | 12001 | +12.9% (2.7) |
| research | 26937 | 30388 | +12.8% (1.9) |
| cities | 1221 | 1246 | +2.0% |
| the largest empire's techs | 1050 | 1079 | +2.8% |
| cities that changed hands | 383 | 280 | -27% (-2.9) |
| civilizations eliminated | 27 | 16 | |

The governed empires grow, produce and research more, and fewer cities change hands. Most of that
last difference comes from the profiles' emphasis: a Builder's or Marshal's production emphasis
(the review found the AI's wonder emphasis turning it off every turn, and fixed it) makes governed
cities produce 8% more and the world's cities change hands a fifth less often (the old behaviour
restored, same seeds: captures +20%, t 2.1; production -8%, t -2.5), with about as many wars
declared: better-built cities fall less, not fewer attacks. The fixes from the review (2026-10-30)
measured against the code before them, same seeds: development within noise (production +4%,
population +2%, techs +2%, research -1%). The first audit (4 seeds, before the review) showed the
same direction with more noise. Checked-build stress games (400 turns, culture, armies and
governors on, seeds 41-46) pass the harness's checks, a governed city never without production
among them.
