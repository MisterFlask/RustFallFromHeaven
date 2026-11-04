# Sieges and Sentinels (spec, not built)

The owner's design (2026-11-04) for taking cities under engagement combat (docs/ENGAGEMENTS.md),
with the defaults Claude proposed for the points the owner left open, accepted the same day. Marked
**(default)**: a value or rule chosen for a gap, open to change when it plays badly. Nothing here is
built yet; "today" below is master as of 2026-11-04.

Why: under formation battles a well-held city hardly falls. The plot falls only when no defender is
left, a battle of three rounds kills a few units a side, and between battles the defenders heal 20% a
turn in the city while the besiegers heal 5%. City defence only costs the attacker slots, in steps
of 25 points that a bombard rarely crosses. This design makes walls something on the field that a
siege wears down, and makes a siege stop the city's healing.

## Sentinels

**What they are.** A city with a wall building has **Sentinels**: wall sections that fight in the
defender's front row. Not units: no unit id, no upkeep, no map presence, never moved, never in an
army; the city holds their state and the engagement adds them as fighters at set-up (the resolver's
`Fighter::tag` is the caller's handle, so it needs no unit). The name avoids "garrison" (the armies'
city garrisons, Civ4's City Garrison promotions).

- **Which cities.** A building's new `iSentinels` (`CIV4BuildingInfos.xml`) gives the count; a city
  has the most any of its buildings gives, not the sum. Palisade, Walls and Wall of Stone: 2
  **(default)**. An occupied city (`city_is_occupation`) has none fighting.
- **Their strength grows with the city's defence** (the owner's rule): its total defence as built,
  `city_total_defense` (buildings, culture level, the player's modifier, FfH's range defence).
  **(default)** max HP `SENTINEL_HP` (100) x (100 + defence) / 100; Dv `SENTINEL_DEFENSE` (3) x
  (100 + defence) / 100; A `SENTINEL_ATTACK` (1). A Developing city with Walls (45): 145 HP, Dv 4.35.
- **How they fight** **(default)**: front row only, one STRIKE a round, speed 0 (they act last);
  no dodge, no crit bonus, no first strikes, not large. They are always in the front row: no
  knock-back, no withdrawal; immune to Fear, capture, enslavement, poison, conversion and every
  promotion or spell effect on units. The Divine role's heals skip them.
- **No XP.** Damaging or killing a Sentinel gives no XP and triggers no per-kill effect (heal on
  kill, gold, created units, capture rolls). A kill a Sentinel makes is shared by its side's units
  (XP below); the Sentinel takes no share.
