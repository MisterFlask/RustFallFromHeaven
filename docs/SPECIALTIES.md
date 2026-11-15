# City specialties: a permanent buff drawn at culture level 3

**Status: built** (2026-11-15). The owner chooses one of **three random options**.
Replaces building tiers: new cities use Standard buildings. The principles and pool below
remain the design record; the implemented rules are in section 2.

## 0. The idea

When a city reaches culture level 3, it offers three random permanent buffs. Its owner chooses one: its **specialty**.
A specialty is one specific rule that pushes the city toward a function:

- Melee units trained here get +3 XP.
- Wonders are built here at 1.5x speed.
- Friendly units move at double speed in this city's cultural radius.

Every city draws, the capital included. This is part of the rules in every game. There is no game
option.

## 1. Principles

These decide whether a buff belongs in the pool and how the draw works. A new entry is checked
against them before it is added.

1. **A specialty changes what the owner does.** The test for every entry: the owner builds or
   does something different in this city because of it. A buff that only makes the city better
   at everything, or that pays out whatever the owner does, fails. Two kinds pass:
   - it changes what the city builds (wonders, one unit class, settlers, missionaries);
   - it changes how the city is used (where armies stage, where specialists are run).
2. **One specific rule, not a multiplier on a category.** A specialty reads as a single sentence
   with its numbers in it. It is not a tier, a rarity, a category bonus or a choice among
   charters. The owner's examples set the register (section 3).
