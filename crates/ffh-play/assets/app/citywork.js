// The city screen's work area on the map (city.js `overlay`, map.js `cityOverlay`), drawn the same
// way by every renderer that can place a plot on the screen (iso.js, r3d/hud.js): one citizen
// token on each plot a citizen works, so the plots worked read at a glance and add up with the
// specialists in the screen's panel. Each plot's rim says what a click does: gilt for worked (and
// the city's own plot), a dashed gilt rim for a plot free to work, blue and dashed for another
// city's, none on a plot that is not the player's (veiled). Over each plot its yields as icons,
// one per point (Civ4's), on a dark plate: full on the worked plots, faded on the others.

import { me } from "./state.js";
import { SPRITES } from "./iso/sprites.js";

const YIELD_KEYS = ["ui_food_s", "ui_production_s", "ui_commerce_s"];
/** A plot's yield drawn as single icons up to this many points; past it, one icon and the number. */
const PIPS_MAX = 5;

const images = new Map();
/** The interface sprite `key` (iso/sprites.js) as an image, or null while it loads; `onload` is
 *  called once it has (the renderer draws again). */
function image(key, onload) {
  if (!Object.hasOwn(SPRITES, key)) return null;
  let e = images.get(key);
  if (!e) {
    const im = new Image();
    e = { im, ok: false, waiting: [] };
    im.onload = () => {
      e.ok = true;
      for (const f of e.waiting.splice(0)) f();
    };
    im.src = new URL(`./iso/${key}.png`, import.meta.url).href;
    images.set(key, e);
  }
  if (!e.ok && onload && !e.waiting.includes(onload)) e.waiting.push(onload);
  return e.ok ? e.im : null;
}

/** What plot `p` of the city screen's city is to the player: "home" (the city's own plot),
 *  "hidden", "worked", "other" (another city works it), "free" (a click works it) or "closed". */
export function plotState(p, mine = me()?.player) {
  if (p.i === 0) return "home";
  if (!p.revealed) return "hidden";
  if (p.worked) return "worked";
  if (p.worked_by !== null && p.worked_by !== undefined) return "other";
  if (p.owner === mine && p.can_work) return "free";
  return "closed";
}

const RIM = {
  home: { fill: "rgba(255,218,120,0.14)", stroke: "#f0cc66", width: 2.5, dash: [] },
  worked: { fill: "rgba(255,218,120,0.20)", stroke: "#f0cc66", width: 2.5, dash: [] },
  free: { fill: null, stroke: "rgba(240,204,102,0.75)", width: 1.5, dash: [7, 5] },
  other: { fill: "rgba(60,110,170,0.24)", stroke: "#8cbcf0", width: 2, dash: [3, 4] },
  closed: { fill: "rgba(6,4,2,0.5)", stroke: null, width: 0, dash: [] },
  hidden: { fill: "rgba(3,10,32,0.7)", stroke: null, width: 0, dash: [] },
};
const PIP_ALPHA = { home: 1, worked: 1, free: 0.72, other: 0.6, closed: 0.45 };

/** Draws city screen overlay `city` ({ plots, ... }) on 2D context `g` (CSS px). `geo`:
 *  `outline(p)` the plot's outline as [[x, y], ...] or null off screen, `anchor(p)` its centre
 *  {x, y} on the ground, `ts` the plot's size on screen (CSS px). `hover` is the plot pointed at
 *  ({x, y}) or null; `redraw` is called when an icon this needed has loaded. */
