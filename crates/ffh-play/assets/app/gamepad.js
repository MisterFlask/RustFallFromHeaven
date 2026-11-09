// A game controller on a phone (docs/HUMAN.md, "A controller on a phone"): the W3C Gamepad API,
// standard mapping only, on a device whose main pointer is a finger (`pointer: coarse`). Nothing
// here runs on a computer (a fine pointer) or on a phone without a controller: the poll starts when
// the browser reports a pad, and the controller's mode starts at its first input there. A finger
// on the screen or a key on a keyboard ends the mode (the last input wins); the pad's next input
// starts it again.
//
// In the mode the map has a cursor (a plot, moved by the left stick or the D-pad by screen
// direction, with the mouse's hover there: outline, tooltip, path and odds), and every other
// layer -- the battle screens, the dialogs, the title screen, the folded menu, the interface
// around the map (Y) -- has a focus moved between its buttons by direction. The buttons go through
// the client's own paths: the map's click and right-click handlers, the page's key handler (synthetic
// Escape, Tab, Space, C), and clicks on the interface's buttons. pad.js holds the pure parts.

import { readPad, anyInput, standard, Edges, BTN, cursorStep, focusStep } from "./pad.js";
import { S, on, ownUnitsAt } from "./state.js";
import * as units from "./units.js";
import { coarse, toast } from "./dom.js";
import { titleShown } from "./title.js";

const P = {
  view: null,
  /** The mode is on (the body's `pad-mode` class, MapView's `pad`). */
  active: false,
  raf: 0,
  edges: new Edges(),
  /** The map cursor's plot, {x, y}. */
  cursor: null,
  /** The interface around the map has the focus (Y; B gives it back to the map). */
  hud: false,
  /** The layer the controller acts on now: {kind, root} (`scopeNow`). */
  scope: null,
  /** The focused element in a layer, and where it was (a re-render replaces it: the nearest
   *  element to that place takes the focus). */
  focus: null,
  focusRect: null,
  /** performance.now() of the pad's last input, and of the last frame. */
  lastInput: -1e9,
  lastFrame: 0,
  /** The camera the cursor was placed under (MapView replaces `cam` on every change). */
  cam: null,
  /** The right stick panned in the last frame. */
  panning: false,
  cursorEl: null,
  told: false,
  /** The element a layer entered next takes the focus on (the menu's first button, `openMenu`). */
  want: null,
};

/** The focusable elements of a layer. */
const FOCUSABLE = "button:not([disabled]), a[href], input:not([disabled]):not([type=hidden]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";
/** What a layer around the map leaves out: the layers above it, and the map itself. */
const NOT_HUD = "#modals, #title-screen, #battle, #toasts, canvas";

/** Whether the controller's mode is on. */
export function padActive() {
  return P.active;
}

/** Starts listening for controllers (main.js boot). `view`: the MapView. */
export function initGamepad(view) {
  P.view = view;
  if (typeof navigator === "undefined" || !navigator.getGamepads) return;
  window.addEventListener("gamepadconnected", () => poll());
  // A finger, a mouse or a key (a real one: not the synthetic keys sent below) ends the mode. A
  // key that comes with the pad's own input (a browser passing a pad's D-pad on as arrows) does
  // not.
  const leave = (ev) => P.active && ev.isTrusted && performance.now() - P.lastInput > 400 && stop();
  window.addEventListener("pointerdown", leave, { capture: true });
  window.addEventListener("keydown", leave, { capture: true });
  on("selection", () => {
    if (P.active && S.sel && P.scope?.kind === "map") setCursor({ x: S.sel.x, y: S.sel.y }, { reveal: false });
  });
  // A pad already known to the page (its button was pressed before this script ran).
  poll();
}

function pads() {
  try {
    return [...(navigator.getGamepads() ?? [])].filter(standard);
  } catch {
    return [];
  }
}

/** Runs the poll while a standard pad is connected and the device's main pointer is a finger. */
function poll() {
  if (P.raf || !coarse() || !pads().length) return;
  P.lastFrame = performance.now();
  P.raf = requestAnimationFrame(frame);
}

