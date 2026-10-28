# AI design (ffh-ai)

Before changing the AI, read `docs/AI_LOG.md`: the changes that did not move the win rate, why,
and what that implies (CLAUDE.md, "AI changes"). `docs/ENTITIES.md` inventories, entity by entity,
how far this AI uses every unit, building and spell, and the gaps that recur.

**It plays the way an expert human would** (the owner's decision, 2026-10-21; CLAUDE.md, "AI
changes"). Each behaviour starts from what a strong player would see, want and do in that
position, and should read as sound strategy or tactics to one, unless flavor or fun dictates
otherwise. Win-rate A/Bs tune the thresholds, weights and timing of such behaviours and catch
implementations that misfire. They do not decide whether a sound behaviour exists: when one
costs win rate, the games say why.

The C++ AI (CvPlayerAI/CvCityAI/CvUnitAI/CvTeamAI, ~60k lines, plus the Python AI hooks
in CvGameUtils.py such as `AI_chooseTech`/`AI_chooseProduction`) is not ported. It is the
main cause of FFH2's late-game turn times (per-unit re-evaluation of every plot, repeated A*
searches, Python callbacks). `ffh-ai` is a new design with two hard requirements -- legal play
and determinism -- and a standing cost preference:

- **Legal play only.** It acts through the same rules APIs a human would: missions pushed on
  selection groups (`group_push_mission`), city orders (`city_push_order`), research
  (`player_push_research`), civics/religion/sliders, deals (`implement_deal`), spells
  (`unit_cast`), upgrades/promotions, scrapping. The rules decide what is allowed; a rules
  bug is reported, never worked around in the AI.
- **Cost is a deferred concern, not a hard gate.** Prefer cheap shapes -- a few bounded floods
  per group, one real path for the chosen target, no per-unit full-map scans -- but do not reject
  a feature on cost alone: richer deterministic computation is allowed, and optimized (a
  behaviour-preserving bounding pass that computes the same decisions faster) only once a
  benchmark shows a real problem at the scale the game reaches. Determinism below is still hard.
- **Deterministic.** Decisions are pure functions of the game state. The AI never draws from
  the game RNG (its "random" war rolls hash the turn and player), never iterates a
  HashMap/HashSet, never reads the clock. Its only memory outside `Game` is a shortcut a fresh
  controller rebuilds identically: the site cache lives for one `update()` call (saves happen
  between updates), and "already played this turn" is also derived from the game state (the
  rules call again within a turn only while some unit is moving, i.e. has spent moves). So a
  game saved and resumed with a new controller plays on exactly (`tests/save_resume.rs`).

## Fog of war

The owner's decisions (2026-10-24): the AI **sees units through fog of war, for its defense and
for its attacks, never through stealth** (attacks too: conflict is much of what makes the game
fun). Unrevealed land stays unknown. Honest inference about unseen
units would need a memory of what was last seen (saved per-team state, decay, scouting to
refresh it); the C++ AI never had one either -- `CvPlayerAI::AI_getPlotDanger`,
`CvPlot::getNumVisibleEnemyDefenders` and `AI_findTargetCity` read through fog and skip only
units whose invisibility type hides them (`CvUnit::isInvisible`).

- **Threats to us: fog included** (`view::known_threat`, switch `FOG_SIGHT`): the danger map
  (`View::danger`: settler and escort risk, workers fleeing, healing, explorers, casters,
  assembly plots), a city's local threat (`production::local_threat`: cities in peril,
  emergency defenders, the garrison quota, settlers held at home, units kept from disbanding),
  barbarians and civ enemies near a city (`production::barbarians_near`, `civ_enemy_near`),
  sorties (`defense::sortie_ok`), the peril that times defensive world spells
  (`world_spells::city_in_peril`) and the enemy's forces near our cities when weighing peace
  (`diplomacy::front_forces`). FfH's hidden units (the Hidden and Invisible promotions,
  assassins, shadows, hidden nationality) stay hidden: stealth is how those units and
  civilizations play.
