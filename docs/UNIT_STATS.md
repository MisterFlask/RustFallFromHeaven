# Unit stats

Status: built 2026-11-05; the owner chose the recommended answer of every decision in section 9.
The content pass (section 7) is still to come: every sheet holds its day-one, derived values.

## 1. The change in one paragraph

Before, every land unit had one strength, and the battle derived A and Dv from it: A was
`iCombat` x the role's A%, Dv `iCombatDefense` x the role's Dv%, under one summed percentage
modifier (docs/ENGAGEMENTS.md, "A and Dv per hit"). Promotions mostly changed that strength.

Now every land unit type has an explicit **stat sheet** instead:

- Attack and Defense;
- Magic and Magic Defense;
- Accuracy, Evasion and Crit;
- HP and Speed.

The sheet is written per unit in `units.jsonc`, so units of one role can differ. Every
promotion names the stats it changes. A physical action resolves Attack against Defense. A spell
resolves Magic against Magic Defense. Whether a hit lands, misses or crits comes from the hitter's
Accuracy and Crit and the target's Evasion.

On day one the numbers were derived from the old data, so the game plays almost as it did
(section 6). A content pass then differentiates the units (section 7).

**What already existed.** The data had half of this, but the content mostly ignored it:

- Units had separate `iCombat` and `iCombatDefense`, yet only about 50 of the 265 land combat
  units set them differently.
- Promotions had separate `iCombatPercent` / `iCombatPercentDefense` and `iExtraCombatStr` /
  `iExtraCombatDefense`, yet all 26 percentage promotions gave equal amounts to both.
- Per-unit HP, speed, dodge and crit (`iHP`, `iEngageSpeed`, `iEngageDodge`, `iEngageCritBonus`)
  could override the role's, but only the Sentinel set any of them.

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
| Accuracy | ACC | the hitter's hit chance before the target's Evasion | 90 (the old 10% base miss) |
| Evasion | EVA | subtracted from a physical hitter's chance to hit it | the role's dodge (Flanker 10, Assassin 15, else 0) |
| Crit | CRIT | its chance to crit on STRIKE, CHARGE, STAB, SHOOT | 10 + the role's crit bonus (Assassin 25) |
| Speed | SPD | turn order in a pass; first strikes add to it as now | the role's speed |

**Roles stay.** They still define rows and actions, the home row, the auto-pick order and
anti-flank. Their A%, Dv%, HP, speed, dodge and crit were the values the derivation read once; they
are gone from the roles, and the unit's sheet is the only source (307 land unit types, Sentinel
included; ships and air units have none). A land type without a sheet warns at load.

**Large units** keep two cells. Their 150% HP is written into their sheet.

**Data format.** Each land unit has a `stats` object in `units.jsonc` (docs/DATA.md); every key is
required, and a value out of its range warns and reads as the range's lowest (HP 1..; evasion and
crit 0..=95; accuracy 0..=200; the rest 0 and above):

```jsonc
"stats": {"hp": 120, "attack": 40, "defense": 40, "magic": 0, "magic_defense": 40,
          "accuracy": 90, "evasion": 0, "crit": 10, "speed": 4},
```

## 3. Hits

**Physical** (STRIKE, CHARGE, STAB, SHOOT, BOMBARD):

- Hit chance = hitter ACC − target EVA − the action's penalty (SHOOT 10, BOMBARD 15), kept
  within 5..100. The old miss chance was 10 + penalty + dodge, so day one is unchanged.
- Crit chance = hitter CRIT (at most 95). A crit deals 150% damage and knocks the target back, as
  before. BOMBARD and splash never crit.
- Expected damage = `40 x ATK / (ATK + DEF)` HP (decision 1). After that, the rest is the old
  rules: ground percentages, the charge bonus, splash, the 80–120% roll, the siege floor and
  withdrawal.

**Magic** (SPELL):

- Expected damage = `40 x MAG / (MAG + MDEF)` x the spell's power x
  `(100 − the target's resistance to the spell's damage type)%`.
- A battle spell used to ignore the target's elemental resistance. Now a Fire Elemental shrugs off
  Ring of Flames.
