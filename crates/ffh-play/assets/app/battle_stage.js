// The battle stage (the battle UI design, DESIGN.md sections 2-5): the attacker's plot and the
// defended plot lifted out of the dimmed map as one isometric board of M7 tiles at 1x, the two
// forces on it in two ranks, the units that stay out off its corners. The same stage serves the
// plan, the battle and the report. Drawing is a function of a scene description (figures, arrows,
// rings, sparks ...), so the watch screen can seek and step back freely. Canvas and DOM; the
// geometry is the design's 1280 x 800 frame (battle.js centres it).
//
// Art: the M7 sprites (`app/iso`, docs/ISO.md) and, for the units that have them, the anim-sprint
// sets (`app/anim/<unit>_<idle|strike|flinch|death|walk>.png`: strips of 160 x 128 frames, the feet
// at (80, 126); anim-sprint/ANIMATING_UNITS.md). Pixel art is never scaled.

import { SPRITES } from "./iso/sprites.js";
import { unitSpriteKeys, ISO_TABLES } from "./iso.js";
import { tyName, tyIdx, roleKey, committed, isBack } from "./battle_log.js";

export const FRAME_W = 1280;
export const FRAME_H = 800;
/** The board's top vertex, and how much the defended plot stands above the other on a hill. */
const OX = 640;
const OY = 78;
const HILL_RAISE = 16;
/** The board's centre line (the units' rows are spread round it). */
const CY = 337;

const spriteUrl = (key) => new URL(`./iso/${key}.png`, import.meta.url).href;
const animUrl = (unit, anim) => new URL(`./anim/${unit}_${anim}.png`, import.meta.url).href;

/** The anim-sprint sets the client ships (their timings are the sets' manifests'), by the short
 *  name of the M7 sprite they animate; `left`: the set faces lower-left. */
const ANIM_SETS = { axeman: {}, champion: { left: true }, phalanx: {}, swordsman: {}, warrior: {} };
export const ANIMS = {
  idle: [250, 250, 250, 250],
  strike: [100, 120, 160, 50, 120, 120, 100, 100],
  flinch: [80, 120, 120, 120],
  death: [80, 120, 140, 200, 80, 80, 80, 400],
  walk: [110, 110, 110, 110, 110, 110],
};
const ANIM_FRAME = { w: 160, h: 128, ax: 80, ay: 126 };
/** M7 unit sprites that face lower-left (the rest face lower-right). */
const FACES_LEFT = new Set(["champion"]);

/** A role's icon on its unit's ring (the M7 interface icons). */
const ROLE_ICON = {
  frontline: "fortify",
  screen: "defense",
  flanker: "moves",
  assassin: "attack",
  archer: "rangeattack",
  siege: "rangeattack",
  arcane: "spell",
  divine: "spell",
};
export const roleIcon = (f) => ROLE_ICON[roleKey(f)] ?? "strength";

export const COLORS = {
  you: "#8fb6ff",
  them: "#ff9a86",
  gold: "#f6c650",
  goldLt: "#ffe58c",
  cyan: "#4fe3ff",
  azure: ["#3356b6", "#0b4290"],
  gules: ["#c8272b", "#8f1e1f"],
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
  const civ = civShort(sides?.[f.side]?.shown_civ ?? sides?.[f.side]?.civ);
  const t = tyIdx("UnitType", f.unit_type);
  const keys = t >= 0 ? unitSpriteKeys(t, civ) : [];
  const own = tyName("UnitType", f.unit_type);
  if (own) keys.unshift(`unit_${own.replace(/^UNIT_/, "").toLowerCase()}`);
  const key = keys.find((k) => SPRITES[k]) ?? "unit_swordsman";
  const short = key.replace(/^unit_/, "");
  fig = { key, anim: ANIM_SETS[short] ? short : null, left: FACES_LEFT.has(short) };
  FIGS.set(f, fig);
  return fig;
}

/** Whether fighter `f` has an animation set (it strikes with its frames). */
export const animated = (sides) => (f) => !!figOf(f, sides).anim;

