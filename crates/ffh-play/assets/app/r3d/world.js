// The 3D world's ground model (docs/R3D.md "Terrain"): what each plot is, from the `map` reply the
// other renderers draw, and the continuous height of the land over it.
//
// Coordinates: map point (mx, my) -- plot (x, y) covers [x, x+1) x [y, y+1), north is +y -- sits at
// world (X, Z) = (mx, -my), Y up; one world unit is one plot. The plot centre (x + 0.5, y + 0.5) is
// where units, cities and markers stand, so the logical grid survives however the ground is shaped.
//
// The height is a blend of the plot kinds around a point (ocean, coast, land, hills, peak), with
// the blend's lookup warped by low noise so coasts and hill feet do not follow the grid; hills add
// rounded crowns; volcanoes add their cone, while ordinary peaks are models. Rivers (plot edges)
// carve a channel. Unrevealed plots are "void": flat, unknown ground (no hidden plot shapes it).

import { S, tname } from "../state.js";
import { noise2, noise2p, fbm2p, ridged2p, hashf } from "./noise.js";
import { smoothstep, clamp } from "./math.js";

/** Plot kinds (the shaders read them too, in the terrain texture's B channel). */
export const KIND = { VOID: 0, OCEAN: 1, COAST: 2, LAND: 3, HILLS: 4, PEAK: 5, ICE: 6 };

/** Ground materials (terrain texture R): one per terrain, as the terrain shader paints them. */
export const GROUND = {
  VOID: 0,
  GRASS: 1,
  PLAINS: 2,
  DESERT: 3,
  TUNDRA: 4,
  SNOW: 5,
  MARSH: 6,
  BURNING_SANDS: 7,
  BROKEN_LANDS: 8,
  PERDITION: 9,
  SHALLOWS: 10,
  COAST: 11,
  OCEAN: 12,
};

const GROUND_OF = {
  TERRAIN_GRASS: GROUND.GRASS,
  TERRAIN_PLAINS: GROUND.PLAINS,
  TERRAIN_DESERT: GROUND.DESERT,
  TERRAIN_TUNDRA: GROUND.TUNDRA,
  TERRAIN_SNOW: GROUND.SNOW,
  TERRAIN_MARSH: GROUND.MARSH,
  TERRAIN_BURNING_SANDS: GROUND.BURNING_SANDS,
  TERRAIN_BROKEN_LANDS: GROUND.BROKEN_LANDS,
  TERRAIN_FIELDS_OF_PERDITION: GROUND.PERDITION,
  TERRAIN_SHALLOWS: GROUND.SHALLOWS,
  TERRAIN_COAST: GROUND.COAST,
  TERRAIN_OCEAN: GROUND.OCEAN,
  // The Sky Falls' Crater (docs/CARDS.md, always hills): the tundra's grey, cracked ground.
  TERRAIN_CRATER: GROUND.TUNDRA,
};

/** Feature classes (terrain texture G). */
export const FEAT = {
  NONE: 0,
  FOREST: 1,
  JUNGLE: 2,
  ANCIENT: 3,
  BURNT: 4,
  YOUNG: 5,
  ICE: 6,
  FLOOD: 7,
  OASIS: 8,
  SCRUB: 9,
  FLAMES: 10,
  VOLCANO: 11,
  OBSIDIAN: 12,
  BLIZZARD: 13,
  SOULS: 14,
  FALLOUT: 15,
};

const FEAT_OF = {
  FEATURE_FOREST: FEAT.FOREST,
  FEATURE_JUNGLE: FEAT.JUNGLE,
  FEATURE_FOREST_ANCIENT: FEAT.ANCIENT,
  FEATURE_FOREST_BURNT: FEAT.BURNT,
  FEATURE_FOREST_NEW: FEAT.YOUNG,
  FEATURE_ICE: FEAT.ICE,
  FEATURE_FLOOD_PLAINS: FEAT.FLOOD,
  FEATURE_OASIS: FEAT.OASIS,
  FEATURE_SCRUB: FEAT.SCRUB,
  FEATURE_FLAMES: FEAT.FLAMES,
  FEATURE_VOLCANO: FEAT.VOLCANO,
  FEATURE_OBSIDIAN_PLAINS: FEAT.OBSIDIAN,
  FEATURE_BLIZZARD: FEAT.BLIZZARD,
  FEATURE_TORMENTED_SOULS: FEAT.SOULS,
  FEATURE_FALLOUT: FEAT.FALLOUT,
};

/** Base height of each kind at its plot centre. */
const BASE = [0.035, -0.42, -0.12, 0.1, 0.1, 0.1, 0.025];

export const SEA = 0;

/** With `World.cliff`: how far under the sea the floor lies at the foot of the land's face (the
 *  face stands in water, not on a beach). */
export const CLIFF_FOOT = 0.05;
/** With `World.cliff`: a lone hill's crown ends at this value of the hill's own distance (`domes`):
 *  there its scarp stands, up to SCARP high where it faces the hill's front, nothing at its back.
 *  CROWN_REACH: how far a hill's pull on the crowns' field reaches (hills side by side join their
 *  crowns into one plateau: `crownPull`). */
const CROWN_AT = 0.72;
const CROWN_REACH = 1.6;
export const SCARP = 0.2;

/** A hill's pull on the crowns' field at distance d of its own (`domes`): 1 at its middle, falling
 *  smoothly to nothing at CROWN_REACH. The crowns stand where the hills' pulls together pass a lone
 *  hill's at CROWN_AT. */
