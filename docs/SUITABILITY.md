# Building suitability: building tiers

**Status: built** (2026-11-05). This file is the rules as they play (sections 0-5) and how they
are built (section 7). Revision history: proposal revisions 1-7 (2026-11-05), then built with the
differences noted in section 6.

## 0. The idea

When a city is founded, it rolls a tier for every building class: **Lesser, Standard, Fine,
Grand or Legendary**. The city builds only its own tier's version of each building.

- A Fine Library is a better Library.
- A Lesser Forge is a worse Forge.
- A Legendary building is the best version, with a name of its own (the Library's is the
  *Athenaeum*) and one extra ability.

Some things never change with the tier:

- A building's cost.
- What it unlocks: units, spells, prerequisites, civic and event bonuses.
- Its drawbacks: crime, unhealthiness, unhappiness, extra maintenance.

The city's choice is what to build first, given the hand it was dealt. Wonders roll a tier too,
but it changes only their cost (section 5).

This is part of the rules in every game. There is no game option.

## 1. Tiers

| Tier | Chance | Color | Shown as | Core effect |
|---|---|---|---|---|
| Lesser | 25% | grey | "Lesser Library" | half |
| Standard | 30% | white | "Library" | FFH 0.41 |
| Fine | 25% | green | "Fine Library" | ×1.5 |
| Grand | 15% | blue | "Grand Library" | ×2, plus a Grand extra |
| Legendary | 5% | gold | "Athenaeum" | ×2, Grand extra, plus a signature |

**Names.** These replace bad/normal/good/exceptional/legendary. The tier word becomes part of
the building's name, and "Bad Walls" reads badly as a name. "Poor" is avoided because FFH already
uses it for a culture level. A Standard building keeps its plain FFH name. A Legendary building
gets a type name of its own rather than a prefix. Legendary versions are common enough (below)
that two cities can both have an Athenaeum, so these names are types, not unique proper names.
The colors follow the loot-rarity convention that players of roguelikes already read at a glance.

**The roll.** Each class is rolled once per city, at founding, independently of every other
roll.

- The roll is kept for the city's whole life. A captured, traded or flipped city keeps it, and a
  destroyed building comes back at the same tier.
- The owner sees the whole hand on the city screen from the moment the city is founded. That
  includes classes it cannot build yet ("Grand Library" waiting for Writing), so the hand can
  shape plans.
- Other players see only what has been built.

**What this means per city.** A city builds about 20 of the roughly 45 generic classes in a
typical game. Of those 20, expect about:

| Lesser | Standard | Fine | Grand | Legendary |
|---|---|---|---|---|
| 5 | 6 | 5 | 3 | 1 |

About 64% of cities have at least one Legendary among what they build. About 90% have one
somewhere in their hand. A ten-city empire ends up with roughly ten Legendary buildings. Under
this distribution, Legendary means *this city's star building*, not a once-a-game event.

**The economy gets richer, and that is fine.** The average tier is worth about 1.2 times a
Standard building: 0.25×½ + 0.30×1 + 0.25×1.5 + 0.15×2 + 0.05×2, before counting the signatures.
That is about +20% building output before the player chooses anything, and more once players
build their good tiers first and skip their bad ones.

I recommend keeping these tier sizes. If games turn out to move too fast, the knob is the global
tech cost (one number), not the tiers. Shrinking the tiers would undercut the point of the
system, and integer rounding of small values (+2 culture becomes +3 whether the multiplier is
×1.25 or ×1.5) makes small tiers hard to tell apart anyway.

## 2. General rules

These rules generate the tables in section 3. A table entry overrides them where it says
otherwise.

1. **Core effects scale.** Every number that is the building's reason to exist scales by the
   tier: yields, commerce, percentage modifiers, happiness, health, defense, food kept, healing,
   experience, and maintenance or war-weariness reductions.
   - Lesser halves, rounding down. A flat +1 stays +1, and the Lesser version loses its
     secondary effect instead.
   - Fine multiplies by 1.5, rounding up: 25% becomes 38%, +3 becomes +5.
   - Grand and Legendary double.
2. **Specialist slots.** Lesser, Standard and Fine have FFH's slots. Grand and Legendary add one
   more slot of the same kind. Some Legendary buildings also come with a *free* specialist: one
   that works without using a citizen.
