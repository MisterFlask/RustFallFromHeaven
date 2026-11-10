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
- The known line ("Second pass", A) is `Game::plot_supply_line_known(p, plot, viewer)`, the same
  search with the viewer's knowledge (`Plot::revealed`, `plot_is_visible`, `unit_is_invisible`,
  `city_is_revealed`; a ship's coastal city counts only if revealed); who is an enemy, owners,
  routes, improvements and a city's occupation are the truth. Not yet used by the client or the AI.

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

### Measured 2026-11-10: the second pass (B built, C removed)

The 8 games of the block above (seeds 1-8, 6 civs, standard map, 250 turns; `ffh stats` with
`FFH_TELEMETRY=1` and an events file) on the tree with B's four fixes (the bump keeps a land unit
connected to home, animals are not army units, the overseas pickup, the last resort; the
`supply_state` counter uncapped) and without C's line-cutting. The games give the same numbers as
C's measurement (whose code never fired on seeds 1-8; `ffh bench` of seeds 1, 3 and 5 gives the
same checksums for both), so the change from the block above is B's two rules fixes. Time a turn: `ffh bench` (the machine to itself), seeds 1, 3
and 5, two rounds each, the mean of the six runs, against `f5558d91` (the last commit with C's
code; the same games).

| | first build (block above, AI rule on) | this tree |
|---|---|---|
| army-turns | 30,529 | 26,444 |
| hungry (counter 1-2) | 51 (0.17%) | 27 (0.10%) |
| starving (counter 3+) | 228 (0.75%) | 14 (0.05%) |
| distinct armies ever cut | - | 16 (2 2 4 3 4 0 0 1) |
| longest episode per game (turns) | 7 1 24 2 4 30+ 0 21 | 3 1 3 1 4 0 0 8 |
| cities captured, civ from civ | 79 (7 5 5 25 16 11 5 5) | 55 (2 7 6 11 3 16 5 5) |
| wars declared / peace | 46 / 37 | 48 / 42 |
| eliminations | 2 | 1 |
| time a turn (`ffh bench`) | 18.1 ms | 16.9 ms (`f5558d91`: 17.0 ms) |

