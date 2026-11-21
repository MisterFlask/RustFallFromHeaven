// Terrain meshes and the map data textures of the 3D renderer (docs/R3D.md "Terrain").
//
// The ground is one heightfield surface cut into CHUNK x CHUNK-plot meshes of RES vertices per plot
// (built once, rebuilt only where the map changed: a revealed plot, a terraformed one). Everything
// that varies per plot (ground type, feature, kind, visibility, owner) goes to the shaders as small
// textures, one texel per plot, so the ground's painting reads the logical grid directly and stays
// world-anchored however the camera moves.

import { GROUND, KIND } from "./world.js";
import { texture, upload, Mesh } from "./gl.js";

export const CHUNK = 16;
/** Vertices per plot along each axis at full detail; distant zooms use LOW_RES. */
export const RES = 6;
export const LOW_RES = 2;
/** Height texture samples per plot (water depth, shore foam). */
export const HRES = 4;
/** The height texture's channels: r the ground's height, g the shore field (`World.shoreAt`: the
 *  ground before any bank or face, continuous across the shore -- what the water measures its
 *  depth by where the land is a slab, as a face's height does not interpolate). */

export const TERRAIN_LAYOUT = { aPos: 3, aNormal: 3 };

/** The heights of a chunk's grid (with a one-sample border for normals): Float32Array of
 *  (n + 3)^2, n = CHUNK * res. */
function chunkHeights(world, cx, cy, res) {
  const n = CHUNK * res;
  const N = n + 3;
  const hs = new Float32Array(N * N);
  const x0 = cx * CHUNK;
  const y0 = cy * CHUNK;
  for (let j = 0; j < N; j++) {
    const my = y0 + (j - 1) / res;
    for (let i = 0; i < N; i++) {
      const mx = x0 + (i - 1) / res;
      hs[j * N + i] = world.heightAt(mx, my);
    }
  }
  return hs;
}

/** The mesh data of chunk (cx, cy) of a world whose ground steps (`World.cliff`: at the shore, and
 *  at the hills' scarps): the grid as `buildChunk` makes it, but a grid square the shore or a scarp
 *  runs through is cut along it -- each of its two triangles clipped by the shore field and then by
 *  the scarp field (`World.shore`, `World.scarp`: each side of their zero is a region: the floor,
 *  the land, a crown) -- into each region's part at that region's height, with a sheer face between
 *  two parts from the upper one's rim down to the lower one's foot. A grid alone cannot draw a
 *  face: its squares would ramp from foot to top over a sixth of a plot, in sawteeth along the
 *  grid. A face's normal is level (y 0: the terrain shader knows a face by that), points out over
 *  the lower ground, and is 2 + its rim's height long (the shader lays the face's art down from the
 *  rim). */
