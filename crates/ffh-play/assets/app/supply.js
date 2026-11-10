// Supply lines on the map (this game's own rules, docs/SUPPLY.md section 5): what the renderers
// draw over the ground, read from the view. Your armies' lines (each army's `supply.line`, from the
// army's plot to its source) and the lines of other players' armies you see (the view's
// `supply_lines`, clipped by the server to the plots you have revealed), in their owner's colour;
// a slow flow of dashes along each line toward its army; and a marker on each of your armies whose
// line was cut (Hunger with the turns left before Starvation, or Starvation).
//
// The renderers project the points themselves (each has its own camera) and call the drawing
// helpers here on their 2D canvas. Pure apart from the drawing: no DOM, no state of its own.

import { S, me, isBarbarian } from "./state.js";
import { playerColor } from "./tiles.js";

/** The colour of your own lines: the vellum of the interface (other players': their colour). */
export const OWN_COLOR = "#f5e6b8";

/** The lines to draw: `[{owner, mine, color, segments: [[[x, y], ...], ...]}]`, each segment from
 *  the army's end toward the source's (only segments of two plots or more). */
export function supplyLines(view = S.view) {
  const out = [];
  const mine = me()?.player ?? null;
  for (const a of view?.armies ?? []) {
    const path = a.supply?.line?.path;
    if (path?.length > 1) out.push({ owner: mine, mine: true, color: OWN_COLOR, segments: [path] });
  }
  for (const l of view?.supply_lines ?? []) {
    const segments = (l.segments ?? []).filter((s) => s.length > 1);
    if (segments.length) out.push({ owner: l.owner, mine: false, color: playerColor(l.owner, isBarbarian(l.owner)), segments });
  }
  return out;
}

/** The markers of your armies whose line is cut: `[{army, x, y, state, text}]`, `state` "hunger"
 *  or "starvation"; `text` the marker's ("Hunger: 2": the turns before Starvation). */
export function supplyMarks(view = S.view) {
  const out = [];
  for (const a of view?.armies ?? []) {
    const s = a.supply;
    if (!s || a.x === null || a.x === undefined || (s.state !== "hunger" && s.state !== "starvation")) continue;
    out.push({ army: a.id, x: a.x, y: a.y, state: s.state, text: s.state === "hunger" ? `Hunger: ${s.turns_to_starvation}` : "Starvation" });
  }
  return out;
}

/** Points along a segment unwrapped across a wrapping map's seam: each column next to the one
 *  before (`x0` the first point's column as the caller places it). `[[x, y]] -> [[ux, y]]`. */
export function unwrapSegment(seg, x0, map) {
  const out = [];
  let ux = x0;
  let px = seg[0][0];
  for (let k = 0; k < seg.length; k++) {
    const [x, y] = seg[k];
    if (k) {
      let dx = x - px;
      if (map?.wrap_x) {
        if (dx > map.w / 2) dx -= map.w;
        if (dx < -map.w / 2) dx += map.w;
      }
      ux += dx;
    }
    px = x;
    out.push([ux, y]);
  }
  return out;
}

/** The dash offset at `now` (ms): the dashes flow from the source toward the army (the segment's
 *  first point) at `speed` px a second; 0 under reduced motion. */
export function flowOffset(now, speed, still = false) {
  return still ? 0 : ((now / 1000) * speed) % 1e6;
}

/**
 * Draws one segment of a line on 2D context `g`: `pts` `[{x, y}]` on the canvas (the army's end
 * first), `color`, `width` (px of the coloured stroke), the flow's `offset` (flowOffset). A dark
 * casing under a dashed stroke, so it reads over any ground.
 */
export function drawLine(g, pts, color, width, offset) {
  if (!pts || pts.length < 2) return;
  g.save();
  g.lineJoin = "round";
  g.lineCap = "round";
  g.beginPath();
  pts.forEach((p, i) => (i ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)));
  g.setLineDash([]);
  g.strokeStyle = "rgba(10,8,6,0.45)";
  g.lineWidth = width + 2.5;
  g.stroke();
  const dash = Math.max(4, width * 2.6);
  g.setLineDash([dash, dash * 1.1]);
  // A positive offset moves the dashes back along the path: toward its first point, the army.
  g.lineDashOffset = offset;
  g.strokeStyle = color;
  g.lineWidth = width;
  g.stroke();
  g.restore();
}

/** Draws mark `m` (supplyMarks) centred at (x, y) on 2D context `g`, `fs` its font size. */
export function drawMark(g, m, x, y, fs, font) {
  g.save();
  g.font = `700 ${Math.round(fs)}px ${font}`;
  g.textAlign = "center";
  g.textBaseline = "middle";
  const w = g.measureText(m.text).width + fs;
  const hgt = fs + 6;
  const x0 = x - w / 2;
  const y0 = y - hgt / 2;
  const r = hgt / 2;
  g.beginPath();
  g.moveTo(x0 + r, y0);
  g.arcTo(x0 + w, y0, x0 + w, y0 + hgt, r);
  g.arcTo(x0 + w, y0 + hgt, x0, y0 + hgt, r);
  g.arcTo(x0, y0 + hgt, x0, y0, r);
  g.arcTo(x0, y0, x0 + w, y0, r);
  g.closePath();
  g.fillStyle = m.state === "starvation" ? "rgba(120,20,16,0.92)" : "rgba(110,72,10,0.92)";
  g.fill();
  g.lineWidth = 1.5;
  g.strokeStyle = m.state === "starvation" ? "#ff7a5a" : "#ffc94a";
  g.stroke();
  g.fillStyle = "#fff4dc";
  g.fillText(m.text, x, y + 0.5);
  g.restore();
}
