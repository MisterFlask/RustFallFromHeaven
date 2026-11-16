// node --test: every leader a game can have (each civilization's `leaders` in the game data,
// the barbarian and the random choices included) has a portrait (app/portraits.js, painted by
// tools/m7/leaders.py and exported with the isometric sprites by `tools/m7/m7.py export`).
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { portraitKey } from "../app/portraits.js";
import { SPRITES } from "../app/iso/sprites.js";

const DATA = new URL("../../../../data/json/", import.meta.url);

test("every playable leader has a portrait", () => {
  const civs = fs.readFileSync(new URL("civilizations.jsonc", DATA), "utf8");
  const lists = [...civs.matchAll(/"leaders": \{([^}]*)\}/g)].map((m) => m[1]);
  const leaders = [...new Set(lists.flatMap((l) => [...l.matchAll(/"([A-Z0-9_-]+)": true/g)].map((m) => m[1])))];
  assert.ok(leaders.length > 30, `${leaders.length} leaders read`);
  assert.deepEqual(leaders.filter((l) => !portraitKey(l)), [], "leaders without a portrait");
  const png = (k) => fs.existsSync(new URL(`../app/iso/${k}.png`, import.meta.url));
  assert.deepEqual(leaders.filter((l) => !png(portraitKey(l))), [], "portraits listed but not exported");
});

test("a leader type's portrait key", () => {
  assert.equal(portraitKey("LEADER_OS-GABELLA"), "leader_os_gabella");
  assert.equal(portraitKey("LEADER_RANDOM_EVIL"), "leader_random");
  assert.equal(portraitKey("LEADER_NOBODY"), null);
  assert.equal(portraitKey(null), null);
});

test("every preferred breakaway leader has its own full portrait and face crop", () => {
  const text = fs.readFileSync(new URL("cards/zz_minor_leaders.jsonc", DATA), "utf8");
  const data = JSON.parse(text.replace(/^\s*\/\/.*$/gm, ""));
  const leaders = data.leader_heads.filter((leader) => leader.preferred_breakaway);
  assert.equal(leaders.length, 105);
  const keys = new Set();
  for (const leader of leaders) {
    const key = `leader_${leader.type.replace(/^LEADER_/, "").toLowerCase()}`;
    assert.equal(portraitKey(leader.type), key, `${leader.type} has its own portrait`);
    assert.ok(!keys.has(key), `${leader.type} has a unique portrait key`);
    keys.add(key);
    for (const [file, size] of [[key, 384], [`${key}_face`, 160]]) {
      assert.ok(Object.hasOwn(SPRITES, file), `${file} is registered`);
      const png = fs.readFileSync(new URL(`../app/iso/${file}.png`, import.meta.url));
      assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
      assert.equal(png.readUInt32BE(16), size, `${file} width`);
      assert.equal(png.readUInt32BE(20), size, `${file} height`);
    }
  }
});
