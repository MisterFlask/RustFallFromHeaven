// The ground in steps (world.js `cliff`, terrain.js buildCliffChunk; docs/HD2D.md §5f): the land a
// slab over its water, a hill's crown over its scarp, and the mesh that cuts their faces.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import * as St from "../app/state.js";
import { buildMap } from "../../r3d-lab/scenes.js";
import { World, KIND, SCARP, CLIFF_FOOT } from "../app/r3d/world.js";
import { buildChunk, TerrainMeshes, CHUNK, RES, LOW_RES } from "../app/r3d/terrain.js";
import { CLIFF } from "../app/r3d/hd2d/config.js";

const data = JSON.parse(fs.readFileSync(new URL("../../r3d-lab/data.json", import.meta.url), "utf8"));
St.setData(data);

/** The point between a (field > 0) and b where `field` crosses zero, by bisection. */
function crossing(world, field, a, b) {
  for (let k = 0; k < 40; k++) {
    const m = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    world.heightAt(m[0], m[1]);
    if (world[field] > 0) a = m;
    else b = m;
  }
  return a;
}

test("the land is a slab: a sheer face at its shore, the floor at its foot", () => {
  const rows = ["~~~~~~~~", "--------", "........", "........", "........"];
  const w = new World(buildMap(data, rows), { cliff: CLIFF });
  const flat = new World(buildMap(data, rows));
  for (let x = 0.7; x < 7.5; x += 0.45) {
    // Rows are listed north first: the land is y 0..3, the water north of it.
    const p = crossing(w, "shore", [x, 1.5], [x, 4.5]);
    w.heightAt(p[0], p[1]);
    assert.ok(Math.abs(w.shore) < 1e-6, "the shore is where the shore field crosses zero");
    assert.ok(Math.abs(w.hLand - w.hFloor - (CLIFF + CLIFF_FOOT)) < 1e-6, `the face at ${x}: ${w.hLand - w.hFloor}`);
    // On each side the ground is that side's, right from the shore: no ramp between.
    const top = w.hLand, foot = w.hFloor;
    assert.ok(Math.abs(w.heightAt(p[0], p[1] - 1e-3) - top) < 5e-3, "the land stands at its rim's height right from the shore");
    assert.ok(Math.abs(w.heightAt(p[0], p[1] + 1e-3) - foot) < 5e-3, "the floor lies at the face's foot right from the shore");
    // The same shore as the world without steps: the land only stands higher over it.
    assert.ok(Math.abs(flat.heightAt(p[0], p[1])) < 1e-4, "the shore has not moved");
  }
  // Inland the whole land is lifted by the same amount; the water's floor is lowered by the foot's.
  assert.ok(Math.abs(w.heightAt(3.5, 0.5) - flat.heightAt(3.5, 0.5) - CLIFF) < 1e-9);
  assert.ok(Math.abs(w.heightAt(3.5, 4.5) - flat.heightAt(3.5, 4.5) + CLIFF_FOOT) < 1e-9);
  // The 3D renderer's world has no step anywhere.
  assert.equal(flat.cliff, 0);
});

test("a hill's crown stands over a scarp on its front and rises smoothly round its back", () => {
  const rows = Array.from({ length: 5 }, (_, y) => (y === 2 ? "..h.." : "....."));
  const map = buildMap(data, rows);
  const w = new World(map, { cliff: CLIFF });
  let most = 0, least = Infinity;
  const front = [0, 0];
  for (let k = 0; k < 48; k++) {
    const a = (k / 48) * Math.PI * 2, dir = [Math.cos(a), Math.sin(a)];
    w.heightAt(2.5, 2.5);
    assert.ok(w.scarp > 0, "the hill's middle is on its crown");
    const p = crossing(w, "scarp", [2.5, 2.5], [2.5 + dir[0] * 1.2, 2.5 + dir[1] * 1.2]);
    w.heightAt(p[0], p[1]);
    const step = w.hCrown - w.hLand;
    assert.ok(step >= -1e-9 && step <= SCARP + 1e-9, `the scarp's height ${step}`);
    // On each side of the scarp the ground is that side's, right from it.
    const top = w.hCrown, foot = w.hLand;
    assert.ok(Math.abs(w.heightAt(p[0] - dir[0] * 1e-3, p[1] - dir[1] * 1e-3) - top) < 5e-3);
    assert.ok(Math.abs(w.heightAt(p[0] + dir[0] * 1e-3, p[1] + dir[1] * 1e-3) - foot) < 5e-3);
    most = Math.max(most, step);
    least = Math.min(least, step);
    // (Where the scarp stands: the mean of those directions is the way it faces.)
    front[0] += dir[0] * step;
    front[1] += dir[1] * step;
  }
  assert.ok(most > SCARP * 0.69, `the scarp stands ${most} at its middle`);
  assert.ok(least < 1e-6, `round the back the crown rises without a step (${least})`);
  assert.ok(front[1] < 0, "a hill's scarp faces about south, toward the camera");
  // The 3D renderer's hill is the same hill without the step: continuous everywhere.
  const flat = new World(map);
  for (let k = 0; k < 200; k++) {
    const x = 1.5 + (k % 20) * 0.1, y = 1.5 + Math.floor(k / 20) * 0.2;
    assert.ok(Math.abs(flat.heightAt(x + 1e-4, y) - flat.heightAt(x, y)) < 1e-3, "no step without `cliff`");
  }
});

