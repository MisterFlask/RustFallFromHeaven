# HD2D: pixel-art sprites in the 3D renderer's lit world

**Status: experimental, phase 1, off by default.** A fifth map renderer of the browser client
(`crates/ffh-play`), beside the flat, 2D pixel art (docs/ART.md), isometric (docs/ISO.md) and 3D
(docs/R3D.md) ones. It draws the game in the manner of Octopath Traveler and Triangle Strategy: the
isometric renderer's pixel-art sprites stand as flat cards in the 3D renderer's lit world, seen
through a perspective camera tilted toward the horizon, with light shafts, glow, a mood for each
region, glowing borders and a tilt-shift blur. The look target is the owner's reference image,
[docs/art/hd2d-reference.png](art/hd2d-reference.png). It reads the game state only: no rule, save
or protocol changed for it.

Phase 1 answers one question: **do today's sprites hold up in a lit 3D world?** No new art was made
and no image model was used. The answer is in §2; the gaps that new art would close are in §10.

Contents: 1 running it · 2 the answer · 3 architecture · 4 what is reused · 5 the sprites ·
5b the mode's own art · 5c towns from a kit of cards · 6 light, moods and the post passes · 7 the lab and tools · 8 on a real GPU · 9 cities and the camera ·
10 the gaps to the reference · 11 configuration · 12 removing it · 13 limitations and next steps ·
14 provenance.

---

## 1. Running it

```
cargo build --release
./target/release/ffh-play serve            # open http://127.0.0.1:8642/?renderer=hd2d
```

* `?renderer=hd2d` for one page, or **Tools → Options → "HD-2D (experimental)"** (remembered by the
  browser like the other renderer choices). Choosing another renderer there switches back.
* The **HD-2D** button at the map's top-left opens the tuning panel (the 3D renderer's panel with
  the mode's own rows, §11) with the frame time and per-pass GPU times.
* `?r3d={"tiltShift":0}` (any key of §11) or `?r3d=plain` override the look for one page.
* It needs WebGL2 with GPU acceleration, as the 3D renderer does (docs/R3D.md §1, the same software
  guard and `?force3d=1`). On Windows, Chrome's default ANGLE backend (Direct3D 11) compiles each
  of the scene's programs at its first draw, the browser drawing nothing meanwhile: the lab's HD-2D
  scene starts in about 25 s on an RTX 4060 laptop (38-41 s while each was compiled twice;
  docs/R3D.md §11.1); with `--use-angle=vulkan` the mode starts in about a second (§8).
* The feature flag is `EXPERIMENTAL_HD2D` in `assets/app/look.js`: false hides the option and
  ignores `?renderer=hd2d`, and the mode's module (`r3d/hd2d/index.js`) is never loaded. The 3D
  renderer alone (`?renderer=3d`) never loads it either.

## 2. The answer: the sprites hold up

