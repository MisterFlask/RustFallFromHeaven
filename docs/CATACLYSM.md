# Cataclysm Mode

Cataclysm is a rebuilt alternate ruleset game mode for apocalyptic fantasy 4X + Ogre Battle 3×3
formation combat. It replaces the Civ4/FFH2 rules engine and AI underneath with clean, deterministic
Rust crates while reusing 100% of the existing visual art stack (HD-2D WebGL2 renderer, isometric
fallback, 444 unit sprites + idle/battle animations, 248 leader/minor-leader portraits, and
`battle_watch.js`).

## Crate Layout

```
crates/ffh-battle      Pure zero-dependency 3x3 Ogre Battle formation combat engine
                       (shared by both classic ffh-core and cataclysm-core)
crates/cataclysm-core  Deterministic command-in / event-out 4X rules engine
                       (depends ONLY on ffh-battle; std-only, WASM-safe)
crates/cataclysm-ai    AI player controller built strictly on cataclysm_core::Command
                       (+ the `cataclysm-tally` bin: AI-only games, counted)
crates/ffh-play/src/cataclysm.rs
                       Transport adapter mapping Cataclysm state into the existing
                       14-layer MapRows, HD-2D/ISO stand positions, and battle_log v2
crates/ffh-play/assets/app/cataclysm.js
crates/ffh-play/assets/app/cataclysm_screens.js
                       Browser UI for New Cataclysm setup, 4-currency top bar,
                       interactive 3x3 Squad editor, Macro City districts, Doctrine
                       tree, Great People Court, and Regional Houses / Apocalypse Clock
```

## Setting: The Failing Worldseal

An age ago, the Great Empire built a heat-adapted subterranean caste of furnace-servitors to run the geothermal works powering its golden age. When the servitors rebelled, the Empire's emergency engineers dropped a sealed stone-and-brass plug-citadel (**Caldera Keep**) into the central volcanic basin and flooded it, trapping the rebels beneath the sea.

Now the **Worldseal** is failing. Pressurized vent-ether (**Mana**) leaks through the crust; deep smelting, vent-tapping, razing, and Lifeboat construction all drive **Worldstrain** toward `100`. Every playable faction is a surviving department or dissident offshoot of that dead Empire:

- **Seventh Garrison** (*Commander Maren Vance*): The unrelieved rim-watch legion (`+15%` front-row damage; free-upkeep, double-credit Crusade pledges).
- **Keepers of the Seal** (*Abbot Oren Hale*): Emergency engineers turned maintenance monks (`Vent the Seal` moves `-3 Worldstrain`, others `-1`; `+25%` Sentinel HP; automatic Turn-160 win if Worldstrain stays below 100).
- **Tappers' College** (*Dean Halden Pell*): Imperial power-scholars piping vent-ether through ceramic towers (`+2 Mana/Academy`; `+20%` battle spell damage; starts with `Arcana`).
- **Postriders** (*Factor Bajan*): The imperial highway and relay-stable network (`+1` open-ground movement; `+50%` Raid Gold; `-25%` reinforcement Gold).
- **Three Hearths** (*Ward-Heir Corin*): Imperial urbanists capped at 3 six-district megacities (`+50%` output & Culture), fed by low-upkeep Outposts.
- **Vault & Crucible** (*Master-Assayer Bram*): Mortal foundry-guilds requiring full bullion vaults (`>= 200` Gold/city) to fire deep furnaces (`+35%` Industry & `+4` Armor; `+25%` Siege Bombard damage).
- **Canopy** (*Mistress Sylva*): Dissidents who abandoned the geothermal grid for ash-filtering forest highways (cost-1 forest movement, forest supply lines, no forest ranged penalties).
- **Wreckers** (*Broker Vane*): Collapse-profiteers trading sabotage with the deep servitors (may win Surface *or* Furnace victory; may raze own cities for `+6 Worldstrain`; Deep Shaft spawns Stokers).
- **King Cinder's Host** (*King Cinder*): The rebel furnace-servitors breaking out at Worldstrain 50; no player can choose them (Great Furnace warded until the Blowout; reinforcements through the Breach; Caldera Keep rises in their hands at the Blowout; Furnace victory only—hold Caldera Keep when the countdown ends; heal `+15%` on kills; thrive on scorched ground).