- Spells still never miss and never crit. Evasion doesn't apply to them.
- The magic bolt (known by casters with no damage spell) is DAMAGE_PHYSICAL at 60% power, which
  only Magic Defense resists.

**Typed weapon damage** works as before. FFH's damage-type strength (Flaming Arrows' fire, a Fire
Elemental's own fire, the Nether Blade's death) adds to ATK, and each type's points are reduced by
the target's resistance to that type.

**HEAL** stays `40 x 50% x roll` (decision 8: scaling it with MAG is left for the content pass).

## 4. Modifiers and promotions

**Each modifier names one stat.** Stat changes come in two kinds, and a promotion may carry both:

- **Flat:** points added to the stat.
- **Percent:** summed with the stat's other percentages, then applied once. A positive sum applies
  as `x (100 + m) / 100`, a negative one as `x 100 / (100 − m)`, as the old strength.

```jsonc
"stat_changes": {"attack": {"percent": 20}, "defense": {"percent": 20}, "magic": {"percent": 5}},
"stat_changes": {"attack": {"flat": 10}},  // Bronze Weapons
```

**Conversion of the old promotion fields** (a one-off script rewrote `promotions.jsonc`; the
old keys are gone from the data and the loader; `PromotionInfo::combat_percent()` and the like give
the old values back for the code that wants them):

| Before | Promotions | Became |
|---|---|---|
| `iCombatPercent` | 26 (Combat I–V +20, Empower, Burning Blood, Diseased −30, ...) | ATK % |
| `iCombatPercentDefense` | 26 (the same list) | DEF % and MDEF % |
| `iExtraCombatStr` | 17 (Bronze Weapons +1, Godslayer +3, ...) | ATK flat x10 |
| `iExtraCombatDefense` | 16 (Bronze/Iron/Mithril Weapons, Heroic Defense, ...) | DEF flat x10 |
| `iSpellDamageModify` | 10 (Combat I–V +5, Metamagic) | MAG % |
| `iFirstStrikesChange`, `iChanceFirstStrikesChange` | 8 (Drill, Dance of Blades, Stoneskin, ...) | stay: SPD in battle, as before |
| Formation (the Screen role) | the Screen's Dv 110% and speed 3 | DEF % and MDEF % +10, SPD -1 |

**Situational modifiers keep their own fields.** Where each one counts:

