# Entities: every unit, building and spell, what it does, how to play it, how the AI uses it

An inventory of the game's entities, assembled in the entities sprint (2026-10-22): every unit (325),
building (170), spell (267) and project (17) in the vendored data, each with what it does in the
rules, what using it well looks like for a player, and how far the AI uses it, judged from the
code (no games were played for it). The entries are in three files:

| File | What |
|---|---|
| [`entities/units.md`](entities/units.md) | units: civilians and constructs, melee, archery, mounted, recon, siege, arcane, disciples, naval, beasts, heroes, great people, summons, equipment, animals, barbarian, event and special units |
| [`entities/buildings.md`](entities/buildings.md) | city buildings, national and world wonders, buildings no city can build (palaces, granted and spell-made), projects and rituals |
| [`entities/spells.md`](entities/spells.md) | sphere spells, world spells, unit, hero, religion and civilization spells, abilities |

The facts are as of commit `dd4bd0f` (master, 2026-10-22). Rules and AI change; when an entry and
the game disagree, the game is right, and the entry is out of date. The live source for what an
entity does is its help text (`ffh-play`, `crates/ffh-play/src/view/help/`, ports of C++
`CvGameTextMgr`), which is built from the same tables; the live source for the AI is
`crates/ffh-ai` (docs/AI.md). "The AI" here is `ffh-ai`, the game's AI. `ffh-cppai`, the port of the
FFH2 0.41 C++ AI kept for comparison (docs/CPPAI.md), is not covered.

