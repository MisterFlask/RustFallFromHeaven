# Unit stats (proposal, not built)

Status: proposal for the owner, 2026-11-05. Nothing here is built yet. Section 9 lists the decisions
that are the owner's; each one gives a recommended answer.

## 1. The change in one paragraph

Today every land unit has one strength, and the battle derives A and Dv from it. A is
`iCombat` x the role's A%, Dv is `iCombatDefense` x the role's Dv%, and one summed percentage
modifier is applied to the result (docs/ENGAGEMENTS.md, "A and Dv per hit"). Promotions mostly
change that strength.

This proposal gives every land unit type an explicit **stat sheet** instead:

- Attack and Defense;
- Magic and Magic Defense;
- Accuracy, Evasion and Crit;
- HP and Speed.

The sheet is written per unit in `units.jsonc`, so units of one role can differ. Every
promotion names the stats it changes. A physical action resolves Attack against Defense. A spell
resolves Magic against Magic Defense. Whether a hit lands, misses or crits comes from the hitter's
Accuracy and Crit and the target's Evasion.

On day one the numbers are derived from today's data, so the game plays almost as it does now
(section 6). A content pass then differentiates the units (section 7).

**What already exists.** The data has half of this already, but the content mostly ignores it:

- Units have separate `iCombat` and `iCombatDefense`, yet only about 50 of the 265 land combat
  units set them differently.
- Promotions have separate `iCombatPercent` / `iCombatPercentDefense` and `iExtraCombatStr` /
  `iExtraCombatDefense`, yet all 26 percentage promotions give equal amounts to both.
- Per-unit HP, speed, dodge and crit (`iHP`, `iEngageSpeed`, `iEngageDodge`, `iEngageCritBonus`)
  can already override the role's, but only the Sentinel sets any of them.

**What is new:**

- magic as its own pair of stats;
- accuracy;
- an explicit, visible sheet for every unit, with the role's hidden A% and Dv% multipliers baked
  into it;
- promotions able to target any stat.

## 2. The stat sheet

All values are integers on a x10 scale: an Axeman (strength 4) has Attack 40.

| Stat | Abbr. | Does | Day-one value (derived) |
|---|---|---|---|
| HP | HP | hit points in battle; the saved `damage` stays a % of it | `iHP`, else the role's (120 / 100 / 80 / 70 / 60), large units 150% |
| Attack | ATK | the hitter's side of STRIKE, CHARGE, STAB, SHOOT, BOMBARD | `iCombat` x 10 x the role's A% |
| Defense | DEF | the target's side of those actions | `iCombatDefense` x 10 x the role's Dv% |
| Magic | MAG | the caster's side of SPELL; 0 means it casts nothing | casters (a row with SPELL): `iCombat` x 10 x A%; others 0 |
| Magic Defense | MDEF | the target's side of SPELL | equal to DEF |
| Accuracy | ACC | the hitter's hit chance before the target's Evasion | 90 (today's 10% base miss) |
| Evasion | EVA | subtracted from a physical hitter's chance to hit it | the role's dodge (Flanker 10, Assassin 15, else 0) |
| Crit | CRIT | its chance to crit on STRIKE, CHARGE, STAB, SHOOT | 10 + the role's crit bonus (Assassin 25) |
| Speed | SPD | turn order in a pass; first strikes add to it as now | the role's speed |

**Roles stay.** They still define rows and actions, the home row, the auto-pick order and
anti-flank. Their A%, Dv%, HP, dodge and crit become the defaults that the derivation script reads
once. After that, the unit's sheet is the only source.

**Large units** keep two cells. Their 150% HP is written into their sheet.

**Data format.** Each land unit gets a `stats` object in `units.jsonc` (the data is JSON with
comments since November 2026: docs/DATA.md), written by the conversion script on day one:

```jsonc
"stats": {"hp": 120, "attack": 40, "defense": 40, "magic": 0, "magic_defense": 40,
          "accuracy": 90, "evasion": 0, "crit": 10, "speed": 4},
```

## 3. Hits

**Physical** (STRIKE, CHARGE, STAB, SHOOT, BOMBARD):

- Hit chance = hitter ACC − target EVA − the action's penalty (SHOOT 10, BOMBARD 15), kept
  within 5..100. Today's equivalent is 100 − (10 + penalty + dodge), so day one is unchanged.
- Crit chance = hitter CRIT (at most 95). A crit deals 150% damage and knocks the target back, as
  now. BOMBARD and splash never crit.
- Expected damage = `40 x ATK / (ATK + DEF)` HP (decision 1). After that, the rest is today's
  rules: ground percentages, the charge bonus, splash, the 80–120% roll, the siege floor and
  withdrawal.

**Magic** (SPELL):

- Expected damage = `40 x MAG / (MAG + MDEF)` x the spell's power x
  `(100 − the target's resistance to the spell's damage type)%`.
- Today a battle spell ignores the target's elemental resistance. Here a Fire Elemental shrugs off
  Ring of Flames.
- Spells still never miss and never crit. Evasion doesn't apply to them.
- The magic bolt (known by casters with no damage spell) is DAMAGE_PHYSICAL at 60% power, which
  only Magic Defense resists.

**Typed weapon damage** works as today. FFH's damage-type strength (Flaming Arrows' fire, a Fire
Elemental's own fire, the Nether Blade's death) adds to ATK, and each type's points are reduced by
the target's resistance to that type.

