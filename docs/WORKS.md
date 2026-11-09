# City works: no worker units

This game's own rules, not FFH2's (the owner's direction, 2026-11-09: "eliminate workers"). No
unit improves the land. Each city has work crews that improve its land, lay its roads and launch
its fishing boats; army units build roads, forts and the cards' field works where they stand;
mages still build mana nodes. The Worker, the Mud Golem, the Work Boat and the Pioneer are gone.

Why: with armies, governors and formation battles the player commands armies and provinces, and
the worker was the last unit moved plot by plot for no decision. A governed city trained workers
that the human then had to move by hand.

Code: rules `crates/ffh-core/src/works.rs` (the crews), `unit/missions.rs` (`unit_can_build`,
`unit_work_rate`: the field works), `plot/terrain.rs` and `player/units.rs` (the Pirate Cove's
rule, a build's civilization); data `builds.jsonc` (`city`, `field`, `prereq_civ`),
`rules_defines.jsonc` (`CITY_WORK_*`, `FIELD_WORK_RATE`), `units.jsonc` (`crew_work`),
`civilizations.jsonc` (`work_rate_modifier`); AI `crates/ffh-ai/src/workers.rs`
(`choose_crew_build`). Tests: `crates/ffh-core/tests/works.rs`.

## Rules

**Crews.** Every turn a city adds its work rate to one build on one plot (`City::crew_build`);
the progress is the plot's, as it was under workers. It keeps that build until it is done or the
crews cannot go on with it, then its owner's controller chooses the next
(`Controller::choose_crew_build`): for the AI's cities and a human's alike, the build worth the
most for the turns it takes, by the valuation the AI's workers had (docs/AI.md, "Workers"). The
player does not order the crews. The barbarians' cities have none (they had no workers).

**Work rate.** `CITY_WORK_RATE` (50, half of FFH2's Worker: "Measures" below), plus `crew_work`
for each unit of the owner in the city that has it (the Slave: 25, half a crew as it was half a
Worker), times the owner's work rate modifiers
(what sped workers up: civics, techs, buildings, traits) and its civilization's
`work_rate_modifier` (the Luchuirp: +50%, as their Mud Golems had); the AI handicap's modifier as
for workers. A city in disorder does no work. A build's gold cost is paid when it starts.

**Where.** A city's crews work the plots of its owner that it is the work city of: the city that
works the plot, else the owner's nearest city (a resource inside the borders that no city works).
Beyond the borders they build only roads: on unowned land next to the owner's land or to a road,
at most `CITY_WORK_ROAD_RANGE` (8) plots from the city. So a road between two cities advances plot
by plot from each end, through land nobody owns. Never in another civilization's land.

**Enemies.** The crews stay off a plot an enemy unit their owner sees stands on; beyond the
borders also off a plot next to one. The progress made stays on the plot.

**Builds** (`builds.jsonc`):

| | builds |
|---|---|
| `city` (the crews) | Road, Farm, Mine, Workshop, Lumbermill, Windmill, Plantation, Quarry, Pasture, Camp, Winery, Cottage, Remove Jungle, Remove Forest; Fishing Boats and Whaling Boats (400 work; FFH2: a Work Boat, used up); the Lanun's Pirate Cove (below) |
| `field` (army units) | Road, Fort |
| a unit's own (`units.jsonc` `builds`) | the mana nodes (Adepts, Mages, Archmages and their like), unchanged |

A card's build takes the builders of its `same_builders_as` build: the Road's (crews and army
units: Siege Lines, Canal Locks, Wayside Shrines, Trench), the Fort's (army units: the Trading
Post), the Fishing Boats' (crews: the Causeway).

**Field works.** An army unit (`Game::unit_is_army_unit`: a land combat unit that is not a mage, a
recon unit, a hero, a summon or a barbarian) does the `field` builds on the plot it stands on, at
`FIELD_WORK_RATE` (50, a crew's) a turn each, under the rules a worker's build had (the unit's moves end;
several units on a plot add up). The Route To order works for them. So an army lays a road behind
its advance, which carries its supply (docs/ARMIES.md), at the price of its march.

**The Pirate Cove** is a build of the Lanun's crews (`BUILD_PIRATE_COVE`, Fishing; FFH2: a Work
Boat's spell): on an empty coast plot of theirs without a resource, with no cove within 2 plots
(`Game::plot_is_pirate_cove_valid`).

**Slaves** build nothing. A Slave in a city of its owner adds 25 to the crews' work rate; it is
still sold, shown in the Freak Show, sent to the arena, or made a Beastman by the Doviello.

## What went with the workers

- Units: `UNIT_WORKER`, `UNIT_MUD_GOLEM`, `UNIT_WORKBOAT`, the card unit `UNIT_PIONEER`; their
  classes. A captured Settler is destroyed (it became a Worker).
- Grants of workers: the goody hut's (`GOODY_WORKER`), the eras' and handicaps' free starting
  workers, the Mercurian Gate's two, the Infernals' one, a barbarian-freed city's
  (`formEmpire`), the Court of Mammon's and the Patrians' (cards).
- The Doviello's Worker -> Beastman cast (`SPELL_UPGRADE_DOVIELLO_WARRIOR`; the Slave's stays).
  The barbarian world spell that turned workers into wolves turns none.
- The Crusade civic forbids only Settlers (it also forbade Workers and Work Boats).
- The workers' automations (`AUTOMATE_BUILD`, `NETWORK`, `CITY`): the rules refuse them.
- Cards: **Pioneers** is now +100% work rate for every city's crews (it was a national worker
  with three times the work rate). **Maponos's Cradle** makes a Settler every 20 turns while a
  site is free, and nothing otherwise (it made a Worker).

## Measures

8 games (6 civs, seeds 1-8, 250 turns; `tools/agents/ab.sh`), the commit before against the crews
at `CITY_WORK_RATE` 100 and 50; averages per civilization:

| | workers (before) | crews at 100 | crews at 50 |
|---|---|---|---|
| turn 100: cities / population / improvements | 2.6 / 9.2 / 6.9 | 3.4 / 13.1 / 14.0 | 3.0 / 11.7 / 7.8 |
| turn 200: cities / population / improvements | 6.8 / 31.4 / 32.8 | 7.4 / 46.0 / 63.0 | 7.0 / 37.8 / 38.9 |
| turn 200: techs / research / production | 15.3 / 64 / 44 | 19.6 / 160 / 66 | 16.4 / 93 / 53 |
| turn 200: military units | 22.2 | 33.6 | 30.1 |
| turn 250: population / techs | 48.7 / 23.6 | 57.3 / 29.1 | 56.3 / 27.7 |
| wars declared / cities captured from civs / civs eliminated | 25 / 29 / 0 | 60 / 74 / 6 | 48 / 70 / 3 |

At a Worker's rate a city the game ran some 40 turns ahead by turn 200 (the AI had trained fewer
workers than cities could use, and the crews cost no hammers), so the rate is 50: the land is
improved about as fast as before. What stays different at 50: the hammers that went into workers
go into settlers and soldiers (a third more military units at turn 200), and with them come about
twice the wars and captures. That is a consequence of free crews, not of their rate.

Checks: `ffh-stress batch` (117 games: eras, world sizes, civ counts, options, 20 random, 300
turns) without a failure; the browser smoke test.

## AI

- `workers::choose_crew_build` is the old worker valuation over `Game::city_crew_plots`, with the
  rules' road plan (`Game::player_road_plan`, the AI's old `road_plan`).
- Production, the governors and the advanced start train no workers or work boats; idle workers
  are no longer disbanded.
- Slaves go to the nearest city and stay (`units::laborer`).
- Trading Post: a spare soldier of a garrison of four or more walks to the site and builds it,
  then the road home. Siege Lines: the weakest unit of the besieging stack that can dig.
- The AI's armies do not lay roads behind an advance (a captured city supplies them, and its
  crews link it up).

## Open points for the owner

- **No orders for the crews.** A player cannot tell a city what to improve next (a cottage rather
  than a farm, this forest left alone). The rule has the hook (`Game::city_set_crew_build`);
  the interface has no control for it.
- **One crew a city**, at half a worker's rate, whatever its size (`CITY_WORK_RATE`: "Measures").
- **Routes to anywhere.** A city lays roads toward its owner's cities within 8 plots; a road to a
  place of the player's choosing is an army's to lay.