function frame(now) {
  P.raf = 0;
  const list = pads();
  if (!list.length) {
    if (P.active) stop();
    return;
  }
  const dt = Math.min(0.1, Math.max(0, (now - P.lastFrame) / 1000));
  P.lastFrame = now;
  // The pad with input this frame (two pads: either drives).
  const states = list.map(readPad);
  const s = states.find(anyInput) ?? states[0];
  if (anyInput(s)) P.lastInput = now;
  if (!P.active) {
    // The first input starts the mode and does nothing else.
    if (anyInput(s) && coarse()) start();
    P.edges.step(s, now);
  } else {
    const e = P.edges.step(s, now);
    try {
      run(s, e, dt);
    } catch (err) {
      console.warn("gamepad:", err);
    }
  }
  P.raf = requestAnimationFrame(frame);
}

function start() {
  P.active = true;
  P.view.pad = true;
  P.scope = null;
  document.body.classList.add("pad-mode");
  if (!P.told) {
    P.told = true;
    toast("Controller on. Its buttons: Menu (Back), Controls.", "info", 4000);
  }
}

function stop() {
  P.active = false;
  P.view.pad = false;
  P.hud = false;
  document.body.classList.remove("pad-mode");
  setFocus(null);
  if (P.scope?.kind === "map") P.view.clearHover();
  P.scope = null;
  showCursor(false);
}

// ---------------------------------------------------------------------------------------------
// Layers

/** The layer the controller acts on: the battle screens, the top dialog, the title screen, the
 *  folded menu, the interface (after Y), else the map. */
function scopeNow() {
  const battle = document.getElementById("battle");
  if (battle) return { kind: "battle", root: battle };
  const tops = document.querySelectorAll("#modals > .backdrop:not(.under) > .modal");
  if (tops.length) return { kind: "modal", root: tops[tops.length - 1] };
  if (titleShown()) return { kind: "title", root: document.getElementById("title-screen") };
  if (document.body.classList.contains("menu-open")) return { kind: "menu", root: document.getElementById("menu") };
  if (P.hud) return { kind: "hud", root: document.body };
  return { kind: "map", root: null };
}

function run(s, e, dt) {
  const sc = scopeNow();
  if (sc.kind !== P.scope?.kind || sc.root !== P.scope?.root) enterScope(sc);
  if (sc.kind === "map") return runMap(s, e, dt);
  runLayer(sc, s, e, dt);
}

function enterScope(sc) {
  const was = P.scope;
  P.scope = sc;
  if (was?.kind === "map") {
    P.view.clearHover();
    showCursor(false);
  }
  if (sc.kind === "map") {
    setFocus(null);
    if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body) document.activeElement.blur();
    const sel = S.sel ? { x: S.sel.x, y: S.sel.y } : null;
    setCursor(P.cursor ?? sel ?? P.view.center(), { reveal: false });
    return;
  }
  // A layer: what it focused itself (a dialog its primary button), else its first button.
  const els = focusables(sc);
  const act = document.activeElement;
  const want = P.want;
  P.want = null;
  const first =
    (want && els.includes(want) && want) ||
    (act && els.includes(act) && act) ||
    (sc.kind === "hud" && els.find((el) => el.matches("#unit-panel button.action"))) ||
    (sc.kind === "hud" && els.find((el) => el.closest("#unit-panel"))) ||
    (sc.kind === "hud" && els.find((el) => el.id === "end-turn")) ||
    els.find((el) => el.matches(".primary")) ||
    els[0] ||
    null;
  setFocus(first);
}

// ---------------------------------------------------------------------------------------------
// The map

function runMap(s, e, dt) {
  const v = P.view;
  if (!v.map) return;
  if (!P.cursor) setCursor(v.center(), { reveal: false });
  if (e.dir && P.cursor) {
    const next = cursorStep(P.cursor, e.dir, v.map, (x, y) => v.screenOf(x, y));
    if (next) setCursor(next);
  }
  if (s.pan) {
    const speed = v.scrollSpeed() * 1.25;
    v.moveView(s.pan.x * speed * dt, s.pan.y * speed * dt);
    P.panning = true;
  } else if (P.panning) {
    // The view panned away from the cursor: the cursor comes to the view's centre.
    P.panning = false;
    const at = P.cursor && v.screenOf(P.cursor.x, P.cursor.y);
    const r = v.clearRect();
    if (!at || at.x < r.x0 || at.y < r.y0 || at.x > r.x1 || at.y > r.y1) {
      const c = v.center();
      if (c) setCursor(c, { reveal: false });
    }
  }
  for (const b of e.down) {
    switch (b) {
      case BTN.a:
        if (P.cursor) act(P.cursor.x, P.cursor.y);
        break;
      case BTN.b:
        key("Escape");
        break;
      case BTN.x:
        key(" ", { code: "Space" });
        break;
      case BTN.y:
        P.hud = true;
        break;
      case BTN.lb:
        key("Tab", { code: "Tab", shiftKey: true });
        break;
      case BTN.rb:
        key("Tab", { code: "Tab" });
        break;
      case BTN.lt:
      case BTN.rt: {
        const at = P.cursor && v.screenOf(P.cursor.x, P.cursor.y);
        v.zoom(b === BTN.rt ? 1 : -1, at?.x ?? v.vw / 2, at?.y ?? v.vh / 2);
        break;
      }
      case BTN.start:
        endTurn();
        break;
      case BTN.back:
        openMenu();
        break;
      case BTN.ls:
      case BTN.rs:
        if (S.sel) setCursor({ x: S.sel.x, y: S.sel.y });
        key("c", { code: "KeyC" });
        break;
    }
  }
  // The camera moved (a glide, a zoom, the stick): the cursor and the tooltip follow the plot.
  if (v.cam !== P.cam) {
    P.cam = v.cam;
    placeCursor();
  }
}

