// node --test: the marks on the cities' name plates and their tooltips, from app/plates.js (the
// boxes a renderer records for a House's shield and a capital's star; the mark under the pointer;
// each mark's tooltip text and reminders).
import test from "node:test";
import assert from "node:assert/strict";
import * as P from "../app/plates.js";
import { SHIELD } from "../app/persons_mark.js";

const furious = { id: 7, capital: true, house: { id: 2, label: "House Aldric", liege: 0, state: "Furious" } };
const rival = { id: 9, capital: false, house: { id: 3, label: "House Alyndra", liege: 1, state: null } };

test("a shield's box (no countdown pill: the third build)", () => {
  const ht = 18;
  const w = (SHIELD.w * ht) / SHIELD.h;
  const [s] = P.shieldHits(rival, 100, 50, ht, w);
  assert.deepEqual(s, { kind: "shield", city: 9, x0: 100 - w / 2, y0: 41, x1: 100 + w / 2, y1: 59 });
  assert.equal(P.shieldHits(rival, 100, 50, ht, w).length, 1);
  assert.deepEqual(P.shieldHits(furious, 100, 50, ht, w + 20).map((x) => x.kind), ["shield"]);
  assert.deepEqual(P.shieldHits({ id: 1, house: null }, 0, 0, 18, 0), []);
});

test("the mark under the pointer: the last drawn first, with a finger's slack", () => {
  const hits = [...P.shieldHits(furious, 100, 50, 18, 36), P.starHit(furious, 104, 44, 6)];
  assert.equal(P.plateHitAt(hits, 104, 44).kind, "star");
  assert.equal(P.plateHitAt(hits, 100, 56).kind, "shield");
  assert.equal(P.plateHitAt(hits, 85, 50), null);
  assert.equal(P.plateHitAt(hits, 89, 50, 6)?.kind, "shield");
  assert.equal(P.plateHitAt([], 0, 0), null);
  assert.equal(P.plateHitAt(undefined, 0, 0), null);
});

test("each mark's tooltip: the House, its state, the rules' outcome, the capital", () => {
  const ctx = { mine: 0, outcome: "House Aldric issues an ultimatum at your next turn. Refused, it breaks away, taking Ventry: war." };
  assert.deepEqual(P.plateTip("shield", furious, ctx), {
    title: "House Aldric (your vassal)",
    lines: [
      { text: "✖ Furious", colour: "#f15b47" },
      { text: "House Aldric issues an ultimatum at your next turn. Refused, it breaks away, taking Ventry: war.", colour: "#f15b47" },
    ],
    reminders: [{ keyword: "KEYWORD_HOUSE" }, { attitude: "ATTITUDE_FURIOUS" }],
  });
  assert.equal(P.plateTip("pill", furious, ctx), null, "no pill");
  assert.deepEqual(P.plateTip("star", furious, ctx), { title: "Capital of House Aldric (your vassal).", lines: [], reminders: [{ keyword: "KEYWORD_HOUSE" }] });
  // Another's House: its liege named, no state (the view sends it to the liege alone).
  assert.deepEqual(P.plateTip("shield", rival, { mine: 0, liegeName: "Svartalfar" }), { title: "House Alyndra (vassal of the Svartalfar)", lines: [], reminders: [{ keyword: "KEYWORD_HOUSE" }] });
  assert.equal(P.plateTip("star", { ...rival, capital: true }, { mine: 0, liegeName: "Svartalfar" }).title, "Capital of House Alyndra (vassal of the Svartalfar).");
  // A capital that is no House's.
  assert.deepEqual(P.plateTip("star", { id: 4, capital: true }, { mine: 0, civ: "Svartalfar" }), { title: "Capital of the Svartalfar.", lines: [], reminders: [] });
  assert.equal(P.plateTip("star", { id: 4, capital: true }, { mine: 0 }), null);
});
