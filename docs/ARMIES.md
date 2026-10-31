# Armies (`GAMEOPTION_ARMIES`)

This game's own rules, not FFH2's: the second system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), "Armies and recruitment". Land combat units serve in named
armies or in a city's garrison; a city sends the units it trains to a destination, and they travel
there off the map. This file says what was built, how the spec's open points and edge cases were
decided, and what is left.

| Phase (spec) | What | Status |
|---|---|---|
| 4 Armies and recruitment | armies, garrisons, build destinations, transit, supply, form / split / join / merge / garrison / rename / disband / upgrade all, target size, "Reinforce to N", AI, UI | done |
| 5 Governors | "Feed army" mode filling an army's target size | next |
| 6 Engagement combat | stance, commander, S-unit engagements | next: the owner answered the inventory (docs/COMBAT_INVENTORY.md, 2026-10-31) |
| 7 FfH layer | Incite Mutiny (cultural action against an army), commander auras | later |

Code: rules `crates/ffh-core/src/army.rs` (hooks: the production queue's train order in
`city/turn.rs`, the player turn in `player/turn.rs`, unit upkeep in `player/economy.rs`,
`unit_convert`); tuning `data/xml/RulesDefines.xml` (`ARMY_*`); AI `crates/ffh-ai/src/armies.rs`;
UI `crates/ffh-play/src/view/armies.rs`, the unit panel's army actions (`view/units.rs`), the
city query's `recruit` and its queue's `dest`, `assets/app/armies.js`. Tests:
`crates/ffh-core/tests/armies.rs`, `crates/ffh-ai/src/armies.rs`.

## The option

`GAMEOPTION_ARMIES` is a visible game option with `bDefault` 1, like the cultural actions: the
browser's new-game screen starts with it ticked, and everything else (`ffh autoplay`, `ffh stats`,
the AI batches, soaks) plays without it unless asked (`--option GAMEOPTION_ARMIES`). Off, nothing of
this runs: no unit has an army, every trained unit appears in its city, the save carries empty
lists.

## Rules

**Army units.** Land units with a strength, except mages (`UNITCOMBAT_ADEPT`), recon units
(`UNITCOMBAT_RECON`), heroes (`PROMOTION_HERO`), summons (a duration) and the barbarians' units.
Ships, air units, workers, settlers and missionaries are not army units either. The others move on
their own, or along with an army's group (a hero travelling with an army is not in it: the
commander slot is phase 6).

**Armies.** An army is a persistent, named container (`Army`, `Unit::army`): "1st Army", "2nd
Army", ... (the lowest free number), renamed at will. Its units are kept in one selection group, so
it moves as one at the speed of its slowest member, as a Civ4 group does; several armies may share
a plot. An army unit on its own outside a city (after a split, a move out of a city) is a one-unit
army.

**Garrisons.** An army unit in one of its owner's cities and in no army is in that city's garrison.
The garrison and every army on the plot defend the city together (Civ4 already picks the defender
among every unit on the plot).

**The army sync.** Units change groups for many reasons (the human's Group and Split buttons, a
unit's move, the AI's grouping), so after every human command, after each army command and in each
player's turn processing (Civ4's doTurn) the armies are settled with the groups: each group of army
units has one army; the oldest group holding units of an army keeps that army, so a part that split
off becomes a new army; a group whose units had none gets a new one (a garrison group stays the
garrison); an army without units is gone.

**Commands** (the unit panel's Army section; each one click):

- *Form a new army* of the selected units (from armies or a garrison).
- *Split into a new army*: some units of one army.
- *Join the N-th Army*: the selected units join an army on their plot (garrison units included);
  *Merge into* when the whole army is selected. The merged army's reinforcements, production
  orders and city defaults go to the army it joined.
- *Join the garrison* of the city they stand in.
- *Disband*: the units march to the nearest city (a go-to order) and join its garrison when they
  arrive; its reinforcements go to that garrison. In a city it is *Join the garrison*.
- *Upgrade all*: each unit takes its best upgrade (the strongest, then the dearest), the most
  experienced first, while the gold lasts.
- *Rename* and *Target size* (the Armies screen): the size the army is meant to have.

**Recruitment.** Every production order for an army unit has a destination (`OrderData::dest`):
the city's default (`City::recruit_default`, itself by default the city's own garrison), the
city's garrison, another city's garrison, or an army. To the city's own garrison the unit appears at
once, as without armies. Otherwise it does not appear on the map: it leaves *in transit*
(`Player::transit`) and appears at its destination when its journey is done, with its city's build
effects (experience, promotions, the free XP buildings) if that city is still its owner's.

- *ETA*: the path from the producing city to the destination over land, at the unit's movement
  (terrain, hills, forests, roads; other units, borders and war ignored), recomputed every turn
  from the city to where the destination is then: an army that marches away takes longer to reach.
