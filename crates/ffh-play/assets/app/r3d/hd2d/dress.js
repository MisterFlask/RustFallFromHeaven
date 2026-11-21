// What the HD-2D mode stands in the world as sprites (docs/HD2D.md): the isometric renderer's art
// (docs/ISO.md), chosen as iso.js chooses it -- a unit's sprite by its civilization, type, class and
// stand-ins (iso.js unitSpriteKeys), an improvement's and a resource's by kind -- and the mode's own
// art (hd2d/art: prop cards, the town kit), placed on the 3D renderer's ground. A stack is a group of
// two or three figures of its head unit under the owner's banner; a city is a town composed of the
// kit's cards (§5c: its landmark, houses, towers and walls); a standing improvement and a resource
// are cards (a tribal village is five huts). The 3D models of what these replace are not placed (`claims`, read by
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
import { hashf, hash32, rng, noise2 } from "../noise.js";
import { SEA, DRAFT, FEAT, KIND, GROUND, RIVER_BANK, RIVER_HALF } from "../world.js";
import { CARDS, IMPROVEMENTS, RESOURCES } from "./art/index.js";
import { standHeight, hexRgb } from "../dress.js";
import { MANA } from "../assets/palette.js";
import { improvementContainsMana } from "../../mana-art.js";
import { CATALOG } from "../assets/catalog.js";
import { clamp } from "../math.js";
import { M } from "./mood.js";

/** World units per art pixel: figures (a 96 x 120 unit canvas is 0.46 x 0.58 of a plot), cities (a
 *  192-wide settlement 1.15 plots: its painted ground spans the plot), the other buildings, the
 *  resources' props, the banners. */
export const PX = { unit: 0.0048, city: 0.0088, building: 0.0038, resource: 0.0042, banner: 0.0053, prop: 0.0031 };

/** Bespoke HD-2D sites first; established works and mana nodes keep their art.
 *  Static dressing and unit clearance must use the same standing footprint. */
export function improvementSpriteKeys(type) {
  const key = IMPROVEMENTS[type];
  const fallback = ISO_TABLES.improvementSprites(type);
  return key ? [key, ...fallback] : fallback;
}
/** Depth toward the eye (world units): a figure before the card of the building it stands at, a
 *  resource on a standing improvement's plot (its ore before the mine) before that card. */
const BIAS = { unit: 0.26, garrison: 0.4, building: 0, resource: 0.08, builtResource: 0.18 };
/** Where a stack stands (plot-local x east, z south); on a city's plot, its front-left corner; on a
 *  standing improvement's, in front of it. */
const SPOT = [0, 0.06];
const BUILT_SPOT = [0.05, 0.3];
const VOLCANO_SPOT = [-0.14, 0.38];
const BONUS_SPOT = [-0.26, 0.2];
/** Where a resource lies on a standing improvement's plot: at the card's front-left, a little behind
 *  its foot (plot-local x, and z back from the foot), drawn before the card (BIAS.builtResource). */
const BUILT_BONUS_SPOT = [-0.28, 0.1];
/** Where a city's banner stands (plot-local): at its front-left. */
const BANNER_SPOT = [-0.44, 0.38];
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

/** The high, supported part of a hill's crown, kept out of river channels. Search in both
 *  directions: crowns are jittered and a fixed front edge is usually at the foot of the hill. */
function hilltopFoot(world, x, y, radius = 0.08) {
  let best = [0, 0, world.heightAt(x + 0.5, y + 0.5)], bestScore = -Infinity;
  for (let dz = -3; dz <= 3; dz++) for (let dx = -3; dx <= 3; dx++) {
    const lx = dx * 0.08, lz = dz * 0.08;
    const mx = x + 0.5 + lx, my = y + 0.5 - lz;
    // (Its sides, its back and its front: the front a little further, where a crown ends in its
    // scarp and a river may run under it.)
    const samples = [[mx, my], [mx - radius, my], [mx + radius, my], [mx, my + radius], [mx, my - radius * 1.2]];
    if (world.riverCells.size && samples.some(([sx, sy]) => world.riverDist(sx, sy) < RIVER_BANK + 0.02)) continue;
    const heights = samples.map(([sx, sy]) => world.heightAt(sx, sy));
    const spread = Math.max(...heights) - Math.min(...heights);
    const score = heights[0] - spread * 0.4 - Math.hypot(lx, lz) * 0.01;
    if (score > bestScore) { best = [lx, lz, heights[0]]; bestScore = score; }
  }
  return best;
}

/** Where a standing work's card stands on a hill, [lx, lz, base]. Its art paints a diamond of
 *  ground half as deep as the card is wide, which an upright card shows as ground reaching back
 *  from its foot (its corners `halfW` to each side, its back tip `2 * halfW` behind). Standing at
 *  the summit (`hilltopFoot`), that diamond lay over land falling away behind and beside it: the
 *  work floated over the hill. So the foot goes where the ground under the whole diamond is
 *  flattest, on the plot's own level (one crown or the shelf, never across a scarp, off the water
 *  and out of river channels), and the card stands at the mean height of that ground: its painted
 *  ground lies on the land, the front tip a little into it rather than over it. */
function hillWorkFoot(world, x, y, halfW) {
  const D = 2 * halfW;
  // (The diamond's outline and middle: the front tip, the corners, the back tip and between.)
  const pts = [[0, 0], [0, 0.5], [0, 1], [-1, 0.5], [1, 0.5], [-0.5, 0.25], [0.5, 0.25], [-0.5, 0.75], [0.5, 0.75]];
  let best = null, bestScore = -Infinity;
  for (let lz = 0.46; lz >= -0.1 + D - 0.5; lz -= 0.04) {
    for (let lx = -0.12; lx <= 0.121; lx += 0.06) {
      let lo = Infinity, hi = -Infinity, sum = 0, ok = true;
      let crown = null;
      for (const [u, v] of pts) {
        const sx = x + 0.5 + lx + u * halfW, sy = y + 0.5 - lz + v * D;
        if (world.riverCells.size && world.riverDist(sx, sy) < RIVER_BANK + 0.02) { ok = false; break; }
        const h = world.heightAt(sx, sy);
        if (world.cliff > 0) {
          // On the land, and all of it on one level: a crown or the shelf below its scarp.
          const up = world.scarp > 0;
          if (world.shore <= 0.012 || (crown !== null && up !== crown)) { ok = false; break; }
          crown = up;
        }
        lo = Math.min(lo, h);
        hi = Math.max(hi, h);
        sum += h;
      }
      if (!ok) continue;
      // Flattest first; then on a crown rather than the shelf, high, and near the plot's front.
      const mean = sum / pts.length;
      const score = -(hi - lo) + (crown ? 0.05 : 0) + mean * 0.02 - Math.abs(lz - 0.3) * 0.02 - Math.abs(lx) * 0.02;
      if (score > bestScore) { bestScore = score; best = [lx, lz, mean]; }
    }
  }
  // (Nowhere clear on the plot: the summit, as before.)
  return best ?? hilltopFoot(world, x, y, halfW * 0.5);
}

