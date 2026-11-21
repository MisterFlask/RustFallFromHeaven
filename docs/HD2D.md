# HD2D: pixel-art sprites in the 3D renderer's lit world

**Status: the default map renderer** (the owner's decision, 2026-10-31), with the isometric 2.5D map
(docs/ISO.md) as its fallback where it cannot run, beside the flat, sprite-less debug renderer (the
2D pixel-art map is retired too). It is built on the 3D renderer (docs/R3D.md), whose own mode,
everything modelled, is retired: no longer offered to players, kept in the r3d lab for the layers
the two share. It draws the game in the manner of Octopath Traveler and Triangle Strategy: the
isometric renderer's pixel-art sprites stand as flat cards in the 3D renderer's lit world, seen
through a perspective camera tilted toward the horizon, with glow, a mood for each
region, glowing borders and a tilt-shift blur. The look target is the owner's reference image,
[docs/art/hd2d-reference.png](art/hd2d-reference.png). It reads the game state only: no rule, save
or protocol changed for it.

Phase 1 answers one question: **do today's sprites hold up in a lit 3D world?** No new art was made
and no image model was used. The answer is in §2; the gaps that new art would close are in §10.

Contents: 1 running it · 2 the answer · 3 architecture · 4 what is reused · 5 the sprites ·
5b the mode's own art · 5c towns from a kit of cards · 5d towns that stand out · 6 light, moods and the post passes · 7 the lab and tools · 8 on a real GPU · 9 cities and the camera ·
10 the gaps to the reference · 11 configuration · 12 removing it · 13 limitations and next steps ·
14 provenance.

---

## 1. Running it

```
cargo build --release
./target/release/ffh-play serve            # open http://127.0.0.1:8642/ (HD-2D is the default)
```

* The plain URL shows it, as does `?renderer=hd2d` (or `?renderer=3d`, the retired mode's address)
  for one page, or **Tools → Options → "HD-2D"** (remembered by the browser like the other renderer
  choices). Choosing another renderer there switches away from it.
* **Its fallback is the isometric map** (main.js `useHd2d`): without WebGL2, with a software one,
  when its shaders do not compile in time, or when its WebGL context is lost, the client says so in
  a note and draws the isometric map; the remembered choice stays HD-2D, so the next page tries
  again.
* The **HD-2D** button at the map's top-left opens the tuning panel (the 3D renderer's panel with
  the mode's own rows, §11) with the frame time and per-pass GPU times.
* `?r3d={"tiltShift":0}` (any key of §11) or `?r3d=plain` override the look for one page.
* It needs WebGL2 with GPU acceleration, as the 3D renderer does (docs/R3D.md §1, the same software
  guard and `?force3d=1`). On Windows, Chrome's default ANGLE backend (Direct3D 11) compiles each
  of the scene's programs at its first draw, the browser drawing nothing meanwhile: the lab's HD-2D
  scene starts in about 25 s on an RTX 4060 laptop (38-41 s while each was compiled twice;
  docs/R3D.md §11.1); with `--use-angle=vulkan` the mode starts in about a second (§8).
* The mode's module (`r3d/hd2d/index.js`) is loaded on demand; a page on another renderer never
  loads it, nor does the 3D renderer alone (the r3d lab's own mode).

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

**The sixth batch** made those towns stand out at the gameplay zoom, where they had read less than
the old bright cards (§5d): four switches -- the towns larger, lit and outlined, on a clearing in
their own fields, flying their owner's banners over the roofs -- compared alone in
[hd2d/towns-stand-out.jpg](hd2d/towns-stand-out.jpg) and on by default in a moderate mix.

**The terrain pass** (2026-11-16) took up the two things the look target's land has and the
mode's did not (the owner's direction: the land should be beautiful; no unit or resource art
changed). **The land is a physical object** (§5f): it stands over its water as a slab with a sheer
rock face at every shore, and the hills are crowns standing over rock scarps on the side they turn
to the camera, neighbours joined into plateaus -- faces the terrain mesh now cuts, drawn from
strips of pixel-art rock under a lip of overhanging turf, with steps where a road climbs one. **The ground is drawn** (§5g): each ground a
tile of pixel art in the cards' hand, laid at the cards' density with its texels sharp and in its
own colours, the borders between grounds cut hard and wandering off the grid, with broad patches
a tone darker or lighter; roads, fields, worn earth and river banks in the same pixels.
[hd2d/terrain-pass.jpg](hd2d/terrain-pass.jpg) shows it before and after (on the owner's GPU).

| | |
|---|---|
| the look target and the lab's composition after it, side by side | [hd2d/lab-vs-reference.jpg](hd2d/lab-vs-reference.jpg) |
| the mode's own art: the seven ground layers and the 25 prop cards | [hd2d/art-sheet.png](hd2d/art-sheet.png) |
| the town kit (batch 5): the eleven families' pieces and the 22 landmarks | [hd2d/town-kit.png](hd2d/town-kit.png) |
| cities before and after the town kit: the lab's two views, the real game, close-ups | [hd2d/towns.jpg](hd2d/towns.jpg) |
| the four ways of making towns stand out, each alone and the default (batch 6) | [hd2d/towns-stand-out.jpg](hd2d/towns-stand-out.jpg) |
| the terrain pass, before and after: the land in steps, the ground as pixel art | [hd2d/terrain-pass.jpg](hd2d/terrain-pass.jpg) |
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
    world.js     + the `cliff` option: the ground in steps (§5f)
    terrain.js   + MapTextures.setMood: the per-plot mood channel; buildCliffChunk: the stepped
                   ground's mesh, its faces cut (§5f); the shore field in the height texture
    shaders.js   + OVERLAY_FS uBorderGlow (0 for the 3D renderer: its ink border is unchanged);
                   TERRAIN_FS CLIFF_FACE (the faces, §5f) and PIXEL_GROUND (the ground as art, §5g)
    materials.js + extra layers after its own for a mode's art
    hud.js       + the resource chips ask the map (`resourceChips`)
    config.js    loadConfig/saveConfig/withPreset take the store and presets
    hd2d/
      index.js    MapHd2d: the 3D renderer's Map3D with the hooks set (the module map.js imports)
      config.js   the HD-2D preset, its light, its panel rows (stored apart: ffh.hd2d.config.4)
      dress.js    which card stands where: unit groups and banners, cities, buildings, resources,
                  light beams, the Sea layer's buoys (docs/R3D.md §5.3); the claims
      mood.js     the per-plot mood channel and its particles
      sprites.js  the sprite atlas (a texture array) and the card instance buffers
      passes.js   the pipeline extension
      shaders.js  the cards, their shadows and ghosts, the beams, the post passes
      art/        the mode's own art: gnd_*.png (the ground's tiles and the faces' strips, drawn
                  by tools/hd2d/ground.py and cliffs.py), the prop cards, index.js (written by
                  tools/hd2d/art.py)
r3d-lab/          scene=hd2d, renderer=hd2d, compile.html (§7)
tools/            r3d-lab.mjs --capture, r3d-perf.mjs, r3d-compare.mjs hd2d shots,
                  hd2d/art.py (the art's post-processing), hd2d/ground.py and cliffs.py (the
                  ground drawn by code), trees.py, rocks.py, draw.py (cards drawn by code)
```

The mode is the 3D renderer with a handful of hook methods overridden (`MapHd2d extends Map3D`)
and an extension object on its pipeline; with the hooks at their defaults and no extension, the 3D
renderer runs exactly as before. Data flow per change is the 3D renderer's (docs/R3D.md §3): a
static redress also rebuilds the static cards (cities, buildings, resources, beams) and the mood
channel; a view change rebuilds the unit cards; per frame only the walking figures' instances and
the uniforms change.

## 4. What is reused

Everything but the figures, the settlements, the standing improvements and the resource markers:
the ground and its relief (`world.js`, `terrain.js`; in steps and drawn as pixel art since
2026-11-16, §5f, §5g), water and rivers, fields
and flat improvements, roads and bridges, the decal and light layers, picking, the camera and its
zoom steps, the shadow, SSAO and glow passes, the compose/finish grade, FXAA, the overlays (with a
glowing border, §6), particles, the HUD (city plates, stack badges, far-zoom counters) and the
lab and tools. The camera is the 3D one with the HD-2D preset's pitch and lens (§9).

## 5. The sprites

Mana has 18 physical mineral-deposit PNGs and 17 matching connected-shrine PNGs, shared with
the isometric fallback (`app/mana-art.js` selects their indexed art). Each sphere has a distinct
material and silhouette: Air's windswept white quartz, Earth's cubic ochre geodes, Fire's split
basalt, Nature's root-bound emerald, Mind's amber spiral and Water's slate spring, for example.
Connected shrines retain the mineral formation inside a carved stone ring with steps and short
standing stones. The cards use the existing art pipeline's palette, hard alpha, upper-left light
and shaded rim; there are no floating pictograms. School-coloured light stays close to the object
with restrained bloom. The flat renderer keeps the sphere colours and two-letter labels.
`scene=mana&renderer=hd2d` in the lab compares deposits beside connected nodes.

* **Atlas** (`sprites.js`): the isometric renderer's PNGs (`assets/app/iso/*.png`, indexed by
  `iso/sprites.js`) loaded on first use into a `TEXTURE_2D_ARRAY` of 256 × 256 layers, one sprite a
  layer at its top-left (the 2x2 city, 384 wide, at two thirds), premultiplied (so the mipmaps
  average colour and coverage together: no dark fringe) and mipmapped; the array doubles when full.
  Banners are drawn by code into the same array (the owner's colour stays a runtime choice).
* **Idle figures**: a unit sprite with an anim-sprint idle (`sprites.js` `IDLE`: 129 of them, the
  generic adept, axeman, champion, horseman, phalanx, settler, swordsman, warrior and worker, the
  goblin, lizardman, ogre, paladin and skeleton, the heroes Donal, Guybrush, Hyborem, Losha and
  Maros, and all 110 of the civilizations' own variants: every warrior (19, the barbarians'
  included), axeman (11), worker (21), settler (21), adept (18) and horseman (17), the Ljosalfar
  archer, the Grigori medic and the Sheaim mage) is stored as its idle strip
  (`assets/app/anim/<unit>_idle.png`: four 160 × 128 frames, the sprite's own pixels bent by a
  mesh warp, frame 0 the sprite itself; anim-sprint/ANIMATING_UNITS.md), two frames a layer, one
  above the other (the frames' empty rows round the seam keep the mipmaps apart). Its card is the
  frame's, the sprite in it standing where the sprite's own card would; the vertex shader picks the
  frame, 250 ms each from the figure's own phase, so a group does not breathe in step. While the
  map does not animate (`animate` off, or reduced motion) it shows frame 0. The horsemen's idles
  come from the mounted rig of the unmerged `anim-all` branch (its `lib/quad.py`: the horse's head,
  neck and back rise a pixel, the tail flicks, the rider breathes, the hooves stay planted); the
  strips are shipped, not the rig. A test checks that every strip's frame 0 is its sprite, pixel
  for pixel, so a sprite exported again needs its strip made again.
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
  texels (it melts into the 3D land instead of ending in the isometric diamond's edge). Its foot
  stands on the ground at the plot's front edge on level land, stepped back off a river.
  On hills (`hillWorkFoot`, 2026-11-21) the foot goes where the ground under the card's whole
  painted diamond is flattest -- the diamond an upright card shows as ground reaching back from its
  foot, as deep as the card is wide -- on one level (a crown or the shelf, never across a scarp),
  off the water and clear of river channels, and the card stands at that ground's mean height. (Until
  then `hilltopFoot` put the foot on the summit at its height: the diamond lay over the land falling
  away behind and beside the summit, and the work floated over its hill, against the sky on a hill
  at the back of the view.) Placement alone could not hide it: a hill's crown is smaller than a
  mine's diamond, and the close zooms' lower camera reads an upright card's painted ground as
  reaching further back. So a hill work drawn from an isometric sprite is **draped**
  (`drapeGrid`, hd2d/sprites.js `buildDraped`, `SPRITE_VS` flag 64): its card below the diamond's
  corners is a grid on the land (7 columns, 5 rows, each vertex at `heightAt` there, as the
  terrain's mesh), and above them it stands from the corners' row. The painted ground follows the
  dome; seen from the camera's pitch rather than the art's 30°, it foreshortens as the ground does.
  The HD-2D sites' own art has no painted ground and stands as before. Checked at 56, 96, 128, 168
  and 220 px a plot. Tribal villages are
  five separate smaller hut cards about a clearing; each hut samples its own ground contact.
  Flat works (farms, pastures, plantations, wineries, quarries) stay the 3D renderer's fields.
  A resource is its family's prop at the plot's front-left; under a standing improvement it lies
  at the card's front-left and is drawn before it. Connected mana shrines and the six world sites
  that produce mana contain their deposit visually (`mana-art.js` `improvementContainsMana`),
  so neither a separate resource card nor its 3D model or particles is drawn beneath them.
  `scene=sites&renderer=hd2d` compares villages on flat land and a jungle hill, hilltop works,
  mountains and mana-producing landmarks. Resources reserve a small clearing around their opaque
  sprite bounds, across neighbouring plots too: trees' crowns, bushes, rocks and shore props keep
  off their silhouettes over the camera's 30–42° pitch range. The check uses the resource's actual
  position beside an improvement, accounts for hills and map wrapping, and leaves the rest of the
  dressing unchanged. Resource sprites take their own plot's fog visibility. A town's card (batch 4: keep mask 0.95)
  takes about half the moods' and the land palette's grade, so it keeps its own colours, and its
  warm lights (windows, fires, braziers: orange to yellow, not its red roofs or blue domes) glow. Mana and gems glow (their bright, saturated pixels
  feed the bloom). What a card stands for is **claimed** from the 3D dressing (no house, wall,
  structure or marker is modelled under it); trees keep out of a town's reach as before.
* Scale (`dress.js` `PX`): a unit's 96 × 120 canvas is 0.46 × 0.58 of a plot, a 192-wide
  settlement 1.7 plots (as the reference's towns, which span about two tiles; the prop cards keep
  clear of its footprint; the kit's towns, §5c, stand at the props' scale instead), other buildings 0.73, resources' canvases 0.81 (their props about 0.3),
  banners 0.8 tall, a lone prop 0.0031 a plot an art pixel (a 150 px oak about 0.47 of a plot; in
  a wood 0.58 of that, about 0.27; the woods' oaks and pines drawn by code, §5b, 45 px cards
  about a third of a plot tall), the props
  drawn by code 2.6 times that (`FINE`: about one art pixel a screen pixel at 96 px a plot).

## 5b. The mode's own art: pixel ground and prop cards

The first review found the gap was the medium around the sprites (§10). The second batch closes
its two largest parts with art of the mode's own, drawn by a local image model (FLUX 3 Image Klein
in ComfyUI on the owner's PC) and brought to one contract by `tools/hd2d/art.py`, as `tools/m7`
does for the isometric sprites. No unit art changed.

* **Pixel-art ground**, as it was until the terrain pass of 2026-11-16 (§5g: the tiles are drawn
  by code now, laid at the cards' density and shown in their own colours)
  (`hd2d/art/gnd_*.png`): grass, dry straw, packed dirt, sand, tundra moss
  and lichen, snow and broken rock, drawn top-down, then pixelated to 128 art pixels a tile (two
  texels each: about the sprites' density on screen), their palettes cut to 40 colours, made
  seamless by a dithered cross-fade (each edge takes, more often the nearer it is, what lies one
  tile further in the source), their height taken from their value into alpha. The mode loads them
  into **its scene's** material layers (`materials.js` GRASS, STRAW, DIRT, SAND, LICHEN, SNOW,
  ROCK; the crop, reed and noise layers stay), with their mean colours: the terrain shader tints a
  layer by its mean (`matTint`, `matShade`), so the land keeps its painted design — which ground
  lies where, the meadow's golden patches, the plot centres — and takes the pixel art's texture.
  No shader changed; the 3D renderer's own layers are untouched.
* **The stand-in as the ground** (the owner's call, 2026-11-03). The ground is
  the terrain shader compiled with the stand-in's defines (hd2d/config.js `TERRAIN_STANDIN`,
  docs/R3D.md §11.1 "Two stages"): the pixel-art materials blended across the plots under each
  ground's mean tones, lit, shadowed and fogged as before, without the painting's value noises,
  the materials' relief bump or their second scale. In play the full painting did not show: the
  turn-200 `stage corrupt` save at (44, 14), the two grounds from one camera, had 16%, 9% and 5% of
  the frame's pixels off by more than 8 of 255 at 168, 96 and 40 px a plot, nearly all bare ground,
  and the one change seen close up was a peak's rock, softer without its relief. Where the
  painting carries a design the stand-in loses it: on the lab's showcase the tone (9 × 9 pixels
  averaged) moves by more than 8 on 29% of the pixels (20% on `scene=hd2d`) -- the hell grounds'
  own marks (the mood pass's glowing cracks over them stay), the marsh's pools, the dunes, ice
  cracks, the meadow's golden patches, and the warped edges between grounds, which turn to soft
  blobs. What it gains: half the program (1358 instructions against 2742), so no second stage at
  the start (the full terrain's compile, about 1.9 s on Direct3D, is gone), and a cheaper frame
  (SwiftShader, the lab's `scene=hd2d`, one run: 2856 to 2432 ms; an integrated GPU's gain is
  still to measure). (The switch back to the full painting with the two stages, `terrainStandin`,
  was removed 2026-11-10.) Since 2026-11-16 the grounds themselves are §5g's tiles; the stand-in's
  defines still shape what is left of the 3D renderer's painting in the mode's program.
* **Prop cards** (`hd2d/art/*.png`, indexed by `hd2d/art/index.js`): single trees (jungle,
  ancient elven giants, dead; the broadleaf trees, pines and snowy pines were the image model's
  too until 2026-11-14 and are drawn by code now, below), bushes, wildflowers, violet
  crystals and lava rocks, drawn as single objects on white, cut out, scaled to a card a set
  number of art pixels tall, their palette cut and a dark outline added; the trees and bushes with
  their middle values deepened (a gamma of 1.25-1.35: the model paints foliage a bright lime, the
  look target's woods are deep greens under bright tips). **Woods** stand as a crowd of small
  trees (their number and size: "The woods' trees drawn by code", below), the back row left out and the middle kept low where something stands on the plot
  behind (species by feature and ground: oaks and pines,
  pines and snowy pines in the cold, jungle trees, an ancient wood's one or two giants among oaks,
  dead trees in burnt woods and hell), thinned where a stack or an improvement holds the plot; **open
  ground** carries only what says what it is (since 2026-11-01, below): grass tufts, flowers,
  reeds, pebbles, crystals on corrupted land and lava rocks on hell
  ground. They keep off rivers, roads, a town's footprint and
  the spot a stack stands on; they cast their outline into the sun's shadow map and a contact
  shadow, so a wood is dark beneath. What they replace is **claimed** from the 3D dressing
  (`claims.trees`, `.flora`, `.rocks`): no modelled tree, plant or rock stands in the mode.
* **Small props drawn by code** (`tools/hd2d/draw.py`, batch 3): grass tufts (green and dry),
  reeds, standing stones and fences -- each a few muted tones a material shaded from the upper left, with
  the cards' dark outline, written as art cards (`art.py index` lists them). Small enough that code
  draws them well; anything with a design (a hut, a ruin) is left to the image model.
* **Density** (batch 3): woods were five by five trees a plot (four by four before) at 0.58 of a
  lone prop's scale, with grass at their sunny edge; open ground's props gather round one or two
  clumps a plot; farms, pastures, cottages and the land round a town add fences and haystacks.
* **The woods' trees drawn by code** (`tools/hd2d/trees.py`, 2026-11-14; they replace the image
  model's oaks, pines and snowy pines: 150-pixel paintings shown at a fifth of their size read as
  a lumpy lime mass). The recipe is the owner's isometric tile sketch: a crown is a cluster of
  spheres shaded from the upper left into five hard tones of one ramp, a crease of shade where a
  nearer lump overlaps the one behind, leaves as two-pixel dashes, a one-pixel outline inside the
  silhouette; a conifer is tiers of lumps. Five broadleaf cards (round, wide and tall crowns, three
  green ramps) and five conifers (two snowy), about 45 art pixels tall, shown about a third of a
  plot tall (`DRAWN` in `dress.js`: near one art pixel a screen pixel at the gameplay zoom).
  A wood is four rows of four, every other row shifted half a step, reaching a little over the
  plot's edge, thinned by a noise field of the map (not of the plot) into clumps and glades that
  run across plots: about twelve trees a plot. Flat woods are three broadleaf to one conifer,
  hill woods one to one. Jungle, ancient and dead trees are still the image model's cards.
* **Stones and haystacks drawn by code** (`tools/hd2d/rocks.py`, 2026-11-14, the trees' recipe cut
  for stone; they replace the image model's two boulders, 48-pixel paintings shown at a third of
  their size, and draw.py's shore stones, pebbles and haystacks, noisy lumps the owner reported:
  the shore stones ran together into a grey smear along every coast). A card is a few separate
  stones, each an angular silhouette flat at its foot and broken into planar facets, a facet one
  of five hard tones by the way it faces the light (a lit top, a middle face, a shaded side), a
  highlight on a lit facet's edge, a crease where a nearer stone overlaps, a few cracks, the
  outline inside the silhouette. Four boulders (`rock_*`, about 20 art pixels tall, one warm, one
  mossy), six shore cards (`shore_*`: two to four wet stones in a loose row under caps of moss)
  and three of pebbles; three haystacks (`hay_*`: the trees' lumps in straw, a point over a broad
  skirt, the straw as short strokes down the stack). All stand at the small props' scale (`FINE`).
* **Mountains are models** (2026-11-15): a peak is the 3D renderer's `feat.mountain` (docs/R3D.md
  "Asset recipes"), a faceted massif of ridges and gullies under snow, lit and shadowed by the sun
  like any model; the peaks' boulder and pebble cards are gone. (A card drawn by code as the trees
  are was tried first and dropped the same day: the owner preferred a more detailed model.)
  The mood pass's land palette (`MOOD_FS`) no longer lifts the cards' middle and dark values (the
  ground's only): their darks are painted, and lifted, a wood read flat and pale.
* **Open ground says what it is** (the owner's call, 2026-11-01: unimproved grassland and plains
  were hard to tell from woods, hills and each other). `clutterOf` keeps only the props that name
  the ground: small green tufts and flowers on grassland, small dry tufts on plains (`TUFT`, 0.7 of
  the other small props' scale: at full size they read as dark stumps), reeds on marsh, moss tufts
  and pebbles on tundra, pebbles on desert and snow, and the regions' own
  things (lava rocks, bones and vents in hell, crystals, obelisks and altars on corrupted land,
  lanterns in the elven lands; since 2026-11-21 ice shards, frosted shrubs and cairns on frost,
  rune stones, glyph rings, obelisks, floating crystals and crystal pillars on the arcane, shrines,
  braziers, angel statues and columns on holy land, soot, slag, coal, ash mounds and vents on ash:
  `clutterOf`, a mood of at least 0.5), and the civilizations' own (`CIV_PROPS`, by the plot's
  owner): totems and skull poles (the Clan of Embers, the Doviello), gallows and red lanterns (the
  Calabim), masks on poles and carnival stalls (the Balseraphs), mine entrances on hills (the
  Khazad), shipwrecks and anchors on the coast (the Lanun). The lone trees, bushes, boulders, ruins, shrines and standing
  stones that had stood on every open plot are gone: they made open land read as a thin wood. The
  Hills now read through the terrain's turf-capped scarps (docs/R3D.md §4), with a grassy crown,
  short soil-and-stone exposures and broad grassy shoulders; `clutterOf` adds no extra rock or
  pebble cards for hills. Hill mines are 0.82 of the usual improvement-card scale and stand where
  their painted apron lies on the hill, as other standing improvements do (`hillWorkFoot`, §5),
  keeping it off river banks.
  The meadow's ground (shaders.js `G_GRASS`) no longer mixes the two either: a grassland stays green
  and a plains golden, their patches interleaving only where they meet (they had been a tenth
  and three quarters golden).
* **Designed props drawn by code** (batch 4, until the image model draws better ones): thatched and
  stone huts (a window lit from within), a well, ruins, gardens; the corrupted land's obelisks (violet
  runes) and altars, hell's bones and smouldering vents, the elven lands' lanterns -- each region's
  own, by the moods (`clutterOf`); the ruins and shrines that once stood anywhere are gone
  (2026-11-01, above). What carries a light (`GLOWS`: runes,
    lamps, embers, hut windows) feeds the bloom.
* **The moods' and civilizations' props** (2026-11-21): OpenAI's image model (`tools/hd2d/gen.py`,
  sheets `frost_arcane`, `holy_ash`, `folk`; two rolls each, both kept as variants `_1`, `_2`), cut
  by connected parts assigned to their grid cell (the sheets' objects cross the cells' lines, and
  `art.py split` miscounts a floating crystal's shards), the shrine's cross cut off at the roof's
  ridge (no Earth faith in Erebus), carded by `art.py prop` at their height in art pixels and stood
  at `PAINTED` scale (1: an art pixel 0.0031 of a plot). Lab: `scene=moods&renderer=hd2d`
  (`&civs=A,B,C,D&religions=R1,,R3,`).
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
* **Raised banks and waterfalls** (batch 4; the bank gave way to the land's face on 2026-11-16,
  §5f, the white water stays): the mode's world raised dry land 0.04 over its water
  (world.js `bank`, `MapHd2d.worldOpts`; the 3D renderer's is 0), all but a narrow strip over the
  waterline, so the shore climbs in a short steep step the terrain shader draws as rock, a boulder of
  the bank now and then among the shore stones. (The terrain mesh, six vertices a plot, cannot draw a
  true cliff face.) Where a river's surface drops steeply (off the hills, over the bank into the
  sea) the water itself turns white: streaks running fast down the slope, churned on in the pool at
  its foot, from any side (docs/R3D.md §5.2, "Rivers"). (Until 2026-11-03 a card of pixel-art white
  water stood across each drop facing the camera, `dressFalls`: an opaque rectangle with straight
  edges over painted water, it read as a sticker, and the owner reported it with the cracks below
  as artifacts in the river. Behind the cards, the river's depth-found rock collars and wakes took
  the sloping bed for something in the stream and drew a net of white cracks over every drop;
  they are now off where a river runs steeply.)
* **Shores** (batch 3, `dressShores`): the look target's land ends at its water in dark
  stones. Along every coast and lake shore a 14 × 14 sampling of the plot (and a quarter plot past
  its edges, as the warped coast wanders off the grid) finds the shallows a little way off the
  land's face (0.02 to 0.075 of a plot out, by the shore field over its slope: §5f) and stands a
  shore stone there, now and then a boulder, in a loose row (until 2026-11-16 a rim of stones and
  pebbles where the ground fell through the waterline on a beach); along rivers, just outside the water's edge
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
  * **Variants only where needed.** Each variant takes a layer of the sprite array, so only the 78
    pieces that carry the band get one. `art.py index` marks them `band` in `art/index.js`. The rest
    (most walls, the low houses) are shared by every owner.
  * **Recoloured before upload.** The recolour is done on the CPU before the mipmaps are built. A
    recolour in the shader would see the band blended with its neighbours at far zooms and miss it.
  * **A capped array.** The array still doubles as it fills, but only up to the GPU's
    `MAX_ARRAY_TEXTURE_LAYERS`. Past that a sprite is not drawn, and the console says so once.
* **The layout** (`dress.js` `dressTown`) is deterministic from the city's id.
  * **Reach.** The town's reach is the 3D renderer's (`CATALOG.cityReach`), so the HUD's plate,
    which stands just in front of that reach, stays in front of the town: an ellipse `reach + 0.06`
    across, `0.78 reach` deep, centred 0.05 behind the plot's centre. A village keeps to about its
    plot; a great walled city spills to about 0.9.
  * **The landmark.** It stands at the back centre. If the back is wet (a river along the plot's
    back edge), it steps toward the front until it reaches dry ground. It grows with the town,
    within the props' own spread of scales: 0.78 for a village, 1 for a town, 1.12 for a city (9+),
    0.06 more for a capital, at most 1.18.
  * **Towers and houses.** A town (size 4+) has one family tower, a city two, at the back corners.
    Houses number 4 + 1.6 a size point, 2 more for a capital, at most 26. They are scattered at
    random in the ellipse, more often the taller ones toward the back, so the roofs climb toward the
    landmark. The heights are well mixed, so the two houses that carry the owner's colour are not
    all at the back; right before the landmark stand only the lowest, so its lower storeys show.
    Houses and towers are mirrored at random and vary from 0.9 to 1.06 in scale, as the land's props
    do: four variants a family do not repeat in rows.
  * **Walls.** A walled city (`city.walls`) gets a ring just outside the houses: the gate at the
    front middle, corner towers at the sides and front corners (not the back corners, where the
    family's towers stand), and a segment every 0.24 or so of the ring between them. On the sides,
    where the ring runs into the view, the segments stand one behind another.
  * **Overlap.** Two pieces at about the same depth never overlap. One well in front of another may
    hide its lower part: roofs behind roofs, a dense town.
  * **Where pieces may stand.** They keep off water and rivers, a lane along each road from the
    plot, and the garrison's and banner's spots. They also keep off a neighbouring plot that is
    unrevealed, a peak, or holds a city, an improvement or a stack.
  * **Each piece's card.** Each piece is its own card with the town flag: half graded, its warm
    lights glowing. It casts its outline into the sun's map and a contact shadow, and is fogged as
    the city's plot. It is tinted by the light where its walls and roof are, at about half its
    height (`SPRITE_VS`). Taken at its foot, a town's taller pieces behind would shade a whole house
    by its doorstep.
* **Round the town.** The props keep out of its reach, and out of a lane before its middle: the
  way to the gate, where the plate stands. The houses spilling onto the plots round it (`townRing`)
  are the town's own houses (the low ones twice as often), in its owner's colour. Code-drawn huts
  remain only round a town whose family has no kit.
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
  environment's proxy (HTTP 502) after about a minute.

  **The review round** (one visual and one code critic) found:
  * the town cores dark, each house tinted by the sun at its foot in its neighbours' shadows;
  * the owner's colour missing from the town fronts and the houses round it;
  * the landmark too large in a village and crowded in a city by eight identical round towers;
  * the four houses of a family repeating in rows;
  * the landmark able to stand in a river along the plot's back edge;
  * the dressing half as slow again (the kit worked out per prop);
  * a layer of the sprite array per owner for every piece.

  Each was fixed as above. Every sheet was looked at and accepted
  (prompts in `towns.json`). The cost was $1.19 for 19 calls at list prices, recorded in
  `tools/hd2d/usage.jsonl`. Eight more calls ended in an HTTP 502 before streaming; whether those
  were billed is unknown.

## 5d. Towns that stand out (batch 6)

The owner liked the kit's towns, but at the gameplay zoom they stood out less than the old bright
isometric cards. Four ways to make them stand out were tried, each a switch in the panel
(`config.js`; 0 or 1 turns it off, for one page `?r3d={"townBanners":0}`):

* **Size** (`townScale`, 1 to 1.6). Every piece is larger by this factor, and so is the spread
  between them (`townShape`, `dressTown`), so the town is denser and larger at once. The pieces'
  pixels get coarser by the same factor: at 1.25 an art pixel is 0.0039 of a plot, still 2.3 times
  finer than the old card's. A larger town reaches further onto the plots round it; the HUD's plate
  keeps its place, at the walls' foot in a great walled city.
* **Lit** (`townLight`, 0 to 1). It works in the cards' shader (`SPRITE_FS`) and on the ground:
  * each piece takes the figures' dark one-pixel outline, so roofs read against a wood;
  * it is brighter (up to 24%) and never in deep shade;
  * its windows feed the bloom up to 2.6 times as much, and a little of every pixel, warmed, feeds
    it too, so the whole town glows softly, as the look target's golden town does;
  * the cities' pools in the 3D renderer's light layer (`decals.js` `rasterLight`) grow up to three
    times as bright and 30% wider, through a hook, `Map3D.cityLight`, that is 1 for the 3D
    renderer.
* **Clearing** (`townClearing`, 0 to 1). What stands tall (trees, bushes, rocks) keeps up to 0.6 of
  a plot further from a town; grass, flowers and the fields do not. Garden plots, haystacks and
  fences lie on the plots next to it (`fields`), fewer one further out, so a town in a wood sits in
  its own farmland.
* **Banners** (`townBanners`, 0 to 1). The owner's banner, drawn by code, flies from behind the
  landmark, higher than anything in the town (0.7 to 1.1 of the city banner's height), and a great
  city's (size 9+) towers fly smaller ones. Like all banners they ripple, take the figures' rim and
  keep their colours under the moods.

[hd2d/towns-stand-out.jpg](hd2d/towns-stand-out.jpg) shows each alone (size at 1.35, the others at
1) and the default, on the lab's two views:

* **Size** does the most. Eärlann and Aurelis take their place on the map, and a great walled city
  dominates its land. In the cities scene, where cities stand two plots apart, larger towns run into
  each other; a real game's cities stand further apart.
* **Lit** turns a town into a lighter, glowing block. At full strength it washes the detail out, so
  the default takes half.
* **Clearing** shows where a town stands in a wood or among clutter; in the lab's open scenes it
  adds mostly fields.
* **Banners** mark a town from afar. Several to a town and larger, as first tried, they buried the
  towns and the plates behind them; hence one over the landmark, and pennants only on a great city's
  towers.

**The default** combines them: size 1.25, lit 0.5, clearing 0.5, banners 0.6. The `plain` preset
turns all four off; any of them can be turned off in the panel. A change to any of them redresses
the map (`MapHd2d.setConfig`).

## 5e. Bespoke map sites and resources

The HD-2D map has dedicated art for all 17 unique landmarks and for the lairs,
ruins, special sites and settlement stages that previously shared generic forts,
caves, ruins or mana-node sprites: 45 improvements in total. The Guardian of
Pristin Pass is a stone giant with gargoyles in its shoulder alcoves; Barrows are
grass-covered burial mounds with a clear stone trilithon entrance, separate from
bear dens, lion dens and dungeons. Another 27 resource cards distinguish the
previously shared crops, animals, trade goods and rare resources. Established
works retain their existing artwork. Mana has another 35 sphere-specific cards (§5),
generated on 2026-11-15; Ice remains a resource of Letum Frigus rather than a buildable shrine.

`tools/hd2d/landmarks.json`, `resources.json` and `mana.json` retain each sheet's exact image
generation prompt, reading order, type mapping and art-pixel height. These sheets
were generated with the image_gen tool on 2026-11-14. Import a sheet with
`python tools/hd2d/landmarks.py SHEET.png NAME`, then run `art.py index`. The importer
cuts nine grid cells (keeping disconnected parts such as the Seven Pines together),
removes detached specks, and uses the existing prop pipeline's limited palette,
hard alpha and one-pixel rim. Every card fits the 256-pixel atlas layer directly;
Barrows uses a 59 x 40 card at twice the pixel scale, preserving its world
footprint while keeping the doorway readable at play zoom; it has no baked blur.

The generated art index exports `IMPROVEMENTS` and `RESOURCES`. Static dressing
prefers these exact type mappings, and unit placement uses the same standing-site
lookup. Worker build buttons use the matching art while the HD-2D map is active.
The coverage test checks all unique landmarks, each new type's actual drawn card,
PNG dimensions and atlas fit, alongside the existing visibility checks.

## 5f. The land in steps: the slab and the hills' scarps (2026-11-16)

The look target's land is a thing: it stands over its water as a slab with rock at its edge, and
its high ground is a plateau over a rock face. Until this pass the mode's land met its water in a
bank 0.04 of a plot high, painted as rock, and its hills were smooth domes with a blurred rock
texture on their steep side: the terrain mesh, six vertices a plot, could only ramp. Now the ground
has steps (the owner's direction, 2026-11-16: the land should be beautiful; no unit or resource art
changed):

* **The world** (`world.js`, the mode's `cliff` option: `CLIFF` 0.18 in hd2d/config.js; the 3D
  renderer's world, without it, is unchanged to the bit). `heightAt` keeps the *shore field* -- the
  ground before any step, continuous, zero at the shore -- lifts all dry land by `CLIFF` and drops
  the floor by `CLIFF_FOOT` (0.05), so at the shore the ground steps by 0.23 of a plot: about 14 px
  of face at the gameplay zoom, 26 at 168. The hills (`domes`) have a *scarp field*, positive on
  their crowns: on the side a hill turns to the south, toward the camera (each hill its own way, as
  before), the crown stands sheer over its shelf, up to `SCARP` (0.2 of a plot; 0.7 to 1 of it,
  each hill its own), tapering out over about a sixth of a plot of rim to nothing round the back,
  where the crown rises in the broad ramp it had. Hills side by side stand on one crown under one
  scarp: the crowns are where the hills' pulls together (`crownPull`) pass a lone hill's at its
  rim, with a pull along the line between two neighbouring hills' middles (diagonal ones too:
  `LINK_R`), so a group never parts into terraces with a strip of the plain between them; the
  step's height and the way it faces are the hills' own, mixed by their pulls. The crown is not
  flat: each hill's top swells out of it (0.08 to 0.13 of a plot, its own height and place, from
  nothing at the crown's rim, so no second step), so a group is one rolling upland with a top for
  each hill and saddles between, and a lone hill a dome over its scarp (2026-11-17; the owner:
  hills that are close together should merge and still read as hills).
  A scarp gives way to a river (over its channel it is the old slope) and to a lake. Toward the ice
  the shore's lift fades instead (no face): the frozen sea is a sheet at the water's level. The
  unknown beside known water lies low, at the water's level, as before; the rest of the unknown is
  lifted with the land (`findLowVoid`), so known land runs level to its frontier and a river there
  does not fall.
* **The mesh** (`terrain.js` `buildCliffChunk`). A grid square the shore or a scarp runs through is
  cut along it: each of its two triangles is clipped by the shore field, then its land by the scarp
  field, into each region's part (the floor, the land, a crown) at that region's height
  (`World.hFloor`, `hLand`, `hCrown`), with an upright quad between two parts. A cut's ends are
  found on the fields themselves (false position, a few steps; more on the far zoom's coarse grid),
  from the same end of a grid edge in the chunks on both of its sides, so neighbouring chunks meet
  without a crack (tested, at both levels of detail: the plan area of a chunk's ground is the
  chunk's, and the vertices on a shared edge are the same from both sides;
  `assets/test/r3d-steps.test.mjs`). A face's normal is level, which is how the shaders know a
  face (steep ground, a volcano's flank, is still ground), and 2 + its rim's height long. A Huge
  map's 56 chunks build in 0.8 s against the plain grid's 0.5. A reveal in the map's edge rows also
  rebuilds the parchment's chunk past the edge (the unknown there turns low beside known water).
* **The faces** (`shaders.js` `CLIFF_FACE`; `hd2d/art/gnd_face.png`, drawn by
  `tools/hd2d/cliffs.py`). A face is drawn from a strip of pixel-art rock laid down from its rim at
  the cards' density (124 art pixels a plot along it and down it; along a wrapping map's width a
  whole number of strips), its texels sharp while they are larger than a pixel: living rock broken
  into blocks of very uneven size, each standing out of the face, level with it or set back -- a
  block that stands out a tone lighter, its upper and left edges in the light, the crevice beside it
  dark; level blocks meet in a change of tone, none ringed in dark (a face of ringed cells read as
  paving, a face of courses as masonry: both were tried) -- lightest under the rim and darker
  toward the foot. The strip's alpha is the *lip*: the turf that hangs over the rim, four to seven
  pixels deep in blades and darker downward, drawn in the ground's own colour there (a meadow's lip
  is green, a prairie's straw, the snow's white); where a scarp tapers out, a face a few pixels
  high is all turf. The rock takes its land's cast (sandstone by a desert, cold grey in the north,
  basalt in hell) and is wet and dark where it stands in the water. Like a card it is tinted, not
  lit: a face turned from the sun is a step darker.
* **Roads climb a face in steps**: where a road's track meets one, treads of packed earth, three
  art pixels each, are cut in the rock -- up a hill's scarp, and down the land's face where a road
  reaches the shore.
* **The water** measures the sea's depth, foam and shore lace by the shore field (the height
  texture's second channel, `SHORE_FIELD`: a face's height does not interpolate; the rivers'
  program keeps the ground itself). The floor at a face's foot is dark stone, not a sandy shelf; a
  beach lies only where the face fades out beside open water. **Shore stones** (`dressShores`)
  stand in the shallows a little way off the face, a loose row with a boulder now and then, their
  feet just over the water's level, no longer a rim on a beach. Rivers still turn white where they
  fall (§5b): off a crown's side, and over the land's face into the sea.
* **What stands on the land** takes its height from the same `heightAt`: a figure, a mine or a
  tree stands on a crown or at its foot, never in the face. What belongs to a land plot stands on
  its land (`onLand`, hd2d/dress.js): a building's foot, a banner, a resource or a group of figures
  whose spot would fall past the rim of the land's face, or on a neighbouring hill's crown, is
  drawn back toward the plot's middle, and no tree or tuft of a land plot stands in the water. The
  borders, the grid and the plots' marks are not drawn on a face (`OVERLAY_FS`).

Not done: a river's mouth is a chute of white water that reads as a waterfall only where it faces
the camera; a peak is still the 3D renderer's model (the owner's choice, §5b); a scarp stands only
on the side a hill turns to the camera; a face takes no cast shadow; what reads the height's
texture or drapes a patch still ramps across a step (a card's contact shadow within a fifth of a
plot of a face, a unit's ring on the land's rim, a sea lane at a port); the far zoom's coarse grid
places a scarp less exactly than the near one.

## 5g. The ground drawn as pixel art (2026-11-16)

Until this pass the ground was the 3D renderer's painting under seven image-model textures (§5b),
laid at twice the cards' density and tinted to each ground's mean tone: at the gameplay zoom a
soft, smooth surface under crisp cards -- two media. Now the ground is art in the cards' hand:

* **The tiles** (`hd2d/art/gnd_*.png`: 256 x 256 art pixels, a texel each; drawn by
  `tools/hd2d/ground.py`, the trees' and the stones' recipe cut for ground -- hard tones of one
  ramp, lit from the upper left, marks a few pixels across). Grass (blades, tufts, flecks of light
  and shade, flowers in drifts), prairie straw combed along the wind, sand under short crests,
  packed dirt with pebbles, pale stony tundra with tufts of moss and frost, wind-carved snow, marsh
  with pools and reeds, rock, the three hell grounds (charred sand with embers, plates of basalt
  with burning seams, clotted red ground with dark pools) and a field's crops (tilled furrows,
  beds, rows of ripe grain, a whole number of art pixels apart). They fill the scene's material
  layers (`MapHd2d` `GROUND_LAYER`: those of the 3D renderer's ground they replace, and four of the
  mode's own after its twelve, `extraMaterials`).
* **Laid as a card is drawn** (`shaders.js` `PIXEL_GROUND`). A tile lies at the small cards'
  density east-west (124 art pixels a plot) and at 0.55 of it north-south: the view foreshortens
  the ground by about the sine of the camera's pitch, so an art pixel comes out square on the
  screen and a blade drawn upright stands upright, as on a card (a tile's first row is its north).
  Its texels are sharp while they are larger than a screen pixel (the cards' filter) and
  mip-filtered when smaller. And the ground's whole painting is worked out at the art pixel's
  centre while magnified, so every edge -- a border between grounds, a road's, a field's -- steps
  with the art's pixels.
* **In the art's own colours.** The tiles are authored in the cards' colour space and shown as the
  cards are: on open, level ground in full sun a tile's colour comes out of the pipeline as
  authored (the finish's brightness and saturation then lift it as they do a card's), and shade,
  cloud, a slope turned from the sun and a town's light tint it. The mood pass's land grade no
  longer lifts the ground's middle values (the ground's keep mask is 0.87, between the painted
  ground's and the cards' 0.9; §6): one palette for the ground and the cards. The ramps keep the
  map about as bright as the owner asked for on 2026-11-14 (grass is shown at about #89b050,
  prairie at #e8c86a, sand at #f8dea8): over six views of the turn-200 game the picture's median
  luminance is 1 to 3% under the map before (the shores' pale beaches are rock now) and its
  saturation 2 to 6% over.
* **Which ground lies where is cut hard**: one ground holds an art pixel. The borders between
  plots' grounds wander off the grid by a noise of the map (up to 0.42 of a plot; a plot's centre
  keeps its own ground) and are broken into lobes a few art pixels across (the utility tile,
  `gnd_pix`). Grassland and prairie are two tiles, cut the same way. Over the water the ground is
  the land's own right to its rim. The unknown's parchment fades in softly, as it did, over the
  nearest known ground.
* **Patches of tone.** A noise of the map at two sizes steps a pixel one tone darker or lighter, in
  each ground's cast (the snow's shade is blue). A tile's alpha pushes that cut, so a patch on
  grass ends in upright teeth, like blades, on straw in lying ones, on sand in an even curve. The
  patches do not repeat with the tile and hide its repeat (2.1 by 3.8 plots).
* **What is laid on the ground**: a wood's floor is the ground's own drawing a deep tone darker (a
  jungle's and an ancient wood's each in their cast); flood plains are the grass's tile in patches
  on the sand; frozen sea is the snow's tile in a pale blue, cut into floes by the utility tile's
  cracks; a road is a light track of the dirt's tile with a darker edge, the grass beside it worn
  dry in patches; a yard is worn to the dirt's tile in two steps; a river's banks are a tone lusher
  and its cut bank wet earth; a field is its crop's tile.
* **Cost.** The frame costs what it did: on the turn-200 `stage corrupt` save at 1280 x 800 under
  SwiftShader 1105, 1158 and 1170 ms a frame before and 1080 and 1116 after at 96 px a plot (the
  scene pass about 405 to 387 ms: the ground's painted branches left its program as the tiles
  came in), and 1007 to 1014 against 1045 and 1067 at 40 (the overlay, which draws the terrain's
  meshes again, 121 to 142 ms). The terrain's triangles go from 166 to 200 thousand at 96 and from
  25 to 37 at 40 (the cuts and the faces). Its program compiles on Direct3D in about 460 ms (525
  before; an RTX 4080 laptop, `r3d-lab/compile.html?only=terrain`). Bringing the whole revealed
  map up to date costs more, once: the height texture with its second channel about 107 to 182
  ms, the dressing about 373 to 481 ms (the stones' search off the faces, the spots kept on
  land). A Huge map fully revealed draws its first frame 4.3 to 4.5 s after the reveal under
  SwiftShader (the limit is 30) and in about a second on the GPU, the Sea layer on or off
  (`tools/r3d-huge.mjs`). A pan is as stable as it was (`r3d-lab --stability` at 40, 96 and 168
  px a plot: docs/R3D.md §11).

Not done: the unexplored's parchment, the snow on a volcano and the sea floor are still the 3D
renderer's painting; a tile's art is one seed's (a second tile a ground, chosen by a noise, would
vary it further); and the water is now the largest surface not in the cards' medium (the review's
note: texel-snapped waves and a pixel foam line at the faces would be the next step).

**The review** (one round: a visual critic on the renders, a code critic on the diff) found, and
the pass fixed before it landed: the polar ice painted as sand and a ragged map edge (the slab's
re-weighting took the ice's ground for water and the unknown's cut ran hard); improvements,
banners, resources and garrisons on a plot with water due south standing at the foot of the face
(their spots lay past the rim: `onLand`); the tiles laid mirrored north-south; steep ground drawn
as a face; the parchment's chunks past the map's edge left stale after a reveal; a hole under the
face where a scarp met the shore; the map 5 to 10% darker (the ramps were lifted, the haze's slab
raised with the land); roads without contrast on grass; hills as separate discs (their crowns
join now); the faces as paving (redrawn).

## 6. Light, moods and the post passes

* **The sun** stands at the upper left of the view (west and a little north, about 45° up), where
  the sprites' painted light comes from; shadows fall toward the lower right. The camera never turns.
  Little fill comes from the sky and the ground, and cast shadows are less floored than the
  painterly look's (`stylize` 0.7): shade goes deep, as the look target's near-black woods and
  hollows under bright accents.
* **Region moods** (`mood.js`): eight per-plot moods in two RGBA textures of the terrain data
  (`MapTextures.setMood`: `mood`, `mood2`), from what the player knows: **hell** (hell terrain;
  flames, volcanoes, souls), **corruption** (the Sheaim; part of the Infernals, Calabim and
  Balseraphs), **lush** (the Ljosalfar, ancient forests), **gloom** (the Svartalfar, Calabim, Sidar,
  Clan of Embers, Doviello); and since 2026-11-21 **frost** (the Illians, the Doviello), **arcane**
  (the Amurites, the Luchuirp), **holy** (the Mercurians, Elohim, Bannor, Malakim) and **ash** (the
  Khazad, the Clan of Embers, the Infernals). A civilization's own entry first (`CIV_MOOD`), else
  its owner's alignment (good: a little holy; evil: a little gloom and corruption). The Kuriotates,
  Hippus, Grigori and Lanun have an entry of nothing: their land keeps its colours and carries only
  their accents (below), so the strongly graded lands stand out. Over that, the owner's **state
  religion** (`RELIGION_MOOD`, screened on at `RELIGION_WEIGHT` 0.65: a mood only adds, a
  civilization's own never weakens): the Fellowship of Leaves lush, the Order and the Empyrean
  holy, the Runes of Kilmorph arcane, the Ashen Veil corruption and ash, the Octopus Overlords and
  the Council of Esus gloom -- so a land's look changes when its faith does, and a faith's spread
  shows on the map. The sea takes half of its owner's mood; hell overrides the others where it holds
  (all but its ash). Unrevealed plots carry none. Read LINEAR
  with a noise-warped lookup, so a region's edge is organic, not the plot grid; off the map's
  edges there is none. The **mood pass** grades the display colour under each pixel (found from the
  depth): lush deepens the greens and warms the light; corruption darkens and cools the land toward
  violet and slate by a multiply (its contrast kept), its bright accents pushed to magenta; hell
  chars it to a dark charcoal-brown — its water left dark steel-blue, what glows (the scene's
  emission) left hot — and splits the ground with **glowing cracks** (a world-anchored network,
  broken into separate fissures, pulsing; on the ground only); gloom cools and dims; frost drains
  the colour toward a cold blue-grey and lays **snow in drifts** on the ground (a world-anchored
  noise, thicker as the frost deepens; not on water or cards); the arcane cools and clears the land,
  pushes its bright accents to cyan and draws **ley lines** on the ground (thin isolines of a noise,
  broken into lengths, pulsing); holy light warms the land and lifts its highlights; ash makes it
  sooty and grey, its shade deeper, what glows left hot. Each mood's air tints the distance a little.
  **Particles** (`moodEmitters`, a mood of at least 0.5): violet motes rise over corrupted land,
  cyan ones over the arcane, gold-green ones in lush woods; snow, ash and pale holy motes fall (a
  negative `rise`: the particle shader runs their life backwards); blue-green wisps in gloomy woods.
  And each civilization's accents, by the plot's owner: pale wisps over the Sidar's land, fireflies
  on the Kuriotates' grassland, dust puffs on the Hippus' steppe and the Malakim's desert.
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
  | lab, just before the brighter map (2026-11-14, GPU) | 0.38 | 0.41 | 18% | 0.46 / 0.34 | 2% |
  | lab, the brighter map | 0.50 | 0.54 | 41% | 0.61 / 0.46 | 0% |

  The target is earthy and muted: its saturation is in the accents (the gold town, the Sheaim's
  violet, lava, the banners) and its shade is a dark olive, not black. The second batch read it as
  near-black woods and raised the saturation (1.16): the opposite of the target. So every pixel but
  the figures and banners (the keep mask) loses chroma by hue -- earth and foliage (orange through
  green) about a third, water about two fifths, the rest a tenth -- except what glows (the
  emission) and gold (bright, saturated yellow-orange); greens are pushed toward olive and water
  toward slate teal; the middle values are lightened a little (a gamma of 0.84) and the shade lifted
  toward a dark olive floor (the painted ground only: the pixel-art ground of §5g is art, as the
  cards are, its keep mask 0.87 -- over the painted ground's values, under the props' 0.9 -- and
  its values stay as authored). The finish pass's saturation is 1.0 and its contrast 1.12 (1.16 and
  1.22 before); the lush mood deepens the greens in value only (its saturation boost, 1.28, went).
  Re-measure with the table's columns after a change to the grade or the art.

  **Brighter than the target** (the owner's request, 2026-11-14: the map read dark and
  desaturated; `docs/hd2d/brighter-before-after.jpg`). The land grade now takes a tenth of the
  chroma of earth, foliage and water (a third and two fifths before) and a twenty-fifth of the
  rest, and its olive and teal pushes are under half as strong. The finish pass has a
  **brightness** (a gamma over the whole picture, figures included: 1.3 here, 1 in the other
  looks; a row in the panel), saturation 1.3 (the gamma washes colour out; this gives it back),
  contrast 1.1 and a vignette of 0.18 (0.42 before, which dimmed the corners by two fifths).
  Exposure and the sky's fill were not the lever: the cards cancel the exposure and the filmic
  curve (`invFilmic(lin) / uExposure`), so raising them lifts the ground only. The light shafts
  went from 0.6 to 0.45 (the gamma lifts their haze too). The gamma also lightens the unknown's
  cover and fogged land: not retuned here. The saved panel settings' key went to
  `ffh.hd2d.config.4`.
* **The Armageddon counter** (the owner's request, 2026-11-17, after two mock-ups: a green map at
  the start, a dark red one at counter 100). The map's light and grade follow the counter of the
  player the view follows (`me().armageddon`; `?doom=<counter>` previews a value for one page,
  never saved). Two curves from the counter's share of 100, `x` (`config.js doomOf`): the **light**
  takes `x` -- `doomLight` blends LIGHT through `LIGHT_DUSK` at 50 (a low amber sun, a golden haze)
  to `LIGHT_DOOM` at 100 (a red sun, a blood-red fill, a dark ember haze; a straight blend from the
  blue haze to the red one went through mauve) -- and the **land** takes `x^1.6`, so it dies later
  than the light turns (counter 20: the land 8%, still green; 50: a third; 100: whole). In the mood
  pass (`uDusk` = x, `uDoom` = x^1.6; after the land palette, before the towns' and figures' share
  of the grade): a golden cast at the middle, given way to the red; the foliage withers toward
  olive-brown; the sea (by its keep mask, mostly 0 -- not by hue, which the grade's colour moves --
  minus the unknown's light parchment) dulls to slate, then goes to a dark blood-red, the rivers
  with it; the land darkens under a red cast, the shade to maroon, the distance to an ember haze,
  the corners close in; from about 60 the hell grounds' glowing cracks open on all land (the
  ground's keep mask only). What glows (the emission: lava, fire, mana, the towns' lights) keeps its
  colour and stands out the more; figures take a quarter of the change in value, towns a little
  under half, as with the region moods. At counter 0 the light is LIGHT itself and the pass adds
  nothing. Screenshots: `docs/hd2d/armageddon-0-50-100.jpg`. Not done: the sprites themselves
  (dead or bare trees, ash in the air) do not change.
* **No light shafts** (removed, the owner's call, 2026-11-21). The mode had a half-resolution pass
  (`RAYS_FS`, config `rays` 0.45): a sunlit haze scattered through the gaps of a drifting cloud
  layer, and broad soft beams painted fanning down from the sun beyond the upper left of the view.
  The pass, its shader, its two targets, its config key and its panel row are gone; a saved panel
  tuning's `rays` is ignored. (The light beams over mana and towers, below, are another thing and
  stay.) The measurements below that name the shafts or the rays predate this.
* **Wide bloom** (quarter resolution) of the emission and the brightest light, added before the
  filmic curve; non-finite pixels give no light (a lone NaN would otherwise spread into a block).
  Both guards test the bits, not `isnan`/`isinf`, which Direct3D 11's compiler folds to false (there
  a NaN then went through `min(c, 32)` as 32: a purple blaze over the Sheaim lake in the lab).
* **Light beams**: restrained school-coloured wisps over mana nodes (0.6 plots tall) and
  resources (0.3); mages' towers retain 1.5-plot columns. Soft additive light, shimmering.
* **Glowing borders**: the overlay's border becomes a hot core in the owner's colour, meandering a
  little inside the owner's side (so the plot grid does not show in it; where two realms meet each
  draws its own colour), its light spilling softly over that side; **no grid**.
* **Mended pixels**: the HD-2D compose gives a non-finite pixel its neighbours' mean, as a guard
  (the 3D renderer's terrain wrote some on a real GPU and its water spread them, until fixed:
  docs/R3D.md §5.2).
* **Tilt-shift** (as the frame goes to the canvas): sharp in a band across the view at the hovered
  plot (else the selection, else the middle; eased, so the band glides), blurring toward a half-
  and then a quarter-resolution copy above and below it — a little more at the top (far) — but
  never past a mild softening; off at the far zooms (fully on from 90 px a plot), and off when the
  frame is drawn smaller than the canvas (render scale under 1, the quality ladder on a slow GPU):
  that frame is stretched and sharpened instead (`UPSCALE_FS`: each pixel pushed from the mean of
  its four neighbours, clamped to their range), not blurred further.

## 6b. A city's ritual

A city whose production is a ritual (its view record's `ritual`, the ritual's name, and
`ritual_type`, its project's type) is marked on the map by a column of light and a storm over it
(`r3d/ritual.js`; the 3D renderer draws it too). The colours below are the arcane look's (violet);
each ritual has its own (see "Looks" below).

* **The beam**: a camera-facing strip from the city's centre to nearly the camera's height (out of
  the top of the view), a white core a quarter of its width and a violet halo, streaming upward and
  pulsing slowly. 0.84 plots across with its halo; never under 14 px (the core 4 px) however far out.
* **The storm**: horizontal discs 1.9 and 2.2 plots over the city, 2.6 plots in radius (the upper
  a little smaller), and a funnel of half the width 0.55 plots under them: grey-violet cloud in
  three spiral arms (value noise sheared by a logarithmic twist, the inner bands turning faster),
  densest and darkest in a ring round an eye where the beam passes, the inner bands lit violet, the
  billows' faces toward the sun lighter, a ragged edge, now and then a flicker of lightning in the
  ring. From the gameplay zoom its near edge stands well over the town and its far half runs out of
  the top of the view. (The 3D renderer's steeper camera sees it from higher: there it hangs over
  the town rather than above it.)
* **The storm's shadow**: the land within about 2.1 plots darkened (30-50% in the lab's shots) and
  tinted violet, its arms turning with the clouds, and a violet pool of light at the beam's foot
  that lights the town's cards. Each pixel's surface is found from the scene's depth, so it falls
  on the ground, trees and buildings alike.
* **Motes**: violet and white particles rising round the town (Map3D.sync adds the emitters).

**Looks.** `RITUAL_LOOK_OF` gives each project a look in `RITUAL_LOOKS` (a project without one
takes `arcane`): the beam's core and halo, the light it casts (the storm's inner bands, the pool at
its foot, the lightning), the cloud's three tones (depths, body, sunlit faces), the shadow's tint and
strength, the storm's turn (speed and sense: the Lanun's sea storm and the hells turn clockwise),
its number of spiral arms, how often lightning flickers (none for Sanctuary, the mist, the night),
its density, and the motes (colours, sizes, rise; some smoke: hellfire's soot, the Delve's dust,
the Sidar's mist, the Lanun's spray). The holy, sun, genesis and phoenix looks have light clouds and
a light shadow; the night, blood and hellfire looks the darkest. A node test
(`test/r3d-ritual.test.mjs`) checks that every project in projects.jsonc has a look.

**Zoom fade.** The storm hides what lies under it, so it fades as the camera rises (`zoomFade`,
`RITUAL_LOOK.fade`, by the pixels a plot): whole from 200 px a plot in, about a third at the
gameplay zoom (96), 6% from 64 out; the shadow keeps at least 20% and the beam 55% (it stays the
ritual's mark on the far view). The motes do not fade.

Drawn in the overlay stage, after the ground marks and the x-ray figures and before the particles:
over the finished display colour, so the storm's darks are as chosen, not lifted by the grade. Per
city the shadow (a quad round the city, depth test off), the storm's three layers (alpha-blended,
depth-tested: what stands nearer the camera hides them), then the whole beam over them (depth-tested:
the town's front hides its foot). One program (`ritual` in pipeline.js's list, linked with the
others), a strip of 8 quads drawn five times a city; nothing drawn or bound while no city holds a
ritual. It moves with the frame's time, and is a still frame of the same when the map does not
animate. The lab shows it with `&ritual=1` (the city nearest the camera's point), `&ritual=all` or
`&ritual=id:3,id:5`, and the look with `&ritualType=PROJECT_RITUAL_RAGING_SEAS` (`cycle`: each city
another).

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
* **Shader compile times**: `r3d-lab/compile.html` (served by `--serve`) compiles the programs the
  mode builds for a configuration (`?renderer=hd2d`, the default, or `3d`; `?r3d=` read as the
  renderer reads it), one at a time or all at once (`&parallel=1`, as a start does), and logs each
  one's link and first-draw times, drawn into the targets the pipeline draws it into
  (`?only=terrain`). docs/R3D.md §11.1, "Measuring", says how to compare the compile-time switches
  (§11) on Direct3D.
* `tools/r3d-compare.mjs` saves `hd2d.png`, `hd2d-close.png`, `hd2d-far.png` beside the art and
  isometric shots; `tools/r3d-perf.mjs` measures the mode (configs: hd2d, no-tilt,
  no-bloom, no-mood, no-hd2d-post, plain). Both drew the 3D renderer's own mode too, until it was
  retired from the client (it was `--renderer hd2d` then).
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
* **Art**: `tools/hd2d/draw.py` (the props drawn by code), `tools/hd2d/trees.py` (the woods' trees), `tools/hd2d/rocks.py` (stones and haystacks), `tools/hd2d/ground.py` (the ground's tiles, §5g; `--sheet OUT.png` for a look at them) and `tools/hd2d/cliffs.py` (the faces' strips, §5f), `tools/hd2d/local_props.py` (the image
  model's, through a local ComfyUI, §5b), `tools/hd2d/gen.py` (sheets from OpenAI's
  `gpt-image-2.5-sunburst`, pinned to its 2026-09-08 snapshot -- gpt-image-1 was shut down on
  2026-10-23 -- the prop sheets of nine and the town kit's sheets of `tools/hd2d/towns.json`, with
  `--ref` images to match an accepted sheet's style; cut apart by `art.py split SHEET PREFIX`, or
  for the town kit by `tools/hd2d/towns.py`, §5c), `tools/hd2d/art.py
  prop SRC NAME HEIGHT [DEEPEN]` (an object on white to a card), `index` (writes
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
* **What each switch costs, against what it shows** (2026-10-30, after batch 4: an RTX 4060 laptop,
  Vulkan, a 1920 × 1080 canvas, the `stage corrupt` save at (46, 27); GPU timer queries, the
  median of rounds alternating the cases; "shows": the share of the frame's pixels a switch moves by
  more than 8 of 255). At 96 px a plot the GPU frame is 10.2 ms: the terrain's shading 3.6, the
  water 1.9, the overlay 0.7, the shafts 0.5 (they show over 34% of the frame), the shadows 0.9 in
  all (15%), the props and the cards 0.3-0.4 each, every other pass 0.1-0.3. The mode's post passes
  all earn their cost. The switches that only weight a term of a shader (dabs, pigment, brushwork,
  cloud shadows) save nothing when set to 0: the term is still computed. Compiled out, the **brush
  dabs** saved 0.5 ms of the terrain's shading (at 40 and 96 px) and 7% of its instructions (7110
  to 6621; Direct3D's compiler alone took 6.1-6.3 s over it against 6.7-9.3, alternating runs: the
  whole start drifted too much between runs on that laptop to show it) for no pixel moved by 8 at
  96 px, 0.1% at 168, and at 220, the closest zoom, a few flower flecks: they are compiled out of
  the mode's terrain since (`NO_DABS`, `MapHd2d.terrainDefines`; the 3D renderer keeps them, its
  meadow's flowers show close up). The brushwork on objects (0.2 ms) stays: the mode's 3D fields and sheaves
  take their texture from it close up. Ambient occlusion (0.25 ms) stays: its shade under the woods
  shows close up. Glow and bloom (0.15 ms each) are cheap.
* **The lean terrain** (2026-10-31): what Direct3D compiles at the
  first draw is mostly the terrain shader (docs/R3D.md §11.1), its time about 2.6e-7 s × N² for N
  instructions. Counted after inlining and optimisation (glslang, spirv-opt; within 5% of FXC's own
  counts), the mode's terrain was 6308 instructions and 125 value noises. Two steps:
  * **Exact, both renderers:** the noises several branches took with the same arguments are worked
    out once a fragment, where one of those branches runs (`gBig`, `gMid`, `gDune`, `gN2`..`gN9`:
    the coast's masses are the meadow's, the burning sands' dunes the desert's, the fields', the
    peaks' snow, the beach, the rock, the roads and the floors' small noises): 6308 to 5736 (107
    noises; the 3D renderer's 6756 to 6184), the five lab stills bit-identical.
  * **Lean, the mode only** (`TERRAIN_LEAN`, hd2d/config.js; its switch `terrainLean` was removed
    2026-11-10): every open ground's broad masses from the
    meadow's noises, fewer octaves where only a broad shape depends on them, the perdition's pools
    from the marsh's noise, the hell marks' mask, pulse and sparks, the road edge's finest wobble,
    the rock's crags and strata, the parchment's dabs, four noises shared with neighbours of about
    their size, the marsh's sheen and the golden grass's streaks: 5736 to 3363 instructions (44
    noises), Direct3D's compile about 10.4 s to 2.9 s on the RTX 4060 laptop (estimated). The
    stills (SwiftShader, 640 × 400): 0.6-1.4% of the pixels moved by more than 8 at 96-168 px a
    plot (the woods' floor, the road edges), 3.5% on the showcase (the marsh's masses and the
    road), 4.8% at 40 px (the hell grounds' masses and cracks, the parchment); the turn-200 game
    (`r3d-compare`, 800 × 500: hell grounds, a desert, a coast) 1.1-1.5% at 40, 96 and 168 px. The
    masses moved, the look kept. Tried and dropped: two octaves for the pools and the hell cracks (the cracks
    widened into blobs), the pigment's drift from the borders' noises (20% of a still a shade
    lighter, for two noises).
* **An integrated GPU** (2026-10-31, the owner's laptop: Chrome's default adapter there is the Intel
  UHD beside the RTX 4080, and WebGL's `powerPreference: "high-performance"` does not switch it on
  Windows; the turn-200 large-map game at 96 px, a 1600 × 900 page at device-pixel ratio 1.5, so a
  2400 × 1350 canvas). The same frame that takes the RTX 4080 3-5 ms took the UHD 87-113 ms (the
  scene pass 55, the rest spread over every pass), and the map animated at rest, so the GPU never
  rested: the page ran at 4 frames a second with nothing moving and panned at 8. The quality ladder
  never stepped in: it timed the frame on the CPU (under 1 ms; WebGL calls return before the GPU
  has done them). The cost is nearly all per pixel (render scale 0.75: 52 ms, 0.5: 30 ms; the rays,
  ambient occlusion, moods or reflections off each moved nothing measurable). Now the ladder reads
  the GPU's own time (one timer query a frame, `Pipeline.gpuFrameMs`; without timer queries the
  loop's call-back gap, which grows once the GPU is behind), has a last step at render scale 0.5,
  and does not step back up to a level that was slow again within 5 s (`autoFloor`); a map at rest
  animates at most every other frame's worth of GPU time (`2 × costMs`, at least the 32 ms it had).
  On the UHD: 138 frames a second at rest (the page's), 19 panning, the ladder at its last step
  within about 3 s, and the page shows a hint to pick the faster GPU once (main.js,
  "ffh:r3d-slow"): Windows' Settings → System → Display → Graphics → the browser → High
  performance, which gives the full map at the RTX's 3-5 ms. On the RTX nothing changes (level 0,
  30 frames a second at rest).
* **The props' and the water's programs, compiled smaller** (2026-10-31; `MapHd2d`
  `propDefines`, `waterDefines`, `riverSplit`, `LITE` in `hd2d/index.js`). After the terrain they
  were the longest of Direct3D's first-draw compiles (docs/R3D.md §11.1). Instruction counts after
  inlining and optimisation (glslang, spirv-opt -O, about 0.95 of FXC's own count; FXC's time about
  2.6e-7 s × N² on the RTX 4060 laptop):

  | program | before | after | FXC, estimated |
  |---|---|---|---|
  | props | 2856 | 1045 | 2.1 s -> 0.28 s |
  | water (sea + rivers) | 3159 | sea 1568 + rivers 1643 | 2.6 s -> 0.64 + 0.70 s |

  In the mode the prop program draws only what no card claims: the banks' boulders, reeds and
  overhanging grass, the farms' rows, sheaves, barns, fences and carts, bridges, palms, volcanoes; no
  crown of leaf clusters (its trees, bushes and saplings are cards), no team cloth, no unit.
  `NO_LEAVES` compiles the leaf clusters' material out (exact: none is drawn); `DABS_1` lays each
  brush mark from the one plane that carries the face instead of blending three (the weights, the
  normal's fourth power, give that plane nearly all of every face; a third of the marks' code);
  `NO_FINE_PIGMENT` drops the fine pigment noise under the marks (a few percent: nothing moved by 8 of
  255). The strokes themselves stay: compiled out (`NO_STROKES`, tried and not taken) the fields'
  rows and the sheaves turn to smooth plastic close up (3.1% of rivertown's pixels moved by more
  than 8). The water's rivers
  are a program of their own (`RIVER_SPLIT`, exact), the sea's normals come from noise gradients
  (`GRAD_NORMALS`: three noises instead of nine; the rivers keep their finite difference, whose step
  across the channel is wide enough that gradients moved what they mirror), the body's mottle has
  two octaves (`BODY_LITE`), and across the fog of war's frontier each drift fades toward its mean
  instead of being worked out twice, moving and at rest (`CALM_FADE`, about 0.5 s of the two
  programs' compile): the only change a player could see, a strip along the frontier where the
  water's strokes and foam calm. Glitter and caustics stay (each another 0.1 s; compiled out, tried
  and not taken, the sea's and rivers' glints go, and a faint net seen close up). Against the
  original programs (SwiftShader, animation frozen) the whole set moves at most 0.28% of a lab
  still's pixels by more than 8 of 255 (0.5% on a close view of the coast: the sea's ripples) and
  0.07% of the turn-200 game's HD-2D shots; the 3D renderer's programs are unchanged (identical
  after preprocessing). (The switches that compiled the original programs or chose among these,
  `lite`, `liteOff` and `liteOn`, and the optional `NO_STROKES`, `NO_GLITTER`, `NO_CAUSTICS` and
  `GRAD_NORMALS_RIVER` were removed 2026-11-10.)

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
map it wears its own dress (`hd2d.css`, `html.iso.hd2d`, set by `syncSkin`), after the menus of
the HD-2D role-playing games (the owner's reference, 2026-11-02): a thin humanist sans in place of
Herald (a font family whose regular weight is Candara Light, Gill Sans Light or Optima, whichever the
system has, and whose bold is the book weight, with lining figures); panels of dark glass the map
shows through, a little blurred, edged with a hairline instead of the gilt frames (iso.css's frame
variables are redefined as hairlines); dialogs titled on a dark strip at the left, sections headed
by dim lettering on darker strips; buttons and list rows lit by a pale band fading to the right; and
the menus' entries (the title screen's, the advisors' rail) as banners with a pointed end, the one
pointed at slid out of the column with a diamond past its point. The icons stay the M7 set's. The
battle screens keep the isometric skin's pixel look. On a desktop the panels are drawn at 80%, so
the map, not the panels, holds the screen.

The figures are cards that face the camera: they look the same at any pitch. The settlements and
buildings are cards too, but their art has its ground painted in, drawn for the isometric map's
2:1 view (about 30° up, and seen along a plot's diagonal, where this camera looks along its side).
Rendered at 30, 40, 50 and 60° ([hd2d/city-pitch.jpg](hd2d/city-pitch.jpg)): at 30-40° the city
sits in the land; at 50° it starts to read as a card standing on the ground (its painted ground no
longer matches the terrain's foreshortening); at 60° it plainly is one. **The steepest camera the
settlement sprites tolerate is about 45°**, so the mode's camera keeps between 30 and 42° at every
zoom.

The one exception is the city screen (2026-11-21, the owner's request): while it is open the camera
turns to look straight down (`Map3D.top` eases it to 90° and back, map3d.js `camera`), so the
city's plots read as a square grid and each citizen's plot is plain to see. The cards then lie flat,
facing up; the tilt-shift is off and the ground's art pixels square again north-south
(`beforeFrame`). The isometric fallback keeps its view: its art is drawn for that angle.

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
3. **True cliffs and wide water** (code, the terrain). The cliffs are closed by the terrain pass
   (§5f: the mesh cuts a sheer face at the shore and at a hill's scarp). What remains: the target's
   rivers are broader and braided, and a river's mouth over the land's face is a chute of white
   water, a waterfall only where it faces the camera.
4. **Darker, moodier sprite variants and figures facing left or animated** (unit art; set aside by
   the owner). The anim-sprint idles now play (§5, "Idle figures"), for the 129 sprites that have one.
5. **The sky and the map's edge** (code): a small map's edge shows the 3D renderer's void.
6. **On a real GPU**: batches 3 and 4 were judged on software renders; the palette's numbers should
   be re-measured on the owner's GPU (§6's table).

## 11. Configuration

`r3d/hd2d/config.js`: the `hd2d` preset (the 3D renderer's painterly switches with: a lower camera,
`pitch` 32 and `fov` 30; `grid` 0; the towns' switches, §5d: `townScale` 1.25, `townLight` 0.5,
`townClearing` 0.5, `townBanners` 0.6; deeper shade, `stylize` 0.7, `ssao` 0.85; `brightness` 1.3,
`saturation` 1.3 and `contrast` 1.1 (the brighter map, §6), `vignette` 0.18, `haze`
0.1, `mist` 0.08, `clouds` 0.3, `glow` 1.0) and the mode's own switches: `tiltShift` (0.3), `mood` (1), `landGrade` (1, the land's palette, §6), `bloom` (0.8, from a lower threshold),
`beams` (1), `borderGlow` (1), `contact` (1), `lean` (1). `LIGHT` gives little sky and ground fill
(0.5 and 0.15). The `plain` preset turns the stylisation and the mode's passes off (the cards stay).
Its `LIGHT` is the one light. Stored apart from the 3D renderer's look (`ffh.hd2d.config.4`, renamed
when the look changes enough that a saved panel tuning would mislead); the panel offers the 3D
switches that apply (not the grid or brush dabs, which the mode's terrain is compiled without, §8)
and the mode's rows. The mode always compiles its terrain, water and props with the value noise
from a hash texture (docs/R3D.md §11.1, `NOISE_TEX`; about half their compile on Direct3D), the
lean terrain (§8), the stand-in's ground (§5b) and the smaller props' and water programs (§8), and
always does the start-up's CPU work while the programs compile, uploading the sprites from memory.
`packScene` (on Direct3D only, unless 1 or 0 says otherwise) packs the scene into one target
(docs/R3D.md §11.1: the scene's programs compile at link there; other backends compile at link
anyway); `r3d-lab/compile.html` takes the same `?r3d=` to compare compile times (§7).
The page-only switches that drew the mode as before these savings -- `noiseTex`, `terrainLean`,
`lite`/`liteOff`/`liteOn`, `terrainDefs`, `fastStart`, `earlySync`, `terrainStandin`, `standin`
and `standinHold` (the two-stage start with the full painting dissolving in over the stand-in) --
were removed 2026-11-10 (their defaults kept), as were the 3D renderer's retired paint filter, ink
outlines and light banding (docs/R3D.md §5).

## 12. Removing it

Delete `assets/app/r3d/hd2d/` (with its `art/`), `assets/test/hd2d.test.mjs` and
`r3d-steps.test.mjs` (with world.js's `cliff` and terrain.js's `buildCliffChunk`, the mode's alone),
`r3d-lab/compile.html`, `tools/hd2d/`, this file and `docs/art/hd2d-reference.png`, and the
`r3d/hd2d/art` folder from `build.rs`; remove the `app/r3d/hd2d/*` lines of `ASSETS` in
`src/transport/http.rs`, `"hd2d"` and its option in `look.js`/`tools.js` (with another default
there), `useHd2d` and the `"hd2d"` branches in `main.js` and `map.js`, `scene=hd2d`/`renderer=hd2d`
in the lab, the `hd2d` shots in
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
* **The region moods** come from tables of civilizations, alignments and state religions; a game
  event (Armageddon, a spreading hell) changes them only through the terrain and the owners. The
  Malakim's heat shimmer and the Lanun's coloured sea were suggested and not built (the shimmer needs
  a screen distortion the mood pass has not; the sea's colour a channel of its own).
* **Many figures stand still**: 129 of the 373 unit sprites breathe (§5), among them the common
  ones (warriors, axemen, workers, settlers, adepts and horsemen, generic and every
  civilization's). Next: the other riders and centaurs, whose unit files are on `anim-all` and
  breathe with its mounted rig as the horsemen do: the knight, horse archer, death knight,
  shadowrider, mounted mercenary, the four centaurs, the two wolf riders and the boar and bison
  riders (that branch has the camel archer's file too). The animals, the other archers and
  casters, and most heroes and unique units stand still as well; an idle for a biped needs only its
  unit file (anim-sprint/ANIMATING_UNITS.md; no image calls). Figures are mirrored when facing
  west, so their painted light then comes from the upper right.
  An idle figure's contact shadow is a 96-wide sprite's (the card is the 160-wide frame), a little
  narrow under Hyborem (105).
* **The sprite array** holds one sprite a 256-texel layer, however small (an idle strip two): the town kit adds up to
  10 layers a civilization in play (7 or so of them owner variants), on top of the props, figures,
  buildings and banners. A game of many civilizations can pass 256 layers, and the array then
  doubles to 512 (about 180 MB of texture with mipmaps, capped at the GPU's limit). Packing small
  cards several to a layer would need an offset per card in the instance data.
* **Cities' garrisons** show one figure (the banner's badge carries the count), as the isometric map
  hides them.
* **The terrain pass's loose ends** (§5f, §5g): a river's mouth over the land's face; a scarp only
  on the side a hill turns to the camera; contact shadows, rings and sea lanes ramping across a
  step; the parchment, a volcano's snow and the sea floor still the 3D renderer's painting; one
  tile of art a ground; the water not yet in the cards' medium.
* Next, by value: new art per §10; the first frame's stall on Direct3D 11
  (§8: cheaper terrain, water and prop shaders); picking by card; idle frames for the
  other riders and centaurs (above), then the other common units still standing; a sky or haze bank beyond the map's north edge.

## 14. Provenance

The units, buildings and resources are the isometric renderer's sprites (docs/ISO.md: drawn by an
image model and brought to one contract by code in `tools/m7/`), as are the cities of a family
without the town kit. The ground's tiles and the faces' strips (`gnd_*.png`) are drawn by code
(`tools/hd2d/ground.py`, `cliffs.py`; since 2026-11-16: the seven ground layers before them were
the image model's). The prop
cards in `assets/app/r3d/hd2d/art/` were drawn by FLUX 3 Image Klein (Black Forest Labs) running
locally in ComfyUI, and post-processed by `tools/hd2d/art.py` (prompts in §5b's spirit: 16-bit
pixel art, top-down for ground, a single object lit from the upper left on white for props). The grass, reeds, standing stones and
fences are drawn by code (`tools/hd2d/draw.py`), as are the woods' oaks, pines and snowy pines
(`tools/hd2d/trees.py`, after the owner's tile sketch) and the boulders, shore stones, pebbles and
haystacks (`tools/hd2d/rocks.py`, the same recipe). `tools/hd2d/gen.py`
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
