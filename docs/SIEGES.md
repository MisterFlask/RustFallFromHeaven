# Sieges and Sentinels

The owner's design (2026-11-04) for taking cities under engagement combat (docs/ENGAGEMENTS.md),
with the defaults Claude proposed for the points the owner left open, accepted the same day, and the
owner's addition while it was built: Sentinels intercept the arrows and spells aimed past them.
Marked **(default)**: a value or rule chosen for a gap, open to change when it plays badly.

Why: under formation battles a well-held city hardly fell. The plot falls only when no defender is
left, a battle of three rounds kills a few units a side, and between battles the defenders healed
20% a turn in the city while the besiegers healed 5%. City defence only cost the attacker slots, in
steps of 25 points that a bombard rarely crossed. Now the walls are something on the field that a
siege wears down, and a siege stops the city's healing.

Code: `crates/ffh-core/src/engagement/siege.rs` (the Sentinels' count, values, damage, repair, the
siege test, the bombard's damage); the engagement's set-up, post-battle steps, slots, auto
formation and resolver (`engage.rs`, `apply.rs`, `slots.rs`, `arrange.rs`, `formation/mod.rs`'s
`Fighter::fixed`); `unit_heal_rate`, `unit_bombard`, `city_is_bombardable`,
`group_get_bombard_turns`, `unit_can_move_into`, `group_attack_engage`, the city's turn and
`player_acquire_city`. Data: `sentinels` on buildings, `UNIT_SENTINEL`, `SENTINEL_REPAIR_PERCENT`
and `SIEGE_BOMBARD_FLOOR_PERCENT` (`rules_defines.jsonc`). Tests: `crates/ffh-core/tests/sieges.rs`.

## Sentinels

**What they are.** A city with a wall building has **Sentinels**: wall sections that fight in the
defender's front row. Not units: no unit id, no upkeep, no map presence, never moved, never in an
army; the city holds their damage and the engagement adds them as fighters at set-up, after the
units (`Setup::sentinel_range`). The name avoids "garrison" (the armies' city garrisons, Civ4's City
Garrison promotions).

- **Which cities.** A building's `sentinels` (`buildings.jsonc`) gives the count; a city has
  the most any of its buildings gives, not the sum. Palisade, Walls and Wall of Stone: 2
  **(default)**. An occupied city (`city_is_occupation`) has none fighting.
- **Their values** are `UNIT_SENTINEL`'s (`units.jsonc`; never trained or placed, so its
  name, role and rows reach the battle screens like any unit type's): its stat sheet (HP 100,
  attack 10, defence 30, magic 0, magic defence 30, accuracy 90, evasion 0, crit 10, speed 0: they
  act last; docs/UNIT_STATS.md), one STRIKE from the front row and nothing from the others
  **(default)**.
