// node --test: a game controller on a phone (app/pad.js): the standard mapping read into buttons,
// sticks and directions, presses and a held direction's repeat, and the direction picks of the
// map's cursor and of the interface's focus.
import test from "node:test";
import assert from "node:assert/strict";
import { BTN, readPad, anyInput, standard, Edges, REPEAT_DELAY, REPEAT_EVERY, cursorStep, focusStep } from "../app/pad.js";

const pad = ({ down = [], axes = [0, 0, 0, 0], mapping = "standard" } = {}) => ({
  mapping,
  connected: true,
  axes,
  buttons: Array.from({ length: 17 }, (_, i) => ({ pressed: down.includes(i), value: down.includes(i) ? 1 : 0 })),
});

test("only the standard mapping is read", () => {
  assert.equal(standard(pad()), true);
  assert.equal(standard(pad({ mapping: "" })), false);
  assert.equal(standard({ ...pad(), connected: false }), false);
  assert.equal(standard(null), false);
});

test("buttons, the D-pad and the left stick as a direction, the right stick as a pan", () => {
  const rest = readPad(pad());
  assert.equal(anyInput(rest), false);
  assert.equal(rest.dir, null);
  assert.equal(rest.pan, null);
  const a = readPad(pad({ down: [BTN.a, BTN.rb] }));
  assert.deepEqual([...a.held].sort((x, y) => x - y), [BTN.a, BTN.rb]);
  assert.equal(anyInput(a), true);
  // The D-pad: screen axes, y down.
  assert.deepEqual(readPad(pad({ down: [BTN.up] })).dir, { x: 0, y: -1 });
  assert.deepEqual(readPad(pad({ down: [BTN.down, BTN.right] })).dir, { x: 1, y: 1 });
  // The left stick: eight ways past the dead zone.
  assert.deepEqual(readPad(pad({ axes: [0.9, 0.1, 0, 0] })).dir, { x: 1, y: 0 });
  assert.deepEqual(readPad(pad({ axes: [-0.7, -0.7, 0, 0] })).dir, { x: -1, y: -1 });
  assert.equal(readPad(pad({ axes: [0.3, 0.2, 0, 0] })).dir, null);
  // The D-pad wins over the stick.
  assert.deepEqual(readPad(pad({ down: [BTN.left], axes: [0.9, 0, 0, 0] })).dir, { x: -1, y: 0 });
  // The right stick: outside the dead zone, rescaled to reach 1.
  const p = readPad(pad({ axes: [0, 0, 1, -0.1] })).pan;
  assert.equal(p.x, 1);
  assert.equal(p.y, 0);
  assert.equal(readPad(pad({ axes: [0, 0, 0.1, 0.1] })).pan, null);
  // An analog trigger past half way is a press.
  const t = pad();
  t.buttons[BTN.rt] = { pressed: false, value: 0.8 };
  assert.ok(readPad(t).held.has(BTN.rt));
});

test("a press is its first frame; a held direction repeats after a delay", () => {
  const e = new Edges();
  const s = (o) => readPad(pad(o));
  assert.deepEqual(e.step(s({ down: [BTN.a] }), 0).down, [BTN.a]);
  assert.deepEqual(e.step(s({ down: [BTN.a] }), 16).down, []);
  assert.deepEqual(e.step(s(), 32).down, []);
  assert.deepEqual(e.step(s({ down: [BTN.a] }), 48).down, [BTN.a]);
  // A direction: at once, not again until the delay, then every REPEAT_EVERY.
  const d = new Edges();
  const right = s({ down: [BTN.right] });
  assert.deepEqual(d.step(right, 1000).dir, { x: 1, y: 0 });
  assert.equal(d.step(right, 1000 + REPEAT_DELAY - 1).dir, null);
  assert.deepEqual(d.step(right, 1000 + REPEAT_DELAY).dir, { x: 1, y: 0 });
  assert.equal(d.step(right, 1000 + REPEAT_DELAY + REPEAT_EVERY - 1).dir, null);
  assert.deepEqual(d.step(right, 1000 + REPEAT_DELAY + REPEAT_EVERY).dir, { x: 1, y: 0 });
  // Another direction acts at once.
  assert.deepEqual(d.step(s({ down: [BTN.up] }), 1000 + REPEAT_DELAY + REPEAT_EVERY + 1).dir, { x: 0, y: -1 });
  // Let go, then pressed again: at once.
  d.step(s(), 2000);
  assert.deepEqual(d.step(right, 2001).dir, { x: 1, y: 0 });
});

