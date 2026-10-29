// What the HD-2D mode stands in the world as sprites (docs/HD2D.md): the isometric renderer's art
// (docs/ISO.md), chosen as iso.js chooses it -- a unit's sprite by its civilization, type, class and
// stand-ins (iso.js unitSpriteKeys), a city's by its size and civilization, an improvement's and a
// resource's by kind -- and placed on the 3D renderer's ground. A stack is a group of two or three
// figures of its head unit under the owner's banner; a city, a standing improvement and a resource
// are one card each. The 3D models of what these replace are not placed (`claims`, read by
// ../dress.js); forests, rocks, fields, flat improvements and the land stay the 3D renderer's.
//
// Records (hd2d/sprites.js packs them): { key, x, y, z (the base point, world), w, h (the card,
// world units), anchor (0..1 across the card: where the base point is), flip, banner, shadow (a
// figure: casts into the sun's map, has a contact shadow), team [r, g, b], bias (depth toward the
// eye), id (object id), glow, phase, uid (walking: hd2d/index.js) }.

import { S, tname } from "../../state.js";
import * as T from "../../tiles.js";
import { stackState } from "../../stacks.js";
import { isBarbarian } from "../../state.js";
import { unitSpriteKeys, civShortName, ISO_TABLES } from "../../iso.js";
import { hashf, hash32, rng } from "../noise.js";
import { SEA, DRAFT, FEAT, KIND, GROUND, RIVER_BANK, RIVER_HALF, RIVER_WANDER } from "../world.js";
import { CARDS } from "./art/index.js";
import { standHeight, hexRgb } from "../dress.js";
import { MANA } from "../assets/palette.js";
import { clamp } from "../math.js";

/** World units per art pixel: figures (a 96 x 120 unit canvas is 0.46 x 0.58 of a plot), cities (a
 *  192-wide settlement 1.15 plots: its painted ground spans the plot), the other buildings, the
 *  resources' props, the banners. */
export const PX = { unit: 0.0048, city: 0.0088, building: 0.0038, resource: 0.0034, banner: 0.0053, prop: 0.0031 };
/** Depth toward the eye (world units): a figure before the card of the building it stands at. */
const BIAS = { unit: 0.26, garrison: 0.4, building: 0, resource: 0.08 };
/** Where a stack stands (plot-local x east, z south); on a city's plot, its front-left corner; on a
 *  standing improvement's, in front of it. */
const SPOT = [0, 0.06];
const BUILT_SPOT = [0.05, 0.3];
const VOLCANO_SPOT = [-0.14, 0.38];
const BONUS_SPOT = [-0.26, 0.2];
/** Where a city's garrison stands (plot-local): in front of the city's card (whose foot is on the
 *  plot's front edge), the first of a few spots out of a river, else the one furthest from it. */
const GARRISON = [[-0.27, 0.36], [0.27, 0.36], [-0.1, 0.42], [0.12, 0.42]];
function garrisonSpot(world, x, y) {
  if (!world.riverCells.size) return GARRISON[0];
  let best = GARRISON[0], bestD = -1;
  for (const s of GARRISON) {
    const d = world.riverDist(x + 0.5 + s[0], y + 0.5 - s[1]);
    if (d >= RIVER_BANK + 0.05) return s;
    if (d > bestD) [best, bestD] = [s, d];
  }
  return best;
}

/** A group of figures about its spot: two in front, one behind between them, far enough apart that
 *  each reads as a figure (about 0.6 of a figure's width), not a single blot. */
const GROUP = {
  3: [[-0.1, 0.07], [0.11, 0.05], [0.0, -0.08]],
  2: [[-0.08, 0.05], [0.09, -0.03]],
  1: [[0, 0]],
};

const ids = new Map();
let nextId = 1;
/** A stable object id in 0..1 (the ghost pass tells a figure's own pixels by it). */
function idOf(key) {
  let v = ids.get(key);
  if (v === undefined) {
    v = ((hash32(nextId++ * 2246822519) % 9973) + 1) / 9975;
    ids.set(key, v);
  }
  return v;
}

function teamCss(owner) {
  return T.playerColor(owner, isBarbarian(owner));
}

/** The first of `keys` the atlas has, or null. */
function first(atlas, keys) {
  for (const k of keys) if (k && atlas.has(k)) return k;
  return null;
}

/** How many figures stand for a unit: one for a big creature, a ship, an engine or a hero-sized
 *  sprite (wider or taller than the unit contract's 96 x 120), two for a settler or a worker, three
 *  for the rest (a company). */
