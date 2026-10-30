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
const { buildMap, buildView, hd2d, settlements } = await import("../../r3d-lab/scenes.js");
const { World, KIND } = await import("../app/r3d/world.js");
await import("../app/r3d/assets/index.js");
const { SPRITES } = await import("../app/iso/sprites.js");
const { CARDS, GROUNDS } = await import("../app/r3d/hd2d/art/index.js");
const { unitSpriteKeys } = await import("../app/iso.js");
const { dressSpriteStatic, dressSpriteUnits, PX } = await import("../app/r3d/hd2d/dress.js");
const { plotMoods } = await import("../app/r3d/hd2d/mood.js");

/** The atlas as the dressing sees it (hd2d/sprites.js SpriteAtlas): the PNG sprites, their owners'
 *  variants (`key@css`, recoloured as they load) and the banners, which are drawn by code in the
 *  browser. */
const plain = (k) => k.split("@")[0];
const atlas = {
  has: (k) => Object.hasOwn(SPRITES, plain(k)) || Object.hasOwn(CARDS, plain(k)) || k.startsWith("banner:"),
  info: (k) => SPRITES[plain(k)] ?? CARDS[plain(k)] ?? { w: 40, h: 150, box: [0, 0, 40, 150] },
  owned: (k, css) => `${k}@${css}`,
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
  // A city is a town of the kit's cards (docs/HD2D.md §5c), not the isometric settlement.
  assert.ok(!list.some((r) => r.key.startsWith("bld_city")), "a city stood as the isometric sprite");
  const town = list.filter((r) => plain(r.key).startsWith("town_"));
  assert.ok(town.some((r) => plain(r.key) === "town_lm_ljosalfar"), "the Ljosalfar capital has its landmark");
  assert.ok(town.some((r) => plain(r.key).startsWith("town_elven_house_")), "and its family's houses");
  for (const r of town) {
    // At the prop cards' density (PX.prop), within the props' own spread of scales (a house a
    // little larger or smaller, a village's landmark smaller and a great city's larger).
    const k = r.w / (CARDS[plain(r.key)].w * PX.prop);
    assert.ok(k > 0.77 && k < 1.19, `${r.key} at ${k} of the props' density`);
    assert.equal(r.key.includes("@"), !!CARDS[plain(r.key)].band, "a town's pieces with the owner-colour band take their owner's colour");
    assert.ok(r.town && r.shadow, "a town's pieces are lit as towns and cast shadows");
  }
});

test("towns grow with their size, walled cities have walls, a gate and towers, deterministic", () => {
  const civs = ["CIVILIZATION_CALABIM", "CIVILIZATION_LJOSALFAR", "CIVILIZATION_KHAZAD"];
  const s = settlements(data, civs, [2, 6, 12], [], { walls: 8 });
  const w = scene(s);
  const env = { unitAt: new Set(), cityAt: new Map() };
  const { list } = dressSpriteStatic(w, s.map, atlas, env, null);
  const pieces = (c) => list.filter((r) => r.id && plain(r.key).startsWith("town_") && Math.hypot(r.x - c.x - 0.5, -r.z - c.y - 0.5) < 1.1 && r.vis?.[0] === c.x + 0.5 && r.vis?.[1] === c.y + 0.5);
  for (const civ of civs) {
    const short = civ.replace("CIVILIZATION_", "").toLowerCase();
    const mine = [...St.S.cities.values()].filter((c) => St.S.players.get(c.owner)?.civ === data.tables.CivilizationType.findIndex((t) => t.type === civ)).sort((a, b) => a.size - b.size);
    assert.equal(mine.length, 3, civ);
    const counts = mine.map((c) => pieces(c).filter((r) => /_house_/.test(plain(r.key))).length);
    assert.ok(counts[0] >= 3 && counts[0] < counts[1] && counts[1] < counts[2], `${short}: houses by size ${counts}`);
    for (const c of mine) {
      const p = pieces(c).map((r) => plain(r.key));
      assert.ok(p.includes(`town_lm_${short}`), `${short} ${c.size}: its landmark`);
      const walls = p.filter((k) => /_wall_/.test(k)).length;
      if (c.walls) {
        assert.ok(walls >= 6 && p.some((k) => /_gate_/.test(k)) && p.filter((k) => /_corner_/.test(k)).length >= 4, `${short} ${c.size}: walls ${walls}`);
      } else assert.equal(walls, 0, `${short} ${c.size}: no walls`);
    }
  }
  assert.deepEqual(dressSpriteStatic(w, s.map, atlas, env, null).list, list, "the same state, the same town");
});

test("towns stand out: larger, on a clearing, under their owner's banners, as the panel asks", () => {
  const civs = ["CIVILIZATION_KHAZAD"];
  const s = settlements(data, civs, [12], [], { walls: 8 });
  const w = scene(s);
  const plainEnv = { unitAt: new Set(), cityAt: new Map() };
  const env = { ...plainEnv, town: { scale: 1.25, clearing: 1, banners: 1 } };
  const plainList = dressSpriteStatic(w, s.map, atlas, plainEnv, null).list;
  const { list } = dressSpriteStatic(w, s.map, atlas, env, null);
  const [c] = St.S.cities.values();
  const lm = (l) => l.find((r) => plain(r.key) === "town_lm_khazad");
  assert.ok(Math.abs(lm(list).w / lm(plainList).w - 1.25) < 1e-9, "a larger town's landmark is larger by the town's scale");
  // A banner flies over the landmark (beside the city's own at its front-left).
  const flags = (l) => l.filter((r) => r.banner && Math.hypot(r.x - c.x - 0.5, -r.z - c.y - 0.5) < 1.2).length;
  assert.ok(flags(list) >= flags(plainList) + 2, `banners over the town: ${flags(list)} against ${flags(plainList)}`);
  // On a clearing, nothing tall (trees, bushes, rocks) stands near it, and fields lie round it.
  const near = (r) => Math.hypot(r.x - c.x - 0.5, -r.z - c.y - 0.5) < 1.3;
  const tall = (l) => l.filter((r) => near(r) && /^(tree|bush|rock)_/.test(r.key)).length;
  assert.ok(tall(list) < tall(plainList) || tall(plainList) === 0, `tall props near the town: ${tall(list)} against ${tall(plainList)}`);
  assert.ok(list.filter((r) => /^(garden|hay)_/.test(r.key)).length > plainList.filter((r) => /^(garden|hay)_/.test(r.key)).length, "fields round a town on a clearing");
  assert.deepEqual(dressSpriteStatic(w, s.map, atlas, env, null).list, list, "the same state, the same town");
});

