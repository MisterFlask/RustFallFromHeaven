# Decks: secret techs and the Armageddon deck

**Status: design draft, nothing built** (2026-11-07). Ideas to choose from, not a spec. The
numbers are placeholders. Each card and crisis carries an *Expert* line: how a strong player uses
it. That line is the test for strategy relevance, and later the AI's brief (CLAUDE.md, "AI
changes"). If an entry has no expert line that differs from a novice's, it does not belong in
the deck.

Settled in discussion so far:

- A great person is born with one secret tech, which the player sees at birth. Discovering it is
  a new great-person ability next to the current ones.
- There are five suits, one per great person that specialists produce: Sage, Prophet, Engineer,
  Merchant, Bard. Great Commanders are left out for now.
- The Armageddon counter becomes a deck of crises triggered at thresholds.
- The random events and crime are to be scrapped. The event machinery stays for the triggers
  that rules code fires (Armageddon, the Adaptive and Insane traits, lair outcomes, ...).

Proposed and not yet decided:

- One world deck. A card leaves the deck only when it is discovered. A card whose great person
  is used another way or dies goes back into the deck.
- A card is a tech that cannot be researched or traded (`disable`, `trade: false`), granted with
  `team_set_has_tech`. Its content gates on it through `prereq_tech`.
- Every card *needs* an ordinary tech for its content (the "needs" line). Knowing the card at
  birth then pulls research toward that tech.
- A card is drawn from its suit, filtered by era band (the player's era: I Ancient, II
  Classical, III Medieval), alignment and state religion where the card says so.

---

## 1. Levers

Everything a card or a crisis can push on. Each item names its data key or system where it
exists. **(new)** marks a lever the engine does not have yet.

### 1.1 Units

- **Stat sheet**: HP, attack, defence, magic, magic defence, accuracy, evasion, crit, speed
  (`stats`).
- **Battle shape**:
  - rows and actions per row (`engage_rows`, `engage_row_order`);
  - attack patterns: volley, pierce, sweep, cleave, trample;
  - damage multipliers against unit combats, large units and walls (`damage_vs`,
    `damage_vs_large`, `damage_vs_walls`), and resistances (`resist_vs`);
  - armor and armor piercing; pursuit (no withdrawal from its hits);
  - target preference; damage by round (`round_percent`);
  - large (two cells, 150% HP); first strikes; defensive strikes (chance, damage); collateral
    damage; withdrawal.
- **Movement**: moves; flat movement cost; ignore terrain cost; all-terrain; impassable;
  flying; water walking; terrain and feature natives (double moves, attack and defence by
  terrain and feature); amphibious; cargo and special cargo.
- **Vision and stealth**: invisible type; see invisible; hidden nationality; sight.
- **Combat aftermath**:
  - capture; enslavement chance; gold from combat;
  - convert or create a unit from combat (`unit_convert_from_combat`,
    `unit_create_from_combat`);
  - explode in combat; suicide;
  - XP values; free XP; promotion from combat;
  - immortal (returns in the capital); duration (summons, `duration_from_combat`).
- **Roles outside battle**: build, work rate, found a city, religion spreads, culture garrison,
  military happiness, discover, hurry, trade mission, great work, join a city as a specialist,
  golden age, force buildings.
- **Availability**:
  - national: a limit at the normal cost, 500% past it; `hard_limit` for a true limit;
  - world unit: once per game;
  - prerequisites: alignment, civic, religion, state religion, building, resource, global
    counter (`prereq_global_counter`), min level, tier;
  - upgrades, `never_obsolete`, `upgrade_civ`.
- **Unit to something else (new)**: a unit that becomes an improvement, a building, or a
  governor.

### 1.2 Promotions

- Stat changes in points and percent (`stat_changes`); combat percent per point of the global
  counter; combat percent inside borders.
- Healing: own, same tile, adjacent, enemy and neutral heal; heal per combat; always heal.
- Fear and fear immunity. Immunity to first strikes, defensive strikes, capture and magic.
  Magic resistance; resistance by damage type.
- Casting: twincast; spell damage modify; caster XP; caster resistance.
- XP per turn; XP from combat; experience percent.
- Expiry: expire chance; removed by combat, by casting or when healed; dispellable.
- Promotion combat apply (a hit gives the target a promotion: poison, curses); random apply;
  summon perk; mutation.
- Betrayal chance; hidden nationality; visibility change; held (cannot move); not alive; race;
  per-turn script (`py_per_turn`).
- Movement: double moves on terrain or feature, river, hills, enemy routes; blitz; boarding.
- Moves the global counter (`modify_global_counter`, also per fight).

### 1.3 Spells

- **On the map**:
  - damage by type (8 types: cold, death, fire, holy, lightning, physical, poison, unholy),
    with resistance, range and limit;
  - summon units (with duration, or permanent);
  - create a building, feature or improvement;
  - convert a unit's type; change population;
  - dispel; immobilize;
  - spread religion; cause war; sacrifice the caster;
  - copy the caster's promotions; add or remove promotions;
  - gold cost; delay;
  - world spell (`global`: once per game per civ);
  - any effect through a script (`py_result`, `py_requirement`).
- **In battle**: damage by power and pattern (`battle_power`, `battle_pattern`); heals
  (`battle_heal`); charms and hexes on a side (`battle_charm`); Wonder's weighted roll
  (`battle_wonder`); battle-only spells (`battle_only`).
- **New charm kinds (new)**: reroll misses; skip an enemy row's action; decoy units; turn a
  battle's dead into a buff.

### 1.4 Buildings and wonders

- Yields and commerce (flat, modifiers, per specialist, per resource); specialists (slots, free
  specialists); great-people rate and type.
