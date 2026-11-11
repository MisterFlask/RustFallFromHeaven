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
// Later phases add their parts here, each a pure helper the renderers call the same way: the named
// seas (a border, a name, a badge with the state), the lanes (drawn as supply lines are), the
// fleets. None of them is drawn yet.
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
