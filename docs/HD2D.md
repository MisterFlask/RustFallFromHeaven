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
6 light, moods and the post passes · 7 the lab and tools · 8 on a real GPU · 9 cities and the camera ·
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
  guard and `?force3d=1`). On Windows, Chrome's default ANGLE backend (Direct3D 11) compiles its
  programs in about 5 s and stalls the first frame for 7-8 s more; with `--use-angle=vulkan` the
  mode starts in about a second (§8, docs/R3D.md §11.1).
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

| | |
|---|---|
| the look target and the lab's composition after it, side by side | [hd2d/lab-vs-reference.jpg](hd2d/lab-vs-reference.jpg) |
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
      config.js   the HD-2D preset, its light, its panel rows (stored apart: ffh.hd2d.config)
      dress.js    which card stands where: unit groups and banners, cities, buildings, resources,
                  light beams; the claims
      mood.js     the per-plot mood channel and its particles
      sprites.js  the sprite atlas (a texture array) and the card instance buffers
      passes.js   the pipeline extension
      shaders.js  the cards, their shadows and ghosts, the beams, the post passes
r3d-lab/          scene=hd2d, renderer=hd2d, compile.html (§7)
tools/            r3d-lab.mjs --capture, r3d-perf.mjs --renderer hd2d, r3d-compare.mjs hd2d shots
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
* **Settlements and buildings**: a city is its civilization's village, town or castle sprite by
  size (`iso.js` `citySprites`), its card's foot on the plot's front edge so its painted ground lies
  over the plot, with the owner's banner at its front-left; its garrison is one figure (80%) in
  front of it, at the front-left corner or, out of a river, the front-right or the front's middle
  (the banner's badge carries the count). A standing improvement (`iso.js` `improvementSprites`:
  cottages, forts, towers, mines, mills, camps, lairs, ruins, mana nodes, boats) is a card likewise,
  smaller than a city, its painted ground thinned out over its foot by a dither fixed to the art's
  texels (it melts into the 3D land instead of ending in the isometric diamond's edge); flat ones
  (farms, pastures, plantations, wineries, quarries) stay the 3D renderer's fields. A resource is
  its family's prop at the plot's front-left. Mana and gems glow (their bright, saturated pixels
  feed the bloom). What a card stands for is **claimed** from the 3D dressing (no house, wall,
  structure or marker is modelled under it); trees keep out of a town's reach as before.
* Scale (`dress.js` `PX`): a unit's 96 × 120 canvas is 0.46 × 0.58 of a plot, a 192-wide
  settlement 1.15 plots, other buildings 0.73, resources' canvases 0.65 (their props about 0.25),
  banners 0.8 tall.

## 6. Light, moods and the post passes

* **The sun** stands at the upper left of the view (west and a little north, about 45° up), where
  the sprites' painted light comes from; shadows fall toward the lower right. The camera never turns.
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
* **Light shafts** (half resolution): the sunlit haze — the sun's light scattered along each view
  ray near the ground, through the gaps of a high drifting cloud layer and past the props' shadow
  map (anchored to the world) — and painted **beams**: broad soft beams fanning from the sun beyond
  the upper left, strongest over the far land, turning slowly with the camera's position (a distant
  layer, not stuck to the lens) and fading out below about 110 px a plot (gone by 60). Seen from
  above, a view ray crosses too little air for the physical shafts alone to show, so the beams are
  painted, as in the target. They lie in the air before the land: over a figure, a third as strong.
* **Wide bloom** (quarter resolution) of the emission and the brightest light, added before the
  filmic curve; non-finite pixels give no light (a lone NaN would otherwise spread into a block).
* **Light beams** over mana nodes (1.5 plots tall), mana resources and mages' towers: soft additive
  columns in the node's school colour (`palette.js` MANA), shimmering.
* **Glowing borders**: the overlay's border becomes a hot core in the owner's colour, meandering a
  little inside the owner's side (so the plot grid does not show in it; where two realms meet each
  draws its own colour), its light spilling softly over that side; **no grid**.
* **Mended pixels**: the HD-2D compose gives a non-finite pixel its neighbours' mean (the 3D
  renderer's water writes some on a real GPU, §13).
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
  as cards; nothing stands on, and no mood comes from, an unrevealed plot).
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
  while the page stays live. The first frame then stalls for 7-8 s more there: ANGLE compiles the
  terrain, water and prop pixel shaders a second time at their first draw into three targets
  (4.4-4.8, 1.4-1.6 and 1.0-1.1 s; the cards 0.03-0.07 s). On Vulkan every program compiles in about
  0.2 s together and the first frame takes 0.3 s; every measurement below is on Vulkan.
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

The camera is the 3D renderer's with a lower pitch, a narrower swing and a wider lens (`pitch` 36,
`fov` 30 against 50 and 24; `pitchAt`): 42° from the horizon at the farthest zoom, about 35° at the
gameplay zoom (96 px a plot) and 30° at the closest, so the top of the view runs out toward the
horizon.