**Yes.** On a real GPU, in the lab composition and in a real AI game at turn 200, today's sprites
stand in the lit 3D world without new art: units read as figures (a dark one-pixel rim, a contact
shadow and their banner carry them on any ground, a busy wood or hell's red included), the
civilizations' towns sit in the land as diorama pieces at the mode's camera (30-42°, §9), pixels
stay crisp close up and do not shimmer as the perspective scales them (§8), and the scene's sun,
placed where the sprites' painted light comes from, never contradicts them. What does not hold up
is the *medium* around them: the 3D renderer's smooth painterly ground and modelled trees read as a
different hand from the pixel-art cards, and the settlement sprites' painted ground limits the
camera. Those are the gaps new art would close (§10). The review round (one visual and one code
critic on the combined result) found the layer that blends the cards into the world too heavy —
hell's grade washing rivers and roads to lava, figures lost red on red, beams and shafts washing
out the top-left, improvements oversized with hard isometric bases, groups merging into one blot,
borders reading as a grid — and that layer was reworked (the shots are after it). The code critic's
findings were fixed too: switching between the 3D renderer and the mode let the old context's loss
take the new renderer down; a failed setup left a renderer running; extreme panel values could put
the horizon in view; buildings took their fog of war from the plot's edge; the shafts ignored the
shadow switch; claimed improvements kept their models' particles.

**The second batch** went after that medium without touching unit art (§5b): pixel-art ground
layers and single-object prop cards (trees, bushes, flowers, rocks, crystals, lava rocks) drawn by
a local image model, the woods stood as crowds of small tree cards in place of the modelled trees,
a darker grade (deep shade, little haze, more contrast), settlement cards about two plots wide, a
closer and lower default camera, and a stronger tilt-shift and bloom. The land, the woods and the
figures are now one medium.

**The third batch** went after what a measured comparison with the look target found (the numbers
in §6): a palette far too saturated (foliage at 0.86-0.95 saturation against the target's 0.49) and
too dark (twice its near-black share), a view zoomed in about twice as far as the target's, the
plain interface's sidebar boxing the map in, sparse open ground and bare shores. It grades the land
to the target's palette (§6, "the land's palette"), wears the isometric map's floating interface
over a full-bleed map at the 3D renderer's gameplay zoom (§9), stands the woods as denser crowds of
smaller trees, fills open ground with clumps of small props drawn by code and rims the coasts and
river banks with stones (§5b). The image-model art for more prop kinds (huts, ruins, elven and
corrupted clutter: `tools/hd2d/gen.py`) waits on API credit; nothing was generated for this batch.
The shots below are the second batch's, on the owner's GPU; [hd2d/batch3.jpg](hd2d/batch3.jpg) is
the third's (software renders: the lab before and after, and the real game).

**The fourth batch** closed the gaps the third left (§10), still without the image model (the API
account had no credit): designed props drawn by code -- huts, a well, ruins, gardens, the corrupted
land's obelisks and altars, hell's bones and vents, elven lanterns -- with towns spilling over their
walls into houses and gardens (§5b); towns graded about half as the land is, their windows and fires
lit (§5, §6); a raised bank where the land meets its water and waterfalls where a river drops toward
the camera (§5b); the floating interface as dark glass, smaller on a desktop (§9); and the shade
pulled back toward the target's (§6). [hd2d/batch4.jpg](hd2d/batch4.jpg) shows it (software
renders).

**The fifth batch** went after the cities, which the owner judged flat: each was still the isometric
map's single settlement sprite stood up as one card, its pixels 2.8 times coarser than the prop
cards round it, unshadowed and lit as one flat tint, its isometric diamond reading as a sticker on
the land. Each city is now a town composed of many cards from a kit of pixel-art buildings drawn
for this camera by OpenAI's `gpt-image-2.5-sunburst` (§5c): its civilization's landmark at the back,
its family's houses packed round it, more and further out as it grows, walls with towers and a gate
round a walled city, every piece at the prop cards' density, lit, shaded and in its owner's colour.
[hd2d/towns.jpg](hd2d/towns.jpg) shows it before and after (software renders). It is one of two
answers being compared: a parallel one builds the towns as 3D geometry with pixel-art textures.

| | |
|---|---|
| the look target and the lab's composition after it, side by side | [hd2d/lab-vs-reference.jpg](hd2d/lab-vs-reference.jpg) |
| the mode's own art: the seven ground layers and the 25 prop cards | [hd2d/art-sheet.png](hd2d/art-sheet.png) |
| the Sheaim's corrupted land (lab) | [hd2d/sheaim.jpg](hd2d/sheaim.jpg) |
| a real game (the `stage corrupt` save at (46, 27)), 96 px a plot | [hd2d/game.jpg](hd2d/game.jpg) |
| the same, close (168) and far (40) | [hd2d/game-close.jpg](hd2d/game-close.jpg), [hd2d/game-far.jpg](hd2d/game-far.jpg) |
| the same spot in the isometric and 3D renderers | [hd2d/game-iso.jpg](hd2d/game-iso.jpg), [hd2d/game-3d.jpg](hd2d/game-3d.jpg) |
| unit groups, 2x | [hd2d/groups.jpg](hd2d/groups.jpg) |
| hell land | [hd2d/hell.jpg](hd2d/hell.jpg) |
| a settlement at four camera pitches | [hd2d/city-pitch.jpg](hd2d/city-pitch.jpg) |
| a pan-and-zoom film (10 s, not in the repository: `tools/r3d-lab.mjs --capture`) | — |

## 3. Architecture

```
assets/app/
  look.js, tools.js, main.js, map.js    the renderer choice "hd2d" (≈25 lines)
  iso.js                                unitSpriteKeys / civShortName exported (the unit's sprite as
                                        the isometric map chooses it; its behaviour is unchanged)
  r3d/
    map3d.js     + mode hooks: loadCfg/saveCfg, light, presets and panel rows, dressEnv,
                   staticExtras, dressUnitsOf, setUnits, ringOf, resourceChips, beforeFrame
    pipeline.js  + `ext`: a mode's passes (shadow, scene, post/compose, grade, underlay, ghosts,
                   additive, present); null for the 3D renderer
    dress.js     + `env.claims`: what a mode stands as sprites is not modelled
    terrain.js   + MapTextures.setMood: the per-plot mood channel
    shaders.js   + OVERLAY_FS uBorderGlow (0 for the 3D renderer: its ink border is unchanged)
    hud.js       + the resource chips ask the map (`resourceChips`)
    config.js    loadConfig/saveConfig/withPreset take the store and presets
    hd2d/
      index.js    MapHd2d: the 3D renderer's Map3D with the hooks set (the module map.js imports)
      config.js   the HD-2D preset, its light, its panel rows (stored apart: ffh.hd2d.config.3)
      dress.js    which card stands where: unit groups and banners, cities, buildings, resources,
                  light beams; the claims
      mood.js     the per-plot mood channel and its particles
      sprites.js  the sprite atlas (a texture array) and the card instance buffers
      passes.js   the pipeline extension
      shaders.js  the cards, their shadows and ghosts, the beams, the post passes
      art/        the mode's own art: gnd_*.png (ground layers), the prop cards, index.js
                  (written by tools/hd2d/art.py)
r3d-lab/          scene=hd2d, renderer=hd2d, compile.html (§7)
tools/            r3d-lab.mjs --capture, r3d-perf.mjs --renderer hd2d, r3d-compare.mjs hd2d shots,
                  hd2d/art.py (the art's post-processing)
```

The mode is the 3D renderer with a handful of hook methods overridden (`MapHd2d extends Map3D`)
and an extension object on its pipeline; with the hooks at their defaults and no extension, the 3D
renderer runs exactly as before. Data flow per change is the 3D renderer's (docs/R3D.md §3): a
static redress also rebuilds the static cards (cities, buildings, resources, beams) and the mood
channel; a view change rebuilds the unit cards; per frame only the walking figures' instances and
the uniforms change.

## 4. What is reused

Everything but the figures, the settlements, the standing improvements and the resource markers:
the ground and its relief (`world.js`, `terrain.js`, materials), water and rivers, the 3D trees
(forests are whole-tile clumps; there are no single trees cut for cards yet, §10), rocks, fields
and flat improvements, roads and bridges, the decal and light layers, picking, the camera and its
zoom steps, the shadow, SSAO and glow passes, the compose/finish grade, FXAA, the overlays (with a
glowing border, §6), particles, the HUD (city plates, stack badges, far-zoom counters) and the
lab and tools. The camera is the 3D one with the HD-2D preset's pitch and lens (§9).

## 5. The sprites

* **Atlas** (`sprites.js`): the isometric renderer's PNGs (`assets/app/iso/*.png`, indexed by
  `iso/sprites.js`) loaded on first use into a `TEXTURE_2D_ARRAY` of 256 × 256 layers, one sprite a
  layer at its top-left (the 2x2 city, 384 wide, at two thirds), premultiplied (so the mipmaps
  average colour and coverage together: no dark fringe) and mipmapped; the array doubles when full.
  Banners are drawn by code into the same array (the owner's colour stays a runtime choice).
* **Cards**: one instanced quad per card, its base point on the ground. It faces the camera
  (`lean` 1; 0 stands it upright, foreshortened by the pitch), so a sprite keeps the proportions it
  was drawn with. Its **depth is an upright card's** at the same base (the vertex shader puts the
  upright card's NDC depth on the facing card's position): it hides and is hidden as a standing
  figure would be, and the ground behind it never cuts into its leaning top. A per-card bias pulls
  the depth toward the eye: figures (0.26; a city's garrison 0.4) stand before the card of the
  building they stand at, resources (0.08) before the ground they lie on.
* **Sharp pixels at any size**: a sharp-bilinear filter while a texel covers more than a screen
  pixel (each texel a flat square, blended with its neighbour over one screen pixel at its edge:
  crisp, no shimmer under the perspective's continuous scaling), ordinary trilinear mip filtering
  when the card is smaller than the art. Cut out by alpha (0.5), so cards need no sorting.
* **Tinted, not re-lit**: the light painted into the sprite (from the upper left) is kept. A card
  is tinted by the light where it stands, relative to full sun on open ground: the sun if it
  reaches the base (four taps of the props' shadow map), the sky's ambient, a little of the local
  light pools; then dimmed and cooled where only remembered (the fog of war). Its colour is written
  through the inverse of the compose pass's filmic curve, so a card in the open comes out of the
  pipeline as the art's own colour. The region moods grade buildings' cards as the land they stand
  on, but figures and banners only by a quarter of the change in value, never in hue (a red imp on
  hell's red must still read).
* **Readable on every ground**: figures and banners get a dark rim one screen pixel wide inside
  their silhouette (from the alpha a pixel away, at the card's own mip level, so it survives
  minification), a soft contact shadow, their outline cast into the sun's shadow map (an upright
  card turned to the sun), a thin team ring round the group (gold and filled when selected), and an
  x-ray silhouette in the owner's colour where scenery hides them (as the 3D figures' ghosts).
* **Groups and banners** (`dress.js`): a stack is its head unit's sprite (chosen exactly as the
  isometric map chooses it: `iso.js` `unitSpriteKeys`, so every unit type and every
  civilization's own look works) drawn three times — two in front, one behind between them, about
  0.6 of a figure's width apart so each reads as a figure, the one behind 6% smaller — or twice for
  settlers and workers, or once for a big sprite (wider than 110 or taller than 130 art px), a
  ship, a flier or an engine. The owner's banner (a gonfalon in the owner's colour with a pale star,
  gold trim and finial) stands behind the group's left, its cloth above the figures' heads; it
  ripples. A walking stack's figures glide as the 3D figures do; a figure that last walked west is
  mirrored (the art faces right).
* **Settlements and buildings**: a city is a town composed of the kit's cards (batch 5, §5c), with
  the owner's banner at its front-left (before batch 5, and still for a family without the kit, it
  was its civilization's village, town or castle sprite by size, `iso.js` `citySprites`, its card's
  foot on the plot's front edge so its painted ground lay over the plot); its garrison is one figure
  (80%) in front of it, at the front-left corner or, out of a river, the front-right or the front's middle
  (the banner's badge carries the count). A standing improvement (`iso.js` `improvementSprites`:
  cottages, forts, towers, mines, mills, camps, lairs, ruins, mana nodes, boats) is a card likewise,
  smaller than a city, its painted ground thinned out over its foot by a dither fixed to the art's
  texels (it melts into the 3D land instead of ending in the isometric diamond's edge); flat ones
  (farms, pastures, plantations, wineries, quarries) stay the 3D renderer's fields. A resource is its family's prop at the plot's front-left. A town's card (batch 4: keep mask 0.95)
  takes about half the moods' and the land palette's grade, so it keeps its own colours, and its
  warm lights (windows, fires, braziers: orange to yellow, not its red roofs or blue domes) glow. Mana and gems glow (their bright, saturated pixels
  feed the bloom). What a card stands for is **claimed** from the 3D dressing (no house, wall,
  structure or marker is modelled under it); trees keep out of a town's reach as before.
* Scale (`dress.js` `PX`): a unit's 96 × 120 canvas is 0.46 × 0.58 of a plot, a 192-wide
  settlement 1.7 plots (as the reference's towns, which span about two tiles; the prop cards keep
  clear of its footprint; the kit's towns, §5c, stand at the props' scale instead), other buildings 0.73, resources' canvases 0.65 (their props about 0.25),
  banners 0.8 tall, a lone prop 0.0031 a plot an art pixel (a 150 px oak about 0.47 of a plot; in
  a wood 0.58 of that, about 0.27: the target's trees are about half a figure's height), the props
  drawn by code 2.6 times that (`FINE`: about one art pixel a screen pixel at 96 px a plot).

## 5b. The mode's own art: pixel ground and prop cards

The first review found the gap was the medium around the sprites (§10). The second batch closes
its two largest parts with art of the mode's own, drawn by a local image model (FLUX 3 Image Klein
in ComfyUI on the owner's PC) and brought to one contract by `tools/hd2d/art.py`, as `tools/m7`
does for the isometric sprites. No unit art changed.

* **Pixel-art ground** (`hd2d/art/gnd_*.png`): grass, dry straw, packed dirt, sand, tundra moss
  and lichen, snow and broken rock, drawn top-down, then pixelated to 128 art pixels a tile (two
  texels each: about the sprites' density on screen), their palettes cut to 40 colours, made
  seamless by a dithered cross-fade (each edge takes, more often the nearer it is, what lies one
  tile further in the source), their height taken from their value into alpha. The mode loads them
  into **its scene's** material layers (`materials.js` GRASS, STRAW, DIRT, SAND, LICHEN, SNOW,
  ROCK; the crop, reed and noise layers stay), with their mean colours: the terrain shader tints a
  layer by its mean (`matTint`, `matShade`), so the land keeps its painted design — which ground
  lies where, the meadow's golden patches, the plot centres — and takes the pixel art's texture.
  No shader changed; the 3D renderer's own layers are untouched.
* **Prop cards** (`hd2d/art/*.png`, indexed by `hd2d/art/index.js`): single trees (broadleaf,
  pine, snowy pine, jungle, ancient elven giants, dead), bushes, wildflowers, boulders, violet
  crystals and lava rocks, drawn as single objects on white, cut out, scaled to a card a set
  number of art pixels tall, their palette cut and a dark outline added; the trees and bushes with
  their middle values deepened (a gamma of 1.25-1.35: the model paints foliage a bright lime, the
  look target's woods are deep greens under bright tips). **Woods** stand as a crowd of small
  trees, a jittered four-by-four a plot at three quarters of a lone prop's scale (an oak about 0.35
  of a plot tall), the back row left out and the middle kept low where something stands on the plot
  behind (species by feature and ground: oaks and pines,
  pines and snowy pines in the cold, jungle trees, an ancient wood's one or two giants among oaks,
  dead trees in burnt woods and hell), thinned where a stack or an improvement holds the plot; **open
  ground** scatters bushes, flowers, a lone tree, boulders (more on hills and peaks), crystals on
  corrupted land and lava rocks on hell ground. They keep off rivers, roads, a town's footprint and
  the spot a stack stands on; they cast their outline into the sun's shadow map and a contact
  shadow, so a wood is dark beneath. What they replace is **claimed** from the 3D dressing
  (`claims.trees`, `.flora`, `.rocks`): no modelled tree, plant or rock stands in the mode.
* **Small props drawn by code** (`tools/hd2d/draw.py`, batch 3): grass tufts (green and dry),
  reeds, pebbles, shore stones (rows of wet, mossy stones, a band much wider than tall), standing
  stones, fences and haystacks -- each a few muted tones a material shaded from the upper left, with
  the cards' dark outline, written as art cards (`art.py index` lists them). Small enough that code
  draws them well; anything with a design (a hut, a ruin) is left to the image model.
* **Density** (batch 3): woods are five by five trees a plot (four by four before) at 0.58 of a
  lone prop's scale, with grass at their sunny edge; open ground carries about a dozen props, most
  gathered round one or two clumps a plot (a lone tree with bushes, flowers and grass round it, a
  rock among pebbles), now and then a ring of standing stones; farms, pastures, cottages and the
    land round a town add fences and haystacks.
* **Designed props drawn by code** (batch 4, until the image model draws better ones): thatched and
  stone huts (a window lit from within), a well, ruins, gardens; the corrupted land's obelisks (violet
  runes) and altars, hell's bones and smouldering vents, the elven lands' lanterns -- each region's
  own, by the moods (`clutterOf`), now and then a ruin anywhere. What carries a light (`GLOWS`: runes,
    lamps, embers, hut windows) feeds the bloom.
* **The image model's versions** (`tools/hd2d/props.json`, `tools/hd2d/local_props.py`): every
  designed prop as one entry -- the card it replaces (a numbered new name joins its kind's
  variants: `hut_3`, `ruin_3`; a new kind, `cart`, `shrine`, stands once its card exists), what to
  draw, its height in art pixels (the code-drawn card's, so the dressing's scale holds). On the
  owner's PC the local FLUX in ComfyUI runs them for free:
  `python tools/hd2d/local_props.py --workflow W.json` (W.json: a text-to-image workflow saved in
  ComfyUI's API format; the script writes the prompt, negative, seed and a 1024 square into it),
  three candidates a prop by default, a contact sheet (`target/hd2d-local/sheet.png`), then the
  chosen ones (`--pick hut_1=2,...`; `--cards` to re-card without generating) through `art.py prop`
  and `index`. Tested against a stand-in ComfyUI server only: the first real run is the owner's.
* **Towns spill over their walls** (batch 4, `townRing`): on the open plots within two of a town,
  houses (one or two a plot next to it, now and then one beyond), gardens, a well, fences and
  haystacks, as the look target's towns spread into the land round them.
* **Raised banks and waterfalls** (batch 4): the mode's world raises dry land 0.04 over its water
  (world.js `bank`, `MapHd2d.worldOpts`; the 3D renderer's is 0), all but a narrow strip over the
  waterline, so the shore climbs in a short steep step the terrain shader draws as rock, a boulder of
  the bank now and then among the shore stones. (The terrain mesh, six vertices a plot, cannot draw a
  true cliff face.) Where a river's surface drops steeply toward the camera on land (off the hills,
  down a bank; not at its mouth, where the ribbon widens flat into the sea) a card of white water as
  wide as the river and about as tall as the slope looks stands across it (`dressFalls`,
  `falls_*`); a river running across the view keeps its slope unmarked, as a card would face the
  wrong way.
* **Shores** (batch 3, `dressShores`): the look target's land ends at its water in a rim of dark
  stones. Along every coast and lake shore a 14 × 14 sampling of the plot (and a quarter plot past
  its edges, as the warped coast wanders off the grid) finds where the ground falls through the
  waterline and stands a shore stone or pebbles there; along rivers, just outside the water's edge
  (`RIVER_HALF` + 0.01-0.055), smaller stones, pebbles and now and then reeds. One a cell of 0.17
  plot at most, on a global grid, so neighbouring plots never stack theirs.

## 5c. Towns from a kit of cards (batch 5)

The fifth batch closes §10's second gap, "cities drawn for this camera". The isometric settlement
sprite was drawn for a 2:1 view along a plot's diagonal, at 0.0088 of a plot an art pixel; the land
round it was the prop cards' 0.0031. Its painted walled diamond stood up as one card, so a city
cast no shadow, took one tint, and read as a sticker. A city is now built from a kit of single
buildings drawn for this camera, each its own card.

* **The kit** (`tools/hd2d/towns.json`, `art/town_*.png`: 121 cards). Eleven architecture families:
  the 3D renderer's (`architecture.js` `CIV_FAMILY`), with its sub-styles made families of their own.
  * human: the Bannor, Elohim, Mercurians, Kuriotates, Grigori, Hippus and Balseraphs;
  * arcane: the Amurites and Sidar; north: the Illians and Doviello;
  * elven: the Ljosalfar; darkelf: the Svartalfar;
  * dwarven: the Khazad and Luchuirp; gothic: the Calabim; infernal: the Infernal and Sheaim;
  * tribal: the Clan of Embers and the barbarians; desert: the Malakim; coastal: the Lanun.

  A family has four houses, a tower, two wall segments, a gate and a corner tower
  (`town_<family>_<piece>_<n>`). Each of the 22 civilizations, barbarians included, has a landmark
  (`town_lm_<civ>`): a keep, temple or palace of its culture that rises over the roofs. Examples:
  the Bannor's fortified cathedral, the Ljosalfar's spires woven round a living tree, the Khazad's
  hold carved into a crag, the Sheaim's planar-gate ziggurat, the Lanun's lighthouse keep. Every
  piece is one object, seen front-on from about 30° up, looking along the plot's side (not the
  isometric diagonal), lit from the upper left, with its foot a straight line. Heights are in art
  pixels:

  | piece | art px | plot heights |
  |---|---|---|
  | house | 64-104 | about a quarter of a plot |
  | tower | 140-160 | |
  | wall segment | 54-62 | |
  | gate | 100-106 | |
  | corner tower | 108-118 | |
  | landmark | 190-242 | about three quarters of a plot |

  The cards stand at `PX.prop`, the prop cards' density. The two widest landmarks were drawn
  shorter, so that no card exceeds the atlas's 256-texel layer.
* **The owner's colour.** The prompts reserve saturated royal blue for what should carry the owner's
  colour: roofs, domes, spires, banners. The atlas makes each piece's owner variant as it loads
  (`SpriteAtlas.owned`: `key@css`, recoloured by `iso.js` `ownerBand`, the isometric map's recolour,
  now shared). The old city card in this mode was never recoloured: its blue showed for every owner.
* **The layout** (`dress.js` `dressTown`) is deterministic from the city's id.
  * **Reach.** The town's reach is the 3D renderer's (`CATALOG.cityReach`), so the HUD's plate,
    which stands just in front of that reach, stays in front of the town: an ellipse `reach + 0.06`
    across, `0.78 reach` deep, centred 0.05 behind the plot's centre. A village keeps to about its
    plot; a great walled city spills to about 0.9.
  * **Pieces by size.** The landmark stands at the back centre (at 0.86 for a village). A town
    (size 4+) has one family tower, a city (9+) two, at the back corners. Houses number 4 + 1.6 a
    size point, 2 more for a capital, at most 26. They are scattered at random in the ellipse, the
    taller ones toward the back, so the roofs climb toward the landmark.
  * **Walls.** A walled city (`city.walls`) gets a ring just outside the houses: the gate at the
    front middle, corner towers at the sides and the four corners, and a segment every 0.24 or so
    of the ring between them. On the sides, where the ring runs into the view, the segments stand
    one behind another.
  * **Overlap.** Two pieces at about the same depth never overlap. One well in front of another may
    hide its lower part: roofs behind roofs, a dense town.
  * **Where pieces may stand.** They keep off water and rivers, a lane along each road from the
    plot, and the garrison's and banner's spots. They also keep off a neighbouring plot that is
    unrevealed, a peak, or holds a city, an improvement or a stack.
  * **Each piece's card.** Each piece is its own card with the town flag: half graded, its warm
    lights glowing. It casts its outline into the sun's map and a contact shadow, is tinted by the
    light at its own base, and is fogged as the city's plot.
* **Round the town.** The props keep out of its reach, and out of a lane before its middle: the
  way to the gate, where the plate stands. The houses spilling onto the plots round it (`townRing`)
  are the town's own two lowest houses, in its owner's colour. Code-drawn huts remain only round a
  town whose family has no kit.
* **Fallback.** A family with no cards stands as the isometric settlement, as before.
* **Making it.**
  * `python3 tools/hd2d/gen.py town:<family> --quality high --size 1024x1024 --ref human.png`: a
    family's sheet in a 3 × 3 grid, the accepted human sheet as the style reference.
  * `gen.py landmark:<n> --quality high --ref <its families' sheets>`: four landmarks in a 2 × 2
    grid, 1024 × 1536.
  * `python3 tools/hd2d/towns.py SHEET_PNG SHEET [k=PIECE ...]`: cuts the sheet apart (`art.py
    split`, cut by the sheet's alpha; a small part beside an object, such as a floating crystal or
    smoke, joins it) and cards each piece at its height. Cards are cut to 48 colours and saved as
    palette PNGs (lossless, a third of RGBA's size: the kit is 1 MB).
  * Then `art.py index`.

  `gen.py` streams its calls: high quality takes 40-50 s, and a silent connection was cut by the
  environment's proxy (HTTP 502) after about a minute. Every sheet was looked at and accepted
  (prompts in `towns.json`). The cost was $1.19 for 19 calls at list prices, recorded in
  `tools/hd2d/usage.jsonl`. Eight more calls ended in an HTTP 502 before streaming; whether those
  were billed is unknown.

## 6. Light, moods and the post passes

* **The sun** stands at the upper left of the view (west and a little north, about 45° up), where
  the sprites' painted light comes from; shadows fall toward the lower right. The camera never turns.
  Little fill comes from the sky and the ground, and cast shadows are less floored than the
  painterly look's (`stylize` 0.7): shade goes deep, as the look target's near-black woods and
  hollows under bright accents.
* **Region moods** (`mood.js`): a per-plot RGBA channel in the terrain data
  (`MapTextures.setMood`), from what the player knows: **r hell** (hell terrain; flames, volcanoes,
  souls), **g corruption** (the Sheaim; part of the Infernal and Calabim), **b lush** (the
  Ljosalfar; the Elohim and Mercurians, ancient forests), **a gloom** (the Svartalfar, Calabim,
  Illians, Doviello, the Clan of Embers). A civilization's own entry first, else its owner's
  alignment (good: a little lush; evil: a little gloom and corruption). The sea takes half of its
  owner's mood; hell overrides the others where it holds. Unrevealed plots carry none. Read LINEAR
  with a noise-warped lookup, so a region's edge is organic, not the plot grid; off the map's
  edges there is none. The **mood pass** grades the display colour under each pixel (found from the
  depth): lush deepens the greens and warms the light; corruption darkens and cools the land toward
  violet and slate by a multiply (its contrast kept), its bright accents pushed to magenta; hell
  chars it to a dark charcoal-brown — its water left dark steel-blue, what glows (the scene's
  emission) left hot — and splits the ground with **glowing cracks** (a world-anchored network,
  broken into separate fissures, pulsing; on the ground only); gloom cools and dims. Each mood's air
  tints the distance a little. Violet motes drift over corrupted land and gold-green ones in lush
  woods.
* **The land's palette** (batch 3, in the mood pass, `landGrade`): measured on the look target's
  map area against the mode's shots (medians; saturation and value as HSV):

  | | brightness | saturation | pixels over 0.6 saturation | foliage saturation / value | near-black |
  |---|---|---|---|---|---|
  | look target | 0.31 | 0.35 | 17% | 0.49 / 0.34 | 11% |
  | lab, batch 2 | 0.21 | 0.89 | 79% | 1.00 / 0.26 | 29% |
  | lab, batch 3 | 0.30 | 0.37 | 9% | 0.44 / 0.33 | 7% (before a last pull-back of the shade) |
  | turn-200 game at 96 px, batch 2 (GPU) | 0.32 | 0.64 | 55% | 0.91 / 0.21 | 20% |
    | the same, batch 3 (software) | 0.35 | 0.36 | 11% | 0.43 / 0.32 | 5% |
  | lab, batch 4 (towns half graded, the shade pulled back) | 0.30 | 0.39 | 15% | 0.47 / 0.33 | 8% |

  The target is earthy and muted: its saturation is in the accents (the gold town, the Sheaim's
  violet, lava, the banners) and its shade is a dark olive, not black. The second batch read it as
  near-black woods and raised the saturation (1.16): the opposite of the target. So every pixel but
  the figures and banners (the keep mask) loses chroma by hue -- earth and foliage (orange through
  green) about a third, water about two fifths, the rest a tenth -- except what glows (the
  emission) and gold (bright, saturated yellow-orange); greens are pushed toward olive and water
  toward slate teal; the middle values are lightened a little (a gamma of 0.84) and the shade lifted
  toward a dark olive floor. The finish pass's saturation is 1.0 and its contrast 1.12 (1.16 and
  1.22 before); the lush mood deepens the greens in value only (its saturation boost, 1.28, went).
  Re-measure with the table's columns after a change to the grade or the art.
* **Light shafts** (half resolution): the sunlit haze — the sun's light scattered along each view
  ray near the ground, through the gaps of a high drifting cloud layer and past the props' shadow
  map (anchored to the world) — and painted **beams**: broad soft beams fanning from the sun beyond
  the upper left, strongest over the far land, turning slowly with the camera's position (a distant
  layer, not stuck to the lens), full from 96 px a plot (the gameplay zoom) and gone by 48. Seen from
  above, a view ray crosses too little air for the physical shafts alone to show, so the beams are
  painted, as in the target. They lie in the air before the land: over a figure, a third as strong.