test("a scarp gives way to a river", () => {
  const rows = Array.from({ length: 5 }, (_, y) => (y === 2 ? "..h.." : "....."));
  // A river along the hill plot's south edge (the edge its scarp faces): corners (2, 2) .. (3, 2).
  const bits = [];
  for (let x = 0; x < 5; x++) bits.push([x, 2, 1]);
  const w = new World(buildMap(data, rows, { rivers: bits }), { cliff: CLIFF });
  for (let x = 2.05; x < 3; x += 0.1) {
    for (let y = 1.75; y < 2.3; y += 0.05) {
      if (w.riverDist(x, y) > 0.3) continue;
      w.heightAt(x, y);
      assert.ok(w.hCrown - w.hLand < 1e-6, `no step over the river's channel at ${x}, ${y}`);
    }
  }
});

test("the unknown is lifted with the land, and lies low beside known water", () => {
  const rows = ["~~~~~~", "------", "......", "......", "......"];
  // Hidden: (5, 1), land with land round it; (0, 3), coast beside known coast and ocean.
  const map = buildMap(data, rows, { hidden: [[5, 1], [0, 3]] });
  const w = new World(map, { cliff: CLIFF });
  assert.equal(w.kindAt(5, 1), KIND.VOID);
  assert.equal(w.voidLow[1 * w.w + 5], 0);
  assert.equal(w.voidLow[3 * w.w + 0], 1);
  // Known land runs level to the unknown beside it: no slope of the land's lift.
  assert.ok(Math.abs(w.heightAt(4.5, 1.5) - w.heightAt(5.5, 1.5)) < 0.1, "land and the unknown beside it stand level");
  // The unknown beside known water meets it without a face.
  const p = crossing(w, "shore", [0.5, 3.5], [1.5, 3.5]);
  w.heightAt(p[0], p[1]);
  assert.ok(w.hLand - w.hFloor < 0.03, `no face between the unknown and known water (${w.hLand - w.hFloor})`);
});

/** The plan area of a mesh's triangles, its faces (level normals) apart. */
function meshStats(mesh) {
  let ground = 0, faces = 0;
  const d = mesh.data;
  for (let t = 0; t < mesh.idx.length; t += 3) {
    const [a, b, c] = [0, 1, 2].map((k) => mesh.idx[t + k] * 6);
    const area = Math.abs((d[b] - d[a]) * (d[c + 2] - d[a + 2]) - (d[c] - d[a]) * (d[b + 2] - d[a + 2])) / 2;
    if (d[a + 4] === 0 && d[b + 4] === 0 && d[c + 4] === 0) {
      faces++;
      assert.ok(area < 1e-9, "a face stands upright");
      for (const v of [a, b, c]) {
        const top = Math.hypot(d[v + 3], d[v + 4], d[v + 5]) - 2;
        assert.ok(d[v + 1] <= top + 1e-5, "a face's vertex lies at or under the rim its normal carries");
      }
    } else ground += area;
  }
  return { ground, faces };
}

test("the stepped ground's mesh: cut along the shore and the scarps, whole, and the same on both sides of a chunk's edge", () => {
  // A coast, a lake, hills at the coast and across a chunk's edge (x 16), a river.
  const rows = Array.from({ length: 20 }, (_, r) => {
    const y = 19 - r;
    let s = "";
    for (let x = 0; x < 24; x++) {
      if (y >= 17 || x === 0) s += y >= 18 || x === 0 ? "~" : "-";
      else if (x === 6 && y === 8) s += "-";
      else if ((x + y * 3) % 7 === 0 || (x >= 15 && x <= 17 && y % 4 === 1) || (y === 16 && x % 5 === 2)) s += "h";
      else s += x % 9 === 4 ? "," : ".";
    }
    return s;
  });
  const bits = [];
  for (let y = 3; y < 17; y++) bits.push([11, y, 2]);
  const w = new World(buildMap(data, rows, { rivers: bits }), { cliff: CLIFF });
  for (const res of [RES, LOW_RES]) {
    const a = buildChunk(w, 0, 0, res), b = buildChunk(w, 1, 0, res);
    for (const m of [a, b]) {
      assert.ok(m.data.every(Number.isFinite), "every vertex is a number");
      assert.ok(m.idx.every((i) => i < m.data.length / 6));
      const st = meshStats(m);
      assert.ok(Math.abs(st.ground - m.pw * m.ph) < 1e-6, `the ground covers the chunk once (${st.ground} of ${m.pw * m.ph}, ${res} a plot)`);
      assert.ok(st.faces > 0, "the shore and the scarps have their faces");
    }
    // The vertices on the shared edge: the same places and heights from both sides (no crack).
    const edge = (m) => {
      const out = new Set();
      for (let v = 0; v < m.data.length; v += 6) if (Math.abs(m.data[v] - CHUNK) < 1e-9) out.add(`${m.data[v + 2].toFixed(5)},${m.data[v + 1].toFixed(5)}`);
      return [...out].sort();
    };
    assert.deepEqual(edge(a), edge(b), `the two chunks meet along their edge (${res} a plot)`);
  }
  // The mesh is the world's ground: a vertex stands at one of the heights the ground has there.
  const m = buildChunk(w, 0, 0, RES);
  for (let v = 0; v < m.data.length; v += 6 * 37) {
    const x = m.data[v], y = -m.data[v + 2];
    w.heightAt(x, y);
    const hs = [w.heightAt(x, y), w.hFloor, w.hLand, w.hCrown];
    assert.ok(hs.some((h) => Math.abs(h - m.data[v + 1]) < 2e-3), `vertex at ${x}, ${y}: ${m.data[v + 1]} is none of ${hs}`);
  }
  // Without `cliff` the mesh is the plain grid.
  const flat = buildChunk(new World(buildMap(data, rows, { rivers: bits })), 0, 0, RES);
  assert.equal(flat.data.length / 6, (CHUNK * RES + 1) ** 2);
});

