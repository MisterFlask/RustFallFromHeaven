# Procedural maps

The new-game screen's **Map type** selects **Fractal** (the default), **Erebus** or **Erebus Terra**. The session
API uses `setup.map_script: "Fractal" | "Erebus" | "ErebusTerra"`; omitted values retain Fractal, and unknown
names are errors. Human and observed games use the same selection. The selected name is recorded
in journals and the existing saved `GameSetup.map_script` field; no binary save-format change.
Loading Erebus restores its script hooks, including for subsequent regeneration.

## Erebus

`crates/ffh-core/src/mapscript/erebus.rs` adapts **Rich Marinaccio (cephalo)'s Creation/Erebus**
script from the original FFH2 map family. The reference is Creation 1.08 (1.07's geography plus
lfgr's 2019 assertion fixes), [PrivateMaps/Erebus.py](https://github.com/akachi10/FFH2-game-files/blob/master/PrivateMaps/Erebus.py),
Git blob `1b66edd67b621e3c5c7b6c5cceb5f7d1d09446b4`. Original copyright: 2007 Rich Marinaccio;
assertion fixes: lfgr. This is the original valley-based Erebus, distinct from ErebusContinent
and WorldOfErebus.

The geographical algorithm follows the reference's stages:

1. Grow separated seeds into irregular regions on the grid of river intersections. Intersections
   claim their four neighboring plots, leaving unclaimed mountain barriers.
2. Choose a central sea region and grow a connected collection of sea basins. Absorb unclaimed
   spaces adjoining water so narrow unassigned walls do not divide the sea.
3. Find river gates between regions and attach land regions downstream to water in an acyclic
   graph. Region elevation increases upstream. Within each valley, build intersection heights
   outward from its gate and route tributaries downhill.
4. Accumulate rainfall into rivers. Distance from the wettest outlet controls regional moisture;
   drainage lowers local altitude and adds moisture beside rivers.
5. Form mountain rims between valleys, with more hills/peaks at higher elevations and traversable
   river outlets. Unreachable pockets become mountains. Land at the map boundary is mountainous.
6. Assign deserts/plains/grassland by moisture, tundra/snow by altitude, then forests, jungles,
   marshes, scrub, flood plains and oases. Climate settings adjust these thresholds.

The map does not wrap in either direction. Medium-sea-level dimensions in plots are 32², 36²,
40², 52², 60² and 88² for Duel through Huge. As in the reference, sea level changes both sea-region
frequency and map dimensions, keeping more room for playable land when seas are larger.

### Native adaptations

This is a native adaptation, not a seed-identical execution of the Python script. It uses the
engine's deterministic map RNG instead of Python's separate random generator. Region growth is
bounded; drainage accumulation uses a topological pass rather than repeatedly walking complete
tributaries. Gates use intersections with an interior neighbor in both regions. River passes
remain traversable even for short, dry tributaries. Geography is generated once per new map and
held on that game's script instance for the terrain, river and feature hooks.

Resources, starting positions, watchtowers and unique improvements use the port's existing
placement systems. The Python script's civilization-specific starting-region preferences and
landmark relocation are not ported. Its additional-lake/river and climate-flattening normalization
steps remain disabled; existing food normalization is retained. No Python runtime is needed,
including in the browser's WebAssembly engine.

## Checks

Erebus geometry tests cover all normal map scales and multiple seeds: repeatability, valid
outflow gates, connected seas, nonwrapping boundaries, usable land and terminating river paths.
`crates/ffh-play/tests/erebus.rs` exercises setup validation and round trips, human/observed
selection, full game creation, starting plots, rivers, sea-level sizing, cold climate and saving
and loading. Browser checks exercise the new-game selector and both native and WebAssembly hosts.


## Erebus Terra

**Erebus Terra** is a separate option; ordinary Erebus and Fractal retain their generation.
It grows two independent Erebus-style regional continents with ocean drainage roots, removes
isolated offshore rock fragments and separates them with at least twelve empty ocean columns.
This leaves deep water between their coastal waters, without early coastal stepping stones.
Their east/west orientation is seeded. There is no wrapping.

The main continent uses the selected Erebus size. The map is widened to accommodate the New
World and ocean. The smaller continent targets one-third of the main continent's **habitable
land** (flat land and hills), rather than its mountain-inclusive footprint. Bounded candidate
search adjusts its size; rare small-world draws can reclaim mountain shoulders to keep enough
settleable land. All normal civilizations, including observer-game AI players, start on the
main continent. The New World has no civilization starts; standard barbarian, lair and unique
improvement rules still apply there.

Gems, Silk, Incense and Mithril are exclusive to the New World, with at least two accessible
copies of each. Normal placement skips those types; a generation-only pass after start
normalization, the existing far-shore pass and mana typing removes any copies introduced on the
main continent and guarantees overseas deposits. It uses legal terrain/features and creates a
small suitable biome patch when a resource otherwise has nowhere to grow. Food, copper, iron,
horses and reagents use normal placement. Existing far-shore rules also add overseas mana.
These are initial map-generation rules, not restrictions on later spells or discoveries.

The selected script is saved and restored without changing the binary save format. Geometry
checks cover the two land areas, relative usable size, ocean gap and repeatability. Full game
checks cover human/observer starts, resource exclusivity after normalization, biome validity,
climate/sea levels and save/load. `tools/web-site-smoke.mjs --map ErebusTerra` exercises the
WebAssembly version through the real new-game form, turns, saving and reloading.
