// The Sea layer (this game's own rules, docs/NAVAL.md section 10): what the renderers draw over
// the ground while the map's layer is "sea", read from the view. Phase 1: the water out of your
// range darkened, the range's edge drawn, and a wear pip on your worn warships. The layer turns on
// by itself while a ship or a unit in boats is selected (`autoSeaLayer`) and is also a toggle with
// the other layers.
//
// Data: the map's `sea` array (one value a plot, revealed plots only: -1 land or no range rule,
// 0 water out of range, else the steps from your nearest port), `me.sea_range` (your team's
// range, null without a city), each own unit's `boats` and `wear`, each city's `sea_port`.
//
// Phase 2 (named seas): each sea you have revealed a plot of has its border as a dotted line
// (`seaBorders`), its water tinted by its state (`drawSeaTint`: pattern and lightness, not hue
// alone), its name on the water and a badge with the state and the two strengths
// (`drawSeaLabel`, whose box a click opens the sea's panel by), and your supply ships' lanes are
// drawn as supply lines are, breaking with a mark where they enter a closed sea (`laneParts`,
// `drawLaneCut`). A coastal city's banner carries a harbour mark when blockaded, on every layer
// (`harbourMark`, `drawHarbourMark`). Data: the map's `sea_id` array (revealed water only), the
// view's `seas` (`{id, name, x, y, state, badge, ...}`) and `sea_lanes`, each city's `harbour`.
//
// Later phases add the fleets here.
//
// The renderers project the points themselves (each has its own camera) and call the drawing
// helpers here on their 2D canvas. Pure apart from the drawing: no DOM, no state of its own.

import { plotIndex } from "./tiles.js";

/** The layer's colours: `dark` over water out of range, `edge` the range's edge over a dark
 *  `casing`, the wear pip's fill and ring. */
export const SEA_STYLE = {
  dark: "rgba(2,5,14,0.6)",
  hatch: "rgba(150,190,235,0.16)",
  edge: "#8fdcff",
  casing: "rgba(4,10,24,0.7)",
  wear: "#e0a030",
  wearHeavy: "#e0503a",
  wearRing: "rgba(16,10,4,0.9)",
  wearHalo: "#fff1d0",
};

/** The cardinal directions `[dx, dy]` (y north) a plot shares a side with. */
export const SIDES = [[0, 1], [1, 0], [0, -1], [-1, 0]];

/** The sea value of plot index `i` (null when the map has none: an old reply, a fixture). */
export function seaValue(map, i) {
  const v = map?.sea?.[i];
  return v === undefined || v === null ? null : v;
}

/** The state of plot index `i` on the Sea layer: "in" (water in your range), "out" (water out of
 *  it), or null (land, unrevealed, or no range rule). */
export function seaState(map, i) {
  const v = seaValue(map, i);
  if (v === null || v < 0) return null;
  return v === 0 ? "out" : "in";
}

/** The sides of plot (x, y) where your range ends: for water in range, its cardinal neighbours
 *  `[dx, dy]` (y north) that are water out of range. Empty for any other plot. */
export function rangeEdges(map, x, y) {
  const i = plotIndex(map, x, y);
  if (i < 0 || seaState(map, i) !== "in") return [];
  const out = [];
  for (const [dx, dy] of SIDES) {
    const n = plotIndex(map, x + dx, y + dy);
    if (n >= 0 && seaState(map, n) === "out") out.push([dx, dy]);
  }
  return out;
}

/** What the layer draws on plot (x, y): null for nothing, else `{dark, edges}` (`dark`: water out
 *  of range; `edges`: rangeEdges). */
export function seaPlot(map, x, y) {
  const i = plotIndex(map, x, y);
  if (i < 0) return null;
  const st = seaState(map, i);
  if (st === "out") return { dark: true, edges: [] };
  if (st !== "in") return null;
  const edges = rangeEdges(map, x, y);
  return edges.length ? { dark: false, edges } : null;
}

/** Whether a selection of `units` (the view's units) wants the Sea layer: a ship (`isShip(unit)`)
 *  or a unit in boats among them. */
export function wantsSea(units, isShip) {
  return (units ?? []).some((u) => u && (u.boats || isShip(u)));
}

/**
 * The Sea layer's automatic switch: `layer` the map's layer now, `auto` the switch's memory (null,
 * or `{prev}`: the layer to go back to, while the Sea layer is on by itself), `wants` whether the
 * selection wants the layer (wantsSea). Returns `{layer, auto}`: the Sea layer turns on with the
 * selection and goes back to the previous layer when the selection no longer wants it; a layer the
 * player picked meanwhile (auto null) stays.
 */