function crownPull(d) {
  const t = 1 - (d / CROWN_REACH) * (d / CROWN_REACH);
  return t > 0 ? t * t : 0;
}
const CROWN_PULL = crownPull(CROWN_AT);
/** With `World.cliff`: the bridge between two neighbouring hills pulls the crowns' field as a hill
 *  of this radius would, along the line between their middles (`domes`). */
const LINK_R = 0.5;
/** With `World.cliff`: each hill's top, a swell on its crown (`domes`): TOP_LOW to TOP_HIGH of a
 *  plot high (each hill its own), TOP_R of a plot round, easing in over TOP_IN of the crowns' field
 *  from the crown's rim. */
const TOP_LOW = 0.08;
const TOP_HIGH = 0.13;
const TOP_R = 0.55;
const TOP_IN = 0.25;

/** How deep a hull sits in the water: its keel is under the surface, so the water is seen over it
 *  (refracted, tinted) and laps at its waterline in a collar of foam (shaders.js WATER_FS). Only
 *  hulls (ships, boats, wrecks, swimmers) take it; what lies on the surface (a resource's ripples, a
 *  whirlpool, a floe) floats at the sea's level. */
export const DRAFT = 0.012;

/** Half a river's width of water, in plots (from its centre line to the water's edge): a broad
 *  river, the painted map's centrepiece. The edge wanders by up to RIVER_WANDER either way (the
 *  water shader draws it, WATER_FS), so the water is at most 2 * (RIVER_HALF + RIVER_WANDER) =
 *  0.44 plot wide -- also where a road crosses it, so a bridge of 0.6 plot spans it with its feet on
 *  the banks -- and with the line's meander the plot centres (where units stand) stay dry. */
export const RIVER_HALF = 0.19;
export const RIVER_WANDER = 0.03;
/** From a river's centre line to the top of its banks (the carved channel's full half-width). */
export const RIVER_BANK = 0.3;
/** How far round a river `riverDist` measures exactly (its cell index reaches this far). */
const RIVER_NEAR = 0.45;

/** Where the hell terrains begin (corrupted ground glows, ash falls). */
export function isHell(g) {
  return g === GROUND.BURNING_SANDS || g === GROUND.BROKEN_LANDS || g === GROUND.PERDITION;
}

export class World {
  /** `map`: the `map` reply (S.map shape). `opts.cliff`: the ground in steps (the HD-2D map;
   *  docs/HD2D.md §5f): the land a slab over its water -- dry land lifted by this much right from
   *  its shore, where it drops to the water in a sheer face, the floor falling away a little at the
   *  face's foot -- and a hill's crown standing over a scarp (`domes`). 0 (the 3D renderer): the
   *  shore slopes gently into the sea, a hill's scarp is a steep slope. */
  constructor(map, opts = {}) {
    this.cliff = opts.cliff ?? 0;
    /** What `heightAt` last worked out beside the height (the terrain mesh reads them after each
     *  call, terrain.js): `shore`, the shore field (see `shoreAt`), and `scarp`, the hills' (positive
     *  on a hill's crown, 0 where its scarp stands); and the point's height were it the floor under
     *  the water (`hFloor`), dry land below any crown (`hLand`) or on the nearest crown (`hCrown`) --
     *  on the shore or on a scarp the ground has two of them, the face between. */
    this.shore = 0;
    this.scarp = -CROWN_PULL;
    this.hFloor = 0;
    this.hLand = 0;
    this.hCrown = 0;
    this.w = map.w;
    this.h = map.h;
    this.wrapX = !!map.wrap_x;
    this.wrapY = !!map.wrap_y;
    this.map = map;
    const n = map.w * map.h;
    this.kind = new Uint8Array(n);
    this.ground = new Uint8Array(n);
    this.feat = new Uint8Array(n);
    this.vis = new Uint8Array(n);
    this.peakJitter = new Float32Array(n * 2);
    // A hill's scarp faces roughly south, with regional variations: cache the shape so sampling
    // the shared ground for meshes, roads, water and picking does not reroll its geology.
    this.hillShape = new Float32Array(n * 4);
    // A hill's top on its crown (with `cliff`, `domes`): where it stands off the hill's middle and
    // how high it swells.
    this.hillTop = new Float32Array(n * 3);
    this.classify();
    if (this.cliff > 0) this.findLowVoid();
    this.findLakes();
    this.buildRivers();
    this.centerH = new Float32Array(n);
    for (let y = 0; y < this.h; y++) for (let x = 0; x < this.w; x++) this.centerH[y * this.w + x] = this.heightAt(x + 0.5, y + 0.5);
  }

  classify() {
    const m = this.map;
    const ptype = (i) => tname("PlotType", m.plot_type[i]);
    for (let i = 0; i < this.w * this.h; i++) {
      this.vis[i] = m.vis[i];
      this.peakJitter[i * 2] = (hashf(i, 7) - 0.5) * 0.18;
      this.peakJitter[i * 2 + 1] = (hashf(i, 8) - 0.5) * 0.18;
      const facing = (hashf(i, 81) - 0.5) * 1.8;
      this.hillShape.set([Math.sin(facing), -Math.cos(facing), 0.58 + hashf(i, 82) * 0.12, hashf(i, 83) * 6.2832], i * 4);
      this.hillTop.set([(hashf(i, 84) - 0.5) * 0.18, (hashf(i, 85) - 0.5) * 0.18, TOP_LOW + (TOP_HIGH - TOP_LOW) * hashf(i, 86)], i * 3);
      if (m.vis[i] === 0) {
        this.kind[i] = KIND.VOID;
        this.ground[i] = GROUND.VOID;
        this.feat[i] = FEAT.NONE;
        continue;
      }
      const t = tname("TerrainType", m.terrain[i]);
      const f = tname("FeatureType", m.feature[i]);
      this.ground[i] = GROUND_OF[t] ?? GROUND.GRASS;
      this.feat[i] = FEAT_OF[f] ?? FEAT.NONE;
      const pt = ptype(i);
      let k = KIND.LAND;
      if (pt === "PLOT_OCEAN") k = t === "TERRAIN_OCEAN" ? KIND.OCEAN : KIND.COAST;
      else if (pt === "PLOT_HILLS") k = KIND.HILLS;
      else if (pt === "PLOT_PEAK") k = KIND.PEAK;
      if (this.feat[i] === FEAT.ICE && (k === KIND.OCEAN || k === KIND.COAST)) k = KIND.ICE;
      // Sea Ice (the Illians' Frozen Passage, the map's `sea_ice` layer): frozen sea, drawn as the
      // polar ice sheet (a blue-white floor with cracks at the water's level), not as snow land.
      if (m.sea_ice?.[i] === 1) {
        k = KIND.ICE;
        this.feat[i] = FEAT.ICE;
        this.ground[i] = GROUND.SNOW;
      }
      this.kind[i] = k;
    }
  }

