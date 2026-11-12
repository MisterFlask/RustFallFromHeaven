// node --test: keywords and reminder text (docs/PLAY_DESIGN.md, "Keywords and reminder text"):
// the chips of an entry, the reminders a chip shows, where the reminder boxes go beside or below
// a tooltip (app/keyword_view.js), and the keyword marks of the markup (app/markup.js).
import test from "node:test";
import assert from "node:assert/strict";
import * as K from "../app/keyword_view.js";
import { tokenize, plain } from "../app/markup.js";

const WR = { key: "KEYWORD_WORLD_RITUAL", name: "World Ritual", text: "One in progress per player.", with: ["KEYWORD_WORLD_RITUAL", "KEYWORD_RITUAL"] };
const R = { key: "KEYWORD_RITUAL", name: "Ritual", text: "Cannot be hurried with gold.", with: ["KEYWORD_RITUAL"] };
const C = { key: "KEYWORD_X", name: "X", text: "A rule.", with: ["KEYWORD_X"] };

test("an entry's chips are its reminders, in the server's order", () => {
  assert.deepEqual(K.chipsOf([WR, R, C]).map((c) => c.name), ["World Ritual", "Ritual", "X"]);
  assert.deepEqual(K.chipsOf(undefined), []);
});

test("a chip shows its own reminder and those it implies", () => {
  assert.deepEqual(K.remindersFor([WR, R, C], "KEYWORD_WORLD_RITUAL").map((r) => r.name), ["World Ritual", "Ritual"]);
  assert.deepEqual(K.remindersFor([WR, R, C], "KEYWORD_X").map((r) => r.name), ["X"]);
  assert.deepEqual(K.remindersFor([R], "KEYWORD_X"), []);
  // Without `with`: the reminder alone.
  assert.deepEqual(K.remindersFor([{ key: "A", name: "A", text: "a" }], "A").map((r) => r.key), ["A"]);
});

test("the reminder boxes go beside the tooltip on a wide window, below it on a narrow one", () => {
  const wide = K.tipLayout({ x: 100, y: 100, vw: 1280, vh: 800, main: { w: 300, h: 200 }, side: { w: 240, h: 150 } });
  assert.equal(wide.stacked, false);
  assert.deepEqual(wide.main, { left: 114, top: 114 });
  assert.deepEqual(wide.side, { left: 114 + 300 + 6, top: 114 });
  const narrow = K.tipLayout({ x: 20, y: 100, vw: 420, vh: 844, main: { w: 380, h: 200 }, side: { w: 380, h: 150 } });
  assert.equal(narrow.stacked, true);
  assert.deepEqual(narrow.side, { left: narrow.main.left, top: narrow.main.top + 200 + 6 });
  assert.ok(narrow.main.left >= 4 && narrow.main.left + 380 <= 420);
  // At the window's right edge the whole goes left of the pointer.
  const edge = K.tipLayout({ x: 1200, y: 100, vw: 1280, vh: 800, main: { w: 300, h: 200 }, side: { w: 240, h: 150 } });
  assert.equal(edge.main.left, 1200 - 14 - 546);
  assert.equal(edge.side.left, edge.main.left + 306);
  // Near the bottom it goes up.
  const low = K.tipLayout({ x: 10, y: 780, vw: 1280, vh: 800, main: { w: 300, h: 200 }, side: { w: 240, h: 260 } });
  assert.equal(low.main.top, 780 - 14 - 260);
  // No reminders: the tooltip alone; reminders alone (a chip): the boxes at the pointer.
  assert.equal(K.tipLayout({ x: 0, y: 0, vw: 800, vh: 600, main: { w: 100, h: 50 } }).side, null);
  assert.deepEqual(K.tipLayout({ x: 50, y: 50, vw: 800, vh: 600, main: { w: 0, h: 0 }, side: { w: 240, h: 90 } }).side, { left: 64, top: 64 });
});

test("a keyword mark is a token of its own; its plain text is the word", () => {
  const t = tokenize("Every project is a [KW=KEYWORD_RITUAL]ritual[\\KW] here.");
  assert.deepEqual(t, [
    { t: "text", s: "Every project is a " },
    { t: "kw", key: "KEYWORD_RITUAL" },
    { t: "text", s: "ritual" },
    { t: "/kw" },
    { t: "text", s: " here." },
  ]);
  assert.equal(plain("Every project is a [KW=KEYWORD_RITUAL]ritual[\\KW] here."), "Every project is a ritual here.");
});