- **They count for capture** (the owner's decision): a Sentinel with HP left is a defender, so the
  plot falls only when every soldier and every Sentinel is gone. A walled city with no soldiers
  fights with its Sentinels alone: moving into it is an attack, an engagement against them.
- **Damage persists.** The city keeps each Sentinel's damage as a percent of its max HP
  (`City::sentinel_damage: Vec<i32>`, one per Sentinel, like a unit's `damage`), so a change of
  defence scales their HP without healing them. A Sentinel at 0 HP is breached: it does not fight
  until it is repaired above 0.
- **Repair** (the owner's rule: only when not under siege): at the city's turn, each Sentinel
  repairs `SENTINEL_REPAIR_PERCENT` (10) of its max HP **(default)** unless the city is besieged
  (below). A breached one repairs from 0 the same way.
- **New and captured walls** **(default)**: Sentinels a city gains (a wall built, bought or
  received) start at full HP, or at 0 if the city is besieged then (buying walls mid-siege buys
  nothing at once). A captured city's Sentinels are breached (0 HP) and repair for the new owner.

## Slots and the grid

**The walls no longer cost the attacker slots.** Instead, in a city with Sentinels **both** sides
lose one slot per Sentinel (the owner's rule: "universally reduce the number of slots by 2"), and
the Sentinels fill the defender's: the defender fields its soldiers in `S - n` slots plus `n`
Sentinels, the attacker `S - n` slots (`S` the plot's slots as today, `n` the Sentinels standing;
`ENGAGEMENT_SLOTS_MIN` applies to the soldiers' slots). The owner's example: 3 soldiers and 2
Sentinels against 3 attackers.

- `ENGAGEMENT_SLOTS_DEFENSE_DIVISOR` and the city-defence and bombarded slot changes (`slots.rs`)
  go. A city without Sentinels has no slot change: its culture defence does nothing in an
  engagement (in a walled city it strengthens the Sentinels).
- Lanes: each side's from its cells as today (`formation::lanes_for`), the defender's counting the
  Sentinels.
- The auto formation places the Sentinels first, in the front row, the lanes nearest the centre
  first; the soldiers' places follow as today (a Frontline unit whose front cell a Sentinel holds
  goes to the nearest row with room). With 2 lanes the two Sentinels are the whole front row: the
  attacker's melee must breach them before it reaches a soldier; shots, spells and bombards reach
  the backs of the lanes and stabs anyone, over the walls.
- An ordered formation (the attacker's) is unchanged; the defender's is always the auto formation.

## Siege

**Besieged** (the owner's rule: "if there are hostile units nearby"; the details are defaults): a
city is besieged while some plot adjacent to it holds a land unit that is at war with the city's
owner (`unit_is_enemy`, so barbarians, animals and hidden-nationality units count), can fight
(strength above 0) and is visible to the city's team. A pure function of the map
(`Game::city_is_besieged`), evaluated where it is needed: no state, nothing saved. A lone enemy
counts; the defender can sally out and kill it.

**What a siege does:**
- **Soldiers do not heal from the city** (the owner's rule): on a besieged city's plot a unit's heal
  loses the city's part (`CITY_HEAL_RATE` and the city's buildings' `heal_rate`) **(default)**. Its
  own extra heal (promotions, a race's regeneration), the medics on and beside the plot and healing
  spells still work: they are the counterplay.
- **Sentinels do not repair.**
- **It shows** (the owner's request): a "Besieged" mark on the city's banner, its tooltip naming
  the besieging units; a line on the city screen with the Sentinels' HP; on a unit in a besieged
  city, the unit panel says it does not heal from the city and why. The banner and the city screen
  show the Sentinels' HP whether or not besieged.

## Bombarding

**A bombard hits every Sentinel and every soldier** (the owner's rule). The unit types with a
bombard rate are unchanged (Catapult 15, Battering Ram 25, Trebuchet 25, Cannon 20, Dwarven Cannon
30, Hill Giant 15, Trireme 10, Frigate 20, Man-o'-War 30; Accuracy +20). It stays one of the unit's
attacks this turn and costs a move.

- **Sentinels** **(default)**: each takes the bombard rate, less the walls' bombard defence as today
  (`building_bombard_defense`, 25 for Walls and Wall of Stone; ignored by units with
  `bIgnoreBuildingDefense`), as a percent of its max HP. A Catapult: 11% each; a Cannon: 20%. A
  bombard can breach a Sentinel (0 HP).
- **Soldiers** **(default)**: every land unit on the city's plot at war with the bomber takes half
  that, less its collateral damage protection (Drill); a bombard never takes a soldier below
  `SIEGE_BOMBARD_FLOOR_PERCENT` (50) of its max HP, so bombarding alone kills no one.
- A city can be bombarded while it has a Sentinel with HP left or a soldier above the floor. Civ4's
  city defence damage (`City::defense_damage`, `bombarded`) is no longer what a bombard changes; it
  goes, or stays at 0 where the duel still reads it (ships).
- **In a battle** **(default, added with the spec)**: the Siege role's BOMBARD takes a standing
  Sentinel in its reach first (siege batters the walls), else the back of a lane as today.

## XP

**Each kill's XP is shared equally** (the owner's rule: "each unit in a combat takes an equal share
of any given kill"). It replaces today's split by the damage dealt (`apply.rs`, step 2), in every
engagement, not only sieges.

- **The kill's XP**: today's value for the killer against that victim,
  `clamp(xp_value(v) x Dv(v <- k) / A(k -> v), MIN, MAX)` **(default)**.
- **Who shares**: the killing side's committed units that did not sit out and are alive after the
  battle, withdrawn ones included; never a Sentinel. Each takes the same share.
- **Fractions** **(default)**: shares are kept in hundredths. Each unit has a carry
  (`Unit::xp_carry`, 0..99); a share adds to it and whole points go to the unit's experience
  (`unit_change_experience`, its caps and modifiers as today). The remainder of the division goes
  to the killer (if it shares), else to the first sharer in fighter order, so a kill's XP is never
  lost or multiplied.
- Withdrawal XP is unchanged. A Sentinel's kill is shared by its side's soldiers; killing a
  Sentinel gives nothing.

## AI

A rules change is not gated on the AI (CLAUDE.md, "How AI changes are checked"); the AI gets
something reasonable:
- its city targeting, defence counts and "undefended city" checks count standing Sentinels as
  defenders, so a stack does not walk at an empty walled city expecting a free capture;
- its siege units bombard a city with Sentinels standing before its stack attacks (its pre-attack
  bombard-first already runs in `group_attack_force`'s preamble; the condition becomes the new
  bombardable rule);
- it still judges attacks on the duel's odds (a known gap, docs/ENGAGEMENTS.md "Open").

## Building it

- Data: `iSentinels` on buildings; defines `SENTINEL_HP`, `SENTINEL_DEFENSE`, `SENTINEL_ATTACK`,
  `SENTINEL_REPAIR_PERCENT`, `SIEGE_BOMBARD_FLOOR_PERCENT`; `ENGAGEMENT_SLOTS_DEFENSE_DIVISOR` goes.
- State: `City::sentinel_damage`, `Unit::xp_carry` (save lists, `save::FORMAT_VERSION` bump).
- Rules: `engagement/` (slots, the defender's set-up and auto formation, the Sentinel fighters,
  writing their damage back, the capture test, BOMBARD's target, XP), `unit_bombard` and
  `city_is_bombardable`, `unit_heal_rate`, the city's turn (repair), capture (breach), the move
  into an empty walled city (an attack).
- Preview, battle log and battle screens: the Sentinels as fighters (a wall sprite on the HD-2D
  battle field); the city banner, city screen, unit panel and plot tooltip (siege, Sentinel HP).
- Telemetry **(default)**: turns besieged per city, Sentinels breached, bombard damage dealt.
- Docs: docs/ENGAGEMENTS.md (slots, the defender, XP, bombard), docs/PORTING.md "Changes from FFH2".