export function autoSeaLayer(layer, auto, wants) {
  if (wants) return layer === "sea" ? { layer, auto } : { layer: "sea", auto: { prev: layer } };
  if (auto && layer === "sea") return { layer: auto.prev ?? "terrain", auto: null };
  return { layer, auto: null };
}

/** The wear pip of a stack: the highest `wear` of your units in it (percent of strength), or 0. */
export function stackWear(units, mine) {
  let w = 0;
  for (const u of units ?? []) if (u.owner === mine && u.wear > w) w = u.wear;
  return w;
}

/** The port line of a city's tooltip on the Sea layer: `Range: 12 plots.` for a port of yours
 *  (`city.sea_port`) while your team has a range; else null. */
export function portLine(city, range) {
  if (!city?.sea_port || range === null || range === undefined) return null;
  return `Range: ${range} plot${range === 1 ? "" : "s"}.`;
}

/** Whether a sea line of the path preview or the unit panel reads as a loss (drawn amber): out of
 *  range, an interception, wear. */
export function seaLineWarns(t) {
  return t.startsWith("Out of range") || t.includes("will be intercepted") || (t.startsWith("Wear:") && !t.startsWith("Wear: none"));
}

/** The two canvas points of the side of a plot toward a neighbour: `corners` the plot's corners
 *  on the canvas (`[{x, y}]`, in order round the plot), `toward` the neighbour's centre: the two
 *  corners nearest it (for any projection: square, diamond or a 3D camera's quad). */
export function sideToward(corners, toward) {
  const d = corners.map((p, k) => [Math.hypot(p.x - toward.x, p.y - toward.y), k]).sort((a, b) => a[0] - b[0]);
  return [corners[d[0][1]], corners[d[1][1]]];
}

/** Fills the polygon `pts` (`[{x, y}]`) with the darkening on 2D context `g`, and a faint hatch
 *  (two strokes across it, parallel to its first-to-third diagonal's normal) so that out of range
 *  reads by pattern as well as by lightness, over the darkest ocean too. */
export function drawDark(g, pts) {
  if (!pts || pts.length < 3) return;
  g.beginPath();
  pts.forEach((p, k) => (k ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)));
  g.closePath();
  g.fillStyle = SEA_STYLE.dark;
  g.fill();
  if (pts.length !== 4) return;
  // Strokes from side 0-1 to side 3-2, at a third and two thirds: a hatch that tiles across plots.
  const lerp = (a, b, t) => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
  const [a, b, c, d] = pts;
  g.save();
  g.clip();
  g.beginPath();
  for (const t of [0.25, 0.75]) {
    const p = lerp(a, b, t);
    const q = lerp(d, c, t - 0.5);
    const p2 = lerp(a, b, t + 0.5);
    const q2 = lerp(d, c, t);
    g.moveTo(p.x, p.y);
    g.lineTo(q.x, q.y);
    g.moveTo(p2.x, p2.y);
    g.lineTo(q2.x, q2.y);
  }
  g.strokeStyle = SEA_STYLE.hatch;
  g.lineWidth = Math.max(1, Math.hypot(b.x - a.x, b.y - a.y) / 22);
  g.stroke();
  g.restore();
}

/** Draws the range's edges `segs` (`[[{x, y}, {x, y}], ...]`) on 2D context `g`, `width` px: a
 *  light line over a dark casing. */
export function drawEdges(g, segs, width) {
  if (!segs.length) return;
  g.save();
  g.lineCap = "round";
  g.beginPath();
  for (const [a, b] of segs) {
    g.moveTo(a.x, a.y);
    g.lineTo(b.x, b.y);
  }
  g.strokeStyle = SEA_STYLE.casing;
  g.lineWidth = width + 2;
  g.stroke();
  g.strokeStyle = SEA_STYLE.edge;
  g.lineWidth = width;
  g.stroke();
  g.restore();
}

/** Draws a wear pip of `wear` percent centred at (x, y), radius `r`, on 2D context `g`: amber,
 *  red from half the maximum (`max`). */
export function drawWearPip(g, x, y, r, wear, max = 50) {
  if (!(wear > 0)) return;
  g.save();
  g.beginPath();
  g.arc(x, y, r, 0, Math.PI * 2);
  g.fillStyle = wear * 2 >= max ? SEA_STYLE.wearHeavy : SEA_STYLE.wear;
  g.fill();
  // A dark ring inside a light one: it reads on a flag, a figure or the water.
  g.lineWidth = Math.max(2, r / 2.5);
  g.strokeStyle = SEA_STYLE.wearHalo;
  g.stroke();
  g.lineWidth = Math.max(1, r / 5);
  g.strokeStyle = SEA_STYLE.wearRing;
  g.stroke();
  // A notch: the share of the maximum worn.
  g.beginPath();
  g.moveTo(x, y);
  g.arc(x, y, r * 0.6, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * Math.min(1, wear / max));
  g.closePath();
  g.fillStyle = SEA_STYLE.wearRing;
  g.fill();
  g.restore();
}

