// The battle stage (the formation-battle prototype's battlefield, phase 6 revision 4): the two
// lines on one isometric strip of M7 tiles at 1x, six ranks deep (the attacker's rear, middle and
// front, then the defender's front, middle and rear) by the lanes, ringed by dimmed ground with
// the ground's scenery (trees round a forest or jungle, the city behind its defenders, hills on a
// hill). The same stage serves the plan, the battle and the report. Drawing is a function of a
// scene description (figures, highlighted cells, effects), so the screens own their state. Canvas
// and DOM; the geometry is the battle overlay's 1280 x 800 frame (battle.js centres it).
//
// Art: the M7 sprites (`app/iso`, docs/ISO.md) and, for the units that have them, the anim-sprint
// sets (`app/anim/<unit>_<idle|strike|flinch|death>.png`: strips of 160 x 128 frames, the feet
// at (80, 126); anim-sprint/ANIMATING_UNITS.md). Pixel art is drawn at 1x, but for the watch
// screen's breathing (a squash of 2% at most) and its snap (battle_snap.js: a lean, up to 8% larger).
// Key poses (`app/pose/<unit>_<attack|hit>.png`: one 160 x 128 frame each, the feet at (80, 126)) give
// a unit without the strips the one frame its snap holds; impact cards (`app/fx/<card>_<back|front>.png`)
// are what a ranged blow does where it lands (anim-sprint/ANIMATING_UNITS.md, "Impact cards and key
// poses"; made by anim-sprint/lib/impact_cards.py).

import { SPRITES } from "./iso/sprites.js";
import { unitSpriteKeys, ISO_TABLES } from "./iso.js";
import { tyName, tyIdx, ROWS, groundTags } from "./battle_log.js";
import { KEY_POSES } from "./key_poses.js";

export const FRAME_W = 1280;
export const FRAME_H = 800;

const spriteUrl = (key) => new URL(`./iso/${key}.png`, import.meta.url).href;
const animUrl = (unit, anim) => new URL(`./anim/${unit}_${anim}.png`, import.meta.url).href;
const poseUrl = (unit, pose) => new URL(`./pose/${unit}_${pose}.png`, import.meta.url).href;
const fxUrl = (card, layer) => new URL(`./fx/${card}_${layer}.png`, import.meta.url).href;

/** The anim-sprint sets the client ships (their timings are the sets' manifests'), by the short
 *  name of the M7 sprite they animate; `left`: the set faces lower-left. The strips the screens
 *  draw (the sets' walk strips are not: nobody walks on the battlefield). */
const ANIM_SETS = { axeman: {}, champion: { left: true }, phalanx: {}, swordsman: {}, warrior: {} };
export const ANIMS = {
  idle: [250, 250, 250, 250],
  strike: [100, 120, 160, 50, 120, 120, 100, 100],
  flinch: [80, 120, 120, 120],
  death: [80, 120, 140, 200, 80, 80, 80, 400],
};
// The key poses the client ships (key_poses.js, written with them): by the short name of the M7
// sprite, `attack` (held while it strikes, shoots, bombards or casts, when it has no strips) and
// `hit` (held while it is struck).
/** The impact cards the client ships (each a `back` layer, drawn behind the target, and a `front`). */
export const IMPACT_CARDS = ["rubble", "hawk"];
/** Units whose every blow lands as their own impact card, by unit type: the ranger's hawk strikes
 *  for him (he stays on his cell, as a shooter does). */
const UNIT_CARDS = { UNIT_RANGER: "hawk" };
/** ms from the start of the strike strip to its blow (the frame the hit lands on). */
export const STRIKE_HIT = 430;
const ANIM_FRAME = { w: 160, h: 128, ax: 80, ay: 126 };
/** M7 unit sprites that face lower-left (the rest face lower-right), from the anim-sprint roster's
 *  facings (anim-sprint/roster_classified.json on the anim-all branch). */
const FACES_LEFT = new Set(["auric", "auric_ascended", "azer", "balseraphs_adept", "barbarian_axeman", "barnaxus", "berserker", "bloodpet", "chalid", "champion", "chaos_marauder", "circle_of_urd", "clockwork_golem", "crusader", "demagog", "dwarven_soldier_runes", "flesh_golem", "harlequin", "high_priest_of_the_veil", "hill_giant", "hyborem", "illians_settler", "kraken", "lich", "lightbringer", "lucian", "mardero", "monk", "myconid", "ogre_embers", "priest_of_kilmorph", "priest_of_leaves", "priest_of_the_veil", "revelers", "sheaim_mage", "sphener", "valkyrie", "workboat"]);

export const COLORS = {
  you: "#8fb6ff",
  them: "#ff9a86",
  gold: "#f6c650",
  goldLt: "#ffe58c",
  cyan: "#4fe3ff",
  azure: ["#4a74d8", "#17368a"],
  gules: ["#d4433b", "#8a1c1c"],
};

// ---------------------------------------------------------------------------------------------
// Images

const IMG = new Map();
const PENDING = new Map();

function loadImage(key, url) {
  if (IMG.has(key)) return Promise.resolve(IMG.get(key));
  if (PENDING.has(key)) return PENDING.get(key);
  const p = new Promise((resolve) => {
    const im = new Image();
    im.onload = () => {
      IMG.set(key, im);
      resolve(im);
    };
    im.onerror = () => {
      console.warn("battle: missing art", url);
      IMG.set(key, null);
      resolve(null);
    };
    im.src = url;
  });
  PENDING.set(key, p);
  return p;
}

const sprite = (key) => (SPRITES[key] ? loadImage(key, spriteUrl(key)) : Promise.resolve(null));

/** The civilization's short name (`calabim`) of a side's `civ` (index or type name). */
function civShort(civ) {
  const n = tyName("CivilizationType", civ);
  return n ? n.replace(/^CIVILIZATION_/, "").toLowerCase() : null;
}

const FIGS = new WeakMap();