export function drawCityWork(g, city, geo, { hover = null, redraw = null } = {}) {
  const mine = me()?.player;
  const shapes = [];
  for (const p of city.plots) {
    const pts = geo.outline(p);
    if (pts) shapes.push({ p, pts, state: plotState(p, mine), hover: !!hover && hover.x === p.x && hover.y === p.y });
  }
  g.save();
  g.lineJoin = "round";
  // The rims: fills first, then the strokes (a rim is not cut by its neighbour's fill).
  for (const s of shapes) {
    const r = RIM[s.state];
    if (!r.fill) continue;
    path(g, s.pts);
    g.fillStyle = r.fill;
    g.fill();
  }
  for (const s of shapes) {
    const r = RIM[s.state];
    if (!r.stroke) continue;
    path(g, s.pts);
    if (s.state === "worked" || s.state === "home") {
      g.setLineDash([]);
      g.strokeStyle = "rgba(12,7,3,0.85)";
      g.lineWidth = r.width + 2.5;
      g.stroke();
    }
    g.setLineDash(r.dash);
    g.strokeStyle = r.stroke;
    g.lineWidth = r.width;
    g.stroke();
  }
  g.setLineDash([]);
  for (const s of shapes) {
    if (!s.hover || s.state === "hidden") continue;
    path(g, s.pts);
    g.fillStyle = "rgba(255,255,255,0.10)";
    g.fill();
    g.strokeStyle = "#fff4c8";
    g.lineWidth = 2.5;
    g.stroke();
  }

  // The yields, then the citizens over them.
  const ts = geo.ts;
  const isz = ts >= 150 ? 24 : 16;
  const icons = YIELD_KEYS.map((k) => image(k, redraw));
  g.imageSmoothingEnabled = false;
  if (icons.every(Boolean)) {
    for (const s of shapes) {
      if (s.state === "hidden" || !s.p.yields) continue;
      const a = geo.anchor(s.p);
      if (a) drawPips(g, s.p.yields, icons, a.x, a.y + isz * 0.1, isz, s.hover ? 1 : PIP_ALPHA[s.state]);
    }
  }
  const person = image("ui_population_s", redraw);
  if (person) {
    const tsz = Math.round(isz * 1.75);
    for (const s of shapes) {
      const ghost = s.hover && s.state === "free";
      if (s.state !== "worked" && s.state !== "other" && !ghost) continue;
      const a = geo.anchor(s.p);
      if (!a) continue;
      g.globalAlpha = ghost ? 0.5 : s.state === "other" ? 0.85 : 1;
      drawToken(g, person, a.x, a.y - tsz * 0.62, tsz, s.state === "other" ? "#8cbcf0" : "#f0cc66");
    }
    g.globalAlpha = 1;
  }
  g.restore();
}

function path(g, pts) {
  g.beginPath();
  g.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) g.lineTo(pts[i][0], pts[i][1]);
  g.closePath();
}

/** A plot's yields centred at (cx, cy): icons one per point (PIPS_MAX at most, then the icon and
 *  the number), overlapping, on a dark plate. */
function drawPips(g, yields, icons, cx, cy, isz, alpha) {
  const step = Math.round(isz * 0.68);
  const groups = [];
  yields.forEach((v, j) => {
    if (v > 0) groups.push({ j, n: v > PIPS_MAX ? 1 : v, label: v > PIPS_MAX ? String(v) : null });
  });
  if (!groups.length) return;
  g.font = `700 ${Math.round(isz * 0.75)}px system-ui, sans-serif`;
  let w = 0;
  for (const gr of groups) {
    gr.w = isz + (gr.n - 1) * step + (gr.label ? g.measureText(gr.label).width + 2 : 0);
    w += gr.w;
  }
  const gap = Math.round(isz * 0.3);
  w += gap * (groups.length - 1);
  const pad = 3;
  const x0 = Math.round(cx - w / 2);
  const y0 = Math.round(cy - isz / 2);
  g.globalAlpha = alpha;
  g.fillStyle = "rgba(10,7,4,0.66)";
  roundRect(g, x0 - pad - 1, y0 - pad + 1, w + 2 * pad + 2, isz + 2 * pad - 2, (isz + 2 * pad) / 2);
  g.fill();
  let x = x0;
  g.textAlign = "left";
  g.textBaseline = "middle";
  for (const gr of groups) {
    for (let k = 0; k < gr.n; k++) g.drawImage(icons[gr.j], x + k * step, y0, isz, isz);
    if (gr.label) {
      g.fillStyle = "#fff4d6";
      g.fillText(gr.label, x + isz + 1, y0 + isz / 2 + 1);
    }
    x += gr.w + gap;
  }
  g.globalAlpha = 1;
}

/** A citizen token: the interface's figure on a dark disc in a `rim` ring, centred at (cx, cy). */
function drawToken(g, person, cx, cy, size, rim) {
  const r = size / 2;
  g.beginPath();
  g.arc(cx, cy, r, 0, Math.PI * 2);
  g.fillStyle = "rgba(20,14,8,0.92)";
  g.fill();
  g.lineWidth = Math.max(2, size * 0.08);
  g.strokeStyle = rim;
  g.stroke();
  g.beginPath();
  g.arc(cx, cy, r + g.lineWidth * 0.5 + 1, 0, Math.PI * 2);
  g.lineWidth = 1.5;
  g.strokeStyle = "rgba(10,6,3,0.9)";
  g.stroke();
  const s = Math.round(size * 0.72);
  g.drawImage(person, Math.round(cx - s / 2), Math.round(cy - s / 2), s, s);
}

function roundRect(g, x, y, w, h, r) {
  r = Math.min(r, w / 2, h / 2);
  g.beginPath();
  g.moveTo(x + r, y);
  g.arcTo(x + w, y, x + w, y + h, r);
  g.arcTo(x + w, y + h, x, y + h, r);
  g.arcTo(x, y + h, x, y, r);
  g.arcTo(x, y, x + w, y, r);
  g.closePath();
}