// ---------------------------------------------------------------------------------------------
// Phase 2: named seas

/** Each state's look: the water's `tint` (lightness) and `stripes` (pattern: none, the enemy's
 *  diagonal, the contested pair), the badge's `fill`, `ring` and `ink`. The states differ by
 *  pattern and lightness, not hue alone: held by you lightest and plain, held by an enemy darker
 *  with wide stripes, contested with alternating light and dark stripes, open plain, patrolled by
 *  a team at peace with you with sparse dots, out of range left to the range's own darkening. */
export const SEA_STATES = {
  held_you: { tint: "rgba(245,230,184,0.20)", stripes: null, fill: "#f5e6b8", ring: "#14213a", ink: "#14213a", label: "Held by you" },
  held: { tint: "rgba(6,10,22,0.30)", stripes: ["rgba(224,138,60,0.50)"], fill: "#e08a3c", ring: "#14213a", ink: "#14213a", label: "Held by an enemy" },
  contested: { tint: "rgba(10,16,30,0.10)", stripes: ["rgba(245,230,184,0.42)", "rgba(224,138,60,0.50)"], fill: "#14213a", ring: "#ffffff", ink: "#ffffff", label: "Contested" },
  open: { tint: null, stripes: null, fill: "#14213a", ring: "#8fb3c7", ink: "#d5e6ef", label: "Open" },
  patrolled: { tint: null, stripes: null, dots: "rgba(200,220,255,0.45)", fill: "#14213a", ring: "#b9a6e0", ink: "#e6defa", label: "Patrolled" },
  out: { tint: null, stripes: null, fill: "#0f1a2e", ring: "#aab6c8", ink: "#d5dceb", label: "Out of range" },
};

/** The sea id of plot index `i` on the Sea layer (-1 for none: land, unrevealed, no sea, or a map
 *  without the array). */
export function seaIdAt(map, i) {
  const v = map?.sea_id?.[i];
  return v === undefined || v === null ? -1 : v;
}

/** The view's seas by id (`view.seas`). */
export function seasById(view) {
  const out = new Map();
  for (const s of view?.seas ?? []) out.set(s.id, s);
  return out;
}

/** The sides of plot (x, y) on a border between two seas: `[dx, dy]` (y north) toward a
 *  neighbour of another sea, east and north only (each border once). Empty off the seas. */
export function seaBorders(map, x, y) {
  const i = plotIndex(map, x, y);
  const s = i < 0 ? -1 : seaIdAt(map, i);
  if (s < 0) return [];
  const out = [];
  for (const [dx, dy] of [[1, 0], [0, 1]]) {
    const n = plotIndex(map, x + dx, y + dy);
    const t = n < 0 ? -1 : seaIdAt(map, n);
    if (t >= 0 && t !== s) out.push([dx, dy]);
  }
  return out;
}

/** What the named-seas part of the layer draws on plot (x, y): null for nothing, else `{state,
 *  borders}` (`state` the plot's sea's state key, null when it has none to tint; `borders`
 *  seaBorders). `seas`: seasById. */
export function seaPlotNamed(map, seas, x, y) {
  const i = plotIndex(map, x, y);
  if (i < 0) return null;
  const id = seaIdAt(map, i);
  if (id < 0) return null;
  const st = seas.get(id)?.state ?? null;
  const borders = seaBorders(map, x, y);
  const look = st ? SEA_STATES[st] : null;
  const tinted = look && (look.tint || look.stripes || look.dots) ? st : null;
  return tinted || borders.length ? { state: tinted, borders } : null;
}

/** Tints the plot polygon `pts` (`[{x, y}]`, four corners in order) by sea state `state` on 2D
 *  context `g`: the state's fill, then its stripes (parallel strokes across the plot that meet
 *  their neighbours' across the sides, so they read as one pattern over the sea) or dots. */
