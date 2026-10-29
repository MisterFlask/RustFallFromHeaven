// node --test: the HD-2D mode's pure parts (app/r3d/hd2d, docs/HD2D.md) on the lab's scenes and the
// real type tables: what stands as a card where, the unit groups and their banners, the per-plot mood
// channel -- deterministic and a function of what the player knows. No WebGL here.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import * as St from "../app/state.js";

const data = JSON.parse(fs.readFileSync(fileURLToPath(new URL("../../r3d-lab/data.json", import.meta.url)), "utf8"));
St.setData(data);
const { buildMap, buildView, hd2d } = await import("../../r3d-lab/scenes.js");
const { World, KIND } = await import("../app/r3d/world.js");
await import("../app/r3d/assets/index.js");
const { SPRITES } = await import("../app/iso/sprites.js");
const { unitSpriteKeys } = await import("../app/iso.js");
const { dressSpriteStatic, dressSpriteUnits, PX } = await import("../app/r3d/hd2d/dress.js");
const { plotMoods } = await import("../app/r3d/hd2d/mood.js");

/** The atlas as the dressing sees it (hd2d/sprites.js SpriteAtlas): the PNG sprites and the banners,
 *  which are drawn by code in the browser. */
const atlas = {
  has: (k) => Object.hasOwn(SPRITES, k) || k.startsWith("banner:"),
  info: (k) => SPRITES[k] ?? { w: 40, h: 150, box: [0, 0, 40, 150] },
  put() {},
};

function scene(s) {
  St.setView(s.view);
  St.setMap(s.map);
  return new World(s.map);
}

test("every unit type of the game stands as a sprite of the isometric renderer's", () => {
  for (let t = 0; t < data.tables.UnitType.length; t++) {
    const key = unitSpriteKeys(t, null).find((k) => Object.hasOwn(SPRITES, k));
    assert.ok(key, `${data.tables.UnitType[t].type} has no sprite`);
  }
});

test("a stack is a group of one to three figures of its head unit under its owner's banner", () => {
  const s = hd2d(data);
  const w = scene(s);
  const { list, stacks } = dressSpriteUnits(w, s.map, atlas);
  assert.equal(stacks.length, St.S.unitsAt.size);
  for (const st of stacks) {
    const mine = list.filter((r) => r.uid?.startsWith(`${st.top.id}:`));
    const figures = mine.filter((r) => !r.banner);
    assert.ok(figures.length >= 1 && figures.length <= 3, `${figures.length} figures`);
    assert.equal(new Set(figures.map((r) => r.key)).size, 1, "one unit's sprite");
    assert.equal(mine.filter((r) => r.banner).length, 1, "one banner");
    for (const r of figures) assert.ok(r.shadow && r.bias > 0, "figures cast shadows and stand before the buildings");
  }
  // The same state, the same cards.
  assert.deepEqual(dressSpriteUnits(w, s.map, atlas).list, list);
});

test("cities, standing improvements and resources stand as cards, and the 3D dressing leaves them", () => {
  const s = hd2d(data);
  const w = scene(s);
  const { list, claims } = dressSpriteStatic(w, s.map, atlas);
  assert.equal(claims.city.size, St.S.cities.size);
  for (const r of list) assert.ok(atlas.has(r.key), r.key);
  const cities = list.filter((r) => r.key.startsWith("bld_city"));
  assert.ok(cities.some((r) => r.key === "bld_city_ljosalfar_castle"), "the Ljosalfar capital is its civilization's castle");
  for (const r of cities) assert.ok(Math.abs(r.w - SPRITES[r.key].w * PX.city) < 1e-9);
});

test("nothing stands on, and no mood comes from, what the player has not seen", () => {
  const rows = ["..,,..", "..ff..", "xx..pp", "......"];
  const hidden = [[1, 1], [2, 1], [4, 1]];
  const map = buildMap(data, rows, {
    hidden,
    owners: [[0, 0, 5, 3, 1]],
    improvements: [[2, 1, "IMPROVEMENT_TOWER"], [4, 2, "IMPROVEMENT_TOWER"]],
    bonuses: [[1, 1, "BONUS_GOLD"]],
  });
  for (const [x, y] of hidden) map.owner[y * map.w + x] = -1;
  const view = buildView(data, { players: [{ civ: "CIVILIZATION_BANNOR" }, { civ: "CIVILIZATION_SHEAIM" }] });
  const w = scene({ map, view });
  const { list } = dressSpriteStatic(w, map, atlas);
  for (const r of list) {
    const i = Math.floor(-r.z - 0.01 + 0.5) * map.w + Math.floor(r.x);
    assert.notEqual(w.kind[Math.max(0, i)], KIND.VOID, `${r.key} on an unrevealed plot`);
  }
  assert.equal(list.filter((r) => r.key === "bld_watchtower").length, 1, "only the seen tower");
  const { data: mood } = plotMoods(w, map);
  for (const [x, y] of hidden) assert.deepEqual([...mood.slice((y * map.w + x) * 4, (y * map.w + x) * 4 + 4)], [0, 0, 0, 0]);
  // The Sheaim's land is corrupted, hell terrain is hell.
  assert.equal(mood[(3 * map.w + 0) * 4 + 1], 255);
  const hell = 1 * map.w + 0;
  assert.equal(mood[hell * 4], 255);
});