- *Supply gates progress*: a unit bound for an army advances only while the army is in supply.
- *An army in the producing city* takes the unit at once.
- *Never lost*: if the destination is gone (the army destroyed or disbanded, the city lost) or a
  garrison cannot be reached by land, the unit goes at once to the garrison of its owner's nearest
  city (one reachable by land first); if its army is out of supply or out of reach by land (aboard
  ships, across the water), it waits, and after `ARMY_TRANSIT_STALL_TURNS` (5) such turns it goes
  to the nearest garrison too. Each change is a message.
- *Upkeep and limits*: units in transit pay unit and military upkeep and count against their unit
  class's limit (national and world units) until they arrive.
- *"Reinforce to N"*: an order bound for an army with `reinforce_to` N is queued again when it
  completes, while the army (its units plus those on their way) counts fewer than N.

**Supply.** An army is in supply within `ARMY_SUPPLY_RANGE` (3) plots of a plot of its owner's
plot groups that holds one of its cities: Civ4's trade network (roads, rivers, coasts once known;
inside one's borders a plot needs a road or a river, as for resources). So supply follows the road
network: building roads behind an advance extends it, pillaging them or taking a city cuts it, and
enemy territory is never part of one's network (war cuts trade there). Out of supply only
reinforcement pauses: no attrition in v1 (the spec).

## Edge cases decided (beyond the spec's text)

- **The garrison does not move, but its units do.** The spec's garrison "cannot move"; here a
  garrison unit the human moves out of its city leaves the garrison and forms (or with others,
  joins) an army as it leaves. Forbidding the move would only add a click (Form army, then move):
  the move is the transfer.
- **An army entering a city stays an army.** It does not dissolve into the garrison (armies pass
  through cities); *Join the garrison* is one click.
- **Armies are their groups.** Civ4's own Group, Split and Ctrl-click selections still work and
  keep the armies consistent through the sync: grouping two armies merges them (the older army
  keeps its name and takes the other's reinforcements and orders), splitting a group splits its
  army. So no unit is ever in two places, and nothing a Civ4 player is used to stops working.
- **No path to an army is a wait, not a loss.** The spec diverts a unit when "no path exists"; an
  army aboard ships has no land path for a few turns, and diverting at once would throw away every
  reinforcement at each crossing. A garrison has a fixed place, so no path there diverts at once.
- **A destroyed army's orders fall back** to their city's default (and a city default naming it
  to the city's own garrison), so a city never trains for an army that is gone.
- **Repeat and Reinforce.** Civ4's repeat flag (`save`) and "Reinforce to N" do not stack: a
  repeating order comes back once, keeping its destination.
- **Units in transit count for upkeep and limits but not as units on the map**: they cannot be
  killed, captured or seen, and they do not count for power or score until they arrive.

## Open points for the owner

- **Supply follows roads.** Civ4's plot groups need a road or a river even inside one's borders,
  so an army three plots beyond the last road is out of supply in its own territory. That is what
  the spec's "connected to one of its owner's cities" means in Civ4 terms, and it makes roads
  matter; widening it to "any plot inside one's borders" is one line if the owner prefers.
- **Transit is invisible.** Units in transit cannot be intercepted; the spec does not ask for it.

## AI (summary; docs/AI.md "Armies")

The AI's groups become armies by themselves through the sync, so its movement logic drives each
army unchanged. What it adds is logistics: at war or preparing one, its cities send the attack
units they train (the attack, city attack, collateral and counter roles) to its main field army
(the largest army of at least two units in supply, the nearest to the enemy on ties) instead of
walking them there, and it sets that army's target size to the army the war needs. Defenders and
everything else stay in their city's garrison; a defender that walked to another of its cities
joins that city's garrison when it arrives.

## UI (ffh-play)

- **Unit panel.** Each unit's row shows its army (amber; red when out of supply) or "garrison".
  With a whole army selected, a line gives its size and target, its reinforcements on the way and
  its supply, with a link to the Armies screen. The Army section lists the commands above that
  the selection can take, each with its help.
- **Armies screen** (F5, the menu's Armies): every army with its name (editable), units, place
  (a click selects it on the map), supply, target size, Upgrade all and Disband, and the units on
  their way with where they come from and how long they need (or why they wait); then every city's
  garrison and the units on their way to it.
- **City screen.** Each order for an army unit in the production queue has a destination selector
  (the city's default, this city's garrison, another city's garrison, an army with its size) and,
  for an army, "reinforce to N"; under the queue, the city's default destination and its garrison.
- **Messages**: a unit leaving in transit, arriving, waiting (out of supply or out of reach) and
  diverted.