How often B's fixes fired in these games (telemetry `supply`): overseas pickups requested 0,
transports sent 0, armies boarded 0; armies disbanded as the last resort 0; armies cut overseas with
no port they may enter 0. The bumps the connected rule redirected are not counted (the rules have
no telemetry for them, and ffh-core was left as it is). The AI's supply rule fired 31 times to fall
back (17 hungry, 14 starving), 8 times to go on hungry next to the city it attacked, and refused 7
moves. With the long episodes of the first build gone (the peace bump of seed 6, the Wolf Pack of
seed 8: B's rules fixes; seed 3's army left overseas does not arise in this tree's seed 3), nothing
was left for the AI's two fixes to do on these seeds.

**Seed 3's case, replayed.** The save of the diagnosis (seed 3 at turn 200, made by the first build)
resumed for 50 turns with this tree's binary (`ffh resume`, trace on): the Balseraphs assault
transport lands its army next to the Clan of Embers city at (43, 35) as before, the two at peace
with open borders. Cut, the army walks into that city (turn 206) and waits there for a ship; while
the transport stands beside it the army is supplied (the ship is a source) and the request lapses,
and while the Balseraphs keep their amphibious target there (a prepared war) the transport goes on
ferrying troops; at turn 223 the empty transport sails for it, the army (an Archer and a Warrior)
boards at turn 225 and is landed at home at turn 227, and the transport goes back for the Warrior
left over (aboard at turn 230). Before: cut from turn 216 to 244, starving.

### The budget: measured variants (not adopted)

For the owner's decision; the committed defines are unchanged. The same 8 games (seeds 1-8, 6
civs, standard map, 250 turns, this tree's binary), with `SUPPLY_LINE_BUDGET` 24 (as committed), 16
and 12, and 24 with `SUPPLY_COST_LAND` 4 (the defines changed in a copy of the data, read through
`FFH_DATA_DIR`). "Distinct armies ever cut": armies with at least one army-turn cut (events file).

| | budget 24 (committed) | budget 16 | budget 12 | budget 24, open land 4 |
|---|---|---|---|---|
| army-turns | 26,444 | 27,645 | 25,842 | 26,863 |
| hungry (counter 1-2) | 27 (0.10%) | 155 (0.56%) | 126 (0.49%) | 62 (0.23%) |
| starving (counter 3+) | 14 (0.05%) | 572 (2.07%) | 54 (0.21%) | 31 (0.12%) |
| distinct armies ever cut | 16 | 87 | 79 | 43 |
| per game | 2 2 4 3 4 0 0 1 | 10 22 26 7 9 2 0 11 | 14 14 10 9 15 5 5 7 | 0 6 13 7 3 5 0 9 |
| longest episode per game (turns) | 3 1 3 1 4 0 0 8 | 9 6 93 5 7 1 0 40 | 17 3 3 3 7 2 4 12 | 0 5 1 4 6 4 0 10 |
| moves refused beyond the line | 7 | 148 | 192 | 34 |
| falls back (hungry / starving / overseas) | 17 / 14 / 0 | 77 / 45 / 14 | 64 / 34 / 0 | 33 / 30 / 0 |
| pickups requested / armies disbanded (last resort) | 0 / 0 | 3 / 3 | 0 / 0 | 0 / 0 |
| cities captured, civ from civ | 55 | 65 | 67 | 72 |
| per game | 2 7 6 11 3 16 5 5 | 8 15 6 13 8 6 5 4 | 13 14 5 12 6 8 6 3 | 2 19 5 8 10 19 5 4 |
| wars declared / peace | 48 / 42 | 44 / 33 | 44 / 29 | 41 / 29 |
| eliminations | 1 | 1 | 2 | 2 |

What the numbers plainly show: a tighter line cuts many more armies (16 at 24; 87 at 16, 79 at 12,
43 with open land at 4) and refuses many more moves, but armies stay cut briefly: hungry and
starving together stay under 3% of army-turns in every variant. Conquest did not fall with a
tighter line: captures from civs rose in every variant (55 to 65, 67, 72), with a spread between
games larger than the differences. Most of the 16 budget's starving army-turns (400 of 572) are 4
one-unit Clan of Embers armies (Archers, the units' default AI a city defender, so played as
garrisons and never by the fall-back rule) that stood in foreign cities, a Balseraphs city and a
barbarian one (the Clan's barbarian peace), cut from turn 158 to the end of seed 3; and one Malakim
army of seed 8 (40 turns, not looked at). The three overseas armies of that game (one unit each) walked to
foreign ports they may enter, asked for a ship, got none (the Clan had no transport and built none in time) and were
disbanded at turn 169.

## 8. Not in this change

- **Raiding** (the owner's discussion, 2026-11-09: recon and light mounted parties that refuse
  battle, live off plunder, and a standing raid order; pillage that disables). Supply lines give
  raiders their target; the raiding rules are their own design.
- **An army size cap and one army a plot** (the answer to the stack of doom): its own design.
- **Starving a besieged garrison.** A city supplies whoever stands in it.
- **Sea lanes.** A ship is a source while its water area touches a home port; the sea route
  itself is not traced or cut.

## Second pass (design, 2026-11-10; B built; C built, measured and not kept)

Four fixes to what the first build left rough. Each is decided; the implementation reports what
it measured.

### A. What a player knows of a line

The rules' line (`Game::plot_supply_line`) is the truth: it bends around every enemy unit, seen or
not. Shown to a player it leaks: the shape of another army's line, or the "supplied here?"
readout probed plot by plot, tells where unseen units stand.

- A second, pure search, `Game::plot_supply_line_known(p, plot, viewer: TeamId)`: the same search
  with the viewer's knowledge in place of the truth. It may enter only plots the viewer has
  revealed; a unit blocks only if the viewer sees it now (its plot visible to the viewer's team
  and the unit not invisible to it); a fort denies its neighbours only if the viewer sees a unit
  in it; a source is a city the viewer has revealed, or a ship the viewer sees. The costs, the
  budget and the tie rule are the rules' own. `viewer` may be `p`'s own team.
- **An army's own state stays the truth** for its owner: its counter, its Hunger and Starvation,
  and the line drawn for it are the rules' (a quartermaster knows the wagons stopped; that
  something unseen cut the line is information the owner is owed).
- **The "supplied here?" readout** before a move uses the known search with the player's own team
  as viewer, so it answers from what the player sees. It can be wrong; the text does not change.
- **Other players' lines** are drawn from the known search with the human's team as viewer: what
  the human can work out, not what is. An army whose known line does not exist shows none.

### B. An army with no way home

Diagnosed (2026-11-10; seeds 1 to 8 of "Measures", every cut army watched every turn; notes in
the session's scratchpad, `stranded_notes.txt`). Three causes, none of them "a pocket of its own
land":

1. **The peace bump** (seed 6: three Kuriotates armies, 57 to 58 turns each, 70% of all the
   starving army-turns of the 8 games). When peace is made, `map_verify_unit_valid_plot` moves
   every unit standing where it may no longer be with `unit_jump_to_nearest_valid_plot`, which
   scores plots by distance only. It put the armies on unowned plots closed in by two neighbours'
   borders (peace, no open borders) and the coast. In Civ4 that costs nothing; here the army can
   reach no plot, traces no line, and starves for the rest of the game.
2. **An army left overseas** (seed 3: a Balseraphs army, 24 turns). Landed by an assault
   transport next to an enemy city; after the peace it stands on a landmass without a city of its
   owner. It can move, but the AI's `fall_back` gives up when no city of ours is on its landmass,
   and nothing sends a ship.
3. **A civilization's animal counted as an army** (seed 8: a Doviello Wolf Pack, 21 turns). The
   army sync makes every group of army units outside a city an army; the AI plays the unit as an
   animal (it wanders and attacks) and never runs the supply code for it.

The fixes, decided:

- **Rules: a bump keeps a land unit connected to home.** When a unit of a player that has a city
  on the unit's landmass is moved by `unit_jump_to_nearest_valid_plot`, only plots that connect
  over land to a city of its owner are taken: plots the unit may enter, flooded from the owner's
  cities on that landmass once per bump. With no such plot, it goes to its owner's nearest city.
  A player without a city there is bumped as before. (docs/PORTING.md, "Changes from FFH2".)
  *Built (2026-11-10):* `Game::unit_jump_to_nearest_valid_plot` floods, per bumped unit, from its
  owner's cities on its area through the plots the bump calls valid, and takes the nearest of
  those (else the nearest city); tests in `crates/ffh-core/tests/bump.rs`.
- **Rules: animals are not army units.** A unit whose type is an animal (`UnitInfo::animal`) is
  in no army and no garrison, so it needs no supply line: it fights beside an army as heroes and
  mages do. (docs/ARMIES.md; docs/PORTING.md.)
  *Built (2026-11-10):* `Game::unit_type_is_army_unit` refuses an animal type; the army sync takes
  an animal out of its army; test `a_wolf_pack_is_in_no_army` (`crates/ffh-core/tests/armies.rs`).
- **AI: stranded overseas.** An army cut on a landmass without a city of its owner goes to the
  coast plot nearest its owner's nearest city and asks for a transport there, as a settler waiting
  in port does; the transport that carries an army ashore does not leave it while no war target
  remains there.
  *Built (2026-11-10), in a port, not on a coast plot:* the rules let only a human's land unit step
  onto a ship at sea (`unit_can_move_into`, an AI-only branch of the C++), so an AI army boards
  only on a ship's own plot, a city. (Since docs/NAVAL.md, phase 1, any player's land unit may step
  onto water, in boats or aboard a ship there: an AI army boards a transport beside it at sea, and
  before asking for a ship it walks home in boats when the way is in range and no enemy warship it
  sees is within 3 plots.) `supply::fall_back`, on a landmass without a city or settler of
  ours, walks the army to the coastal city it may enter (another team's, open borders, a vassal or
  its master; on a sea one of our ports is on) nearest our nearest city, and there asks for a ship
  (the group's mission AI `MISSIONAI_PICKUP` on its plot); `naval::pickup` sends the empty transport
  that reaches it soonest (one per army; before the assault's staging port and a waiting settler),
  which waits in that port while the army boards, then carries it home (the transport's "cargo
  with no plan" path); a port of ours builds a transport when we have none (`production::decide`,
  `army_ship`). An army overseas with nothing left to fight there boards a ship of ours on its plot
  even while that ship supplies it. With no such port in reach it asks for nothing. Tests in
  `crates/ffh-ai/src/supply.rs`.
- **AI: the last resort.** An army that has starved for `STRANDED_TURNS` (10, an ffh-ai constant)
  of its owner's turns in a row, has no path over land to a city of its owner, and (overseas) has
  had no transport come, is disbanded unit by unit (`unit_scrap`; heroes and units that cannot be
  scrapped excepted): an expert stops paying for units he can neither feed nor fetch. Counted in
  telemetry. Asking the neighbour for open borders, or counting a trapped army as a reason for
  war, is not in this change.
  *Built (2026-11-10):* in `supply::fall_back`, at a counter of `SUPPLY_HUNGER_TURNS` +
  `STRANDED_TURNS` or more: on its own landmass when no city of ours there has a path for it (all
  of them searched), overseas when it cannot board and no transport of ours is on its way to it
  (`MISSIONAI_PICKUP` on its plot); the army's units in the group are scrapped (`unit_scrap`), but
  heroes, units of a limited class and units the rules will not scrap. Telemetry `supply`
  `disband_unit` / `disband_army` (`land`, `overseas`). Tests in `crates/ffh-ai/src/supply.rs`.
- **Telemetry:** the `supply_state` counter is not capped at 30.
  *Built (2026-11-10):* `cut_NN` is the counter itself (at most one label per turn of a game).

### C. The AI cuts a line (built, measured, not kept)

The design: each turn, for at most two enemy armies at war with us that we see within 6 plots of
our cities, their known line (A, our team as viewer); a cut plot is a plot of it (not next to the
army, not a city; at most six tested from the source end) that a local search over the 7x7 plots
around it finds no way around within the budget; a spare of a garrison that keeps three fighters
(recon and mounted first) and reaches it within two turns, where the danger map is within its
defence, goes there and fortifies, and goes home when the army leaves the 6 plots, our sight or
the war. Built in `crates/ffh-ai/src/supply_cut.rs` (switch `SUPPLY_CUT`, telemetry `supply_cut`).

**Measured** (2026-11-10; the settings of "Measures", the switch off against on, the same binary;
B's two rules fixes in): on seeds 1-8 it never fired (each game identical on both sides). Over
seeds 1-32 it sent 12 units in 7 games (all after turn 190); the line was gone on arrival 3 times;
2 units were killed on their plot; the watched army became hungry within 3 turns once. Off / on:
hungry 130 / 132 army-turns (0.13%), starving 389 / 389 (0.38%), cities captured from civs 298 /
296: nothing measurable.

**Not kept (2026-11-10)**, the designer's decision: the lines of enemy armies near our cities are
short and far inside the budget (median 4 plots; 94% with a slack of 10 or more of the budget of
24), so one unit cannot cut them; a line is cut by one plot only at a choke point or near the
budget. The local bypass test also mispredicted 9 of its 12 cuts. The code was removed; the last
commit that contains it is `f5558d91` (docs/AI_LOG.md).
