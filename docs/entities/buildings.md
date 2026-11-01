# Buildings and projects

Part of the entity inventory: what each entry does, how to play it, and how far the AI uses it. How to read the entries, the AI grades and the AI machinery they cite are in [docs/ENTITIES.md](../ENTITIES.md).

| Section | Entries | Directed | Valued | Partial | Incidental | Unused | Not an AI choice |
|---|---|---|---|---|---|---|---|
| [City buildings](#city-buildings) | 68 | 2 | 41 | 16 | 2 | 7 |  |
| [World wonders](#world-wonders) | 30 |  | 5 | 14 | 9 | 2 |  |
| [National wonders and limited buildings](#national-wonders-and-limited-buildings) | 34 |  | 3 | 2 | 1 | 7 | 21 |
| [Buildings no city can build (palaces, granted and spell-made)](#buildings-no-city-can-build-palaces-granted-and-spell-made) | 38 | 9 |  | 5 | 2 | 13 | 9 |
| [Projects and rituals](#projects-and-rituals) | 17 | 2 |  |  | 4 | 11 |  |

## City buildings

### Adularia Chamber (`BUILDING_ADULARIA_CHAMBER`)
*Unique building, Luchuirp · 180 hammers · Necromancy and Shadow mana in the city · one of three golem workshops*

**What it does.** Nothing for the city itself: every unit with the Golem promotion finished in the
city gets the Hidden promotion (`ev_unit_built` in `crates/ffh-core/src/ffh/events.rs`): invisible
to other teams, +1 first strike, lost as soon as the unit fights or casts. It never captures
(`bNeverCapture`).

**Playing it well.** Worth it only in the one city that trains the Luchuirp's golems, and only
when a Shadow node feeds that city. Hidden golems walk up to a target unseen and open with an
extra first strike: a stack of Wood or Iron Golems can arrive next to a city before the defender
reacts, and a hidden Clockwork Golem lands its collateral first. It pairs with the Blasting
Workshop and the Pallens Engine in the same city. Elsewhere it is 180 hammers for nothing.

**AI: Unused.** No field it has is read by `production::generic_building_value` (value 0), no
other path names it, and Necromancy comes after Writing in the AI's research plan (`tech_plan.rs`
puts Writing before Mysticism), when a city always has a process to fall back on, so it is never
built (docs/AI.md lists it among the unique content no AI used).

### Adventurers' Guild (`BUILDING_ADVENTURERS_GUILD`)
*Unique building, Grigori · 120 hammers · Currency · +2 Adventurer points, +25% great people, +2 XP for land units*

**What it does.** +2 great people points toward an Adventurer (the Grigori's great person: a 2/2
unit with the Hero promotion that upgrades into an Adept, Warrior or Scout line), +25% great
people rate in the city, and +2 experience for every land unit trained there.

**Playing it well.** The Grigori have no religion and no Great Prophets; this is how they get
their heroes. Build it in the city that already makes great people (sages, merchants, the
capital), where the 25% compounds, and train the army there too for the 2 XP. Adventurers are
worth upgrading along the line the war needs (Adept for a caster, Warrior for a fighter) and
taking promotions early, since the Hero promotion speeds their experience.

**AI: Valued.** The generic value reads all three effects (great people points at 40 a point,
the modifier by population, the land experience by the city's production), so it is built where
production and population are good. What the AI does with the Adventurers is the weak part: they
play as ordinary army units and are upgraded by the generic upgrade (`units::upkeep`; docs/AI_LOG.md
notes that keeping them as casters was never tried).

### Alchemy Lab (`BUILDING_ALCHEMY_LAB`)
*City building · 300 hammers · Sorcery · every civilization but the Ljosalfar, Hippus, Doviello, Clan of Embers, Svartalfar and Calabim*

**What it does.** +4 research and +10% research in the city, and one Sage specialist slot.

**Playing it well.** A late research building for the science cities that already have a
Library: the flat +4 is what makes it worth 300 hammers in a mid-sized city, the 10% only in a
big one. Sorcery is usually on the way for any arcane civilization, so it arrives in time for
the mid-game research push; skip it in small or production-only cities.

**AI: Valued.** All three effects are read (`production::generic_building_value`: 4 research at
9 a point, the 10% on the city's research rate, the slot from size 5), and as a research building
it counts as economy (`production::is_economy_building`), so it is built ahead of surplus army
when finances are strained. Research values Sorcery partly by it (`research::tech_value`).

### Aqueduct (`BUILDING_AQUEDUCT`)
*City building · 180 hammers · Sanitation · every civilization but the Infernal · Supplies can build it*

**What it does.** +2 health. Unlike in Civilization IV it needs no river or lake.

**Playing it well.** Build it when a city is about to hit its health cap: big food cities,
cities working flood plains, cities with Forges or Public Baths. Two health is two more working
citizens, so in a city with spare food it is one of the best mid-game buildings; in a small
healthy city it can wait.

**AI: Valued.** The +2 health is read at 120 a point when the city is at its health cap, 60
within 2 and 20 otherwise (`production::generic_building_value`), so it is built first where
health binds. The AI never uses Supplies to construct it (`units::great_person_build` builds only
shrines).

### Archery Range (`BUILDING_ARCHERY_RANGE`)
*City building · 100 hammers · Archery · every civilization · +10% defense, opens the archer line*

**What it does.** +10% city defense (not against gunpowder units) and the right to train the
archer line in the city: Archers (3/5, first strike, +25% city defense), Longbowmen, Crossbowmen
and Marksmen, and their civilization versions (Dwarven Slinger, Firebow, Flurry). Defender
leaders build it at double speed; Supplies can build it.

**Playing it well.** The defensive building of the early game: one or two in the cities that will
train garrisons, and one in every frontier city that needs archers on the spot. Archers on hills
behind 10% and walls hold against axemen easily. Do not build one in every city; garrisons walk.

**AI: Valued.** Built by `production::enabler_building` where it opens a defender a third better
than the city's best (an Archer's city defense is 83% above a Warrior's), in at most half the
cities; the generic value adds only its 10% defense (5 points, 30 under threat). Marksmen have
min level 6 and come only by upgrade. docs/AI_LOG.md ("An enabling building by strength squared
per hammer") records why the rule is linear: weighted per hammer, nobody built one.

### Arena (`BUILDING_ARENA`)
*Unique building, Balseraphs (replaces the Training Yard) · 100 hammers · Bronze Working*

**What it does.** The Balseraphs' Training Yard: it opens the same melee line (Swordsman,
Mimic, Phalanx, Immortal, Berserker) and adds the Arena Battle ability: a melee or recon unit, or
a slave, in the city fights in the arena; the city gets +1 happy for 3 turns
(`spell_arena_battle`, `crates/ffh-core/src/ffh/spells_a.rs`), and the unit has an even chance of
winning 1-6 experience and coming out at 25% damage (a slave becomes a Warrior) or of dying.
+1 happy with Nationhood.

**Playing it well.** Build it where the melee army is trained, as any Training Yard. The arena is
a cheap way to turn captured slaves into Warriors and to give spare units a gamble at experience,
and the happiness covers a city at its cap; do not send units you cannot afford to lose.

**AI: Partial.** Built only through `production::enabler_building` (the Swordsman's 4 is exactly
a third above a Warrior's 3; its own value is 0), `campaign::siege_building` (for the Berserker,
when no city of ours can train a siege unit), or before Writing as `production::fallback`'s
quickest building. Arena Battle has no `bAllowAI`, so the rules refuse it to AI players
(`unit_can_cast`): the AI never fights in its arenas.

### Asylum (`BUILDING_ASYLUM`)
*Religious building, Octopus Overlords · 150 hammers · Mind Stapling, the Overlords in the city and as state religion*

**What it does.** +15% research, a Sage slot, +1 Great Prophet point, -25% war weariness, +1
unhappiness, and it opens the Lunatic (7/4, collateral, Crazed). In Python (`ev_unit_built`),
every living unit trained there that is not a world unit has an 11% chance to come out Crazed and
Enraged (+20% attack, -20% defense, +1 move, out of its owner's control until it fights).

**Playing it well.** An Overlords theocracy's research building and its Lunatic factory: Lunatics
are cheap (90 hammers) shock troops whose collateral softens stacks. Keep the Asylum out of the
city that trains the regular army, where one unit in nine goes berserk; the war weariness cut
makes it good in a long war.

**AI: Valued.** The research, slot and prophet point are read and the unhappiness counted as a
cost (`production::generic_building_value`); the Crazed chance and the war weariness are not. The
Lunatic can bring it through `production::enabler_building` (a third better per hammer than a
Champion), and, having collateral damage and `UNITAI_ATTACK_CITY`, through
`campaign::siege_building` when no city of ours can train a siege unit. Nothing keeps the AI from
training its army there.

### Basilica (`BUILDING_BASILICA`)
*Religious building, the Order · 200 hammers · Religious Law, the Order in the city and as state religion*

**What it does.** -40% maintenance in the city, -10 crime, +1 Great Commander point; +1 happy
with Social Order. It stacks with the Courthouse.

**Playing it well.** An Order state's second courthouse: worth it in the far, costly cities of a
wide empire, where 40% of the maintenance is several gold a turn, and not in the capital. Crime
only matters for the crime-gated random events (bandits, theft, assassination at 10-35 crime).

**AI: Valued.** The maintenance saving is read as gold (80 a gold, 160 when finances are short)
and the commander point at 40 (`production::generic_building_value`); crime is not read.

### Blasting Workshop (`BUILDING_BLASTING_WORKSHOP`)
*Unique building, Luchuirp · 120 hammers · Sorcery and Fire mana in the city · a golem workshop*

**What it does.** Every unit with the Golem promotion finished in the city gets Fire II
(`ev_unit_built`), which lets it cast Fireball. Nothing else.

**Playing it well.** The strongest of the three golem workshops: every golem out of the city
becomes its own artillery, casting a Fireball before the stack attacks. Build it in the golem city
once a Fire node is connected, together with the Training Yard line there.

**AI: Unused.** Value 0 (no read field) and no other path names it; Sorcery comes after Writing,
so it is never built (listed as unused in docs/AI.md). A golem with Fire II would cast Fireball like any of our units
near a target (`magic::before_attack` takes every unit within two plots that can cast).

### Breeding Pit (`BUILDING_BREEDING_PIT`)
*Unique building, Calabim · 120 hammers · no tech · +2 food, 20% food kept, +1 unhealthiness*

**What it does.** +2 food, keeps 20% of the food box after growth, -1 health.

**Playing it well.** The Calabim's growth building, available from turn one: their vampires Feast
on population and their Governor's Manors turn unhappy citizens into hammers, so fast-growing
cities are the point. Build it early in food-poor cities and in the cities the vampires will
feed in; watch health.

**AI: Valued.** The food (+2 at 20 a point), the food kept and the health cost are all read
(`production::generic_building_value`), so it is built early. That the Calabim use the
population (Feast, `civs/calabim.rs`) is not part of its value.

### Carnival (`BUILDING_CARNIVAL`)
*City building · 120 hammers · Festivals · every civilization · +1 happy, +20% culture, a Bard slot*

**What it does.** +1 happy, +20% culture, a Bard slot; needed for the Balseraphs' Hall of
Mirrors and Harlequin and for the animal cages (Gorilla Cage, Wolf Pen, Lion Cage, Tiger Cage).
Creative leaders build it at double speed. A Sheaim Planar Gate in the city can call a Chaos
Marauder.

**Playing it well.** An early happiness building that also feeds culture; in a culture city it
is worth more than its face. For the Balseraphs it is the gate to their whole Carnival line.

**AI: Valued.** Happiness, culture modifier and slot are read (`production::generic_building_value`),
and for a cultural victory it is one of the culture buildings `victory::production` picks
(`building_culture`). For the Balseraphs the Harlequin can also bring it through
`production::enabler_building`.

### Cave of Ancestors (`BUILDING_CAVE_OF_ANCESTORS`)
*Unique building, Amurites · 120 hammers · Sorcery · arcane units start with experience*

**What it does.** Every arcane unit (Adept combat type) finished in the city gets 1 experience
for each type of mana the city has (`ev_unit_built`). The Amurite palace alone gives three.

**Playing it well.** The Amurites' caster academy: build it in the capital or the node city that
trains Adepts, before the caster wave. Three or four free experience is a promotion's worth on
every Adept, a sphere level sooner.

**AI: Unused.** Value 0 and no other path names it; Sorcery is after Writing, so it is never
built (docs/AI.md: unused). The Amurites do build the Mage Guild and train casters
(`civs::unique_casters`), without the Cave.

### Chancel of Guardians (`BUILDING_CHANCEL_OF_GUARDIANS`)
*Unique building, Elohim · 120 hammers · Priesthood · +25% defense*

**What it does.** +25% city defense (not against gunpowder), and each unit trained there has a
20% chance of the Defensive promotion (double fortification bonus) (`ev_unit_built`).

**Playing it well.** The Elohim's walls and recruitment depot in one; their style is to hold
cities and let others break on them. Build it in border cities and train garrisons there, where
one defender in five gets the Defensive promotion for free.

**AI: Partial.** The 25% is read like Walls (12 points, 75 under threat in
`production::generic_building_value`: small, so it comes late); the promotion chance is not seen.

### Citadel of Light (`BUILDING_CITADEL_OF_LIGHT`)
*Unique building, Malakim · 240 hammers · Sorcery · +5% defense and a self-firing fire attack*

**What it does.** +5% defense. Every turn (`ev_citadel_of_light`) it burns the plot within 2
tiles holding the strongest visible stack of units at war with its owner (plots with neutral units
are skipped): every unit there takes 20 fire damage, never past 40% damage, and a forest or
jungle there catches fire about one time in five.

**Playing it well.** A free, endless fire attack on whatever besieges the city: it softens a
siege stack every turn and keeps raiders off the improvements. Build it in frontier cities facing
a likely enemy; it does nothing in the interior.

**AI: Partial.** Only the 5% defense is read (2 points, 15 under threat), so it is built late, as
a last positive building; the burning is not seen. docs/AI_LOG.md ("Malakim Citadel of Light")
records a try at valuing it in frontier cities that changed nothing because the Malakim AI never
researched Sorcery then.

### Command Post (`BUILDING_COMMAND_POST`)
*City building · 180 hammers · Military Strategy and an Organized leader · +20% military production, +2 XP for land units*

**What it does.** +20% production of military units, +2 experience for land units trained in
the city, +1 Great Commander point. Only Organized leaders can build it; a Great Commander can
also build it.

**Playing it well.** The Organized leader's barracks: one or two in the high-production cities
that train the army, stacked with a Training Yard line. Two experience points and a fifth more
units per hammer is a strong military city; anywhere else it is wasted.

**AI: Valued.** All three effects are read: experience and military production by the city's
production (`production::generic_building_value`), so it goes to the productive cities first. The
AI's Great Commanders do not construct it (`units::great_person_build` builds only shrines).

### Courthouse (`BUILDING_COURTHOUSE`)
*City building · 120 hammers · Code of Laws · every civilization but the Calabim (Governor's Manor) · -40% maintenance*

**What it does.** -40% maintenance in the city, -10 crime, +1 happy with Social Order. Organized
leaders build it at double speed.

**Playing it well.** The answer to FfH's high city maintenance: build it first in distant and
large cities, where 40% is several gold a turn, and early in a wide empire. In a capital or a
small nearby city it can wait. Lower crime keeps the bandit and theft events away.

**AI: Valued.** The maintenance saved is read as gold (80 a gold, 160 when finances are short) in
`production::generic_building_value`, and it counts in a strained empire's escape research
(`finance::escape_tech`: a tech whose building would cut the empire's maintenance, counted at
half, by 3 gold or more), which makes Code of Laws the way out of a deficit. Crime is not read.

### Demons' Altar (`BUILDING_DEMONS_ALTAR`)
*Religious building, Ashen Veil · 150 hammers · Infernal Pact, a Temple of the Veil, the Veil in the city and as state religion*

**What it does.** +1 unhappiness; units trained there get the Scourge promotion (+40% against
disciples); demons trained there get +2 experience (`ev_unit_built`); and any living unit of level
2 or more in the city can be sacrificed for 10 + level squared research toward the current tech
(`spell_sacrifice_altar`).

**Playing it well.** A Veil empire's military altar: build it in the city that trains the army
for a war against a religious enemy (Scourge is +40% against every religion's disciple units), and
use the sacrifice for veterans too damaged or obsolete to keep. Its unhappiness makes it a poor pick
anywhere else.

**AI: Unused.** The only field the generic value reads is its -1 happiness, so its value is
always negative and it is never built (Infernal Pact is long after Writing). The Sacrifice ability
has no `bAllowAI`, so the AI could not use it anyway.

### Desert Shrine (`BUILDING_DESERT_SHRINE`)
*Unique building, Malakim (replaces the Pagan Temple) · 100 hammers · Mysticism*

**What it does.** The Pagan Temple (+10% culture, a Priest slot, +1 happy with Incense) plus +2
experience for every disciple trained in the city. Spiritual leaders build it at double speed.

**Playing it well.** As a Pagan Temple, plus the city to train the Malakim's religious units
in: their disciples (and the religion's temple units) start two points closer to a promotion.

**AI: Valued.** Culture, slot, incense and the disciple experience are all read
(`production::generic_building_value` counts the 2 XP by the city's production whether or not it
trains disciples). A culture building for `victory::production`.

### Dungeon (`BUILDING_DUNGEON`)
*City building · 160 hammers · Masonry · -10 crime, -25% war weariness*

**What it does.** -10 crime and -25% war weariness in the city. One choice of the Order-Veil
religious strife event needs a dungeon.

**Playing it well.** Rarely worth it: build one in a big city that suffers war weariness in a long
war, or in a city whose crime triggers bandit, theft or assassination events (10-35 crime).

**AI: Incidental.** Neither crime nor war weariness is read, so its value is 0 and the building
step never picks it; Masonry comes early, though, so before Writing a city with nothing else to do
can build it as `production::fallback`'s quickest building.

### Dwarven Smithy (`BUILDING_DWARVEN_SMITHY`)
*Unique building, Khazad (replaces the Forge) · 250 hammers · Smelting · +20% hammers, +10% more with each metal*

**What it does.** +20% hammers, +10% more with each of Copper, Iron and Mithril connected (up to
+50%), an Engineer slot, -1 health, and units in the city lose the Rusted promotion. Industrious
leaders build it at double speed.

**Playing it well.** The Khazad's production core: with all three metals it is a +50% Forge.
Build it in every city with real production; the metals are reasons to fight for copper and
iron beyond the weapons.

**AI: Valued.** The 20%, the slot and the health cost are read; the metal bonuses
(`BonusYieldModifiers`) and the rust removal are not, so it is valued as a slightly weaker Forge.
It does not open the Iron Golem (the Luchuirp's), whatever its class says.

### Elder Council (`BUILDING_ELDER_COUNCIL`)
*City building · 60 hammers · Mysticism · every civilization but the Calabim · +2 research, a Sage slot*

**What it does.** +2 research and a Sage slot. Philosophical leaders build it at double speed.
Every Infernal city gets one for free.

**Playing it well.** The cheapest research there is: 60 hammers for 2 beakers pays back fast in
any city, and the Sage slot is an early source of Great Sages. Build it early in the capital and
second cities.

**AI: Valued.** The flat research (18 points a beaker) and the slot are read, and as a research
building it is economy (`production::is_economy_building`), so it is built early and ahead of
surplus army when finances are strained.

### Forge (`BUILDING_FORGE`)
*City building · 250 hammers · Smelting · every civilization but the Khazad (Dwarven Smithy) · +25% hammers*

**What it does.** +25% hammers, an Engineer slot, +1 happy from Gold, -1 health, and units in the
city lose the Rusted promotion; the Luchuirp need it to build Iron Golems. Industrious leaders
build it at double speed; Supplies can build it. Every Infernal city gets one for free.

**Playing it well.** The production multiplier: build it in cities with 10+ base hammers, and
before the wonders and the expensive units. Mind the health; an Aqueduct or Herbalist may have to
follow.

**AI: Valued.** The 25% on the city's production, the slot, the gold happiness and the health cost
are read (`production::generic_building_value`). For the Luchuirp the Iron Golem (10 against a
Wood Golem's 6) also brings it through `production::enabler_building` where Iron or Mithril is
connected.

### Gambling House (`BUILDING_GAMBLING_HOUSE`)
*City building · 200 hammers · Mathematics · +3 gold, +1 happy per 10% gold rate*

**What it does.** +3 gold, +1 happy for every 10% of the gold rate, +10% maintenance, +10 crime.
It costs a quarter under the Gambling Ring resolution, and a Sheaim Planar Gate in the city can
call Revelers.

**Playing it well.** A happiness building for an empire that runs a high gold rate: at 50% gold it
is +5 happy, more than any temple. An empire on full research gets little from it; the crime and
maintenance make it a bad pick in small far cities.

**AI: Partial.** The +3 gold is read (and the extra maintenance as a cost), but not the happiness
per gold rate (`CommerceHappinesses`), its main effect; whatever happiness it gives depends on the
slider the finance logic leaves. The AI never proposes or votes for the Gambling Ring
(`Controller::choose_election` and `diplo_vote` keep their defaults in ffh-ai).

### Governor's Manor (`BUILDING_GOVERNORS_MANOR`)
*Unique building, Calabim (replaces the Courthouse) · 180 hammers · Code of Laws · -20% maintenance, hammers from unhappiness*

**What it does.** -20% maintenance, -10 crime, -25% war weariness, +1 hammer for every unhappy
citizen, +1 unhappiness, +1 happy with Social Order; needed to train Vampires. Organized leaders
build it at double speed.

**Playing it well.** The Calabim run their cities over the happiness cap, feeding the excess to
their vampires; the Manor turns that excess into production. Build it where cities are over the
cap and where Vampires (the Calabim Champion) are trained.

**AI: Partial.** The maintenance saving is read and the extra unhappiness counted as a cost, so
in a city at its cap its value is often negative; the hammers from unhappy citizens and the war
weariness are not read. It counts in the escape research (`finance::escape_tech`). The Vampire (5)
is not a third better than the Moroi (4), so `production::enabler_building` does not build it for
them; docs/AI_LOG.md ("Vampires, Feast, Gift Vampirism") notes that the Calabim's vampire play is
gated behind Feudalism, Governor's Manors and Moroi upgrades.

### Granary (`BUILDING_GRANARY`)
*City building · 120 hammers · Agriculture · every civilization but the Infernal · 20% food kept*

**What it does.** Keeps 20% of the food box after growth, +1 health from Corn, Rice and Wheat.
Expansive leaders build it at double speed.

**Playing it well.** Good in fast-growing food cities and with the grain resources; FfH's growth
is slow enough that 20% is a real saving. In small, food-poor cities a Smokehouse or nothing.

**AI: Valued.** Food kept is read by the city's food rate and the grain health where the city has
the resource (`production::generic_building_value`).

### Grigori Tavern (`BUILDING_TAVERN_GRIGORI`)
*Unique building, Grigori (replaces the Tavern) · 250 hammers · Mercantilism · +1 trade route, +1 Adventurer point*

**What it does.** The Tavern with an Adventurer point instead of a Bard point: +1 trade route,
+25% trade route yield, +10% maintenance, +5 crime, +1 Adventurer point, +1 happy with
Consumption.

**Playing it well.** As the Tavern, in the core commerce cities; for the Grigori it also feeds
their Adventurers, so pair it with the Adventurers' Guild.

**AI: Valued.** The trade route (60), the great people point (40) and the maintenance cost are
read; the 25% trade route yield and the crime are not.

### Grove (`BUILDING_GROVE`)
*City building · 300 hammers · Feral Bond · opens Druids and Beastmasters*

**What it does.** Needed to train Druids (Commune with Nature, Neutral alignment; Dwarven Druids
for the dwarves) and Beastmasters (Animal Mastery; the Khazad's Myconid); +2 happy with Guardian
of Nature. A Sheaim Planar Gate in the city can call a Manticore.

**Playing it well.** Build one, in the city that will train the tier-4 nature units, once the techs
for them are near: by itself it does nothing.

**AI: Valued.** Its value is 0; it comes through `production::enabler_building` when the city can
train a Druid (8, against a Champion's 6) or a Beastmaster (14) and the building takes 20 turns or
less, and research counts those units once the Grove's tech is in (`research::unit_available`).
Before that rule, no Grove was built in 380 forced games (docs/AI.md, "Unique units").

### Hall of Mirrors (`BUILDING_HALL_OF_MIRRORS`)
*Unique building, Balseraphs · 180 hammers · Alteration, a Carnival in the city · +1 happy, illusions*

**What it does.** +1 happy. Every turn (`ev_hall_of_mirrors`) a copy of an enemy unit standing
next to the city (one at war, the last found) appears in the city as an Illusion for 3 turns (5
with the Summoner trait).

**Playing it well.** A defensive building for the frontier: each turn a besieger walks up, the
city gets a free copy of it to strike back with, which costs nothing when it dies. The stronger the
enemy's units, the better the copy.

**AI: Partial.** Only the +1 happy is read (`production::generic_building_value`), so it is built
as a happiness building; the illusions are not seen; the copies play by their unit type's default AI
(`units::role_of`) for the turns they last.

### Harbor (`BUILDING_HARBOR`)
*City building · 120 hammers · Fishing, a coastal city · every civilization but the Lanun (Sea Haven) · +50% trade route yield*

**What it does.** +50% trade route yield and +1 health from Clam, Crab and Fish. Expansive leaders
build it at double speed. (It gives no food on water tiles; the Lighthouse does.)

**Playing it well.** A trade building for coastal cities with good trade routes (overseas and
foreign routes, big partners), and a health building where seafood is worked.

**AI: Partial.** Only the seafood health is read, and only where the city has those resources:
elsewhere its value is 0 and it is not built (before Writing, `production::fallback` can still
pick it as the quickest building). The trade route yield, its main effect, is
not read; valuing it was tried and not kept (docs/AI.md, "Value the building effects
`building_value` ignores").

### Herbalist (`BUILDING_HERBALIST`)
*City building · 90 hammers · Agriculture · every civilization but the Infernal · +1 health, faster healing*

**What it does.** +1 health, +1 health from Reagents, units in the city heal 10% more a turn.

**Playing it well.** A cheap early health building, and a hospital for the city where wounded
stacks return; build it before the city hits its health cap.

**AI: Valued.** Health and healing are both read (`production::generic_building_value`).

### Hunting Lodge (`BUILDING_HUNTING_LODGE`)
*City building · 100 hammers · Hunting · opens the recon line*

**What it does.** Needed to train Hunters, Rangers, Assassins and Hawks (and their civilization
versions: Divided Soul, Hellhound, Lizardman, Chanter, Devout, Ghost, Taskmaster); +1 health from
Deer, +1 happy from Fur. Supplies can build it.

**Playing it well.** One lodge for the recon line: Hunters clear animals early and gain experience
toward strong Rangers, and Assassins pick off the weakest unit of a stack. Build it in a city with
deer or fur, or in the city that trains the recon units.

**AI: Valued.** The resource health and happiness are read where the city has them; otherwise it
comes through `production::enabler_building` for a Hunter (4 against a Warrior's 3), Ranger or
Assassin. The Hawk passes no production role and is never trained.

### Infirmary (`BUILDING_INFIRMARY`)
*City building · 300 hammers · Medicine · every civilization but the Infernal · +3 health, faster healing*

**What it does.** +3 health and units in the city heal 25% more a turn; the Grigori need it to
train their Medics.

**Playing it well.** The late health building for the big cities, and the field hospital of the
city behind the front, where damaged stacks come back. For the Grigori it is the Medic depot:
one Infirmary in a production city is enough.

**AI: Valued.** Health (up to 120 a point at the cap) and healing are read
(`production::generic_building_value`). The Grigori also build one on purpose: at war with 4+
attackers, if no city has one, the most productive city builds it to train Medics
(`civs/grigori.rs`, `medic`).

### Inn (`BUILDING_INN`)
*City building · 250 hammers · Trade · +1 trade route, faster healing*

**What it does.** +1 trade route, +25% trade route yield, units heal 25% more a turn, +1 happy from
Wine, +5 crime.

**Playing it well.** A commerce building for well-connected cities (coastal, on a road net with
foreign partners), and a healing stop on the way to the front; a wine city gets a face as well.

**AI: Valued.** The trade route (60), the healing and the wine happiness are read; the 25% trade
route yield and the crime are not (`production::generic_building_value`).

### Jeweler (`BUILDING_JEWELER`)
*Unique building, Kuriotates · 120 hammers · Smelting · a free Jewelry resource*

**What it does.** Provides one Jewelry (a luxury: +1 happy in every connected city), +1 happy and
+10% commerce in the city with each of Gems, Gold and Pearls.

**Playing it well.** Build one, early, in any Kuriotates city: the Jewelry is a face in every city
of the empire, which matters for a civilization of few, very large cities; the commerce bonus
makes the gem or gold city the place for it.

**AI: Partial.** Only the happiness from Gems, Gold and Pearls is read, where the city has them:
elsewhere its value is 0 and it is never built, and the free Jewelry, its main effect, is not
seen (valuing it was tried and not kept: docs/AI.md, "Value the building effects `building_value`
ignores"). The commerce bonus is not read either.

### Library (`BUILDING_LIBRARY`)
*City building · 120 hammers · Writing · every civilization but the Doviello and the Clan of Embers*

**What it does.** +25% research in the city and one Sage specialist slot; it is needed for the
Great Library and the National Epic, and Philosophical leaders build it at double speed. Supplies
can build it.

**Playing it well.** Build it first where the commerce is: capital, cottage and river cities,
cities working Sages. In a 2-commerce city it is a waste of 120 hammers for a long time. Pair it
with a high research slider and with the Great Library or National Epic in the best science
city.

**AI: Valued.** The generic value reads both effects (`production::generic_building_value`: the
25% on the city's research rate at 9 a point, the specialist slot from size 5), so it is built
where research is high, ahead of the army when finances are strained (`is_economy_building`);
research values Writing by it (`research::tech_value`). That it opens the two wonders is not
valued.

### Lighthouse (`BUILDING_LIGHTHOUSE`)
*City building · 160 hammers · Sailing, a coastal city · +1 food on water tiles, +1 trade route*

**What it does.** +1 food on every water tile the city works and +1 trade route; needed for the
Great Lighthouse. Organized leaders build it at double speed.

**Playing it well.** The growth building of a coastal city: with it a coast tile (1 food, 2 commerce)
feeds its own worker, so a city with many water tiles can grow on them and work coast for
commerce. Build it early in such cities.

**AI: Partial.** Only the trade route is read (60 points); the water food, its main effect, is not
(`SeaPlotYieldChanges`), so it ranks low and is rarely built: valuing the food took Lighthouses
from 2 to 29 over four games in a measured try that was not kept (docs/AI.md, "Value the building
effects `building_value` ignores").

### Mage Guild (`BUILDING_MAGE_GUILD`)
*City building · 120 hammers · Knowledge of the Ether · every civilization but the Doviello and the Clan of Embers · opens the Adept*

**What it does.** Needed to train Adepts (and the Infernal Imps) and for the Catacomb Libralus.
Arcane leaders build it at double speed. Every Infernal city has one for free; a Sheaim Planar
Gate in the city can call a Mobius Witch.

**Playing it well.** The door to the whole caster line (Adept, Mage, Archmage and the
civilizations' versions). One or two, in the cities with the most mana and production (a Cave of
Ancestors, a node city), are enough for most empires; arcane civilizations want them early.

**AI: Partial.** Its own value is 0, so the generic AI never builds it ("no AI built a Mage Guild",
docs/AI.md). Two civilizations build it on purpose: the Amurites and the Svartalfar, whose
Wizards and Illusionists come from Adepts, put one in their most productive city before training
casters up to (cities + 2) / 2 (`civs::unique_casters`). The Doviello and the Clan train Shamans,
which need no guild, and the Infernal get one in every city. Any other civilization has a guild,
and so Adepts, only in a city it captured with one.

### Market (`BUILDING_MARKET`)
*City building · 60 hammers · Festivals · every civilization · +3 gold, a Merchant slot*

**What it does.** +3 gold, -1 research, +5 crime, a Merchant slot, +1 happy with Consumption and
with Mercantilism. Financial leaders build it at double speed.

**Playing it well.** Cheap gold for any city, the earliest fix for a strained treasury; the
research loss is small. With Consumption or Mercantilism it is also a happiness building.

**AI: Valued.** The gold, the research loss and the slot are read (`production::generic_building_value`);
as a gold building it is economy (`is_economy_building`) and comes ahead of surplus army when
finances are strained. Crime and the civic happiness are not read.

### Money Changer (`BUILDING_MONEYCHANGER`)
*City building · 250 hammers · Currency · every civilization · +25% gold*

**What it does.** +25% gold and a Merchant slot. Financial leaders build it at double speed.

**Playing it well.** A gold multiplier for the cities with real gold (shrine cities, markets,
merchants), or for an empire running a high gold rate; in a city with 4 gold it is a long payback.

**AI: Valued.** The 25% on the city's gold rate and the slot are read, and it is an economy
building (`is_economy_building`).

### Monument (`BUILDING_MONUMENT`)
*City building · 60 hammers · Ancient Chants · every civilization · +2 culture*

**What it does.** +2 culture (never obsolete). Creative leaders build it at double speed.

**Playing it well.** The border pop for a new city: the first ring of land comes sooner, and a
contested border city holds its tiles. Build it first in new cities that need the land, skip it
where a temple comes soon.

**AI: Valued.** The 2 culture is read at 4 a point (`production::generic_building_value`). The extra
weight that function gives cheap culture buildings in new cities (x150 for a city at culture
level 1 or less) reads only `CommerceChanges`, and the Monument's culture is in
`ObsoleteSafeCommerceChanges`, so it never applies: the Monument is valued at 80, as a minor
culture building, not as a border pop. For a cultural victory it is one of the culture buildings
`victory::production` builds.

### Obsidian Gate (`BUILDING_OBSIDIAN_GATE`)
*City building · 300 hammers · Pass Through the Ether · airlift and +1 trade route*

**What it does.** The city can airlift one land unit a turn to any city of its team and has +1
trade route. Every Infernal city has one for free; it is lost when an Infernal city is taken. A
Sheaim Planar Gate in the city can call a Minotaur.

**Playing it well.** A late network for moving defenders between fronts: two or three gates in
the border cities and the military core let one army answer two wars. The trade route alone is
not worth 300 hammers.

**AI: Partial.** Only the trade route is read (60 points), plus, for Thessa, +400 as a favourite
wonder through the C++ index quirk (her late wonder, a building, read as a class index lands on
this class). The AI never airlifts.

### Pagan Temple (`BUILDING_PAGAN_TEMPLE`)
*City building · 100 hammers · Mysticism · every civilization but the Malakim, Grigori and Illians · +10% culture, a Priest slot*

**What it does.** +10% culture, a Priest slot, +1 happy from Incense. Spiritual leaders build it at
double speed. (The Malakim have the Desert Shrine, the Illians the Temple of the Hand; the agnostic
Grigori have neither.)

**Playing it well.** An early culture and Great Prophet building with no religion needed: a Priest
in the capital speeds the first prophet for a shrine. With incense it is also a face.

**AI: Valued.** Culture, slot and incense happiness are read (`production::generic_building_value`),
and it is a culture building for `victory::production`.

### Palisade (`BUILDING_PALISADE`)
*City building · 40 hammers · no tech · +10% defense*

**What it does.** +10% defense (not against gunpowder). Defender leaders build it at double speed;
Supplies can build it.

**Playing it well.** The first defense of a city facing barbarians or an early rush, for 40
hammers; Walls replace its role later. Not worth it in safe interior cities.

**AI: Valued.** The 10% is read, weakly (5 points, 30 under threat), so it is built when nothing
better is left, and quickly when a city is threatened.

### Pallens Engine (`BUILDING_PALLENS_ENGINE`)
*Unique building, Luchuirp · 180 hammers · Divination and Sun mana in the city · a golem workshop*

**What it does.** Every unit with the Golem promotion finished in the city gets Perfect Sight
(`ev_unit_built`): it sees invisible units (and fights illusions at +50%).

**Playing it well.** A counter-pick against invisible enemies (Svartalfar, Sidar, hidden units and
illusions): a few golems with Perfect Sight in each stack stop ambushes. Build it only if those
enemies are in the game and a Sun node feeds the golem city.

**AI: Unused.** Value 0 and no other path names it; Divination is after Writing, so it is never
built (docs/AI.md: unused).

### Planar Gate (`BUILDING_PLANAR_GATE`)
*Unique building, Sheaim · 300 hammers · Knowledge of the Ether · summons planar creatures*

**What it does.** Each turn (`ev_planar_gate`) a 6% chance (9%, 12%, 15% at Armageddon counts of
50, 75, 100) to call one creature, chosen among those whose building the city has: Revelers
(Gambling House), Mobius Witch (Mage Guild, level 4 with random sphere promotions), Chaos Marauder
(Carnival), Manticore (Grove), Succubus (Public Baths), Minotaur (Obsidian Gate), Tar Demon (Temple
of the Veil). Each kind is limited to one per gate the player has (two, three, four as the counter
rises).

**Playing it well.** Build it in a city that already has several of the trigger buildings
(Carnival, Gambling House, Public Baths and a Mage Guild are all ordinary city buildings), and
push the Armageddon counter, which the Sheaim want anyway: the creatures are free units.

**AI: Unused.** Value 0 and no other path names it, so it is never built; valuing it by its
creatures was tried and changed nothing, built 0.1 a game (docs/AI_LOG.md, "Sheaim Planar Gate").

### Public Baths (`BUILDING_PUBLIC_BATHS`)
*City building · 150 hammers · Sanitation · every civilization but the Infernal · +3 happy, +1 unhealthiness*

**What it does.** +3 happy, -1 health, +5 crime. A Sheaim Planar Gate in the city can call a
Succubus.

**Playing it well.** The big happiness building of the classical age: three faces for 150 hammers
in a city at its cap, if the city has the health to spare (build the Aqueduct first or with it).

**AI: Valued.** Happiness (180 a face at the cap) and the health cost are read
(`production::generic_building_value`), and research values Sanitation by its happiness when
cities are capped (`research::tech_value`).

### Reliquary (`BUILDING_RELIQUARY`)
*Unique building, Elohim · 100 hammers · Way of the Wise · +2 culture, a Great Prophet point, Spirit Guide*

**What it does.** +2 culture, +1 Great Prophet point, -1 health, and units trained there get the
Spirit Guide promotion: when such a unit dies it passes part of its experience to another unit.

**Playing it well.** Train the Elohim's army in the Reliquary city: veterans that die leave their
experience behind, which keeps a defensive army strong through a long war. The culture and
prophet point make it cheap to justify.

**AI: Partial.** Culture, the prophet point and the health cost are read; Spirit Guide, its
distinctive effect, is not (`FreePromotion` is invisible to `production::generic_building_value`).

### Sculptor's Studio (`BUILDING_SCULPTORS_STUDIO`)
*Unique building, Luchuirp (replaces the Training Yard) · 90 hammers · Construction · opens the golems*

**What it does.** +1 culture; needed to train Wood Golems (6/6, the Luchuirp Axeman) and
Gargoyles, and, as the Luchuirp's Training Yard, Clockwork, Bone and Nullstone Golems; +1 happy with
Nationhood.

**Playing it well.** The Luchuirp's military building: Wood Golems at 90 hammers outclass early
axemen, but golems heal slowly (Golem: -10% healing in friendly land) and, with no combat type,
take no promotions, so promoted Dwarven Slingers remain the better garrison. Build Studios in the cities that train the army, not in every city.

**AI: Valued.** The +1 culture alone gives it a small value, and `production::enabler_building`
builds it where it opens a better unit (a Wood Golem doubles a Warrior); the Clockwork Golem's
collateral can also bring it through `campaign::siege_building`. Measured: Wood Golems
appear in 95% of Luchuirp games instead of 45% since the enabling buildings count
`PrereqBuilding` (docs/AI.md, "Unique units"). Older experiments (`civs/luchuirp.rs`,
docs/AI_LOG.md) found that golems held cities worse than slingers, which the per-hammer rule now
corrects.

### Sea Haven (`BUILDING_HARBOR_LANUN`)
*Unique building, Lanun (replaces the Harbor) · 90 hammers · Fishing, a coastal city · cheaper and better than the Harbor*

**What it does.** The Harbor (+50% trade route yield, +1 health from Clam, Crab, Fish) plus -10%
maintenance and +25% production of water units; needed for the Lanun's Boarding Party (5/5,
Boarding: their Champion). Expansive leaders build it at double speed.

**Playing it well.** Every Lanun coastal city wants one: cheap, it fits their sea trade and their
navy, and it is where Boarding Parties and ships are built.

**AI: Partial.** The maintenance saving and the seafood health are read, so it has a small value
everywhere; the trade route yield and the naval production are not. The Boarding Party can bring
it through `production::enabler_building` while the city's best defender is a Warrior (5 against
3.75 in a city).

### Shipyard (`BUILDING_SHIPYARD`)
*City building · 200 hammers · Iron Working, a coastal city · +4 XP and +25% production for ships*

**What it does.** New water units get +4 experience and are built 25% faster; needed for the Queen
of the Line (9/9, cargo 6). Aggressive leaders build it at double speed.

**Playing it well.** One Shipyard in the main port of a naval power: veteran ships win the sea
fights and the Queen of the Line is the best transport-warship. A land empire does not need one.

**AI: Unused.** `production::generic_building_value` reads domain experience for land units only,
and not the naval production modifier, so its value is 0; `production::enabler_building` looks
only at land units, so the Queen of the Line does not bring it; Iron Working comes after Writing.
It is never built, and with it the Queen of the Line is never trained.

### Siege Workshop (`BUILDING_SIEGE_WORKSHOP`)
*City building · 120 hammers · Construction · every civilization but the Ljosalfar and Svartalfar · opens siege engines and chariots*

**What it does.** Needed to train Catapults, Trebuchets (Khazad), Cannons and Dwarven Cannons,
Chariots, the Lanun War Tortoise and the Doviello War Machine. Aggressive leaders build it at
double speed. Nothing else.

**Playing it well.** One or two workshops in production cities, before the first war against a
walled neighbour: catapults bring walls down and soften stacks with collateral, which is what lets
a melee stack take a city. Chariots are a fast early strike force where horses are available.

**AI: Directed.** `campaign::siege_building` builds it in the most productive city when the siege
operation wants engines and no city of ours can train one (and none is building it); it can also
come through `production::enabler_building` for a Chariot or War Tortoise a third better than the
city's best. Its own value is 0. Since the enabling buildings count `PrereqBuilding`, every
civilization fields catapults and the Khazad Trebuchets appear in 80% of their games (docs/AI.md,
"Unique units"); before, no Siege Workshop was built in 380 forced games. The War Machine is trained as a hero
once a workshop stands, but nothing builds one for it.

### Smokehouse (`BUILDING_SMOKEHOUSE`)
*City building · 120 hammers · Animal Husbandry · every civilization but the Infernal · 20% food kept*

**What it does.** Keeps 20% of the food box after growth (like the Granary, and it stacks), +1
health from Cow, Pig, Sheep and Toads.

**Playing it well.** The livestock city's granary; worth it with two of those resources or in a
city that grows fast, otherwise later than everything else.

**AI: Valued.** Food kept by the city's food rate and the resource health are read
(`production::generic_building_value`).

### Smugglers' Port (`BUILDING_SMUGGLERS_PORT`)
*City building · 120 hammers · a coastal city and the Smuggling Ring resolution · +25% gold, +1 trade route*

**What it does.** +25% gold, a Merchant slot, +1 trade route, +10 crime. The rules allow it only
while the player is under the Smuggling Ring (an Undercouncil resolution, `py_cannot_construct`);
the Smugglers random event (a coastal city with 30 crime) can grant one.

**Playing it well.** A strong gold building for a coastal Undercouncil member: propose the Smuggling
Ring when you have many ports. Expect crime events in those cities.

**AI: Incidental.** The generic value would read it (gold %, slot, trade route), but in an AI-only
game the Smuggling Ring never passes: ffh-ai neither picks elections nor votes
(`Controller::choose_election` and `diplo_vote` keep their defaults: no election, abstain), so the
AI cannot build it. It can get one from the Smugglers event, whose choice values a granted
building at twice its hammers (`events::choose`).

### Stable (`BUILDING_STABLE`)
*City building · 100 hammers · Horseback Riding · every civilization but the Kuriotates and the Clan of Embers · opens the mounted line*

**What it does.** Needed to train Horsemen, Horse Archers and Knights and their civilization
versions (Boar Rider, Camel Archer, Fyrdwell, Nyxkin, Hornguard, Bison Rider, Death Knight).
Supplies can build it. Nothing else.

**Playing it well.** One or two in cities with horses connected, for the fast units that pick off
siege, raid and catch withdrawing enemies; Horse Archers and Knights are the mid-game strike force.

**AI: Valued.** Value 0; it comes through `production::enabler_building` where it opens an attacker
a third better than the city's best (a Horseman against a Warrior, a Horse Archer against an
Axeman, a Knight against a Champion) and the city has the horse the unit needs, in at most half
the cities; research counts the Hornguard and Bison Rider once the Stable's tech is in
(`research::unit_available`). Before Writing it can also be `production::fallback`'s quickest
building.

### Tailor (`BUILDING_TAILOR`)
*Unique building, Kuriotates · 140 hammers · Crafting · a free Fine Clothes resource*

**What it does.** Provides one Fine Clothes (a luxury: +1 happy in every connected city), +1 happy
and +10% commerce in the city with each of Dye, Cotton and Silk.

**Playing it well.** As the Jeweler: one early, anywhere, for a face in every city, ideally in the
city with dye, cotton or silk.

**AI: Partial.** Only the resource happiness is read, where the city has those resources; the free
Fine Clothes, its main effect, is not, so elsewhere its value is 0 (Crafting is a starting-area
tech, so before Writing `production::fallback` may still build it as the quickest building).

### Tavern (`BUILDING_TAVERN`)
*City building · 250 hammers · Mercantilism · every civilization but the Grigori · +1 trade route, +1 Bard point*

**What it does.** +1 trade route, +25% trade route yield, +1 Great Bard point, +10% maintenance, +5
crime, +1 happy with Consumption.

**Playing it well.** A core-city commerce building, as its strategy text says: in the big trade
cities near the capital, not in far cities whose maintenance is already high.

**AI: Valued.** The trade route, the bard point and the maintenance cost are read; the trade route
yield and the crime are not.

### Tax Office (`BUILDING_TAX_OFFICE`)
*City building · 200 hammers · Taxation · +25% gold, +1 unhappiness*

**What it does.** +25% gold and +1 unhappiness.

**Playing it well.** A second gold multiplier for gold-heavy cities with a face to spare; with a
Market and Money Changer it makes a shrine or merchant city a treasury.

**AI: Valued.** The gold modifier is read on the city's gold rate and the unhappiness as a cost
(`production::generic_building_value`), so it goes to cities with gold and happiness room.

### Temple of Kilmorph (`BUILDING_TEMPLE_OF_KILMORPH`)
*Religious building, Runes of Kilmorph · 120 hammers · Way of the Earthmother, the Runes in the city · every civilization but the agnostic Grigori*

**What it does.** +2 gold, +20% culture, a Priest and a Merchant slot, +1 happy from Gems, +1 happy
with the Religion civic. It trains the Runes' units: the Thane of Kilmorph (the disciple, its
missionary), the Stonewarden (Priesthood) and the Paramander (Fanaticism, Copper), the last two
only with the Runes as state religion. A Stonewarden can sacrifice itself to found a temple in a
city.

**Playing it well.** Every Runes city with some culture or gold to multiply gets one; the city that
trains the Runes' units needs one. A dwarven-flavoured economy temple: gold and a merchant slot.

**AI: Valued.** Gold, culture modifier, slots and gem happiness are read
(`production::generic_building_value`); it is a culture building for `victory::production`. As the
missionary's building it matters more than its value says: the AI's missionary step trains only
temple disciples (`production::best_missionary`), so its state religion spreads only from cities
that have this temple. Found Temple has no `bAllowAI`.

### Temple of Leaves (`BUILDING_TEMPLE_OF_LEAVES`)
*Religious building, Fellowship of Leaves · 120 hammers · Way of the Forests, the Fellowship in the city · every civilization but the Grigori*

**What it does.** +1 health, +20% culture, a Priest and a Bard slot, +1 happy from Incense, +1 happy
with Religion; trains the Disciple of Leaves (missionary) and the Priest of Leaves (Priesthood,
state religion). Plots worked by the city triple the chance that Treants rise when an enemy at war
steps alone into an owned ancient forest of a Fellowship state (`on_move_ancient_forest`).

**Playing it well.** As any temple, and the forest defense of a Fellowship empire: keep ancient
forests around the temple cities and invaders walking through them meet Treants.

**AI: Valued.** Health, culture modifier, slots and incense are read; the Treant chance is not.
It is the missionaries' building for a Fellowship state (`production::best_missionary`).

### Temple of the Empyrean (`BUILDING_TEMPLE_OF_THE_EMPYREAN`)
*Religious building, Empyrean · 120 hammers · Honor, the Empyrean in the city · every civilization but the Grigori*

**What it does.** +10% research, +20% culture, two Priest slots, +1 happy from Incense, +1 happy with
Religion; trains the Ecclesiastic (missionary) and the Vicar (Priesthood, state religion).

**Playing it well.** The research temple: build it in the science cities of an Empyrean empire,
where its 10% stacks with the Library.

**AI: Valued.** Research and culture modifiers, slots and incense are read
(`production::generic_building_value`); it is the missionaries' building for an Empyrean state.

### Temple of the Hand (`BUILDING_TEMPLE_OF_THE_HAND`)
*Unique building, Illians (replaces the Pagan Temple) · 100 hammers · Mysticism · turns the land around the city to snow*

**What it does.** The Pagan Temple (+10% culture, a Priest slot, +1 happy from Incense), and when
built (`ev_building_built`) every land plot within 2 tiles of the city becomes snow: smoke, flames,
flood plains and scrub are removed and jungle becomes forest. Snow gives the Illians 2 food (a
civilization yield), so grassland loses nothing, tundra and desert gain, and plains trade their
hammer for a food.

**Playing it well.** Build it early in Illian cities on tundra, desert or poor land, which it turns
into 2-food plots, and think twice in a plains-hill production site (hills stay hills). It also
wipes the city's flood plains.

**AI: Partial.** Valued as a Pagan Temple (culture, slot, incense); the terrain change, which can
help or hurt the city a lot, is not seen.

### Temple of the Order (`BUILDING_TEMPLE_OF_THE_ORDER`)
*Religious building, the Order · 120 hammers · Orders from Heaven, the Order in the city · every civilization but the Grigori*

**What it does.** +20% culture, two Priest slots, +10% military production, -5 crime, +1 happy from
Incense, +1 happy with Religion; trains the Acolyte (missionary), the Confessor (Priesthood) and
the Crusader (Fanaticism, a metal), the last two with the Order as state religion. Donal's Recruit
raises twice as many units in a city with one.

**Playing it well.** The military temple: build it in the army cities of an Order empire, and keep
Donal's Recruit for a big city that has it.

**AI: Valued.** Culture modifier, slots, incense and the military production are read
(`production::generic_building_value`). It is named once in ffh-ai: Donal's Recruit counts double
units in a city with the temple (`magic::recruit_value`). It is the missionaries' building for an
Order state, and the Crusader can bring it through `production::enabler_building` where the best
attacker is an Axeman.

### Temple of the Overlords (`BUILDING_TEMPLE_OF_THE_OVERLORDS`)
*Religious building, Octopus Overlords · 120 hammers · Message from the Deep, the Overlords in the city · every civilization but the Grigori*

**What it does.** +3 culture, +20% culture, a Priest and a Sage slot, +1 happy from Incense, +1
happy with Religion; trains the Zealot (missionary), the Cultist (Priesthood) and the Stygian Guard
(Fanaticism), the last two with the Overlords as state religion. In an Overlords state, a Warrior in
the city can be turned into a Drown (3/3 undead, water walking) for 60 gold (`SPELL_DROWN`).

**Playing it well.** The culture temple; and the Drown conversion turns obsolete Warriors into
amphibious raiders cheaply.

**AI: Valued.** Culture, culture modifier, slots and incense are read; a culture building for
`victory::production`. Drown is allowed to the AI but FfH's terms value the conversion at less than
its 60 gold (`casting::ffh_value`: +150 for the unit, -240 for the gold), so it is not cast.

### Temple of the Veil (`BUILDING_TEMPLE_OF_THE_VEIL`)
*Religious building, Ashen Veil · 120 hammers · Corruption of Spirit, the Veil in the city · every civilization but the Grigori*

**What it does.** +2 research, +20% culture, a Priest and a Sage slot, +1 happy from Reagents, +1
happy with Religion; trains the Savant (missionary), the Ritualist (Priesthood) and the Beast of
Agares (Malevolent Designs), the last two with the Veil as state religion; needed for the Demons'
Altar. A Sheaim Planar Gate in the city can call a Tar Demon; the Order-Veil strife events can tear
it down.

**Playing it well.** As any temple, in the cities of a Veil empire, and first in the city where the
Demons' Altar and the Beasts of Agares will come.

**AI: Valued.** Research, culture modifier, slots and reagent happiness are read; it is the
missionaries' building for a Veil state. That it opens the Demons' Altar does not matter, since
the Altar is never built.

### Theatre (`BUILDING_THEATRE`)
*City building · 250 hammers · Drama · +3 culture, +1 happy per 10% culture rate*

**What it does.** +3 culture, a Bard slot, +1 happy from Dye, +1 happy with Consumption, and +1
happy for every 10% of the culture rate; needed for the Theatre of Dreams.

**Playing it well.** A happiness building for an empire that runs a culture slider (20-30% is 2-3
faces in every Theatre city), and a culture building for a cultural victory.

**AI: Partial.** The flat culture, slot and dye happiness are read; the happiness per culture rate
(`CommerceHappinesses`) is not, and the AI keeps the culture slider at 0 except when going for a
cultural victory (`victory::culture_slider`), so it rarely gets those faces. A culture building
for `victory::production`.

### Training Yard (`BUILDING_TRAINING_YARD`)
*City building · 100 hammers · Bronze Working · every civilization but the Luchuirp, Balseraphs and Sheaim · opens the melee line*

**What it does.** Needed to train Axemen, Champions, Phalanxes, Immortals, Berserkers and their
civilization versions (Swordsman, Moroi, Mimic, Ogre, Dragon Slayer, Demagog, Balor, Brujah,
golems...); +1 happy with Nationhood. Supplies can build it; every Infernal city has one for free.
The Sheaim have none (no tier-3 melee).

**Playing it well.** The melee army's barracks: one or two in the production cities, with copper
or iron connected. Warriors upgrade into Axemen once it exists.

**AI: Valued.** Value 0; it comes through `production::enabler_building`: an Axeman's 4 is a third
above a Warrior's 3, so cities whose best attacker is a Warrior build one, in at most half the
cities. Weighted per hammer as the unit choice is, a Warrior beat an Axeman and no Training Yard
was built (docs/AI_LOG.md, "An enabling building by strength squared per hammer"); the kept rule
is linear. `campaign::siege_building` can also build it for a Berserker-class unit (collateral)
when no city of ours can train a siege unit, and before Writing it can be
`production::fallback`'s quickest building.

### Walls (`BUILDING_WALLS`)
*City building · 120 hammers · Masonry · +25% defense, -25% bombard damage*

**What it does.** +25% city defense and 25% less damage to it from bombardment (neither against
gunpowder units). Defender leaders build it at double speed.

**Playing it well.** For border cities and the capital before the first war, and in cities that
barbarians reach; they make archers on a hill nearly untakeable without siege. Interior cities
never need them.

**AI: Valued.** The 25% defense is read, weakly: 12 points, 75 under threat
(`production::generic_building_value`), so a threatened city builds them quickly and a safe one
late; the bombard defense is not read.

### Warrens (`BUILDING_WARRENS`)
*Unique building, Clan of Embers · 180 hammers · Masonry · doubles the units trained in the city*

**What it does.** Every unit finished in the city that is not a world or national unit, siege or
naval gets a free copy (`ev_unit_built`), with the city's production experience; settlers and
workers too.

**Playing it well.** The Clan's war factory: build it early in the main military city and train
everything there. Two Ogres for one, two settlers for one in the expansion phase. It makes the
Clan's cheap units the best value in the game; heroes and siege get no copy.

**AI: Directed.** Its effect lives in Python, so the generic value is 0;
`ClanOfEmbers::building_value` (`civs/clan_of_embers.rs`) adds half the city's production as
hammers (150 a hammer), so it is built in productive cities. Measured: 3.4 Warrens a game by turn
300, 7.6 by the end; in the leave-one-out the Warrens cost share at turn 300 but paid later
(docs/AI.md, "Clan of Embers"). The AI does not pick which units go through them.

## World wonders

What every world wonder shares on the AI side, cited in the entries below: no code picks a city
for a wonder. The first city whose queue empties and finds it worth it starts it, through the
generic value (`production::generic_building_value`, score = value x 100 / (turns + 3), only within
50 turns, 12 while the city grows toward a settler), plus +300 for a leader's `FavoriteWonder` and
+400 for its favourite early or late wonder class. Once one is under way, workers chop forests
into it (`workers.rs`: chops go only to settlers, workers and world wonders) and a Great Engineer
rushes it when two thirds of the rush goes in (`units::great_person_build`). Under a cultural
victory goal the cities nearest legendary build culture wonders by their flat culture and
culture % (`victory::production`, `building_culture`). A city holding world wonders is worth +300
each as a conquest target (`campaign.rs`, city value).

### Aquae Sucellus (`BUILDING_AQUAE_SUCELLUS`)
*World wonder · 600 hammers · Sanitation, Life Mana connected to the city · every civilization ·
favourite of Sabathiel and Capria (Bannor)*

**What it does.** +2 health in the city and +1 in every city of its owner on the same continent;
units in the city heal 30% more a turn, and any unit there loses the Diseased promotion at the
start of the city's turn. (The FfH strategy text also promises cures for Withered and Plagued;
the data removes only Diseased.)

**Playing it well.** A health wonder for a wide empire on one landmass: it lets every city grow
one size further without Harbors or Aqueducts. The heal bonus makes the city a good staging and
recovery point behind a front, and it cleans up plague events and Diseased units. Its real price
is the Life mana it needs in the city, which also opens Life sphere spells; without a Life node
or a Ljosalfar, Luchuirp, Malakim or Mercurian palace it is out of reach.

**AI: Valued.** The generic value reads all three numbers (local, area and heal rate) and adds +300 for
the two Bannor leaders who favour it, so a healthy-capped city with Life mana builds it.
The AI never builds mana nodes now (only the Tower goal, which the AI no longer pursues, did: `victory::mana_node`), so it
gets the wonder only when its palace, a shrine or a pre-placed node supplies Life mana.

### Bone Palace (`BUILDING_BONE_PALACE`)
*World wonder · 700 hammers (double speed with Marble) · Philosophy · every civilization ·
favourite of Auric Ulvin (Illians)*

**What it does.** Starts a golden age when completed, +10 culture and 2 Great Engineer points in
the city, and units in the city resist magic 50% better.

**Playing it well.** A golden age on demand at Philosophy is the reason to build it: time it for a
moment when your cities are working big tiles or when you are swapping civics (the anarchy is
covered). With Marble it costs half. The 10 culture makes it a decent early border-pusher and a
cultural-victory piece, and the magic resistance protects a capital from enemy curses and
Domination.

**AI: Partial.** The generic value reads the 10 culture and the GPP (and +700 for Auric
Ulvin), so it gets built as a culture building; the golden age, its main effect,
and the magic resistance are invisible to it, and nothing times the completion.

### Catacomb Libralus (`BUILDING_CATACOMB_LIBRALUS`)
*World wonder · 600 hammers · Knowledge of the Ether, a Mage Guild in the city · every civilization
· favourite of Dain (Amurites) and Thessa (Ljosalfar)*

**What it does.** A free Mage Guild in every city of its owner, +4 research, +4 culture, one Sage
slot and 2 Great Sage points in the city.

**Playing it well.** The arcane civilizations' wonder: every city can train Adepts at once, which
turns a caster army from one guild city into an empire-wide production. Build it where the first
Mage Guild stands, in a strong production city, as soon as Knowledge of the Ether is in; for a
civilization with few casters it is only a modest research building.

**AI: Partial.** The generic value reads the research, culture, Sage slot and GPP; the free guilds
in every city, the point of the wonder, are not valued. It also needs a Mage Guild in the city,
which no AI builds for its value (0): only the Amurites and Svartalfar build one through
`civs::unique_casters`, and Infernal cities get one free, so in practice only they can start it.
Once it stands the free guilds do feed the AI's caster step (docs/AI.md, "Spellcasting").

### Celestial Compass (`BUILDING_CELESTIAL_COMPASS`)
*World wonder · 500 hammers · Astronomy · every civilization · favourite class of Varn Gosam,
Falamar and Hannah*

**What it does.** +50% production toward rituals (projects) in every city, +4 culture and 2 Great
Prophet points in the city.

**Playing it well.** Worth it only for an empire that plans rituals: the Illians' chain of winter
rituals (Samhain to Ascension), the Elohim's Hallowing, the Sheaim's Elegy, or a race for a world
ritual. Otherwise it is 500 hammers for 4 culture.

**AI: Incidental.** The generic value reads only the 4 culture and the GPP (+400 for its three
favouring leaders); the ritual bonus is not read, and the AI builds projects only when no building
is worth building (`production::best_project`). It is filler for an idle production city.

### City of a Thousand Slums (`BUILDING_CITY_OF_A_THOUSAND_SLUMS`)
*World wonder · 500 hammers · Engineering · every civilization*

**What it does.** The city works a radius-3 area (37 plots instead of 21), and its crime rises by
10 (crime only gates some random events in this game).

**Playing it well.** Build it in a capital or a high-food city with a lot of good land beyond its
normal reach, especially under civics that reward huge cities; it effectively merges a city's
worth of extra tiles into one city's multipliers. Check happiness and health first: the extra
tiles only pay if the city can grow into them.

**AI: Unused.** Nothing the generic value reads is on it and no leader favours it, so its value is
0 and production never picks it (Engineering comes long after Writing, so the pre-Writing fallback
never reaches it either).

### Crown of Akharien (`BUILDING_CROWN_OF_AKHARIEN`)
*World wonder, equipment · 900 hammers · Arcane Lore · every civilization · favourite of Valledia
(Amurites), favourite late wonder of six more playable leaders*

**What it does.** +100% research, +2 research, +2 culture and 2 Great Sage points in its city. It
is equipment: a unit in the city can take it (Take Crown of Akharien), carrying it as a promotion
that makes it immune to magic, and drop it into another of its cities (Drop Crown of Akharien),
where the wonder stands again. A carrier that dies leaves it on its plot as an equipment unit, which enemies capture by moving onto it.

**Playing it well.** Build it in the best research city, usually the capital with a Library and
the Great Library or a Sage specialist city. Leave it there: carrying it out is worth it only to
save it from a city about to fall, or to hand magic immunity to a hero for a decisive fight.

**AI: Valued.** The generic value reads the +100% research on the city's rate, the flat research
and culture, and the GPP, plus the favourite bonus of seven leaders, so a high-research city builds it. Taking it
out of the building is not allowed to the AI (the Take spell has no `bAllowAI`); an AI unit that
carries it (picked up as an equipment unit, which the AI may do) drops it in the first city of its own
it ends a turn in (`casting::ffh_value`: the building's value against a promotion worth 1).

### Form of the Titan (`BUILDING_FORM_OF_THE_TITAN`)
*World wonder · 500 hammers (double speed with Copper) · Warfare, a unit of level 6 · every
civilization · favourite of fourteen playable leaders*

**What it does.** Every unit its owner trains in any city starts with 2 more experience points;
2 Great Commander points in the city.

**Playing it well.** For a military empire: with a Training Yard or other experience buildings, +2
pushes new units over a promotion threshold, so every soldier leaves with one more promotion. It
needs a level-6 unit first, usually a hero or a veteran from the early wars. Copper halves the
cost.

**AI: Incidental.** The generic value reads `iExperience` (the city's own), not
`iGlobalExperience`, so the +2 for every city is invisible; what is left is 2 GPP (80) and the
favourite bonus of the many leaders who list it, which is what gets it built.

### Grand Menagerie (`BUILDING_GRAND_MENAGERIE`)
*World wonder · 120 hammers · Festivals; a Wolf Pen, Dancing Bear, Tiger Cage, Gorilla Cage and
Lion Cage in the city · every civilization*

**What it does.** A free Carnival in every city of its owner, +2 happiness in all its cities, +8
culture and +20% culture in its city and a free Bard specialist.

**Playing it well.** Cheap for what it gives, but the five animal buildings are the work: hunters
or recon units with Subdue Animal capture a wolf, bear, tiger, gorilla and lion and walk each into a
city that has a Carnival (the Dancing Bear does not need one), where they turn into the cages.
Plan it with a recon force early, while animals are still around.

**AI: Unused.** The AI never builds the cages: the only unit construction it orders is a great
person's shrine (`units::great_person_build`); captured animals fight as animals
(`units::role_of`, `UNITAI_ANIMAL`). Without the five cages the rules never let a city start it.

### Guild of Hammers (`BUILDING_GUILD_OF_HAMMERS`)
*World wonder · 550 hammers · Engineering · every civilization · favourite of eight playable
leaders (Garrim Gyr, Arturus Thorne and Kandros Fir above all)*

**What it does.** A free Forge in every city (+25% production and an Engineer slot each), +1
hammer from every Engineer and Great Engineer specialist in all cities, +8 culture and 2 Great
Engineer points.

**Playing it well.** One of the strongest economy wonders for a wide empire: it saves building a
Forge in every city and pays back many times over. Build it early in the best production city once
Engineering arrives, before a rival does.

**AI: Partial.** The generic value reads the 8 culture and the GPP (and the favourites); the free
Forges, its main effect, and the specialist hammers are not read.

### Guild of the Nine (`BUILDING_GUILD_OF_THE_NINE`)
*World wonder · 500 hammers · Currency · every civilization · favourite of Tasunke and
Rhoanna (Hippus)*

**What it does.** +25% city defense (not against gunpowder units), +4 culture and a Great
Commander point; and while its owner has it, any unit in one of its cities can cast Recruit
Mercenary: 180 gold for a Mercenary of the civilization's type, which cannot move that turn.

**Playing it well.** Mercenaries turn gold into soldiers where they are needed, at once: a
treasury becomes an emergency garrison when a city is attacked, or a late push without waiting for
production. Build it if you run a gold economy or expect a defensive war.

**AI: Partial.** The generic value reads the defense (more under threat), the culture and the GPP,
so it is built now and then; the mercenaries are never hired. The rules let an AI cast Recruit
Mercenary only with an enemy unit next to the caster and never as the Khazad, and FfH's terms then
value the spell at -4 a gold (-720) with nothing to set against it (`casting::ffh_value`), so no AI
casts it.

### Hall of Kings (`BUILDING_HALL_OF_KINGS`)
*World wonder · 600 hammers (double speed with Marble) · Feudalism · every civilization ·
favourite of Cassiel and Ethne*

**What it does.** +2 culture from every specialist in all cities of its owner, +10 culture and 2
Great Bard points in its city.

**Playing it well.** A specialist economy's wonder (Caste System, cities working Sages and
Priests): each specialist adds borders and culture everywhere. With Marble it is cheap culture for
a cultural victory.

**AI: Partial.** The generic value reads the 10 culture and the GPP; the culture from specialists,
the empire-wide part, is not read. It is also picked by the cultural-victory path for its flat
culture.

### Heron Throne (`BUILDING_HERON_THRONE`)
*World wonder · 250 hammers (double speed with Marble) · Fishing, a coastal city on 10+ water
tiles · every civilization*

**What it does.** +1 hammer on every water tile the city works, +25% city defense (not against
gunpowder) and +8 culture.

**Playing it well.** A cheap early wonder for a coastal capital or a city with mostly sea tiles:
the hammer on every ocean tile turns a fishing town into a production city. Build it early with
Fishing, before the AIs' coastal cities get to it.

**AI: Partial.** The generic value reads the culture and the defense; the sea hammers, its main
effect, are not read. At 250 hammers with 8 culture it is often built anyway, early, as one of the
cheaper culture buildings.

### Infernal Grimoire (`BUILDING_INFERNAL_GRIMOIRE`)
*World wonder, equipment · 400 hammers · Infernal Pact · every civilization · favourite of
Hyborem*

**What it does.** One free technology when completed, and a Great Sage point; when it is
completed there is a 21% chance that a barbarian Balor appears next to the city. It is equipment:
a unit can take it from the city (Take Infernal Grimoire) and carry it.

**Playing it well.** A free tech for 400 hammers is good value if your Infernal Pact is early;
have a strong garrison ready for the Balor. It is Hyborem's favourite, so expect the Infernals to compete for it.

**AI: Incidental.** The generic value reads only the GPP (40) and Hyborem's favourite
(+300); the free tech is not read. Taking it from the city is refused to AI players (neither Take
spell allows the AI).

### Mercurian Gate (`BUILDING_MERCURIAN_GATE`)
*World wonder · 600 hammers · Fanaticism, not in the capital, not with Ashen Veil as state
religion, not with No Hyborem or Basium · every civilization*

**What it does.** On completion the units in the city are moved out and a new player appears on
the builder's team: the Mercurians led by Basium, with Basium, a Settler, six Angels and two
Workers in the city. The new civilization is played by the builder's controller. The gate city
also gets -100% war weariness and every city -20%, +4 culture and 2 Great Prophet points.

**Playing it well.** For a good civilization it is an army of angels and a permanent ally for 600
hammers: build it in a minor city you can spare, since Basium's Convert City spell makes the gate
city the Mercurians' own, and use Basium and the Angels in your wars. Do not let an evil rival
build it.

**AI: Partial.** Built through the generic value thanks to its `iAIWeight` of 300 (FfH's flat
preference) plus culture and GPP, enough for it to be built often (the Mercurians appeared in 76
of 120 full games measured for `civs/mercurians.rs`); the spawn itself is not valued. Basium then
converts the gate city (Convert City, `iAIWeight` 1000, which FfH's terms cast at once).
The FfH Python's ban for evil AI players is not applied (docs/AI_ONLY_RULES.md), so an evil AI can
build it.

### Mines of Gal-Dur (`BUILDING_MINES_OF_GALDUR`)
*World wonder · 700 hammers · Arete, Runes of Kilmorph in the city and as state religion · every
civilization*

**What it does.** Provides 3 Iron, three Engineer slots, a Great Engineer point, and -2 health in
the city.

**Playing it well.** Three Iron is what matters: iron weapons for every melee unit and access to
iron units for a civilization without an iron mine, and two spare Iron to trade. Build it in a big
production city that can use the Engineer slots; mind the health.

**AI: Partial.** The generic value reads the Engineer slots (from size 5), the GPP and the -2
health, so a large Kilmorph city builds it; the Iron is invisible (free resources are not read,
and the AI's unit power ignores weapon promotions).

### Mokka's Cauldron (`BUILDING_MOKKAS_CAULDRON`)
*World wonder · 600 hammers · Necromancy, Death Mana connected to the city · every civilization*

**What it does.** A unit of its owner killed in the city comes back at once as its unholy version
(the Python's undead or demon counterpart), with the Demon promotion, half damaged, its moves
spent. +2 culture.

**Playing it well.** Build it in a frontier fortress city of an evil civilization: every defender
lost there returns, which makes a siege of that city very expensive. It does nothing elsewhere.

**AI: Incidental.** The generic value reads only the 2 culture (80); no leader favours it. It is
built only as filler where Death mana is available.

### Pillar of Chains (`BUILDING_PILLAR_OF_CHAINS`)
*World wonder · 500 hammers · Taxation · every civilization · favourite of Flauros (Calabim)*

**What it does.** Unhappy citizens in the city produce +1 hammer each, civic anger is cancelled
for its owner's team, -10% war weariness in every city, -10 crime and +4 culture. While it stands the
Armageddon Counter is 4 higher.

**Playing it well.** An evil empire's tool: switch to the civics you want without the anger, and
let an overgrown capital turn its unhappy citizens into hammers. It also pushes the Armageddon
Counter, which an evil player wants anyway.

**AI: Incidental.** The generic value reads only the 4 culture (and +300 for
Flauros); none of its own effects are read.

### Prophecy of Ragnarok (`BUILDING_PROPHECY_OF_RAGNAROK`)
*World wonder · 500 hammers · Way of the Wicked · every civilization · favourite of the Sheaim
leaders and Hyborem*

**What it does.** Every unit trained in the city gets the Prophecy Mark promotion: the Armageddon
Counter rises by 1 while that unit lives and falls when it dies. +8 culture, a Priest slot, 2 Great
Prophet points.

**Playing it well.** For the Sheaim or another evil civilization racing the Armageddon Counter: make
it your unit factory, and every soldier it trains pushes the counter toward Armageddon spells and
the end of the world. A good civilization should take the city or kill its units.

**AI: Partial.** The generic value reads the culture, the Priest slot and the GPP (and +700 for the Sheaim leaders); the Prophecy Mark is invisible. FfH's AI-only rule that only evil AIs may
build it is not applied (docs/AI_ONLY_RULES.md), so good AIs build it too.

### Ride of the Nine Kings (`BUILDING_RIDE_OF_THE_NINE_KINGS`)
*World wonder · 600 hammers · Warhorses, Horse connected · every civilization · favourite of
Rhoanna*

**What it does.** Mounted units trained in its city start with 10 experience, and units trained
in or entering the city get Morale (+10% strength and city attack, which may wear off). +4 culture,
2 Great Commander points.

**Playing it well.** The Hippus' wonder: make the city your stable, and every horseman leaves with
three promotions (10 experience is level 4). Stacks marching through pick up Morale on the way to a war.

**AI: Partial.** The generic value reads the culture, the GPP and the mounted experience, but caps
experience by unit type at 4, so the 10 counts as 4; Morale (a free promotion) is invisible. No
code makes the city train the mounted units.

### Shrine of Sirona (`BUILDING_SHRINE_OF_SIRONA`)
*World wonder · 500 hammers (double speed with Marble) · Divination, Spirit Mana connected · every
civilization · favourite of Ethne*

**What it does.** Once a turn one living unit of its owner anywhere can cast Sirona's Touch to heal
15 damage. +10 culture and 2 Great Prophet points.

**Playing it well.** A free heal each turn for the hero or best unit in the field, worth most in a
long war; the 10 culture with Marble makes it a cheap culture wonder too.

**AI: Partial.** The generic value reads the 10 culture and the GPP, so it is built; Sirona's
Touch is never cast by the AI: the rules let an AI unit cast it only with 15+ damage, and it
creates nothing `magic.rs` models and has no AI weight, so FfH's terms value it at 0 and no cast
choice takes it.

### Soul Forge (`BUILDING_SOUL_FORGE`)
*World wonder · 600 hammers · Malevolent Designs · every civilization · favourite of Faeryl
and Alexis*

**What it does.** Provides one Death mana; every living unit killed within one plot of the city
(either side) adds its experience + 10 hammers to the city's production. -2 health, 2 Great
Engineer points.

**Playing it well.** Build it in a frontier city where wars will be fought: every kill around it
feeds production, and the Death mana opens Death spells and the Mokka's Cauldron. Good for evil
civilizations that fight at home.

**AI: Incidental.** The generic value reads the GPP and the -2 health (which can make it negative
in a city at its health cap); the mana and the production from kills are not read. Its two
favouring leaders build it for the +300.

### Syliven's Perfect Lyre (`BUILDING_SYLIVENS_PERFECT_LYRE`)
*World wonder, equipment · 900 hammers · Drama · every civilization · favourite of Keelyn,
Alexis and Flauros*

**What it does.** +100% culture, +4 culture and 2 Great Bard points in its city. Equipment, like
the Crown of Akharien: a unit can take it out and drop it in another city.

**Playing it well.** The cultural victory's centrepiece: put it in a city that already makes a lot
of culture and will be one of the three legendary cities. Moving it between cities is possible but
the doubling works best where the culture already is.

**AI: Valued.** The generic value reads the +100% culture on the city's rate and the flat culture,
and the cultural-victory path takes it in a city nearest legendary for the same reason
(`victory::production`). The AI cannot take it out of a city; a carrier drops it in its first own
city (as for the Crown of Akharien).

### Temple of Temporance (`BUILDING_TEMPLE_OF_TEMPORENCE`)
*World wonder · 650 hammers (double speed with Marble) · Religious Law · every civilization ·
favourite of five playable leaders*

**What it does.** +1 happiness in every city of its owner on the continent, +10 culture and 2
Great Bard points.

**Playing it well.** A happiness wonder for a big empire on one landmass, best with Marble. One
more citizen in every city is a lot in the mid-game.

**AI: Valued.** The generic value reads the area happiness (per city and again x30 for each other
city) and the culture, which are the wonder.

### The Eyes and Ears Network (`BUILDING_EYES_AND_EARS_NETWORK`)
*World wonder · 1500 hammers · Guilds, 5 Inns · every civilization · favourite late wonder of five leaders*

**What it does.** Each turn its owner gets every technology that at least three players it has
open borders with know (except Arete, Mind Stapling, Hidden Paths, Infernal Pact and Seafaring).
+10 crime.

**Playing it well.** For a diplomat with open borders to most of the world: the technologies
trickle in for free for the rest of the game, which lets research go into the techs nobody else
has. Expensive; worth it only with three or more partners who are ahead of you.

**AI: Incidental.** Nothing the generic value reads is on it; only the +400 favourite of its five
leaders (Perpentach, Jonas Endain, Sheelba, Alexis, Flauros) gives it a value, and only they build it,
once they have 5 Inns. The AI does sign open borders (`diplomacy.rs`), so the effect then works
for it.

### The Great Library (`BUILDING_GREAT_LIBRARY`)
*World wonder · 350 hammers (double speed with Marble) · Writing, 3 Libraries and one in the city ·
every civilization · favourite of Sandalphon, Cassiel and Valledia*

**What it does.** A free Sage specialist in the city, +1 research from every Sage and Great Sage in
all cities, +8 culture and 2 Great Sage points.

**Playing it well.** Build it in the capital or best research city once three Libraries stand: the
free Sage and the Sage bonus start a Great Sage farm, and the 8 culture helps early borders.

**AI: Partial.** The generic value reads the culture and the GPP; the free Sage (a free specialist
by type, not `iFreeSpecialist`) and the specialist research are not read.

### The Great Lighthouse (`BUILDING_GREAT_LIGHTHOUSE`)
*World wonder · 600 hammers · Sailing, a Lighthouse in a coastal city · every civilization ·
favourite of Falamar and Hannah*

**What it does.** +2 trade routes in every coastal city of its owner, +6 culture and 2 Great
Merchant points.

**Playing it well.** For an island or coastal empire, two extra trade routes per port is a large
commerce boost. Inland empires should skip it.

**AI: Partial.** The generic value reads the culture and the GPP; the coastal trade routes (a
different field from `iTradeRoutes`) are not read.

### The Nexus (`BUILDING_THE_NEXUS`)
*World wonder · 600 hammers · Pass Through the Ether · every civilization*

**What it does.** A free Obsidian Gate (+1 trade route, airlift) in every city, and a Great Sage
point.

**Playing it well.** Trade routes everywhere and the ability to airlift units between cities make
it strong for a wide empire, both economically and for moving defenders.

**AI: Incidental.** The generic value reads only the Great Sage point (40); the free gates are
invisible. With no leader favouring it, it is built only when nothing better remains.

### Theatre of Dreams (`BUILDING_THEATRE_OF_DREAMS`)
*World wonder · 500 hammers · Drama, 3 Theatres and one in the city · every civilization ·
favourite of Perpentach, Ethne, Einion and Cardith*

**What it does.** +1 happiness in all cities and one more in its own, +2 culture from every Bard and
Great Bard everywhere, two Bard slots, +8 culture and 2 Great Bard points. Only a quarter of
captures keep it.

**Playing it well.** Happiness for every city and a Bard economy: run Bards in the city for Great
Bards and culture. Good for a cultural victory.

**AI: Valued.** The generic value reads the happiness (x50 per other city), the Bard slots, the
culture and the GPP, nearly all it does; the culture from Bards elsewhere is not read.

### Tower of Complacency (`BUILDING_TOWER_OF_COMPLACENCY`)
*World wonder · 750 hammers · Mind Stapling, Octopus Overlords in the city and as state religion ·
every civilization*

**What it does.** No unhappiness in its city, -50% maintenance there, -10% military production,
a Great Commander point.

**Playing it well.** Build it in the biggest city of an Overlords empire: happiness no longer
limits its size. The military penalty means it should not be your unit factory.

**AI: Partial.** The generic value reads the maintenance saving and the -10% military production;
the end of unhappiness, its main effect, is not read.

### Tower of Eyes (`BUILDING_TOWER_OF_EYES`)
*World wonder · 700 hammers · Optics · every civilization · favourite of Varn Gosam,
Amelanchier and three more leaders*

**What it does.** A free Dungeon in every city (-10 crime and -25% war weariness each), units
trained in its city get Sentry (+1 sight), +3 culture and a Great Commander point.

**Playing it well.** A war-weariness wonder for a long war across many cities, and better sight for
the units of its city. Usually a low priority.

**AI: Incidental.** The generic value reads only the 3 culture and the GPP; the free Dungeons and
Sentry are invisible. Its five favouring leaders build it for the bonus.

## National wonders and limited buildings

### Altar of the Luonnotar (final) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_FINAL`)
*National wonder, victory building · 4000 hammers · Omniscience, the Exalted altar in the city ·
good and neutral civilizations only · never captured*

**What it does.** The seventh and last stage of the altar, built with hammers where the six stages
made by Great Prophets stand; it replaces the Exalted altar and, when the Altar of the Luonnotar
victory is on, wins the game. Its own effects: a golden age, +3 happiness in all cities, +3 hammers
from every Priest and Great Prophet specialist, three Priest slots, +14 experience for new
disciple units and Blessed (+1 holy strength, +20% against demons, lost in combat) for units
trained in or entering the city; +8 culture.

**Playing it well.** The religious victory of a good or neutral civilization: it needs six Great
Prophets spent on the stages (each after its tech: Mysticism, Philosophy, Priesthood, Fanaticism,
Righteousness), then Omniscience and 4000 hammers in that one city. Pick a high-production holy or
temple city for it early, run Priests there, and defend it: a captured altar is destroyed. Against
a rival on this path, take or raze that city.

**AI: Unused.** `victory.rs` still estimates the race (`victory::altar_eta`, only once the Exalted
stage stands) but never makes it the AI's own goal: `victory::plan_ours` drops the Tower of
Mastery and the Altar (the owner's decision of 2026-10-21, commit `3041631`), so
`victory::production` never asks for it, and the six stages need Great Prophets, which build only
shrines (`units::great_person_build`). What the AI does do: a rival whose Altar estimate is short is
named in `victory::plan`, and its altar city is worth +3000 as a campaign target
(`victory::rival_city_bonus`).

### Amurite Palace (`BUILDING_PALACE_AMURITES`)
*Palace (national wonder) · given free to the first city; 160 hammers to build elsewhere with 4+
cities · Amurites (replaces the Bannor Palace) · never captured*

**What it does.** Every civilization's first city gets its palace free. It makes the city the
capital: distance maintenance counts from it (as from any government centre), +1 hammer, +8
commerce, +2 culture, +1 happiness and -10 crime, and three free resources, for the Amurites
Metamagic, Fire and Body mana. When the capital falls the rules move the palace to the best
remaining city at once (`player_find_new_capital`); with four or more cities a player may also
build it in another city to move the capital.

**Playing it well.** The palace mana set a civilization's first spheres: a sphere I promotion
needs its mana, and every new arcane unit gets sphere I free for each mana type its owner has two
of (sphere II with three and Channeling II), so a second node of a palace mana is an early
priority for a caster empire. The Amurites start with Floating Eye, Haste and Fireball (Fire II)
in reach. Moving the palace is rarely worth 160 hammers, except after conquests shift the
empire's centre.

**AI: Not an AI choice.** The rules give and move it; production never builds a palace while the
capital stands (`production::building_allowed`), and because the rules move it when the capital
falls, that exception almost never applies, so the AI never relocates its capital. Its mana feeds
the casters' sphere choice (`magic::promotion_value` values a sphere by the spells it opens): the
Amurite adepts' Haste and Floating Eye in docs/AI_LOG.md ("Training casters for their spells").

### Balseraphs Palace (`BUILDING_PALACE_BALSERAPHS`)
*Palace · as the Amurite Palace · Balseraphs*

**What it does.** As the Amurite Palace, with Mind, Chaos and Air mana. **Playing it well.** Mind
gives Inspiration and later Charm Person and Domination, Chaos gives Dance of Blades: a court of
illusionists and enchanters.

**AI: Not an AI choice.** As the Amurite Palace.

### Bannor Palace (`BUILDING_PALACE_BANNOR`)
*Palace · the default palace (the others replace it) · Bannor*

**What it does.** As the Amurite Palace, with Law, Spirit and Earth mana, and -10% war weariness
in every city. **Playing it well.** Spirit (Courage, Hope) and Law (Loyalty, Valor) fit a
crusading army; Earth gives Wall of Stone for defense.

**AI: Not an AI choice.** As the Amurite Palace.

### Bazaar of Mammon (`BUILDING_BAZAAR_OF_MAMMON`)
*National wonder · 600 hammers · Mathematics · every civilization · never captured*

**What it does.** +100% gold in the city, three Merchant slots, 2 Great Merchant points, +10
crime.

**Playing it well.** The gold city: build it where the commerce is highest (the capital with
cottages, a shrine city), run Merchants there, and it funds a high research slider or rushes.

**AI: Valued.** The generic value reads the +100% on the city's gold rate, the Merchant slots and
the GPP, and the building counts as an economy building (`production::is_economy_building`), so a
strained empire builds it ahead of the army interleave.

### Calabim Palace (`BUILDING_PALACE_CALABIM`)
*Palace · as the Amurite Palace · Calabim*

**What it does.** As the Amurite Palace, with Body, Law and Shadow mana, and -10% war weariness in
every city. **Playing it well.** Haste and Regeneration (Body) and Blur (Shadow) for vampire
armies.

**AI: Not an AI choice.** As the Amurite Palace.

### Clan of Embers Palace (`BUILDING_PALACE_CLAN_OF_EMBERS`)
*Palace · as the Amurite Palace · Clan of Embers*

**What it does.** As the Amurite Palace, with Fire, Nature and Body mana, and -40% war weariness in
every city. **Playing it well.** The war weariness cut suits the Clan's endless wars; Body and
Nature give Haste and Treetop Defense, Fire leads to Fireball.

**AI: Not an AI choice.** As the Amurite Palace.

### Deruptus Brewing House (`BUILDING_BREWERY`)
*National wonder · 240 hammers · Crafting, a river · every civilization (not under the Crusade
civic) · a quarter of captures keep it*

**What it does.** Provides 2 Ale (a happiness resource), +10% commerce in the city with each of
Corn, Rice and Wheat, and dwarven units (the Dwarf promotion) trained there start with 2 more
experience.

**Playing it well.** An early happiness resource for the whole network, with a spare Ale to trade.
Build it in a river city with grain; a civilization with dwarven units also makes it their
training city.

**AI: Unused.** Nothing on it is a field the generic value reads (free resources and
resource-based modifiers are not), so its value is 0 and no AI chooses it; only the pre-Writing
fallback, which builds the quickest building whatever it does (`production::fallback`), could
start it.

### Doviello Palace (`BUILDING_PALACE_DOVIELLO`)
*Palace · as the Amurite Palace · Doviello*

**What it does.** As the Amurite Palace, with Chaos, Earth and Body mana, and -40% war weariness
in every city. **Playing it well.** The war weariness cut keeps a warlike Doviello content; Body's
Haste helps their raids.

**AI: Not an AI choice.** As the Amurite Palace.

### Elohim Palace (`BUILDING_PALACE_ELOHIM`)
*Palace · as the Amurite Palace · Elohim*

**What it does.** As the Amurite Palace, with Spirit, Nature and Water mana, and +20% war
weariness in every city. **Playing it well.** A peaceful palace: Spirit's Hope and Water's Spring
build up cities, and wars cost the Elohim more happiness.

**AI: Not an AI choice.** As the Amurite Palace.

### Grigori Palace (`BUILDING_PALACE_GRIGORI`)
*Palace · as the Amurite Palace · Grigori*

**What it does.** As the Amurite Palace, with Spirit, Enchantment and Water mana, plus +15% great
people birth rate and 2 Adventurer points in the capital. **Playing it well.** The capital is the
Grigori's Adventurer farm; Enchantment's Enchanted Blade arms their melee.

**AI: Not an AI choice.** As the Amurite Palace.

### Heroic Epic (`BUILDING_HEROIC_EPIC`)
*National wonder · 200 hammers (double speed with Marble) · Military Strategy, a unit of level 4 ·
every civilization · never captured*

**What it does.** +100% production of military units in the city, +4 culture and a Great Commander
point.

**Playing it well.** Your military factory: build it in the best production city with the
Training Yard and other unit buildings, and train the expensive units there.

**AI: Valued.** The generic value reads the military production modifier on the city's hammers
(100 x the city's hammers / 4: 500 in a 20-hammer city) and the culture; no code then makes that city
the one that trains the army.

### Hippus Palace (`BUILDING_PALACE_HIPPUS`)
*Palace · as the Amurite Palace · Hippus*

**What it does.** As the Amurite Palace, with a Horse, Nature mana and Air mana.
**Playing it well.** The free Horse guarantees mounted units from the first city.

**AI: Not an AI choice.** As the Amurite Palace.

### Illian Palace (`BUILDING_PALACE_ILLIANS`)
*Palace · as the Amurite Palace · Illians*

**What it does.** As the Amurite Palace, with Ice, Enchantment and Law mana. **Playing it well.**
Ice (Slow, Snowfall) is the Illians' own sphere; Enchantment adds happiness and Enchanted Blade.

**AI: Not an AI choice.** As the Amurite Palace.

### Infernal Palace (`BUILDING_PALACE_INFERNAL`)
*Palace · as the Amurite Palace · Infernal*

**What it does.** As the Amurite Palace, with Entropy mana, Fire mana and Iron, and more: +50%
hammers and +25% defense in the capital, +1 gold, research and culture from every state religion
building, and no war weariness anywhere (-100%).

**Playing it well.** Hyborem's capital is a production city from the start, and the Infernals can
fight forever without war weariness; Iron arms their demons.

**AI: Not an AI choice.** As the Amurite Palace.

### Khazad Palace (`BUILDING_PALACE_KHAZAD`)
*Palace · as the Amurite Palace · Khazad*

**What it does.** As the Amurite Palace, with Earth mana, Gold and Law mana, plus +20% defense in
all cities and -10% war weariness. **Playing it well.** Gold is one more happiness resource; the
defense bonus suits a defensive style.

**AI: Not an AI choice.** As the Amurite Palace.

### Kuriotate Palace (`BUILDING_PALACE_KURIOTATES`)
*Palace · as the Amurite Palace · Kuriotates*

**What it does.** As the Amurite Palace, with Sun, Spirit and Water mana, plus +1 health in all
cities and +20% war weariness. **Playing it well.** The health helps the Kuriotates' few large
cities grow.

**AI: Not an AI choice.** As the Amurite Palace.

### Lanun Palace (`BUILDING_PALACE_LANUN`)
*Palace · as the Amurite Palace · Lanun*

**What it does.** As the Amurite Palace, with Water, Chaos and Air mana, plus +1 food on water
tiles in all cities. **Playing it well.** Coastal cities grow on the sea from the start.

**AI: Not an AI choice.** As the Amurite Palace.

### Ljosalfar Palace (`BUILDING_PALACE_LJOSALFAR`)
*Palace · as the Amurite Palace · Ljosalfar*

**What it does.** As the Amurite Palace, with Nature, Air and Life mana (Life adds health), and
+10% war weariness. **Playing it well.** Nature's Treetop Defense suits forest cities; Life mana
also opens Aquae Sucellus.

**AI: Not an AI choice.** As the Amurite Palace.

### Luchuirp Palace (`BUILDING_PALACE_LUCHUIRP`)
*Palace · as the Amurite Palace · Luchuirp*

**What it does.** As the Amurite Palace, with Enchantment, Life and Earth mana (happiness and
health), and +10% war weariness. **Playing it well.** Enchantment's Repair and Enchanted Blade for
golems, Earth for Wall of Stone.

**AI: Not an AI choice.** As the Amurite Palace.

### Malakim Palace (`BUILDING_PALACE_MALAKIM`)
*Palace · as the Amurite Palace · Malakim*

**What it does.** As the Amurite Palace, with Sun, Mind and Life mana, and +10% war weariness.
**Playing it well.** Sun gives Scorch (plains to desert, snow to tundra) and Blinding Light, Mind
gives Inspiration.

**AI: Not an AI choice.** As the Amurite Palace.

### Mercurian Palace (`BUILDING_PALACE_MERCURIANS`)
*Palace · as the Amurite Palace · Mercurians*

**What it does.** As the Amurite Palace, with Life mana, Earth mana and Iron, plus +2 gold, +2
research and 2 more culture in the capital, +40% military production there and -50% war weariness.
**Playing it well.** The Mercurians' first city (taken over from the Mercurian Gate's builder) is
their angel factory.

**AI: Not an AI choice.** As the Amurite Palace.

### National Epic (`BUILDING_NATIONAL_EPIC`)
*National wonder · 250 hammers (double speed with Marble) · Warfare, a Library in the city · every
civilization · never captured*

**What it does.** +100% great people birth rate in the city, +4 culture and a Great Commander
point.

**Playing it well.** Put it in the city with the most specialists and great people points (the
Great Library city is the classic choice) and it doubles your great people from there.

**AI: Valued.** The generic value reads the great people modifier (by the city's population) and
the culture, which is the wonder; it does not look for the city with the most great people points.

### Sheaim Palace (`BUILDING_PALACE_SHEAIM`)
*Palace · as the Amurite Palace · Sheaim*

**What it does.** As the Amurite Palace, with Death, Chaos and Fire mana, and -10% war weariness.
**Playing it well.** Death's Raise Skeleton and Fire's Fireball are the Sheaim's summoning start.

**AI: Not an AI choice.** As the Amurite Palace.

### Shrine of the Champion (`BUILDING_SHRINE_OF_THE_CHAMPION`)
*National wonder · 250 hammers · no tech; only after the civilization's hero has been trained and
is no longer alive · every civilization with a hero (not the Doviello, Grigori or Luchuirp) ·
never captured*

**What it does.** Units trained in the city start with one free promotion pick, +5 culture and 3
Great Prophet points.

**Playing it well.** A consolation for a dead hero: one more promotion on every unit from your
military city. Build it in the same city as the Heroic Epic.

**AI: Partial.** Once the rules allow it, the generic value reads the culture and the GPP, so it is
built; the free promotion, the point of it, is not read. The rules' hero table (`cf_get_hero`)
lists no hero for the Doviello, Grigori and Luchuirp, so they can never build it.

### Sidar Palace (`BUILDING_PALACE_SIDAR`)
*Palace · as the Amurite Palace · Sidar*

**What it does.** As the Amurite Palace, with Spirit, Enchantment and Shadow mana, plus +1 culture
from Bards, +1 research from Sages, +1 gold from Merchants and +1 hammer from Engineers (and their
great counterparts) in all cities. **Playing it well.** The Sidar's specialist economy starts with
the palace.

**AI: Not an AI choice.** As the Amurite Palace.

### Summer Palace (`BUILDING_FORBIDDEN_PALACE`)
*National wonder · 300 hammers · 8 cities · every civilization · never captured*

**What it does.** A second government centre: distance maintenance in the cities near it counts
from it instead of the capital. +4 culture, -10 crime.

**Playing it well.** Build it in the middle of a distant cluster of cities (an overseas colony, a
conquered region), where maintenance is highest.

**AI: Incidental.** The generic value reads only the 4 culture; the maintenance it saves in other
cities is not read (the value counts maintenance only through `iMaintenanceModifier` in the city
itself), and no code places it far from the capital.

### Svartalfar Palace (`BUILDING_PALACE_SVARTALFAR`)
*Palace · as the Amurite Palace · Svartalfar*

**What it does.** As the Amurite Palace, with Shadow, Nature and Mind mana, and -10% war weariness.
**Playing it well.** Shadow's Blur and Mind's Charm and Domination fit the Svartalfar's illusionists.

**AI: Not an AI choice.** As the Amurite Palace.

### Tower of Alteration (`BUILDING_TOWER_OF_ALTERATION`)
*National wonder, Tower of Mastery piece · 800 hammers · Alteration; Body, Life, Enchantment and
Nature mana all connected to the city · every civilization · never captured*

**What it does.** Its owner's units resist spells 20% better, and enemy units resist its spells 20%
less. One of the four towers the Tower of Mastery needs. (The four are in Civ4's "any of" resource list, which
FfH's rules read as "all of".)

**Playing it well.** A must for a caster civilization fighting other casters, and a step toward the
Tower of Mastery. The four manas are the work: convert raw nodes with adepts and trade for the
rest.

**AI: Unused.** Nothing the generic value reads is on it (value 0), and the Tower of Mastery is
never the AI's goal any more (`victory::plan_ours`, commit `3041631`), so `victory::production`
never builds it. A rival's tower cities are worth +3000 and +500 a tower as campaign targets when
its Tower estimate is short (`victory::rival_city_bonus`); a captured tower is destroyed.

### Tower of Divination (`BUILDING_TOWER_OF_DIVINATION`)
*National wonder, Tower of Mastery piece · 800 hammers · Divination; Law, Sun, Spirit and Mind
mana all connected · every civilization · never captured*

**What it does.** One free technology when completed; one of the four towers the Tower of Mastery
needs.

**Playing it well.** The free tech pays back part of its cost; build it when the four manas line
up, usually on the way to the Tower of Mastery.

**AI: Unused.** As the Tower of Alteration: value 0, never a goal.

### Tower of Mastery (`BUILDING_TOWER_OF_MASTERY`)
*National wonder, victory building · 4000 hammers · Strength of Will, the four lesser towers, the
Tower of Mastery victory enabled · every civilization · never captured*

**What it does.** Owning it together with the Towers of Alteration, Divination, Necromancy and the
Elements wins the Tower of Mastery victory. It has no other effect.

**Playing it well.** A magic empire's victory: sixteen manas for the four towers, 3200 hammers for
them and 4000 for this one in the best city. Defend the tower cities and push research to Strength
of Will; a rival on this path must lose one tower city to be stopped.

**AI: Unused.** Value 0, and `victory::plan_ours` never picks the Tower goal (the owner's decision
of 2026-10-21: a solitaire victory). `victory::tower_eta` still estimates rivals, so a civilization
close to it is named in `victory::plan` and its tower cities become campaign targets
(`victory::rival_city_bonus`).

### Tower of Necromancy (`BUILDING_TOWER_OF_NECROMANCY`)
*National wonder, Tower of Mastery piece · 800 hammers · Necromancy; Chaos, Death, Entropy and
Shadow mana all connected · every civilization · never captured*

**What it does.** Provides one Death mana, and every undead unit of its owner, existing and new,
gets Strong (+1 strength and defense). One of the four towers. (Its help also promises five more
skeletons; no rule in this game limits skeletons, so that line does nothing.)

**Playing it well.** For an undead army (the Sheaim, the Calabim, death casters): +1 on every
skeleton and spectre adds up.

**AI: Unused.** As the Tower of Alteration: value 0, never a goal.

### Tower of the Elements (`BUILDING_TOWER_OF_THE_ELEMENTS`)
*National wonder, Tower of Mastery piece · 800 hammers · Elementalism; Earth, Fire, Air and Water
mana all connected · every civilization · never captured*

**What it does.** Summons one permanent elemental (air, earth, fire or water at random) in the
city with the Held promotion, and every elemental of its owner, existing and new, gets Strong (+1
strength and defense). One of the four towers.

**Playing it well.** For an elementalist: stronger summoned elementals and a free guardian.

**AI: Unused.** As the Tower of Alteration: value 0, never a goal.

### Winter Palace (`BUILDING_WINTER_PALACE`)
*National wonder · 800 hammers (double speed with Marble) · Code of Laws, 12 cities · every
civilization · never captured*

**What it does.** A third government centre (see the Summer Palace) and +10 culture.

**Playing it well.** For a very large empire: put it at the far end from the capital and the Summer
Palace.

**AI: Partial.** The generic value reads the 10 culture (and the cultural-victory path its flat
culture); the maintenance relief is not read and no code places it.

## Buildings no city can build (palaces, granted and spell-made)

### Academy (`BUILDING_ACADEMY`)
*Granted building · no hammer cost · made by a Great Sage, or by the Controversial Philosopher
event · every civilization*

**What it does.** +50% research and +4 culture in the city. A Great Sage in a city can spend
itself to found one there (it is one of the Sage's forced buildings); the Controversial
Philosopher event offers one for 100 gold.

**Playing it well.** A Great Sage turned into an Academy in the capital or the Great Library city is
a permanent +50% research; early in the game it is often worth more than the one technology a
lightbulb gives. Pair it with the Library, the Crown of Akharien and Sage specialists.

**AI: Incidental.** The AI's Great Sages never found one: `units::great_person_build` builds only
shrines and rushes wonders, and a Sage otherwise lightbulbs, joins or starts a golden age. The AI
gets an Academy only from the event, whose choice values any building without a hammer cost at a
flat 300 gold (`events::building_value`, `SPECIAL_BUILDING_VALUE`), not by its research.

### Altar of the Luonnotar (`BUILDING_ALTAR_OF_THE_LUONNOTAR`)
*Altar stage 1 of 7 · made by a Great Prophet · good and neutral civilizations · one per player ·
never captured*

**What it does.** The first stage of the Altar of the Luonnotar victory: a Great Prophet spends
itself to found it in a city. +2 culture, a Priest slot, +2 experience for new disciple units. A
later stage, built by another Great Prophet in the same city, replaces it, and once a later stage
exists the earlier ones can no longer be built.

**Playing it well.** Only worth starting if you mean to finish: six Great Prophets and the final
4000 hammers. Choose the city once (a high-production temple city that is safe), because each
stage needs the one before in the same city. The stages themselves give modest culture and
happiness along the way.

**AI: Unused.** Great Prophets build only shrines (`units::great_person_build`: a building with
`HolyCity` and `GlobalReligionCommerce`); the altar has neither, and `victory::plan_ours` no longer
pursues the Altar victory (commit `3041631`). No AI ever founds it.

### Altar of the Luonnotar (anointed) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_ANOINTED`)
*Altar stage 2 · made by a Great Prophet · Mysticism, the first altar in the city*

**What it does.** As the first altar, one stage further: +3 culture, +1 happiness in the city, a
Priest slot, +4 experience for new disciples. **Playing it well.** The second Great Prophet of the
plan.

**AI: Unused.** As the Altar of the Luonnotar.

### Altar of the Luonnotar (blessed) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_BLESSED`)
*Altar stage 3 · made by a Great Prophet · Philosophy, the anointed altar in the city*

**What it does.** +4 culture, +1 happiness in all cities, +1 hammer from Priests and Great
Prophets in all cities, a Priest slot, +6 experience for new disciples, a Great Prophet point, and
Blessed (+1 holy strength, +20% against demons, lost in combat) for units trained in or entering
the city. **Playing it well.** From here the altar starts paying for itself: empire-wide happiness
and Blessed troops from the altar city.

**AI: Unused.** As the Altar of the Luonnotar.

### Altar of the Luonnotar (consecrated) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_CONSECRATED`)
*Altar stage 4 · made by a Great Prophet · Priesthood, the blessed altar in the city*

**What it does.** As the blessed altar with +5 culture, +2 happiness in all cities, two Priest
slots and +8 experience for new disciples. **Playing it well.** Keep the Great Prophet flow going.

**AI: Unused.** As the Altar of the Luonnotar.

### Altar of the Luonnotar (divine) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_DIVINE`)
*Altar stage 5 · made by a Great Prophet · Fanaticism, the consecrated altar in the city*

**What it does.** As the consecrated altar with +6 culture, +2 hammers from Priests and Great
Prophets in all cities and +10 experience for new disciples. **Playing it well.** The Priests'
hammers now make the altar city a production city as well.

**AI: Unused.** As the Altar of the Luonnotar.

### Altar of the Luonnotar (exalted) (`BUILDING_ALTAR_OF_THE_LUONNOTAR_EXALTED`)
*Altar stage 6 · made by a Great Prophet · Righteousness, the divine altar in the city*

**What it does.** As the divine altar with +7 culture, +3 happiness in all cities and +12
experience for new disciples; it is what the final altar needs. **Playing it well.** The last
prophet: from here it is Omniscience and 4000 hammers, so the city's production decides the race.

**AI: Unused.** As the Altar of the Luonnotar; `victory::altar_eta` starts counting only once this
stage stands, which no AI reaches.

### Code of Junil (`BUILDING_CODE_OF_JUNIL`)
*Shrine of the Order (world wonder) · made by a Great Prophet in the Order's holy city · every
civilization*

**What it does.** +1 gold for every city in the world that has the Order, a free Law mana, two
Priest slots and a Merchant slot, +4 culture, 2 Great Engineer points and -10 crime; it spreads the
Order from its city.

**Playing it well.** Every shrine is the main payoff of founding a religion: the gold grows with
every city the religion reaches, at home and abroad, so spread it with missionaries everywhere,
including to neighbours. Settle the first Great Prophet in the holy city for it, rather than for a
lightbulb.

**AI: Directed.** `units::great_person_build` sends a Great Prophet of ours to a holy city of ours
it can reach within 20 turns and founds the shrine there, before any lightbulb (docs/AI.md: 65
shrines in 24 full games, none before the rule). The AI takes any shrine it can construct; what
the shrine yields is not weighed, and missionaries spread only the state religion in our own
cities (`units::missionary`), never abroad where the shrine's gold would grow.

### Dancing Bear (`BUILDING_DANCING_BEAR`)
*Animal building · made by a captured Bear or Polar Bear · every civilization · a quarter of
captures keep it*

**What it does.** +1 happiness and +3 culture; one of the five buildings the Grand Menagerie
needs. Unlike the other cages it needs no Carnival.

**Playing it well.** Capture a bear with a Subdue Animal unit, walk it home and settle it into a
city: a cheap happiness, and a step to the Grand Menagerie.

**AI: Unused.** The AI never orders an animal to construct: the only construction it orders is a
great person's shrine (`units::great_person_build`). Its recon units favour Subdue Animal (its `iAIWeight`
of 300 counts in `units::promotion_value`), but captured animals fight as animals
(`units::role_of`, `UNITAI_ANIMAL`).

### Demonic Citizens (`BUILDING_DEMONIC_CITIZENS`)
*Marker building · given to every Infernal city · never captured*

**What it does.** Every city the Infernals found or take gets it (with an Elder Council, Training
Yard, Obsidian Gate, Forge and Mage Guild): no unhappiness, and population causes no
unhealthiness. Fires never burn it.

**Playing it well.** It is why Infernal cities ignore happiness and health: grow them as far as
food allows.

**AI: Not an AI choice.** The rules place it; the AI has nothing to decide.

### Dies Diei (`BUILDING_DIES_DIEI`)
*Shrine of the Empyrean (world wonder) · made by a Great Prophet in the Empyrean's holy city*

**What it does.** As the Code of Junil, for the Empyrean: +1 gold per Empyrean city in the world,
Sun mana, a Priest and a Sage slot, +2 experience for new disciples, -10 crime; it also reveals all
invisible units within its owner's borders and guarantees eligibility for diplomatic votes.

**Playing it well.** Besides the gold, it exposes invisible enemies inside your borders, including an Esus
empire's units under Nox Noctis.

**AI: Directed.** As the Code of Junil; seeing invisible units is not used by any AI logic beyond
what the units see.

### Dwarf Cage (`BUILDING_DWARF_CAGE`)
*Freak-show cage · made by a Balseraph slave with the Dwarf race · Balseraphs*

**What it does.** A Balseraph slave of dwarven race in one of their cities can be added to the
Freak Show (Add to Freak Show), which consumes it: +1 happiness, +1 culture and a Great Bard point
in that city, once per city.

**Playing it well.** Turn surplus slaves into permanent happiness and culture, one cage of each race
per city; keep the slaves you need as workers.

**AI: Partial.** The spell is an ability, valued by FfH's terms (`casting::ffh_value`): the cage's
full building value plus its `iAIWeight` of 30, and the sacrifice costs nothing because a slave has
no strength. So any Balseraph slave that ends its turn in one of their cities without that cage
casts it; nothing weighs the worker lost, and no code brings slaves to a city for it.

### Dwarven Vault (`BUILDING_DWARVEN_VAULT`)
*Khazad vault level · set by the rules each turn · Khazad · never captured*

**What it does.** Every turn each Khazad city gets the vault level matching the treasury's gold per
city: under 50 Empty (-2 happiness), 50-99 Low (-1), 100-149 this plain Vault (no
effect), 150-199 Stocked (+1), 200-299 Abundant (+2 and +10% hammers), 300-499 Full (+2 and +25%
hammers), 500 or more Overflowing (+3, +40% hammers and +25% great people).

**Playing it well.** The Khazad want a fat treasury: keep the gold per city high (fewer, larger
cities help), spend in bursts, and plan expansion knowing each new city dilutes it.

**AI: Not an AI choice.** The rules set the level; the AI does not steer its gold toward the next
level (saving for it was tried and dropped: docs/AI.md, the Khazad's vault target, score -5 +- 37
over 40 games).

### Dwarven Vault (Abundant) (`BUILDING_DWARVEN_VAULT_ABUNDANT`)
*Khazad vault level · 200-299 gold per city*

**What it does.** As the Dwarven Vault: +2 happiness and +10% hammers in every Khazad city.
**Playing it well.** A good level to hold in the mid-game.

**AI: Not an AI choice.** As the Dwarven Vault.

### Dwarven Vault (Empty) (`BUILDING_DWARVEN_VAULT_EMPTY`)
*Khazad vault level · under 50 gold per city*

**What it does.** As the Dwarven Vault: -2 happiness in every Khazad city. **Playing it well.**
Avoid it: an empty treasury costs every city two citizens' worth of happiness.

**AI: Not an AI choice.** As the Dwarven Vault.

### Dwarven Vault (Full) (`BUILDING_DWARVEN_VAULT_FULL`)
*Khazad vault level · 300-499 gold per city*

**What it does.** As the Dwarven Vault: +2 happiness and +25% hammers in every Khazad city.
**Playing it well.** A rich small empire lives here.

**AI: Not an AI choice.** As the Dwarven Vault.

### Dwarven Vault (Low) (`BUILDING_DWARVEN_VAULT_LOW`)
*Khazad vault level · 50-99 gold per city*

**What it does.** As the Dwarven Vault: -1 happiness in every Khazad city. **Playing it well.**
Climb out of it before growing the cities.

**AI: Not an AI choice.** As the Dwarven Vault.

### Dwarven Vault (Overflowing) (`BUILDING_DWARVEN_VAULT_OVERFLOWING`)
*Khazad vault level · 500+ gold per city*

**What it does.** As the Dwarven Vault: +3 happiness, +40% hammers and +25% great people in every
Khazad city. **Playing it well.** The top level; worth reaching with a few cities and a big
treasury.

**AI: Not an AI choice.** As the Dwarven Vault.

### Dwarven Vault (Stocked) (`BUILDING_DWARVEN_VAULT_STOCKED`)
*Khazad vault level · 150-199 gold per city*

**What it does.** As the Dwarven Vault: +1 happiness in every Khazad city. **Playing it well.**
The first positive level.

**AI: Not an AI choice.** As the Dwarven Vault.

### Elf Cage (`BUILDING_ELF_CAGE`)
*Freak-show cage · made by a Balseraph slave with the Elf or Dark Elf race · Balseraphs*

**What it does.** As the Dwarf Cage, for an elven or dark elven slave (two spells make it): +1
happiness, +1 culture, a Great Bard point. **Playing it well.** As the Dwarf Cage.

**AI: Partial.** As the Dwarf Cage.

### Freak Show (`BUILDING_FREAK_SHOW`)
*Balseraph unique building · made by a Freak · Balseraphs · never captured*

**What it does.** +1 happiness, +2 culture and a Great Bard point. A Freak (a Balseraph unit, 60
hammers, Festivals, strength 3) in one of their cities can spend itself to found it.

**Playing it well.** A cheap happiness and culture source for every Balseraph city: a Freak costs
60 hammers, less than most buildings with the same effect.

**AI: Unused.** The AI trains Freaks only as fighters (`production::best_unit`, their `UnitAIs`
are attack and counter types) and never orders them to construct
(`units::great_person_build` builds only shrines).

### Gorilla Cage (`BUILDING_GORILLA_CAGE`)
*Animal building · made by a captured Gorilla in a city with a Carnival*

**What it does.** As the Dancing Bear, for a gorilla, and it needs a Carnival in the city: +1
happiness, +3 culture; part of the Grand Menagerie. **Playing it well.** As the Dancing Bear.

**AI: Unused.** As the Dancing Bear.

### Hope (`BUILDING_HOPE`)
*Spell-made building · cast by a Spirit II caster in the city (Hope) · lost when no caster there
can cast it · never captured*

**What it does.** +1 happiness and +4 culture, and Courage (immune to fear, faster healing on its
plot) for units trained in or entering the city. The building disappears when no unit in the city
can cast Hope.

**Playing it well.** Park a Spirit caster in a city at its happiness cap or a new city that needs
culture; it is a free temple for as long as the caster stays.

**AI: Directed.** `magic.rs` has a rule for city spells: a caster in our city casts it at the end of
the turn for a quarter of its building value (`magic::turn_value`: here the happiness and culture;
Courage is not read), casters are sent to our cities lacking it when that quarter is worth 15 or
more (`magic::caster_target`), and a caster that alone keeps it stays unless twice as much waits
elsewhere (`magic::upkeep_value`).

### Human Cage (`BUILDING_HUMAN_CAGE`)
*Freak-show cage · made by a Balseraph slave with no race*

**What it does.** As the Dwarf Cage, for a human slave (one with no racial promotion): +1
happiness, +1 culture, a Great Bard point. **Playing it well.** As the Dwarf Cage.

**AI: Partial.** As the Dwarf Cage.

### Inspiration (`BUILDING_INSPIRATION`)
*Spell-made building · cast by a Mind I caster with Channeling I (Inspiration) · lost when no caster
there can cast it · never captured*

**What it does.** +2 research and a Great Sage point while a caster able to cast Inspiration stays
in the city.

**Playing it well.** Cheap research from an early adept with Mind mana, best in the capital or a
Sage city; leave the adept there in peacetime.

**AI: Directed.** As Hope: valued at a quarter of its research and GPP (55 points), which brings a
Mind caster to a city lacking it and keeps it there; without the keep rule casters recast
Inspiration a hundred times a game (docs/AI.md, "Spellcasting").

### Lion Cage (`BUILDING_LION_CAGE`)
*Animal building · made by a captured Lion or Lion Pride in a city with a Carnival*

**What it does.** As the Gorilla Cage, for a lion. **Playing it well.** As the Dancing Bear.

**AI: Unused.** As the Dancing Bear.

### Nox Noctis (`BUILDING_NOX_NOCTIS`)
*Shrine of the Council of Esus (world wonder) · made by a Great Prophet or Great Merchant in the
Council's holy city*

**What it does.** As the Code of Junil, for the Council of Esus: +1 gold per Esus city, Shadow mana,
a Bard and a Merchant slot, +2 experience for new recon units, +10 crime; it makes all its owner's
units invisible while they are inside its borders and outside cities (invisible units never
defend, but can attack), and guarantees vote eligibility.

**Playing it well.** Your army at home becomes invisible to the enemy and picks its fights against
invading stacks; since invisible units never defend, keep visible defenders in the cities. It is
an ambush wonder as much as an economic one.

**AI: Directed.** As the Code of Junil (a Great Merchant may found it too); the invisibility is not
used by any AI logic.

### Orc Cage (`BUILDING_ORC_CAGE`)
*Freak-show cage · made by a Balseraph slave with the Orc race*

**What it does.** As the Dwarf Cage, for an orcish slave. **Playing it well.** As the Dwarf Cage.

**AI: Partial.** As the Dwarf Cage.

### Song of Autumn (`BUILDING_SONG_OF_AUTUMN`)
*Shrine of the Fellowship of Leaves (world wonder) · made by a Great Prophet or Great Bard in the
Fellowship's holy city*

**What it does.** As the Code of Junil, for the Fellowship of Leaves: +1 gold per Leaves city,
Nature mana, two Priest slots and a Bard slot, +4 culture, 2 Great Bard points.

**Playing it well.** The elves' natural shrine; the Nature mana also opens Treetop Defense.

**AI: Directed.** As the Code of Junil (a Great Bard may found it too, unless a cultural victory sends
it to perform a great work first: `victory::great_work`).

### Spider Pen (`BUILDING_SPIDER_PEN`)
*Animal building · made by a captured Giant Spider · Animal Handling*

**What it does.** Provides one Silk (a happiness resource).

**Playing it well.** A spider caught with Subdue Animal is a permanent luxury.

**AI: Unused.** As the Dancing Bear.

### Stigmata on the Unborn (`BUILDING_STIGMATA_ON_THE_UNBORN`)
*Shrine of the Ashen Veil (world wonder) · made by a Great Prophet or Great Sage in the Veil's
holy city*

**What it does.** As the Code of Junil, for the Ashen Veil, but its religion commerce is +1
research per Veil city; Entropy mana, two Priest slots and a Sage slot, +4 culture; units trained
in its city get Stigmata (a strength bonus that grows with the Armageddon Counter); while it stands
the Armageddon Counter is 5 higher.

**Playing it well.** For the Veil's evil empire: research from every Veil city and units that grow
stronger as the world ends.

**AI: Directed.** As the Code of Junil (a Great Sage may found it too); the Stigmata promotion and
the counter are not weighed.

### Tablets of Bambur (`BUILDING_TABLETS_OF_BAMBUR`)
*Shrine of the Runes of Kilmorph (world wonder) · made by a Great Prophet in the Runes' holy city*

**What it does.** As the Code of Junil, for the Runes of Kilmorph: +1 gold per Kilmorph city, Earth
mana, two Priest slots and an Engineer slot, +4 culture, 2 Great Engineer points.

**Playing it well.** The dwarves' shrine; Earth mana adds Wall of Stone and Stoneskin.

**AI: Directed.** As the Code of Junil.

### The Dragon's Hoard (`BUILDING_THE_DRAGONS_HORDE`)
*World wonder, equipment · no hammer cost · placed in the barbarian city that trains Acheron ·
favourite of Jonas Endain and Sheelba (moot: no city can build it)*

**What it does.** When a barbarian city trains Acheron the Red Dragon, the Hoard appears there. In
a barbarian city it spawns a Disciple of Acheron (a Son of the Inferno once they have Sorcery) on a
turn with a chance equal to the difficulty's lair spawn rate. For a civilization that takes the
city (it always survives capture): +4 gold, +8 culture, Gems and Gold resources, and Enchanted
Blade (+20% strength and defense for melee) for units trained in or entering the city. It is
equipment, like the Crown of Akharien.

**Playing it well.** Killing Acheron and taking his city is one of the big prizes of the
mid-game: two luxuries, gold and an Enchanted Blade barracks. Bring a strong stack; the spawned
disciples defend it.

**AI: Not an AI choice.** The rules place it. An AI gets it by taking the city, which its
campaigns value like any wonder city (+300 a world wonder in `campaign.rs`; barbarian cities near
ours are targets); the AI cannot take it out of the building, and a carrier drops it in its first
own city.

### The Necronomicon (`BUILDING_THE_NECRONOMICON`)
*Shrine of the Octopus Overlords (world wonder) · made by a Great Prophet in the Overlords' holy
city*

**What it does.** As the Code of Junil, for the Octopus Overlords: +1 gold per Overlords city,
Water mana, two Priest slots and a Sage slot, +4 culture, 2 Great Prophet points; while it stands
the Armageddon Counter is 3 higher.

**Playing it well.** As the other shrines: spread the Overlords to collect the gold.

**AI: Directed.** As the Code of Junil.

### Tiger Cage (`BUILDING_TIGER_CAGE`)
*Animal building · made by a captured Tiger in a city with a Carnival*

**What it does.** As the Gorilla Cage, for a tiger. **Playing it well.** As the Dancing Bear.

**AI: Unused.** As the Dancing Bear.

### Unyielding Order (`BUILDING_UNYIELDING_ORDER`)
*Spell-made building · cast by an Order caster with Channeling III and Divine (Unyielding Order) ·
lost when no caster there can cast it · never captured*

**What it does.** The cast ends the city's resistance (occupation) at once and cuts its hurry anger
by 9 turns; the building then gives no unhappiness at all, -100% maintenance and -10 crime while
the caster stays.

**Playing it well.** The Order's tool for a freshly conquered city: no resistance, no unhappiness,
no upkeep while the priest stands there. Bring the high priest behind the army.

**AI: Partial.** As Hope (`magic::turn_value`, `caster_target`, `upkeep_value`), but the value reads
only the maintenance saving; the end of unhappiness and of resistance, its main effects, are not
read, so the caster goes where it saves most upkeep, not to the city just taken.

### Wall of Stone (`BUILDING_WALL_OF_STONE`)
*Spell-made building · cast by an Earth I caster with Channeling I (Wall of Stone) · lost when no
caster there can cast it · never captured*

**What it does.** +25% city defense and -25% damage to defenses from bombardment (neither against
gunpowder units), while a caster able to cast it stays in the city.

**Playing it well.** An instant wall for a threatened city: send an Earth adept in before the
siege and keep it there for as long as the wall should stand.

**AI: Incidental.** City spells are valued with no threat (`building_value(.., 0)`), so its value is
a quarter of 25 / 2, about 3 points: too little for `magic::caster_target` (15) to move a caster
for it, under siege or not. It is cast only when an Earth caster already stands in a city and
nothing else is worth casting.

### Wolf Pen (`BUILDING_WOLF_PEN`)
*Animal building · made by a captured Wolf or Wolf Pack in a city with a Carnival*

**What it does.** As the Gorilla Cage, for a wolf, but only +2 culture (no happiness).
**Playing it well.** As the Dancing Bear.

**AI: Unused.** As the Dancing Bear.

## Projects and rituals

### Ascension (`PROJECT_ASCENSION`)
*Illian ritual, one per game · 1200 hammers · Omniscience, and someone must have completed The Draw*

**What it does.** Creates Auric Ascended (30/30 with 30 cold damage, 3 moves, flying, the Avatar
promotion) in the city (`ev_ascension`). In answer, the strongest rival (a random roll plus rank,
humans first) receives the Godslayer, the one weapon made to kill him, at its capital (a human
through the Godslayer event).

**Playing it well.** The Illians' end game: Auric Ascended wins wars alone, but the road there is
The Draw, which halves the empire and declares war on everyone. Plan it as a final conquest:
the army ready, The Draw done, Omniscience in, and the city able to build 1200 hammers in a few
turns; then keep Auric away from whoever holds the Godslayer.

**AI: Unused.** `production::best_project` would take it (iAIWeight 1000), but it needs The Draw,
which the AI never builds (excluded from `best_project`, and `civs/illians.rs` builds only The White
Hand and Stir from Slumber), so it is never available.

### Bane Divine (`PROJECT_BANE_DIVINE`)
*Ritual, one per game · 10000 hammers (double speed with Death or Entropy mana) · Malevolent Designs, Armageddon counter 70+*

**What it does.** Kills every disciple unit in the world, the builder's own included
(`ev_project_built`; the strategy text's "level 1 prophets" is not what the rules do). It needs an
Armageddon count of 70.

**Playing it well.** A weapon of an evil empire against religious ones: every unit of the Disciple
combat type dies at once (missionaries, priests, Crusaders, Paramanders, Druids and the like).
Build it when the enemy's army leans on disciples and yours does not; with Death and Entropy mana
the cost halves.

**AI: Unused.** Excluded from `production::best_project` ("rituals with large side effects: left to
dedicated logic"), and no dedicated logic exists, so it is never built.

### Birthright Regained (`PROJECT_BIRTHRIGHT_REGAINED`)
*Ritual, one per game · 700 hammers · Omniscience · only after its builder has cast its world spell*

**What it does.** Lets the builder cast its civilization's world spell again (it clears the
world-spell feat; the rules refuse it until that spell has been cast, `py_cannot_create`).

**Playing it well.** Worth it for the civilizations whose world spell wins wars or turns a game
(Stasis, Wild Hunt, Rally, Raging Seas, Warcry, Into the Mist): time the second cast as carefully
as the first.

**AI: Unused.** Its iAIWeight is 0, so `production::best_project` never takes it, and nothing else
does.

### Blood of the Phoenix (`PROJECT_BLOOD_OF_THE_PHOENIX`)
*Ritual, one per game · 1000 hammers (double speed with Life mana) · Divine Essence*

**What it does.** Gives the Immortal promotion to every living unit the builder has at that moment:
each comes back once at the capital after being killed.

**Playing it well.** Build it with the largest army you will have, right before the decisive war:
every unit gets a second life. Units trained after it get nothing.

**AI: Incidental.** Built only through `production::best_project` (iAIWeight 300, within 40
turns), which runs only when the city has no building with a positive value left, so it comes
whenever a productive city runs out of buildings, not when the army is large.

### Elegy of the Sheaim (`PROJECT_ELEGY_OF_THE_SHEAIM`)
*Sheaim ritual, repeatable · 600 hammers · Way of the Wicked · +5 to the Armageddon counter*

**What it does.** Raises the Armageddon counter by 5, each time it is built.

**Playing it well.** The Sheaim gain from a high counter (their Planar Gates call more creatures, and
the counter's events hurt others), so they build it repeatedly in a spare city, watching the
thresholds' disasters.

**AI: Unused.** iAIWeight 0: `production::best_project` never takes it.

### Genesis (`PROJECT_GENESIS`)
*Ritual, one per game · 1250 hammers (double speed with Nature mana) · Commune with Nature · not for the Illians*

**What it does.** Warms all the builder's land by one step (`cf_genesis`): snow to tundra, tundra to
plains, desert and burning sands to plains, plains, fields of perdition and broken lands to
grassland; unimproved original grassland grows forest; plot counters drop by 100 (hell terrain
recedes).

**Playing it well.** Big for an empire on poor land or one spoiled by the Armageddon's hell terrain:
every desert and tundra tile becomes farmable. Build it once the land is claimed, with Nature mana;
note that it plants forest on unimproved grassland.

**AI: Incidental.** Only `production::best_project` (iAIWeight 100), when a city has nothing better
to build; its terrain effect is not valued.

### Glory Everlasting (`PROJECT_GLORY_EVERLASTING`)
*Ritual, one per game · 10000 hammers (double speed with Law or Life mana) · Righteousness, Armageddon counter 70+*

**What it does.** Kills every unit in the world with the Demon or the Undead promotion, the
builder's own included (`ev_project_built`).

**Playing it well.** A good empire's answer to the Infernal, the Sheaim's summons and undead armies:
build it when the enemy's army is mostly demons and undead, and yours is not.

**AI: Unused.** Excluded from `production::best_project` with the other dangerous rituals; no other
logic builds it.

### Hallowing of the Elohim (`PROJECT_HALLOWING_OF_THE_ELOHIM`)
*Elohim ritual, repeatable · 600 hammers · Way of the Wise · -5 to the Armageddon counter*

**What it does.** Lowers the Armageddon counter by 5 each time (it needs a counter of at least 1).

**Playing it well.** The Elohim's way to hold back the Armageddon: keep a spare city on it when the
counter approaches a threshold you want to avoid.

**AI: Unused.** iAIWeight 0: `production::best_project` never takes it.

### Nature's Revolt (`PROJECT_NATURES_REVOLT`)
*Ritual, one per game · 1000 hammers (double speed with Nature mana) · Animal Handling*

**What it does.** Every barbarian worker, scout, warrior, hunter and axeman becomes three animals
(wolves, lions, tigers, bears) where it stands, and every animal in the world gets Heroic Defense I
and II and Heroic Strength I and II (`ev_natures_revolt`).

**Playing it well.** A disruptive ritual: it floods the world with strong animals, which hurts
whoever has the most exposed land and the weakest army. Build it when your own army is strong and
your rivals are spread thin; every animal of every player gets the promotions, yours included.

**AI: Unused.** iAIWeight 0: `production::best_project` never takes it.

### Pact of the Nilhorn (`PROJECT_PACT_OF_THE_NILHORN`)
*Ritual, one per game · 500 hammers · Cartography*

**What it does.** Three Hill Giants (7/7, bombard 15, Sentry, hidden nationality), named Larry,
Curly and Moe, join the builder in the city.

**Playing it well.** Cheap and early: three 7-strength units at Cartography are an early war
party, and their hidden nationality lets them raid without a declaration. Build it in a good
production city before anyone else does.

**AI: Incidental.** Only through `production::best_project` (iAIWeight 100), when a city has no
building left with a positive value, which is rare early, when it matters. The giants keep their
animal AI (`UNITAI_ANIMAL`) and play as animals do (`units::barbarian`: attack what they can beat
nearby, else wander), not with the army.

### Purge the Unfaithful (`PROJECT_PURGE_THE_UNFAITHFUL`)
*Ritual, one per game · 350 hammers · Priesthood, a state religion · +3 to the Armageddon counter*

**What it does.** Removes every religion but the state religion (holy cities excepted) and their
buildings from the builder's cities; each city falls into disorder for a turn per religion
removed plus one more half the time, one less for an Order state, so even a city that loses
nothing can riot for a turn (`ev_purge_the_unfaithful`).

**Playing it well.** For a theocracy whose cities hold rival religions that cost happiness or feed
an enemy's religious victory. Expect a turn or two of disorder everywhere; with only the state
religion present it buys nothing but the riots.

**AI: Unused.** Excluded from `production::best_project`; nothing else builds it.

### Rites of Oghma (`PROJECT_RITES_OF_OGHMA`)
*Ritual, one per game · 2500 hammers (double speed with Metamagic mana) · Strength of Will*

**What it does.** Places new raw mana nodes on random free plots of the world (7 on a standard map;
4 on a duel map, up to 10 on a huge one) (`cf_add_bonus`).

**Playing it well.** For an arcane empire short of mana types (the Tower of Mastery needs many):
the nodes appear anywhere, so the builder needs casters ready to claim the ones in its land.

**AI: Unused.** iAIWeight 0: `production::best_project` never takes it.

### Samhain (`PROJECT_SAMHAIN`)
*Illian ritual, one per game · 200 hammers · no tech*

**What it does.** Every Illian city gets 20 turns of +1 happiness, and barbarian frostlings appear
around the world: (civilizations alive + difficulty index - 5) groups of two Frostlings, a
Frostling Archer and a Frostling Wolf Rider (four groups with six civilizations at Noble), plus the
frostling hero Mokka (`ev_project_built`, `cf_add_unit`).

**Playing it well.** The Illians' first ritual and a cheap one: the frostlings harass everyone, the
Illians included, so build it when your cities are defended and your rivals' are not, early enough
that the barbarians slow their expansion.

**AI: Unused.** Excluded from `production::best_project`, and `civs/illians.rs` does not build it,
so the AI never casts the first spell of winter.

### Stir from Slumber (`PROJECT_STIR_FROM_SLUMBER`)
*Illian ritual, one per game · 900 hammers · Divine Essence, and the builder must have eliminated a player*

**What it does.** Drifa the White Dragon (14/14, 7 cold damage, 3 moves, first strike, magic immune)
joins the builder in the city. The rules refuse it until the builder has eliminated another
player (`py_cannot_create`).

**Playing it well.** A dragon for the Illians' late wars: plan it after the first conquest, from
the best production city.

**AI: Directed.** `civs/illians.rs` (`ritual`) builds it in the most productive city as soon as the
rules allow, when not threatened or broke; it is excluded from `best_project`. In the measured
games it was never built: it needs Divine Essence and an eliminated player (docs/AI_LOG.md, "Ritual
projects"). Drifa plays as an army unit.

### The Deepening (`PROJECT_THE_DEEPENING`)
*Illian ritual, one per game · 600 hammers · Priesthood*

**What it does.** Cools the world for a while (`ev_the_deepening`): about a quarter of the land
plots change for 10 to 89 turns at normal speed (tundra to snow, grassland, marsh and plains to
tundra, desert to plains), and a few of them get a Blizzard.

**Playing it well.** Snow and tundra cost every civilization food, while snow gives the Illians 2
food: build it when rivals are growing, ideally with Samhain's frostlings out, to slow everyone but
yourself.

**AI: Incidental.** Only through `production::best_project` (iAIWeight 300) when a city has nothing
better to build; `civs/illians.rs` does not build it, so it comes late if at all.

### The Draw (`PROJECT_THE_DRAW`)
*Illian ritual, one per game · 900 hammers · Strength of Will · +10 to the Armageddon counter*

**What it does.** The builder declares war on every other living, non-vassal team and can no longer
talk with its enemies; the damage of every unit of its own doubles, to at least 50% (at most 99%), and
every city loses half its population (`ev_the_draw`). It opens Ascension.

**Playing it well.** Only as the first step of Ascension, with the army healed, the cities grown and
the 1200 hammers for Ascension ready right after; on its own it is a disaster.

**AI: Unused.** Excluded from `production::best_project`, and `civs/illians.rs` does not build it;
without it, Ascension is out of reach too.

### The White Hand (`PROJECT_THE_WHITE_HAND`)
*Illian ritual, one per game · 400 hammers · Philosophy*

**What it does.** Three Priests of Winter (5/5 with 2 cold damage, Channeling I and II, Ice I, Medic I
and II, Divine), named Dumannios, Riuros and Anagantios, join the builder in the city.

**Playing it well.** Three ice casters with healing for 400 hammers: they summon Ice Elementals and
heal the stacks. Build it as soon as Philosophy comes, and march them with the army.

**AI: Directed.** `civs/illians.rs` (`ritual`) builds it in the most productive city as soon as it
can (not when threatened or broke). Measured: built 0.6 a game, but late, at turns 325-372, because
Philosophy comes late, and the priests then mostly summoned Ice Elementals (docs/AI_LOG.md, "Ritual
projects"); it is back under the owner's rule that civilizations use their own mechanics.
