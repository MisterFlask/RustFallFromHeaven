// node --test: the Sea layer's helpers, from app/sea.js (docs/NAVAL.md section 10): the plots'
// states from the map's `sea` array, the range's edges, the automatic switch, the wear pip, the
// port line, a side of a plot toward a neighbour.
import test from "node:test";
import assert from "node:assert/strict";
import * as Sea from "../app/sea.js";

/** A 4 x 3 map (wraps east-west), y north; `sea` given row by row from the south. */
function map(rows, wrap = true) {
  const h = rows.length;
  const w = rows[0].length;
  return { w, h, wrap_x: wrap, wrap_y: false, sea: rows.flat() };
}

test("plot states from the sea array", () => {
  const m = map([
    [1, 2, 0, -1],
    [-1, 1, 0, 0],
    [-1, -1, -1, -1],
  ]);
  assert.equal(Sea.seaState(m, 0), "in");
  assert.equal(Sea.seaState(m, 2), "out");
  assert.equal(Sea.seaState(m, 3), null, "land or unrevealed");
  assert.equal(Sea.seaState({ w: 1, h: 1 }, 0), null, "a map without the array");
  assert.equal(Sea.seaValue(m, 1), 2);
});

test("the range's edges: in-range water next to out-of-range water", () => {
  const m = map([
    [1, 2, 0, -1],
    [-1, 1, 0, 0],
    [-1, -1, -1, -1],
  ]);
  // (1, 0): east is out of range; (1, 1): east too; north of (1, 0) is in range.
  assert.deepEqual(Sea.rangeEdges(m, 1, 0), [[1, 0]]);
  assert.deepEqual(Sea.rangeEdges(m, 1, 1), [[1, 0]]);
  // (0, 0) wraps west to (3, 0): land, no edge.
  assert.deepEqual(Sea.rangeEdges(m, 0, 0), []);
  assert.deepEqual(Sea.rangeEdges(m, 2, 0), [], "an out-of-range plot draws no edge");
  assert.deepEqual(Sea.seaPlot(m, 2, 0), { dark: true, edges: [] });
  assert.deepEqual(Sea.seaPlot(m, 1, 0), { dark: false, edges: [[1, 0]] });
  assert.equal(Sea.seaPlot(m, 0, 2), null);
  // Wrapping: (3, 1) is out; its west neighbour (2, 1) is out too, its east neighbour (0, 1) land.
  const w = map([[1, 1, 1, 0]]);
  assert.deepEqual(Sea.rangeEdges(w, 0, 0), [[-1, 0]], "west across the seam");
  assert.deepEqual(Sea.rangeEdges(map([[1, 1, 1, 0]], false), 0, 0), [], "no seam without wrap");
});

test("the Sea layer turns on with a ship or boats and goes back after", () => {
  const ship = (u) => u.type === "ship";
  assert.equal(Sea.wantsSea([{ type: "warrior" }], ship), false);
  assert.equal(Sea.wantsSea([{ type: "warrior", boats: true }], ship), true);
  assert.equal(Sea.wantsSea([{ type: "ship" }], ship), true);
  assert.equal(Sea.wantsSea([], ship), false);
  // On by itself, and back to the layer before.
  let r = Sea.autoSeaLayer("political", null, true);
  assert.deepEqual(r, { layer: "sea", auto: { prev: "political" } });
  r = Sea.autoSeaLayer(r.layer, r.auto, true);
  assert.deepEqual(r, { layer: "sea", auto: { prev: "political" } }, "stays while wanted");
  r = Sea.autoSeaLayer(r.layer, r.auto, false);
  assert.deepEqual(r, { layer: "political", auto: null });
  // Picked by the player (no memory): it stays.
  assert.deepEqual(Sea.autoSeaLayer("sea", null, false), { layer: "sea", auto: null });
  assert.deepEqual(Sea.autoSeaLayer("yield", null, false), { layer: "yield", auto: null });
});

test("the wear pip and the port line", () => {
  const units = [
    { owner: 0, wear: 5 },
    { owner: 0, wear: 20 },
    { owner: 1, wear: 50 },
    { owner: 0 },
  ];
  assert.equal(Sea.stackWear(units, 0), 20, "your units' worst");
  assert.equal(Sea.stackWear(units, 2), 0);
  assert.equal(Sea.portLine({ sea_port: true }, 12), "Range: 12 plots.");
  assert.equal(Sea.portLine({ sea_port: true }, 1), "Range: 1 plot.");
  assert.equal(Sea.portLine({ sea_port: false }, 12), null);
  assert.equal(Sea.portLine({ sea_port: true }, null), null, "no range rule");
});

test("the side of a plot toward a neighbour", () => {
  // A diamond (isometric) around (0, 0): top, right, bottom, left.
  const corners = [{ x: 0, y: -1 }, { x: 2, y: 0 }, { x: 0, y: 1 }, { x: -2, y: 0 }];
  const side = Sea.sideToward(corners, { x: 2, y: -1 });
  assert.deepEqual(side.map((p) => [p.x, p.y]).sort(), [[0, -1], [2, 0]]);
  // A square.
  const sq = [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }, { x: 0, y: 1 }];
  assert.deepEqual(Sea.sideToward(sq, { x: 0.5, y: -0.5 }).map((p) => [p.x, p.y]).sort(), [[0, 0], [1, 0]]);
});

