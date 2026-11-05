# Building suitability (proposal)

Status: **proposal, not built**. Revision 2, 2026-11-05: building variants, a plain random
roll, no cost tier. Nothing here is in the rules yet. The code paths named are the ones the change
would touch.

When a city is founded, it rolls a suitability for each generic building class. The city can only
build the variant of each building that matches its roll:

- an **Unsuited** city gets a weaker variant,
- a **Plain** city gets the FFH2 building,
- a **Suited** city gets a stronger variant.

Each variant is a real building with its own name and its own numbers: *Cramped Library*,
*Library*, *Grand Library*. In lists, the name and its text color show the suitability. Nothing
in the city modifies building effects at runtime: a variant does exactly what its row says. Cost
does not change. The FFH economy is kept: the tech tree, the prerequisites and the kinds of
effects are unchanged, and Plain cities play FFH2.

The roguelike selection is in the building choice. The city is dealt a hand at founding, and the
player builds into its strengths. A Grand Library gives more per hammer than a Library, so it is
worth building sooner. A Cramped Forge may never be worth building at all.

A rules option controls all of this, and it is off by default.

## 1. Why variants rather than a modifier

Revision 1 of this proposal stored a tier per city. It then scaled building effects inside
`city_process_building`, which meant splitting that function, scaling commerce on the read side,
and adding a harness check that recomputed the scaled totals. Variants remove all of that:

- `city_process_building`, the building-commerce code and the AI's `building_value` read the
  variant's own row, so they need no change.
- A tooltip shows the numbers the building really has. There is no second "x150% here" layer for
  the player to work back into the building.
- Hand-tuning one variant is a data edit. For example, if a Grand Harbor at +2 food per sea tile
  turns out too strong, change that row.

The cost of this approach is in code that names a specific building (section 4).

## 2. Variants

### 2.1 Generated, not hand-authored

`ffh-data` builds the variant rows when it loads the data: two extra `BuildingInfo` rows for each
building in scope, derived from the base row by the tier table. Generating them, rather than
writing them in the XML, has three advantages:

- the XML stays FFH2's;
- a change to a base building carries over to its variants;
- civilizations' unique buildings get variants automatically. The Governor's Manor gets a Grand
  and a Cramped Governor's Manor.

A small override file, `data/xml/Buildings/CIV4BuildingVariantInfos.xml`, can give a variant its
own name, or its own value for one field, where the generated one is wrong. Example: a Suited
Harbor keeps +1 food per sea tile and gets something else instead.

Each variant row has `variant_of: Option<BuildingType>`, pointing at its base, and
`suitability: Suitability`. Every base building's own row is Plain.

### 2.2 The tier table

| Suitability | Effect | Rounding | Color | Default name |
|---|---|---|---|---|
| Unsuited | 50% | down | muted / grey-red | "Cramped {name}" |
| Plain | 100% | — | normal | "{name}" |
| Suited | 150% | up | gold / green | "Grand {name}" |

The percentages and the roll weights (section 3) are defines (`SUITABILITY_*` in
`RulesDefines.xml`). The name patterns are text keys. Variant names are data, so they may carry
flavor, and the override file can replace the pattern for one building (a Suited Herbalist could
be a *Physic Garden*).

**What the generator scales.** It scales only a building's city-local, additive, positive numbers:

- `yield_change`, `yield_modifier`, `sea_plot_yield_change`, `river_plot_yield_change`;
- `commerce_change`, `obsolete_safe_commerce_change`, `commerce_modifier`;
- positive `happiness` and `health`;
- `specialist_count`, `free_specialist_count`, `great_people_rate_change` and `_modifier`;
- `free_experience`, `unit_combat_free_experience`, `domain_free_experience`;
- `food_kept`, `defense_modifier`, `bombard_defense_modifier`, `heal_rate_change`;
- `trade_routes`, `trade_route_modifier`;
- `military_production_modifier`, `domain_production_modifier`, `resist_magic`.

It copies everything else unchanged:

- **Negative entries.** The Market's -1 research and the Forge's -1 health stay as they are, so a
  Grand Forge is no unhealthier.
- **Yes/no and one-off effects:** power, government center, free bonuses, plot radius, free
  promotions, religion influence.
- **Effects outside the city:** player, team, area and global effects, plus `asset_value` and
  `power_value`.
- **Cost** (`iCost`), prerequisites, and `iAIWeight`.

**Code effects.** Some building effects live in code (the table in
`crates/ffh-ai/src/code_effects.rs`). Those read the variant's base building, and scale where the
effect is a number, such as the Cave of Ancestors' experience.

**Rounding.** FFH building values are small integers. If a value were simply truncated, a Suited
+1 health would stay +1, so the generator rounds up for Suited and down for Unsuited: a +1 becomes
+2 or 0, and +25% becomes +38% or +12%. Rounding up doubles a +1, so the first data pass should go
through every +1 in a per-plot field and decide each one in the override file.

### 2.3 Scope

Variants are generated for the generic classes with no instance limit and a real cost: about 40
classes, about 60 rows once unique buildings are counted. These are left out and stay
single-variant:

