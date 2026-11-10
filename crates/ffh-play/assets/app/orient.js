// The screen's orientation on a phone (docs/HUMAN.md, "Upright, on its side, locked"): the page
// follows the phone (the layouts for a phone upright and on its side, style.css and iso.css), or
// keeps the one the player locks it to in Options (`orientChoice`: "auto", "landscape",
// "portrait"; remembered by this browser).
//
// A lock is the browser's where it has one (the Screen Orientation API's `lock`, which Android's
// browsers grant in fullscreen: choosing a lock enters fullscreen, and the first tap of a later
// page does). Where it has none (an iPhone's Safari), or after fullscreen ends, the page turns
// itself: while the phone's orientation is not the one locked, the body is turned 90 degrees so its
// content stays put on the phone, and the page is shown a turned viewport, so its own layout code
// lays out the locked orientation without knowing: the stylesheets' media queries and viewport
// units swap width and height (through the CSSOM), and `innerWidth`/`innerHeight`, `matchMedia`,
// pointer and touch coordinates, `getBoundingClientRect` and `elementFromPoint` answer in the
// turned axes. Taps reach the right element in any case: the browser hit-tests through the turn.
// Only a device whose main pointer is a finger (`pointer: coarse`) is turned; elsewhere the choice
// is not offered and nothing here changes anything.

import { coarse } from "./dom.js";

const KEY = "ffh-orientation";
const CHOICES = ["auto", "landscape", "portrait"];

const O = {
  /** The content's turn, CSS degrees clockwise: 0, 90 or -90. */
  rot: 0,
  /** The physical viewport while turned (CSS px). */
  W: 0,
  H: 0,
  /** The angle of the last landscape the browser showed (Screen Orientation `angle`: 90 or 270),
   *  so a turn keeps the content where it was on the phone. */
  landscapeAngle: 90,
  /** The page has tried the browser's lock (once a page, at the first tap). */
  tried: false,
  /** The lock the browser granted, and whether the page entered fullscreen for it. */
  locked: false,
  fullscreen: false,
  installed: false,
};

/** The browser's own answers, before the turn's shims. */
const raw = {};

// ---------------------------------------------------------------------------------------------
// The choice

/** The orientation chosen in Options: "auto" (follow the phone), "landscape" or "portrait". */
export function orientChoice() {
  try {
    const v = globalThis.localStorage?.getItem(KEY);
    if (CHOICES.includes(v)) return v;
  } catch {
    /* no storage */
  }
  return "auto";
}

/** Chooses the orientation (from a tap in Options: the browser's lock needs that gesture). */
export async function setOrientChoice(v) {
  if (!CHOICES.includes(v)) return;
  try {
    globalThis.localStorage?.setItem(KEY, v);
  } catch {
    /* kept for this page only: applied below all the same */
  }
  O.tried = true;
  await browserLock(v);
  apply();
}

/** Whether the orientation can be chosen here (a finger-first device). */
export function orientOffered() {
  return coarse();
}

/** The state, for the tests: the content's turn and the conversion of a point of the turned
 *  viewport to the screen's. */
export function orientState() {
  return { choice: orientChoice(), rot: O.rot, locked: O.locked, portrait: physicalPortrait(), toScreen: (x, y) => toPhysical(x, y) };
}

// ---------------------------------------------------------------------------------------------
// The browser's lock