// ---- Phase 2: named seas (app/sea.js, app/seas.js) --------------------------------------------

import { setData } from "../app/state.js";
import { barPercent, sendCommands } from "../app/seas.js";

/** A map given row by row from the south: `sea_id`. */
function named(rows) {
  return { w: rows[0].length, h: rows.length, wrap_x: false, wrap_y: false, sea_id: rows.flat() };
}

test("borders between seas, each once, east and north", () => {
  const m = named([
    [0, 0, 1, -1],
    [0, 2, 1, 1],
    [-1, -1, -1, -1],
  ]);
  assert.deepEqual(Sea.seaBorders(m, 0, 0), [], "same sea east and north");
  assert.deepEqual(Sea.seaBorders(m, 1, 0), [[1, 0], [0, 1]], "sea 1 east, sea 2 north");
  assert.deepEqual(Sea.seaBorders(m, 2, 0), [], "land east: no border");
  assert.deepEqual(Sea.seaBorders(m, 3, 0), [], "land has none");
  assert.equal(Sea.seaIdAt({ w: 1, h: 1 }, 0), -1, "a map without the array");
});

test("what a plot of a named sea draws, by its sea's state", () => {
  const m = named([[0, 1]]);
  const seas = Sea.seasById({ seas: [{ id: 0, state: "held" }, { id: 1, state: "open" }] });
  assert.deepEqual(Sea.seaPlotNamed(m, seas, 0, 0), { state: "held", borders: [[1, 0]] });
  assert.equal(Sea.seaPlotNamed(m, seas, 1, 0), null, "open, no border to draw from here");
  // The states differ by pattern or lightness, not by hue alone.
  const looks = Object.values(Sea.SEA_STATES).map((s) => `${s.tint}|${s.stripes?.length ?? 0}|${!!s.dots}|${s.fill}|${s.ring}`);
  assert.equal(new Set(looks).size, looks.length);
  assert.ok(Sea.SEA_STATES.held.stripes && Sea.SEA_STATES.contested.stripes.length === 2 && !Sea.SEA_STATES.held_you.stripes);
});

test("a label's box answers a click", () => {
  const hits = [{ id: 3, x0: 10, y0: 10, x1: 50, y1: 30 }];
  assert.equal(Sea.seaHitAt(hits, 20, 20), 3);
  assert.equal(Sea.seaHitAt(hits, 60, 20), null);
  assert.equal(Sea.seaHitAt(null, 0, 0), null);
  const s = Sea.seaLabelSizes(8);
  assert.ok(s.name >= 11 && s.badge >= 10, "legible zoomed out");
});

test("a lane's parts: open, then cut from the closed sea's first plot", () => {
  const lane = { path: [[5, 1], [4, 1], [3, 1], [2, 1]], cut: { at: 2, sea: "Bay", holder: "Lanun" } };
  assert.deepEqual(Sea.laneParts(lane), { open: [[5, 1], [4, 1], [3, 1]], cut: [[3, 1], [2, 1]], at: [3, 1] });
  assert.deepEqual(Sea.laneParts({ path: [[1, 1], [2, 1]], cut: null }), { open: [[1, 1], [2, 1]], cut: null, at: null });
});

test("the harbour mark: none when open, two states when blockaded", () => {
  assert.equal(Sea.harbourMark({ harbour: null }), null);
  assert.equal(Sea.harbourMark({ harbour: { state: "open" } }), null);
  assert.equal(Sea.harbourMark({ harbour: { state: "blockaded", besieged: false } }).level, 1);
  assert.equal(Sea.harbourMark({ harbour: { state: "blockaded", besieged: true } }).level, 2);
});

test("sea lines that read as a loss", () => {
  assert.ok(Sea.seaLineWarns("Enters The Grey Narrows (contested) at plot 4: will be intercepted by Lanun's patrol."));
  assert.ok(Sea.seaLineWarns("Out of range from plot 6: -15% HP a turn."));
  assert.ok(!Sea.seaLineWarns("Harbour open. The city heals and repairs at 50% during the siege."));
  assert.ok(!Sea.seaLineWarns("On Patrol: Bay of Lorda."));
});

test("the panel's bars and the send's commands", () => {
  assert.equal(barPercent(14, [14, 17]), 82);
  assert.equal(barPercent(0, [17]), 0);
  assert.equal(barPercent(1, [400]), 2, "a strength above 0 shows");
  setData({ tables: { MissionType: [{ type: "MISSION_MOVE_TO" }, { type: "MISSION_PLUNDER" }] }, defines: {} });
  assert.deepEqual(sendCommands({ here: false, units: [7], x: 4, y: 1 }), [
    { c: "move", units: [7], x: 4, y: 1, append: false },
    { c: "mission", units: [7], mission: 1, data1: -1, data2: -1, flags: 0, append: true },
  ]);
  assert.deepEqual(sendCommands({ here: true, units: [7] }), [{ c: "mission", units: [7], mission: 1, data1: -1, data2: -1, flags: 0, append: false }]);
});
