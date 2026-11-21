// Raids (docs/RAIDS.md, section 8; docs/HUMAN.md, "Raids"): how a raided city reads on the maps.
// The view's city tiles carry `siege` and `raid` (`{state, label, text, percent, raiders, calc}`:
// `contested`, `overrun` or `stripped`, null with no raid); every map appends their labels to the
// city's banner (`Vallus · Overrun`), and the HD-2D map rings the plate: solid red while besieged
// (that wins), dashed amber while contested or overrun. Pure: no DOM and no state, so the node tests read it.

/** The ring of a besieged city's plate (r3d/hud.js's red). */
export const SIEGE_RING = "#e0442c";
/** The ring of a contested or overrun city's plate: the palette's warning amber (iso.css
 *  `--b-amber`, the Annoyed mark's). */
export const RAID_RING = "#ffb63b";

/** The banner's suffix of `city` (a view city tile): its siege's label, then its raid's, each
 *  after ` · ` (` · Hunger 2/3 · Overrun`); "" for none. */
export function plateTag(city) {
  return [city?.siege?.label, city?.raid?.label]
    .filter(Boolean)
    .map((l) => ` · ${l}`)
    .join("");
}

/** The ring around `city`'s plate, `{colour, dash}` (a canvas line dash, [] solid), or null:
 *  solid red while besieged; dashed amber while contested or overrun (not stripped: nothing is
 *  taken), dashed so that it reads apart from a gold owner's own plate border. */
export function plateRing(city) {
  if (city?.besieged) return { colour: SIEGE_RING, dash: [] };
  const s = city?.raid?.state;
  return s === "overrun" || s === "contested" ? { colour: RAID_RING, dash: [6, 4] } : null;
}

/** The class of a raid's line (the panels' `p.raid-status`, the chips): `warn` while it takes,
 *  `dim` while stripped; "" with none. */
export function raidTone(raid) {
  if (!raid) return "";
  return raid.state === "stripped" ? "dim" : "warn";
}
