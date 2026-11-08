// node --test: the lair and goody hut odds as text (app/explore.js).
import test from "node:test";
import assert from "node:assert/strict";
import { pct, tierLine, exploreShort, exploreFull } from "../app/explore.js";
import { plain } from "../app/markup.js";

const dname = (kind, i) => `${kind}#${i}`;

test("chances in parts per million", () => {
  assert.equal(pct(0), "0%");
  assert.equal(pct(130000), "13%");
  assert.equal(pct(33333), "3.3%");
  assert.equal(pct(500), "<0.1%");
  assert.equal(pct(999990), ">99.9%");
  assert.equal(pct(1000000), "100%");
});

const lair = {
  kind: "lair", epic: false, in_view: true, unit: 7, unit_type: 3, level: 2,
  tiers: [{ tier: "big_bad", ppm: 120000 }, { tier: "bad", ppm: 300000 }, { tier: "neutral", ppm: 300000 }, { tier: "good", ppm: 200000 }, { tier: "big_good", ppm: 80000 }],
  lair_survives_ppm: 400000, grace_ppm: 1000000,
  outcomes: [
    { tier: "big_bad", key: "MONSTER", label: "Barbarian monster", ppm: 120000, destroy_pct: 0, replaces_lair: false },
    { tier: "bad", key: "COLLAPSE", label: "Collapse: 50% damage", ppm: 30000, destroy_pct: 100, replaces_lair: false },
    { tier: "neutral", key: "CAGE", label: "Cage", ppm: 25000, destroy_pct: 0, replaces_lair: true },
    { tier: "good", key: "HIGH_GOLD", label: "Gold: 20-120", ppm: 20000, destroy_pct: 90, replaces_lair: false },
  ],
  big_bad: { monsters: ["Azer", "Ogre"], henchmen: ["Wolf"], promotions: ["Strong"] },
  others: [{ unit: 8, unit_type: 4, level: 5, tiers: [{ tier: "big_bad", ppm: 90000 }, { tier: "bad", ppm: 300000 }, { tier: "neutral", ppm: 300000 }, { tier: "good", ppm: 200000 }, { tier: "big_good", ppm: 110000 }] }],
};

test("the lair tooltip names the unit, the tiers and the lair's fate", () => {
  const t = plain(exploreShort(lair, dname));
  assert.match(t, /Explore Lair \(UnitType#3, level 2\)/);
  assert.match(t, /Big bad 12% · Bad 30% · Neutral 30% · Good 20% · Big good 8.0%/);
  assert.match(t, /Lair remains: 40%/);
  assert.match(t, /then UnitType#4, level 5: Big bad 9.0%/);
  assert.equal(tierLine([{ tier: "bad", ppm: 500000 }]), "[COLOR_NEGATIVE_TEXT]Bad 50%[COLOR_REVERT]");
});

test("the lair table lists each outcome with what becomes of the lair", () => {
  const t = plain(exploreFull(lair, dname));
  assert.match(t, /3.0% Collapse: 50% damage \(lair destroyed\)/);
  assert.match(t, /2.5% Cage \(lair replaced\)/);
  assert.match(t, /2.0% Gold: 20-120 \(lair destroyed 90%\)/);
  assert.match(t, /Monster: one of Azer, Ogre\./);
  assert.match(t, /Grace period: 100%/);
});

test("a fogged lair or no unit: the tiers alone", () => {
  const fogged = { kind: "lair", epic: true, in_view: false, unit: 7, unit_type: 3, level: 1, tiers: [{ tier: "big_bad", ppm: 530000 }, { tier: "big_good", ppm: 470000 }] };
  const t = plain(exploreFull(fogged, dname));
  assert.match(t, /Explore Lair \(epic\)/);
  assert.match(t, /The outcomes show while the plot is in view/);
  const none = plain(exploreShort({ ...fogged, unit: null }, dname));
  assert.match(none, /level 1; select a unit/);
});

test("goody hut: allowed outcomes by chance, refused ones greyed in the table", () => {
  const hut = {
    kind: "hut", unit: 2, unit_type: 5, slots_total: 20, attempts: 10, nothing_ppm: 0,
    outcomes: [
      { goody: 0, label: "Gold: 20-60", ppm: 450000, slots: 4, allowed: true },
      { goody: 1, label: "Heals 100%", ppm: 0, slots: 1, allowed: false },
      { goody: 2, label: "Map", ppm: 550000, slots: 5, allowed: true },
    ],
  };
  const s = plain(exploreShort(hut, dname));
  assert.match(s, /Goody hut \(UnitType#5 enters\)/);
  assert.ok(s.indexOf("55% Map") < s.indexOf("45% Gold"), s);
  assert.doesNotMatch(s, /Heals/);
  const f = plain(exploreFull(hut, dname));
  assert.match(f, /0% Heals 100% \(1\/20, not possible now\)/);
  assert.match(f, /10 draws from 20 slots/);
});