  /** With `cliff`: the unexplored plots that lie low, at the water's level -- those next to known
   *  water or ice (`voidLow`). The rest of the unknown is lifted with the land (heightAt), so known
   *  land runs level to its frontier (and a river there does not fall), while known water still
   *  meets the unknown's parchment at its own level, without a face. */
  findLowVoid() {
    const n = this.w * this.h;
    this.voidLow = new Uint8Array(n);
    const wet = (k) => k === KIND.OCEAN || k === KIND.COAST || k === KIND.ICE;
    for (let i = 0; i < n; i++) {
      if (this.kind[i] !== KIND.VOID) continue;
      const x = i % this.w, y = (i - x) / this.w;
      for (let dy = -1; dy <= 1 && !this.voidLow[i]; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const j = this.index(x + dx, y + dy);
          if (j >= 0 && wet(this.kind[j])) this.voidLow[i] = 1;
        }
      }
    }
  }

  /** Whether the unknown at plot (x, y) lies low (`findLowVoid`); off the map, as the map's nearest
   *  plot does or would: low beside water, ice or low unknown. */
  lowVoid(x, y) {
    let i = this.index(x, y);
    if (i >= 0) return this.voidLow[i] === 1;
    i = this.index(this.wrapX ? x : clamp(x, 0, this.w - 1), this.wrapY ? y : clamp(y, 0, this.h - 1));
    const k = this.kind[i];
    return k === KIND.VOID ? this.voidLow[i] === 1 : k === KIND.OCEAN || k === KIND.COAST || k === KIND.ICE;
  }

  /** Lakes: bodies of water of at most 12 plots (8-connected, as the game's areas) that touch no
   *  unrevealed plot. Their floors are sunk into bowls (heightAt): the blend with the land around
   *  would leave a small lake all shelf, so it painted as surf and pale shallows, like a salt pan. */
  findLakes() {
    const n = this.w * this.h;
    this.lake = new Uint8Array(n);
    this.hasLakes = false;
    const seen = new Uint8Array(n);
    const water = (k) => k === KIND.OCEAN || k === KIND.COAST;
    for (let s = 0; s < n; s++) {
      if (seen[s] || !water(this.kind[s])) continue;
      const comp = [s];
      seen[s] = 1;
      let open = false;
      for (let q = 0; q < comp.length && comp.length <= 12; q++) {
        const x = comp[q] % this.w, y = (comp[q] - x) / this.w;
        for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const j = this.index(x + dx, y + dy);
            if (j < 0 || j === comp[q]) continue;
            if (this.kind[j] === KIND.VOID) open = true;
            else if (!seen[j] && water(this.kind[j])) {
              seen[j] = 1;
              comp.push(j);
            }
          }
        }
      }
      if (comp.length <= 12 && !open) {
        for (const i of comp) this.lake[i] = 1;
        this.hasLakes = true;
      }
    }
  }

  /** The round bowls of the lake plots near (mx, my), summed (0 at a bowl's rim, 1 at a centre). */
  lakeBowl(mx, my) {
    let s = 0;
    const cx = Math.floor(mx);
    const cy = Math.floor(my);
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const i = this.index(cx + dx, cy + dy);
        if (i < 0 || !this.lake[i]) continue;
        const t = clamp(1 - Math.hypot(mx - (cx + dx + 0.5), my - (cy + dy + 0.5)) / 0.8, 0, 1);
        s += t * t * (3 - 2 * t);
      }
    }
    return s;
  }

  /** The plot index of (x, y) after wrapping, or -1 off the map. */
  index(x, y) {
    if (this.wrapX) x = ((x % this.w) + this.w) % this.w;
    else if (x < 0 || x >= this.w) return -1;
    if (this.wrapY) y = ((y % this.h) + this.h) % this.h;
    else if (y < 0 || y >= this.h) return -1;
    return y * this.w + x;
  }

  kindAt(x, y) {
    const i = this.index(x, y);
    return i < 0 ? KIND.VOID : this.kind[i];
  }

  isWater(i) {
    const k = this.kind[i];
    return k === KIND.OCEAN || k === KIND.COAST;
  }

  /** The ground height at map point (mx, my). */
  heightAt(mx, my, carve = true) {
    // Warp the blend lookup so coastlines and hill feet wander off the grid lines.
    // Two octaves: broad bays and headlands, then a ragged edge (plot centres stay inside their
    // plot: the warp never reaches half a plot).
    const N = (f, ox, oy) => this.n2(mx, my, f, ox, oy);
    const wx = mx + 0.34 * N(0.7, 3.1, -1.7) + 0.1 * N(2.6, -4.4, 9.1);
    const wy = my + 0.34 * N(0.7, -7.3, 5.5) + 0.1 * N(2.6, 6.2, -2.3);
    const u = wx - 0.5;
    const v = wy - 0.5;
    const x0 = Math.floor(u);
    const y0 = Math.floor(v);
    const fx = smoother(u - x0);
    const fy = smoother(v - y0);
    let h = 0, hill = 0, peak = 0, water = 0, ice = 0, voidW = 0, voidLo = 0, sand = 0, lakeW = 0;
    for (let j = 0; j < 2; j++) {
      for (let i = 0; i < 2; i++) {
        const k = this.kindAt(x0 + i, y0 + j);
        const wgt = (i ? fx : 1 - fx) * (j ? fy : 1 - fy);
        h += wgt * BASE[k];
        const gi = this.index(x0 + i, y0 + j);
        if (gi >= 0 && (this.ground[gi] === GROUND.DESERT || this.ground[gi] === GROUND.BURNING_SANDS) && k === KIND.LAND) sand += wgt;
        if (gi >= 0 && this.lake[gi]) lakeW += wgt;
        if (k === KIND.HILLS) hill += wgt;
        else if (k === KIND.PEAK) {
          // Ordinary peaks are models, with no terrain mound to bury their flanks.
          if (this.feat[gi] === FEAT.VOLCANO) peak += wgt;
        } else if (k === KIND.OCEAN || k === KIND.COAST) water += wgt;
        else if (k === KIND.ICE) ice += wgt;
        else if (k === KIND.VOID) {
          voidW += wgt;
          if (this.cliff > 0 && this.lowVoid(x0 + i, y0 + j)) voidLo += wgt;
        }
      }
    }
    const land = 1 - water - ice - voidW;
    // Gentle undulation of the lowlands; rough sea floor.
    h += land * 0.05 * this.fbm(mx, my, 1.1, 40, -20, 3);
    h += water * 0.06 * this.fbm(mx, my, 0.9, -11, 4, 3);
    // A lake's floor falls away just inside its shore (the shore is where the blend crosses sea
    // level, at about 0.45 of the lake's weight), so a lake is a bowl with a narrow rim of surf;
    // round bowls about its plots' centres keep the deep water from following the plot grid.
    if (lakeW > 0.4) h -= 0.2 * smoothstep(0.42, 0.95, lakeW) * Math.min(1, this.lakeBowl(mx, my) * 1.4);
    if (sand > 0.05) {
      // Dunes: wind-combed ridges, sharp crest and long back slope, wandering with the noise.
      const ph = 2 * Math.PI * (mx * this.freq(1.25) + my * 0.55) + 2.6 * this.n2(mx, my, 0.6, 12.5, -4.1);
      const crest = Math.pow(0.5 + 0.5 * Math.sin(ph), 3);
      h += smoothstep(0.05, 0.8, sand) * 0.045 * crest * (0.6 + 0.4 * this.n2(mx, my, 1.4, -2.2, 7.7));
    }
    // Hills and mountains do not spill into a lake (their domes reach over the plot's edge).
    const dry = lakeW > 0.3 ? 1 - smoothstep(0.3, 0.6, lakeW) : 1;
    // (The nearest river: its channel is carved below, and with `cliff` a scarp gives way to it.)
    const rd = this.riverCells.size && (carve || this.cliff > 0) ? this.riverDist(mx, my) : Infinity;
    // With `cliff`: the scarp field here, and how far the ground stands higher on the nearest crown
    // and at this point (0 off the crowns) than the hills' continuous ground.
    let scarp = -CROWN_PULL, riseIn = 0, rise = 0;
    if (hill > 0 && dry > 0) {
      // Turf-capped scarps: broad crowns above a short exposed face and a lower grassy shelf.
      // This is the ground itself, shared by everything that stands on or follows the hill.
      const r = this.ridged(mx, my, 1.6, 5.2, -3.4, 3);
      h += dry * smoothstep(0, 1, hill) * (0.04 + 0.025 * r);
      h += dry * this.domes(mx, my, rd);
      if (this.cliff > 0) {
        scarp = this.crown;
        riseIn = dry * this.crownIn;
        rise = dry * this.crownAt;
      }
    }
    if (peak > 0 && dry > 0) {
      // Mountains: ridged massif, plus a summit near each peak plot's centre.
      // Sharp crests (ridged noise raised to a power: narrow ridges, deep gullies), a second,
      // finer set of spurs, and the summit of each peak plot.
      const r = this.ridged(mx, my, 1.1, -2.7, 8.1, 4);
      const r2 = this.ridged(mx, my, 2.6, 5.3, -1.9, 3);
      const pw = Math.pow(peak, 1.4);
      // Only volcanoes use terrain relief; ordinary mountains supply their own flanks.
      h += 0.16 * dry * pw * (0.18 + 0.62 * Math.pow(r, 1.7) + 0.16 * r2 * r);
      h += dry * this.summits(mx, my) * (0.75 + 0.35 * r2);
    }
    if (voidW > 0) {
      h = h * (1 - voidW) + BASE[KIND.VOID] * voidW;
      riseIn *= 1 - voidW;
      rise *= 1 - voidW;
    }
    // The shore field: the ground before any bank or face, continuous across the shore, which is
    // where it crosses the sea's level.
    this.shore = h;
    this.scarp = scarp;
    let cut = 0, bedFloor = -Infinity;
    // Rivers carve their channel (`carve` false: the ground the river runs over, uncut).
    if (carve && this.riverCells.size) {
      const d = rd;
      if (d < RIVER_BANK) {
        // A flat bed under all the water the shader may draw (its edge wanders, RIVER_WANDER), then
        // a steep cut bank up to the bank tops. The bed runs on under the water's edge: the terrain
        // mesh (RES vertices a plot) is too coarse to draw the shore, so the water shader does.
        const cv = 1 - smoothstep(RIVER_HALF + RIVER_WANDER, RIVER_BANK, d);
        cut = 0.09 * cv * cv * (3 - 2 * cv);
        // A river's bed never sinks deep under sea level -- on land: over the sea the floor stays
        // the sea's (raised, it would show through the clear water as a sandbar along the river).
        if (cv > 0.5) bedFloor = -0.06 - 0.5 * smoothstep(0.3, 0.7, water);
      }
    }
    if (this.cliff > 0) {
      // The land a slab: lifted whole from its shore, where it drops sheer to the floor at the
      // face's foot. Not the ice (a sheet at the water's level) nor the unknown beside known water
      // (`findLowVoid`): toward them the lift fades, and the land slopes down to them as it did.
      // And a hill's crown stands over its scarp (`domes`). The ground steps at the shore and at a
      // scarp: the terrain mesh cuts its faces there (terrain.js).
      const f = 1 - smoothstep(0.15, 0.6, ice + voidLo);
      const land = h + this.cliff * f - cut;
      this.hFloor = Math.max(h - CLIFF_FOOT * f - cut, bedFloor);
      this.hLand = Math.max(land, bedFloor);
      this.hCrown = Math.max(land + riseIn, bedFloor);
      if (h <= 0) return this.hFloor;
      return scarp > 0 ? Math.max(land + rise, bedFloor) : this.hLand;
    }
    return Math.max(h - cut, bedFloor);
  }

  /** The shore field at (mx, my): the ground's height before any bank or face (and uncut by the
   *  rivers), positive on dry land and negative under water, continuous across the shore. Where it
   *  is 0 the shore runs; with `cliff` the land's face stands there. */
  shoreAt(mx, my) {
    this.heightAt(mx, my, false);
    return this.shore;
  }

  // Noise of the height, at about f cycles a plot, periodic over the map's width when it wraps
  // (so the ground meets itself at the seam, and every copy of the map is the same ground).

  period(f) {
    return this.wrapX ? Math.max(1, Math.round(this.w * f)) : 1 << 20;
  }

  freq(f) {
    return this.wrapX ? this.period(f) / this.w : f;
  }

  n2(mx, my, f, ox, oy) {
    return noise2p(mx * this.freq(f) + ox, my * f + oy, this.period(f));
  }

  fbm(mx, my, f, ox, oy, oct) {
    return fbm2p(mx * this.freq(f) + ox, my * f + oy, this.period(f), oct);
  }

  ridged(mx, my, f, ox, oy, oct) {
    return ridged2p(mx * this.freq(f) + ox, my * f + oy, this.period(f), oct);
  }

  /** Hills merge at their feet into a continuous surface. Rounded crowns have an irregular
   *  scarp on one flank, a lower shelf, and a gentle back slope. Peaks have their own models.
   *  With `cliff` the scarp is a step: the crown's front stands sheer over the shelf, up to SCARP
   *  high at its middle (each hill its own height, 0.7 to 1 of it) and nothing at its ends and
   *  round the back (where the crown rises in the broad ramp it has there), and gives way to a
   *  river (`rd`: how far the nearest runs). Hills side by side stand on one crown, a plateau under
   *  one scarp: the crowns are where the hills' pulls together (`crownPull`) pass a lone hill's at
   *  its rim (neighbouring hills bridged by a pull along the line between their middles), and the
   *  step's height and the way it faces are the hills' own, mixed by their pulls; each hill's top
   *  swells out of the crown (TOP_LOW..TOP_HIGH), so a group rolls over a top for each hill.
   *  The value is then the hills' ground without the step, and beside it: `crown`, the scarp field
   *  (positive on a crown); `crownIn`, how far a crown would stand over that ground here;
   *  `crownAt`, how far the ground itself does (0 off the crowns). */
  domes(mx, my, rd = Infinity) {
    let s = 0;
    const scarps = this.cliff > 0;
    const fade = scarps ? smoothstep(RIVER_BANK, RIVER_BANK + 0.14, rd) : 0;
    // (With `cliff`: each hill's ground apart from its crown's ramp, and that ramp; the pulls'
    // sum, and the hills' sheerness and height of scarp weighed by them.)
    let n = 0, pull = 0, sheerSum = 0, highSum = 0;
    const rest = this.domeRest ?? (this.domeRest = new Float64Array(9)), ramp = this.domeRamp ?? (this.domeRamp = new Float64Array(9));
    // (With `cliff`, every hill of the 3 by 3 plots round the point: its window cell, its middle,
    // how sheer it stands and how high its scarp toward the point, and its top's swell here.)
    let m = 0, swell = 0;
    const hc = this.domeHills ?? (this.domeHills = new Float64Array(9 * 6));
    const cx = Math.floor(mx);
    const cy = Math.floor(my);
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const i = this.index(cx + dx, cy + dy);
        if (i < 0 || this.kind[i] !== KIND.HILLS) continue;
        const px = cx + dx + 0.5 + this.peakJitter[i * 2] * 1.6;
        const py = cy + dy + 0.5 + this.peakJitter[i * 2 + 1] * 1.6;
        const k = i * 4, radius = this.hillShape[k + 2], phase = this.hillShape[k + 3];
        const u = (mx - px) / radius, v = (my - py) / (radius * 0.86);
        const d0 = Math.hypot(u, v);
        const front = smoothstep(-0.35, 0.65, (u * this.hillShape[k] + v * this.hillShape[k + 1]) / Math.max(0.01, d0));
        if (scarps) {
          // (About the hill's middle the way a point lies from it turns fast: there the front
          // counts half, on every side, so the crown has no crease through its middle.)
          // The scarp tapers out over a sixth of a plot of the rim (a broad turn from front to
          // back: on the crown, between the two, the ground ramps from the step's height to the
          // back's slope, and a quick turn would be a crease the grid cannot draw).
          const o = m * 6;
          hc[o] = dx;
          hc[o + 1] = dy;
          hc[o + 2] = px;
          hc[o + 3] = py;
          hc[o + 4] = 0.5 + (smoothstep(0.22, 0.82, front) - 0.5) * smoothstep(0.1, 0.35, d0);
          hc[o + 5] = 0.7 + 0.3 * Math.abs(Math.sin(phase * 3.7));
          m++;
          // Its top: a broad rounded swell about a point near its middle, each hill its own
          // height and place, so a crown of many hills rolls over as many tops.
          const sd = Math.hypot(mx - px - this.hillTop[i * 3], my - py - this.hillTop[i * 3 + 1]) / TOP_R;
          if (sd < 1) swell += this.hillTop[i * 3 + 2] * (1 - sd * sd) * (1 - sd * sd);
        }
        // (A hill's ground reaches 1.35; with `cliff` its pull on the crowns CROWN_REACH.)
        if (d0 > (scarps ? CROWN_REACH + 0.05 : 1.35)) continue;
        // Broad lobes break the rim without turning the crown into noisy rubble.
        // (With `cliff` fewer and shallower: each lobe of a crown's rim is a buttress of its scarp.)
        const d = d0 + (scarps ? 0.04 * Math.sin(u * 5 + phase) * Math.sin(v * 4.5 - phase) : 0.055 * Math.sin(u * 8 + phase) * Math.sin(v * 7 - phase));
        // A short exposure within a broad turf slope, rather than a sheer ring holding up a
        // flat pedestal. The outer shoulder carries the grass continuously down to the plain.
        const width = 0.62 - 0.42 * front;
        const shelf = 0.035 * (1 - smoothstep(0.96, 1.24, d));
        const shoulder = 0.10 * (1 - smoothstep(0.18, 1.34, d));
        const turf = 0.045 * Math.max(0, 1 - d * d);
        if (!scarps) {
          const crown = 0.16 * (1 - smoothstep(0.72 - width * 0.5, 0.72 + width * 0.5, d));
          s = Math.max(s, crown + shelf + shoulder + turf);
          continue;
        }
        rest[n] = shelf + shoulder + turf;
        ramp[n++] = 0.16 * (1 - smoothstep(0.41, 1.03, d));
        const g = crownPull(d);
        pull += g;
        sheerSum += g * hc[(m - 1) * 6 + 4];
        highSum += g * hc[(m - 1) * 6 + 5];
      }
    }
    if (scarps) {
      // Neighbouring hills (diagonal ones too) are bridged: a pull along the line between their
      // middles, nothing at its ends, so the crown never parts between two of them (their own
      // pulls alone left a strip of the plain between two rows), its step and facing theirs.
      for (let a = 0; a < m; a++) {
        for (let b = a + 1; b < m; b++) {
          const oa = a * 6, ob = b * 6;
          if (Math.max(Math.abs(hc[oa] - hc[ob]), Math.abs(hc[oa + 1] - hc[ob + 1])) !== 1) continue;
          const ex = hc[ob + 2] - hc[oa + 2], ey = hc[ob + 3] - hc[oa + 3];
          const t = clamp(((mx - hc[oa + 2]) * ex + (my - hc[oa + 3]) * ey) / (ex * ex + ey * ey), 0, 1);
          if (t <= 0 || t >= 1) continue;
          const g = crownPull(Math.hypot(mx - hc[oa + 2] - ex * t, my - hc[oa + 3] - ey * t) / LINK_R) * Math.sin(Math.PI * t);
          pull += g;
          sheerSum += g * 0.5 * (hc[oa + 4] + hc[ob + 4]);
          highSum += g * 0.5 * (hc[oa + 5] + hc[ob + 5]);
        }
      }
      // Where the front is sheer the crown's ramp is the step instead.
      const sheer = pull > 0 ? (sheerSum / pull) * fade : 0;
      for (let k = 0; k < n; k++) s = Math.max(s, ramp[k] * (1 - sheer) + rest[k]);
      // The tops swell out of the crown, from nothing at its rim (no second step).
      s += swell * smoothstep(0, TOP_IN, pull - CROWN_PULL);
      this.crown = pull - CROWN_PULL;
      this.crownIn = pull > 0 ? SCARP * sheer * (highSum / pull) : 0;
      this.crownAt = this.crown > 0 ? this.crownIn : 0;
    }
    return s;
  }

  /** The summit cones of the peak plots near (mx, my). */
  summits(mx, my) {
    let s = 0;
    const cx = Math.floor(mx);
    const cy = Math.floor(my);
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const i = this.index(cx + dx, cy + dy);
        if (i < 0 || this.kind[i] !== KIND.PEAK || this.feat[i] !== FEAT.VOLCANO) continue;
        // Only volcanoes use terrain relief for their body.
        const px = cx + dx + 0.5 + this.peakJitter[i * 2];
        const py = cy + dy + 0.5 + this.peakJitter[i * 2 + 1];
        const d = Math.hypot(mx - px, my - py);
        if (d > 0.9) continue;
        // Not a cone: the reach of the summit's flanks varies around it (two low harmonics, their
        // phases per plot), so spurs run out from the top with gullies between them.
        const a = Math.atan2(my - py, mx - px);
        const p1 = hashf(i, 71) * 6.2832, p2 = hashf(i, 72) * 6.2832;
        const reach = 0.62 * (1 + 0.3 * Math.sin(3 * a + p1) + 0.14 * Math.sin(5 * a + p2));
        const t = clamp(1 - d / reach, 0, 1);
        s = Math.max(s, 0.55 * Math.pow(t, 1.25));
        // A lower second summit on one shoulder: a lone mountain is a lopsided massif.
        const b = hashf(i, 73) * 6.2832;
        const d2 = Math.hypot(mx - (px + Math.cos(b) * 0.3), my - (py + Math.sin(b) * 0.3));
        const t2 = clamp(1 - d2 / 0.36, 0, 1);
        s = Math.max(s, 0.36 * Math.pow(t2, 1.1));
      }
    }
    return s;
  }

  /** The height at the centre of plot (x, y): where units and buildings stand. */
  plotHeight(x, y) {
    const i = this.index(x, y);
    if (i < 0) return BASE[KIND.VOID];
    return Math.max(this.centerH[i], SEA);
  }

  /** The normal of the ground at (mx, my) (world space: x east, y up, z south). */
  normalAt(mx, my, e = 0.05) {
    const hx = this.heightAt(mx + e, my) - this.heightAt(mx - e, my);
    const hy = this.heightAt(mx, my + e) - this.heightAt(mx, my - e);
    // World z = -my, so d/dz = -d/dmy.
    const n = [-hx / (2 * e), 1, hy / (2 * e)];
    const l = Math.hypot(n[0], n[1], n[2]);
    return [n[0] / l, n[1] / l, n[2] / l];
  }

  // --- rivers -----------------------------------------------------------------------------------

  /** River polylines in map coordinates, from the plot-edge river bits: chained through shared
   *  corners, smoothed (Chaikin) and given a slight meander. `riverCells` indexes their segments by
   *  integer cell for distance queries. */
  buildRivers() {
    const m = this.map;
    const w = this.w;
    const segs = [];
    for (let y = 0; y < this.h; y++) {
      for (let x = 0; x < w; x++) {
        const i = y * w + x;
        if (m.vis[i] === 0) continue;
        const r = m.river[i];
        if (r <= 0) continue;
        if (r & 1) segs.push([[x, y], [x + 1, y]]); // the plot's south edge
        if (r & 2) segs.push([[x + 1, y], [x + 1, y + 1]]); // its east edge
      }
    }
    // Chain the segments through shared corners.
    const key = ([x, y]) => `${this.wrapX ? ((x % w) + w) % w : x},${y}`;
    const at = new Map();
    segs.forEach((s, k) => {
      for (const p of s) {
        const kk = key(p);
        if (!at.has(kk)) at.set(kk, []);
        at.get(kk).push(k);
      }
    });
    const used = new Uint8Array(segs.length);
    const lines = [];
    // The way out of corner `p` along segment k (a unit step; across the seam, the short way).
    const step = (k, p) => {
      const s = segs[k];
      const o = key(s[0]) === key(p) ? s[1] : s[0];
      let dx = o[0] - p[0];
      if (this.wrapX) dx -= Math.round(dx / w) * w;
      return [dx, o[1] - p[1]];
    };
    // Walks from corner `from` along segment `start`, through corners shared by two segments and
    // straight on through a junction (a broad river must not break into a sharp corner where a
    // tributary joins it: the tributary ends there, in its water), until an end.
    const walk = (start, from) => {
      const line = [from.slice()];
      let cur = start;
      let p = from;
      for (;;) {
        used[cur] = 1;
        const [dx, dy] = step(cur, p);
        const s = segs[cur];
        const other = key(s[0]) === key(p) ? s[1] : s[0];
        // Keep the line continuous across the wrap seam.
        const last = line[line.length - 1];
        let qx = other[0];
        if (this.wrapX) qx += Math.round((last[0] - qx) / w) * w;
        line.push([qx, other[1]]);
        p = other;
        const all = at.get(key(p)) ?? [];
        const next = all.filter((k) => !used[k]);
        if (all.length === 2 && next.length === 1) cur = next[0];
        else {
          const on = all.length > 2 ? next.find((k) => { const [ex, ey] = step(k, p); return ex * dx + ey * dy > 0.5; }) : undefined;
          if (on === undefined) break;
          cur = on;
        }
      }
      return line;
    };
    // Start from ends, then from junctions (what is left of them: the tributaries), then any loops.
    for (const pass of [1, 3]) {
      for (const [kk, ks] of at) {
        if (pass === 1 ? ks.length !== 1 : ks.length < 3) continue;
        for (const k of ks) {
          if (used[k]) continue;
          const s = segs[k];
          lines.push(walk(k, key(s[0]) === kk ? s[0] : s[1]));
        }
      }
    }
    segs.forEach((s, k) => {
      if (!used[k]) lines.push(walk(k, s[0]));
    });
    // Smoothed from the corners themselves (a turn rounds to about a third of a plot's radius: a
    // broad river's ribbon would fold over itself in a tighter one), then subdivided so even a
    // one-edge reach (2 points, a straight bar otherwise) has interior points to meander (F18).
    this.rivers = lines.map((l) => meander(subdivide(chaikin(chaikin(chaikin(l))), 0.1)));
    // Segment index by cell.
    this.riverCells = new Map();
    this.riverSegs = [];
    for (const line of this.rivers) {
      for (let i = 0; i + 1 < line.length; i++) {
        const a = line[i], b = line[i + 1];
        const k = this.riverSegs.length;
        this.riverSegs.push([a[0], a[1], b[0], b[1]]);
        const x0 = Math.floor(Math.min(a[0], b[0]) - RIVER_NEAR), x1 = Math.floor(Math.max(a[0], b[0]) + RIVER_NEAR);
        const y0 = Math.floor(Math.min(a[1], b[1]) - RIVER_NEAR), y1 = Math.floor(Math.max(a[1], b[1]) + RIVER_NEAR);
        for (let y = y0; y <= y1; y++) {
          for (let x = x0; x <= x1; x++) {
            const ck = `${this.wrapX ? ((x % w) + w) % w : x},${y}`;
            if (!this.riverCells.has(ck)) this.riverCells.set(ck, []);
            this.riverCells.get(ck).push(k);
          }
        }
      }
    }
  }

  /** Distance from map point (mx, my) to the nearest river (exact within RIVER_NEAR; beyond it
   *  larger, or Infinity when none is near). */
  riverDist(mx, my) {
    const w = this.w;
    const cx = Math.floor(mx);
    const ck = `${this.wrapX ? ((cx % w) + w) % w : cx},${Math.floor(my)}`;
    const list = this.riverCells.get(ck);
    if (!list) return Infinity;
    let best = Infinity;
    for (const k of list) {
      const [ax0, ay, bx0, by] = this.riverSegs[k];
      // Bring the segment next to the point across the wrap seam.
      let ax = ax0, bx = bx0;
      if (this.wrapX) {
        const s = Math.round((mx - ax) / w) * w;
        ax += s;
        bx += s;
      }
      best = Math.min(best, segDist(mx, my, ax, ay, bx, by));
    }
    return best;
  }

  /** The nearest point of a river line to map point (mx, my) and its distance, [x, y, d] (within
   *  RIVER_NEAR), or null when no river is near: where the water's surface over (mx, my) is laid
   *  (scene.js riverLevel). */
  riverPoint(mx, my) {
    const w = this.w;
    const cx = Math.floor(mx);
    const list = this.riverCells.get(`${this.wrapX ? ((cx % w) + w) % w : cx},${Math.floor(my)}`);
    if (!list) return null;
    let best = null;
    for (const k of list) {
      const [ax0, ay, bx0, by] = this.riverSegs[k];
      const s = this.wrapX ? Math.round((mx - ax0) / w) * w : 0;
      const ax = ax0 + s, bx = bx0 + s;
      const dx = bx - ax, dy = by - ay;
      const t = clamp(((mx - ax) * dx + (my - ay) * dy) / (dx * dx + dy * dy || 1e-9), 0, 1);
      const px = ax + dx * t, py = ay + dy * t, d = Math.hypot(mx - px, my - py);
      if (!best || d < best[2]) best = [px, py, d];
    }
    return best;
  }
}