3. **One per city.** A city's identity is one line a player can remember: "the wonder city",
   "where the archers come from". This is why building tiers were the wrong direction (the
   owner's reasons, 2026-11-15):
   - Tiers gave each city about 45 independent rolls. They average out, so every city ends up
     with much the same mix and none has a character.
   - The decision they create is sorting a table (build the good tiers first, skip the bad
     ones), the same exercise in every city.
   - Nobody remembers 45 tiers per city, so the city screen has to be reopened to know what a
     city is good at.
   - Tiers work strangely with city automation: a city that chooses its own builds has a
     different table to sort in every city.
4. **Only buffs.** No specialty has a price, and none makes the city worse at anything. A random
   penalty reads as punishment, which is what Lesser buildings did.
5. **Different cities, different specialties.** Draws are without replacement within an empire.
   A second city with the same specialty adds much less than the first, and distinct cities are
   the point.
6. **A specialty is never blank.** Three ways an entry can be blank, and the rule for each:
   - *The site lacks what it acts on.* An entry that needs a coast, mines, farms or rivers is
     drawn only by a city that has them.
   - *A civic grants the same thing.* An entry that grants access to an action a civic also
     grants adds something on top, so it still matters once the civic is adopted.
   - *The city cannot train the unit class.* The class is rolled from what the city can train
     when it draws.
7. **Passive value is not a specialty.** A buff whose value comes from where the city sits
   (defense on a border city, maintenance on a distant one), or that is shared by the whole
   empire (mana), does not shape the city. It is cut however good it is.
8. **Culture is the gate, on purpose.** The draw comes at a culture level, so it comes after the
   city has a few buildings and borders, and a city that invests in culture specialises sooner.
9. **Text is bare** (CLAUDE.md, "Rules text and tooltips"): the rule and its numbers, no flavor.

## 2. Rules

- At Developing (culture level index 3: 100 culture at Normal speed), a city offers three
  distinct, uniformly drawn eligible entries. The capital participates. Outposts do not.
- The player picks one permanent effect. The turn cannot end until all specialty choices are
  answered, even with a forced end turn. “Later” sets the dialog aside without discarding it.
- Offers, their rolled unit classes and the selection are saved; loading or querying cannot
  reroll them. The city screen shows the trigger before it is reached and the effect afterward;
  the cities list also shows the chosen effect. Options are revealed at the trigger.
- A chosen entry is excluded from that owner's other offers. If another city chooses an entry
  already offered elsewhere, the conflicting offer is refreshed immediately. Unchosen options
  do not reserve entries. With fewer than three unused eligible entries, all remaining unused
  entries appear and repeats fill the offer. Each offer still has three distinct entries.
- The unit-class XP effect is one pool entry; its class is drawn from trainable unit combats
  when offered and saved with the offer. Coast, mines/quarries and farms have site eligibility.
- Captured cities keep their chosen specialty; it counts against the new owner's offers.
  Captures may create duplicates already held. Pending offers transfer and refresh as needed.
- Units retain their training effects after leaving the city and through upgrades. Settler
  growth, free upkeep and reliable missionary spread are tracked on the trained unit.
- The draft specialty grants one extra draft per 10 game turns (population and normal city
  eligibility still apply), with no unhappiness. Civic drafts remain available between these
  extra drafts, using the civic's normal cost and unhappiness. At most one draft per city/turn.
- Gold rushing requires no civic and costs half; ritual restrictions still apply. Population
  rushing always has access and causes no unhappiness. Existing population limits still apply.
- Full healing requires supply. Movement doubles valid movement inside the friendly city's
  owned cultural radius; forced stops (such as an enemy border effect) still apply.
- AI cities choose from the same offers, using a simple preference for food, specialists and
  military training. Rules are not gated on AI tuning.

Implementation: `city/specialties.rs`, the city production/yield hooks, unit provenance,
`save` format 40, and the play client's `choose_specialty` command and pending dialog.
Verification: `tests/specialties.rs`, the session decision test, and `ffh-play stage specialties DIR`.

## 3. The pool

Numbers are first guesses. Entries marked *owner* are the owner's own examples.

### 3.1 Changes what the city builds

| # | Specialty | Needs |
|---|---|---|
| 1 | [Class] units trained here get +3 XP. *(owner, as Melee)* | a trainable class |
| 2 | Units trained here start with Mobility I. | |
| 3 | Units trained here cost no upkeep. | |
| 4 | Wonders are built here at 1.5x speed. *(owner)* | |
| 5 | Mines and quarries worked by this city give +1 production. | hills or stone |
| 6 | Farms worked by this city give +1 food. | farmable land |
| 7 | Settlers trained here do not stop growth, and the cities they found start at size 2. | |
| 8 | Great Person points are doubled here. | |
| 9 | Each specialist in this city gives +1 research. | |
| 10 | 1.5x culture in this city. *(owner)* | |
| 11 | Missionaries trained here never fail to spread their religion. | |
| 12 | Each religion in this city gives +2 culture and +1 gold. | |
| 13 | Water plots worked by this city give +1 food. | coast |
| 14 | Naval units trained here get +1 movement. | coast |
| 15 | Trade routes with other coastal cities yield double here. | coast |

### 3.2 Changes how the city is used

| # | Specialty | Needs |
|---|---|---|
| 16 | Units in this city heal fully each turn. | |
| 17 | Friendly units move at double speed in this city's cultural radius. *(owner)* | |
| 18 | This city gains +5 food each turn. *(owner)* | |

Food is the one flat yield in the pool: it changes how a city is run (specialists, population
rushing), where health, happiness and the other flat yields do not.

### 3.3 Access to an action a civic also grants

Each has its access and an extra on top (principle 6).

| # | Specialty | Still to settle |
|---|---|---|
| 19 | This city can always rush production with population, and rushing here causes no unhappiness. *(owner, the access)* | |
| 20 | This city can draft one unit every 10 turns with no unhappiness. | the extra once a civic allows drafting |
| 21 | Rushing production with gold costs half here. | whether a civic gates gold rushing |

### 3.4 Not yet placed

- A free specialist in this city (Engineer, Sage, Merchant). *(owner)* It is small next to the
  rest of the pool. To decide: keep it as it is, or make it two specialists.

## 4. Cut, and why

| Entry | Why |
|---|---|
| Enemy units in this city's cultural radius cannot heal. | Passive (principle 7). |
| Units defending this city get +2 first strikes. | Passive. |
| Bombardment removes this city's defense at half speed. | Passive. |
| This city never revolts and cannot be flipped by culture. | Passive; blank on an interior city. |
| This city pays no maintenance. | Passive gold; changes no build. |
| This city provides 1 mana of a random sphere. | Mana is shared across the empire. |
| This city's cultural radius is one ring larger. | A one-time head start on land the city would get anyway. |
| +3 health in this city. +3 happiness in this city. | They raise a cap every city wants equally. |
| Buildings are built here at 1.25x speed. | Everything is a building: it points nowhere. |
| This city keeps 50% of its food when it grows. | Overlaps the Granary and Smokehouse and the 60% cap on food kept. |

Also rejected, as whole designs (2026-11-14): one built building becoming its Legendary version;
a multiplier on a family of buildings; buffs with a price; free specialist slots as the whole
system; a choice among named charters.

## 5. Remaining design work

- More pool entries, especially trade and active defense, remain possible additions.
- The free specialist proposal in 3.4 remains unimplemented: its amount was not settled and it
  is outside the 21 settled pool entries.
- The initial 21 effects' numbers and the AI's preferences may need balance tuning.
