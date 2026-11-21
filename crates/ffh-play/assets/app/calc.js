// Calculated numbers (docs/PLAY_DESIGN.md, "Calculated numbers"): a number the rules calculate
// shows how it is made in a tooltip, its parts and how they combine, with the values in play
// (`Gain: 5.5 × 39% = 2.1 a turn.`). The rules write the lines (their `*_calc` reads, sent as the
// views' `calc` fields); this module draws them. Hover or focus shows the tip, and so does a tap
// (a touch screen has no hover). A text with a calculation is marked by a dotted underline.

import { h } from "./dom.js";
import { tip, showTip } from "./tips.js";

/** A calculation's lines, flat (a list of lines; empty ones left out); [] for none. */
export function calcLines(calc) {
  if (!Array.isArray(calc)) return [];
  return calc.filter(Boolean);
}

/** A calculation's tooltip: a line a row; null for none. */
export function calcTip(calc) {
  const lines = calcLines(calc);
  return lines.length ? h("div.tip-calc", lines.map((l) => h("div.tip-calc-line", l))) : null;
}

/** Element `el` with calculation `calc` as its tooltip, shown on hover, focus or a tap, and marked
 *  as a calculated number. Returns `el`, unchanged without one. */
export function withCalc(el, calc) {
  if (!calcLines(calc).length) return el;
  el.classList.add("calc");
  if (!el.hasAttribute("tabindex") && !/^(BUTTON|A|INPUT|SELECT)$/.test(el.tagName)) el.tabIndex = 0;
  tip(el, () => calcTip(calc));
  el.addEventListener("click", () => showTip(el));
  return el;
}

/** A `tag` element (an `h` selector) holding `text`, with calculation `calc` as its tooltip. */
export function calcText(tag, text, calc) {
  return withCalc(h(tag, text), calc);
}
