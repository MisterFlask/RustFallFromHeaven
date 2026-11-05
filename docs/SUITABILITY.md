# Building suitability (proposal)

**Status: proposal, not built.** Revision 4, 2026-11-05. This revision sets out the rules only:
the five tiers, what each building class is at each tier, and how likely each tier is.
Implementation waits until the XML-to-JSON conversion going on in another session is finished.
The appendix keeps the implementation decisions made so far.

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
   a drawback worse, and a bad tier never adds one. A few Legendary signatures remove a drawback;
   where they do, the table says so.
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
   Most signatures use effects FFH buildings already have (empire-wide or continent-wide
   modifiers, free specialists, free promotions, war-weariness and health switches). The few that
   would be new rules are listed in section 6.

## 3. The buildings

The *Standard* column of each table is FFH 0.41's building, from `CIV4BuildingInfos.xml`.

### 3.1 Knowledge and magic

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Elder Council**: +2 research; 1 scientist slot | +1 research | +3 research | +4 research; 2 slots | **Council of Sages**: Grand + 1 free scientist |
| **Library**: +25% research; 1 scientist slot | +12% | +38% | +50%; 2 slots | **Athenaeum**: Grand + 1 free scientist |
| **Alchemy Lab**: +4 research, +10% research; 1 scientist slot | +2, +5% | +6, +15% | +8, +20%; 2 slots | **Athanor**: Grand + 5% research in all your cities |
| **Mage Guild**: unlocks Adepts, Imps | arcane units −25% production | arcane +2 XP | arcane +4 XP | **Arcanum**: arcane +4 XP and Empower I |

### 3.2 Commerce and trade

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Market**: +3 gold, −1 research; 1 merchant slot; crime +5 | +1 gold | +5 gold | +6 gold; 2 slots | **Emporium**: Grand + 1 free merchant, +1 trade route |
| **Money Changer**: +25% gold; 1 merchant slot | +12% | +38% | +50%; 2 slots | **Counting House**: Grand + 5% gold in all your cities |
| **Tax Office**: +25% gold; −1 happiness | +12% | +38% | +50% | **Exchequer**: Grand + 1 gold per 2 citizens |
| **Gambling House**: +3 gold; +1 happiness per 10% of the gold rate; crime +10; +10% maintenance | +1 gold; +1 happiness per 20% | +5 gold; +3 happiness per 20% | +6 gold; +1 happiness per 5% | **House of Fortune**: Grand, and its crime is removed |
| **Smugglers' Port** (coast): +25% gold; +1 trade route; 1 merchant slot; crime +10 | +12% | +38% | +50%; 2 trade routes; 2 slots | **Black Wharf**: Grand + naval units trained here get Hidden Nationality |
| **Inn**: +25% trade-route yield; +1 trade route; healing +25; +1 happiness with Wine; crime +5 | +12%; healing +12 | +38%; healing +38 | +50%; 2 trade routes; healing +50; +2 with Wine | **Caravanserai**: Grand + 2 great merchant points per turn |
| **Tavern**: +25% trade-route yield; +1 trade route; +1 great artist point; crime +5; +10% maintenance | +12% | +38% | +50%; 2 trade routes; +2 points | **Mead Hall**: Grand + 2 happiness |
| **Obsidian Gate**: airlift 1; +1 trade route | airlift 1; no trade route | airlift 2 | airlift 2; 2 trade routes | **Gate of Night**: airlift 3; 2 trade routes; +25% trade-route yield |

### 3.3 Sea

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Harbor**: +50% trade-route yield; +1 health per Clam, Crab, Fish | +25% | +75% | +100%; +2 health per seafood | **Deepwater Quay**: Grand + 1 trade route, +25% naval production |
| **Lighthouse**: +1 food on water plots; +1 trade route | no trade route | 2 trade routes | 2 trade routes; +1 commerce on water plots | **Beacon Tower**: Grand + naval units trained here get Navigation I |
| **Shipyard**: naval units +4 XP; +25% naval production | +2 XP; +12% | +6 XP; +38% | +8 XP; +50% | **Admiralty**: Grand + naval units trained here get Expanded Hull |

The Lighthouse's +1 food per water plot does not scale: +2 food on every ocean plot would
outweigh everything else in this document. Its tiers act on trade routes and commerce instead.

### 3.4 Production

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Forge**: +25% production; 1 engineer slot; +1 happiness with Gold; −1 health; removes Rusted | +12% | +38% | +50%; 2 slots | **Foundry**: Grand + 1 free engineer |

### 3.5 Culture and happiness

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Monument**: +2 culture | +1 | +3 | +4; +1 happiness | **Standing Stones**: Grand + 1 happiness in all your cities on this continent |
| **Theatre**: +3 culture; +1 happiness per 10% of the culture rate; 1 artist slot; +1 happiness with Dye | +1 culture; +1 happiness per 20% | +5; +3 happiness per 20% | +6; +1 happiness per 5%; 2 slots | **Amphitheatre**: Grand + 1 free artist |
| **Carnival**: +1 happiness; +20% culture; 1 artist slot | +1; +10% | +2; +30% | +2; +40%; 2 slots | **Midsummer Fair**: Grand, with +3 happiness; −50% war weariness |
| **Public Baths**: +3 happiness; −1 health; crime +5 | +1 | +5 | +6 | **Thermae**: Grand, and its unhealthiness and crime are removed |

