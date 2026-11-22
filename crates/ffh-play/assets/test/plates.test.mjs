// node --test: the marks on the cities' name plates and their tooltips, from app/plates.js (the
// box a renderer records for a capital's star; the mark under the pointer; the mark's tooltip
// text).
import test from "node:test";
import assert from "node:assert/strict";
import * as P from "../app/plates.js";

const capital = { id: 7, capital: true };

test("the mark under the pointer: the last drawn first, with a finger's slack", () => {
  const hits = [P.starHit(capital, 100, 50, 6), P.starHit({ id: 8 }, 104, 44, 6)];
  assert.equal(P.plateHitAt(hits, 104, 44).city, 8);
  assert.equal(P.plateHitAt(hits, 98, 52).city, 7);
  assert.equal(P.plateHitAt(hits, 85, 50), null);
  assert.equal(P.plateHitAt(hits, 89, 50, 6)?.city, 7);
  assert.equal(P.plateHitAt([], 0, 0), null);
  assert.equal(P.plateHitAt(undefined, 0, 0), null);
});

test("a capital's star: its civilization", () => {
  assert.deepEqual(P.plateTip("star", capital, { civ: "Svartalfar" }), { title: "Capital of the Svartalfar.", lines: [], reminders: [] });
  assert.equal(P.plateTip("star", capital, {}), null);
  assert.equal(P.plateTip("shield", capital, { civ: "Svartalfar" }), null, "no other mark");
});