3. **Drawbacks never scale.** The Market's −1 research, the Forge's −1 health, the Tax Office's
   −1 happiness, crime, and extra maintenance are the same at every tier. A good tier never makes
   a drawback worse, and a bad tier never adds one.
4. **Unlocks never scale.** Every tier unlocks the same units and spells and counts for the same
   prerequisites, civics and events. A Lesser Stable still lets the city train Horsemen.
5. **Unlocking buildings act through their units.** The Training Yard, Archery Range, Stable,
   Siege Workshop, Mage Guild, Hunting Lodge, Grove and the religious temples all unlock units,
   and the tier also acts on the units they unlock *when those units are trained in this city*:

   | Tier | Units they unlock, trained here |
   |---|---|
   | Lesser | −25% production |
   | Fine | +2 XP |
   | Grand | +4 XP |
   | Legendary | +4 XP and a named promotion |

   +2 XP is one promotion. +4 XP is one promotion and most of the way to a second.

   Lesser works through production because these buildings have nothing else to halve. A Lesser
   Training Yard would otherwise be the same as a Standard one. A −25% production effect is in
   FFH's own vocabulary: it is the Shipyard's +25% turned around. It acts on the units, not on the
   building's cost. This gives cities military specialties: archers come from the city with the
   Grand Archery Range.
6. **Resource-based effects** (+1 health with Fish, +1 happiness with Fur) stay +1 up to Fine and
   become +2 at Grand and Legendary.
7. **Trade routes** are counts. Lesser keeps the route unless the table says otherwise, Fine keeps
   it, and Grand and Legendary add one.
8. **Food kept from all buildings together is capped at 60%.** Without the cap, a Legendary
   Granary and a Legendary Smokehouse would keep 80% of the food, and a city would regrow almost
   at once after every growth. 60% is already FFH's own maximum, reached by the Calabim with
   their Breeding Pit.
9. **Legendary = Grand + signature.** The signature is one ability on top of the Grand version.
   Most signatures use effects FFH buildings already have (free specialists, free promotions,
   empire-wide or continent-wide modifiers, war-weariness and health switches). The few that
   would be new rules are listed in section 6.
10. **Extras push the same way as the building.** A Grand extra or a Legendary signature makes
    the building better at what it already does: a culture building's signature is a big culture
    effect, a health building's a big health effect, and a building that trains units gives those
    units more. A signature is never an unrelated stat added on, and never just the removal of a
    drawback. A good Monument is a better source of culture, not a source of happiness.

## 3. The buildings

The *Standard* column of each table is FFH 0.41's building, from `CIV4BuildingInfos.xml`.

### 3.1 Knowledge and magic

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Elder Council**: +2 research; 1 scientist slot | +1 research | +3 research | +4 research; 2 slots | **Council of Sages**: Grand + 1 free scientist |
| **Library**: +25% research; 1 scientist slot | +12% | +38% | +50%; 2 slots | **Athenaeum**: Grand + 1 free scientist |
| **Alchemy Lab**: +4 research, +10% research; 1 scientist slot | +2, +5% | +6, +15% | +8, +20%; 2 slots | **Athanor**: Grand + 5% research in all your cities |
| **Mage Guild**: unlocks Adepts, Imps | arcane units −25% production | arcane +2 XP | arcane +4 XP | **Arcanum**: arcane +4 XP and Potency |

