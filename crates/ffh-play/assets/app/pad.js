// A game controller on a phone (docs/HUMAN.md, "A controller on a phone"), without the DOM (tested
// by assets/test/pad.test.mjs): the W3C Gamepad API's standard mapping read into named buttons,
// presses with a held direction's repeat, and the direction picks of the map's cursor (the next plot
// on screen) and of the interface's focus (the next button on screen). gamepad.js applies them.

/** The standard mapping's buttons (https://w3c.github.io/gamepad/#remapping), by name. */
export const BTN = {
  a: 0,
  b: 1,
  x: 2,
  y: 3,
  lb: 4,
  rb: 5,
  lt: 6,
  rt: 7,
  back: 8,
  start: 9,
  ls: 10,
  rs: 11,
  up: 12,
  down: 13,
  left: 14,
  right: 15,
};

/** A stick counts as a direction past this deflection (and as the D-pad does). */
export const DIR_DEAD = 0.5;
/** The right stick pans past this deflection. */
export const PAN_DEAD = 0.18;
/** A direction held repeats after REPEAT_DELAY ms, then every REPEAT_EVERY ms. */
export const REPEAT_DELAY = 320;
export const REPEAT_EVERY = 110;

/** Whether gamepad `p` uses the standard mapping (the only one read: its buttons are where the
 *  table above says on every brand). */
export function standard(p) {
  return !!p && p.connected !== false && p.mapping === "standard";
}

const pressed = (b) => !!b && (typeof b === "object" ? b.pressed || b.value > 0.5 : b > 0.5);

/** The state of standard gamepad `p` at one frame: `held` (the button indices down), the left
 *  stick or D-pad as a direction `dir` ({x, y} with each in -1, 0, 1, screen axes: y down; null at
 *  rest), and the right stick `pan` ({x, y}, each in -1..1 outside the dead zone, else null). */
export function readPad(p) {
  const held = new Set();
  (p.buttons ?? []).forEach((b, i) => pressed(b) && held.add(i));
  const ax = p.axes ?? [];
  let dx = (held.has(BTN.right) ? 1 : 0) - (held.has(BTN.left) ? 1 : 0);
  let dy = (held.has(BTN.down) ? 1 : 0) - (held.has(BTN.up) ? 1 : 0);
  if (!dx && !dy) {
    const lx = ax[0] ?? 0;
    const ly = ax[1] ?? 0;
    if (Math.hypot(lx, ly) > DIR_DEAD) {
      // Eight ways, by the stick's angle.
      const a = Math.round(Math.atan2(ly, lx) / (Math.PI / 4));
      dx = Math.round(Math.cos((a * Math.PI) / 4));
      dy = Math.round(Math.sin((a * Math.PI) / 4));
    }
  }
  const rx = ax[2] ?? 0;
  const ry = ax[3] ?? 0;
  const dz = (v) => (Math.abs(v) < PAN_DEAD ? 0 : (v - Math.sign(v) * PAN_DEAD) / (1 - PAN_DEAD));
  const pan = Math.hypot(rx, ry) > PAN_DEAD ? { x: dz(rx), y: dz(ry) } : null;
  return { held, dir: dx || dy ? { x: dx, y: dy } : null, pan };
}

/** Whether frame state `s` shows any input (a button, a direction, the right stick). */
export function anyInput(s) {
  return s.held.size > 0 || !!s.dir || !!s.pan;
}

/** Edges between frames: `new Edges()`, then `step(state, now)` each frame returns `{ down: [the
 *  buttons pressed since the last frame], dir: the direction to act on now (null: none) }`. A
 *  direction acts when it changes, then repeats while held (REPEAT_DELAY, then REPEAT_EVERY). */
export class Edges {
  constructor() {
    this.held = new Set();
    this.dir = null;
    this.next = 0;
  }

  step(s, now) {
    const down = [...s.held].filter((b) => !this.held.has(b));
    this.held = new Set(s.held);
    let dir = null;
    const same = s.dir && this.dir && s.dir.x === this.dir.x && s.dir.y === this.dir.y;
    if (s.dir && !same) {
      dir = s.dir;
      this.next = now + REPEAT_DELAY;
    } else if (s.dir && now >= this.next) {
      dir = s.dir;
      this.next = now + REPEAT_EVERY;
    }
    this.dir = s.dir;
    return { down, dir };
  }

