// Outposts' lanes on the Sea layer (this game's own rules, docs/NAVAL.md sections 9.3 and 10): the
// view's `outpost_lanes`, drawn as supply lines are (supply.js) in their owner's colour (yours in the
// interface's vellum), the dashes flowing to the home port; a cut lane breaks over the sea that cuts
// it: the rest of it to the port dotted, as a supply ship's cut lane (sea.js `drawLaneCut`), with a
// mark where it enters that sea (yours: the turns left before the outpost starves).
// Others' lanes come over the plots you have revealed only. Also the banner's tag of an outpost.
//
// The flat and isometric maps call their function from their Sea layer (`drawSea`): the projection
// is their own (map.js: the flat camera; iso.js: the diamond grid). The 3D and HD-2D maps lay the
// lanes on the water in the world (r3d/sealayer.js, from `outpostLanes` and `laneMark`). Pure apart
// from the drawing: no DOM, no state of its own.

import { S, me, isBarbarian } from "./state.js";
import * as T from "./tiles.js";
import { OWN_COLOR, unwrapSegment, drawLine, drawMark } from "./supply.js";
import { drawLaneCut } from "./sea.js";

/** The lanes to draw: `[{owner, mine, color, segments: [[[x, y], ...]], cut: {x, y, sea, turns} | null}]`
 *  (each segment from the port's end toward the outpost). */
export function outpostLanes(view = S.view) {
  const mine = me()?.player ?? null;
  return (view?.outpost_lanes ?? [])
    .map((l) => ({
      owner: l.owner,
      mine: l.owner === mine,
      color: l.owner === mine ? OWN_COLOR : T.playerColor(l.owner, isBarbarian(l.owner)),
      segments: (l.segments ?? []).filter((s) => s.length > 1),
      cutSegments: (l.cut_segments ?? []).filter((s) => s.length > 1),
      cut: l.cut ?? null,
    }))
    .filter((l) => l.segments.length || l.cutSegments.length || l.cut);
}

/** The mark of a cut lane (supply.js's `drawMark`): "Cut: 3" (the turns before the outpost starves),
 *  or "Cut" on another's lane. */
export function laneMark(cut) {
  const n = cut.turns;
  return { state: n === 0 ? "starvation" : "hunger", text: n === null || n === undefined ? "Cut" : n === 0 ? "Cut: starved" : `Cut: ${n}` };
}

/** The banner's tag of an outpost (the cities' `outpost` field): " (outpost)", else "". */
export function outpostTag(city) {
  return city?.outpost ? " (outpost)" : "";
}

/** map.js (the flat map, drawn on demand: the lines stand still). `m` the MapView. */
export function drawLanesFlat(m, g, font = "sans-serif") {
  const ts = m.cam.ts;
  const drawn = { lines: 0, marks: 0 };
  m.lanesDrawn = drawn;
  for (const L of outpostLanes()) {
    const flat = (seg) => {
      const s0 = T.mapToScreen(m.cam, m.map, m.vw, m.vh, seg[0][0], seg[0][1]);
      return unwrapSegment(seg, 0, m.map).map(([dx, y]) => ({ x: s0.sx + dx * ts + ts / 2, y: T.columnToScreen(m.cam, m.vw, m.vh, 0, y).sy + ts / 2 }));
    };
    for (const seg of L.segments) {
      drawLine(g, flat(seg), L.color, 2.5, 0);
      drawn.lines++;
    }
    for (const seg of L.cutSegments) drawLaneCut(g, flat(seg), Math.max(6, Math.min(11, ts * 0.22)));
    if (L.cut) {
      const p = T.mapToScreen(m.cam, m.map, m.vw, m.vh, L.cut.x, L.cut.y);
      drawMark(g, laneMark(L.cut), p.sx + ts / 2, p.sy + ts, Math.max(9, Math.min(12, ts * 0.2)), font);
      drawn.marks++;
    }
  }
}

/** iso.js (the isometric map, drawn on demand: the lines stand still). `iso` the IsoMap, `f` its frame. */
export function drawLanesIso(iso, g, f, font = "sans-serif") {
  const v = iso.view;
  const drawn = { lines: 0, marks: 0 };
  iso.lanesDrawn = drawn;
  const width = Math.max(2, 4 * f.s);
  for (const L of outpostLanes()) {
    const onIso = (seg) => {
      const c0 = iso.center(seg[0][0], seg[0][1]);
      return unwrapSegment(seg, seg[0][0], v.map).map(([ux, y]) => {
        const st = iso.step(ux - seg[0][0], y - seg[0][1]);
        const x = T.wrapCoord(ux, v.map.w, v.map.wrap_x) ?? ux;
        return { x: c0.sx + st.x, y: c0.sy + st.y + iso.dropAt(x, y) };
      });
    };
    for (const seg of L.segments) {
      drawLine(g, onIso(seg), L.color, width, 0);
      drawn.lines++;
    }
    for (const seg of L.cutSegments) drawLaneCut(g, onIso(seg), Math.max(6, 11 * f.s * 2));
    if (L.cut) {
      const c = iso.center(L.cut.x, L.cut.y);
      drawMark(g, laneMark(L.cut), c.sx, c.sy + iso.dropAt(L.cut.x, L.cut.y) + 24 * f.s, Math.max(9, 12 * f.s), font);
      drawn.marks++;
    }
  }
}