* **Wide bloom** (quarter resolution) of the emission and the brightest light, added before the
  filmic curve; non-finite pixels give no light (a lone NaN would otherwise spread into a block).
* **Light beams** over mana nodes (1.5 plots tall), mana resources and mages' towers: soft additive
  columns in the node's school colour (`palette.js` MANA), shimmering.
* **Glowing borders**: the overlay's border becomes a hot core in the owner's colour, meandering a
  little inside the owner's side (so the plot grid does not show in it; where two realms meet each
  draws its own colour), its light spilling softly over that side; **no grid**.
* **Mended pixels**: the HD-2D compose gives a non-finite pixel its neighbours' mean, as a guard
  (the 3D renderer's terrain wrote some on a real GPU and its water spread them, until fixed:
  docs/R3D.md §5.2).
* **Tilt-shift** (as the frame goes to the canvas): sharp in a band across the view at the hovered
  plot (else the selection, else the middle; eased, so the band glides), blurring toward a half-
  and then a quarter-resolution copy above and below it — a little more at the top (far) — but
  never past a mild softening; off at the far zooms (fully on from 90 px a plot).

## 7. The lab and tools

* **Lab**: `node tools/r3d-lab.mjs --gpu --out a.png --query "scene=hd2d&renderer=hd2d&ts=128"`.
  `renderer=hd2d` draws any lab scene in the mode; `scene=hd2d` is the look target's composition
  (an elven city on a river bend in the Ljosalfar's forest, snowy peaks, a golden town on the hills,
  the Sheaim's corrupted land with its city, a mana node and a mage's tower, infernal hell land to
  the south-west and south-east, groups of archers, warriors, a hunter, adepts, mages, imps and
  wolves). The lab waits for the sprites before it is ready.
