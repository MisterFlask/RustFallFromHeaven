// node --test: the tech tree's layout (app/empire.js `treeLayout`): columns by prerequisite
// depth, rows in a band per research path, a tech near its prerequisites.
import test from "node:test";
import assert from "node:assert/strict";
import { treeLayout, treeRelated } from "../app/empire.js";

const tech = (id, depth, and = [], or = []) => ({ tech: id, depth, and, or });
const path = (id, techs, capstones = []) => ({ path: id, techs, capstones });

// Two paths: path 10 (techs 2, 4, 6), path 11 (techs 3, 5, 6; 6 is its capstone); 0, 1, 7 in none.
const TECHS = [
  tech(0, 0),
  tech(1, 0),
  tech(2, 1, [0]),
  tech(3, 1, [1]),
  tech(4, 2, [2]),
  tech(5, 2, [3]),
  tech(6, 3, [4], [5]),
  tech(7, 3, [4]),
  tech(8, 0),
  tech(9, 4, [6, 7]),
  tech(12, 2, [0]),
  tech(13, 2, [2, 3]),
];
const PATHS = [path(10, [2, 4, 6]), path(11, [3, 5, 6], [6])];

/** Checks what holds for any layout of `techs`. */
function check(techs, paths) {
  const L = treeLayout(techs, paths);
  const byId = new Map(techs.map((x) => [x.tech, x]));
  // Every tech has a lane and a row inside that lane's band.
  for (const x of techs) {
    const ln = L.lanes[L.lane.get(x.tech)];
    assert.ok(ln, `tech ${x.tech} has a lane`);
    const r = L.row.get(x.tech);
    assert.ok(r >= ln.top && r < ln.top + ln.rows, `tech ${x.tech} in its band`);
  }
  // A prerequisite is in an earlier column.
  for (const x of techs) for (const p of [...x.and, ...x.or]) if (byId.has(p)) assert.ok(byId.get(p).depth < x.depth);
  // No two techs share a cell.
  const cells = new Set(techs.map((x) => `${x.depth}:${L.row.get(x.tech)}`));
  assert.equal(cells.size, techs.length);
  // The bands follow each other without overlapping.
  let top = 0;
  for (const ln of L.lanes) {
    assert.equal(ln.top, top);
    assert.ok(ln.rows >= 1);
    top += ln.rows;
  }
  assert.equal(L.rows, top);
  return L;
}

test("lanes: a path's techs, a capstone, the neighbours' lane, General", () => {
  const L = check(TECHS, PATHS);
  const laneOf = (t) => L.lanes[L.lane.get(t)].path;
  // In one path: its lane; in two: the one it is a capstone of.
  assert.deepEqual([2, 4].map(laneOf), [10, 10]);
  assert.deepEqual([3, 5, 6].map(laneOf), [11, 11, 11]);
  // In none: the lane of most of its prerequisites and successors (7: after 4, before 9 which
  // follows 6 and 7; 0 leads to 2 and 12, 12 follows 0).
  assert.equal(laneOf(7), 10);
  assert.equal(laneOf(0), 10);
  assert.equal(laneOf(12), 10);
  assert.equal(laneOf(1), 11);
  // A tie (13: one prerequisite in each lane) goes to the lower lane; a tech whose neighbours
  // get their lane in the same round takes the lane of those that had one (9: 6's).
  assert.equal(laneOf(13), 10);
  assert.equal(laneOf(9), 11);
  assert.equal(laneOf(8), null);
  assert.equal(L.columns, 5);
  // General sits in the middle.
  assert.equal(L.lanes[1].path, null);
});

test("a chain runs straight; a lone column is centred on its prerequisites", () => {
  const L = check(
    [tech(0, 0), tech(1, 1, [0]), tech(2, 1, [0]), tech(3, 1, [0]), tech(4, 2, [3]), tech(5, 3, [4])],
    [],
  );
  assert.equal(L.lanes.length, 1);
  assert.equal(L.lanes[0].rows, 3);
  // 0 alone in its column sits in the middle of the three it leads to; 4 and 5 follow 3.
  assert.equal(L.row.get(0), 1);
  assert.equal(L.row.get(4), L.row.get(3));
  assert.equal(L.row.get(5), L.row.get(3));
});

test("deterministic, and empty", () => {
  const a = treeLayout(TECHS, PATHS);
  const b = treeLayout([...TECHS].reverse(), PATHS);
  assert.deepEqual([...a.row].sort((x, y) => x[0] - y[0]), [...b.row].sort((x, y) => x[0] - y[0]));
  assert.deepEqual(a.lanes, b.lanes);
  assert.deepEqual(treeLayout([], []), { row: new Map(), lane: new Map(), lanes: [], columns: 0, rows: 0 });
  // Without paths everything is General.
  const g = check(TECHS, []);
  assert.deepEqual(g.lanes.map((l) => l.path), [null]);
});

test("a tech's path (all its prerequisites) and the techs it leads to", () => {
  const all = [tech(0, 0), tech(1, 0), tech(2, 1, [1]), tech(3, 1, [0]), tech(4, 2, [], [2, 3]), tech(5, 3, [4])];
  const byId = new Map(all.map((x) => [x.tech, x]));
  const { path, next } = treeRelated(byId, 4);
  assert.deepEqual([...path].sort(), [0, 1, 2, 3, 4]);
  assert.deepEqual([...next], [5]);
  assert.deepEqual([...treeRelated(byId, 5).next], []);
});
