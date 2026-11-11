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