/** A at plot (x, y): in a plot pick, picks it; with units selected and none of ours there, the
 *  right-click's move (or attack: its preview was the cursor's hover); else the click (select
 *  the units there, open our city, show the plot). */
function act(x, y) {
  const h = P.view.h;
  const ev = { pointerType: "pad" };
  if (units.currentMode()) return h.click?.(x, y, ev);
  if (S.sel && S.status?.phase === "human" && !ownUnitsAt(x, y).length) return h.rightClick?.(x, y, ev);
  h.click?.(x, y, ev);
}

function setCursor(p, { reveal = true } = {}) {
  if (!p) return;
  P.cursor = { x: p.x, y: p.y };
  if (reveal) P.view.reveal(p.x, p.y);
  P.view.hoverPlot(p.x, p.y);
  placeCursor();
}

/** The cursor's ring over the plot (the map's own hover outline is drawn by its renderer). */
function placeCursor() {
  const v = P.view;
  const s = P.cursor && P.scope?.kind === "map" ? v.screenOf(P.cursor.x, P.cursor.y) : null;
  if (!s) return showCursor(false);
  if (!P.cursorEl) {
    P.cursorEl = document.createElement("div");
    P.cursorEl.className = "pad-cursor";
    P.cursorEl.setAttribute("aria-hidden", "true");
    v.wrap.appendChild(P.cursorEl);
  }
  const c = v.canvas.getBoundingClientRect();
  const r = v.wrap.getBoundingClientRect();
  const size = Math.round(Math.max(28, Math.min(120, (s.ts ?? 48) * 0.8)));
  const el = P.cursorEl;
  el.style.width = el.style.height = `${size}px`;
  el.style.transform = `translate(${Math.round(s.x + c.left - r.left - size / 2)}px, ${Math.round(s.y + c.top - r.top - size / 2)}px)`;
  showCursor(true);
  // The tooltip stays by the plot.
  if (!v.tip.hidden) v.hoverPlot(P.cursor.x, P.cursor.y);
}

function showCursor(on) {
  if (P.cursorEl) P.cursorEl.hidden = !on;
}

// ---------------------------------------------------------------------------------------------
// Layers: the focus between buttons

function runLayer(sc, s, e, dt) {
  // The focused element went (a re-render, a closed section): the one nearest its place.
  if (!P.focus || !P.focus.isConnected || !focusables(sc).includes(P.focus)) {
    const els = focusables(sc);
    setFocus(P.focusRect ? nearest(els, P.focusRect) : (els.find((el) => el.matches(".primary")) ?? els[0] ?? null));
  }
  if (e.dir) moveFocus(sc, e.dir);
  if (s.pan) scrollBy(sc, s.pan, dt);
  for (const b of e.down) {
    switch (b) {
      case BTN.a:
        press(sc);
        break;
      case BTN.b:
        if (sc.kind === "hud") P.hud = false;
        else if (document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLTextAreaElement) document.activeElement.blur();
        else key("Escape");
        break;
      case BTN.back:
        if (sc.kind === "menu") key("Escape");
        else if (sc.kind === "hud") openMenu();
        break;
      case BTN.start:
        if (sc.kind === "hud") endTurn();
        break;
    }
  }
}