export function drawSeaTint(g, pts, state) {
  const look = SEA_STATES[state];
  if (!look || !pts || pts.length !== 4) return;
  g.save();
  g.beginPath();
  pts.forEach((p, k) => (k ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)));
  g.closePath();
  if (look.tint) {
    g.fillStyle = look.tint;
    g.fill();
  }
  const [a, b, c, d] = pts;
  const lerp = (p, q, t) => ({ x: p.x + (q.x - p.x) * t, y: p.y + (q.y - p.y) * t });
  const side = Math.hypot(b.x - a.x, b.y - a.y);
  if (look.stripes) {
    g.clip();
    // Stripes from side a-b to side d-c, shifted by half a plot: diagonals that continue into the
    // next plot. Two per plot for one colour; four, alternating, for two.
    const n = look.stripes.length === 1 ? 2 : 4;
    g.lineWidth = Math.max(1.5, side / (n === 2 ? 7 : 10));
    for (let k = 0; k < n; k++) {
      g.beginPath();
      const t = k / n;
      for (const off of [0, 1]) {
        const p = lerp(a, b, t + off - 0.5 + 0.5 / n);
        const q = lerp(d, c, t + off - 1 + 0.5 / n);
        g.moveTo(p.x, p.y);
        g.lineTo(q.x, q.y);
      }
      g.strokeStyle = look.stripes[k % look.stripes.length];
      g.stroke();
    }
  } else if (look.dots) {
    const m = lerp(lerp(a, c, 0.5), lerp(b, d, 0.5), 0.5);
    g.beginPath();
    g.arc(m.x, m.y, Math.max(1, side / 18), 0, Math.PI * 2);
    g.fillStyle = look.dots;
    g.fill();
  }
  g.restore();
}

/** Draws the borders between seas `segs` (`[[{x, y}, {x, y}], ...]`) on 2D context `g`, `width`
 *  px: dark dots with a light core, so the dotted line reads over any water. */
export function drawSeaBorders(g, segs, width) {
  if (!segs.length) return;
  g.save();
  g.lineCap = "round";
  g.beginPath();
  for (const [a, b] of segs) {
    g.moveTo(a.x, a.y);
    g.lineTo(b.x, b.y);
  }
  g.setLineDash([0.1, width * 3.2]);
  g.strokeStyle = "rgba(8,14,28,0.85)";
  g.lineWidth = width + 1.5;
  g.stroke();
  g.strokeStyle = "rgba(225,236,250,0.9)";
  g.lineWidth = Math.max(1, width - 1);
  g.stroke();
  g.restore();
}

/** The font sizes of a sea's name and badge for plot size `ts` (CSS px): legible at every zoom
 *  (the name at least 11 px, the badge at least 10 px). */
export function seaLabelSizes(ts) {
  return { name: Math.round(Math.max(11, Math.min(22, ts * 0.42))), badge: Math.round(Math.max(10, Math.min(12, ts * 0.24))) };
}

/**
 * Draws sea `sea`'s (a `view.seas` entry) name and badge centred at (x, y) on 2D context `g`, the
 * name over the badge, in sizes `sizes` (seaLabelSizes). Returns the box a click opens the sea's
 * panel by: `{id, x0, y0, x1, y1}`.
 */
export function drawSeaLabel(g, sea, x, y, sizes) {
  const look = SEA_STATES[sea.state] ?? SEA_STATES.open;
  g.save();
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.font = `italic 600 ${sizes.name}px "Iowan Old Style", "Palatino Linotype", Palatino, "URW Palladio L", Georgia, serif`;
  const nw = g.measureText(sea.name).width;
  const ny = y - sizes.badge * 0.9;
  g.lineJoin = "round";
  g.lineWidth = Math.max(3, sizes.name / 4);
  g.strokeStyle = "rgba(6,10,22,0.85)";
  g.strokeText(sea.name, x, ny);
  g.fillStyle = sea.state === "out" ? "#c8d2e0" : "#ffffff";
  g.fillText(sea.name, x, ny);
  g.font = `700 ${sizes.badge}px system-ui, "DejaVu Sans", "Segoe UI", sans-serif`;
  const bw = g.measureText(sea.badge).width + sizes.badge * 1.4;
  const bh = sizes.badge + 8;
  const by = ny + sizes.name * 0.62;
  const r = bh / 2;
  const x0 = x - bw / 2;
  g.beginPath();
  g.moveTo(x0 + r, by);
  g.arcTo(x0 + bw, by, x0 + bw, by + bh, r);
  g.arcTo(x0 + bw, by + bh, x0, by + bh, r);
  g.arcTo(x0, by + bh, x0, by, r);
  g.arcTo(x0, by, x0 + bw, by, r);
  g.closePath();
  g.fillStyle = look.fill;
  g.fill();
  g.lineWidth = sea.state === "contested" ? 2 : 1.25;
  g.strokeStyle = look.ring;
  g.stroke();
  g.fillStyle = look.ink;
  g.fillText(sea.badge, x, by + bh / 2 + 0.5);
  g.restore();
  const half = Math.max(nw, bw) / 2 + 4;
  return { id: sea.id, x0: x - half, y0: ny - sizes.name * 0.7, x1: x + half, y1: by + bh + 2 };
}

/** The sea whose label box (drawSeaLabel's) holds point (px, py) of the canvas, or null. */
export function seaHitAt(hits, px, py) {
  for (let k = (hits?.length ?? 0) - 1; k >= 0; k--) {
    const b = hits[k];
    if (px >= b.x0 && px <= b.x1 && py >= b.y0 && py <= b.y1) return b.id;
  }
  return null;
}