/** A fighter's art: its M7 sprite key, its anim-sprint set (or null) and its facing. */
export function figOf(f, sides) {
  let fig = FIGS.get(f);
  if (fig) return fig;
  // A city's Sentinel (docs/SIEGES.md): its wall section, the watchtower's art scaled to a figure.
  if (f.sentinel && SPRITES.bld_watchtower) {
    fig = { key: "bld_watchtower", anim: null, short: null, poses: new Set(), card: null, left: false, scale: 0.55 };
    FIGS.set(f, fig);
    return fig;
  }
  const civ = civShort(sides?.[f.side]?.shown_civ ?? sides?.[f.side]?.civ);
  const t = tyIdx("UnitType", f.unit_type);
  const keys = t >= 0 ? unitSpriteKeys(t, civ) : [];
  const own = tyName("UnitType", f.unit_type);
  if (own) keys.unshift(`unit_${own.replace(/^UNIT_/, "").toLowerCase()}`);
  const key = keys.find((k) => SPRITES[k]) ?? "unit_swordsman";
  const short = key.replace(/^unit_/, "");
  fig = { key, anim: ANIM_SETS[short] ? short : null, short, poses: new Set(KEY_POSES[short] ?? []), card: (own && UNIT_CARDS[own]) || null, left: FACES_LEFT.has(short) };
  FIGS.set(f, fig);
  return fig;
}

/** Whether fighter `f` has an animation set (it strikes, flinches and dies with its frames). */
export const animated = (sides) => (f) => !!figOf(f, sides).anim;

/** Fighter `f`'s key poses: a Set of "attack" and "hit" (empty: none). */
export const keyPoses = (sides) => (f) => figOf(f, sides).poses;

/** Fighter `f`'s own impact card (its blows land as it, from its cell), or null. */
export const unitCard = (sides) => (f) => figOf(f, sides).card;

const DRY = /PLAINS|DESERT|BURNING_SANDS|BROKEN_LANDS|FIELDS_OF_PERDITION|CRATER/;
const COLD = /TUNDRA|SNOW/;

function tileKey(terrain) {
  const k = ISO_TABLES.TERRAIN_TILE[tyName("TerrainType", terrain)] ?? "tile_grass";
  return SPRITES[k] ? k : "tile_grass";
}

function hillsKey(terrain) {
  const n = tyName("TerrainType", terrain) ?? "";
  const k = COLD.test(n) ? "ovl_hills_snow" : DRY.test(n) ? "ovl_hills_dry" : "ovl_hills";
  return SPRITES[k] ? k : "ovl_hills";
}

function featureKey(feature) {
  const n = tyName("FeatureType", feature);
  return (n && ISO_TABLES.FEATURE_SPRITES[n]?.find((k) => SPRITES[k])) || null;
}

function cityKey(plot, sides) {
  const c = plot.city;
  const civ = civShort(sides?.find((s) => s.player === c.owner)?.civ ?? sides?.[1]?.civ);
  return ISO_TABLES.citySprites(c.size ?? 4, civ).find((k) => SPRITES[k]) ?? "bld_city";
}

/** Loads the art a battle needs: the board's tiles, overlays and scenery, every fighter's sprite
 *  and the animation strips the screens draw. */
export function loadArt({ fighters, plot, from, sides }) {
  const jobs = [];
  for (const p of [plot, from]) {
    if (!p) continue;
    const t = tileKey(p.terrain);
    jobs.push(sprite(t), sprite(`${t}_b`), sprite(hillsKey(p.terrain)));
    const fk = featureKey(p.feature);
    if (fk) jobs.push(sprite(fk));
  }
  if (plot?.city) jobs.push(sprite(cityKey(plot, sides)));
  for (const f of fighters) {
    const fig = figOf(f, sides);
    jobs.push(sprite(fig.key));
    if (fig.anim) for (const a of Object.keys(ANIMS)) jobs.push(loadImage(`anim:${fig.anim}:${a}`, animUrl(fig.anim, a)));
    for (const k of fig.poses) jobs.push(loadImage(`pose:${fig.short}:${k}`, poseUrl(fig.short, k)));
  }
  for (const c of IMPACT_CARDS) for (const l of ["back", "front"]) jobs.push(loadImage(`fx:${c}:${l}`, fxUrl(c, l)));
  return Promise.all(jobs);
}

// ---------------------------------------------------------------------------------------------
// Geometry: depth d (0-2 the attacker's rear to front, 3-5 the defender's front to rear) and lane l
// of a cell; tiles are 192 x 128 with the top diamond in rows 0-95.

/** The board's geometry for `lanes` ([attacker's, defender's]): centred in the frame, below the
 *  top panels. */
export function geom(lanes) {
  const L = Math.max(lanes?.[0] ?? 2, lanes?.[1] ?? 2);
  return { L, lanes: [lanes?.[0] ?? 2, lanes?.[1] ?? 2], ox: 640 - ((5 - (L - 1)) / 2) * 96, oy: 136 + (4 - L) * 24 };
}

export const depthOf = (side, row) => (side === 0 ? 2 - row : 3 + row);
const tileTL = (G, d, l) => [G.ox + (d - l) * 96 - 96, G.oy + (d + l) * 48];

/** The feet of a unit standing in cell (`lane`, `row`) of side `side`. */
export function cellXY(G, side, lane, row) {
  const d = depthOf(side, row);
  return [G.ox + (d - lane) * 96, G.oy + (d + lane) * 48 + 50];
}

/** Where fighter `f` at `pos` ({lane, row}) stands: a large unit between its two cells. */
export function unitXY(G, f, pos) {
  const [x, y] = cellXY(G, f.side, pos.lane, pos.row);
  if (!f.large) return [x, y];
  const [x2, y2] = cellXY(G, f.side, pos.lane + 1, pos.row);
  return [(x + x2) / 2, (y + y2) / 2];
}

/** The cell under frame point (x, y): `{side, lane, row}`, or null. */
export function cellAt(G, x, y) {
  const uu = (x - G.ox) / 96;
  const vv = (y - G.oy - 48) / 48;
  const d = Math.round((uu + vv) / 2);
  const l = Math.round((vv - uu) / 2);
  if (d < 0 || d > 5 || l < 0) return null;
  const side = d <= 2 ? 0 : 1;
  const row = side === 0 ? 2 - d : d - 3;
  if (l >= G.lanes[side]) return null;
  return { side, lane: l, row };
}

