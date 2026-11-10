# Supply lines

**Status: rules built (2026-11-10: sections 2 to 4, `crates/ffh-core/src/army/supply.rs`, tests in
`crates/ffh-core/tests/supply.rs`); the AI (section 6, `crates/ffh-ai/src/supply.rs`) built, its
measures in "Measures"; the client (section 5; docs/HUMAN.md) built.** This file
is the spec for the implementation. It replaces the supply rule of docs/ARMIES.md ("Supply": within
`ARMY_SUPPLY_RANGE` plots of the trade network, which only paused an army's recruits).

## 1. Why

Supply is here to do three things the old rule did not: put a limit on how far and how long an
army operates away from home, give the defender something to strike other than the army, and make
roads, forts and captured cities matter to a campaign. The old rule's only effect was that
recruits waited, so nobody had a reason to cut it.

The owner's design: a literal line, drawn on the map, from each army back to the nearest city or
ship. Cutting it brings two turns of Hunger, then Starvation, which weakens the army and damages
its units turn by turn. It is visible, it warns before it bites, and the one who cut it still has
to come and fight.

## 2. The line

Every army (docs/ARMIES.md: a named body of land army units) needs a supply line. Garrisons,
units in no army (heroes, mages, recon units, settlers, summons) and the barbarians need none.

**Sources.** A line ends at a source of the army's owner:
- a city of the owner's team that is not in occupation (`occupation_timer == 0`);
- a ship of the owner with room for land units (`cargo_space > 0`, `domain_cargo` land) that
  stands in a water area on which the owner's team has a coastal city: the line ends on a land
  plot next to the ship's plot.

An army standing in a city of its owner's team is supplied, whatever the city's state.

**Tracing.** The line is the cheapest path over land from the army's plot to a source, by plot
entered:

| Plot | Cost |
|---|---|
| has a route (anyone's road, wherever it is) | `SUPPLY_COST_ROAD` (1) |
| owned by the army's owner's team, no route | `SUPPLY_COST_OWN` (1) |
| any other land | `SUPPLY_COST_LAND` (3) |

It cannot enter: water, impassable plots, a plot a unit at war with the owner stands on (any
such unit, seen or not), a city of a team at war with the owner, a plot next to a fort
(`IMPROVEMENT_FORT` and what it upgrades to) that holds a unit at war with the owner, or a plot of
a team whose borders the owner may not enter. The army's own plot is always allowed.

The army is **supplied** when the cheapest path costs at most `SUPPLY_LINE_BUDGET` (24): 24 plots
of road, or 8 plots of open country beyond the last road or border. The line re-routes by itself
every time it is traced; it is **cut** when no path fits the budget. Ties between paths: the lower
cost, then the fewer plots, then the lower plot index at the first difference (deterministic).

So: a road laid behind an advance (army units build roads, docs/WORKS.md) reaches three times as
far as open country; the defender's own roads carry the invader's supply until they are pillaged;
a unit on a road forces a detour that may not fit the budget; a manned fort denies the plots
around it; an encircled army has no line at all.

`Game::plot_supply_line(p, plot) -> Option<SupplyLine>` (the path's plots from the army to the
source, its cost, the source) is a pure function: the rules, the view and the AI call it. Nothing
is cached.

## 3. Hunger and Starvation

`Army::unsupplied_turns` (saved). At the start of its owner's turn, before its units heal, each
army's line is traced: supplied sets the counter to 0, cut adds 1.

| Counter | State | Effects |
|---|---|---|
| 0 | Supplied | none |
| 1 to `SUPPLY_HUNGER_TURNS` (2) | **Hunger** | the army's units do not heal; recruits bound for it wait where they are |
| above | **Starvation** | as Hunger; and at the start of each of the owner's turns every unit of the army loses `STARVATION_DAMAGE_PERCENT` (10) % of its full HP, never going below `STARVATION_FLOOR_PERCENT` (25) % of it (a unit already below is not healed up to it); and the army's units deal `STARVATION_DAMAGE_DEALT_PERCENT` (-25) % damage in a battle |

Starvation never kills. A restored line ends both at the owner's next turn start: the counter
goes to 0, healing and recruits resume, nothing lingers.

A unit that leaves a hungry or starving army for a garrison or no army is out of it at once. An
army split from one takes its counter; a merged army takes the higher counter. An army's counter
is 0 when it forms from a garrison.

The battle effect goes through the engagement's pre-battle hook (`engage_pre_battle`,
docs/ENGAGEMENTS.md "Open", phase 7: a modifier on the side's damage), applies to the units of
starving armies on the field only, and shows in the attack preview and the battle log as its own
line.

The old rule goes: `ARMY_SUPPLY_RANGE`, `Game::plot_in_supply` and `army_in_supply`'s network
test. "In supply" for marching recruits (docs/ARMIES.md, "Supply gates progress") means the
army's counter is 0.

## 4. Defines (`rules_defines.jsonc`)

`SUPPLY_LINE_BUDGET` 24, `SUPPLY_COST_ROAD` 1, `SUPPLY_COST_OWN` 1, `SUPPLY_COST_LAND` 3,
`SUPPLY_HUNGER_TURNS` 2, `STARVATION_DAMAGE_PERCENT` 10, `STARVATION_FLOOR_PERCENT` 25,
`STARVATION_DAMAGE_DEALT_PERCENT` -25. `ARMY_SUPPLY_RANGE` is removed.

**As built** (choices where the text above is silent):
- An army aboard a source ship (on its water plot) is supplied by it (cost 0); an army standing next
  to one is too.
- A crusade Host (docs/CRUSADE.md), which owns no city at first, counts its lenders' and the
  victim's cities and land as its own team's (as it heals there).
- A player with no city (its starting units before the capital is founded) has no supply rule at
  all: its armies' counters stay 0 (a Host with its lenders' cities has cities).
- "Do not heal" covers every heal of the unit's own turn (Civ4's `doHeal`, the Pillar of Dawn), not
  spells cast on it.