function groupSize(info, art) {
  if (!art || art.w > 110 || art.h > 130) return 1;
  const dom = info ? tname("DomainType", info.domain) : null;
  const combat = info ? tname("UnitCombatType", info.unit_combat) : null;
  if (dom === "DOMAIN_SEA" || dom === "DOMAIN_AIR" || combat === "UNITCOMBAT_SIEGE" || combat === "UNITCOMBAT_NAVAL") return 1;
  if (info?.found || info?.worker) return 2;
  return 3;
}

/** The owner's banner as a sprite (drawn here, so the colour stays a runtime choice): a gonfalon on
 *  a pole -- the cloth in the owner's colour with a pale charge and gold trim, hanging from a
 *  crossbar under a gold finial. 40 x 150 art px, its pole's foot the card's base. */
export function bannerKey(atlas, css) {
  const key = `banner:${css}`;
  if (atlas.has(key)) return key;
  const W = 40, H = 150;
  const c = typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(W, H) : Object.assign(document.createElement("canvas"), { width: W, height: H });
  const g = c.getContext("2d");
  const px = (x, y, w, h, col) => {
    g.fillStyle = col;
    g.fillRect(x, y, w, h);
  };
  const [r, gg, b] = hexRgb(cssHex(css)).map((v) => v * 255);
  const shade = (k) => `rgb(${Math.round(r * k)},${Math.round(gg * k)},${Math.round(b * k)})`;
  // Pole and finial.
  px(4, 8, 4, 142, "#4a2c14");
  px(4, 8, 1, 142, "#8a5a2c");
  px(3, 2, 6, 6, "#f0c040");
  px(4, 1, 4, 1, "#fff0a0");
  px(3, 7, 6, 1, "#8a5a10");
  // Crossbar.
  px(4, 12, 32, 3, "#6a4018");
  px(34, 11, 3, 5, "#f0c040");
  // The cloth, a swallow tail at its foot.
  const top = 15, bottom = 92, L = 9, R = 35;
  for (let y = top; y < bottom; y++) {
    const notch = y > bottom - 14 ? Math.round(((y - (bottom - 14)) / 14) * 7) : 0;
    for (let x = L; x < R; x++) {
      const mid = (L + R) / 2;
      if (notch && Math.abs(x + 0.5 - mid) < notch) continue;
      const edge = x === L || x === R - 1 || y === top;
      const fold = (x - L) % 9 < 2 ? 0.86 : 1;
      px(x, y, 1, 1, edge ? "#e8b838" : shade(fold * (y < top + 3 ? 1.12 : 1)));
    }
  }
  // The charge: a pale four-pointed star in a ring.
  const cx = 22, cy = 44;
  const pale = "#f4eedc";
  for (let k = -9; k <= 9; k++) {
    const t = 9 - Math.abs(k);
    px(cx + k, cy - Math.floor(t / 4), 1, Math.floor(t / 2) + 1, pale);
    px(cx - Math.floor(t / 4), cy + k, Math.floor(t / 2) + 1, 1, pale);
  }
  px(cx - 2, cy - 2, 5, 5, pale);
  g.strokeStyle = "rgba(244,238,220,0.9)";
  g.lineWidth = 1;
  g.beginPath();
  g.arc(cx + 0.5, cy + 0.5, 12, 0, Math.PI * 2);
  g.stroke();
  // Gold trim across the cloth's foot.
  px(L, 70, R - L, 2, "#e8b838");
  atlas.put(key, c);
  return key;
}

