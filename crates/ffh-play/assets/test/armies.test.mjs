// node --test: this game's armies in words and marks (app/armies.js, app/iso.js): a marching
// recruit's line in the unit panel and the Armies screen, and its mark on the map (docs/ARMIES.md).
import test from "node:test";
import assert from "node:assert/strict";
import { marchLine, marchState } from "../app/armies.js";
import { marchMark } from "../app/iso.js";

const march = (extra = {}) => ({ dest: { army: 1 }, to: "the 1st Army", from: "Ithralia", turns_left: 3, stalled: 0, diverted: false, ...extra });

test("a marching recruit's line says where it goes and how long", () => {
  assert.equal(marchLine(march()), "Marching to the 1st Army (3 turns)");
  assert.equal(marchLine(march({ turns_left: 1 })), "Marching to the 1st Army (1 turn)");
  assert.equal(marchLine(march({ turns_left: 0 })), "Marching to the 1st Army (arrives)");
  assert.equal(marchLine(march({ turns_left: null })), "Marching to the 1st Army (no way there now)");
  // A stall (its army out of supply, the way blocked) says how long it has waited.
  assert.equal(marchLine(march({ stalled: 2 })), "Marching to the 1st Army (waiting 2 turns)");
  assert.equal(marchLine(march({ to: "the garrison of Bannor", dest: { garrison: 4 } })), "Marching to the garrison of Bannor (3 turns)");
});

test("the Armies screen's state of a recruit: away, arriving, waiting or without a way", () => {
  assert.deepEqual([marchState(march()).text, marchState(march()).warn], ["3 turns away", false]);
  assert.equal(marchState(march({ turns_left: 0 })).text, "arrives");
  assert.deepEqual([marchState(march({ stalled: 1 })).text, marchState(march({ stalled: 1 })).warn], ["waiting 1 turn", true]);
  assert.deepEqual([marchState(march({ turns_left: null })).text, marchState(march({ turns_left: null })).warn], ["no way there now", true]);
});

test("a marching recruit's map mark: our own shown unit only, with its destination", () => {
  assert.equal(marchMark(null, 0), null);
  assert.equal(marchMark({ owner: 0, march: null }, 0), null);
  assert.equal(marchMark({ owner: 1, march: march() }, 0), null);
  assert.equal(marchMark({ owner: 0, march: march() }, 0).title, "Marching to the 1st Army");
});