/** Loads the art a battle needs: the board's tiles and overlays, the city, every fighter's sprite
 *  and animation strips, and the role icons. */
export function loadArt({ fighters, plot, from, sides }) {
  const jobs = [];
  for (const p of [plot, from]) {
    if (!p) continue;
    const t = tileKey(p.terrain);
    jobs.push(sprite(t), sprite(`${t}_b`), sprite(hillsKey(p.terrain)));
    for (const k of featureKeys(p.feature)) jobs.push(sprite(k));
  }
  if (plot?.city) jobs.push(sprite(cityKey(plot, sides)));
  for (const f of fighters) {
    const fig = figOf(f, sides);
    jobs.push(sprite(fig.key));
    if (fig.anim) for (const a of Object.keys(ANIMS)) jobs.push(loadImage(`anim:${fig.anim}:${a}`, animUrl(fig.anim, a)));
  }
  for (const n of new Set([...Object.values(ROLE_ICON), "strength"])) jobs.push(sprite(`ui_${n}_s`));
  return Promise.all(jobs);
}

// ---------------------------------------------------------------------------------------------
// The board

const DRY = /PLAINS|DESERT|BURNING_SANDS|BROKEN_LANDS|FIELDS_OF_PERDITION/;
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

function featureKeys(feature) {
  const n = tyName("FeatureType", feature);
  return (n && ISO_TABLES.FEATURE_SPRITES[n]?.filter((k) => SPRITES[k]).slice(0, 1)) || [];
}

function cityKey(plot, sides) {
  const c = plot.city;
  const civ = civShort(sides?.find((s) => s.player === c.owner)?.civ ?? sides?.[1]?.civ);
  return ISO_TABLES.citySprites(c.size ?? 4, civ).find((k) => SPRITES[k]) ?? "bld_city";
}

/** Tile sprites are 192 x 128: the top diamond in rows 0-95 (vertices (96, 0) (192, 48) (96, 96)
 *  (0, 48)), the earth sides below. Tile (i, j) of the board; i > j is the defended plot. */
function tileXY(ox, oy, i, j) {
  return [ox + (i - j) * 96 - 96, oy + (i + j) * 48];
}

/** The tiles of each plot that show its hills or its feature (round the units' ranks). */
const HILL_TILES = { def: [[2, 0], [3, 1], [4, 2], [4, 3], [3, 0], [4, 1]], att: [[0, 2], [1, 3], [2, 4], [0, 3], [1, 4]] };
const FEATURE_TILES = { def: [[1, 0], [4, 3]], att: [[0, 1], [3, 4]] };

/** The stage: a canvas of the design's frame, the board cached per screen. */
export class Stage {
  constructor(canvas) {
    this.canvas = canvas;
    canvas.width = FRAME_W;
    canvas.height = FRAME_H;
    this.g = canvas.getContext("2d");
    this.g.imageSmoothingEnabled = false;
    this.board = null;
    this.dy = 0;
    this.raise = 0;
    this.shake = 0;
    this.city = null;
    /** The viewer's side: its rings and band are Azure, the other side's Gules. */
    this.me = 0;
  }