/** "#rrggbb" for any CSS colour tiles.js gives (#rgb, #rrggbb, hsl()). */
function cssHex(css) {
  if (/^#[0-9a-f]{6}$/i.test(css)) return css;
  if (/^#[0-9a-f]{3}$/i.test(css)) return `#${[...css.slice(1)].map((c) => c + c).join("")}`;
  const c = (typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(1, 1) : document.createElement("canvas")).getContext("2d");
  c.fillStyle = css;
  return c.fillStyle.startsWith("#") ? c.fillStyle : "#888888";
}

/** A card record. */
function card(key, art, px, x, y, z, o = {}) {
  return { key, x, y, z, w: art.w * px, h: art.h * px, anchor: 0.5, flip: false, banner: false, shadow: false, team: [1, 1, 1], bias: 0, id: 0, glow: 0, phase: 0, ...o };
}

/** The mana school of a node or a resource, from its name (`IMPROVEMENT_MANA_FIRE`), else null. */
function manaColour(name) {
  const m = /MANA_([A-Z]+)/.exec(name ?? "");
  return m ? MANA[m[1]] ?? "#b48aff" : /MANA/.test(name ?? "") ? "#b48aff" : null;
}

/** The variants of a prop card (`tree_oak` -> tree_oak_1, tree_oak_2, ...), from the art index. */
const variantsOf = new Map();
function variants(kind) {
  let v = variantsOf.get(kind);
  if (!v) {
    // A kind names its numbered cards (`tree_oak` -> tree_oak_1, ...), or one card itself.
    v = Object.keys(CARDS).filter((k) => k.startsWith(`${kind}_`) && /^\d+$/.test(k.slice(kind.length + 1))).sort();
    if (!v.length && Object.hasOwn(CARDS, kind)) v = [kind];
    variantsOf.set(kind, v);
  }
  return v;
}

const WOODED = new Set([FEAT.FOREST, FEAT.JUNGLE, FEAT.ANCIENT, FEAT.BURNT, FEAT.YOUNG]);
/** A wood's trees against a lone prop's scale (PX.prop): smaller, many to a plot -- the look
 *  target's woods are crowds of trees about half a figure's height. */
const TREE = 0.58;
/** The props drawn by code (tools/hd2d/draw.py: grass, reeds, pebbles, shore stones, standing
 *  stones, fences, haystacks) are small art drawn for about one art pixel a screen pixel at the
 *  gameplay zoom (96 px a plot): their scale against PX.prop. */
const FINE = 2.6;
const HELLISH = new Set([GROUND.BURNING_SANDS, GROUND.BROKEN_LANDS, GROUND.PERDITION]);

/** The trees of a wooded plot, by its feature and ground: [kind, weight, scale]. */
function woodOf(feat, ground, hills) {
  const cold = ground === GROUND.TUNDRA || ground === GROUND.SNOW;
  if (feat === FEAT.JUNGLE) return [["tree_jungle", 3, 1], ["tree_oak", 1, 0.9]];
  if (feat === FEAT.ANCIENT) return [["tree_ancient", 1, 1], ["tree_oak", 2, 1]];
  if (feat === FEAT.BURNT || HELLISH.has(ground)) return [["tree_dead", 1, 0.9]];
  if (feat === FEAT.YOUNG) return [["tree_oak", 2, 0.62], ["bush", 1, 1.2]];
  if (cold) return [[ground === GROUND.SNOW ? "tree_pine_snow" : "tree_pine", 3, 1], ["tree_pine_snow", 1, 0.95]];
  return hills ? [["tree_pine", 2, 1], ["tree_oak", 1, 1]] : [["tree_oak", 3, 1], ["tree_pine", 1, 1]];
}

/** What scatters over open ground, by its ground and mood: [kind, how many (expected), scale]. The
 *  look target fills every plot with small things, in clumps: a lone tree with bushes and grass
 *  round it, a rock among pebbles. */
function clutterOf(kind, ground, mood) {
  const out = [];
  if (HELLISH.has(ground)) out.push(["lava_rock", 1.3, 1], ["tree_dead", 0.35, 0.75], ["pebbles", 1.5, FINE]);
  else if (ground === GROUND.SNOW) out.push(["tree_pine_snow", 0.6, 0.75], ["rock", 0.5, 1], ["pebbles", 1.0, FINE]);
  else if (ground === GROUND.TUNDRA) out.push(["tree_pine", 0.6, 0.75], ["rock", 0.7, 1], ["bush", 0.6, 0.85], ["grass", 2.0, FINE], ["pebbles", 1.0, FINE]);
  else if (ground === GROUND.DESERT) out.push(["rock", 0.6, 1], ["pebbles", 1.5, FINE], ["grass_3", 1.2, FINE]);
  else if (ground === GROUND.MARSH) out.push(["bush", 1.4, 0.9], ["reeds", 3.0, FINE], ["grass", 2.0, FINE]);
  else {
    const lush = ground === GROUND.GRASS;
    out.push(["bush", 1.6, 0.85], ["flowers", lush ? 1.6 : 0.7, 0.9], ["tree_oak", 0.7, 0.7], ["rock", 0.35, 0.8], ["grass", lush ? 4.5 : 3.0, FINE], ["pebbles", 0.8, FINE], ["stones", 0.05, FINE]);
    if (!lush) out.push(["grass_3", 1.5, FINE]);
  }
  if (kind === KIND.HILLS) out.push(["rock", 1.2, 1.0], ["pebbles", 1.5, FINE]);
  if (kind === KIND.PEAK) out.push(["rock", 2.2, 1.2], ["pebbles", 2.0, FINE]);
  // Each region's own things (the moods, hd2d/mood.js: [hell, corruption, lush, gloom]).
  if (HELLISH.has(ground)) out.push(["bones", 0.4, FINE], ["vent", 0.35, FINE]);
  if (mood && mood[1] >= 0.5 && !HELLISH.has(ground)) out.push(["crystal", 1.0, 0.9], ["obelisk", 0.14, FINE], ["altar", 0.06, FINE]);
  if (mood && mood[2] >= 0.5) out.push(["lantern", 0.14, FINE]);
  if (!HELLISH.has(ground) && ground !== GROUND.SNOW) out.push(["ruin", 0.025, FINE], ["shrine", 0.02, FINE]);
  return out;
}

/** Props that carry a light of their own (hut windows, runes, lamps, embers): their bright,
 *  saturated pixels feed the bloom. */
const GLOWS = { crystal: 1.3, lava_rock: 1.3, obelisk: 1.4, altar: 1.2, vent: 1.4, lantern: 1.4, hut: 0.6 };

/** A town spills over its walls, as the look target's do: on the open plots round it, huts,
 *  gardens, a well, fences and haystacks -- more on the ring next to it than on the one beyond.
 *  Returns how far the nearest city is (1 or 2 plots), or 0. */
function townRing(map, world, x, y) {
  let best = 0;
  for (let dy = -2; dy <= 2; dy++) {
    for (let dx = -2; dx <= 2; dx++) {
      const j = world.index(x + dx, y + dy);
      if (j < 0 || map.city[j] < 0) continue;
      const d = Math.max(Math.abs(dx), Math.abs(dy));
      if (d > 0 && (!best || d < best)) best = d;
    }
  }
  return best;
}

/** Props too low to cast a shadow worth the shadow map's while. */
const FLAT = new Set(["flowers", "grass", "grass_3", "pebbles", "garden", "bones"]);

/** Farmland's own clutter, near a town or on a farm, pasture or cottage plot: fences and haystacks. */
const FARMED = /FARM|PASTURE|COTTAGE|HAMLET|VILLAGE|PLANTATION|WINERY/;

/** The prop cards of the land: stands of single trees on wooded plots (a jittered three by three,
 *  thinned where something stands on the plot), bushes, flowers, rocks, crystals and lava rocks
 *  scattered on open ground -- off rivers, roads, towns and the spot where a stack stands. A pure
 *  function of the state and the plot index. */
function dressProps(world, map, atlas, env, moods, list) {
  const w = world.w;
  const H = (mx, my) => world.heightAt(mx, my);
  const cityNear = (x, y, lx, lz) => {
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const j = world.index(x + dx, y + dy);
        if (j < 0 || map.city[j] < 0) continue;
        // A settlement's card spans about 1.7 plots; keep its footprint (round its plot) clear.
        const ex = lx - dx, ez = lz + dy;
        if ((ex * ex) / (0.95 * 0.95) + ((ez - 0.15) * (ez - 0.15)) / (0.75 * 0.75) < 1) return true;
      }
    }
    return false;
  };
  for (let i = 0; i < w * world.h; i++) {
    const kind = world.kind[i];
    if (kind === KIND.VOID || kind === KIND.OCEAN || kind === KIND.COAST || kind === KIND.ICE || map.city[i] >= 0) continue;
    const x = i % w, y = (i - x) / w;
    const feat = world.feat[i];
    const r = rng(i * 7919 + 101);
    const held = env.unitAt.has(i) || map.improvement[i] >= 0;
    const road = map.route[i] >= 0;
    // Something stands on the plot behind (north): this plot's trees stand between it and the
    // camera, so its back row goes and its middle row keeps low.
    const n = world.index(x, y + 1);
    const northHeld = n >= 0 && (env.unitAt.has(n) || map.city[n] >= 0);
    const ok = (lx, lz) => {
      const mx = x + 0.5 + lx, my = y + 0.5 - lz;
      if (world.riverCells.size && world.riverDist(mx, my) < RIVER_HALF + 0.1) return false;
      if (road && (Math.abs(lx) < 0.12 || Math.abs(lz) < 0.12)) return false;
      if (held && Math.hypot(lx, lz - 0.08) < 0.3) return false;
      return !cityNear(x, y, lx, lz);
    };
    const put = (kindKey, lx, lz, scale) => {
      const vs = variants(kindKey);
      if (!vs.length || !ok(lx, lz)) return;
      const key = vs[r.int(0, vs.length - 1)];
      const art = atlas.info(key);
      const mx = x + 0.5 + lx, my = y + 0.5 - lz;
      const glow = GLOWS[kindKey] ?? (kindKey.startsWith("hut") ? GLOWS.hut : 0);
      list.push(card(key, art, PX.prop * scale, mx, H(mx, my) - 0.01, -my, { shadow: !FLAT.has(kindKey), flip: r() < 0.5, glow, id: 0.0002, vis: [x + 0.5, y + 0.5] }));
    };
    if (WOODED.has(feat)) {
      const wood = woodOf(feat, world.ground[i], kind === KIND.HILLS);
      const total = wood.reduce((s, [, wt]) => s + wt, 0);
      // The giants of an ancient wood stand one or two to a plot, in its middle.
      if (feat === FEAT.ANCIENT && variants("tree_ancient").length) {
        put("tree_ancient", r.range(-0.12, 0.12), r.range(-0.18, 0.0), r.range(0.95, 1.15));
      }
      // A crowd of small trees, as the look target's woods: five by five, jittered.
      for (let gy = 0; gy < 5; gy++) {
        for (let gx = 0; gx < 5; gx++) {
          let pick = r() * total, k = wood[0];
          for (const it of wood) if ((pick -= it[1]) <= 0) { k = it; break; }
          if (k[0] === "tree_ancient") k = ["tree_oak", 1, 1];
          const s = TREE * k[2] * r.range(0.8, 1.18);
          const lx = (gx - 2) * 0.19 + r.range(-0.055, 0.055), lz = (gy - 2) * 0.19 + r.range(-0.055, 0.055);
          if (northHeld && gy === 0) continue;
          put(k[0], lx, lz, northHeld && gy === 1 ? s * 0.7 : s);
        }
      }
      if (r() < 0.7) put("bush", r.range(-0.42, 0.42), r.range(-0.42, 0.42), r.range(0.7, 1.0));
      // The wood's floor at its edge: grass and a fern-like bush where light reaches.
      for (let k = 0; k < 2; k++) put("grass", r.range(-0.46, 0.46), r.range(0.3, 0.48), FINE * r.range(0.85, 1.1));
    } else if (kind === KIND.LAND || kind === KIND.HILLS || kind === KIND.PEAK) {
      // Clumps: most things gather round one or two spots of the plot, the rest lie anywhere.
      const clumps = [[r.range(-0.3, 0.3), r.range(-0.3, 0.3)], [r.range(-0.3, 0.3), r.range(-0.3, 0.3)]];
      const at = () => {
        if (r() < 0.3) return [r.range(-0.44, 0.44), r.range(-0.44, 0.44)];
        const c = clumps[r() < 0.6 ? 0 : 1];
        return [c[0] + r.range(-0.16, 0.16), c[1] + r.range(-0.14, 0.14)];
      };
      for (const [k, n, s] of clutterOf(kind, world.ground[i], moods?.[i])) {
        let count = Math.floor(n) + (r() < n - Math.floor(n) ? 1 : 0);
        while (count-- > 0) put(k, ...at(), s * r.range(0.8, 1.15));
      }
      // Farmland and a town's outskirts: fences along a side of the plot, a haystack or two,
      // and next to the town its houses and gardens.
      const imp = map.improvement[i] >= 0 ? tname("ImprovementType", map.improvement[i]) : "";
      const ring = HELLISH.has(world.ground[i]) ? 0 : townRing(map, world, x, y);
      if (FARMED.test(imp) || (ring && r() < 0.6)) {
        if (r() < 0.75) put("fence", r.range(-0.2, 0.2), r() < 0.5 ? 0.42 : -0.4, FINE);
        if (r() < 0.55) put("hay", r.range(-0.35, 0.35), r.range(-0.1, 0.38), FINE * r.range(0.85, 1.05));
      }
      if (ring && !imp) {
        const huts = ring === 1 ? (r() < 0.75 ? 2 : 1) : r() < 0.45 ? 1 : 0;
        // Two houses keep to either half of the plot, so they never stand in each other.
        for (let k = 0; k < huts; k++) put(r() < 0.25 ? "hut_stone" : "hut", huts === 1 ? r.range(-0.3, 0.3) : k ? r.range(0.08, 0.32) : r.range(-0.32, -0.08), r.range(-0.3, 0.3), FINE * r.range(0.9, 1.05));
        if (r() < (ring === 1 ? 0.5 : 0.25)) put("garden", r.range(-0.25, 0.25), r.range(-0.3, 0.35), FINE);
        if (ring === 1 && r() < 0.12) put("well", r.range(-0.3, 0.3), r.range(-0.2, 0.3), FINE);
        // (Kinds the image model may add, tools/hd2d/props.json: none stand until their cards exist.)
        if (r() < 0.12) put("cart", r.range(-0.3, 0.3), r.range(-0.2, 0.35), FINE);
      }
    }
  }
}

