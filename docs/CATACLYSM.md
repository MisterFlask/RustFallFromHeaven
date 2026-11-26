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

## Core Rules Summary

- **Squad as the atomic map unit**: 3×3 Ogre Battle grid (4 places opening, 5 at Tier 2), Large
  2-cell units, named Captain with embedded portrait and passive aura, Wounded Hero recovery.
- **9 asymmetric factions**: Bannor, Elohim, Amurites, Hippus, Kuriotates, Khazad, Ljosalfar,
  Sheaim, and Infernals (Hyborem rising at Armageddon 50).
- **25 unit classes**: 24 recruitable classes across 3 tiers plus persistent city Wall Sentinels,
  each mapped to real `UNIT_*` sprite keys.
- **4 decoupled currencies**: Gold, Industry, Lore, and Mana (no commerce slider), with formula
  breakdowns, physical Caravans on the map, Raids & Sallies, literal Supply Lines (budget 18;
  road=1 / own=1 / wild=3; enemy/fort ZoC blocks → Hunger → Starvation), and zero Workers
  (automatic city Work Crews + squad field roads/forts).
- **Macro Cities & Regional Houses**: Size 1..10, 4–6 structural District slots (`Barracks`,
  `Market`, `Academy`, `Temple`, `Forge`, `Bulwark`), 2 Lifeboat Wonders (`Tower of Mastery`,
  `Altar of the Luonnotar`), and Regional Houses (up to 4 cities per House under a minor-leader
  portrait Lord whose loyalty scales output and disables the House perk if Furious).
- **Compact 36-tech tree + Faith + Rituals**: 8 Foundation techs + 7 Doctrine Ladders × 4 tiers
  (commit to 1 Primary Doctrine + optional Tier 1–2 Secondary dip), separate world race for 5
  Religions, and 6 empire-level Strategic Rituals cast with Mana.
- **Great People Court**: 6 empire-wide suit Renown meters (Sage, Prophet, Engineer, Merchant,
  Bard, Commander), arriving with a visible Secret Tech drawn from a shared 24-card world deck
  and a 3-way choice: (1) Discover Secret Tech, (2) Instant Tempo Burst (returns card), or
  (3) Commission permanently as a Squad Hero Captain or House Lord (returns card).
- **160-turn Apocalypse Clock & Lifeboat Endgame**: Coupled Armageddon Counter, 10-card Crisis
  Deck (AC 10..100), Hyborem arrival at AC 50, Carcosa rising from the central sea with land
  bridges at AC 100 + 40-turn countdown, Mortal Lifeboat vs. Demonic Raze-All victory, and
  Turn-160 Elohim Vigil / Score resolution.