  /** The board for `model` ({plot, from, sides}), shifted down by `dy` (the report raises it). */
  setBoard(model, dy = 0) {
    const { plot, from, sides } = model;
    this.dy = dy;
    this.raise = plot?.hills ? HILL_RAISE : 0;
    const c = document.createElement("canvas");
    c.width = FRAME_W;
    c.height = FRAME_H;
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = false;
    const n = 5;
    const oy = OY + dy;
    g.fillStyle = "rgba(0, 4, 20, 0.45)";
    g.beginPath();
    g.moveTo(OX, oy + 40);
    g.lineTo(OX + n * 96 + 30, oy + n * 48 + 40);
    g.lineTo(OX, oy + n * 96 + 64);
    g.lineTo(OX - n * 96 - 30, oy + n * 48 + 40);
    g.fill();
    const defT = tileKey(plot?.terrain);
    const attT = tileKey(from?.terrain);
    const overlays = [];
    if (plot?.hills) for (const [i, j] of HILL_TILES.def) overlays.push([i, j, hillsKey(plot.terrain)]);
    if (from?.hills) for (const [i, j] of HILL_TILES.att) overlays.push([i, j, hillsKey(from.terrain)]);
    for (const k of featureKeys(plot?.feature)) for (const [i, j] of FEATURE_TILES.def) overlays.push([i, j, k]);
    for (const k of featureKeys(from?.feature)) for (const [i, j] of FEATURE_TILES.att) overlays.push([i, j, k]);
    for (let s = 0; s <= 2 * n - 2; s++) {
      for (let i = 0; i < n; i++) {
        const j = s - i;
        if (j < 0 || j >= n) continue;
        const def = i > j;
        let [x, y] = tileXY(OX, oy, i, j);
        if (def) y -= this.raise;
        const alt = (i * 31 + j * 17) % 3 === 0;
        const base = def ? defT : attT;
        const t = (alt && IMG.get(`${base}_b`)) || IMG.get(base);
        if (t) g.drawImage(t, x, y);
        for (const [oi, oj, key] of overlays) {
          const im = IMG.get(key);
          if (oi !== i || oj !== j || !im) continue;
          g.drawImage(im, x, y + (im.height < 128 ? 128 - im.height : 0) - (key.startsWith("feat") ? im.height - 128 : 0));
        }
      }
    }
    this.board = c;
    this.city = plot?.city ? cityKey(plot, sides) : null;
  }

  /** Draws a scene: `{figs: [Fig], bands: [{side, alpha}], rings, arrows, numbered, projs, sparks,
   *  shards, cityFlag}`; Fig `{x, y, pic, flip, alpha, ring (false: none), ringAlpha, side, sel,
   *  glow, glowA, tint, tintA, dx, dy, fallen, roleIcon, plate}`. */
  draw(sc) {
    const g = this.g;
    g.clearRect(0, 0, FRAME_W, FRAME_H);
    g.save();
    if (this.shake) g.translate(Math.round((Math.random() - 0.5) * 2 * this.shake), 0);
    if (this.board) g.drawImage(this.board, 0, 0);
    for (const b of sc.bands ?? []) this.band(b);
    for (const r of sc.rings ?? []) targetRing(g, r.x, r.y, r.color ?? COLORS.cyan, r.alpha ?? 1, r.dash ?? true);
    const items = [];
    const city = this.city && IMG.get(this.city);
    if (city) {
      const [cx, cy] = tileXY(OX, OY + this.dy, 4, 0);
      const top = cy - this.raise + 128 - city.height;
      items.push({
        y: cy - this.raise + 70,
        draw: () => {
          g.drawImage(city, cx, top);
          if (sc.cityFlag) drawFlag(g, cx + 128, top + 6, sc.cityFlag);
        },
      });
    }
    for (const f of sc.figs ?? []) items.push({ y: f.y + (f.fallen ? -4 : 0), draw: () => this.fig(f) });
    items.sort((a, b) => a.y - b.y);
    for (const it of items) it.draw();
    for (const a of sc.arrows ?? []) arrow(g, a.x0, a.y0, a.x1, a.y1, a);
    for (const n of sc.numbered ?? []) numberedBadge(g, n.x, n.y, n.n);
    for (const p of sc.projs ?? []) drawProj(g, p);
    for (const s of sc.sparks ?? []) drawSpark(g, s);
    for (const s of sc.shards ?? []) {
      g.fillStyle = s.color;
      g.globalAlpha = s.alpha;
      g.fillRect(Math.round(s.x), Math.round(s.y), 4, 4);
      g.globalAlpha = 1;
    }
    for (const l of sc.lines ?? []) {
      g.save();
      g.strokeStyle = COLORS.goldLt;
      g.lineWidth = 2;
      g.setLineDash([4, 4]);
      g.beginPath();
      g.moveTo(l.x0, l.y0);
      g.lineTo(l.x1, l.y1);
      g.stroke();
      g.restore();
    }
    g.restore();
  }