* **Film**: `node tools/r3d-lab.mjs --gpu --capture out.mp4 --query "scene=hd2d&renderer=hd2d"`
  renders a scripted pan, zoom in, pan with the pointer wandering (the tilt-shift's band follows
  it), zoom out to the far view and back, each frame at its own time, and encodes it with ffmpeg.
* **Shader compile times**: `r3d-lab/compile.html` (served by `--serve`) compiles every program of
  the 3D renderer and of the mode, one at a time, and logs each one's link and first-draw times
  (`?only=terrain`; docs/R3D.md §11.1).
* `tools/r3d-compare.mjs` also saves `hd2d.png`, `hd2d-close.png`, `hd2d-far.png`;
  `tools/r3d-perf.mjs --renderer hd2d` measures the mode (configs: hd2d, no-rays, no-tilt, no-bloom,
  no-mood, no-hd2d-post, plain).
* `--gpu` on Windows: `FFH_CHROMIUM_CHANNEL=chrome` (Playwright's own Chromium may be refused:
  "spawn UNKNOWN") and `FFH_ANGLE=vulkan` (§8); `--gpu` now also passes
  `--force_high_performance_gpu`, without which a laptop's integrated GPU draws.
* Browser smoke test (`tools/web-smoke.mjs`): a `?renderer=hd2d` step — the frame is drawn and not
  blank, the sprites load and the units stand as cards, 3D → HD-2D → art.
