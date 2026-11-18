// node --test: the marks of Persons of Note, from app/persons_mark.js (the state's word, glyph,
// class and text; the Houses' colours and labels; the shield's outline and its mark on the map).
import test from "node:test";
import assert from "node:assert/strict";
import * as M from "../app/persons_mark.js";

test("one mark for each state: word, glyph, class", () => {
  assert.deepEqual(M.STATES, ["devoted", "loyal", "restless", "disloyal"]);
  assert.deepEqual(M.STATES.map(M.stateWord), ["Devoted", "Loyal", "Restless", "Disloyal"]);
  assert.deepEqual(M.STATES.map(M.stateGlyph), ["▲", "●", "◆", "✖"]);
  assert.deepEqual(M.STATES.map(M.stateClass), ["ps-devoted", "ps-loyal", "ps-restless", "ps-disloyal"]);
  // The host's names, any case; an unknown or missing state is Loyal (an empty seat).
  assert.equal(M.stateKey("Restless"), "restless");
  assert.equal(M.stateKey("DISLOYAL"), "disloyal");
  assert.equal(M.stateKey(null), "loyal");
  assert.equal(M.stateKey("Wavering"), "loyal");
  for (const s of M.STATES) assert.match(M.stateColour(s), /^#[0-9a-f]{6}$/);
  assert.equal(new Set(M.STATES.map(M.stateColour)).size, 4);
});

test("the mark as text, with the turns of a countdown", () => {
  assert.equal(M.stateText("Disloyal", 8), "Disloyal ✖ 8 turns");
  assert.equal(M.stateText("disloyal", 1), "Disloyal ✖ 1 turn");
  assert.equal(M.stateText("Loyal"), "Loyal ●");
  assert.equal(M.stateText("Devoted", null), "Devoted ▲");
  assert.equal(M.stateText("Restless", 0), "Restless ◆ 0 turns");
});

test("eight distinct House colours, wrapped by index", () => {
  assert.equal(M.HOUSE_COLOURS.length, 8);
  assert.equal(M.HOUSE_TINCTURES.length, 8);
  for (const c of M.HOUSE_COLOURS) assert.match(c, /^#[0-9a-f]{6}$/);
  assert.equal(new Set(M.HOUSE_COLOURS).size, 8);
  assert.equal(M.houseColour(0), M.HOUSE_COLOURS[0]);
  assert.equal(M.houseColour(7), M.HOUSE_COLOURS[7]);
  assert.equal(M.houseColour(8), M.HOUSE_COLOURS[0]);
  assert.equal(M.houseColour(-1), M.HOUSE_COLOURS[7]);
  assert.equal(M.houseColour(undefined), M.HOUSE_COLOURS[0]);
  // No two colours too close to tell apart (a rough RGB distance).
  const rgb = (c) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
  for (let i = 0; i < 8; i++)
    for (let j = i + 1; j < 8; j++) {
      const [a, b] = [rgb(M.HOUSE_COLOURS[i]), rgb(M.HOUSE_COLOURS[j])];
      const d = Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
      assert.ok(d > 60, `${M.HOUSE_TINCTURES[i]} and ${M.HOUSE_TINCTURES[j]} are too close (${d.toFixed(0)})`);
    }
});

test("a House's label", () => {
  assert.equal(M.houseLabel("Ashledger"), "House Ashledger");
  assert.equal(M.houseLabel("House Arne"), "House Arne");
});

test("the shield: a path and a polygon in the same box", () => {
  assert.equal(M.SHIELD.w, 16);
  assert.equal(M.SHIELD.h, 18);
  assert.match(M.SHIELD.path, /^M[\d .HVCZ]+$/);
  const pts = M.shieldPoints();
  assert.deepEqual(pts[0], [1, 1]);
  assert.deepEqual(pts[1], [15, 1]);
  // The point at the bottom middle, the outline closing on the left flank.
  assert.ok(pts.some(([x, y]) => x === 8 && y === 17));
  assert.deepEqual(pts[pts.length - 1], [1, 8]);
  for (const [x, y] of pts) assert.ok(x >= 0 && x <= 16 && y >= 0 && y <= 18);
  // Scaled to another box.
  const big = M.shieldPoints(32, 36);
  assert.deepEqual(big[1], [30, 2]);
});

test("what the map draws on a shield", () => {
  assert.equal(M.shieldMark("Devoted"), null);
  assert.equal(M.shieldMark("Loyal", 3), null);
  assert.deepEqual(M.shieldMark("Restless"), { glyph: "◆", colour: M.stateColour("restless"), count: null });
  assert.deepEqual(M.shieldMark("Disloyal", 8), { glyph: "✖", colour: M.stateColour("disloyal"), count: 8 });
});
