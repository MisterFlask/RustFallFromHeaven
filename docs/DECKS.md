# Decks: secret techs and the Armageddon deck

**Status: design; being built** (2026-11-07). The decks as built, and each card's status:
`docs/CARDS.md`. Ideas to choose from, not a spec. The
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
- New rules may be drafted; spawning civs and revolutions are in scope.
- Unrest is worth having, and what raises and lowers it is mostly specific to each
  civilization (section 5).
- The Balseraphs' first split-off civ is Balseraphs, led by Somehow Also Perpentach.
- Vendetta (section 6.2) is liked: keep it.
- **Self-contained payoffs** (section 7): a card's or crisis's effect on the player's decisions
  must not rest chiefly on how other civs respond, since the AI may or may not respond
  reasonably.

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
- **Plot type**: land, hills, peak, water. `plot_set_plot_type` changes it mid-game, areas
  recalculated; FFH's volcano event turns a plot into land. Terraforming land into sea or sea
  into land therefore needs only a rule that calls it.

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

### 1.9 New rules: civs born mid-game, and revolutions (draft)

The engine already adds players mid-game (Hyborem, Basium, the crusade Hosts), and the AI plays
any civilization. These two rules turn that into a lever cards and crises can call.

**A civ's birth.** Every birth needs:
- **A slot.** Reserve spare player slots, as crusades reserve Host slots. With none free, the
  birth's cities go to the barbarians instead.
- **A civilization.** Prefer one not in play this game that fits the birth (its alignment,
  religion, race): a 6-civ game leaves 15 of the 21 unused. Otherwise the parent's own
  civilization under a new name (the Free Bannor).
- **A leader.** The governor who led it (its traits become leader traits), a hero, or a leader
  of the chosen civilization.
- **A start.** The parent's techs; the cities' garrisons; gold by population; alignment from its
  state religion; and a memory of its birth with every civ ("seceded from", "liberated by",
  "released by").
- **Diplomacy at birth.** At war with the parent (civil war) or at peace (a released state), by
  the kind of birth.
- **Standing.** A full civ: its share of the world counts in the score, and it can win.

A civ can be born in six ways, each a rule a card or a crisis can call:
1. **Secession:** a group of a civ's cities becomes a new civ (revolution, below).
2. **Release:** an owner frees cities on purpose (a Free City, a Holy See, a vassal colony).
3. **Barbarian crowning:** a barbarian holding of 3+ cities becomes a civ.
4. **Return:** a destroyed civ comes back where its culture survives.
5. **Arrival:** a civ from outside the world (Hyborem and Basium; the Host of Agares; a nomad
   horde).
6. **Mirror:** a sibling of an existing civ.

**Revolutions.**
- **Unrest** (0-100, a city's, new). Most of it comes from the civilization: each civ has its
  own risers and calmers (section 5). The Doviello grow restless without plunder; the Calabim's
  cattle grow restless when fed on. A thin core is the same for every civ, so rivals always
  have a lever on it:
  - **Core risers:** turns in disorder; another civ's culture on the city's plot; Foment Revolt
    pressure (the cultural channel's pressure becomes Unrest); espionage (Incite Unrest,
    section 3.6); a disloyal governor.
  - **Core calmers:** garrison units; a loyal governor; Courthouse, Governor's Manor and
    Dungeon (the crime they reduced did nothing; Unrest gives them a real job); the Vigil
    process and Jesters (section 3.6).
  - The first draft's generic risers (unhappiness, distance from the capital, an opposed
    religion, war weariness) become options that a civilization can take in section 5.
  - A directly ruled city gains no Unrest. The four direct-rule slots get a second meaning: the
    cities you cannot afford to lose.
- **The revolt.** At 100 the city revolts. Governed cities of the same owner within 5 plots
  with Unrest 60 or more go with it.
- **Where it goes.** If one foreign civ's culture is the city's majority, the city joins that
  civ (as Foment Revolt does today). Otherwise the group secedes as a new civ, led by the
  governor of its largest city. This changes Foment Revolt: today a revolting city always joins
  the caster. Some civilizations' rebels become another civilization (the Ljosalfar's
  Svartalfar, the Balseraphs' second Perpentach: section 5.3).
- **The owner's answer.**
  - *Let Go:* peace.
  - *Fight:* civil war. Each rebel city gets a free unit per 3 population, and the parent's
    other cities lose 10 Unrest, rallying to the crown.
- **Governors gain loyalty.** New traits: Loyal (less Unrest), Ambitious (more Unrest, better
  output), Popular (less Unrest; leads a secession if one comes).

*Expert:*
- Keep the far, foreign-cultured, religiously mixed cities under direct rule, or garrisoned
  under loyal governors.
- Against a rival, Foment Revolt can push his soft periphery into secession instead of taking
  it. A new weak neighbour costs less than a city to defend.

**Other big rules within reach:**
- leader succession (leaders age and die; heirs come with traits);
- city-states (one-city civs with their own diplomacy);
- puppet states (a vassal its master moves directly; the Balseraphs' flavor);
- nomad civs (an army that moves and demands tribute);
- colonies that grow into vassal civs.

### 1.10 Levers to build

Mechanics the code does not have yet (checked against `ffh-core` and `ffh-ai`), each with what it
would open up. Some partial versions exist, and the entry says so.

**Generic primitives.** Many cards need these, so they come first.
- **Timed status effects on cities, plots and players.** Units have promotions with expiry. A
  city, plot or player has only a few fixed timers (happy turns, revolt turns, golden age).
  A general "Blessed for 10 turns" / "Cursed" / "Quarantined" status opens up every timed card
  and crisis.
- **History counters.** Per city: units killed defending it, sieges survived, owners, razings.
  Per player: forests cut, cities razed, treaties broken, counter points added. The world
  remembers what happened in it. Ghost Garrisons, the Green Revenge and culpability rest on
  these.
- **Scheduled effects.** "In 20 turns, X happens to Y", announced now: prophecies, debts falling
  due, a curse that lands later. Spells have a delay; nothing general exists.
- **Hooks for cards.** "When a city is founded / a unit dies / a great person is born / a war
  is declared, do X", so a card can attach a rule without new code at each site.

**Characters**
- **Mortal leaders and succession.** Leaders age and die. The heir arrives with drawn traits,
  and the transition is a moment of weakness (Unrest, a claimant). It is the strongest stream
  of plan-changing news in Old World and Crusader Kings; leaders here are immortal.
- **Leader agendas.** Each AI leader draws a hidden agenda per game, revealed over contact
  (Civ6). Rivals then differ from game to game, not only by leader.
- **Hero relationships.** Rivalries and nemeses that remember who killed whom (Shadow of
  Mordor), loyalty to a leader, a hero who leaves when slighted.
- **Governor ambitions.** A governor wants something (a building, a war, a post) and its
  loyalty moves with whether it gets it. Governors have profiles and traits, but no wants.
- **A leader on the field.** A leader unit whose death costs Unrest and succession, and whose
  presence lifts an army.

**Inside a civilization**
- **Factions or estates.** Nobles, clergy, guilds, the army, the cattle, each with approval,
  wants and power (Europa Universalis' estates, Victoria's interest groups). This is the
  natural source of per-civ Unrest (the Calabim's nobles against their cattle) and of who leads
  a secession.
- **Population by people.** A city's citizens split by race or culture (elves and men,
  dwarves and orcs, cattle and lords). Today only units have races. It decides Unrest, the
  rebels' identity and assimilation.
- **Migration.** People move between cities and civs, toward happiness, food and faith, and
  away from war. Exodus becomes a rule, not a one-off.
- **An orders budget.** A limited number of the leader's actions per turn (Old World's
  orders). It makes governors and direct rule matter more and limits micromanagement.
- **Legitimacy and ambitions.** A leader's standing, raised by completing ambitions drawn
  from an offer (Old World). It is a long-term goal the player chooses among random ones.

**War**
- **Supply and attrition.** Units beyond supply lose HP each turn, more in winter, desert and
  enemy land. Supply exists only for marching recruits. It makes the Caravanserai, Automatons,
  winter and logistics real.
- **Zones of control.** Units slow or stop enemy movement next to them; there is none now.
  Borders and chokepoints get a military shape.
- **War goals and war score.** A war declared for a stated goal (a city, a plot, tribute,
  revenge), a score from battles and occupation, and peace terms that follow from it. Today
  peace is an ordinary trade. Wars end for reasons, which helps expert play and the AI alike.
- **Casus belli and claims.** Claims on plots and cities (from history, culture, marriage,
  religion) that lower the cost of a war over them. They tie into history counters and
  secession.
- **Ceasefires and demilitarized zones.** Pauses that are not peace; borders with rules.
- **Prisoners and ransom.** Captured combat units and heroes held, exchanged, ransomed or
  executed. Today only non-combat units are captured.
- **Persistent morale.** An army's morale carried across battles (winning streaks, routs,
  bad news from home). Morale now is only the Fear check inside one battle.
- **Army traditions.** An army gains XP and traits as a unit (its standards, its battles), not
  only its soldiers. Armies already exist and have names.
- **Battles that last.** A battle that runs into the next turn, so neighbouring stacks can
  reinforce it. Today an engagement resolves the plot's units in one go.
- **Siege damage to the prize.** Buildings, population and improvements damaged during a
  siege, not only at capture.

**Diplomacy**
- **Reputation.** A public record of treaties broken, demands extorted and allies abandoned,
  read by every civ. Memories are per leader now, not a reputation.
- **Guarantees and coalitions.** "Attack them and you fight me" made explicit; a coalition that
  forms against a runaway. Crusades cover part of this.
- **New deal items.** Loans, sworn peace, hostages, marriages, embassies, shared vision,
  passage of arms (section 3.6).
