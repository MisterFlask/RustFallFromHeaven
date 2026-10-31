# Engagement combat: inventory of FfH combat effects

The owner's design spec "FfH Culture Actions & Army Combat" (2026-10-29) replaces Civ4's one-on-one
combat with engagements: each side commits up to S units, units have roles (frontline, screen,
backline, flanker, assassin, support) with attack A, defense Dv and max HP, and a battle runs
pre-battle spells, opening volleys, rounds, withdrawal and post-battle effects. The spec says:
"Before implementing, inventory every FfH promotion and ability with a combat effect and propose
keep, remap or remove for each, for review." This is that inventory. Nothing here is implemented yet;
the owner answered the questions on 2026-10-31 ("The owner's answers", at the end).

**Sources.** Promotions: `data/xml/Units/CIV4PromotionInfos.xml` (261 promotions). Unit abilities:
`data/xml/Units/CIV4UnitInfos.xml` (325 unit types). Spells: `data/xml/Units/CIV4SpellInfos.xml` (267).
What the rules read: `crates/ffh-core/src/unit/combat.rs` (`unit_resolve_combat`,
`unit_update_combat`, `unit_max_combat_str`, `unit_do_defensive_strike`, `unit_collateral_combat`,
`unit_flanking_strike_combat`, `unit_combat_won`), `unit/promotions.rs` (`unit_set_has_promotion`
and the strength getters), `unit/ffh.rs`, `plot/units.rs` (`plot_get_best_defender`), the ported
Python in `crates/ffh-core/src/ffh/` (post-combat callbacks, `ev_unit_killed`, damage spells), checked
against `CvUnit.cpp` (`resolveCombat`, `updateCombat`, `maxCombatStr`, `combatWon`). Values below
are the XML's. Defines quoted are the loaded ones (GlobalDefinesAlt.xml overrides GlobalDefines.xml).

**Result.** 179 of the 261 promotions have a combat effect and appear below by type name,
grouped by mechanic (a promotion with several effects appears in each of its sections); the
other 82 are listed in the appendix. Every unit-type field the combat code reads is covered,
by mechanic, with the notable units named.

**How to read the proposals.** KEEP: works unchanged in the engagement model (the same field,
the same number, read at the obvious point). REMAP: the field keeps its value but means something
new, said exactly. REMOVE: the field stops mattering in combat, with the reason. Where the spec's
remap table or unit-stats rules decide the case, the proposal follows them and says so; where the
spec is silent the proposal is the mapping I would pick, and the open decisions are numbered
**Q1..Q13** and collected in "Questions for the owner" at the end.

## Summary

The promotion counts overlap: a promotion with several effects counts in each of its rows
(179 distinct). Unit counts are unit types (of 325) that set the field, or that get the
promotion free where the row says so.

| # | Mechanic | Promotions | Unit types with the ability | Proposal |
|---|---|---|---|---|
| 1 | Strength percentage (`iCombatPercent`, `iCombatPercentDefense`) | 26 | none | REMAP to A and Dv (spec); honour the unused defense field (Q1) |
| 2 | Flat strength and affinities | 24 | 20 (affinities) | KEEP: base A / base Dv |
| 3 | Damage types and resistances | 41 | 88 typed | REMAP typed strength (Q2); KEEP resistances |
| 4 | First strikes and immunity | 11 | 36 strike, 1 chance, 17 immune | REMAP to opening volleys (spec); immunity = not a volley target |
| 5 | Defensive strike | 8 | 20 strike, 12 immune | REMAP to opening strikes from the defending plot (Q5) |
| 6 | Withdrawal | 8 | 43 attack, 2 defensive | REMAP (spec); one roll per engagement, defenders too (Q3) |
| 7 | Collateral damage | 5 | 14 | KEEP formula; once per engagement, by field not role (Q4) |
| 8 | Flanking strikes | 0 | 0 | REMOVE (spec; no FfH unit has one) |
| 9 | Blitz | 4 | 4 (free promotion) | REMAP: one extra attack per turn (spec) |
| 10 | City attack, city defense, bombard, building defense | 9 | 50 attack, 26 defense, 11 bombard, 6 ignore | KEEP (spec): A, Dv, slots |
| 11 | Terrain, hills and feature modifiers | 13 | 17 | KEEP: attack mods to A, defense mods to Dv (spec) |
| 12 | Fortification | 2 | 1 (free promotion) | KEEP: Dv only (spec) |
| 13 | River crossing and amphibious attack | 4 | 0 | REMAP: river-capable units fill the lost slots (Q9) |
| 14 | Bonuses against unit combats, classes and animals | 13 | 25 combat, 18 animal, 1 class | KEEP (spec): A when hitting, Dv when hit |
| 15 | Bonuses against promotions (slaying, weapons) | 9 | 0 | KEEP: as section 14 |
| 16 | Situational auras (in borders, Armageddon counter) | 2 | 0 | KEEP: A and Dv |
| 17 | Defender choice and targeting | 6 | 0 | REMAP Marksman and Guardsman to targeting (Q6); REMOVE the rest |
| 18 | Fear | 12 | 10 (free promotion) | REMAP: pre-battle morale check and rout (Q8) |
| 19 | Combat limit | 1 | 2 | REMAP: per-target damage cap |
| 20 | Heal on a kill | 3 | 2 (Python) | REMAP: post-battle, per kill |
| 21 | Medics and heal rates | 12 | 32 (free Medic I) | KEEP turn healing; ADD a post-battle medic heal (spec; Q12) |
| 22 | Capture and enslavement | 10 | 2 enslave, 2 no-capture, 23 equipment | REMAP: per kill, post-battle |
| 23 | Rewards per victory (XP, gold, counter, duration) | 5 | 2 gold, 2 duration | REMAP: per kill |
| 24 | Afflictions spread by combat (plague, disease, wither, poison) | 5 | 23 poison | REMAP: once per engagement |
| 25 | Removed by combat | 4 | 0 | REMAP: removed after any engagement fought |
| 26 | Cannot attack (`bOnlyDefensive`) | 1 | 14 | KEEP |
| 27 | Immortality | 1 | 7 (+1 by combat) | KEEP |
| 28 | Stealth and sight | 5 | 3 invisible types | KEEP |
| 29 | Not alive | 8 | via race promotions | KEEP |
| 30 | Spell resistance, spell damage, twincast | 23 | via promotions | KEEP (applies to combat-phase spells) |
| 31 | Summon perks and random grants | 12 | via promotions | KEEP |
| U1 | Separate defense strength (`iCombatDefense`) | - | 281 (52 differ from attack) | KEEP: base of Dv (spec) |
| U2 | No defensive bonus (`bNoDefensiveBonus`) | - | 56 | KEEP |
| U3 | Missiles and exploders (`bSuicide`, `bExplodeInCombat`) | - | 2 suicide, 4 explode | REMAP Fireball and Meteor to opening strikes (Q11) |
| U4 | Units created or converted by combat | - | 8 create, 5 convert, 1 promotion | REMAP: per kill, post-battle |
| U5 | Python combat callbacks and death hooks | - | 26 units, 27 callbacks | KEEP: per kill pair, post-battle |
| U6 | Civ4 fields FfH never sets | - | 0 | REMOVE from the combat path |
| U7 | Rule-level modifiers (handicap, barbarians) | - | - | KEEP as A/Dv %; REMOVE free wins (all 0) |
| S | Spells with an in-battle phase (`iCombatPhase`) | - | 60 of 267 spells | 14 opening, 40 pre-battle, 6 post-battle |

## How combat works today, in one paragraph

An attack is a duel between the attacker and the defended plot's "best defender"
(`plot_get_best_defender`). Each side's strength is its base strength (plus FfH typed strength)
times one summed percentage modifier (`unit_max_combat_str`): the unit's own +%, terrain,
fortification, city defense, attack bonuses of the attacker turned into minuses on the defender,
bonuses against unit combats and promotions, typed strength reduced by the opponent's
resistances. Before the duel FfH runs defensive strikes, fear and the attacker's collateral damage.
The duel is rounds of `COMBAT_DAMAGE` 20 (scaled by firepower) at odds strength / (sum); first
strikes are rounds in which the striker cannot be hurt; withdrawal saves an attacker about to die;
FfH adds defensive withdrawal, a combat limit that ends the duel early, and after the duel
`unit_combat_won`: XP, heal-on-kill, capture, enslavement, spread of afflictions, created and
converted units, gold, and the units' Python callbacks.

## Ground rules this inventory assumes

These follow from the spec and are used throughout instead of being repeated per item.

- **G1. Modifiers are evaluated per hit.** A hit has a hitter and a target. The hitter's
  attack-side modifiers (its own strength %, city attack, terrain/feature attack against the
  target's plot, bonuses against the target's unit combat, class or promotions) go into A; the
  target's defense-side modifiers (its own strength %, terrain, fortification, city defense,
  bonuses against the hitter) go into Dv (spec, "Unit stats"). An attacker being hit back has no
  terrain, fortification or city defense (it is assaulting, as today); a defender hitting back has
  no attack-side terrain or city bonus (it is not attacking, as today).
- **G2. "Combat won" becomes "kill credit".** FfH's `combatWon` effects fire per duel won, which is
  always a kill. In an engagement they fire for the unit whose hit (volley, round hit, defensive
  strike or damage spell) killed the target, once per such kill, resolved in the post-battle step.
  Effects of losing fire on the dead unit. Units killed by collateral or spells give kill credit to
  the unit or caster that dealt it.
- **G3. Damage stays in percent of max HP.** The spec keeps the damage field a percentage, so every
  FfH damage number (spell damage and limits, defensive strike, collateral and its limit, combat
  limit, heals) keeps its value as a percentage of the target's max HP. Only B x A/(A+Dv) x r is new.
- **G4. Map casting stays.** `iCombatPhase` adds an in-battle cast; every spell can still be cast on
  the map before attacking, as today.
- **G5. Visibility, war and capture rules stay.** Who may be attacked, which units the attacker can
  see on the plot, hidden nationality and city capture work as today.

---

## 1. Strength percentage

**Today.** `iCombatPercent` is added to the unit's combat modifier (`unit_change_extra_combat_percent`)
and so raises its strength both attacking and defending. `iCombatPercentDefense` is read from the XML
but never applied: no rules code reads it, in C++ or in the port (only `ffh-ai`'s `magic::promo_pct`
looks at it, a misvaluation already noted in docs/ENTITIES.md). For 22 of these promotions the two
fields are equal; they differ for Enraged (+20 / -20) and Shield of Faith (+10 / +30), and Withered
and Puppet have no defense field.

**Under engagements.** The spec's remap table maps Combat I-V and strength % to A and Dv. With A
and Dv separate, the defense field finally has a place to go, and the values read as the authors'
intent: Enraged a reckless berserk (hits harder, guards worse), Shield of Faith a defensive blessing.

