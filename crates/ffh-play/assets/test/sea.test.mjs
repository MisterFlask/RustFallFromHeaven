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