- Happiness and health: city, area and global; no unhappiness; no unhealthy population.
- Defence: city defence, Sentinels (wall sections), bombard defence, all-city defence, magic
  resistance.
- Military: production modifiers (military, by domain, by resource); free XP by domain and unit
  combat; free promotion and free promotion pick for units trained; heal rate; see invisible;
  hide units.
- Free things: free resources (up to three), a free building, free techs, golden age.
- Trade routes and trade route modifier; maintenance; war weariness; food kept; city radius
  (`plot_radius`).
- Religion: holy city, religion change, state religion commerce, global religion commerce.
- Moves the global counter (`modify_global_counter`); team share; never captured; conquest
  probability; equipment storage; requires a caster; unhappy production.
- **Building tiers**: Lesser to Legendary, rolled per city at founding (docs/SUITABILITY.md).
  Rerolling a tier would be new.
- **Rituals** (projects): world-unique or not, prerequisite global counter, move the counter,
  scripted effects.

### 1.5 Map

- **Improvements**: unique improvements; lairs that spawn units (`spawn_unit_type`); acts as a
  city; range defence; outside borders; permanent; goody huts; research bonus on a tech
  (the 17 unique features); visibility.
- **Portals**: plots with a portal exit (`portal_exit_x/y`): teleport machinery that exists.
- **Features**: create or remove; flammable (burning forests); defence; engagement slots;
  impassable; growth and spread; health percent; Blizzard, Flames, Ice, Smoke, Tormented Souls.
- **Terrain**: hell terrain driven by the plot counter (Burning Sands, Broken Lands, Fields of
  Perdition); temporary terrain with a timer (`temp_terrain_timer`, `real_terrain_type`: the
  Thaw machinery); terrain-changing spells.
- **Resources**: revealed by a tech; mana types (and raw mana a player assigns); free promotion
  from a resource; research modifier; great-people rate modifier; mutate chance.
- **Terraforming land into sea, or sea into land (new).**

### 1.6 The player

- Traits, including Adaptive (choose a trait periodically) and Insane (traits reshuffled): the
  machinery to swap traits exists.
- Alignment (good, neutral, evil, and shifts); state religion; civics (31: their anger,
  upkeep, prerequisites, `block_alignment`, `prereq_war`, hidden civics).
- Golden ages; anarchy; gold; research modifiers; the great-people threshold; upkeep; maximum
  cities.
- Diplomacy:
  - attitude and memory modifiers; contact; open borders; pacts; vassals;
  - gifts and demands (`tribute.rs`); standing deals and their cancellation;
  - **new deal items (new)**: loans, sworn peace.
- Councils: Overcouncil and Undercouncil votes (force war, the rings, the slave trade, ...).
- Crusades: calls, pledges, the Host player, allied loans. A Host is a temporary AI player
  whose only goal is one war: the machinery to create one for any purpose exists.
- Governors: the bench, profiles, traits, the four direct-rule slots, the replacement cost.
- Armies: requisition, target size, supply (within 3 plots of the trade network), marching
  recruits.
- Cultural channels: Foment Revolt, Spread Religion; falloff with distance squared; resistance
  from culture and garrison.
- National units' over-limit price.
- The score: a share of people, land and knowledge.
- **Information (new)**: see rivals' gold, deals, cards held, great-people progress; fake what
  rivals see.

### 1.7 The world

- The Armageddon counter (0-100): its sources and sinks (Ashen Veil founding and spread,
  Octopus Overlords founding, razing, rituals, certain units and buildings, a player's death by
  alignment), and what reads it (Planar Gate limits, units and rituals that need it, Stigmata).
- Mid-game players added (`addPlayer`: Hyborem, Basium, the crusade Hosts).
- Barbarians: animals, lairs, barbarian heroes (Orthus, the Sailor's Dirge), world units
  (Acheron, the Horsemen).
- World wonders; world spells; the Tower of Mastery and the Altar (not pursued by the AI).
- **New**:
  - **culpability**: each player's net share of the counter's rise;
  - per-city history (units killed defending it, its razings);
  - per-player history (forests cut);
  - weather over an area;
  - deck operations: draw, peek, keep one of two, put back, copy.

### 1.8 Battles as a lever set

Slots by ground (open land 10; fewer on rough ground and against Sentinels). Rows and lanes.
Reserves. Three rounds. Ground effects:
- a city halves the attacker's arrows;
- forest and jungle halve arrows and stones and stop flanking;
- hills weaken the attacker and strengthen the defender's arrows.

Also: fear before the battle; defensive strikes; withdrawal; Sentinels intercept down their
lanes; XP shared per kill. **New**: weather on the battlefield, decoys, a side's mood (charms
that build as units fall).

---

## 2. Themes

### 2.1 From the lore (data/text)

- **The twenty angels and their aspects.** Good and neutral:
  - Lugus (light), Sirona (wisdom), Bhall (fire), Nantosuelta (faith);
  - Amathaon (fertility; his archangel is Maponos the Young), Junil (justice);
  - Arawn (death), Oghma (knowledge), Danalin (water), Dagda (balance);
  - Kilmorph (earth), Sucellus (nature; his archangel is Cernunnos), Tali (air).

  The seven that fell, each a virtue turned: Agares (hope to despair), Camulos (peace to war),
  Aeron (strength to rage), Ceridwen (stars to magic), Mammon (foresight to greed), Esus (trust
  to deception), Mulcarn (ice to winter).
- **Nemed**, who gave up his precept (Life) to father mankind.
- **Nyx**, Agares's world in black and gold, with its infinite wells of raw elements.
- **Other lore**: the Age of Ice; the Compact; the Mercurians and the Infernals; Hyborem and
  Basium.
