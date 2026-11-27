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

Now the **Worldseal** is failing. Pressurized vent-ether (**Mana**) leaks through the crust; deep smelting, vent-tapping, razing, and Lifeboat construction all drive **Worldseal Strain** toward `100`. Every playable faction is a surviving department or dissident offshoot of that dead Empire:

- **Seventh Garrison** (*Commander Maren Vance*): The unrelieved rim-watch legion (`+15%` front-row damage; free-upkeep, double-credit Crusade pledges).
- **Keepers of the Seal** (*Abbot Oren Hale*): Emergency engineers turned maintenance monks (`Vent the Seal` drains `-5 Strain`; `+25%` Sentinel HP; automatic Turn-160 win if Strain stays below 100).
- **Tappers' College** (*Dean Halden Pell*): Imperial power-scholars piping vent-ether through ceramic towers (`+2 Mana/Academy`; `+20%` battle spell damage; starts with `Arcana`).
- **Postriders** (*Factor Bajan*): The imperial highway and relay-stable network (`+1` open-ground movement; `+50%` Raid Gold; `-25%` reinforcement Gold).
- **Three Hearths** (*Ward-Heir Corin*): Imperial urbanists capped at 3 six-district megacities (`+50%` output & Culture), fed by low-upkeep Outposts.
- **Vault & Crucible** (*Master-Assayer Bram*): Mortal foundry-guilds requiring full bullion vaults (`>= 200` Gold/city) to fire deep furnaces (`+35%` Industry & `+4` Armor; `+25%` Siege Bombard damage).
- **Canopy** (*Mistress Sylva*): Dissidents who abandoned the geothermal grid for ash-filtering forest highways (cost-1 forest movement, forest supply lines, no forest ranged penalties).
- **Wreckers** (*Broker Vane*): Collapse-profiteers trading sabotage with the deep servitors (may win Surface *or* Furnace victory; may raze own cities for `+6 Strain`; Deep Shaft spawns Stokers).
- **King Cinder's Host** (*King Cinder*): The rebel furnace-servitors breaking out at Strain 50 (Furnace victory only—raze every Lifeboat after Strain 100; heal `+15%` on kills; thrive on scorched ground).

## Core Rules Summary

- **Squad as the atomic map unit**: 3×3 Ogre Battle grid (4 places opening, 5 at Tier 2), Large
  2-cell units, named Captain with embedded portrait and passive aura, Wounded Hero recovery.
- **9 asymmetric factions**: Seventh Garrison, Keepers of the Seal, Tappers' College, Postriders,
  Three Hearths, Vault & Crucible, Canopy, Wreckers, and King Cinder's Host (rising at Strain 50).
- **25 unit classes**: 24 recruitable classes across 3 tiers (standard core troops plus signature
  Worldseal units `Mender`, `Ashbound`, `Stoker`, and `Slagbreaker`) plus persistent
  city `Wall Sentinels`, each mapped to real `UNIT_*` sprite keys.
- **4 decoupled currencies**: Gold, Industry, Lore, and Mana (no commerce slider), with formula
  breakdowns, physical Caravans on the map, Raids & Sallies, literal Supply Lines (budget 18;
  road=1 / own=1 / wild=3; enemy/fort ZoC blocks → Hunger → Starvation), and zero Workers
  (automatic city Work Crews + squad field roads/forts).
- **Macro Cities & Regional Houses**: Size 1..10, 4–6 structural District slots (`Barracks`,
  `Market`, `Academy`, `Temple`, `Forge`, `Bulwark`), 2 Lifeboat Wonders (`Siphon Spire` bores
  into deep vents; `Great Cistern` drains cooling aquifers—each adding `+15 Strain`), and
  Regional Houses (up to 4 cities per House under a minor-leader portrait Lord whose loyalty
  scales output and disables the House perk if Furious).
- **Compact 36-tech tree + Creeds + Rituals**: 8 Foundation techs + 7 Doctrine Ladders × 4 tiers
  (`Steel`, `Cavalry`, `Arcane`, `Faith`, `Artifice`, `Wild`, `Mercantile`; commit to 1 Primary
  Doctrine + optional Tier 1–2 Secondary dip), separate world race for 5 Creeds (`Code of the Watch`,
  `Weight & Measure`, `Old Growth`, `Choir Below`, `Open the Vent`), and 6 empire-level Strategic
  Rituals cast with Mana.
- **Great People Court**: 6 empire-wide suit Renown meters (Sage, Prophet, Engineer, Merchant,
  Bard, Commander), arriving with a visible Secret Tech drawn from a shared 24-card world deck of
  surviving Imperial manuals and a 3-way choice: (1) Discover Secret Tech, (2) Instant Tempo Burst
  (returns card), or (3) Commission permanently as a Squad Hero Captain or House Lord (returns card).
- **160-turn Worldseal Clock & Lifeboat Endgame**: Coupled **Worldseal Strain** (`0–100`), 10-card
  Crisis Deck (Strain 10..100), King Cinder's Host emerging at Strain 50, **Caldera Keep** heaving
  up from the central sea on basalt bridges at Strain 100 + 40-turn Blowout countdown, Surface
  Lifeboat vs. Furnace Raze-All victory, and Turn-160 Keepers of the Seal / Score resolution.

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
- **The top bar's chips carry state**: `warn` (Strain from 50, the Blowout countdown, a negative
  Gold rate, a House whose perk is off) and `accent` (no research chosen, a Great Person waiting)
  are drawn on every skin. The turn is shown against the turn limit.
- **One screen, one subject**: Cities (with the caravans), Research / Creeds / Rituals (tabs of one
  screen), the Court, the Houses, Diplomacy (with the crusades), the Worldseal (Strain, Lifeboats,
  how the game ends). The menu and the F keys name those screens; the classic game's advisors that
  have no counterpart are hidden while a Cataclysm game is open.
- **Touch**: a tap selects, a second tap on the same tile moves or attacks (the map sends no
  right-click from a finger).
- **Text is bare** (CLAUDE.md, "Rules text and tooltips"): labels, numbers, the rule.
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