### 3.2 Commerce and trade

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Market**: +3 gold, −1 research; 1 merchant slot; crime +5 | +1 gold | +5 gold | +6 gold; 2 slots | **Emporium**: Grand + 1 free merchant, +1 trade route |
| **Money Changer**: +25% gold; 1 merchant slot | +12% | +38% | +50%; 2 slots | **Counting House**: Grand + 5% gold in all your cities |
| **Tax Office**: +25% gold; −1 happiness | +12% | +38% | +50% | **Exchequer**: Grand + 1 gold per 2 citizens |
| **Gambling House**: +3 gold; +1 happiness per 10% of the gold rate; crime +10; +10% maintenance | +1 gold; +1 happiness per 20% | +5 gold; +3 happiness per 20% | +6 gold; +1 happiness per 5% | **House of Fortune**: Grand + 25% gold |
| **Smugglers' Port** (coast): +25% gold; +1 trade route; 1 merchant slot; crime +10 | +12% | +38% | +50%; 2 trade routes; 2 slots | **Black Wharf**: Grand + 1 free merchant, +50% trade-route yield |
| **Inn**: +25% trade-route yield; +1 trade route; healing +25; +1 happiness with Wine; crime +5 | +12%; healing +12 | +38%; healing +38 | +50%; 2 trade routes; healing +50; +2 with Wine | **Caravanserai**: Grand + 1 trade route (3 in all), +25% more trade-route yield (+75%) |
| **Tavern**: +25% trade-route yield; +1 trade route; +1 great artist point; crime +5; +10% maintenance | +12% | +38% | +50%; 2 trade routes; +2 points | **Mead Hall**: Grand + 1 trade route (3 in all), +2 more great artist points (+4) |
| **Obsidian Gate**: airlift 1; +1 trade route | airlift 1; no trade route | airlift 2 | airlift 2; 2 trade routes | **Gate of Night**: airlift 3; 2 trade routes; +25% trade-route yield |

### 3.3 Sea

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Harbor**: +50% trade-route yield; +1 health per Clam, Crab, Fish | +25% | +75% | +100%; +2 health per seafood | **Deepwater Quay**: Grand + 1 trade route, +1 food per Clam, Crab, Fish |
| **Lighthouse**: +1 food on water plots; +1 trade route | no trade route | 2 trade routes | 2 trade routes; +1 food per Clam, Crab, Fish | **Beacon Tower**: Grand, and water plots get +2 food instead of +1 |
| **Shipyard**: naval units +4 XP; +25% naval production | +2 XP; +12% | +6 XP; +38% | +8 XP; +50% | **Admiralty**: Grand + naval units trained here get Expanded Hull |

The Lighthouse's +1 food per water plot stays +1 up to Grand: +2 food on every water plot is
too strong to come with a 25% Fine roll. It is the Legendary signature instead, which makes a
Legendary Lighthouse a fishing metropolis.

### 3.4 Production

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Forge**: +25% production; 1 engineer slot; +1 happiness with Gold; −1 health; removes Rusted | +12% | +38% | +50%; 2 slots | **Foundry**: Grand + 1 free engineer |

### 3.5 Culture and happiness

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Monument**: +2 culture | +1 | +3 | +4 | **Standing Stones**: Grand + 1 free artist |
| **Theatre**: +3 culture; +1 happiness per 10% of the culture rate; 1 artist slot; +1 happiness with Dye | +1 culture; +1 happiness per 20% | +5; +3 happiness per 20% | +6; +1 happiness per 5%; 2 slots | **Amphitheatre**: Grand + 1 free artist |
| **Carnival**: +1 happiness; +20% culture; 1 artist slot | +1; +10% | +2; +30% | +2; +40%; 2 slots | **Midsummer Fair**: Grand, with +3 happiness; −50% war weariness |
| **Public Baths**: +3 happiness; −1 health; crime +5 | +1 | +5 | +6 | **Thermae**: Grand + 1 happiness in all your cities on this continent |

### 3.6 Growth and health

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Granary**: 20% food kept; +1 health per Corn, Rice, Wheat | 10% | 30% | 40%; +2 per grain | **Horreum**: Grand + 10% food |
| **Smokehouse**: 20% food kept; +1 health per Cow, Pig, Sheep, Toad | 10% | 30% | 40%; +2 per livestock | **Larder Hall**: Grand + 1 food per Cow, Pig, Sheep, Toad |
| **Herbalist**: +1 health; healing +10; +1 health with Reagents | +1 health; no healing | +2; healing +15 | +2; healing +20; +2 with Reagents | **Physic Garden**: Grand, with +3 health; +1 health in all your cities on this continent |
| **Aqueduct**: +2 health | +1 | +3 | +4 | **Great Cistern**: Grand, and the city has no unhealthiness from buildings |
| **Infirmary**: +3 health; healing +25; unlocks Grigori Medic | +1; healing +12 | +5; healing +38 | +6; healing +50 | **House of Mercy**: Grand, and units in the city heal fully each turn |