function smoother(t) {
  return t * t * t * (t * (t * 6 - 15) + 10);
}

function segDist(px, py, ax, ay, bx, by) {
  const dx = bx - ax, dy = by - ay;
  const l2 = dx * dx + dy * dy || 1e-9;
  const t = clamp(((px - ax) * dx + (py - ay) * dy) / l2, 0, 1);
  return Math.hypot(px - ax - dx * t, py - ay - dy * t);
}

/** Insert interior points so each segment is about `step` long (keeps the end points), so short
 *  reaches have points to bend. */
function subdivide(pts, step) {
  if (pts.length < 2) return pts;
  const out = [pts[0]];
  for (let i = 0; i + 1 < pts.length; i++) {
    const a = pts[i], b = pts[i + 1];
    const n = Math.max(1, Math.round(Math.hypot(b[0] - a[0], b[1] - a[1]) / step));
    for (let k = 1; k <= n; k++) out.push([a[0] + ((b[0] - a[0]) * k) / n, a[1] + ((b[1] - a[1]) * k) / n]);
  }
  return out;
}

/** One round of Chaikin corner cutting (keeps the end points). */
function chaikin(pts) {
  if (pts.length < 3) return pts;
  const out = [pts[0]];
  for (let i = 0; i + 1 < pts.length; i++) {
    const a = pts[i], b = pts[i + 1];
    out.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25], [a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
  }
  out.push(pts[pts.length - 1]);
  return out;
}

/** A gentle sideways wander along a polyline (deterministic by position): long, low swings (a
 *  broad river's ribbon must not fold in a tight wiggle; the water's edge wanders finer, in the
 *  shader), fading in from the ends so a river still meets its mouth or its tributary's corner. */
function meander(pts) {
  const along = [0];
  for (let i = 1; i < pts.length; i++) along.push(along[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const total = along[along.length - 1];
  return pts.map((p, i) => {
    if (i === 0 || i === pts.length - 1) return p;
    const a = pts[i - 1], b = pts[i + 1];
    let tx = b[0] - a[0], ty = b[1] - a[1];
    const l = Math.hypot(tx, ty) || 1;
    tx /= l;
    ty /= l;
    const ends = smoothstep(0, 0.6, Math.min(along[i], total - along[i]));
    const o = ends * (0.075 * noise2(p[0] * 0.9 + 0.5, p[1] * 0.9 - 0.5) + 0.025 * noise2(p[0] * 2.1 - 2.5, p[1] * 2.1 + 1.5));
    return [p[0] - ty * o, p[1] + tx * o];
  });
}

/** The world built from the current map (S.map), or null. */
export function worldOf(map = S.map) {
  return map ? new World(map) : null;
}