/** The painted ground of an isometric work's card (`c`, art `art`: its diamond, as wide as the
 *  art, half as deep, its tip at the opaque bounds' foot) laid on the land: a grid of vertices,
 *  `cols` across the card and, up it, rows on the ground from the diamond's front tip to its
 *  corners (each at the height of the ground there, `heightAt`, as the terrain's mesh), then one
 *  row at the card's top standing over the corners' row. Each vertex: [u across the card, y up it,
 *  world x, y, z, its height over that place in the card's height] (hd2d/sprites.js buildDraped,
 *  shaders.js SPRITE_VS). An upright card's painted ground reads as level ground at its foot's
 *  height, which on a hill's dome hung over the land falling away round it (floating, more so at
 *  the close zooms' lower camera); laid on the land it follows the dome. */
export function drapeGrid(world, c, art) {
  const cols = 7, ground = 5;
  const tip = (art.h - (art.box?.[3] ?? art.h)) / art.h;
  const corners = Math.min(0.9, tip + art.w / 4 / art.h);
  const half = c.w / 2;
  const v = new Float32Array(cols * (ground + 1) * 6);
  const anchor = c.anchor ?? 0.5;
  for (let j = 0; j <= ground; j++) {
    const top = j === ground;
    const y = top ? 1 : (corners * j) / (ground - 1);
    const back = top ? half : Math.min(1, Math.max(0, (y - tip) / (corners - tip))) * half;
    for (let i = 0; i < cols; i++) {
      const u = i / (cols - 1);
      const x = c.x + (u - anchor) * c.w, z = c.z - back;
      const k = (j * cols + i) * 6;
      v.set([u, y, x, world.heightAt(x, -z) + DRAPE_LIFT, z, top ? 1 - corners : 0], k);
    }
  }
  return { cols, rows: ground + 1, v };
}
/** How far a draped card's ground lies over the land (the terrain's mesh runs straight between its
 *  vertices, a sixth of a plot apart, a little under the ground's height on a dome). */
const DRAPE_LIFT = 0.006;

/** A spot of land plot (x, y), plot-local [x east, z south], kept on the plot's own level of land
 *  where the ground steps (world.js `cliff`): drawn toward the plot's middle until it is off the
 *  water, back from the rim of the land's face, and off a neighbouring hill's crown. (A plot's
 *  middle is always its own land.) */
function onLand(world, x, y, lx, lz, reach = 0) {
  if (!(world.cliff > 0)) return [lx, lz];
  const crown = world.kind[world.index(x, y)] === KIND.HILLS;
  const dry = (ax, az) => {
    world.heightAt(x + 0.5 + ax, y + 0.5 - az, false);
    return world.shore > 0.012 && (crown || world.scarp <= 0);
  };
  // (`reach`: what stands there is that wide -- a group of figures: its sides and front as well.)
  for (let k = 0; k < 8; k++) {
    if (dry(lx, lz) && (!reach || (dry(lx - reach, lz) && dry(lx + reach, lz) && dry(lx, lz + reach)))) break;
    lx *= 0.8;
    lz *= 0.8;
  }
  return [lx, lz];
}

/** On level land the painted ground reaches back from the front edge; retreat off a river, and off
 *  the water and a neighbouring hill's crown where the ground steps (`onLand`). */
function builtFoot(world, x, y) {
  let lz = 0.5;
  for (let k = 0; k < 6 && world.riverCells.size && world.riverDist(x + 0.5, y + 0.5 - lz) < RIVER_BANK; k++) lz -= 0.05;
  lz = onLand(world, x, y, 0, lz)[1];
  return [0, lz, world.heightAt(x + 0.5, y + 0.5 - lz, false)];
}

/** Five separate households around a small open centre, rather than one oversized tribal hut.
 *  Each card has its own ground contact, including on a hill's crown. */
const TRIBAL_HUTS = [[-0.23, -0.16, 0.46], [0.03, -0.21, 0.52], [0.25, -0.10, 0.44], [-0.19, 0.17, 0.49], [0.16, 0.21, 0.47]];

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
  // (In memory: a 2D canvas on the GPU is read back to upload.)
  const g = c.getContext("2d", { willReadFrequently: true });
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

/** A sea's buoy as a sprite (the Sea layer, docs/NAVAL.md 10): a tarred float with a gold band, a
 *  pole and the holder's banner -- as the owner's banner, smaller -- or, for a contested sea, two
 *  poles crossed over the float, yours on the left and the other party's on the right. 56 x 120
 *  art px (72 wide crossed), the float's middle the card's base; its foot is under the water. */
export function seaBuoyKey(atlas, cssA, cssB = null) {
  const key = `seabuoy:${cssA}:${cssB ?? ""}`;
  if (atlas.has(key)) return key;
  const W = cssB ? 72 : 56, H = 120;
  const c = typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(W, H) : Object.assign(document.createElement("canvas"), { width: W, height: H });
  const g = c.getContext("2d", { willReadFrequently: true });
  const px = (x, y, w, h, col) => {
    g.fillStyle = col;
    g.fillRect(Math.round(x), Math.round(y), w, h);
  };
  const cx = W / 2;
  // A pole from its foot (x0, 104) to its top (x1, 12), a pixel column a row.
  const pole = (x0, x1) => {
    for (let y = 12; y <= 104; y++) {
      const x = x0 + ((x1 - x0) * (104 - y)) / 92;
      px(x - 1, y, 3, 1, "#4a2c14");
      px(x - 1, y, 1, 1, "#8a5a2c");
    }
    px(x1 - 2, 6, 5, 6, "#f0c040");
    px(x1 - 1, 5, 3, 1, "#fff0a0");
  };
  // A cloth hanging from the top of a pole at x, to the right (dir 1) or the left (-1).
  const cloth = (x, dir, css) => {
    const [r, gg, b] = hexRgb(cssHex(css)).map((v) => v * 255);
    const shade = (k) => `rgb(${Math.round(r * k)},${Math.round(gg * k)},${Math.round(b * k)})`;
    const L = dir > 0 ? x + 1 : x - 21, R = L + 20;
    px(Math.min(x, L), 14, 22, 2, "#6a4018");
    const top = 16, bottom = 58;
    for (let y = top; y < bottom; y++) {
      const notch = y > bottom - 10 ? Math.round(((y - (bottom - 10)) / 10) * 5) : 0;
      for (let xx = L; xx < R; xx++) {
        const mid = (L + R) / 2;
        if (notch && Math.abs(xx + 0.5 - mid) < notch) continue;
        const edge = xx === L || xx === R - 1 || y === top;
        const fold = (xx - L) % 7 < 2 ? 0.86 : 1;
        px(xx, y, 1, 1, edge ? "#e8b838" : shade(fold * (y < top + 3 ? 1.12 : 1)));
      }
    }
    // The charge: a small pale star.
    const sx = (L + R) / 2 - 0.5, sy = 32;
    for (let k = -5; k <= 5; k++) {
      const t = 5 - Math.abs(k);
      px(sx + k, sy - Math.floor(t / 3), 1, Math.floor(t / 1.5) + 1, "#f4eedc");
      px(sx - Math.floor(t / 3), sy + k, Math.floor(t / 1.5) + 1, 1, "#f4eedc");
    }
    px(L, 48, R - L, 2, "#e8b838");
  };
  if (cssB) {
    pole(cx + 5, cx - 16);
    pole(cx - 5, cx + 16);
    cloth(cx - 16, -1, cssA);
    cloth(cx + 16, 1, cssB);
  } else {
    pole(cx - 1, cx - 1);
    cloth(cx - 1, 1, cssA);
  }
  // The float: a squat tarred cask, planked, a gold band round it, its foot in the water.
  for (let y = 98; y < 120; y++) {
    const t = (y - 108.5) / 11;
    const half = Math.round(13 * Math.sqrt(Math.max(0, 1 - t * t * 0.55)));
    for (let x = -half; x < half; x++) {
      const plank = ((x + 40) % 5 === 0) ? 0.75 : 1;
      const lit = x < -half / 3 ? 1.25 : x > half / 2 ? 0.75 : 1;
      const v = Math.round(58 * plank * lit);
      px(cx + x, y, 1, 1, `rgb(${v},${Math.round(v * 0.8)},${Math.round(v * 0.62)})`);
    }
  }
  px(cx - 12, 105, 24, 2, "#d8a838");
  px(cx - 12, 105, 6, 1, "#fff0a0");
  px(cx - 9, 98, 18, 1, "#2a1e14");
  atlas.put(key, c);
  return key;
}

