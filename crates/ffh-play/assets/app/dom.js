// DOM helpers shared by the screens: an element builder, toasts, and the busy indicator.

import { tip } from "./tips.js";

/** How the short texts of buttons and headings are drawn: `null` (plain text), or a function
 *  `(tagName, text, el) => Node | string` that look.js sets once the art has loaded (the bitmap
 *  fonts of docs/ART.md 11.6; the text stays the element's `textContent`). */
let textHook = null;

export function setTextHook(f) {
  textHook = f;
}

/** The icon of a game type (`(kind, index, size) => Node | null`), set by look.js once the art
 *  has loaded; `typeIconOf` answers null before that (and in the node tests). Kinds: those of
 *  look.js `typeIcon` and "civicoption" (the civic categories). */
let iconOf = null;

export function setTypeIconHook(f) {
  iconOf = f;
}

export function typeIconOf(kind, index, size = 16) {
  return iconOf?.(kind, index, size) ?? null;
}

/** The elements whose text children go through the text hook. */
const HOOKED = new Set(["BUTTON", "H2", "H3", "H4", "KBD"]);

/** Replaces an element's content with `text`, through the text hook as `h` would. */
export function setText(el, text) {
  el.replaceChildren(textHook && HOOKED.has(el.tagName) ? textHook(el.tagName, String(text), el) : String(text));
}

/** `h("button.primary#end", {onclick, title}, "End turn", child...)`: an element with classes
 *  and id from the tag string, properties or attributes from `props` (`on*` are listeners,
 *  `data-*` and `aria-*` attributes, `style` an object, `tip` a rich tooltip: tips.js), and text
 *  or node children (null and false are skipped, arrays flattened). The text children of buttons
 *  and headings go through the text hook (`setTextHook`). */
