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

// ---- The HD-2D map's look (app/sea.js, drawn by r3d/sealayer.js) -------------------------------

/** A map given row by row from the south: `sea_id` and, optionally, `sea`. */
function seaMap(ids, range = null, wrap = false) {
  return { w: ids[0].length, h: ids.length, wrap_x: wrap, wrap_y: false, sea_id: ids.flat(), ...(range ? { sea: range.flat() } : {}) };
}

test("a name's layout: along a long sea's axis, inside its water, never upside down", () => {
  // A sea running east-west, 8 x 2, in land.
  const ew = seaMap([
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, 4, 4, 4, 4, 4, 4, 4, 4, -1],
    [-1, 4, 4, 4, 4, 4, 4, 4, 4, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
  ]);
  const a = Sea.seaLabelLayout(ew, { id: 4, x: 3, y: 1 });
  assert.equal(a.angle, 0, "level along an east-west sea");
  assert.equal(a.n, 16);
  assert.equal(a.len, 8);
  assert.equal(a.across, 2);
  assert.ok(Sea.inSea(ew, 4, a.x, a.y), "its centre on the sea");
  assert.ok(a.x > 3.5 && a.x < 6.5, "near the middle");
  // A diagonal sea running south-west to north-east: tilted up to the right, at most 45 degrees.
  const ids = [];
  for (let y = 0; y < 9; y++) ids.push(Array.from({ length: 9 }, (_, x) => (Math.abs(x - y) <= 1 ? 2 : -1)));
  const d = Sea.seaLabelLayout(seaMap(ids), { id: 2, x: 4, y: 4 });
  assert.ok(d.angle > 0.6 && d.angle <= Math.PI / 4 + 1e-9, `angle ${d.angle}`);
  // The other diagonal: tilted down to the right, never past -45 degrees (and so never upside down).
  const flip = ids.map((row) => [...row].reverse());
  const e = Sea.seaLabelLayout(seaMap(flip), { id: 2, x: 4, y: 4 });
  assert.ok(e.angle < -0.6 && e.angle >= -Math.PI / 4 - 1e-9, `angle ${e.angle}`);
  assert.ok(Math.abs(Math.cos(e.angle)) > 0.7, "reads left to right");
  // A round sea: level.
  const round = seaMap([
    [-1, 1, 1, 1, -1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [-1, 1, 1, 1, -1],
  ]);
  const r = Sea.seaLabelLayout(round, { id: 1, x: 2, y: 2 });
  assert.equal(r.angle, 0);
  assert.deepEqual(r.plot, [2, 2], "the deepest plot, at the centre");
  assert.equal(r.depth, 1);
  // Sized to the sea, clamped.
  assert.ok(r.h > a.h - 1e-9 || r.n <= a.n);
  assert.equal(Sea.seaLabelLayout(seaMap([[1]]), { id: 1, x: 0, y: 0 }).h, 0.3, "the smallest clamp");
  assert.equal(Sea.seaLabelLayout(round, { id: 9, x: 0, y: 0 }), null, "no revealed plot: no name");
  // A name too long for its sea is made smaller, never below the floor.
  assert.equal(Sea.fitNameHeight({ h: 1, len: 4 }, 6), 0.6);
  assert.equal(Sea.fitNameHeight({ h: 1, len: 40 }, 6), 1);
  assert.equal(Sea.fitNameHeight({ h: 1, len: 1 }, 20), 0.3);
});

test("a name's layout on a wrapping map takes the copy of each plot nearest the label", () => {
  // Sea 3 across the seam: columns 9, 0 and 1 of a 10-wide map.
  const row = [3, 3, -1, -1, -1, -1, -1, -1, -1, 3];
  const m = seaMap([row, row], null, true);
  const pts = Sea.seaPlots(m, 3, 0);
  assert.deepEqual(pts.map(([x]) => x).sort((p, q) => p - q), [-1, -1, 0, 0, 1, 1]);
  const lay = Sea.seaLabelLayout(m, { id: 3, x: 0, y: 0 });
  assert.equal(lay.len, 3, "one stretch over the seam, not the map's width");
  assert.ok(Sea.inSea(m, 3, lay.x, lay.y));
});

test("the buoy stands on the sea's water, beside its name", () => {
  const m = seaMap([
    [-1, -1, -1, -1, -1, -1, -1, -1],
    [-1, 5, 5, 5, 5, 5, 5, -1],
    [-1, 5, 5, 5, 5, 5, 5, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1],
  ]);
  const lay = Sea.seaLabelLayout(m, { id: 5, x: 3, y: 1 });
  const b = Sea.seaBuoySpot(m, { id: 5 }, lay, 2);
  assert.ok(Sea.inSea(m, 5, b.x, b.y));
  assert.ok(b.x < lay.x, "west of the name");
  // A name wider than its water: the buoy still finds the sea.
  const c = Sea.seaBuoySpot(m, { id: 5 }, lay, 30);
  assert.ok(Sea.inSea(m, 5, c.x, c.y));
});

test("the bands: by state, in the holder's colour", () => {
  const mine = "#111111", them = "#222222";
  assert.deepEqual(Sea.seaBand({ state: "held_you" }, mine, them), { code: Sea.BAND.HELD_YOU, a: mine, b: mine, strength: 1 });
  assert.deepEqual(Sea.seaBand({ state: "held" }, mine, them), { code: Sea.BAND.HELD, a: them, b: them, strength: 1 });
  assert.equal(Sea.seaBand({ state: "held", inferred: true }, mine, null).a, Sea.SEA_LOOK.band.unknown, "an unknown holder");
  assert.deepEqual(Sea.seaBand({ state: "contested" }, mine, them), { code: Sea.BAND.CONTESTED, a: mine, b: them, strength: 1 });
  const p = Sea.seaBand({ state: "patrolled" }, mine, them);
  assert.ok(p.code === Sea.BAND.PATROLLED && p.a === them && p.strength < 1, "fainter");
  for (const state of ["open", "out", undefined]) assert.equal(Sea.seaBand({ state }, mine, them).code, Sea.BAND.NONE);
  const players = [{ id: 0, team: 0 }, { id: 3, team: 2 }];
  assert.equal(Sea.teamColorOf(players, 2, (id) => `c${id}`), "c3");
  assert.equal(Sea.teamColorOf(players, 7, (id) => `c${id}`), null);
  assert.equal(Sea.teamColorOf(players, null, (id) => `c${id}`), null);
});

test("the names fade with zoom, and show faintly far out with the layer off", () => {
  const on = (ts) => Sea.seaNameAlpha(ts, true);
  const off = (ts) => Sea.seaNameAlpha(ts, false);
  assert.ok(on(28) >= on(96) && on(96) > on(168) && on(168) > 0, "clear far out, faint close in");
  assert.ok(off(28) > 0 && off(28) < on(28), "fainter with the layer off");
  assert.ok(off(40) > 0 && off(56) === 0 && off(96) === 0, "only at the farthest zooms");
});

test("the tooltip: name, badge, then the rules", () => {
  assert.deepEqual(Sea.seaTipLines({ name: "Sirens' Water", badge: "HELD: LANUN 0 to 31", state: "held" }), [
    "Sirens' Water",
    "HELD: LANUN 0 to 31",
    "Held at twice the enemy's strength on patrol.",
    "Only ships you see are counted.",
  ]);
  const t = Sea.seaTipLines({ name: "Bay", badge: "HELD: LANUN 4 to ?", state: "held", inferred: true }, 300);
  assert.equal(t[2], "Held at 3 times the enemy's strength on patrol.");
  assert.equal(t[3], "Its ships on patrol are not all in sight.");
  assert.equal(Sea.seaTipLines({ name: "Far", badge: "OUT OF RANGE", state: "out" })[2], "No plot of it is in your range.");
  assert.equal(Sea.holdTimes(150), "150% of");
});

test("the fog bank: none in range, thin at the range's edge, full beyond, held up over the land", () => {
  // y north; `sea`: 1 in range, 0 out, -1 land.
  const m = { w: 6, h: 1, wrap_x: false, wrap_y: false, sea: [1, 1, 0, 0, 0, -1] };
  const f = Sea.seaFogRamp(m);
  assert.deepEqual([...f], [0, 0, 128, 255, 255, 255]);
  assert.deepEqual([...Sea.seaFogRamp({ w: 3, h: 1, sea: [1, 1, -1] })], [0, 0, 0], "no water out of range");
  assert.deepEqual([...Sea.seaFogRamp({ w: 3, h: 1, sea: [0, 0, -1] })], [255, 255, 255], "no range at all");
  assert.deepEqual([...Sea.seaFogRamp({ w: 2, h: 1 })], [0, 0], "no sea array");
  // Land in range of nothing out keeps 0.
  assert.deepEqual([...Sea.seaFogRamp({ w: 4, h: 1, sea: [-1, 1, 0, -1] })], [0, 0, 128, 128]);
});

test("a lane's curve: through the plots' centres, rounded, measured", () => {
  const c = Sea.laneCurve([[0, 0], [1, 0], [1, 1]]);
  assert.deepEqual([c[0].x, c[0].y], [0.5, 0.5], "starts at the first plot's centre");
  assert.deepEqual([c.at(-1).x, c.at(-1).y], [1.5, 1.5], "ends at the last's");
  assert.equal(c.length, 4, "the corner cut");
  assert.ok(c.every((p, k) => !k || p.s > c[k - 1].s), "the distance grows");
  assert.ok(c.at(-1).s < 2 && c.at(-1).s > 1.5);
  assert.deepEqual(Sea.laneCurve([[2, 3]]), [{ x: 2.5, y: 3.5, s: 0 }]);
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
