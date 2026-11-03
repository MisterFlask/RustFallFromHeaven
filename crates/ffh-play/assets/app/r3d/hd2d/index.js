// The HD-2D map (docs/HD2D.md): the isometric renderer's pixel-art sprites standing in the 3D
// renderer's lit world, as in Octopath Traveler or Triangle Strategy -- a perspective camera tilted
// toward the horizon, figures, towns and props as flat cards facing it, the light painted in the
// sprites matched by a sun from the upper left, light shafts and glow, a mood for each region,
// glowing borders, and a tilt-shift blur. The default map (look.js rendererChoice; the isometric
// map where it cannot run); this module is loaded on demand (map.js useR3d), so the 3D renderer
// alone (the r3d lab's own mode) never loads it.
//
// It is the 3D renderer (../map3d.js Map3D) with its mode hooks set: its own look (hd2d/config.js),
// the cards' dressing (hd2d/dress.js) in place of the modelled figures, cities, buildings and
// resource markers, the per-plot mood channel (hd2d/mood.js), and its passes in the pipeline
// (hd2d/passes.js). Everything else -- the ground, water, trees, picking, camera, HUD, lab and
// tools -- is the 3D renderer's.

import { Map3D } from "../map3d.js";
import { loadConfig, saveConfig, withPreset, PANEL } from "../config.js";
import { clamp, smoothstep } from "../math.js";
import { STORE, LIGHT, DEFAULTS_HD2D, PRESETS_HD2D, PANEL_HD2D, TERRAIN_LEAN, TERRAIN_STANDIN } from "./config.js";
import { SpriteLayer } from "./sprites.js";
import { Hd2dPasses } from "./passes.js";
import { dressSpriteStatic, dressSpriteUnits } from "./dress.js";
import { plotMoods, moodEmitters } from "./mood.js";
import { staticEnv } from "../dress.js";
import { MAT, bakeLayer } from "../materials.js";
import { GROUNDS } from "./art/index.js";

/** The mode's compile-time savings (MapHd2d.liteDefines; shaders.js). `on`: compiled in by default;
 *  `opt`: only with `cfg.liteOn`.
 *  Props: NO_LEAVES -- no crown of leaf clusters stands in the mode (its trees, bushes and saplings
 *  are cards); DABS_1 -- a brush mark from the one plane that carries the face, not a blend of three;
 *  NO_FINE_PIGMENT -- the fine pigment noise (under the brush marks, a few percent) left out.
 *  (NO_STROKES, the brush strokes on matte props, is optional: the fields and sheaves lose their
 *  texture close up without them.)
 *  Water: CALM_FADE -- the frontier's drifts fade toward their means instead of being mixed twice;
 *  BODY_LITE -- the body's mottle in two octaves; GRAD_NORMALS -- the sea's normals from noise
 *  gradients (GRAD_NORMALS_RIVER the rivers' too: optional, it moves what they mirror). RIVER_SPLIT:
 *  the rivers a program of their own. */
const LITE = {
  prop: { on: ["NO_LEAVES", "DABS_1", "NO_FINE_PIGMENT"], opt: ["NO_STROKES"] },
  water: { on: ["CALM_FADE", "BODY_LITE", "GRAD_NORMALS"], opt: ["NO_GLITTER", "NO_CAUSTICS", "GRAD_NORMALS_RIVER"] },
  split: { on: ["RIVER_SPLIT"], opt: [] },
};

/** The 3D renderer's gameplay zoom: on the full-bleed map (the isometric screen's floating
 *  panels) it shows about fifteen plots across, as the look target does, and its figures are about
 *  the target's share of the screen. */
export const DEFAULT_ZOOM = 96;

/** How long the start waits at most for the sprites its first frame asks for (ms; `modeReady`). */
const SPRITE_WAIT_MS = 3000;

/** How far the land stands over its water (plots; dry land's base is 0.1 over the sea). */
export const BANK = 0.04;

/** The pixel-art ground layers (hd2d/art, tools/hd2d/art.py) and the material layer each replaces
 *  in the 3D renderer's ground (materials.js): the terrain shader tints a layer by its mean colour,
 *  so the land keeps its painted design and takes the pixel art's texture. */