- **Villains and monsters**: the Horsemen (Stephanos, Buboes, Yersinia, Ars Moriendi) and
  Wrath; the dragons (Acheron, Abashi, Eurabatres); Orthus and the Sailor's Dirge.
- **Institutions**: the Guild of the Nine; the Overcouncil and the Undercouncil.
- **Magic**: planar gates and the Hall of Mirrors; golems (wood, clockwork, iron, flesh, bone,
  mithril); the mana spheres; the unique features (Yggdrasil, Odio's Prison, the Pool of
  Tears, ...).
- **The civilizations' cultures**: the Kuriotates' few great cities, the Balseraphs' puppets
  and freak shows, the Calabim's cattle, the Sidar's fading, the Luchuirp's constructs, the
  Lanun's coves.

### 2.2 Wider

Go wide; any of these can carry a card or a crisis.

- **Time**: prophecy, clocks, debts falling due, rewinding, inheritance.
- **Knowledge**: true names, maps that change the land, forbidden libraries, plagiarism,
  forgetting.
- **Bodies**: plague and quarantine, vats, grafting, familiars, ghosts of the dead.
- **Money**: interest, insurance, bounties, monopolies, black markets, tolls, bribes, auctions.
- **Faith**: oaths, relics, pilgrimage, heresy, silence of the gods, false colours.
- **Craft**: tunnels, canals, causeways, walking fortresses, cities on wheels, weather machines.
- **Story**: sagas, satire, opera, masks, lullabies, legends who leave their posts.
- **Nature striking back**: drowning coasts, long winters, walking forests, meteors, ash
  skies.
- **History coming back**: the razed rising, the fought-over city's ghosts, old oaths
  dissolving.
- **Mirrors and doubles**: copies of your own best units, false armies, illusions.

### 2.3 Suits

| Suit | Great person | Its ground |
|---|---|---|
| Sage | Great Sage (`UNIT_SCIENTIST`) | magic, mana, knowledge, information, the deck itself |
| Prophet | Great Prophet | the angels, relics, faith, oaths, death and the world clock |
| Engineer | Great Engineer | constructs, siege, tunnels, terraforming, governors of brass |
| Merchant | Great Merchant | gold, contracts, mercenaries, markets, supply, bought wars |
| Bard | Great Bard (`UNIT_ARTIST`) | battle morale, culture, diplomacy by story, illusion, legends |

---

## 3. The great-person deck (61 cards)

Format: **Name** · band · needs (the ordinary tech the content requires) · vehicle. Tags:
**[data]** = existing keys can express it; **[new: ...]** = a rule to write. "National, N" =
N at the normal cost, then 500% each; "hard" = a true limit.

### 3.1 Sage (12)

**The Second Sky** · I · needs Calendar · wonder (national) · [new: map reveal]
When built, shows every mana node, unique feature, lair and goody hut on the map, and keeps
them shown (not the units on them). Recon units trained here get +1 sight.
*Expert:* early, this is a settling map. A unique feature doubles a tech's research for whoever
holds it, so the Second Sky tells you which border to push.

**Familiars** · I · needs Mysticism · national unit, 4 · [new: bound interception]
Bind: the Familiar binds to a caster in its stack and stands in that caster's lane. In battle
it takes the first spell or shot aimed at its caster, as a Sentinel intercepts down its lane.
The bound caster gains +1 XP a turn.
*Expert:* bind to the two mages that carry the army's damage. A caster that survives the first
round is worth more than a second caster.

**Homunculus Vats** · I · needs Alteration · building + national unit, 3 · [new: chosen spell]
The Homunculus is a frail caster that picks one sphere I battle spell when trained and knows it
without the mana.
*Expert:* buy the sphere you lack (Life's Mend for healing, Body's Quicken) instead of
reshaping your mana for it.

**The Index of Unwritten Books** · I · needs Writing · wonder (national) · [new: deck operations]
When built, look at the top three cards of the deck and put them back in any order. From then
on, each great person born in your empire draws two cards: you keep one and the other goes to
the bottom.
*Expert:* first in a specialist economy. It turns every later great person into a choice of
two, and every future draw into one you shaped.

**The Calculus of Chance** · II · needs Mathematics · building + national unit, 2 · [new: charm kind]
Units trained in the city get +10 crit. The Augur's battle charm, *Foreseen*: the first miss
each unit on its side makes in the battle hits instead.
*Expert:* pair the Augur with low-accuracy, high-damage units (berserkers, siege); it adds most
where misses cost most.

**Grammar of True Names** · II · needs Divination · spell (adepts with Mind I) · [new: named type]
*Speak the Name*: name a unit type a rival owns that you have seen. For 25 turns, your units
ignore that type's evasion and your spells ignore half its magic defence. One name per rival at
a time.
*Expert:* name the backbone of the army you are about to meet (its Champions, its mages)
before the war, not after.

**Waystones** · II · needs Knowledge of the Ether · building · [data-ish: portals]
A mage in a city with a Waystone can *Fold* its stack to another of your Waystone cities, once
every 5 turns per stack.
*Expert:* two Waystones at two fronts let one army defend both. Build them where the wars will
be, not where the cities are.

**Mana Grafting** · II · needs Elementalism · spell (adept on a node in your borders) · [new]
*Graft*: the node becomes the type of another node you own. It cannot be grafted again for 30
turns.
*Expert:* double the sphere your battle mages use, or make the third node of a sphere whose
tier-III spell you want.