function buildCliffChunk(world, cx, cy, res) {
  const n = CHUNK * res;
  const N = n + 3;
  const x0 = cx * CHUNK, y0 = cy * CHUNK;
  const step = 1 / res;
  // The ground on the chunk's grid, with a one-sample border for the normals: its height, the two
  // fields and the region each vertex stands in (0 the floor, 1 the land, 2 a crown).
  const hs = new Float32Array(N * N), ss = new Float32Array(N * N), ts = new Float32Array(N * N);
  const rg = new Uint8Array(N * N);
  for (let j = 0; j < N; j++) {
    for (let i = 0; i < N; i++) {
      const k = j * N + i;
      hs[k] = world.heightAt(x0 + (i - 1) / res, y0 + (j - 1) / res);
      ss[k] = world.shore;
      ts[k] = world.scarp;
      rg[k] = world.shore > 0 ? (world.scarp > 0 ? 2 : 1) : 0;
    }
  }
  const pw = Math.min(CHUNK, world.w - cx * CHUNK);
  const ph = CHUNK;
  const vw = pw * res + 1, vh = ph * res + 1;
  const data = [];
  let lo = Infinity, hi = -Infinity;
  const push = (x, y, h, nx, ny, nz) => {
    data.push(x, h, -y, nx, ny, nz);
    if (h < lo) lo = h;
    if (h > hi) hi = h;
    return data.length / 6 - 1;
  };
  // A slope from the neighbours in this vertex's own region (across a step the height jumps).
  const slope = (k, d) => {
    const m = rg[k - d] === rg[k], p = rg[k + d] === rg[k];
    if (m && p) return (hs[k + d] - hs[k - d]) / (2 * step);
    if (p) return (hs[k + d] - hs[k]) / step;
    if (m) return (hs[k] - hs[k - d]) / step;
    return 0;
  };
  for (let j = 0; j < vh; j++) {
    for (let i = 0; i < vw; i++) {
      const k = (j + 1) * N + i + 1;
      const nx = -slope(k, 1), nz = slope(k, N);
      const l = Math.hypot(nx, 1, nz);
      // (x0 + i / res: the same number in the chunks on both sides of a shared edge.)
      push(x0 + i / res, y0 + j / res, hs[k], nx / l, 1 / l, nz / l);
    }
  }
  const idx = [];
  // A corner of the grid as a polygon's vertex: its place, the fields there, its mesh vertex.
  const corner = (i, j) => {
    const k = (j + 1) * N + i + 1;
    return { x: x0 + i / res, y: y0 + j / res, s: ss[k], t: ts[k], v: j * vw + i, k };
  };
  // A point between two vertices where a field (`key`: "s" the shore's, "t" the scarp's) crosses
  // zero: found on the field itself, from the lower of the two (whichever way the caller walks the
  // edge, in this chunk or the next: both find the same point), with the heights the ground has
  // there in each region.
  const found = new Map();
  const cross = (p, q, key) => {
    if (q.y < p.y || (q.y === p.y && q.x < p.x)) [p, q] = [q, p];
    const id = p.k !== undefined && q.k !== undefined ? (p.k * N * N + q.k) * 2 + (key === "s" ? 0 : 1) : -1;
    let c = id >= 0 ? found.get(id) : undefined;
    if (c) return c;
    // (False position, the end that stays put halved each time it does -- the Illinois step: a
    // field bent between the two ends is still closed in on. More steps on the coarse grid, whose
    // edges are half a plot long.)
    let fa = p[key], fb = q[key];
    let ta = 0, tb = 1, t = fa / (fa - fb), kept = 0;
    for (let it = res >= RES ? (key === "s" ? 3 : 2) : 8; ; it--) {
      const x = p.x + (q.x - p.x) * t, y = p.y + (q.y - p.y) * t;
      world.heightAt(x, y);
      c = { x, y, s: world.shore, t: world.scarp, v: -1, h: [world.hFloor, world.hLand, world.hCrown] };
      const fv = c[key];
      if (it <= 0 || Math.abs(fv) < 1e-5) break;
      if (fv > 0 === fa > 0) {
        ta = t;
        fa = fv;
        if (kept === 1) fb *= 0.5;
        kept = 1;
      } else {
        tb = t;
        fb = fv;
        if (kept === 2) fa *= 0.5;
        kept = 2;
      }
      t = ta + ((tb - ta) * fa) / (fa - fb);
    }
    if (id >= 0) found.set(id, c);
    return c;
  };
  // A convex polygon cut by a field's zero: its part where the field is positive, the rest, and
  // the cut's two ends (null where there is none).
  const split = (poly, key) => {
    const pos = [], neg = [], chord = [];
    for (let e = 0; e < poly.length; e++) {
      const p = poly[e], q = poly[(e + 1) % poly.length];
      const pp = p[key] > 0;
      (pp ? pos : neg).push(p);
      if (pp !== q[key] > 0) {
        const c = cross(p, q, key);
        pos.push(c);
        neg.push(c);
        chord.push(c);
      }
    }
    return [pos.length >= 3 ? pos : null, neg.length >= 3 ? neg : null, chord.length === 2 ? chord : null];
  };
  // A region's part of a triangle: a fan. A vertex on a cut takes this region's height there, and
  // the normal of the part's first grid vertex (the ground is smooth on its own side of a step).
  const part = (poly, r) => {
    if (!poly) return;
    const g = poly.find((p) => p.v >= 0);
    const o = g ? g.v * 6 + 3 : -1;
    const ids = poly.map((p) => (p.v >= 0 ? p.v : push(p.x, p.y, p.h[r], o < 0 ? 0 : data[o], o < 0 ? 1 : data[o + 1], o < 0 ? 0 : data[o + 2])));
    for (let q = 1; q + 1 < ids.length; q++) idx.push(ids[0], ids[q], ids[q + 1]);
  };
  // A face along a cut from a to b: from region `up`'s height down to region `down`'s, looking out
  // over the lower ground (`toward`: a point of it).
  const face = (a, b, up, down, toward) => {
    const ta = a.h[up], tb = b.h[up], fa = a.h[down], fb = b.h[down];
    if (Math.max(ta - fa, tb - fb) < 1e-3) return;
    let nx = a.y - b.y, ny = b.x - a.x;
    const l = Math.hypot(nx, ny);
    if (l < 1e-9) return;
    const sgn = nx * (toward.x - a.x) + ny * (toward.y - a.y) < 0 ? -1 : 1;
    nx *= sgn / l;
    ny *= sgn / l;
    const la = 2 + ta, lb = 2 + tb;
    const v0 = push(a.x, a.y, ta, nx * la, 0, -ny * la), v1 = push(b.x, b.y, tb, nx * lb, 0, -ny * lb);
    const v2 = push(b.x, b.y, fb, nx * lb, 0, -ny * lb), v3 = push(a.x, a.y, fa, nx * la, 0, -ny * la);
    idx.push(v0, v1, v2, v0, v2, v3);
  };
  // A vertex of `poly` off the cut (a, b), for the side a face looks to.
  const off = (poly, a, b) => poly.find((p) => p.v >= 0) ?? poly.find((p) => p !== a && p !== b);
  const tri = (p0, p1, p2) => {
    const [land, floor, shore] = split([p0, p1, p2], "s");
    const [crown, low, scarp] = land ? split(land, "t") : [null, null, null];
    part(floor, 0);
    part(low, 1);
    part(crown, 2);
    if (shore && floor) {
      // The shore's face: under a crown from the crown's height, else from the land's; where a
      // scarp runs into the shore here, each on its own stretch.
      const [a, b] = shore;
      const ra = a.t > 0 ? 2 : 1, rb = b.t > 0 ? 2 : 1;
      const to = off(floor, a, b);
      if (ra === rb) face(a, b, ra, 0, to);
      else {
        // (Its foot there on the floor's edge, which runs straight from a to b.)
        const m0 = cross(a, b, "t");
        const u = Math.hypot(m0.x - a.x, m0.y - a.y) / (Math.hypot(b.x - a.x, b.y - a.y) || 1);
        const m = { ...m0, h: [a.h[0] + (b.h[0] - a.h[0]) * u, m0.h[1], m0.h[2]] };
        face(a, m, ra, 0, to);
        face(m, b, rb, 0, to);
      }
    }
    if (scarp && low) face(scarp[0], scarp[1], 2, 1, off(low, scarp[0], scarp[1]));
  };
  for (let j = 0; j + 1 < vh; j++) {
    for (let i = 0; i + 1 < vw; i++) {
      const a = j * vw + i, b = a + 1, c = a + vw, d = c + 1;
      const k = (j + 1) * N + i + 1;
      if (rg[k] === rg[k + 1] && rg[k] === rg[k + N] && rg[k] === rg[k + N + 1]) {
        // Split each quad along the diagonal that follows the ground's fold (fewer sawteeth).
        const ha = hs[k], hb = hs[k + 1], hc = hs[k + N], hd = hs[k + N + 1];
        if (Math.abs(ha - hd) < Math.abs(hb - hc)) idx.push(a, d, b, a, c, d);
        else idx.push(a, c, b, b, c, d);
        continue;
      }
      const pa = corner(i, j), pb = corner(i + 1, j), pc = corner(i, j + 1), pd = corner(i + 1, j + 1);
      tri(pa, pd, pb);
      tri(pa, pc, pd);
    }
  }
  return { data: new Float32Array(data), idx: new Uint32Array(idx), x0, y0, pw, ph, lo, hi };
}