/** The buoys of the seas held, contested or patrolled (r3d/sealayer.js `banners`), as cards: each
 *  stands in the water at its spot, its banner rippling. */
export function dressSeaBuoys(buoys, atlas) {
  const out = [];
  for (const b of buoys ?? []) {
    const key = seaBuoyKey(atlas, b.a, b.b);
    const r = card(key, atlas.info(key), PX.banner * 1.25, b.x, SEA - 0.04, -b.y, { anchor: 0.5, banner: true, team: hexRgb(cssHex(b.a)), bias: BIAS.unit, id: idOf(`sea${b.id}`), phase: hashf(b.id, 11) });
    r.uid = `sea:${b.id}`;
    r.ground = false;
    out.push(r);
  }
  return out;
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
/** The trees drawn by code (tools/hd2d/trees.py: oaks, pines, snowy pines) are about 45 art
 *  pixels tall against the image model's 150: their scale against the other trees', for an oak
 *  about a third of a plot tall (near one art pixel a screen pixel at the gameplay zoom). */
const DRAWN = { tree_oak: 4.6, tree_pine: 4.4, tree_pine_snow: 4.4 };
/** The props drawn by code (tools/hd2d/draw.py: grass, reeds, standing stones, fences;
 *  tools/hd2d/rocks.py: boulders, shore stones, pebbles, haystacks) are small art drawn for about one art pixel a screen pixel at the
 *  gameplay zoom (96 px a plot): their scale against PX.prop. */
const FINE = 2.6;
/** Open ground's grass tufts, smaller than the other small props: they say what the ground is
 *  without standing out of it (at FINE they read as dark stumps under the grade). */
const TUFT = FINE * 0.7;
/** The image model's props of the region moods and the civilizations (tools/hd2d/gen.py sheets
 *  `frost_arcane`, `holy_ash`, `folk`) are carded at their height in art pixels at this scale. */
const PAINTED = 1;
const HELLISH = new Set([GROUND.BURNING_SANDS, GROUND.BROKEN_LANDS, GROUND.PERDITION]);

/** The trees of a wooded plot, by its feature and ground: [kind, weight, scale]. */
function woodOf(feat, ground, hills) {
  const cold = ground === GROUND.TUNDRA || ground === GROUND.SNOW;
  if (feat === FEAT.JUNGLE) return [["tree_jungle", 3, 1], ["tree_oak", 1, 0.9]];
  if (feat === FEAT.ANCIENT) return [["tree_ancient", 1, 1], ["tree_oak", 2, 1]];
  if (feat === FEAT.BURNT || HELLISH.has(ground)) return [["tree_dead", 1, 0.9]];
  if (feat === FEAT.YOUNG) return [["tree_oak", 2, 0.62], ["bush", 1, 1.2]];
  if (cold) return [[ground === GROUND.SNOW ? "tree_pine_snow" : "tree_pine", 3, 1], ["tree_pine_snow", 1, 0.95]];
  return hills ? [["tree_pine", 1, 1], ["tree_oak", 1, 1]] : [["tree_oak", 3, 1], ["tree_pine", 1, 1]];
}

/** What scatters over open ground, by its ground and mood: [kind, how many (expected), scale].
 *  Only what says what the ground is: green tufts and flowers on grassland, dry tufts on plains,
 *  reeds on marsh, moss on tundra, stones on improved plots, the
 *  regions' own things (lava rocks, crystals, ...). No lone trees, bushes, boulders, ruins or
 *  standing stones on open ground: they made grassland and plains read as woods, hills or a site
 *  (the owner's call, 2026-11-01). */
function clutterOf(kind, ground, mood) {
  const out = [];
  if (HELLISH.has(ground)) out.push(["lava_rock", 1.3, 1], ["pebbles", 1.5, FINE]);
  else if (ground === GROUND.SNOW) out.push(["pebbles", 0.8, FINE]);
  else if (ground === GROUND.TUNDRA) out.push(["grass", 1.5, TUFT], ["pebbles", 1.0, FINE]);
  else if (ground === GROUND.DESERT) out.push(["pebbles", 1.5, FINE]);
  else if (ground === GROUND.MARSH) out.push(["reeds", 3.5, FINE], ["grass", 1.0, TUFT]);
  else if (ground === GROUND.GRASS) out.push(["grass", 3.0, TUFT], ["flowers", 1.2, 0.9]);
  else out.push(["grass_3", 3.5, TUFT]);
  // Hills are turf-capped scarps in the terrain itself; their silhouette carries the elevation.
  // (A peak carries no cards: its mountain is a model, assets/props.js feat.mountain.)
  // Each region's own things (the moods, hd2d/mood.js MOODS: hell, corruption, lush, gloom, frost,
  // arcane, holy, ash), where its mood is at least half.
  if (HELLISH.has(ground)) out.push(["bones", 0.4, FINE], ["vent", 0.35, FINE]);
  if (!mood) return out;
  if (mood[M.corruption] >= 0.5 && !HELLISH.has(ground)) out.push(["crystal", 1.0, 0.9], ["obelisk", 0.14, FINE], ["altar", 0.06, FINE]);
  if (mood[M.lush] >= 0.5) out.push(["lantern", 0.14, FINE]);
  if (mood[M.frost] >= 0.5 && !HELLISH.has(ground)) out.push(["ice_shard", 0.45, PAINTED], ["frost_shrub", 0.25, PAINTED], ["frost_cairn", 0.08, PAINTED]);
  if (mood[M.arcane] >= 0.5) out.push(["rune_stone", 0.22, PAINTED], ["glyph_ring", 0.06, PAINTED], ["arcane_obelisk", 0.07, PAINTED], ["float_crystal", 0.05, PAINTED], ["crystal_pillar", 0.04, PAINTED]);
  if (mood[M.holy] >= 0.5) out.push(["holy_shrine", 0.06, PAINTED], ["brazier", 0.07, PAINTED], ["angel_statue", 0.03, PAINTED], ["marble_column", 0.05, PAINTED]);
  if (mood[M.ash] >= 0.5) out.push(["soot_rocks", 0.45, PAINTED], ["slag", 0.25, PAINTED], ["coal", 0.15, PAINTED], ["ash_mound", 0.15, PAINTED], ["ash_vent", 0.1, PAINTED]);
  return out;
}

/** The civilizations' own things on the land they hold (hd2d/mood.js gives a plot its owner's
 *  civilization): [kind, how many (expected), scale, where]. `where`: "hills" on hills only, "coast"
 *  on land next to the sea only, else any open ground. */
const CIV_PROPS = {
  CIVILIZATION_CLAN_OF_EMBERS: [["totem", 0.1, PAINTED], ["skull_pole", 0.13, PAINTED]],
  CIVILIZATION_DOVIELLO: [["totem", 0.07, PAINTED], ["skull_pole", 0.1, PAINTED]],
  CIVILIZATION_CALABIM: [["gallows", 0.03, PAINTED], ["red_lantern", 0.1, PAINTED]],
  CIVILIZATION_BALSERAPHS: [["mask_pole", 0.11, PAINTED], ["carnival_stall", 0.06, PAINTED]],
  CIVILIZATION_KHAZAD: [["mine_entrance", 0.3, PAINTED, "hills"]],
  CIVILIZATION_LANUN: [["shipwreck", 0.2, PAINTED, "coast"], ["anchor", 0.2, PAINTED, "coast"]],
};

/** Props that carry a light of their own (hut windows, runes, lamps, embers): their bright,
 *  saturated pixels feed the bloom. */
const GLOWS = {
  crystal: 1.3, lava_rock: 1.3, obelisk: 1.4, altar: 1.2, vent: 1.4, lantern: 1.4, hut: 0.6,
  rune_stone: 1.3, glyph_ring: 1.3, arcane_obelisk: 1.4, float_crystal: 1.3, crystal_pillar: 1.3,
  brazier: 1.4, holy_shrine: 1.0, ash_vent: 1.4, ash_mound: 1.1, red_lantern: 1.4, mine_entrance: 0.8,
};

/** The town kit's architecture families (tools/hd2d/towns.json, docs/HD2D.md §5c), by the short
 *  name of a civilization (iso.js civShortName): the 3D renderer's families (architecture.js
 *  CIV_FAMILY), its sub-styles families of their own. */
const TOWN_FAMILY = {
  bannor: "human", elohim: "human", mercurians: "human", kuriotates: "human", grigori: "human", hippus: "human", balseraphs: "human",
  amurites: "arcane", sidar: "arcane", illians: "north", doviello: "north", ljosalfar: "elven", svartalfar: "darkelf",
  khazad: "dwarven", luchuirp: "dwarven", calabim: "gothic", infernal: "infernal", sheaim: "infernal",
  clan_of_embers: "tribal", barbarian: "tribal", malakim: "desert", lanun: "coastal",
};

/** A city's town kit: its family and the card kinds it is built from (a family's
 *  town_<family>_house_1.., tower, wall, gate, corner; the civilization's landmark town_lm_<civ>),
 *  or null while the family has no cards (the town then stands as the isometric sprite). */
const kits = new Map();
function townKit(city) {
  const civ = civShortName(city.owner);
  if (kits.has(civ)) return kits.get(civ);
  const fam = TOWN_FAMILY[civ] ?? "human";
  const k = (piece) => variants(`town_${fam}_${piece}`);
  const lm = variants(`town_lm_${civ}`);
  const kit = k("house").length ? { fam, civ, house: k("house"), tower: k("tower"), wall: k("wall"), gate: k("gate"), corner: k("corner"), landmark: lm.length ? lm[0] : null } : null;
  kits.set(civ, kit);
  return kit;
}

/** How far a town reaches round its plot centre (plot-local, z toward the camera): an ellipse
 *  about (0, cz), rx across and rz deep, from the 3D renderer's reach (CATALOG.cityReach: a village
 *  keeps to its plot, a great walled capital spills to about 0.9), so the HUD's plate, which stands
 *  just in front of that reach, stays in front of the town. */
function townShape(city, scale = 1) {
  const reach = CATALOG.cityReach?.({ size: city.size ?? 1, walls: !!city.walls, capital: !!city.capital }) ?? 0.5;
  // A larger town (config.js townScale) spreads as far as its pieces grow.
  return { cz: -0.05 * scale, rx: (reach + 0.06) * scale, rz: reach * 0.78 * scale };
}

/** The pieces of a town as card records: the civilization's landmark at the back centre rising
 *  over the roofs, the family's towers at its back corners (a town has one, a city two), houses
 *  packed irregularly round them -- more and further out as it grows, the taller ones behind -- and
 *  for a walled city a ring of wall segments with corner towers and a gate at its front. Each piece
 *  is its own card at the prop cards' density (PX.prop): lit, shaded and depth-sorted as props are.
 *  Deterministic from the city's id; pieces keep off water, rivers, roads, the garrison's and the
 *  banner's spots, and neighbouring plots that are unknown or hold something. */
function dressTown(world, map, atlas, env, city, kit, list) {
  const x = city.x, y = city.y;
  const r = rng(hash32(city.id * 7919 + 4099));
  const size = city.size ?? 1;
  // Larger towns (config.js townScale): every piece and the spread between them.
  const S = env?.town?.scale ?? 1;
  const { cz, rx, rz } = townShape(city, S);
  const css = teamCss(city.owner);
  // Only a piece with the owner-colour band (art.py index) needs its owner's variant.
  const own = (key) => (atlas.owned && CARDS[key]?.band ? atlas.owned(key, css) : key);
  const id = idOf(`c${city.id}`);
  const vis = [x + 0.5, y + 0.5];
  const placed = [];
  const garrison = garrisonSpot(world, x, y);
  // Where a piece may stand (plot-local lx east, lz toward the camera).
  const groundOk = (lx, lz) => {
    const mx = x + 0.5 + lx, my = y + 0.5 - lz;
    const j = world.index(Math.floor(mx), Math.floor(my));
    if (j < 0) return false;
    const kind = world.kind[j];
    if (kind === KIND.VOID || kind === KIND.OCEAN || kind === KIND.COAST || kind === KIND.ICE || world.isWater(j)) return false;
    if (world.heightAt(mx, my) < SEA + 0.01) return false;
    if (world.riverCells.size && world.riverDist(mx, my) < RIVER_HALF + 0.05) return false;
    if (j !== y * world.w + x) {
      // A neighbour's plot: open land with nothing of its own on it.
      if (kind === KIND.PEAK || map.city[j] >= 0 || map.improvement[j] >= 0) return false;
      if (env?.unitAt?.has(j)) {
        const ux = Math.floor(mx) + 0.5 + SPOT[0], uy = Math.floor(my) + 0.5 - SPOT[1];
        if (Math.hypot(mx - ux, my - uy) < 0.3) return false;
      }
    }
    // Roads run from the plot centre to its neighbours': keep a lane along each.
    if (map.route[y * world.w + x] >= 0) {
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const n = world.index(x + dx, y + dy);
          if ((!dx && !dy) || n < 0 || map.route[n] < 0) continue;
          const ex = dx, ez = -dy, t = clamp((lx * ex + lz * ez) / (ex * ex + ez * ez), 0, 1);
          if (Math.hypot(lx - ex * t, lz - ez * t) < 0.07) return false;
        }
      }
    }
    // The garrison's and the banner's spots stay clear.
    if (Math.hypot(lx - garrison[0], lz - garrison[1]) < 0.13) return false;
    return Math.hypot(lx - BANNER_SPOT[0], lz - BANNER_SPOT[1]) >= 0.09;
  };
  // Two upright cards at about the same depth must not overlap (they would cut into each other);
  // one well in front of another may hide its lower part (roofs behind roofs, a dense town), not
  // all of it.
  const clear = (lx, lz, w) => placed.every((p) => {
    const dx = Math.abs(lx - p.lx), dz = Math.abs(lz - p.lz), half = (w + p.w) / 2;
    return dz < 0.06 * S ? dx > half * 0.9 : dz < 0.15 * S ? dx > half * 0.35 : true;
  });
  const put = (key, lx, lz, o = {}) => {
    const art = atlas.info(key);
    const s = (o.scale ?? 1) * S;
    const w = art.w * PX.prop * s;
    if (!o.force && (!clear(lx, lz, w) || !groundOk(lx, lz))) return false;
    const mx = x + 0.5 + lx, my = y + 0.5 - lz;
    placed.push({ lx, lz, w });
    list.push(card(own(key), art, PX.prop * s, mx, world.heightAt(mx, my) - 0.006, -my, { id, bias: BIAS.building, vis, town: true, shadow: true, glow: 1.1, flip: !!o.flip }));
    return true;
  };
  const inEllipse = (a, b) => [a * rx, cz + b * rz];
  // The landmark, back centre, growing with the town (within the props' own spread of scales): a
  // village's hall, a town's keep, a great city's palace over everything.
  const lmScale = Math.min(1.18, (size <= 3 ? 0.78 : size <= 8 ? 1 : 1.12) + (city.capital && size > 3 ? 0.06 : 0));
  let lm = null;
  if (kit.landmark) {
    // At the back, or as near it as there is dry ground (a river along the plot's back edge): stepped
    // toward the front, at the plot's centre if nowhere else.
    const lx = r.range(-0.05, 0.05);
    let lz = 0;
    for (let k = 0; k <= 8; k++) {
      const z = cz - rz * 0.42 + k * 0.06 * S;
      if (groundOk(lx, z)) {
        lz = z;
        break;
      }
    }
    lm = { lx, lz, w: atlas.info(kit.landmark).w * PX.prop * lmScale * S };
    put(kit.landmark, lx, lz, { scale: lmScale, force: true });
  }
  // The towers at the back corners.
  const towers = size >= 9 ? 2 : size >= 4 ? 1 : 0;
  const side = r.sign();
  const towerAt = [];
  for (let k = 0; k < towers && kit.tower.length; k++) {
    const s = k ? -side : side;
    for (let tries = 0; tries < 8; tries++) {
      const at = inEllipse(s * r.range(0.5, 0.72), r.range(-0.55, -0.15));
      if (put(r.pick(kit.tower), ...at, { flip: r() < 0.5 })) {
        towerAt.push(at);
        break;
      }
    }
  }
  // Banners over the roofs (config.js townBanners): the owner's colour flying from behind the
  // landmark, higher than anything in the town, and from behind each tower -- as the look target's
  // towns fly theirs, readable from afar. (Drawn by code, so the colour is the owner's own; they
  // ripple, take the figures' rim and keep their colours under the moods.)
  const flags = env?.town?.banners ?? 0;
  if (flags > 0) {
    const bk = bannerKey(atlas, css);
    const team = hexRgb(cssHex(css));
    const flag = (lx, lz, k, n) => {
      const mx = x + 0.5 + lx, my = y + 0.5 - lz;
      list.push(card(bk, atlas.info(bk), PX.banner * k, mx, world.heightAt(mx, my), -my, { anchor: 5.5 / 40, banner: true, team, bias: 0.06, id: idOf(`ct${city.id}:${n}`), phase: hashf(city.id, 13 + n) }));
    };
    // One over the landmark; a great city's towers fly pennants too.
    if (lm) flag(lm.lx + lm.w * 0.18, lm.lz - 0.03 * S, 0.7 + 0.4 * flags, 0);
    if (size >= 9) towerAt.forEach(([lx, lz], n) => flag(lx + 0.03 * S, lz - 0.03 * S, 0.55 + 0.2 * flags, n + 1));
  }
  // The walls: segments round an ellipse just outside the houses', corner towers at its sides and
  // corners, the gate at the front middle.
  if (city.walls && kit.wall.length) {
    const wx = rx + 0.02, wz = rz + 0.03;
    const at = (deg) => [wx * Math.cos((deg * Math.PI) / 180), cz + wz * Math.sin((deg * Math.PI) / 180)];
    const post = (keys, deg) => {
      const [lx, lz] = at(deg);
      if (keys.length && groundOk(lx, lz)) put(keys[0], lx, lz, { force: true });
    };
    post(kit.gate, 90);
    // (Not at the back corners, where the family's towers stand: the landmark would be crowded.)
    const corners = [0, 35, 145, 180];
    for (const deg of corners) post(kit.corner, deg);
    // Between the posts, a segment every 0.24 or so of the ring (on its sides, where the ring runs
    // into the view, they stand one behind another).
    const ring = [0, 35, 90, 145, 180, 360];
    for (let k = 0; k + 1 < ring.length; k++) {
      const a = ring[k], b = ring[k + 1];
      let len = 0;
      for (let d = a; d < b; d += 5) {
        const [x0, z0] = at(d), [x1, z1] = at(d + 5);
        len += Math.hypot(x1 - x0, z1 - z0);
      }
      const n = Math.max(1, Math.round(len / (0.24 * S)));
      for (let m = 1; m <= n; m++) {
        const [lx, lz] = at(a + ((b - a) * m) / (n + 1));
        if (groundOk(lx, lz)) put(r.pick(kit.wall), lx, lz, { force: true });
      }
    }
  }
  // The houses: rejection sampling in the ellipse, the tall ones toward the back.
  const want = Math.min(26, Math.round(4 + size * 1.6 + (city.capital ? 2 : 0)));
  const byHeight = [...kit.house].sort((a, b) => atlas.info(a).h - atlas.info(b).h);
  let n = 0;
  for (let tries = 0; tries < want * 30 && n < want; tries++) {
    const a = r.range(-1, 1), b = r.range(-1, 1);
    if (a * a + b * b > 1) continue;
    // Behind (b < 0) more often the taller houses, in front the lower ones (well mixed: the two
    // that carry the owner's colour are not all at the back); right before the landmark only the
    // lowest, so its lower storeys show.
    const [lx, lz] = inEllipse(a * (city.walls ? 0.9 : 1), b * (city.walls ? 0.86 : 1));
    const t = clamp(0.5 - b * 0.3 + r.range(-0.45, 0.45), 0, 0.999);
    const before = lm && Math.abs(lx - lm.lx) < lm.w * 0.3 && lz > lm.lz && lz < lm.lz + 0.35 * S;
    const key = byHeight[before ? 0 : Math.floor(t * byHeight.length)];
    if (put(key, lx, lz, { flip: r() < 0.5, scale: r.range(0.9, 1.06) })) n++;
  }
}