/** The look target's land ends at its water in a rim of dark, wet stones: rows of them along the
 *  sea's and the lakes' shores (where the ground falls through the waterline) and along the river
 *  banks, with reeds here and there. One stone a cell of about a fifth of a plot at most (a global
 *  grid, so two plots never stack theirs); a pure function of the state. */
function dressShores(world, map, atlas, list) {
  const w = world.w;
  const taken = new Set();
  const cellOf = (mx, my) => `${Math.floor(mx / 0.17)},${Math.floor(my / 0.17)}`;
  const place = (kindKey, mx, my, y, scale, r) => {
    const vs = variants(kindKey);
    const c = cellOf(mx, my);
    if (!vs.length || taken.has(c)) return;
    taken.add(c);
    const key = vs[r.int(0, vs.length - 1)];
    const art = atlas.info(key);
    list.push(card(key, art, PX.prop * scale, mx, y, -my, { flip: r() < 0.5, id: 0.0002, vis: [Math.floor(mx) + 0.5, Math.floor(my) + 0.5] }));
  };
  const dry = (k) => k === KIND.LAND || k === KIND.HILLS || k === KIND.PEAK;
  for (let i = 0; i < w * world.h; i++) {
    const kind = world.kind[i];
    if (!dry(kind) || map.city[i] >= 0) continue;
    const x = i % w, y = (i - x) / w;
    const r = rng(i * 104729 + 7);
    let coast = false;
    for (let dy = -1; dy <= 1 && !coast; dy++) for (let dx = -1; dx <= 1; dx++) if ((dx || dy) && world.index(x + dx, y + dy) >= 0 && world.isWater(world.index(x + dx, y + dy))) coast = true;
    const river = world.riverCells.size > 0 && world.riverDist(x + 0.5, y + 0.5) < 1.2;
    if (!coast && !river) continue;
    const road = map.route[i] >= 0;
    // Over the plot and a quarter plot past its edges (the warped coast wanders off the grid).
    for (let gy = 0; gy < 14; gy++) {
      for (let gx = 0; gx < 14; gx++) {
        const mx = x - 0.25 + (gx + r.range(0.2, 0.8)) * 0.107, my = y - 0.25 + (gy + r.range(0.2, 0.8)) * 0.107;
        const own = world.index(Math.floor(mx), Math.floor(my));
        // A point past the edge belongs to this plot only if it lies over water (or the next plot
        // is not land, whose own pass draws it).
        if (own !== i && (own < 0 || dry(world.kind[own]))) continue;
        if (road && Math.abs(mx - x - 0.5) < 0.12) continue;
        if (coast) {
          const h = world.heightAt(mx, my);
          if (h > -0.004 && h < 0.02 && r() < 0.8) {
            // Mostly rows of shore stones; now and then a boulder of the bank, its rocky edge.
            const k = r();
            if (k < 0.12) place("rock", mx, my, Math.max(SEA, h) - 0.006, r.range(0.38, 0.5), r);
            else place(k < 0.85 ? "shore" : "pebbles", mx, my, Math.max(SEA, h) - 0.004, FINE * r.range(0.8, 1.05), r);
            continue;
          }
        }
        if (river) {
          const d = world.riverDist(mx, my);
          if (d > RIVER_HALF + 0.01 && d < RIVER_HALF + 0.055 && r() < 0.55) {
            const k = r() < 0.15 ? "reeds" : r() < 0.7 ? "shore" : "pebbles";
            place(k, mx, my, world.heightAt(mx, my) - 0.004, FINE * (k === "shore" ? r.range(0.55, 0.8) : r.range(0.8, 1)), r);
          }
        }
      }
    }
  }
}