/** The mesh data of chunk (cx, cy): interleaved position + normal, and indices. */
export function buildChunk(world, cx, cy, res = RES) {
  if (world.cliff > 0) return buildCliffChunk(world, cx, cy, res);
  const n = CHUNK * res;
  const N = n + 3;
  const hs = chunkHeights(world, cx, cy, res);
  // Chunks at the map's east end may hold fewer plots (no parchment chunk is drawn past the east
  // edge, so there is no gap there).
  const pw = Math.min(CHUNK, world.w - cx * CHUNK);
  // Every chunk is a full CHUNK of rows tall (F15). The last in-map chunk used to stop at row
  // world.h while the north parchment chunk began at ceil(h/CHUNK)*CHUNK, leaving an unmeshed band
  // (12 rows when h = 52). A full-height chunk runs through to the next chunk boundary, meeting the
  // parchment with no gap; its off-map rows read as parchment (heightAt and the textures default
  // there), and chunkVisible already culls each chunk as CHUNK rows tall.
  const ph = CHUNK;
  const vw = pw * res + 1;
  const vh = ph * res + 1;
  const data = new Float32Array(vw * vh * 6);
  const step = 1 / res;
  let lo = Infinity, hi = -Infinity;
  let o = 0;
  for (let j = 0; j < vh; j++) {
    for (let i = 0; i < vw; i++) {
      const h = hs[(j + 1) * N + i + 1];
      const hx = hs[(j + 1) * N + i + 2] - hs[(j + 1) * N + i];
      const hy = hs[(j + 2) * N + i + 1] - hs[j * N + i + 1];
      const nx = -hx / (2 * step);
      const nz = hy / (2 * step);
      const l = Math.hypot(nx, 1, nz);
      data[o++] = cx * CHUNK + i * step;
      data[o++] = h;
      data[o++] = -(cy * CHUNK + j * step);
      data[o++] = nx / l;
      data[o++] = 1 / l;
      data[o++] = nz / l;
      lo = Math.min(lo, h);
      hi = Math.max(hi, h);
    }
  }
  const idx = new Uint32Array((vw - 1) * (vh - 1) * 6);
  let k = 0;
  for (let j = 0; j + 1 < vh; j++) {
    for (let i = 0; i + 1 < vw; i++) {
      const a = j * vw + i, b = a + 1, c = a + vw, d = c + 1;
      // Split each quad along the diagonal that follows the ground's fold (fewer sawteeth).
      const ha = data[a * 6 + 1], hb = data[b * 6 + 1], hc = data[c * 6 + 1], hd = data[d * 6 + 1];
      if (Math.abs(ha - hd) < Math.abs(hb - hc)) {
        idx[k++] = a; idx[k++] = d; idx[k++] = b;
        idx[k++] = a; idx[k++] = c; idx[k++] = d;
      } else {
        idx[k++] = a; idx[k++] = c; idx[k++] = b;
        idx[k++] = b; idx[k++] = c; idx[k++] = d;
      }
    }
  }
  return { data, idx, x0: cx * CHUNK, y0: cy * CHUNK, pw, ph, lo, hi };
}