The food-kept cap of 60% (rule 8) applies to the Granary and Smokehouse together.

### 3.7 Order and war

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Courthouse**: −40% maintenance; crime −10 | −20%; −5 | −60%; −15 | −80%; −20 | **High Court**: no maintenance in this city; crime −20 |
| **Dungeon**: −25% war weariness; crime −10 | −12%; −5 | −38%; −15 | −50%; −20 | **Oubliette**: no war weariness in this city; crime −20 |
| **Command Post** (Organized leaders): +20% military production; land units +2 XP; +1 great commander point | +10%; +1 XP | +30%; +3 XP | +40%; +4 XP; +2 points | **War Room**: Grand + land units trained here get Drill I |

### 3.8 Defense

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Palisade**: +10% defense; 2 Sentinels | +5%; 1 Sentinel | +15%; 2 | +20%; 3 | **Thornwall**: Grand, and its Sentinels repair twice as fast |
| **Walls**: +25% defense; +25% bombard defense; 2 Sentinels | +12%; +12%; 1 Sentinel | +38%; +38%; 2 | +50%; +50%; 3 | **Bastion**: +50%; +50%; 4 Sentinels |

Sentinels are this game's siege mechanic (docs/SIEGES.md). A Sentinel count does not halve
cleanly, so Lesser walls have 1 Sentinel and the extra Sentinels come at Grand and Legendary.

### 3.9 Military buildings that unlock units

These follow rule 5. The table gives each building's own effects at each tier and its Legendary
promotion.

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Training Yard**: unlocks melee units | melee −25% production | melee +2 XP | melee +4 XP | **Proving Ground**: melee +4 XP and Combat I |
| **Archery Range**: +10% defense; unlocks archers | +5%; archers −25% production | +15%; archers +2 XP | +20%; archers +4 XP | **Bowyers' Yard**: Grand + archers trained here get City Garrison I |
| **Stable**: unlocks mounted units | mounted −25% production | mounted +2 XP | mounted +4 XP | **Royal Stables**: mounted +4 XP and Mobility I |
| **Siege Workshop**: unlocks siege units and chariots | −25% production | +2 XP | +4 XP | **Engine Works**: +4 XP; siege units also get Barrage I |
| **Hunting Lodge**: +1 health with Deer; +1 happiness with Fur; unlocks recon units | recon −25% production | recon +2 XP | recon +4 XP; +2 with Deer and Fur | **Huntsman's Hall**: Grand + recon units trained here get Sentry (+1 vision) |
| **Grove**: unlocks Druids, Beastmasters, Myconids | those −25% production | +2 XP | +4 XP | **Heartwood**: +4 XP and Woodsman I |

### 3.10 Religion

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Pagan Temple**: +10% culture; 1 priest slot; +1 happiness with Incense | +5% | +15% | +20%; 2 slots | **Stone Circle**: Grand + 1 free priest |

**The six religions' temples** share one pattern. Each Standard temple has +20% culture, two
specialist slots and the extras below, and unlocks that religion's disciples and priests.

| Tier | Culture | Extras | Slots | The religion's units, trained here |
|---|---|---|---|---|
| Lesser | +10% | halved | 2 | −25% production |
| Fine | +30% | ×1.5 | 2 | +2 XP |
| Grand | +40% | ×2 | 3 (an extra priest) | +4 XP |
| Legendary | +40% | ×2 | 3 | +4 XP; the city spreads its religion as a holy shrine does |

| Temple (extras) | Lesser | Fine | Grand | Legendary name |
|---|---|---|---|---|
| Kilmorph (+2 gold) | +1 | +3 | +4 | Deep Hall of Kilmorph |
| Leaves (+1 health) | +1 | +2 | +2 | Heart-Tree |
| Empyrean (+10% research) | +5% | +15% | +20% | Sun Spire |
| The Order (+10% military production, crime −5) | +5%, −2 | +15%, −8 | +20%, −10 | Cathedral of the Order |
| Overlords (+3 culture) | +1 | +5 | +6 | Drowned Sanctum |
| Ashen Veil (+2 research) | +1 | +3 | +4 | Black Cathedral |