## Core Rules Summary

- **Squad as the atomic map unit**: 3×3 Ogre Battle grid (4 places opening, 5 at Tier 2), Large
  2-cell units, named Captain with embedded portrait and passive aura, Wounded Hero recovery.
- **9 asymmetric factions**: Seventh Garrison, Keepers of the Seal, Tappers' College, Postriders,
  Three Hearths, Vault & Crucible, Canopy, Wreckers, and King Cinder's Host (rising at Worldstrain 50).
- **25 unit classes**: 24 recruitable classes across 3 tiers (standard core troops plus signature
  Worldseal units `Mender`, `Ashbound`, `Stoker`, and `Slagbreaker`) plus persistent
  city `Wall Sentinels`, each mapped to real `UNIT_*` sprite keys.
- **4 decoupled currencies**: Gold, Industry, Lore, and Mana (no commerce slider), with formula
  breakdowns, physical Caravans on the map, Raids & Sallies, literal Supply Lines (budget 18;
  road=1 / own=1 / wild=3; enemy/fort ZoC blocks → Hunger → Starvation), and zero Workers
  (automatic city Work Crews + squad field roads/forts).
- **Macro Cities & Regional Houses**: Size 1..10, 4–6 structural District slots (`Barracks`,
  `Market`, `Academy`, `Temple`, `Forge`, `Bulwark`), 2 Lifeboat Wonders (`Siphon Spire` bores
  into deep vents; `Great Cistern` drains cooling aquifers—each adding `+15 Worldstrain`), and
  Regional Houses (up to 4 cities per House under a minor-leader portrait Lord whose loyalty
  scales output and disables the House perk if Furious).
- **Compact 36-tech tree + Creeds + Rituals**: 8 Foundation techs + 7 Doctrine Ladders × 4 tiers
  (`Steel`, `Cavalry`, `Arcane`, `Faith`, `Artifice`, `Wild`, `Mercantile`; commit to 1 Primary
  Doctrine + optional Tier 1–2 Secondary dip), separate world race for 5 Creeds (`Code of the Watch`,
  `Weight & Measure`, `Old Growth`, `Choir Below`, `Open the Vent`), and 6 empire-level Strategic
  Rituals cast with Mana (two of them move Worldstrain, on a 12-turn recast).
- **Great People Court**: 6 empire-wide suit Renown meters (Sage, Prophet, Engineer, Merchant,
  Bard, Commander), arriving with a visible Secret Tech drawn from a shared 24-card world deck of
  surviving Imperial manuals and a 3-way choice: (1) Discover Secret Tech, (2) Instant Tempo Burst
  (returns card), or (3) Commission permanently as a Squad Hero Captain or House Lord (returns card).
- **160-turn Worldseal Clock & Lifeboat Endgame**: Coupled **Worldstrain** (`0–100`), 10-card
  Crisis Deck (Worldstrain 10..100), King Cinder's Host emerging at Worldstrain 50, **Caldera Keep** heaving
  up from the central sea on basalt bridges at Worldstrain 100 + 40-turn Blowout countdown, Surface
  Lifeboat vs. Furnace hold-the-Keep victory, and Turn-160 Keepers of the Seal / Score resolution.

## Screens

The client's Cataclysm screens (`cataclysm.js`: the top bar, the squad panel, the map's clicks and
tooltip; `cataclysm_screens.js`: the modal screens; `cataclysm_help.js`: the Help) follow these rules,
set by the UX pass of 2026-11-26:

- **The view states the rules.** A cost, a percentage, a threshold or a duration is never typed into
  the JavaScript. The adapter (`crates/ffh-play/src/cataclysm.rs`) sends it, or the whole sentence,
  from the rules' own constants and checks: `supply` on a squad, `notes` and `raze` on a city,
  `why` beside every `can_*`, `order_help`, `sally_help`, `court_rules`, `rules_help`,
  `apocalypse.rules`, `victory`. The Help is assembled from the same sentences, so it cannot drift
  from the rules. A data string that promised more than the rules do was corrected to the rules.
