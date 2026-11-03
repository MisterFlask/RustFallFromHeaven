// The HD-2D mode's sprites on the GPU (docs/HD2D.md): the isometric renderer's pixel art
// (assets/app/iso/*.png, indexed by iso/sprites.js) and the mode's own prop cards (hd2d/art/*.png,
// indexed by hd2d/art/index.js: trees, bushes, rocks, crystals) in one texture array, a sprite a layer, and the
// cards that stand them in the 3D world, built from the records hd2d/dress.js makes. Drawn by the
// HD-2D passes (hd2d/passes.js) in the scene, the sun's shadow map and the overlays.
//
// A unit sprite with an anim-sprint idle (IDLE) is stored as its idle strip instead: the four
// frames, two to a layer, and its card plays them (the cards' vertex shader picks the frame).

import { SPRITES } from "../../iso/sprites.js";
import { ownerBand } from "../../iso.js";
import { CARDS } from "./art/index.js";
import { ATTR } from "../gl.js";

/** Texels a side of each layer: every unit, building, feature and resource sprite fits (the 2x2
 *  city, 384 wide, is drawn at two thirds). */
const LAYER = 256;
const LEVELS = 9;
/** Floats a card instance: iPosRot, iScale, iTint, iTeam, aExtra. */
const FLOATS = 20;

/** The unit sprites that breathe: sprite key -> its idle strip (`app/anim/<name>_idle.png`, a copy
 *  of anim-sprint's `final_<name>/sheet_idle.png`: four 160 x 128 frames side by side, 250 ms each,
 *  frame 0 the sprite itself, its feet at (80, 126); anim-sprint/ANIMATING_UNITS.md). */
export const IDLE = {
  unit_axeman: "axeman",
  unit_champion: "champion",
  unit_donal: "donal",
  unit_goblin: "goblin",
  unit_guybrush: "guybrush",
  unit_hyborem: "hyborem",
  unit_lizardman: "lizardman",
  unit_losha: "losha",
  unit_maros: "maros",
  unit_ogre_embers: "ogre",
  unit_paladin: "paladin",
  unit_phalanx: "phalanx",
  unit_skeleton: "skeleton",
  unit_swordsman: "swordsman",
  unit_warrior: "warrior",
};
/** An idle frame (art px) and the frames of a strip. The cards' shader plays them (SPRITE_VS). */
export const IDLE_FRAME = { w: 160, h: 128, n: 4 };
/** The card flag of an idle strip (iScale.w; SPRITE_VS). */
export const IDLE_FLAG = 32;

/** A figure's card over the idle frame instead of its sprite (`info`, the sprite's size): the frame
 *  is wider and taller, the sprite in it centred across and standing on its foot, so the figure
 *  stands where its sprite would. `{ w, h, anchor }` for a card of `w` x `h` world units (record
 *  `r` is the sprite's card); flipped, the anchor mirrors with the art. */
export function idleCard(r, info) {
  const px = r.w / info.w;
  // (The recipe places the sprite at x = floor(80 - w / 2): an odd width is half a pixel left.)
  const mid = Math.floor(IDLE_FRAME.w / 2 - info.w / 2) + info.w / 2;
  const anchor = mid / IDLE_FRAME.w;
  return { w: IDLE_FRAME.w * px, h: IDLE_FRAME.h * px, anchor: r.flip ? 1 - anchor : anchor };
}

function canvas(w, h) {
  if (typeof OffscreenCanvas !== "undefined") return new OffscreenCanvas(w, h);
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return c;
}

/** Image `im` with its owner-colour band in `css`, on a canvas. */
function recoloured(im, css) {
  const c = canvas(im.width, im.height);
  const g = c.getContext("2d", { willReadFrequently: true });
  g.drawImage(im, 0, 0);
  const d = g.getImageData(0, 0, c.width, c.height);
  ownerBand(d.data, css);
  g.putImageData(d, 0, 0);
  return c;
}

/** The sprites in a texture array (premultiplied colour, mipmapped), each loaded on first use.
 *  `direct` (config `fastStart`, the default): each sprite goes to the GPU from where it is on the
 *  CPU -- the decoded image, or a canvas in memory (a recoloured or scaled one) -- into its corner of
 *  the layer (the rest stays clear: a new array is zeroed). Otherwise (the original) each is drawn
 *  into a 256-square 2D canvas first: Chrome keeps such a canvas on the GPU, so uploading it reads
 *  it back, a round trip to the GPU process that waits for all the work queued before it -- a frame
 *  being drawn, a shader compiling at its first draw on Direct3D (docs/R3D.md §11.1). */
