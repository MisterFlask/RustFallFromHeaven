# Engagement combat

This game's own land combat (not FFH2's), and the game's only land combat: it is not an option
(the owner's decision, 2026-11-03: `GAMEOPTION_ENGAGEMENTS` was removed, and with it the
requirement of `GAMEOPTION_ARMIES`, which nothing in a battle needs: an army commander's aura
applies only to units in an army). Every game fights engagements, the AI-only games of
`ffh autoplay`, `ffh-stress` and `ai_metrics.py` too (AI measurements from before 2026-11-03 are of
the duel); saves made before do not load (the game options changed). When units attack a land plot, they fight a **formation battle** (revision 4, the
owner's Ogre Battle-style engagements, 2026-10-31, designed in a browser prototype): each side's
units stand on a grid of three rows by its lanes and act by row and speed for three rounds;
the plot falls only when no defender is left on it. The owner's answers on every FfH combat
ability (Q1-Q13) and the ground rules G1-G5 are in docs/COMBAT_INVENTORY.md (its answers about
rounds, stances and targeting are superseded by revision 4: this document).

Naval and air combat stay the duel.

Code: `crates/ffh-core/src/engagement/` (`mod.rs` lists the modules): the pure resolver in
`formation/` (`formation::resolve`, the auto formation `formation::auto_arrange`); on `Game` the
dispatch rule (`state.rs`), the units' rows, HP and A and Dv per hit (`stats.rs`),
the slots (`slots.rs`), who can fight (`pick.rs`), where each side's units stand (`arrange.rs`),
the engagement (`engage.rs`, `apply.rs`, `sink.rs`), the reports and the battle log
(`report.rs`), and the attack preview (`preview.rs`). Tests: the resolver's in
`formation/tests.rs`; scenarios in `crates/ffh-core/tests/engagements.rs`; slots, who fights and
the formations in `tests/engagement_pick.rs`; the preview in `tests/engagement_preview.rs`; the
units' stats in `tests/engagement_stats.rs`; state and saves in `tests/engagement_state.rs`; the
data in `crates/ffh-data/tests/engagement_data.rs` and `engagement_overrides.rs`.

## The rules

**The field.** Slots per side as before (`slots.rs`: 8 base, +2 open flat land, -2 a forest or
jungle, -2 hills; a walled city's standing Sentinels cost each side one each (docs/SIEGES.md: a
city's defence costs no slots); at least 3; across a river two of the attacker's slots are only for
`bRiver` units, Q9). A walled city's Sentinels stand in the defender's front row beside its
soldiers' slots (docs/SIEGES.md). Each side stands on a grid of 3 rows
(front 0, middle 1, rear 2) by `max(2, ceil((slots + 2) / 3))` lanes (`formation::lanes_for`, of the
side's ordinary and river slots together; the sides' lane counts may differ). A large unit (`bLarge`,
below) fills two cells side by side in one row and counts 2 slots, and is the front or back unit
of both its lanes.

**Each unit's own rows** (the owner's decision, 2026-11-06: there are no roles). Every land unit
type has, in `units.jsonc`:

| Key | What | Absent |
|---|---|---|
| `stats` | its numbers (docs/UNIT_STATS.md) | warns |
| `engage_rows` | what it does from each row: three rows, front, middle and rear, each an `action` and its `count` | warns; no action from any row |
| `engage_row_order` | its rank preference: `FRONT`, `MIDDLE` and `REAR`, once each, in the order the auto formation tries them | warns; front, middle, rear |
| `large` | it fills two cells of a row and counts 2 slots | one cell |

The battle levers (docs/UNIT_ROLES.md, built 2026-11-06), each optional:

| Key | Where | What | Absent |
|---|---|---|---|
| `pattern` | a row of `engage_rows` | whom else the row's action hits (below) | `SINGLE` |
| `damage_vs` | unit, promotion | percent more damage to units of each unit combat (below 0: less) | 0 |
| `resist_vs` | unit, promotion | percent less damage from units of each unit combat (below 0: more) | 0 |
| `damage_vs_large`, `damage_vs_walls` | unit | percent more damage to large units, to a city's Sentinels | 0 |
| `armor` | unit, promotion | HP off each physical hit it takes (at least 0.01 HP gets through) | 0 |
| `armor_piercing` | unit, promotion | its hits ignore armor | no |
| `pursuit` | unit, promotion | a hit of it cannot be withdrawn from | no |
| `heal_percent` | unit | its heal, in percent of a hit | `ENGAGEMENT_HEAL_PERCENT` |
| `target_preference` | unit | unit combats it goes for first | none |
| `round_percent` | unit | its hits' damage in rounds 1, 2 and 3 (and later), percent | 100 each |

A unit's `damage_vs`, `resist_vs` and `armor` are its type's plus its promotions'.

```jsonc
"engage_rows": [{"action": "STRIKE", "count": 2}, {"action": "STRIKE", "count": 1}, {"action": "STRIKE", "count": 1}],
"engage_row_order": ["FRONT", "MIDDLE", "REAR"],
```

An action is STRIKE, CHARGE, STAB, SHOOT, SPELL, BOMBARD, HEAL or NONE. A row's count is above 0
exactly when its action is not NONE, and some row acts; rows that are not three, an unknown action
or rows that break those rules warn at load and read as none. A unit's **kind** is what it does from
the row it prefers (the first of its rank order); the auto formation sorts by it. A striker that
holds a promotion with `anti_flank` (Formation) is **anti-flanking** (Q7). A **flanker** is a unit
whose rows have a CHARGE.

When the roles went (2026-11-06), each unit type's rows and rank order were written from the role
it had, so every battle played the same (same seeds, same game checksums). Those were:

| Was | Unit types | Front | Middle | Rear | Rank order |
|---|---|---|---|---|---|
| Frontline | melee, animals, beasts, recon, units without a unit combat | 2 strike | 1 strike | 1 strike | front, middle, rear |
| Screen | a frontline unit with Formation: anti-flanking | 2 strike | 1 strike | 1 strike | front, middle, rear |
| Flanker | mounted | 2 charge | 1 strike | 1 strike | front, middle, rear |
| Assassin | recon with Marksman free | 1 stab | 2 stab | 1 stab | middle, rear, front |
| Archer | archers, the Airship | 1 shoot | 2 shoot | 2 shoot | middle, rear, front |
| Siege | siege | none | 1 bombard | 2 bombard | rear, middle, front |
| Arcane | adepts | 1 strike | 1 spell | 2 spell | rear, middle, front |
| Divine | disciples (seven that never cast were Frontline) | 1 strike | 1 heal | 2 heal | middle, rear, front |

Since then (each recorded in docs/PORTING.md, "Changes from FFH2"): the recon units that do not
stab strike twice from the front and shoot once from the middle and the rear, and prefer the
middle row, then the front, then the rear (2026-11-06).

A unit's numbers (HP, attack, defence, magic, magic defence, accuracy, evasion, crit, speed) are its
own stat sheet's (docs/UNIT_STATS.md; below). The sheets began as the roles' old values: HP 120
frontline, 100 flanker, 80 archer and siege, 70 arcane and divine, 60 assassin; attack and defence
the unit's strength times the role's percent (flanker 110/80, assassin 120/60, archer and siege
100/60, arcane and divine 100/50, screen defence 110); speed 4, screen 3, flanker 6, assassin 7,
archer 4, siege 1, arcane and divine 3; evasion 10 for flankers and 15 for assassins, crit 10 and
25 for assassins; accuracy 90.