  fig(f) {
    const g = this.g;
    if (f.ring !== false) ring(g, f.x, f.y, f.side === this.me ? COLORS.azure : COLORS.gules, f.ringAlpha ?? f.alpha ?? 1);
    if (f.plate) questionPlate(g, f.x, f.y);
    if (f.sel) selRing(g, f.x, f.y, f.sel);
    if (!f.pic) return;
    if (f.fallen) {
      drawFallen(g, f.pic, f.x + (f.dx || 0), f.y, { flip: f.flip, alpha: f.alpha ?? 0.85 });
      return;
    }
    drawPic(g, f.pic, f.x, f.y, f);
    const ic = f.roleIcon && IMG.get(`ui_${f.roleIcon}_s`);
    if (ic) g.drawImage(ic, Math.round(f.x - 34), Math.round(f.y - 9), 16, 16);
  }

  /** The shield band on the ground between a side's ranks. */
  bandX(side) {
    return side === 0 ? 462 : 816;
  }

  bandY(side) {
    const r = side === 1 ? this.raise : 0;
    return [196 + this.dy - r, 470 + this.dy - r];
  }

  band(b) {
    const [y0, y1] = this.bandY(b.side);
    shieldBand(this.g, this.bandX(b.side), y0, y1, b.side === this.me ? COLORS.you : COLORS.them, b.alpha ?? 1);
  }

  /** The band of `side` breaking at progress `t` (0..1): its shards. */
  shards(side, t) {
    const x = this.bandX(side);
    const [y0] = this.bandY(side);
    const out = [];
    for (let k = 0; k < 18; k++) {
      const y = y0 + 10 + k * 15;
      const vx = (((k * 37) % 11) - 5) * 0.12;
      const vy = -0.2 - ((k * 13) % 5) * 0.05;
      out.push({ x: x - 4 + vx * t * 120, y: y + vy * t * 120 + 0.0008 * (t * 400) ** 2, alpha: Math.max(0, 1 - t), color: side === this.me ? COLORS.you : COLORS.them });
    }
    return out;
  }

  /** Where the city stands (the report's arrow into it, the flag). */
  cityPoint() {
    const [cx, cy] = tileXY(OX, OY + this.dy, 4, 0);
    return { x: cx + 96, y: cy - this.raise + 40 };
  }
}

// ---------------------------------------------------------------------------------------------
// Where everyone stands

/** Feet positions `{b: {x, y, reserve}}` for fighters with display slots: each side's front and
 *  back ranks, the units out of the fight off the board's corners. `raise`: the defended plot's. */
export function layout(fighters, { dy = 0, squeeze = 1, raise = 0 } = {}) {
  const pos = {};
  for (const side of [0, 1]) {
    const mine = fighters.filter((f) => f.side === side && committed(f)).sort((a, b) => a.slot - b.slot);
    for (const back of [false, true]) {
      const r = mine.filter((f) => isBack(f) === back);
      const n = r.length;
      const step = n <= 1 ? 0 : n === 2 ? 112 : n === 3 ? 92 : n <= 5 ? 74 : Math.max(36, Math.floor(296 / (n - 1)));
      r.forEach((f, k) => {
        const y = CY + (k - (n - 1) / 2) * step;
        let x;
        if (side === 0) x = !back ? 548 + (k % 2 ? -28 : 26) : 384 + (k % 2 ? -16 : 12);
        else x = !back ? 736 + (k % 2 ? 30 : -18) : 890 + (k % 2 ? -20 : 18);
        pos[f.b] = { x, y: CY + (y - CY) * squeeze - (side === 1 ? raise : 0) + dy };
      });
    }
    const res = fighters.filter((f) => f.side === side && !committed(f));
    res.forEach((f, k) => {
      const x = side === 0 ? 92 + (k % 2) * 58 : 1180 - (k % 2) * 58;
      pos[f.b] = { x, y: CY - 30 + k * Math.min(76, 300 / Math.max(1, res.length)) * squeeze + dy, reserve: true };
    });
  }
  return pos;
}