/** What may stand on a town's clearing (config.js townClearing): low things and the fields. */
const LOW = new Set(["flowers", "grass", "grass_3", "pebbles", "garden", "fence", "hay", "well", "cart"]);

/** Props too low to cast a shadow worth the shadow map's while. */
const FLAT = new Set(["flowers", "grass", "grass_3", "pebbles", "garden", "bones"]);

/** Agricultural improvements carry fences and haystacks. */
const FARMED = /FARM|PASTURE|COTTAGE|HAMLET|VILLAGE|PLANTATION|WINERY/;

/** The prop cards of the land: stands of single trees on wooded plots (a jittered three by three,
 *  thinned where something stands on the plot), bushes, flowers, rocks, crystals and lava rocks
 *  scattered on open ground -- off rivers, roads, towns and the spot where a stack stands. A pure
 *  function of the state and the plot index. */
function dressProps(world, map, atlas, env, moods, civs, list) {
  const w = world.w;
  const H = (mx, my) => world.heightAt(mx, my);
  /** A town of the kit's shape (townShape), by its plot: worked out once a dressing. */
  const shapeAt = new Map();
  const townScale = env.town?.scale ?? 1;
  for (const c of S.cities.values()) {
    if (townKit(c)) shapeAt.set(c.y * w + c.x, townShape(c, townScale));
  }
  // A town on a clearing keeps tall vegetation farther from its buildings.
  const clearing = env.town?.clearing ?? 0;
  const margin = (kindKey) => (clearing > 0 && !LOW.has(kindKey) ? 0.6 * clearing : 0);
  const cityNear = (x, y, lx, lz, m = 0) => {
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const j = world.index(x + dx, y + dy);
        if (j < 0 || map.city[j] < 0) continue;
        const ex = lx - dx, ez = lz + dy;
        const shape = shapeAt.get(j);
        if (shape) {
          // A town of the kit: its reach (townShape) and a little round it.
          const { cz, rx, rz } = shape;
          const ax = rx + 0.06 + m, az = rz + 0.1 + m;
          if ((ex * ex) / (ax * ax) + ((ez - cz) * (ez - cz)) / (az * az) < 1) return true;
          // And a lane before its middle: the way to its gate, where the HUD's plate stands.
          if (Math.abs(ex) < 0.24 && ez > cz && ez < cz + rz + 0.45) return true;
          // A settlement's card spans about 1.7 plots; keep its footprint (round its plot) clear.
        } else if ((ex * ex) / (0.95 * 0.95) + ((ez - 0.15) * (ez - 0.15)) / (0.75 * 0.75) < 1) return true;
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
    const tribal = tname("ImprovementType", map.improvement[i]) === "IMPROVEMENT_GOODY_HUT";
    const villageFoot = tribal && kind === KIND.HILLS ? hilltopFoot(world, x, y, 0.18) : [0, 0];
    const road = map.route[i] >= 0;
    // Something stands on the plot behind (north): this plot's trees stand between it and the
    // camera, so its back row goes and its middle row keeps low.
    const n = world.index(x, y + 1);
    const northHeld = n >= 0 && (env.unitAt.has(n) || map.city[n] >= 0);
    const ok = (lx, lz, m = 0) => {
      const mx = x + 0.5 + lx, my = y + 0.5 - lz;
      if (world.riverCells.size && world.riverDist(mx, my) < RIVER_HALF + 0.1) return false;
      if (road && (Math.abs(lx) < 0.12 || Math.abs(lz) < 0.12)) return false;
      if (held && Math.hypot(lx - (tribal ? villageFoot[0] : 0), lz - (tribal ? villageFoot[1] : 0.08)) < (tribal ? 0.46 : 0.3)) return false;
      return !cityNear(x, y, lx, lz, m);
    };
    const put = (kindKey, lx, lz, scale) => {
      if (map.improvement[i] < 0 && ["rock", "pebbles", "lava_rock"].includes(kindKey)) return;
      const vs = variants(kindKey);
      if (!vs.length || !ok(lx, lz, margin(kindKey))) return;
      const key = vs[r.int(0, vs.length - 1)];
      const art = atlas.info(key);
      const mx = x + 0.5 + lx, my = y + 0.5 - lz;
      const glow = GLOWS[kindKey] ?? (kindKey.startsWith("hut") ? GLOWS.hut : 0);
      const flip = r() < 0.5;
      const base = H(mx, my);
      // (Where the land is a slab nothing of a land plot stands past its rim, in the water.)
      if (world.cliff > 0 && world.shore <= 0.004) return;
      list.push(card(key, art, PX.prop * scale, mx, base - 0.01, -my, { shadow: !FLAT.has(kindKey), flip, glow, id: 0.0002, vis: [x + 0.5, y + 0.5] }));
    };
    if (WOODED.has(feat)) {
      const wood = woodOf(feat, world.ground[i], kind === KIND.HILLS);
      const total = wood.reduce((s, [, wt]) => s + wt, 0);
      // The giants of an ancient wood stand one or two to a plot, in its middle.
      if (feat === FEAT.ANCIENT && variants("tree_ancient").length) {
        put("tree_ancient", r.range(-0.12, 0.12), r.range(-0.18, 0.0), r.range(0.95, 1.15));
      }
      // A crowd of trees: four rows of four, every other row shifted half a step (no columns of
      // trees show), jittered, reaching a little over the plot's edge. A field of the map (not of
      // the plot) thins them into clumps and glades that run across plots, so a wood is not a
      // filled square.
      for (let gy = 0; gy < 4; gy++) {
        for (let gx = 0; gx < 4; gx++) {
          let pick = r() * total, k = wood[0];
          for (const it of wood) if ((pick -= it[1]) <= 0) { k = it; break; }
          if (k[0] === "tree_ancient") k = ["tree_oak", 1, 1];
          const s = TREE * (DRAWN[k[0]] ?? 1) * k[2] * r.range(0.78, 1.2);
          const lx = (gx - 1.5 + (gy % 2 ? 0.25 : -0.25)) * 0.27 + r.range(-0.07, 0.07), lz = (gy - 1.5) * 0.26 + r.range(-0.07, 0.07);
          if (northHeld && gy === 0) continue;
          const grove = noise2((x + 0.5 + lx) * 2.3 + 17.0, (y + 0.5 - lz) * 2.3 + 5.0);
          if (grove < -0.18) continue;
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
      for (const [k, n, s, where] of CIV_PROPS[civs?.[i]] ?? []) {
        if (where === "hills" && kind !== KIND.HILLS) continue;
        if (where === "coast" && !nearSea(world, x, y)) continue;
        if (HELLISH.has(world.ground[i])) continue;
        if (r() < n) put(k, ...at(), s * r.range(0.85, 1.1));
      }
      // Only worked land carries agricultural props; nearby bare terrain stays open.
      const imp = map.improvement[i] >= 0 ? tname("ImprovementType", map.improvement[i]) : "";
      if (FARMED.test(imp)) {
        if (r() < 0.75) put("fence", r.range(-0.2, 0.2), r() < 0.5 ? 0.42 : -0.4, FINE);
        if (r() < 0.55) put("hay", r.range(-0.35, 0.35), r.range(-0.1, 0.38), FINE * r.range(0.85, 1.05));
      }
    }
  }
}

/** Whether a land plot touches the sea (or a lake). */
function nearSea(world, x, y) {
  for (let dy = -1; dy <= 1; dy++) {
    for (let dx = -1; dx <= 1; dx++) {
      const j = world.index(x + dx, y + dy);
      if ((dx || dy) && j >= 0 && world.isWater(j)) return true;
    }
  }
  return false;
}

/** The look target's land ends at its water in dark, wet stones: a loose row of them in the
 *  shallows at the foot of the land's face, along the sea's and the lakes' shores, and rows along
 *  the river banks, with reeds here and there. One stone a cell of about a fifth of a plot at most (a global
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
          // The land ends in a face (world.js `cliff`): its fallen stones stand in the shallows at
          // its foot, a little way out (the shore field just under the sea's level) -- a loose
          // row, a boulder now and then, not a rim.
          const s = world.shoreAt(mx, my);
          if (s < 0 && s > -0.12) {
            // (How far out: the field's value over its slope.)
            const e = 0.03;
            const off = -s / (Math.hypot(world.shoreAt(mx + e, my) - s, world.shoreAt(mx, my + e) - s) / e || 1);
            if (off > 0.02 && off < 0.075 && r() < 0.17) {
              // (Their feet just over the water's level: a card cut by the water's plane would
              // wear its foam in a straight bar.)
              if (r() < 0.2) place("rock", mx, my, SEA + 0.003, FINE * r.range(0.6, 0.85), r);
              else place("shore", mx, my, SEA + 0.003, FINE * r.range(0.8, 1.05), r);
              continue;
            }
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

/** Keep a little sky/ground around a resource's silhouette. A tree's crown can hide a resource
 *  on the next plot, so test the cards' opaque bounds across plots, at the gameplay camera's
 *  full 30–42 degree range. Removing props after dressing keeps every other prop's RNG unchanged.
 *  Buckets make this local work even on a fully revealed Huge map. */
function resourceClearings(world, atlas, props, resources) {
  if (!resources.size) return props;
  const bounds = (it) => {
    const art = atlas.info(it.key);
    const [x0, y0, x1, y1] = art.box ?? [0, 0, art.w, art.h];
    const left = (x0 / art.w - it.anchor) * it.w;
    const right = (x1 / art.w - it.anchor) * it.w;
    return { left: it.flip ? -right : left, right: it.flip ? -left : right,
      bottom: (1 - y1 / art.h) * it.h, top: (1 - y0 / art.h) * it.h };
  };
  const nearby = new Map();
  const targets = new Map([...resources].map(([i, it]) => [i, { ...it, ...bounds(it) }]));
  return props.filter((it) => {
    const x = Math.floor(it.x), y = Math.floor(-it.z), index = world.index(x, y);
    const cell = index < 0 ? `${x},${y}` : index;
    let around = nearby.get(cell);
    if (!around) {
      around = [];
      for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) {
        const target = targets.get(world.index(x + dx, y + dy));
        if (target) around.push(target);
      }
      nearby.set(cell, around);
    }
    if (!around.length) return true;
    const b = bounds(it), pad = 0.08;
    return !around.some((r) => {
      let dx = it.x - r.x;
      if (world.wrapX) dx -= Math.round(dx / world.w) * world.w;
      if (dx + b.right < r.left - pad || dx + b.left > r.right + pad) return false;
      let dy = r.z - it.z;
      if (world.wrapY) dy -= Math.round(dy / world.h) * world.h;
      const dh = it.y - r.y;
      // Conservative bounds over the camera's pitch, including intermediate angles. The cards
      // face the camera; only their bases move in screen height as the pitch changes.
      const lo = Math.min(dy * 0.5, dy * 0.7071) + Math.min(dh * 0.7071, dh * 0.8661);
      const hi = Math.max(dy * 0.5, dy * 0.7071) + Math.max(dh * 0.7071, dh * 0.8661);
      return hi + b.top > r.bottom - pad && lo + b.bottom < r.top + pad;
    });
  });
}

/** The static cards (cities, standing improvements, resources and their banners, the land's prop
 *  cards) and the light beams over mana nodes and towers; `claims` names what the 3D dressing no
 *  longer models (with prop cards: its trees, plants and rocks). */
export function dressSpriteStatic(world, map, atlas, env = null, moods = null, civs = null) {
  const list = [];
  const beams = [];
  const claims = { city: new Set(), improvement: new Set(), bonus: new Set() };
  if (env && Object.keys(CARDS).length) {
    dressProps(world, map, atlas, env, moods, civs, list);
    dressShores(world, map, atlas, list);
    claims.trees = claims.flora = claims.rocks = true;
  }
  const propCount = list.length;
  const resources = new Map();
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
      const kit = townKit(city);
      const key = kit ? null : first(atlas, ISO_TABLES.citySprites(city.size ?? 1, civShortName(city.owner)));
      if (kit) dressTown(world, map, atlas, env, city, kit, list);
      else if (key) {
        // No kit for its family: the isometric settlement, its foot on the plot's front edge (its
        // painted ground then lies over the plot).
        const art = atlas.info(key);
        const [mx, my] = at(0, 0.5);
        // A town: graded lighter than the land (it keeps its colours), its windows and fires lit.
        list.push(card(key, art, PX.city, mx, H(x + 0.5, y + 0.5), -my, { id: idOf(`c${city.id}`), bias: BIAS.building, vis: [x + 0.5, y + 0.5], town: true, glow: 1.1 }));
      }
      if (kit || key) {
        const css = teamCss(city.owner);
        const bk = bannerKey(atlas, css);
        const [bx, by] = at(...onLand(world, x, y, ...BANNER_SPOT));
        list.push(card(bk, atlas.info(bk), PX.banner, bx, H(bx, by), -by, { anchor: 5.5 / 40, banner: true, team: hexRgb(cssHex(css)), bias: BIAS.unit, id: idOf(`cb${city.id}`), phase: hashf(city.id, 5) }));
        claims.city.add(i);
      }
    }
    const imp = map.improvement[i] >= 0 ? tname("ImprovementType", map.improvement[i]) : null;
    // The plot-local z of a standing improvement card's foot, when one stands here.
    let footZ = null, footX = 0;
    if (imp && !city) {
      const key = first(atlas, improvementSpriteKeys(imp).filter((k) => k.startsWith("bld_") || k.startsWith("feat_")));
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        const mine = imp === "IMPROVEMENT_MINE" && world.kind[i] === KIND.HILLS;
        const px = PX.building * (art.pixelScale ?? 1) * (mine ? 0.82 : 1);
        const tribal = imp === "IMPROVEMENT_GOODY_HUT";
        const hill = world.kind[i] === KIND.HILLS;
        const [lx, lz, base] = water ? [0, 0.5, SEA - DRAFT]
          : hill ? (tribal ? hilltopFoot(world, x, y, 0.18) : hillWorkFoot(world, x, y, art.w * px * 0.5))
          : tribal ? [0, 0, H(x + 0.5, y + 0.5)] : builtFoot(world, x, y);
        footZ = lz;
        footX = lx;
        const [mx, my] = at(lx, lz);
        const mana = manaColour(imp);
        // Coarser authored pixels keep a small burial mound's doorway legible at
        // play zoom, while its world footprint stays the same as the full card.
        const huts = tribal ? TRIBAL_HUTS : [[0, 0, 1]];
        for (const [ox, oz, scale] of huts) {
          const hx = mx + ox, hy = my - oz;
          const c = card(key, art, px * scale, hx, tribal ? H(hx, hy) : base, -hy, { id: idOf(`i${i}`), glow: mana ? 0.4 : 0, fade: !water, vis: [x + 0.5, y + 0.5] });
          // A hill work drawn from an isometric sprite lays its painted ground on the hill (not the
          // HD-2D sites' own art, which has none).
          if (hill && !tribal && !key.startsWith("bld_site_") && art.box) c.drape = drapeGrid(world, c, art);
          list.push(c);
        }
        claims.improvement.add(i);
        const tower = /TOWER/.test(imp) && !/WATCH|WARNING/.test(imp);
        if (mana || tower) {
          const col = hexRgb(mana ?? "#ffe6a8");
          const [cx, cy] = at(0, 0.02);
          beams.push({ key: null, x: cx, y: H(cx, cy), z: -cy, w: mana ? 0.22 : 0.4, h: mana ? 0.6 : 1.5, anchor: 0.5, team: col, glow: mana ? 0.12 : 0.3, id: 0, phase: hashf(i, 9) });
        }
      }
    }
    const bonus = map.bonus[i] >= 0 ? tname("BonusType", map.bonus[i]) : null;
    // A connected shrine or mana-producing landmark represents the deposit itself.
    const connectedMana = improvementContainsMana(imp, bonus);
    if (connectedMana) claims.bonus.add(i);
    if (bonus && !city && !connectedMana) {
      const key = first(atlas, [RESOURCES[bonus], ISO_TABLES.resourceSprite(bonus)]);
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        // Before what stands on the plot (the mine's card would hide its ore), else at the front-left.
        const built = footZ !== null;
        const [mx, my] = at(...(water ? (built ? [footX + BUILT_BONUS_SPOT[0], footZ - BUILT_BONUS_SPOT[1]] : BONUS_SPOT) : onLand(world, x, y, ...(built ? [footX + BUILT_BONUS_SPOT[0], footZ - BUILT_BONUS_SPOT[1]] : BONUS_SPOT))));
        const base = water ? SEA : H(mx, my);
        const glow = /MANA/.test(bonus) ? 0.3 : /GEMS|JEWELS|MITHRIL/.test(bonus) ? 1.2 : 0;
        const resource = card(key, art, PX.resource, mx, base, -my, { id: idOf(`b${i}`), glow, bias: built ? BIAS.builtResource : BIAS.resource, vis: [x + 0.5, y + 0.5] });
        list.push(resource);
        resources.set(i, resource);
        claims.bonus.add(i);
        if (/MANA/.test(bonus)) beams.push({ key: null, x: mx, y: base, z: -my, w: 0.16, h: 0.3, anchor: 0.5, team: hexRgb(manaColour(bonus) ?? "#b48aff"), glow: 0.08, id: 0, phase: hashf(i, 11) });
      }
    }
  }
  const props = resourceClearings(world, atlas, list.slice(0, propCount), resources);
  return { list: props.concat(list.slice(propCount)), beams, claims };
}

