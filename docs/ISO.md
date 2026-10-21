# ISO.md: the isometric 2.5D map ("M7", HD pixel art)

**Status: playable; the default map renderer.** One of the browser client's map renderers
(`crates/ffh-play`), beside the 2D pixel art (docs/ART.md), the flat debug renderer and the experimental 3D one
(docs/R3D.md). It draws the game as a bright, saturated "HD pixel art" diorama on a 2:1 isometric
board, close to the owner's reference screenshot (`tools/m7/ref/reference_full.png`). Units, cities,
features and terrain are sprites drawn by an image model (OpenAI `gpt-image-2.5-sunburst`) and
brought to one contract by code: the **M7 art package** in `tools/m7/`. Everything around the
sprites (placement, banners, shadows, borders, rivers, roads, plaques, paths) is drawn by
`assets/app/iso.js`. It reads the same game state as the other renderers; no rule, save or protocol
changed for it. While it draws the map, the client's interface takes the reference's look too
(section 9).

This direction replaces the earlier "Songs of Conquest" proposal for the isometric map (its
document and reference images were removed in the same change).

Contents: 1 running it · 2 the art package · 3 the contract · 4 what the game maps to which
sprite · 5 drawn by the engine · 6 adding art · 7 the sprites so far · 8 gaps and next steps ·
9 the interface.

---

## 1. Running it

```
cargo build --release
./target/release/ffh-play serve            # open http://127.0.0.1:8642/ (iso is the default)
```

