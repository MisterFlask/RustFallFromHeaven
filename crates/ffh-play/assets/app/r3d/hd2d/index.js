// The HD-2D map (docs/HD2D.md): the isometric renderer's pixel-art sprites standing in the 3D
// renderer's lit world, as in Octopath Traveler or Triangle Strategy -- a perspective camera tilted
// toward the horizon, figures, towns and props as flat cards facing it, the light painted in the
// sprites matched by a sun from the upper left, light shafts and glow, a mood for each region,
// glowing borders, and a tilt-shift blur. Chosen with `?renderer=hd2d` or Tools -> Options while
// look.js EXPERIMENTAL_HD2D is on; this module is loaded on demand (map.js useR3d), so the 3D
// renderer never loads it.
//
// It is the 3D renderer (../map3d.js Map3D) with its mode hooks set: its own look (hd2d/config.js),
// the cards' dressing (hd2d/dress.js) in place of the modelled figures, cities, buildings and
// resource markers, the per-plot mood channel (hd2d/mood.js), and its passes in the pipeline
// (hd2d/passes.js). Everything else -- the ground, water, trees, picking, camera, HUD, lab and
// tools -- is the 3D renderer's.

import { Map3D } from "../map3d.js";
import { loadConfig, saveConfig, withPreset, PANEL } from "../config.js";
import { clamp, smoothstep } from "../math.js";
import { STORE, LIGHT, DEFAULTS_HD2D, PRESETS_HD2D, PANEL_HD2D } from "./config.js";
import { SpriteLayer } from "./sprites.js";
import { Hd2dPasses } from "./passes.js";
import { dressSpriteStatic, dressSpriteUnits } from "./dress.js";
import { plotMoods, moodEmitters } from "./mood.js";
import { staticEnv } from "../dress.js";
import { MAT } from "../materials.js";
import { GROUNDS } from "./art/index.js";

/** The 3D renderer's gameplay zoom: on the full-bleed map (the isometric screen's floating
 *  panels) it shows about fifteen plots across, as the look target does, and its figures are about
 *  the target's share of the screen. */
export const DEFAULT_ZOOM = 96;

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
      this.sprites = new SpriteLayer(this.gl, () => {
        this.dirty = true;
      });
      this.pipeline.ext = new Hd2dPasses(this.gl, this.pipeline, this.sprites);
    } catch (e) {
      this.dispose();
      throw e;
    }
    this.groundReady = this.loadGround();
  }

  /** The pixel-art ground into the 3D renderer's material layers (this scene's own: the 3D
   *  renderer's are untouched), with their means; resolves when done (a layer that fails to load
   *  keeps the painted one). */
  async loadGround() {
    const gl = this.gl, mats = this.scene.materials;
    const T = gl.TEXTURE_2D_ARRAY;
    let n = 0;
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
          n++;
        } catch (e) {
          console.warn(`hd2d: ground ${name} did not load`, e);
        }
      }),
    );
    if (n && this.running) {
      gl.bindTexture(T, mats.tex);
      gl.generateMipmap(T);
      gl.bindTexture(T, null);
      this.dirty = true;
    }
  }

  /** Resolves once the sprites asked for and the ground have loaded (a capture waits for it). */
  async whenReady() {
    await this.groundReady;
    await this.sprites.atlas.whenLoaded();
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
    return [...PANEL.filter(([k]) => !["kuwahara", "kuwaharaRadius", "outline", "outlineWidth", "inkLost", "inkColour", "grid"].includes(k)), ...PANEL_HD2D];
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
    this.moods = plotMoods(world, this.view.map);
    const st = dressSpriteStatic(world, this.view.map, this.sprites.atlas, env, this.moods.moods);
    env.claims = st.claims;
    this.spriteStatic = st;
    return env;
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