**HEAL** stays `40 x 50% x roll` (decision 8 asks whether it should scale with MAG).

## 4. Modifiers and promotions

**Each modifier names one stat.** Stat changes come in two kinds, and a promotion may carry both:

- **Flat:** points added to the stat.
- **Percent:** summed with the stat's other percentages, then applied once. A positive sum applies
  as `x (100 + m) / 100`, a negative one as `x 100 / (100 − m)`, as today's strength.

```jsonc
"stat_changes": {"attack": {"percent": 20}, "defense": {"percent": 20}, "magic": {"percent": 5}},
"stat_changes": {"attack": {"flat": 10}},  // Bronze Weapons
```

**Conversion of today's promotion fields** (a one-off script rewrites `promotions.jsonc`;
the old keys are then deleted from the data and the loader):

| Today | Promotions | Becomes |
|---|---|---|
| `iCombatPercent` | 26 (Combat I–V +20, Empower, Burning Blood, Diseased −30, ...) | ATK % |
| `iCombatPercentDefense` | 26 (the same list) | DEF % and MDEF % |
| `iExtraCombatStr` | 17 (Bronze Weapons +1, Godslayer +3, ...) | ATK flat x10 |
| `iExtraCombatDefense` | 16 (Bronze/Iron/Mithril Weapons, Heroic Defense, ...) | DEF flat x10 |
| `iSpellDamageModify` | 10 (Combat I–V +5, Metamagic) | MAG % |
| `iFirstStrikesChange`, `iChanceFirstStrikesChange` | 8 (Drill, Dance of Blades, Stoneskin, ...) | stay: SPD in battle, as now |

**Situational modifiers keep their own fields.** These already belong to one side, so they now say
so explicitly:

- **On ATK** when the unit hits: city attack, hills, terrain and feature attack, and the attack
  half of the unit-combat and class modifiers.
- **On DEF** when the unit is hit: fortification, city defence, hills, terrain and feature defence
  (City Garrison, Guerilla, Woodsman, Formation's "vs mounted"), and the defence half of the
  unit-combat modifiers.
- Homeland, Stigmata and an army commander's aura go on both ATK and DEF.

All of these are physical only. MAG and MDEF see only stat changes and elemental resistances.

**The unit's own changes.** Python effects that change a unit's base strength map to flat ATK and
DEF changes on the unit. These include the post-combat strength effects (Ira, the Wolf Rider and
others), adding to a Flesh Golem, and affinities (`BonusAffinities`).

**`iResistMagic` stays.** It is the chance to resist a hostile spell outside battle, such as a
charm or a hold, and has nothing to do with damage. The `Unit` fields `base_combat` and `base_combat_defense` become a
per-stat flat change array, and the save format is bumped.

**New promotion content.** Today no promotion gives Accuracy, Evasion, Crit or Magic Defense. These
are suggestions for the owner, not part of the conversion:

- Drill I–IV: +ACC.
- Mobility or Flanking I–III: +EVA, keeping their withdrawal.
- Marksman: +CRIT.
- the existing Magic Resistance promotion (20% resistance to hostile spells): also +MDEF %.
- Spell Extension or Metamagic: +MAG.

## 5. What is removed

- The roles' A% and Dv% as live multipliers (baked into each unit's sheet).
- The single land-battle "strength", and its one modifier pool covering everything.
- `iCombatPercent`, `iCombatPercentDefense`, `iExtraCombatStr`, `iExtraCombatDefense` and
  `iSpellDamageModify` in promotions (replaced by `stat_changes`).
- The per-unit override tags `iAttack`, `iDefense`, `iEngageDodge` and `iEngageCritBonus` (the
  sheet replaces them).

## 6. Day one: how close to today it plays

The derivation reproduces today's battle numbers, except for three deliberate changes, recorded in
docs/PORTING.md "Changes from FFH2":

1. Battle spells are reduced by the target's elemental resistance (section 3).
2. Flat strength promotions give a flat +10 ATK or DEF regardless of role. Today they are scaled
   by the role's percentage, so an Archer's Bronze Weapons gives +0.6 Dv today and +10 DEF here.