**Orrery of Ceridwen** · II · needs Astronomy · world wonder · [new: bridge to the Armageddon deck]
You see the next two Armageddon cards and their thresholds; others see one, and only when it is
close. The city gets +2 research per 10 points of the counter.
*Expert:* knowing that 60 is the Silence of the Gods an era ahead lets you plan the war around
it.

**The Plagiarist's Lens** · III · needs Arcane Lore · building (national) · [new: copy a card]
Once, learn a secret tech held by a civ you have met. That civ learns you did (-2 attitude for
50 turns).
*Expert:* copy the strongest card in the world, or the one built to beat yours.

**The Recursive Lemma** · III · needs Arcane Lore · promotion (casters of level 6+) · [data: twincast]
Recursion in one sphere: that sphere's battle spells are cast twice.
*Expert:* choose the sphere whose battle spell has the widest pattern (Chain Lightning,
Firestorm); twincasting a single-target spell is the weak pick.

**Chronicle of Recall** · III · needs Omniscience · world spell · [new: loss log]
Once per game: every unit you lost in the last 3 turns returns in your capital at half HP, with
its promotions.
*Expert:* it makes one all-in battle affordable: attack, lose, recall, attack again. Hold it for
the decisive war; using it after a skirmish wastes it.

### 3.2 Prophet (13)

Alignment in brackets where the card is filtered by it.

**Maponos's Cradle** · I · needs Agriculture · building (national) · [new]
+2 food. Every 20 turns the city makes a free Settler without losing population. With no free
site within 10 plots it makes a Worker instead.
*Expert:* worth a great person only while land is open. Drawn late, it is a worker mill; settle
the great person instead.

**Breath of Tali** · I · needs Ancient Chants · national unit, 2 · [data-ish: charm]
The Windcaller's battle charm, *Tailwind*: own side +2 speed in round 1. On the map, its stack
gets +1 move on flat land and coast.
*Expert:* with a mounted army it means striking first in every battle; with slow infantry it is
a waste.

**Lantern of Lugus** [good] · I · needs Ancient Chants · national unit, 2 · [data-ish]
The Dawnbringer is a holy-damage shooter that prefers the middle row, with the hex *Glare*
(enemy front row -15 accuracy in round 1). It sees invisible units within 2 plots.
*Expert:* the answer to a neighbour that fights hidden (Svartalfar, the Council of Esus).

**Agares's Well** [evil] · I · needs Way of the Wicked · building (national) · [data: free bonus; new: choice]
The city gains a mana resource of a type you choose: a well of raw element, as Agares made in
Nyx. Building it adds +3 to the counter.
*Expert:* the sphere your mages lack, paid for in doom; worth more the earlier it comes.

**Tidecall of Danalin** · II · needs Message from the Deep · national unit, 1 (hard) · [new: land to sea]
*Drown*: a flat land plot next to water within 2 plots, with no city, becomes Shallows for
good. Units on it that cannot swim move to the nearest land. Once every 10 turns.
*Expert:* cut the land bridge the enemy's army needs, or drown a rival's coastal farms; one
plot at the right isthmus can end a war plan.

**Heartwood of Sucellus** [not evil] · II · needs Way of the Forests · building · [new: features wake]
When enemy land units enter your borders within 3 plots of the city, a forest plot next to each
enemy stack may wake as a Treant for 5 turns (one per stack per turn).
*Expert:* leave forest belts around border cities. The axe that chops them chops your walls.

**Scales of Junil** [good] · II · needs Orders from Heaven · world wonder · [new: deal item]
Sworn Peace becomes a deal item: a 30-turn peace sworn on the Scales. The side that breaks it
loses half its gold to the other and takes -3 attitude with every civ.
*Expert:* swear with the strong neighbour to be safe while you take the weak one; or bind the
weak one to peace while you fight elsewhere.

**Ledger of Mammon** [evil] · II · needs Currency · building (national) · [new: interest]
Gold earns 3% a turn, on up to 1000. While the treasury holds over 500, +1 counter every 10
turns. If gold falls to 0, the largest governed city falls into disorder for 3 turns.
*Expert:* hoard in peace and spend before war; never run the treasury dry with it.

**Masks of Esus** [evil] · II · needs Deception · national unit, 2 · [new: false colours]
*False Colours*: the Masker's stack shows as another civ's units (choose which). The first
attack it makes counts as that civ's attack in the victim's memory and diplomacy.
*Expert:* start a war between two rivals and stay out of it; keep the Masker out of sight
afterwards.

**Ossuary of Arawn** · II · needs Necromancy · building · [new: souls]
Each of your units that dies within 5 plots of the city adds a Soul (at most 20). Spend 5 Souls
for a Bone Golem, or 2 to bring back one unit killed this turn.
*Expert:* build it behind the front: a defensive war of attrition pays in golems.

**The Pilgrim Road** [good or neutral] · II · needs Priesthood · building (holy city) · [new]
Every 15 turns, each foreign civ whose state religion is this city's sends a Pilgrim. The
Pilgrim walks to the city and joins it (+1 population) or gives gold (50). Killing a pilgrim
costs -2 attitude with every civ of that religion.
*Expert:* worth it to the holder of a widespread religion; it pays to spread the faith to
others' capitals.

**Brand of Aeron** [evil] · III · needs Rage · national unit, 1 · [data-ish]
*Brand* gives a unit in the stack Aeron's Brand: +50% attack, pursuit, no withdrawal. A branded
unit that has not fought for 5 turns loses 10% HP a turn.
*Expert:* brand the assault units at the start of a siege, never the garrison.