- **A disabled control shows its reason** as text on it, and details go in the rich tooltip (`tip`),
  not in the native `title` (which a touch screen never shows).
- **The player makes the rules' choices.** Where the rules take a target (a caravan's destination,
  the squad a hero joins, the House a Lord takes, the squad lent to a crusade, a new squad's
  members), the screen offers the list.
- **Hostile and irreversible actions ask first**, stating their effect: a declaration of war
  (including the one an attack on a peaceful empire implies), a crusade, a doctrine, a creed, razing,
  disbanding, and a build order that would discard progress.
- **A rival's city shows what a rival may know**: size, defense, Sentinels, districts; not its
  yields or its build queue.
- **The top bar's chips carry state**: `warn` (Worldstrain from 50, the Blowout countdown, a negative
  Gold rate, a House whose perk is off) and `accent` (no research chosen, a Great Person waiting)
  are drawn on every skin. The turn is shown against the turn limit.
- **One screen, one subject**: Cities (with the caravans), Research / Creeds / Rituals (tabs of one
  screen), the Court, the Houses, Diplomacy (with the crusades), the Worldseal (Worldstrain, Lifeboats,
  how the game ends). The menu and the F keys name those screens; the classic game's advisors that
  have no counterpart are hidden while a Cataclysm game is open.
- **Touch**: a tap selects, a second tap on the same tile moves or attacks (the map sends no
  right-click from a finger).
- **Text is bare** (CLAUDE.md, "Rules text and tooltips"): labels, numbers, the rule.
- **Rules text and game-noun reminders are separated** (`cataclysm_tips.js`, matching
  `docs/PLAY_DESIGN.md` 4.8): every Cataclysm tooltip (including Reinforce, Upgrade, Formation cells,
  squad orders, top-bar chips, map plots, and modal screens) states only what that control or state
  does in its main box with game nouns marked `[KW=CAT_...]`, while every referenced noun's reminder
  card (built from `cataclysm-core` constants in `cataclysm.keywords`) is drawn in its own side box
  (beside on wide screens, stacked below on narrow screens).
- **An eliminated player's game ends**: with no city and no squad left the status is `over`
  (`cataclysm_kind: "Eliminated"`) and the game-over screen says Defeat with the rule; before, End
  turn answered an error forever.
- **The Help is this mode's** (help.js lists a topic of the open game's mode only): how to play, the
  keys, squads and supply, battles, cities, research, Great People, Houses, the Worldseal.

`node tools/web-cataclysm-smoke.mjs --bin <built>/ffh-play` starts a Cataclysm game from the title
screen and checks every screen, the Help, the map tooltip and a few turns for errors and for an
"undefined" in any text.

## Rules fixes

- **A bloody battle's Strain fires its crisis** (2026-11-26). A battle with 3 or more units fallen
  adds 1 Strain. It did so with a bare `apocalypse.shift`, which marks every crisis whose threshold
  the change crosses as triggered without applying it: when that +1 reached a multiple of 10 the
  crisis's effect was skipped, and at 100 Caldera Keep never rose and the countdown never started
  (seen in a played game: Strain 100, no Lifeboat, no end before turn 160). It now goes through
  `shift_ac`, after the battle is settled. `check_invariants` holds the pair: the Blowout crisis is
  marked exactly when Caldera Keep has risen.

Open: the alignments keep the words Good, Neutral and Evil (a working rule: creeds, Houses and some
units read them); their names in this setting are the owner's to choose. Movement is one straight
step at a time toward the clicked tile (no path-finding around impassable ground).

## Rules changes: the pacing pass (2026-11-26)

120 AI-only games of the merged ruleset showed Strain at 100 by a median turn 29 of 160, research
over by turn 46, wars that never ended and a Caldera Keep nobody took (4 of 222 winning Lifeboats).
These rules changed, each on purpose; the numbers were then set with `cataclysm-tally` (below).

- **A tech moves Strain once for the world.** Its Strain applies to the first civilization that
  researches it; later researchers move nothing, and starting techs count as researched. Before,
  every civilization's research added the same Strain again, so the clock ran faster with more
  civilizations.