/** The terrain's chunks on the GPU, built lazily and rebuilt when the plots under them change. */
export class TerrainMeshes {
  constructor(gl) {
    this.gl = gl;
    this.chunks = new Map(); // "cx,cy,res" -> {mesh, x0, y0, pw, ph, lo, hi}
    this.world = null;
  }

  setWorld(world, dirtyPlots = null) {
    const same = this.world && world && this.world.w === world.w && this.world.h === world.h;
    this.world = world;
    if (!same || !dirtyPlots) {
      this.clear();
      return;
    }
    // A plot's change reaches one plot around it (the height blend); drop the chunks it touches.
    const drop = new Set();
    for (const i of dirtyPlots) {
      const x = i % world.w, y = Math.floor(i / world.w);
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) {
          let xx = x + dx;
          if (world.wrapX) xx = ((xx % world.w) + world.w) % world.w;
          // (Not bounded north and south: a plot of the map's edge rows shapes the ground past
          // the edge -- the unknown there lies low beside known water, world.js -- which is the
          // parchment's chunks', one row of chunks beyond the map each way.)
          const yy = y + dy;
          if (xx < 0 || xx >= world.w) continue;
          drop.add(`${Math.floor(xx / CHUNK)},${Math.floor(yy / CHUNK)}`);
        }
      }
    }
    for (const [k, c] of [...this.chunks]) {
      if (drop.has(k.split(",").slice(0, 2).join(","))) {
        c.mesh.dispose();
        this.chunks.delete(k);
      }
    }
  }

  clear() {
    for (const c of this.chunks.values()) c.mesh.dispose();
    this.chunks.clear();
  }

  /** Chunk (cx, cy) at `res`, building it on first use. `budget.n` limits builds per frame. */
  get(cx, cy, res, budget) {
    const k = `${cx},${cy},${res}`;
    let c = this.chunks.get(k);
    if (c) return c;
    if (budget && budget.n <= 0) return this.chunks.get(`${cx},${cy},${res === RES ? LOW_RES : RES}`) ?? null;
    if (budget) budget.n--;
    const b = buildChunk(this.world, cx, cy, res);
    c = { mesh: new Mesh(this.gl, b.data, TERRAIN_LAYOUT, { indices: b.idx }), x0: b.x0, y0: b.y0, pw: b.pw, ph: b.ph, lo: b.lo, hi: b.hi };
    this.chunks.set(k, c);
    return c;
  }

  dispose() {
    this.clear();
  }
}

/** The per-plot data textures (one texel per plot, NEAREST) and the soft visibility and height
 *  textures (LINEAR). */