export class SpriteAtlas {
  constructor(gl, onload, direct = true) {
    this.gl = gl;
    this.onload = onload;
    this.direct = direct;
    /** key -> { layer, layers, ready, failed, w, h, u, v, idle } (`idle`: the layers hold its
     *  idle strip, two frames each, the first above). */
    this.entries = new Map();
    /** The next free layer (an idle strip takes two). */
    this.next = 0;
    /** layer -> the 256-square canvas uploaded there (kept for a re-upload when the array grows). */
    this.sources = [];
    this.drawn = new Map();
    /** An owner's variant (`key@css`, `owned`) -> { key, css }. */
    this.base = new Map();
    this.pending = 0;
    this.waiters = [];
    this.cap = 0;
    this.tex = null;
    this.mipsDirty = false;
    /** Start-up counters (performance marks). */
    this.stats = { loads: 0, uploads: 0, allocs: 0, mips: 0, recolours: 0 };
    this.alloc(64);
  }

  alloc(n) {
    const gl = this.gl;
    const old = this.tex;
    this.tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.tex);
    gl.texStorage3D(gl.TEXTURE_2D_ARRAY, LEVELS, gl.RGBA8, LAYER, LAYER, n);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.cap = n;
    this.stats && this.stats.allocs++;
    this.sources.forEach((c, layer) => c && this.upload(layer, c));
    if (old) gl.deleteTexture(old);
    this.mipsDirty = true;
  }

  has(key) {
    return Object.hasOwn(SPRITES, key) || Object.hasOwn(CARDS, key) || this.drawn.has(key) || this.base.has(key);
  }

  /** Sprite `key` in its owner's colour `css`: the art's owner-colour band (iso.js ownerBand, its
   *  stock saturated blue) turned to `css` as it loads. Returns the variant's key. */
  owned(key, css) {
    const id = `${key}@${css}`;
    if (!this.base.has(id)) this.base.set(id, { key, css });
    return id;
  }

  /** The sprite's native size and opaque box (art px). */
  info(key) {
    const b = this.base.get(key);
    if (b) return this.info(b.key);
    const s = SPRITES[key] ?? CARDS[key];
    if (s) return s;
    const c = this.drawn.get(key);
    return c ? { w: c.width, h: c.height, box: [0, 0, c.width, c.height] } : null;
  }

  /** A sprite drawn by code (a banner) under `key`. */
  put(key, c) {
    if (this.drawn.has(key)) return;
    this.drawn.set(key, c);
  }

  /** The idle strip of sprite `key` (or of the sprite an owner's variant recolours), or null. */
  idle(key) {
    const b = this.base.get(key);
    return IDLE[b ? b.key : key] ?? null;
  }

  /** The layers sprite `key` takes: two for an idle strip's four frames, else one. */
  layersOf(key) {
    return this.idle(key) ? IDLE_FRAME.n / 2 : 1;
  }

  /** Room for `n` layers in all: the array grows once to the next power of two (the original
   *  doubled as the sprites were asked for, each time a new array and every loaded sprite again). */
  reserve(n) {
    if (n <= this.cap) return;
    const max = this.gl.getParameter(this.gl.MAX_ARRAY_TEXTURE_LAYERS);
    let cap = this.cap;
    while (cap < n && cap < max) cap *= 2;
    this.alloc(Math.min(cap, max));
  }

  /** Sprite `key` in the array: `{ layer, u, v, idle }` (its extent in the layer; `idle`: its idle
   *  strip's frames, IDLE_FRAME), or null while it loads (it starts loading) or when there is no
   *  such sprite. */
  get(key) {
    let e = this.entries.get(key);
    if (!e) {
      const info = this.info(key);
      if (!info) return null;
      const layers = this.layersOf(key);
      const layer = this.next;
      if (layer + layers > this.cap) {
        // The array doubles, up to what the GPU allows (WebGL2 guarantees 256 layers).
        const max = this.gl.getParameter(this.gl.MAX_ARRAY_TEXTURE_LAYERS);
        if (layer + layers > max) {
          if (!this.full) console.warn(`hd2d: the sprite atlas is full (${max} layers); ${key} and later sprites are not drawn`);
          this.full = true;
          return null;
        }
        this.alloc(Math.min(this.cap * 2, max));
      }
      this.next += layers;
      e = { layer, layers, ready: false, failed: false, idle: false };
      this.entries.set(key, e);
      const drawn = this.drawn.get(key);
      if (drawn) this.store(e, drawn);
      else if (layers > 1) this.loadIdle(key, e);
      else this.load(key, e);
    }
    return e.ready ? e : null;
  }

  load(key, e) {
    if (this.pending === 0) performance.mark("hd2d:sprites-busy");
    this.pending++;
    this.stats.loads++;
    if (this.base.has(key)) this.stats.recolours++;
    const b = this.base.get(key);
    const src = b ? b.key : key;
    const im = new Image();
    im.src = new URL(Object.hasOwn(CARDS, src) ? `./art/${src}.png` : `../../iso/${src}.png`, import.meta.url).href;
    im.decode()
      .then(() => this.store(e, b ? recoloured(im, b.css) : im))
      .catch(() => {
        e.failed = true;
        console.warn(`hd2d: sprite ${key} did not load`);
      })
      .finally(() => {
        this.pending--;
        this.changed();
      });
  }

  /** Sprite `key`'s idle strip into its two layers, frames 0 and 1 in the first (one above the
   *  other: 160 x 256), 2 and 3 in the second. A strip that does not load leaves the sprite. */
  loadIdle(key, e) {
    if (this.pending === 0) performance.mark("hd2d:sprites-busy");
    this.pending++;
    this.stats.loads++;
    const b = this.base.get(key);
    const im = new Image();
    im.src = new URL(`../../anim/${this.idle(key)}_idle.png`, import.meta.url).href;
    im.decode()
      .then(() => {
        const { w, h, n } = IDLE_FRAME;
        for (let l = 0; l < n / 2; l++) {
          // (In memory: willReadFrequently keeps Chrome's 2D canvas off the GPU.)
          const c = canvas(w, 2 * h);
          const g = c.getContext("2d", { willReadFrequently: true });
          g.drawImage(im, 2 * l * w, 0, w, h, 0, 0, w, h);
          g.drawImage(im, (2 * l + 1) * w, 0, w, h, 0, h, w, h);
          const src = b ? recoloured(c, b.css) : c;
          this.sources[e.layer + l] = src;
          this.upload(e.layer + l, src);
        }
        if (b) this.stats.recolours++;
        Object.assign(e, { ready: true, idle: true, w, h, u: w / LAYER, v: h / LAYER });
        this.mipsDirty = true;
      })
      .catch(() => {
        console.warn(`hd2d: the idle strip of ${key} did not load; it stands still`);
        this.load(key, e);
      })
      .finally(() => {
        this.pending--;
        this.changed();
      });
  }

  /** The image into its layer: at the top-left, scaled down only when it does not fit. */
  store(e, im) {
    const w = im.width, h = im.height;
    const k = Math.min(1, LAYER / w, LAYER / h);
    if (this.direct) {
      let src = im;
      if (k < 1) {
        // (A canvas in memory: willReadFrequently keeps Chrome's 2D canvas off the GPU.)
        src = canvas(Math.round(w * k), Math.round(h * k));
        const g = src.getContext("2d", { willReadFrequently: true });
        g.imageSmoothingEnabled = true;
        g.drawImage(im, 0, 0, src.width, src.height);
      }
      this.sources[e.layer] = src;
      this.upload(e.layer, src);
      Object.assign(e, { ready: true, w, h, u: (w * k) / LAYER, v: (h * k) / LAYER });
      this.mipsDirty = true;
      return;
    }
    const c = canvas(LAYER, LAYER);
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = k < 1;
    g.drawImage(im, 0, 0, Math.round(w * k), Math.round(h * k));
    this.sources[e.layer] = c;
    this.upload(e.layer, c);
    Object.assign(e, { ready: true, w, h, u: (w * k) / LAYER, v: (h * k) / LAYER });
    this.mipsDirty = true;
  }

  upload(layer, c) {
    const gl = this.gl;
    this.stats && this.stats.uploads++;
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.tex);
    // A 2D canvas holds premultiplied colour: kept so, the mipmaps average colour and coverage
    // together (no dark fringe where a sprite meets its transparent surround).
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
    // (A direct source covers its own corner of the layer; the original's canvases the whole.)
    const w = c.naturalWidth || c.width, h = c.naturalHeight || c.height;
    gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, layer, w, h, 1, gl.RGBA, gl.UNSIGNED_BYTE, c);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  }

  changed() {
    if (this.pending === 0) performance.mark("hd2d:sprites-idle", { detail: { ...this.stats, layers: this.next, cap: this.cap } });
    if (this.pending === 0) for (const f of this.waiters.splice(0)) f();
    this.onload();
  }

  /** Resolves once every sprite asked for so far has loaded (or failed). */
  whenLoaded() {
    return this.pending === 0 ? Promise.resolve() : new Promise((r) => this.waiters.push(r));
  }

  /** The mipmaps, once before drawing after an upload. */
  flush() {
    if (!this.mipsDirty) return;
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.tex);
    gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
    this.stats.mips++;
    this.mipsDirty = false;
  }

  dispose() {
    this.gl.deleteTexture(this.tex);
  }
}