/** Waterfalls: where a river's surface (as scene.js riverLevel lays it) drops steeply toward the
 *  camera -- over a raised bank into the sea (world.js `bank`), or off the hills -- a card of white
 *  water stands across it, its foot on the lower water. A river running across the view keeps its
 *  slope unmarked (a card would face the wrong way). One a stretch of 0.7 plot at most. */
function dressFalls(world, list, atlas) {
  const vs = variants("falls");
  if (!vs.length || !world.rivers?.length) return;
  const level = (mx, my) => Math.max(SEA + 0.004, world.heightAt(mx, my) + 0.045);
  const placed = [];
  world.rivers.forEach((line, li) => {
    const r = rng(li * 7907 + 13);
    // Each point against the one about 0.2 plot on (the lines are subdivided every 0.1 plot).
    for (let i = 0; i + 2 < line.length; i++) {
      let [px, py] = line[i], [qx, qy] = line[i + 2];
      let hi = level(px, py), lo = level(qx, qy);
      // The falls face the camera: the water drops toward the south (smaller y), down the view.
      if (lo > hi) [px, py, qx, qy, hi, lo] = [qx, qy, px, py, lo, hi];
      const len = Math.hypot(qx - px, qy - py);
      // On land: at a river's mouth its ribbon widens flat into the sea, and a wall of white water
      // there would stand over nothing.
      if (len < 1e-6 || (py - qy) / len < 0.55 || hi - lo < 0.022 || lo < SEA + 0.012) continue;
      if (placed.some(([x, y]) => Math.hypot(x - px, y - py) < 0.7)) continue;
      const j = world.index(Math.floor(qx), Math.floor(qy));
      if (j < 0 || world.kind[j] === KIND.VOID) continue;
      placed.push([px, py]);
      const key = vs[r.int(0, vs.length - 1)];
      const art = atlas.info(key);
      // As wide as the river's water (2 RIVER_HALF, its edge wandering RIVER_WANDER), about as tall
      // as the slope it covers looks from the camera.
      const c = card(key, art, 1, qx, lo - 0.01, -qy, { id: 0.0002, bias: 0.04, vis: [qx, qy] });
      c.w = 2 * (RIVER_HALF + RIVER_WANDER);
      c.h = clamp(0.05 + (hi - lo) * 2.2, 0.08, 0.2);
      list.push(c);
    }
  });
}