* Node tests: `assets/test/hd2d.test.mjs` (every unit type has a sprite; a stack is one to three
  figures of one sprite and one banner, deterministic; cities, buildings and resources are claimed
  as cards, a city a town of the kit's cards at the props' density in its owner's colour; towns
  grow with their size, walled ones have walls, a gate and towers, deterministic; the houses round
  a town are its own; nothing stands on, and no mood comes from, an unrevealed plot).
* **Art**: `tools/hd2d/draw.py` (the props drawn by code), `tools/hd2d/local_props.py` (the image
  model's, through a local ComfyUI, §5b), `tools/hd2d/gen.py` (sheets from OpenAI's
  `gpt-image-2.5-sunburst`, pinned to its 2026-09-08 snapshot -- gpt-image-1 was shut down on
  2026-10-23 -- the prop sheets of nine and the town kit's sheets of `tools/hd2d/towns.json`, with
  `--ref` images to match an accepted sheet's style; cut apart by `art.py split SHEET PREFIX`, or
  for the town kit by `tools/hd2d/towns.py`, §5c), `tools/hd2d/art.py ground SRC NAME` (a top-down texture to a seamless ground layer),
  `prop SRC NAME HEIGHT [DEEPEN]` (an object on white to a card), `index` (writes
  `art/index.js`); run it with Python with Pillow and numpy (ComfyUI's bundled one has both). The
  images came from the local FLUX 3 Image Klein (the owner's `local-image-gen` skill); `build.rs`
  embeds `assets/app/r3d/hd2d/art/*.png` beside the isometric sprites, and the server's client
  paths may now have five parts (`app/r3d/hd2d/art/x.png`).
* `ffh-play stage corrupt DIR` now tries seeds 11 to 30 until one has an evil civilization holding a
  capital at turn 200 (an AI change had left seed 11 without one) and reports the seed it used.

## 8. On a real GPU

Measured on the owner's machine: Windows 11, an NVIDIA GeForce RTX 4080 Laptop GPU (driver 621.01)
beside an Intel UHD integrated GPU, headless Chrome 156 (`FFH_CHROMIUM_CHANNEL=chrome`).

* **The renderer string.** Headless Chrome draws with the *integrated* GPU unless told otherwise
  ("ANGLE (Intel, Intel(R) UHD Graphics … Direct3D11)"): `--gpu` now passes
  `--force_high_performance_gpu`, and the tools print "ANGLE (NVIDIA, NVIDIA GeForce RTX 4080 Laptop
  GPU … Direct3D11)" — or, with `FFH_ANGLE=vulkan`, "… Vulkan 1.4.357 (NVIDIA NVIDIA GeForce RTX 4080
  Laptop GPU …)". Never a software rasteriser. A player's browser may make the same choice: on a
  laptop with two GPUs, Windows' graphics settings decide which one Chrome gets.
* **Direct3D 11 gets through now, slowly at first.** On ANGLE's Direct3D 11 backend (Chrome's
  default on Windows) the 3D renderer's terrain shader (`TERRAIN_FS`) had not finished compiling
  after 15 minutes, so neither the 3D renderer nor the mode started for Windows players on Chrome's
  defaults. It now compiles in 4.8 s (docs/R3D.md §11.1: the grounds are no longer painted in a
  loop over the four plots, and the noise is a third of its size), the water in 1.5 s (5.3 before),
  the props in 1.0 s and the HD-2D cards, shafts and mood in 0.03-0.08 s each, all in parallel
  while the page stays live. The first frame then stalled for 7-8 s more there: ANGLE compiled the
  terrain, water and prop pixel shaders a second time at their first draw into three targets
  (4.4-4.8, 1.4-1.6 and 1.0-1.1 s; the cards 0.03-0.07 s). Since 2026-10-30 that first draw is their
  only compile, made before the first frame, one program at a time (docs/R3D.md §11.1, "Compiled
  once"). On Vulkan every program compiles in about 0.2 s together and the first frame takes 0.3 s;
  every measurement below is on Vulkan.
* **Per-pass cost** (`tools/r3d-perf.mjs --gpu`, the `stage corrupt` save at (46, 27), a 1920 × 1080
  page whose map canvas is 1580 × 794, 15 frames a case, median frame; GPU times from
  `EXT_disjoint_timer_query_webgl2`, ms; one run each — on this laptop GPU, runs of one case vary by
  a few ms, so only differences larger than that mean anything):

  | ms | 3D painterly ts 40 | HD-2D ts 40 | 3D painterly ts 96 | HD-2D ts 96 |
  |---|---|---|---|---|
  | shadow | 1.1 | 0.6 | 0.6 | 0.5 |
  | scene (terrain, props, cards) | 10.7 | 14.2 | 9.9 | 9.8 |
  | water | 1.4 | 1.0 | 1.0 | 0.7 |
  | ssao, glow, compose, finish, fxaa | 0.2 | 0.1 | 0.6 | 0.3 |
  | rays + bloom + mood + tilt-shift (HD-2D) | — | 0.4 | — | 0.4 |
  | overlay | 0.2 | 0.1 | 0.3 | 3.0 |
  | **frame (wall, with a read-back)** | **21.6** | **20.8** | **16.1** | **16.1** |
  | drawn | 2394 draws, 1.03 M prop tris | 2986 draws, 1.32 M | 1312 draws, 0.83 M | 1310 draws, 0.81 M |

  The mode's own post passes cost about 0.1-0.2 ms each. Its scene pass costs more at the far zoom
  because the lower camera sees more of the map (2986 draw calls against 2394). Its overlay stage
  costs about 3 ms at 96 px against 0.3: not the light beams, the contact shadows or the glowing
  border (each off in turn leaves it at 2.6-3.8 ms); the overlay redraws the terrain mesh twice
  (in front and ghosted) and the low camera puts more terrain on screen — to profile in phase 2.
  At 1280 × 800 (canvas 940 × 640) frames are 8-12 ms in both. Start-up (sync, CPU): world 9 ms,
  textures 61, dressing 166, instance upload 118, paint layers 101 — as the 3D renderer's.
* **After the second batch** (the same save, page and runs; `r3d-perf --renderer hd2d --configs
  hd2d --zooms 40,96,128`): frames **6.1, 4.5 and 5.9 ms** at 40, 96 and 128 px a plot (16-21 ms
  before), scene pass 1.6, 1.5 and 2.9 ms; prop triangles 154k, 60k and 70k (1.32 M and 0.81 M
  before): the cards that stand for the woods, plants and rocks are a quad each, where the models
  they replace were hundreds of triangles. Sync: dressing 110 ms, instance upload 19 ms.
* **Panning stability** (`tools/r3d-lab.mjs --gpu --stability`, `scene=hd2d` at 96 px, a 4 px pan,
  mean |diff| over 0-255 and the share of pixels off by more than 24, `{"animate":false}`):

  | | mean | > 24 |
  |---|---|---|
  | HD-2D | 5.47 | 6.62% |
  | HD-2D, tilt-shift off | 6.13 | 7.87% |
  | HD-2D, tilt-shift and shafts off | 5.68 | 7.29% |
  | HD-2D plain | 5.73 | 7.44% |
  | 3D painterly | 3.64 | 3.83% |
  | 3D painterly with HD-2D's camera (pitch 36, fov 30) | 5.82 | 7.61% |
  | HD-2D, a 1 px pan | 2.27 | 1.63% |

  The mode scores as the 3D renderer does **with the same camera**: the difference to the 3D
  renderer's default is the lower, wider camera's parallax (a pan moves near things more than far
  ones; the tool realigns by one shift), not shimmer from the cards, the shafts or the blur.
* **Film**: `hd2d-pan-zoom.mp4` (10 s, 30 fps, 1280 × 720, `--capture` on the lab scene: a pan, a
  zoom in, a pan with the pointer wandering, a zoom out and back), rendered on the GPU in 85 s. On
  consecutive frames the figures keep their shape and value (no sparkle, no swimming pixels), the
  tilt-shift's band glides after the pointer, and the far zoom fades the blur and the beams out.

## 9. Cities and the camera

The camera is the 3D renderer's with a lower pitch, a narrower swing and a wider lens (`pitch` 32,
`fov` 30 against 50 and 24; `pitchAt`): 38° from the horizon at the farthest zoom, about 32° at the
default zoom and 26° at the closest, so the top of the view runs out toward the horizon. The default
zoom is the 3D renderer's 96 px a plot (128 in batches 1-2): with the map full bleed under the
isometric interface's floating panels (below) it shows about fifteen plots across, as the look
target does, and its figures take about the target's share of the screen.

