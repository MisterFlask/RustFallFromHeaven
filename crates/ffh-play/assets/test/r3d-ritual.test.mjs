// node --test: the ritual effect's looks and zoom fade (app/r3d/ritual.js, docs/R3D.md §5).
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { RITUAL_LOOKS, RITUAL_LOOK_OF, ritualLook, zoomFade, ritualCities, ritualEmitters } from "../app/r3d/ritual.js";

const projects = JSON.parse(
  fs.readFileSync(new URL("../../../../data/json/projects.jsonc", import.meta.url), "utf8").replace(/^\s*\/\/.*$/gm, ""),
);

test("every project has a look, and every look named exists", () => {
  const types = (Array.isArray(projects) ? projects : Object.values(projects).find(Array.isArray)).map((p) => p.type);
  assert.ok(types.length > 20);
  for (const t of types) assert.ok(RITUAL_LOOK_OF[t], `${t} has no look`);
  for (const [t, k] of Object.entries(RITUAL_LOOK_OF)) assert.ok(RITUAL_LOOKS[k], `${t}: no look ${k}`);
  assert.equal(ritualLook("PROJECT_NONE"), RITUAL_LOOKS.arcane);
  assert.equal(ritualLook(undefined), RITUAL_LOOKS.arcane);
});

test("each look is well formed", () => {
  const hex = /^#[0-9a-f]{6}$/;
  for (const [k, L] of Object.entries(RITUAL_LOOKS)) {
    for (const c of ["core", "halo", "glow", "dark", "mid", "top", "shade"]) assert.match(L[c], hex, `${k}.${c}`);
    assert.ok(Number.isInteger(L.arms) && L.arms >= 2, `${k}.arms`);
    assert.ok(L.spin !== 0, `${k}.spin`);
    assert.ok(L.motes.length > 0, `${k}.motes`);
    for (const m of L.motes) assert.match(m[0], hex, `${k} mote`);
  }
});

test("the storm fades as the camera rises", () => {
  const near = zoomFade(220), mid = zoomFade(96), far = zoomFade(28);
  assert.deepEqual(near, [1, 1, 1]);
  for (let i = 0; i < 3; i++) assert.ok(far[i] < mid[i] && mid[i] < near[i]);
  assert.ok(far[1] < 0.1, "the storm is nearly gone far out");
  assert.ok(mid[1] < 0.6, "a veil at the gameplay zoom");
  assert.ok(far[2] >= 0.5, "the beam stays the ritual's mark");
});

test("a city's look follows its ritual's type, and its motes the look", () => {
  const cities = [{ x: 3.5, y: 0, z: -2.5, city: { id: 7, ritual: "Ritual: Raging Seas", ritual_type: "PROJECT_RITUAL_RAGING_SEAS" } }];
  const list = ritualCities(cities, new Map(), 10);
  assert.equal(list.length, 1);
  assert.equal(list[0].look, RITUAL_LOOKS.sea);
  const em = ritualEmitters(list);
  assert.equal(em.length, RITUAL_LOOKS.sea.motes.length);
  assert.ok(em.some((e) => e.smoke));
});
