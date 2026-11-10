# Battle tactics

**Status: rules, data and lab built (2026-11-10); the human client's selectors and the AI's rule
(section 6) next.** This file was the spec for the implementation. Sections 1 to 6 are decided;
section 7 is what the lab has to show before a tactic ships, section 8 what it showed, section 9
lists what is left open, and section 10 the tactic that was tried and not shipped.

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

Four. Each has an **Expert** line (when a strong player picks it, in the manner of docs/DECKS.md):
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
forest or a jungle (a charge is a strike there).

### Volley (`TACTIC_VOLLEY`)

- Selection: a screen first, one guard for each lane of the side (the strongest healthy ones; a
  large unit screens its two lanes); then the support units, in the order shooters, casters,
  bombards, stabbers, healers, the healthy and then the strongest first; then the remaining
  guards.
- Placement: as Line (the screen in front, each support unit in its own best row).

*Expert:* more good archers, mages or siege than Line would let fight, against melee that has to
come through a screen; a garrison behind walls. Dies to a Charge, which reaches the support.

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

`data/json/battle_tactics.jsonc` (`BattleTacticType`, `BattleTacticInfo` in ffh-data; four entries;
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
  // Placement: LINE or VANGUARD. Absent: LINE.
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
- Client: the Armies screen shows each army's tactic as a selector (four options, the help as the
  tooltip, "from next turn" while a change is pending); the city screen the garrison's, next to
  where units trained there go. The battle plan's board stops taking drags and clicks that move
  units, and loses Auto-arrange: it shows the formation the tactic gives, and names both sides'
  tactics. The human `engage` command refuses a `formation` ("An army fights in its tactic.").
- docs/HUMAN.md: the commands, the screens, the board.

**AI (ffh-ai).** Each army's tactic is set by a rule when the army's make-up changes (and for each
city's garrison), from the lab's matrix (section 7); the starting rule, to be corrected by it:
Charge with chargers at least 40% of the army's fighting units; else Volley with support at least
50%; else Line. Garrisons: Volley with support at least
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

(Measured history: these matrices were measured with five tactics, Square among them (section
10), before it was dropped; they are kept as measured. Measured 2026-11-10, `battles_tactics.py`: 1,800 matchups of 400 battles, 4 armies x 3 levels
x 8 and 12 units x 3 grounds = 72 conditions, every tactic against every tactic attacking and
defending; the knobs as in `battle_tactics.jsonc`, none tuned. Armies as section 7: early Axemen,
Horsemen, Archers, Fire I Adepts; mid Champions, Horse Archers, Longbowmen, Fire II Mages; late
Phalanxes, Knights, Marksmen, Fire II Archmages; the hero army's hero is its core unit with the
Hero promotion, its veterans level 5 with Combat I-IV; walls are fortified 5 turns.)

A score is the head-to-head mean of attacking and defending, `(net(X>Y) - net(Y>X)) / 2`, of the
net share of HP (percent). Where two tactics give the same formation (Vanguard without veterans,
Volley or Charge where the selection is Line's) the score is exactly 0.

**Mean score per ground, row against column** (over the 24 conditions of each ground):

| open | Line | Charge | Volley | Square | Vanguard |
|---|---|---|---|---|---|
| Line | . | +1.4 | -5.5 | +13.5 | +4.6 |
| Charge | -1.4 | . | -5.4 | +12.3 | +2.9 |
| Volley | +5.5 | +5.4 | . | +16.8 | +9.4 |
| Square | -13.5 | -12.3 | -16.8 | . | -9.5 |
| Vanguard | -4.6 | -2.9 | -9.4 | +9.5 | . |

| forest | Line | Charge | Volley | Square | Vanguard |
|---|---|---|---|---|---|
| Line | . | +5.5 | +4.8 | +16.3 | +3.7 |
| Charge | -5.5 | . | +2.9 | +9.4 | -2.0 |
| Volley | -4.8 | -2.9 | . | +4.0 | -2.9 |
| Square | -16.3 | -9.4 | -4.0 | . | -10.6 |
| Vanguard | -3.7 | +2.0 | +2.9 | +10.6 | . |

| walls | Line | Charge | Volley | Square | Vanguard |
|---|---|---|---|---|---|
| Line | . | +0.7 | -1.0 | +8.2 | +1.2 |
| Charge | -0.7 | . | -1.3 | +8.3 | +0.8 |
| Volley | +1.0 | +1.3 | . | +9.4 | +2.1 |
| Square | -8.2 | -8.3 | -9.4 | . | -6.6 |
| Vanguard | -1.2 | -0.8 | -2.1 | +6.6 | . |

**Against Line per army and ground**: the mean score over levels and sizes (conditions won / lost
beyond 2 standard errors, of 6):

| | melee | | | mounted | | | ranged | | | hero | | |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| | open | forest | walls | open | forest | walls | open | forest | walls | open | forest | walls |
| Charge | -4.5 (0/3) | -14.0 (1/5) | -3.0 (1/3) | -0.4 (2/2) | +15.3 (4/0) | -1.2 (3/3) | +2.2 (2/1) | -28.1 (0/6) | -0.7 (2/2) | -2.9 (0/5) | +4.6 (6/0) | +2.2 (4/1) |
| Volley | +13.2 (3/0) | +8.1 (6/0) | +4.7 (3/1) | 0 | 0 | 0 | +2.2 (2/1) | -28.1 (0/6) | -0.4 (2/3) | +6.5 (5/0) | +0.8 (2/2) | -0.2 (2/3) |
| Square | -13.3 (1/5) | -21.4 (0/6) | -15.6 (0/6) | -6.6 (1/4) | -11.9 (0/6) | -5.6 (0/6) | -30.5 (0/6) | -17.1 (0/6) | -8.6 (0/6) | -3.7 (1/5) | -14.8 (0/6) | -3.0 (1/5) |
| Vanguard | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | -18.5 (0/6) | -15.0 (0/6) | -5.0 (0/6) |

The largest wins over Line: Charge, the early mounted army in a forest, +26.9 (SE 0.7: 6 slots,
Charge fills them with Horsemen where Line takes the archers); Volley, the late and mid 12-unit
melee armies in the open, +32.7 (SE 0.5) and +30.4 (SE 0.4: Line leaves a mage out, Volley takes
both and both archers behind a screen).

**Dominance (a)**: of the 504 cells a tactic can be compared in (each condition, against each of
the other three tactics attacking and defending, and the head-to-head), the cells where the column
tactic is at least as good as the row tactic:

| | Line | Charge | Volley | Square | Vanguard |
|---|---|---|---|---|---|
| Line | . | 271 | 327 | 60 | 379 |
| Charge | 293 | . | 320 | 139 | 259 |
| Volley | 363 | 316 | . | 95 | 290 |
| Square | 444 | 365 | 409 | . | 379 |
| Vanguard | 503 | 305 | 400 | 125 | . |

**Vanguard (c)**: in the battles the hero army's side won, its veterans (the hero and the two
level-5 units) fell 9.8% of the time in Vanguard and 20.2% in Line (8,260 of 84,000 and 16,955 of
84,000): 0.49 of Line's. Per condition it is far lower in a forest (2.8% against 40.5% early, 0.7%
against 32.0% mid at 12 units) and on open land at 12 units, and higher on open land at 8 units
(mid 27.4% against 22.8%, late 33.3% against 25.5%: with 4 lanes and a short front the veterans'
cover falls first).