/** The frame point of grid coordinates (d, l): a cell's centre at integers, its corners at halves. */
const gridPt = (G, d, l) => [G.ox + (d - l) * 96, G.oy + (d + l) * 48 + 48];

/** The board's light (buildBoard): `grade` how far the tiles' colour is pulled to grey (the units
 *  are the brightest, most saturated thing on the field, the ground supports them); `off` the
 *  darkening of the ground off the grid; `fall` the
 *  falloff into the dark and `pool` the warm light, as [r, alpha] stops along the light's ellipse
 *  (r 1: the grid's half extents); `tint`, `hair` (a dark line, a cream one beside it), `outline`
 *  the sides' blocks. */
const LIGHT = {
  grade: 0.32,
  off: 0.26,
  fall: [[0, 0], [1.04, 0], [1.2, 0.3], [1.42, 0.62], [1.7, 0.88]],
  pool: [[0, 0.14], [0.6, 0.09], [1.12, 0]],
  poolOp: "soft-light",
  tint: 0.05,
  hair: [0.32, 0.16],
  outline: 0.75,
};

/** Where the ranks' names stand: lanes past the near lane (1: the centre of the tile outside it). */
const LABEL_AT = 0.8;

/** The ellipse the board's light is centred on: the grid's centre, its half extents. */
function lightEllipse(G) {
  const [cx, cy] = gridPt(G, 2.5, (G.L - 1) / 2);
  return { cx, cy, rx: (6 + G.L) * 48, ry: (6 + G.L) * 24 };
}

/** How dark the ground is at frame point (x, y) (0..1): the falloff with the darkening off the grid. */
function darkAt(G, x, y) {
  const E = lightEllipse(G);
  const r = Math.hypot((x - E.cx) / E.rx, (y - E.cy) / E.ry);
  const S = LIGHT.fall;
  let a = S.at(-1)[1];
  for (let i = 1; i < S.length; i++)
    if (r <= S[i][0]) {
      const [r0, a0] = S[i - 1];
      const [r1, a1] = S[i];
      a = a0 + ((a1 - a0) * (r - r0)) / (r1 - r0);
      break;
    }
  return 1 - (1 - a) * (1 - LIGHT.off);
}

/** A deterministic hash of a tile (the gaps in a wood). */
function treeHash(d, l) {
  let h = Math.imul(d + 100, 0x27d4eb2d) ^ Math.imul(l + 100, 0x165667b1);
  h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
  return (h ^ (h >>> 13)) >>> 0;
}

const DARK = new Map();

/** `im` darkened by `a` (a dark fill over its pixels), cached by key and level. */
function darkImage(key, im, a) {
  if (a <= 0) return im;
  const id = `${key}@${a}`;
  let c = DARK.get(id);
  if (!c) {
    c = document.createElement("canvas");
    c.width = im.width;
    c.height = im.height;
    const g = c.getContext("2d");
    g.drawImage(im, 0, 0);
    g.globalCompositeOperation = "source-atop";
    g.fillStyle = `rgba(4, 6, 16, ${a})`;
    g.fillRect(0, 0, c.width, c.height);
    DARK.set(id, c);
  }
  return c;
}

function diamond(g, x, y) {
  g.beginPath();
  g.moveTo(x, y - 48);
  g.lineTo(x + 96, y);
  g.lineTo(x, y + 48);
  g.lineTo(x - 96, y);
  g.closePath();
}

// ---------------------------------------------------------------------------------------------
// The stage

export class Stage {
  constructor(canvas) {
    this.canvas = canvas;
    this.g = canvas.getContext("2d");
    /** The canvas's supersampling: its backing store is `n` pixels a frame px (battle.js `fit`). */
    this.n = 0;
    this.setScale(1);
    this.board = null;
    this.boardKey = "";
    this.model = null;
    this.G = geom([2, 2]);
    this.scenery = [];
    /** The viewer's side: its rings and cells are Azure, the other side's Gules. */
    this.me = 0;
  }

  /** Supersamples the canvas `n` times (1-3): every drawing stays in frame coordinates under
   *  `base`, the offscreen caches stay at 1x and are drawn through it. */
  setScale(n) {
    if (n === this.n) return;
    this.n = n;
    this.canvas.width = FRAME_W * n;
    this.canvas.height = FRAME_H * n;
    // resizing resets the context's state
    this.g.imageSmoothingEnabled = false;
  }

  /** Sets the base transform: frame coordinates, offset by (dx, dy) frame px. */
  base(dx = 0, dy = 0) {
    const n = this.n;
    this.g.setTransform(n, 0, 0, n, n * dx, n * dy);
  }

  /** The board of `model` ({plot, from, sides, lanes}); kept while the model's ground and lanes
   *  do not change. */
  setBoard(model) {
    this.model = model;
    this.G = geom(model.lanes);
    const key = JSON.stringify([model.plot, model.from, model.lanes, this.me]);
    if (key === this.boardKey && this.board) return;
    this.boardKey = key;
    this.board = this.buildBoard();
    this.scenery = this.buildScenery();
  }