- **Two rituals move Strain, and they have a recast time.** Crack the Crust +3, Vent the Seal -1
  (the Keepers of the Seal -3); the same civilization can cast each again only 12 turns later. The
  other four rituals move no Strain (Vent-Ether Surge was +2, Furnace Levy +4, Draft-Wind March +1),
  and a ritual had no limit but Mana. One civilization working a lever all game now moves the
  Blowout by about a dozen turns.
- **Every third bloody battle** in the world adds 1 Strain, not every one.
- **Valve-Keeper's Ledger halves the drift** (every second +1 is skipped) instead of stopping it.
- **Doctrine techs cost 150, 400, 900 and 1750 Lore by tier** (were 65-75, 110-120, 165-175,
  230-245), the same in every Doctrine; the Primary discount (75%) and the Foundation costs are
  unchanged. Set from the measured Lore income (10 a turn early, 60 late) so that a Primary ladder
  takes until about turn 90.
- **Lore is never idle.** A civilization that has adopted both Doctrines and has no tech it can
  research converts its Lore to Sage Renown at its turn start, 10 Lore per 1 Renown (with a
  Doctrine still to adopt, the Lore is kept for the techs that opens).
- **Peace needs both sides, then a truce.** Make Peace records an offer that stands 5 turns; peace
  is signed when the other side offers while it stands. For 10 turns after, neither may declare
  war on the other, attack it, call a Crusade against it or lend a squad to one. It was one-sided
  and instant, and war could be redeclared the same turn. A Crusade ends when its caller and its
  target sign peace.
- **A finished order waits when there is no room for it**: a squad past the squad cap (the cap
  was checked only when the order was given: 39 squads at a cap of 7 in one AI game), a Caravan
  with every slot in use. The city screen says so.
- **A lost Wonder race refunds the Industry as Gold**, 1 for 1, with a log line (it was discarded
  without a word).
- **The turn skips a player eliminated at the wrap** (it went to a player who could not end it),
  and Furnace Levy is refused before it spends its Mana when there is no city.

## Rules changes: King Cinder's Host (2026-11-27)

The Host never won. Entering at Worldstrain 50 (turn ~48) with one city and half an average
civilization's army, at war with every Good civilization, it was eliminated in 229 of 240 games,
a median 11 turns after it entered: its squads were picked off in the field (it lost 7 units for
every 1 it killed when attacked), then its city fell. A Host that started the game won 6%.
Target for this pass: the Host wins 10-20% of the games it enters, at the default 6 civilizations.

- **It only enters mid-game.** No player can choose King Cinder's Host on the new-game screen
  (the owner's decision, 2026-11-27); every game has it enter at Worldstrain 50.
- **It enters in force, away from the surface.** On the unclaimed land farthest from every city,
  with squads until it holds 33% of the squad power of all the civilizations together (at least 2,
  at most 8). The land within 2 plots of Great Furnace becomes scorched ground (it fought on
  scorched ground anywhere at random before).
- **Great Furnace cannot be attacked or besieged until the Blowout.** A turn-limited ward (15,
  30 turns) only moved the elimination to a few turns after it ended, whatever the army; an
  unwarded besieged city also starved its garrison (a besieged city supplies no squad).
- **The Breach.** A squad joins the Host at its capital every 8 turns while it is under its squad
  cap; at the Blowout it gets squads until it holds 33% of the others' squad power together (at
  most 10; in Caldera Keep since the next section).
- **Caldera Keep is the Furnace's objective.** When the countdown ends with Caldera Keep razed, the
  Furnace civilizations alive (the Host, the Wreckers) win, whoever holds the other Lifeboats; and
  when it ends with no Lifeboat held by a civilization that can win by Lifeboat survival, they win
  too (no one won). Every Lifeboat razed is still a Furnace victory at once. Killing the Host before
  the countdown ends is the surface's answer to a razed Keep. (Replaced the same day: below.)
- **Worldstrain drifts +1 every 4 turns** (was 3): a Host that lives keeps cracking the crust, and the
  Blowout had moved 10 turns earlier (median turn 90).