**Dagda's Balance** [neutral] · III · needs Theology · world wonder · [new: bridge to the Armageddon deck]
While you hold it, the counter moves 1 point a turn toward 50. Once every 20 turns, *Tilt*:
move the counter 3 points either way.
*Expert:* hold the world just under the threshold whose card hurts you, or just over the one
that helps; with the Orrery you know which.

### 3.3 Engineer (12)

**Cairn Golem** · I · needs Masonry · national unit, 3 · [new: unit to improvement]
Large, armored, 1 move. *Raise the Cairn* (on hills): it becomes a permanent Cairn, +50%
defence for units on it and sight 3. A battle on the Cairn counts it as a Sentinel on open
ground.
*Expert:* plant cairns at the chokepoints before borders harden; three cairns decide where the
next war is fought.

**Causeway** · I · needs Fishing · worker build · [new: sea to land]
Over 15 turns, a Shallows or Coast plot that touches land on two sides becomes grassland.
*Expert:* bridge to the island or the rival's coast. It is the counter to the Lanun and to
Tidecall.

**Tunnel Heads** · II · needs Construction · building · [data-ish: portals]
Your land units move between any two of your cities with Tunnel Heads in one move. An enemy
that takes a Tunnel city can use the network until you *Collapse* it from another Tunnel city.
*Expert:* interior lines between two fronts; collapse the moment a head falls.

**Clockwork Steward** · II · needs Engineering · national unit, 3 · [new: governor kind]
It can be installed as a governor: no traits, no replacement cost, and it builds from a list you
set. The city stays governed, so the four direct-rule slots are unchanged.
*Expert:* the city that feeds your armies gets a Steward; that makes it a fifth directly ruled
city in all but name.

**Tierwright's Forge** · II · needs Smelting · building · [new: tier reroll]
Every 30 turns, reroll one building class's tier in this city. The new tier can be lower.
*Expert:* reroll the Lesser tier of the class this city lives on; leave Grand and Legendary
alone.

**Hollow Mountain** · II · needs Mining · building (city next to a peak) · [data: hide units]
The peak becomes workable (+2 production, +1 commerce). Units in the city are hidden from
enemies, and an attack on the city faces one more Sentinel.
*Expert:* an ambush city; the enemy attacks what it thinks is an empty city.

**Burning Mirror** · II · needs Optics · national siege unit, 2 · [data + small]
Bombards Sentinels at double rate where there is no forest, jungle, snow or blizzard. Each
bombard has a 10% chance to burn one building in the city. In battle it volleys fire on the back
row.
*Expert:* a siege engine that also wrecks the prize; bring it against cities you will raze or
cannot hold.

**Canal Locks** · II · needs Engineering · worker build · [new]
A Canal is a land plot ships can enter; it must touch water or another canal.
*Expert:* cut the isthmus the rival's navy sails around, or float a fleet to an inland lake
city.

**Weather Engine** · III · needs Machinery · building (national) · [new: battlefield weather]
Once every 15 turns, set a weather for 5 turns over a 5-plot radius within 10 plots of the
city:
- Blizzard: no healing, -1 move.
- Storm: shots and stones -50% in battles there.
- Fog: no shots in round 1; units are hidden beyond 1 plot.

*Expert:* Storm over your walls before the archer army arrives; Fog over your own advance.

**Bastion** · III · needs Machinery · national unit, 1 (hard) · [data: Sentinels; new: mobile]
A large walking fortress that cannot attack. The stack it stands in fights every battle with
two Sentinels on its side.
*Expert:* with the main army, it turns every field battle into a siege the enemy must breach
first.

**Wheeled City** · III · needs Mithril Working · building · [new, large]
*Pack Up*: the city becomes a City Caravan (population, buildings and tiers kept; its culture
lost), 1 move. It refounds on a valid site within 12 plots. A caravan defends with its garrison;
taken, it is destroyed.
*Expert:* pull a doomed border city out of a lost war, or move a city onto a newly found unique
feature or a crater's mana.

**Automatons** · III · needs Mithril Working · national unit, 6 · [data: not alive, immunities; new: supply]
Immune to fear, charms and hexes. Never out of supply (armies' supply rule ignores them). They
gain no XP and heal only in a city with a Forge.
*Expert:* the core of a long campaign beyond the supply line; never the garrison of a forgeless
city.

### 3.4 Merchant (12)

**Toll Roads** · I · needs Trade · building (national) · [new]
Foreign units pay their owner's gold to you, 2 per plot moved on your roads. Units of civs at
war with you get no road movement inside your borders.
*Expert:* the border with the warlike neighbour; his invasion crawls and his peace pays you.

**Black Market** · I · needs Trade · building · [new: bought resources]
Buy a resource you lack for gold per turn; the price falls with how many civs hold it. Mana
counts.
*Expert:* Iron or Mithril for the civ that has none; the price of a resource is the price of
the army it unlocks.

**Ledger of Nations** · I · needs Writing · building (national) · [new: information]
Shows every met civ's gold, deals, power, held secret techs and great-people progress.
*Expert:* know who is weak, and who holds the card you fear, before you declare.

**Insurance of the Nine** · I · needs Currency · building (national) · [new]
Each of your units killed refunds 40% of its hammer cost in gold.
*Expert:* makes trading units cheap; best for the civ that attacks every turn.

**Mercenary Hall** · II · needs Trade · building · [new: hiring board]
A board of three units, drawn from every civ's unit types at your tech level (other civs'
unique units included, heroes not), renewed every 10 turns. Hire for twice the hammer cost in
gold; they come with 4 XP and cost 2 gold a turn. Unpaid, they desert to the barbarians.
*Expert:* buy the unit your army lacks: a healer, a siege engine, another civ's best line.

