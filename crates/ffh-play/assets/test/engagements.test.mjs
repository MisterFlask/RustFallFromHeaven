// node --test: engagement combat in the client's words and marks (app/engagements.js,
// docs/ENGAGEMENTS.md): a battle's outcome for the viewer, its line, and the map's markers from the
// view's battles.
import test from "node:test";
import assert from "node:assert/strict";
import { S } from "../app/state.js";
import { outcomeFor, battleLine, battleMarkers, battleById } from "../app/engagements.js";

const battle = (extra = {}) => ({
  id: 3,
  turn: 4,
  x: 10,
  y: 5,
  from: { x: 9, y: 5 },
  place: "Prespur",
  city: 1,
  viewer_side: 0,
  result: "AttackerWon",
  plot_taken: true,
  city_captured: 2,
  has_log: true,
  sides: [{ lost: 0 }, { lost: 3 }],
  ...extra,
});

test("a battle's outcome is the viewer's", () => {
  assert.equal(outcomeFor(battle()), "won");
  assert.equal(outcomeFor(battle({ result: "DefenderHeld" })), "repelled");
  assert.equal(outcomeFor(battle({ viewer_side: 1 })), "lost");
  assert.equal(outcomeFor(battle({ viewer_side: 1, result: "DefenderHeld" })), "held");
  assert.equal(outcomeFor(battle({ result: "Draw" })), "draw");
  assert.equal(outcomeFor(battle({ result: "Mutual" })), "draw");
});

test("a battle's line names the place, the outcome and both sides' losses as the viewer sees them", () => {
  assert.equal(battleLine(battle()), "Prespur: won, the city taken; 0 lost, 3 of theirs");
  assert.equal(battleLine(battle({ city_captured: null })), "Prespur: won; 0 lost, 3 of theirs");
  assert.equal(battleLine(battle({ viewer_side: 1, result: "DefenderHeld", place: null })), "(10, 5): held; 3 lost, 0 of theirs");
});

test("the map's markers: one per plot, the newest battle there (the view lists newest first)", () => {
  S.view = { battles: [battle({ id: 7, result: "DefenderHeld" }), battle({ id: 3 }), battle({ id: 2, x: 4, y: 4, has_log: false })] };
  const m = battleMarkers();
  assert.deepEqual(
    m.map((x) => [x.id, x.x, x.y, x.kind, x.has_log]),
    [
      [7, 10, 5, "repelled", true],
      [2, 4, 4, "won", false],
    ],
  );
  assert.match(m[0].title, /^Battle #7, Prespur: repelled/);
  assert.equal(battleById(3)?.id, 3);
  assert.equal(battleById(99), null);
  S.view = null;
  assert.deepEqual(battleMarkers(), []);
});