  /** The ground, lit as a diorama: tiles over the whole frame (the attacker's plot's terrain on
   *  its half, the defender's on the other), the ground off the grid a little darker and falling
   *  off into the dark towards the frame's edges, a warm pool of light on the grid; each side's
   *  block of cells outlined in its colour, the contact line between the fronts in gold. */
  buildBoard() {
    const { plot, from } = this.model;
    const G = this.G;
    const c = document.createElement("canvas");
    c.width = FRAME_W;
    c.height = FRAME_H;
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = false;
    const tiles = [];
    for (let d = -14; d <= 20; d++)
      for (let l = -14; l <= 20; l++) {
        const [x, y] = tileTL(G, d, l);
        if (x + 192 > 0 && x < FRAME_W && y + 128 > 0 && y < FRAME_H) tiles.push([d, l]);
      }
    tiles.sort((a, b) => a[0] + a[1] - (b[0] + b[1]) || a[0] - b[0]);
    for (const [d, l] of tiles) {
      const [x, y] = tileTL(G, d, l);
      const p = d >= 3 ? plot : from;
      const base = tileKey(p?.terrain);
      const alt = ((d + 40) * 31 + (l + 40) * 17) % 3 === 0;
      const t = (alt && IMG.get(`${base}_b`)) || IMG.get(base);
      if (t) g.drawImage(t, x, y);
      if (p?.hills) {
        const hl = IMG.get(hillsKey(p.terrain));
        if (hl) g.drawImage(hl, x, y + (hl.height < 128 ? 128 - hl.height : 0));
      }
    }
    // the ground's colour, muted
    g.save();
    g.globalCompositeOperation = "saturation";
    g.globalAlpha = LIGHT.grade;
    g.fillStyle = "#808080";
    g.fillRect(0, 0, FRAME_W, FRAME_H);
    g.restore();
    const P = (d, l) => gridPt(G, d, l);
    const [n0, n1] = G.lanes;
    const gridPath = () => {
      g.beginPath();
      for (const [i, [d, l]] of [[-0.5, -0.5], [5.5, -0.5], [5.5, n1 - 0.5], [2.5, n1 - 0.5], [2.5, n0 - 0.5], [-0.5, n0 - 0.5]].entries()) g[i ? "lineTo" : "moveTo"](...P(d, l));
      g.closePath();
    };
    // (a) the ground off the grid, a little darker
    g.save();
    g.beginPath();
    g.rect(0, 0, FRAME_W, FRAME_H);
    gridPath();
    g.fillStyle = `rgba(6, 10, 28, ${LIGHT.off})`;
    g.fill("evenodd");
    g.restore();
    // (b) the falloff into the dark, and (c) the warm pool of light on the grid
    const E = lightEllipse(G);
    const ellipse = (stops, op) => {
      g.save();
      g.globalCompositeOperation = op;
      g.translate(E.cx, E.cy);
      g.scale(1, E.ry / E.rx);
      const R = E.rx * stops.at(-1)[0];
      const gr = g.createRadialGradient(0, 0, 0, 0, 0, R);
      for (const [r, col] of stops) gr.addColorStop(r / stops.at(-1)[0], col);
      g.fillStyle = gr;
      g.fillRect(-4000, -4000, 8000, 8000);
      g.restore();
    };
    ellipse(LIGHT.fall.map(([r, a]) => [r, `rgba(4, 6, 16, ${a})`]), "source-over");
    ellipse(LIGHT.pool.map(([r, a]) => [r, `rgba(255, 236, 196, ${a})`]), LIGHT.poolOp);
    // the sides' blocks: a faint tint, cream hairlines between the cells, the outline in the side's colour
    for (const side of [0, 1]) {
      const n = G.lanes[side];
      const [d0, d1] = side === 0 ? [-0.5, 2.5] : [2.5, 5.5];
      const far = side === 0 ? d0 : d1;
      const col = (side === this.me ? COLORS.azure : COLORS.gules)[0];
      g.save();
      g.beginPath();
      for (const [i, [d, l]] of [[d0, -0.5], [d1, -0.5], [d1, n - 0.5], [d0, n - 0.5]].entries()) g[i ? "lineTo" : "moveTo"](...P(d, l));
      g.closePath();
      g.globalAlpha = LIGHT.tint;
      g.fillStyle = col;
      g.fill();
      g.globalAlpha = 1;
      g.lineWidth = 1;
      for (const [dy, line] of [[0, `rgba(12, 20, 10, ${LIGHT.hair[0]})`], [1, `rgba(255, 245, 210, ${LIGHT.hair[1]})`]]) {
        g.strokeStyle = line;
        g.beginPath();
        for (let l = 0.5; l < n - 0.5; l++) g.moveTo(P(d0, l)[0], P(d0, l)[1] + dy), g.lineTo(P(d1, l)[0], P(d1, l)[1] + dy);
        for (let d = d0 + 1; d < d1; d++) g.moveTo(P(d, -0.5)[0], P(d, -0.5)[1] + dy), g.lineTo(P(d, n - 0.5)[0], P(d, n - 0.5)[1] + dy);
        g.stroke();
      }
      g.strokeStyle = col;
      g.globalAlpha = LIGHT.outline;
      g.lineWidth = 2;
      g.lineJoin = "round";
      g.beginPath();
      g.moveTo(...P(2.5, -0.5));
      g.lineTo(...P(far, -0.5));
      g.lineTo(...P(far, n - 0.5));
      g.lineTo(...P(2.5, n - 0.5));
      g.stroke();
      g.restore();
    }
    // the contact line between the two fronts
    g.save();
    g.lineWidth = 2;
    g.setLineDash([8, 6]);
    for (const [dy, col] of [[1, "rgba(10, 6, 2, 0.5)"], [0, "rgba(255, 212, 96, 0.95)"]]) {
      const [ax, ay] = P(2.5, -0.5);
      const [bx, by] = P(2.5, G.L - 0.5);
      g.beginPath();
      g.moveTo(ax, ay + dy);
      g.lineTo(bx, by + dy);
      g.strokeStyle = col;
      g.stroke();
    }
    g.restore();
    return c;
  }

