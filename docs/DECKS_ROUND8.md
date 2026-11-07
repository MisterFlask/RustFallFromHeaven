# Decks, round 8: 100 great-person candidates, 50 Armageddon candidates

**Status: design draft** (2026-11-07). Written against every verdict and principle in
`docs/DECKS.md` (sections 7, 10, 11.3, 12-15), then put through adversarial review (results at
the end of this file).

Conventions:
- **national, N** = N at the normal cost, then 500% each; **hard** = a true limit.
- **Bands:** I Ancient, II Classical, III Medieval.
- **world spell** = once per game.
- **ritual** = a project built in a city with production; **repeatable** = it may be built
  again after it completes.
- **Religion cards** (marked with a faith) can be drawn whatever your religion. Besides their
  effect, they spread that faith to three of your cities.
- **Stat scale:**

  | Unit | HP | Attack | Defence | Other |
  |---|---|---|---|---|
  | Champion | 120 | 60 | 60 | |
  | Paladin | 120 | 110 | 110 | |
  | Archmage | | | | magic 55 |
  | Hill Giant (large) | 180 | 70 | 70 | |
  | Greater Werewolf | 120 | 110 | 110 | |
  | Mithril Golem (large) | 180 | 250 | 250 | 1200 hammers |

## A. Great-person cards

### Sage (G01-G20)

**G01 Elemental Foundry** · II · needs Elementalism · national building
This city can train the elementals of the mana you hold: the existing Air, Earth, Fire, Water
and Ice Elementals, 180 hammers each, national 2 per element.
*Expert:* a mana-rich empire's tier-4 line without the summoning spells.

**G02 Mana Font** · II · needs Knowledge of the Ether · improvement on a mana node you own
The node counts as two of its mana for every rule that counts mana: unit affinities, and
buildings that need several of one mana.
*Expert:* double the sphere your casters live on.

**G03 Arcane Siegecraft** · II · needs Sorcery · promotion for casters of level 3+
In attacks on walled cities, the caster's damage spells strike the Sentinels first and deal
them double damage.
*Expert:* mages that open walls.

**G04 Evoker** · II · needs Sorcery · promotion for casters of level 4+
The caster's damage spells deal +50% damage, but it can no longer cast charms or heals.
*Expert:* the damage specialist, in an army that has another healer.

**G05 Abjurer** · II · needs Alteration · promotion for casters of level 4+
Its side is immune to hexes. At the start of each round, the enemy side's charms end.
*Expert:* the answer to charm-stacked casters.

**G06 Spellsword** · II · needs Alteration · promotion for melee units of level 3+
When the promotion is taken, choose one sphere I battle spell of a mana you hold. The unit
casts it, with its attack as its magic, in place of its first strike each round.
*Expert:* a front line that reaches the back row with a spell.

**G07 Astral Leap** · III · needs Pass Through the Ether · world spell
Move one stack from any of your cities to any land plot you can see, at once.
*Expert:* the surprise that takes an undefended capital.

**G08 Null Zone** · II · needs Divination · improvement, national (at most 3)
In battles on plots within 2 of it, neither side casts spells.
*Expert:* where your steel army meets the caster army.

**G09 Elemental Attunement** · II · needs Elementalism · building
Units trained here are immune to one damage type, chosen when the building is built: fire,
cold or lightning (the existing Immune Fire, Immune Cold, Immune Lightning promotions).
*Expert:* build the one your enemy's spells use.

**G10 Seal of Binding** · III · needs Strength of Will · world spell
Choose a world unit or a hero you can see. It is Held (the existing promotion: cannot move)
for 20 turns.
*Expert:* freeze the dragon or hero the enemy's war rests on.

