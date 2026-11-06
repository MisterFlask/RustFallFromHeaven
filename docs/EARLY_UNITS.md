# Early units: recon in any row, sphere-I battle magic, trained creatures

**Status: proposal** (2026-11-06), for the owner to mark up. Sections 1 and 2 are built (with the
differences their sections give); section 3 is not. It follows the
owner's direction of 2026-11-06: more variety in the early unit lineup, arcane units that matter
before Sorcery, recon units that fight from the front and the back (the Ogre Battle 64 Valkyrie
pattern), and trained creatures, which also bring large units early. Values marked **(default)**
are Claude's proposals for points the owner has not decided.

## 0. Why

**The lineup.** Before tier 3, the units most civilizations train are the Warrior, the Scout and
the Skirmisher (tier 1; the Skirmisher, an early national archer, since 2026-11-06), and the tier-2
melee unit (Axeman, or the Swordsman that replaces it for seven civilizations), the Archer, the
Horseman, the Chariot, the Hunter, the Adept, the six disciples and the Catapult (tier 2). In a
battle (docs/ENGAGEMENTS.md) the melee, recon and beast units all fought the same way: 2
strikes from the front row, 1 from the middle and rear, every strike at the front enemy of a lane.
So a Hunter was an Axeman with other promotions (until section 1 was built).

**Arcane.** The Adept comes early. Knowledge of the Ether is 640 beakers of research from the
start, about the same as Bronze Working (720). Sorcery, which brings the Mage, is not unusually
late: 11 techs, 5,940 beakers, near Iron Working (4,920) for tier-3 melee. The problem is that the
Adept's spheres do not reach battles:

- Every sphere-I spell is a map spell: a buff (Haste, Blur, Courage, Enchanted Blade, Dance of
  Blades, Loyalty, Fair Winds), a terrain spell (Blaze, Scorch, Spring, Sanctify), a summon
  (Skeleton, Floating Eye) or a utility (Repair, Wall of Stone, Inspiration, Treetop Defence).
- A battle uses only spells with damage (`iDamage > 0` or `battle_only`). The first are sphere II's
  Fireball and Maelstrom. So in a battle every Adept casts the same magic bolt (60%, one target),
  whatever mana its civilization holds.
- Four sphere-I effects carry into a battle when cast on the map first: Enchanted Blade (+20%
  attack, defence and magic defence, melee units only), Dance of Blades (a first strike, which is
  +2 battle speed), Rust (-10%) and Courage (immune to Fear's morale check). The others do
  nothing in a battle.

**Large units** (2 slots, 180 HP) exist, but early ones are rare: the Luchuirp's Wood Golem and Mud
Golem, and the War Elephant.

**Production.** Early cities have no spare production: in 8 AI games on master, no city was on
Build Wealth, Research or Culture through turn 300 (1% at turns 350-400, 4% at 450). So these units
are for variety, not a hammer sink. Where a unit below is national, the limit (this game's rule
since 2026-11-06: up to the limit at the normal cost, then 500% each) works as a count limit early;
the 500% copies become a sink only later.

## 1. Recon units fight from any row

**Built** (2026-11-06), without a new role: the owner's decision removed roles altogether, and
each unit type now has its own rows and rank preference (`engage_rows`, `engage_row_order`;
docs/ENGAGEMENTS.md). The 20 recon unit types that do not stab strike twice from the front, shoot
once from the middle and once from the rear, and prefer the middle row, then the front, then the
rear (docs/PORTING.md, "Changes from FFH2"). The assassins (recon with Marksman) keep their stabs.

## 2. Sphere-I battle magic

**Built** (2026-11-06, the owner's request: adepts that matter in a battle, and battle spells
beyond Fire II), with E1, E2 and E4 below; the spells as built and their numbers are in
docs/ENGAGEMENTS.md ("Battle spells"), the lab that tuned them in docs/UNIT_ROLES.md section 9.
Differences from this proposal: the damage spells are 120 for one target (Shock 80 in a VOLLEY,
Wave 70 in a PIERCE), since at 80 they were barely better than the bolt; every sphere has a charm,
hex, heal or damage spell without the summon and denial pieces (E3, E5): Earth's Tremor is a hex
(-15 evasion, -1 speed on the enemy front row), Death's Grave Touch a damage spell, Mind's Daze -10
accuracy on the enemy side, Ice's Chill a hex only; charms are stat changes of their own (damage
dealt and taken, accuracy, evasion, crit, speed), not the promotions' `stat_changes`; Law's Hold
the Line is -15% damage taken (no knock-back immunity), Spirit's Battle Hymn +20 accuracy (no Fear
immunity); a caster casts every charm it knows, in id order, then its damage spell (the most
output) or its heal, when an ally is hurt enough. Several spheres II have a stronger spell (Tidal
Surge, Sunburst, Death Grip, Frost Lance, Stone Ward, Bramble Snare, Healing Light), and the map
spells that only dealt damage became battle spells (Wither, Contagion, Crown of Brilliance, as
Maelstrom before them). The map spells stay as they are. The AI counts a caster with any battle
spell as one that marches with the army and values each battle spell a sphere opens as Fireball's.

