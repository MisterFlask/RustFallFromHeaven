// node --test: every leader a game can have (each civilization's <LeaderName>s in the game's XML,
// the barbarian and the random choices included) has a portrait (app/portraits.js, painted by
// tools/m7/leaders.py and exported with the isometric sprites by `tools/m7/m7.py export`).
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { portraitKey } from "../app/portraits.js";

const XML = new URL("../../../../data/xml/Civilizations/", import.meta.url);

test("every playable leader has a portrait", () => {
  const civs = fs.readFileSync(new URL("CIV4CivilizationInfos.xml", XML), "utf8");
  const leaders = [...new Set([...civs.matchAll(/<LeaderName>([A-Z0-9_-]+)<\/LeaderName>/g)].map((m) => m[1]))];
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