- **Minor factions.** Non-civ actors on the map: city-states, monster nations, mercenary
  companies, orders of knights that can be courted, hired, absorbed or destroyed (Endless
  Legend's minor factions). Crusade Hosts already show that temporary players work.

**Economy**
- **Stockpiles.** Strategic resources as quantities that units and buildings consume (Iron 3
  per Champion). Today resources are on or off. Supply then gives trade, raids and cornering a
  market something to bite on.
- **Depletion.** A mine runs out; a herd dies. Discovery exists (an improvement can reveal a
  resource); depletion does not. The map's value shifts over a game.
- **Prices.** Resources and units with prices that move with supply and demand (the Black
  Market, the Last Market).
- **Trade as caravans.** Trade routes carried by units or ships that can be raided, escorted
  or blockaded.
- **Debt and interest.** Loans, interest, default.
- **Harvests.** Food that varies by year and season; famine as a pressure.

**Information**
- **Spies on the map.** There are no spy units, and espionage has one mission. Spies would
  carry the missions of section 3.6.
- **False information.** What a rival sees can be wrong: a stack shown smaller (Masquerade),
  false colours (the Masks of Esus), forged letters.
- **Intelligence.** How much a civ knows about another: its army, gold, plans, cards, Unrest.
  Embassies, Informers and the Ledger buy it.
- **Forecasts.** What is coming, shown in advance: the next Armageddon card, the heir's traits,
  a famine. The roguelike principle's "warning" leg.

**Magic**
- **A mana pool.** Spells cost mana from a civ's or a city's pool, filled by nodes. Today a
  spell costs at most gold and a turn. Cards could then grant, steal or overflow mana.
- **Learning spells.** Casters learn spells from scrolls, tutors or study, beyond the sphere
  promotions.
- **Wild magic.** Surges and miscasts that grow with how much magic a civ uses; corruption
  from casting.
- **Summons with upkeep.** Lasting summons that drain mana or gold, so an army of them is a
  choice.

**The world**
- **Seasons that matter.** Entirely new. The data has only four season names, used by Civ4's
  Seasons calendar, a date-label option that no game uses: the standard calendar counts years,
  many to a turn early on. A seasonal rule needs its own cycle (say four seasons of 5 turns)
  for winter attrition, the elven courts' reigns and harvests.
- **Weather.** Area effects for a few turns: storm, fog, blizzard, drought (the Weather Engine,
  the Ashen Sky).
- **Climate drift.** Temperature that moves terrain over time. The hell terrain's plot counter
  is one existing case.
- **Plague as an entity.** A disease that spreads along trade routes and armies and is fought
  by quarantine and medicine; health is static now.
- **Rivers that move.** Floods, new courses, dried beds.

**Map content**
- **Dungeons.** A lair with depths: multi-step exploration with choices, guardians and rewards
  that grow with the depth (Stellaris' anomaly chains). Lairs today are one roll.
- **Ruins with stories.** A razed city's ruins keep its name, its owner and what it had: claims,
  ghosts, treasure.

**Meta**
- **An objectives deck.** Public and secret objectives drawn per game, scored toward victory
  or rewards (Twilight Imperium's objectives, Old World's ambitions). Each game's goals differ,
  the roguelike principle applied to winning itself.
- **Deck operations.** Draw, peek, keep one of two, put back, copy, burn (section 3).

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

## 3. The great-person deck (66 cards)

Section 7 reworks or cuts the cards whose payoff rested on how rivals respond.

Format: **Name** · band · needs (the ordinary tech the content requires) · vehicle. Tags:
**[data]** = existing keys can express it; **[new: ...]** = a rule to write. "National, N" =
N at the normal cost, then 500% each; "hard" = a true limit.

### 3.1 Sage (13)

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

**Mirror Kingdom** · III · needs Pass Through the Ether · world spell · [new: civ birth, mirror]
Once per game, a mirror of your civilization opens on the largest unclaimed region. It is a new
civ of your civilization, led by a mirrored leader with your traits, with 2 cities, copies of 4
of your units and all your techs. It is your permanent ally (a defensive pact it never cancels)
and plays for itself. Its share of the world is its own, not yours.
*Expert:* an ally on a front where you could never expand. Cast it when that land would
otherwise go to a rival, not when it could go to you.

### 3.2 Prophet (14)

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
*Drown*: a flat land plot next to water within 2 plots, with no city, becomes Coast (water) for
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

**Holy See** · III · needs Theology · building (a holy city you hold) · [new: release]
Declare the city a Holy See. It becomes a new civ, a theocracy of its religion, and your
permanent ally. The See trains its religion's heroes and units at half cost and calls crusades
of its own against the religion's enemies. Civs of its religion take -3 attitude toward anyone
at war with it.
*Expert:* trade a city for a crusading ally that your faith's civs will not fight; best when
the holy city sits on a border you cannot hold.

### 3.3 Engineer (13)

**Cairn Golem** · I · needs Masonry · national unit, 3 · [new: unit to improvement]
Large, armored, 1 move. *Raise the Cairn* (on hills): it becomes a permanent Cairn, +50%
defence for units on it and sight 3. A battle on the Cairn counts it as a Sentinel on open
ground.
*Expert:* plant cairns at the chokepoints before borders harden; three cairns decide where the
next war is fought.

**Causeway** · I · needs Fishing · worker build · [new: sea to land]
Over 15 turns, a Coast plot that touches land on two sides becomes grassland.
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

**Free City Charter** · II · needs Construction · building · [new: release]
Release the city as a Free City: a one-city civ at peace with everyone. Attacking it costs the
attacker -2 attitude with every civ. It trades with all and pays you half its commerce for 50
turns.
*Expert:* release the border city you cannot defend. It becomes a neutral buffer that pays,
and a rival who takes it pays in reputation.

### 3.4 Merchant (13)

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

**Charter Company** · III · needs Mercantilism · national unit, 1 · [new: colony civ birth]
The Company founds a city on another landmass. The city is a new civ, your vassal,
which settles and grows on its own and pays you 25% of its gold. It may declare independence
once your power falls below its own.
*Expert:* land you cannot govern well becomes a vassal that expands itself; keep your power
above its, or it walks.

### 3.5 Bard (13)

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

**The Pretender's Song** · II · needs Drama · building · [new: Unrest]
Name a rival's governed city. For 30 turns its Unrest rises 3 a turn. If it secedes, the new
civ starts Friendly to you, with open borders and a defensive pact on offer.
*Expert:* break a leader's periphery into a friendly buffer instead of conquering it. Aim at
the far city with the foreign culture.

### 3.6 Capabilities and civics (28)

These cards work the way existing techs do: Construction gives bridges and irrigation,
Cartography map trading and open borders, Honor defensive pacts, Feudalism vassals, Drama the
commerce slider, and techs unlock civics. Each card here unlocks a capability or a civic. A
civic behind a card is held by one civ in the world, since the card leaves the deck: this game,
only one empire has a Satrapy.

Today there are five civic options with 31 civics, three processes (Wealth, Research,
Culture), two hurries (population, gold) and one espionage mission (See Demographics). Each of
those is a slot these cards can fill.

**Civics**

**Satrapy** · Merchant · II · needs Code of Laws · Government
Only the capital is directly ruled. Governors get one more trait. Governed cities gain half the
Unrest. No maintenance from distance.
*Expert:* a wide empire that delegates; world wonders, which need direct rule, are then built in
the capital alone.

**Regency Council** · Bard · II · needs Code of Laws · Government
Six direct-rule slots instead of four. Every governor has one trait fewer, and governed cities
gain +1 Unrest a turn.
*Expert:* the tall empire's wonder government; a wide empire pays for it in Unrest.

**Confederation** · Merchant · III · needs Mercantilism · Government
A city at Unrest 100 does not revolt; it becomes Autonomous. It stays yours for the score and
trade routes, but gives no production, units or taxes until its Unrest falls below 50.
*Expert:* holds a big restless empire together through a crisis (the Free Cities, the
Sundering), at the price of the output.

**Ancestor Worship** · Prophet · I · needs Ancient Chants · Cultural Values
Cities you founded gain 1 Unrest less a turn for every 25 turns of their age. Graveyards give
+2 culture. Captured cities gain +1 Unrest a turn.
*Expert:* a builder that never loses its own cities pays for it in holding conquests.

**Carnival** · Bard · II · needs Festivals · Cultural Values
The Culture process also lowers the city's Unrest. Golden ages last 50% longer. -10% research.
*Expert:* the culture civ's answer to Unrest crises; the research cost is the price.

**Stoicism** · Sage · II · needs Philosophy · Cultural Values
War weariness causes no unhappiness, and unhappiness raises no Unrest. -15% culture.
*Expert:* the long-war civic.

**Corvée** · Engineer · I · needs Masonry · Labor
A city may draft a citizen as a Worker (population -1, regrown normally). Improvements build
25% faster. Each draft gives the city +10 Unrest.
*Expert:* the early land-improvement rush, paid in Unrest.

**Golem Labor** · Engineer · III · needs Mithril Working · Labor
A golem in a city works a plot as a citizen and eats nothing. Golems cost +50% hammers.
*Expert:* growth past the food cap; for the Luchuirp it is the whole economy.

**Tribute State** · Merchant · II · needs Taxation · Economy
Demands paid to you are worth 50% more. Vassals pay you 10% of their commerce. Civs you have
made demands of take -1 more attitude.
*Expert:* the strong civ's economy of fear.

**Plunder** · Bard · I · needs Warfare · Economy
Pillage gives 3 times the gold. A captured city gives you 10% of its owner's treasury. Captured
cities keep 25% fewer buildings.
*Expert:* the raider's economy (the Doviello's, the Clan's); a poor choice for a civ that means
to keep what it takes.

**The Grey Council** · Sage · II · needs Philosophy · Membership
A third council, with a vote source of its own. Its resolutions:
- Truce of Dagda: no war declarations for 10 turns.
- Hold the Counter: the Armageddon counter cannot rise for 20 turns.
- Open the Archives: each member gets the tech known by the most members.
- Sanction: a named civ's deals and trade routes with members end.

*Expert:* the neutral civs' instrument against the other two councils' wars and the world
clock.

**Processes**

**Vigil** · Prophet · I · needs Mysticism
Production lowers the city's Unrest: 1 point per 10 hammers.
*Expert:* a restless city works for its own peace before it revolts.

**Penance** [good or neutral] · Prophet · II · needs Religious Law
Production lowers the Armageddon counter: 1 point per 500 hammers, counted as penitence.
*Expert:* the good civs' way to race culpability (the Census of the Damned, the Shepherd's
Mercy).

**Muster** · Bard · I · needs Military Strategy
Production gives XP to units in the city: 1 XP per 30 hammers, spread among them.
*Expert:* an idle military city before a war.

**Hurries**

**Patronage** · Bard · II · needs Drama
Spend the city's culture to finish production, at 2 culture a hammer. Spent culture counts
toward neither borders nor the culture victory.
*Expert:* a culture city that has its borders turns culture into wonders.

**Diplomacy** (new deal items)

**Embassies** · Merchant · I · needs Writing
Exchange embassies. Each side sees the other's cities' Unrest and production, and its held
secret techs.
*Expert:* needed to aim Foment Revolt or the Pretender's Song well.

**Hostages** · Merchant · I · needs Code of Laws
A unit given as a hostage with a peace. If the giver breaks the peace, the hostage joins the
holder. If the holder breaks it, the hostage goes home with two levels more.
*Expert:* give a hero to buy a peace your neighbour will believe.

**Royal Marriage** · Bard · II · needs Code of Laws
Permanent +2 attitude between the two leaders. When either civ's cities revolt, the rebels may
join the spouse instead of seceding.
*Expert:* marry the neighbour whose empire is about to break.

**Shared Vision** · Sage · I · needs Cartography
A deal item: each side sees everything the other's units and cities see.
*Expert:* an alliance's eyes; a cheap gift to a friend at war.

**Passage of Arms** · Engineer · II · needs Honor
The other side's armies may march through your land and stay in supply there, without open
borders' trade and culture.
*Expert:* how a crusade reaches a target behind a neutral.

**Espionage**

**Spymasters** · Merchant · II · needs Deception
Unlocks missions, paid in espionage points (the fourth commerce):
- Incite Unrest: +25 Unrest in a city.
- Poison the Wells: -2 population.
- Steal Plans: see a civ's held secret techs and its next great person's card.
- Forge Letters: two civs take -2 attitude to each other for 20 turns.
- Turn a Governor: one of the target's governors becomes Disloyal.

*Expert:* Unrest becomes a weapon at range.

**Movement**

**Mountaineering** · Engineer · II · needs Engineering
Land units can enter peaks, at 3 moves. A unit on a peak gets +50% defence and sight 3.
*Expert:* the impassable border is a road for you alone.

**Ferries** · Engineer · I · needs Fishing
Land units cross a one-plot strait in one move, as if it were land.
*Expert:* the early island neighbour is in reach without boats.

**Forest Paths** · Prophet · I · needs Way of the Forests
Your units move through forest and jungle as through open ground. Enemy units in your forests
have -1 move.
*Expert:* a forest border becomes your highway and the enemy's mud.

**Specialists**

**Jesters** · Bard · I · needs Festivals
A new specialist: +1 culture, and -3 Unrest a turn in its city.
*Expert:* a citizen spent on order, in the city about to revolt.

**Informers** · Merchant · II · needs Code of Laws
A new specialist: +3 espionage points, and shows the Unrest of foreign cities within 6 plots.
*Expert:* the border city that watches the neighbour's soft cities.

**Improvements**

**Watchtower** · Engineer · I · needs Construction
+3 sight. A unit on it strikes first when attacked (a defensive strike).
*Expert:* the early warning line on an open border.

**Wayside Shrines** · Prophet · I · needs Priesthood
Built on a road: +1 culture to the city that works it. Units passing heal 10%.
*Expert:* roads to the front that heal the army on its way.

### 3.7 Deck notes

- **Count.** 94 cards: the 66 above (13 per suit, 14 for the Prophet) and the 28
  capabilities and civics. The bands of the 66 are spread across I-III: Sage 4/5/4, Prophet
  4/7/3, Engineer 2/7/4, Merchant 4/5/4, Bard 4/6/3. A deck of 94 is above the 40-70 first
  estimated: games repeat less, and the content bill is larger.
- **Civ births and Unrest (section 1.9)** carry Mirror Kingdom, Holy See, Free City Charter,
  Charter Company and the Pretender's Song.
- **Strongest cards to watch:** Chronicle of Recall, The Free Company, the Index, Bastion,
  Wheeled City. They may need a hard limit or a bigger price.
- **Where the two decks touch:** the Orrery and Dagda's Balance read and steer the counter;
  Agares's Well and the Ledger of Mammon feed it; the Census of the Damned and Dragon Wake
  (below) give secret-tech draws.

---

## 4. The Armageddon deck (36 new cards)

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
  no longer come in every game. With the 36 new cards, the pool is 42 cards for about 10 slots
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
  Coast (water).
- *70:* the Drowned rise from the new coast as barbarians.

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
The score leader's governed city with the most Unrest secedes as a new civ (section 1.9), led by
its governor, with half its units. Its owner chooses Let Go or Fight.
*Expert:* the leader calms or directly rules its restless cities before 60; rivals foment revolt
in them before it fires.

**Rise of the Barbarian Kings** · 30-50 · single
Every barbarian holding of 3 or more cities within 6 plots of each other crowns a king and
becomes a civ. Orthus leads one if he lives; otherwise a named chief. It researches, trades and
can make peace. Barbarian units near its cities join it.
*Expert:* the barbarian land you farmed for XP now has borders and allies; take its cities
before the crowning, or court the new king.

**The Kurgan** · 30-50 · single (a nomad horde)
A horde civ with no cities appears in the emptiest region: a great army with settlers. Every 5
turns it marches toward the nearest civ and demands tribute (the gifts and demands rules). If
paid, it moves on toward the next civ. If refused, it is at war, and it settles the first city
it takes.
*Expert:* pay it when its next stop is your rival; fight it only from your walls.

**The Free Cities** · 40-60 · single
Every city in the world with Unrest 70 or more revolts at once. Revolting cities within 8 plots
of each other form one civ, the League of Free Cities.
*Expert:* bring Unrest down between the reveal and the threshold. The League is a new partner
for whoever has a navy and a market.

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
Every city in disorder or under siege loses 2 population. The refugees found a new civ, the
Dispossessed, in the nearest unclaimed land: one city per 4 refugees, at least one. They
remember who drove them out.
*Expert:* keep cities out of disorder. The Dispossessed are an ally for whoever did not burn
their homes.

**The Returning** · 60-80 · single
Every destroyed civ whose culture is 20% or more in some city returns. The city where its
culture is highest revolts back to it, with a leader and a garrison.
*Expert:* conquerors who left foreign culture standing pay; assimilate before 60 or hold the
city with a garrison.

**Schism** · 60-80 · single
In every civ whose cities hold two religions of opposed alignment, the cities whose majority
religion is not the state religion secede together as a theocracy of that religion.
*Expert:* keep the faith uniform before 60, or let the heretics go as a buffer.

**The Sundering** · 80 · single
The civ with the most cities splits. The third of its governed cities farthest from the capital
secede together, led by their most senior governor. Its owner chooses Let Go (peace) or Fight
(civil war; each rebel city gets a free unit per 3 population).
*Expert:* the largest empire brings its far cities under direct rule, garrisons or loyal
governors; its rivals court the new civ before it picks a side.

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

- **Count.** 36 new cards: 6 early, 17 middle, 10 late, 3 new endings. With the six FFH2 cards
  in the table above (Apocalypse among them), the pool is 42.
- **Civ births and Unrest (section 1.9)** carry the Hollow Crown, Rise of the Barbarian Kings,
  the Kurgan, the Free Cities, Exodus, the Returning, Schism, the Sundering and Agares
  Unchained.
- **The counter's pace.** Scrapping the random events removes 26 outcomes that moved the
  counter, so its pace needs retuning with the deck.
- **New systems the deck leans on:** culpability; civ births and Unrest; per-city and
  per-player history (ruins, defenders killed, forests cut); terraforming; area weather;
  quarantine. The rest uses existing levers.

---

## 5. Unrest by civilization

### 5.1 How it works (proposal)

- The core of section 1.9 applies to every civ. On top of it, each civilization has its own
  risers and calmers: what its people need and what calms them.
- Three options are drafted per civ. Choose one, or mix them. Each option names one riser and
  one calmer.
- The flavor sets each civ's rhythm: the Doviello must raid on a schedule, the Elohim must not
  start wars, the Sheaim want the world clock to run. Managing Unrest becomes part of playing
  each civ well, so the AI manages it per civ. That follows "civilizations use their unique
  mechanics" (CLAUDE.md).
- A riser must be visible to its owner before it bites. The city screen shows each source of
  Unrest. That includes the Council of Esus, a hidden religion (`hidden`, `sneak_attack`), in
  the owner's own cities.

### 5.2 The options

**Bannor**: bound to fight chaos and disorder.
1. *The oath.* Rises while an evil civ you have met is not at war with you, more the stronger
   it is. Falls with each evil or demon unit killed.
2. *The Order.* Rises in cities without The Order. Falls with each Temple of the Order and
   while the Crusade civic is in force.
3. *Idle swords.* Rises in every city once 20 turns pass at peace. Falls with each battle won,
   doubled during a crusade.

**Malakim**: desert faithful of Lugus, led by an exiled elven prince.
1. *The open sky.* Rises in cities that work no desert, plains or flood plains. Falls per
   desert plot worked, doubled with a Desert Shrine.
2. *Lugus's light.* Rises in cities without the Empyrean while the Empyrean exists in the
   world. Falls with each Empyrean temple and each spread of the Empyrean by your missionaries.
3. *The caravan.* Rises in cities with no trade route to a foreign city. Falls per foreign
   trade route.

**Elohim**: Sirona's shelter for the wise.
1. *Shelter.* Rises in every city when you declare a war. Falls each turn at peace, and with
   refugees you take in (Exodus).
2. *Holy ground.* Rises when a unique feature leaves your borders. Falls per unique feature
   inside them.
3. *Wisdom.* Rises in cities without an Elder Council or Library. Falls with each great person
   born in the empire.

**Mercurians**: angels who care only for the war on demons.
1. *The war in Hell.* Rises in every city while the Infernals live, 1 per 10 demons alive.
   Falls with each demon killed.
2. *Orders.* Rises while at peace with an evil civ. Falls each turn of war against one.
3. *Indifference.* Unhappiness raises no Unrest. Instead it rises with turns your angels spend
   inside your own borders, and falls per angel fighting outside them.

**Luchuirp**: the Open-Skiers, makers of golems.
1. *Idle hands.* Rises in cities where golems outnumber citizens. Falls with each building or
   wonder completed.
2. *The masterwork.* Rises while no wonder, Pallen's Engine or Adularia Chamber is in
   production anywhere in the empire. Falls with each one completed.
3. *The surface.* Rises in cities on hills. Falls per Workshop worked. The rebels become Khazad
   (5.3).

**Kuriotates**: squabbling tribes gathered under a boy-king; few cities, many Settlements.
1. *The tribes.* Settlements gain Unrest by distance to the nearest full city. Falls with each
   luxury available (the Tailor's Fine Clothes, the Jeweler's Jewels).
2. *The court.* Rises in a full city whose culture level is below the empire's average. Falls
   with Legends (the world spell) and each great artist.
3. *Representation.* Rises in Settlements while none has been promoted for 30 turns. Falls in
   every Settlement when one is promoted.

**Ljosalfar**: the Seelie court, returning from the hidden groves.
1. *The grove.* Rises per forest cut within the city's radius. Falls per ancient forest in it.
2. *The court's faith.* Rises in cities where the Council of Esus is present. Falls per
   Fellowship of Leaves temple. Rebels with Esus become Svartalfar (5.3).
3. *The seasons.* Rises in autumn and winter and falls in spring and summer: the two courts'
   reigns. Needs the seasonal cycle (section 1.10).

**Khazad**: dwarves of the deep halls and the vaults.
1. *The vaults.* FFH2's vault levels already turn gold per city into happiness. Unrest rises
   with Empty and Low vaults and falls with Abundant and Overflowing.
2. *Under the mountain.* Rises in cities with no hill or peak in their radius. Falls per mine
   worked. Rebels on flat land become Luchuirp (5.3).
3. *The grudge.* Rises in every city while another civ holds a city taken from the Khazad.
   Falls when one is retaken.

**Sidar**: shades, detached from the world.
1. *Detachment.* Rises during a war the Sidar declared. Falls each turn at peace.
2. *The aberration.* Rises while undead units are within 10 plots of a city. Falls with each
   undead destroyed.
3. *Fading.* A Sidar city at Unrest 100 never revolts. It fades instead: -2 population, and
   its garrison is hidden in the mist for 10 turns. Unrest rises with population above 10 and
   falls as units Wane into shades.

**Lanun**: sea people of Danalin, and pirates.
1. *The sea's call.* Rises in cities with no coast. Falls per ocean plot worked.
2. *Plunder.* Rises once 15 turns pass without a raid at sea (a ship sunk or a coast
   pillaged). Falls with each raid.
3. *The coves.* Falls per Pirate Cove. Rises when a cove is lost.

**Grigori**: free people who want the gods gone.
1. *No gods.* Rises per religion present in a city. Falls with each Adventurer born.
2. *Adventure.* Rises in cities with an Adventurers' Guild while no Adventurer is in the
   field. Falls with each lair explored and each level an Adventurer gains.
3. *Free people.* Rises under Theocracy, God King or Social Order. Falls under Liberty or
   Republic.

**Hippus**: horselords and sell-swords.
1. *The herds.* Rises in cities with no Horses or pasture in their radius. Falls per pasture.
2. *Riders.* Rises per mounted unit idle in a city for 5 turns or more. Falls per mounted unit
   outside the borders.
3. *Sell-swords.* Rises once 20 turns pass without a unit lent or hired out (allied loans, the
   Mercenary Hall). Falls with each one.

**Amurites**: the Children of Kylorin.
1. *The Art.* Rises in cities without a Mage Guild. Falls per spell cast by casters trained
   there.
2. *Kylorin's heirs.* Rises while another civ has more Archmages than you. Falls per Archmage.
3. *The nodes.* Rises with each mana node lost. Falls per node within 5 plots of the city.

**Doviello**: the pack follows the strongest.
1. *Raid hunger* (the owner's idea). Rises 2 a turn in every city once 15 turns pass without a
   sack (a city captured or razed) or a tribute paid to them. Falls 30 in every city with each
   sack or paid demand.
2. *The weak are left behind.* Rises in cities smaller than half the empire's average size.
   Falls with each wolf pack formed in the city.
3. *The strongest leads.* Rises while a civ you have met has more power than you. Falls with
   each battle won by your largest army.

**Balseraphs**: Perpentach's court of freaks, puppets and mirrors.
1. *Boredom.* Rises each turn without entertainment. Falls with each Arena battle, each freak
   added to the Freak Show, and each Revelry.
2. *Sanity.* Rises each turn the leader's traits stay the same. Perpentach's Insane trait
   resets it when it reshuffles them. Under Keelyn it rises while no summon is active.
3. *The audience.* Falls per Puppet and Harlequin in the city. Rises while the Hall of Mirrors
   stands unbuilt in an empire of 6 cities or more.

**Clan of Embers**: the horde, kin to the barbarians.
1. *Kin.* Rises when barbarians near your borders are destroyed by other civs. Falls when
   barbarian units join you.
2. *The warrens.* Rises in cities of population 8 or more without Warrens. Falls per unit
   trained in a city with Warrens.
3. *The horde must move.* Rises in every city once 25 turns pass without a city taken. Falls
   with each city taken.

**Svartalfar**: the Unseelie court.
1. *Intrigue.* Rises once 15 turns pass without a successful Kidnap or espionage mission.
   Falls with each.
2. *The court's faith.* Rises in cities where the Fellowship of Leaves is present. Falls with
   the Council of Esus. Rebels with the Fellowship become Ljosalfar (5.3).
3. *The night.* Rises in spring and summer and falls in autumn and winter. Needs the seasonal
   cycle (section 1.10).

**Calabim**: vampire lords and their cattle.
1. *The cattle.* Rises 10 with each Feast in the city. Falls with a Breeding Pit and a
   Governor's Manor.
2. *The hungry nobles.* Rises in cities where no vampire has fed for 20 turns. Falls with each
   Feast. (The opposite pull of option 1; take one.)
3. *Sunlight.* Rises with The Order or the Empyrean present. Falls with the Ashen Veil. The
   rebels are the cattle, a human civ (5.3).

**Sheaim**: gathered from every people to end creation.
1. *The cause.* Rises when the Armageddon counter falls. Falls when it rises.
2. *The gates.* Falls per Planar Gate summon alive. Rises with each summon destroyed.
3. *The gathered.* Foreign culture raises Unrest twice as fast as in the core. Falls with each
   foreign city that takes the Ashen Veil.

**Infernals**: Hyborem's host.
1. *Hunger for souls.* Rises each turn. Falls per unit killed by Infernal units.
2. *The angels.* Rises while the Mercurians live. Falls with each angel killed.
3. *Hell on earth.* Falls per hell terrain plot in the radius. Rises per plot of ordinary
   terrain.

**Illians**: the lost men of Letum Frigus.
1. *Winter.* Rises per worked plot that is not snow or tundra. Falls per snow plot.
2. *The god-king.* Falls with each step of Auric's rise (the Draw, Ascension). Rises once 30
   turns pass without one.
3. *The world sleeps.* Rises while another civ researches faster than you. Falls while Stasis
   holds.

### 5.3 Who the rebels become

By default, rebels found a civ of their own civilization under a new leader. Five
civilizations' rebels become something else, and in each the rule turns on something the owner
can see and manage:

- **Ljosalfar and Svartalfar, by faith.**
  - Ljosalfar rebels whose cities hold the Council of Esus become Svartalfar. Svartalfar rebels
    whose cities hold the Fellowship of Leaves become Ljosalfar. Rebels without the other
    court's faith stay their own civ.
  - If the other court is already in the game and has met them, the rebels join it, as the
    courts reclaim their own. Otherwise they found it, led by a court noble (or Faeryl or
    Arendel when not in play).
  - The lore is this civil war. The Svartalfar's own text: the Unseelie court, in power when
    Sucellus died, decreed Esus in his place, punished the elves who would not stop
    worshipping Sucellus, and set off a civil war that only the Age of Ice stopped.
- **Khazad and Luchuirp, by ground.** Khazad rebels whose cities sit on flat land become
  Luchuirp, the Open-Skiers. Luchuirp rebels on hills become Khazad. The Luchuirp's text calls
  the Khazad their cousins who kept the caves.
- **Mercurians and Infernals, by faith.** Mercurian cities where the Ashen Veil is the majority
  fall as Infernals. Infernal cities with The Order rise as Mercurians, led by a Repentant
  Angel.
- **Calabim, by species.** The cattle rise: Calabim rebels become a human civ not in play
  (Bannor, Malakim or Elohim, by the rebels' majority religion).
- **Balseraphs: Perpentach again** (the owner's idea).
  - The first Balseraph secession founds a Balseraph civ led by *Somehow Also Perpentach*, with
    Perpentach's traits, Insane included. Later secessions are led by *Perpentach the Other*,
    *Definitely Perpentach* and *Perpentach (Original)*.
  - Every Perpentach has -2 attitude toward every other one ("Impostor").
  - When a Perpentach civ is destroyed, the Perpentach with the most cities gains one of its
    traits.
  - Under Keelyn the rebels are still led by a Perpentach. Nobody can explain it.

Unit and building conversion uses the class mapping crusade Hosts already use: a Fyrdwell
becomes a Nyxkin, a Flurry a crossbowman, a Dwarven Smithy a Forge. Heroes stay with the
parent.

---

## 6. What the four general mechanisms make possible

The four mechanisms of section 1.10: timed statuses, history counters, scheduled effects, hooks.

### 6.1 How to build them so they pay

- **A status is declared, not scripted.** It has:
  - a name;
  - a scope (city, plot, player, unit);
  - a duration (turns, or until a condition);
  - modifiers from the existing lever vocabulary (yields, Unrest, attitude, combat stats,
    movement, research, ...);
  - optionally a per-turn effect.

  The help text and the AI read the same declaration, so a new status costs neither help code
  nor AI code. A status that runs arbitrary code needs both, every time. Keep the few verbs that
  need code (spawn, convert, terraform) as named effects that a status or schedule calls.
- **Hooks attach to the moments that already exist:** the ~40 `ev_*` callbacks ported from
  FFH's event manager (`crates/ffh-core/src/ffh/events.rs`: `ev_city_built`, `ev_city_razed`,
  `ev_city_acquired`, `ev_combat_result`, `ev_great_person_born`, `ev_building_built`,
  `ev_change_war`, `ev_first_contact`, `ev_golden_age`, ...). A card, a trait or a civilization
  registers a hook in data: a moment, a condition and an effect.
- **History counters** are a keyed table per city, plot and player (a `BTreeMap`, saved),
  written by hooks. Plots already keep `ownership_duration`; that is the pattern.
- **Scheduled effects** are a queue of (turn, target, effect), public by default. Secrecy is a
  property a card grants explicitly.
- **They can absorb the one-off timers.** City: `occupation_timer`, `happiness_timer`,
  `hurry_anger_timer`, `defy_resolution_anger_timer`. Player: `sanctuary_timer`,
  `golden_age_turns`. Plot: `temp_terrain_timer`, `force_unowned_timer`. Unit:
  `immobile_timer`. This is optional cleanup, not a requirement.
- **Everything is visible.**
  - Every status shows its icon and the turns left.
  - Counters appear on the city and leader screens: each game's Chronicle.
  - A Coming panel lists the public schedule: loans due, truces ending, prophecies, the next
    Armageddon card.

  That is the forecast leg of the roguelike principle, and what lets an expert plan.

### 6.2 Systems built from them

The payoff is in combinations. Each entry names the mechanisms it uses.

**Vendetta** (hook, history, status, schedule)
Losing a city gives the loser *Vengeance on X* for 30 turns: +20% attack against X's units and
no war weariness against X. Retaking the city in time gives *Triumph* (+2 happiness empire-wide,
10 turns).
*Expert:* retake it fast or not at all. The captor knows the window and holds the city for 30
turns.

**Scars of war** (history on plots, status)
A plot where 10 or more units died becomes a *Battlefield*. Left alone, it is *Haunted*: ghosts
rise against whoever stands there. With a memorial built on it, it is *Hallowed*: +2 culture,
and defenders on it start battles with +10% damage. Over a long game the map gathers the scars
of its wars, different every game.
*Expert:* build memorials on the battlefields you will fight over again.

**Salted earth** (hook, status, history)
Razing a city may *Salt* its plots for 50 turns: no food, no new city. The ruin keeps the
razer's name, which feeds claims and Children of the Ash.
*Expert:* deny a contested site to a rival who cannot hold it either.

**Epithets** (history, permanent player status)
History counters past a threshold give a leader a public epithet with its own effects:
- *the Razer* (5 cities razed): -2 attitude from good civs, +1 from evil.
- *the Oathbreaker* (a sworn peace or treaty broken): no civ accepts sworn peace or hostages
  from you for 50 turns.
- *the Merciful* (cities released or spared): +1 attitude from everyone; released cities start
  Friendly.
- *the Dragonslayer*: +1 attitude from good civs, and dragons avoid your lands.
- *the Builder* (5 wonders): +10% wonder production.

*Expert:* an epithet is a reputation you choose by how you fight; plan the one you want.

**Curses and blessings with a fuse** (status, schedule)
- A curse that lands later: *this city burns in 15 turns unless dispelled*.
- A blessing that turns: *Sanctified while its founder holds it, Desecrated if captured*.

*Expert:* a short-term priority with a clear answer: dispel in time, or move what matters out.

**Prophecy** (schedule, hook)
A Prophet announces: *City X will fall within 30 turns*. If it falls by anyone's hand, the
prophet's religion spreads to every city of the civ that took it. If it stands, the prophet's
holy city loses its holy-city commerce for 30 turns.
*Expert:* prophesy against a city your neighbours already want; third parties get a reason to
act, and the target gets a warning.

**Contracts that come due** (schedule, hook, history)
Loans, tribute, hostage release, truce ends, allied pledges and debt defaults, all on the Coming
panel. A missed payment fires its hook, which may give a casus belli or an epithet.
*Expert:* time wars to the calendar of what others owe and are owed.

**Mourning and succession** (hook, status, schedule; once leaders are mortal)
A leader's death gives *Mourning* for 10 turns (no war declarations, -10% production) and
schedules the coronation. The heir's traits are revealed at the coronation. Rivals see the
window, and claimants (Unrest, a Pretender) may rise in it.
*Expert:* the turns before a coronation are the moment to strike, or to make peace.

**Earned unlocks** (hook, history)
Abilities unlocked by what a civ has done, not by research:
- 10 battles won in forests: your archers learn *Forest Volley*.
- 3 dragons slain: Dragon Slayers for all your champions.
- 5 cities founded on coast: *Seafaring* moves.

*Expert:* play shapes the tree; an expert leans into the deeds the civ already does.

**Old growth** (history on plots, status)
A forest untouched for 100 turns becomes ancient forest. A desert worked for 100 turns becomes
an oasis. The map ages by how it was used.
*Expert:* leave the forest belts alone; they grow into walls (Heartwood of Sucellus).

**Claims** (history, hook)
A civ holds a claim on a city it owned for 20 turns or more, fading over 50 turns after losing
it. A claim lowers the war weariness of a war to retake the city and is a war goal (section
1.10).
*Expert:* reconquest wars are cheap for a while; the captor's defence plan follows the claim's
fade.

**Governor ambitions** (hook, status)
A governor wants a building, a war or a post. Getting it makes the governor *Content* (loyal);
being denied for 20 turns makes it *Resentful* (Unrest, and it may lead a secession).
*Expert:* read the ambitions before choosing which city a governor gets.

### 6.3 What else turns into data

- **Civilization flavor** (section 5):
  - the Doviello's raid hunger is a counter (turns since a sack) and two hooks (city captured,
    tribute paid);
  - the Balseraphs' boredom is a hook on Arena battles;
  - the Sheaim's cause is a hook on the counter's change.

  Most of section 5 becomes data.
- **Leader traits** can be written as hooks. For example *Vengeful*: when a city of yours is
  captured, Vengeance lasts 50 turns.
- **The Armageddon deck** is a schedule. Its reveal, its lingering cards and its chains are
  statuses and scheduled stages.
- **The objectives deck** is scored by hooks and counters.
- **The great-person deck's** non-tech effects (the Index's draws, the Lens's copy, the Recall's
  return) are hooks.

### 6.4 Great-person cards built on the four (24 candidates)

These go into the pool beside the 94 of section 3. Each names the mechanisms it uses: status,
history, schedule, hook.

**Sage**

**Contingency** · II · needs Sorcery · promotion for mages · [status, hook]
A mage prepares one contingency, which holds until it fires:
- *When my stack is attacked:* cast my strongest battle spell before the battle opens.
- *When I would die:* return to the capital at 1 HP.

*Expert:* the first on the army's mage, the second on the archmage you cannot replace.

**The Long Experiment** · II · needs Alteration · building · [schedule]
Start an experiment: three outcomes are drawn (a free tech of the next era, a permanent mana
resource of a type, a unique unit) and you choose one. It completes in 40 turns, publicly. If
the city is captured first, the captor gets the result.
*Expert:* run it in the safest city; every rival knows the date and has a reason to take that
city.

**The Predictive Engine** · III · needs Machinery · wonder (national) · [schedule]
You see every scheduled effect in the world, secret ones included. Once every 30 turns, delay
one of them by 10 turns: a loan's due date, a truce's end, a curse's fuse, an Armageddon reveal.
*Expert:* delay the curse on your city, or the end of the truce with the strong neighbour.

**Temporal Anchor** · III · needs Pass Through the Ether · spell · [status, schedule; heavy]
Anchor a city of yours for 10 turns. When the anchor ends, the city returns to its state at the
anchoring: its population, buildings and owner.
*Expert:* anchor the city the enemy is about to take. They take it, and lose it again when the
anchor ends.

**Prophet**

**Book of Prophecies** · II · needs Divination · national unit, 1 · [schedule, hook]
Prophesy: *City X will fall within 30 turns*. If anyone takes it, your religion spreads to every
city of the taker. If it stands, your holy city loses its holy-city commerce for 30 turns.
*Expert:* prophesy against a city your neighbours already want.

**Martyrs' Calendar** · II · needs Priesthood · building (holy city) · [history, status]
Each of your religion's disciples killed is a martyr. The city that trained it is *Blessed* for
20 turns (+2 happiness, +2 culture). At 10 martyrs, the religion gains a Saint, a hero.
*Expert:* a war on your faith strengthens it; rivals who know this leave your priests alone.

**Holy Days** · II · needs Religious Law · building (holy city) · [schedule, status]
Every 20 turns, every city of your religion, in any civ, keeps 3 Holy Days: +2 happiness, -25%
production. A civ that is at war with you during Holy Days takes *Sacrilege*: -2 attitude from
every civ of the religion.
*Expert:* time your wars between the Holy Days; a co-religionist enemy cannot.

**Relic Cult** · I · needs Ancient Chants · building · [hook, status]
When a hero of yours dies, its relic rests in the nearest city of yours. That city is
*Sanctified* while it holds the relic: +3 culture, -2 Unrest a turn, and units trained there get
+1 XP. A relic is taken with its city.
*Expert:* a dead hero still pays; relic cities become war goals for both sides.

**Oath of the Blood-Price** [evil or neutral] · II · needs Warfare · wonder (national) · [history, status; Vendetta]
Once every 50 turns, declare Vengeance (section 6.2) on any civ that has ever taken a city of
yours. It needs no fresh loss and lasts 40 turns.
*Expert:* old grievances become war bonuses on your schedule.

**Excommunication** · III · needs Theology · spell (holy city owner) · [status]
Excommunicate a civ whose state religion is yours. For 30 turns its cities of your religion
gain +2 Unrest a turn, its disciples cast no spells, and every civ of the religion takes -2
attitude toward it.
*Expert:* break a rival that shares your faith before a war; it will not be helped by its
brothers.

**Engineer**

**Walls that Remember** · I · needs Masonry · building · [history]
A city's Sentinels gain +20% HP for each siege the city has survived (permanent).
*Expert:* the border city that held becomes the fortress; never let a besieger retreat
cheaply.

**Snares** · I · needs Hunting · worker build · [status on plots]
A hidden trap on a plot in your borders, lasting 30 turns. The first enemy stack to enter takes
15% damage and ends its move.
*Expert:* trap the approach roads in the turns before a war you expect.

**Floodgates** · II · needs Engineering · building (city on a river) · [status on plots]
Once every 20 turns, flood the river plots within 4 plots of the city for 10 turns: they become
marsh (temporary terrain), impassable to enemy units.
*Expert:* open the gates the turn the invasion reaches the river.

**Time-Locked Vault** · II · needs Currency · building · [schedule]
Deposit gold. In 30 turns, publicly, it returns doubled. If the city falls first, the captor
gets the deposit.
*Expert:* the safe city's investment; rivals see the date and the sum.

**Scorched Earth** · II · needs Smelting · spell (any unit, own borders) · [status on plots; needs attrition]
A plot in your borders gives no yield and no healing for 15 turns. With supply and attrition
(section 1.10), an invader there starves.
*Expert:* burn the invaders' path, not your own cities' land.

**Merchant**

**Letters of Credit** · I · needs Currency · deal item · [schedule, history]
Buy now, pay in 20 turns at 25%. A missed payment gives the creditor a claim on your nearest
city and you the *Debtor* epithet for 30 turns.
*Expert:* borrow for a war you will win before the date.

**Blood Money** · II · needs Code of Laws · deal item · [status; Vendetta]
Offer gold to end a Vengeance against you, priced by the size of the city taken. The avenger
may refuse. If it accepts, its Vengeance ends and it cannot start another against you for 50
turns.
*Expert:* buy off a vendetta you cannot fight; refusing it is the avenger's statement.

**Favoured Partner** · II · needs Trade · building (national) · [history]
Each 10 turns of unbroken trade with a civ adds a step. At 5 steps: permanent +2 attitude and
one more trade route with that civ. Breaking a deal with it resets the count and costs -2
attitude.
*Expert:* long partnerships pay more than any single deal; choose the partner who will last.

**The Black Book** · III · needs Guilds · building (national) · [history]
You see every civ's history counters (treaties broken, cities razed, debts). Once, publish one:
the target gains the matching epithet for 50 turns.
*Expert:* publish the Oathbreaker's record before the council votes.

**Bard**

**The Living Chronicle** · I · needs Ancient Chants · wonder (national) · [history, hook]
Each first in your history is a verse: first wonder, 10 battles won, a city held through 3
sieges, a dragon slain. Each verse gives +5 culture a turn. Every 5 verses add +1 happiness in
every city.
*Expert:* every deed counts once; seek the firsts you have not done.

**Songs of Grievance** · II · needs Drama · building · [history, Vendetta]
While you hold a claim on a city (section 6.2), your cultural channel aimed at its holder
counts double against that city.
*Expert:* lost cities come back by culture, without a war.

**Hall of Heroes** · II · needs Arete · building · [history]
A unit trained in this city may take the name and one promotion of a fallen hero of yours.
*Expert:* a dead hero's best promotion lives on in your next champion.

**Rumour Mill** · II · needs Drama · building · [status on players]
Once every 30 turns, set a rumour on a leader for 20 turns:
- *Weak:* the AI counts it at half its power.
- *Treacherous:* no deals with it.
- *Plague-ridden:* no open borders with it.

*Expert:* make the leader look weak to his neighbours the turn before you need allies against
him.

**The Feast of Ceasefire** · I · needs Festivals · building · [schedule, status]
Announce a world feast 10 turns ahead, lasting 5 turns. Civs at peace with you through it gain
+2 attitude toward you. Any civ that declares war during it takes the epithet *the Spoiler* for
50 turns.
*Expert:* schedule it over the turns you need peace; it binds even those who dislike you.

### 6.5 Armageddon cards built on the four (12 candidates)

These go into the pool beside section 4's.

**Omens** · 20 · single [schedule]
Every civ is told its omen: an effect personal to it, scheduled 30 turns ahead and drawn from a
small set:
- famine in its largest city;
- a great person;
- a governor who turns Disloyal;
- a meteor on a named plot;
- a hero who will be born in a named city.

*Expert:* prepare for your omen; read your rivals' (they are public) and plan around them.

**The Signs** · 30 · recurring [status, schedule]
From 30 on, the sky shows a sign every 10 turns: a world status for 10 turns. The next two signs
are always shown. The signs belong to the angels, replacing FFH2's Sign events, which were only
golden ages:
- *Sign of Camulos:* war declarations cost no attitude.
- *Sign of Junil:* war declarations cost -2 attitude with everyone.
- *Sign of Esus:* espionage costs half.
- *Sign of Kilmorph:* +25% production.
- *Sign of Mulcarn:* no healing outside cities.

*Expert:* time each move to the sign that favours it; the shown two signs are a schedule.

**The Unforgiven** · 40 · single [history, status; Vendetta]
Every civ that ever lost a city to another gets Vengeance on its captor for 30 turns, all at
once.
*Expert:* the conquerors brace; the conquered strike together. Know your history before 40.

**Witching Nights** · 40 · recurring [schedule, status]
For 50 turns, every 10th turn is a Witching Night (shown in advance): summons last twice as long,
undead deal +25% damage, and divine spells fail.
*Expert:* evil civs attack on the witching turns; good civs dig in for them.

**The Reckoning of Debts** · 50 · single [history, schedule]
Each civ's broken treaties and razed cities are counted. Each is a debt, collected in 10 turns:
a population from the largest city, or a unit of the civ's choice.
*Expert:* the Oathbreaker and the Razer pay; between the reveal and the date, a clean record is
worth something.

**Mark of the Beast** · 50 · single [status, hook]
The most culpable civ's units bear the Mark. Each kill by a Marked unit adds 1 to the counter;
each Marked unit killed takes 1 off.
*Expert:* the marked civ fights to hasten the end; everyone else gains by killing its units.

**Harvest of Souls** · 55 · lingering [hook]
While the counter is 55 or more, every 20 unit deaths in the world add 1 to the counter.
*Expert:* wars hasten the end, so the evil civs start them and the good civs push for peace
(the Grey Council's Truce of Dagda).

**The Hollow Years** · 60 · single [status]
For 20 turns, no great person is born anywhere. Great-people points accumulate at double rate
during them.
*Expert:* the deck's draws pause; stock specialists for the burst after.

**The Dead Remember** · 60 · single [history]
On every plot where 5 or more units died, Haunted dead rise, hostile to all; the more dead, the
more of them.
*Expert:* the old battlefields are the danger; clear or avoid them.

**The Forgetting** · 65 · single [history]
The world forgets:
- every history counter, epithet and claim is wiped;
- every Vengeance ends;
- memory-based attitude resets to neutral.

*Expert:* the Oathbreaker starts clean, and the claims you were counting on are gone. Settle
old wars before 65 or after.

**Ash of Empires** · 70 · single [history]
Every ruin of a razed city becomes Hellfire with a demon. The razers' lands get the demons.
*Expert:* every razing is a debt that comes due at 70.

**The Countdown** · 70 · single [schedule]
The counter stops moving up. The Apocalypse is scheduled 50 turns away, publicly. It is
cancelled only if the counter is brought below 50 before the date.
*Expert:* the whole world gets one goal and a deadline; the good civs organise the Penance,
and the Sheaim defend the date.

---

## 7. The self-contained rule, and every card checked against it

### 7.1 The rule

A card's or a crisis's main payoff must come from mechanics the player controls or can
calculate: yields, combat, movement, the map, timing, Unrest, the counter.

An effect whose value depends on how rivals choose to react may be a side effect, never the
reason to take the card. This covers attitude, deterrence, deals a rival must accept, votes, a
rival taking bait, and a rival being fooled.

Risk from the board is fine. "The city might be attacked" is a fact the player prices into his
own plan. It is not a bet on one particular response.

The rule also makes the AI's job tractable. Playing a card becomes using mechanics it can
calculate, not modelling what other AIs will do.

### 7.2 What fails, and the fix

- **Attitude as the payoff.** Replace it with Unrest, combat, yields or production.
- **Deterrence** ("breaking it costs attitude"). Replace it with **enforcement**: the thing
  cannot happen for N turns.
- **Deal items that need consent.** Make them unilateral actions or rules. A deal's value
  starts with an AI saying yes.
- **Bait** (a public prize that lures rivals; prophesying what the neighbours want). Make it a
  **wager on your own action**: you commit to doing something, with a reward and a forfeit.
- **Deception aimed at what the AI perceives** (false colours, rumours, false strength).
  Replace it with the mechanical form of surprise: ambush, sneak attack, invisibility. The
  Council of Esus' `sneak_attack` is an existing lever.
- **Votes.** Replace them with a seal held and used unilaterally.
- **AI allies the card creates.** Keep the cities yours (governed), or give them enforced
  protection.

### 7.3 Verdicts

**Pass** (the payoff is the player's own mechanics):
- **Sage:** the Second Sky, Familiars, Homunculus Vats, the Index, the Calculus of Chance, the
  Grammar of True Names, Waystones, Mana Grafting, the Orrery, the Plagiarist's Lens, the
  Recursive Lemma, the Chronicle of Recall, Contingency, the Long Experiment, the Predictive
  Engine, Temporal Anchor.
- **Prophet:** Maponos's Cradle, Breath of Tali, the Lantern of Lugus, Agares's Well, Tidecall,
  the Heartwood, the Ledger of Mammon, the Ossuary, the Pilgrim Road, the Brand of Aeron,
  Dagda's Balance, the Martyrs' Calendar, the Relic Cult, the Oath of the Blood-Price.
- **Engineer:** every card.
- **Merchant:** the Black Market, the Ledger of Nations, Insurance, the Mercenary Hall, the
  Caravanserai, Cornering the Market, the Paymaster, Merchant Princes.
- **Bard:** the Saga, War Drums, Siren Rock, the Lullaby, the Opera, Anthem, Living Legend, the
  Ballad, the Chorus, the Illusory Host, the Living Chronicle, Songs of Grievance, the Hall of
  Heroes.
- **Capabilities:** every civic but Tribute State and the Grey Council; the processes and the
  hurry; Spymasters (without Forge Letters); movement, specialists, improvements.
- **Armageddon:** every card not named below, including Vendetta's crisis form (the
  Unforgiven), Omens, Witching Nights, the Reckoning, the Mark, the Hollow Years, the Dead
  Remember, the Forgetting, Ash of Empires.

**Reworked** (new text replaces the card's):
- **Mirror Kingdom → Mirror Colony.** Two cities of yours appear on the largest unclaimed
  region, governed, joined to your capital by a portal for land units.
- **Holy See.** The See stays yours. Civs of its religion cannot attack it (enforced), and it
  trains its religion's heroes and units at half cost.
- **Charter Company → Company Towns.** The colony is yours and governed. Its governor trains
  settlers and founds cities on that landmass on its own.
- **Scales of Junil → Judgment of Junil.** Once every 40 turns, name a civ: neither of you can
  declare war on or attack the other for 20 turns (enforced, no consent).
- **Masks of Esus → False Colours.** The stack may attack a civ you are at peace with without a
  declaration; war follows the battle. In that first battle the defenders get no defensive
  strikes and no fortification.
- **Masquerade.** The stack is invisible to rivals until it is next to their units. Its first
  battle from hiding denies the defenders' defensive strikes.
- **Hollow Mountain.** The hidden garrison also strikes first in the first battle at the city in
  each war (it no longer relies on the enemy being fooled).
- **Free City Charter.** The Free City cannot be attacked by anyone for 50 turns (enforced),
  and pays you half its commerce.
- **Satire of Kings.** The target's cities within 10 plots of yours gain +2 Unrest a turn for
  20 turns.
- **Bounty Office → Bounty.** Name a civ: for 30 turns your units earn 15 gold and +1 XP per
  kill of its units.
- **The Grey Council → Dagda's Seal** (Membership civic). Once every 40 turns, impose the Truce
  of Dagda (no war declarations in the world for 10 turns) or Hold the Counter (20 turns), with
  no vote.
- **Embassies.** Open an embassy with any civ you have met, without its consent.
- **Shared Vision → Scrying Pool** (Sage). See what one civ's units and cities see for 10
  turns, once every 20.
- **Passage of Arms → Forced March.** Once every 30 turns per civ, your armies may cross its
  land for 5 turns without open borders or war, staying in supply.
- **Letters of Credit.** Borrow from the Guild (the rules), not a civ: gold now, repaid with 25%
  over 20 turns. A missed payment: -25% production in every city until it is paid.
- **Blood Money.** The avenger cannot refuse: pay (priced by the city's size) and its Vengeance
  against you ends.
- **Book of Prophecies → Prophecy of Ruin.** Name a rival city.
  - If *you* take it within 30 turns: your religion spreads to every city of the victim, and the
    city keeps all its buildings.
  - If you do not: your holy city loses its holy-city commerce for 30 turns.

  It is a wager on your own plan.
- **Holy Days.** Sacrilege becomes mechanical: units of civs at war with you during Holy Days
  deal -20% damage to your units.
- **Excommunication.** The attitude part goes; the Unrest and the silenced disciples stay.
- **The Feast of Ceasefire.** For its 5 turns, announced 10 ahead, no civ can declare war on
  you, and you cannot declare war either (enforced).
- **The Signs.**
  - Sign of Camulos: +25% attack for every unit in the world.
  - Sign of Junil: no war declarations while it lasts.
- **The Countdown.** Cancelled if the counter falls below 50, or when any one civ lowers it by
  10 with its own Penance before the date.
- **Epithets** (section 6.2). Each gives its holder a mechanical effect, not only attitude:
  - the Razer: +50% gold from razing, and -1 happiness per 5 cities razed;
  - the Oathbreaker: -10% damage against the civ whose treaty was broken, for 30 turns;
  - the Merciful: released cities pay you 10% of their commerce;
  - the Dragonslayer: +25% damage against large units;
  - the Builder: +10% wonder production.

**Cut** (no mechanical core once the response is taken out):
- Debt Bonds: needs an AI to borrow and default.
- Hostages: needs an AI to break a peace.
- Royal Marriage: needs consent, and pays in attitude.
- Favoured Partner: needs an AI to keep trading.
- Rumour Mill: pays in an AI's misjudgment.
- Tribute State: needs AIs to pay demands.
- Spymasters' Forge Letters: pays in attitude.
- The Black Book's "publish": the record stays visible to you.

**Kept, flagged as depending on an AI:**
- **The Free Company.** Its war is waged by the crusade Host's AI. That AI is ours, purpose-built
  and measurable, but the card is only as good as it is.
- **The Pretender's Song.** The friendly new civ is a side effect; the Unrest is the payoff.
- **The Unbinding of Oaths.** Its effect on you is mechanical (deals end, vassals may go), but
  much of its weight is rivals' new freedom to attack.
- **The Pale Election.** The Herald's bonus is mechanical; the crusade call rests on AI
  pledges.
- **Toll Roads.** The war effect is the payoff; the gold from foreign movement is a side
  effect.

**Expert lines to rewrite** (the mechanics pass, but the line leaned on rivals):
- the Long Experiment and the Time-Locked Vault: "run it where you can hold the city for the
  term";
- the Martyrs' Calendar: drop "rivals leave your priests alone";
- Harvest of Souls: "your own wars raise the counter: fight short";
- the Unforgiven: "if you are owed, strike in the window; if you owe, garrison the cities you
  took".

---

## 8. Succession, the integrated version (draft)

The goal: succession that adds no new stats or screens. Its inputs are state the game already
has (traits, governors, history counters, Unrest), and its outputs are levers it already has
(trait changes, statuses, Unrest, secession).

**What it leaves out**, because each piece is a game of its own beside the 4X:
- ages and health;
- family trees, marriages, dynasties;
- court intrigue;
- character stats apart from traits.

**Why not age.** Half of FFH's leaders do not die of age:
- the angels Sabathiel, Cassiel and Basium; the demon Hyborem;
- the vampires Alexis and Flauros;
- Tebryn, released from Hell; Os-Gabella, made immortal; the shade Sandalphon;
- Auric, who means to become a god;
- the elves.

The trigger has to come from play.

### 8.1 The three parts

**1. What changes is the leader's traits.** Traits carry nearly all of a leader's mechanics,
and swapping them exists:
- Adaptive swaps a trait every 100 turns (normal speed), keeping the leader's permanent trait.
- Insane swaps at random (2% a turn).
- Both go through `player_set_has_trait` (`crates/ffh-core/src/ffh/events.rs`,
  `random_events_a.rs` `do_trait`).

A successor keeps the civilization's permanent trait (its identity) and brings the rest.

**2. The heirs are your governors.** The game already has characters with names, profiles and
traits. A governor's traits become leader traits:

| Governor trait or profile | Leader trait |
|---|---|
| Industrious, Builder | Industrious |
| Martial, Marshal | Aggressive |
| Drillmaster | Organized |
| Beloved | Charismatic |
| Frugal, Merchant | Financial |
| Learned, Scholar | Philosophical |
| Patron | Creative |
| Farmer | Expansive |
| Zealot | Spiritual |

The bad governor traits (Harsh, Lazy, Corrupt) come along as the heir's flaw. Governors should
also grow traits with time in a city (the Living Legend card's rule made general), so that
choosing where a governor serves is also grooming an heir. No second character system.

**3. The trigger is play, in two kinds:**
- **The regular stream: reigns.** At each Adaptive cycle (every 100 turns, normal speed), every
  leader's non-permanent traits are redrawn from an offer of three. The offer is shaped by
  history counters: a civ that fought a lot is offered Aggressive or Raiders, one that built
  wonders Industrious or Creative. It is Adaptive for everyone, earned by deeds. It delivers the
  roguelike stream (new information that changes the plan, on a known schedule) without
  anyone dying.
- **Rare true successions,** when the throne is lost in play:
  - the capital falls;
  - the leader dies in the field. This is opt-in: a leader may take the field as a hero unit
    with an aura for its army, as Hyborem and Basium already are both leaders and units.

  The heir is chosen from your governors.

### 8.2 The weak moment

It uses the primitives of section 6:
- *Mourning*, a status for 10 turns;
- the coronation, scheduled and public;
- passed-over governors with Ambitious or Popular traits become claimants: their cities gain
  Unrest, and may secede (section 1.9).

All of it is the player's own governors and Unrest, so it passes the self-contained rule
(section 7).

### 8.3 Expert play

- **Governors.** Placement gains a second dimension: grooming an heir in a safe city.
- **The leader in the field.** Its aura against the risk of a succession is a calculation, not
  a guess.
- **Reigns.** The trait offers pivot strategy on a known schedule. The choice is local (one of
  three), so the AI can make it from its own plan.

### 8.4 Open points

- **The AI's personality after a succession.** Today the leader's flavors, favourite civic and
  alignment steer the AI. The heir's governor profile has flavors, so the AI could follow the
  heir. Alignment stays the civilization's.
- **The successor's name and portrait.** Governor names exist; leader portraits exist only for
  the named leaders.
- **The Balseraphs.** Every heir is named Perpentach.

---

## 9. The pool as it stands

Every card that meets the criteria: an expert line, a payoff that does not rest on how rivals
respond (section 7), and visible before it bites. Where section 7 reworked a card, this is the
reworked card. Cut cards are left out. The five flagged as depending on an AI are listed at the
end. Band: I Ancient, II Classical, III Medieval.

### 9.1 Great-person cards (109)

**Sage (20)**
- *The Second Sky* (I): reveals every mana node, unique feature, lair and goody hut, for good.
- *Familiars* (I): a unit bound to a caster takes the first spell or shot aimed at it.
- *Homunculus Vats* (I): a frail caster that knows one chosen sphere I battle spell without the
  mana.
- *The Index of Unwritten Books* (I): order the deck's top three. Every later great person
  draws two, and you keep one.
- *Scrying Pool* (I): see what one civ sees for 10 turns, once every 20.
- *The Calculus of Chance* (II): +10 crit for units trained. The Augur's charm turns each
  unit's first miss into a hit.
- *Grammar of True Names* (II): name a rival unit type; ignore its evasion and half its magic
  defence for 25 turns.
- *Waystones* (II): mages fold their stack between Waystone cities.
- *Mana Grafting* (II): turn a mana node into another type you own.
- *Orrery of Ceridwen* (II): see the next two Armageddon cards and their thresholds.
- *Stoicism* (II, civic): war weariness causes no unhappiness, and unhappiness no Unrest;
  -15% culture.
- *Dagda's Seal* (II, membership civic): once every 40 turns, impose the Truce of Dagda or Hold
  the Counter, with no vote.
- *Contingency* (II): a mage's prepared trigger: a battle spell before the battle opens, or a
  return home instead of death.
- *The Long Experiment* (II): choose one of three drawn rewards, delivered in 40 turns. The
  captor gets it if the city falls first.
- *The Plagiarist's Lens* (III): once, learn a secret tech another civ holds.
- *The Recursive Lemma* (III): one sphere's battle spells are cast twice.
- *The Predictive Engine* (III): see every scheduled effect; delay one by 10 turns every 30.
- *Temporal Anchor* (III): a city returns to its anchored state, owner included, after 10
  turns.
- *Chronicle of Recall* (III): once, every unit lost in the last 3 turns returns.
- *Mirror Colony* (III): two governed cities of yours on the largest unclaimed region, joined
  to the capital by a portal.

**Prophet (25)**
- *Maponos's Cradle* (I): a free Settler every 20 turns without losing population.
- *Breath of Tali* (I): the Windcaller's Tailwind, +2 speed in round 1; +1 move on flat land.
- *Lantern of Lugus* [good] (I): a holy shooter with Glare (-15 enemy accuracy); sees
  invisible.
- *Agares's Well* [evil] (I): a mana resource of your choice; +3 counter.
- *Relic Cult* (I): a dead hero's relic Sanctifies the city that holds it.
- *Ancestor Worship* (I, civic): cities you founded calm with age; captured cities are
  restless.
- *Vigil* (I, process): production lowers the city's Unrest.
- *Forest Paths* (I): forest and jungle are open ground for you, -1 move for invaders.
- *Wayside Shrines* (I): road shrines give culture, and passing units heal.
- *Tidecall of Danalin* (II): turn a flat land plot into Coast (water) for good.
- *Heartwood of Sucellus* [not evil] (II): forests wake as Treants against invaders.
- *Judgment of Junil* [good] (II): name a civ; no war between you for 20 turns, enforced.
- *Ledger of Mammon* [evil] (II): 3% interest on gold; feeds the counter while you are rich.
- *False Colours* [evil] (II): attack a civ at peace without a declaration; its defenders get no
  defensive strikes or fortification.
- *Ossuary of Arawn* (II): souls of your dead buy Bone Golems or bring a unit back.
- *The Pilgrim Road* [good or neutral] (II): foreign cities of your religion send pilgrims
  (population or gold).
- *Penance* [good or neutral] (II, process): production lowers the Armageddon counter.
- *Prophecy of Ruin* (II): name a rival city. Take it within 30 turns for the reward, or lose
  holy-city commerce.
- *Martyrs' Calendar* (II): each disciple killed Blesses its home city; 10 martyrs make a Saint.
- *Holy Days* (II): every 20 turns, a festival in every city of your religion. Enemies at war
  with you then deal -20% damage.
- *Oath of the Blood-Price* [evil or neutral] (II): declare Vengeance on any civ that ever took a
  city of yours.
- *Brand of Aeron* [evil] (III): +50% attack, pursuit, no withdrawal; it decays when idle.
- *Excommunication* (III): a rival of your faith gets Unrest, and its disciples are silent for 30
  turns.
- *Dagda's Balance* [neutral] (III): the counter drifts toward 50; tilt it 3 points every 20
  turns.
- *Holy See* (III): a holy city that civs of its religion cannot attack; its religion's units at
  half cost.

**Engineer (24)**
- *Cairn Golem* (I): a construct that becomes a permanent fortified hill.
- *Causeway* (I): coast becomes land.
- *Corvée* (I, civic): draft citizens as Workers; faster improvements; Unrest per draft.
- *Ferries* (I): land units cross one-plot straits.
- *Watchtower* (I): sight 3 and a defensive strike.
- *Walls that Remember* (I): Sentinels +20% HP per siege survived.
- *Snares* (I): hidden traps on your plots.
- *Tunnel Heads* (II): cities with Tunnel Heads are one move apart.
- *Clockwork Steward* (II): a governor that builds from a list you set.
- *Tierwright's Forge* (II): reroll a building tier every 30 turns.
- *Hollow Mountain* (II): a workable peak; a hidden garrison that strikes first in the first
  battle of each war.
- *Burning Mirror* (II): siege that doubles against Sentinels in the open and burns buildings.
- *Canal Locks* (II): ships through land.
- *Free City Charter* (II): release a city that nobody may attack for 50 turns; it pays you half
  its commerce.
- *Mountaineering* (II): units enter peaks, +50% defence there.
- *Forced March* (II): armies cross a neutral's land for 5 turns, in supply.
- *Floodgates* (II): flood a river valley into impassable marsh for 10 turns.
- *Time-Locked Vault* (II): gold returns doubled in 30 turns.
- *Scorched Earth* (II): your plots give no yield or healing for 15 turns.
- *Weather Engine* (III): storm, fog or blizzard over an area.
- *Bastion* (III): a walking fortress; its stack fights with two Sentinels.
- *Wheeled City* (III): pack a city up and refound it.
- *Automatons* (III): never out of supply; immune to fear, charms and hexes.
- *Golem Labor* (III, civic): golems work plots as citizens.

**Merchant (18)**
- *Black Market* (I): buy a resource you lack.
- *Ledger of Nations* (I): see rivals' gold, deals, power, cards and great-people progress.
- *Insurance of the Nine* (I): 40% of a lost unit's cost back.
- *Embassies* (I): see a civ's Unrest, production and cards, without its consent.
- *Letters of Credit* (I): borrow from the Guild; a default cuts your production.
- *Mercenary Hall* (II): hire from a board of any civ's units.
- *Bounty* (II): your units earn gold and XP for each kill of a named civ's units.
- *Caravanserai* (II): armies stay in supply along roads into open-border lands.
- *Cornering the Market* (II): rivals' copies of a resource stop working.
- *Satrapy* (II, civic): only the capital ruled directly; stronger governors; half the Unrest.
- *Spymasters* (II): Incite Unrest, Poison the Wells, Steal Plans, Turn a Governor.
- *Informers* (II, specialist): espionage; see foreign Unrest within 6 plots.
- *Blood Money* (II): pay to end a Vengeance against you.
- *Paymaster* (III): buy an adjacent enemy unit.
- *Merchant Princes* (III): governors that turn surplus into gold and pay requisitions in gold.
- *Company Towns* (III): an overseas colony whose governor settles the landmass for you.
- *Confederation* (III, civic): restless cities go autonomous instead of revolting.
- *The Black Book* (III): see every civ's history counters.

**Bard (22)**
- *Saga of the Named* (I): an army's victories make a named hero and culture.
- *War Drums* (I): +1 speed and fear immunity in round 1.
- *Siren Rock* (I): pulls and damages enemy ships near the coast.
- *Satire of Kings* (I): +2 Unrest a turn in a rival's cities near yours, for 20 turns.
- *Plunder* (I, civic): triple pillage gold; a share of a captured city's owner's treasury.
- *Muster* (I, process): production becomes XP.
- *Jesters* (I, specialist): -3 Unrest a turn.
- *The Living Chronicle* (I): each first in your history is a verse of culture.
- *The Feast of Ceasefire* (I): a scheduled 5 turns in which no one declares war on you, nor
  you on them.
- *Masquerade* (II): an invisible stack; its first battle from hiding denies defensive strikes.
- *Lullaby of Ceridwen* (II): the enemy's rear row skips its first action.
- *The Unfinished Opera* (II): three Bards, three acts of culture.
- *Anthem* (II): a cultural channel that falls off with distance, not distance squared.
- *Living Legend* (II): governors grow traits; call one to arms as a hero.
- *Regency Council* (II, civic): six directly ruled cities; weaker governors.
- *Carnival* (II, civic): the Culture process calms Unrest; longer golden ages.
- *Patronage* (II, hurry): spend culture to finish production.
- *Songs of Grievance* (II): your cultural channel counts double against cities you have a claim
  on.
- *Hall of Heroes* (II): a new unit takes a fallen hero's name and a promotion.
- *Ballad of the Traitor* (III): invaders in your borders may change sides.
- *Chorus of the Fallen* (III): your dead raise your side's damage for the rest of the battle.
- *Illusory Host* (III): decoy soldiers in battle.

### 9.2 Armageddon cards (52)

**FFH2's, as cards**
- *Blight* (30): unhealthiness everywhere; 25% death damage to every living unit.
- *Pestilence* (35, recurring): Blight for one civ at a time.
- *The Four Horsemen* (40 → 50 → 60 → 70): Stephanos, Buboes, Yersinia and Ars Moriendi, one
  per stage.
- *Hellfire* (80): empty flat plots become Hellfire, each with a demon.
- *Wrath* (90): Wrath appears; land units may become Enraged.

**Early (10-30)**
- *Omens* (20): each civ is told its own fate 30 turns ahead (famine, a traitor governor, a
  meteor on a named plot, a hero's birth).
- *The Red Moon* (20): barbarian and animal spawns ×3 for 20 turns; units outside borders bleed.
- *Children of the Ash* (20-30): the ruins of razed cities rise as Pyre Zombies.
- *Whispers in the Wells* (20 → 40): river cities lose fresh water; then cities without a
  Temple poison healing units.
- *Census of the Damned* (20-40): culpability published. The most culpable is Marked, and the
  most penitent gets a great person.
- *The Mirror Hour* (30): each civ's three strongest units get barbarian doubles.
- *The Signs* (30, recurring): an angel's world rule every 10 turns, the next two always shown
  (Camulos +25% attack; Junil no war declarations; Esus cheap espionage; Kilmorph +25%
  production; Mulcarn no healing outside cities).

**Middle (30-60)**
- *The Drowning Bells* (30 → 50 → 70): coastal unhappiness; the sea takes unimproved coastal
  flats; the Drowned rise.
- *The Long Winter* (35 → 55 → 75): the snow line moves toward the equator; the coasts freeze;
  the Avatar of Mulcarn walks.
- *Rise of the Barbarian Kings* (30-50): barbarian holdings of 3+ cities become civs.
- *The Kurgan* (30-50): a city-less horde that demands tribute and moves on if paid.
- *The Green Revenge* (30-50): Treants rise near the worst forest-cutters.
- *Plague Ships* (40): plague spreads along trade routes; Quarantine stops it at the price of
  trade.
- *The Ashen Sky* (40-60, lingering): -1 food, sight 1, and shots and stones halved in battles.
- *Demonic Embassies* (40-60): every civ may take an Infernal Pact (+25% production, +1
  happiness) that feeds the counter.
- *The Last Market* (40-60): for 10 turns gold buys units, techs, and cities in disorder.
- *The Free Cities* (40-60): every city at Unrest 70+ revolts; neighbours form a League.
- *The Unforgiven* (40): every civ that ever lost a city gets Vengeance on its captor at once.
- *Witching Nights* (40, recurring): every 10th turn summons last twice as long, undead hit
  harder, and divine spells fail.
- *Rift Storms* (50+, recurring): rifts spill creatures; adepts close them (-2) or harness them
  (+1).
- *Starfall* (50): meteors destroy and leave craters with new mana or Mithril.
- *The Fissure* (50): a ridge of peaks with one pass splits the largest continent.
- *Bloodmoon Tithe* (50, recurring): a unit per 8 cities, or 2 population.
- *The Reckoning of Debts* (50): each broken treaty and razed city costs population or a unit
  in 10 turns.
- *Mark of the Beast* (50): the most culpable civ's kills raise the counter, and its dead lower
  it.
- *Harvest of Souls* (55, lingering): every 20 deaths in the world add 1 to the counter.
- *The Hollow Crown* (60): the score leader's most restless governed city secedes.
- *The Silence of the Gods* (60, lingering): no religion spreads, divine spells fail half the
  time, and religious buildings give no culture.
- *The Hollow Years* (60): no great person born for 20 turns, then double points.
- *The Dead Remember* (60): the haunted dead rise on old battlefields.

**Late (60-90)**
- *The Long Nightmare* (60-80, lingering): one trait per leader replaced at random for 20 turns.
- *Dragon Wake* (60 → 80 → 95): dragons wake; they raid the richest cities; a slain dragon
  leaves a hoard and a secret-tech draw.
- *The Shepherd's Mercy* (60-80): Angels for every civ that lowered the counter; the Repentant
  Angel for the most penitent.
- *The Returning* (60-80): destroyed civs return where their culture survives.
- *Schism* (60-80): cities of a religion opposed to the state religion secede as a theocracy.
- *The Forgetting* (65): history, epithets, claims and vendettas are wiped.
- *Ghost Garrisons* (70): cities get Ghost defenders for every defender who died there.
- *Bone Roads* (70 → 90): bone roads link evil and good capitals; the dead march them.
- *Ash of Empires* (70): the ruins of razed cities become Hellfire with demons.
- *The Countdown* (70): the counter freezes; Apocalypse is scheduled in 50 turns, cancelled below
  50 or by one civ's 10 points of Penance.
- *Exodus* (80): cities in disorder or under siege lose population to a new civ, the
  Dispossessed.
- *The Sundering* (80): the largest empire's far governed cities secede, in civil war or peace.
- *The Black Tide* (80-90, lingering): ships at sea bleed; Krakens and Leviathans spawn.

**Endings (100, one drawn, shown at 95)**
- *Apocalypse*: half of every city's population; units may die.
- *Agares Unchained*: the Host of Agares invades the most culpable civs; destroying it resets the
  counter to 50.
- *The Judgment*: the guilty lose population and the penitent gain it, by culpability.
- *The Unmaking*: all culture is reset to the cities' own plots, and every deal ends.

### 9.3 Flagged as depending on an AI (not in the counts above)

- *Toll Roads* and *The Pretender's Song* (Merchant, Bard): the mechanical part is the payoff,
  but the card leans on rivals' moves.
- *The Free Company* (Merchant): only as good as the crusade Host's AI.
- *The Unbinding of Oaths* and *The Pale Election* (Armageddon): mechanical for you, but much of
  their weight is what rivals do with the opening.

### 9.4 The owner's verdicts so far (2026-11-07)

**Sage**
- **Liked:** the Second Sky, Familiars, Homunculus Vats, Mana Grafting, the Orrery of Ceridwen,
  Stoicism, Contingency (a promotion mages take), the Plagiarist's Lens, Mirror Colony.
- **Disliked:**
  - the Long Experiment: not specific enough to implement;
  - the Index of Unwritten Books: self-referential;
  - Scrying Pool: low impact;
  - the Calculus of Chance: low impact, too subtle, little effect on decisions;
  - Waystones: fiddly to manage;
  - Dagda's Seal;
  - the Predictive Engine: fiddly;
  - Temporal Anchor: too weird;
  - Chronicle of Recall: too opaque to opponents for its impact.

**Prophet**
- **Liked:** Maponos's Cradle, the Lantern of Lugus, Agares's Well, Vigil, Forest Paths (inside
  your own borders only), Wayside Shrines (a worker-built improvement on a road), Tidecall of
  Danalin, False Colours.
- **Disliked:**
  - Breath of Tali: the Windcaller was undefined;
  - Relic Cult: fiddly, dicey access to dead heroes' relics, low impact;
  - Ancestor Worship: too vague to implement;
  - Heartwood of Sucellus: too much like the Ljosalfar's March of the Trees, which already
    turns every forest in their borders into a Treant for 5 turns;
  - Ledger of Mammon: interest mechanics reward doing nothing;
  - Ossuary of Arawn, the Pilgrim Road, Prophecy of Ruin: too vague to implement;
  - Penance: too non-interactive for a world effect;
  - Martyrs' Calendar: vague, many fiddly counters for a small effect, the Saint undefined.

**Second round** (2026-11-07):
- **Agreed with section 10.1's predictions, except** Cornering the Market: out. Mildly harming
  other civs' economies, or stopping a unit for a short time, is too subtle.
- **Causeway:** keep, simplified. A special improvement on a coast plot, with its own defence,
  that units move over. No terraforming.
- **Siren Rock:** out, too fiddly. In general, a payoff of 10% of anything is too fiddly.
- **Governors:** parked. No card uses governors as a mechanical effect until the governor
  mechanic is reworked (it is unsatisfying as it stands).
- **Unrest:** parked. No card pushes on Unrest while the mechanic is being iterated.
- **Espionage:** out. A fiddly mechanic, likely to be removed later.
- **Ignored for now:** corporations, mutations, slaves, and the unusual building keys
  (airlift, culture doubling over time, promotion on entering a city, and the like). This takes
  out the Mutagen Font (section 15).
- **Ships are fine** (the owner reversed an earlier "ignore"). Leviathan Harpoons, Canal Locks
  and the Black Tide stay. The Fireship stays out: its effect was empty (see section 15).
- **Short lists are enumerated.** A card that offers "one of a short list" names every item on
  the list.
- **Supply:** not stable, not being built now. Cards that use it are out (the Caravanserai,
  Scorched Earth, Forced March, the supply part of Automatons).
- **Engagement combat** (the Ogre Battle formation battles) is stable ground: the specific
  numbers may change, the concept will not.

Grammar of True Names, the Recursive Lemma, Judgment of Junil, the Brand of Aeron,
Excommunication, Dagda's Balance, the Holy See, Holy Days, the Oath of the Blood-Price, and the
Engineer, Merchant, Bard and Armageddon cards beyond section 10.1: not yet reviewed.

---

## 10. What makes a card good (from the owner's verdicts)

1. **A card is a spec, not a pitch.** It names its vehicle (unit, promotion, building,
   improvement, civic, process, spell), its numbers, its targets and its scope (Forest Paths:
   inside your borders). Every noun is defined: a Windcaller or a Saint that is only a name is
   a reject. Test: could it be implemented from the card text alone?
2. **The impact is worth a great person.** A card costs a great person's other uses, so its
   effect must be one the player notices and plans around. Rejects: a little information
   (Scrying Pool), a few points of crit, a small bonus behind conditions (Relic Cult).
3. **One decision with lasting consequences, or a passive rule; no routine upkeep.**
   - Liked: one-shots (the Second Sky, the Lens, Mirror Colony); one choice that lasts
     (Agares's Well, Mana Grafting); passive rules (Stoicism, Forest Paths); set-once triggers
     (Contingency).
   - Rejected: actions to repeat and route (Waystones), schedules to juggle (the Predictive
     Engine), counters to tend for a small payoff (Martyrs, Relic Cult).
   - An occasional high-stakes action is fine (Tidecall every 10 turns). A frequent low-stakes
     one is not.
4. **Act on the world, not on the game's own machinery.** Cards change the map, units, cities,
   mana and civics. Rejected: manipulating the deck (the Index), the schedule (the Predictive
   Engine), or the world's rules by fiat (Dagda's Seal). Information about the machinery is
   fine (the Orrery). Acquiring another civ's tech through it is fine too (the Lens).
5. **Physical, not metaphysical.** Strange effects on the map are welcome: land into sea, sneak
   attacks, cities through a portal. Effects that bend time or undo the past are not (Temporal
   Anchor, Chronicle of Recall).
6. **Big effects are visible to opponents.** A large swing the opponent cannot see coming or
   plan for is too opaque (Chronicle of Recall). Hidden effects are fine when they are bounded
   (False Colours: one battle).
7. **Reward acting, not waiting.** Interest, hoarding and passive conversion into a world
   effect are out (the Ledger of Mammon, Penance). A world-scale effect must come through play
   on the map that others can contest, not through a solitary sink. A local tool for a local
   problem is fine (Vigil).
8. **Add new verbs; do not hand out a civilization's signature.** A card that gives anyone a
   civ's identity (Heartwood against the Ljosalfar's March of the Trees) is out.
9. **Self-contained** (section 7): the payoff does not rest on how rivals respond.

10. **Effects are qualitative or large.** A card says can or cannot, immune, doubled, a unit,
    a plot, a city. Small percentages are fiddly: a payoff of 10% of anything is out. As a
    rule of thumb, a modifier is +25% or more, or it is not the card's point.
11. **The payoff is felt by the player who holds the card.** A mild, invisible harm to other
    civs' economies is too subtle, even when it is mechanical (Cornering the Market).
12. **Build only on stable ground.** Cards hook into the core: the map, units, battles,
    cities, buildings, techs, mana, religion, civics. Not into mechanics still being iterated
    (governors, Unrest) or likely to be removed (espionage). A card can be revisited when its
    mechanic settles.
13. **The simplest mechanism that delivers the effect.** An improvement units walk over beats
    turning the sea into land (Causeway). A new unit, building, improvement or promotion beats
    a new system.

14. **No push-button cooldowns.** Something repeatable is a ritual (a project in a city: it
    costs production, it is visible, it can be raided), or it happens once, as a world spell.
    Watch the duty cycle: an effect that is up half the time is a permanent state.
15. **Record history in the game's own state, not in side counters.** The game already keeps a
    city's original owner, previous owner, the civs that ever owned it, the turns it was
    founded and acquired, and City Ruins on the map. Cards may read these. A new tally (deaths
    per plot, verses, martyrs, culpability) is suspect. If history must be kept, keep it where
    the player sees it: as map state (a feature or improvement), not a number on a screen.
16. **Permanent beats temporary for medium effects.** A temporary change must be large to
    matter (the Long Nightmare is too subtle unless permanent). An effect the game's own systems
    quickly undo (culture regrowing, deals struck again) is low impact.
17. **No diplomatic legwork.** Offers that may be refused, about things that may not exist,
    are too many moving parts for the gain.
18. **Military stays military.** No converting the army back into the economy.
19. **A generic card should serve most civs that draw it.** A card only one civilization can
    really use (Golem Labor) is out.
20. **No new trackers for modest effects.** A new piece of state is worth adding only for an
    effect that matters a great deal.
21. **The content vehicles are presumptively in.** These already fit the player's ways of
    deciding: a promotion units can take, a unit players can build, a building or improvement
    players can build, a spell that can be cast.
22. **Changing the diplomatic landscape is welcome.** New civilizations, spawned vassals,
    forever-enemies.
23. **One-off global effects are welcome:** world spells a player casts once. They must be worth
    the great person.

28. **City defense is a weak family.** City-defense effects amount to similar things (the Levy
    Hall verdict). Keep very few, or none.
29. **A card must open an interesting play pattern,** not only boost the economy, research or XP
    (Market Towns, Grand Exhibition, Bard's Retinue).
30. **Effects that encourage aggression are good.** Demon and undead slaying that invites a war
    on demons and undead is strategically relevant (Sanctified Steel). Effects that encourage
    defensive play are not.
31. **"More barbarians and animals" is mostly uninteresting,** in the Armageddon deck as
    elsewhere. It encourages defensive play (Leviathans Rise, Unbound Elementals, the Gargoyles,
    Orthus's Host).
32. **Situational defence needs a bundle:** a thematic economic or military benefit that comes
    with it (Elemental Attunement).
33. **Optionality first, numbers later.** A card that adds options is fine even if its numbers
    need balancing later (Wardance).

### 10.1 What the principles predict for the unreviewed cards

To check the principles against the owner's next verdicts:
- **Likely out:**
  - the Time-Locked Vault and Letters of Credit (interest, principle 7);
  - Snares and Forced March (small or fiddly, 2 and 3);
  - the Living Chronicle and Songs of Grievance (counters for small effects, or subtle; 2, 3);
  - Insurance, Informers, the Black Book and Embassies (low impact, 2);
  - the Feast of Ceasefire (a world rule by fiat, 4);
  - Anthem (subtle, 2);
  - the Signs and Witching Nights (world rules by fiat; but they are crises, where principle 4
    may not apply);
  - the Reckoning of Debts (counters, but a large effect);
  - the Mercenary Hall (a board to watch, 3).
- **Likely in:**
  - Cairn Golem, Causeway, Canal Locks, Mountaineering, Ferries, Bastion, Automatons, Wheeled
    City (physical, 5);
  - the Weather Engine (occasional, high stakes);
  - Cornering the Market, the Paymaster, Company Towns;
  - Satrapy and Confederation (civics);
  - War Drums, the Lullaby, the Illusory Host, the Chorus (battle rules);
  - Muster (a local tool).

### 10.2 What the second round changes in the pool

**Causeway, reworked.** A worker build on a Coast plot next to land. Land units
move onto and over it as over land, and units on it get +50% defence. Open point: whether ships
still pass (a bridge) or not (a dam).

**Out:** Cornering the Market; Siren Rock.

**Parked with governors:** Satrapy, the Regency Council, the Clockwork Steward, Merchant
Princes, Company Towns, Living Legend. Mirror Colony stays, as two cities of yours with no word
about governors.

**Parked with Unrest:**
- Vigil, Jesters, Satire of Kings, Confederation, Excommunication, the Pretender's Song.
- The Armageddon cards that run on Unrest or secession: the Hollow Crown, the Free Cities, the
  Sundering.
- Cards that keep their core with the Unrest part removed:
  - Stoicism: war weariness causes no unhappiness;
  - Carnival: longer golden ages;
  - Corvée: draft a citizen as a Worker, at the cost of the population.

**Out with espionage:** Spymasters, Informers; Embassies and the Ledger of Nations as
information cards.

**Fail the 10% rule; out or need a bigger effect:**
- Wayside Shrines (liked; units passing heal 10%). It needs a bigger effect, e.g. units on its
  plot heal as in a city.
- Snares (15% damage).
- Walls that Remember (+20% wall HP per siege).
- Insurance (40% refund).
- Bounty (gold and XP per kill).
- The Ballad of the Traitor (5% a round).
- The Chorus of the Fallen (+5% damage per death).
- Holy Days (-20% damage).
- The small parts of liked cards: the Lantern's Glare (-15 accuracy) goes, and its point stays
  (a holy shooter that sees invisible units).

### 10.2a Third round of verdicts (2026-11-07)

- **Push-button cooldowns are out.** A "once every N turns" ability on a button becomes either
  a ritual that can repeat (a project built in a city, with its cost and its visibility) or a
  one-off world spell. Otherwise, at steady state, someone may spend half the game unable to
  attack you.
- **Golem Labor:** out. Military should not convert back into labor, and only the Luchuirp
  really have golems.
- **Saga of the Named:** out. Counters that exist outside the game's own systems are suspect
  in general.
- **Blood Money:** out. Too much diplomatic legwork: an offer to end something that may not
  exist, which may be refused. Too many moving parts for the gain.
- **Favoured Partner:** out (confirmed), for the same reason.
- **The Long Nightmare:** too subtle unless the trait changes are permanent.
- **The Unmaking:** out. Low impact: culture regrows a city's borders at once, and every deal
  is simply struck again.
- **The Judgment:** out, boring.
- **The Signs, Witching Nights, Pestilence:** out, as predicted. Recurring world effects are
  rejected in the Armageddon deck too: principle 3 covers crises, not only the cards a player
  holds.

### 10.3 Predicted verdicts on everything not yet reviewed

Confidence: H high, M medium, L low. Numbers are the principles of section 10.

**Great-person cards**

| Card | Prediction | Why |
|---|---|---|
| Grammar of True Names | out (M) | ignoring evasion is about +10% to hit: subtle (10) |
| The Recursive Lemma | in (H) | twincast doubles a sphere's battle spells: qualitative, combat (10, 12) |
| Judgment of Junil | in (M) | a 20-turn enforced non-aggression with one civ: blunt, targeted; risk: read as a rule by decree (4) |
| Brand of Aeron | in, simplified (M) | +50% attack, pursuit, no withdrawal is the point; the 10%-a-turn decay goes ("the brand ends after 5 turns without a fight") (10) |
| Dagda's Balance | out (H) | steers the world machinery, with a cooldown (3, 4) |
| Holy See | in (L) | half-cost religious units is large; "cannot be attacked by co-religionists" may read as decree (4) |
| Oath of the Blood-Price | in (H) | Vendetta on demand, and Vendetta is liked |
| Holy Days | out (H) | recurring festivals, a 20% modifier (3, 10) |
| Watchtower | out (M) | sight and a defensive strike: low impact (2) |
| Tunnel Heads | in, without Collapse (M) | a passive movement network is big; Collapse is upkeep (3) |
| Tierwright's Forge | out (M) | a recurring small gamble (3, 10) |
| Hollow Mountain | out (M) | situational, small (2) |
| Burning Mirror | in, simplified (M) | siege doubled against walls in the open is the point; the 10% building burn goes (10) |
| Free City Charter | out (M) | the payoff is commerce over time, and the protection is by decree (2, 4) |
| Floodgates | in (H) | occasional, high-stakes, physical (3, 5) |
| Golem Labor | in (L) | qualitative, but mostly the Luchuirp's golems benefit (8) |
| Corvée, without Unrest | in (L) | drafting a citizen as a Worker is qualitative; "25% faster improvements" is at the threshold |
| Automatons, without supply | in (M) | immune to fear, charms and hexes: qualitative, combat |
| Black Market | in (M) | buying a missing resource unlocks units: qualitative |
| Blood Money | in (L) | the counter to Vendetta; it lives or dies with Vendetta |
| Toll Roads | out (H) | the war effect is small and the gold is the AI's movement (7, 11) |
| The Free Company | out (M) | the war's value is the Host AI's play (9) |
| Saga of the Named | in (L) | a counter, but the payoff is a hero; risk: the Skald is a unit to manage (3) |
| Plunder | in (M) | triple pillage gold and a treasury share on capture: large, rewards acting (7) |
| Masquerade | in (H) | bounded hidden effect, like False Colours (6) |
| The Unfinished Opera | out (H) | +5% culture an act (10) |
| Carnival, without Unrest | out (M) | longer golden ages: situational (2) |
| Patronage | in (M) | a new verb: culture spent as hammers (13) |
| Hall of Heroes | out (H) | a name and one promotion (2) |
| The Chorus of the Fallen | out unless qualitative (M) | +5% per death (10); a qualitative version: "when half your side has fallen, the rest are immune to fear and deal double damage" |

**Armageddon cards**

| Card | Prediction | Why |
|---|---|---|
| Blight, the Four Horsemen, Hellfire, Wrath, Apocalypse | in (H) | FFH2's own; physical, qualitative |
| Pestilence | out (M) | a recurring random hit on one civ; 25% damage (3) |
| Omens | in, without the governor omen (M) | personal forecasts, the roguelike principle itself; governors parked (12) |
| The Red Moon | in, without the bleed (M) | triple spawns is the point; 5% a turn outside borders goes (10) |
| Children of the Ash | in (H) | simple, physical, the past returning |
| Whispers in the Wells | out (M) | small health and 10% poison (10) |
| Census of the Damned | out (M) | a culpability ledger and +10% attack (10) |
| The Mirror Hour | in (H) | dramatic, qualitative: your best units against yourself |
| The Signs | out (M) | a world rule that changes every 10 turns: tracking (3) |
| Witching Nights | out (M) | specific turns to track (3) |
| The Drowning Bells | in, without stage 1 (H) | the sea taking the coast is physical and large; -1 coastal happiness goes (10) |
| The Long Winter | in (H) | physical, large, a chain with a clear answer |
| Rise of the Barbarian Kings | in (M) | a civ birth with no Unrest |
| The Kurgan | in (L) | rests on tribute rules; the horde's behaviour is scripted, not diplomacy |
| The Green Revenge | out (M) | a forest-cut counter for a medium effect (3) |
| Plague Ships | out (M) | a spreading infection to track (3) |
| The Ashen Sky | in, simplified (M) | shots and stones halved is large; -1 food and sight 1 are the fiddly part |
| Demonic Embassies | in (M) | a big, clear choice: +25% production for doom |
| The Last Market | in (L) | a 10-turn window where gold buys anything: large; "cities in disorder" may not survive |
| The Unforgiven | in (H) | Vendetta at world scale |
| Rift Storms | out (M) | recurring, with ±1 and +1-turn payoffs (3, 10) |
| Starfall | in (H) | physical, reshuffles mana and Mithril |
| The Fissure | in (H) | the map changes shape |
| Bloodmoon Tithe | out (H) | a recurring tax; routine upkeep (3, 7) |
| The Reckoning of Debts | out (L) | rests on history counters (broken treaties) for a moderate effect |
| Mark of the Beast, Harvest of Souls | out (H) | ±1 counter per kill or per 20 deaths (3, 10) |
| The Silence of the Gods | in (H) | qualitative: no spread, divine spells fail, no religious culture |
| The Hollow Years | in (M) | simple and qualitative: no great people, then double |
| The Dead Remember | in (M) | qualitative; needs a death count per plot |
| The Long Nightmare | in (L) | big and announced; random trait swaps may feel arbitrary |
| Dragon Wake | in (H) | FFH's dragons; physical; the hoard draw ties the decks |
| The Shepherd's Mercy | rework (M) | angels are good; "who lowered the counter" needs culpability; reward a simpler test |
| The Returning | in (M) | a civ birth from surviving culture, no Unrest |
| Schism | in (M) | a secession by religion, no Unrest |
| The Forgetting | out (H) | acts on the game's machinery (4) |
| Ghost Garrisons | in (H) | qualitative, history made physical |
| Bone Roads | in (H) | physical |
| Ash of Empires | in (H) | simple, physical, a debt for razing |
| The Countdown | rework (M) | a fixed doom date is dramatic; cancelling through Penance is disliked; cancel through the map (a Doom Herald to kill) |
| Exodus | in (L) | disorder is core; a new civ from refugees |
| The Black Tide | in, simplified (M) | "no ship survives a turn at sea outside a port" beats 20% a turn (10) |
| Agares Unchained | in (H) | a boss invasion |
| The Judgment | out (M) | population by share of culpability: arithmetic (10) |
| The Unmaking | in (L) | huge and physical; risk: too weird (5) |
| The Unbinding of Oaths | in (L) | a blunt diplomatic shock; risk: it leans on rivals (9) |
| The Pale Election | out (M) | the crusade call rests on AI pledges (9) |

**Systems of section 6 and after**

- **In:**
  - Scars of war (Haunted and Hallowed plots, M);
  - Salted earth (H);
  - curses with a fuse, once each is specified (M);
  - earned unlocks, once each is specified (M).
- **Out:**
  - epithets (attitude and small modifiers, M);
  - contracts that come due (deals, interest, H);
  - claims (war-weariness numbers, M);
  - old growth (slow, low impact, L).
- **Parked:**
  - governor ambitions, and succession (section 8: the heirs are governors);
  - Unrest by civilization (section 5);
  - the rebels' identities (section 5.3).

  The Ljosalfar-Svartalfar split and Somehow Also Perpentach could come back through Schism or
  the Returning, which do not need Unrest.

### 10.4 What the third round changes

- **Rituals instead of cooldowns:**
  - *Tidecall of Danalin:* the Drown becomes a ritual in a coastal city: when completed, a chosen
    flat land plot within 3 plots of the city becomes Coast (water) for good.
  - *Weather Engine*, *Floodgates*, *Oath of the Blood-Price:* each use is a ritual.
  - *Mana Grafting:* a node can be grafted once, for good; no cooldown.
- ***Judgment of Junil:*** a one-off world spell (20 turns of enforced peace with one civ, once
  per game). A ritual with a long cooldown is the alternative, but even then the duty cycle
  stays high.
- **History in the game's own state:**
  - Vendetta reads the city's previous owner and the turn it was acquired.
  - The Unforgiven reads `original_owner` and `ever_owned`.
  - Children of the Ash and Ash of Empires read City Ruins.
  - All of these are already kept.
- **At risk, because they need new tallies:** Ghost Garrisons (defenders killed per city), the
  Dead Remember and Scars of war (deaths per plot). They survive only if the record lives on
  the map: a big battle leaves a *Battlefield* feature on its plot, visible to all, and the
  cards read Battlefields.
- **The Long Nightmare, permanent version:** each leader permanently loses one non-permanent
  trait and gains one drawn at random.
- **Out:** Golem Labor, Saga of the Named, Blood Money, Favoured Partner, the Unmaking, the
  Judgment.
- **Ghost Garrisons, the Dead Remember, Scars of war:** out (fourth round). New trackers whose
  only job is to power one-off effects of modest importance are out; that is also why crime
  was disliked.

---

## 11. A batch in the favoured shapes (fourth round, draft)

Every card here is a promotion, a unit, a building, an improvement, a spell, a ritual or a
one-off world spell. A few change the diplomatic landscape. All of them:
- use no push-button cooldowns and no new trackers;
- stay off governors, Unrest, espionage and supply;
- have qualitative or large effects.

"National, N" = N at the normal cost, then 500% each. "Hard" = a true limit.

### 11.1 Great-person cards (30)

**Sage**
- **Ceridwen's Fall** (III, needs Arcane Lore; world spell). Choose a plot you can see. Every
  unit within 2 plots of it takes 50% fire damage, and every improvement there is destroyed.
  *Expert:* the answer to the doomstack at your gates, or the opening of the decisive siege.
- **The Great Summoning** (III, needs Arcane Lore; world spell). For each of Air, Earth, Fire,
  Water and Ice mana you have, a permanent elemental of that kind appears in your capital.
  *Expert:* collect the five manas first; it is five tier-3 units for one great person.
- **Wardstone** (II, needs Knowledge of the Ether; improvement). In battles on plots within 2
  of a Wardstone, the attacker's damage spells deal no damage (charms and hexes still work).
  *Expert:* ring your border cities against the caster civs (Amurites, Sheaim, Balseraphs).
- **Spell Turning** (II, needs Alteration; promotion for casters of level 4+). The first enemy
  damage spell aimed at this unit in each battle strikes its caster instead.
  *Expert:* the archmage's insurance in a mage war.
- **Spellbinder** (II, needs Divination; national unit, 2). A caster whose battle hex
  *Silence* stops the enemy caster with the most magic from casting for the whole battle.
  *Expert:* bring it against armies built around one archmage or hero caster.
- **Unweaving** (III, needs Strength of Will; world spell). Every summoned unit, temporary or
  permanent, of every civ at war with you is destroyed, wherever it stands.
  *Expert:* the summoners' doom (the Sheaim's gates, Keelyn's Bond); hold it for the war that
  needs it.

**Prophet**
- **Rite of Return** (III, needs Divine Essence; world spell). A destroyed civilization
  returns as your vassal in one of your cities that it originally founded (the
  city's original owner, which the game keeps), with a garrison of four of its own units.
  *Expert:* turn a conquest you struggle to hold into a vassal that fights with you.
- **Curse of the Nemesis** [evil] (III, needs Malevolent Designs; world spell). Name a civ you
  have met. A new civ (an unused evil civilization, else a demon host) appears on the
  unclaimed land nearest it, or on its border, with one tier-3 unit per city the target has. It
  is permanently at war with the target: no peace, ever.
  *Expert:* give the rival a war it can never end, before you start yours.
- **Crusade of the Faithful** [good or neutral] (II, needs Fanaticism; world spell). Your holy
  city receives one unit of your religion's strongest buildable unit per city in the world
  that follows your religion, up to 12.
  *Expert:* spread the faith first, then call the crusade.
- **Hallowed Ground** [good] (II, needs Honor; world spell). Every hell-terrain plot in your
  borders returns to its natural terrain, and your units deal +25% damage in battles inside
  your borders, for the rest of the game.
  *Expert:* the good civ's permanent home-ground advantage; cast it when hell terrain starts to
  creep in.
- **Exorcist** (II, needs Priesthood; national unit, 3). A disciple that deals double damage to
  units with the Demon or Undead promotion; its battle heal also ends Fear.
  *Expert:* the line against the Infernals, the Sheaim and the Calabim.
- **The Covenant Ark** (III, needs Theology; national unit, 1, hard). A large unit. Every unit
  in its stack is immune to fear and takes half damage from unholy and death damage.
  *Expert:* the centre of the army that marches on the evil civs.

**Engineer**
- **Raise the Ramparts** (II, needs Construction; world spell). Every city of yours gains its
  next wall: none to Palisade, Palisade to Walls, Walls to Wall of Stone.
  *Expert:* the turn before a two-front war.
- **Hill Fort** (I, needs Masonry; improvement on hills). Acts as a city for healing and
  movement (`acts_as_city`), and battles on it are fought with two Sentinels for the defender.
  *Expert:* walls in the field, at the pass the enemy must take.
- **Siege Tower** (II, needs Construction; unit, siege). In an attack on a walled city, the
  attacking side's melee strikes past the Sentinels at the soldiers behind them, and the Siege
  Tower takes the Sentinels' blows.
  *Expert:* takes a walled city without first grinding down its walls.
- **Raise the Mountain** (III, needs Machinery; ritual, repeatable). A hill within 3 plots of
  the city becomes a peak, or a flat plot becomes a hill.
  *Expert:* close a pass for good, or give a city its hills.
- **Forgemasters' Secrets** (II, needs Smelting; two promotions for melee units).
  *Giantsbane*: double damage against large units (`damage_vs_large`). *Wallbreaker*: double
  damage against walls (`damage_vs_walls`).
  *Expert:* the answer to dragons, golems and giants, and to the walled capital.
- **The Iron Road** (III, needs Machinery; a worker build). Unlocks the Railroad route (in the
  data, but no build makes it): your units move along it at no movement cost.
  *Expert:* one army for the whole empire; build it between your fronts.

**Merchant**
- **Client Kingdom** (II, needs Feudalism; world spell). Release one to three connected cities
  of yours (not the capital) as a new civ: an unused civilization that fits the cities, else
  yours under a new name. It is your vassal while you hold more power.
  *Expert:* the far cities you cannot defend become a buffer that fights with you.
- **Sellsword Nation** (III, needs Guilds; world spell). Choose a civ you are at war with. A
  mercenary civ appears on its border with an army (one tier-3 unit per two of its cities). It
  is your vassal and at war with that civ.
  *Expert:* a second front you do not have to march to.
- **Condottiero** (II, needs Trade; national unit, 1). A tier-3 melee unit with the Hero
  promotion (acts once more in battles), bought only with gold, in any of your cities, at any
  time.
  *Expert:* the emergency champion for the city under attack this turn.
- **The Golden Hoard** (III, needs Mercantilism; world spell). For 10 turns, gold spent to hurry
  production buys twice the hammers.
  *Expert:* save gold for a war, then buy the army in one burst.
- **Raider's Mark** (I, needs Horseback Riding; promotion for mounted and recon units).
  Pillaging costs no movement and gives triple gold.
  *Expert:* the raiding economy's tool, best with Plunder.
- **Black Market** (I, needs Trade; building). As in section 3.4: buy a resource you lack for
  gold per turn.

**Bard**
- **The Great Epic** (III, needs Strength of Will; world spell). Every living unit of yours
  gains a free promotion pick.
  *Expert:* cast it with the army at its largest, the turn before the war.
- **Hall of Legends** (II, needs Arete; building, national). Every unit of yours that reaches
  level 5 gains the Hero promotion (acts once more in battles).
  *Expert:* veterans become heroes; fight many battles with the same units.
- **War Chanter** (I, needs Festivals; national unit, 2). Battle charm *Battle Chant*: its side
  is immune to fear and has +1 speed for the whole battle.
  *Expert:* the answer to fear armies, from the first turns.
- **Herald of Defiance** (II, needs Drama; national unit, 1). When the city it stands in is
  attacked, every defender acts once more in round 1.
  *Expert:* the capital's garrison, or the city the enemy will hit first.
- **The Kingmaker's Ballad** (III, needs Rage; world spell). Name two civs, neither
  yours. They become permanently at war with each other: no peace, ever.
  *Expert:* lock the two strongest rivals into a war that bleeds them both.
- **Illusory Host** (III, needs Strength of Will; national unit, 2). As in section 3.5: decoy
  soldiers in battle.

### 11.2 Armageddon cards that change the diplomatic landscape (5)

- **The Lost Legion** (40). An unused civilization appears on the largest unclaimed region,
  with three cities and an army, at peace with everyone.
  *Expert:* a new neighbour to court, or the last free land is taken; settle before 40.
- **Breaking of the Chains** (50). Every vassal breaks free and is at war with its former
  master for 20 turns.
  *Expert:* masters garrison the border with their vassals before 50.
- **Twin Thrones** (60). Every civ with 12 or more cities splits. The half of its cities
  farthest from its capital becomes a civ of the same civilization, at war with the parent for
  20 turns. No Unrest needed: size is the test.
  *Expert:* stay under 12 cities, or hold the far half with armies before 60.
- **The Nemesis of Kings** (70). A forever-enemy civ appears beside the score leader, with an
  army of one tier-3 unit per two of its cities. It can never make peace with the leader.
  *Expert:* the leader fortifies the border it will appear on; the rest let it bleed.
- **The Exiled Court** (30-50). For each civ whose original capital is held by another civ (the
  game keeps the original owner), a court in exile appears on unclaimed land near it. It is a
  civ of the dispossessed civilization, permanently at war with the captor.
  *Expert:* every conqueror of a capital gets a nemesis; conquerors weigh it before 30.

### 11.3 Fifth round of verdicts (2026-11-07)

- **Spawned civs must have an answer to "why is this civ relevant?"** A one-city civ next to
  its enemies dies at once or stops mattering. Curse of the Nemesis, Sellsword Nation and the
  Nemesis of Kings are too weak as written. The Infernals are the model: they arrive with a
  very strong hero and unique mechanics that let them grow quickly.
- **Hall of Legends:** far too strong. The Hero promotion is huge on its own, and acting twice
  in battle compounds it.
- **The Exiled Court:** cannot be implemented as written.
- **No direct interaction with Vendetta** in deck cards.
- **Process:** no mass drafting yet. Review continues in rounds, so the drafts learn the
  owner's taste first.

24. **A spawned civ needs a reason to matter:** a hero far above its era, mechanics that let
    it grow fast, enough land and cities to stand. Without one, the card is a speed bump.
25. **Respect the power of existing promotions.** Hero, the extra action and similar ones are
    rare for a reason; a card does not hand them out wholesale.
26. **Implementable from existing state.** "Original capital", "where it appears" and "who it
    is" must each have a definite answer in the game's data.

## 12. Review batch 1 (10 cards, 2026-11-07)

Stat scale for reference: Champion 120 HP / 60 attack / 60 defence; Paladin 110 / 110; Hill
Giant (large) 180 HP / 70 / 70; Archmage magic 55; Acheron 190 / 190.

**1. Rite of the Necropolis** [evil or neutral] · Prophet · III · needs Malevolent Designs ·
world spell (once per game)
- **Requires:** at least 3 City Ruins on the map.
- **Effect:** a new civ, the Necropolis, rises on the 3 City Ruins nearest your capital: three
  cities of size 2, each with Walls.
- **Its leader and hero:** the Lich-Prince. A Lich with the Hero promotion, magic 70 (an
  Archmage has 55), Death II's battle spell, and `immortal` (returns to its capital when
  killed).
- **Its mechanics:**
  - It has the Infernals' Fallow trait: its cities need no food.
  - Every unit killed in a battle inside its borders rises there as a Pyre Zombie of the
    Necropolis.
- **Diplomacy:** permanently at peace with you, and permanently at war with every good-aligned
  civ.
- **Why it matters:** a hero above its era, walled cities from its first turn, and an army that
  grows from every war fought against it.
- **Expert:** cast it on the ruins your rivals must march past, never beside your own
  expansion.

**2. Wake the Hill Clans** · Engineer · I · needs Mining · world spell
- **Requires:** an unclaimed region with at least 6 hill or peak plots within 3 plots of each
  other.
- **Effect:** a new civ of giants, the Hill Clans, your vassal. It gets two cities on
  that region's hills, each with a Palisade, and 4 Hill Giants.
- **Its hero:** Thrumm the Mountain-King, large, 200 HP, 100 attack, 100 defence.
- **Its mechanics:**
  - It alone trains Hill Giants (90 hammers).
  - Its cities get +1 production per hill worked.
- **Why it matters:** large units that outclass the Ancient era, a hero, and defensible cities.
- **Expert:** cast it early on the hills between you and your nearest rival: a wall of giants
  that fights for you.

**3. Gryphon Riders** · Bard · II · needs Animal Handling · national unit, 4 · trained in a
city with a Stable
- **Stats:** tier 3, mounted (flanker). HP 100, attack 65, defence 45, magic defence 34,
  accuracy 90, evasion 15, crit 10, speed 7; 3 moves.
- **Flies:** crosses peaks and water at 1 move a plot, ignoring terrain costs. It cannot end a
  turn over water.
- **In battle:** its charge reaches any enemy unit, the rear row included, in every round.
- **Expert:** the killer of casters, archers and siege engines, and the only land unit that
  crosses mountains.

**4. Shieldbearer** · Engineer · II · needs Bronze Working · promotion for melee units of
level 3+
- **Effect:** while the Shieldbearer stands in the front row, shots, stones and damage spells
  aimed at units in its lane hit it instead (as a Sentinel intercepts down its lane). It takes
  half damage from them.
- **Expert:** one per lane makes the back row safe from archers and mages.

**5. Spire of the Archmagi** · Sage · II · needs Sorcery · national building
- **Effect:** casters trained in this city start with one extra sphere I promotion, chosen at
  training among the mana types your empire holds, with no level requirement. Every adept from
  here arrives with a battle spell.
- **Expert:** the mage school; worth most to a mana-rich empire.

**6. Battlemage** · Sage · III · needs Arcane Lore · promotion for Mages and Archmages of
level 5+
- **Effect:** the caster's damage battle spells strike every unit in the target row (the sweep
  pattern), and the caster may stand in the front row.
- **Expert:** one Battlemage against a massed front line.

**7. The Gilded Legion** · Merchant · III · needs Mercantilism · national building
- **Effect:** when built, choose one civilization you have met. This city can train that
  civilization's versions of each unit class (its unique units; not heroes or world units) at
  150% cost, with its own building prerequisites.
- **Expert:** buy the unique line that answers your enemy: golems, Fyrdwell horse archers,
  vampires.

**8. Trading Post** · Merchant · I · needs Trade · improvement, built outside borders at least
3 plots from any city
- **Effect:** its plot and the 8 around it become your territory, fixed (culture does not move
  it). Resources there join your trade network by road. A unique feature there counts as
  inside your borders (its doubled research).
- **Removal:** an enemy that pillages it removes the claim.
- **Expert:** claim a far mana node, unique feature or strategic resource without founding a
  city.

**9. Font of Rebirth** [good or neutral] · Prophet · II · needs Priesthood · building
- **Effect:** the first time a unit of yours would die in a battle within 3 plots of the city,
  it is carried to the city at 10% HP instead. It gains the promotion *Reborn*, which marks the
  Font as used for that unit: a Reborn unit dies normally.
- **Expert:** the fortress city's garrison survives its first lost battle; fight near the Font.

**10. Charter of the Free Peoples** · Bard · II · needs Code of Laws · world spell
- **Requires:** at least 3 barbarian cities in the world.
- **Effect:** every barbarian city in the world, with the barbarian units within 3 plots of it,
  becomes a new civ, the Free Peoples, your vassal. Orthus, if alive, becomes its
  leader-hero; otherwise a barbarian warlord hero (Champion line, 90 attack).
- **Why it matters:** it starts with every barbarian city at once, often 4 to 10 cities across
  the map, and their armies.
- **Expert:** cast it when the barbarians hold many cities nobody has cleared; your vassal then
  sits on every rival's flank.

**Verdicts on batch 1:** all liked, except the Charter of the Free Peoples (out: too close to the
Clan of Embers' world spell For the Horde, which turns barbarians to the Clan's side).

## 13. Review batch 2 (10 cards)

**1. The Usurper's Crown** · Bard · III · needs Rage · world spell
- **Requires:** a civ you have met with 8 or more cities.
- **Effect:** the third of that civ's cities farthest from its capital (at least 2), with the
  units inside them, secede as a new civ of the same civilization. It is permanently at war
  with its parent and permanently at peace with you.
- **Its leader:** the Usurper, a hero of the civilization's best melee line, with the Hero
  promotion.
- **Why it matters:** it takes a third of a large empire's cities at once, with their
  garrisons and walls.
- **Expert:** cast it on the leader just before your own war on it, so it fights on two fronts.

**2. Oath of Fealty** · Merchant · II · needs Feudalism · world spell
- **Requires:** you are at war with a civ whose power is a third of yours or less.
- **Effect:** that civ becomes your vassal at once: the war ends, its cities and
  units become a vassal's.
- **Expert:** end a won war without the last sieges; the cities you would have taken fight for
  you instead.

**3. Runeguard** · Engineer · II · needs Construction · national unit, 4
- **Stats:** tier 3 frontline. HP 140, attack 55, defence 70.
- **Effect:** immune to spell damage, charms and hexes in battle (`immune_to_magic`).
- **Expert:** the front row against a caster army; useless against plain steel.

**4. Witch-Hunter** · Prophet · II · needs Priesthood · national unit, 3
- **Stats:** tier 3 recon (stabs). HP 90, attack 55.
- **Effect:** double damage against adepts and disciples (`damage_vs` those unit combats). It
  targets casters first (target preference), and is immune to hexes.
- **Expert:** two of them in every army that faces mages or priests.

**5. Ritual Circle** · Sage · II · needs Knowledge of the Ether · improvement, national (one
per civ)
- **Effect:** a unit summoned by a caster standing on the Circle is permanent instead of
  temporary. The Circle holds at most 4 permanent summons at a time; when one dies, it frees
  its place.
- **Expert:** four permanent elementals or demons; best for a summoner and a mana-rich
  empire.

**6. Earthshaper's Decree** · Engineer · III · needs Machinery · world spell
- **Effect:** every unimproved land plot in your borders that can take an improvement gets the
  one that suits it best (farm, mine, lumber mill, pasture, ...), at once.
- **Expert:** cast it after your borders have grown, not before; it is a decade of worker
  turns in one.

**7. Academy of War** · Bard · II · needs Military Strategy · national building
- **Effect:** land units trained in this city start at level 3 (two promotions).
- **Expert:** the army's factory; requisition your armies from this city.

**8. Call the Aifon** · Merchant · II · needs Sailing · world spell (a new civ)
- **Requires:** an unclaimed coast or island with room for two cities.
- **Effect:** the Aifon, the sea-god's people of the Lanun's lore (`IMPROVEMENT_AIFON_ISLE`),
  found two coastal cities there as your vassal.
- **Its hero:** the Deep Mother, a Kraken.
- **Its mechanics:** its land units have Water Walking, so they cross the sea without ships.
- **Why it matters:** an army no navy can stop, and a Kraken.
- **Expert:** cast it on the far coast of the rival you will fight across the water. (Risk: it
  sits close to the Lanun's sea identity.)

**9. Spellbreaker** · Sage · II · needs Alteration · promotion for melee units of level 2+
- **Effect:** each hit it lands removes the target's charms and spell-given promotions for the
  battle. It deals double damage to summoned units.
- **Expert:** the answer to buffed and summoned armies (Keelyn's Bond, the Sheaim's gates).

**10. Fire Arrows** · Engineer · I · needs Archery · promotion for archers
- **Effect:** the archer's shots strike Sentinels as a siege engine's stones do. Archers become
  wall-breakers.
- **Expert:** take early walled cities without siege engines.

## 14. Review batch 3: the religions (10 cards)

Each card needs the named state religion to be drawn and used, unless it says "any". It draws on
that religion's own flavor and content.

**1. Seed of Yggdrasil** · Prophet · Fellowship of Leaves · II · needs Way of the Forests ·
worker build on a forest plot in your borders, one per city
- **Effect:** a Sapling that grows into a Great Tree in 30 turns, as a cottage grows into a
  hamlet (`improvement_upgrade`, `upgrade_time`). The Great Tree gives the city a Fruit of
  Yggdrasil resource (an existing bonus) and +2 food. Units in the city's radius heal as in a
  city.
- **Expert:** plant early, in the cities you will hold longest.

**2. Holy Inquisition** · Prophet · The Order · II · needs Orders from Heaven · spell for the
Order's disciples, in a city of yours
- **Effect:** removes one religion other than the Order from the city, and its temples there
  become Temples of the Order. If the city is that religion's holy city, its shrine is
  destroyed and the religion loses its holy city.
- **Expert:** take a rival faith's holy city, then purge it; its founder loses the shrine
  everywhere.

**3. Altar of the Deep** · Prophet · Octopus Overlords · II · needs Message from the Deep ·
building, coastal cities
- **Effect:** every land unit killed in a battle within 2 plots of the city rises there as a
  Drown (the Overlords' undead) under your control.
- **Expert:** fight on your own coast; every failed invasion there feeds you.

**4. Mithril Forge** · Engineer · Runes of Kilmorph · III · needs Mithril Working · national
building
- **Effect:** melee units trained in this city get the mithril weapon tier, with or without the
  Mithril resource.
- **Expert:** the armoury of an empire without Mithril.

**5. Soul-Bound** · Prophet · The Ashen Veil · II · needs Corruption of Spirit · promotion for
units of level 4+
- **Effect:** when the unit dies, a Balor (the Veil's tier-4 demon) appears on its plot under
  your control for 5 turns.
- **Expert:** veterans are worth more dead in the middle of a siege; take it on the assault
  units.

**6. Radiant Bastion** · Prophet · The Empyrean · II · needs Honor · building
- **Effect:** in battles at this city the attackers are Blinded: they act last in every round,
  whatever their speed. Hidden units within 3 plots of the city are revealed.
- **Expert:** the city that fast armies and shadow armies cannot take.

**7. The Faceless One** · Bard · Council of Esus · II · needs Deception · national unit, 1
(hard)
- **Effect:** invisible (the Council's own invisibility). Its spell *Unmake* kills an adjacent
  enemy hero or world unit, and the Faceless One dies with it. An immortal hero returns as
  usual.
- **Expert:** kill the hero the enemy's war is built on.

**8. Temple Ward** · Engineer · any religion · II · needs Priesthood · building
- **Effect:** in a city with a temple of your state religion, the walls have one more
  Sentinel (three instead of two).
- **Expert:** the walls of the cities that matter, in any faith.

**9. The Second Shrine** · Prophet · any religion, for the civ that holds its holy city · III ·
needs Theology · building
- **Effect:** build your religion's holy shrine in a second city. Both shrines give the
  shrine's effects; if one is lost, the other stands.
- **Expert:** insurance for the holy city, and double the shrine where the faith is wide.

**10. Syncretism** · Sage · any religion · III · needs Divine Essence · civic (Cultural Values)
- **Effect:** choose a second state religion of the same alignment as your first, or a neutral
  one. Both count as your state religion for buildings, units, civics and spells.
- **Expert:** two faiths' heroes and units, at the cost of the Cultural Values slot.

**Sixth round of verdicts:** forever-at-war and forever-at-peace are out in general. Attitude
modifiers are fine.

27. **No permanent war or permanent peace.** A spawned or changed civ's leanings are set
    through attitude modifiers (a strong grudge against its parent, gratitude toward the caster),
    and its AI then decides. The card's payoff is still the board change (the cities and units),
    not the attitude (principle 9).

What changes:
- *Rite of the Necropolis:* +4 attitude toward the caster, -4 toward good civs, in place of
  permanent peace and war.
- *The Usurper's Crown:* +4 toward the caster, -6 toward its parent.
- *The Kingmaker's Ballad, Curse of the Nemesis, the Nemesis of Kings:* out; their effect was
  the permanent war.
- *Judgment of Junil* (20 turns of enforced peace, once) and *Free City Charter* (50 turns
  unattackable) are timed rather than permanent. Flagged: the owner to say whether timed
  enforcement is also out.
- "Vassal" (the Hill Clans, Rite of Return, Client Kingdom, Call the Aifon) is
  vassalage, not war or peace; flagged for the same question.

**Seventh round of verdicts:**
- **Religion cards are offered whatever the drawer's religion.** Besides its effect, each
  spreads its religion to three of your cities: an option for an abrupt religious shift. For
  batch 3, cards 1-7 work this way. The religion's own units (the Order's disciples for the
  Inquisition) are then in reach once the faith is in your cities. Cards 8-10 already work
  with any religion and spread nothing.
- **"Vassal", not "permanent vassal",** everywhere (replaced throughout this file). Ordinary
  vassal rules apply.
- Still open: whether timed enforcement (Judgment of Junil, Free City Charter) is out too.

**Free City Charter, reworked** (the owner: no unattackable city; strong defences that make
attacking it costly instead). Engineer · II · needs Construction · world spell.
- **Effect:** release one of your cities (not the capital) as a Free City: a one-city civ that
  keeps its buildings and garrison, with +4 attitude toward you.
- **Its defences:** it gains a Wall of Stone, three Sentinels instead of two, and +50% defence
  for units fighting in the city.
- **Expert:** turn the border city you cannot hold into a hard, friendly buffer between you and
  the rival.

Judgment of Junil stays: a one-time enforced peace is fine (the owner). What is out is lasting
enforcement: permanent war or peace, and a city made unattackable for decades.

## 15. Concepts from mechanics no card uses yet

What the game already has and the decks have not touched:
- 23 equipment items (Godslayer, Orthus's Axe, Crown of Akharien, Rod of Winds, potions, ...);
- 6 lair types that spawn units (Barrow: skeletons; Bear and Lion Dens; Goblin Fort; Ruins:
  lizardmen; Hellfire);
- 54 goody and lair outcomes, among them prisoners and treasure vaults;
- 20 mutation promotions (Strong, Stoneskin, Regeneration, Blitz, Heavy, ...);
- capture;
- Fear;
- disease;
- ships and the naval duel;
- unique features (the research-doubling rule);
- world units, which are once per game;
- the `no_bad_goodies` unit key. (`explode_in_combat` is only a visual flag in the port: no rule
  reads it. The Fireship, now out, wrongly relied on it.)

**Items**
- **Rite of Forging** (Engineer · II · needs Smelting · ritual, repeatable). Forge one of these
  three new items. Items follow the existing equipment rules (carried, dropped, captured):
  - *Banner of the Last Host*: the bearer's whole side is immune to fear;
  - *Seven-League Boots*: +2 moves;
  - *Ward Amulet*: immune to spell damage.

  *Expert:* the banner goes with the main army; the amulet on the hero the enemy mages
  target.

**Lairs and goodies**
- **Covenant of the Wild** (Prophet · I · needs Animal Husbandry · building, national). Lairs
  inside your borders (Bear and Lion Dens, Barrows, Goblin Forts) spawn their units for you
  instead of for the barbarians.
  *Expert:* do not clear the dens you will grow your borders over; they become free armies.
- **Ruin Delvers** (Merchant · I · needs Exploration · promotion for recon units). The unit can
  explore City Ruins as it would a lair, for the lair outcomes (gold, techs, items, prisoners),
  never the bad ones (`no_bad_goodies`).
  *Expert:* the razed cities of old wars become treasure.

**Mutation**
- **Mutagen Font** (Sage · II · needs Alteration · building). Each unit trained here gains one
  mutation, drawn from the 20 mutation promotions.
  *Expert:* a gamble on every unit, worth it for armies of cheap ones. (Risk: close to the
  Balseraphs' Freaks.)

**Capture, Fear, disease**
- **Beast-Catchers' Nets** (Bard · I · needs Hunting · promotion for recon units). Animals and
  beasts this unit defeats join you (capture).
  *Expert:* an early army of bears and wolves from the wild.
- **Dread Banner** (Bard · III · needs Rage · item from a ritual). The bearer has Fear: before
  each battle, enemy units of lower level than the bearer may sit it out.
  *Expert:* the hero's banner; it empties the enemy's front row before the battle starts.
- **Plague Carrier** [evil] (Prophet · II · needs Corruption of Spirit · national unit, 3).
  Its hits give the Diseased promotion (`promotion_combat_apply`).
  *Expert:* an army whose wounded stay wounded.

**Ships**
- **Fireship** (Engineer · I · needs Sailing · ship). Cheap. When it attacks, it is destroyed
  (`explode_in_combat`); if it wins, the defender sinks.
  *Expert:* the poor navy's answer to the rich one.
- **Leviathan Harpoons** (Merchant · II · needs Optics · promotion for ships). Double damage
  against sea monsters and large ships; a Kraken or Leviathan it kills is captured.
  *Expert:* tame the monsters that close the sea lanes.

**Unique features**
- **The Wonder Unearthed** (Sage · III · needs Arcane Lore · world spell). One new unique
  feature of your choice appears on an unimproved plot in your borders. Each doubles research
  on its tech while inside a civ's borders, by the existing unique-feature rule. None of these
  techs has a feature today.
  - *The Iron Tree* (forest): Iron Working.
  - *The Whispering Stacks* (any land): Guilds.
  - *The Weeping Monolith* (hills): Righteousness.
  - *The Horse Lords' Barrow* (grassland or plains): Warhorses.
  - *Starfall Crater* (any flat land): Precision.
  *Expert:* pick the feature whose tech is on your beeline.

**World units**
- **Hall of Rebirth** (Prophet · III · needs Divine Essence · national building). A hero of
  yours that dies can be trained again here at full cost, once per hero. World units are
  otherwise once per game.
  *Expert:* the hero-centred army stops being one battle from collapse.

**Verdicts on batch 2:** all liked, except the Ritual Circle (out: summons are within-combat
entities only in the current system, so there is nothing to make permanent). The same fact
removes Unweaving (section 11.1), which destroyed summoned units on the map. Spellbreaker's
double damage against summoned units still applies inside battles.


---

## 16. Round 8: 150 candidates and adversarial review

The candidates, the three reviews' findings and the verdicts are in `docs/DECKS_ROUND8.md`
(section C).
- **Great-person cards:** 58 of 100 survive, ranked A (16), B (29) and C (13).
- **Armageddon cards:** 19 of 50 survive.

Its rules for the decks (C.3) apply to every card here as well:
- a card's world spell is its own one-off;
- remote targeting is one shared system;
- a floor of 2 gold a hammer;
- few fortress buildings;
- one Fear card;
- no invisible stacks on the map;
- naval cards are fine (the owner: an AI sprint will give the AI naval combat, later);
- every card checked against the civ world spells and existing content;
- regions defined as connected groups of plots;
- "a level" means the XP for the next level.

Corrections to earlier cards that the review found:
- **Shallows is a land terrain** (the hell form of Marsh), not water. Tidecall, Causeway and the
  Drowning Bells now say Coast (water).
- **Holy Inquisition** (batch 3) overlaps the existing Inquisition spell, which already purges a
  city's non-state religions and their buildings. Its own part is the purge of a holy city and
  its shrine.
- **Hall of Rebirth** (section 15) overlaps the existing Resurrection spell, which brings back
  the civ's dead hero.
- **Beast-Catchers' Nets** (section 15) is the existing Subdue Animal promotion (recon units
  capture the animals they defeat): out.
- **The Great Summoning** (section 11.1) is replaced by Elemental Foundry (round 8, G01).
- **Call the Leviathan and Leviathans Rise:** the Leviathan is one per game.