3. Spell power modifiers (`iSpellDamageModify`, which today affect only spells cast outside
   battle) also raise MAG in battle.

## 7. The content pass (after day one)

This is where the change starts paying off: units of a role stop being clones, and magic becomes a
counter to armour. Here is a sketch to react to. The numbers are the owner's call, and none are in
the build above.

- **Armour against magic.** Heavy infantry (Frontline and Screen) get high DEF and low MDEF.
  Arcane and Divine units get low DEF and high MDEF. Mages then crack shield walls, and archers and
  cavalry kill mages.
- **Flankers:** EVA 15–20 and moderate DEF. The cavalry dodges arrows but falls to a held line.
- **Assassins:** CRIT 25–30 and EVA 20. They are glass.
- **Archers:** ACC 95. Elven archers get +5 ACC and +5 EVA.
- **Races:**
  - Dwarves: +DEF, −EVA.
  - Elves: +EVA, +ACC.
  - Undead: low MDEF, but high resistance to death and poison as today.
  - Golems and constructs: EVA 0 and high MDEF; the Nullstone Golem is near immune to magic.
  - Angels and demons: high MDEF.
- **Heroes** get their own sheets, distinct from their unit type.

I can draft this pass as one table per unit combat for review, as a separate step after the build.

## 8. Build plan

1. **Data.** A `StatType` enum, the `stats` sheet on units and `stat_changes` on promotions, with
   loader validation. A conversion script writes every land unit's sheet and every promotion's
   changes, then the old fields go.
2. **Rules.** In stats.rs, one term pool per stat. The resolver's pair table carries ATK/DEF and
   MAG/MDEF. Hit, crit and evasion come per unit. Spells apply resistance.
3. **Everything else that read strength.** The naval duel, the Python strength effects, the
   Sentinels (they get a sheet; city defence scales their DEF, MDEF and HP), the preview and the
   forecast.
4. **Client.** The unit card shows the sheet; the map chip shows ATK/DEF; promotion help lists stat
   changes; the battle log and the attack preview break down whichever pair a hit used.
5. **AI.** Promotion values by stat and role: MAG for casters, EVA and CRIT for assassins, DEF and
   MDEF for frontline units. Battle odds already come from the forecast, so they follow
   automatically.
6. **Docs and tests.** ENGAGEMENTS.md, SIEGES.md, PORTING.md, save format bump. A test that the
   derived sheet reproduces today's pair table for every land unit, kept only for the conversion
   commit and then deleted (CLAUDE.md: no machinery whose only purpose is FFH equivalence).

## 9. Decisions for the owner

1. **Damage formula.** Recommended: **keep the ratio `40 x ATK / (ATK + DEF)`**. It behaves across
   FFH's wide tier range (strength 2 to 33): a weak unit still chips a strong one, and today's
   balance carries over.
   - Alternative, **subtractive** `max(floor, ATK − DEF)`: armour feels absolute and the numbers
     read plainly. But at FFH's tier spread a tier-1 unit deals only the floor to a tier-3 one, and
     every flat +DEF promotion becomes huge.
   - Alternative, **reduction** `ATK x 100 / (100 + DEF)`: Defense reads as "% of damage taken".
     But damage then grows linearly with Attack, so HP must grow with tier too.
2. **Magic.** Recommended: **a separate MAG stat**. An Adept is weak in melee but casts hard, and
   Metamagic and Combat's spell bonus have something to raise. The alternative is that spells use
   ATK, which gives fewer stats but leaves no spell power to train.
3. **Scale.** Recommended: **x10 integers** (ATK 40), so small promotions have room. The
   alternative is today's decimals (4.0).
4. **Promotions.** Recommended: **flat and percent both**, as today's data already does.
   Percentages keep training useful on strong units, flat changes suit weapons. The alternative is
   flat only: simpler cards, but Combat V on a Warrior then outweighs it on a Champion.
5. **Growth on level-up.** Recommended: **none**; promotions are the growth. The alternative is a
   small automatic gain per level by unit combat, the RPG way.
6. **Outside battles.** Recommended: **`iCombat` stays the unit's rating** for the naval duel, the
   FFH Python rules that read strength (spell chances and requirements, post-combat effects, Flesh
   Golem merging, event values) and the AI's power estimates. The alternative is to derive the rating from ATK and DEF
   and drop `iCombat`, which means more churn now for one fewer number.
7. **Situational modifiers.** Recommended: **keep them as ATK or DEF percentages, physical only**
   (section 4).
8. **Heals.** Recommended: **unchanged for now**, with heals scaling with MAG left for the content
   pass.
9. **The content pass.** Recommended: **I draft it as tables after the build** (section 7), and the
   owner edits.
