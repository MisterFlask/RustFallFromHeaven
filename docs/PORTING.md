# Porting status

The rules port is complete: every rules function of the FFH2 0.41 DLL and rules Python is ported
(no `todo!()` is left in `ffh-core`), and an audit wave has compared every `ffh-core` file and the
`ffh-data` loaders line by line with the C++/Python. Per-class function maps live in
`docs/porting/*.md`; the AI (new code) is described in `docs/AI.md`.

Legend: **done** = fully ported and tested; **ported + audited** = every function ported, then
re-read against the original in the audit wave (see "Audit and stress testing" below).

## Data (ffh-data, CvInfos.cpp read functions)

| Module | Classes | Status |
|---|---|---|
| infos/game.rs | Commerce, GameSpeed, World, Climate, SeaLevel, Era, Handicap, GameOption, Goody, Hurry, Upkeep, CultureLevel, Victory, Process, Emphasize | done, audited |
| infos/game.rs | Diplomacy (+ DiplomacyResponse): the leaders' lines, presentation only (ffh-play `view/speech.rs`) | done |
| infos/civics.rs | Specialist, Religion, Corporation, CivicOption, Civic, Vote, VoteSource, EspionageMission, Project, Tech | done, audited |
| infos/unit.rs | Unit, UnitClass, Promotion, Spell, Build, SpecialUnit, Automate, Command, Mission | done, audited |
| infos/building.rs, civ.rs | Building, BuildingClass, SpecialBuilding, Trait, Civilization, LeaderHead | done, audited |
| infos/terrain.rs, basic.rs | Yield, Terrain, Feature, BonusClass, Bonus, Improvement, Route + 12 basic | done, audited |
| infos/event.rs | Event, EventTrigger | done, audited |
| defines (GlobalDefines) | generated `defines_gen.rs` | done, audited (unlisted text defines read as -1, see DATA.md) |

## Rules engine (ffh-core)

| C++ | Rust | Status |
|---|---|---|
| CvRandom | rng.rs | done (bit-exact, parity-tested) |
| CvStructs.h | structs.rs | done |
| CvDefines.h | defs.rs | done |
| CvMap / CvPlot / CvArea / CvPlotGroup | map.rs, plot/, area.rs, plot_group.rs | ported + audited ([map.md](porting/map.md)) |
| CvUnit | unit/ | ported + audited ([unit.md](porting/unit.md)) |
| CvCity | city/ | ported + audited ([city.md](porting/city.md)) |
| CvPlayer | player/ | ported + audited ([player.md](porting/player.md)) |
| CvTeam / CvSelectionGroup / CvDeal | team/, group/, deal.rs | ported + audited ([team_group_deal.md](porting/team_group_deal.md)) |
| CvGame / CvGameCoreUtils / CvMapGenerator | game/, utils.rs, path.rs, mapgen/ | ported + audited ([game.md](porting/game.md)); `FAStar` (exe) re-implemented in path.rs |
| CvPlayerAI::AI_foundValue (+ bonus values) | found_value.rs | ported + audited (rules need it: start plots, barbarian cities); `AI_baseBonusVal` cached and reset as in C++ |
| CvCityAI::AI_assignWorkingPlots | governor.rs | new code: C++ rules-visible steps kept, choice by the governor |
| CvFractal | mapgen/fractal.rs | done (bit-exact, parity-tested) |
| Map script (BTS Fractal.py) | mapscript.rs | done (re-implementation; Python map scripts are not FFH2 rules) |

## Python (ffh-core/src/ffh)

| Python | Status |
|---|---|
| XML call-expression parser | done (ffh/pycall.rs) |
| CvSpellInterface.py | done, audited (spells_a.rs, spells_b.rs: 242 callbacks) |
| CvEventManager.py (rules handlers) | done, audited (events.rs) |
| CustomFunctions.py | done, audited (custom.rs; AI war policy and UI functions excluded) |
| CvGameUtils.py (enabled callbacks: cannotDoCivic, cannotConstruct, cannotResearch, getBuildingCostMod, canBuild, cannotTrain, ...) | done, audited (game_utils.rs) |
| CvRandomEventInterface.py | done, audited (random_events_a.rs, random_events_b.rs; the callbacks no XML references were removed, see "Changes from FFH2") |
| ScenarioFunctions.py | out of scope (scenarios are not the base game) |

## Audit and stress testing

**Audit wave.** Every `ffh-core` file group and the `ffh-data` loaders were compared line by line
with the original (conditions, constants, defaults, rounding, RNG order, side effects, FfH
blocks); `git log --grep audit` lists the commits. Most groups had no findings. Fixes:

- `CvTeam::setHasTech` / `CvPlayer::chooseTech`: the free-tech popups only reach human players; AI
  players pick research in their own turn (no free tech for AI from Python `chooseTech`).
- Random events: the Iron Orb free tech is for humans only; the Pron events use the player-level
  attitude.