  /** Forgets what is held (a button down now is a press on the next frame it is seen down). */
  reset() {
    this.held = new Set();
    this.dir = null;
  }
}

/** The plot the map's cursor moves to from `from` ({x, y}) in screen direction `dir` ({x, y}, y
 *  down): of its eight neighbours (`map`: {w, h, wrap_x, wrap_y}), the one whose place on screen
 *  (`screenOf(x, y)` -> {x, y} CSS px, as MapView's) lies nearest that direction, within 45
 *  degrees of it; null when none does (the map's edge). The pick is by screen place, so it is
 *  right in each renderer's projection (the flat map's squares, the isometric diamonds, the 3D
 *  map's perspective). */
export function cursorStep(from, dir, map, screenOf) {
  const here = screenOf(from.x, from.y);
  if (!here) return null;
  const n = Math.hypot(dir.x, dir.y) || 1;
  const ux = dir.x / n;
  const uy = dir.y / n;
  let best = null;
  let bestCos = Math.SQRT1_2 - 1e-6;
  for (let dy = -1; dy <= 1; dy++) {
    for (let dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue;
      const x = wrap(from.x + dx, map.w, map.wrap_x);
      const y = wrap(from.y + dy, map.h, map.wrap_y);
      if (x === null || y === null) continue;
      const s = screenOf(x, y);
      if (!s) continue;
      const vx = s.x - here.x;
      const vy = s.y - here.y;
      const len = Math.hypot(vx, vy);
      if (len < 1e-6) continue;
      const cos = (vx * ux + vy * uy) / len;
      // Ties (none in the renderers' projections): the nearer on screen.
      if (cos > bestCos + 1e-9 || (best && Math.abs(cos - bestCos) <= 1e-9 && len < best.len)) {
        bestCos = cos;
        best = { x, y, len };
      }
    }
  }
  return best ? { x: best.x, y: best.y } : null;
}

function wrap(v, size, wraps) {
  if (v >= 0 && v < size) return v;
  if (!wraps) return null;
  return ((v % size) + size) % size;
}

/** The index in `rects` (each {left, top, right, bottom}, CSS px) the interface's focus moves to
 *  from rect `from` in screen direction `dir` ({x, y}, y down), or -1 when none lies that way.
 *  A candidate must lie ahead (up, down, left, right: starting past `from`'s far edge; a diagonal:
 *  its centre past `from`'s); the nearest wins, the distance across the direction counting double (a button straight ahead beats a nearer one off to the
 *  side, as in a grid). */
export function focusStep(from, dir, rects) {
  const cx = (r) => (r.left + r.right) / 2;
  const cy = (r) => (r.top + r.bottom) / 2;
  const fx = cx(from);
  const fy = cy(from);
  const horiz = dir.x !== 0 && dir.y === 0;
  const vert = dir.y !== 0 && dir.x === 0;
  let best = -1;
  let bestScore = Infinity;
  rects.forEach((r, i) => {
    const dx = cx(r) - fx;
    const dy = cy(r) - fy;
    let along;
    let across;
    if (horiz) {
      // Ahead: starting past the current one's far edge (4 px of overlap allowed, for frames).
      if (dir.x > 0 ? r.left < from.right - 4 : r.right > from.left + 4) return;
      // The gap along the row, not the centres: wide buttons next to narrow ones.
      along = Math.max(0, dir.x > 0 ? r.left - from.right : from.left - r.right);
      across = Math.max(0, Math.abs(dy) - (r.bottom - r.top + from.bottom - from.top) / 2);
    } else if (vert) {
      if (dir.y > 0 ? r.top < from.bottom - 4 : r.bottom > from.top + 4) return;
      along = Math.max(0, dir.y > 0 ? r.top - from.bottom : from.top - r.bottom);
      across = Math.max(0, Math.abs(dx) - (r.right - r.left + from.right - from.left) / 2);
    } else {
      // A diagonal: by the centres.
      const n = Math.hypot(dir.x, dir.y);
      along = (dx * dir.x + dy * dir.y) / n;
      if (along <= 0) return;
      across = Math.abs(dx * dir.y - dy * dir.x) / n;
    }
    const score = along + 2 * across;
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}