async function browserLock(v) {
  const so = globalThis.screen?.orientation;
  if (v === "auto") {
    O.locked = false;
    try {
      so?.unlock?.();
    } catch {
      /* nothing locked */
    }
    // The fullscreen entered for the lock ends with it.
    if (O.fullscreen && document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
    O.fullscreen = false;
    return;
  }
  if (!so?.lock) return;
  const el = document.documentElement;
  let entered = false;
  try {
    // Android's browsers lock only in fullscreen.
    if (!document.fullscreenElement && el.requestFullscreen) {
      await el.requestFullscreen({ navigationUI: "hide" });
      entered = true;
    }
    await so.lock(v);
    O.locked = true;
    O.fullscreen ||= entered;
  } catch {
    O.locked = false;
    // No lock here: the page turns itself; the fullscreen entered for the lock ends.
    if (entered && document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
  }
}

// ---------------------------------------------------------------------------------------------
// Following the phone

function physicalPortrait() {
  const m = raw.matchMedia?.call(window, "(orientation: portrait)");
  if (m) return m.matches;
  return raw.innerHeight() >= raw.innerWidth();
}

function screenAngle() {
  const a = globalThis.screen?.orientation?.angle;
  if (typeof a === "number") return ((a % 360) + 360) % 360;
  const w = globalThis.orientation;
  return typeof w === "number" ? ((w % 360) + 360) % 360 : 0;
}

/** Turns the content when the phone's orientation is not the one locked; else leaves it. */
function apply() {
  const choice = orientChoice();
  const portrait = physicalPortrait();
  if (!portrait) {
    const a = screenAngle();
    if (a === 90 || a === 270) O.landscapeAngle = a;
  }
  let rot = 0;
  if (choice !== "auto" && coarse() && (choice === "portrait") !== portrait) {
    // The turn that brings the content back to where it was on the phone: the locked
    // orientation's angle less the one the browser shows now.
    const want = choice === "portrait" ? 0 : O.landscapeAngle;
    const d = (((want - screenAngle()) % 360) + 360) % 360;
    rot = d === 270 ? -90 : 90;
  }
  setRot(rot);
}

function setRot(rot) {
  const W = raw.innerWidth();
  const H = raw.innerHeight();
  if (rot === O.rot && (!rot || (W === O.W && H === O.H))) return;
  const was = O.rot;
  O.rot = rot;
  O.W = W;
  O.H = H;
  const root = document.documentElement;
  const body = document.body;
  root.classList.toggle("turned", !!rot);
  if (rot) {
    // The body, a W x H screen's content turned: H wide and W high, turned about its corner and
    // moved back onto the screen.
    Object.assign(body.style, {
      position: "fixed",
      left: "0",
      top: "0",
      width: `${H}px`,
      height: `${W}px`,
      transformOrigin: "0 0",
      transform: rot === 90 ? `translate(${W}px, 0) rotate(90deg)` : `translate(0, ${H}px) rotate(-90deg)`,
    });
  } else {
    for (const p of ["position", "left", "top", "width", "height", "transformOrigin", "transform"]) body.style[p] = "";
  }
  if (!!was !== !!rot) swapSheets(!!rot);
  // The page lays itself out again (MapView and the panels follow their own sizes; the rest
  // listens for a resize).
  window.dispatchEvent(new Event("resize"));
}

// ---------------------------------------------------------------------------------------------
// The turned viewport: coordinates

/** A point of the screen (CSS px) in the turned viewport. */
function toVirtual(px, py) {
  if (O.rot === 90) return { x: py, y: O.W - px };
  if (O.rot === -90) return { x: O.H - py, y: px };
  return { x: px, y: py };
}

/** A point of the turned viewport on the screen. */
function toPhysical(vx, vy) {
  if (O.rot === 90) return { x: O.W - vy, y: vx };
  if (O.rot === -90) return { x: vy, y: O.H - vx };
  return { x: vx, y: vy };
}

function getter(obj, name) {
  for (let o = obj; o; o = Object.getPrototypeOf(o)) {
    const d = Object.getOwnPropertyDescriptor(o, name);
    if (d) return d;
  }
  return null;
}

/** Puts the turned viewport's answers in front of the browser's (once; they pass the browser's
 *  answers through while the content is not turned). */
function install() {
  if (O.installed) return;
  O.installed = true;
  const iw = getter(window, "innerWidth");
  const ih = getter(window, "innerHeight");
  raw.innerWidth = () => (iw?.get ? iw.get.call(window) : document.documentElement.clientWidth);
  raw.innerHeight = () => (ih?.get ? ih.get.call(window) : document.documentElement.clientHeight);
  raw.matchMedia = window.matchMedia;
  try {
    Object.defineProperty(window, "innerWidth", { configurable: true, get: () => (O.rot ? raw.innerHeight() : raw.innerWidth()) });
    Object.defineProperty(window, "innerHeight", { configurable: true, get: () => (O.rot ? raw.innerWidth() : raw.innerHeight()) });
  } catch {
    /* not replaceable here: the layout code mostly reads element sizes */
  }
  if (raw.matchMedia) window.matchMedia = (q) => raw.matchMedia.call(window, O.rot ? swapMedia(String(q)) : q);
  // Pointer, mouse, wheel and touch coordinates.
  const coord = (proto, nx, ny) => {
    const dx = getter(proto, nx);
    const dy = getter(proto, ny);
    if (!dx?.get || !dy?.get) return;
    Object.defineProperty(proto, nx, { configurable: true, enumerable: dx.enumerable, get() {
      const x = dx.get.call(this);
      return O.rot ? toVirtual(x, dy.get.call(this)).x : x;
    } });
    Object.defineProperty(proto, ny, { configurable: true, enumerable: dy.enumerable, get() {
      const y = dy.get.call(this);
      return O.rot ? toVirtual(dx.get.call(this), y).y : y;
    } });
  };
  if (globalThis.MouseEvent) coord(MouseEvent.prototype, "clientX", "clientY");
  if (globalThis.Touch) coord(Touch.prototype, "clientX", "clientY");
  // Element rectangles.
  const rect = Element.prototype.getBoundingClientRect;
  Element.prototype.getBoundingClientRect = function () {
    const r = rect.call(this);
    if (!O.rot) return r;
    const a = toVirtual(r.left, r.top);
    const b = toVirtual(r.right, r.bottom);
    const x = Math.min(a.x, b.x);
    const y = Math.min(a.y, b.y);
    return new DOMRect(x, y, Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  };
  const at = Document.prototype.elementFromPoint;
  Document.prototype.elementFromPoint = function (x, y) {
    if (!O.rot) return at.call(this, x, y);
    const p = toPhysical(x, y);
    return at.call(this, p.x, p.y);
  };
}

// ---------------------------------------------------------------------------------------------
// The turned viewport: the stylesheets

/** A media query or a value with width and height swapped (and portrait and landscape, and the
 *  viewport units). */
function swapMedia(q) {
  return q.replace(/\b(width|height|portrait|landscape)\b/g, (m) => ({ width: "height", height: "width", portrait: "landscape", landscape: "portrait" })[m]);
}
const UNIT = /(\d|\.)(d|s|l)?v(w|h)\b/g;
const swapUnits = (v) => v.replace(UNIT, (m, d, k, a) => `${d}${k ?? ""}v${a === "w" ? "h" : "w"}`);

/** The rules changed while turned: rule -> its media text, or its declarations with viewport
 *  units ([prop, value, priority]), as written. */
const originals = new Map();

function eachRule(list, f) {
  for (const r of list ?? []) {
    f(r);
    let inner = null;
    try {
      inner = r.cssRules;
    } catch {
      inner = null;
    }
    if (inner?.length) eachRule(inner, f);
  }
}

function swapRule(r) {
  if (originals.has(r)) return;
  if (globalThis.CSSMediaRule && r instanceof CSSMediaRule) {
    const t = r.media.mediaText;
    const s = swapMedia(t);
    if (s !== t) {
      originals.set(r, t);
      r.media.mediaText = s;
    }
    return;
  }
  const st = r.style;
  if (!st) return;
  const decls = [];
  for (let i = 0; i < st.length; i++) {
    const p = st[i];
    const v = st.getPropertyValue(p);
    if (UNIT.test(v)) decls.push([p, v, st.getPropertyPriority(p)]);
    UNIT.lastIndex = 0;
  }
  if (!decls.length) return;
  originals.set(r, decls);
  for (const [p, v, pr] of decls) st.setProperty(p, swapUnits(v), pr);
}

function restoreRule(r, o) {
  if (typeof o === "string") r.media.mediaText = o;
  else for (const [p, v, pr] of o) r.style.setProperty(p, v, pr);
}

function sheetRules(s) {
  try {
    return s.cssRules;
  } catch {
    return null;
  }
}

function swapSheets(on) {
  if (!on) {
    for (const [r, o] of originals) restoreRule(r, o);
    originals.clear();
    return;
  }
  for (const s of document.styleSheets) eachRule(sheetRules(s), swapRule);
}

// ---------------------------------------------------------------------------------------------

/** Starts following the phone (main.js, first). */
export function initOrientation() {
  if (!coarse()) {
    // A computer: no turn, no shims.
    raw.innerWidth = () => window.innerWidth;
    raw.innerHeight = () => window.innerHeight;
    raw.matchMedia = window.matchMedia;
    return;
  }
  install();
  const again = () => apply();
  raw.matchMedia?.call(window, "(orientation: portrait)").addEventListener?.("change", again);
  // (The browser's resizes: the page's own, sent above after a turn, are not.)
  window.addEventListener("resize", (ev) => ev.isTrusted && again());
  globalThis.screen?.orientation?.addEventListener?.("change", again);
  // Fullscreen ended (the back gesture): the browser's lock ended with it; the page turns itself.
  document.addEventListener("fullscreenchange", () => {
    if (document.fullscreenElement) return;
    O.locked = false;
    O.fullscreen = false;
    again();
  });
  // Stylesheets added while turned (the art's, the battle screens'): turned too.
  new MutationObserver(() => {
    if (O.rot) for (const s of document.styleSheets) eachRule(sheetRules(s), swapRule);
  }).observe(document.head, { childList: true, subtree: true });
  document.addEventListener("load", (ev) => O.rot && ev.target?.sheet && eachRule(sheetRules(ev.target.sheet), swapRule), true);
  // A lock chosen on an earlier page: the browser's lock at the first tap (it needs one).
  window.addEventListener(
    "pointerdown",
    () => {
      if (O.tried) return;
      O.tried = true;
      const c = orientChoice();
      if (c !== "auto" && globalThis.screen?.orientation?.lock) browserLock(c).then(apply);
    },
    { capture: true, once: true },
  );
  apply();
}