- **What we attack: fog included** (`view::sees_units`, switch `FOG_ATTACK`): a city's garrison
  on any revealed plot (`combat_sim::visible_defenders`, the siege and amphibious planners, the
  war gate's `diplomacy::city_best_defense`), hunting targets (`military::find_targets`),
  opportune and empty-city captures (`military::opportune`, `CAPTURE`); stealthed units left
  out. With the switch off, only what is visible: in the fog the siege and amphibious planners
  assume a defender count (2 + population/5 for a civ city, 1 + age/25 for a barbarian town)
  and the war gate assumes the best defender the owner can train (`best_trainable_defense`).
  The pathing checks (`plot_is_visible_enemy_unit_for_unit`) are the rules' and unchanged.

Map knowledge is unchanged: city sites, paths and exploration use revealed plots only.

Measured (6 civs, standard map, 300 turns). Development seeds 1-24 against master `77495f2`:
legacy +4.8%, research +7.7%, land +4.7% (|t| <= 1.5), wars declared -55% (t -4.4),
eliminations -54% (t -2.1). Fresh seeds 6101-6148, one binary, each against both switches off
(paired t):

| | `FOG_SIGHT` only | honest attacks only | both |
|---|---|---|---|
| legacy score | +3.3% (1.4) | +4.0% (2.7) | +7.3% (3.3) |
| research | +5.5% (1.4) | +9.7% (3.0) | +12.1% (2.9) |
| techs | +1.1% (0.4) | +3.3% (2.2) | +5.7% (2.3) |
| cities / land | +2.9% / +2.5% | +0.1% / +2.8% | +4.8% / +4.6% (2.0) |
| power | +7.9% (1.8) | +0.9% (0.2) | +6.6% (1.7) |
| civ cities captured | +28% (2.1) | -21% (-1.6) | -11% (-0.7) |
| wars declared | +15% (1.5) | -52% (-6.1) | -48% (-4.6) |

Seeing threats in the fog makes civs safer and their wars more effective; the honest war gate
halves the wars (the fogged estimate, the best defender the owner can train, is pessimistic:
real garrisons lag the tech) while captures fall far less, and the economy gains. That table
measured the first version, where attacks used only what was visible (then switch
`FOG_HONEST`); the owner chose attacks through fog instead, for the conflict.

`FOG_ATTACK` (fresh seeds 6201-6248, one binary, on against off, paired t): wars declared +105%
(5.1), civ cities captured +84% (3.2) -- the extra wars take cities --, civ cities lost to
barbarians -26% (-1.4), military +5% (1.4); legacy -2.8% (-1.4), research -5.3% (-1.4), techs
-3.3% (-1.6): war costs some economy. Against master before any fog change (`77495f2`, seeds
1-24), both switches on: legacy +0.5%, research +2.9%, captures +13%, wars -13% (all |t| < 1.3).

## Where the rules call the AI

`Ai` implements `ffh_core::Controller`; the rules call it where the C++ calls the AI:

| C++ | Controller | Module |
|---|---|---|
| `CvPlayerAI::AI_unitUpdate` (from `CvGame::updateMoves`) | `play_turn` | `lib.rs` |
| `CvCityAI::AI_chooseProduction` (from `CvCity::doProduction`) | `choose_production` | `production.rs` |
| `CvPlayerAI::AI_chooseResearch` / `AI_chooseFreeTech` | `choose_research` | `research.rs`, `tech_plan.rs` |
| `CvPlayerAI::AI_conquerCity` | `raze_conquered_city` | `diplomacy.rs` |
| `CvPlayerAI::AI_considerOffer` | `consider_offer` | `diplomacy.rs` |
| `CvPlayerAI::AI_doAdvancedStart` (from `CvPlayer::setTurnActive`) | `play_turn` while start points remain | `advanced_start.rs` |
| `CvPlayerAI::AI_chooseEvent` | `choose_event` | `events.rs` |
| `CvTeamAI::AI_chooseElection` / `CvPlayerAI::AI_diploVote` | `choose_election` / `diplo_vote` | `council.rs` |
| captured/gifted units | defaults in `controller.rs` | -- |
| `CvCityAI::AI_assignWorkingPlots` (citizens) | `Game::city_assign_working_plots` | `ffh-core/src/governor.rs` |

`CvPlayerAI::AI_foundValue` (and its bonus valuation) is ported faithfully into the rules
(`ffh-core/src/found_value.rs`) because starting plots and barbarian cities depend on it; the AI
reuses it to score city sites. The rules also keep the C++ team AI counters (war/peace
counters, war success, war plans) that the diplomacy below reads.

## A turn (`play_turn`)

0. **Advanced start** (`advanced_start.rs`, `GAMEOPTION_ADVANCED_START`): while the player has
   start points, C++ `AI_doAdvancedStart`'s plan with fixed choices instead of its random
   draws: the capital on the starting plot, ~10% of the points on revealing the land around
   it, the leader's unit share (10% + `iBuildUnitProb`/3) on defenders, workers and field
   units, the rest on cities (best found value within 8 plots, 500 + 250 x cities minimum)
   with culture, improvements for the citizens they can keep happy and population to work
   them, technologies (research's pick; a quarter of the city points while placing cities),
   an explorer and buildings by value per point; then `ADVANCEDSTARTACTION_EXIT`.
1. **Empire** (`empire.rs`): research when the queue is empty (a broke empire first
   researches its way out, see Finance; the victory goal's techs beeline, see Victory pursuit);
   civics every 10 turns when the summed gain in civic value exceeds a flat 100 (the anarchy's
   length is not weighed; sooner under a structural deficit; a Spiritual leader every 2 turns at
   40, see "Leader traits");
   state religion; the culture slider a cultural victory wants, then the highest research
   slider that keeps the treasury from running dry; units it cannot pay for go by More Naval
   AI's rule (see Finance, "Unit support"): at peace, while costs take more than 60% of the
   commerce, the military unit of least value (field units outside our borders first, city
   defenders last, never one with experience); idle workers beyond the need while support holds
   research below its target.
2. **Emergency production** (`production::emergency`): a city whose visible threat exceeds
   its garrison switches to a defender (the rules keep the hammers); a capital or a city near
   barbarians that has only one defender after turn 30 puts a second one first. Either switch
   happens only when `decide` would then put a fighter first (switch `EMERGENCY_DECIDE`): the two
   rules disagreed for a city whose garrison `decide` finds full (`needed + 2` defenders), which was
   switched every turn and re-chose a building, often another one -- an Illian city under a threat
   three times its defense alternated a Courthouse and an Elder Council 21 times in 30 turns, now it
   builds the Courthouse. Whether a full garrison should still grow under such a threat is a
   question for `decide`'s rule, unchanged. Measured with the contact fix ("Generic gaps"), every AI with both
   against every AI without (`EMERGENCY_DECIDE`, `CONTACT`): legacy at turns 300 / 400 1080 / 1585
   -> 1076 / 1598 (seeds 1-32) and 1090 / 1605 -> 1084 / 1609 (fresh 44001-44048); peace treaties
   437 -> 395 and 569 -> 538 (no deals with a Crusader or a player that cannot be contacted),
   eliminations 35 -> 39 and 59 -> 55: neutral, as a bug fix should be. Then, while the
   empire cannot research at its target share, cities re-decide long builds for the economy
   building they need (`production::trouble_rebuild`, see Finance), and a top city finishing a
   world wonder at peace emphasizes production (`production::wonder_emphasis`, see Citizens).
3. **Diplomacy** (`diplomacy.rs`), see below.
4. **View** (`view.rs`): enemies, a *danger map* (each visible enemy unit's attack power
   added to the plots it can reach next turn; animals only on unowned plots since they cannot
   enter owned land; city garrisons ignored), field-army targets, cities in peril, the road
   plan, escort requests. O(enemy units x reach^2 + our cities). Built after diplomacy, since
   war declarations change it; steps 1-3 read only its header (`View::header`: who we are at
   war with), so the turn analyses it once (`View::analyze`).
5. **Units** (`units.rs`): promotions and upgrades first, for every unit (switch
   `PROMOTE_ALL`, as C++ `AI_doTurnUnitsPost` promotes every unit; before it only a group's head,
   before it acted, and units that get no orders, so a unit inside a marching stack waited for its
   promotions until a strike took it out of the stack; measured on fresh seeds 9501-9540 against
   the same binary with it off: unspent promotions 0.27 -> 0.10 per player and 50-turn
   checkpoint, every game changed, win rate 16.7% on both sides, share@end +29 (t 0.30), players
   alive at the end 0.85 -> 0.80 (per player -0.04, t -1.6; eliminations 37 -> 48, cities the
   barbarians took 156 -> 162)), then every group that needs orders gets one decision by its head
   unit's role (`Unit::ai_type`, set when the city trains it):
   - **settlers** (`expansion.rs`, see "Settlers and escorts" below) go with an escort to the
     best reachable site whose danger the escort covers; the first city is founded in place
     unless a clearly better site is reachable this turn; with no site left on its landmass a
     settler goes overseas (see Naval);
   - **workers** (`workers.rs`), see below;
   - **garrisons** fill each city's quota (the rival civilizations near the city included, see
     "City defense against rival civilizations"); one beyond it attacks out of its city only while
     our strength there is 175% of the enemy's within two plots;
   - **field armies** (`military.rs`), see below;
   - **explorers** reveal the nearest unknown area and goody huts, one turn at a time to a plot
     no enemy they see can reach (a multi-turn move gets no orders until it ends, and explorers
     walked on into the animals their first steps revealed); with nothing safe in reach while a
     fifth of the home landmass is unknown (before turn 150) they wait in a city, then join the
     army (joining at once had production train another explorer);
   - **great people** build a shrine they can construct in a holy city of ours (a gold a turn for
     every city in the world with the religion, culture, a free mana: Great Prophets, and the
     Bard's, Sage's and Merchant's shrines), Great Engineers rush a wonder one of our cities is
     building when two thirds of the rush go into it; otherwise they discover, join as
     specialists or start a golden age;
   - **missionaries** go to our cities without the state religion, and, once the shrine of our
     state religion stands, to foreign cities whose borders are open to us (cities without a
     religion first; see Diplomacy, "Missionaries abroad"); **casters** go with the
     stack nearest an operation's target, to the units their blessings are for, the cities
     their city spells are for or the land they terraform (`magic::caster_target`), else to
     the most threatened city; barbarians and animals attack what they can beat,
     else roam; **ships** see Naval. Units aboard a ship get no orders (the ship lands them).
     Any group still idle is skipped so the turn always ends.
6. **End-of-turn spells** (`casting.rs`): FfH's own AI casts spells at the end of every turn
   (`CvGame::updateMoves`, "BETTER AI (Spellcasting end of turn)"): `CvUnit::chooseSpell`, then
   buffs and summons, ported term by term with this AI's values. Every civilization but the
   barbarians chooses by what each spell does instead (`magic.rs`, see "Spellcasting" below),
   which also casts before our stacks attack (`campaign::strike`, `military::assault`).

## Cities

**Production** (`production.rs`) is a priority list, evaluated when a city's queue empties
(a process at the head is re-decided every turn, so a city is never idle):

1. no defender, or a garrison overwhelmed by the visible threat -> best defender;
2. the first worker once there is something to improve; a transport for a settler waiting in
   port (also forced to the head of that city's queue by `emergency`), a sea explorer while
   our land is full and nothing is known overseas, work boats for unimproved sea resources;
3. a second defender after turn 30 in the capital and in cities near barbarians;
4. an explorer from the capital while a fifth of the home landmass is unknown;
5. settlers while sites remain on the city's landmass and nothing threatens it: at most
   (cities+1)/2 at a time (half at war), none while bankrupt, from cities of size 3+ (4+ with
   a food surplus of 3; size 1 if the city is at its happy cap: a settler eats the food
   surplus as production, so a size-2 city needs 30+ turns for one), not from a city more
   than twice as slow as our fastest one on that landmass; with no site left at home, one
   overseas settler at a time from a coastal city of size 3+ that has or can build a
   transport; while growing toward a settler the city builds only short items (warriors
   that explore and escort); a settler 3 turns from done (or waiting in the city) with no
   fighter to spare gets the cheapest adequate escort queued behind it (`expansion.rs`);
   then civilization-specific items (Kuriotates settlers for settlements and full cities and
   settlement garrisons, Lanun cove boats; see Civilization-specific below);
6. FfH tier-2 units need a building in the training city (Archery Range for archers,
   Training Yard for axemen, ...): when one would open a defender or attacker a third
   stronger, about half the cities build it;
7. extra defenders up to the quota (capital, threat, nearby barbarians, the rival civilizations
   near the city, leader; barbarians an interior city's garrison holds add none once units
   cost gold, see Finance, "Unit support");
8. a small mobile force (1 + cities/2) for barbarians and animals; more workers as worked
   plots need improving; the victory goal's items (`victory::production`: towers, culture in
   the cities nearest legendary; before expansion, item 4, when the goal is 30 turns away);
   at peace, a **wonder slot** in the top third of our cities by production (More Naval AI's
   quick wonder, `production::wonder_slot`, switch `WONDER_SLOT`): the world wonder of most
   value per turn that the city finishes within 2 x cities + 1 turns (four times that in a
   city of size 6+ making more than three hammers a citizen), not while growing toward a
   settler; heroes (not at the cost of the first expansion), only in the top third of the
   cities by production that can train them (switch `TOP_CITIES`: the rules let one city at
   a time build a hero, so the first empty queue used to lock it in);
9. the siege operation's force (`campaign::army_choice`, from `needed_force`: about two
   attackers per defender of its target, siege engines -- or the Siege Workshop that trains
   them -- for walls or 3+ defenders; city attackers only if they have at least half the
   strength of the target's best defender; every other slot in peacetime); then the army
   target (larger for military-minded leaders, doubled at war or while preparing one; at war
   the attackers are chosen for strength against cities -- city attack bonus, first strikes,
   collateral -- not mobility), a third of it siege engines at war, and a couple of siege
   engines whenever a stack is camped at a city it cannot take; in peacetime army and
   buildings alternate;
10. buildings by marginal value per production turn (yields, commerce, culture for a new
    city, happiness and health when capped, maintenance savings, specialists, experience,
    ...; a world wonder outside the top third of our cities by production only if it is done
    within the wonder slot's 2 x cities + 1 turns, `TOP_CITIES`); casters; missionaries;
    projects that do not need dedicated logic; a process.

Nothing left to build: workers, then field units (never garrisons a broke treasury would
scrap); when no more units are affordable, the strongest one whatever it costs (support is
the same for any unit and the cheapest are disbanded first, so idle hammers buy quality).
Units are chosen by power squared per production turn, so stronger units win unless much
dearer.

**Effects in the rules' code** (`code_effects.rs`, switch `CODE_EFFECTS`). FfH 0.41 gives a
building's plain effects in the XML, which the value above reads, and its special ones in Python,
ported to Rust in `ffh_core::ffh` (`events.rs`). Those were invisible to the AI: a building whose
worth lies there was valued by its XML alone, often 0, so it was never or rarely built. FfH's own
AI had the same blind spot (`CvCityAI::AI_buildingValue` sees them only through `iAIWeight`, 0
for all of them). A table now says what each such effect does, and the value adds it on the same
scale (one hammer a turn = 150):
- on units finished in the city: the Warrens' copies (half the city's production, as the Clan's
  module valued them before); experience (Cave of Ancestors, one per mana resource, for adepts at
  double weight; Brewery for dwarves and Demon's Altar for demons, 2) as the XML's free
  experience is valued; promotions (Chancel of Guardians, 20% Defensive; the Luchuirp golem
  workshops) as 15% of what the city puts into such units. These count the share of our units the
  effect covers, concentrated in the cities that can train them (adepts come from Mage Guild
  cities);
- the Planar Gate's creatures: each creature of the city's buildings (half for the ones it can
  build now) worth its hammers over 50 turns, below the rules' cap per gate; with a gate, the
  enabling buildings (Gambling House, Carnival, Public Baths, Mage Guild, ...) gain their
  creature. A summoned unit's hammers: its strength's (squared x 10 / 3) or its tier's mean cost;
- city defense in code (Citadel of Light's fire, Hall of Mirrors' copies, Mokka's Cauldron, Soul
  Forge): a fixed worth when threatened; in a frontier city (a met rival civilization's city
  within 8 plots: an expert builds it there before the war) half at war and a quarter at peace;
  elsewhere a quarter at war and a tenth at peace;
- the Mercurian Gate: the units the Mercurians bring to our team (Basium, six Angels, a settler,
  two workers), each worth its hammers over 50 turns;
- the Eyes and Ears Network's free techs (by the open borders we have), the Temple of the Hand's
  snow (the yields of the land within 2, from the terrain data: snow is 2/0/0 for the Illians),
  the towers' Strong promotion and free elemental, the Infernal Grimoire's 20% barbarian Balor
  (negative).
Not covered: the Asylum (10% Crazed, which makes a unit Enraged: +20% attack, -20% defense, and it
fights on its own: a wash), buildings that only open a spell (Arena, Demon's Altar: the spells'
own logic, "Spells FfH kept from its AI" below), the Dwarven Vaults (set by the rules; the Khazad's
finance keeps their gold, "Civilization-specific"). `FFH_AI_CODE_EFFECTS_OFF` leaves the table out
but for the Warrens. Buildings that need a mana resource are also worth that much to the mana node
that opens them (`mana::node_value`, switch `MANA_BUILDINGS`): the Luchuirp's Blasting Workshop
(Fire), Pallens Engine (Sun) and Adularia Chamber (Shadow), never built before, come once a caster
makes the node.

**Spells FfH kept from its AI** (`unflagged.rs`). 57 of the 267 spells lack `bAllowAI`, and C++
refused them to every AI player: the old AI could not value them. The rules no longer do
(docs/PORTING.md, "Changes from FFH2"); the generic casting keeps them out
(`casting::castable_spells`), and four are cast by logic of their own:
- Sacrifice Altar: a unit we disband anyway (finance, bankruptcy) standing in a Demon's Altar
  city is sacrificed first, for 10 + level squared research (switch `SACRIFICE`);
- Arena Battle (Balseraphs): an Arena city with an unhappy citizen and no games running sends a
  slave, else a spare tier-1 melee or recon unit: +1 happiness for 3 turns, and the unit dies or
  gains 1-6 experience (a slave becomes a Warrior);
- Wane (Sidar): at peace with no war prepared, one level-6+ unit a turn that its city's garrison
  can spare (not heroes, limited classes or casters, whose levels are their spheres) becomes a
  Shade, which then joins a city as a great person;
- Kidnap (Svartalfar): a recon unit of strength 4+ goes into a city with settled great people of
  the team we prepare a war against, once that war is near, and takes one (8% a strength point);
  failing, it dies or goes home and the war starts, which it would anyway. The AIs settle great
  people rarely (they discover techs), so it fires against humans more than against AIs; a test
  covers it (`a_shadow_kidnaps_as_the_war_opens`).

Measured (2026-10-27, with the Khazad's vault policy, "Civilization-specific"; full games, fresh
seeds). The four civilizations' own logic (Khazad vaults, Balseraphs' Arena, Sidar's Wane,
Svartalfar's Kidnap) against the same binary with it off (`civ_ab.py --off-base`, seeds
43001-43048, 192 pairs): win 14.1% -> 20.3% (23 / 11, p 0.058), share@end +183 (t 2.01); Khazad
+244 (t 1.19), Balseraphs +172, Sidar +222 (t 1.51), Svartalfar +93. The whole batch against master,
nine civilizations forced (seeds 43101-43148, 432 pairs): win 14.8% -> 13.0% (25 / 33), share@end
-78 (t -1.41) -- the stronger Khazad: in the 179 games of the other eight with a Khazad opponent
they won 33 -> 20, in the 205 without one 22 -> 23 (share@end -43, t -1.07). The generic parts
(frontier defense, the Mercurian Gate, mana buildings, sacrifices) are neutral there: the
Mercurians join slot 0's team in 37 -> 34 games, Malakim Citadels 1 -> 16 (dev), Luchuirp
workshops 1 -> 8 (dev). A first fresh run (seeds 42001-42148) had a bug in the slider's saving
clause (every civilization refused small deficits; found from the first turn its games left
master's) and is not counted.

Measured as one civilization's edge (`civ_ab.py --base-env FFH_AI_CODE_EFFECTS_OFF=ALL --new-env
FFH_AI_CODE_EFFECTS_ONLY={civ}`, full games): the buildings now come -- Planar Gates 0 -> 2.6 a
Sheaim game (dev) and 3.8 (fresh), with Mage Guilds 2.2 -> 4.2, Public Baths and Gambling Houses
in the gate cities and about a dozen creatures a game (Mobius Witches 2.4, Chaos Marauders 2.3,
Tar Demons 2.1, Revelers 1.9, Succubi 1.8, Manticores 1.4); Cave of Ancestors 0 -> 0.5, Hall of
Mirrors 0.1 -> 0.7, Temple of the Hand 3.0 -> 4.2, the Citadel of Light 0.4 -> 0.8 (when the
Malakim have Sorcery), Breweries earlier for the dwarves. Development (seeds 1-32, Sheaim,
Amurites, Elohim, Luchuirp, Khazad, Illians, Balseraphs, Malakim, Clan of Embers; 288 pairs): win
17.0% -> 17.7% (6 / 4), share@300 +11.8 (t 1.66), share@end -21.9 (t -0.80). Fresh (seeds
41001-41048, the same civilizations but the Clan; 384 pairs): win 18.8% -> 19.0% (10 / 9),
share@300 +4.9 (t 0.96), share@end -0.6 (t -0.02); by civilization share@end from Luchuirp +93 and
Amurites +90 to Khazad -78 and Illians -125 (|t| < 1.2 for all; the Sheaim, Luchuirp and
Illians changed sign between the two sets, the Khazad and Balseraphs were below on both). Forks at the firings (`ai-branch.sh`): a Sheaim gate
city has 2 Mobius Witches and 2 Succubi 80 turns on, none without; the Illians' Temple of the Hand
takes a city's food surplus from +4 to +6 for one hammer; a Khazad capital that builds a Brewery
instead of a Granary at turn 49 (its Ale clears the unhappiness that held the city back) has 7
cities to 6 at turn 148; a Luchuirp capital that builds one instead of Walls, 7 to 5 at turn 202.

**The standing army against rival civilizations** (`defense.rs`, round 7e): how
many defenders beyond the barbarian and capital quotas our cities hold, and where.
- *In peacetime, deterrence* (`defense::deterrence`): an expert keeps enough army that no
  neighbour who can reach him and would fight him sees an easy target, and puts it where that
  neighbour's army would land. Every AI here starts a war of expansion through the same gate
  (`diplomacy::territory_check`): its power, with the power of the civs already at war with the
  target, at least its leader's need ratio (85-180% by temperament, +30 when broke) of the
  target's defensive power; the target's cities within 16 plots; the target's weakest city
  takeable by its best attacker; a cautious leader sparing a civ it likes. For each neighbour that
  passes the other tests (met, not a vassal or master, not already at war with us), the
  defensive power that keeps its ratio under its bar with a 10% margin. The largest shortfall
  against our defensive power, divided by the power of the best defender we can train, is the
  number of extra defenders, placed in the cities facing that neighbour (on its landmass, within
  16 plots of its cities), nearest first, one a city per round, at most three a city. No
  neighbour that would see an easy target: no extra defender, however close or hostile it is.
  (In a traced game, most civs need none or one to three; a civ beside a much stronger Khazad
  needed 11-22 across its border.) More army also raises our own power against others' defense,
  so wars of expansion rise a little with it (+9% declared, Round 7e).
- *At war* (or preparing one) the flat defender every city used to add is a pool of one per city
  shared out by each city's share of the war threat (More Naval AI's `CvCityAI::AI_cityThreat`
  and floating defenders): each enemy's closeness (its cities on the landmass within 12 plots,
  `(20 + both populations) x (13 - distance) / 13`, the nearest in full and the rest a quarter)
  times its hostility (at war 3, our war in preparation 1.8) times its power over ours when it is
  the stronger (up to 3). The front holds two or three, the interior none, at the old cost.
- Production, the defenders' placement (`units::garrison_quota`: spare defenders walk from the
  interior to the border) and the upkeep scrapping read the same number. Sorties
  (`defense::sortie_ok`, More Naval AI's `AI_leaveAttack(1, 70, 175)`): a defender beyond its
  city's quota attacks out at 80% odds only while our strength in the city is 175% of the enemy's
  within two plots.

**Unit support** (`economy.rs`): FfH units beyond the free ones (handicap, civics,
population) cost gold every turn. Every optional unit (extra defenders beyond one per city
and two in the capital, the mobile force beyond its first unit, the army target, siege
engines for a camped stack, cheap units while growing, casters) is built only while one more
unit is affordable with research at 60% (30% at war or while preparing one): the estimate
takes the marginal unit cost (handicap/AI/era percentages and inflation as they apply now)
against the gold the empire would make at that slider. Without it, AIs that had run out of
buildings trained units until research was at zero and nothing but units was left to
build.

**Citizens** (`governor.rs`, in ffh-core; replaces `AI_assignWorkingPlots` and is used by the
rules wherever the C++ asks the city AI): a greedy assignment plus a swap pass. Food is valued
by what it does: a deficit is expensive; growth is worth more the more happy and healthy room
the city has, the first two points of surplus count double (a city at +0 never grows), more
than the growth target counts a third; at the happy cap a surplus is slightly unwelcome;
food building a settler counts as production. Hammers below a minimum count double;
commerce follows the sliders and is worth up to twice as much when the research slider has
been pushed down to pay the bills, or when the costs measured from the rules would leave
less than 60% for research (3 points of weight per point of shortfall in percent of the
commerce; merchants and scientists gain with it over poor tiles); a culture-starved new city
values culture. While a city in the top third of ours by production pushes a world wonder home at
peace (done within the wonder slot's horizon, Production item 8), the AI sets the player's
`EMPHASIZE_PRODUCTION` on it (`production::wonder_emphasis`, switch `WONDER_SLOT`; the governor
weighs hammers 1.5x under it, as More Naval AI's special yield multiplier does while building a
wonder) and clears it when the wonder is done, switched or at war; no change to the governor. On
every wonder for the whole build, as More Naval AI does, it cost growth and commerce for few
hammers (round 7d).

The growth target is at least the food that grows the city in 12 turns (3 at size 5, 4 at size
8), and specialist jobs worth a citizen (3+ yields and commerce) count as room to grow. Before,
the target fell to 2 at size 5 whatever the size, and a big city with happy room sat at +1 food:
on the same city states (ffh-ai games at turns 120, 200 and 300, every city assigned by both
governors) the C++ AI's `AI_assignWorkingPlots` kept cities of size 8+ at +2-3 food where ours
kept +1. Measured (300 turns): seeds 1-24 population +3.6 a civ (t 2.4), research flat; fresh
seeds 425-472 population +0.3, techs +0.4, legacy +12, every |t| < 1. Kept as the growth an
expert keeps going while there is room, not for a measured gain.

## Finance (`finance.rs`)

The zero-research death spiral: an empire whose costs (city maintenance, civic upkeep, unit
support, inflated) eat its commerce pushes the slider down to pay them, and at zero research
never reaches the techs that cut those costs (City States: Cartography; Aristocracy and
Courthouses: Code of Laws), so it stays there for the rest of the game. `finance::state`
reads the empire's finances from the rules' numbers (no cache): the gold rate at the
research share we want to keep (60% at peace, 40% at war), the cost breakdown, whether the
deficit is **structural** (maintenance and civic upkeep alone eat what that share leaves for
gold) or temporary (unit support), and the turns of reserve left. The responses, each a
single hook tagged `bankruptcy-guard`:

- **Escape research** (`empire::research`): while research is down to 30% or the deficit is
  structural, the tech (prerequisites included) with the most gold saved per beaker: one that
  unlocks a civic we would adopt that costs 3+ gold a turn less than ours, or a building that
  cuts maintenance by as much (Courthouse, counted at half). The current research keeps its
  beakers. This is the decisive response: without it the others leave twice as many civs at
  20% research or less. It overrides the research plan (`tech_plan.rs`; see Research for the
  combinations measured).
- **Civics** (`empire::civic_value`, `empire::civics`): upkeep and maintenance are valued at
  the broke rate (40 points per gold) as soon as the deficit is structural, maintenance from
  the cities' actual distance and number-of-cities parts; Aristocracy is charged for the
  cottages FfH's AI-only `canBuild` bans under it once Education is near; a structural
  deficit re-checks civics outside the 10-turn cadence when a cheaper civic would win.
- **Unit support** (`empire::disband`, `finance::disband_military`; once switch `KEEP_UNITS`): built
  units are kept, by More Naval AI's rule (C++ `CvPlayerAI::AI_doMilitary` /
  `AI_disbandUnit`), brought back by the owner's decision. At peace, while costs take more than
  60% of the commerce (its "funded percent" below 40) and units cost gold, the military unit of
  least value goes, one at a time until that clears: only combat units with at most 1
  experience; value by cost, experience and level, three times in our territory, six times
  defending a city, city defenders six times and field units twice an untyped unit; never a
  city's last two units that keep it content, a hero, a unit that would defect, a summon.
  Nothing reads the threats in sight: what was trained is kept and research pays for it. Idle
  workers beyond twice the need go while support keeps research below its target
  (`finance::disband_workers`). Kept alone it cost research around turn 200 (more units paid
  for), and the build side was changed to match (`production::garrison_holds`): an interior city
  (no other civilization's city within 8 plots, no enemy civilization's unit in sight) whose
  garrison holds the barbarians it sees (their weighted attack no more than its defense) trains
  no extra defender for them once units cost gold, since that defender would be kept for good;
  border cities still do, because the defenders trained against raiders are also the ones that
  hold them when a neighbour attacks (round 7d; docs/AI_LOG.md, "More Naval AI's disband rule").
  The previous rule (the switch off, removed once this was kept): while support takes a tenth
  of the commerce and keeps research below its target (at peace), up to two units a turn go,
  obsolete ones first (weakest relative to what we can train now): field units beyond
  1 + cities/4, garrison units beyond one per city (two in the capital and near barbarians) where no threat is in sight, idle workers
  beyond twice the need; a bankrupt empire scraps surplus garrisons and field units. That rule and
  the build rules both read the moment, so a raider walking in and out of range had cities train
  defenders the treasury scrapped a few turns later; holding defenders by a quota instead lost
  (docs/AI_LOG.md, "Young units never scrapped", "Garrison quota from lasting threats only",
  "Garrison quota from a record of threats seen").
- **Production re-decided** (`production::trouble_rebuild`, switch `TROUBLE_REBUILD`): More
  Naval AI clears every city's production when the empire runs into financial trouble
  (`CvPlayerAI::AI_doCheckFinancialTrouble`), keeping items within 3 turns of done, wonders,
  world projects and heroes (`CvCityAI::AI_chooseProduction`). Here, while the empire cannot
  research at its target share (`Finance::deficit`), a city in the middle of a longer build
  re-runs the production ladder every fifth turn and switches only when the ladder now picks an
  economy building -- one whose value is mostly commerce, research, specialists, trade or
  maintenance savings (Market, Library, Elder Council, Courthouse): the building it needs now.
  Kept: items within 3 turns of done, world and national wonders, projects, heroes, units
  without combat (settlers, workers, great people), a fighter for a city without defenders or
  under threat, every fighter at war, an economy building already under way.
- **Citizens**: the governor's commerce weight above.
- `finance::city_affordable` (a new city's maintenance and upkeep against the gold left at
  the target share) is for the settler logic and not wired in. Wired into `settler_choice`
  it removed nearly all the remaining traps (on seeds 1-12 one civ at 20% research or less at
  turn 200, none at turn 300) at the cost of 7% of the cities and no better score.

Measured (6 civs, standard map, 300 turns, seeds 1-12; seeds 13-24 in brackets): civs at 20%
research or less at turn 200 24 -> 4 [24 -> 8], at turn 300 17 -> 4 [16 -> 8], at 0% at turn
300 10 -> 3 [11 -> 4]; mean research slider at turn 300 53% -> 64% [50% -> 59%]; mean techs
19.8 -> 22.5 [20.2 -> 22.7], score 803 -> 887 [846 -> 920]; unit support 11.8% -> 9.5% of the
commerce [11.7% -> 8.5%]; no measurable change in turn time.

Tried and dropped (no measurable gain on seeds 1-12): scaling the value of
maintenance and commerce buildings (Courthouse, Market, Library, Harbor) by the share of
commerce lost to costs in `production::decide`; a fallback for cities with nothing left to
build that prefers a worker with work waiting or a garrison upgrade over the strongest unit
(the fallback is only reached by cities that know no process: `decide` picks a process
whenever one is available); capping optional units at 12% of the commerce.

**Conquest gets no XML weight (`empire::civic_value`).** `civic_value` starts from the civic's
`iAIWeight` per city. Conquest's is 700, more than any economy civic's other terms, so at turn 300
29 of 56 civs ran it (seeds 1-8; Agrarianism 12, Foreign Trade 4). Under Conquest
(`bMilitaryFoodProduction`) a city training a military unit puts its whole food surplus into the
unit and stops growing (`city_food_difference`), and production trains units most of the time: 71
of the 107 Conquest cities training units showed food +0 over a positive raw surplus (288 food a
turn into hammers), against none outside Conquest. The weight is dropped for such civics, which are
then valued by their other terms like any civic (FfH's own `AI_civicValue` penalises the food rule
only below 30 population; this AI's production ladder keeps the rule active far more often).

Measured against the AI before it (6 civs, standard map; t is the paired t over seeds):

| | 300 turns, seeds 801-944 (144 games) | full games, seeds 901-916 (`--until-supremacy`) |
|---|---|---|
| legacy score | +3.6% (t 7.8) | +5.0% at turn 300 (t 4.9), +1.7% at turn 450 |
| techs | +1.4% (t 2.7) | +3.4% (t 2.5), +4.9% at turn 450 (t 1.7) |
| research / commerce / production | +3.8% / +3.3% / +3.8% (t 2.8 / 3.5 / 3.6) | |
| population | +9.8% (t 12.9) | +10.8% (t 5.5) |
| power / military | -0.7% / -0.8% (t -0.9) | +2.2% / +4.0% at turn 300; flat at 450 |
| civ cities captured | -5.7% (t -1.3) | +6% |
| civs eliminated | -13% (t -2.5) | +8% |
| outcome | | supremacy 7 -> 9 of 16; decided 30 turns earlier (548 -> 518, t -1.1) |

Variants measured on the same seeds and dropped: the weight only for cities at their happy cap
(legacy +2.6%, pop +7.7%: the same direction, weaker everywhere), the weight kept while at war or
preparing one (legacy +1.5%, pop +4.6%; no power gained by running Conquest at war).

## Research (`research.rs`, `tech_plan.rs`)

Score = value / (turns to research + 4), with one level of lookahead (children at a quarter of
their static value and of the improvements they bring: cottages behind Education). Value terms:
the leader's favorite early techs and religion, an unfounded religion (1500, the leader's
favorite 1200 more), flavors, AI weights; buildings the civ can use (a static value, plus
production's marginal value of the building in the six cities that gain most: libraries where
there is research to multiply, markets where there is gold); land improvements counted on our
cities' plots (growing improvements such as cottages at two thirds of their grown yield,
commerce worth more when broke); a better defender or attacker weighted by the military
pressure we see (barbarians near our cities, war, time), with an early "get past warriors"
bonus, also through the next tech (Mining -> Bronze Working); the first arcane unit; happiness
buildings when cities are capped; better civics than the ones we run (City States and
Aristocracy cut maintenance); technology and open-borders trading with the civs we know; the
first transport when our landmass is full or a settler waits in port and no port of ours can
train one (and the tech before it; a settler waiting for a ship makes it a goal, see Naval);
processes (Writing, Mathematics) when cities have nothing left to build. `FFH_AI_TRACE_TECH=1`
with `FFH_AI_TRACE` lists every candidate's value and score, and the goals below.

**Goals (`tech_plan.rs`).** On top of the score, research follows an ordered list of goals
adapted from FfH's own AI (`CvGameUtils.AI_chooseTech`), recomputed at every choice (no
cache): a tier-2 military tech until one of Bronze Working, Archery, Hunting or Horseback
Riding is known (the first we can ever research of the leader's favorite early military tech,
the civ's default, Hunting for the elves and Construction for the Luchuirp, and Bronze
Working); Exploration, Mining, Calendar, Ancient Chants, Education; an early religion nobody
has founded yet, unless we know one of the three early religion techs or have a holy city
(Sanitation for agnostics); Code of Laws once there are 4+ cities or maintenance takes a fifth
of the commerce; Writing (not for `TRAIT_BARBARIAN`); Mysticism, Festivals, Cartography,
Sailing for coastal empires; Trade, Sanitation, Currency, Construction, Taxation; the per-civ
beelines (Amurites Bowyers/Sorcery, Bannor Fanaticism, Calabim Feudalism, Hippus and
Kuriotates Stirrups, Sheaim Corruption of Spirit/Necromancy/Sorcery, Sidar and Svartalfar
Poisons, Luchuirp Construction then Elementalism/Sorcery, Elohim and Malakim Priesthood, Iron
Working for the Khazad, Clan of Embers, Doviello, Lanun, Grigori, Illians and Balseraphs, then
Medicine for the Grigori; each civilization's in its module, `CivPlay::tech_goals`, with its
default military tech and lore religion, so `FFH_AI_CIV_OFF` covers them); the magic spheres
once tier 3 and economy tier 2 are reached. The
religion is the first unfounded one of the player's preferences, most preferred first: the
civ's own (Runes of Kilmorph for the Khazad, Fellowship of Leaves for the elves), the leader's
favorite early and favorite religions (data; FfH leaves these unset), then -- data-driven from
each religion's own `<Alignment>` -- the religions whose alignment (or best alignment) fits the
player's, cheapest to found first, so a good civ still grabs the cheap Runes of Kilmorph early
while the deeper aligned faiths (The Order, the Ashen Veil) follow as fallbacks; then the three
early religions as a last resort. Reading the alignment from the data (rather than a hardcoded
good->Earthmother / evil->Message-from-the-Deep map) keeps the choice correct and makes the
later religions reachable as content changes; measured economy-neutral (identical through turn
150) with the later religions now founded where before only the early three appeared.
This is not `AI_chooseTech`'s order: that function returns one tech from tier-gated blocks in
which the last assignment wins (the tier-2 military tech, then the early economy, then once
both are known the religion, Cartography, Festivals, Mysticism, Writing, Animal Husbandry,
Code of Laws for financial leaders with 6+ cities, Archery, Sailing, the leader's early techs
and wonder, then economy tier 2, the civ beelines and the tier-3 block). The list keeps those
main precedences without the tier gates, takes Writing to Cartography in the reverse order,
and leaves out Archery, Animal Husbandry, the leader's early techs and wonders, the
starting-resource techs (the score's improvement terms cover them) and the tier-3 extras
(victory techs, favorite wonders, religion heroes). The barbarians have no goals. The
researchable techs on the prerequisite path (cheapest OR branch) of the first two unmet goals,
the second at half weight, get the goal's own value plus 6000, shared by each step's part of
the path's turns (+4), and at least 150 per turn of the step: every step then scores like
researching the whole path, so a cheap gateway (Ancient Chants) inherits Education's value.
The situational terms are not scaled down and still win when large (military pressure,
transports, processes).

**City defense first (`research::defense_goal`).** Under military pressure 4+ (barbarians
near our cities past turn 60, or a war) while the best city defender we could train is below
an archer's (5 defense, +25% in cities), the cheapest researchable tech that brings one
(Archery) is researched next, whatever the plan says. Without it, civs that met the tier-2
goal without a city defender (Bronze Working without copper, Hunting, Horseback Riding, or a
leader whose "military" favorite is Festivals) never researched Archery, and established civs
fell to the barbarians mid-game (seeds 25-72: 9 civs with cities at turn 200 eliminated by
turn 300, 2 before the plan, 0 with the defense goal).

**Finance first.** The deficit escape of `finance.rs` runs before the choice and replaces it
while it applies (research at 30% or less, or a structural deficit). Measured on seeds 1-24
against that order, before the defense goal (civs at 20% research or less at turn 200, techs
at turn 300): the escape
tech as the plan's first goal instead of an override 7 -> 11, +0.1; an override only at 30%
research or less, a goal otherwise, 7 -> 13, -0.6; no escape at all 7 -> 16, +0.1.

Measured (6 civs, standard map, 300 turns, `ffh stats`; seeds 1-72, the AI before the plan
against the plan with the religion fix and the defense goal; t is the paired t-statistic over
seeds):

| at turn 300 | before | after |
|---|---|---|
| techs | 21.5 | 28.7 (t 13.2) |
| research (beakers a turn) | 85 | 144 (t 12.3) |
| legacy score (alive civs / dead as 0) | 874 / 819 | 1090 / 1033 (t 11.9 / 8.7) |
| cities / population | 7.1 / 41.4 | 7.6 / 50.1 |
| religions founded per game (turn 200 / 300) | 0.1 / 1.2 | 2.2 / 3.7 |
| civs at 20% research or less | 37 | 25 |
| cities lost to barbarians / to civs | 215 / 82 | 149 / 97 |
| civs eliminated (with cities at turn 200) | 27 (4) | 23 (4) |
| ms per turn (`ffh bench`, seeds 1-8, one game at a time on a shared machine) | 48.1 | 51.8 |

The time per turn follows the larger empires (more cities, people and units; per seed -16% to
+47%, the games being different, on a machine shared with other jobs).

The religion fix: before it, every civ headed for Way of the Earthmother while Runes of
Kilmorph was unfounded, then Way of the Forests, then Message from the Deep (the fallbacks
were searched first), so the leaders' preferences were never used; the totals barely moved
(seeds 1-24: techs 30.9 -> 30.5, religions per game 3.9 either way), but the civs now head
for their own (seeds 1-8: the Octopus Overlords founded by evil Balseraphs, Calabim and
Doviello, where the first-come order gave it to whoever was left). Without the defense goal the
plan reached 30.0 techs and 1135 legacy score (alive civs) but lost 378 cities (236 to
barbarians, 142 to civs) and 37 civs (12 with cities at turn 200).

Tried and not kept (seeds 25-72, against the plan without the defense goal: civs eliminated,
techs): Archery as a fixed goal right after the tier-2 goal (23 -> 19, 29.8 -> 26.8), after the
early religion (24, 29.0), after Cartography (25, 29.5); the plan's bonus scaled down by
military pressure, to 75% / 50% / 35% at pressure 4 / 5 / 6 (16, 27.4).

**Guns vs butter: siege research (`research::siege_offense`).** Catapult-class siege engines
crack city walls (bombard, collateral) but have low raw combat, so the score's military terms
(attacker/defender strength) never pick their tech -- Construction was chosen about once a game
where Archery was chosen fifty times, and civs ground against walled cities with no answer. A
contextual goal (the user's guns-vs-butter hunch) can route a civ toward its siege engine: when
its campaign is marching on a walled or well-garrisoned rival city that needs siege
(`campaign::needed_force().siege > 0`) **and** it is decisively stronger than that rival -- the
same `TOTAL_WAR_RATIO` (power at 140% of the target's defence) diplomacy uses to commit to total
war -- the nearest researchable step toward the engine is beelined (score `SIEGE_SCORE`, above
the economy plan, below the survival `defense_goal`). A defensive, parity, or underdog civ, and a
barbarian target, keep their research on economy. The siege engine is found data-drivenly (any
trainable unit of our civ with combat, bombard/collateral, and a city-attack/collateral unit AI --
no unit name), mirroring production's `Role::Siege`.

The gate is the finding. Measured on seeds 1-16, 690 turns, against the AI with no siege goal
(captures per war = conquest conversion; techs at fixed turns = economy; final power/empire =
domination):

| vs no-siege baseline | every offensive civ (ungated) | strength-gated (kept) |
|---|---|---|
| captures per war | +19.3% | -4.8% (noise) |
| techs @ turn 300 | -7.2% | +1.0% |
| techs @ turn 450 | -8.4% | +4.8% |
| final power | -7.7% | +0.9% |
| avg max empire | -5.2% | -0.5% |
| civs eliminated | ±0% | -9.1% |

Routing *every* offensive civ to siege works as a weapon -- wars convert far more decisively
(+19% captures per war, and mid-game power at turn 300 rises ~11%) -- but it is a losing
guns-vs-butter trade: the research a civ spends beelining siege (-7% techs by turn 300) compounds
against it, and by the endgame its power and territory are down ~7%. Lowering the beeline's
priority did not help (`SIEGE_SCORE` at 0.6M instead of 1.5M: still techs@300 -6.5%, final power
-7.4%) -- the civ pursues siege until it has it either way, so it pays the same research; only
*when* changes. The conversion gain and the economy cost both come from the same civs, the
near-parity attackers who must crack walls, so tuning cannot separate them.

The strength gate separates them by excluding that population: a civ decisively stronger than its
target resolves its war fast enough for a cracked city to pay off before the research deficit
compounds. That gate is non-regressive (economy and power flat-to-up, 9% fewer civs eliminated),
but it fires rarely -- about one to three times a game (`FFH_AI_TRACE_TECH`): by the time a civ is
that dominant it has usually already teched past Construction, or its targets are too weak to need
siege. So the validated guns-vs-butter answer is **butter almost always wins** -- the civs that
most need siege are the ones that can least afford its research -- and the AI now reserves a siege
beeline for the rare dominant attacker who can. The ungated version is kept out; the gated one is
kept in as the holistically-correct (if seldom-triggered) policy, strictly better than a baseline
that never deliberately chose siege at all.

## Settlers and escorts (`expansion.rs`)

`settler_choice` (when a city builds settlers, item 5 above) and the settler role live here.
A settler never leaves a city alone: a city founded without a defender trains one at one or
two hammers a turn, and the barbarians took such cities within a few turns of founding.

- **Escort**: the settler takes a fighter from its plot (field units first, else a garrison
  unit the city can spare: two defenders stay in the capital, one elsewhere, and the rest
  must still hold the visible threat), a second one when the best site needs more. Once per
  turn (`prepare_escorts`), a city 3 turns or less from a settler, or holding one that waits,
  with no fighter to spare queues the cheapest unit that defends at least 70% as well as its
  best one, so both leave together; the escort becomes the new city's first garrison. Waiting
  settlers also ask single field units within 6 turns to come (`View::escort_requests`).
- **Risk**: each reachable site's risk is the worst danger on the straight line there, or
  twice the danger at the site (the new city has to hold it). A settler goes only where its
  escort's defense covers the risk; with none it waits in the city (up to 10 turns, then it
  goes only where nothing can reach it; an escorted one then accepts twice its escort). A
  settler in the field whose sites all turned dangerous walks back to the nearest city.
- **Target**: sites are scored value / (turns + 3) and re-planned every turn; the site the
  settler was walking to (kept as the group's `MISSIONAI_FOUND` plot, saved with the game)
  scores 1.5x, so settlers no longer flip between sites of similar value.
- **First city**: when the "clearly better" site the first settler (or the last settler of a civ
  without cities) heads for turns out unreachable -- the rules give an AI group that cannot fight
  no path into or through danger (`path_dest_valid`, `pathValid`), a wider danger than this AI's
  map -- it founds where it stands. Before, it retried every turn and the civ never had a city:
  5 of 6 such civs in seeds 1-100 were eliminated by turn 300. On seeds 801-944 the fix changed
  8 of 144 games; 5 civs that died without a city now stand at turn 300 with 1-9 cities and 14-29
  techs (all games: legacy +0.6% at turn 300, eliminations -8%).

Measured over seeds 1-48 (6 civs, standard, 300 turns) against the version without them:
settlers destroyed 50 -> 28, cities razed 38 -> 32, cities captured by barbarians 77 -> 71,
techs at turn 300 19.6 -> 19.8; each part matters (without the forced wait captures were
104, without escort production 112). Cities per civ are unchanged (2.1 at turn 100).

Tried and dropped (same seeds): two settlers at a time for one or two cities, sites within 5
plots of our cities, the capital giving up its second defender as an escort, settlers at size
2 or before the first worker, deferring buildings while a city grows toward a settler, and
chopping forests while a settler is built (C++ `bChop`; the workers now chop for settlers,
measured flat, "Chops" under Workers). The fastest mix had 2.4-2.5 cities
per civ at turn 100 (4th city at turn 131 instead of 150) but 0.8-1.4 fewer techs and fewer
points at turn 300, the same number of cities by then, and more cities lost to barbarians,
also on top of the finance and research changes. Re-measured 2026-10-21 with a per-civilization
switch, everyone and one civilization against the rest (held threats no longer stopping settler
production, a settler per city, fewer capital defenders, no buildings while growing toward a
settler): no gain either way; docs/AI_LOG.md, "Expansion: the expert opening". The rules couple barbarian cities to ours:
`createBarbarianCities` starts once civs average two cities and its target grows with the
number of civ cities, so faster expansion brings more barbarian cities sooner.

## Workers (`workers.rs`)

A worker floods 8 turns of reach and scores every (plot, build) in our cities' radius:
yield change weighted by the working city's needs (food while it can grow, production,
commerce; cottages at two thirds of their grown yield), plus the value of connecting a
resource; worked plots count fully, others at 60%; existing improvements are replaced only
for a clear gain and never one that connects a resource. Roads come from a per-turn plan:
the next missing plot between two of our cities on the same landmass, and links from improved
resources. Score / (travel + build time). Workers flee when enemies can reach them and skip
plots under danger. FfH's AI-only build bans (`CvGameUtils.canBuild`) are respected: no forts,
no farms for Fallow civs, no workshops before Guilds, no cottages under Aristocracy, no
windmills under Arete.

**Fleeing (`workers::refuge`).** A threatened worker goes to the best plot it reaches this turn:
a city of ours, else a plot no visible enemy can reach, else one our soldiers hold, else the
least dangerous (nearer our nearest city first), and moves there with `MOVE_IGNORE_DANGER`. It
used to walk to the nearest city with a plain move, which the rules' AI path finder refuses for a
non-combat group that starts or ends a turn next to the enemy (`pathValid`, `AI_getPlotDanger`):
54% of the flee orders left the worker where it stood (seeds 1-8), and 808 of the 815 civ workers
the barbarians captured had been left on a plot in danger. Measured against the AI before it: 300
turns, seeds 801-944 (144 games), population +4.0% (t 2.2), legacy +2.7%, research +4.4%, power
+3.7%, land +2.4% (t 1.6 each); full games (seeds 901-948, 48 games) no significant change.
Choosing build targets the rules' path finder can reach (a bounded `group_generate_path` over the
best four) added nothing on top of it (300 turns legacy +1.5% for both together) and was left out.
Training workers only once an improvement is buildable (no early idle workers: -68% idle
worker-turns) measured flat to negative on the same seeds (legacy -1.0%, full games power -10.6%
at turn 300) and was not kept.

**Chops.** A plain Remove Forest or Remove Jungle is valued only when its hammers go to a city
building a settler, a worker or a world wonder (a chop's 20 hammers count as 10 turns of
production on the plot), less the feature's yields on a worked plot. Kept: the Ancient Forest, a
feature a civic of ours makes happy (the elves' forests are maintained anyway), a healthy feature
in a city with at most one health to spare, and a city's last three forests. A worker with
nothing to build within 8 turns looks up to 25 turns away (the newer cities) before it waits:
from turn 200 most idle workers had improved everything within 8 turns. Seeds 1-4 at 300 turns:
113 chops (4.7 a civ), 29 more lumber mills, 73 fewer farms. Measured with the governor change
above: seeds 1-24 research +10 (t 1.3), but fresh seeds 401-472 flat (population +0.4,
research +3.6, legacy -5, every |t| < 1; workers alone also flat). Kept as an expert's use of
the forests, not for a measured gain. Valuing every improvement on a feature by the clearing
(the feature's lost yields, its hammers) cost research (docs/AI_LOG.md, "Workers and citizens").

## Field armies (`military.rs`, `campaign.rs`, `combat_sim.rs`)

Each army group is a stack; per decision, in order:

1. follow through a group attack the rules queued (`ai_group_attack`);
2. become the garrison of an undefended city of ours it stands in;
3. merge the army units on its plot (up to 8; only units with full moves);
4. take an enemy city at war, plan or not, that it reaches this turn and finds empty, or that is
   next to it and that it clearly takes: the siege estimate with the march reserve and odds the
   assault goes on with (`opportune`, switch `CAPTURE`; More Naval AI's `AI_stackAttackCity(1,
   160)`; the targets of our operations are left to their strikes);
5. attack an adjacent target worth it (never a civ city outside a war plan, nor the target of
   our siege operation, which is left to its strike);
6. heal (`heal`, round 7f; More Naval AI's `AI_heal`, fitted to our small stacks; before it, a
   stack healed where it stood when one unit was over 35% damage and it was safe): in one of
   our cities the wounded (20%+ damage, the field rule's threshold, so a stack leaves a city only
   in a state the field rule accepts: with More Naval AI's 30% it walked out and back each turn,
   "Forks of the healing rule" below) split off and heal while the rest go on; in the field
   a stack committed to a siege stops only when more than half of it is hurt (20%+ damage), and
   only where healing is better than 10% a turn (our land or a city, not neutral or enemy land),
   else it walks into a city of ours within two turns or onto our land within one, else it
   carries on; a loose stack heals where it stands for one unit over 35%, as before (walking it
   home let raiders run: Round 7b); never a stack about to strike;
7. defend a city of ours in peril within 5 turns (not a stack marching on a siege);
8. escort a settler that asks for one (single units);
9. its siege operation (below), if the stack is committed to one: at war (or while preparing
   one) always; against a barbarian city once the force is gathered, or to step back from it;
10. hunt the best barbarian/animal/lair target within 6 turns it can take (entering an empty
   lair destroys it; an empty enemy city at war within 2 turns is walked into, `CAPTURE`);
11. join a bigger stack within 3 turns, else hold in the nearest city.

**Deciding an attack** (round 7g). Every attack in steps 1, 4, 5 and 10 and a landing's
assault (`naval.rs`) is decided on the simulation of the group attack
(`military::field_sim`: `combat_sim::simulate` in the rules' own attacker order,
`AI_getBestGroupAttacker` with its `AI_attackOdds`, against the defenders we see) as an expert
decides (`military::field_go`): attack when the attacker the rules send has good odds (65%, 75%
for a hurt stack); wear the defenders down at worse odds only when the stack clears the plot
(every defender 80% sure to fall) and the kills are worth more than the expected losses
(`gain`: `combat_sim::worth` = cost or 15 hammers a strength point, whichever is more, plus a tenth
per experience point; raiders -- any enemy but animals -- on our land or within three plots of one
of our cities count four times their worth, since left alone they take the city); a city to
capture (an opportune capture, a landing) when it falls. Before: `assess`/`attack_ok`, the best odds of 65%, or every attacker's
odds against the best defender at full health plus a third of a kill for each lost fight,
whatever the defender's strength or first strikes -- it sent four Warriors at a Griffon at 1% and
six units at an Elephant at 0.2%. A pure trade rule (attack whenever the kills are worth more)
lost units early, and the same rule with raiders at face value lost cities to the barbarians;
both were replaced (docs/AI_LOG.md, round 7g).

**Siege operations** (`campaign.rs`): one per player, rebuilt from the game state on the first
army decision of each `play_turn` call (nothing outside `Game` is remembered).

- *Target*: at war (or preparing one), a revealed enemy city within 20 plots of ours on a
  shared landmass; in peacetime a barbarian city within 14 plots. Every order of an operation
  carries its target as the group's mission AI (`MISSIONAI_ASSAULT` and the plot, as C++
  `AI_setMissionAI`; saved with the game). The city with the most committed units keeps its
  place while the force it needs stays within reach (twice our force at war, our force + 4
  against barbarians); otherwise the best value (400 + 100 per population, wonders, holy city,
  resources, a barbarian town near our cities) for its cost (the force it needs, the
  distance). Without that stickiness the target changed whenever a stack came close enough to
  see a garrison, and stacks walked back and forth between two or three cities.
- *Force* (`combat_sim::simulate`, round 7g): the strike is played out in expected values with
  the rules' odds (fortification, city defense, first strikes, the attacker's health) as it will
  be fought: the attackers in the strike's own order, each against the defender the rules put up
  against it; a lost fight still wounds (the rounds won before dying, first strikes counted),
  a siege engine's win leaves the defender at its combat limit, collateral damage hits the
  other defenders, siege engines that can bring the walls down in three turns count as having
  done so. The force covers the city when every defender is 70% sure to be dead (60% for the
  conquerors, `BOLD_STRIKE`); committing to *march* wants a reserve, every defender 90% sure
  (so stacks do not advance understrength), but a committed force already positioned next to
  or beside the city *strikes at acceptable odds* -- it expects to clear the defenders without
  keeping that reserve (round 7h) -- rather than retreating to muster an overwhelming force
  against a target that only hardens while it waits (a lone unit still never
  qualifies: it cannot clear a city). Unseen cities: 2 + population/5 defenders for a civ,
  1 + age/25 for a barbarian town -- but the force needed to *approach and reveal* an unseen
  target is capped at a scouting-sized stack (not that growing assumption, which otherwise made
  a hardening barb town need an ever-larger force so the army mustered forever and never marched
  to look). Once the stack reaches the assembly ring and the real defenders show, the strike /
  step-back decision above applies. Measured (seeds 1-8, turn 300): barb towns taken 5 -> 8 with
  losses to barbarians flat (8 -> 9) and the economy unchanged.
- *Commitment*: at war every stack on the landmass within 24 plots; against barbarians the
  nearest stacks until they hold twice the force needed (plus those already on their way).
- *Phases*: **Gather** (the force falls short, or the war is not declared yet): at war the
  stacks wait in the rally city (ours nearest the target); against a barbarian town they stay
  where they are, hunting raiders and holding our cities (gathering them in one city bared the
  others); a stack next to the city steps back, so single units no longer camp next to towns
  they cannot take. **March**: to an assembly plot two plots out (one that sees the city, then
  the best defense), and wait there. **StepIn**: the force within two plots covers the city:
  the stacks on the assembly ring step next to it (those still on their way come to the ring
  first). **Strike**: the force next to it covers the city: every committed stack there
  attacks in the same turn -- siege engines bombard, collateral attackers first while several
  defenders stand, then units summoned for good (skeletons and the like, `magic::raised`: free
  to replace, they wear the defenders down as siege fodder; the owner's call, 2026-10-20), then
  the best odds when 68%+, else the unit cheapest to lose for the damage it is expected to deal
  (C++ `AI_getBestGroupSacrifice` weighs strength; round 7f), never one that would
  deal under a tenth of a defender's hit points -- while the attackers left still cover the
  defenders left (or the defenders are down to half their health and outnumbered). Up to three
  units follow the victor into the city. The expected damage of a lost fight, in the strike's
  simulation and its sacrifices, counts first strikes (`fight`, round 7f): in the rounds a
  side has first strikes left it cannot be hurt. Heroes and veterans (switch `PRECIOUS`; More
  Naval AI's `AI_getBestGroupAttacker`) below their odds -- a hero's 85% plus 1%
  a level, a unit above level 4 at 70% plus 3% a level (at most 95%) -- attack last in a strike:
  while another unit can attack, and only in a strike expected to take the city, never to wear
  the defenders down; they count in the force that takes it. (Held out of strikes altogether,
  they turned captures into stalls: "Forks of the hero gate" below.) A group assault
  (`military::assault`) stops when the rules would send one below its odds, and a stack does not
  pick such a target, adjacent or hunted.
- *A second operation* (switch `SECOND_OP`; More Naval AI's `AI_pickTargetCity` passes over a
  city already targeted by more than max(6, 3 x its defenders) units): when the stacks committed
  at war hold more than twice what the first target (seen) needs, it keeps the stacks already
  ordered against it and the nearest others up to twice its need (six units at least), and the
  rest go for a second enemy city on the landmass that they cover on their own with the march
  reserve, the best value for its cost (`campaign::split`). The assignment sticks through the
  stacks' mission AI: a stack ordered against the second city stays with it while the second
  force still takes it at acceptable odds and the first keeps its need; otherwise everyone goes
  back to the first. (A first version rebuilt the split each turn from distances; the targets
  swapped every turn as cities fell or needs moved, and stacks walked between them.)
- Campaign moves go one turn at a time (the rest of the path is dropped once the moves are
  spent): an AI group on a multi-turn mission gets no orders until it ends, and stacks walked on
  to plots chosen for an earlier phase.
- Stacks past gathering do not turn back for our cities in peril (the threat usually comes
  from the target; turning back halved the cities taken at war, measured).

Measured on seeds 1-96 (6 civs, standard, 300 turns) against the stack-at-a-time `war_move`
(baseline -> now): conquests of civ cities (game log) 31 -> 73, per war declared (`ffh stats`,
which also counts cities traded or flipped) 0.99 -> 1.85; conquests of barbarian towns 99 ->
199 (41 of them size-1 towns razed on capture, so `captured_from_barb` rises only 135 -> 172);
civ cities captured or razed by barbarians 221 -> 217 (`captured_by_barb`, which leaves out the
razed ones, 147 -> 165: fewer of the lost cities were size-1 towns); barbarian cities at turn
300 6.45 -> 6.24; lone units next to a city of 3+ defenders (seeds 1-12, `FFH_AI_TRACE` "next to"
lines) 43 -> 0; cities, population, techs and score unchanged; same cost per turn.
`CvGame::createBarbarianCities` founds a barbarian city, in the fog, whenever an area holds
fewer than its unowned tiles / `iUnownedTilesPerBarbarianCity`, and at turn 300 about half of
them stand within 6 plots of a civ city with 6 units inside: out of reach of the armies the
civs field. `FFH_AI_TRACE` prints each turn's operation ("campaign vs ..."), every stack next
to its target ("next to ...", with our army units around the city) and each strike ("strike
on ...").

**Assault estimate**: for each attacker, the rules' combat odds against the defender it would
face; the stack's expected "kills" are the sum of the odds plus a third of the rest (a lost
fight still wounds) plus collateral. Attack when the best odds are 65%+, or when the expected
kills cover the defenders (plus a margin for cities) with decent odds or a numbers advantage.
`assault` then attacks repeatedly through `MISSION_MOVE_TO` with `MOVE_DIRECT_ATTACK` until the
plot falls or the odds turn bad, and moves in.

## Naval (`naval.rs`)

- **Overseas sites**: when no site is left on a landmass where we have cities, `sites.rs` also
  scores the coastal plots of other landmasses within 14 plots of our coastal cities.
- **Settling by sea**: a settler with no site it can walk to goes to the nearest coastal city
  and boards a transport there (`loadUnit`, its escort too when there is room); production
  builds the transport (`UNITAI_SETTLER_SEA`; Trireme, Caravel, Galleon...) in that port.
  The transport floods the sea (25 turns), sails next to the best site it can reach and
  moves the cargo onto it (what C++ `groupAmphibMove` orders); the settler founds on its next
  turn. An empty transport sails to a port where a settler waits, else waits in port; cargo
  it has no plan for is unloaded in port.
- **A ship nobody can build**: the settler's wait is its group's `MISSIONAI_LOAD_SETTLER` (as
  in C++). While one waits and no port of ours can train a transport (`city_can_train`: the
  Trireme also needs Bronze Working and copper), research takes the nearest step toward the
  transport a port could train soonest (Optics for the Caravel without copper) next, at
  siege's score (`research::transport_step_tech`). Before, research counted Sailing alone as
  having a transport, and settlers waited in port up to 159 turns. Seeds 1-16, 300 turns:
  settler-turns in port with no transport possible 395 -> 188, settlers that sailed 11 -> 16.
  Fresh seeds 801-944 at turn 300: overseas cities +62% (t 3.4), legacy score +0.9% (t 2.3),
  techs +1.6% (t 2.9), power +2.5% (t 2.6), with the leading civ 1.3% weaker (t -2.4): the
  stuck civs catch up. Full games (seeds 901-948, 48 games): no significant change (supremacy
  wins 9 -> 6 on 901-916, 14 -> 14 on 917-948).
- **Sea explorers** (`UNITAI_EXPLORE_SEA`, one at a time) reveal coasts while our land is full
  and nothing overseas is known; when nothing is left to reveal, one that can carry becomes a
  transport.
- **Work boats** improve sea resources in our borders (one per unimproved resource, at most
  one per city at a time).
- **Amphibious invasion** (`amphibious_target`, `UNITAI_ASSAULT_SEA`): a civ can ship an army
  across water to take an enemy city on a landmass where it has no city (the siege campaigns
  only target landmasses we already hold a city on). The target is the best overseas enemy (or
  barbarian) city -- revealed, coastal, on a landmass with no city of ours, within 24 plots of
  one of our coastal cities, whose weakest defender our best trainable attacker can beat
  (`can_take_city`) -- by value (population, wonders) for its cost (force needed, distance);
  deterministic ordering, recomputed each turn. *Production* (the coastal city nearest the
  target, the staging port): a fleet of assault transports big enough to carry the whole force in
  one wave (`ceil(need / cargo per ship)` of them -- one transport only trickles a few units per
  trip, which rarely masses enough to take a defended city), then attackers up to what one assault
  needs (`need` = defenders x2 + 1, capped). *Execution* reuses the overseas-settling
  machinery: the nearest army stacks (up to `need`) walk to the staging port and board the
  transport there (as a settler and its escort board); the transport, carrying an army rather
  than a settler, sails to a water plot beside the target city and lands the army on a land plot
  next to it; the landed stack then takes the city with `military::assault` (which holds rather
  than throwing the stack away if the revealed defenders are too strong). Every trip lands on the
  *same* plot -- the reachable land plot beside the city already holding the most of our army (then
  the soonest-reachable, then the strongest ground) -- so a trickle of small landings piles onto
  one plot and merges (`military::army` merges army units sharing a plot) into a stack that can
  strike, instead of scattering across the city's approaches and never massing. One transport trip
  still carries only the ship's cargo capacity, so with early, small transports the force builds up
  over several trips; bigger invasions wait for better ships.
- **The assault fleet** (`naval::fleet_waits`, switch `FLEET`; More Naval AI's
  `AI_assaultSeaMove`, which waits in port until the cargo reaches the invasion size and sails
  escorted): a transport with an army aboard waits in port while what the transports there carry,
  with what has already landed next to the target, falls short of the force the target needs;
  then the loaded transports in port and our idle warships there sail as one stack and land
  together (the escorts sail with them; back in port, empty, the ships part again). Before, each
  transport sailed as soon as one fighter was aboard -- a Trireme or a Caravel carries one -- and
  the army landed a unit at a time next to a defended city, where the landed units assaulted it
  piecemeal.

## Diplomacy (`diplomacy.rs`)

Relations come from alignment, state religion, war history, open borders and the Trust spell
(+2 towards a player who has cast it: the spell only sets a feat, and FfH's C++ AI read it as
+4 attitude, `AI_getTrustAttitude`; before 2026-10-23 nothing read it, so Trust did nothing in
this game, docs/ENTITIES.md). One war at a
time; war is **greedy expansion**, not a dice roll: when free to start one (checked every 5
turns from turn 50, with 2+ cities), a civ prepares war on the met, land-reachable (within 16
plots) rival whose weakest known city its best trainable attacker can take (siege engines
lower the bar, `can_take_cities`), when our power is a sufficient edge over their defensive
power. The territory is the point -- domination and conquest are this rule run to its
conclusion -- so there is no grudge requirement and no roll. Flavour biases the bar, not the
drive: the leader's `iMaxWarRand` (low = war-eager) sets the required power edge, from near
parity (an eager warmonger accepts a slight underdog as acceptable risk) up to clear
superiority for a cautious leader, who also spares civs it likes; the greediest leaders strike
any profitable, reachable target. A broke empire wants a bigger edge; a bankrupt one does not
start wars. (This replaces the earlier port of `AI_doWar`'s three odds-gated kinds, which left
strong civs idle for want of a grudge or a lucky roll.) Reachable means by land (a shared
landmass within 16 plots) or, failing that, by sea: a civ whose takeable city is within
sea-reach of one of our coastal cities (`naval::amphibious_city`) is a valid target too, so an
isolated or across-the-water rival can now be invaded (see Naval / amphibious invasion). Among
the targets we can reach and take, a flavour nudge prefers one of the opposed alignment (good vs
evil, the core FFH conflict): a small bonus added to the target score, after the reachability
gate, so it only re-ranks viable targets and never sends a civ at a rival it cannot reach. In an
8-game A/B it lifted good-vs-evil wars from 35% to 59% of a civ's wars where opposed rivals were
reachable (inert where they are across the water), with captures and economy slightly up, not
traded -- a secondary bias that does not cost the primary. The
war plan starts as `WARPLAN_PREPARING_*` (production builds the army,
stacks gather on the rally city) and is declared when the army is ready and two thirds of it
stands within 3 plots of our city nearest to the target (or after 40 turns): an expert declares
with the army at the border, so the war's first turns (and the ten before peace is possible) are
spent fighting, not marching. Before (2026-10-21) the declaration came while the army was still
at home: in 24 full games 22% of our declared wars saw no combat, and a war sold for tribute at
turn 11 often had its army still on the road (traced: 19 plots, never arrived). Measured
(`FFH_AI_WAR_STAGING_OFF=ALL` against on), full games: everyone, seeds 1-24, wars 448 -> 312,
cities taken 423 -> 397 (a city per war 0.94 -> 1.27); fresh seeds 9701-9724, wars 409 -> 236
(t -4.4), cities taken 358 -> 307 (0.88 -> 1.30 a war), games decided before the time limit 8 ->
11 of 24, population +4.3 and research +9 at turn 450 (t 1.8, 1.2); one civilization against the
rest, seeds 1-6 (114 pairs) wins 22.8% -> 22.8%, share@end +22, fresh seeds 9601-9612 (228
pairs) wins 14.5% -> 14.0% (8 / 9), share@end +15 (t 0.4). Not selling a war for tribute before
it has taken a city or lasted 30 turns cost wins (docs/AI_LOG.md, "Wars").
Peace is sought when the war goes badly (war success and power), or drags on without gains;
it is offered as a real peace-treaty deal the other AI must accept. Every 5 turns an AI swaps
technologies one for one at similar cost (once `TECH_TRADE` allows it) and signs open borders
with civs it likes; `consider_offer` accepts tech swaps worth at least 80% of what it gives,
never with the civ it prepares a war against, and both obey the tech-trade discipline below.
Peace weighs the forces at the front, quiet wars end, a target's other wars count toward our
edge, and a prepared war is re-checked before it is declared (the paragraphs after "Hire a war
ally").

**Defensive pacts (`pacts.rs`).** A weaker civ's check against a stronger, warlike neighbour.
A civ is threatened when a team within 16 plots by land has at least 120% of its defensive power
and is warlike (at war with a civ now, or fought one in the last 50 turns) -- only what a human
could see. On the trade cadence, at peace, a threatened civ that is not preparing a war of its own
offers a pact to the AI leader it does not dislike that the same neighbour threatens, else the
one it likes best, else the strongest (at most 2 pacts a team). `consider_offer` accepts from a
civ it does not dislike when it is threatened itself or likes the proposer (relation 2+), never
when it is about to attack the proposer or the proposer is its victory rival. The rules do the
rest: the pact counts in `team_get_defensive_power`, which every war gate divides by, and an
attacked member's partners declare war on the attacker (`WARPLAN_DOGPILE`). Pacts need Honor on
either side and both teams at peace with everyone, and a member's own declaration cancels its
pacts, so they appear only from about turn 190 (dev seeds 1-8, full games: 0-5 a game).
Measured (full games, seeds 1301-1364, 64 games, against the tip): 23 games change, nearly all
after turn 300 (300 turns, seeds 1101-1244: 140 of 144 games identical); at turn 450 legacy
+0.7%, techs +1.4%, power +1.0% (t 1.4-1.6); outcomes supremacy 35 -> 31, cultural 17 -> 20, time
limit 12 -> 13; eliminations in the changed games 42 -> 42. Mechanically working and harmless,
but not yet a measurable protection: the gating (late Honor, peace with everyone) keeps most
threatened civs from signing. Levers if it should bite: research Honor when threatened, or a
rules change to the trading prerequisite.

**Vassal states (`vassals.rs`).** The owner's decision, 2026-10-21 (CLAUDE.md, "AI changes"):
what an expert does once the game has gone against it. The rules (Feudalism for the master; a
vassal joins its master's wars and the master its vassal's, a capitulation makes peace) were
ported; the judgement is here. Only what a human could see is read.

- **Hopelessly behind** a neighbour (cities within 14 plots): the neighbour has 3x our power, knows
  6 more technologies and 40% more than we do, and our share of the world is under half an
  average civilization's. Then we offer to become its vassal, if the two are at least decently
  aligned: no good/evil opposition, and an attitude at or above the leader's
  `iVassalRefuseAttitudeThreshold`. That threshold is one step more willing than FfH's AI, which
  refuses at it, so pride still varies by leader. We never pick a neighbour whose wars would set
  us against a stronger civ next door. Checked every 5 turns; the strongest such neighbour is
  asked first.
- **Crushed in a war:** at war for 10 turns, the enemy has twice our power, and its war success
  exceeds ours by three captured cities' worth (two when we have three cities or fewer left).
  Then we capitulate to the enemy doing the most damage, whatever we think of it. This comes
  after the peace offer that turn failed.
- **Asked** to become someone's vassal (by a human or an AI): we agree when the same tests hold.
- **As the master:** a vassal is taken unless its wars would drag us into one against a stronger
  civ, a friend or a treaty partner. A capitulation is taken unless the loser has two cities or
  fewer and we can press the conquest; then we take the cities instead.
- **Humans:** asked on the leader's contact cadence. A human can't be an AI's vassal (rules).

Measured with `FFH_AI_VASSAL_OFF` (the switch), full games of ffh-ai only. The switch is a
civilization switch like the others (`switches::VASSAL`): `FFH_AI_VASSAL_OFF=ALL` (or `1`, as
before) turns vassal play off for everyone, `FFH_AI_VASSAL_OFF=X,..` / `FFH_AI_VASSAL_ONLY=X,..`
for the players of those civilizations, each deciding for itself (it neither seeks nor agrees to
vassalage, and as the master takes a vassal only when the deal costs it nothing).

| | Development seeds 1-12 | | Fresh seeds 201-224 | |
|---|---|---|---|---|
| | off | on | off | on |
| Vassalages (capitulations / peaceful) | – | 15 / 3 | – | 22 / 3 |
| Civs eliminated | 18 | 11 | 38 | 30 |
| Supremacy | 8 | 7 | 12 | 16 |
| Cultural victory | 3 | 1 | 10 | 5 |
| Conquest victory | 0 | 1 | 0 | 2 |
| Time limit | 1 | 3 | 2 | 1 |
| Average decision turn | 519 | 492 | 496 | 455 |

Capitulations fall between turns 290 and 625. On the fresh seeds, 260 capitulation offers from
civs with two cities or fewer were refused (the conqueror took them).

Fewer civilizations die. Games are decided sooner and more often by supremacy or conquest, less
often by culture. The mechanism: score v2 credits a master with half its vassals' people and land
(docs/SCORE.md), and conquest counts vassals as conquered. So a leader with vassals reaches clear
supremacy before the culture race ends, as Civ4's vassal empires snowball. This is a sound
behaviour, kept under the expert-play rule; the shift in outcomes is the game's balance, worth
the owner's eye.

Peaceful vassalage stays rare. About one civ per game meets the three "hopeless" tests against
the strongest civ after turn 300 (12 development games: 13 of 219 civ-snapshots), and the
technology gap is the binding test. A vassal never leaves its master on its own initiative, even
after it recovers. One vassal was released in the 24 fresh games; telemetry records releases as
`vassal.freed`.

**Resources and gold (`trade.rs`).** The AI also trades resources (bonuses) and gold -- a port of
the base game's `CvPlayerAI` trade valuation (`AI_bonusTradeVal`, `AI_bonusTrade`, `AI_dealVal`,
`AI_goldTradeValuePercent`/`AI_goldPerTurnTradeVal`, `AI_maxGold(PerTurn)Trade`). The value of
*holding* a resource is ffh-core's already-ported `player_ai_bonus_val` (happiness/health,
unit/building enablement, the FfH mana and evil-resource terms); `trade.rs` is the trade layer on
top: a resource's trade value (scaled by the smaller of the two empires, the data
`AITradeModifier`, over a treaty's length), a deal's total gold-equivalent value (`deal_val`, techs
via the research-cost proxy since `AI_techTradeVal` is unported), gold valuation (worth more when
broke) and the gold caps (a research buffer plus an already-traded-to limit), and the give-refusal
(`bonus_trade`: never to our worst enemy, nor a resource they already have, nor a strategic /
happiness / health resource to a civ we do not like enough -- one step more generous toward a civ
of our alignment or favourite religion). `consider_offer` values any resource/gold/mixed deal
(accept when what we receive is worth at least what we give, no gold-for-gold, never a resource the
refusal blocks); `trade()` (AI<->AI) and `offers.rs` (AI->human) buy the resource we most want and
a neighbour can spare, paying with a surplus of ours or with gold, only at peace with civs we like.
No corporations exist in FfH 0.41, so that value is 0 and its branches are dead; `AI_getAttitude`
and `AI_isFinancialTrouble` reuse this AI's `attitude`/`production::financial_trouble` proxies
(docs/PORTING.md). It adds no saved state -- deal memory rides ported `Game` fields updated by
`implement_deal` -- and draws no RNG (deterministic, checksum-stable).

Measured (16-seed, 690-turn A/B vs the no-trade tip): trading is positive-sum, so every civ grows
-- final power +8.7%, research at turn 300 +3.4% (the hard guard), and 7% fewer civs eliminated
(weaker civs trade to stay viable). The cost is less *concentrated* domination -- biggest empire
-4.5%, top-power share -9.4%, and games run longer (6 vs 3 reaching turn 690) -- but the leader's
absolute power is about flat (it is the field catching up, not the leader weakening), and research
at turn 450 slips -3.1%. Kept as faithful, more human-like play that grows total power and mid-game
economy at the cost of some domination concentration.

**Peace with reparations (`diplomacy::end_war_val`, `trade::peace_tribute`).** A port of
`CvTeamAI::AI_endWarVal` and the `CvPlayerAI::AI_doDiplo` peace construction: ending a war carries a
value (large when weak/losing, cut to a fifth once we dominate at >120% power -- the FfH
divide-by-our-power variant, plus the land-ratio cut and the war-plan ×2/×1.5 multiplier; the
DAGGER/financial/warRand warmonger sub-factors are omitted as unported). The gap between the two
sides' `end_war_val` funds reparations: when clearly winning, an AI now *initiates* peace and demands
tribute (the costliest tech the loser can give, then gold) instead of a valueless white peace, and a
losing AI *sweetens* its peace offer with tribute of its own so the winner will accept -- converting
military success into economy. A civ pursuing conquest/domination against that enemy presses on
rather than cashing out (victory-pursuit override in both the offer and `consider_offer`), and so does
a civ *overwhelmingly* winning any war (`press_conquest`: our power >= 200% of the enemy's defensive
power) -- it takes cities instead of selling the war for a token tribute. The latter guard keys on the
war's power edge, not the civ's victory goal, because an adversarial check of the first A/B found
strong civs (2-5x power) tribute-farming weak victims for whole games instead of finishing them -- the
primary (conquest) traded for a secondary tech. Below 2x the enemy's cities are not reliably stormable,
so a tribute peace is a sensible outcome and the guard does not fire. The old
willingness (take a bare peace unless clearly winning, and always past a 60-turn exhaustion floor)
stays as the fallback, so even/stalemated wars still resolve and research is not starved.
**Deliberate deviation from the C++ quirk:** `AI_doDiplo` adds tribute gold at *face* value because
the proposing AI implements the deal unilaterally; here AI-AI deals are judged by the *recipient's*
`consider_offer`, whose `deal_val` scales gold by `gold_trade_value_percent` (200-300%), so face-value
gold would read as overpriced and always be refused. `peace_tribute` instead builds the gold that is
*worth* the gap under `deal_val` (the `AI_counterPropose` scaling), so the deal balances for the
judge. Cities-as-tribute are omitted (`AI_cityTradeVal` unported). No new saved state; no RNG.

Measured (16-seed, 690-turn A/B vs the pre-reparations tip, with the `press_conquest` guard):
captured cities +16%, civs eliminated +25%, the leader's world-share at turn 690 35.9% -> 42.5%
(x1.20 -> x1.40 over the runner-up) -- a markedly stronger march toward domination, the primary
goal. War declarations are *down* 10% and decision turns slightly faster, so pressing conquest
resolves wars rather than dragging them. Research at turn 300 is flat within noise (-1.7%, lower in
10 of 16 seeds / higher in 4, per-seed swings +-46), i.e. the economic engine is not sacrificed.
An adversarial trace audit of the first (un-guarded) cut caught strong civs tribute-farming weak
victims for whole games instead of conquering them (primary traded for a secondary tech); the
`press_conquest` guard fixed it (worst seed: peace events 31 -> 6).

**Gift tech to a weaker ally (`give_help`).** A port of C++ `AI_doDiplo`'s give-help (FfH "Better
Diplomatics" block B): a team leader gifts a free technology to a civ markedly weaker than itself
(team assets below half ours) that it likes (attitude over the target leader's `NoGiveHelp`
threshold, eased one step by shared alignment or favourite religion), at peace, on the deterministic
`contact_due` cadence (~150 turns). Deliberate deviations from the C++: the gift is the *cheapest*
tech we can give (deterministic -- ffh-ai draws no RNG -- and so it never hands over an expensive key
enabler) rather than a random one; and we never gift a civ that is our victory rival (the project's
"do not prop up a rival" rule). The gift does not raise attitude (`relation()` does not track gifts;
C++ `AI_getTradeAttitude` is unported), matching how resource/open-borders trading was integrated.
Like the base game it is rare (the gating plus the ~150-turn cadence fire it only a handful of times
across a 16-game set), so it is a faithful, low-impact "keep an ally alive" gesture rather than a
force on the outcome. No new saved state; no RNG. (Measured 16-seed/690-turn A/B vs the
peace-with-reparations tip: flat within noise -- research at turn 300 +1.2%, leader world-share at
turn 690 42.5% -> 42.8%, war/peace/elimination counts unchanged -- with research at turn 690 +11% and
one domination win, i.e. harmless-to-slightly-helpful, as expected from a feature that fires only a
handful of times.)

**Hire a war ally (`hire_war_ally`, `declare_war_trade_val`).** A port of C++ `AI_doDiplo`'s paid
war-bribe (`TRADE_WAR`): a civ already at war pays a third AI, at peace with it, tech + gold to
declare war on one of its enemies -- turning gold into a second front. The price is a faithful port
of `CvTeamAI::AI_declareWarTradeVal` (the target's size, the bribee's attitude to the target and to
us, the relative powers, a dogpile bonus when the target has no other wars, scaled up as the game
ages); `consider_offer` values an incoming bribe with the same function and runs the AI half of
`AI_declareWarTrade` that ffh-core leaves open (one war at a time, the target not too strong even
with our help, and the bribee's attitude to briber and target). Deterministic deviations from the
C++: the target is the enemy the bribee most dislikes (`AI_getAttitudeWeight`) and the payment is the
costliest tech(s) the bribee would value plus a gold top-up, rather than the C++ random picks; a
staggered period from the leader's `DeclareWarTradeRand` replaces the 1-in-N roll; and we never pay a
civ that is our victory rival (do not strengthen a would-be winner). `AI_isAllyLandTarget` is unported
(assumed false); nukes are 0 in FfH 0.41. No new saved state; no RNG.

Measured (16-seed/690-turn A/B vs the gift tip, and an adversarial audit of briber bankruptcy and
bribee prop-up): research at turn 300 flat (-0.9%, hard guard PASS -- it moves on only 2 seeds); war
declarations flat (285 -> 286, so bribes organise dogpiles onto existing wars rather than spawning new
ones); and more games resolve with a genuine victory before the turn cap (8/16 -> 11/16, domination
wins 1 -> 2). The audit refuted the two risks: the winning team is unchanged in 15/16 seeds and the
most-bribed civ never snowballs (bribe + tribute income is too small to move outcomes -- leader
world-share barely moves, 42.8% -> 43.7%), and bribers cannot bankrupt themselves (`max_gold_trade`
caps the gold under treasury with a deficit buffer, payment is tech-first, and an offer is made only
when the payment is worth over three quarters of the price) -- in practice the bribers are the
*winners*, converting gold into a decisive second front (seed 5: the dogpile cuts the target from 13
cities to 2 and the briber-side winner closes out by domination 64 turns early). No new saved state;
no RNG.

**Peace by the forces at the front (`end_war_val_seen`, `front_forces`; switch `PEACE_FORCES`).**
More Naval AI divides the value of peace by our attackers en route against theirs in our land
(`CvTeamAI::AI_endWarVal`, x200 / range(ratio, 200, 600)). Here the same scale applies to
`end_war_val` wherever this AI weighs peace (its own offers, `consider_offer`, the offers to
humans, its council votes): our units able to attack within 2 plots of their city where we have
the most (their next city) against theirs within 2 plots of ours that we can see, at least 2.
Nothing at the front: x1; four times their number: x1/2; six times or more: x1/3. A winner's
cash-out (peace for tribute) then goes ahead only when the tribute pays for what the army at
their gates is worth to us (the drop in our want for peace; nothing when no army is near), and a
bare peace offer is refused while our army stands there with three times theirs. That is the
expert's test the 30-turn rule lacked (docs/AI_LOG.md, "Wars"): sell the war when the next city
is out of reach, fight on when it is close.

**Quiet wars end (`quiet_war`; switch `QUIET_PEACE`).** More Naval AI makes peace in a war of
10+ turns where nothing happens and no stack is moving (`AI_doWar`). Here: past 15 turns, less
than two attacks' worth of war success on both sides, no army of ours near their cities nor of
theirs (that we see) near ours, and no stack of ours marching on them (the campaign's
`MISSIONAI_ASSAULT` tag) -- we offer peace, with tribute either way as the war values say.

**Dogpiles (`dogpile_power`; switch `DOGPILE`).** In the war-of-expansion gate the power of a
target's current enemies counts with ours (More Naval AI's `AI_doWar` dogpile power; an enemy that
shares no landmass with it counts half): a civ busy on another front is a weaker target than its
defensive power says, and the runaway leader, at war with a neighbour, becomes reachable for the
civs around it. The rest of the gate (reachable, takeable, the leader's bar) is unchanged, and the
declaration's re-check (below) drops the war if the other front has closed since.

**Re-check before declaring (`still_a_target`; switch `WAR_RECHECK`).** More Naval AI's "last
chance to change our mind" (`AI_doWar`): when a prepared war is about to be declared (the army
staged, or the 40-turn fallback), the target must still pass a gate that would choose it now --
the victory plan's, or the war of expansion's with 10% slack on the power bar and per-unit parity
for takeability (the army is massed at the border; the 120% margin selects wars, it does not
re-judge them). A target with new pacts behind it, a friend since, or cities our best attacker no
longer matches is dropped, the army kept for the next choice (`war_drop` decisions: power,
not_takeable, unreachable).

**Tech-trade discipline (`tech_denial`, `TechCtx`; switch `TECH_DISCIPLINE`).** More Naval AI's
`CvTeamAI::AI_techTrade`, in the form an expert would keep: no technology to our victory rival; a
swap with the world's clear leader (a share 1.5x the runner-up's) only when we get at least 125% of
what we give (by research cost; 80% for anyone else, as before); no military technology (one
opening a unit of the recipient's civilization at 60% of the world's best land unit) to anyone
while we prepare or fight a war (it can be passed on); and, to anyone we are not friendly with,
no monopoly worth keeping: the technology of a world wonder, world unit or world project one of
our cities is building (More Naval AI's DENIAL_MYSTERY), or a military technology a land
neighbour (within 16 plots) would gain while most civs we have met lack it. `trade()` asks only for
a technology the partner's own discipline lets go (the trade screen shows a refusal) and offers
the closest-in-cost one ours lets go; `consider_offer` and the offers to humans apply the same
rules. A first version kept every technology worth More Naval AI's monopoly value of 50 (an
unbuilt wonder, strong units) while most civs lacked it, and refused the clear leader outright:
1,000 swaps withheld in 24 full games, the world 4.5% behind in technologies at turn 300, and one
civilization with it against the rest won 18.4% of its games against 21.1% (seeds 1-4, 76 pairs):
a civ that refuses a swap forgoes the technology it would have received while the others trade
on.

**Councils (`council.rs`; switch `COUNCIL`).** The Overcouncil (the Overcouncil civic, good and
neutral civs) and Undercouncil (evil and neutral) meet every 10 and 15 turns once two civs run
their civic; members elect a head councilor, who proposes one resolution a session, and half the
population-weighted votes pass it. Before, every AI abstained and never proposed, so both were
inert. Now a member votes by what the resolution does to it (`council::value`): a war declared on
our enemy, or on a civ our own gate would attack, is welcome, one on us or a neighbour we would
not choose is not; a forced peace is judged like the peace it makes (`want_peace`, the war
values), stopping a winning front-runner and sparing a losing friend; an embargo against an
enemy or a disliked civ; a mana ban hurts those who hold the mana and helps against the
enemies who do; a forced civic against our own civic values; trade routes (More Naval AI: a
large empire gains little), free trade, free units we can pay for, open borders and pacts by who
they bind us to; Fund Dissidents against the Overcouncil members we fight or dislike; crime
counts against. Positive: yes; negative: no; nothing at stake: abstain. A resolution worth -300
or less (war on us, one of our cities given away) is defied (the rules' veto: unrest, and the
loyal membership lost). For head councilor we vote for ourselves when we stand, else our master,
else the candidate we like best that is neither our victory rival nor the clear leader. The head
councilor proposes the resolution that gains it most among those its model of the members' votes
(our AI's members by the same values, humans and other AIs abstaining) says will pass -- or, for
one in force that hurts it, will fail and so be repealed; nothing when none does. Deterministic:
none of the C++ AI's defiance or proposal dice. Telemetry counts the results (`vote_passed`,
`vote_failed`) and the AI's `vote` and `election` decisions.

**Missionaries abroad (`units::missionary`, `production.rs`; switch `MISSIONARY_ABROAD`).** More
Naval AI's missionary strategy: once the holy city of our state religion is ours and has its
shrine (a gold a turn for every city in the world with the religion), a missionary with no city of
ours to convert goes to a foreign city it may enter -- a civ at peace whose borders are open to us,
whose civics let the religion in -- cities without a religion first (the spread is then almost
sure), within 15 turns. Production weighs one such missionary like a building worth the shrine's
gold (`MISSIONARY_ABROAD_VALUE`, 60 of `building_value`'s 80 a gold, for the odds of failing on
the way) against the best building, while such a city lies within 15 plots on the landmass. It
almost never fires: the shrine comes late (after turn 300, a Great Prophet's work), open borders
are rare by then, and a building nearly always outscores a gold a turn.

## Random events (`events.rs`)

The rules offer an AI player the trigger's events it can do, in trigger order (`player_trigger_data`).
Until `choose_event` was implemented the controller default took the first, and FfH lists the
decline / pay / lose-a-unit answer first in many triggers: War Games disbanded the unit, the Strange
Advisor added inflation for good, Clairone's first answer turned down a free Great Artist, the Spider
Mine's third released a Giant Spider, and every Adaptive leader (Varn, Cardith, Cassiel) became
Aggressive every 100 turns. About 78 events fire per 300-turn game, about 25 of them with a real
choice (seeds 1-8).

`events::value` follows C++ `AI_eventValue` term by term in one unit (gold) instead of the C++ mix
of gold, hammers x100 and per-turn yields x20: gold, tech progress, free units (great people 600)
and buildings at twice their hammers, a disbanded unit and its experience, golden ages, building
yield/commerce changes in the cities that have the building, happiness and health only where a city
is at or near its cap, food, population, revolt and hurry anger, plot yields, improvements, routes
and resources, experience and free promotions, unit support, inflation on the pre-inflation costs,
the Armageddon counter by alignment, religion conversions, attitude, follow-up and cleared events.
Permanent changes count 50 turns. No random draws: `rand(iAIValue)` is its mean, the final +0-20% is
dropped. Python callback effects are 0 as in C++, except a short table of frequent clear ones
(hostile spawns, the Iron Orb's free tech, a seceding city) and the Adaptive trait choice, valued
from the trait's data (`trait_value`: extra yield at the threshold on worked plots, civic upkeep,
commerce per city, capped happiness and health, great people rate, wonder production, free
promotions, no anarchy). An event that would declare a war we are not in is never chosen.

Measured (6 civs, standard map, 300 turns, seeds 801-944, 144 paired games against the first-option
default; t is the paired t over seeds): at turn 300 legacy score +1.2% (t 1.0), techs +1.4% (t 1.0),
research +2.0% (t 0.8), production +2.5% (t 1.4), research slider +2.9% (t 1.5; +3.8%, t 2.4 at turn
200), culture +8.2% (t 1.8); captures, eliminations and wars unchanged. A small lever (a few real
choices per civ in 300 turns), economy-positive throughout and no regression.

## Spellcasting (`magic.rs`)

FfH's own AI casts by fixed priorities: at the end of every turn the best spell by
`CvUnit::chooseSpell`'s weights, then a buff, then a summon when an enemy is in reach
(`casting.rs`, ported term by term), and before every group attack its damage spells, summons and
buffs (`CvSelectionGroup::groupAttack`, which the rules port leaves to the AI). The base AI had only
the first: damage and summons fell only when an enemy happened to be in reach at the end of the
turn, casters spent their turns on Haste and Regeneration, and no AI built a Mage Guild
(docs/AI_LOG.md, "Casters through a Mage Guild"). `magic.rs` values a spell by what it does to the
units it reaches, read from the spell data, and uses that value before attacks, at the end of the
turn, to move casters and to promote them:

- **Effects**, in hit points: damage by the rules' formula (the caster's spell damage bonus, the
  target's resistance -- twice its defense, twice its level, a quarter of its city's defense, its
  resistance to the damage type -- up to the spell's limit, and the resist roll of resistable
  spells); a promotion given or taken as the share of the unit's health its strength change is worth
  (Enchanted Blade's +20% on an attacker, Rust's lost weapons on a defender, a charmed or slowed
  attacker that cannot strike); immobility and dispels; a summon by an expected fight of its type
  against the defender the rules put up, with its collateral damage; Python spells by what their
  Python does (Crush, Pillar of Fire, Destroy Undead, Domination, Heal, Earthquake, Spring, Scorch,
  Vitalize). Our own units hit count double against; hitting a third party rules a spell out (spell
  damage on a neutral's land declares war, C++ `doDamage`).
- **Before an attack** (`campaign::strike`, `military::assault`): every caster within two plots of
  the target casts, the best cast of all first and every value recomputed after each: what weakens
  the defenders, then summons, which attack the target at once (summon-suicide mode, as FfH's
  end-of-turn casting); the strike then weighs the defenders as they are left, and blessings on the
  units about to attack come only once it goes ahead (Dance of Blades wears off by our next turn).
  A unit casts once a turn, so a caster casts here only what is worth more than its best cast at
  the end of the turn (the Balseraphs' Loki keeps his for the puppet FfH's weight asks for).
- **At the end of the turn** (`casting::end_turn`): damage, curses and summons on the enemies that
  can reach us (summons attack as FfH's do, `casting::summons_attack`: of the enemies next to the
  plots they reach with moves left this turn, the one where their odds are best, at 5% or better,
  never a stack of temporary summons; a summon is worth the fight it would take, nothing when it
  would take none; one about to expire also takes worse odds, below); lasting blessings on our
  units that lack them; short ones by what
  they add to our defense in the enemies' turn, before they wear off (in full where an enemy we see
  can strike, a quarter elsewhere: Dance of Blades' first strike; Haste's move adds nothing);
  healing; terraforming; city spells. World spells, abilities (equipment, crews, cages) and
  Python-only spells keep FfH's terms (and world spells their timing, `world_spells.rs`); the
  civilizations keep their say (`civs::spell_value`). A unit for which nothing is worth casting by
  these values casts what FfH's own AI would (its weights, then a buff, then a summon): a free cast
  (Haste on a quiet turn), and the same game as FfH's where the values see no difference -- a cast
  is never quite a no-op (it rolls for a miscast and ends promotions casting removes), and skipping
  them split games for nothing (docs/AI_LOG.md, "Spellcasting").
- **Movement** (`units::caster`): a caster that knows a spell for a fight marches with the stack of
  our operation nearest its target (in the rally city while the force for a war gathers), so it
  stands next to the city when the strike comes; the others go where their lasting blessings, city
  spells or terraforming are worth most for the turns it takes. A city spell's building goes when no
  unit in the city can cast it (`bRequiresCaster`, C++ `CvCity::doTurn`): the caster that keeps one
  stays unless twice as much waits elsewhere (without this rule casters recast Inspiration a hundred
  times a game). Adepts and better move by their role in the corps (below): the node builder,
  the terraformer and the city mages to their work, the war wizards as above.
- **The caster school** (2026-10-27): how the rules raise an arcane caster decides how to use it.
  An Adept gains experience by itself, each turn with chance (spellcaster XP - experience)%
  (bFreeXP: 20 with Channeling I, more with Channeling II and III and with Potency), not by casting
  or by where it stands; at level 4 with Sorcery it becomes a Mage (Channeling II: Fireball,
  Maelstrom, the Spectre, Pit Beast and Ice Elemental), at level 6 with Strength of Will an
  Archmage (the elementals). No Channeling I spell summons a fighter or does damage, so an Adept
  adds little at the front, and its worth is its age: lost there (strength 3, two times in three
  alone on the way), it took the 30 to 70 turns it had studied. So: an **apprentice** (an arcane
  caster without Channeling II that can still become a Mage, not a hero -- at any level: an Adept
  of level 4 casts an Adept's spells until the tech and the gold make it a Mage) works at home --
  city spells, blessings and terraforming in our land where no enemy we see can reach -- and
  otherwise waits in the city of ours it stands in, or goes to the nearest quiet one
  (`magic::study_city`; first versions sent it to the least threatened city, and apprentices
  shuttled between two cities as the danger moved and died on the open ground between); **every
  caster** moves toward its target only onto plots where it is sheltered (`magic::safe_step`: our
  city, a plot no enemy we see can reach outside an enemy's land, or beside a unit of ours at least
  as strong, not another caster), waiting at the last one when the next step is not, and goes to
  study when it can reach none (the same steps take a caster with nothing to do to the most
  threatened city); a player with an apprentice a level short of its upgrade
  researches the upgrade's tech (+300 for each such apprentice, up to five, and +100 on the techs
  on the way: `magic::school_tech_value`) and pays for the upgrade with a defender's gold reserve
  (`units::upgrade`).
- **Casters in the stacks** (`CASTER_STACKS`, 2026-10-27): a caster going to an operation
  (`magic::strike_target`) joins a committed stack of it with four fighters or more where it
  stands, before or after its sheltered step, and marches with it from then on
  (`magic::stack_to_join`, `join_stack`): the stack guards it and carries it to the city at the
  stack's pace (on its own it waited at the last sheltered plot while the stack went on). Joining
  any committed group, a third of the joins went to groups of one to three units -- stragglers and
  reinforcements on their way -- and the casters lost rose by a quarter: they shared those groups'
  fate. It joins as a war wizard, the AI type FfH's group attack
  sends below 95% odds only when no other unit can attack (`AI_getBestGroupAttacker`, Sephi's
  block), and it does not fight for the stack: below those odds the strike leaves it out, a stack's
  assault holds rather than send it (`military::precious_holds`), the campaign's force estimates do
  not count it (`campaign::fights`), and a captured city is entered by fighters first. It leaves the
  stack (`magic::leave_stack`) once the stack is out of our operations or fewer than two fighters
  are left in it (a strike takes the attackers out of their stacks, a battle kills them), and takes
  its own orders at once; a group a
  caster heads splits at the start of the turn. Inside a stack it never heads, it gets
  its promotions at the start of the turn (`units::play`, also without `PROMOTE_ALL`). A stack
  moves at its slowest unit's pace: a caster slows a mounted stack.
- **Promotions** (`units::promotion_value`): a sphere is worth the spells it opens, plus half of
  those the next promotion of the sphere opens (a quarter for an adept, which gets Channeling II only
  as a mage): Fire I for Fireball, Enchantment I for Enchanted Blade.
- **Summons spent** (`casting::summon_target`, `SUMMON_SPEND`; More Naval AI's
  `AI_summonAttackMove`, which attacks at any odds): a temporary summon that will be gone before it
  could act again (duration 1: every summon but a Summoner's on the turn it is cast, a Summoner's
  on its second turn, which now attacks before our other units move instead of wandering as an army
  unit) attacks below 5% odds too, ranked by the hit points the fight takes (5 or more, by
  `magic::summon_fight`), a 5% target still first. Two exceptions an expert sees: it stays when it
  is its stack's cover (an enemy can strike the plot next turn and the lasting units it stands with
  defend worse: the Balseraphs' Loki lost his health to a Spectre his puppet had left him alone
  with, docs/AI_LOG.md, "Spellcasting"), and at the end of our turn it leaves cities alone, whose
  defenders heal before anything of ours could use the damage (a Summoner's summon at the start of
  our turn may weaken a city our stacks then strike). Permanent summons are never spent. The
  end-of-turn value of a summon spell counts the same fights.
- **Govannon** (the Amurites' hero; `casters::govannon`, `TEACH_SPELLS`; More Naval AI's
  `AI_Govannonmove`): Teach Spellcasting gives every unit of ours on his plot (not animals or
  birds) his first-level spheres and Channeling I; its value is the spells those spheres open for
  each unit
  (`magic::teach_value`, `promotion_value`'s scale), so he casts it where he stands when that is
  worth 40, else walks alone to the stack or city of ours worth most for the turns it takes (within
  10, on our land, no enemy in sight there). FfH's own value for the spell is 0 (its effect is all
  Python), and no AI ever cast it.
- **Mana nodes** (`mana.rs`, `MANA_NODES`; More Naval AI's `AI_upgrademanaMove` and its mana
  valuation): raw mana in our cities' radius is worth nothing until a caster builds a node on it,
  and a node is for good (it cannot become another type). A node type is worth, on
  `building_value`'s scale for the whole empire, the bonus's own effects in every city, read from
  the bonus data (Mind +3% research, Law -5% maintenance, Spirit +5% great people, Enchantment +1
  happiness and Life +1 health -- again for every node, `bModifierPerBonus` --, Body healing, Earth
  discoveries), plus, for the first node of a type, three times the spells its sphere opens: half
  its first promotion's worth to each caster of ours that lacks it, in full to one we could train
  (`magic::sphere_value`), plus the affinities of our units. A caster with no strike to join and
  no city spell to keep goes to build the node worth most for the turns it takes (within 12, on a
  plot no enemy can reach next turn); production trains one caster for nodes when none of ours can
  build them and the jobs are worth 300 (the Mage Guild first, in a city with half our best city's
  production or more, when no city of ours can train an adept); research values the techs that
  open node types by the gain over the types we can already build, for each raw node of ours
  (`research::builds_value`). Before, only the Tower of Mastery goal built nodes, and it is never
  ours: no civilization converted its raw mana. In 24 full games the nodes built were two thirds
  Enchantment (a happy face in every city per node), then Entropy and Mind (round 7a).
- **The caster corps** (`casters.rs`, round 7a; More Naval AI's adept roles): each caster
  (an adept or better, not a hero) has a role kept in its unit AI type, so it is saved with the
  unit: the node builder (`UNITAI_MANA_UPGRADE`, one while there are node jobs), the terraformer
  (`UNITAI_TERRAFORMER`, one while a caster knows Spring, Scorch or Vitalize and our worked land
  has plots it would improve), city mages (`UNITAI_MAGE`, one for every four cities, each in a city
  of its own, the most threatened first, then the capital, then the largest, where they keep their
  city spells and bless and defend the garrison at the end of the turn) and war wizards
  (`UNITAI_WARWIZARD`, the rest, with the stacks as above). Production keeps the corps -- the city
  mages, and at war or preparing one a war wizard and one more for every ten attackers, five in
  all at most -- while our mana gives a fresh adept a first-level sphere worth 60 or more
  (`magic::sphere_value`: Enchanted Blade, Rust, Slow, Bless, Raise Skeleton with Spectre, Wall
  of Stone with Stoneskin, Inspiration with Charm), after the workers and before heroes, the army
  and buildings, in a safe city that can afford it; the Mage Guild where no city trains adepts.
  More Naval AI keeps a mage in every city and more; an expert's corps is smaller and pays in the
  economy first. Three earlier attempts at training casters for their fight spells alone lost
  wins (docs/AI_LOG.md, "Spellcasting", "Casters through a Mage Guild").

`FFH_AI_MAGIC_OFF=CIVILIZATION_X,..|X|all` makes those civilizations cast by FfH's terms
(`casting.rs`; their summons attack as everyone's do: the measurement switch);
the switches `CASTER_SCHOOL` and `CASTER_STACKS` (`switches.rs`: `FFH_AI_CASTER_SCHOOL_OFF`,
`FFH_AI_CASTER_STACKS_OFF`, `.._ONLY`) turn the caster school and the casters in the stacks off
(the stacks need the school; with each off the same games result as before it);
`FFH_AI_TRACE_CAST=1` prints every AI cast (turn, player, civilization, unit, spell, where,
value), every strike (our arcane casters within two plots of the city, Mages anywhere) and every
caster joining or leaving a stack. Tests:
`crates/ffh-ai/tests/magic.rs` (a mage Fireballs a city before the stack attacks; an adept
blesses its stack with Enchanted Blade, and values Haste at nothing on a quiet turn; a fresh
summon attacks a Warrior but not a Champion; a Spectre walks two plots to a wounded Warrior, and
is worth nothing next to a city it cannot hurt; an apprentice stays home, and a Mage steps only
beside our units), `magic::tests` (the worth table's order) and
`campaign::tests::a_mage_in_the_stack_does_not_attack` (the strike takes the city without the
Mage in its stack, and a stack whose only unit left to attack is the Mage holds).

Measured: each civilization forced into slot 0, its spellcasting on against the same binary with
`FFH_AI_MAGIC_OFF` for it (every other civilization casting by `magic.rs`), full games (6 civs,
standard map, to a decision or turn 690), 19 civilizations x 20 seeds (the Infernals and the
Mercurians are not forced). Fresh seeds 8201-8220, the kept version: win 17.7% -> 18.5% (11 / 8
seeds won by one side only), share@end +36 (t 1.39), legacy@end +18 (t 0.67), share@300 +0.4; 278
of 379 games are the same game on both sides (a civilization's own spellcasting changes about a
quarter of its games; the 380th, Calabim seed 8210 with their spellcasting off, hit a rules crash
fixed since: docs/PORTING.md, "A unit dies once"). By civilization the end shares run from
Calabim +357 (t 1.04) and Clan of Embers +315 (t 1.48) to Khazad -142 (t -0.96) and Balseraphs -92
(t -1.73); only Sidar's +85 has |t| above 2. Development (seeds 1-20, with the caster production
since removed): share@end +34 (t 1.27), win 19.2% -> 16.8% (9 / 18: victory races between
civilizations of equal share, on seeds that chose the civilizations' own logic in round 5). The variants that failed along the way, and
why, are in docs/AI_LOG.md ("Spellcasting"); fresh sets 8001-8020 and 8101-8120 were spent on
earlier versions.

**Summons and skeletons (2026-10-20).** Two follow-ups from the 53 traced validation games, kept
under the owner's relaxed gate for this work (magic is niche for the AIs: kept for doing what it
should, unless it clearly harms). (1) Summon reach: summons attacked only adjacent plots, so the
Spectres FfH's rule casts for an enemy two plots away never attacked (84 of 84), and the end-of-turn
value counted fights a summon would not take (13 of 15 valued Fireballs never attacked); now a
summon attacks the best target it reaches this turn and is worth only that fight. In 14 traced
games the summons that attacked rose from 10% to 14% (Fire Elementals cast by FfH's rule 19% ->
59%). Measured with the change for the forced civilization only, fresh seeds 8301-8320, the first
8 civilizations (160 pairs, 34 games changed; the run was stopped there): win 18.8% -> 18.8%
(2 / 2), share@end -1 (t -0.03), legacy@300 +9 (t 2.16), legacy@end -15 (t -0.32). The barbarians
keep the adjacent rule: with the reach (barbarians only, 40 paired games, random civilizations)
they took 3.2 -> 3.8 cities a game (t 0.9), the civilizations lost 3.95 -> 4.70 to them (t 1.1)
and their legacy score at turn 400 fell 54 (t -1.7). (2) Skeletons: a strike's units summoned for
good go right after its collateral attackers (the owner's call: siege fodder; `campaign.rs`);
strikes had skeletons ready 35 times in four traced Calabim games and used 10, now 15 and 10
skeleton attacks in two replays where the old order made 1 and 8. Calabim forced, fresh seeds
8501-8520 (7 of 20 games changed): win 10% -> 15% (1 / 0), share@end +22 (t 0.37), legacy@end -2.5
(t -0.02).

## Civilization-specific (`civs/`)

Each civilization has a module in `crates/ffh-ai/src/civs/` implementing `CivPlay`, whose
methods default to "nothing special"; a civilization without logic of its own is a documented
stub naming what was tried. The generic AI calls the hooks at fixed points (marked
`// civ-mechanics`): `production` (after the settler step), `turn` (once per turn before units
move), `unit` (a group's orders before its role logic), `sea_worker`, `spell_value` (the
end-of-turn spell choice; 0 = not cast there), `building_value`, `tech_value`, `promotion_value`,
`civic_value`, `site_value` and `tech_goals` (the research plan's civilization part:
`tech_plan.rs`). Every hook dispatches on `civs::civ`, so a game without a
civilization that has logic plays exactly as before (same checksum), and
`FFH_AI_CIV_OFF=CIVILIZATION_X` (comma-separated, `X` alone, or `all`) makes a civilization play
as a generic one: the measurement switch. `civs/world_spell.rs` holds a world spell for a chosen
moment (`spell_value` returns 0 for it and the `turn` hook casts it once the rules and the AI's
default timing allow it). The research flavours of `tech_plan.rs` (FfH's per-civilization
beelines) are hooks too (`tech_goals`), under the switch since 2026-10-28; what remains outside
`civs/` mirrors the rules (the Doviello's and Illians' snow kept from Scorch, the rules' own
AI-only `reqScorch`) or is a flavor drive (`campaign::conqueror`).

**Unique mechanics are used** (the owner's decision, 2026-10-20): a civilization's own units,
spells and abilities are part of how it plays, so its AI uses them whether or not the games show a
direct benefit; the win-rate gate below decides how they are used, not whether.

**How it was measured.** `ffh stats --civ CIVILIZATION_X` forces a civilization into slot 0 (the
leader picked by the seed; the other slots stay random and avoid it), and
`crates/ffh-cli/scripts/civ_ab.py` plays the same seeds twice and follows that civilization only:
win rate (it decided the game -- victory, last standing or `--until-supremacy`'s clear supremacy
-- or held the largest share at the turn cap), with the seeds won by one side only and an exact
sign test; world share (score v2, basis points, 10000 = the world, fair share 1667 with six civs;
dead = 0) and legacy score at checkpoints, a finished game's last value carried forward, with the
paired t; survival, rank, world-spell use, and the unit, building and project counts that differ
(`ffh stats --detail`). All games are full games (6 civs, standard map, to a decision or turn 690,
`--until-supremacy`). Development (round 5, one implementer per group of five civilizations, in
parallel) used seeds 1-40 against the base `2d95e9c`, with the group's other civilizations
switched off so each number is that civilization's own. Seeds 1-40 chose among variants, so they
flatter what was kept; every civilization was then re-measured on fresh seeds 5001-5064, the
integrated tip (`d4ff7e3`) with its logic on against the same binary with it off
(`civ_ab.py --off-base`, identical opponents), and a civilization's logic was kept only if
neither its fresh win rate nor its fresh share at the end fell. What failed, and why in the
games, is in docs/AI_LOG.md.

**Result on fresh seeds** (64 games each; "won by one side only" = seeds the civilization won
with its logic and not without / without and not with; share diff with the paired t):

| civ | logic | win, off -> on | won by one side only (p) | share@300 | share@end |
|---|---|---|---|---|---|
| Lanun | coves (round 3, unchanged) | 6.2% -> 25.0% | 14 / 2 (0.004) | +229 (t 3.71) | +404 (t 2.26) |
| Balseraphs | Loki's Disrupt (new) | 9.4% -> 23.4% | 9 / 0 (0.004) | +228 (t 2.70) | +530 (t 3.18) |
| Clan of Embers | Warrens, Rantine, For the Horde (new) | 34.4% -> 40.6% | 7 / 3 (0.34) | +172 (t 2.43) | +147 (t 0.77) |
| Kuriotates | settlements (round 3) + placement, garrisons, rebuild (new) | 4.7% -> 9.4% | 5 / 2 (0.45) | +159 (t 3.42) | +215 (t 1.58) |
| Luchuirp | Golden Hammers (round 4, unchanged) | 3.1% -> 7.8% | 4 / 1 (0.38) | +42 (t 1.04) | +79 (t 0.55) |
| Sidar | Into the Mist held for a war (new) | 9.4% -> 9.4% | 0 / 0 | -3 (t -1.34) | +126 (t 1.81) |
| *reverted:* Amurites | Mage Guild and casters | 4.7% -> 1.6% | 0 / 2 | -13 (t -0.47) | -25 (t -0.31) |
| Calabim | vampires, Feast, River of Blood held | 10.9% -> 4.7% | 2 / 6 | +21 (t 0.83) | -77 (t -0.61) |
| Hippus | Warcry held for the strike | 14.1% -> 10.9% | 1 / 3 | -1 (t -0.06) | -6 (t -0.07) |
| Illians | ritual projects | 15.6% -> 14.1% | 2 / 3 | -9 (t -1.61) | -40 (t -0.56) |
| Sheaim | Mage Guild and adepts at war | 0.0% -> 1.6% | 1 / 0 | -22 (t -0.87) | -88 (t -0.97; @400 -144, t -2.17) |

Measured separately and held for a balance decision: the Illians' Stasis cast on their first
turns ("World spells: AI timing" below; fresh seeds 6001-6064 against FfH's condition, win 4.7% ->
23.4%, 13 / 1).
Over the six kept civilizations (384 games): win rate 11.2% -> 19.3%, seeds won by one side only
39 / 8 (sign test p 0.000006); this round's new logic alone (Balseraphs, Clan of Embers, Sidar):
17.7% -> 24.5%, 16 / 3 (p 0.004). The Kuriotates' new part alone (round 3's logic against this
round's, same seeds and opponents): win 4.7% -> 9.4% (5 / 2), share@300 +40 (t 0.83),
share@end +171 (t 1.34). The eight civilizations with nothing kept (Bannor, Doviello, Elohim,
Grigori, Khazad, Ljosalfar, Malakim, Svartalfar) play the same game with the switch on or off in
64 of 64 seeds. The Infernals and the Mercurians join mid-game and cannot be forced into a slot;
they were measured over the fresh games they appear in, replayed with their logic off (below).
In ordinary games (random civilizations, every AI the same code, seeds 5001-5048), the integrated
tip before the reverts against the base: development per civ unchanged (legacy score at turn 300
-0.4%, t -0.3; full games at turn 500 -1.6%, t -0.7), victories 25 -> 27 of 48, cities taken from
the barbarians +17% (t 2.0: Rantine). The civilization logic changes who wins, not how far the
world develops.

**The base AI per civilization** (census: 40 full games each, seeds 1-40, base `2d95e9c`):

| civ | win rate | alive at end | share@300 | share@end | world spell cast |
|---|---|---|---|---|---|
| Amurites | 5% | 50% | 1281 | 884 | 0/40 |
| Balseraphs | 22% | 65% | 1785 | 1640 | 32/40 |
| Bannor | 15% | 62% | 1379 | 1323 | 18/40 |
| Calabim | 10% | 62% | 1364 | 1001 | 32/40 |
| Clan of Embers | 35% | 88% | 2094 | 2394 | 34/40 |
| Doviello | 22% | 65% | 1575 | 1424 | 30/40 |
| Elohim | 15% | 60% | 1526 | 1240 | 15/40 |
| Grigori | 15% | 75% | 1707 | 1609 | 31/40 |
| Hippus | 15% | 65% | 1578 | 1497 | 30/40 |
| Illians | 12% | 75% | 1626 | 1460 | 35/40 |
| Khazad | 28% | 74% | 1758 | 1837 | 28/39 |
| Kuriotates | 5% | 60% | 1777 | 1112 | 36/40 |
| Lanun | 40% | 70% | 1788 | 1930 | 5/40 |
| Ljosalfar | 28% | 72% | 1642 | 1707 | 16/40 |
| Luchuirp | 25% | 80% | 1833 | 2049 | 35/40 |
| Malakim | 30% | 72% | 1619 | 1642 | 22/40 |
| Sheaim | 2% | 52% | 1310 | 912 | 0/40 |
| Sidar | 5% | 70% | 1531 | 1333 | 22/40 |
| Svartalfar | 20% | 78% | 1512 | 1513 | 32/40 |

Unique content the base AI never used in those games: the Amurites' Arcane Lacuna and Cave of
Ancestors, the Clan's Warrens, the Sheaim's Worldbreak (Planar Gate 1/40), the Luchuirp's
Blasting Workshop, Pallens Engine and Adularia Chamber, the heroes Corlindale (Elohim), Wilboman
(Illians), War Machine (Doviello) and Eurabatres (Kuriotates), and every civ's adepts (no AI
built a Mage Guild: 29 adepts in 3408 civ positions at turn 300). Since 2026-10-27 the
buildings among them whose effects are in the rules' code are valued (`code_effects.rs`, "Cities").

### Kept

- **Lanun** (round 3, unchanged): a work boat can turn an empty coast plot of theirs into a
  Pirate Cove (`SPELL_PIRATE_COVE`: +2 commerce; it grows into a Pirate Harbor and a Pirate Port,
  3/2/6 on coast; no other cove within 2 plots). As in C++ `AI_neededSeaWorkers` /
  `AI_buildPirateCove`, a city that works a plot where one can go trains a work boat (one at a
  time), and a Lanun work boat with no sea resource to improve in reach sails to the best site
  (worked plots of big cities first) and casts there. Round 3 measured it at 300 turns (score
  +23% in 23 games); the fresh full games above are its first measurement as a civilization's win
  rate, and it is the largest lever of the set. Fishing already comes at turns 30-65. Raging Seas
  is rare (5 of 40 games, around turn 590; see "World spells" below).
- **Balseraphs**: Loki (`bRivalTerritory`) casts Disrupt in another team's city: 0-2 of the
  owner's culture there is lost (`spellDisrupt`), and a city left with none becomes the
  Balseraphs' without war (its owner's attitude -4). The generic AI used him as a 2-strength
  fighter. At peace he goes to the nearest reachable (12 turns) revealed city of a civ we are not
  at war with whose owner culture is below 20 and whose culture rate is 0 (one Disrupt a turn
  drains one point on average, so a city producing culture never falls) and casts every turn:
  1-10 cities a game in traced games (about 3). Development: win 17.5% -> 20.0% (5 / 4),
  share@300 +168 (t 2.16), share@end +267 (t 1.52), cities +1.1 at turn 300 (t 2.4). Revelry (a
  golden age at 25 population) stays with the default timing. Risk: each flip costs -4 attitude
  with a civ at peace with us.
- **Clan of Embers**: Warrens (Masonry, 180 hammers: every unit finished in the city that is not
  a world, national, siege or naval unit gets a free copy; Python only, so valued 0 and never
  built) are valued as the hammers the copies add (half the city's production; since 2026-10-27
  in `code_effects.rs` with the other buildings whose effects are in code, "Cities"): 3.4 a game by
  turn 300, 7.6 by the end. Rantine casts Convert City in a barbarian city whose units are no
  stronger than him -- the Clan is at peace with the barbarians (`TRAIT_BARBARIAN`), so no army
  ever went there -- walking to the nearest such city within 15 turns while no civ war is on:
  +1.2 cities at turn 300 (t 3.2). For the Horde (half the barbarian orcs become the Clan's) is
  held for a civ war or its preparation (first seen around turn 320 instead of 218), so the
  converts arrive with a war to fight and the barbarians keep harassing the Clan's rivals
  meanwhile. Development: win 33.3% -> 43.6% (6 / 2), share@300 +147 (t 1.95), share@end +588
  (t 2.94); leave-one-out at 300 turns: Rantine +93 (t 2.0), For the Horde +141 (t 2.4), Warrens
  -40 (t -1.8), but share@end +588 with the Warrens against +362 without, so they pay late.
- **Kuriotates** (round 3's settlements, with this round's placement, garrisons and rebuild):
  past their limit of full cities (3 on a standard map) a new city is a settlement
  (`CvCity::init`: radius 2, -75% yields, nothing to build, no maintenance or civic upkeep; it
  claims land and resources). Round 3 builds one settler at a time for a settlement site on the
  city's landmass, from a full city of size 5+ (or at its happy cap) at peace; the ported
  `AI_foundValue` already scores settlement-sized sites for a sprawling player at its limit. In
  full games that alone hurt late: a plot is worked by the city nearest to it
  (`CvPlot::updateWorkingCity`), so a settlement 4-5 plots from a full city takes the full city's
  third-ring plots and works one of them at a quarter of its yield (development, the base with
  and without settlements: population at turn 300 37 against 41, commerce 117 against 159, share
  at the end 1095 against 1385, t -1.8; but share@200 +202, t 4.9: the land). Now: no settlement
  site within 5 plots of a full city (`site_value`, in `sites::compute`); the nearest full city
  trains a defender for a settlement that has none (settlements cannot train units); and after
  losing a full city a settler re-founds one (the generic settler step counts settlements towards
  the limit and never built another: seed 6 of the base sat with one full city and eight
  settlements from turn 233). Development (this round's part alone): seeds 1-40 win 5.1% ->
  5.1%, share@300 +69 (t 1.07); seeds 41-80 win 2.5% -> 12.5% (4 / 0). Their late decline is not
  fixed (5-7 cities lost by turn 450-500 with or without it). Legends keeps the default timing
  (5 cities, settlements counted; about turn 200).
- **Luchuirp** (round 4, unchanged): Gifts of Nantosuelta leaves a Golden Hammer in each city,
  which can become a free Engineer there (`SPELL_GIVE_HAMMER_TO_CRAFTSMAN`) or +1 strength on a
  unit that takes it (`SPELL_TAKE_GOLDEN_HAMMER_UNIT`). FfH's end-of-turn spell choice valued the
  take higher, so the garrison warriors took nearly every hammer (seeds 1-16: 59 takes, 13
  gives); units in our own cities now leave it, and the hammer gives itself (round 4, the 47 games
  where Luchuirp cast Gifts: production +11.8% at turn 300, t 2.5, great people +47%).
- **Khazad** (2026-10-27): the Dwarven Vaults. The rules set every Khazad city's vault by the
  treasury's gold per city: -2 happiness under 50, -1 under 100, +1 from 150, +2 and +10%
  production from 200, +2 and +25% from 300, +3 and +40% from 500. The generic AI spent its
  treasury down, so the cities sat in the Empty vault (8.3 a game). Now the treasury saves toward
  the highest level whose happiness (where cities are near their cap) and production repay the
  missing gold within 30 turns (15 at war), the slider keeping a gold rate that reaches it in about
  20 turns (never below 30% research for it); the current level is a floor that hurrying,
  upgrades, gold in deals, council costs and deficits leave (`civs/khazad.rs`, `civs::gold_target`
  / `gold_floor`). Vaults at the end: Empty 8.3 -> 0, Full 4.2, Overflowing 2.0 a game. Forks from
  turn 80: power at turn 229 +11-24%, research equal or better after an early dip. Fresh seeds
  43001-43048 against its logic off: win 27.1% -> 31.2% (7 / 5), share@end +244 (t 1.19),
  legacy@end +418 (t 1.75); its neighbours pay for it (see "Spells FfH kept from its AI").
- **Sidar**: Into the Mist (every unit gets `PROMOTION_HIDDEN`: invisible to other teams outside
  our cities and +1 first strike, until it fights or casts; units trained later do not get it)
  was cast the first turn the default timing (40 units) allowed, usually at peace, and the army
  lost it against barbarians before any war. It is held for a war: cast when at war with a
  civilization or preparing one, so the army marches unseen and opens with an extra first strike;
  with no war, once it has 60 units. Development: win 5.0% -> 7.5% (1 / 0), share@end +74
  (t 0.94); fresh: the hold changes the cast in 15 of 64 games, share@end +126 (t 1.81).

- **Unique units** (2026-10-20, the owner's rule: civilizations use their unique units). Over
  380 forced games (seeds 8201-8220) many unique units never appeared. Most need a building by
  `PrereqBuilding` (a building type, not a class), which production and research ignored: no
  Siege Workshop or Grove was built in any of those games, so no civilization ever had a
  catapult, trebuchet or cannon either. Now `production::unit_building` gives production
  (`enabler_building`, the siege step of `campaign::army_choice`) and research (`unit_available`)
  either kind of building; siege engines are trained for the siege role only
  (`production::siege_engine`), not as the army's attackers (the Khazad fielded about 30 Dwarven
  Cannons a game otherwise). On seeds 8201-8203 the Khazad went from 0 to 34 Trebuchets and 92
  Dwarven Cannons (before that rule), the Luchuirp from 14 to 34 Wood Golems, the Calabim from 0
  to 4 Vampires, and every civilization fielded catapults. Research values a tech that completes
  the techs of one of our unique units (its building's included) at 1500 and one of those
  techs at 500 (`civs::unique_units`, under `FFH_AI_CIV_OFF`). Back from the round-5 reverts
  under the same rule: the Calabim vampires in the army with Feast and Gift Vampirism, the
  Illians' ritual projects, the Grigori Medics at war, and the Infernal Manes added to cities
  beyond the garrison quota (round 5 kept one defender and lost the army). Civilizations whose
  casters are their own keep casters (`civs::unique_casters`, the Amurites' round-5 rule made
  common): the best city trains up to (cities + 2) / 2, with a Mage Guild where the Adepts need
  one -- the Amurites' Wizards and the Svartalfar's Illusionists come by upgrade at level 4, the
  Doviello's and the Clan's Shamans are their Adepts. A city builds the enabling building
  (`enabler_building`) when it opens a unit a third stronger than its best, or a third stronger
  per hammer: by strength alone the Luchuirp built Studios and never the Archery Range their
  Dwarven Slingers need, so 90-hammer Wood Golems held their cities; by strength squared per
  hammer nobody built a Training Yard (docs/AI_LOG.md, "Unique units").
  Measured with every civilization forced into slot 0, seeds 8701-8720 (380 pairs), against the
  AI before this work. Of the 101 unique units, counted per civilization (units at most two
  playable civilizations train), 27 -> 18 never appeared in a snapshot (every 50 turns), and the
  mean share of games in which one appears rose 37% -> 44%:
  Khazad Trebuchets 0 -> 80% of games, Svartalfar Illusionists 0 -> 60%, Doviello Shamans
  20 -> 80%, Clan Shamans 10 -> 70%, Wood Golems 45 -> 95%, Amurite Wizards 0 -> 50%, Calabim
  Vampires 15 -> 60%, Dwarven Cannons 0 -> 35%, Grigori Medics 0 -> 30%, War Tortoises 0 -> 25%;
  fewer Boar Riders (Luchuirp 40 -> 10%), Harlequins (20 -> 5%) and Dwarven Shadows (20 -> 0%,
  both dwarven civilizations). The 18 left are tier-4 units and heroes the games end before
  (Divine Essence, Strength of Will, Theology, Fanaticism, Optics...), units reached only by an
  upgrade at level 6 or more, and the Dwarven Shadow (Council of Esus as state religion). Win
  12.1% -> 13.9% (29 / 22, p 0.40), share@200 +57 (t 1.82), share@end +20 (t 0.28), legacy@end
  -67 (t -0.86); no civilization significantly worse in wins or share at the end, the Malakim
  better (wins 1 -> 7, share@end +833, t 2.20). Two costs. The Luchuirp economy (turn 300:
  economy buildings 23.7 -> 9.8, t -3.9; research 211 -> 150, t -2.0; wins 3 -> 3, share@end
  -23): with the Wood Golem (their Axeman) out of reach, their army was Warriors at 25 hammers and
  their research the highest at turn 300 (with the Lanun's); the army target counts units
  (production step 8), so their golems
  at 90 hammers cost what the others' Axemen do, and they went to war more often (0.3 -> 0.7 wars
  at turn 300, cities taken 0.6 -> 1.2). And the Amurites were eliminated in 10 games instead of
  5 (alive t -2.03; share@end +40, the survivors larger): lost wars in the games read, one at
  turn 163 before any caster, so not traced to the casters.
- **Unique units in play** (2026-10-20). A census of 190 games (seeds 9101-9110, each
  civilization forced into slot 0, all six players counted), by a throwaway build that printed
  each unique unit's creation, its role and place every turn, its fights with their odds, its
  casts and its death (the same games, to the checksum): the civilizations' own versions of the
  common units are used as much as the units they replace, or more (Wood Golems attack twice as
  often as Axemen, Trebuchets bombard, Myconids cast Spores 16 times each, Loki makes a puppet
  every turn), and most of the units never seen are late (Divine Essence, Strength of Will,
  Warhorses) or come by an upgrade at level 6 or more, in games that end near turn 520. Four
  cheap faults, fixed:
  - Workers cast into Beastmen fight (`units::fighters_off_worker_ai`), and only workers our
    land does not need are cast (`civs::doviello`). The rules cast the Doviello's Worker ->
    Beastman at war while they have as many workers as cities, and the Beastman keeps the
    worker's AI (`CvUnit::convert`): 558 of their Beastmen came from workers, built nothing, and
    were counted as workers, so production trained none (Battlemasters spent 28% of their turns
    on the worker AI, and some Doviello played 200 turns with almost no workers). Fighting
    without the gate, the cast took the workers production then trained at war, and the Doviello
    survived less often (docs/AI_LOG.md, "Unique units"). With it, against the same build without
    either (seeds 9201-9240): share at the end -17 (t -0.20), alive flat; from turn 300, 1.3 more
    workers (t 2.9), improvements +4-10%, research +8-13%.
  - Heroes production never chose (`production::role_ok`): the Hero promotion makes a hero
    (Wilboman and the War Machine keep an attacker's default AI and, as attackers, never won on
    cost: one Wilboman in 47 Illian games), so does Channeling I at strength 0 (Corlindale: none
    in 37 Elohim games), and a coastal city trains a hero ship (`Role::SeaHero`: Black Wind, none
    in 66 Lanun games; it serves as a transport).
  - Heroes that cannot attack move as casters (`units::role_of`): Loki walked alone as an army
    unit, and 30 of 42 Lokis were killed, 89% of the attacks finding him alone.
  - Donal's Recruit (once: population / 3 + 1 units in his city, twice with a Temple of the
    Order) is valued (`magic::recruit_value`): FfH's terms gave it nothing, and none of 20
    Donals recruited. Asking for a city half as good as our best, 5 of 9 Donals left their city
    with the army and never recruited; he now recruits in the first city that gives two, at the
    start of our turn (`civs::bannor`): a unit trained at the end of a turn first moves on the
    next, and the army took 3 of 9 Donals out of their city before the end-of-turn cast.
  On the census seeds (60 games, before -> after): Loki killed 78% -> 61% of the time (puppets
  per Loki 193 -> 230), Corlindale trained in 0% -> 67% of Elohim games, Wilboman 5% -> 45%,
  Black Wind 0% -> 69%, Donal's recruits 0 -> 1 a Donal (all 9), the Doviello's Beastmen, Sons
  of Asena and Battlemasters on the worker AI 19%, 12%, 28% of their turns -> 2%, 0%, 0%.
  Measured with the six civilizations they touch forced into slot 0 (seeds 9201-9220, 120
  pairs), against the build before: win 14.2% -> 17.5% (6 / 2, p 0.29), share at the end +172
  (t 1.75), legacy@end +184 (t 1.64); the Elohim +594 (t 2.08, wins 1 -> 3), the Illians +167
  (t 1.65), the Balseraphs +339 (wins 6 -> 8), the Lanun +173; the Doviello -20 (alive 0.7 ->
  0.6, t -1.83) and the Bannor -219 (t -1.03), both flat against the same build without their
  own fix (seeds 9201-9240: Doviello -17, t -0.20; Bannor with Recruit off +0, t 0.00, wins 6 /
  6): their rivals gained from the others. Share at turns 200 and 300 is lower (-45 and -61, t
  -1.5 and -1.7; not traced).
  Not fixed (worth a look, not yet cheap): the Luchuirp lose 6.0 Mud Golems a game to
  barbarians, the other civilizations 0.3-2.2 workers; Moroi garrisons upgraded to Brujahs
  (strength 11, defense 5) and Vampires stay garrisons (57% and 49% of their turns); research
  never plans a path to a late unique unit (Warhorses: Hornguard, Herne, Magnadine); Govannon
  never cast Teach Spellcasting (he does since round 7a); the Bannor train a Flagbearer in 18% of
  the games they could
  (56%), though one in a stack casts Morale every turn.

### Measured over the games they join (Infernals, Mercurians)

- **Mercurians** (kept; they join the team of the civilization that built the Mercurian Gate,
  and Basium converts the gate city). Their starting settler had no site (their one city stands
  in the builder's culture, where `sites.rs` finds nothing) and waited all game paying support;
  with no site on its landmass 20 turns after it appeared, it is disbanded. Development (the 76
  of 120 games where they appear on both sides): settlers left at the end 68 -> 9, share
  351 -> 385 (t 1.21). Fresh (the 35 games of seeds 5001-5064, Bannor forced, where they appear,
  replayed with their logic off): idle settlers at the end 0.83 -> 0.06 a game, their team's
  share at the end flat (2958 -> 2956, t -0.02), their team won 9 -> 12 games (3 / 0). Judged on
  the team, whose result it is: a small fix, neutral on share, kept.
- **Infernals** (reverted; docs/AI_LOG.md): Fallow cities never grow except through Manes (Add to
  City, which FfH's spell value never casts); Manes in a city with another defender there added
  themselves. Development (6 games): share 1056 -> 1557 (t 2.21); fresh (the 60 games of seeds
  5001-5064 where they appear, logic on against off): population +5.0 (t 2.23) but units -6.2
  (t -2.77), alive 0.8 -> 0.7, share -29 (t -0.68): the Manes were their army.

### Reverted, and tried without effect

Each module keeps a stub comment naming the attempt and its numbers; the diagnosis of each failure
from the games (what happened when it fired, and what that implies) is in docs/AI_LOG.md. Back since
2026-10-20 under the owner's rule (see "Unique units" above): the Amurite casters, the Illians'
rituals, the Calabim vampire rules, the Grigori Medics, the Infernal Manes (with a garrison quota),
and the building-gated units, now generic. In short: the casters of the Amurites and the Sheaim
(Mage Guilds and adepts) cast almost only buffs and cost army and economy; the Hippus' held Warcry
fell on the same strikes the cast at the declaration already covered; the Illians' White Hand came
at turns 325-372 for three priests that only summon; the Calabim's vampire rules barely fire
(vampires from turn 280+); holding Stasis, Religious Fervor, Wild Hunt or never casting Veil of
Night lost already in development; the Khazad's, Luchuirp's and Lanun's `PrereqBuilding` units
(siege engines, Hornguard, golems, War Tortoise) did not change their wars; Bannor's Crusade timing,
Grigori Medics, Elohim's Sanctuary hold, more Ljosalfar workers and the Kuriotates variants (no
settlements, extra defenders) were flat or worse. Older civilization notes kept for the record: the
Khazad's Dwarven Vault target (round 3: saving gold for the vault level cost more research than the
happiness repaid, score -5 +- 37 over 40 games; back since 2026-10-27 counting the vaults'
production, "Kept") and the elves' forests (`MaintainFeatures` keeps the
forest under every improvement they build on their own plots; `FFH_LOG` shows no forest cleared by
them).

### What the round shows

- **Development seeds mislead at this size.** Of the nine forced-civ changes kept after
  development, five lost on fresh seeds, and the Infernals' (six games in development, sixty
  fresh) too. The survivors act early and directly on what the score counts -- cities (Loki's
  flips, Rantine's conversions, settlement placement) and commerce (coves, Engineers) -- while the
  failures pay late (casters from turn 230-250, ritual projects after turn 300, rare vampires) or
  only retime a world spell. A 40-seed development A/B with |t| < 2 is a candidate, not a result.
- **World spells: the cast at the gate is hard to beat.** Holding one for a better moment lost in
  development for Stasis, Religious Fervor, Veil of Night (never cast) and Wild Hunt, and on fresh
  seeds for Warcry and River of Blood (with the Calabim's other changes): a spell cast at its
  gate keeps its effect for the rest of the game. The holds that remain are For the Horde, inside
  the Clan's package (its development leave-one-out: share@300 +141, t 2.4), and Into the Mist.
- **Casters are a generic gap, not a civilization's.** No AI builds a Mage Guild (value 0 in
  `building_value`), so no civilization has adepts; and when the Amurites and the Sheaim had
  them, the casters spent their turns on Haste, Regeneration and Dance of Blades, because the AI
  casts only at the end of the turn (the port omits FfH's casting before a stack attacks).
  Casters worth their cost need combat casting in strikes first, for every civilization.

### World spells: AI timing (`world_spells.rs`)

FfH 0.41 timed the AI's world spells in the rules, as AI-only branches of the spells' Python
requirements (`if pPlayer.isHuman() == False:` in `CvSpellInterface.py` `req*`). Those are AI
policy, so the port took them out of the rules (docs/PORTING.md, "Changes from FFH2"; a human's
requirements were never affected) and the AI applies them itself: `world_spells::ready` holds
FfH's condition per spell, and `casting::choose_spell` and the held casts (`civs/world_spell.rs`)
cast a world spell only once it holds. The move alone changes no AI game (every playable
civilization forced, two fresh seeds each, full games: the same checksums before and after).

FfH's conditions, the AI's default timing: Stasis, Legends, Rally, River of Blood and Gifts of
Nantosuelta at 5 cities; Warcry at war with 25 units; Veil of Night at peace with 25 units; Wild
Hunt at war with 20 units; Into the Mist at 40 units; Sanctuary, March of the Trees and Divine
Retribution at war with two teams; Raging Seas at war with a player whose fleet is worth more than
10; Ardor at 5 great people; Revelry at 25 population; Religious Fervor with 5 cities of the state
religion; For the Horde with 60 barbarian units; Mother Lode at 10 mines; Arcane Lacuna with 7
typed mana bonuses on the whole map; Worldbreak at an Armageddon counter of 50 (Hyborem's Whisper
had none). Several almost never hold, so their spell is never cast: Arcane Lacuna (0 of 40 games),
Worldbreak (about 3%, near the end), Raging Seas (5 of 40, around turn 590), March of the Trees and
Sanctuary (16 and 15 of 40, late).

**Expert moments** (`world_spells::expert_moment`, 2026-10-21; the owner's rule that
civilizations use their own mechanics): three spells FfH's condition almost never let the AI
cast. Arcane Lacuna (Amurites) at war with a civilization, instead of 7 typed mana nodes on the
map (0 casts in 24 full games); Worldbreak (Sheaim) at war once the Armageddon counter is 20,
instead of 50 (0 casts); Religious Fervor (Malakim) with the state religion in five cities, or
in three that are two thirds of ours (a small Malakim empire never had five). Measured with the
civilization forced, against the same binary with FfH's conditions (`FFH_AI_WS_TIMING_OFF=ALL`),
full games: development seeds 1-24, casts Amurites 0 -> 14 games, Sheaim 0 -> 6, Malakim 16 ->
14; fresh seeds 9601-9624, casts 0 -> 13, 0 -> 10, 13 -> 17, share@end +7, +24, +47 (every |t| <
0.3), wins 1 / 1, 0 / 2, 0 / 1 pairs. Earlier For the Horde (a dozen orcs) and Ardor (the
third great person) cost and were dropped (docs/AI_LOG.md).

**A losing war** (`world_spells::losing_war`, `city_in_peril`; off by default, the switch
`WS_LOSING_WAR` with `FFH_AI_WS_LOSING_WAR_ONLY=ALL` turns it on, 2026-10-22): Sanctuary
(Elohim) and March of the Trees (Ljosalfar) would wait, instead of FfH's two wars at once, for
More Naval AI's gate -- the civilizations at war with us have 1.5 times our power -- and an
invasion at the gates: the capital or two cities, each with four or more units of a civilization
at war with us within three plots whose attack exceeds the city's garrison at 1.5 times its
defense (barbarians do not count). Every version measured (the ratio alone, with one city
threatened, with four units there, with two cities or the capital) cost the Elohim share at the
end on development seeds (-127 to -162, t -1.1 to -1.7; the Ljosalfar -34, t -0.3): the one-shot
spell went on the first raid of a stronger neighbour, which the city held anyway (seed 21, turn
141; the invasion that took nine cities came seventy turns later, where FfH's condition had cast
it), or after cities had already fallen (seed 7). FfH's condition stays the default
(docs/AI_LOG.md, "Sanctuary and March of the Trees at a losing war").

**Moments of their own** (an experiment, not merged: `world_spells::turn`, once a turn before units
move, commit `0e4945b`, reachable from the `fanout/ws-*` job branches; measured with the
civilization forced against the same binary with FfH's condition, `FFH_AI_WS_DEFAULT=SPELL_X`,
development seeds 1-40 then fresh seeds 6001-6064):

| spell | moment | development | fresh | |
|---|---|---|---|---|
| Stasis (Illians) | as soon as they have a city | win 15.0% -> 22.5% (4 / 1), share@end -4 | **4.7% -> 23.4% (13 / 1, p 0.002)**, share@300 +419 (t 3.26), share@end +551 (t 3.05) | held: a balance decision |
| Raging Seas (Lanun) | a strike on a coastal city, or a coastal city in peril | 37.5% -> 25.0% (0 / 5) | 28.1% -> 25.0% (2 / 4), share@end +44 | not kept |
| Sanctuary (Elohim) | a city in peril in a civ war | 10.3% -> 10.3%, share@end +198 | 7.8% -> 6.2% (0 / 1), share@end +8 | not kept |
| March of the Trees (Ljosalfar) | a city in peril with 6+ of our forests near | 32.5% -> 30.0%, share@end -158 | 37.5% -> 28.1% (2 / 8), share@end -60 | not kept |
| Worldbreak (Sheaim) | a strike, counter 20+ | 5.0% -> 5.0% | 3.1% -> 1.6% (0 / 1), share@end -58 | not kept |

Stasis freezes every other player's production and research for 20 turns; FfH's condition (5
cities) cast it around turn 205, the new moment on turn 1-2, when the rivals' whole opening is at
stake. It is the largest single win-rate lever measured so far, and a balance question rather than an
AI one: an Illian player who casts it at once takes the first 20 turns of production and research
from every rival, a human's included. In the ffh-play bot test (`t1_scripted_bot_games`, seed 1, a
peaceful game) the bot standing in for a human lost its only city by turn 50, its opening frozen
while the barbarians roamed. A human could always do this; FfH's AI-only gate kept its AI from
it. Not merged until that is decided (cast at once, a later moment that does not cripple an
opening, or a rule such as no Stasis before some turn). Why the other
four failed is in docs/AI_LOG.md (in short: the defensive triggers fired during the civilization's
own winning offensives, and March of the Trees costs the elves their Ancient Forests for good).
What the group-C diagnosis would choose: Raging Seas the turn before a stack strikes a coastal city (its
defenders on a coast plot take ~35-50% damage); March of the Trees at one war when a city is in
peril with 8+ of our forest plots within 2 of the threat; Legends right after the third full
city; Gifts of Nantosuelta after the last planned city; Mother Lode when the windfall lifts the
Dwarven Vault a level. Group D's: Worldbreak the turn a campaign strikes a city, once the counter
is 20+ (half-health defenders in every enemy city); Divine Retribution only when the enemy's demons
and undead outweigh ours and the neutrals' (it hits everyone); Into the Mist at the start of a war
(as now held); Veil of Night early, as soon as the army is built (its peace condition is a real
constraint); Religious Fervor once the religion is in most cities and a war is near. Hyborem's
Whisper needs Malevolent Designs, never reached in these games. Given the round's lesson, an
earlier cast is the first thing to measure.

### Generic gaps these civilizations exposed (not fixed here)

- **Combat casting** (addressed by "Spellcasting" above): the AI cast only at the end of the turn
  (`casting::end_turn`: the best spell, then a buff, then a summon); FfH's casting before a stack
  attacks was omitted, so damage spells and summons rarely fell in a fight and casters filled their
  turns with buffs (docs/AI_LOG.md).
- **`PrereqBuilding` units** (fixed by `b6faa0a`): specific buildings that open units -- the
  Siege Workshop (Trebuchet, Dwarven Cannon, War Tortoise, War Machine), the Stable (Hornguard,
  Bison Rider), the Sculptor's Studio (golems, Gargoyle) -- were never built by the generic
  production (`enabler_building` and `campaign::siege_building` read only `PrereqBuildingClass`),
  and `research::unit_available` skipped such units. Now `production::unit_building` gives both
  the unit's `PrereqBuilding` or its civilization's building of the `PrereqBuildingClass`, and
  `unit_available` counts such units unless the building needs a religion. Still open: the Mage
  Guild (all adepts) has a value of 0 and `enabler_building` takes only defenders and attackers,
  so only `civs::unique_casters` builds one (docs/ENTITIES.md).
- **Foreign units through `UpgradeCiv`** (fixed; the owner's decision: a bug, closed in the
  rules). FfH's `canTrain` checks a unit with `UpgradeCiv` (Kuriotates Centaurs, the Clan's Ogres,
  Goblins and Wolf Riders, Calabim Vampires and Brujah, the Infernal Balor) only against that
  civilization's line, so it admits them in any city; FfH's UI and AI never ask, because they walk
  the city civilization's unit per class (`getCivilizationUnits`). The port's AI and ffh-play
  walked every unit type, so every AI trained them (the Luchuirp Studio opened the Clan's Ogre; the
  Khazad and Lanun fielded Centaurs and Ogres by the dozen). First the AI and ffh-play filtered with
  `city_offers_unit` (`b7d67f6`); then `city_can_train` itself refused another civilization's
  `UpgradeCiv` units (docs/PORTING.md, "Changes from FFH2"; `canUpgrade` unchanged, so a captured
  Centaur still upgrades to a Centaur Archer). The rules change plays the same games as the filter
  (identical sync checksums, seeds 7001-7008). The Airship is not part of this: it has no
  `UpgradeCiv`, and a Frigate upgrades to it only in the Kuriotates' hands or an Assimilation
  player's (`getUpgradeCity` checks the owner's line).
  Measured as the filter's commit against its parent, fresh seeds:
  - other civilizations' `UpgradeCiv` units at turn 300, of all civ units: 8.6% -> 0.0% in
    Kuriotates games, 8.1% -> 0.0% in Clan games (mostly Centaurs, Centaur Chargers and Archers);
  - 300 turns, seeds 7001-7048 (48 games): legacy score +3.4% (t 1.1), production +7.0% (t 2.0),
    gold -20% (t -2.4), every other metric |t| < 1.6; civ cities captured -16% (t -1.5);
  - full games, seeds 7101-7124 (24, `--until-supremacy`): legacy at turn 300 -1.6% (t -0.4), at
    500 -0.9% (t -0.1); supremacy 11 -> 12, cultural 9 -> 10, time limit 4 -> 2; decided 34 turns
    earlier (t -0.9);
  - Kuriotates forced (`civ_ab.py`, full games, seeds 7201-7232): win 6.2% -> 3.1% (won by one
    side only 0 / 1, p 1.0), share@300 -29 (t -0.2), share@end +129 (t 0.6), legacy@end +306
    (t 1.6); Centaur Archers 9.7 -> 11.2 per game, Ogres 1.1 -> 0. The rivals' Centaurs did not
    cost them a measurable edge;
  - Clan of Embers forced (same seeds): win 43.8% -> 34.4% (1 / 4, p 0.38), share@300 -3 (t 0.0),
    share@end -27 (t -0.1); the Clan had fielded 10 Kuriotates Centaur Archers a game and builds
    Axemen (4.8 -> 11.8), its own Wolf Riders (0 -> 5.9) and Lizardman Rangers instead.
  Neutral overall; the Clan's win-rate dip is within noise (5 seeds differ) and is the number to
  watch when the Clan's army logic is next measured. The per-civilization measurements above
  predate the fix; the Studio, Stable and Workshop experiments are worth re-measuring.
- **Workers stuck on a move the path finder refuses**: the worker logic picks a target by its own
  danger map, but `MISSION_MOVE_TO` with `MOVE_SAFE_TERRITORY` is refused by the rules' AI path
  finder (`AI_getPlotDanger`) and the worker retries every turn (a Lanun work boat at (42,23),
  seed 7, turns 200-239). The flee code passes `MOVE_IGNORE_DANGER` for this reason; the build
  move does not. Taking the next best job on a refused move cost a little production and
  research (docs/AI_LOG.md, "Stuck workers take their next best job"): the repeats are rare
  (6-19 worker-turns in a 200-turn game) and usually a passing danger.
- **Allied barbarians count as a threat**: the Clan of Embers and the Doviello's Charadon are at
  peace with the barbarians, but the garrison and research counts (`production::barbarians_near`)
  treat nearby barbarians as a threat all the same. Dropping them cost the Clan cities to its
  rivals and wins (docs/AI_LOG.md, "Allied barbarians no longer counted as a threat"): the term
  stands in for a missing rival-civilization term in the garrison quota.
- **AI-to-AI deals ignored `player_can_contact`** (fixed 2026-10-28, `diplomacy::can_talk`,
  switch `CONTACT`): a Crusading Bannor (`bNoDiplomacyWithEnemies`) still made peace with its
  enemies, and deals went to players FfH had just created (the temporary-player timer), vassals
  making their own peace and teams in a permanent war or peace. Peace, gifts, war bribes, trades,
  open borders and pacts now need both sides' `canContact`, and so do the offers to humans (C++
  `AI_doDiplo` asks only the proposer's, so a Crusader's enemy could still offer it peace).
