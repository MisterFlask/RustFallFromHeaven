// Calculated numbers (docs/PLAY_DESIGN.md, "Calculated numbers"): a number the rules calculate
// shows how it is made in a tooltip, its parts and how they combine, with the values in play
// (`Gain: 5.5 × 39% = 2.1 a turn.`). The rules write the lines (their `*_calc` reads, sent as the
// views' `calc` fields); this module draws them. Hover or focus shows the tip, and so does a tap
// (a touch screen has no hover). A text with a calculation is marked by a dotted underline.

import { h } from "./dom.js";
import { tip, showTip } from "./tips.js";

/** The parts of a courtship's calculation, in the order its tooltip shows them. */
const COURTSHIP_PARTS = ["requirement", "inflow", "reach", "gain", "guard", "turn"];

/** A calculation's lines, flat: a list of lines, or a courtship's `{requirement, inflow, reach,
 *  gain, guard, turn}` in that order (`parts`: only those); [] for none. */
export function calcLines(calc, parts = COURTSHIP_PARTS) {
  if (!calc) return [];
  if (Array.isArray(calc)) return calc.filter(Boolean);
  return parts.flatMap((k) => calc[k] ?? []).filter(Boolean);
}

/** A calculation's tooltip: a line a row; null for none. */
export function calcTip(calc, parts) {
  const lines = calcLines(calc, parts);
  return lines.length ? h("div.tip-calc", lines.map((l) => h("div.tip-calc-line", l))) : null;
}

/** Element `el` with calculation `calc` (`parts` of a courtship's) as its tooltip, shown on hover,
 *  focus or a tap, and marked as a calculated number. Returns `el`, unchanged without one. */
export function withCalc(el, calc, parts) {
  if (!calcLines(calc, parts).length) return el;
  el.classList.add("calc");
  if (!el.hasAttribute("tabindex") && !/^(BUTTON|A|INPUT|SELECT)$/.test(el.tagName)) el.tabIndex = 0;
  tip(el, () => calcTip(calc, parts));
  el.addEventListener("click", () => showTip(el));
  return el;
}

/** A `tag` element (an `h` selector) holding `text`, with calculation `calc` as its tooltip. */
export function calcText(tag, text, calc, parts) {
  return withCalc(h(tag, text), calc, parts);
}