- **Place bonuses on both ATK and DEF** (the owner's rule, 2026-11-05): the attacker's city attack,
  hills, terrain and feature attack against the plot it attacks, and the defender's fortification,
  city defence, hills, terrain and feature defence on the plot it holds (City Raider, City
  Garrison, Guerilla, Woodsman, Defensive). In a formation battle both sides hit, so a bonus for
  where the unit fights counts on the blows it deals and the ones it takes; the duel had put each
  on one side. The amphibious penalty stays on the attacker's ATK.
- **On ATK** when the unit hits: the attack half of the unit-combat and class modifiers.
- **On DEF** when the unit is hit: the defence half of the unit-combat modifiers (Formation's "vs
  mounted").
- Homeland, Stigmata and an army commander's aura go on both ATK and DEF.

All of these are physical only. MAG and MDEF see only stat changes and elemental resistances.

**The unit's own changes** (`Unit::stat_flat`, `Unit::stat_pct`, by stat): a promotion's
`stat_changes` (`Game::unit_apply_stat_changes`) and every change of the unit's strength. The
strength outside land battles stays the C++ fields (`base_combat`, `base_combat_defense`,
`extra_combat_percent`, `spell_damage_modify`: decision 6); the functions that change them
(`unit_change_base_combat_str`, `_defense`, `unit_change_extra_combat_percent`,
`unit_change_spell_damage_modify`, and the setters the Python effects use) change the stat arrays
too, ten points a strength point. So the post-combat strength effects (Ira, the Wolf Rider and
others), adding to a Flesh Golem and affinities (`BonusAffinities`) reach the battle. A promotion's
attack and defence points change the strength outside battles in tens; its attack percentage, the
strength's percentage; its magic percentage, the spell damage of spells cast outside battles.

**`iResistMagic` stays.** It is the chance to resist a hostile spell outside battle, such as a
charm or a hold, and has nothing to do with damage.

**New promotion content.** No promotion gives Accuracy, Evasion, Crit or Magic Defense yet (but
Formation's magic defence). These are suggestions for the owner, not part of the conversion:

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
- The per-unit overrides `attack`, `defense`, `hp`, `engage_speed`, `engage_dodge` and
  `engage_crit_bonus` (the sheet replaces them), the roles' `attack_percent`, `defense_percent`,
  `max_hp`, `speed`, `dodge` and `crit_bonus`, and `ENGAGEMENT_MISS_PERCENT`, `_CRIT_PERCENT` and
  `_LARGE_HP_PERCENT` (baked into the sheets).
- `Unit::extra_combat_percent_defense` (its defence percentage is `stat_pct`'s).

## 6. Day one: how close to before it plays

The derivation reproduces the old battle numbers (checked by the battle stat tests, which kept
their old expected values but where a change below applies), except for these deliberate changes,
recorded in docs/PORTING.md "Changes from FFH2":

1. A battle spell is the caster's magic against the target's magic defence. They take only the
   units' own percentages, not the situational terms the old A and Dv had (fortification, city,
   hills, terrain and feature defence, city attack, unit-combat bonuses, Homeland, Stigmata), and
   the target's resistance to the spell's damage type reduces it (section 3).
2. Flat strength gives 10 points of attack or defence a strength point whatever the role (it was
   scaled by the role's percentage: an Archer's Bronze Weapons gave +0.6 Dv, now +10 DEF); typed
   strength too.
3. A promotion's spell damage percentage raises magic in a battle (it only touched spells cast
   outside battles), and its attack percentage no longer raises a caster's spells: Combat I gives
   +5% magic where it gave +20% to the spell's A. Promoted casters cast weaker spells than before;
   the content pass can give the Combat line more magic.
4. Formation gives +10% defence and magic defence and -1 speed (the Screen role's Dv 110% and
   speed 3), its +10% summed with the other percentages instead of multiplying the base: a
   Formation axeman against a knight (+40% vs mounted) has Dv 60 where it had 61.6.

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

## 8. What was built

1. **Data** (`crates/ffh-data/src/infos/engagement.rs`): `StatType`, `StatSheet`, `StatChange`,
   `read_stats`, `read_stat_changes`; `UnitInfo::stats`, `PromotionInfo::stat_changes`;
   `GameData::unit_stat`. A one-off conversion wrote the 307 sheets and 50 promotions' changes.
2. **Rules** (`crates/ffh-core/src/engagement/stats.rs`): `Game::unit_stat` and the per-unit
   accessors; A and Dv from the sheet; magic and magic defence (`engage_magic_x100`,
   `engage_spell_pct`). The resolver's `Pair` carries `ma`, `md`, `spell_pct`; each `Fighter` its
   accuracy, evasion and crit; `miss_chance(kind, h, t)`.
3. **Everything else**: the Sentinels' values from `UNIT_SENTINEL`'s sheet, its defence, magic
   defence and HP grown by the city's (`siege.rs`); the battle log's fighters (accuracy, evasion,
   crit, magic and magic defence with their parts) and pairs (the magic pair); the Python effects
   through the strength setters. The naval duel and the rules reading strength are unchanged.
4. **Client**: the unit card and tooltips (ATK, DEF, MAG, MDEF, ACC, EVA, CRIT, speed), the plan's
   fighters, the battle log's notes, the promotion help (a line a stat change) and the unit help (its
   sheet).
5. **AI** (`crates/ffh-ai/src/units.rs`, `stat_changes_value`): a promotion's stat changes valued as
   percentages of the unit's own stats, by what it does with them (magic for casters, defence for
   defenders, magic defence more on the front line, accuracy, evasion, crit and speed by points).
   Battle odds come from the forecast, so they follow.
6. **Save format 15.**

## 9. Decisions (the owner chose each recommended answer, 2026-11-05)

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
