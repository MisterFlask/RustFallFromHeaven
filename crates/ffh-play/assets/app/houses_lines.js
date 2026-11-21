// This game's Houses (docs/HOUSES.md, rule 15): the pure parts of the client's Houses, shared by
// houses.js (the screens), the renderers (the plates' shields) and the node tests. No DOM and no
// state: what the server sends in, what to draw out. Every line of rules is the server's; this
// module only groups and labels them.

/** The five states, best first, as the server names them (`Friendly`, ...). */
export const STATES = ["Friendly", "Pleased", "Cautious", "Annoyed", "Furious"];

/** Each state's colour (style.css: lichen, a paler lichen, stone, amber, scarlet), for a canvas
 *  that cannot read the CSS variables. */
const STATE_COLOUR = { Friendly: "#bddb5d", Pleased: "#d9e8a6", Cautious: "#adb3bd", Annoyed: "#ffb63b", Furious: "#f15b47" };

/** `state` as one of {@link STATES} (any case); an unknown one is `Cautious`. */
export function stateName(state) {
  const s = String(state ?? "").toLowerCase();
  return STATES.find((x) => x.toLowerCase() === s) ?? "Cautious";
}

/** The state's colour as `#rrggbb`. */
export function stateColour(state) {
  return STATE_COLOUR[stateName(state)];
}

/** The state's class (`hs-pleased`). */
export function stateClass(state) {
  return `hs-${stateName(state).toLowerCase()}`;
}

/** The Houses' colours, a small fixed palette (a House's by its id): gules, azure, vert, purpure,
 *  or, tenné, argent, sable-grey. Each shield is drawn with a dark edge ({@link SHIELD_EDGE}), so
 *  the light ones read on the plates and the dark ones on the interface. */
export const HOUSE_COLOURS = ["#d8323a", "#3b7bd9", "#2f9b57", "#9550c6", "#e8c42a", "#d9822b", "#dfe3e8", "#6e7682"];

export const SHIELD_EDGE = "#140d0c";

/** House `id`'s colour. */
export function houseColour(id) {
  const n = HOUSE_COLOURS.length;
  const i = Number.isFinite(Number(id)) ? Math.trunc(Number(id)) : 0;
  return HOUSE_COLOURS[((i % n) + n) % n];
}

/** The heater shield in a 16 x 18 box (SVG path data, in box units; a canvas draws it with
 *  `new Path2D(SHIELD.path)`, made once). */
export const SHIELD = { w: 16, h: 18, path: "M1 1H15V8C15 12.5 12 15.5 8 17C4 15.5 1 12.5 1 8Z" };

/** What a plate's shield carries besides its colour: the red ✖ of a Furious House (its perk is
 *  off), the amber ◆ of an Annoyed one; null otherwise. `{glyph, colour}`. */
export function shieldMark(state) {
  const s = stateName(state);
  if (s === "Furious") return { glyph: "✖", colour: STATE_COLOUR.Furious };
  if (s === "Annoyed") return { glyph: "◆", colour: STATE_COLOUR.Annoyed };
  return null;
}

/** The box of a plate's House shield `ht` high centred at (`cx`, `cy`), for its tooltip
 *  (plates.js `plateHitAt`): `{kind: "house", city, x0, y0, x1, y1}`; null without a House. */
export function shieldHit(city, cx, cy, ht) {
  if (!city?.house) return null;
  const w = (SHIELD.w * ht) / SHIELD.h;
  return { kind: "house", city: city.id, x0: cx - w / 2, y0: cy - ht / 2, x1: cx + w / 2, y1: cy + ht / 2 };
}

/** The tooltip of a plate's shield: `House Aldric: Pleased` (the view's `city.house`, `{id,
 *  label, state}`), with the House keyword's reminder; null without one. */
export function shieldTip(city) {
  const x = city?.house;
  if (!x) return null;
  return { title: `${x.label}: ${stateName(x.state)}`, lines: [], reminders: [{ keyword: "KEYWORD_HOUSE" }] };
}