/** A supply ship's lane (`view.sea_lanes` entry) in the parts the renderers draw: `open` (the
 *  path's points from the ship up to where it enters the closed sea, or all of it), `cut` (from
 *  there on to the port, null when open) and `at` (the point of the break, null when open). */
export function laneParts(lane) {
  const path = lane?.path ?? [];
  const at = lane?.cut ? Math.max(0, Math.min(path.length - 1, lane.cut.at ?? 0)) : -1;
  if (at < 0) return { open: path, cut: null, at: null };
  return { open: path.slice(0, at + 1), cut: path.slice(at), at: path[at] };
}

/** The lanes' colour: your own lines' (supply.js OWN_COLOR); the part beyond a cut is `cutColor`. */
export const LANE_STYLE = { cutColor: "#ffb07a", markFill: "#0f1a2e" };

/** Draws the cut part of a lane (`pts` `[{x, y}]` on the canvas) dotted, and the break's mark (a
 *  ring with a cross) at its first point, `r` its radius, on 2D context `g`. */
export function drawLaneCut(g, pts, r) {
  if (!pts?.length) return;
  g.save();
  g.lineCap = "round";
  if (pts.length > 1) {
    g.beginPath();
    pts.forEach((p, i) => (i ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)));
    g.setLineDash([0.1, Math.max(5, r * 0.9)]);
    g.strokeStyle = LANE_STYLE.cutColor;
    g.lineWidth = Math.max(2, r * 0.3);
    g.stroke();
    g.setLineDash([]);
  }
  const { x, y } = pts[0];
  g.beginPath();
  g.arc(x, y, r, 0, Math.PI * 2);
  g.fillStyle = LANE_STYLE.markFill;
  g.fill();
  g.lineWidth = Math.max(1.5, r / 5);
  g.strokeStyle = LANE_STYLE.cutColor;
  g.stroke();
  const k = r * 0.45;
  g.beginPath();
  g.moveTo(x - k, y - k);
  g.lineTo(x + k, y + k);
  g.moveTo(x + k, y - k);
  g.lineTo(x - k, y + k);
  g.lineWidth = Math.max(1.5, r / 4);
  g.stroke();
  g.restore();
}

/** A city's harbour mark (the view's `city.harbour`): null when its harbour is open or it is on
 *  no sea; else `{level, fill, rim, title}`: 1 blockaded, 2 blockaded and besieged. */
export function harbourMark(city) {
  const hb = city?.harbour;
  if (!hb || hb.state !== "blockaded") return null;
  if (hb.besieged) return { level: 2, fill: "#c0241c", rim: "#ffe58c", title: "Besieged and blockaded: no healing from the city, no repair" };
  return { level: 1, fill: "#1d3550", rim: "#ffb07a", title: "Blockaded: sea plots not worked, no sea trade" };
}

/** Draws city `city`'s harbour mark (harbourMark), if any, as a disc of radius `r` centred at
 *  (`cx`, `cy`) with an anchor in it; `u` the line unit. */
export function drawHarbourMark(g, city, cx, cy, r, u = 1) {
  const mk = harbourMark(city);
  if (!mk) return;
  g.save();
  g.beginPath();
  g.arc(cx, cy, r, 0, Math.PI * 2);
  g.fillStyle = mk.fill;
  g.fill();
  g.lineWidth = Math.max(1, u) * (mk.level === 2 ? 1.6 : 1.2);
  g.strokeStyle = mk.rim;
  g.stroke();
  // The anchor: a ring, a shank, a stock and the arms.
  const s = r * 0.62;
  g.strokeStyle = "#f5ecd8";
  g.lineWidth = Math.max(1, r / 5);
  g.lineCap = "round";
  g.beginPath();
  g.arc(cx, cy - s * 0.78, s * 0.2, 0, Math.PI * 2);
  g.moveTo(cx, cy - s * 0.58);
  g.lineTo(cx, cy + s * 0.85);
  g.moveTo(cx - s * 0.4, cy - s * 0.3);
  g.lineTo(cx + s * 0.4, cy - s * 0.3);
  g.moveTo(cx - s * 0.75, cy + s * 0.25);
  g.quadraticCurveTo(cx - s * 0.6, cy + s * 0.9, cx, cy + s * 0.85);
  g.quadraticCurveTo(cx + s * 0.6, cy + s * 0.9, cx + s * 0.75, cy + s * 0.25);
  g.stroke();
  g.restore();
}

// ---------------------------------------------------------------------------------------------
// The HD-2D map's look (r3d/sealayer.js draws it in the world; the flat and isometric maps keep the
// drawing above). Pure: the layout and the colours, from the map's arrays and the view's seas.