**Rules (as proposed):**

1. **Damage spheres.** The sphere-I spell becomes a battle spell of the sphere's damage type at
   power 80, one target, in place of the magic bolt (60). Sphere II's Fireball and Maelstrom stay
   at 100 with a splash.
2. **Charms and hexes.** A charm (on the caster's side) or a hex (on the enemy's) is cast with the
   caster's first action of the battle and lasts the battle. Its other actions are its damage spell
   or the bolt.
3. **Several sphere-I spells.** One charm or hex per action from the caster's first actions, in
   sphere order, then its damage spell for the rest.
4. The map spells stay as they are.

| Sphere (node: school tech) | Map spell (FFH2) | Battle effect | Kind | Engine |
|---|---|---|---|---|
| Air (Elementalism) | Fair Winds | **Shock**: lightning, 80 | damage | E1 |
| Earth (Elementalism) | Wall of Stone | **Wall of Stone**: a Sentinel (docs/SIEGES.md) stands in front of the caster's lane for the battle | summon | E3 |
| Fire (Elementalism) | Blaze | **Firebolt**: fire, 80 | damage | E1 |
| Water (Elementalism) | Spring | **Wave**: physical, 60, at the front enemy of a lane, which is always knocked back a row | damage | E1 (a strike's reach, a knock-back) |
| Chaos (Necromancy) | Dance of Blades | **Wild Edge**: own front row +15 crit | charm | E2 |
| Death (Necromancy) | Raise Skeleton | **Raise the Fallen**: the first unit of either side to fall rises as a Skeleton on the caster's side for the rest of the battle | summon | E3 |
| Entropy (Necromancy) | Rust | **Rust**: enemy front row -10% attack, defence and magic defence (Rusted's numbers) | hex | E2 |
| Shadow (Necromancy) | Blur | **Blur**: own front row +10 evasion | charm | E2 |
| Body (Alteration) | Haste | **Haste**: own front row +2 speed | charm | E2 |
| Enchantment (Alteration) | Enchanted Blade, Repair | **Enchanted Blade**: own front row +20% attack, defence and magic defence (every unit combat, not only melee) | charm | E2 |
| Nature (Alteration) | Treetop Defence | **Thornlash**: poison, 80 | damage | E1 |
| Law (Divination) | Loyalty | **Hold the Line**: own front row +10% defence and is never knocked back | charm | E2 |
| Life (Divination) | Sanctify | **Mend**: its actions heal the most hurt ally (60% of a disciple's heal) | heal | E4 |
| Mind (Divination) | Inspiration | **Daze**: the enemy with the most actions loses its next one | hex | E5 |
| Spirit (Divination) | Courage | **Courage**: own side immune to Fear and +10 accuracy | charm | E2 |
| Sun (Divination) | Scorch | **Flare**: holy, 80 | damage | E1 |
| Ice (no node to build) | Slow | **Chill**: cold, 60, and the target -2 speed for the battle | damage + hex | E1, E2 |
| Metamagic (Sorcery) | Floating Eye | **Ward**: own side +20% magic defence | charm | E2 |

Metamagic's node needs Sorcery and Ice has none to build, so in practice 16 spheres matter before
Sorcery.

**Engine pieces**, in the order to build them:

- **E1** per-spell battle power and splash (new spell keys). Today every battle spell hits at power
  100 with a splash, so a sphere-I spell would hit as hard as Fireball. 6 spheres.
- **E2** charms and hexes: a battle spell that applies stat changes (the `stat_changes` format of
  promotions) to a row or a side for the battle. 9 spheres.
- **E3** summons in battle: a unit added to a side during the battle (a Skeleton, a Sentinel).
- **E4** an arcane caster whose actions heal.
- **E5** an enemy's action denied.

**AI:** none in battle (battle spells cast themselves). Its choice of sphere promotions should
weigh the battle effects (to check: how `ffh-ai` values sphere promotions today).

**Interface:** the battle log names the spell; the unit help lists each known battle effect.

## 3. Trained creatures

Creatures are trained like units, from two sources: mana nodes and wild habitats. They are national
units, so they follow the owner's rule for those (2026-11-06, with the Skirmisher): above their
era's curve, with no upgrade; the defaults below are set that way (a tier-2 unit is strength 4 for
60 hammers; a creature is 5 for 70, a large one 8 for 120). They reuse
existing art (each is a new unit type whose sprites and key poses come from an existing unit
through the client's aliases, `UNIT_ALIASES` in `iso.js`). Several are large: this is the early
route to large units.

### 3.1 Mana-bound creatures

- A city with a node of a mana type in its radius trains that mana's creature once the node's school
  tech is known (default). The four schools each cost 640 beakers after Knowledge of the Ether, so
  creatures come well before Sorcery.
- **National, priced.** A player trains as many at the normal cost as it has nodes of that mana
  (default: its own nodes, not traded mana). Each one past that costs 500%. They are not casters, so
  their limit is a price, not a hard limit.
- Each is a new unit type: the summoned creature keeps its own numbers.

| Mana (school) | Creature (art) | Size | Strength (default) | Cost (default) |
|---|---|---|---|---|
| Air (Elementalism) | Air Elemental | 1 | 5 | 70 |
| Earth (Elementalism) | Earth Elemental | large | 8 | 120 |
| Fire (Elementalism) | Fire Elemental | 1 | 5 | 70 |
| Water (Elementalism) | Water Elemental | 1 | 5 | 70 |
| Chaos (Necromancy) | Chaos Marauder | 1 | 5 | 70 |
| Death (Necromancy) | Skeleton (2 per training) | 1 | 3 | 70 |
| Entropy (Necromancy) | Pit Beast | 1 | 5 | 70 |
| Shadow (Necromancy) | Mistform | 1 | 5 | 70 |
| Body (Alteration) | Flesh Golem | large | 8 | 120 |
| Enchantment (Alteration) | Wood Golem | large | 8 | 120 |
| Nature (Alteration) | Treant | large | 8 | 120 |
| Law (Divination) | Einherjar | 1 | 5 | 70 |
| Sun (Divination) | Aurealis | 1 | 5 | 70 |
| Life, Mind, Spirit (Divination) | none with art yet | | | |

### 3.2 Beasts

- A city with a Hunting Lodge trains the beasts whose habitat is in its radius (default). The
  Hunting Lodge already unlocks the recon units, and its tier acts on the units it unlocks (so a Grand
  Hunting Lodge's beasts would start with +4 XP, docs/SUITABILITY.md).
- National, priced: 4 of each at the normal cost (default), then 500%.
- FFH2's animals carry the barbarian-animal flag (they cannot enter borders and so on), so trained
  beasts are new unit types (unit combat BEAST) that reuse the animals' art.

| Habitat in the radius | Beast (art) | Size | Strength (default) | Tech (default) | Cost (default) |
|---|---|---|---|---|---|
| Forest | Wolf Pack | 1 | 4 | Hunting | 50 |
| Forest or tundra | Bear | 1 | 5 | Hunting | 70 |
| Jungle | Tiger | 1 | 5 | Hunting | 70 |
| Jungle | Gorilla | 1 | 5 | Hunting | 70 |
| Desert | Scorpion | 1 | 5 | Hunting | 70 |
| Ivory | Elephant | large | 9 | Animal Handling | 120 |
| Hills | Hill Giant | large | 9 | Animal Handling | 120 |
| Mountains | Griffon | 1 | 6 | Animal Handling | 80 |

**Engine:**

- A national limit from a count (the player's nodes of a mana), a new unit class key; otherwise the
  class's `max_player_instances`.
- A training prerequisite "in the city's radius" (a mana node, a feature, a terrain, a bonus); FFH2's
  prerequisites read the city's trade network, not its radius.
- The client's art aliases.
- **AI:** none expected beyond the rules allowing it: its unit choice (power squared per build turn)
  picks a creature when it is the best unit for the job, and since the Skirmisher it weighs a
  national unit at the over-limit price once its units alive and in production fill the limit.

## 4. Covens (later)

Several Adepts with the same sphere-I spell in one stack cast that sphere's sphere-II effect
together (three Fire-I Adepts: a Fireball). Early magic gets a route through numbers instead of
tech, and it is the nearest thing to an early hammer sink. It is the costliest piece: counting
casters in a stack, combining them in a battle, and an AI that stacks them.

## 5. Open questions for the owner

1. (Decided and built: recon rows 2 strike / 1 shoot / 1 shoot, preferring the middle row.)
2. Sphere-I damage: power 80 on one target?
3. Charms on the front row (as above) or the whole side?
4. The table of battle effects: any to change.
5. Creatures: the list; Enchantment's Wood Golem is also the Luchuirp's own unit (Sculptor's
   Studio): keep both, or give Enchantment another creature.
6. A mana creature's limit: the player's own nodes, or every source of that mana including trade?
7. Where a creature is trained: only a city with the node or habitat in its radius (a city
   identity from the map), or any city once the player has the mana?
8. Life, Mind, Spirit (and Ice, Metamagic): creatures need new art.

## 6. Build order

1. Recon rows and rank preference (built 2026-11-06).
2. E1 and E2 (14 of the 18 spheres).
3. Beasts and mana-bound creatures (new unit types, the limit from a count, the radius
   prerequisite, art aliases).
4. E3 to E5 (the other spheres).
5. Covens.

Each step is a rules change, recorded in docs/PORTING.md ("Changes from FFH2").