- **AI**: the Host's worn squads (below 60% health) go home to Great Furnace while it is warded and
  stay until healed (95%).

120-480 AI games: the Host alive at the Blowout in 459 of 459 games with one, alive at the end in
425 of 477; it wins 12% of the games it enters at 6 civilizations (59 of 477, 27 of them shared
with the Wreckers), 26% at 2 (fewer Lifeboats to deny), 22% at 4, 9% at 8; 17% when one
civilization pushes Worldstrain all game, 2% when nobody raises Worldstrain by ritual (no Blowout in 71% of
those games).

## Rules changes: Caldera Keep to King Cinder's Host (2026-11-27)

The owner's call: give Caldera Keep to King Cinder. The Host's win above took three steps the AI
never took on purpose (a surface civilization takes the Keep from the Wild Clans, the Host takes it
back, razes it); now there is one objective both sides see on the map. No tuning: the Infernal
army compositions are still changing (the owner, 2026-11-27).

- **Caldera Keep rises in the Host's hands.** At the Blowout it is King Cinder's Host's city, and
  the Host's Blowout squads (33% of the others' squad power, at least 1, at most 10) appear in it
  instead of at Great Furnace. With the Host dead it rises with the Wild Clans and their guard
  squad, as before (no game of the 459 with a Blowout had a dead Host).
- **Caldera Keep can never be razed** (it could after its first 12 turns).
- **The Furnace wins by holding it.** When the countdown ends with a Furnace civilization (the
  Host, the Wreckers) holding Caldera Keep, the Furnace civilizations alive win, whoever holds the
  other Lifeboats. This replaces "Caldera Keep razed" and the at-once win for every Lifeboat razed,
  which an unrazeable Keep makes impossible. No Lifeboat held by a surface civilization at the end
  is still a Furnace win. The Host holding any other Lifeboat wins nothing.
- **AI**: while a Furnace civilization holds the Keep, every surface civilization sends its free
  squads at it (a holder keeps its Lifeboat guards) and goes to war with its holder; a Furnace
  civilization sends its free squads at the Keep while none of them holds it. A holder of the Keep
  stays home for it before any other Lifeboat.

480 AI games, 6 civilizations, standard policy (turns before the Blowout identical to the run
above): the Host wins 116 of 477 (24%, 78 of them shared with the Wreckers; it was 12%); the
surface takes the Keep back in the other 342 of 459 Blowout games. A Lifeboat is captured in 81%
of Blowout games (73%). The Keep is now held by a winner in every game by construction (whoever
holds it at the end wins), so the "Caldera Keep among the winning Lifeboats" row says nothing any
more. Not re-measured: quiet, pusher, and 2, 4 and 8 civilizations.

## The AI (`crates/cataclysm-ai`)

Stateless: every turn it works out its roles again from the game, and it plays only through
`Command`. `military.rs`: home guards in threatened cities, the capital and a held Lifeboat;
attacks only on the rules' own forecast (`Game::preview_attack`: the plot likely to fall, or a
trade clearly in its favour); after the Blowout every free squad at Caldera Keep while a Furnace
civilization holds it (a surface civilization) or does not (a Furnace one), else at one Lifeboat
for a civilization holding none; raids, tribute
before a city is stripped, roads in peace. `diplomacy.rs`: war only with a clear edge on a city in
reach (the Postriders, the Seventh Garrison, the Wreckers and King Cinder's Host at a smaller
edge), never a second front; peace when weaker, when a war is 15 turns old and going nowhere, or
when it holds a Lifeboat; King Cinder's Host never signs, and its worn squads heal in Great
Furnace while it is warded. `rituals.rs`: one stance on Strain (push
when it would hold a Lifeboat if the Blowout came now, or to deny a turn-160 win; pull for the
Keepers and for a score leader with no Keepers alive), then the useful rituals with the Mana left.
`Policy::Quiet` and `Policy::Pusher` are measuring instruments, not ways the game's AI plays.

## Measuring: `cataclysm-tally`

