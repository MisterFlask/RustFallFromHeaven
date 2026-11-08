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
| CustomFunctions.py | done, audited (custom.rs; AI war policy and UI functions excluded; the `exploreLair*` functions in lairs.rs, each tier split into a read-only outcome list and its effect, shared with the odds the interface shows) |
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

- **Building tiers** (docs/SUITABILITY.md; the owner's design of 2026-11-05; part of the rules in
  every game, no option): a city rolls a tier for every building class at founding (Lesser 25%,
  Standard 30%, Fine 25%, Grand 15%, Legendary 5%; `building_tiers.jsonc`) and keeps it for life,
  also when it changes hands. Each ordinary building (the FFH2 building is its Standard tier) has
  four more variants in `building_variants.jsonc` with weaker or stronger effects, a Legendary one
  with a name and an ability of its own; a city builds only its own tier's, and a building given
  to it at another tier becomes its own. Costs and unlocks are the same at every tier; the
  buildings that unlock units give those units experience, a promotion or slower production by
  tier. The civilization buildings whose effects are code (Warrens, Planar Gate, Citadel of Light,
  ...) scale them by tier (`ffh/events.rs`). A wonder's tier is its cost there (125/100/85/70/50%);
  not the Tower of Mastery or the Altar of the Luonnotar. New: `PROMOTION_SPIRIT_GUIDE2` (the
  Legendary Reliquary's). Save format 17.

- **Battle spells** (docs/ENGAGEMENTS.md, "Battle spells"; the owner's decisions, 2026-11-05).
  Fire II's Fireball and Air II's Maelstrom are cast only in formation battles, by their caster's
  spell actions in place of the magic bolt: fire or lightning damage (cut by the target's
  resistance) with the 50% splash on adjacent units, as the other damage spells known in battle.
  They are no longer cast on the map (FFH2: Fireball summoned a missile that struck a plot and
  died; Maelstrom did 15 lightning damage, to 30%, to the units within 2 plots). A unit with magic
  that knows a battle spell casts it in place of each attack (strikes, charges, stabs, shots,
  bombards), in every row; a healer keeps its rows. Meshabber, Mary and Auric Ascended, the
  heroes with a battle spell of their own, have a caster's magic (110% of their attack) and cast
  it in place of their strikes. The battle spell needs its religion (Ring of Flames the Ashen
  Veil's, Crown of Brilliance the Empyrean's). The Arcane Barge, a ship, keeps its Fireball as an
  ability of its class (`SPELL_FIREBALL_BARGE`, the missile); the Wonder spell still summons the
  missile it summoned (`UNIT_FIREBALL`). New spell key `battle_only`; one new spell, so saves made
  before do not load.
- **The decks** (docs/CARDS.md; the owner's design, docs/DECKS.md, 2026-11-07; save format 19, 20 since the
  Gathering's vassals are ordinary ones):
  - **Great-person cards.** A Sage (Great Scientist), Prophet, Engineer, Merchant or Bard (Great
    Artist) is born holding a secret tech, a card drawn from one world deck: its suit's cards of
    the player's era band. Once its owner knows the card's needs, the great person can discover
    it (`SPELL_DISCOVER_CARD`, using it up), and the card's content opens. A card whose great
    person dies or is used another way goes back into the deck. Card techs are never researched
    or traded. A card's world spell is cast once a game by the team that knows the card, with
    its own flag, never the civilization's world spell. Some spells are cast at a distance (a plot, city, unit, player or
    option chosen). Each card is listed in docs/CARDS.md section 5.
  - **The Armageddon deck.** FFH2's fixed Armageddon events become cards dealt face down to the
    counter's slots (10, 20, ... 90, 100) on the first turn: one card a slot, a chain's stages in
    the later slots they name. FFH2's events are cards: Blight 30, the Four Horsemen 40-50-60-70
    as one chain, Hellfire 80, Wrath 90, Apocalypse 100. They no longer all come in every game.
    A card is shown to everyone when the counter is within 5 of its slot and plays when the
    counter reaches it. FFH2's message at 10 (`EVENTTRIGGER_ARMAGEDDON`) is gone.
  - **No random events.** The weighted roll for a random event each turn is gone, and with it
    every random event and quest. The triggers rules code fires (the Adaptive and Insane traits,
    lairs, Aeron's Chosen, alignment picks) still fire. Crime, which only enabled random
    events, is gone: the global and city crime, the crime of buildings, votes and events, and the
    triggers' crime thresholds (a new city no longer draws two random numbers for it).
  - **The cards' rules** (one line a card: suit, band, needs; docs/CARDS.md section 5 has their
    status and section 2 the data keys they added):
    - *Wall of the Kings* (Engineer, II, Engineering; world wonder): land units at war with the
      owner, and barbarians, end their move when they enter its borders from outside (the path
      finder knows it).
    - *Harbor Chain* (Engineer, II, Sailing; coastal building): ships at war with the city's
      owner, and barbarian ships, cannot enter the water plots next to the city.
    - *Siege Lines* (Engineer, II, Construction; worker build): only next to a city of a civ at
      war with the builder, also in that civ's land; the holder's units on it heal as in their own
      city.
    - *Powder Charge* (Engineer, III, Blasting Powder; national unit, 2): Detonate next to a city
      of a civ at war breaches every Sentinel of that city, with no battle, and uses the Charge up.
    - *Prize Courts* (Merchant, II, Sailing; national building): the owner's ships capture an
      enemy ship they defeat half the time.
    - *Hall of Judgment* (Prophet, the Order, Orders from Heaven; national building): the owner's
      conquests keep every capturable building and lose no population.
    - *Causeway* (Engineer, I, Fishing): Work Boats build a dam on a Coast plot next to land
      (the boat is used up); land units walk onto and over it, with +50% defence there; ships
      cannot enter it.
    - *Canal Locks* (Engineer, II, Engineering): Workers dig a canal on a land plot in their
      borders next to water or another canal; ships enter it unless it is in the land of a civ at
      war with them, and cannot defend there.
    - *Mountaineering* (Engineer, II, Engineering): the holder's land units enter peaks at 3 moves,
      with +50% defence and sight 3 there.
    - *Ferries* (Engineer, I, Fishing): the holder's land units cross a one-plot strait (water
      with land on two opposite sides) as land.
    - *Wheeled City* (Engineer, III, Mithril Working; building): Pack Up turns a city (not the
      capital or a holy city) into a City Caravan keeping its name, population, buildings and
      tiers (its culture, religions and stores are lost); Unpack refounds it within 12 plots of
      where it packed up. The caravan cannot fight and is destroyed when captured.
    - *Forest Paths* (Prophet, I, Way of the Forests): in the holder's borders its units cross
      forest and jungle as open ground; enemies that start their turn in such a plot there have a
      move less (never below one).
    - *Wayside Shrines* (Prophet, I, Priesthood; worker build on a route, also outside the
      borders): the holder's units on it heal as in their own city.
    - *False Colours* (Prophet, evil, II, Deception): the Masker (national unit, 2) raises false
      colours over the owner's units on its plot until their next turn; under them they may attack
      units of a civ at peace on land without a declaration (not vassals, masters, barbarians or
      crusade hosts). The war is declared at the battle, whose defenders have no fortification and
      no defensive strikes; the colours are spent by that battle.
    - *Astral Leap* (Sage, III, Pass Through the Ether; world spell at a plot): cast by a unit in
      one of your cities at a visible land plot with no unit or city of another team; the city's
      units that can enter it move there with no moves left (not an attack).
    - *Seal of Binding* (Sage, III, Strength of Will; world spell at a unit): a world unit or hero
      you see, of a civ at war with you or of the barbarians, cannot move, attack or cast for 20
      turns; it still defends.
    - *Masque of the Red Death* (Bard, evil, III, Malevolent Designs; world spell at a city): a
      city you see of a civ at war with you, of 2 or more population, loses half its population.
    - *Purchase Charter* (Merchant, III, Mercantilism; world spell at a city): buy a city of a civ
      you have met and are at peace with, within 6 plots of your borders, of 5 or less population,
      not a capital, holy city or world-wonder city, for 100 gold a population and 2 gold a hammer
      of its buildings; the owner gets the gold and -4 attitude toward you.
    - *Cultural Annexation* (Bard, II, Philosophy; world spell at a city): a foreign city, not a
      capital, within 3 plots of your borders, where your culture is at least 30% of its plot's,
      joins you as in a culture flip; the owner gets -4 attitude toward you.
    - *Transmutation Charter* (Merchant, II, Alteration; world spell at a choice): a resource on a
      plot of yours becomes another of its group (strategic, luxury or food; water for water, land
      for land).
    - *Bazaar of Wonders* (Merchant, III, Mercantilism; world spell at a city): a city of yours
      building a world wonder (not a victory wonder) completes it at once, for 3 gold a hammer it
      still needs.
    - *Call of the Faithful* (Prophet, III, Theology; world spell): the state religion's heroes
      that no player has alive join you, in its holy city if yours, else your capital.
    - *Grand Order* (Merchant, III, Guilds; world spell): each city's current unit or non-wonder
      building gets up to 10 turns of the city's production, completing it if enough.
    - *Mammon's Bargain* (Merchant, evil, II, Currency; world spell): +1500 gold and +10 to the
      Armageddon counter.
    - *Thunderhead Tower* (Sage, II, Elementalism; building): at the start of every battle at its
      city, after the morale check and the defensive strikes, each attacking unit in the rear row
      loses 30% of its max HP, less its lightning resistance, never below 1 HP.
    - *Arcane Fortress* (Sage, III, Arcane Lore; national wonder, never captured): its city's
      Sentinels take 50% less damage from map bombards and from Bombard and Shoot in battle.
    - *Elemental Forge* (Sage, II, Elementalism; buildings): a city builds a Fire Forge (needs
      Fire mana) or a Storm Forge (needs Air mana), not both; the units it trains deal fire or
      lightning damage with their physical attack (cut by the target's resistance) and are immune
      to that element.
    - *Living Walls* (Prophet, the Fellowship of Leaves, Way of the Forests; national building):
      its city's Sentinels have the Treant's base HP (180) and attack (100), still grown by the
      city's defence.
    - *Templar* (Prophet, the Order, Orders from Heaven; promotion for Crusaders): at the end of
      each battle round the unit heals the nearest ally behind it 20 HP; immune to fear.
    - *Herald of Agares* (Prophet, the Ashen Veil, Malevolent Designs; one per player): a tier 4
      demon with Fear, trained in a city with the Ashen Veil.
    - *Siege Ballista* (Engineer, II, Mathematics; tier 2): Bombard Pierce from the middle and rear
      rows, combat limit 100, no collateral damage; upgrades to the Cannon.
    - *Trench* (Engineer, I, Mining; any worker, on the Fort's terrains or hills in its borders):
      the attacker of its plot has 3 fewer battle slots (the minimum still applies).
    - *Moat* (Engineer, I, Masonry; building): in the first battle at its city each game turn, the
      attacker's Strike, Charge and Stab in round 1 are spent.
    - *Siege Colossus* (Engineer, III, Machinery; one per player, tier 4, large): Bombard in round
      1, then Strike with Trample from the front, +100% damage to Sentinels; it bombards cities up
      to 2 plots away, and a city it bombards counts as besieged until its next turn (its Sentinels
      do not repair).
    - *The Unconquered City* (Bard, II, Code of Laws; building, never captured): while its city has
      never changed hands, its defenders (units and Sentinels) deal 50% more damage.
    - *Last Stand Hymn* (Bard, II, Drama; building): in a battle at its city, once half the
      defender's units in the battle have fallen, the rest deal double damage.
    - *Wardance* (Bard, II, Bronze Working; promotion for melee units of level 5): the unit's
      first Strike or Charge each round cleaves.
    - Map generation draws no random number for a card's improvement that never appears, so the
      cards leave the generated maps as they were.
    - *Starmetal Meteor* (Sage, II, Astronomy; world spell at a plot): a land plot you see within
      6 plots of your borders, not a peak, with no city, improvement, or unit or resource you see,
      loses its feature and gains Mithril.
    - *Anointed Champion* (Bard, II, Arete; world spell at a unit): a unit of yours that can fight,
      not a hero or a summon, becomes a hero and gains the experience for two more promotions.
    - *Legacy of Kings* (Bard, II, Philosophy; world spell at a choice): your leader gains one of
      the ten common traits it lacks, for good (the Adaptive and Insane swaps keep it).
    - *Evoker* (Sage, II, Sorcery; promotion, casters of level 4): its damage spells deal 50% more;
      it casts no charm, hex or heal.
    - *Abjurer* (Sage, II, Alteration; promotion, casters of level 4): while it is on the field the
      enemy's hexes (Madness and Gravity Well included) do not reach its side, and the enemy's
      charms end at the start of rounds 2 and 3.
    - *Spellsword* (Sage, II, Alteration; six promotions, melee units of level 3, one each): its
      first strike each round is its spell, cast with half its attack as its magic.
    - *Gravity Well* (Sage, II, Alteration; promotion, casters with Earth I): in the line when the
      battle starts, it turns the enemy's charges into strikes for the battle.
    - *The Whisperer* (Prophet, the Octopus Overlords, Message from the Deep; national unit, 2):
      in round 1 the enemy unit with the highest attack hits its own side.
    - *Halo* (Prophet, the Empyrean, Honor; promotion, disciples of level 3): once a battle its
      heal brings back the first of its side to fall, at 25% HP.
    - *Spellbreaker* (Sage, II, Alteration; promotion, melee units of level 2): +100% damage to
      summoned units and illusions; its hits end the target's charms and the effect of its
      dispellable promotions for the battle.
    - *Contingency* (Sage, II, Sorcery; two promotions for mages, one held at a time, spent when it
      fires): Strike (a defending mage casts its damage spell before round 1) or Recall (a killing
      hit sends the mage to its capital with 1 HP).
    - *Battlemage* (Sage, III, Arcane Lore; promotion, mages of level 5): its damage spells hit the
      whole target row, and from the front row it casts as many spells as from its best row.
    - *Lullaby of Ceridwen* (Bard, II, Philosophy; the Lutist, national unit, 2): its hex makes each
      enemy unit in the enemy's rearmost row in use skip its next action.
    - *Illusory Host* (Bard, III, Strength of Will; the Host Illusionist): its first spell action
      puts 4 illusory soldiers in its side's free cells; they do not act, die to any spell's hit and
      exist only in the battle.
    - *Familiars* (Sage, I, Mysticism; national unit, 4): bound to the highest-level caster in its
      stack, it takes the first spell or shot aimed at that caster in battle; the caster gains 1
      XP a turn while they share a plot.
    - *Homunculus Vats* (Sage, I, Alteration; national building): trains the Homunculus (national,
      3), which takes one of 18 promotions each teaching one sphere I battle spell.
    - A charm or hex aimed at the "rear" lands on the side's rearmost row in use; a unit holds at
      most one battle-spell promotion and one strike-spell promotion.
    - **Civilizations born mid-game** (docs/CARDS.md section 6): a card's new civilization gets a
      new player slot (`Game::add_player_slot`: no card waits for a free one), knows its founder's techs but the
      cards, and is its founder's vassal (under No Vassal States, or with a founder that is a
      vassal, it signs open borders and a defensive pact instead). No other war or peace is forced:
      its leanings are attitude modifiers. Like every civilization it is at war with the
      barbarians on both sides.
    - *The Patrian Awakening* (Sage, III, Arcane Lore; world spell): the Patrians, two cities (the
      first beside the Remnants of Patria when no civilization owns them), Mage Guilds in every
      city, casters trained at level 3 with Channeling II; the Archon leads them; +4 toward the
      caster.
    - *Founding of the Dawn Knights* (Prophet, the Order, good, Righteousness; world spell): a good
      order of two cities with four Dawn Knights (Paladin strength, its own uncapped class) and the
      Grand Master; its units heal fully after a battle won against evil civilizations; +4 toward
      good civilizations.
    - *The Fey Court Awakens* (Prophet, the Fellowship of Leaves, Commune with Nature; world spell):
      the Fey Court in the largest unclaimed forest, with Satyrs, Fawns, Treants and the Erlking;
      its units have +50% defence and +2 first strikes in forest.
    - *The Condottieri Free State* (Merchant, III, Mercantilism; world spell): the Free Companies,
      with 6 Mercenaries, 1000 gold and the Captain-General; they have +100% gold and buy units at
      2 gold a hammer left under any civic.
    - *Free City Charter* (Engineer, II, Construction; world spell at a city): one of your cities
      other than the capital becomes the Free City of its name, an independent civilization (not a
      vassal) with your land units in it, a Wall of Stone, 3 Sentinels and +50% defence for the
      units fighting in it; +4 toward you.
    - *Mirror Colony* (Sage, III, Pass Through the Ether; world spell): two cities of yours on the
      largest unclaimed region; a Mirror Gate in your capital and the first of them, through which
      a land unit steps between your gate cities that hold no enemy unit (its moves spent).
    - **The AI's attitude counts the rules' attitude modifiers** (`ai_attitude_extra`, at half: FFH's
      4 is about a step, the AI's step is 2): the Infernal Pact's +4 toward its founder and the
      cards' modifiers now change what the AI thinks.
    - *Pioneers* (Engineer, I, Crafting; national unit, 4): a worker with three times the work rate;
      captured, it becomes a Worker.
    - *Gold Standard* (Merchant, III, Mercantilism; Economy civic): its player buys units at 2 gold a
      hammer left with the gold hurry, under any Labor civic (the Condottieri's rate); -20%
      research.
    - *Beast Market* (Merchant, I, Hunting; building): the city trains Bears (75), Lions (50) and
      Tigers (75).
    - *Admiralty* (Merchant, II, Sailing; national building, water cities) and *Academy of War*
      (Bard, II, Military Strategy; national building): ships, or land units, trained there start
      with 5 experience.
    - *Elemental Foundry* (Sage, II, Elementalism; national building): trains Air, Earth, Fire and
      Water Elementals at 300 where its owner has that mana, one of each, then at 500%.
    - *Magocracy* (Sage, III, Arcane Lore; Government civic): adept units cost 50% less, other units
      25% more.
    - *Golems of the Earthmother* (Prophet, the Runes of Kilmorph, Mithril Working): the Mithril
      Golem needs no Armageddon counter and costs 600.
    - *Trading Post* (Merchant, II, Trade; worker build): only outside every border, 3 plots or more
      from every city; its plot and the land plots around it are its builder's territory whatever
      the culture, until it is pillaged.
    - *Font of Rebirth* (Prophet, II, Priesthood, not drawn by evil players; building): the first
      time a land unit of its owner would die in a battle within 3 plots, it is carried to the city
      at 10% HP instead; a second death is final.
    - *Spire of the Archmagi* (Sage, II, Sorcery; national building): each unit trained there takes
      one sphere I promotion it can acquire, its owner's choice.
    - *Mana Grafting* (Sage, II, Elementalism; spell for adept units on a mana node in their borders):
      the node becomes the type of another node its owner has; once a node, for good.
    - *Stoicism* (Sage, II, Philosophy; Cultural Values civic): war weariness causes no unhappiness;
      -15% culture.
    - *Maponos's Cradle* (Prophet, I, Agriculture; national building): +2 food, and every 20 turns a
      free Settler (a Worker when no city site is free within 10 plots).
    - *Agares's Well* (Prophet, evil, II, Way of the Wicked; national building): its owner picks one
      of 17 mana resources for the city; +3 to the Armageddon counter when built.
    - *Paymaster* (Merchant, III, Guilds; national unit, 1): Buy Out takes an adjacent enemy unit
      (not a hero or a world unit) for three times its hammer cost in gold.
    - *Muster* (Bard, II, Military Strategy; a process): 1 experience per 30 hammers, spread over the
      city's units.
    - *Sing the Stones* (Bard, II, Drama; world spell): every City Ruins your team has seen within
      3 plots of your borders, where you could found a city, becomes a city of yours of size 3.
    - *Dragon's Oath* (Bard, III, Animal Mastery; world spell): Sylvarith, a dragon of your own (one
      in the world, large, 180 HP, 150/120, Drifa's movement, Breath Fire, no Fear), joins you in
      your capital; it can be called again once it is dead.
    - *Call the Leviathan* (Prophet, the Octopus Overlords, Message from the Deep; world spell at a
      coastal city): the Leviathan comes, or a Kraken while a Leviathan lives.
    - *Judgment of Junil* (Prophet, good, II, Orders from Heaven; world spell at a player): a war
      with the civilization named ends, and a 20-turn truce holds: neither can declare war on the
      other or its vassals, defensive pacts, vassals and votes cannot start it, and their units
      are not enemies.
    - *Earthshaper's Decree* (Engineer, III, Machinery; world spell): every unimproved land plot you
      own gets the Workers' best improvement there that keeps its feature and resource (one that
      connects its resource first).
    - *The Second Sky* (Sage, I, Calendar; national wonder): when built, your team sees every plot
      with a mana resource, a unique feature, a lair or a goody hut; recon units trained there have
      +1 sight.
    - *The Plagiarist's Lens* (Sage, III, Arcane Lore; world spell at a choice): your team learns a
      card a civilization you have met knows; each civilization that knows it gets -2 attitude
      toward you, for good.
    - *Weather Engine* (Engineer, III, Machinery; ritual, 300, repeatable): each completion leaves a
      charge, spent on Blizzard (no healing, a move less), Storm (Shoot and Bombard deal half
      damage) or Fog (no Shoot in round 1; units hidden from afar) over 5 plots around a plot within
      10 of the city, for this turn and the next 5.
    - *Tidecall of Danalin* (Prophet, II, Message from the Deep; ritual, 150, coastal cities):
      its charge turns a flat land plot within 3 plots of the city, next to water, into Coast.
    - *Pillar of Dawn* (Prophet, the Empyrean, Righteousness; ritual, 500, each completion costs
      twice the last): for 10 turns your units within 5 plots of the city heal fully each turn.
    - *Rite of the Necropolis* (Prophet, III, Malevolent Designs, not drawn by good players; world
      spell): the Necropolis rises on the three City Ruins nearest the caster's capital, three cities
      of size 2 with Walls and Pyre Zombies, led by the Lich-Prince; +4 toward the caster, -4 toward
      good civilizations, not a vassal. The living units killed in a land battle on its land rise as
      its Pyre Zombies.
    - *Wake the Hill Clans* (Engineer, I, Mining; world spell): two cities on a hill site with 6
      unowned hills or peaks around it, with Hill Giants and Thrumm; the caster's vassal; +1
      production on its plots on hills.
    - *The Usurper's Crown* (Bard, III, Rage; world spell at a player): a civilization of 8 or more
      cities loses the third of them farthest from its capital to a usurper of its own civilization
      (with their buildings and garrisons) led by a hero; +4 toward the caster, -6 toward the
      parent.
    - *Oath of Fealty* (Merchant, II, Feudalism; world spell at a player): a civilization at war
      with the caster, of a third of its power or less, makes peace and becomes its vassal.
    - *Call the Aifon* (Merchant, II, Sailing; world spell): two cities on an ocean coast of the
      largest unowned region; the Deep Mother (a Kraken) leads them, and their land units walk on
      water; the caster's vassal.
    - **Civilizations born without a founder** (the Armageddon cards below) know every tech that at
      least half the civilizations' teams know, and are played by the AI.
    - *Sanctified Steel* (Prophet, II, Priesthood; building): the melee, beast and disciple units
      its city trains have Demon Slaying and Undead Slaying.
    - *Shieldbearer* (Engineer, II, Bronze Working; promotion, melee units of level 3): in the front
      row it takes the shots, stones and damage spells aimed down its lanes, at 50% less damage.
    - *Fire Arrows* (Engineer, I, Archery; promotion, archers): its shots hit the standing Sentinels
      first, at a stone's damage, with +100% damage to walls.
    - *War Drums* (Bard, I, Festivals; the War Drummer, national unit, 2): while it fights, its side
      is immune to fear and has +1 speed in round 1.
    - *Leviathan Harpoons* (Merchant, II, Optics; promotion, ships): double naval damage against sea
      beasts and the largest ships; its ship captures the Kraken and the Leviathan it defeats.
    - *Runeguard* (Engineer, II, Construction; national unit, 4): in battle immune to spell damage,
      charms and hexes.
    - *Witch-Hunter* (Prophet, II, Priesthood; national unit, 3): a stabber with +100% damage to
      adepts and disciples, which it attacks first; immune to hexes.
    - *Gryphon Riders* (Bard, II, Animal Handling; national unit, 4, needs a Stable): flying, it
      cannot end a turn over water, and its charge works as in the open on any ground.
    - *The Gilded Legion* (Merchant, III, Mercantilism; national building): its city chooses a
      civilization it has met and trains that civilization's units at 150% cost.
    - *Cairn Golem* (Engineer, I, Masonry; national unit, 3): Raise the Cairn turns it into a
      permanent Cairn on a hill: +50% defence and a Sentinel for the defender in battles there, sight
      3 for units on it.
    - *Automatons* (Engineer, III, Mithril Working; national unit, 6): not alive (immune to fear),
      immune to charms and hexes in battle, no experience, healing only in a city with a Forge.
    - *Bastion* (Engineer, III, Machinery; one per player, cannot attack): when its plot is
      attacked, its side fights with two more Sentinels at +100% HP and defence.
    - *Lantern of Lugus* (Prophet, good, I, Ancient Chants; the Dawnbringer, national unit, 2): an archer whose shots deal holy damage; it sees invisible units within 2
      plots.
    - *Orrery of Ceridwen* (Sage, II, Astronomy; world wonder): its city has +2 research per 10
      points of the Armageddon counter, and its owner sees the next two Armageddon cards before
      they are revealed.
  - **The Armageddon cards** (the counter's slot; each is dealt to its slot or not, docs/CARDS.md
    section 4). A card that brings barbarians is dealt again under No Barbarians. The Lairs
    Deepen, The Second Sun, The Twilight of the Gods, The Gathering and The Godless Age were cut
    by the owner (2026-11-08).
    - *The Wild Awakens* (20): every lair spawns three of its units at once (Barrow, Bear Den,
      Goblin Fort, Hellfire, Lion Den, Ruins), unless another civilization's unit stands on it.
    - *The Sea Recedes* (30): a stretch of sea named at the reveal (the open coast within 7 plots
      of the coast plot with the most of it, at least 10 plots); its unimproved Coast outside city
      radii becomes Grassland, its sea resources go, its ships move to the nearest water.
    - *Volcanic Spring* (40): a hill within 3 plots of each of the six largest cities, named at
      the reveal, becomes flat land with a Volcano; within 1 plot of it units lose half their max
      HP, improvements (not permanent ones) are destroyed and cities lose 2 population; the plots
      next to it give +1 food from then on.
    - *Werewolf Moon* (40): three barbarian Ravenous Werewolves next to each civilization's
      largest city; every barbarian Wolf and Wolf Pack becomes a Greater Werewolf.
    - *The Blade Returns* (50): if no Godslayer is in the world, it appears on the unowned land
      plot farthest from any city, guarded by four barbarian Greater Werewolves.
    - *The Sky Falls* (80): the ten largest cities lose 2 population (never below 1), and the units
      in them half their max HP; in each, half of the land plots of its work radius (rounded
      down; not the city's plot, peaks, water, other cities or Crater) become Crater, a new
      terrain (2026-11-08): hills with no improvement, feature or resource (routes stay), no
      yields; no improvement can be built on it (mines neither) and no city works it.
    - *The Long Night* (100, lasting): -1 food on every plot; units heal each turn only in cities.
    - *The Scaled Kingdom* (30): a new civilization of lizardmen rises in the largest unowned region
      of jungle and marsh: three cities and the Slitherking; its units move through jungle and marsh
      as open ground with +50% defence there; -2 toward the civilizations near its cities.
    - *The Brood* (30, then 50): a Spider Nest, a lair of Baby Spiders, appears in each of the three
      largest unowned forest regions; at 50 each nest still standing becomes a city of the Brood,
      with Giant Spiders (60 hammers for it) and the Spider Queen.
    - *The Infernal Gate Opens* (60; the owner's design, 2026-11-08): the Court of Mammon, an evil
      infernal faction of its own (not Hyborem's), arrives on the best free site within 4 plots of
      the borders of the civilization with the most gold, without a founder: Aurivane (its hero),
      two Champions, two Longbowmen, two Settlers, a Worker, two Manes and 200 gold. Its leader is
      Avaricious (+100% gold, buys units at 2 gold per hammer under any civic) and Organized; its
      land units Buy Out as the Paymaster does. It has the Infernals' population rules (Fallow,
      cities of 3 with Demonic Citizens, no Manes from its razed cities, the Beast of Agares costs
      its cities nothing, the Blight and the Pestilence spare its cities), and shares the Manes of
      the evil dead with them: each Manes goes to whichever of the two is in play with a city, or
      to one of them at random while both are (a draw only then). -4 attitude between it and the
      Infernals, both ways. Not under the No Hyborem or Basium option's rule: the Court is not
      Hyborem.
    - *The Sundering* (90, sharing the slot with Wrath; the owner's design, 2026-11-08): every
      civilization with more than 3 cities and a capital (not the Sheaim, the Infernals, the
      barbarians or crusade Hosts) loses the quarter of its cities farthest from its capital
      (rounded down, at least one; ties to the lower city id). They secede as a new faction of the
      same civilization type, released as the Usurper's Crown releases (buildings, culture, land
      garrisons; heroes stay), the most populous its capital, named "<civilization> of <capital>".
      Its leader is one of its civilization's leaders not in play, else a minor leader of it (a new
      `leader_heads` key, `minor_leader_of`: FFH2's scenario leaders by the civilization their
      scenarios gave them, and six invented in the card's file); a civilization with no free
      leader does not split (so the card-born civilizations never do). The faction knows its
      parent's techs (not its cards), has met who its parent has met, is nobody's vassal and at
      peace (a Barbarian-trait leader's at peace with the barbarians too), with -4 attitude
      between it and its parent both ways. The AI plays it. Played only when a civilization splits.
    - **The Infernal Pact is FFH2's again** (2026-11-08): the first team to learn it brings the
      Infernals whatever the Armageddon deck did (batch B8 had made them come once a game, by the
      Pact or the card). Whoever brings them, they do not learn their patron's cards.

**Balance** (after `classic-1`, each the owner's decision)

- **Unit roles in engagements** (2026-11-06; the owner's target: every unit of every tier has a
  place in some lineup and every formation has one that beats it; docs/UNIT_ROLES.md, which lists
  every value). New battle levers in the rules (docs/ENGAGEMENTS.md): attack patterns per row
  (volley, pierce, sweep, cleave, trample; a sweeping heal heals a row), damage against and
  resistance to unit combats as damage multipliers (`damage_vs`, `resist_vs`, and against large
  units and walls), flat armor and armor piercing, pursuit (no withdrawal from its hits), a unit's own
  heal power, target preference, damage by round; and the auto formation places two large units
  side by side in a row of 4 lanes (it left the second a row back, and a fourth out). The unit values
  that use them, tuned in the balance lab: archers and longbowmen against melee (longbowmen also
  against mounted, in volleys), mounted raiders against back-line units, hunters, rangers and
  crossbowmen against mounted, Formation +50% damage to and -50% from mounted (free on the
  Phalanx, now the Guard: attack 90, defence 130, armor), the Champion and Immortal as armored
  lines, trampling chariots and knights, cleaving berserkers that grow stronger each round,
  piercing bolts and faster casters with a ward against arrows, sweeping heals and stronger healers,
  siege engines against walls and large units, assassins and marksmen that go for casters and
  healers first. Costs: Chariot 120 to 100, War Elephant 90 to 100, Horse Archer 120 to 150, Ranger
  150 to 120. Defines: `ENGAGEMENT_HILLS_ATTACKER_PERCENT` 75 to 85, `ENGAGEMENT_BOLT_PERCENT` 60
  to 80, new `ENGAGEMENT_VOLLEY_PERCENT`, `_PIERCE_`, `_SWEEP_`, `_CLEAVE_`, `_TRAMPLE_`. The AI
  values the new promotion keys (Formation's) when it promotes.

- **Sphere spells in battle** (2026-11-06; the owner's request: adepts that matter in a battle, and
  battle spells beyond Fire II; docs/ENGAGEMENTS.md, "Battle spells", lists them). 25 new spells,
  cast only in formation battles: every sphere I gives an adept one (Firebolt, Shock, Wave,
  Thornlash, Flare and Grave Touch deal damage; Life's Mend heals; the other eleven are charms on
  the caster's side or hexes on the enemy's, for the battle: Quicken, Keen Edge, Hold the Line,
  Tremor, Shadow Veil, Battle Hymn, Wild Edge, Corrode, Daze, Ward, Chill), and seven spheres II
  give a mage a stronger one (Tidal Surge, Sunburst, Death Grip, Frost Lance, Stone Ward, Bramble
  Snare, Healing Light). A caster casts each charm it knows with its first spell actions, then its
  damage spell, or its heal when an ally is hurt by half the heal; its damage spell is the one with
  the most output (power times its pattern's extra targets), then the most damage. The map spells
  that only dealt damage are battle spells now, as Maelstrom: Entropy III's Wither, Mary's
  Contagion, the Empyrean's Crown of Brilliance (no longer cast on the map). The magic bolt is back
  at 60% (`ENGAGEMENT_BOLT_PERCENT`, 80 since the unit roles). New spell keys `battle_power`,
  `battle_pattern`, `battle_heal`, `battle_charm`; new spells, so saves made before do not load.
  The AI counts a caster with any battle spell (a charm or heal too) as one that marches with the
  army.

- **Sphere III in battle; Wonder's roll** (2026-11-06; the owner's request; docs/ENGAGEMENTS.md,
  "Battle spells"). Each sphere III gives an archmage a battle spell: 15 new battle-only spells
  (Firestorm, Chain Lightning, Deluge, Solar Wrath, Finger of Death, Wrath of the Wild; Restoration,
  a heal; Surge, Runeblades, Aegis, Quake, Nightfall, Exaltation, Confusion, Disjunction, charms and
  hexes on a whole side), Snowfall a battle power and splash of its own (still a map spell), and
  Wither a hex on the enemy side in place of its damage. Chaos III's Wonder, still a map spell,
  is a caster's damage spell in a battle before any other: each cast rolls one of 10 outcomes by
  weight (strong damage, a heal, charms and hexes on a side; one in 18 a backfire, one in 18
  nothing), new battle-only spells known by no unit. A caster casts no charm of a lower sphere
  tier than its damage spell (an archmage with Firestorm skips Hold the Line), the highest sphere
  first, and none in the battle's last round (it cast every one in id order, in any round). New spell
  key `battle_wonder`; new spells, so saves made before do not load.

- **Buff spells that pay** (2026-11-06; the owner's request: whether casters do better without
  knowing a buff spell; docs/UNIT_ROLES.md, section 9). In the spells lab, Battle Hymn and Tremor
  barely paid, and Mend, Healing Light, Restoration, Bramble Snare and Nightfall did not pay or
  cost next to Fire's damage spell. Now: Battle Hymn own side +5% damage and +10 accuracy (+20
  accuracy); Tremor +5% taken and -2 speed (-1); Chill +5% taken; Bramble Snare +15% taken (+10);
  Nightfall also -10% taken; Mend 68 (60), Healing Light 80 (70), Restoration 130 (110). Every
  charm, hex and heal now pays, with its sphere alone and next to Fire.

- **Recon units fight from any row** (2026-11-06; the Ogre Battle 64 Valkyrie pattern; this game's
  engagement combat, docs/ENGAGEMENTS.md, not FFH2's). The 20 recon unit types that do not stab
  (the Scout, Hunter, Ranger, Beastmaster and their kin; the assassins with Marksman keep their
  stabs) strike twice from the front row, as before, and shoot once from the middle and once from
  the rear (they struck once from each: a strike reaches the front enemy of a lane, a shot its
  backmost unit). They prefer the middle row, then the front, then the rear (`engage_row_order`):
  with room the auto formation puts them in the middle, behind the line, where they shoot at the
  enemy's archers, casters and siege. Their kind is now a shot, so Formation no longer makes them
  anti-flanking, and the auto formation takes them among the shooters. Stat sheets unchanged.
  The AI needs nothing new: its formations are the auto formation.

- **A national unit's limit is a price** (2026-11-06; `NATIONAL_UNIT_OVER_LIMIT_COST_PERCENT` in
  `rules_defines.jsonc`, a new unit class key `hard_limit`). FFH2's national classes (most
  tier-4 units, 4 per player each) stopped at the limit; now the limit is how many a player trains at
  the normal cost, and each one past it costs 500% (`player_get_production_needed_unit`; the
  units alive count, so a unit still in production costs 500% once others of its class are
  finished, and fewer alive bring the normal cost back). Upgrading into such a class pays the
  same price (the upgrade's gold follows the production cost). Meant as a sink for late-game
  production that is not more economy: rich cities turn spare hammers into army quality instead of
  every building and half their worth in gold. The price does not climb past 500%, so the sink
  never runs out. The casters' classes keep FFH2's limit (`hard_limit`: the Archmage, the Druid and
  the seven High Priests, units that start with Channeling): one more caster is more spells a
  turn, which no battle slot limits. World units (heroes) are unchanged. The AI trains one with
  spare production (docs/AI.md, "Production", step 11). Measured on seeds 3-8 (4 AIs, small map,
  500 turns): on master no civilization held more than 4 of a national class; with the price, a
  civ held 1-10 national units past the old limit at the end of every game (the Bannor of seed 7
  11 Paladins and 7 Berserkers), all after turn 400, when tier 4 arrives in these games.

- **The Summoner trait makes summons much stronger** (2026-11-05; `traits.jsonc`, a new trait key
  `summon_promotion`). Every unit a spell of a Summoner leader's units summons (temporary or
  lasting, a puppet too) gets the new `PROMOTION_SUMMONERS_BOND`: +50% HP, attack, defence, magic
  and magic defence. FFH2's Summoner gave its summons a turn more (`iSummonDuration` 1) and its
  adepts the Summoner promotion (their summons with Combat I), and FFH2's Python gave a Summoner's
  Wonder creatures and Hall of Mirrors copies more turns (2 for 1; 5 for 3, by tier); none of that
  is the trait's now (the Summoner promotion stays a hero's): those creatures and copies get the
  Bond too. Keelyn, Tebryn, Os-Gabella and Hyborem have it (and
  Malchavic, who leads no civilization). One new promotion: saves made before do not load.

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
- **The Skirmisher, an early national unit** (2026-11-06, the owner's decision: national units
  are above their era's curve, with no upgrade). New `UNITCLASS_SKIRMISHER` (national: 2 per
  player at the normal cost, then 500% each, 250 hammers, as for every national class above) and
  `UNIT_SKIRMISHER`: an archer (Archer role, middle and rear rows) at Exploration, with no
  building, 50 hammers, tier 1; 4/4 with 2 moves, a first strike, +25% city and hills defence and
  the Archer's defensive strike; sheet HP 90, ATK 40, DEF 30, MDEF 30, ACC 100, EVA 10, CRIT 15,
  SPD 5 (the Archer, tier 2 at 60 hammers and an Archery Range: 80/30/30/30/95/5/10/4). It
  upgrades to nothing, so it is never obsolete and its two places stay filled. Every civilization
  but the Mercurians and the barbarians (who block it as they block the other mortal early units).
  Why: before tier 2 (a tech and a building: Bronze Working is 720 beakers in) every unit a civ
  can train fights as Frontline, so the formation battle's rows and roles do nothing for the
  opening; the limit keeps the Archer line worth building (a third Skirmisher costs more than
  four Archers). The AI needs nothing new: it is the Defense role's best pick while it costs 50 hammers
  (`production::best_unit`, city defence 550 against the Warrior's 375; at 250 hammers the
  production turns put it behind), it never passes Attack (no `ATTACK` AI), so the spare-production
  step does not take it, and requisition never takes a limited class, so it garrisons. Measured
  (seeds 1-8, 150 turns, 4 civilizations): 59 trained, one in most civilizations by turn 50 and
  two in 21 of 32 by turn 100, when no civilization had an Archer yet, none with more than two by
  turn 150; 9 kills and 3 losses. (Two AI fixes for the national price: the unit choice weighs a
  national unit at the over-limit price once its units alive and in production fill the limit, and
  a settler's escort is never one; docs/AI.md, "Production" and "Settlers and escorts".)
  Art: it shows the Archer's figure (by unit combat) until it has its own.

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

- **Players are added mid-game without a limit** (the owner's decision, 2026-11-08). FFH2 sizes
  its player arrays at 35 civ slots, and a civilization that appears mid-game (`addPlayerAdvanced`:
  the Infernal Pact, the Mercurian Gate, `formEmpire`'s events; BtS colonies) needs a slot no
  civilization has used, else nothing happens (the colony recycles a dead player's). Here such a
  civilization, a card's civilization and a crusade Host each get a new slot
  (`Game::add_player_slot`, docs/ARCHITECTURE.md): none of them fails or is withheld for lack of
  a slot (the Split City and Sovereign City events, the cards' "Needs a free player slot", a
  crusade that found no Host slot), a colony never takes over a dead civilization's slot, and the
  barbarians still move last in a turn.

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

- Not FFH2: crusade Hosts (temporary AI players that own lent units and fight one war) take a
  player slot of their own (a freed Host slot, else a new one: `Game::add_player_slot`), a passed
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
