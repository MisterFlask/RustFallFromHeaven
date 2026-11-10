# Governors

This game's own rules, not FFH2's. The government civic decides who runs a civilization's cities:
the player by hand, or governors. Under governors the player sets empire priorities, may overrule a
city's role, and places empire orders (armies' units, settlers, wonders, rituals) at the cities it
picks; it never picks an ordinary building. This file is the rules as they play, what was decided
beyond the design, and what is left.

History: the first rules (the owner's design spec "FfH Culture Actions & Army Combat", 2026-10-29:
four directly ruled cities, every other city under a governor from a bench, governors with a
profile and numeric traits) were replaced on 2026-11-10 by the owner's design "Governors and army
requisition" (2026-11-09) with the revisions agreed on it. The reason: a fixed count of hand-run
cities is most of a small empire and a rounding error in a thirty-city one, so where delegation
matters most the hand-run cities were still a chore; and matching governors with yield traits to
cities was the same micromanagement in another form.

| Part | What | Status |
|---|---|---|
| 1 | the civic decides who runs cities; no cap, no bench | done |
| 2 | city roles from building tiers, empire priorities, diagnoses, overrule | done |
| 3 | governors as characters: a name and one judgment trait | done (death and defection events: not built) |
| 4 | empire orders: units, wonders, palaces and rituals at a chosen city | done |
| 5 | governors that differ by civic (Republic, Theocracy, City States, Aristocracy) | not built (below) |

Code: rules `crates/ffh-core/src/governors.rs` (hooks: `player_set_civics`, `player_init_city`,
`city_kill`, the player turn, the production queue in `city/turn.rs`); data
`data/json/city_roles.jsonc`, `governor_traits.jsonc`, `governor_names.jsonc`, the civics'
`direct_rule`, the tiers' `core_effect_percent` (`ffh-data` `infos/governor.rs`), tuning
`data/json/rules_defines.jsonc` (`GOVERNOR_*`); AI `crates/ffh-ai/src/governor.rs` (what a governor
builds, its plan, the AI's priorities and empire orders); UI `crates/ffh-play/src/view/governors.rs`,
`assets/app/governors.js`. Tests: `crates/ffh-core/tests/governors.rs`; the harness checks that every
city is in its owner's mode, that a governed city has production, a role and a governor, and that
requisition orders, then empire orders, lead its queue (`ffh_harness::check`).

Every game has governors (no game option).

## Rules

**Who runs cities.** A civic may carry `direct_rule`; Despotism and God King do. A player with such
a civic rules every city directly: they play as cities always have. A player without one has every
city governed, the capital included. There is no hand-run core and no cap. A city gained (founded,
captured, traded, flipped) takes its owner's mode. A change of civics hands every city over, or
takes every city back, at once. Barbarian cities are never governed.

- *Handed over*: the city's queue keeps its requisition orders and the orders that may be empire
  orders (they become empire orders); its other orders are dropped. Its default recruitment
  destination is dropped and its role's citizen emphasis is set.
- *Taken back*: the queue is kept, the emphasis is dropped.

The two hand-run civics cost more with size (2026-11-10): Despotism +40% city-count maintenance
(was +25%); God King +25% distance maintenance (was +10%) and +30% city-count maintenance (new).

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
They weigh in every proposal (above). The game reports diagnoses: a priority role no city has (every
role, in an empire of six or more governed cities); an overruled city under 100% suitability for
its role; an empire order more than 30 turns from done.

**Governors.** A city gets a governor the first time it is governed: a name from its
civilization's list and, with 50% (`GOVERNOR_TRAIT_CHANCE`, the game's RNG), one trait. Traits are
about judgment, not yield: *Cautious* keeps one more defender; *Ambitious* builds the costliest
building it may first. The governor stays with the city (without effect while it is ruled
directly) and leaves the game when the city is lost. Governors have no yield, happiness, health,
production or experience modifiers, there is no bench, and nothing is assigned.

**What a governor builds** (ffh-ai `governor.rs`, for the human's cities and the AI's alike):
1. the garrison the city's danger asks for (the city AI's quota, the role's and the trait's
   defenders; against a Foment Revolt or Flip City aimed at the city, defenders up to three);
2. the best building by the city AI's value per turn of production, weighted by the role (up to
   +50% for a building whose flavors are all ones the role weighs 5); an Ambitious governor takes
   the costliest instead;
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

- **No hand-run core under governors, the capital included.** What a hand-run capital was for
  (settlers, wonders, rituals) is done by empire orders at any city, so a wonder's tier still makes
  "which city builds it" a choice (docs/SUITABILITY.md, section 5).
- **Roles are the old profiles.** The six profiles became the six roles; the role belongs to the
  city, not to the governor.
- **One number prices an overrule**: the suitability, shown for every role on the city's card.
- **A change of government gives one message**, not one per city; a founded or gained city
  announces its governor.
- **Choosing the proposal is always allowed**, under 75% too, so an overrule can always be ended.
- **A city that changes hands** takes its new owner's mode, a new governor and a fresh proposal.
- **Tall play has two governments** (accepted for now): both hand-run civics come by Mysticism, so
  a player who wants to run cities by hand takes no later government.
- **The AI plays through the same layer**: its governed cities are built by the same governor, and
  it places empire orders (docs/AI.md, "Governors").

## Open points for the owner

- **The acceptance test.** Governed play must be at least as strong as hand play in a wide empire,
  or the careful player is back to running fifteen cities by hand because it wins. The audit below
  compares governors with the AI's own choices, not with a careful human, and the size costs above
  are a first step, not a measured answer. To be judged in play.
- **Governors that differ by civic** (the design's section 3: Republic's elected governors whose
  overrule costs, Theocracy's priests, City States' strong independent ones, Aristocracy's houses,
  and each civic's terms for army orders): not built. Every governing civic has the same governor
  until the uniform one is judged sound.
- **A late hand-run government**, if tall play should have a government to grow into.
- **Suitability ignores the site**: a landlocked city's Production suitability counts its Shipyard
  and its Growth suitability its Harbor. Key buildings the city can never build should probably be
  left out.
- **A great-people role**, and whether every role should have an empire-level output shown.
- **The standing requisition** fills by a role mix the player does not choose; with unit orders it
  may no longer be needed for the human.
- **Death and defection** of governors as events; a line a governor says when it takes a city or
  is overruled (the messages are bare today).
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
