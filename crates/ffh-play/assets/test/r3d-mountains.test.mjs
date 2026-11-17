// The mountain's open rim must meet the visible terrain, including on slopes and at the sea.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import * as St from "../app/state.js";
import { buildMap, buildView } from "../../r3d-lab/scenes.js";
import { World } from "../app/r3d/world.js";
import { buildChunk, CHUNK, RES, LOW_RES } from "../app/r3d/terrain.js";
import { dressStatic } from "../app/r3d/dress.js";
import { buildAsset, buildAssetLOD } from "../app/r3d/assets/registry.js";
import "../app/r3d/assets/index.js";

const data = JSON.parse(fs.readFileSync(new URL("../../r3d-lab/data.json", import.meta.url), "utf8"));
St.setData(data);

// Extract actual open mesh edges, independently of the mountain's placement or recipe constants.
const rims = new WeakMap();
function rim(mesh) {
  if (rims.has(mesh)) return rims.get(mesh);
  const edges = new Map();
  const key = (v) => v.map((n) => Math.round(n * 1e6)).join(",");
  for (let t = 0; t < mesh.count; t += 3) {
    const vs = [0, 1, 2].map((k) => Array.from(mesh.data.slice((t + k) * 13, (t + k) * 13 + 3)));
    for (let k = 0; k < 3; k++) {
      const a = vs[k], b = vs[(k + 1) % 3];
      const id = [key(a), key(b)].sort().join(";");
      const e = edges.get(id);
      if (e) e.count++;
      else edges.set(id, { a, b, count: 1 });
    }
  }
  const points = [];
  for (const { a, b, count } of edges.values()) {
    // Teeth and boulders also have open edges; only the buried skirt is a ground contact.
    if (count !== 1 || a[1] > 0 || b[1] > 0) continue;
    for (const t of [0, 0.5, 1]) points.push(a.map((v, k) => v + (b[k] - v) * t));
  }
  assert.ok(points.length > 0, "the mountain has an open rim to bury");
  rims.set(mesh, points);
  return points;
}

// Sample the triangles actually drawn, so the coarse terrain cannot hide a gap in the test: the
// lowest of those over the point (where the ground steps, world.js `cliff`, a point on the step
// has the upper ground's triangle and the lower's; a face, seen from above, is none).
function terrainHeight(world, res) {
  const chunks = new Map();
  return (mx, my) => {
    mx = ((mx % world.w) + world.w) % world.w;
    const cx = Math.floor(mx / CHUNK), cy = Math.floor(my / CHUNK);
    const key = `${cx},${cy}`;
    if (!chunks.has(key)) {
      // The chunk's triangles by the grid square they lie in.
      const mesh = buildChunk(world, cx, cy, res);
      const cells = new Map();
      for (let t = 0; t < mesh.idx.length; t += 3) {
        const tri = [0, 1, 2].map((k) => mesh.data.slice(mesh.idx[t + k] * 6, mesh.idx[t + k] * 6 + 3));
        const x = Math.floor(((tri[0][0] + tri[1][0] + tri[2][0]) / 3 - mesh.x0) * res), y = Math.floor((-(tri[0][2] + tri[1][2] + tri[2][2]) / 3 - mesh.y0) * res);
        const k = `${x},${y}`;
        if (!cells.has(k)) cells.set(k, []);
        cells.get(k).push(tri);
      }
      chunks.set(key, { mesh, cells });
    }
    const { mesh, cells } = chunks.get(key);
    const x = Math.floor((mx - mesh.x0) * res), y = Math.floor((my - mesh.y0) * res);
    let low = Infinity;
    for (const [a, b, c] of cells.get(`${x},${y}`) ?? []) {
      const den = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
      if (Math.abs(den) < 1e-12) continue;
      const u = ((b[2] - c[2]) * (mx - c[0]) + (c[0] - b[0]) * (-my - c[2])) / den;
      const v = ((c[2] - a[2]) * (mx - c[0]) + (a[0] - c[0]) * (-my - c[2])) / den;
      if (u >= -1e-6 && v >= -1e-6 && u + v <= 1 + 1e-6) low = Math.min(low, u * a[1] + v * b[1] + (1 - u - v) * c[1]);
    }
    if (low === Infinity) assert.fail(`no terrain triangle at ${mx}, ${my}`);
    return low;
  };
}