  /** Scenery on the ring: trees round a plot with a forest or a jungle, the city behind the
   *  defenders. */
  buildScenery() {
    const { plot, from, sides } = this.model;
    const G = this.G;
    const out = [];
    const tree = (p) => featureKey(p?.feature);
    const td = tree(plot);
    const ta = tree(from);
    // the city behind the defenders' first lane: the far corner of the field, clear of the panels
    const cityL = 0;
    // the trees of each half's wood: the ring round the grid (as the prototype had it), and the
    // outer ground with gaps (a hash keeps about 70% of its tiles); never on the board's cells, nor
    // in the row behind the attacker's rear
    for (let d = -14; d <= 20; d++)
      for (let l = -14; l <= 20; l++) {
        const key = d >= 3 ? td : ta;
        if (!key) continue;
        if (d >= 0 && d <= 5 && l >= 0 && l < G.L) continue;
        if (d === -1 && l >= 0 && l < G.L) continue;
        if (plot?.city && d === 6 && l === cityL) continue;
        const [x, y] = tileTL(G, d, l);
        if (!(x + 192 > 0 && x < FRAME_W && y + 96 > 0 && y < FRAME_H)) continue;
        const ring = d >= -1 && d <= 6 && l >= -1 && l <= G.L;
        if (!ring && treeHash(d, l) % 100 >= 70) continue;
        out.push({ key, d, l });
      }
    if (plot?.city) out.push({ key: cityKey(plot, sides), d: 6, l: cityL, city: true });
    return out.map((s) => {
      const [x, y] = tileTL(G, s.d, s.l);
      // sunk into the falloff: the dark of the ground under it, in steps
      const a = darkAt(G, x + 96, y + 64) * (s.city ? 0.8 : 1);
      const dark = Math.round(a * 5) / 5;
      return { ...s, x, y, base: y + 96, dark };
    });
  }

  /** The ranks' names along the near edge, one a depth, on the ground just outside the near lane. */
  labels() {
    const g = this.g;
    g.save();
    g.font = "20px Herald, Georgia, serif";
    g.textAlign = "center";
    g.textBaseline = "middle";
    for (let d = 0; d <= 5; d++) {
      const side = d <= 2 ? 0 : 1;
      const row = side === 0 ? 2 - d : d - 3;
      const [x, y] = gridPt(this.G, d, this.G.L - 1 + LABEL_AT);
      const t = ROWS[row];
      // a soft dark edge and a 1 px drop shadow: they read on bright sand and over trees
      g.globalAlpha = 0.45;
      g.strokeStyle = "#05040c";
      g.lineWidth = 3;
      g.lineJoin = "round";
      g.strokeText(t, x, y);
      g.globalAlpha = 0.9;
      g.fillStyle = "#05040c";
      g.fillText(t, x + 1, y + 1);
      g.globalAlpha = 0.92;
      g.fillStyle = side === this.me ? "#bcd0ff" : "#ffc2b8";
      g.fillText(t, x, y);
    }
    g.restore();
  }

  /** Draws a scene: `{figs: [Fig], cells: [{side, lane, row, color}], fx: [Fx], bars: true,
   *  shake: [dx, dy] | null}` (`shake`: the whole scene drawn that far off; the transform stays for
   *  what is drawn over it, until the next `draw`). Fig `{b, x, y, side, large, pic (null: its ring
   *  alone), flip, alpha, ring (false: none), ringAlpha, sel, glow, tint, tintA, dx, dy, sy (a
   *  vertical squash about the feet: breathing), fallen, bar: {hp, max} | null, odds: text | null,
   *  oddsHot}`; Fx `{type: "float", x, y, text, kind ("dmg", "crit", "heal", "miss"), t}`, `t` its
   *  progress 0..1. */
  draw(sc) {
    const g = this.g;
    this.base();
    g.clearRect(0, 0, FRAME_W, FRAME_H);
    if (!this.board) return;
    if (sc.shake) this.base(Math.round(sc.shake[0]), Math.round(sc.shake[1]));
    g.drawImage(this.board, 0, 0);
    for (const c of sc.cells ?? []) {
      const [x, y] = cellXY(this.G, c.side, c.lane, c.row);
      g.save();
      diamond(g, x, y - 2);
      g.fillStyle = c.fill ?? "rgba(246, 198, 80, 0.13)";
      g.fill();
      g.strokeStyle = c.color ?? "rgba(246, 198, 80, 0.6)";
      g.lineWidth = 1.5;
      g.stroke();
      g.restore();
    }
    const figs = sc.figs ?? [];
    for (const f of figs) if (f.ring !== false) this.ring(f);
    const items = [...this.scenery.map((s) => ({ s, base: s.base })), ...figs.map((f) => ({ f, base: f.y + (f.fallen ? -4 : 0) }))];
    items.sort((a, b) => a.base - b.base);
    for (const it of items) {
      if (it.s) {
        const im = IMG.get(it.s.key);
        if (im) g.drawImage(darkImage(it.s.key, im, it.s.dark), it.s.x, it.s.y + 128 - im.height - (it.s.city ? 10 : 0));
      } else this.fig(it.f);
    }
    this.labels();
    if (sc.bars !== false) for (const f of figs) this.bar(f);
    for (const e of sc.fx ?? []) drawFx(g, e);
  }

  /** A unit's ground mark: a soft contact shadow, then its side's ring with a faint fill. */
  ring(f) {
    const g = this.g;
    const [c0, c1] = f.side === this.me ? COLORS.azure : COLORS.gules;
    const w = f.large ? 70 : 34;
    const h = f.large ? 30 : 15;
    const a = f.ringAlpha ?? f.alpha ?? 1;
    g.save();
    g.fillStyle = "#05040c";
    for (const [k, al] of [[1.14, 0.17], [0.96, 0.22]]) {
      g.globalAlpha = a * al;
      g.beginPath();
      g.ellipse(f.x + 4, f.y + 3, w * k, h * k, 0, 0, Math.PI * 2);
      g.fill();
    }
    g.beginPath();
    g.ellipse(f.x, f.y, w, h, 0, 0, Math.PI * 2);
    g.globalAlpha = a * 0.22;
    g.fillStyle = c1;
    g.fill();
    g.globalAlpha = a;
    g.lineWidth = f.sel ? 3 : 2;
    g.strokeStyle = f.sel ?? c0;
    g.stroke();
    g.restore();
  }

  fig(f) {
    const g = this.g;
    if (!f.pic) return;
    if (f.fallen) {
      drawFallen(g, f.pic, f.x + (f.dx || 0), f.y, { flip: f.flip, alpha: f.alpha ?? 0.85, side: f.side });
      return;
    }
    drawPic(g, f.pic, f.x, f.y, f);
  }

