# Battle tactics

**Status: design (the owner's direction, 2026-11-09: "doctrines before grids"); not built.** This
file is the spec for the implementation. Sections 1 to 6 are decided; section 7 is what the lab
has to show before a tactic ships, and section 9 lists what is left open.

The term is **tactic** (an army's battle tactic). "Doctrine" is taken: it is what players see a
research path as. "Stance" was the removed combat model's word, "formation" is the grid of one
battle (`Place`, `Formation`), "order" is a city's.

## 1. Why

Formation combat (docs/ENGAGEMENTS.md) asks where each unit stands. Today that is decided per
attack: the attacker may drag its units on the battle plan's board, the defender and the AI always
fight in the auto formation. So careful play means arranging a grid before every attack for an
edge, nothing can be prepared for a defence, and most battles are the auto formation anyway.

A tactic moves the decision from the battle to the army. An army has one standing tactic, chosen on
the Armies screen and changed rarely; it says who of the army fights when the slots are few and
where the kinds of units stand. Every battle of that army, attacking or defending, is arranged by
it. The grid of one battle is never edited by hand.

## 2. What a tactic decides

The auto formation makes two choices, and a tactic is a variant of each:

1. **Selection.** When the pool holds more units than the side has slots, which fight. Today:
   one unit of each kind, then the kinds in a fixed order (anti-flanking strikers, strikers,
   shooters, chargers, casters, healers, bombards, stabbers), the healthy and then the strongest
   first.
2. **Placement.** The row and lane of each chosen unit. Today: large units first, each unit in the
   first row of its own rank order with room, the lanes nearest the centre first.

Two words used below. A **guard** is a unit whose kind is a strike or a charge (anti-flanking
strikers, strikers, chargers). A **support** unit is every other kind (shooters, casters, healers,
bombards, stabbers). A unit's kind is what it does from the row it prefers
(`Game::unit_engage_kind`), as the auto formation reads it now.

What makes rows and lanes matter (docs/ENGAGEMENTS.md, "Action kinds and reach"; "What the first
battery showed"), and so what the tactics trade:

- A strike hits the front unit of a lane; a shot, a spell and a bombard hit the backmost. A unit
  with one unit before it and one behind it in its lane is hit by neither.
- A melee unit acts twice from the front row and once from the others; a shooter twice from the
  middle and the rear; a caster and a bombard twice from the rear.
- A charge works only from the front row, and the auto formation puts the second charger behind
  the strikers, where it does not charge.
- The rear is the exposed row: mages stand there and 66 to 73% of them fall.

## 3. The tactics

Five. Each has an **Expert** line (when a strong player picks it, in the manner of docs/DECKS.md):
a tactic without one that differs from the others' does not belong in the game.

### Line (`TACTIC_LINE`), the default

Today's auto formation, unchanged: same selection, same placement, the same battles as before for
the same units. Every army, garrison and player starts with it.

*Expert:* a mixed army against an enemy it has not scouted.

### Charge (`TACTIC_CHARGE`)

- Selection: chargers first, then anti-flanking strikers, strikers, shooters, stabbers, casters,
  healers, bombards; no "one of each kind first".
- Placement: as Line, with the chargers placed before the other guards, so they take the front
  row from the centre out. The other guards fill what is left of the front, then the middle.

*Expert:* a mounted army in the open against an army without Formation units. Not into a city, a
forest or a jungle (a charge is a strike there), and not into a Square.

### Volley (`TACTIC_VOLLEY`)

- Selection: a screen first, one guard for each lane of the side (the strongest healthy ones; a
  large unit screens its two lanes); then the support units, in the order shooters, casters,
  bombards, stabbers, healers, the healthy and then the strongest first; then the remaining
  guards.
- Placement: as Line (the screen in front, each support unit in its own best row).

*Expert:* more good archers, mages or siege than Line would let fight, against melee that has to
come through a screen; a garrison behind walls. Dies to a Charge, which reaches the support.

### Square (`TACTIC_SQUARE`)

- Selection: as Line.
- Placement: lane by lane from the centre out, each lane filled to its three rows before the
  next: the front cell the next guard, the middle cell the next support unit, the rear cell the
  next guard. When the guards run out, the rear cell and then the front cell take support units;
  when the support runs out, the middle cell takes a guard. Large units go first and take a front
  or a rear pair of cells. Within guards and within support, the strongest first; anti-flanking
  strikers take the rear cells before other guards (a charger's reach takes them first wherever
  they stand).

So the support stands in the middle with a guard before and behind it: out of reach of strikes,
shots, spells and bombards. The price is the rear guards, who act once instead of twice, and a
narrower front.

*Expert:* an army whose mages, healers or siege must live, against archers, casters or cavalry.
Loses to a plain melee Line, which out-strikes its narrow front.

### Vanguard (`TACTIC_VANGUARD`)

- Selection: as Line.
- Placement: as Line, but a **veteran** guard stands in the middle row behind a guard that is not
  one, while there are such guards to stand in front (lanes from the centre out: the front cell a
  guard that is no veteran, the weakest first; the middle cell behind it a veteran, the most
  valuable first). Veterans with nobody to stand behind are placed as in Line. A veteran is a unit
  with the Hero promotion, or of level `TACTIC_VETERAN_LEVEL` (4) or above.

The veterans act once instead of twice and are hit by strikes only when the unit before them
falls.

*Expert:* an army that carries heroes or high-level units through a battle it will win anyway,
with expendable units to spend. Never the tactic that wins a close battle.

## 4. Whose tactic a battle uses

- **An army** has a tactic (`Army::tactic`). **A city** has one for its garrison
  (`City::garrison_tactic`). Both start as Line.
- A side's tactic in a battle is the tactic of the body most of its units on the field belong to:
  count the pool's (the defender's candidates') units by army, the garrison units of the plot's
  city counting for the city; the largest count wins, an army before the garrison on a tie, the
  lower army id on a tie between armies. Units in no army and no garrison (heroes, mages, recon
  units, summons) count for nobody and are arranged by the winner's tactic. A side with no unit of
  an army or a garrison fights in Line.
- **A change takes effect at the start of its owner's next turn** (`Army::tactic_next`,
  `City::garrison_tactic_next`; the change is free). A tactic is a standing choice, not something
  switched before each attack; the army that is attacked this turn fights as it was set.
- A new army formed from units takes the tactic of the army or garrison most of them came from;
  a merged army keeps the receiving army's; a city that changes hands goes back to Line.
- Barbarians fight in Line.

## 5. Data

`data/json/battle_tactics.jsonc` (`BattleTacticType`, `BattleTacticInfo` in ffh-data; five entries;
the Rust consts by name through `d.t`), so a tactic's knobs are tuned without code:

```jsonc
{
  "type": "TACTIC_VOLLEY",
  "description": "TXT_KEY_TACTIC_VOLLEY",
  "help": "TXT_KEY_TACTIC_VOLLEY_HELP",
  // Selection: the kinds in the order they take the slots (ANTI_FLANK is a striker with
  // Formation). Absent: Line's.
  "pick_order": ["ANTI_FLANK", "STRIKE", "CHARGE", "SHOOT", "SPELL", "BOMBARD", "STAB", "HEAL"],
  // One unit of each kind before the rest (Line's rule). Absent: true.
  "one_of_each": false,
  // Guards taken before anything else, in lanes of the side (Volley: 1 a lane). Absent: 0.
  "screen_per_lane": 1,
  // Placement: LINE, SQUARE or VANGUARD. Absent: LINE.
  "placement": "LINE",
  // LINE placement only: kinds placed before the others (Charge: its chargers). Absent: none.
  "place_first": []
}
```

`rules_defines.jsonc`: `TACTIC_VETERAN_LEVEL` (4). Text (`data/text/en.tsv`): each tactic's name and
a help of bare rules, one bullet a rule (CLAUDE.md, "Rules text and tooltips"), for example
`[ICON_BULLET]Chargers take the slots and the front row first.`

## 6. What changes where

**Rules (ffh-core).**
- `formation::auto_arrange_around` takes the tactic's knobs; with Line's it returns exactly what it
  returns today (a test pins that on many random pools: the same `Placement`s as the present
  function, kept in the test as the reference).
- `Game::engage_auto_formation(pool, side, slots, ctx)` and `engage_defense_formation` resolve the
  side's tactic (section 4) and use it. A new `Game::engage_side_tactic(units, plot) ->
  BattleTacticType` is the resolution; the preview, the battle log and the view read it.
- `Army::tactic`, `Army::tactic_next`, `City::garrison_tactic`, `City::garrison_tactic_next`; applied
  in the owner's turn processing; saved (`save/state.rs`, `FORMAT_VERSION` + 1, a line in its
  history). Commands `Game::army_set_tactic(p, army, tactic)` and `city_set_garrison_tactic(p, city,
  tactic)` (an `ArmyResult`-style refusal for another player's).
- The `formation` parameter of `group_engage` stays (the lab's `@row`, tests): the rules still
  accept an ordered formation.
- docs/PORTING.md, "Changes from FFH2": one entry.

**Human play (ffh-play).**
- Commands `army_tactic {army, tactic}` and `garrison_tactic {city, tactic}`; views: each army's
  `tactic` and `tactic_next` on the Armies screen's data, the city's on the city query's
  `governance`/garrison part; the static data lists the tactics (name, help).
- Client: the Armies screen shows each army's tactic as a selector (five options, the help as the
  tooltip, "from next turn" while a change is pending); the city screen the garrison's, next to
  where units trained there go. The battle plan's board stops taking drags and clicks that move
  units, and loses Auto-arrange: it shows the formation the tactic gives, and names both sides'
  tactics. The human `engage` command refuses a `formation` ("An army fights in its tactic.").
- docs/HUMAN.md: the commands, the screens, the board.

**AI (ffh-ai).** Each army's tactic is set by a rule when the army's make-up changes (and for each
city's garrison), from the lab's matrix (section 7); the starting rule, to be corrected by it:
Charge with chargers at least 40% of the army's fighting units; else Volley with support at least
50%; else Square with support at least 25%; else Line. Garrisons: Volley with support at least
50%, else Line. The AI does not use Vanguard. docs/AI.md: a short section.

**Lab (ffh-cli).** `ffh battles`: ground words `tactic:NAME` (the attackers') and `dtactic:NAME`
(the defenders'), default LINE; `crates/ffh-cli/scripts/battles_tactics.py` writes the round robin
of section 7 and prints its matrices.

## 7. What the lab has to show

The owner's balance target (docs/ENGAGEMENTS.md, "Balance lab"): every formation has a formation
that beats it. Measured with `ffh battles`, both sides the same army (so only the tactics differ),
every tactic against every tactic, attacking and defending, 400 battles a cell:

- armies: a melee core with 2 archers and 2 mages; a mounted army with 2 archers; an archer and
  mage army with 3 guards; a core with a hero and 2 level-5 units; each at an early, a middle and
  a late tech level, of 8 and of 12 units (12: selection matters);
- ground: open, forest, a walled city (defender only).

A tactic ships when (a) no other tactic is at least as good as it in every cell, (b) it beats
Line clearly (the net share of HP beyond 2 standard errors) with some army on some ground, and
(c) for Vanguard instead of (b): its veterans' deaths are at most half of Line's in the battles
its side wins. A tactic that fails is reported with its matrix, not tuned by changing unit stats;
its own knobs (section 5) may be tuned. The matrices go into this file (section 8).

## 8. Measures

(Filled by the implementation.)

## 9. Open

- **A grid of the player's own** for an army (the "grids" after the tactics): a saved custom
  formation an army uses instead of a tactic. Not in this change.
- **Civilization tactics**: one tactic of its own for a civilization (a Doviello tactic without a
  rear, a Luchuirp tactic around its golems). Not in this change; the data table makes room.
- **Seeing an enemy's tactic** before attacking: the preview shows it as it shows the defender's
  formation today. Whether it should be hidden without a scout is undecided.
- **Requisition by tactic**: an army's requisition filling the mix its tactic wants (a Volley army
  asks for archers) is undecided.