Units of tier `ENGAGEMENT_EXTRA_ACTION_TIER` (4) and above and heroes get
`ENGAGEMENT_EXTRA_ACTIONS` (1) more actions in their best row, the row with the most (the first
such). Speed is the unit's (below) plus `ENGAGEMENT_FIRST_STRIKE_SPEED` (2) per first strike and per
chance first strike rolled at the battle's start (first strikes are speed now; immunity to first
strikes has no effect in an engagement).

**Per-unit values** (the owner's requests, 2026-10-31 and 2026-11-06; docs/UNIT_STATS.md).
Promotions change a unit's stats (`stat_changes`); a missing or out-of-range value warns. One
accessor per value resolves the unit: `Game::unit_stat` and `unit_engage_max_hp`, `_speed`,
`_accuracy`, `_evasion`, `_crit` (its sheet, its own points and percentages), `unit_engage_rows`,
`unit_engage_best_row`, `unit_engage_actions` (with the extra action), `unit_engage_row_order`,
`unit_engage_kind`, `unit_engage_anti_flank`, `unit_engage_flanks`, `unit_engage_large`,
`unit_engage_spell` (stats.rs, arrange.rs); the rules' fighters, the auto formation, the preview,
the view and the client read these.

**Action kinds and reach** (`ffh_data::ActionKind`):
- STRIKE: the front unit of each enemy lane; physical; a crit knocks the target back.
- CHARGE: where flanking works (the defended plot has no city and no forest or jungle), every
  enemy on the field, the target the one with the highest threat for its HP left (threat: its A
  times its actions from its current row; compared `threat_x x hp_y`), and in the first round
  `ENGAGEMENT_CHARGE_BONUS` (100) % more damage (the cavalry's first-round charge in the field, the
  owner's request). Elsewhere it is a STRIKE (front reach, no bonus, the normal choice; the log says
  strike). Physical; a crit knocks back.
- STAB: any enemy. Physical.
- (Sieges, docs/SIEGES.md: a standing Sentinel of the defended city takes the SHOOT and SPELL aimed
  down its lane, and BOMBARD takes the Sentinels first.)
- SHOOT: the backmost unit of each enemy lane. Physical, `ENGAGEMENT_SHOOT_MISS_PERCENT` (10) more
  miss.
- SPELL: the backmost unit of each enemy lane; never misses, never crits. A caster that knows a
  damage spell (below) casts it at its `battle_power` (100 when absent) with its `battle_pattern`
  (none: a splash); one that knows none casts the magic bolt, `ENGAGEMENT_BOLT_PERCENT` (60), one
  target (or its row's pattern). The log names the spell.
- **Battle spells** (the owner's decisions, 2026-11-05 and 2026-11-06). A unit's battle spells
  (`Game::unit_battle_spells`, `GameData::engage_battle_spells`) are of the spells whose
  promotion, unit, unit class, unit combat and religion prerequisites it meets (not a Wonder's
  outcomes, below):
  * its **damage spell**: of the spells with `iDamage > 0`, `battle_only`, a `battle_power` or a
    `battle_wonder` (and no heal or charm), a Wonder first, then the most output
    (`GameData::battle_spell_output`: the power times `100 +` what its pattern adds when every
    extra target is there: a splash two at 50%, VOLLEY, PIERCE and CLEAVE two, SWEEP three,
    TRAMPLE one, each at its percentage), then the highest `iDamage`, then the lower id;
  * its **heal** (`battle_heal`: percent of a hit, `battle_pattern` SINGLE or SWEEP), the
    strongest: cast with a SPELL action in place of the damage spell when an ally on the field is
    missing at least half of the heal (the most hurt ally, as HEAL);
  * its **charms and hexes** (`battle_charm`: `{"side": "OWN"|"ENEMY", "rows": "FRONT"|"ALL",
    "damage", "taken", "accuracy", "evasion", "crit", "speed"}`), every one it knows but those of
    a lower sphere tier than its damage spell (`GameData::spell_tier`: Fire III's Firestorm is 3,
    Law I's Hold the Line 1, so an archmage with both does not spend its opening action on Hold the
    Line; the bolt is 0), cast with its first SPELL actions, one an action, the highest tier first
    (Earth III's before Earth II's before Earth I's), then by id, and none in the battle's last
    round (there it casts its damage spell or heal: a charm then would last a pass or two); each draws
    nothing and lasts the battle, on the units of that side on the field as it is cast (the front
    row: row 0 then). `damage` and `taken` are percent more damage dealt and taken (each hit times
    `(100 + dealt)(100 + taken)` before armor); accuracy, evasion, crit and speed are points
    (speed orders the passes).

  So a caster of several spheres casts its charms and hexes of its damage spell's tier or above,
  then one damage spell, the same at every target (its damage type is not chosen against each
  target's resistances), or its heal when an ally is hurt enough: a mage with Fire I and
  Enchantment I casts Keen Edge, then Firebolt; an archmage with Fire III and Law I casts only
  Firestorm; one with Fire III and Law III casts Aegis, then Firestorm; one with Earth I to III
  casts Quake, Stone Ward and Tremor, then the bolt.

  **Wonder** (Chaos III; the owner's request, 2026-11-06): a `battle_wonder` (`{"SPELL_X":
  weight, ..}`) is a damage spell whose every cast rolls one of its outcomes by weight ("Engagement
  wonder", the action's first draw) and casts it as the caster's own: a damage spell (of Wonder's
  damage type: none, so no resistance cuts it), a heal (nothing when no ally is hurt) or a charm or
  hex. The outcomes are `battle_only` spells of their own (`SPELL_WONDER_*`), known by no unit
  (`wonder_outcome`); the log names the outcome. Wonder stays a map spell as well.

  | Outcome | Weight | Effect |
  |---|---|---|
  | Starfall | 3 | power 170, splash |
  | Thunderclap | 2 | power 200, VOLLEY |
  | Smite | 2 | power 300, one target |
  | Blessing | 2 | own side +20% damage, +10 accuracy |
  | Sanctuary | 2 | own side -20% taken |
  | Doom | 2 | enemy side +20% taken, -10 evasion |
  | Renewal | 2 | heal 120, SWEEP |
  | Time Warp | 1 | own side +6 speed, +15 evasion |
  | Backfire | 1 | own side -15 accuracy |
  | Butterflies | 1 | nothing |

  A unit with magic that knows any battle spell casts with SPELL in place of each attack, in every
  row: its strikes, charges, stabs, shots and bombards are SPELL actions, the same counts
  (`rows_with_battle_spell`); a healer (a row that heals) keeps its rows. So a Fire II mage casts
  Fireball from the front row too, and Meshabber, Mary and Auric Ascended (heroes with a battle
  spell of their own, given a caster's magic: 110% of their attack) cast Fireball, Contagion and
  Greater Snowfall in place of their strikes. A `battle_only` spell is cast only in battle, never
  on the map (`unit_can_cast` refuses it). Every sphere has a battle spell at each of its three
  promotions but a few at II (the owner's requests, 2026-11-06: adepts that matter in a battle,
  battle spells beyond Fire II, and sphere III):

  | Sphere | I | II | III |
  |---|---|---|---|
  | Air | Shock: lightning 80, VOLLEY | Maelstrom: lightning 100, splash | Chain Lightning: lightning 150, VOLLEY |
  | Body | Quicken: own front row +5 speed, +10 evasion | | Surge: own side +6 speed, +20 evasion |
  | Chaos | Wild Edge: own front row +35 crit | | Wonder (above) |
  | Death | Grave Touch: death 120 | Death Grip: death 150 | Finger of Death: death 200 |
  | Earth | Tremor: enemy front row +5% taken, -15 evasion, -2 speed | Stone Ward: own front row -25% taken | Quake: enemy side -20 evasion, -3 speed, +10% taken |
  | Enchantment | Keen Edge: own front row +20% damage | | Runeblades: own side +35% damage |
  | Entropy | Corrode: enemy front row +20% taken | | Wither: enemy side +20% taken, -10 accuracy |
  | Fire | Firebolt: fire 120 | Fireball: fire 100, splash | Firestorm: fire 150, splash |
  | Ice | Chill: enemy front row +5% taken, -15 evasion, -3 speed | Frost Lance: cold 100, PIERCE | Snowfall: cold 150, splash |
  | Law | Hold the Line: own front row -15% taken | | Aegis: own side -20% taken |
  | Life | Mend: heal 68 | Healing Light: heal 80, SWEEP | Restoration: heal 130, SWEEP |
  | Metamagic | Ward: own side -10% taken | | Disjunction: enemy side -20% damage |
  | Mind | Daze: enemy side -10 accuracy | | Confusion: enemy side -20 accuracy, -3 speed |
  | Nature | Thornlash: poison 120 | Bramble Snare: enemy front row +15% taken, -20 evasion, -4 speed | Wrath of the Wild: poison 160, CLEAVE |
  | Shadow | Shadow Veil: own front row +15 evasion | | Nightfall: own side -10% taken, +25 evasion |
  | Spirit | Battle Hymn: own side +5% damage, +10 accuracy | | Exaltation: own side +10% damage, +20 accuracy, +30 crit |
  | Sun | Flare: holy 120 | Sunburst: holy 100, VOLLEY | Solar Wrath: holy 140, splash |
  | Water | Wave: physical 70, PIERCE | Tidal Surge: physical 90, PIERCE | Deluge: physical 150, PIERCE |

  (Power without a pattern: one target.) The map spells that only dealt damage are battle spells
  now: Fire II's Fireball (FFH2's summoned a missile), Air II's Maelstrom (FFH2's struck the units
  within 2 plots), Entropy III's Wither (a hex in battle), Mary's Contagion and the Empyrean's
  Crown of Brilliance. Snowfall and Ring of Flames, which also change the land, stay map spells
  (and are their casters' damage spells in a battle: Snowfall with its own power, Ring of Flames
  at 100 with a splash). The other map spells of the spheres (the summons, Haste, Wall of Stone,
  ...) stay as they were. The Arcane Barge, a ship, keeps a Fireball of its own
  (`SPELL_FIREBALL_BARGE`), the missile.
- BOMBARD: the backmost unit of each enemy lane; physical, `ENGAGEMENT_BOMBARD_MISS_PERCENT` (15)
  more miss; never crits.
- HEAL: the most hurt ally on the field (most HP missing; ties by lane, then row), itself
  included: `ENGAGEMENT_BASE_DAMAGE x` the healer's `heal_percent` (else `ENGAGEMENT_HEAL_PERCENT`,
  50) `x r`. No one hurt: the action is spent. A heal with the SWEEP pattern also heals every other
  hurt ally in its target's row for `ENGAGEMENT_SWEEP_PERCENT` of the heal (one log event each).
- **Attack patterns** (a row's `pattern`): the action's extra hits, with the target's hit (no miss
  or crit roll of their own, r drawn, withdrawal as any hit), never crit, at a percentage of a hit:
  VOLLEY (`ENGAGEMENT_VOLLEY_PERCENT`) the unit the kind would hit in each lane beside the target's
  (the backmost, a Sentinel first, for a shot or spell; the front for a strike); PIERCE
  (`_PIERCE_`) every other unit in the target's lanes; SWEEP (`_SWEEP_`) every other unit in the
  target's row; CLEAVE (`_CLEAVE_`) the front unit of each lane beside the target's; TRAMPLE
  (`_TRAMPLE_`) the nearest unit behind the target in its lanes. A pattern takes the place of a
  collateral splash; a known spell's splash stays the spell's (a caster's bolt follows the row's
  pattern). The log marks the extra hits as splash hits.
- Splash: a unit with collateral damage (`iCollateralDamage > 0`: siege, Berserker, Fire
  Elemental, ...) splashes its damaging hits (not a miss) on the target's neighbours (beside it in
  its row, in front and behind in its lanes) for `ENGAGEMENT_SPLASH_PERCENT` (50) % of a hit on
  each, never taking one past its collateral limit (`iCollateralDamageLimit`). As the duel's
  collateral damage, that splash is times `100 +` the splasher's extra collateral damage (Barrage
  I-III's `iCollateralDamageChange`: +40, +60, +100) and times `100 -` the splashed unit's
  collateral damage protection (Drill III and IV's `iCollateralDamageProtection`: 20 each) percent;
  a unit with 100 or more protection is not splashed (and draws nothing). A known spell splashes
  the same 50% as the spell: no limit, no Barrage, no protection (a caster's bolt is not a spell's
  splash: a unit with collateral damage splashes it as collateral damage). Splash hits never crit.
  This replaces the round model's collateral step before the battle (Q4).
- Siege (combat limit below 100, attackers only): its hits never take a target below
  `(100 - limit)%` of its max HP, and such targets are not in its reach.
- Anti-flank: whatever a flanker can reach (with any of its actions), anti-flanking units
  in that reach are taken first.

**Order.** `ENGAGEMENT_ROUNDS` (3) rounds; in each, pass k (k from 1 to the most actions any unit
on the field has from its row at the round's start) lets every unit on the field with at least k
actions from its current row act once. Within a pass the faster act first, equal speeds by a coin
each mover draws at the pass's start (`0..65535`: the prototype's `2^20` does not fit the game's
CvRandom; equal coins keep fighter order). One unit acts at a time and each hit lands at once; a
unit killed, withdrawn or knocked to a row with fewer actions before its turn in a pass does not
act in it. The battle ends after the last round, or at once when a side has no unit left on the
field.

**Targeting** (everyone, the player does not choose; `Formation::choose`):
1. A flanker's pool is the anti-flank units in its reach when there are any.
2. **Target preference** (`target_preference`): the units of its preferred unit combats in the
   pool, when there are any, wherever they stand (nearby first does not apply to them).
3. **Nearby first** (the owner's rule, 2026-10-31): of the pool, the units whose lanes are within
   one lane of the hitter's (`Formation::lane_distance`: the least over both units' lanes, a large
   unit's two; lanes are numbered the same on both sides) come first; only when none of those is
   in reach does it consider the rest. A charge where flanking works keeps going for its target
   wherever it is; heals are not affected; a splash's centre is chosen by this rule too.
4. A charge where flanking works takes the highest threat for its HP left; otherwise, if this hit's
   expected damage would kill some, the one with the highest A among them, else the one it hurts
   most for its HP left (expected / HP, by cross-multiplication). The expected damage is capped at
   what the hitter's combat limit lets it take, so a siege engine takes no kill. Ties by the higher
   A (the unit's A against nobody in particular), then lane, then row, never by list order.

**Hits.** Expected damage, centi-HP: `ENGAGEMENT_BASE_DAMAGE (40) x 100 x A / (A + Dv)` (an even
hit 20 HP; six kill a frontline unit); a SPELL's A and Dv are the caster's magic and the target's
magic defence, and its damage is times `100 -` the target's resistance to the spell's damage type
(none for the magic bolt). Then times the hit's percentages (a spell's power, the ground's, the
first round's charge, the pair's type percentage: `100 +` the hitter's `damage_vs` against the
target's unit combat (and `damage_vs_large`, `damage_vs_walls`), times `100 -` the target's
`resist_vs` against the hitter's; the hitter's `round_percent`; a splash's or pattern's) over one
division. A hit: `max(1, expected x r)`, r in
`ENGAGEMENT_DAMAGE_RAND_MIN..=MAX` (80..120) %. A physical action hits with the hitter's accuracy
less the target's evasion and the kind's (`ENGAGEMENT_SHOOT_MISS_PERCENT` 10, `_BOMBARD_` 15),
within 5..100, rolled before the crit and the damage. Strike, charge, stab and shoot crit with the
hitter's crit (at most 95), for `ENGAGEMENT_CRIT_DAMAGE_PERCENT` (150) % of the hit; a strike's or a
charge's crit knocks the target back a row when every cell behind it is free (it acts from that row
then). A physical hit (a pattern's and a splash's too) loses the target's armor, at least 1 centi-HP
getting through, unless the hitter has `armor_piercing`.

**Withdrawal** replaces a killing hit: a unit with a withdrawal chance (attackers: FfH's
`withdrawalProbability`; defenders: the defensive withdrawal) that a hit (a splash too) would kill
rolls it; on a success the hit does nothing and the unit leaves the battle alive. Never a city's
defenders (Q3; the resolver enforces it), and never from a hit of a unit with `pursuit`.

**The ground** (the defended plot's, `Game::engage_ground`: a city on it; a forest or jungle,
which is a feature with `iEngagementSlotsChange` below 0: Forest, New Forest, Ancient Forest,
Jungle; hills; several multiply; `formation::Ground::rules` lists those that apply): a city: the
attacker's SHOOT deals `ENGAGEMENT_CITY_ATTACKER_SHOT_PERCENT` (50) %; a forest or jungle: SHOOT and
BOMBARD deal `ENGAGEMENT_FOREST_RANGED_PERCENT` (50) % on both sides; hills: every attacker hit
(spells too) deals `ENGAGEMENT_HILLS_ATTACKER_PERCENT` (75) %, the defender's SHOOT and BOMBARD
`ENGAGEMENT_HILLS_DEFENDER_RANGED_PERCENT` (125) %. A city, a forest or jungle stop flanking.

**Large units** (`bLarge` in `units.jsonc`): the spec's Hill Giant, War Elephant and Elephant,
the dragons (Abashi, Acheron, Drifa, Eurabatres), every golem (Bone, Clockwork, Flesh, Iron,
Mithril, Mud, Nullstone, Wood) and the Treant, and of that size class the giant heroes Gurid,
Margalard and Wilboman (a frost giant), Barnaxus (a golem), the War Machine, the Earth Elemental (a
golem of rock) and the War Tortoise. Demons (the Balor, the Pit Beast) and the great beasts (the
Giant Spider, the Manticore) stay one cell.

## A and Dv per hit

`Game::engage_hit` (stats.rs; G1, Q1, Q2, Q9): the hitter's A and the target's Dv are its base (its
sheet's attack or defence plus its own points, x10, plus its typed strength scaled by the opponent's
resistance) under one summed percentage modifier: its own percentage (`Unit::stat_pct`) and the
duel's terms. Magic and magic defence take only the unit's own points and percentage
(docs/UNIT_STATS.md). Revision 4
changed two rows of the term table (stats.rs's module comment): **the defended plot's own defence
no longer adds to Dv** (its terrain or feature cover, hills, a fort's improvement defence, the
city's building and culture defence, FfH's range defence: the duel's `plot_defense_modifier`) and
**`CITY_BARBARIAN_DEFENSE_MODIFIER` is gone** (a defended-plot term). The ground's rules above and
the slots replace them. What stays on the defending side are the unit's own terms: its
fortification, its city defence in a city (`iCityDefense`, promotions), its hills defence on hills,
its feature (else terrain) defence. Everything else is as before: the promotions' own percentages,
Homeland, Stigmata, the bonuses against the opponent (unit combat, class, promotions, domain,
animals), the handicap's on the target, the attacker's city, hills, feature and terrain attack and
the amphibious term, the river (slots, Q9). **The place bonuses count on both sides** (the owner's
rule, 2026-11-05): the defender's own terms above are on its A as well as its Dv, and the
attacker's city, hills, feature and terrain attack on its Dv as well as its A, since both sides hit
in a formation battle (G1 had put each on one side, as in the duel). The amphibious penalty stays
on the attacker's A only. A unit's A against nobody in particular
(`Game::engage_a_alone`) is a fighter's "most dangerous", its ties and the auto formation's
strength. A missile's strike (below) keeps both rows: against it the plot's own defence and the
city's barbarian defence still count in Dv (`HitCtx::cover`), as before revision 4.

## Where the units stand

`arrange.rs`. A `Place { unit, lane, row }` puts a unit in its side's grid (a large unit's first
lane).

**The auto formation** (`Game::engage_auto_formation`, over `formation::auto_arrange`, the
prototype's `autoArrange`): one unit of each kind first (what it does from the row it prefers:
`formation::kind_pick_order`), in the order anti-flanking strikers, strikers, shooters, chargers,
casters, healers, bombards, stabbers (the old roles' order: Screen, Frontline, Archer, Flanker,
Arcane, Divine, Siege, Assassin), then the rest in that order; within a kind the healthy units first (not below `ENGAGEMENT_AUTO_MIN_HP_PERCENT`), then the strongest (its
A on its side against nobody in particular, with its promotions; a large unit's counts double), then
the lower unit id; while the slots last (a large unit needs 2; units without `bRiver` at most the
ordinary slots, Q9's river rule). Then the large units are placed first, each unit in the first row
of its rank order (`engage_row_order`) with room, the lanes nearest the centre first (then the
lower); while another large unit is still to place, a large unit does not split a pair of free
cells (it starts after an even run of free cells on its left), so a row of 4 lanes holds two. Deterministic, no RNG, the same for humans and the AI. Heroes and
veterans are not held back. The attacker fights it when its order gives no formation; the defender
always does (`Game::engage_defense_formation`: the hook for standing formations per army, a later
step).

**An ordered formation** (`Game::engage_validate_formation`, the `engage` command's): every unit in
the attacker's pool once, inside its side's lanes and rows, no two units in one cell (a large unit
on two free cells of one row), at most the attacker's slots (a large unit counts 2), the units
without `bRiver` at most its ordinary slots. The error is the reason the UI shows; a refused order
changes nothing.

## Which attacks are engagements

`Game::engage_applies(u, to)`: the option is on, the attacker is a land unit (one carried on a
transport counts: an amphibious attack) and `to` is land. The rule is applied at each entry:

- `group_attack_force(g, x, y, flags, formation)`, the one entry for every order of a group into an
  enemy plot (the human `move` and `engage` commands, AI and barbarian missions, the C++ AI
  benchmark); `group_attack(g, x, y, flags)` is it with no formation. Today's preamble (the path
  step for `MOVE_THROUGH_ENEMY`, the AI's pre-attack spells and bombard-first), then for an
  engagement: no candidate defender there means today's attack (it captures non-combatants, if
  any); Alertness for a human order without a formation; the AI's war declaration as today; the
  `doCombat` hook; then one engagement launched by the group, or for a group of missiles one
  missile strike per missile. `formation`: where the attacker's units stand (checked), `None` the
  auto formation; a call parameter, never saved. Each order into an enemy plot is one engagement; a
  follow-up is a new order. The AI attacks through it with no formation (the auto formation) and
  decides its attacks on the duel's odds as before (the AI is paused).
- `unit_attack(u, to)` (Taunt, the Shrine of Camulos, Gift of Kylorin): a one-unit engagement,
  forced (whatever the unit's moves and attacks this turn, as today's `attack`), in the auto
  formation, or a missile's strike; with only non-combatants there, today's attack.

## The engagement

`Game::engage` (`engage.rs`), in this order:

1. **Set-up.** Each side's slots and lanes. The attacker's pool: every unit of the player on the
   attacking plot that can attack the plot (`engage_attackers`: across its armies, garrison,
   heroes, mages and recon units); its formation: the order's (checked) or the auto formation. The
   defender's candidates: every unit on the plot that can defend, is an enemy (hidden nationality
   as in the duel) and is not invisible to the attacker (`engage_defenders`); its formation: the
   auto formation of them. An empty formation or no defender is a refusal, and nothing has
   changed. The committed units in the engagement's order (the attacker's by lane, then row; then
   the defender's), each a resolver `Fighter` (`Game::engage_fighter`), and the pair table of A and
   Dv (`engage_hit`, which nothing before the battle changes). Phase 7's pre-battle hook reads the
   set-up.
2. **Morale** (Q8, "Im afeared!"): a committed Fear defender makes each committed attacker not
   immune roll today's check; one that fails sits out (its attack spent, its cell empty: it never
   acts). Then a Fear attacker that did not sit out does the same to the committed defenders,
   which stay on the plot but do not fight.
3. **Opening.** Defensive strikes (Q5): for each committed attacker that did not sit out, every unit
   on the plot with a strike chance, at war and not used up, committed or in reserve, rolls; the
   best success strikes with today's damage, never past 95%, and is used up unless it has Blitz.
   It never kills. (The round model's collateral step is gone: splash, above.) Then phase 7's
   opening hook, on the set-up with its HP after the strikes.
4. **The battle**: the committed units that did not sit out, with their HP after the opening, on
   the formation resolver (`formation::resolve`) on the game's sync RNG.
5. **After the battle** (`apply.rs`, G2): damage written back (a Sentinel's to its city); battle
   XP before any per-kill effect (each kill's XP shared equally by the killing side's units that
   fought and live, in hundredths: docs/SIEGES.md, "XP"; withdrawals get the withdrawal XP); per kill in kill order the war weariness, war success and the
   barbarian counter, then `combatWon`'s per-kill part (capture, enslavement, gold, created units,
   the promotion from combat, the heal on a kill, the Python callbacks; a conversion rolled per kill
   and applied once after the killer's last kill); per engagement the promotions removed by combat,
   the combat-apply promotions and poison; the victims die (immortals are reborn); medics heal once
   a turn per unit (Q12), then phase 7's post-battle hook; withdrawn defenders flee (outside a
   city); every committed attacker has made its attack (one a turn, Blitz one more, counting
   engagements, bombards and missile strikes) and pays the attack's moves unless it moves in.
6. **The result** (spec 1.7): defenders stay in place until annihilated. **The plot falls only when
   no defender is left on it** (in the line or in reserve, or among those that sat out, a city's
   standing Sentinels included: docs/SIEGES.md): then the
   launching group moves in (a city is captured as today), and only it. Otherwise nobody moves: the
   attackers stay on their plot and every survivor keeps its damage. Withdrawn attackers stay with
   their army. The result (`report::EngageResult`): `Taken` (the plot fell), `Cleared` (the
   defender's line is gone, defenders are left on the plot), `Repulsed` (the attacker's line is
   gone), `Held` (both lines stand after the last round). There is no win, draw or mutual by
   damage, and no Fear rout (it needed a won battle). The armies of everyone involved are settled.
7. **The report** (`report.rs`): every engagement's summary (sides with their real and shown
   owners, slots, lanes, the slots used, committed, sat out, fallen by type, escaped, standing; the
   result, the rounds, whether the plot was taken and the city captured, the defenders left) is
   kept for the current and previous turn and the last `ENGAGEMENT_REPORTS_KEPT`; a battle a human
   could see also gets its full battle log (below). A game-log message to each side's players names
   the report.

It never sets the duel's combat state (`combat_unit`, the attack plot, combat timers), and nothing
outlives the call. Every loop is bounded (three rounds, passes by the most actions, finite lists),
so every engagement ends; a turn holds at most `1 + Blitz` engagements per unit in a pool, since
each spends an attack of every committed unit.

**Missiles** (Q11): a missile (the dragons' Meteor; the Fireball missile, which only the Wonder
spell summons since Fire II's Fireball became a battle spell) ordered into an enemy plot strikes instead:
`formation::missile_hit` on the defender's auto formation (a strike's reach and choice from outside
the field: no nearby first, no miss, no crit, no splash, no withdrawal; r drawn; never past the
missile's combat limit), then its collateral (today's formula); it dies, with the kill if it killed
(counted as an engagement's kill is). It strikes at the strength it had before formation battles
(spec 1.9: missiles unchanged): `ENGAGEMENT_MISSILE_PERCENT` (50) of a hit
(`Formation::missile_expected`: `ENGAGEMENT_BASE_DAMAGE` doubled to 40 for the battles, so half of
it is the round model's 20), the defended plot's own defence in its target's Dv
(`HitCtx::missile`), and none of the ground's rules (it does not fight on the field). Its target's
choice reads that hit.

**RNG**, in this order and nowhere else: the morale rolls ("Im afeared!", attackers then
defenders); per committed attacker that did not sit out its defensive strike's roll and damage
("Defensive Strike"); then the resolver's (`formation/mod.rs`'s module comment): per battle, each
fighter with chance first strikes in fighter order ("Engagement first strikes", `0..=chance`); per
pass each mover's coin in fighter order ("Engagement order"); per action a Wonder's outcome
("Engagement wonder"), a physical one's miss ("Engagement miss"), if it lands the crit roll of strike, charge, stab and shoot ("Engagement
crit"), r ("Engagement hit"), the target's withdrawal if the hit would kill it ("Engagement
withdraw"), then each splashed neighbour's r and withdrawal in (lane, row) order (not one at its
floor or fully protected from collateral damage); a heal's r
("Engagement heal"); an action with no target and a heal with no one hurt draw nothing; then the
post-battle steps' (combat-apply, poison, the per-kill effects' own rolls, the flight's plot).

**The resolver's API** (`engagement::formation`, pure: nothing of `Game`): `Formation::new(params,
ground, lanes, fighters, pair)` (each `Fighter`'s side, tag, lane, row, large, max HP and HP, its
speed, first strikes and chance first strikes, actions per row, A against nobody, accuracy, evasion, crit,
anti-flank, flanker, withdrawal chance, combat limit, collateral limit, extra and protection,
spell), `Formation::validate`
(inside the grid, one unit a cell), `Params::from_defines`, `Ground` and `Ground::rules`;
`resolve(&formation, rng)` / `resolve_quiet` give an `Outcome`: each fighter's `End` (HP, final row,
speed, the first strikes it rolled, its fate: standing, fell by whom in which round, withdrew in
which round; damage dealt, taken, healed), the kills in order, the damage matrix (`dealt(h, t)`,
battle XP's shares), the rounds begun, which sides were cleared, and the events (`Event`: round,
pass, actor, kind as fought, spell, target, hits `{target, dmg, expected, r, crit, miss, killed,
withdrew, splash}`, heal, knock-back, note). `auto_arrange(candidates, lanes, slots, ordinary)` is
the auto formation; `missile_hit(formation, h, rng)` a missile's hit (`missile_expected` its
expected damage).

## The battle log

Written through an optional sink (`sink.rs`) only for a battle a human could see (a human fights on
either side, or a human team sees the plot), `report::BattleLog`:
- the slots and lanes per side, the ground (city, forest or jungle, hills) and its rules that
  apply, and whether each side's formation was ordered;
- the fighters (`LogFighter`): the committed units first in the engagement's order (sat-out ones in
  their place), then the reserves on both plots (no cell): the unit, its name, unit type, owner and
  shown owner, side, cell `(lane, row)`, large, level, XP, hero, max HP, HP before the opening
  and when the battle began, its speed in the battle, first strikes, their chance and what it
  rolled, actions per row, the spell it casts, accuracy, evasion, crit, anti-flank, flanker, withdrawal
  chance, combat limit, collateral limit, defensive strike, immunities, Fear, A and Dv with their
  parts, its fate (standing, fell by whom in which round, withdrew in which round, sat out) and the
  damage it dealt, took and healed;
- the events in resolution order (`LogEvent`): the stages (pre-battle, morale, opening, battle,
  post), the morale rolls and the units that sat out, the defensive strikes, every action of the
  battle (`Action`: the resolver's `Event` with fighters as log indices and a spell as its
  `SpellType` index: `{round, pass, actor, kind, spell, target, hits: [{target, dmg, expected, r,
  crit, miss, killed, withdrew, splash}], heal, knocked: {unit, from, to}, note}`), the end (the
  rounds and which lines are gone), then the post-battle steps (XP with its sources, kill effects,
  heals, flight, the move in);
- the A and Dv of every pair that hit, with the pair-only terms;
- the plan the attacker saw, for a human attacker (`FrozenForecast`: the preview recomputed with the
  same seed: the chances the defender's line is wiped out, the plot falls, the attacker's line is
  wiped out, the units lost per side, each fighter's chance to fall and to escape).

The view (`ffh-play`'s `view/battles.rs`) shapes it as JSON for the battle screens (the contract is
`crates/ffh-play/assets/app/battle_log.js`'s header; docs/HUMAN.md, `battle_log`): `events` are what
the battle shows (the opening's units that sat out and defensive strikes at round 0, then the
actions), `after` the post-battle steps; the stages, the morale rolls that passed and the end are
left out (the outcome says how it ended). Writing it draws no random number and
changes no state: the same engagement with and without it leaves the same RNG and the same game
(test `the_sink_changes_nothing`). Saved: a report keeps its log for the current and previous turn
and the last `ENGAGEMENT_REPORT_LOGS_KEPT` logged reports, its summary for the last
`ENGAGEMENT_REPORTS_KEPT` (`Game::battle_reports_trim`).

## The attack preview

`preview.rs` (`Game::engage_preview`, pure): the plan of an attack before it is made: the
attacker's pool, its slots and lanes, the plan's formation (the one given, checked, or the auto
formation) and the auto formation, the pool's units not in it; the defenders the attacker's team
sees (invisible units are never candidates) and their formation (the rules' auto formation of them,
as the real fight forms them) and reserve; the ground and its rules; the war the attack would
declare; and the forecast.

- **The forecast** plays `ENGAGEMENT_PREVIEW_RUNS` (200) battles of the real fight's code on that
  set-up: the morale check and the defensive strikes as the same pure rolls the real fight makes,
  then the formation resolver, on a private RNG seeded from the plan (both formations, sorted, the
  plots, the turn). The same plan always shows the same numbers whatever the game's RNGs hold, and
  the game's RNG is never drawn (test `the_preview_is_a_function_of_its_inputs_and_changes_nothing`).
  A move that declares war previews the battle after the declaration (its strikers test war as that
  fight does). It gives each side's expected units fallen and escaped (x100), the chance the
  defender's line is wiped out, the chance the plot falls (the line wiped out with no defender left:
  none in reserve, none that sat out; withdrawn defenders flee), the chance the attacker's line is
  wiped out, and per committed fighter its chance to fall and to escape.
- The round model's What-if alternatives, the swap search and the moment estimator are gone.
- **Time** (release, this 4-core shared machine; `preview_timing`, `--ignored`, 2026-10-31): a plan
  of 200 battles takes 5.4 ms for 6 v 6, 6.9 ms for 8 v 8 and 8.6 ms for 10 v 10; the auto formation
  a few microseconds.

## Telemetry

`engage*` counts and the `engagement` event (docs/TELEMETRY.md): the result (`taken`, `cleared`,
`repulsed`, `held`), the rounds, the slots, the committed attackers and the fallen by kind, the
withdrawals, the morale failures, the defensive strikes, the missiles; the per-unit `attack` /
`defend` counts stay comparable with the duel's; each kill, a missile strike's too, counts as a
combat won (`kill_by`, the war's kills, a summon's kills). `telemetry.py`'s combat section prints
them.

## Balance lab

`ffh battles FILE [--runs N] [--seed S]` (`crates/ffh-cli/src/battles.rs`) fights staged
engagements through the rules' own entry: each line of FILE is a matchup (`name | ATTACKERS |
DEFENDERS | GROUND`, units as `[N*]UNIT[+PROMOTION]..[@row]`, ground words `open`, `forest`,
`hills`, `city`, `walls`, `fort:N`, `dmg:N`, `waves:N`, `bombard:N` (turns of siege before the
assault; an AI stack with siege units goes on bombarding until the Sentinels are breached, up to 10
turns); the module comment has the details), fought
`--runs` times from a save of the staged world with the RNG reseeded, both sides in the auto
formation, units without races, traits or weapons; one JSON line of totals per matchup (results,
units and HP lost, cost, damage by action kind, per unit type: fate, damage dealt, taken and
healed, actions, misses, crits, kills, rows; per run, the net share of HP and the net hammers of
HP destroyed, summed and squared for standard errors). About 5 to 7 ms a battle.
`crates/ffh-cli/scripts/battles_battery.py OUT` writes the first battery below as spec files and
`battles_report.py` makes its tables (`rows`, `matrix`, `hpmatrix`, `types`);
`battles_squads.py` writes the squad round robins per tech level (and the casters' labs:
`casters`, a core squad with two of each sphere's casters; `spells`, casters with and without each
charm, hex and heal) and `battles_meta.py` analyses them (`meta`: payoffs, the equilibrium and
dominance; `swaps`: unit against unit; `h2h`; `pairs`: with and without a spell, cell by cell;
docs/UNIT_ROLES.md, section 9).

**The balance target** (the owner's, 2026-11-06): every unit of every tier has a valuable place
in some lineup, and every formation has a formation that beats it (docs/UNIT_ROLES.md proposes the
levers and the units to pull them on). Units past a national class's
limit are a deliberate, inefficient sink for production (the over-limit price): squads are not
balanced for them, and the battery's squads hold at most the limit of each class.

**What the first battery showed** (2026-11-06, before the unit roles; about 1,100 matchups of 200 to 400 battles; each
line is one turn, so healing between turns and bombarding on the map are not in it; its late
armies of 5 to 8 of one national class are past the limit, so its late results are superseded by
the squad round robin below):

- **Numbers beat quality, and cheap units are the most cost-efficient in the game.** 4 Warriors
  (100 hammers) attacking one Phalanx (240) kill it in 99% of battles and lose 0.4 Warriors; 3
  Warriors kill a Champion in 99.5%. 19 Warriors (475) take a plot held by 6 fortified Axemen
  every time, losing 3.4; 2 Phalanxes (480) are repulsed in 92%. Defending against 12 Axemen, 19
  Warriors always hold and 2 Phalanxes always fall. Warriors stop being buildable once a city can
  train Archers and Axemen, but the step holds wherever both units are buildable: 8 Axemen against 4
  Champions (the same cost) trade 136 hammers of losses for 260, 8 Champions (never obsolete)
  against 4 Phalanxes 378 for 530 (the tier-4 extra action narrows that one). Why: a hit is at
  most 40 HP whatever the strength ratio (`40 x A / (A + Dv)`: a 4:1 ratio gives 32 against 8),
  every frontline unit has 120 HP at every tier, and a unit's actions do not grow with its
  strength (tier 4 adds one), so a unit four times as strong trades about four times as well
  while it costs ten times as much.
- **Mounted armies are the strongest pure army of every era in the open** (8 Horsemen and 8 Horse
  Archers top their eras' round robins); 68-74% of their damage is the charge (+100% in
  the first round, any target, the most threatening first), with speed 6 plus first strikes,
  evasion 15 and 25-35% withdrawal. A forest, a city or Formation takes most of it away (8 Horsemen
  on 8 Axemen: 1.5 lost to 5.1 in the open, 1.7 to 2.6 in a forest, 2.3 to 3.6 against Formation).
- **The auto formation puts the second charger behind the strikers**: strikers are placed first
  and fill the front row, so in a mixed army half the mounted units stand in the middle row,
  where they strike once and do not charge. Moving them to the front: 2 Knights with 4 Phalanxes
  and 2 Crossbowmen lose 3.4 units instead of 4.3; 3 Horse Archers with 5 Champions 2.0 instead of
  2.6.
- **Healers do not pay in battle at any tier.** Replacing 2 healers with 2 frontline units is
  better in every era; a High Priest of the Order (240) heals exactly as a Disciple (60) does, 20 HP
  a heal, and the two armies trade evenly. Healers prefer the middle row, where they heal once a
  round; in the rear they heal twice but are the backmost unit and are shot: no better. 63-70% of
  them fall in a battle.
- **A mage without a battle spell is a net loss; with one it is among the best picks.** The magic
  bolt (60%) makes the Adept the worst early addition and the Mage and Archmage among the worst
  later; Fire II or Air II (Fireball, Maelstrom) deal about four times as much and put the Mage
  with the Horse Archer and the Assassin. Mages stand in the rear, are the backmost unit and take
  the enemy's shots: 66-73% fall.
- **The rear row is the exposed one.** Shots, spells and bombards hit the backmost unit of each
  lane; the middle row is safe only with someone behind it. On open land 8 units stand 4 in front
  and 4 in the middle, so the middle row is the back row.
- **Crit units are the best additions of the mid and late eras**: Assassin, Shadow, Marksman (28%
  crit with Marksman). Crossbowmen and Marksmen are the best late support; Rangers (150) are safe in
  the middle (6% fall) but deal little for their cost.
- **Ground: hills outweigh walls.** 8 Axemen against 2 Axemen and 3 Archers take the plot in 98%
  of battles in the open or in an unwalled city, 25% in a forest, 0.5% in a walled city, 0% on
  hills, where they also lose more units than they kill (3.4 to 1.6). A city alone hardly matters
  (its defence counts only through Walls' Sentinels). In one turn, 16 Axemen (two waves) always
  take a walled, fortified city held by 5; held by 8 it takes 24; on hills with walls 24 Axemen
  (four waves, losing 9).
- **Siege engines do little in battle**: Catapults and Trebuchets are net losses or neutral as
  additions (they cannot finish a target, a third withdraw), and 8 Axemen, 4 Archers and 4
  Catapults lose more against a walled city than 16 Axemen (3.1 units to 2.5). Their bombard on
  the map is not in this battery; with it, they take walled cities (below).
- **Chariots** (120) are the worst early army: a Horseman's role at twice its cost, with less
  defence and no first strike.

**Squads by tech level** (2026-11-06, before the unit roles of docs/UNIT_ROLES.md, whose section
9 has the round robins after them; `battles_squads.py` and `battles_meta.py`). Each level's
generic units (each class's default unit): early (Axeman, Archer, Horseman, Hunter, Adept,
Disciple, Catapult, Chariot, War Elephant, at most 2 Skirmishers; Warrior and Scout as leftovers),
mid (Champion, Longbowman, Horse Archer, Mage with the bolt or Fire II, Priest, Assassin, Ranger,
Catapult, Archer, Chariot, Crusader, War Elephant), late (at most 4 of each of Phalanx, Immortal,
Knight, Paladin, Berserker, Beastmaster, Crossbowman, Marksman, Archmage with the bolt or Fire II,
High Priest; Arquebus, Cannon, Mage, Champion, War Elephant). Squads at an equal budget (480, 960,
1920) within 10 slots: every unit alone, front pairs, a front type with each support type, triples
(65-70 a level); a full round robin on open land (40 battles a matchup, both roles), then about 25
finalists a level on forest, hills and a walled, fortified city (60). The result is the net share
of HP, the mean of attacking and defending.

- **Each ground has a squad that nothing beats** (none beats it by 2 standard errors), at every
  level: early, 6 Horsemen and 2 Skirmishers in the open and on hills, 8 Horsemen in a forest, 6
  Axemen and 2 Skirmishers at walls; mid, 6 Horse Archers and 4 Archers in the open, 4 Horse
  Archers, 2 Assassins and 2 Fire II Mages on hills, 4 Champions and 4 Fire II Mages at walls (in
  a forest three Horse Archer squads form a cycle); late, 4 Knights and 5 Arquebuses in the open,
  4 Fire II Archmages and 4 Paladins in a forest, 4 Marksmen and 4 Paladins at walls (on hills
  three Knight squads nearly cycle). The open-land winners of mid and late beat every other squad
  both attacking and defending. Over the four grounds together, 8 Horsemen (early) and 4 Horse
  Archers, 2 Assassins and 2 Fire II Mages (mid) beat or tie every squad; only late is a mix (4
  Knights and 4 Marksmen, 4 Beastmasters and 4 Paladins, 4 Fire II Archmages and 4 Knights), and
  per ground it is not.
- **Mounted units have no counter but the ground.** Every winner off walls has a mounted front;
  Formation axemen still lose to horsemen. The charge picks the highest threat for its HP, so the
  enemy's archers and casters die first: in the winners the cheap ranged units (Archers,
  Arquebuses) are as much a sink for the enemy's charges as a source of damage (96% of the
  Arquebuses fall in the late mirror).
- **Units without a place**: replacing them with the level's main units improves a squad on every
  ground (comparisons of squads that differ only in that unit, against every opponent, both roles):
  the healers (Disciple, Priest, High Priest), casters without a battle spell (Adept, Mage,
  Archmage), the Chariot (the Horseman beats it in 99.8% at half the cost), the War Elephant (with the placement
  bug under "Open"); on open land, the Immortal and the Berserker (the Phalanx beats the Immortal
  in 99%: the same cost, cap and tier, 120/120 against 90/90). Marginal: the Longbowman (the
  Assassin or a Fire II Mage does better in its slot), the Hunter (about an Axeman), the Ranger
  (its best squads are 10th of 30 in a forest and on hills).
- **Siege engines' place is the siege, not the battle.** In the battle they add little (the
  Catapult loses to almost any substitute; at late, the Arquebus beats the Cannon in 80% of
  comparisons), and the walled-city ground above had no siege turns. With them (`bombard:3`, then
  the AI's own bombards until the Sentinels are breached), against a walled, fortified city: 4
  Axemen, 2 Skirmishers and 2 Catapults take one held by 5 in every assault after 6 turns, losing
  0.3 units (none of the squads without siege engines ever takes it); 6 Champions and 2 Catapults
  in 92% after 3 (0.5% without the turns); 4 Paladins, 2 Marksmen and 2 Cannons take one held by 8
  in 90% (0% without). The siege is not free: the besiegers wait 3 to 6 turns next to the city.
- **Strictly better units**: Marksman over Crossbowman (better in 91% of comparisons, worse in
  0.3%: the same cost, cap and tier; both can still fill a squad, being capped); Paladin over
  Phalanx (97%; good civilizations only); Crusader over Champion (100%; the Order only); Skirmisher
  over Archer (98%; by design, 2 a player); Fire II Mage over Mage, Fire II Archmage over Archmage.
- **Degenerate squads**: all of one type wins only early (8 Horsemen, the best squad of its level
  over the four grounds). 8 Horse Archers are 7th of 30 mid; all-archer, all-caster, all-healer,
  all-siege and all-large squads are at the bottom of every level. 10 Cannons attack no walled
  city (an AI owner bombards instead).

## Decided while building it

The spec (the owner's, revision 4) holds; these were left open and are decided so:

- Sat-out attackers are not struck by defensive strikes (they never close in); the Fear source on
  the attacking side must itself fight. A sat-out defender stays on the plot: the plot does not
  fall while it is there.
- A defensive strike that does no damage is still a strike: the striker is used up, as today.
- Battle XP is shared equally per kill (docs/SIEGES.md, "XP"; until 2026-11-04 it was shared by
  the damage dealt). Withdrawal XP takes the enemy side's least restrictive XP cap.
- `unit_attack` is forced: the unit given, whatever its moves and attacks this turn, if it is alive,
  can attack and fights land battles, in the auto formation.
- The fighters' order (the resolver's draw order for first strikes and coins) is the engagement's:
  the attacker's formation by lane then row, then the defender's; so the same formation is the same
  battle however the order lists it.
- A large unit's HP is in its stat sheet (150% of its old role's when the sheets were derived).
- The forest flag of the ground is a feature with `iEngagementSlotsChange` below 0 (the forests and
  the jungle); the city flag a city on the plot (a fort is not a city here).
- The auto formation's strength is the unit's A against nobody in particular on its side (its
  attack with its promotions), for both sides.
- Barrage's extra collateral damage and Drill's protection scale only a collateral splash, as in
  the duel: not the hit on the target, not a known spell's splash. The battle log does not list
  them as values (the fighter's promotions name them).
- A missile's hit is a strike's choice without nearby first (it comes from outside the field), with
  no withdrawal (as the round model's missile hit had none). Its target is the formation's choice,
  not the round model's random pick by role order. Its strength is the round model's: half a
  battle's hit, the plot's own defence in Dv, no ground rules.
- The plan of an AI attacker is not frozen into the log (the moment estimator that made it is gone).
- Per-kill game-log messages are replaced by one message per side and engagement, naming the report.
- A medic's heal goes only to a hurt unit, and only then uses its once a turn; the patient's own
  same-tile heal counts.
- A land plot with no unit that can engage but a unit that can still defend in the duel (a ship in
  a port) fights today's duel (see "Open").

## Open

- The AI decides its attacks on the duel's odds and attacks in the auto formation (the AI is paused,
  the owner's call). The defender always fights in the auto formation: standing formations per army
  are a later step (`Game::engage_defense_formation` is the hook).
- A city whose only defenders are ships in port is attacked with today's duel (ships never defend
  land in an engagement, but nothing else defends there).
- **Mages and summons** (the owner's decisions, 2026-11-05): mages fight in person, in the line,
  as they do (that they are harder to keep alive is accepted; easier training can make up for it
  if they turn out too weak); battle spells are done ("Battle spells" above). A caster's summon is
  an extra unit in its battles (not a swap for the caster); summons have no duration (a summon is
  in the battle it is called to, not on the map for some turns); a unit created in the middle of a
  battle (the Air Elemental's Lightning Elemental on a kill) fights on until the battle ends;
  Extension I and II, whose only effect is summons' moves, may go. Not built yet. The Summoner
  trait is a large bonus to its summons' strength, not more turns (done: Summoner's Bond, +50% HP,
  attack, defence, magic and magic defence; docs/PORTING.md, "Balance").
- Phase 7: spells in combat phases, summons in battle, the Guardsman's bodyguard, commander auras
  (the hooks are in place: plan section 14). The three spell hooks, `engage_pre_battle`,
  `engage_opening_spells` and `engage_post_battle_spells` (`engage.rs`), are called at their places
  and return nothing yet; the first two return `BattleEffect`s (a modifier scales the pair table, a
  damage lowers HP before the battle), the third `Cast` records.
