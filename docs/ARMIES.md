# Armies (`GAMEOPTION_ARMIES`)

This game's own rules, not FFH2's: the second system of the owner's design spec "FfH Culture
Actions & Army Combat" (2026-10-29), "Armies and recruitment", with the owner's redesign of
recruitment (2026-10-31): "armies (as directed by the player) requisition soldiers from cities,
which interrupts normal production", and the recruits "just appear as normal but are
uncontrollable: they just go that direction". Land combat units serve in named armies or in a
city's garrison; an army requisitions soldiers from the cities its owner picks for it, and a unit
bound elsewhere marches there on the map. This file says what was built, how the spec's open points
and edge cases were decided, and what is left.

| Phase (spec) | What | Status |
|---|---|---|
| 4 Armies and recruitment | armies, garrisons, build destinations, supply, form / split / join / merge / garrison / rename / disband / upgrade all, target size, AI, UI | done |
| 5 Governors | (the "Feed army" mode, replaced by requisition on 2026-10-31) | done |
| Requisition and marching recruits (2026-10-31) | rules, saves | done; AI and UI next |
| 6 Engagement combat | stance, commander, S-unit engagements | after the owner's review of the promotion inventory |
| 7 FfH layer | Incite Mutiny (cultural action against an army), commander auras | later |

Code: rules `crates/ffh-core/src/army/` (`mod.rs`: armies, garrisons, the sync, the commands,
supply, destinations; `requisition.rs`; `march.rs`; hooks: the production queue's train order and
`city_push_order` in `city/turn.rs`, the player turn and its units' turn in `player/turn.rs`,
`unit_convert` and `unit_kill` in `unit/lifecycle.rs`, `city_kill`, the controller's
`requisition_unit`); tuning `data/xml/RulesDefines.xml` (`ARMY_*`); AI `crates/ffh-ai/src/armies.rs`,
`governor.rs` (`requisition_unit`); UI `crates/ffh-play/src/view/armies.rs`, the unit panel's army
actions (`view/units.rs`), the city query's `recruit` and its queue's `dest`,
`assets/app/armies.js`. Tests: `crates/ffh-core/tests/armies.rs`, `crates/ffh-ai/src/armies.rs`;
the harness checks that a marching unit is in no army and that requisition orders lead their
queue (`ffh_harness::check`).

## The option

`GAMEOPTION_ARMIES` is a visible game option with `bDefault` 1, like the cultural actions: the
browser's new-game screen starts with it ticked, and everything else (`ffh autoplay`, `ffh stats`,
the AI batches, soaks) plays without it unless asked (`--option GAMEOPTION_ARMIES`). Off, nothing of
this runs: no unit has an army or marches, no army requisitions, the save carries empty lists.

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

**Garrisons.** An army unit in one of its owner's cities, in no army and not marching, is in that
city's garrison. The garrison and every army on the plot defend the city together (Civ4 already
picks the defender among every unit on the plot; a marching recruit on the plot defends too).

**The army sync.** Units change groups for many reasons (the human's Group and Split buttons, a
unit's move, the AI's grouping), so after every human command, after each army command and in each
player's turn processing (Civ4's doTurn) the armies are settled with the groups: each group of army
units has one army; the oldest group holding units of an army keeps that army, so a part that split
off becomes a new army; a group whose units had none gets a new one (a garrison group stays the
garrison); an army without units is gone. Marching recruits are left out: they are in no army
until they arrive.