/**
 * The Sea layer's look on the HD-2D map, in one place: the names' strengths (alpha of the pale
 * lettering) by zoom with the layer on and off, the bands' width and strength, the fog.
 *
 * - `name.on`: with the Sea layer on, `far` at `farTs` px a plot and below, falling to `close` at
 *   `closeTs` and above (log-linear in between).
 * - `name.off`: with another layer, only at the farthest zooms: `far` at `farTs` and below, nothing
 *   from `untilTs` on.
 */
export const SEA_LOOK = {
  name: {
    color: "#efe4c6",
    on: { far: 0.62, close: 0.2, farTs: 40, closeTs: 168 },
    off: { far: 0.3, farTs: 28, untilTs: 50 },
  },
  band: { width: 0.5, alpha: 0.62, patrolled: 0.45, unknown: "#8a8478" },
  fog: { color: "#cfd8df", alpha: 0.8, lift: 0.16 },
  outline: "#fff6d8",
};

/** The names' alpha at plot size `ts` (px), the Sea layer on or off (SEA_LOOK.name). */
export function seaNameAlpha(ts, layerOn, look = SEA_LOOK.name) {
  if (!layerOn) {
    const o = look.off;
    if (ts >= o.untilTs) return 0;
    if (ts <= o.farTs) return o.far;
    return o.far * (1 - (ts - o.farTs) / (o.untilTs - o.farTs));
  }
  const o = look.on;
  if (ts <= o.farTs) return o.far;
  if (ts >= o.closeTs) return o.close;
  const t = Math.log(ts / o.farTs) / Math.log(o.closeTs / o.farTs);
  return o.far + (o.close - o.far) * t;
}

/** The revealed plots of sea `id` (the map's `sea_id`) as `[x, y]`, x unwrapped to the copy
 *  nearest `nearX` on a map that wraps. */
export function seaPlots(map, id, nearX = 0) {
  const out = [];
  const ids = map?.sea_id;
  if (!ids) return out;
  const w = map.w;
  const half = Math.floor(w / 2);
  for (let i = 0; i < ids.length; i++) {
    if (ids[i] !== id) continue;
    let x = i % w;
    const y = (i - x) / w;
    if (map.wrap_x) x = nearX + ((((x - nearX + half) % w) + w) % w) - half;
    out.push([x, y]);
  }
  return out;
}

/**
 * Where and how sea `sea` (a `view.seas` entry: `id`, `x`, `y`) has its name laid on the water,
 * from its revealed plots: `{x, y}` the name's centre (map point; x unwrapped near `sea.x`),
 * `angle` its direction (radians from map east, y north: along the plots' principal axis, never
 * upside down and at most `maxTilt` from level; level for a sea about as wide as long), `len` and
 * `across` its revealed extent along and across that axis (plots), `n` its revealed plots, `depth`
 * the centre's steps (8 ways) from the sea's edge, `h` the letters' height (plots, from its size,
 * clamped) and `plot` the centre's plot. Null for a sea with no revealed plot.
 *
 * The centre is the plot nearest the plots' centroid among the deepest (at least the deepest but
 * one): inside the water, in the middle of its widest part.
 */
