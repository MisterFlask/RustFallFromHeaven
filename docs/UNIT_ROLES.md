# Unit roles: levers for diverse compositions

**Status: proposal** (2026-11-06), for the owner to mark up. Nothing here is built. It follows the
owner's balance target (docs/ENGAGEMENTS.md, "Balance lab"): every unit of every tier has a
valuable place in some lineup, and every formation has a formation that beats it. Units past a
national class's limit are a deliberate sink and are not balanced for. **The auto formation is out
of scope** (the owner, 2026-11-06): everything below assumes each side's units stand where they
serve best; where the lab's results came from the auto formation's placement, that is said and set
aside. Values marked **(default)** are Claude's proposals; none is decided.

## 0. What the lab shows

From the squad round robins (docs/ENGAGEMENTS.md, "Balance lab"; squads at an equal budget per
tech level, open land, forest, hills and a walled city, both roles):

- **P1. Cavalry has no counter but the ground.** On open land and hills a mounted front wins at
  every level (6 Horsemen and 2 Skirmishers; 6 Horse Archers and 4 Archers; 4 Knights and 5
  Arquebuses beat every other squad). The charge does +100% in round 1, reaches any enemy and
  takes the highest threat for its HP (the archers and casters); the riders also act first (speed
  6, first strikes), dodge (evasion 15) and escape killing blows (withdrawal 25-35%). Formation
  (+40% against mounted, anti-flank) is not enough: 8 Horsemen attacking 8 Formation Axemen kill
  3.6 and lose 2.3.
- **P2. Healers have no place.** A heal is 20 HP whatever the healer's tier, from a 70 HP unit
  with defence 15-40 that stands at the back, where every shot lands.
- **P3. Casters are all or nothing.** Without a battle spell (the 60% bolt, one target) an Adept,
  Mage or Archmage is the worst pick of its level; with Fire II or Air II (100%, splash) a Mage is
  among the best.
- **P4. Large units are weak per slot**: one unit's actions for two cells, struck from two lanes.
  (Their placement in the auto formation is out of scope here.)
- **P5. Same-cost duplicates.** The Horseman beats the Chariot (at half its cost), the Marksman the
  Crossbowman, the Phalanx the Immortal, the Arquebus the Cannon in battle; the Longbowman, Ranger
  and Hunter are marginal.
- **P6. Numbers beat quality.** A hit is at most 40 HP (`40 x A / (A + D)`) and every frontline unit
  has 120 HP, so a unit four times as strong trades four times as well at ten times the cost (4
  Warriors kill a Phalanx in 99% of battles).
- **P7. The ground decides more than the squad.** Hills outweigh walls; a forest or a city turns off
  the charge, so the winner changes with the ground, not because of a counter squad.
- Siege engines have a place, the siege (their bombards before the assault): squads with them take
  walled cities that squads without them never take. In the battle itself they add little.

## 1. Where roles come from: reach

Who can hit whom decides what a unit is for (docs/ENGAGEMENTS.md, "Action kinds and reach"):

| Action | Reaches | So the unit |
|---|---|---|
| STRIKE | the front unit of each enemy lane | trades with the enemy's front |
| CHARGE (open land) | any enemy, the highest threat for its HP first; +100% in round 1 | dives the back line |
| STAB | any enemy | picks off whom it likes |
| SHOOT, SPELL, BOMBARD | the backmost unit of each enemy lane | trades with the enemy's back line |
| HEAL | the most hurt ally | sustains |

Consequences a design has to use: the **front** is hit by strikes; the **back** (the backmost unit
of a lane, which is the middle row when the rear is empty) by shots, spells and bombards; a unit
in the middle with someone behind it is hit only by charges, stabs and splash. Two interceptions
exist: anti-flank units take the charges aimed at their side's units in reach, and a city's
Sentinels take the shots and spells aimed down their lanes. Everything that dives (charge, stab)
is checked only by interception and by what it does to the diver.

**Magnitudes.** Type modifiers today are percentages of A and D, and the damage formula flattens
them. At equal A and D (20 HP a hit):

| Modifier | On the hitter's A | On the target's D |
|---|---|---|
| +25% | 22.2 (+11%) | 17.8 (-11%) |
| +50% | 24.0 (+20%) | 16.0 (-20%) |
| +100% | 26.7 (+33%) | 13.3 (-33%) |
| +200% | 30.0 (+50%) | 10.0 (-50%) |

A modifier that counts on both sides (today's `unit_combat_mods`: the attack half when the unit
hits that type, the defence half when that type hits it) of +100% makes a unit deal 26.7 and take
13.3 against that type: 2 to 1. Formation's +40% is 1.4 to 1, which a first-round charge outweighs.