for (const cliff of [0, 0.18]) {
  test(`mountain rims meet the terrain at full and distant detail (cliff ${cliff})`, () => {
    const rows = Array.from({ length: 13 }, (_, y) => (y < 3 ? "-" : y > 8 ? "h" : ".").repeat(30));
    const map = buildMap(data, rows, { wrap: true });
    const peaks = [];
    const grounds = ["TERRAIN_GRASS", "TERRAIN_TUNDRA", "TERRAIN_DESERT", "TERRAIN_BROKEN_LANDS"];
    const units = [];
    for (const y of [2, 5, 8, 11]) for (let x = 0; x < map.w; x += 3) {
      const i = y * map.w + x;
      map.plot_type[i] = data.tables.PlotType.findIndex((p) => p.type === "PLOT_PEAK");
      map.terrain[i] = data.tables.TerrainType.findIndex((p) => p.type === grounds[peaks.length % grounds.length]);
      if (peaks.length % 2) units.push({ type: "UNIT_WARRIOR", x, y });
      peaks.push(i);
    }
    St.setMap(map);
    St.setView(buildView(data, { players: [{ civ: "CIVILIZATION_BANNOR" }], units }));
    const world = new World(map, { cliff });
    const mountains = dressStatic(world, map, peaks).list.filter((it) => it.asset === "feat.mountain");
    assert.equal(mountains.length, peaks.length);
    assert.equal(new Set(mountains.map((it) => it.variant)).size, 6, "all six variants are covered");
    const surfaces = [terrainHeight(world, RES), terrainHeight(world, LOW_RES)];
    let worst = -Infinity, where;
    for (const it of mountains) {
      const full = buildAsset(it.asset, it.variant, it.param);
      for (const mesh of [full, buildAssetLOD(it.asset, it.variant, it.param)].filter(Boolean)) {
        for (const [x, y, z] of rim(mesh)) {
          const mx = it.x + Math.cos(it.yaw) * x * it.sx + Math.sin(it.yaw) * z * it.sz;
          const my = -it.z + Math.sin(it.yaw) * x * it.sx - Math.cos(it.yaw) * z * it.sz;
          const ground = Math.max(0, Math.min(world.heightAt(mx, my), ...surfaces.map((h) => h(mx, my))));
          const gap = it.y + y * it.sy - ground;
          if (gap > worst) { worst = gap; where = `variant ${it.variant}, ${it.param || "grass"}, plot ${it.plot}`; }
        }
      }
      assert.ok(it.y + full.hi[1] * it.sy > 0.3, "a coastal mountain stays above water");
    }
    assert.ok(worst <= 0.002, `${where}: open rim floats ${worst.toFixed(4)} plots above the visible surface`);
  });
}


test("ordinary peaks add no hill pedestal; volcanoes retain their terrain cone", () => {
  const flat = buildMap(data, Array(7).fill("......."));
  const peak = buildMap(data, [".......", ".......", ".......", "...^...", ".......", ".......", "......."]);
  // Use identical ground so the comparison isolates plot elevation.
  peak.terrain = [...flat.terrain];
  const volcano = buildMap(data, [".......", ".......", ".......", "...v...", ".......", ".......", "......."]);
  const a = new World(flat), b = new World(peak), v = new World(volcano);
  for (let y = 2.5; y <= 4.5; y += 0.125) for (let x = 2.5; x <= 4.5; x += 0.125) {
    assert.ok(Math.abs(a.heightAt(x, y) - b.heightAt(x, y)) < 1e-9, "mountain supplies all its own relief");
  }
  assert.ok(v.heightAt(3.5, 3.5) > a.heightAt(3.5, 3.5) + 0.3, "volcano still has its cone");
});