export function seaLabelLayout(map, sea, { maxTilt = Math.PI / 4, minH = 0.34, maxH = 1.3 } = {}) {
  const pts = seaPlots(map, sea.id, sea.x);
  const n = pts.length;
  if (!n) return null;
  const key = (x, y) => `${x},${y}`;
  const inside = new Set(pts.map(([x, y]) => key(x, y)));
  // Steps from the edge (a plot with a neighbour, 8 ways, outside the sea's revealed plots).
  const depth = new Map();
  let queue = [];
  for (const [x, y] of pts) {
    let edge = false;
    for (let dy = -1; dy <= 1 && !edge; dy++) for (let dx = -1; dx <= 1; dx++) if ((dx || dy) && !inside.has(key(x + dx, y + dy))) edge = true;
    if (edge) {
      depth.set(key(x, y), 0);
      queue.push([x, y]);
    }
  }
  while (queue.length) {
    const next = [];
    for (const [x, y] of queue) {
      const d = depth.get(key(x, y));
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const k = key(x + dx, y + dy);
          if (inside.has(k) && !depth.has(k)) {
            depth.set(k, d + 1);
            next.push([x + dx, y + dy]);
          }
        }
      }
    }
    queue = next;
  }
  let cx = 0, cy = 0;
  for (const [x, y] of pts) {
    cx += x + 0.5;
    cy += y + 0.5;
  }
  cx /= n;
  cy /= n;
  let sxx = 0, syy = 0, sxy = 0;
  for (const [x, y] of pts) {
    const dx = x + 0.5 - cx, dy = y + 0.5 - cy;
    sxx += dx * dx;
    syy += dy * dy;
    sxy += dx * dy;
  }
  // The principal axis of the plots' covariance; level when the two spreads are close.
  const tr = sxx + syy;
  const det = sxx * syy - sxy * sxy;
  const disc = Math.sqrt(Math.max(0, (tr * tr) / 4 - det));
  const l1 = tr / 2 + disc, l2 = tr / 2 - disc;
  let angle = l1 > 1.5 * Math.max(l2, 1e-9) ? 0.5 * Math.atan2(2 * sxy, sxx - syy) : 0;
  // Never upside down (reading left to right on the screen, the camera looking north).
  if (angle > Math.PI / 2) angle -= Math.PI;
  if (angle <= -Math.PI / 2) angle += Math.PI;
  angle = Math.max(-maxTilt, Math.min(maxTilt, angle));
  const maxD = Math.max(...depth.values());
  let best = null;
  for (const [x, y] of pts) {
    const d = depth.get(key(x, y));
    if (d < maxD - 1) continue;
    const dist = (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 - d * 0.5;
    if (!best || dist < best.dist - 1e-9) best = { x, y, d, dist };
  }
  const ux = Math.cos(angle), uy = Math.sin(angle);
  let a0 = Infinity, a1 = -Infinity, b0 = Infinity, b1 = -Infinity;
  const ox = best.x + 0.5, oy = best.y + 0.5;
  for (const [x, y] of pts) {
    const dx = x + 0.5 - ox, dy = y + 0.5 - oy;
    const a = dx * ux + dy * uy, b = -dx * uy + dy * ux;
    a0 = Math.min(a0, a);
    a1 = Math.max(a1, a);
    b0 = Math.min(b0, b);
    b1 = Math.max(b1, b);
  }
  const h = Math.max(minH, Math.min(maxH, 0.26 * Math.sqrt(n)));
  return { x: ox, y: oy, angle, len: a1 - a0 + 1, across: b1 - b0 + 1, n, depth: best.d, h, plot: [best.x, best.y] };
}

/** The letters' height (plots) of a name whose lettering is `aspect` times as wide as tall, laid
 *  per `lay` (seaLabelLayout): `lay.h`, smaller where the name would run past the sea's revealed
 *  length (to 90% of it), never under `minH`. */
export function fitNameHeight(lay, aspect, minH = 0.3) {
  if (!(aspect > 0)) return lay.h;
  return Math.max(minH, Math.min(lay.h, (0.9 * lay.len) / aspect));
}

/** Whether map point (x, y) (x unwrapped) lies on a revealed plot of sea `id`. */
export function inSea(map, id, x, y) {
  const i = plotIndex(map, Math.floor(x), Math.floor(y));
  return i >= 0 && seaIdAt(map, i) === id;
}

/**
 * Where sea `sea`'s buoy stands (its banner), from its name's layout `lay` and the name's width
 * `width` (plots): just west of the name's first letter, else east of its last, else under its
 * middle, else in the name's centre plot; the first of these on a revealed plot of the sea.
 * `{x, y}` (map point).
 */
export function seaBuoySpot(map, sea, lay, width) {
  const ux = Math.cos(lay.angle), uy = Math.sin(lay.angle);
  const half = width / 2 + 0.3;
  const below = lay.h * 0.75 + 0.2;
  const cands = [
    [lay.x - ux * half, lay.y - uy * half - lay.h * 0.15],
    [lay.x + ux * half, lay.y + uy * half - lay.h * 0.15],
    [lay.x + uy * below, lay.y - ux * below],
  ];
  for (const [x, y] of cands) if (inSea(map, sea.id, x, y)) return { x, y };
  return { x: lay.plot[0] + 0.5, y: lay.plot[1] + 0.25 };
}

/** The colour of team `team` (its first player in `players`, the view's players, by
 *  `colorOf(playerId)`), or null. */
export function teamColorOf(players, team, colorOf) {
  if (team === null || team === undefined) return null;
  const p = [...(players ?? [])].find((q) => q.team === team);
  return p ? colorOf(p.id) : null;
}

/** The band codes the HD-2D sea pass reads (r3d/sealayer.js). */
export const BAND = { NONE: 0, HELD_YOU: 1, HELD: 2, CONTESTED: 3, PATROLLED: 4 };

/**
 * The band of sea `sea` (a `view.seas` entry) on the HD-2D map: `{code, a, b, strength}` (`a`, `b`
 * colours: the holder's; contested, yours then the other party's, alternating in dashes). Held by
 * you: your colour (`mine`). Held by an enemy, seen or inferred: the holder's (`partyColor`; an
 * unknown holder SEA_LOOK.band.unknown). Patrolled by a team at peace with you: its colour,
 * fainter. Open and out of range: none.
 */
