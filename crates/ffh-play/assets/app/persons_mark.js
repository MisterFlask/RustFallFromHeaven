// Persons of Note: the one mark of a loyalty state and the Houses' heraldry (docs/PERSONS.md,
// "UI": principle 2, parts A and G). Pure: no DOM and no state, so persons.js, the map's name
// plates (canvas) and the node tests draw from the same numbers.

/** The five states, the attitude scale, from the rules' `LoyaltyState` (the host sends
 *  `Friendly`, ...; any case; docs/PERSONS.md, "The second build", rule 5). */
export const STATES = ["friendly", "pleased", "cautious", "annoyed", "furious"];

const WORD = { friendly: "Friendly", pleased: "Pleased", cautious: "Cautious", annoyed: "Annoyed", furious: "Furious" };
const GLYPH = { friendly: "▲", pleased: "△", cautious: "●", annoyed: "◆", furious: "✖" };
/** The stylesheet's colour of each state (style.css palette entries: lichen, a paler lichen,
 *  stone, amber, scarlet), for a canvas that cannot read the CSS variables. */
const COLOUR = { friendly: "#bddb5d", pleased: "#d9e8a6", cautious: "#adb3bd", annoyed: "#ffb63b", furious: "#f15b47" };

/** The state's key in {@link STATES} (`"Annoyed"` → `"annoyed"`); an unknown or missing state
 *  is `"cautious"` (loyalty 0). */
export function stateKey(state) {
  const k = String(state ?? "").toLowerCase();
  return STATES.includes(k) ? k : "cautious";
}

/** `Friendly`, `Pleased`, `Cautious`, `Annoyed`, `Furious`. */
export function stateWord(state) {
  return WORD[stateKey(state)];
}

/** ▲ △ ● ◆ ✖ */
export function stateGlyph(state) {
  return GLYPH[stateKey(state)];
}

/** The mark's class beside `pstate`: `ps-friendly`, `ps-pleased`, `ps-cautious`, `ps-annoyed`,
 *  `ps-furious`. */
export function stateClass(state) {
  return `ps-${stateKey(state)}`;
}

/** The state's colour as `#rrggbb` (the same as the mark's CSS). */
export function stateColour(state) {
  return COLOUR[stateKey(state)];
}

/** `8 turns`, `1 turn`. */
export function turnsText(turns) {
  return `${turns} ${turns === 1 ? "turn" : "turns"}`;
}

/** The mark as text: `Cautious ●`, and with the turns left of a countdown `Furious ✖ 8 turns`
 *  (`turns` null or undefined: none shown). */
export function stateText(state, turns) {
  const base = `${stateWord(state)} ${stateGlyph(state)}`;
  return turns === null || turns === undefined ? base : `${base} ${turnsText(turns)}`;
}

/** The Houses' tinctures, by the rules' colour index (`House::color`, below `HOUSE_COLORS` = 8):
 *  gules, azure, vert, purpure, or, tenné, argent, sable-grey. Each is drawn with a dark edge
 *  (SHIELD_EDGE), so the light ones read on the map's pale plates and the dark ones on the
 *  interface's navy. */
export const HOUSE_COLOURS = ["#d8323a", "#3b7bd9", "#2f9b57", "#9550c6", "#e8c42a", "#d9822b", "#dfe3e8", "#6e7682"];

/** The tinctures' names, in the same order (for a tooltip or a test). */
export const HOUSE_TINCTURES = ["gules", "azure", "vert", "purpure", "or", "tenné", "argent", "sable"];

/** The shield's edge colour. */
export const SHIELD_EDGE = "#140d0c";

/** House colour `index` as `#rrggbb` (wrapped into the eight). */
export function houseColour(index) {
  const n = HOUSE_COLOURS.length;
  const i = Number.isFinite(Number(index)) ? Math.trunc(Number(index)) : 0;
  return HOUSE_COLOURS[((i % n) + n) % n];
}

/** `House Ashledger` (a name that already says House stays as it is). */
export function houseLabel(name) {
  const s = String(name ?? "").trim();
  return /^House\s/.test(s) ? s : `House ${s}`;
}

/** The heater shield in a 16 x 18 box: a flat chief, straight flanks to the middle, curves to the
 *  point. `path` is SVG path data (an SVG `<path d>` or a canvas `new Path2D(path)`), in box units. */
export const SHIELD = { w: 16, h: 18, path: "M1 1H15V8C15 12.5 12 15.5 8 17C4 15.5 1 12.5 1 8Z" };

/** The same outline as a polygon of points `[x, y]` scaled to `w` x `h` (for a canvas that
 *  draws with `moveTo`/`lineTo`, or an SVG `<polygon>`); `steps` points along each curve. */
export function shieldPoints(w = SHIELD.w, h = SHIELD.h, steps = 6) {
  const sx = w / SHIELD.w;
  const sy = h / SHIELD.h;
  const cubic = (p0, p1, p2, p3, t) => {
    const u = 1 - t;
    return u * u * u * p0 + 3 * u * u * t * p1 + 3 * u * t * t * p2 + t * t * t * p3;
  };
  const pts = [[1, 1], [15, 1], [15, 8]];
  // C15 12.5 12 15.5 8 17, then C4 15.5 1 12.5 1 8.
  const curves = [
    [[15, 8], [15, 12.5], [12, 15.5], [8, 17]],
    [[8, 17], [4, 15.5], [1, 12.5], [1, 8]],
  ];
  for (const [a, b, c, d] of curves) {
    for (let i = 1; i <= steps; i++) {
      const t = i / steps;
      pts.push([cubic(a[0], b[0], c[0], d[0], t), cubic(a[1], b[1], c[1], d[1], t)]);
    }
  }
  return pts.map(([x, y]) => [Math.round(x * sx * 100) / 100, Math.round(y * sy * 100) / 100]);
}

/** What the map draws on a House's shield (part G): nothing for Friendly, Pleased and Cautious
 *  (and for a House not ours: no state sent); the amber ◆ for Annoyed; the red ✖ and the turns
 *  left for Furious. `{glyph, colour, count}` or null. */
export function shieldMark(state, turns) {
  if (state === null || state === undefined) return null;
  const k = stateKey(state);
  if (k === "annoyed") return { glyph: GLYPH.annoyed, colour: COLOUR.annoyed, count: null };
  if (k === "furious") return { glyph: GLYPH.furious, colour: COLOUR.furious, count: turns ?? null };
  return null;
}