**Debt Bonds** · II · needs Taxation · deal item · [new]
Lend gold for 20 turns at 30%, paid per turn. If the debtor misses a payment, you may claim one
of its cities within 6 plots of yours. A refusal is war, with no war weariness for you for 20
turns.
*Expert:* lend to the weak neighbour whose border city you want.

**Bounty Office** · II · needs Code of Laws · building (national) · [new]
Name a civ. Whoever kills one of its units (barbarians excluded) gets 15 gold from your
treasury.
*Expert:* pay the whole world to bleed the leader without declaring war yourself.

**Caravanserai** · II · needs Taxation · building · [armies supply]
Your armies stay in supply along roads to any foreign city you have open borders with, and
anywhere inside an ally's borders.
*Expert:* needed for a far crusade or an allied loan war.

**Cornering the Market** · II · needs Mathematics · building (national) · [new]
Choose a resource you hold 3 or more of. For 30 turns, other civs get its effect only through
trade with you; their own copies give nothing. Once every 50 turns.
*Expert:* corner Horses before the Hippus' war, or Wine before a rival's golden age.

**Paymaster** · III · needs Guilds · national unit, 1 · [new]
*Buy Out*: an adjacent enemy unit (not a hero or world unit) joins you for 3 times its hammer
cost in gold.
*Expert:* buy the enemy's strongest unit on the eve of battle; you pay full price and he loses
a piece.

**The Free Company** · III · needs Mercantilism · building (national) · [crusade Host]
Pay gold (scaled by the target's power) to launch a mercenary crusade: a Host player wars on a
civ you name for 30 turns, with units from the mercenary pool at your tech level. The war is
the Host's, not yours.
*Expert:* buy a war on the runner-up while you build; you keep your peace treaties.

**Merchant Princes** · III · needs Mercantilism · governor kind · [new]
Merchant Princes join your bench, one per 3 cities with a Market. A city governed by one turns
its surplus production into gold, 1:1, and pays its army requisitions in gold instead of
hammers.
*Expert:* fund a war without stalling the governed cities' growth.

### 3.5 Bard (12)

**Saga of the Named** · I · needs Ancient Chants · national unit, 3 · [new]
The Skald rides with an army. Each battle the army wins adds a verse. At 5 verses, the army's
most experienced unit becomes a Hero with a name of its own, and the saga gives +5 culture a
turn to the capital for the rest of the game.
*Expert:* ride with the army that will fight most, behind the line.

**War Drums** · I · needs Festivals · national unit, 2 · [data-ish: charm]
The Drummer's battle charm, *Drums*: own side +1 speed and immune to fear in round 1.
*Expert:* the answer to fear-heavy armies (the undead, the Infernals); elsewhere a modest
charm.

**Siren Rock** · I · needs Sailing · building (coastal) · [new]
Enemy ships that end a turn within 2 plots are pulled one plot toward the city and take 10%
damage.
*Expert:* coastal defence for a civ without a navy; it also traps raiders for your own ships.

**Satire of Kings** · I · needs Drama · building · [attitude lever]
Every 30 turns, publish a satire of a leader you have met. Every civ that has met that leader
gets -1 attitude to them for 30 turns; the target gets -2 to you.
*Expert:* isolate the leader before a council's war resolution or a crusade call.

**Masquerade** · II · needs Drama · spell (Bard-trained unit) · [new: false strength]
A stack shows to rivals as a weak stack of Warriors until it attacks or is attacked.
*Expert:* hide the army's size until it stands at their walls.

**Lullaby of Ceridwen** · II · needs Philosophy · national unit, 2 · [new: charm kind]
The Lutist's battle hex, *Lullaby*: the enemy's rear row skips its first action.
*Expert:* silences the enemy's archers and casters for a round; best against shooting
armies.

**The Unfinished Opera** · II · needs Drama · world wonder · [new: multi-stage]
It takes three Bards; each one spent adds an act. Each act gives +5% culture empire-wide and
+1 attitude from every civ. The third act counts double toward the culture victory.
*Expert:* the culture civ's place for its later Bards; for anyone else, one act is enough.

**Anthem** · II · needs Philosophy · building · [cultural channels]
This city's cultural channel falls off with distance, not distance squared.
*Expert:* aim a Foment Revolt at a rival's far, soft city from deep inside your land.

**Living Legend** · II · needs Arete · building · [new: governor to hero]
The city's governor gains a trait every 25 turns (up to 4). A governor with 3 or more traits
can be *Called to Arms*: it becomes a hero unit with its traits as promotions, and the city
takes a new governor from the bench.
*Expert:* grow a hero in a quiet city, then call him to the war that needs him.

**Ballad of the Traitor** · III · needs Malevolent Designs · building · [data: betrayal chance]
Units of civs at war with you that fight inside your borders have a 5% chance per battle round
to change sides.
*Expert:* makes invading you a gamble for the attacker; worth most to the defender of a long
border.

**Chorus of the Fallen** · III · needs Theology · building · [new: charm kind]
In a battle, each of your units killed gives the rest of your side +5% damage for the rest of
the battle. Armies trained in the city start each battle with Battle Hymn.
*Expert:* big attritional armies; useless with small elite stacks.

**Illusory Host** · III · needs Strength of Will · national unit, 2 · [new: decoys]
The Illusionist's battle spell adds 4 illusory soldiers to its side's front row: 60 HP each,
they deal no damage and vanish when a spell hits them.
*Expert:* in a siege, decoys soak the Sentinels' and archers' first volleys.

### 3.6 Deck notes

- **Count.** 61 cards: 12 per suit, 13 for the Prophet. The bands are spread across I-III:
  Sage 4/5/3, Prophet 4/7/2, Engineer 2/6/4, Merchant 4/5/3, Bard 4/5/3.
- **Strongest cards to watch:** Chronicle of Recall, The Free Company, the Index, Bastion,
  Wheeled City. They may need a hard limit or a bigger price.
- **Where the two decks touch:** the Orrery and Dagda's Balance read and steer the counter;
  Agares's Well and the Ledger of Mammon feed it; the Census of the Damned and Dragon Wake
  (below) give secret-tech draws.

---

## 4. The Armageddon deck (30 new cards)

### 4.1 How it runs (proposal)

- **Slots.** One every 10 points: 10, 20, ... 90, and 100 for the ending.
- **The draw.** At game start, one card per slot is drawn face down from the cards allowed at
  that slot (each card names its thresholds). A chain's later stages take the later slots its
  stages name.
- **Reveal.** A card is shown to everyone once the counter is within 5 of its slot. The Orrery
  shows two ahead.
- **Firing.** Once, when the counter first reaches the slot. A *lingering* card's effect ends if
  the counter falls back below its slot.
- **Culpability (new).** Each player's net points added to the counter (lowering counts
  negative). A card can hit the world, the most culpable, the most penitent, an alignment, or
  the score leader.