/** Where a unit that left the fight ends up: off the board on its side, after its reserves. */
export function exitPos(f, fighters, k, dy = 0) {
  const nRes = fighters.filter((g) => g.side === f.side && !committed(g)).length;
  const i = nRes + k;
  return { x: f.side === 0 ? 60 + (i % 2) * 70 : 1210 - (i % 2) * 70, y: CY - 30 + i * 70 + dy };
}

// ---------------------------------------------------------------------------------------------
// Figures: the sprite or an animation frame

/** Whether fighter `f` (of `side`) is drawn flipped to face `toward` ("left", "right"; default:
 *  the enemy). */
export function faceFlip(f, sides, toward) {
  const fig = figOf(f, sides);
  const wantLeft = toward === "left" || (toward === undefined && f.side === 1);
  return wantLeft !== fig.left;
}

function frameAt(ms, list, loop) {
  const total = list.reduce((a, b) => a + b, 0);
  let t = loop ? ((ms % total) + total) % total : Math.min(ms, total - 1);
  for (let k = 0; k < list.length; k++) {
    if (t < list[k]) return k;
    t -= list[k];
  }
  return list.length - 1;
}

/** The picture of fighter `f`: animation `anim` at `ms` when it has the set, else its sprite. */
export function pic(f, sides, anim = null, ms = 0, loop = false) {
  const fig = figOf(f, sides);
  if (anim && fig.anim) {
    const im = IMG.get(`anim:${fig.anim}:${anim}`);
    if (im) return { im, sx: frameAt(ms, ANIMS[anim], loop) * ANIM_FRAME.w, w: ANIM_FRAME.w, h: ANIM_FRAME.h, ax: ANIM_FRAME.ax, ay: ANIM_FRAME.ay };
  }
  const im = IMG.get(fig.key);
  return im ? { im, sx: 0, w: im.width, h: im.height, ax: im.width / 2, ay: im.height - 2 } : null;
}

/** The last frame of the death animation (the fallen), or null when it has none. */
export function deadPic(f, sides) {
  const fig = figOf(f, sides);
  return fig.anim ? pic(f, sides, "death", 1e9, false) : null;
}

const SIL = new Map();

