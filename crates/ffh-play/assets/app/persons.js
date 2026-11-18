// Persons of Note in the browser client (docs/PERSONS.md, "UI"): the one mark of a loyalty state
// (every screen draws it with stateMark, never restyled), a House's shield, and the two entries
// other screens call: the Court screen and a city's grant list. The Court and the grant list are
// placeholders until their screens are built; their signatures are the contract.

import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateClass, stateGlyph, stateKey, stateWord, turnsText } from "./persons_mark.js";

const SVG = "http://www.w3.org/2000/svg";

/** The mark of a state: `span.pstate.ps-<state>` with the glyph and the word, and for a running
 *  countdown the turns left (`Disloyal ✖ 8 turns`). `state` as the host sends it (`Restless`) or
 *  a key of persons_mark.js STATES; `turns` null or undefined: none. */
export function stateMark(state, turns) {
  const key = stateKey(state);
  const hasTurns = turns !== null && turns !== undefined;
  return h(
    `span.pstate.${stateClass(key)}`,
    { "data-state": key },
    h("span.pstate-glyph", { "aria-hidden": "true" }, stateGlyph(key)),
    h("span.pstate-word", stateWord(key)),
    hasTurns ? h("span.pstate-turns", turnsText(turns)) : null,
  );
}

/** A House's shield in its colour (`House::color`), `size` CSS px high: an inline SVG. */
export function houseShield(colourIndex, size = 16) {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 ${SHIELD.w} ${SHIELD.h}`);
  svg.setAttribute("width", String(Math.round((size * SHIELD.w) / SHIELD.h)));
  svg.setAttribute("height", String(size));
  svg.setAttribute("class", "house-shield");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", SHIELD.path);
  path.setAttribute("fill", houseColour(colourIndex));
  path.setAttribute("stroke", SHIELD_EDGE);
  path.setAttribute("stroke-width", "1.2");
  path.setAttribute("stroke-linejoin", "round");
  svg.appendChild(path);
  return svg;
}

/** Opens the Court screen (part A); `focus`: `{house}` or `{person}` (ids) to open at that card,
 *  or nothing. A placeholder until the Court is built. */
export function openCourt(focus = null) {
  return openModal({
    title: "Court",
    tag: "court",
    wide: true,
    body: [h("p.dim", "Not built yet.")],
    buttons: [{ label: "Close", value: null }],
  });
}

/** Opens the grant list of city `cityId` (part B); `prefer`: `{house}`, `{person}` or `{unit}`
 *  (ids) to put that candidate first, or nothing. A placeholder until the grant list is built. */
export function openGrant(cityId, prefer = null) {
  return openModal({
    title: "Grant",
    tag: "grant",
    body: [h("p.dim", "Not built yet.")],
    buttons: [{ label: "Close", value: null }],
  });
}