**The interface** (batch 3) is the isometric map's (`isoui.js`: the floating panels, the advisors'
rail, the city panel over the map; main.js `syncSkin`), as the look target's, not the plain one's
sidebar and bars round a boxed map. Its unit portraits and build icons take the sprite the
isometric map would choose (`unitKeyOf`), as there is no isometric map behind it. Over the HD-2D
map it is a variant (batch 4, iso.css `html.iso.hd2d`, set by `syncSkin`): its panels are
near-black glass under the gilt frames (the map shows through, a little blurred), and on a desktop
they are drawn at 80%, so the map, not the panels, holds the screen.

The figures are cards that face the camera: they look the same at any pitch. The settlements and
buildings are cards too, but their art has its ground painted in, drawn for the isometric map's
2:1 view (about 30° up, and seen along a plot's diagonal, where this camera looks along its side).
Rendered at 30, 40, 50 and 60° ([hd2d/city-pitch.jpg](hd2d/city-pitch.jpg)): at 30-40° the city
sits in the land; at 50° it starts to read as a card standing on the ground (its painted ground no
longer matches the terrain's foreshortening); at 60° it plainly is one. **The steepest camera the
settlement sprites tolerate is about 45°**, so the mode's camera keeps between 30 and 42° at every
zoom.

## 10. The gaps to the reference

Batches 3 and 4 closed the gaps the second batch's review ranked (the palette, the framing, the
interface, prop density, designed props, towns, shores and banks). What remains, by how much of the
distance each closes:

1. **Art with a hand behind it** (the image model). The props drawn by code read well at their size
   but are simple; huts, ruins, shrines and each region's things drawn by the image model would
   match the trees' and the target's detail. `tools/hd2d/gen.py` asks for them in sheets of nine
   (`gpt-image-2.5-sunburst`, billed by token: $5 a million text tokens in, $8 image in, $30 image
   out; `art.py split` cuts a sheet apart).
2. **Cities drawn for this camera** (new art): closed by batch 5's town kit (§5c). What remains:
   the pieces are flat cards, so a wall ring's sides are segments standing one behind another
   rather than a wall running into the view. The landmark cannot exceed the atlas's 256-texel
   layer at the props' density, about 0.8 of a plot; the reference's palace is somewhat taller
   against its figures. The parallel answer, towns as 3D geometry with pixel-art textures, would
   close the first; a larger atlas layer for a few cards would close the second.
3. **True cliffs and wide water** (code, the terrain). The raised bank is a steep step, not a cliff
   face: the terrain mesh (six vertices a plot) cannot draw one; the target's rivers are broader and
   braided. A finer mesh along shores, or cliff cards along the bank, would close it.
4. **Darker, moodier sprite variants and figures facing left or animated** (unit art; set aside by
   the owner).
5. **The sky and the map's edge** (code): a small map's edge shows the 3D renderer's void under
   the shafts.
6. **On a real GPU**: batches 3 and 4 were judged on software renders; the palette's numbers should
   be re-measured on the owner's GPU (§6's table).