/** The picture's silhouette in `color` (a flash, a glow), cached. */
function silhouette(p, flip, color) {
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

function drawPic(g, p, x, y, { flip = false, alpha = 1, dx = 0, dy = 0, tint = null, tintA = 1, glow = null, glowA = 1 } = {}) {
  const X = Math.round(x + dx);
  const Y = Math.round(y + dy);
  const x0 = X - (flip ? p.w - p.ax : p.ax);
  const y0 = Y - p.ay;
  if (glow) {
    const s = silhouette(p, flip, glow);
    g.save();
    g.globalAlpha = glowA;
    for (const [ox, oy] of [[-2, 0], [2, 0], [0, -2], [0, 2], [-2, -2], [2, 2], [-2, 2], [2, -2]]) g.drawImage(s, x0 + ox, y0 + oy);
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
  if (tint) {
    g.save();
    g.globalAlpha = tintA;
    g.drawImage(silhouette(p, flip, tint), x0, y0);
    g.restore();
  }
}

const FALLEN = new Map();

/** A unit with no death frames, fallen: its sprite tipped a quarter turn and darkened. */
function drawFallen(g, p, x, y, { flip = false, alpha = 0.85 } = {}) {
  const key = `${p.im.src}|${p.sx}|${flip}`;
  let c = FALLEN.get(key);
  if (!c) {
    c = document.createElement("canvas");
    c.width = p.h;
    c.height = p.w;
    const cg = c.getContext("2d");
    cg.imageSmoothingEnabled = false;
    cg.translate(c.width / 2, c.height / 2);
    cg.rotate(((flip ? -1 : 1) * Math.PI) / 2);
    if (flip) cg.scale(-1, 1);
    cg.drawImage(p.im, p.sx, 0, p.w, p.h, -p.w / 2, -p.h / 2, p.w, p.h);
    cg.setTransform(1, 0, 0, 1, 0, 0);
    cg.globalCompositeOperation = "source-atop";
    cg.fillStyle = "rgba(20, 13, 12, 0.25)";
    cg.fillRect(0, 0, c.width, c.height);
    FALLEN.set(key, c);
  }
  g.save();
  g.globalAlpha = alpha;
  g.drawImage(c, Math.round(x - c.width / 2), Math.round(y - c.height + 26));
  g.restore();
}

// ---------------------------------------------------------------------------------------------
// Canvas marks

function ring(g, x, y, [f, s], alpha = 1) {
  g.save();
  g.globalAlpha = alpha;
  g.fillStyle = "#140d0c";
  g.beginPath();
  g.ellipse(x, y + 1, 29, 11, 0, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = s;
  g.beginPath();
  g.ellipse(x, y + 1, 27, 9, 0, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = f;
  g.beginPath();
  g.ellipse(x, y - 1, 27, 8, 0, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = "rgba(0,0,0,0.28)";
  g.beginPath();
  g.ellipse(x, y - 1, 17, 5, 0, 0, Math.PI * 2);
  g.fill();
  g.restore();
}

/** A "may fight" plate: a defender the stance may or may not commit (an unseen stance). */
function questionPlate(g, x, y) {
  g.save();
  g.fillStyle = "#0c0703";
  g.fillRect(x + 14, y - 12, 18, 18);
  g.fillStyle = COLORS.gold;
  g.fillRect(x + 16, y - 10, 14, 14);
  g.fillStyle = "#1a1205";
  g.font = "700 20px Herald";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText("?", x + 23, y - 2);
  g.restore();
}

function selRing(g, x, y, color) {
  g.save();
  g.lineWidth = 5;
  g.strokeStyle = "#0c0703";
  g.beginPath();
  g.ellipse(x, y + 1, 36, 13, 0, 0, Math.PI * 2);
  g.stroke();
  g.lineWidth = 3;
  g.strokeStyle = color;
  g.beginPath();
  g.ellipse(x, y + 1, 36, 13, 0, 0, Math.PI * 2);
  g.stroke();
  g.restore();
}

function targetRing(g, x, y, color, alpha, dash) {
  g.save();
  g.globalAlpha = alpha;
  for (const [w, c, d] of [
    [5, "#0c0703", []],
    [3, color, dash ? [7, 5] : []],
  ]) {
    g.setLineDash(d);
    g.lineWidth = w;
    g.strokeStyle = c;
    g.beginPath();
    g.ellipse(x, y + 1, 40, 14, 0, 0, Math.PI * 2);
    g.stroke();
  }
  g.restore();
}

function numberedBadge(g, x, y, n) {
  g.save();
  g.fillStyle = "#0c0703";
  g.fillRect(x - 11, y - 11, 22, 22);
  g.fillStyle = COLORS.gold;
  g.fillRect(x - 9, y - 9, 18, 18);
  g.fillStyle = "#1a1205";
  g.font = "700 20px Herald";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(String(n), x, y + 1);
  g.restore();
}

function drawFlag(g, x, y, color) {
  g.fillStyle = "#0c0703";
  g.fillRect(x - 1, y - 2, 4, 42);
  g.fillStyle = "#e9dbb3";
  g.fillRect(x, y, 2, 40);
  g.fillStyle = "#0c0703";
  g.fillRect(x + 2, y - 1, 26, 18);
  g.fillStyle = color;
  g.fillRect(x + 3, y, 24, 16);
  g.fillStyle = "#fffaec";
  g.fillRect(x + 12, y + 5, 6, 6);
}

const SHIELD_PX = ["ooooooo", "o#####o", "o#####o", "o#####o", ".o###o.", "..o#o..", "...o..."];

function shieldBand(g, x, y0, y1, color, alpha = 1) {
  g.save();
  g.globalAlpha = 0.22 * alpha;
  g.fillStyle = color;
  g.fillRect(x - 7, y0, 14, y1 - y0);
  g.globalAlpha = alpha;
  g.setLineDash([6, 6]);
  g.lineWidth = 2;
  g.strokeStyle = color;
  g.beginPath();
  g.moveTo(x, y0);
  g.lineTo(x, y1);
  g.stroke();
  g.setLineDash([]);
  for (let y = y0 + 18; y < y1 - 6; y += 54) {
    SHIELD_PX.forEach((row, ry) =>
      [...row].forEach((c, rx) => {
        if (c === ".") return;
        g.fillStyle = c === "o" ? "#0c0703" : color;
        g.fillRect(x - 7 + rx * 2, y + ry * 2, 2, 2);
      }),
    );
  }
  g.restore();
}

/** An arrow from (x0, y0) to (x1, y1), bending up by `bend`. */
export function arrow(g, x0, y0, x1, y1, { color = "#9fd0ff", bend = 0, dash = [], width = 3, head = 9, alpha = 1, outline = true } = {}) {
  const mx = (x0 + x1) / 2;
  const my = (y0 + y1) / 2 - bend;
  g.save();
  g.globalAlpha = alpha;
  g.lineCap = "round";
  const path = () => {
    g.beginPath();
    g.moveTo(x0, y0);
    g.quadraticCurveTo(mx, my, x1, y1);
  };
  if (outline) {
    g.setLineDash(dash);
    g.strokeStyle = "rgba(12,7,3,0.85)";
    g.lineWidth = width + 3;
    path();
    g.stroke();
  }
  g.setLineDash(dash);
  g.strokeStyle = color;
  g.lineWidth = width;
  path();
  g.stroke();
  g.setLineDash([]);
  const tx = x1 - mx;
  const ty = y1 - my;
  const l = Math.hypot(tx, ty) || 1;
  const ux = tx / l;
  const uy = ty / l;
  const hx = x1 - ux * head;
  const hy = y1 - uy * head;
  g.beginPath();
  g.moveTo(x1 + ux * 2, y1 + uy * 2);
  g.lineTo(hx - uy * head * 0.6, hy + ux * head * 0.6);
  g.lineTo(hx + uy * head * 0.6, hy - ux * head * 0.6);
  g.closePath();
  if (outline) {
    g.lineWidth = 3;
    g.strokeStyle = "rgba(12,7,3,0.9)";
    g.stroke();
  }
  g.fillStyle = color;
  g.fill();
  g.restore();
}

/** The point at `u` (0..1) of the arc from (x0, y0) to (x1, y1) bending up by `bend`, and its
 *  direction. */
export function arcAt(x0, y0, x1, y1, bend, u) {
  const mx = (x0 + x1) / 2;
  const my = (y0 + y1) / 2 - bend;
  const x = (1 - u) * (1 - u) * x0 + 2 * (1 - u) * u * mx + u * u * x1;
  const y = (1 - u) * (1 - u) * y0 + 2 * (1 - u) * u * my + u * u * y1;
  const dx = 2 * (1 - u) * (mx - x0) + 2 * u * (x1 - mx);
  const dy = 2 * (1 - u) * (my - y0) + 2 * u * (y1 - my);
  return { x, y, ang: Math.atan2(dy, dx) };
}

function drawProj(g, { x, y, ang, kind }) {
  g.save();
  g.translate(Math.round(x), Math.round(y));
  if (kind === "bolt") {
    g.fillStyle = "#0c0703";
    g.fillRect(-5, -5, 10, 10);
    g.fillStyle = "#a276da";
    g.fillRect(-4, -4, 8, 8);
    g.fillStyle = "#ffe78c";
    g.fillRect(-2, -2, 4, 4);
  } else {
    g.rotate(ang);
    g.fillStyle = "#0c0703";
    g.fillRect(-7, -2, 14, 4);
    g.fillStyle = "#e9dbb3";
    g.fillRect(-6, -1, 10, 2);
    g.fillStyle = "#fffaec";
    g.fillRect(4, -1, 2, 2);
  }
  g.restore();
}

/** An impact spark in three sizes: a glance, a hit, a heavy hit (gold). */
function drawSpark(g, s) {
  const r = s.tier === 2 ? 12 : s.tier === 1 ? 8 : 4;
  const a = Math.max(0, 1 - s.t);
  const k = Math.round(r * (0.6 + 0.6 * s.t));
  const X = Math.round(s.x);
  const Y = Math.round(s.y);
  g.save();
  g.globalAlpha = a;
  g.fillStyle = "#0c0703";
  g.fillRect(X - k - 1, Y - 2, 2 * k + 2, 4);
  g.fillRect(X - 2, Y - k - 1, 4, 2 * k + 2);
  g.fillStyle = s.tier === 2 ? "#ffe78c" : "#fffaec";
  g.fillRect(X - k, Y - 1, 2 * k, 2);
  g.fillRect(X - 1, Y - k, 2, 2 * k);
  if (s.tier) {
    const d = Math.round(k * 0.6);
    for (const [ox, oy] of [[-d, -d], [d - 2, -d], [-d, d - 2], [d - 2, d - 2]]) g.fillRect(X + ox, Y + oy, 2, 2);
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
  skull: [".ooooo.", "o#####o", "o#o#o#o", "o#####o", ".o#o#o.", "..ooo.."],
  shield: SHIELD_PX,
  die: ["oooooo", "o####o", "o#o##o", "o####o", "o##o#o", "oooooo"],
  play: ["#...", "##..", "###.", "####", "###.", "##..", "#..."],
  pause: ["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"],
  back: ["...#", "..##", ".###", "####", ".###", "..##", "...#"],
  skip: ["#..#..", "##.##.", "######", "##.##.", "#..#.."],
};

/** A head-and-shoulders crop of fighter `f`'s sprite (a portrait chip at 1x). */
export function headChip(f, sides, { w = 32, h = 30, dead = false, flip = false } = {}) {
  const key = figOf(f, sides).key;
  const s = SPRITES[key];
  const el = document.createElement("span");
  el.className = `bt-head${dead ? " dead" : ""}`;
  const [x0, y0, x1] = s?.box ?? [24, 8, 72];
  const cx = Math.round((x0 + x1) / 2);
  el.style.cssText = `width:${w}px;height:${h}px;background-image:url(${spriteUrl(key)});background-position:${-(cx - Math.round(w / 2))}px ${-(y0 + 2)}px;${flip ? "transform:scaleX(-1);" : ""}`;
  el.setAttribute("aria-hidden", "true");
  return el;
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

/** A health colour, as the map's bars. */
export const hpColor = (p) => (p > 0.6 ? "#45e05c" : p > 0.3 ? "#f2c230" : "#e04030");

/** Appends `children` to `el` as dom.js `h` does: arrays flattened, null, undefined and false
 *  skipped, the rest as text. */
export function add(el, ...children) {
  for (const c of children.flat(Infinity)) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

/** Places `node` at (x, y) of the frame in `parent` (absolutely). */
export function at(parent, x, y, node) {
  node.style.position = "absolute";
  node.style.left = `${Math.round(x)}px`;
  node.style.top = `${Math.round(y)}px`;
  parent.append(node);
  return node;
}

/** A button of the battle screens: its label as plain text (no text hook), `on` lit. */
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

/** A health bar `w` px wide: `hp` of `max` (any unit), `pending` still to land (white), `tick` the
 *  withdrawal line (a fraction). */
export function hpBar(hp, max, { w = 48, pending = 0, tick = null } = {}) {
  const bar = document.createElement("span");
  bar.className = "bt-hp";
  bar.style.width = `${w}px`;
  const p = Math.max(0, hp) / max;
  const q = Math.max(0, hp - pending) / max;
  const v = document.createElement("i");
  v.style.width = `${Math.round(q * w)}px`;
  v.style.background = hpColor(q);
  bar.append(v);
  if (pending) {
    const pe = document.createElement("i");
    pe.className = "pend";
    pe.style.left = `${Math.round(q * w)}px`;
    pe.style.width = `${Math.round((p - q) * w)}px`;
    bar.append(pe);
  }
  if (tick !== null) {
    const t = document.createElement("b");
    t.className = "tick";
    t.style.left = `${Math.round(tick * w) - 1}px`;
    bar.append(t);
  }
  return bar;
}