- **The pool.** FFH2's nine events go in as cards. The four Horsemen become one chain, so they
  no longer come in every game. With the 30 new cards, the pool is 36 cards for about 10 slots
  per game.

FFH2's events, as cards:

| Card | Threshold | Effect |
|---|---|---|
| Blight | 30 | Unhealthiness in every non-Infernal city; every living unit takes 25% death damage. |
| Pestilence | 35+, recurring | Blight for one player at a time. |
| The Four Horsemen | 40 → 50 → 60 → 70 | Stephanos, Buboes, Yersinia, Ars Moriendi, one per stage. |
| Hellfire | 80 | Empty flat plots become Hellfire with a demon Champion. |
| Wrath | 90 | Wrath appears; land units may become Enraged. |
| Apocalypse | 100 | Half of every city's population; units may die. |

### 4.2 Early (10-30)

**The Red Moon** · 20 · single
For 20 turns, barbarian and animal spawns ×3 and every lair spawns once more. Units outside
culture borders lose 5% HP a turn unless on a road.
*Expert:* pull scouts and settlers inside borders and clear the near lairs before it rises.

**Children of the Ash** · 20-30 · single
At every City Ruins on the map, a band of Pyre Zombies rises: one per ruin, plus one more every
5 turns for 20 turns.
*Expert:* the razers pay where they razed; build over your ruins or garrison near them.

**Whispers in the Wells** · 20 → 40 · chain
- *20:* river and lake cities lose their fresh-water health for good.
- *40:* a unit healing in a city without a Temple takes 10% poison damage instead.

*Expert:* Temples and Sanitation move up the list; the second stage punishes armies resting in
small cities.

**Census of the Damned** · 20-40 · single
Every civ's culpability is published. The most culpable gets the Mark of Agares: +10% attack,
and its units that die rise as Manes for the barbarians. The most penitent gets a free great
person of its choice, with its secret tech draw.
*Expert:* from the reveal on, lowering the counter is a race. The Hallowing of the Elohim and
Sphener become prizes.

**The Unbinding of Oaths** · 30 · single
Every standing deal ends. Peace treaties lose their protection (war can be declared at once).
Open borders close. Each vassal may break free.
*Expert:* garrison the borders before 30; a strong civ readies a strike for the turn it fires.

**The Mirror Hour** · 30 · single
Each civ's three strongest units get a barbarian double next to them.
*Expert:* split the heroes up or keep them in stacks that can beat themselves.

### 4.3 Middle (30-60)

**The Drowning Bells** · 30 → 50 → 70 · chain
- *30:* bells ring under the sea; -1 happiness in coastal cities.
- *50:* the sea rises. Every coastal flat land plot with no improvement and no city becomes
  Shallows.
- *70:* the Drowned rise from the new Shallows as barbarians.

*Expert:* improve the coast before 50, and settle and plan wars away from low shores. The Lanun
gain and everyone else loses land.

**The Long Winter** · 35 → 55 → 75 · chain (Mulcarn)
- *35:* the snow line moves 3 rows toward the equator for 40 turns (temporary terrain, the
  Thaw machinery run backwards).
- *55:* shallow coasts freeze into Ice: land units cross and ships cannot.
- *75:* the Avatar of Mulcarn (a new world unit) marches on the warmest capital.

*Expert:* food stocks and cold-native civs (Doviello, Illians); the ice opens new land routes
for one winter.

**Plague Ships** · 40 · single
A plague starts in the most culpable civ's capital and spreads along trade routes: an infected
city loses 1 population every 4 turns for 12 turns. *Quarantine* (a new city action) closes the
city's trade routes for 15 turns and blocks infection.
*Expert:* quarantine early and lose the trade, or hold the trade and lose the people; an
Herbalist halves the loss.

**The Ashen Sky** · 40-60 · lingering
For 15 turns: -1 food on grassland and plains; sight 1 for every unit; shots and stones -50% in
battles.
*Expert:* stock food, and fight with melee during it; archers wait.

**The Pale Election** · 40-60 · single
The most culpable evil civ becomes the Herald of Agares: +25% production and +10% attack for 30
turns. Every good civ is called to a crusade against it (the crusade machinery).
*Expert:* good civs ready their pledges; an evil civ near the top decides whether it wants the
crown.

**Demonic Embassies** · 40-60 · single, a choice for every civ
Every civ is offered an Infernal Pact: +25% production and +1 happiness in every city while it
holds, and +1 counter every 5 turns per pact. Good leaders remember who accepted (-2 attitude).
Acceptance is public.
*Expert:* take it as a sprint before a war and renounce it after; a good civ that takes it pays
for it with its friends.