## 11. Configuration

`r3d/hd2d/config.js`: the `hd2d` preset (the 3D renderer's painterly switches with: a lower camera,
`pitch` 32 and `fov` 30; `grid` 0; deeper shade, `stylize` 0.7, `ssao` 0.85; `saturation` 1.0 and
`contrast` 1.12 (batch 3: the land's palette is graded in the mood pass), `vignette` 0.42, `haze`
0.1, `mist` 0.08, `clouds` 0.3, `glow` 1.0) and the mode's own switches: `tiltShift` (0.6), `rays`
(0.6), `mood` (1), `landGrade` (1, the land's palette, §6), `bloom` (0.8, from a lower threshold),
`beams` (1), `borderGlow` (1), `contact` (1), `lean` (1). `LIGHT` gives little sky and ground fill
(0.5 and 0.15). The `plain` preset turns the stylisation and the mode's passes off (the cards stay).
Its `LIGHT` is the one light. Stored apart from the 3D renderer's look (`ffh.hd2d.config.3`, renamed
when the look changes enough that a saved panel tuning would mislead); the panel offers the 3D
switches that apply (not the paint filter, ink or grid) and the mode's rows.

## 12. Removing it

Delete `assets/app/r3d/hd2d/` (with its `art/`), `assets/test/hd2d.test.mjs`,
`r3d-lab/compile.html`, `tools/hd2d/`, this file and `docs/art/hd2d-reference.png`, and the
`r3d/hd2d/art` folder from `build.rs`; remove the `app/r3d/hd2d/*` lines of `ASSETS` in
`src/transport/http.rs`, `EXPERIMENTAL_HD2D` and its option in `look.js`/`tools.js`, the `"hd2d"`
branches in `main.js` and `map.js`, `scene=hd2d`/`renderer=hd2d` in the lab, the `hd2d` shots in
`r3d-compare.mjs` and `--renderer` in `r3d-perf.mjs`. The hooks left in the 3D renderer's files are
its defaults and do no harm.

## 13. Limitations and next steps

* **Windows' default browser backend** cannot compile the 3D renderer's terrain shader (§8): the
  first fix the mode needs is the 3D renderer's, a terrain shader FXC can compile (fewer unrolled
  loops), or the client detecting it and saying so.