**G11 Spell Thief** · III · needs Arcane Lore · promotion for casters of level 5+
When it kills a caster in battle, it gains one sphere promotion of the victim's that it lacks
(its owner's choice, without the mana requirement).
*Expert:* grow a mage by hunting mages.

**G12 The Patrian Awakening** · III · needs Arcane Lore · world spell (a new civ)
- **Requires:** the Remnants of Patria (`IMPROVEMENT_REMNANTS_OF_PATRIA`) outside every other
  civ's borders.
- **Effect:** the last Patrian archmages wake. A new civ, the Patrians (a new civilization of
  mages), founds a city on the Remnants and one more within 4 plots, as your vassal.
- **Hero:** the Archon, an Archmage with the Hero promotion and magic 80.
- **Mechanics:** every caster it trains starts at level 3 with Channeling II; its cities start
  with a Mage Guild.
- **Attitude:** +4 toward you.
- **Why it matters:** a hero above any mage of the era, and casters that start where others
  end.

**G13 The Great Conjunction** · II · needs Astronomy · world spell
Every unit of yours with magic gains one level.
*Expert:* cast it when your mage corps is at its largest.

**G14 Thunderhead Tower** · II · needs Elementalism · building
At the start of every battle at this city, each enemy unit in the rear row takes lightning
damage equal to 30% of its HP (lightning resistance applies).
*Expert:* punishes siege and archers before they fire.

**G15 Unseen Servants** · I · needs Mysticism · building
Workers build improvements in this city's radius twice as fast.
*Expert:* the developing city's crew.

**G16 Arcane Fortress** · III · needs Arcane Lore · national building
The city's Sentinels take no damage from bombards or siege stones; only spells and strikes harm
them.
*Expert:* a capital that siege engines cannot open.

**G17 Starmetal Meteor** · II · needs Astronomy · world spell
Call a meteor onto an unimproved land plot you can see, within 6 plots of your borders. What is
on it is destroyed, and the plot gains a Mithril resource.
*Expert:* Mithril for the civ that has none.

**G18 Gravity Well** · II · needs Alteration · promotion for casters with Earth I
Its battle hex stops enemy charges for the battle: mounted units and flankers can only strike
the front of a lane.
*Expert:* the answer to cavalry armies.

**G19 Magocracy** · III · needs Arcane Lore · civic (Government)
Casters cost 50% less; other units cost 25% more.
*Expert:* the mage empire's government.

**G20 Sunder Node** · II · needs Knowledge of the Ether · spell for casters
On a mana node of a civ you are at war with, destroy its mana improvement; the node returns to
raw mana.
*Expert:* take the enemy's sphere away before the battle.

### Prophet (P01-P20)

**P01 Living Walls** · Fellowship of Leaves · II · needs Way of the Forests · building
The city's two Sentinels fight as Treants (180 HP, attack 100) and strike in every round.
*Expert:* walls that hit back.

**P02 The Verdant Pact** · Fellowship of Leaves · III · needs Commune with Nature · world spell
Every unimproved land plot in your borders without a city grows a forest (jungle on hot
terrain).
*Expert:* the forest empire's fortress, cast before a defensive war.

**P03 Courts of Junil** · The Order · II · needs Orders from Heaven · national building
Cities you capture keep all their buildings and lose no population.
*Expert:* the lawful conqueror's prize.

**P04 Templar** · The Order · II · needs Orders from Heaven · national unit, 4
Tier 3 frontline (HP 130, attack 60, defence 70), holy damage, immune to fear. At the end of
each round, it heals the units in its lane 20 HP.
*Expert:* a front row that keeps itself alive.

**P05 The Whisperer** · Octopus Overlords · II · needs Message from the Deep · national unit, 2
A disciple with the battle hex *Madness*: in round 1, the enemy unit with the most attack
strikes its own side instead.
*Expert:* turn their champion against them.

**P06 Call the Leviathan** · Octopus Overlords · II · needs Message from the Deep · world spell
A Leviathan (the existing sea beast) joins you in the waters of a coastal city of yours.
*Expert:* one monster rules a sea.

**P07 Golems of the Earthmother** · Runes of Kilmorph · III · needs Mithril Working · rule
You can train Mithril Golems (Kilmorph's unit: large, 250 attack, 1200 hammers) whatever the
Armageddon counter. Its requirement of 70 is lifted for you.
*Expert:* the Kilmorph civ's ultimate unit in the middle game.

**P08 Vault of the Earthmother** · Runes of Kilmorph · II · needs Way of the Earthmother ·
national building
The city gains an Iron and a Mithril resource.
*Expert:* steel for an empire that has none.

**P09 Hellforge** · The Ashen Veil · II · needs Corruption of Spirit · building
This city can train Hellhounds (the Infernals' hunters, 60 hammers) and Balors (240 hammers).
*Expert:* demons for the evil civ that is not the Infernals.

**P10 Herald of Agares** · The Ashen Veil · III · needs Malevolent Designs · national unit, 1
(hard)
Tier 4 demon (HP 160, attack 120, defence 90). In every battle it fights, its side has Fear:
enemy units of lower level than the Herald may sit out.
*Expert:* the centrepiece of the evil army.

**P11 Halo** · The Empyrean · II · needs Honor · promotion for disciples of level 3+
Once per battle, its heal also revives one unit of its side killed earlier in that battle, at
25% HP.
*Expert:* the healer that undoes a loss.

**P12 Pillar of Dawn** · The Empyrean · III · needs Righteousness · ritual, repeatable (500
hammers)
For 10 turns, your units within 5 plots of the city heal fully each turn.
*Expert:* build it before the decisive siege near home.

**P13 Shadow Roads** · Council of Esus · II · needs Deception · worker build
A road that hides: your units on it are invisible to rivals.
*Expert:* move armies unseen to the border.

**P14 Mask of a Thousand Faces** · Council of Esus · II · needs Deception · world spell
For 10 turns, your units have Hidden Nationality: they may attack any civ without a
declaration of war.
*Expert:* a decade of unaccountable war.

**P15 Heaven's Gate** [good] · III · needs Divine Essence · ritual, repeatable (600 hammers),
only while the Armageddon counter is 40 or more
Each completion: an Angel (the existing unit) appears in the city.
*Expert:* the good civ's army grows as the world darkens.

**P16 Call of the Faithful** · any religion · III · needs Theology · world spell
Your state religion's two heroes (`religion_hero1`, `religion_hero2`) that are not alive in the
world join you in your holy city, or your capital.
*Expert:* the faith's champions, all at once.

**P17 Sanctified Steel** · any religion · II · needs Priesthood · building
Units trained here get Demon Slaying and Undead Slaying (the existing promotions).
*Expert:* the army for the war against the Infernals or the Sheaim.

**P18 Founding of the Dawn Knights** · The Order or the Empyrean [good] · III · needs
Righteousness · world spell (a new civ)
- **Effect:** a knightly order founds a fortress city (Wall of Stone, a temple of the card's
  faith) on a free site within 6 plots of your borders, as your vassal: the Knights of the
  Dawn, a new civilization.
- **Start:** 4 Paladins, and the hero the Grand Master (a Paladin with the Hero promotion,
  attack 130).
- **Mechanics:** its cities train Paladins at half cost. Its units heal fully after each
  battle they win against units of evil-aligned civs.
- **Attitude:** +4 toward good civs.
- **Why it matters:** tier-4 units and a hero from its first turn.

**P19 The Fey Court Awakens** · Fellowship of Leaves · III · needs Commune with Nature · world
spell (a new civ)
- **Requires:** an unclaimed forest region of 12 or more forest plots.
- **Effect:** the fey found two cities there as your vassal: the Fey Court, a new civilization
  of Satyrs, Fawns and Treants.
- **Hero:** the Erlking (large, tier 4, HP 160, attack 110, invisible in forests).
- **Mechanics:** its units are invisible in forest and jungle, and its cities' Sentinels fight
  as Treants.
- **Why it matters:** an army nobody sees in the woods, and a hero above its era.

**P20 Wings of Faith** [good] · III · needs Divine Essence · promotion for units of level 6+
The unit gains Flying: it crosses peaks and water, ignoring terrain.
*Expert:* veterans that cross any border.

### Engineer (E01-E20)

**E01 Master Masons' Guild** · II · needs Construction · national building
Palisades, Walls and Walls of Stone cost half in every city of yours.
*Expert:* wall the empire before the wars.

**E02 Gatehouse** · II · needs Construction · building
The city's Sentinels strike with the attack of the city's strongest defender.
*Expert:* walls that fight like champions.

**E03 Boiling Oil** · I · needs Masonry · building
At the start of every battle at this city, each enemy unit in the front row takes damage equal
to 25% of its HP.
*Expert:* the assault's first rank pays before it strikes.

**E04 Siege Ballista** · II · needs Mathematics · unit (tier 2 siege)
Its shots pierce: each one hits every unit in the target lane.
*Expert:* the answer to deep formations.

**E05 Trench** · I · needs Mining · worker build in your borders
In battles on this plot, the attacker has 3 fewer slots.
*Expert:* the narrow front at the border.

**E06 Sapper** · II · needs Construction · promotion for siege units of level 2+
Its bombard destroys one Sentinel outright, instead of damaging the walls.
*Expert:* two sappers breach any wall in a turn.

**E07 Wall of the Kings** · II · needs Engineering · world wonder
Enemy land units that enter your borders end their move there.
*Expert:* invasions crawl.

**E08 Mantlets** · II · needs Construction · promotion for melee units
Immune to defensive strikes and to the Sentinels' blows.
*Expert:* the assault rank.

**E09 Grand Siege Park** · II · needs Engineering · national building
Siege units trained here are large (two cells, +50% HP).
*Expert:* siege that survives the counter-attack.

**E10 Harbor Chain** · I · needs Sailing · building, coastal
Enemy ships cannot enter the water plots next to the city.
*Expert:* a coastal city no fleet can blockade.

**E11 Pioneers** · I · needs Crafting · national unit, 4
A worker that builds improvements three times as fast.
*Expert:* the early land rush.

**E12 Siege Lines** · II · needs Construction · worker build next to a city of a civ you are at
war with
Your units on it heal as in your own city.
*Expert:* a siege that never has to retreat to heal.

**E13 Powder Charge** · III · needs Blasting Powder · national siege unit, 2
When it attacks a walled city, it is destroyed, and every Sentinel of the city is breached.
*Expert:* the wall-opener for the decisive assault.

**E14 Moat** · I · needs Masonry · building
In round 1 of a battle at this city, the attacker's melee units cannot strike; only shots,
stones and spells.
*Expert:* the defender's archers get a free round.

**E15 Siege Colossus** · III · needs Machinery · national unit, 1 (hard)
A large siege engine that bombards a city from 2 plots away.
*Expert:* open the walls before the army arrives.

**E16 Avalanche Charges** · II · needs Mining · hidden improvement on a peak in your borders
The first enemy stack to enter a plot next to it takes damage equal to 40% of its HP. The
charge is then spent.
*Expert:* mine the mountain passes.

**E17 Bombard Barge** · II · needs Construction · ship
A ship that bombards a coastal city's Sentinels at a catapult's rate.
*Expert:* open the sea walls.

**E18 Terraced Hills** · I · needs Mining · worker build on hills
+2 food. The hill keeps its defence.
*Expert:* hill cities that grow.

**E19 Shatter the Walls** · III · needs Machinery · world spell
Every Sentinel of one city you can see is breached, and the city loses its walls.
*Expert:* the opening of the war's great siege.

**E20 Levy Hall** · II · needs Bronze Working · building
In every battle at this city, two levies (units of the city's best defender type) join the
defenders for that battle only.
*Expert:* the city that always has a garrison.

### Merchant (M01-M20)

**M01 Gold Standard** · III · needs Mercantilism · civic (Economy)
Rushing production costs 1 gold per hammer. Research -20%.
*Expert:* the rich empire buys its army.

**M02 Prize Courts** · II · needs Sailing · national building
Enemy ships that your ships defeat are captured instead of sunk.
*Expert:* a navy that grows by fighting.

**M03 Bazaar of Wonders** · III · needs Mercantilism · world spell
Buy one world wonder not yet built anywhere, completed at once in a city of yours, for 3 times
its cost in gold.
*Expert:* the wonder your rival is two turns from.

**M04 Grand Bazaar** · II · needs Trade · national building
The city gains one copy each of up to three luxury resources you lack, drawn from those that
exist in the world.
*Expert:* happiness for a narrow empire.

**M05 Hired Fleet** · II · needs Trade · world spell
Four of the best warships you can build appear in a coastal city of yours.
*Expert:* a navy for one decisive crossing.

**M06 Beast Market** · I · needs Hunting · building
This city can train Bears (60 hammers), Lions (50), Tigers (60) and Griffons (120), the
existing animals.
*Expert:* animal armies before bronze.

**M07 The Condottieri Free State** · III · needs Mercantilism · world spell (a new civ)
- **Effect:** a mercenary state founds two cities on free land within 8 plots of your borders,
  as your vassal: the Free Companies, a new civilization of mercenaries.
- **Start:** 6 Mercenaries and 1000 gold.
- **Hero:** the Captain-General (a Champion with the Hero promotion, attack 100).
- **Mechanics:** its cities make double gold, and it rushes units at 1 gold per hammer.
- **Why it matters:** an army on its first day, and an economy that buys more.

**M08 Purchase Charter** · III · needs Mercantilism · world spell
Buy one city of a civ you are at peace with, for 100 gold per population. It must not be the
capital, must have population 5 or less, and must lie within 6 plots of your borders. The city
keeps its buildings; its owner gets the gold and -4 attitude toward you.
*Expert:* the border city you want, without the war.

**M09 Transmutation Charter** · II · needs Alteration · world spell
Turn one resource plot of yours into another resource of the same class: strategic for
strategic, luxury for luxury, food for food.
*Expert:* the Iron you need, from the Copper you have.

**M10 Admiralty** · II · needs Sailing · national building
Ships trained here start with two promotions.
*Expert:* the navy's school.

**M11 Armourers' Guild** · II · needs Bronze Working · national building
Upgrading units costs no gold.
*Expert:* the big army that keeps up with the tech.

**M12 Patrons' Guild** · II · needs Currency · building
Great-people points in this city are doubled.
*Expert:* the great-people city.

**M13 Market Towns** · II · needs Currency · rule
Cottages grow into hamlets, villages and towns twice as fast in your borders.
*Expert:* the commerce empire's growth.

**M14 Regime Change** · III · needs Guilds · world spell
A civ you have met with less power than you adopts the Government civic you name, with 3 turns
of anarchy.
*Expert:* three turns without production or research for the rival, and a civic that suits
you.

**M15 Purchased Peace** · II · needs Currency · world spell
End one war you are in: a 10-turn peace is imposed on both sides, and you pay the other side
200 gold.
*Expert:* buy the decade you need.

**M16 Grand Exhibition** · II · needs Trade · world spell
You learn every tech known by at least three civs you have met.
*Expert:* the backward civ's catch-up.

**M17 Reserve Arsenal** · II · needs Bronze Working · national building
Up to 4 units may be stored here. Stored units cost no upkeep, cannot be attacked, and leave at
once when released.
*Expert:* an army in reserve for the war you expect.

**M18 Mammon's Bargain** [evil] · II · needs Currency · world spell
+3000 gold at once, and +5 to the Armageddon counter.
*Expert:* buy the war now; the world pays later.

**M19 Grand Order** · III · needs Guilds · world spell
Every city of yours completes its current production at once (not wonders or rituals).
*Expert:* time it to the turn your cities start their most expensive builds.

**M20 Golden Muster** · II · needs Currency · world spell
Spend all your gold: one unit of your best melee type per 150 gold appears in your capital.
*Expert:* the treasury becomes an army.

### Bard (B01-B20)

**B01 Cultural Diaspora** · II · needs Drama · world spell
For 20 turns, your culture counts triple in foreign cities within 5 plots of your borders, on
their plots and toward culture flips.
*Expert:* take the border cities' land and loyalty without a war.

**B02 Sing the Stones** · II · needs Drama · world spell
Every City Ruins inside your borders becomes a city of yours of size 1.
*Expert:* rebuild an old war's ruins at once.

**B03 Golden Epic** · I · needs Festivals · world spell
Every city of yours gains culture equal to 50 turns of its current culture rate.
*Expert:* borders jump at once; cast it after many temples.

**B04 Banshee Choir** · II · needs Drama · national unit, 2
A disciple whose presence gives its side Fear against units of tier 1 and 2: they may sit out
the battle.
*Expert:* the old armies stay home.

**B05 Shadow Theatre** · II · needs Drama · building
Units trained here are Hidden (the existing promotion) until they first fight.
*Expert:* an army nobody counted.

**B06 Cultural Annexation** · II · needs Philosophy · world spell
One foreign city that joins you with its buildings. It must not be a capital, must lie within 3
plots of your borders, and your culture must be at least 30% of the culture on its plot.
*Expert:* the city that already half belongs to you.

**B07 Legacy of Kings** · II · needs Philosophy · world spell
Your leader gains one more trait of your choice among the selectable ones: Philosophical,
Aggressive, Spiritual, Expansive, Industrious, Creative, Financial, Organized, Charismatic,
Arcane, Raiders.
*Expert:* pick the trait that makes your plan.

**B08 Masque of the Red Death** [evil] · III · needs Malevolent Designs · world spell
One city you can see, of a civ you are at war with, loses half its population.
*Expert:* the enemy capital, before the siege.

**B09 Bard's Retinue** · II · needs Drama · promotion for heroes
Units in the hero's stack gain double XP.
*Expert:* the hero's army becomes a veteran army.

**B10 The Heroes' Saga** · III · needs Strength of Will · world spell
Every hero of yours gains two levels.
*Expert:* cast it with many heroes alive.

**B11 The Unconquered City** · II · needs Code of Laws · building
While the city has never changed hands (the game keeps its original owner and the turn it was
acquired), its defenders deal +50% damage.
*Expert:* the old cities' pride; lost once, gone for good.

**B12 Feast of Plenty** · I · needs Festivals · world spell
Every city of yours gains 2 population.
*Expert:* cast it when happiness allows.

**B13 Dragon's Oath** · III · needs Animal Mastery · world spell
A new dragon, Sylvarith the Singing Wyrm, joins you: large, 180 HP, attack 140, defence 140,
flying, fire breath.
*Expert:* the late war's centrepiece.

**B14 Last Stand Hymn** · II · needs Drama · building
In a battle at this city, once half of your side has fallen, the rest are immune to fear and
deal double damage.
*Expert:* the city that fights hardest when losing.

**B15 Wardance** · II · needs Bronze Working · promotion for melee units of level 3+
Its strikes cleave: each also hits the units on both sides of its target.
*Expert:* the answer to tight formations.

**B16 Cult of Heroes** · III · needs Strength of Will · civic (Cultural Values)
Heroes and units of level 6+ gain double XP and +1 move.
*Expert:* the hero empire's civic.

**B17 Silver Tongue** · II · needs Philosophy · world spell
Every war you are in ends at once, and each side is held to 10 turns of peace (enforced).
*Expert:* the escape from a two-front war.

**B18 Anointed Champion** · II · needs Arete · world spell
One unit of yours gains the Hero promotion.
*Expert:* the veteran that carries the next war.

**B19 Wager of Glory** · II · needs Arete · world spell
Name a city of a civ you are at war with. If you capture it within 20 turns, every unit that
fought in its capture gains 2 levels. If not, your capital loses 2 population.
*Expert:* wager on the siege you are about to win.

**B20 Heartland Charter** · II · needs Code of Laws · building
Foreign culture cannot take plots within 2 of this city, and the city cannot flip by culture.
*Expert:* the border city that stays yours.

## B. Armageddon cards (A01-A50)

Each card names its threshold, which is its slot. Stages of a chain take the later slots they
name.

### Early (10-30)

**A01 The Wild Awakens** (20): every lair (Barrows, Bear and Lion Dens, Goblin Forts, Ruins)
spawns three of its units at once.
*Counter-play:* clear the lairs near your cities before 20.

**A02 The Sea Recedes** (30): every unimproved Coast or Shallows plot next to land becomes
grassland for good.
*Counter-play:* new land bridges and island links appear; settle and garrison them.

**A03 Collapse of the Roads** (30): every road outside the cities' radii is destroyed.
*Counter-play:* armies far from home are stranded; plan the logistics.

**A04 The Scaled Kingdom** (30, a new civ)
- **Effect:** lizardmen found a civ in the largest jungle or marsh region: three cities with
  Palisades and 6 Lizardman Rangers.
- **Hero:** the Slitherking (a Lizardman Assassin with the Hero promotion, attack 90).
- **Mechanics:** its units are jungle and marsh natives (they move freely there, with +50%
  defence).
- **Attitude:** -2 toward its neighbours.
- **Why it matters:** a hero, three cities, and ground nobody else fights well on.

**A05 The Gods Answer** (30): each religion's holy-city owner receives that religion's first
hero (`religion_hero1`), if it is not alive in the world.

**A06 The Brood** (30 → 50, chain)
- *30:* giant spider nests (lairs that spawn Baby Spiders) appear in the three largest forest
  regions.
- *50:* the nests found the Brood, a new civ. The three nest plots become cities, with 6 Giant
  Spiders and the hero the Spider Queen (large, 160 HP, attack 90, poisoned hits). Its cities
  train Giant Spiders at half cost.
- **Why it matters:** three cities in the forests, a hero, and a cheap large line.

### Middle (30-60)

**A07 The Burning Plains** (40): every plot inside evil-aligned civs' borders turns to hell
terrain (its plot counter jumps to the maximum).

**A08 The Great Flood** (40): every flat land plot next to a river loses its improvement and
becomes flood plains.

**A09 Volcanic Spring** (40): six volcanoes (the Volcano feature) erupt on hills named at the
reveal, near the six largest cities. Everything within 1 plot is destroyed; afterwards those
plots give +1 food.

**A10 Leviathans Rise** (40): a barbarian Leviathan appears in every ocean of 20 or more plots.

**A11 Werewolf Moon** (40): three Ravenous Werewolves appear near each civ's largest city, and
every barbarian Wolf becomes a Greater Werewolf.

**A12 The Gold Rot** (40): every civ's gold is halved.

**A13 The Great Fire** (40): every city of size 10 or more loses 2 population and its most
expensive building (not a wonder).

**A14 The Iron Blight** (50): every unit with Iron Weapons becomes Rusted (the existing
promotion).

**A15 Mana Collapse** (50): every mana node's improvement is destroyed; the nodes become raw
mana again.

**A16 The Fire Goes Out** (50, lingering): one sphere, drawn when the card is dealt and shown at
its reveal, stops working for 20 turns: its spells and battle spells fail.

**A17 Unbound Elementals** (50): next to every Fire, Water, Air, Earth and Ice mana node, a
barbarian elemental of that node's element appears.

**A18 The Great Tremor** (50): every Mine in the world collapses.

**A19 The Gargoyles Wake** (50): a barbarian Gargoyle appears next to every city with Walls or a
Wall of Stone, and those walls lose one level.

**A20 The Golem Rebellion** (60): every golem in the world (Wood, Mud, Clockwork, Iron, Flesh,
Bone, Mithril and Nullstone Golems) becomes barbarian.

**A21 The Desert Wakes** (60): two barbarian Sand Lions appear in every desert region of 10 or
more plots.

**A22 The Host of Nyx** (60, a new civ)
- **Effect:** the creatures of Agares's world rise in the largest unclaimed region: three cities
  on hell terrain with Walls, and 6 demons (Balors and Hellhounds).
- **Hero:** the Black-and-Gold Herald (a Balor with the Hero promotion, attack 140).
- **Mechanics:** its cities turn the land within 2 plots to hell terrain.
- **Attitude:** -4 toward good civs.
- **Why it matters:** a hero, demons, and a homeland that remakes the land around it.

**A23 The Shattering of Peaks** (60): every peak next to a city becomes a hill.

**A24 The Burning Libraries** (60): every civ loses its most recently learned tech; relearning
it costs half.

**A25 The Iron Rain** (60): an Iron and a Copper resource fall on unclaimed land within 8 plots
of each civ's capital.

**A26 The Four Winds** (20 → 40 → 60 → 80, chain)
- *Air (20):* every ship at sea loses half its HP.
- *Earth (40):* every city's walls lose one level.
- *Fire (60):* every forest next to a city burns.
- *Water (80):* every Fishing Boats and Whaling Boats improvement is destroyed.

**A27 The Unchaining of Orthus** (60): Orthus, if he is not in the world, appears with 10 orcs
(Ogres and Goblins) in the largest barbarian-held region, or next to the score leader if there
is none.

**A28 The Unseen War** (50, lingering): for 30 turns, every invisible unit in the world is
visible to all.

**A29 The Shroud Falls** (40): every civ forgets the map outside its own borders: those plots
become unexplored again.

**A30 The Lairs Deepen** (60): for the rest of the game, every lair spawns a stronger kind:
Barrows spawn Wraiths, Dens Greater Werewolves, Goblin Forts Ogres, Ruins Lizardman Rangers.

**A31 Peace of the Grave** (60): every war in the world ends, and each pair of civs is held to
10 turns of peace (enforced).

**A32 The Second Sun** (60): every Snow plot becomes Tundra, every Tundra Plains, and every
Plains Desert.

**A33 The Blade Returns** (50): the Godslayer, if it is not in the world, appears in the largest
unclaimed region, guarded by four Greater Werewolves.

**A34 The Towers Fall** (70): every Tower building is destroyed: the Towers of Alteration,
Divination, Necromancy, the Elements, Complacency and Eyes (not the Tower of Mastery).

**A35 The Arcane Tide** (60): every unit with magic in the world gains one level.

**A36 The Ghost Fleet** (40): three barbarian ships with Skeleton Crew appear near each civ's
largest coastal city.

### Late (60-90)

**A37 The Plague of Undeath** (80): every city of size 12 or more loses 3 population, and a
barbarian Pyre Zombie appears beside it for each.

**A38 The Bleeding Sky** (80): every unit outside a city takes damage equal to 30% of its HP.

**A39 The Twilight of the Gods** (80): every religion's holy city loses its shrine.

**A40 The Sky Falls** (80): meteors strike the ten largest cities. Units there take damage equal
to 50% of their HP, and each city loses 2 population.

**A41 The Feast of Crows** (70, lingering): for 20 turns, every battle's dead rise on the
battlefield as barbarian Skeletons.

**A42 The Final Plague** (90): every city loses half its population, except cities with a temple
of a good religion.

**A43 The Fall of the Mighty** (70): the score leader's capital loses half its population and its
walls.

**A44 The Siege of Capitals** (90): a barbarian army of six tier-3 units appears beside every
capital.

**A45 The Dimming of Wonders** (80, lingering): world wonders give no effects for 30 turns.

**A46 The Last Harvest** (80): every Farm in the world is destroyed.

### Endings (100)

**A47 Hell on Earth:** every plot outside cultural borders becomes hell terrain, and a barbarian
Pit Beast appears on every tenth such plot.

**A48 The Gathering:** every civ with less than half the power of the strongest civ within 15
plots becomes that civ's vassal.

**A49 The Long Night:** for the rest of the game, units see one plot less, nothing heals outside
cities, and every plot gives -1 food.

**A50 The Godless Age:** every religion is removed from every city.

---

## C. Review results (2026-11-07)

### C.1 Method

Three independent reviewers read all 150 cards. They worked from the files, not from the
designer's view:
- **Taste:** the owner's verdict history and principles in `docs/DECKS.md`.
- **Implementability:** every name checked against `data/json`, every mechanic against
  `crates/ffh-core`, plus collisions with existing content.
- **Power and duplication:** worth a great person, exploits, AI dependence, civ signatures,
  duplicates.

Their raw verdicts:

| Lens | Great-person cards (keep / fix / cut) | Armageddon cards (keep / fix / cut) |
|---|---|---|
| Taste | 57 / 19 / 24 | 19 / 8 / 23 |
| Implementability | 35 / 56 / 9 | 23 / 25 / 2 |
| Power | 25 / 41 / 34 | 15 / 16 / 19 |

The designer then weighed each objection against the card's best one-line reply, and checked
the engine facts the verdicts rest on in the code.

**Result:**
- **Great-person cards:** 58 survive (16 strong, 29 solid, 13 marginal), 42 cut.
- **Armageddon cards:** 19 survive, 31 cut.

These counts are after the owner's naval decision (rule 7), which brought back Harbor Chain,
Admiralty, Prize Courts and Leviathans Rise.

### C.2 Engine facts the cards got wrong (checked in the code)

- `TERRAIN_SHALLOWS` is land, the hell form of Marsh (`found`, no `water`). Water is Coast and
  Ocean. This also corrects earlier cards: Tidecall, Causeway and the Drowning Bells.
- Every world spell sets one per-player flag, `FEAT_GLOBAL_SPELL` (`unit/ffh.rs:277-283`,
  `:800-802`). A card's world spell would use up the civ's own.
- **Fear:** a unit sits out unless `rand(40) <= max(4, its strength + 20 + its level - the
  source's)` (`engagement/engage.rs:879`). An equal source sits out about half of the enemy, a
  tier-4 one about 60% of Champions: the strongest lever in battle.
- Invisible units are not engaged as defenders unless the attacker sees them
  (`engagement/pick.rs:157`). A stack made invisible on the map cannot be attacked.
- **Charms and hexes:** charms add up. Hexes are cast at the caster's speed, by units with
  magic, after a cavalry charge. Morale (Fear) is checked before the battle only.
- **World units, one per game:** the Leviathan, the Mithril Golem, Acheron. Paladins are capped
  at 4 per player.
- **Things that do not exist:** no Ice mana node (Ice mana comes from the Illian palace and
  Letum Frigus); no spider lair; no dragon with Flying.
- **Wrong stats and placement:**
  - The Ice Elemental is tier 3, 20/20.
  - Giant Spiders cost -1 and are not large.
  - The Wall of Stone is made by a spell and cannot be built.
  - Flood Plains grow only on desert.
  - The Volcano needs flat land.
  - Peaks cannot be entered by workers.
  - Improvements cannot be built in another civ's land.
  - The Remnants of Patria cannot take a city.
- **Already in FFH2 or this game:**
  - the Genesis ritual already plants forest in its builder's borders;
  - the Inquisition spell already purges a city's other religions;
  - the Resurrection spell already brings back a dead hero;
  - Subdue Animal already captures animals;
  - the National Epic already doubles a city's great-people points;
  - Orthus already appears at turn 75;
  - Drifa already has the drafted dragon's stats.
- **The AI has no naval combat and no blockade** (docs/AI.md). Cards whose payoff is beating
  AI ships pay little now.

### C.3 Rules for the decks that follow from them

1. **A card's world spell is its own one-off**, with a flag of its own, never the civ's world
   spell.
2. **Remote targeting** ("a city, plot or unit you can see") is a new shared system: a target
   for a spell, its UI, and an AI chooser. About 15 cards need it; it is built once.
3. **A floor for gold.** No card turns gold into production below 2 gold a hammer, and a
   one-shot completion is capped at 10 turns of the city's own production.
4. **Fortress buildings are few.** At most about three in the deck. The strong ones are
   national, or act in the first battle at the city each turn.
5. **One Fear card at most.** This round's Herald of Agares or the pool's Dread Banner. A dragon
   says whether it has Fear.
6. **No card makes a stack invisible on the map.**
7. **Naval cards are fine** (the owner, 2026-11-07). An AI sprint will give the AI naval combat;
   until then, their payoff against the AI is small.
8. **Every card is checked before it enters the deck** against the 21 civ world spells, the civs'
   unique units, and the existing spells, promotions and rituals.
9. **Regions are defined** as the largest connected group of unowned plots of a terrain or
   feature. One shared helper.
10. **"Gains a level"** means it gains the XP for its next level.
11. **New civs:**
    - They share 4 spare player slots with Hyborem and Basium.
    - Vassal births need a fallback when the No Vassal States option is on.
    - Each new civ is a content project: data, leader, art, AI.

### C.4 Verdicts, great-person cards

Rank: **A** strong, **B** solid, **C** marginal (keep only if the deck needs volume).

| id | name | verdict | rank | change, or reason cut |
|---|---|---|---|---|
| G01 | Elemental Foundry | keep | B | 300 hammers, national 1 per element; Air, Earth, Fire, Water only; replaces the Great Summoning |
| G02 | Mana Font | cut | | almost nothing reads a doubled mana count |
| G03 | Arcane Siegecraft | cut | | wall-breaking is crowded; Powder Charge and Wallbreaker cover it |
| G04 | Evoker | keep | B | say that the magic bolt counts |
| G05 | Abjurer | keep | A | enemy charms end each round and are not recast |
| G06 | Spellsword | keep | A | the six damage spells only (Shock, Grave Touch, Firebolt, Thornlash, Flare, Wave); magic is half its attack; in place of its first strike each round |
| G07 | Astral Leap | keep | A | all your units on one of your city plots move to a visible land plot with no foreign unit or city; they arrive with no moves left; not an attack |
| G08 | Null Zone | cut | | the pool's Wardstone |
| G09 | Elemental Attunement | keep | C | national; three buildings (fire, cold, lightning); the Immune promotions get unit combats |
| G10 | Seal of Binding | keep | B | cannot move for 20 turns (the immobile timer); still defends |
| G11 | Spell Thief | cut | | rare trigger, small gain |
| G12 | The Patrian Awakening | keep | A | founded next to the Remnants, else on the largest unowned region; an Expert line |
| G13 | The Great Conjunction | cut | | the XP half of the Amurites' Arcane Lacuna; a small Great Epic |
| G14 | Thunderhead Tower | keep | B | 30% of max HP, never below 1; the deck's one pre-battle damage building |
| G15 | Unseen Servants | cut | | Pioneers covers it |
| G16 | Arcane Fortress | keep | A | Sentinels take half damage from map bombards, BOMBARD and SHOOT |
| G17 | Starmetal Meteor | keep | A | a plot with no units; destroys its improvement or feature only |
| G18 | Gravity Well | keep | B | a pre-battle effect of the caster's presence, so it beats the charge |
| G19 | Magocracy | keep | B | adept-combat units cost 50% less, others 25% more |
| G20 | Sunder Node | cut | | Dispel Magic, and pillage, already do it |
| P01 | Living Walls | keep | B | national; its Sentinels use the Treant's attack and HP, one cell each; replaces the Gatehouse |
| P02 | The Verdant Pact | cut | | the Genesis ritual, same tech, already plants the forest |
| P03 | Courts of Junil | keep | A | keep every building that can be captured, and lose no population; rename (the Order's shrine is the Code of Junil) |
| P04 | Templar | keep | C | a promotion for Crusaders (the Order's own frontline) that heals the unit behind it |
| P05 | The Whisperer | keep | A | Madness becomes a pre-battle effect: the enemy's highest-attack unit (not a Sentinel) strikes its own side in round 1 |
| P06 | Call the Leviathan | keep | C | a Kraken if the Leviathan lives |
| P07 | Golems of the Earthmother | keep | A | the Mithril Golem (one in the world) at 600 hammers, whatever the counter; Kilmorph still its state religion |
| P08 | Vault of the Earthmother | cut | | strong but redundant: the Mithril Forge, Starmetal Meteor |
| P09 | Hellforge | cut | | the Infernals' own units |
| P10 | Herald of Agares | keep | B | rename (UNIT_HERALD exists); the existing Fear check; the deck's one Fear card, or the pool's Dread Banner |
| P11 | Halo | keep | A | revives the first unit of its side to fall, in its cell if free, else the rear; never a Sentinel |
| P12 | Pillar of Dawn | keep | B | each completion costs twice the last (the duty cycle) |
| P13 | Shadow Roads | cut | | invisible stacks cannot be attacked; the Sidar's Into the Mist |
| P14 | Mask of a Thousand Faces | cut | | the Svartalfar's Veil of Night |
| P15 | Heaven's Gate | cut | | weak; the Mercurians' Angels |
| P16 | Call of the Faithful | keep | B | "its heroes (one or two)"; dead ones return; their techs are not required |
| P17 | Sanctified Steel | keep | C | reaches melee, beast and disciple units only |
| P18 | Founding of the Dawn Knights | keep | B | two cities; its own Dawn Knight line at half cost; a Grand Master far above a Paladin; an Expert line |
| P19 | The Fey Court Awakens | keep | B | a defined region; +2 first strikes and +50% defence in forest instead of invisibility; no Treant walls; Fellowship as its state religion |
| P20 | Wings of Faith | cut | | Flying handed out wholesale; the Gryphon Riders |
| E01 | Master Masons' Guild | cut | | small; the Wall of Stone has no build cost |
| E02 | Gatehouse | cut | | Living Walls covers it |
| E03 | Boiling Oil | cut | | the Thunderhead Tower is the one kept |
| E04 | Siege Ballista | keep | B | sheet: tier 2 siege, 100 hammers, BOMBARD with PIERCE |
| E05 | Trench | keep | B | as written |
| E06 | Sapper | cut | | breaches any wall in a turn; crowded theme |
| E07 | Wall of the Kings | keep | A | "enemy" means units of civs at war with you, and barbarians |
| E08 | Mantlets | cut | | low impact |
| E09 | Grand Siege Park | cut | | large is nearly a downgrade for siege |
| E10 | Harbor Chain | keep | B | band II (Sailing is Classical) |
| E11 | Pioneers | keep | B | as written |
| E12 | Siege Lines | keep | B | may be built in the land of a civ you are at war with, next to its city |
| E13 | Powder Charge | keep | B | an action next to a walled city: the Charge is destroyed and every Sentinel is breached; no battle |
| E14 | Moat | keep | A | in the first battle at the city each turn, the attacker's STRIKE, CHARGE and STAB actions are spent in round 1 |
| E15 | Siege Colossus | keep | C | bombard rate 30 from 2 plots; the city counts as besieged that turn |
| E16 | Avalanche Charges | cut | | needs workers on peaks and hidden improvements; pays when an enemy walks into it |
| E17 | Bombard Barge | cut | | ships already bombard walls |
| E18 | Terraced Hills | cut | | an ordinary yield improvement (the Windmill is near) |
| E19 | Shatter the Walls | cut | | Powder Charge is the one wall-opener; Earthquake is near |
| E20 | Levy Hall | keep | C | national; the first battle each turn; the strongest land unit type the city can train |
| M01 | Gold Standard | keep | B | 2 gold a hammer, units only, under any Labor civic; research -20% |
| M02 | Prize Courts | keep | C | your ships capture the enemy ships they defeat (the Boarding promotion's effect); overlaps the Lanun, whose Boarding Party and Guybrush are the only units with Boarding today |
| M03 | Bazaar of Wonders | keep | B | a wonder with a positive cost whose prerequisites the city meets; not a victory wonder |
| M04 | Grand Bazaar | cut | | the Black Market |
| M05 | Hired Fleet | cut | | naval, small |
| M06 | Beast Market | keep | C | Bears 75, Lions 50, Tigers 75; no Griffons |
| M07 | The Condottieri Free State | keep | B | a Captain-General at attack 130; rushing at the 2-gold floor; an Expert line |
| M08 | Purchase Charter | keep | A | not a holy city or a wonder city; the price adds its buildings' cost |
| M09 | Transmutation Charter | keep | B | enumerate the classes from the data (the RUSH bonuses; the happiness bonuses; the food bonuses) |
| M10 | Admiralty | keep | B | as written: the naval Academy of War |
| M11 | Armourers' Guild | cut | | the Ingenuity trait's effect |
| M12 | Patrons' Guild | cut | | the National Epic |
| M13 | Market Towns | keep | C | as written |
| M14 | Regime Change | cut | | small, and felt by the rival |
| M15 | Purchased Peace | cut | | Judgment of Junil |
| M16 | Grand Exhibition | keep | C | the two cheapest techs known by three civs you have met |
| M17 | Reserve Arsenal | cut | | rewards hoarding; undefined when the city falls |
| M18 | Mammon's Bargain | keep | B | 1500 gold, +10 counter |
| M19 | Grand Order | keep | A | each city completes at most 10 turns of its own production |
| M20 | Golden Muster | cut | | the Golden Hoard; below the gold floor |
| B01 | Cultural Diaspora | cut | | rests on Foment Revolt flips while Unrest is parked |
| B02 | Sing the Stones | keep | B | City Ruins within 3 plots of your borders; size 3 |
| B03 | Golden Epic | cut | | the Kuriotates' Legends |
| B04 | Banshee Choir | cut | | a second Fear card |
| B05 | Shadow Theatre | cut | | the Sidar's Into the Mist |
| B06 | Cultural Annexation | keep | B | the owner keeps peace with you, with -4 attitude |
| B07 | Legacy of Kings | keep | A | Philosophical excluded (it feeds the deck) |
| B08 | Masque of the Red Death | keep | B | as written |
| B09 | Bard's Retinue | keep | C | as written |
| B10 | The Heroes' Saga | cut | | small; the Great Epic |
| B11 | The Unconquered City | keep | C | as written; counts toward the fortress-building limit |
| B12 | Feast of Plenty | cut | | the Calabim's River of Blood |
| B13 | Dragon's Oath | keep | A | a dragon of its own, not Drifa (large, 180 HP, 150/120, the dragons' movement, Breath Fire, no Fear) |
| B14 | Last Stand Hymn | keep | B | no fear clause: once half the committed defenders (not Sentinels) have fallen, the rest deal double damage; it is the Chorus of the Fallen's rework |
| B15 | Wardance | keep | C | level 5+; only the first strike each round cleaves |
| B16 | Cult of Heroes | cut | | little felt impact for a civic slot |
| B17 | Silver Tongue | cut | | Corlindale's Peace; Judgment of Junil |
| B18 | Anointed Champion | keep | B | also two levels (the hero's extra action does not stack with tier 4's) |
| B19 | Wager of Glory | cut | | vague; an expert names a city already falling |
| B20 | Heartland Charter | cut | | rests on Foment Revolt flips |

### C.5 Verdicts, Armageddon cards

| id | name | verdict | rank | change, or reason cut |
|---|---|---|---|---|
| A01 | The Wild Awakens | keep | B | the six lairs enumerated (Hellfire included) |
| A02 | The Sea Recedes | keep | B | one sea named at the reveal: its unimproved Coast outside city radii becomes grassland; ships move to the nearest water; sea resources there go |
| A03 | Collapse of the Roads | cut | | workers rebuild roads |
| A04 | The Scaled Kingdom | keep | A | a defined region; note the lizardmen are the Clan of Embers' units too |
| A05 | The Gods Answer | cut | | Call of the Faithful; a boon, not a crisis |
| A06 | The Brood | keep | A | unclaimed forest only; a new spider-nest lair (not permanent); Giant Spiders 60 hammers for the Brood; the Spider Queen large |
| A07 | The Burning Plains | cut | | the base rule already spreads hell over evil land from 50 |
| A08 | The Great Flood | cut | | a huge tax with nothing to prepare; Flood Plains are desert-only |
| A09 | Volcanic Spring | keep | A | a hill within 3 of each of the six largest cities, named at the reveal; it becomes flat with a Volcano; units within 1 take 50%, improvements go, cities lose 2 population but stand; then +1 food (it replaces FFH2's random volcano event) |
| A10 | Leviathans Rise | keep | C | a barbarian Kraken in every ocean of 20 or more plots, and the Leviathan (one per game) in the largest if it is not alive |
| A11 | Werewolf Moon | keep | B | as written |
| A12 | The Gold Rot | cut | | arithmetic, falls on the Khazad |
| A13 | The Great Fire | cut | | an unanswerable tax on big cities |
| A14 | The Iron Blight | cut | | -10%, and the Rust spell exists |
| A15 | Mana Collapse | cut | | workers rebuild nodes |
| A16 | The Fire Goes Out | cut | | may touch nobody; temporary |
| A17 | Unbound Elementals | keep | C | raw and typed nodes alike, outside city radii; no Ice |
| A18 | The Great Tremor | cut | | workers rebuild mines |
| A19 | The Gargoyles Wake | keep | C | a Gargoyle beside every walled city; no "lose a level" (Walls and Wall of Stone are the same) |
| A20 | The Golem Rebellion | cut | | deletes the Luchuirp |
| A21 | The Desert Wakes | cut | | trivial |
| A22 | The Host of Nyx | keep | A | reworked: **The Infernal Gate Opens**. If the Infernals are not in play, they arrive as with the Infernal Pact, beside the evil civ with the most cities; otherwise the card is dealt again. It uses the Infernals that exist instead of copying them. |
| A23 | The Shattering of Peaks | cut | | small and mixed |
| A24 | The Burning Libraries | cut | | no tech order is kept; small |
| A25 | The Iron Rain | cut | | obsolete by 60 |
| A26 | The Four Winds | cut | | four mild repairs |
| A27 | The Unchaining of Orthus | keep | C | Orthus's Host: 10 Ogres join Orthus if he lives, else a new warlord (Champion line with Hero, attack 110) leads them from the largest unowned region |
| A28 | The Unseen War | cut | | narrow and temporary |
| A29 | The Shroud Falls | cut | | acts on knowledge; scouts undo it |
| A30 | The Lairs Deepen | keep | B | enumerate every lair, Hellfire included |
| A31 | Peace of the Grave | cut | | a rule by fiat; Corlindale's Peace |
| A32 | The Second Sun | keep | B | one step from each plot's terrain before the card: Snow to Tundra, Tundra to Plains (no Plains to Desert) |
| A33 | The Blade Returns | keep | B | at the unowned land plot farthest from every city |
| A34 | The Towers Fall | cut | | no AI builds the towers |
| A35 | The Arcane Tide | cut | | the Great Conjunction |
| A36 | The Ghost Fleet | cut | | barbarian ships hold |
| A37 | The Plague of Undeath | cut | | the Sky Falls is the one big-city card |
| A38 | The Bleeding Sky | cut | | Blight |
| A39 | The Twilight of the Gods | keep | B | as written |
| A40 | The Sky Falls | keep | B | as written |
| A41 | The Feast of Crows | cut | | tier-1 skeletons at 70 are clutter |
| A42 | The Final Plague | cut | | Apocalypse; its good-temple exemption could join Apocalypse |
| A43 | The Fall of the Mighty | cut | | one city, no answer |
| A44 | The Siege of Capitals | cut | | a speed bump at 90 |
| A45 | The Dimming of Wonders | cut | | temporary, by fiat |
| A46 | The Last Harvest | cut | | workers rebuild farms |
| A47 | Hell on Earth | cut | | the base rule and Hellfire |
| A48 | The Gathering | keep | A | 15 plots capital to capital; vassals made this way do not count toward domination |
| A49 | The Long Night | keep | B | -1 food and no healing outside cities; no sight change |
| A50 | The Godless Age | keep | B | state religions end, holy cities and shrines are gone |

### C.6 Where the designer overruled a reviewer

- **Kept the Wall of the Kings** against "Sanctuary is near": the Elohim's Sanctuary expels and
  bars entry, while this card stops movement at the border. Different verbs.
- **Kept Legacy of Kings** against "the Adaptive trait": one permanent pick is not a periodic
  swap. Philosophical is excluded because it feeds the deck.
- **Cut the Verdant Pact** although taste and power kept it: the Genesis ritual, on the same
  tech, already does it. The implementability finding decides.
- **Cut the Burning Plains** although taste kept it: the base rule already turns evil land
  hellish from 50.
- **Reworked the Host of Nyx into the Infernal Gate**, as the power review proposed: it uses
  the Infernals the game already has.
- **Cut the Vault of the Earthmother** (taste 4): it was strong, but the deck already has metal
  gifts; Starmetal Meteor is the one kept.