The three buildings that only one religion can build:

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Basilica** (Order): −40% maintenance; crime −10; +1 great commander point | −20%; −5 | −60%; −15 | −80%; −20; +2 points | **Basilica of Junil**: Grand, and no maintenance in this city |
| **Asylum** (Overlords): +15% research; 1 scientist slot; +1 great prophet point; −25% war weariness; −1 happiness; unlocks Lunatics; 10% of units trained here go Crazed | +7%; −12% | +23%; −38% | +30%; −50%; +2 points; 2 slots | **Bedlam**: Grand + 1 free scientist |
| **Demons' Altar** (Veil): units trained here get Scourge; −1 happiness; enables Sacrifice the Weak | no Scourge | Scourge; units +2 XP | Scourge; units +4 XP | **Altar of Agares**: Grand + units trained here get Fear |

## 4. Unique buildings

**Replacements for a generic building** take that class's roll: the Governor's Manor
(Courthouse), Dwarven Smithy (Forge), Sea Haven (Harbor), Grigori Tavern, Arena and Sculptor's
Studio (Training Yard), and Desert Shrine and Temple of the Hand (Pagan Temple).

- Numbers they share with the generic building scale as in its row.
- Their own extras scale by the general rules. For example, the Desert Shrine's +2 disciple XP
  becomes +1, +3, +4 and +4.
- A Legendary one keeps its own name with the prefix "Legendary" and gets the class's signature.

**The civilization buildings** are covered in section 4.1.

**Not rolled at all:** cages, vaults, the Dancing Bear, and other buildings that can't be built
normally.

### 4.1 Civilization buildings

These are the buildings in the three civilization slots (`CIV_BUILDING1-3`). They roll like
every other class. Many of their effects are triggered abilities rather than flat numbers: a
chance, a free copy, a granted promotion, fire damage. So one more general rule applies to them:

11. **Chances, durations and granted promotions.**
    - A chance, a duration, an amount of damage or a count scales like any other number: Lesser
      halves it, Fine multiplies it by 1.5, Grand doubles it.
    - A promotion that every unit trained there already receives can't go above 100%. So:
      Lesser gives it only half the time (a 50% chance per unit), Fine adds +2 XP to the units
      that get it, Grand adds +4 XP, and Legendary adds +4 XP and a second promotion.

The *Standard* column below is FFH 0.41, from `CIV4BuildingInfos.xml` and the effects in the
rules code (`ffh/events.rs`).

