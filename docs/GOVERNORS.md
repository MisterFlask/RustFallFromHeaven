# Governors

This game's own rules, not FFH2's. A city is run by its owner by hand or by a governor: under City
States and Aristocracy every city is governed; under any other civic the owner hands cities to
governors, and takes them back, city by city. Over its governed cities the player sets empire
priorities, may overrule a city's role, and places empire orders (armies' units, settlers,
wonders, rituals) at the cities it picks; it never picks an ordinary building there. This file is the rules as they play, what was decided
beyond the design, and what is left. A city granted to a House (docs/PERSONS.md) is the House's
player's since the second build (2026-11-18): the House is a vassal civilization of its own and
governs its cities itself; the first build's order quota is gone. Its liege reaches its cities only
by the levy (docs/ARMIES.md, "Requisition").

History: the first rules (the owner's design spec "FfH Culture Actions & Army Combat", 2026-10-29:
four directly ruled cities, every other city under a governor from a bench, governors with a
profile and numeric traits) were replaced on 2026-11-10 by the owner's design "Governors and army
requisition" (2026-11-09) with the revisions agreed on it. The reason: a fixed count of hand-run
cities is most of a small empire and a rounding error in a thirty-city one, so where delegation
matters most the hand-run cities were still a chore; and matching governors with yield traits to
cities was the same micromanagement in another form.

| Part | What | Status |
|---|---|---|
| 1 | who runs cities: no cap, no bench; since 2026-11-21 governed civics govern every city, under the others the owner chooses city by city | done |
| 2 | city roles from building tiers, empire priorities, diagnoses, overrule | done |
| 3 | governors as characters: a name and one judgment trait | removed 2026-11-18 (below) |
| 4 | empire orders: units, wonders, palaces and rituals at a chosen city | done |
| 5 | governors that differ by civic (Republic, Theocracy, City States, Aristocracy) | retired 2026-11-21: governors are mandatory only where a government is about them (below) |

