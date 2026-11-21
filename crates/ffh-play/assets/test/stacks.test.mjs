// node --test: the stacks on the map, from app/stacks.js (each tile's head unit and its state, the
// state marks, the units' moves since the last view).
import test from "node:test";
import assert from "node:assert/strict";
import * as St from "../app/stacks.js";

test("the head of a stack and its state", () => {
  const info = { 1: { hero: false, combat: "MELEE", rating: 30 }, 2: { hero: true, combat: "MELEE", rating: 60 }, 3: { hero: false, combat: "NONE", rating: 0 } };
  const acts = ["ACTIVITY_AWAKE", "ACTIVITY_HOLD", "ACTIVITY_SLEEP", "ACTIVITY_HEAL", "ACTIVITY_SENTRY"];
  const ctx = (extra = {}) => ({ me: 0, selected: [], activity: (i) => acts[i], unitInfo: (t) => info[t], automated: () => false, facing: (id) => (id === 11 ? "left" : "right"), ...extra });
  // Own units: the first that can move; moves, mark, hero, count.
  const own = [
    { id: 10, owner: 0, type: 1, hp: 100, max_hp: 100, moves: 0, activity: 2, group: 1 },
    { id: 11, owner: 0, type: 2, hp: 50, max_hp: 100, moves: 60, activity: 0, group: 2 },
  ];
  let st = St.stackState(own, ctx());
  assert.equal(st.head.id, 11);
  assert.deepEqual(st.state, { own: true, selected: false, hp: 50, count: 2, hero: true, facing: "left", moves: true, mark: null });
  // The selected unit leads; a fortified fighter shows the shield, a sleeping worker the crescent.
  st = St.stackState(own, ctx({ selected: [10] }));
  assert.equal(st.head.id, 10);
  assert.equal(st.state.selected, true);
  assert.equal(st.state.moves, false);
  assert.equal(st.state.mark, "fortify");
  st = St.stackState([{ id: 12, owner: 0, type: 3, hp: 100, max_hp: 100, moves: 60, activity: 2, group: 3 }], ctx());
  assert.equal(st.state.mark, "sleep");
  // A promotion waiting takes precedence; automated groups show the gear.
  st = St.stackState([{ ...own[0], promo_ready: true }], ctx());
  assert.equal(st.state.mark, "promote");
  st = St.stackState([own[1]], ctx({ automated: () => true }));
  assert.equal(st.state.mark, "auto");
  // A marching recruit (this game's armies: the rules move it) shows the march mark, before a
  // promotion waiting.
  st = St.stackState([{ ...own[0], promo_ready: true, march: { to: "the 1st Army", turns_left: 3, stalled: 0 } }], ctx());
  assert.equal(st.state.mark, "march");
  // Foreign stacks: the strongest stands in front; no moves, no mark.
  const foreign = [
    { id: 20, owner: 1, type: 1, hp: 100, max_hp: 100 },
    { id: 21, owner: 1, type: 2, hp: 80, max_hp: 100 },
  ];
  st = St.stackState(foreign, ctx());
  assert.equal(st.head.id, 21);
  assert.equal(st.state.own, false);
  assert.equal(st.state.moves, undefined);
  assert.equal(st.state.mark, undefined);
  assert.equal(St.stackState([], ctx()), null);
});

test("the unit every map draws for a stack: stackHead rates units by their type's rating", () => {
  const data = { tables: { UnitType: [{ type: "UNIT_WORKER", rating: 0 }, { type: "UNIT_AXEMAN", rating: 50 }] } };
  assert.equal(St.unitRating(data, { type: 1 }), 50);
  assert.equal(St.unitRating(data, { type: 9 }), 0, "an unknown type");
  assert.equal(St.unitRating(null, { type: 1 }), 0, "no data yet");
  const units = [
    { id: 1, owner: 2, type: 0, moves: 60 },
    { id: 4, owner: 2, type: 1, moves: 60 },
    { id: 2, owner: 0, type: 0, moves: 0 },
    { id: 3, owner: 0, type: 0, moves: 60 },
  ];
  // Own units: the selection's first, else the first that can move.
  assert.equal(St.stackHead(units, 0, [], data).id, 3);
  assert.equal(St.stackHead(units, 0, [2], data).id, 2);
  // A foreign stack: its strongest unit, not the worker listed first.
  assert.equal(St.stackHead(units, 5, [], data).id, 4);
  assert.equal(St.stackHead([], 0, [], data), null);
});

test("marks from Civ4's activities", () => {
  assert.equal(St.markOf("ACTIVITY_SLEEP", { canFight: true }), "fortify");
  assert.equal(St.markOf("ACTIVITY_SLEEP"), "sleep");
  assert.equal(St.markOf("ACTIVITY_SENTRY"), "sentry");
  assert.equal(St.markOf("ACTIVITY_HEAL"), "heal");
  assert.equal(St.markOf("ACTIVITY_AWAKE"), null);
  assert.equal(St.markOf("ACTIVITY_HEAL", { promote: true }), "promote");
});

test("unitMoves: net displacement of the units that changed tile, wrap-aware", () => {
  let { positions, moved } = St.unitMoves(new Map(), [{ id: 1, x: 5, y: 7 }, { id: 2, x: 0, y: 0 }], 20, true);
  assert.equal(moved.length, 0, "no previous positions: nothing moved");
  assert.deepEqual(positions.get(1), { x: 5, y: 7 });
  // A straight step east and a step north; a unit standing still is not moved.
  ({ positions, moved } = St.unitMoves(positions, [{ id: 1, x: 6, y: 8 }, { id: 2, x: 0, y: 0 }], 20, true));
  assert.deepEqual(moved, [{ id: 1, dx: 1, dy: 1 }]);
  // A step west across the seam (19 -> 0) is +1, not +19 - 0.
  ({ positions, moved } = St.unitMoves(positions, [{ id: 1, x: 6, y: 8 }, { id: 2, x: 19, y: 0 }], 20, true));
  assert.deepEqual(moved, [{ id: 2, dx: -1, dy: 0 }]);
  // A brand-new unit (id 3) is never "moved"; a vanished unit drops out of positions.
  ({ positions, moved } = St.unitMoves(positions, [{ id: 3, x: 1, y: 1 }], 20, true));
  assert.equal(moved.length, 0);
  assert.equal(positions.has(1), false);
  assert.deepEqual(positions.get(3), { x: 1, y: 1 });
});