- `spellRoar` stops on a NULL unit, like the Python.
- No `AI_chooseResearch` while research is disabled (FfH Stasis).
- `AI_reset` also clears the `AI_baseBonusVal` cache.
- Text defines not listed in `SetPostGlobalsGlobalDefines` (e.g. `FLOOD_PLAINS_FEATURE`) read as -1
  like C++, so flood plains are removed when a city is founded, as in the original DLL.

**Stress runs.** `ffh-stress` (see README) played about 1,400 AI games over the settings matrix
(game speeds, handicaps, start eras, world sizes, 2 to 35 civs, climates, sea levels, options, teams,
victories), also on a build with debug assertions and overflow checks, with bookkeeping
cross-checks (cities, units, groups, areas, teams) every 5 turns. Debug assertions that the C++
itself violates became `// C++ quirk:` notes, the values kept as in C++: plot city-radius counts
that go negative when a radius-3 city dies, war symmetry for players added mid-game, shared
(`bTeamShare`) building counts for team members added mid-game, blockade and
specialist-commerce counts that can go negative, and world wonder created counts above
`iMaxGlobalInstances` (FfH's equipment wonder Syliven's Perfect Lyre is counted as created again
each time a unit drops it in a city; found by the `ffh-play soak` human games).

## Not ported (by design)

- C++ AI (CvUnitAI, CvCityAI, CvPlayerAI, CvTeamAI, CvSelectionGroupAI, CvGameAI) and
  the Python AI hooks (`AI_chooseTech`, `AI_chooseProduction`, `AI_MageTurn`,
  `CustomFunctions.warScript`, ...): replaced by the new AI in `ffh-ai`. A port of them for
  head-to-head comparison lives in `crates/ffh-cppai` (docs/CPPAI.md; not the game's AI).
- UI, graphics, sound, networking, Python screens, worldbuilder, replays. The help texts of
  `CvGameTextMgr` are ported for the human client, in ffh-play (`src/view/help/`, docs/HUMAN.md
  "Help.").

## Changes from FFH2

Every way the port's rules differ from FFH2 0.41, so the starting point can be reconstructed.
Matching FFH2 is not a requirement (CLAUDE.md, "FFH2 is the starting point"): this list records
the differences; it does not ask for them to be undone.

The rules as they stood before the first sweeping rules change are the tag `classic-1`, which
stays playable at https://misterflask.github.io/RustFallFromHeaven/classic/ (docs/WEB.md, "The
classic site"); the changes below up to that tag are its differences from FFH2.

**This game's own systems** (after `classic-1`: the owner's design spec "FfH Culture Actions &
Army Combat", 2026-10-29). They were game options until 2026-11-05 (engagement combat until
2026-11-03); the owner's decision made them part of every game, so FFH2's rules without them are
no longer playable side by side. Save format 16 removed the last three options and the crusade
flag.

