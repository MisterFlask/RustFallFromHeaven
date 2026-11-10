# Supply lines

**Status: rules built (2026-11-10: sections 2 to 4, `crates/ffh-core/src/army/supply.rs`, tests in
`crates/ffh-core/tests/supply.rs`); the client (section 5) and the AI (section 6) next.** This file
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

## 7. Measures the implementation reports

With the AI rule in, against the commit before (the tools of docs/AI.md; 8 games of 6 civs, 250
turns): the share of army-turns hungry and starving, units lost to nothing (Starvation never
kills: 0), cities captured from civs and wars declared (the AI's conquests should not collapse:
report the change, do not tune the rules to hide it), time a turn. And a staged test of each rule
of sections 2 and 3.

## 8. Not in this change

- **Raiding** (the owner's discussion, 2026-11-09: recon and light mounted parties that refuse
  battle, live off plunder, and a standing raid order; pillage that disables). Supply lines give
  raiders their target; the raiding rules are their own design.
- **An army size cap and one army a plot** (the answer to the stack of doom): its own design.
- **Starving a besieged garrison.** A city supplies whoever stands in it.
- **Sea lanes.** A ship is a source while its water area touches a home port; the sea route
  itself is not traced or cut.