  /** The HP bar over a unit's head, and its chance to fall beside it. */
  bar(f) {
    if (!f.bar) return;
    const g = this.g;
    const w = f.large ? 48 : 36;
    const x = Math.round(f.x + (f.dx || 0) - w / 2);
    const y = Math.round(f.y + (f.dy || 0) - (f.large ? 124 : 108));
    const k = Math.max(0, Math.min(1, f.bar.hp / Math.max(1, f.bar.max)));
    g.save();
    g.globalAlpha = f.alpha ?? 1;
    // a 1 px dark frame round a 5 px fill (a lighter top line) and the side's underline
    g.fillStyle = "#070814";
    g.fillRect(x - 1, y - 1, w + 2, 8);
    g.fillStyle = "#262a3c";
    g.fillRect(x, y, w, 5);
    const fw = Math.round(w * k);
    if (fw > 0) {
      const col = hpColor(k);
      g.fillStyle = col;
      g.fillRect(x, y, fw, 5);
      g.fillStyle = HP_LIGHT[col] ?? "rgba(255, 255, 255, 0.45)";
      g.fillRect(x, y, fw, 1);
    }
    g.fillStyle = (f.side === this.me ? COLORS.azure : COLORS.gules)[0];
    g.fillRect(x, y + 5, w, 1);
    if (f.odds && f.odds !== "0%") {
      g.font = "10px Herald, Georgia, serif";
      g.textAlign = "left";
      g.textBaseline = "middle";
      const tw = Math.ceil(g.measureText(f.odds).width);
      const px = x + w + 4;
      g.fillStyle = f.oddsHot ? "rgba(70, 12, 10, 0.88)" : "rgba(7, 8, 20, 0.82)";
      g.beginPath();
      g.roundRect(px, y - 3, tw + 8, 12, 3);
      g.fill();
      g.fillStyle = f.oddsHot ? "#ffb4a8" : "#e8ecf6";
      g.fillText(f.odds, px + 4, y + 3.5);
    }
    g.restore();
  }
}

/** The hit box of a figure at (x, y) (the screens' pointer tests). */
export function figBox(f, x, y) {
  return { x0: x - (f.large ? 56 : 30), x1: x + (f.large ? 56 : 30), y0: y - (f.large ? 118 : 100), y1: y + 10 };
}

// ---------------------------------------------------------------------------------------------
// Figures: the sprite or an animation frame

/** Whether fighter `f` is drawn flipped to face the enemy (`toward`: "left" or "right" instead). */
export function faceFlip(f, sides, toward) {
  const fig = figOf(f, sides);
  // The attacker stands upper left and faces lower right; the defender faces upper left.
  const wantLeft = toward === "left" || (toward === undefined && f.side === 1);
  return wantLeft !== fig.left;
}

function frameAt(ms, list, loop) {
  const total = list.reduce((a, b) => a + b, 0);
  let t = loop ? ((ms % total) + total) % total : Math.min(Math.max(ms, 0), total - 1);
  for (let k = 0; k < list.length; k++) {
    if (t < list[k]) return k;
    t -= list[k];
  }
  return list.length - 1;
}

/** The picture of fighter `f`: animation `anim` at `ms` when it has the set, its key pose when
 *  `anim` names one it has ("attack", "hit"), else its sprite. */
export function pic(f, sides, anim = null, ms = 0, loop = false) {
  const fig = figOf(f, sides);
  if (anim && fig.poses.has(anim)) {
    // a big unit's pose has a bigger canvas: the feet at its bottom centre, 2 px up, as on the 160 x 128 one
    const im = IMG.get(`pose:${fig.short}:${anim}`);
    if (im) return { im, sx: 0, w: im.width, h: im.height, ax: im.width / 2, ay: im.height - 2, anim };
  }
  if (anim && fig.anim) {
    const im = IMG.get(`anim:${fig.anim}:${anim}`);
    if (im) return { im, sx: frameAt(ms, ANIMS[anim], loop) * ANIM_FRAME.w, w: ANIM_FRAME.w, h: ANIM_FRAME.h, ax: ANIM_FRAME.ax, ay: ANIM_FRAME.ay, anim };
  }
  let im = IMG.get(fig.key);
  if (im && fig.scale) im = scaledImage(fig.key, im, fig.scale);
  return im ? { im, sx: 0, w: im.width, h: im.height, ax: im.width / 2, ay: im.height - 2 } : null;
}

const SCALED = new Map();

/** `im` scaled by `k` (smoothed), drawn once per key. */
function scaledImage(key, im, k) {
  const id = `${key}@${k}`;
  let c = SCALED.get(id);
  if (!c) {
    c = document.createElement("canvas");
    c.width = Math.max(1, Math.round(im.width * k));
    c.height = Math.max(1, Math.round(im.height * k));
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = true;
    g.drawImage(im, 0, 0, c.width, c.height);
    SCALED.set(id, c);
  }
  return c;
}

/** Layer `layer` ("back" or "front") of impact card `card`, or null (not loaded, or missing). */
export const impactPic = (card, layer) => IMG.get(`fx:${card}:${layer}`) ?? null;

/** The last frame of the death animation (the fallen), or null when it has none. */
export function deadPic(f, sides) {
  const fig = figOf(f, sides);
  return fig.anim ? pic(f, sides, "death", 1e9, false) : null;
}

const SIL = new Map();

/** The picture's silhouette in `color` (a flash, a glow, an outline), cached. */
export function silhouette(p, flip, color) {
  const key = `${p.im.src}|${p.sx}|${flip}|${color}`;
  let c = SIL.get(key);
  if (c) return c;
  c = document.createElement("canvas");
  c.width = p.w;
  c.height = p.h;
  const g = c.getContext("2d");
  if (flip) {
    g.translate(p.w, 0);
    g.scale(-1, 1);
  }
  g.drawImage(p.im, p.sx, 0, p.w, p.h, 0, 0, p.w, p.h);
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.globalCompositeOperation = "source-in";
  g.fillStyle = color;
  g.fillRect(0, 0, p.w, p.h);
  SIL.set(key, c);
  return c;
}