- **Cultural channels** (docs/CULTURE.md; not an option since 2026-11-05, the owner's decision: every game has it, the AI-only ones too; older saves do not load): a city may channel half its
  culture (that culture reaches neither the city's culture, its plots nor the culture victory) into
  one standing order aimed at a civilization, until its owner changes it: Foment Revolt (revolt
  pressure on the target's cities: unhappiness, disorder, then the city joins the caster; never a
  capital) or Spread Religion (a chance a turn to spread a religion of the city to one of the
  target's cities). The effect falls with the square of the distance; a city resists revolt with
  part of its own culture and its garrison. Not acts of war; the target's leader remembers them.
  Every channel is public to the civilizations that have met the caster. Save format 8 (it replaced the cultural
  queue of timed actions of formats 4-5: Festival, Trade Fair, Incite Unrest, Siphon Knowledge,
  Foment Revolt, Flip City, Spread Religion, removed on the owner's direction of 2026-10-30).
- **Armies** (docs/ARMIES.md; not an option since 2026-11-05, the owner's decision: every game has it, the AI-only ones too; older saves do not load): land combat units (not mages, recon units,
  heroes or summons) serve in named armies, kept one selection group each, or in a city's
  garrison. An army requisitions soldiers from the cities its owner picks for it (any city,
  directly ruled or governed) while it is short of its target size: an order at the front of the
  city's queue that interrupts its production (which keeps its progress) and cannot be taken off
  by hand. A trained land combat unit goes to its order's destination (the city's garrison by
  default, another city's garrison, or an army): anywhere but its own garrison it is created in
  the city as any unit and marches there, moved by the rules once a turn at the end of its
  owner's turn, after its owner moved (it takes no orders, never attacks, can be attacked and
  killed), advancing toward an army only
  while the army is in supply (within 3 plots of the owner's trade network); 5 turns without
  progress or a destination gone send it to the nearest garrison it can reach. Form, split, join
  and merge, garrison, rename, disband (march to the nearest garrison), upgrade all, target size,
  requisition cities. Save format 9 (2026-10-31, the owner's design: requisition and marching recruits
  replace the invisible transit, "Reinforce to N" and the governed cities' feed mode of formats
  6-8).
- **Governors** (docs/GOVERNORS.md; not an option since 2026-11-05, the owner's decision: every game has it, the AI-only ones too; older saves do not load): a player rules four cities directly
  (the capital always); every other city is governed: its production is its governor's (it cannot
  be set by hand), developing the city; an army's requisition goes ahead of it (the feed mode,
  formats 7-8, is gone since format 9, 2026-10-31). World wonders, palaces, rituals and settlers
  only in directly ruled cities. Governors come from a
  bench drawn toward the leader's flavors, with a profile (the flavors their city's production
  leans toward, its citizens' emphasis) and up to two traits (small modifiers of their city);
  replacing one costs the city 25% of its production and commerce for 5 turns; a lost city's
  governor leaves the game. Swapping a city in or out of direct rule locks it for 10 turns. A
  governed city's cultural channel is its owner's, never its governor's (2026-10-30). Save format 7.
- **Engagement combat** (docs/ENGAGEMENTS.md; not an option since 2026-11-03, the owner's
  decision: `GAMEOPTION_ENGAGEMENTS` was removed, every game fights it, the AI-only ones too, with or
  without armies; older saves do not load): a land attack is a **formation battle** (revision 4,
  the owner's Ogre Battle-style engagements, 2026-10-31), not a duel. Each side has its slots (10 a
  side on open land, fewer on rough ground and against a walled city's Sentinels) and a grid of three rows by
  `max(2, ceil((slots + 2) / 3))` lanes; the attacker stands in the formation its order gives (the
  player's) or the rules' auto formation, the defender in the auto formation; large units (giants,
  elephants, dragons, golems, treants, ...) fill two cells and have 150% HP. Units take one of eight
  roles (Frontline, Screen, Flanker, Assassin, Archer, Siege, Arcane, Divine) that say what they do
  from each row (strike, charge, stab, shoot, bombard, cast, heal) with their speed, dodge and crits;
  units of tier 4 and heroes act once more. Fear sits units out and defensive strikes open the
  battle (reserves included), then three rounds of passes, fastest first (first strikes are speed),
  every hit landing at once: melee reaches the front of a lane, shots, spells and stones its back,
  stabs anyone, a cavalry charge anyone in the field (twice as hard in the first round), each unit
  takes the units within a lane of it first, then the kill on offer or the most hurt; misses, crits
  and knock-backs; collateral damage splashes its hits on the target's neighbours (Barrage
  strengthens it, Drill's protection weakens it); a withdrawal
  replaces a killing hit (never a city's defenders). The ground: a city halves the attacker's
  arrows, a forest or jungle halves arrows and stones and stops flanking, hills weaken the
  attacker's hits and strengthen the defender's arrows; the plot's own defence (cover, hills,
  walls, culture) no longer adds to the defenders' strength. The plot falls only when no defender
  is left on it: then the launching group moves in (a city is captured); otherwise nobody moves.
  Kill credit per kill (G2) and the post-battle effects as for a duel's winner; no Fear rout.
  Stances are gone. Ships and air units keep the duel; missiles strike one unit (at half a
  battle's hit, against the plot's own defence, as before the formation battles). Save format 10.
- **Unit stats** (docs/UNIT_STATS.md; the owner's design, 2026-11-05). Every land unit type has a
  stat sheet (`stats`: HP, attack, defence, magic, magic defence, accuracy, evasion, crit, speed),
  and promotions name the stats they change (`stat_changes`: points and percentages). A physical
  hit is attack against defence, a spell magic against magic defence; the chance to hit is the
  hitter's accuracy less the target's evasion and the kind's, the crit chance the hitter's own.
  Roles keep their rows, home row, auto-pick place and anti-flank; their A%, Dv%, HP, speed, dodge
  and crit bonus became each unit's sheet, which reproduces the old numbers, and
  `ENGAGEMENT_MISS_PERCENT`, `_CRIT_PERCENT` and `_LARGE_HP_PERCENT` are gone. The strength outside
  land battles (ships, the rules that read strength, the AI's power estimates) is unchanged. What
  plays differently from day one: (1) a spell in a battle is the caster's magic against the
  target's magic defence, which take only the units' own percentages (no fortification, terrain,
  city, unit-combat, Homeland or Stigmata terms), and the target's resistance to the spell's damage
  type reduces it; (2) a promotion's or an effect's flat strength gives 10 points of attack or
  defence a strength point whatever the role (it was scaled by the role's percent), typed strength
  too; (3) a promotion's spell damage percentage raises magic in a battle, and its attack
  percentage no longer raises a caster's spells (Combat I gives +5% magic, not +20%); (4)
  Formation, which makes a Frontline unit a Screen, gives +10% defence and magic defence and -1
  speed (the Screen's Dv 110% and speed 3 before), its +10% now summed with the other percentages.
  Save format 15.
- **Place bonuses count on both sides of a battle** (the owner's rule, 2026-11-05;
  docs/UNIT_STATS.md). In a formation battle a defender's fortification, city defence, hills,
  feature and terrain defence also raise its attack on the blows it deals back, and an attacker's
  city, hills, feature and terrain attack against the plot also raise its defence on the blows it
  takes. In the duel each was on one strength only (the defender's on its defence, the attacker's
  on its attack). Why: both sides hit in a formation battle, and the place promotions (City Raider,
  City Garrison, Guerilla, Woodsman, Defensive) were worth about half of an always-on promotion
  when they counted one way. A missile's strike and the naval duel keep the one-sided terms.
- **The unit stats content pass** (the owner's approval, 2026-11-05; docs/UNIT_STATS.md section 7,
  docs/STATS_CONTENT.md). Every land unit's magic defence, accuracy, evasion, crit and a caster's
  magic now follow its role and kind instead of the day-one values derived from the old roles
  (HP, attack, defence and speed unchanged): magic defence a share of defence (Frontline 75%,
  casters 250%, ...), flankers EVA 15, assassins EVA 20 and CRIT 30 with their free Marksman,
  archers ACC 95, casters' magic 110% of their attack. Promotions gained battle stats: Drill I-IV
  +5 ACC and +10 CRIT, Flanking I-III +8 EVA, Marksman +10 ACC and +20 CRIT, Magic Resistance
  +75% MDEF, Hero +5 ACC, +5 CRIT and +20% MDEF, the Combat line +10% MAG (from +5%), and the
  races (Dwarves +10% DEF and MDEF, -5 EVA; Elves +5 ACC and EVA; Orcs +10% HP, -5 ACC; ...).
  Why: units of a role were clones, and the takeable promotions are balanced against each other.
  Spells outside battles keep FfH's damage bonus: a promotion's `spell_damage_modify` is its own
  key again, apart from its battle magic.
- **Sieges and Sentinels** (docs/SIEGES.md; the owner's design, 2026-11-04). A city with walls
  (Palisade, Walls, Wall of Stone: `iSentinels`) has two Sentinels, wall sections that fight in the
  defender's front row with `UNIT_SENTINEL`'s values (weak blows, 100 HP), their HP and Dv grown by
  the city's defence. They cost each side a slot (the city's defence no longer costs the attacker
  any), intercept the arrows and spells aimed down their lanes, are bombarded first by siege, give
  no XP, and count for capture: a walled city falls only when they are breached, and a walled city
  with no soldiers fights with them. A city with an enemy land unit next to it is besieged: its
  units get no heal from the city and its Sentinels do not repair (10% a turn otherwise); walls
  built under siege start breached, a conquered city's are breached. A bombard hits the Sentinels
  (its rate less the walls' bombard defence, in percent of their HP) and the soldiers (half that,
  never below half their HP), no longer the city's defence. Each kill's XP is shared equally by the
  killing side's units in the battle (it was shared by damage dealt). Save format 14.
- **Gifts and demands** (`crates/ffh-core/src/tribute.rs`; the owner's design, 2026-11-03). A
  deal proposed from the diplomacy screen in which the proposer only gives is a gift, one in which
  it only takes is a demand. The receiver of a gift and the payer of a demand record it with its
  value in their trade points for 30 turns (`Player::tributes`, save format 13). Gifts raise the
  receiver's relations with the giver by one per tier of value (150 points per era of the game,
  at most +3); each paid demand lowers the payer's by one, and the payer pays no other demand of
  the same player for 10 turns. An AI pays a demand when it asks no more than a tier per half of
  its own power that the demander has beyond it (the demander at 1.5 times its power or more,
  with a city within 16 plots of one of its own by land), less what it paid that player in the
  last 30 turns (ffh-ai `diplomacy::max_extorted`). Before, the AI accepted every gift with no
  effect on its attitude, and refused every demand. The AI's own one-sided deals (help to an ally,
  a vassal's offer) and those of events and loans record nothing.
- **Standing deals are cancelled as in Civ4** (2026-11-03). A deal that goes on (resources,
  gold per turn, open borders, pacts) may be cancelled by either side once
  `PEACE_TREATY_LENGTH` (10) turns have passed: the human from the diplomacy screen
  (`cancel_deal`), and an AI from twice that when it would no longer make the deal (C++
  `AI_doDiplo`: with a human, when it now dislikes them, prepares a war on them, is no longer
  pleased enough for open borders, would no longer pay a demand, or gets less than it gives, each
  side valued as its loss; with another AI, when a trade denial now applies, which ffh-core's
  rules denials make rare). Before, nothing could cancel a deal in the port: the human had no way
  to, and ffh-ai never did. A loan's payment runs with its loan.

**Balance** (after `classic-1`, each the owner's decision)

- **The palace gives +3 production instead of +1** (all 21 civilization palaces, `buildings.jsonc`;
  2026-11-01). Meant to speed up the early game without a second starting settler. Measured on
  seeds 1-24 (6 AIs, standard map, 300 turns) against the old palace: production +46% at turn 25;
  cities per civ 2.2 -> 2.7 at turn 100, 5.1 -> 6.1 at turn 200 and 7.0 -> 7.35 at turn 300;
  population +13% and FFH2 score +10% at turn 300. The AI spends the early hammers on units before
  settlers, so cities at turn 50 barely move (1.01 -> 1.06). Wars are more frequent (2.8 -> 4.0
  per game) and cities taken from other civs about double (3.2 -> 6.25 per game). Two starting
  settlers were also tried and reached about the same end state. They brought barbarian cities from
  turn 1 (`createBarbarianCities` starts once civs average two cities), and civs lost 21 cities to
  barbarians by turn 100 instead of 12.
- **Unique features double research on a tech** (2026-11-05). Each of the 17 unique features names
  a tech (`tech_research_bonus`, `tech_research_percent: 100` in `improvements.jsonc`). While the
  feature is inside the borders of a living member of a team, that team's players research the tech
  at double rate (`player_calculate_research_modifier` multiplies its modifier by 2, so commerce
  research and overflow on it count double; lump grants such as a Great Sage's are not affected).
  The cost is unchanged. It is checked every turn, so the bonus follows the tile when it changes
  hands, and progress is kept. Bonuses on one tech do not add up (the largest counts). Meant to
  make the features worth long-term planning and border fights; the techs are medium to large and
  form paths. Yggdrasil: Commune with Nature; Tomb of Sucellus: Divine Essence; Mirror of Heaven:
  Theology; Seven Pines: Religious Law; Standing Stones: Mithril Working; Guardian of Pristin
  Pass: Engineering; Broken Sepulcher: Malevolent Designs; Pyre of the Seraphic: Fanaticism;
  Odio's Prison: Rage; Letum Frigus: Sorcery; Maenalus: Arcane Lore; Bradeline's Well: Strength
  of Will; Maelstrom: Pass Through the Ether; Ring of Carcer: Astronomy; Dragon Bones: Animal
  Mastery; Remnants of Patria: Taxation; Pool of Tears: Medicine. The AI compares techs by research
  left at the rate it gets (`tech_plan::effective_left`) and values city sites with an unclaimed
  feature in reach a quarter higher per feature (`sites::research_feature_bonus`).

**Container order and path finding**

- **Per-owner object lists iterate in creation order.** C++ keeps units, cities, selection groups
  and plot groups (and deals and areas) in `FFreeListTrashArray`s, which iterate in slot order and
  reuse freed slots LIFO; after deletions the two orders differ. This changes only the order of
  RNG draws across a player's cities/units (e.g. the `CvPlayer::doTurn` city loop, unit and group
  turn loops, Python `getUnitList` loops, disband draws) and first-match tie-breaks among equal
  values (`CvCity::updateTradeRoutes`, `killGoldenAgeUnits`). No player-visible rule changes, so
  slot order is deliberately not emulated. Ids are never reused.
- **Diplomatic vote containers** do reproduce the C++ ids and slot order (`doVoteResults` order);
  a new vote takes the highest free slot, which differs from the C++ free list only if a vote
  waits an extra turn and two votes are added in one later turn. `clearSecretaryGeneral` reads
  uninitialized data in C++; the port uses id 0.
- **Path finding** re-implements the exe's closed-source `FAStar` as the A* Explorer algorithm
  over the DLL's own callbacks. Its tie-breaking is deterministic but cannot match the exe: lowest
  total cost first, ties to the most recently opened node, children in `DirectionType` order;
  finders with NULL callbacks cost 1 per step with a `stepDistance`/0 heuristic (only
  reachability and the closed set are used). Details in the `path.rs` module comment.

**No rules caches written by reads** (2026-10-28: C++ fidelity is no longer a goal in itself; `&self`
now means read-only, so a read can never change a later result or the save bytes)

- **Every path search is fresh.** C++ `FAStar`'s `bReuse` continued the previous search tree
  (not refreshed when the game changed), and `CvSelectionGroup::getPathFirstPlot` /
  `getPathEndTurnPlot` read "the last path generated by any group" back from the global finder.
  Here a group search returns its path (`Game::group_find_path` -> `GroupPath`), and the
  finders are scratch storage, not saved. Results can differ from a reused search only in ties
  between equal-cost paths. `CvPlayer::countUnimprovedBonuses` floods the border finder once
  instead of searching per plot (same answers).
- **`AI_baseBonusVal` is computed when asked** (C++ cached it per player until the next
  `AI_updateBonusValue`, each turn, so a value could lag behind the player's cities and techs, and
  the first read in a turn decided what later reads saw). Feeds `AI_foundValue` (start plots,
  barbarian cities, the AI's sites).
- **City ranks are computed when asked** (`findPopulationRank`, `findBaseYieldRateRank`,
  `findYieldRateRank`, `findCommerceRateRank`; C++ cached them in `mutable` members cleared by
  `invalidate*RankCache`, and a cache could be stale). They feed `getLargestCityHappiness` and
  the great-person city choice. Save format 3.

**Add and remove are symmetric** (2026-10-28; `crates/ffh-core/tests/add_remove.rs` adds and
removes every promotion and building and compares the saves)

- **A building's Armageddon-counter and magic-resistance effects are undone when it goes**
  (`iModifyGlobalCounter`, `iGlobalResistModify`, `iGlobalResistEnemyModify`). C++ applied them
  again on removal, so a captured or destroyed wonder (Tower of Alteration, the counter wonders)
  doubled its effect instead of ending it.
- **A radius building's city goes back to its own radius when the building goes** (3 for a full
  Kuriotates city, else 2); C++ set 2.
- **A city whose radius shrinks stops working the plots outside it.** C++ compared the radii after
  assigning the new one, so its loop never ran and the city kept citizens on plots it no longer had.

**Unreachable content removed** (2026-10-28): the 138 random-event callbacks of
CvRandomEventInterface.py that no FFH2 XML references (stock BtS events: Gold Rush, Influenza,
the Crusade, Blessed Sea, Holy Mountain, Tsunami, ...) were ported and are now deleted; the rules
could never run them.
- **`getCombatOdds`** uses IEEE single precision with the same operations as the source types; the
  original ran on the x87 FPU, so rounding at a 0.5/1000 edge could differ in rare cases (it only
  feeds AI pillage decisions and the combat log).

**Decisions that were popups or AI calls**

- **Controller decisions.** Every C++ AI decision and human popup that rules code waits on goes
  through `Controller` for every player: random-event choices (`choose_event`), diplomatic votes
  and elections (`diplo_vote`, `choose_election`), research and free techs, production,
  conquered cities, trade acceptance (`consider_offer`). Event texts are stored as English
  templates; messages go to the game log.
- **`getTradeDenial`**: the rules-based denials (war/peace feasibility of vassal and surrender
  deals, peace or war with a third team) are ported; the C++ AI's own judgement (attitude, worst
  enemy, valuations, power, victory threats) reads as no denial. Whether an AI accepts a deal is
  `Controller::consider_offer`.
- **Headless.** No active player: mission timers stay 0 and combat is always quick (no
  animation delay); "active player" UI branches never run.
- **Kuriotates settlements**: the human-only `BUTTONPOPUP_CONFIRMSETTLEMENT` (city or settlement)
  is the controller's `confirm_settlement` (ffh-play asks the human); with no answer the city stays
  as it is (radius 2, not a settlement). AI players follow the C++ rule (radius 3).
- **Team attitude value** (`CvTeamAI::AI_getAttitudeVal`, used to pick the worst enemy) averages
  the controller's `AttitudeType` (0-4) instead of the C++ raw attitude score.
- **Barbarian city founding unit**: C++ asks `CvCityAI::AI_bestUnitAI(UNITAI_CITY_DEFENSE)`
  (then ATTACK) for the free units of a new barbarian city; the port uses the city's conscript
  unit, the same stand-in FfH itself uses in `CvPlot::setOwner`.
- **Barbarian slot**: the exe (not the DLL) gives it `BARBARIAN_CIVILIZATION` /
  `BARBARIAN_LEADER` and an AI slot status; `GameSetup::standard` does the same.

**AI policy and AI bookkeeping not run by the rules**

- **AI-only Python restrictions are AI policy, not rules.** FFH2's CvGameUtils
  `cannotTrain`, `cannotConstruct`, `cannotCreate`, `cannotResearch` and `canBuild`
  contain `if not pPlayer.isHuman(): ...` branches that stop *AI* players from
  building/researching certain things (e.g. more than one scout, airships, religion
  techs the leader dislikes; in `canBuild`: no forts, no farms under Fallow, workshops need
  Guilds, no cottages under Aristocracy, no windmills under Arete). These encode the old AI's
  build policy, so they are not rules (`py_can_build` always answers -1); `ffh-ai` applies the
  `canBuild` bans itself. All branches that apply to every player are ported.
- **World spells have the same requirements for every player** (user decision, 2026-10-18).
  FFH2's `req*` functions of 20 of the 21 world spells (`CvSpellInterface.py`) add AI-only
  conditions under `if pPlayer.isHuman() == False:` -- Stasis, Legends, Rally, River of Blood and
  Gifts of Nantosuelta at 5 cities, Warcry at war with 25 units, Veil of Night at peace with 25,
  Wild Hunt at war with 20, Into the Mist at 40 units, Sanctuary, March of the Trees and Divine
  Retribution at war with two teams, Raging Seas at war with a fleet worth more than 10, Ardor at
  5 great people, Revelry at 25 population, Religious Fervor with 5 cities of the state religion,
  For the Horde with 60 barbarian units, Mother Lode at 10 mines, Arcane Lacuna with 7 typed mana
  bonuses on the map, Worldbreak at an Armageddon counter of 50. They are the old AI's timing for
  its once-a-game spell, not rules: the port drops them from the rules, and ffh-ai applies the
  same conditions as its default timing (`world_spells::ready`), so the move alone changes no AI
  game. Hyborem's Whisper had none.
- **A spell's `bAllowAI` is the AIs' to apply** (user decision, 2026-10-27). C++ `CvUnit::canCast`
  refuses a spell without `bAllowAI` (57 of the 267) to every AI player. That is the old AI's
  policy -- spells it could not value -- not a rule, so `unit_can_cast` no longer checks it: the
  rule is the same for every player. Each AI applies it: ffh-ai's generic casting keeps such
  spells out (`casting::castable_spells`) and casts only those it has logic for (Arena Battle,
  Sacrifice Altar, Wane, Kidnap: `ffh-ai/src/unflagged.rs`); the C++ AI port keeps C++'s check
  (`ffh-cppai`, `ai_can_cast`). With the other 53 kept out, no AI game changes until a spell's
  logic is added.
- **AI-only branches in the other spells' requirements are kept** (for now). Unlike CvGameUtils,
  the `isHuman() == False` checks in the other CvSpellInterface `req*` functions are ported as
  written: they decide whether a spell is castable at all for AI players. They are AI policy like
  the world spells' (docs/AI_ONLY_RULES.md, with every other AI-only branch of the rules).
- **`CustomFunctions.warScript`/`dogpile`** (AI war declarations called from
  `onBeginPlayerTurn` for AI players) are AI behaviour and are replaced by the new
  AI's diplomacy.
- **Omitted AI calls** (marked `// AI:` in the code): `AI_doFavoriteReligion` is ported
  (`player_ai_do_favorite_religion`) but not run from `doTurn`, so `favorite_religion` stays unset
  and `canConvert`'s AI-only favorite-religion restriction never applies; `AI_nowHasTech`
  (re-choose production after a tech) is omitted; the AI spell casting before `groupAttack`
  (`AI_DirectDamageCast`, `AI_SummonCast`, `AI_BuffCast`) is omitted from the rules (ffh-ai casts
  before its own attacks: `magic::before_attack`, docs/AI.md "Spellcasting"); `CvUnit::found`'s
  `isUnitAllowedPermDefense` conversion (CvUnitAI bookkeeping) is omitted.

**Game score (user decision: the score measures relative success)**

- **The score is a share of the world, not FFH2's `calculateScore`** (details, rationale and the
  held-out evidence: `docs/SCORE.md`). A civ's score is the average of its shares of the world's
  people (city sizes), land (owned land plots) and knowledge (its team's tech score, era + 1 per
  tech, counted once per team and split among the members by population), in basis points:
  alive civs always sum to 10000 and score at least 1 (largest-remainder rounding); dead players
  and barbarians score 0. A vassal keeps half of its people and land and its master's leader gets
  the rest (a total-conserving version of `getPopScore`/`getLandScore`'s half credit). Integer
  only (`Game::world_scores`, `game/turn.rs`). FFH2's formula is kept only as
  `player_legacy_score` for tools; no rule reads it. The final/victory variants of
  `getScoreComponent` (Hall of Fame, replays: not ported) were removed. Save format 2.
  Player-visible effects:
  - **Time victory** goes to the team with the largest share of the world at max turns (a tie
    still means no winner). In the 26 time-victory games of the score dataset the winner changed
    in 10.
  - **Score victory**: `target_score` is in basis points of the world (5000 = half); 0 = off.
  - **Ranks** (`updateScore`, ties to the higher index) follow the new score, and with them
    Ascension's Godslayer target, the switch-civs random event (rank 1 human) and the Cut Losers,
    High to Low and Flexible Difficulty challenge options.
  - **Barbarian-trait war** (a human Barbarian-trait civ at 1.5x the rank-1 player's score):
    identical up to turn 100, somewhat more frequent mid-game (a player leads the runner-up by
    1.5x in 6/11/11/14% of snapshots at turns 150/200/300/500, against 3/8/6/11% with FFH2's
    score).
  - The score is also marked dirty when land plots, vassals or team membership change (FFH2's
    score only read the lagged land score).

**Small differences**

- **No civilization trains another's unique units** (user decision: an FFH2 bug). FFH2's
  `canTrain` checks a unit with `UpgradeCiv` (the Kuriotates Centaurs, the Clan of Embers' Ogres,
  Goblins and Wolf Riders, the Calabim Vampires and Brujah, the Infernal Balor) only against that
  civilization's line, so any city could train them; FFH2's city screen and AI never asked, since
  they walk the city civilization's unit per class. `city_can_train` now admits such a unit only
  as the city's (or its owner's) civilization's unit for its class, like every other unit.
  `canUpgrade` is unchanged, so a unit with `UpgradeCiv` still upgrades along its civilization's
  line in anyone's hands (a captured Centaur to a Centaur Archer), as `UpgradeCiv` intends.
- **`deleteCity` clears the plots' working-city references** to the dead city (C++ leaves them,
  but `getWorkingCity()` then returns NULL, which is equivalent).
- **Python `getUnitPlayerID`** (CustomFunctions) finds any unit by id; the original only found
  units whose per-player slot was below the unit count, so some lair events silently failed.
- **Unspecified C++ evaluation order** (two RNG draws in one expression) follows MSVC x86:
  call arguments right to left, binary operands left to right (see CLAUDE.md).
- **`convert` never makes a unit its own transport.** `initUnit` places a new unit before its free
  promotions, so the Kuriotates Airship (a land unit that flies by its free Flying promotion, with
  room for land units) made by upgrading a Frigate at sea boards that Frigate; `convert` then hands
  the Frigate's cargo, the Airship included, to the Airship, which in FFH2 becomes its own
  transport (`isHiddenNationality` then recurses without end; FFH2's AI never got there, as
  `cannotTrain` keeps AI players from Airships, but a human could). The port takes the new unit
  off the unit it replaces first: the Airship stays on the water plot with the Frigate's cargo.
- **A unit dies once.** FFH2's `onUnitKilled` gifts the Infernals two Manes when a unit of the
  Council of Esus, the Ashen Veil or the Octopus Overlords (or with Death I or Entropy I) dies,
  and the Mercurians an Angel for a unit of the Empyrean, the Order or the Runes of Kilmorph (or
  their own); each rises in a random city of theirs. When that is the city the unit dies in (one
  that changed hands with an enemy unit inside: `CvPlot::setOwner` kills it there), the newcomer
  captures the dying unit (`setXY`), which kills it again: FFH2 runs that kill in full, its event
  included (more Manes, which capture it again; each gift comes at 100 - 3 x civilizations alive
  percent, so with few civilizations left this goes hundreds deep), and the outer kills then go on
  with the deleted unit. In the port a kill of a unit whose own death event is running does
  nothing: the event gifts its units once, and the unit dies in the first kill, captured if the
  newcomer captured it (`Game::dying`). Seed 8210 with the Calabim forced and their spellcasting
  off panicked on the deleted unit at turn 495: a city flipped to the Infernals by culture with an
  Ashen Veil worker inside.

**Crusades and allied loans (docs/CRUSADE.md; a ruleset flag, off by default, until 2026-11-05:
the owner's decision made them part of every game, the AI-only ones too)**

- Not FFH2: `CrusadeRules::host_slots` player slots are reserved
  for crusade Hosts (temporary AI players that own lent units and fight one war), a passed
  `VOTE_FORCE_WAR` calls the voting members to pledge forces instead of declaring war for them
  ("dedicate or defy": refusing defies the resolution at the deadline), lending declares war on the
  target, lent units earn their lender contribution when they kill or die, and dissolution drafts
  the Host's cities and gold by contribution. Allied loans are small, agreed, time-limited crusades,
  priced freely by the rest of the deal (an ongoing payment runs as long as the loan). Save
  formats 11 and 12; 16 removed the flag (`CrusadeRules::enabled`).

**Human play (ffh-play; docs/PLAY_DESIGN.md §10)**

- Leaders have lines for each civilization they speak to (`data/text/barks.md`, new writing, not
  FFH2's), mixed with FFH2's own diplomacy lines; the new-game screen's "Vanilla experience" keeps
  FFH2's alone. Presentation only.

- Human popups whose rules answer can be deferred are answered at the next control point, as in
  C++: production, events, diplomatic votes, the research popup.
- Free tech, keep/raze/disband and elections are answered during the rules call that raises them
  (C++ shows the free tech and election popups later; keep/raze is immediate there too).
- The keep/raze and disband popups share one modal (the controller cannot tell them apart); there
  is no gift button.
- Commands are applied only when no human group is busy; the exe's automatic end of turn (when
  nothing is left to move) is not ported: the human ends the turn.
- Automated and FfH AI-controlled human units idle (no `update_automated_group` AI yet).

## Next steps

1. **Save/load**: done (`crates/ffh-core/src/save/`, `Game::save` / `Game::load`; resume test in
   `crates/ffh-ai/tests/save_resume.rs`). Scenarios (world-builder saves + ScenarioFunctions.py) are out
   of scope for now; the focus is the sandbox game.
2. **Controller hooks** for the remaining rules-side stand-ins: the barbarian city defender
   (`AI_bestUnitAI`) and the favorite religion (`AI_doFavoriteReligion`); `confirm_settlement`
   (Kuriotates `BUTTONPOPUP_CONFIRMSETTLEMENT`) is done.
3. **AI quality**: see `docs/AI.md` for the current AI and its known gaps.
4. Keep the stress matrix green after rules changes (`ffh-stress batch --matrix all`, also on the
   checked build).