- **Rules questions**: Wane and Kidnap (Sidar, Svartalfar) lack `bAllowAI`, so the AI can never
  cast them; `GAMEOPTION_NO_BARBARIANS` does not stop barbarians spawned by FfH's Python (a
  barbarian city holding the Dragon's Horde keeps sending units); C++ `AI_foundValue`'s
  settlement-site switch counts settlements as cities.
- Fixed in this round, found by these games: `units::great_person` read `Player::capital_city`
  raw and panicked on a dangling id (the Clan's conversions; now `player_get_capital_city`), and
  `unit_convert` could leave a converted unit as its own transport (an Airship, a stack overflow;
  a rules fix, docs/PORTING.md).

## Leader traits (`traits.rs`)

The rules apply every trait's effect by themselves (free promotions, cheaper civics, more health,
...), and the generic AI already profits where it reads numbers a trait changes: wonder turns
(Industrious), civic upkeep (Organized), upgrade prices (Ingenuity), combat odds (the free
promotions). What it did not do was play to them. `traits.rs` adds the decisions a trait calls for
(the owner's decision, 2026-10-26: this is flavour as much as strength, so a trait's play is kept
whatever the win rate does, and a drop is investigated as a flaw to fix). The hooks are marked
`// trait-play`:

| trait | leaders | play |
|---|---|---|
| Raiders (pillaged gold +100%, Commando) | Decius, Basium, Amelanchier, Hannah, Tasunke, Mahala, Weevil, Faeryl, Sallos | an army stack at war pillages the improved enemy plot it stands on with one unit (taken out of the stack and back), before its orders when that costs no march (no operation needs the stack and no city of ours is in peril, or every unit has a move to spare) and after them while it holds; a stack no operation needs raids the best enemy improvement within two turns whose danger is under two thirds of its defense and that is not next to an enemy city; the land within two plots of the city our war operation is taking is spared (it is ours once the city falls). The generic AI never pillages. |
| Spiritual (no anarchy) | Capria, Varn, Arendel, Jonas, Hyborem, Lethe | civics every 2 turns at a summed gain of 40 (others every 10 at 100); keeping the state religion weighs 5 instead of 15 |
| Arcane (adepts +20% experience) | Thessa, Valledia, Dain, Tya, Perpentach, Faeryl, Tebryn | a caster corps of (cities + 2) / 3, trained in the most productive safe city, its Mage Guild first (`civs::casters_up_to`, the civilizations' unique casters' helper) |
| Summoner (summons +1 turn, Combat I) | Keelyn, Tebryn, Os-Gabella, Malchavic, Hyborem | a bigger caster corps, (cities + 2) / 2; research toward its Adepts (the techs the Adept and its Mage Guild need, Knowledge of the Ether: +1500 once researchable, +500 on the way); its casters value a summoning sphere (a mana sphere opening a summon any civilization may cast: Death I-III, Fire II-III, Air/Earth/Water III, ...) at 1.5x + 60, a temporary summon at 1.5x (it lasts a turn longer), and raise lasting summons (Raise Skeleton) at peace too while the army is short of its peacetime target and a unit is affordable (others only at war); without a mana whose Mage-level spell summons a creature (Death: Spectre, Entropy: Pit Beast, Law: Einherjar, Ice: Ice Elemental; Keelyn's Balseraphs start with Mind, Chaos and Air), it goes for one first (switch `SUMMON_MANA`, 2026-10-28): its node on our raw mana is worth 3000 more (`mana::node_value`; nodes run 1000-3000), so research takes the node's tech first and the first node goes to it; it is worth 20 more in a trade (two happy resources on `AI_bonusVal`'s scale: the AI pays its own valuation in gold); and its casters value the sphere's first promotion (Entropy I before Entropy II) like a summoning one |
| Philosophical (great people +100%) | Einion, Cardith, Cassiel, Koun, Dain, Alexis, Judecca, Gosea | the great-people emphasis (`EMPHASIZE_GREAT_PEOPLE`) on its best great-people cities, 1 + cities / 8, size 4+, only cities that can employ a specialist making great people; a city is replaced only by one half as good again |
| Financial (+1 commerce on plots with 2+) | Garrim, Beeri, Kandros, Hannah, Rhoanna, Flauros, Dumannios | workers count the extra commerce an improvement brings at the threshold (`plot_calculate_improvement_yield_change` leaves it out), on an unimproved plot a city of ours works, weighted as the yields are (the improvement 1, its grown form 2) |
| Industrious (limited wonders +50% production) | Capria, Tethira, Beeri, Arturus, Sandalphon, Cassiel, Os-Gabella, Statius | limited wonders (world, team and national) are worth 1.5x |
| Aggressive (Combat I) | Basium, Kandros, Tasunke, Charadon, Duin, Weevil, Sheelba, Hafgan, Volanna, Alexis, Mahon, Averax, Ouzza | once its home landmass is full (`naval::home_is_full`), the war bar counts the free Combat I: the power edge it needs over a target is divided by 1.2 and its best trainable attacker counts 20% stronger against a city's defenders (the power rating and a unit type's strength leave the promotion out) |
| Ingenuity (upgrades at half price) | Basium, Arturus, Kandros, Mahala | upgrades keep half the gold reserve |
| Tolerant (Assimilation) | Ethne, Einion | never razes a conquered city |
| Expansive (+3 health a city) | Cardith, Thessa, Falamar, Rhoanna, Melisandre, Jonas, Meresin | one more settler at a time |

Passive (nothing to decide; their effect reaches every decision that reads the game):
Charismatic, Creative, Defender, Organized, Magic Resistant. Played elsewhere: Adaptive (the
trait choice, `events.rs`), Agnostic and Barbarian (`tech_plan.rs`), Fallow (`workers.rs`),
Sprawling (`civs/kuriotates.rs`).

`FFH_AI_TRAIT_OFF=TRAIT_X,..` (or `X`, or `all`) turns traits' play off and
`FFH_AI_TRAIT_CIVS=CIVILIZATION_X,..` keeps it only for those civilizations' players; with every
trait off the games are the same as without the module. `crates/ffh-cli/scripts/trait_ab.py
--base off.json --new on.json` pairs every player of two `ai_metrics.py` runs of the same seeds
and reports per trait (the leader's traits at the start; the Infernals and Mercurians, who join
mid-game and share the wins of the team they join, only with `--joiners`).

**How it was measured.** Full games (6 civs, standard map, to a decision or turn 690), the same
binary with every trait's play on against `FFH_AI_TRAIT_OFF=all`, read per trait with
`trait_ab.py`; development on seeds 1-40 (and 41-129 for Financial), validation on fresh seeds
7401-7496. Reading a trait's numbers needs care: a trait's play re-draws every game its leaders
are in, so on 40 seeds a group's win rate moves by 10 points on chaos alone -- a group that won
more than its share falls back and one that won less rises, whatever the change (in runs where
only Industrious or Spiritual was played, the Summoner leaders went 0% -> 14% with their own play
off). A drop was therefore checked with the trait played alone (the other traits off, so only its
leaders' games change), and three of them were flaws, fixed (docs/AI_LOG.md, "Leader traits"):
Raiders pillaged the land of the city they were taking, Financial workers built cottages nobody
worked, Aggressive leaders went to war while land was still free.

Fresh seeds 7401-7496, every trait on against off (578 players; "one side only" = seeds the
player won with the play and not without / without and not with):

| trait | players | win, off -> on | one side only | share@300 | share@end |
|---|---|---|---|---|---|
| Raiders | 110 | 6.4% -> 10.0% | 5 / 1 | -11 (t -0.23) | -55 (t -0.52) |
| Spiritual | 63 | 28.6% -> 22.2% | 4 / 8 | -95 (t -1.31) | -228 (t -1.73) |
| Arcane | 95 | 11.6% -> 15.8% | 10 / 6 | +20 (t 0.28) | +186 (t 1.79) |
| Summoner | 49 | 14.3% -> 12.2% | 2 / 3 | +133 (t 1.36) | +298 (t 2.30) |
| Philosophical | 89 | 7.9% -> 4.5% | 3 / 6 | +22 (t 0.38) | -74 (t -0.58) |
| Financial | 91 | 19.8% -> 26.4% | 15 / 9 | +81 (t 1.18) | +120 (t 0.76) |
| Industrious | 110 | 12.7% -> 13.6% | 9 / 8 | -33 (t -0.63) | -5 (t -0.04) |
| Aggressive | 85 | 21.2% -> 15.3% | 7 / 12 | +40 (t 0.80) | +3 (t 0.02) |
| Ingenuity | 45 | 20.0% -> 24.4% | 6 / 4 | -41 (t -0.62) | +93 (t 0.56) |
| Tolerant | 40 | 10.0% -> 12.5% | 3 / 2 | -96 (t -1.24) | -236 (t -1.16) |
| Expansive | 78 | 28.2% -> 23.1% | 5 / 9 | -12 (t -0.15) | -146 (t -1.06) |

No trait's difference is significant (sign tests p >= 0.22). (The Summoner row is the first
version, before the bigger corps and the summoning changes; the kept one is measured below.) The two lower win rates were played
alone on the same seeds: Aggressive (the kept version) 21.2% -> 23.5% (5 / 3), share@end +30,
cities taken +0.8 (t 1.7); Spiritual 28.6% -> 30.2% (5 / 4), share@end -141 (t -0.95) -- their
leaders lose in the pooled games to what their neighbours' traits do, not to their own play.
Financial alone on 60 seeds with a Financial leader: 18.4% -> 17.2% (6 / 7), share@end -39
(t -0.33); Industrious alone (seeds 1-40): 10.9% -> 10.9%, world wonders 2.1 -> 2.9 (t 1.9).
What the play does in the games: Summoner leaders keep casters and summon. Over 40 fresh seeds
with a Summoner leader (7501-7604, 48 Summoner players, `FFH_AI_TRACE_CAST`), the kept version
against all traits off: 16.4 Raise Skeleton / Summon casts a game instead of 2.4 (Tebryn 35.1,
Os-Gabella 20.9; the first version of the trait play cast 10.3: Tebryn 27.1, Os-Gabella 7.8),
12.2 Fireballs instead of 0.2, 2.3 casters at the end instead of 0.1; Keelyn's Balseraphs take no
summoning sphere and their casters make puppets instead, 652 a game (48 without the play, 425 with
the first version). Summons come in fights (a temporary one with an enemy in its reach, a lasting
one at war, or at peace while the army is short), so they bunch in the wars. Win rate 16.7% ->
12.5% (0 / 2), share@end +14 (t 0.13): neutral. Raiders pillage (Hannah, seed 2: 28 improvements in 210
turns, 10-66 gold each); Industrious leaders build more wonders; Aggressive ones take more
cities once their land is full.

**A summon mana first (2026-10-28, the owner's proposal; switch `SUMMON_MANA`).** Keelyn's
Balseraphs had no mana whose Mage-level spell summons a creature, so their casters never summoned
above Raise Skeleton. With the switch, seed 7502: Necromancy at turn 291 instead of 321, Entropy
nodes planned on all four raw mana plots, the first node Entropy (it was Enchantment). Measured
with the Balseraphs forced into slot 0 on seeds led by Keelyn, the same binary with
`FFH_AI_SUMMON_MANA_OFF=all` on the base side. Fresh seeds (40 of 47081-47160): Pit Beast,
Spectre, Einherjar and Ice Elemental casts 22 -> 213, games with one 6 -> 12, the first at turn
479 -> 428 (median); Raise Skeleton 42 -> 24 and Archmage summons 38 -> 26 (the first node went to
Entropy rather than Death or Air), all summons 102 -> 263; win 32.5% -> 35.0% (1 / 0), share@end
+50 (t 1.02). Development seeds (40 of 47001-47080): 28 -> 136, 10 -> 15 games, win 30% on both
sides. What still limits them: half the games never see a Mage of hers (20 of 40), and summons come
only in fights.

The games are decided sooner: final turn 527 -> 490 (t -2.48), clear supremacy 37 -> 52 of 96
games and 12 instead of 5 decided by turn 250 -- the leaders whose traits snowball (Financial,
Expansive) win earlier. In the games still running at turn 300 on both sides the world develops
the same (techs -1.3%, population -0.8%, research 0.0%, all |t| < 1).

## Measured together

Finance, settlers and escorts, siege operations and the civilization-specific code were
developed separately and merged without changes: their hooks sit in different steps
(`empire::manage`, `production::decide` items 5 and 9, the settler role, `military::army`).
6 civs, standard map, 300 turns, `ffh stats` and the game log (`FFH_LOG`); before = the AI
without the four, after = with them; t is the paired t-statistic over seeds.

| at turn 300 | seeds 1-12, 41-52 | fresh seeds 331-426 |
|---|---|---|
| score (alive civs) | 782 -> 891 (t 3.4) | 818 -> 894 (t 6.2) |
| score (dead civs as 0) | 752 -> 841 (t 2.4) | 789 -> 856 (t 4.9) |
| techs | 18.7 -> 22.2 | 19.4 -> 22.0 (t 7.6) |
| cities / population | 6.14 -> 7.13 / 38.9 -> 41.9 | 6.41 -> 7.07 / 41.1 -> 42.7 |
| research / commerce | 58 -> 87 / 92 -> 120 | 67 -> 88 / 101 -> 122 |
| research slider | 46% -> 63% | 49% -> 62% |
| civs at 20% research or less (at 0%) | 46 -> 7 (28 -> 5) | 166 -> 46 (95 -> 29) |
| unit support, % of commerce | 12.5 -> 9.3 | 12.1 -> 9.3 |
| cities taken civ <- civ / civ <- barbarian / barbarian <- civ | 16 / 29 / 34 -> 26 / 54 / 70 | 65 / 123 / 175 -> 110 / 232 / 240 |
| civs eliminated | 6 -> 8 | 21 -> 26 |
| settlers killed (log) | 33 -> 14 | 106 -> 64 |
| civ-founded cities lost to barbarians (log, first loss) | 45 -> 78 | 220 -> 243 (5.8% -> 6.0% of those founded) |
| CPU ms per turn | 46.5 -> 47.6 | 47.4 -> 47.6 |

Each part alone against the whole (fresh seeds 331-378; score alive / dead as 0 / techs, against
the AI without the four): finance +61 / +67 / +2.2, settlers and escorts -27 / -15 / -0.9,
siege operations -4 / -18 / -0.5, civilization-specific -2 / -2 / 0.0 (only Kuriotates and
Lanun games change), all four +72 / +66 / +2.8. Leaving one out (same seeds, against all four):
without settlers and escorts -6 / 0 / -0.2 and settlers killed 36 -> 82 (alone it costs
research; with the finance module it no longer does); without siege operations +7 / +19 (t 1.3)
/ 0.0, with civ cities conquered 26 -> 10, barbarian towns taken 50 -> 29, civ cities lost to
barbarians 118 -> 88 and civs eliminated 11 -> 7. The siege operations cost about as many cities
to the barbarians as they take from them, as they did alone (towns taken and lost again: civ <->
barbarian recaptures 56 -> 141 over seeds 331-426).

Tried and not kept: stacks committed to a barbarian town turn back for our cities in peril
unless they strike this turn (`campaign::on_the_move` at war only). Seeds 331-378 and 1-12,
41-52: score (dead as 0) +15 (t 2.0), 16% fewer civ cities lost to barbarians; seeds 379-426:
-7, 4% more lost; all 120 seeds: +6 (t 1.2), techs +0.2 (t 1.7), eliminations unchanged.

### Round 7h (2026-10-25): striking at the strike phase's odds; bolder conquerors

The test of whether the AI is too timid in sieges (the owner's request, after round 7g). A siege
entered its strike phase when the adjacent ring took the city every defender 70% sure to fall, but
`strike` started only at 90% (with a reserve): stacks stood next to cities, the phase saying
strike and the strike holding. `STRIKE_AT_PHASE` starts the strike at the phase's own odds;
`BOLD_STRIKE` also lowers the phase's bar to 60%. Both fire rarely (3 bold starts in four traced
games). Forks: the Khazad took a six-defender Doviello city at once (the reserve rule never took
it in 12 turns; power 419 against 394 at turn 300); a Calabim lone unit struck a barbarian town
at 75%, lost, and missed the town the reserve rule took a turn later with a second unit.

As one player's edge (fresh seeds 40001-40016, 304 paired games, 300 turns): `STRIKE_AT_PHASE`
win rate 14.1% -> 14.5% (1 / 0), share +0.5% (t 1.5), 264 games identical; `BOLD_STRIKE` win
14.1% -> 12.8% (0 / 4, p 0.13), share +0.6% (t 1.2), 238 identical. Across all players (seeds
41001-41096): `STRIKE_AT_PHASE` civ cities taken +2.7%; `BOLD_STRIKE` civ cities +8.2% (t 1.8),
barbarian towns +7.5%, civs eliminated +30% (t 2.8), techs -1.7% (t -2.2). Neither makes a
player win more: the AI is not too timid in sieges, and fewer cities changing hands under the
faithful simulation (round 7g) is a matter of flavor. Kept: `STRIKE_AT_PHASE` for every
civilization (a consistency fix, neutral), `BOLD_STRIKE` for the conquerors only
(`campaign::conqueror`: the Clan of Embers, the Doviello, the Bannor, Aggressive leaders;
CLAUDE.md, "Conquest and domination are a flavor drive").

### Round 7g (2026-10-25): the simulation for every attack

The owner's request: fix the strike simulation and run it for every attack. Two switches, both
on (the code and its forks: "Siege operations" and "Deciding an attack" under Field armies).
- `SIM_FAITHFUL` (`campaign::simulate_in_order`, `Order::Strike`): the strike played as it will
  be fought -- the attackers in the strike's own order, each against the defender the rules put
  up against it (strength against that attacker, first strikes, a world unit halved); a siege
  engine's win leaves the defender at its combat limit; a lost fight deals at most a round short
  of a kill; a defender has fallen when it is 90% sure to be dead (70% once positioned), in place
  of an eighth of its expected health and a reserve fifth of the force. Fork (seed 26001, saved
  at turn 221 from the game with the switch off): the old estimate expected four attackers to
  take a barbarian town; the Sheaim struck with six, lost five and the town held; with the switch
  the six held back (expected losses 3.5, a defender likely standing) and kept all 13 army units.
- `FIELD_SIM` (`military::field_sim`, `field_go`): every field attack decided on the simulation
  of the group attack in the rules' own order (`AI_getBestGroupAttacker` with its
  `AI_attackOdds`). Forks: the Kuriotates killed a barbarian Warrior two turns sooner, a
  one-city Elohim killed a Gargoyle beside its capital (off, the Gargoyle killed a Warrior of
  theirs); it no longer grinds four Warriors into a Griffon at 1%. Two versions were replaced on
  the way (docs/AI_LOG.md, round 7g): a pure trade of kill and loss values lost units early;
  good odds or a clearing grind with raiders at face value lost cities to the barbarians.
- Bugs the simulation's own traces turned up before any measurement: wounded defenders counted
  as already dead, follow-up attacks never played (a defender's blended expected health read as
  dead while it was alive half the time; fallen is now by the chance it is alive), the rules'
  attacker picked by the exact odds where the rules use their own estimate.

Measured as one player's edge (`civ_ab.py --civ all`, 304 paired games a set, 300 turns), the
kept version: `FIELD_SIM` on seeds 33001-33016 win 12.2% -> 13.8%, share -1.0% (t -0.5);
35001-35016 win 17.1% -> 13.8%, share -0.2%; fresh 37001-37016 win 16.8% -> 15.8%, share +0.9%,
legacy +2.1% (t 1.0). `SIM_FAITHFUL` alone: 33001-33016 win 12.2% -> 12.8%, share 0.0%;
35001-35016 win 17.1% -> 16.1%, share -0.9%. Both, fresh 37001-37016 (sureness 95% / 80%): win
16.8% -> 15.8%, share +0.8%; fresh 39001-39016 (the kept 90% / 70%): win 19.1% -> 20.7% (22 / 17),
share -2.0% (t -0.9), legacy -1.4% (t -0.6). Neutral on every set. Time per turn unchanged (25.90
-> 25.88 ms, all AIs, seeds 38001-38096). With every AI using both (38001-38096): legacy -1.4% (t
-0.9), civ cities captured -14% (t -1.8; -23% at 95% / 80%), barbarian towns taken -11%, wars
declared -11% (t -1.7): fewer cities change hands (Known gaps).

### Round 7f (2026-10-24): healing and the hero gate at their firings

The first use of forks (Tools, "Forks first") on the two military behaviours then off by default
(round 7): each forked at a few of its firings (seeds 26001-26006), the switch on against off
from the same save. Both proved locally helpful where they fired as meant, and each had a misfire
the win rate could not have named; both misfires are fixed, and a third flaw the forks turned up
(the strike's damage estimate) with them. All three are on.

**Forks of the healing rule** (`HEAL`). What it should change: a committed stack with one badly
wounded unit no longer stops whole; it reaches its target sooner and strikes earlier.
- Seed 26006, turn 282 (Elohim, 8-unit stack 558, "marches on with its wounded"): on, it reached
  the barbarian-founded city at (63,33) and took it at t285 in 10 attacks; off, it healed in
  place two turns, and its strike at t287 held after 11 attacks, the city falling at t288 after 4
  more. Ten turns on: one city more, army 31 against 28, power 314 against 291.
- Seed 26001, turn 266 (Hippus, 7 Horsemen, stack 771): on, the stack was next to its target
  (52,29) at t271, three turns before off (where it healed in place t267-271); the strike held
  for lack of margin (the simulation needed all 8 attackers), a fifth defender came, then peace.
  Neither took the city; nothing was lost by marching.
- Seed 26006, turn 239 (Luchuirp, stack 696, the misfire): the stack walked into a city to heal,
  walked out, and was sent back the next turn, five times in twelve turns. In our cities a unit
  split off to heal only over 30% damage (More Naval AI's `AI_heal(30, 1)`), while the field rule
  sends a stack back when most of it is 20%+ hurt: units healed to 21-30% left, and the field rule
  returned them. The city threshold is now the field rule's (20%); replayed from the same save
  (`ai-branch.sh --save`), the stack healed in the city through t247 and then marched one plot a
  turn to its target.

**Forks of the hero gate** (`PRECIOUS`). What it should change: a hero or high-level veteran does
not attack at odds it may well lose; captures should not suffer.
- Seed 26001, turn 246 (Elohim strike on a Sheaim city, the misfire): on, the two veterans were
  held out of the strike, the other seven units attacked three times, the simulation without the
  veterans no longer covered, and the strike stopped: three units lost, the city untaken, the
  siege back to gathering. Off, the city fell in six attacks and the war ended. The gate had also
  kept them out of the operation's force count. Now precious units attack last below their odds
  (while another unit can attack, and only in a strike expected to take the city, never to wear
  defenders down) and count in the force; replayed from the same save, the city fell in seven
  attacks, the veterans unhurt, two more Archers spent than off.
- Seed 26003, turn 127 (Balseraphs, a three-unit stack whose best attacker was a level-5 Warrior
  at 35%): on, it kept the Warrior (off threw it in and lost it, then a second unit), but stood
  next to the same raider for twelve turns, the hold repeated each turn. Now a stack does not pick
  a target the gate would hold (adjacent or hunted); replayed, it hunted elsewhere and ended
  army 18 against 16.
- Seed 26004, turn 239 (Hippus strike on a barbarian city with two veterans present): the same in
  both branches -- two Horsemen at 81% took it, the veterans never needed.

**Forks of the strike's damage estimate** (`STRIKE_DAMAGE`). Seed 26002, turn 274 (Calabim)
showed a strike flaw of its own, with the gate on or off: Moroi and Bloodpets went at a barbarian
city at odds 0, turn after turn, and left all five defenders at full health. The defender was
Acheron (strength 19 in the city, four first strikes, against a Moroi's 7). While a side has first
strikes left it cannot be hurt, so a Moroi died in rounds where it could not land a blow; the
expected damage of a lost fight (`campaign::fight`) ignored first strikes and credited each Moroi
with 12 of Acheron's hit points, and the strike's simulation, built on the same figure, expected
twelve attackers to take the city. Now first strikes count (the side with more has that many
rounds in which only the other is hurt; a chance first strike counts half), a sacrifice is the
unit whose expected damage, not strength, is cheapest for its cost, and none is sent that would
deal under a tenth of a defender's hit points.
- Seed 26002, turn 274 replayed: the Calabim left Acheron's city alone and took another barbarian
  city at t279 in five attacks (off: their first city at t284, after the failed strikes); army 54
  against 46 at t288.
- Seed 26005, saved at turn 191 from the game with the switch off (Lanun, eight Warriors next to
  a barbarian city of three Archers, one first strike each): off, they struck at 2-6% odds, lost
  six, the city held, and fell at t202 to a second force; on, the estimate said the force was
  short, they waited three turns for eleven units and took it at t195 in one strike (army 34
  against 30 at t204).
- Seed 26005, turn 105: the force needed for an Archer-held barbarian city rose from 4 to 5 units;
  nothing else changed in 25 turns.

**Measured** (fresh seeds, the same build, each switch forced on against all three off). Over every
AI (`FFH_AI_<NAME>_ONLY=ALL`, 30001-30192, 300 turns): healing legacy -1.2% (t -1.3; +1.2% on the
first 96 seeds, -3.6% on the next), captures of civ cities +12%, power -2.6%; the hero gate -0.3%,
cities +1.6% (t 2.3); the damage estimate -0.3%, civ cities captured +8%, cities destroyed +18%
(t 2.7); all three -1.1% (t -1.0), civs eliminated +26%. Full games, all three (31001-31096):
legacy -1.1% / -2.0% / -2.1% / -2.2% at turns 200 / 300 / 400 / 500 (|t| < 1.5), civs eliminated
0.95 -> 1.20 a game (t 2.05), victories before the turn limit 14 -> 19 of 96. The per-seed loss
followed the eliminations (correlation -0.57 for healing), not weaker play: every AI got better at
taking cities, and a dead civilization counts 0 in the mean (Tools, "A behaviour every AI gets is
not a player's win rate"). Given to one player against opponents without it (`civ_ab.py --civ all
--new-env FFH_AI_<NAME>_ONLY={civ}`, 32001-32016, 304 paired games, 300 turns):

| switch | win rate | share at 300 | legacy at 300 |
|---|---|---|---|
| healing (`HEAL`) | 13.5% -> 15.8% (13 seeds won only with it, 6 only without; p 0.17) | +3.4% (t 1.67) | +2.7% (t 1.44) |
| hero gate (`PRECIOUS`) | 13.5% -> 14.5% (10 / 7) | +3.3% (t 2.04) | +3.0% (t 1.90) |
| damage estimate (`STRIKE_DAMAGE`) | 13.5% -> 15.1% (7 / 2; p 0.18) | +1.2% (t 1.29) | +0.5% |

### Round 7e (2026-10-24): the standing army by deterrence

`RIVAL_DEFENSE` turned on by default (the owner's request), its peacetime defenders sized by
deterrence instead of a threat threshold ("The standing army against rival civilizations"; the
old rule and why it went: docs/AI_LOG.md, "Peacetime defenders by a rival-threat threshold").
Re-measured on seeds 26001-26192 (300 turns) once `HEAL` and `PRECIOUS` were off, the old rule
was neutral against the switch off (legacy -0.4%, t -0.55), as was each of its parts; its round-7
cost (-3.2%) came with the other two on. Deterrence against off, same seeds: legacy -0.3% (t
-0.4), research -2.4% (t -1.3), military +4.0% (t 2.8), wars declared +9% (t 2.0: our own power
counts in our war bar too), civs eliminated -11% (t -1.3).

Validated on fresh seeds, the final build with the switch on against off. At turn 300 (28001-28192,
192 games): legacy -0.6% (t -0.7), research -2.3% (t -1.4), military +1.6%, civ cities taken
-12% (t -2.1). Full games (28201-28296, 96): legacy +0.2% / +1.8% / +1.8% / +0.6% at turns 200 /
300 / 400 / 500 (t 0.3-1.4), techs +2.5% at turn 300 (t 1.9), military +4.6% / +6.3% at 300 / 400
(t 2.7 / 2.1); civs eliminated 1.31 -> 1.01 a game (t -2.6), civ cities taken -14.5% (t -1.7),
cities lost to the barbarians -19.5% (t -1.5); supremacy 58 -> 62, other victories 38 -> 34 of 96.
Fewer civilizations die and fewer cities change hands, at no cost to the score. After merging
master's fog of war for the AI, a check on 96 more seeds (29001-29096, 300 turns) read legacy
-2.1% (t -1.8), techs -2.4% (t -1.7): the three 300-turn sets lean slightly negative (pooled
about -0.8%, none significant) while the full games lean positive. Worth a full-game check under
fog of war.

### Round 7 (2026-10-22): More Naval AI's ideas, the four rounds together

The behaviours of rounds 7a-7d, each measured alone in its round, were merged and validated
together against master (before round 7) on fresh seeds 24001-24192 (300 turns) and 24201-24296
(full games). Neutral to positive, nothing falling: at turn 300 (192 games) legacy +0.7% (t 0.5),
research +7.5% (t 3.3), population +2.6% (t 1.6), military +4.2% (t 2.1); in full games legacy
+0.2% / -0.2% / -0.4% / +1.8% at turns 200 / 300 / 400 / 500 (all |t| < 0.6), research +27.9% and
military +13.5% at turn 500 (31 games, t 3.6 and 2.6), victories 38 -> 43 and supremacies 58 ->
53 of 96, civ cities taken -10% and cities lost to the barbarians +29% (|t| < 1.3). Inside the
merged build, two behaviours were checked again alone (seeds 24001-24096 and 24201-24248):
`KEEP_UNITS` gained early (legacy +2.0% and research +6.2% at turn 300, t 1.4 and 2.5) and cost
late (research -7.5% at turn 400, t -1.6; legacy -2.0% at 500); `HEAL` cost late (legacy -2.3% /
-5.6% at turns 400 / 500, t -0.7 / -1.95) and lost 29-43% more cities to the barbarians, which led
to loose stacks healing where they stand again (Field armies, step 6). Refined, on new fresh seeds
25001-25096 / 25201-25248: legacy -1.2% at turn 300 (t -1.1), civ cities taken +20% (t 1.9; sieges
go on instead of stopping beside the target), and -1.6% / -3.2% / +1.0% at turns 300 / 400 / 500
in full games (|t| < 1.2): a small cost left (Known gaps).

Merged again with master (2026-10-23) and measured against it on seeds 26001-26192 (300 turns),
the package lost: legacy -3.1% at turn 300 (t -2.6), techs -4.9% (t -3.5). Switching each round
off inside the merged build put the whole cost on round 7b, and switching its behaviours off one
at a time on three: `RIVAL_DEFENSE` (legacy -3.2%, t -3.8; research -7.2%), `PRECIOUS` (-2.0%, t
-2.6; research -5.1%) and `HEAL` (-2.1%, t -2.1); `CAPTURE`, `SECOND_OP` and `FLEET` were neutral.
Those three were turned off by default (`RIVAL_DEFENSE` came back on in round 7e, `HEAL` and
`PRECIOUS` in round 7f, each once its cost was found). The final configuration
against master on fresh seeds 27001-27192 (300 turns) and 27201-27296 (full games): legacy -0.5%
at turn 300 (t -0.5), research +3.6% (t 1.9), military +3.5% (t 2.3); in full games legacy +1.5% /
+0.4% / -1.1% / +3.4% at turns 200 / 300 / 400 / 500 (|t| < 1.8), research +28.3% at turn 500
(t 5.3), techs +6.2% (t 1.75); cities -4.8% at turn 400 (t -3.0), civs eliminated 1.23 -> 1.02 a
game.

### Round 7a (2026-10-22): magic, casters, summons

Five behaviours borrowed from More Naval AI's magic ("More Naval AI compared" below) in the form an
expert would play them, each behind its own switch (`switches.rs`) and described where its area
is ("Spellcasting", "World spells: AI timing"): mana nodes (`MANA_NODES`), the caster corps
(`CASTER_ROLES`), summons spent before they expire (`SUMMON_SPEND`), Govannon's Teach
Spellcasting (`TEACH_SPELLS`) and Sanctuary and March of the Trees at a losing war
(`WS_LOSING_WAR`, off by default). Each was measured alone: the same binary with its switch off
(`FFH_AI_<NAME>_OFF=ALL`) against everything on, 6 civilizations on a standard map, development
seeds 1-48 (300 turns) and 1-24 (full games, `--until-supremacy`), then fresh seeds 20001-20192
(300 turns) and 20201-20296 (full games), never used to choose anything. Percentages are the change
of the per-civilization mean (dead civilizations as 0), t the paired t over seeds; a checkpoint
counts the games still running. The development runs used the build before the last world-spell
change (`WS_LOSING_WAR` then on, which changes only games with the Elohim or the Ljosalfar); the
fresh runs, the final build.

**What fired** (24 development full games, final build, `FFH_TELEMETRY=1`; four of them traced):
182 mana nodes built (Enchantment 122, Entropy 31, Mind 11, Life 5, Fire 5, Earth 3, Law 2, Death,
Metamagic, Water), the raw mana left unconverted in our lands falling from 0.76 a civilization at
turn 300 to 0.14 at turn 500 on fresh seeds (mana types 3.3 -> 4.1); 678 adepts and 62 Shamans
trained (on fresh seeds about two adepts a civilization at turn 400, two and a half at 500), 143
Mage Guilds; role assignments
(changes) node builder 146, terraformer 53, city mage 225, war wizard 275; Teach Spellcasting cast
89 times by 6 Govannons (in 4 games; he walks to stacks worth 800-2000 on `promotion_value`'s
scale, Enchanted Blade for a stack); summons spent at worse than 5% 5-45 times a game (four traced
games: Loki's puppets 26, Pit Beasts 13, Einherjar 10, Fireballs 9; half below 1%); Sanctuary 1
and March of the Trees 4 casts (FfH's condition).

| behaviour | development, 300 turns (48) | development, full games (24) | fresh, 300 turns (192) | fresh, full games (96) |
|---|---|---|---|---|
| mana nodes | legacy +0.3% (t 0.6), pop +1.3% (t 1.9), prod +1.8% (t 1.3) at 300 | at 300: legacy +1.0% (t 1.8), pop +2.1% (t 2.8), prod +3.7% (t 2.6); research +10.9% at 400 (t 2.0); power +9.9% at 500 (t 2.3) | at 300: legacy +0.5% (t 1.7), research +2.3% (t 2.4), pop +1.1% (t 2.7), prod +0.9% (t 1.4) | at 300: research +2.6% (t 1.9), pop +1.8% (t 2.6), power +1.9% (t 2.2); at 400: legacy +1.5% (t 1.5), pop +4.1% (t 2.2); at 500: legacy +2.3% (t 2.6), techs +2.4% (t 1.6), research +6.9% (t 1.8); civ cities taken 16.5 -> 15.3 a game (t -1.7) |
| caster corps | at 300: legacy -0.2% (t -0.5), techs -0.7% (t -1.2), prod +2.4% (t 2.7) | techs -1.5% at 300 (t -1.6), prod +2.6% (t 2.2); research +10.4% (t 1.7) and pop +5.7% (t 1.9) at 400; power +4.9% at 500 (t 1.3) | at 300: legacy +0.1% (t 0.4), techs +0.6% (t 1.3), research -0.5% (t -0.4), power +2.4% (t 3.1) | legacy +0.6% / +1.6% / +1.8% at 300 / 400 / 500 (t 1.3, 1.3, 1.2), research +2.4% at 300 (t 1.2), techs +3.0% at 500 (t 1.7); production -4.5% and power -3.4% at 500 (t -1.1); civ cities taken 16.8 -> 15.3 (t -1.8) |
| summons spent | flat (every \|t\| < 1.1 at 300); barbarian towns taken 3.21 -> 2.79 (t -2.0) | flat to 400; power -5.0% at 500 (t -1.5) | at 300: legacy +0.7% (t 1.4), techs +0.9% (t 1.3), research +1.5% (t 1.2), pop +0.6% (t 1.3); 78 of 192 games unchanged | at 300: legacy +1.7% (t 2.6), techs +2.0% (t 2.3), pop +2.2% (t 2.3), prod +2.4% (t 2.1); at 500 legacy +2.9% (t 2.0); victories 35 -> 43, supremacy 61 -> 53 of 96 |
| Teach Spellcasting | (no Govannon before turn 300) | 4 of 24 games changed; every \|t\| < 1.7 | -- | 10 of 96 games changed (each one with the Amurites' Govannon); every \|t\| < 1.5; those Amurites' share at the end 1842 -> 1944 (9 games) |

- **Mana nodes** pay from turn 300 in every set: an Enchantment node is a happy face in every
  city, and the cities grow (population +1-4%), research following. Fewer civilization cities
  change hands in fresh full games (-7%, t -1.7). Kept.
- **The caster corps** loses nothing on either fresh set and leans positive in full games, without
  the costs the three earlier caster attempts measured (a fifth of the Luchuirp's research, the
  Amurites' -22 beakers, docs/AI_LOG.md): it is trained only while our mana gives an adept a sphere
  worth having, and pays in nodes, terraforming and city spells first. The cost to watch: by turn
  500 production -4.5% and power -3.4% (t -1.1), and civilization cities taken 9% fewer (t -1.8):
  late in the game the war wizards and city mages take hammers from the army. Kept; the next lever
  is the number of war wizards against what their spells add to a strike.
- **Summons spent**: flat in development, positive on both fresh sets (legacy +0.7% at turn 300,
  t 1.4, and +1.7% in full games, t 2.6). Most games diverge after the first spent summon, so part
  of this is chance; neither set
  shows the loss of the earlier any-odds attempt, which the cover rule and the cities left alone
  at the end of the turn were for (in four traced games 26 of 64 spent summons were Loki's
  puppets). Kept.
- **Teach Spellcasting**: Govannon is the Amurites' hero and comes late (Arcane Lore), in 4 of 24
  and 10 of 96 games: too few to measure. One development game shows a cost: in seed 22 Govannon
  left the Amurites' war to teach elsewhere, and they took 2 cities instead of 4 between turns 400
  and 500 (share at the end 3871 -> 1978). An expert teaches the stack he fights with while at war
  and walks to others in peace; not done. Kept (the Amurites use their hero).
- **A losing war**: off by default (above, "World spells: AI timing"; docs/AI_LOG.md).

### Round 7b (2026-10-22): military

Six behaviours from the More Naval AI comparison, each behind its own switch and described where
its area is ("City defense against rival civilizations", "Field armies", "Siege operations",
"The assault fleet"): defense weighed by rival civilizations (`RIVAL_DEFENSE`), healing
(`HEAL`), opportunistic captures (`CAPTURE`), a second operation (`SECOND_OP`), heroes and
veterans at good odds (`PRECIOUS`), the assault fleet (`FLEET`). Each was measured alone on
development seeds 1-48 (300 turns), the same binary with its switch off against everything on;
the numbers below come from the builds before the last refinements, and the full-game and fresh
runs of this round were cut short (fresh numbers for the round are those of the four rounds
together, below).

**What fired** (seeds 1-12, 300 turns, telemetry): 158 defenders trained for the rival term (445
for the rest of the quota), 349 sorties held back by the strength check; 157 wounded split off to
heal in our cities, 239 stack heals, 199 stacks walked back to heal, 337 times the old rule would
have stopped the whole stack; 12 opportunistic assault turns taking 3 cities (no enemy city was
ever found empty); 54 turns with a second operation (traced: two cities taken the same turn, seed
13); 16 strikes with a hero or veteran present, 30 of them held back; 23 fleet waits.

| behaviour | development, 300 turns (48), at turn 300 |
|---|---|
| rival defense | first version (thresholds in place of the war +1): legacy -2.6% (t -1.5), power -5.6% (t -2.4), civ cities lost +23% (t 1.7); kept (a pool of one defender per city at war, shared by war threat): legacy +0.2%, military +1.9%, civ cities lost +3% (t 0.25) |
| healing | More Naval AI's rule as is: civ cities lost +41% (t 2.7), military -6.4% (t -2.1) (docs/AI_LOG.md, "Healing only when most of the stack is hurt"); kept version: legacy -3.7% (t -1.1), research -9.7% (t -2.1), cities -5.2% (t -1.7), no single part of it the cause |
| captures | legacy +1.3% (t 2.3), cities +1.0% (t 2.5), pop +1.4% (t 2.8) |
| second operation | legacy +0.4%, civ cities lost -16% (t -1.8), civ cities captured 4.38 -> 3.92 a game (t -2.2) |
| heroes and veterans | legacy +2.5% (t 1.9), pop +3.2% (t 1.95), civ cities lost -25% (t -2.2), civ cities captured -18% (t -1.9) |
| assault fleet | 44 of 48 games identical, no effect at 300 turns; a full game traced (seed 6): futile landed assaults 23 -> 2 |

Integrating the four rounds exposed a deadlock in the fleet: with no room aboard and no transport
coming it still waited for half the force, so a port whose only transport carries one unit never
sailed. It now waits only while waiting can grow the load and otherwise ferries what it carries.

### Round 7c (2026-10-22): wars, diplomacy, councils, missionaries

Seven behaviours read from More Naval AI (Diplomacy above: peace by the forces at the front,
quiet wars end, dogpiles, the re-check before declaring, tech-trade discipline, councils,
missionaries abroad), each behind its switch and measured alone: the same binary with
`FFH_AI_<NAME>_OFF=ALL` against all on. Development: seeds 1-24 full games and 1-48 at 300 turns;
validation: fresh seeds 22201-22296 (full games, `--until-supremacy`) and 22001-22192 (300 turns;
22001-22096 for the dogpile, the discipline, the peace terms and all seven), never used to choose
anything. The development measurements ran on the build before the kept tech-trade discipline
(the same binary on both sides of each comparison). The dogpile and the tech-trade discipline are relative edges and
were also measured as one civilization with the behaviour against the rest without it
(`civ_ab.py --civ all`, seeds 1-4 and 22201-22204, 76 pairs each). Two first versions were
replaced (docs/AI_LOG.md, "Round 7c"). Legacy score and techs are per-civ means, t the paired t.

How often they fired, 96 fresh full games (telemetry): 280 of 1,160 war preparations were
dogpiles (the target's other enemies counted; 160 dogpile wars declared); 147 prepared wars
were dropped at the re-check (82 no longer takeable at parity, 63 on the power bar); 170
cash-outs were refused while our army stood at their next city and 120 quiet wars ended in peace
(against 307 cash-outs and 250 peaces wanted); tech swaps were withheld or refused for the
leader's 125% (3,010 checks, nearly all of the leader's swaps), the victory rival (814), a
wonder we were building (545), military technology at war (396) and military monopolies to
neighbours (83); one missionary went abroad.

Councils, same 96 games: the Overcouncil met in 51 games, the Undercouncil in 42; 1,362 yes
votes, 180 no, 35 abstentions, no defiance; every proposal but two passed (the head councilor
proposes only what its model of the votes says will pass), two were repeals. Resolutions passed:
forced war 91 (Overcouncil) and 39 (Undercouncil) in 30 and 23 games -- the head councilor
recruiting the council into the war it fights (the average gain of a forced-war proposal is
180: a war we are in counts 200) -- embargoes 48, forced peace 17, defensive pacts between all
members 25, open borders 11, the three mana bans 111 (Death 40, Entropy 35, Shadow 36), Liberty
42, Single Currency 46, Free Trade 46, Develop Secret Codes 35, Enlist the Nightwatch 25, Fund
Dissidents 30, Smuggling Ring 27, Gambling Ring 17; the head councilor seat 164 and 87 times.
With councils active, wars rise (below) and supremacy is rarer: a balance question for the owner,
since the councils now gang up on whoever they fight.

| switch (off -> on) | development, full games 1-24 | fresh full games 22201-22296 | fresh 300 turns 22001-22192 |
|---|---|---|---|
| all seven | legacy -4.0%/-4.8%/-4.6% at 300/400/500 (t -3.0/-2.9/-2.0), techs -4.4% at 300; wars +37% (t 3.6), captures +23% (with the first tech-trade discipline) | legacy +0.9%/+2.3%/+1.3% at 300/400/500 (t 1.2/2.0/1.0), techs +3.9% at 500 (t 2.1); wars +15% (t 2.2), captures -2%; supremacy 50 -> 52, cultural 35 -> 36, conquest 4 -> 2, domination 2 -> 1, time 5 -> 5 | 96 games: legacy +0.1%/+0.2% at 200/300 (\|t\| < 0.5); wars -13% (t -3.3), captures -3% |
| `PEACE_FORCES` | legacy -0.5%/-1.9%/-1.3% (t -1.8/-1.9/-1.1); wars -8%, captures +3% | legacy +0.4%/+1.0%/-0.6% (t 1.5/1.1/-0.6), techs +0.5% at 300 (t 2.5); wars -4%, captures -4% (t -1.3); supremacy 58 -> 52, cultural 29 -> 36 | 96 games: legacy -0.5% at 300 (t -1.9); wars -5% (t -2.1), captures +1% |
| `QUIET_PEACE` | legacy -0.1%/-1.1%/-3.2% (t -1.4/-1.3/-2.2); wars +4%, captures +5% | legacy +0.4%/-0.1%/-1.2% (t 1.8/-0.3/-1.6), techs -1.5% at 500 (t -1.5); wars +3%, captures -2% | (not run: 45 of 48 development games identical at turn 300) |
| `DOGPILE` | legacy -0.6%/-1.0%/+0.4% (\|t\| < 1), techs +3.2% at 500 (t 1.8); wars +20% (t 1.8), captures +3% | legacy -0.3%/+0.2%/-0.8% (\|t\| < 1); wars +13% (t 2.3), captures +4% (t 1.1); decided 13 turns later (t 2.2), supremacy 57 -> 52 | 96 games (81 identical): legacy -0.1% (t -0.2); wars +6% (t 2.3), captures +2% |
| `TECH_DISCIPLINE` | first version: legacy -3.7% at 300 (t -2.7), techs -4.5% (t -3.0) (AI_LOG) | legacy -0.1%/-0.4%/-0.4% (\|t\| < 1), techs -1.4% at 400 (t -2.3), +1.0% at 500; wars +10% (t 1.8); decided 11 turns later (t 1.8) | 96 games: legacy -0.1%/-0.3% at 200/300 (t -1.6/-1.6), techs -0.4% at 300 (t -2.0) |
| `WAR_RECHECK` | legacy -0.7%/+3.4%/+1.0% (t -0.5/1.5/0.4), techs +5.7% at 400 (t 1.6); wars +3% | legacy +0.5%/+0.8%/+0.7% (t 1.3/1.1/1.2), techs +2.0% at 500 (t 1.6); wars -19% (t -3.2), captures -11% (t -2.9; a city per war 1.11 -> 1.21); decided 15 turns sooner (t -1.7), time limit 8 -> 5 | 192 games: legacy +0.5% at 300 (t 1.2); wars -15% (t -5.5), captures -6% (t -2.2) |
| `COUNCIL` | legacy -0.9% at 400 (t -1.6), +0.1% at 500; wars +31% (t 3.6); supremacy 15 -> 10 of 24 | legacy +0.0%/+0.1%/+0.9% (\|t\| <= 1); wars +14% (t 3.3), captures -1%; supremacy 58 -> 52, cultural 31 -> 36 | (inert before turn 300) |
| `MISSIONARY_ABROAD` | 23 of 24 games identical | 1 missionary abroad in 96 games | -- |

One civilization with the behaviour against the rest without it (76 pairs each):

| | seeds 1-4 | fresh seeds 22201-22204 |
|---|---|---|
| `DOGPILE` | wins 15.8% -> 17.1% (1 / 0), share@end +22 (t 0.6) | wins 11.8% -> 11.8% (0 / 0), share@end -2 (t -0.1); 68 of 76 games identical |
| `TECH_DISCIPLINE` (kept version) | wins 18.4% -> 15.8% (0 / 2), share@end -77 (t -1.2) | wins 11.8% -> 10.5% (0 / 1), share@end -2 (t -0.1), legacy@end -30 (t -0.9) |

What the round shows. The bundle is neutral to slightly positive for the world on fresh seeds
(legacy +2.3% at turn 400, techs +3.9% at turn 500) and makes the full game more warlike (wars
+15%; before turn 300, when the councils have not met, 13% fewer): the councils' forced wars and
the dogpiles add wars, the re-check removes the ones that would not take a city. The first tech-trade discipline cost the whole world 4-5% of its technologies by
turn 300 and the civ using it its games; the kept version is flat for the world, and for the
civ using it 3 discordant games went to the base (p 0.25 over both sets), share at the end flat
on fresh seeds: a small, unproven cost of not feeding the runaway leader. Quiet wars ending costs a
little late (legacy at turn 500 -3.2%, t -2.2, development; -1.2%, t -1.6, fresh; Known gaps).
The re-check makes wars fewer and more decisive (a city per war 1.11 -> 1.21) and games end sooner. The peace terms
weigh the front without a measurable effect on the economy; they shift outcomes from supremacy
toward culture (58 -> 52 supremacy). Missionaries abroad almost never fire (shrines come late,
open borders are rare by then, a building outscores a gold a turn); the councils, inert before,
now meet in about half the games and pass nearly everything their head councilor proposes.

### Round 7d (2026-10-22): production and finance

Four behaviours from the More Naval AI comparison, each behind its own switch
(`crates/ffh-ai/src/switches.rs`; `FFH_AI_<NAME>_OFF=ALL` turns one off) and measured alone: the
same binary with all four on against it with that one off, development seeds 1-48 at 300 turns
(KEEP_UNITS on 1-96) and 1-24 full games (`--until-supremacy`), then fresh seeds used for nothing
else: 300 turns from 23001 (192 games for KEEP_UNITS, 96 for the others) and full games from
23201 (96 games, 48 for TOP_CITIES). Percentages are the change from switching the behaviour on;
t is the paired t over seeds (shown for the larger ones).

- **KEEP_UNITS** (Finance, "Unit support"): More Naval AI's disband rule, brought back by the
  owner's decision after it was reverted the same day, and an interior city that holds the
  barbarians it sees no longer trains another defender for them once units cost gold. The rule
  alone kept 5% more military at turn 200 and cost 3.9% of the research there; the build-side
  variants tried for that cost, and why the others were dropped, are in docs/AI_LOG.md ("More
  Naval AI's disband rule", point 4).
- **TOP_CITIES** (Cities, items 8 and 10): heroes only from the top third of our cities by
  production, a world wonder outside it only when quick.
- **WONDER_SLOT** (Cities, item 8; Citizens): the quick-wonder slot and the production emphasis
  on a top city's final push.
- **TROUBLE_REBUILD** (Finance): long builds give way to the economy building a city needs while
  the empire cannot research at its target share.

How often they fired (seeds 1-16, 300 turns, `FFH_TELEMETRY`, all four on against all four off):
military units disbanded 859 -> 258 (and 22 idle workers), defenders trained for the quota
1249 -> 1006 and second defenders 826 -> 552, while defenders trained for an overwhelmed garrison
rose 309 -> 467; player-turns with unit support above a tenth of the commerce and research below
target 849 -> 1504; 71 wonder-slot picks and world wonders built 51 -> 71; hero picks 47 -> 39;
344 long builds switched to an economy building in financial trouble (seed 3: Markets, Elder
Councils and Courthouses in place of Archery Ranges, Warrens, Granaries and army units).

Development seeds (KEEP_UNITS on 1-96; the others 1-48; full games 1-24). These were measured with
the production emphasis on every wonder; the emphasis was then cut to a top city's final push
(below), and the fresh seeds measure the final code.

| at turn 200 / 300 | KEEP_UNITS | TOP_CITIES | WONDER_SLOT | TROUBLE_REBUILD |
|---|---|---|---|---|
| legacy score | -0.9% (t -1.2) / -1.1% | -0.8% / +0.2% | -0.6% (t -2.6) / +1.1% | +0.5% / +0.4% |
| techs | -0.9% / -1.4% | -0.9% / -0.3% | -0.3% / +0.2% | +0.2% / +0.1% |
| research | +0.7% / -1.7% | -1.2% / -2.7% | -0.7% / -4.3% (t -1.5) | +0.3% / -2.2% |
| world wonders | | +3.3% / -2.0% | +14.8% / +45.1% (t 5.0) | |
| military / unit support | -1.0% / +1.6%; -1.0% / +0.9% | | -2.1% / -1.7% | -0.4% / 0.0% |
| full games, turn 300 / 400 | legacy -0.6% / -1.8%, research -4.3% / -5.2% | heroes -11% / -12%, culture -10% / -17% (\|t\| < 1.5) | wonders +37% / +26% (t 2.8 / 4.7), research -9.4% (t -2.5) / -5.2% | legacy +1.1% / +0.8%, techs +1.9% / +2.8% |
| full games, outcomes | cultural 10 -> 8, supremacy 12 -> 14 of 24; decided 9 turns earlier | cultural 5 -> 8; decided 42 turns later (t 1.7) | cultural 10 -> 8 | cultural 12 -> 8; decided 15 turns earlier |

On the code before KEEP_UNITS' build side (round 7d's first binary, seeds 1-48),
TROUBLE_REBUILD had measured research +4.1% (t 1.6), slider +5.4% (t 2.3), gold +11% (t 2.3) at
turn 300; on the final base the slider was -4.2% (t -1.7). The two disagree within noise.

**The production emphasis.** As More Naval AI does it, on every world wonder for the whole build,
it cost growth and commerce (seeds 1-48: population -1.3% (t -2.6) and commerce -1.3% (t -2.4) at
turn 200, research -4.3% at turn 300) for few hammers: the slot without the emphasis built as
many wonders (+43%) for population -0.1%, commerce -0.2%, research -1.4%. Only for a top city's
final push (the wonder done within the slot's horizon) it built the most (+46% at turn 300; full
games +48% at turn 300, t 3.9) with research -3.9% / -3.6% at turn 300 (300-turn / full games),
against -9.4% for the emphasis on every wonder in full games. The research the slot still costs
is the hammers that went into wonders instead of libraries (buildings -1.0% to -1.7%).

Fresh seeds, the final code:

| 300-turn games, at turn 200 / 300 | KEEP_UNITS | TOP_CITIES | WONDER_SLOT | TROUBLE_REBUILD |
|---|---|---|---|---|
| legacy score | -0.5% / +0.4% | +0.7% (t 1.9) / +1.1% | +0.4% (t 1.7) / +0.8% | +0.1% / +1.0% |
| techs | -0.4% / +0.7% | +1.0% (t 1.9) / +0.6% | +0.2% / +0.5% | +0.2% / +1.0% |
| research | +0.6% / +1.8% | +2.9% (t 1.7) / -0.4% | -0.2% / +1.3% | +0.1% / +3.0% (t 1.5) |
| world wonders | | -15.4% (t -2.1) / -3.8% | +10.0% / +47.8% (t 8.9) | -0.3% / -0.1% |
| military | +0.4% / +0.2% | +0.7% / +3.8% (t 2.1) | -0.5% / +1.4% | +0.8% / +2.6% (t 1.7) |
| cities | -0.7% / -0.2% | -0.4% / +1.1% | +0.2% / +1.1% | -0.1% / +1.7% (t 2.1) |

| full games, at turn 200 / 300 / 400 | KEEP_UNITS | TOP_CITIES | WONDER_SLOT | TROUBLE_REBUILD |
|---|---|---|---|---|
| legacy score | +0.3% / +0.7% / +1.9% | -0.7% / -0.4% / -0.2% | -0.3% / -0.3% / -0.6% | -0.5% / +0.2% / +1.3% |
| techs | +0.6% / +1.3% / +1.4% | -0.5% / -2.3% / -1.7% | -0.2% / +0.1% / +3.0% (t 1.8) | -0.9% (t -1.9) / +1.1% / +0.4% |
| research | +4.1% (t 1.7) / +2.4% / -1.2% | -1.3% / +0.9% / -3.8% | -1.7% (t -2.2) / -1.8% / -4.2% | -0.8% / -0.6% / +4.0% |
| world wonders | | -39% (t -3.4) / +8% / -5% | +4% / +36% (t 6.2) / +24% (t 6.8) | |
| other | military -1.4% / +0.5% / +6.9% (t 1.9) | heroes 0 / +0.5% / +12.6% | buildings -1.3% / -2.3% (t -2.1) / -7.3% (t -4.4); cities -4.2% at turn 400 (t -3.2) | gold +2% / +7% (t 1.7) / +8% |
| outcomes (off -> on) | cultural 34 -> 31, supremacy 54 -> 53, time limit 6 -> 10; decided 8 turns later | cultural 15 -> 15, supremacy 29 -> 29; decided 23 turns later | cultural 27 -> 31, supremacy 58 -> 53; decided 25 turns later (t 2.1) | cultural 26 -> 31, supremacy 55 -> 53, conquest 5 -> 2; decided 24 turns later (t 1.6) |

What the four cost, from the games:

- **KEEP_UNITS** no longer costs research where the rule alone did (turn 200: +4.1% in full games
  instead of -3.9%). Its cost is late: research -5.7% at turn 500 (t -1.3, the 40 games still
  running), slider -7.8% (t -1.8), following the wars the kept armies start (civs at war at turn
  400 +50%, t 2.1; military +6.9%, unit support +8.6%); more civ cities change hands (+11%) and
  more fall to the barbarians (+20%), and 4 more games reach the time limit. docs/AI_LOG.md ("More
  Naval AI's disband rule", point 4) has the variants tried and where the lever is (the war gate,
  peace terms, a garrison term for rival civilizations).
- **WONDER_SLOT**'s research cost in the development full games (-9.4% at turn 300, t -2.5) was the
  emphasis on every wonder, now cut to a top city's final push; on fresh seeds research is -1.8% at
  turn 300 and -4.2% at turn 400 (\|t\| < 1.3) for a third more wonders, the hammers going into
  wonders instead of buildings (-7.3% at turn 400, t -4.4) and, late, cities (-4.2% at turn 400,
  t -3.2; not traced). Cultural victories 27 -> 31 of 96.
- **TOP_CITIES** moves wonders from slow cities to fast ones: fewer early (-15% at turn 200 in
  300-turn games, -39% in full games) as the slow cities stop starting them, as many by turn 300;
  heroes were not fewer on fresh seeds (development full games: -8% to -12%, \|t\| < 1.5).
- **TROUBLE_REBUILD** is cheap and mildly positive at turn 300 (research +3.0%, cities +1.7%);
  in full games gold +7-8% and five more cultural victories, flat otherwise.

All four delay the decision by 8-25 turns (supremacy to culture). Not measured together against
the AI before them.

### Round 6 (2026-10-21): great people, explorers

Six fixes for play a good player would not make, found by tracing seeds 1-18; two kept, the rest
reverted (settlers under a held threat, allied barbarians, worker demand and stuck workers, and
after review the 20-turn scrapping rule and garrison quotas from lasting threats and from a
per-city record of the threats seen:
docs/AI_LOG.md, "Low-hanging fixes"). Each was measured alone against the same binary with it
off (seeds 1-48, 300 turns; the great people also in full games, seeds 1-24):

- **Great people** (`units::great_person_build`): before, every great person that could
  lightbulbed, and no AI ever built a shrine (0 in 16 games, 28 holy cities at turn 300; in three
  games to turn 520, 10 of 13 Great Prophets were born to a civ holding 1-3 holy cities). Now 65
  shrines in 24 full games; development: research +3% and culture +5% from turn 300, power +8%
  at turn 500 (t 2.1), techs -0.6 to -1.3% (|t| < 1). Engineers rushed a wonder 3 times of 10 in
  a traced game (the others found none in progress).
- **Explorers** (`units::explore`, `safe_step`): 10-21 explorers a 300-turn game were trained
  and nearly as many scouts killed, mostly by animals a few turns out; an explorer with nothing
  in reach turned itself into an army unit and production trained another. Seeds 1-6, 150
  turns: scouts trained 87 -> 65, killed 94 -> 82. Development: legacy +4.1% at turn 200 (t 1.8),
  techs +5.0% (t 2.3), +2.7% at turn 300 (t 1.0).

With the 20-turn scrapping rule (since removed), the three together against the same binary
with them off:

| | 300 turns, seeds 10001-10096 (96 games) | full games, seeds 10101-10148 (48) |
|---|---|---|
| legacy score | -0.7% at turn 200, -1.1% at turn 300 (t -0.5) | -3.7% at turn 200 (t -1.3), +0.2% at 300, +2.9% at 400, +8.1% at 500 (t 1.6) |
| techs | -0.5%, -2.0% (t -0.8) | -2.1% at 200, +3.7% at 400, +12.1% at 500 (t 1.9) |
| research / production | -4.3% / -2.6% at turn 200 (t -1.1 / -1.0); +0.6% / -0.2% at 300 | research +8.5% at turn 300 (t 1.6); production -5.7% at 400 (t -1.0) |
| power | -0.3% at turn 300 | -7.1% at turn 200 (t -2.0), +7.2% at 400 (t 1.0) |
| outcomes | | supremacy 22 -> 30, cultural 26 -> 18; decided 38 turns earlier (t -1.4); civ cities captured -23% (t -1.7) |

The two alone against master, on fresh seeds again:

| | 300 turns, seeds 11001-11096 (96 games) | full games, seeds 11101-11148 (48) |
|---|---|---|
| legacy score | -1.2%, -1.3%, -1.8% at turns 100/200/300 (\|t\| < 1.5) | +3.4%, +5.5%, +6.8% at turns 200/300/400 (t 1.3-2.0); -4.1% at 500 (t -1.3, 18 games) |
| techs / research | -1.9% / -1.7% at turn 200, -1.2% / -2.5% at 300 | research +13.8% at turn 200 (t 2.2); techs +9.6% at 400 (t 2.4); research -12.4% at 500 (t -2.1) |
| power / military | -2.6% / -3.1% at turn 300 (t -0.9 / -1.2) | power +15.1% at turn 400 (t 2.9), military +11% at 300-400 (t 2.1-2.5) |
| outcomes | | supremacy 26 -> 21, cultural 22 -> 27; decided 15 turns later (t 0.6) |

Neutral: the 300-turn games are flat, the full games lean positive to turn 400 and negative at
turn 500 (only the 18 games still undecided), and the two sets disagree at turns 200-300 within
noise. Kept as play that is plainly right (a holy city's shrine, an engineer's hammers in a
wonder, a scout that does not walk into the animals it has just seen), not as a measured gain.

### Round 4 (2026-10-18): event choice, Conquest, settler, hammers, worker flee, transport

Six changes from a diagnosis of the whole AI (one finder per dimension, each candidate re-checked
by a skeptic, every merge verified on reserved seeds against the tip it went on), measured together
against the AI before them on seeds none of the work used:

| | 300 turns, seeds 1101-1244 (144 games) | full games, seeds 1301-1332 (32) |
|---|---|---|
| legacy score | +8.0% (t 4.6) | +9.0% at turn 300 (t 2.4), +1.8% at turn 450 |
| techs / research | +5.6% (t 3.0) / +7.5% (t 2.6) | +6.5% / +8.5% at turn 300; research -10.5% at turn 450 (t -1.8) |
| population / production | +14.6% (t 7.2) / +11.2% (t 5.3) | population +14.2% at turn 300 (t 3.5) |
| cities / land / power | +4.2% / +3.8% / +5.5% (t 2.3-2.6) | power +14.7% at turn 450 (t 1.9) |
| civs eliminated; civ cities captured | -26% (t -1.9); -16.5% (t -2.1) | captures +26% (t 1.2) |
| outcome | | supremacy 22 -> 17, cultural 9 -> 9, last standing 1 -> 0, time limit 0 -> 6 |

The cost is decisiveness: 6 of 32 full games now reach the time limit, each a world of two or three
near-equal powers (the leader 34-51% of the world but only 1.04-1.32x the runner-up). No single
change causes it; leaving one out of the tip (same 32 games) gives time-limit games: without event
choice 4, without Conquest 3 (and legacy -4.0% at turn 300, t -5.2), without the settler fix 6,
without the hammers 6, without worker flee 3, without transport 5. It is the general effect of
civs that grow evenly meeting a war gate that wants a clear power edge (see "War conversion vs.
decisive wars" under Known gaps) and a cultural pursuit reserved for a clear front-runner. Endgame
play between equals -- a long war of attrition, a coalition against the leader, or a culture race
when no one is a front-runner -- is the next strategic gap.

### Round 3b: goal-directed research + victory pursuit together

Re-measured on fresh seeds (601-648 at 300 turns, 601-624 at full 690-turn games with
`--until-supremacy`) against master, both verified to survive and to be synergistic, so both were
merged:
- **beelines-fixed** alone (601-648, 300 turns): techs 22.1 -> 30.3, research 83.7 -> 147.9,
  legacy score 876 -> 1115, cities 6.9 -> 7.6, civs eliminated 16 -> 8; CPU flat.
- **victory-pursuit** alone (601-624, full games): actual cultural victories 1/24 -> 10/24, games
  decided ~31 turns earlier, at a late cost (research -17% at t500, legacy -2% at t690); early
  game (turn 300) unchanged.
- **Together** (601-624, full games): cultural victories **13/24**, decided earliest (avg 509 vs
  master 562, victory-pursuit 531), and the research victory-pursuit spent on the culture slider
  is restored by beelines (research at t500 251 vs victory-pursuit's 204, master's 245; legacy at
  t690 3073 vs 2907). Leave-one-out (cultural victories per 24 full games): master 1, beelines
  alone 8 (its stronger economy reaches the culture threshold by itself), victory-pursuit alone
  10, together 13 -- so beelines adds +3 over victory-pursuit alone and the planner adds +5 over
  beelines alone; each contributes. The goal-directed economy reaches the (achievable)
  cultural victory faster, so the two reinforce rather than fight. The combined research now
  weighs the plan's goal_bonus against the victory goal-tech bonus, so the AI no longer always
  beelines a single expensive victory tech (e.g. a rare Tower run); that is judged by the win rate
  above, not by a unit test (`tower_victory_makes_the_last_tower_tech_the_goal`).

## Victory pursuit (`victory.rs`)

Without it the AI never aimed at a victory: 19 of 24 full games (seeds 1-24, 6 civs, standard
map, normal speed, played to the rules' end at turn 690) ended on the time limit, the other five
were won by civs that got there by accident (3 cultural, 2 domination).

**Estimates** (pure functions of the game state, nothing cached; O(cities + techs) per civ): the
turns each civ needs for each condition of `CvGame::testVictory` (`CIV4VictoryInfo.xml`).
- *Cultural*: three cities at legendary culture (50000 at normal speed). Per city, the culture
  missing at its culture rate with the slider all in and the culture process (production to
  culture at 50%); the third city's turns. Before Drama frees the culture slider, Drama's
  research is added.
- *Tower of Mastery*: the Towers of Alteration, Divination, Necromancy and the Elements (800
  hammers each) and the Tower of Mastery (4000, Strength of Will): the missing techs'
  research (prerequisites included, the lesser towers built meanwhile in the four best cities),
  then the Tower of Mastery in the best city. FfH makes a building's `PrereqBonuses` AND
  requirements (`CvCity::canConstruct`): each lesser tower needs all four of its manas, 16 in
  all. The estimate is out of reach while the raw mana nodes in our cities' radius cannot supply
  the manas we lack (the AI does not trade for mana).
- *Domination*: land at 78% - 2 per civ team ever alive (66% with 6 civs) and people 25 points
  above the runner-up's share; once we hold half the land needed, the cities still to take at
  30 turns each. *Conquest*: when our power is twice all the rivals' together, their cities at
  30 turns each. *Altar of the Luonnotar*: only once the Exalted altar (six Great Prophet
  stages) stands, the final altar's research and hammers.
- Religious, score (no target score) and time victories are not pursued.

**Not pursued** (the owner's decision, 2026-10-21): the Tower of Mastery and the Altar of the
Luonnotar are never our goal -- boring solitaire victories, and the Tower needs a huge territory
anyway. Their estimates stay, so a rival about to win one is still a *rival* (below) and gets
stopped. Play is unchanged: 24 of 24 full games (seeds 1-24, 6 civs, standard map,
`--until-supremacy`) identical to the build before it (same checksums), and the planner never
picked either in 48 full games against the C++ AI.

**Plan** (`plan`, `plan_ours`): from turn 200 (scaled to the game's length; from 100 when the team
holds 1.5x the next team's share of the world) the condition with the fewest turns that fits
before the time limit; a cultural victory only for the front-runner (no other civ's estimate
under 4/5 of ours: only one civ wins it, the others' culture is research lost). A *rival* is
another civ whose own best estimate is within 60 turns and shorter than ours.

**Pursuit** (each hook tagged `victory-pursuit`):
- research: the goal's techs (the towers', Drama, the final altar's) count double plus 4000 and
  are charged at most 15 turns of research (a beeline);
- production (`victory::production`, after the garrison, the first army and the workers; before
  expansion when the goal is 30 turns away): the towers in the best cities (the Tower of Mastery
  in the best one; the rules let one city build each) and a caster when the towers' mana nodes
  need one; for a cultural victory, in the three cities nearest legendary, culture buildings and
  wonders by culture per turn of production, else `PROCESS_CULTURE`; the final altar where the
  Exalted altar stands;
- culture slider (`empire::sliders`): the lowest of 40/60/80/100% that brings the cities to
  legendary within 100 turns (and 10 turns before the time limit), else 100%, lowered in steps
  only when the treasury runs short. The finance logic reads research and culture together
  (`economy::spend_pct`): counting research alone, a civ going for culture took itself for
  bankrupt and disbanded its army;
- a Great Bard (great work, 4000 culture) goes to the one of those cities furthest from
  legendary; casters build the mana nodes the towers lack;
- defense (`victory::guard`): once the goal is within 90 turns (the others will come for it), the
  cities that carry it keep two more defenders;
- wars (`diplomacy::victory_war`): against a rival about to win (defensive power below 1.5x ours,
  reachable over land within 20 plots, a city we can take), and for a domination or conquest
  goal against the weakest civ we can reach, prepared without the leader's roll or grudge; the
  rival's victory cities (those nearest legendary, those with towers) are worth 3000 more to the
  siege campaign; these wars end only when lost (or after 80 turns).

**Flavor drive (the owner's decision, 2026-10-21; not implemented yet).** Conquest and domination
are a *drive* of the warlike, not a victory goal pursued once in reach: the Clan of Embers, the
Doviello, the Bannor, any other canonically warlike civilization, and every leader with
`TRAIT_AGGRESSIVE` (in the data: Charadon of the Doviello, Sheelba of the Clan, Tasunke of the
Hippus, Alexis of the Calabim, Kandros of the Khazad, Basium of the Mercurians, and the minor
leaders Duin, Weevil, Hafgan, Volanna, Mahon, Averax, Ouzza) keep asking what the easiest next
growth is -- a settler site, or the cities of a weaker civ nearby -- and take it, over and over,
whether or not it raises their win rate (the win-rate rule decides how well they do it, not
whether). Today the planner is civ-agnostic and in practice picks culture or nothing (conquest
needs twice all rivals' power, domination 66% of the land: neither qualifies before it is nearly
won), so this is an ongoing comparison of settling against conquering a weaker neighbour, from
the early game on, not a bias on the victory estimates.

Measured on full games (seeds 1-24, 6 civs, standard map, normal speed, to the rules' end;
`victory_check.py`); economy per alive civ, at turns 400 / 500 (one game was over by 500):

| | before | after |
|---|---|---|
| games decided before the time limit | 5 / 24 (21%) | 19 / 24 (79%) |
| by victory | cultural 3, domination 2 | cultural 18, domination 1 |
| decision turn (decided games) | 651 [608-679] | 603 [494-684] |
| games still on at turn 600 | 24 | 16 |
| legacy score | 1361 / 1911 | 1352 / 1877 |
| techs | 37.9 / 51.4 | 37.4 / 50.7 |
| cities / population | 7.2 / 8.0, 55.3 / 77.2 | 7.2 / 8.0, 55.3 / 75.9 |
| research (beakers/turn) | 158 / 249 | 145 / 197 (the culture slider: 70% -> 61% research) |
| civs eliminated; wars declared; civ cities taken by civs | 48; 175; 497 | 45; 175; 534 |

300-turn games (seeds 1-24) are unchanged but one (23 identical checksums: the planner rarely
acts before turn 300); CPU time per 300-turn game 13.9 -> 14.1 s (seeds 1, 2, 3, 5; about 47 ms
per turn, measured alternately on a shared machine).

How it got there (same 24 full games; decided / cultural + domination / mean decision turn):

| version | decided | by victory | turn |
|---|---|---|---|
| no planner | 5 | 3 + 2 | 651 |
| planner; culture slider the lowest that finishes before the time limit | 19 | 17 + 2 | 660 |
| + front-runner only, slider for a win within 100 turns, great works | 17 | 16 + 1 | 607 |
| + finance reads research and culture together, guard (kept) | 19 | 18 + 1 | 603 |
| kept, without the front-runner rule | 19 | 18 + 1 | 596 |
| kept, without the guard | 18 | 17 + 1 | 604 |
| kept, without the victory wars | 21 | 21 + 0 | 597 |

- The first version won at the deadline (turn ~680): the lowest slider that finishes in time.
- Pushing harder made the culture front-runner an easy target: a culture slider of 60%+ left
  research under 40%, which the finance logic took for bankruptcy (garrisons capped, field units
  disbanded, no new army), and the other civs' rival wars took its culture cities (seeds 14, 23,
  24). Reading research and culture together and guarding the victory cities fixed it.
- The front-runner rule is neutral on decisions (19 either way, 7 turns later with it) and keeps
  research: 21 instead of 26 civs on culture at turn 500, research +3%.
- The victory wars cost two decisions (seeds 9, 10: the culture front-runner was stopped, as
  intended) and turn one cultural game into a domination: 175 wars instead of 157, civ cities
  taken by civs 534 instead of 458. Kept: stopping a rival is the point.
- Before FfH's AND reading of `PrereqBonuses` was found, the Tower of Mastery looked reachable:
  the Balseraphs of seed 3 beelined every tower tech by turn 363 and then could build no tower
  for want of most of the 16 manas. The estimate now requires the manas, so no civ beelines
  the arcane techs for nothing.

## Looked promising on development seeds, failed on fresh ones

Each of these gained on seeds 1-96 and was re-measured on reserved seeds (801-944 at 300 turns,
901-916 full games) against the tip it would go on; none survived, so none was merged.

- **Promote every unit of a moving stack** (`units::play` promotes only the head of a group ready
  to move, and C++ `AI_doTurnUnitsPost` promotes every unit each turn: 37% of promotion-ready
  unit-turns are members behind the head). Seeds 1-48: civ cities captured +38% (t 2.1). Fresh
  seeds: captures -5.8%, legacy -1.0%, every |t| < 1.4; full games flat. With the Mobility value
  cut from 60 to 20 a move for non-explorers as well: seeds 1-96 captures +23% (t 2.3); fresh
  seeds legacy -1.7%, techs -2.7% (t -1.8), research slider -4.7% (t -2.4), eliminations +20%.
  The promotion gap is real but costs nothing measurable.
- **Value the building effects `building_value` ignores**: the Lighthouse's sea-plot food, the
  Harbor's and Inn's trade-route modifier, the Brewery's, Tailor's and Jeweler's free happiness
  resource (Lighthouses 2 -> 29 and Breweries 0 -> 15 over four games). Seeds 1-80 with the
  Conquest change: legacy +6-7% at turn 300 (t 3). Fresh seeds on top of the Conquest change:
  legacy +1.0% (t 0.9), eliminations +27% (t 2.2); full games power -5.8% at turn 300 (t -2.1),
  military -12% at turn 450 (t -3.6), civ cities captured -23%, cities lost to barbarians +68%,
  supremacy wins 9 -> 5 of 16. Alone, full games: wars -26% (t -2.2), captures -18%. The new
  buildings take production from the army, and the civs that build them lose cities.

## Cost control

- **Reachability**: `reach.rs` floods once from a unit (Dijkstra on movement points, bounded
  by a turn budget, unrevealed plots assumed passable) and scores every reached plot; only the
  chosen target gets a real path through the rules' `MISSION_MOVE_TO`.
- **City sites** (`sites.rs`): scored at most once per `update()` per player, only within 7
  plots of our cities (overseas: coastal plots within 14 of our coastal cities, only when
  the home landmasses are full), greedily picked with city spacing.
- Targets, peril and the road plan are computed once per turn per player.
- Measured with `ffh stats` (6 civs, standard map, 300 turns, 20 seeds): about 24 ms per
  turn on average (four games in parallel on four cores), two thirds of it inside the
  controller (including the moves it orders).

**The caster school (2026-10-27).** Measured for what the casters do, not for wins (the owner's
question: are the mages used better?), with `caster_ab.py --common` (each seed's two games up to
the last checkpoint both reached), the same binary with `FFH_AI_CASTER_SCHOOL_OFF=all` on the base
side, every civilization's arcane casters. Fresh seeds 7901-7940 (17950 game turns a side, 35 of 40
games changed): casters lost per 1000 caster-turns 4.6 -> 1.8 (lost alone 3.1 -> 0.5; in all 231
-> 96, the levels they took with them 972 -> 500), upgrades 177 -> 237, caster-turns +10%
(fewer trained, 485 -> 414: fewer die to be replaced), Mage-turns +29% (Archmage-turns even),
Adepts at a Mage's level waiting for the upgrade -14%, casts +15%, Channeling II and III summons
and damage per 1000 caster-turns 30.5 -> 35.5. Casts before strikes did not follow: 300 -> 277,
0.80 -> 0.66 per city the casters' civilizations took (375 -> 421); they come from a few war
games (two games that conquered less on the new side, 65 -> 35 and 88 -> 31; three others 0-7 ->
22-27), and a stack that marches on leaves the caster at the last sheltered plot behind it (seed
7930: the Amurite Wizards stood 5 or more plots from their strike target on 111 of 403 turns).
Development seeds 7501-7520 (two fixes found there: Adepts at level 4 without Sorcery went to the
front as Mages, and apprentices shuttled between cities): lost per 1000 caster-turns 7.8 -> 1.6,
Mage-turns +79%, Channeling II+ summons and damage 25.4 -> 62.5 per 1000 caster-turns, casts
before strikes 90 -> 195 (0.57 -> 1.01 per city taken). For context only: players with arcane
casters won 21.8% -> 21.8% (8 / 8 seeds won by one side only), share@end +130 (t 1.14), on the
fresh seeds; 12.5% -> 18.8% (4 / 1), share@end +269 (t 1.90) on the development seeds.
(Measured before master's caster corps, `casters.rs`, was merged in.)

**Casters in the stacks (2026-10-27).** The owner's proposal for the gap above: casters march in
the attacking stack's group and are almost never its attackers. Measured with `caster_ab.py
--common --base-env FFH_AI_CASTER_STACKS_OFF=all` (the same binary, the school on both sides).
Fresh seeds 9401-9440 (16950 game turns a side, 30 of 40 games changed): casters joined stacks
613 times; strikes by a player with a Mage had one within two plots in 33% -> 49% of them; casts
before attacks 145 -> 262 (per such strike 0.90 -> 1.47, per city the caster civilizations took
0.23 -> 0.39); Channeling II and III summons and damage per 1000 caster-turns 13.3 -> 21.5;
casters lost 114 -> 128 (per player +0.05, t 1.12; lost alone 43 -> 44), the Amurites, whose
Wizards march most, 4 -> 15 (t 1.96 over 16 players); upgrades 359 -> 377. Development seeds
9301-9340, where the first version (joining any committed group) lost casters a quarter more
often (144 -> 186) and the version kept did not (151 -> 153): a Mage within range 28% -> 42%,
casts before attacks per strike 1.37 -> 1.81, Channeling II+ summons and damage +35%; seeds
7501-7530 (the first version) 44% -> 53%. For context only: players with arcane casters won
22.2% -> 22.2% (9 / 9 seeds won by one side only), share@end -12 (t -0.34), on the fresh seeds.

## Tools

For A/B tests compare `legacy_score`, techs, cities, research and the like, not `score`: the
share score (docs/SCORE.md) averages 10000/N whatever the AI does.

**A behaviour every AI gets is not a player's win rate.** `ai_metrics.py` with a switch on for all
(`FFH_AI_<NAME>_ONLY=ALL`) averages legacy over every civilization, the dead ones as 0; a change
that makes every AI a better conqueror kills more civilizations and lowers that mean even when each
civilization plays better (round 7f: the fixed healing read -1.2% on 192 seeds, and its per-seed
change correlated -0.57 with the change in eliminations; the 35 seeds with more eliminations
averaged -139). Read eliminations and captures beside it, and measure a general behaviour as one
player's edge too: `civ_ab.py --civ all --new-env FFH_AI_<NAME>_ONLY={civ}` gives it to player 0
only, against opponents without it, and reports that player's wins and share.

**Forks first** (CLAUDE.md, "AI changes"): a behaviour is judged first at its firings.
- `tools/ai-branch.sh --bin $B --seed S --turn T --switch NAME [--turns N] [--trace P] -- WATCH..`
  plays seed S (as `ffh stats` does; run options after the script's own) to the start of turn T
  with the behaviour on, saves it, and resumes the save twice for N turns, the switch forced on
  (`FFH_AI_<NAME>_ONLY=ALL`) and off (`FFH_AI_<NAME>_OFF=ALL`). It prints the watched facts turn by
  turn for both, `*` marking the lines that differ, and with `--trace P` keeps player P's decision
  trace of each branch. Watches (`ffh resume --watch`): `city:X,Y` (owner, size, defenders and
  their health, defense damage), `player:P` (cities, units, army, power, research, gold, wars),
  `area:X,Y,R` (each player's units within R plots: count, health, types), `unit:ID`.
- Pick T a turn or two before the firing (the trace line's turn), the watches on what the
  behaviour should change (the stack's area, the city it besieges, the hero's id), N the
  behaviour's horizon. Combat is random and the dice part ways at the fork, so one fork can be
  luck: look at a few firings, and read the traces for why each branch went as it did.
- After a fix, `--save DIR/fork.ffhsave` (an earlier fork's save) replays the same position with
  the new binary, instead of playing to turn T again (the game before the fork changes with the
  fix when the behaviour fired earlier in it).
- Underneath: `ffh save FILE --to-turn T [run options]` and `ffh resume FILE --turns N
  [--watch SPEC]..` (`crates/ffh-cli/src/branch.rs`); a resumed save plays on exactly as the
  game it was saved from (same checksums).

- `ffh stats --seed N --at T1,T2,.. [--every N]` plays one game and prints one JSON line:
  per-civ snapshots after the given numbers of turns played (and always at the end), game-wide
  values at the same turns, whole-game events (captures by and from barbarians, wars, peace,
  eliminations), the outcome and the turn each civ died. Snapshots only read the game (the sync
  checksum is the same with or without them). Fields of a snapshot row (one per ever-alive
  player, barbarians included; all computed from game state at the end of that turn):
  - identity: `p` player id, `ctrl` (controller name: `ffh-ai`, `cppai`), `team`, `civ`, `leader`, `alignment`, `barb`, `alive`, `elim_turn`
    (turns played when found dead), `era`, `religion` (state religion), `vassal`;
  - size: `cities`, `pop`, `land` (plots owned), `land_scored`, `area_tiles`, `area_civs`
    (other civs on the capital's landmass), `overseas` cities;
  - score (top-level `score_version: 2`; docs/SCORE.md): `score` (share of the world in basis
    points, 10000 = all of it) and its shares `score_pop_share`, `score_land_share`,
    `score_know_share` (parts per million); `legacy_score` (FFH2 `calculateScore`, which was
    `score` in output without `score_version`) and its raw inputs `score_pop_raw`,
    `score_land_raw`, `score_tech_raw` (sum of era+1 over known techs), `score_wonders_raw` (5
    per limited wonder); Civ4 `power` and `assets`;
  - technology: `techs` known, `tech_cost` (beakers of known techs at this game's settings),
    `tech_progress` (beakers into unfinished techs), `research` (beakers/turn), `pct` (research
    slider), `research_commerce`;
  - economy: `prod` (hammers/turn), `food`, `food_surplus` (sum of city food differences),
    `yield_commerce` (gross commerce), `commerce` (gold + research commerce), `commerce_total`
    (net of expenses, plus culture and espionage), `gold`, `gold_commerce`, `gold_rate` (net
    gold/turn), `maint`, `ucost` (unit cost + supply), `civ_upkeep`, `culture` (total, /100),
    `culture_rate`, `espionage_rate`, `max_culture_level`, `legendary_cities`,
    `golden_age_turns`, `anarchy_turns`, `impr` (worked improved plots);
  - buildings and people: `wonders` (world wonders), `team_wonders`, `nat_wonders`, `buildings`
    (other buildings), `projects` (team project count), `holy_cities`, `gp_created`,
    `gg_created`, `gp_units` (great-person units alive), `gp_settled` (settled great specialists);
  - units: `units`, `mil`, `workers`, `settlers`, `animals`, `ships`, `heroes` (PROMOTION_HERO),
    `world_units`, `summons`, `casters` (adept + disciple units), `promo_ready` (units with a
    promotion they have not taken), arcane casters (unit combat
    Adept, summons left out: Adepts, Mages, Archmages and their variants) `arcane` alive,
    `arcane_mages` (Channeling II), `arcane_archmages` (Channeling III), `arcane_levels`, and
    cumulative `arcane_made`, `arcane_lost` (while the owner lives), `arcane_lost_alone` (of those,
    the ones that had ended the turn before outside our cities with no other unit of ours that
    fights), `arcane_lost_levels`, `arcane_lost_with_owner`, `arcane_upgraded` (a new caster of ours
    on its plot, of a type it upgrades to, with at least its experience), and caster-turns
    `arcane_turns`, `arcane_ready_turns` (no Channeling II at level 4 or more, not heroes: waiting
    for the upgrade), `arcane_mage_turns`, `arcane_archmage_turns`; `ucombat` (count by unit combat
    type, `NONE` for workers/settlers/great people), `levels` and `xp` (summed over fighting
    units); `mil_str` = sum over non-naval units that can fight of max(attack, defence) base
    strength x (100 + promotion combat %) / 100 x hit-point fraction, `mil_best` (strongest
    such unit), `naval_str` (same over ships);
  - happiness/health: `happy_margin` (sum over cities of happy - unhappy), `min_happy_margin`,
    `unhappy_cities`, `angry_pop`, `disorder_cities`, `health_margin` (good - bad health),
    `min_health_margin`, `unhealthy_cities`;
  - diplomacy and conquest (cumulative, from comparing city owners turn by turn): `met`, `wars`
    (alive civ teams), `taken_civ`, `taken_barb` (cities taken and kept), `razed` (cities that
    vanished with this player's units on the ruins), `lost_civ`, `lost_barb` (taken or razed by
    them), `lost_other`, `founded`, `cities_lost_rule` (Civ4 `getCitiesLost`), `players_killed`.
  Game-wide (`world_at`): `game_turn`, `alive_civs`, `war_pairs`, `armageddon` (counter %),
  `religions` (founded so far), `land_plots`, `estimate_end_turn`, and the legacy score's
  `max_*`/`init_*` (population, land, tech, wonders). Players can appear mid-game (the Mercurians join the team of the civ that built the
  Mercurian Gate; the Infernals and empires formed by events get a new team), so aggregate by
  `team`.
  - victory progress (alive civs): `towers` (Tower of Mastery classes owned), `tower_eta`,
    `culture_eta`, `domination_eta`, `conquest_eta` (the planner's estimates, null when out of
    reach), `legend3_pm` (the third-best city's culture, per mille of legendary), `altar_stage`,
    `mana_raw` (raw mana nodes in our cities' radius), `mana_types`, `culture_pct` (culture
    slider), `goal` and `goal_eta` (the planner's pick), `rival` (the player it would stop).
  `crates/ffh-cli/scripts/ai_metrics.py --seeds 1-12 [--every 10] [--out runs.json] [--compare
  old.json]` runs it over many seeds in parallel and summarizes (with the planner's goals and the
  victories before the time limit). With the agents' cache present it replays games already played
  from the shared run cache and takes a machine-wide CPU slot per game; `tools/agents/ab.sh` runs
  it on a base and on your worktree (docs/AGENTS.md).
- **Per-civilization measurement.** `ffh stats --civ CIVILIZATION_X` (repeatable: slots 0, 1,
  ...) forces civilizations into the first slots, each with a leader picked by the seed (the
  other slots stay random and avoid them; `ai_metrics.py --civ` passes it through). Every
  snapshot row has `world_spell` (the player has cast its world spell); `--detail` adds per-type
  counts: `unit_types`, `building_types` (cities with the building) and `project_types`.
  `FFH_AI_CIV_OFF=CIVILIZATION_X,...` (or `X`, or `all`) makes those civilizations play without
  their civilization-specific logic (`civs/`; the run cache keys runs by every `FFH_*`
  variable). `crates/ffh-cli/scripts/civ_ab.py --civ X|all --seeds S --new-bin N [--off-base |
  --base-bin B] [--out-dir D | --load D] [--at T,..] [--summary-json F]` plays (or reloads) the
  same seeds on two sides and reports the forced civilization only: win rate with the seeds won
  by one side only and an exact sign test, share and legacy score per checkpoint (a finished
  game's last value carried forward) with the paired t, survival, rank, world-spell use, the games
  that are identical on both sides and the per-type counts that differ; `--off-base` makes the
  base side the same binary with `FFH_AI_CIV_OFF` for that civilization (identical opponents).
- `crates/ffh-cli/scripts/caster_ab.py --bin DIR --seeds S --out-dir D [--base-bin DIR]
  [--base-env K=V ..] [--new-env K=V ..] [--common] [--civ] [--load]` plays the seeds on two sides
  (by default the same binary with `FFH_AI_CASTER_SCHOOL_OFF=all` on the base side) with the cast
  trace and reports how every civilization used its arcane casters: the `arcane_*` counters above,
  their casts by Channeling tier, context (before a strike, end of turn) and kind, and the rates
  per 1000 caster-turns; `--common` compares each seed's games up to the last 50-turn checkpoint
  both reached (a longer game has more caster-turns, and more casters lost with an eliminated
  owner).
  Generic behaviours have switches of their own (`switches.rs`): `FFH_AI_<NAME>_OFF=X,..|ALL`
  plays without it, `FFH_AI_<NAME>_ONLY=X,..` only for those civilizations (`ALL` or `1`: every
  one; names with or without `CIVILIZATION_`, any case). The module switches `FFH_AI_MAGIC_OFF`,
  `FFH_AI_CIV_OFF`, `FFH_AI_VASSAL_OFF` and `FFH_AI_TRAIT_CIVS` are read the same way
  (`switches::env_list`; `_ONLY` works for the first three too), and `FFH_AI_TRAIT_OFF` lists
  traits in the same form.
  **Implementation forks do not stay.** A switch whose off side is the algorithm a change
  replaced (not merely the behaviour absent) is removed, with the old algorithm, the round after
  the change is kept: later comparisons with the old version run against an older revision
  (`tools/agents/ab.sh --base <rev>`, `civ_ab.py --base-bin`), not a switch nobody plays and no
  test exercises. Feature toggles (off = the behaviour absent), flavor switches and default-off
  experiments stay. Removed so (2026-10-28): `KEEP_UNITS` (7d), `CASTER_ROLES` (7a),
  `RIVAL_DEFENSE` (7e), `HEAL` and `STRIKE_DAMAGE` (7f), `SIM_FAITHFUL` and `FIELD_SIM` (7g),
  `STRIKE_AT_PHASE` (7h); the rounds below name them as they were measured. A contest between
  civilizations (a land race, a war) is measured as one civilization with the behaviour against
  opponents without it: `civ_ab.py --civ all --base-env FFH_AI_<NAME>_OFF=ALL --new-env
  FFH_AI_<NAME>_ONLY={civ}` (the same binary on both sides; `{civ}` is the forced civilization).
  Measuring only with every civilization on can mislead either way: a faster opening for everyone
  cost research, and for one civilization against the rest it still gained nothing.
- `crates/ffh-cli/scripts/victory_check.py --seeds 1-24 [--out games.json | --load games.json]
  [--compare base.json] [--min-share 0.5]` plays full games (to turn 690 or an earlier
  victory) and reports the share decided before the time limit, per victory type, the decision
  turn, and the economy at turns 300-600; `--min-share` makes it a pass/fail check. A short
  version is the ignored test `run::tests::full_games_end_in_victories` (`cargo test --release
  -p ffh-cli -- --ignored`).
- **Head to head against the FFH2 C++ AI** (docs/CPPAI.md): `FFH_CPPAI=even|odd|all|<p>,..`
  makes those players play with the port of the original DLL AI (`crates/ffh-cppai`);
  `crates/ffh-cppai/scripts/h2h.py --bin $B/ffh --seeds S --turns 690 --until-supremacy
  [--handicap-ffh HANDICAP_X]` plays each seed with the seats swapped and reports both sides
  (snapshot rows carry `ctrl`, the controller that played the player).
- **Telemetry** (docs/TELEMETRY.md): `FFH_TELEMETRY=1` adds to `ffh stats` what each player
  did (missions, casts, hurries, upgrades, deals, wars, combats by odds, deaths by cause), which
  spells its units knew, could cast, left idle and cast with what effect, and why the AI acted
  (plans as they change, decisions with the rule that made them and the alternatives);
  `FFH_TELEMETRY_FILE=f.jsonl` writes every event. `crates/ffh-cli/scripts/telemetry.py`
  summarizes it per AI, civilization or player; `h2h.py --telemetry` adds it per side. The AI
  side reports through `Game::tel_intent`, `tel_decision`, `tel_cast_reason` and
  `tel_war_reason`; play is identical with telemetry on or off.
- `ffh autoplay --report T` prints, per civ, costs, unit roles, improvements, civics and every
  city's yields, trade, happiness and production; `ffh inspect <player>` shows each city's
  plots (worked, yields, improvements).
- `FFH_AI_TRACE=all|<player>` traces AI decisions to stderr; `FFH_LOG=all|<player>` prints the
  game log.
- **Spectators' flags.** Someone watching the AI in the browser (an observed game: docs/HUMAN.md,
  "Watching the AI") flags play that looks wrong; the flag's text has the build, the turn, the
  plot, the units there (ids, the AI's role, orders) and three ways to replay that turn with the
  trace on: `ffh-play observe --setup '<settings>' --to-turn T --plot X,Y` (from the settings),
  `ffh-play observe --load <pinned save> --to-turn T` (from the start of turn T: one turn of
  work), and the same game in `ffh autoplay ... --turns T+1`. Check out the flag's build first:
  the same seed plays another game once the AI changes. The trace lines of that turn are tagged
  `[tT pP]`; `observe` prints the checksum of each turn (the flag's is the last) and the units at
  the plot, so a fix can be checked on the same situation from the pinned save.
- `--speed GAMESPEED_X`, `--handicap HANDICAP_X` (the game handicap: the AIs' bonuses),
  `--era ERA_X`, `--climate CLIMATE_X`, `--sea-level SEALEVEL_X`, `--max-turns N`,
  `--no-victory VICTORY_X` (repeatable) and `--leader LEADER_X` (the first slot's leader) give
  `ffh autoplay` / `stats` / `bench` the rest of a custom game's settings, so every game a
  spectator watches plays here too.
- `--option GAMEOPTION_X` (repeatable) turns game options on for any of these commands (for
  example `GAMEOPTION_ADVANCED_START`).

## More Naval AI compared (2026-10-22)

More Naval AI (Tholal's FfH2 AI: FfH 0.41's C++ AI plus Better BTS AI, parts of K-Mod and
Tholal's own changes; source at sourceforge.net/p/tholalsffhaimod/code, r1832) was read area by
area against this AI. Its disband rule was tried and reverted, then brought back by the owner's
decision with a build side to match (round 7d; docs/AI_LOG.md, "More Naval AI's disband rule"),
and fog-busting, which it does not do, failed here ("Fog-busting posts").

Where this AI is ahead, with measurements: victory planning (MNAI's culture victory is switched
off, "the AI doesnt know how to pursue a culture victory"; its conquest stages are weighted dice),
research (goals and lookahead against MNAI's depth-3 value plus a random 500), production
allocation and finance (MNAI's treasury floor sets research to 0 below about 100 gold, the spiral
our finance module prevents), the one-operation war campaign, war staging and reparations,
spellcasting and the civilization mechanics (MNAI special-cases a handful of heroes and spells),
exploration. Not to borrow (tried here and lost, or worse here): a unit-spending cap, gold
reserves for upgrades, two escorts per settler, random war rolls, the Tower and Altar pursuit (the
owner's decision), the power ratio alone for Sanctuary and March of the Trees (round 7a). Its
caster roles, summons spent at any odds, mana nodes and Govannon's teaching were borrowed in
round 7a, each in the form an expert would play (a smaller corps, summons that keep covering and
leave cities' defenders that heal before we strike).

Where MNAI does something this AI does not, ranked by expected worth (each untried unless noted):
1. **Mana nodes as an economy** (done, round 7a: `mana.rs`). One adept per civ builds nodes on
   our raw mana (`CvUnitAI::AI_upgrademanaMove`), the type chosen by the bonus's effects and the
   spells its sphere opens; research values node techs by the raw mana we own.
2. **Defense weighed by rival civilizations.** `CvCityAI::AI_cityThreat` scores each rival's
   closeness to the city times war/war plan/attitude times its power over ours, and spreads a
   pool of floating defenders by it; sorties need 175% of the enemy's local strength
   (`AI_leaveAttack`). Here the quotas read units in sight only; ffh-ai lost 12 cities a game to
   civs against the C++ AI's 7 (docs/CPPAI.md), and AI_LOG's Clan entry names this term. Measure
   one civ against the rest (`civ_ab.py`); the garrison entries in AI_LOG warn about the cost.
   *Done in round 7b* ("City defense against rival civilizations"; Round 7b below).
3. **Peace terms that weigh our forces** (`CvTeamAI::AI_endWarVal`: the value of peace divided
   by our attackers en route against theirs, down to a third; a war with nothing moving ends).
   AI_LOG "Wars" names this lever: sell the war when the next city is out of reach. Done in round
   7c (Diplomacy, "Peace by the forces at the front", "Quiet wars end").
4. **Healing.** MNAI's `AI_heal` stops a stack in the field only when over half of it is hurt,
   and only where healing is above 10% (not in enemy or neutral land); wounded units in our
   cities split off. Here one unit over 35% damage halts the whole stack where it stands
   (`military::army`), committed stacks included, next to defenders that heal faster.
   *Done in round 7b*, fitted to our small stacks (Field armies, step 6).
5. **Heroes and wonders in the best cities.** MNAI builds world wonders and heroes only in the
   top third of cities by production. Done (round 7d): heroes there only, world wonders outside
   them only when quick, and MNAI's quick-wonder slot ("Cities", items 8 and 10).
6. **Council votes.** MNAI votes and proposes (`AI_diploVote`, `AI_chooseElection`); this AI
   abstained on every vote and never proposed, so the Overcouncil and Undercouncil were inert.
   Done in round 7c (`council.rs`; Diplomacy, "Councils"); a balance question too.
7. Smaller, all done: re-deciding production when the empire enters financial trouble (round 7d,
   Finance); dogpile wars on a civ already at war, `AI_doWar`; no tech trades with the victory
   rival, no military techs while preparing a war, keep monopolies, `AI_techTrade`; missionaries
   abroad once the shrine stands (round 7c); a stack next to an empty or weak enemy city outside
   the plan takes it, `AI_stackAttackCity`; a second operation for a clear surplus; assault
   fleets wait for the landing force and sail escorted, `AI_assaultSeaMove`; heroes and veterans
   attack only at their odds, `AI_getBestGroupAttacker` (round 7b); Govannon's Teach
   Spellcasting (round 7a). Sanctuary and March of the Trees at 1.5x the enemy's power: tried,
   off by default (round 7a).

## Known gaps (future work)

- Leader traits (`traits.rs`): Charismatic, Creative, Defender, Organized and Magic Resistant have
  no play of their own (their effects reach every decision that reads the game). Candidates if
  they should: Defender stacks counterattacking inside our borders (Homeland), Creative leaders
  leaning to the cultural victory, Charismatic ones promoting for the faster levels. Raiders
  pillage where they stand and raid nearby; they do not yet send fast units deep into enemy land.
- **Fewer cities change hands with the faithful simulation (round 7g).** With `SIM_FAITHFUL` and
  `FIELD_SIM` every civilization took 14% fewer civ cities (t -1.8) and 11% fewer barbarian towns
  in 300 turns, and declared 11% fewer wars, at no cost to anyone's win rate: the AI no longer
  strikes on an estimate that was optimistic, and waits for a force that takes the city. Round 7h
  recovers part of it (strikes at the strike phase's odds for all; 60% for the conquerors, whose
  bolder bar took 8% more cities when every civilization had it); striking at lower odds did not
  make a player win more, so the rest is a flavor choice. Faster force building for the
  conquerors is the other lever.

- **Councils gang up (round 7c).** With `council.rs` the councils meet in about half the games
  and pass nearly every proposal; a head councilor at war has the council declare war on its
  enemy (in 96 full games, 91 forced-war resolutions passed in 30 games of the Overcouncil, 39
  in 23 of the Undercouncil), and supremacy is rarer (58 -> 52 of 96
  against councils off). The AI plays the rules as they are; whether forced wars should be this
  cheap to pass is the owner's balance question. Not done: a member that would be dragged into a
  war it cannot fight leaving the council (changing its membership civic) before the vote.
- **Quiet wars end, then a new one is prepared (round 7c).** `QUIET_PEACE` ends wars where
  nothing happens; late in full games the economy is a little lower with it (legacy at turn 500
  -3.2%, t -2.2, on development seeds; -1.2%, t -1.6, on fresh ones). Untested hypothesis: the
  peace frees the one-war-at-a-time gate, and the civ prepares (builds an army for) another war
  where the phony one cost it nothing.
- **Vassals never leave.** A vassal that has recovered stays one. FfH's AI cancels the agreement
  when `AI_vassalTrade` would now refuse it. Masters do nothing for their vassals beyond the war
  joining the rules impose (Diplomacy, "Vassal states").

- Naval: amphibious invasions now ship an army across water to take an overseas enemy city
  (see Naval), but there is no naval combat and no blockade, escorts are only the idle warships
  that happen to be in the port (none are built for the fleet), and early transports carry one
  unit each, so the fleet waits in port until enough of them are built and invasions are slow
  until good transports arrive late in the tech tree. Overseas settling still sends one settler at a time.
  The trickle no longer scatters and no longer under-ships: each trip lands on the reachable plot
  already holding the most of our army (they pile onto one plot and strike), and the staging port
  builds a fleet big enough to carry the force in one wave. Traced water-isolated civs that used to
  land repeatedly without ever assaulting now take overseas cities; 4 of 5 such civs convert and
  held overseas cities rose ~20% at a fixed turn (8-game A/B), economy flat.
  The remaining hard case is a civ whose *main army is on a different landmass from the staging
  port* -- an isolated civ (e.g. Doviello, seed 5) whose only target sits beyond its home coast's
  sea-range, reachable only from a small forward colony that cannot build transports and holds
  little army. `committed` only musters army already on the staging landmass, so the home army
  (3000+ idle stack-turns) never joins. Two simple fixes were tried and both measured worse, so
  both were reverted: (a) letting *any* coastal city build the assault transports (so the fleet is
  built and the colony's few units cross) made civs start more overseas wars they still cannot
  convert -- captures -8/-9%, economy -7% at t690; (b) staging from the port on the landmass that
  holds the most of our army (ship the home army directly, however far) got the home army aboard
  (boards 1->11) but the home->target journey was so long the transports sailed all game and
  delivered almost nothing (333 sail-actions, 3 landings, 0 assaults) -- the target is simply too
  far from home for direct shipping, which is why the forward colony exists. The real fix is the
  original DLL's approach: classify the home landmass `AREAAI_ASSAULT_MASSING`, commit the home
  army, and ferry it in *two hops* (home -> forward colony, then a short colony -> target assault),
  massing at the colony before striking, sized so the landed force can take the city and gated so
  it does not bankrupt the economy. That is a multi-hop logistics pipeline (a new transport mode
  plus cross-landmass `committed`), a dedicated feature rather than a staging tweak.
- War conversion vs. decisive wars: civs with a real army but no reachable *takeable* target sit
  idle (traced `big army / took=0` civs, e.g. Sheaim seed 1: ~71 power of army, exactly one war
  prepared all game, 2283 unit-turns "0 targets"). The dominant land-war gate is `can_take_cities`
  (diplomacy.rs): it clears a war only when our best *trainable* attacker reaches the enemy's
  weakest city defender x1.2 (the fortification margin). Tracing the greedy war loop across seeds
  1-3 (all civs) found this gate rejects ~700 land-reachable (<=16 plots), already-power-parity
  targets -- ~8x the wars that pass -- and none of those rejections are catastrophic per-unit gaps
  (worst ~0.6x; 44% are cases where our unit already *beats* their defender, blocked only by the
  x1.2 margin). Hypothesis: the x1.2 "solo the fortress" bar is wrong because a war is declared
  only after massing `army_needed_for_war` (4-14) attackers, so a stack should take a fortified
  city at per-unit parity. Dropping the no-siege multiplier 120->100 (attacker >= defender) was
  measured (16-seed A/B, full 690-turn games): wars_declared +47%, but net territory flat (final
  cities 838->816), biggest empire flat (avg 22.8->22.1), economy 2-8% behind at *every* fixed
  checkpoint t200/t300/t450, and the +16% final "power" was standing military that never converted
  (NEW games ran *longer*, 6 vs 4 reaching t690 -- grinding, not decisive). Reverted: the per-unit
  bar turns out to select for *decisive* wars; the near-parity fights it filters out grind and
  mutually destroy power (seed 3: the relaxed gate flipped the winner and left the victor with half
  the base winner's power), an arms race rather than domination -- and over-militarization is a BTS
  pathology this AI otherwise avoids.
  A second attempt (v2) built the "decisive-force" gate that reasoning implied: SELECTION loosened
  to per-unit parity PLUS a numbers requirement (the force we will field overwhelms the softest
  revealed city's defender *count* with a reserve, via the campaign's own `rough_need`/`margin`),
  and a new DECLARE backstop that commits a prepared war only when the *real gathered army*
  decisively covers a visible target city -- reusing `combat_sim::covers`/`campaign::force_on`, the exact
  predicate the march logic uses, so a declared war is one the army would actually march on. This is
  the numbers-aware, truthfully-computed gate v1 lacked. It also measured net-negative (16-seed A/B):
  wars +15% but captures-per-war -19% (wars LESS decisive, not more), economy -8% at t300 worsening
  to -9.3% at t450 (a *growing*, sustained drag -- not one-time war-prep cost -- so an affordability
  pre-gate would not recover it), games longer (7 vs 4 reaching t690), eliminations +30%. The only
  gain was domination *concentration* (biggest empire +10%, top-power share +14%), but it came from
  a more war-torn world that suppressed everyone's research. Reverted.
  Conclusion from both attempts: the war-DECLARATION gate is not the lever. Wars are dynamic -- the
  enemy mobilizes and reinforces once war is declared -- so any declaration-time snapshot (per-unit,
  numbers, or real-army-covers-the-opening) cannot guarantee the *whole* war is decisive; the
  opening advantage erodes into a grind. The baseline's standing gross-power-ratio + per-unit bar is
  a near-optimal heuristic for a declaration gate precisely because it demands a *persistent* edge.
  If this pathology is pursued again, the lever is the campaign/prosecution layer (sustain local
  force superiority through a war, concentrate faster, pick targets that stay takeable) or the
  one-war-at-a-time constraint (diplomacy.rs) -- not the decision to declare.
- Sparse worlds (2 civs on a standard map): barbarians and their cities dominate the empty
  land; expansion is slow because settlers wait until no enemy is near their city. Letting
  them go out under a held threat founded more cities but lost many more to the barbarians
  (measured), so the gate stays.
- Expansion is slow in the first 100 turns (about two cities; second city at turn 70, third at
  105): a settler costs 120 food and hammers, 6-9 a turn from a size-3 capital, and cities wait
  for size 3-4 and for the barbarians to leave. Faster openings cost research by turn 300, for
  everyone and for one civilization against the rest (measured again 2026-10-21, an expert
  opening in four parts: docs/AI_LOG.md, "Expansion: the expert opening"); the levers left are
  commerce per new city and clearing the lairs that block expansion.
- Economy: research at about two thirds of the commerce by turn 300 (145 beakers a turn, 29
  techs); maintenance and civic upkeep take a quarter of the commerce. The first 100 turns are
  slow whatever the research order (6-7 techs at turn 100 with two cities).
  The civs still stuck at low research expanded to 7-8 cities in Despotism before reaching
  Cartography: only the settler gate (`finance::city_affordable`, not wired in) prevents that,
  and gating expansion costs cities (no better score by turn 300).
  A diagnosis (instrumented Finance + per-city builds) found research is cost-limited, not
  slider-limited (`empire::sliders` already maxes research subject to solvency), and the dominant
  cost is unit support (35-48% of costs at t300), while research-multiplier buildings are starved:
  at war + army-short the production ladder returned a military unit every turn and never reached
  the building step, so libraries were built in ~1 of 6-8 cities and beakers = commerce x slider
  with no multiplier. Fix (`production.rs` decide(), the "economy-before-the-army-tail" gate): a
  high-value economy building (identified data-drivenly by its gold/research commerce + specialist/
  GP/trade value, `is_economy_building`) now preempts the surplus-army *interleave* -- but only for
  a civ whose finances are actually strained by upkeep (`Finance::support_heavy` or `structural`),
  so the research-suppressed laggards get relief while healthy conquerors keep their army. It is
  strictly downstream of `army_choice` (the operation's real needed force) and every garrison, so
  only the non-converting attacker tail is deferred; nothing is disbanded. Measured (16-seed A/B):
  research@t300 +4%, final power +12%, biggest empire +3% and top-power share +10% (domination up),
  net territory and eliminations flat. A broader variant (gate on any economy building, no finance
  condition) gave more research (+12%) but cut captures ~25% and shrank the top empire -- it
  dampened healthy conquerors, so the upkeep-strained condition was kept.
- Sieges: one operation at a time. At the declaration the army is large (23 units against a
  computed need of 10, 24 full games, 2026-10-21) and the committed stacks stay together (3-6
  stacks, striking together, in the traced wars), but a war we declare takes a city 38% of the
  time; about 40% of them end in our own peace for tribute at turn 11, which pays better than
  fighting on (refusing it cost wins, docs/AI_LOG.md, "Wars"). The lever left was the operation's
  own estimate: sell the war when the next city is out of reach, fight on when it is close (round
  7c weighs the army at their next city, Diplomacy, "Peace by the forces at the front"). A
  barbarian town of 5+ archers is out of reach and is left alone; casters cast before the strike but do not attack
  ("Spellcasting"); a city emptied of defenders is entered even when a counterattack will retake
  it. Tried and dropped (no measurable effect on seeds 1-48): raiding (pillaging) while a war
  operation gathers, a "can we hold it" check before the strike, every unit following the victor
  into the city, stacks marching on a barbarian town turning back for cities in peril (fewer towns
  taken, no fewer cities lost), barbarian targets out to 20 plots, a larger strike margin.
  One prosecution defect WAS fixed: the Strike gate fired off the adjacent ring alone
  (`ok_adj = covers(adj, spare=false)`) while a committed stack one plot out (dist 2) was not yet
  in the fight (`strike()` pools only units a single plot from the city), so the adjacent force
  struck at acceptable odds and bounced, then re-mustered (16 "steps back" for one civ in seed 3).
  `build_op` now defers a MARGINAL strike one turn (Strike->StepIn) when a committed dist-2 stack
  can reach an adjacent plot that turn and the combined force would then take the city with a
  reserve -- strictly one-turn-bounded (a stack that cannot close now is not counted, so a
  hardening city is still struck at acceptable odds), and it only ever downgrades Strike to StepIn
  so it can never make a strike weaker. Measured (16-seed A/B): "steps back" ~23->5 and razed
  cities -29% (captures kept intact, not destroyed), final power +4.6%, eliminations -5.7%,
  research flat; but gross captures-per-war was flat (-1.9%), so this is a coherence/retention
  improvement (the army stops striking piecemeal and keeps more of what it takes), NOT a
  conversion win -- the outcome deltas other than razing are within 16-seed noise.
- Loose stacks exposed to a stronger enemy (the classic Civ4 "a stack parked in front of
  something that one-shots it"). Measured with a gated diagnostic (`FFH_AI_EXPOSURE`, which flags
  army stacks standing where a single visible enemy can reach and out-fight their best defender
  next turn): 230-450 *distinct* such stacks a game (4 games x 690t), ~90% with enough attackers
  to wipe the stack, a tail at 3-7x strength, concentrated in the mid/late war era (t250+), not
  early-game barbarian noise. It is entirely *loose* stacks -- stacks in an operation are never
  exposed, because `campaign.rs` keeps them concentrated -- so the gross cases (a lone unit next
  to a death stack, attacking into a stronger defender) are already handled by the odds-gated
  attack logic and the idle-retreat-to-city fallback; this is the residue. It looks bad but is
  not costing the AI: making an outclassed loose stack fall back to the safest plot it can reach
  (a city, or a plot where it is not outclassed with terrain credited), strictly after every
  attack/garrison/heal/city-defence/escort/operation/hunt step so offence is untouched, measured
  clearly NET-NEGATIVE on the primary goal (16-seed, 690t A/B): final power -8.1%, biggest empire
  -9.2%, top-power share -10.8%, techs@450 -7.3%, games longer (5 vs 3 reaching t690, i.e.
  grinding), for only +4% captures per war; and it barely moved the exposure itself (-18% over
  four games, worse in two of them -- the stack retreats toward a safe haven it cannot reach in
  one turn, so it ends the turn on another exposed plot and churns). The exposed stacks apparently
  do useful work -- pressure, map presence, attrition -- that retreating removes, so pulling them
  back makes the AI timid, the same over-caution the war-declaration and strike-hold experiments
  hit. Reverted; the `FFH_AI_EXPOSURE` counter is kept as observation tooling. A fix that helped
  would have to avoid *creating* the exposure -- danger-aware movement that ends each turn on a
  safe plot -- without making stacks passive, which is a pathfinding change, not a retreat rule.
- No hurrying beyond the civic-gated gold/population hurries and chops for settlers, workers and
  world wonders, no wonder racing.
- Speed: path finding is about 30% of late-game time.
- Measured on branches never merged (built on the master of 2026-10-15, not re-measured on fresh
  seeds; expect conflicts): `origin/handoff/ai3b-siege-estimate` (exact strike odds over the
  defender's health states, `odds.rs`, instead of the expected-value simulation: strike success
  82%->88%, units lost in failed strikes 116->51 over 96 seeds, economy flat; the war-execution
  lever the war-conversion gap above points at) and `origin/handoff/ai3b-kuriotates-tuning`
  (Kuriotates settlements without the research collapse: +54 legacy score, t 2.3, over 80 games;
  possibly superseded by `civs/kuriotates.rs`).
- Victories: nearly every decided game is a cultural victory. The Tower of Mastery needs 16
  manas, which the AI neither seeks in trades nor conquers for (and it does not pursue the Tower
  any more, the owner's decision); Great Prophets lightbulb instead of building the six altar
  stages; domination needs two thirds of the land and the siege operations take about one city
  per war; the religious victory (80% of the world's people) is not pursued.
- Casters ("Spellcasting"): a small corps by role and mana nodes on our raw mana (round 7a).
  Not tried: casters for a lasting summon (the Sheaim's Raise Skeleton, an army unit a turn while
  at war), magic earlier in the game (research values the first caster unit with a flat bonus and
  the node techs by our raw mana, not Knowledge of the Ether by the spells our mana would give a
  caster; it comes around turn 230), a caster's sphere chosen for its role (a terraformer is a
  caster that happens to know Spring or Scorch), and trading for the mana a sphere needs. A
  lasting summon is an army unit with no plan of its own; world spells keep their own timing
  (`world_spells.rs`), and the defensive ones (Sanctuary, March of the Trees) wait for FfH's two
  wars: a losing war was the wrong moment without an estimate of the invasion coming
  (docs/AI_LOG.md). Civilization mechanics handled and tried: "Civilization-specific"
  and docs/AI_LOG.md (Infernal and Mercurian projects, Khazad vaults and Armageddon rituals have
  no logic; the Sheaim's Planar Gates are valued by their creatures, "Cities", effects in the
  rules' code).
  Casters march to the front in the stacks (`CASTER_STACKS`); a player with two operations at once
  spreads them over both, and a stack too small to join leaves them on their own, so half the
  strikes still find no Mage within range.