function drawPic(g, p, x, y, { flip = false, alpha = 1, dx = 0, dy = 0, sy = 1, tint = null, tintA = 1, glow = null, glowA = 1 } = {}) {
  const X = Math.round(x + dx);
  const Y = Math.round(y + dy);
  const x0 = X - (flip ? p.w - p.ax : p.ax);
  const y0 = Y - p.ay;
  if (sy !== 1) {
    // squashed about the feet: the same drawing under a vertical scale
    g.save();
    g.translate(0, Y);
    g.scale(1, sy);
    g.translate(0, -Y);
    drawPic(g, p, x, y, { flip, alpha, dx, dy, tint, tintA, glow, glowA });
    g.restore();
    return;
  }
  if (glow) {
    const s = silhouette(p, flip, glow);
    g.save();
    g.globalAlpha = glowA * alpha;
    for (const [ox, oy] of [[-2, 0], [2, 0], [0, -2], [0, 2], [-1, -1], [1, 1], [-1, 1], [1, -1]]) g.drawImage(s, x0 + ox, y0 + oy);
    g.restore();
  }
  g.save();
  g.globalAlpha = alpha;
  if (flip) {
    g.translate(x0 + p.w, y0);
    g.scale(-1, 1);
    g.drawImage(p.im, p.sx, 0, p.w, p.h, 0, 0, p.w, p.h);
  } else g.drawImage(p.im, p.sx, 0, p.w, p.h, x0, y0, p.w, p.h);
  g.restore();
  if (tint && tintA > 0) {
    g.save();
    g.globalAlpha = tintA * alpha;
    g.drawImage(silhouette(p, flip, tint), x0, y0);
    g.restore();
  }
}

/** The canvas rotation (radians, clockwise on screen: + tips the head right) that leans a figure
 *  by `lean` (+ the way it faces) when it is drawn mirrored or not (`flip`) and its art faces left
 *  or right (`left`: figOf's; the Champion's faces left). It faces left as drawn when exactly one
 *  of the two holds. */
export const leanTurn = (lean, flip, left = false) => (flip !== left ? -lean : lean);

/** Picture `p` with its feet at (x, y) under a transform about the feet: `scale`, a vertical
 *  squash `sy`, a `lean` (radians; + leans the way the figure faces: `left` says which way its art
 *  faces, `leanTurn`), with a `glow` round it and a `tint` over it (the watch screen's snap,
 *  battle_snap.js). */
export function drawPicX(g, p, x, y, { flip = false, left = false, alpha = 1, tint = null, tintA = 0, glow = null, scale = 1, sy = 1, lean = 0 } = {}) {
  g.save();
  g.translate(Math.round(x), Math.round(y));
  if (lean) g.rotate(leanTurn(lean, flip, left));
  g.scale(flip ? -scale : scale, scale * sy);
  const x0 = -p.ax;
  const y0 = -p.ay;
  if (glow) {
    const sil = silhouette(p, false, glow);
    g.globalAlpha = alpha * 0.95;
    for (const [ox, oy] of [[-2, 0], [2, 0], [0, -2], [0, 2], [-1, -1], [1, 1], [-1, 1], [1, -1]]) g.drawImage(sil, x0 + ox, y0 + oy);
  }
  g.globalAlpha = alpha;
  g.drawImage(p.im, p.sx, 0, p.w, p.h, x0, y0, p.w, p.h);
  if (tint && tintA > 0) {
    g.globalAlpha = tintA * alpha;
    g.drawImage(silhouette(p, false, tint), x0, y0);
  }
  g.restore();
}

/** A unit with no death frames, fallen: its sprite laid on its side at its feet (falling back,
 *  away from the enemy) and darkened. */
function drawFallen(g, p, x, y, { flip = false, alpha = 0.85, side = 0 } = {}) {
  g.save();
  g.globalAlpha = alpha;
  g.translate(Math.round(x), Math.round(y - 4));
  g.rotate(side === 0 ? -Math.PI / 2 : Math.PI / 2);
  if (flip) g.scale(-1, 1);
  g.drawImage(p.im, p.sx, 0, p.w, p.h, -p.ax, -p.ay, p.w, p.h);
  g.drawImage(silhouette(p, false, "rgba(10,8,20,0.55)"), -p.ax, -p.ay);
  g.restore();
}

// ---------------------------------------------------------------------------------------------
// Effects

/** An effect at its progress `t` (0..1): a word floating up (the watch screen's actions with no
 *  blow; battle_snap.js draws the blows). */
function drawFx(g, e) {
  const t = e.t;
  if (t < 0 || t > 1) return;
  g.save();
  if (e.type === "float") {
    const y = e.y - 34 * Math.min(1, t * 1.6);
    const big = e.kind === "crit";
    g.globalAlpha = t < 0.75 ? 1 : 1 - (t - 0.75) / 0.25;
    g.font = `700 ${big ? 26 : e.kind === "miss" ? 16 : 20}px Herald, Georgia, serif`;
    g.textAlign = "center";
    g.lineJoin = "round";
    g.lineWidth = 4;
    g.strokeStyle = "rgba(8, 6, 18, 0.9)";
    g.strokeText(e.text, e.x, y);
    g.fillStyle = { crit: COLORS.goldLt, heal: "#8dffb4", miss: "#d6dbe8", dmg: "#ffffff" }[e.kind] ?? "#ffffff";
    g.fillText(e.text, e.x, y);
  }
  g.restore();
}

// ---------------------------------------------------------------------------------------------
// DOM marks

/** A tiny pixel glyph as inline SVG: rows of '#' (colour) and 'o' (outline) at `s` px a pixel. */
export function pixGlyph(rows, color, s = 2, outline = "#0c0703") {
  const ns = "http://www.w3.org/2000/svg";
  const h = rows.length;
  const w = rows[0].length;
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", "bt-px");
  svg.setAttribute("width", String(w * s));
  svg.setAttribute("height", String(h * s));
  svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
  svg.setAttribute("shape-rendering", "crispEdges");
  svg.setAttribute("aria-hidden", "true");
  rows.forEach((row, y) =>
    [...row].forEach((c, x) => {
      if (c !== "#" && c !== "o") return;
      const r = document.createElementNS(ns, "rect");
      r.setAttribute("x", String(x));
      r.setAttribute("y", String(y));
      r.setAttribute("width", "1");
      r.setAttribute("height", "1");
      r.setAttribute("fill", c === "#" ? color : outline);
      svg.append(r);
    }),
  );
  return svg;
}