**Commands** (the unit panel's Army section; each one click):

- *Form a new army* of the selected units (from armies or a garrison).
- *Split into a new army*: some units of one army.
- *Join the N-th Army*: the selected units join an army on their plot (garrison units included);
  *Merge into* when the whole army is selected. The merged army's recruits on the way, its
  requisition (its cities join the other's list) and production orders, and the cities' defaults
  naming it go to the army it joined.
- *Join the garrison* of the city they stand in.
- *Disband*: the units march to the nearest city (a go-to order) and join its garrison when they
  arrive; its recruits on the way go to that garrison, its requisition ends (its orders are
  withdrawn). In a city it is *Join the garrison*.
- *Upgrade all*: each unit takes its best upgrade (the strongest, then the dearest), the most
  experienced first, while the gold lasts.
- *Rename*, *Target size* and *Requisition from* (the Armies screen): the size the army is meant
  to have, the cities it requisitions from.

**Recruitment.** Every production order for an army unit has a destination (`OrderData::dest`):
the city's default (`City::recruit_default`, itself by default the city's own garrison), the
city's garrison, another city's garrison, or an army. To the city's own garrison the unit stays, as
without armies. Otherwise it is created in the city as any unit and marches there (below). An army
standing in the producing city takes the unit at once. Governed cities' units stay in their
garrison (docs/GOVERNORS.md); their only orders bound elsewhere are requisitions.

**Requisition** (`army/requisition.rs`). An army requisitions soldiers from the cities its owner
picks for it (`Army::requisition`, in the order they were added): any of the owner's cities,
directly ruled or governed, the capital included. A city lost, traded, razed or flipped leaves
every list. Requisition needs a target size: an army with none (0) requisitions nothing.

- *The shortfall*: the target size less the army's units, its recruits marching to it, its
  requisition orders in production and the other orders bound for it at the head of a city's queue
  (the first order that is not a requisition: what the city builds next, its destination or the
  city's default naming the army).
- *Once a turn*, in the owner's turn processing before its cities produce (so a new requisition
  takes this turn's production): orders without a reason any more (the army gone or disbanding,
  the city released) are withdrawn; then for each army, in formation order, while its shortfall is
  positive, a picked city without a requisition order for it gets one, one unit per city at a
  time, the city whose unit would join soonest first (its production turns there plus its march,
  estimated over land; a city out of reach by land last; ties: the order the cities were picked
  in). Orders beyond the shortfall (the army filled up another way, the target lowered) are
  withdrawn, those that would join last first. What was put into a withdrawn order stays with the
  unit type, as for any order taken off a queue.
- *The order* is a train order at the front of the city's queue, flagged as a requisition
  (`OrderData::requisition`, its destination the army): it interrupts what the city was building,
  which keeps its progress and resumes after it. Requisition orders stay at the front: an order
  pushed to the front goes behind them, and replacing what a city builds replaces the first order
  behind them. The player cannot take a requisition order off the queue by hand (the city screen
  says why: release the city from the army's requisition on the Armies screen); a governed city's
  queue is the governor's anyway, and the requisition goes ahead of the governor's choice.
- *The unit* is a controller decision (`Controller::requisition_unit`); the rules' default, the
  human's: siege (`UNITCOMBAT_SIEGE`) while siege is less than a third of the army (its recruits
  marching to it and its requisition orders counted), else the strongest unit against cities (a
  unit that can take the city attack role: strength times its city attack bonus), else the
  strongest attacker; only army units the city can train (ties: the cheaper). A picked city that
  can train none is skipped that turn. The AI's: the governors' old feeding choice
  (`ffh-ai governor::requisition_unit`).

**Marching recruits** (`army/march.rs`). A unit bound elsewhere is created in the producing city as
a normal unit (build effects, experience, promotions, upkeep, unit class limits: all as for any
unit) and marches (`Unit::march`: its destination, the turns without progress, the turn it left,
the city it came from, whether it was diverted): the rules move it, not its owner.

- *Movement*: once a turn as its owner's turn begins, after its units' turn gave them their moves
  and before the controller plays (so neither the human nor the AI moves it), it moves with its
  moves along the rules' path (as a go-to order would: terrain, roads, borders, war; recomputed
  every step) to where its destination is then (an army's plot, recomputed every turn; a garrison's
  city). It never attacks and never declares war: a step that would be an attack, or that it
  cannot simply move into, is not taken, and the path goes around visible enemy units when it
  can. It then holds for the rest of the turn. Deterministic: the units in their owner's order, the
  path finder's own tie-breaks.
- *Arrival*: in the destination city it joins the garrison (in no army). An army takes it when it
  stands on the army's plot as its march begins (the army came to it) or at the end of its owner's
  turn processing (before the army sync), so that a recruit that reached the army with its moves
  spent never holds the army back for the rest of the turn; if the army moved on meanwhile, it
  marches on. Either way it is an ordinary unit from then on. A message each.
- *Supply gates progress*: a recruit bound for an army advances only while the army is in supply;
  otherwise it waits where it is.
- *No way there*: a turn without progress (the army out of supply, no path, the way blocked, it
  cannot move) counts; after `ARMY_TRANSIT_STALL_TURNS` (5) such turns in a row it marches to the
  nearest city of its owner it can reach by the rules' path (not the garrison it stalled on) and
  joins its garrison on arrival. A recruit already diverted once that stalls again for as long, or
  one with no city it can reach, stops marching where it stands and is an ordinary unit again
  (a message), so no recruit stays out of its owner's hands for ever.
- *Destination gone*: the army destroyed (or emptied into a garrison) → at once to the nearest
  garrison it can reach; the army disbanded → to the city the army marches to; the army merged
  into another → to that army; the destination city lost → the nearest other garrison.
- *A real unit*: it can be attacked and defends itself; it can die on the way (a message, the
  telemetry counts it); an upgrade or a conversion that keeps its owner keeps its march. It is in no
  army and no garrison while marching (the army sync leaves it out, army commands refuse it).