| Civ | Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|---|
| Amurites | **Cave of Ancestors**: arcane units trained here get +1 XP per mana resource the city has | +1 XP per 2 mana | +3 XP per 2 mana | +2 XP per mana | **Ancestral Vault**: +3 XP per mana |
| Balseraphs | **Freak Show** (made by a Freak, not built): +2 culture; +1 happiness; +1 great artist point | +1 culture; +1 happiness | +3 culture; +2 happiness; +2 points | +4 culture; +2 happiness; +2 points | **Circus of Wonders**: Grand + 1 free artist |
| Balseraphs | **Hall of Mirrors** (needs a Carnival): +1 happiness; each turn, an illusion copy of an enemy unit next to the city appears in it for 3 turns | copies last 2 turns | 5 turns | 6 turns | **House of a Thousand Faces**: Grand, and it copies enemies up to 2 plots away |
| Calabim | **Breeding Pit**: +2 food; 20% food kept; −1 health | +1 food; 10% | +3 food; 30% | +4 food; 40% | **Feeding Halls**: Grand + 10% food |
| Clan of Embers | **Warrens**: every unit trained here gets a free copy (not world, national, siege or naval units) | copy; units it copies −25% production here (1.5× units on net) | copy; both units +2 XP | copy; both units +4 XP | **Deep Warrens**: Grand, and siege units are copied too |
| Elohim | **Chancel of Guardians**: +25% defense; units trained here have a 20% chance of Defensive | +12%; 10% | +38%; 30% | +50%; 40% | **Sanctum of Guardians**: +50% defense; units trained here always get Defensive |
| Elohim | **Reliquary**: +2 culture; +1 great prophet point; units trained here get Spirit Guide (on death, half their XP goes to another unit); −1 health | +1 culture; Spirit Guide half the time | +3 culture; +2 points; Spirit Guide; units +2 XP | +4 culture; +2 points; Spirit Guide; units +4 XP | **Ossuary of Saints**: Grand, and Spirit Guide passes on all the unit's XP instead of half |
| Grigori | **Adventurers' Guild**: +2 Adventurer points; +25% great people rate; land units +2 XP | +1 point; +12%; +1 XP | +3 points; +38%; +3 XP | +4 points; +50%; +4 XP | **Free Companies' Hall**: Grand, and Adventurers born in this city start with +10 XP |
| Kuriotates | **Tailor**: a free Fine Clothes resource; +1 happiness and +10% gold for each of Dye, Cotton, Silk | Fine Clothes; +1 happiness and +5% gold each | +15% gold each | +2 happiness and +20% gold each | **House of Silks**: Grand, with 2 Fine Clothes resources (one to trade) |
| Kuriotates | **Jeweler**: a free Jewels resource; +1 happiness and +10% gold for each of Gems, Gold, Pearl | Jewels; +1 happiness and +5% gold each | +15% gold each | +2 happiness and +20% gold each | **Crown Jewelers**: Grand, with 2 Jewels resources |
| Luchuirp | **Blasting Workshop** (needs Fire mana): golems trained here get Fire II | Fire II half the time | Fire II; golems +2 XP | Fire II; +4 XP | **Ember Forge**: Grand + Fire III |
| Luchuirp | **Pallens Engine** (needs Sun mana): golems trained here get Perfect Sight | Perfect Sight half the time | Perfect Sight; +2 XP | Perfect Sight; +4 XP | **Sun Engine**: Grand + Sentry (+1 vision) |
| Luchuirp | **Adularia Chamber** (needs Shadow mana): golems trained here get Hidden | Hidden half the time | Hidden; +2 XP | Hidden; +4 XP | **Chamber of Night**: Grand + Shadowwalk |
| Malakim | **Citadel of Light**: +5% defense; each turn burns the strongest enemy stack within 2 plots for 20 fire damage (to at most 40% damage) | +2%; 10 damage (to 20%) | +8%; 30 damage (to 60%) | +10%; 40 damage (to 80%) | **Lantern of the Sun**: Grand, and it burns the two strongest stacks |
| Sheaim | **Planar Gate**: each turn a 6% chance (up to 15% as the Armageddon Counter rises) to summon a creature for one of the city's buildings (Gambling House: Revelers, Mage Guild: Mobius Witch, Carnival: Chaos Marauder, Grove: Manticore, Public Baths: Succubus, Obsidian Gate: Minotaur, Temple of the Veil: Tar Demon); at most 1 of each creature per gate (up to 4 with the counter) | half the chance | 1.5× the chance | 2× the chance; +1 to each creature's limit | **Open Rift**: 3× the chance; +2 to each creature's limit |

Notes on these tiers:

- **Breeding Pit.** The 60% cap on food kept (rule 8) also covers it, the Granary and the
  Smokehouse together.
- **Freak Show.** It is made by a Freak unit, not built, so its tier is simply the city's roll for
  that class.
- **Warrens.** A Lesser Warrens still copies every unit it copies at Standard, but those units
  take −25% production in the city, so the city puts out 1.5× its units instead of 2×. The
  penalty applies only to the units the Warrens copies. Siege, naval, world and national units
  are trained at the normal rate.
- **Kuriotates resources.** The Tailor's and Jeweler's free resource is the same at every tier
  below Legendary; a resource can't be halved. Their Lesser tier loses half the gold instead.