Code: rules `crates/ffh-core/src/governors.rs` (the hand-over and take-back
`player_set_city_governed`; hooks: `player_set_civics`, `player_init_city`, `city_kill`, the
player turn, the production queue in `city/turn.rs`; the controller's `choose_city_governance` and
`govern_gained_city`); the command `govern_city {city, governed}` (`crates/ffh-play`); data
`data/json/city_roles.jsonc`, `governor_names.jsonc` (read by nothing now; kept for the Houses of
docs/PERSONS.md), the civics'
`governed`, the tiers' `core_effect_percent` (`ffh-data` `infos/governor.rs`), tuning
`data/json/rules_defines.jsonc` (`GOVERNOR_*`); AI `crates/ffh-ai/src/governor.rs` (which of its
cities it governs, what a governor builds, its plan, the AI's priorities and empire orders); UI
`crates/ffh-play/src/view/governors.rs`, `assets/app/governors.js`, the city screen's toggle
(`assets/app/city.js`), the civics preview (`view/player.rs`, `assets/app/empire.js`). Tests:
`crates/ffh-core/tests/governors.rs`, `crates/ffh-play/tests/governors.rs`; the harness checks
that under a governed civic every city is governed, that a governed city has production and a
role, and that requisition orders, then empire orders, lead its queue (`ffh_harness::check`).

Every game has governors (no game option).

## Rules

**Who runs cities** (the owner's decision of 2026-11-21: governors are mandatory only where a
government is about them). A civic may carry `governed`; City States and Aristocracy do.

- *Under a governed civic* every city of its player is governed, the capital included; there is no
  hand-run core and no cap. Adopting one hands every city run by hand over at once, with one
  message; no city can be taken back while it lasts (`City States: every city is run by a
  governor.`).
- *Under any other civic* (Despotism, God King, Republic, Theocracy, ...) a city is run by hand
  unless its owner hands it to a governor. The owner hands over any of its cities and takes any
  back at will: no cost, no delay, city by city (the city screen's `Hand to a governor` and `Run
  by hand`, the Governors screen's toggles). An empire may be mixed; a governed city in it is
  governed in every way as under a governed civic.
- *Leaving a governed civic changes no city*: each stays governed until its owner takes it back.
- *A city gained* (founded, captured, traded, flipped, an outpost raised) is run by hand, unless
  a governed civic governs it. An outpost has no governor (docs/NAVAL.md, 9.2).
- Barbarian cities are never governed.

- *Handed over*: the city's queue keeps its requisition orders and the orders that may be empire
  orders (they become empire orders); its other orders are dropped. Its default recruitment
  destination is dropped and its role's citizen emphasis is set.
- *Taken back*: the queue is kept, the emphasis is dropped.

Despotism and God King cost more with size (2026-11-10, when they were the two hand-run civics;
unchanged by the decision of 2026-11-21): Despotism +40% city-count maintenance (was +25%); God
King +25% distance maintenance (was +10%) and +30% city-count maintenance (new).

**Roles.** A governed city has one role: Growth, Research, Commerce, Culture, Garrison or
Production (`city_roles.jsonc`). A role has the city AI's flavors it weighs, the citizen emphasis
it sets, the defenders it adds to the garrison, and four key building classes:

| Role | Key buildings |
|---|---|
| Growth | Granary, Smokehouse, Harbor, Aqueduct |
| Research | Library, Elder Council, Alchemy Lab, Mage Guild |
| Commerce | Market, Moneychanger, Gambling House, Tax Office |
| Culture | Monument, Pagan Temple, Theatre, Carnival |
| Garrison | Palisade, Walls, Training Yard, Archery Range |
| Production | Forge, Shipyard, Command Post, Siege Workshop |

- *Suitability* of a city for a role: the mean of its tiers' core effect (Lesser 50, Standard 100,
  Fine 150, Grand 200, Legendary 200; docs/SUITABILITY.md) over the role's key buildings that its
  civilization can build. So 100% is a city of Standard buildings.
- *The proposal*: the role with the highest suitability, a priority role's counted at +30%
  (`GOVERNOR_PRIORITY_WEIGHT`). A city takes the proposal when it becomes governed, when it changes
  hands and when its owner's priorities change.
- *Overruling*: the player may give a governed city any role it has at least 75% suitability for
  (`GOVERNOR_ROLE_MIN_SUITABILITY`). The city keeps that role until the player changes it; choosing
  the proposal ends the overrule. Overruling has no cost of its own: its price is the difference
  in suitability, which is what the city's buildings of that role will be worth.

A role has no modifier of its own. It decides what the governor builds first and what the citizens
work; the tiers decide what that is worth.

**Empire priorities.** A player names up to two roles (`GOVERNOR_MAX_PRIORITIES`) as priorities.
They weigh in every proposal of a governed city (above). The game reports diagnoses over the
governed cities only: a priority role no governed city has (every role, with six or more governed
cities); an overruled city under 100% suitability for its role; an empire order more than 30 turns
from done. In a mixed empire the cities run by hand count for none of them.

**Governors.** A governor is not a character: it has no name and no trait. Named governors (a name
from the civilization's list and, with 50%, one trait: *Cautious*, one more defender; *Ambitious*,
the costliest building first) were removed on 2026-11-18 by the owner's decision (docs/PERSONS.md:
the House's name takes their place). Governors have no yield, happiness, health, production or
experience modifiers, there is no bench, and nothing is assigned.

**What a governor builds** (ffh-ai `governor.rs`, for the human's cities and the AI's alike):
1. the garrison the city's danger asks for (the city AI's quota and the role's defenders; against a Foment Revolt or Flip City aimed at the city, defenders up to three);
2. the best building by the city AI's value per turn of production, weighted by the role (up to
   +50% for a building whose flavors are all ones the role weighs 5);
3. a missionary, for a role that weighs religion, while the state religion misses our cities;
4. the process the role prefers (research, gold or culture).

It never builds a wonder, a palace, a ritual, a settler or a unit for the field: those are empire
orders. Its next buildings are shown as its plan.

**Empire orders.** In a governed city the player cannot queue buildings or processes, and may
place:
- a unit, any the city can train, bound for an army, a garrison or nowhere (settlers, heroes,
  missionaries, ships);
- a world, national or team wonder, or a palace;
- a ritual (project).

The order goes ahead of the governor's choice, which keeps its progress and resumes after: an
order's cost is delay. The queue's order is requisition orders (docs/ARMIES.md), then empire orders
as placed, then the governor's choice. The player may take an empire order off. Empire orders do not
repeat. This is how an army's composition is the player's: ask for a unit type for an army, see the
cities that can train it with what each would delay, pick one (the client pre-selects the nearest
capable city). The standing requisition (an army's target size and the cities it draws on) is
unchanged and fills by its role mix.

**Cultural channels are the player's** (the owner's decision, 2026-10-30): a governor never sets,
changes or ends a city's channel (docs/CULTURE.md).

## Decided beyond the design's text

- **No hand-run core under a governed civic, the capital included.** What a hand-run capital was for
  (settlers, wonders, rituals) is done by empire orders at any city, so a wonder's tier still makes
  "which city builds it" a choice (docs/SUITABILITY.md, section 5).
- **Roles are the old profiles.** The six profiles became the six roles; the role belongs to the
  city, not to the governor.
- **One number prices an overrule**: the suitability, shown for every role on the city's card.
- **Adopting a governed civic gives one message**, not one per city. Handing a city over or taking
  it back by hand gives none (the screen shows it).
- **Choosing the proposal is always allowed**, under 75% too, so an overrule can always be ended.
- **A city that changes hands** is run by hand under its new owner (governed under a governed
  civic, with a new governor and a fresh proposal); its old role is not kept.
- **Handing over and taking back is one rule each way**: the queue rule of "Handed over" and
  "Taken back" above, whatever the reason (the owner's toggle, a governed civic adopted).
- **The capital may be handed over** like any city.
- **The AI plays through the same layer**: its governed cities are built by the same governor, and
  it places empire orders (docs/AI.md, "Governors"). It hands every city to a governor unless its
  government is Despotism or God King, and takes every city back under those two, through the same
  call as the human's command (`player_set_city_governed`, from `Controller::choose_city_governance`
  after its civics change and in its turn before its cities produce); a city it gains is handed
  over at once (`Controller::govern_gained_city`). That is what the rules made it do until
  2026-11-21, so AI games play as before (the same checksums over 200 turns, seeds 1-4). Whether
  the AI should govern from the first turn, or keep some cities by hand, is not measured.

## Open points for the owner

- **The acceptance test.** Governed play must be at least as strong as hand play in a wide empire,
  or the careful player is back to running fifteen cities by hand because it wins. The audit below
  compares governors with the AI's own choices, not with a careful human, and the size costs above
  are a first step, not a measured answer. To be judged in play.
- **Governors that differ by civic** (the design's section 3: Republic's elected governors whose
  overrule costs, Theocracy's priests, City States' strong independent ones, Aristocracy's houses,
  and each civic's terms for army orders): retired by the decision of 2026-11-21. Governors are
  mandatory only under the civics about them, and the same governor runs every governed city.
  The earlier point that tall play had only two governments (both hand-run civics by Mysticism) is
  resolved: every government but City States and Aristocracy lets cities be run by hand.
- **Aristocracy** shares City States' mandate for now; its own identity is open (docs/HOUSES.md).
- **The size costs of Despotism and God King** were raised on 2026-11-10 to price ruling by hand;
  ruling by hand is now the default of four governments (Despotism, God King, Republic, Theocracy),
  and only those two pay for it.
- **Suitability ignores the site**: a landlocked city's Production suitability counts its Shipyard
  and its Growth suitability its Harbor. Key buildings the city can never build should probably be
  left out.
- **A great-people role**, and whether every role should have an empire-level output shown.
- **The standing requisition** fills by a role mix the player does not choose; with unit orders it
  may no longer be needed for the human.
- **The AI's wonders.** With empire orders the AIs build about 60% more world wonders by turn 300
  than before the change (8 seeds; docs/AI.md, "Governors"); founding, captures and army sizes are
  level. The bar of the AI's wonder step can be raised if that is too many.
- **An empire order's destination** is set when it is placed and cannot be changed after.
- **Supply** (docs/SUPPLY.md) is not shown when a city is picked for an army's unit.

## The production audit (history: the rules before 2026-11-10)

Measured under the first rules (four directly ruled cities, a bench of governors with profiles and
numeric traits). Kept because the result it gives, that a governed city is built at least as well as
the AI builds its own, is what the present rules rest on.

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