### 3.6 Growth and health

| Standard | Lesser | Fine | Grand | Legendary |
|---|---|---|---|---|
| **Granary**: 20% food kept; +1 health per Corn, Rice, Wheat | 10% | 30% | 40%; +2 per grain | **Horreum**: Grand + 10% food |
| **Smokehouse**: 20% food kept; +1 health per Cow, Pig, Sheep, Toad | 10% | 30% | 40%; +2 per livestock | **Larder Hall**: Grand + 1 happiness per livestock resource |
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
| **Hunting Lodge**: +1 health with Deer; +1 happiness with Fur; unlocks recon units | recon −25% production | recon +2 XP | recon +4 XP; +2 with Deer and Fur | **Huntsman's Hall**: Grand + recon units trained here get Woodsman I |
| **Grove**: unlocks Druids, Beastmasters, Myconids | those −25% production | +2 XP | +4 XP; +1 health | **Heartwood**: Grand + forest plots in the city's radius give +1 food |

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
| Legendary | +40% | ×2 | 3 | +4 XP; the city spreads its religion to nearby cities twice as often |

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
| **Basilica** (Order): −40% maintenance; crime −10; +1 great commander point | −20%; −5 | −60%; −15 | −80%; −20; +2 points | **Basilica of Junil**: Grand + units trained here get +2 XP |
| **Asylum** (Overlords): +15% research; 1 scientist slot; +1 great prophet point; −25% war weariness; −1 happiness; unlocks Lunatics; 10% of units trained here go Crazed | +7%; −12% | +23%; −38% | +30%; −50%; +2 points; 2 slots | **Bedlam**: Grand, and units trained here never go Crazed |
| **Demons' Altar** (Veil): units trained here get Scourge; −1 happiness; enables Sacrifice the Weak | no Scourge | Scourge; units +2 XP | Scourge; units +4 XP | **Altar of Agares**: Grand, and its unhappiness is removed |

## 4. Unique buildings

**Replacements for a generic building** take that class's roll: the Governor's Manor
(Courthouse), Dwarven Smithy (Forge), Sea Haven (Harbor), Grigori Tavern, Arena and Sculptor's
Studio (Training Yard), and Desert Shrine and Temple of the Hand (Pagan Temple).

- Numbers they share with the generic building scale as in its row.
- Their own extras scale by the general rules. For example, the Desert Shrine's +2 disciple XP
  becomes +1, +3, +4 and +4.
- A Legendary one keeps its own name with the prefix "Legendary" and gets the class's signature.

**The civilization buildings** (`BUILDINGCLASS_CIV_BUILDING1-3`: Warrens, Planar Gate, Cave of
Ancestors, Pallens Engine and the rest) are always Standard. They are a civilization's identity,
not a site's. Most of their effect is code (the Warrens' copies, the Planar Gate's summons) that
has no number to scale.

**Not rolled at all:** cages, vaults, the Dancing Bear, and other buildings that can't be built
normally.

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

## 6. Open points

1. **Lesser on units through production** (rule 5). The alternative is that unlocking buildings
   cannot roll Lesser, with that 25% becoming Standard. I recommend the production penalty:
   without it, the military buildings would be the only classes that never roll badly.
2. **New rules a few signatures need.** Most signatures use effects FFH already has. These would
   be new:
   - Exchequer: gold per citizen.
   - House of Mercy: full heal each turn.
   - Heartwood: food on forest plots.
   - The Legendary temples: twice the religious spread.
   - Thornwall: faster Sentinel repair.
   - Black Wharf and Beacon Tower: a promotion limited to naval units.

   Each is small. Any of them could be swapped for an existing effect if keeping new rules down
   matters more.
3. **Economy.** About +20% building output on average (section 1). I recommend accepting it and,
   if needed, tuning through tech cost.

## Appendix: implementation decisions so far (on hold)

These were settled in earlier revisions. They wait for the XML-to-JSON conversion and will be
revisited against the new data format.

- **Each tier is its own building.** There are no runtime modifiers on the city. The generic
  effect code, the AI's building valuation and the tooltips read the building itself, so they
  stay as they are. The variants are generated from the Standard building by the rules in
  section 2, with per-building overrides for the table entries the general rules don't produce.
- **"The same building."** "The city has X" means it has X or any tier of X. That covers unit and
  spell prerequisites, hard-coded building names in the rules, and class-to-building lookups for
  a city. A test checks that every reference to a building accepts all its tiers.
- **Wonders** stay one building each. Their tier is a cost multiplier on the building's final
  cost, applied after FFH's own cost modifiers.
- **AI.** The AI ranks buildings by value per turn of production, so better tiers rise in its
  build order by themselves. It only starts a slow world wonder in its highest-production city;
  that test should compare turns to finish this wonder instead, which takes the wonder's cost tier
  into account.
- The roll is saved with the city. There is no game option.