/** The cards: sets of records (`static`: cities, buildings, resources and their banners; `units`:
 *  the figures and banners of the stacks; `beams`: light columns) packed into instance buffers.
 *  A record whose sprite is still loading is left out until it has loaded (`stale`). */
export class SpriteLayer {
  constructor(gl, onload, direct = true) {
    this.gl = gl;
    this.atlas = new SpriteAtlas(gl, () => {
      this.stale = true;
      onload();
    }, direct);
    this.records = { static: [], units: [], beams: [] };
    this.bufs = {};
    this.stale = false;
    this.quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-0.5, 0, 0, 0.5, 0, 0, 0.5, 1, 0, -0.5, 0, 0, 0.5, 1, 0, -0.5, 1, 0]), gl.STATIC_DRAW);
  }

  set(name, records) {
    this.records[name] = records;
    this.build(name);
  }

  /** Re-packs the sets whose sprites have loaded since they were built. */
  refresh() {
    if (!this.stale) return;
    this.stale = false;
    for (const name of Object.keys(this.records)) this.build(name);
  }

  build(name) {
    const gl = this.gl;
    const list = this.records[name];
    if (this.atlas.direct) {
      // The array sized once for every sprite these records ask for.
      const fresh = new Set();
      for (const r of list) if (r.key && !this.atlas.entries.has(r.key)) fresh.add(r.key);
      let layers = 0;
      for (const k of fresh) layers += this.atlas.layersOf(k);
      if (layers) this.atlas.reserve(this.atlas.next + layers);
    }
    const arr = new Float32Array(list.length * FLOATS);
    let n = 0;
    for (const r of list) {
      let layer = 0, u = 1, v = 1;
      let { w, h } = r, anchor = r.anchor ?? 0.5, idle = false;
      if (r.key) {
        const e = this.atlas.get(r.key);
        if (!e) continue;
        [layer, u, v] = [e.layer, e.u, e.v];
        if (e.idle) {
          ({ w, h, anchor } = idleCard(r, this.atlas.info(r.key)));
          idle = true;
        }
      }
      const flags = (r.flip ? 1 : 0) | (r.banner ? 2 : 0) | (r.shadow ? 4 : 0) | (r.fade ? 8 : 0) | (r.town ? 16 : 0) | (idle ? IDLE_FLAG : 0);
      const team = r.team ?? [1, 1, 1];
      const vis = r.vis ? [r.vis[0], r.vis[1], 1] : [0, 0, 0];
      arr.set([r.x, r.y, r.z, layer, w, h, r.phase ?? 0, flags, u, v, r.id ?? 0, r.glow ?? 0, team[0], team[1], team[2], r.bias ?? 0, anchor, ...vis], n * FLOATS);
      n++;
    }
    let b = this.bufs[name];
    if (!b) {
      b = this.bufs[name] = { vao: gl.createVertexArray(), ibuf: gl.createBuffer(), n: 0 };
      gl.bindVertexArray(b.vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
      gl.enableVertexAttribArray(ATTR.aPos);
      gl.vertexAttribPointer(ATTR.aPos, 3, gl.FLOAT, false, 12, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, b.ibuf);
      ["iPosRot", "iScale", "iTint", "iTeam", "aExtra"].forEach((a, k) => {
        gl.enableVertexAttribArray(ATTR[a]);
        gl.vertexAttribPointer(ATTR[a], 4, gl.FLOAT, false, FLOATS * 4, k * 16);
        gl.vertexAttribDivisor(ATTR[a], 1);
      });
      gl.bindVertexArray(null);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, b.ibuf);
    gl.bufferData(gl.ARRAY_BUFFER, arr.subarray(0, n * FLOATS), gl.DYNAMIC_DRAW);
    b.n = n;
    // Shadow casters and contact shadows are the figures and the props that stand up (trees,
    // rocks): a building's card carries its painted ground, which would cast a slab of shadow.
    if (name === "units" || name === "static") {
      const casts = [...this.records.units, ...this.records.static].filter((r) => r.shadow);
      this.set("casters", casts);
    }
  }

  /** Draws set `name` (each copy of the map in `copies`) with `prog`, which is in use. */
  draw(prog, name, copies) {
    const b = this.bufs[name];
    if (!b || !b.n) return;
    const gl = this.gl;
    gl.bindVertexArray(b.vao);
    for (const copy of copies) {
      prog.set({ uCopyX: copy });
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, b.n);
    }
    gl.bindVertexArray(null);
  }

  dispose() {
    const gl = this.gl;
    for (const b of Object.values(this.bufs)) {
      gl.deleteVertexArray(b.vao);
      gl.deleteBuffer(b.ibuf);
    }
    gl.deleteBuffer(this.quad);
    this.atlas.dispose();
  }
}