**The Green Revenge** · 30-50 · single (Sucellus)
Each civ's forests cut over the game are counted (new). Treants rise in the forests near the
worst cutters' cities, more for more cut. Fellowship of Leaves civs are spared.
*Expert:* replant or keep the forest belts, and know your count before it fires.

**Rift Storms** · 50+ · recurring
Every 5 turns a rift opens at an unowned mana node and spills summoned creatures as barbarians.
An adept on the rift can *Close* it (-2 counter) or *Harness* it (the owner's summons +1 turn
of duration, +1 counter).
*Expert:* send adepts out; a summoner civ harnesses and everyone else closes.

**Starfall** · 50 · single (Ceridwen)
For 20 turns, every 5 turns a meteor strikes a random land plot inside each civ's borders. It
destroys the improvement and the units there and leaves a crater with a new mana node of a
random type, or Mithril.
*Expert:* spread units out; claim the craters in unowned land first, since they reshuffle who
has which sphere.

**The Fissure** · 50 · single
An earthquake splits the largest continent along the line between two capitals of different
civs: a ridge of peaks with one pass. Roads across it break.
*Expert:* the pass becomes the war's whole front; fortify it or race to it.

**The Last Market** · 40-60 · single (Mammon)
For 10 turns, gold buys anything:
- any civ's units at 3 times their hammer cost;
- techs at 4 gold a beaker;
- a city in disorder, from its owner, at a price by population.

*Expert:* hoard gold from the reveal; a city you could not take you may buy.

**Bloodmoon Tithe** · 50 · recurring every 10 turns while the counter is 50 or more
Each civ gives up one unit per 8 cities (its choice) or its largest city loses 2 population.
*Expert:* keep cheap tithe units (slaves, warriors); slaves become worth something.

**The Hollow Crown** · 60 · single
The score leader's most unhappy governed city breaks away as a barbarian city-state, led by its
governor, with half its units.
*Expert:* the leader keeps its governed cities content before 60; rivals foment revolt in them
before it fires.

**The Silence of the Gods** · 60 · lingering
For 20 turns, no religion spreads, divine spells fail half the time, and religious buildings
give no culture.
*Expert:* the arcane civs' window for a war; religious civs make peace or dig in.

### 4.4 Late (60-90)

**Ghost Garrisons** · 70 · single
Each city gets Ghost defenders equal to the units killed defending it over the game (at most 6),
for whoever owns it now.
*Expert:* take the city that has been fought over before 70, or hold it.

**The Long Nightmare** · 60-80 · lingering
For 20 turns, one trait of every leader is replaced at random (the Insane machinery).
*Expert:* plans that hang on one trait wait it out.

**Bone Roads** · 70 → 90 · chain
- *70:* roads of bone link each evil capital to the nearest good capital, through any terrain.
  Evil units get +1 move on them.
- *90:* the dead march them: barbarian undead walk toward the good capitals every 5 turns.

*Expert:* good civs pillage the bone road at its narrowest and fort the end; evil civs use it
to invade.

**Dragon Wake** · 60 → 80 → 95 · chain
- *60:* the dragons not yet in play (Acheron, Abashi, Eurabatres) wake in lairs.
- *80:* every 15 turns, each one flies at the richest city within 20 plots.
- *95:* a dragon slain from then on leaves a hoard: 500 gold and a secret tech draw.

*Expert:* units with damage against large units, and a plan for the hoard; the richest city
needs the most garrison.

**The Shepherd's Mercy** · 60-80 · single
Each civ that has lowered the counter gets an Angel. The most penitent gets the Repentant
Angel, a hero.
*Expert:* lowering the counter before the reveal pays for itself; the good civs race for the
hero.

**Exodus** · 80 · single
Every city in disorder or under siege loses 2 population. It reappears as Settlers in the
nearest friendly city of another civ; taking them in gives +1 attitude.
*Expert:* keep cities out of disorder; a stable empire next to a burning one gains settlers.

**The Black Tide** · 80-90 · lingering
Ships at sea lose 20% HP a turn outside a port. Krakens and Leviathans spawn.
*Expert:* get the armies across the water before 80; island civs are cut off for its length.

### 4.5 Endings (100)

One is drawn for slot 100; each is shown when the counter reaches 95.

**Apocalypse** · FFH2's: half of every city's population; units may die.

**Agares Unchained** · A new player, the Host of Agares, appears with an army in each of the
three most culpable civs' lands. When it is destroyed, the counter falls to 50 and the deck
deals new cards for slots 60-90.
*Expert:* the end of the world becomes a boss war; who helps destroy it matters for after.

**The Judgment** · Each civ is judged by its culpability. The guilty lose population in
proportion to their share; the penitent gain it.
*Expert:* the counter's history becomes the score; switch sides in time.

**The Unmaking** · All culture is reset: every city keeps only its own plot and regrows its
borders. Every deal ends.
*Expert:* armies in the field and settlers in hand claim the world anew; this ending rewards
units over buildings.

### 4.6 Deck notes

- **Count.** 30 new cards: 6 early, 14 middle, 7 late, 3 new endings. With the six FFH2 cards
  in the table above (Apocalypse among them), the pool is 36.
- **The counter's pace.** Scrapping the random events removes 26 outcomes that moved the
  counter, so its pace needs retuning with the deck.
- **New systems the deck leans on:** culpability; per-city and per-player history (ruins,
  defenders killed, forests cut); terraforming; area weather; quarantine. The rest uses existing
  levers.
