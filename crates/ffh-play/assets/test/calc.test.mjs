// node --test: calculated numbers' tooltips (app/calc.js; docs/PLAY_DESIGN.md, "Calculated
// numbers"): a calculation's lines in order, a courtship's parts flattened or picked.
import test from "node:test";
import assert from "node:assert/strict";
import { calcLines } from "../app/calc.js";

test("a calculation's lines", () => {
  assert.deepEqual(calcLines(null), []);
  assert.deepEqual(calcLines(["Ventry: 6.0 × 50% = 3.0.", "", "Inflow: 3.0 a turn."]), ["Ventry: 6.0 × 50% = 3.0.", "Inflow: 3.0 a turn."]);
  const cv = {
    turn: ["Next liege turn: 5.3 + 2.1 − 0.0 = 7.4, less 5%: 7.0."],
    gain: ["Gain: 5.5 × 39% = 2.1 a turn."],
    requirement: ["Requirement: (0 + 6 − 2 × 1) × 2 = 8."],
    reach: ["Reach: 8 plots: (5 ÷ 8)² = 39%."],
    inflow: ["Inflow: your cities' culture 11.0 × 50% = 5.5 a turn."],
    guard: [],
  };
  // A courtship's parts in the tooltip's order, whatever the object's.
  assert.deepEqual(calcLines(cv), [cv.requirement[0], cv.inflow[0], cv.reach[0], cv.gain[0], cv.turn[0]]);
  assert.deepEqual(calcLines(cv, ["inflow", "reach", "gain"]), [cv.inflow[0], cv.reach[0], cv.gain[0]]);
  assert.deepEqual(calcLines(cv, ["guard"]), []);
});