The figures are cards that face the camera: they look the same at any pitch. The settlements and
buildings are cards too, but their art has its ground painted in, drawn for the isometric map's
2:1 view (about 30° up, and seen along a plot's diagonal, where this camera looks along its side).
Rendered at 30, 40, 50 and 60° ([hd2d/city-pitch.jpg](hd2d/city-pitch.jpg)): at 30-40° the city
sits in the land; at 50° it starts to read as a card standing on the ground (its painted ground no
longer matches the terrain's foreshortening); at 60° it plainly is one. **The steepest camera the
settlement sprites tolerate is about 45°**, so the mode's camera keeps between 30 and 42° at every
zoom.

## 10. The gaps to the reference

Side by side ([hd2d/lab-vs-reference.jpg](hd2d/lab-vs-reference.jpg)), ranked by how much of the
remaining distance each closes; each says what new art (or code) would close it.

1. **Ground textures** (new art). The reference's land is a dense mosaic of pixel-painted ground —
   meadow, moss, rock, sand, every tile busy with small props; ours is the 3D renderer's smooth,
   painterly materials, which read as a different medium under pixel-art cards and leave open
   ground flat. Pixel-art ground textures for the 3D materials (tiling, world-anchored, at the
   sprites' texel density) would close the largest part of the gap.
2. **Single trees cut for cards** (new art). The reference's forests are crowds of pixel-art trees;
   ours are the 3D renderer's modelled crowns — well lit, but the one 3D-looking thing among the
   cards. The iso forest sprites are whole-tile clumps with their ground painted in, so they
   cannot stand as cards without that ground showing. Single trees (and bushes, rocks) cut out for
   cards, a few variants each, would let woods be scattered card by card.
3. **Cities without their ground patch** (new art). A settlement's painted diamond of ground is the
   isometric map's, seen along a plot's diagonal from 30°: on this camera it reads as a plinth, and
   it limits the camera to about 45° (§9). The same towns drawn without ground (and, ideally, a
   little larger, as the reference's Eärlann spans two tiles) would sit in the 3D land at any
   pitch; a matching ground decal can be drawn by code.
4. **Darker and moodier variants of the sprites** (new art, or a code stop-gap). The sprites are
   painted for a bright sunny board; the corrupted and hell regions grade them only at 40% (so they
   stay readable). The reference's Sheaim and infernal towns and figures are themselves dark,
   purple and ember-lit. Night-and-corruption variants (or a palette-mapped recolour per mood, which
   code could do now) would fit them into their regions.
5. **Mana crystals and region props** (new art). The reference strews its corrupted land with
   purple crystals, its forests with glowing flowers and its hell with lava pools; our regions get
   their mood from the grade, the cracks and particles only. A small set of crystal, flower and
   lava-rock cards would carry each mood into the land itself.
6. **Unit sprites facing left and animated** (new art). Figures are mirrored when they face west
   (the light painted from the upper left then comes from the upper right), and stand still; the
   reference's groups face the viewer. Idle frames already exist for a few units in
   `anim-sprint/`; a two-frame idle and a left-facing pose would bring the groups to life.
7. **Tilt-shift and haze strength, the sky** (code, phase 2). The reference blurs more strongly and
   shows no map edge; ours stays mild by the brief (the top third must read), and the edge of a
   small map shows the 3D renderer's parchment void. A painted sky or far haze bank for the map's
   north edge is code.

## 11. Configuration

`r3d/hd2d/config.js`: the `hd2d` preset (the 3D renderer's painterly switches with: a lower camera,
`pitch` 36 and `fov` 30; `grid` 0; `saturation` 1.14, `contrast` 1.14, `vignette` 0.32, `haze` 0.22,
`clouds` 0.22, `ssao` 0.6, `glow` 0.8) and the mode's own switches: `tiltShift` (0.4), `rays` (0.55),
`mood` (1), `bloom` (0.45), `beams` (1), `borderGlow` (1), `contact` (1), `lean` (1). The `plain`
preset turns the stylisation and the mode's passes off (the cards stay). Its `LIGHT` is the one
light. Stored apart from the 3D renderer's look (`ffh.hd2d.config`); the panel offers the 3D
switches that apply (not the paint filter, ink or grid) and the mode's rows.

## 12. Removing it

Delete `assets/app/r3d/hd2d/`, `assets/test/hd2d.test.mjs`, `r3d-lab/compile.html`, this file and
`docs/art/hd2d-reference.png`; remove the `app/r3d/hd2d/*` lines of `ASSETS` in
`src/transport/http.rs`, `EXPERIMENTAL_HD2D` and its option in `look.js`/`tools.js`, the `"hd2d"`
branches in `main.js` and `map.js`, `scene=hd2d`/`renderer=hd2d` in the lab, the `hd2d` shots in
`r3d-compare.mjs` and `--renderer` in `r3d-perf.mjs`. The hooks left in the 3D renderer's files are
its defaults and do no harm.

## 13. Limitations and next steps

* **Windows' default browser backend** cannot compile the 3D renderer's terrain shader (§8): the
  first fix the mode needs is the 3D renderer's, a terrain shader FXC can compile (fewer unrolled
  loops), or the client detecting it and saying so.
* **NaN in the 3D renderer's water**: on a real GPU its water shader writes non-finite pixels along
  contour-like lines in lakes and on the shelf (black dashes in the 3D renderer too; found by
  marking non-finite pixels in the compose). The mode mends them in its compose (each takes its
  neighbours' mean); the cause in `WATER_FS` is still to be found.
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

Every sprite is the isometric renderer's (docs/ISO.md: drawn by an image model and brought to one
contract by code in `tools/m7/`); the banners, the cracks, the beams and every pattern are drawn by
code at run time. No new art, model, texture, font or library was added for this mode; the
reference image is the owner's.