export class MapTextures {
  constructor(gl) {
    this.gl = gl;
    this.terrain = null;
    this.vis = null;
    this.owner = null;
    this.height = null;
    this.w = 0;
    this.h = 0;
  }

  /** Rebuilds the textures from the world and the map reply; `dirty` (plot indices) limits the
   *  height samples recomputed to the plots around them. */
  update(world, map, dirty = null) {
    const gl = this.gl;
    const w = world.w, h = world.h, n = w * h;
    if (w !== this.w || h !== this.h) {
      this.dispose();
      dirty = null;
      this.w = w;
      this.h = h;
      const nearest = { filter: gl.NEAREST, wrapS: world.wrapX ? gl.REPEAT : gl.CLAMP_TO_EDGE };
      this.terrain = texture(gl, w, h, nearest);
      this.owner = texture(gl, w, h, nearest);
      this.vis = texture(gl, w, h, { internal: gl.R8, format: gl.RED, filter: gl.LINEAR, wrapS: world.wrapX ? gl.REPEAT : gl.CLAMP_TO_EDGE });
      this.height = texture(gl, w * HRES, h * HRES, { internal: gl.RG16F, format: gl.RG, type: gl.FLOAT, filter: gl.LINEAR, wrapS: world.wrapX ? gl.REPEAT : gl.CLAMP_TO_EDGE });
    }
    const t = new Uint8Array(n * 4);
    const o = new Uint8Array(n * 4);
    const v = new Uint8Array(n);
    for (let i = 0; i < n; i++) {
      t[i * 4] = world.ground[i];
      t[i * 4 + 1] = world.feat[i];
      t[i * 4 + 2] = world.kind[i];
      t[i * 4 + 3] = world.vis[i] * 127;
      v[i] = world.vis[i] === 2 ? 255 : world.vis[i] === 1 ? 128 : 0;
      const ow = map.owner[i];
      o[i * 4] = ow >= 0 ? ow + 1 : 0;
      o[i * 4 + 1] = map.city[i] >= 0 ? 255 : 0;
      o[i * 4 + 2] = map.improvement[i] >= 0 ? 255 : 0;
      o[i * 4 + 3] = map.route[i] >= 0 ? 255 : 0;
    }
    upload(gl, this.terrain, t);
    upload(gl, this.owner, o);
    upload(gl, this.vis, v);
    const W = w * HRES;
    const sample = (i, j) => {
      this.H[(j * W + i) * 2] = world.heightAt((i + 0.5) / HRES, (j + 0.5) / HRES);
      this.H[(j * W + i) * 2 + 1] = world.shore;
    };
    if (!dirty || !this.H) {
      this.H = new Float32Array(W * h * HRES * 2);
      for (let j = 0; j < h * HRES; j++) {
        for (let i = 0; i < W; i++) sample(i, j);
      }
    } else {
      // A plot's change reaches the height one plot (plus the warp) around it.
      const redo = new Set();
      for (const p of dirty) {
        const x = p % w, y = Math.floor(p / w);
        for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) {
          const k = world.index(x + dx, y + dy);
          if (k >= 0) redo.add(k);
        }
      }
      for (const k of redo) {
        const x = k % w, y = Math.floor(k / w);
        for (let j = y * HRES; j < (y + 1) * HRES; j++) {
          for (let i = x * HRES; i < (x + 1) * HRES; i++) sample(i, j);
        }
      }
    }
    upload(gl, this.height, this.H);
  }

  /** The per-plot mood channels (the HD-2D mode's region moods, hd2d/mood.js): two RGBA8 textures
   *  of four moods each, a texel a plot, read LINEAR, so a mood blends over the plot between two
   *  centres. Made on first use. */
  setMood(world, data, data2) {
    const gl = this.gl;
    for (const [k, d] of [["mood", data], ["mood2", data2]]) {
      if (!d) continue;
      if (!this[k] || this[k].w !== world.w || this[k].h !== world.h) {
        if (this[k]) gl.deleteTexture(this[k]);
        this[k] = texture(gl, world.w, world.h, { filter: gl.LINEAR, wrapS: world.wrapX ? gl.REPEAT : gl.CLAMP_TO_EDGE });
      }
      upload(gl, this[k], d);
    }
  }

  dispose() {
    const gl = this.gl;
    for (const k of ["terrain", "vis", "owner", "height", "mood", "mood2"]) if (this[k]) gl.deleteTexture(this[k]);
    this.terrain = this.vis = this.owner = this.height = this.mood = this.mood2 = null;
    this.w = this.h = 0;
  }
}

export { GROUND, KIND };