* **Picking** is the 3D renderer's (the ground under the pointer): clicking a tall figure's head
  can pick the plot behind it; the cards' boxes could be picked first, as the isometric map does.
* **The overlay stage** costs about 3 ms at 1080p against 0.3 in the 3D renderer (§8); not yet
  profiled.
* **Shafts** are partly painted (a screen-space fan turned by the camera's position); the physical
  haze through the cloud gaps barely shows from above.
* **The region moods** come from a table of civilizations and alignments; a game event (Armageddon,
  a spreading hell) changes them only through the terrain and the owners.
* **Figures stand still** (no idle frames) and are mirrored when facing west, so their painted
  light then comes from the upper right.
* **Cities' garrisons** show one figure (the banner's badge carries the count), as the isometric map
  hides them.
* Next, by value: new art per §10 (ground textures first); the first frame's stall on Direct3D 11
  (§8: cheaper terrain, water and prop shaders); picking by card; idle frames from `anim-sprint/`;
  a sky or haze bank beyond the map's north edge.

## 14. Provenance

The units, buildings and resources are the isometric renderer's sprites (docs/ISO.md: drawn by an
image model and brought to one contract by code in `tools/m7/`), as are the cities of a family
without the town kit. The ground layers and prop
cards in `assets/app/r3d/hd2d/art/` were drawn by FLUX 3 Image Klein (Black Forest Labs) running
locally in ComfyUI, and post-processed by `tools/hd2d/art.py` (prompts in §5b's spirit: 16-bit
pixel art, top-down for ground, a single object lit from the upper left on white for props). The grass, reeds, pebbles, shore and
standing stones, fences and haystacks are drawn by code (`tools/hd2d/draw.py`). `tools/hd2d/gen.py`
asks OpenAI's `gpt-image-2.5-sunburst` (snapshot 2026-09-08) for sheets of props on a transparent
background (cut apart by `art.py split`, then carded by `prop`). **The town kit**
(`art/town_*.png`, §5c) was drawn by `gpt-image-2.5-sunburst-2026-09-08` through `gen.py` (prompts
in `tools/hd2d/towns.json`, usage in `tools/hd2d/usage.jsonl`) and carded by `tools/hd2d/towns.py`;
under OpenAI's terms its outputs belong to the user.
**Licence:** FLUX 3 Image Klein is under the FLUX Kommunity License — non-commercial use, with a
commercial-output exception for "Qualifying Users" under its terms; check
https://huggingface.co/black-forest-labs/flux-3-image-klein-comfy/blob/main/LICENSE.md before
shipping these images in a game that is sold. The banners, the cracks, the beams and every pattern
are drawn by code at run time; the reference image is the owner's.