export function h(tag, props = {}, ...children) {
  const m = /^([a-z0-9]+)?((?:[.#][\w-]+)*)$/i.exec(tag);
  const el = document.createElement(m?.[1] || "div");
  for (const part of (m?.[2] ?? "").match(/[.#][\w-]+/g) ?? []) {
    if (part[0] === ".") el.classList.add(part.slice(1));
    else el.id = part.slice(1);
  }
  if (props && (typeof props !== "object" || props instanceof Node || Array.isArray(props))) {
    children.unshift(props);
    props = {};
  }
  for (const [k, v] of Object.entries(props ?? {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
    else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
    else if (k === "tip") tip(el, v);
    else if (k === "class") el.className += ` ${v}`;
    else if (k.startsWith("data-") || k.startsWith("aria-") || k === "role" || k === "for") el.setAttribute(k, v === true ? "" : v);
    else el[k] = v;
  }
  append(el, children);
  return el;
}

function append(el, children) {
  const hook = textHook && HOOKED.has(el.tagName) ? textHook : null;
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(el, c);
    else if (c instanceof Node) el.appendChild(c);
    else if (hook) {
      const n = hook(el.tagName, String(c), el);
      el.appendChild(n instanceof Node ? n : document.createTextNode(String(n)));
    } else el.appendChild(document.createTextNode(String(c)));
  }
}

export const $ = (sel, root = document) => root.querySelector(sel);

/** Whether the device's main pointer is a finger (`pointer: coarse`): the screens then offer
 *  on-screen choices for what Shift and Ctrl do with a mouse. */
export function coarse() {
  return globalThis.matchMedia?.("(pointer: coarse)").matches ?? false;
}

/** The small-screen layout's media query (style.css, iso.css): a phone upright (narrow) or on
 *  its side (short). */
export const COMPACT = "(max-width: 760px), (max-height: 520px)";

/** Whether the page has the small-screen layout (a phone). */
export function compactScreen() {
  return globalThis.matchMedia?.(COMPACT).matches ?? false;
}

/** A row of toggle buttons choosing what a tap does, for touch screens (shown only there, by
 *  style.css: `.tap-mode`): `options` are `[value, label, title]`; `set(value)` is called with the
 *  one picked. */
export function tapModes(current, options, set) {
  return h(
    "div.tap-mode",
    { role: "group", "aria-label": "What a tap does" },
    h("span.dim", "A tap:"),
    options.map(([v, label, title]) => h("button.small", { class: v === current ? "on" : "", "aria-pressed": String(v === current), "data-tap": v, title, onclick: () => set(v) }, label)),
  );
}

/** Replaces an element's children. */
export function fill(el, ...children) {
  el.replaceChildren();
  append(el, children);
  return el;
}

/** The most toasts shown at once (the oldest goes first). */
const TOASTS_SHOWN = 4;

/** Called with the text of every error toast (main.js: the Messages tab keeps it). */
let errorHook = null;

export function setErrorHook(f) {
  errorHook = f;
}

/** A transient message at the top right of the map: `kind` is "info", "error" or "log". A
 *  click dismisses it. An error stays 10 s (at least) and is also kept under Messages. */
export function toast(text, kind = "info", ms = 4500) {
  if (kind === "error") {
    ms = Math.max(ms, 10000);
    try {
      errorHook?.(String(text));
    } catch (e) {
      console.warn("error hook:", e);
    }
  }
  return toastNode(h(`div.toast.${kind}`, { role: kind === "error" ? "alert" : "status", title: "Click to dismiss" }, text), ms);
}

/** The plain sentence of each error code the server can answer (src/command.rs `CmdErr`,
 *  src/query.rs `QueryErr`, src/transport: the protocol's and the HTTP host's codes). */
const ERROR_SENTENCES = {
  not_your_turn: "It is not your turn.",
  decision_pending: "Answer the open decision first.",
  invalid: "The game did not accept this order.",
  not_owner: "That is not yours.",
  gone: "That no longer exists.",
  cannot: "Not possible now.",
  busy: "The game is busy. Try again in a moment.",
  crashed: "The game crashed.",
  no_game: "No game is loaded.",
  starting: "The game is starting. Try again in a moment.",
  stopped: "The server has stopped.",
  not_found: "Not found.",
  forbidden: "Not allowed.",
  method: "The request failed.",
  http: "The request failed.",
  art: "An image failed to load.",
};

/** Whether a refusal's detail can be shown to the player: no JSON, code, HTTP request, engine
 *  type name (`SPELL_X`) or decision id. */
function plainDetail(t) {
  return !!t && !/[{}`"<>]|\/api|\b(GET|POST|HTTP)\b|\b[A-Z][A-Z0-9]*_[A-Z0-9_]+\b|\bdecision \d|\bfield\b/.test(t);
}

/** A sentence: capital first letter, a full stop. */
function sentence(t) {
  const s = t.trim();
  return `${s[0].toUpperCase()}${s.slice(1)}${/[.!?]$/.test(s) ? "" : "."}`;
}

/** An error for the player: a refusal (`{code, detail}`: api.js `ApiError`) as a plain sentence,
 *  its reason when the rules gave one (`cannot`, a plain `invalid`); a transport failure as a
 *  connection problem. The technical detail goes to the console. */
export function errorText(e) {
  if (e && typeof e.code === "string") {
    if (e.detail) console.warn(`refused (${e.code}): ${e.detail}`);
    if ((e.code === "cannot" || e.code === "invalid") && plainDetail(e.detail)) return sentence(e.detail);
    return ERROR_SENTENCES[e.code] ?? "The request failed.";
  }
  console.warn("request failed:", e?.message ?? e);
  return "Connection problem: the server does not answer.";
}

/** Shows a built toast element (`div.toast`) for `ms`; returns it. */
export function toastNode(el, ms = 4500) {
  const box = $("#toasts");
  if (!box) return null;
  box.appendChild(el);
  while (box.children.length > TOASTS_SHOWN) box.firstChild.remove();
  // Fade out before removing (CSS transition on `.leaving`); a fallback removes it even when no
  // transition runs (reduced motion, a hidden tab).
  const leave = () => {
    if (el.classList.contains("leaving")) return;
    el.classList.add("leaving");
    el.addEventListener("transitionend", () => el.remove(), { once: true });
    setTimeout(() => el.remove(), 400);
  };
  el.addEventListener("click", leave);
  setTimeout(leave, ms);
  return el;
}

let busyCount = 0;

/** Runs `f` with the busy indicator shown. */
export async function busy(f) {
  busyCount++;
  document.body.classList.add("busy");
  try {
    return await f();
  } finally {
    if (--busyCount === 0) document.body.classList.remove("busy");
  }
}

/** Formats a signed number: +3, -2, 0. */
export function signed(n) {
  return n > 0 ? `+${n}` : `${n}`;
}