- *Uncontrollable*: the rules say so (`Game::unit_march`, `unit_march_refusal`: "marching to the
  2nd Army"); the human's command layer refuses its orders and the AI leaves it alone. It can be
  selected to look at.
- Messages: setting out ("Ithralia's Axeman marches to the 2nd Army."), waiting (the first turn of a
  stall, with why), diverted, arrived, stopped, killed on the way. Telemetry: `march` (`sent`,
  `arrived` with the turns, `diverted`, `stopped`, `killed`) and `requisition` (`placed`,
  `withdrawn`), docs/TELEMETRY.md.

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
  keeps its name and takes the other's recruits, requisition and orders), splitting a group splits
  its army. So no unit is ever in two places, and nothing a Civ4 player is used to stops working.
- **No path to an army is a wait, not a loss.** An army aboard ships has no land path for a few
  turns, and diverting at once would throw away every recruit at each crossing; so every stall
  waits `ARMY_TRANSIT_STALL_TURNS` turns, for an army or a garrison alike.
- **A destroyed army's orders fall back** to their city's default (and a city default naming it
  to the city's own garrison), so a city never trains for an army that is gone; its requisition
  orders are withdrawn.
- **Repeat and requisition.** Civ4's repeat flag (`save`) keeps an order's destination when it
  comes back; a requisition order never repeats (the army requisitions again while it is short).
- **Requisition orders lead the queue**, whatever pushes orders: the C++ "push to the front" and
  "replace what is built" (`pushOrder`'s `bAppend` false and `bPop`) act behind them, so neither
  the player's city screen nor an AI's or governor's choice displaces one; only the requisition
  withdraws them.
- **One requisition order per city and army.** A city picked by two armies serves both (each order
  at the front, in the order they were placed); an army takes one unit from a city at a time, so a
  fast city does not fill it alone while others stand idle.
- **A recruit that reaches its army with its moves spent joins at the turn's end.** Joining at once
  would leave it without moves inside the army's group, and the army's next move this turn would
  split it off as a new one-unit army (Civ4 moves a group only with the units that can move).
- **A diverted recruit stalls once.** A second stall of `ARMY_TRANSIT_STALL_TURNS` turns (or no
  city it can reach) ends its march where it stands: an ordinary unit, its owner's again, rather
  than a recruit hopping between blocked cities out of its owner's hands.
- **Upgrades and conversions keep the march** when the unit keeps its owner; a unit that changes
  hands (an FfH domination) is lost on the way (counted as killed).

## Open points for the owner

- **Supply follows roads.** Civ4's plot groups need a road or a river even inside one's borders,
  so an army three plots beyond the last road is out of supply in its own territory. That is what
  the spec's "connected to one of its owner's cities" means in Civ4 terms, and it makes roads
  matter; widening it to "any plot inside one's borders" is one line if the owner prefers.
- **A recruit marches alone.** Recruits bound for one army march each on its own path; marching
  together (a convoy that defends itself better) would be a next step if lone recruits are
  picked off too often.

## AI (summary; docs/AI.md "Armies")

The AI's groups become armies by themselves through the sync, so its movement logic drives each
army unchanged. What it adds is logistics: at war or preparing one, its cities send the attack
units they train (the attack, city attack, collateral and counter roles) to its main field army
(the largest army of at least two units in supply, the nearest to the enemy on ties): they march
to it. It sets that army's target size to the army the war needs. Defenders and everything else
stay in their city's garrison; a defender that walked to another of its cities joins that city's
garrison when it arrives. The unit a city trains for a requisition is the governors' old feeding
choice (`governor::requisition_unit`). Which cities its armies requisition from, and leaving
marching recruits alone in its unit play, are next (the AI's part of the 2026-10-31 redesign).

## UI (ffh-play)

(Before the 2026-10-31 redesign; the requisition and marching recruits come next.)

- **Unit panel.** Each unit's row shows its army (amber; red when out of supply) or "garrison".
  With a whole army selected, a line gives its size and target, its recruits on the way and its
  supply, with a link to the Armies screen. The Army section lists the commands above that the
  selection can take, each with its help.
- **Armies screen** (F5, the menu's Armies): every army with its name (editable), units, place
  (a click selects it on the map), supply, target size, Upgrade all and Disband, and the recruits
  on their way (the view's `transit`, now the marching units: where they come from, how many turns
  they need, or how long they have waited); then every city's garrison and the recruits on their
  way to it.
- **City screen.** Each order for an army unit in the production queue has a destination selector
  (the city's default, this city's garrison, another city's garrison, an army with its size); a
  requisition order is flagged (`requisition`). Under the queue, the city's default destination and
  its garrison.
- **Messages**: a recruit setting out, waiting, diverted, arrived, stopped and killed on the way; a
  requisition placed or withdrawn.
