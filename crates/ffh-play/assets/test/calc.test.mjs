// node --test: calculated numbers' tooltips (app/calc.js; docs/PLAY_DESIGN.md, "Calculated
// numbers"): a calculation's lines in order.
import test from "node:test";
import assert from "node:assert/strict";
import { calcLines } from "../app/calc.js";

test("a calculation's lines", () => {
  assert.deepEqual(calcLines(null), []);
  assert.deepEqual(calcLines(["Wounds: 2 × 10% = 20%.", "", "Heals in 3 turns."]), ["Wounds: 2 × 10% = 20%.", "Heals in 3 turns."]);
  assert.deepEqual(calcLines({ gain: ["x"] }), [], "only a list of lines");
});
