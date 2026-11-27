// node --test: Cataclysm tooltips (cataclysm_tips.js): keyword extraction from `[KW=CAT_...]`
// markup, transitive `with` expansion showing all referenced cards once in order of first mention,
// unit action/trait keyword extraction, and Reinforce precondition checks.
import test from "node:test";
import assert from "node:assert/strict";
import { extractCatReminders, reinforceWhy, unitActionKeywords } from "../app/cataclysm_tips.js";

const DICT = {
  CAT_RAID: { key: "CAT_RAID", name: "Raid", text: "Raid rule.", with: ["CAT_RAID", "CAT_SALLY", "CAT_STRIPPED", "CAT_SUPPLY"] },
  CAT_SALLY: { key: "CAT_SALLY", name: "Sally", text: "Sally rule.", with: ["CAT_SALLY", "CAT_RAID", "CAT_STRIPPED", "CAT_SUPPLY"] },
  CAT_STRIPPED: { key: "CAT_STRIPPED", name: "Stripped", text: "Stripped rule.", with: ["CAT_STRIPPED", "CAT_RAID"] },
  CAT_SUPPLY: { key: "CAT_SUPPLY", name: "Supply", text: "Supply rule.", with: ["CAT_SUPPLY"] },
  CAT_FORMATION: { key: "CAT_FORMATION", name: "Formation", text: "Formation rule.", with: ["CAT_FORMATION"] },
  CAT_CHARGE: { key: "CAT_CHARGE", name: "Charge", text: "Charge rule.", with: ["CAT_CHARGE", "CAT_STRIKE"] },
  CAT_STRIKE: { key: "CAT_STRIKE", name: "Strike", text: "Strike rule.", with: ["CAT_STRIKE"] },
  CAT_TRAMPLE: { key: "CAT_TRAMPLE", name: "Trample", text: "Trample rule.", with: ["CAT_TRAMPLE"] },
  CAT_LARGE: { key: "CAT_LARGE", name: "Large", text: "Large rule.", with: ["CAT_LARGE"] },
  CAT_ARMOR_PIERCING: { key: "CAT_ARMOR_PIERCING", name: "Armor-Piercing", text: "AP rule.", with: ["CAT_ARMOR_PIERCING"] },
};

test("extractCatReminders expands all referenced cards once in order of first mention", () => {
  const got = extractCatReminders(["Begins a [KW=CAT_RAID]Raid[\\KW] on this plot."], [], DICT);
  assert.deepEqual(got.map((r) => r.key), ["CAT_RAID", "CAT_SALLY", "CAT_STRIPPED", "CAT_SUPPLY"]);
});

test("extractCatReminders deduplicates repeated marks and extra keys safely", () => {
  const got = extractCatReminders(
    ["Uses [KW=CAT_FORMATION]Formation[\\KW] and [KW=CAT_CHARGE]Charge[\\KW] with [KW=CAT_STRIKE]Strike[\\KW]."],
    ["CAT_CHARGE", "CAT_TRAMPLE", "CAT_UNKNOWN"],
    DICT,
  );
  assert.deepEqual(got.map((r) => r.key), ["CAT_FORMATION", "CAT_CHARGE", "CAT_STRIKE", "CAT_TRAMPLE"]);
});

test("unitActionKeywords collects unit traits, actions, and non-single patterns", () => {
  const knight = {
    large: false,
    anti_flank: false,
    armor_piercing: true,
    damage_vs_walls: 0,
    actions: [
      { kind: "Charge", n: 2, pattern: "Trample" },
      { kind: "Strike", n: 1, pattern: "Single" },
      { kind: "None", n: 0, pattern: "Single" },
    ],
  };
  assert.deepEqual(unitActionKeywords(knight), ["CAT_ARMOR_PIERCING", "CAT_CHARGE", "CAT_TRAMPLE", "CAT_STRIKE"]);
});

test("reinforceWhy checks borders, free places, 2-cell fit, and Gold", () => {
  const data = { currencies: { gold: 90 } };
  const guard = { name: "Guard", cost_gold: 30, large: false };
  const ogre = { name: "Ogre", cost_gold: 120, large: true };
  const sqOk = { in_home_borders: true, places_used: 2, place_cap: 4, can_fit_1cell: true, can_fit_2cell: true };

  assert.equal(reinforceWhy(data, sqOk, guard), null);
  assert.match(reinforceWhy(data, { ...sqOk, in_home_borders: false }, guard), /inside your borders/i);
  assert.match(reinforceWhy(data, { ...sqOk, places_used: 3, place_cap: 4 }, ogre), /2 free places/i);
  assert.match(reinforceWhy(data, { ...sqOk, can_fit_2cell: false }, ogre), /adjacent free cells/i);
  assert.match(reinforceWhy(data, sqOk, ogre), /Need 120 Gold \(have 90\)/);
});
