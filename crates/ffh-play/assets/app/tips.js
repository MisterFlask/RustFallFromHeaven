// Rich tooltips: a help text in Civ4 markup (what a tech, a building, a unit, ... does) in a
// floating box while the pointer is over an element or it has the keyboard focus, in place of the
// browser's plain `title` tooltip, so that the help keeps its icons, colours and line breaks. One
// box serves the page; it follows the pointer and stays inside the window. `h(tag, {tip}, ..)`
// (dom.js) attaches one.
//
// A tip's reminder text (keywords.js `remindersNode`: its keywords' rules, docs/PLAY_DESIGN.md
// "Keywords and reminder text") is never drawn in the box: it goes to a second box of its own
// (`.tip-side`), beside the tooltip on a wide window and below it on a narrow one
// (keyword_view.js `tipLayout`).

import { render as markup, plain } from "./markup.js";
import { tipLayout, stackedLayout } from "./keyword_view.js";

let box = null;
let side = null;
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

function ensureSide() {
  if (side?.isConnected) return side;
  side = document.createElement("div");
  side.className = "tip-side";
  side.hidden = true;
  document.body.appendChild(side);
  return side;
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

/** Places the box (and its reminder boxes) near (x, y) (a pointer position), inside the window. */
function placeAt(x, y) {
  const b = ensureBox();
  const s = side && !side.hidden ? side : null;
  const vw = window.innerWidth;
  if (s) {
    s.classList.toggle("stacked", stackedLayout(vw));
    // Below the tooltip, the boxes take its width.
    s.style.width = stackedLayout(vw) && !b.hidden ? `${b.offsetWidth}px` : "";
  }
  const main = b.hidden ? { w: 0, h: 0 } : { w: b.offsetWidth, h: b.offsetHeight };
  const at = tipLayout({ x, y, vw, vh: window.innerHeight, main, side: s ? { w: s.offsetWidth, h: s.offsetHeight } : null });
  b.style.left = `${at.main.left}px`;
  b.style.top = `${at.main.top}px`;
  if (s && at.side) {
    s.style.left = `${at.side.left}px`;
    s.style.top = `${at.side.top}px`;
  }
}

function show(el, x, y) {
  const make = tips.get(el);
  if (!make) return;
  const b = ensureBox();
  owner = el;
  const frag = content(make);
  // The reminder text leaves the box for the side boxes.
  const reminders = [...frag.querySelectorAll(".tip-reminders")];
  for (const r of reminders) r.remove();
  b.replaceChildren(frag);
  const s = ensureSide();
  s.replaceChildren(...reminders);
  s.hidden = !reminders.length;
  b.hidden = !b.firstChild;
  if (b.hidden && s.hidden) return hide();
  if (x === undefined) {
    const r = el.getBoundingClientRect();
    placeAt(r.left, r.bottom);
  } else placeAt(x, y);
}

/** Shows `el`'s tip under it (a tap on a touch screen, where no pointer hovers). */
export function showTip(el) {
  show(el);
}

/** Hides the tip (of `el` only, when given). */
export function hide(el) {
  if (el && el !== owner) return;
  owner = null;
  if (box) box.hidden = true;
  if (side) side.hidden = true;
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
  el.addEventListener("mouseleave", (ev) => {
    hide(el);
    // Back over an element whose tip holds this one (a keyword chip in a build's button): its tip.
    for (let up = el.parentElement; up; up = up.parentElement) {
      if (tips.has(up)) {
        if (up.contains(ev.relatedTarget)) show(up, ev.clientX, ev.clientY);
        break;
      }
    }
  });
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
