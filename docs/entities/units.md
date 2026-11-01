# Units

Part of the entity inventory: what each entry does, how to play it, and how far the AI uses it. How to read the entries, the AI grades and the AI machinery they cite are in [docs/ENTITIES.md](../ENTITIES.md).

| Section | Entries | Directed | Valued | Partial | Incidental | Unused | Not an AI choice |
|---|---|---|---|---|---|---|---|
| [Settlers, workers and constructs](#settlers-workers-and-constructs) | 11 | 2 | 7 |  | 1 | 1 |  |
| [Melee](#melee) | 38 | 1 | 26 | 5 | 6 |  |  |
| [Archery](#archery) | 10 |  | 6 | 1 | 3 |  |  |
| [Mounted](#mounted) | 21 |  | 11 | 7 | 3 |  |  |
| [Recon](#recon) | 23 | 2 | 2 | 18 | 1 |  |  |
| [Siege](#siege) | 4 | 4 |  |  |  |  |  |
| [Arcane](#arcane) | 8 | 3 | 1 | 1 | 3 |  |  |
| [Disciples](#disciples) | 29 |  | 14 | 6 | 8 | 1 |  |
| [Naval](#naval) | 12 | 4 |  | 3 | 2 | 3 |  |
| [Beasts](#beasts) | 2 |  |  | 1 |  |  | 1 |
| [Heroes](#heroes) | 47 | 4 | 22 | 11 |  |  | 10 |
| [Great people](#great-people) | 7 | 4 |  | 2 |  |  | 1 |
| [Summons](#summons) | 24 |  | 15 | 6 |  | 3 |  |
| [Equipment](#equipment) | 23 | 1 |  |  |  | 3 | 19 |
| [Animals](#animals) | 16 |  |  |  |  |  | 16 |
| [Barbarian, event and special units](#barbarian-event-and-special-units) | 50 | 1 | 1 | 1 | 8 | 9 | 30 |

## Settlers, workers and constructs

### Bone Golem (`UNIT_BONE_GOLEM`)
*Construct, tier 4 · 360 hammers · 13/13, 1 move · Divine Essence, a Sculptor's Studio (the Luchuirp Training Yard) and Ivory · Luchuirp only, replaces the Immortal · national unit, 4 at most*

**What it does.** The Luchuirp Immortal, but trainable: where everyone else must promote a unit to level 6 and upgrade it, the Luchuirp build 13-strength golems outright. It is immortal (reborn in the capital at 75% damage whenever it dies, as long as a capital stands) and starts with Cannibalize (+10 hit points healed after each win over a living unit) and Golem: immune to death and poison, immune to fear, slow healing (-10 in friendly land, -5 in neutral), and with no unit combat it can never take promotions. Nothing can upgrade into it.

**Playing it well.** Four unkillable 13s are the spearhead of a late Luchuirp army: send them first into the strongest defender, accept the loss, and walk them back from the capital. Since they never level, their deaths cost nothing but time; keep Ivory secured, because without it no more can be built. A reborn golem comes back at 75% damage and heals slowly, so it sits out several turns: rotate the four rather than throwing them all in at once.

**AI: Valued.** A Defense and Attack fighter (`production::role_ok`), scored by strength (`best_unit`), the Ivory and Studio checked by the rules' `city_can_train`; research counts Divine Essence as a Luchuirp unique-unit tech (`civs::unique_unit_tech_value`, 1500 when it completes the unit). It fights in the stacks and operations (`military::army`, `campaign.rs`) or garrisons. Immortality is not in its power, but the rules apply it on their own; the strike's "cheapest to lose" pick (`campaign::strike`) does not know the golem comes back.

### Clockwork Golem (`UNIT_CLOCKWORK_GOLEM`)
*Construct, tier 4 · 360 hammers · 15/9, 1 move, collateral 40 (up to 4 units, to 20%) · Machinery and a Sculptor's Studio · Luchuirp only, replaces the Berserker · national unit, 4 at most*

**What it does.** The Luchuirp Berserker: harder-hitting (15 attack against 11) and on Machinery instead of Rage, it damages up to four other units of a stack it attacks. It is a Golem (no promotions, slow healing, immune to death, poison and fear) and cannot be upgraded into.

**Playing it well.** It opens a stack attack: hit the biggest stack or the best-defended city first so the collateral softens everyone for the Iron Golems that follow. Its 9 defense makes it a poor stack defender, so it travels with Nullstone or Iron Golems. Four is the cap; spend them where collateral counts (stacks of three or more), not on lone units.

**AI: Valued.** It passes the Defense, Attack and Siege roles (`production::role_ok`; its default AI is `UNITAI_ATTACK`, so it is not a `siege_engine` and may be trained as an ordinary attacker); `best_unit` counts its collateral, double for CityAttack and with a bonus in the Siege role. In a strike, collateral attackers go first while two or more defenders stand (`campaign::simulate`, `campaign::strike`).

### Gargoyle (`UNIT_GARGOYLE`)
*Construct, tier 3 · 180 hammers · 8/8, 1 move, +25% city defense · Engineering and a Sculptor's Studio, 25% faster with Marble · Luchuirp only, replaces the Longbowman*

**What it does.** The Luchuirp's archer-line defender: 8 strength with +25% in cities and the Defensive promotion (fortification bonus doubled), plus the Golem traits (no promotions, slow healing, immune to death, poison and fear). It never goes obsolete, and nothing upgrades into it.

**Playing it well.** Garrison a Gargoyle or two in each border city once Engineering is in; with Marble they are cheap. Unlike a promoted Longbowman it never gains City Garrison levels, so against a late siege stack add a promoted defender and walls. Its main weakness is attrition: it heals slowly, so rotate damaged ones out of the front city.

**AI: Valued.** Its `UnitAIs` hold only `CITY_COUNTER` and `CITY_DEFENSE`, so it is trained only as a defender (`production::role_ok`, Defense: city defense 8 x 125% in `type_city_defense`), and the Studio comes through `production::enabler_building` when a city's best defender is a third weaker. The Defensive promotion is not in its power. An earlier Luchuirp-only rule that built Studios for it (docs/AI_LOG.md, "Sculptor's Studio for the Luchuirp") lost share and was reverted; the generic path now does the same job.

### Hawk (`UNIT_HAWK`)
*Recon (air), tier 1 · 20 hammers · no strength, 3-plot recon range · Hunting and a Hunting Lodge · every civilization*

**What it does.** A flying scout carried by Rangers, Hunters and Druids: it reveals up to three plots away and sees invisible land units, but cannot fight or pillage and gives no military happiness.

**Playing it well.** Cheap insurance against invisible units: a Hawk flown from a Hunter reveals Shadows, Assassins and hidden units before they strike a stack or a worker, and scouts a city's garrison before the siege commits.

**AI: Unused.** No production role accepts it: its `UnitAIs` list holds only `UNITAI_UNKNOWN` (the Explore role needs `EXPLORE` there), and its `iAIWeight` of -999 would sink its score anyway (`production::best_unit`). No AI ever has one, and nothing in `ffh-ai` flies air recon.

### Iron Golem (`UNIT_IRON_GOLEM`)
*Construct, tier 3 · 180 hammers · 10/10, 1 move · Iron Working, a Forge and Iron or Mithril · Luchuirp only, replaces the Champion*

**What it does.** The Luchuirp Champion: 10 strength where a Champion has 6 (plus weapons), with the Golem traits (no promotions ever, slow healing, immune to death, poison and fear) but without the Champion's +25% against melee. It never goes obsolete and nothing upgrades into it.

**Playing it well.** The bulk of a Luchuirp army from the middle game: stronger than any unpromoted rival unit of its tier, and losing one costs only hammers, never a veteran. Because it cannot take City Raider, bring Clockwork Golems or casters to break city defenses, and keep the Forge cities on Iron. Golems built in a city with a Blasting Workshop come with Fire II and can cast Fireball themselves.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), scored by strength per hammer (`best_unit`); the Forge comes from its own value or `production::enabler_building` (10 against an Axeman-class 6 is well over the third it asks), and the Iron or Mithril requirement is checked there too. It fights in the army and siege operations (`military::army`, `campaign.rs`). Since it can never be promoted, `units::upkeep` has nothing to pick for it.

### Mud Golem (`UNIT_MUD_GOLEM`)
*Worker, tier 1 · 100 hammers · 0/2, 2 moves, work rate 150 · no tech · Luchuirp only, replaces the Worker*

**What it does.** The Luchuirp worker: it builds every improvement a Worker builds half again as fast (work rate 150 against 100), and unlike a Worker it is not built with food, so the city keeps growing while it is in the queue. It is a Golem (immune to death and poison) with a defense of 2, though it cannot attack.

**Playing it well.** Build them from the capital's surplus while it grows instead of stalling on workers; two Mud Golems do the work of three Workers. They are still captured or killed by anything that reaches them, so keep them inside the borders and behind a garrison when barbarians roam.

**AI: Valued.** The Worker role (`production::role_ok`; `best_unit` scores workers by 100 + work rate per turn, so the golem's 150 counts) at the first-worker and worker-quota steps of `production::decide`; it works through `workers::worker` and flees danger (`workers::refuge`). That cities grow while it builds is not weighed. Measured: the Luchuirp lose 6.0 Mud Golems a game to barbarians, against 0.3-2.2 workers for the other civilizations (docs/AI.md, "Unique units in play"), a gap noted there as not fixed.

### Nullstone Golem (`UNIT_NULLSTONE_GOLEM`)
*Construct, tier 4 · 240 hammers · 13/13, 1 move · Mithril Weapons and a Sculptor's Studio, 25% faster with Marble · Luchuirp only, replaces the Phalanx · national unit, 4 at most*

**What it does.** The Luchuirp Phalanx, built for defense: 13 strength, Magic Immune (no spell affects it, and it resists cold, fire and lightning damage by half) and the Golem traits (no promotions, slow healing, immune to death, poison and fear). It loses the Phalanx's city attack bonus.

**Playing it well.** The answer to a magic-heavy neighbour: park one in each city a Sheaim, Amurite or Svartalfar army can reach, where damage spells, curses and Domination fail against it and fire or lightning summons hit it at half their bonus. In the field it holds a stack that enemy spells would otherwise strip. With four at most, put them where the enemy casters are.

**AI: Valued.** Trained only as a defender: its `UnitAIs` are `CITY_DEFENSE` and `RESERVE`, so `production::role_ok` gives it the Defense role and no other, and it garrisons (`units::role_of`: its default AI is `UNITAI_CITY_DEFENSE`). Its 13 is all the AI sees; the magic immunity is worth nothing in `type_city_defense`, and no city is picked for it by the threat of enemy casters.

### Settler (`UNIT_SETTLER`)
*Civilian, tier 1 · 120 hammers · 2 moves, defends only · no tech · every civilization · built with food; double speed for Expansive leaders*

**What it does.** Founds a city. While it is built the city's food surplus goes into it and the city does not grow; captured, it becomes a Worker. The Infernals and the Mercurians receive two and one when they enter the game.

**Playing it well.** Expansion is the strongest early investment: build settlers from cities at size 3-4 with a food surplus (the food converts into hammers), chop forests into them, and send each with an escort, which becomes the new city's first defender. Settle on rivers, coasts and resources, and claim the land toward your neighbours first. Stop when maintenance outruns what new cities bring, or when the sites left are poor.

**AI: Directed.** `expansion::settler_choice` decides when (any visible threat stops it; at most (cities + 1) / 2 at once, halved at war; the city grown to 3-4 first unless at its happy cap; a slow city leaves it to a fast one) and `expansion::settler` where: the best site by `AI_foundValue` (`sites::sites`) whose risk the escort covers, with an escort trained (`expansion::prepare_escorts`) and picked up, overseas by transport (`naval::settler_overseas`). Workers chop for it (`workers::clearing_value`). The Kuriotates settle settlements past their city limit (`civs::kuriotates`), and the Mercurians disband a settler with no site after 20 turns (`civs::mercurians`). Measured: settlers under a held threat founded 10% more cities by turn 100 and cost 7% of the techs by turn 300 (docs/AI_LOG.md, "Settlers under a threat the garrison holds").

### Slave (`UNIT_SLAVE`)
*Civilian, tier 1 · not trainable (Never) · 2 moves, work rate 50 · from the Slavery civic · upgrades to the Lunatic*

**What it does.** A half-speed worker: a civilization under Slavery may turn a living, non-animal unit it defeats into a Slave. It builds everything a Worker builds at half speed, can hurry production by a little (10 base), and upgrades to the Lunatic. The Balseraphs can add it to a Freak Show (it dies and leaves a Human Cage: +1 happiness, +1 culture, +1 Bard point); the Doviello can turn it into a Beastman for 20 gold; with the Slave Trade resolution passed it can be sold for 10 gold.

**Playing it well.** Slaves are free labour from a war: put them on roads and mines behind the front, upgrade the extras into Lunatics once you have an Asylum, or, as the Balseraphs, cage them in your cities for happiness, culture and Bard points.

**AI: Incidental.** The AI never adopts Slavery for slaves (the civic is valued generically, `empire::civic_value`); those it gets work as workers (`units::role_of`: `UNITAI_WORKER`, `workers::worker`) and count toward the worker quota (`production::count_ai`), so production trains fewer real workers. It never hurries with them (no hurry mission outside `units::great_person_build`), never sells them (Sell a Slave is `bAllowAI` 0), and may upgrade them into Lunatics through `units::upkeep`. The Balseraphs' Add to Freak Show is cast by FfH's terms (the Human Cage's building value plus `iAIWeight` 30, `casting::ffh_value`) whenever a slave stands in a city that can take it; the Doviello's Beastman cast only for workers beyond the quota (`civs::doviello`).

### Wood Golem (`UNIT_WOOD_GOLEM`)
*Construct, tier 2 · 90 hammers · 6/6, 1 move · Construction and a Sculptor's Studio · Luchuirp only, replaces the Axeman*

**What it does.** The Luchuirp's first golem: 6 strength where an Axeman has 4, on Construction instead of Bronze Working, with no weapon promotions. Golem traits (no promotions, slow healing, immune to death, poison and fear) and Vulnerable to Fire (fire damage resisted at -50%).

**Playing it well.** At Construction it outclasses every other civilization's tier-2 unit, which makes it the Luchuirp's window for an early war: a stack of Wood Golems with a few catapults takes cities Axemen cannot. Keep it away from fire users (Pyre Zombies, Fireballs, the Sheaim) and upgrade it to an Iron Golem once Iron Working comes. As a defender it is weaker than a promoted Dwarven Slinger.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), and the Studio comes through `production::enabler_building`, which compares strength and strength per hammer against the city's best: 6 against a Warrior's 3 opens it in up to half the cities. Measured: Wood Golems went from 14 to 34 a game (seeds 8201-8203) once `PrereqBuilding` buildings counted, they appear in 95% of Luchuirp games and attack twice as often as Axemen do elsewhere (docs/AI.md, "Unique units"); the Luchuirp economy paid for it (economy buildings at turn 300 23.7 -> 9.8). The fire vulnerability is not weighed.

### Worker (`UNIT_WORKER`)
*Civilian, tier 1 · 75 hammers · 2 moves, work rate 100, defends only · no tech · every civilization but the Luchuirp · built with food*

**What it does.** Builds improvements and roads, clears jungle and chops forests (a chop's hammers go to the nearest city). While it is built the city's food goes into it and the city does not grow. Captured, it stays a Worker for its captor. The Doviello can turn one into a Beastman for 20 gold.

**Playing it well.** One worker per city early, more for cottage and mine economies; improve the plots the city actually works first, connect resources, and chop forests into settlers and wonders. Keep them out of reach of barbarians and enemies: a lost worker is a tempo loss and a free worker for the enemy.

**AI: Directed.** `production::decide` trains the first one once something can be improved, then more up to `production::worker_target` (one a city up to three, one for every three worked unimproved plots, at most two a city), and the fallback up to two a city. `workers::worker` picks the best plot and build by the yield the working city needs, plans roads (`workers::road_plan`), chops only for a settler, a worker or a world wonder (`workers::clearing_value`), respects FfH's AI build bans, and flees danger (`workers::refuge`). The Doviello turn only surplus workers into Beastmen (`civs::doviello`); the elves never clear forest. Measured attempts are in docs/AI_LOG.md, "Workers and citizens".

## Melee

### Axeman (`UNIT_AXEMAN`)
*Melee, tier 2 · 60 hammers · 4/4, 1 move, +10% city attack · Bronze Working and a Training Yard in the city · the axeman of ten civilizations (Bannor, Elohim, Mercurians, Khazad, Sidar, Grigori, Hippus, Clan of Embers, Infernal, Illians) and the barbarians*

**What it does.** The first real melee soldier: a third stronger than the Warrior it replaces, with a small city attack bonus, and it takes the bronze, iron and mithril weapon promotions (+1, +2, +4 strength) its owner's resources give in its cities. It upgrades to the Champion or the Chariot.

**Playing it well.** The backbone of an early war: escort settlers, clear lairs and barbarian towns, and take a neighbour's capital before archers and walls come, ideally with Copper for the weapons and a few catapults for walls and stacks. Build the Training Yard in one or two production cities rather than everywhere, and promote toward City Raider (Warfare) for sieges or Shock against melee. Warriors upgrade into Axemen cheaply once the tech is in.

**AI: Valued.** Trained for the Defense and Attack roles (`production::role_ok`), scored by strength per hammer (`best_unit`); the Training Yard comes through `production::enabler_building` (4 against the Warrior's 3 is exactly the third it asks for). It fights in stacks and siege operations (`military::army`, `campaign.rs`), or garrisons when trained as a defender; Warriors are upgraded into it when the rules allow and gold suffices (`units::upkeep`). The weapon promotions are not counted in its power.

### Balor (`UNIT_BALOR`)
*Melee, tier 4 · 240 hammers · 7/3 +4 unholy, 1 move · Rage and a Training Yard · Infernal only, replaces the Berserker · national unit, 4 at most*

**What it does.** The Infernal's Berserker has no collateral damage; instead it is a demon (immune to death, poison, unholy damage and fear; holy damage hurts it 25% more) with Resist Fire and Stigmata, which adds half the Armageddon counter to its strength as a percentage. Its 4 unholy damage makes it an 11-strength attacker that holy and unholy resistances blunt. Ashen Veil priests can also summon one for a turn (Summon Balor), an Infernal Grimoire may release a barbarian one, and exploring a lair may raise one as a barbarian monster once the counter passes 40.

**Playing it well.** It grows with the end of the world: late in a game where the counter is high, a Balor out-hits anything of its tier. Use it on attack only (defense 3) and keep it away from Paladins, Angels and holy damage. The Infernals push the counter on purpose; four Balors are their late strike core.

**AI: Valued.** Defense and Attack roles (`production::role_ok`; its `UnitAIs` are `ATTACK` and `PILLAGE`, so not Siege), scored by strength plus damage type (`best_unit`); it fights as an army unit (`military::army`, `campaign.rs`). Stigmata's bonus, the counter it depends on, and the holy weakness are not in its power. The summoned Balor attacks at once when cast before an attack or at the end of the turn (`magic::summon_value`, `casting::summons_attack`).

### Battlemaster (`UNIT_BATTLEMASTER`)
*Melee, tier 3 · 150 hammers · 6/6, 1 move, +50% on tundra · Iron Working, no building · Doviello only, replaces the Champion*

**What it does.** The Doviello Champion: the same 6 strength without the Champion's +25% against melee, but no Training Yard needed and +50% attack and defense on tundra. A Doviello Sons of Asena becomes one for 80 gold (Upgrade to Battlemaster, Iron Working). It upgrades to the Immortal, Berserker and Phalanx lines (Skuld, Circle of Urd, Verdandi) and to the Paladin, Eidolon and Knight.

**Playing it well.** The Doviello's mid-game army: with no building to wait for, every city can train them, and on tundra, common in Doviello lands, they fight as 9s. Convert veteran Sons of Asena with the gold spell rather than build fresh ones; the upgrade keeps their promotions.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), by strength per hammer (`best_unit`); the tundra bonus is not counted. The Upgrade to Battlemaster cast is FfH's choice by its terms (`casting::ffh_value`: the Battlemaster's worth less the Sons of Asena's, 4 a gold for the 80 gold, plus `iAIWeight` 100: positive, so a Sons of Asena casts it at the end of a turn when nothing is worth more); `units::upkeep` also upgrades by the generic route. Measured: before the worker-AI fix, Battlemasters spent 28% of their turns on the worker AI (docs/AI.md, "Unique units in play"); now 0%.

### Beastman (`UNIT_BEASTMAN`)
*Melee, tier 1 · 25 hammers · 3/3, 1 move, +20% city attack, +25% city defense · no tech · Doviello only, replaces the Warrior*

**What it does.** The Doviello Warrior with a +20% city attack bonus on top of the Warrior's city defense. It takes bronze weapons only. For 40 gold and Bronze Working it becomes a Sons of Asena (Upgrade to Son of Asena); a Doviello Worker or Slave becomes a Beastman for 20 gold. With Octopus Overlords as state religion and a Temple of the Overlords in its city it can turn into a Drown for 60 gold.

**Playing it well.** The Doviello's early rush unit: a few Beastmen take barbarian camps and a careless neighbour's first cities. Convert workers into Beastmen only in an emergency (a city about to fall, a worker about to be caught): the improvements are worth more than a 3-strength unit. Upgrade the survivors into Sons of Asena by the gold spell.

**AI: Valued.** Defense and Attack roles by strength per hammer (`production::best_unit`), and a cheap settler escort (`expansion::escort_unit` takes the cheapest unit that defends 70% as well as the best). The Upgrade to Son of Asena is cast by FfH's terms at the end of a turn (`casting::ffh_value`: worth gained 150, less 160 for the gold, plus `iAIWeight` 50); the Drown conversion is never cast (worth gained 150 against 240 for the gold). Beastmen made from workers fight (`units::fighters_off_worker_ai`): before that fix 558 of them sat on the worker AI (docs/AI.md, "Unique units in play"); only workers beyond `production::worker_target` are converted (`civs::doviello`).

### Berserker (`UNIT_BERSERKER`)
*Melee, tier 4 · 240 hammers · 11/5, 1 move, collateral 40 (up to 4 units, to 20%) · Rage and a Training Yard · thirteen civilizations and the barbarians · national unit, 4 at most*

**What it does.** A shock attacker: 11 attack (5 defense) and collateral damage on up to four more units of the stack it attacks, down to 80% of their health. It takes all weapon promotions.

**Playing it well.** Open the assault on a stack or a city with the Berserkers, then send the Champions and Phalanxes into the softened defenders. Protect them with a good defender in the stack: on defense they are a 5. Four at most, so promote them toward Shock and City Raider and heal them between fights.

**AI: Valued.** It passes Defense, Attack and Siege (`production::role_ok`: `COLLATERAL` in its `UnitAIs` and collateral damage), so `best_unit` counts the collateral for attackers and the siege step may train it as a siege unit where no catapult can be trained. In a strike collateral attackers go first while two or more defenders stand (`campaign::simulate`, `campaign::strike`), and `military::assess` counts each collateral attacker extra when it weighs an assault. As a defender it counts at its 5 (`type_city_defense`), so it is seldom picked as one.

### Bloodpet (`UNIT_BLOODPET`)
*Melee, tier 1 · 25 hammers · 3/3, 1 move, +25% city defense · no tech · Calabim only, replaces the Warrior*

**What it does.** The Calabim Warrior, identical in the field (bronze weapons, +25% city defense), and food for vampires: a unit with the Vampire promotion stacked with a Bloodpet can Feed, sacrificing it to heal and attack again. It upgrades to the Moroi (Axeman line) or the Archer.

**Playing it well.** Early garrison and escort; later, a couple of Bloodpets in a vampire's stack let it fight twice in a turn of siege. Upgrade the rest into Moroi, the line that becomes vampires.

**AI: Valued.** Defense and Attack roles (`production::best_unit`), escorts, early garrisons. Feed is FfH's cast (`casting::ffh_value`: only its `iAIWeight` of 10) behind the rules' AI-only gate of 20+ damage on the vampire (docs/AI_ONLY_RULES.md, A2), so a hurt vampire standing with a Bloodpet eats it at the end of the turn; nothing keeps Bloodpets in the vampires' stacks for that.

### Boarding Party (`UNIT_BOARDING_PARTY`)
*Melee, tier 3 · 120 hammers · 5/5, 1 move, +25% against naval units · Iron Working and a Harbor · Lanun only, replaces the Champion*

**What it does.** The Lanun Champion: one point weaker, but with the Boarding promotion it can attack ships in adjacent water, and a ship it defeats becomes the Lanun's. It takes all weapon promotions and upgrades on the Champion's lines.

**Playing it well.** Garrison coastal cities with them and pounce on ships that come within reach, especially loaded transports and lone frigates; a captured warship is worth more than the Boarding Party. On land it is a slightly weak Champion, so keep the real sieges for Swordsmen upgraded to it with promotions.

**AI: Partial.** Its `UnitAIs` (`CITY_COUNTER`, `COUNTER`, `RESERVE`) give it only the Defense role (`production::role_ok`), so it is trained as a garrison, never as an attacker; Lanun Swordsmen become Boarding Parties by `units::upkeep`. Nothing looks for ships to board: a ship is attacked only when it happens to be next to the unit and the odds clear the bar of a garrison (`units::attack_adjacent`, 80%) or of a stack (`military::best_adjacent`). There is no naval combat logic in `ffh-ai`.

### Brujah (`UNIT_BRUJAH`)
*Melee, tier 4 · 240 hammers · 11/5, 1 move, collateral 40 (4 units, to 20%) · Rage and a Training Yard · Calabim only, replaces the Berserker · national unit, 4 at most*

**What it does.** A Berserker that is also a vampire: the same strength and collateral damage, plus Body I, Channeling I-II, Death I and the Vampire promotion (+10% strength, faster healing). It can Feast (in a Calabim city of size 3+, a citizen dies and it gains population - 2 experience), Feed on Bloodpets, Gift Vampirism to level 6+ units on its plot, cast Haste on its stack and Raise Skeleton.

**Playing it well.** Feast it to level fast in a city with unhappy citizens to spare, then lead stacks: Haste before the attack, collateral on the defenders, and a Skeleton raised every turn as fodder. Gift Vampirism makes veterans vampires. Four at most; their low defense needs a defender in the stack.

**AI: Valued.** Trained by strength and collateral (`production::role_ok`: Defense, Attack, Siege). Its spells are cast by the generic casting: Haste and Raise Skeleton by `magic.rs` (a lasting skeleton is worth its strength while at war, `magic::summon_value`), Feast and Gift Vampirism by `civs::calabim` (Feast only in a city above its happy cap and below 30 experience; Gift by the promotion for each level 6+ unit, level 4+ Moroi, on the plot). A Moroi upgraded from a garrison stays a garrison: Brujahs spent 57% of their turns in garrisons (docs/AI.md, "Unique units in play").

### Champion (`UNIT_CHAMPION`)
*Melee, tier 3 · 120 hammers · 6/6, 1 move, +25% against melee · Iron Working and a Training Yard · thirteen civilizations*

**What it does.** The upgrade of Axemen and Swordsmen: 6 strength, +25% against melee units, all weapon promotions. It upgrades to the Immortal (at level 6), Berserker, Phalanx, Paladin, Eidolon and Knight. Hellfire spawns them, and the Infernals start with two.

**Playing it well.** The main conquest unit of the classical age. Take Iron for +2, promote toward City Raider for sieges and Combat or Shock for field armies, and plan each veteran's upgrade: Phalanx for city assaults, Paladin or Eidolon by religion, Immortal for an undying level-6 unit.

**AI: Valued.** Defense and Attack roles by strength per hammer (`production::best_unit`), the Training Yard from `production::enabler_building`; the army's and the siege operation's main attacker (`campaign::army_choice`, CityAttack at war). Axemen and Swordsmen are upgraded into it (`units::upkeep`), and its own upgrade is the best by power when the rules allow it (a Phalanx, 12, before a Berserker, 11, before an Immortal, 9). The +25% against melee is not counted.

### Circle of Urd (`UNIT_CIRCLE_OF_URD`)
*Melee, tier 4 · 240 hammers · 11/5, 1 move, collateral 40 (4 units, to 20%) · Rage, no building · Doviello only, replaces the Berserker · national unit, 4 at most*

**What it does.** The Doviello Berserker: identical in combat, but it needs no Training Yard.

**Playing it well.** As the Berserker: the stack opener of a late Doviello army, trainable in any city once Rage is in.

**AI: Valued.** As the Berserker (`production::role_ok`: Defense, Attack, Siege; collateral first in a strike, `campaign::strike`).

### Demagog (`UNIT_DEMAGOG`)
*Melee, tier 3 · 60 hammers · 5/5, 1 move, +25% against melee · Iron Working, a Training Yard, the Crusade civic · Bannor only · leaves if Crusade ends*

**What it does.** A half-price Champion for the Bannor at war: 5 strength and +25% against melee for 60 hammers, but it abandons its owner when Crusade is no longer the civic. Under Crusade each town of the Bannor without an enemy in sight also has a 20% chance a turn to spawn a Demagog and shrink to a village.

**Playing it well.** Crusade is a war footing: switch to it for a decisive war, flood the front with Demagogs, take the cities, and accept that they vanish with the peace. Do not hold Crusade in a long war you cannot win: it bans settlers and workers and eats your towns.

**AI: Valued.** Defense and Attack roles (`production::role_ok`): at half a Champion's price it often wins `best_unit`'s strength-squared-per-turn score while Crusade lasts. Crusade itself is adopted in any war by its `iAIWeight` (`empire::civic_value`); holding it for won wars changed nothing measurable (`civs::bannor`, docs/AI_LOG.md, "Bannor Crusade only in won wars"). Nothing plans for the desertion when Crusade ends (the conquest flag the rules give it on creation is not read by `ffh-ai`).

### Diseased Corpses (`UNIT_DISEASED_CORPSE`)
*Melee, tier 2 · 90 hammers · 4/4 +2 death, 1 move · Corruption of Spirit, Ashen Veil as state religion and in the city · every civilization*

**What it does.** An undead melee unit (immune to death, poison and fear; holy damage hurts it 50% more) that carries the Diseased promotion and passes it on: the enemies it fights may catch it (-30% strength and slower healing on living units). By the data the promotion also weakens the Corpse itself by 30%. No military happiness; it upgrades to the Eidolon.

**Playing it well.** A plague cart, not a fighter: send Corpses into the stack you are about to attack so the defenders catch Diseased, then strike with real units. Undead, golem and demon armies are immune to the disease, and a Medic II unit cures it.

**AI: Partial.** Defense and Attack roles (`production::role_ok`), and `best_unit` scores it at 4 + 2 (the damage type counts, the Diseased penalty on itself does not), so the AI overrates it as a 90-hammer attacker and never uses it to spread disease on purpose: it fights where `military::army` and `campaign::strike` send attackers.

### Dragon Slayer (`UNIT_DRAGON_SLAYER`)
*Melee, tier 3 · 120 hammers · 6/6, 1 move, +25% against melee · Iron Working and a Training Yard · Grigori only, replaces the Champion*

**What it does.** The Grigori Champion with Courage (immune to fear, +10 healing for units on its plot) and Dragon Slaying (+40% against dragons).

**Playing it well.** Use it as a Champion; the fear immunity matters against Spectres, Wraiths and dragons, whose Fear makes ordinary soldiers refuse to attack. Against Acheron's lair or a rival's dragon, bring Dragon Slayers.

**AI: Valued.** As the Champion (`production::best_unit`, `production::enabler_building`, `campaign.rs`). Courage and Dragon Slaying are not in its power; nothing sends it after dragons.

### Drown (`UNIT_DROWN`)
*Melee, tier 2 · 90 hammers · 3/3 +1 unholy, 1 move · Message from the Deep, Octopus Overlords as state religion and in the city · every civilization · upgrades to the Stygian Guard*

**What it does.** The Overlords' undead infantry: an Axeman's 4 total strength, Undead (immune to death, poison, disease and fear), Vulnerable to Fire, and Water Walking: it walks on coast plots (not ocean) and attacks from them. No military happiness. A Warrior in a city with a Temple of the Overlords can become one for 60 gold.

**Playing it well.** Raid along the coast without boats: Drowns cross bays and inlets to hit cities and workers from the water. They counter Diseased Corpses (immune) and are the cheapest path to Stygian Guards. Keep them away from fire.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), by strength including its unholy point (`best_unit`). The rules' path finder lets it walk the coast, but nothing in `ffh-ai` plans coastal raids. The Warrior-to-Drown conversion is never cast (FfH's terms: worth gained 150 against 240 for the gold, `casting::ffh_value`).

### Flagbearer (`UNIT_FLAGBEARER`)
*Melee, tier 2 · 120 hammers · 5/5, 1 move · Fanaticism and the Crusade civic · Bannor only*

**What it does.** A Bannor standard-bearer: it starts with Morale (+10% attack and defense, +10% city attack) and casts Morale on every living unit of ours within one plot; the promotion wears off at 10% a turn.

**Playing it well.** One Flagbearer per assault stack: cast Morale the turn before and the turn of the attack, and the whole stack fights 10% better. It is a support unit, not a front-line fighter; protect it.

**AI: Partial.** Production sees only its 5 strength for 120 hammers (`production::best_unit`), so it rarely wins a slot: the Bannor trained one in 18% of the games they could (56%) (docs/AI.md, "Unique units in play"). Once it exists the generic casting uses it well: Morale is a blessing `magic.rs` casts on the attackers before a strike (`magic::bless`) and as a lasting blessing at the end of the turn; one in a stack casts it every turn.

### Freak (`UNIT_FREAK`)
*Melee, tier 1 · 60 hammers · 3/3, 1 move · Festivals · Balseraphs only*

**What it does.** A Balseraph carnival recruit with Mutated: at creation each mutation promotion (good and bad: Strong, Weak, Cannibalize, Sentry, Vulnerable to Fire and others) is rolled at 8%. It takes bronze and iron weapons and upgrades to the Axeman, Hunter or Archer lines. A Freak can be spent to build a Freak Show in a city (+1 happiness, +2 culture, +1 Bard point; no city can build one otherwise).

**Playing it well.** Keep the freaks that roll strong promotions for the army and turn the weak ones into Freak Shows in your bigger cities, where the happiness and Bard points pay back the 60 hammers quickly.

**AI: Partial.** Defense and Attack roles (`production::role_ok`); its `iAIWeight` of 100 adds a point of strength to its score (`best_unit`). It fights as an army unit and never builds a Freak Show: the AI constructs buildings with units only for great people's shrines (`units::great_person_build`). The rolled promotions only reach the upgrade choice (`units::upkeep`, by power).

### Grigori Medic (`UNIT_GRIGORI_MEDIC`)
*Melee, tier 3 · 180 hammers · 4/4, 1 move · Medicine and an Infirmary · Grigori only · upgrades to the Druid*

**What it does.** A field surgeon: Medic I-III (+10, +15, +20 healing for units on its plot, +20 for those next to it), Cure Disease (removes Diseased, Plagued, Withered) and Heal (20 hit points to every living unit on its plot, removes Poisoned). It may enter rival territory, cannot pillage, and takes weapons like any melee unit.

**Playing it well.** One Medic per siege stack lets the stack heal where it stands between assaults instead of walking home; with Medic III the plots around heal too. Cast Heal after the strike, and keep the Medic out of the fighting itself.

**AI: Directed.** `civs::grigori` trains it at war with a civilization once the army has 4+ attackers, one Medic for every four attackers (at most 3), as an army unit that joins the stacks, and builds an Infirmary in the most productive city if none has one. In the stacks Heal and Cure Disease are cast by `magic.rs` (Heal valued as the hit points it restores, `magic::py_value`). Measured: 0.9 Medics a game with no measurable effect in round 5 (docs/AI_LOG.md, "Grigori Medics at war"); Grigori Medics now appear in 30% of their games (docs/AI.md, "Unique units").

### Immortal (`UNIT_IMMORTAL`)
*Melee, tier 4 · 240 hammers · 9/9, 1 move · Divine Essence and a Training Yard · by upgrade only, from a level-6 unit · fifteen civilizations · national unit, 4 at most*

**What it does.** A veteran who cannot die: when killed it is reborn in the capital at 75% damage (as long as there is a capital), every time. It cannot be built; a level-6 Champion (or Battlemaster, Ogre, Mimic, Vampire, Boarding Party, Dragon Slayer, Monk) is upgraded into it.

**Playing it well.** Make your best veterans Immortals once their promotions are worth keeping: they can attack at poor odds, hold a doomed city, or dive behind enemy lines, and come back. Four at most; choose the units with the best promotions.

**AI: Incidental.** The AI gets it only by upgrade (`units::upkeep`, the best upgrade by power, gold allowing), and Phalanx (12) and Berserker (11) outrank it (9) when the rules allow those: an Immortal comes when they do not (tech, national limit). Immortality is not counted anywhere: the strike's sacrifice pick (`campaign::strike`) and the army's odds bars (`military::attack_ok`) treat it as a mortal unit.

### Lunatic (`UNIT_LUNATIC`)
*Melee, tier 3 · 90 hammers · 7/4, 1 move, collateral 20 (4 units, to 10%) · Mind Stapling and an Asylum (Octopus Overlords) · every civilization · upgrades to the Berserker*

**What it does.** A cheap, fragile attacker: 7 attack, 4 defense, no defensive bonuses at all, a little collateral damage, and Crazed, which now and then makes it Enraged (+1 move, +20% attack, -20% defense, out of its owner's control until it fights). Slaves upgrade into Lunatics.

**Playing it well.** Lunatics are expendable siege attackers at 90 hammers: throw them first at a city or stack, let the collateral wear the defenders down, and keep them out of the line of counterattack, where their 4 defense with no fortification bonus dies. An Overlords civilization with slaves can upgrade them into Lunatics, then into Berserkers.

**AI: Valued.** Defense, Attack and Siege roles (`production::role_ok`: `ATTACK_CITY` and collateral), scored with its collateral (`best_unit`); the Asylum comes from its own value or `production::enabler_building` (7 against an Axeman's 4 clears the third). `type_city_defense` does not know it gets no defensive bonuses, so it can be picked as a garrison where nothing better is trainable. Enraged needs no handling for an AI player.

### Mimic (`UNIT_MIMIC`)
*Melee, tier 3 · 120 hammers · 5/5, 1 move · Iron Working and a Training Yard · Balseraphs only, replaces the Champion*

**What it does.** The Balseraph Champion: a point weaker and without the +25% against melee, but each time it wins it steals a random promotion the loser had and it lacks (not equipment, race, weapons, Channeling III, Divine or Great Commander).

**Playing it well.** Feed it fights against veterans: a Mimic that beats promoted units collects their Combat, Shock, Woodsman and the like, and outgrows any Champion. Pick its fights at good odds (it is only 5 to start), against units with many promotions, and upgrade the best ones on the Champion lines.

**AI: Valued.** As the Champion: Defense and Attack roles by strength per hammer (`production::best_unit`), the Training Yard from `production::enabler_building`, army and siege operations (`campaign.rs`). The stolen promotions come by themselves; nothing picks its fights for them.

### Moroi (`UNIT_MOROI`)
*Melee, tier 2 · 60 hammers · 4/4, 1 move, +10% city attack · Bronze Working and a Training Yard · Calabim only, replaces the Axeman*

**What it does.** The Calabim Axeman, with Burning Blood: it can enrage itself (+1 move, Blitz, +20% attack and defense, wearing off at 10% a turn), at a 20% betrayal chance (the unit may go over to the barbarians). It upgrades to the Vampire (Champion line) or the Chariot, and a level-4 Moroi can receive Gift Vampirism from a vampire.

**Playing it well.** The Calabim's line toward vampires: level Moroi on barbarians, give the level-4 ones Vampirism, and upgrade the veterans into Vampires once Feudalism and a Governor's Manor are in. Burning Blood is a gamble for a decisive attack only.

**AI: Valued.** As the Axeman (`production::best_unit`, `production::enabler_building`, `units::upkeep` into the Vampire). Burning Blood is `bAllowAI` 0, so the AI never casts it. `civs::calabim` counts level-4+ Moroi as recipients when it values Gift Vampirism. Moroi trained as garrisons stay garrisons when upgraded (docs/AI.md, "Unique units in play").

### Ogre (`UNIT_OGRE`)
*Melee, tier 3 · 180 hammers · 8/8, 1 move · Iron Working and a Training Yard · Clan of Embers (and the barbarians), replaces the Champion*

**What it does.** The Clan's Champion: 8 strength for 180 hammers (Orcish race), with all weapon promotions, and no +25% against melee. It upgrades to the Ogre Warchief, the Berserker and the Stoneskin Ogre; nothing upgrades into it. A barbarian city founded once the barbarians know Iron Working starts with an Ogre.

**Playing it well.** The Clan's heavy infantry: two strength above a Champion makes it the best tier-3 unit for holding a city and for breaking one. It is slow and costly, so mass it in production cities with Warrens, where each Ogre comes with a free copy.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), by strength squared per turn (`best_unit`, where its 8 outweighs its price); the Training Yard through `production::enabler_building`; the Warrens that double it are valued by `civs::clan_of_embers::building_value`. It fights in the army and siege operations (`campaign.rs`) and upgrades by power (`units::upkeep`: the Stoneskin Ogre, 14, first).

### Ogre Warchief (`UNIT_OGRE_WARCHIEF`)
*Melee, tier 4 · 240 hammers · 11/11, 1 move · Divine Essence and a Training Yard · by upgrade only, from a level-6 unit · Clan of Embers (and the barbarians), replaces the Immortal · national unit, 4 at most*

**What it does.** The Clan's Immortal: 11 strength, reborn in the capital at 75% damage whenever it dies.

**Playing it well.** As the Immortal, with two more strength: make the Clan's best level-6 Ogres Warchiefs and use them where losing a unit is otherwise unacceptable.

**AI: Incidental.** Only by upgrade (`units::upkeep`), where the Stoneskin Ogre (14) outranks it by power when the rules allow; immortality is not counted.

### Phalanx (`UNIT_PHALANX`)
*Melee, tier 4 · 240 hammers · 12/12, 1 move, +25% city attack · Mithril Weapons and a Training Yard · sixteen civilizations · national unit, 4 at most*

**What it does.** The strongest common melee unit: 12 strength, +25% against cities, all weapon promotions (+4 with Mithril).

**Playing it well.** The finishers of a late siege: bombard and soften with siege engines, magic or disease, then send the Phalanxes in. Four at most, so make them out of promoted Champions (City Raider) rather than building fresh ones when you can.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), the CityAttack role's favourite at war (`best_unit` multiplies by the city attack bonus); Champions and other level-6 units upgrade into it first (`units::upkeep`, by power). It fights in the siege operations (`campaign.rs`).

### Pyre Zombie (`UNIT_PYRE_ZOMBIE`)
*Melee, tier 2 · 60 hammers · 3/3 +1 fire, 1 move, +10% city attack · Bronze Working, no building · Sheaim only, replaces the Axeman*

**What it does.** The Sheaim Axeman: undead (immune to death, poison, disease and fear), Resist Fire, and when it dies it explodes: 10 fire damage to every unit, friend or foe, on the eight plots around it, and a 20% chance to set each adjacent forest or jungle ablaze. No military happiness, and no upgrade.

**Playing it well.** Expendable front-line troops: attack with them first and let the explosions wound the defenders and the stacks next to the city. Keep your own stacks off the plots adjacent to where they die, and do not fight next to your own forests.

**AI: Valued.** Defense and Attack roles (`production::role_ok`, 3 + 1 fire in `best_unit`); with the Sheaim's Training Yard blocked it is their main attacker. The explosion is not valued: neither the damage to enemies nor the risk to our own stacks around it.

### Radiant Guard (`UNIT_RADIANT_GUARD`)
*Melee, tier 2 · 90 hammers · 4/4, 1 move, +10% city attack · Honor, the Empyrean as state religion and in the city · every civilization · upgrades to the Champion or the Ratha*

**What it does.** The Empyrean's axeman: no building needed, it sees invisible units, and with a free Sun II it casts Blinding Light (enemy units within one plot are held for 2 turns; it can be resisted, at +30% to the chance).

**Playing it well.** Escort stacks with one to catch Shadows and Assassins, and cast Blinding Light on an approaching enemy stack to freeze it for your archers and catapults, or on a city's defenders the turn before your attack so they cannot sally.

**AI: Valued.** Defense and Attack roles (`production::best_unit`). Blinding Light is modelled by `magic.rs` (immobility): a Radiant Guard next to a target casts it on the defenders before an attack (`magic::soften`), and at the end of the turn on enemies that can reach us. Seeing invisible units is not weighed anywhere.

### Repentant Angel (`UNIT_REPENTANT_ANGEL`)
*Melee, tier 4 · 240 hammers · 12/12 +4 holy, 3 moves · Warhorses · by upgrade only, from a level-3 Angel · Mercurians only · national unit, 4 at most*

**What it does.** A three-move angel (not alive: immune to fear; resists holy and poison fully, death by half) with 16 total strength. It pays for its repentance: each time it defeats a living unit while above 5 strength, it loses 5 strength for good. It can sacrifice itself for research at a Pyre of the Seraphic.

**Playing it well.** A hunter of demons and undead, which do not cost it strength: point it at Infernal and Sheaim armies, Wraiths and Balors. Against living enemies it burns out in two kills, so spend those on targets that matter.

**AI: Incidental.** Only by upgrade from Angels (`units::upkeep`, which picks it over the Valkyrie by power: 16 against 9). The strength loss against living units is not weighed, so it fights whatever the army fights; Sacrifice is `bAllowAI` 0.

### Skuld (`UNIT_SKULD`)
*Melee, tier 4 · 240 hammers · 9/9, 1 move · Divine Essence, no building · by upgrade only, from a level-6 unit · Doviello only, replaces the Immortal · national unit, 4 at most*

**What it does.** The Doviello Immortal, needing no Training Yard: 9 strength, reborn in the capital at 75% damage when it dies.

**Playing it well.** As the Immortal: turn the Doviello's best level-6 Battlemasters into Skulds when promotions matter more than raw strength.

**AI: Incidental.** Only by upgrade (`units::upkeep`), where the Verdandi (12) and the Circle of Urd (11) outrank it by power when the rules allow.

### Soldier of Kilmorph (`UNIT_DWARVEN_SOLDIER_RUNES`)
*Melee, tier 2 · 90 hammers · 4/4, 1 move · Way of the Earthmother, Runes of Kilmorph as state religion and in the city · every civilization · upgrades to the Paramander*

**What it does.** The Runes' early soldier: an axeman's strength with no Training Yard, the Dwarven race, and it can be spent to hurry production (45 hammers at normal speed).

**Playing it well.** For a Runes civilization that founded the religion early: defenders before Bronze Working, and hammers on legs for new cities, where a Soldier sacrificed into a monument or a worker saves many turns.

**AI: Partial.** Defense and Attack roles (`production::role_ok`, by strength per hammer: at 90 hammers it wins mainly where no Training Yard stands). It never hurries production: the only hurry mission the AI gives is a Great Engineer's wonder rush (`units::great_person_build`).

### Sons of Asena (`UNIT_SONS_OF_ASENA`)
*Melee, tier 2 · 60 hammers · 4/4, 1 move, +10% city attack · Bronze Working, no building · Doviello only, replaces the Axeman*

**What it does.** The Doviello Axeman, with no Training Yard needed. Beastmen become one for 40 gold; it becomes a Battlemaster for 80 gold with Iron Working (Upgrade to Battlemaster). It upgrades to the Champion line (Battlemaster) or the Chariot.

**Playing it well.** The Doviello's early army: every city trains it, and the gold upgrades keep veterans growing along the Beastman, Sons of Asena, Battlemaster line. Spend the treasury on those upgrades rather than on new units.

**AI: Valued.** Defense and Attack roles (`production::best_unit`); the Upgrade to Battlemaster is cast by FfH's terms at the end of a turn (positive: worth gained 300, 320 for the gold, `iAIWeight` 100; `casting::ffh_value`). A gold reserve for these upgrade spells was tried and lost: fewer Sons of Asena (docs/AI_LOG.md, "Doviello").

### Stoneskin Ogre (`UNIT_STONESKIN_OGRE`)
*Melee, tier 4 · 240 hammers · 14/14, 1 move, +25% city attack · Mithril Weapons and a Training Yard · Clan of Embers (and the barbarians), replaces the Phalanx · national unit, 4 at most*

**What it does.** The Clan's Phalanx, two points stronger, and starting with Stoneskin (+2 defense, +3 first strikes, half damage from every damage type) until its first victory removes it.

**Playing it well.** The strongest melee unit any civilization can train: lead with it into the city's best defender while Stoneskin's first strikes still hold, then heal. Upgrade promoted Ogres into it.

**AI: Valued.** Defense and Attack roles (`production::role_ok`), by its 14 (`best_unit`; Stoneskin's first strikes and defense are not counted, as promotions never are); Ogres upgrade into it first (`units::upkeep`). It fights in the siege operations (`campaign.rs`).

### Stygian Guard (`UNIT_STYGIAN_GUARD`)
*Melee, tier 3 · 120 hammers · 5/5 +2 unholy, 1 move · Fanaticism, a Temple of the Overlords, Octopus Overlords as state religion and in the city · every civilization · upgrades to the Eidolon*

**What it does.** The Overlords' champion: a demon (immune to death, poison, unholy damage and fear; holy hurts it more) with March (heals every turn, even moving, +10 healing) and Water Walking (it walks and attacks across coast plots). Drowns and Zealots upgrade into it.

**Playing it well.** A self-healing raider of coastal lands: it crosses bays without ships and keeps fighting where others must rest. Pair it with Drowns for an amphibious army without transports.

**AI: Valued.** Defense and Attack roles (`production::role_ok`, 5 + 2 unholy in `best_unit`); the Temple of the Overlords comes from its own value or `production::enabler_building`. March and Water Walking are not in its power, and nothing plans coastal raids.

### Swordsman (`UNIT_SWORDSMAN`)
*Melee, tier 2 · 60 hammers · 4/4, 1 move, +10% city attack · Bronze Working and a Training Yard · the axeman of seven civilizations (Malakim, Kuriotates, Ljosalfar, Lanun, Amurites, Balseraphs, Svartalfar)*

**What it does.** Identical to the Axeman in every number; only the art and the civilizations differ. It upgrades to the Champion (or that civilization's version: Boarding Party, Mimic) or the Chariot.

**Playing it well.** As the Axeman.

**AI: Valued.** As the Axeman (`production::role_ok`, `best_unit`, `enabler_building`, `units::upkeep`).

### Valkyrie (`UNIT_VALKYRIE`)
*Melee, tier 4 · 10 hammers · 6/6 +3 holy, 1 move · Divine Essence · by upgrade only, from a level-6 Angel · Mercurians only · national unit, 4 at most*

**What it does.** An immortal angel (reborn in the capital at 75% damage): 9 total strength, the Angel race, and Sacrifice at a Pyre of the Seraphic. Its 10-hammer cost makes the upgrade nearly free.

**Playing it well.** An undying stack defender: put a Valkyrie in the field stacks to absorb the first attacks, every turn, at no lasting cost.

**AI: Incidental.** Only by upgrade (`units::upkeep`), where the Repentant Angel (16 total strength) outranks it whenever the rules allow both, so an Angel becomes a Valkyrie only when Warhorses is missing or four Repentant Angels exist. Immortality is not counted; Sacrifice is `bAllowAI` 0.

### Vampire (`UNIT_VAMPIRE`)
*Melee, tier 3 · 180 hammers · 5/5, 1 move · Feudalism and a Governor's Manor · Calabim only, replaces the Champion*

**What it does.** The Calabim's signature unit: 5 strength with the Vampire promotion (+10% attack and defense, +5 healing), Body I, Channeling I-II and Death I. It Feasts (in a Calabim city of size 3+, a citizen dies, the city gets 3 turns of hurry anger, and the Vampire gains population - 2 experience), Feeds on Bloodpets, Gifts Vampirism to level 6+ units (level 4+ Moroi) on its plot, and casts Haste and Raise Skeleton. It upgrades to the Vampire Lord (level 12), the Brujah, Paladin, Eidolon and Knight.

**Playing it well.** Feast in big cities with unhappy citizens to level vampires quickly, spread Vampirism to your veterans, and lead with Hasted, skeleton-screened stacks. Their power is experience, not strength: a level-10 vampire with a few skeletons is worth a stack.

**AI: Valued.** Trained for strength (`production::role_ok`: Defense, Attack); at 5 for 180 hammers and needing a Governor's Manor, it comes mostly by Moroi upgrades (`units::upkeep`), from turn 280 on in the traced games (docs/AI_LOG.md, "Vampires, Feast..."). The rules put every new Vampire on `UNITAI_FEASTING`, which no part of the AI plays; `civs::calabim::turn` puts it back on `UNITAI_ATTACK`. Feast and Gift Vampirism are valued by `civs::calabim` (Feast only above the happy cap and below 30 experience), Haste and Raise Skeleton by `magic.rs`. Vampires spent 49% of their turns in garrisons (docs/AI.md, "Unique units in play").

### Vampire Lord (`UNIT_VAMPIRE_LORD`)
*Melee, tier 4 · 240 hammers · 9/9, 1 move · Divine Essence and a Training Yard · by upgrade only, from a level-12 unit · Calabim only, replaces the Immortal · national unit, 4 at most*

**What it does.** An immortal vampire (reborn in the capital at 75% damage) with Channeling III and Mind I added to the Vampire's spells, so it can also cast Inspiration (a city gets +2 research and +1 Sage point).

**Playing it well.** The end of a Feasting career: a level-12 vampire becomes an undying caster-warrior that leads every assault.

**AI: Incidental.** Only by upgrade at level 12 (`units::upkeep`), where the Brujah (11, no level needed) outranks it, so a Vampire may become a Brujah long before level 12. Its spells are cast as the Vampire's are (`civs::calabim`, `magic.rs`; Inspiration as a city spell, a quarter of the building's value, `magic::turn_value`).

### Verdandi (`UNIT_VERDANDI`)
*Melee, tier 4 · 240 hammers · 12/12, 1 move, +25% city attack · Mithril Weapons, no building · Doviello only, replaces the Phalanx · national unit, 4 at most*

**What it does.** The Doviello Phalanx, needing no Training Yard.

**Playing it well.** As the Phalanx.

**AI: Valued.** As the Phalanx (`production::role_ok`, `best_unit`; Battlemasters upgrade into it first, `units::upkeep`).

### Warrior (`UNIT_WARRIOR`)
*Melee, tier 1 · 25 hammers · 3/3, 1 move, +25% city defense · no tech · every civilization but the Mercurians, Doviello and Calabim (and the barbarians)*

**What it does.** The starting soldier: 3 strength, +25% in cities, bronze weapons only. It upgrades to the Axeman or the Archer. With Octopus Overlords as state religion and a Temple of the Overlords in its city it can become a Drown for 60 gold.

**Playing it well.** One per city early for happiness and against animals, escorts for the first settlers, and lair clearers with a little luck. Upgrade them to Axemen or Archers as soon as gold allows rather than building more.

**AI: Valued.** The default early defender and attacker (`production::role_ok`, `best_unit`), the cheap settler escort (`expansion::escort_unit`), and the unit that holds a new city until something better comes (`production::choose` makes a fighter in an undefended city its garrison). Upgraded by power when gold allows (`units::upkeep`); the Drown conversion is never cast (`casting::ffh_value` is negative).

## Archery

### Archer (`UNIT_ARCHER`)
*Archery, tier 2 · 60 hammers · 3/5, 1 move, 1 first strike, +25% city and hills defense · Archery and an Archery Range in the city · every civilization but the Luchuirp, Khazad (Dwarven Slinger), Doviello and Illians (Javelin Thrower)*

**What it does.** The first real city defender: 5 defense against 3 attack, a first strike, +25% in cities and on hills, and a defensive strike (20% chance to deal +10% damage to each unit attacking its stack). It needs no resource, takes the City Garrison line, and upgrades to the Longbowman, the Crossbowman or the Horse Archer.

**Playing it well.** Two per border city as soon as barbarians or neighbours stir, fortified on a hill city where possible; it holds against Warriors, Axemen and Horsemen that attack at 3-4. It is a poor attacker, so pair it with Axemen or Horsemen for anything offensive. Build the Archery Range in a few cities only and move the archers out; upgrade the veterans to Longbowmen rather than training new ones once Bowyers is in.

**AI: Valued.** The AI's standard defender: it passes the Defense role only (`production::role_ok`: `CITY_DEFENSE`/`CITY_COUNTER`), where its city defense power is 687 (5 defense, +25%, the first strike: `production::type_city_defense`) against the Warrior's 375, and `best_unit`'s power squared per turn keeps it ahead of the Longbowman (825 for twice the hammers) in cities making under about 20 hammers. The Archery Range comes through `production::enabler_building` (183% of a Warrior). Research makes Archery the defense goal under pressure (`research::defense_goal`). It garrisons (`units::garrison_unit`), is promoted toward city defense (`units::promotion_value`: City Garrison's 30% x3 for a garrison), and upgrades by city defense power to the Crossbowman, else the Longbowman (`units::upkeep`). The defensive strike is not counted anywhere.

### Arquebus (`UNIT_ARQUEBUS`)
*Archery, tier 4 · 180 hammers · 10/10, 1 move · Blasting Powder and Gunpowder, no building · 13 civilizations (not the Mercurians, Ljosalfar, Hippus, Doviello, Calabim, Svartalfar, Infernal, Clan of Embers)*

**What it does.** A gunpowder soldier at 10 attack and 10 defense, with a defensive strike (30% for +15%) and the one rule that matters: it ignores building defenses (`bIgnoreBuildingDefense`), so walls and castles do not count when it attacks a city. It needs no Archery Range, and the Longbowman and Firebow upgrade into it.

**Playing it well.** The answer to a walled, fortified capital late in the game: bring a stack of them with a few siege units for collateral and they fight the garrison on its base strength and culture alone. Ten defense for 180 hammers also makes it a decent field defender for siege stacks. Secure Gunpowder early; without the resource there is no Arquebus at all.

**AI: Valued.** It passes Defense and Attack/CityAttack (`role_ok`: `ATTACK`, `CITY_DEFENSE`), with 1000 in each (`best_unit`; the city defense power has no bonus to add), which makes it a good war pick per hammer (the Knight's CityAttack power is 1100 for 240 hammers) and, in productive cities, a garrison choice once the four Crossbowmen are in. Its building-defense rule is not in `best_unit`, but the strike sees it: `campaign::fight` asks the rules for the defender's strength against that attacker (`unit_max_combat_str`), so the odds an Arquebus gets against a walled city are its real ones, and `campaign::strike` sends it by those odds. Research values Blasting Powder by it (`research::military_value`), not by the resource it needs.

### Crossbowman (`UNIT_CROSSBOWMAN`)
*Archery, tier 4 · 240 hammers · 9/13, 1 move, 1 first strike, +25% city and hills defense · Machinery and an Archery Range · national unit (4) · every civilization but the Mercurians and the Ljosalfar (Flurry)*

**What it does.** The best ordinary city defender: 13 defense with a first strike, +25% in cities, defensive strikes (40% for +20%), and bronze and iron weapons. Limited to four per player, so it upgrades into and from the archer line rather than being spammed.

**Playing it well.** Put one in each city that matters (capital, wonder city, the border fortress) and promote it along City Garrison; with walls and culture it makes the city a siege project. Upgrade veteran Archers and Longbowmen into the four slots rather than building fresh ones, and back them with Marksmen or assassins against the stack-killers that target weak units.

**AI: Valued.** The strongest pick of the Defense role (city defense power 1787 for 240 hammers against the Longbowman's 825 for 120: `best_unit` takes it whenever the national limit allows, `city_can_train` counting the four), and the first target of a garrison's upgrade (`units::upkeep` picks the upgrade with the most city defense). It also passes Attack (990), but the four slots usually fill as garrisons first (the Defense steps come earlier in `production::decide`). It garrisons and takes City Garrison promotions (`units::promotion_value`). Research does not see it: national units are left out of `research::unit_available`, so Machinery is never valued for its defender.

### Dwarven Slinger (`UNIT_DWARVEN_SLINGER`)
*Archery, tier 2 · 60 hammers · 3/5, 1 move, 1 first strike, +25% city and hills defense · Archery and an Archery Range · Luchuirp and Khazad (replaces the Archer)*

**What it does.** The dwarven Archer: the same numbers, plus the Dwarven race (double moves on hills, 25% poison resistance). It upgrades to the Crossbowman (the dwarves have no Longbowman).

**Playing it well.** As the Archer: the early garrison of dwarven cities, best on hills where it moves freely between hill cities and forts. Since the dwarven line jumps straight from Slinger to Crossbowman, keep the veterans alive and upgrade the best four.

**AI: Valued.** As the Archer (`role_ok` Defense only, power 687; garrison; city-defense promotions). Its Archery Range is the case `production::enabler_building` was written for: by strength alone the Luchuirp built Sculptor's Studios for Wood Golems and never the Range (docs/AI.md, "Unique units"); the rule now compares per hammer as well. It counts as a unique unit for research (`civs::unique_unit_tech_value`: Archery). Its only upgrade is the Crossbowman, so a garrison upgrades once one of the four slots is free.

### Firebow (`UNIT_FIREBOW`)
*Archery, tier 3 · 120 hammers · 5/4, 1 move, 1 first strike, +25% city and hills defense · Bowyers and an Archery Range · Amurites (replaces the Longbowman)*

**What it does.** The Amurite longbowman is weaker than the Longbowman it replaces (4 defense instead of 6) but starts with Channeling II and Fire I. It casts nothing from birth (Fire I's Blaze also needs Channeling I, which it lacks); it becomes a caster once it takes Fire II (it already holds both prerequisites), which gives it Fireball: a 4-strength fire summon with collateral damage, cast every turn. It upgrades to the Crossbowman, Marksman or Arquebus.

**Playing it well.** Promote it to Fire II at its first level-up, then keep it in border cities and stacks: it fireballs whatever stands next to it on your turn and still defends on theirs. With a few of them a stack softens enemy stacks without a mage. Do not count on its defense alone; it is a caster with a bow.

**AI: Incidental.** The AI almost never trains it: it passes only the Defense role, where the Amurite Archer (687 for 60 hammers) always beats it (550 for 120). It comes by the upgrade of Archers when neither a Crossbowman slot nor a Horse Archer is open (`units::upkeep` ranks the upgrades by city defense: 1787, 660, then 550). Once it has one, the use is good: `units::promotion_value` with `magic::promotion_value` rates Fire II at its weight 100 plus the Fireball it opens (about 120, against 90 for City Garrison I), and Fireball is a modelled summon (`magic::understood`) cast at the end of the turn when the Fireball can reach an enemy it beats at 5% or better, or before an attack within two plots of the target (`magic::before_attack`).

### Flurry (`UNIT_FLURRY`)
*Archery, tier 4 · 240 hammers · 12/12, 2 moves, 1 first strike, Blitz · Machinery and an Archery Range · national unit (4) · Ljosalfar (replaces the Crossbowman)*

**What it does.** The elven crossbowman is a strike unit, not a defender: 12 strength, 2 moves, a first strike, defensive strikes (40% for +20%), and Blitz from birth, so it can attack several times a turn. It has no city defense bonus, and it takes Mobility promotions to go further.

**Playing it well.** The clean-up unit: after an Archmage or catapults have wounded a stack, one Flurry with Blitz kills two or three of the survivors in a turn, and its 2 moves let it strike from a forest two plots away and stay safe. Keep it with the stack, promote Mobility and Combat, and do not waste it as a city garrison.

**AI: Partial.** Production sees its strength: it passes Defense and Attack/CityAttack (`role_ok`), with an Attack power of 1650 (strength, the second move, the first strike), among the highest of its era, and it fights as an army unit (`military::army`, `campaign.rs`). Blitz is not used: `campaign::strike` takes each attacker once (`made_attack` filters it out), and letting Blitz units attack again in a strike measured flat (civs/hippus.rs, docs/AI_LOG.md "Hippus Blitz units"). Research never values Machinery for it (a national unit, outside `research::unit_available`); the unique-unit bonus does (`civs::unique_unit_tech_value`).

### Javelin Thrower (`UNIT_JAVELIN_THROWER`)
*Archery, tier 2 · 90 hammers · 4/4, 1 move, 1 first strike, +25% city and hills defense · Archery, no building · Doviello and Illians (replaces the Archer)*

**What it does.** A tougher-hitting, weaker-defending archer that needs no Archery Range: 4 attack and 4 defense, a first strike, +25% in cities and on hills, defensive strikes (20% for +10%). It upgrades to the Longbowman, Crossbowman or Horse Archer.

**Playing it well.** Its point is the missing building: every new city can train its own garrison at once, which suits the Doviello and Illians' fast, building-light expansion. It is no better a defender than an Archer for 50% more hammers, so build an Archery Range later in one or two cities for the Longbowmen and upgrade the throwers there.

**AI: Valued.** The Doviello and Illians' early defender: Defense role only (`role_ok`), power 550, trained in any city since no building gates it. Once Bowyers is in, the Longbowman (825) is a 150% gain and `production::enabler_building` builds the Archery Range for it in up to half the cities. It counts as a unique unit for research (`civs::unique_unit_tech_value`). Garrisons are upgraded by city defense power (`units::upkeep`).

### Longbowman (`UNIT_LONGBOWMAN`)
*Archery, tier 3 · 120 hammers · 5/6, 1 move, 1 first strike, +25% city and hills defense · Bowyers and an Archery Range · every civilization but the Luchuirp, Khazad and Amurites (Firebow)*

**What it does.** The best defender without a national limit: 6 defense, a first strike, +25% in cities and on hills, defensive strikes (30% for +15%), and bronze and iron weapons. It upgrades to the Crossbowman, Marksman (at level 6) or Arquebus.

**Playing it well.** The mid-game garrison: two in every threatened city, City Garrison promotions, iron for the weapon bonus. Upgrading Archers is cheaper than training fresh Longbowmen. It cannot attack well, so a siege still needs melee and mounted units.

**AI: Valued.** Defense role only (`role_ok`), power 825. `best_unit`'s power squared per production turn (+4) prefers the Archer (687 for half the hammers) in cities making under about 20 hammers a turn, so low-production cities keep training Archers after Bowyers; productive ones train Longbowmen. Garrisons upgrade into it when no Crossbowman slot is free (`units::upkeep`), and its Archery Range is the Archer's. Research values Bowyers by its defense, more under military pressure (`research::military_value`).

### Marksman (`UNIT_MARKSMAN`)
*Archery, tier 4 · 240 hammers · 11/11, 1 move, 1 first strike · Precision and an Archery Range · only by upgrade from a level 6 unit · national unit (4) · every civilization but the Mercurians, Luchuirp, Khazad and Doviello*

**What it does.** A veteran sniper: 11 strength, a first strike, defensive strikes (40% for +20%), Can See Stealth, and the Marksman promotion, so when it attacks it fights the weakest unit of the stack (unless a Guardsman protects it). It cannot be built, only upgraded from a level-6 Assassin-class unit or Longbowman-class archer.

**Playing it well.** Keep your best Longbowmen and Assassins alive to level 6 for this upgrade: a Marksman kills the mages, priests and wounded units an enemy hides in its stacks, and it sees the invisible assassins and shadows hunting your own casters. Escort it; it is one unit of four with nothing to replace it.

**AI: Incidental.** It cannot be trained (`iMinLevel` 6; no production role passes, `role_ok` sees only `UNITAI_RESERVE`). It comes by `units::upkeep`: a level-6 garrison upgrades to it when no Crossbowman slot is free (its city defense power 1210 beats the Arquebus's 1000), a level-6 Assassin when the Marksman's strength beats the Shadow's (11 against 10). It fights by its AI type (Garrison or Army); the weakest-unit targeting is not chosen on purpose, but the odds the AI reads come from the rules' defender choice (`plot_get_best_defender` with the attacker), so a strike does send it where those odds are good.

### Nightwatch (`UNIT_NIGHTWATCH`)
*Archery, tier 3 · 120 hammers · 3/3 +2 poison, 1 move, 1 first strike · Bowyers, Council of Esus in the city and as state religion · every civilization*

**What it does.** The Council of Esus's archer: 3 strength plus 2 poison damage, a first strike, defensive strikes (30% for +15%), and Hidden Nationality, so it can attack anyone without a declaration of war and is attacked as a barbarian; Declare Nationality removes the promotion. It never goes obsolete and upgrades to the Assassin or the Shadowrider.

**Playing it well.** A tool for deniable raids: walk it into a rival's land, kill a lone worker or settler escort, and keep the peace treaty. As a garrison it is weaker than an Archer for twice the hammers. Its real value is the upgrade path to the Shadowrider, the strongest hidden-nationality unit; keep a few and level them.

**AI: Incidental.** It passes only the Defense role (`role_ok`: `CITY_DEFENSE`, `CITY_COUNTER`), where its power (550 for 120 hammers) loses to the Archer (687 for 60) and the Longbowman; it is trained only in a productive Esus city without an Archery Range. Research never values it (`research::unit_available` skips state-religion units). Hidden Nationality is thrown away: at the end of a turn the FfH spell choice casts Declare Nationality (`casting::ffh_value`: the ability's AI weight 50 less 1 for the promotion; the rules' AI gate only bars units with the pirate-ship or animal AI, hidden or invisible units and barbarians), and the AI has no logic for deniable attacks.

## Mounted

### Bison Rider (`UNIT_BISON_RIDER`)
*Mounted, tier 4 · 240 hammers · 12/10, 2 moves, 35% withdrawal · Warhorses and a Stable, no resource · national unit (4, the Knight class) · Doviello (replaces the Knight)*

**What it does.** The Doviello knight trades a move for raw strength: 12 attack and 10 defense against the Knight's 11/7 and 3 moves, with the same 35% withdrawal and bronze, iron and mithril weapons. It needs no horses, gets no defensive bonuses, and nothing upgrades out of it.

**Playing it well.** The Doviello's late shock troops: the strongest of the knight class at 2 moves, it breaks Longbowmen in the open and survives counterattacks better than any knight thanks to its 10 defense. Put the four in your main siege stack with a catapult or two; their withdrawal keeps them alive against cities they cannot quite crack.

**AI: Valued.** It passes Defense and Attack/CityAttack (`production::role_ok`: `UNITAI_ATTACK`), with an Attack power of 1762 (strength, the second move, the withdrawal: `production::best_unit`) and 1200 against cities, the Doviello's best field unit. Its Stable (`PrereqBuilding`) comes only through `production::enabler_building` (the Stable has nothing else the generic value reads), so it appears once a city's best attacker is under three quarters of its strength; earlier attempts to build Stables for it never fired because research had not reached Warhorses (civs/doviello.rs, docs/AI_LOG.md). Research values Warhorses for it only as a unique unit (`civs::unique_unit_tech_value`: national units are outside `research::unit_available`). It fights as an army unit (`military::army`, `campaign.rs`), and `campaign::strike` favours a withdrawing unit as the one to risk when odds are poor.

### Boar Rider (`UNIT_BOAR_RIDER`)
*Mounted, tier 2 · 60 hammers · 4/4, 2 moves, 25% withdrawal, +40% against archery units · Horseback Riding, a Stable and Pig in the city · Luchuirp and Khazad (replaces the Horseman)*

**What it does.** The dwarven horseman rides a pig: one move less than the Horseman, no first strike, the same 25% withdrawal and +40% against archers, plus the Dwarven race (double moves on hills, 25% poison resistance). It needs Pig instead of Horse, and upgrades to the Chariot.

**Playing it well.** A mediocre unit, as its own text says. Use it for what 2 moves and withdrawal give you: pillaging, catching workers, finishing wounded units, and hitting archers in the open where its +40% matters. For real attacks the dwarves have better options (Axemen for the Khazad, golems for the Luchuirp).

**AI: Valued.** It passes Defense and Attack/CityAttack (`role_ok`); its Attack power (562: strength, the second move, withdrawal) beats a Warrior's 300 and the Axeman's 400 in peacetime Attack steps, in cities with Pig and a Stable, which `production::enabler_building` builds when it opens a unit a third stronger (4 against a Warrior's 3). In the measurement of that rule, Luchuirp games with a Boar Rider fell from 40% to 10% (docs/AI.md, "Unique units"). The bonus against archers counts only through the rules' odds when it fights (`military::assess`).

### Camel Archer (`UNIT_CAMEL_ARCHER`)
*Mounted, tier 3 · 120 hammers · 6/6, 3 moves, 1 first strike, immune to first strikes, 35% withdrawal · Stirrups, Archery and a Stable, no resource · Malakim (replaces the Horse Archer)*

**What it does.** The Malakim horse archer is a Horse Archer that needs no horses: 6 strength, 3 moves, a first strike, immunity to first strikes, 35% withdrawal and a defensive strike (20% for +10%). No defensive bonuses; it upgrades to the Knight.

**Playing it well.** As the Horse Archer: the fastest useful attacker of the classical era, for picking off units in the open, raiding, and wearing down defenders thanks to its withdrawal. In the desert, where horses are rare, it is the Malakim's mobile army; keep a few as a reaction force between cities.

**AI: Valued.** As the Horse Archer: Defense and Attack/CityAttack (`role_ok`), Attack power 1163 for 120 hammers (strength, two extra moves, the first strike, the withdrawal), the best per hammer of its era, so peacetime army steps and the war's CityAttack steps (660 there) pick it wherever a Stable stands (`production::enabler_building`). Without the horse requirement the Malakim can train it in every such city. Research values Stirrups by it (`research::military_value`) and as a unique unit (`civs::unique_unit_tech_value`). It fights as an army unit (`military::army`, `campaign.rs`).

### Centaur (`UNIT_CENTAUR`)
*Mounted, tier 2 · 60 hammers · 4/4, 3 moves, 25% withdrawal, +40% against archery units · Horseback Riding, no building or resource · Kuriotates (replaces the Horseman)*

**What it does.** The Kuriotate horseman without the Horseman's needs: no Stable, no horses, and it keeps terrain and city defense bonuses that other mounted units lose. It has the Centaur race and Sprint (+1 move for the turn, not while Fatigued). No first strike, unlike the Horseman. It upgrades to the Centaur Archer or Centaur Charger; no other unit can upgrade into it.

**Playing it well.** Every Kuriotate city can train it at once, so it is the early answer to barbarians across a spread-out empire: a mobile defender that fortifies on a hill or in a settlement, and a raider with 3 moves (4 with Sprint) that catches workers and settlers. Use withdrawal against strong defenders and the +40% against archers.

**AI: Partial.** Production trains it for its strength: Defense and Attack/CityAttack (`role_ok`), Attack power 675 for 60 hammers, the Kuriotates' best early attacker since it needs no building, and settlements' defenders come from full cities (`civs::kuriotates`, `settlement_defender`, through the Defense role, where Archers usually win). It fights as an army unit. Sprint is never cast: the spell is `bAllowAI` 0. The defensive bonuses it keeps are not part of `production::type_city_defense`, nor of anything in the field logic.

### Centaur Archer (`UNIT_CENTAUR_ARCHER`)
*Mounted, tier 3 · 120 hammers · 6/6, 3 moves, 1 first strike, immune to first strikes, 35% withdrawal · Stirrups and Archery, no building or resource · Kuriotates (replaces the Horse Archer)*

**What it does.** A Horse Archer without the Stable and horses, which keeps defensive bonuses and has Sprint: 6 strength, 3 moves, first strike, first-strike immunity, 35% withdrawal, a defensive strike (20% for +10%). It upgrades to the Knight class (the Centaur Lancer).

**Playing it well.** The core of a Kuriotate army and a fine mobile defender: fortified in a settlement it defends with its first strike and terrain, and it can ride out to kill raiders in the same turn. Sprint gives the reach to strike from four plots away.

**AI: Partial.** Its strength is valued: Defense and Attack/CityAttack (`role_ok`), Attack power 1163 for 120 hammers, which wins most army steps from Stirrups on (the Kuriotates fielded 9.7 to 11.2 a game in the measurement of docs/AI.md, "Foreign units through `UpgradeCiv`"). Sprint is `bAllowAI` 0, so it is never cast, and the AI does not use the defensive bonuses on purpose.

### Centaur Charger (`UNIT_CENTAUR_CHARGER`)
*Mounted, tier 2 · 120 hammers · 5/3, 3 moves, 25% withdrawal · Trade and Construction, no building or resource · Kuriotates (replaces the Chariot)*

**What it does.** The Kuriotate chariot: 5 attack, 3 defense, 3 moves, 25% withdrawal, Sprint and weapon promotions (bronze to mithril), without the Chariot's Siege Workshop and horses. No defensive bonuses. It upgrades to the Centaur Lancer.

**Playing it well.** A hitter with withdrawal for the gap between Centaurs and Centaur Archers: attack strong defenders, retreat, let the next unit finish. It dies to any counterattack, so escort it with Centaur Archers as the strategy text says, and stop building it once Stirrups is in.

**AI: Partial.** Defense and Attack/CityAttack (`role_ok`); Attack power 843 for 120 hammers, which competes with the Centaur's 675 for 60 until Stirrups and then loses to the Centaur Archer (1163 for the same price), so it is trained in a short window. Sprint is `bAllowAI` 0. It fights as an army unit; the field logic does not keep its 3 defense out of reach of counterattacks.

### Centaur Lancer (`UNIT_CENTAUR_LANCER`)
*Mounted, tier 4 · 240 hammers · 11/7, 3 moves, 35% withdrawal · Warhorses, no building or resource · national unit (4, the Knight class) · Kuriotates (replaces the Knight)*

**What it does.** The Kuriotate knight: the Knight's 11/7, 3 moves and 35% withdrawal, without Stable or horses, with Sprint and the Centaur race (which keeps defensive bonuses).

**Playing it well.** As the Knight: the best general-purpose unit of the late medieval era. Four of them with Sprint reach and hit any target within four plots, and they defend themselves better than knights since they keep terrain bonuses.

**AI: Partial.** Its strength is valued: Attack power 1938 for 240 hammers (`best_unit`), among the best of the Kuriotates, trained in any city since nothing gates it, and research gets to Warhorses for it as a unique unit (`civs::unique_unit_tech_value`), not as a military unit (national units are outside `research::unit_available`). It fights as an army unit. Sprint is `bAllowAI` 0.

### Chariot (`UNIT_CHARIOT`)
*Mounted, tier 2 · 120 hammers · 5/3, 3 moves, 25% withdrawal, -25% attacking into jungle or forest · Trade, Construction, a Siege Workshop and Horse or Nightmare · every civilization but the Kuriotates, Ljosalfar and Svartalfar*

**What it does.** A strong attacker with weak defense: 5 attack, 3 defense, 3 moves, 25% withdrawal and the full weapon range, but no defensive bonuses and a penalty in woods. It needs both a Siege Workshop and horses, and upgrades to the Knight.

**Playing it well.** It softens strong defenders between the Horseman and the Knight: attack, withdraw, let the axes finish. Since it dies to any counter, end its turn in a city or under a strong stack. The Siege Workshop doubles as the catapult building, so build it where you build siege engines.

**AI: Incidental.** It passes Defense and Attack/CityAttack (`role_ok`), but its Attack power (843 for 120 hammers) loses to the Horseman (742 for 60) and then to the Horse Archer (1163 for 120), which need only a Stable. So it is trained only in a city that has a Siege Workshop (from `campaign::siege_building` or `production::enabler_building`) and horses but no Stable; where both buildings stand, the Horseman is the better buy until the Chariot and Horse Archer make it obsolete (`city_all_upgrades_available`), and then the Horse Archer wins. As an upgrade it is taken only when the stronger Champion or Horse Archer is not available (`units::upkeep`). It fights as an army unit.

### Death Knight (`UNIT_DEATH_KNIGHT`)
*Mounted, tier 4 · 240 hammers · 8/4 +2 death +2 unholy, 3 moves, 35% withdrawal · Warhorses, a Stable and Horse or Nightmare · national unit (4, the Knight class) · Infernal (replaces the Knight)*

**What it does.** The Infernal knight: 8 attack plus 2 death and 2 unholy damage (which demons and the undead resist), 3 moves, 35% withdrawal, and free Channeling I and II, Death I and the Demon race. It casts Raise Skeleton (a permanent Skeleton, 2 strength +1 death, every turn) and is one promotion from Death II's Summon Spectre.

**Playing it well.** A caster that rides with the army: raise a skeleton each turn on the march and throw the skeletons at a city first to soak its first strikes and defensive strikes; take Death II for spectres. Its 4 defense makes it fragile in the open, so keep it in the stack, and remember that undead and demons resist its death and unholy damage.

**AI: Valued.** The strongest Attack-role power of its class (2115: 12 strength with the damage types, the moves, the withdrawal), so the Infernals train their four wherever a Stable stands (`production::enabler_building`), and research gets to Warhorses for it as a unique unit (`civs::unique_unit_tech_value`). In the field it is an army unit that also casts: Raise Skeleton is a modelled lasting summon (`magic::summon_value`: worth its strength x10 while at war or in danger), cast at the end of every such turn, and the skeletons are spent first in a strike (`magic::raised`, `campaign::strike`). Its promotions value Death II by Summon Spectre (`magic::promotion_value`).

### Dwarven Hornguard (`UNIT_HORNGUARD`)
*Mounted, tier 4 · 240 hammers · 12/8, 2 moves, 35% withdrawal · Warhorses and a Stable, no resource · national unit (4, the Knight class) · Luchuirp and Khazad (replaces the Knight)*

**What it does.** The dwarven knight on a rhinoceros: slower than the Knight (2 moves) but stronger (12/8), with 35% withdrawal, mithril weapons and the Dwarven race (double moves on hills). It needs no horses.

**Playing it well.** The dwarves' best attacker until their late units: hills make up for the lost move, and 12 strength with withdrawal cracks Longbowmen. Four of them lead the siege stack behind Dwarven Cannons or Trebuchets.

**AI: Valued.** Attack power 1762 for 240 hammers (`best_unit`), the Khazad's and Luchuirp's best field attacker. Its Stable (`PrereqBuilding`) is built by `production::enabler_building` (`production::unit_building` reads `PrereqBuilding`, and the comment there names the Hornguard), and research reaches Warhorses for it as a unique unit (`civs::unique_unit_tech_value`, which counts the Stable's techs too). Before both, no Hornguard was ever trained: a Khazad-only Stable rule got none because research never valued Warhorses (civs/khazad.rs, docs/AI_LOG.md "Siege Workshop and Stable for the Khazad"). It fights as an army unit (`military::army`, `campaign.rs`).

### Fyrdwell (`UNIT_FYRDWELL`)
*Mounted, tier 3 · 120 hammers · 6/6, 3 moves, 1 first strike, immune to first strikes, 35% withdrawal · Stirrups, Archery, a Stable and Deer in the city · Ljosalfar (replaces the Horse Archer)*

**What it does.** The elven horse archer rides deer instead of horses; otherwise it is the Horse Archer (6 strength, 3 moves, first strike, first-strike immunity, 35% withdrawal, a defensive strike), with the Elven race (+10% attack in forest, double moves in forest, -25% cold resistance). It upgrades to the Knight.

**Playing it well.** As the Horse Archer, in forests: elven lands are wooded, and double moves there make the Fyrdwell the fastest unit on its own ground. Secure Deer early; without it the Ljosalfar have no horse archer at all.

**AI: Valued.** As the Horse Archer (`role_ok` Defense and Attack/CityAttack, Attack power 1163), in the cities that have Deer and a Stable (`production::enabler_building` checks the resource before building the Stable). Research values Stirrups by it (`research::military_value`; resources are not checked there) and as a unique unit (`civs::unique_unit_tech_value`). It fights as an army unit; forest movement is used only as far as the rules' path finder uses it.

### Horse Archer (`UNIT_HORSE_ARCHER`)
*Mounted, tier 3 · 120 hammers · 6/6, 3 moves, 1 first strike, immune to first strikes, 35% withdrawal · Stirrups, Archery, a Stable and Horse or Nightmare · 14 civilizations (the Malakim, Kuriotates, Ljosalfar, Svartalfar have their own; not the Luchuirp, Khazad, Clan of Embers)*

**What it does.** The classical-era mobile attacker: 6 strength, 3 moves, a first strike, immunity to first strikes, 35% withdrawal and a defensive strike (20% for +10%). No defensive bonuses. Horsemen and Archers upgrade into it, and it upgrades to the Knight.

**Playing it well.** The flexible core of a classical army: picks off units in the open, raids improvements, catches invaders anywhere in your land in a turn, and wears down a city's defenders with its withdrawal and first strike. Pair it with Axemen or Champions to hold the plot it ends on, since it defends without terrain bonuses.

**AI: Valued.** The best Attack-role pick per hammer of its era: power 1163 for 120 hammers (`best_unit` counts its two extra moves, the first strike and the withdrawal), and even in war's CityAttack role (660) it edges out the Champion (600 for the same price) by its first strike. The Stable comes through `production::enabler_building`. It fights as an army unit (`military::army`, `campaign.rs`); `campaign::strike` prefers a withdrawing unit as the one to risk when no attacker has good odds. Where a city has a Stable but no Archery Range, the Defense role can make it a garrison (660), and `production::type_city_defense` does not know it gets no defensive bonuses there.

### Horseman (`UNIT_HORSEMAN`)
*Mounted, tier 2 · 60 hammers · 4/4, 3 moves, 1 first strike, 25% withdrawal, +40% against archery units · Horseback Riding, a Stable and Horse or Nightmare · every civilization but the Luchuirp, Khazad (Boar Rider), Kuriotates (Centaur) and Clan of Embers (Wolf Rider)*

**What it does.** The first mounted unit: 4 strength, 3 moves, a first strike, 25% withdrawal and +40% against archers, but no defensive bonuses. Scouts and Goblins upgrade into it; it upgrades to the Horse Archer or the Chariot.

**Playing it well.** Early mobility: kill barbarian archers in the open, escort settlers, pillage and grab workers in a war, and react to invaders anywhere in the empire. It is a poor city attacker against fortified Archers, its +40% notwithstanding, so leave sieges to Axemen and catapults; upgrade to Horse Archers at Stirrups.

**AI: Valued.** Attack power 742 for 60 hammers (`best_unit`: the moves, first strike and withdrawal) makes it the AI's favourite peacetime army unit before Stirrups: it outscores the Axeman (400) in every Attack step, and the Stable is built for it once it is a third stronger than the city's best attacker (4 against a Warrior's 3, `production::enabler_building`). In war's CityAttack role it ties the Axeman (440). It fights as an army unit, hunting barbarians and animals near our land (`military::hunt`), and upgrades to the Horse Archer by attack power (`units::upkeep`).

### Knight (`UNIT_KNIGHT`)
*Mounted, tier 4 · 240 hammers · 11/7, 3 moves, 35% withdrawal · Warhorses, a Stable and Horse or Nightmare · national unit (4) · 13 civilizations (the Doviello, Kuriotates, Luchuirp, Khazad, Infernal and Lanun have their own; the Mercurians and the Clan of Embers none)*

**What it does.** The medieval all-rounder: 11 attack, 7 defense, 3 moves, 35% withdrawal and the full weapon range, no defensive bonuses, four per player. Champions, chariots, horse archers and many others upgrade into it.

**Playing it well.** Use the four for what nothing else does: strike a stack or a weakened city from three plots away and get back, kill the siege engines an enemy brings, pillage the key resource. Upgrade veteran Horse Archers rather than training new ones.

**AI: Valued.** Attack power 1938 for 240 hammers (`best_unit`), among the best of most civilizations' rosters, so the four slots fill quickly once a Stable stands (`production::enabler_building` reads its `PrereqBuilding`), and Horse Archers upgrade into it by attack power (`units::upkeep`). It fights as an army unit (`military::army`, `campaign.rs`). Research never values Warhorses for it: national units are left out of `research::unit_available`, so the tech comes only for its other merits.

### Nyxkin (`UNIT_NYXKIN`)
*Mounted, tier 3 · 120 hammers · 6/6, 3 moves, 1 first strike, immune to first strikes, 35% withdrawal · Stirrups, Archery and a Stable, no resource · Svartalfar (replaces the Horse Archer)*

**What it does.** The Svartalfar horse archer: the Horse Archer's numbers without the horse requirement, with the Elven race (+10% attack and double moves in forest). It upgrades to the Knight.

**Playing it well.** As the Horse Archer, and since it needs no resource every Svartalfar city with a Stable can train it: the dark elves' raiders, fast in their forests, for picking off units and workers while their assassins and illusionists work.

**AI: Valued.** As the Horse Archer: Defense and Attack/CityAttack (`role_ok`), Attack power 1163 for 120 hammers, the Stable from `production::enabler_building`, research for Stirrups by `research::military_value` and as a unique unit (`civs::unique_unit_tech_value`). It fights as an army unit.

### Ratha (`UNIT_RATHA`)
*Mounted, tier 2 · 180 hammers · 5/3, 3 moves, 25% withdrawal, +50% against recon units · Trade, Construction, the Empyrean in the city and as state religion, Horse or Nightmare · every civilization*

**What it does.** The Empyrean war chariot: Chariot numbers (5/3, 3 moves, 25% withdrawal, the full weapon range) without the Siege Workshop, plus Can See Stealth, +50% against recon units, Sun II and the Blinding Light spell (every enemy unit within one plot is held in place for 2 turns unless it resists).

**Playing it well.** An Empyrean player's answer to invisible assassins and shadows: it sees them and hits recon units at +50%. Blinding Light freezes an enemy stack next to it for two turns, so your army can gather and strike before it moves or retreats; cast it on the stack you mean to kill, or on raiders heading for your workers.

**AI: Partial.** Production trains it only where it wins on strength: its Attack power (843 for 180 hammers) loses to the Horseman and Horse Archer wherever a Stable stands, so it is trained in Empyrean cities without one. Research never values it (`research::unit_available` skips religious units). Blinding Light is modelled by `magic.rs` (immobility worth 25 hit points a turn, up to two, for each enemy attacker next to it, times its chance not to resist; `magic::reached_value`), so it is cast at the end of a turn in which enemies that can attack stand next to it. Its sight of hidden units helps the stack it rides with, but nothing sends it hunting assassins.

### Royal Guard (`UNIT_ROYAL_GUARD`)
*Mounted, tier 3 · 180 hammers · 4/8, 3 moves · Feudalism, the Aristocracy civic, Horse or Nightmare · national unit (4) · every civilization · leaves if Aristocracy is dropped*

**What it does.** A mounted bodyguard: 4 attack, 8 defense, 3 moves, and it keeps defensive bonuses. It starts with Guardsman (attackers that target the weakest unit, Marksmen, cannot do so in its stack) and Spirit II, which gives the Hope spell: in a city, a Hope building (+1 happiness, +4 culture, Courage for the units there) that lasts while a caster stays. It abandons a civilization that drops Aristocracy.

**Playing it well.** Escort your most valuable stack (mages, heroes) against Marksmen and assassins, or park one in a growing city to cast Hope for happiness and culture. Do not build them if you plan to leave Aristocracy.

**AI: Incidental.** It passes Defense and Attack (`role_ok`: its `UnitAIs` hold `UNITAI_ATTACK`), but its Defense power (800 for 180 hammers) loses to the Archer and Longbowman per hammer and its Attack power (600) to every horse unit, so it is trained only as a garrison where no Archery Range stands. As a garrison (its default AI) it casts Hope at the end of the turn in a city that lacks the building (`magic::turn_value`: a quarter of the building's value), and it stays as long as the garrison holds. Guardsman's protection is not valued, and the civic choice (`empire::civic_value`) does not count the guards that would leave.

### Shadowrider (`UNIT_SHADOWRIDER`)
*Mounted, tier 4 · 240 hammers · 9/9 +2 poison, 3 moves, 35% withdrawal, immune to defensive strikes · Warhorses, the Council of Esus in the city and as state religion, Horse or Nightmare · national unit (4) · every civilization · leaves if Esus is dropped*

**What it does.** The strongest hidden-nationality land unit: 9 strength plus 2 poison, 3 moves, 35% withdrawal, immunity to defensive strikes, the full weapon range, and +1 strength for each Shadow mana its owner has (affinity). With Hidden Nationality it can attack anyone without war; Declare Nationality ends that. Nightwatch upgrade into it.

**Playing it well.** Use the four for deniable war: kill a rival's settlers, workers and stray units while at peace, or bleed an enemy's ally without a declaration. Stack Shadow mana for its affinity. Keep it hidden; once declared it is just a strong knight.

**AI: Partial.** Production sees a knight: Attack power 1938 for 240 hammers (`best_unit`), trained in Esus cities with horses as an army unit, outside the Knight's four slots. Research never values it (a religious and national unit, outside `research::unit_available`). Hidden Nationality is discarded: at the end of a turn the FfH spell choice casts Declare Nationality (`casting::ffh_value`, the ability's weight 50 less 1), and the AI has no use for deniable attacks; its affinity counts only in the strength the rules give it. The civic and religion logic does not count the units that would leave.

### War Elephant (`UNIT_WAR_ELEPHANT`)
*Mounted, tier 3 · 90 hammers · 7/7, 2 moves, 25% withdrawal, +50% against mounted units · Horseback Riding · only by upgrade from an Elephant of level 1+ · every civilization*

**What it does.** A tamed war beast: 7 strength, 2 moves, 25% withdrawal, +50% against mounted units, no defensive bonuses. It cannot be built: an Elephant (a wild animal) captured by a recon unit's Subdue Animal or a Satyr's Mesmerize Animal upgrades into it.

**Playing it well.** When your hunters capture an Elephant, upgrade it: 7 strength in the early classical era beats anything but fortified Archers, and +50% against mounted units makes it the counter to Horsemen and Horse Archers. Use it as the hammer of an early stack.

**AI: Incidental.** The AI cannot train it (minimum level 1; `city_can_train`). It appears only when an AI recon unit with Subdue Animal (taken eagerly: its AI weight 300, `units::promotion_value`) captures an Elephant and `units::upkeep` upgrades the Elephant when gold allows. The captured animal keeps its `UNITAI_ANIMAL` type (captures are created with the default AI, upgrades keep the AI type), so the War Elephant moves as an animal (`units::barbarian`: attacks what it beats at 50%, else hunts or wanders), never in the army.

### War Tortoise (`UNIT_WAR_TORTOISE`)
*Mounted, tier 4 · 240 hammers · 11/19, 1 move, +50% against mounted units · Warhorses and a Siege Workshop · national unit (4, the Knight class) · Lanun (replaces the Knight)*

**What it does.** The Lanun knight is a walking fortress: 11 attack, 19 defense, 1 move, +50% against mounted units, Water Walking (it crosses water), and no defensive bonuses. It needs a Siege Workshop and no horses.

**Playing it well.** A stack defender for amphibious wars: 19 defense holds a beachhead against counterattacks, and Water Walking lets it cross straits without a ship, escorting the attackers it protects. In a city it is weaker than its number says, because it gets no fortification or wall bonuses.

**AI: Partial.** Production values it highest as a defender: its city defense power (1900, from 19 defense) beats the Crossbowman's 1787 (`production::type_city_defense`), so a Lanun city with a Siege Workshop garrisons it, although it gets none of the city's defensive bonuses there. The Siege Workshop comes from `production::enabler_building` (a 230% defender against a Longbowman) or `campaign::siege_building`; with the enabling-building rule War Tortoises appeared in 25% of Lanun games (0% before, docs/AI.md "Unique units"; an earlier Lanun-only rule had made 0.3 a game, civs/lanun.rs). Water Walking is used only as far as the rules' path finder uses it; no amphibious plan counts on it.

### Wolf Rider (`UNIT_WOLF_RIDER`)
*Mounted, tier 2 · 60 hammers · 4/4, 3 moves, 10% withdrawal, +40% against archery units · Horseback Riding, no building or resource · Clan of Embers and the barbarians (replaces the Horseman)*

**What it does.** The orc horseman rides a wolf: 4 strength, 3 moves, +40% against archers, the Orcish race (+10% attack in jungle, 25% fire resistance), but only 10% withdrawal and no first strike. It needs no Stable (the Clan has none). A Goblin that kills a Wolf or Wolf Pack turns into one. It upgrades to the Chariot.

**Playing it well.** The Clan's fast raiders: steal workers, pillage, and take cities left with weak defenders. Its low withdrawal makes it a poor siege softener, so use it for speed rather than strength, and promote Goblins into it by sending them against wolves early.

**AI: Valued.** Defense and Attack/CityAttack (`role_ok`), Attack power 630 for 60 hammers, which wins the Clan's peacetime Attack steps over its Axemen in any city since nothing gates it; once other civilizations' Centaurs were no longer trainable, the Clan fielded 5.9 Wolf Riders a game instead of 0 (docs/AI.md, "Foreign units through `UpgradeCiv`"). It fights as an army unit. The Goblin conversion happens only by chance, for instance when an AI Goblin explorer attacks a wolf it beats (`units::explore` attacks at 75% odds).

## Recon

### Assassin (`UNIT_ASSASSIN`)
*Recon, tier 3 · 120 hammers · 5/3 +1 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, -50% city attack and defense · Poisons and a Hunting Lodge · every civilization but the Elohim, Sidar, Amurites, Balseraphs and Clan of Embers (who have their own)*

**What it does.** A stack-sniper: it starts with Marksman, so when it attacks it fights the weakest unit of the defending stack (unless a Guardsman is there), with a first strike, first-strike immunity, immunity to defensive strikes and 2 moves. Half strength against and inside cities. Illian assassins also have Whiteout (on snow: the assassin becomes invisible, +1 first strike, until it fights or casts). It upgrades to the Shadow or, at level 6, the Marksman.

**Playing it well.** Kill the mages, priests, wounded units and siege engines hiding in a stack, before your main army attacks it; it is wasted on a lone strong defender and useless against cities. Keep it moving with the army, out of reach of counterattacks (3 defense), and level it toward the Marksman upgrade.

**AI: Partial.** Production can train it only in Attack steps (`production::role_ok`: its default AI `UNITAI_EXPLORE` keeps it out of the Defense role, and its `UnitAIs` lack `EXPLORE`), with a power of 825 (6 strength with the poison, the second move, the first strike) that beats a Champion's 600 for the same price but loses to the Horse Archer, and only 330 in war's CityAttack role. The Hunting Lodge comes through `production::enabler_building` when it is a third stronger than the city's best attacker. It is an army unit (production trains it `UNITAI_ATTACK`); its weakest-unit targeting is never sought, though the odds the AI reads come from the rules' defender choice against that attacker (`plot_get_best_defender`), so `campaign::strike` sends it when its odds are best. An Illian assassin casts Whiteout at the end of a turn on snow (`magic.rs` values the promotion on the caster, plus the spell's AI weight 50).

### Assassin (`UNIT_LIZARDMAN_ASSASSIN`)
*Recon, tier 3 · 120 hammers · 5/3 +1 poison, 2 moves, 1 first strike, -50% city attack and defense · Poisons and a Hunting Lodge · Clan of Embers and the barbarians (replaces the Assassin)*

**What it does.** The Clan's assassin: the Assassin (`UNIT_ASSASSIN`) with the Lizardman race (amphibious attacks, double moves and +10% in marsh, 10% poison resistance).

**Playing it well.** As the Assassin; the Clan's armies of orcs and goblins carry few casters to protect, so use it against enemy stacks with adepts and wounded units, and through marshes.

**AI: Partial.** As the Assassin (`UNIT_ASSASSIN`): Attack steps only (power 825), Hunting Lodge from `production::enabler_building`, an army unit whose sniping is only as good as the rules' odds make it. Against the Clan's Wolf Riders (630 for 60 hammers) it wins Attack steps only in productive cities.

### Beastmaster (`UNIT_BEASTMASTER`)
*Recon, tier 4 · 240 hammers · 14/14, 2 moves, -20% city attack, +50% against animals, carries one bird · Animal Mastery and a Grove · national unit (4) · every civilization but the Mercurians, Luchuirp, Khazad (Myconid) and Clan of Embers (its own)*

**What it does.** The top of the recon line: 14 strength, 2 moves, +50% against animals, better results from tribal villages, sees hidden animals, and room for a Hawk. With the recon promotions Subdue Animal (after Combat I, Animal Husbandry) and Subdue Beasts (after Combat III, Animal Mastery) it captures the animals and beasts it defeats. Rangers and Harlequins upgrade into it.

**Playing it well.** A field monster for its era: clear the strongest animals and barbarian beasts, take Subdue Animal and Subdue Beasts, and turn the captures (bears, tigers, griffons, elephants) into an army. It fights well in the open but poorly against cities, so let it guard and hunt while melee units take cities. The Grove costs 300 hammers for nothing else; build one, in a productive city.

**AI: Partial.** Its strength is valued: Attack steps only (`role_ok`, default AI `UNITAI_EXPLORE`), with a power of 1750 (only the knight class and a few other unique units score more), and the Grove, which has no other effect the generic value reads, comes only through `production::enabler_building` (a 300-hammer build: only a city that finishes it within 20 turns). It is an army unit hunting near our lands (`military::hunt`). Subdue Animal and Subdue Beasts are taken at once (their AI weight 300 in `units::promotion_value`), but what it captures keeps the animal AI (captures are created with the default AI), so the animals roam and attack as animals (`units::barbarian`) instead of joining the army. The AI never trains Hawks (no production role), so the cargo slot stays empty.

### Beastmaster (`UNIT_LIZARDMAN_BEASTMASTER`)
*Recon, tier 4 · 240 hammers · 14/14, 2 moves, -20% city attack, +50% against animals, carries one bird · Animal Mastery and a Grove · national unit (4) · Clan of Embers and the barbarians (replaces the Beastmaster)*

**What it does.** The Clan's Beastmaster (`UNIT_BEASTMASTER`) with the Lizardman race (amphibious attacks, marsh mobility and +10% there, 10% poison resistance).

**Playing it well.** As the Beastmaster: the Clan's strongest early-medieval field unit, and its captured animals and beasts add to the horde.

**AI: Partial.** As the Beastmaster (`UNIT_BEASTMASTER`): trained in Attack steps (power 1750) once `production::enabler_building` builds a Grove, an army unit, Subdue promotions taken for their weight, captures left to the animal AI, no Hawks.

### Chanter (`UNIT_CHANTER`)
*Recon, tier 3 · 120 hammers · 5/3 +1 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, -50% city attack and defense · Poisons and a Hunting Lodge · Amurites (replaces the Assassin)*

**What it does.** The Amurite assassin: the Assassin's numbers and Marksman, plus Escape, which teleports it to its capital (the rules keep an AI from casting it at 50 or more damage).

**Playing it well.** Strike deep, then Escape home before the counterattack: one Chanter can snipe a mage out of an enemy stack every few turns and survive. Use Escape also to bring a wounded veteran home to heal and upgrade.

**AI: Partial.** As the Assassin (`UNIT_ASSASSIN`): Attack steps only (power 825), an army unit, sniping only through the rules' odds. Escape is never cast: it is a Python-only spell `magic.rs` does not model, and FfH's terms give it nothing (`casting::ffh_value`: no term applies and its AI weight is 0).

### Courtesan (`UNIT_COURTESAN`)
*Recon, tier 4 · 240 hammers · 8/7 +2 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, -25% city attack and defense · Guilds and the Council of Esus in the city · national unit (4, the Shadow class) · Balseraphs (replaces the Shadow)*

**What it does.** The Balseraph shadow: invisible to most units, Marksman (fights the weakest unit of a stack), investigates rival cities, explores rival territory, and 25% of its victories over non-animals produce a slave. As a Council of Esus unit it can Steal equipment where it stands while Esus is its owner's state religion (each item a 20% chance of war).

**Playing it well.** The perfect counter to a stack led by High Priests or Archmages: walk in unseen, kill the caster, take a slave. Its invisibility makes it a scout of enemy cities and armies as well. Keep the four alive; they are irreplaceable at the national limit.

**AI: Partial.** Attack steps only (`role_ok`; power 1375), which it wins against most units but the Knight class and the Beastmaster, in Esus cities; research does not see it unless Esus is the state religion (`civs::unique_unit_tech_value` counts it only then, `research::unit_available` skips every religious unit). It is an army unit; invisibility and Marksman work through the rules, not by choice, and the slaves it takes join the workers (their default AI). Investigating cities is never done (no espionage missions). Steal (AI weight 50) is cast whenever its rare condition holds, and its 20% chance of war is not weighed (`casting::ffh_value`).

### Devout (`UNIT_DEVOUT`)
*Recon, tier 3 · 120 hammers · 5/5, 2 moves, immune to defensive strikes, -50% city attack and defense · Poisons and a Hunting Lodge · Elohim (replaces the Assassin)*

**What it does.** The Elohim's assassin is a pilgrim: 5/5 without the poison, first strike or Marksman, but with Channeling I and Life I and the Sanctify spell, and it explores rival territory without open borders. Sanctify removes City Ruins or a Graveyard on its plot (each -1 to the Armageddon counter; a Graveyard also gives an Einherjar) and clears the hell plot counters within one plot. It upgrades to the Shadow or one of four priests.

**Playing it well.** The Elohim's tool against the Armageddon counter: send Devouts to the ruins of razed cities and graveyards, even in foreign lands, and sanctify them; clean hell terrain around your cities. As a fighter it is an ordinary 5/5; keep it for the spell.

**AI: Partial.** It is trained only as an attacker (`role_ok`: Attack steps, power 625 for 120 hammers, a weak pick against the Horse Archer or Champion), and Sanctify is never cast: the spell lives in Python, `magic.rs` does not model it, and FfH's terms value it at 0 (`casting::ffh_value`). Nothing in the AI tracks the Armageddon counter's ruins. Its upgrade is chosen by attack power (`units::upkeep`), not by the priests' spells.

### Divided Soul (`UNIT_DIVIDED_SOUL`)
*Recon, tier 2 · 60 hammers · 4/4, 2 moves, -20% city attack, +50% against animals, carries one bird · Hunting and a Hunting Lodge · Sidar (replaces the Hunter)*

**What it does.** The Sidar hunter: the Hunter's numbers, plus Sever Soul, which creates a Severed Soul (strength 0, flying, invisible to everyone, defends only) that can call the Divided Soul's body to it (Call Form).

**Playing it well.** A two-stage scout: send the Severed Soul ahead to reveal the land safely, then call the body over when it has found a goody hut or a good site. Otherwise use it as a Hunter.

**AI: Partial.** As the Hunter: trained as the explorer (`role_ok` Explore, power 220) or in Attack steps (500), explores one safe step a turn (`units::explore`) and later joins the army. Sever Soul is never cast: the Severed Soul has no strength, so `magic::summon_value` finds no fight for it and FfH's summon term is 0, and Call Form is `bAllowAI` 0.

### Dwarven Shadow (`UNIT_DWARVEN_SHADOW`)
*Recon, tier 4 · 240 hammers · 8/7 +2 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, collateral damage 40 (to 50%), -25% city attack and defense · Guilds and the Council of Esus in the city · national unit (4, the Shadow class) · Luchuirp and Khazad (replaces the Shadow)*

**What it does.** The dwarven shadow adds collateral damage to the Shadow: invisible, Marksman, investigates cities, explores rival territory, and each attack also damages other units of the stack, with the Dwarven race. Under an Esus state religion it can Steal equipment (20% chance of war per item).

**Playing it well.** An invisible stack-softener: it kills the weakest unit and wounds the rest in one attack, so send it in first against a big stack. Like every shadow, keep it alive and away from units that see invisible.

**AI: Partial.** Attack steps only (`role_ok`), power 1415 (the Shadow's plus its collateral), with collateral counted again in the strike, where `campaign::strike` sends collateral attackers first while several defenders stand. It needs the Council of Esus in the city: in the measurement of docs/AI.md ("Unique units") no dwarven game had one (20% before), the religion being the reason given there. Invisibility and Marksman work only through the rules; Steal is cast whenever possible without weighing its chance of war.

### Fawn (`UNIT_FAWN`)
*Recon, tier 2 · 90 hammers · 4/4, 2 moves, Woodsman I · Way of the Forests, the Fellowship of Leaves in the city and as state religion · every civilization*

**What it does.** The Fellowship's early soldier: 4/4 with 2 moves and Woodsman I (+30% attack and defense in forest and jungle), no building needed. It upgrades to the Satyr at level 4.

**Playing it well.** For a Leaves empire before tier-2 units: a forest defender for border cities on wooded ground and a 2-move raider in the woods. Level them toward the Satyr upgrade.

**AI: Valued.** It passes Defense and Attack/CityAttack (`role_ok`: `UNITAI_ATTACK`, `UNITAI_COUNTER`), with 400 as a defender and 500 as an attacker (strength and the second move) for 90 hammers, so it is trained where no Archery Range or better attacker is available; research never values it (`research::unit_available` skips religious units). Woodsman counts only through the rules' odds when it fights. Fawns that reach level 4 become Satyrs by `units::upkeep`.

### Ghost (`UNIT_GHOST`)
*Recon, tier 3 · 120 hammers · 5/3 +1 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, -50% city attack and defense · Poisons and a Hunting Lodge · Sidar (replaces the Assassin)*

**What it does.** The Sidar assassin starts Hidden (invisible, +1 first strike, lost when it fights or casts) with Marksman and Stealth, and Hide makes it invisible again in any turn it has not attacked. It keeps Hide when it upgrades to the Marksman.

**Playing it well.** Approach unseen, strike the weakest unit of a stack, then Hide again next turn; Ghosts are the best assassins for surviving the retaliation. Hide also turns it into a perfect scout.

**AI: Partial.** As the Assassin (`UNIT_ASSASSIN`): Attack steps only (power 825), an army unit, sniping only through the rules' odds. It is invisible until its first fight, and then for good visible: Hide is `bAllowAI` 0.

### Goblin (`UNIT_GOBLIN`)
*Recon, tier 1 · 15 hammers · 2/2, 1 move, +50% against animals · no tech or building · Clan of Embers and the barbarians (replaces the Scout)*

**What it does.** The Clan's scout: 2 strength, only 1 move, +50% against animals, the Orcish race, better results from tribal villages, and it sees hidden animals. A Goblin that kills a Wolf or Wolf Pack becomes a Wolf Rider. It upgrades to the Wolf Rider (the Clan's Horseman).

**Playing it well.** Cheap and slow: pop the huts near home, hunt wolves early to get Wolf Riders, and use the rest as fodder. It is a poor explorer at 1 move, so let Wolf Riders do the long trips.

**AI: Directed.** The Clan's explorer: it passes only the Explore role (`role_ok`; power 160), and the capital trains one while a fifth of the home landmass is unknown (`production::decide`, explorer step). It moves by `units::explore`: one safe step a turn toward unrevealed land and goody huts, attacking an adjacent enemy at 75% odds or better, which is how a Goblin may beat a wolf and become a Wolf Rider. With nothing left to explore it joins the army as an attacker.

### Harlequin (`UNIT_HARLEQUIN`)
*Recon, tier 3 · 150 hammers · 6/6, 2 moves, 25% withdrawal, -20% city attack, +50% against animals · Animal Handling and a Carnival · Balseraphs (replaces the Ranger)*

**What it does.** The Balseraph ranger is a jester: 6 strength with 25% withdrawal, better tribal villages, sees hidden animals, and Taunt: every adjacent enemy unit that fails to resist must attack the Harlequin's stack. It needs a Carnival instead of a Hunting Lodge, and upgrades to the Beastmaster or the Druid.

**Playing it well.** Park a strong defensive stack next to an enemy army and Taunt: their units throw themselves at your best defender on your terms, often across a river or uphill. In peace it clears animals and barbarians like a Ranger.

**AI: Partial.** Production trains it for its strength only (`role_ok`: Attack steps, power 843 for 150 hammers, below the Horse Archer's 1163 for 120), and the Carnival is built for its own value (happiness, an artist slot, a commerce bonus), not for the unit; Harlequins appeared in fewer games once the enabling-building rule opened better units (20% to 5%, docs/AI.md "Unique units"). Taunt is `bAllowAI` 0, so it is never cast.

### Hellhound (`UNIT_HELLHOUND`)
*Recon, tier 2 · 60 hammers · 3/3 +1 unholy, 3 moves, -20% city attack, +50% against animals, carries one bird · Hunting and a Hunting Lodge · Infernal (replaces the Hunter)*

**What it does.** The Infernal hunter is faster and weaker: 3 strength plus 1 unholy damage and 3 moves, with the Hunter's other abilities.

**Playing it well.** A fast explorer and raider for the late-arriving Infernals: scout the land they join, pick off workers and stragglers, and hunt animals for experience.

**AI: Partial.** As the Hunter: the explorer of choice for the Infernals (`role_ok` Explore, power 230, the 3 moves counting) and an Attack-step pick (600 for 60 hammers); it explores one safe step a turn, then joins the army. Subdue Animal is taken for its AI weight, but captures keep the animal AI, and the bird slot stays empty (the AI trains no Hawks).

### Hunter (`UNIT_HUNTER`)
*Recon, tier 2 · 60 hammers · 4/4, 2 moves, -20% city attack, +50% against animals, carries one bird · Hunting and a Hunting Lodge · every civilization but the Sidar, Clan of Embers and Infernal (who have their own)*

**What it does.** The Scout's successor: 4 strength, 2 moves, +50% against animals, better tribal villages, sees hidden animals, and it carries a Hawk, a flying scout that reveals land without risk. With Subdue Animal it captures the animals it defeats. It upgrades to the Ranger or the Assassin.

**Playing it well.** Explore farther and safer than a Scout, using the Hawk to look ahead; hunt animals for experience and, with Subdue Animal, capture them as cheap troops. In war it raids and kills workers; it is no city attacker.

**AI: Partial.** Production takes it two ways (`role_ok`): as the explorer (Explore power 220 against the Scout's 170, so a capital with a Hunting Lodge and some production trains a Hunter instead) and in Attack steps (500 for 60 hammers, ahead of the Axeman's 400). The Hunting Lodge comes from its own value (health with Deer, happiness with Fur) or from `production::enabler_building` (4 against a Warrior's 3). It explores one safe step a turn (`units::explore`), then joins the army; Subdue Animal is taken for its AI weight 300 (`units::promotion_value`), but captured animals keep the animal AI and roam (`units::barbarian`). Hawks are never trained (no production role), so the cargo slot is never used.

### Lizardman (`UNIT_LIZARDMAN`)
*Recon, tier 2 · 60 hammers · 4/4, 2 moves, -20% city attack, +50% against animals, carries one bird · Hunting and a Hunting Lodge · Clan of Embers and the barbarians (replaces the Hunter); also spawned by Ruins*

**What it does.** The Clan's hunter: the Hunter with the Lizardman race (amphibious attacks, double moves and +10% in marsh, 10% poison resistance). Barbarian Lizardmen come out of Ruins lairs.

**Playing it well.** As the Hunter, at home in marsh and jungle; the Clan's scout once it outgrows the Goblin.

**AI: Partial.** As the Hunter: the Clan's explorer once it outscores the Goblin (Explore power 220 against 160, in a productive capital with a Hunting Lodge) and an Attack-step pick (500); Subdue Animal taken, captures left to the animal AI, no Hawks.

### Myconid (`UNIT_MYCONID`)
*Recon, tier 4 · 240 hammers · 14/14, 2 moves, -20% city attack · Animal Mastery and a Grove · national unit (4, the Beastmaster class) · Khazad (replaces the Beastmaster)*

**What it does.** The Khazad's fungus-folk beastmaster: 14 strength and 2 moves, without the +50% against animals or the bird slot, but with Spores: every living unit not of its team within one plot (neutrals included) is held in place for 3 turns unless it resists.

**Playing it well.** A crowd-controller for dwarven wars: walk next to an enemy stack and Spores freezes it for three turns, so the slow dwarven army can close in and catapults can work on it; or freeze an army marching on your city while you bring defenders. Mind your allies' units next to the target.

**AI: Valued.** Production trains it for its strength (Attack steps, power 1750) once `production::enabler_building` builds the Grove. Spores is modelled by `magic.rs` (immobility worth 25 hit points a turn, up to two, for each enemy attacker next to it, times its chance not to resist; our own units are immune), so it is cast at the end of any turn with enemy attackers next to it; in a census of 190 games each Myconid cast it 16 times (docs/AI.md, "Unique units in play"). The neutral units it would also freeze count for nothing.

### Ranger (`UNIT_LIZARDMAN_RANGER`)
*Recon, tier 3 · 150 hammers · 7/7, 2 moves, -20% city attack, +50% against animals, carries one bird · Animal Handling and a Hunting Lodge · Clan of Embers and the barbarians (replaces the Ranger)*

**What it does.** The Clan's Ranger (`UNIT_RANGER`) with the Lizardman race (amphibious attacks, marsh mobility and +10% there, 10% poison resistance).

**Playing it well.** As the Ranger: clear barbarians and animals around Clan lands; with Subdue Animal its captures add to the horde.

**AI: Partial.** As the Ranger (`UNIT_RANGER`): Attack steps only (power 875), an army unit hunting near our lands; the Clan builds more of them since other civilizations' Centaurs left its roster (docs/AI.md, "Foreign units through `UpgradeCiv`"). Captures keep the animal AI; no Hawks.

### Ranger (`UNIT_RANGER`)
*Recon, tier 3 · 150 hammers · 7/7, 2 moves, -20% city attack, +50% against animals, carries one bird · Animal Handling and a Hunting Lodge · every civilization but the Luchuirp, Khazad, Balseraphs (Harlequin) and Clan of Embers (its own)*

**What it does.** The mid-game recon fighter: 7 strength, 2 moves, +50% against animals, better tribal villages, a bird slot, and -20% against cities. With Subdue Animal (and later Subdue Beasts) it captures animals. It upgrades to the Beastmaster or the Druid.

**Playing it well.** Scout and clear the wilds: barbarians too strong for Hunters, lairs, animals for experience and captures. Leave city attacks to melee units. Its promotions should run Combat into Subdue Animal, then Woodsman or Guerilla for terrain.

**AI: Partial.** Attack steps only (`role_ok`: default AI `UNITAI_EXPLORE` bars Defense, and its `UnitAIs` lack `EXPLORE`), power 875 for 150 hammers: chosen over the Champion (600 for 120) but not over the Horse Archer. As an army unit it does what a Ranger is for, hunting barbarians, animals and lairs near our lands (`military::hunt`), and takes Subdue Animal for its AI weight 300 (`units::promotion_value`); the animals it captures keep the animal AI and roam on their own (`units::barbarian`). The bird slot is never used.

### Satyr (`UNIT_SATYR`)
*Recon, tier 3 · 150 hammers · 9/4, 2 moves, Woodsman I · Animal Handling, the Fellowship of Leaves in the city and as state religion · only by upgrade from a level 4 Fawn · every civilization*

**What it does.** The Fawn grown up: 9 attack, 4 defense, 2 moves, Woodsman I, and Mesmerize Animal: every adjacent animal at war with it that fails to resist joins the caster's side, without a fight. It cannot be built (minimum level 4).

**Playing it well.** Level Fawns to 4 for it: a Satyr walks the forests collecting animals into a free army and hits hard (9 attack) in the woods. Its 4 defense makes it a poor defender; keep it with a stack.

**AI: Incidental.** The AI cannot train it (`iMinLevel` 4); it comes when a Fawn reaches level 4 and `units::upkeep` upgrades it. Mesmerize Animal is cast whenever an animal at war stands next to it (its requirement), valued by FfH's terms at its AI weight 50 (`casting::ffh_value`; `magic.rs` does not model the Python effect). The animals it gains keep the animal AI and roam (`units::barbarian`).

### Scout (`UNIT_SCOUT`)
*Recon, tier 1 · 25 hammers · 2/2, 2 moves, +50% against animals · no tech or building · every civilization but the Mercurians and the Clan of Embers (Goblin)*

**What it does.** The first explorer: 2 strength, 2 moves, +50% against animals, better results from tribal villages (no bad outcomes), sees hidden animals. It upgrades to the Hunter or the Horseman.

**Playing it well.** Build one or two at the start: pop the nearby huts (scouts get the good results), find the neighbours and city sites, and park one on a hill as a lookout. Avoid fighting anything but weak animals; upgrade survivors to Hunters or Horsemen later.

**AI: Directed.** The explorer the production list asks for (`production::decide`, explorer step: the capital trains one while a fifth of the home landmass is unknown, between turns 5 and 150, one at a time, within 10 turns; `role_ok` Explore, power 170). `units::explore` moves it one safe step a turn toward the plot that reveals most (goody huts count extra) and attacks an adjacent enemy only at 75% odds; with nothing safe in reach it waits in a city, and when the land is known it becomes an army unit. Stepping to plots no seen enemy can reach cut scouts killed from 94 to 82 and trained from 87 to 65 in six 150-turn games (docs/AI.md, round 6). It upgrades to the Hunter or Horseman when gold allows (`units::upkeep`).

### Shadow (`UNIT_SHADOW`)
*Recon, tier 4 · 240 hammers · 8/7 +2 poison, 2 moves, 1 first strike, immune to first strikes and defensive strikes, -25% city attack and defense · Guilds and the Council of Esus in the city · national unit (4) · every civilization but the Luchuirp, Khazad and Balseraphs (who have their own)*

**What it does.** The top assassin: invisible to most units, Marksman (fights the weakest unit of a stack), investigates rival cities, explores rival territory, and under an Esus state religion can Steal equipment (20% chance of war per item). Assassins and the other assassin-class units upgrade into it.

**Playing it well.** Hunt enemy summoners, archmages and high priests: invisible, it walks next to their stack and kills them one a turn. It also scouts enemy armies and cities unseen. Counter it with units that see invisible (Marksmen, Rathas) and with Guardsmen.

**AI: Partial.** Attack steps only (`role_ok`; power 1375), in Esus cities, where it loses to the Knight class and the Beastmaster but beats most else; research never values it (`research::unit_available` skips religious and national units). It fights as an army unit; its invisibility and weakest-unit targeting work only through the rules (the strike sends it by its odds), never as a hunt for casters. No investigation (no espionage missions). Steal, with its AI weight 50, is cast whenever its rare condition holds, ignoring the chance of war (`casting::ffh_value`).

### Taskmaster (`UNIT_TASKMASTER`)
*Recon, tier 3 · 120 hammers · 5/3 +1 poison, 2 moves, immune to defensive strikes, -50% city attack and defense · Poisons and a Hunting Lodge · Balseraphs (replaces the Assassin)*

**What it does.** The Balseraph assassin trades the first strike and its immunity for slaving: Marksman, and 25% of its victories over non-animal units produce a Slave (a worker at half speed, which Balseraphs can also put in their Freak Show or sell).

**Playing it well.** Pick off weak units from stacks and turn them into slaves: workers for the Balseraph economy, or bodies for the Freak Show. Without a first strike it is more fragile than other assassins; strike only at good odds.

**AI: Partial.** Attack steps only (`role_ok`; power 750 for 120 hammers, below the other assassins for lacking the first strike), an army unit; the slaves it takes arrive with the worker AI and join the workers (`workers::worker`, counted by production as workers). The sniping works only through the rules' odds.

## Siege

### Cannon (`UNIT_CANNON`)
*Siege, tier 4 · 180 hammers · 8/6, 1 move, withdraws 25%, collateral 100 (up to 7 units, to 60%), bombards 20% a turn · Blasting Powder, a Siege Workshop and Gunpowder · fourteen civilizations*

**What it does.** The late siege engine: it knocks 20% a turn off a city's defense, ignoring the walls' resistance to bombardment, and its attack damages up to seven other units of the defending stack down to 60% of their health. It gets no defensive bonuses and is a poor defender; Catapults and Trebuchets upgrade into it.

**Playing it well.** Two or three Cannons in a siege stack bring any city's defense to zero in a few turns and then strip the garrison with collateral before the Phalanxes go in. Protect them with a strong defender: a stack of Cannons caught alone is a gift. Secure Gunpowder before you plan the war around them.

**AI: Directed.** Trained only for the siege role (`production::siege_engine` keeps it out of the attacker roles): the siege operation asks for one to three engines when its target has three or more defenders or walls of 25% or more (`campaign::needed_force`, `campaign::army_choice`), production keeps a third of the army in siege at war and adds a couple when a stack camps at a city it cannot take (`production::decide`), and the most productive city builds the Siege Workshop when no city can train an engine (`campaign::siege_building`). In the operation the force counts the walls engines can bring down within three turns (`campaign::wall_cut`, which knows Cannons ignore building defense), engines bombard every turn next to the target (`campaign::bombard`), and in the strike they wait while walls stand, then attack first as collateral attackers (`campaign::strike`). Research beelines the siege tech when the operation needs siege and we are decisively stronger (`research::siege_offense`).

### Catapult (`UNIT_CATAPULT`)
*Siege, tier 2 · 90 hammers · 4/3, 1 move, withdraws 80%, damage capped at 75%, collateral 100 (up to 6 units, to 50%), bombards 15% a turn · Construction and a Siege Workshop · every civilization but the Ljosalfar, Khazad and Svartalfar*

**What it does.** The first siege engine: it bombards a city's defense by 15% a turn and, attacking, wounds up to six other units of the stack down to half health; it withdraws from 80% of its losing fights and cannot take a defender below 25% health. No defensive bonuses; it upgrades to the Cannon.

**Playing it well.** The classical war's key unit: a couple per siege stack bombard walls and Palisades away before the attack and then soften the garrison with near-free collateral attacks (the high withdrawal brings most back). Build the Siege Workshop in one production city and ferry engines from there; guard them in the stack.

**AI: Directed.** As the Cannon: the siege role only (`production::siege_engine`), asked for by `campaign::army_choice` and the siege step of `production::decide`, the Siege Workshop from `campaign::siege_building`, bombardment and strike order in `campaign.rs`. Measured: before `PrereqBuilding` buildings counted, no Siege Workshop was ever built and no civilization had a catapult; after, every civilization fielded them (docs/AI.md, "Unique units").

### Dwarven Cannon (`UNIT_DWARVEN_CANNON`)
*Siege, tier 4 · 180 hammers · 10/7, 1 move, withdraws 25%, collateral 100 (up to 7 units, to 60%), bombards 30% a turn · Blasting Powder, a Siege Workshop and Gunpowder · Khazad only, replaces the Cannon*

**What it does.** The Khazad Cannon: stronger (10/7) and the fastest bombard in the game, 30% a turn, ignoring the walls' resistance.

**Playing it well.** As the Cannon, faster: one Dwarven Cannon zeroes most cities' defenses in two or three turns, so a Khazad siege stack needs fewer engines and waits less next to the walls.

**AI: Directed.** As the Cannon (`campaign::army_choice`, `campaign::siege_building`, `campaign::bombard`, `campaign::strike`). As army attackers the Khazad once fielded about 30 Dwarven Cannons a game, which is why siege engines are now trained for the siege role only (`production::siege_engine`); they appear in 35% of Khazad games (docs/AI.md, "Unique units").

### Trebuchet (`UNIT_TREBUCHET`)
*Siege, tier 2 · 90 hammers · 6/4, 1 move, withdraws 80%, damage capped at 75%, collateral 100 (up to 6 units, to 70%), bombards 25% a turn · Construction and a Siege Workshop · Khazad only, replaces the Catapult*

**What it does.** The Khazad Catapult: half again as strong, bombarding 25% a turn, with the same withdrawal and damage cap but a gentler collateral limit (to 70%). It upgrades to the (Dwarven) Cannon.

**Playing it well.** The Khazad's early siege advantage: Trebuchets make Construction a war tech, cracking a rival's walls in a couple of turns while their axemen hold the stack.

**AI: Directed.** As the Catapult (`campaign::army_choice`, `campaign::siege_building`, the strike order in `campaign.rs`); measured: Khazad Trebuchets in 80% of their games once the Siege Workshop counted, against none before (docs/AI.md, "Unique units"). A Khazad-only Workshop rule tried earlier built 2.5 a game and changed no war (docs/AI_LOG.md, "Siege Workshop and Stable for the Khazad").

## Arcane

### Adept (`UNIT_ADEPT`)
*Arcane, tier 2 · 90 hammers · 3/3, 1 move, work rate 300 · Knowledge of the Ether and a Mage Guild in the city · every civilization but the Doviello, the Clan of Embers and the Infernals (they have the Shaman or the Imp) · Channeling I free; upgrades to the Mage*

**What it does.** The entry-level caster: weak in combat, but Channeling I lets it take the first promotion of every sphere whose mana its owner holds, and each sphere opens that sphere's spells. Like every caster with `bFreeXP` it gains experience on its own: each turn a chance of (20 - its experience)% to gain one point, the 20 coming from Channeling I, so it creeps up to level 4 even without fighting (about 70 turns on average from nothing) and can then become a Mage once Sorcery is known. It can also build any of the seventeen mana nodes on a raw mana resource, which is how a civilization makes its own mana. Like every arcane unit it is born with the first promotion of each sphere whose mana its owner has two or more of (three or more with Channeling II: the second; four with Channeling III: the third; `ev_unit_created`). A Balseraph Adept (any arcane unit of theirs) can also cast Create Puppet, a summon with an `iAIWeight` of 1000.

**Playing it well.** The point of an Adept is the Mage it becomes and the nodes it builds. Build one or two early in a Mage Guild city, have them turn raw mana into the spheres your plan needs (Fire for Fireball later, Body for Haste, Earth, Death for skeletons), and keep them safe behind the army while they level. Pick spheres for what the army lacks: a summon or a damage spell for sieges, a lasting blessing for the stack. Never send one to attack; its 3 strength dies to anything.

**AI: Partial.** How it is obtained decides everything. The Amurites and the Svartalfar train it on purpose: `civs::unique_casters` (it names `UNITCLASS_ADEPT`) has their most productive unthreatened city build the Mage Guild and then train casters up to (cities + 2) / 2, through the Mage role (`production::best_unit`), as `UNITAI_MAGE`. Every other civilization that has the Adept never gets one in practice: the Mage Guild has no field the generic building value reads (value 0), `production::enabler_building` only considers defenders and attackers, and the generic caster step (`production::decide` step 10, (cities + 2) / 4 casters) comes after every building worth building, so it fires only in a city that already has a guild (a captured one). Once it has one, the AI uses it fully: Caster role (`magic::caster_target`: with the stack nearest an operation's target, else where its blessings or city spells are worth most), promotions toward the spheres by the spells they open (`magic::promotion_value`, a quarter of the next tier's worth since Channeling II only comes as a Mage), spells cast by `magic.rs` before attacks and at the end of the turn, and the upgrade to the Mage when gold allows (`units::upkeep`). It never builds a mana node: the only code that would (`victory::mana_node`) serves a Tower of Mastery goal, which no AI pursues (`victory::plan_ours` drops it).

### Archmage (`UNIT_ARCHMAGE`)
*Arcane, tier 4 · 240 hammers · 5/5, 1 move, work rate 400 · Strength of Will, Reagents, level 6 (upgrade only) · every civilization but the Khazad and the Sheaim (Eater of Dreams) · national unit, 4 at most · Channeling I-III free*

**What it does.** The top caster: Channeling III opens the third promotion of every sphere it has mana for, with the strongest damage spells and summons, and its strength 5 is still a caster's. It cannot be trained; a Mage (or another tier-3 caster) of level 6 upgrades into it with Reagents in reach. A Malakim Archmage can also cast Summon Sand Lion. Its own passive experience runs to the 90 its three Channeling promotions give.

**Playing it well.** Plan for it from the first Adepts: keep a few casters alive and levelling, secure Reagents, and research Strength of Will once Mages are at level 5-6. Two or three Archmages with the right spheres decide a war: area damage and strong summons in front of every assault, and they can still build nodes. They are national units and slow to replace, so they never lead an attack and always travel in a defended stack.

**AI: Incidental.** No production role takes it (its `iMinLevel` keeps `city_can_train` from it, and its default `UNITAI_RESERVE` would make it an army unit). The AI gets one only when a Mage of level 6 is upgraded by `units::upkeep` (the best upgrade by power, when gold leaves the reserve). An upgraded unit keeps its AI type, so an Archmage that was a trained caster stays `UNITAI_MAGE`: it moves as a caster, takes third-tier spheres by the spells they open (`magic::promotion_value`) and casts by `magic.rs`. Nothing plans for Reagents or Strength of Will for it; research counts the first arcane unit only (`research::tech_value`), so Archmages come rarely and late.

### Eater of Dreams (`UNIT_EATER_OF_DREAMS`)
*Arcane, tier 4 · 240 hammers · 5/5, 1 move, work rate 400 · Strength of Will, Reagents, level 6 (upgrade only) · Sheaim, replaces the Archmage · national unit, 4 at most · Channeling I-III free*

**What it does.** The Sheaim Archmage: everything the Archmage has, plus Consume Soul, cast in one of its own cities: the city loses a citizen and the Eater regains its spell for the turn (it removes the "has cast" mark), so a big city lets it cast twice a turn (two summons, say).

**Playing it well.** Park it in a large Sheaim city near the front and turn surplus population into extra summons when a war needs them, or use it as an ordinary archmage in the field. Population is the price, so feed it from a city at its happiness or health cap.

**AI: Incidental.** As the Archmage: reached only by upgrading a level-6 caster (`units::upkeep`), then a caster like any other. Consume Soul has `bAllowAI` 0, so the rules refuse it to every AI (`unit_can_cast`): the AI's Eater never sacrifices population for a second cast. The Sheaim have no logic of their own (`civs/sheaim.rs` is a stub) and no path builds them a Mage Guild, so the Adepts this needs rarely exist.

### Illusionist (`UNIT_ILLUSIONIST`)
*Arcane, tier 3 · 120 hammers · 4/4, 1 move, work rate 300 · Sorcery, level 4 (upgrade only) · Svartalfar, replaces the Mage · Channeling I-II and Illusionist free; upgrades to the Archmage*

**What it does.** The Svartalfar Mage. Its free Illusionist promotion gives every unit it summons the Illusion promotion: the summon cannot do more than 90% damage (combat limit 90), so it never kills, but it heals fully after each fight and resists death and poison. It takes second-tier spheres like any Mage.

**Playing it well.** Use its summons to wear defenders down to a sliver and let a real unit take the kill and the experience, or to absorb a counterattack, since the illusion is free and comes back each turn. For killing blows prefer damage spells and blessings on your own units over summons.

**AI: Directed.** The Svartalfar keep casters (`civs::unique_casters`: the Mage Guild in their best city, then Adepts up to (cities + 2) / 2), and an Adept of level 4 is upgraded into the Illusionist when gold allows (`units::upkeep`), keeping its caster AI. Research gets 1500 for the tech that completes it (`civs::unique_unit_tech_value`: it is a Svartalfar unique unit). Its spells are cast by `magic.rs`, but the value of a summon (`magic::summon_fight`) is the fight it would win, which an illusion never finishes: the AI does not plan the weaken-then-kill sequence, though a strike's later attackers do profit from the wounds.

### Imp (`UNIT_IMP`)
*Arcane, tier 2 · 90 hammers · 3/3 + 1 unholy, 1 move · Knowledge of the Ether and a Mage Guild in the city · Infernal, replaces the Adept · Channeling I, Demon and Resist Fire free; upgrades to the Mage*

**What it does.** The Infernal Adept: the same caster with a demon's body (Demon: immune to death, poison and unholy damage, fearless, +10% on hell terrain, but weak to holy damage), an extra point of unholy damage and fire resistance. The Infernals also start with one (with Mobility I) among the units their pact gives them (`ffh/events.rs`).

**Playing it well.** As the Adept: build nodes and level toward Mage. The Infernals spread hell terrain, which their demons fight better on, so Imps are sturdier at home than Adepts.

**AI: Valued.** The Infernals have no caster logic of their own (`civs/infernal.rs` handles Manes only), but the rules give every Infernal city a Mage Guild when it is founded or taken (`INFERNAL_CITY_BUILDINGS` in `crates/ffh-core/src/ffh/events.rs`), so the generic caster step of `production::decide` trains Imps there (role Mage, up to (cities + 2) / 4 casters while a unit is affordable). They move as casters (`units::role_of`, `magic::caster_target`) and cast by `magic.rs`; the starting Imp is created with its default `UNITAI_MAGE` and plays the same way. The Infernals are the one civilization outside `civs::unique_casters` whose casters are not blocked by the Mage Guild.

### Mage (`UNIT_MAGE`)
*Arcane, tier 3 · 120 hammers · 4/4, 1 move, work rate 300 · Sorcery, level 4 (upgrade only) · every civilization but the Khazad, the Amurites (Wizard) and the Svartalfar (Illusionist) · Channeling I-II free; upgrades to the Archmage*

**What it does.** The middle caster: Channeling II opens the second promotion of each sphere (Fireball with Fire II, the stronger curses, blessings and summons). It comes only by upgrade from a level-4 Adept, Imp, Shaman or Savant. Its passive experience runs up to 50, enough to reach level 6 and the Archmage. A Malakim Mage can cast Summon Sand Lion (a 5-strength, 3-move summon).

**Playing it well.** The workhorse of a magic army: two or three Mages with Fire II cast a Fireball each next to a city or a stack every turn, softening defenders for free before the melee goes in; Earth, Body or Spirit blessings keep the stack alive. Keep them in the stack, never alone, and level them toward Archmage.

**AI: Incidental.** Reached only by upgrading a level-4 caster in `units::upkeep` (the best upgrade by power, whenever gold stays above twice the reserve of 20 + 3 per city). The casters it comes from exist for the Doviello and the Clan of Embers (Shamans, `civs::unique_casters`) and rarely elsewhere (no Mage Guild, see the Adept), and nothing in research plans Sorcery for the Mage's sake. Once it exists it is a full caster: Caster role, second-tier spheres valued by their spells (`magic::promotion_value`, Fire II for Fireball), spells cast by `magic.rs` before attacks (summons attack the target at once) and at the end of the turn.

### Shaman (`UNIT_SHAMAN`)
*Arcane, tier 2 · 90 hammers · 3/3, 1 move, work rate 300 · Knowledge of the Ether, no building · Doviello and Clan of Embers, replaces the Adept · Channeling I free; upgrades to the Mage*

**What it does.** The barbarian civilizations' Adept, identical in strength and abilities but needing no Mage Guild: any city can train it once Knowledge of the Ether is known.

**Playing it well.** Cheap mana-node builders and future Mages for civilizations whose armies are otherwise all melee: a couple of Shamans that grow into Mages with Fire or Death give a Doviello or Clan army the ranged softening it lacks.

**AI: Directed.** The Doviello and the Clan of Embers keep casters (`civs::unique_casters`): their most productive unthreatened city trains Shamans as casters up to (cities + 2) / 2 while a unit is affordable (no guild to build first). They move and cast as casters (`magic::caster_target`, `magic.rs`), take spheres by the spells they open, and are upgraded to Mages at level 4 when Sorcery and gold allow (`units::upkeep`). Docs/AI.md measured Shamans in 20% -> 80% (Doviello) and 10% -> 70% (Clan) of games once this was in.

### Wizard (`UNIT_WIZARD`)
*Arcane, tier 3 · 120 hammers · 4/4, 1 move, work rate 300 · Sorcery, level 4 (upgrade only) · Amurites, replaces the Mage · Channeling I-II and Spellstaff free; upgrades to the Archmage*

**What it does.** The Amurite Mage, with a Spellstaff: casting Spellstaff (it ignores the "has cast" mark) consumes the staff and restores the Wizard's spell for the turn, one extra cast once.

**Playing it well.** Save the staff for the turn that matters: two Fireballs or two summons on the assault turn, or a summon plus a heal. Otherwise a Mage.

**AI: Directed.** The Amurites train Adepts on purpose (`civs::unique_casters`, Mage Guild first) and upgrade them into Wizards at level 4 (`units::upkeep`); research gets 1500 for the tech that completes it (`civs::unique_unit_tech_value`). The AI never casts Spellstaff: `magic.rs` models it (it removes a promotion) but sees no effect on any unit in it (value 0, `magic::turn_value`), and FfH's terms count the lost Spellstaff promotion (`iAIWeight` 150) against it (`casting::ffh_value`), so the extra cast is never taken. The Wizard otherwise casts and moves as a caster.

## Disciples

### Acolyte (`UNIT_DISCIPLE_THE_ORDER`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Orders from Heaven, a Temple of the Order in a city with the Order · every civilization · Medic I free; upgrades to the Confessor or the Crusader*

**What it does.** The Order's missionary: it can spread the Order to a city (a 40% base chance, halved in another team's city, plus a bonus the fewer religions the city has; the disciple is used up whether it succeeds or not), create a Great Work (+20 culture, used up) and heal the stack it travels with (Medic I, +10% healing on its plot). Like every disciple it cannot pillage.

**Playing it well.** Spread the Order to your own cities early (temples, priests and the religion's happiness), and through open borders to neighbours to build its holy city's income and diplomatic ties. Keep one or two with stacks for healing, and let a surviving Acolyte grow into a Confessor or, with metal, a Crusader.

**AI: Valued.** The missionary step of `production::decide` (step 10, after the buildings worth building) trains one disciple at a time, as `UNITAI_MISSIONARY`, while one of our cities lacks the state religion: `production::best_missionary` takes the first unit that spreads it. The Missionary role (`units::missionary`) walks it, in safe territory, to the most populous of our cities without the religion within 10 turns and spreads there. It never spreads abroad, never creates a Great Work (only great people pursue a culture goal, `victory::great_work`) and never travels with a stack for its Medic. It also passes the Defense role but loses it to the city's ordinary defenders (a Warrior is stronger and costs 25). Missionary-AI disciples below level 2 cannot upgrade (the rules' AI-only gate in `unit_can_upgrade`), and they are spent on spreading, so the Order's priests rarely come this way.

### Confessor (`UNIT_PRIEST_OF_THE_ORDER`)
*Disciple, tier 3 · 120 hammers · 4/4 + 1 holy, 1 move · Priesthood, a Temple of the Order, the Order as state religion · every civilization · Channeling II, Divine, Medic I-II, Spirit Guide free; upgrades to the Prior, Paladin, Druid or Eidolon*

**What it does.** The Order's priest. Its spells: Bless (the Blessed promotion on every living unit in its stack: +1 holy damage and +20% against demons, until the unit fights), Cure Disease (Medic II: removes Diseased, Plagued and Withered from its plot) and Found Temple of the Order (it builds the temple and spreads the Order, sacrificing itself; `bAllowAI` 0). Spirit Guide passes half its experience to another of our units when it dies; its Medic promotions speed healing on its plot. It gains experience on its own up to the 30 its Channeling II gives, slowly enough that level 6 for the Prior takes about 190 turns from nothing.

**Playing it well.** A support unit for an Order army: bless the attack stack before a war against demons or undead, heal the stack, and keep it out of the front line. Its upgrade choice depends on alignment: a Good civilization turns it into a Paladin (and keeps its spells), otherwise the Prior at level 6.

**AI: Partial.** No step trains it as a priest: it passes the Defense role (`UnitAIs` `CITY_DEFENSE`), where it is judged by city defense per hammer (5 with its holy point, +8 `iAIWeight`) and wins only where it beats the city's other defenders per hammer (a productive temple city without an Archery Range, say). Trained that way it is a garrison (`units::garrison_unit`) and casts from the city: Bless on the garrison's units that lack it and Cure Disease where a unit is diseased, both valued by `magic.rs` (`magic::ours_worth`, `reached_value`), never Found Temple. Nothing takes it to an army that is about to fight demons, and its experience-driven upgrade goes to the strongest target by city defense (`units::upkeep`: a Paladin or Eidolon of 14-15 before a Prior of 8).

### Crusader (`UNIT_CRUSADER`)
*Disciple, tier 3 · 120 hammers · 6/6 + 1 holy, 1 move · Fanaticism, a Temple of the Order, the Order as state religion, Copper, Iron or Mithril · every civilization · Demon Slaying I, Immune to Disease, Medic I free; upgrades to the Paladin*

**What it does.** The Order's holy soldier: as strong as a Champion with a point of holy damage, +40% against demons (Demon Slaying I), immune to Diseased and Plagued, and a healer for its stack.

**Playing it well.** The answer to the Ashen Veil, the Infernals and demon summons, and to their disease: send Crusaders where Diseased Corpses and plague have been spread, and against Eidolons and Balors. Against ordinary armies it is a decent Champion that heals.

**AI: Valued.** Trained for the Defense and Attack roles (`production::role_ok`: `ATTACK`, `CITY_COUNTER`, `COUNTER` in its `UnitAIs`) by strength per hammer, holy damage included (`production::type_city_defense`, `best_unit`); it then fights as an army unit (`military::army`, the siege operation in `campaign.rs`) or garrisons. Demon Slaying, the disease immunity and the Medic do not count in its power, and no one sends it against demons in particular.

### Cultist (`UNIT_PRIEST_OF_THE_OVERLORDS`)
*Disciple, tier 3 · 120 hammers · 5/5, 1 move · Priesthood, a Temple of the Overlords, the Octopus Overlords as state religion · every civilization · Channeling II, Divine, Medic I-II, Water Walking free; upgrades to the Speaker, Paladin, Druid or Eidolon*

**What it does.** The Overlords' priest, walking on water (Water Walking: amphibious and crossing rivers freely). Its spells: Cure Disease, Found Temple of the Overlords (`bAllowAI` 0) and Tsunami (`bAllowAI` 0: range 2 damage to units and improvements on water and coast, and it can only be cast next to water). It gains experience on its own up to 30.

**Playing it well.** Use its water walking to strike coastal cities and stacks from the sea, and Tsunami against units on coast plots before a landing. It levels by itself toward the Speaker.

**AI: Partial.** Trained only through the Defense role by city defense per hammer (5 + 8 `iAIWeight`), then a garrison (`units::garrison_unit`); it never uses the water walking. Of its spells only Cure Disease is cast (by `magic.rs` when a unit on its plot is diseased); Tsunami and Found Temple are `bAllowAI` 0. Upgrades by city defense (`units::upkeep`).

### Disciple of Leaves (`UNIT_DISCIPLE_FELLOWSHIP_OF_LEAVES`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Way of the Forests, a Temple of Leaves in a city with the Fellowship · every civilization · Medic I free; upgrades to the Priest of Leaves or the Ranger*

**What it does.** As the Acolyte, for the Fellowship of Leaves: spreads it (40% base), creates a Great Work (+20 culture), heals its stack, cannot pillage.

**Playing it well.** As the Acolyte; a surviving one upgrades into a Priest of Leaves or a Ranger.

**AI: Valued.** As the Acolyte: the missionary step trains one at a time for the state religion and it spreads only in our own cities (`units::missionary`); no Great Work, no healing duty, rarely an upgrade. Its `iAIWeight` of -20 makes it lose the Defense role even more surely.

### Druid (`UNIT_DRUID`)
*Disciple, tier 4 · 240 hammers · 8/8, 2 moves, -20% city attack, carries 1 bird · Commune with Nature, a Grove in the city, Neutral alignment · every civilization but the Luchuirp and the Khazad (Dwarven Druid), the Grigori (Luonnotar) and the Clan of Embers (Lizardman Druid) · national unit, 4 at most · Channeling I-III, Medic I, Nature I free*

**What it does.** A strong, mobile nature priest: it moves through impassable terrain (peaks), gets better results from tribal villages, has +50% against animals and +1 Nature affinity, and starts with all three Channeling promotions and Nature I, so it learns the Nature sphere and more. Its own spells: Entangle (range 1, resistable: enemy units, not flying ones, cannot move for 2 turns) and Treetop Defense (in a forest: our units on its plot get five turns of fortification). It gains experience on its own up to 90.

**Playing it well.** Neutral civilizations' elite: Entangle a stack before it reaches your lines or to pin defenders before a siege, fortify a forest line with Treetop Defense, and use the peaks to flank. Its -20% city attack makes it a poor city assaulter; use it to clear the field and as a caster.

**AI: Valued.** It passes the Attack role (`UNITAI_ATTACK` in `UnitAIs`; its default `EXPLORE` keeps it out of Defense) and is scored by strength per hammer with its moves (`production::best_unit`), which suits an 8-strength, 2-move unit; the Grove comes through `production::enabler_building` when the Druid is a third stronger than the city's best attacker. Trained as an attacker it fights as an army unit (`military::army`, `campaign.rs`). Entangle is valued by `magic.rs` (immobility on enemies that can attack, `magic::reached_value`) at the end of the turn and before strikes; Treetop Defense only by its `iAIWeight` of 10 when nothing else is worth casting (`casting::ffh_value`). Its peak movement and village bonus are not used on purpose.

### Druid (`UNIT_LIZARDMAN_DRUID`)
*Disciple, tier 4 · 240 hammers · 8/8, 2 moves, -20% city attack · Commune with Nature, a Grove, Neutral alignment · Clan of Embers (and the barbarians), replaces the Druid · national unit, 4 at most · Channeling I-III, Lizardman, Medic I, Nature I free*

**What it does.** The Clan's Druid: the Druid's body and Nature magic with the Lizardman race (amphibious, crosses rivers freely, +10% attack and defense and double moves in marsh, 10% poison resistance), but without Entangle (which only `UNIT_DRUID` casts); Treetop Defense remains.

**Playing it well.** As the Druid, minus the pinning spell: a mobile 8-strength fighter and caster for the Clan's marsh-and-river fronts, taking the Nature sphere for summons and blessings.

**AI: Valued.** As the Druid: the Attack role and `enabler_building`'s Grove, an army unit, Treetop Defense by its weight of 10, sphere spells by `magic.rs`.

### Dwarven Druid (`UNIT_DWARVEN_DRUID`)
*Disciple, tier 4 · 240 hammers · 8/8, 2 moves, -20% city attack · Commune with Nature, a Grove, Neutral alignment · Luchuirp and Khazad, replaces the Druid · national unit, 4 at most · Channeling I-III, Dwarven, Earth I, Medic I free*

**What it does.** The dwarves' Druid commands earth instead of nature: Earth I instead of Nature I (+1 Earth affinity), the Dwarven race (double moves in hills, 25% poison resistance), and two spells of its own: Crush (the plot within 2 that holds the most enemy units and no one else takes 50 physical damage on each unit, to at most 75%) and Wall of Stone (a city building, +25% defense and 25 bombard defense, kept only while a caster who can cast it stays in the city).

**Playing it well.** Crush is a strong pre-assault strike on an enemy stack two plots away; Wall of Stone turns a threatened frontier city into a fortress as long as the Druid stays. Take the Earth sphere further for its blessings.

**AI: Valued.** The Attack role and the Grove as for the Druid. Crush is one of the Python spells `magic.rs` models (`Py::Crush`: 50 damage, limit 75, on the best enemy-only plot within 2), cast before strikes and at the end of the turn; Wall of Stone is valued as a quarter of the building's value in its city (`magic::turn_value`), and a caster that keeps a `bRequiresCaster` building stays (`magic::caster_target`, `upkeep_value`). As an army unit it is not held in a city for the wall, though.

### Ecclesiastic (`UNIT_DISCIPLE_EMPYREAN`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Honor, a Temple of the Empyrean in a city with the Empyrean · every civilization · Medic I free; upgrades to the Vicar*

**What it does.** As the Acolyte, for the Empyrean: spreads it (40% base), creates a Great Work (+20 culture), heals its stack.

**Playing it well.** As the Acolyte; it upgrades into the Vicar, whose Revelation finds hidden units.

**AI: Valued.** As the Acolyte: one at a time for the state religion, our own cities only (`units::missionary`), no Great Work, rarely an upgrade.

### Eidolon (`UNIT_EIDOLON`)
*Disciple, tier 4 · 240 hammers · 11/11 + 4 unholy, 1 move · Malevolent Designs, Evil alignment, Iron or Mithril · every civilization · national unit, 4 at most · Demon and March free; +1 to the Armageddon counter*

**What it does.** The evil counterpart of the Paladin: a demon (immune to death, poison and unholy damage, fearless, weak to holy damage, +10% on hell terrain) of 15 effective strength that heals every turn wherever it is (March: +10% healing in friendly, neutral and enemy land, even after moving). Melee units and priests can upgrade into it and keep their promotions and spells, so an Eidolon made from a priest still casts.

**Playing it well.** Upgrade experienced Champions or priests into Eidolons rather than building them: they keep their promotions. March makes it an endless attacker that never needs to stop to heal. Paladins and Crusaders, with holy damage and Demon Slaying, are what kills it.

**AI: Valued.** Trained for the Defense and Attack roles by strength per hammer, unholy damage included (`production::best_unit`), and fights as an army unit or garrison; it is also the strongest upgrade by power for an evil civilization's priests and champions (`units::upkeep`). March, the demon resistances and the Armageddon point are not counted.

### High Priest of Leaves (`UNIT_HIGH_PRIEST_OF_LEAVES`)
*Disciple, tier 4 · 240 hammers · 8/8, 1 move · Theology, Incense, the Fellowship as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free; leaves a civilization without the religion*

**What it does.** The Fellowship's high priest: Bloom (a forest on its plot in our borders, after 2 turns), Cure Disease, Heal (Medic III: 20 hit points to every living unit on its plot and Poisoned removed), Summon Tiger (a permanent 4-strength, 2-move tiger), Summon Treant (in a forest or ancient forest: a permanent 10-strength treant). Medic III heals its plot and the plots around it +20%.

**Playing it well.** A forest general: plant forests, summon treants on forest plots before a war (each is a permanent 10-strength unit for free), and heal the army. Keep it in the stack; a national unit lost is hard to replace.

**AI: Incidental.** Reached only from a level-6 Priest of Leaves (`units::upkeep`), and that upgrade takes the strongest target by power, so a Druid, Paladin or Eidolon of the same priest wins when one is available. Once it exists it casts by `magic.rs`: Heal on its hurt stack (`Py::Heal`), Cure Disease, Summon Tiger and Summon Treant as lasting summons while at war or with enemies about (`magic::summon_value`: strength x 10), Bloom by FfH's flat +10 for a feature (`casting::ffh_value`). As a garrison (the AI it inherits) it is not walked to forests to summon treants.

### High Priest of Winter (`UNIT_HIGH_PRIEST_OF_WINTER`)
*Disciple, tier 4 · 240 hammers · 6/6 + 3 cold, 1 move · Theology, level 6 (upgrade only) · Illians · national unit, 4 at most · Channeling I-III, Divine, Ice I, Immune to Cold, Medic I-III, Winterborn free*

**What it does.** The Illian high priest, upgraded from the three Priests of Winter of The White Hand ritual: 3 cold damage, immune to cold, Winterborn (+10% attack and defense on snow and tundra), Medic III, and spells: Call Blizzard (moves a blizzard from a neighbouring plot onto its own, turning grassland, plains or desert there into tundra), Cure Disease, Heal, and Slow (Ice I: range 1, resistable, enemy units lose a move; removes Haste).

**Playing it well.** Slow an enemy stack so it cannot reach you or flee, heal your army, and spread the blizzards that make Illian land deadly to others. Call Blizzard harms farmland; use it on enemy approaches, not in your own core.

**AI: Incidental.** The Illians build The White Hand in their most productive city (`civs/illians.rs`); its priests are army units (their default `UNITAI_RESERVE`) and become High Priests of Winter only when one reaches level 6 and the upgrade is affordable (`units::upkeep`). Slow is valued by `magic.rs` (a curse on enemies that can attack, more when it stops them), Heal and Cure Disease too; Call Blizzard is cast by its `iAIWeight` of 50 whenever a blizzard is adjacent and nothing is worth more (`casting::ffh_value`), whatever farmland it turns to tundra.

### Lightbringer (`UNIT_LIGHTBRINGER`)
*Disciple, tier 1 · 60 hammers · 2/2, 1 move · no tech · Malakim · Sentry free; upgrades to any of the six disciples*

**What it does.** The Malakim's early religious scout: weak, with +1 sight (Sentry), unable to pillage, and able to upgrade into the disciple of whatever religion the Malakim adopt.

**Playing it well.** Post Lightbringers on hills around your land to watch for barbarians and stop them spawning in the fog, then upgrade them into the disciples of your religion when it comes.

**AI: Unused.** It passes only the Defense role (`UNITAI_COUNTER`), where its city defense (2, `iAIWeight` 0) at 60 hammers always loses to the Warrior's (3, +25% in cities, 25 hammers) or, once the Warrior is obsolete, to the units that replace it, in `production::best_unit`; no other step names it, so the AI never trains one and never posts sentries (fog-busting posts were tried and reverted, docs/AI_LOG.md).

### Luonnotar (`UNIT_LUONNOTAR`)
*Disciple, tier 4 · 240 hammers · 10/10, 1 move · Strength of Will, level 6 (upgrade only) · Grigori, replaces the Druid · national unit, 4 at most · Channeling II-III, Inquisitor, Magic Immune, Medic I free*

**What it does.** The Grigori's top disciple, upgraded from a Grigori Medic or a priest at level 6: 10 strength, +50% against disciple units, immune to magic (and half resistant to cold, death, fire and lightning damage), a healer, and Inquisition (in a city of ours or of a civilization with our state religion: removes every non-state religion that is not a holy city there, with its buildings; takes 3 turns).

**Playing it well.** A strong, spell-proof field unit and the Grigori's religion cleaner: inquisition conquered cities to rid them of foreign temples and unrest, and use it against priest-heavy armies.

**AI: Incidental.** The Grigori train Medics at war (`civs/grigori.rs`: one per 4 attackers, at most 3, as army units), and a level-6 Medic is upgraded into the Luonnotar when gold allows (`units::upkeep`). It then fights as an army unit. Inquisition is never cast: it is an ability `magic.rs` does not model and FfH's terms give it 0 (`casting::ffh_value`).

### Luridus (`UNIT_HIGH_PRIEST_OF_THE_EMPYREAN`)
*Disciple, tier 4 · 240 hammers · 6/6 + 1 fire, 1 holy, 1 move · Theology, Incense, the Empyrean as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free*

**What it does.** The Empyrean high priest: Crown of Brilliance (30 holy damage, to at most 60%, on every enemy unit within 1, and the Crown promotion on itself: it sees invisible units and repeats the damage each turn until the crown fades, a 25% chance a turn), Revelation (range 3: strips enemy units of hidden, invisible and hidden-nationality status, destroys enemy illusions and dispels their blessings; only when such a unit is within 3), Cure Disease and Heal.

**Playing it well.** Walk it next to an enemy stack and crown: the repeated holy damage softens everything around for several turns. Revelation is the counter to Svartalfar illusions, hidden assassins and pirates.

**AI: Incidental.** Reached only by upgrading a level-6 Vicar, after stronger targets (`units::upkeep`). Its spells are all valued by `magic.rs`: Crown of Brilliance as damage on enemies in reach (`magic::expected_damage`, before strikes and at the end of the turn), Revelation as a dispel, Heal and Cure Disease on our hurt and diseased units; the lasting crown's repeated damage is not counted.

### Monk (`UNIT_MONK`)
*Disciple, tier 3 · 120 hammers · 6/6, 2 moves · Priesthood, no building · Elohim · Demon Slaying I, Medic I free; upgrades to the Immortal or the Paladin*

**What it does.** The Elohim's mobile holy fighter: 6 strength and 2 moves, +40% against demons, a healer for its stack, trainable in any city without a temple.

**Playing it well.** Fast stack healers and demon hunters for a peaceful Elohim, good escorts and a quick reaction force; upgrade the experienced ones into Paladins.

**AI: Valued.** Trained for the Defense and Attack roles by strength per hammer, its second move counted (`production::best_unit`), and used as an army unit or garrison. Demon Slaying and the Medic are not counted.

### Paladin (`UNIT_PALADIN`)
*Disciple, tier 4 · 240 hammers · 11/11 + 3 holy, 1 move · Righteousness, Good alignment, Iron or Mithril · every civilization · national unit, 4 at most · Courage, Demon Slaying I, Immune to Disease, Medic I free*

**What it does.** The good civilizations' elite: 14 effective strength with holy damage, +40% against demons, immune to disease, fearless (Courage, +10% healing on its plot), a healer. Many units upgrade into it (champions, crusaders, monks, priests), keeping their promotions and spells.

**Playing it well.** Upgrade your best Champions, Crusaders and priests into Paladins rather than building them. They lead the army against evil civilizations and their demons and undead.

**AI: Valued.** Trained for the Defense and Attack roles by strength per hammer, holy damage included, then an army unit or garrison; and the strongest upgrade by power for a good civilization's priests, crusaders and monks (`units::upkeep`). Demon Slaying, Courage and the disease immunity are not counted.

### Paramander (`UNIT_PARAMANDER`)
*Disciple, tier 3 · 120 hammers · 7/7, 1 move · Fanaticism, a Temple of Kilmorph, Copper, the Runes of Kilmorph as state religion · every civilization · Demon Slaying I, Medic I free; upgrades to the Paladin*

**What it does.** The Runes of Kilmorph's soldier: 7 strength for 120 hammers, +40% against demons, a healer. It is also the upgrade of the Soldier of Kilmorph and the Thane.

**Playing it well.** A strong mid-game defender and attacker for Kilmorph civilizations with copper: one of the best strengths per hammer of its tier.

**AI: Valued.** Trained for the Defense and Attack roles by strength per hammer (`production::best_unit`), then an army unit or garrison; Demon Slaying and the Medic not counted.

### Priest of Leaves (`UNIT_PRIEST_OF_LEAVES`)
*Disciple, tier 3 · 120 hammers · 5/5, 1 move · Priesthood, a Temple of Leaves, the Fellowship as state religion · every civilization · Channeling II, Divine, Medic I-II free; upgrades to the High Priest of Leaves, Paladin, Druid or Eidolon*

**What it does.** The Fellowship's priest: Bloom (a forest on its plot in our borders, after 2 turns), Cure Disease, Summon Tiger (a permanent 4-strength, 2-move tiger, which cannot move for 2 turns after it appears) and Found Temple of Leaves (`bAllowAI` 0). It gains experience on its own up to 30.

**Playing it well.** Plant forests where they help (Fellowship buildings and elves love them), summon a tiger each turn in wartime for a free stream of fighters, heal the army.

**AI: Partial.** Trained only through the Defense role by city defense per hammer (5 + 8 `iAIWeight`), then a garrison. From its city it casts Summon Tiger at war or with enemies about (`magic::summon_value`, strength x 10 for a lasting summon), and the tigers join the army as attackers; Cure Disease when needed; Bloom by FfH's +10 when nothing else is worth casting. Never Found Temple.

### Prior (`UNIT_HIGH_PRIEST_OF_THE_ORDER`)
*Disciple, tier 4 · 240 hammers · 6/6 + 2 holy, 1 move · Theology, Incense, the Order as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free*

**What it does.** The Order's high priest: Bless and Cure Disease like the Confessor, Heal (20 hit points to every living unit on its plot), and Unyielding Order (in one of our cities: ends the occupation timer, cuts hurry anger, and a building that removes unhappiness and maintenance there while a caster who can cast it stays).

**Playing it well.** Park it in a freshly conquered or rioting city: Unyielding Order ends the resistance at once and keeps it content. Otherwise a healer and blesser for the army.

**AI: Incidental.** Only by upgrading a level-6 Confessor, after the Paladin, Druid or Eidolon when one is available (`units::upkeep`). Unyielding Order is valued as a quarter of its building's value (`magic::turn_value`; the building's -100% maintenance reads as gold), and a caster keeping it stays in its city (`magic::upkeep_value`); the occupation-ending effect is Python and not valued, so it is not brought to conquered cities. Bless, Heal and Cure Disease by `magic.rs`.

### Profane (`UNIT_HIGH_PRIEST_OF_THE_VEIL`)
*Disciple, tier 4 · 240 hammers · 6/6 + 2 unholy, 1 move · Theology, Incense, the Ashen Veil as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free*

**What it does.** The Ashen Veil's high priest: Ring of Flames (15 fire damage, to at most 40%, on every unit within 1, and it sets the forests and jungles around on fire; it declares war on a neutral it hurts), Hellfire (for an Infernal caster only: a Hellfire improvement on a land plot with no city or improvement and no other Hellfire within 2; an AI only outside its own land), Summon Balor (a temporary 7-strength demon), Cure Disease and Heal.

**Playing it well.** A front-line destroyer for the Veil: Ring of Flames on stacks adjacent to it, a Balor every turn before an assault, and heal the army. Mind the war it declares on anyone caught in the flames.

**AI: Incidental.** Only by upgrading a level-6 Ritualist, after stronger targets (`units::upkeep`). Its combat spells are valued by `magic.rs`: Ring of Flames as damage (never where it would hit a neutral, `magic::reached_value`), Summon Balor as the fight the Balor would win next to the target (`magic::summon_fight`); Hellfire, castable only by an Infernal Profane, by FfH's +10 for an improvement; Heal and Cure Disease on hurt units.

### Ritualist (`UNIT_PRIEST_OF_THE_VEIL`)
*Disciple, tier 3 · 120 hammers · 4/4 + 1 unholy, 1 move · Priesthood, a Temple of the Veil, the Ashen Veil as state religion · every civilization · Channeling II, Divine, Medic I-II, Unholy Taint free; upgrades to the Profane, Paladin, Druid or Eidolon*

**What it does.** The Veil's priest. Unholy Taint weakens it (-25% attack and defense and -1 strength, +1 unholy damage, +10% spell damage, 25% unholy resistance). Spells: Ring of Flames (15 fire damage, to at most 40%, on every unit within 1, burning the woods around), Cure Disease and Found Temple of the Veil (`bAllowAI` 0). It gains experience on its own up to 30.

**Playing it well.** Bring it next to an enemy stack and burn it before your attack; it is a poor fighter itself. Level it toward the Profane.

**AI: Partial.** Trained only through the Defense role (rated at 5 with its unholy point; the Taint's penalty is not counted), then a garrison. Ring of Flames is valued by `magic.rs` and cast from the city on enemies adjacent to it (never when a neutral would be hit); nothing brings it to the front. Cure Disease when needed, never Found Temple.

### Runekeeper (`UNIT_HIGH_PRIEST_OF_KILMORPH`)
*Disciple, tier 4 · 240 hammers · 7/7 + 1 holy, 1 move · Theology, Gems, the Runes of Kilmorph as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free*

**What it does.** Kilmorph's high priest: Earthquake (every plot within 1: units lose their fortification, a quarter of each city's buildings and improvements may be destroyed, and it declares war on the owners of cities and improvements it hits; for the AI only when every owned plot in range belongs to a team at war with it), Shield of Faith (+10% attack and +30% defense for its stack's units, lasting), Cure Disease and Heal.

**Playing it well.** Shield the army before a war, heal it, and quake an enemy city from an adjacent plot to strip its defenses and buildings before the assault.

**AI: Incidental.** Only by upgrading a level-6 Stonewarden, after stronger targets (`units::upkeep`). Shield of Faith is valued by `magic.rs` as a lasting blessing on the units that lack it; Earthquake before a strike at 15 per enemy unit in the target city (`magic::py_value`), although that value does not check that the caster stands next to the city (it is cast from up to two plots away, where the quake misses the city); Heal and Cure Disease when needed.

### Savant (`UNIT_DISCIPLE_THE_ASHEN_VEIL`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Corruption of Spirit, a Temple of the Veil in a city with the Veil · every civilization · Medic I free; upgrades to the Ritualist or the Mage*

**What it does.** The Veil's disciple: it spreads the Ashen Veil (40% base), heals its stack, and instead of a Great Work it can "discover": it is used up for 15 research points toward a technology (scaled by game speed), a token amount next to a great person's lightbulb. It is the only disciple that can upgrade into an arcane unit, the Mage.

**Playing it well.** Spread the Veil; the discovery is not worth a unit, but a Savant kept with a stack can level into a Ritualist or, outside the Mage Guild path, a Mage.

**AI: Valued.** As the Acolyte: the missionary step trains one at a time for the state religion and it spreads in our own cities only. It never discovers a technology (only great people use `MISSION_DISCOVER`, `units::great_person`) and is rarely upgraded (spent on spreading; the missionary upgrade gate).

### Speaker (`UNIT_HIGH_PRIEST_OF_THE_OVERLORDS`)
*Disciple, tier 4 · 240 hammers · 7/7 + 1 unholy, 1 move · Theology, Incense, the Octopus Overlords as state religion, level 6 (upgrade only) · every civilization · national unit, 4 at most · Channeling II-III, Divine, Medic I-III free*

**What it does.** The Overlords' high priest: Summon Kraken (next to water: a permanent 17-strength, 4-move sea monster), Tsunami (`bAllowAI` 0), Cure Disease and Heal.

**Playing it well.** Raise a kraken fleet from the coast: each cast is a permanent ship-killer that dominates the seas around, and Tsunami hits coastal stacks and improvements.

**AI: Incidental.** Only by upgrading a level-6 Cultist, after stronger targets (`units::upkeep`). Summon Kraken is valued as a lasting summon while at war or with enemies about (`magic::summon_value`); the krakens it makes are naval units this AI does not command (they hold, `units::play`). Tsunami is refused to the AI; Heal and Cure Disease when needed.

### Stonewarden (`UNIT_PRIEST_OF_KILMORPH`)
*Disciple, tier 3 · 120 hammers · 5/5, 1 move · Priesthood, a Temple of Kilmorph, the Runes of Kilmorph as state religion · every civilization · Channeling II, Divine, Medic I-II free; upgrades to the Runekeeper, Paladin, Druid or Eidolon*

**What it does.** Kilmorph's priest: Shield of Faith (+10% attack and +30% defense for its stack's units, until dispelled), Cure Disease and Found Temple of Kilmorph (`bAllowAI` 0). It gains experience on its own up to 30.

**Playing it well.** Shield every stack before a war and every garrison under threat; a lasting +30% defense on a whole stack is one of the best blessings of its tier.

**AI: Partial.** Trained only through the Defense role by city defense per hammer, then a garrison: from its city it casts Shield of Faith on the garrison's units that lack it (`magic.rs`, a lasting blessing: `magic::ours_worth`), which helps the city, but no one takes it to the armies, where the blessing would matter more. Cure Disease when needed, never Found Temple.

### Thane of Kilmorph (`UNIT_DISCIPLE_RUNES_OF_KILMORPH`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Way of the Earthmother, a Temple of Kilmorph in a city with the Runes · every civilization · Medic I free; upgrades to the Stonewarden or the Paramander*

**What it does.** As the Acolyte, for the Runes of Kilmorph: spreads it (40% base), creates a Great Work (+20 culture), heals its stack.

**Playing it well.** As the Acolyte; with copper a surviving Thane upgrades into the strong Paramander.

**AI: Valued.** As the Acolyte: one at a time for the state religion, our own cities only; `iAIWeight` -20; no Great Work, rarely an upgrade.

### Vicar (`UNIT_PRIEST_OF_THE_EMPYREAN`)
*Disciple, tier 3 · 120 hammers · 4/4 + 1 holy, 1 move · Priesthood, a Temple of the Empyrean, the Empyrean as state religion · every civilization · Channeling II, Divine, Medic I-II free; upgrades to the Luridus, Paladin, Druid or Eidolon*

**What it does.** The Empyrean's priest: Revelation (range 3: strips enemy units of hidden, invisible and hidden-nationality status, destroys enemy illusions and dispels their blessings; castable only when such a unit is within 3), Cure Disease and Found Temple of the Empyrean (`bAllowAI` 0). It gains experience on its own up to 30.

**Playing it well.** The detector: keep one with each army and in border cities against Svartalfar illusions, Sidar ghosts and hidden-nationality raiders.

**AI: Partial.** Trained only through the Defense role, then a garrison. Revelation is valued by `magic.rs` as a dispel of enemy blessings in range (`magic::reached_value`), so a garrison Vicar casts it when such units come within 3 of its city; it is not sent with the armies. Cure Disease when needed, never Found Temple.

### Zealot (`UNIT_DISCIPLE_OCTOPUS_OVERLORDS`)
*Disciple, tier 2 · 60 hammers · 3/3, 1 move · Message from the Deep, a Temple of the Overlords in a city with the Overlords · every civilization · Medic I free; upgrades to the Cultist or the Stygian Guard*

**What it does.** As the Acolyte, for the Octopus Overlords: spreads it (40% base), creates a Great Work (+20 culture), heals its stack.

**Playing it well.** As the Acolyte; a surviving one becomes a Cultist or a Stygian Guard.

**AI: Valued.** As the Acolyte: one at a time for the state religion, our own cities only; `iAIWeight` -20; no Great Work, rarely an upgrade.

## Naval

### Airship (`UNIT_AIRSHIP`)
*Naval combat type, land domain, tier 4 · 300 hammers · 3/9, 3 moves, Flying, carries 6 land units · Astronomy, Engineering and Iron or Mithril, no Shipyard · Kuriotates (replaces the Queen of the Line)*

**What it does.** A flying transport: it moves over land and water, carries six land units, defends at 9, and can be built in any city, coastal or not. Frigates of the Kuriotates upgrade into it.

**Playing it well.** It frees the Kuriotates from coasts: an inland city builds the fleet, and six units fly over mountains or straits to land next to an enemy city with no navy able to intercept. Escort it or keep it out of reach; at 3 attack it is no warship.

**AI: Unused.** No production role takes it: `best_unit` asks transports from the sea domain only, and `naval::is_transport_type` requires a sea unit, so no boarding, settler ferry or assault ever uses it. It can appear only when a Kuriotate Frigate upgrades and the Man O'War is not available (`units::upkeep` takes the stronger, 13 against 3); `units::role_of` then sees a land unit with strength and runs it as an army unit (`military::army`), its cargo empty. Its class counts in the enemy-fleet test for Raging Seas (`world_spells.rs`).

### Arcane Barge (`UNIT_ARCANE_BARGE`)
*Naval, tier 3 · 250 hammers · 8/8, 3 moves, carries 1 · Astronomy and Sorcery · every civilization*

**What it does.** A floating fire mage: 8 strength, 3 moves, room for one unit, and Fire II from birth, so it casts Fireball every turn (a temporary 4-strength fire summon with collateral damage that bombards city defenses).

**Playing it well.** Coastal fire support: sail next to an enemy coastal city or a stack on the shore and fireball it each turn before your landing, or burn enemy ships' escorts. It is expensive for its hull; one or two with an invasion fleet pay for themselves.

**AI: Unused.** It passes only the Transport role (`production::role_ok`: one cargo slot), where its score (230 for 250 hammers: `best_unit` weighs cargo, moves and strength per turn) is a third of the Caravel's per hammer (255 for 100), and the Caravel is always trainable by then (Astronomy requires Optics). So it is never trained, and nothing upgrades into it. The Fireball it could cast is modelled for land targets (`magic::summon_value` skips water plots for a land summon).

### Caravel (`UNIT_CARAVEL`)
*Naval, tier 3 · 100 hammers · 7/7, 4 moves, carries 1, Sentry · Optics · every civilization*

**What it does.** The first ocean-going ship: 4 moves, one cargo slot, Sentry (+1 sight), and it can enter rival territory without open borders. Galleys upgrade into it; it upgrades to the Galleon.

**Playing it well.** Map the oceans and meet the other continents early, carry a lone settler to an island, or ferry a scout or hero. It is a poor warship; once Astronomy is in, Galleons carry three times as much.

**AI: Directed.** It serves both of the AI's naval jobs. As a transport (`role_ok` Transport, score 255 for 100 hammers, within a few percent per hammer of the Trireme and Privateer, and the only one of them that needs no resource) it is the ship a port builds for a settler waiting in port for an overseas site and for assault fleets (`production::decide`, `naval::assault_production`; run by `naval::transport`), and research heads for Optics when a settler waits and no port can train a ship (`research::transport_step_tech`: Optics for the Caravel without copper). As a sea explorer (360 for 100) it replaces the Galley once that is obsolete, reveals coasts while our land is full (`naval::sea_explore`), then becomes a transport. It never fights (no naval combat logic), and its open-borders exploring is used only as the explorer's path allows. It upgrades to the Galleon.

### Frigate (`UNIT_FRIGATE`)
*Naval, tier 3 · 150 hammers · 10/10, 3 moves, bombards 20% a turn, carries 2 · Optics and Iron or Mithril · every civilization*

**What it does.** The first true warship: 10 strength, bombards a coastal city's defenses by 20% a turn, and carries two units. Galleys and Triremes upgrade into it; it upgrades to the Man O'War or the Queen of the Line.

**Playing it well.** Control the coastal seas: sink enemy transports and galleys, bombard coastal walls before a landing, and carry the first two units of an invasion. Upgrade Triremes into it as soon as iron is connected.

**AI: Incidental.** Production never prefers it: as a transport it scores 340 for 150 hammers (2.3 a hammer against the Caravel's 2.6), as a sea explorer 330 for 150 against the Galley's 230 for 50 and the Caravel's 360 for 100 (`production::best_unit`). It arrives by upgrade: Galley explorers and Trireme transports take it as their strongest upgrade (`units::upkeep`) and keep their job, exploring or carrying (`naval.rs`). It never fights, escorts or bombards (no naval combat logic; `campaign::bombard` is for land stacks).

### Galleon (`UNIT_GALLEON`)
*Naval, tier 3 · 150 hammers · 7/7, 5 moves, carries 3 · Astronomy and Iron or Mithril · every civilization*

**What it does.** The great transport of the age: three cargo slots and 5 moves for 150 hammers, a weak 7 strength. Caravels upgrade into it.

**Playing it well.** Move armies: two or three Galleons land a real invasion force in one wave, and settle distant islands with settler and escort together. Keep them away from enemy warships or escort them with Frigates.

**AI: Directed.** The AI's transport of choice once it has Astronomy and iron or mithril: the best Transport score per hammer (485 for 150; only the Queen of the Line, behind a Shipyard the AI never builds, is a hair better), so the settler-ship step, the emergency ship for a waiting settler (`production::emergency`) and the assault fleet (`naval::assault_production`: as many as one wave of the force needs, `need / cargo`) build Galleons, and `naval::transport` ferries settlers and lands armies beside the target city. Research gives a transport tech a bonus only while no port can train any transport (`research::tech_value`), so Astronomy gets none for the Galleon once Caravels exist. It sails unescorted (docs/AI.md, "Known gaps").

### Galley (`UNIT_GALLEY`)
*Naval, tier 2 · 50 hammers · 5/5, 3 moves, Sentry, cannot enter ocean · Sailing · every civilization*

**What it does.** The first warship: 5 strength, 3 moves, Sentry, limited to coast. It carries nothing. It upgrades to the Caravel, the Privateer or the Frigate.

**Playing it well.** Scout the coastline early to meet neighbours and find coastal sites, kill barbarian galleys, and guard work boats. Upgrade it to a Frigate once you have iron.

**AI: Partial.** It is the AI's first sea explorer: only the SeaExplore role takes it (`role_ok`), and its score per hammer (230 for 50) beats every other explorer while it can be trained, so the sea-explorer step (`production::decide`: our land full, nothing known overseas) builds a Galley. It reveals what it can reach without entering ocean (`naval::sea_explore`), and with nothing left it waits in port (`naval::port`), not being a transport, until gold upgrades it to its strongest upgrade (the Frigate with iron, `units::upkeep`). It never fights barbarian ships or guards the work boats (no naval combat logic).

### Man O'War (`UNIT_MAN_O_WAR`)
*Naval, tier 4 · 225 hammers · 13/13, 5 moves, bombards 30% a turn, carries 2 · Blasting Powder, Astronomy and Gunpowder · every civilization*

**What it does.** The ultimate warship: 13 strength, 5 moves, bombards coastal defenses by 30% a turn, two cargo slots. Frigates upgrade into it.

**Playing it well.** Rule the seas late in the game: sink everything afloat, bombard coastal cities to nothing in two or three turns before a landing, and escort the Galleons.

**AI: Incidental.** Production never prefers it (Transport score 415 for 225 hammers, 1.8 a hammer against the Galleon's 3.2; it lacks the sea-explorer AI types). It arrives when a Frigate upgrades (its 13 strength beats the Queen of the Line's 9 in `units::upkeep`) and keeps the Frigate's job, transport or explorer; it never fights, escorts or bombards.

### Pirate (`UNIT_PIRATE`)
*Naval, tier 3 · 100 hammers · 8/8, 4 moves, 20% withdrawal, carries 1 · Optics and Copper, Iron or Mithril · Lanun (replaces the Privateer)*

**What it does.** The Lanun privateer: Hidden Nationality (it can attack any ship and blockade any city without war, and can carry only hidden-nationality units), 20% withdrawal, and +3 gold for every victory. Declare Nationality ends the disguise.

**Playing it well.** Raid at peace: blockade rivals' coastal cities to cut trade routes and skim gold, sink their work boats and transports, and pocket 3 gold a kill. Keep a few hidden for the whole game; declare only to carry ordinary units.

**AI: Partial.** Production trains it only as a transport (`role_ok` Transport, 260 for 100 hammers, tied per hammer with the Trireme and Caravel; its pirate AI type is no other role). Trained `UNITAI_SETTLER_SEA` or `UNITAI_ASSAULT_SEA`, it is not covered by the rules' AI gate on Declare Nationality (only the pirate-ship AI is), so the end-of-turn choice casts it (`casting::ffh_value`, the ability's weight 50 less 1), after which it can carry the settler or soldier a hidden ship could not. Raiding, blockades and the gold from victories are never used (no naval combat or blockade missions).

### Privateer (`UNIT_PRIVATEER`)
*Naval, tier 3 · 100 hammers · 8/8, 4 moves, 20% withdrawal, carries 1 · Optics and Copper, Iron or Mithril · every civilization but the Lanun (Pirate)*

**What it does.** A ship with Hidden Nationality: it attacks any ship and blockades any city without a declaration of war, but carries only hidden-nationality units until it casts Declare Nationality. 20% withdrawal. Galleys and Triremes upgrade into it.

**Playing it well.** The peacetime raider: cut a rival's sea trade with blockades, sink its work boats and settler ships, all without war. Its cargo slot is for Council of Esus units (also hidden); for ordinary units use a Caravel.

**AI: Partial.** As the Pirate: trained only as a cheap transport (Transport score 260 for 100 hammers, tied with the Trireme and Caravel per hammer), and Declare Nationality is cast at the end of a turn (the rules' AI gate spares a ship with a transport AI type), which makes it an ordinary one-slot ship. Its hidden-nationality raiding is never used.

### Queen of the Line (`UNIT_QUEEN_OF_THE_LINE`)
*Naval, tier 4 · 225 hammers · 9/9, 3 moves, carries 6 · Astronomy, Engineering, a Shipyard and Iron or Mithril · every civilization but the Kuriotates (Airship)*

**What it does.** A ship of the line built to carry armies: six cargo slots and 9 strength. It needs a Shipyard in the city. Frigates upgrade into it.

**Playing it well.** The invasion ship: two Queens land a twelve-unit army in one wave, enough to take a defended coastal city the turn after landing. Build the Shipyard in your best port (it also gives sea units experience and builds them faster).

**AI: Unused.** Its Transport score per hammer (735 for 225) would just beat the Galleon's, but no AI city builds a Shipyard: the Shipyard's effects (sea experience, sea production) are fields the generic building value does not read, and `production::enabler_building` considers only land units. It is trained only where a Shipyard came with a captured city, and a Frigate's upgrade goes to the stronger Man O'War when Gunpowder is there and otherwise needs a Shipyard in the nearest coastal city (`unit_get_upgrade_city_for`).

### Trireme (`UNIT_TRIREME`)
*Naval, tier 2 · 75 hammers · 7/7, 2 moves, bombards 10% a turn, carries 1, cannot enter ocean · Sailing, Bronze Working and Copper · every civilization*

**What it does.** The first transport and the first ship that bombards: one cargo slot, 7 strength, coastal only, and it knocks 10% a turn off a coastal city's defenses. It upgrades to the Privateer or the Frigate.

**Playing it well.** Ferry a settler and its escort to a nearby island or along the coast long before Optics, and bring siege-less armies a way to batter coastal walls. Upgrade to the Frigate when iron arrives.

**AI: Directed.** The AI's first transport where a port has copper (`role_ok` Transport, 195 for 75 hammers, about tied per hammer with the Caravel and Privateer): it is built for a settler waiting in port and for assault fleets and run by `naval::transport`. `research::transport_step_tech` counts it only where a port has copper. It also passes SeaExplore but loses there to the Galley. Its bombard is never used, and coastal-only movement limits which overseas sites the transport flood (`naval::transport`) reaches. It upgrades to the Frigate by strength (`units::upkeep`).

### Work Boat (`UNIT_WORKBOAT`)
*Sea worker, tier 1 · 30 hammers · 2 moves, cannot enter ocean · Fishing · every civilization · consumed by its build*

**What it does.** Builds Fishing Boats (Fishing) or Whaling Boats (Sailing) on a water resource in the city's radius, and is consumed. For the Lanun it can also turn an empty coast plot of theirs into a Pirate Cove (+2 commerce, growing into a Pirate Harbor and a Pirate Port), with no cove within two plots.

**Playing it well.** One for each seafood resource, early: fish, clams and crabs feed coastal cities faster than any land tile. The Lanun build extra boats for coves on unused coast, ideally on plots their cities work.

**AI: Directed.** Production has a step for it: a city with an unimproved sea resource in its borders trains one, when unthreatened, while the work boats alive or in training are fewer than such resources across the empire and than our cities (`production::decide`, `sea_resources_unimproved`; `role_ok` SeaWorker). `workers::worker` sends it to the best build. The Lanun have their own logic (`civs::lanun`): a city of size 3+ working a valid cove plot trains a boat (one at a time), and a boat with no resource left in reach sails to the best cove site and casts Pirate Cove.

## Beasts

### Acheron the Red Dragon (`UNIT_ACHERON`)
*Beast, tier 4 · 19/19, 3 moves, 1 first strike · barbarian only · world unit, one at most*

**What it does.** The world's red dragon, built by a barbarian city: on its creation it is Held in place (it cannot move), the city gets the Dragon's Horde, and every forest and jungle around the city bursts into flames. It is a Dragon (Fear, immune to fear), Magic Immune, with Sentry I-II and Stoneskin, and it can Breathe Fire (summon a Meteor) and Roar (a fear attack on the units around it).

**Playing it well.** A guarded treasure: leave Acheron's city alone until you can field a stack built to kill it, ideally Grigori Dragon Slayers (+40% against dragons), fear-immune units (Courage, undead, golems) and siege to strip the city first, since magic does not touch it. The Dragon's Horde is the prize.

**AI: Not an AI choice.** Only a barbarian city builds it: the barbarians' fixed list (`cf_barb_city_production`, ported FfH Python) puts it first while it has never existed, unless the No Acheron option is set. As a barbarian unit it is a garrison by its default AI (`units::role_of`), held by the rules anyway; its spells are cast by FfH's terms because `magic.rs` is off for the barbarians (`magic::enabled`): Breathe Fire when an enemy is within its Meteor's reach (`casting::summon_value`), whose Meteor attacks only what is next to it at 5% or better (`casting::summons_attack`), and Roar for its `iAIWeight` of 30. No civilization's AI plans for it; an army attacks its city only when the barbarian-city operation or the hunt finds a force that covers its defenders (`campaign.rs`, `military::hunt`).

### Beast of Agares (`UNIT_BEAST_OF_AGARES`)
*Beast, tier 4 · 120 hammers · 11/11 +5 unholy, 2 moves, +25% against Paladins · Malevolent Designs, a Temple of the Veil, the Ashen Veil as state religion and in the city · every civilization · national unit, 4 at most*

**What it does.** The Veil's demon beast: 16 total strength and two moves for 120 hammers, a demon (immune to death, poison, unholy and fear), healed fully after it defeats a Paladin. The price is outside the hammers: the city that builds it loses 4 population (to 1 at least) and falls into 4 turns of disorder, unless it is an Infernal city, and each Beast raises the Armageddon counter by 1.

**Playing it well.** Build it only in a big, unimportant city or as the Infernals, who pay no population. Four Beasts are a fast, hard-hitting core against holy armies; aim them at Paladins and Angels, and keep in mind the counter if you do not want Armageddon.

**AI: Partial.** Defense and Attack roles (`production::role_ok`) by its 16 total strength at 120 hammers, which makes it one of the best scores `best_unit` can find for a Veil civilization; nothing there sees the 4 population, the 4 turns of disorder or the Armageddon point, so the AI may build it in any city that passes the requirements, its capital included. It fights as an army unit (`military::army`, `campaign.rs`).

## Heroes

### Abashi the Black Dragon (`UNIT_ABASHI`)
*Hero (beast), tier 4 · 360 hammers · 21/21, 3 moves, 1 first strike · Divine Essence · Sheaim · world unit · Dragon, Magic Immune, Sentry free*

**What it does.** The Sheaim dragon: 21 strength, 3 moves that ignore terrain and impassable plots, a first strike, immune to magic (and half resistant to cold, death, fire and lightning damage), +1 sight, but no defensive bonuses. Dragon gives fear (weaker units may refuse to attack it). Two abilities: Breathe Fire summons a Meteor (6 fire strength, flying, a suicide attacker with collateral damage 100 on up to 4 units, to at most 60%, and 15% bombard), and Roar is a fear attack on every enemy unit next to it (resistable; units that fail flee).

**Playing it well.** A one-unit army from the late game: breathe fire on a stack or a city before it attacks, then kill the weakened defender; roar to scatter what stands next to it. Being immune to magic it ignores the enemy's spells, so only strong melee and Dragon Slayers threaten it. Do not leave it alone on open ground where several attackers can reach it.

**AI: Valued.** The hero step of `production::decide` (step 7, ahead of the army and the buildings) trains it as soon as a city can, by strength per hammer (`production::best_unit`, role Hero), and research gets 1500 for Divine Essence once it completes the Sheaim's unique unit (`civs::unique_unit_tech_value`). It fights as an army unit (`military::army`, the operations' forces in `campaign.rs` count heroes). Breathe Fire is an ability `magic.rs` leaves to FfH's terms: at the end of the turn it is worth the Meteor's 600 when an enemy is within the Meteor's reach (`casting::ffh_value`), and the Meteor attacks at once (`casting::summons_attack`); before a strike `magic::cast_phases` would also value the Meteor's fight, but keeps the cast for the end of the turn whenever FfH's 600 is higher. Roar (`iAIWeight` 30) loses to Breathe Fire whenever both can be cast.

### Adventurer (`UNIT_ADVENTURER`)
*Hero, tier 2 · not trainable · 2/2, 1 move, defends only · Grigori (great people of their Adventurers' Guild, palace and tavern; a lair goody) · Hero free; upgrades to Adept, Warrior or Scout*

**What it does.** The Grigori's great person: a weak, defend-only unit with the Hero promotion, so it gains a point of experience every turn (up to 100) and resists magic. Having no unit combat type it can take almost no promotions, and an upgrade cuts experience to 10 (`MAX_EXPERIENCE_AFTER_UPGRADE`), so its value is what it becomes: an Adept, a Warrior or a Scout, and through them any unit down those lines (the rules search the whole upgrade chain), keeping the Hero promotion and its free experience from then on.

**Playing it well.** Upgrade it as soon as you can afford it into the best melee, recon or arcane unit you can train: experience it gathers as an Adventurer beyond 10 is lost at the upgrade, while a hero-promoted Champion, Ranger or Adept keeps gaining a point a turn and levels fast. It is the Grigori's answer to having no religious heroes.

**AI: Not an AI choice.** The Grigori get it from great people points and lairs. With its default `UNITAI_ATTACK` it is an army unit (`units::role_of`) that cannot attack, so it rides in stacks until `units::upkeep` upgrades it, which happens as soon as gold allows (the right timing): but the AI looks only at its direct upgrades (Adept, Warrior, Scout) and takes the strongest by attack strength, then upgrades the result one class a turn, rather than jumping to the best unit of the chain the rules would allow (the Adept and the Warrior tie at 3; the first class wins).

### Alazkan the Assassin (`UNIT_ALAZKAN`)
*Hero (recon), tier 3 · 120 hammers · 6/4 + 2 poison, 2 moves, 1 first strike, -50% city attack and defense · Poisons · Svartalfar · world unit · Black Mirror, Elven, Hero, Hidden Nationality, Marksman free*

**What it does.** The Svartalfar assassin: a fast, poisonous recon hero that is immune to first strikes and defensive strikes, targets the weakest unit of a stack (Marksman) and carries no nationality (Hidden Nationality: he can attack anyone without a war and anyone can attack him). Declare Nationality ends the disguise. His Black Mirror lets him cast Mirror (not while an enemy is on his plot and not if he is immune to magic): an illusory copy of himself, with his promotions (not the mirror) and damage, that lasts one turn and, as an illusion, cannot kill. As a Svartalfar recon unit he could also Kidnap great people (`bAllowAI` 0).

**Playing it well.** An assassin of rival stacks' weakest units and of lone workers and settlers, at war or, disguised, at peace. Before attacking a stack, cast Mirror: the copy attacks first and wounds the defender, then Alazkan finishes it. Keep him out of cities, where his -50% is crippling.

**AI: Partial.** The hero step trains him (`production::best_unit`, role Hero) and research gets 1500 for Poisons (`civs::unique_unit_tech_value`); he then fights as an army unit. As the head of a group he may attack other civilizations' units next to him even at peace (the rules count every other team as hostile to a hidden-nationality unit, and `military::best_adjacent` asks them), but nothing plans such raids. At the end of every turn he casts Mirror by FfH's weight of 50 when he can (`casting::ffh_value`: `magic.rs` sees no effect in it), so a copy stands in his stack through the enemies' turn and vanishes before it could attack; it is never cast before an attack, where it would help (`magic::cast_phases` values it at 0). On a turn Mirror cannot be cast he declares his nationality (FfH's weight of 50 less the small value of the lost promotion).

### Arthendain (`UNIT_ARTHENDAIN`)
*Hero (archery), tier 4 · 180 hammers · 9/9, 1 move, 1 first strike · Medicine, the Runes of Kilmorph as state religion · every civilization · world unit; leaves a civilization without the religion · Divine, Dwarven, Hero, Immune to Disease, Life II, Medic I-II free*

**What it does.** Kilmorph's healer-archer: 9 strength, a first strike, a 40% chance of a defensive strike (+20% damage) on units attacking his stack, immune to disease, Medic II for his plot, and two spells: Cure Disease (his plot) and Destroy Undead (30 holy damage on every undead unit within 1, no limit; an AI casts it only on undead of a team it is at war with).

**Playing it well.** The anchor of a Kilmorph defense and the counter to undead armies (Calabim skeletons, Sheaim and Ashen Veil necromancy): stand him on a hill with the army, let the defensive strikes and his first strike punish attackers, and burn undead stacks next to him.

**AI: Valued.** The hero step trains him once the Runes are the state religion and Medicine is known, and he fights as an army unit. Destroy Undead is one of the Python spells `magic.rs` models (`Py::DestroyUndead`), cast before strikes and at the end of the turn on undead enemies within 1; Cure Disease when his plot holds a diseased unit. The defensive strike is not counted in his power.

### Auric Ascended (`UNIT_AURIC_ASCENDED`)
*Hero (avatar), tier 4 · not trainable · 30/30 + 30 cold, 3 moves, flying · the Illians' Ascension ritual · world unit · Avatar, Flying, Sentry I-II free*

**What it does.** The Illian god-king: 60 effective strength, flying, sees invisible units, immune to magic, fear and capture, and Snowfall (range 2: 80 cold damage, to 100%, on every non-team unit, and the land around turns to ice for a while). The Ascension ritual (Omniscience, 1200 hammers, after anyone's The Draw) creates him and hands the Godslayer to the strongest rival; if he loses a fight the Illians lose their leader traits, and fighting the Godslayer's bearer kills him even when he wins.

**Playing it well.** The Illians' victory weapon: Snowfall wipes stacks and the god-king takes cities alone. The one counter is the Godslayer; know where it went and kill its bearer first.

**AI: Not an AI choice.** No AI can reach him: the Ascension needs The Draw, which `production::best_project` skips as a ritual for dedicated logic and no civilization logic builds (`civs/illians.rs` builds only The White Hand and Stir from Slumber). If the rules ever gave him to an AI he would be an army unit, and Snowfall is damage `magic.rs` values.

### Bambur (`UNIT_BAMBUR`)
*Hero (melee), tier 3 · 180 hammers · 5/5, 1 move · Arete, the Runes of Kilmorph as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I, Dwarven, Enchanted Blade, Enchantment I, Hero free*

**What it does.** Kilmorph's weaponsmith: a modest fighter that already carries an Enchanted Blade (+20% attack and defense) and casts Enchanted Blade on every melee unit of his stack (lasting, until dispelled; it also removes Rusted), plus Repair (20 hit points back to the golems, ships and siege engines on his plot). He uses bronze, iron and mithril weapons.

**Playing it well.** March him with the melee stack: one cast gives every axeman and champion a lasting +20%. Pair him with siege engines or golems and repair them between assaults.

**AI: Valued.** The hero step trains him and he fights as an army unit. Enchanted Blade is a lasting blessing `magic.rs` values on every melee unit that lacks it (`magic::ours_worth`; the stack's attackers before a strike), so he blesses the stacks he travels with. Repair is never cast: it is an ability `magic.rs` does not model and FfH's terms give it 0 (`casting::ffh_value`).

### Barnaxus (`UNIT_BARNAXUS`)
*Hero (golem), tier 2 · 180 hammers · 5/5, 1 move · Construction · Luchuirp · world unit · Golem, Pieces of Barnaxus free*

**What it does.** The Luchuirp's golem hero: every Combat I-V promotion he holds gives every golem of his owner the matching Empower I-V (`cf_do_turn_luchuirp`), so the whole golem army grows with him. As a golem he heals slowly (less in friendly and neutral land), is fearless and immune to death and poison. When he dies his pieces drop as equipment; a unit that carries them into a Luchuirp city can cast Rebuild Barnaxus there to raise him again (at 75% damage; a non-Luchuirp rebuilder earns the Luchuirp's gratitude).

**Playing it well.** The Luchuirp's most important unit: promote him straight down the Combat line, keep him out of risky fights (he levels from safe kills and his Hero experience), and recover his pieces at once if he falls.

**AI: Partial.** The hero step trains him and research gets 1500 for Construction (`civs::unique_unit_tech_value`); he fights as an army unit like any other, attacking what he can beat. His promotions are the generic choice (`units::promotion_value`), which likes Combat for its strength but does not know it powers the golems. If he dies he is gone: the AI never casts Rebuild Barnaxus (FfH's terms count only the lost Pieces promotion, a negative value, `casting::ffh_value`), and nothing sends a unit to pick the pieces up for that purpose.

### Baron Duin Halfmorn (`UNIT_DUIN`)
*Hero (beast), tier 3 · 500 hammers · 11/11, 3 moves · Feral Bond · every civilization (not with the No Duin option) · world unit · Magic Resistance, Werewolf free*

**What it does.** A werewolf lord: 11 strength, 3 moves, 20% magic resistance (and 20% against cold, death, fire and lightning), and after each fight he wins a 75% chance that a Ravenous Werewolf joins him. He has no Hero promotion (no free experience).

**Playing it well.** Hunt with him: every living unit he kills may become a werewolf, so feed him weak, isolated units (barbarians, workers, scouts) and build a pack. He is expensive; do not trade him into a fortified city.

**AI: Valued.** The hero step trains him (his `UNITAI_HERO`), and once the empire has three cities it starts him whatever the 500 hammers' turns (`production::decide`), and he fights as an army unit that hunts what it can beat (`military::army`), which feeds the werewolf chance. Nothing aims his fights at living, weak targets for the sake of the pack.

### Basium (`UNIT_BASIUM`)
*Hero (angel), tier 4 · 240 hammers · 7/7 + 4 holy, 2 moves · Iron Working · Mercurians · world unit · Angel, Hero, Loyalty, Sentry free; +5 to the Armageddon counter*

**What it does.** The Mercurian leader in person: an angel (immune to fear, holy damage and poison, half resistant to death) of 11 effective strength, immune to capture (Loyalty), using bronze to mithril weapons. When a civilization completes the Mercurian Gate, the Mercurians appear on its team with Basium, six Angels, a Settler and two Workers in the gate city, and Basium's Convert City takes that city for the Mercurians (+300 culture). Divine Retribution (world spell) deals 50 holy damage to every demon and undead unit in the world. If he dies for good the Mercurians lose their leader traits; Sacrifice (on a Pyre of the Seraphic: research, `bAllowAI` 0).

**Playing it well.** As the Mercurians, take the gate city at once, then use Basium and his angels against the Infernals and the undead; save Divine Retribution for when the enemy's demons and undead outnumber yours (it hits everyone).

**AI: Not an AI choice.** The rules create him (`ffh/events.rs`, the Mercurian Gate). Convert City has an `iAIWeight` of 1000 and FfH's terms cast it at the end of a turn he spends in the gate city (`casting::ffh_value`; docs/AI.md: Basium converts the gate city); nothing else in the AI names him. Divine Retribution is cast by FfH's weight (300) once the AI timing allows, at war with two teams (`world_spells::ready`). He fights as an army unit.

### Brigit (`UNIT_BRIGIT`)
*Hero (angel), tier 4 · not trainable · 6/6 + 3 holy, 1 move · freed from the Ring of Carcer · world unit · Angel, Hero free; resurrected in the capital if killed*

**What it does.** An angel hero held at the Ring of Carcer: whoever frees her (by taking Brigit Held there) gets her: 9 effective strength, +1 Fire affinity, the angels' resistances, bronze to mithril weapons, and immortality (she comes back in her owner's capital when killed). Sacrifice is `bAllowAI` 0.

**Playing it well.** Free her as soon as you have a unit of level 15 (the Ring admits no lower one): an immortal hero is a permanent front-line asset that can be thrown at the hardest defender every turn.

**AI: Not an AI choice.** Only a level-15 unit can enter the Ring (`unit_can_move_into`), and nothing in the AI aims for it. If an AI did free her she would be an army unit (`UNITAI_HERO`, `military::army`), whose resurrection the AI does not exploit.

### Brigit (`UNIT_BRIGIT_HELD`)
*Special, tier 4 · not trainable · 0/0, immobile · created by the Ring of Carcer for the barbarians · world unit · Angel free*

**What it does.** The captive Brigit: an immobile, strengthless barbarian unit on the Ring of Carcer (whose plot only units of level 15 or more may enter). Taking her captures her as the free Brigit (`Capture` Brigit) and removes the Ring.

**Playing it well.** Bring a level-15 unit to the Ring and walk in.

**AI: Not an AI choice.** The barbarians hold her; no AI unit is sent to the Ring, and the AI rarely has a level-15 unit.

### Chalid Astrakein (`UNIT_CHALID`)
*Hero (disciple), tier 3 · 300 hammers · 7/7, 2 moves · Religious Law, the Empyrean as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I-III, Divine, Hero, Law I, Sentry, Sun I free; +2 Sun affinity, +1 Overcouncil vote*

**What it does.** The Empyrean's prophet and an archmage of Sun and Law: Pillar of Fire (his own: 50 fire damage, to at most 75%, on every unit of the plot within 2 that holds the most enemies and no one else), Crown of Brilliance (30 holy damage on enemies within 1, repeated while the crown lasts), Revelation (unmasks hidden and illusory enemies within 3), Loyalty (Law I: our units in his stack cannot be captured) and Scorch (Sun I: a plains plot to desert or a snow plot to tundra; an AI only scorches others' plains and its own snow), and he learns further Sun, Law and other spheres.

**Playing it well.** A siege caster that fights: pillar the strongest stack in front of the army each turn, crown when enemies close in, reveal assassins and illusions. His Overcouncil vote matters in a diplomatic game.

**AI: Valued.** The hero step trains him once the Empyrean is the state religion, and he fights as an army unit, so he is near the strikes; every caster within two plots of a target casts first (`magic::before_attack`). Pillar of Fire is modelled by `magic.rs` (`Py::PillarOfFire`), Crown of Brilliance as damage, Revelation as a dispel, Loyalty as a small lasting blessing, Scorch as terraforming of our own snow (`magic::terraform`); his promotions take the spheres by the spells they open (`magic::promotion_value`). Being an army unit he also attacks with his 7 strength, at the risk of the hero (the rules want 85% odds before a group picks a hero to attack, `AI_getBestGroupAttacker`).

### Corlindale (`UNIT_CORLINDALE`)
*Hero (arcane), tier 2 · 300 hammers · 0 strength, 2 moves · Fanaticism · Elohim · world unit · Channeling I-III, Earth I, Hero, Mind I, Spirit I free*

**What it does.** The Elohim sage: no combat strength at all, but Channeling III and three spheres from the start, and he learns more (passive experience to 90, plus the Hero's point a turn). His spells: Courage (Spirit I: fear immunity and +10% healing for the living units of his stack), Inspiration (Mind I: +2 research and +1 Sage point in his city while he stays), Wall of Stone (Earth I: +25% defense in his city while he stays) and Peace (his own: he sacrifices himself to make peace with every civilization at war with us, and the Armageddon counter is halved).

**Playing it well.** A superb mage kept safe in the middle of a stack, or a city's sage. Peace is the Elohim's emergency exit from a losing war, and halving the Armageddon counter can matter more; it costs the hero, so hold it for the moment it saves a city or the world.

**AI: Partial.** Production trains him because of a rule written for him: a hero of strength 0 passes the Hero role when it is born a caster (`production::born_caster`, Channeling I free), and a hero that cannot attack moves as a caster (`units::role_of`), with the stack nearest an operation's target (`magic::caster_target`: Courage makes him a combat caster). Docs/AI.md: Corlindale trained in 0% -> 67% of Elohim games after the rule. His spheres are taken by the spells they open; Courage is valued as a lasting blessing, Inspiration and Wall of Stone as a quarter of their building's value in the city he is in, and a caster keeping one stays (`magic::upkeep_value`). Peace is never cast: FfH's terms charge the sacrifice (level x his worth, `casting::ffh_value`), and nothing in the AI considers it as a way out of a war.

### Donal Lugh (`UNIT_DONAL`)
*Hero (melee), tier 3 · 300 hammers · 7/7, 2 moves · Fanaticism · Bannor · world unit · Empty Bier, Guardsman, Hero, Recruiter, Spirit II free*

**What it does.** The Bannor hero: Recruit (once, in one of our cities: population / 3 + 1 random units of the classes our techs open, from scouts to champions and longbowmen, twice as many with a Temple of the Order there); he regains it each time he defeats a demon or an undead. The Empty Bier (equipment) gives +2 defense and doubles his fortification bonus; Guardsman makes him defend his stack first; Spirit II gives Hope (a city building, +4 culture, +1 happiness and Courage for units in the city, while he stays).

**Playing it well.** Recruit in your biggest city with a Temple of the Order for a free army, then take him to war against the Infernals or the undead to earn the next Recruit. He is a sturdy stack defender with the bier.

**AI: Directed.** The hero step trains him; `civs/bannor.rs` casts Recruit at the start of our turn in the first city of ours giving two or more recruits, or as many as our best (`magic::recruit_value`; FfH's terms gave it nothing and none of 20 Donals recruited), so every Donal recruits once (9 of 9 on the census seeds, docs/AI.md). Afterwards he is an army unit; regaining Recruit through kills is incidental (nothing hunts demons for it), and Hope is valued as a city spell when he stands in a city.

### Drifa the White Dragon (`UNIT_DRIFA`)
*Hero (beast), tier 4 · not trainable · 14/14 + 7 cold, 3 moves, 1 first strike · the Illians' Stir from Slumber ritual · world unit · Dragon, Immune to Cold, Magic Immune, Sentry, Vulnerable to Fire free*

**What it does.** The Illians' dragon, woken by Stir from Slumber (Divine Essence, 900 hammers, only after the Illians have eliminated a player): 21 effective strength, the dragons' movement, first strike, fear and magic immunity, +10% on tundra and +50% on ice, immune to cold but taking 50% more fire damage. Breathe Fire and Roar as the other dragons.

**Playing it well.** An Illian spearhead in their snowy lands; keep her away from fire casters and fire-damage units.

**AI: Valued.** The Illians build Stir from Slumber in their most productive city as soon as the rules allow (`civs/illians.rs`); the dragon is then an army unit (`UNITAI_HERO`). Breathe Fire and Roar as for Abashi: FfH's terms, the Meteor attacking at once.

### Eurabatres the Gold Dragon (`UNIT_EURABATRES`)
*Hero (beast), tier 4 · 360 hammers · 23/23, 3 moves, 1 first strike · Divine Essence · Kuriotates · world unit · Dragon, Magic Immune, Sentry free*

**What it does.** The strongest dragon, 23 strength, with the dragons' movement, first strike, fear and magic immunity, Breathe Fire and Roar.

**Playing it well.** As Abashi: the Kuriotates' late-game hammer; let him take the hardest defender and breathe fire on stacks.

**AI: Valued.** As Abashi: the hero step trains him and research gets 1500 for Divine Essence (`civs::unique_unit_tech_value`); an army unit, Breathe Fire by FfH's terms. Docs/AI.md lists him among the unique units the base AI never used in 40 games; Divine Essence is rarely reached before games end.

### Gaelan (`UNIT_GAELAN`)
*Hero (arcane), tier 3 · not trainable · 6/4, 1 move, work rate 300 · the Circle of Gaelan event · world unit · Channeling I-II, Chaos I, Hero free; upgrades to the Archmage*

**What it does.** A rebellious apprentice who joins through the Circle of Gaelan event chain (a city with a Mage Guild): a mage-level caster with Chaos I (Dance of Blades: +1 first strike for his stack's units until our next turn), the Hero's free experience and the casters' passive experience (to 50), and an upgrade to the Archmage at level 6.

**Playing it well.** Take him when the event offers him: a free Mage that grows into an Archmage. Build nodes with him and keep him in a stack.

**AI: Not an AI choice.** The event hands him over when the AI picks that answer (`events::choose` values a free unit of cost -1 like a great person, 600). His default `UNITAI_RESERVE` makes him an army unit (`units::role_of`), not a caster: he merges into stacks and may attack with his 6 strength. His spheres are chosen by the spells they open (`magic::promotion_value`) and he casts by `magic.rs` (Dance of Blades as a short blessing, by the defense it adds through the enemies' turn), and `units::upkeep` makes him an Archmage at level 6 when gold allows.

### Gibbon Goetia (`UNIT_GIBBON`)
*Hero (arcane), tier 3 · 180 hammers · 5/5, 1 move · Deception, the Council of Esus as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I-III, Hero, Illusionist free*

**What it does.** The Esus hero: an archmage-level caster from the start whose summons are illusions (Illusionist: they cannot kill but heal fully after each fight), able to enter rival territory without open borders, and Impersonate Leader: in another (non-human) civilization's city he sacrifices himself and his owner takes control of that civilization for 5-14 turns (`bAllowAI` 0).

**Playing it well.** A spy-mage: use the rival-territory movement to scout and wear down enemies with illusory summons, and keep Impersonate Leader for a decisive moment (dismantling a rival's army or treasury while you hold it).

**AI: Partial.** The hero step trains him once Esus is the state religion; with his 5 strength and `UNITAI_HERO` he is an army unit, not a caster (`units::role_of`), so he fights and travels with stacks. His spheres are chosen by the spells they open and cast by `magic.rs`; the AI does not count that his summons cannot kill (`magic::summon_fight`). Impersonate Leader is refused to AIs, and his rival-territory movement is not used.

### Gilden Silveric (`UNIT_GILDEN`)
*Hero (archery), tier 2 · 120 hammers · 5/6, 1 move, 1 first strike, +25% city defense · Archery · Ljosalfar · world unit · Dexterous, Elven, Hero free*

**What it does.** The Ljosalfar's early hero: an archer of 6 defense (+1 strength from Dexterous), a first strike, +25% in cities and on hills, a 30% chance of a defensive strike (+15% damage) on attackers of his stack, and the elves' double moves in forests and +10% attack there.

**Playing it well.** One of the earliest heroes: hold the capital or a border hill fort with him against early rushes, then take him on offensives in forest country, where he moves and fights best. He gains a level's worth of experience every few turns from the Hero promotion.

**AI: Partial.** The hero step trains him (role Hero, scored by attack strength) and research gets 1500 for Archery (`civs::unique_unit_tech_value`); trained as `UNITAI_HERO` he is a field army unit (`military::army`), although his data (`CITY_DEFENSE`, `CITY_COUNTER`, +25% city defense) makes him a defender. He garrisons only when he happens to stand in an undefended city of ours or defends a city in peril with the stack.

### Govannon (`UNIT_GOVANNON`)
*Hero (arcane), tier 4 · 480 hammers · 5/5, 1 move, work rate 400 · Arcane Lore · Amurites · world unit · Channeling I-III, Hero free*

**What it does.** The Amurite archmage-teacher: Channeling III and the Hero's experience, so he learns any sphere up to its third tier, and Teach Spellcasting: every living unit of ours on his plot (animals and birds aside) gets every first-tier sphere promotion Govannon has, and Channeling I, so it can cast those spells itself.

**Playing it well.** Learn the first tier of the spheres your army needs (Body for Haste, Enchantment for Enchanted Blade, Death for skeletons, Chaos for Dance of Blades), then teach a whole stack of soldiers: every unit becomes a caster of those spells, a huge force multiplier. Keep him safe; he is expensive and irreplaceable.

**AI: Partial.** The hero step trains him (480 hammers, started anyway once we have three cities) and research gets 1500 for Arcane Lore (`civs::unique_unit_tech_value`). With 5 strength he is an army unit (`units::role_of`), so he fights. His spheres are chosen by the spells they open and cast by `magic.rs`. Teach Spellcasting is never cast: its effect is Python `magic.rs` does not model and FfH's terms give it 0 (`casting::ffh_value`); docs/AI.md lists it as a known gap.

### Guybrush Threepwood (`UNIT_GUYBRUSH`)
*Hero (melee), tier 3 · 360 hammers · 7/7, 1 move · Iron Working · Lanun · world unit · Boarding, Hero free*

**What it does.** The Lanun pirate hero: a 7-strength swordsman with Boarding (from a ship he can attack naval units and capture them) and bronze to mithril weapons. Sing does nothing (`bAllowAI` 0).

**Playing it well.** Carry him on a ship and board enemy vessels to steal their navy; on land he is a solid melee hero for raids on coastal cities.

**AI: Partial.** The hero step trains him and research gets 1500 for Iron Working (`civs::unique_unit_tech_value`); he is a land army unit. Boarding is never used: the AI has no naval combat logic and never fights from a ship (`units::play`, docs/ENTITIES.md "Units: in the field").

### Hemah (`UNIT_HEMAH`)
*Hero (arcane), tier 4 · 180 hammers · 7/7, 1 move, work rate 200 · Arcane Lore, the Octopus Overlords as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I-III, Hero free*

**What it does.** The Overlords' archmage: third-tier spheres long before Strength of Will, the Hero's experience, and Hastur's Razor (every unit within 1, ours and theirs, gets a damage value drawn at random from the damage of all those units; an AI casts it only when one of its own units in range is hurt).

**Playing it well.** Learn the strongest spheres you have mana for and use him as an early archmage. Hastur's Razor is a gamble that pays when a wounded stack of yours stands next to a healthy enemy one: the wounds are shared out.

**AI: Partial.** The hero step trains him once the Overlords are the state religion; with 7 strength he is an army unit, not a caster (`units::role_of`). His spheres are chosen by the spells they open (`magic::promotion_value`) and cast by `magic.rs`. Hastur's Razor is cast by its flat `iAIWeight` of 30 whenever one of our units next to him is hurt and nothing is worth more (`casting::ffh_value`), without looking at whose damage it shares out.

### Herne (`UNIT_HERNE`)
*Hero (mounted), tier 4 · 240 hammers · 11/7, 3 moves, 25% withdrawal · Warhorses · Kuriotates · world unit · Blitz, Centaur, Hero free*

**What it does.** The Kuriotates' centaur hero: 11 attack, 3 moves, Blitz (several attacks a turn), withdrawal, no defensive bonuses, bronze to mithril weapons. Sprint (Hasted and then Fatigued) is `bAllowAI` 0.

**Playing it well.** A raider and finisher: attack two or three wounded units a turn with Blitz, withdraw from bad fights, and never leave him exposed where his 7 defense without terrain bonuses will be attacked.

**AI: Valued.** The hero step trains him, scored with his moves and withdrawal (`production::best_unit`), and research gets 1500 for Warhorses; he fights as an army unit. Blitz is not exploited: a campaign strike attacking again with Blitz units was tried and measured flat (`civs/hippus.rs`). Docs/AI.md notes research never plans a path to Warhorses for him.

### Hyborem (`UNIT_HYBOREM`)
*Hero (demon), tier 4 · 100 hammers · 7/7 + 2 unholy, 2 moves · the Infernals' founding · Infernal · world unit · Demon, Gela, Hero, Loyalty, Sentry free (and Immortal at the start); +5 to the Armageddon counter*

**What it does.** The Infernal leader in person, created with the Infernals when the first team learns Infernal Pact (immortal once: he returns to the capital the first time he dies). Gela gives +2 unholy damage; Loyalty makes him immune to capture. Hyborem's Whisper (world spell, Malevolent Designs): an event offers the three most valuable Ashen Veil cities of other civilizations (not capitals), and the chosen one becomes the Infernals'. If he dies for good the Infernals lose their leader traits except Fallow.

**Playing it well.** Keep him alive (traits ride on him) and use the Whisper as soon as Malevolent Designs comes, on the richest Ashen Veil city in reach of your armies.

**AI: Not an AI choice.** The rules create him (`ffh/events.rs`). He is an army unit (`UNITAI_HERO`) with no care for his traits. Hyborem's Whisper is cast by FfH's weight of 300 as soon as it can be (the AI timing has no condition for it, `world_spells::ready`), and the event's city is chosen by `events::choose`; docs/AI.md notes Malevolent Designs was never reached in the measured games.

### Kithra Kyriel (`UNIT_KITHRA`)
*Hero (mounted), tier 3 · 180 hammers · 8/8, 3 moves, 1 first strike, 35% withdrawal · Feral Bond, the Fellowship of Leaves as state religion · every civilization · world unit; leaves a civilization without the religion · Elven, Hero free*

**What it does.** The Fellowship's horse-archer hero: 8 strength, 3 moves, a first strike, immune to first strikes, a 35% withdrawal chance, sees stealthy units, the elves' forest movement, no defensive bonuses.

**Playing it well.** A fast striker and hunter of hidden units (assassins, shadows); pick off wounded stragglers and withdraw.

**AI: Valued.** The hero step trains him once the Fellowship is the state religion, scored with moves, first strike and withdrawal; he fights as an army unit. His stealth detection and elven movement are not used on purpose.

### Loki (`UNIT_LOKI`)
*Hero, tier 1 · 180 hammers · 2/2, 2 moves, defends only, 100% defensive withdrawal · no tech · Balseraphs · world unit · Channeling I-II, Chaos I, Spell Extension I, Magic Immune, Mind I free*

**What it does.** The Balseraph trickster: he enters rival territory, investigates cities, cannot attack, and escapes from almost any fight (he withdraws when attacked while an open plot remains, so he must be surrounded to die). His spells: Disrupt (in another team's city: two turns of hurry anger and 0-2 of its owner's culture there lost; a city left with none becomes his owner's without war, its owner's attitude -4), Entertain (in a city: two turns of extra happiness, and in a foreign city he takes population / 2 + 1 gold from its owner), Create Puppet (a temporary 2-strength puppet that copies his promotions, so it can cast his Dance of Blades), Dance of Blades and Inspiration.

**Playing it well.** Walk him into a neighbour's new or low-culture border city at peace and Disrupt every turn until it flips: cities for free, without a war. Entertain in rivals' big cities for gold. Keep him out of encirclement.

**AI: Directed.** The hero step trains him; `civs/balseraphs.rs` walks him at peace to the nearest reachable city of a civilization we are not at war with whose owner culture is below 20 and does not grow, and casts Disrupt there every turn; otherwise he moves as a caster (`units::role_of`: a hero that cannot attack) with the stacks, and casts Create Puppet every turn by its `iAIWeight` of 1000 (`magic::turn_value`). Docs/AI.md measured it: 1-10 cities a game (about 3), the Balseraphs' fresh-seed win rate 9.4% -> 23.4%, and Lokis killed 78% -> 61% of the time once he moved as a caster. Entertain (weight 50), Inspiration and Dance of Blades lose to the puppet's 1000 at the end of every turn he has not cast Disrupt, so they are practically never cast, and Entertain's gold is not valued.

### Losha Valas (`UNIT_LOSHA`)
*Hero (melee), tier 3 · 300 hammers · 7/7, 1 move · Fanaticism · Calabim · world unit · Body I, Channeling I-II, Death I, Hero, Vampirism free*

**What it does.** The Calabim's vampire hero: Vampirism (+10% attack and defense, +5% healing everywhere) and after each living unit she defeats she gains the Immortal promotion (she returns to the capital when killed). Spells: Feast (in our city: a citizen lost, experience for her), Feed (with a Bloodpet in the stack: it is sacrificed to heal her and let her attack again), Gift Vampirism (the Vampire promotion for every level-6+ unit on her plot), Haste (Body I: +1 move to her stack until our next turn) and Raise Skeleton (Death I: a permanent 2-strength skeleton).

**Playing it well.** Keep her killing living units so she is always immortal, feast in cities above their happiness cap to level her, gift vampirism to your veterans, and raise skeletons as siege fodder.

**AI: Directed.** The hero step trains her and research gets 1500 for Fanaticism; she fights as an army unit. `civs/calabim.rs` values her Feast (in a city with a citizen above its happy cap, while she has under 30 experience) and Gift Vampirism (the Vampire promotion's worth for each level-6 unit on her plot), which FfH's terms never cast; Raise Skeleton is a lasting summon `magic.rs` casts at war (skeletons go first in strikes, `magic::raised`), Haste is worth nothing at the end of a quiet turn, and Feed is cast by its weight of 10 when she is hurt 20 or more with a Bloodpet in the stack (the rules' AI gate). Nothing chooses living targets to renew her immortality.

### Lucian (`UNIT_LUCIAN`)
*Hero (melee), tier 2 · 60 hammers · 4/4, 1 move, +10% city attack · no tech · Doviello · world unit · Winterborn free*

**What it does.** A cheap early Doviello champion: an axeman's strength with +10% city attack, Winterborn (+10% on snow and tundra), bronze to mithril weapons, and no Hero promotion.

**Playing it well.** An early rush or escort unit; he is a world unit only in name, so spend him freely.

**AI: Valued.** His `UNITAI_HERO` makes him a hero for production (`production::is_hero`), so the hero step trains him for 60 hammers as soon as a city can, and he fights as an army unit.

### Magnadine (`UNIT_MAGNADINE`)
*Hero (mounted), tier 3 · 360 hammers · 11/11, 4 moves, 50% withdrawal · Warhorses, Horse or Nightmare · Hippus · world unit · Commando, Courage, Hero free*

**What it does.** The Hippus warlord: 11 strength, 4 moves, a 50% withdrawal chance, Commando (uses enemy roads, a chance of a first strike), Courage, no defensive bonuses, bronze to mithril weapons. Hire Units (in our cities: an event to hire units, `bAllowAI` 0).

**Playing it well.** The best raider of his era: strike deep, withdraw from bad odds, and finish wounded stacks. Hire mercenaries at the front when the war needs numbers fast.

**AI: Valued.** The hero step trains him, scored with his moves and withdrawal, and research gets 1500 for Warhorses (`civs::unique_unit_tech_value`); he fights as an army unit. Hire Units is refused to AIs; Commando's road use is incidental.

### Mardero (`UNIT_MARDERO`)
*Hero (disciple), tier 4 · 240 hammers · 10/10 + 4 unholy, 2 moves · Malevolent Designs, the Ashen Veil as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I-III, Demon, Entropy I, Hero, March, Stigmata free; +3 to the Armageddon counter*

**What it does.** The Veil's champion: 14 effective strength, +40% against disciple units, a demon's resistances, March (heals every turn wherever he is), Stigmata (+1% strength for every 2 points of the Armageddon counter) and Rust (Entropy I: range 1, resistable: enemy melee units lose their bronze, iron and mithril weapons and get Rusted, -10%).

**Playing it well.** Lead the Veil's armies against the Order and its Paladins, rusting their weapons before the fight; he never needs to stop to heal.

**AI: Valued.** The hero step trains him once the Veil is the state religion; he fights as an army unit. Rust is valued by `magic.rs` as the weapons it takes from defenders (before strikes) and attackers in reach (at the end of the turn), and his promotions take Entropy further by the spells it opens.

### Maros (`UNIT_MAROS`)
*Hero (melee), tier 4 · 180 hammers · 6/12, 1 move, +50% city defense · Iron Working · Khazad · world unit · Dwarven, Hero free*

**What it does.** The Khazad's stone wall: 12 defense with +50% in cities, 6 attack, the dwarves' hill movement, bronze to mithril weapons.

**Playing it well.** Fortify him in the capital or the most threatened border city; promoted with city defense he holds almost anything of his era.

**AI: Partial.** The hero step trains him, but it scores heroes by attack strength (6), and trained as `UNITAI_HERO` he is a field army unit (`military::army`), not a garrison: the AI uses its best city defender as a slow attacker. He defends a city only when he happens to stand in an undefended city of ours (he then becomes its garrison) or the stack defends a city in peril.

### Meshabber of Dis (`UNIT_MESHABBER`)
*Hero (demon), tier 4 · 1200 hammers · 19/19 + 6 fire, 6 unholy, 2 moves · Infernal Pact, the Ashen Veil as state religion, the Veil's holy city, an Armageddon counter of 70 · every civilization · world unit; leaves a civilization without the religion · Demon, Divine, Fire II, Immune to Fire, Sentry free; +3 to the Armageddon counter*

**What it does.** A great demon of 31 effective strength, immune to fire, with Fire II: he casts Fireball (a temporary 4-strength flying fire attacker with collateral damage and bombard) every turn. He can only be trained in the Ashen Veil's holy city once the world is far toward Armageddon.

**Playing it well.** Late-game shock: fireball the defenders, then crush the weakened city yourself. Pair him with Veil priests and push the Armageddon counter to get him.

**AI: Valued.** The hero step trains him when the holy city of a Veil civilization can (at 1200 hammers, started once we have three cities whatever the turns, which ties that city up for a long time); nothing drives the Armageddon counter toward his threshold. He fights as an army unit, and Fireball is cast by `magic.rs` before strikes next to the target and at the end of the turn only when the Fireball reaches an enemy it beats at 5% odds or better (`magic::summon_value`).

### Orthus (`UNIT_ORTHUS`)
*Hero (melee), tier 2 · not trainable · 5/5, 1 move, +25% city defense · appears for the barbarians at turn 75 (normal speed; not with the No Orthus option) · world unit · Hero, Loyalty, Orcish, Orthus's Axe free*

**What it does.** The barbarians' first hero: an orc warlord placed on an empty plot (barbarian land preferred) at turn 75, with Orthus's Axe (equipment: Blitz and +1 fire damage) and immunity to capture. When he dies, his axe falls to the victor as equipment.

**Playing it well.** An early menace for whoever is nearest: meet him with two or three axemen on defensive terrain or behind walls, and take his axe for one of your heroes.

**AI: Not an AI choice.** The rules create him for the barbarians (`ffh/events.rs`, `cf_add_unit`). As a barbarian unit he attacks what he can beat at 50% odds or better and otherwise roams (`units::barbarian`). For the civilizations he is an ordinary barbarian threat (`production::barbarians_near`, the army's hunts).

### Rantine (`UNIT_RANTINE`)
*Hero (melee), tier 2 · 180 hammers · 4/4, 1 move · Bronze Working · Clan of Embers · world unit · Hero, Orcish free*

**What it does.** The Clan's orc chief: an axeman's strength with +25% against melee and bronze to mithril weapons, and Convert City: in a barbarian city where no barbarian unit is stronger than him (base strength), the city becomes the Clan's, without a fight.

**Playing it well.** The Clan is at peace with the barbarians, so Rantine can walk into their towns: promote him for strength and collect every barbarian city in reach whose garrison he outclasses, an early and cheap expansion.

**AI: Directed.** The hero step trains him and research gets 1500 for Bronze Working (`civs::unique_unit_tech_value`). `civs/clan_of_embers.rs` walks him, while no civilization is at war with us and he is healthy, to the nearest barbarian city within 15 turns whose visible units are no stronger than him, and casts Convert City there; at war he joins the army. Docs/AI.md: +1.2 cities at turn 300 (t 3.2), and cities taken from the barbarians +17% in ordinary games.

### Rathus Denmora (`UNIT_RATHUS`)
*Hero (recon), tier 3 · 180 hammers · 5/5, 2 moves · Poisons · Sidar · world unit · Hero, Hidden, Marksman, Nether Blade, Stealth free*

**What it does.** The Sidar ghost-hero: invisible (Hidden: other teams cannot see him, and +1 first strike, until he fights or casts), Marksman (attacks the weakest unit of a stack), immune to defensive strikes, +50% against melee, better results from villages, and the Nether Blade (+80% against heroes, +2 death damage). Hide makes him invisible again (`bAllowAI` 0).

**Playing it well.** A hero-killer and assassin: walk unseen to an enemy hero or a valuable weak unit, strike, then hide again next turn.

**AI: Partial.** The hero step trains him and research gets 1500 for Poisons; he fights as an army unit. He starts invisible, which protects him until his first fight, but the AI can never re-hide him (Hide is refused to AIs), and nothing points him at enemy heroes for his Nether Blade.

### Rosier Oathtaker (`UNIT_ROSIER_OATHTAKER`)
*Hero (mounted), tier 4 · not trainable · 7/7 + 1 holy, 3 moves, 25% withdrawal · no source in the base game · world unit · Hero, Immune to Disease free*

**What it does.** Rosier before his fall: the holy twin of Rosier the Fallen (+1 holy instead of unholy, +40% against disciple units, no defensive bonuses). Nothing in the vendored data or the rules creates him: no building, event, spell or civilization names his class.

**Playing it well.** Not available in a normal game.

**AI: Not an AI choice.** No path gives him to anyone; were he created, his `UNITAI_HERO` would make him an army unit.

### Rosier the Fallen (`UNIT_ROSIER`)
*Hero (mounted), tier 4 · 180 hammers · 7/7 + 1 unholy, 3 moves, 25% withdrawal · Corruption of Spirit, the Ashen Veil as state religion · every civilization · world unit; leaves a civilization without the religion · Hero, Immune to Disease free; +2 to the Armageddon counter*

**What it does.** The Veil's fallen knight: 8 effective strength, 3 moves, withdrawal, +40% against disciple units (priests, paladins, monks), immune to disease, no defensive bonuses.

**Playing it well.** A priest-hunter and raider: pick off enemy disciples and wounded units, withdraw from bad fights.

**AI: Valued.** The hero step trains him once the Veil is the state religion, scored with moves and withdrawal, and he fights as an army unit. His bonus against disciples is not used to choose targets beyond the generic odds.

### Saverous (`UNIT_SAVEROUS`)
*Hero (demon), tier 2 · 180 hammers · 5/5 + 2 unholy, 1 move · Mind Stapling, the Octopus Overlords as state religion · every civilization · world unit; leaves a civilization without the religion · Demon, Hero free*

**What it does.** The Overlords' champion: a demon (immune to death, poison and unholy, fearless) of 7 effective strength with +25% against melee units, bronze to mithril weapons.

**Playing it well.** Lead the Overlords' infantry against melee armies; keep him away from holy damage.

**AI: Valued.** The hero step trains him once the Overlords are the state religion, and he fights as an army unit.

### Sphener (`UNIT_SPHENER`)
*Hero (angel, disciple), tier 4 · 240 hammers · 12/12 + 4 holy, 2 moves · Righteousness, the Order as state religion · every civilization · world unit; leaves a civilization without the religion · Angel, Channeling I-III, Demon Slaying I, Divine, Hero, Life I, Medic I-III free; -3 to the Armageddon counter*

**What it does.** The Order's archangel: 16 effective strength, 2 moves, +40% against demons, an angel's resistances, Medic III, and the Order's high-priest spells: Bless, Heal, Cure Disease, Unyielding Order (ends unrest in one of our cities while he stays), Sanctify (Life I: removes city ruins and graveyards, -1 Armageddon each, a graveyard yielding an Einherjar; with the No Plot Counter option it also restores hell terrain around), and Sacrifice (`bAllowAI` 0); further spheres with Channeling III.

**Playing it well.** The Order's best unit: lead the army against demons and the Veil, bless and heal the stack each turn, and sanctify ruins to pull the Armageddon counter back.

**AI: Valued.** The hero step trains him once the Order is the state religion; he fights as an army unit, blesses and heals his stack by `magic.rs` (Bless before strikes and at the end of the turn, Heal on the hurt), and casts Unyielding Order as a city spell when he stands in a city. Sanctify is never cast: Python `magic.rs` does not model, and FfH's terms give it 0 (`casting::ffh_value`).

### Teutorix (`UNIT_TEUTORIX`)
*Hero (archery), tier 4 · 180 hammers · 12/12, 2 moves · Blasting Powder · Malakim · world unit · Hero, Nomad free; +1 Overcouncil vote*

**What it does.** The Malakim's siege hero: 12 strength that ignores the defense of city buildings (walls, castles), +25% attack and defense in desert, Nomad (+20% more there and double moves on desert), and a 30% chance of a defensive strike (+25% damage) on units attacking his stack.

**Playing it well.** Take him to sieges: walls do not protect against him, so he can attack a fortified city at once. In the desert he is nearly unbeatable.

**AI: Valued.** The hero step trains him and research gets 1500 for Blasting Powder (`civs::unique_unit_tech_value`); he fights as an army unit and in the operations' strikes. His wall-ignoring attack is not in his power score (`production::best_unit` reads strength, moves, first strikes and withdrawal), though the strike's odds see it.

### The Black Wind (`UNIT_BLACK_WIND`)
*Hero (ship), tier 3 · 300 hammers · 10/10, 4 moves, 20% withdrawal, carries 2 land units · Optics · Lanun · world unit · Hero, Hidden Nationality free*

**What it does.** The Lanun hero ship: a 10-strength pirate vessel with no nationality (it can attack anyone without a war, and anyone can attack it), +3 gold for each fight it wins, and room for two land units. Declare Nationality ends the disguise.

**Playing it well.** Raid rival shipping and coasts at peace, ferry a hero and an escort for landings, and collect gold from every victory.

**AI: Partial.** A coastal city trains it in the hero-ship step (`production::decide`, `Role::SeaHero`), on its default `UNITAI_ASSAULT_SEA`; docs/AI.md: Black Wind trained in 0% -> 69% of Lanun games after the rule. With cargo space it is a transport (`units::role_of`, `naval::transport`): settlers overseas and amphibious assaults. It never fights at sea (no naval combat logic), and at the end of its first turn it declares its nationality (FfH's weight of 50, `casting::ffh_value`), so the pirate disguise is thrown away unused.

### The Mithril Golem (`UNIT_MITHRIL_GOLEM`)
*Hero (golem), tier 4 · 1200 hammers · 25/25, 1 move, +60% city attack · Mithril Working, Mithril, the Runes of Kilmorph as state religion, Kilmorph's holy city, an Armageddon counter of 70 · every civilization · world unit; leaves a civilization without the religion · Channeling III, Divine, Golem, Sentry free*

**What it does.** Kilmorph's avatar: 25 strength with +60% city attack, a golem (fearless, immune to death and poison, slow to heal), no unit can upgrade into it, and Earthquake (every plot within 1: units unfortified, about a quarter of each city's buildings and improvements destroyed; it declares war on the owners it hits).

**Playing it well.** A city breaker for the end of the world: quake the city from next door, then walk in. Getting it means owning Kilmorph's holy city and letting the Armageddon counter climb.

**AI: Valued.** The hero step trains it in Kilmorph's holy city when the rules allow (1200 hammers, started once we have three cities whatever the turns); nothing drives the counter toward 70. It fights as an army unit; Earthquake is valued before strikes at 15 per enemy in the target city (`magic::py_value`), a value that does not check that the golem stands next to the city.

### The War Machine (`UNIT_WAR_MACHINE`)
*Hero (mounted), tier 4 · 240 hammers · 19/19, 3 moves, 25% withdrawal · Machinery and a Siege Workshop in the city · Doviello · world unit · Hero free*

**What it does.** The Doviello's siege beast: 19 strength, 3 moves, withdrawal, +50% against mounted units, no defensive bonuses. It has the Hero promotion but an attacker's default AI.

**Playing it well.** A mid-game battering ram that outclasses everything of its era: build a Siege Workshop for it as soon as Machinery comes and lead every Doviello offensive with it.

**AI: Valued.** The Hero promotion makes it a hero for production (`production::is_hero`; as an ordinary attacker it never won on cost), so the hero step trains it, but only in a city that already has a Siege Workshop: `production::enabler_building` does not build buildings for heroes, so the workshop comes from `campaign::siege_building` (for siege engines, in the most productive city, when an operation wants them). Research gets 1500 for the tech that completes it (Machinery with the workshop's techs, `civs::unique_unit_tech_value`). It fights as an army unit (`UNITAI_HERO`).

### TumTum (`UNIT_TUMTUM`)
*Hero (mounted), tier 3 · 150 hammers · 12/10, 3 moves · Trade · the barbarians only · world unit · Hero, Orcish, Valor free*

**What it does.** The barbarians' boar-riding ogre: 12 attack, 3 moves, +25% against recon units, Valor (an extra experience point from each fight, 10% magic resistance).

**Playing it well.** When TumTum appears, guard workers and border towns with strong defenders; he hits hard but defends at 10 without terrain help, so attack him on open ground.

**AI: Not an AI choice.** No civilization trains him; barbarian cities put him near the top of their fixed list once they can (`cf_barb_city_production`: 100000 while none exists, below only the Acheron's 150000), called from `production::choose`. He then raids as a barbarian unit (`units::barbarian`).

### Valin Phanuel (`UNIT_VALIN`)
*Hero (mounted), tier 4 · 180 hammers · 6/6 + 2 holy, 3 moves, 25% withdrawal · Orders from Heaven, the Order as state religion · every civilization · world unit; leaves a civilization without the religion · Demon Slaying I, Hero, Immune to Disease free; -2 to the Armageddon counter*

**What it does.** The Order's knight: 8 effective strength, 3 moves, withdrawal, +40% against demons, immune to disease, no defensive bonuses.

**Playing it well.** An early Order hero for hunting barbarians, demons and undead; his free experience makes him a strong mounted unit in time.

**AI: Valued.** The hero step trains him once the Order is the state religion (Orders from Heaven), scored with moves and withdrawal, and he fights as an army unit.

### Wilboman (`UNIT_WILBOMAN`)
*Hero (melee), tier 3 · 360 hammers · 7/7, 1 move · Iron Working · Illians · world unit · Hero, Immune to Cold, Sentry, Vulnerable to Fire free*

**What it does.** The Illians' frost giant: 7 strength, +25% on tundra and ice, immune to cold, 50% more fire damage, +1 sight, bronze to mithril weapons; he cannot enter desert.

**Playing it well.** A defender and attacker for the Illians' frozen lands; keep him away from fire casters.

**AI: Valued.** The Hero promotion makes him a hero (`production::is_hero`; his attacker's default AI made him lose every comparison on cost before), so the hero step trains him: docs/AI.md, Wilboman trained in 5% -> 45% of Illian games after the rule. Research gets 1500 for Iron Working (`civs::unique_unit_tech_value`). He fights as an army unit.

### Yvain (`UNIT_YVAIN`)
*Hero (disciple), tier 4 · 240 hammers · 9/9, 2 moves · Commune with Nature, the Fellowship of Leaves as state religion · every civilization · world unit; leaves a civilization without the religion · Channeling I-III, Divine, Elemental, Hero, Life I, Medic I-III, Nature I free; +2 Nature affinity*

**What it does.** The Fellowship's forest lord: 9 strength, +50% in forest and +100% in ancient forest, +50% against animals, moves through impassable terrain, an elemental (fearless, immune to death and poison), Medic III, and the Fellowship's spells: Summon Treant (in forest: a permanent 10-strength treant), Summon Tiger, Bloom, Heal, Cure Disease, Treetop Defense (fortifies his stack in a forest) and Sanctify.

**Playing it well.** Defend the Fellowship's forests with him: in an ancient forest he doubles his strength, and each turn there he can raise another permanent treant. Heal the army between fights.

**AI: Valued.** The hero step trains him once the Fellowship is the state religion; he fights as an army unit. Summon Treant and Summon Tiger are cast by `magic.rs` as lasting summons at war or with enemies about (`magic::summon_value`), Heal and Cure Disease when needed, Treetop Defense by its weight of 10, Bloom by FfH's +10; Sanctify is never cast (value 0). Nothing keeps him in forests, where he is strongest and can summon treants.

## Great people

### Decius (`UNIT_DECIUS`)
*Great person · not trainable · 2 moves, defends only · no civilization in the base game*

**What it does.** A named Great Commander from a scenario: no playable civilization has him (his class is the Great Commander's, but no civilization lists him). A unit of strength 6 or more on his plot can take him as the Great Commander promotion (Join Commander, Decius); splitting him off again gives Decius back.

**Playing it well.** Not available in normal play.

**AI: Not an AI choice.** No base-game player gets him. If one did, `units::role_of` would make him a great person (`UNITAI_ENGINEER`), who walks to the capital and finds nothing to do there (he can neither lightbulb, join a city nor start a golden age, `units::great_person`); a strong unit standing with him would absorb him by FfH's terms (the promotion's value, `casting::ffh_value`).

### Great Bard (`UNIT_ARTIST`)
*Great person · not trainable · 2 moves, defends only · from Bard points (Bards, Taverns, Freak Shows and cages, wonders) · every civilization*

**What it does.** It can perform a Great Work (+4000 culture in a city), discover a technology (1000 research plus 2 per population), join a city as a Great Bard specialist, help start a golden age, or build the Song of Autumn, the shrine of the Fellowship of Leaves in its holy city.

**Playing it well.** A Great Work makes a border city's culture win flips and pushes a culture victory; early on, a lightbulb or a settled Bard (culture and commerce every turn) is usually worth more. With the Fellowship's holy city, the Song of Autumn shrine pays a gold a turn for every Fellowship city in the world.

**AI: Directed.** `units::great_person` first gives it a Great Work in the culture city furthest from legendary when the AI pursues a cultural victory (`victory::great_work`), then the Song of Autumn if one of our cities is the Fellowship's holy city and can build it (`units::great_person_build`), and otherwise walks it to the capital to lightbulb (`MISSION_DISCOVER`), else to join as a specialist, else a golden age. Since every Bard that can lightbulbs, Bards join cities almost never.

### Great Commander (`UNIT_COMMANDER`)
*Great person · not trainable · 2 moves, defends only · from Commander points (Command Post, Heroic and National Epics, Basilica, several wonders) or freed from a lair · every civilization*

**What it does.** A war leader with no research of its own: a unit of strength 6 or more on its plot can take it as the Great Commander promotion (+1 strength and defense, +1 experience per fight; Join Commander), which can later be split off again. With Recruiter it can recruit in one of our cities (population / 3 + 1 random units, from scouts to champions and longbowmen as our techs allow, twice as many with a Temple of the Order; the Commander is spent). It can also build a Command Post or help start a golden age.

**Playing it well.** Recruit in a big city at the start of a war for an instant army, or attach it to your best attacker so it levels faster; the Command Post (+20% military production, +2 experience) is worth it only in the city that trains your army.

**AI: Partial.** It is a great person (`units::role_of`: `UNITAI_ENGINEER`): `units::great_person` walks it to the capital, where it cannot lightbulb or join and a golden age needs two different great people (`BASE_GOLDEN_AGE_UNITS` 2), so it usually waits. At the end of a turn in a city of ours it casts Recruit when that city gives two recruits or as many as our best (`magic::recruit_value`; the Bannor cast it at the start of the turn, `civs::bannor`), unless a strength-6+ unit on its plot takes it first by FfH's terms (Join Commander, valued as the promotion, `casting::ffh_value`). The Command Post is never built by it (`units::great_person_build` builds only shrines).

### Great Engineer (`UNIT_ENGINEER`)
*Great person · not trainable · 2 moves, defends only · from Engineer points (Guild of Hammers, Mines of Galdur, Soul Forge, shrines) · every civilization*

**What it does.** It can hurry production (500 hammers plus 20 per population), discover a technology (1000 research plus 2 per population), join a city as a Great Engineer specialist, or help start a golden age.

**Playing it well.** Its best use is a wonder: rush the Great Library, a Tower or a religion's wonder you would otherwise lose the race for. With no wonder in sight, a lightbulb or a settled engineer in the production city.

**AI: Directed.** `units::great_person_build` sends it to a city of ours within 20 turns that is building a world or national wonder, when at least two thirds of the rush would go into it (`MISSION_HURRY`); otherwise `units::great_person` lightbulbs in the capital (then join, then golden age). In a traced game it rushed a wonder 3 times of 10; the others found no wonder in progress (docs/AI.md, "Round 6"). No city starts a wonder for it to rush.

### Great Merchant (`UNIT_MERCHANT`)
*Great person · not trainable · 2 moves, defends only · from Merchant points (Bazaar of Mammon, Great Lighthouse, Nox Noctis) · every civilization*

**What it does.** It can conduct a trade mission in a foreign city (500 gold plus 200 for every point of trade-route profit between that city and our capital), discover a technology, join a city as a Great Merchant, help start a golden age, or build Nox Noctis, the Council of Esus's shrine in its holy city. It may enter rival territory.

**Playing it well.** A trade mission to a big distant city funds a war or a round of upgrades at once; a lightbulb or a settled merchant pays over time. With the Esus holy city, the shrine.

**AI: Partial.** `units::great_person` builds Nox Noctis in a holy city of ours (`units::great_person_build`) and otherwise lightbulbs in the capital (then join, then golden age); the AI never conducts trade missions (no `MISSION_TRADE` anywhere in `ffh-ai`). Nothing compares the gold of a mission with the research of a lightbulb.

### Great Prophet (`UNIT_PROPHET`)
*Great person · not trainable · 2 moves, defends only · from Prophet points (Priests, the Asylum, Dies Diei, The Necronomicon, other wonders and the Altar stages) · every civilization*

**What it does.** It can discover a technology, join a city as a Great Prophet, help start a golden age, and build a religion's shrine in its holy city (Code of Junil, Dies Diei, Nox Noctis, Song of Autumn, Stigmata on the Unborn, Tablets of Bambur, The Necronomicon: a gold a turn for every city in the world with the religion, plus culture and more) or a stage of the Altar of the Luonnotar.

**Playing it well.** The first Prophet of a religion you founded builds its shrine: that is the best gold source of the early game. Later Prophets go to the Altar of the Luonnotar if you aim at its victory, or lightbulb religious and philosophical techs.

**AI: Directed.** `units::great_person_build` sends it to build a shrine in a holy city of ours it can reach within 20 turns, shrines first; otherwise it lightbulbs in the capital (`units::great_person`). Measured: before this rule no AI ever built a shrine (0 in 16 games); with it, 65 shrines in 24 full games, research +3% and culture +5% from turn 300 (docs/AI.md, "Round 6"). The Altar stages are never built (only shrines are); the Altar of the Luonnotar is never the AI's own victory goal anyway (`victory::plan_ours` drops it, the owner's decision), so Prophets beyond the shrines lightbulb.

### Great Sage (`UNIT_SCIENTIST`)
*Great person · not trainable · 2 moves, defends only · from Sage points (Great Library, Catacomb Libralus, Inspiration, Sages) · every civilization*

**What it does.** The research great person: it discovers a technology for more than the others (1500 research plus 3 per population), joins a city as a Great Sage, helps start a golden age, and can build an Academy (+50% research in its city) or the Ashen Veil's shrine, Stigmata on the Unborn, in its holy city.

**Playing it well.** The first Sage usually builds an Academy in the capital or the best research city: it pays every turn for the whole game, where a lightbulb pays once. Later Sages lightbulb key techs or settle.

**AI: Directed.** Lightbulbs in the capital (`units::great_person`, `MISSION_DISCOVER`), or builds Stigmata on the Unborn in a Veil holy city of ours (`units::great_person_build`); the Academy is never built, because `great_person_build` builds only shrines, so the AI trades a lasting +50% for a one-time tech.

## Summons

### Air Elemental (`UNIT_AIR_ELEMENTAL`)
*Summon, tier 4 · 5/5 +2 lightning, 2 moves, flying · Summon Air Elemental (Air III), for one turn · Air affinity +1 a node*

**What it does.** A flying elemental that crosses water and any terrain and sees two plots further (Sentry I-II); immune to lightning, death and poison. Each win over a living unit creates a Lightning Elemental for the same duration. It gains a point of strength for every Air mana its owner has. Like every temporary summon it lasts the turn it is called (a turn more for a Summoner leader); the Tower of the Elements gives its builder one random permanent elemental, held in its city, and makes every elemental Strong (+1/+1).

**Playing it well.** Weak for a tier-4 summon, so use its reach: strike a wounded unit across a lake or behind a front, or pick off a lone worker, so the win spawns a Lightning Elemental for a second attack. With several Air nodes it becomes a real attacker.

**AI: Valued.** `magic.rs` values the cast as the fight the summon wins at once (`magic::summon_fight`): before an attack when the caster stands next to the target (`magic::soften`, then `magic::summons_attack` sends it in), and at the end of the turn at the best target it reaches at 5% or better (`magic::summon_value`, `casting::summons_attack`). The estimate uses the unit type's 5 + 2: affinity, Strong and the Lightning Elementals are not counted, so the AI undervalues it as its Air mana grows. A summon that lives into the next turn plays as an army unit (`military::army`). Casters take Air spheres by the summons they open (`magic::spell_worth`), but the AI trains few casters (`production::decide`, caster step).

### Aurealis (`UNIT_AUREALIS`)
*Summon, tier 4 · 6/6, 1 move · Summon Aurealis (Sun III), for one turn · Sun affinity +2 a node*

**What it does.** A sun elemental (immune to death, poison and fear) of 6 strength, plus two for every Sun mana its owner has: the only summon with an affinity of 2.

**Playing it well.** The Sun mage's hammer: with two or three Sun nodes an Aurealis hits at 10-12, more than an Earth Elemental, every turn the mage can cast it next to a target.

**AI: Valued.** As the Air Elemental: summoned and sent in at once by `magic.rs` (`magic::summon_value`, `magic::summons_attack`), valued at its base 6; the doubled Sun affinity, its point, is not counted.

### Battering Ram (`UNIT_BATTERING_RAM`)
*Siege, tier 2 · 1/1, 1 move, bombards 25% a turn · Create Battering Ram: a Khazad melee unit in a forest, with Engineering (3 turns), for good*

**What it does.** A Khazad field engine: a melee unit standing in a forest builds one over three turns, and it bombards a city's defense by 25% a turn. It cannot really fight, gets no defensive bonuses and gives no military happiness.

**Playing it well.** The Khazad's siege without a workshop: build rams in the forests next to an enemy city and bring its defense down before the axes go in.

**AI: Unused.** Create Battering Ram is `bAllowAI` 0, so the rules refuse it to AI players; no AI ever has a ram. (If one existed, `magic::raised` would count it as a unit summoned for good, and `campaign.rs` would bombard with it.)

### Djinn (`UNIT_DJINN`)
*Summon, tier 4 · 1/1, 2 moves · Summon Djinn (Metamagic III), for one turn · affinity +1 with every mana*

**What it does.** A Metamagic elemental of base strength 1 that gains a point for every mana resource of any type its owner has: weak for a poor empire, a powerhouse for one with a dozen nodes.

**Playing it well.** Worth casting only with many mana sources: a mana-rich empire (or one that trades for mana) turns its Metamagic mages into Djinn summoners that outhit most units.

**AI: Partial.** Summoned as the other summons are (`magic::summon_value`), but valued by the type's strength of 1, with no affinity (`magic::summon_fight`), so it rarely clears the 5% bar and its promotion worth is minimal (`magic::spell_worth`): the AI casts it much less than its real strength would justify.

### Earth Elemental (`UNIT_EARTH_ELEMENTAL`)
*Summon, tier 4 · 11/11, 1 move · Summon Earth Elemental (Earth III), for one turn · Earth affinity +1 a node*

**What it does.** The strongest common elemental: 11 strength (plus a point per Earth mana), slow, immune to death, poison and fear.

**Playing it well.** A city and stack breaker: walk the summoner next to the target and summon it there every turn of the siege. Its one move means the mage must stand adjacent, so protect him.

**AI: Valued.** As the Air Elemental (`magic::summon_value`, `magic::soften`, `magic::summons_attack`); at 11 strength it wins many of the fights `magic::summon_fight` estimates, and casters with Earth take it by `magic::spell_worth`. Affinity and the Tower of the Elements' Strong are not counted.

### Fire Elemental (`UNIT_FIRE_ELEMENTAL`)
*Summon, tier 4 · 6/6 +3 fire, 2 moves, collateral 50 (up to 4 units, to 30%) · Summon Fire Elemental (Fire III), for one turn · Fire affinity +1 a node*

**What it does.** A fast fire elemental with collateral damage, immune to fire, death and poison; it refuses to enter ice.

**Playing it well.** Fire's stack-softener: its collateral wears down every defender of a stack or city before your attack, and with Fire nodes and the Tower of the Elements it grows strong. It does little against fire-immune units and less against the demons' 20% fire resistance.

**AI: Valued.** As the Air Elemental, and `magic::summon_fight` counts its collateral on the other defenders, so before a strike it ranks high (`magic::soften`); the Fire sphere's worth counts it half again for the collateral (`magic::spell_worth`).

### Fireball (`UNIT_FIREBALL`)
*Summon, tier 3 · 0/0 +4 fire, 1 move, flying, collateral 50 (up to 4 units, to 30%), bombards 10% · Fireball (Fire II), for one turn*

**What it does.** A flying missile: it ignores terrain, is immune to first strikes, does collateral damage, and dies in the attack (suicide). It cannot capture, and its death causes no war weariness; it can also bombard a city's defense by 10%.

**Playing it well.** Several mages casting Fireball every turn of a siege replace catapults: the collateral wounds the whole garrison and the defenders gain little. Cast them right before your real attack.

**AI: Valued.** `magic.rs` casts Fireball before an attack (the summon's fight and collateral, `magic::summon_fight`) and at the end of the turn when a target at 5% or better is in reach; the Fireball attacks at once (`magic::summons_attack`, `casting::summons_attack`). The AI casts in the summon-suicide mode (`casting::end_turn`, `magic::cast_phases`), in which the rules leave the Fireball to attack rather than bombard (`unit_cast_create_unit`), so its Fireballs never bring walls down.

### Flesh Golem (`UNIT_FLESH_GOLEM`)
*Summon, tier 3 · 6/6, 1 move · Graft Flesh (Body III), for good*

**What it does.** A permanent golem (no promotions, slow healing, immune to death, poison and fear) of 6 strength. Other units stacked with it can be sacrificed into it (Add to Flesh Golem): it takes their promotions and gains a point of strength when the sacrificed unit was stronger.

**Playing it well.** A sink for obsolete units: feed it old warriors, wounded veterans and promoted scouts, and it grows into a heavily promoted monster that costs the support of one unit.

**AI: Partial.** Graft Flesh is cast at the end of the turn while at war or with enemies about, worth its strength (`magic::summon_value`: a lasting summon); the golem is an army unit and, as a unit summoned for good, is spent first in a strike after the collateral attackers (`campaign::strike`, `magic::raised`). Add to Flesh Golem is `bAllowAI` 0, so the AI's golems never grow.

### Floating Eye (`UNIT_EYE`)
*Summon (air), tier 0 · no strength, 4-plot recon range · Floating Eye (Metamagic I), for one turn*

**What it does.** A flying eye that reveals four plots around it, sees invisible units and may enter rival territory; it cannot fight.

**Playing it well.** Cheap reconnaissance: before a march, an Eye shows the enemy city's garrison and any hidden Shadows or Assassins lying in wait.

**AI: Unused.** Every AI value of the cast is 0: `magic::summon_fight` and FfH's summon term both rate a summon by its strength, which is 0 (`magic::summon_value`, `casting::summon_value`), so neither `magic.rs` nor FfH's choice ever casts it. (The Amurites' adepts, with Metamagic from their palace, could cast it and Haste, and cast nothing useful, docs/AI_LOG.md, "Training casters for their spells".)

### Host of the Einherjar (`UNIT_EINHERJAR`)
*Summon, tier 3 · 3/3 +2 holy, 1 move · Summon Host of the Einherjar (Law II), one turn, +1 a win · disciple*

**What it does.** Angelic warriors (immune to fear; resist holy and poison fully, death by half): every fight they win adds a turn to their duration, so a Host that keeps winning never leaves. Sanctify cast on a Graveyard also raises one.

**Playing it well.** Mop up weak units: hunt barbarians, workers and wounded stragglers every turn so the Host stays. Holy damage makes it good against undead and demons.

**AI: Valued.** As the other summons (`magic::summon_value`, `casting::summons_attack`); a Host that wins lives on and plays as an army unit (`military::army`), which attacks what it can beat. Nothing hunts on purpose to keep it alive.

### Ice Elemental (`UNIT_ICE_ELEMENTAL`)
*Summon, tier 3 · 2/2 +2 cold, 1 move, +10% on tundra, +20% on ice · Summon Ice Elemental (Ice II), for one turn · Ice affinity +1 a node*

**What it does.** A weak elemental of the cold, immune to cold and vulnerable to fire; it cannot enter desert or burning sands.

**Playing it well.** Only worth it with Ice nodes and on snow and tundra, where the bonuses and affinity add up; otherwise a minor chip damage spell.

**AI: Valued.** As the other summons (`magic::summon_value`, summoned as attackers whatever its default garrison AI); at 4 total strength it seldom clears the 5% bar except against hurt units.

### Kraken (`UNIT_KRAKEN`)
*Summon (sea), tier 4 · 17/17, 4 moves · Summon Kraken (an Overlords disciple with Channeling III and Divine, next to water), for good*

**What it does.** A permanent sea beast of 17 strength with Stealth (it can Hide), the strongest naval unit most players will ever see. It gives no military happiness. Exploring a lair at sea may also release one as a barbarian.

**Playing it well.** Sea control for the Overlords: a Kraken sinks transports and warships, blockades a rival's coast and escorts your own fleets.

**AI: Partial.** Summoned at the end of the turn while at war or with enemies about (`magic::summon_value`: a lasting summon worth its strength), and then never used: as a sea unit on the attack AI it is a "naval" unit to `units::role_of`, which holds (there is no naval combat logic). Hide is `bAllowAI` 0.

### Meteor (`UNIT_METEOR`)
*Summon, tier 4 · 0/0 +6 fire, 1 move, flying, collateral 100 (up to 4 units, to 60%), bombards 15% · Breathe Fire (a dragon's ability), for one turn*

**What it does.** A dragon's breath: a suicide flying missile with 6 fire strength and heavy collateral damage, immune to first strikes, causing no war weariness. Acheron, Drifa and the other dragons cast it.

**Playing it well.** A dragon breathes before it attacks: the Meteor's collateral wounds the whole stack the dragon is about to hit.

**AI: Valued.** A dragon of ours breathes at the end of the turn by FfH's terms when an enemy is within the Meteor's reach (`casting::ffh_value`: the Meteor's 6 strength x100; Breathe Fire is an ability, outside `magic.rs`'s own terms), and the Meteor attacks at once at 5% or better (`casting::summons_attack`). That end-of-turn value (600) is also the bar a cast before an attack must clear (`magic::cast_phases` keeps a caster's cast for its better end-of-turn use), and the Meteor's fight in hit points never does, so a dragon does not breathe on the city it is about to strike. Barbarian Acheron breathes by FfH's terms only.

### Mistform (`UNIT_MISTFORM`)
*Summon, tier 4 · 6/6, 2 moves · Summon Mistform (Shadow III), for one turn · Shadow affinity +1 a node*

**What it does.** An invisible elemental with Hidden Nationality: it can attack the units of players we are not at war with, ignores a city's building defense, and can Declare Nationality to drop the disguise.

**Playing it well.** Deniable violence: kill a rival's settler or worker, or wound a garrison, without a war; or slip past enemy lines unseen and hit the stack behind them.

**AI: Partial.** Cast like any summon when an enemy at war is in reach (`magic::summon_value` counts enemies only). Once it exists, though, the end-of-turn summon attack takes the best target among all units `plot_is_visible_enemy_unit_for_unit` lets it attack, and a hidden-nationality unit counts every other team's units (`unit_is_always_hostile`): it may hit a neutral's unit when the odds there are better, by accident, never by plan (`casting::summon_target`). Declare Nationality is refused to the AI for invisible units (docs/AI_ONLY_RULES.md, A2).

### Pit Beast (`UNIT_PIT_BEAST`)
*Summon, tier 3 · 4/4 +1 unholy, 1 move · Summon Pit Beast (Entropy II), one turn, +1 a win*

**What it does.** A demon beast (immune to death, poison, unholy and fear) that stays a turn longer for every fight it wins.

**Playing it well.** Like the Einherjar for the dark spheres: feed it easy kills every turn and it remains, a free extra attacker for an Entropy mage.

**AI: Valued.** As the other summons (`magic::summon_value`, `casting::summons_attack`); one that wins lives on as an army unit (`military::army`).

### Puppet (`UNIT_PUPPET`)
*Summon, tier 2 · 2/2, 1 move · Create Puppet (Balseraph arcane units), for one turn*

**What it does.** A puppet of its maker: it copies the caster's promotions, so it can cast the caster's spells, at -20% strength and -10% spell damage (Puppet promotion), and it is not alive. It gives no military happiness.

**Playing it well.** The Balseraph trick: send the puppet to cast what is dangerous for the mage itself (Wonder, Domination, spells at the front) or to explore, and let it die instead of the mage.

**AI: Partial.** Create Puppet carries an `iAIWeight` of 1000, which `magic::turn_value` adds to its value, so a Balseraph caster with nothing worth more makes one every turn; the puppet attacks at once at 5% or better (`casting::summons_attack`). It is made after the end-of-turn loop has listed our units (`casting::end_turn`), so it casts only if it lives into our next turn (a Summoner leader's), and then by the generic choice like any caster, never because a spell is too risky for its maker. Valuing it as a plain summon instead lost share (docs/AI_LOG.md, "Balseraphs: Create Puppet valued as a plain summon").

### Puppet (`UNIT_PUPPET_LOKI`)
*Summon, tier 2 · 2/2, 1 move · Create Puppet (Loki only), for one turn*

**What it does.** Loki's own puppet: it copies his promotions, may enter rival territory, and withdraws from 10% of the fights it loses on defense.

**Playing it well.** Loki's scout and decoy: cast one every turn to scout the city he is heading for, and to take the hits meant for him.

**AI: Partial.** As the Balseraph puppet: `iAIWeight` 1000 makes Loki cast it every turn (`magic::turn_value`; docs/AI.md: puppets per Loki 193 -> 230 in the census), and it attacks at 5% or better (`casting::summons_attack`). A version that attacked where it took most hit points sent puppets at a shadowing Spectre every turn and got Loki hurt (docs/AI_LOG.md, "Summons at the end of the turn...").

### Sand Lion (`UNIT_SAND_LION`)
*Summon, tier 3 · 5/5 +1 fire, 3 moves, +40% on desert · Summon Sand Lion (a Malakim arcane unit with Channeling II, on desert), for one turn*

**What it does.** A fast desert beast (Nomad: double moves and +20% more on desert) that sees invisible units; the Malakim summon it only from desert plots.

**Playing it well.** The Malakim's ambush: from the dunes it reaches and hits units three plots away at up to 9 strength. Fight wars in the desert and keep Channeling II adepts there.

**AI: Valued.** As the other summons (`magic::summon_value` from its 5 + 1, `casting::summons_attack`); the desert bonuses are not in the estimate, and nothing moves casters onto desert for it.

### Severed Soul (`UNIT_SEVERED_SOUL`)
*Summon, tier 2 · no strength, 1 move, flying, invisible · Sever Soul (the Divided Soul), for one turn*

**What it does.** The wandering soul of the Divided Soul: invisible to all, it explores around its body and can Call Form, teleporting the Divided Soul to its plot (and ending itself).

**Playing it well.** A teleport: send the Soul to the plot you want, scout it unseen, and call the body there.

**AI: Unused.** It defends only and has no strength, so every summon value is 0 (`magic::summon_fight` returns nothing for it; `casting::summon_value` rates strength): Sever Soul is never cast, and Call Form is `bAllowAI` 0 anyway.

### Skeleton (`UNIT_SKELETON`)
*Summon, tier 1 · 2/2 +1 death, 1 move · Raise Skeleton (Death I and Channeling I), for good*

**What it does.** A permanent undead soldier (immune to death, poison, disease and fear; holy hurts it more), which cannot move for two turns after it is raised. Barrows spawn them as barbarians, and barbarian skeletons that reach level 3 become Spectres. No military happiness.

**Playing it well.** Raise one every turn of a war and use them as fodder: they cost nothing, so throw them at the defenders first to strip their health, and pile them in front of a city to absorb counterattacks.

**AI: Valued.** Raise Skeleton is cast at the end of the turn while at war or with enemies about (`magic::summon_value`, a lasting summon worth its strength); skeletons are army units, and in a strike they attack right after the collateral attackers as siege fodder (`campaign::strike`, `magic::raised`; the owner's call, docs/AI.md, "Field armies"). The AI trains no casters for it: adepts for Raise Skeleton are an untried idea (docs/AI.md, "Known gaps").

### Spectre (`UNIT_SPECTRE`)
*Summon, tier 4 · 3/3, 2 moves · Summon Spectre (Death II), for one turn · Death affinity +1 a node*

**What it does.** An undead ghost with Fear: living units without fear immunity may refuse to attack it, and when it wins outside a city, enemies on that plot may flee. Barbarian Skeletons become Spectres at level 3, and barbarian Spectres become Wraiths at level 5.

**Playing it well.** A scatterer: summon it against a stack of living units, win a fight, and watch the rest of the stack run. Useless against undead, golems and fear-immune units.

**AI: Valued.** As the other summons (`magic::summon_value` from its 3 strength, `casting::summons_attack`); Fear is not in the estimate.

### Treant (`UNIT_TREANT`)
*Summon, tier 4 · 10/10, 1 move · Summon Treant (a Fellowship disciple with Channeling III and Divine, in a forest), for good*

**What it does.** A permanent tree spirit of 10 strength with Woodsman I-II (+60% attack and defense in forest and jungle, double moves there), vulnerable to fire. When it loses a fight, a young forest grows on its plot. March of the Trees and lair exploration can also create Treants.

**Playing it well.** Forest defenders: Treants on forested hills next to your cities hold against anything without fire; in your own forests they move fast. Keep them away from fire magic and Pyre Zombies.

**AI: Valued.** Summoned at the end of the turn while at war or with enemies about (`magic::summon_value`, worth its strength); an army unit afterwards, spent early in strikes as a unit summoned for good (`campaign::strike`, `magic::raised`). Woodsman terrain is not weighed when it chooses where to fight.

### Water Elemental (`UNIT_WATER_ELEMENTAL`)
*Summon, tier 4 · 8/8, 1 move · Summon Water Elemental (Water III), for one turn · Water affinity +1 a node*

**What it does.** A water-walking elemental that fights on land or across coast; when it loses, it splits into two smaller Water Elementals (25% damage each, Weak unless it was Strong) with the same remaining duration.

**Playing it well.** Losing is half a win: a defeated Water Elemental leaves two that can attack again, so summon it against strong defenders you mean to grind down.

**AI: Valued.** As the other summons (`magic::summon_value`, `casting::summons_attack`); the split is not in the estimate, and the halves, as new units, are not sent in by the summon attack, so they act only if they live into the next turn.

### Wraith (`UNIT_WRAITH`)
*Summon, tier 4 · 6/6 +3 death, 2 moves · Summon Wraith (Death III), for one turn · Death affinity +1 a node*

**What it does.** The strongest death summon: undead with Fear and Vile Touch (the living units it fights may wither: -10% strength, -20 healing, half experience). Call of the Grave on a Graveyard raises three, and barbarian Spectres become Wraiths at level 5.

**Playing it well.** Strike and scatter: its Fear breaks living stacks, Vile Touch cripples what survives. Its death damage is wasted on undead, demons and golems.

**AI: Valued.** As the other summons (`magic::summon_value` from its 6 + 3, `casting::summons_attack`); Fear and Vile Touch are not in the estimate.

## Equipment

### Athame (`EQUIPMENT_ATHAME`)
*Item · not trainable · carried as the Athame promotion: +3 poison strength, inflicts Plagued in combat · Yersinia's own item (drops when she dies)*

**What it does.** Like every item, the Athame exists in two forms: a 0-strength, immobile unit lying on a plot, and a promotion on the unit that carries it. A unit on the item's plot picks it up with Take Athame (`SPELL_TAKE_ATHAME_UNIT`, which kills the item unit and gives the promotion), or takes it from another unit of the same owner in its stack (`SPELL_TAKE_ATHAME_PROMOTION`); ships, siege engines, summons and illusions cannot (`spells_b.rs::take_equipment_checks`). When the carrier dies, the item drops on its plot as a unit of the carrier's owner (`unit/lifecycle.rs`, `unit_kill`), and any enemy that moves onto that plot captures it, even a unit that cannot capture (`unit/combat.rs`, "equipment is always captured"). The Athame itself adds 3 poison strength and may afflict the living units its bearer fights with Plagued (-60% strength, 20 less healing a turn, until cured). Its only source is Yersinia, the fourth Horseman of the Apocalypse (barbarian).

**Playing it well.** Kill Yersinia with a stack that can advance onto her plot in the same turn, so the Athame is yours at once; give it to a fast attacker, where the poison counts in every fight and a plagued survivor is nearly harmless afterwards. Poison-immune targets (undead, demons, elementals) ignore the extra strength.

**AI: Not an AI choice.** The rules make and drop it. The AI never walks to an item: a unit of ours that ends the turn on the item's plot takes it by FfH's spell terms at the end of the turn (`casting::end_turn`, `casting::ffh_value`: the item promotion's `units::promotion_value`, here 3 poison x 40, and no `iAIWeight`), the first of our units there to cast; an enemy's item near our cities counts as an enemy unit for the field armies (`military::find_targets`) and as a goody for explorers (`units::explore_target`, through `plot_is_revealed_goody`). Take Athame (promotion) has no AI weight, so the AI never passes the Athame on to a better unit, and the plague it spreads is not valued.

### Black Mirror (`EQUIPMENT_BLACK_MIRROR`)
*Item (world limit 1) · not trainable · carried as the Black Mirror promotion, which casts Mirror · Alazkan's own item*

**What it does.** An item (see Athame for how items are carried, dropped and captured). The promotion has no combat effect; it lets its bearer cast Mirror (`SPELL_MIRROR`, `spells_a.rs::spell_mirror`): a copy of the bearer with all its promotions (but no items), its damage, level and experience appears on its plot as an Illusion for one turn, already having cast (an Illusion cannot take a unit below 10% health and heals fully when it wins). The bearer must not be immune to magic, and no visible enemy may stand on its plot. It comes into play when Alazkan the Assassin, the Svartalfar hero who starts with it, dies.

**Playing it well.** A second copy of a high-level hero for a turn: cast it before an attack so the illusion takes the first, worst fight, or next to a city to make an attack look stronger than it is. It is worth most on a strong, heavily promoted unit.

**AI: Not an AI choice.** Picked up by a unit of ours that ends the turn on it (`casting::ffh_value`: the promotion is worth only the minimum 1 to `units::promotion_value`), never fetched, never passed on (Take Black Mirror (promotion) has no AI weight). A bearer casts Mirror at the end of every turn it can for its flat `iAIWeight` of 50 (`casting::ffh_value`; Mirror lives in Python, so `magic::understood` leaves it to FfH's terms), whatever the situation: the copy stands through the other players' turns and expires at the start of ours (`unit_do_turn`), so it never attacks.

### Crown of Akharien (`EQUIPMENT_CROWN_OF_AKHARIEN`)
*Item · not trainable · carried as the Crown of Akharien promotion: immune to magic · also the wonder of the same name (Arcane Lore, 900 hammers, world limit 1)*

**What it does.** An item that is also a world wonder (see Athame for items). Built as the wonder `BUILDING_CROWN_OF_AKHARIEN` (+2 research, +2 culture, +100% research, 2 Great Sage points), it can be lifted out of its city by a unit there (Take Crown of Akharien (building), which removes the wonder and gives the promotion) and set down again in one of the owner's cities (Drop Crown of Akharien, which builds the wonder there); the item unit itself can also construct it. Carried, it makes the bearer immune to magic. Razing a city that holds the wonder leaves the item to the razer (`player/cities.rs`).

**Playing it well.** Usually it is a research wonder: build it in the best science city and leave it there. Lift it only to move it to a better city, or to make a key unit (a hero facing enemy mages) immune to magic for a war, and drop it back afterwards: carried, the +100% research is lost.

**AI: Not an AI choice.** The AI builds the wonder for its research and culture (the generic building value; buildings entry). Take (building) is `bAllowAI` 0, so the AI never lifts it. A unit of ours ending the turn on the item unit takes it (`casting::ffh_value`: minimum 1, no AI weight; magic immunity is not valued), and a bearer in one of our cities drops it at the end of the turn, rebuilding the wonder, because Drop is valued by the building's value there (`casting::ffh_value`, `production::building_value`), far above the 1 the promotion is worth. The item unit never constructs it (it holds, `units::role_of`: Other).

### Crown of Command (`EQUIPMENT_CROWN_OF_COMMAND`)
*Item · not trainable · carried as the Crown of Command promotion: 30% chance to capture a defeated unit · Stephanos's own item*

**What it does.** An item (see Athame). Its bearer has a 30% chance to capture the unit it defeats in combat (`iCombatCapturePercent`, `unit/combat.rs`): the loser becomes the winner's unit at 75% damage, keeping its promotions. It enters play when Stephanos, the first Horseman of the Apocalypse (barbarian, Armageddon counter 40), dies.

**Playing it well.** Put it on your best attacker in a war against strong units: every third win or so adds an enemy unit to your army, which pays most against expensive units: champions, knights, beasts. Units immune to capture (Loyalty, many heroes) and immortal ones never join you.

**AI: Not an AI choice.** Picked up by a unit of ours that ends the turn on it, at the minimum value (`units::promotion_value` does not read the capture chance), never fetched, never passed on (Take (promotion) has no AI weight). The captures happen in the rules whenever its bearer wins; the AI does not pick that bearer's fights for them.

### Empty Bier (`EQUIPMENT_EMPTY_BIER`)
*Item · not trainable · carried as the Empty Bier promotion: +2 defense strength, double fortification bonus · Donal Lugh's own item*

**What it does.** An item (see Athame). Its bearer gets +2 defense strength and twice the fortification bonus. It drops when Donal Lugh, the Bannor hero who carries it, dies.

**Playing it well.** A defender's item: give it to the best garrison unit of a threatened city, where the fortification bonus is doubled every turn it stays put. On an attacker it is nearly wasted.

**AI: Not an AI choice.** A unit of ours ending the turn on it takes it (`casting::ffh_value`: `iAIWeight` 50 + `units::promotion_value`, which reads the promotion's own weight of 100 and the +2 defense, at 40 a point for a garrison unit and 15 for others); a stronger unit in the stack may take it over (Take (promotion), weight 50, AI gate: strength 2 above the holder's; docs/AI_ONLY_RULES.md, A2), but not from Donal, who has it for free. The AI does not route it to a city defender; the doubled fortification is not valued.

### Gela (`EQUIPMENT_GELA`)
*Item · not trainable · carried as the Gela promotion: +2 unholy strength · Hyborem's own item*

**What it does.** An item (see Athame). Its bearer gets 2 unholy strength. It drops only when Hyborem, the Infernal hero who carries it, truly dies (an immortal Hyborem is reborn in his capital and keeps it).

**Playing it well.** Two points of unholy damage suit a fast attacker; holy and unholy-resistant targets (angels, demons, undead) shrug part of it off.

**AI: Not an AI choice.** Taken by a unit of ours ending the turn on it (`casting::ffh_value`: weight 50 + the promotion's own weight 100 + 2 unholy x 40 in `units::promotion_value`), and passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate), never taken from Hyborem. Never fetched.

### Godslayer (`EQUIPMENT_GODSLAYER`)
*Item · not trainable, invisible · carried as the Godslayer promotion: +3/+3, 100% magic and damage-type resistance, +1000% against the Avatar · given to a rival when the Illians complete Ascension*

**What it does.** An item (see Athame). When Auric Ascends (the Illian ritual Ascension), the rules give the Godslayer to the non-barbarian player of another team ranked best by a random draw (+2000 for a human, +50 per rank above 20th; `ffh/events.rs`, `ev_ascension`): a human gets it through an event that hides it on a remote plot (`random_events_a.rs::do_godslayer`, `cf_place_treasure`), an AI in its capital (docs/AI_ONLY_RULES.md, A4). Its bearer gets +3 attack and defense, full resistance to magic and to every damage type, and +1000% against units with the Avatar promotion (Auric Ascended, the only one); if Auric Ascended wins against its bearer he dies anyway (`spells_a.rs::post_combat_auric_ascended_won`).

**Playing it well.** Hand it to a fast unit and go for Auric Ascended before the Illians use him: the +1000% makes the fight a formality, and killing him strips the Illians of their leader traits. Until then it makes any unit a strong, spell-proof defender.

**AI: Not an AI choice.** In an AI's capital the first garrison unit to cast takes it at the end of the turn (`casting::ffh_value`: weight 50 + the promotion's weight 100 + 3 strength x 40 + 3 defense at 40 for a garrison unit), a stronger unit in the city takes it over (Take (promotion), weight 50, AI gate). Nothing then sends that unit after Auric Ascended: the +1000% against the Avatar and the resistances are not valued (`units::promotion_value`), and the bearer stays a garrison unit.

### Golden Hammer (`EQUIPMENT_GOLDEN_HAMMER`)
*Item · not trainable · carried as the Golden Hammer promotion: +1 strength; or given to the city as a free Engineer specialist · one in every Luchuirp city after Gifts of Nantosuelta*

**What it does.** Gifts of Nantosuelta, the Luchuirp world spell, puts a Golden Hammer in each of the caster's cities (`spells_a.rs::spell_gifts_of_nantosuelta`). Each can either be picked up as an item (see Athame) giving +1 strength, or used in its city with Give Hammer to Craftsman (`SPELL_GIVE_HAMMER_TO_CRAFTSMAN`: the hammer is consumed and the city gets a permanent free Engineer specialist, `spells_a.rs::spell_give_hammer_to_craftsman`).

**Playing it well.** Almost always give it: a free Engineer in every city (+2 production and 3 Great Engineer points a turn each, for the rest of the game) is worth far more than +1 strength on one unit. Cast Gifts once you have most of your cities, and give the hammers in production cities first if any must wait.

**AI: Directed.** `civs/luchuirp.rs` (`spell_value`) zeroes Take Golden Hammer (unit) for a unit standing in one of our own cities, so the hammer gives itself at the end of the turn (Give Hammer has `iAIWeight` 20 and costs nothing by FfH's terms, `casting::ffh_value`): measured, production +11.8% at turn 300 and great people +47% in the games where the Luchuirp cast Gifts (docs/AI.md, "Luchuirp"; before it the garrison warriors took 59 of 72 hammers). A hammer that falls to another civilization is taken for its +1 strength by that civilization's units (the take is worth 90 to them, the give 20). The hammer unit itself has the great-person role (`units::role_of`: `UNITAI_ENGINEER`) and simply holds, being immobile.

### Healing Salve (`EQUIPMENT_HEALING_SALVE`)
*Item · not trainable · carried as the Healing Salve promotion, used up by the Healing Salve ability · from lairs and the Alchemist event*

**What it does.** An item (see Athame) that is a single use: its bearer can cast Healing Salve (`spells_a.rs::spell_healing_salve`), which heals it completely and cures Diseased, Plagued, Poisoned and Withered, then removes the promotion. Sources: exploring a lair (`custom.rs::cf_explore_lair_good`), opening a treasure chest, and the Alchemist event, which gives the promotion to a unit directly.

**Playing it well.** A free full heal for a key unit (a hero, a high-level attacker) in the middle of a campaign, best used right after a fight that left it badly hurt and far from a city; it also clears plague and withering, which ordinary healing is slow to fix.

**AI: Not an AI choice.** An AI almost never gets one: it cannot explore lairs (the Explore Lair spells are `bAllowAI` 0 and an AI unit entering a lair destroys it; docs/AI_ONLY_RULES.md, A4) nor open chests. One it does hold is picked up by a unit ending the turn on it (weight 50 + the minimum promotion value), and a bearer drinks it at the end of the first turn it has 25 damage or more (the rules' AI gate in `req_healing_salve`; FfH's weight 100, `casting::ffh_value`), wherever it is.

### Infernal Grimoire (`EQUIPMENT_INFERNAL_GRIMOIRE`)
*Item · not trainable · carried as the Infernal Grimoire promotion, which casts Read the Grimoire · also the wonder of the same name (Infernal Pact, 400 hammers, world limit 1)*

**What it does.** An item that is also a world wonder (see Athame and Crown of Akharien). The wonder (1 free tech, 1 Great Sage point; a 20% chance on completion that a barbarian Balor appears beside the city) can be taken out of its city as the promotion, whose bearer can cast Read the Grimoire (`spells_b.rs::spell_read_the_grimoire`): a random result each time, 20% Summon Spectre, 20% Unholy Taint, 20% Wither, 10% Summon Pit Beast, 10% Burning Blood, 10% the Demon promotion, 10% the reader dies.

**Playing it well.** As a wonder it is a free tech for 400 hammers; as an item it is a gamble that summons a Spectre or Pit Beast half the time and kills its bearer one time in ten, so read it with a cheap unit, never a hero.

**AI: Unused.** All three Take spells and Read the Grimoire are `bAllowAI` 0 (docs/AI_ONLY_RULES.md, D), so an AI never takes it from the wonder, never picks the item up, never reads it. It can only build the wonder for its generic value (the Great Sage point; the free tech is invisible to it, buildings entry).

### Jade Torc (`EQUIPMENT_JADE_TORC`)
*Item · not trainable · carried as the Jade Torc promotion: 20% magic resistance, immune to poison · from lairs and treasure chests*

**What it does.** An item (see Athame). Its bearer resists hostile spells 20% more often and takes no poison damage. It comes from exploring a lair (`custom.rs::cf_explore_lair_big_good`) or opening a treasure chest (`spells_b.rs::spell_open_chest`, 1 chance in 7).

**Playing it well.** A defensive charm for a hero or caster that will face enemy mages or poison users (scorpions, spiders, the Scorpion Clan, assassins with Poisoned Blade).

**AI: Not an AI choice.** An AI gets one only by capturing it: it never explores lairs or opens chests (A4). If held, it is picked up by a unit ending the turn on it (weight 50 + the minimum promotion value: magic resistance and poison immunity are not valued) and passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate).

### Nether Blade (`EQUIPMENT_NETHER_BLADE`)
*Item · not trainable · carried as the Nether Blade promotion: +2 death strength, +80% against heroes · Rathus Denmora's own item*

**What it does.** An item (see Athame). Its bearer gets 2 death strength and +80% against units with the Hero promotion. It drops when Rathus Denmora, the Sidar hero who carries it, dies.

**Playing it well.** The hero-killer: give it to a strong, fast attacker and hunt enemy heroes, which are usually the hardest single units in a stack; death damage does nothing to undead and demons.

**AI: Not an AI choice.** Picked up by a unit ending the turn on it (weight 50 + 2 death x 40), passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate), never taken from Rathus. The +80% against heroes is not valued (`units::promotion_value`) and nothing sends the bearer after heroes.

### Orthus's Axe (`EQUIPMENT_ORTHUSS_AXE`)
*Item · not trainable · carried as the Orthus's Axe promotion: Blitz, +1 fire strength · Orthus's own item*

**What it does.** An item (see Athame). Its bearer gets Blitz (it may attack several times a turn) and 1 fire strength. It drops when Orthus, the barbarian hero who appears around turn 75 (normal speed, `ffh/events.rs`, `ev_begin_game_turn`), is killed.

**Playing it well.** Blitz on a strong attacker is the prize: one unit can clear several weak stacks or finish off damaged defenders in one turn. Kill Orthus with a unit that can step onto his plot afterwards to capture the axe at once.

**AI: Not an AI choice.** Picked up by a unit ending the turn on it (weight 50 + 1 fire x 40; Blitz is not valued), passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate). The AI's stacks do not use Blitz for repeated attacks (`campaign::strike` drops units that have attacked; docs/AI_LOG.md, Warcry).

### Pieces of Barnaxus (`EQUIPMENT_PIECES_OF_BARNAXUS`)
*Item · not trainable · carried as the Pieces of Barnaxus promotion, which rebuilds Barnaxus in a Luchuirp city · Barnaxus's own item*

**What it does.** An item (see Athame) with no combat effect. When Barnaxus, the Luchuirp golem hero, is destroyed, his pieces drop. A unit carrying them into a Luchuirp city can cast Rebuild Barnaxus (`spells_b.rs::spell_rebuild_barnaxus`): Barnaxus is rebuilt for the city's owner at 75% damage with the city's production experience, and if the carrier is not Luchuirp, the Luchuirp like it 2 attitude points more.

**Playing it well.** For the Luchuirp, carry the pieces home and rebuild their hero at once. For anyone else at peace with the Luchuirp, returning them buys goodwill; at war, keep them out of Luchuirp cities.

**AI: Unused.** A unit ending the turn on the pieces picks them up (weight 50), but Rebuild Barnaxus is never cast: by FfH's terms it is worth only minus the promotion it removes (`casting::ffh_value`; it is an ability, so `magic.rs` leaves it alone), and `civs/luchuirp.rs` does not value it. A Luchuirp AI never gets its hero back this way.

### Potion of Invisibility (`EQUIPMENT_POTION_OF_INVISIBILITY`)
*Item · not trainable · carried as the Potion of Invisibility promotion, drunk for Hidden · from lairs*

**What it does.** An item (see Athame), single use: drinking it (`SPELL_POTION_OF_INVISIBILITY`) removes the potion and gives the Hidden promotion: invisible to other teams and +1 first strike, until the unit fights or casts. It comes from exploring lairs.

**Playing it well.** Drink it to slip a unit past a defensive line, to scout a city, or to move a hero to the front unseen; the first strike also helps the opening attack, after which it is gone.

**AI: Not an AI choice.** An AI gets one only by capturing it (it never explores lairs, A4). Its bearer drinks it at the end of the first turn it holds it, wherever it is, because Hidden is worth its weight and first strike to `units::promotion_value` (`casting::ffh_value`, also `casting::buff_cast`); nothing times it before a move or an attack.

### Potion of Restoration (`EQUIPMENT_POTION_OF_RESTORATION`)
*Item · not trainable · carried as the Potion of Restoration promotion, drunk to cast again in the same turn · from lairs*

**What it does.** An item (see Athame), single use: drinking it (`SPELL_POTION_OF_RESTORATION`, castable even after the unit has cast) removes the potion and the unit's "has cast" flag, so a caster can cast a second spell that turn. It comes from exploring lairs.

**Playing it well.** Give it to your strongest caster and drink it on the turn a second spell decides a fight: two Fireballs or Maelstroms before an assault, or a summon and a blessing.

**AI: Unused.** It is picked up like any item (weight 50), but never drunk: by FfH's terms the spell is worth only minus the promotion it removes (`casting::ffh_value`; an ability, outside `magic::understood`).

### Rod of Winds (`EQUIPMENT_ROD_OF_WINDS`)
*Item · not trainable · carried as the Rod of Winds promotion: +1 lightning strength per Air mana its owner has · from lairs and treasure chests*

**What it does.** An item (see Athame). Its bearer gets an affinity for Air mana: 1 lightning strength for every Air mana resource its owner has (`unit/ffh.rs`, `unit_update_bonus_affinity`). It comes from lairs and chests (1 chance in 7).

**Playing it well.** Worth carrying for a civilization with several Air mana nodes; with none it does nothing. Lightning damage is resisted by few units.

**AI: Not an AI choice.** An AI gets one only by capturing it. A unit ending the turn on it picks it up (weight 50 + the minimum value: affinity is not valued by `units::promotion_value`), whoever it is, and a stronger unit in the stack takes it over (AI gate); the AI neither steers it to a caster nor builds Air nodes for it.

### Scorched Staff (`EQUIPMENT_SCORCHED_STAFF`)
*Item · not trainable · carried as the Scorched Staff promotion: +1 fire strength per Fire mana its owner has · from the Meteor event*

**What it does.** An item (see Athame). Its bearer gets an affinity for Fire mana: 1 fire strength for every Fire mana resource its owner has. It comes from the Meteor event (option 2, for 50 gold, offered only to a player with Enchantment mana): the staff appears in the event's city.

**Playing it well.** For a civilization with Fire mana nodes (the Sheaim, the Balseraphs, a Fire-focused mage empire), it turns one unit into a fire striker; without Fire mana, sell the stone instead.

**AI: Not an AI choice.** The event choice (`events::value`) counts the free staff as a 600-gold unit (its production cost is -1), so an AI with Enchantment mana takes it over the 50 gold. The first unit in that city to cast at the end of the turn then picks it up (weight 50 + the minimum value), usually a garrison unit; the Fire-mana affinity is not valued.

### Staff of Souls (`EQUIPMENT_STAFF_OF_SOULS`)
*Item · not trainable · carried as the Staff of Souls promotion: +3 death strength, +1 death per Death mana its owner has · Ars Moriendi's own item*

**What it does.** An item (see Athame). Its bearer gets 3 death strength and an affinity for Death mana (+1 death strength per Death mana resource). It enters play when Ars Moriendi, the third Horseman of the Apocalypse (barbarian, counter 70), is killed.

**Playing it well.** One of the strongest items for an attacker of an evil civilization with Death mana; death damage is useless against undead and demons, which are common late in an Armageddon game.

**AI: Not an AI choice.** Picked up by a unit ending the turn on it (weight 50 + 3 death x 40; the affinity is not valued), passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate).

### Syliven's Perfect Lyre (`EQUIPMENT_SYLIVENS_PERFECT_LYRE`)
*Item · not trainable · carried as the Syliven's Perfect Lyre promotion (no combat effect) · also the wonder of the same name (Drama, 900 hammers, world limit 1)*

**What it does.** An item that is also a world wonder, handled like the Crown of Akharien: the wonder (+4 culture, +100% culture, 2 Great Bard points) can be lifted out as the promotion (Take (building)), carried, and dropped in another city of the owner to rebuild it (Drop); razing its city leaves the item to the razer. Carried, it does nothing.

**Playing it well.** Build it in the city you want to push to legendary culture; lift it only to move the culture bonus to a better city or to save it from a city about to fall.

**AI: Not an AI choice.** The AI builds the wonder for its culture (generic value; culture victory pursuit, `victory::production`), never lifts it (Take (building) is `bAllowAI` 0), picks up the item unit when one of its units ends the turn on it (weight 50), and a bearer in one of our cities drops it there, rebuilding the wonder (Drop valued by the building's value, `casting::ffh_value`). It does not choose the city.

### The Dragon's Hoard (`EQUIPMENT_DRAGONS_HORDE`)
*Item · not trainable · carried as the Dragon's Hoard promotion (no combat effect) · also the building placed in Acheron's city (+4 gold, +8 culture, free Gems and Gold, Enchanted Blade for its units)*

**What it does.** When Acheron, the Red Dragon, appears in a barbarian city, the rules put the Dragon's Hoard in it (`ffh/events.rs`, `ev_unit_built`); in a barbarian city it also spawns a Disciple of Acheron (a Son of the Inferno once the barbarians know Sorcery) at the lair spawn rate each turn. The building gives +4 gold, +8 culture, a free Gems and a free Gold resource, and the Enchanted Blade promotion (+20% strength, dispellable) to melee units built in or passing through the city. Like the Crown of Akharien it can be lifted out as an item (Take (building)), carried, and dropped in a city of the owner (Drop); razing its city leaves the item to the razer.

**Playing it well.** Kill Acheron and take his city rather than razing it: the hoard's gold, culture and two luxury resources stay with the city, and the stream of fire adepts stops. Lift it only to move it to a safer or more central city.

**AI: Not an AI choice.** An AI that conquers the city keeps the building; it never lifts it (Take (building) is `bAllowAI` 0). Its item unit is picked up by a unit ending the turn on it (no AI weight: the minimum value), and a bearer in one of our cities drops it at once (Drop valued by the building's value: the gold and culture, not the resources or the promotion, `casting::ffh_value`).

### Timor Mask (`EQUIPMENT_TIMOR_MASK`)
*Item · not trainable · carried as the Timor Mask promotion: Fear, immune to fear · from lairs and treasure chests*

**What it does.** An item (see Athame). Its bearer causes Fear and is itself immune to it: an attacker that is not immune may lose its attack against the bearer (the odds rise with the bearer's strength and level over the attacker's), and when the bearer wins outside a city the other enemy units on the plot may flee (`unit/combat.rs`). It comes from lairs and chests (1 chance in 7).

**Playing it well.** On a strong attacker in the field, each win can scatter the rest of the enemy stack; on a city or hill defender, attackers balk and lose their turn; and immunity keeps the bearer itself fighting against demons and undead with Fear.

**AI: Not an AI choice.** An AI gets one only by capturing it. A unit ending the turn on it picks it up (weight 50 + the minimum value: fear is not valued), and a stronger unit in the stack takes it over (AI gate).

### War (`EQUIPMENT_WAR`)
*Item · not trainable · carried as the War promotion: +3 unholy strength, +1 to the Armageddon counter per combat · Buboes's own item*

**What it does.** An item (see Athame). Its bearer gets 3 unholy strength, and every combat it fights adds 1 to the Armageddon counter (`iModifyGlobalCounterOnCombat`; the counter brings the other Horsemen, Blight, Pestilence and finally Armageddon). It enters play when Buboes, the second Horseman of the Apocalypse (barbarian, counter 50), is killed.

**Playing it well.** For an evil civilization that wants Armageddon, a fighting bearer pushes the counter; a good civilization should keep it off the battlefield (or on a garrison unit that rarely fights), since every fight brings the next disaster closer.

**AI: Not an AI choice.** Picked up by a unit ending the turn on it (weight 50 + 3 unholy x 40), passed to a stronger unit in the stack (Take (promotion), weight 50, AI gate); its effect on the Armageddon counter is not weighed by anything in the AI, whatever the civilization's alignment.

## Animals

### Baby Spider (`UNIT_BABY_SPIDER`)
*Animal · not trainable · 2/2, 1 move, +1 poison, -25% city attack · 2% chance to grow into a Giant Spider on a win · born from Giant Spiders' victories*

**What it does.** A weak poisonous spider. It is never spawned on its own: a Giant Spider that wins a fight has a 5% chance to create one beside it (`UnitCreateFromCombat`), and lair exploration can call one as a henchman (`custom.rs::cf_explore_lair_big_bad`). Each win gives it a 2% chance to become a Giant Spider. Like every animal it fights with the handicap's animal modifier, gets no defensive bonuses, and as a barbarian may not enter owned land, nor plots with a resource or improvement except to attack (`unit/movement.rs`).

**Playing it well.** Free experience for scouts and warriors; kill spider nests early, before baby spiders grow into Giant Spiders in your jungles. A Hunter with Subdue Animal captures it.

**AI: Not an AI choice.** Animals belong to the barbarians and are played by `units::barbarian` (role Animal, `units::role_of`): attack an adjacent enemy at 50% odds or better (a city counts extra), else walk toward a visible enemy within 4 turns whose power is at most 80% of its own, else wander to a pseudo-random reachable plot; they cast through FfH's terms (`casting::end_turn`; `magic.rs` is off for the barbarians, `magic::enabled`). Civilizations capture animals with Subdue Animal (recon, Animal Husbandry; its weight of 300 makes the AI take it, `units::promotion_value`). A captured animal keeps its AI type (`unit_convert`), so an AI's spider is still played by `units::barbarian`: it roams and fights alone, never garrisons or joins a stack.

### Bear (`UNIT_BEAR`)
*Animal · not trainable · 5/5, 1 move, -25% city attack · Create Den · native to forests · spawned by Bear Dens*

**What it does.** A strong early animal of the forests (`game/world.rs::create_animals` picks among the barbarian animals native to the plot's feature or terrain, on unowned land no civilization sees). Alone on an unowned, unimproved plot with no other bear den within 4 plots and undamaged, it can cast Create Den (`spells_a.rs::spell_create_den`): the plot becomes a Bear Den, a lair that spawns new bears now and then (`plot/terrain.rs`, at the handicap's lair spawn rate, while the area has few barbarians). A captured bear can be turned into a Dancing Bear (+1 happiness, +3 culture in a city, `MISSION_CONSTRUCT`). The Nature's Revolt ritual turns barbarian workers, warriors, scouts, hunters and axemen into bears, lions, tigers and wolves.

**Playing it well.** Early on a bear beats warriors and scouts: move around forests with care, clear dens before they multiply, and catch one with a Subdue Animal hunter for the Dancing Bear.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider); den-spawned bears get `UNITAI_ATTACK` and go through `military::army`, which for the barbarians is the same routine. The barbarians cast Create Den whenever they can (`iAIWeight` 300, `casting::ffh_value`). A rules quirk the AI walks into: Create Den converts its caster into a new barbarian bear, so an AI's captured bear that ends a turn alone on unowned forest casts it too (FfH's weight 300) and is lost to the barbarians with a new lair. The AI never builds the Dancing Bear (it issues `MISSION_CONSTRUCT` only for shrines, `units::great_person_build`).

### Elephant (`UNIT_ELEPHANT`)
*Animal · not trainable · 8/8, 1 move, can only defend · native to grassland and plains · upgrades to War Elephant*

**What it does.** The strongest early animal, but defensive only: it never attacks, it just blocks the way. A captured elephant upgrades to the War Elephant (7/7, 2 moves), the only animal with an upgrade.

**Playing it well.** Walk around it; capture one with a Subdue Animal hunter only if you can upgrade it, since an 8-strength defender that cannot attack is of little use in the field (it can garrison a city).

**AI: Not an AI choice.** For the barbarians `units::barbarian` does nothing with it but wander (its attack power is 0, so it neither attacks nor hunts). An AI's captured elephant is upgraded to the War Elephant when the rules allow it and gold suffices (`units::upkeep`, the best upgrade by attack power); before that it wanders like a barbarian animal.

### Giant Spider (`UNIT_GIANT_SPIDER`)
*Animal · not trainable · 4/4, 1 move, +1 poison, 1 first strike, invisible · native to jungle and forests · 5% chance to spawn a Baby Spider on a win*

**What it does.** An invisible ambusher of jungles and forests: most units cannot see it until it attacks. Wins can spawn Baby Spiders. The Spider Mine event can hide one (barbarian, hidden nationality) on a mine plot (`random_events_a.rs::do_spider_mine3`). A captured one can become a Spider Pen (a free Silk resource, needs Animal Handling).

**Playing it well.** Escort workers and settlers through jungle; units that see invisible (recon units, Sentry) find them. A captured spider is worth the Spider Pen's Silk.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider). The event choice prices the Spider Mine's hostile spider at -150 gold (`events::python_value`). The AI never builds the Spider Pen.

### Giant Tortoise (`UNIT_GIANT_TORTOISE`)
*Sea animal · not trainable · 6/9, 1 move · native to ocean*

**What it does.** A sea animal of the open ocean, spawned in water areas (a fifth as many as on land, none in waters under 20 unowned tiles, `create_animals`). Tough on defense (9).

**Playing it well.** An obstacle for early galleys and triremes: go around it or attack it with a stronger ship.

**AI: Not an AI choice.** A sea unit with no transport or exploring role is Naval for `units::role_of`, and Naval units hold (`units::hold`: fortify or sleep), barbarian ones included: the tortoise never moves or attacks.

### Gorilla (`UNIT_GORILLA`)
*Animal · not trainable · 4/4, 1 move, -25% city attack · native to jungle*

**What it does.** A jungle animal. A captured one becomes a Gorilla Cage (+1 happiness, +3 culture) in a city with a Carnival; the Hunter event can sell one; lair exploration in Ruins can call one as a henchman.

**Playing it well.** As the Bear: a danger to early scouts, experience for warriors, a cage for a Carnival city.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider). The AI never builds the cage.

### Griffon (`UNIT_GRIFFON`)
*Animal · not trainable · 4/4, 1 move, Flying, +50% against mounted units · native to grassland and plains*

**What it does.** A flying animal: the Flying promotion gives +1 move and lets it cross water and impassable terrain at a flat cost of one move a plot (`unit/movement.rs`). It has +50% against mounted units and, unlike the other animals, no city attack penalty. Lair exploration can call one as a henchman.

**Playing it well.** Keep horsemen away from it; spearmen and archers handle it.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider).

### Hill Giant (`UNIT_HILL_GIANT`)
*Beast (animal) · not trainable · 7/7, 1 move, bombards city defenses 15% a turn, Sentry · native to desert · from the Pig Giant event and the Pact of the Nilhorn*

**What it does.** A beast that counts as an animal (it spawns with the animals in deserts and, as a barbarian, cannot enter owned land), strong for its era and able to bombard city walls. It is captured only with Subdue Beasts (Combat III, Animal Mastery), not Subdue Animal. The Pig Giant event either gives one to the player (option 1, for a pig resource) or sends a barbarian giant with Commando to raid the city (option 3); the Pact of the Nilhorn project (Cartography, 500 hammers) gives its builder three, Larry, Curly and Moe, with hidden nationality.

**Playing it well.** Three hidden-nationality giants are raiders that can hit anyone without a war declaration; a captured or gifted giant is a solid early attacker that also bombards.

**AI: Not an AI choice.** Barbarian giants are played by `units::barbarian` (the raider from the event is `UNITAI_ATTACK_CITY_LEMMING`, so `military::army`, the same routine for the barbarians). The event choice prices the raider at -250 gold (`events::python_value`) and counts a gifted giant as a 600-gold unit. A giant the AI owns (event, Pact of the Nilhorn, which `production::best_project` takes only when no building has value) keeps `UNITAI_ANIMAL` and is played by `units::barbarian` like an animal; the hidden nationality is not used to raid.

### Lion (`UNIT_LION`)
*Animal · not trainable · 3/3, 1 move, -25% city attack · native to grassland, plains and desert · 10% chance to grow into a Lion Pride on a win · spawned by Lion Dens*

**What it does.** The common animal of open land. Winning fights may turn it into a Lion Pride, which can found a Lion Den. A captured one becomes a Lion Cage (+1 happiness, +3 culture) in a city with a Carnival; the Hunter event sells one for 15 gold.

**Playing it well.** Kill lions while they are lions: prides make dens, dens make more lions.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider). The AI never builds the cage.

### Lion Pride (`UNIT_LION_PRIDE`)
*Animal · not trainable · 4/4, 1 move, -25% city attack · Create Den (Lion Den) · from Lions' wins*

**What it does.** A grown lion. Like the Bear, alone on unowned, unimproved land with no Lion Den within 4 plots, it can cast Create Den to make a Lion Den, a lair that spawns lions. A captured pride can also become a Lion Cage.

**Playing it well.** A priority target near your borders: each pride left alone is a future lair.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider); the barbarians cast Create Den whenever they can (weight 300). An AI's captured pride has the Bear's quirk: alone on unowned land, it casts Create Den and turns barbarian.

### Polar Bear (`UNIT_POLAR_BEAR`)
*Animal · not trainable · 5/5, 1 move, -25% city attack, Winterborn · Create Den (Bear Den) · native to snow and forests*

**What it does.** The bear of the snow, with Winterborn (+10% attack and defense on snow and tundra, some cold resistance). It is in the Bear's class (so it can found Bear Dens, which spawn ordinary Bears) and enters play through its own class, `UNITCLASS_POLAR_BEAR`, whose default unit it is. Lair exploration on snow can call one as a henchman. It can become a Dancing Bear.

**Playing it well.** As the Bear, in the north.

**AI: Not an AI choice.** As the Bear (see Bear), including the Create Den quirk for a captured one.

### Scorpion (`UNIT_SCORPION`)
*Animal · not trainable · 4/4, 1 move, +1 poison, 1 first strike, -25% city attack · native to desert*

**What it does.** A desert animal with poison and a first strike, harder than its strength suggests for early units. Lair exploration can call one as the monster or a henchman.

**Playing it well.** Bring two units or a hunter into the desert early; poison-immune units ignore its extra damage.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider).

### Sea Serpent (`UNIT_SEA_SERPENT`)
*Sea animal · not trainable · 8/8, 1 move · native to ocean*

**What it does.** The strongest sea animal, spawned in large unowned waters (see Giant Tortoise) and called by lair exploration on water.

**Playing it well.** It outclasses early ships: keep triremes and settler boats out of its reach, or attack it with several ships.

**AI: Not an AI choice.** Like every barbarian sea unit, it is Naval for `units::role_of` and holds: it never moves or attacks (`units::hold`).

### Tiger (`UNIT_TIGER`)
*Animal · not trainable · 4/4, 2 moves, -25% city attack · native to jungle · also summoned for good by Summon Tiger*

**What it does.** A fast jungle animal. Fellowship of Leaves priests with Channeling II and Divine summon tigers for good (Summon Tiger, a permanent summon). A captured one becomes a Tiger Cage in a city with a Carnival; the Hunter event sells one.

**Playing it well.** Its two moves catch workers in the open; a summoned tiger is a cheap early attacker and lair-clearer.

**AI: Not an AI choice.** Barbarian tigers are played by `units::barbarian` (see Baby Spider). A summoned tiger of a civilization is an ordinary army unit afterwards (`magic::raised`: spent right after the collateral attackers in a strike, `campaign::strike`); one captured or bought keeps `UNITAI_ANIMAL` and roams like a barbarian animal.

### Wolf (`UNIT_WOLF`)
*Animal · not trainable · 2/2, 2 moves, -25% city attack · native to tundra, snow and forest · 10% chance to grow into a Wolf Pack on a win · Doviello: Form Wolf Pack, Add to Wolf Pack*

**What it does.** The first animal most scouts meet: weak but fast. Two Doviello wolves on one plot can Form Wolf Pack (the pair becomes a Wolf Pack with their summed experience, `spells_a.rs::spell_form_wolf_pack`), and a Doviello wolf can Add to Wolf Pack (it dies and the pack gains the next Empower promotion, +10% strength each, up to five). The Doviello's Wild Hunt gives a wolf for every unit with strength, stronger beside strong units (base strength 2 + half the unit's strength above 2, `spells_b.rs::spell_wild_hunt`). A captured wolf becomes a Wolf Pen (+2 culture) in a Carnival city; the Hunter event sells one for 10 gold. A Scorpion Clan Goblin that kills a wolf becomes a Wolf Rider.

**Playing it well.** For the Doviello, merge Wild Hunt's wolves into packs and empower them, rather than leave a crowd of 2-strength units.

**AI: Not an AI choice.** Barbarian wolves are played by `units::barbarian` (see Baby Spider). Wild Hunt's wolves keep `UNITAI_ANIMAL`, so the Doviello AI plays them the same way: each attacks, hunts or wanders alone (making them army units in the stacks was tried and not kept, `civs/doviello.rs`). It merges and empowers them at the end of the turn whenever two meet (`casting::ffh_value`: both spells weigh 100), by chance rather than by gathering them. In the Hunter event the AI buys the wolf: at 10 gold it is the cheapest of the animals, each counted as a 600-gold unit (`events::value`); it then roams like a barbarian animal.

### Wolf Pack (`UNIT_WOLF_PACK`)
*Animal · not trainable · 3/3, 2 moves, -25% city attack · 10% chance to create a Wolf on a win · from Wolves' wins and Form Wolf Pack*

**What it does.** A grown wolf, a "mobile lair": its wins may create new wolves beside it. Doviello packs take Empower promotions from Add to Wolf Pack.

**Playing it well.** Kill it before it breeds; for the Doviello, an empowered pack is a fast raider that replenishes itself.

**AI: Not an AI choice.** Played by `units::barbarian` (see Baby Spider); the Doviello's packs are formed and empowered as described under Wolf.

## Barbarian, event and special units

### Angel (`UNIT_ANGEL`)
*Melee, tier 3 · not trainable · 4/4, 1 move, +1 holy, Angel race · Mercurians only, given by the rules · upgrades at levels 3-6 to the Repentant Angel, Herald, Ophanim, Angel of Death, Seraph, Valkyrie*

**What it does.** The Mercurians' only source of their strong units. The rules give Angels (`custom.rs::cf_gift_unit`, in a random Mercurian city): six when the Mercurian Gate brings the Mercurians in; one, with the dead unit's experience, when a unit of the Empyrean, the Order or the Runes of Kilmorph dies, or any living non-animal Mercurian unit; and, when a city founded by a good civilization is razed, one try per two citizens plus one (per four citizens plus one for a neutral founder; `ffh/events.rs`, `ev_city_razed`). Each gift succeeds with 100% minus 3% per living civilization plus the experience carried, between 5% and 95%. The Angel race is not alive: immune to fear, holy and poison damage, half death damage. An Angel can be sacrificed at the Pyre of the Seraphic (Sacrifice).

**Playing it well.** Keep Angels alive until level 4, then upgrade them into the unit the war needs (Heralds for raw strength, Ophanim to fly over water, Seraphs against stacks, Angels of Death to raid); good units dying in your service refill the supply, so a long war of attrition works for the Mercurians.

**AI: Not an AI choice.** The rules hand them out; the AI plays them as army units (`units::role_of`: `UNITAI_ATTACK`, `military::army`) and upgrades them when gold allows (`units::upkeep`), choosing the upgrade with the highest attack strength: the Repentant Angel (16) where allowed, else the Herald (14), Ophanim (12), Angel of Death or Seraph (11), Valkyrie (9). It never sacrifices them (Sacrifice is `bAllowAI` 0), and no Mercurian logic keeps Angels out of fights until they level (`civs/mercurians.rs` handles only their settler).

### Angel of Death (`UNIT_ANGEL_OF_DEATH`)
*Recon, tier 4 · upgrade only (min level 4, Guilds), national limit 4 · 8/6, 2 moves, +3 holy, 1 first strike, immune to first strikes · Invisible, Marksman, Vile Touch · Mercurians*

**What it does.** An invisible raider: other teams cannot see it, it attacks the weakest unit of a stack (Marksman), withers what it fights (Vile Touch: Withered, -10% strength, half experience, slower healing), and every win against a unit in a city costs that city a citizen (`spells_a.rs::post_combat_reduce_population`). It can investigate rival cities and move in rival territory without open borders.

**Playing it well.** Use it to pick off workers, lone defenders and wounded units deep in enemy land, and to shrink cities before a siege by attacking their weakest defender each turn. It is fragile on defense (6), so end turns out of reach or inside stacks.

**AI: Incidental.** It comes only as an Angel's upgrade (`units::upkeep`), chosen by attack strength (11) only when the Repentant Angel, Herald and Ophanim are not available. It keeps the Angel's `UNITAI_ATTACK` and fights as an army unit (`military::army`); its invisibility, targeting and population loss are not part of any decision, and its power for the AI counts only its strength and first strike.

### Archer (`UNIT_ARCHER_SCORPION_CLAN`)
*Archery, tier 2 · not trainable · 3/5, 1 move, +1 poison, 1 first strike, +25% city and hills defense, defensive strike · Orcish · the Scorpion Clan's archer (barbarian)*

**What it does.** The Scorpion Clan (the goblins of the Goblin Forts) version of the Archer, with a point of poison and the Orcish race (+10% attack in jungle, -10% work rate, 25% fire resistance). At the start of the game every Goblin Fort gets one (`ffh/events.rs`, unless No Barbarians); a barbarian Scorpion Goblin can become one (Upgrade Archer, barbarians only, needs Archery); and a player not at war with the barbarians can hire one at a Goblin Fort for 60 gold (Hire Archer).

**Playing it well.** A Goblin Fort's archer is a sturdy defender for early units to break; bring two or three attackers. At peace with the barbarians, hiring Scorpion Clan units at a fort is a quick source of troops for gold.

**AI: Not an AI choice.** A barbarian unit. With its default `UNITAI_CITY_DEFENSE` it is a garrison (`units::garrison_unit`): it walks to the barbarian city that most needs a defender, or the nearest one, and holds the fort only when the barbarians have no city. The Hire spells are `bAllowAI` 0, so no AI civilization hires one.

### Ars Moriendi (`UNIT_ARS`)
*Mounted, tier 4 · not trainable, world unit · 13/13, 3 moves, +11 death, +5 unholy · Demon, Fear, Flying, Magic Immune, Vile Touch, Staff of Souls · Call of the Grave · the third Horseman (barbarian, Armageddon counter 70)*

**What it does.** When the Armageddon counter reaches 70, Ars Moriendi appears for the barbarians on an empty land plot of a large area, preferably barbarian land (`random_events_a.rs::do_armageddon_ars`, `custom.rs::cf_add_unit`). Every win over a living unit raises a Wraith beside it, cities it takes are razed (`bAutoRaze`), it ignores spells (Magic Immune), causes Fear, flies over water, and carries the Staff of Souls, which it drops when killed. Call of the Grave deals 40% death damage to every enemy unit within 2 plots, heals damaged undead and turns graveyards into three Wraiths each.

**Playing it well.** Death damage is wasted on undead and demons, so meet it with Undead, Demon or death-resistant units, in a city or on a hill; kill it with several strong attackers in one turn, since every win it gets adds a Wraith. Advance onto its plot to take the Staff of Souls.

**AI: Not an AI choice.** A barbarian unit played by `units::barbarian` (attack an adjacent target at 50% odds or better, cities included; else hunt a visible unit it outpowers within 4 turns; else wander). It never casts Call of the Grave: barbarians cast by FfH's terms, none of which applies to it (`casting::ffh_value`, value 0). Civilizations fight it as any enemy; nothing in the AI prepares for the Horsemen.

### Auric Ulvin (`UNIT_AURIC`)
*Melee, tier 2 · not trainable, world unit · 4/4, 1 move*

**What it does.** The Illian leader in person, a plain 4/4 world unit. No rule, event or spell of the base game creates it (it belongs to the scenarios, which this port does not cover); the Illians' Ascension creates Auric Ascended, a different unit.

**Playing it well.** Not met in a normal game.

**AI: Not an AI choice.** It never appears. Were one given to an AI, its default `UNITAI_CITY_DEFENSE` would make it a garrison (`units::garrison_unit`).

### Avatar of Wrath (`UNIT_WRATH`)
*Melee, tier 4 · not trainable, world unit · 33/33, 2 moves · Demon, Water Walking · +3 to the Armageddon counter · Ira Unleashed · barbarian, Armageddon counter 90*

**What it does.** The last and strongest of the Armageddon creatures: at counter 90 it appears for the barbarians (`random_events_a.rs::do_armageddon_wrath`), and each living land unit of every player that is not a world unit becomes Enraged with the define's chance. While it lives the counter is 3 higher. Ira Unleashed (`spells_a.rs::spell_ira_unleashed`) fills its owner's Ira up to four, beside it.

**Playing it well.** It cannot be beaten in a fair fight; weaken it with spells and collateral first, then throw many units at it in one turn, and keep its Ira from snowballing (they grow with every kill). Walls and many defenders delay it.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi). The barbarians cast Ira Unleashed whenever they have fewer than four Ira (FfH's `iAIWeight` 300, `casting::ffh_value`).

### Azer (`UNIT_AZER`)
*Melee, tier 3 · not trainable · 3/3, 1 move, +3 fire · Elemental, Flying, Immune to Fire · explodes when killed*

**What it does.** A fire spirit. When it dies it explodes (`spells_a.rs::post_combat_explode`): every unit on the adjacent plots takes 10% fire damage, and adjacent forests and jungles may catch fire. Barbarian Azers come from the Azer event (once someone knows Elementalism, on an empty desert or plains plot of the player's land, `random_events_a.rs::do_azer`) and from lair exploration (as the monster or a henchman).

**Playing it well.** Kill it with a unit that is not stacked with others, away from your forests; fire-immune units ignore most of its strength.

**AI: Not an AI choice.** A barbarian unit played by `units::barbarian` (see Ars Moriendi). The AI's attackers do not account for the explosion.

### Blooded Werewolf (`UNIT_WEREWOLF`)
*Beast, tier 3 · not trainable · 8/8, 2 moves · Werewolf · on a win: 30% to create a Ravenous Werewolf, 5% to become a Greater Werewolf*

**What it does.** A werewolf that spreads its curse: when it beats a living unit, it has a 30% chance to create a Ravenous Werewolf of its owner's (`UnitCreateFromCombat`), and 5% to become a Greater Werewolf. A barbarian one is released next to a city by the Werewolf event (option 1, `random_events_a.rs::do_werewolf1`), and lair exploration can call one.

**Playing it well.** Kill werewolves quickly and with strong units: each victory over your living units can add a Ravenous Werewolf to the pack. Undead and other non-living units do not feed it.

**AI: Not an AI choice.** A barbarian unit played by `units::barbarian` (see Ars Moriendi). The AI's event choice avoids releasing it (the option's AI value is -1000, `events::value`).

### Buboes (`UNIT_BUBOES`)
*Mounted, tier 4 · not trainable, world unit · 25/25, 3 moves, +2 unholy · Blitz, Demon, Fear, Flying, Magic Immune, War · Rage · the second Horseman (barbarian, Armageddon counter 50)*

**What it does.** The Horseman of war: appears for the barbarians at counter 50 (`random_events_a.rs::do_armageddon_buboes`), attacks several times a turn (Blitz), flies, causes Fear, ignores spells, razes the cities it takes, and carries War (+3 unholy, +1 to the counter for each fight), which it drops when killed. Rage puts Burning Blood (+20% strength, +1 move, Blitz, but a 20% chance to betray) on the living units within one plot that fail to resist; the spell does not spare its own side.

**Playing it well.** Its 25 strength beats nearly everything at that stage; defend in cities with walls and many units, and attack it only with a large stack in one turn. Leaving it alive costs a city a turn.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi); the barbarians cast Rage by FfH's terms, which value its promotion as if on Buboes himself (`casting::ffh_value`).

### Chaos Marauder (`UNIT_CHAOS_MARAUDER`)
*Melee, tier 2 · not trainable · 4/4, 1 move · Demon · from a Sheaim Planar Gate in a city with a Carnival*

**What it does.** A demon soldier the size of an Axeman, with the Demon race (resistant to death, fire, poison and unholy, vulnerable to holy). A Planar Gate city (Sheaim, Knowledge of the Ether) has a chance each turn to summon a creature matching one of its buildings: 6% (9% from Armageddon counter 50, 12% from 75, 15% at 100), one creature of each kind per Planar Gate (more as the counter rises; `ffh/events.rs`, `ev_planar_gate`); the Carnival calls Chaos Marauders. Lair exploration can call one as a henchman.

**Playing it well.** Free soldiers for the Sheaim: build Carnivals in Planar Gate cities for a steady trickle, and a higher Armageddon counter speeds them up.

**AI: Unused.** No AI builds a Planar Gate: its generic building value is 0 (`production::generic_building_value` reads none of its fields), and valuing it by its creatures was tried and dropped (0.1 a game, docs/AI_LOG.md; `civs/sheaim.rs`). Barbarian marauders are played by `units::barbarian`; one an AI held would be an ordinary army unit (`UNITAI_ATTACK`).

### Chariot (`UNIT_CHARIOT_SCORPION_CLAN`)
*Mounted, tier 2 · not trainable · 5/3, 3 moves, +1 poison, 25% withdrawal · Orcish · the Scorpion Clan's chariot (barbarian)*

**What it does.** The Scorpion Clan Chariot. A barbarian Scorpion Wolf Rider becomes one with Upgrade Chariot (barbarians only, needs Trade); a player at peace with the barbarians can hire one at a Goblin Fort for 120 gold.

**Playing it well.** A raider that hits hard and withdraws: keep strong defenders with your workers and settlers near goblin country.

**AI: Not an AI choice.** A barbarian unit played by `units::barbarian` (see Ars Moriendi); the barbarians cast Upgrade Chariot as soon as they can (`casting::ffh_value`: FfH's weight 300 plus the strength gained). Hire Chariot is `bAllowAI` 0.

### Disciple of Acheron (`UNIT_DISCIPLE_OF_ACHERON`)
*Arcane, tier 3 · not trainable · 3/3, 1 move, +1 fire · Channeling I-II, Fire I-II, Loyalty · Fireball · barbarian, from the Dragon's Hoard*

**What it does.** A barbarian fire adept: each turn a barbarian city holding the Dragon's Hoard (Acheron's city) has the lair spawn rate's chance to produce one (a Son of the Inferno once the barbarians know Sorcery; `ffh/events.rs`, `ev_city_do_turn`). It casts Fireball (Fire II); it also knows Blaze, which no AI may cast (`bAllowAI` 0). Loyalty makes it immune to capture.

**Playing it well.** Take Acheron's city early to stop the stream; until then keep stacks out of Fireball range of the city or cover them with fire-resistant units.

**AI: Not an AI choice.** A barbarian caster (`units::role_of`: `UNITAI_MAGE`). Without `magic.rs` (off for the barbarians), `units::caster` takes it beside a barbarian unit that stands next to the civilization city nearest the barbarian cities (`military::war_target`), else to the most threatened barbarian city; it summons its Fireball at the end of the turn when an enemy is within the Fireball's reach (`casting::summon_value`), and the Fireball attacks only next to it, at 5% odds or better (`casting::summons_attack`).

### Frostling (`UNIT_FROSTLING`)
*Recon, tier 1 · not trainable · 2/2, 1 move · Frostling race · barbarian, from Samhain and the Frostling event*

**What it does.** A small creature of the north. The Frostling race fights 25% better on snow (10% on tundra) and 25% worse in desert, adds 1 cold damage, is immune to cold and takes 50% more fire damage. Frostlings come for the barbarians from the Illian ritual Samhain (two per wave, with an Archer and a Wolf Rider, as many waves as civilizations alive plus the handicap minus 5, and Mokka; `ffh/events.rs`) and from the Frostling event (20% of games, on an empty snow or tundra plot of the player's).

**Playing it well.** Weak alone; Samhain's horde is the danger: it appears on empty land of large areas, wherever the draw puts it (`custom.rs::cf_add_unit`), so the Illians perform the ritual (which also gives their cities 20 turns of extra happiness) when their own cities can hold and their rivals' cannot.

**AI: Not an AI choice.** A barbarian unit. `cf_add_unit` creates the Samhain units as `UNITAI_ATTACK`, played by `units::barbarian` (see Ars Moriendi); the event's frostling keeps its default `UNITAI_EXPLORE` and explores (`units::explore`) until nothing is left to reveal, then joins that routine. The AI never performs Samhain: `production::best_project` skips it and `civs/illians.rs` builds only The White Hand and Stir from Slumber.

### Frostling Archer (`UNIT_FROSTLING_ARCHER`)
*Archery, tier 2 · not trainable · 2/4, 1 move, 1 first strike, +25% city defense · Frostling race · barbarian, from Samhain*

**What it does.** A Frostling archer, one per Samhain wave; lair exploration on snow can call one.

**Playing it well.** As the Frostling.

**AI: Not an AI choice.** As the Frostling: Samhain's units are `UNITAI_ATTACK` and played by `units::barbarian`.

### Frostling Wolf Rider (`UNIT_FROSTLING_WOLF_RIDER`)
*Mounted, tier 2 · not trainable · 3/3, 2 moves, 10% withdrawal, +40% against archers · Frostling race · barbarian, from Samhain*

**What it does.** A Frostling raider, one per Samhain wave; lair exploration on snow can call one.

**Playing it well.** As the Frostling; it threatens workers and lone archers.

**AI: Not an AI choice.** As the Frostling.

### Goblin (`UNIT_GOBLIN_SCORPION_CLAN`)
*Recon, tier 1 · not trainable · 2/2, 1 move, +1 poison, +50% against animals · Orcish · spawned by Goblin Forts*

**What it does.** The Scorpion Clan's scout, spawned by Goblin Forts (a lair, at the handicap's spawn rate while the area holds few barbarians, `plot/terrain.rs`). A goblin that kills a Wolf or Wolf Pack becomes a Wolf Rider (the barbarians' ordinary one, `spells_a.rs::post_combat_wolf_rider`). Barbarian goblins can become Scorpion Clan Archers (Archery) or Wolf Riders (Horseback Riding) with their Upgrade spells. A player at peace with the barbarians can hire one at a fort for 30 gold.

**Playing it well.** Clear Goblin Forts near your land early, before the barbarians learn Archery and Horseback Riding and the goblins grow into archers and wolf riders.

**AI: Not an AI choice.** Lair spawns are `UNITAI_ATTACK`, so `units::barbarian` plays them (see Ars Moriendi). The barbarians cast the Upgrade spells as soon as they can (weight 300 plus the strength gained, `casting::ffh_value`), the Wolf Rider first when both are open (it gains more). Hire Goblin is `bAllowAI` 0. Field armies clear Goblin Forts near our cities (`military::find_targets`); an AI unit entering one destroys it (docs/AI_ONLY_RULES.md, A4).

### Greater Werewolf (`UNIT_GREATER_WEREWOLF`)
*Beast, tier 3 · not trainable · 11/11, 3 moves · Werewolf · on a win: 45% to create a Ravenous Werewolf · from Blooded Werewolves*

**What it does.** The last stage of the werewolf curse: a Blooded Werewolf that wins has a 5% chance to become one. It spreads Ravenous Werewolves faster than the Blooded one.

**Playing it well.** As the Blooded Werewolf, with more urgency: 11 strength and 3 moves at mid-game.

**AI: Not an AI choice.** Played by `units::barbarian` when barbarian (see Ars Moriendi); a civilization's werewolves descend from Baron Duin Halfmorn (a hero any civilization can train with Feral Bond, whose wins create a Ravenous Werewolf 75% of the time) and fight as army units (`UNITAI_ATTACK`, `military::army`).

### Guardian Vines (`UNIT_GUARDIAN_VINES`)
*Immobile, tier 3 · not trainable · 9/9, 0 moves*

**What it does.** A rooted 9/9 guardian that cannot move. No rule, event or spell of the base game creates it (a scenario unit).

**Playing it well.** Not met in a normal game.

**AI: Not an AI choice.** It never appears. Were one given to an AI, it would be an army unit that cannot move (`units::role_of`: strength and `UNITAI_ATTACK`).

### Gurid (`UNIT_GURID`)
*Beast, tier 4 · not trainable, world unit · 22/22, 1 move · Sentry · +1 strength per Banana resource of its owner · great beast (barbarian)*

**What it does.** One of the great beasts: in a fifth of games, once a player knows Animal Handling, the Gurid appears for the barbarians on an empty land plot of a large area (`random_events_a.rs::do_great_beast_gurid`). It has an affinity for Bananas: +1 strength for each Banana resource its owner has. Like any beast it can be captured with Subdue Beasts (Combat III, Animal Mastery).

**Playing it well.** Avoid it early; later, a Subdue Beasts recon unit that wins against it (after others have worn it down) captures a 22-strength world unit.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi). AI recon units take Subdue Beasts for its weight of 300 (`units::promotion_value`), but nothing sends them after the Gurid.

### Herald (`UNIT_HERALD`)
*Recon, tier 4 · upgrade only (min level 4, Animal Mastery), national limit 4 · 14/14, 2 moves, -20% city attack, +50% against animals · moves through impassable terrain, carries one bird · Herald's Call · Mercurians*

**What it does.** The strongest Angel upgrade on paper. Herald's Call (`spells_a.rs::spell_heralds_call`) gives every living unit of its owner on its plot Valor, Blessed, Courage and Loyalty, and limits them to one more turn of life; for the Mercurians, the living units that die this way can come back as Angels.

**Playing it well.** Before the decisive assault, call it over a stack of expendable living units (or units the Mercurians will get back as Angels): one turn of greatly strengthened attackers. Otherwise it is a big, fast fighter that crosses mountains.

**AI: Incidental.** It comes only as an Angel's upgrade, which the AI picks when the Repentant Angel is not available because the Herald has the highest attack strength of the rest (`units::upkeep`). It keeps the Angel's `UNITAI_ATTACK` and fights as an army unit; Herald's Call is `bAllowAI` 0.

### Ira (`UNIT_IRA`)
*Melee, tier 4 · not trainable, national limit 4 · 8/8, 2 moves, +4 unholy · Cannibalize, Demon, Water Walking · +1 strength per kill · made by the Avatar of Wrath*

**What it does.** The Avatar of Wrath's spawn (Ira Unleashed keeps its owner at four). Every win against a living unit raises its attack and defense by one, until its strength reaches 32 (`spells_a.rs::post_combat_ira`); Cannibalize heals it 10% on each win, and it walks on water.

**Playing it well.** Kill Ira while they are young: a few kills turn them into a second wave of Avatars. Non-living defenders (undead, golems, angels) do not feed them.

**AI: Not an AI choice.** Barbarian units played by `units::barbarian` (see Ars Moriendi). The kills they feed on are whatever that routine attacks.

### Leviathan (`UNIT_LEVIATHAN`)
*Sea beast, tier 4 · not trainable, world unit · 22/22, 4 moves · great beast (barbarian)*

**What it does.** The great beast of the sea: in a fifth of games, once a player knows Optics, it appears for the barbarians on an empty water plot of a large sea (`random_events_a.rs::do_great_beast_leviathan`). It outfights any ship of its era.

**Playing it well.** Keep transports away from it; it is only dangerous where it is, so route around it.

**AI: Not an AI choice.** Like every barbarian sea unit it is Naval for `units::role_of` and holds (`units::hold`): it never moves or attacks. The AI has no naval combat to hunt it.

### Lich (`UNIT_LICH`)
*Arcane, tier 4 · not trainable, national limit 4 · 5/5, 1 move, +2 death, work rate 400 (builds mana nodes) · Undead · made by Lichdom*

**What it does.** A caster that has become undead. Lichdom (a Death III spell; not for Flesh Golems, puppets or world units, at most four Liches) turns its caster into a Lich, keeping its promotions and experience and adding Undead (immune to death and poison, resistant to unholy, vulnerable to holy). Mokka's Cauldron turns a tier-4 adept who dies in its owner's city into a demon Lich; lair exploration can call a barbarian one. A Lich can build mana nodes.

**Playing it well.** Make your Death III archmage a Lich once it is valuable enough to protect: the extra strength and death damage make it a combat caster, and undead immunity shrugs off Death and poison spells. Use its work rate to turn raw mana into nodes in the field.

**AI: Valued.** The AI casts Lichdom when `magic::turn_value` finds it worth casting (the Undead promotion on the caster, and the Lich's strength over the caster's type through `casting::ffh_misc_value`). The Lich keeps the caster's `UNITAI_MAGE` and goes on as a caster (`magic::caster_target`). It never builds mana nodes (no AI does; the Tower goal that did is never the AI's, `victory::plan_ours`). Death III casters are rare, because the AI trains few casters (`production::decide` caster step).

### Lightning Elemental (`UNIT_LIGHTNING_ELEMENTAL`)
*Elemental, tier 2 · not trainable · 0 strength, +3 lightning, 2 moves · Blitz, Elemental, Immune to Lightning · born from Air Elementals' wins*

**What it does.** Every time an Air Elemental beats a living unit, a Lightning Elemental appears beside it (100%), with the Air Elemental's remaining duration. It fights with its 3 lightning damage only, attacks several times a turn (Blitz), gets no defensive bonuses, and heals half its damage when it wins (`spells_a.rs::post_combat_heal50`).

**Playing it well.** Free chained attacks after each Air Elemental win: throw them at damaged defenders or weak units the same turn, before they expire.

**AI: Not an AI choice.** The rules create it; the AI then never moves it. `units::role_of` counts only base strength, which is 0, so it is Other and holds (`units::hold`); it is not a suicide summon either, so `casting::summons_attack` leaves it alone. AI Lightning Elementals expire unused.

### Manes (`UNIT_MANES`)
*Tier 1 · not trainable · 2/2, 1 move, +1 unholy · Demon · Add to City (+1 population) · upgrades to Warrior, Scout, Adept · Infernal only, given by the rules*

**What it does.** The Infernals' population and cannon fodder. The rules give Manes to the Infernals: three when Hyborem's Infernals appear; two, with no experience, when a unit of the Ashen Veil, Council of Esus or Octopus Overlords, or a Death I or Entropy I caster dies; and when an evil founder's city is razed (one try per citizen) or a neutral one (two per four citizens plus two). Each gift succeeds at the Angels' chance (100% - 3% per civilization alive), and for an AI Infernal a gifted Manes immediately joins its city as a citizen with a chance of 100% minus 5% per citizen (`custom.rs::cf_gift_unit`; docs/AI_ONLY_RULES.md, A3 k). Add to City sacrifices a Manes in its own city for +1 population, the Infernals' only growth, since their cities produce no food.

**Playing it well.** Add Manes to the cities that can use the citizens (high-yield tiles, specialists), keep a few as a cheap upgrade path to Adepts once there is gold, and wage wars on religious civilizations: their dead feed the Infernals.

**AI: Directed.** `civs/infernal.rs` walks a lone Manes to the nearest of our cities and casts Add to City with every Manes there that the city's garrison does not need (`units::garrison_quota`, which rises with danger); FfH's own value of the spell was negative, so Manes used to stay 2-strength army units all game (docs/AI.md, "Infernals"). Manes left over are army units and are upgraded by `units::upkeep` when gold allows.

### Manticore (`UNIT_MANTICORE`)
*Beast, tier 4 · not trainable · 12/8, 3 moves, +1 poison, +25% against mounted · Cannibalize, Flying · from a Sheaim Planar Gate in a city with a Grove*

**What it does.** A fast, flying beast attacker. The Planar Gate (see Chaos Marauder) calls it in a city with a Grove; lair exploration in Ruins can call a barbarian one. Subdue Beasts captures it.

**Playing it well.** One of the best Planar Gate creatures: a 12-strength flier for raids and finishing damaged stacks; weak on defense (8).

**AI: Unused.** No AI builds a Planar Gate (value 0; see Chaos Marauder). A barbarian Manticore is played by `units::barbarian`.

### Margalard (`UNIT_MARGALARD`)
*Beast, tier 4 · not trainable, world unit · 22/22, 1 move · Sentry · great beast (barbarian)*

**What it does.** A great beast like the Gurid (a fifth of games, once a player knows Animal Handling; `random_events_a.rs::do_great_beast_margalard`), without the Gurid's banana affinity. Subdue Beasts captures it.

**Playing it well.** As the Gurid.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi).

### Mary Morbus (`UNIT_MARY`)
*Melee, tier 3 · not trainable, world unit · 6/6, 2 moves, +2 poison, 50% withdrawal · Plague Carrier · Contagion · from the Mary event*

**What it does.** The bearer of the plague: the units she fights may catch Plagued (-60% strength, slower healing), and Contagion deals 10% death damage (up to 40%) and Plagued to living enemies within one plot. She comes from the Mary event in a city with an Alchemy Lab: continue the experiments (option 2) for Mary and -2 health in the city for good.

**Playing it well.** Send her in first against a stack: Contagion and her attacks leave the defenders plagued at -60%, and her 50% withdrawal keeps her alive. Plagued units need a priest or healing to recover.

**AI: Not an AI choice.** The event choice (`events::value`) counts Mary as a 600-gold unit (production cost -1) against at most a few gold a turn for the health, so an AI takes her. She is an army unit (`UNITAI_ATTACK`, `military::army`); `magic.rs` models Contagion (damage and a promotion), so she casts it before attacks and at the end of the turn when its hit points on enemies are worth it (`magic::before_attack`, `magic::choose_turn_spell`).

### Mercenary (`UNIT_MERCENARY`)
*Melee, tier 3 · not trainable · 5/5, 1 move, 10% withdrawal · death causes no war weariness · Recruit Mercenary (Guild of the Nine) and events*

**What it does.** A hired soldier. The owner of the Guild of the Nine can Recruit Mercenary in any of its cities for 180 gold (`spells_b.rs::spell_recruit_mercenary`; the Hippus get the Mounted Mercenary); events also sell them: the Mercenary event (Currency, 120 gold), the Guild of the Nine's offers (a plain mercenary for 200 gold, or companies with elven, amphibious, mutated, dwarven or orcish mercenaries and companions), the City Feud and Magnadine's hire. Its deaths cause no war weariness.

**Playing it well.** Gold into soldiers on the spot, where a city is threatened or a war needs bodies; mercenaries are the units to spend in costly attacks, since losing them does not tire your people.

**AI: Incidental.** The AI never recruits: FfH's value of Recruit Mercenary is its 180-gold cost and nothing else (`casting::ffh_value`, -720), and the rules' AI gate also wants an enemy within one plot (docs/AI_ONLY_RULES.md, A2). It buys mercenaries from events whose units are XML (`events::value`: a 600-gold unit for 120 or 200 gold); the Guild's companies made in Python count only their price, so it declines them. Bought mercenaries are army units (`military::army`).

### Minotaur (`UNIT_MINOTAUR`)
*Melee, tier 4 · not trainable · 8/11, 1 move, +25% city defense · from a Sheaim Planar Gate in a city with an Obsidian Gate*

**What it does.** A strong defender called by the Planar Gate (see Chaos Marauder) in a city with an Obsidian Gate.

**Playing it well.** A free 11-defense city garrison for the Sheaim.

**AI: Unused.** No AI builds a Planar Gate (see Chaos Marauder).

### Mobius Witch (`UNIT_MOBIUS_WITCH`)
*Arcane, tier 3 · not trainable · 4/4, 1 move, work rate 300 (builds mana nodes) · Channeling I-II, level 4 with random sphere I promotions · from a Sheaim Planar Gate in a city with a Mage Guild · upgrades to Archmage*

**What it does.** The Planar Gate's caster (see Chaos Marauder), called in a city with a Mage Guild: it arrives at level 4 with 14 experience, and each of the sixteen sphere I promotions with a 1-in-10 chance (`ffh/events.rs`, `ev_planar_gate`). It can build mana nodes and upgrades to the Archmage.

**Playing it well.** A free mage with random spells for the Sheaim; keep the ones that rolled useful spheres (Fire, Death, Chaos) and upgrade them.

**AI: Unused.** No AI builds a Planar Gate (value 0), nor a Mage Guild for its own sake (docs/AI.md, "Casters"). A witch an AI held would be a caster (`UNITAI_WARWIZARD`, `magic::caster_target`).

### Mokka (`UNIT_MOKKA`)
*Arcane, tier 3 · not trainable, world unit · 6/6, 1 move, +1 cold · Channeling I-II, Frostling · barbarian, from Samhain*

**What it does.** The leader of Samhain's frostling horde, created once for the barbarians by the Illian ritual (`ffh/events.rs`). It has Channeling but no sphere, so it casts nothing (its only listed spell, Summon Sand Lion, is Malakim-only).

**Playing it well.** Kill it with the horde; it is a 6-strength fighter, nothing more.

**AI: Not an AI choice.** A barbarian army unit (`UNITAI_ATTACK`, `units::barbarian`). No AI performs Samhain (see Frostling).

### Mounted Mercenary (`UNIT_MERCENARY_MOUNTED`)
*Mounted, tier 3 · not trainable · 5/5, 3 moves, 20% withdrawal · death causes no war weariness · the Hippus's Mercenary*

**What it does.** The Hippus version of the Mercenary: same strength, three moves and 20% withdrawal. Hippus get it from Recruit Mercenary and the events that give the Mercenary class (the Mercenary event, the Guild of the Nine's plain offer); the Python companies give the foot Mercenary.

**Playing it well.** As the Mercenary, as a fast reserve for the Hippus.

**AI: Incidental.** As the Mercenary: bought from events, never recruited.

### Muirin (`UNIT_MUIRIN`)
*Melee, tier 2 · not trainable, world unit · 3/3, 1 move, +1 unholy · Undead, Water Walking, Vulnerable to Fire*

**What it does.** An undead water-walker that cannot enter ocean. No rule, event or spell of the base game creates it (a scenario unit).

**Playing it well.** Not met in a normal game.

**AI: Not an AI choice.** It never appears; an AI's would be an army unit (`UNITAI_ATTACK`).

### Ophanim (`UNIT_OPHANIM`)
*Mounted, tier 4 · upgrade only (min level 4, Warhorses, Horse or Nightmare), national limit 4 · 9/6, 2 moves, +3 holy, 25% withdrawal, +50% against archers · Angel, Flying · Mercurians*

**What it does.** A flying angelic chariot: it crosses water and impassable terrain, withdraws from a quarter of its losing fights, and is strong against archers.

**Playing it well.** The Mercurians' strike unit over sea and mountains: hit archer-defended cities and isolated units, then fall back; its 6 defense needs a stack around it.

**AI: Incidental.** An Angel's upgrade (`units::upkeep`), taken when the Repentant Angel and the Herald are not available (attack strength 12). It keeps the Angel's `UNITAI_ATTACK` and fights as an army unit; flying is used only through ordinary pathing, and no plan sends it over water.

### Priest of Winter (`UNIT_PRIEST_OF_WINTER`)
*Disciple, tier 3 · not trainable · 5/5, 1 move, +2 cold · Channeling I-II, Divine, Ice I, Medic I-II, Winterborn, immune to cold · Call Blizzard, Slow, Cure Disease · Illians, from The White Hand*

**What it does.** The Illians' winter priests: The White Hand (Philosophy, 400 hammers) creates three, Dumannios, Riuros and Anagantios (`ffh/events.rs`). They heal the units with them (Medic), cast Slow (Ice I), Cure Disease (Medic II) and Call Blizzard (moves an adjacent blizzard onto their plot, turning grassland, plains and desert there into tundra), and reach Ice II's ice elementals with a promotion. Their Divine promotion opens a religion's priest spells only for a unit of that religion, and the ritual creates them with none.

**Playing it well.** Escorts for the Illian army: medics that keep stacks healthy, Slow on the defenders before an assault, blizzards to make the land Illian. Get Philosophy early enough for them to matter.

**AI: Partial.** `civs/illians.rs` builds The White Hand in the most productive city as soon as it can (not under threat or when broke), but Philosophy comes late: in traced games the priests arrived at turns 325-372 and spent the game summoning Ice Elementals (docs/AI_LOG.md, "Ritual projects"). They keep their default `UNITAI_RESERVE`, so they move as army units (`military::army`), not as casters, and cast what `magic.rs` and FfH's terms pick.

### Ravenous Werewolf (`UNIT_RAVENOUS_WEREWOLF`)
*Beast, tier 2 · not trainable · 5/5, 1 move, -25% city attack · Enraged, Werewolf · becomes a Blooded Werewolf on its first win (100%), 15% to create another*

**What it does.** A newly cursed werewolf, created by the wins of Blooded and Greater Werewolves, Baron Duin Halfmorn and other Ravenous Werewolves. It starts Enraged (+1 move, +20% attack, -20% defense, it acts on its own until it fights) and becomes a Blooded Werewolf after its first win.

**Playing it well.** Kill it before its first victory; once blooded it is an 8-strength beast that breeds more.

**AI: Not an AI choice.** Barbarian ones are played by `units::barbarian`; a civilization's (from Duin) are army units (`UNITAI_ATTACK`).

### Revelers (`UNIT_REVELERS`)
*Recon, tier 3 · not trainable · 6/6, 2 moves, +1 poison, 35% withdrawal · sees invisible land units · from a Sheaim Planar Gate in a city with a Gambling House*

**What it does.** A Planar Gate creature (see Chaos Marauder) called in a city with a Gambling House: a fast scout that sees invisible units around it.

**Playing it well.** Use it to find invisible enemies (assassins, shadows, Hidden units) near your cities and armies.

**AI: Unused.** No AI builds a Planar Gate (see Chaos Marauder).

### Sailor's Dirge (`UNIT_SAILORS_DIRGE`)
*Naval, tier 3 · not trainable, world unit · 8/8, 3 moves, +1 death, 20% withdrawal, carries 3 · Undead · barbarian ghost ship*

**What it does.** In a fifth of games, once a player knows Sailing, the Sailor's Dirge appears for the barbarians on an empty water plot of a large sea, preferably unowned, with three Skeletons aboard (`random_events_a.rs::do_sailors_dirge`). Whoever sinks it gets a treasure chest placed for it somewhere on the map (`spells_a.rs::post_combat_lost_sailors_dirge`).

**Playing it well.** Sink it with two or three warships for the chest; until then keep transports clear.

**AI: Not an AI choice.** A barbarian ship with cargo room is a Transport for `units::role_of`, and barbarian transports hold (`units::hold`): it never sails or lands its skeletons. The AI has no naval combat to sink it, and could not open the chest.

### Seraph (`UNIT_SERAPH`)
*Melee, tier 4 · upgrade only (min level 4, Rage), national limit 4 · 8/2, 1 move, +3 holy, collateral damage (40, up to 20%) · Angel, Channeling II-III, Divine, Fire I · Mercurians*

**What it does.** A fire angel that attacks with collateral damage. Its Divine promotion opens a religion's priest spells only for a unit of that religion, and Angels (hence Seraphs) are created without one (`cf_gift_unit` makes them with `player_init_unit`; only trained units adopt a religion, `city/turn.rs`); Fire I without Channeling I opens nothing, so it casts nothing until a promotion such as Fire II opens Fireball.

**Playing it well.** An attacker against stacks: its collateral softens a whole stack before the main attack. With 2 defense it must never be left exposed.

**AI: Incidental.** An Angel's upgrade, chosen only when the Repentant Angel, Herald and Ophanim are not available (attack strength 11, tied with the Angel of Death; `units::upkeep`). It keeps the Angel's `UNITAI_ATTACK`, so it is an army unit, not a siege engine: `campaign::strike` uses its collateral only as any attacker's. A Mercurian AI casting with `magic.rs` may promote it toward Fire II for Fireball (`magic::promotion_value` counts for any unit).

### Shade (`UNIT_SHADE`)
*Tier 4 · not trainable, national limit 4 · 2/2, 1 move · joins a city as a Great Artist, Engineer, Merchant or Scientist · Sidar, from Wane*

**What it does.** A retired hero: a Sidar unit of level 6 or more can cast Wane (`SPELL_WANE`) to become a Shade (at most four), which then joins a city as a free great specialist of one of four kinds.

**Playing it well.** Retire high-level units that have outlived their war (a level-6 axeman in a late game) into super specialists in your best cities; guard those cities, since the specialists are lost with them.

**AI: Unused.** Wane is `bAllowAI` 0 (docs/AI_ONLY_RULES.md, D), so no AI makes a Shade. One an AI held would not join a city either: it would keep its caster's AI type (`unit_convert`), not a great-person one, so it would be an army unit with 2 strength (`units::role_of`).

### Son of the Inferno (`UNIT_SON_OF_THE_INFERNO`)
*Arcane, tier 4 · not trainable · 5/5, 1 move, +3 fire · Channeling I-III, Fire I-III, Loyalty · Fireball, Summon Fire Elemental · barbarian, from the Dragon's Hoard after Sorcery*

**What it does.** The Disciple of Acheron's stronger form: once the barbarians know Sorcery, the Dragon's Hoard city produces these instead (see Disciple of Acheron). It summons Fireballs and Fire Elementals.

**Playing it well.** As the Disciple, with more urgency: Fire Elementals and Fireballs from the city every turn make a siege of Acheron's city costly.

**AI: Not an AI choice.** A barbarian caster, played as the Disciple of Acheron (`units::caster`, FfH's summon terms).

### Spiderkin (`UNIT_SPIDERKIN`)
*Recon, tier 4 · not trainable, national limit 4 · 12/12, 2 moves, +2 poison, 1-3 first strikes, +50% against animals · a random result of Wonder, for one turn*

**What it does.** One of the 66 random results of Wonder (a Chaos III spell with three to five results a cast, `spells_b.rs::spell_wonder`): a Spiderkin for one turn (two for a Summoner). It sits in the Beastmaster's class, but no civilization trains it.

**Playing it well.** A surprise strong attacker for a turn when Wonder rolls it: attack with it at once.

**AI: Incidental.** The AI casts Wonder for FfH's weight of 300 (`casting::ffh_value`; it lives in Python, outside `magic::understood`), and a Spiderkin is one chance in 66 per result. Cast at the end of the turn, a one-turn Spiderkin expires at the start of our next turn, when units' turns are run (`player_set_turn_active`, `unit_do_turn`), before it gets an order; a Summoner's two-turn one gets one turn as an explorer (its default `UNITAI_EXPLORE`, `units::explore`: it attacks only what it beats at 75% odds next to it).

### Stephanos (`UNIT_STEPHANOS`)
*Mounted, tier 4 · not trainable, world unit · 19/19, 3 moves, +5 unholy · Command I-III, Crown of Command, Demon, Fear, Flying, Magic Immune · the first Horseman (barbarian, Armageddon counter 40)*

**What it does.** The Horseman of conquest: appears for the barbarians at counter 40 (`random_events_a.rs::do_armageddon_stephanos`), flies, causes Fear, ignores spells, razes the cities it takes, and makes four separate capture rolls on every win (Command I 10%, Command II and III 20% each, the Crown of Command 30%), so most units it defeats join the barbarians. It drops the Crown of Command when killed.

**Playing it well.** Do not feed it units one by one: every loss can become a barbarian unit. Kill it with a few strong attackers in one turn and take the Crown.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi); the captures happen in the rules.

### Succubus (`UNIT_SUCCUBUS`)
*Melee, tier 3 · not trainable · 4/4, 2 moves, +2 unholy · Demon, Mind II · Charm Person · from a Sheaim Planar Gate in a city with Public Baths, and the Sallos event*

**What it does.** A demon that charms: Charm Person (Mind II) makes an adjacent living enemy Charmed, able only to defend, unless it resists. The Planar Gate (see Chaos Marauder) calls it in a city with Public Baths; the Sallos event (a fifth of games, for a player with the Ashen Veil as state religion) gives one, Kalra.

**Playing it well.** Charm the strongest attacker of an enemy stack next to you before it can strike, or a defender you do not want counter-attacking.

**AI: Incidental.** No AI builds a Planar Gate (see Chaos Marauder), but the AI accepts Sallos's offer (a 600-gold unit, `events::value`). Its Succubus keeps its default `UNITAI_RESERVE` and is an army unit; `magic.rs` models Charm Person (a promotion added), so it is cast before attacks and at the end of the turn when the curse is worth its hit points (`magic::before_attack`, `magic::choose_turn_spell`).

### Supplies (`UNIT_SUPPLIES`)
*Tier 1 · not trainable · 0 strength, 1 move · builds an Aqueduct, Archery Range, Forge, Hunting Lodge, Library, Palisade, Stable, Training Yard or a temple instantly · from lairs and the Bareke event*

**What it does.** A wagon of materials: in one of the owner's cities it constructs one of thirteen early buildings at once (`MISSION_CONSTRUCT`). It comes from exploring a lair and from the Bareke event (option 3, for 50 gold).

**Playing it well.** Drop it in a new or weak city for the building it most needs (a Library in a commerce city, a Forge or Training Yard in a production city, the state religion's temple for happiness).

**AI: Unused.** The AI takes it from the Bareke event: it counts it as a 600-gold unit (production cost -1), a shade above that event's Great Engineer option (`events::value`). Then it never uses it: with no strength and no worker builds it is Other for `units::role_of` and holds, and the AI issues `MISSION_CONSTRUCT` only for shrines (`units::great_person_build`).

### Tar Demon (`UNIT_TAR_DEMON`)
*Disciple, tier 3 · not trainable · 2/7, 1 move, +1 unholy, flat movement costs · Demon, Ashen Veil · splits in two when killed · from a Sheaim Planar Gate in a city with a Temple of the Veil*

**What it does.** A defensive demon (7 defense) that does not die easily: killed, it splits into two Tar Demons at 25% damage, Weak ones (-1 strength) unless it was Strong; Weak ones do not split again (`spells_a.rs::post_combat_split`). The Planar Gate (see Chaos Marauder) calls it in a city with a Temple of the Veil.

**Playing it well.** A sticky defender and blocker: enemies must kill it three times. Put it on chokepoints and in threatened cities.

**AI: Unused.** No AI builds a Planar Gate (see Chaos Marauder).

### Treasure Chest (`EQUIPMENT_TREASURE`)
*Item · not trainable, invisible · Open Chest · from lairs, the Treasure event and the Sailor's Dirge*

**What it does.** A chest placed for a player on a remote empty land plot (preferably unowned or on a small island, `custom.rs::cf_place_treasure`): the result of exploring a lair, the Treasure event (a map for 40 gold) and sinking the Sailor's Dirge. A unit on its plot can Open Chest (`spells_b.rs::spell_open_chest`): a 25% chance of a trap (poison, fire damage or three turns of immobility), then one of seven results: nothing, a lot of gold, a Healing Salve, Jade Torc, Rod of Winds, Timor Mask, or a technology. It is invisible, and an enemy at war that steps onto it captures it.

**Playing it well.** Send a fast, healthy, expendable unit (a scout, a hero with Mobility) for it; one in seven chests is a tech, and the items are worth a detour.

**AI: Unused.** Open Chest is `bAllowAI` 0, so no AI opens one. The AI declines the Treasure event (its two options score 460 and 500, `events::value`), gets no chests from lairs (it never explores them) and never sinks the Sailor's Dirge.

### Wolf Rider (`UNIT_WOLF_RIDER_SCORPION_CLAN`)
*Mounted, tier 2 · not trainable · 4/4, 3 moves, +1 poison, 10% withdrawal, +40% against archers · Orcish · the Scorpion Clan's wolf rider (barbarian)*

**What it does.** The Scorpion Clan's raider: a barbarian Scorpion Goblin becomes one with Upgrade Wolf Rider (Horseback Riding), and it can become a Scorpion Chariot (Upgrade Chariot, Trade). A player at peace with the barbarians can hire one at a Goblin Fort for 60 gold.

**Playing it well.** Three moves and +40% against archers make it a threat to undefended workers and archer-held towns; spearmen and walls stop it.

**AI: Not an AI choice.** A barbarian unit played by `units::barbarian` (see Ars Moriendi); the barbarians cast its upgrade as soon as they can. Hire Wolf Rider is `bAllowAI` 0.

### Yersinia (`UNIT_YERSINIA`)
*Mounted, tier 4 · not trainable, world unit · 15/15, 3 moves, +7 poison, +5 unholy · Athame, Cannibalize, Demon, Fear, Flying, Magic Immune, Plague Carrier · the fourth Horseman (barbarian, Armageddon counter 60)*

**What it does.** The Horseman of pestilence: appears for the barbarians at counter 60 (`random_events_a.rs::do_armageddon_yersinia`), plagues what she fights (twice over, with Plague Carrier and the Athame), heals on wins (Cannibalize), flies, causes Fear, ignores spells and razes the cities she takes. She drops the Athame when killed.

**Playing it well.** Poison-immune defenders (undead, demons, elementals, golems) blunt most of her strength; meet her with them and keep healers (priests, Cure Disease) near the front for the plague.

**AI: Not an AI choice.** Played by `units::barbarian` (see Ars Moriendi).