test("a range's peaks are joined by ridges: each link once, none across a block, none to a volcano, a held peak or the unknown", () => {
  // North first: a 2x2 block of peaks with a tail, a diagonal pair, a volcano beside the range, a
  // peak a unit stands on, and a peak beside the unknown.
  const rows = ["..........", ".^^.^v....", ".^^^......", "....^.^...", ".......^..", ".^^^......"];
  // (A unit stands on (1, 0); (3, 0) is unknown.)
  const hidden = [[3, 0]];
  const map = buildMap(data, rows, { hidden });
  St.setMap(map);
  St.setView(buildView(data, { players: [{ civ: "CIVILIZATION_BANNOR" }], units: [{ type: "UNIT_WARRIOR", x: 1, y: 0 }] }));
  for (const cliff of [0, 0.18]) {
    const world = new World(map, { cliff });
    const list = dressStatic(world, map).list;
    const ridges = list.filter((it) => it.asset === "feat.ridge");
    const at = (it) => [it.x, -it.z];
    const peaks = list.filter((it) => it.asset === "feat.mountain");
    // A ridge lies between the two mountains it joins: name each end by the mountain nearest it.
    const links = new Set();
    for (const it of ridges) {
      const [x, y] = at(it), dir = [Math.cos(it.yaw), Math.sin(it.yaw)];
      const half = (it.variant < 4 ? 0.5 : Math.SQRT1_2) * it.sx;
      const ends = [1, -1].map((s) => {
        const e = [x + dir[0] * half * s, y + dir[1] * half * s];
        const m = peaks.reduce((b, p) => (Math.hypot(p.x - e[0], -p.z - e[1]) < Math.hypot(b.x - e[0], -b.z - e[1]) ? p : b));
        assert.ok(Math.hypot(m.x - e[0], -m.z - e[1]) < 1e-6, "a ridge ends at a mountain's place");
        return m.plot;
      });
      const key = ends.sort((a, b) => a - b).join("-");
      assert.ok(!links.has(key), `the link ${key} is laid once`);
      links.add(key);
    }
    const P = (x, y) => y * map.w + x;
    const want = [
      [P(1, 4), P(2, 4)], [P(1, 3), P(2, 3)], [P(2, 3), P(3, 3)], [P(1, 3), P(1, 4)], [P(2, 3), P(2, 4)],
      // Diagonal neighbours with no peak between them.
      [P(3, 3), P(4, 2)], [P(3, 3), P(4, 4)], [P(6, 2), P(7, 1)],
    ].map((l) => l.sort((a, b) => a - b).join("-"));
    assert.deepEqual([...links].sort(), want.sort(), "the links of the range");
    // A ridge is rooted as a mountain is: its open rim nowhere over the ground drawn.
    const surfaces = [terrainHeight(world, RES), terrainHeight(world, LOW_RES)];
    for (const it of ridges) {
      for (const mesh of [buildAsset(it.asset, it.variant, it.param), buildAssetLOD(it.asset, it.variant, it.param)].filter(Boolean)) {
        for (const [x, y, z] of rim(mesh)) {
          const mx = it.x + Math.cos(it.yaw) * x * it.sx + Math.sin(it.yaw) * z * it.sz;
          const my = -it.z + Math.sin(it.yaw) * x * it.sx - Math.cos(it.yaw) * z * it.sz;
          const ground = Math.max(0, Math.min(world.heightAt(mx, my), ...surfaces.map((h) => h(mx, my))));
          assert.ok(it.y + y * it.sy - ground <= 0.002, `a ridge's rim floats ${(it.y + y * it.sy - ground).toFixed(4)} over the ground (plot ${it.plot}, cliff ${cliff})`);
        }
      }
    }
  }
});
