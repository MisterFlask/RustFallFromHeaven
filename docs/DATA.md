# Game data

## The format

The game's data is `data/json/*.jsonc`: JSON with comments (`//` and `/* */`) and trailing
commas allowed. One file per info table, named after its `GameData` field (`units.jsonc`,
`promotions.jsonc`, `leader_heads.jsonc`, ...); each holds a list of entries, each entry an object
that starts with its `type`. Beside them: `global_types.jsonc` (the type lists that are no table's
entries: flavors, contacts, directions, the automations' names, ...) and the defines files
`global_defines.jsonc`, `global_defines_alt.jsonc`, `python_callback_defines.jsonc` and
`rules_defines.jsonc` (this game's own; each an object from define name to value, read in that
order, a later file's value replacing an earlier one's). `data/text/en.tsv` holds the English
text the entries name by `TXT_KEY_*`.

```jsonc
[
  {
    "type": "UNIT_AXEMAN",
    "class": "UNITCLASS_AXEMAN",
    "unit_combat": "UNITCOMBAT_MELEE",
    "combat": 4,
    "unit_class_upgrades": {"UNITCLASS_CHAMPION": true, "UNITCLASS_CHARIOT": true},
    "prereq_tech": "TECH_BRONZE_WORKING",
    // only what differs from the defaults
  },
]
```

**Values.**
- A number, a flag (`true`/`false`) or a type name (`"UNITCOMBAT_MELEE"`).
- A per-type value is an object from type names to values (`"unit_combat_mods": {"UNITCOMBAT_MELEE": 25}`).
- A list of types is an array of names.
- Yields and commerces are arrays by position (`[food, production, commerce]`).
- A record is an object, and a list of records an array of objects.

**Defaults.** A key an entry leaves out takes its default: the one its reader passes
(`rd.int_or("max_latitude", 90)`), else 0, false, none or empty. Each key is read with one
method of `Rd` (`crates/ffh-data/src/registry.rs`), which says the kind of value it expects.

**Checks.** Loading warns about a value of the wrong kind, an unknown type name, and a key no
reader looks up (a misspelt or obsolete key: `unknown key "combt": nothing reads it`); a key that
appears twice in one object fails the load, with its line and column. `ffh data` must report
0 warnings.

**Adding a field.** Add the key to the entries that need it and read it in the table's reader
(`crates/ffh-data/src/infos/*.rs`) with its default; entries without it are unchanged.

**Keys from the old XML.** The data began as FFH2 0.41's Civ4 XML, converted in November 2026
(the loaded game data identical before and after). A key is its XML tag without the `i`/`b`/`f`
prefix, in snake_case (`iCombatDefense` -> `combat_defense`, `UnitClassUpgrades` ->
`unit_class_upgrades`, `UnitAIs` -> `unit_ais`), so a C++ getter or a Civ4 tag named in a comment
maps to its key; where two tags of one entry made the same key, one was renamed: `Combat` ->
`unit_combat`, a unit's `Invisible` -> `invisible_type` and `PrereqBonuses` ->
`prereq_or_bonuses`, `bPrereqReligion` -> `is_prereq_religion`, a building's `bStateReligion` ->
`is_state_religion` and `iPower` -> `power_value`. Values equal to their default were left out,
and the tags nothing read (Civ4's art, sound, music, hotkey priorities for the exe, popup texts)
were dropped. The action hotkeys the client shows were kept as `hotkeys` and `order_priority`
on missions, commands, builds and automations.

## Sources

| What | Source | Pin |
|---|---|---|
| FFH2 0.41 XML, Python, DLL source | github.com/jlippitt/FFH2-Better-AI | `39c753c` "Mod in initial state" (unmodified FFH2 0.41, pre-AI-mod) |
| Stock BTS XML that FFH2 inherits | github.com/rigiechang/Broken-BTS | `7bb2280` (BTS), `9a5358c` (vanilla) |

FFH2 ships only the XML files it changes; everything else was inherited from Beyond the Sword
3.19. The inherited files the data was vendored from:

- the attitude, denial, domain, calendar, season and month infos (`Basicinfos/`)
- `yields.jsonc`, `upkeeps.jsonc`, `climates.jsonc`, `sea_levels.jsonc` (byte-identical across the
  vanilla/Warlords/BTS copies -> stock)
- `hurries.jsonc` from the vanilla copy (30 production per pop, 3 gold per production). Both BTS
  copies in the reference repo carry mod edits (90/1 and 120/3).
- `global_defines.jsonc`, reconstructed (below).

FFH2's `diplomacies.jsonc` (the leaders' lines for each diplomacy moment, by leader, attitude and
relative power; presentation only) names lines from FFH2's own text and from the stock
`CIV4DiplomacyText.xml` files, so `data/text/en.tsv` merged each text layer's
`CIV4DiplomacyText*.xml` before its `CIV4GameText*.xml` when it was vendored.

`data/text/barks.md` is not vendored: it is the port's own writing, the lines each civilization's
leaders have for each other civilization at eleven diplomacy moments (ffh-play
`view/speech.rs`, `Barks`; off in a game started with the vanilla experience).

## global_defines.jsonc reconstruction

The reference repo's BTS `GlobalDefines.xml` was edited by that repo's mod. For all 30 keys where
its value differs from the Warlords file, the vanilla, Warlords and BTS-root copies all agree with
each other, so these are the mod's edits and were reset to the vanilla/Warlords value when the
data was vendored (e.g. `INITIAL_CITY_POPULATION 3 -> 1`, `UNIT_PRODUCTION_PERCENT 50 -> 100`).
`CIV4_VERSION`/`SAVE_VERSION` keep their BTS values. Defines that exist only in BTS (94 keys) are
taken as-is; they cannot be cross-checked against another copy.

Residual risk: a BTS-only define the mod also edited would go unnoticed. FFH2's
`global_defines_alt.jsonc` overrides 25 BTS keys on top of this.

## Generated code

`crates/ffh-data/src/defines_gen.rs` has one field per define: `tools/gen_defines.py` writes it
from the defines files (and, with the FFH2 reference sources from `tools/fetch-reference.sh`, the
defines the C++ reads that the data lacks; without them it keeps the ones the file lists).

## Quirks handled by the loader

- `automates.jsonc` entries have no `type`; names come from the `automate_types` list in
  `global_types.jsonc` (as in C++).
- Text-valued defines listed in C++ `SetPostGlobalsGlobalDefines` (`BARBARIAN_CIVILIZATION`,
  `SLAVE_UNIT`, ...) resolve to type indices; other text defines read as -1 as ints (C++
  `getDefineINT` returns the string's address there, matching no type; e.g. FLOOD_PLAINS_FEATURE).