/** The static cards (cities, standing improvements, resources and their banners, the land's prop
 *  cards) and the light beams over mana nodes and towers; `claims` names what the 3D dressing no
 *  longer models (with prop cards: its trees, plants and rocks). */
export function dressSpriteStatic(world, map, atlas, env = null, moods = null) {
  const list = [];
  const beams = [];
  const claims = { city: new Set(), improvement: new Set(), bonus: new Set() };
  if (env && Object.keys(CARDS).length) {
    dressProps(world, map, atlas, env, moods, list);
    dressShores(world, map, atlas, list);
    dressFalls(world, list, atlas);
    claims.trees = claims.flora = claims.rocks = true;
  }
  const w = world.w;
  const H = (mx, my) => world.heightAt(mx, my);
  const cityAt = new Map();
  for (const c of S.cities.values()) cityAt.set(c.y * w + c.x, c);
  for (let i = 0; i < w * world.h; i++) {
    if (world.kind[i] === KIND.VOID) continue;
    const x = i % w, y = (i - x) / w;
    const at = (lx, lz) => [x + 0.5 + lx, y + 0.5 - lz];
    const city = cityAt.get(i);
    if (city) {
      const key = first(atlas, ISO_TABLES.citySprites(city.size ?? 1, civShortName(city.owner)));
      if (key) {
        // The card's foot on the plot's front edge: its painted ground then lies over the plot.
        const art = atlas.info(key);
        const [mx, my] = at(0, 0.5);
        // A town: graded lighter than the land (it keeps its colours), its windows and fires lit.
        list.push(card(key, art, PX.city, mx, H(x + 0.5, y + 0.5), -my, { id: idOf(`c${city.id}`), bias: BIAS.building, vis: [x + 0.5, y + 0.5], town: true, glow: 1.1 }));
        const css = teamCss(city.owner);
        const bk = bannerKey(atlas, css);
        const [bx, by] = at(-0.44, 0.38);
        list.push(card(bk, atlas.info(bk), PX.banner, bx, H(bx, by), -by, { anchor: 5.5 / 40, banner: true, team: hexRgb(cssHex(css)), bias: BIAS.unit, id: idOf(`cb${city.id}`), phase: hashf(city.id, 5) }));
        claims.city.add(i);
      }
    }
    const imp = map.improvement[i] >= 0 ? tname("ImprovementType", map.improvement[i]) : null;
    if (imp && !city) {
      const key = first(atlas, ISO_TABLES.improvementSprites(imp).filter((k) => k.startsWith("bld_") || k.startsWith("feat_")));
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        const [mx, my] = at(0, 0.5);
        const base = water ? SEA - DRAFT : H(x + 0.5, y + 0.5);
        const mana = manaColour(imp);
        list.push(card(key, art, PX.building, mx, base, -my, { id: idOf(`i${i}`), glow: mana ? 1.4 : 0, fade: !water, vis: [x + 0.5, y + 0.5] }));
        claims.improvement.add(i);
        const tower = /TOWER/.test(imp) && !/WATCH|WARNING/.test(imp);
        if (mana || tower) {
          const col = hexRgb(mana ?? "#ffe6a8");
          const [cx, cy] = at(0, 0.02);
          beams.push({ key: null, x: cx, y: H(cx, cy), z: -cy, w: 0.4, h: 1.5, anchor: 0.5, team: col, glow: mana ? 0.45 : 0.3, id: 0, phase: hashf(i, 9) });
        }
      }
    }
    const bonus = map.bonus[i] >= 0 ? tname("BonusType", map.bonus[i]) : null;
    if (bonus && !city) {
      const key = first(atlas, [ISO_TABLES.resourceSprite(bonus)]);
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        const [mx, my] = at(...BONUS_SPOT);
        const base = water ? SEA : H(mx, my);
        const glow = /MANA|GEMS|JEWELS|MITHRIL/.test(bonus) ? 1.2 : 0;
        list.push(card(key, art, PX.resource, mx, base, -my, { id: idOf(`b${i}`), glow, bias: BIAS.resource }));
        claims.bonus.add(i);
        if (/MANA/.test(bonus)) beams.push({ key: null, x: mx, y: base, z: -my, w: 0.26, h: 0.9, anchor: 0.5, team: hexRgb(manaColour(bonus) ?? "#b48aff"), glow: 0.3, id: 0, phase: hashf(i, 11) });
      }
    }
  }
  return { list, beams, claims };
}