/** The layer's focusable elements that can be seen. */
function focusables(sc) {
  const all = [...sc.root.querySelectorAll(FOCUSABLE)];
  return all.filter((el) => {
    if (sc.kind === "hud" && el.closest(NOT_HUD)) return false;
    if (el.closest("[hidden], [inert]")) return false;
    if (el.checkVisibility && !el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return false;
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return false;
    // The interface around the map: only what is on screen (the layers scroll inside themselves).
    if (sc.kind === "hud" && (r.right <= 0 || r.bottom <= 0 || r.left >= innerWidth || r.top >= innerHeight)) return false;
    return true;
  });
}

function nearest(els, at) {
  const cx = (at.left + at.right) / 2;
  const cy = (at.top + at.bottom) / 2;
  let best = null;
  let bd = Infinity;
  for (const el of els) {
    const r = el.getBoundingClientRect();
    const d = Math.hypot((r.left + r.right) / 2 - cx, (r.top + r.bottom) / 2 - cy);
    if (d < bd) {
      bd = d;
      best = el;
    }
  }
  return best;
}

function setFocus(el) {
  if (P.focus && P.focus !== el) P.focus.classList.remove("pad-focus");
  P.focus = el;
  if (!el) return;
  el.classList.add("pad-focus");
  // (A text field takes the focus only on A: on a phone the focus brings the keyboard up.)
  const text = (el instanceof HTMLInputElement && !["button", "submit", "checkbox", "radio", "range", "color", "file"].includes(el.type)) || el instanceof HTMLTextAreaElement;
  if (!text) el.focus({ preventScroll: true });
  el.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  P.focusRect = rectOf(el);
}

function rectOf(el) {
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
}

function moveFocus(sc, dir) {
  const els = focusables(sc).filter((el) => el !== P.focus);
  if (!P.focus) return setFocus(els[0] ?? null);
  const i = focusStep(rectOf(P.focus), dir, els.map(rectOf));
  if (i >= 0) return setFocus(els[i]);
  // Nothing that way on screen: the list scrolls, if it can.
  const box = scroller(P.focus);
  if (box) box.scrollBy({ left: dir.x * box.clientWidth * 0.6, top: dir.y * box.clientHeight * 0.6 });
}

function scroller(el) {
  for (let n = el?.parentElement; n && n !== document.body; n = n.parentElement) {
    const st = getComputedStyle(n);
    if ((/(auto|scroll)/.test(st.overflowY) && n.scrollHeight > n.clientHeight + 1) || (/(auto|scroll)/.test(st.overflowX) && n.scrollWidth > n.clientWidth + 1)) return n;
  }
  return null;
}

function scrollBy(sc, pan, dt) {
  const box = scroller(P.focus) ?? (sc.root !== document.body ? sc.root : null);
  if (!box) return;
  const speed = 900;
  box.scrollBy(pan.x * speed * dt, pan.y * speed * dt);
}

function press(sc) {
  const el = P.focus;
  if (!el) return;
  if (el instanceof HTMLSelectElement) {
    try {
      el.focus();
      el.showPicker?.();
    } catch {
      /* no picker from script here: the focus at least */
    }
    return;
  }
  if ((el instanceof HTMLInputElement && !["button", "submit", "checkbox", "radio", "range", "color", "file"].includes(el.type)) || el instanceof HTMLTextAreaElement) {
    el.focus();
    return;
  }
  // An order or a unit on the selection's card goes back to the map (its next unit, its plot pick).
  const card = sc.kind === "hud" && !!el.closest("#unit-panel");
  el.click();
  if (card) P.hud = false;
}

// ---------------------------------------------------------------------------------------------
// The page's own controls

/** A key press for the page's key handler (main.js `onKey`, and the battle screens' while they
 *  are open). */
function key(k, { code = k, shiftKey = false } = {}) {
  document.dispatchEvent(new KeyboardEvent("keydown", { key: k, code, shiftKey, bubbles: true, cancelable: true }));
}

function endTurn() {
  const b = document.getElementById("end-turn");
  if (b && !b.disabled) b.click();
}

/** The menu: the folded menu's ☰ (a phone), else the interface's focus on the menu's buttons. */
function openMenu() {
  const b = document.getElementById("btn-menu");
  if (b && b.getBoundingClientRect().width > 0 && getComputedStyle(b).display !== "none") {
    P.hud = false;
    b.click();
    return;
  }
  const first = document.querySelector("#menu button:not([disabled]):not([hidden])");
  if (P.hud && P.scope?.kind === "hud") return first && setFocus(first);
  P.hud = true;
  P.want = first;
}
