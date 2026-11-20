// node --test: Houses in the map's colours, plates, the diplomacy list and the alerts (docs/PERSONS.md,
// "The second build", rules 1 and 13): tiles.js `setColourPlayers` / `playerColor`, persons_lines.js
// `plateMark` and `alertFocus`, diplo.js `byLiege`.
import test from "node:test";
import assert from "node:assert/strict";
import { setColourPlayers, colourPlayer, colourKey, playerColor, PLAYER_COLORS } from "../app/tiles.js";
import { plateMark, alertFocus } from "../app/persons_lines.js";
import { byLiege } from "../app/diplo.js";

test("a House is drawn in its liege's colour", () => {
  setColourPlayers([[5, 0], [7, 1], [3, 3]]);
  assert.equal(colourPlayer(5), 0);
  assert.equal(colourPlayer(3), 3, "its own colour: no entry");
  assert.equal(colourPlayer(9), 9);
  assert.equal(playerColor(5), PLAYER_COLORS[0]);
  assert.equal(playerColor(7), playerColor(1));
  assert.equal(playerColor(5, true), "#5a1010", "the barbarians stay dark red");
  const key = colourKey();
  setColourPlayers([[7, 1], [5, 0]]);
  assert.equal(colourKey(), key, "the key does not depend on the order");
  setColourPlayers([[7, 1]]);
  assert.notEqual(colourKey(), key, "a House breaking away changes the key");
  assert.equal(playerColor(5), PLAYER_COLORS[5]);
  setColourPlayers([]);
});

test("the plate's mark: every state for the liege (no countdown: the third build)", () => {
  assert.equal(plateMark(null), null, "another's House: its shield only");
  assert.deepEqual(Object.keys(plateMark("Cautious")).sort(), ["colour", "glyph"]);
  assert.equal(plateMark("Friendly").glyph, "▲");
  assert.equal(plateMark("Furious").glyph, "✖");
});

test("an alert opens the Court at its House", () => {
  const houses = [{ id: 2, label: "House Arne" }, { id: 4, label: "Arne" }];
  assert.deepEqual(alertFocus({ house: 4, text: "House Arne requests: ..." }, houses), { house: 4 }, "the House the entry tags");
  assert.deepEqual(alertFocus({ text: "Torrin granted to House Arne." }, houses), { house: 2 }, "the longest label named");
  assert.equal(alertFocus({ text: "Nothing of ours." }, houses), null);
  // A House of another realm the text names: its card (a courtship of yours, the fourth build).
  const others = [{ id: 7, label: "House Alyndra" }];
  assert.deepEqual(alertFocus({ text: "House Alyndra: your favour is 130 of 188." }, houses, others), { foreign: 7 });
  assert.deepEqual(alertFocus({ text: "House Arne and House Alyndra." }, houses, others), { house: 2 }, "yours first");
});

test("the diplomacy list: your Houses first, each House under its liege", () => {
  const p = (id, house = null) => ({ id, house });
  const players = [p(1), p(2), p(3, { liege: 0 }), p(4, { liege: 1 }), p(5, { liege: 9 }), p(6, { liege: 0 })];
  assert.deepEqual(
    byLiege(players, 0).map((x) => x.id),
    [3, 6, 1, 4, 2, 5],
  );
});
