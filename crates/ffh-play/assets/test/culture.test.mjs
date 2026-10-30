// node --test: the cultural channels' numbers and words (app/culture.js): hundredths of culture
// and of a percent for people, how far revolt pressure goes, where it settles and when it gets
// there, as the rules run a turn (docs/CULTURE.md).
import test from "node:test";
import assert from "node:assert/strict";
import { pts, chancePct, pressureStage, unrestOf, outcome, settleOf, turnsTo, turnsBelow, nextStage } from "../app/culture.js";
import { revoltMark } from "../app/iso.js";

// The defaults of data/xml/RulesDefines.xml, as the server's `thresholds` sends them.
const th = { unhappy_step: 2500, unhappy_max: 4, disorder: 10000, near_flip: 20000, flip: 30000, fade_percent: 5, resist_percent: 50, garrison: 200, spread_max: 2500, flipping: true };

test("hundredths of culture and of a percent", () => {
  assert.equal(pts(0), "0");
  assert.equal(pts(306), "3.1");
  assert.equal(pts(300), "3");
  assert.equal(pts(4210), "42");
  assert.equal(pts(996), "10");
  assert.equal(chancePct(2500), "25%");
  assert.equal(chancePct(1250), "13%");
  assert.equal(chancePct(260), "2.6%");
  assert.equal(chancePct(34), "0.3%");
  assert.equal(chancePct(0), "0%");
});

test("stages and their words", () => {
  assert.equal(pressureStage(2499, th), 0);
  assert.equal(pressureStage(2500, th), 1);
  assert.equal(pressureStage(10000, th), 2);
  assert.equal(pressureStage(20000, th), 3);
  assert.equal(pressureStage(30000, th), 4);
  assert.equal(pressureStage(30000, th, true), 3, "a capital never changes hands");
  assert.equal(pressureStage(30000, { ...th, flipping: false }), 3);
  assert.equal(unrestOf(7600, th), 3);
  assert.equal(unrestOf(50000, th), 4);
  assert.equal(outcome(0, th), "no effect");
  assert.equal(outcome(5000, th), "unrest (−2 happy)");
  assert.equal(outcome(12000, th), "revolt (disorder)");
  assert.equal(outcome(31000, th, { who: "the Sheaim" }), "the city joins the Sheaim");
  assert.match(outcome(31000, th, { capital: true }), /capital never/);
});

test("where pressure settles and when it gets there", () => {
  // 10 culture a turn channelled 7 plots away (51%): +5.1 a turn against 2 resisted settles
  // near 19 x 5.1 - 20 x 2 = 56.9 where the city's turn weighs it.
  const inflow = 510;
  const resist = 200;
  const s = settleOf(inflow, resist, th);
  assert.ok(Math.abs(s - 5690) <= 20, `settles at ${s}`);
  assert.equal(settleOf(100, 300, th), 0);
  // Less than a culture a turn net: the minimum fade eats it.
  assert.equal(settleOf(250, 200, th), 0);
  // Never more than it settles at; never at all when the city resists it all.
  assert.equal(turnsTo(0, inflow, resist, 10000, th), null);
  assert.equal(turnsTo(0, 100, 300, 2500, th), null);
  const n = turnsTo(0, inflow, resist, 2500, th);
  assert.ok(n > 0 && n < 20, `unrest after ${n} turns`);
  assert.equal(turnsTo(3000, inflow, resist, 2500, th), 0);
  // The turn it gets there, by the rules' step: +inflow, then -resist and -max(5%, 1 culture).
  let p = 0;
  for (let t = 1; t <= n; t++) p = Math.max(0, p + inflow - resist - Math.max(Math.floor(((p + inflow) * 5) / 100), 100));
  assert.ok(p >= 2500);
  assert.equal(nextStage(0, inflow, resist, th), `unrest in ~${n} turns`);
  assert.equal(nextStage(6000, inflow, resist, th), null, "as far as it settles");
  // Without inflow it fades: below unrest after a few turns.
  const f = turnsBelow(5000, 300, 2500, th);
  assert.ok(f > 0 && f < 10, `${f} turns`);
});

test("the map's revolt marks by level", () => {
  assert.equal(revoltMark({}), null);
  assert.equal(revoltMark({ revolt: 0 }).glyph, "");
  assert.equal(revoltMark({ revolt: 1, unrest: 2 }).glyph, "!");
  assert.match(revoltMark({ revolt: 1, unrest: 2 }).title, /−2 happy/);
  assert.notEqual(revoltMark({ revolt: 2 }).fill, revoltMark({ revolt: 1 }).fill);
  assert.equal(revoltMark({ revolt: 3 }).glyph, "!!");
});