`cargo run --release -p cataclysm-ai --bin cataclysm-tally -- [--games N] [--opponents K]
[--policy standard|quiet|pusher] [--seat0 FACTION] [--rows] [--calibrate]` plays AI-only games (120 take a few
seconds) and reports the turn of the Blowout, what raised Strain, the Lore income by turn, when
research ran out, how wars ended, which Lifeboats won, how King Cinder's Host fared, and the
attack forecast against what the attacks did. Its last line (`TALLY ...`) is what a commit that changes a number quotes. `quiet`:
nobody raises Strain by ritual. `pusher`: seat 0 raises it all game. A number of the rules is
checked under all three, and at 2 and 8 civilizations, so that it is not tuned to one AI script.

### Pacing targets

Written before the first number was tuned; "before" is the ruleset as first merged (74378bfc, with
the turn-order fix), "now" after the King Cinder's Host pass (2026-11-27). 6 civilizations unless
said.

| | Target | Before | Now |
|---|---|---|---|
| Blowout, standard | median turn 95-120, 10th percentile 70 or later | 29 (22) | 96 (77); none in 4% of games |
| Blowout, quiet | none by turn 160 in 20-70% of games | none in 60% | none in 71% |
| Blowout, pusher | median 10+ turns before standard, 10th percentile 50 or later | 28 | 82 (66) |
| Blowout with 2, 4 and 8 civilizations | median within 20 turns of the 6-civilization one | 45, -, 24 | 97, 96, 93 |
| Primary Doctrine capstone | median turn 75-100 | 37 | 101 |
| Keepers' turn-160 win, standard | 3-25% of their games | 0% | 6% |
| A Lifeboat captured after the Blowout | half the games or more | 70% | 81% |
| Caldera Keep among the winning Lifeboats | 10% or more | 1% | 36% (always held by a winner since the Keep went to the Host) |
| Peace made | 1 per game or more | 0 | 6.9 |
| Seat 0 eliminated before turn 60 | 15% of games or fewer | 30% | 11% |
| King Cinder's Host wins (written 2026-11-27) | 10-20% of the games it enters | 0% | 24% since the Keep went to the Host (12% before); not tuned |

The quiet, pusher and 2/4/8-civilization rows were measured before Caldera Keep went to the Host
(nothing before the Blowout changed with it).

Two targets were restated after measuring, and why:
- "30% or fewer civilizations out of research before turn 100" collides with the capstone target:
  a civilization can research 14 techs in all and the capstone is the last of them. Now 44%; Lore
  has a use after it (above).
- "10 Great People per civilization or fewer": the count follows the length of the game, not the
  Lore conversion (15 a game at 5 Lore per Renown, 14 at 10, 14 at 15). Now 14.9 over a full game.

Not met, and left: with 2 civilizations no Blowout comes in a third of the games (often neither of
the two pushes); with nobody raising Strain by ritual no Blowout comes in 71% (one over the bound).
Civilizations eliminated per game are 2.2 of 7 (2.1 without King Cinder's Host).

### What the tally shows that is not fixed (balance, for the owner)

- **Most wins are shared.** A Lifeboat win belongs to every holder, a Furnace win to the Host and
  the Wreckers alive: 1 winner in 93 of 480 games, 2 in 239, 3 in 130, 4 in 18.
- **Faction shares in AI hands**: the Keepers 48% of their games among the winners, Three Hearths
  47%, Tappers' College 42%, the Wreckers 39% (they share the Host's Furnace wins), the Seventh
  Garrison 29%, King Cinder's Host 24%, Postriders 17%, Canopy 16%, Vault & Crucible 14%. It says
  as much about the AI script as about the factions.
- **Great Cistern and Siphon Spire** are the only Lifeboats a civilization can build, and only the
  Faith and Arcane Doctrines open them.

Open (rules): vassalage has a command that breaks the oath and nothing that creates a vassal; a
Cataclysm game cannot be saved or resumed; the attack forecast sees one defending squad, so a city
held by two looks takeable in one battle; a guard in a besieged city with no other city in supply
range goes hungry; the Wild Clans never move; Industry made while a city has no order is lost;
Gold has no use late in a game (squads full and upgraded: about 2,000 unspent per AI civilization).