It is the default renderer, so the plain URL shows it; `?renderer=art` switches back to the 2D
pixel art for one page, and **Tools → Options → "Isometric 2.5D"** / "2D pixel art" is remembered by
the browser.
Zoom (`+` `-`, `Q` `E`, the wheel): 48, 64, 96 (the default), 128, 192 (the art's own size, 1x) and 384 CSS px
a tile. `--assets crates/ffh-play/assets` serves the client and its sprites from disk, so a new
export (section 6) shows on reload without a rebuild.

## 2. The art package (`tools/m7/`)

`tools/m7/README.md` is the authority on how art is made: the prompts, the style anchors, the
process (three variants per call, a review sheet, pick one, revise or recut), the quality checklist
and the costs. The package is self-contained: `m7.py` (the one command), `lib/` (the API wrapper with
its spend ledger and budget caps, keying, scaling, the seamless-tile maths, the reference
compositor `lib/compose.py`), `prompts/` (the style prompts and the exact prompt of every accepted
sprite), `anchors/` and `ref/` (the images every call is anchored on), `roster/` (the accepted
sprites and `manifest.json`), `spend.jsonl` (every call made, with its cost), `budget.json` (the
caps). `work/` is scratch and is not committed.

Two additions for this game:

- **Overlays.** An inset tile (README "Terrain b": a farm, hills, a quarry ... pasted inside a base
  tile) also writes the object alone, clipped to the diamond, and `accept` stores it as
  `roster/terrain/ovl_<name>.png`. The game draws the overlay over whatever base tile the plot has,
  so one farm serves grassland, plains and desert alike. (The shipped farm, pasture and green hills
  were cut out of their tiles by their difference to `tile_grass`, their base.)
- **`python3 tools/m7/m7.py export`** copies the roster into the client
  (`crates/ffh-play/assets/app/iso/<key>.png`, inset tiles as their overlays only) and writes the
  index `iso/sprites.js` (each sprite's size and opaque box). `build.rs` embeds every PNG there.

## 3. The contract (tools/m7/README.md, "The asset contract")

All sizes are art pixels; at 1x a tile is 192 px wide. The renderer draws every sprite at
`ts / 192` of its size in device pixels: by nearest neighbour at whole factors; other factors are
scaled once per zoom and cached (reductions smoothed, halving first; fractional enlargements by
nearest neighbour to the next whole factor, then smoothed down, so they stay crisp). Each sprite's
edges are rounded from their exact positions and tiles keep hard diamond edges, so tiles meet
without seams or pinholes at any device-pixel ratio (a browser zoom of 110% included). On a map
that wraps east-west every copy of a plot in view is drawn.

| Kind | Key | Canvas | Anchor |
|---|---|---|---|
| tile | `tile_<terrain>`, `tile_<terrain>_b` | 192 x 128 | top-left = the tile's top-left; the diamond is rows 0-95, a 30 px earth/water skirt below |
| overlay | `ovl_<name>` | 192 x 128 | as a tile, drawn over it |
| feature | `feat_<name>` | >= 192 wide | bottom-centre on the tile's bottom vertex |
| building | `bld_<name>` | 192 wide, <= 256 tall | bottom-centre on the tile's bottom vertex |
| resource | `res_<name>` | 192 wide (a small prop centred) | bottom-centre 40 px right of and 30 px below the tile centre (its front-right quarter) |
| unit | `unit_<name>` | 96 x 120, wider for big units | bottom-centre 6 px below the tile centre |

Tiles go back to front; water tiles 8 px lower, so the land's skirts show as earth banks along the
coast and the map's edges. Then everything that stands up, sorted by its plot's depth and, on one
plot: feature, building or city, resource, unit. A unit that something in front hides by more than
a quarter is drawn again, faded, over it; clicking a figure (which rises into the plots behind it)
picks the figure's plot.

## 4. What the game maps to which sprite

The tables are at the top of `iso.js`; every lookup has fallbacks, so a missing sprite never
leaves a hole.

- **Terrain:** one tile per base terrain, its `_b` variant on about half the plots (a stable hash;
  the variants are `pair`ed with their base, so they meet without seams);
  coast and shallows use `tile_water`. **Hills** are an overlay chosen by the terrain: green
  (`ovl_hills`), dry (`ovl_hills_dry`: plains, desert, hell terrains) or snowy (`ovl_hills_snow`:
  tundra, snow). **Peaks** are `feat_mountain` (`feat_mountain_hell` on hell terrains); a volcano
  replaces the peak.
- **Features:** forests, jungle, scrub, flames and tormented souls stand on the tile; oasis, flood
  plains and ice are overlays.
- **Improvements:** flat works are overlays (farm, pasture, plantation, winery, quarry), the rest
  stand on the plot (mine, lumbermill, windmill, workshop, camp, cottage, hamlet, fort,
  watchtower, harbor, fishing and whaling boats, mana nodes); lairs, ruins, graveyards and the
  unique landmarks share a few sprites by kind (`improvementSprites`). The exact improvement shows
  on hover.
- **Resources:** a prop per family (`resourceSprite`: grain, cattle, sheep, horses, deer, fish,
  whales, iron, gold, gems, marble, mithril, wine, fruit, mana, trade goods).
- **Cities:** a one-plot settlement by size: `bld_city_village` (1-3), `bld_city_town` (4-8),
  `bld_city_castle` (9+), its saturated blue (the art's stock roofs, spires and flags) recoloured to
  the owner's colour (slate roofs stay slate), with the owner's banner at its front-left and a
  plaque over the front of its base (size, name, turns left on its build for your own; a star on a
  capital). As in the 2D renderer a garrison would hide its city, so it shows only while selected
  or sliding (at the gate, front-right); the city's banner carries its count.
- **Units:** the first sprite that exists of: `unit_<civilization>_<type>` (e.g.
  `unit_ljosalfar_archer`, `unit_sheaim_mage`), `<civilization>_<class>`, the type
  (`unit_swordsman` for UNIT_SWORDSMAN), the type's aliases, the class (`unit_axeman` for
  UNITCLASS_AXEMAN), the class's aliases (`UNIT_ALIASES`), settler or worker by ability, then the
  combat type's and the domain's stand-ins, then the swordsman. So a new sprite named after a type,
  a class or a civilization's type is picked up with no code change.

## 5. Drawn by the engine

As the contract says, faction colour stays a runtime choice: the **banner** beside each unit (a
pole and swallow-tailed flag in the owner's colour with a gold charge, placed just left of the
figure's opaque box so a big unit does not hide it) and each city, the **contact shadow** under each
unit, and the **selection highlight** (a gold inset diamond). Also: owner **borders** along plot
edges (under what stands up, and again faintly over it, so a forest's owner still shows),
**rivers** along plot edges (a blue channel between earth banks; an edge facing unexplored land is
left out), white **surf** where land meets water, **roads** between plot centres, the stack count on
the banner, a pip on the finial for your units (green while it can move), a health bar when hurt,
the **path** preview with turn roundels (red to an attack, orange when it declares war), the
waypoints, the target and hover outlines, the combat flashes, the **Political** and **Yields**
layers, **fog** (remembered plots desaturated and darkened, keeping their hue), and **clouds** over
the unexplored plots along the edge of the known world (unexplored plots are not drawn: the world
floats in a summer sky, the reference's). Units face right and are mirrored after a move to the left
of the screen; your own moves slide as in the 2D renderer. The map's lettering (city plaques, stack
counts, yields, path roundels) is the interface's Herald (section 9), at 1 or 2 CSS px a pixel by
the zoom; a city's plaque is a navy plate in the interface's gilt frame.

## 6. Adding art

1. Make it with the package (tools/m7/README.md): `cd tools/m7`, `export OPENAI_API_KEY=...`, then
   `python3 m7.py unit|building|feature|tile ...`, look at the review sheet, `accept`. Name it by
   the keys of section 4: a unit after its type or class (`unit_hunter`), or
   `unit_<civilization>_<type>` for a civilization's own look.
2. `python3 m7.py export` (from anywhere): the client's sprites and index.
3. If it is a new kind of thing (a new improvement family, a terrain), map it in `iso.js`.
4. Look at it in the game (`--assets`, `?renderer=iso`), then build: `build.rs` embeds the PNGs.

Costs at the measured prices: one 3-variant batch is about $0.08 (`m7.py spend`).

## 7. The sprites so far

The package arrived with 20 accepted sprites; the first batch for the game added about 110 more,
made in five parallel runs of the package (one folder each, merged afterwards), for about $12.
Everything below is in `tools/m7/roster/` with its exact prompt in `tools/m7/prompts/assets/`.

- **Terrain (tiles, each with a `_b` variant except water):** grass, plains, desert, tundra, snow,
  marsh, water (coast), ocean, burning sands, broken lands, fields of perdition.
- **Overlays:** green, dry and snowy hills; farm, pasture, plantation, winery, quarry; oasis,
  flood plains, ice.
- **Features:** forest, new forest, ancient forest, burnt forest, jungle, scrub, mountain, hell
  mountain, volcano, flames, tormented souls, mana node, fishing boats, whaling boats.
- **Resources:** wheat, cow, sheep, horse, deer, fish, whale, iron, gold, gems, marble, mithril,
  wine, fruit, mana, goods.
- **Settlements and buildings:** city village, town and castle (one plot each), the 2x2 city;
  cottage, hamlet, fort, watchtower, lumbermill, windmill, workshop, mine, camp, lair, ruins,
  graveyard, goblin fort, harbor; the mage guild and temple of leaves (not on the map yet).
- **Units (56):** settler, worker, warrior, scout, hunter, archer, crossbowman, axeman, swordsman,
  phalanx, champion, adept, mage, archmage, disciple, priest, paladin, druid, assassin, great
  person, commander, goblin, lizardman, skeleton; horseman, knight, chariot, war elephant,
  catapult, trireme, frigate, workboat; wolf, bear, lion, giant spider, griffon, werewolf, demon,
  manes, fire, earth, water and air elementals, golem, angel, treant, sea serpent, spectre; and the
  package's own lich, hyborem, hill giant, ogre, priest of leaves, ljosalfar archer, sheaim mage.

Notes from making them (also in the package README): one-tile buildings need the "CAMERA CHECK"
sentence (now the default) or their footprint comes out too steep; `--height` is the trimmed
height including a raised weapon, so units with tall spears or staves were cut taller (up to 112)
to keep their bodies at the swordsman's size; a magenta net or rigging traps the background key, so
ask for solid nets; the cottage is deliberately small, padded to sit in the middle of its plot.

## 8. Gaps and next steps

- **Roads, rivers and coasts** are drawn as lines and banks, not art: the README's main gap to the
  reference look (neighbour-aware river and coast overlays, roads along tile edges).
- **Cultures:** one settlement set for every civilization; the README's route to elven, dwarven,
  infernal and undead cities is the same prompt shape with other materials.
- **Units:** about fifty sprites stand in for 325 unit types. Heroes and civilization-specific
  units (`unit_<civilization>_<type>`) are the next batches.
- **Resources** by family; the exact resource shows on hover.
- The minimap is MapView's top-down one.

## 9. The interface

While the isometric renderer draws the map, the client's chrome is redrawn after the reference
screenshot's panels too (`assets/app/isoui.js`, `iso.css`, everything under `html.iso`): the map
fills the page and the interface floats over it.

- **Layout** (the reference's): a single top bar (the date and turn; the empire's numbers with
  their icons, "245 (+18)"; the system buttons as icon tiles: options, save, load, new game,
  keys, the raw request), wrapping onto a second row on narrow screens (the panels below follow
  its height); a rail of advisor tiles at the left (research, techs, cities, civics, religion,
  diplomacy); the turn report at the top left, folded to its tabs until one is clicked; the
  selection's card at the bottom left (the lead unit's figure on a painted meadow, its strength,
  movement, health and level, the stack, its orders as icon tiles over a description of the one
  pointed at with its key and help (a box as large as the largest of them, so pointing moves
  nothing), spells, promotions and upgrades as labelled buttons, the
  seals and the stack buttons); a city panel at the right (your capital, or the city last
  opened or selected: its picture in its owner's colour, population and growth, production,
  yields, mood and health, the build queue, "Open city"); and at the bottom right End turn, the
  map layer and zoom tiles and the minimap. The card shows only with a selection or a plot.
- **Frames**: drawn by isoui.js as nine-slices on a 2 px grid (`--hu`): a gilt rule lit from the
  upper left (outline, light gold, gold, shade, outline; every edge the same way, so it reads as
  a bar), mitred corners and, at the panels' corners, the reference's gilt knot (cut from its city
  panel's corner on the grid and cleaned, mirrored at each corner, reaching into the panel);
  thinner rules for boxes, tiles and bars; the panels' deep navy (the reference's) carries a
  faint seeded grain.
- **Lettering**: Herald (`isofont.js`), a pixel serif drawn for the game after the reference's
  lettering: capitals and ascenders 7 px, x-height 5, descenders 2, 1 px strokes; its bold is the
  pixel smear, drawn by hand for the letters whose counters the smear would close (m n w v k y M
  N W); accented letters are composed. As in the reference, rows, values and buttons are in the
  regular weight and titles in the bold. The glyphs are grids in the module, made into a TrueType
  font when the page loads (`FontFace`), so every label, button and paragraph takes it through
  CSS, wraps and selects as text. It is set at 20px (2 CSS px a pixel) with 24px lines, so every
  baseline falls on a whole pixel; the tech tree's fixed boxes take it at 10px (1x). Other sizes
  blur it: iso.css overrides style.css's 11-17px sizes.
- **Icons**: the M7 package's interface icons (`ui_<name>`, 64 of them: yields, commerce, mood,
  health, the top bar's, the menu's, the orders', the layers'), drawn by the image model on
  `tools/m7/anchors/anchor_ui.png` (the reference's panels), four sheets of sixteen, and cut to
  24 (`ui_<name>`, the order tiles) and 16 (`ui_<name>_s`) blocks of 2 px (`tools/m7/ui.py`,
  about $0.03 a sheet; prompts in `tools/m7/prompts/ui/` and `prompts/assets/ui_*.txt`). A
  worker's builds show the map's own improvement sprites (a farm, a mine, a cottage ...), a
  clearing its feature with a cancel mark, an upgrade the target unit's figure. `ui_backdrop` (the
  meadow behind the portrait and the city picture) is one painting, re-gridded. The 2D art's
  icons that have no M7 counterpart (promotion seals, religions, civic categories) stay, at 2x.
- **Leader portraits** (`portraits.js`): a bust of each leader a game can have, painted by the
  same model in the same style (`tools/m7/leaders.py`; the descriptions in
  `tools/m7/prompts/leaders/` are written from each leader's Civilopedia entry), 80 art px in
  2 px blocks, exported as `leader_<key>`. They show in every renderer: 80 px by each player in
  Diplomacy, 160 px at the top of the trade screen and on the new-game screen, 80 px on a deal
  offered to you and on the take-control question, 40 px by each candidate of a vote; in this skin in the boxes' inset frame.
- **How it is wired**: the skin follows the renderer (`main.js` `syncSkin`, on every renderer
  change): it moves the advisor buttons to the rail and End turn, the layers, zoom and minimap
  to the bottom-right cluster, and puts them back when the renderer changes. look.js's helpers
  (`pix`, `iconEl`, `uiIcon`, `barEl`) answer in the skin's look while it is on (plain text, the
  M7 icons, its bars), so the screens built with them follow without changes of their own.

### The city screen

With the isometric map the city screen is Civ4's city view (`city.js` `overlay`,
`MapView.cityOverlay`): the map glides to the city at the largest zoom step whose projected work
area fits between the screen's panels (`CITY_FRAME`), veils the revealed plots outside the radius
and edges the radius in gilt; each plot shows its yields on a plate (a worked plot's gilt-framed
and the plot lit in a gilt ring, the others' muted), each worked plot a citizen (half-seen when
another city works it), plots not yours are tinted and
the one pointed at is outlined, with what a click does as its tooltip. A click on a plot works it
or stops working it (`TASK_CHANGE_WORKING_PLOT`, as the grid's buttons do); other clicks on the
map do nothing meanwhile. The rest of the screen stands in panels at the sides (the citizens, the
city's numbers and buildings at the left; its orders and production at the right), the title,
summary and buttons as plaques; the rest of the interface steps aside, and when the screen closes
the camera goes back to where it was. The production popup stays a dialog; the other renderers
keep the grid.

### W A S D

Held, `W` `A` `S` `D` scroll the map smoothly in every renderer (`MapView.scrollHold`: a
screen-space glide that eases in and out, at most of the view's short side a second), by the
keys' place on the keyboard. Civ4's plain-letter orders on those keys keep their buttons (the
group cycle is `Tab` and `Enter`).
