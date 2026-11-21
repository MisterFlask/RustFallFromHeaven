// The marks on the cities' name plates and their tooltips (docs/HUMAN.md): a capital's star. The
// marks stay compact on the map; pointing at one (a tap on a touch screen) gives its explainer in
// the map's tooltip, with the keywords' reminders, as a sea's name does on the Sea layer. Each
// renderer records the boxes of the marks it drew (`plateHits`: the HD-2D map in r3d/hud.js, the
// isometric map in iso.js, the flat map in map.js); map.js finds the mark under the pointer
// ({@link plateHitAt}) and shows {@link plateTip}'s lines. Pure: no DOM and no state, so the node
// tests read the same texts.

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

/** The tooltip of plate mark `kind` (`star`) on `city` (the view's city), as `{title, lines:
 *  [{text, colour?}], reminders: [{keyword} | {attitude}]}` (the keywords' and the attitudes'
 *  reminders to show with it); null when there is nothing to say. `ctx`: `civ` (the owner's
 *  civilization).
 *  - star: `Capital of the Svartalfar.` */
export function plateTip(kind, city, { civ = null } = {}) {
  switch (kind) {
    case "star":
      return city && civ ? { title: `Capital of the ${civ}.`, lines: [], reminders: [] } : null;
  }
  return null;
}