/** The unit stacks in sight as groups of figures under their banners. Returns `{list, stacks}` in the
 *  shape of ../dress.js dressUnits (stacks: for the HUD's badges and the team rings). */
export function dressSpriteUnits(world, map, atlas) {
  const list = [];
  const stacks = [];
  const w = world.w;
  const mine = S.view?.me?.player;
  const sel = S.sel?.units ?? [];
  const strength = (u) => S.data?.tables?.UnitType?.[u.type]?.combat ?? 0;
  const sctx = {
    me: mine,
    selected: sel,
    unitInfo: (t) => {
      const s = strength({ type: t });
      return { hero: false, combat: s > 0 ? "COMBAT" : "NONE", strength: s };
    },
    activity: (a) => tname("ActivityType", a),
    automated: (g) => {
      const grp = S.groups?.get?.(g);
      return !!grp && grp.automated !== null && grp.automated !== undefined;
    },
  };
  const standing = new Set();
  for (let i = 0; i < map.improvement.length; i++) {
    if (map.improvement[i] < 0) continue;
    const imp = tname("ImprovementType", map.improvement[i]);
    if (ISO_TABLES.improvementSprites(imp).some((k) => (k.startsWith("bld_") || k.startsWith("feat_")) && atlas.has(k))) standing.add(i);
  }
  for (const units of S.unitsAt.values()) {
    const u0 = units[0];
    const i = u0.y * w + u0.x;
    const st = stackState(units, sctx);
    const top = st?.head ?? u0;
    const info = S.data?.tables?.UnitType?.[top.type] ?? null;
    const key = first(atlas, unitSpriteKeys(top.type, civShortName(top.owner)));
    if (!key) continue;
    const art = atlas.info(key);
    const onCity = map.city[i] >= 0;
    // A city's garrison is one figure at its front, so it does not crowd the town's walls and gate:
    // at the front-left corner, else the front-right or the front's middle, out of a river.
    const spot = onCity ? garrisonSpot(world, u0.x, u0.y) : world.feat[i] === FEAT.VOLCANO ? VOLCANO_SPOT : standing.has(i) ? BUILT_SPOT : SPOT;
    const n = onCity ? 1 : groupSize(info, art);
    const dom = info ? tname("DomainType", info.domain) : null;
    const css = teamCss(top.owner);
    const team = hexRgb(cssHex(css));
    const scale = onCity ? 0.8 : 1;
    let anchor = null;
    GROUP[n].forEach(([ox, oz], k) => {
      const lx = spot[0] + ox, lz = spot[1] + oz;
      const mx = u0.x + 0.5 + lx, my = u0.y + 0.5 - lz;
      const ground = Math.max(SEA, standHeight(world, mx, my, 0.06));
      const y = dom === "DOMAIN_SEA" ? SEA - DRAFT : dom === "DOMAIN_AIR" ? ground + 0.12 : ground;
      // The figures behind the first a touch smaller, as the 3D renderer's company.
      const s = scale * (k === 0 ? 1 : 0.94);
      const r = card(key, art, PX.unit * s, mx, y, -my, { shadow: true, team, bias: (onCity ? BIAS.garrison : BIAS.unit) + (k === 0 ? 0.02 : 0), id: idOf(`u${top.id}`), phase: hashf(top.id * 3 + k, 3) });
      r.uid = `${top.id}:${k}`;
      r.ground = dom !== "DOMAIN_SEA" && dom !== "DOMAIN_AIR";
      list.push(r);
      if (k === 0) anchor = { x: mx, y, z: -my, height: r.h * 0.85 };
    });
    // The banner behind the group's left, its cloth above the figures' heads.
    const bk = bannerKey(atlas, css);
    const bx = u0.x + 0.5 + spot[0] - (n === 1 ? Math.min(0.3, art.w * PX.unit * 0.42) : 0.26);
    const by = u0.y + 0.5 - (spot[1] - 0.11);
    const bg = Math.max(SEA, world.heightAt(bx, by));
    const b = card(bk, atlas.info(bk), PX.banner * scale, bx, dom === "DOMAIN_SEA" ? SEA : bg, -by, { anchor: 5.5 / 40, banner: true, team, bias: BIAS.unit, id: idOf(`ub${top.id}`), phase: hashf(top.id, 7) });
    b.uid = `${top.id}:b`;
    b.ground = dom !== "DOMAIN_SEA";
    list.push(b);
    stacks.push({ units, top, x: u0.x, y: u0.y, k: i, anchor, picked: sel.includes(top.id), team: css, spot, mark: st?.state?.mark ?? null, hero: st?.state?.hero ?? false });
  }
  return { list, stacks };
}
