// Area effects and tunnels on the map (docs/CARDS.md, "Area effects"; the Lanun's Raging Seas, the
// Weather Engine's Blizzard, Storm and Fog, the Pillar of Dawn; the Khazad's Delve): the view's
// `area_effects` (each with the plots of it you have revealed, sent while you see one of them) and
// `delves` (each tunnel end you have revealed). An effect tints its plots in its kind's colour and
// carries a tag at its centre ("Raging Seas: 12", the turns left); a tunnel end carries a "Delve"
// tag. The plot tooltip's lines are `plotNotes`.
//
// Each renderer calls its one function after its supply lines (map.js: the flat camera; iso.js: the
// diamond grid; r3d/hud.js: the 3D camera). Pure apart from the drawing: no DOM, no state of its own.

import { S, playerName } from "./state.js";

/** Each kind's colour: tint and tag. */
const KIND_COLOR = {
  raging_seas: "#2a6fd6",
  storm: "#7a7f8c",
  blizzard: "#d8ecff",
  fog: "#b8b8b0",
  pillar_of_dawn: "#f2c94c",
};
const DELVE_COLOR = "#8a5a2b";

/** The effects to draw: `[{kind, label, owner, x, y, turns_left, plots: [[x, y]...], color}]`. */
export function areaEffects(view = S.view) {
  return (view?.area_effects ?? []).map((e) => ({ ...e, color: KIND_COLOR[e.kind] ?? "#9a9a9a" }));
}

/** The tunnel ends to draw: `[{x, y, to: [x, y], to_place}]` (`to_place`: the other end, named by
 *  the nearest known city). */
export function delveEnds(view = S.view) {
  return view?.delves ?? [];
}

/** The tooltip's lines for plot (x, y): the effects over it and its tunnel. */
export function plotNotes(x, y, view = S.view) {
  const out = [];
  for (const e of areaEffects(view)) {
    if (e.plots.some(([px, py]) => px === x && py === y)) out.push(`${e.label} (${ownerName(e.owner)}): ${e.turns_left} ${e.turns_left === 1 ? "turn" : "turns"} left.`);
  }
  for (const d of delveEnds(view)) {
    if (d.x === x && d.y === y) out.push(`Delve: tunnel to ${d.to_place ?? "another plot"}.`);
  }
  return out;
}

function ownerName(p) {
  try {
    return playerName(p);
  } catch {
    return `Player ${p}`;
  }
}

/** A tag centred at (x, y): `text` in a rounded box of colour `color`, `fs` its font size. */
function drawTag(g, text, x, y, fs, font, color) {
  g.save();
  g.font = `700 ${Math.round(fs)}px ${font}`;
  g.textAlign = "center";
  g.textBaseline = "middle";
  const w = g.measureText(text).width + fs;
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
  g.fillStyle = "rgba(14,12,10,0.88)";
  g.fill();
  g.lineWidth = 1.5;
  g.strokeStyle = color;
  g.stroke();
  g.fillStyle = "#fff4dc";
  g.fillText(text, x, y + 0.5);
  g.restore();
}

/** Fills polygon `pts` `[{x, y}]` with `color` at `alpha`. */
function tint(g, pts, color, alpha) {
  if (!pts || pts.length < 3) return;
  g.save();
  g.globalAlpha = alpha;
  g.fillStyle = color;
  g.beginPath();
  pts.forEach((p, i) => (i ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)));
  g.closePath();
  g.fill();
  g.restore();
}

const tagText = (e) => `${e.label}: ${e.turns_left}`;

/** map.js (the flat map). `m` the MapView. */
export function drawMarksFlat(m, g, font = "sans-serif") {
  const ts = m.cam.ts;
  const fs = Math.max(9, Math.min(12, ts * 0.2));
  const drawn = { effects: 0, delves: 0 };
  m.marksDrawn = drawn;
  for (const e of areaEffects()) {
    for (const [x, y] of e.plots) m.eachCopy(x, y, (sx, sy) => tint(g, [{ x: sx, y: sy }, { x: sx + ts, y: sy }, { x: sx + ts, y: sy + ts }, { x: sx, y: sy + ts }], e.color, 0.22));
    m.eachCopy(e.x, e.y, (sx, sy) => drawTag(g, tagText(e), sx + ts / 2, sy + ts * 0.2, fs, font, e.color));
    drawn.effects++;
  }
  for (const d of delveEnds()) {
    m.eachCopy(d.x, d.y, (sx, sy) => drawTag(g, "Delve", sx + ts / 2, sy + ts * 0.2, fs, font, DELVE_COLOR));
    drawn.delves++;
  }
}

/** iso.js (the isometric map). `iso` the IsoMap, `f` its frame. */
export function drawMarksIso(iso, g, f, font = "sans-serif") {
  const fs = Math.max(9, 12 * f.s);
  const drawn = { effects: 0, delves: 0 };
  iso.marksDrawn = drawn;
  const corners = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
  const at = (x, y) => {
    const c = iso.center(x, y);
    return { x: c.sx, y: c.sy + iso.dropAt(x, y) };
  };
  for (const e of areaEffects()) {
    for (const [x, y] of e.plots) {
      const c = at(x, y);
      tint(g, corners.map(([dx, dy]) => {
        const st = iso.step(dx, dy);
        return { x: c.x + st.x, y: c.y + st.y };
      }), e.color, 0.22);
    }
    const c = at(e.x, e.y);
    drawTag(g, tagText(e), c.x, c.y - 18 * f.s, fs, font, e.color);
    drawn.effects++;
  }
  for (const d of delveEnds()) {
    const c = at(d.x, d.y);
    drawTag(g, "Delve", c.x, c.y - 18 * f.s, fs, font, DELVE_COLOR);
    drawn.delves++;
  }
}

/** r3d/hud.js (the 3D map). `r3d` the renderer. */
export function drawMarksR3d(r3d, cam, g, inView, font = "sans-serif") {
  const world = r3d.world;
  const ts = r3d.ts;
  const fs = Math.max(9, Math.min(13, ts * 0.12));
  const drawn = { effects: 0, delves: 0 };
  r3d.marksDrawn = drawn;
  const ground = (x, y, u, w) => r3d.project(cam, [r3d.nearX(x) + u, world.plotHeight(x, y) + 0.03, -(y + w)]);
  for (const e of areaEffects()) {
    for (const [x, y] of e.plots) {
      const pts = [ground(x, y, 0, 0), ground(x, y, 1, 0), ground(x, y, 1, 1), ground(x, y, 0, 1)];
      if (pts.some((p) => !p) || !pts.some((p) => inView(p))) continue;
      tint(g, pts, e.color, 0.22);
    }
    const p = ground(e.x, e.y, 0.5, 0.5);
    if (p && inView(p)) drawTag(g, tagText(e), p.x, p.y - Math.max(10, ts * 0.15), fs, font, e.color);
    drawn.effects++;
  }
  for (const d of delveEnds()) {
    const p = ground(d.x, d.y, 0.5, 0.5);
    if (p && inView(p)) {
      drawTag(g, "Delve", p.x, p.y - Math.max(10, ts * 0.15), fs, font, DELVE_COLOR);
      drawn.delves++;
    }
  }
}