/** A card's sections, in the order the screens draw them (`x`: a House of the `houses` query or a
 *  card of a pick's offer): `lines` (alignment, religion, sympathy or the outsider's line, the
 *  perk's name), then the perk's effect lines and, for a House whose perk is off, that line; the
 *  opinion line; the happiness line. Lines the server did not send are left out. */
export function cardSections(x) {
  const lines = (x?.lines ?? []).filter(Boolean);
  const perkName = lines.find((l) => l.startsWith("Perk: ")) ?? null;
  const leaning = lines.filter((l) => l !== perkName);
  return {
    leaning,
    perk: perkName,
    perkLines: (x?.perk?.lines ?? []).filter(Boolean),
    perkOff: x?.perk_off ?? null,
    opinion: x?.opinion_line ?? null,
    happiness: x?.happiness_line ?? null,
  };
}

/** A preview's lines as one list (`[{lines}]`, the server's `house_preview`); [] for none. */
export function previewLines(list) {
  return (list ?? []).flatMap((x) => x?.lines ?? []).filter(Boolean);
}

/** The Houses previews of the items of a deal being made (the trade query's `house_preview`:
 *  `{peace, open_borders, defensive_pact}`): those the deal holds, by the item type names in
 *  `items` (`TRADE_PEACE_TREATY`, `TRADE_OPEN_BORDERS`, `TRADE_DEFENSIVE_PACT`). */
export function dealPreview(preview, items) {
  if (!preview) return [];
  const has = (t) => items.includes(t);
  return [
    ...(has("TRADE_PEACE_TREATY") ? previewLines(preview.peace) : []),
    ...(has("TRADE_OPEN_BORDERS") ? previewLines(preview.open_borders) : []),
    ...(has("TRADE_DEFENSIVE_PACT") ? previewLines(preview.defensive_pact) : []),
  ];
}

/** The log entries to alert among `entries` (newest last): the Houses' messages (`mark:
 *  "house"`). `fresh`: the game's log read for the first time: this turn's only. */
export function alertEntries(entries, { fresh = false, turn = null } = {}) {
  return (entries ?? []).filter((e) => (e.mark === "house" || e.kind === "house") && !(fresh && turn !== null && e.t !== turn));
}

/** A House alert's portrait's leader (the view's `houses` brief by the entry's House id), null
 *  when the entry names none of the human's Houses. */
export function alertHouse(entry, brief) {
  const id = entry?.house;
  if (id === undefined || id === null) return null;
  return (brief ?? []).find((b) => b.id === id) ?? null;
}

// ---- the map's plates --------------------------------------------------------------------------

let shieldPath = null;

/** A city's House shield on a plate (the view's `city.house`), `ht` px high centred at (`cx`,
 *  `cy`) on canvas `g`: the House's colour with a dark edge, and on it the mark of an Annoyed or
 *  Furious House. Returns the width it takes (0 without a House). One cached path; nothing
 *  measured or blurred. */
export function drawPlateShield(g, city, cx, cy, ht) {
  const x = city?.house;
  if (!x) return 0;
  const s = ht / SHIELD.h;
  const w = SHIELD.w * s;
  if (!shieldPath && typeof Path2D !== "undefined") shieldPath = new Path2D(SHIELD.path);
  if (!shieldPath) return 0;
  g.save();
  g.translate(cx - w / 2, cy - ht / 2);
  g.scale(s, s);
  g.fillStyle = houseColour(x.id);
  g.strokeStyle = SHIELD_EDGE;
  g.lineWidth = 1.6 / s;
  g.lineJoin = "round";
  g.fill(shieldPath);
  g.stroke(shieldPath);
  g.restore();
  const mk = shieldMark(x.state);
  if (mk) {
    const r = ht * 0.28;
    const my = cy - ht * 0.06;
    g.save();
    g.beginPath();
    g.arc(cx, my, r, 0, Math.PI * 2);
    g.fillStyle = "rgba(18,14,12,0.92)";
    g.fill();
    g.fillStyle = mk.colour;
    g.font = `700 ${Math.max(7, Math.round(r * 1.5))}px system-ui, 'DejaVu Sans', sans-serif`;
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(mk.glyph, cx, my + 0.5);
    g.restore();
  }
  return w;
}