/** The unit stacks in sight as groups of figures under their banners, and the seas' buoys `buoys`
 *  (on the Sea layer: dressSeaBuoys; null without). Returns `{list, stacks}` in the shape of
 *  ../dress.js dressUnits (stacks: for the HUD's badges and the team rings). */
export function dressSpriteUnits(world, map, atlas, buoys = null) {
  const list = [];
  const stacks = [];
  const w = world.w;
  const mine = S.view?.me?.player;
  const sel = S.sel?.units ?? [];
  const rating = (u) => S.data?.tables?.UnitType?.[u.type]?.rating ?? 0;
  const sctx = {
    me: mine,
    selected: sel,
    unitInfo: (t) => {
      const s = rating({ type: t });
      return { hero: false, combat: s > 0 ? "COMBAT" : "NONE", rating: s };
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
    if (improvementSpriteKeys(imp).some((k) => (k.startsWith("bld_") || k.startsWith("feat_")) && atlas.has(k))) standing.add(i);
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
    const dom = info ? tname("DomainType", info.domain) : null;
    let spot = onCity ? garrisonSpot(world, u0.x, u0.y) : world.feat[i] === FEAT.VOLCANO ? VOLCANO_SPOT : standing.has(i) ? BUILT_SPOT : SPOT;
    // (A group on a land plot stands on its land, not past the rim of the land's face.)
    if (!world.isWater(i) && world.kind[i] !== KIND.ICE && dom !== "DOMAIN_SEA" && dom !== "DOMAIN_AIR") spot = onLand(world, u0.x, u0.y, spot[0], spot[1], 0.13);
    const n = onCity ? 1 : groupSize(info, art);
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
  if (buoys) list.push(...dressSeaBuoys(buoys, atlas));
  return { list, stacks };
}
