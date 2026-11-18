// node --test: the marks on the cities' name plates and their tooltips, from app/plates.js (the
// boxes a renderer records for a House's shield, its countdown's pill and a capital's star; the
// mark under the pointer; each mark's tooltip text and reminders).
import test from "node:test";
import assert from "node:assert/strict";
import * as P from "../app/plates.js";
import { SHIELD } from "../app/persons_mark.js";

const furious = { id: 7, capital: true, house: { id: 2, label: "House Aldric", liege: 0, state: "Furious", turns: 8 } };
const rival = { id: 9, capital: false, house: { id: 3, label: "House Alyndra", liege: 1, state: null, turns: null } };

test("a shield's box, and its pill's at its left when it drew one", () => {
  const ht = 18;
  const w = (SHIELD.w * ht) / SHIELD.h;
  const [s] = P.shieldHits(rival, 100, 50, ht, w);
  assert.deepEqual(s, { kind: "shield", city: 9, x0: 100 - w / 2, y0: 41, x1: 100 + w / 2, y1: 59 });
  assert.equal(P.shieldHits(rival, 100, 50, ht, w).length, 1);
  const hits = P.shieldHits(furious, 100, 50, ht, w + 20);
  assert.deepEqual(hits.map((x) => x.kind), ["shield", "pill"]);
  assert.equal(hits[1].x1, 100 - w / 2);
  assert.equal(hits[1].x0, 100 - w / 2 - 20);
  assert.deepEqual(P.shieldHits({ id: 1, house: null }, 0, 0, 18, 0), []);
});

test("the mark under the pointer: the last drawn first, with a finger's slack", () => {
  const hits = [...P.shieldHits(furious, 100, 50, 18, 36), P.starHit(furious, 104, 44, 6)];
  assert.equal(P.plateHitAt(hits, 104, 44).kind, "star");
  assert.equal(P.plateHitAt(hits, 100, 56).kind, "shield");
  assert.equal(P.plateHitAt(hits, 85, 50).kind, "pill");
  assert.equal(P.plateHitAt(hits, 60, 50), null);
  assert.equal(P.plateHitAt(hits, 70, 50, 6)?.kind, "pill");
  assert.equal(P.plateHitAt([], 0, 0), null);
  assert.equal(P.plateHitAt(undefined, 0, 0), null);
});

test("each mark's tooltip: the House, its state, the rules' outcome, the capital", () => {
  const ctx = { mine: 0, outcome: "House Aldric will break away with Ventry in 8 turns: war." };
  assert.deepEqual(P.plateTip("shield", furious, ctx), {
    title: "House Aldric (your vassal)",
    lines: [{ text: "✖ Furious 8 turns", colour: "#f15b47" }],
    reminders: [{ keyword: "KEYWORD_HOUSE" }, { attitude: "ATTITUDE_FURIOUS" }],
  });
  assert.deepEqual(P.plateTip("pill", furious, ctx), {
    title: "House Aldric: Furious.",
    lines: [{ text: "House Aldric will break away with Ventry in 8 turns: war.", colour: "#f15b47" }],
    reminders: [{ keyword: "KEYWORD_HOUSE" }, { attitude: "ATTITUDE_FURIOUS" }],
  });
  assert.deepEqual(P.plateTip("star", furious, ctx), { title: "Capital of House Aldric (your vassal).", lines: [], reminders: [{ keyword: "KEYWORD_HOUSE" }] });
  // Another's House: its liege named, no state (the view sends it to the liege alone).
  assert.deepEqual(P.plateTip("shield", rival, { mine: 0, liegeName: "Svartalfar" }), { title: "House Alyndra (vassal of the Svartalfar)", lines: [], reminders: [{ keyword: "KEYWORD_HOUSE" }] });
  assert.equal(P.plateTip("pill", rival, { mine: 0 }), null);
  assert.equal(P.plateTip("star", { ...rival, capital: true }, { mine: 0, liegeName: "Svartalfar" }).title, "Capital of House Alyndra (vassal of the Svartalfar).");
  // A capital that is no House's.
  assert.deepEqual(P.plateTip("star", { id: 4, capital: true }, { mine: 0, civ: "Svartalfar" }), { title: "Capital of the Svartalfar.", lines: [], reminders: [] });
  assert.equal(P.plateTip("star", { id: 4, capital: true }, { mine: 0 }), null);
});