How the inventory was made: the help, strategy and effects texts of every entry come from the
`data` query of `ffh-play agent`; the fields from the vendored XML (`data/xml`); the rules' Python
effects from `crates/ffh-core/src/ffh/` (the ports of FFH2's spell and event Python); the AI's use
from reading `crates/ffh-ai`, with every place that names the entity's type string.
`tools/entity_facts.py` writes those facts as one sheet per entity; its AI lines re-implement the
AI's filters (the production roles, the fields the building value reads, the spells `magic.rs`
models, FfH's spell terms), described below so they can be checked against the code.

**Keeping it current.** When the rules or the AI change, regenerate the sheets at the old and the
new commit and diff them: the entities whose sheet changed are the entries to revisit
(`B=$(tools/agents/build-rev.sh <rev>)`, then `tools/entity_facts.py --bin $B/ffh-play --out DIR`).
An AI change that names an entity, or changes a valuation below, should update its entries and the
summary.

## How to read an entry

```
### Axeman (`UNIT_AXEMAN`)
*Melee, tier 2 · 60 hammers · 5/5, 1 move · Bronze Working, a Training Yard · every civilization but ...*

**What it does.** The rules: strength, abilities, what is special, effects that live in Python.

**Playing it well.** What an expert player does with it: when it is worth its cost, how to use
it, what it counters and what counters it, what it combines with.

**AI: Valued.** How the AI gets it (which production step, which valuation) and how it uses it
(which role, which spell choice), and what that misses, with the code that decides.
```

Grades (how fully the AI uses the entity, read from the code; for a unit, getting it and using
it both count, and the weaker of the two decides):

| Grade | Meaning |
|---|---|
| **Directed** | ffh-ai has logic for this entity's own mechanic (it names the type, or a rule written for its kind: shrines, siege engines, rituals, world-spell timing) and uses it on purpose. |
| **Valued** | No logic of its own, but the generic valuation sees what the entity is for: it is built, trained or cast for its main effect. |
| **Partial** | The AI gets or uses it, but its valuation misses a main part of what it does (a building whose effect is a free promotion or lives in Python; a unit trained for its strength whose abilities go unused; a spell cast for a flat weight whatever the situation). |
| **Incidental** | The AI has it only as a by-product: an upgrade, an early fallback build, a flat AI weight with no reason behind it, a unit the rules hand it. Nothing in the AI decides to get it for its effect. |
| **Unused** | A player could use it; the AI never does: the rules refuse AI casts (`bAllowAI` 0), its value is always 0, no production role takes it, it is excluded on purpose, or it needs something the AI never does. |
| **Not an AI choice** | The rules create, control or consume it (equipment dropped on death, event and lair units, barbarian and animal units, units only a ritual or event spawns); the entry says how the AI handles the unit once it has one. |

## How the AI gets and uses entities

Every entry's "AI" paragraph refers to the machinery below. Function names are given as
`file::function` (in `crates/ffh-ai/src/`), which is stable across edits; docs/AI.md describes the
same code by area, with the measurements behind each choice.

### Units: production

`production::decide` is a priority list, run when a city's queue empties (docs/AI.md, "Cities"):
a defender when the city has none or is outmatched; the first worker; a second defender after
turn 30 (capital, cities near barbarians); a victory item when the goal is 30 turns away; an
explorer from the capital while a fifth of the home landmass is unknown; settlers
(`expansion::settler_choice`); the civilization's own items (`civs::production`); an amphibious
assault fleet; a transport for a settler waiting in port; a sea explorer; work boats; a
unit-enabling building; defenders up to the garrison quota; a small mobile force; more workers;
the victory goal's items; **heroes**; hero ships; the siege operation's force
(`campaign::army_choice`) and siege engines; buildings by value (below); the army interleave;
**casters** (`(cities + 2) / 4`, one more for a leader with `iAIValueMage` > 100, only while a unit
is affordable); **missionaries** (one at a time, for the state religion, while a city of ours
lacks it); attackers to the army target; the best building; a project; a process.

Each unit step asks `production::best_unit(city, role)`, which takes only the units the city's
civilization has for their class (`city_offers_unit`) and that the rules let it train
(`city_can_train`), and whose type passes `production::role_ok` for the role:

| Role | A unit qualifies when (`role_ok`) | Scored by (`best_unit`) |
|---|---|---|
| Defense | a *fighter* (strength > 0, does not found cities, not a hero, default AI not `UNITAI_MAGE`) whose `UnitAIs` list holds `CITY_DEFENSE`, `CITY_COUNTER`, `ATTACK` or `COUNTER`, default AI not `EXPLORE` | city defense: defense strength (or strength) + damage types, x city defense %, + first strikes |
| Attack, CityAttack | a fighter, not `bOnlyDefensive`, with `ATTACK` or `ATTACK_CITY` in `UnitAIs`, and not a siege engine (default AI `ATTACK_CITY`/`COLLATERAL` with bombard or collateral) | strength + damage types, + moves, first strikes, withdrawal, collateral (CityAttack: x city attack %, collateral x2) |
| Siege | a fighter with `ATTACK_CITY` or `COLLATERAL` in `UnitAIs` and bombard or collateral damage | attack power + bombard x20 + collateral x3 |
| Worker / SeaWorker | `WORKER` (`WORKER_SEA`) in `UnitAIs`, strength 0 | 100 + work rate, per turn |
| Settler | `SETTLE` in `UnitAIs` and `bFound` | per turn |
| Hero / SeaHero | default AI `UNITAI_HERO` or the Hero promotion for free; strength > 0, or Channeling I for free (Corlindale) | as Attack |
| Mage | default AI `UNITAI_MAGE`, or `WARWIZARD` in `UnitAIs` | 100 + moves x10 + tier x50 |
| Explore / SeaExplore | `EXPLORE` in `UnitAIs` (at sea: strength > 0 and `EXPLORE_SEA` or `ATTACK_SEA`) | moves, tier |
| Transport | a ship with cargo space for land units and no special cargo | cargo x100 + moves x30 + strength x5 |
| Missionary (`best_missionary`) | `MISSIONARY` in `UnitAIs` and it spreads the state religion | the first that qualifies |

The unit's `iAIWeight` is added to the power. Military roles score power squared per turn of
production (`power^2 / (turns + 4)`), so a stronger unit wins unless it is much dearer; free
promotions, spells and abilities do not count in the power (only strength, damage types,
moves, first strikes, withdrawal, collateral, bombard and city bonuses do). Units with
`iMinLevel` above 0 cannot be trained by the AI (the rules: `city_can_train` refuses them unless
`GAMEOPTION_AI_NO_MINIMUM_LEVEL`, and never at min level 1); they come by upgrade. Optional units
are trained only while one more is affordable (`economy::can_afford_unit`). A unit that passes no
role is never trained by the generic steps; only a civilization step that names it
(`civs/*.rs`) or a victory step can train it.

Buildings that gate units (`PrereqBuilding`, `PrereqBuildingClass`) are built for them by three
paths only: `production::enabler_building` (a defender or attacker at least a third better, or a
third better per hammer, than the city's best, in at most half the cities, within 20 turns);
`campaign::siege_building` (the Siege Workshop in the most productive city when the siege
operation wants engines and no city can train one); and `civs::unique_casters` (the Amurites,
Clan of Embers, Doviello and Svartalfar: the Adept's building in the most productive city, then
casters up to `(cities + 2) / 2`). Otherwise a gating building comes only from its own value.

Research toward units: `research::tech_value` counts a better defender or attacker against the
military pressure it sees, the first arcane unit, the first transport; `civs::tech_value` adds
1500 for a tech that completes a unique unit of the civilization (its tech and its building's).

### Units: in the field

`units::play` gives every group one decision a turn by its head unit's role (`units::role_of`,
from the unit's AI type, set by production when it trains the unit, or its default AI):

| Role | Default AI types | What it does |
|---|---|---|
| Settler | `SETTLE`, or `bFound` | `expansion::settler`: to the best reachable site with an escort, overseas by transport |
| Worker | `WORKER`, or anything that can build | `workers::worker`: best (plot, build) by yield and need; flees danger; chops for settlers, workers and world wonders |
| Garrison | `CITY_DEFENSE`, `CITY_COUNTER`, `CITY_SPECIAL` | fills the city that most needs a defender, fortifies |
| Army | any other unit with strength (the `_` branch: `ATTACK`, `ATTACK_CITY`, `COLLATERAL`, `COUNTER`, `RESERVE`, `PILLAGE`, `FEASTING`, ...) | `military::army`: merge into stacks, attack what it can beat, heal, defend cities in peril, escort settlers, the siege operation (`campaign.rs`: gather, march, strike), hunt barbarians and animals |
| Explorer | `EXPLORE` | reveal the land and goody huts one safe turn at a time, then join the army |
| Great person | `PROPHET`, `ARTIST`, `SCIENTIST`, `GENERAL`, `MERCHANT`, `ENGINEER` | a great work for a culture victory; a shrine in a holy city of ours; an Engineer rushes a wonder; else lightbulb (`MISSION_DISCOVER`), join as a specialist, start a golden age |
| Missionary | `MISSIONARY` | spreads its religion in our cities that lack it (never abroad) |
| Caster | `MAGE`, `WARWIZARD`, `TERRAFORMER`, `MANA_UPGRADE`, `MEDIC`; heroes that cannot attack | `magic::caster_target`: with the stack nearest the operation's target, to the units its lasting blessings are for, to the cities its city spells are for, to land to terraform; else the most threatened city; Tower goal: mana nodes (`victory::mana_node`) |
| Animal | `ANIMAL` | attacks what it can beat, else roams (barbarian and animal players) |
| Transport, sea explorer | ships | `naval::transport` (settlers overseas, amphibious assaults), `naval::sea_explore` |
| Naval, other | other ships, air units, units with nothing to do | hold (there is no naval or air combat logic) |

Missions the AI never issues: pillage, espionage, trade missions (Great Merchants lightbulb, join
or start golden ages instead), air missions, blockades, ranged attacks. Every unit of the AI is
promoted (`units::promotion_value`: strength, defense for garrisons, moves for field units, first
strikes, healing, spell damage for casters, and the spells a sphere opens) and upgraded when gold
allows (`units::upkeep`: the best upgrade by power, keeping a reserve of 20 + 3 per city, twice
that for non-garrisons). A bankrupt empire scraps surplus garrisons and its weakest field units
(`empire::disband`; heroes never).

### Buildings

A building is built through these paths:

1. **The generic value** (`production::generic_building_value`, step 9-10 of `decide`; also what
   research uses for the techs that open buildings): points per turn, a hammer being worth 15,
   then score = value x 100 / (turns + 3), only buildings within 50 turns (12 while the city grows
   toward a settler). It reads these fields and nothing else:

   | XML | Weight |
   |---|---|
   | `YieldChanges`, `YieldModifiers`, `GlobalYieldModifiers` | food 20, production 15, commerce 7 a point (modifiers on the city's rate; global ones x cities up to 8 / 2) |
   | `CommerceChanges` + `ObsoleteSafeCommerceChanges`, `CommerceModifiers`, `GlobalCommerceModifiers` | gold 8, research 9, culture 4, espionage 1 a point; flat culture again (x150, up to 3) in a city at culture level 1 or less for a building up to 120 hammers |
   | `iHappiness`, `iAreaHappiness`, `iGlobalHappiness`, `iStateReligionHappiness` (with a `StateReligion`), `BonusHappinessChanges` (resources the city has) | 180 a face at the cap, 100 one below, 35 otherwise; area and global ones also x30 / x50 per other city |
   | `iHealth`, `iAreaHealth`, `iGlobalHealth`, `BonusHealthChanges` | 120 at the cap, 60 within 2, 20 otherwise |
   | `SpecialistCounts`, `iFreeSpecialist`, `iGreatPeopleRateChange`, `iGreatPeopleRateModifier` | slots x15 x (population - 4, 0-8); free specialist 120; great people points 40 |
   | `iDefense` | /2, x6 under threat |
   | `iExperience`, `UnitCombatFreeExperiences` (up to 4), `DomainFreeExperiences` (land only), `iMilitaryProductionModifier` | by the city's production |
   | `iTradeRoutes`, `iFoodKept`, `iWorkerSpeedModifier`, `iMaintenanceModifier`, `iHealRateChange`, `iAIWeight` | 60 a route; food kept by the food rate; maintenance saved as gold (80, 160 when short) |
   | leader favourites | +300 for the leader's `FavoriteWonder`, +400 for its `iFavoriteEarlyWonder`/`iFavoriteLateWonder` class |

   Everything else a building does is invisible to it: free resources (`FreeBonus`, including the
   mana of the palaces and many wonders), free promotions (`FreePromotion`,
   `bApplyFreePromotionOnMove`), religion spread, shrine commerce, specialist yields and commerce,
   sea plot yields, resource-based modifiers, free techs, golden ages, the Armageddon counter,
   spell resistance, crime, war weariness, and every effect that lives in Python. A building whose
   value is 0 or less is never chosen by this path; after Writing a city always has a process to
   fall back on, so a building with no value is not built at all (before it, `production::fallback`
   builds the quickest building the city can construct, whatever its value).
2. **Unit-gating buildings**: `production::enabler_building`, `campaign::siege_building`,
   `civs::unique_casters`, the Grigori's Infirmary (see Units: production).
3. **Victory pursuit** (`victory::production`): the five towers for a Tower of Mastery victory
   (the Tower of Mastery itself in the best city), culture buildings and wonders in the cities
   nearest legendary for a cultural victory (by `building_culture`: flat culture and culture %
   only), the final Altar of the Luonnotar (its stages are built only by Great Prophets, which the
   AI does not do).
4. **Civilization hooks** (`civs::building_value`): the Clan of Embers' Warrens is the only
   building valued by name.
5. **Great people** (`units::great_person_build`): a Great Prophet (or Bard, Sage, Merchant) builds
   a shrine (a building with `HolyCity` and `GlobalReligionCommerce`) in a holy city of ours; an
   Engineer rushes a world or national wonder a city is building. No other building is built by a
   great person.
6. **City spells** (`magic::turn_value`, `magic::caster_target`): the buildings spells create
   (`CreateBuildingType`) are valued as a quarter of their building value; a caster that keeps a
   `bRequiresCaster` building stays in its city.

The Palace (`bCapital`) is never built unless the capital was lost (`production::building_allowed`).
Wonders go through the same generic value as any building (no city is chosen to build them; the
first city whose queue empties and finds one worth it starts it).

### Spells

Every AI cast goes through the rules' `unit_can_cast`, which refuses spells with `bAllowAI` 0 to AI
players, plus the AI-only conditions some `req*` functions keep (docs/AI_ONLY_RULES.md, A2).

- **End of the turn** (`casting::end_turn`, every unit of the player): the best spell by
  `magic::choose_turn_spell` (below); when nothing is worth casting by it, FfH's own choice
  (`casting::choose_spell`, a port of `CvUnit::chooseSpell`), then a buff (`casting::buff_cast`: the
  spell whose added promotions have the highest `iAIWeight`), then a summon (`casting::summon_cast`,
  when an enemy is within the summon's moves). Summons cast this way attack at once
  (`casting::summons_attack`).
- **FfH's terms** (`casting::ffh_value`): summons by their strength when an enemy is near, damage x10
  per enemy in range (minus own and neutral units), added promotions by the caster's promotion
  value, removed ones minus, unit conversion (new type's power - old), created building (its
  building value), feature or improvement +10, dispel, population x50, gold cost -4 a gold,
  immobility, the caster's sacrifice (minus level x power), halved when resistable, + `iAIWeight`.
  A spell none of whose terms apply and whose `iAIWeight` is 0 is worth 0 and is never cast by
  this choice. `iAIWeightCity` and `iAIWeightWar` are not read.
- **`magic.rs`** (every civilization but the barbarians, `magic::enabled`) values in hit points
  what a spell does to the units it reaches, for the spells it models (`magic::understood`: not a
  world spell, not an ability, and with damage, a summon, a promotion added or removed,
  immobility, a dispel, a created building, or one of the Python spells it models: Crush, Pillar
  of Fire, Destroy Undead, Domination, Heal, Earthquake, Spring, Scorch, Vitalize, Recruit). Other
  spells keep FfH's terms. It casts before attacks (`magic::before_attack`, from
  `campaign::strike` and `military::assault`: every caster within two plots of the target casts
  what weakens the defenders, then summons, then blessings on the attackers) and at the end of the
  turn (damage and curses on enemies that can reach us, summons that attack, lasting blessings,
  short ones by the defense they add, healing, terraforming, city spells).
- **World spells** (`bGlobal`, once a game): cast by the same choices once `world_spells::ready`
  allows (FfH's AI-only timing, moved out of the rules, with three expert moments: Arcane Lacuna,
  Worldbreak, Religious Fervor); the Sidar hold Into the Mist for a war (`civs/sidar.rs`).
- **Civilization hooks** (`civs::spell_value`, `civs::turn`, `civs::unit`): Calabim Feast and Gift
  Vampirism, Clan of Embers For the Horde and Rantine's Convert City, Doviello worker upgrades,
  Luchuirp Golden Hammers, Bannor Recruit, Balseraphs Loki's Disrupt, Lanun Pirate Cove, Infernal
  Manes (Add to City), Sidar Into the Mist.
- **Casters**: promoted toward the spheres whose spells they can cast (`magic::promotion_value`,
  `magic::spell_worth`); moved by `magic::caster_target`. The AI trains casters only through the
  caster step of production, the victory Tower goal and `civs::unique_casters`; no AI builds a Mage
  Guild for its own sake (its value is 0).

### Projects

`production::best_project` takes a project with `iAIWeight` > 0, within 40 turns, at
`weight x 100 / (turns + 5)`, only when no building is worth building; it skips six rituals left to
dedicated logic (Samhain, The Draw, Stir from Slumber, Bane Divine, Glory Everlasting, Purge the
Unfaithful). The Illians build The White Hand and Stir from Slumber in their most productive city
as soon as they can (`civs/illians.rs`).
