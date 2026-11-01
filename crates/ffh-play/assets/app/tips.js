// Rich tooltips: a help text in Civ4 markup (what a tech, a building, a unit, ... does) in a
// floating box while the pointer is over an element or it has the keyboard focus, in place of the
// browser's plain `title` tooltip, so that the help keeps its icons, colours and line breaks. One
// box serves the page; it follows the pointer and stays inside the window. `h(tag, {tip}, ..)`
// (dom.js) attaches one.

import { render as markup, plain } from "./markup.js";

let box = null;
/** The element whose tip is shown, and the content maker of each element with a tip. */
let owner = null;
const tips = new WeakMap();

function ensureBox() {
  if (box?.isConnected) return box;
  box = document.createElement("div");
  box.className = "tooltip richtip";
  box.hidden = true;
  box.setAttribute("role", "tooltip");
  document.body.appendChild(box);
  return box;
}

/** The content of a tip: a markup string, a Node, or an array of them (null entries skipped). */
function content(c) {
  const v = typeof c === "function" ? c() : c;
  const frag = document.createDocumentFragment();
  for (const part of [v].flat(Infinity)) {
    if (part === null || part === undefined || part === false || part === "") continue;
    frag.appendChild(part instanceof Node ? part : markup(String(part)));
  }
  return frag;
}

/** Places the box near (x, y) (a pointer position), inside the window. */
function placeAt(x, y) {
  const b = ensureBox();
  const pad = 14;
  const w = b.offsetWidth;
  const hgt = b.offsetHeight;
  let left = x + pad;
  let top = y + pad;
  if (left + w > window.innerWidth - 4) left = Math.max(4, x - pad - w);
  if (top + hgt > window.innerHeight - 4) top = Math.max(4, y - pad - hgt);
  b.style.left = `${left}px`;
  b.style.top = `${top}px`;
}

function show(el, x, y) {
  const make = tips.get(el);
  if (!make) return;
  const b = ensureBox();
  owner = el;
  b.replaceChildren(content(make));
  if (!b.firstChild) return hide();
  b.hidden = false;
  if (x === undefined) {
    const r = el.getBoundingClientRect();
    placeAt(r.left, r.bottom);
  } else placeAt(x, y);
}

/** Hides the tip (of `el` only, when given). */
export function hide(el) {
  if (el && el !== owner) return;
  owner = null;
  if (box) box.hidden = true;
}

/** Attaches tip `c` (a markup string, a Node, an array of them, or a function making one when the
 *  tip shows) to `el`; returns `el`. The plain text also becomes the element's accessible
 *  description. A falsy `c` attaches nothing. */
export function tip(el, c) {
  if (!c) return el;
  const first = !tips.has(el);
  tips.set(el, c);
  if (typeof c === "string") el.setAttribute("aria-description", plain(c).slice(0, 500));
  if (!first) return el;
  el.addEventListener("mouseenter", (ev) => show(el, ev.clientX, ev.clientY));
  el.addEventListener("mousemove", (ev) => owner === el && placeAt(ev.clientX, ev.clientY));
  el.addEventListener("mouseleave", () => hide(el));
  el.addEventListener("focus", () => show(el));
  el.addEventListener("blur", () => hide(el));
  el.addEventListener("mousedown", () => hide(el));
  return el;
}

// A tip whose element went away (a dialog redrawn under the pointer) does not stay up.
if (typeof document !== "undefined") {
  document.addEventListener("mousemove", () => owner && !owner.isConnected && hide(), { passive: true });
  document.addEventListener("keydown", (ev) => ev.key === "Escape" && hide(), true);
  window.addEventListener?.("blur", () => hide());
}