- **They grow with the city's defence** (the owner's rule): max HP, defence and magic defence times
  (100 + its total defence) / 100, `city_total_defense` (buildings, culture level, the player's
  modifier, FfH's range defence). A Developing city with Walls (45): 145 HP, defence 43.5.
- **Fixed** (`Fighter::fixed`): always in their front-row cells, the lanes nearest the centre; never
  knocked back, never withdraw, never healed in the battle; Fear, defensive strikes and every
  per-kill effect pass them by. Collateral splash and spells hurt them as anyone.
- **They intercept** (the owner's addition): a SHOOT or SPELL aimed down a lane with a standing
  Sentinel in it hits the Sentinel instead of the back of the lane; lanes without one stay open. A
  BOMBARD (siege) takes the Sentinels on the field first, wherever they stand **(default)**.
- **No XP.** Damaging or breaching a Sentinel gives no XP and has no per-kill effect; a kill a
  Sentinel makes is shared by its side's units (XP below), and has no per-kill effect either.
- **They count for capture** (the owner's decision): a Sentinel with HP left is a defender, so the
  plot falls only when every soldier and every Sentinel is gone. A walled city with no soldiers
  fights with its Sentinels alone: moving into it is an attack (`unit_can_move_into` treats its
  standing Sentinels as visible enemies), an engagement against them.
- **Damage persists.** The city keeps each Sentinel's damage as a percent of its max HP
  (`City::sentinel_damage`, like a unit's `damage`), so a change of defence scales its HP without
  healing it. A Sentinel at 100 is breached: it does not fight until it repairs.
- **Repair** (the owner's rule: only when not under siege): at the city's turn each Sentinel repairs
  `SENTINEL_REPAIR_PERCENT` (10) of its max HP **(default)** unless the city is besieged. A breached
  one repairs the same way.
- **New and conquered walls** **(default)**: Sentinels a city gains (a wall built, bought or
  received) start whole, or breached if the city is besieged then (buying walls mid-siege buys
  nothing at once). A conquered city's Sentinels are breached and repair for the new owner.

## Slots and the grid

**The city's defence no longer costs the attacker slots.** Instead, in a city with standing
Sentinels both sides lose one slot per Sentinel (the owner's rule), and the Sentinels fill the
defender's: it fields its soldiers in `S - n` slots plus its `n` Sentinels, the attacker `S - n`
(`S` the plot's slots, `ENGAGEMENT_SLOTS_MIN` applied to the soldiers' slots;
`SlotSource::Sentinels`). Open flat land with Walls: 8 attackers against 8 soldiers and 2 Sentinels.

- `ENGAGEMENT_SLOTS_DEFENSE_DIVISOR` and the city-defence and bombarded slot sources are gone. A city
  without walls has no slot change: its culture defence does nothing in an engagement (in a walled
  city it grows the Sentinels).
- Lanes: the defender's count its Sentinels' cells (`engage_lanes`).
- The defender's auto formation leaves the Sentinels' cells (`engage_sentinel_cells`: the front row,
  the lanes nearest the centre, then the lower) and places its units as before in the cells left.
- An ordered formation (the attacker's) is unchanged.

## Siege

**Besieged** (the owner's rule: "if there are hostile units nearby"): a city is besieged while a land
plot next to it holds a land unit at war with the city's owner (`unit_is_enemy`, so barbarians,
animals and hidden-nationality units count), with strength above 0, that the city's team sees
**(default)**. A pure test of the map (`Game::city_is_besieged`, `city_besiegers`): no state, nothing
saved. A lone enemy counts; the defender can sally out and kill it. A great-person card adds one
case (docs/CARDS.md, the Siege Colossus): a unit with `bombard_besieges` that bombards the city
(from up to its `bombard_range`, 2 plots) makes it count as besieged until the city's next turn
(`City::besieged_by_bombard`, cleared at the city's turn after the Sentinels' repair). Other cards
change a city's Sentinels: the Living Walls give them the Treant's attack and HP
(`sentinel_unit`), the Arcane Fortress halves the damage they take from bombards, BOMBARD and
SHOOT (`sentinel_ranged_resist`).

**What a siege does:**
- **Units do not heal from the city** (the owner's rule): on a besieged city's plot a unit's heal
  loses `CITY_HEAL_RATE` and the city's buildings' heal **(default)**. Its own extra heal
  (promotions, a race's regeneration) and the medics on and beside the plot still work.
- **Sentinels do not repair.**
- **It shows** (the owner's request): the city's banner and the city screen say it is besieged and
  show its Sentinels; a unit in a besieged city says it does not heal from the city.

## Bombarding

**A bombard hits every Sentinel and every soldier** (the owner's rule). The unit types with a
bombard rate are unchanged (Catapult 15, Battering Ram 25, Trebuchet 25, Cannon 20, Dwarven Cannon
30, Hill Giant 15, Trireme 10, Frigate 20, Man-o'-War 30; Accuracy +20). It stays one of the unit's
attacks this turn and costs a move.

- **Sentinels** **(default)**: each standing one takes the bombard rate, less the walls' bombard
  defence (25 for Walls and Wall of Stone; none against `bIgnoreBuildingDefense` units), in percent
  of its max HP. A Catapult: 11% each; a Cannon: 20%. A bombard can breach a Sentinel.
- **Soldiers** **(default)**: every land unit on the city's plot at war with the bomber takes half
  that, less its collateral protection (Drill); a bombard never takes a soldier below
  `SIEGE_BOMBARD_FLOOR_PERCENT` (50) of its HP, so bombarding alone kills no one.
- A city can be bombarded while it has a Sentinel standing or a soldier above the floor. Civ4's city
  defence damage (`City::defense_damage`) is no longer what a bombard changes.

## XP

**Each kill's XP is shared equally** (the owner's rule: "each unit in a combat takes an equal share
of any given kill"), in every engagement, not only sieges; it replaces the split by damage dealt.

- **The kill's XP**: the value for the killer against that victim,
  `clamp(xp_value(v) x Dv(v <- k) / A(k -> v), MIN, MAX)` **(default)**.
- **Who shares**: the killing side's committed units that did not sit out and are alive after the
  battle, withdrawn ones included; never a Sentinel.
- **Fractions** **(default)**: shares are in hundredths; each unit keeps what is below a point
  (`Unit::xp_carry`) and whole points go to its experience (`unit_change_experience`, its caps and
  modifiers as before). The remainder of the division goes to the killer (if it shares), else the
  first sharer, so a kill's XP is never lost or multiplied.
- Withdrawal XP is unchanged.

## AI

A rules change is not gated on the AI (CLAUDE.md, "How AI changes are checked"); what it does:
- a walled enemy city with no units is an attack, not a free capture: its moves into one fight the
  Sentinels (the move rules), like a human's;
- its siege units bombard a city with Sentinels standing before its stack attacks: the pre-attack
  bombard-first in `group_attack_force` counts the turns to breach the least damaged Sentinel
  (`group_get_bombard_turns`);
- it still judges attacks on the duel's odds (a known gap, docs/ENGAGEMENTS.md "Open"), which do not
  see the Sentinels.

## Open

- The AI's target choice and odds do not see Sentinels (above).
- Telemetry of sieges (turns besieged, Sentinels breached, bombard damage) is not counted yet.
- The HD-2D battle field draws a Sentinel with the watchtower's art, scaled down: a stand-in.