## 2. The levers

### 2.1 The owner's six

| # | Lever | Today | Note |
|---|---|---|---|
| L1 | Damage to a unit type | `unit_combat_mods` (attack half), `unit_class_attack_mods`, on units and promotions | acts on A (the table above). A **damage multiplier** key (`damage_vs`, engine) would make "+50% damage" mean +50% |
| L2 | Resistance to damage from a unit type | `unit_combat_mods` (defence half) | the same; a `resist_vs` multiplier (engine) |
| L3 | Attacks from a rank | `engage_rows` counts | tier 4 and heroes add 1 in the best row |
| L4 | Targets an attack hits | one target; splash (collateral units, battle spells) hits the neighbours at 50% | **engine**: an attack pattern per row (section 2.3) |
| L5 | Damage type | `damage_type_combats` (typed strength added to A, less the target's resistance); spells have types | 8 types: physical, fire, cold, lightning, poison, death, holy, unholy |
| L6 | Resistance to a damage type | `damage_type_resists` | applies to typed strength and to spells |

### 2.2 Other levers

| # | Lever | Today | What it gives |
|---|---|---|---|
| L7 | Reach of an action | fixed per kind | **engine**: a shot at the front (javelins), a strike that reaches the second row (polearms), a stab only at the back row |
| L8 | Target priority | fixed per kind | **engine**: a unit that goes for mounted units, casters and healers, the wounded, the largest |
| L9 | Interception | anti-flank (charges), Sentinels (shots, spells) | a shield unit that takes the shots aimed down its lane; a bodyguard that takes the stabs at its neighbours (**engine**, after the Sentinels' rule) |
| L10 | Accuracy, evasion, crit | the stat sheet | an evasion-against-accuracy axis (evasive raiders, accurate missiles); crit immunity for armour (**engine**) |
| L11 | Flat armour | none | **engine**: each physical hit less N HP (at least 1); armour piercing ignores it. Answers swarms and multi-target chip damage (P6) without touching HP |
| L12 | Timing | speed, first strikes, the charge's round-1 bonus | **engine** for the rest: a round-1 volley, rage that grows each round, a last stand |
| L13 | Withdrawal and pursuit | withdrawal | pursuit (**engine**): a hit from this unit cannot be withdrawn from |
| L14 | HP | the stat sheet (by role, 120 for every frontline tier) | HP by tier: the direct answer to P6 |
| L15 | Heal power and pattern | heal = `ENGAGEMENT_HEAL_PERCENT` (global) | per-unit heal power, a heal on a whole row, a shield (temporary HP) (**engine**) |
| L16 | Charms and hexes | none in battle | docs/EARLY_UNITS.md section 2 (E2): Bless, Haste, Rust, Ward ... cast at the battle's start |
| L17 | Knock-back | a crit's | a shove action; immovable units (**engine**) |
| L18 | Ground per unit | the units' own city, hills, terrain and feature bonuses (both sides); the ground rules per action kind | forest and hill fighters; mounted units weaker in a forest |
| L19 | Size | large: 2 cells, 2 slots | actions per lane it fills; cover for the cells behind it |
| L20 | Splash, collateral limit, combat limit | per unit | finishing against softening |
| L21 | Morale | Fear | immunity, a rout when a side's front falls |
| L22 | Cost and limits | cost, national limits | outside the battle |

### 2.3 Attack patterns (L4)

One target is today's only pattern. Patterns, each a per-row option in `engage_rows`
**(default)**:

| Pattern | Hits | For |
|---|---|---|
| single | one target (today) | everyone |
| volley N | the backmost unit of N adjacent lanes, each at 100/N% **(default)** | archers: spread damage over a back line |
| pierce | every unit down one lane, front to back, at 50% **(default)** | casters' bolt: a deep, armoured lane |
| sweep | every unit of one enemy row at 35% **(default)** | siege in the field, area spells |
| cleave | the front units of its lane and the two beside it, 60% each **(default)** | berserkers: wide, cheap fronts |
| trample | the target and the unit behind it in its lane, 60% for the second **(default)** | chariots, knights |
| heal row | every hurt ally in one row | healers |

## 3. The counter web

The archetypes and the cycle they should form. Each is beaten by at least one other; the cycle at
the core is Missile > Line > Guard > Cavalry > Missile.

| Archetype | Stands | Hits | Beats | Beaten by | Made so by |
|---|---|---|---|---|---|
| Line (heavy infantry) | front | the enemy's front | Guard, Breaker | Missile, Caster | L1 vs Guard, L11 armour |
| Guard (pikes, anti-cavalry) | front | the enemy's front; takes the charges (anti-flank) | Cavalry, Assassin (as bodyguard) | Line, Caster | L1/L2 vs mounted, L9 |
| Shock cavalry | front | the front, trampling | Line, Missile in the open | Guard, Breaker | L3, L4 trample, L2 vs archers |
| Raider (light cavalry) | front, dives | the back line (charge) | Missile, Caster, Support, Siege | Guard, anti-cavalry Missile | L1 vs back-line types, L13 pursuit |
| Missile | middle, rear | the enemy's back line | Line, Breaker | Raider, Assassin | L1 vs melee, L4 volley, L12 |
| Anti-cavalry missile | middle, rear | the back line | Cavalry | Line, Assassin | L1 vs mounted, L11 piercing |
| Skirmisher (recon) | any row | front (strikes) and back (shots) | Caster, Siege; on rough ground | Missile (accuracy), pursuit | L10, L18, L13 |
| Caster | rear | the back line, or down a lane | Line, Guard, Breaker (low magic defence) | Raider, Assassin | L4 pierce, L5 |
| Assassin | middle | anyone (stab) | Caster, Support, Missile | Guard (bodyguard), armour | L1 vs casters and healers, L1 -50% vs melee, L11 |
| Support (healer, charms) | rear | allies | lengthens fights, wins sieges of several turns | Raider, Assassin | L15, L16 |
| Breaker (large) | front, 2 lanes | both lanes' fronts | Line, Cavalry | Caster, Missile, Siege | L19, L1 |
| Siege | rear | Sentinels, then the back line | walled cities (the siege) | Raider | L1 vs Sentinels and large, L4 sweep |

## 4. Proposals per unit

The generic units (each class's default unit) the lab tests, by tech level. "Role" is the
archetype of section 3; magnitudes are **(default)**, for the lab to tune. "Engine" marks a lever
that needs code.

### 4.1 Early (tier 1-2)

| Unit | Role | Levers | Why |
|---|---|---|---|
| Axeman | Line | the baseline; Formation (below) turns it into a Guard | P1 needs a Guard reachable without XP |
| Warrior, Scout | leftovers | none (obsolete once Archery and Bronze Working are in) | |
| Archer | Missile | L1 +50% against melee; L12 a round-1 volley (+50%, engine) | today it only trades with the enemy's archers |
| Skirmisher | Missile (national, 2 a player) | none (above its era's curve by design) | |
| Horseman | Raider | L1 +50% against archers, adepts, disciples and siege (today +40% against archers); nothing more | stays the answer to back lines; the Guard answers it |
| Hunter | Anti-cavalry skirmisher | L1 +75% against mounted, beasts and animals; L3 two shots from the middle | the early answer to P1 without a promotion; distinct from the Axeman |
| Chariot | Shock cavalry | L4 trample (engine); L2 +100% against archers (armour); L3 three charges from the front | P5: a Horseman at twice the cost today; this makes it the front-breaker the Horseman is not |
| War Elephant | Breaker (large) | L19 a strike set for each lane it fills (engine), or L3 four charges; keep +50% against mounted; L1 for archers +50% against it (engine: large as a target type) | P4; a second answer to cavalry |
| Adept | Caster | L4 pierce for the bolt (engine) or L3 three bolts from the rear; the sphere-I spells of docs/EARLY_UNITS.md section 2 (L5) | P3 |
| Disciple | Support | L15 heal a row (engine); L3 two heals in the middle, three in the rear; L5 holy strikes for the good religions, L6 undead and demons take +50% holy | P2 |
| Catapult | Siege | its place is the siege; in battle L4 sweep (engine) or L1 +100% against Sentinels and large units | keeps the field weak, the siege strong |

**Formation** (promotion; the Guard for any line unit): L1/L2 against mounted from +40% to +100%
**(default)**, or with the engine's `resist_vs` -50% damage from mounted units; anti-flank as
today. With anti-flank pulling the charges onto it, a front of two Formation units should turn a
cavalry dive into a loss; the lab measures how many it takes.

### 4.2 Mid (tier 3)

| Unit | Role | Levers | Why |
|---|---|---|---|
| Champion | Line | keep +25% against melee; L11 armour 4 (engine) | the Line that breaks Guards; armour against swarms (P6) |
| Longbowman | Missile against infantry | L1 +50% against melee; L4 volley 2 (engine) | P5: today the Assassin or a Fire II Mage does its job better |
| Horse Archer | Raider, or mobile missile | rows charge 2 / shoot 2 / shoot 1: it charges from the front and shoots from the middle | two roles, chosen by where it stands |
| Mage | Caster | L4 pierce for the bolt; the sphere spells by damage type (L5) against resistances (L6) | P3; the spheres give a choice of element |
| Priest | Support | L15 heal a row; L16 Bless (docs/EARLY_UNITS.md E2); holy strikes | P2 |
| Assassin | Assassin | L1 +50% against adepts, disciples, archers and siege, -50% against melee; L8 goes for casters and healers (engine) | it stays the answer to back lines, and stops winning front fights |
| Ranger | Skirmisher of rough ground | Woodsman and Guerilla built in (L18); L3 two shots from the middle | where the charge is off, it is the mobile fighter |
| Crusader (the Order) | Guard with holy strikes | Formation built in; L5 holy | the religion's Guard |
| Catapult (Trebuchet) | Siege | as early | |

### 4.3 Late (tier 4; at most 4 of each national class at the normal price)

| Unit | Role | Levers | Why |
|---|---|---|---|
| Phalanx | Guard | Formation built in; L2 -50% damage from mounted; attack 120 to 90, defence 120 to 130 **(default)** | today it is the best line unit and beats the Immortal outright (P5); as a Guard it gives up the Line's job |
| Immortal | Line, against casters | L6 50% against fire, cold, lightning and death; its rebirth on the map | the front that casters cannot break |
| Berserker | Line breaker | L4 cleave (engine); L12 +15% a round (engine); its low defence stays | against wide, cheap fronts (P6) |
| Knight | Shock cavalry | L4 trample (engine); its 3 charges (tier 4) | distinct from the Horse Archer's raiding |
| Paladin (good), Eidolon (evil) | Holy or unholy Line | L5 holy or unholy strikes; L6 the opposite at +50% damage taken | the alignment's answer to undead, demons or angels |
| Beastmaster | Recon with beasts | L1 +75% against beasts and animals; brings trained beasts (docs/EARLY_UNITS.md section 3) | |
| Crossbowman | Anti-cavalry missile | L1 +75% against mounted; L11 armour piercing | P5: it stops being a weaker Marksman |
| Marksman | Sniper | its crits (Marksman); L8 goes for casters and the most dangerous unit (engine) | |
| Arquebus | Mass missile (no limit) | L10 accuracy -15; L11 armour piercing; L12 shoots in rounds 1 and 3 only (engine) | cheap volume with a cost; today it is the best ranged unit with no limit |
| Cannon | Siege | as the Catapult; L1 +100% against large units | |
| Archmage (4) | Caster | L4 pierce or sweep for its spells | |
| High Priest (4) | Support | L15 heal a row at twice a disciple's heal; L16 a side-wide charm | P2: today it heals exactly as a Disciple does |
| Druid (4) | Nature caster and healer | L5 poison spells; L15 heals; L16 entangle (a hex) | |

## 5. Ground (L18)

The ground should favour roles, not decide battles (P7). Options, each a define or the units'
own bonuses: hills' attacker penalty from 75% to 85% **(default)**; mounted units -25% in a forest
(on top of no charge) so the Ranger and Guard have their ground; the walls as they are, now that
the siege is in the lab.

## 6. How to measure

- **Formations:** with the auto formation out of scope, each squad should fight in its best
  formation. The lab's `@front|@middle|@rear` overrides do it by hand; a formation search per squad
  (try each unit's rows, keep the best against a reference field) is the tool to build.
- **Targets**, per tech level and ground (`battles_meta.py`): no squad that nothing beats by 2
  standard errors; every unit in the equilibrium support (5% or more) of some ground, or in a top-5
  squad; no unit strictly dominated by a unit of the same cost and availability (`swaps`).
- **Squads:** the generator adds each new role's squads (Guard fronts, shock and raid cavalry
  together, caster lanes, healer-backed lines).

## 7. Open questions for the owner

1. L1/L2: keep them as A and D percentages (flattened, section 1) or add damage multipliers?
2. The early Guard: the Hunter as anti-cavalry skirmisher, Formation on the Axeman, or a new unit?
3. Which attack patterns (2.3) to build: volley, pierce, sweep, cleave, trample, heal row.
4. Flat armour (L11): yes or no, and how much.
5. Siege engines in the field: keep them weak (their place is the siege), or give them a sweep?
6. The ground: should hills stay stronger than walls?
7. The late duplicates (Crossbowman, Marksman, Arquebus): three roles as above, or fewer units?
8. HP by tier (L14) for P6, or armour and patterns only?

## 8. Build order (if accepted)

1. Data only (no engine): L1, L2, L3, L5, L6 values above; Formation; the Phalanx's and the
   Arquebus's numbers. Run the lab after each group.
2. Engine: attack patterns (L4), damage multipliers by type (L1, L2), flat armour (L11), target
   priority (L8).
3. docs/EARLY_UNITS.md's E2 (charms and hexes) for the supports.
4. A formation search in the lab.
