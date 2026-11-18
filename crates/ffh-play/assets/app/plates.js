// The marks on the cities' name plates and their tooltips (docs/PERSONS.md, "UI", part G;
// docs/HUMAN.md): a House's shield, its Furious countdown's pill, a capital's star. The marks stay
// compact on the map; pointing at one (a tap on a touch screen) gives its explainer in the map's
// tooltip, with the keywords' reminders, as a sea's name does on the Sea layer. Each renderer
// records the boxes of the marks it drew (`plateHits`: the HD-2D map in r3d/hud.js, the isometric
// map in iso.js, the flat map in map.js); map.js finds the mark under the pointer
// ({@link plateHitAt}) and shows {@link plateTip}'s lines. Pure: no DOM and no state, so the node
// tests read the same texts.

import { SHIELD, stateKey, stateWord, stateGlyph, stateColour, turnsText } from "./persons_mark.js";

/** The boxes of a plate's House marks: the shield `ht` high centred at (`cx`, `cy`), and when the
 *  shield drew more than itself (`drawn`, persons_lines.js `drawPlateShield`'s width: the Furious
 *  countdown's pill at its left), the pill. `[{kind: "shield" | "pill", city, x0, y0, x1, y1}]`. */
export function shieldHits(city, cx, cy, ht, drawn) {
  if (!city?.house) return [];
  const w = (SHIELD.w * ht) / SHIELD.h;
  const out = [{ kind: "shield", city: city.id, x0: cx - w / 2, y0: cy - ht / 2, x1: cx + w / 2, y1: cy + ht / 2 }];
  if (drawn > w + 0.5) out.push({ kind: "pill", city: city.id, x0: cx - w / 2 - (drawn - w), y0: cy - ht / 2, x1: cx - w / 2, y1: cy + ht / 2 });
  return out;
}

/** The box of a capital's star of radius `r` centred at (`cx`, `cy`). */
export function starHit(city, cx, cy, r) {
  return { kind: "star", city: city.id, x0: cx - r, y0: cy - r, x1: cx + r, y1: cy + r };
}

/** The mark under (`x`, `y`) among `hits` (the last drawn first: it is on top), `pad` px of slack
 *  round each (a finger's tap); null when none. */
export function plateHitAt(hits, x, y, pad = 0) {
  for (let i = (hits?.length ?? 0) - 1; i >= 0; i--) {
    const b = hits[i];
    if (x >= b.x0 - pad && x <= b.x1 + pad && y >= b.y0 - pad && y <= b.y1 + pad) return b;
  }
  return null;
}

/** A House's place in its realm, as its tooltips name it: `your vassal`, `vassal of the
 *  Svartalfar` (`liegeName`: the liege's civilization, the view's `players[].house.liege_name`). */
function vassalOf(house, mine, liegeName) {
  return house.liege === mine ? "your vassal" : liegeName ? `vassal of the ${liegeName}` : "a vassal";
}

/** The tooltip of plate mark `kind` (`shield`, `pill`, `star`) on `city` (the view's city, with
 *  its `house`: `{label, liege, state, turns}`, the state and turns for its liege only), as
 *  `{title, lines: [{text, colour?}], reminders: [{keyword} | {attitude}]}` (the keywords' and the
 *  attitudes' reminders to show with it); null when there is nothing to say. `ctx`: `mine` (your
 *  player), `liegeName` (the House's liege's civilization, `Svartalfar`), `outcome` (the rules'
 *  stated outcome of its countdown: `House Aldric will break away with Ventry in 8 turns: war.`),
 *  `civ` (the owner's civilization, for a capital that is no House's).
 *  - shield: `House Aldric (your vassal)` and, for its liege, its state (`✖ Furious 8 turns`).
 *  - pill: `House Aldric: Furious.` and the rules' outcome.
 *  - star: `Capital of House Aldric (your vassal).`, `Capital of the Svartalfar.` */
export function plateTip(kind, city, { mine = null, liegeName = null, outcome = null, civ = null } = {}) {
  const x = city?.house;
  const state = x?.state ? stateKey(x.state) : null;
  const stateLine = state ? { text: `${stateGlyph(state)} ${stateWord(state)}${x.turns !== null && x.turns !== undefined ? ` ${turnsText(x.turns)}` : ""}`, colour: stateColour(state) } : null;
  const reminders = (withState) => [{ keyword: "KEYWORD_HOUSE" }, ...(withState && state ? [{ attitude: `ATTITUDE_${state.toUpperCase()}` }] : [])];
  switch (kind) {
    case "shield":
      if (!x) return null;
      return { title: `${x.label} (${vassalOf(x, mine, liegeName)})`, lines: stateLine ? [stateLine] : [], reminders: reminders(true) };
    case "pill":
      if (!x || !state) return null;
      return { title: `${x.label}: ${stateWord(state)}.`, lines: outcome ? [{ text: outcome, colour: stateColour(state) }] : [], reminders: reminders(true) };
    case "star":
      if (x) return { title: `Capital of ${x.label} (${vassalOf(x, mine, liegeName)}).`, lines: [], reminders: reminders(false) };
      return civ ? { title: `Capital of the ${civ}.`, lines: [], reminders: [] } : null;
  }
  return null;
}