- world and national wonders;
- the civilization classes `BUILDINGCLASS_CIV_BUILDING1-3`;
- buildings that cannot be built normally (cost -1: cages, vaults, the Dancing Bear and the like).

Religious temples are in scope. That needs a decision from the owner (section 6).

## 3. The roll

`player_init_city` makes one roll per in-scope class with `soren_rand`, weighted by the
defines. The proposed weights are Unsuited 15, Plain 70, Suited 15. With about 40 classes, a city
gets about 6 Grand classes and 6 Cramped ones, and stays mostly FFH2. The roll is symmetric, so
the expected output of a city is about unchanged. Rounding moves a +1 by exactly one step either
way, so it stays symmetric too.

The result is stored on the city as `building_suitability: IdxVec<BuildingClassType,
Suitability>`. This field only picks which variant the city may build. It modifies nothing. The
roll is made once and never changes.

- **Capture, trade, flip.** The roll belongs to the city, so it is kept. When a city changes
  owner, FFH already converts each building to the new owner's civilization's building of the same
  class. The conversion then takes the variant of that building that matches the city's roll.
- **Free buildings** (from traits, civics or events, given by class) are given as the city's
  variant.
- **Option off.** No rolls are made, so the RNG stream and the game are bit-identical to today.

## 4. Where variants cost work: "the same building"

The engine identifies buildings by `BuildingType`. A Grand Stable is a different type from a
Stable, so every place that asks for one particular building must also accept its variants:

- **XML prerequisites.** 41 `PrereqBuilding` entries, 8 `BuildingPrereq` and 18
  `CreateBuildingType` name a building. The in-scope ones include the Siege Workshop (8 units),
  the Stable (4) and the six religions' temples (23 in total). A unit that needs a Stable must
  accept a Grand Stable.
- **Hard-coded names in Rust.** About 40 `t::<BuildingType>("BUILDING_…")` lookups in `ffh-core`
  and `ffh-ai`. Most are wonders or unique buildings and so out of scope. The in-scope ones are
  Walls, Granary, Dungeon, Smugglers' Port, Gambling House, Arena and the religious temples.
- **Class to building.** `GameData::civ_building(civ, class)` has 44 call sites. The ones in a
  city's context (free buildings, conversion on capture, the "can build" lists) become
  `city_building_for_class(c, class)`, which returns the civilization's building and then the
  city's variant of it.

The fix is a single rule, applied everywhere. "The city has building X" means "the city has X or
a variant of X": `city_has_building_family(c, x)`, using `variant_of`. Prerequisites in the data
are matched on the family. A test enumerates every building reference in the XML and in the
`pycall` dispatch tables, and checks that each one resolves through the family. The test exists so
a newly added prerequisite cannot silently exclude Suited cities.

A smaller cost: there are about 70 more `BuildingType` rows. Loops that run over every building
per city, such as `city_update_building_commerce`, get about 40% longer, and the per-building
`IdxVec`s on `City` and `Player` grow by the same amount. The cost is real but small. Check it once
with `ffh bench`.

## 5. AI, interface, save

- **AI.** `building_value` reads the variant's row, so a Grand Library is valued at what it does.
  The production chooser ranks value against `city_production_turns_left_building`. A Suited
  building therefore rises in the queue and an Unsuited one sinks, with no new AI code. This is
  a rules change, so the AI only needs to use it sensibly. The local check: with the option on,
  count how much of the building production goes to Grand and to Cramped variants. Grand should
  clearly lead.
- **Interface.**
  - The build list, the city's building list and the tooltips show the variant's name in its
    suitability's color. The tooltip text stays bare: `Grand Library: +38% research, 2 scientist
    slots.`
  - A third line gives the base: `Suitability: Suited (Library: +25% research).`
  - The browser's data endpoint (`data`) has to send `variant_of` and `suitability`, so the
    client can color names without a lookup table of its own.
- **Save.** Add `building_suitability` to the `persist_struct!(City { .. })` list in
  `save/state.rs` and bump `FORMAT_VERSION`. The variant rows are generated from the data, so the
  save format needs nothing for them.
- **Docs.** Record the rule in docs/PORTING.md under "Changes from FFH2".

## 6. Open points

1. **Religious temples in scope?** A Cramped Temple of the Order makes a city a poorer home for
   the faith, which is interesting. It also means a religion's 23 data references go through the
   family rule. I recommend including them, since the family rule is needed anyway.
2. **Roll weights.** 15/70/15 is a starting guess. More non-Plain classes give more variety but
   also more noise in which cities are good.
3. **Default for the browser's new-game screen.** Leave it unticked until the AI check and a few
   human games look right.

## 7. Phases

| Phase | What | Check |
|---|---|---|
| 1 | Generator (`variant_of`, tier table, rounding, override file), `ffh data` 0 warnings | data tests: each variant's scaled fields, untouched negatives, names resolve |
| 2 | Option, roll, `building_suitability` on City, `city_building_for_class`, `city_has_building_family`, capture conversion, save | gate; with the option off, sync checksums equal the current binary's; the XML-reference test |
| 3 | Interface: names, colors, tooltips, data endpoint | web smoke |
| 4 | AI local check, `ffh bench` | Grand and Cramped shares; per-turn cost within noise |