const MAP = { w: 10, h: 6, wrap_x: true, wrap_y: false };
/** The flat map's projection (north up: y grows upwards on screen), the copy nearest column 0. */
const flat = (x, y) => ({ x: (x > MAP.w / 2 ? x - MAP.w : x) * 10, y: -y * 10 });
/** A diamond projection (the isometric map's kind): east is down-right, north up-right. */
const iso = (x, y) => ({ x: ((x > MAP.w / 2 ? x - MAP.w : x) + y) * 16, y: (y - (x > MAP.w / 2 ? x - MAP.w : x)) * -8 });

test("the cursor steps to the neighbour that lies that way on screen", () => {
  const at = { x: 2, y: 2 };
  assert.deepEqual(cursorStep(at, { x: 0, y: -1 }, MAP, flat), { x: 2, y: 3 });
  assert.deepEqual(cursorStep(at, { x: 1, y: 0 }, MAP, flat), { x: 3, y: 2 });
  assert.deepEqual(cursorStep(at, { x: -1, y: 1 }, MAP, flat), { x: 1, y: 1 });
  // Diamonds: screen up and screen right are diagonal steps on the map.
  const up = cursorStep(at, { x: 0, y: -1 }, MAP, iso);
  const s0 = iso(at.x, at.y);
  const s1 = iso(up.x, up.y);
  assert.ok(s1.y < s0.y && Math.abs(s1.x - s0.x) < 1e-9, `up went to ${JSON.stringify(up)}`);
  const right = cursorStep(at, { x: 1, y: 0 }, MAP, iso);
  const s2 = iso(right.x, right.y);
  assert.ok(s2.x > s0.x && Math.abs(s2.y - s0.y) < 1e-9, `right went to ${JSON.stringify(right)}`);
});

test("the cursor wraps where the map does and stops at its edges", () => {
  // West of column 0 is column 9 on a map wrapping east-west.
  assert.deepEqual(cursorStep({ x: 0, y: 2 }, { x: -1, y: 0 }, MAP, flat), { x: 9, y: 2 });
  // North of the top row: nothing.
  assert.equal(cursorStep({ x: 3, y: MAP.h - 1 }, { x: 0, y: -1 }, MAP, flat), null);
  const flatNoWrap = { ...MAP, wrap_x: false };
  assert.equal(cursorStep({ x: 0, y: 2 }, { x: -1, y: 0 }, flatNoWrap, (x, y) => ({ x: x * 10, y: -y * 10 })), null);
  // No screen place (the 3D map's behind-the-eye answer): no step.
  assert.equal(cursorStep({ x: 3, y: 3 }, { x: 1, y: 0 }, MAP, () => null), null);
});

const R = (left, top, w = 40, h = 30) => ({ left, top, right: left + w, bottom: top + h });

test("the focus moves to the nearest button that way, straight ahead first", () => {
  // Two rows of three, then a wide button under them.
  const rects = [R(0, 0), R(50, 0), R(100, 0), R(0, 40), R(50, 40), R(100, 40), R(0, 80, 140)];
  // The pick from button i among the others, as an index into `rects`.
  const pick = (i, dir) => {
    const others = rects.filter((_, j) => j !== i);
    const k = focusStep(rects[i], dir, others);
    return k < 0 ? -1 : rects.indexOf(others[k]);
  };
  assert.equal(pick(0, { x: 1, y: 0 }), 1);
  assert.equal(pick(1, { x: 1, y: 0 }), 2);
  assert.equal(pick(2, { x: 1, y: 0 }), -1);
  assert.equal(pick(1, { x: 0, y: 1 }), 4);
  assert.equal(pick(4, { x: 0, y: -1 }), 1);
  assert.equal(pick(5, { x: 0, y: 1 }), 6);
  assert.equal(pick(6, { x: 0, y: 1 }), -1);
  assert.equal(pick(4, { x: -1, y: 0 }), 3);
  // Up from the wide button: the row above, the nearest across (its left part).
  assert.equal(pick(6, { x: 0, y: -1 }), 3);
  // A diagonal: by the centres.
  assert.equal(pick(0, { x: 1, y: 1 }), 4);
});

test("a button straight ahead beats a nearer one off to the side", () => {
  const from = R(0, 0);
  const ahead = R(0, 200);
  const side = R(300, 40);
  assert.equal(focusStep(from, { x: 0, y: 1 }, [side, ahead]), 1);
});