**Proposal.** REMAP: `iCombatPercent` adds to A; `iCombatPercentDefense` adds to Dv (**Q1**). For
Withered and Puppet, add `iCombatPercentDefense` -10 and -20 to the XML so they keep weakening Dv as
they do today. If the owner prefers the spec's literal reading, `iCombatPercent` goes to both and
nothing changes from today. Combat I-V also carry `iSpellDamageModify` +5 (section 30) and give the
holder's summons Empower I-V (section 31).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BURNING_BLOOD` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_COMBAT1` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_COMBAT2` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_COMBAT3` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_COMBAT4` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_COMBAT5` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_DISEASED` | iCombatPercent -30; iCombatPercentDefense -30 | REMAP: A -30%, Dv -30% (same under Q1 or not) |
| `PROMOTION_EMPOWER1` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_EMPOWER2` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_EMPOWER3` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_EMPOWER4` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_EMPOWER5` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_ENCHANTED_BLADE` | iCombatPercent +20; iCombatPercentDefense +20 | REMAP: A +20%, Dv +20% (same under Q1 or not) |
| `PROMOTION_ENRAGED` | iCombatPercent +20; iCombatPercentDefense -20 | REMAP: A +20%, Dv -20% under Q1 (today +20% on both) |
| `PROMOTION_FATIGUED` | iCombatPercent -10; iCombatPercentDefense -10 | REMAP: A -10%, Dv -10% (same under Q1 or not) |
| `PROMOTION_HEAVY` | iCombatPercent +30; iCombatPercentDefense +30 | REMAP: A +30%, Dv +30% (same under Q1 or not) |
| `PROMOTION_LIGHT` | iCombatPercent -20; iCombatPercentDefense -20 | REMAP: A -20%, Dv -20% (same under Q1 or not) |
| `PROMOTION_MORALE` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_PLAGUED` | iCombatPercent -60; iCombatPercentDefense -60 | REMAP: A -60%, Dv -60% (same under Q1 or not) |
| `PROMOTION_POISONED` | iCombatPercent -10; iCombatPercentDefense -10 | REMAP: A -10%, Dv -10% (same under Q1 or not) |
| `PROMOTION_PUPPET` | iCombatPercent -20 | REMAP: A -20%, Dv -20% (Q1: today's Dv effect kept only if the XML gets iCombatPercentDefense -20) |
| `PROMOTION_RUSTED` | iCombatPercent -10; iCombatPercentDefense -10 | REMAP: A -10%, Dv -10% (same under Q1 or not) |
| `PROMOTION_SHIELD_OF_FAITH` | iCombatPercent +10; iCombatPercentDefense +30 | REMAP: A +10%, Dv +30% under Q1 (today +10% on both) |
| `PROMOTION_UNHOLY_TAINT` | iCombatPercent -25; iCombatPercentDefense -25 | REMAP: A -25%, Dv -25% (same under Q1 or not) |
| `PROMOTION_VAMPIRE` | iCombatPercent +10; iCombatPercentDefense +10 | REMAP: A +10%, Dv +10% (same under Q1 or not) |
| `PROMOTION_WITHERED` | iCombatPercent -10 | REMAP: A -10%, Dv -10% (Q1: today's Dv effect kept only if the XML gets iCombatPercentDefense -10) |

## 2. Flat strength and affinities

**Today.** `iExtraCombatStr` changes the unit's base (attack) strength and `iExtraCombatDefense`
its base defense strength while the promotion is held. `BonusAffinities` add strength per resource
of that kind the owner has (`unit_update_bonus_affinity`): for a mana resource with a damage type
(Air: lightning, Fire and Sun: fire, Death: death, Entropy: unholy, Ice and Water: cold) the bonus is
typed strength (section 3), otherwise plain strength.

**Under engagements.** The spec derives A and Dv from "the unit's existing strength" (and the
separate defense strength as the base of Dv) times the role multiplier.

**Proposal.** KEEP: `iExtraCombatStr` adds to base A and `iExtraCombatDefense` to base Dv, before
the role multiplier and the percentage modifiers, exactly as they add to base strength today.
Promotions that give only one side (Dexterous, Golden Hammer, Sinister, Heroic Strength: attack;
Empty Bier, Heroic Defense, Stoneskin: defense) become more distinct, which is their intent.

`PROMOTION_GREAT_COMMANDER` (+1 / +1, and +1 XP per kill in section 23) is what the Commander unit
gives the unit it joins; whether that makes it an army commander is Q13.

Unit abilities: 20 unit types have `BonusAffinities`: the elementals and summons tied to a mana
(Air, Earth, Fire, Ice, Water Elementals, Aurealis +2 Sun, Spectre and Wraith Death, Mistform and
Shadowrider Shadow), the Djinn (+1 per mana of every kind), Druids (Nature or Earth), Yvain (+2 Nature),
Chalid (+2 Sun), Brigit (Fire) and Gurid (+1 per Banana). KEEP.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BRONZE_WEAPONS` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_BUCCANEERS` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_DEXTEROUS` | iExtraCombatStr +1 | KEEP: base A +1 |
| `PROMOTION_EMPTY_BIER` | iExtraCombatDefense +2 | KEEP: base Dv +2 |
| `PROMOTION_GODSLAYER` | iExtraCombatStr +3; iExtraCombatDefense +3 | KEEP: base A +3, base Dv +3 |
| `PROMOTION_GOLDEN_HAMMER` | iExtraCombatStr +1 | KEEP: base A +1 |
| `PROMOTION_GREAT_COMMANDER` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_HEROIC_DEFENSE` | iExtraCombatDefense +1 | KEEP: base Dv +1 |
| `PROMOTION_HEROIC_DEFENSE2` | iExtraCombatDefense +1 | KEEP: base Dv +1 |
| `PROMOTION_HEROIC_STRENGTH` | iExtraCombatStr +1 | KEEP: base A +1 |
| `PROMOTION_HEROIC_STRENGTH2` | iExtraCombatStr +1 | KEEP: base A +1 |
| `PROMOTION_IRON_WEAPONS` | iExtraCombatStr +2; iExtraCombatDefense +2 | KEEP: base A +2, base Dv +2 |
| `PROMOTION_MITHRIL_WEAPONS` | iExtraCombatStr +4; iExtraCombatDefense +4 | KEEP: base A +4, base Dv +4 |
| `PROMOTION_NIGHTMARE` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_ROD_OF_WINDS` | BonusAffinities Air +1 | KEEP: +1 lightning strength per Air mana owned (typed, section 3) |
| `PROMOTION_SCORCHED_STAFF` | BonusAffinities Fire +1 | KEEP: +1 fire strength per Fire mana owned (typed, section 3) |
| `PROMOTION_SINISTER` | iExtraCombatStr +1 | KEEP: base A +1 |
| `PROMOTION_SKELETON_CREW` | iExtraCombatStr -1; iExtraCombatDefense -1 | KEEP: base A -1, base Dv -1 |
| `PROMOTION_STAFF_OF_SOULS` | BonusAffinities Death +1 | KEEP: +1 death strength per Death mana owned (typed, section 3) |
| `PROMOTION_STONESKIN` | iExtraCombatDefense +2 | KEEP: base Dv +2 |
| `PROMOTION_STRONG` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_UNHOLY_TAINT` | iExtraCombatStr -1; iExtraCombatDefense -1 | KEEP: base A -1, base Dv -1 |
| `PROMOTION_WARCRY` | iExtraCombatStr +1; iExtraCombatDefense +1 | KEEP: base A +1, base Dv +1 |
| `PROMOTION_WEAK` | iExtraCombatStr -1; iExtraCombatDefense -1 | KEEP: base A -1, base Dv -1 |

## 3. Damage types and resistances

**Today.** Typed strength (`DamageTypeCombats` on units and promotions, plus typed mana affinities)
is added to base strength, attacking and defending alike (`unit_base_combat_str`,
`unit_base_combat_str_defense`). Against a given opponent each type's points are scaled by
(100 - the opponent's resistance to it)% (`unit_max_combat_str`; resistance clamped to -100..100),
so a 6 + 3 fire Fire Elemental fights a fire-immune unit at 6. Resistances also cut spell damage
(`unit_do_damage`) and the chance to be poisoned after combat (section 24). Negative resistances
are vulnerabilities (Vulnerable to Fire -50, Frostling fire -50, Undead holy -50, Demon holy -25,
Elf and Dark Elf cold -25, Bronze and Iron Weapons lightning -25).

Unit abilities: 88 unit types carry typed strength: unholy 26, poison 23, holy 19, fire 10, death 7,
cold 6, lightning 2. Three are all typed: Fireball (0 + 4 fire), Meteor (0 + 6 fire), Lightning
Elemental (0 + 3 lightning). Large typed shares: Auric Ascended (30 + 30 cold), Ars (13 + 11 death
+ 5 unholy), Yersinia (15 + 7 poison + 5 unholy), Meshabber (19 + 6 fire + 6 unholy), Drifa (14 + 7 cold).

**Under engagements.** The spec: "FfH's typed extra damage (fire, cold, poison and so on) adds to
each hit, reduced by the target's matching resistance."

**Proposal (Q2).** Two readings, and the choice matters:

- *Flat per-hit damage (the spec's wording).* A and Dv drop the typed points; each hit adds
  K x points x (100 - resistance)% of damage, K a new define. No K works across the range. With
  B = 20 and K = 1: an archer 3 + 1 fire against Dv 3 hits for 11 (today's equivalent, typed
  points in A, is 11.4): fine. Ars against Dv 10 hits for 11.3 + 16 = 27 instead of 15: nearly
  double. Fireball, Meteor and the Lightning Elemental have A = 0 and hit a Dv 5 target for 4,
  6 and 3 instead of 9, 11 and 7.5. Typed units also lose their typed points from Dv (a Fire
  Elemental defends at 6 instead of 9).
- *Typed points in A and Dv, scaled per target (FfH's rule).* Each hit computes
  A = physical A + sum of typed points x (100 - target's resistance)%, and the target's
  Dv = physical Dv + its typed points x (100 - hitter's resistance)%, then the spec's formula.
  "Adds to each hit, reduced by the target's matching resistance" holds per hit, the balance of
  FfH's typed units carries over, and no new constant is needed.

I recommend the second. Resistances: KEEP in both (they also keep cutting spell damage and poison).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AERONS_CHOSEN` | DamageTypeCombats Unholy +2 | typed strength: REMAP (Q2) |
| `PROMOTION_ANGEL` | DamageTypeResists Death +50, Fire +20, Holy +100, Poison +100 | resistances: KEEP |
| `PROMOTION_ATHAME` | DamageTypeCombats Poison +3 | typed strength: REMAP (Q2) |
| `PROMOTION_BLESSED` | DamageTypeCombats Holy +1 | typed strength: REMAP (Q2) |
| `PROMOTION_BRONZE_WEAPONS` | DamageTypeResists Lightning -25 | resistances: KEEP |
| `PROMOTION_COLD_RESISTANCE` | DamageTypeResists Cold +50 | resistances: KEEP |
| `PROMOTION_DARK_ELF` | DamageTypeResists Cold -25 | resistances: KEEP |
| `PROMOTION_DEMON` | DamageTypeResists Death +100, Fire +20, Holy -25, Poison +100, Unholy +100 | resistances: KEEP |
| `PROMOTION_DWARF` | DamageTypeResists Poison +25 | resistances: KEEP |
| `PROMOTION_ELEMENTAL` | DamageTypeResists Death +100, Poison +100 | resistances: KEEP |
| `PROMOTION_ELF` | DamageTypeResists Cold -25 | resistances: KEEP |
| `PROMOTION_FIRE_RESISTANCE` | DamageTypeResists Fire +50 | resistances: KEEP |
| `PROMOTION_FLAMING_ARROWS` | DamageTypeCombats Fire +1 | typed strength: REMAP (Q2) |
| `PROMOTION_FROSTLING` | DamageTypeCombats Cold +1; DamageTypeResists Cold +100, Fire -50 | typed strength: REMAP (Q2); resistances: KEEP |
| `PROMOTION_GELA` | DamageTypeCombats Unholy +2 | typed strength: REMAP (Q2) |
| `PROMOTION_GODSLAYER` | DamageTypeResists Cold +100, Death +100, Fire +100, Holy +100, Lightning +100, Poison +100, Unholy +100 | resistances: KEEP |
| `PROMOTION_GOLEM` | DamageTypeResists Death +100, Poison +100 | resistances: KEEP |
| `PROMOTION_ILLUSION` | DamageTypeResists Death +100, Poison +100 | resistances: KEEP |
| `PROMOTION_IMMUNE_COLD` | DamageTypeResists Cold +100 | resistances: KEEP |
| `PROMOTION_IMMUNE_FIRE` | DamageTypeResists Fire +100 | resistances: KEEP |
| `PROMOTION_IMMUNE_LIGHTNING` | DamageTypeResists Lightning +100 | resistances: KEEP |
| `PROMOTION_IRON_WEAPONS` | DamageTypeResists Lightning -25 | resistances: KEEP |
| `PROMOTION_JADE_TORC` | DamageTypeResists Poison +100 | resistances: KEEP |
| `PROMOTION_LIGHTNING_RESISTANCE` | DamageTypeResists Lightning +50 | resistances: KEEP |
| `PROMOTION_LIZARDMAN` | DamageTypeResists Poison +10 | resistances: KEEP |
| `PROMOTION_MAGIC_IMMUNE` | DamageTypeResists Cold +50, Death +50, Fire +50, Lightning +50 | resistances: KEEP |
| `PROMOTION_MAGIC_RESISTANCE` | DamageTypeResists Cold +20, Death +20, Fire +20, Lightning +20 | resistances: KEEP |
| `PROMOTION_NETHER_BLADE` | DamageTypeCombats Death +2 | typed strength: REMAP (Q2) |
| `PROMOTION_ORC` | DamageTypeResists Fire +25 | resistances: KEEP |
| `PROMOTION_ORTHUSS_AXE` | DamageTypeCombats Fire +1 | typed strength: REMAP (Q2) |
| `PROMOTION_POISON_RESISTANCE` | DamageTypeResists Poison +50 | resistances: KEEP |
| `PROMOTION_POISONED_BLADE` | DamageTypeCombats Poison +1 | typed strength: REMAP (Q2) |
| `PROMOTION_SHEUT_STONE` | DamageTypeCombats Death +1; DamageTypeResists Death +25, Holy -10 | typed strength: REMAP (Q2); resistances: KEEP |
| `PROMOTION_STAFF_OF_SOULS` | DamageTypeCombats Death +3 | typed strength: REMAP (Q2) |
| `PROMOTION_STONESKIN` | DamageTypeResists Cold +50, Death +50, Fire +50, Holy +50, Lightning +50, Poison +50, Unholy +50 | resistances: KEEP |
| `PROMOTION_SUNDERED` | DamageTypeResists Unholy +25 | resistances: KEEP |
| `PROMOTION_UNDEAD` | DamageTypeResists Death +100, Holy -50, Poison +100, Unholy +50 | resistances: KEEP |
| `PROMOTION_UNHOLY_TAINT` | DamageTypeCombats Unholy +1; DamageTypeResists Unholy +25 | typed strength: REMAP (Q2); resistances: KEEP |
| `PROMOTION_VULNERABLE_TO_FIRE` | DamageTypeResists Fire -50 | resistances: KEEP |
| `PROMOTION_WAR` | DamageTypeCombats Unholy +3 | typed strength: REMAP (Q2) |
| `PROMOTION_WINTERBORN` | DamageTypeResists Cold +20, Fire -10 | resistances: KEEP |

## 4. First strikes and first-strike immunity

**Today.** When a duel starts each side gets `iFirstStrikes` (unit + promotions) plus a roll of 0
to `iChanceFirstStrikes` ("First Strike" sync rand in `unit_set_combat_unit`), or none if the
opponent is immune (`bFirstStrikeImmune` on the unit, `bImmuneToFirstStrikes` on a promotion).
During its first-strike rounds a unit deals damage when it wins a round and takes none when it loses.

Unit abilities: 36 unit types have 1 first strike: the archer line (Archer, Longbowman,
Crossbowman, Marksman, Firebow, Dwarven Slinger, Javelin Thrower, Frostling Archer, Scorpion Clan
Archer, Nightwatch, Flurry, Gilden, Arthendain), the horse archers (Horse Archer, Camel Archer,
Centaur Archer, Fyrdwell, Nyxkin, Kithra), the assassins (Assassin, Shadow, Dwarven Shadow,
Courtesan, Ghost, Chanter, Lizardman Assassin, Alazkan, Angel of Death), the dragons Abashi,
Acheron, Drifa and Eurabatres, the Horseman, Scorpion, Giant Spider and Spiderkin; Spiderkin also
has 2 chance first strikes. 17 are immune: the assassins, the horse archers, Fireball and Meteor.

**Under engagements.** The spec's remap table turns first strikes and Drill from free combat
rounds into opening volleys: "Units with first strikes fire that many volleys before melee and take
no return damage."

**Proposal.** REMAP: a unit's volleys = its first strikes + a 0..chance roll made when the
engagement starts (the same sync-rand draw as today). Both sides fire; volley k is fired by every
unit with at least k volleys, damage applied together at the end of each volley round like a
normal round. A volley is a normal hit (same formula, same role targeting). Immunity: REMAP to
"cannot be the target of an opening volley" (a volley picks among non-immune targets in its
priority order; with none it is lost); the immune unit still fires its own volleys. Note the shift:
today a first strike also shields the striker while melee is going on; in the engagement it only
buys free hits before melee starts.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AERONS_CHOSEN` | bImmuneToFirstStrikes | REMAP: never targeted by opening volleys |
| `PROMOTION_BLUR` | bImmuneToFirstStrikes | REMAP: never targeted by opening volleys |
| `PROMOTION_COMMANDO` | iChanceFirstStrikesChange +1 | REMAP: 0 to 1 extra volleys, rolled at the start |
| `PROMOTION_DANCE_OF_BLADES` | iFirstStrikesChange +1 | REMAP: +1 opening volley |
| `PROMOTION_DRILL1` | iChanceFirstStrikesChange +1 | REMAP: 0 to 1 extra volleys, rolled at the start |
| `PROMOTION_DRILL2` | iFirstStrikesChange +1 | REMAP: +1 opening volley |
| `PROMOTION_DRILL3` | iChanceFirstStrikesChange +2 | REMAP: 0 to 2 extra volleys, rolled at the start |
| `PROMOTION_DRILL4` | iFirstStrikesChange +2 | REMAP: +2 opening volleys |
| `PROMOTION_FLANKING3` | bImmuneToFirstStrikes | REMAP: never targeted by opening volleys |
| `PROMOTION_HIDDEN` | iFirstStrikesChange +1 | REMAP: +1 opening volley |
| `PROMOTION_STONESKIN` | iFirstStrikesChange +3 | REMAP: +3 opening volleys |

## 5. Defensive strike

**Today** (FfH, `unit_do_defensive_strike`, before every attack). Every unit on the defended plot
at war with the attacker that has a strike chance and has not attacked this turn (or has Blitz)
rolls its chance; the successful striker with the highest strike damage hits the attacker for
rand(D) + rand(D) - 2 x the attacker's defense strength - the attacker's level, never taking it
past 95 damage. The striker is then used up for the turn unless it has Blitz, and gains 1 XP.
Immune attackers (`bImmuneToDefensiveStrike`) skip it. Uncommitted archers in a stack thus wear
down each attacker before its duel.

Unit abilities: 20 unit types strike (chance / damage): Archer, Scorpion Clan Archer, Dwarven
Slinger, Javelin Thrower, Frostling Archer and the horse archers but Kithra 20% / 10; Longbowman,
Firebow, Nightwatch, Arquebus, Gilden 30% / 15; Crossbowman, Flurry, Marksman, Arthendain 40% / 20;
Teutorix 30% / 25. Immune: Assassin, Shadow, Dwarven Shadow,
Courtesan, Ghost, Chanter, Lizardman Assassin, Alazkan, Devout, Taskmaster, Rathus, Shadowrider.

**Under engagements.** The spec is silent, and opening volleys only cover committed units.

**Proposal (Q5).** REMAP to opening strikes from the defending plot: in the opening phase, for each
committed attacker that is not immune, the defending plot's strikers (committed or in reserve, not
used up this turn) roll as today, and the best success strikes it with today's formula, as a
percentage of the target's max HP (G3); each striker strikes at most once per turn unless it has
Blitz. This keeps the archers in reserve worth something, keeps Drill, Keen Sight and Precision
meaningful, and keeps the assassins' immunity a real trait. The alternative is REMOVE (opening
volleys stand in for archers), which would leave Drill I-IV with only their first strikes and
collateral protection and Keen Sight and Precision with nothing.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BLUR` | bImmuneToDefensiveStrike | KEEP: immune to defensive strikes (Q5) |
| `PROMOTION_DRILL1` | iDefensiveStrikeChance +10; iDefensiveStrikeDamage +5 | REMAP per Q5: strike chance +10%, strike damage +5 |
| `PROMOTION_DRILL2` | iDefensiveStrikeChance +10; iDefensiveStrikeDamage +5 | REMAP per Q5: strike chance +10%, strike damage +5 |
| `PROMOTION_DRILL3` | iDefensiveStrikeChance +10; iDefensiveStrikeDamage +5 | REMAP per Q5: strike chance +10%, strike damage +5 |
| `PROMOTION_DRILL4` | iDefensiveStrikeChance +10; iDefensiveStrikeDamage +5 | REMAP per Q5: strike chance +10%, strike damage +5 |
| `PROMOTION_KEEN_SIGHT` | iDefensiveStrikeChance +20 | REMAP per Q5: strike chance +20% |
| `PROMOTION_PRECISION` | iDefensiveStrikeDamage +10 | REMAP per Q5: strike damage +10 |
| `PROMOTION_SHADOWWALK` | bImmuneToDefensiveStrike | KEEP: immune to defensive strikes (Q5) |

## 6. Withdrawal

**Today.** An attacker whose next hit taken would kill it rolls its withdrawal chance
(`iWithdrawalProb` + promotions' `iWithdrawalChange`) and escapes alive with its damage, +1 XP
(`EXPERIENCE_FROM_WITHDRAWL`). FfH adds defensive withdrawal: a defender outside a city about to die
rolls `iWithdrawlProbDefensive` + the same promotion bonus, and flees to the nearest valid plot (the
attacker may then advance). Withdrawal is 0 for a land unit on water and for an immobile unit (Held,
Blinding Light, Entangle, Spores).

Unit abilities: 43 unit types withdraw. Catapult and Trebuchet 80; Magnadine and Mary 50; Knight,
Hornguard, Bison Rider, Centaur Lancer, Death Knight, Shadowrider, the horse archers, Kithra,
Revelers 35; Horseman, Boar Rider, Centaur, Centaur Charger, both Chariots, War Elephant, War
Machine, Cannon, Dwarven Cannon, Herne, Ophanim, Harlequin, Ratha, both Rosiers, Valin 25; Black
Wind, Pirate, Privateer, Mounted Mercenary, Sailor's Dirge 20; the three Wolf Riders and the
Mercenary 10. Defensive: Loki 100, Loki's Puppet 10.

**Under engagements.** Spec: "After each round, an attacking unit with a withdrawal chance and HP
below [WITHDRAW_HP_PCT]% rolls to leave the engagement alive." (WITHDRAW_HP_PCT is 40%.)

**Proposal.** REMAP per the spec, with two points (**Q3**):

- *One roll per unit per engagement*, made the first time it ends a round below 40% (or would die).
  Rolled every round, a 35% Knight is out with 73% chance by its third roll and the Catapult's 80%
  becomes 99%: the XML numbers would stop meaning what they say.
- *Defenders too.* A defending unit outside a city rolls its defensive withdrawal (unit field +
  promotions' `iWithdrawalChange`, as today) under the same trigger, leaves the engagement and flees
  to an adjacent plot after it. Without this Loki's whole defensive identity disappears.

A withdrawn unit keeps its damage and gets the withdrawal XP; withdrawn attackers stay in their army.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_FAIR_WINDS` | iWithdrawalChange +25 | KEEP: naval, unchanged in v1 |
| `PROMOTION_FLANKING1` | iWithdrawalChange +20 | REMAP: +20% on the withdrawal roll (Q3), attacking and, outside cities, defending |
| `PROMOTION_FLANKING2` | iWithdrawalChange +20 | REMAP: +20% on the withdrawal roll (Q3), attacking and, outside cities, defending |
| `PROMOTION_FLANKING3` | iWithdrawalChange +10 | REMAP: +10% on the withdrawal roll (Q3), attacking and, outside cities, defending |
| `PROMOTION_HELD` | bHeld | KEEP: no withdrawal roll while held |
| `PROMOTION_HOMELAND` | iWithdrawalChange +10 | REMAP: +10% on the withdrawal roll (Q3), attacking and, outside cities, defending |
| `PROMOTION_HORSELORD` | iWithdrawalChange +10 | REMAP: +10% on the withdrawal roll (Q3), attacking and, outside cities, defending |
| `PROMOTION_LIGHT` | iWithdrawalChange +10 | REMAP: +10% on the withdrawal roll (Q3), attacking and, outside cities, defending |

## 7. Collateral damage

**Today** (`unit_collateral_combat`, once per attack, before the duel). The attacker damages up to
`iCollateralDamageMaxUnits` other visible enemy defenders on the plot (picked at random weighted by
current HP): COLLATERAL_COMBAT_DAMAGE 10 x (C + F) / (T + F), where C is its base strength x
`iCollateralDamage`%, T the target's base strength and F = (C + T + 1) / 2; then x (100 + Barrage %)
and x (100 - the target's protection %); never past `iCollateralDamageLimit`% (scaled down for
stronger targets). Only the attacking side splashes.

Unit abilities (damage % / limit % / max units): Catapult 100/50/6, Trebuchet 100/70/6, Cannon and
Dwarven Cannon 100/60/7, Meteor 100/60/4, Fire Elemental and Fireball 50/30/4, Berserker, Brujah,
Circle of Urd, Clockwork Golem, Seraph 40/20/4, Dwarven Shadow 40/50/2, Lunatic 20/10/4.
`UnitCombatCollateralImmunes` is empty in FfH.

**Under engagements.** Spec: "Siege units with collateral damage also hit up to their existing
max-units count of other enemies, within their existing damage limit. This includes uncommitted
units on the defended plot, on purpose."

**Proposal.** KEEP the formula, the three unit fields and the two promotion fields. Two points the
spec leaves open (**Q4**): (a) *timing*: once per engagement per committed collateral unit, in the
opening phase, like today's once per attack (every round, a Cannon would splash 7 units up to 40
times); (b) *by field, not by role*: every unit with `iCollateralDamage` splashes, which includes the
ten non-siege ones above (Berserker, Brujah, Circle of Urd, Clockwork Golem, Seraph, Dwarven
Shadow, Lunatic, Fire Elemental, Fireball, Meteor; the last two per Q11). Targets: any enemy unit
on the defended plot that the attacker can see, committed or not. Defending collateral units still
do not splash, as today.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BARRAGE1` | iCollateralDamageChange +40 | KEEP: +40% collateral damage dealt |
| `PROMOTION_BARRAGE2` | iCollateralDamageChange +60 | KEEP: +60% collateral damage dealt |
| `PROMOTION_BARRAGE3` | iCollateralDamageChange +100 | KEEP: +100% collateral damage dealt |
| `PROMOTION_DRILL3` | iCollateralDamageProtection +20 | KEEP: 20% less collateral damage taken |
| `PROMOTION_DRILL4` | iCollateralDamageProtection +20 | KEEP: 20% less collateral damage taken |

## 8. Flanking strikes

**Today.** `FlankingStrikes` (unit field: damage to listed unit classes on the plot after the duel
or a withdrawal) is handled by `unit_flanking_strike_combat`, but no FfH unit type sets it (0 of 325):
the code path never fires. The Flanking I-III promotions are withdrawal promotions (section 6).

**Proposal.** REMOVE (spec: "Removed; the flanker role covers it"). Nothing in the data is lost.

## 9. Blitz

**Today.** A unit with Blitz may attack again while it has moves, and may defensive-strike even
after attacking. Free Blitz: Buboes, Flurry, Herne, Lightning Elemental.

**Proposal.** REMAP per the spec: one extra attack per turn. Keep the defensive-strike exemption
(section 5).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BLITZ` | bBlitz | REMAP: one extra attack per turn (spec); a Blitz striker may also defensive-strike after attacking |
| `PROMOTION_BURNING_BLOOD` | bBlitz | REMAP: one extra attack per turn (spec); a Blitz striker may also defensive-strike after attacking |
| `PROMOTION_ORTHUSS_AXE` | bBlitz | REMAP: one extra attack per turn (spec); a Blitz striker may also defensive-strike after attacking |
| `PROMOTION_WARCRY` | bBlitz | REMAP: one extra attack per turn (spec); a Blitz striker may also defensive-strike after attacking |

## 10. City attack, city defense, bombard and building defense

**Today.** City attack % raises the attacker against a city's defender; city defense % raises a
unit defending a city; bombard lowers the city's defense; `bIgnoreBuildingDefense` drops the
building part of the city's defense (walls) when that unit attacks. CITY_BARBARIAN_DEFENSE_MODIFIER
(+25) helps cities against barbarian attackers.

Unit abilities: `iCityAttack` on 50 types (Mithril Golem +60, Phalanx, Stoneskin Ogre, Verdandi +25,
Beastman +20, Axeman line, Lucian, Radiant Guard +10; animals, spiders, Ravenous Werewolf and
Scorpion -25; hunters, rangers, druids, beastmasters and other recon -20; assassins -50 and
Shadows -25). `iCityDefense` on 26 (archers +25, Maros +50, Minotaur, Orthus, Gargoyle, Warrior,
Bloodpet, Beastman +25; assassins -50, Shadows -25). `iBombardRate` on 11 (Battering Ram 25,
Catapult 15, Trebuchet 25, Cannon 20, Dwarven Cannon 30, Hill Giant 15, Fireball 10, Meteor 15;
Trireme, Frigate, Man-o-War). `bIgnoreBuildingDefense`: Arquebus, Cannon, Dwarven Cannon,
Mistform, Shadowrider, Teutorix.

**Under engagements.** Spec: city-attack bonuses apply to A, city defense to Dv only, and attacking
a city costs the attacker one slot per 25% (SLOTS_DEFENSE_DIVISOR) of the city's current defense.

**Proposal.** KEEP per the spec. Bombard matters more than today: every 25% of defense knocked
down returns one attacker slot. `bIgnoreBuildingDefense` stays per hit (the target's Dv drops the
building part when this unit hits); it does not change the slot count, which belongs to the side.
City Garrison III and City Raider III also carry +20% against melee (section 14); Morale also +10%
strength (section 1).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_ACCURACY` | iBombardRateChange +20 | KEEP: +20% bombard rate; lowers the city defense that costs attacker slots |
| `PROMOTION_CITY_GARRISON1` | iCityDefense +30 | KEEP: Dv +30% while defending a city |
| `PROMOTION_CITY_GARRISON2` | iCityDefense +30 | KEEP: Dv +30% while defending a city |
| `PROMOTION_CITY_GARRISON3` | iCityDefense +30 | KEEP: Dv +30% while defending a city |
| `PROMOTION_CITY_RAIDER1` | iCityAttack +30 | KEEP: A +30% against targets in a city |
| `PROMOTION_CITY_RAIDER2` | iCityAttack +30 | KEEP: A +30% against targets in a city |
| `PROMOTION_CITY_RAIDER3` | iCityAttack +30 | KEEP: A +30% against targets in a city |
| `PROMOTION_MORALE` | iCityAttack +10 | KEEP: A +10% against targets in a city |
| `PROMOTION_SHADOWWALK` | bIgnoreBuildingDefense | KEEP: its hits ignore the target city's building defense in Dv |

## 11. Terrain, hills and feature modifiers

**Today.** A unit's terrain, hills and feature *defense* modifiers raise it when it defends on such
a plot; its *attack* modifiers raise it when it attacks a plot of that kind (no FfH unit or promotion
has `iHillsAttack`). The plot's own defense (hills +25, forest, etc.) and the unit's modifiers add.

Unit abilities: `iHillsDefense` +25 on 9 archer types; terrain attack and defense: Battlemaster
(tundra 50), Drifa (snow 50, tundra 10), Ice Elemental (snow 20, tundra 10), Sand Lion (desert 40),
Teutorix (desert 25), Wilboman (tundra and snow 25); feature: Chariot (forest and jungle attack -25),
Yvain (forest 50, ancient forest 100, attack and defense): 17 unit types in all.

**Proposal.** KEEP per the spec ("Terrain ... apply to Dv only") and G1: defense modifiers go into
the Dv of a unit defending on that terrain; attack modifiers go into the A of an attacker hitting a
target on that terrain. The new plot-level `iEngagementSlotsChange` is separate and adds to this.
One consequence to be aware of, decided by the spec: today a defender's terrain bonus also raises
the damage it deals (one strength number); under engagements it only cuts the damage it takes.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_DARK_ELF` | FeatureAttacks Forest +10, Ancient Forest +10 | KEEP: A vs targets on Forest +10%, Ancient Forest +10% |
| `PROMOTION_DEMON` | TerrainAttacks Broken Lands +10, Fields of Perdition +10, Burning Sands +10; TerrainDefenses Broken Lands +10, Fields of Perdition +10, Burning Sands +10 | KEEP: A vs targets on Broken Lands +10%, Fields of Perdition +10%, Burning Sands +10%; Dv on Broken Lands +10%, Fields of Perdition +10%, Burning Sands +10% |
| `PROMOTION_ELF` | FeatureAttacks Forest +10, Ancient Forest +10 | KEEP: A vs targets on Forest +10%, Ancient Forest +10% |
| `PROMOTION_FROSTLING` | TerrainAttacks Snow +25, Tundra +10, Desert -25; TerrainDefenses Snow +25, Tundra +10, Desert -25 | KEEP: A vs targets on Snow +25%, Tundra +10%, Desert -25%; Dv on Snow +25%, Tundra +10%, Desert -25% |
| `PROMOTION_GUERILLA1` | iHillsDefense +40 | KEEP: Dv +40% on hills |
| `PROMOTION_GUERILLA2` | iHillsDefense +60 | KEEP: Dv +60% on hills |
| `PROMOTION_LIZARDMAN` | TerrainAttacks Marsh +10; TerrainDefenses Marsh +10 | KEEP: A vs targets on Marsh +10%; Dv on Marsh +10% |
| `PROMOTION_NIGHTMARE` | TerrainAttacks Broken Lands +10, Fields of Perdition +10, Burning Sands +10; TerrainDefenses Broken Lands +10, Fields of Perdition +10, Burning Sands +10 | KEEP: A vs targets on Broken Lands +10%, Fields of Perdition +10%, Burning Sands +10%; Dv on Broken Lands +10%, Fields of Perdition +10%, Burning Sands +10% |
| `PROMOTION_NOMAD` | TerrainAttacks Desert +20; TerrainDefenses Desert +20 | KEEP: A vs targets on Desert +20%; Dv on Desert +20% |
| `PROMOTION_ORC` | FeatureAttacks Jungle +10 | KEEP: A vs targets on Jungle +10% |
| `PROMOTION_WINTERBORN` | TerrainAttacks Snow +10, Tundra +10; TerrainDefenses Snow +10, Tundra +10 | KEEP: A vs targets on Snow +10%, Tundra +10%; Dv on Snow +10%, Tundra +10% |
| `PROMOTION_WOODSMAN1` | FeatureAttacks Jungle +30, Forest +30, Ancient Forest +30; FeatureDefenses Jungle +30, Forest +30, Ancient Forest +30 | KEEP: A vs targets on Jungle +30%, Forest +30%, Ancient Forest +30%; Dv on Jungle +30%, Forest +30%, Ancient Forest +30% |
| `PROMOTION_WOODSMAN2` | FeatureAttacks Jungle +30, Forest +30, Ancient Forest +30; FeatureDefenses Jungle +30, Forest +30, Ancient Forest +30 | KEEP: A vs targets on Jungle +30%, Forest +30%, Ancient Forest +30%; Dv on Jungle +30%, Forest +30%, Ancient Forest +30% |

## 12. Fortification

**Today.** +5% (FORTIFY_MODIFIER_PER_TURN) per turn fortified, up to 5 turns; `bDoubleFortifyBonus`
doubles it. Units without defensive bonus cannot fortify. Gargoyle has Defensive free. The Treetop
Defence spell sets full fortification.

**Proposal.** KEEP: fortification goes into Dv only (spec).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_DEFENSIVE` | bDoubleFortifyBonus | KEEP: fortification bonus doubled (Dv only) |
| `PROMOTION_EMPTY_BIER` | bDoubleFortifyBonus | KEEP: fortification bonus doubled (Dv only) |

## 13. River crossing and amphibious attack

**Today.** Attacking across a river gives the defender +40% (RIVER_ATTACK_MODIFIER -40, from
GlobalDefinesAlt.xml) unless the attacker has `bRiver`; attacking from water onto land gives it +50%
(AMPHIB_ATTACK_MODIFIER -50) unless the attacker has `bAmphib`. No unit type has either; the four
promotions below grant both.

**Under engagements.** Spec: "Attacking across a river costs the attacker [SLOTS_RIVER] slots."
(SLOTS_RIVER is 2.)

**Proposal (Q9).** REMAP: the slot loss replaces the +40% (not both). River-capable units ignore it:
the force gets S - 2 slots for ordinary units plus up to 2 slots only `bRiver` units may fill, so a
Lizardman army crosses at full width. Amphibious attack: naval is unchanged in v1; KEEP the -50% as
an A penalty on units attacking from water, cancelled by `bAmphib`.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AMPHIBIOUS` | bRiver; bAmphib | REMAP: may fill river-lost slots (Q9); no amphibious penalty |
| `PROMOTION_FLYING` | bRiver; bAmphib | REMAP: may fill river-lost slots (Q9); no amphibious penalty |
| `PROMOTION_LIZARDMAN` | bRiver; bAmphib | REMAP: may fill river-lost slots (Q9); no amphibious penalty |
| `PROMOTION_WATER_WALKING` | bRiver; bAmphib | REMAP: may fill river-lost slots (Q9); no amphibious penalty |

## 14. Bonuses against unit combats, unit classes and animals

**Today.** `UnitCombatMods` work both ways (`unit_max_combat_str` adds the defender's bonus against
the attacker's combat and subtracts the attacker's bonus against the defender's): +40% against
melee helps attacking melee and defending against it. `UnitClassAttackMods` work only when attacking
that class; `iAnimalCombat` both ways against animals.

Unit abilities: `UnitCombatMods` on 25 types: against melee Champion, Dragon Slayer, Demagog,
Rantine, Saverous +25, Rathus +50; against archers the Horseman line (Horseman, Boar Rider, Centaur,
Wolf Rider, Frostling Wolf Rider) +40, Ophanim +50; against mounted War Elephant, War Machine, War
Tortoise, Griffon +50, Manticore +25; against disciples Luonnotar +50, Mardero and Rosier +40;
against recon Ratha +50, Tumtum +25; against naval Boarding Party +25. `iAnimalCombat` +50 on 18
(Scout, Hunter, Ranger, Lizardman lines, Beastmasters, Druids, Goblins, Herald, Hellhound, Divided
Soul, Harlequin, Yvain). `UnitClassAttackMods`: Beast of Agares +25 against Paladins.
`UnitClassDefenseMods` and `DomainMods` are empty.

**Under engagements.** The spec's remap table: a bonus against a unit class applies to A or Dv
"when targeting or targeted by that class".

**Proposal.** KEEP per the spec: the bonus goes into A when the holder hits a unit of that kind and
into Dv when a unit of that kind hits it; class attack mods into A only. Targeting stays as the
roles say (uniform within a priority class): a Formation unit does not seek out cavalry. FfH has no
spear line, and Formation is its anti-cavalry promotion; **Q7** asks whether Formation should make
a frontline unit a Screen.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BARRAGE2` | UnitCombatMods Melee +20 | KEEP: A and Dv +20% against melee units |
| `PROMOTION_CITY_GARRISON3` | UnitCombatMods Melee +20 | KEEP: A and Dv +20% against melee units |
| `PROMOTION_CITY_RAIDER3` | UnitCombatMods Melee +20 | KEEP: A and Dv +20% against melee units |
| `PROMOTION_COVER` | UnitCombatMods Archer +40 | KEEP: A and Dv +40% against archer units |
| `PROMOTION_COVER2` | UnitCombatMods Archer +40 | KEEP: A and Dv +40% against archer units |
| `PROMOTION_DRILL4` | UnitCombatMods Mounted +10 | KEEP: A and Dv +10% against mounted units |
| `PROMOTION_FORMATION` | UnitCombatMods Mounted +40 | KEEP: A and Dv +40% against mounted units |
| `PROMOTION_FORMATION2` | UnitCombatMods Mounted +40 | KEEP: A and Dv +40% against mounted units |
| `PROMOTION_SCOURGE` | UnitCombatMods Disciple +40 | KEEP: A and Dv +40% against disciple units |
| `PROMOTION_SHOCK` | UnitCombatMods Melee +40 | KEEP: A and Dv +40% against melee units |
| `PROMOTION_SHOCK2` | UnitCombatMods Melee +40 | KEEP: A and Dv +40% against melee units |
| `PROMOTION_SUBDUE_ANIMAL` | UnitCombatMods Animal +25 | KEEP: A and Dv +25% against animal units |
| `PROMOTION_SUBDUE_BEASTS` | UnitCombatMods Beast +25 | KEEP: A and Dv +25% against beast units |

## 15. Bonuses against promotions (slaying, weapons, heroes)

**Today.** `iPromotionCombatMod` with `PromotionCombatType` gives +X% against any unit holding that
promotion, both ways (the same symmetric handling as unit-combat bonuses).

**Proposal.** KEEP, read like section 14: into A when hitting a unit with that promotion, into Dv
when hit by one. Nether Blade +80% against heroes pairs well with the assassin role's hero priority.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BLESSED` | iPromotionCombatMod +20 vs PROMOTION_DEMON | KEEP: A and Dv +20% against a unit with PROMOTION_DEMON |
| `PROMOTION_DEMON_SLAYING` | iPromotionCombatMod +40 vs PROMOTION_DEMON | KEEP: A and Dv +40% against a unit with PROMOTION_DEMON |
| `PROMOTION_DRAGON_SLAYING` | iPromotionCombatMod +40 vs PROMOTION_DRAGON | KEEP: A and Dv +40% against a unit with PROMOTION_DRAGON |
| `PROMOTION_GODSLAYER` | iPromotionCombatMod +1000 vs PROMOTION_AVATAR | KEEP: A and Dv +1000% against a unit with PROMOTION_AVATAR |
| `PROMOTION_IRON_WEAPONS` | iPromotionCombatMod +10 vs PROMOTION_BRONZE_WEAPONS | KEEP: A and Dv +10% against a unit with PROMOTION_BRONZE_WEAPONS |
| `PROMOTION_MITHRIL_WEAPONS` | iPromotionCombatMod +10 vs PROMOTION_IRON_WEAPONS | KEEP: A and Dv +10% against a unit with PROMOTION_IRON_WEAPONS |
| `PROMOTION_NETHER_BLADE` | iPromotionCombatMod +80 vs PROMOTION_HERO | KEEP: A and Dv +80% against a unit with PROMOTION_HERO |
| `PROMOTION_PERFECT_SIGHT` | iPromotionCombatMod +50 vs PROMOTION_ILLUSION | KEEP: A and Dv +50% against a unit with PROMOTION_ILLUSION |
| `PROMOTION_UNDEAD_SLAYING` | iPromotionCombatMod +40 vs PROMOTION_UNDEAD | KEEP: A and Dv +40% against a unit with PROMOTION_UNDEAD |

## 16. Situational auras

**Today.** Homeland: +10% strength while the unit stands inside its owner's borders
(`unit_get_extra_combat_percent`; an attacker counts its own plot). Stigmata: + (Armageddon
counter x 50 / 100)% (free on Balor and Mardero; Sundered gives it to its demon caster's summons).

**Proposal.** KEEP: both into A and Dv. Homeland also carries +10% withdrawal (section 6).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_HOMELAND` | iCombatPercentInBorders +10 | KEEP: A and Dv +10% while standing in own borders |
| `PROMOTION_STIGMATA` | iCombatPercentGlobalCounter +50 | KEEP: A and Dv + (Armageddon counter x 50 / 100)% |

## 17. Defender choice and targeting

**Today.** Every attack meets the plot's best defender (`unit_is_better_defender_than`), whose
rating is scaled by `iBetterDefenderThanPercent` (100 plus the promotions'): Guardsman x2, each
Channeling level -25 points (a Channeling III mage rates at a quarter), Avatar x11. Marksman
(`bTargetWeakestUnit`) makes its holder attack the *weakest* defender instead, unless a unit with
`bTargetWeakestUnitCounter` (Guardsman) is on the plot. Marksman is free on the assassins
(Assassin, Shadow, Dwarven Shadow, Courtesan, Ghost, Chanter, Lizardman Assassin, Taskmaster,
Alazkan, Angel of Death, Rathus) and the Marksman archer; Guardsman on Donal and the Royal Guard.
The unit fields `UnitClassTargets`, `UnitCombatTargets`, `UnitClassDefenders` and
`UnitCombatDefenders` are empty in FfH (section U6).

**Under engagements.** The defender's force comes from its stance and each hitter picks targets by
role. Assassins (spec role): "Support units and heroes, then other back-position enemies, then
front. Ignores screens." FfH's assassins are `UNITCOMBAT_RECON`, which the spec makes frontline, so
the role needs an override for them.

The C++ rating also weighs first strikes, collateral and `iAsset` (cargo); all of that goes with the
best-defender choice, which the stance auto-pick replaces.

**Proposal (Q6).**

- Marksman: REMAP to "targets with the Assassin profile". That is what the promotion means in
  FfH, and since every FfH assassin carries it free, the Assassin role can default to "has
  Marksman" instead of a hand-made list; an archer or recon unit that earns it at Combat IV keeps its
  own role and position but picks targets like an assassin. "Hero" = has `PROMOTION_HERO`.
- Guardsman: REMAP to a bodyguard: while a living Guardsman is committed on its side, enemy units
  targeting with the Assassin profile must target a Guardsman first (FfH's counter); its x2 rating
  becomes "auto-pick commits Guardsmen first when defending".
- Channeling: REMOVE: it exists to keep mages from defending, which the Support role does.
- Avatar: REMOVE: auto-pick takes the highest Dv in role, and the Auric Ascended (Dv 30) is the
  strongest unit in the game.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AVATAR` | iBetterDefenderThanPercent +1000 | REMOVE: auto-pick already commits the strongest defender (Auric Ascended, Dv 30) |
| `PROMOTION_CHANNELING1` | iBetterDefenderThanPercent -25 | REMOVE: the Support role (back position, Dv x0.5) keeps mages out of the line |
| `PROMOTION_CHANNELING2` | iBetterDefenderThanPercent -25 | REMOVE: as Channeling I |
| `PROMOTION_CHANNELING3` | iBetterDefenderThanPercent -25 | REMOVE: as Channeling I |
| `PROMOTION_GUARDSMAN` | iBetterDefenderThanPercent +100; bTargetWeakestUnitCounter | REMAP (Q6): bodyguard; enemy assassin-profile units must target it first; auto-pick commits it first when defending |
| `PROMOTION_MARKSMAN` | bTargetWeakestUnit | REMAP (Q6): the holder targets with the Assassin profile (support and heroes first, ignores screens and front shielding) |

## 18. Fear

**Today** (FfH, two effects). (a) Before an attack on a defender with Fear, a non-immune attacker
rolls: chance = its base strength + 20 + its level - the defender's base strength - the defender's
level (at least 4), out of 40; on failure it loses the attack and its moves ("is afraid to attack";
equal units balk 47% of the time). (b) After a Fear attacker kills its defender outside a city,
every other non-immune enemy on the plot rolls (fear wins when rand(20) <= the attacker's base
strength + 10 - its own) and flees to the nearest valid plot. Free Fear: Ars, Buboes, Spectre,
Stephanos, Wraith, Yersinia; Dragon: Abashi, Acheron, Drifa, Eurabatres.

**Proposal (Q8).** REMAP: (a) becomes a pre-battle morale check: if the defending force has a
committed Fear unit, each committed non-immune attacker rolls today's check against the strongest
one; those that fail sit out (their attack is used, their slot stays empty). New, and part of
Q8: the same check for a Fear unit in the attacking force against the committed defenders (those
that fail hold the plot but do not fight). (b) becomes a rout: when an attacking force with a Fear
unit wins outside a city, each remaining non-immune enemy on the plot rolls today's check and
flees. The dragons' Roar spell uses
the same roll pre-battle (Spells).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AERONS_CHOSEN` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_ANGEL` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_AVATAR` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_BURNING_BLOOD` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_COURAGE` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_DEMON` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_DRAGON` | bFear; bImmuneToFear | causes fear: REMAP per Q8; immune to fear: KEEP |
| `PROMOTION_ELEMENTAL` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_FEAR` | bFear | causes fear: REMAP per Q8 |
| `PROMOTION_GOLEM` | bImmuneToFear | immune to fear: KEEP |
| `PROMOTION_TIMOR_MASK` | bFear; bImmuneToFear | causes fear: REMAP per Q8; immune to fear: KEEP |
| `PROMOTION_UNDEAD` | bImmuneToFear | immune to fear: KEEP |

## 19. Combat limit

**Today.** An attacker with `iCombatLimit` below 100 cannot take its defender past that damage: the
hit that would is cut to the limit and the duel ends with the attacker withdrawing (+1 XP). Siege
weakens and never kills. Units: Catapult and Trebuchet 75 (every other type 100). Promotion: Illusion
90 (`unit_calc_combat_limit` takes the lowest).

**Proposal.** REMAP to a per-target cap: the holder's hits never push a target past its limit
(percent of max HP, G3); a target already at the cap is skipped in its target choice; when every
target it may hit is capped, the holder leaves the engagement as withdrawn (+1 XP, as today), so an
all-siege attack cannot run to MAX_ROUNDS.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_ILLUSION` | iCombatLimit 90 | REMAP: its hits never take a target past 90% damage; leaves the fight when every target is capped |

## 20. Heal on a kill

**Today.** After a win over a living unit the winner heals `iCombatHealPercent`. Python adds two
units: the Lightning Elemental halves its damage after every win (`postCombatHeal50`) and the Beast of
Agares heals fully after killing a Paladin (`postCombatConsumePaladin`). Cannibalize is free on the
Bone Golem, Ira, Manticore and Yersinia.

**Proposal.** REMAP: in the post-battle step, once per living enemy the unit got kill credit for
(G2); the Python heals likewise. Illusions (heal 100 per kill) stay what they are: disposable copies
that come out of each won fight whole.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AERONS_CHOSEN` | iCombatHealPercent 20 | REMAP: heals 20 after the engagement, once per living enemy it killed |
| `PROMOTION_CANNIBALIZE` | iCombatHealPercent 10 | REMAP: heals 10 after the engagement, once per living enemy it killed |
| `PROMOTION_ILLUSION` | iCombatHealPercent 100 | REMAP: heals 100 after the engagement, once per living enemy it killed |

## 21. Medics and heal rates

**Today.** At turn start a unit heals the base rate for where it stands, plus its own heal-rate
changes (enemy / neutral / friendly territory), plus the best of: the highest same-tile heal on its
plot (Medic I 10, II 15, III 20, Courage 10) and the highest adjacent heal from units next to it
(Medic III 20). Medic I is free on 32 unit types (every disciple and priest, druids, Paladin, Monk,
Crusader, Paramander, Grigori Medic, Arthendain, Luonnotar, Sphener, Yvain), Medic II on 18, Medic
III on 10.

**Under engagements.** The spec's remap table: Medic, which heals the stack over time, also heals
after battle.

**Proposal.** KEEP turn healing and every heal-rate promotion as is (they are not combat). ADD the
post-battle heal (**Q12** for the amount): after each engagement, each side's committed survivors
heal the best same-tile heal among their side's units on their plot (attackers: on the plot they
attacked from), or the best adjacent heal from their side's units next to it, whichever is higher,
as the turn heal picks it; each unit at most once per turn from medics, so a stack that fights three
engagements in a turn is not healed three times.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_COURAGE` | iSameTileHealChange +10 | turn heal +10 on its plot: KEEP; also a post-battle heal of 10 (Q12) |
| `PROMOTION_DISEASED` | iEnemyHealChange -10; iNeutralHealChange -10; iFriendlyHealChange -10 | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_GOLEM` | iNeutralHealChange -5; iFriendlyHealChange -10 | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_MARCH` | iEnemyHealChange +10; iNeutralHealChange +10; iFriendlyHealChange +10; bAlwaysHeal | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_MEDIC1` | iSameTileHealChange +10 | turn heal +10 on its plot: KEEP; also a post-battle heal of 10 (Q12) |
| `PROMOTION_MEDIC2` | iSameTileHealChange +15 | turn heal +15 on its plot: KEEP; also a post-battle heal of 15 (Q12) |
| `PROMOTION_MEDIC3` | iSameTileHealChange +20; iAdjacentTileHealChange +20 | turn heal +20 on its plot: KEEP; also a post-battle heal of 20 (Q12); adjacent heal +20: KEEP, and post-battle for its side on adjacent plots |
| `PROMOTION_PLAGUED` | iEnemyHealChange -20; iNeutralHealChange -20; iFriendlyHealChange -20 | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_POISONED` | iEnemyHealChange -3; iNeutralHealChange -5; iFriendlyHealChange -8 | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_REGENERATION` | iEnemyHealChange +10; iNeutralHealChange +10; iFriendlyHealChange +10; bAlwaysHeal | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_VAMPIRE` | iEnemyHealChange +5; iNeutralHealChange +5; iFriendlyHealChange +5 | own heal rate: KEEP (turn healing, not combat) |
| `PROMOTION_WITHERED` | iEnemyHealChange -20; iNeutralHealChange -20; iFriendlyHealChange -20 | own heal rate: KEEP (turn healing, not combat) |

## 22. Capture and enslavement

**Today** (`unit_combat_won`). Each held promotion with `iCombatCapturePercent` rolls separately on a
win over a living unit (Command I-IV: 10, 20, 20, 20; Crown of Command 30; Stephanos has Command
I-III and the Crown); success converts the loser to the winner's side at 75 damage. `CaptureUnitCombat`
captures every loser of that combat (Subdue Animal, Subdue Beasts, Boarding). Enslavement
(`iEnslavementChance`: Taskmaster and Courtesan 25, plus the player's) turns a living non-animal
loser into a Slave. Blocked by the loser's capture immunity or immortality and by the winner's
`bNoCapture` (Fireball, Meteor). Equipment units (23 types) are always captured.

**Proposal.** REMAP: rolled at kill credit (G2), resolved after the engagement; the captured or
enslaved unit appears on the capturer's plot, outside the engagement and its army. Numbers KEEP.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AVATAR` | bImmuneToCapture | KEEP: cannot be captured |
| `PROMOTION_BOARDING` | CaptureUnitCombat UNITCOMBAT_NAVAL; bBoarding | KEEP: naval boarding, unchanged in v1 |
| `PROMOTION_COMMAND1` | iCombatCapturePercent 10 | REMAP: 10% capture roll per living enemy it kills, resolved after the engagement |
| `PROMOTION_COMMAND2` | iCombatCapturePercent 20 | REMAP: 20% capture roll per living enemy it kills, resolved after the engagement |
| `PROMOTION_COMMAND3` | iCombatCapturePercent 20 | REMAP: 20% capture roll per living enemy it kills, resolved after the engagement |
| `PROMOTION_COMMAND4` | iCombatCapturePercent 20 | REMAP: 20% capture roll per living enemy it kills, resolved after the engagement |
| `PROMOTION_CROWN_OF_COMMAND` | iCombatCapturePercent 30 | REMAP: 30% capture roll per living enemy it kills, resolved after the engagement |
| `PROMOTION_LOYALTY` | bImmuneToCapture | KEEP: cannot be captured |
| `PROMOTION_SUBDUE_ANIMAL` | CaptureUnitCombat UNITCOMBAT_ANIMAL | REMAP: captures every animal it kills, after the engagement |
| `PROMOTION_SUBDUE_BEASTS` | CaptureUnitCombat UNITCOMBAT_BEAST | REMAP: captures every beast it kills, after the engagement |

## 23. Rewards per victory

**Today.** On a win: `iFreeXPFromCombat` XP (and the player's trait value), `iGoldFromCombat` gold
against non-animals (units: Black Wind and Pirate 3), `iModifyGlobalCounterOnCombat` against living
units, and a summon's `iDurationFromCombat` (Host of the Einherjar and Pit Beast +1 turn).
`iExperiencePercent` scales all XP the unit gains.

**Proposal.** REMAP: per kill credit (G2). Battle XP follows the spec ("Civ4's formula against the
units they helped kill"), scaled by `iExperiencePercent` as today. KEEP the numbers. The unit fields
`iXPValueAttack` / `iXPValueDefense` (what a unit is worth in XP, scaled by
BARBARIAN_EXPERIENCE_MODIFIER for animals and barbarians and capped by their max XP values) feed
that formula: KEEP.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BOUNTY_HUNTER` | iGoldFromCombat +2 | REMAP: +2 gold per non-animal enemy it kills |
| `PROMOTION_GREAT_COMMANDER` | iFreeXPFromCombat +1 | REMAP: +1 XP per enemy it kills |
| `PROMOTION_VALOR` | iFreeXPFromCombat +1 | REMAP: +1 XP per enemy it kills |
| `PROMOTION_WAR` | iModifyGlobalCounterOnCombat +1 | REMAP: Armageddon counter +1 per living enemy it kills |
| `PROMOTION_WITHERED` | iExperiencePercent -50 | KEEP: -50% XP from every source, battle XP included |

## 24. Afflictions spread by combat

**Today.** `PromotionCombatApply`: when the holder wins, every enemy unit on the loser's plot that
can take it (living, where the affliction requires it) rolls COMBAT_APPLY_CHANCE (50%); when the
holder loses, its killer rolls. Poison: when a unit with poison typed strength loses, a damaged
winner rolls against its poison resistance to become Poisoned (-10%, slower healing). Plague Carrier
is free on Mary and Yersinia, Vile Touch on Angel of Death, Ars and Wraith, Diseased on the Diseased
Corpse, Athame on Yersinia; 23 unit types have poison strength (the assassins and shadows, spiders,
Scorpion Clan, Manticore, Nightwatch, Revelers, Mary, Yersinia).

**Proposal.** REMAP: once per engagement, not per kill (else a long fight rolls the same plague on
the same units many times): if the holder got kill credit at least once, every enemy unit on the plot
of its victims rolls once; if the holder died, its killer rolls once. Poison: each surviving enemy that
took damage from a poison-typed unit rolls once after the engagement. KEEP the chances.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_ATHAME` | PromotionCombatApply PROMOTION_PLAGUED | REMAP: once per engagement: if it killed, each enemy unit on its victims' plot rolls 50% for PROMOTION_PLAGUED; if it died, its killer rolls |
| `PROMOTION_DISEASED` | PromotionCombatApply PROMOTION_DISEASED | REMAP: once per engagement: if it killed, each enemy unit on its victims' plot rolls 50% for PROMOTION_DISEASED; if it died, its killer rolls |
| `PROMOTION_IMMUNE_DISEASE` | PromotionImmune1 PROMOTION_DISEASED; PromotionImmune2 PROMOTION_PLAGUED | KEEP: never gains Diseased or Plagued |
| `PROMOTION_PLAGUE_CARRIER` | PromotionCombatApply PROMOTION_PLAGUED | REMAP: once per engagement: if it killed, each enemy unit on its victims' plot rolls 50% for PROMOTION_PLAGUED; if it died, its killer rolls |
| `PROMOTION_VILE_TOUCH` | PromotionCombatApply PROMOTION_WITHERED | REMAP: once per engagement: if it killed, each enemy unit on its victims' plot rolls 50% for PROMOTION_WITHERED; if it died, its killer rolls |

## 25. Removed by combat

**Today.** `bRemovedByCombat` promotions are removed from the winner of a duel (Hidden ends,
Stoneskin breaks, Blessed fades, Enraged calms). A unit that withdraws keeps them.

**Proposal.** REMAP: removed after any engagement the unit was committed to, won, withdrawn or
drawn. One engagement is "one fight" for these one-fight effects.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_BLESSED` | bRemovedByCombat | REMAP: removed after any engagement the unit was committed to |
| `PROMOTION_ENRAGED` | bRemovedByCombat | REMAP: removed after any engagement the unit was committed to |
| `PROMOTION_HIDDEN` | bRemovedByCombat | REMAP: removed after any engagement the unit was committed to |
| `PROMOTION_STONESKIN` | bRemovedByCombat | REMAP: removed after any engagement the unit was committed to |

## 26. Cannot attack

**Today.** A unit with `bOnlyDefensive` (unit or Charmed promotion) cannot attack. Unit types:
the Commander, Decius, Loki, the Elephant, the Severed Soul and the non-combatants (Adventurer,
Artist, Engineer, Merchant, Prophet, Scientist, Settler, Supplies, Worker).

**Proposal.** KEEP: never picked for an attacking force; still defends.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_CHARMED` | bOnlyDefensive | KEEP: never picked for an attacking force; still defends |

## 27. Immortality

**Today.** An immortal unit that dies is reborn once at 75 damage and escapes to its owner's
capital, spending its immortality (the Immortal promotion, or one count of the unit flag). Unit
types: Brigit, Bone Golem, Immortal, Ogre Warchief, Skuld, Vampire Lord, Valkyrie; Losha Valas
gains Immortal from its first win over a living unit (`PromotionFromCombat`).

**Proposal.** KEEP: killed in an engagement it is reborn as today, after the engagement, and counts
as a casualty in the battle report.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_IMMORTAL` | bImmortal | KEEP: killed in an engagement, it returns as today |

## 28. Stealth and sight

**Today.** An invisible unit is skipped as a defender and as a collateral target by a team that
cannot see it; Hidden Nationality lets a unit attack without war (and causes no war weariness); See
Invisible reveals invisible units. Unit types with their own invisibility type: Giant Spider
(seen by animals), Severed Soul and the Godslayer item; the Invisible promotion is free on Shadow,
Dwarven Shadow, Courtesan, Mistform and the Angel of Death, Hidden on Ghost and Rathus.

**Proposal.** KEEP (G5): an invisible unit on the defended plot is not committed, targeted,
splashed or hit by area spells unless the attacker sees it; Hidden is removed after the engagement
(section 25).

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_CROWN_OF_BRILLANCE` | bSeeInvisible | KEEP: its side may commit against, target and splash invisible enemies it sees |
| `PROMOTION_HIDDEN` | bInvisible | KEEP: not committed, targeted or splashed by a side that cannot see it |
| `PROMOTION_HIDDEN_NATIONALITY` | bHiddenNationality | KEEP: attacks without war, no war weariness |
| `PROMOTION_INVISIBLE` | bInvisible | KEEP: not committed, targeted or splashed by a side that cannot see it |
| `PROMOTION_PERFECT_SIGHT` | bSeeInvisible | KEEP: its side may commit against, target and splash invisible enemies it sees |

## 29. Not alive

**Today.** Not-alive units (race promotions below) give no heal-on-kill, cannot be captured or
enslaved, do not catch living-only afflictions or poison, and are immune to spells flagged
`bImmuneNotAlive` (Bless, Courage, Haste, Charm Person, Domination, Wither, Contagion, Spores and others).

**Proposal.** KEEP.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_ANGEL` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_AVATAR` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_DEMON` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_ELEMENTAL` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_GOLEM` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_ILLUSION` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_PUPPET` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |
| `PROMOTION_UNDEAD` | bNotAlive | KEEP: gives no heal-on-kill; cannot be captured, enslaved or poisoned; immune to living-only spells |

## 30. Spell resistance, spell damage and twincast

**Today.** These change spells cast on the map: resistance chance (`iResistMagic`, the caster's
`iCasterResistModify` on its targets), immunity (`bImmuneToMagic`), extra damage on damage spells
(`iSpellDamageModify`, added to the spell's damage before the two damage draws), Twincast (twice the
summons). Crown of Brilliance recasts its own damage spell every turn (`PyPerTurn`).

**Proposal.** KEEP: the same rules apply to spells cast in a combat phase. A twincaster's second
summon fits in SUMMON_EXTRA_SLOTS.

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_AVATAR` | bImmuneToMagic; iCasterResistModify -50 | KEEP: immune to magic, its targets resist -50% |
| `PROMOTION_CHANNELING2` | iCasterResistModify -10 | KEEP: its targets resist -10% |
| `PROMOTION_CHANNELING3` | iCasterResistModify -10 | KEEP: its targets resist -10% |
| `PROMOTION_COMBAT1` | iSpellDamageModify +5 | KEEP: spell damage +5 |
| `PROMOTION_COMBAT2` | iSpellDamageModify +5 | KEEP: spell damage +5 |
| `PROMOTION_COMBAT3` | iSpellDamageModify +5 | KEEP: spell damage +5 |
| `PROMOTION_COMBAT4` | iSpellDamageModify +5 | KEEP: spell damage +5 |
| `PROMOTION_COMBAT5` | iSpellDamageModify +5 | KEEP: spell damage +5 |
| `PROMOTION_CROWN_OF_AKHARIEN` | bImmuneToMagic | KEEP: immune to magic |
| `PROMOTION_CROWN_OF_BRILLANCE` | PyPerTurn effectCrownOfBrillance(pCaster) | KEEP: recasts Crown of Brilliance (30 holy, range 1) every turn on the map |
| `PROMOTION_GODSLAYER` | iResistMagic +100 | KEEP: resist +100% |
| `PROMOTION_HERO` | iResistMagic +20 | KEEP: resist +20% |
| `PROMOTION_JADE_TORC` | iResistMagic +20 | KEEP: resist +20% |
| `PROMOTION_MAGIC_IMMUNE` | bImmuneToMagic; iResistMagic +100 | KEEP: immune to magic, resist +100% |
| `PROMOTION_MAGIC_RESISTANCE` | iResistMagic +20 | KEEP: resist +20% |
| `PROMOTION_METAMAGIC1` | iCasterResistModify -5; iSpellDamageModify +5 | KEEP: its targets resist -5%, spell damage +5 |
| `PROMOTION_METAMAGIC2` | iCasterResistModify -5; iSpellDamageModify +5 | KEEP: its targets resist -5%, spell damage +5 |
| `PROMOTION_METAMAGIC3` | iCasterResistModify -5; iSpellDamageModify +5 | KEEP: its targets resist -5%, spell damage +5 |
| `PROMOTION_POTENCY` | iCasterResistModify -10 | KEEP: its targets resist -10% |
| `PROMOTION_PUPPET` | iCasterResistModify +20; iSpellDamageModify -10 | KEEP: its targets resist +20%, spell damage -10 |
| `PROMOTION_TWINCAST` | bTwincast | KEEP: summons twice (second one within SUMMON_EXTRA_SLOTS) |
| `PROMOTION_UNHOLY_TAINT` | iSpellDamageModify +10 | KEEP: spell damage +10 |
| `PROMOTION_VALOR` | iResistMagic +10 | KEEP: resist +10% |

## 31. Summon perks and random grants

**Today.** `PromotionSummonPerk` gives the caster's summons a promotion (Combat I-V: Empower I-V,
+10% each; Summoner: Combat I; Illusionist and Illusion: Illusion; Sundered: Stigmata; Puppet:
Puppet; Hidden Nationality: Hidden Nationality). `PromotionRandomApply`: Crazed (Lunatic) has 3% a
turn to become Enraged.

**Proposal.** KEEP: they act through the promotion they grant, and pre-battle summons get them too.
(Extension I and II grant Mobility, which has no combat effect; they are in the appendix.)

| Promotion | XML values | Under engagements |
|---|---|---|
| `PROMOTION_COMBAT1` | PromotionSummonPerk PROMOTION_EMPOWER1 | KEEP: the caster's summons get PROMOTION_EMPOWER1, pre-battle summons included |
| `PROMOTION_COMBAT2` | PromotionSummonPerk PROMOTION_EMPOWER2 | KEEP: the caster's summons get PROMOTION_EMPOWER2, pre-battle summons included |
| `PROMOTION_COMBAT3` | PromotionSummonPerk PROMOTION_EMPOWER3 | KEEP: the caster's summons get PROMOTION_EMPOWER3, pre-battle summons included |
| `PROMOTION_COMBAT4` | PromotionSummonPerk PROMOTION_EMPOWER4 | KEEP: the caster's summons get PROMOTION_EMPOWER4, pre-battle summons included |
| `PROMOTION_COMBAT5` | PromotionSummonPerk PROMOTION_EMPOWER5 | KEEP: the caster's summons get PROMOTION_EMPOWER5, pre-battle summons included |
| `PROMOTION_CRAZED` | PromotionRandomApply PROMOTION_ENRAGED | KEEP: 3% a turn to gain PROMOTION_ENRAGED |
| `PROMOTION_HIDDEN_NATIONALITY` | PromotionSummonPerk PROMOTION_HIDDEN_NATIONALITY | KEEP: the caster's summons get PROMOTION_HIDDEN_NATIONALITY, pre-battle summons included |
| `PROMOTION_ILLUSION` | PromotionSummonPerk PROMOTION_ILLUSION | KEEP: the caster's summons get PROMOTION_ILLUSION, pre-battle summons included |
| `PROMOTION_ILLUSIONIST` | PromotionSummonPerk PROMOTION_ILLUSION | KEEP: the caster's summons get PROMOTION_ILLUSION, pre-battle summons included |
| `PROMOTION_PUPPET` | PromotionSummonPerk PROMOTION_PUPPET | KEEP: the caster's summons get PROMOTION_PUPPET, pre-battle summons included |
| `PROMOTION_SUMMONER` | PromotionSummonPerk PROMOTION_COMBAT1 | KEEP: the caster's summons get PROMOTION_COMBAT1, pre-battle summons included |
| `PROMOTION_SUNDERED` | PromotionSummonPerk PROMOTION_STIGMATA | KEEP: the caster's summons get PROMOTION_STIGMATA, pre-battle summons included |

---

## U1. Separate defense strength

**Today.** FfH's `iCombatDefense` is the strength used to defend (`unit_base_combat_str_defense`).
281 unit types set it; 52 differ from their attack: archers defend higher (Archer 3/5, Crossbowman
9/13, Longbowman 5/6), as do Maros 6/12, Royal Guard 4/8, War Tortoise 11/19, Minotaur 8/11, Tar Demon
2/7; shock units lower (Berserker line 11/5, Knight 11/7, Seraph 8/2, Balor 7/3, Death Knight 8/4,
Catapult 4/3, assassins 5/3).

**Proposal.** KEEP: the spec makes it the base of Dv, before the role multiplier.

## U2. No defensive bonus

**Today.** 56 unit types (mounted, siege, animals, Lunatic, giants) get no plot defense and cannot
fortify; their own terrain, feature and city defense modifiers still count.

**Proposal.** KEEP: their Dv takes no plot defense or fortification.

## U3. Missiles and exploders

**Today.** Fireball (Fire II spell; 0 + 4 fire, collateral 50/30/4, first-strike immune, flying) and
Meteor (dragons' Breathe Fire; 0 + 6 fire, collateral 100/60/4, bombard 15) are one-turn summons that
attack and die (`bSuicide`, `bNoCapture`, `bNoWarWeariness`); FfH's AI casts them in its
"summon-suicide" mode. `bExplodeInCombat` (Pyre Zombie, Azer, Fireball, Meteor) is graphics only;
the Pyre Zombie's and Azer's real explosion is their Python on losing (`postCombatExplode`): 10 fire,
up to 100%, to every unit on the eight plots around, and smoke on forests.

**Proposal (Q11).** REMAP the two missiles: their spells are opening-phase effects that resolve the
missile's strike (one hit on a target picked by its role, plus its collateral) and remove it,
without taking a slot. Committing a strength-0 suicide unit to a force, or making it fight 40 rounds,
fits neither. The death explosion: KEEP (post-battle, it hits both sides around the dead unit).

## U4. Units created or converted by combat

**Today** (`unit_combat_won`, on a win). `UnitCreateFromCombat`: Air Elemental -> Lightning Elemental
100%, Ars -> Wraith 100%, Duin -> Ravenous Werewolf 75%, Greater Werewolf -> Ravenous Werewolf 45%,
Werewolf -> Ravenous Werewolf 30%, Ravenous Werewolf -> another 15%, Wolf Pack -> Wolf 10%, Giant
Spider -> Baby Spider 5%. `UnitConvertFromCombat` (the winner becomes): Ravenous Werewolf ->
Werewolf 100%, Lion -> Lion Pride 10%, Wolf -> Wolf Pack 10%, Werewolf -> Greater Werewolf 5%, Baby
Spider -> Giant Spider 2%. `PromotionFromCombat`: Losha Valas -> Immortal. Enslavement and duration
from combat: sections 22 and 23.

**Proposal.** REMAP: rolled per kill credit, applied after the engagement; created units appear on
the winner's plot, outside the engagement.

## U5. Python combat callbacks and death hooks

`PythonPostCombatWon` / `PythonPostCombatLost` run on the winner and the loser of a duel
(`crates/ffh-core/src/ffh/spells_a.rs`). **Proposal: KEEP all**, run per kill pair (the dead unit and
the unit with kill credit) in the post-battle step; units they create appear after the engagement.

| Unit | Callback | Effect |
|---|---|---|
| Angel of Death | won: `postCombatReducePopulation` | a city on the victim's plot loses 1 population |
| Auric Ascended | won: `postCombatAuricAscendedWon` | dies anyway if the victim carried the Godslayer |
| Beast of Agares | won: `postCombatConsumePaladin` | heals fully after killing a Paladin |
| Mimic | won: `postCombatMimic` | copies one random promotion of the victim |
| Donal | won: `postCombatDonal` | gains Recruiter after killing a demon or undead |
| Ira | won: `postCombatIra` | +1 strength per living kill (base set to base + typed - 3) while below 32 |
| Lightning Elemental | won: `postCombatHeal50` | halves its damage |
| Repentant Angel | won: `postCombatReduceCombat1` | -1 strength per living kill (base set to base + typed - 5) while above 5 |
| Goblin, Scorpion Clan Goblin | won: `postCombatWolfRider` | becomes a Wolf Rider after killing a wolf |
| Pyre Zombie, Azer | lost: `postCombatExplode` | explodes (U3) |
| Tar Demon, Water Elemental | lost: `postCombatSplit` | unless Weak, splits into two copies at 25 damage (Weak ones unless it was Strong) |
| Treant | lost: `postCombatTreant` | plants a new forest on its plot |
| Auric Ascended, Basium, Hyborem | lost | the owner loses its traits (Hyborem keeps Fallow) |
| Brigit (held) | lost: `postCombatBrigitHeld` | frees the Ring of Carcer |
| Sailor's Dirge | lost | starts the "Sailor's Dirge defeated" event for the victor |
| Acheron, Ars, Buboes, Leviathan, Orthus, Stephanos, Yersinia | lost | trophies for a human victor (UI only) |

Death hooks (`ev_unit_killed`, any death, combat or not): Soul Forge production, Mokka's Cauldron
demon, Manes for Esus, Veil and Overlords units and Death I / Entropy I holders, angels for the good
religions and the Mercurians, Spirit Guide's XP hand-off. KEEP: they fire for engagement deaths.

## U6. Civ4 fields FfH never sets

`FlankingStrikes`, `UnitClassTargets`, `UnitCombatTargets`, `UnitClassDefenders`,
`UnitCombatDefenders`, `UnitClassDefenseMods`, `DomainMods` (units and promotions),
`UnitCombatCollateralImmunes`, `iHillsAttack` (units and promotions), `iKamikazePercent`,
`bAlwaysHostile`, `bLineOfSight`, `iAirCombat` / `iBombRate` on land units: no FfH data uses them.

**Proposal.** REMOVE from the engagement resolver (nothing to port); the fields can stay in the
loader until a cleanup. Targeting priority, if XML is wanted for it, belongs in CombatRoleInfos.

## U7. Rule-level modifiers (not promotions, for completeness)

- Handicap barbarian bonus (`iBarbarianBonus` -30 / -20 / -10 on the three easiest levels against
  humans, `iAIBarbarianBonus` -25 against AIs; animal bonuses all 0): KEEP as A and Dv %.
- Free wins against barbarians (odds capped for the first `iFreeWinsVsBarbs` kills): REMOVE, it is 0
  on every level.
- CITY_BARBARIAN_DEFENSE_MODIFIER +25 against barbarian attackers: KEEP in Dv.
- War weariness and the AI's war success per unit killed (`WW_*`, `WAR_SUCCESS_*`; none for
  `bNoWarWeariness` units: Fireball, Meteor, both Mercenaries, and for summons and hidden
  nationality): KEEP, per casualty.
- River and amphibious modifiers: section 13.

---

## Spells: proposed `iCombatPhase`

The spec: pre-battle (each support unit may cast one buff, debuff or summon; summons join up to
SUMMON_EXTRA_SLOTS beyond S), opening (damage spells resolve with the volleys), post-battle (heals).
"V1 converts a whitelist: area damage to opening, buffs and summons to pre-battle, heals to
post-battle. Casting in battle uses up the unit's spell for the turn."

**How an in-battle cast applies.** A buff applies to the caster's side's committed units that
qualify (the spell's own unit-combat and alive filters), a debuff to the enemy's committed units
(resist rolls as on the map). Area damage cast by the attacking side hits every unit on the
defended plot that its flags allow, committed or not (the spec's point about piling onto one plot);
cast by the defending side, it hits the committed attackers. Range and the map side effects (snow,
fire, forests) stay as written. Only committed units cast. Two things are open (**Q10**): the
spec gives the pre-battle cast to support units, but the whitelist holds abilities of other units
(the dragons' Breathe Fire and Roar, the Flagbearer's Morale, the Myconid's Spores, the Moroi's
Burning Blood, Chalid's Pillar of Fire, Mary's Contagion); and whether the defending side casts at
all. `ffh-ai`'s `magic::phase` already sorts spells into Soften / Summon / Bless before a
strike, which matches opening / pre-battle summons / pre-battle buffs below.

### Opening (14)

| Spell | Who | Effect today |
|---|---|---|
| `SPELL_MAELSTROM` | Air II | 15 lightning, to 30%, range 2 |
| `SPELL_RING_OF_FLAMES` | Ashen Veil divine | 15 fire, to 40%, range 1; sets forests alight |
| `SPELL_SNOWFALL` | Ice III | 40 cold, to 80%, range 1; snow |
| `SPELL_SNOWFALL_GREATOR` | Auric Ascended | 80 cold, to 100%, range 2 |
| `SPELL_WITHER` | Entropy III | 10 death, to 40%, plus Withered |
| `SPELL_CONTAGION` | Mary Morbus | 10 death, to 40%, plus Plagued |
| `SPELL_CROWN_OF_BRILLANCE` | Empyrean divine | 30 holy, to 60%, range 1 (also recast each turn by its promotion) |
| `SPELL_DESTROY_UNDEAD` | Life II | 30 holy to undead, can kill |
| `SPELL_PILLAR_OF_FIRE` | Chalid | fire to 75% on the strongest enemy-only plot within 2 |
| `SPELL_CRUSH` | Dwarven Druid | physical to 75% on the most crowded enemy-only plot within 2 |
| `SPELL_CALL_OF_THE_GRAVE` | Ars | death to 100% within 2; heals undead; raises Wraiths from graveyards |
| `SPELL_TSUNAMI` | Overlords divine | 30 cold, to 75%, near water, range 2 |
| `SPELL_FIREBALL` | Fire II | the Fireball's strike and collateral (U3, Q11) |
| `SPELL_BREATH_FIRE` | dragons | the Meteor's strike and collateral (U3, Q11) |

For an in-battle cast, Pillar of Fire and Crush target the defended plot instead of choosing one.

### Pre-battle: buffs (15)

| Spell | Who | Effect |
|---|---|---|
| `SPELL_BLESS` | Order divine | Blessed: +1 holy, +20% against demons, gone after the fight |
| `SPELL_BLUR` | Shadow I | immune to first strikes (opening volleys) and defensive strikes |
| `SPELL_COURAGE` | Spirit I | immune to fear (matters with Q8) |
| `SPELL_DANCE_OF_BLADES` | Chaos I | +1 first strike = +1 opening volley |
| `SPELL_ENCHANTED_BLADE` | Enchantment I | melee +20% |
| `SPELL_FLAMING_ARROWS` | Enchantment II | archers +1 fire |
| `SPELL_LOYALTY` | Law I | immune to capture |
| `SPELL_MORALE` | Flagbearer | +10%, +10% city attack, range 1 |
| `SPELL_POISONED_BLADE` | Nature II | recon +1 poison |
| `SPELL_SHADOWWALK` | Shadow II | ignore building defense, immune to defensive strikes |
| `SPELL_SHIELD_OF_FAITH` | Kilmorph divine | A +10%, Dv +30% under Q1 (+10% today) |
| `SPELL_STONESKIN` | Earth II | caster: +2 Dv, +3 volleys, 50% resistances, gone after the fight |
| `SPELL_VALOR` | Law III | +1 XP per kill, +10% magic resistance |
| `SPELL_TREETOP_DEFENCE` | Nature I | full fortification in a forest (a defender's spell) |
| `SPELL_BURNING_BLOOD` | Moroi | self: +20%, Blitz (not cast by the AI today) |

### Pre-battle: debuffs (8)

| Spell | Who | Effect in battle |
|---|---|---|
| `SPELL_RUST` | Entropy I | enemy melee -10%, enemy weapons stripped |
| `SPELL_DISPEL_MAGIC` | Metamagic II | strips enemy blessings (Enchanted Blade, Shield of Faith, Stoneskin...) |
| `SPELL_REVELATION` | Empyrean divine | dispels, unmasks invisible units, destroys illusions |
| `SPELL_CHARM_PERSON` | Mind II | charmed attackers cannot attack: they leave the attacking force |
| `SPELL_BLINDING_LIGHT` | Sun II | immobile: attackers leave the force; immobile defenders cannot withdraw |
| `SPELL_ENTANGLE` | Druid | as Blinding Light (not flying units) |
| `SPELL_SPORES` | Myconid | as Blinding Light (3 turns) |
| `SPELL_ROAR` | dragons | today's fear roll: failures flee, and leave the engagement |

Charm Person, Blinding Light, Entangle and Spores only bite in battle when cast by the defender
(Q10): cast by the attacker they do what they do on the map.

### Pre-battle: summons (17)

Rule: a summon spell is pre-battle when its unit is temporary and can fight. `SPELL_SUMMON_AIR_ELEMENTAL`,
`SPELL_SUMMON_AUREALIS`, `SPELL_SUMMON_BALOR`, `SPELL_SUMMON_DJINN`, `SPELL_SUMMON_EARTH_ELEMENTAL`,
`SPELL_SUMMON_FIRE_ELEMENTAL`, `SPELL_SUMMON_EINHERJAR`, `SPELL_SUMMON_ICE_ELEMENTAL`,
`SPELL_SUMMON_MISTFORM`, `SPELL_SUMMON_PIT_BEAST`, `SPELL_SUMMON_SAND_LION`, `SPELL_SUMMON_SPECTRE`,
`SPELL_SUMMON_WATER_ELEMENTAL`, `SPELL_SUMMON_WRAITH`, `SPELL_CREATE_PUPPET`, `SPELL_CREATE_PUPPET_LOKI`,
`SPELL_MIRROR` (an Illusion copy of the caster).
They join their side's force within SUMMON_EXTRA_SLOTS and take roles like any unit (summons are
frontline by the spec).

### Post-battle (6)

| Spell | Who | Effect |
|---|---|---|
| `SPELL_HEAL` | Medic III | 20 to every living unit on the caster's plot, removes Poisoned |
| `SPELL_CURE_DISEASE` | Medic II | removes Diseased, Plagued, Withered on the plot |
| `SPELL_REPAIR` | Dwarven Enchantment I | 20 to golems, ships and siege on the plot |
| `SPELL_SIRONAS_TOUCH` | Shrine of Sirona | the caster heals 15 (does not use its cast) |
| `SPELL_HEALING_SALVE` | salve carrier | heals fully, removes afflictions |
| `SPELL_FEED` | vampires | heals 20, kills a Bloodpet in the stack |

### Phase none (207)

- World spells (21): `SPELL_ARCANE_LACUNA`, `SPELL_ARDOR`, `SPELL_DIVINE_RETRIBUTION`,
  `SPELL_FOR_THE_HORDE`, `SPELL_GIFTS_OF_NANTOSUELTA`, `SPELL_HYBOREMS_WHISPER`, `SPELL_INTO_THE_MIST`,
  `SPELL_LEGENDS`, `SPELL_MARCH_OF_THE_TREES`, `SPELL_MOTHER_LODE`, `SPELL_RAGING_SEAS`, `SPELL_RALLY`,
  `SPELL_RELIGIOUS_FERVOR`, `SPELL_REVELRY`, `SPELL_RIVER_OF_BLOOD`, `SPELL_SANCTUARY`, `SPELL_STASIS`,
  `SPELL_VEIL_OF_NIGHT`, `SPELL_WARCRY`, `SPELL_WILD_HUNT`, `SPELL_WORLDBREAK`: once a game, global.
- Held back for v2 (7): `SPELL_DOMINATION` (steals a unit: side changes mid-battle),
  `SPELL_MEZMERIZE_ANIMAL` (likewise), `SPELL_EARTHQUAKE` (unfortifies and wrecks walls: a strong
  pre-battle debuff candidate), `SPELL_HASTURS_RAZOR` (shuffles damage among both sides),
  `SPELL_TAUNT` (forces attacks), `SPELL_RAGE` (Burning Blood on everyone), `SPELL_WONDER` (random).
- Permanent or non-combat summons (8): `SPELL_RAISE_SKELETON`, `SPELL_GRAFT_FLESH`, `SPELL_SUMMON_TIGER`,
  `SPELL_SUMMON_TREANT`, `SPELL_SUMMON_KRAKEN` (naval) and `SPELL_IRA_UNLEASHED` (the barbarian
  Avatar of Wrath's permanent Ira) are army building, cast on the map; `SPELL_FLOATING_EYE` and
  `SPELL_SEVER` summon units that cannot fight.
- Terrain (9): `SPELL_BLAZE`, `SPELL_BLOOM`, `SPELL_CALL_BLIZZARD`, `SPELL_CALL_BLIZZARD_HIGH_PRIEST`,
  `SPELL_HELLFIRE`, `SPELL_SANCTIFY`, `SPELL_SCORCH`, `SPELL_SPRING`, `SPELL_VITALIZE`.
- Movement, stealth, casting and other unit utilities (21): `SPELL_CALL_FORM`,
  `SPELL_DECLARE_NATIONALITY`, `SPELL_ENTER_PORTAL`, `SPELL_ESCAPE`, `SPELL_FAIR_WINDS`, `SPELL_HASTE`,
  `SPELL_HIDE`, `SPELL_MASK`, `SPELL_POTION_OF_INVISIBILITY`, `SPELL_POTION_OF_RESTORATION`,
  `SPELL_RELEASE_FROM_CAGE`, `SPELL_REVEAL`, `SPELL_SLOW`, `SPELL_SPRINT`, `SPELL_WATER_WALKING`,
  `SPELL_WHITEOUT`, `SPELL_ENCHANT_SPELLSTAFF`, `SPELL_SPELLSTAFF`, `SPELL_FORM_WOLF_PACK`,
  `SPELL_REGENERATION` (heal rate), `SPELL_MUTATION` (permanent, random).
- Equipment take and drop (53, `SPELL_TAKE_*`, `SPELL_DROP_*`), lairs (9, `SPELL_EXPLORE_LAIR_*`), ship
  crews (4, `SPELL_CREW_*`), conversions and upgrades (12: `SPELL_UPGRADE_*`, `SPELL_DROWN`,
  `SPELL_LICHDOM`, `SPELL_WANE`), and city, economy and unit-building spells (63: temples, freak show,
  hires, commanders, conversions of cities, resurrection, Hope, Inspiration, Wall of Stone, Unyielding
  Order and the rest).

---

## Questions for the owner

1. **Q1 Defense percentages.** Apply `iCombatPercentDefense` to Dv (Enraged +20 A / -20 Dv, Shield of
   Faith +10 A / +30 Dv; add -10 and -20 for Withered and Puppet), or `iCombatPercent` to both as
   today? Recommended: the defense field.
2. **Q2 Typed damage.** Flat per-hit damage per typed point (the spec's wording; needs a new constant
   and mis-scales: Ars nearly doubles, Fireball and Meteor halve), or typed points inside A and Dv,
   scaled per target by resistance (FfH's rule)? Recommended: the latter.
3. **Q3 Withdrawal.** One roll per engagement (recommended) or every round below 40%? And do
   defenders outside cities keep FfH's defensive withdrawal (Loki 100%, Flanking promotions)?
   Recommended: yes.
4. **Q4 Collateral.** Once per engagement in the opening phase (recommended), and for every unit with
   collateral, not only siege (Berserker, Brujah, Circle of Urd, Clockwork Golem, Seraph, Fire
   Elemental, Dwarven Shadow, Lunatic)?
5. **Q5 Defensive strike.** Keep it as opening strikes from the defending plot, reserve archers
   included (recommended), or remove it?
6. **Q6 Marksman and Guardsman.** Marksman gives the Assassin targeting profile and defines the
   Assassin role's default (every FfH assassin has it free); Guardsman becomes a bodyguard that
   assassin-profile units must hit first. Agreed?
7. **Q7 Formation as Screen.** FfH has no spear line; should Formation (anti-mounted +40%) make a
   frontline unit a Screen?
8. **Q8 Fear.** A pre-battle morale check (failed attackers sit out; failed defenders do not fight)
   and a rout after a won attack outside cities, with today's rolls?
9. **Q9 River.** Does the 2-slot loss replace the +40% defender bonus (recommended), and may
   river-capable units fill the 2 lost slots?
10. **Q10 Who casts in battle.** Only committed support units (the spec) or every committed unit
    with a whitelisted spell (dragons, Flagbearer, Myconid, Moroi, heroes)? And may the defender's
    casters cast during the attacker's turn; if so, does that use up their next own-turn cast (the
    has-casted flag resets at their owner's turn) or only the current turn's?
11. **Q11 Fireball and Meteor.** Resolve their spells as opening-phase strikes (one hit plus
    collateral, no slot), instead of committing strength-0 suicide units to the force?
12. **Q12 Medic after battle.** How much: the medic's turn value (10 / 15 / 20) once, as proposed, and
    at most once per unit per turn?
13. **Q13 Great Commander.** The Commander unit joins a unit as `PROMOTION_GREAT_COMMANDER` (+1 / +1,
    +1 XP per kill). Should that unit also qualify for the army commander slot and aura (phase 7),
    which the spec reserves for heroes?

## The owner's answers (2026-10-31)

"Go with your recommendations on each." Where a question had no recommendation, the proposal in
this inventory stands; Q10 and Q13 were decided as recommended in the conversation that day. The
other KEEP / REMAP / REMOVE proposals above stand as written.

1. **Q1:** promotions' defense percentage (`iCombatPercentDefense`) applies to Dv, the attack
   percentage (`iCombatPercent`) to A; Withered -10 and Puppet -20 on Dv.
2. **Q2:** typed damage stays FfH's rule: typed points inside A and Dv, scaled per target by its
   resistance (no flat per-hit constant).
3. **Q3:** one withdrawal roll per engagement (the first round the unit is below
   `WITHDRAW_HP_PCT`); defenders outside cities keep FfH's defensive withdrawal.
4. **Q4:** collateral resolves once per engagement in the opening phase, for every unit with
   collateral damage, not only siege.
5. **Q5:** defensive strike stays, as opening strikes from the defending plot, reserve archers
   included.
6. **Q6:** Marksman gives the Assassin targeting profile and is the Assassin role's default;
   Guardsman is a bodyguard that assassin-profile units must hit first.
7. **Q7:** Formation makes a frontline unit a Screen.
8. **Q8:** Fear is a pre-battle morale check (failed attackers sit out, failed defenders do not
   fight) and a rout after a won attack outside cities, with today's rolls.
9. **Q9:** the river's 2-slot loss replaces the defender's +40% river bonus; river-capable units
   (`isRiver`) may fill the 2 lost slots.
10. **Q10:** every committed unit with a whitelisted spell may cast in battle (dragons, the
    Flagbearer, Myconid, Moroi, heroes), not only support units. Defending casters cast too, once
    between their own turns, and doing so uses up their next own-turn cast.
11. **Q11:** Fireball and Meteor resolve as opening-phase strikes (one hit plus collateral, no
    slot), not as committed strength-0 suicide units.
12. **Q12:** a medic heals its turn value (10 / 15 / 20) once after a battle, at most once per unit
    per turn.
13. **Q13:** a unit with `PROMOTION_GREAT_COMMANDER` qualifies for the army commander slot and
    aura (phase 7), like a hero.

---

## Appendix: promotions with no combat effect (82)

Grouped by why. None of them sets a field the combat code reads; where they matter to a fight it is
through a spell (Spells section) or a death hook (U5).

- **Sphere promotions (51), which unlock spells:** `PROMOTION_AIR1`, `PROMOTION_AIR2`, `PROMOTION_AIR3`,
  `PROMOTION_BODY1`, `PROMOTION_BODY2`, `PROMOTION_BODY3`, `PROMOTION_CHAOS1`, `PROMOTION_CHAOS2`,
  `PROMOTION_CHAOS3`, `PROMOTION_DEATH1`, `PROMOTION_DEATH2`, `PROMOTION_DEATH3`, `PROMOTION_EARTH1`,
  `PROMOTION_EARTH2`, `PROMOTION_EARTH3`, `PROMOTION_ENCHANTMENT1`, `PROMOTION_ENCHANTMENT2`,
  `PROMOTION_ENCHANTMENT3`, `PROMOTION_ENTROPY1`, `PROMOTION_ENTROPY2`, `PROMOTION_ENTROPY3`,
  `PROMOTION_FIRE1`, `PROMOTION_FIRE2`, `PROMOTION_FIRE3`, `PROMOTION_ICE1`, `PROMOTION_ICE2`,
  `PROMOTION_ICE3`, `PROMOTION_LAW1`, `PROMOTION_LAW2`, `PROMOTION_LAW3`, `PROMOTION_LIFE1`,
  `PROMOTION_LIFE2`, `PROMOTION_LIFE3`, `PROMOTION_MIND1`, `PROMOTION_MIND2`, `PROMOTION_MIND3`,
  `PROMOTION_NATURE1`, `PROMOTION_NATURE2`, `PROMOTION_NATURE3`, `PROMOTION_SHADOW1`,
  `PROMOTION_SHADOW2`, `PROMOTION_SHADOW3`, `PROMOTION_SPIRIT1`, `PROMOTION_SPIRIT2`,
  `PROMOTION_SPIRIT3`, `PROMOTION_SUN1`, `PROMOTION_SUN2`, `PROMOTION_SUN3`, `PROMOTION_WATER1`,
  `PROMOTION_WATER2`, `PROMOTION_WATER3`.
- **Movement and sight (9):** `PROMOTION_HASTED`, `PROMOTION_SLOW`, `PROMOTION_MOBILITY1`,
  `PROMOTION_MOBILITY2`, `PROMOTION_NAVIGATION1`, `PROMOTION_NAVIGATION2`, `PROMOTION_SENTRY`,
  `PROMOTION_SENTRY2`, `PROMOTION_STARTING_SETTLER`.
- **Ship cargo (2):** `PROMOTION_EXPANDED_HULL`, `PROMOTION_LONGSHOREMEN`.
- **Summon perks of movement only (2):** `PROMOTION_EXTENSION1`, `PROMOTION_EXTENSION2`.
- **Equipment and items without combat fields (9):** `PROMOTION_BLACK_MIRROR` (Mirror spell),
  `PROMOTION_DRAGONS_HORDE`, `PROMOTION_HEALING_SALVE` (Healing Salve spell), `PROMOTION_INFERNAL_GRIMOIRE`,
  `PROMOTION_PIECES_OF_BARNAXUS`, `PROMOTION_POTION_OF_INVISIBILITY`, `PROMOTION_POTION_OF_RESTORATION`,
  `PROMOTION_SYLIVENS_PERFECT_LYRE`, `PROMOTION_SPELLSTAFF`.
- **Markers, spell gates and map effects (9):** `PROMOTION_CENTAUR` (race; Sprint), `PROMOTION_DIVINE`
  (divine spells), `PROMOTION_INQUISITOR` (Inquisition), `PROMOTION_MUTATED` (one Mutation per unit),
  `PROMOTION_PROPHECY_MARK` (+1 Armageddon counter while held), `PROMOTION_RECRUITER` (Recruit),
  `PROMOTION_SPIRIT_GUIDE` (hands half its XP on when it dies, U5), `PROMOTION_STEALTH` (the Hide
  ability), `PROMOTION_WEREWOLF` (marker).