export function seaBand(sea, mine, partyColor) {
  const other = partyColor ?? SEA_LOOK.band.unknown;
  switch (sea?.state) {
    case "held_you":
      return { code: BAND.HELD_YOU, a: mine, b: mine, strength: 1 };
    case "held":
      return { code: BAND.HELD, a: other, b: other, strength: 1 };
    case "contested":
      return { code: BAND.CONTESTED, a: mine, b: other, strength: 1 };
    case "patrolled":
      return { code: BAND.PATROLLED, a: other, b: other, strength: SEA_LOOK.band.patrolled };
    default:
      return { code: BAND.NONE, a: null, b: null, strength: 0 };
  }
}

/** "twice" for a hold of 200 percent (the define SEA_HOLD_PERCENT), else "N times" or "N% of" (as
 *  the server's panel words it). */
export function holdTimes(pct) {
  if (pct % 100 === 0) return pct === 200 ? "twice" : `${pct / 100} times`;
  return `${pct}% of`;
}

/** The tooltip of sea `sea` (a `view.seas` entry) on the HD-2D map, as lines: its name, the
 *  badge's text, then the rules (`holdPct` the define SEA_HOLD_PERCENT). */
export function seaTipLines(sea, holdPct = 200) {
  const lines = [sea.name, sea.badge];
  if (sea.state === "out") lines.push("No plot of it is in your range.");
  lines.push(`Held at ${holdTimes(holdPct)} the enemy's strength on patrol.`);
  lines.push(sea.inferred ? "Its ships on patrol are not all in sight." : "Only ships you see are counted.");
  return lines;
}

/**
 * The fog bank's density by plot (Uint8Array, 0..255, one value a plot): 0 on water in your range,
 * over water out of it rising with the steps (8 ways, through that water) from the nearest water in
 * range -- half at the first step, full from the second -- and on land and unrevealed plots the
 * highest of their neighbours' (so the bank does not thin toward a coast; the shader keeps it off
 * the land and the unknown). All 0 without the map's `sea` array or water out of range; with no
 * water in range (no city) all of it full.
 */
export function seaFogRamp(map) {
  const w = map.w, h = map.h;
  const out = new Uint8Array(w * h);
  const sea = map.sea;
  if (!sea) return out;
  const dist = new Int32Array(w * h).fill(-1);
  let queue = [];
  let anyOut = false;
  for (let i = 0; i < w * h; i++) {
    const v = sea[i];
    if (v > 0) {
      dist[i] = 0;
      queue.push(i);
    } else if (v === 0) anyOut = true;
  }
  if (!anyOut) return out;
  while (queue.length) {
    const next = [];
    for (const i of queue) {
      const x = i % w, y = (i - x) / w;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const j = plotIndex(map, x + dx, y + dy);
          if (j < 0 || dist[j] >= 0 || sea[j] !== 0) continue;
          dist[j] = dist[i] + 1;
          next.push(j);
        }
      }
    }
    queue = next;
  }
  for (let i = 0; i < w * h; i++) {
    if (sea[i] !== 0) continue;
    const d = dist[i] < 0 ? 2 : Math.min(2, dist[i]);
    out[i] = Math.round((d / 2) * 255);
  }
  const dil = out.slice();
  for (let i = 0; i < w * h; i++) {
    if (sea[i] === 0 || sea[i] > 0) continue;
    const x = i % w, y = (i - x) / w;
    let m = 0;
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const j = plotIndex(map, x + dx, y + dy);
        if (j >= 0 && sea[j] === 0) m = Math.max(m, out[j]);
      }
    }
    dil[i] = m;
  }
  return dil;
}

/**
 * A lane on the water (the HD-2D map's sea pass): the polyline `pts` (`[[x, y], ...]` plot
 * coordinates, unwrapped) through the plots' centres, its corners rounded (one Chaikin pass, the
 * ends kept), with each point's distance along it from the first (`s`, plots). `[{x, y, s}]`.
 */
export function laneCurve(pts) {
  const c = (pts ?? []).map(([x, y]) => [x + 0.5, y + 0.5]);
  if (c.length < 2) return c.map(([x, y]) => ({ x, y, s: 0 }));
  const sm = [c[0]];
  for (let k = 0; k + 1 < c.length; k++) {
    const [ax, ay] = c[k], [bx, by] = c[k + 1];
    if (k > 0) sm.push([ax * 0.75 + bx * 0.25, ay * 0.75 + by * 0.25]);
    if (k + 2 < c.length) sm.push([ax * 0.25 + bx * 0.75, ay * 0.25 + by * 0.75]);
  }
  sm.push(c[c.length - 1]);
  let s = 0;
  return sm.map(([x, y], k) => {
    if (k) s += Math.hypot(x - sm[k - 1][0], y - sm[k - 1][1]);
    return { x, y, s };
  });
}
