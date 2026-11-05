// node --test: the isometric renderer's lookups (app/iso.js) name only sprites that exist
// (app/iso/sprites.js, written by `tools/m7/m7.py export`), for every terrain, feature,
// improvement and bonus of the game data (data/json), so a typo or a missing export shows here rather than as
// a silent fallback on the map.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { ISO_TABLES } from "../app/iso.js";
import { SPRITES } from "../app/iso/sprites.js";

const DATA = new URL("../../../../data/json/", import.meta.url);

function types(file) {
  // Each entry of a table's file starts with its "type".
  return [...fs.readFileSync(new URL(file, DATA), "utf8").matchAll(/^ {4}"type": "([A-Z0-9_]+)"/gm)].map((m) => m[1]);
}

function missing(keys) {
  return [...new Set(keys)].filter((k) => !Object.hasOwn(SPRITES, k));
}

test("every terrain and feature sprite the renderer names exists", () => {
  const T = ISO_TABLES;
  assert.deepEqual(missing(Object.values(T.TERRAIN_TILE)), []);
  assert.deepEqual(missing(Object.values(T.FEATURE_SPRITES).flat()), []);
  assert.deepEqual(missing(["ovl_hills", "ovl_hills_dry", "ovl_hills_snow", "feat_mountain", "feat_mountain_hell", "feat_volcano"]), []);
  for (const t of types("features.jsonc")) {
    if (/DOOR|WALLS|FALLOUT|BLIZZARD|OBSIDIAN|VOLCANO/.test(t)) continue; // drawn as their tile alone, or the volcano (above)
    assert.ok(T.FEATURE_SPRITES[t], `no sprite for ${t}`);
  }
});

test("every improvement and bonus maps to sprites that exist", () => {
  const T = ISO_TABLES;
  for (const t of types("improvements.jsonc")) assert.deepEqual(missing(T.improvementSprites(t)), [], t);
  for (const t of types("bonuses.jsonc")) assert.deepEqual(missing([T.resourceSprite(t)]), [], t);
  for (const size of [1, 4, 9, 20]) assert.deepEqual(missing(T.citySprites(size)), []);
});

test("a city takes its civilization's settlement for its size first, then the shared one", () => {
  const T = ISO_TABLES;
  assert.deepEqual(T.citySprites(5, "khazad").slice(0, 4), ["bld_city_khazad_town", "bld_city_town", "bld_city_khazad_village", "bld_city_village"]);
  assert.deepEqual(T.citySprites(12).slice(0, 2), ["bld_city_castle", "bld_city_town"]);
});

test("every unit stand-in exists", () => {
  const T = ISO_TABLES;
  const names = [...Object.values(T.UNIT_ALIASES).flat(), ...Object.values(T.COMBAT_SPRITES), ...Object.values(T.DOMAIN_SPRITES), "swordsman", "settler", "worker"];
  assert.deepEqual(missing(names.map((n) => `unit_${n}`)), []);
});