test("hills side by side stand on one crown, under one scarp", () => {
  const rows = Array.from({ length: 5 }, (_, y) => (y === 2 ? ".hhh." : "....."));
  const w = new World(buildMap(data, rows), { cliff: CLIFF });
  // Along the row of hills, through their middles: the ground between two of them is still crown.
  for (let x = 1.5; x <= 3.5; x += 0.05) {
    w.heightAt(x, 2.5);
    assert.ok(w.scarp > 0, `the crown holds between the hills at ${x.toFixed(2)}`);
  }
  // A step across it is one rise everywhere on the crown (no second step inside it).
  for (let x = 1.2; x <= 3.8; x += 0.07) {
    for (let y = 2.0; y <= 3.0; y += 0.07) {
      const a = w.heightAt(x, y), sa = w.scarp;
      const b = w.heightAt(x + 0.01, y), sb = w.scarp;
      if (sa > 0 && sb > 0) assert.ok(Math.abs(a - b) < 0.02, `a jump of ${Math.abs(a - b).toFixed(3)} inside the crown at ${x.toFixed(2)}, ${y.toFixed(2)}`);
    }
  }
});

test("a group of hills is one crown with a top for each hill", () => {
  // Two rows of hills, and a pair that touch only at a corner.
  const rows = [".......", ".hhh...", ".hhh...", ".....h.", "....h..", "......."];
  const w = new World(buildMap(data, rows), { cliff: CLIFF });
  // (Rows are listed north first: the block's rows are y 3 and 4.) Between the rows and between the
  // pair, all the way across, the ground is crown: no strip of the plain parts them.
  for (let x = 1.3; x <= 3.7; x += 0.05) {
    w.heightAt(x, 4);
    assert.ok(w.scarp > 0, `the crown holds between the rows at ${x.toFixed(2)}`);
  }
  for (let t = 0.2; t <= 0.8; t += 0.05) {
    w.heightAt(4.5 + t, 1.5 + t);
    assert.ok(w.scarp > 0, `the crown holds between the corner pair at ${t.toFixed(2)}`);
  }
  // Each hill's top stands over the saddles to its neighbours.
  const top = (x, y) => {
    let best = -Infinity;
    for (let dy = -0.25; dy <= 0.25; dy += 0.05) for (let dx = -0.25; dx <= 0.25; dx += 0.05) best = Math.max(best, w.heightAt(x + 0.5 + dx, y + 0.5 + dy));
    return best;
  };
  for (const y of [3, 4]) {
    for (const x of [1, 2]) {
      const saddle = Math.min(...[-0.1, 0, 0.1].map((dy) => w.heightAt(x + 1, y + 0.5 + dy)));
      assert.ok(Math.min(top(x, y), top(x + 1, y)) - saddle > 0.03, `a saddle between the tops of (${x}, ${y}) and (${x + 1}, ${y})`);
    }
  }
});

test("a plot revealed in the map's edge rows drops the parchment's chunk past the edge", () => {
  const rows = Array.from({ length: 16 }, () => "-".repeat(16));
  const w = new World(buildMap(data, rows), { cliff: CLIFF });
  const meshes = new TerrainMeshes(null);
  meshes.world = w;
  const fake = () => ({ mesh: { dispose() {} } });
  for (const k of ["0,-1,6", "0,0,6", "0,1,6", "0,0,2"]) meshes.chunks.set(k, fake());
  // Plot (3, 0), in the south edge row: the ground changes up to two plots round it, past the edge.
  meshes.setWorld(w, [3]);
  assert.deepEqual([...meshes.chunks.keys()], ["0,1,6"], "the map's chunk and the one south of it are rebuilt");
  // Plot (3, 15), in the north edge row of a map sixteen rows high.
  for (const k of ["0,-1,6", "0,0,6", "0,1,6"]) meshes.chunks.set(k, fake());
  meshes.setWorld(w, [15 * 16 + 3]);
  assert.deepEqual([...meshes.chunks.keys()].sort(), ["0,-1,6"], "the map's chunk and the one north of it are rebuilt");
});