const GROUND_LAYER = { gnd_grass: MAT.GRASS, gnd_straw: MAT.STRAW, gnd_dirt: MAT.DIRT, gnd_sand: MAT.SAND, gnd_lichen: MAT.LICHEN, gnd_snow: MAT.SNOW, gnd_rock: MAT.ROCK };

export class MapHd2d extends Map3D {
  constructor(view) {
    super(view);
    this.mode = "hd2d";
    this.focus = null;
    // The 3D renderer is up (its canvas, loop and panel button): if the mode's own setup fails (a
    // program that does not compile), it goes again, so the caller's fallback finds a clean map.
    try {
      // (`fastStart` 0, a page-only override `?r3d={"fastStart":0}`: the original start-up.)
      this.sprites = new SpriteLayer(this.gl, () => {
        this.dirty = true;
      }, this.cfg.fastStart !== 0);
      this.pipeline.ext = new Hd2dPasses(this.gl, this.pipeline, this.sprites);
    } catch (e) {
      this.dispose();
      throw e;
    }
    performance.mark("hd2d:ctor");
    this.groundReady = this.loadGround().then(() => performance.mark("hd2d:ground"));
  }

  /** The pixel-art ground into the 3D renderer's material layers (this scene's own: the 3D
   *  renderer's are untouched), with their means; resolves when done (a layer that fails to load
   *  keeps the painted one). */
  async loadGround() {
    const gl = this.gl, mats = this.scene.materials;
    const T = gl.TEXTURE_2D_ARRAY;
    let n = 0;
    const done = new Set();
    await Promise.all(
      GROUNDS.filter((name) => GROUND_LAYER[name] !== undefined).map(async (name) => {
        try {
          const im = new Image();
          im.src = new URL(`./art/${name}.png`, import.meta.url).href;
          await im.decode();
          if (!this.running || im.width !== 256 || im.height !== 256) return;
          const c = document.createElement("canvas");
          c.width = c.height = 256;
          const g = c.getContext("2d", { willReadFrequently: true });
          g.drawImage(im, 0, 0);
          const px = g.getImageData(0, 0, 256, 256).data;
          let r = 0, gg = 0, b = 0;
          for (let k = 0; k < px.length; k += 4) {
            r += px[k];
            gg += px[k + 1];
            b += px[k + 2];
          }
          const m = 255 * 256 * 256;
          gl.bindTexture(T, mats.tex);
          // The PNG as it is: colour not premultiplied (alpha is the layer's height), no colour
          // conversion (the shader treats the layers' colours as authored).
          gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
          gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
          gl.texSubImage3D(T, 0, 0, 0, GROUND_LAYER[name], 256, 256, 1, gl.RGBA, gl.UNSIGNED_BYTE, im);
          gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.BROWSER_DEFAULT_WEBGL);
          mats.mean.set([r / m, gg / m, b / m], GROUND_LAYER[name] * 3);
          done.add(GROUND_LAYER[name]);
          n++;
        } catch (e) {
          console.warn(`hd2d: ground ${name} did not load`, e);
        }
      }),
    );
    // (fastStart: the scene left the replaced layers clear, replacedMaterials; one whose art did
    // not load is painted now, as the 3D renderer paints it.)
    if (this.running && this.cfg.fastStart !== 0) {
      for (const layer of this.replacedMaterials()) {
        if (done.has(layer)) continue;
        const b = bakeLayer(layer);
        gl.bindTexture(T, mats.tex);
        gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
        gl.texSubImage3D(T, 0, 0, 0, layer, 256, 256, 1, gl.RGBA, gl.UNSIGNED_BYTE, b.data);
        mats.mean.set(b.mean, layer * 3);
        n++;
      }
    }
    if (n && this.running) {
      gl.bindTexture(T, mats.tex);
      gl.generateMipmap(T);
      gl.bindTexture(T, null);
      this.dirty = true;
    }
  }

  /** The pixel-art ground's layers: the scene does not paint them (fastStart). */
  replacedMaterials() {
    return new Set(Object.values(GROUND_LAYER));
  }

  /** Resolves once the ground's layers have loaded (left clear by the scene, fastStart): the first
   *  frame never shows them empty -- and the sprites the first frame asks for, or SPRITE_WAIT_MS
   *  have passed: it shows its figures, towns and props, rather than each popping in as it loads.
   *  (They load while the programs compile, the dressing having asked for them -- `prepare`, asked
   *  again in case the game's map came after; Map3D.ready waits for them before the loop draws, so
   *  what MapView draws meanwhile stays until then.) */
  modeReady() {
    if (this.cfg.fastStart === 0) return Promise.resolve();
    this.prepare();
    const sprites = Promise.race([this.sprites.atlas.whenLoaded(), new Promise((r) => setTimeout(r, SPRITE_WAIT_MS))]);
    return Promise.all([this.groundReady, sprites]);
  }

  /** Resolves once the sprites asked for, the ground and the full terrain have loaded (a capture
   *  waits for it: `terrainReady`). */
  async whenReady() {
    await this.groundReady;
    await this.sprites.atlas.whenLoaded();
    await this.terrainReady();
  }

  dispose() {
    try {
      this.sprites?.dispose();
    } catch {
      /* the context may be gone */
    }
    super.dispose();
  }

  loadCfg() {
    return loadConfig(STORE, DEFAULTS_HD2D, PRESETS_HD2D, { hd2d: LIGHT });
  }

  /** A low bank where the land meets its water (world.js `bank`): the look target's land stands
   *  over its water. */
  worldOpts() {
    return { bank: BANK };
  }

  /** The ground's brush dabs compiled out (shaders.js NO_DABS): the pixel-art ground carries its
   *  own marks, and they showed only as a few flower flecks at the closest zoom, for 0.5 ms a 1080p
   *  frame and about a second of the start on Direct3D (docs/HD2D.md §8). */
  terrainDefines() {
    const d = { NO_DABS: 1 };
    // The lean terrain (config.js terrainLean; read once, when the programs are built): the parts of
    // the ground's painting this mode shows little of, computed more cheaply (shaders.js LEAN_*).
    if ((this.cfg.terrainLean ?? 0) > 0) Object.assign(d, TERRAIN_LEAN);
    // The ground is the stand-in's painting (config.js terrainStandin, TERRAIN_STANDIN; docs/HD2D.md
    // §5b): in play it reads as the full one, and it is half the program.
    if (this.groundIsStandin()) Object.assign(d, TERRAIN_STANDIN);
    // A development switch, one page's: `?r3d={"terrainDefs":"LEAN_FBM=3,LEAN_POOL"}` adds defines.
    for (const kv of String(this.cfg.terrainDefs ?? "").split(",").filter(Boolean)) {
      const [k, v = "1"] = kv.split("=");
      d[k.trim()] = v.trim();
    }
    return d;
  }

  /** The scene packed into one target (pipeline.js `packed`): on Direct3D 11 the scene's programs
   *  then compile at link -- on ANGLE's worker threads, the page live, kept in the browser's program
   *  cache -- instead of at their first draws, one at a time with the browser frozen.
   *  Only there by default: other backends compile at link anyway, and on them the packing only adds
   *  its unpack pass (and on SwiftShader the first complete frame came at 12-13 s packed against
   *  4.3-4.9 s unpacked, its pipelines compiled in the first frame instead of the warm-up).
   *  `?r3d={"packScene":0}` draws the three targets everywhere, `{"packScene":1}` packs everywhere. */
  packedScene() {
    const on = this.cfg.packScene;
    return on === undefined ? /direct3d|d3d1/i.test(this.glName) : on > 0;
  }

  /** Whether the ground is the stand-in's painting (config.js terrainStandin; read once, when the
   *  programs are built). */
  groundIsStandin() {
    return (this.cfg.terrainStandin ?? 1) > 0;
  }

  /** The stand-in ground (config.js TERRAIN_STANDIN, over terrainDefines' own) where the scene is
   *  packed and the ground is the full painting (`?r3d={"terrainStandin":0}`): its programs compile
   *  at link, and the map draws once all but the full terrain have -- on Direct3D about half a
   *  second in, the terrain's own compile (about 1.9 s) going on behind the stand-in (docs/R3D.md
   *  §11.1 "Two stages"). `?r3d={"standin":0}` waits for the full terrain. With the stand-in's
   *  painting as the ground itself (the default) there is nothing to stand in for. */
  standinTerrain() {
    if (this.groundIsStandin()) return null;
    return this.packedScene() && (this.cfg.standin ?? 1) > 0 ? TERRAIN_STANDIN : null;
  }

  /** The value noise from a hash texture (shaders.js NOISE_TEX): the terrain's, water's and props'
   *  noise patterns move, their look stays, and Direct3D compiles them in about half the time
   *  (docs/R3D.md §11.1). `?r3d={"noiseTex":0}` compiles the arithmetic noise. */
  sceneDefines() {
    return this.cfg.noiseTex === 0 ? {} : { NOISE_TEX: 1 };
  }

  /** What the mode compiles out of the props' and the water's programs, to shorten Direct3D's
   *  compile of them at the first frame (docs/R3D.md §11.1, docs/HD2D.md §8). `cfg.lite` 0 compiles
   *  the 3D renderer's programs; `cfg.liteOff` (a comma list of names) leaves those out, `cfg.liteOn`
   *  adds the optional ones (LITE.opt). */
  liteDefines(kind) {
    const L = LITE[kind];
    if (!(this.cfg.lite ?? 1)) return {};
    const list = (k) => String(this.cfg[k] ?? "").split(",").map((s) => s.trim()).filter(Boolean);
    const off = new Set(list("liteOff"));
    const out = {};
    for (const n of [...L.on, ...list("liteOn").filter((n) => L.opt.includes(n))]) if (!off.has(n)) out[n] = 1;
    return out;
  }

  propDefines() {
    return this.liteDefines("prop");
  }

  waterDefines() {
    return this.liteDefines("water");
  }

  riverSplit() {
    return !!this.liteDefines("split").RIVER_SPLIT;
  }

  saveCfg(cfg) {
    saveConfig(cfg, STORE);
  }

  light() {
    return LIGHT;
  }

  presetNames() {
    return Object.keys(PRESETS_HD2D);
  }

  withPreset(name) {
    return withPreset(this.cfg, name, PRESETS_HD2D);
  }

  panelRows() {
    // (Brush dabs: compiled out, terrainDefines.)
    return [...PANEL.filter(([k]) => !["kuwahara", "kuwaharaRadius", "outline", "outlineWidth", "inkLost", "inkColour", "grid", "dabs"].includes(k)), ...PANEL_HD2D];
  }

  panelLabel() {
    return "HD-2D";
  }

  offersLight() {
    return false;
  }

  /** A narrower swing than the 3D renderer's: 42° far out, 36° at the gameplay zoom, 30° close
   *  (with `pitch` 36) -- the settlement sprites, drawn for about 30°, tolerate up to about 45°. */
  pitchAt(zt) {
    // Never so low that the top of the view reaches the horizon (the visible rect would run off
    // to infinity), whatever the panel's pitch and lens.
    return Math.max(this.cfg.fov / 2 + 8, this.cfg.pitch + 6 - zt * 12);
  }

  /** The cards claim the cities, standing improvements and resources from the 3D dressing; the
   *  static cards, the beams and the mood channel are made with it. */
  dressEnv(world) {
    const env = staticEnv(world);
    // How the towns stand out (config.js townScale, townClearing, townBanners; townLight is mostly
    // the cards' shader's).
    env.town = { scale: this.cfg.townScale ?? 1, clearing: this.cfg.townClearing ?? 0, banners: this.cfg.townBanners ?? 0 };
    this.moods = plotMoods(world, this.view.map);
    const st = dressSpriteStatic(world, this.view.map, this.sprites.atlas, env, this.moods.moods);
    env.claims = st.claims;
    this.spriteStatic = st;
    return env;
  }

  /** Lit towns light the ground round them more (config.js townLight). */
  cityLight() {
    return 1 + 2 * (this.cfg.townLight ?? 0);
  }

  /** A change to how the towns are dressed (their size, clearing or light pools) redresses the map. */
  setConfig(cfg) {
    const redress = ["townScale", "townClearing", "townLight", "townBanners"].some((k) => cfg[k] !== this.cfg[k]);
    super.setConfig(cfg);
    if (redress) this.viewSeen = null;
  }

  staticExtras(map, env, emitters) {
    const st = this.spriteStatic;
    this.sprites.set("static", st?.list ?? []);
    this.sprites.set("beams", st?.beams ?? []);
    const { data, moods } = this.moods ?? plotMoods(this.world, map);
    this.scene.tex.setMood(this.world, data);
    for (const e of moodEmitters(this.world, moods)) emitters.push(e);
  }

  /** A group's ring: a thin one in the owner's colour round the whole group (the banner already
   *  says whose it is), gold and filled when it is selected. */
  ringOf(s) {
    return {
      x: s.x + 0.5 + s.spot[0],
      y: s.y + 0.5 - s.spot[1] + 0.02,
      r: 0.26,
      color: s.picked ? "#fff2a8" : s.team,
      rim: s.picked ? "#3a2a08" : "#140e0a",
      width: s.picked ? 0.04 : 0.022,
      fill: s.picked ? 0.16 : 0,
    };
  }

  /** The resources are sprites that read from about 72 px a plot; further out, the HUD's chips. */
  resourceChips() {
    return this.ts < 72;
  }

  dressUnitsOf(world, map) {
    return dressSpriteUnits(world, map, this.sprites.atlas);
  }

  /** The figures as they stand now: a walking one partway, and facing the way it last walked (the
   *  art faces right: a figure that walked west is mirrored). The 3D renderer's figure layer
   *  stays empty. */
  setUnits(list) {
    if (this.scene.layers.units.length) this.scene.setInstances("units", []);
    const out = list.map((it) => (it.yaw !== undefined && it.yaw < -0.35 && !it.banner ? { ...it, flip: true } : it));
    this.sprites.set("units", out);
  }

  /** The frame's inputs to the HD-2D passes: the config in force, the tilt-shift's focus (the
   *  hovered plot, else the selection, else the middle of the view; eased so the band glides), its
   *  strength by zoom (none far out, where the whole map must read), the mood texture. */
  beforeFrame(cam, fr) {
    this.sprites.refresh();
    const v = this.view;
    const o = v.overlay;
    const p = o.hover ?? o.sel ?? null;
    let want = 0.5;
    if (p) {
      // Only a mark in view moves the band (a selection scrolled away leaves it in the middle).
      const s = this.screenOf(p.x, p.y);
      if (s.y >= 0 && s.y <= v.vh && s.x >= 0 && s.x <= v.vw) want = clamp(1 - s.y / Math.max(1, v.vh), 0.25, 0.75);
    }
    // (A scripted film sets `focusClock`, its frame's time: tools/r3d-lab.mjs --capture.)
    const now = this.focusClock ?? performance.now();
    const dt = this.focusT ? Math.min(0.2, (now - this.focusT) / 1000) : 1;
    this.focusT = now;
    this.focus = this.focus === null || v.reduceMotion() ? want : this.focus + (want - this.focus) * (1 - Math.exp(-dt * 6));
    if (Math.abs(this.focus - want) > 0.002) this.dirty = true;
    const ext = this.pipeline.ext;
    ext.frame = {
      cfg: this.qualityCfg(),
      focus: this.focus,
      tilt: smoothstep(48, 90, this.ts),
      beamZoom: smoothstep(48, 96, this.ts),
      farDepth: cam.dist * 2.2,
      mood: this.scene.tex.mood,
      mapSize: [this.world.w, this.world.h],
      wrapX: this.world.wrapX ? 1 : 0,
    };
    fr.uniforms.uExposure = fr.exposure;
  }
}

export { MapHd2d as Map3D };