**Verdict** (section 7):

- **Charge ships**: dominated by none; beats Line beyond 2 SE in 25 of 72 conditions (mounted in a
  forest, the hero army in a forest), loses in 31 (the ranged army in a forest: -28.1, its archers
  left out).
- **Volley ships**: dominated by none; beats Line in 25 of 72 (melee with archers and mages, the
  open and the forest), loses in 16 (the ranged army in a forest, where its 3 guards cannot screen
  and the selection is the ranged army's own).
- **Square does not ship** (the designer's decision, 2026-11-10; section 10): it loses to every
  other tactic on every ground (the mean scores above), beats Line beyond 2 SE in only 4 of 72
  conditions and loses in 67. Its Expert line (support that must live) does not show in net HP.
- **Vanguard ships by the letter, barely**: (c) holds at 0.49 against the 0.5 required; (a) holds
  by one cell (Line is at least as good as it in 503 of 504 cells: where the army has no veterans
  the two are the same formation, and with veterans it costs HP: -18.5 in the open). As its Expert
  line says, it is never the tactic that wins a close battle.

## 9. Open

- **A grid of the player's own** for an army (the "grids" after the tactics): a saved custom
  formation an army uses instead of a tactic. Not in this change.
- **Civilization tactics**: one tactic of its own for a civilization (a Doviello tactic without a
  rear, a Luchuirp tactic around its golems). Not in this change; the data table makes room.
- **Seeing an enemy's tactic** before attacking: the preview shows it as it shows the defender's
  formation today. Whether it should be hidden without a scout is undecided.
- **Requisition by tactic**: an army's requisition filling the mix its tactic wants (a Volley army
  asks for archers) is undecided.

## 10. Tried and not shipped

- **Square** (`TACTIC_SQUARE`, removed 2026-11-10). Selection as Line; placement lane by lane from
  the centre out, each lane filled to its three rows: a guard in front, a support unit in the
  middle, a guard in the rear (anti-flanking strikers first), so the support stood out of reach of
  strikes, shots, spells and bombards. Measured (section 8): the weakest tactic on every ground,
  losing to each of the others (-13.5 against Line in the open, -16.3 in a forest, -8.2 at walls);
  the rear guards act once instead of twice and the front is narrower. The designer dropped it.