export const GLYPH = {
  play: ["#...", "##..", "###.", "####", "###.", "##..", "#..."],
  pause: ["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"],
  step: ["#..#", "##.#", "####", "##.#", "#..#"],
  skip: ["#..#..", "##.##.", "######", "##.##.", "#..#.."],
};

const HEADS = new Map();

/** Where the head of sprite `im` is: going down from its top, the first three rows whose longest
 *  run of opaque pixels is 13 or more (a raised sword, spear, staff or bow is narrower, outline
 *  and all: it is passed over), centred on the third row's run. */
function headOf(key, im) {
  let s = HEADS.get(key);
  if (s) return s;
  s = { cx: im.width / 2, y0: 0 };
  try {
    const c = document.createElement("canvas");
    c.width = im.width;
    c.height = im.height;
    const g = c.getContext("2d");
    g.drawImage(im, 0, 0);
    const d = g.getImageData(0, 0, im.width, im.height).data;
    const W = im.width;
    const H = im.height;
    /** The longest run of opaque pixels in row `y`: [start, length]. */
    const run = (y) => {
      let best = [0, 0];
      let start = -1;
      for (let x = 0; x <= W; x++) {
        const on = x < W && d[(y * W + x) * 4 + 3] > 64;
        if (on && start < 0) start = x;
        if (!on && start >= 0) {
          if (x - start > best[1]) best = [start, x - start];
          start = -1;
        }
      }
      return best;
    };
    const runs = [];
    for (let y = 0; y < H; y++) runs.push(run(y));
    for (let y = 0; y + 3 <= H; y++) {
      if (runs[y][1] >= 13 && runs[y + 1][1] >= 13 && runs[y + 2][1] >= 13) {
        const [x0, n] = runs[y + 2];
        s = { cx: Math.round(x0 + n / 2), y0: Math.max(0, y - 3) };
        break;
      }
    }
  } catch {
    /* no pixels to read: the middle of the sprite */
  }
  HEADS.set(key, s);
  return s;
}

/** A head-and-shoulders crop of fighter `f`'s sprite (a portrait chip at 1x), drawn once the
 *  sprite has loaded. */
export function headChip(f, sides, { w = 32, h = 30, dead = false, flip = false } = {}) {
  const key = figOf(f, sides).key;
  const c = document.createElement("canvas");
  c.className = `bt-head${dead ? " dead" : ""}`;
  c.width = w;
  c.height = h;
  c.style.width = `${w}px`;
  c.style.height = `${h}px`;
  c.setAttribute("aria-hidden", "true");
  sprite(key).then((im) => {
    if (!im) return;
    const { cx, y0 } = headOf(key, im);
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = false;
    if (flip) {
      g.translate(w, 0);
      g.scale(-1, 1);
    }
    g.drawImage(im, Math.round(cx - w / 2), Math.max(0, y0 - 2), w, h, 0, 0, w, h);
  });
  return c;
}

/** The M7 interface icon `ui_<name>_s` as an image of `px` CSS px. */
export function uiIcon(name, px = 16) {
  const im = document.createElement("img");
  const key = SPRITES[`ui_${name}_s`] ? `ui_${name}_s` : "ui_strength_s";
  im.src = spriteUrl(key);
  im.width = px;
  im.height = px;
  im.alt = "";
  im.className = "bt-ic";
  return im;
}

/** The lighter top line of each health colour (the stage's bars). */
const HP_LIGHT = { "#5ad16a": "#a6f2ae", "#f0b040": "#ffd98a", "#ee5040": "#ff9a8a" };

/** A health colour, as the map's bars. */
export const hpColor = (p) => (p > 0.5 ? "#5ad16a" : p > 0.25 ? "#f0b040" : "#ee5040");

/** Appends `children` to `el` as dom.js `h` does: arrays flattened, null, undefined and false
 *  skipped, the rest as text. */
export function add(el, ...children) {
  for (const c of children.flat(Infinity)) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

/** A button of the battle screens: its label as plain text (no text hook), `on` lit. */
/** The battle's ground as one gold line of names ("Hills · Forest"), each name's rules as its
 *  tooltip (the screens keep rules out of their text). */
export function groundLine(ground, plot) {
  const line = document.createElement("div");
  line.className = "small gold bt-ground";
  groundTags(ground, plot).forEach((g, i) => {
    if (i) line.append(" · ");
    const s = document.createElement("span");
    s.textContent = g.label;
    s.title = g.text;
    line.append(s);
  });
  return line;
}

export function btn(label, { cls = "", title, onclick, on = false, disabled = false, key } = {}) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = `bt-btn${cls ? ` ${cls}` : ""}${on ? " on" : ""}`;
  if (title) b.title = title;
  if (key) b.dataset.k = key;
  if (disabled) b.disabled = true;
  if (onclick) b.addEventListener("click", onclick);
  for (const c of [label].flat()) {
    if (c === null || c === undefined || c === false) continue;
    b.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  b.setAttribute("aria-pressed", String(!!on));
  return b;
}

/** Text parts `[{text, cls}]` (or strings) as text and spans. */
export function partsEl(parts) {
  const out = document.createDocumentFragment();
  for (const p of parts) {
    if (typeof p === "string") out.append(document.createTextNode(p));
    else if (p.cls) {
      const s = document.createElement("span");
      s.className = p.cls;
      s.textContent = p.text;
      out.append(s);
    } else out.append(document.createTextNode(p.text));
  }
  return out;
}

/** A health bar `w` px wide: `hp` of `max` (any unit). */
export function hpBar(hp, max, { w = 48 } = {}) {
  const bar = document.createElement("span");
  bar.className = "bt-hp";
  bar.style.width = `${w}px`;
  const q = Math.max(0, hp) / Math.max(1, max);
  const v = document.createElement("i");
  v.style.width = `${Math.round(q * w)}px`;
  v.style.background = hpColor(q);
  bar.append(v);
  return bar;
}