test("woods stand as single tree cards and open ground as scattered props, claimed from the 3D dressing", () => {
  assert.ok(GROUNDS.includes("gnd_grass") && Object.keys(CARDS).some((k) => k.startsWith("tree_oak_")), "the mode's art is there");
  const rows = ["ffff..", "ffff..", "aa....", "......"];
  const map = buildMap(data, rows, { hidden: [[5, 0]] });
  const view = buildView(data, { players: [{ civ: "CIVILIZATION_BANNOR" }] });
  const w = scene({ map, view });
  const env = { unitAt: new Set(), cityAt: new Map() };
  const { list, claims } = dressSpriteStatic(w, map, atlas, env, null);
  assert.ok(claims.trees && claims.flora && claims.rocks);
  const at = (r) => [Math.floor(r.x), Math.floor(-r.z)];
  const wooded = list.filter((r) => r.key.startsWith("tree_"));
  for (let y = 2; y < 4; y++) {
    for (let x = 0; x < 4; x++) {
      const n = wooded.filter((r) => at(r)[0] === x && at(r)[1] === y).length;
      assert.ok(n >= 5, `a wooded plot (${x}, ${y}) holds a stand of trees, not ${n}`);
    }
  }
  assert.ok(list.some((r) => r.key.startsWith("tree_ancient_")), "an ancient wood has its giants");
  for (const r of list) assert.ok(atlas.has(r.key), r.key);
  // The hidden plot (5, 0) holds nothing.
  assert.ok(!list.some((r) => at(r)[0] === 5 && at(r)[1] === 0), "a card on an unrevealed plot");
  assert.deepEqual(dressSpriteStatic(w, map, atlas, env, null).list, list, "the same state, the same cards");
});

test("shores are rimmed with stones and open ground is filled with small props in clumps", () => {
  const rows = ["~~~~~~", "~~~~~~", "......", "......", "......"];
  const map = buildMap(data, rows);
  const view = buildView(data, { players: [{ civ: "CIVILIZATION_BANNOR" }] });
  const w = scene({ map, view });
  const env = { unitAt: new Set(), cityAt: new Map() };
  const { list } = dressSpriteStatic(w, map, atlas, env, null);
  const shore = list.filter((r) => r.key.startsWith("shore_"));
  // Along the coast (the land's last row, y 2 -- the rows are listed north first -- and the warped
  // waterline about it), not inland.
  assert.ok(shore.length >= 8, `a coast six plots long has a rim of stones, not ${shore.length}`);
  for (const r of shore) assert.ok(-r.z > 1.6 && -r.z < 3.5, `a shore stone inland at ${-r.z}`);
  for (const r of shore) assert.ok(Math.abs(r.y - Math.max(0, w.heightAt(r.x, -r.z))) < 0.03, "a shore stone at the waterline");
  // Open grassland carries grass tufts, on every inland plot.
  for (let x = 0; x < 6; x++) {
    const n = list.filter((r) => r.key.startsWith("grass_") && Math.floor(r.x) === x && Math.floor(-r.z) === 0).length;
    assert.ok(n >= 2, `grass tufts on (${x}, 0): ${n}`);
  }
  assert.deepEqual(dressSpriteStatic(w, map, atlas, env, null).list, list, "the same state, the same cards");
});

test("towns spill into houses round them, regions carry their own things, rivers fall off the hills", () => {
  const s = hd2d(data);
  St.setView(s.view);
  St.setMap(s.map);
  const w = new World(s.map, { bank: 0.04 });
  const { moods } = plotMoods(w, s.map);
  const { list } = dressSpriteStatic(w, s.map, atlas, { unitAt: new Set(), cityAt: new Map() }, moods);
  const n = (kind) => list.filter((r) => r.key.startsWith(`${kind}_`)).length;
  // The houses round a town are its own family's (the lab's towns' families all have the kit).
  const ring = list.filter((r) => r.id === 0.0002 && /^town_\w+_house_/.test(plain(r.key)));
  assert.ok(ring.length >= 6, `houses round the towns: ${ring.length}`);
  assert.ok(n("obelisk") + n("altar") >= 1, "the Sheaim's corrupted land has its obelisks and altars");
  assert.ok(n("lantern") >= 1, "the Ljosalfar's lush land has its lanterns");
  assert.ok(n("bones") + n("vent") >= 4, "hell has its bones and vents");
  assert.ok(n("falls") >= 1, "the lab's river falls off its hills");
  // Houses only round towns: within two plots of a city.
  const cities = [...St.S.cities.values()];
  for (const r of list.filter((r) => r.key.startsWith("hut") || /^town_\w+_house_/.test(plain(r.key)))) {
    assert.ok(cities.some((c) => Math.abs(Math.floor(r.x) - c.x) <= 2 && Math.abs(Math.floor(-r.z) - c.y) <= 2), `a house far from any town at ${r.x}, ${-r.z}`);
  }
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