- **Hall of Mirrors durations.** The same with the Summoner trait since 2026-11-05 (FFH2 gave a
  Summoner's copies 5 turns, the tiers 3/8/10): a Summoner's copies have Summoner's Bond instead
  (docs/PORTING.md, "Balance").

## 5. Wonders: cost only

World and national wonders roll with the same chances. The tier changes only the cost; the
effect is FFH's in every city:

| Tier | Wonder cost |
|---|---|
| Lesser | 125% |
| Standard | 100% |
| Fine | 85% |
| Grand | 70% |
| Legendary | 50% |

The average cost comes to about 95% of FFH's, so wonders get slightly cheaper overall. The
choice a wonder's tier adds is which city builds it. A world-wonder race favors the civilization
with a Grand or Legendary site and production to match.

A wonder keeps its name, shown in its tier's color. The tooltip adds `Site: Grand. Cost: 70%.`

Palaces, the Summer Palace and the Winter Palace roll like any other wonder. Left out: the Tower
of Mastery and the Altar of the Luonnotar, which are victory buildings whose pacing a 50% roll
would break, and buildings that can't be built normally.

## 6. As built: where the rules differ from the proposal

- **Arcanum: Potency, not Empower I.** A building gives a promotion only to units whose unit
  combat the promotion allows (`free_promotion`), and Empower I allows none. The Ancestral Vault
  (which had Potency) gives +3 XP per mana instead, its own direction.
- **Fine Hall of Mirrors: 5 turns**: 3 x 1.5 rounded up, as rule 1 says (the table had 4).
- **Legendary temples spread their religion as a holy shrine does** (`religion_changes`, the
  field FFH2's shrines use): an existing effect in place of "twice as often", which no rule
  counted.
- **Unique buildings' Legendary names** are "Legendary {name}" (the Legendary Grigori Tavern).
- **Lesser unlocking buildings: -25% production** (open point 1 of the proposal, as recommended).
- **Economy**: the tiers are as proposed; nothing was retuned (open point 3).

## 7. How it is built

- **Data.** `data/json/building_tiers.jsonc`: the five tiers (`BuildingTierType`, order fixed),
  their weights and wonder cost percents. `data/json/building_variants.jsonc`: 276 variants of 69
  buildings, each `variant_of` a Standard building at `variant_tier`; the loader lays each over a
  copy of its Standard building's entry (a key the variant leaves out keeps the Standard value)
  and appends it to the buildings table after FFH2's (`ffh-data` `add_building_variants`). Derived
  at load: each family's `tier_variants`, the Standard building's tier, `rolls_tier` on the
  classes (a class with variants, or a buildable wonder class no victory needs). New building
  keys: `unlocked_unit_experience`, `unlocked_unit_production_modifier`, `unlocked_unit_promotion`,
  `extra_free_promotion`, `bonus_yield_changes`, `population_commerces`, `sentinel_repair_change`
  (comments at the top of building_variants.jsonc). Names: `TXT_KEY_BUILDING_<X>_<TIER>` in
  en.tsv.
- **Rules** (`crates/ffh-core/src/city/tiers.rs`). `City::building_tiers`, rolled in `city_init`
  for a new city (one `soren_rand` draw per class), carried over by `player_acquire_city`. A city
  holds only its tier's variant: `city_can_construct` refuses the others, and
  `city_set_num_real_building_timed` / `city_set_num_free_building` set a building given at another
  tier at the city's (events, spells, captures, free buildings need no tier). "The city has X"
  asks for any tier: `city_num_family` and its kin, used wherever the rules name a building (unit
  and spell prerequisites, events, Python ports); loops over every building type read the exact
  counts. Civic and event bonuses to a building class apply to every tier. Wonders: a cost
  multiplier in `city_production_needed_building`. The code effects of the civilization buildings
  read their tier (`city_real_family_tier`, ffh/events.rs). Save format 17.
- **AI.** `building_value` reads the variant, so better tiers rise in the build order by
  themselves; the new keys are valued in `generic_building_value`. A slow world wonder goes to a
  top city by production weighed by its cost tier there (`top_wonder_city`). Where the AI names a
  building for a city (the building a unit needs, a city spell's building), it takes the city's
  tier of it.
- **Interface.** A building's name says its tier ("Fine Library", a Legendary one its own name)
  and is coloured by it (grey, plain, green, blue, gold); the tooltip says `Tier: Fine.` or, for a
  wonder, `Site: Grand. Cost: 70%.`; the city screen's "Building tiers" section lists the city's
  hand (every class not Standard). The building help describes the new keys.
- **Invariant.** `ffh_harness::check` fails a game in which a city holds or queues a building of
  another tier than its own.
- **Tests.** `crates/ffh-core/tests/building_tiers.rs` (the roll, own tier only, any tier unlocks,
  wonder cost, capture, Lesser Warrens, save); `crates/ffh-data/tests/building_civ_data.rs` (every
  ordinary building has five tiers of the same cost and class, names, inheritance).