- The battle effect is `BattleEffect::Dealt` (every hit of the fighter, its pairs' damage
  percentage), returned by `engage_pre_battle`; the preview lists the units (`EngagePreview::
  starving`), the battle log has a `LogEvent::Starvation` line before the morale check.
- Messages: `The 1st Army has no supply line: Hunger, turn 1 of 2. No healing, no recruits.`, `The
  1st Army is starving: units lose HP each turn, deal less damage.`, `The 1st Army is supplied
  again.` (at the army's plot); a waiting recruit's reason is `the army has no supply line`.

## 5. What the player sees

- **The line on the map**: for each of the player's armies, an animated line along its supply
  path from the army to its source (a slow flow toward the army). Hunger: the line is gone and
  the army carries a Hunger marker with the turns left before Starvation; Starvation: its marker.
  The lines of other players' armies the player sees are drawn over the plots the player has
  revealed, in their owner's colour: that is what makes cutting one a plan.
- **Before a move**: with an army selected, the plot under the pointer says whether the army would
  be supplied there (the path preview's line: "Supplied (cost 14 of 24)" or "No supply line").
- **The Armies screen and the unit panel**: the army's state in bare words: `Supplied: 14 of 24,
  from Ithralia.` / `Hunger: turn 1 of 2. No healing, no recruits.` / `Starvation: -10% HP a turn
  (to 25%), -25% damage dealt.`
- **Messages** at the start of the turn an army becomes hungry, starts starving, or is supplied
  again, with the army's plot.

Text is bare (CLAUDE.md, "Rules text and tooltips").

## 6. AI (ffh-ai)

A rules change: the AI does something reasonable with it, ad hoc (CLAUDE.md, "AI changes").
- An army does not move to a plot where it would have no line, unless an enemy city it is
  marching on is within two turns of that plot.
- A hungry army goes on only when the city it attacks is within two plots; else, and always when
  starving, it goes back along its last line toward its source until it is supplied.
- Nothing about cutting an enemy's line on purpose (a known gap: units that stand on a road cut it
  by being there).

**As built** (`crates/ffh-ai/src/supply.rs`; docs/AI.md, "Supply lines"):
- The gate (`supply::may_move`) reads the plot the move reaches *this turn* (the rules' path,
  `group_find_path`'s end-turn plot), not the final destination: an army walks no turn into a plot
  without a line. "Within two turns" is twice the slowest unit's base moves, in plots, from that
  plot to the city. A move into a city of ours and staying put are always allowed, and a plot of our
  own territory counts as supplied without a search (own land costs 1 a plot from a city: it is cut
  there only when enemy units close every way home, or in a pocket cut off from our cities).
  Gated: the
  hunt (the target or the plot next to it; a refused candidate gives way to the next of the four
  it looks at), joining a bigger stack, and every move of a siege operation (`campaign::march`:
  refused, the stack holds where it is). Not gated: crusade Hosts, amphibious landings, the
  Raiders' raids, escorts, marches to defend our cities.
- Falling back (`supply::fall_back`) runs once a stack has merged, before its captures and
  attacks. "The city it attacks" is its siege operation's target, else any enemy city at war
  within two plots. "Its last line toward its source" is not remembered (the AI keeps nothing
  outside `Game`): it goes toward the nearest of the three nearest cities of ours on its landmass
  that it has a path to (else to the plot of our territory, or nearest that city, it reaches within
  three turns), and stops falling back as soon as the plot it stands on has a line (its counter goes
  to 0 at the next turn start).
- A cut army does not stop to heal in the field (it cannot), and it is re-planned at the start of
  every turn: its multi-turn order is cleared (`units::play`).
- Cost: at most one path search and one line search per gated move; for a cut army, one line search
  and up to three path searches a turn.
- Switch `SUPPLY` (`FFH_AI_SUPPLY_OFF`): off, the AI ignores supply. Telemetry: `supply_state`
  (each army-turn: `supplied`, or `cut_NN` for its counter), `supply` (`move_refused`,
  `fall_back`, `hungry_goes_on`).

## 7. Measures the implementation reports

With the AI rule in, against the commit before (the tools of docs/AI.md; 8 games of 6 civs, 250
turns): the share of army-turns hungry and starving, units lost to nothing (Starvation never
kills: 0), cities captured from civs and wars declared (the AI's conquests should not collapse:
report the change, do not tune the rules to hide it), time a turn. And a staged test of each rule
of sections 2 and 3.

## Measures

Measured 2026-11-10: 8 games (seeds 1-8, 6 civs, standard map, 250 turns; `tools/agents/ab.sh
--base a46576a --seeds 1-8 --turns 250 --at 100,200,250 --jobs 3` and `ai_metrics.py` with the same
arguments). Three sides: the commit before supply lines (a46576a); this tree (the rules of supply
lines, the AI's supply rule of section 6 and its battle-tactic rule, docs/TACTICS.md); and this tree
with the AI's supply rule off (`FFH_AI_SUPPLY_OFF=ALL`: the rules of supply lines and the tactic
rule, the AI ignoring supply). Totals over the 8 games; population and military units are the
per-civ means at turn 250. Time a turn: `ffh bench` (standard map, 250 turns, the machine to
itself), seeds 1, 3 and 5, two rounds each, the mean of the six runs; the games differ between the
sides, so it says "about this much", no more.

| | a46576a | this tree, AI rule off | this tree |
|---|---|---|---|
| cities captured, civ from civ | 70 | 88 | 79 |
| per game | 3 14 3 18 15 3 5 9 | 6 5 20 25 6 10 5 11 | 7 5 5 25 16 11 5 5 |
| cities captured, civ from barbarians / barbarians from civ | 19 / 4 | 14 / 2 | 12 / 2 |
| wars declared / peace | 48 / 37 | 44 / 33 | 46 / 37 |
| eliminations | 3 | 3 | 2 |
| population a civ, turn 250 | 56.3 | 54.6 | 54.2 |
| military units a civ, turn 250 | 43.2 | 41.2 | 41.9 |
| time a turn (`ffh bench`) | 16.3 ms | 17.3 ms | 18.1 ms |

The conquests did not collapse: 79 cities taken from civs against 70 before supply lines, with as
many wars. The AI's rule costs some of them against the same rules without it (79 against 88; seed
3: 5 against 20), which is what a rule that stops marches beyond the line does; the spread between
games (3 to 25) is larger than either difference, so 8 games do not say more than that. Population
and military are within a few percent. The time: about +6% for the rules (the line traced for every
army each turn, and the tactics' pass) and about +5% more for the AI's rule (its path and line
searches).

**Hunger and Starvation** (telemetry `supply_state`, one count per army per turn of its owner, the
army's counter that turn; the same 8 games, this tree with the AI's rule off and on):

| | AI rule off | AI rule on |
|---|---|---|
| army-turns | 30,528 | 30,529 |
| hungry (counter 1-2) | 67 (0.22%) | 51 (0.17%) |
| starving (counter 3+) | 346 (1.13%) | 228 (0.75%) |
| episodes (army-turns at counter 1), per game | 1 2 6 2 4 2 0 15 (32) | 1 2 17 2 2 2 0 4 (30) |
| army-turns at counter 3, 10, 20, 30+ | 32, 11, 4, 140 | 15, 5, 5, 86 |
| longest episode per game (turns; 30 = 30 or more) | 12 1 4 9 30 30 0 14 | 7 1 24 2 4 30 0 21 |

(A split army takes its counter and a merged one the higher, so an episode can start above 1:
counts at a counter are army-turns, not episodes.) With the rule, fewer armies reach Starvation (15
army-turns at counter 3 against 32) and fewer stay cut for 10 turns (5 against 11). What is left of
the long episodes are armies that cannot move home at all: the 30+ cut army-turns of seed 6 are two
of the Kuriotates' armies (a lone archer, and 7 centaurs with an archer) on a plot of theirs from
which they reach no other plot (`approach ... reach 1` in the trace) and that has no line, for the
last 50 turns of the game. The AI's rule fired 46 times to fall back, 9 times to go on hungry next to the
city it attacked, and refused 42 moves beyond the line (telemetry `supply`). Units lost to
Starvation: 0 (it never kills).

## 8. Not in this change

- **Raiding** (the owner's discussion, 2026-11-09: recon and light mounted parties that refuse
  battle, live off plunder, and a standing raid order; pillage that disables). Supply lines give
  raiders their target; the raiding rules are their own design.
- **An army size cap and one army a plot** (the answer to the stack of doom): its own design.
- **Starving a besieged garrison.** A city supplies whoever stands in it.
- **Sea lanes.** A ship is a source while its water area touches a home port; the sea route
  itself is not traced or cut.
